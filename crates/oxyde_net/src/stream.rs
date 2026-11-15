//! Réponses HTTP lues au fil de l'eau : le corps est remis morceau par
//! morceau, à mesure qu'il arrive, au lieu d'être attendu en entier. C'est ce
//! qu'il faut à `EventSource`, dont le serveur garde la réponse ouverte.
//!
//! LIMITES CONNUES : seuls les corps non compressés ou compressés en `gzip`,
//! `deflate` ou Brotli se lisent ainsi ; un flux n'a pas de délai une fois son
//! en-tête reçu (c'est voulu : il peut rester muet longtemps), mais il occupe
//! le fil qui l'a ouvert jusqu'à sa fin ou son abandon.

use std::io::Write as _;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use http_body_util::BodyExt;
use hyper::StatusCode;
use url::Url;

use crate::{Client, HeadStep, HttpRequest, MAX_REDIRECTS, NetError, REQUEST_TIMEOUT};

/// Intervalle auquel un flux muet vérifie s'il a été abandonné.
const CANCEL_POLL: Duration = Duration::from_millis(250);

/// Ce qui arrive à une réponse lue au fil de l'eau.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StreamEvent {
    /// En-tête de la réponse finale, redirections suivies.
    Head {
        /// Adresse finale.
        url: Url,
        /// Code d'état.
        status: u16,
        /// En-têtes (sans `Set-Cookie`).
        headers: Vec<(String, String)>,
    },
    /// Morceau du corps, décompressé.
    Chunk(Vec<u8>),
    /// Le corps est fini. Aucun événement ne suit.
    End,
    /// La requête ou la lecture a échoué. Aucun événement ne suit.
    Failed(String),
}

/// Décompression à mesure que les octets arrivent.
enum Inflater {
    Plain,
    Gzip(flate2::write::GzDecoder<Vec<u8>>),
    Deflate(flate2::write::ZlibDecoder<Vec<u8>>),
    Brotli(Box<brotli_decompressor::DecompressorWriter<Vec<u8>>>),
}

impl Inflater {
    fn new(encoding: Option<&str>) -> Option<Self> {
        match encoding.map(|e| e.trim().to_ascii_lowercase()).as_deref() {
            None | Some("" | "identity") => Some(Self::Plain),
            Some("gzip" | "x-gzip") => Some(Self::Gzip(flate2::write::GzDecoder::new(Vec::new()))),
            Some("deflate") => Some(Self::Deflate(flate2::write::ZlibDecoder::new(Vec::new()))),
            Some("br") => Some(Self::Brotli(Box::new(
                brotli_decompressor::DecompressorWriter::new(Vec::new(), 4096),
            ))),
            Some(_) => None,
        }
    }

    /// Octets décompressés que donne `chunk`.
    fn push(&mut self, chunk: &[u8]) -> std::io::Result<Vec<u8>> {
        match self {
            Self::Plain => Ok(chunk.to_vec()),
            Self::Gzip(decoder) => {
                decoder.write_all(chunk)?;
                decoder.flush()?;
                Ok(std::mem::take(decoder.get_mut()))
            }
            Self::Deflate(decoder) => {
                decoder.write_all(chunk)?;
                decoder.flush()?;
                Ok(std::mem::take(decoder.get_mut()))
            }
            Self::Brotli(decoder) => {
                decoder.write_all(chunk)?;
                decoder.flush()?;
                Ok(std::mem::take(decoder.get_mut()))
            }
        }
    }
}

