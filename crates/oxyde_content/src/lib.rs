//! Page web d'Oxyde : enchaîne analyse HTML, style, scripts, mise en page et peinture.
//!
//! C'est le pendant, côté processus de rendu, de `//content/renderer` dans
//! Chromium : une [`Page`] possède le document, son moteur de script, et sait
//! produire sa liste d'affichage pour une taille de fenêtre donnée.
//!
//! LIMITES CONNUES : encodage supposé UTF-8, pas de mise en page incrémentale
//! (tout est recalculé à chaque changement), images limitées à `<img src>`
//! (pas de `srcset`, `<picture>` ni d'images CSS), images animées réduites à
//! leur première image ; les images et feuilles externes ajoutées par un
//! script ne sont pas chargées. Pour les scripts, voir le module `script`.

mod cors;
#[path = "generated/dom.rs"]
mod dom_idl;
mod script;
mod svg;
pub mod wire;

pub use cors::preflight_allows;

use std::collections::HashMap;
use std::io::Cursor;
use std::time::{Duration, Instant};

pub use oxyde_css::Viewport;
use oxyde_css::{Color, StyleMap, Stylesheet, parse_stylesheet};
use oxyde_dom::{Document, NodeId};
pub use oxyde_html::encoding;
pub use oxyde_js::ConsoleMessage;
use oxyde_layout::{Fragment, LayoutTree};
use oxyde_paint::DisplayList;
pub use oxyde_paint::Image;
use oxyde_text::TextContext;
pub use url::Url;

use crate::script::ScriptEngine;

/// Rapport de couverture du DOM : pour chaque interface décrite par les
/// fichiers Web IDL d'Oxyde, les membres réellement disponibles aux scripts.
/// Le rapport est établi en interrogeant une page d'essai, pas d'après une
/// liste tenue à la main.
#[must_use]
pub fn dom_coverage_report() -> String {
    script::dom_coverage_report()
}

/// Ordre qu'un script donne à une connexion `WebSocket` ou à un flux
/// `EventSource`, à exécuter par l'embarqueur (voir
/// [`Page::take_socket_commands`]).
///
/// Un flux `EventSource` passe par les mêmes ordres : son adresse est en
/// `http` ou `https`. L'embarqueur annonce alors [`SocketEvent::Open`] si
/// la réponse est un flux d'événements (code 200, type
/// `text/event-stream`), chaque morceau du corps par
/// [`SocketEvent::Binary`], et la fin par [`SocketEvent::Close`] : code
/// 1000 (flux fini) ou 1006 (coupure) si la page peut se reconnecter, tout
/// autre code pour un refus définitif.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SocketCommand {
    /// Ouvrir la connexion `id`.
    Open {
        /// Identifiant à rendre à [`Page::socket_event`].
        id: u64,
        /// Adresse, en `ws` ou `wss` ; en `http` ou `https` pour un flux
        /// d'événements.
        url: Url,
        /// Adresse du document qui ouvre la connexion (en-tête `Origin`,
        /// choix des cookies).
        document: Url,
        /// Sous-protocoles proposés au serveur ; pour un flux d'événements,
        /// le dernier identifiant d'événement reçu, s'il y en a un.
        protocols: Vec<String>,
    },
    /// Envoyer un message texte.
    Text {
        /// Connexion visée.
        id: u64,
        /// Le message.
        text: String,
    },
    /// Envoyer un message binaire.
    Binary {
        /// Connexion visée.
        id: u64,
        /// Le message.
        bytes: Vec<u8>,
    },
    /// Fermer la connexion.
    Close {
        /// Connexion visée.
        id: u64,
        /// Code de fermeture, s'il y en a un.
        code: Option<u16>,
        /// Raison de la fermeture.
        reason: String,
    },
}

impl SocketCommand {
    /// Connexion que vise cet ordre.
    #[must_use]
    pub fn id(&self) -> u64 {
        match self {
            Self::Open { id, .. }
            | Self::Text { id, .. }
            | Self::Binary { id, .. }
            | Self::Close { id, .. } => *id,
        }
    }
}

/// Ce qui arrive à une connexion `WebSocket`, à remettre à
/// [`Page::socket_event`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SocketEvent {
    /// La connexion est ouverte.
    Open {
        /// Sous-protocole retenu par le serveur, vide s'il n'y en a pas.
        protocol: String,
    },
    /// Message texte reçu.
    Text(String),
    /// Message binaire reçu.
    Binary(Vec<u8>),
    /// La connexion est finie ; aucun événement ne doit suivre.
    Close {
        /// Code de fermeture (1006 : coupure sans fermeture).
        code: u16,
        /// Raison donnée par le serveur.
        reason: String,
        /// Vrai si la fermeture s'est faite dans les règles.
        clean: bool,
    },
}

/// Requête réseau émise par un script (`fetch`), à exécuter par l'embarqueur.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FetchRequest {
    /// Identifiant à rendre à [`Page::complete_fetch`].
    pub id: u64,
    /// Adresse absolue, toujours en `http` ou `https`.
    pub url: Url,
    /// Méthode HTTP.
    pub method: String,
    /// En-têtes posés par le script (nom, valeur), déjà débarrassés de ceux
    /// qu'un script n'a pas le droit de fixer.
    pub headers: Vec<(String, String)>,
    /// Corps de la requête ; vide s'il n'y en a pas.
    pub body: Vec<u8>,
    /// Valeur de l'en-tête `Origin` à envoyer, s'il en faut un.
    pub origin: Option<String>,
    /// Adresse du document qui émet la requête (pour le choix des cookies).
    pub initiator: Url,
    /// La requête vise une autre origine et sort des requêtes « simples » :
    /// l'embarqueur doit d'abord envoyer une requête `OPTIONS` munie de
    /// [`FetchRequest::preflight_headers`] et soumettre la réponse à
    /// [`preflight_allows`].
    pub needs_preflight: bool,
}

/// Touche du clavier, telle que la décrivent les UI Events.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyInput<'a> {
    /// Valeur de la touche (`"a"`, `"Enter"`, `"ArrowLeft"`…).
    pub key: &'a str,
    /// Touche physique (`"KeyA"`, `"Enter"`…).
    pub code: &'a str,
    /// Touche Ctrl enfoncée.
    pub ctrl: bool,
    /// Touche Maj enfoncée.
    pub shift: bool,
    /// Touche Alt enfoncée.
    pub alt: bool,
    /// Touche Méta (Windows) enfoncée.
    pub meta: bool,
    /// Répétition automatique d'une touche maintenue.
    pub repeat: bool,
}

/// État du pointeur pour un événement de souris.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MouseInput {
    /// Abscisse du pointeur ; chaque fonction dit dans quel repère.
    pub x: f32,
    /// Ordonnée du pointeur.
    pub y: f32,
    /// Bouton en cause : 0 principal, 1 du milieu, 2 secondaire.
    pub button: u8,
    /// Boutons enfoncés : 1 principal, 2 secondaire, 4 du milieu, additionnés.
    pub buttons: u8,
    /// Rang du clic dans une série rapprochée (1 simple, 2 double) ; 0
    /// hors d'un clic.
    pub detail: u8,
    /// Touche Ctrl enfoncée.
    pub ctrl: bool,
    /// Touche Maj enfoncée.
    pub shift: bool,
    /// Touche Alt enfoncée.
    pub alt: bool,
    /// Touche Méta (Windows) enfoncée.
    pub meta: bool,
}

/// Étape d'un contact tactile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TouchPhase {
    /// Le doigt se pose.
    Start,
    /// Le doigt glisse.
    Move,
    /// Le doigt se lève.
    End,
    /// Le système interrompt le contact.
    Cancel,
}

/// Contact tactile en cours.
#[derive(Clone, Copy, Debug)]
struct TouchState {
    /// Point de départ, dans le repère de la page.
    start: (f32, f32),
    /// Le doigt s'est écarté du point de départ : ce n'est plus un appui.
    moved: bool,
    /// Un script a annulé `touchstart` ou `touchmove`.
    prevented: bool,
}

/// Écart, en pixels CSS, au-delà duquel un contact n'est plus un appui.
const TAP_SLOP: f32 = 10.0;

/// Corps d'une soumission de formulaire par la méthode `POST`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormPost {
    /// Valeur de l'en-tête `Content-Type`.
    pub content_type: String,
    /// Données du formulaire, déjà encodées.
    pub body: Vec<u8>,
}

/// Navigation demandée par un script, à exécuter par l'embarqueur.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Navigation {
    /// Charger `url` (`location.assign`, `location.href = …`, soumission
    /// d'un formulaire) ; `replace` : la page prend la place de l'entrée
    /// courante de l'historique.
    Load {
        /// Adresse à charger.
        url: Url,
        /// Remplacer l'entrée courante au lieu d'en ajouter une.
        replace: bool,
        /// Formulaire soumis par `POST` : la page est demandée avec ce corps.
        post: Option<FormPost>,
    },
    /// Recharger la page (`location.reload()`).
    Reload,
    /// Parcourir l'historique de ce nombre d'entrées (`history.back()`…).
    Traverse(i32),
    /// Le document garde sa place mais change d'adresse (`history.pushState`,
    /// `replaceState`, changement de fragment).
    SameDocument {
        /// Nouvelle adresse du document.
        url: Url,
        /// Remplacer l'entrée courante au lieu d'en ajouter une.
        replace: bool,
    },
}

/// Réponse à une [`FetchRequest`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FetchResponse {
    /// Adresse finale, après redirections.
    pub url: Url,
    /// Code d'état HTTP.
    pub status: u16,
    /// En-têtes de la réponse (nom, valeur).
    pub headers: Vec<(String, String)>,
    /// Corps.
    pub body: Vec<u8>,
}

/// Côté maximal d'une image décodée, en pixels.
const MAX_IMAGE_SIDE: u32 = 16_384;
/// Mémoire maximale allouée pour décoder une image (256 Mio).
const MAX_IMAGE_ALLOC: u64 = 256 * 1024 * 1024;
/// Nombre maximal d'images chargées par page.
const MAX_IMAGES: usize = 500;
/// Nombre maximal de scripts exécutés par page.
const MAX_SCRIPTS: usize = 500;

/// Décode une image PNG, JPEG, WebP, GIF, BMP ou ICO, ou rend une image
/// SVG à sa taille propre.
///
/// Les dimensions et la mémoire sont bornées : une image hostile (« bombe de
/// décompression ») est refusée plutôt que décodée. Renvoie `None` si le
/// format est inconnu, les données invalides ou les limites dépassées.
#[must_use]
pub fn decode_image(bytes: &[u8]) -> Option<Image> {
    decode_raster(bytes).or_else(|| svg::decode(bytes))
}

/// Décode une image en pixels (tout format sauf SVG).
fn decode_raster(bytes: &[u8]) -> Option<Image> {
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_SIDE);
    limits.max_image_height = Some(MAX_IMAGE_SIDE);
    limits.max_alloc = Some(MAX_IMAGE_ALLOC);
    reader.limits(limits);
    let rgba = reader.decode().ok()?.into_rgba8();
    let (width, height) = rgba.dimensions();
    Image::from_rgba(width, height, rgba.into_raw())
}

/// Feuille de style du document, dans l'ordre du source.
#[derive(Clone, Debug)]
enum SheetSlot {
    /// `<style>`
    Inline(Stylesheet),
    /// `<link rel=stylesheet>` ; `None` tant que la ressource n'est pas chargée.
    External { url: Url, sheet: Option<Stylesheet> },
}

/// Ce qu'il faut, outre le document, pour le mettre en page : ses feuilles
/// de style externes déjà reçues et la taille de ses images. Le moteur de
/// script en garde une copie pour répondre aux lectures de géométrie.
#[derive(Clone, Debug)]
pub(crate) struct LayoutEnv {
    sheets: Vec<SheetSlot>,
    base: Url,
    sizes: HashMap<NodeId, (f32, f32)>,
    /// Tailles des images déjà reçues, par adresse : un `<img>` qu'un
    /// script vient d'insérer (ou de cloner) a sa taille sans attendre.
    known_sizes: HashMap<String, (f32, f32)>,
    /// Images décodées de la page, par élément : un canevas peut les
    /// dessiner (`drawImage`, `createPattern`).
    pub(crate) pictures: HashMap<NodeId, Image>,
    fonts: Vec<FontSlot>,
    pub(crate) animations: std::sync::Arc<Animations>,
    pub(crate) scrolling: std::sync::Arc<Scrolling>,
}

/// Défilement de la fenêtre et des éléments, partagé entre la page — qui le
/// borne à chaque peinture et y route la molette — et son moteur de script.
#[derive(Debug, Default)]
pub(crate) struct Scrolling {
    tree: std::sync::Mutex<oxyde_compositor::ScrollTree>,
}

impl Scrolling {
    pub(crate) fn lock(&self) -> std::sync::MutexGuard<'_, oxyde_compositor::ScrollTree> {
        // Un verrou empoisonné garde des décalages valides : ils sont
        // bornés de nouveau à la prochaine peinture.
        self.tree
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// Identifiant de défilement d'un nœud.
pub(crate) fn scroll_id(node: NodeId) -> u32 {
    u32::try_from(node.index()).unwrap_or(oxyde_compositor::ROOT_SCROLLER - 1)
}

/// Durée d'une image d'animation.
const FRAME: Duration = Duration::from_millis(16);

/// Animations et transitions d'une page, et l'horloge qui les fait avancer.
/// La page et son moteur de script calculent le style chacun de leur côté :
/// ils partagent cet état pour voir les mêmes effets au même instant.
#[derive(Debug)]
pub(crate) struct Animations {
    /// Origine des temps de la page.
    epoch: Instant,
    state: std::sync::Mutex<AnimationClock>,
}

#[derive(Debug)]
struct AnimationClock {
    animator: oxyde_css::Animator,
    /// Instant le plus avancé communiqué à la page, en millisecondes depuis
    /// l'origine : un lanceur de tests fait avancer le temps plus vite que
    /// l'horloge réelle.
    latest: f64,
    /// Instant figé pendant une tâche de script : le temps des animations
    /// n'avance pas au milieu d'une tâche.
    frozen: Option<f64>,
}

impl Animations {
    fn new() -> Self {
        Self {
            epoch: Instant::now(),
            state: std::sync::Mutex::new(AnimationClock {
                animator: oxyde_css::Animator::new(),
                latest: 0.0,
                frozen: None,
            }),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, AnimationClock> {
        // Un verrou empoisonné garde un état cohérent : chaque calcul de
        // style repart d'un `begin`.
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Instant courant, en millisecondes depuis l'origine de la page.
    pub(crate) fn now(&self) -> f64 {
        let real = self.epoch.elapsed().as_secs_f64() * 1000.0;
        let state = self.lock();
        state.frozen.unwrap_or_else(|| real.max(state.latest))
    }

    /// Fige l'horloge à l'instant courant (`true`) ou la relâche : une tâche
    /// de script voit un seul instant, comme dans une image d'un navigateur.
    pub(crate) fn freeze(&self, frozen: bool) {
        let now = frozen.then(|| self.now());
        self.lock().frozen = now;
    }

    /// Fait avancer l'horloge jusqu'à `instant`.
    fn advance(&self, instant: Instant) {
        let ms = instant.saturating_duration_since(self.epoch).as_secs_f64() * 1000.0;
        let mut state = self.lock();
        state.latest = state.latest.max(ms);
    }

    /// Instant `ms` millisecondes après l'origine.
    fn instant_at(&self, ms: f64) -> Instant {
        self.epoch + Duration::from_secs_f64((ms / 1000.0).max(0.0))
    }

    /// Calcule les styles à l'instant courant. Renvoie aussi cet instant si
    /// le résultat en dépend (une animation ou une transition y contribue).
    fn styles(
        &self,
        doc: &Document,
        sheets: &[Stylesheet],
        viewport: Viewport,
    ) -> (StyleMap, Option<f64>) {
        let now = self.now();
        let mut state = self.lock();
        state.animator.advance_script_animations(now);
        let styles = oxyde_css::compute_styles_at(doc, sheets, viewport, &mut state.animator, now);
        let time = state.animator.is_effective().then_some(now);
        (styles, time)
    }

    /// Donne à `f` l'animateur et l'instant courant : le moteur de script
    /// pilote par là les animations qu'il crée (`Element.animate()`).
    pub(crate) fn script<R>(&self, f: impl FnOnce(&mut oxyde_css::Animator, f64) -> R) -> R {
        let now = self.now();
        let mut state = self.lock();
        f(&mut state.animator, now)
    }

    /// Vrai si une animation ou une transition demande une nouvelle image.
    fn needs_frames(&self) -> bool {
        self.lock().animator.needs_frames()
    }

    fn take_events(&self) -> Vec<oxyde_css::AnimationEvent> {
        self.lock().animator.take_events()
    }
}

/// Police déclarée par `@font-face`.
#[derive(Clone, Debug)]
struct FontSlot {
    /// Nom de famille que la page lui donne.
    family: String,
    /// Adresse du fichier.
    url: Url,
    /// Vrai une fois la ressource reçue (ou déclarée indisponible).
    resolved: bool,
    data: Option<std::sync::Arc<Vec<u8>>>,
}

/// Nombre maximal de polices chargées par page.
const MAX_FONTS: usize = 64;

/// Relève les polices déclarées par les feuilles `slots` ; celles de
/// `previous` gardent leurs données.
fn collect_fonts(slots: &[SheetSlot], base: &Url, previous: Vec<FontSlot>) -> Vec<FontSlot> {
    let mut fonts = previous;
    for slot in slots {
        let (sheet, sheet_base) = match slot {
            SheetSlot::Inline(sheet) => (sheet, base),
            SheetSlot::External {
                url,
                sheet: Some(sheet),
            } => (sheet, url),
            SheetSlot::External { sheet: None, .. } => continue,
        };
        for face in &sheet.font_faces {
            // La première adresse lisible : les formats compressés sont écartés.
            let source = face
                .sources
                .iter()
                .filter_map(|source| sheet_base.join(source).ok())
                .find(|url| {
                    let path = url.path().to_ascii_lowercase();
                    !path.ends_with(".woff") && !path.ends_with(".woff2")
                });
            let Some(url) = source else { continue };
            let known = fonts
                .iter()
                .any(|f| f.family == face.family && f.url == url);
            if !known && fonts.len() < MAX_FONTS {
                fonts.push(FontSlot {
                    family: face.family.clone(),
                    url,
                    resolved: false,
                    data: None,
                });
            }
        }
    }
    fonts
}

/// Enregistre dans `text` les polices reçues parmi `fonts`.
fn register_fonts(fonts: &[FontSlot], text: &mut TextContext) {
    for font in fonts {
        if let Some(data) = &font.data {
            text.register_web_font(&font.family, font.url.as_str(), data);
        }
    }
}

/// Feuilles de style utilisables parmi `slots`.
fn usable_sheets(slots: &[SheetSlot]) -> Vec<Stylesheet> {
    slots
        .iter()
        .filter_map(|s| match s {
            SheetSlot::Inline(sheet)
            | SheetSlot::External {
                sheet: Some(sheet), ..
            } => Some(sheet.clone()),
            SheetSlot::External { sheet: None, .. } => None,
        })
        .collect()
}

/// Calcule le style puis la mise en page de `doc` dans son état du moment,
/// pour une fenêtre de taille `viewport`.
/// Styles calculés du document `doc`, sans mise en page : ce que demande
/// une lecture de style qui ne dépend d'aucune boîte (`display`,
/// `visibility`, une couleur…). Rend aussi l'instant du calcul s'il en
/// dépend (animation ou transition en cours).
pub(crate) fn style_snapshot(
    doc: &Document,
    env: &LayoutEnv,
    viewport: (f32, f32),
) -> (StyleMap, Option<f64>) {
    let slots = collect_sheets(doc, &env.base, env.sheets.clone());
    let viewport = Viewport {
        width: viewport.0,
        height: viewport.1,
    };
    env.animations.styles(doc, &usable_sheets(&slots), viewport)
}

pub(crate) fn layout_snapshot(
    doc: &Document,
    env: &LayoutEnv,
    viewport: (f32, f32),
    text: &mut TextContext,
) -> (StyleMap, LayoutTree, Option<f64>) {
    register_fonts(&env.fonts, text);
    // Les `<style>` sont relus dans le document, qui a pu changer.
    let slots = collect_sheets(doc, &env.base, env.sheets.clone());
    let viewport = Viewport {
        width: viewport.0,
        height: viewport.1,
    };
    let (styles, time) = env.animations.styles(doc, &usable_sheets(&slots), viewport);
    let intrinsic = |node: NodeId| {
        env.sizes
            .get(&node)
            .copied()
            .or_else(|| {
                let el = doc.element(node).filter(|el| el.is_html("img"))?;
                let target = image_target(el, &env.base)?;
                env.known_sizes.get(target.as_str()).copied()
            })
            .or_else(|| svg::is_outer_root(doc, node).then(|| svg::intrinsic_size(doc, node)))
    };
    let tree = oxyde_layout::layout_document(doc, &styles, text, viewport, &intrinsic);
    (styles, tree, time)
}

/// État de chargement de l'image d'un élément `<img>`.
#[derive(Debug)]
struct ImageSlot {
    url: Url,
    /// Vrai une fois la ressource reçue (décodée ou non).
    resolved: bool,
    image: Option<Image>,
}

/// Script classique du document, dans l'ordre du source.
#[derive(Debug)]
enum ScriptSlot {
    Inline(String),
    /// `<script type="module">` en ligne.
    InlineModule(String),
    /// `<script src>` ; `source` vaut `None` tant que la ressource n'est pas
    /// reçue, et reste `None` si elle est indisponible.
    External {
        url: Url,
        resolved: bool,
        source: Option<String>,
        /// `<script type="module" src>`.
        module: bool,
    },
}

/// Cadre (`<iframe>`) du document, dans l'ordre du source.
#[derive(Debug)]
struct FrameSlot {
    /// L'élément `<iframe>`.
    element: NodeId,
    /// Adresse du document du cadre.
    url: Url,
    /// Vrai une fois la ressource fournie (ou déclarée indisponible).
    resolved: bool,
    /// Document analysé ; `None` tant qu'il n'est pas reçu ou s'il est
    /// indisponible.
    document: Option<Document>,
    /// Scripts classiques du document du cadre.
    scripts: Vec<ScriptSlot>,
    /// Vrai une fois le document remis au cadre (ou abandonné).
    loaded: bool,
    /// Type MIME du document s'il est en XML.
    xml: Option<&'static str>,
}

impl FrameSlot {
    /// Analyse `source` comme document du cadre, HTML ou XML selon
    /// `content_type`, et relève ses scripts.
    fn fill(&mut self, source: &str, content_type: Option<&str>) {
        let (document, xml) = parse_typed(source, content_type);
        self.xml = xml;
        self.scripts = frame_scripts(&document, &self.url);
        self.document = Some(document);
        self.resolved = true;
    }
}

/// Type MIME retenu pour un document XML servi comme `content_type`
/// (`application/xhtml+xml`, `image/svg+xml`, `text/xml`, tout type en
/// `+xml`…) ; `None` pour un document HTML.
#[must_use]
pub fn xml_content_type(content_type: &str) -> Option<&'static str> {
    let essence = content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    match essence.as_str() {
        "application/xhtml+xml" => Some("application/xhtml+xml"),
        "image/svg+xml" => Some("image/svg+xml"),
        "text/xml" => Some("text/xml"),
        "application/xml" => Some("application/xml"),
        other if other.ends_with("+xml") => Some("application/xml"),
        _ => None,
    }
}

/// Analyse `source` en HTML, ou en XML si `content_type` est un type XML.
/// Rend le document et, pour un document XML bien formé, son type MIME. Un
/// document XML mal formé est remplacé par une page HTML qui dit la faute.
fn parse_typed(source: &str, content_type: Option<&str>) -> (Document, Option<&'static str>) {
    let Some(xml) = content_type.and_then(xml_content_type) else {
        return (oxyde_html::parse_document(source), None);
    };
    match oxyde_html::parse_xml(source) {
        Ok(document) => (document, Some(xml)),
        Err(error) => {
            let message = error.to_string().replace('&', "&amp;").replace('<', "&lt;");
            let page = format!(
                "<!DOCTYPE html><html><head><title>Erreur d'analyse XML</title></head>\
                 <body><parsererror style=\"display:block;white-space:pre-wrap;\
                 border:2px solid #c00;padding:1em;margin:1em;background:#fdd;color:#000\">\
                 <h3>Ce document XML est mal formé.</h3><div>{message}</div>\
                 </parsererror></body></html>"
            );
            (oxyde_html::parse_document(&page), None)
        }
    }
}

/// Scripts classiques du document `doc` d'un cadre, dans l'ordre du source.
fn frame_scripts(doc: &Document, base: &Url) -> Vec<ScriptSlot> {
    let mut scripts = Vec::new();
    for node in doc.descendants(doc.root()) {
        let Some(el) = doc.element(node) else {
            continue;
        };
        if !el.is_html("script")
            || !is_classic_script(el.attr("type"))
            || !script_event_allows(el)
            || scripts.len() >= MAX_SCRIPTS
        {
            continue;
        }
        match el.attr("src").filter(|s| !s.trim().is_empty()) {
            Some(src) => {
                if let Ok(target) = base.join(src.trim()) {
                    scripts.push(ScriptSlot::External {
                        url: target,
                        resolved: false,
                        source: None,
                        module: false,
                    });
                }
            }
            None => {
                let source = doc.text_content(node);
                if !source.trim().is_empty() {
                    scripts.push(ScriptSlot::Inline(source));
                }
            }
        }
    }
    scripts
}

/// Textes des scripts classiques disponibles parmi `scripts`.
fn frame_sources(scripts: &[ScriptSlot]) -> Vec<String> {
    scripts
        .iter()
        .filter_map(|slot| match slot {
            ScriptSlot::Inline(source)
            | ScriptSlot::External {
                source: Some(source),
                module: false,
                ..
            } => Some(source.clone()),
            _ => None,
        })
        .collect()
}

/// Vrai si `a` et `b` ont la même origine (deux fichiers locaux comptent
/// pour une même origine). Tant que les cadres partagent le processus et la
/// mémoire de la page, seul un cadre de même origine est chargé : un
/// document d'une autre origine pourrait lire celui qui l'accueille.
pub(crate) fn same_origin(a: &Url, b: &Url) -> bool {
    (a.scheme() == "file" && b.scheme() == "file") || a.origin() == b.origin()
}

/// Nombre maximal de cadres chargés par page.
const MAX_FRAMES: usize = 50;

/// Surlignage des occurrences d'une recherche dans la page.
const FIND_HIGHLIGHT: Color = Color {
    r: 0xFF,
    g: 0xD5,
    b: 0x4F,
    a: 0x73,
};
/// Surlignage de l'occurrence courante.
const FIND_CURRENT: Color = Color {
    r: 0xFF,
    g: 0x8A,
    b: 0x00,
    a: 0x99,
};

/// Ce qu'un menu contextuel doit savoir de la page au point visé.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ContextInfo {
    /// Cible du lien sous le pointeur.
    pub link: Option<Url>,
    /// Du texte est sélectionné, dans la page ou dans le champ qui a le focus.
    pub selection: bool,
    /// Un champ de saisie a le focus : on peut y coller.
    pub editable: bool,
    /// Un script a annulé `contextmenu` : aucun menu ne doit s'ouvrir.
    pub prevented: bool,
}

/// Surlignage du texte sélectionné.
const SELECTION: Color = Color {
    r: 0x33,
    g: 0x90,
    b: 0xFF,
    a: 0x59,
};

/// Ce que désigne la sélection de texte : elle est recalculée à chaque
/// mise en page, les points restant ceux de la page.
#[derive(Clone, Copy, Debug, PartialEq)]
enum SelectionKind {
    /// Du point d'ancrage au point courant.
    Range {
        anchor: (f32, f32),
        focus: (f32, f32),
    },
    /// Le mot sous ce point.
    Word { x: f32, y: f32 },
    /// Toute la page.
    All,
}

/// Sélection de texte en cours.
#[derive(Debug)]
struct Selected {
    kind: SelectionKind,
    current: oxyde_layout::TextSelection,
}

/// Avancement d'une recherche dans la page.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FindStatus {
    /// Nombre d'occurrences trouvées.
    pub count: usize,
    /// Rang de l'occurrence courante, à partir de 1 ; nul s'il n'y en a aucune.
    pub current: usize,
}

/// Recherche en cours dans la page.
#[derive(Debug)]
struct Find {
    query: String,
    /// Rectangles de chaque occurrence, dans l'ordre du document.
    matches: Vec<Vec<oxyde_layout::Rect>>,
    /// Index de l'occurrence courante. INVARIANT : nul, ou inférieur à
    /// `matches.len()`.
    current: usize,
}

/// Une page chargée.
#[derive(Debug)]
pub struct Page {
    url: Url,
    /// Image de chaque canevas dessiné par un script, par élément.
    canvas_images: HashMap<NodeId, Image>,
    /// Rendu des éléments `<svg>` de la page, avec l'empreinte de ce dont il
    /// dépend : il n'est refait que si elle change.
    svg_images: HashMap<NodeId, (u64, Image)>,
    /// Type MIME du document s'il est en XML ; `None` pour du HTML.
    xml: Option<&'static str>,
    /// Adresse de base de résolution des liens (`<base href>` ou l'adresse de la page).
    base: Url,
    doc: Document,
    sheets: Vec<SheetSlot>,
    /// Polices déclarées par les feuilles de style.
    fonts: Vec<FontSlot>,
    images: HashMap<NodeId, ImageSlot>,
    scripts: Vec<ScriptSlot>,
    /// Élément `<script>` de chaque entrée de `scripts`, dans le même ordre.
    script_nodes: Vec<NodeId>,
    /// Modules importés par les scripts de type module, par adresse ; `None` :
    /// module indisponible.
    modules: HashMap<Url, Option<String>>,
    /// Carte d'import de la page.
    import_map: std::sync::Arc<ImportMap>,
    /// Cadres du document dont le contenu vient d'une adresse ou de `srcdoc`.
    frames: Vec<FrameSlot>,
    engine: Option<Box<ScriptEngine>>,
    /// Instructions accordées à chaque tâche de script ; `None` : la valeur
    /// par défaut du moteur.
    task_budget: Option<u64>,
    /// Texte de `document.cookie`, tel que l'embarqueur l'a fourni.
    cookies: String,
    styles: StyleMap,
    tree: Option<LayoutTree>,
    list: DisplayList,
    viewport: Viewport,
    /// Animations et transitions en cours, partagées avec le moteur de script.
    animations: std::sync::Arc<Animations>,
    /// Défilement de la fenêtre et des éléments, partagé de même.
    scrolling: std::sync::Arc<Scrolling>,
    /// Vrai si le style, les images, le DOM ou la taille ont changé depuis la dernière mise en page.
    dirty: bool,
    /// Recherche de texte en cours.
    find: Option<Find>,
    /// Sélection de texte en cours.
    selection: Option<Selected>,
    /// Texte que l'utilisateur vient de copier (Ctrl C), en attente du
    /// presse-papiers de l'embarqueur.
    copied: Option<String>,
    /// Élément sous le pointeur au dernier déplacement connu.
    hovered: Option<NodeId>,
    /// Zones qui ont défilé à l'image précédente : celles qui ne bougent
    /// plus à l'image suivante reçoivent `scrollend`.
    scroll_ending: Vec<u32>,
    /// Boutons de la souris enfoncés (voir [`MouseInput::buttons`]).
    buttons: u8,
    /// Pointeur au dernier bouton relâché : le clic qui suit en reprend
    /// le rang et les touches.
    released: MouseInput,
    /// Contacts tactiles en cours, par identifiant.
    touches: HashMap<u32, TouchState>,
    /// Nombre d'éléments de `list` qui viennent de la page ; les suivants
    /// sont le surlignage de la recherche.
    painted: usize,
}

fn attr_has_token(value: &str, token: &str) -> bool {
    value
        .split_ascii_whitespace()
        .any(|t| t.eq_ignore_ascii_case(token))
}

/// Vrai si l'attribut `type` d'un `<script>` désigne un script classique.
pub(crate) fn is_classic_script(kind: Option<&str>) -> bool {
    let kind = kind.unwrap_or("").trim().to_ascii_lowercase();
    matches!(
        kind.as_str(),
        "" | "text/javascript"
            | "application/javascript"
            | "text/ecmascript"
            | "application/ecmascript"
            | "application/x-javascript"
    )
}

/// Vrai si les attributs `for` et `event` d'un `<script>` le laissent
/// s'exécuter : absents, ou `for=window` avec `event=onload` (ancienne
/// syntaxe d'Internet Explorer, HTML Standard §4.12.1.1).
pub(crate) fn script_event_allows(el: &oxyde_dom::ElementData) -> bool {
    let (Some(target), Some(event)) = (el.attr("for"), el.attr("event")) else {
        return true;
    };
    let blank = |c: char| matches!(c, '\t' | '\n' | '\u{c}' | '\r' | ' ');
    let target = target.trim_matches(blank);
    let event = event.trim_matches(blank);
    target.eq_ignore_ascii_case("window")
        && (event.eq_ignore_ascii_case("onload") || event.eq_ignore_ascii_case("onload()"))
}

/// Vrai si l'attribut `type` d'un `<script>` désigne un module.
pub(crate) fn is_module_script(kind: Option<&str>) -> bool {
    kind.is_some_and(|k| k.trim().eq_ignore_ascii_case("module"))
}

/// Nombre maximal de modules importés par une page.
const MAX_MODULES: usize = 512;

/// Carte d'import (`<script type="importmap">`) : elle donne une adresse aux
/// spécificateurs « nus » (`"react"`, `"lodash/merge"`).
///
/// LIMITE : seule la table `imports` est lue ; les portées (`scopes`) et
/// l'intégrité sont ignorées.
#[derive(Debug, Default)]
pub(crate) struct ImportMap {
    /// (clé, adresse), dans l'ordre de la carte.
    imports: Vec<(String, Url)>,
}

impl ImportMap {
    /// Lit le texte JSON d'une carte ; les adresses relatives se résolvent
    /// par rapport à `base`. Une carte illisible est vide.
    fn parse(text: &str, base: &Url) -> Self {
        let imports = json_string_table(text, "imports")
            .into_iter()
            .filter_map(|(key, target)| Some((key, base.join(&target).ok()?)))
            .collect();
        Self { imports }
    }

    /// Adresse du module désigné par `specifier` dans le module `referrer` :
    /// la carte d'abord (clé exacte, sinon le plus long préfixe terminé par
    /// `/`), puis les règles ordinaires.
    pub(crate) fn resolve(&self, referrer: &Url, specifier: &str) -> Option<Url> {
        if let Some((_, target)) = self.imports.iter().find(|(key, _)| key == specifier) {
            return Some(target.clone());
        }
        let prefixed = self
            .imports
            .iter()
            .filter(|(key, _)| key.ends_with('/') && specifier.starts_with(key.as_str()))
            .max_by_key(|(key, _)| key.len());
        if let Some((key, target)) = prefixed {
            return target.join(specifier.get(key.len()..)?).ok();
        }
        resolve_module(referrer, specifier)
    }
}

/// Lit, dans l'objet JSON `text`, la table de chaînes rangée sous la clé
/// `wanted` : ses couples (clé, valeur) dont la valeur est une chaîne. Rend
/// une table vide si le texte n'a pas cette forme.
fn json_string_table(text: &str, wanted: &str) -> Vec<(String, String)> {
    /// Lecteur JSON minimal : il ne garde que les chaînes.
    struct Reader<'a> {
        chars: std::iter::Peekable<std::str::Chars<'a>>,
        /// Profondeur d'imbrication restante : borne la récursion.
        budget: usize,
    }
    impl Reader<'_> {
        fn space(&mut self) {
            while self.chars.next_if(|c| c.is_ascii_whitespace()).is_some() {}
        }
        fn eat(&mut self, wanted: char) -> Option<()> {
            self.space();
            self.chars.next_if_eq(&wanted).map(|_| ())
        }
        fn string(&mut self) -> Option<String> {
            self.eat('"')?;
            let mut out = String::new();
            loop {
                match self.chars.next()? {
                    '"' => return Some(out),
                    '\\' => match self.chars.next()? {
                        'n' => out.push('\n'),
                        't' => out.push('\t'),
                        'r' => out.push('\r'),
                        'b' => out.push('\u{8}'),
                        'f' => out.push('\u{c}'),
                        'u' => {
                            let code: String = self.chars.by_ref().take(4).collect();
                            out.push(char::from_u32(u32::from_str_radix(&code, 16).ok()?)?);
                        }
                        other => out.push(other),
                    },
                    other => out.push(other),
                }
            }
        }
        /// Saute une valeur ; rend la chaîne si c'en est une.
        fn value(&mut self) -> Option<Option<String>> {
            self.space();
            match *self.chars.peek()? {
                '"' => self.string().map(Some),
                '{' => self.object(None).map(|_| None),
                '[' => {
                    self.budget = self.budget.checked_sub(1)?;
                    self.chars.next();
                    if self.eat(']').is_none() {
                        loop {
                            self.value()?;
                            if self.eat(',').is_none() {
                                self.eat(']')?;
                                break;
                            }
                        }
                    }
                    self.budget += 1;
                    Some(None)
                }
                _ => {
                    // Nombre, `true`, `false`, `null`.
                    let mut any = false;
                    while self
                        .chars
                        .next_if(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '+' | '.'))
                        .is_some()
                    {
                        any = true;
                    }
                    any.then_some(None)
                }
            }
        }
        /// Lit un objet ; avec `wanted`, rend la table rangée sous cette clé,
        /// sinon les couples de l'objet dont la valeur est une chaîne.
        fn object(&mut self, wanted: Option<&str>) -> Option<Vec<(String, String)>> {
            self.budget = self.budget.checked_sub(1)?;
            self.eat('{')?;
            let mut found = Vec::new();
            if self.eat('}').is_none() {
                loop {
                    let key = self.string()?;
                    self.eat(':')?;
                    self.space();
                    match wanted {
                        Some(name) if key == name && self.chars.peek() == Some(&'{') => {
                            found = self.object(None)?;
                        }
                        Some(_) => {
                            self.value()?;
                        }
                        None => {
                            if let Some(text) = self.value()? {
                                found.push((key, text));
                            }
                        }
                    }
                    if self.eat(',').is_none() {
                        self.eat('}')?;
                        break;
                    }
                }
            }
            self.budget += 1;
            Some(found)
        }
    }
    let mut reader = Reader {
        chars: text.chars().peekable(),
        budget: 64,
    };
    reader.object(Some(wanted)).unwrap_or_default()
}

