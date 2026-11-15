//! Client WebSocket (RFC 6455) : poignée de main HTTP/1.1, trames, masquage,
//! ping et pong, fermeture.
//!
//! Une connexion vit dans deux tâches de la boucle du [`Client`] : l'une lit
//! les trames du serveur et les annonce par des [`SocketEvent`], l'autre écrit
//! ce que le [`SocketHandle`] demande. Lâcher le [`SocketHandle`] coupe la
//! connexion.
//!
//! LIMITES CONNUES : aucune extension n'est proposée (pas de compression
//! `permessage-deflate`) ; pas de WebSocket sur HTTP/2 ; pas de proxy ; les
//! redirections de la poignée de main ne sont pas suivies.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use rustls::pki_types::ServerName;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use url::Url;

use crate::{Access, Client, USER_AGENT, same_site};

/// Valeur fixe que la norme fait ajouter à la clé avant de la condenser.
const ACCEPT_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
/// Taille maximale de la réponse à la poignée de main.
const MAX_HANDSHAKE_BYTES: usize = 16 * 1024;
/// Taille maximale d'un message reçu (64 Mio).
const MAX_MESSAGE_BYTES: usize = 64 * 1024 * 1024;
/// Délai accordé à l'ouverture de la connexion.
const OPEN_TIMEOUT: Duration = Duration::from_secs(30);
/// Délai accordé au serveur pour répondre à notre trame de fermeture.
const CLOSE_TIMEOUT: Duration = Duration::from_secs(3);

const OP_CONTINUATION: u8 = 0x0;
const OP_TEXT: u8 = 0x1;
const OP_BINARY: u8 = 0x2;
const OP_CLOSE: u8 = 0x8;
const OP_PING: u8 = 0x9;
const OP_PONG: u8 = 0xA;

/// Ce qui arrive à une connexion WebSocket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SocketEvent {
    /// La poignée de main a réussi.
    Open {
        /// Sous-protocole retenu par le serveur, vide s'il n'y en a pas.
        protocol: String,
    },
    /// Message texte.
    Text(String),
    /// Message binaire.
    Binary(Vec<u8>),
    /// La connexion est finie ; aucun événement ne suit.
    Close {
        /// Code de fermeture (1005 : aucun ; 1006 : coupure sans fermeture).
        code: u16,
        /// Raison donnée par le serveur.
        reason: String,
        /// Vrai si la fermeture s'est faite dans les règles.
        clean: bool,
    },
}

/// Ordre donné à la tâche d'écriture.
enum Command {
    Text(String),
    Binary(Vec<u8>),
    /// Fermeture demandée par la page.
    Close {
        code: Option<u16>,
        reason: String,
    },
    /// Réponse à un ping du serveur.
    Pong(Vec<u8>),
    /// Réponse à la fermeture demandée par le serveur.
    EchoClose(Option<u16>),
    /// Le serveur n'a pas répondu à notre fermeture.
    CloseTimedOut,
    /// Plus personne n'écoute : tout couper.
    Abort,
}

/// Poignée d'une connexion WebSocket ouverte par [`Client::open_socket`].
#[derive(Debug)]
pub struct SocketHandle {
    commands: mpsc::UnboundedSender<Command>,
}

impl SocketHandle {
    /// Envoie un message texte.
    pub fn send_text(&self, text: String) {
        let _ = self.commands.send(Command::Text(text));
    }

    /// Envoie un message binaire.
    pub fn send_binary(&self, bytes: Vec<u8>) {
        let _ = self.commands.send(Command::Binary(bytes));
    }

    /// Demande la fermeture, avec un code et une raison (123 octets au plus,
    /// le reste est coupé).
    pub fn close(&self, code: Option<u16>, reason: String) {
        let _ = self.commands.send(Command::Close { code, reason });
    }
}

impl Drop for SocketHandle {
    fn drop(&mut self) {
        let _ = self.commands.send(Command::Abort);
    }
}

impl std::fmt::Debug for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Text(_) => "Text",
            Self::Binary(_) => "Binary",
            Self::Close { .. } => "Close",
            Self::Pong(_) => "Pong",
            Self::EchoClose(_) => "EchoClose",
            Self::CloseTimedOut => "CloseTimedOut",
            Self::Abort => "Abort",
        })
    }
}

