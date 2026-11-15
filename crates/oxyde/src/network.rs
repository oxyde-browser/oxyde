//! Service réseau : tout ce qui parle aux serveurs (HTTP, TLS, cookies).
//!
//! En mode multi-processus, il vit dans un processus à part
//! (`oxyde --type=network`), lancé dans le bac à sable : le code qui analyse
//! les octets venus du réseau n'a pas la main sur le navigateur. Le
//! navigateur lui adresse ses requêtes par un canal ; s'il disparaît, les
//! requêtes en cours échouent et un nouveau processus est lancé à la
//! suivante.
//!
//! Sinon (mode mono-processus, tests, capture d'écran), les mêmes appels
//! sont servis sur place.
//!
//! LIMITE CONNUE : les cookies vivent en mémoire dans le service ; ils sont
//! perdus si son processus s'arrête.

use std::collections::HashMap;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError, mpsc};

use oxyde_ipc::{Bytes, ChannelError, Receiver, Sender, connect_to_parent, ipc_enum, ipc_struct};
use oxyde_net::{Client, HttpRequest, NetError, SocketEvent, SocketHandle, StreamEvent};
use url::Url;

/// Nature d'un échec réseau, pour le choix de la page d'erreur.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FailureClass {
    /// Site injoignable (connexion, délai).
    Unreachable,
    /// Négociation TLS refusée.
    Tls,
    /// Redirections en boucle.
    Redirects,
    /// Autre.
    Other,
}
ipc_enum!(FailureClass {
    0 => Unreachable,
    1 => Tls,
    2 => Redirects,
    3 => Other,
});

/// Échec réseau, sous une forme qui traverse les processus.
#[derive(Clone, Debug)]
pub(crate) struct NetFailure {
    /// Code au format de Chromium (`ERR_…`).
    pub(crate) code: String,
    /// Vrai si le certificat du serveur a été refusé.
    pub(crate) certificate: bool,
    pub(crate) class: FailureClass,
    /// Description technique.
    pub(crate) text: String,
}
ipc_struct!(NetFailure {
    code,
    certificate,
    class,
    text
});

impl NetFailure {
    fn other(text: &str) -> Self {
        Self {
            code: "ERR_FAILED".to_owned(),
            certificate: false,
            class: FailureClass::Other,
            text: text.to_owned(),
        }
    }
}

impl From<&NetError> for NetFailure {
    fn from(error: &NetError) -> Self {
        Self {
            code: error.code().to_owned(),
            certificate: error.is_certificate_error(),
            class: match error {
                NetError::Connect { .. } | NetError::Timeout => FailureClass::Unreachable,
                NetError::Tls { .. } => FailureClass::Tls,
                NetError::TooManyRedirects => FailureClass::Redirects,
                _ => FailureClass::Other,
            },
            text: error.to_string(),
        }
    }
}

/// Réponse HTTP.
#[derive(Clone, Debug)]
pub(crate) struct NetResponse {
    /// Adresse finale, après redirections.
    pub(crate) url: Url,
    pub(crate) status: u16,
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) body: Vec<u8>,
}

/// Requête HTTP sur le fil.
#[derive(Debug)]
struct WireRequest {
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    body: Bytes,
    initiator: Option<String>,
}
ipc_struct!(WireRequest {
    method,
    url,
    headers,
    body,
    initiator
});

/// Réponse HTTP sur le fil.
#[derive(Debug)]
struct WireResponse {
    url: String,
    status: u16,
    headers: Vec<(String, String)>,
    body: Bytes,
}
ipc_struct!(WireResponse {
    url,
    status,
    headers,
    body
});

/// Ce qui arrive à une connexion WebSocket, sur le fil.
#[derive(Debug)]
enum SocketNews {
    Open {
        protocol: String,
    },
    Text {
        text: String,
    },
    Binary {
        bytes: Bytes,
    },
    Close {
        code: u16,
        reason: String,
        clean: bool,
    },
}
ipc_enum!(SocketNews {
    0 => Open { protocol },
    1 => Text { text },
    2 => Binary { bytes },
    3 => Close { code, reason, clean },
});