/// Adresse du module désigné par `specifier` dans le module `referrer`.
/// Comme dans un navigateur, un spécificateur « nu » (`"lodash"`) ne désigne
/// rien : seuls les chemins (`./`, `../`, `/`) et les adresses absolues sont
/// résolus, et seulement vers `http`, `https` ou le protocole de la page.
pub(crate) fn resolve_module(referrer: &Url, specifier: &str) -> Option<Url> {
    let relative = ["./", "../", "/"].iter().any(|p| specifier.starts_with(p));
    let url = if relative {
        referrer.join(specifier).ok()?
    } else {
        Url::parse(specifier).ok()?
    };
    (matches!(url.scheme(), "http" | "https") || url.scheme() == referrer.scheme()).then_some(url)
}

/// Vrai si `node` est dans un `<noscript>` : les scripts étant actifs, son
/// contenu est inerte.
fn in_noscript(doc: &Document, node: NodeId) -> bool {
    doc.ancestors(node)
        .any(|a| doc.element(a).is_some_and(|e| e.is_html("noscript")))
}

/// Adresse de l'image d'un `<img>` : son `src`, à défaut la première
/// candidate de son `srcset`.
fn image_target(el: &oxyde_dom::ElementData, base: &Url) -> Option<Url> {
    let src = el.attr("src").map(str::trim).filter(|s| !s.is_empty());
    let first_candidate = || {
        el.attr("srcset")?
            .split(',')
            .next()?
            .split_ascii_whitespace()
            .next()
    };
    base.join(src.or_else(first_candidate)?).ok()
}

/// Images du document. Celles de `previous` dont l'adresse n'a pas changé
/// sont conservées ; une image déjà reçue sert à tous les éléments qui la
/// montrent.
fn collect_images(
    doc: &Document,
    base: &Url,
    mut previous: HashMap<NodeId, ImageSlot>,
) -> HashMap<NodeId, ImageSlot> {
    let mut images = HashMap::new();
    for node in doc.descendants(doc.root()) {
        if images.len() >= MAX_IMAGES {
            break;
        }
        let Some(el) = doc.element(node) else {
            continue;
        };
        if !el.is_html("img") || in_noscript(doc, node) {
            continue;
        }
        let Some(target) = image_target(el, base) else {
            continue;
        };
        let slot = match previous.remove(&node) {
            Some(slot) if slot.url == target => slot,
            _ => {
                let known = previous
                    .values()
                    .chain(images.values())
                    .find(|slot: &&ImageSlot| slot.resolved && slot.url == target);
                ImageSlot {
                    resolved: known.is_some(),
                    image: known.and_then(|slot| slot.image.clone()),
                    url: target,
                }
            }
        };
        images.insert(node, slot);
    }
    images
}

/// Feuilles de style du document, dans l'ordre du source. Les feuilles
/// externes déjà chargées dans `previous` sont conservées.
fn collect_sheets(doc: &Document, base: &Url, previous: Vec<SheetSlot>) -> Vec<SheetSlot> {
    let mut loaded: Vec<(Url, Option<Stylesheet>)> = previous
        .into_iter()
        .filter_map(|slot| match slot {
            SheetSlot::External { url, sheet } => Some((url, sheet)),
            SheetSlot::Inline(_) => None,
        })
        .collect();
    let mut sheets = Vec::new();
    // Le document, puis chaque arbre fantôme : ses feuilles ne valent que
    // pour lui.
    //
    // LIMITE : un `<link rel=stylesheet>` dans un arbre fantôme est ignoré.
    let trees = std::iter::once((doc.root(), None)).chain(
        doc.shadow_root_list()
            .into_iter()
            .map(|root| (root, Some(root))),
    );
    for (node, scope) in
        trees.flat_map(|(root, scope)| doc.descendants(root).map(move |n| (n, scope)))
    {
        let Some(el) = doc.element(node) else {
            continue;
        };
        // Le contenu d'un `<noscript>` est inerte : ses feuilles aussi.
        if in_noscript(doc, node) {
            continue;
        }
        if oxyde_css::is_style_element(el) {
            sheets.push(SheetSlot::Inline(Stylesheet {
                scope,
                ..parse_stylesheet(&doc.text_content(node))
            }));
        } else if scope.is_none()
            && el.is_html("link")
            && el
                .attr("rel")
                .is_some_and(|r| attr_has_token(r, "stylesheet"))
            && !el
                .attr("rel")
                .is_some_and(|r| attr_has_token(r, "alternate"))
            && let Some(target) = el.attr("href").and_then(|h| base.join(h).ok())
        {
            let sheet = loaded
                .iter()
                .position(|(u, _)| *u == target)
                .and_then(|i| loaded.swap_remove(i).1);
            sheets.push(SheetSlot::External { url: target, sheet });
        }
    }
    sheets
}

impl Page {
    /// Analyse `html` comme le document situé à `url`.
    #[must_use]
    pub fn from_html(url: Url, html: &str) -> Self {
        Self::from_source(url, html, None)
    }

    /// Analyse `source` comme le document situé à `url`, servi avec le type
    /// MIME `content_type` : en XML pour un type XML (voir
    /// [`xml_content_type`]), en HTML sinon. Un document XML mal formé
    /// laisse place à une page qui dit la faute.
    #[must_use]
    pub fn from_source(url: Url, source: &str, content_type: Option<&str>) -> Self {
        let (mut doc, xml) = parse_typed(source, content_type);
        // `<template shadowrootmode>` : arbres fantômes posés par l'analyseur
        // HTML.
        if xml.is_none() {
            doc.adopt_declarative_shadow_roots();
        }
        let base = doc
            .find_html_element(doc.root(), "base")
            .and_then(|n| doc.element(n)?.attr("href").and_then(|h| url.join(h).ok()))
            .unwrap_or_else(|| url.clone());
        let sheets = collect_sheets(&doc, &base, Vec::new());
        let images = collect_images(&doc, &base, HashMap::new());
        let mut scripts = Vec::new();
        let mut script_nodes = Vec::new();
        let mut import_map = ImportMap::default();
        let mut frames: Vec<FrameSlot> = Vec::new();
        for node in doc.descendants(doc.root()) {
            let Some(el) = doc.element(node) else {
                continue;
            };
            // Les scripts sont actifs : le contenu d'un `<noscript>` est
            // inerte, ses images, cadres et scripts ne sont pas chargés.
            if in_noscript(&doc, node) {
                continue;
            }
            if el.is_html("iframe") && frames.len() < MAX_FRAMES {
                let mut slot = FrameSlot {
                    element: node,
                    url: base.clone(),
                    resolved: false,
                    document: None,
                    scripts: Vec::new(),
                    loaded: false,
                    xml: None,
                };
                if let Some(srcdoc) = el.attr("srcdoc") {
                    slot.fill(srcdoc, None);
                    frames.push(slot);
                } else if let Some(target) = el
                    .attr("src")
                    .filter(|s| !s.trim().is_empty())
                    .and_then(|s| base.join(s.trim()).ok())
                    .filter(|u| matches!(u.scheme(), "http" | "https" | "file"))
                    .filter(|u| same_origin(u, &url))
                {
                    slot.url = target;
                    frames.push(slot);
                }
            } else if el.is_html("script")
                && el
                    .attr("type")
                    .is_some_and(|kind| kind.trim().eq_ignore_ascii_case("importmap"))
            {
                // La première carte d'import de la page fait foi.
                if import_map.imports.is_empty() {
                    import_map = ImportMap::parse(&doc.text_content(node), &base);
                }
            } else if el.is_html("script")
                // `for` et `event` ne filtrent que les scripts classiques.
                && ((is_classic_script(el.attr("type")) && script_event_allows(el))
                    || is_module_script(el.attr("type")))
                && scripts.len() < MAX_SCRIPTS
            {
                let module = is_module_script(el.attr("type"));
                // `nomodule` : repli pour les navigateurs sans modules.
                if !module && el.has_attr("nomodule") {
                    continue;
                }
                let before = scripts.len();
                match el.attr("src").filter(|s| !s.trim().is_empty()) {
                    Some(src) => {
                        if let Ok(target) = base.join(src.trim()) {
                            scripts.push(ScriptSlot::External {
                                url: target,
                                resolved: false,
                                source: None,
                                module,
                            });
                        }
                    }
                    None => {
                        let source = doc.text_content(node);
                        if !source.trim().is_empty() {
                            scripts.push(if module {
                                ScriptSlot::InlineModule(source)
                            } else {
                                ScriptSlot::Inline(source)
                            });
                        }
                    }
                }
                if scripts.len() > before {
                    script_nodes.push(node);
                }
            }
        }
        let fonts = collect_fonts(&sheets, &base, Vec::new());
        Self {
            url,
            base,
            doc,
            fonts,
            sheets,
            images,
            scripts,
            script_nodes,
            xml,
            canvas_images: HashMap::new(),
            svg_images: HashMap::new(),
            import_map: std::sync::Arc::new(import_map),
            modules: HashMap::new(),
            frames,
            engine: None,
            task_budget: None,
            cookies: String::new(),
            styles: StyleMap::default(),
            tree: None,
            list: DisplayList::default(),
            viewport: Viewport {
                width: 0.0,
                height: 0.0,
            },
            animations: std::sync::Arc::new(Animations::new()),
            scrolling: std::sync::Arc::default(),
            dirty: true,
            find: None,
            selection: None,
            copied: None,
            hovered: None,
            scroll_ending: Vec::new(),
            buttons: 0,
            released: MouseInput::default(),
            touches: HashMap::new(),
            painted: 0,
        }
    }

    /// Adresse de la page.
    #[must_use]
    pub fn url(&self) -> &Url {
        &self.url
    }

    /// Document analysé.
    #[must_use]
    pub fn document(&self) -> &Document {
        &self.doc
    }

    /// Adresses des feuilles de style externes pas encore chargées.
    #[must_use]
    pub fn pending_stylesheets(&self) -> Vec<Url> {
        self.sheets
            .iter()
            .filter_map(|s| match s {
                SheetSlot::External { url, sheet: None } => Some(url.clone()),
                _ => None,
            })
            .collect()
    }

    /// Fournit le contenu d'une feuille de style externe demandée par
    /// [`Page::pending_stylesheets`]. Renvoie `false` si `url` n'était pas attendue.
    pub fn set_stylesheet(&mut self, url: &Url, css: &str) -> bool {
        let mut found = false;
        for slot in &mut self.sheets {
            if let SheetSlot::External { url: u, sheet } = slot
                && u == url
                && sheet.is_none()
            {
                *sheet = Some(parse_stylesheet(css));
                found = true;
            }
        }
        self.dirty |= found;
        if found {
            let previous = std::mem::take(&mut self.fonts);
            self.fonts = collect_fonts(&self.sheets, &self.base, previous);
            self.sync_layout_env();
        }
        found
    }

    /// Adresses des images pas encore chargées, sans doublon, dans l'ordre du document.
    #[must_use]
    pub fn pending_images(&self) -> Vec<Url> {
        let mut nodes: Vec<(&NodeId, &ImageSlot)> = self
            .images
            .iter()
            .filter(|(_, slot)| !slot.resolved)
            .collect();
        nodes.sort_by_key(|(node, _)| **node);
        let mut urls: Vec<Url> = Vec::new();
        for (_, slot) in nodes {
            if !urls.contains(&slot.url) {
                urls.push(slot.url.clone());
            }
        }
        urls
    }

    /// Fournit le résultat du chargement d'une image demandée par
    /// [`Page::pending_images`] ; `None` si elle est indisponible ou illisible
    /// (le texte alternatif reste alors affiché). Renvoie `false` si `url`
    /// n'était pas attendue.
    pub fn set_image(&mut self, url: &Url, image: Option<Image>) -> bool {
        let mut found = false;
        for slot in self
            .images
            .values_mut()
            .filter(|s| !s.resolved && &s.url == url)
        {
            slot.resolved = true;
            slot.image.clone_from(&image);
            found = true;
        }
        self.dirty |= found && image.is_some();
        found
    }

    /// Adresses des fichiers de police (`@font-face`) qu'il reste à fournir
    /// par [`Page::set_font`]. À demander après les feuilles de style, qui
    /// les déclarent.
    #[must_use]
    pub fn pending_fonts(&self) -> Vec<Url> {
        let mut urls: Vec<Url> = Vec::new();
        for font in self.fonts.iter().filter(|font| !font.resolved) {
            if !urls.contains(&font.url) {
                urls.push(font.url.clone());
            }
        }
        urls
    }

    /// Fournit le fichier de police demandé par [`Page::pending_fonts`] ;
    /// `None` s'il est indisponible. Renvoie `false` si `url` n'était pas
    /// attendue.
    pub fn set_font(&mut self, url: &Url, data: Option<Vec<u8>>) -> bool {
        let data = data.map(std::sync::Arc::new);
        let mut found = false;
        for font in &mut self.fonts {
            if font.url == *url && !font.resolved {
                font.resolved = true;
                font.data.clone_from(&data);
                found = true;
            }
        }
        if found {
            self.dirty = true;
            self.sync_layout_env();
        }
        found
    }

    /// Adresses des documents de cadres (`<iframe src>`) qu'il reste à
    /// fournir par [`Page::set_frame`].
    ///
    /// Les cadres créés ou redirigés par les scripts s'y ajoutent au fil de
    /// l'exécution : à rappeler après chaque tâche, puis
    /// [`Page::load_ready_frames`] une fois les ressources fournies.
    pub fn pending_frames(&mut self) -> Vec<Url> {
        let requests = self
            .engine
            .as_mut()
            .map(|engine| engine.take_frame_requests())
            .unwrap_or_default();
        for (element, url) in requests {
            if self.frames.len() < MAX_FRAMES * 4 {
                self.frames.push(FrameSlot {
                    element,
                    url,
                    resolved: false,
                    document: None,
                    scripts: Vec::new(),
                    loaded: false,
                    xml: None,
                });
            }
        }
        self.frames
            .iter()
            .filter(|frame| !frame.resolved)
            .map(|frame| frame.url.clone())
            .collect()
    }

    /// Fournit le HTML d'un document de cadre demandé par
    /// [`Page::pending_frames`] ; `None` s'il est indisponible (le cadre
    /// reste vide). Ses scripts externes s'ajoutent à
    /// [`Page::pending_scripts`]. Renvoie `false` si `url` n'était pas
    /// attendue.
    pub fn set_frame(&mut self, url: &Url, html: Option<&str>) -> bool {
        self.set_frame_typed(url, html, None)
    }

    /// Comme [`Page::set_frame`], pour un document servi avec le type MIME
    /// `content_type` : un type XML est analysé en XML.
    pub fn set_frame_typed(
        &mut self,
        url: &Url,
        source: Option<&str>,
        content_type: Option<&str>,
    ) -> bool {
        let html = source;
        let mut found = false;
        for frame in &mut self.frames {
            if frame.url == *url && !frame.resolved {
                match html {
                    Some(html) => frame.fill(html, content_type),
                    None => {
                        frame.resolved = true;
                        frame.loaded = true;
                    }
                }
                found = true;
            }
        }
        found
    }

    /// Remet aux cadres les documents reçus depuis [`Page::run_scripts`] dont
    /// tous les scripts sont fournis, et exécute ces scripts. Renvoie `true`
    /// si au moins un cadre a été chargé.
    pub fn load_ready_frames(&mut self) -> bool {
        let Some(engine) = self.engine.as_mut() else {
            return false;
        };
        let mut any = false;
        for frame in &mut self.frames {
            let ready = !frame.loaded
                && frame.scripts.iter().all(|slot| {
                    !matches!(
                        slot,
                        ScriptSlot::External {
                            resolved: false,
                            ..
                        }
                    )
                });
            let (true, Some(document)) = (ready, &frame.document) else {
                continue;
            };
            let sources = frame_sources(&frame.scripts);
            engine.load_frame(
                &mut self.doc,
                frame.element,
                &frame.url,
                document,
                &sources,
                frame.xml,
            );
            frame.loaded = true;
            any = true;
        }
        if any && engine.take_dirty() {
            self.dom_changed();
        }
        any
    }

    /// Adresses des scripts externes pas encore chargés, dans l'ordre du document.
    #[must_use]
    pub fn pending_scripts(&self) -> Vec<Url> {
        self.scripts
            .iter()
            .chain(self.frames.iter().flat_map(|frame| frame.scripts.iter()))
            .filter_map(|s| match s {
                ScriptSlot::External {
                    url,
                    resolved: false,
                    ..
                } => Some(url.clone()),
                _ => None,
            })
            .collect()
    }

    /// Fournit le source d'un script externe demandé par
    /// [`Page::pending_scripts`] ; `None` s'il est indisponible (il est alors
    /// sauté). Renvoie `false` si `url` n'était pas attendue.
    pub fn set_script(&mut self, url: &Url, source: Option<String>) -> bool {
        let mut found = false;
        let frame_scripts = self
            .frames
            .iter_mut()
            .flat_map(|frame| frame.scripts.iter_mut());
        for slot in self.scripts.iter_mut().chain(frame_scripts) {
            if let ScriptSlot::External {
                url: u,
                resolved,
                source: slot_source,
                ..
            } = slot
                && u == url
                && !*resolved
            {
                *resolved = true;
                slot_source.clone_from(&source);
                found = true;
            }
        }
        found
    }

    /// Scripts de type module du document dont le texte est connu :
    /// (adresse servant d'identifiant et de base de résolution, texte).
    fn module_entries(&self) -> Vec<(Url, &str)> {
        let mut entries = Vec::new();
        for (index, slot) in self.scripts.iter().enumerate() {
            match slot {
                ScriptSlot::InlineModule(source) => {
                    // Un module en ligne se résout par rapport à la page ; le
                    // fragment distingue plusieurs modules en ligne.
                    let mut id = self.base.clone();
                    id.set_fragment(Some(&format!("module-{index}")));
                    entries.push((id, source.as_str()));
                }
                ScriptSlot::External {
                    url,
                    source: Some(source),
                    module: true,
                    ..
                } => entries.push((url.clone(), source.as_str())),
                _ => {}
            }
        }
        entries
    }

    /// Adresses des modules importés (`import … from`) qu'il reste à fournir
    /// par [`Page::set_module`]. À rappeler après chaque fourniture : un module
    /// reçu peut en importer d'autres. Vide quand le graphe est complet.
    #[must_use]
    pub fn pending_modules(&self) -> Vec<Url> {
        let mut pending: Vec<Url> = Vec::new();
        let known = self.module_entries().into_iter().chain(
            self.modules
                .iter()
                .filter_map(|(url, source)| Some((url.clone(), source.as_deref()?))),
        );
        for (referrer, source) in known {
            // Un module invalide n'importe rien : son erreur sera signalée à l'exécution.
            for specifier in oxyde_js::module_requests(source).unwrap_or_default() {
                if let Some(url) = self.import_map.resolve(&referrer, &specifier)
                    && !self.modules.contains_key(&url)
                    && !pending.contains(&url)
                    && self.modules.len() + pending.len() < MAX_MODULES
                {
                    pending.push(url);
                }
            }
        }
        pending
    }

    /// Fournit le texte d'un module demandé par [`Page::pending_modules`] ;
    /// `None` s'il est indisponible (les scripts qui l'importent échoueront).
    ///
    /// LIMITE : un module d'une autre origine est accepté sans contrôle CORS.
    pub fn set_module(&mut self, url: &Url, source: Option<String>) {
        if self.modules.len() < MAX_MODULES || self.modules.contains_key(url) {
            self.modules.insert(url.clone(), source);
        }
    }

    /// Exécute les scripts du document — les scripts classiques dans l'ordre
    /// du source, puis les modules — et distribue `DOMContentLoaded` et
    /// `load`. À appeler une fois, après avoir fourni les scripts externes et
    /// les modules importés. Les exceptions vont dans la console.
    pub fn run_scripts(&mut self) {
        // Sans script, le moteur n'est créé que pour porter les documents des
        // cadres, qui se dessinent.
        if self.scripts.is_empty() && self.frames.iter().all(|frame| frame.document.is_none()) {
            self.autofocus();
            return;
        }
        let entries: Vec<(String, String)> = self
            .module_entries()
            .into_iter()
            .map(|(id, source)| (id.to_string(), source.to_owned()))
            .collect();
        let modules = std::mem::take(&mut self.modules);
        let scripts = std::mem::take(&mut self.scripts);
        let script_nodes = std::mem::take(&mut self.script_nodes);
        let mut engine = self.new_engine();
        engine.set_import_map(std::sync::Arc::clone(&self.import_map));
        // Les cadres relevés à l'analyse sont chargés ici, pas à la demande.
        let noted: Vec<(NodeId, String)> = self
            .frames
            .iter()
            .filter_map(|frame| {
                let element = self.doc.element(frame.element)?;
                Some((
                    frame.element,
                    script::frame_fingerprint(element.attr("srcdoc"), element.attr("src")),
                ))
            })
            .collect();
        engine.note_static_frames(&noted);
        if let Some(budget) = self.task_budget {
            engine.set_task_budget(budget);
        }
        engine.set_cookies(&self.cookies);
        engine.set_viewport(self.viewport.width, self.viewport.height);
        engine.set_layout_env(self.layout_env());
        for (index, slot) in scripts.iter().enumerate() {
            let source = match slot {
                ScriptSlot::Inline(source)
                | ScriptSlot::External {
                    source: Some(source),
                    module: false,
                    ..
                } => source,
                _ => continue,
            };
            engine.eval_element(&mut self.doc, script_nodes.get(index).copied(), source);
        }
        // Les modules sont différés : ils s'exécutent après les scripts classiques.
        engine.set_modules(
            modules
                .into_iter()
                .filter_map(|(url, source)| Some((url.to_string(), source?)))
                .collect(),
        );
        for (id, source) in &entries {
            engine.eval_module(&mut self.doc, source, id);
        }
        let root = self.doc.root();
        engine.dispatch(&mut self.doc, Some(root), "DOMContentLoaded");
        // Les cadres se chargent avant l'événement `load` de la page.
        for frame in &mut self.frames {
            frame.loaded = true;
            let Some(document) = &frame.document else {
                continue;
            };
            let sources = frame_sources(&frame.scripts);
            engine.load_frame(
                &mut self.doc,
                frame.element,
                &frame.url,
                document,
                &sources,
                frame.xml,
            );
        }
        engine.dispatch(&mut self.doc, None, "load");
        engine.take_dirty();
        self.engine = Some(engine);
        self.dom_changed();
        self.autofocus();
    }

    /// La page est chargée : le premier élément `autofocus` prend le focus,
    /// si rien ne l'a déjà.
    fn autofocus(&mut self) {
        let wanted = self.doc.focus().is_none()
            && self.doc.descendants(self.doc.root()).any(|node| {
                self.doc
                    .element(node)
                    .is_some_and(|e| e.has_attr("autofocus"))
            });
        if !wanted {
            return;
        }
        self.ensure_engine();
        if let Some(engine) = self.engine.as_mut() {
            engine.autofocus(&mut self.doc);
            if engine.take_dirty() {
                self.dom_changed();
            }
        }
    }

    /// Crée le moteur de script s'il n'existe pas : les contrôles de
    /// formulaire (focus, saisie, cases, envoi) en dépendent, même dans une
    /// page qui n'a aucun script.
    fn ensure_engine(&mut self) {
        if self.engine.is_none() {
            self.engine = Some(self.new_engine());
        }
    }

    /// Moteur de script neuf pour cette page, informé de la nature de son
    /// document.
    fn new_engine(&self) -> Box<ScriptEngine> {
        let mut engine = Box::new(ScriptEngine::new(&self.url));
        if let Some(xml) = self.xml {
            engine.set_xml_document(self.doc.root(), xml);
        }
        engine
    }

    /// À appeler quand un script a pu modifier le DOM : les feuilles de style
    /// en ligne sont relues et la mise en page invalidée.
    fn dom_changed(&mut self) {
        let previous = std::mem::take(&mut self.sheets);
        self.sheets = collect_sheets(&self.doc, &self.base, previous);
        // Les images qu'un script a ajoutées, ou dont il a changé l'adresse.
        let previous = std::mem::take(&mut self.images);
        self.images = collect_images(&self.doc, &self.base, previous);
        self.dirty = true;
    }

    /// Fixe le nombre d'instructions qu'une tâche de script (script, événement,
    /// minuteur) peut exécuter avant d'être interrompue. À appeler avant
    /// [`Page::run_scripts`] ; un lanceur de tests y met une valeur large.
    pub fn set_task_budget(&mut self, budget: u64) {
        self.task_budget = Some(budget);
        if let Some(engine) = self.engine.as_mut() {
            engine.set_task_budget(budget);
        }
    }

    /// Fixe la taille de la fenêtre que voient les scripts avant la première
    /// mise en page ([`Page::update`] la fixe ensuite à chaque appel).
    pub fn set_viewport(&mut self, viewport: Viewport) {
        if viewport != self.viewport {
            self.viewport = viewport;
            self.dirty = true;
        }
        if let Some(engine) = self.engine.as_mut() {
            engine.set_viewport(viewport.width, viewport.height);
        }
    }

    /// Feuilles de style reçues et tailles d'images, pour la mise en page à
    /// la demande des scripts.
    fn layout_env(&self) -> LayoutEnv {
        #[expect(clippy::cast_precision_loss, reason = "côtés d'image bornés à 16 384")]
        let sizes = self
            .images
            .iter()
            .filter_map(|(node, slot)| {
                let image = slot.image.as_ref()?;
                Some((*node, (image.width() as f32, image.height() as f32)))
            })
            .collect();
        #[expect(clippy::cast_precision_loss, reason = "côtés d'image bornés à 16 384")]
        let known_sizes = self
            .images
            .values()
            .filter_map(|slot| {
                let image = slot.image.as_ref()?;
                Some((
                    slot.url.as_str().to_owned(),
                    (image.width() as f32, image.height() as f32),
                ))
            })
            .collect();
        LayoutEnv {
            sheets: self.sheets.clone(),
            base: self.base.clone(),
            sizes,
            known_sizes,
            pictures: self
                .images
                .iter()
                .filter_map(|(node, slot)| Some((*node, slot.image.clone()?)))
                .collect(),
            fonts: self.fonts.clone(),
            animations: std::sync::Arc::clone(&self.animations),
            scrolling: std::sync::Arc::clone(&self.scrolling),
        }
    }

    /// Remet au moteur de script de quoi mettre la page en page.
    fn sync_layout_env(&mut self) {
        let env = self.layout_env();
        if let Some(engine) = self.engine.as_mut() {
            engine.set_layout_env(env);
        }
    }

    /// Fixe le texte que `document.cookie` rend aux scripts : les cookies du
    /// pot de l'embarqueur qu'un script a le droit de lire, sous la forme
    /// `nom=valeur; autre=valeur`. À rappeler quand le pot change.
    pub fn set_cookies(&mut self, cookies: &str) {
        cookies.clone_into(&mut self.cookies);
        if let Some(engine) = self.engine.as_mut() {
            engine.set_cookies(cookies);
        }
    }

    /// Textes affectés à `document.cookie` par les scripts depuis le dernier
    /// appel, dans l'ordre : l'embarqueur les soumet à son pot, qui décide de
    /// les accepter, puis rend la vue à jour par [`Page::set_cookies`].
    pub fn take_cookie_writes(&mut self) -> Vec<String> {
        self.engine
            .as_mut()
            .map(|engine| engine.take_cookie_writes())
            .unwrap_or_default()
    }

    /// Messages écrits dans la console par les scripts depuis le dernier appel.
    pub fn take_console(&mut self) -> Vec<ConsoleMessage> {
        self.engine
            .as_mut()
            .map(|e| e.take_console())
            .unwrap_or_default()
    }

    /// Touche enfoncée (`down`) ou relâchée : les scripts reçoivent
    /// `keydown` (puis `keypress` pour un caractère) ou `keyup`. Renvoie
    /// (action par défaut annulée, page à redessiner).
    pub fn key_event(&mut self, down: bool, input: &KeyInput<'_>) -> (bool, bool) {
        // Sans script, une touche ne compte que pour le focus et la saisie.
        if self.doc.focus().is_some() || input.key == "Tab" {
            self.ensure_engine();
        }
        let (prevented, mut redraw) = match self.engine.as_mut() {
            Some(engine) => {
                let prevented = engine.key_event(&mut self.doc, down, input);
                // Texte copié ou coupé dans un champ de saisie.
                if let Some(text) = engine.take_copied() {
                    self.copied = Some(text);
                }
                let redraw = engine.take_dirty();
                if redraw {
                    self.dom_changed();
                }
                (prevented, redraw)
            }
            None => (false, false),
        };
        // Raccourcis de la sélection, si les scripts n'ont rien annulé.
        if down && !prevented && self.selection_shortcut(input) {
            redraw = true;
        }
        (prevented, redraw)
    }

    /// Ctrl A : sélectionne tout le texte de la page ; Ctrl C : remet le
    /// texte sélectionné à l'embarqueur (voir [`Page::take_copied`]).
    /// Renvoie `true` si l'affichage a changé.
    ///
    /// Dans un champ de saisie, ces touches sont celles du champ.
    fn selection_shortcut(&mut self, input: &KeyInput<'_>) -> bool {
        if !input.ctrl || input.alt || input.meta || input.shift {
            return false;
        }
        if self.focused_field().is_some() {
            return false;
        }
        if input.key.eq_ignore_ascii_case("a") {
            return self.select_all();
        }
        if input.key.eq_ignore_ascii_case("c") && !self.selected_text().is_empty() {
            self.copied = Some(self.selected_text().to_owned());
        }
        false
    }

    /// Champ de saisie (d'une ligne, ou zone de texte) qui a le focus.
    fn focused_field(&self) -> Option<NodeId> {
        self.doc.focus().filter(|node| {
            script::is_text_input(&self.doc, *node)
                || self
                    .doc
                    .element(*node)
                    .is_some_and(|e| e.is_html("textarea"))
        })
    }

    /// Texte que « Copier » prendrait : la sélection du champ qui a le
    /// focus (jamais celle d'un mot de passe), sinon celle de la page.
    fn copy_text(&self) -> Option<String> {
        let Some(node) = self.focused_field() else {
            return Some(self.selected_text())
                .filter(|text| !text.is_empty())
                .map(str::to_owned);
        };
        let element = self.doc.element(node)?;
        let secret = element
            .attr("type")
            .is_some_and(|kind| kind.eq_ignore_ascii_case("password"));
        if secret || !script::is_text_input(&self.doc, node) {
            return None;
        }
        let (a, b) = (self.doc.selection_anchor(), self.doc.caret());
        element
            .attr("value")?
            .get(a.min(b)..a.max(b))
            .filter(|text| !text.is_empty())
            .map(str::to_owned)
    }

    /// Ce qu'un menu contextuel ouvert au point (`x`, `y`) de la page doit
    /// proposer.
    #[must_use]
    pub fn context_at(&self, x: f32, y: f32) -> ContextInfo {
        ContextInfo {
            link: self.link_at(x, y),
            selection: self.copy_text().is_some(),
            editable: self.focused_field().is_some(),
            prevented: false,
        }
    }

    /// « Copier » : remet le texte sélectionné à l'embarqueur (voir
    /// [`Page::take_copied`]).
    pub fn copy(&mut self) {
        if let Some(text) = self.copy_text() {
            self.copied = Some(text);
        }
    }

    /// « Tout sélectionner » : la valeur du champ d'une ligne qui a le
    /// focus, sinon le texte de la page. Renvoie `true` si l'affichage a
    /// changé.
    pub fn select_everything(&mut self) -> bool {
        let Some(node) = self.focused_field() else {
            return self.select_all();
        };
        if !script::is_text_input(&self.doc, node) {
            return false;
        }
        let end = self
            .doc
            .element(node)
            .and_then(|e| e.attr("value"))
            .map_or(0, str::len);
        self.doc.set_field_selection(Some(node), 0, end);
        self.dom_changed();
        true
    }

    /// Colle `text` (Ctrl V) dans le champ de saisie qui a le focus, à la
    /// place de sa sélection. Renvoie `true` si la page doit être
    /// redessinée ; sans champ qui ait le focus, rien ne se passe.
    pub fn paste(&mut self, text: &str) -> bool {
        if self.doc.focus().is_none() {
            return false;
        }
        self.ensure_engine();
        let Some(engine) = self.engine.as_mut() else {
            return false;
        };
        engine.paste(&mut self.doc, text);
        let redraw = engine.take_dirty();
        if redraw {
            self.dom_changed();
        }
        redraw
    }

    /// Texte que l'utilisateur a copié (Ctrl C) depuis le dernier appel :
    /// l'embarqueur le place dans le presse-papiers.
    pub fn take_copied(&mut self) -> Option<String> {
        self.copied.take()
    }

    /// Navigations demandées par les scripts depuis le dernier appel. Un
    /// changement d'adresse sans changement de document
    /// ([`Navigation::SameDocument`]) est déjà appliqué à la page.
    pub fn take_navigations(&mut self) -> Vec<Navigation> {
        let navigations = self
            .engine
            .as_mut()
            .map(|e| e.take_navigations())
            .unwrap_or_default();
        for navigation in &navigations {
            if let Navigation::SameDocument { url, .. } = navigation {
                self.url.clone_from(url);
            }
        }
        navigations
    }

    /// L'embarqueur a parcouru l'historique de `delta` entrées jusqu'à une
    /// entrée du même document, d'adresse `url` : la page prend cette adresse
    /// et prévient ses scripts (`popstate`). Renvoie `true` si la page doit
    /// être redessinée.
    pub fn traverse(&mut self, url: &Url, delta: i32) -> bool {
        self.url.clone_from(url);
        let Some(engine) = self.engine.as_mut() else {
            return false;
        };
        engine.traverse(&mut self.doc, url, delta);
        if engine.take_dirty() {
            self.dom_changed();
            return true;
        }
        false
    }

    /// Requêtes `fetch` émises par les scripts depuis le dernier appel.
    /// L'embarqueur les exécute puis rend chaque réponse par
    /// [`Page::complete_fetch`].
    pub fn take_fetches(&mut self) -> Vec<FetchRequest> {
        self.engine
            .as_mut()
            .map(|e| e.take_fetches())
            .unwrap_or_default()
    }

    /// Ordres que les scripts ont donnés à leurs connexions `WebSocket`
    /// depuis le dernier appel, dans l'ordre. L'embarqueur ouvre les
    /// connexions et rend ce qui leur arrive par [`Page::socket_event`].
    pub fn take_socket_commands(&mut self) -> Vec<SocketCommand> {
        self.engine
            .as_mut()
            .map(|e| e.take_socket_commands())
            .unwrap_or_default()
    }

    /// Vrai si la connexion `WebSocket` `id` est en cours dans cette page.
    /// Sinon (page quittée entre-temps, connexion finie), l'embarqueur n'a
    /// plus à la garder ouverte.
    pub fn has_socket(&mut self, id: u64) -> bool {
        self.engine.as_mut().is_some_and(|e| e.has_socket(id))
    }