/// Flux d'octets d'une connexion, chiffrée ou non.
trait Stream: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Stream for T {}

/// Destinataire des événements : après `Close`, plus rien ne part.
struct Emitter {
    on_event: Box<dyn FnMut(SocketEvent) + Send>,
    closed: bool,
}

impl Emitter {
    fn emit(&mut self, event: SocketEvent) {
        if self.closed {
            return;
        }
        self.closed = matches!(event, SocketEvent::Close { .. });
        (self.on_event)(event);
    }
}

type SharedEmitter = Arc<Mutex<Emitter>>;

fn emit(emitter: &SharedEmitter, event: SocketEvent) {
    if let Ok(mut emitter) = emitter.lock() {
        emitter.emit(event);
    }
}

/// Coupure sans fermeture dans les règles.
fn abnormal() -> SocketEvent {
    SocketEvent::Close {
        code: 1006,
        reason: String::new(),
        clean: false,
    }
}

impl Client {
    /// Ouvre une connexion WebSocket vers `url` (`ws:` ou `wss:`) au nom du
    /// document `document`, en proposant les sous-protocoles `protocols`.
    /// `on_event` est appelée depuis un fil du client pour chaque événement ;
    /// le dernier est toujours [`SocketEvent::Close`], même si la connexion
    /// n'a jamais pu s'ouvrir.
    pub fn open_socket<F>(
        &self,
        url: &Url,
        document: &Url,
        protocols: &[String],
        on_event: F,
    ) -> SocketHandle
    where
        F: FnMut(SocketEvent) + Send + 'static,
    {
        let (commands, receiver) = mpsc::unbounded_channel();
        let emitter: SharedEmitter = Arc::new(Mutex::new(Emitter {
            on_event: Box::new(on_event),
            closed: false,
        }));
        // Les cookies du site accompagnent la poignée de main, comme pour une
        // requête HTTP vers la même adresse.
        let mut http_url = url.clone();
        let http_scheme = if url.scheme() == "wss" {
            "https"
        } else {
            "http"
        };
        let cookie = if http_url.set_scheme(http_scheme).is_ok() {
            let cross_site = !same_site(document, &http_url);
            self.jar
                .lock()
                .ok()
                .and_then(|mut jar| jar.header_for(&http_url, Access::Http, cross_site))
        } else {
            None
        };
        let request = Handshake {
            url: url.clone(),
            origin: document.origin().ascii_serialization(),
            protocols: protocols.to_vec(),
            cookie,
            tls: self.tls_http1.clone(),
        };
        let sender = commands.clone();
        self.runtime.spawn(async move {
            let opened = tokio::time::timeout(OPEN_TIMEOUT, request.connect()).await;
            let (stream, leftover, protocol) = match opened {
                Ok(Ok(opened)) => opened,
                Ok(Err(_)) | Err(_) => {
                    emit(&emitter, abnormal());
                    return;
                }
            };
            emit(&emitter, SocketEvent::Open { protocol });
            let (reader, writer) = tokio::io::split(stream);
            let reading = tokio::spawn(read_loop(
                reader,
                leftover,
                Arc::clone(&emitter),
                sender.clone(),
            ));
            write_loop(writer, receiver, sender, emitter, reading.abort_handle()).await;
        });
        SocketHandle { commands }
    }
}

/// Paramètres d'une poignée de main.
struct Handshake {
    url: Url,
    origin: String,
    protocols: Vec<String>,
    cookie: Option<String>,
    tls: tokio_rustls::TlsConnector,
}