impl From<SocketEvent> for SocketNews {
    fn from(event: SocketEvent) -> Self {
        match event {
            SocketEvent::Open { protocol } => Self::Open { protocol },
            SocketEvent::Text(text) => Self::Text { text },
            SocketEvent::Binary(bytes) => Self::Binary {
                bytes: Bytes(bytes),
            },
            SocketEvent::Close {
                code,
                reason,
                clean,
            } => Self::Close {
                code,
                reason,
                clean,
            },
        }
    }
}

impl From<SocketNews> for SocketEvent {
    fn from(news: SocketNews) -> Self {
        match news {
            SocketNews::Open { protocol } => Self::Open { protocol },
            SocketNews::Text { text } => Self::Text(text),
            SocketNews::Binary { bytes } => Self::Binary(bytes.0),
            SocketNews::Close {
                code,
                reason,
                clean,
            } => Self::Close {
                code,
                reason,
                clean,
            },
        }
    }
}

/// Appel au service réseau.
#[derive(Debug)]
enum Call {
    Send(WireRequest),
    Cookies {
        document: String,
    },
    SetCookie {
        document: String,
        text: String,
    },
    /// Ouvrir la connexion WebSocket `socket` ; ce qui lui arrive revient
    /// par des [`Answer::Socket`].
    SocketOpen {
        socket: u64,
        url: String,
        document: String,
        protocols: Vec<String>,
    },
    SocketText {
        socket: u64,
        text: String,
    },
    SocketBinary {
        socket: u64,
        bytes: Bytes,
    },
    SocketClose {
        socket: u64,
        code: Option<u16>,
        reason: String,
    },
    /// Plus personne n'écoute la connexion `socket` : la couper.
    SocketDrop {
        socket: u64,
    },
}
ipc_enum!(Call {
    0 => Send(request),
    1 => Cookies { document },
    2 => SetCookie { document, text },
    3 => SocketOpen { socket, url, document, protocols },
    4 => SocketText { socket, text },
    5 => SocketBinary { socket, bytes },
    6 => SocketClose { socket, code, reason },
    7 => SocketDrop { socket },
});

/// Réponse du service réseau.
#[derive(Debug)]
enum Answer {
    Response(Result<WireResponse, NetFailure>),
    Cookies(String),
    Done,
    /// Pas une réponse : une nouvelle de la connexion WebSocket `socket`,
    /// envoyée avec l'identifiant d'appel [`SOCKET_NEWS`].
    Socket {
        socket: u64,
        news: SocketNews,
    },
}
ipc_enum!(Answer {
    0 => Response(result),
    1 => Cookies(text),
    2 => Done,
    3 => Socket { socket, news },
});

/// Identifiant d'appel réservé aux nouvelles des connexions WebSocket ;
/// les appels sont numérotés à partir de 1.
const SOCKET_NEWS: u64 = 0;

#[derive(Debug)]
struct ToNetwork {
    id: u64,
    call: Call,
}
ipc_struct!(ToNetwork { id, call });

#[derive(Debug)]
struct FromNetwork {
    id: u64,
    answer: Answer,
}
ipc_struct!(FromNetwork { id, answer });

/// Client HTTP du processus : créé à la première requête servie sur place.
fn client() -> Result<&'static Client, NetFailure> {
    static CLIENT: OnceLock<Result<Client, String>> = OnceLock::new();
    CLIENT
        .get_or_init(|| Client::new().map_err(|e| e.to_string()))
        .as_ref()
        .map_err(|e| NetFailure::other(e))
}

/// Connexions WebSocket ouvertes par ce processus, par identifiant.
fn served_sockets() -> &'static Mutex<HashMap<u64, SocketHandle>> {
    static SERVED: OnceLock<Mutex<HashMap<u64, SocketHandle>>> = OnceLock::new();
    SERVED.get_or_init(Mutex::default)
}

/// Flux d'événements (`EventSource`) lus par ce processus : de quoi les
/// abandonner, par identifiant.
fn served_streams() -> &'static Mutex<HashMap<u64, Arc<AtomicBool>>> {
    static STREAMS: OnceLock<Mutex<HashMap<u64, Arc<AtomicBool>>>> = OnceLock::new();
    STREAMS.get_or_init(Mutex::default)
}

/// Abandonne le flux d'événements `socket`, s'il est lu ici ; vrai s'il
/// l'était.
fn cancel_stream(socket: u64) -> bool {
    let cancelled = served_streams()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .remove(&socket);
    match cancelled {
        Some(flag) => {
            flag.store(true, Ordering::Relaxed);
            true
        }
        None => false,
    }
}

