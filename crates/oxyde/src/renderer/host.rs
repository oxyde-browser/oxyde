//! Côté navigateur : la page d'un onglet, qu'elle vive dans ce processus
//! ([`PageHost::Local`]) ou dans un processus de rendu
//! ([`PageHost::Remote`]).
//!
//! Le processus de rendu n'est pas digne de confiance : ses messages sont
//! décodés avec méfiance, ses requêtes réseau sont filtrées (il ne lit un
//! fichier que s'il affiche lui-même un document `file:`), et sa disparition
//! n'emporte que son onglet.

use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

use oxyde_compositor::{ROOT_SCROLLER, ScrollTree};
use oxyde_content::{FindStatus, FormPost, KeyInput, Navigation, Page};
use oxyde_css::{Color, Viewport};
use oxyde_ipc::{Bytes, ChannelError, Receiver, Sender};
use oxyde_paint::DisplayList;
use oxyde_paint::wire::ResourceStore;
use oxyde_text::TextContext;
use url::Url;

use super::protocol::{FromRenderer, KeyMessage, NetworkReply, NetworkRequest, ToRenderer};
use crate::loader::{self, LoadError};

/// Fin d'un chargement mené par un processus de rendu.
pub(crate) struct Commit {
    pub(crate) url: Url,
    pub(crate) error: Option<LoadError>,
}

/// Page hébergée par un processus de rendu.
pub(crate) struct RemotePage {
    process: oxyde_sandbox::Process,
    sender: Sender<ToRenderer>,
    receiver: Receiver<FromRenderer>,
    store: ResourceStore,
    list: DisplayList,
    /// Défilement tenu par le compositeur, de ce côté-ci : la molette
    /// n'attend pas le processus de rendu.
    scroll: ScrollTree,
    content: (f32, f32),
    background: Color,
    url: Url,
    title: String,
    viewport: (f32, f32),
    /// Vrai si une image est arrivée depuis la dernière mise à jour.
    dirty: bool,
    navigations: Vec<Navigation>,
    commits: Vec<Commit>,
    /// Dernier point de survol transmis, et sa réponse.
    hover: Cell<Option<(f32, f32)>>,
    over_link: bool,
    /// Avancement de la recherche dans la page, tel que le processus de
    /// rendu l'a annoncé en dernier.
    found: FindStatus,
    crashed: bool,
    /// Site auquel ce processus est voué, tenu par le navigateur seul.
    lock: Arc<Mutex<SiteLock>>,
}

/// Verrou de site d'un processus de rendu. Le navigateur le tient d'après ce
/// qu'il a lui-même demandé et reçu du réseau, jamais d'après ce que le
/// processus de rendu affirme : un processus pris ne peut pas s'attribuer le
/// site d'un autre.
#[derive(Debug, Default)]
struct SiteLock {
    /// Adresse du document que le processus affiche ; `None` tant qu'il
    /// n'en a chargé aucun.
    site: Option<Url>,
    /// Navigation que le navigateur vient de lui confier : la seule
    /// requête de document qu'il a le droit d'émettre.
    expected: Option<Url>,
    /// Adresse vers laquelle cette navigation a été redirigée, sur un
    /// autre site : elle doit être reprise par un autre processus.
    redirected: Option<Url>,
}

impl SiteLock {
    /// Vrai si `url` est du site auquel le processus est voué.
    fn allows(&self, url: &Url) -> bool {
        self.site
            .as_ref()
            .is_some_and(|site| oxyde_net::same_site(site, url))
    }
}

fn locked(lock: &Mutex<SiteLock>) -> std::sync::MutexGuard<'_, SiteLock> {
    // Un verrou empoisonné garde un état lisible : le site reste celui que
    // le navigateur y a inscrit.
    lock.lock().unwrap_or_else(PoisonError::into_inner)
}

impl std::fmt::Debug for RemotePage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemotePage")
            .field("url", &self.url.as_str())
            .field("crashed", &self.crashed)
            .finish_non_exhaustive()
    }
}

