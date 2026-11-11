//! Chargement des ressources d'une page : `file:`, `about:`, `data:` et `http(s):`.

use std::path::Path;
use std::sync::OnceLock;

use base64::Engine as _;
use oxyde_content::{Page, Url, decode_image};
use oxyde_net::NetError;

/// Taille maximale d'une ressource locale chargée en mémoire (64 Mio).
const MAX_RESOURCE_BYTES: u64 = 64 * 1024 * 1024;

/// Moteur de recherche utilisé quand la saisie n'est pas une adresse. Sa
/// version HTML fonctionne sans JavaScript. Le choix du moteur sera un
/// réglage de l'écran Paramètres (jalon 7).
const SEARCH_URL: &str = "https://html.duckduckgo.com/html/";

/// Nature d'une erreur de chargement ; détermine la page d'erreur affichée.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ErrorKind {
    /// Le site n'a pas pu être joint.
    Network,
    /// Le certificat du site a été refusé.
    Certificate,
    /// Autre échec (fichier introuvable, adresse invalide…).
    Other,
}

/// Erreur de chargement, présentée à l'utilisateur dans une page d'erreur.
#[derive(Clone, Debug)]
pub(crate) struct LoadError {
    pub(crate) kind: ErrorKind,
    pub(crate) title: String,
    pub(crate) message: String,
    /// Code au format de Chromium, affiché au bas de la page.
    pub(crate) code: &'static str,
    /// Détail technique, affiché sur demande.
    pub(crate) detail: String,
}

impl LoadError {
    pub(crate) fn other(title: &str, message: String, code: &'static str) -> Self {
        Self {
            kind: ErrorKind::Other,
            title: title.to_owned(),
            detail: message.clone(),
            message,
            code,
        }
    }
}

/// Résultat d'un chargement : la page, et l'erreur s'il a échoué (la page est
/// alors vide et porte l'adresse demandée, pour que « Réessayer » fonctionne).
#[derive(Debug)]
pub(crate) struct Loaded {
    pub(crate) page: Page,
    pub(crate) error: Option<LoadError>,
}

fn looks_like_host(input: &str) -> bool {
    let host = input.split(['/', '?', '#']).next().unwrap_or("");
    let host = match host.rsplit_once(':') {
        Some((h, port)) if !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()) => h,
        _ => host,
    };
    host == "localhost"
        || host.parse::<std::net::IpAddr>().is_ok()
        || (host.contains('.')
            && !host.starts_with('.')
            && !host.ends_with('.')
            && host
                .chars()
                .all(|c| c.is_alphanumeric() || matches!(c, '.' | '-')))
}

/// Interprète ce que l'utilisateur a saisi : chemin local, URL absolue, nom
/// d'hôte sans schéma (complété en `https://`), ou sinon recherche.
pub(crate) fn resolve_input(input: &str) -> Result<Url, LoadError> {
    let input = input.trim();
    let invalid = |e: &dyn std::fmt::Display| {
        LoadError::other(
            "Adresse invalide",
            format!("{input} : {e}"),
            "ERR_INVALID_URL",
        )
    };
    let path = Path::new(input);
    if path.exists() {
        let absolute = path.canonicalize().map_err(|e| {
            LoadError::other(
                "Fichier introuvable",
                format!("{input} : {e}"),
                "ERR_FILE_NOT_FOUND",
            )
        })?;
        return Url::from_file_path(&absolute).map_err(|()| invalid(&"chemin non convertible"));
    }
    if !input.contains(char::is_whitespace) && looks_like_host(input) {
        return Url::parse(&format!("https://{input}")).map_err(|e| invalid(&e));
    }
    if let Ok(url) = Url::parse(input)
        && matches!(url.scheme(), "http" | "https" | "file" | "about" | "data")
    {
        return Ok(url);
    }
    Url::parse_with_params(SEARCH_URL, [("q", input)]).map_err(|e| invalid(&e))
}