/// Lit, sur un fil dédié, le flux d'événements `url` pour le document
/// `document`, et l'annonce comme une connexion : ouverture si la réponse
/// est bien un flux d'événements, morceaux, puis fermeture (voir
/// [`oxyde_content::SocketCommand`] pour le sens des codes).
fn serve_stream(socket: u64, url: Url, document: Url, last_event_id: Option<String>) {
    let cancelled = Arc::new(AtomicBool::new(false));
    served_streams()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(socket, Arc::clone(&cancelled));
    let close = |code: u16, clean: bool| SocketEvent::Close {
        code,
        reason: String::new(),
        clean,
    };
    let spawned = std::thread::Builder::new()
        .name("oxyde-flux".to_owned())
        .spawn(move || {
            let Ok(client) = client() else {
                cancel_stream(socket);
                return announce(socket, close(1006, false));
            };
            let cross_origin = url.origin() != document.origin();
            let origin = document.origin().ascii_serialization();
            let mut headers = vec![
                ("accept".to_owned(), "text/event-stream".to_owned()),
                ("cache-control".to_owned(), "no-cache".to_owned()),
            ];
            if let Some(last) = last_event_id {
                headers.push(("last-event-id".to_owned(), last));
            }
            if cross_origin {
                headers.push(("origin".to_owned(), origin.clone()));
            }
            let request = HttpRequest {
                headers,
                initiator: Some(document),
                ..HttpRequest::get(&url)
            };
            let flag = Arc::clone(&cancelled);
            client.stream(&request, &cancelled, &mut |event| match event {
                StreamEvent::Head {
                    status, headers, ..
                } => {
                    let header = |wanted: &str| {
                        headers
                            .iter()
                            .find(|(name, _)| name.eq_ignore_ascii_case(wanted))
                            .map(|(_, value)| value.as_str())
                    };
                    let event_stream = header("content-type").is_some_and(|kind| {
                        kind.split(';')
                            .next()
                            .unwrap_or_default()
                            .trim()
                            .eq_ignore_ascii_case("text/event-stream")
                    });
                    // Une autre origine doit autoriser la lecture.
                    let allowed = !cross_origin
                        || header("access-control-allow-origin")
                            .is_some_and(|value| value == "*" || value == origin);
                    if status == 200 && event_stream && allowed {
                        announce(
                            socket,
                            SocketEvent::Open {
                                protocol: String::new(),
                            },
                        );
                    } else {
                        // Refus définitif : le code d'état s'il dit
                        // pourquoi, sinon une faute de protocole.
                        flag.store(true, Ordering::Relaxed);
                        cancel_stream(socket);
                        let code = if status == 200 { 1002 } else { status };
                        announce(socket, close(code, false));
                    }
                }
                StreamEvent::Chunk(bytes) => announce(socket, SocketEvent::Binary(bytes)),
                StreamEvent::End => {
                    cancel_stream(socket);
                    announce(socket, close(1000, true));
                }
                StreamEvent::Failed(_) => {
                    cancel_stream(socket);
                    announce(socket, close(1006, false));
                }
            });
        });
    if spawned.is_err() {
        cancel_stream(socket);
        announce(socket, close(1006, false));
    }
}

/// Dans le processus réseau : de quoi envoyer au navigateur les nouvelles
/// des connexions WebSocket. Absent quand le réseau est servi sur place.
static NEWS_TO_BROWSER: OnceLock<Sender<FromNetwork>> = OnceLock::new();

/// Annonce ce qui arrive à la connexion `socket` à qui l'a ouverte.
fn announce(socket: u64, event: SocketEvent) {
    if matches!(event, SocketEvent::Close { .. }) {
        // Dernière nouvelle : la connexion est oubliée.
        served_sockets()
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&socket);
    }
    match NEWS_TO_BROWSER.get() {
        Some(sender) => {
            let answer = Answer::Socket {
                socket,
                news: event.into(),
            };
            // Le navigateur a pu disparaître entre-temps.
            let _ = sender.send(&FromNetwork {
                id: SOCKET_NEWS,
                answer,
            });
        }
        None => deliver(socket, event),
    }
}