impl Drop for RemotePage {
    fn drop(&mut self) {
        // L'onglet disparaît : son processus aussi. S'il est déjà mort, il
        // n'y a rien à tuer.
        self.process.kill();
        self.process.wait();
    }
}

/// Vrai tant que les processus de rendu sont lancés dans le bac à sable.
static SANDBOX: AtomicBool = AtomicBool::new(true);

/// Renonce au bac à sable pour les processus de rendu lancés ensuite
/// (`--no-sandbox`) : pour le diagnostic seulement.
pub(crate) fn disable_sandbox() {
    SANDBOX.store(false, Ordering::Relaxed);
}

/// Lance le processus de rendu : dans le bac à sable, sauf s'il a été
/// désactivé ou si le système le refuse — ce qui est alors signalé.
fn launch() -> std::io::Result<oxyde_sandbox::Spawned> {
    let executable = std::env::current_exe()?;
    let arguments = ["--type=renderer"];
    if SANDBOX.load(Ordering::Relaxed) {
        match oxyde_sandbox::spawn(&executable, &arguments, oxyde_sandbox::Policy::Locked) {
            Ok(spawned) => return Ok(spawned),
            Err(error) => {
                eprintln!("bac à sable indisponible, processus de rendu lancé sans lui : {error}");
            }
        }
    }
    oxyde_sandbox::spawn_unsandboxed(&executable, &arguments)
}

impl RemotePage {
    /// Lance un processus de rendu et lui fait charger `url`. `wake` est
    /// appelé, depuis un autre fil, chaque fois qu'il a quelque chose à dire.
    pub(crate) fn spawn(
        url: Url,
        post: Option<FormPost>,
        viewport: (f32, f32),
        wake: impl Fn() + Send + 'static,
    ) -> std::io::Result<Self> {
        let child = launch()?;
        let mut page = Self {
            process: child.process,
            sender: Sender::new(child.stdin),
            receiver: Receiver::new(child.stdout, wake),
            store: ResourceStore::new(),
            list: DisplayList::default(),
            scroll: ScrollTree::new(),
            content: (0.0, 0.0),
            background: Color::WHITE,
            url: url.clone(),
            title: String::new(),
            viewport,
            dirty: true,
            navigations: Vec::new(),
            commits: Vec::new(),
            hover: Cell::new(None),
            over_link: false,
            found: FindStatus::default(),
            crashed: false,
            lock: Arc::default(),
        };
        page.load(url, post);
        Ok(page)
    }

    fn send(&mut self, message: &ToRenderer) {
        if self.sender.send(message).is_err() {
            self.crashed = true;
        }
    }

    /// Fait charger `url` par le processus de rendu ; la fin du chargement
    /// arrive par [`RemotePage::take_commits`].
    pub(crate) fn load(&mut self, url: Url, post: Option<FormPost>) {
        {
            let mut lock = locked(&self.lock);
            if url.scheme() == "data" {
                // Un document `data:` ne passe pas par le réseau : le
                // processus le décode lui-même, et le navigateur l'y voue
                // d'emblée.
                lock.site = Some(url.clone());
                lock.expected = None;
            } else {
                lock.expected = Some(url.clone());
            }
        }
        let message = ToRenderer::Load {
            url: url.to_string(),
            post,
            width: self.viewport.0,
            height: self.viewport.1,
        };
        self.send(&message);
    }

    /// Vrai si le processus de rendu tourne dans le bac à sable.
    pub(crate) fn is_sandboxed(&self) -> bool {
        self.process.is_sandboxed()
    }

    /// Vrai si ce processus peut charger `url` : il n'a encore rien
    /// chargé, ou `url` est du site auquel il est voué. Sinon la navigation
    /// revient à un autre processus.
    pub(crate) fn can_host(&self, url: &Url) -> bool {
        let lock = locked(&self.lock);
        lock.site.is_none() || lock.allows(url)
    }

    /// Vrai si le processus de rendu a disparu.
    pub(crate) fn is_crashed(&self) -> bool {
        self.crashed
    }

