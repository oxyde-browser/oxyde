//! Pile réseau d'Oxyde (pendant de `//net` dans Chromium).
//!
//! Client HTTP/1.1 et HTTP/2 sur TLS 1.2/1.3 : hyper pour HTTP, rustls pour
//! TLS avec le fournisseur cryptographique RustCrypto (aucun code C), racines
//! de confiance Mozilla (`webpki-roots`).
//!
//! AVERTISSEMENT DE SÉCURITÉ : `rustls-rustcrypto` est une préversion que ses
//! auteurs ne déclarent pas auditée. Elle est retenue pour tenir la règle
//! « 100 % Rust » de la maquette ; elle doit être réévaluée avant toute
//! diffusion du navigateur.
//!
//! LIMITES CONNUES : pas de HTTP/3, pas de cache, cookies en mémoire
//! seulement (voir [`cookies`]), pas de
//! réutilisation des connexions, décompression `gzip` et `deflate` seulement
//! (ni Brotli ni Zstandard, qui ne sont donc pas annoncés), résolution DNS
//! par le système, pas de proxy ; le corps d'une requête est envoyé d'un bloc.

use std::sync::Arc;
use std::time::Duration;

pub mod cookies;

use bytes::Bytes;
pub use cookies::{Access, CookieJar, same_site};
use http_body_util::{BodyExt, Full};
use hyper::header::{self, HeaderValue};
use hyper::{Method, Request, StatusCode};
use hyper_util::rt::{TokioExecutor, TokioIo};
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, RootCertStore};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
pub use url::Url;

/// Nombre maximal de redirections suivies (même valeur que Chromium).
const MAX_REDIRECTS: usize = 20;
/// Taille maximale d'un corps de réponse gardé en mémoire (64 Mio).
const MAX_BODY_BYTES: usize = 64 * 1024 * 1024;
/// Délai maximal pour une requête complète, redirections non comprises.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// Identité annoncée aux serveurs.
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Oxyde/0.1";

/// Erreur réseau.
#[derive(Debug, thiserror::Error)]
pub enum NetError {
    /// Schéma autre que `http` ou `https`.
    #[error("schéma non pris en charge : {0}")]
    UnsupportedScheme(String),
    /// URL sans nom d'hôte.
    #[error("adresse sans nom d'hôte : {0}")]
    MissingHost(String),
    /// Résolution de nom ou connexion TCP impossible.
    #[error("connexion à {host} impossible : {source}")]
    Connect {
        /// Hôte visé.
        host: String,
        /// Cause.
        source: std::io::Error,
    },
    /// Échec de la négociation TLS (certificat invalide, protocole…).
    #[error("connexion sécurisée à {host} impossible : {source}")]
    Tls {
        /// Hôte visé.
        host: String,
        /// Cause.
        source: std::io::Error,
    },
    /// Erreur du protocole HTTP.
    #[error("erreur HTTP : {0}")]
    Http(#[from] hyper::Error),
    /// Requête ou redirection mal formée.
    #[error("requête invalide : {0}")]
    InvalidRequest(String),
    /// Trop de redirections successives.
    #[error("trop de redirections")]
    TooManyRedirects,
    /// Le serveur n'a pas répondu dans le délai imparti.
    #[error("délai d'attente dépassé")]
    Timeout,
    /// Corps de réponse plus grand que la limite.
    #[error("réponse trop volumineuse (plus de {MAX_BODY_BYTES} octets)")]
    BodyTooLarge,
    /// Corps compressé illisible, ou compressé d'une façon non prise en charge.
    #[error("corps de réponse illisible (encodage « {0} »)")]
    Encoding(String),
    /// Initialisation de la pile impossible.
    #[error("initialisation du réseau impossible : {0}")]
    Init(String),
}

/// Code d'erreur de certificat correspondant à une erreur rustls, s'il y en a un.
fn certificate_code(error: &std::io::Error) -> Option<&'static str> {
    let rustls::Error::InvalidCertificate(reason) =
        error.get_ref()?.downcast_ref::<rustls::Error>()?
    else {
        return None;
    };
    // `CertificateError` est non exhaustif et ses variantes « avec contexte »
    // changent d'une version à l'autre : on classe d'après le nom de la variante.
    let name = format!("{reason:?}");
    Some(
        if name.starts_with("Expired") || name.starts_with("NotValidYet") {
            "NET::ERR_CERT_DATE_INVALID"
        } else if name.starts_with("NotValidForName") {
            "NET::ERR_CERT_COMMON_NAME_INVALID"
        } else if name.starts_with("UnknownIssuer") {
            "NET::ERR_CERT_AUTHORITY_INVALID"
        } else if name.starts_with("Revoked") {
            "NET::ERR_CERT_REVOKED"
        } else {
            "NET::ERR_CERT_INVALID"
        },
    )
}