impl Handshake {
    /// Se connecte et négocie : rend le flux, les octets déjà lus après la
    /// réponse et le sous-protocole retenu.
    async fn connect(self) -> Result<(Box<dyn Stream>, Vec<u8>, String), String> {
        let secure = match self.url.scheme() {
            "wss" => true,
            "ws" => false,
            other => return Err(format!("schéma « {other} » non pris en charge")),
        };
        let host = self.url.host_str().ok_or("adresse sans hôte")?.to_owned();
        let port = self.url.port().unwrap_or(if secure { 443 } else { 80 });
        let connect_host = host
            .trim_start_matches('[')
            .trim_end_matches(']')
            .to_owned();
        let tcp = TcpStream::connect((connect_host.as_str(), port))
            .await
            .map_err(|e| e.to_string())?;
        let _ = tcp.set_nodelay(true);
        let mut stream: Box<dyn Stream> = if secure {
            let name = ServerName::try_from(connect_host).map_err(|e| e.to_string())?;
            Box::new(
                self.tls
                    .connect(name, tcp)
                    .await
                    .map_err(|e| e.to_string())?,
            )
        } else {
            Box::new(tcp)
        };

        let mut nonce = [0u8; 16];
        getrandom::fill(&mut nonce).map_err(|e| e.to_string())?;
        let key = base64(&nonce);
        let authority = match self.url.port() {
            Some(port) => format!("{host}:{port}"),
            None => host,
        };
        let path = match self.url.query() {
            Some(query) => format!("{}?{query}", self.url.path()),
            None => self.url.path().to_owned(),
        };
        let mut request = format!(
            "GET {path} HTTP/1.1\r\nHost: {authority}\r\nUpgrade: websocket\r\n\
             Connection: Upgrade\r\nSec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\
             Origin: {}\r\nUser-Agent: {USER_AGENT}\r\n",
            self.origin
        );
        if !self.protocols.is_empty() {
            request.push_str("Sec-WebSocket-Protocol: ");
            request.push_str(&self.protocols.join(", "));
            request.push_str("\r\n");
        }
        if let Some(cookie) = &self.cookie {
            request.push_str("Cookie: ");
            request.push_str(cookie);
            request.push_str("\r\n");
        }
        request.push_str("\r\n");
        stream
            .write_all(request.as_bytes())
            .await
            .map_err(|e| e.to_string())?;

        // Réponse : jusqu'à la ligne vide.
        let mut received = Vec::new();
        let end = loop {
            if let Some(at) = received.windows(4).position(|w| w == b"\r\n\r\n") {
                break at;
            }
            if received.len() > MAX_HANDSHAKE_BYTES {
                return Err("réponse à la poignée de main trop longue".to_owned());
            }
            let mut chunk = [0u8; 2048];
            let read = stream.read(&mut chunk).await.map_err(|e| e.to_string())?;
            if read == 0 {
                return Err("connexion fermée pendant la poignée de main".to_owned());
            }
            received.extend_from_slice(chunk.get(..read).unwrap_or_default());
        };
        let head = String::from_utf8_lossy(received.get(..end).unwrap_or_default()).into_owned();
        let leftover = received.get(end + 4..).unwrap_or_default().to_vec();
        let protocol = check_response(&head, &key, &self.protocols)?;
        Ok((stream, leftover, protocol))
    }
}

/// Valeur de l'en-tête `Sec-WebSocket-Accept` qu'un serveur doit rendre pour
/// la clé `key` reçue dans `Sec-WebSocket-Key`.
#[must_use]
pub fn accept_for(key: &str) -> String {
    base64(&sha1(format!("{key}{ACCEPT_GUID}").as_bytes()))
}

/// Vérifie la réponse `head` du serveur à la poignée de main de clé `key` ;
/// rend le sous-protocole retenu.
fn check_response(head: &str, key: &str, protocols: &[String]) -> Result<String, String> {
    let mut lines = head.split("\r\n");
    let status = lines.next().unwrap_or_default();
    let mut parts = status.split(' ');
    if !parts.next().is_some_and(|v| v.starts_with("HTTP/1.1")) || parts.next() != Some("101") {
        return Err(format!("le serveur a refusé la connexion : {status}"));
    }
    let headers: Vec<(String, &str)> = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim()))
        .collect();
    let header = |wanted: &str| {
        headers
            .iter()
            .find(|(name, _)| name == wanted)
            .map(|(_, value)| *value)
    };
    if !header("upgrade").is_some_and(|v| v.eq_ignore_ascii_case("websocket")) {
        return Err("en-tête Upgrade absent ou invalide".to_owned());
    }
    let upgrades = header("connection").is_some_and(|v| {
        v.split(',')
            .any(|token| token.trim().eq_ignore_ascii_case("upgrade"))
    });
    if !upgrades {
        return Err("en-tête Connection absent ou invalide".to_owned());
    }
    let expected = accept_for(key);
    if header("sec-websocket-accept") != Some(expected.as_str()) {
        return Err("en-tête Sec-WebSocket-Accept absent ou invalide".to_owned());
    }
    // Aucune extension n'a été proposée : le serveur n'en choisit pas.
    if header("sec-websocket-extensions").is_some_and(|v| !v.is_empty()) {
        return Err("le serveur impose une extension non proposée".to_owned());
    }
    match header("sec-websocket-protocol") {
        None | Some("") => Ok(String::new()),
        Some(chosen) if protocols.iter().any(|offered| offered == chosen) => Ok(chosen.to_owned()),
        Some(chosen) => Err(format!("sous-protocole non proposé : {chosen}")),
    }
}