    /// Chargements terminés depuis le dernier appel.
    pub(crate) fn take_commits(&mut self) -> Vec<Commit> {
        std::mem::take(&mut self.commits)
    }

    /// Exécute, sur un fil dédié, une requête réseau du processus de rendu,
    /// et lui renvoie la réponse.
    fn serve(&self, id: u64, request: NetworkRequest) {
        let sender = self.sender.clone();
        let lock = Arc::clone(&self.lock);
        let spawned = std::thread::Builder::new()
            .name("oxyde-reseau".to_owned())
            .spawn(move || {
                let reply = answer(request, &lock);
                // Le processus de rendu a pu disparaître entre-temps.
                let _ = sender.send(&ToRenderer::Network { id, reply });
            });
        if spawned.is_err() {
            let refused = LoadError::other(
                "La page n'a pas pu être chargée",
                "aucun fil disponible pour la requête".to_owned(),
                "ERR_FAILED",
            );
            let _ = self.sender.send(&ToRenderer::Network {
                id,
                reply: NetworkReply::Resource(Err(refused)),
            });
        }
    }

    /// Traite les messages du processus de rendu. Renvoie `true` si
    /// l'onglet doit être redessiné.
    pub(crate) fn pump(&mut self) -> bool {
        let mut redraw = false;
        loop {
            let message = match self.receiver.try_recv() {
                Ok(Some(message)) => message,
                Ok(None) => break,
                Err(error) => {
                    if !self.crashed {
                        // Canal fermé ou message illisible : le processus
                        // est perdu, ou ne mérite plus d'être écouté.
                        if !matches!(error, ChannelError::Closed) {
                            eprintln!("processus de rendu écarté : {error}");
                            self.process.kill();
                        }
                        self.crashed = true;
                        redraw = true;
                    }
                    break;
                }
            };
            match message {
                FromRenderer::Network { id, request } => self.serve(id, request),
                FromRenderer::Committed { url, error } => {
                    // Redirection vers un autre site : la navigation est
                    // reprise par un autre processus, et cet échec-ci n'est
                    // pas montré.
                    let redirected = locked(&self.lock).redirected.take();
                    if let Some(url) = redirected {
                        self.navigations.push(Navigation::Load {
                            url,
                            replace: true,
                            post: None,
                        });
                        continue;
                    }
                    if let Ok(url) = Url::parse(&url) {
                        // L'adresse affichée est celle que le navigateur
                        // connaît, pas celle que le processus annonce.
                        let url = self.trusted_url(url);
                        self.url = url.clone();
                        self.scroll = ScrollTree::new();
                        self.commits.push(Commit { url, error });
                        redraw = true;
                    }
                }
                FromRenderer::Frame(frame) => {
                    let frame = *frame;
                    match self.store.decode(frame.list) {
                        Ok(list) => {
                            self.list = list;
                            self.content = (frame.content[0], frame.content[1]);
                            self.scroll
                                .update(&self.list.scrollers, self.viewport, self.content);
                            for (id, x, y) in frame.scroll {
                                self.scroll.scroll_within_limit(id, x, y);
                            }
                            // Ces défilements viennent du processus de rendu :
                            // inutile de les lui renvoyer.
                            self.scroll.take_changed();
                            let [r, g, b, a] = frame.background;
                            self.background = Color { r, g, b, a };
                            if let Ok(url) = Url::parse(&frame.url) {
                                self.url = self.trusted_url(url);
                            }
                            self.title = frame.title;
                            self.dirty = true;
                            redraw = true;
                        }
                        Err(error) => {
                            eprintln!("processus de rendu écarté : image illisible ({error})");
                            self.process.kill();
                            self.crashed = true;
                            return true;
                        }
                    }
                }
                FromRenderer::Navigate(navigation) => {
                    if self.may_navigate(&navigation) {
                        self.navigations.push(navigation);
                    }
                }
                FromRenderer::Clicked { url } => {
                    if let Ok(url) = Url::parse(&url) {
                        let navigation = Navigation::Load {
                            url,
                            replace: false,
                            post: None,
                        };
                        if self.may_navigate(&navigation) {
                            self.navigations.push(navigation);
                        }
                    }
                }
                FromRenderer::Found { count, current } => {
                    self.found = FindStatus {
                        count: usize::try_from(count).unwrap_or(usize::MAX),
                        current: usize::try_from(current).unwrap_or(usize::MAX),
                    };
                    redraw = true;
                }
                FromRenderer::Hover { link } => {
                    if link != self.over_link {
                        self.over_link = link;
                        redraw = true;
                    }
                }
            }
        }
        redraw
    }