impl NetError {
    /// Code d'erreur au format de Chromium (`ERR_CONNECTION_REFUSED`, …),
    /// affiché au bas des pages d'erreur.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnsupportedScheme(_) => "ERR_UNKNOWN_URL_SCHEME",
            Self::MissingHost(_) | Self::InvalidRequest(_) => "ERR_INVALID_URL",
            Self::Connect { source, .. } => match (source.raw_os_error(), source.kind()) {
                // Codes Winsock : hôte introuvable.
                (Some(11001 | 11002 | 11004), _) => "ERR_NAME_NOT_RESOLVED",
                // Codes Winsock : réseau arrêté ou inaccessible.
                (Some(10050 | 10051 | 10065), _) => "ERR_INTERNET_DISCONNECTED",
                (_, std::io::ErrorKind::ConnectionRefused) => "ERR_CONNECTION_REFUSED",
                (_, std::io::ErrorKind::TimedOut) => "ERR_CONNECTION_TIMED_OUT",
                _ => "ERR_CONNECTION_FAILED",
            },
            Self::Tls { source, .. } => {
                certificate_code(source).unwrap_or("ERR_SSL_PROTOCOL_ERROR")
            }
            Self::Http(_) => "ERR_INVALID_HTTP_RESPONSE",
            Self::TooManyRedirects => "ERR_TOO_MANY_REDIRECTS",
            Self::Timeout => "ERR_TIMED_OUT",
            Self::BodyTooLarge => "ERR_RESPONSE_TOO_LARGE",
            Self::Encoding(_) => "ERR_CONTENT_DECODING_FAILED",
            Self::Init(_) => "ERR_FAILED",
        }
    }

    /// Vrai si l'erreur vient d'un certificat de serveur refusé.
    #[must_use]
    pub fn is_certificate_error(&self) -> bool {
        matches!(self, Self::Tls { source, .. } if certificate_code(source).is_some())
    }
}

/// Réponse HTTP complète.
#[derive(Debug, Clone)]
pub struct Response {
    /// Adresse finale, après redirections.
    pub url: Url,
    /// Code d'état.
    pub status: u16,
    /// Valeur de l'en-tête `Content-Type`, si présent.
    pub content_type: Option<String>,
    /// En-têtes de la réponse : noms en minuscules, dans l'ordre reçu. Les
    /// valeurs qui ne sont pas du texte lisible sont omises.
    pub headers: Vec<(String, String)>,
    /// Corps.
    pub body: Vec<u8>,
}

/// Requête HTTP à envoyer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequest {
    /// Méthode (`GET`, `POST`…).
    pub method: String,
    /// Adresse, en `http` ou `https`.
    pub url: Url,
    /// En-têtes ajoutés à ceux du client (nom, valeur). `Host`,
    /// `Content-Length` et `Accept-Encoding` sont toujours fixés par le client.
    pub headers: Vec<(String, String)>,
    /// Corps ; vide s'il n'y en a pas.
    pub body: Vec<u8>,
    /// Document qui émet la requête ; `None` pour une navigation demandée
    /// par l'utilisateur. Décide des cookies joints : une requête vers un
    /// autre site que celui du document ne reçoit que les cookies
    /// `SameSite=None`.
    pub initiator: Option<Url>,
}

impl HttpRequest {
    /// Requête `GET` sans en-tête particulier.
    #[must_use]
    pub fn get(url: &Url) -> Self {
        Self {
            method: "GET".to_owned(),
            url: url.clone(),
            headers: Vec::new(),
            body: Vec::new(),
            initiator: None,
        }
    }
}