impl Client {
    /// Envoie `request`, suit ses redirections et remet la réponse à
    /// `on_event` à mesure qu'elle arrive : son en-tête, puis les morceaux
    /// de son corps, puis [`StreamEvent::End`] ou [`StreamEvent::Failed`].
    ///
    /// Bloque le fil appelant jusqu'à la fin du corps, ou jusqu'à ce que
    /// `cancelled` passe à vrai (vérifié à chaque morceau et quatre fois par
    /// seconde sur un flux muet) ; dans ce dernier cas, plus aucun événement
    /// n'est remis.
    pub fn stream(
        &self,
        request: &HttpRequest,
        cancelled: &AtomicBool,
        on_event: &mut dyn FnMut(StreamEvent),
    ) {
        let outcome = self.runtime.block_on(async {
            let mut current = request.clone();
            let mut hops = 0;
            let (head, mut body) = loop {
                let step = tokio::time::timeout(REQUEST_TIMEOUT, self.send_head(&current))
                    .await
                    .map_err(|_| NetError::Timeout)??;
                match step {
                    // Un flux garde sa connexion jusqu'au bout.
                    HeadStep::Final(head, body, _) => break (head, body),
                    HeadStep::Redirect(next, status) => {
                        hops += 1;
                        if hops > MAX_REDIRECTS {
                            return Err(NetError::TooManyRedirects);
                        }
                        if current.url.scheme() == "https" && next.scheme() != "https" {
                            return Err(NetError::InvalidRequest(format!(
                                "redirection non sécurisée vers {next}"
                            )));
                        }
                        if status == StatusCode::SEE_OTHER {
                            current.method = "GET".to_owned();
                            current.body.clear();
                        }
                        if next.origin() != current.url.origin() {
                            current
                                .headers
                                .retain(|(name, _)| !name.eq_ignore_ascii_case("authorization"));
                        }
                        current.url = next;
                    }
                }
            };
            let Some(mut inflater) = Inflater::new(head.encoding.as_deref()) else {
                return Err(NetError::Encoding(head.encoding.unwrap_or_default()));
            };
            if cancelled.load(Ordering::Relaxed) {
                return Ok(false);
            }
            on_event(StreamEvent::Head {
                url: current.url.clone(),
                status: head.status,
                headers: head.headers,
            });
            loop {
                if cancelled.load(Ordering::Relaxed) {
                    return Ok(false);
                }
                let frame = match tokio::time::timeout(CANCEL_POLL, body.frame()).await {
                    // Rien pour l'instant : le temps de vérifier l'abandon.
                    Err(_) => continue,
                    Ok(None) => return Ok(true),
                    Ok(Some(frame)) => frame?,
                };
                if let Some(chunk) = frame.data_ref() {
                    let bytes = inflater
                        .push(chunk)
                        .map_err(|_| NetError::Encoding("flux compressé illisible".to_owned()))?;
                    if !bytes.is_empty() && !cancelled.load(Ordering::Relaxed) {
                        on_event(StreamEvent::Chunk(bytes));
                    }
                }
            }
        });
        match outcome {
            Ok(true) => on_event(StreamEvent::End),
            // Abandonné : plus personne n'écoute.
            Ok(false) => {}
            Err(error) => {
                if !cancelled.load(Ordering::Relaxed) {
                    on_event(StreamEvent::Failed(error.to_string()));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::Arc;
    use std::sync::mpsc;

    /// Serveur d'essai : lit une requête, puis laisse `script` répondre.
    fn serve(script: impl FnOnce(&mut std::net::TcpStream, String) + Send + 'static) -> Url {
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
            script(&mut stream, String::from_utf8_lossy(&request).into_owned());
        });
        Url::parse(&format!("http://127.0.0.1:{port}/flux")).expect("url")
    }

    #[test]
    fn chunks_arrive_before_the_body_ends() {
        let (proceed, wait) = mpsc::channel::<()>();
        let url = serve(move |stream, request| {
            assert!(
                request.contains("accept: text/event-stream\r\n"),
                "{request}"
            );
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\
                      Transfer-Encoding: chunked\r\n\r\n6\r\ndata: \r\n",
                )
                .expect("début");
            stream.flush().expect("envoi");
            // La suite n'est envoyée qu'une fois le premier morceau reçu.
            wait.recv().expect("signal");
            stream.write_all(b"3\r\nun\n\r\n0\r\n\r\n").expect("fin");
        });
        let client = Client::new().expect("client");
        let request = HttpRequest {
            headers: vec![("accept".to_owned(), "text/event-stream".to_owned())],
            ..HttpRequest::get(&url)
        };
        let mut events = Vec::new();
        client.stream(&request, &AtomicBool::new(false), &mut |event| {
            if matches!(event, StreamEvent::Chunk(_)) && events.len() == 1 {
                proceed.send(()).expect("signal");
            }
            events.push(event);
        });
        assert!(
            matches!(&events[0], StreamEvent::Head { status: 200, headers, .. }
                if headers.iter().any(|(n, v)| n == "content-type" && v == "text/event-stream"))
        );
        assert_eq!(
            events[1..],
            [
                StreamEvent::Chunk(b"data: ".to_vec()),
                StreamEvent::Chunk(b"un\n".to_vec()),
                StreamEvent::End
            ]
        );
    }

    #[test]
    fn a_silent_stream_can_be_abandoned() {
        let url = serve(|stream, _| {
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n")
                .expect("en-tête");
            // Le serveur se tait, connexion ouverte.
            std::thread::sleep(Duration::from_secs(20));
        });
        let client = Client::new().expect("client");
        let cancelled = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&cancelled);
        let started = std::time::Instant::now();
        let mut events = Vec::new();
        client.stream(&HttpRequest::get(&url), &cancelled, &mut |event| {
            // L'en-tête reçu, la page s'en va.
            flag.store(true, Ordering::Relaxed);
            events.push(event);
        });
        assert_eq!(events.len(), 1, "{events:?}");
        assert!(started.elapsed() < Duration::from_secs(10));
        // Personne n'écoute : un échec, pas un blocage.
        let closed = {
            let listener = TcpListener::bind("127.0.0.1:0").expect("écoute");
            listener.local_addr().expect("adresse").port()
        };
        let gone = Url::parse(&format!("http://127.0.0.1:{closed}/")).expect("url");
        let mut events = Vec::new();
        client.stream(
            &HttpRequest::get(&gone),
            &AtomicBool::new(false),
            &mut |event| {
                events.push(event);
            },
        );
        assert!(matches!(events.as_slice(), [StreamEvent::Failed(_)]));
    }
}
