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
//! LIMITES CONNUES : pas de HTTP/3, pas de cache, pas de cookies, pas de
//! réutilisation des connexions, pas de décompression (`Accept-Encoding:
//! identity`), résolution DNS par le système, pas de proxy.

use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use http_body_util::{BodyExt, Empty};
use hyper::header::{self, HeaderValue};
use hyper::{Request, StatusCode};
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
    /// Corps.
    pub body: Vec<u8>,
}

/// Client HTTP bloquant, partageable entre fils d'exécution.
pub struct Client {
    runtime: tokio::runtime::Runtime,
    tls: TlsConnector,
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
        self.runtime.block_on(async {
            let mut current = url.clone();
            for _ in 0..=MAX_REDIRECTS {
                let step = tokio::time::timeout(REQUEST_TIMEOUT, self.get_once(&current))
                    .await
                    .map_err(|_| NetError::Timeout)??;
                match step {
                    Step::Done(response) => return Ok(response),
                    Step::Redirect(next) => {
                        // Jamais de rétrogradation de https vers http.
                        if current.scheme() == "https" && next.scheme() != "https" {
                            return Err(NetError::InvalidRequest(format!(
                                "redirection non sécurisée vers {next}"
                            )));
                        }
                        current = next;
                    }
                }
            }
            Err(NetError::TooManyRedirects)
        })
    }

    async fn get_once(&self, url: &Url) -> Result<Step, NetError> {
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
        let build = |target: &str| {
            Request::builder()
                .method("GET")
                .uri(target)
                .header(header::HOST, &authority)
                .header(header::USER_AGENT, USER_AGENT)
                .header(
                    header::ACCEPT,
                    "text/html,application/xhtml+xml,text/css,*/*;q=0.8",
                )
                .header(header::ACCEPT_ENCODING, "identity")
                .body(Empty::<Bytes>::new())
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
                    .send_request(build(&format!("https://{authority}{path}"))?)
                    .await?
            } else {
                let (mut sender, connection) = hyper::client::conn::http1::handshake(io).await?;
                tokio::spawn(connection);
                sender.send_request(build(&path)?).await?
            }
        } else {
            let (mut sender, connection) =
                hyper::client::conn::http1::handshake(TokioIo::new(tcp)).await?;
            tokio::spawn(connection);
            sender.send_request(build(&path)?).await?
        };

        let status = response.status();
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
            return Ok(Step::Redirect(next));
        }

        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
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
        Ok(Step::Done(Response {
            url: url.clone(),
            status: status.as_u16(),
            content_type,
            body,
        }))
    }
}

enum Step {
    Done(Response),
    Redirect(Url),
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
}