/// Ouvre la connexion WebSocket `socket` dans ce processus.
fn serve_socket(socket: u64, url: &str, document: &str, protocols: &[String]) {
    let failed = || {
        announce(
            socket,
            SocketEvent::Close {
                code: 1006,
                reason: String::new(),
                clean: false,
            },
        );
    };
    let (Ok(url), Ok(document), Ok(client)) = (Url::parse(url), Url::parse(document), client())
    else {
        return failed();
    };
    if matches!(url.scheme(), "http" | "https") {
        // Un flux d'événements, annoncé comme une connexion.
        return serve_stream(socket, url, document, protocols.first().cloned());
    }
    if !matches!(url.scheme(), "ws" | "wss") {
        return failed();
    }
    // Le verrou est tenu pendant l'ouverture : la première nouvelle ne
    // peut pas précéder l'enregistrement de la connexion.
    let mut served = served_sockets()
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let handle = client.open_socket(&url, &document, protocols, move |event| {
        announce(socket, event);
    });
    served.insert(socket, handle);
}

/// Applique `action` à la connexion `socket`, si elle est ouverte ici.
fn with_socket(socket: u64, action: impl FnOnce(&SocketHandle)) {
    let served = served_sockets()
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if let Some(handle) = served.get(&socket) {
        action(handle);
    }
}

/// Sert un appel dans ce processus.
fn serve_here(call: Call) -> Answer {
    match call {
        Call::SocketOpen {
            socket,
            url,
            document,
            protocols,
        } => {
            serve_socket(socket, &url, &document, &protocols);
            Answer::Done
        }
        Call::SocketText { socket, text } => {
            with_socket(socket, |handle| handle.send_text(text));
            Answer::Done
        }
        Call::SocketBinary { socket, bytes } => {
            with_socket(socket, |handle| handle.send_binary(bytes.0));
            Answer::Done
        }
        Call::SocketClose {
            socket,
            code,
            reason,
        } => {
            if cancel_stream(socket) {
                // Un flux abandonné se tait : sa fermeture est annoncée ici.
                announce(
                    socket,
                    SocketEvent::Close {
                        code: 1000,
                        reason: String::new(),
                        clean: true,
                    },
                );
            } else {
                with_socket(socket, |handle| handle.close(code, reason));
            }
            Answer::Done
        }
        Call::SocketDrop { socket } => {
            cancel_stream(socket);
            // Lâcher la poignée coupe la connexion.
            let dropped = served_sockets()
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .remove(&socket);
            drop(dropped);
            Answer::Done
        }
        Call::Send(request) => Answer::Response((|| {
            let url = Url::parse(&request.url)
                .map_err(|e| NetFailure::other(&format!("adresse invalide : {e}")))?;
            // Seul le web passe par ici : les fichiers restent au navigateur.
            if !matches!(url.scheme(), "http" | "https") {
                return Err(NetFailure::other("protocole refusé par le service réseau"));
            }
            let initiator = match request.initiator {
                Some(text) => Some(
                    Url::parse(&text)
                        .map_err(|e| NetFailure::other(&format!("émetteur invalide : {e}")))?,
                ),
                None => None,
            };
            let response = client()?
                .send(&HttpRequest {
                    method: request.method,
                    url,
                    headers: request.headers,
                    body: request.body.0,
                    initiator,
                })
                .map_err(|e| NetFailure::from(&e))?;
            Ok(WireResponse {
                url: response.url.to_string(),
                status: response.status,
                headers: response.headers,
                body: Bytes(response.body),
            })
        })()),
        Call::Cookies { document } => Answer::Cookies(
            Url::parse(&document)
                .ok()
                .and_then(|document| Some(client().ok()?.cookies_for_script(&document)))
                .unwrap_or_default(),
        ),
        Call::SetCookie { document, text } => {
            if let (Ok(document), Ok(client)) = (Url::parse(&document), client()) {
                client.set_cookie_from_script(&document, &text);
            }
            Answer::Done
        }
    }
}

/// Appels en attente de leur réponse.
type Pending = Arc<Mutex<HashMap<u64, mpsc::Sender<Answer>>>>;

/// Lien du navigateur avec le processus réseau.
struct Connection {
    sender: Sender<ToNetwork>,
    pending: Pending,
    /// Faux une fois le canal fermé.
    alive: Arc<AtomicBool>,
    next_id: u64,
    /// Gardé pour l'arrêter avec le navigateur.
    process: oxyde_sandbox::Process,
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.process.kill();
        self.process.wait();
    }
}

