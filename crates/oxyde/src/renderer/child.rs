//! Processus de rendu : il héberge une page — chargement, scripts, style,
//! mise en page, peinture — et n'a d'autre lien avec le monde que le canal
//! vers le processus navigateur. Il n'ouvre ni fichier ni connexion : tout
//! ce qu'il lui faut du réseau, il le demande au navigateur.

use std::collections::{HashMap, VecDeque};
use std::process::ExitCode;
use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant};

use oxyde_content::{KeyInput, Page};
use oxyde_css::Viewport;
use oxyde_ipc::{Bytes, ChannelError, Receiver, Sender, connect_to_parent};
use oxyde_paint::wire::ResourceSender;
use oxyde_text::TextContext;
use url::Url;

use super::protocol::{Frame, FromRenderer, NetworkReply, NetworkRequest, ToRenderer};
use crate::loader::{self, LoadError};

/// Lien du processus de rendu avec le navigateur.
pub(crate) struct Link {
    sender: Sender<FromRenderer>,
    state: Mutex<LinkState>,
}

struct LinkState {
    receiver: Receiver<ToRenderer>,
    /// Messages reçus pendant l'attente d'une réponse réseau.
    backlog: VecDeque<ToRenderer>,
    next_id: u64,
}

static LINK: OnceLock<Link> = OnceLock::new();

/// Lien avec le navigateur, si ce processus est un processus de rendu.
pub(crate) fn link() -> Option<&'static Link> {
    LINK.get()
}

impl Link {
    fn state(&self) -> std::sync::MutexGuard<'_, LinkState> {
        // Le processus de rendu n'a qu'un fil : le verrou n'est jamais
        // disputé, et un empoisonnement ne laisse rien d'incohérent.
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn send(&self, message: &FromRenderer) -> Result<(), ChannelError> {
        self.sender.send(message)
    }

    /// Émet une requête réseau sans attendre sa réponse ; renvoie son
    /// identifiant.
    fn post(&self, request: NetworkRequest) -> Result<u64, ChannelError> {
        let id = {
            let mut state = self.state();
            state.next_id = state.next_id.wrapping_add(1);
            state.next_id
        };
        self.send(&FromRenderer::Network { id, request })?;
        Ok(id)
    }

    /// Émet une requête réseau et attend sa réponse. Les autres messages
    /// reçus entre-temps sont mis de côté pour la boucle principale.
    fn call(&self, request: NetworkRequest) -> Result<NetworkReply, ChannelError> {
        let id = self.post(request)?;
        let mut state = self.state();
        loop {
            match state.receiver.recv()? {
                ToRenderer::Network {
                    id: answered,
                    reply,
                } if answered == id => return Ok(reply),
                other => state.backlog.push_back(other),
            }
        }
    }

    /// Prochain message du navigateur ; `timeout` : durée d'attente
    /// maximale, sans limite si `None`. `Ok(None)` si rien n'est arrivé.
    fn next_message(&self, timeout: Option<Duration>) -> Result<Option<ToRenderer>, ChannelError> {
        let mut state = self.state();
        if let Some(message) = state.backlog.pop_front() {
            return Ok(Some(message));
        }
        match timeout {
            Some(timeout) => state.receiver.recv_timeout(timeout),
            None => state.receiver.recv().map(Some),
        }
    }

    /// Récupère une ressource par l'entremise du navigateur.
    pub(crate) fn get(
        &self,
        url: &Url,
        initiator: Option<&Url>,
    ) -> Result<(Url, Vec<u8>), LoadError> {
        self.resource(
            url,
            NetworkRequest::Get {
                url: url.to_string(),
                initiator: initiator.map(Url::to_string),
            },
        )
    }

    /// Demande une page par `POST` par l'entremise du navigateur.
    pub(crate) fn post_form(
        &self,
        url: &Url,
        form: &oxyde_content::FormPost,
    ) -> Result<(Url, Vec<u8>), LoadError> {
        self.resource(
            url,
            NetworkRequest::Post {
                url: url.to_string(),
                form: form.clone(),
            },
        )
    }

    fn resource(&self, url: &Url, request: NetworkRequest) -> Result<(Url, Vec<u8>), LoadError> {
        let broken = |detail: String| {
            LoadError::other("La page n'a pas pu être chargée", detail, "ERR_FAILED")
        };
        match self.call(request) {
            Ok(NetworkReply::Resource(Ok((final_url, Bytes(bytes))))) => {
                let final_url = Url::parse(&final_url)
                    .map_err(|e| broken(format!("{url} — adresse finale invalide : {e}")))?;
                Ok((final_url, bytes))
            }
            Ok(NetworkReply::Resource(Err(error))) => Err(error),
            Ok(_) => Err(broken(format!("{url} — réponse inattendue du navigateur"))),
            Err(error) => Err(broken(format!("{url} — {error}"))),
        }
    }