fn read_file(url: &Url) -> Result<Vec<u8>, LoadError> {
    let path = url
        .to_file_path()
        .map_err(|()| LoadError::other("Chemin invalide", url.to_string(), "ERR_INVALID_URL"))?;
    let not_found = |e: std::io::Error| {
        LoadError::other(
            "Fichier introuvable",
            format!("{} : {e}", path.display()),
            "ERR_FILE_NOT_FOUND",
        )
    };
    let size = std::fs::metadata(&path).map_err(not_found)?.len();
    if size > MAX_RESOURCE_BYTES {
        return Err(LoadError::other(
            "Fichier trop volumineux",
            format!("{} ({size} octets)", path.display()),
            "ERR_FILE_TOO_BIG",
        ));
    }
    std::fs::read(&path).map_err(not_found)
}

/// Décode une URL `data:` (RFC 2397) : `data:[type][;base64],données`.
fn read_data_url(url: &Url) -> Result<Vec<u8>, LoadError> {
    let invalid = || {
        LoadError::other(
            "Adresse invalide",
            "URL data: mal formée".to_owned(),
            "ERR_INVALID_URL",
        )
    };
    // `Url::path` exclut la requête, qui fait pourtant partie des données.
    let rest = &url[url::Position::BeforePath..url::Position::AfterQuery];
    let (meta, payload) = rest.split_once(',').ok_or_else(invalid)?;
    let bytes: Vec<u8> = percent_encoding::percent_decode_str(payload).collect();
    if meta.trim_end().to_ascii_lowercase().ends_with(";base64") {
        // Blancs ignorés ; le remplissage « = » est facultatif dans les URL data:.
        let mut compact: Vec<u8> = bytes
            .into_iter()
            .filter(|b| !b.is_ascii_whitespace())
            .collect();
        while compact.last() == Some(&b'=') {
            compact.pop();
        }
        base64::engine::general_purpose::STANDARD_NO_PAD
            .decode(compact)
            .map_err(|_| invalid())
    } else {
        Ok(bytes)
    }
}

/// Client HTTP partagé par tous les chargements du processus.
fn http_client() -> Result<&'static oxyde_net::Client, LoadError> {
    static CLIENT: OnceLock<Result<oxyde_net::Client, String>> = OnceLock::new();
    CLIENT
        .get_or_init(|| oxyde_net::Client::new().map_err(|e| e.to_string()))
        .as_ref()
        .map_err(|e| LoadError::other("Réseau indisponible", e.clone(), "ERR_FAILED"))
}

/// Traduit une erreur réseau en page d'erreur de la maquette.
fn network_error(url: &Url, error: &NetError) -> LoadError {
    let host = url.host_str().unwrap_or("ce site");
    let code = error.code();
    let detail = format!("{url} — {error}");
    if error.is_certificate_error() {
        let reason = match code {
            "NET::ERR_CERT_DATE_INVALID" => "a expiré ou n'est pas encore valable",
            "NET::ERR_CERT_COMMON_NAME_INVALID" => "n'est pas délivré pour ce nom de site",
            "NET::ERR_CERT_AUTHORITY_INVALID" => "n'émane pas d'une autorité reconnue",
            "NET::ERR_CERT_REVOKED" => "a été révoqué",
            _ => "n'est pas valide",
        };
        return LoadError {
            kind: ErrorKind::Certificate,
            title: "Cette connexion n'est pas sécurisée".to_owned(),
            message: format!(
                "Le certificat de {host} {reason}. Quelqu'un pourrait intercepter ce que vous \
                 envoyez à ce site : mots de passe, messages, coordonnées bancaires."
            ),
            code,
            detail,
        };
    }
    let (kind, title, message) = match error {
        NetError::Connect { .. } | NetError::Timeout => (
            ErrorKind::Network,
            if code == "ERR_INTERNET_DISCONNECTED" {
                "Pas de connexion à Internet"
            } else {
                "Ce site est inaccessible"
            },
            format!(
                "Oxyde ne parvient pas à joindre {host}. Vérifiez le câble réseau ou le Wi-Fi, \
                 puis le proxy et le pare-feu."
            ),
        ),
        NetError::Tls { .. } => (
            ErrorKind::Network,
            "La connexion sécurisée a échoué",
            format!("{host} n'a pas pu établir une connexion chiffrée avec Oxyde."),
        ),
        NetError::TooManyRedirects => (
            ErrorKind::Other,
            "Trop de redirections",
            format!("{host} vous redirige en boucle."),
        ),
        _ => (
            ErrorKind::Other,
            "La page n'a pas pu être chargée",
            format!("Le chargement de {host} a échoué."),
        ),
    };
    LoadError {
        kind,
        title: title.to_owned(),
        message,
        code,
        detail,
    }
}