impl Connection {
    fn open() -> std::io::Result<Self> {
        let executable = std::env::current_exe()?;
        let arguments = ["--type=network"];
        let policy = oxyde_sandbox::Policy::Networked;
        let spawned = match oxyde_sandbox::spawn(&executable, &arguments, policy) {
            Ok(spawned) => spawned,
            Err(error) => {
                eprintln!("bac à sable indisponible, processus réseau lancé sans lui : {error}");
                oxyde_sandbox::spawn_unsandboxed(&executable, &arguments)?
            }
        };
        let pending: Pending = Arc::default();
        let alive = Arc::new(AtomicBool::new(true));
        let mut receiver: Receiver<FromNetwork> = Receiver::new(spawned.stdout, || {});
        let (waiting, living) = (Arc::clone(&pending), Arc::clone(&alive));
        // Ce fil rend chaque réponse à l'appel qui l'attend.
        std::thread::Builder::new()
            .name("oxyde-reseau-reponses".to_owned())
            .spawn(move || {
                while let Ok(message) = receiver.recv() {
                    if message.id == SOCKET_NEWS {
                        if let Answer::Socket { socket, news } = message.answer {
                            deliver(socket, news.into());
                        }
                        continue;
                    }
                    let caller = waiting
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .remove(&message.id);
                    if let Some(caller) = caller {
                        // L'appelant a pu renoncer : sa réponse est perdue.
                        let _ = caller.send(message.answer);
                    }
                }
                // Canal fermé : les appels en attente échouent, leur
                // émetteur disparaissant avec la table.
                living.store(false, Ordering::SeqCst);
                // Les connexions WebSocket qu'il portait sont coupées.
                fail_all_sockets();
                waiting
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .clear();
            })?;
        Ok(Self {
            sender: Sender::new(spawned.stdin),
            pending,
            alive,
            next_id: 0,
            process: spawned.process,
        })
    }

    /// Émet `call` ; la réponse arrivera sur le récepteur rendu.
    fn post(&mut self, call: Call) -> Result<mpsc::Receiver<Answer>, ChannelError> {
        self.next_id = self.next_id.wrapping_add(1);
        let id = self.next_id;
        let (caller, answer) = mpsc::channel();
        self.pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(id, caller);
        if let Err(error) = self.sender.send(&ToNetwork { id, call }) {
            self.pending
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .remove(&id);
            return Err(error);
        }
        Ok(answer)
    }
}

/// Vrai si le service réseau vit dans un processus à part.
static OUT_OF_PROCESS: AtomicBool = AtomicBool::new(false);

/// Confie désormais le réseau à un processus à part.
pub(crate) fn use_process() {
    OUT_OF_PROCESS.store(true, Ordering::Relaxed);
}

fn connection() -> &'static Mutex<Option<Connection>> {
    static CONNECTION: OnceLock<Mutex<Option<Connection>>> = OnceLock::new();
    CONNECTION.get_or_init(|| Mutex::new(None))
}

/// Adresse un appel au service réseau et attend sa réponse.
fn call(call: Call) -> Result<Answer, NetFailure> {
    if !OUT_OF_PROCESS.load(Ordering::Relaxed) {
        return Ok(serve_here(call));
    }
    let gone = || NetFailure::other("le processus réseau s'est arrêté");
    let answer = {
        let mut slot = connection().lock().unwrap_or_else(PoisonError::into_inner);
        // Processus absent ou disparu : on en lance un.
        if slot
            .as_ref()
            .is_none_or(|open| !open.alive.load(Ordering::SeqCst))
        {
            match Connection::open() {
                Ok(open) => *slot = Some(open),
                Err(error) => {
                    // Sans processus réseau, le navigateur sert lui-même
                    // ses requêtes plutôt que de ne rien charger.
                    eprintln!(
                        "processus réseau indisponible, réseau servi par le navigateur : {error}"
                    );
                    OUT_OF_PROCESS.store(false, Ordering::Relaxed);
                    drop(slot);
                    return Ok(serve_here(call));
                }
            }
        }
        let open = slot.as_mut().ok_or_else(gone)?;
        open.post(call).map_err(|_| gone())?
        // Le verrou est rendu ici : les appels s'attendent en parallèle.
    };
    answer.recv().map_err(|_| gone())
}