/// Trame reçue.
struct Frame {
    fin: bool,
    opcode: u8,
    payload: Vec<u8>,
}

/// Lit une trame du serveur ; `Err` porte le code de fermeture à annoncer
/// (1002 : faute de protocole ; 1009 : message trop gros), ou `None` si la
/// connexion est coupée.
async fn read_frame(
    reader: &mut ReadHalf<Box<dyn Stream>>,
    buffer: &mut Vec<u8>,
) -> Result<Frame, Option<u16>> {
    async fn fill(
        reader: &mut ReadHalf<Box<dyn Stream>>,
        buffer: &mut Vec<u8>,
        wanted: usize,
    ) -> Result<(), Option<u16>> {
        while buffer.len() < wanted {
            let mut chunk = [0u8; 8192];
            match reader.read(&mut chunk).await {
                Ok(0) | Err(_) => return Err(None),
                Ok(read) => buffer.extend_from_slice(chunk.get(..read).unwrap_or_default()),
            }
        }
        Ok(())
    }
    fill(reader, buffer, 2).await?;
    let (first, second) = match buffer.as_slice() {
        [first, second, ..] => (*first, *second),
        _ => return Err(None),
    };
    let fin = first & 0x80 != 0;
    let opcode = first & 0x0f;
    // Bits réservés sans extension, ou trame masquée par le serveur : fautes.
    if first & 0x70 != 0 || second & 0x80 != 0 {
        return Err(Some(1002));
    }
    let short = usize::from(second & 0x7f);
    let (header, length) = match short {
        126 => {
            fill(reader, buffer, 4).await?;
            let bytes = buffer.get(2..4).and_then(|b| <[u8; 2]>::try_from(b).ok());
            (4, usize::from(u16::from_be_bytes(bytes.ok_or(None)?)))
        }
        127 => {
            fill(reader, buffer, 10).await?;
            let bytes = buffer.get(2..10).and_then(|b| <[u8; 8]>::try_from(b).ok());
            let length = u64::from_be_bytes(bytes.ok_or(None)?);
            (10, usize::try_from(length).map_err(|_| Some(1009))?)
        }
        short => (2, short),
    };
    let control = opcode & 0x8 != 0;
    if control && (!fin || length > 125) {
        return Err(Some(1002));
    }
    if length > MAX_MESSAGE_BYTES {
        return Err(Some(1009));
    }
    let end = header + length;
    fill(reader, buffer, end).await?;
    let payload = buffer.get(header..end).ok_or(None)?.to_vec();
    buffer.drain(..end);
    Ok(Frame {
        fin,
        opcode,
        payload,
    })
}