    /// Adresse à afficher pour ce processus : `claimed` si elle est du site
    /// auquel il est voué, sinon celle que le navigateur connaît déjà.
    fn trusted_url(&self, claimed: Url) -> Url {
        if locked(&self.lock).allows(&claimed) {
            claimed
        } else {
            self.url.clone()
        }
    }

    /// Vrai si le navigateur accepte cette navigation demandée par la page.
    fn may_navigate(&self, navigation: &Navigation) -> bool {
        let lock = locked(&self.lock);
        match navigation {
            // Une page du web n'ouvre pas un fichier local.
            Navigation::Load { url, .. } => {
                url.scheme() != "file"
                    || lock
                        .site
                        .as_ref()
                        .is_some_and(|site| site.scheme() == "file")
            }
            // Un changement d'adresse sans chargement reste sur le site.
            Navigation::SameDocument { url, .. } => lock.allows(url),
            Navigation::Reload | Navigation::Traverse(_) => true,
        }
    }

    /// Transmet au processus de rendu les zones que le compositeur vient de
    /// faire défiler.
    fn sync_scroll(&mut self) {
        for id in self.scroll.take_changed() {
            let (x, y) = self.scroll.offset(id);
            self.send(&ToRenderer::Scroll { id, x, y });
        }
    }
}