fn fetch_http(url: &Url) -> Result<(Url, Vec<u8>), LoadError> {
    let response = http_client()?
        .get(url)
        .map_err(|e| network_error(url, &e))?;
    if response.status >= 400 && response.body.is_empty() {
        return Err(LoadError::other(
            "La page n'a pas pu être chargée",
            format!(
                "{url} — le serveur a répondu avec le code {}.",
                response.status
            ),
            "ERR_HTTP_RESPONSE_CODE_FAILURE",
        ));
    }
    Ok((response.url, response.body))
}

/// Exécute une requête `fetch` émise par un script. L'échec est décrit par
/// un message destiné au script (`TypeError`).
pub(crate) fn fetch_for_script(url: &Url) -> Result<oxyde_content::FetchResponse, String> {
    // La page ne demande que du http(s) ; on le revérifie avant d'ouvrir une connexion.
    if !matches!(url.scheme(), "http" | "https") {
        return Err(format!("protocole « {} » refusé", url.scheme()));
    }
    let client = http_client().map_err(|e| e.message)?;
    let response = client.get(url).map_err(|e| e.to_string())?;
    Ok(oxyde_content::FetchResponse {
        url: response.url,
        status: response.status,
        headers: response.headers,
        body: response.body,
    })
}

/// Exécute, l'une après l'autre, les requêtes `fetch` en attente de `page`
/// jusqu'à ce qu'elle n'en émette plus (rendu hors écran : pas de boucle
/// d'événements pour les traiter en arrière-plan).
pub(crate) fn settle_fetches(page: &mut Page) {
    /// Borne le nombre de vagues : une page qui relance sans fin est abandonnée.
    const MAX_ROUNDS: usize = 16;
    for _ in 0..MAX_ROUNDS {
        let requests = page.take_fetches();
        if requests.is_empty() {
            break;
        }
        for request in requests {
            page.complete_fetch(request.id, fetch_for_script(&request.url));
        }
    }
    report_console(page);
}

/// Récupère la ressource `url` ; renvoie son adresse finale (après
/// redirections) et ses octets.
pub(crate) fn fetch(url: &Url) -> Result<(Url, Vec<u8>), LoadError> {
    match url.scheme() {
        "file" => read_file(url).map(|bytes| (url.clone(), bytes)),
        // Les pages internes (nouvel onglet) sont dessinées par l'interface.
        "about" => Ok((url.clone(), Vec::new())),
        "data" => read_data_url(url).map(|bytes| (url.clone(), bytes)),
        "http" | "https" => fetch_http(url),
        other => Err(LoadError::other(
            "Protocole non pris en charge",
            format!("Le chargement en « {other}: » n'est pas disponible ({url})."),
            "ERR_UNKNOWN_URL_SCHEME",
        )),
    }
}

/// Décode les octets d'une ressource texte.
///
/// LIMITE : l'encodage est supposé UTF-8 (pas encore de détection par
/// `<meta charset>` ni par l'en-tête HTTP) ; la marque d'ordre d'octets est retirée.
pub(crate) fn decode(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    String::from_utf8_lossy(bytes).into_owned()
}