/// Destinataire des nouvelles d'une connexion WebSocket.
type Listener = Box<dyn FnMut(SocketEvent) + Send>;

/// Destinataires des connexions WebSocket ouvertes par ce processus.
fn listeners() -> &'static Mutex<HashMap<u64, Listener>> {
    static LISTENERS: OnceLock<Mutex<HashMap<u64, Listener>>> = OnceLock::new();
    LISTENERS.get_or_init(Mutex::default)
}

/// Remet `event` au destinataire de la connexion `socket`. Les nouvelles
/// d'une même connexion arrivent l'une après l'autre : le destinataire
/// est sorti de la table le temps de l'appel, pour ne pas tenir le verrou
/// pendant qu'il travaille.
fn deliver(socket: u64, event: SocketEvent) {
    let last = matches!(event, SocketEvent::Close { .. });
    let taken = listeners()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .remove(&socket);
    let Some(mut listener) = taken else {
        return;
    };
    listener(event);
    if !last {
        listeners()
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entry(socket)
            .or_insert(listener);
    }
}

/// Le processus réseau a disparu : toutes les connexions échouent.
fn fail_all_sockets() {
    let sockets: Vec<u64> = listeners()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .keys()
        .copied()
        .collect();
    for socket in sockets {
        deliver(
            socket,
            SocketEvent::Close {
                code: 1006,
                reason: String::new(),
                clean: false,
            },
        );
    }
}

/// Connexion WebSocket ouverte par [`open_socket`]. La lâcher la coupe.
#[derive(Debug)]
pub(crate) struct Socket {
    id: u64,
}

impl Socket {
    /// Envoie un message texte.
    pub(crate) fn send_text(&self, text: String) {
        let _ = call(Call::SocketText {
            socket: self.id,
            text,
        });
    }

    /// Envoie un message binaire.
    pub(crate) fn send_binary(&self, bytes: Vec<u8>) {
        let _ = call(Call::SocketBinary {
            socket: self.id,
            bytes: Bytes(bytes),
        });
    }

    /// Demande la fermeture.
    pub(crate) fn close(&self, code: Option<u16>, reason: String) {
        let _ = call(Call::SocketClose {
            socket: self.id,
            code,
            reason,
        });
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        listeners()
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&self.id);
        let _ = call(Call::SocketDrop { socket: self.id });
    }
}

/// Ouvre une connexion WebSocket vers `url` au nom du document `document`.
/// `on_event` reçoit, depuis un autre fil, ce qui lui arrive ; la dernière
/// nouvelle est toujours une fermeture.
pub(crate) fn open_socket(
    url: &Url,
    document: &Url,
    protocols: &[String],
    on_event: impl FnMut(SocketEvent) + Send + 'static,
) -> Socket {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let id = NEXT.fetch_add(1, Ordering::Relaxed) + 1;
    listeners()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(id, Box::new(on_event));
    let opened = call(Call::SocketOpen {
        socket: id,
        url: url.to_string(),
        document: document.to_string(),
        protocols: protocols.to_vec(),
    });
    if opened.is_err() {
        deliver(
            id,
            SocketEvent::Close {
                code: 1006,
                reason: String::new(),
                clean: false,
            },
        );
    }
    Socket { id }
}

/// Envoie une requête HTTP et suit ses redirections.
pub(crate) fn send(request: &HttpRequest) -> Result<NetResponse, NetFailure> {
    let wire = WireRequest {
        method: request.method.clone(),
        url: request.url.to_string(),
        headers: request.headers.clone(),
        body: Bytes(request.body.clone()),
        initiator: request.initiator.as_ref().map(Url::to_string),
    };
    match call(Call::Send(wire))? {
        Answer::Response(Ok(response)) => Ok(NetResponse {
            url: Url::parse(&response.url)
                .map_err(|e| NetFailure::other(&format!("adresse finale invalide : {e}")))?,
            status: response.status,
            headers: response.headers,
            body: response.body.0,
        }),
        Answer::Response(Err(failure)) => Err(failure),
        _ => Err(NetFailure::other("réponse inattendue du service réseau")),
    }
}

/// Valeur de `document.cookie` pour `document`.
pub(crate) fn cookies(document: &Url) -> Option<String> {
    match call(Call::Cookies {
        document: document.to_string(),
    }) {
        Ok(Answer::Cookies(text)) => Some(text),
        _ => None,
    }
}

