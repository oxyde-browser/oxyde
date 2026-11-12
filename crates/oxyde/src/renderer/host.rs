//! Côté navigateur : la page d'un onglet, qu'elle vive dans ce processus
//! ([`PageHost::Local`]) ou dans un processus de rendu
//! ([`PageHost::Remote`]).
//!
//! Le processus de rendu n'est pas digne de confiance : ses messages sont
//! décodés avec méfiance, ses requêtes réseau sont filtrées (il ne lit un
//! fichier que s'il affiche lui-même un document `file:`), et sa disparition
//! n'emporte que son onglet.

use std::cell::Cell;
use std::process::{Child, Command};
use std::time::Instant;

use oxyde_compositor::{ROOT_SCROLLER, ScrollTree};
use oxyde_content::{FormPost, KeyInput, Navigation, Page};
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
    process: Child,
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
    crashed: bool,
    /// Vrai si le document affiché est un fichier local : seul cas où le
    /// processus de rendu peut demander des fichiers.
    file_access: bool,
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
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
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
        let executable = std::env::current_exe()?;
        let child = oxyde_ipc::spawn::<ToRenderer, FromRenderer>(
            Command::new(executable).arg("--type=renderer"),
            wake,
        )?;
        let mut page = Self {
            process: child.process,
            sender: child.sender,
            receiver: child.receiver,
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
            crashed: false,
            file_access: false,
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
        self.file_access = url.scheme() == "file";
        let message = ToRenderer::Load {
            url: url.to_string(),
            post,
            width: self.viewport.0,
            height: self.viewport.1,
        };
        self.send(&message);
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
        let file_access = self.file_access;
        let spawned = std::thread::Builder::new()
            .name("oxyde-reseau".to_owned())
            .spawn(move || {
                let reply = answer(request, file_access);
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
                            let _ = self.process.kill();
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
                    if let Ok(url) = Url::parse(&url) {
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
                                self.url = url;
                            }
                            self.title = frame.title;
                            self.dirty = true;
                            redraw = true;
                        }
                        Err(error) => {
                            eprintln!("processus de rendu écarté : image illisible ({error})");
                            let _ = self.process.kill();
                            self.crashed = true;
                            return true;
                        }
                    }
                }
                FromRenderer::Navigate(navigation) => self.navigations.push(navigation),
                FromRenderer::Clicked { url } => {
                    if let Ok(url) = Url::parse(&url) {
                        self.navigations.push(Navigation::Load {
                            url,
                            replace: false,
                            post: None,
                        });
                    }
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

    /// Transmet au processus de rendu les zones que le compositeur vient de
    /// faire défiler.
    fn sync_scroll(&mut self) {
        for id in self.scroll.take_changed() {
            let (x, y) = self.scroll.offset(id);
            self.send(&ToRenderer::Scroll { id, x, y });
        }
    }
}

/// Répond à une requête réseau d'un processus de rendu.
fn answer(request: NetworkRequest, file_access: bool) -> NetworkReply {
    let invalid = |url: &str| {
        NetworkReply::Resource(Err(LoadError::other(
            "Adresse invalide",
            url.to_owned(),
            "ERR_INVALID_URL",
        )))
    };
    // Un processus de rendu ne lit un fichier que pour un document local.
    let allowed = |url: &Url| url.scheme() != "file" || file_access;
    let refused = |url: &Url| {
        NetworkReply::Resource(Err(LoadError::other(
            "Accès refusé",
            format!("{url} — une page web ne peut pas lire un fichier local."),
            "ERR_ACCESS_DENIED",
        )))
    };
    match request {
        NetworkRequest::Get { url, initiator } => {
            let Ok(target) = Url::parse(&url) else {
                return invalid(&url);
            };
            if !allowed(&target) {
                return refused(&target);
            }
            let initiator = initiator.and_then(|i| Url::parse(&i).ok());
            NetworkReply::Resource(
                loader::fetch_from(&target, initiator.as_ref())
                    .map(|(final_url, bytes)| (final_url.to_string(), Bytes(bytes))),
            )
        }
        NetworkRequest::Post { url, form } => {
            let Ok(target) = Url::parse(&url) else {
                return invalid(&url);
            };
            if !matches!(target.scheme(), "http" | "https") {
                return refused(&target);
            }
            NetworkReply::Resource(
                loader::post_http(&target, &form)
                    .map(|(final_url, bytes)| (final_url.to_string(), Bytes(bytes))),
            )
        }
        NetworkRequest::Script(request) => NetworkReply::Script(loader::fetch_for_script(&request)),
        NetworkRequest::Cookies { document } => NetworkReply::Cookies(
            Url::parse(&document)
                .ok()
                .and_then(|document| loader::read_cookies(&document))
                .unwrap_or_default(),
        ),
        NetworkRequest::SetCookie { document, text } => {
            if let Ok(document) = Url::parse(&document) {
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

    #[test]
    fn only_local_documents_may_read_files() {
        let path = std::env::current_exe().expect("exécutable de test");
        let file = Url::from_file_path(&path)
            .expect("chemin absolu")
            .to_string();
        let get = || NetworkRequest::Get {
            url: file.clone(),
            initiator: None,
        };
        // Une page du web : refus, sans toucher au disque.
        match answer(get(), false) {
            NetworkReply::Resource(Err(error)) => assert_eq!(error.code, "ERR_ACCESS_DENIED"),
            other => panic!("réponse inattendue : {other:?}"),
        }
        // Un document local : lecture accordée.
        match answer(get(), true) {
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
        assert!(matches!(answer(post, true), NetworkReply::Resource(Err(_))));
        let garbage = NetworkRequest::Get {
            url: "pas une adresse".to_owned(),
            initiator: None,
        };
        match answer(garbage, true) {
            NetworkReply::Resource(Err(error)) => assert_eq!(error.code, "ERR_INVALID_URL"),
            other => panic!("réponse inattendue : {other:?}"),
        }
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