/// Client HTTP bloquant, partageable entre fils d'exécution.
pub struct Client {
    runtime: tokio::runtime::Runtime,
    tls: TlsConnector,
    /// Cookies reçus, joints aux requêtes suivantes.
    jar: std::sync::Mutex<CookieJar>,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client").finish_non_exhaustive()
    }
}

impl Client {
    /// Crée le client : boucle d'exécution asynchrone et configuration TLS.
    ///
    /// # Errors
    /// [`NetError::Init`] si la boucle d'exécution ou la configuration TLS ne
    /// peuvent pas être créées.
    pub fn new() -> Result<Self, NetError> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("oxyde-net")
            .enable_all()
            .build()
            .map_err(|e| NetError::Init(e.to_string()))?;
        let mut roots = RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let mut config =
            ClientConfig::builder_with_provider(Arc::new(rustls_rustcrypto::provider()))
                .with_safe_default_protocol_versions()
                .map_err(|e| NetError::Init(e.to_string()))?
                .with_root_certificates(roots)
                .with_no_client_auth();
        config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
        Ok(Self {
            runtime,
            tls: TlsConnector::from(Arc::new(config)),
            jar: std::sync::Mutex::new(CookieJar::new()),
        })
    }

    /// Texte de `document.cookie` pour un document d'adresse `url` : les
    /// cookies qu'un script a le droit de lire.
    #[must_use]
    pub fn cookies_for_script(&self, url: &Url) -> String {
        self.jar
            .lock()
            .ok()
            .and_then(|mut jar| jar.header_for(url, Access::Script, false))
            .unwrap_or_default()
    }

    /// Enregistre le cookie qu'un script du document `url` écrit dans
    /// `document.cookie`. Renvoie `false` s'il est refusé.
    pub fn set_cookie_from_script(&self, url: &Url, text: &str) -> bool {
        self.jar
            .lock()
            .is_ok_and(|mut jar| jar.store(url, text, Access::Script, false))
    }

    /// Récupère `url` par une requête GET émise par le document `initiator`
    /// (une feuille de style, une image, un script de la page).
    ///
    /// # Errors
    /// Voir [`Client::send`].
    pub fn get_from(&self, url: &Url, initiator: &Url) -> Result<Response, NetError> {
        self.send(&HttpRequest {
            initiator: Some(initiator.clone()),
            ..HttpRequest::get(url)
        })
    }

    /// Récupère `url` par une requête GET, en suivant les redirections.
    ///
    /// Bloque le fil appelant ; à ne pas appeler depuis le fil de l'interface.
    ///
    /// # Errors
    /// Voir [`NetError`]. Un code d'état 4xx ou 5xx n'est pas une erreur : la
    /// réponse est renvoyée telle quelle.
    pub fn get(&self, url: &Url) -> Result<Response, NetError> {
        self.send(&HttpRequest::get(url))
    }

    /// Envoie `request` et suit les redirections. Après une redirection 303,
    /// ou 301 et 302 pour un `POST`, la suite se fait en `GET` sans corps ;
    /// 307 et 308 gardent la méthode et le corps.
    ///
    /// Bloque le fil appelant ; à ne pas appeler depuis le fil de l'interface.
    ///
    /// # Errors
    /// Voir [`NetError`]. Un code d'état 4xx ou 5xx n'est pas une erreur.
    pub fn send(&self, request: &HttpRequest) -> Result<Response, NetError> {
        self.runtime.block_on(async {
            let mut current = request.clone();
            for _ in 0..=MAX_REDIRECTS {
                let step = tokio::time::timeout(REQUEST_TIMEOUT, self.send_once(&current))
                    .await
                    .map_err(|_| NetError::Timeout)??;
                match step {
                    Step::Done(response) => return Ok(response),
                    Step::Redirect(next, status) => {
                        // Jamais de rétrogradation de https vers http.
                        if current.url.scheme() == "https" && next.scheme() != "https" {
                            return Err(NetError::InvalidRequest(format!(
                                "redirection non sécurisée vers {next}"
                            )));
                        }
                        let to_get = status == StatusCode::SEE_OTHER
                            || (matches!(
                                status,
                                StatusCode::MOVED_PERMANENTLY | StatusCode::FOUND
                            ) && current.method == "POST");
                        if to_get {
                            current.method = "GET".to_owned();
                            current.body.clear();
                            current.headers.retain(|(name, _)| {
                                !name.to_ascii_lowercase().starts_with("content-")
                            });
                        }
                        // Un jeton d'accès ne suit pas vers une autre origine.
                        if next.origin() != current.url.origin() {
                            current
                                .headers
                                .retain(|(name, _)| !name.eq_ignore_ascii_case("authorization"));
                        }
                        current.url = next;
                    }
                }
            }
            Err(NetError::TooManyRedirects)
        })
    }

    async fn send_once(&self, request: &HttpRequest) -> Result<Step, NetError> {
        let url = &request.url;
        let method = Method::from_bytes(request.method.as_bytes())
            .map_err(|e| NetError::InvalidRequest(e.to_string()))?;
        let secure = match url.scheme() {
            "https" => true,
            "http" => false,
            other => return Err(NetError::UnsupportedScheme(other.to_owned())),
        };
        let host = url
            .host_str()
            .ok_or_else(|| NetError::MissingHost(url.to_string()))?
            .to_owned();
        let port = url
            .port_or_known_default()
            .unwrap_or(if secure { 443 } else { 80 });
        // Les littéraux IPv6 sont entre crochets dans l'URL, pas pour la connexion.
        let connect_host = host
            .trim_start_matches('[')
            .trim_end_matches(']')
            .to_owned();
        let tcp = TcpStream::connect((connect_host.as_str(), port))
            .await
            .map_err(|source| NetError::Connect {
                host: host.clone(),
                source,
            })?;
        // Désactiver Nagle n'est qu'une optimisation ; un échec est sans conséquence.
        let _ = tcp.set_nodelay(true);

        let authority = match url.port() {
            Some(p) => format!("{host}:{p}"),
            None => host.clone(),
        };
        let path = match url.query() {
            Some(q) => format!("{}?{q}", url.path()),
            None => url.path().to_owned(),
        };
        // En-têtes fixés par le client : l'appelant ne peut pas les remplacer.
        let reserved = |name: &str| {
            ["host", "content-length", "accept-encoding", "connection"]
                .iter()
                .any(|fixed| name.eq_ignore_ascii_case(fixed))
        };
        let has = |wanted: &str| {
            request
                .headers
                .iter()
                .any(|(name, _)| name.eq_ignore_ascii_case(wanted))
        };
        // Cookies du pot, sauf si l'appelant a fixé l'en-tête lui-même.
        let cross_site = request
            .initiator
            .as_ref()
            .is_some_and(|initiator| !same_site(initiator, url));
        let cookie = if has("cookie") {
            None
        } else {
            self.jar
                .lock()
                .ok()
                .and_then(|mut jar| jar.header_for(url, Access::Http, cross_site))
        };
        // `host` : en HTTP/2 l'autorité voyage dans la cible (`:authority`) ;
        // l'envoyer aussi en en-tête fait rejeter la requête par certains
        // serveurs (PROTOCOL_ERROR chez Google).
        let build = |target: &str, host_header: bool| {
            let mut builder = Request::builder()
                .method(method.clone())
                .uri(target)
                .header(header::ACCEPT_ENCODING, "gzip, deflate");
            if host_header {
                builder = builder.header(header::HOST, &authority);
            }
            if !has("user-agent") {
                builder = builder.header(header::USER_AGENT, USER_AGENT);
            }
            if !has("accept") {
                builder = builder.header(
                    header::ACCEPT,
                    "text/html,application/xhtml+xml,text/css,*/*;q=0.8",
                );
            }
            // LIMITE : la langue annoncée est fixe ; elle suivra les paramètres.
            if !has("accept-language") {
                builder = builder.header(header::ACCEPT_LANGUAGE, "fr-FR,fr;q=0.9,en;q=0.8");
            }
            for (name, value) in &request.headers {
                if !reserved(name) {
                    builder = builder.header(name.as_str(), value.as_str());
                }
            }
            if let Some(cookie) = &cookie {
                builder = builder.header(header::COOKIE, cookie.as_str());
            }
            builder
                .body(Full::new(Bytes::from(request.body.clone())))
                .map_err(|e| NetError::InvalidRequest(e.to_string()))
        };

        let response = if secure {
            let name = ServerName::try_from(connect_host.clone())
                .map_err(|e| NetError::InvalidRequest(e.to_string()))?;
            let tls = self
                .tls
                .connect(name, tcp)
                .await
                .map_err(|source| NetError::Tls {
                    host: host.clone(),
                    source,
                })?;
            let http2 = tls.get_ref().1.alpn_protocol() == Some(b"h2");
            let io = TokioIo::new(tls);
            if http2 {
                let (mut sender, connection) =
                    hyper::client::conn::http2::handshake(TokioExecutor::new(), io).await?;
                tokio::spawn(connection);
                // HTTP/2 exige une cible absolue (schéma et autorité).
                sender
                    .send_request(build(&format!("https://{authority}{path}"), false)?)
                    .await?
            } else {
                let (mut sender, connection) = hyper::client::conn::http1::handshake(io).await?;
                tokio::spawn(connection);
                sender.send_request(build(&path, true)?).await?
            }
        } else {
            let (mut sender, connection) =
                hyper::client::conn::http1::handshake(TokioIo::new(tcp)).await?;
            tokio::spawn(connection);
            sender.send_request(build(&path, true)?).await?
        };

        let status = response.status();
        // Les cookies posés par la réponse, y compris celle d'une redirection.
        if let Ok(mut jar) = self.jar.lock() {
            for value in response.headers().get_all(header::SET_COOKIE) {
                if let Ok(text) = value.to_str() {
                    jar.store(url, text, Access::Http, cross_site);
                }
            }
        }
        if matches!(
            status,
            StatusCode::MOVED_PERMANENTLY
                | StatusCode::FOUND
                | StatusCode::SEE_OTHER
                | StatusCode::TEMPORARY_REDIRECT
                | StatusCode::PERMANENT_REDIRECT
        ) && let Some(location) = response
            .headers()
            .get(header::LOCATION)
            .and_then(|v: &HeaderValue| v.to_str().ok())
        {
            let next = url
                .join(location)
                .map_err(|e| NetError::InvalidRequest(e.to_string()))?;
            if !matches!(next.scheme(), "http" | "https") {
                return Err(NetError::UnsupportedScheme(next.scheme().to_owned()));
            }
            return Ok(Step::Redirect(next, status));
        }

        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let encoding = response
            .headers()
            .get(header::CONTENT_ENCODING)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let headers = response
            .headers()
            .iter()
            // `Set-Cookie` s'arrête ici : il ne remonte ni aux pages ni aux scripts.
            .filter(|(name, _)| *name != header::SET_COOKIE)
            // Le corps est rendu décompressé : ces en-têtes ne le décrivent plus.
            .filter(|(name, _)| {
                encoding.is_none()
                    || (*name != header::CONTENT_ENCODING && *name != header::CONTENT_LENGTH)
            })
            .filter_map(|(name, value)| {
                Some((name.as_str().to_owned(), value.to_str().ok()?.to_owned()))
            })
            .collect();
        let mut body = Vec::new();
        let mut stream = response.into_body();
        while let Some(frame) = stream.frame().await {
            if let Some(chunk) = frame?.data_ref() {
                if body.len().saturating_add(chunk.len()) > MAX_BODY_BYTES {
                    return Err(NetError::BodyTooLarge);
                }
                body.extend_from_slice(chunk);
            }
        }
        let body = match encoding {
            Some(encoding) if !body.is_empty() => decode_body(body, &encoding)?,
            _ => body,
        };
        Ok(Step::Done(Response {
            url: url.clone(),
            status: status.as_u16(),
            content_type,
            headers,
            body,
        }))
    }
}