/// Soumet un cookie écrit par un script de `document`.
pub(crate) fn set_cookie(document: &Url, text: &str) {
    let _ = call(Call::SetCookie {
        document: document.to_string(),
        text: text.to_owned(),
    });
}

/// Point d'entrée du processus réseau (`oxyde --type=network`). Il se
/// termine quand le navigateur ferme le canal.
pub(crate) fn run() -> ExitCode {
    let refused = oxyde_sandbox::harden();
    if !refused.is_empty() {
        eprintln!(
            "processus réseau : protections refusées par le système : {}",
            refused.join(", ")
        );
    }
    let (sender, mut receiver) = connect_to_parent::<FromNetwork, ToNetwork>();
    // Les nouvelles des connexions WebSocket repartent par ce canal.
    let _ = NEWS_TO_BROWSER.set(sender.clone());
    loop {
        let message = match receiver.recv() {
            Ok(message) => message,
            Err(ChannelError::Closed) => return ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("processus réseau : {error}");
                return ExitCode::FAILURE;
            }
        };
        // Un fil par requête : une réponse lente ne retient pas les autres.
        let reply_to = sender.clone();
        let id = message.id;
        let spawned = std::thread::Builder::new()
            .name("oxyde-requete".to_owned())
            .spawn(move || {
                let answer = serve_here(message.call);
                // Le navigateur a pu disparaître entre-temps.
                let _ = reply_to.send(&FromNetwork { id, answer });
            });
        if spawned.is_err() {
            let answer = Answer::Response(Err(NetFailure::other(
                "aucun fil disponible pour la requête",
            )));
            let _ = sender.send(&FromNetwork { id, answer });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxyde_ipc::{from_bytes, to_bytes};

    #[test]
    fn failures_keep_what_error_pages_need() {
        let failure = NetFailure::from(&NetError::TooManyRedirects);
        assert_eq!(failure.code, "ERR_TOO_MANY_REDIRECTS");
        assert_eq!(failure.class, FailureClass::Redirects);
        assert!(!failure.certificate);
        let failure = NetFailure::from(&NetError::Timeout);
        assert_eq!(failure.class, FailureClass::Unreachable);
        // Et traversent le fil.
        let back: NetFailure = from_bytes(&to_bytes(&failure)).expect("décodage");
        assert_eq!(back.code, "ERR_TIMED_OUT");
        assert_eq!(back.class, FailureClass::Unreachable);
    }

    #[test]
    fn sockets_report_to_their_listener() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("écoute");
        let port = listener.local_addr().expect("adresse").port();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("connexion");
            let mut request = Vec::new();
            let mut byte = [0u8; 1];
            while !request.ends_with(b"\r\n\r\n") {
                stream.read_exact(&mut byte).expect("requête");
                request.push(byte[0]);
            }
            let text = String::from_utf8_lossy(&request).into_owned();
            let key = text
                .lines()
                .find_map(|line| line.strip_prefix("Sec-WebSocket-Key: "))
                .expect("clé")
                .trim()
                .to_owned();
            let response = format!(
                "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\n\
                 Connection: Upgrade\r\nSec-WebSocket-Accept: {}\r\n\r\n",
                oxyde_net::websocket::accept_for(&key)
            );
            stream.write_all(response.as_bytes()).expect("réponse");
            // Un message texte, puis la fermeture (code 1000).
            stream
                .write_all(&[0x81, 5, b's', b'a', b'l', b'u', b't', 0x88, 2, 0x03, 0xE8])
                .expect("trames");
            // La réponse du client à la fermeture.
            let mut reply = [0u8; 8];
            let _ = stream.read(&mut reply);
        });
        let (sender, events) = mpsc::channel();
        let url = Url::parse(&format!("ws://127.0.0.1:{port}/")).expect("url");
        let document = Url::parse("http://127.0.0.1/page").expect("url");
        let socket = open_socket(&url, &document, &[], move |event| {
            let _ = sender.send(event);
        });
        let next = || {
            events
                .recv_timeout(std::time::Duration::from_secs(10))
                .expect("événement")
        };
        assert_eq!(
            next(),
            SocketEvent::Open {
                protocol: String::new()
            }
        );
        assert_eq!(next(), SocketEvent::Text("salut".to_owned()));
        assert_eq!(
            next(),
            SocketEvent::Close {
                code: 1000,
                reason: String::new(),
                clean: true
            }
        );
        // Après la fermeture, le destinataire est oublié ; écrire ne fait rien.
        for _ in 0..200 {
            if !listeners().lock().expect("verrou").contains_key(&socket.id) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(!listeners().lock().expect("verrou").contains_key(&socket.id));
        socket.send_text("dans le vide".to_owned());
        // Une adresse refusée échoue sans rien ouvrir.
        let (sender, events) = mpsc::channel();
        let bad = Url::parse("http://127.0.0.1/").expect("url");
        let _refused = open_socket(&bad, &document, &[], move |event| {
            let _ = sender.send(event);
        });
        assert!(matches!(
            events.recv_timeout(std::time::Duration::from_secs(10)),
            Ok(SocketEvent::Close {
                code: 1006,
                clean: false,
                ..
            })
        ));
    }

    #[test]
    fn event_streams_are_announced_like_sockets() {
        use std::io::{Read, Write};
        // Sert une réponse par connexion : un flux, puis une page ordinaire.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("écoute");
        let port = listener.local_addr().expect("adresse").port();
        std::thread::spawn(move || {
            for stream in listener.incoming().take(2) {
                let mut stream = stream.expect("connexion");
                let mut request = Vec::new();
                let mut byte = [0u8; 1];
                while !request.ends_with(b"\r\n\r\n") {
                    stream.read_exact(&mut byte).expect("requête");
                    request.push(byte[0]);
                }
                let text = String::from_utf8_lossy(&request).into_owned();
                if text.starts_with("GET /flux ") {
                    assert!(text.contains("last-event-id: 7\r\n"), "{text}");
                    assert!(text.contains("accept: text/event-stream\r\n"), "{text}");
                    stream
                        .write_all(
                            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream; charset=utf-8\r\n\
                              Content-Length: 10\r\nConnection: close\r\n\r\ndata: un\n\n",
                        )
                        .expect("flux");
                } else {
                    stream
                        .write_all(
                            b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\
                              Content-Length: 2\r\nConnection: close\r\n\r\nok",
                        )
                        .expect("page");
                }
            }
        });
        let document = Url::parse(&format!("http://127.0.0.1:{port}/page")).expect("url");
        let listen = |path: &str, last: &[String]| {
            let (sender, events) = mpsc::channel();
            let url = Url::parse(&format!("http://127.0.0.1:{port}{path}")).expect("url");
            let socket = open_socket(&url, &document, last, move |event| {
                let _ = sender.send(event);
            });
            (socket, events)
        };
        let next = |events: &mpsc::Receiver<SocketEvent>| {
            events
                .recv_timeout(std::time::Duration::from_secs(10))
                .expect("événement")
        };
        let (_flux, events) = listen("/flux", &["7".to_owned()]);
        assert_eq!(
            next(&events),
            SocketEvent::Open {
                protocol: String::new()
            }
        );
        assert_eq!(next(&events), SocketEvent::Binary(b"data: un\n\n".to_vec()));
        assert_eq!(
            next(&events),
            SocketEvent::Close {
                code: 1000,
                reason: String::new(),
                clean: true
            }
        );
        // Une réponse qui n'est pas un flux d'événements : refus définitif.
        let (_page, events) = listen("/page", &[]);
        assert_eq!(
            next(&events),
            SocketEvent::Close {
                code: 1002,
                reason: String::new(),
                clean: false
            }
        );
    }

    #[test]
    fn service_only_speaks_http() {
        let call = Call::Send(WireRequest {
            method: "GET".to_owned(),
            url: "file:///C:/Windows/win.ini".to_owned(),
            headers: Vec::new(),
            body: Bytes(Vec::new()),
            initiator: None,
        });
        match serve_here(call) {
            Answer::Response(Err(failure)) => assert!(failure.text.contains("protocole refusé")),
            other => panic!("réponse inattendue : {other:?}"),
        }
        let call = Call::Send(WireRequest {
            method: "GET".to_owned(),
            url: "pas une adresse".to_owned(),
            headers: Vec::new(),
            body: Bytes(Vec::new()),
            initiator: None,
        });
        assert!(matches!(serve_here(call), Answer::Response(Err(_))));
    }
}