    /// Valeur de `document.cookie` pour `document`.
    pub(crate) fn cookies(&self, document: &Url) -> Option<String> {
        match self.call(NetworkRequest::Cookies {
            document: document.to_string(),
        }) {
            Ok(NetworkReply::Cookies(text)) => Some(text),
            _ => None,
        }
    }

    /// Soumet un cookie écrit par un script de `document`.
    pub(crate) fn set_cookie(&self, document: &Url, text: &str) {
        let _ = self.call(NetworkRequest::SetCookie {
            document: document.to_string(),
            text: text.to_owned(),
        });
    }
}

/// État du processus de rendu.
struct Renderer {
    link: &'static Link,
    text: TextContext,
    page: Option<Page>,
    viewport: Viewport,
    resources: ResourceSender,
    /// Décalages de défilement connus du navigateur.
    known_scroll: HashMap<u32, (f32, f32)>,
    /// Avancement de la recherche connu du navigateur.
    sent_found: oxyde_content::FindStatus,
    /// Requêtes `fetch` des scripts en cours : identifiant réseau → requête.
    fetches: HashMap<u64, u64>,
    /// Vrai si une image doit partir même sans changement de la page.
    force_frame: bool,
    /// Dernier titre et dernière adresse envoyés.
    sent_meta: (String, String),
}

impl Renderer {
    fn handle(&mut self, message: ToRenderer) -> Result<(), ChannelError> {
        match message {
            ToRenderer::Load {
                url,
                post,
                width,
                height,
            } => {
                self.viewport = Viewport { width, height };
                loader::set_viewport_hint(width, height);
                let Ok(url) = Url::parse(&url) else {
                    return Ok(());
                };
                // Adresse de diagnostic : fait planter le processus, pour
                // vérifier que le navigateur y survit.
                if url.as_str() == "about:crash" {
                    std::process::abort();
                }
                let loaded = loader::load_with(url, post);
                self.link.send(&FromRenderer::Committed {
                    url: loaded.page.url().to_string(),
                    error: loaded.error,
                })?;
                self.page = Some(loaded.page);
                self.known_scroll.clear();
                self.sent_found = oxyde_content::FindStatus::default();
                self.fetches.clear();
                self.force_frame = true;
            }
            ToRenderer::Resize { width, height } => {
                self.viewport = Viewport { width, height };
                loader::set_viewport_hint(width, height);
            }
            ToRenderer::Click { x, y } => {
                if let Some(page) = self.page.as_mut() {
                    let link = page.click(x, y);
                    loader::report_console(page);
                    if let Some(url) = link {
                        self.link.send(&FromRenderer::Clicked {
                            url: url.to_string(),
                        })?;
                    }
                }
            }
            ToRenderer::Hover { x, y } => {
                let link = self
                    .page
                    .as_ref()
                    .is_some_and(|page| page.link_at(x, y).is_some());
                self.link.send(&FromRenderer::Hover { link })?;
            }
            ToRenderer::Key(key) => {
                if let Some(page) = self.page.as_mut() {
                    page.key_event(
                        key.down,
                        &KeyInput {
                            key: &key.key,
                            code: &key.code,
                            ctrl: key.ctrl,
                            shift: key.shift,
                            alt: key.alt,
                            meta: key.meta,
                            repeat: key.repeat,
                        },
                    );
                    loader::report_console(page);
                }
            }
            ToRenderer::Scroll { id, x, y } => {
                if let Some(page) = self.page.as_mut() {
                    page.set_scroll(id, x, y);
                    // Le navigateur connaît ce décalage : inutile de le lui
                    // renvoyer, même si la page l'a borné autrement.
                    let actual = page.scroll_of(id);
                    self.known_scroll.insert(id, actual);
                }
            }
            ToRenderer::Find { query } => {
                if let Some(page) = self.page.as_mut() {
                    page.find(&query);
                    // Le surlignage change sans nouvelle mise en page.
                    self.force_frame = true;
                }
            }
            ToRenderer::FindStep { forward } => {
                if let Some(page) = self.page.as_mut() {
                    page.find_step(forward);
                    self.force_frame = true;
                }
            }
            ToRenderer::Traverse { url, delta } => {
                if let (Some(page), Ok(url)) = (self.page.as_mut(), Url::parse(&url)) {
                    page.traverse(&url, delta);
                    loader::report_console(page);
                }
            }
            ToRenderer::Network { id, reply } => {
                // Réponse à une requête `fetch` d'un script.
                if let (Some(fetch), Some(page)) = (self.fetches.remove(&id), self.page.as_mut()) {
                    let result = match reply {
                        NetworkReply::Script(result) => result,
                        _ => Err("réponse inattendue du navigateur".to_owned()),
                    };
                    page.complete_fetch(fetch, result);
                    loader::report_console(page);
                }
            }
        }
        Ok(())
    }

