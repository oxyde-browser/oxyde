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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError, mpsc};

use oxyde_ipc::{Bytes, ChannelError, Receiver, Sender, connect_to_parent, ipc_enum, ipc_struct};
use oxyde_net::{Client, HttpRequest, NetError};
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

/// Appel au service réseau.
#[derive(Debug)]
enum Call {
    Send(WireRequest),
    Cookies { document: String },
    SetCookie { document: String, text: String },
}
ipc_enum!(Call {
    0 => Send(request),
    1 => Cookies { document },
    2 => SetCookie { document, text },
});

/// Réponse du service réseau.
#[derive(Debug)]
enum Answer {
    Response(Result<WireResponse, NetFailure>),
    Cookies(String),
    Done,
}
ipc_enum!(Answer {
    0 => Response(result),
    1 => Cookies(text),
    2 => Done,
});

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

/// Sert un appel dans ce processus.
fn serve_here(call: Call) -> Answer {
    match call {
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