    /// Remet aux scripts ce qui arrive à la connexion `WebSocket` `id`.
    /// Renvoie `true` si la page doit être redessinée.
    pub fn socket_event(&mut self, id: u64, event: SocketEvent) -> bool {
        let Some(engine) = self.engine.as_mut() else {
            return false;
        };
        engine.socket_event(&mut self.doc, id, event);
        if engine.take_dirty() {
            self.dom_changed();
            return true;
        }
        false
    }

    /// Livre aux scripts la réponse (ou l'échec, décrit par un message) de la
    /// requête `id`. Une réponse d'une autre origine que la page n'est remise
    /// au script que si son serveur l'autorise (`Access-Control-Allow-Origin`).
    /// Renvoie `true` si la page doit être redessinée.
    pub fn complete_fetch(&mut self, id: u64, result: Result<FetchResponse, String>) -> bool {
        let Some(engine) = self.engine.as_mut() else {
            return false;
        };
        engine.complete_fetch(&mut self.doc, id, result);
        if engine.take_dirty() {
            self.dom_changed();
            return true;
        }
        false
    }

    /// Exécute les minuteurs arrivés à échéance. Renvoie `true` si la page
    /// doit être redessinée.
    pub fn tick(&mut self, now: Instant) -> bool {
        self.animations.advance(now);
        // Les événements de défilement précèdent les minuteurs et les
        // rappels d'animation de la même image.
        let mut redraw = self.pump_scroll();
        if let Some(engine) = self.engine.as_mut() {
            engine.run_timers(&mut self.doc, now);
            if engine.take_dirty() {
                self.dom_changed();
                redraw = true;
            }
        }
        redraw | self.pump_animations()
    }

    /// Distribue les événements `scroll` des zones qui ont défilé, et
    /// `scrollend` à celles qui avaient défilé à l'image précédente et ne
    /// bougent plus. Renvoie `true` si la page doit être redessinée.
    fn pump_scroll(&mut self) -> bool {
        let changed = self.scrolling.lock().take_changed();
        let ended: Vec<u32> = std::mem::take(&mut self.scroll_ending)
            .into_iter()
            .filter(|id| !changed.contains(id))
            .collect();
        self.scroll_ending.clone_from(&changed);
        if changed.is_empty() && ended.is_empty() {
            return false;
        }
        if let Some(engine) = self.engine.as_mut() {
            engine.scroll_events(&mut self.doc, &changed, &ended);
            if engine.take_dirty() {
                self.dom_changed();
            }
        }
        true
    }

    /// Défilement de la fenêtre, en pixels CSS.
    #[must_use]
    pub fn scroll_offset(&self) -> (f32, f32) {
        self.scrolling
            .lock()
            .offset(oxyde_compositor::ROOT_SCROLLER)
    }

    /// Défilement de la zone `id` de la liste d'affichage
    /// ([`oxyde_paint::ROOT_SCROLLER`] pour la fenêtre) : à passer au
    /// compositeur avec [`Page::display_list`].
    #[must_use]
    pub fn scroll_of(&self, id: u32) -> (f32, f32) {
        self.scrolling.lock().offset(id)
    }

    /// Fait défiler la zone `id` jusqu'à (`x`, `y`), dans ses bornes : pour
    /// un compositeur qui a déjà fait défiler la zone de son côté.
    pub fn set_scroll(&mut self, id: u32, x: f32, y: f32) -> bool {
        self.scrolling.lock().scroll_within_limit(id, x, y)
    }

    /// Décalages de défilement non nuls : (zone, x, y).
    #[must_use]
    pub fn scroll_offsets(&self) -> Vec<(u32, f32, f32)> {
        self.scrolling.lock().offsets()
    }

    /// Molette de (`dx`, `dy`) au point (`x`, `y`) de la fenêtre : fait
    /// défiler la zone située sous le pointeur, ou la fenêtre. Renvoie
    /// `true` si quelque chose a défilé ; seule la composition est à
    /// refaire.
    pub fn wheel(&mut self, x: f32, y: f32, dx: f32, dy: f32) -> bool {
        self.scrolling.lock().wheel(x, y, dx, dy).is_some()
    }

    /// Fait avancer les animations et les transitions jusqu'à l'instant
    /// courant et distribue leurs événements. Renvoie `true` si la page doit
    /// être redessinée.
    fn pump_animations(&mut self) -> bool {
        let animating = self.animations.needs_frames();
        if animating {
            // Le calcul de style fait avancer les effets et relève leurs
            // événements ; la page est redessinée à la prochaine mise à jour.
            let sheets = usable_sheets(&self.sheets);
            let _ = self.animations.styles(&self.doc, &sheets, self.viewport);
            self.dirty = true;
        }
        let events = self.animations.take_events();
        if !events.is_empty()
            && let Some(engine) = self.engine.as_mut()
        {
            engine.animation_events(&mut self.doc, &events);
            if engine.take_dirty() {
                self.dom_changed();
            }
        }
        animating
    }

    /// Instant du prochain minuteur à exécuter, ou de la prochaine image
    /// d'une animation en cours, s'il y en a.
    #[must_use]
    pub fn next_wake(&mut self) -> Option<Instant> {
        let timer = self.engine.as_mut().and_then(|engine| engine.next_due());
        // Une animation en cours, ou un événement `scroll` à distribuer.
        let frame = (self.animations.needs_frames()
            || self.scrolling.lock().has_changed()
            || !self.scroll_ending.is_empty())
        .then(|| self.animations.instant_at(self.animations.now()) + FRAME);
        match (timer, frame) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }

    /// Clic au point (`x`, `y`) de la page : distribue l'événement `click` aux
    /// scripts, puis renvoie la cible du lien à suivre si l'action par défaut
    /// n'a pas été annulée.
    pub fn click(&mut self, x: f32, y: f32) -> Option<Url> {
        let node = self.node_at(x, y)?;
        // Le clic reprend le rang et les touches du bouton relâché.
        let pointer = self.client_input(&MouseInput {
            x,
            y,
            button: 0,
            buttons: 0,
            detail: self.released.detail.max(1),
            ..self.released
        });
        // Ce relâchement est consommé : un clic venu d'ailleurs ne le reprend pas.
        self.released = MouseInput::default();
        let link = self.click_with(node, Some(&pointer));
        self.place_caret(node, x, y);
        link
    }

    /// Le même pointeur, dans le repère de la fenêtre (`clientX`,
    /// `clientY`) : `input` est dans celui de la page.
    fn client_input(&self, input: &MouseInput) -> MouseInput {
        let (scroll_x, scroll_y) = self.scroll_offset();
        MouseInput {
            x: input.x - scroll_x,
            y: input.y - scroll_y,
            ..*input
        }
    }

    /// Élément visé par un événement de souris au point (`x`, `y`) de la
    /// page : celui sous le pointeur, ou celui qui contient le texte visé.
    fn mouse_target(&self, x: f32, y: f32) -> Option<NodeId> {
        let node = self.node_at(x, y)?;
        std::iter::once(node)
            .chain(self.doc.ancestors(node))
            .find(|n| self.doc.element(*n).is_some())
    }

    /// Distribue les événements de souris `kinds` sur `target` et note si
    /// la page doit être redessinée. Renvoie `true` si l'un est annulé.
    fn fire_mouse(
        &mut self,
        kinds: &[&str],
        target: Option<NodeId>,
        related: Option<NodeId>,
        input: &MouseInput,
    ) -> bool {
        let Some(engine) = self.engine.as_mut() else {
            return false;
        };
        let mut prevented = false;
        for kind in kinds {
            prevented |= engine.mouse_event(&mut self.doc, kind, target, related, input);
        }
        if engine.take_dirty() {
            self.dom_changed();
        }
        prevented
    }

    /// Le pointeur est au point (`input.x`, `input.y`) de la page : les
    /// scripts reçoivent `pointermove` et `mousemove`, précédés, quand
    /// l'élément survolé change, de `mouseout` et `mouseleave` sur ceux
    /// qui sont quittés puis de `mouseover` et `mouseenter` sur ceux qui
    /// sont rejoints (et de leurs équivalents `pointer…`). Renvoie `true`
    /// si la page doit être redessinée.
    ///
    /// Une page sans script ne reçoit rien : personne n'écoute.
    ///
    /// LIMITES : `:hover` ne suit pas le pointeur ; la sortie du pointeur
    /// hors de la page n'est pas signalée ; `offsetX` et `offsetY` valent
    /// `pageX` et `pageY` au lieu d'être relatifs à la cible.
    pub fn mouse_move(&mut self, input: &MouseInput) -> bool {
        if self.engine.is_none() {
            return false;
        }
        let target = self.mouse_target(input.x, input.y);
        let pointer = MouseInput {
            buttons: self.buttons,
            detail: 0,
            ..self.client_input(input)
        };
        let before = self.dirty;
        let old = self.hovered;
        if old != target {
            let chain = |node: Option<NodeId>| -> Vec<NodeId> {
                node.into_iter()
                    .flat_map(|n| std::iter::once(n).chain(self.doc.ancestors(n)))
                    .filter(|n| self.doc.element(*n).is_some())
                    .collect()
            };
            let (left, entered) = (chain(old), chain(target));
            if old.is_some() {
                self.fire_mouse(&["pointerout", "mouseout"], old, target, &pointer);
            }
            // Du plus profond au plus englobant.
            for node in left.iter().filter(|n| !entered.contains(n)) {
                self.fire_mouse(
                    &["pointerleave", "mouseleave"],
                    Some(*node),
                    target,
                    &pointer,
                );
            }
            if target.is_some() {
                self.fire_mouse(&["pointerover", "mouseover"], target, old, &pointer);
            }
            // Du plus englobant au plus profond.
            for node in entered.iter().rev().filter(|n| !left.contains(n)) {
                self.fire_mouse(&["pointerenter", "mouseenter"], Some(*node), old, &pointer);
            }
            self.hovered = target;
        }
        self.fire_mouse(&["pointermove", "mousemove"], target, None, &pointer);
        self.dirty && !before
    }

    /// Bouton `input.button` enfoncé (`down`) ou relâché au point
    /// (`input.x`, `input.y`) de la page : `pointerdown` et `mousedown`,
    /// ou `pointerup` et `mouseup`. Le clic lui-même est [`Page::click`].
    /// Renvoie `true` si la page doit être redessinée.
    ///
    /// LIMITE : annuler `mousedown` n'empêche ni le focus ni la sélection
    /// de texte.
    pub fn mouse_button(&mut self, down: bool, input: &MouseInput) -> bool {
        let bit = match input.button {
            0 => 1,
            1 => 4,
            2 => 2,
            _ => 0,
        };
        if down {
            self.buttons |= bit;
        } else {
            self.buttons &= !bit;
            self.released = *input;
        }
        if self.engine.is_none() {
            return false;
        }
        let target = self.mouse_target(input.x, input.y);
        let pointer = MouseInput {
            buttons: self.buttons,
            ..self.client_input(input)
        };
        let before = self.dirty;
        let kinds: &[&str] = if down {
            &["pointerdown", "mousedown"]
        } else {
            &["pointerup", "mouseup"]
        };
        self.fire_mouse(kinds, target, None, &pointer);
        self.dirty && !before
    }

    /// Contact tactile `id` au point (`x`, `y`) de la page : les scripts
    /// reçoivent `touchstart`, `touchmove`, `touchend` ou `touchcancel`, sur
    /// l'élément où le contact a commencé. Un appui bref que rien n'a annulé
    /// se termine par les événements de souris et le clic. Renvoie `true`
    /// si un script a annulé l'événement : l'embarqueur ne fait alors pas
    /// défiler la page.
    ///
    /// LIMITES : pas d'événements `pointer…` de type `touch` ; la fenêtre
    /// du navigateur ne transmet pas encore l'écran tactile.
    pub fn touch(&mut self, phase: TouchPhase, id: u32, x: f32, y: f32) -> bool {
        /// Nombre de contacts suivis à la fois.
        const MAX_TOUCHES: usize = 16;
        let scroll = self.scroll_offset();
        let at = (x - scroll.0, y - scroll.1);
        let (kind, target) = match phase {
            TouchPhase::Start => {
                if self.touches.len() >= MAX_TOUCHES && !self.touches.contains_key(&id) {
                    return false;
                }
                ("touchstart", self.mouse_target(x, y))
            }
            TouchPhase::Move => ("touchmove", None),
            TouchPhase::End => ("touchend", None),
            TouchPhase::Cancel => ("touchcancel", None),
        };
        if phase != TouchPhase::Start && !self.touches.contains_key(&id) {
            return false;
        }
        let Some(engine) = self.engine.as_mut() else {
            return false;
        };
        let prevented = engine.touch_event(&mut self.doc, kind, target, id, at, scroll);
        if engine.take_dirty() {
            self.dom_changed();
        }
        match phase {
            TouchPhase::Start => {
                self.touches.insert(
                    id,
                    TouchState {
                        start: (x, y),
                        moved: false,
                        prevented,
                    },
                );
            }
            TouchPhase::Move => {
                if let Some(state) = self.touches.get_mut(&id) {
                    state.prevented |= prevented;
                    state.moved |= (x - state.start.0).abs() > TAP_SLOP
                        || (y - state.start.1).abs() > TAP_SLOP;
                }
            }
            TouchPhase::End => {
                // Un appui bref, que rien n'a annulé : souris et clic.
                let tap = self
                    .touches
                    .remove(&id)
                    .is_some_and(|state| !state.moved && !state.prevented);
                if tap && !prevented {
                    let input = MouseInput {
                        x,
                        y,
                        detail: 1,
                        ..MouseInput::default()
                    };
                    self.mouse_move(&input);
                    self.mouse_button(true, &input);
                    self.mouse_button(false, &input);
                    self.click(x, y);
                }
            }
            TouchPhase::Cancel => {
                self.touches.remove(&id);
            }
        }
        prevented
    }

    /// Clic droit au point (`x`, `y`) de la page : les scripts reçoivent
    /// `contextmenu`. Renvoie `true` s'ils l'ont annulé : l'embarqueur
    /// n'ouvre alors pas son menu.
    pub fn context_menu_event(&mut self, x: f32, y: f32) -> bool {
        let target = self.mouse_target(x, y);
        let pointer = self.client_input(&MouseInput {
            x,
            y,
            button: 2,
            ..MouseInput::default()
        });
        self.fire_mouse(&["contextmenu"], target, None, &pointer)
    }

    /// Molette de (`dx`, `dy`) au point (`input.x`, `input.y`) de la
    /// fenêtre : les scripts reçoivent `wheel` (et l'ancien `mousewheel`).
    /// Renvoie `true` s'ils ont annulé le défilement : l'embarqueur
    /// n'appelle alors pas [`Page::wheel`].
    pub fn wheel_event(&mut self, input: &MouseInput, dx: f32, dy: f32) -> bool {
        let (scroll_x, scroll_y) = self.scroll_offset();
        let target = self.mouse_target(input.x + scroll_x, input.y + scroll_y);
        let Some(engine) = self.engine.as_mut() else {
            return false;
        };
        if engine.wheel_listeners() == 0 {
            return false;
        }
        let pointer = MouseInput {
            buttons: self.buttons,
            detail: 0,
            ..*input
        };
        let prevented = engine.wheel_event(&mut self.doc, target, &pointer, (dx, dy));
        if engine.take_dirty() {
            self.dom_changed();
        }
        prevented
    }

    /// Écouteurs de molette de la page : 0 aucun, 1 des passifs seulement,
    /// 2 au moins un qui peut annuler le défilement — l'embarqueur doit
    /// alors attendre [`Page::wheel_event`] avant de faire défiler.
    pub fn wheel_listeners(&mut self) -> u8 {
        self.engine
            .as_mut()
            .map_or(0, |engine| engine.wheel_listeners())
    }

    /// Après un clic sur le champ de texte `node`, qui vient de prendre le
    /// focus : le curseur va au caractère le plus proche du point (`x`,
    /// `y`) (de l'abscisse seule dans un champ d'une ligne).
    fn place_caret(&mut self, node: NodeId, x: f32, y: f32) {
        if self.doc.focus() != Some(node) {
            return;
        }
        if script::is_text_area(&self.doc, node) {
            let Some(shown) = self
                .tree
                .as_ref()
                .and_then(|tree| oxyde_layout::area_caret_at(tree, node, x, y))
            else {
                return;
            };
            let value = self.doc.text_content(node);
            let mut caret = shown.min(value.len());
            while !value.is_char_boundary(caret) {
                caret -= 1;
            }
            if caret != self.doc.caret() || caret != self.doc.selection_anchor() {
                self.doc.set_focus(Some(node), caret);
                self.dom_changed();
            }
            return;
        }
        if !script::is_text_input(&self.doc, node) {
            return;
        }
        let (Some(tree), Some(element)) = (&self.tree, self.doc.element(node)) else {
            return;
        };
        let value = element.attr("value").unwrap_or("");
        // Un champ vide montre son texte indicatif : le curseur reste au début.
        let (Some(shown), false) = (
            oxyde_layout::field_caret_at(tree, node, x),
            value.is_empty(),
        ) else {
            return;
        };
        let secret = element
            .attr("type")
            .is_some_and(|kind| kind.eq_ignore_ascii_case("password"));
        let mut caret = if secret {
            // Un point affiché par caractère de la valeur.
            value
                .char_indices()
                .nth(shown / oxyde_layout::PASSWORD_MASK.len_utf8())
                .map_or(value.len(), |(at, _)| at)
        } else {
            shown.min(value.len())
        };
        while !value.is_char_boundary(caret) {
            caret -= 1;
        }
        if caret != self.doc.caret() || caret != self.doc.selection_anchor() {
            self.doc.set_focus(Some(node), caret);
            self.dom_changed();
        }
    }

    /// Clic sur le nœud `node`, comme [`Page::click`] quand ce nœud est sous
    /// le pointeur.
    pub fn click_node(&mut self, node: NodeId) -> Option<Url> {
        self.click_with(node, None)
    }

    /// Clic sur `node` ; `pointer` : le pointeur, dans le repère de la
    /// fenêtre, quand le clic vient de la souris.
    fn click_with(&mut self, node: NodeId, pointer: Option<&MouseInput>) -> Option<Url> {
        let link = self.link_from(node);
        // Une page sans script a aussi des formulaires : le moteur naît au
        // premier geste qui en a besoin.
        self.ensure_engine();
        let Some(engine) = self.engine.as_mut() else {
            return link;
        };
        let prevented = engine.click(&mut self.doc, node, pointer);
        if engine.take_dirty() {
            self.dom_changed();
        }
        if prevented { None } else { link }
    }

