//! Chargement des ressources d'une page : `file:`, `about:`, `data:` et `http(s):`.

use std::path::Path;

use base64::Engine as _;
use oxyde_content::{Page, Url, decode_image};

use crate::network::{self, FailureClass, NetFailure};

/// Taille maximale d'une ressource locale chargée en mémoire (64 Mio).
const MAX_RESOURCE_BYTES: u64 = 64 * 1024 * 1024;

/// Moteurs de recherche proposés dans les paramètres. Le premier est celui
/// par défaut : sa version HTML fonctionne sans JavaScript.
pub(crate) const SEARCH_ENGINE_NAMES: [&str; 4] = ["DuckDuckGo", "Brave", "Mojeek", "Google"];

/// Adresse de recherche de chaque moteur de [`SEARCH_ENGINE_NAMES`] ; la
/// requête va dans le paramètre `q`.
const SEARCH_URLS: [&str; 4] = [
    "https://html.duckduckgo.com/html/",
    "https://search.brave.com/search",
    "https://www.mojeek.com/search",
    "https://www.google.com/search",
];

/// Nature d'une erreur de chargement ; détermine la page d'erreur affichée.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ErrorKind {
    /// Le site n'a pas pu être joint.
    Network,
    /// Le certificat du site a été refusé.
    Certificate,
    /// Autre échec (fichier introuvable, adresse invalide…).
    Other,
    /// Le processus de rendu de l'onglet s'est arrêté.
    Crashed,
}
oxyde_ipc::ipc_enum!(ErrorKind {
    0 => Network,
    1 => Certificate,
    2 => Other,
    3 => Crashed,
});

/// Erreur de chargement, présentée à l'utilisateur dans une page d'erreur.
#[derive(Clone, Debug)]
pub(crate) struct LoadError {
    pub(crate) kind: ErrorKind,
    pub(crate) title: String,
    pub(crate) message: String,
    /// Code au format de Chromium, affiché au bas de la page.
    pub(crate) code: String,
    /// Détail technique, affiché sur demande.
    pub(crate) detail: String,
}
oxyde_ipc::ipc_struct!(LoadError {
    kind,
    title,
    message,
    code,
    detail
});

impl LoadError {
    pub(crate) fn other(title: &str, message: String, code: &str) -> Self {
        Self {
            kind: ErrorKind::Other,
            title: title.to_owned(),
            detail: message.clone(),
            message,
            code: code.to_owned(),
        }
    }