/// Répond à une requête réseau d'un processus de rendu, dans les limites de
/// son verrou de site :
///
/// - une requête de document n'est servie que si c'est la navigation que le
///   navigateur lui a confiée ; sa réponse fixe le site du processus, et une
///   redirection vers un autre site lui est refusée ;
/// - une ressource, une requête de script ou un accès aux cookies ne sont
///   servis qu'au nom d'un document de son site — c'est ce nom qui décide
///   des cookies joints ;
/// - un fichier local n'est lu que pour un document local.
fn answer(request: NetworkRequest, lock: &Mutex<SiteLock>) -> NetworkReply {
    let invalid = |url: &str| {
        NetworkReply::Resource(Err(LoadError::other(
            "Adresse invalide",
            url.to_owned(),
            "ERR_INVALID_URL",
        )))
    };
    let refused = |url: &Url, why: &str| {
        NetworkReply::Resource(Err(LoadError::other(
            "Accès refusé",
            format!("{url} — {why}"),
            "ERR_ACCESS_DENIED",
        )))
    };
    // Réponse à une requête de document : elle fixe le site du processus.
    let commit = |target: &Url, result: Result<(Url, Vec<u8>), LoadError>| match result {
        Ok((final_url, bytes)) => {
            let mut lock = locked(lock);
            if lock.site.is_some() && !lock.allows(&final_url) {
                // Le processus a déjà affiché un autre site : ce document
                // ira dans un processus à lui.
                lock.redirected = Some(final_url);
                drop(lock);
                return refused(target, "redirection vers un autre site.");
            }
            lock.site = Some(final_url.clone());
            NetworkReply::Resource(Ok((final_url.to_string(), Bytes(bytes))))
        }
        Err(error) => {
            // Même en échec, le processus est voué au site demandé : la
            // page d'erreur porte son adresse.
            let mut lock = locked(lock);
            if lock.site.is_none() {
                lock.site = Some(target.clone());
            }
            NetworkReply::Resource(Err(error))
        }
    };
    // Vrai si `target` est la navigation confiée au processus ; elle ne
    // sert qu'une fois.
    let expected = |target: &Url| {
        let mut lock = locked(lock);
        if lock.expected.as_ref() == Some(target) {
            lock.expected = None;
            true
        } else {
            false
        }
    };
    match request {
        NetworkRequest::Get {
            url,
            initiator: None,
        } => {
            let Ok(target) = Url::parse(&url) else {
                return invalid(&url);
            };
            if !expected(&target) {
                return refused(&target, "navigation que le navigateur n'a pas demandée.");
            }
            commit(&target, loader::fetch_from(&target, None))
        }
        NetworkRequest::Get {
            url,
            initiator: Some(initiator),
        } => {
            let (Ok(target), Ok(initiator)) = (Url::parse(&url), Url::parse(&initiator)) else {
                return invalid(&url);
            };
            {
                let lock = locked(lock);
                if !lock.allows(&initiator) {
                    return refused(&target, "ressource demandée au nom d'un autre site.");
                }
                // Un fichier local n'est lu que pour un document local.
                if target.scheme() == "file" && initiator.scheme() != "file" {
                    return refused(&target, "une page web ne peut pas lire un fichier local.");
                }
            }
            NetworkReply::Resource(
                loader::fetch_from(&target, Some(&initiator))
                    .map(|(final_url, bytes)| (final_url.to_string(), Bytes(bytes))),
            )
        }
        NetworkRequest::Post { url, form } => {
            let Ok(target) = Url::parse(&url) else {
                return invalid(&url);
            };
            if !matches!(target.scheme(), "http" | "https") {
                return refused(&target, "un formulaire ne s'envoie qu'au web.");
            }
            if !expected(&target) {
                return refused(&target, "navigation que le navigateur n'a pas demandée.");
            }
            commit(&target, loader::post_http(&target, &form))
        }
        NetworkRequest::Script(request) => {
            if !locked(lock).allows(&request.initiator) {
                return NetworkReply::Script(
                    Err("requête émise au nom d'un autre site".to_owned()),
                );
            }
            NetworkReply::Script(loader::fetch_for_script(&request))
        }
        NetworkRequest::Cookies { document } => NetworkReply::Cookies(
            Url::parse(&document)
                .ok()
                .filter(|document| locked(lock).allows(document))
                .and_then(|document| loader::read_cookies(&document))
                .unwrap_or_default(),
        ),
        NetworkRequest::SetCookie { document, text } => {
            if let Ok(document) = Url::parse(&document)
                && locked(lock).allows(&document)
            {
                loader::write_cookie(&document, &text);
            }
            NetworkReply::Done
        }
    }
}

/// Vrai si `url` se charge dans un processus de rendu : les pages du web et
/// les fichiers. Les pages internes (`about:`) restent dans le navigateur.
pub(crate) fn is_remote(url: &Url) -> bool {
    matches!(url.scheme(), "http" | "https" | "file" | "data") || url.as_str() == "about:crash"
}

/// Page d'un onglet.
#[derive(Debug)]
pub(crate) enum PageHost {
    /// Dans ce processus : pages internes, mode mono-processus, tests.
    Local(Box<Page>),
    /// Dans un processus de rendu.
    Remote(Box<RemotePage>),
}

impl From<Page> for PageHost {
    fn from(page: Page) -> Self {
        Self::Local(Box::new(page))
    }
}

impl PageHost {
    pub(crate) fn url(&self) -> &Url {
        match self {
            Self::Local(page) => page.url(),
            Self::Remote(page) => &page.url,
        }
    }

    pub(crate) fn title(&self) -> String {
        match self {
            Self::Local(page) => page.title(),
            Self::Remote(page) => page.title.clone(),
        }
    }

    pub(crate) fn display_list(&self) -> &DisplayList {
        match self {
            Self::Local(page) => page.display_list(),
            Self::Remote(page) => &page.list,
        }
    }

    pub(crate) fn scroll_offset(&self) -> (f32, f32) {
        self.scroll_of(ROOT_SCROLLER)
    }

    pub(crate) fn scroll_of(&self, id: u32) -> (f32, f32) {
        match self {
            Self::Local(page) => page.scroll_of(id),
            Self::Remote(page) => page.scroll.offset(id),
        }
    }

