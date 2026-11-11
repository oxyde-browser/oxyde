//! Chargement des ressources d'une page : `file:`, `about:` et `http(s):`.

use std::path::Path;
use std::sync::OnceLock;

use oxyde_content::{Page, Url};

/// Taille maximale d'une ressource locale chargée en mémoire (64 Mio).
const MAX_RESOURCE_BYTES: u64 = 64 * 1024 * 1024;

/// Erreur de chargement, présentée à l'utilisateur dans une page d'erreur.
#[derive(Debug)]
pub(crate) struct LoadError {
    pub(crate) title: &'static str,
    pub(crate) detail: String,
}

/// Interprète ce que l'utilisateur a saisi : URL absolue, chemin local, ou
/// nom d'hôte sans schéma (complété en `https://`).
pub(crate) fn resolve_input(input: &str) -> Result<Url, LoadError> {
    let input = input.trim();
    let path = Path::new(input);
    if path.exists() {
        let absolute = path.canonicalize().map_err(|e| LoadError {
            title: "Fichier introuvable",
            detail: format!("{input} : {e}"),
        })?;
        return Url::from_file_path(&absolute).map_err(|()| LoadError {
            title: "Chemin invalide",
            detail: absolute.display().to_string(),
        });
    }
    if let Ok(url) = Url::parse(input)
        && matches!(url.scheme(), "http" | "https" | "file" | "about")
    {
        return Ok(url);
    }
    Url::parse(&format!("https://{input}")).map_err(|e| LoadError {
        title: "Adresse invalide",
        detail: format!("{input} : {e}"),
    })
}

fn read_file(url: &Url) -> Result<Vec<u8>, LoadError> {
    let path = url.to_file_path().map_err(|()| LoadError {
        title: "Chemin invalide",
        detail: url.to_string(),
    })?;
    let not_found = |e: std::io::Error| LoadError {
        title: "Fichier introuvable",
        detail: format!("{} : {e}", path.display()),
    };
    let size = std::fs::metadata(&path).map_err(not_found)?.len();
    if size > MAX_RESOURCE_BYTES {
        return Err(LoadError {
            title: "Fichier trop volumineux",
            detail: format!("{} ({size} octets)", path.display()),
        });
    }
    std::fs::read(&path).map_err(not_found)
}

/// Client HTTP partagé par tous les chargements du processus.
fn http_client() -> Result<&'static oxyde_net::Client, LoadError> {
    static CLIENT: OnceLock<Result<oxyde_net::Client, String>> = OnceLock::new();
    CLIENT
        .get_or_init(|| oxyde_net::Client::new().map_err(|e| e.to_string()))
        .as_ref()
        .map_err(|e| LoadError {
            title: "Réseau indisponible",
            detail: e.clone(),
        })
}

fn fetch_http(url: &Url) -> Result<(Url, Vec<u8>), LoadError> {
    use oxyde_net::NetError;
    let response = http_client()?.get(url).map_err(|e| {
        let title = match &e {
            NetError::Connect { .. } => "Ce site est inaccessible",
            NetError::Tls { .. } => "La connexion n'est pas sécurisée",
            NetError::Timeout => "Le site met trop de temps à répondre",
            NetError::TooManyRedirects => "Trop de redirections",
            _ => "La page n'a pas pu être chargée",
        };
        LoadError {
            title,
            detail: format!("{url} — {e}"),
        }
    })?;
    if response.status >= 400 && response.body.is_empty() {
        return Err(LoadError {
            title: "La page n'a pas pu être chargée",
            detail: format!(
                "{url} — le serveur a répondu avec le code {}.",
                response.status
            ),
        });
    }
    Ok((response.url, response.body))
}

/// Récupère la ressource `url` ; renvoie son adresse finale (après
/// redirections) et ses octets.
pub(crate) fn fetch(url: &Url) -> Result<(Url, Vec<u8>), LoadError> {
    match url.scheme() {
        "file" => read_file(url).map(|bytes| (url.clone(), bytes)),
        "about" => Ok((url.clone(), about_page(url.path()).into_bytes())),
        "http" | "https" => fetch_http(url),
        other => Err(LoadError {
            title: "Protocole non pris en charge",
            detail: format!("Le chargement en « {other}: » n'est pas disponible ({url})."),
        }),
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

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Page d'erreur aux couleurs de la maquette.
fn error_page(error: &LoadError) -> String {
    format!(
        "<!DOCTYPE html><title>{title}</title>\
         <style>body{{margin:0;background:#0E1116;color:#E6E9ED;font-family:Inter,'Segoe UI',sans-serif}}\
         main{{max-width:560px;margin:0 auto;padding:120px 32px}}\
         h1{{margin:0;font-size:32px;line-height:40px;font-weight:600}}\
         p{{margin:16px 0 0 0;color:#A3ADB9;font-size:15px;line-height:24px}}</style>\
         <main><h1>{title}</h1><p>{detail}</p></main>",
        title = escape_html(error.title),
        detail = escape_html(&error.detail),
    )
}

fn about_page(name: &str) -> String {
    match name {
        "blank" => String::new(),
        _ => "<!DOCTYPE html><title>Nouvel onglet</title>\
              <style>body{margin:0;background:#0E1116;color:#E6E9ED;font-family:Inter,'Segoe UI',sans-serif}\
              main{max-width:560px;margin:0 auto;padding:160px 32px;text-align:center}\
              h1{margin:0;font-size:40px;line-height:48px;font-weight:600}\
              p{margin:16px 0 0 0;color:#A3ADB9;font-size:15px;line-height:24px}</style>\
              <main><h1>Oxyde</h1><p>Navigateur écrit en Rust. Saisissez une adresse avec Ctrl L.</p></main>"
            .to_owned(),
    }
}

/// Charge la page `url` et ses feuilles de style ; en cas d'échec, renvoie une
/// page d'erreur à la même adresse.
pub(crate) fn load_page(url: Url) -> Page {
    let (url, html) = match fetch(&url) {
        Ok((final_url, bytes)) => (final_url, decode(&bytes)),
        Err(error) => (url, error_page(&error)),
    };
    let mut page = Page::from_html(url, &html);
    for sheet_url in page.pending_stylesheets() {
        // Une feuille indisponible est remplacée par une feuille vide : la page reste lisible.
        let css = fetch(&sheet_url)
            .map(|(_, bytes)| decode(&bytes))
            .unwrap_or_default();
        page.set_stylesheet(&sheet_url, &css);
    }
    page
}

/// Page d'erreur pour une saisie qui n'a pas pu être interprétée.
pub(crate) fn error_page_for(error: &LoadError) -> Page {
    let url = Url::parse("about:erreur").expect("URL constante valide");
    Page::from_html(url, &error_page(error))
}