    /// Titre du document (contenu de `<title>`, blancs normalisés).
    #[must_use]
    pub fn title(&self) -> String {
        self.doc
            .find_html_element(self.doc.root(), "title")
            .map(|n| {
                self.doc
                    .text_content(n)
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_default()
    }

    /// Vrai si la page doit être remise en page au prochain [`Page::update`].
    #[must_use]
    pub fn needs_update(&self) -> bool {
        self.dirty
    }

    /// Recalcule style, mise en page et peinture si la fenêtre, le style, le
    /// DOM ou les images ont changé. Renvoie `true` si la liste d'affichage a
    /// été reconstruite.
    pub fn update(&mut self, text: &mut TextContext, viewport: Viewport) -> bool {
        if !self.dirty && viewport == self.viewport {
            return false;
        }
        self.viewport = viewport;
        self.dirty = false;
        if let Some(engine) = self.engine.as_mut() {
            engine.set_viewport(viewport.width, viewport.height);
        }
        register_fonts(&self.fonts, text);
        let sheets = usable_sheets(&self.sheets);
        (self.styles, _) = self.animations.styles(&self.doc, &sheets, viewport);
        // Les canevas redessinés depuis la dernière peinture.
        if let Some(engine) = self.engine.as_mut() {
            for (index, width, height, rgba) in engine.take_canvas_frames() {
                let Some(node) = self.doc.id_from_index(index as usize) else {
                    continue;
                };
                match Image::from_rgba(width, height, rgba) {
                    Some(image) => self.canvas_images.insert(node, image),
                    None => self.canvas_images.remove(&node),
                };
            }
        }
        let images = &self.images;
        // Les `<svg>` sont des éléments remplacés : leur taille propre vient
        // de leurs attributs.
        let svg_sizes: HashMap<NodeId, (f32, f32)> = svg::outer_roots(&self.doc)
            .into_iter()
            .map(|node| (node, svg::intrinsic_size(&self.doc, node)))
            .collect();
        #[expect(clippy::cast_precision_loss, reason = "côtés d'image bornés à 16 384")]
        let intrinsic = |node: NodeId| {
            images
                .get(&node)
                .and_then(|slot| slot.image.as_ref())
                .map(|image| (image.width() as f32, image.height() as f32))
                .or_else(|| svg_sizes.get(&node).copied())
        };
        let tree =
            oxyde_layout::layout_document(&self.doc, &self.styles, text, viewport, &intrinsic);
        let frame_lists = self.paint_frames(&tree, text);
        self.render_svgs(&tree, &svg_sizes, text);
        let images = &self.images;
        let canvases = &self.canvas_images;
        let svgs = &self.svg_images;
        let image_of = |node: NodeId| {
            images
                .get(&node)
                .and_then(|slot| slot.image.clone())
                .or_else(|| canvases.get(&node).cloned())
                .or_else(|| svgs.get(&node).map(|(_, image)| image.clone()))
        };
        self.list = oxyde_paint::paint(&tree, viewport, &image_of, &|node| frame_lists.get(&node));
        self.painted = self.list.items.len();
        // Les décalages de défilement sont ramenés dans les bornes du
        // nouveau contenu.
        self.scrolling.lock().update(
            &self.list.scrollers,
            (viewport.width, viewport.height),
            (tree.width, tree.height),
        );
        self.tree = Some(tree);
        // La mise en page a changé : les occurrences aussi.
        if let (Some(find), Some(tree)) = (&mut self.find, &self.tree) {
            find.matches = oxyde_layout::find_text(tree, &find.query);
            find.current = find.current.min(find.matches.len().saturating_sub(1));
        }
        // La sélection aussi : ses points sont ceux de la page.
        self.refresh_selection();
        self.paint_find();
        true
    }

    /// Rend les éléments `<svg>` de `roots` à la taille de leur boîte de
    /// contenu dans `tree` ; un rendu dont rien n'a changé est gardé.
    fn render_svgs(
        &mut self,
        tree: &oxyde_layout::LayoutTree,
        roots: &HashMap<NodeId, (f32, f32)>,
        text: &mut TextContext,
    ) {
        if roots.is_empty() {
            self.svg_images.clear();
            return;
        }
        let index = svg::Index::new(&self.doc);
        let mut kept = HashMap::new();
        for (node, rect) in oxyde_paint::content_boxes(tree, &|node| roots.contains_key(&node)) {
            let signature = svg::signature(&self.doc, &self.styles, node, rect.w, rect.h);
            let image = match self.svg_images.remove(&node) {
                Some((known, image)) if known == signature => Some(image),
                _ => svg::render(
                    &self.doc,
                    &self.styles,
                    &index,
                    node,
                    (rect.w, rect.h),
                    svg::PAGE_SCALE,
                    Some(&mut *text),
                ),
            };
            if let Some(image) = image {
                kept.insert(node, (signature, image));
            }
        }
        self.svg_images = kept;
    }

    /// Met en page et peint le document de chaque cadre (`<iframe>`) chargé
    /// qui occupe une place dans `tree`, à la taille de sa boîte de contenu.
    ///
    /// LIMITES : le document du cadre est recopié et remis en page à chaque
    /// peinture de la page ; ses feuilles de style liées, ses images, ses
    /// animations et les cadres qu'il contient ne sont pas dessinés ; on ne
    /// peut ni y cliquer ni le faire défiler.
    fn paint_frames(
        &mut self,
        tree: &oxyde_layout::LayoutTree,
        text: &mut TextContext,
    ) -> HashMap<NodeId, DisplayList> {
        let mut lists = HashMap::new();
        let Some(engine) = self.engine.as_mut() else {
            return lists;
        };
        let documents: HashMap<NodeId, NodeId> =
            engine.frame_documents(&self.doc).into_iter().collect();
        if documents.is_empty() {
            return lists;
        }
        let boxes = oxyde_paint::content_boxes(tree, &|node| documents.contains_key(&node));
        for (element, rect) in boxes {
            let Some(document) = documents.get(&element) else {
                continue;
            };
            if rect.w <= 0.0 || rect.h <= 0.0 {
                continue;
            }
            let mut copy = Document::new();
            let children: Vec<NodeId> = self.doc.children(*document).collect();
            for child in children {
                let imported = copy.import(&self.doc, child);
                let root = copy.root();
                copy.append_child(root, imported);
            }
            let base = self
                .frames
                .iter()
                .find(|frame| frame.element == element)
                .map_or(&self.url, |frame| &frame.url);
            let sheets = usable_sheets(&collect_sheets(&copy, base, Vec::new()));
            let viewport = Viewport {
                width: rect.w,
                height: rect.h,
            };
            let styles = oxyde_css::compute_styles(&copy, &sheets, viewport);
            let frame_tree =
                oxyde_layout::layout_document(&copy, &styles, text, viewport, &|_| None);
            lists.insert(
                element,
                oxyde_paint::paint_frame(&frame_tree, viewport, &|_| None),
            );
        }
        lists
    }

    /// Cherche `query` dans le texte de la page, sans tenir compte de la
    /// casse, et surligne ses occurrences. L'occurrence courante est la
    /// première à partir du haut de la fenêtre ; la page défile jusqu'à
    /// elle si elle n'est pas visible. Une recherche vide efface le
    /// surlignage. La page doit avoir été mise en page ([`Page::update`]).
    pub fn find(&mut self, query: &str) -> FindStatus {
        if query.is_empty() {
            self.find = None;
        } else if !self.find.as_ref().is_some_and(|find| find.query == query) {
            let matches = self
                .tree
                .as_ref()
                .map(|tree| oxyde_layout::find_text(tree, query))
                .unwrap_or_default();
            let (_, top) = self.scroll_offset();
            let current = matches
                .iter()
                .position(|rects| rects.first().is_some_and(|r| r.y + r.h > top))
                .unwrap_or(0);
            self.find = Some(Find {
                query: query.to_owned(),
                matches,
                current,
            });
            self.reveal_find();
        }
        self.paint_find();
        self.find_status()
    }

    /// Passe à l'occurrence suivante (`forward`) ou précédente de la
    /// recherche en cours, en bouclant, et fait défiler la page jusqu'à elle.
    pub fn find_step(&mut self, forward: bool) -> FindStatus {
        if let Some(find) = &mut self.find
            && !find.matches.is_empty()
        {
            let count = find.matches.len();
            let step = if forward { 1 } else { count - 1 };
            find.current = (find.current + step) % count;
        }
        self.reveal_find();
        self.paint_find();
        self.find_status()
    }

    /// Avancement de la recherche en cours.
    #[must_use]
    pub fn find_status(&self) -> FindStatus {
        match &self.find {
            Some(find) if !find.matches.is_empty() => FindStatus {
                count: find.matches.len(),
                current: find.current + 1,
            },
            _ => FindStatus::default(),
        }
    }

    /// Fait défiler la fenêtre pour que l'occurrence courante soit visible.
    fn reveal_find(&mut self) {
        let rect = self
            .find
            .as_ref()
            .and_then(|find| find.matches.get(find.current))
            .and_then(|rects| rects.first())
            .copied();
        let Some(rect) = rect else {
            return;
        };
        let (x, y) = self.scroll_offset();
        let (width, height) = (self.viewport.width, self.viewport.height);
        // Une marge : l'occurrence ne colle pas au bord, ni ne passe sous la
        // barre de recherche.
        let margin = 64.0f32.min(height / 4.0);
        let fits_x = rect.x >= x && rect.x + rect.w <= x + width;
        let fits_y = rect.y >= y + margin && rect.y + rect.h <= y + height - margin;
        if fits_x && fits_y {
            return;
        }
        let target_x = if fits_x {
            x
        } else {
            (rect.x - margin).max(0.0)
        };
        let target_y = if fits_y {
            y
        } else {
            (rect.y + rect.h / 2.0 - height / 2.0).max(0.0)
        };
        self.set_scroll(oxyde_compositor::ROOT_SCROLLER, target_x, target_y);
    }

    /// Recalcule la sélection dans la mise en page courante.
    fn refresh_selection(&mut self) {
        let (Some(selection), Some(tree)) = (&mut self.selection, &self.tree) else {
            return;
        };
        selection.current = match selection.kind {
            SelectionKind::Range { anchor, focus } => {
                oxyde_layout::select_between(tree, anchor, focus)
            }
            SelectionKind::Word { x, y } => oxyde_layout::select_word(tree, x, y),
            SelectionKind::All => oxyde_layout::select_all(tree),
        };
    }

    /// Remplace la sélection par `kind` ; renvoie `true` si le texte
    /// sélectionné, donc l'affichage, a changé.
    fn select(&mut self, kind: Option<SelectionKind>) -> bool {
        let before = self
            .selection
            .take()
            .map(|selection| selection.current.rects)
            .unwrap_or_default();
        self.selection = kind.map(|kind| Selected {
            kind,
            current: oxyde_layout::TextSelection::default(),
        });
        self.refresh_selection();
        self.paint_find();
        let after = self
            .selection
            .as_ref()
            .map_or(&[][..], |selection| selection.current.rects.as_slice());
        before != after
    }

    /// Commence une sélection de texte au point (`x`, `y`) de la page
    /// (bouton de la souris enfoncé) : l'ancienne est effacée. Renvoie
    /// `true` si l'affichage a changé.
    ///
    /// LIMITE : la sélection n'est pas visible des scripts
    /// (`getSelection()` reste vide) et n'émet pas `selectionchange`.
    pub fn select_start(&mut self, x: f32, y: f32) -> bool {
        self.select(Some(SelectionKind::Range {
            anchor: (x, y),
            focus: (x, y),
        }))
    }

    /// Étend la sélection commencée par [`Page::select_start`] jusqu'au
    /// point (`x`, `y`) (souris déplacée, bouton enfoncé). Renvoie `true`
    /// si l'affichage a changé.
    pub fn select_to(&mut self, x: f32, y: f32) -> bool {
        match self.selection.as_ref().map(|selection| selection.kind) {
            Some(SelectionKind::Range { anchor, .. }) => self.select(Some(SelectionKind::Range {
                anchor,
                focus: (x, y),
            })),
            _ => false,
        }
    }

    /// Sélectionne le mot sous le point (`x`, `y`) (double clic).
    pub fn select_word(&mut self, x: f32, y: f32) -> bool {
        self.select(Some(SelectionKind::Word { x, y }))
    }

    /// Sélectionne tout le texte de la page (Ctrl A).
    pub fn select_all(&mut self) -> bool {
        self.select(Some(SelectionKind::All))
    }

    /// Efface la sélection de texte.
    pub fn clear_selection(&mut self) -> bool {
        self.select(None)
    }

    /// Texte sélectionné, une ligne par bloc de texte ; vide sans sélection.
    #[must_use]
    pub fn selected_text(&self) -> &str {
        self.selection
            .as_ref()
            .map_or("", |selection| selection.current.text.as_str())
    }

    /// Remplace le surlignage de la recherche à la fin de la liste d'affichage.
    fn paint_find(&mut self) {
        self.list.items.truncate(self.painted);
        if let Some(selection) = &self.selection {
            for rect in &selection.current.rects {
                self.list.rect(*rect, SELECTION, 0.0);
            }
        }
        let Some(find) = &self.find else {
            return;
        };
        for (index, rects) in find.matches.iter().enumerate() {
            let color = if index == find.current {
                FIND_CURRENT
            } else {
                FIND_HIGHLIGHT
            };
            for rect in rects {
                self.list.rect(*rect, color, 2.0);
            }
        }
    }

    /// Liste d'affichage de la dernière mise en page (coordonnées de la page).
    #[must_use]
    pub fn display_list(&self) -> &DisplayList {
        &self.list
    }

    /// Largeur et hauteur du contenu, en pixels CSS.
    #[must_use]
    pub fn content_size(&self) -> (f32, f32) {
        self.tree
            .as_ref()
            .map_or((0.0, 0.0), |t| (t.width, t.height))
    }

    /// Hauteur totale du contenu, en pixels CSS.
    #[must_use]
    pub fn content_height(&self) -> f32 {
        self.tree.as_ref().map_or(0.0, |t| t.height)
    }

    /// Couleur de fond du canevas.
    #[must_use]
    pub fn background(&self) -> Color {
        self.tree
            .as_ref()
            .map_or(Color::WHITE, oxyde_paint::canvas_color)
    }

    fn root_fragment(&self) -> Option<&Fragment> {
        self.tree.as_ref()?.root.as_ref()
    }

    /// Nœud situé au point (`x`, `y`) de la page.
    #[must_use]
    pub fn node_at(&self, x: f32, y: f32) -> Option<NodeId> {
        let tree = self.scrolling.lock();
        let offset = |node: Option<NodeId>| {
            tree.offset(node.map_or(oxyde_compositor::ROOT_SCROLLER, scroll_id))
        };
        oxyde_layout::hit_test_scrolled(&self.doc, self.root_fragment()?, x, y, &offset)
    }

    /// Cible du lien qui contient `node`, s'il y en a un.
    fn link_from(&self, node: NodeId) -> Option<Url> {
        std::iter::once(node)
            .chain(self.doc.ancestors(node))
            .find_map(|n| {
                let el = self.doc.element(n)?;
                if !el.is_html("a") {
                    return None;
                }
                self.base.join(el.attr("href")?).ok()
            })
    }

    /// Cible du lien situé au point (`x`, `y`) de la page, s'il y en a un.
    #[must_use]
    pub fn link_at(&self, x: f32, y: f32) -> Option<Url> {
        self.link_from(self.node_at(x, y)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxyde_paint::DisplayItem;

    /// Élément d'identifiant `id` de la page.
    fn doc_node(p: &Page, id: &str) -> NodeId {
        p.document()
            .descendants(p.document().root())
            .find(|n| {
                p.document()
                    .element(*n)
                    .is_some_and(|e| e.attr("id") == Some(id))
            })
            .expect("élément")
    }

    fn page(html: &str) -> Page {
        Page::from_html(
            Url::parse("https://exemple.test/dossier/page.html").expect("url"),
            html,
        )
    }

    const VIEWPORT: Viewport = Viewport {
        width: 400.0,
        height: 300.0,
    };

    /// PNG 2×1 : un pixel rouge, un pixel bleu.
    fn tiny_png() -> Vec<u8> {
        let mut out = Vec::new();
        let rgba =
            image::RgbaImage::from_raw(2, 1, vec![255, 0, 0, 255, 0, 0, 255, 255]).expect("tampon");
        image::DynamicImage::ImageRgba8(rgba)
            .write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png)
            .expect("encodage PNG");
        out
    }

    /// Texte visible du corps (sans le source des scripts), blancs normalisés.
    fn body_text(p: &Page) -> String {
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        let in_script = |n: NodeId| {
            doc.ancestors(n)
                .any(|a| doc.element(a).is_some_and(|e| e.is_html("script")))
        };
        let text: String = doc
            .descendants(body)
            .filter(|n| !in_script(*n))
            .filter_map(|n| doc.text(n))
            .collect();
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    #[test]
    fn title_links_and_stylesheets() {
        let mut p = page(
            "<title> Bonjour\n  Oxyde </title>\
             <link rel='stylesheet' href='../a.css'><link rel=icon href=i.png>\
             <style>body{margin:0} p{margin:0;font-size:20px;line-height:30px}</style>\
             <p><a href='suite.html#x'>un lien</a></p>",
        );
        assert_eq!(p.title(), "Bonjour Oxyde");
        let pending = p.pending_stylesheets();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].as_str(), "https://exemple.test/a.css");

        let mut text = TextContext::new();
        assert!(p.update(&mut text, VIEWPORT));
        assert!(!p.update(&mut text, VIEWPORT), "rien n'a changé");
        assert_eq!(p.content_height(), 30.0);
        assert_eq!(
            p.link_at(5.0, 15.0).map(String::from).as_deref(),
            Some("https://exemple.test/dossier/suite.html#x")
        );
        assert_eq!(p.link_at(5.0, 200.0), None);

        assert!(p.set_stylesheet(&pending[0], "p { line-height: 50px !important }"));
        assert!(!p.set_stylesheet(&pending[0], ""), "déjà fournie");
        assert!(p.update(&mut text, VIEWPORT));
        assert_eq!(p.content_height(), 50.0);
    }

    #[test]
    fn animations_ask_for_frames_and_move_the_paint() {
        let mut p = page(
            "<style>body{margin:0} @keyframes glisse { from { width: 100px } to { width: 300px } }             div{height:10px;background:red;animation:glisse 1s linear}</style><div></div>",
        );
        let mut text = TextContext::new();
        assert!(p.update(&mut text, VIEWPORT));
        let red_width = |p: &Page| {
            p.display_list().items.iter().find_map(|item| match item {
                DisplayItem::Rect { rect, color, .. } if *color == Color::rgb(255, 0, 0) => {
                    Some(rect.w)
                }
                _ => None,
            })
        };
        let start = red_width(&p).expect("boîte rouge");
        assert!(start < 110.0, "au départ : {start}");
        // L'animation demande une image, et l'instant suivant la fait avancer.
        let wake = p
            .next_wake()
            .expect("une animation en cours demande une image");
        assert!(
            p.tick(wake + Duration::from_millis(500)),
            "la page doit être redessinée"
        );
        assert!(p.update(&mut text, VIEWPORT));
        let middle = red_width(&p).expect("boîte rouge");
        assert!(middle > 190.0 && middle < 230.0, "à mi-course : {middle}");
        // Terminée : retour au style de base, plus d'image demandée.
        p.tick(wake + Duration::from_secs(2));
        p.update(&mut text, VIEWPORT);
        assert_eq!(p.next_wake(), None);
    }

    #[test]
    fn wheel_scrolls_the_zone_under_the_pointer() {
        let mut p = page(
            "<style>body{margin:0}</style>\
             <div id=zone style='height:100px;overflow:auto'>\
             <a id=haut href='haut.html' style='display:block;height:150px'>haut</a>\
             <a id=bas href='bas.html' style='display:block;height:150px'>bas</a></div>\
             <div style='height:1000px'></div>",
        );
        let mut text = TextContext::new();
        assert!(p.update(&mut text, VIEWPORT));
        let link_at = |p: &Page, x: f32, y: f32| {
            let node = p.node_at(x, y)?;
            p.link_from(node).map(|url| url.path().to_owned())
        };
        assert_eq!(
            link_at(&p, 10.0, 50.0).as_deref(),
            Some("/dossier/haut.html")
        );
        // Au-dessus de la zone : c'est elle qui défile, pas la fenêtre.
        assert!(p.wheel(10.0, 50.0, 0.0, 120.0));
        assert_eq!(p.scroll_offset(), (0.0, 0.0));
        assert_eq!(
            link_at(&p, 10.0, 50.0).as_deref(),
            Some("/dossier/bas.html")
        );
        // Ailleurs : la fenêtre, jusqu'au bout du contenu.
        assert!(p.wheel(10.0, 200.0, 0.0, 5000.0));
        assert_eq!(p.scroll_offset(), (0.0, 800.0));
        assert!(!p.wheel(10.0, 200.0, 0.0, 10.0), "en butée");
        // Le défilement ne redemande pas de mise en page.
        assert!(!p.needs_update());
    }

    #[test]
    fn decoding_rejects_garbage_and_accepts_png() {
        assert!(decode_image(b"").is_none());
        assert!(decode_image(b"pas une image").is_none());
        let image = decode_image(&tiny_png()).expect("PNG valide");
        assert_eq!((image.width(), image.height()), (2, 1));
        // Un PNG tronqué ne doit ni paniquer ni produire d'image.
        let png = tiny_png();
        for n in 0..png.len() {
            let _ = decode_image(&png[..n]);
        }
    }

    #[test]
    fn every_supported_format_decodes() {
        use image::ImageFormat::{Bmp, Gif, Jpeg, Png, WebP};
        for format in [Png, Jpeg, WebP, Gif, Bmp] {
            // Le JPEG n'a pas de canal alpha : on l'encode depuis du RGB.
            let source = if format == Jpeg {
                image::DynamicImage::ImageRgb8(
                    image::RgbImage::from_raw(4, 3, vec![200u8; 4 * 3 * 3]).expect("tampon"),
                )
            } else {
                image::DynamicImage::ImageRgba8(
                    image::RgbaImage::from_raw(4, 3, vec![200u8; 4 * 3 * 4]).expect("tampon"),
                )
            };
            let mut encoded = Vec::new();
            source
                .write_to(&mut Cursor::new(&mut encoded), format)
                .expect("encodage");
            let decoded = decode_image(&encoded).unwrap_or_else(|| panic!("{format:?} illisible"));
            assert_eq!((decoded.width(), decoded.height()), (4, 3), "{format:?}");
        }
    }

    #[test]
    fn images_size_the_box_and_are_painted() {
        let mut p = page(
            "<style>body{margin:0} img{display:block}</style>\
             <img src='a.png'><img src='a.png' width=100><img src='absente.png' alt='texte'>",
        );
        let pending = p.pending_images();
        assert_eq!(
            pending.len(),
            2,
            "une même adresse n'est demandée qu'une fois"
        );
        assert_eq!(pending[0].as_str(), "https://exemple.test/dossier/a.png");

        let mut text = TextContext::new();
        p.update(&mut text, VIEWPORT);
        let images = |p: &Page| -> Vec<(f32, f32)> {
            p.display_list()
                .items
                .iter()
                .filter_map(|i| match i {
                    DisplayItem::Image { rect, .. } => Some((rect.w, rect.h)),
                    _ => None,
                })
                .collect()
        };
        assert!(images(&p).is_empty(), "rien à peindre avant le chargement");

        assert!(p.set_image(&pending[0], decode_image(&tiny_png())));
        assert!(p.set_image(&pending[1], None));
        assert!(p.pending_images().is_empty());
        assert!(p.update(&mut text, VIEWPORT));
        // Taille intrinsèque 2×1, puis largeur imposée à 100 avec proportions conservées.
        assert_eq!(images(&p), vec![(2.0, 1.0), (100.0, 50.0)]);
    }

    #[test]
    fn scripts_build_and_modify_the_document() {
        let mut p = page(
            "<title>Avant</title><style>body{margin:0}</style>
             <ul id=liste></ul><p id=cible class='a b'>texte</p>
             <script>
               const liste = document.getElementById('liste');
               for (const nom of ['un', 'deux', 'trois']) {
                 const li = document.createElement('li');
                 li.textContent = nom;
                 liste.appendChild(li);
               }
               const cible = document.querySelector('p.a');
               cible.classList.remove('a');
               cible.classList.add('c');
               cible.style.backgroundColor = 'red';
               cible.setAttribute('data-n', liste.children.length);
               cible.innerHTML = '<b>gras</b> et <i>italique</i>';
               document.title = 'Après ' + document.querySelectorAll('li').length;
               console.log(cible.outerHTML);
             </script>
             <script src='externe.js'></script>
             <script type='text/plain'>document.title = 'type inconnu ignoré';</script>
             <script>document.body.append(' fin');</script>",
        );
        assert_eq!(p.pending_scripts().len(), 1);
        assert!(p.set_script(
            &p.pending_scripts()[0],
            Some("document.body.append(' externe');".to_owned())
        ));
        p.run_scripts();
        assert_eq!(p.title(), "Après 3");
        assert_eq!(body_text(&p), "undeuxtroisgras et italique externe fin");
        let console = p.take_console();
        assert_eq!(console.len(), 1, "{console:?}");
        assert_eq!(
            console[0].text,
            r#"<p id="cible" class="b c" style="background-color: red" data-n="3"><b>gras</b> et <i>italique</i></p>"#
        );
        // Le style posé par le script est pris en compte par la mise en page.
        let mut text = TextContext::new();
        assert!(p.update(&mut text, VIEWPORT));
        let red = p.display_list().items.iter().any(|item| {
            matches!(item, DisplayItem::Rect { color, .. } if *color == Color::rgb(255, 0, 0))
        });
        assert!(red, "le fond rouge posé par le script doit être peint");
    }

    #[test]
    fn events_timers_and_default_actions() {
        let mut p = page(
            "<style>body{margin:0} p{margin:0;font-size:20px;line-height:30px}</style>
             <p><a id=lien href='suite.html'>lien</a></p>
             <p><a id=bloque href='autre.html'>bloqué</a></p>
             <p id=compteur>0</p>
             <script>
               let clics = 0;
               const compteur = document.getElementById('compteur');
               document.getElementById('bloque').addEventListener('click', e => { e.preventDefault(); clics++; });
               document.body.onclick = e => { compteur.textContent = e.target.id + ':' + (++clics); };
               window.addEventListener('load', () => console.log('chargé'));
               setTimeout(() => { compteur.textContent += ' minuteur'; }, 0);
               const id = setTimeout(() => console.log('annulé'), 0);
               clearTimeout(id);
             </script>",
        );
        p.run_scripts();
        assert_eq!(
            p.take_console()
                .iter()
                .map(|m| m.text.as_str())
                .collect::<Vec<_>>(),
            ["chargé"]
        );
        let mut text = TextContext::new();
        p.update(&mut text, VIEWPORT);

        // Lien ordinaire : le script est prévenu, puis le lien est suivi.
        let followed = p.click(5.0, 15.0);
        assert_eq!(
            followed.map(String::from).as_deref(),
            Some("https://exemple.test/dossier/suite.html")
        );
        assert!(body_text(&p).ends_with("lien:1"));
        // Lien dont le clic est annulé par `preventDefault()`.
        p.update(&mut text, VIEWPORT);
        assert_eq!(p.click(5.0, 45.0), None);
        assert!(body_text(&p).ends_with("bloque:3"), "{}", body_text(&p));

        // Minuteur : exécuté à l'échéance, une seule fois.
        assert!(p.next_wake().is_some());
        assert!(p.tick(Instant::now() + std::time::Duration::from_secs(1)));
        assert!(body_text(&p).ends_with("bloque:3 minuteur"));
        assert!(p.next_wake().is_none());
        assert!(!p.tick(Instant::now() + std::time::Duration::from_secs(2)));
        assert!(
            p.take_console().is_empty(),
            "le minuteur annulé n'a pas tourné"
        );
    }

    #[test]
    fn timer_callbacks_reach_the_document() {
        let mut p = page(
            "<p id=p>a</p><script>setTimeout(() => { document.getElementById('p').textContent = 'fini'; }, 500);</script>",
        );
        p.run_scripts();
        let mut text = TextContext::new();
        p.update(&mut text, VIEWPORT);
        assert!(!p.tick(Instant::now()));
        assert!(p.tick(Instant::now() + std::time::Duration::from_secs(1)));
        assert_eq!(body_text(&p), "fini");
        assert!(p.next_wake().is_none());
    }

    #[test]
    fn promises_and_async_functions_drive_the_page() {
        let mut p = page(
            "<title>t</title><p id=p>a</p><script>
               const cible = document.getElementById('p');
               const attendre = ms => new Promise(resolve => setTimeout(resolve, ms));
               (async () => {
                 cible.textContent = 'début';
                 await attendre(100);
                 cible.textContent += ', minuteur';
                 const valeurs = await Promise.all([1, 2].map(async n => n * 10));
                 cible.textContent += ', ' + valeurs.join('+');
               })();
               Promise.resolve('micro').then(v => { document.title = v; });
               Promise.reject(new Error('perdu'));
             </script>",
        );
        p.run_scripts();
        // Les microtâches s'exécutent à la fin du script, avant tout minuteur.
        assert_eq!(body_text(&p), "début");
        assert_eq!(p.title(), "micro");
        let console = p.take_console();
        assert_eq!(console.len(), 1, "{console:?}");
        assert!(console[0].text.contains("Error: perdu"));
        // Le minuteur résout la promesse ; la suite de la fonction async
        // s'exécute dans la même tâche.
        assert!(p.tick(Instant::now() + std::time::Duration::from_secs(1)));
        assert_eq!(body_text(&p), "début, minuteur, 10+20");
        assert!(p.next_wake().is_none());
    }

    #[test]
    fn fetch_goes_through_the_embedder_and_the_same_origin_policy() {
        let mut p = page(
            "<p id=p></p><script>
               const sortie = document.getElementById('p');
               const noter = texte => { sortie.textContent += texte + ';'; };
               fetch('donnees.json')
                 .then(r => { noter(r.status + ' ' + r.ok + ' ' + r.headers.get('Content-Type') + ' ' + r.redirected); return r.json(); })
                 .then(d => noter('json ' + d.valeurs.join('+')));
               (async () => {
                 const r = await fetch('https://ailleurs.test/ouvert.txt');
                 noter('ouvert ' + r.type + ' ' + await r.text());
               })();
               fetch('https://ailleurs.test/ferme.txt').catch(e => noter('bloqué ' + e.name));
               fetch('/absent').then(r => noter('absent ' + r.status + ' ' + r.ok));
               fetch('/panne').catch(e => noter('panne ' + e.message));
               fetch('file:///C:/secret.txt').catch(() => noter('fichier refusé'));
               fetch('x', { method: 'TRACE' }).catch(() => noter('TRACE refusé'));
             </script>",
        );
        p.run_scripts();
        // Les refus immédiats n'atteignent jamais l'embarqueur. La méthode
        // est refusée la première : `Request` la contrôle avant tout envoi.
        assert_eq!(body_text(&p), "TRACE refusé;fichier refusé;");
        let requests = p.take_fetches();
        let urls: Vec<&str> = requests.iter().map(|r| r.url.as_str()).collect();
        assert_eq!(
            urls,
            [
                "https://exemple.test/dossier/donnees.json",
                "https://ailleurs.test/ouvert.txt",
                "https://ailleurs.test/ferme.txt",
                "https://exemple.test/absent",
                "https://exemple.test/panne",
            ]
        );
        assert!(
            p.take_fetches().is_empty(),
            "chaque requête n'est remise qu'une fois"
        );

        let respond =
            |request: &FetchRequest, status: u16, headers: &[(&str, &str)], body: &str| {
                Ok(FetchResponse {
                    url: request.url.clone(),
                    status,
                    headers: headers
                        .iter()
                        .map(|(n, v)| ((*n).to_owned(), (*v).to_owned()))
                        .collect(),
                    body: body.as_bytes().to_vec(),
                })
            };
        assert!(p.complete_fetch(
            requests[0].id,
            respond(
                &requests[0],
                200,
                &[("content-type", "application/json")],
                "{\"valeurs\":[1,2]}"
            ),
        ));
        assert!(p.complete_fetch(
            requests[1].id,
            respond(
                &requests[1],
                200,
                &[("access-control-allow-origin", "*")],
                "bonjour"
            ),
        ));
        // Autre origine sans autorisation : le script ne voit pas la réponse.
        assert!(p.complete_fetch(requests[2].id, respond(&requests[2], 200, &[], "secret")));
        assert!(p.complete_fetch(requests[3].id, respond(&requests[3], 404, &[], "")));
        assert!(p.complete_fetch(requests[4].id, Err("connexion impossible".to_owned())));
        // Identifiant inconnu (réponse tardive d'une autre page) : sans effet.
        assert!(!p.complete_fetch(requests[0].id, respond(&requests[0], 200, &[], "x")));
        assert!(!p.complete_fetch(u64::MAX, Err("perdu".to_owned())));
        assert_eq!(
            body_text(&p),
            "TRACE refusé;fichier refusé;200 true application/json false;json 1+2;ouvert cors bonjour;\
             bloqué TypeError;absent 404 false;panne fetch : connexion impossible;"
        );
        assert!(!body_text(&p).contains("secret"));
        assert!(p.take_console().is_empty());
    }

    #[test]
    fn module_scripts_import_their_dependencies() {
        let mut p = page(
            "<title>t</title><p id=p></p>
             <script type=module>
               import { saluer } from './lib/salut.js';
               import defaut, * as outils from '/outils.js';
               document.getElementById('p').textContent += saluer(outils.nom) + ' ' + defaut + ' ' + typeof prive + ';';
             </script>
             <script type=module src='entree.js'></script>
             <script type=module>import 'paquet-nu';</script>
             <script type=module>import './absent.js';</script>
             <script>var prive = 'global'; document.getElementById('p').textContent += 'classique;';</script>",
        );
        // Le module externe est demandé comme un script ordinaire.
        let scripts = p.pending_scripts();
        assert_eq!(scripts.len(), 1);
        assert!(
            p.set_script(
                &scripts[0],
                Some(
                    "import { saluer } from './lib/salut.js'; const prive = 1;
                 document.getElementById('p').textContent += saluer('entrée') + ';';"
                        .to_owned()
                )
            )
        );
        // Puis le graphe d'importations est fourni vague par vague.
        let mut asked = Vec::new();
        for _ in 0..8 {
            let pending = p.pending_modules();
            if pending.is_empty() {
                break;
            }
            for url in pending {
                let source = match url.path() {
                    "/dossier/lib/salut.js" => Some(
                        "import { ponctuation } from './ponctuation.js';
                         export const saluer = nom => 'bonjour ' + nom + ponctuation;",
                    ),
                    "/dossier/lib/ponctuation.js" => Some("export const ponctuation = ' !';"),
                    "/outils.js" => Some("export const nom = 'outils'; export default 42;"),
                    _ => None,
                };
                asked.push(url.path().to_owned());
                p.set_module(&url, source.map(str::to_owned));
            }
        }
        asked.sort();
        assert_eq!(
            asked,
            [
                "/dossier/absent.js",
                "/dossier/lib/ponctuation.js",
                "/dossier/lib/salut.js",
                "/outils.js"
            ]
        );
        p.run_scripts();
        // Classiques d'abord, puis modules dans l'ordre ; chaque module a sa
        // portée, mais voit les globales des scripts classiques.
        assert_eq!(
            body_text(&p),
            "classique;bonjour outils ! 42 string;bonjour entrée !;"
        );
        let console = p.take_console();
        assert_eq!(console.len(), 2, "{console:?}");
        assert!(console[0].text.contains("paquet-nu"), "{}", console[0].text);
        assert!(console[1].text.contains("absent.js"), "{}", console[1].text);
    }

    #[test]
    fn generated_bindings_are_up_to_date() {
        // Le fichier versionné doit être exactement ce que le générateur
        // produit à partir des fichiers IDL.
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("idl");
        let mut paths: Vec<_> = std::fs::read_dir(&directory)
            .expect("dossier idl")
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|path| path.extension().is_some_and(|e| e == "webidl"))
            .collect();
        paths.sort();
        let mut definitions = Vec::new();
        for path in &paths {
            let source = std::fs::read_to_string(path).expect("fichier IDL lisible");
            definitions.extend(oxyde_bindings::parse(&source).expect("IDL valide"));
        }
        let interfaces = oxyde_bindings::resolve(&definitions).expect("IDL cohérent");
        let expected = oxyde_bindings::generate(&interfaces);
        let actual = include_str!("generated/dom.rs").replace("\r\n", "\n");
        assert!(
            actual == expected,
            "src/generated/dom.rs est périmé : relancer \
             `cargo run -p oxyde_bindings -- crates/oxyde_content/idl crates/oxyde_content/src/generated/dom.rs`"
        );
    }

    #[test]
    fn dom_interfaces_come_from_the_idl() {
        let mut q = page(
            "<title>t</title><p id=p>texte<!--c--></p><svg id=s></svg><script>
               const el = document.getElementById('p'), texte = el.firstChild, svg = document.getElementById('s');
               document.title = [
                 el instanceof HTMLElement, el instanceof Node, el instanceof EventTarget,
                 texte instanceof Text, texte instanceof CharacterData, texte instanceof Element,
                 el.lastChild instanceof Comment, document instanceof Document, document instanceof Element,
                 svg instanceof Element, svg instanceof HTMLElement,
                 Object.getPrototypeOf(HTMLElement) === Element, Object.getPrototypeOf(Element.prototype) === Node.prototype,
                 Node.ELEMENT_NODE, el.TEXT_NODE, Node.DOCUMENT_POSITION_CONTAINED_BY,
                 Object.prototype.toString.call(el), Object.prototype.toString.call(texte),
                 Node.prototype.appendChild.length, Element.prototype.setAttribute.length, Node.prototype.cloneNode.length,
                 typeof texte.getAttribute, typeof texte.remove, typeof el.click, typeof svg.click,
                 typeof document.getElementById, typeof el.getElementById, typeof document.createElement,
                 Object.hasOwn(Node.prototype, 'appendChild'), Object.hasOwn(Element.prototype, 'appendChild'),
               ].join(' ');
               try { new Node(); } catch (e) { document.title += ' ' + e.name; }
               try { Element(); } catch (e) { document.title += ' ' + e.name; }
             </script>",
        );
        q.run_scripts();
        assert_eq!(q.take_console(), Vec::new());
        assert_eq!(
            q.title(),
            "true true true true true false true true false true false true true 1 3 16 \
             [object HTMLParagraphElement] [object Text] 1 2 0 \
             undefined function function undefined function undefined function true false \
             TypeError TypeError"
        );
    }

    #[test]
    fn dom_coverage_is_measured() {
        let report = dom_coverage_report();
        assert!(
            report.contains("| `Node` | `EventTarget` | 18 / 18 |"),
            "{report}"
        );
        assert!(report.contains("## Membres absents"));
        // La couverture ne doit pas régresser : 125 membres au moment où le
        // rapport a été branché.
        let done: usize = report
            .split("**Total : ")
            .nth(1)
            .and_then(|rest| rest.split(' ').next())
            .and_then(|n| n.parse().ok())
            .expect("total lisible");
        assert!(done >= 310, "couverture en régression : {done} membres");
    }

    #[test]
    fn script_errors_do_not_break_the_page() {
        let mut p = page(
            "<p id=a>a</p>
             <script>document.getElementById('absent').textContent = 'x';</script>
             <script>ceci n'est pas du javascript</script>
             <script>while (true) {}</script>
             <script>document.getElementById('a').textContent = 'suite';</script>",
        );
        p.run_scripts();
        assert_eq!(
            body_text(&p),
            "suite",
            "les scripts suivants s'exécutent malgré les erreurs"
        );
        let console = p.take_console();
        assert_eq!(console.len(), 3, "{console:?}");
        assert!(console[0].text.contains("TypeError"));
        assert!(console[1].text.contains("SyntaxError"));
        assert!(console[2].text.contains("temps d'exécution dépassé"));
    }

    #[test]
    fn events_follow_capture_target_and_bubble_phases() {
        let mut p = page(
            "<div id=outer><p id=inner>x</p></div>
             <script>
               const log = [];
               const outer = document.getElementById('outer');
               const inner = document.getElementById('inner');
               outer.addEventListener('ping', e => log.push('capture ' + e.eventPhase), true);
               outer.addEventListener('ping', e => log.push('bulle ' + e.eventPhase));
               inner.addEventListener('ping', e => {
                 log.push('cible ' + e.eventPhase + ' ' + (e.target === inner));
               });
               inner.addEventListener('ping', { handleEvent(e) { log.push('objet ' + e.detail.n); } });
               inner.addEventListener('ping', () => log.push('une fois'), { once: true });
               inner.addEventListener('ping', e => e.preventDefault());
               const event = new CustomEvent('ping', {
                 bubbles: true, cancelable: true, detail: { n: 7 },
               });
               log.push('retour ' + inner.dispatchEvent(event));
               inner.dispatchEvent(new CustomEvent('ping', { detail: { n: 8 } }));
               log.push(event instanceof Event, event.isTrusted, event.eventPhase);
               document.body.textContent = log.join('|');
             </script>",
        );
        p.run_scripts();
        assert_eq!(
            body_text(&p),
            concat!(
                "capture 1|cible 2 true|objet 7|une fois|bulle 3|retour false|",
                "capture 1|cible 2 true|objet 8|true|false|0"
            )
        );
    }

    #[test]
    fn fetch_sends_method_headers_and_body_under_cors_rules() {
        let mut p = page(
            "<script>
               fetch('/api', {
                 method: 'post',
                 headers: { 'X-Jeton': ' 1 ', Cookie: 'a=b', 'Content-Type': 'application/json' },
                 body: '{}',
               });
               fetch('https://ailleurs.test/simple', { method: 'POST', body: 'texte' });
               fetch('https://ailleurs.test/riche', { method: 'PUT', headers: [['X-A', '1']] });
               fetch('/lecture', { body: 'x' }).catch(e => { document.body.textContent = e.message; });
             </script>",
        );
        p.run_scripts();
        // Le refus vient du constructeur `Request`, que `fetch` appelle.
        assert_eq!(body_text(&p), "une requête GET ne peut pas avoir de corps");
        let requests = p.take_fetches();
        assert_eq!(requests.len(), 3);
        let pair = |n: &str, v: &str| (n.to_owned(), v.to_owned());
        // Même origine : tout est permis, sans pré-vérification.
        assert_eq!(requests[0].method, "POST");
        assert_eq!(
            requests[0].headers,
            [
                pair("content-type", "application/json"),
                pair("x-jeton", "1")
            ]
        );
        assert_eq!(requests[0].body, b"{}");
        assert_eq!(requests[0].origin.as_deref(), Some("https://exemple.test"));
        assert!(!requests[0].needs_preflight);
        // Autre origine, requête simple : type de contenu par défaut.
        assert_eq!(
            requests[1].headers,
            [pair("content-type", "text/plain;charset=UTF-8")]
        );
        assert!(!requests[1].needs_preflight);
        // Autre origine, méthode et en-tête hors liste : pré-vérification.
        assert!(requests[2].needs_preflight);
        assert_eq!(
            requests[2].preflight_headers(),
            [
                pair("Access-Control-Request-Method", "PUT"),
                pair("Access-Control-Request-Headers", "x-a"),
                pair("Origin", "https://exemple.test"),
            ]
        );
    }

    #[test]
    fn document_cookie_goes_through_the_embedder() {
        let mut p = page(
            "<script>
               document.cookie = 'a=1; Path=/';
               document.cookie = 'b=2';
               document.cookie = 'a=; Max-Age=0';
               document.cookie = 'secret=1; HttpOnly';
               document.body.textContent = document.cookie;
             </script>",
        );
        p.set_cookies("z=9");
        p.run_scripts();
        // La page voit aussitôt ses propres écritures…
        assert_eq!(body_text(&p), "z=9; b=2");
        // … que l'embarqueur soumet à son pot, dans l'ordre.
        assert_eq!(
            p.take_cookie_writes(),
            ["a=1; Path=/", "b=2", "a=; Max-Age=0", "secret=1; HttpOnly"]
        );
        assert!(p.take_cookie_writes().is_empty());
    }

    #[test]
    fn blobs_requests_and_responses_are_constructible() {
        let mut p = page(
            "<script>
               const out = [];
               const blob = new Blob(['bon', 'jour'], { type: 'Text/Plain' });
               out.push(blob.size, blob.type, blob.slice(3).size);
               const request = new Request('/a?b=1', { method: 'post', body: 'x' });
               out.push(request.method, request.url.endsWith('/a?b=1'),
                        request.headers.get('content-type'));
               const response = new Response('{\"n\":2}', { status: 201, headers: { 'X-A': '1' } });
               out.push(response.status, response.ok, response.headers.get('x-a'));
               let refused = '';
               try { new Response('x', { status: 204 }); } catch (e) { refused = e.name; }
               Promise.all([blob.text(), response.json(), Response.json([1]).text()])
                 .then(([text, json, serialized]) => {
                   out.push(text, json.n, serialized, response.bodyUsed, refused);
                   document.body.textContent = out.join('|');
                 });
             </script>",
        );
        p.run_scripts();
        assert_eq!(
            body_text(&p),
            "7|text/plain|4|POST|true|text/plain;charset=UTF-8|201|true|1|bonjour|2|[1]|true|TypeError"
        );
    }

    #[test]
    fn xml_http_request_runs_over_fetch() {
        let mut p = page(
            "<script>
               const log = [];
               const xhr = new XMLHttpRequest();
               xhr.onreadystatechange = () => log.push('état ' + xhr.readyState);
               xhr.addEventListener('load', () => {
                 log.push(xhr.status, xhr.responseText, xhr.getResponseHeader('X-Essai'));
               });
               xhr.onloadend = () => { document.body.textContent = log.join('|'); };
               xhr.open('POST', '/depot');
               xhr.setRequestHeader('X-Jeton', 'abc');
               xhr.send('corps');
               log.push(xhr instanceof EventTarget, XMLHttpRequest.DONE);
             </script>",
        );
        p.run_scripts();
        let requests = p.take_fetches();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "POST");
        assert_eq!(requests[0].body, b"corps");
        assert!(
            requests[0]
                .headers
                .contains(&("x-jeton".to_owned(), "abc".to_owned()))
        );
        assert!(p.complete_fetch(
            requests[0].id,
            Ok(FetchResponse {
                url: requests[0].url.clone(),
                status: 201,
                headers: vec![("x-essai".to_owned(), "oui".to_owned())],
                body: b"fait".to_vec(),
            }),
        ));
        assert_eq!(
            body_text(&p),
            "état 1|true|4|état 2|état 3|état 4|201|fait|oui"
        );
    }