    /// Erreur affichée quand le processus de rendu d'un onglet a disparu.
    pub(crate) fn crashed(url: &Url) -> Self {
        Self {
            kind: ErrorKind::Crashed,
            title: "Cet onglet a planté".to_owned(),
            message: "Le processus qui affichait cette page s'est arrêté. Les autres onglets ne sont pas touchés : rechargez la page pour la retrouver."
                .to_owned(),
            code: "ERR_RENDERER_CRASHED".to_owned(),
            detail: url.to_string(),
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
    resolve_input_with(input, 0)
}

/// Comme [`resolve_input`] ; une recherche va au moteur d'index `engine`
/// dans [`SEARCH_ENGINE_NAMES`] (le premier si l'index est hors bornes).
pub(crate) fn resolve_input_with(input: &str, engine: usize) -> Result<Url, LoadError> {
    let search = SEARCH_URLS.get(engine).copied().unwrap_or(SEARCH_URLS[0]);
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
    Url::parse_with_params(search, [("q", input)]).map_err(|e| invalid(&e))
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

/// Traduit une erreur réseau en page d'erreur de la maquette.
fn network_error(url: &Url, error: &NetFailure) -> LoadError {
    let host = url.host_str().unwrap_or("ce site");
    let code = error.code.as_str();
    let detail = format!("{url} — {}", error.text);
    if error.certificate {
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
            code: code.to_owned(),
            detail,
        };
    }
    let (kind, title, message) = match error.class {
        FailureClass::Unreachable => (
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
        FailureClass::Tls => (
            ErrorKind::Network,
            "La connexion sécurisée a échoué",
            format!("{host} n'a pas pu établir une connexion chiffrée avec Oxyde."),
        ),
        FailureClass::Redirects => (
            ErrorKind::Other,
            "Trop de redirections",
            format!("{host} vous redirige en boucle."),
        ),
        FailureClass::Other => (
            ErrorKind::Other,
            "La page n'a pas pu être chargée",
            format!("Le chargement de {host} a échoué."),
        ),
    };
    LoadError {
        kind,
        title: title.to_owned(),
        message,
        code: code.to_owned(),
        detail,
    }
}

fn fetch_http(url: &Url, initiator: Option<&Url>) -> Result<(Url, Vec<u8>), LoadError> {
    let request = oxyde_net::HttpRequest {
        initiator: initiator.cloned(),
        ..oxyde_net::HttpRequest::get(url)
    };
    let response = network::send(&request).map_err(|e| network_error(url, &e))?;
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

/// Demande la page `url` par `POST`, avec le corps d'un formulaire.
pub(crate) fn post_http(
    url: &Url,
    form: &oxyde_content::FormPost,
) -> Result<(Url, Vec<u8>), LoadError> {
    // Un processus de rendu n'ouvre pas de connexion : il demande au navigateur.
    if let Some(link) = crate::renderer::child::link() {
        return link.post_form(url, form);
    }
    let response = network::send(&oxyde_net::HttpRequest {
        method: "POST".to_owned(),
        url: url.clone(),
        headers: vec![("Content-Type".to_owned(), form.content_type.clone())],
        body: form.body.clone(),
        initiator: None,
    })
    .map_err(|e| network_error(url, &e))?;
    Ok((response.url, response.body))
}

/// Exécute une requête `fetch` émise par un script, précédée de sa
/// pré-vérification CORS si la page la demande. L'échec est décrit par un
/// message destiné au script (`TypeError`).
pub(crate) fn fetch_for_script(
    request: &oxyde_content::FetchRequest,
) -> Result<oxyde_content::FetchResponse, String> {
    // La page ne demande que du http(s) ; on le revérifie avant d'ouvrir une connexion.
    if !matches!(request.url.scheme(), "http" | "https") {
        return Err(format!("protocole « {} » refusé", request.url.scheme()));
    }
    let send = |method: &str, headers: Vec<(String, String)>, body: Vec<u8>| {
        network::send(&oxyde_net::HttpRequest {
            method: method.to_owned(),
            url: request.url.clone(),
            headers,
            body,
            initiator: Some(request.initiator.clone()),
        })
        .map(|response| oxyde_content::FetchResponse {
            url: response.url,
            status: response.status,
            headers: response.headers,
            body: response.body,
        })
        .map_err(|e| e.text)
    };
    if request.needs_preflight {
        let answer = send("OPTIONS", request.preflight_headers(), Vec::new())?;
        oxyde_content::preflight_allows(request, &answer)?;
    }
    let mut headers = request.headers.clone();
    if let Some(origin) = &request.origin {
        headers.push(("Origin".to_owned(), origin.clone()));
    }
    send(&request.method, headers, request.body.clone())
}

/// Charge les feuilles de style et les images apparues dans `page` après
/// son chargement (ajoutées par ses scripts). Renvoie `true` s'il y en avait.
///
/// LIMITE : la page attend pendant que ses ressources arrivent.
pub(crate) fn load_late_resources(page: &mut Page) -> bool {
    let document = page.url().clone();
    let sheets = page.pending_stylesheets();
    let images = page.pending_images();
    if sheets.is_empty() && images.is_empty() {
        return false;
    }
    for (sheet_url, fetched) in sheets.iter().zip(fetch_many(&sheets, &document)) {
        let css = fetched.map(|(_, bytes)| decode(&bytes)).unwrap_or_default();
        page.set_stylesheet(sheet_url, &css);
    }
    for (image_url, fetched) in images.iter().zip(fetch_many(&images, &document)) {
        let image = fetched.ok().and_then(|(_, bytes)| decode_image(&bytes));
        page.set_image(image_url, image);
    }
    true
}

/// Laisse `page` finir de se construire, pour un rendu hors écran (pas de
/// boucle d'événements) : exécute, l'une après l'autre, ses requêtes
/// (`fetch`, scripts insérés) et ses minuteurs, en avançant un temps virtuel
/// de deux secondes au plus.
pub(crate) fn settle_fetches(page: &mut Page) {
    /// Borne le nombre de vagues : une page qui relance sans fin est abandonnée.
    const MAX_ROUNDS: u32 = 40;
    /// Temps virtuel écoulé entre deux vagues.
    const STEP: std::time::Duration = std::time::Duration::from_millis(50);
    /// Durée réelle au-delà de laquelle la page est rendue telle quelle.
    const DEADLINE: std::time::Duration = std::time::Duration::from_secs(20);
    /// Vagues sans activité après lesquelles la page est tenue pour stable.
    const QUIET_ROUNDS: u32 = 4;
    let start = std::time::Instant::now();
    let mut quiet = 0;
    for round in 1..=MAX_ROUNDS {
        let requests = page.take_fetches();
        let mut active = !requests.is_empty();
        // Les requêtes d'une vague partent ensemble ; les réponses sont
        // remises dans l'ordre d'émission.
        for batch in requests.chunks(PARALLEL_FETCHES) {
            let answers: Vec<_> = std::thread::scope(|scope| {
                let workers: Vec<_> = batch
                    .iter()
                    .map(|request| scope.spawn(move || fetch_for_script(request)))
                    .collect();
                workers
                    .into_iter()
                    .map(|worker| {
                        worker
                            .join()
                            .unwrap_or_else(|_| Err("le chargement a été interrompu".to_owned()))
                    })
                    .collect()
            });
            for (request, answer) in batch.iter().zip(answers) {
                page.complete_fetch(request.id, answer);
            }
        }
        active |= page.tick(start + STEP * round);
        active |= load_late_resources(page);
        quiet = if active { 0 } else { quiet + 1 };
        if quiet >= QUIET_ROUNDS || start.elapsed() > DEADLINE {
            break;
        }
    }
    report_console(page);
}

/// Nombre de ressources d'une page demandées en même temps.
const PARALLEL_FETCHES: usize = 8;

/// Récupère les ressources `urls` du document `initiator`, plusieurs à la
/// fois ; les résultats sont rendus dans l'ordre de `urls`.
pub(crate) fn fetch_many(urls: &[Url], initiator: &Url) -> Vec<Result<(Url, Vec<u8>), LoadError>> {
    let mut results = Vec::with_capacity(urls.len());
    for batch in urls.chunks(PARALLEL_FETCHES) {
        // Un processus de rendu n'a qu'un fil : il émet ses demandes d'un
        // coup, le navigateur les sert en parallèle.
        // Les adresses `data:` et `about:` ne sortent pas du processus.
        let remote = |url: &Url| !matches!(url.scheme(), "data" | "about");
        if let Some(link) = crate::renderer::child::link() {
            let asked: Vec<Url> = batch.iter().filter(|url| remote(url)).cloned().collect();
            let mut answers = link.get_many(&asked, initiator).into_iter();
            for url in batch {
                results.push(if remote(url) {
                    answers
                        .next()
                        .unwrap_or_else(|| fetch_from(url, Some(initiator)))
                } else {
                    fetch_from(url, Some(initiator))
                });
            }
            continue;
        }
        let fetched: Vec<_> = std::thread::scope(|scope| {
            let workers: Vec<_> = batch
                .iter()
                .map(|url| scope.spawn(move || fetch_from(url, Some(initiator))))
                .collect();
            workers
                .into_iter()
                .zip(batch)
                .map(|(worker, url)| {
                    worker.join().unwrap_or_else(|_| {
                        Err(LoadError::other(
                            "La page n'a pas pu être chargée",
                            format!("{url} — le chargement a été interrompu."),
                            "ERR_FAILED",
                        ))
                    })
                })
                .collect()
        });
        results.extend(fetched);
    }
    results
}

/// Récupère la ressource `url` ; renvoie son adresse finale (après
/// redirections) et ses octets.
pub(crate) fn fetch(url: &Url) -> Result<(Url, Vec<u8>), LoadError> {
    fetch_from(url, None)
}

/// Comme [`fetch`], pour une ressource demandée par le document
/// `initiator` (feuille de style, image, script) : les cookies d'un autre
/// site ne l'accompagnent pas.
pub(crate) fn fetch_from(url: &Url, initiator: Option<&Url>) -> Result<(Url, Vec<u8>), LoadError> {
    // Un processus de rendu n'ouvre ni fichier ni connexion : il demande au
    // navigateur. Les adresses `data:` et `about:` ne sortent pas du processus.
    if let Some(link) = crate::renderer::child::link()
        && !matches!(url.scheme(), "data" | "about")
    {
        return link.get(url, initiator);
    }
    match url.scheme() {
        "file" => read_file(url).map(|bytes| (url.clone(), bytes)),
        // Les pages internes (nouvel onglet) sont dessinées par l'interface.
        "about" => Ok((url.clone(), Vec::new())),
        "data" => read_data_url(url).map(|bytes| (url.clone(), bytes)),
        "http" | "https" => fetch_http(url, initiator),
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
/// Profondeur maximale d'un graphe de modules suivi au chargement.
const MAX_MODULE_ROUNDS: usize = 64;

pub(crate) fn load(url: Url) -> Loaded {
    load_with(url, None)
}

/// Taille de la zone où les pages s'affichent, telle que la dernière image
/// l'a établie : les scripts d'une page qui se charge la voient avant sa
/// première mise en page.
static VIEWPORT_HINT: std::sync::Mutex<Option<(f32, f32)>> = std::sync::Mutex::new(None);

/// Note la taille de la zone où les pages s'affichent.
pub(crate) fn set_viewport_hint(width: f32, height: f32) {
    if width > 0.0 && height > 0.0 {
        // Un verrou empoisonné ne protège qu'une taille : on la remplace.
        *VIEWPORT_HINT
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some((width, height));
    }
}

/// Taille notée par [`set_viewport_hint`] ; nulle si elle n'est pas connue.
pub(crate) fn viewport_size() -> (f32, f32) {
    viewport_hint().map_or((0.0, 0.0), |v| (v.width, v.height))
}

/// Onglet qui attend son chargement : une page vide à l'adresse demandée.
/// Les pages internes sont chargées sur le champ.
pub(crate) fn placeholder(input: &str) -> Loaded {
    match resolve_input(input) {
        Ok(url) if crate::renderer::host::is_remote(&url) => Loaded {
            page: Page::from_html(url, ""),
            error: None,
        },
        Ok(url) => load(url),
        Err(error) => Loaded {
            page: blank_page(),
            error: Some(error),
        },
    }
}

fn viewport_hint() -> Option<oxyde_css::Viewport> {
    let (width, height) = (*VIEWPORT_HINT
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner))?;
    Some(oxyde_css::Viewport { width, height })
}

/// Comme [`load`] ; `post` : la page est demandée par `POST` avec ce corps
/// (soumission d'un formulaire).
pub(crate) fn load_with(url: Url, post: Option<oxyde_content::FormPost>) -> Loaded {
    // Adresse de diagnostic. Dans un processus de rendu, elle le fait
    // planter pour de bon ; chargée dans le navigateur lui-même, elle montre
    // seulement l'écran correspondant.
    if url.as_str() == "about:crash" {
        return Loaded {
            error: Some(LoadError::crashed(&url)),
            page: Page::from_html(url, ""),
        };
    }
    let first = match &post {
        Some(form) if matches!(url.scheme(), "http" | "https") => post_http(&url, form),
        _ => fetch(&url),
    };
    let (url, html) = match first {
        Ok((final_url, bytes)) => (final_url, decode(&bytes)),
        Err(error) => {
            return Loaded {
                page: Page::from_html(url, ""),
                error: Some(error),
            };
        }
    };
    let mut page = Page::from_html(url, &html);
    let document = page.url().clone();
    if let Some(cookies) = read_cookies(&document) {
        page.set_cookies(&cookies);
    }
    let sheets = page.pending_stylesheets();
    for (sheet_url, fetched) in sheets.iter().zip(fetch_many(&sheets, &document)) {
        // Une feuille indisponible est ignorée : la page s'affiche sans elle.
        let css = fetched.map(|(_, bytes)| decode(&bytes)).unwrap_or_default();
        page.set_stylesheet(sheet_url, &css);
    }
    let fonts = page.pending_fonts();
    for (font_url, fetched) in fonts.iter().zip(fetch_many(&fonts, &document)) {
        // Une police indisponible laisse place aux polices du système.
        page.set_font(font_url, fetched.ok().map(|(_, bytes)| bytes));
    }
    let images = page.pending_images();
    for (image_url, fetched) in images.iter().zip(fetch_many(&images, &document)) {
        // Une image indisponible ou illisible laisse place à son texte alternatif.
        let image = fetched.ok().and_then(|(_, bytes)| decode_image(&bytes));
        page.set_image(image_url, image);
    }
    let frames = page.pending_frames();
    for (frame_url, fetched) in frames.iter().zip(fetch_many(&frames, &document)) {
        // Un cadre indisponible reste vide.
        let html = fetched.ok().map(|(_, bytes)| decode(&bytes));
        page.set_frame(frame_url, html.as_deref());
    }
    let scripts = page.pending_scripts();
    for (script_url, fetched) in scripts.iter().zip(fetch_many(&scripts, &document)) {
        // Un script indisponible est sauté ; les suivants s'exécutent quand même.
        let source = fetched.ok().map(|(_, bytes)| decode(&bytes));
        page.set_script(script_url, source);
    }
    // Modules importés : un module reçu peut en importer d'autres.
    for _ in 0..MAX_MODULE_ROUNDS {
        let pending = page.pending_modules();
        if pending.is_empty() {
            break;
        }
        for module_url in pending {
            let source = fetch_from(&module_url, Some(&document))
                .ok()
                .map(|(_, bytes)| decode(&bytes));
            page.set_module(&module_url, source);
        }
    }
    if let Some(viewport) = viewport_hint() {
        page.set_viewport(viewport);
    }
    page.run_scripts();
    report_console(&mut page);
    Loaded { page, error: None }
}

/// Soumet au pot du client HTTP les cookies écrits par les scripts de la
/// page, puis rend à la page la vue à jour de `document.cookie`.
fn sync_cookies(page: &mut Page) {
    let writes = page.take_cookie_writes();
    let document = page.url().clone();
    if !matches!(document.scheme(), "http" | "https") {
        return;
    }
    // Dans un processus de rendu, chaque lecture est un aller-retour avec le
    // navigateur : la vue de `document.cookie` n'est rafraîchie qu'après une
    // écriture.
    //
    // LIMITE : un cookie posé entre-temps par une réponse HTTP n'y apparaît
    // qu'au chargement suivant ou après la prochaine écriture.
    if writes.is_empty() && crate::renderer::child::link().is_some() {
        return;
    }
    for text in &writes {
        write_cookie(&document, text);
    }
    if let Some(cookies) = read_cookies(&document) {
        page.set_cookies(&cookies);
    }
}

/// Valeur de `document.cookie` pour `document`.
pub(crate) fn read_cookies(document: &Url) -> Option<String> {
    if let Some(link) = crate::renderer::child::link() {
        return link.cookies(document);
    }
    network::cookies(document)
}

/// Soumet au pot de cookies un cookie écrit par un script de `document`.
pub(crate) fn write_cookie(document: &Url, text: &str) {
    if let Some(link) = crate::renderer::child::link() {
        link.set_cookie(document, text);
    } else {
        network::set_cookie(document, text);
    }
}

/// Écrit sur la sortie d'erreur les messages de console de la page.
///
/// C'est aussi le point où les cookies de la page et ceux du client HTTP
/// sont accordés : il est appelé après chaque tâche de script.
///
/// LIMITE : en attendant les outils de développement (jalon 9), la console
/// des pages n'est visible que depuis le terminal qui a lancé Oxyde.
pub(crate) fn report_console(page: &mut Page) {
    sync_cookies(page);
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