    /// Met la page à jour pour une zone d'affichage `viewport` ; renvoie
    /// `true` si sa liste d'affichage a changé.
    pub(crate) fn update(&mut self, text: &mut TextContext, viewport: Viewport) -> bool {
        match self {
            Self::Local(page) => page.update(text, viewport),
            Self::Remote(page) => {
                let size = (viewport.width, viewport.height);
                if size != page.viewport {
                    page.viewport = size;
                    page.scroll
                        .update(&page.list.scrollers, page.viewport, page.content);
                    page.send(&ToRenderer::Resize {
                        width: size.0,
                        height: size.1,
                    });
                }
                std::mem::take(&mut page.dirty)
            }
        }
    }

    /// Fait avancer la page jusqu'à `now` ; renvoie `true` si elle doit être
    /// redessinée.
    pub(crate) fn tick(&mut self, now: Instant) -> bool {
        match self {
            Self::Local(page) => {
                let redraw = page.tick(now);
                loader::report_console(page);
                redraw
            }
            Self::Remote(page) => page.pump(),
        }
    }

    pub(crate) fn next_wake(&mut self) -> Option<Instant> {
        match self {
            Self::Local(page) => page.next_wake(),
            // Le processus de rendu tient ses propres minuteurs et réveille
            // le navigateur quand il a une image.
            Self::Remote(_) => None,
        }
    }

    /// Molette au point (`x`, `y`) de la zone d'affichage.
    pub(crate) fn wheel(&mut self, x: f32, y: f32, dx: f32, dy: f32) -> bool {
        match self {
            Self::Local(page) => page.wheel(x, y, dx, dy),
            Self::Remote(page) => {
                let scrolled = page.scroll.wheel(x, y, dx, dy).is_some();
                page.sync_scroll();
                scrolled
            }
        }
    }

    /// Clic au point (`x`, `y`) de la page. Renvoie le lien à suivre s'il
    /// est connu sur le champ ; un processus de rendu le fait savoir plus
    /// tard, par [`PageHost::take_navigations`].
    pub(crate) fn click(&mut self, x: f32, y: f32) -> Option<Url> {
        match self {
            Self::Local(page) => {
                let link = page.click(x, y);
                loader::report_console(page);
                link
            }
            Self::Remote(page) => {
                page.send(&ToRenderer::Click { x, y });
                None
            }
        }
    }

    /// Vrai si le point (`x`, `y`) de la page est sur un lien. Pour un
    /// processus de rendu, la réponse est celle du dernier point connu.
    pub(crate) fn over_link(&self, x: f32, y: f32) -> bool {
        match self {
            Self::Local(page) => page.link_at(x, y).is_some(),
            Self::Remote(page) => {
                if page.hover.get() != Some((x, y)) && !page.crashed {
                    page.hover.set(Some((x, y)));
                    let _ = page.sender.send(&ToRenderer::Hover { x, y });
                }
                page.over_link
            }
        }
    }

    /// Transmet une touche ; renvoie (action par défaut annulée, page à
    /// redessiner).
    pub(crate) fn key_event(&mut self, down: bool, input: &KeyInput<'_>) -> (bool, bool) {
        match self {
            Self::Local(page) => {
                let outcome = page.key_event(down, input);
                loader::report_console(page);
                outcome
            }
            Self::Remote(page) => {
                page.send(&ToRenderer::Key(KeyMessage {
                    down,
                    key: input.key.to_owned(),
                    code: input.code.to_owned(),
                    ctrl: input.ctrl,
                    shift: input.shift,
                    alt: input.alt,
                    meta: input.meta,
                    repeat: input.repeat,
                }));
                (false, false)
            }
        }
    }

    /// Cherche `query` dans le texte de la page et surligne ses
    /// occurrences ; vide : efface la recherche. Pour une page distante,
    /// l'avancement arrive plus tard (voir [`PageHost::find_status`]).
    pub(crate) fn find(&mut self, query: &str) {
        match self {
            Self::Local(page) => {
                page.find(query);
            }
            Self::Remote(page) => {
                page.found = FindStatus::default();
                page.send(&ToRenderer::Find {
                    query: query.to_owned(),
                });
            }
        }
    }