    #[test]
    fn url_search_params_and_headers() {
        let mut p = page(
            "<script>
               const log = [];
               const url = new URL('../a b/c?x=1&y=deux mots#h', 'https://Exemple.test:8443/d/e/');
               log.push(url.href, url.origin, url.pathname, url.searchParams.get('y'));
               url.searchParams.append('z', 'é&=');
               url.port = '443';
               url.hash = '';
               log.push(url.search, url.host, String(url), URL.canParse('pas une url'));
               const params = new URLSearchParams({ a: '1', b: 'x y' });
               params.set('a', '2');
               log.push(params.toString(), [...params.keys()].join(','), params.size);
               const headers = new Headers([['X-B', '1'], ['x-a', ' 2 ']]);
               headers.append('X-B', '3');
               log.push([...headers].join(';'), headers.get('x-b'), headers.has('X-C'));
               try { new URL('::'); } catch (e) { log.push(e.name); }
               fetch('/form', { method: 'POST', headers, body: params });
               document.body.textContent = log.join('|');
             </script>",
        );
        p.run_scripts();
        assert_eq!(
            body_text(&p),
            concat!(
                "https://exemple.test:8443/d/a%20b/c?x=1&y=deux%20mots#h|",
                "https://exemple.test:8443|/d/a%20b/c|deux mots|",
                "?x=1&y=deux+mots&z=%C3%A9%26%3D|exemple.test|",
                "https://exemple.test/d/a%20b/c?x=1&y=deux+mots&z=%C3%A9%26%3D|false|",
                "a=2&b=x+y|a,b|2|x-a,2;x-b,1, 3|1, 3|false|TypeError"
            )
        );
        let requests = p.take_fetches();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].body, b"a=2&b=x+y");
        let pair = |n: &str, v: &str| (n.to_owned(), v.to_owned());
        assert_eq!(
            requests[0].headers,
            // `Headers` rend ses noms en minuscules, triés.
            [
                pair(
                    "content-type",
                    "application/x-www-form-urlencoded;charset=UTF-8"
                ),
                pair("x-a", "2"),
                pair("x-b", "1, 3"),
            ]
        );
    }

    #[test]
    fn prelude_web_apis() {
        let mut p = page(
            "<script>
               const log = [];
               log.push(btoa('Oxyde!'), atob('T3h5ZGUh'), atob(' T3g= '));
               const bytes = new TextEncoder().encode('é€😀');
               log.push(bytes.length, new TextDecoder().decode(bytes));
               log.push(new TextDecoder().decode(new Uint8Array([0x41, 0xff, 0xe2, 0x82])));
               const controller = new AbortController();
               controller.signal.addEventListener('abort', () => log.push('abandon'));
               controller.abort();
               log.push(controller.signal.aborted, controller.signal.reason.name);
               const source = { date: new Date(5), liste: [1, { a: new Map([[1, 2]]) }] };
               source.soi = source;
               const copy = structuredClone(source);
               log.push(copy !== source, copy.soi === copy, copy.date.getTime());
               log.push(copy.liste[1].a.get(1));
               const key = new KeyboardEvent('keydown', { key: 'a', ctrlKey: 1, bubbles: true });
               log.push(key.key, key.ctrlKey, key.bubbles, key instanceof UIEvent);
               const mouse = new MouseEvent('click', { clientX: 4 });
               log.push(mouse.pageX, mouse.button, mouse instanceof Event);
               const legacy = document.createEvent('Event');
               legacy.initEvent('vieux', true, false);
               log.push(legacy.type, legacy.bubbles);
               log.push(matchMedia('(min-width: 0px) and (max-width: 5000px)').matches);
               log.push(matchMedia('print, (prefers-color-scheme: dark)').matches);
               log.push(new XMLSerializer().serializeToString(document.createElement('br')));
               log.push(new Image(3).getAttribute('width'), typeof performance.mark('m').startTime);
               try { atob('@'); } catch (e) { log.push(e.name, e.code, e instanceof DOMException); }
               queueMicrotask(() => { document.body.textContent = log.join('|'); });
             </script>",
        );
        p.run_scripts();
        assert_eq!(
            body_text(&p),
            concat!(
                "T3h5ZGUh|Oxyde!|Ox|9|é€😀|A\u{fffd}\u{fffd}|abandon|true|AbortError|",
                "true|true|5|2|a|true|true|true|4|0|true|vieux|true|true|false|<br>|3|number|",
                "InvalidCharacterError|5|true"
            )
        );
    }

    #[test]
    fn mutation_observers_receive_records_at_the_end_of_the_task() {
        let mut p = page(
            "<div id=root><p id=a class=x>un</p></div><pre id=out></pre>
             <script>
               const log = [];
               const root = document.getElementById('root');
               const a = document.getElementById('a');
               const describe = r => [
                 r.type, r.target.id || r.target.nodeName, r.attributeName, r.oldValue,
                 r.addedNodes.length, r.removedNodes.length,
               ].join(' ');
               const observer = new MutationObserver((records, self) => {
                 log.push('lot ' + records.length + ' ' + (self === observer));
                 for (const r of records) log.push(describe(r));
                 document.getElementById('out').textContent = log.join('|');
               });
               observer.observe(root, {
                 subtree: true, childList: true, attributeOldValue: true,
                 characterDataOldValue: true, attributeFilter: ['class'],
               });
               a.className = 'y';
               a.setAttribute('title', 'ignoré');
               a.firstChild.data = 'deux';
               root.appendChild(document.createElement('b'));
               a.remove();
               log.push('avant ' + observer.takeRecords().length);
               a.className = 'hors du sous-arbre';
               root.setAttribute('class', 'z');
               const silent = new MutationObserver(() => log.push('jamais'));
               silent.observe(root, { attributes: true });
               silent.disconnect();
               root.removeAttribute('class');
               try { observer.observe(root, {}); } catch (e) { log.push(e.name); }
             </script>",
        );
        p.run_scripts();
        let out = p
            .document()
            .descendants(p.document().root())
            .find(|n| {
                p.document()
                    .element(*n)
                    .is_some_and(|e| e.attr("id") == Some("out"))
            })
            .map(|n| p.document().text_content(n))
            .unwrap_or_default();
        assert_eq!(
            out,
            concat!(
                "avant 4|TypeError|lot 3 true|",
                "attributes a class y 0 0|",
                "attributes root class  0 0|attributes root class z 0 0"
            )
        );
    }

    #[test]
    fn location_and_history_ask_the_embedder_to_navigate() {
        let mut p = page(
            "<script>
               const log = [];
               history.pushState({ n: 1 }, '', 'page2?x=1');
               log.push(location.pathname + location.search, history.length, history.state.n);
               history.replaceState(null, '', '#frag');
               location.hash = 'autre';
               log.push(document.URL === location.href, location.hash);
               addEventListener('popstate', e => {
                 log.push('pop ' + JSON.stringify(e.state) + ' ' + location.hash);
               });
               addEventListener('hashchange', e => {
                 log.push('fragment ' + new URL(e.newURL).hash);
                 document.body.textContent = log.join('|');
               });
               try { history.pushState(null, '', 'https://ailleurs.test/'); } catch (e) { log.push(e.name); }
               location.assign('/suite');
               location.replace('fin');
               location.reload();
               history.back();
               document.body.textContent = log.join('|');
             </script>",
        );
        p.run_scripts();
        let at = |path: &str| Url::parse(&format!("https://exemple.test{path}")).expect("url");
        assert_eq!(
            p.take_navigations(),
            [
                Navigation::SameDocument {
                    url: at("/dossier/page2?x=1"),
                    replace: false
                },
                Navigation::SameDocument {
                    url: at("/dossier/page2?x=1#frag"),
                    replace: true
                },
                Navigation::SameDocument {
                    url: at("/dossier/page2?x=1#autre"),
                    replace: false
                },
                Navigation::Load {
                    url: at("/suite"),
                    replace: false,
                    post: None
                },
                Navigation::Load {
                    url: at("/dossier/fin"),
                    replace: true,
                    post: None
                },
                Navigation::Reload,
                Navigation::Traverse(-1),
            ]
        );
        assert_eq!(p.url(), &at("/dossier/page2?x=1#autre"));
        assert_eq!(
            body_text(&p),
            "/dossier/page2?x=1|2|1|true|#autre|SecurityError"
        );
        // L'embarqueur revient d'une entrée, dans le même document.
        let redraw = p.traverse(&at("/dossier/page2?x=1#frag"), -1);
        assert!(redraw, "{:?}", p.take_console());
        assert_eq!(p.url(), &at("/dossier/page2?x=1#frag"));
        assert_eq!(
            body_text(&p),
            "/dossier/page2?x=1|2|1|true|#autre|SecurityError|pop null #frag|fragment #frag"
        );
    }

    #[test]
    fn keyboard_events_reach_the_page() {
        let mut p = page(
            "<script>
               const log = [];
               for (const type of ['keydown', 'keypress', 'keyup']) {
                 document.addEventListener(type, e => {
                   log.push(type + ' ' + e.key + ' ' + e.code + ' ' + e.keyCode + ' ' + e.ctrlKey);
                   if (e.key === 'Enter') e.preventDefault();
                   document.body.textContent = log.join('|');
                 });
               }
             </script>",
        );
        p.run_scripts();
        let key = |key: &'static str, code: &'static str, ctrl: bool| KeyInput {
            key,
            code,
            ctrl,
            shift: false,
            alt: false,
            meta: false,
            repeat: false,
        };
        assert_eq!(p.key_event(true, &key("a", "KeyA", false)), (false, true));
        assert_eq!(p.key_event(false, &key("a", "KeyA", false)), (false, true));
        assert_eq!(
            p.key_event(true, &key("Enter", "Enter", true)),
            (true, true)
        );
        assert_eq!(
            body_text(&p),
            concat!(
                "keydown a KeyA 65 false|keypress a KeyA 97 false|keyup a KeyA 65 false|",
                "keydown Enter Enter 13 true"
            )
        );
    }

    #[test]
    fn typed_elements_and_form_submission() {
        let mut p = page(
            "<form id=f action='/recherche' method=get>
               <input name=q value='deux mots'>
               <input type=checkbox name=c id=c value=oui>
               <input type=radio name=r value=1 checked><input type=radio name=r value=2 id=r2>
               <select name=s><option>un<option value=2 selected>deux</select>
               <textarea name=t>texte</textarea>
               <input name=ignore disabled value=x>
               <button id=b name=bouton value=ok>Envoyer</button>
             </form>
             <form id=g action='/depot' method=post><input name=a value=1></form>
             <a id=lien href='page?x=1#h'>lien</a><pre id=out></pre>
             <script>
               const log = [];
               const $ = id => document.getElementById(id);
               const form = $('f');
               log.push(form instanceof HTMLFormElement, form instanceof HTMLElement);
               log.push(form.elements.length, form.method, form.action);
               log.push($('lien').pathname + $('lien').search, $('lien') instanceof HTMLAnchorElement);
               form.querySelector('select').value = 'un';
               log.push(form.querySelector('select').selectedIndex, form.querySelector('textarea').value);
               log.push(new URLSearchParams([...new FormData(form)]).toString());
               $('c').addEventListener('change', () => log.push('change ' + $('c').checked));
               form.addEventListener('submit', e => {
                 log.push('envoi ' + e.submitter.id);
                 $('out').textContent = log.join('|');
               });
             </script>",
        );
        p.run_scripts();
        let by_id = |p: &Page, id: &str| {
            let doc = p.document();
            doc.descendants(doc.root())
                .find(|n| doc.element(*n).is_some_and(|e| e.attr("id") == Some(id)))
                .expect("élément")
        };
        for id in ["c", "r2", "b"] {
            let node = by_id(&p, id);
            p.click_node(node);
        }
        let doc = p.document();
        let out = doc
            .descendants(doc.root())
            .find(|n| doc.element(*n).is_some_and(|e| e.attr("id") == Some("out")))
            .map(|n| doc.text_content(n))
            .unwrap_or_default();
        assert_eq!(
            out,
            concat!(
                "true|true|8|get|https://exemple.test/recherche|/dossier/page?x=1|true|0|texte|",
                "q=deux+mots&r=1&s=un&t=texte|change true|envoi b"
            )
        );
        let at = |path: &str| Url::parse(&format!("https://exemple.test{path}")).expect("url");
        assert_eq!(
            p.take_navigations(),
            [Navigation::Load {
                url: at("/recherche?q=deux+mots&c=oui&r=2&s=un&t=texte&bouton=ok"),
                replace: false,
                post: None,
            }]
        );
        // Soumission par POST : les données partent dans le corps.
        let mut p = page(
            "<form id=g action='/depot' method=post><input name=a value='1 2'></form>
             <script>document.getElementById('g').submit();</script>",
        );
        p.run_scripts();
        assert_eq!(
            p.take_navigations(),
            [Navigation::Load {
                url: at("/depot"),
                replace: false,
                post: Some(FormPost {
                    content_type: "application/x-www-form-urlencoded".to_owned(),
                    body: b"a=1+2".to_vec(),
                }),
            }]
        );
    }

    #[test]
    fn storage_fragments_collections_and_dataset() {
        let mut p = page(
            "<p id=a data-user-id=7 name=n>x</p><p>y</p>
             <script>
               const log = [];
               localStorage.setItem('k', 1);
               localStorage.theme = 'sombre';
               log.push(localStorage.getItem('k'), localStorage.theme, localStorage.length);
               log.push(localStorage.key(1), localStorage.getItem('absent'));
               localStorage.removeItem('k');
               sessionStorage.setItem('s', 'v');
               log.push(localStorage.length, sessionStorage.length);
               const fragment = document.createDocumentFragment();
               fragment.append(document.createElement('b'), 'texte');
               log.push(fragment.nodeType, fragment.childNodes.length);
               const host = document.createElement('div');
               host.appendChild(fragment);
               log.push(host.innerHTML, fragment.childNodes.length);
               const paragraphs = document.getElementsByTagName('p');
               log.push(paragraphs.item(1).textContent, paragraphs.namedItem('n').id);
               log.push(document.querySelectorAll('p').item(5));
               const a = document.getElementById('a');
               a.dataset.fooBar = 'z';
               log.push(a.dataset.userId, a.getAttribute('data-foo-bar'));
               document.body.textContent = log.join('|');
             </script>",
        );
        p.run_scripts();
        assert_eq!(
            body_text(&p),
            "1|sombre|2|theme||1|1|11|2|<b></b>texte|0|y|a||7|z"
        );
    }

    #[test]
    fn dom_tree_operations() {
        let mut p = page(
            "<script>
               const log = [];
               const d = document.createElement('div');
               d.innerHTML = '<b>a</b>c';
               d.firstChild.after('x', document.createElement('i'));
               d.lastChild.before(document.createComment('k'));
               d.querySelector('i').replaceWith('y');
               d.normalize();
               const text = d.childNodes[1];
               text.insertData(1, '-');
               const tail = text.splitText(2);
               log.push(d.innerHTML, d.childNodes.length, tail.wholeText);
               log.push(d.firstChild.toggleAttribute('hidden'), d.firstChild.getAttributeNames());
               d.insertAdjacentHTML('afterbegin', '<u>z</u>');
               log.push(d.firstChild.outerHTML, d.compareDocumentPosition(d.firstChild));
               log.push(d.firstChild.isEqualNode(d.firstChild.cloneNode(true)));
               d.replaceChildren('fin');
               log.push(d.textContent);
               document.body.textContent = log.join('|');
             </script>",
        );
        p.run_scripts();
        assert_eq!(
            body_text(&p),
            "<b>a</b>x-y<!--k-->c|5|x-y|true|hidden|<u>z</u>|20|true|fin"
        );
    }

    #[test]
    fn find_highlights_steps_and_scrolls() {
        let mut text = TextContext::new();
        let mut page = page(
            "<body style='margin:0'><p>Premier MOT</p><div style='height:2000px'></div>\
             <p>second mot, troisième <b>mot</b></p></body>",
        );
        // Avant la mise en page, rien n'est trouvé ; la mise en page rattrape.
        assert_eq!(page.find("mot"), FindStatus::default());
        page.update(&mut text, VIEWPORT);
        assert_eq!(
            page.find_status(),
            FindStatus {
                count: 3,
                current: 1
            }
        );
        let painted = page.display_list().items.len();
        let highlights = |page: &Page| {
            page.display_list()
                .items
                .iter()
                .filter(|item| {
                    matches!(item, DisplayItem::Rect { color, .. }
                        if *color == FIND_HIGHLIGHT || *color == FIND_CURRENT)
                })
                .count()
        };
        assert_eq!(highlights(&page), 3);
        assert_eq!(page.scroll_offset(), (0.0, 0.0));

        // L'occurrence suivante est plus bas : la page défile jusqu'à elle.
        assert_eq!(
            page.find_step(true),
            FindStatus {
                count: 3,
                current: 2
            }
        );
        assert!(page.scroll_offset().1 > 1000.0);
        assert_eq!(page.find_step(true).current, 3);
        // La recherche boucle, dans les deux sens.
        assert_eq!(page.find_step(true).current, 1);
        assert_eq!(page.scroll_offset().1, 0.0);
        assert_eq!(page.find_step(false).current, 3);

        // Une autre recherche repart de ce qui est à l'écran.
        assert_eq!(
            page.find("troisième"),
            FindStatus {
                count: 1,
                current: 1
            }
        );
        assert_eq!(highlights(&page), 1);
        assert_eq!(page.find("absent"), FindStatus::default());
        assert_eq!(highlights(&page), 0);
        // Effacer la recherche rend la liste d'affichage telle qu'elle était.
        page.find("mot");
        page.find("");
        assert_eq!(page.find_status(), FindStatus::default());
        assert_eq!(page.display_list().items.len(), painted - 3);
    }

    fn node_by_id(p: &Page, id: &str) -> NodeId {
        let doc = p.document();
        doc.descendants(doc.root())
            .find(|n| doc.element(*n).is_some_and(|e| e.attr("id") == Some(id)))
            .expect("élément")
    }

    fn press(p: &mut Page, key: &'static str) {
        let input = KeyInput {
            key,
            code: "",
            ctrl: false,
            shift: false,
            alt: false,
            meta: false,
            repeat: false,
        };
        p.key_event(true, &input);
        p.key_event(false, &input);
    }

    fn attribute(p: &Page, id: &str, name: &str) -> Option<String> {
        let node = node_by_id(p, id);
        p.document()
            .element(node)
            .and_then(|e| e.attr(name).map(str::to_owned))
    }

    #[test]
    fn typing_in_a_focused_field() {
        let mut p = page(
            "<form id=f action='/ok'>
               <input id=nom value='Ada'>
               <input id=court maxlength=2>
               <input id=fige value='x' readonly>
               <input id=case type=checkbox>
               <textarea id=zone>a</textarea>
               <select id=liste><option>un<option>deux<option>trois</select>
               <input id=envoi type=submit>
             </form>
             <p id=journal></p>
             <script>
               const log = [];
               for (const type of ['focus', 'blur', 'input', 'change', 'submit', 'focusin']) {
                 document.addEventListener(type, e => {
                   log.push(type + ':' + e.target.id);
                   if (type === 'submit') e.preventDefault();
                   document.getElementById('journal').textContent =
                     log.join(' ') + ' @' + document.activeElement.id;
                 }, true);
               }
             </script>",
        );
        p.run_scripts();
        let journal = |p: &Page| {
            let node = node_by_id(p, "journal");
            p.document().text_content(node)
        };

        // Le clic donne le focus ; le curseur est en fin de valeur.
        let nom = node_by_id(&p, "nom");
        p.click_node(nom);
        assert_eq!(p.document().focus(), Some(nom));
        assert_eq!(journal(&p), "focus:nom focusin:nom @nom");
        press(&mut p, "!");
        press(&mut p, "ArrowLeft");
        press(&mut p, "Backspace");
        press(&mut p, "é");
        press(&mut p, "Home");
        press(&mut p, "Delete");
        press(&mut p, "End");
        press(&mut p, "?");
        assert_eq!(attribute(&p, "nom", "value").as_deref(), Some("dé!?"));
        assert_eq!(p.document().caret(), "dé!?".len());

        // Tab passe au champ suivant : `change` puis `blur` sur l'ancien.
        press(&mut p, "Tab");
        assert_eq!(p.document().focus(), Some(node_by_id(&p, "court")));
        assert!(
            journal(&p).ends_with("change:nom blur:nom focus:court focusin:court @court"),
            "{}",
            journal(&p)
        );
        // `maxlength` borne la frappe ; un champ en lecture seule l'ignore.
        for key in ["a", "b", "c"] {
            press(&mut p, key);
        }
        assert_eq!(attribute(&p, "court", "value").as_deref(), Some("ab"));
        press(&mut p, "Tab");
        press(&mut p, "z");
        assert_eq!(attribute(&p, "fige", "value").as_deref(), Some("x"));

        // Espace coche la case ; la zone de texte reçoit la frappe en fin.
        press(&mut p, "Tab");
        press(&mut p, " ");
        assert_eq!(attribute(&p, "case", "checked").as_deref(), Some(""));
        press(&mut p, "Tab");
        press(&mut p, "b");
        press(&mut p, "Enter");
        let zone = node_by_id(&p, "zone");
        assert_eq!(p.document().text_content(zone), "ab\n");

        // Les flèches changent l'option d'une liste déroulante, sans en sortir.
        press(&mut p, "Tab");
        press(&mut p, "ArrowDown");
        press(&mut p, "ArrowDown");
        press(&mut p, "ArrowDown");
        press(&mut p, "ArrowUp");
        let chosen: Vec<String> = {
            let doc = p.document();
            doc.descendants(node_by_id(&p, "liste"))
                .filter(|n| doc.element(*n).is_some_and(|e| e.has_attr("selected")))
                .map(|n| doc.text_content(n))
                .collect()
        };
        assert_eq!(chosen, ["deux"]);
        assert!(journal(&p).contains("change:liste"), "{}", journal(&p));

        // Entrée dans un champ de texte envoie le formulaire.
        p.click_node(nom);
        press(&mut p, "Enter");
        assert!(journal(&p).contains("submit:f"), "{}", journal(&p));

        // `focus()` et `blur()` depuis un script ; cliquer ailleurs retire le focus.
        let journal_node = node_by_id(&p, "journal");
        p.click_node(journal_node);
        assert_eq!(p.document().focus(), None);
        assert!(journal(&p).ends_with("@"), "{}", journal(&p));
    }

    #[test]
    fn focus_changes_the_style_and_draws_a_caret() {
        let mut text = TextContext::new();
        let mut p = page(
            "<style>input { border: 1px solid rgb(1, 2, 3) } input:focus { border-color: rgb(9, 8, 7) }
                    label { color: rgb(0, 0, 0) } :checked + label { color: rgb(200, 0, 0) }</style>
             <input id=a value='abc'><input id=c type=checkbox><label id=l for=c>case</label>",
        );
        p.run_scripts();
        p.update(&mut text, VIEWPORT);
        let border = |p: &Page, wanted: Color| {
            p.display_list().items.iter().any(|item| match item {
                DisplayItem::Rect { color, .. } | DisplayItem::StrokeRect { color, .. } => {
                    *color == wanted
                }
                _ => false,
            })
        };
        let rects = |p: &Page| {
            p.display_list()
                .items
                .iter()
                .filter(|item| matches!(item, DisplayItem::Rect { .. }))
                .count()
        };
        let focused = Color::rgb(9, 8, 7);
        assert!(!border(&p, focused));
        let before = rects(&p);

        let field = node_by_id(&p, "a");
        p.click_node(field);
        assert!(
            p.update(&mut text, VIEWPORT),
            "le focus redemande une mise en page"
        );
        assert!(border(&p, focused), "`:focus` s'applique");
        assert_eq!(rects(&p), before + 1, "le curseur est un rectangle de plus");

        // Cliquer sur l'étiquette coche sa case, et `:checked` suit.
        let label = node_by_id(&p, "l");
        p.click_node(label);
        assert_eq!(attribute(&p, "c", "checked").as_deref(), Some(""));
        assert_eq!(p.document().focus(), Some(node_by_id(&p, "c")));
    }

    #[test]
    fn scripts_inserted_by_scripts_run() {
        let mut p = page(
            "<p id=p></p><script id=premier>
               const sortie = document.getElementById('p');
               const noter = texte => { sortie.textContent += texte + ';'; };
               noter('courant ' + document.currentScript.id);
               const enLigne = document.createElement('script');
               enLigne.id = 'enligne';
               enLigne.textContent = \"noter('en ligne ' + document.currentScript.id);\";
               document.head.appendChild(enLigne);
               noter('après insertion');
               for (const [id, src] of [['externe', 'lot.js'], ['ailleurs', 'https://ailleurs.test/x.js'], ['absent', '/absent.js']]) {
                 const script = document.createElement('script');
                 script.id = id;
                 script.src = src;
                 script.onload = () => noter('load ' + id + ' ' + (document.currentScript === null));
                 script.onerror = () => noter('error ' + id);
                 document.body.appendChild(script);
               }
               const inerte = document.createElement('script');
               inerte.type = 'application/json';
               inerte.textContent = '{';
               document.body.appendChild(inerte);
               const repli = document.createElement('script');
               repli.noModule = true;
               repli.setAttribute('nomodule', '');
               repli.textContent = \"noter('repli');\";
               document.body.appendChild(repli);
             </script>
             <script nomodule>noter('repli statique');</script>",
        );
        p.run_scripts();
        // Le script en ligne s'exécute dès son insertion, avant la suite du
        // script qui l'a inséré.
        assert_eq!(
            body_text(&p),
            "courant premier;en ligne enligne;après insertion;"
        );
        let requests = p.take_fetches();
        let urls: Vec<&str> = requests.iter().map(|r| r.url.as_str()).collect();
        assert_eq!(
            urls,
            [
                "https://exemple.test/dossier/lot.js",
                "https://ailleurs.test/x.js",
                "https://exemple.test/absent.js",
            ]
        );
        let respond = |request: &FetchRequest, status: u16, body: &str| {
            Ok(FetchResponse {
                url: request.url.clone(),
                status,
                headers: Vec::new(),
                body: body.as_bytes().to_vec(),
            })
        };
        // Une autre origine sans en-tête CORS : un script se charge quand même.
        assert!(p.complete_fetch(
            requests[1].id,
            respond(
                &requests[1],
                200,
                "noter('ailleurs ' + document.currentScript.id);"
            )
        ));
        assert!(p.complete_fetch(
            requests[0].id,
            respond(
                &requests[0],
                200,
                "noter('lot ' + document.currentScript.src);
                 const suite = document.createElement('script');
                 suite.textContent = \"noter('suite');\";
                 document.body.appendChild(suite);"
            )
        ));
        assert!(p.complete_fetch(requests[2].id, respond(&requests[2], 404, "introuvable")));
        assert_eq!(
            body_text(&p),
            "courant premier;en ligne enligne;après insertion;\
             ailleurs ailleurs;load ailleurs true;\
             lot https://exemple.test/dossier/lot.js;suite;load externe true;\
             error absent;"
        );
        assert!(p.take_fetches().is_empty());
    }

    #[test]
    fn window_messages_crypto_and_observers() {
        let mut text = TextContext::new();
        let mut p = page(
            "<div id=haut style='height: 50px'></div>
             <div id=bas style='margin-top: 5000px; height: 50px'></div>
             <p id=p></p>
             <script>
               const sortie = document.getElementById('p');
               const noter = texte => { sortie.textContent += texte + ';'; };
               noter([frames === window, onload === null, typeof postMessage, name === '', isSecureContext].join(' '));
               const octets = crypto.getRandomValues(new Uint8Array(32));
               noter('aléa ' + (new Set(octets).size > 8) + ' ' + /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(crypto.randomUUID()));
               try { crypto.getRandomValues(new Float32Array(1)); } catch (e) { noter(e.name); }
               const canal = new MessageChannel();
               canal.port1.onmessage = e => noter('port ' + e.data.n);
               canal.port2.postMessage({ n: 1 });
               addEventListener('message', e => noter('fenêtre ' + e.data + ' ' + (e.source === window)));
               postMessage('bonjour', '*');
               new IntersectionObserver(entries => {
                 for (const e of entries) noter('vu ' + e.target.id + ' ' + e.isIntersecting + ' ' + e.intersectionRatio);
               }).observe(document.getElementById('haut'));
               new IntersectionObserver(entries => {
                 for (const e of entries) noter('vu ' + e.target.id + ' ' + e.isIntersecting);
               }).observe(document.getElementById('bas'));
               const tailles = new ResizeObserver(entries => {
                 for (const e of entries) noter('taille ' + e.contentRect.height + ' ' + e.borderBoxSize[0].blockSize);
                 tailles.disconnect();
               });
               tailles.observe(document.getElementById('haut'));
               noter('fin');
             </script>",
        );
        p.run_scripts();
        p.update(&mut text, VIEWPORT);
        assert_eq!(
            body_text(&p),
            "true true function true true;aléa true true;TypeMismatchError;fin;"
        );
        // Les messages arrivent dans une tâche ultérieure, les observateurs
        // à leur premier relevé.
        let later = Instant::now() + std::time::Duration::from_secs(1);
        assert!(p.tick(later));
        p.update(&mut text, VIEWPORT);
        let _ = p.tick(later + std::time::Duration::from_secs(1));
        let seen = body_text(&p);
        for expected in [
            "port 1;",
            "fenêtre bonjour true;",
            "vu haut true 1;",
            "vu bas false;",
            "taille 50 50;",
        ] {
            assert!(seen.contains(expected), "{expected} absent de {seen}");
        }
    }

    #[test]
    fn intl_formats_numbers_dates_and_lists() {
        let mut p = page(
            "<p id=p></p><script>
               const lignes = [];
               const noter = (...valeurs) => lignes.push(valeurs.join('|'));
               const espaces = texte => texte.replace(/[\\u00a0\\u202f]/g, '_');
               const fr = new Intl.NumberFormat('fr-FR');
               noter(espaces(fr.format(1234567.891)), Intl.NumberFormat('en-US').format(1234567.891));
               noter(espaces(new Intl.NumberFormat('fr-FR', { style: 'currency', currency: 'EUR' }).format(-12.5)),
                     new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD' }).format(1234.5));
               noter(espaces(new Intl.NumberFormat('fr', { style: 'percent' }).format(0.256)),
                     new Intl.NumberFormat('en', { maximumFractionDigits: 0 }).format(2.5),
                     new Intl.NumberFormat('en', { minimumFractionDigits: 2 }).format(3),
                     new Intl.NumberFormat('en', { notation: 'compact' }).format(1250000),
                     (12345.6).toLocaleString('en-US'));
               noter(fr.formatToParts(1.5).map(part => part.type).join('+'), typeof fr.format, fr.resolvedOptions().locale);
               const date = new Date(Date.UTC(2026, 10, 14, 9, 5, 7));
               const utc = { timeZone: 'UTC' };
               noter(new Intl.DateTimeFormat('fr-FR', utc).format(date),
                     new Intl.DateTimeFormat('en-US', utc).format(date),
                     new Intl.DateTimeFormat('en-GB', utc).format(date));
               noter(new Intl.DateTimeFormat('fr-FR', { dateStyle: 'full', ...utc }).format(date),
                     new Intl.DateTimeFormat('en-US', { dateStyle: 'long', ...utc }).format(date));
               noter(espaces(new Intl.DateTimeFormat('fr-FR', { dateStyle: 'medium', timeStyle: 'short', ...utc }).format(date)),
                     espaces(new Intl.DateTimeFormat('en-US', { hour: 'numeric', minute: '2-digit', ...utc }).format(date)),
                     date.toLocaleDateString('fr-FR', { month: 'long', year: 'numeric', ...utc }),
                     date.toLocaleTimeString('fr-FR', utc));
               noter(new Intl.DateTimeFormat('fr', utc).resolvedOptions().timeZone,
                     typeof Intl.DateTimeFormat().resolvedOptions().timeZone);
               noter(new Intl.PluralRules('en').select(1), new Intl.PluralRules('en').select(2),
                     new Intl.PluralRules('fr').select(0), new Intl.PluralRules('en', { type: 'ordinal' }).select(22));
               noter(new Intl.RelativeTimeFormat('fr').format(-3, 'day'),
                     new Intl.RelativeTimeFormat('en').format(2, 'hours'),
                     new Intl.RelativeTimeFormat('fr', { numeric: 'auto' }).format(-1, 'day'));
               noter(new Intl.ListFormat('fr').format(['a', 'b', 'c']),
                     new Intl.ListFormat('en', { type: 'disjunction' }).format(['a', 'b', 'c']));
               noter(['z', 'é', 'a', 'E'].sort(new Intl.Collator('fr').compare).join(''),
                     ['a10', 'a2'].sort(new Intl.Collator('fr', { numeric: true }).compare).join(','),
                     'a'.localeCompare('b'));
               noter(Array.from(new Intl.Segmenter('fr', { granularity: 'word' }).segment('Bonjour à tous'))
                       .filter(piece => piece.isWordLike).map(piece => piece.segment).join('/'),
                     new Intl.DisplayNames('fr', { type: 'region' }).of('US'),
                     new Intl.Locale('FR-fr').toString(), Intl.getCanonicalLocales('EN-us')[0],
                     Intl.NumberFormat.supportedLocalesOf(['de', 'en-GB']).join(','));
               try { new Intl.NumberFormat('fr', { style: 'currency' }); } catch (e) { noter(e.name); }
               try { new Intl.DateTimeFormat('pas une langue'); } catch (e) { noter(e.name); }
               document.getElementById('p').textContent = lignes.join(' ; ');
             </script>",
        );
        p.run_scripts();
        let console: Vec<String> = p.take_console().into_iter().map(|m| m.text).collect();
        assert_eq!(
            body_text(&p),
            "1_234_567,891|1,234,567.891 ; -12,50_€|$1,234.50 ; 26_%|3|3.00|1.3M|12,345.6 ; \
             integer+decimal+fraction|function|fr-FR ; 14/11/2026|11/14/2026|14/11/2026 ; \
             samedi 14 novembre 2026|November 14, 2026 ; 14 nov. 2026 à 09:05|9:05_AM|novembre 2026|09:05:07 ; \
             UTC|string ; one|other|one|two ; il y a 3 jours|in 2 hours|hier ; a, b et c|a, b, or c ; \
             aEéz|a2,a10|-1 ; Bonjour/à/tous|États-Unis|fr-FR|en-US|en-GB ; TypeError ; RangeError",
            "{console:?}"
        );
    }

    #[test]
    fn modules_load_on_demand_and_through_the_import_map() {
        let mut p = page(
            "<p id=p></p>
             <script type=importmap>
               { \"scopes\": { \"/x/\": { \"a\": \"/b.js\" } }, \"entier\": 3, \"liste\": [1, {\"a\": \"b\"}],
                 \"imports\": { \"outil\": \"/libs/outil.js\", \"paquet/\": \"https://cdn.test/paquet@2/\" } }
             </script>
             <script>
               const sortie = document.getElementById('p');
               const noter = texte => { sortie.textContent += texte + ';'; };
               import('./chemin/a.js').then(m => noter('a ' + m.valeur + ' ' + m.venu));
               import('outil').then(m => noter('outil ' + m.default));
               import('paquet/sous/module.js').then(m => noter('paquet ' + m.nom));
               import('/absent.js').catch(e => noter('absent ' + e.name));
               import('inconnu').catch(e => noter('inconnu ' + e.name));
               const inséré = document.createElement('script');
               inséré.type = 'module';
               inséré.src = '/tardif.js';
               inséré.onload = () => noter('load tardif');
               document.head.appendChild(inséré);
             </script>",
        );
        p.run_scripts();
        let respond = |request: &FetchRequest, status: u16, body: &str| {
            Ok(FetchResponse {
                url: request.url.clone(),
                status,
                headers: Vec::new(),
                body: body.as_bytes().to_vec(),
            })
        };
        let mut asked = Vec::new();
        // Chaque vague de réponses peut demander d'autres modules.
        for _ in 0..4 {
            for request in p.take_fetches() {
                asked.push(request.url.to_string());
                let answer = match request.url.path() {
                    "/dossier/chemin/a.js" => respond(
                        &request,
                        200,
                        "import { venu } from './b.js'; export const valeur = 1; export { venu };",
                    ),
                    "/dossier/chemin/b.js" => respond(&request, 200, "export const venu = 'de b';"),
                    "/libs/outil.js" => respond(&request, 200, "export default 'marteau';"),
                    "/paquet@2/sous/module.js" => {
                        respond(&request, 200, "export const nom = 'paquet';")
                    }
                    "/tardif.js" => respond(&request, 200, "noter('tardif exécuté');"),
                    _ => respond(&request, 404, ""),
                };
                // Le booléen dit seulement si la page doit être redessinée.
                let _ = p.complete_fetch(request.id, answer);
            }
        }
        asked.sort();
        assert_eq!(
            asked,
            [
                "https://cdn.test/paquet@2/sous/module.js",
                "https://exemple.test/absent.js",
                "https://exemple.test/dossier/chemin/a.js",
                "https://exemple.test/dossier/chemin/b.js",
                "https://exemple.test/libs/outil.js",
                "https://exemple.test/tardif.js",
            ]
        );
        let seen = body_text(&p);
        for expected in [
            "a 1 de b;",
            "outil marteau;",
            "paquet paquet;",
            "absent TypeError;",
            "inconnu TypeError;",
            "tardif exécuté;",
            "load tardif;",
        ] {
            assert!(seen.contains(expected), "{expected} absent de {seen}");
        }
    }

    #[test]
    fn custom_elements_dom_parser_and_template_content() {
        let mut p = page(
            "<p id=p></p>
             <mon-compteur id=present depart=3></mon-compteur>
             <template id=gabarit><li class=item>un</li><li class=item>deux</li></template>
             <script>
               const sortie = document.getElementById('p');
               const noter = texte => { sortie.textContent += texte + ';'; };
               class Compteur extends HTMLElement {
                 static observedAttributes = ['depart'];
                 constructor() { super(); this.valeur = 0; noter('construit ' + (this.id || 'neuf')); }
                 connectedCallback() { noter('branché ' + (this.id || 'neuf')); }
                 disconnectedCallback() { noter('débranché ' + this.id); }
                 attributeChangedCallback(nom, avant, après) { noter(nom + ' ' + avant + '>' + après); }
                 plus() { return ++this.valeur; }
               }
               customElements.define('mon-compteur', Compteur);
               const présent = document.getElementById('present');
               noter([présent instanceof Compteur, présent instanceof HTMLElement, présent.plus(),
                      customElements.get('mon-compteur') === Compteur, customElements.getName(Compteur)].join(' '));
               const créé = document.createElement('mon-compteur');
               noter('créé ' + (créé instanceof Compteur) + ' ' + créé.plus());
               const direct = new Compteur();
               noter('direct ' + direct.localName);
               try { customElements.define('sans tiret', class extends HTMLElement {}); } catch (e) { noter(e.name); }
               try { new HTMLElement(); } catch (e) { noter(e.name); }
               customElements.whenDefined('mon-compteur').then(c => noter('défini ' + (c === Compteur)));
               créé.id = 'cree';
               document.body.appendChild(créé);
               présent.setAttribute('depart', '4');
               présent.remove();

               const gabarit = document.getElementById('gabarit');
               noter('gabarit ' + gabarit.content.childNodes.length + ' ' + gabarit.childNodes.length + ' '
                     + document.querySelectorAll('.item').length + ' ' + gabarit.content.querySelectorAll('.item').length);
               const copie = gabarit.content.cloneNode(true);
               noter('copie ' + copie.firstChild.textContent);

               const html = new DOMParser().parseFromString(
                 '<!doctype html><title>Titre</title><body class=c><p id=a>salut</p>', 'text/html');
               noter('html ' + [html.title, html.body.className, html.getElementById('a').textContent,
                                html.head.childNodes.length, html === document].join(' '));
               const xml = new DOMParser().parseFromString(
                 '<?xml version=\"1.0\"?><!-- c --><r xmlns:x=\"urn:x\" a=\"1 &amp; 2\"><x:e>t&#233;</x:e><![CDATA[<b>]]><vide/></r>',
                 'application/xml');
               const racine = xml.documentElement;
               noter('xml ' + [racine.localName, racine.getAttribute('a'), racine.firstChild.namespaceURI,
                               racine.firstChild.textContent, racine.childNodes[1].data, racine.lastChild.localName,
                               xml.firstChild.nodeType].join(' '));
               const cassé = new DOMParser().parseFromString('<a><b></a>', 'text/xml');
               noter('cassé ' + cassé.documentElement.localName);
               const svg = new DOMParser().parseFromString(
                 '<svg xmlns=\"http://www.w3.org/2000/svg\"><path d=\"M0 0\"/></svg>', 'image/svg+xml');
               noter('svg ' + svg.documentElement.firstChild.namespaceURI);
             </script>",
        );
        p.run_scripts();
        let console: Vec<String> = p.take_console().into_iter().map(|m| m.text).collect();
        assert_eq!(
            body_text(&p),
            "construit present;depart null>3;branché present;true true 1 true mon-compteur;\
             construit neuf;créé true 1;construit neuf;direct mon-compteur;SyntaxError;TypeError;\
             depart 3>4;gabarit 2 0 0 2;copie un;html Titre c salut 1 false;\
             xml r 1 & 2 urn:x té <b> vide 8;cassé parsererror;svg http://www.w3.org/2000/svg;\
             branché cree;débranché present;défini true;",
            "{console:?}"
        );
    }

    #[test]
    fn streams_and_element_from_point() {
        let mut text = TextContext::new();
        let mut p = page(
            "<style>body { margin: 0 } #boite { width: 100px; height: 50px } #dedans { width: 40px; height: 20px }</style>
             <div id=boite><div id=dedans></div></div><p id=p></p>
             <script>
               const sortie = document.getElementById('p');
               const noter = texte => { sortie.textContent += texte + ';'; };
               (async () => {
                 const flux = new ReadableStream({
                   start(c) { c.enqueue('a'); c.enqueue('b'); },
                   pull(c) { c.enqueue('c'); c.close(); },
                 });
                 const lus = [];
                 for await (const morceau of flux) lus.push(morceau);
                 noter('lus ' + lus.join(''));
                 const majuscules = new TransformStream({ transform(m, c) { c.enqueue(m.toUpperCase()); } });
                 const lecteur = new ReadableStream({ start(c) { c.enqueue('x'); c.enqueue('y'); c.close(); } })
                   .pipeThrough(majuscules).getReader();
                 noter('transformés ' + (await lecteur.read()).value + (await lecteur.read()).value + ' ' + (await lecteur.read()).done);
                 const écrits = [];
                 const puits = new WritableStream({ write(m) { écrits.push(m); } });
                 await new ReadableStream({ start(c) { c.enqueue(1); c.enqueue(2); c.close(); } }).pipeTo(puits);
                 noter('écrits ' + écrits.join('+'));
                 const [un, deux] = new ReadableStream({ start(c) { c.enqueue('t'); c.close(); } }).tee();
                 noter('tee ' + (await un.getReader().read()).value + (await deux.getReader().read()).value);
                 const octets = new ReadableStream({ start(c) { c.enqueue('é'); c.close(); } }).pipeThrough(new TextEncoderStream());
                 noter('encodés ' + (await octets.getReader().read()).value.length);
                 const cassé = new ReadableStream({ start(c) { c.error(new Error('panne')); } });
                 await cassé.getReader().read().catch(e => noter('erreur ' + e.message));
                 setTimeout(() => noter('point ' + document.elementFromPoint(10, 10).id + ' ' + document.elementFromPoint(60, 30).id
                       + ' ' + document.elementsFromPoint(10, 10).map(e => e.id || e.localName).join('>')
                       + ' ' + document.elementFromPoint(-5, 0)), 10);
               })();
             </script>",
        );
        p.run_scripts();
        p.update(&mut text, VIEWPORT);
        let _ = p.tick(Instant::now() + std::time::Duration::from_secs(1));
        let console: Vec<String> = p.take_console().into_iter().map(|m| m.text).collect();
        assert_eq!(
            body_text(&p),
            "lus abc;transformés XY true;écrits 1+2;tee tt;encodés 2;erreur panne;\
             point dedans boite dedans>boite>body>html null;",
            "{console:?}"
        );
    }

    #[test]
    fn text_is_selected_with_the_mouse() {
        let mut text = TextContext::new();
        let mut p = page(
            "<body style='margin:0;font-size:16px'>
               <p style='margin:0'>Premier paragraphe</p><p style='margin:0'>Second paragraphe</p>
             </body>",
        );
        p.update(&mut text, VIEWPORT);
        let highlights = |p: &Page| {
            p.display_list()
                .items
                .iter()
                .filter(
                    |item| matches!(item, DisplayItem::Rect { color, .. } if *color == SELECTION),
                )
                .count()
        };
        assert_eq!(p.selected_text(), "");
        // Bouton enfoncé : rien n'est encore sélectionné.
        assert!(!p.select_start(0.0, 8.0));
        assert_eq!(highlights(&p), 0);
        // Glisser jusqu'à la seconde ligne sélectionne les deux.
        assert!(p.select_to(400.0, 28.0));
        assert_eq!(p.selected_text(), "Premier paragraphe\nSecond paragraphe");
        assert_eq!(highlights(&p), 2);
        // La sélection survit à une nouvelle mise en page.
        p.set_viewport(Viewport {
            width: 500.0,
            height: 300.0,
        });
        p.update(
            &mut text,
            Viewport {
                width: 500.0,
                height: 300.0,
            },
        );
        assert_eq!(p.selected_text(), "Premier paragraphe\nSecond paragraphe");
        assert_eq!(highlights(&p), 2);
        // Double clic : un mot ; Ctrl A : tout ; un clic simple efface.
        assert!(p.select_word(4.0, 8.0));
        assert_eq!(p.selected_text(), "Premier");
        assert!(p.select_all());
        assert_eq!(p.selected_text(), "Premier paragraphe\nSecond paragraphe");
        assert!(p.clear_selection());
        assert_eq!((p.selected_text(), highlights(&p)), ("", 0));
        assert!(!p.select_to(100.0, 8.0), "sans ancrage, rien ne s'étend");
    }

    #[test]
    fn the_selection_is_copied_with_the_keyboard() {
        let mut text = TextContext::new();
        let mut p = page(
            "<body style='margin:0;font-size:16px'>
               <p style='margin:0'>Bonjour le monde</p><input id=champ>
               <script>
                 addEventListener('keydown', e => { if (e.key === 'b') e.preventDefault(); });
               </script>
             </body>",
        );
        p.update(&mut text, VIEWPORT);
        let key = |key: &'static str, ctrl: bool| KeyInput {
            key,
            code: "",
            ctrl,
            shift: false,
            alt: false,
            meta: false,
            repeat: false,
        };
        // Sans sélection, Ctrl C ne copie rien.
        p.key_event(true, &key("c", true));
        assert_eq!(p.take_copied(), None);
        // Ctrl A sélectionne tout ; la touche relâchée ne compte pas.
        assert_eq!(p.key_event(true, &key("a", true)), (false, true));
        assert_eq!(p.selected_text(), "Bonjour le monde");
        p.key_event(false, &key("c", true));
        assert_eq!(p.take_copied(), None);
        p.key_event(true, &key("C", true));
        assert_eq!(p.take_copied().as_deref(), Some("Bonjour le monde"));
        assert_eq!(p.take_copied(), None, "le texte n'est remis qu'une fois");
        // Sans Ctrl, « a » n'est qu'une lettre.
        p.clear_selection();
        p.key_event(true, &key("a", false));
        assert_eq!(p.selected_text(), "");
        // Dans un champ de saisie, Ctrl A est laissé au champ.
        let field = p
            .document()
            .descendants(p.document().root())
            .find(|n| p.document().element(*n).is_some_and(|e| e.is_html("input")))
            .expect("champ");
        p.click_node(field);
        p.key_event(true, &key("a", true));
        assert_eq!(p.selected_text(), "");
    }

    #[test]
    fn text_areas_edit_at_the_caret_and_tab_follows_tabindex() {
        let mut text = TextContext::new();
        let mut p = page(
            "<body style='margin:0'>
               <textarea id=zone style='width:300px;height:100px;font-size:16px'>abc\ndefgh\nij</textarea>
               <input id=trois tabindex=3>
               <input id=libre>
               <input id=un tabindex=1 autofocus>
               <input id=hors tabindex=-1>
               <script>
                 var saisies = 0;
                 addEventListener('input', () => {
                   document.documentElement.setAttribute('data-saisies', ++saisies);
                 });
               </script>
             </body>",
        );
        p.run_scripts();
        p.update(&mut text, VIEWPORT);
        let by_id = |p: &Page, id: &str| {
            p.document()
                .descendants(p.document().root())
                .find(|n| {
                    p.document()
                        .element(*n)
                        .is_some_and(|e| e.attr("id") == Some(id))
                })
                .expect("élément")
        };
        let focused = |p: &Page| {
            p.document()
                .focus()
                .and_then(|node| p.document().element(node)?.attr("id").map(str::to_owned))
        };
        let key = |key: &'static str, shift: bool| KeyInput {
            key,
            code: "",
            ctrl: false,
            shift,
            alt: false,
            meta: false,
            repeat: false,
        };
        // `autofocus` a donné le focus au chargement.
        assert_eq!(focused(&p).as_deref(), Some("un"));
        // Tab : les `tabindex` positifs d'abord, par valeur ; puis l'ordre du
        // document ; un `tabindex` négatif n'y est pas.
        let mut order = Vec::new();
        for _ in 0..4 {
            p.key_event(true, &key("Tab", false));
            order.push(focused(&p).unwrap_or_default());
        }
        assert_eq!(order, ["trois", "zone", "libre", "un"]);
        p.key_event(true, &key("Tab", true));
        assert_eq!(focused(&p).as_deref(), Some("libre"));

        // Zone de texte : le focus place le curseur en fin de texte.
        let zone = by_id(&p, "zone");
        let content = |p: &Page| p.document().text_content(by_id(p, "zone"));
        p.click_node(zone);
        assert_eq!(p.document().caret(), "abc\ndefgh\nij".len());
        // Haut : même colonne sur la ligne d'au-dessus ; Début : début de ligne.
        p.key_event(true, &key("ArrowUp", false));
        assert_eq!(p.document().caret(), "abc\nde".len());
        p.key_event(true, &key("ArrowUp", false));
        assert_eq!(p.document().caret(), "ab".len());
        p.key_event(true, &key("X", false));
        assert_eq!(content(&p), "abXc\ndefgh\nij");
        p.key_event(true, &key("End", false));
        p.key_event(true, &key("Enter", false));
        assert_eq!(content(&p), "abXc\n\ndefgh\nij");
        assert_eq!(p.document().caret(), "abXc\n".len());
        // Bas depuis une ligne vide, puis Retour arrière au milieu.
        p.key_event(true, &key("ArrowDown", false));
        p.key_event(true, &key("ArrowRight", false));
        p.key_event(true, &key("Backspace", false));
        assert_eq!(content(&p), "abXc\n\nefgh\nij");
        // Maj Fin sélectionne jusqu'à la fin de la ligne ; coller la remplace.
        p.key_event(true, &key("End", true));
        p.paste("12\r\n34");
        assert_eq!(content(&p), "abXc\n\n12\n34\nij");
        assert_eq!(p.document().caret(), "abXc\n\n12\n34".len());
        assert_eq!(
            p.document()
                .element(
                    p.document()
                        .find_html_element(p.document().root(), "html")
                        .expect("html")
                )
                .and_then(|e| e.attr("data-saisies")),
            Some("4")
        );
        // Un clic place le curseur : en haut à gauche, au début du texte.
        p.update(&mut text, VIEWPORT);
        p.click(3.0, 3.0);
        assert_eq!(focused(&p).as_deref(), Some("zone"));
        assert_eq!(p.document().caret(), 0);
        // Et le curseur est dessiné.
        p.update(&mut text, VIEWPORT);
        let before = p.display_list().items.len();
        // Un clic hors de tout champ retire le focus, et le curseur avec.
        let body = p
            .document()
            .find_html_element(p.document().root(), "body")
            .expect("body");
        p.click_node(body);
        assert_eq!(focused(&p), None);
        p.update(&mut text, VIEWPORT);
        assert!(p.display_list().items.len() < before);
    }

    #[test]
    fn a_select_drops_down_and_picks_by_click_or_key() {
        let mut text = TextContext::new();
        let mut p = page(
            "<body style='margin:0'>
               <select id=liste style='font-size:16px'>
                 <option>un</option><option>deux</option><option disabled>trois</option><option>quatre</option>
               </select>
               <p id=ailleurs style='margin-top:200px'>ailleurs</p>
               <script>
                 var journal = [];
                 liste.addEventListener('change', () => {
                   journal.push(liste.value);
                   document.body.setAttribute('data-journal', journal.join(','));
                 });
               </script>
             </body>",
        );
        p.run_scripts();
        p.update(&mut text, VIEWPORT);
        let by_id = |p: &Page, id: &str| {
            p.document()
                .descendants(p.document().root())
                .find(|n| {
                    p.document()
                        .element(*n)
                        .is_some_and(|e| e.attr("id") == Some(id))
                })
                .expect("élément")
        };
        let key = |key: &'static str| KeyInput {
            key,
            code: "",
            ctrl: false,
            shift: false,
            alt: false,
            meta: false,
            repeat: false,
        };
        let select = by_id(&p, "liste");
        let open = |p: &Page| {
            p.document()
                .has_state(by_id(p, "liste"), oxyde_dom::STATE_SELECT_OPEN)
        };
        let journal = |p: &Page| {
            let doc = p.document();
            let body = doc.find_html_element(doc.root(), "body").expect("body");
            doc.element(body)
                .and_then(|e| e.attr("data-journal"))
                .unwrap_or_default()
                .to_owned()
        };
        // Rectangle, dans la page, de la ligne de l'option de rang `rank`
        // dans la liste déroulée.
        let row = |p: &Page, rank: usize| {
            let option = p
                .document()
                .descendants(by_id(p, "liste"))
                .filter(|n| {
                    p.document()
                        .element(*n)
                        .is_some_and(|e| e.is_html("option"))
                })
                .nth(rank)
                .expect("option");
            let mut stack: Vec<&oxyde_layout::Fragment> = p
                .tree
                .as_ref()
                .and_then(|tree| tree.root.as_ref())
                .into_iter()
                .collect();
            while let Some(fragment) = stack.pop() {
                match fragment {
                    oxyde_layout::Fragment::Box(b) if b.node == Some(option) => {
                        return b.border_box;
                    }
                    oxyde_layout::Fragment::Box(b) => stack.extend(b.children.iter()),
                    oxyde_layout::Fragment::Text(text) => stack.extend(text.boxes.iter()),
                }
            }
            panic!("ligne de l'option absente de la mise en page");
        };
        let closed = p.display_list().items.len();
        // Un clic déroule la liste : ses options se dessinent sous elle.
        p.click_node(select);
        p.update(&mut text, VIEWPORT);
        assert!(open(&p));
        assert!(p.display_list().items.len() > closed);
        let second = row(&p, 1);
        assert!(second.y > 16.0, "{second:?}");
        // Un clic sur une option la choisit et referme la liste.
        p.click(second.x + 5.0, second.y + second.h / 2.0);
        p.update(&mut text, VIEWPORT);
        assert!(!open(&p));
        assert_eq!(journal(&p), "deux");
        assert_eq!(p.display_list().items.len(), closed);
        // Une option désactivée ne se choisit pas ; la liste se referme.
        p.click_node(select);
        p.update(&mut text, VIEWPORT);
        let third = row(&p, 2);
        p.click(third.x + 5.0, third.y + third.h / 2.0);
        assert!(!open(&p));
        assert_eq!(journal(&p), "deux");
        // Au clavier : Entrée déroule, les flèches choisissent, Échap referme.
        p.key_event(true, &key("Enter"));
        assert!(open(&p));
        p.key_event(true, &key("ArrowDown"));
        assert_eq!(journal(&p), "deux,quatre");
        p.key_event(true, &key("Escape"));
        assert!(!open(&p));
        // Un clic ailleurs referme aussi.
        p.key_event(true, &key(" "));
        assert!(open(&p));
        p.click_node(by_id(&p, "ailleurs"));
        assert!(!open(&p));
    }

    #[test]
    fn a_field_selects_copies_cuts_and_pastes() {
        let mut text = TextContext::new();
        let mut p = page(
            "<body style='margin:0'>
               <input id=champ value='bonjour le monde' style='width:300px;font-size:16px'>
               <input id=secret type=password value='motdepasse'>
               <textarea id=zone>un</textarea>
               <script>
                 var saisies = 0;
                 addEventListener('input', () => {
                   document.documentElement.setAttribute('data-saisies', ++saisies);
                 });
               </script>
             </body>",
        );
        p.run_scripts();
        p.update(&mut text, VIEWPORT);
        let by_id = |p: &Page, id: &str| {
            p.document()
                .descendants(p.document().root())
                .find(|n| {
                    p.document()
                        .element(*n)
                        .is_some_and(|e| e.attr("id") == Some(id))
                })
                .expect("élément")
        };
        let value = |p: &Page, id: &str| {
            p.document()
                .element(by_id(p, id))
                .and_then(|e| e.attr("value"))
                .unwrap_or("")
                .to_owned()
        };
        let key = |key: &'static str, ctrl: bool, shift: bool| KeyInput {
            key,
            code: "",
            ctrl,
            shift,
            alt: false,
            meta: false,
            repeat: false,
        };
        let field = by_id(&p, "champ");
        p.click_node(field);
        // Le focus place le curseur en fin de valeur ; Maj Début sélectionne
        // tout, comme Ctrl A.
        p.key_event(true, &key("Home", false, true));
        assert_eq!(
            (p.document().selection_anchor(), p.document().caret()),
            (16, 0)
        );
        p.key_event(true, &key("a", true, false));
        assert_eq!(
            (p.document().selection_anchor(), p.document().caret()),
            (0, 16)
        );
        // Ctrl A dans un champ ne sélectionne pas le texte de la page.
        assert_eq!(p.selected_text(), "");
        p.key_event(true, &key("c", true, false));
        assert_eq!(p.take_copied().as_deref(), Some("bonjour le monde"));
        assert_eq!(value(&p, "champ"), "bonjour le monde");
        // Une flèche referme la sélection sur son bord ; Maj flèche l'étend
        // d'un caractère ; la frappe la remplace.
        p.key_event(true, &key("ArrowLeft", false, false));
        assert_eq!(
            (p.document().selection_anchor(), p.document().caret()),
            (0, 0)
        );
        for _ in 0..7 {
            p.key_event(true, &key("ArrowRight", false, true));
        }
        p.key_event(true, &key("S", false, true));
        assert_eq!(value(&p, "champ"), "S le monde");
        assert_eq!(p.document().caret(), 1);
        // Ctrl X coupe ; Retour arrière efface la sélection entière.
        p.key_event(true, &key("End", false, true));
        p.key_event(true, &key("x", true, false));
        assert_eq!(p.take_copied().as_deref(), Some(" le monde"));
        assert_eq!(value(&p, "champ"), "S");
        // Coller remplace la sélection ; les sauts de ligne n'entrent pas.
        p.key_event(true, &key("a", true, false));
        assert!(p.paste("deux\r\nlignes"));
        assert_eq!(value(&p, "champ"), "deuxlignes");
        assert_eq!(p.document().caret(), "deuxlignes".len());
        p.key_event(true, &key("ArrowLeft", false, true));
        p.key_event(true, &key("Backspace", false, false));
        assert_eq!(value(&p, "champ"), "deuxligne");
        // Un mot de passe ne se copie ni ne se coupe.
        p.click_node(by_id(&p, "secret"));
        p.key_event(true, &key("a", true, false));
        p.key_event(true, &key("c", true, false));
        p.key_event(true, &key("x", true, false));
        assert_eq!(p.take_copied(), None);
        assert_eq!(value(&p, "secret"), "motdepasse");
        // Une zone de texte reçoit le texte collé à sa fin.
        let area = by_id(&p, "zone");
        p.click_node(area);
        assert!(p.paste(" deux\r\ntrois"));
        assert_eq!(p.document().text_content(area), "un deux\ntrois");
        // Chaque changement de valeur a prévenu les scripts.
        let html = p
            .document()
            .find_html_element(p.document().root(), "html")
            .expect("html");
        assert_eq!(
            p.document()
                .element(html)
                .and_then(|e| e.attr("data-saisies")),
            Some("5")
        );
    }

    #[test]
    fn a_click_places_the_caret_and_the_selection_is_drawn() {
        let mut text = TextContext::new();
        let mut p = page(
            "<body style='margin:0'>
               <input value='abcdefghij' style='width:300px;font-size:16px;margin:0'>
             </body>",
        );
        p.update(&mut text, VIEWPORT);
        let rects = |p: &Page| p.display_list().items.len();
        // Un clic tout à gauche du champ : curseur au début ; tout à droite
        // du texte : à la fin.
        p.click(4.0, 10.0);
        assert_eq!(p.document().caret(), 0);
        p.click(290.0, 10.0);
        assert_eq!(p.document().caret(), 10);
        p.update(&mut text, VIEWPORT);
        let plain = rects(&p);
        // La sélection ajoute son surlignage à l'affichage.
        p.key_event(
            true,
            &KeyInput {
                key: "a",
                code: "",
                ctrl: true,
                shift: false,
                alt: false,
                meta: false,
                repeat: false,
            },
        );
        p.update(&mut text, VIEWPORT);
        assert_eq!(rects(&p), plain + 1);
    }

    #[test]
    fn a_context_menu_knows_what_is_under_the_pointer() {
        let mut text = TextContext::new();
        let mut p = page(
            "<body style='margin:0;font-size:16px'>
               <p style='margin:0'><a href='/cible'>Un lien</a></p>
               <p style='margin:0'>Du texte</p>
               <input id=champ value='abc'>
             </body>",
        );
        p.update(&mut text, VIEWPORT);
        // Sur le lien : sa cible ; rien de sélectionné, rien où coller.
        let on_link = p.context_at(10.0, 8.0);
        assert_eq!(
            on_link.link.as_ref().map(Url::as_str),
            Some("https://exemple.test/cible")
        );
        assert!(!on_link.selection && !on_link.editable);
        assert_eq!(p.context_at(10.0, 28.0).link, None);
        // « Tout sélectionner » puis « Copier » : le texte de la page.
        assert!(p.select_everything());
        assert!(p.context_at(10.0, 28.0).selection);
        p.copy();
        assert_eq!(p.take_copied().as_deref(), Some("Un lien\nDu texte"));
        // Dans un champ : sa valeur, et l'on peut coller.
        let field = p
            .document()
            .descendants(p.document().root())
            .find(|n| p.document().element(*n).is_some_and(|e| e.is_html("input")))
            .expect("champ");
        p.click_node(field);
        let in_field = p.context_at(10.0, 28.0);
        assert!(in_field.editable && !in_field.selection);
        assert!(p.select_everything());
        assert!(p.context_at(10.0, 28.0).selection);
        p.copy();
        assert_eq!(p.take_copied().as_deref(), Some("abc"));
    }

    #[test]
    fn scripts_receive_mouse_events() {
        let mut text = TextContext::new();
        let mut p = page(
            "<body style='margin:0'>
               <div id=a style='height:40px'><span id=s>dedans</span></div>
               <div id=b style='height:40px'></div>
               <div style='height:3000px'></div>
               <script>
                 const journal = [];
                 const note = (e) => journal.push(
                   e.type + ':' + (e.target.id || e.target.nodeName) + '@' + e.clientX + ',' + e.clientY);
                 for (const kind of ['mouseover', 'mouseout', 'mousemove', 'mousedown', 'mouseup',
                   'click', 'dblclick', 'contextmenu', 'pointerdown']) {
                   document.addEventListener(kind, note);
                 }
                 for (const id of ['a', 'b']) {
                   const element = document.getElementById(id);
                   element.addEventListener('mouseenter', note);
                   element.addEventListener('mouseleave', note);
                 }
                 document.addEventListener('click', (e) => {
                   document.documentElement.setAttribute('data-clic', [
                     e instanceof MouseEvent, e.isTrusted, e.button, e.detail, e.ctrlKey,
                     e.pageY, e.buttons].join(' '));
                 });
                 document.getElementById('b').addEventListener('contextmenu', (e) => e.preventDefault());
                 globalThis.lire = () => {
                   document.documentElement.setAttribute('data-journal', journal.join(' '));
                   journal.length = 0;
                 };
                 document.addEventListener('keydown', () => lire());
               </script>
             </body>",
        );
        p.run_scripts();
        p.update(&mut text, VIEWPORT);
        let attr = |p: &Page, name: &str| {
            let doc = p.document();
            let html = doc.find_html_element(doc.root(), "html").expect("html");
            doc.element(html)
                .and_then(|e| e.attr(name))
                .unwrap_or("")
                .to_owned()
        };
        // Une touche fait écrire le journal dans le document.
        let read = |p: &mut Page| {
            p.key_event(
                true,
                &KeyInput {
                    key: "j",
                    code: "",
                    ctrl: false,
                    shift: false,
                    alt: false,
                    meta: false,
                    repeat: false,
                },
            );
            attr(p, "data-journal")
        };
        let at = |x: f32, y: f32| MouseInput {
            x,
            y,
            ..MouseInput::default()
        };
        // Entrée dans la page, sur le texte de `a` : la cible est l'élément.
        p.mouse_move(&at(5.0, 10.0));
        assert_eq!(
            read(&mut p),
            "mouseover:s@5,10 mouseenter:a@5,10 mousemove:s@5,10"
        );
        // De `a` à `b` : sortie, puis entrée, puis déplacement.
        p.mouse_move(&at(5.0, 50.0));
        assert_eq!(
            read(&mut p),
            "mouseout:s@5,50 mouseleave:a@5,50 mouseover:b@5,50 mouseenter:b@5,50 mousemove:b@5,50"
        );
        // Sans changer d'élément : un déplacement seul.
        p.mouse_move(&at(6.0, 51.0));
        assert_eq!(read(&mut p), "mousemove:b@6,51");
        // Bouton enfoncé, relâché, puis clic : un `MouseEvent` de confiance
        // qui porte le rang du clic et les touches.
        let press = MouseInput {
            detail: 2,
            ctrl: true,
            ..at(5.0, 10.0)
        };
        p.mouse_button(true, &press);
        p.mouse_button(false, &press);
        p.click(5.0, 10.0);
        assert_eq!(
            read(&mut p),
            "pointerdown:s@5,10 mousedown:s@5,10 mouseup:s@5,10 click:s@5,10 dblclick:s@5,10"
        );
        assert_eq!(attr(&p, "data-clic"), "true true 0 2 true 10 0");
        // `element.click()` : un `MouseEvent` sans coordonnées.
        let b = p
            .document()
            .descendants(p.document().root())
            .find(|n| {
                p.document()
                    .element(*n)
                    .is_some_and(|e| e.attr("id") == Some("b"))
            })
            .expect("b");
        p.click_node(b);
        assert_eq!(read(&mut p), "click:b@0,0");
        // `contextmenu` : annulé sur `b`, pas sur `a`.
        assert!(p.context_menu_event(5.0, 50.0));
        assert!(!p.context_menu_event(5.0, 10.0));
        assert_eq!(read(&mut p), "contextmenu:b@5,50 contextmenu:s@5,10");
        // Page défilée : `clientY` est dans la fenêtre, `pageY` dans la page.
        p.set_scroll(oxyde_compositor::ROOT_SCROLLER, 0.0, 30.0);
        p.mouse_button(false, &at(5.0, 45.0));
        p.click(5.0, 45.0);
        assert_eq!(read(&mut p), "mouseup:b@5,15 click:b@5,15");
        assert_eq!(attr(&p, "data-clic"), "true true 0 1 false 45 0");
    }

    #[test]
    fn the_wheel_reaches_scripts_and_can_be_canceled() {
        let mut text = TextContext::new();
        let source = |listener: &str| {
            format!(
                "<body style='margin:0'><div id=zone style='height:3000px'></div>
                 <script>
                   let vu = '';
                   {listener}
                   addEventListener('keydown', () => document.documentElement.setAttribute('data-vu', vu));
                 </script></body>"
            )
        };
        let seen = |p: &mut Page| {
            p.key_event(
                true,
                &KeyInput {
                    key: "j",
                    code: "",
                    ctrl: false,
                    shift: false,
                    alt: false,
                    meta: false,
                    repeat: false,
                },
            );
            let doc = p.document();
            let html = doc.find_html_element(doc.root(), "html").expect("html");
            doc.element(html)
                .and_then(|e| e.attr("data-vu"))
                .unwrap_or("")
                .to_owned()
        };
        let input = MouseInput {
            x: 10.0,
            y: 10.0,
            shift: true,
            ..MouseInput::default()
        };
        // Sans écouteur : rien n'est distribué, rien n'est annulé.
        let mut p = page(&source(""));
        p.run_scripts();
        p.update(&mut text, VIEWPORT);
        assert_eq!(p.wheel_listeners(), 0);
        assert!(!p.wheel_event(&input, 0.0, 100.0));
        // Écouteur passif (par défaut sur la fenêtre) : l'événement arrive,
        // sans pouvoir être annulé.
        let mut p = page(&source(
            "addEventListener('wheel', (e) => { e.preventDefault();
               vu = [e.type, e.target.id, e.deltaY, e.wheelDeltaY, e.cancelable, e.shiftKey,
                     e.isTrusted, e.defaultPrevented].join(' '); });",
        ));
        p.run_scripts();
        p.update(&mut text, VIEWPORT);
        assert_eq!(p.wheel_listeners(), 1);
        assert!(!p.wheel_event(&input, 0.0, 100.0));
        assert_eq!(seen(&mut p), "wheel zone 100 -120 false true true false");
        // Écouteur non passif sur un élément : il annule le défilement ;
        // l'ancien `mousewheel` est distribué lui aussi.
        let mut p = page(&source(
            "const zone = document.getElementById('zone');
             zone.addEventListener('mousewheel', (e) => { vu += e.type + ' '; });
             zone.addEventListener('wheel', (e) => { vu += e.type + ' ' + e.cancelable + ' ';
               e.preventDefault(); });",
        ));
        p.run_scripts();
        p.update(&mut text, VIEWPORT);
        assert_eq!(p.wheel_listeners(), 2);
        assert!(p.wheel_event(&input, 0.0, 100.0));
        assert_eq!(seen(&mut p), "wheel true mousewheel ");
    }

    #[test]
    fn shadow_trees_are_scripted_styled_and_laid_out() {
        let mut text = TextContext::new();
        let mut p = page(
            "<body style='margin:0'><style>p { color: rgb(5, 5, 5) }</style>
               <div id=hote><span slot=titre id=t>Titre</span>corps</div>
               <script>
                 const journal = [];
                 const host = document.getElementById('hote');
                 const root = host.attachShadow({ mode: 'open' });
                 root.innerHTML = '<style>:host { display: block; color: rgb(1, 2, 3) } p { color: rgb(9, 9, 9); margin: 0 }</style>'
                   + '<p id=dedans>dedans</p><slot name=titre></slot><slot id=reste></slot>';
                 const inner = root.getElementById('dedans');
                 const out = [
                   root instanceof ShadowRoot, root instanceof DocumentFragment, String(root),
                   host.shadowRoot === root, root.host === host, root.mode,
                   document.querySelector('#dedans') === null, inner.isConnected,
                   inner.getRootNode() === root, inner.getRootNode({ composed: true }) === document,
                   document.getElementById('t').assignedSlot === root.querySelector('slot'),
                   root.getElementById('reste').assignedNodes().length,
                   root.querySelector('slot').assignedElements()[0].id,
                   getComputedStyle(inner).color,
                   root.innerHTML.includes('<p id=\"dedans\">dedans</p>'),
                 ];
                 try { host.attachShadow({ mode: 'open' }); } catch (e) { out.push(e.name); }
                 try { document.createElement('a').attachShadow({ mode: 'open' }); } catch (e) { out.push(e.name); }
                 try { document.createElement('div').attachShadow({}); } catch (e) { out.push(e.name); }
                 const closed = document.createElement('div');
                 closed.attachShadow({ mode: 'closed' });
                 out.push(closed.shadowRoot);
                 document.documentElement.setAttribute('data-out', out.join(' '));

                 // Éléments personnalisés imbriqués et `slotchange`.
                 class Inner extends HTMLElement {
                   connectedCallback() { journal.push('interne relié'); }
                 }
                 customElements.define('x-inner', Inner);
                 class Outer extends HTMLElement {
                   constructor() {
                     super();
                     this.attachShadow({ mode: 'open' }).innerHTML = '<x-inner></x-inner><slot></slot>';
                   }
                   connectedCallback() { journal.push('externe relié'); }
                 }
                 customElements.define('x-outer', Outer);
                 document.body.appendChild(document.createElement('x-outer'));
                 root.getElementById('reste').addEventListener('slotchange', () => journal.push('slotchange'));
                 host.append('plus');
                 addEventListener('keydown', () => {
                   document.documentElement.setAttribute('data-journal', journal.join(', '));
                 });
               </script>
             </body>",
        );
        p.run_scripts();
        p.update(&mut text, VIEWPORT);
        let attr = |p: &Page, name: &str| {
            let doc = p.document();
            let html = doc.find_html_element(doc.root(), "html").expect("html");
            doc.element(html)
                .and_then(|e| e.attr(name))
                .unwrap_or("")
                .to_owned()
        };
        assert_eq!(
            attr(&p, "data-out"),
            "true true [object ShadowRoot] true true open true true true true true 1 t \
             rgb(9, 9, 9) true NotSupportedError NotSupportedError TypeError "
        );
        // L'affichage suit l'arbre aplati : l'arbre fantôme, puis ce que
        // l'hôte y distribue.
        p.select_all();
        assert_eq!(p.selected_text(), "dedans\nTitrecorpsplus");
        // Les rappels et `slotchange` sont partis à la fin de la tâche.
        p.key_event(
            true,
            &KeyInput {
                key: "j",
                code: "",
                ctrl: false,
                shift: false,
                alt: false,
                meta: false,
                repeat: false,
            },
        );
        // LIMITE : l'ordre est celui des mutations, pas celui de l'arbre
        // (un navigateur relie l'élément externe avant l'interne).
        let journal = attr(&p, "data-journal");
        let mut seen: Vec<&str> = journal.split(", ").collect();
        seen.sort_unstable();
        assert_eq!(seen, ["externe relié", "interne relié", "slotchange"]);
    }

    #[test]
    fn events_cross_shadow_trees_with_retargeting() {
        let mut text = TextContext::new();
        let mut p = page(
            "<body style='margin:0'><div id=hote></div>
               <script>
                 const host = document.getElementById('hote');
                 const root = host.attachShadow({ mode: 'open' });
                 root.innerHTML = '<button id=b>ok</button><input id=champ>';
                 const journal = [];
                 const note = (who) => (e) => journal.push(
                   who + ':' + e.type + ':' + (e.target.id || e.target.nodeName) + ':' + e.eventPhase);
                 const button = root.getElementById('b');
                 const field = root.getElementById('champ');
                 for (const kind of ['click', 'prive', 'focusin', 'keydown']) {
                   button.addEventListener(kind, note('bouton'));
                   field.addEventListener(kind, note('champ'));
                   root.addEventListener(kind, note('racine'));
                   host.addEventListener(kind, note('hote'));
                   document.addEventListener(kind, note('document'));
                 }
                 // Un événement non composé ne sort pas de l'arbre fantôme.
                 button.dispatchEvent(new Event('prive', { bubbles: true }));
                 journal.push('|');
                 document.addEventListener('prive', (e) => journal.push('chemin ' + e.composedPath().length));
                 button.dispatchEvent(new Event('prive', { bubbles: true, composed: true }));
                 globalThis.lire = () => {
                   document.documentElement.setAttribute('data-journal', journal.join(' '));
                   document.documentElement.setAttribute('data-actif',
                     document.activeElement.id + ' ' + (root.activeElement && root.activeElement.id));
                   journal.length = 0;
                 };
                 addEventListener('keyup', () => lire());
               </script>
             </body>",
        );
        p.run_scripts();
        p.update(&mut text, VIEWPORT);
        let attr = |p: &Page, name: &str| {
            let doc = p.document();
            let html = doc.find_html_element(doc.root(), "html").expect("html");
            doc.element(html)
                .and_then(|e| e.attr(name))
                .unwrap_or("")
                .to_owned()
        };
        let key = |key: &'static str| KeyInput {
            key,
            code: "",
            ctrl: false,
            shift: false,
            alt: false,
            meta: false,
            repeat: false,
        };
        let read = |p: &mut Page| {
            p.key_event(false, &key("j"));
            attr(p, "data-journal")
        };
        assert_eq!(
            read(&mut p),
            "bouton:prive:b:2 racine:prive:b:3 | bouton:prive:b:2 racine:prive:b:3 \
             hote:prive:hote:2 document:prive:hote:3 chemin 7"
        );
        // Un clic dans l'arbre fantôme : dedans, la cible est le bouton ;
        // dehors, c'est l'hôte, qui est lui-même une cible.
        let inside = |p: &Page, name: &str| {
            let doc = p.document();
            let host = doc
                .descendants(doc.root())
                .find(|n| doc.element(*n).is_some_and(|e| e.id() == Some("hote")))
                .expect("hôte");
            let root = doc.shadow_root(host).expect("racine fantôme");
            doc.descendants(root)
                .find(|n| doc.element(*n).is_some_and(|e| e.is_html(name)))
                .expect("élément de l'arbre fantôme")
        };
        let button = inside(&p, "button");
        p.click_node(button);
        assert_eq!(
            read(&mut p),
            "bouton:focusin:b:2 racine:focusin:b:3 hote:focusin:hote:2 document:focusin:hote:3 \
             bouton:click:b:2 racine:click:b:3 hote:click:hote:2 document:click:hote:3"
        );
        assert_eq!(attr(&p, "data-actif"), "hote b");
        // Une touche vise l'élément qui a vraiment le focus, et la frappe
        // entre dans le champ de l'arbre fantôme.
        let field = inside(&p, "input");
        p.click_node(field);
        read(&mut p);
        p.key_event(true, &key("a"));
        assert_eq!(
            read(&mut p),
            "champ:keydown:champ:2 racine:keydown:champ:3 hote:keydown:hote:2 document:keydown:hote:3"
        );
        assert_eq!(
            p.document().element(field).and_then(|e| e.attr("value")),
            Some("a")
        );
        assert_eq!(attr(&p, "data-actif"), "hote champ");
    }

    #[test]
    fn tables_are_scripted_by_rows_and_cells() {
        let mut p = page(
            "<table id=t><caption>titre</caption><thead><tr><th>a</th></tr></thead>
               <tbody><tr><td>1</td><td>2</td></tr><tr><td>3</td></tr></tbody>
               <tfoot><tr><td>pied</td></tr></tfoot></table>
             <script>
               const t = document.getElementById('t');
               const out = [];
               out.push(t.rows.length, t.tBodies.length, t.tBodies[0].rows[0].cells.length);
               out.push(t.rows instanceof HTMLCollection, t.caption.textContent, t.tHead.rows[0].cells[0].textContent);
               out.push(t.tFoot.rows[0].rowIndex, t.tBodies[0].rows[1].sectionRowIndex);
               out.push(t.rows[1].cells[1].cellIndex, t.rows[1].cells[1].colSpan);
               // Une ligne insérée va dans la section de la ligne voisine.
               const row = t.insertRow(1);
               row.insertCell().textContent = 'neuf';
               row.insertCell(0).textContent = 'avant';
               out.push(row.parentNode.localName, row.rowIndex, row.textContent);
               t.deleteRow(-1);
               out.push(t.tFoot.rows.length, t.rows.length);
               t.tBodies[0].deleteRow(0);
               out.push(t.tBodies[0].rows.length);
               try { t.deleteRow(99); } catch (e) { out.push(e.name); }
               // Sections créées à leur place, une seule fois.
               t.deleteTHead();
               const head = t.createTHead();
               out.push(head === t.createTHead(), head.previousElementSibling.localName);
               const empty = document.createElement('table');
               out.push(empty.insertRow().parentNode.localName, empty.createTBody().localName);
               document.body.setAttribute('data-out', out.join(' '));
             </script>",
        );
        p.run_scripts();
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        assert_eq!(
            doc.element(body).and_then(|e| e.attr("data-out")),
            Some(
                "4 1 2 true titre a 3 1 1 1 tbody 1 avantneuf 0 4 2 IndexSizeError \
                 true caption tbody tbody"
            )
        );
    }

    #[test]
    fn uncaught_exceptions_reach_window_error_handlers() {
        let mut p = page(
            "<body><script>
               const out = [];
               window.onerror = (message, file, line, column, error) => {
                 out.push('onerror:' + error.message + ':' + (typeof message));
                 return true;
               };
               addEventListener('error', (e) => {
                 out.push('error:' + e.error.message + ':' + e.isTrusted + ':' + e.defaultPrevented);
               });
               addEventListener('unhandledrejection', (e) => {
                 out.push('rejet:' + e.reason + ':' + (e.promise instanceof Promise));
                 e.preventDefault();
                 document.body.setAttribute('data-out', out.join(' '));
               });
               reportError(new Error('un'));
               document.body.addEventListener('click', () => { throw new Error('deux'); });
               document.body.click();
               Promise.reject('trois');
             </script>",
        );
        p.run_scripts();
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        assert_eq!(
            doc.element(body).and_then(|e| e.attr("data-out")),
            Some(
                "error:un:true:false onerror:un:string error:deux:true:false onerror:deux:string \
                 rejet:trois:true"
            )
        );
    }

    #[test]
    fn abort_signals_run_algorithms_and_dependents_in_order() {
        let mut p = page(
            "<body><script>
               const out = [];
               const controller = new AbortController();
               const any = AbortSignal.any([controller.signal]);
               const deep = AbortSignal.any([any]);
               deep.addEventListener('abort', () => out.push('profond'));
               any.addEventListener('abort', () => out.push('suiveur:' + deep.aborted));
               controller.signal.addEventListener('abort', (e) => out.push('source:' + any.aborted + ':' + e.isTrusted));
               // L'option `signal` retire l'écouteur à l'abandon.
               let clicks = 0;
               document.body.addEventListener('click', () => { clicks += 1; }, { signal: controller.signal });
               document.body.click();
               controller.abort('fini');
               document.body.click();
               document.body.addEventListener('click', () => { clicks += 10; }, { signal: controller.signal });
               document.body.click();
               out.push(clicks, deep.reason, AbortSignal.abort().reason.name);
               document.body.setAttribute('data-out', out.join(' '));
             </script>",
        );
        p.run_scripts();
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        assert_eq!(
            doc.element(body).and_then(|e| e.attr("data-out")),
            Some("source:true:true suiveur:true profond 1 fini AbortError")
        );
    }

    #[test]
    fn observables_share_a_producer_and_compose() {
        let mut p = page(
            "<body><script>
               const out = [];
               let producers = 0;
               const source = new Observable((subscriber) => {
                 producers += 1;
                 subscriber.addTeardown(() => out.push('nettoyage'));
                 queueMicrotask(() => {
                   for (const value of [1, 2, 3, 4]) subscriber.next(value);
                   subscriber.complete();
                 });
               });
               source.map((v) => v * 10).filter((v) => v > 10).take(2).subscribe({
                 next: (v) => out.push(v),
                 complete: () => out.push('fin'),
               });
               source.subscribe((v) => out.push('b' + v));
               Observable.from([5, 6]).toArray().then((values) => out.push(values.join('+')));
               const clicks = document.body.when('click').map((e) => e.type);
               const controller = new AbortController();
               clicks.subscribe((type) => out.push(type), { signal: controller.signal });
               document.body.click();
               controller.abort();
               document.body.click();
               source.last().then((v) => {
                 out.push('dernier' + v, producers);
                 document.body.setAttribute('data-out', out.join(' '));
               });
             </script>",
        );
        p.run_scripts();
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        assert_eq!(
            doc.element(body).and_then(|e| e.attr("data-out")),
            Some("click b1 20 b2 30 fin b3 b4 nettoyage 5+6 dernier4 1")
        );
    }

    #[test]
    fn touches_reach_scripts_and_a_tap_clicks() {
        let mut p = page(
            "<body style='margin:0'><div id=a style='width:100px;height:100px'></div>
             <script>
               const out = [];
               const a = document.getElementById('a');
               for (const kind of ['touchstart', 'touchmove', 'touchend']) {
                 a.addEventListener(kind, (e) => {
                   const t = e.changedTouches[0];
                   out.push(kind + ':' + e.touches.length + ':' + e.targetTouches.length + ':'
                     + t.clientX + ',' + t.clientY + ':' + (t.target === a) + ':' + e.cancelable
                     + ':' + e.isTrusted + ':' + (e.touches instanceof TouchList));
                   document.body.setAttribute('data-out', out.join(' '));
                 }, { passive: kind !== 'touchmove' });
               }
               a.addEventListener('click', (e) => {
                 out.push('click:' + e.clientX);
                 document.body.setAttribute('data-out', out.join(' '));
               });
             </script>",
        );
        p.run_scripts();
        let mut text = TextContext::new();
        p.update(&mut text, VIEWPORT);
        // Un appui bref : contact, lever, puis clic.
        assert!(!p.touch(TouchPhase::Start, 1, 20.0, 30.0));
        assert!(!p.touch(TouchPhase::End, 1, 20.0, 30.0));
        // Un glissement : pas de clic ; le contact garde sa cible hors d'elle.
        p.touch(TouchPhase::Start, 2, 20.0, 30.0);
        p.touch(TouchPhase::Move, 2, 60.0, 300.0);
        p.touch(TouchPhase::End, 2, 60.0, 300.0);
        // Un contact inconnu est ignoré.
        assert!(!p.touch(TouchPhase::Move, 9, 1.0, 1.0));
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        assert_eq!(
            doc.element(body).and_then(|e| e.attr("data-out")),
            Some(
                "touchstart:1:1:20,30:true:false:true:true touchend:0:0:20,30:true:true:true:true \
                 click:20 touchstart:1:1:20,30:true:false:true:true \
                 touchmove:1:1:60,300:true:true:true:true touchend:0:0:60,300:true:true:true:true"
            )
        );
    }

    #[test]
    fn mutation_observers_are_notified_in_a_microtask() {
        let mut p = page(
            "<body><div id=a></div><script>
               const out = [];
               const a = document.getElementById('a');
               new MutationObserver((records) => out.push('mo' + records.length)).observe(a, { attributes: true });
               queueMicrotask(() => out.push('avant'));
               a.setAttribute('x', '1');
               a.setAttribute('y', '2');
               Promise.resolve().then(() => {
                 out.push('apres');
                 a.setAttribute('z', '3');
                 Promise.resolve().then(() => {
                   out.push('fin');
                   document.body.setAttribute('data-out', out.join(' '));
                 });
               });
             </script>",
        );
        p.run_scripts();
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        assert_eq!(
            doc.element(body).and_then(|e| e.attr("data-out")),
            Some("avant mo2 apres mo1 fin")
        );
    }

    #[test]
    fn small_dom_conformance_points() {
        let mut p = page(
            "<body><div id=a>x</div><script>
               const out = [];
               const a = document.getElementById('a');
               try { document.querySelector('('); } catch (e) { out.push(e instanceof DOMException, e.name, e.code); }
               try { a.firstChild.appendData(); } catch (e) { out.push(e.name); }
               a.textContent = undefined;
               out.push(JSON.stringify(a.textContent));
               const mouse = new MouseEvent('x');
               mouse.initMouseEvent('clic', true, true, window, 2, 1, 2, 30, 40, true, false, false, false, 1, a);
               out.push(mouse.type, mouse.detail, mouse.clientX, mouse.ctrlKey, mouse.button, mouse.relatedTarget === a);
               const key = new KeyboardEvent('x');
               key.initKeyboardEvent('touche', true, true, window, 'k', 1, false, true);
               out.push(key.type, key.key, key.location, key.altKey);
               const range = new StaticRange({ startContainer: a, startOffset: 0, endContainer: a, endOffset: 0 });
               out.push(range.collapsed, range.startContainer === a);
               try { new StaticRange({ startContainer: a, startOffset: 0 }); } catch (e) { out.push(e.name); }
               out.push(Element.prototype[Symbol.unscopables].remove, 'event' in window, window.event);
               const pi = document.createProcessingInstruction('cible', 'd');
               out.push(pi.isEqualNode(pi), pi.isEqualNode(document.createProcessingInstruction('cible', 'e')));
               document.body.setAttribute('data-out', out.join(' '));
             </script>",
        );
        p.run_scripts();
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        assert_eq!(
            doc.element(body).and_then(|e| e.attr("data-out")),
            Some(
                "true SyntaxError 12 TypeError \"\" clic 2 30 true 1 true touche k 1 true \
                 true true TypeError true true  true false"
            )
        );
    }

    #[test]
    fn outer_html_replaces_the_element() {
        let mut p = page(
            "<body><div id=a><p id=b>x</p></div><script>
               const out = [];
               const a = document.getElementById('a');
               const b = document.getElementById('b');
               const records = [];
               new MutationObserver((list) => records.push(...list)).observe(a, { childList: true });
               b.outerHTML = '<i id=c>un</i><b id=d>deux</b>';
               out.push(a.innerHTML, b.parentNode === null, document.getElementById('b') === null);
               const loose = document.createElement('p');
               loose.outerHTML = '<i></i>';
               out.push(loose.localName);
               try { document.documentElement.outerHTML = '<html></html>'; } catch (e) { out.push(e.name); }
               a.innerHTML = '<u>trois</u>';
               Promise.resolve().then(() => {
                 out.push(records.map((r) => r.addedNodes.length + '+' + r.removedNodes.length).join(','));
                 document.body.setAttribute('data-out', out.join(' '));
               });
             </script>",
        );
        p.run_scripts();
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        assert_eq!(
            doc.element(body).and_then(|e| e.attr("data-out")),
            Some(
                "<i id=\"c\">un</i><b id=\"d\">deux</b> true true p NoModificationAllowedError 2+1,1+2"
            )
        );
    }

    #[test]
    fn collections_hold_their_indices_and_names() {
        let mut p = page(
            "<body><p id=x name=y data-user-id=7 data-a=1></p><i></i><script>
               'use strict';
               const out = [];
               const c = document.body.children;
               out.push(Object.keys(c).join(','), Object.getOwnPropertyNames(c).join(','));
               for (const run of [() => { c[0] = 1; }, () => { delete c[0]; }, () => { delete c.x; },
                 () => Object.defineProperty(c, 0, { value: 5 }), () => { c[9] = 1; }, () => { c.x = 1; }]) {
                 try { run(); out.push('ok'); } catch (e) { out.push(e.name); }
               }
               c.plus = 3;
               out.push(c.plus, c[0] === document.getElementById('x'), delete c[9]);
               out.push('values' in c, 'map' in c, typeof c[Symbol.iterator], [...c].length);
               const all = document.querySelectorAll('p');
               out.push(typeof all.forEach, 'map' in all, typeof document.body.childNodes.forEach);
               const p = document.getElementById('x');
               out.push(Object.getOwnPropertyNames(p.attributes).join(','), Object.keys(p.dataset).join(','));
               out.push(delete p.dataset.userId, p.hasAttribute('data-user-id'));
               c.late = 'posé';
               document.body.appendChild(document.createElement('b')).id = 'late';
               out.push(c.late, c['4294967295']);
               document.body.setAttribute('data-out', out.join(' '));
             </script>",
        );
        p.run_scripts();
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        assert_eq!(
            doc.element(body).and_then(|e| e.attr("data-out")),
            Some(
                "0,1,2 0,1,2,x,y TypeError TypeError TypeError TypeError TypeError TypeError \
                 3 true true false false function 3 function false function \
                 0,1,2,3,id,name,data-user-id,data-a userId,a true false posé "
            )
        );
    }

    #[test]
    fn scrolling_ends_with_scrollend_and_style_strings_apply() {
        let mut p = page(
            "<body><div id=c></div><script>
               const out = [];
               const t = document.createElement('div');
               t.style = 'overflow: scroll; height: 100px; width: 100px';
               const child = document.createElement('div');
               child.style = 'height: 300px; width: 50px';
               t.appendChild(child);
               document.getElementById('c').appendChild(t);
               const note = (text) => {
                 out.push(text);
                 document.body.setAttribute('data-out', out.join(' '));
               };
               t.addEventListener('scroll', () => note('scroll' + t.scrollTop));
               t.onscrollend = () => note('fin' + t.scrollTop);
               note(t.getAttribute('style').length > 0, 'onscrollend' in window, t.clientHeight);
               t.scrollTop = 40;
               requestAnimationFrame(() => note('image'));
             </script>",
        );
        p.run_scripts();
        for _ in 0..4 {
            let Some(due) = p.next_wake() else { break };
            p.tick(due);
        }
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        assert_eq!(
            doc.element(body).and_then(|e| e.attr("data-out")),
            Some("true scroll40 image fin40")
        );
    }

    #[test]
    fn element_animate_drives_style_and_finishes() {
        let mut p = page(
            "<body><div id=a></div><script>
               const out = [];
               const note = (...items) => {
                 out.push(...items);
                 document.body.setAttribute('data-out', out.join(' '));
               };
               const a = document.getElementById('a');
               const anim = a.animate({ opacity: [0, 1] }, { duration: 100, fill: 'forwards' });
               note(anim.playState, a.getAnimations().length, anim.effect.target === a);
               anim.pause();
               anim.currentTime = 50;
               note(anim.playState, getComputedStyle(a).opacity);
               anim.play();
               anim.onfinish = (e) => note(e.type);
               anim.finished.then((same) => note('fini', same === anim, anim.playState,
                 getComputedStyle(a).opacity));
               const short = a.animate([{ marginLeft: '0px' }, { marginLeft: '10px' }], 20);
               const first = short.finished;
               short.cancel();
               note(short.playState, a.getAnimations().length);
               first.catch((e) => note(e.name));
               try { a.animate({}, { easing: 'pas-une-courbe' }); } catch (e) { note(e.name); }
             </script>",
        );
        p.run_scripts();
        for _ in 0..40 {
            let Some(due) = p.next_wake() else { break };
            p.tick(due);
        }
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        assert_eq!(
            doc.element(body).and_then(|e| e.attr("data-out")),
            Some(
                "running 1 true paused 0.5 idle 1 TypeError AbortError \
                 fini true finished 1 finish"
            )
        );
    }

    #[test]
    fn named_access_lang_dir_and_moved_custom_elements() {
        let mut p = page(
            "<body><div id=un></div><img name=deux><form name=trois></form>
             <p lang=fr-CA dir=rtl><span id=s></span></p>
             <fieldset disabled><legend><input id=libre></legend><input id=pris></fieldset>
             <script>
               const out = [];
               out.push(typeof un, window.un === document.getElementById('un'), window.rien);
               out.push(document.deux.localName, document.trois.localName, document.un);
               const s = document.getElementById('s');
               out.push(s.matches(':lang(fr)'), s.matches(':lang(en, \"fr-CA\")'), s.matches(':lang(f)'));
               out.push(s.matches(':dir(rtl)'), document.body.matches(':dir(ltr)'));
               out.push(document.getElementById('pris').matches(':disabled'),
                 document.getElementById('libre').matches(':disabled'));
               class Simple extends HTMLElement {
                 connectedCallback() { out.push('connecté'); }
                 disconnectedCallback() { out.push('déconnecté:' + this.isConnected); }
               }
               class Mobile extends HTMLElement {
                 connectedMoveCallback() { out.push('déplacé'); }
                 connectedCallback() { out.push('jamais'); }
               }
               customElements.define('x-simple', Simple);
               const simple = document.body.appendChild(document.createElement('x-simple'));
               const target = document.getElementById('un');
               Promise.resolve().then(() => {
                 target.moveBefore(simple, null);
                 customElements.define('x-mobile', Mobile);
                 const mobile = document.createElement('x-mobile');
                 const loose = document.createElement('div');
                 loose.appendChild(mobile);
                 loose.moveBefore(mobile, null);
                 document.body.setAttribute('data-out', out.join(' '));
               });
             </script>",
        );
        p.run_scripts();
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        assert_eq!(
            doc.element(body).and_then(|e| e.attr("data-out")),
            Some(
                "object true  img form  true true false true true true false \
                 connecté déconnecté:true connecté"
            )
        );
    }

    #[test]
    fn declarative_shadow_roots_reach_scripts() {
        let mut p = page(
            "<body><div id=h><template shadowrootmode=open><p id=dedans>ombre</p><slot></slot></template><b>clair</b></div>
             <div id=f><template shadowrootmode=closed><i></i></template></div>
             <script>
               const out = [];
               const h = document.getElementById('h');
               out.push(h.shadowRoot instanceof ShadowRoot, h.shadowRoot.mode, h.shadowRoot.host === h);
               out.push(h.shadowRoot.querySelector('#dedans').textContent, document.getElementById('dedans'));
               out.push(h.querySelector('template'), h.children.length, h.querySelector('b').assignedSlot.localName);
               out.push(document.getElementById('f').shadowRoot);
               document.body.setAttribute('data-out', out.join(' '));
             </script>",
        );
        p.run_scripts();
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        assert_eq!(
            doc.element(body).and_then(|e| e.attr("data-out")),
            Some("true open true ombre   1 slot ")
        );
    }

    #[test]
    fn radio_groups_follow_their_tree() {
        let mut p = page(
            "<body><input type=radio name=g id=a checked><input type=radio name=g id=b>
             <div id=h></div><script>
               const out = [];
               const root = document.getElementById('h').attachShadow({ mode: 'open' });
               root.innerHTML = '<input type=radio name=g id=c checked><input type=radio name=g id=d>';
               root.getElementById('d').click();
               out.push(root.getElementById('c').checked, root.getElementById('d').checked,
                 document.getElementById('a').checked);
               document.getElementById('b').click();
               out.push(document.getElementById('a').checked, root.getElementById('d').checked);
               document.body.setAttribute('data-out', out.join(' '));
             </script>",
        );
        p.run_scripts();
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        assert_eq!(
            doc.element(body).and_then(|e| e.attr("data-out")),
            Some("false true true false true")
        );
    }

    #[test]
    fn foreign_attributes_token_lists_and_documents_without_window() {
        let mut p = page(
            "<body class='  a  a b '><svg id=s viewbox='0 0 1 1' xmlns:xlink='http://www.w3.org/1999/xlink'>
               <use id=u xlink:href='#x'></use></svg><p id=p></p>
             <script>
               const out = [];
               const s = document.getElementById('s');
               const u = document.getElementById('u');
               out.push(s.attributes.getNamedItem('viewBox') !== null, s.hasAttribute('viewbox'));
               const href = u.getAttributeNodeNS('http://www.w3.org/1999/xlink', 'href');
               out.push(href.prefix, href.localName, href.value, u.getAttribute('xlink:href'));
               out.push(JSON.stringify(String(document.body.classList)));
               const all = document.getElementsByTagName('p');
               out.push(all.item(4294967296) === all[0], all.item(4294967295));
               out.push(document.implementation.createHTMLDocument('x').location, document.location === location);
               out.push(document.createEvent('TextEvent') instanceof TextEvent);
               try { document.createEvent('DeviceMotionEvent'); } catch (e) { out.push(e.name); }
               out.push(document.getElementById('p').matches('*|p'), document.querySelectorAll('*|*').length > 3);
               document.body.setAttribute('data-out', out.join(' '));
             </script>",
        );
        p.run_scripts();
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        assert_eq!(
            doc.element(body).and_then(|e| e.attr("data-out")),
            Some(
                "true false xlink href #x #x \"  a  a b \" true   true true \
                 NotSupportedError true true"
            )
        );
    }

    #[test]
    fn attribute_selectors_respect_case_and_namespaces() {
        let mut p = page(
            "<body><div id=d viewBox=1></div><svg id=s viewBox='0 0 1 1' xlink:href='#a'></svg>
             <script>
               const out = [];
               const ids = (selector) => Array.from(document.querySelectorAll(selector), (e) => e.id).join('+');
               out.push(ids('[viewBox]'), ids('[viewbox]'), ids('[*|href]'), ids('[|viewBox]'));
               const loose = document.createElementNS('', 'img');
               loose.setAttribute('name', 'sans');
               document.body.appendChild(loose);
               out.push(document.body.children.namedItem('sans'), document.body.children.namedItem('d').id);
               document.body.setAttribute('data-out', out.join(' '));
             </script>",
        );
        p.run_scripts();
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        assert_eq!(
            doc.element(body).and_then(|e| e.attr("data-out")),
            Some("d+s d s d+s  d")
        );
    }

    #[test]
    fn xml_documents_create_plain_elements_and_records_carry_namespaces() {
        let mut p = page(
            "<body><div id=a></div><script>
               const out = [];
               const xml = new Document();
               const plain = xml.createElement('DIV');
               out.push(plain.localName, plain.namespaceURI, plain.constructor === Element);
               const xhtml = document.implementation.createDocument('http://www.w3.org/1999/xhtml', 'html', null);
               out.push(xhtml.createElement('p').namespaceURI === 'http://www.w3.org/1999/xhtml',
                 document.createElement('p') instanceof HTMLElement);
               const a = document.getElementById('a');
               new MutationObserver((records) => {
                 out.push(records.map((r) => r.attributeName + '@' + r.attributeNamespace).join(','));
                 document.body.setAttribute('data-out', out.join(' '));
               }).observe(a, { attributes: true });
               a.setAttributeNS('http://example.org/', 'e:x', '1');
               a.setAttribute('y', '2');
             </script>",
        );
        p.run_scripts();
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        assert_eq!(
            doc.element(body).and_then(|e| e.attr("data-out")),
            Some("DIV  true true true x@http://example.org/,y@null")
        );
    }

    #[test]
    fn normalize_carries_ranges_and_observers_start_clean() {
        let mut p = page(
            "<body><div id=a></div><p id=b></p><script>
               const out = [];
               const a = document.getElementById('a');
               const t1 = a.appendChild(document.createTextNode('Hello'));
               const t2 = a.appendChild(document.createTextNode(', '));
               const t3 = a.appendChild(document.createTextNode('world'));
               a.appendChild(document.createTextNode(''));
               const range = document.createRange();
               range.setStart(t2, 1);
               range.setEnd(t3, 3);
               const edge = document.createRange();
               edge.setStart(a, 2);
               edge.setEnd(a, 3);
               a.normalize();
               out.push(a.childNodes.length, range.startContainer === t1, range.startOffset, range.endOffset,
                 range.toString(), edge.startContainer === t1, edge.startOffset, edge.endOffset);
               const b = document.getElementById('b');
               const seen = [];
               const observer = new MutationObserver((records) => seen.push(...records.map((r) => r.oldValue)));
               b.id = 'avant';
               observer.observe(b, { attributes: true });
               b.id = 'un';
               observer.disconnect();
               observer.observe(b, { attributes: true, attributeOldValue: true });
               b.id = 'deux';
               out.push(document.implementation.createHTMLDocument('x').defaultView, document.defaultView === window);
               Promise.resolve().then(() => {
                 out.push(seen.join(','));
                 document.body.setAttribute('data-out', out.join(' '));
               });
             </script>",
        );
        p.run_scripts();
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        assert_eq!(
            doc.element(body).and_then(|e| e.attr("data-out")),
            Some("1 true 6 10  wor true 7 12  true un")
        );
    }

    #[test]
    fn removed_subtrees_stay_observed_until_delivery() {
        let mut p = page(
            "<body><div id=parent><div id=child></div></div><script>
               const out = [];
               const parent = document.getElementById('parent');
               const child = document.getElementById('child');
               const observer = new MutationObserver((records) => {
                 out.push('obs:' + records.map((r) => r.type + '@' + r.target.id).join(','));
               });
               observer.observe(parent, { attributes: true, childList: true, subtree: true });
               child.remove();
               Promise.resolve().then(() => out.push('promesse'));
               child.setAttribute('data-x', '1');
               setTimeout(() => {
                 child.setAttribute('data-y', '2');
                 setTimeout(() => document.body.setAttribute('data-out', out.join(' ')), 0);
               }, 0);
             </script>",
        );
        p.run_scripts();
        for _ in 0..6 {
            let Some(due) = p.next_wake() else { break };
            p.tick(due);
        }
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        assert_eq!(
            doc.element(body).and_then(|e| e.attr("data-out")),
            Some("obs:childList@parent,attributes@child promesse")
        );
    }

    #[test]
    fn base_element_and_live_search_params() {
        let mut p = page(
            "<head><base id=b></head><body><a id=a href='x/y'>l</a><script>
               const out = [];
               const a = document.getElementById('a');
               const before = a.href;
               document.getElementById('b').setAttribute('href', 'https://autre.test/dossier/');
               out.push(before !== a.href, a.href, document.baseURI);
               const params = new URLSearchParams('p0=0&p1=1&p2=2');
               const seen = [];
               for (const [name] of params) {
                 seen.push(name);
                 params.delete('p1');
               }
               out.push(seen.join('+'));
               document.body.setAttribute('data-out', out.join(' '));
             </script>",
        );
        p.run_scripts();
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        assert_eq!(
            doc.element(body).and_then(|e| e.attr("data-out")),
            Some("true https://autre.test/dossier/x/y https://autre.test/dossier/ p0+p2")
        );
    }

    #[test]
    fn text_decoder_reads_legacy_encodings() {
        let mut p = page(
            "<body><script>
               const out = [];
               const latin = new TextDecoder('latin1');
               out.push(latin.encoding, latin.decode(new Uint8Array([0x63, 0x61, 0x66, 0xe9, 0x80])));
               out.push(new TextDecoder('koi8-r').decode(new Uint8Array([0xf0, 0xd2, 0xc9])));
               const wide = new TextDecoder('utf-16le');
               out.push(wide.decode(new Uint8Array([0xff, 0xfe, 0x68]), { stream: true })
                 + wide.decode(new Uint8Array([0x00, 0x3d, 0xd8]), { stream: true })
                 + wide.decode(new Uint8Array([0x00, 0xde])));
               try { new TextDecoder('windows-1253', { fatal: true }).decode(new Uint8Array([0xaa])); }
               catch (e) { out.push(e.name); }
               try { new TextDecoder('iso-2022-kr'); } catch (e) { out.push(e.name); }
               const sjis = new TextDecoder('shift_jis');
               out.push(sjis.encoding, sjis.decode(new Uint8Array([0x93]), { stream: true })
                 + sjis.decode(new Uint8Array([0xfa, 0x96]), { stream: true })
                 + sjis.decode(new Uint8Array([0x7b])));
               const jis = new TextDecoder('iso-2022-jp');
               out.push(jis.decode(new Uint8Array([0x61, 0x1b, 0x24]), { stream: true })
                 + jis.decode(new Uint8Array([0x42, 0x46]), { stream: true })
                 + jis.decode(new Uint8Array([0x7c, 0x4b, 0x5c]), { stream: true })
                 + jis.decode());
               out.push(new TextDecoder('gbk').decode(new Uint8Array([0x81, 0x30, 0x81, 0x30])).charCodeAt(0));
               try { new TextDecoder('big5', { fatal: true }).decode(new Uint8Array([0xa4])); }
               catch (e) { out.push(e.name); }
               out.push(new TextDecoder().encoding);
               document.body.setAttribute('data-out', out.join(' '));
             </script>",
        );
        p.run_scripts();
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        assert_eq!(
            doc.element(body).and_then(|e| e.attr("data-out")),
            Some(
                "windows-1252 café€ При h😀 TypeError RangeError shift_jis 日本 a日本 128 TypeError utf-8"
            )
        );
    }

    /// Sert les requêtes de la page et fait avancer ses minuteurs jusqu'à
    /// ce que plus rien ne soit demandé.
    fn serve(p: &mut Page, files: &[(&str, &str)]) {
        for round in 1..40 {
            for request in p.take_fetches() {
                let found = files.iter().find(|(path, _)| *path == request.url.path());
                let _ = p.complete_fetch(
                    request.id,
                    Ok(FetchResponse {
                        url: request.url.clone(),
                        status: if found.is_some() { 200 } else { 404 },
                        headers: vec![("content-type".to_owned(), "text/javascript".to_owned())],
                        body: found
                            .map(|(_, body)| body.as_bytes().to_vec())
                            .unwrap_or_default(),
                    }),
                );
            }
            let _ = p.tick(Instant::now() + Duration::from_secs(round));
        }
    }

    #[test]
    fn inline_svg_is_a_replaced_element_painted_as_an_image() {
        use oxyde_paint::DisplayItem;
        let mut text = TextContext::new();
        let mut p = page(
            "<body style='margin:0;color:rgb(0,128,0)'>             <svg id=a viewBox='0 0 10 5' style='display:block;width:100px'>               <title>pas de texte</title><rect width='5' height='5' fill='currentColor'/>\n               <text y='4' font-size='1'>ni ici</text></svg>             <svg id=b width='30' height='20' style='display:block'><circle r='5'/></svg>",
        );
        p.update(&mut text, VIEWPORT);
        let images: Vec<(f32, f32, f32, f32, u32, u32)> = p
            .display_list()
            .items
            .iter()
            .filter_map(|item| match item {
                DisplayItem::Image { rect, image } => Some((
                    rect.x,
                    rect.y,
                    rect.w,
                    rect.h,
                    image.width(),
                    image.height(),
                )),
                _ => None,
            })
            .collect();
        // La hauteur suit les proportions du `viewBox` ; le rendu se fait à
        // deux pixels par pixel CSS.
        assert_eq!(
            images,
            vec![
                (0.0, 0.0, 100.0, 50.0, 200, 100),
                (0.0, 50.0, 30.0, 20.0, 60, 40)
            ]
        );
        // Le contenu d'un `<svg>` n'est pas mis en page comme du texte.
        assert!(
            !p.display_list()
                .items
                .iter()
                .any(|item| matches!(item, DisplayItem::Glyphs(_))),
            "{:?}",
            p.display_list().items
        );
        let first = doc_node(&p, "a");
        let (_, image) = p.svg_images.get(&first).expect("rendu gardé");
        let data = image.data().data.data();
        assert_eq!(&data[..4], &[0, 128, 0, 255], "couleur courante");
        let far = (199 * 4) as usize;
        assert_eq!(data[far + 3], 0, "le reste est transparent");
        // Rien n'a changé : le rendu est gardé tel quel.
        let before = data.as_ptr();
        p.update(&mut text, VIEWPORT);
        let (_, image) = p.svg_images.get(&first).expect("rendu gardé");
        assert_eq!(image.data().data.data().as_ptr(), before);
    }

    #[test]
    fn svg_paint_properties_cascade_and_are_read_back() {
        let mut p = page(
            "<body><style>.i { fill: currentColor; stroke: url(#g) red; stroke-width: 2px }</style>
             <svg id=s class=i style='color:rgb(1,2,3)' width=30 height=20>
               <rect id=r width=5 height=5 stroke-dasharray='4 2' fill-rule=evenodd /></svg>
             <script>
               const cs = getComputedStyle(document.getElementById('r'));
               const box = document.getElementById('s').getBoundingClientRect();
               document.body.setAttribute('data-out', [cs.fill, cs.stroke, cs.strokeWidth,
                 cs.strokeDasharray, cs.fillRule, cs.strokeLinecap, cs.fillOpacity,
                 box.width, box.height].join('|'));
             </script>",
        );
        p.run_scripts();
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        assert_eq!(
            doc.element(body).and_then(|e| e.attr("data-out")),
            Some("rgb(1, 2, 3)|url(\"#g\") rgb(255, 0, 0)|2px|4px, 2px|evenodd|butt|1|30|20")
        );
    }

    #[test]
    fn frames_are_painted_inside_their_content_box() {
        use oxyde_paint::DisplayItem;
        let mut text = TextContext::new();
        let mut p = page(
            "<body style='margin:0'><div style='height:20px'></div>
             <iframe style='border:5px solid black;margin-left:30px;width:100px;height:60px;display:block'
               srcdoc=\"<body style='margin:0'><div style='width:40px;height:10px;background:rgb(1,2,3)'></div>
                 <script>document.body.firstChild.style.height = '25px';</script>\"></iframe>
             <iframe style='display:none' srcdoc=\"<p style='background:rgb(9,9,9)'>caché</p>\"></iframe>",
        );
        p.run_scripts();
        p.load_ready_frames();
        p.update(&mut text, VIEWPORT);
        let items = &p.display_list().items;
        // Le contenu du cadre est découpé à sa boîte de contenu et décalé
        // jusqu'à elle ; sa mise en page est celle du document après ses
        // scripts.
        let clip = items
            .iter()
            .position(|item| {
                matches!(item, DisplayItem::PushClip { rect, .. }
                    if (rect.x, rect.y, rect.w, rect.h) == (35.0, 25.0, 100.0, 60.0))
            })
            .expect("découpe du cadre");
        assert!(
            matches!(items.get(clip + 1), Some(DisplayItem::PushTransform { matrix })
                if matrix[4..] == [35.0, 25.0]),
            "{:?}",
            items.get(clip + 1)
        );
        let inner = Color::rgb(1, 2, 3);
        assert!(
            items[clip..].iter().any(|item| {
                matches!(item, DisplayItem::Rect { rect, color, .. }
                    if *color == inner && (rect.x, rect.y, rect.w, rect.h) == (0.0, 0.0, 40.0, 25.0))
            }),
            "{items:?}"
        );
        // Un cadre sans boîte n'est pas dessiné.
        let hidden = Color::rgb(9, 9, 9);
        assert!(
            !items
                .iter()
                .any(|item| matches!(item, DisplayItem::Rect { color, .. } if *color == hidden))
        );
    }

    #[test]
    fn windows_post_messages_to_each_other() {
        let mut p = page(
            "<body><iframe id=cadre srcdoc=\"<script>
                 addEventListener('message', (e) => {
                   // Le cadre répond à qui lui a écrit.
                   e.source.postMessage({ echo: e.data, vu: e.source === parent, de: e.origin }, '*');
                 });
               </script>\"></iframe>
             <script>
               const out = [];
               const show = () => document.body.setAttribute('data-out', out.join(' '));
               const cadre = document.getElementById('cadre');
               addEventListener('message', (e) => {
                 if (e.source === window) out.push('soi:' + e.data);
                 else out.push('cadre:' + JSON.stringify(e.data) + ':' + (e.source === cadre.contentWindow)
                   + ':' + (e.data instanceof Object));
                 show();
               });
               addEventListener('load', () => {
                 cadre.contentWindow.postMessage('salut', '*');
                 postMessage('local', location.origin);
                 // Origine cible qui n'est pas celle de la fenêtre visée : ignoré.
                 cadre.contentWindow.postMessage('perdu', 'https://ailleurs.test');
                 try { postMessage('x', 'pas une origine'); } catch (e) { out.push(e.name); }
                 show();
               });
             </script>",
        );
        p.run_scripts();
        for round in 1..6 {
            let _ = p.tick(Instant::now() + Duration::from_secs(round));
        }
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        let out = doc
            .element(body)
            .and_then(|e| e.attr("data-out"))
            .unwrap_or_default()
            .to_owned();
        assert!(out.starts_with("SyntaxError"), "{out}");
        assert!(out.contains("soi:local"), "{out}");
        assert!(
            out.contains(
                "cadre:{\"echo\":\"salut\",\"vu\":true,\"de\":\"https://exemple.test\"}:true:true"
            ),
            "{out}"
        );
        assert!(!out.contains("perdu"), "{out}");
    }

    #[test]
    fn constructed_sheets_are_adopted_and_measures_take_options() {
        let mut text = TextContext::new();
        let mut p = page(
            "<body><p id=p>texte</p><div id=hote></div>
               <script>
                 const out = [];
                 // Le rappel d'attribut part pendant `setAttribute`, une seule fois.
                 const seen = [];
                 class Guarded extends HTMLElement {
                   static observedAttributes = ['a'];
                   attributeChangedCallback(name, before, after) {
                     seen.push(name + ':' + before + '>' + after + ':' + this.busy);
                   }
                   // Un accesseur de la classe passe avant l'attribut reflété.
                   get href() { return 'classe ' + this.kept; }
                   set href(value) { this.kept = value; }
                 }
                 customElements.define('x-guarded', Guarded);
                 const guarded = document.createElement('x-guarded');
                 document.body.append(guarded);
                 guarded.busy = true;
                 guarded.setAttribute('a', '1');
                 guarded.busy = false;
                 guarded.href = 'u';
                 out.push(guarded.href, guarded.hasAttribute('href'), guarded.localName);
                 out.push(seen.length, [...guarded.attributes].map((a) => a.name).join(','));
                 out.push([...document.body.style].length);
                 const sheet = new CSSStyleSheet();
                 sheet.replaceSync('@import url(x.css); p { color: rgb(1, 2, 3) } /* } */ a[title=\"}\"] { top: 0 }');
                 out.push(sheet.cssRules.length, sheet.cssRules[0].cssText);
                 document.adoptedStyleSheets = [sheet];
                 const p = document.getElementById('p');
                 out.push(getComputedStyle(p).color);
                 sheet.insertRule('p { color: rgb(4, 5, 6) }', sheet.cssRules.length);
                 out.push(getComputedStyle(p).color);
                 const root = document.getElementById('hote').attachShadow({ mode: 'open' });
                 root.innerHTML = '<p id=q>dedans</p>';
                 const inner = new CSSStyleSheet();
                 root.adoptedStyleSheets.push(inner);
                 inner.replace('p { color: rgb(7, 8, 9) }').then((same) => {
                   out.push(same === inner, getComputedStyle(root.getElementById('q')).color);
                   document.adoptedStyleSheets = [];
                   out.push(getComputedStyle(p).color, root.adoptedStyleSheets.length);
                   try { document.adoptedStyleSheets = [{}]; } catch (e) { out.push(e.name); }
                   try { sheet.insertRule('p {}', 9); } catch (e) { out.push(e.name); }
                   performance.mark('a', { startTime: 10 });
                   const m = performance.measure('m', { start: 'a', duration: 5, detail: { n: 1 } });
                   out.push(m.startTime, m.duration, m.detail.n);
                   out.push(performance.measure('n', { start: 2, end: 9 }).duration);
                   out.push(performance.measure('o', {}, 'a').duration);
                   try { performance.measure('o', { detail: 1 }); } catch (e) { out.push(e.name); }
                   guarded.removeAttribute('a');
                   out.push(/^\\p{RGI_Emoji}$/v.test('\\u{1F468}\\u200D\\u{1F469}\\u200D\\u{1F467}'));
                   out.push(/^\\p{RGI_Emoji}$/v.test('\\u{1F1EB}\\u{1F1F7}'), /^\\p{RGI_Emoji}$/v.test('\\u{1F1FD}\\u{1F1FD}'));
                 }).then(() => Promise.resolve()).then(() => {
                   out.push(seen.join(','));
                   document.documentElement.setAttribute('data-out', out.join(' | '));
                 });
               </script>
             </body>",
        );
        p.run_scripts();
        p.update(&mut text, VIEWPORT);
        let doc = p.document();
        let html = doc.find_html_element(doc.root(), "html").expect("html");
        assert_eq!(
            doc.element(html).and_then(|e| e.attr("data-out")),
            Some(
                "classe u | false | x-guarded | 1 | a | 0 | 2 | p { color: rgb(1, 2, 3) } | rgb(1, 2, 3) | rgb(4, 5, 6) | true | \
                 rgb(7, 8, 9) | rgb(0, 0, 0) | 1 | TypeError | IndexSizeError | 10 | 5 | 1 | 7 | 10 | \
                 TypeError | true | true | false | a:null>1:true,a:1>null:false"
            )
        );
    }

    #[test]
    fn canvas_shadows_are_cast_blurred_and_offset() {
        let mut p = page(
            "<body><canvas id=c width=60 height=40></canvas><script>
               const out = [];
               const ctx = document.getElementById('c').getContext('2d');
               const at = (x, y) => [...ctx.getImageData(x, y, 1, 1).data].join(',');
               // Ombre nette, décalée : le rouge du carré, le bleu de son ombre.
               ctx.shadowColor = 'blue';
               ctx.shadowOffsetX = 20;
               ctx.shadowOffsetY = 10;
               ctx.fillStyle = 'red';
               ctx.fillRect(5, 5, 10, 10);
               out.push(at(10, 10), at(30, 20), at(50, 35));
               // Le carré passe devant son ombre ; le décalage ignore la transformation.
               ctx.clearRect(0, 0, 60, 40);
               ctx.shadowOffsetX = 2;
               ctx.shadowOffsetY = 2;
               ctx.save();
               ctx.scale(2, 2);
               ctx.fillRect(5, 5, 5, 5);
               ctx.restore();
               out.push(at(15, 15), at(21, 21), at(25, 25));
               // Ombre floue : elle s'étale et s'estompe autour du carré.
               ctx.clearRect(0, 0, 60, 40);
               ctx.shadowOffsetX = 0;
               ctx.shadowOffsetY = 0;
               ctx.shadowBlur = 8;
               ctx.fillRect(25, 15, 10, 10);
               const near = ctx.getImageData(22, 20, 1, 1).data;
               const far = ctx.getImageData(19, 20, 1, 1).data;
               out.push(near[2] > 250 && near[0] === 0 && near[3] > far[3] && far[3] > 0, at(2, 2));
               // Sans couleur d'ombre, ou sans flou ni décalage : pas d'ombre.
               ctx.clearRect(0, 0, 60, 40);
               ctx.shadowBlur = 0;
               ctx.fillRect(5, 5, 10, 10);
               out.push(at(17, 10));
               ctx.shadowOffsetX = 10;
               ctx.shadowColor = 'transparent';
               ctx.fillRect(5, 25, 10, 10);
               out.push(at(22, 30), ctx.shadowColor);
               // Une forme hors de la surface y porte quand même son ombre.
               ctx.clearRect(0, 0, 60, 40);
               ctx.shadowColor = 'blue';
               ctx.shadowOffsetX = 0;
               ctx.shadowOffsetY = 40;
               ctx.fillRect(0, -40, 10, 10);
               out.push(at(5, 5), at(5, 15));
               document.body.setAttribute('data-out', out.join(' '));
             </script>",
        );
        p.run_scripts();
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        assert_eq!(
            doc.element(body).and_then(|e| e.attr("data-out")),
            Some(
                "255,0,0,255 0,0,255,255 0,0,0,0 255,0,0,255 0,0,255,255 0,0,0,0 true 0,0,0,0 \
                 0,0,0,0 0,0,0,0 rgba(0, 0, 0, 0) 0,0,255,255 0,0,0,0"
            )
        );
    }

    #[test]
    fn canvas_text_is_measured_and_drawn() {
        let mut p = page(
            "<body><canvas id=c width=200 height=60></canvas><script>
               const out = [];
               const ctx = document.getElementById('c').getContext('2d');
               // Nombre de pixels non transparents d'un rectangle.
               const inked = (x, y, w, h) => {
                 const data = ctx.getImageData(x, y, w, h).data;
                 let count = 0;
                 for (let i = 3; i < data.length; i += 4) if (data[i] > 0) count++;
                 return count;
               };
               ctx.font = '20px Inter';
               const one = ctx.measureText('Oxyde');
               const two = ctx.measureText('Oxyde Oxyde');
               out.push(one instanceof TextMetrics, one.width > 20, two.width > one.width * 1.9);
               out.push(one.actualBoundingBoxAscent > one.actualBoundingBoxDescent);
               ctx.fillText('Oxyde', 10, 30);
               // L'encre est au-dessus de la ligne de base, à droite du point d'ancrage.
               out.push(inked(10, 5, 80, 26) > 100, inked(10, 36, 80, 20), inked(110, 0, 90, 60));
               // Aligné à droite : l'encre passe à gauche du point.
               ctx.clearRect(0, 0, 200, 60);
               ctx.textAlign = 'right';
               ctx.textBaseline = 'top';
               ctx.fillStyle = 'red';
               ctx.fillText('Oxyde', 190, 30);
               out.push(inked(100, 30, 90, 30) > 100, inked(100, 0, 100, 29), inked(0, 0, 60, 60));
               const [r, g, b] = (() => {
                 const data = ctx.getImageData(100, 30, 90, 30).data;
                 for (let i = 0; i < data.length; i += 4) if (data[i + 3] === 255) return data.slice(i, i + 3);
                 return [0, 0, 0];
               })();
               out.push(r + ',' + g + ',' + b);
               // Contour, largeur maximale, texte vide.
               ctx.clearRect(0, 0, 200, 60);
               ctx.textAlign = 'start';
               ctx.strokeText('Oxyde', 10, 10);
               const outlined = inked(0, 0, 200, 60);
               ctx.clearRect(0, 0, 200, 60);
               ctx.fillText('Oxyde Oxyde Oxyde', 10, 10, 50);
               out.push(outlined > 50, inked(0, 0, 61, 60) > 50, inked(62, 0, 138, 60));
               ctx.clearRect(0, 0, 200, 60);
               ctx.fillText('', 10, 10);
               ctx.fillText('x', NaN, 10);
               out.push(inked(0, 0, 200, 60), ctx.measureText('').width);
               document.body.setAttribute('data-out', out.join(' '));
             </script>",
        );
        p.run_scripts();
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        assert_eq!(
            doc.element(body).and_then(|e| e.attr("data-out")),
            Some("true true true true true 0 0 true 0 0 255,0,0 true true 0 0 0")
        );
    }

    #[test]
    fn a_canvas_draws_reads_back_and_shows_in_the_page() {
        let mut text = TextContext::new();
        let mut p = page(
            "<body style='margin:0'><canvas id=c width=40 height=20></canvas><script>
               const out = [];
               const c = document.getElementById('c');
               const ctx = c.getContext('2d');
               const at = (x, y, context = ctx) => [...context.getImageData(x, y, 1, 1).data].join(',');
               out.push(ctx === c.getContext('2d'), c.getContext('webgl'), ctx.canvas === c);
               out.push(at(5, 5));
               ctx.fillStyle = 'red';
               ctx.fillRect(0, 0, 20, 20);
               out.push(ctx.fillStyle, at(5, 5), at(25, 5));
               // Transformation, tracé, remplissage semi-transparent.
               ctx.save();
               ctx.translate(20, 0);
               ctx.fillStyle = 'rgba(0, 0, 255, 0.5)';
               ctx.beginPath();
               ctx.moveTo(0, 0); ctx.lineTo(20, 0); ctx.lineTo(20, 20); ctx.lineTo(0, 20);
               ctx.fill();
               ctx.restore();
               out.push(ctx.fillStyle, at(30, 10));
               // Découpe : seul le haut reçoit le vert.
               ctx.save();
               ctx.beginPath();
               ctx.rect(0, 0, 40, 5);
               ctx.clip();
               ctx.fillStyle = '#0f0';
               ctx.fillRect(0, 0, 40, 20);
               ctx.restore();
               out.push(at(5, 2), at(5, 10));
               // Effacement, puis dégradé linéaire du noir au blanc.
               ctx.clearRect(0, 10, 40, 10);
               out.push(at(5, 15));
               const gradient = ctx.createLinearGradient(0, 0, 40, 0);
               gradient.addColorStop(0, '#000');
               gradient.addColorStop(1, '#fff');
               ctx.fillStyle = gradient;
               ctx.fillRect(0, 10, 40, 10);
               const [left] = ctx.getImageData(1, 15, 1, 1).data;
               const [right] = ctx.getImageData(38, 15, 1, 1).data;
               out.push(left < 30, right > 225);
               // Contour, cercle, point dans le tracé.
               ctx.beginPath();
               ctx.arc(10, 10, 5, 0, 2 * Math.PI);
               out.push(ctx.isPointInPath(10, 10), ctx.isPointInPath(30, 10));
               // Écriture de pixels, canevas hors page, image copiée.
               const pixels = ctx.createImageData(2, 1);
               pixels.data.set([1, 2, 3, 255, 0, 0, 0, 0]);
               ctx.putImageData(pixels, 0, 0);
               out.push(at(0, 0), at(1, 0));
               const off = new OffscreenCanvas(4, 4);
               const octx = off.getContext('2d');
               octx.fillStyle = 'yellow';
               octx.fillRect(0, 0, 4, 4);
               ctx.drawImage(off, 30, 0);
               out.push(at(31, 1), octx instanceof OffscreenCanvasRenderingContext2D);
               out.push(c.toDataURL().slice(0, 30), new Path2D('M0 0h10v10z') instanceof Path2D);
               try { ctx.arc(0, 0, -1, 0, 1); } catch (e) { out.push(e.name); }
               ctx.fillStyle = 'pas une couleur';
               out.push(typeof ctx.fillStyle);
               c.width = 40;
               out.push(at(5, 2));
               ctx.fillStyle = '#123456';
               ctx.fillRect(0, 0, 40, 20);
               createImageBitmap(c, 0, 0, 2, 2).then((bitmap) => {
                 out.push(bitmap.width + 'x' + bitmap.height);
                 document.body.setAttribute('data-out', out.join(' '));
               });
             </script>",
        );
        p.run_scripts();
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        assert_eq!(
            doc.element(body).and_then(|e| e.attr("data-out")),
            Some(
                "true  true 0,0,0,0 #ff0000 255,0,0,255 0,0,0,0 #ff0000 0,0,255,128 \
                 0,255,0,255 255,0,0,255 0,0,0,0 true true true false 1,2,3,255 0,0,0,0 \
                 255,255,0,255 true data:image/png;base64,iVBORw0K true IndexSizeError object 0,0,0,0 2x2"
            )
        );
        // Le canevas s'affiche dans la page : son image est peinte.
        let before = p.display_list().items.len();
        assert!(p.update(&mut text, VIEWPORT) || before > 0);
        let canvas = doc_node(&p, "c");
        assert!(
            p.canvas_images
                .get(&canvas)
                .is_some_and(|image| image.width() == 40)
        );
    }

    #[test]
    fn subtle_crypto_hashes_signs_encrypts_and_derives() {
        let mut p = page(
            "<body><script>
               const out = [];
               const hex = (buffer) => [...new Uint8Array(buffer)].map((b) => b.toString(16).padStart(2, '0')).join('');
               const text = (s) => new TextEncoder().encode(s);
               const bytes = (h) => new Uint8Array(h.match(/../g).map((x) => parseInt(x, 16)));
               const name = (e) => e.name;
               (async () => {
                 const s = crypto.subtle;
                 out.push(hex(await s.digest('sha-256', text('abc'))).slice(0, 16));
                 out.push(hex(await s.digest({ name: 'SHA-1' }, text('abc'))).slice(0, 8));
                 // HMAC : RFC 4231, cas 2.
                 const mac = await s.importKey('raw', text('Jefe'), { name: 'HMAC', hash: 'SHA-256' }, true, ['sign', 'verify']);
                 const signature = await s.sign('HMAC', mac, text('what do ya want for nothing?'));
                 out.push(hex(signature).slice(0, 16), mac.type, mac.algorithm.length, mac.usages.join('+'));
                 out.push(await s.verify('HMAC', mac, signature, text('what do ya want for nothing?')));
                 out.push(await s.verify('HMAC', mac, signature, text('autre chose')));
                 const jwk = await s.exportKey('jwk', mac);
                 out.push(jwk.kty + ':' + jwk.k + ':' + jwk.alg);
                 // AES-GCM : aller et retour, altération, clé engendrée.
                 const aes = await s.importKey('raw', new Uint8Array(16), 'AES-GCM', false, ['encrypt', 'decrypt']);
                 const iv = new Uint8Array(12);
                 const sealed = await s.encrypt({ name: 'AES-GCM', iv }, aes, new Uint8Array(16));
                 out.push(hex(sealed).slice(0, 16), sealed.byteLength);
                 out.push(hex(await s.decrypt({ name: 'AES-GCM', iv }, aes, sealed)).length);
                 const altered = new Uint8Array(sealed); altered[0] ^= 1;
                 out.push(await s.decrypt({ name: 'AES-GCM', iv }, aes, altered).catch(name));
                 out.push(await s.exportKey('raw', aes).catch(name));
                 const fresh = await s.generateKey({ name: 'AES-GCM', length: 256 }, true, ['encrypt']);
                 out.push((await s.exportKey('raw', fresh)).byteLength);
                 out.push(await s.decrypt({ name: 'AES-GCM', iv }, fresh, sealed).catch(name));
                 // PBKDF2 (vecteur courant, HMAC-SHA-256, une itération) et HKDF (RFC 5869, cas 1).
                 const password = await s.importKey('raw', text('password'), 'PBKDF2', false, ['deriveBits', 'deriveKey']);
                 const stretched = await s.deriveBits({ name: 'PBKDF2', hash: 'SHA-256', salt: text('salt'), iterations: 1 }, password, 256);
                 out.push(hex(stretched).slice(0, 16));
                 const ikm = await s.importKey('raw', bytes('0b'.repeat(22)), 'HKDF', false, ['deriveBits']);
                 const okm = await s.deriveBits({ name: 'HKDF', hash: 'SHA-256', salt: bytes('000102030405060708090a0b0c'), info: bytes('f0f1f2f3f4f5f6f7f8f9') }, ikm, 336);
                 out.push(hex(okm).slice(0, 16), hex(okm).slice(-8));
                 const derived = await s.deriveKey({ name: 'PBKDF2', hash: 'SHA-256', salt: text('salt'), iterations: 2 }, password, { name: 'AES-GCM', length: 128 }, true, ['encrypt']);
                 out.push(derived.algorithm.name + derived.algorithm.length);
                 // Refus.
                 out.push(await s.digest('MD5', text('x')).catch(name));
                 out.push(await s.generateKey({ name: 'RSA-OAEP' }, true, ['encrypt']).catch(name));
                 out.push(await s.importKey('raw', text('k'), { name: 'HMAC', hash: 'SHA-256' }, true, []).catch(name));
                 out.push(await s.sign('HMAC', aes, text('x')).catch(name));
                 out.push(crypto.subtle instanceof SubtleCrypto, mac instanceof CryptoKey);
               })().catch((e) => out.push('échec ' + e)).finally(() => {
                 document.body.setAttribute('data-out', out.join(' '));
               });
             </script>",
        );
        p.run_scripts();
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        assert_eq!(
            doc.element(body).and_then(|e| e.attr("data-out")),
            Some(
                "ba7816bf8f01cfea a9993e36 5bdcc146bf60754e secret 32 sign+verify true false \
                 oct:SmVmZQ:HS256 0388dace60b6a392 32 32 OperationError InvalidAccessError 32 \
                 InvalidAccessError 120fb6cffcf8b32c 3cb25f25faacd57a 87185865 AES-GCM128 \
                 NotSupportedError NotSupportedError SyntaxError InvalidAccessError true true"
            )
        );
    }

    #[test]
    fn event_sources_parse_their_stream_and_reconnect() {
        let mut p = page(
            "<body><script>
               const out = [];
               const show = () => document.body.setAttribute('data-out', out.join(' '));
               const es = new EventSource('/flux');
               out.push(es.url, es.readyState);
               es.onopen = () => { out.push('open:' + es.readyState); show(); };
               es.onmessage = (e) => { out.push('msg:' + JSON.stringify(e.data) + ':' + e.lastEventId); show(); };
               es.addEventListener('tic', (e) => { out.push('tic:' + e.data + ':' + e.origin); show(); });
               es.onerror = () => { out.push('error:' + es.readyState); show(); };
               const refused = new EventSource('/refus');
               refused.onerror = () => { out.push('refus:' + refused.readyState); show(); };
               show();
             </script>",
        );
        p.run_scripts();
        let commands = p.take_socket_commands();
        let opened: Vec<(u64, String, Vec<String>)> = commands
            .iter()
            .map(|command| match command {
                SocketCommand::Open {
                    id, url, protocols, ..
                } => (*id, url.to_string(), protocols.clone()),
                other => panic!("ouverture attendue : {other:?}"),
            })
            .collect();
        assert_eq!(opened.len(), 2);
        assert_eq!(opened[0].1, "https://exemple.test/flux");
        assert!(opened[0].2.is_empty());
        let (flux, refus) = (opened[0].0, opened[1].0);
        let open = || SocketEvent::Open {
            protocol: String::new(),
        };
        let close = |code: u16| SocketEvent::Close {
            code,
            reason: String::new(),
            clean: code == 1000,
        };
        let _ = p.socket_event(flux, open());
        // Commentaire, événement sur deux lignes de données, caractère et fin
        // de ligne coupés entre deux morceaux.
        let first = ": bonjour\n\ndata: un\ndata:deux\nid: 7\n\nevent: tic\ndata: \u{e9}"
            .as_bytes()
            .to_vec();
        let (start, end) = first.split_at(first.len() - 1);
        let _ = p.socket_event(flux, SocketEvent::Binary(start.to_vec()));
        let mut rest = end.to_vec();
        rest.extend_from_slice(b"t\xC3\xA9\r");
        let _ = p.socket_event(flux, SocketEvent::Binary(rest));
        let _ = p.socket_event(
            flux,
            SocketEvent::Binary(b"\n\r\nretry: 50\ndata: sans fin".to_vec()),
        );
        // Le flux se coupe : l'événement inachevé est perdu, la page se
        // reconnecte après le délai donné par le serveur.
        let _ = p.socket_event(flux, close(1006));
        let _ = p.socket_event(refus, close(404));
        assert!(p.take_socket_commands().is_empty());
        let _ = p.tick(Instant::now() + Duration::from_secs(1));
        let again = p.take_socket_commands();
        let [SocketCommand::Open { id, protocols, .. }] = again.as_slice() else {
            panic!("reconnexion attendue : {again:?}");
        };
        assert_eq!(protocols, &["7".to_owned()]);
        let second = *id;
        let _ = p.socket_event(second, open());
        let _ = p.socket_event(second, SocketEvent::Binary(b"data: revenu\n\n".to_vec()));
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        assert_eq!(
            doc.element(body).and_then(|e| e.attr("data-out")),
            Some(
                "https://exemple.test/flux 0 open:1 msg:\"un\\ndeux\":7 \
                 tic:\u{e9}t\u{e9}:https://exemple.test error:0 refus:2 open:1 msg:\"revenu\":7"
            )
        );
    }

    #[test]
    fn web_sockets_talk_through_the_embedder() {
        let mut p = page(
            "<body><script>
               const out = [];
               const show = () => document.body.setAttribute('data-out', out.join(' '));
               const ws = new WebSocket('wss://exemple.test/canal', ['chat', 'v2']);
               ws.binaryType = 'arraybuffer';
               out.push(ws.url, ws.readyState, WebSocket.OPEN);
               try { ws.send('trop tôt'); } catch (e) { out.push(e.name); }
               ws.onopen = () => {
                 out.push('open:' + ws.protocol + ':' + ws.readyState);
                 ws.send('salut');
                 ws.send(new Uint8Array([1, 2]));
                 show();
               };
               ws.onmessage = (e) => {
                 if (typeof e.data === 'string') out.push('texte:' + e.data);
                 else {
                   out.push('octets:' + new Uint8Array(e.data).join(','));
                   ws.close(1000, 'fini');
                 }
                 show();
               };
               ws.onerror = () => { out.push('error'); show(); };
               ws.onclose = (e) => {
                 out.push('close:' + e.code + ':' + e.reason + ':' + e.wasClean + ':' + ws.readyState);
                 show();
               };
               for (const bad of ['ftp://x/', 'ws://x/#f']) {
                 try { new WebSocket(bad); } catch (e) { out.push(e.name); }
               }
               try { new WebSocket('wss://x/', ['a', 'a']); } catch (e) { out.push(e.name); }
               try { ws.close(1001); } catch (e) { out.push(e.name); }
               const perdu = new WebSocket('wss://exemple.test/perdu');
               perdu.onerror = () => { out.push('perdu:error'); show(); };
               perdu.onclose = (e) => { out.push('perdu:' + e.code + ':' + e.wasClean); show(); };
               show();
             </script>",
        );
        p.run_scripts();
        let commands = p.take_socket_commands();
        assert_eq!(commands.len(), 2);
        let SocketCommand::Open {
            id,
            url,
            document,
            protocols,
        } = &commands[0]
        else {
            panic!("ouverture attendue : {commands:?}");
        };
        assert_eq!(url.as_str(), "wss://exemple.test/canal");
        assert_eq!(document.as_str(), "https://exemple.test/dossier/page.html");
        assert_eq!(protocols, &["chat".to_owned(), "v2".to_owned()]);
        let (id, lost) = (*id, commands[1].id());
        let _ = p.socket_event(
            id,
            SocketEvent::Open {
                protocol: "v2".to_owned(),
            },
        );
        assert_eq!(
            p.take_socket_commands(),
            [
                SocketCommand::Text {
                    id,
                    text: "salut".to_owned()
                },
                SocketCommand::Binary {
                    id,
                    bytes: vec![1, 2]
                },
            ]
        );
        let _ = p.socket_event(id, SocketEvent::Text("écho".to_owned()));
        let _ = p.socket_event(id, SocketEvent::Binary(vec![9, 8]));
        assert_eq!(
            p.take_socket_commands(),
            [SocketCommand::Close {
                id,
                code: Some(1000),
                reason: "fini".to_owned()
            }]
        );
        let _ = p.socket_event(
            id,
            SocketEvent::Close {
                code: 1000,
                reason: "fini".to_owned(),
                clean: true,
            },
        );
        let _ = p.socket_event(
            lost,
            SocketEvent::Close {
                code: 1006,
                reason: String::new(),
                clean: false,
            },
        );
        // Après la fermeture, plus rien n'est remis.
        let _ = p.socket_event(id, SocketEvent::Text("trop tard".to_owned()));
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        assert_eq!(
            doc.element(body).and_then(|e| e.attr("data-out")),
            Some(
                "wss://exemple.test/canal 0 1 InvalidStateError SyntaxError SyntaxError SyntaxError \
                 InvalidAccessError open:v2:1 texte:écho octets:9,8 close:1000:fini:true:3 \
                 perdu:error perdu:1006:false"
            )
        );
    }

    #[test]
    fn workers_run_in_their_own_realm() {
        let mut p = page(
            "<body><script>
               const out = [];
               const done = () => document.body.setAttribute('data-out', out.join(' '));
               const w = new Worker('/w.js', { name: 'calcul' });
               w.onmessage = (e) => {
                 out.push(JSON.stringify(e.data), e.data.liste instanceof Array, e.data.quand instanceof Date);
                 done();
               };
               w.onerror = (e) => { out.push('erreur:' + e.message.includes('boum')); done(); };
               const tampon = new Uint8Array([1, 2, 3]).buffer;
               w.postMessage({ n: 20, tampon, ensemble: new Set([1]) }, [tampon]);
               out.push('détaché:' + tampon.byteLength);
               try { w.postMessage(() => 1); } catch (e) { out.push(e.name); }
               w.postMessage('boum');
               new Worker('/absent.js').onerror = () => { out.push('absent'); done(); };
               const inline = new Worker(URL.createObjectURL(new Blob(['postMessage(typeof window)'])));
               inline.addEventListener('message', (e) => { out.push('blob:' + e.data); done(); });
               const canal = new MessageChannel();
               canal.port2.onmessage = (e) => { out.push('port:' + e.data + ':' + e.ports.length); done(); };
               const autre = new MessageChannel();
               canal.port1.postMessage('via', [autre.port1]);
             </script>",
        );
        p.run_scripts();
        serve(
            &mut p,
            &[
                (
                    "/w.js",
                    "importScripts('lib.js');
                     const local = 2;
                     onmessage = (e) => {
                       if (e.data === 'boum') throw new Error('boum');
                       postMessage({
                         double: doubler(e.data.n) + local,
                         env: [typeof window, typeof document, self instanceof WorkerGlobalScope, name,
                               e.data.ensemble instanceof Set, e.data.tampon.byteLength].join(),
                         liste: [1, 2],
                         quand: new Date(0),
                       });
                     };",
                ),
                ("/lib.js", "const doubler = (n) => n * 2;"),
            ],
        );
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        let out = doc
            .element(body)
            .and_then(|e| e.attr("data-out"))
            .unwrap_or_default()
            .to_owned();
        for expected in [
            "détaché:0",
            "DataCloneError",
            "\"double\":42",
            "undefined,undefined,true,calcul,true,3",
            "true true",
            "erreur:true",
            "absent",
            "blob:undefined",
            "port:via:1",
        ] {
            assert!(out.contains(expected), "« {expected} » manque dans : {out}");
        }
    }

    #[test]
    fn xhr_parses_xml_responses() {
        let mut p = page(
            "<body><script>
               const out = [];
               const ask = (path, kind, done) => {
                 const xhr = new XMLHttpRequest();
                 xhr.open('GET', path);
                 xhr.responseType = kind;
                 xhr.onload = () => { done(xhr); document.body.setAttribute('data-out', out.join(' ')); };
                 xhr.send();
               };
               ask('/a.xml', '', x => out.push('a', x.responseXML.documentElement.localName, typeof x.response));
               ask('/b.xml', '', x => out.push('b', x.responseXML));
               ask('/c.html', 'document', x => out.push('c', x.response.body.textContent, x.response === x.responseXML));
               ask('/c.html', '', x => out.push('d', x.responseXML));
               ask('/e.bin', 'arraybuffer', x => out.push('e', x.response.byteLength));
             </script>",
        );
        p.run_scripts();
        let mut requests = p.take_fetches();
        requests.sort_by_key(|request| request.id);
        for request in requests {
            let (kind, body) = match request.url.path() {
                "/a.xml" => ("application/xml", "<racine><fils/></racine>"),
                "/b.xml" => ("text/xml", "<racine>"),
                "/c.html" => ("text/html", "<p>salut"),
                _ => ("application/octet-stream", "abc"),
            };
            let _ = p.complete_fetch(
                request.id,
                Ok(FetchResponse {
                    url: request.url.clone(),
                    status: 200,
                    headers: vec![("content-type".to_owned(), kind.to_owned())],
                    body: body.as_bytes().to_vec(),
                }),
            );
        }
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        assert_eq!(
            doc.element(body).and_then(|e| e.attr("data-out")),
            Some("a racine string b  c salut true d  e 3")
        );
    }

    #[test]
    fn xml_pages_and_parsers() {
        let url = Url::parse("https://exemple.test/dossier/page.xhtml").expect("url");
        let source = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
            <!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Strict//EN\" \
            \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-strict.dtd\">\n\
            <html xmlns=\"http://www.w3.org/1999/xhtml\"><head><title>t</title></head>\
            <body><p id=\"p\">a&nbsp;b</p><svg xmlns=\"http://www.w3.org/2000/svg\"><rect/></svg>\
            <script><![CDATA[\n\
              const out = [];\n\
              out.push(document.contentType, document instanceof XMLDocument);\n\
              out.push(document.defaultView === window);\n\
              out.push(document.createElement('DIV').localName);\n\
              out.push(document.getElementById('p').textContent.length);\n\
              out.push(document.querySelector('rect').namespaceURI);\n\
              const parser = new DOMParser();\n\
              const doc = parser.parseFromString(\n\
                '<!DOCTYPE a [<!ENTITY e \"<b x=\\'1\\'>gras</b>\">]><a xmlns:p=\"urn:p\"><p:c/>&e;</a>',\n\
                'application/xml');\n\
              out.push(doc.contentType, doc.doctype.name, doc.documentElement.childNodes.length);\n\
              out.push(doc.documentElement.firstChild.namespaceURI);\n\
              out.push(doc.querySelector('b').getAttribute('x'));\n\
              const bad = parser.parseFromString('<a><b></a>', 'text/xml');\n\
              out.push(bad.documentElement.localName, bad.contentType);\n\
              const svg = parser.parseFromString('<svg xmlns=\"http://www.w3.org/2000/svg\"/>', 'image/svg+xml');\n\
              out.push(svg.documentElement.namespaceURI === 'http://www.w3.org/2000/svg');\n\
              document.body.setAttribute('data-out', out.join(' '));\n\
            ]]></script></body></html>";
        let mut p = Page::from_source(url.clone(), source, Some("application/xhtml+xml"));
        p.run_scripts();
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        assert_eq!(
            doc.element(body).and_then(|e| e.attr("data-out")),
            Some(
                "application/xhtml+xml true true DIV 3 http://www.w3.org/2000/svg \
                 application/xml a 2 urn:p 1 parsererror text/xml true"
            )
        );
        // Un document mal formé laisse place à une page qui dit la faute.
        let broken = Page::from_source(url, "<html><p></html>", Some("application/xhtml+xml"));
        let doc = broken.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        assert!(doc.text_content(body).contains("ligne 1"));
        assert_eq!(
            xml_content_type("image/svg+xml; charset=utf-8"),
            Some("image/svg+xml")
        );
        assert_eq!(
            xml_content_type("application/atom+xml"),
            Some("application/xml")
        );
        assert_eq!(xml_content_type("text/html"), None);
    }

    #[test]
    fn forms_validate_their_constraints() {
        let mut p = page(
            "<body><form id=f action='/envoi'>
               <input id=nom required>
               <input id=mel type=email value='pas-un-mel'>
               <input id=age type=number min=18 max=99 step=1 value=12.5>
               <input id=code pattern='[A-Z]{3}' value='abc'>
               <input id=jour type=date min='2026-01-01' value='2025-12-31'>
               <select id=choix required><option value=''>Choisir</option><option value=a>A</option></select>
               <input id=cache type=hidden required><button id=b>ok</button>
             </form><script>
               const out = [];
               const $ = (id) => document.getElementById(id);
               const flaws = (id) => Object.keys(ValidityState.prototype)
                 .filter((k) => k !== 'valid' && $(id).validity[k]).join('+') || 'valide';
               out.push(flaws('nom'), flaws('mel'), flaws('age'), flaws('code'), flaws('jour'), flaws('choix'));
               out.push($('cache').willValidate, flaws('cache'), $('nom').validity === $('nom').validity);
               let invalid = 0;
               $('f').addEventListener('invalid', () => { invalid += 1; }, true);
               let sent = 0;
               $('f').addEventListener('submit', (e) => { sent += 1; e.preventDefault(); });
               out.push($('f').checkValidity(), invalid, $('nom').matches(':invalid'), $('f').matches(':invalid'));
               $('f').requestSubmit();
               out.push(sent, document.activeElement === $('nom'));
               $('nom').value = 'Ada';
               $('mel').value = 'ada@exemple.test';
               $('age').value = '30';
               $('code').value = 'ABC';
               $('jour').value = '2026-02-01';
               $('choix').lastElementChild.setAttribute('selected', '');
               out.push($('f').checkValidity(), $('nom').matches(':valid'));
               $('code').setCustomValidity('refusé');
               out.push($('code').validationMessage, $('code').validity.customError, $('f').reportValidity());
               $('code').setCustomValidity('');
               $('f').requestSubmit();
               out.push(sent);
               document.body.setAttribute('data-out', out.join(' '));
             </script>",
        );
        p.run_scripts();
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        assert_eq!(
            doc.element(body).and_then(|e| e.attr("data-out")),
            Some(
                "valueMissing typeMismatch rangeUnderflow+stepMismatch patternMismatch rangeUnderflow \
                 valueMissing false valide true false 6 true true 0 true true true refusé true false 1"
            )
        );
    }

    #[test]
    fn dialogs_and_popovers_open_and_close() {
        let mut p = page(
            "<body><button id=avant>avant</button>
             <dialog id=d><form method=dialog><input id=champ autofocus><button id=oui value=accord>oui</button></form></dialog>
             <button id=inv popovertarget=p>ouvrir</button><div id=p popover>bulle</div>
             <div id=m popover=manual>fixe</div>
             <script>
               const out = [];
               const note = (...items) => {
                 out.push(...items);
                 document.body.setAttribute('data-out', out.join(' '));
               };
               const $ = (id) => document.getElementById(id);
               const d = $('d');
               $('avant').focus();
               note(getComputedStyle(d).display, d.matches(':modal'));
               d.showModal();
               note(d.open, d.matches(':modal'), d.matches(':open'), getComputedStyle(d).position,
                 document.activeElement.id);
               try { d.show(); } catch (e) { note(e.name); }
               d.addEventListener('close', () => note('close:' + d.returnValue));
               $('oui').click();
               note(d.open, d.matches(':modal'), document.activeElement.id);
               const p = $('p');
               note(getComputedStyle(p).display, p.popover, $('m').popover);
               p.addEventListener('beforetoggle', (e) => note(e.oldState + '>' + e.newState));
               $('inv').click();
               note(p.matches(':popover-open'), getComputedStyle(p).display !== 'none');
               $('m').showPopover();
               document.body.click();
               note(p.matches(':popover-open'), $('m').matches(':popover-open'));
               try { $('avant').showPopover(); } catch (e) { note(e.name); }
               note($('m').togglePopover());
             </script>",
        );
        p.run_scripts();
        for _ in 0..4 {
            let Some(due) = p.next_wake() else { break };
            p.tick(due);
        }
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        assert_eq!(
            doc.element(body).and_then(|e| e.attr("data-out")),
            Some(
                "none false true true true fixed champ InvalidStateError false false avant \
                 none auto manual closed>open true true open>closed false true NotSupportedError false \
                 close:accord"
            )
        );
    }

    #[test]
    fn document_write_inserts_after_the_running_script() {
        let mut p = page(
            "<head><script>document.write('<meta name=ecrit content=oui><p id=tete>depuis la tête</p>');</script></head>
             <body><div id=avant></div><script id=s>
               document.write('<b id=un>un</b>');
               document.writeln('<i id=deux>deux</i>', '<scr' + 'ipt>window.execute = (window.execute || 0) + 1;</scr' + 'ipt>');
               const out = [];
               const s = document.getElementById('s');
               out.push(s.nextElementSibling.id, s.nextElementSibling.nextElementSibling.id, window.execute);
               out.push(document.querySelector('meta[name=ecrit]').parentNode.localName,
                 document.getElementById('tete').parentNode.localName,
                 document.body.firstElementChild.id);
               out.push(document.open() === document, document.getElementById('avant') !== null);
               document.body.setAttribute('data-out', out.join(' '));
             </script><p id=apres></p></body>",
        );
        p.run_scripts();
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        assert_eq!(
            doc.element(body).and_then(|e| e.attr("data-out")),
            Some("un deux 1 head body tete true true")
        );
    }

    #[test]
    fn document_write_after_load_replaces_the_document() {
        let mut p = page(
            "<body><p id=ancien>ancien</p><script>
               setTimeout(() => {
                 document.write('<title>Neuf</title><p id=neuf>un');
                 document.write('deux</p>');
                 document.close();
                 document.body.setAttribute('data-out', [document.title,
                   document.getElementById('ancien'), document.getElementById('neuf').textContent,
                   document.documentElement.localName].join(' '));
               }, 0);
             </script>",
        );
        p.run_scripts();
        for _ in 0..3 {
            let Some(due) = p.next_wake() else { break };
            p.tick(due);
        }
        let doc = p.document();
        let body = doc.find_html_element(doc.root(), "body").expect("body");
        assert_eq!(
            doc.element(body).and_then(|e| e.attr("data-out")),
            Some("Neuf  undeux html")
        );
    }
}