/// Tâche de lecture : annonce les messages du serveur jusqu'à la fermeture.
async fn read_loop(
    mut reader: ReadHalf<Box<dyn Stream>>,
    mut buffer: Vec<u8>,
    emitter: SharedEmitter,
    commands: mpsc::UnboundedSender<Command>,
) {
    // Message en cours de réception, fragment après fragment.
    let mut partial: Option<(u8, Vec<u8>)> = None;
    let failure = loop {
        let frame = match read_frame(&mut reader, &mut buffer).await {
            Ok(frame) => frame,
            Err(code) => break code,
        };
        match frame.opcode {
            OP_TEXT | OP_BINARY | OP_CONTINUATION => {
                let (kind, mut message) = match (frame.opcode, partial.take()) {
                    (OP_CONTINUATION, Some(started)) => started,
                    (OP_TEXT | OP_BINARY, None) => (frame.opcode, Vec::new()),
                    // Suite sans début, ou début au milieu d'un message.
                    _ => break Some(1002),
                };
                if message.len().saturating_add(frame.payload.len()) > MAX_MESSAGE_BYTES {
                    break Some(1009);
                }
                message.extend_from_slice(&frame.payload);
                if !frame.fin {
                    partial = Some((kind, message));
                } else if kind == OP_TEXT {
                    match String::from_utf8(message) {
                        Ok(text) => emit(&emitter, SocketEvent::Text(text)),
                        Err(_) => break Some(1007),
                    }
                } else {
                    emit(&emitter, SocketEvent::Binary(message));
                }
            }
            OP_PING => {
                let _ = commands.send(Command::Pong(frame.payload));
            }
            OP_PONG => {}
            OP_CLOSE => {
                let (code, reason) = match frame.payload.as_slice() {
                    [] => (None, String::new()),
                    [high, low, reason @ ..] => {
                        let code = u16::from_be_bytes([*high, *low]);
                        let valid = matches!(code, 1000..=1003 | 1007..=1011 | 3000..=4999);
                        match (valid, std::str::from_utf8(reason)) {
                            (true, Ok(reason)) => (Some(code), reason.to_owned()),
                            _ => break Some(1002),
                        }
                    }
                    [_] => break Some(1002),
                };
                let _ = commands.send(Command::EchoClose(code));
                emit(
                    &emitter,
                    SocketEvent::Close {
                        code: code.unwrap_or(1005),
                        reason,
                        clean: true,
                    },
                );
                return;
            }
            _ => break Some(1002),
        }
    };
    // Faute de protocole ou coupure : la connexion échoue.
    if let Some(code) = failure {
        let _ = commands.send(Command::Close {
            code: Some(code),
            reason: String::new(),
        });
    }
    emit(&emitter, abnormal());
    let _ = commands.send(Command::Abort);
}

/// Écrit la trame `opcode` de contenu `payload`, masquée comme l'exige la
/// norme pour un client.
async fn write_frame(
    writer: &mut WriteHalf<Box<dyn Stream>>,
    opcode: u8,
    payload: &[u8],
) -> std::io::Result<()> {
    let mut frame = Vec::with_capacity(payload.len() + 14);
    frame.push(0x80 | opcode);
    match payload.len() {
        length @ 0..=125 => frame.push(0x80 | u8::try_from(length).unwrap_or(125)),
        length @ 126..=0xffff => {
            frame.push(0x80 | 126);
            frame.extend_from_slice(&u16::try_from(length).unwrap_or(u16::MAX).to_be_bytes());
        }
        length => {
            frame.push(0x80 | 127);
            frame.extend_from_slice(&u64::try_from(length).unwrap_or(u64::MAX).to_be_bytes());
        }
    }
    let mut mask = [0u8; 4];
    getrandom::fill(&mut mask).map_err(|e| std::io::Error::other(e.to_string()))?;
    frame.extend_from_slice(&mask);
    frame.extend(
        payload
            .iter()
            .zip(mask.iter().cycle())
            .map(|(byte, key)| byte ^ key),
    );
    writer.write_all(&frame).await?;
    writer.flush().await
}

/// Contenu d'une trame de fermeture : le code, puis la raison coupée à ce
/// que la trame peut porter, sur une frontière de caractère.
fn close_payload(code: Option<u16>, reason: &str) -> Vec<u8> {
    let Some(code) = code else {
        return Vec::new();
    };
    let mut end = reason.len().min(123);
    while !reason.is_char_boundary(end) {
        end -= 1;
    }
    let mut payload = code.to_be_bytes().to_vec();
    payload.extend_from_slice(reason.get(..end).unwrap_or_default().as_bytes());
    payload
}