    /// Passe à l'occurrence suivante ou précédente de la recherche.
    pub(crate) fn find_step(&mut self, forward: bool) {
        match self {
            Self::Local(page) => {
                page.find_step(forward);
            }
            Self::Remote(page) => page.send(&ToRenderer::FindStep { forward }),
        }
    }

    /// Avancement de la recherche dans la page.
    pub(crate) fn find_status(&self) -> FindStatus {
        match self {
            Self::Local(page) => page.find_status(),
            Self::Remote(page) => page.found,
        }
    }

    pub(crate) fn traverse(&mut self, url: &Url, delta: i32) {
        match self {
            Self::Local(page) => {
                page.traverse(url, delta);
            }
            Self::Remote(page) => {
                page.url = url.clone();
                page.send(&ToRenderer::Traverse {
                    url: url.to_string(),
                    delta,
                });
            }
        }
    }

    pub(crate) fn take_fetches(&mut self) -> Vec<oxyde_content::FetchRequest> {
        match self {
            Self::Local(page) => page.take_fetches(),
            // Un processus de rendu adresse ses requêtes par son canal.
            Self::Remote(_) => Vec::new(),
        }
    }

    pub(crate) fn complete_fetch(
        &mut self,
        id: u64,
        result: Result<oxyde_content::FetchResponse, String>,
    ) -> bool {
        match self {
            Self::Local(page) => {
                let redraw = page.complete_fetch(id, result);
                loader::report_console(page);
                redraw
            }
            Self::Remote(_) => false,
        }
    }

    pub(crate) fn take_navigations(&mut self) -> Vec<Navigation> {
        match self {
            Self::Local(page) => page.take_navigations(),
            Self::Remote(page) => std::mem::take(&mut page.navigations),
        }
    }

    /// Couleur de fond du canevas.
    pub(crate) fn background(&self) -> Color {
        match self {
            Self::Local(page) => page.background(),
            Self::Remote(page) => page.background,
        }
    }