    /// Fait avancer la page jusqu'à maintenant et envoie au navigateur ce
    /// qui a changé.
    fn advance(&mut self) -> Result<(), ChannelError> {
        let Some(page) = self.page.as_mut() else {
            return Ok(());
        };
        page.tick(Instant::now());
        loader::report_console(page);
        for request in page.take_fetches() {
            let fetch = request.id;
            let id = self.link.post(NetworkRequest::Script(Box::new(request)))?;
            self.fetches.insert(id, fetch);
        }
        for navigation in page.take_navigations() {
            self.link.send(&FromRenderer::Navigate(navigation))?;
        }
        let repainted = page.update(&mut self.text, self.viewport);
        // L'avancement de la recherche, quand il a changé (nouvelle
        // recherche, occurrence suivante, page remise en page).
        let found = page.find_status();
        if found != self.sent_found {
            self.sent_found = found;
            self.link.send(&FromRenderer::Found {
                count: u32::try_from(found.count).unwrap_or(u32::MAX),
                current: u32::try_from(found.current).unwrap_or(u32::MAX),
            })?;
        }
        // Zones que les scripts ont fait défiler à l'insu du navigateur.
        let mut scroll = Vec::new();
        let current: HashMap<u32, (f32, f32)> = page
            .scroll_offsets()
            .into_iter()
            .map(|(id, x, y)| (id, (x, y)))
            .collect();
        for (id, offset) in &current {
            if self.known_scroll.get(id) != Some(offset) {
                scroll.push((*id, offset.0, offset.1));
            }
        }
        for id in self.known_scroll.keys() {
            if !current.contains_key(id) {
                scroll.push((*id, 0.0, 0.0));
            }
        }
        let meta = (page.url().to_string(), page.title());
        if !repainted && !self.force_frame && scroll.is_empty() && meta == self.sent_meta {
            return Ok(());
        }
        self.known_scroll = current;
        self.force_frame = false;
        self.sent_meta = meta.clone();
        let background = page.background();
        let (content_w, content_h) = page.content_size();
        let frame = Frame {
            list: self.resources.encode(page.display_list()),
            content: [content_w, content_h],
            background: [background.r, background.g, background.b, background.a],
            scroll,
            url: meta.0,
            title: meta.1,
        };
        self.link.send(&FromRenderer::Frame(Box::new(frame)))
    }

    fn run(&mut self) -> Result<(), ChannelError> {
        loop {
            // Attente : jusqu'au prochain minuteur de la page, ou sans
            // limite si rien n'est prévu.
            let timeout = self
                .page
                .as_mut()
                .and_then(Page::next_wake)
                .map(|due| due.saturating_duration_since(Instant::now()));
            if let Some(message) = self.link.next_message(timeout)? {
                self.handle(message)?;
                // Les messages déjà arrivés sont traités avant de peindre.
                while let Some(message) = self.link.next_message(Some(Duration::ZERO))? {
                    self.handle(message)?;
                }
            }
            self.advance()?;
        }
    }
}

/// Point d'entrée du processus de rendu (`oxyde --type=renderer`). Il se
/// termine quand le navigateur ferme le canal.
pub(crate) fn run() -> ExitCode {
    // Avant tout contenu : le processus renonce à ce dont il n'a pas besoin.
    let refused = oxyde_sandbox::harden();
    if !refused.is_empty() {
        eprintln!(
            "processus de rendu : protections refusées par le système : {}",
            refused.join(", ")
        );
    }
    let (sender, receiver) = connect_to_parent::<FromRenderer, ToRenderer>();
    let link = LINK.get_or_init(|| Link {
        sender,
        state: Mutex::new(LinkState {
            receiver,
            backlog: VecDeque::new(),
            next_id: 0,
        }),
    });
    let mut renderer = Renderer {
        link,
        text: TextContext::new(),
        page: None,
        viewport: Viewport {
            width: 0.0,
            height: 0.0,
        },
        resources: ResourceSender::new(),
        known_scroll: HashMap::new(),
        sent_found: oxyde_content::FindStatus::default(),
        fetches: HashMap::new(),
        force_frame: false,
        sent_meta: (String::new(), String::new()),
    };
    match renderer.run() {
        // Le navigateur a fermé le canal : fin normale.
        Ok(()) | Err(ChannelError::Closed) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("processus de rendu : {error}");
            ExitCode::FAILURE
        }
    }
}