/// Tâche d'écriture : exécute les ordres jusqu'à la fin de la connexion.
async fn write_loop(
    mut writer: WriteHalf<Box<dyn Stream>>,
    mut commands: mpsc::UnboundedReceiver<Command>,
    sender: mpsc::UnboundedSender<Command>,
    emitter: SharedEmitter,
    reading: tokio::task::AbortHandle,
) {
    let mut close_sent = false;
    while let Some(command) = commands.recv().await {
        let written = match command {
            Command::Text(text) if !close_sent => {
                write_frame(&mut writer, OP_TEXT, text.as_bytes()).await
            }
            Command::Binary(bytes) if !close_sent => {
                write_frame(&mut writer, OP_BINARY, &bytes).await
            }
            Command::Pong(payload) if !close_sent => {
                write_frame(&mut writer, OP_PONG, &payload).await
            }
            Command::Text(_) | Command::Binary(_) | Command::Pong(_) => Ok(()),
            Command::Close { code, reason } => {
                if close_sent {
                    Ok(())
                } else {
                    close_sent = true;
                    // Si le serveur ne répond pas, la connexion est coupée.
                    let later = sender.clone();
                    tokio::spawn(async move {
                        tokio::time::sleep(CLOSE_TIMEOUT).await;
                        let _ = later.send(Command::CloseTimedOut);
                    });
                    write_frame(&mut writer, OP_CLOSE, &close_payload(code, &reason)).await
                }
            }
            Command::EchoClose(code) => {
                if !close_sent {
                    let _ = write_frame(&mut writer, OP_CLOSE, &close_payload(code, "")).await;
                }
                break;
            }
            Command::CloseTimedOut => {
                emit(&emitter, abnormal());
                break;
            }
            Command::Abort => break,
        };
        if written.is_err() {
            emit(&emitter, abnormal());
            break;
        }
    }
    reading.abort();
    let _ = writer.shutdown().await;
}

/// Condensat SHA-1 (RFC 3174) de `data`. Il ne sert qu'à la poignée de main,
/// où la norme l'impose ; ce n'est pas une protection.
fn sha1(data: &[u8]) -> [u8; 20] {
    let mut state: [u32; 5] = [
        0x6745_2301,
        0xEFCD_AB89,
        0x98BA_DCFE,
        0x1032_5476,
        0xC3D2_E1F0,
    ];
    let mut message = data.to_vec();
    let bits = u64::try_from(data.len())
        .unwrap_or(u64::MAX)
        .wrapping_mul(8);
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bits.to_be_bytes());
    let (blocks, _) = message.as_chunks::<64>();
    for block in blocks {
        let mut words = [0u32; 80];
        let (quads, _) = block.as_chunks::<4>();
        for (word, quad) in words.iter_mut().zip(quads) {
            *word = u32::from_be_bytes(*quad);
        }
        for i in 16..80 {
            let mixed = words[i - 3] ^ words[i - 8] ^ words[i - 14] ^ words[i - 16];
            words[i] = mixed.rotate_left(1);
        }
        let [mut a, mut b, mut c, mut d, mut e] = state;
        for (i, word) in words.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | (!b & d), 0x5A82_7999),
                20..=39 => (b ^ c ^ d, 0x6ED9_EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC),
                _ => (b ^ c ^ d, 0xCA62_C1D6_u32),
            };
            let next = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(*word);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = next;
        }
        for (slot, value) in state.iter_mut().zip([a, b, c, d, e]) {
            *slot = slot.wrapping_add(value);
        }
    }
    let mut digest = [0u8; 20];
    for (out, word) in digest.as_chunks_mut::<4>().0.iter_mut().zip(state) {
        out.copy_from_slice(&word.to_be_bytes());
    }
    digest
}