    /// Page hébergée par un processus de rendu, si c'est le cas.
    pub(crate) fn remote_mut(&mut self) -> Option<&mut RemotePage> {
        match self {
            Self::Local(_) => None,
            Self::Remote(page) => Some(page),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lock_on(site: Option<&str>, expected: Option<&str>) -> Mutex<SiteLock> {
        let parse = |text: &str| Url::parse(text).expect("url");
        Mutex::new(SiteLock {
            site: site.map(parse),
            expected: expected.map(parse),
            redirected: None,
        })
    }

    fn refusal(reply: &NetworkReply) -> Option<&str> {
        match reply {
            NetworkReply::Resource(Err(error)) => Some(error.code.as_str()),
            _ => None,
        }
    }

    #[test]
    fn only_local_documents_may_read_files() {
        let path = std::env::current_exe().expect("exécutable de test");
        let file = Url::from_file_path(&path)
            .expect("chemin absolu")
            .to_string();
        let get = |initiator: &str| NetworkRequest::Get {
            url: file.clone(),
            initiator: Some(initiator.to_owned()),
        };
        // Une page du web : refus, sans toucher au disque.
        let web = lock_on(Some("https://exemple.test/"), None);
        let reply = answer(get("https://exemple.test/page"), &web);
        assert_eq!(refusal(&reply), Some("ERR_ACCESS_DENIED"));
        // Un document local : lecture accordée.
        let local = lock_on(Some("file:///C:/page.html"), None);
        match answer(get("file:///C:/page.html"), &local) {
            NetworkReply::Resource(Ok((url, Bytes(bytes)))) => {
                assert_eq!(url, file);
                assert!(!bytes.is_empty());
            }
            other => panic!("réponse inattendue : {other:?}"),
        }
        // Un `POST` ne vise que le web, et une adresse illisible est refusée.
        let post = NetworkRequest::Post {
            url: file.clone(),
            form: FormPost {
                content_type: "text/plain".to_owned(),
                body: Vec::new(),
            },
        };
        assert!(refusal(&answer(post, &local)).is_some());
        let garbage = NetworkRequest::Get {
            url: "pas une adresse".to_owned(),
            initiator: None,
        };
        assert_eq!(refusal(&answer(garbage, &local)), Some("ERR_INVALID_URL"));
    }

    #[test]
    fn a_renderer_only_acts_for_its_own_site() {
        let site = lock_on(Some("https://exemple.test/"), None);
        // Ressource demandée au nom d'un autre site : c'est ce nom qui
        // choisirait les cookies joints.
        let reply = answer(
            NetworkRequest::Get {
                url: "https://banque.test/compte".to_owned(),
                initiator: Some("https://banque.test/".to_owned()),
            },
            &site,
        );
        assert_eq!(refusal(&reply), Some("ERR_ACCESS_DENIED"));
        // Cookies d'un autre site : rien.
        let reply = answer(
            NetworkRequest::Cookies {
                document: "https://banque.test/".to_owned(),
            },
            &site,
        );
        assert!(matches!(reply, NetworkReply::Cookies(text) if text.is_empty()));
        // Requête de script au nom d'un autre site : refusée avant tout envoi.
        let url = Url::parse("https://banque.test/api").expect("url");
        let reply = answer(
            NetworkRequest::Script(Box::new(oxyde_content::FetchRequest {
                id: 1,
                url: url.clone(),
                method: "GET".to_owned(),
                headers: Vec::new(),
                body: Vec::new(),
                origin: None,
                initiator: url,
                needs_preflight: false,
            })),
            &site,
        );
        assert!(matches!(reply, NetworkReply::Script(Err(_))));
        // Un processus qui n'a encore rien chargé n'agit pour personne.
        let fresh = lock_on(None, None);
        let reply = answer(
            NetworkRequest::Get {
                url: "https://exemple.test/a.css".to_owned(),
                initiator: Some("https://exemple.test/".to_owned()),
            },
            &fresh,
        );
        assert_eq!(refusal(&reply), Some("ERR_ACCESS_DENIED"));
    }

    #[test]
    fn document_requests_follow_the_navigation_the_browser_asked_for() {
        let page = "data:text/html,<p>bonjour";
        let navigation = || NetworkRequest::Get {
            url: page.to_owned(),
            initiator: None,
        };
        // Sans navigation confiée : refus.
        let idle = lock_on(None, None);
        assert_eq!(
            refusal(&answer(navigation(), &idle)),
            Some("ERR_ACCESS_DENIED")
        );
        assert!(locked(&idle).site.is_none());
        // Navigation confiée : servie une fois, et elle fixe le site.
        let fresh = lock_on(None, Some(page));
        assert!(matches!(
            answer(navigation(), &fresh),
            NetworkReply::Resource(Ok(_))
        ));
        assert_eq!(locked(&fresh).site.as_ref().map(Url::as_str), Some(page));
        assert_eq!(
            refusal(&answer(navigation(), &fresh)),
            Some("ERR_ACCESS_DENIED")
        );
        // Un processus voué à un site ne reçoit pas le document d'un autre :
        // la navigation est mise de côté pour un autre processus.
        let busy = lock_on(Some("file:///C:/page.html"), Some(page));
        assert_eq!(
            refusal(&answer(navigation(), &busy)),
            Some("ERR_ACCESS_DENIED")
        );
        let lock = locked(&busy);
        assert_eq!(lock.redirected.as_ref().map(Url::as_str), Some(page));
        assert_eq!(
            lock.site.as_ref().map(Url::as_str),
            Some("file:///C:/page.html")
        );
    }

    #[test]
    fn web_pages_go_to_a_renderer_process() {
        let remote = |text: &str| is_remote(&Url::parse(text).expect("url"));
        assert!(remote("https://exemple.test/"));
        assert!(remote("file:///C:/page.html"));
        assert!(remote("about:crash"));
        assert!(!remote("about:newtab"));
    }
}