/// Charge la page `url`, ses feuilles de style et ses images.
pub(crate) fn load(url: Url) -> Loaded {
    let (url, html) = match fetch(&url) {
        Ok((final_url, bytes)) => (final_url, decode(&bytes)),
        Err(error) => {
            return Loaded {
                page: Page::from_html(url, ""),
                error: Some(error),
            };
        }
    };
    let mut page = Page::from_html(url, &html);
    for sheet_url in page.pending_stylesheets() {
        // Une feuille indisponible est remplacée par une feuille vide : la page reste lisible.
        let css = fetch(&sheet_url)
            .map(|(_, bytes)| decode(&bytes))
            .unwrap_or_default();
        page.set_stylesheet(&sheet_url, &css);
    }
    for image_url in page.pending_images() {
        // Une image indisponible ou illisible laisse place à son texte alternatif.
        let image = fetch(&image_url)
            .ok()
            .and_then(|(_, bytes)| decode_image(&bytes));
        page.set_image(&image_url, image);
    }
    for script_url in page.pending_scripts() {
        // Un script indisponible est sauté ; les suivants s'exécutent quand même.
        let source = fetch(&script_url).ok().map(|(_, bytes)| decode(&bytes));
        page.set_script(&script_url, source);
    }
    page.run_scripts();
    report_console(&mut page);
    Loaded { page, error: None }
}

/// Écrit sur la sortie d'erreur les messages de console de la page.
///
/// LIMITE : en attendant les outils de développement (jalon 9), la console
/// des pages n'est visible que depuis le terminal qui a lancé Oxyde.
pub(crate) fn report_console(page: &mut Page) {
    for message in page.take_console() {
        eprintln!("[console.{}] {}", message.level, message.text);
    }
}

/// Charge ce que l'utilisateur a saisi (voir [`resolve_input`]).
pub(crate) fn load_input(input: &str) -> Loaded {
    match resolve_input(input) {
        Ok(url) => load(url),
        Err(error) => Loaded {
            page: blank_page(),
            error: Some(error),
        },
    }
}

/// Adresse de la page « Nouvel onglet ».
pub(crate) fn new_tab_url() -> Url {
    Url::parse("about:newtab").expect("URL constante valide")
}

/// Page vide à l'adresse du nouvel onglet.
pub(crate) fn blank_page() -> Page {
    Page::from_html(new_tab_url(), "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_resolution() {
        let resolved = |s: &str| resolve_input(s).map(String::from).unwrap_or_default();
        assert_eq!(resolved("exemple.fr/a?b=1"), "https://exemple.fr/a?b=1");
        assert_eq!(resolved("localhost:8080"), "https://localhost:8080/");
        assert_eq!(resolved("http://exemple.fr"), "http://exemple.fr/");
        assert_eq!(resolved("about:newtab"), "about:newtab");
        assert_eq!(
            resolved("moteur de rendu"),
            "https://html.duckduckgo.com/html/?q=moteur+de+rendu"
        );
        assert_eq!(resolved("rust"), "https://html.duckduckgo.com/html/?q=rust");
        assert_eq!(
            resolved("c++ & rust?"),
            "https://html.duckduckgo.com/html/?q=c%2B%2B+%26+rust%3F"
        );
    }

    #[test]
    fn data_urls() {
        let bytes = |s: &str| read_data_url(&Url::parse(s).expect("url")).ok();
        assert_eq!(
            bytes("data:,bonjour%20monde"),
            Some(b"bonjour monde".to_vec())
        );
        assert_eq!(
            bytes("data:text/plain;base64,T3h5ZGU="),
            Some(b"Oxyde".to_vec())
        );
        assert_eq!(
            bytes("data:text/plain;base64,T3h5ZGU"),
            Some(b"Oxyde".to_vec())
        );
        assert_eq!(bytes("data:text/html,<p>a?b#c"), Some(b"<p>a?b".to_vec()));
        assert_eq!(bytes("data:;base64,@@@"), None);
        assert_eq!(bytes("data:sans-virgule"), None);
    }

    #[test]
    fn failed_load_keeps_the_requested_address() {
        let url = Url::parse("file:///Z:/oxyde/absent-0123456789.html").expect("url");
        let loaded = load(url.clone());
        let error = loaded.error.expect("erreur attendue");
        assert_eq!(error.kind, ErrorKind::Other);
        assert_eq!(error.code, "ERR_FILE_NOT_FOUND");
        assert_eq!(loaded.page.url(), &url);
    }
}