/// Écriture de `bytes` en base64, avec remplissage.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let symbol = |index: u32| char::from(ALPHABET[(index & 0x3f) as usize]);
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let group = match *chunk {
            [a, b, c] => (u32::from(a) << 16) | (u32::from(b) << 8) | u32::from(c),
            [a, b] => (u32::from(a) << 16) | (u32::from(b) << 8),
            [a] => u32::from(a) << 16,
            _ => 0,
        };
        out.push(symbol(group >> 18));
        out.push(symbol(group >> 12));
        out.push(if chunk.len() > 1 {
            symbol(group >> 6)
        } else {
            '='
        });
        out.push(if chunk.len() > 2 { symbol(group) } else { '=' });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc as std_mpsc;

    #[test]
    fn digest_and_base64_match_the_standard() {
        // Exemple de la RFC 6455, §1.3.
        let accept = base64(&sha1(
            format!("dGhlIHNhbXBsZSBub25jZQ=={ACCEPT_GUID}").as_bytes(),
        ));
        assert_eq!(accept, "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        let hex =
            |data: &[u8]| -> String { sha1(data).iter().map(|b| format!("{b:02x}")).collect() };
        // Vecteurs de la RFC 3174 : un bloc, puis deux.
        assert_eq!(hex(b"abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(
            hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "84983e441c3bd26ebaae4aa1f95129e5e54670f1"
        );
    }

    /// Lit une trame masquée du client : (opcode, contenu).
    fn client_frame(stream: &mut std::net::TcpStream) -> (u8, Vec<u8>) {
        let mut head = [0u8; 2];
        stream.read_exact(&mut head).expect("en-tête de trame");
        assert_eq!(head[1] & 0x80, 0x80, "une trame de client est masquée");
        let mut length = usize::from(head[1] & 0x7f);
        if length == 126 {
            let mut wide = [0u8; 2];
            stream.read_exact(&mut wide).expect("longueur");
            length = usize::from(u16::from_be_bytes(wide));
        }
        let mut mask = [0u8; 4];
        stream.read_exact(&mut mask).expect("masque");
        let mut payload = vec![0u8; length];
        stream.read_exact(&mut payload).expect("contenu");
        for (byte, key) in payload.iter_mut().zip(mask.iter().cycle()) {
            *byte ^= key;
        }
        (head[0] & 0x0f, payload)
    }

    /// Trame du serveur, non masquée.
    fn server_frame(first: u8, payload: &[u8]) -> Vec<u8> {
        let mut frame = vec![first, u8::try_from(payload.len()).expect("trame courte")];
        frame.extend_from_slice(payload);
        frame
    }

    /// Serveur d'essai : accepte une connexion, répond à la poignée de main
    /// puis laisse `script` dialoguer.
    fn serve(
        accept_protocol: Option<&'static str>,
        script: impl FnOnce(&mut std::net::TcpStream) + Send + 'static,
    ) -> Url {
        let listener = TcpListener::bind("127.0.0.1:0").expect("écoute");
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
            assert!(text.starts_with("GET /chemin?x=1 HTTP/1.1\r\n"), "{text}");
            assert!(text.contains("Origin: https://exemple.test\r\n"), "{text}");
            assert!(
                text.contains("Sec-WebSocket-Protocol: chat, superchat\r\n"),
                "{text}"
            );
            let key = text
                .lines()
                .find_map(|line| line.strip_prefix("Sec-WebSocket-Key: "))
                .expect("clé")
                .trim()
                .to_owned();
            let accept = base64(&sha1(format!("{key}{ACCEPT_GUID}").as_bytes()));
            let mut response = format!(
                "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\n\
                 Connection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n"
            );
            if let Some(protocol) = accept_protocol {
                response.push_str(&format!("Sec-WebSocket-Protocol: {protocol}\r\n"));
            }
            response.push_str("\r\n");
            stream.write_all(response.as_bytes()).expect("réponse");
            script(&mut stream);
        });
        Url::parse(&format!("ws://127.0.0.1:{port}/chemin?x=1")).expect("url")
    }

    fn open(client: &Client, url: &Url) -> (SocketHandle, std_mpsc::Receiver<SocketEvent>) {
        let (sender, events) = std_mpsc::channel();
        let document = Url::parse("https://exemple.test/page").expect("url");
        let handle = client.open_socket(
            url,
            &document,
            &["chat".to_owned(), "superchat".to_owned()],
            move |event| {
                let _ = sender.send(event);
            },
        );
        (handle, events)
    }

    fn next(events: &std_mpsc::Receiver<SocketEvent>) -> SocketEvent {
        events
            .recv_timeout(Duration::from_secs(10))
            .expect("événement attendu")
    }

    #[test]
    fn messages_flow_both_ways_and_close_cleanly() {
        let url = serve(Some("superchat"), |stream| {
            // Écho du texte, en deux fragments avec un ping au milieu.
            let (opcode, payload) = client_frame(stream);
            assert_eq!(opcode, OP_TEXT);
            let (start, rest) = payload.split_at(3);
            stream
                .write_all(&server_frame(OP_TEXT, start))
                .expect("fragment");
            stream
                .write_all(&server_frame(0x80 | OP_PING, b"p"))
                .expect("ping");
            stream
                .write_all(&server_frame(0x80 | OP_CONTINUATION, rest))
                .expect("fin");
            assert_eq!(client_frame(stream), (OP_PONG, b"p".to_vec()));
            // Écho du binaire, long de plus de 125 octets.
            let (opcode, payload) = client_frame(stream);
            assert_eq!((opcode, payload.len()), (OP_BINARY, 300));
            let mut frame = vec![0x80 | OP_BINARY, 126, 1, 44];
            frame.extend_from_slice(&payload);
            stream.write_all(&frame).expect("binaire");
            // Fermeture demandée par le client.
            let (opcode, payload) = client_frame(stream);
            assert_eq!(opcode, OP_CLOSE);
            assert_eq!(payload, [&[0x03, 0xE8][..], "fini".as_bytes()].concat());
            stream
                .write_all(&server_frame(0x80 | OP_CLOSE, &payload))
                .expect("fermeture");
        });
        let client = Client::new().expect("client");
        let (socket, events) = open(&client, &url);
        assert_eq!(
            next(&events),
            SocketEvent::Open {
                protocol: "superchat".to_owned()
            }
        );
        socket.send_text("héllo wörld".to_owned());
        assert_eq!(next(&events), SocketEvent::Text("héllo wörld".to_owned()));
        socket.send_binary(vec![7; 300]);
        assert_eq!(next(&events), SocketEvent::Binary(vec![7; 300]));
        socket.close(Some(1000), "fini".to_owned());
        assert_eq!(
            next(&events),
            SocketEvent::Close {
                code: 1000,
                reason: "fini".to_owned(),
                clean: true
            }
        );
    }

    #[test]
    fn failures_end_with_an_abnormal_close() {
        let client = Client::new().expect("client");
        // Trame masquée par le serveur : faute de protocole.
        let url = serve(None, |stream| {
            stream
                .write_all(&[0x80 | OP_TEXT, 0x81, 0, 0, 0, 0, b'a'])
                .expect("trame");
            let (opcode, payload) = client_frame(stream);
            assert_eq!((opcode, payload), (OP_CLOSE, vec![0x03, 0xEA]));
        });
        let (_socket, events) = open(&client, &url);
        assert_eq!(
            next(&events),
            SocketEvent::Open {
                protocol: String::new()
            }
        );
        assert_eq!(next(&events), abnormal());
        // Sous-protocole non proposé : la connexion ne s'ouvre pas.
        let url = serve(Some("autre"), |_| {});
        let (_socket, events) = open(&client, &url);
        assert_eq!(next(&events), abnormal());
        // Personne n'écoute.
        let closed = {
            let listener = TcpListener::bind("127.0.0.1:0").expect("écoute");
            listener.local_addr().expect("adresse").port()
        };
        let url = Url::parse(&format!("ws://127.0.0.1:{closed}/chemin?x=1")).expect("url");
        let (_socket, events) = open(&client, &url);
        assert_eq!(next(&events), abnormal());
        // Fermeture venue du serveur, sans code.
        let url = serve(None, |stream| {
            stream
                .write_all(&server_frame(0x80 | OP_CLOSE, b""))
                .expect("fermeture");
            assert_eq!(client_frame(stream), (OP_CLOSE, Vec::new()));
        });
        let (_socket, events) = open(&client, &url);
        assert_eq!(
            next(&events),
            SocketEvent::Open {
                protocol: String::new()
            }
        );
        assert_eq!(
            next(&events),
            SocketEvent::Close {
                code: 1005,
                reason: String::new(),
                clean: true
            }
        );
    }
}