enum Step {
    Done(Response),
    /// Adresse suivante et code d'état de la redirection.
    Redirect(Url, StatusCode),
}

/// Décompresse `body` d'après l'en-tête `Content-Encoding` : une liste
/// d'encodages appliqués dans l'ordre, donc défaits en sens inverse. Le
/// résultat est borné à [`MAX_BODY_BYTES`] (une réponse minuscule peut se
/// dilater sans limite).
fn decode_body(mut body: Vec<u8>, encodings: &str) -> Result<Vec<u8>, NetError> {
    use std::io::Read;
    let unreadable = || NetError::Encoding(encodings.to_owned());
    let limit = u64::try_from(MAX_BODY_BYTES).unwrap_or(u64::MAX);
    let inflate = |reader: &mut dyn Read| -> Option<Vec<u8>> {
        let mut out = Vec::new();
        reader
            .take(limit.saturating_add(1))
            .read_to_end(&mut out)
            .ok()?;
        Some(out)
    };
    for encoding in encodings.rsplit(',') {
        let decoded = match encoding.trim().to_ascii_lowercase().as_str() {
            "" | "identity" => continue,
            "gzip" | "x-gzip" => inflate(&mut flate2::read::MultiGzDecoder::new(body.as_slice())),
            // `deflate` désigne le format zlib ; des serveurs envoient pourtant
            // le flux brut, sans en-tête.
            "deflate" => inflate(&mut flate2::read::ZlibDecoder::new(body.as_slice()))
                .or_else(|| inflate(&mut flate2::read::DeflateDecoder::new(body.as_slice()))),
            _ => None,
        }
        .ok_or_else(unreadable)?;
        if decoded.len() > MAX_BODY_BYTES {
            return Err(NetError::BodyTooLarge);
        }
        body = decoded;
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    /// Petit serveur HTTP/1.1 local : répond aux `count` premières connexions.
    fn serve(count: usize, respond: fn(&str) -> String) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").expect("écoute locale");
        let port = listener.local_addr().expect("adresse locale").port();
        std::thread::spawn(move || {
            for stream in listener.incoming().take(count) {
                let Ok(mut stream) = stream else { continue };
                let mut buffer = [0u8; 4096];
                let n = stream.read(&mut buffer).unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..n]).into_owned();
                let path = request.split_whitespace().nth(1).unwrap_or("/").to_owned();
                let _ = stream.write_all(respond(&path).as_bytes());
            }
        });
        port
    }

    #[test]
    fn plain_http_get_and_redirect() {
        let port = serve(2, |path| {
            match path {
            "/depart" => "HTTP/1.1 302 Found\r\nLocation: /arrivee?x=1\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_owned(),
            _ => "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: 7\r\nConnection: close\r\n\r\nbonjour".to_owned(),
        }
        });
        let client = Client::new().expect("client");
        let url = Url::parse(&format!("http://127.0.0.1:{port}/depart")).expect("url");
        let response = client.get(&url).expect("réponse");
        assert_eq!(response.status, 200);
        assert_eq!(response.url.path(), "/arrivee");
        assert_eq!(response.url.query(), Some("x=1"));
        assert_eq!(
            response.content_type.as_deref(),
            Some("text/html; charset=utf-8")
        );
        assert_eq!(response.body, b"bonjour");
    }

    /// Serveur local qui renvoie, dans le corps, la première ligne de la
    /// requête, ses en-têtes `x-essai` et `content-type`, puis son corps.
    fn echo(count: usize) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").expect("écoute locale");
        let port = listener.local_addr().expect("adresse locale").port();
        std::thread::spawn(move || {
            for stream in listener.incoming().take(count) {
                let Ok(mut stream) = stream else { continue };
                let mut buffer = [0u8; 4096];
                let n = stream.read(&mut buffer).unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..n]).into_owned();
                let (head, body) = request.split_once("\r\n\r\n").unwrap_or((&request, ""));
                let first = head.lines().next().unwrap_or("");
                let reply = if first.starts_with("POST /redirige") {
                    "HTTP/1.1 303 See Other\r\nLocation: /suite\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_owned()
                } else {
                    let kept: Vec<&str> = head
                        .lines()
                        .filter(|line| {
                            let lower = line.to_ascii_lowercase();
                            lower.starts_with("x-essai") || lower.starts_with("content-type")
                        })
                        .collect();
                    let text = format!("{first}|{}|{body}", kept.join(";"));
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",
                        text.len()
                    )
                };
                let _ = stream.write_all(reply.as_bytes());
            }
        });
        port
    }

    #[test]
    fn post_sends_method_headers_and_body() {
        let port = echo(1);
        let client = Client::new().expect("client");
        let request = HttpRequest {
            method: "POST".to_owned(),
            url: Url::parse(&format!("http://127.0.0.1:{port}/depot")).expect("url"),
            headers: vec![
                ("X-Essai".to_owned(), "oui".to_owned()),
                ("Content-Type".to_owned(), "text/plain".to_owned()),
                ("Host".to_owned(), "ailleurs.test".to_owned()),
            ],
            body: b"bonjour".to_vec(),
            initiator: None,
        };
        let response = client.send(&request).expect("réponse");
        let text = String::from_utf8_lossy(&response.body).into_owned();
        assert!(text.starts_with("POST /depot HTTP/1.1|"), "{text}");
        assert!(text.to_ascii_lowercase().contains("x-essai: oui"), "{text}");
        assert!(
            text.to_ascii_lowercase()
                .contains("content-type: text/plain"),
            "{text}"
        );
        assert!(text.ends_with("|bonjour"), "{text}");
    }

    #[test]
    fn see_other_turns_post_into_get() {
        let port = echo(2);
        let client = Client::new().expect("client");
        let request = HttpRequest {
            method: "POST".to_owned(),
            url: Url::parse(&format!("http://127.0.0.1:{port}/redirige")).expect("url"),
            headers: vec![("Content-Type".to_owned(), "text/plain".to_owned())],
            body: b"corps".to_vec(),
            initiator: None,
        };
        let response = client.send(&request).expect("réponse");
        let text = String::from_utf8_lossy(&response.body).into_owned();
        assert_eq!(text, "GET /suite HTTP/1.1||");
    }

    #[test]
    fn redirect_loop_is_bounded() {
        let port = serve(MAX_REDIRECTS + 2, |_| {
            "HTTP/1.1 301 Moved\r\nLocation: /encore\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_owned()
        });
        let client = Client::new().expect("client");
        let url = Url::parse(&format!("http://127.0.0.1:{port}/")).expect("url");
        assert!(matches!(client.get(&url), Err(NetError::TooManyRedirects)));
    }

    #[test]
    fn errors_are_typed() {
        let client = Client::new().expect("client");
        let ftp = Url::parse("ftp://exemple.test/").expect("url");
        assert!(matches!(
            client.get(&ftp),
            Err(NetError::UnsupportedScheme(_))
        ));
        // Port 1 sur la boucle locale : personne n'écoute.
        let closed = Url::parse("http://127.0.0.1:1/").expect("url");
        assert!(matches!(client.get(&closed), Err(NetError::Connect { .. })));
    }

    #[test]
    fn compressed_bodies_are_decoded() {
        use flate2::Compression;
        use flate2::write::{DeflateEncoder, GzEncoder, ZlibEncoder};
        let text = b"bonjour, bonjour, bonjour".to_vec();
        let gzip = {
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&text).expect("écriture");
            encoder.finish().expect("fin")
        };
        let zlib = {
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&text).expect("écriture");
            encoder.finish().expect("fin")
        };
        let raw = {
            let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&text).expect("écriture");
            encoder.finish().expect("fin")
        };
        assert_eq!(decode_body(gzip.clone(), "gzip").expect("gzip"), text);
        assert_eq!(
            decode_body(gzip.clone(), " GZIP , identity").expect("liste"),
            text
        );
        assert_eq!(decode_body(zlib, "deflate").expect("zlib"), text);
        assert_eq!(decode_body(raw, "deflate").expect("flux brut"), text);
        assert_eq!(
            decode_body(text.clone(), "identity").expect("identité"),
            text
        );
        // Un encodage inconnu ou un corps abîmé est une erreur, pas du texte.
        assert!(matches!(
            decode_body(gzip.clone(), "br"),
            Err(NetError::Encoding(_))
        ));
        assert!(matches!(
            decode_body(gzip[..gzip.len() / 2].to_vec(), "gzip"),
            Err(NetError::Encoding(_))
        ));
    }
}
