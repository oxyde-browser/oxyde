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

pub use cors::preflight_allows;

use std::collections::HashMap;
use std::io::Cursor;
use std::time::Instant;

use oxyde_css::{Color, StyleMap, Stylesheet, Viewport, parse_stylesheet};
use oxyde_dom::{Document, NodeId};
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
    /// La requête vise une autre origine et sort des requêtes « simples » :
    /// l'embarqueur doit d'abord envoyer une requête `OPTIONS` munie de
    /// [`FetchRequest::preflight_headers`] et soumettre la réponse à
    /// [`preflight_allows`].
    pub needs_preflight: bool,
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

/// Décode une image PNG, JPEG, WebP, GIF, BMP ou ICO.
///
/// Les dimensions et la mémoire sont bornées : une image hostile (« bombe de
/// décompression ») est refusée plutôt que décodée. Renvoie `None` si le
/// format est inconnu, les données invalides ou les limites dépassées.
#[must_use]
pub fn decode_image(bytes: &[u8]) -> Option<Image> {
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
#[derive(Debug)]
enum SheetSlot {
    /// `<style>`
    Inline(Stylesheet),
    /// `<link rel=stylesheet>` ; `None` tant que la ressource n'est pas chargée.
    External { url: Url, sheet: Option<Stylesheet> },
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

/// Une page chargée.
#[derive(Debug)]
pub struct Page {
    url: Url,
    /// Adresse de base de résolution des liens (`<base href>` ou l'adresse de la page).
    base: Url,
    doc: Document,
    sheets: Vec<SheetSlot>,
    images: HashMap<NodeId, ImageSlot>,
    scripts: Vec<ScriptSlot>,
    /// Modules importés par les scripts de type module, par adresse ; `None` :
    /// module indisponible.
    modules: HashMap<Url, Option<String>>,
    engine: Option<Box<ScriptEngine>>,
    styles: StyleMap,
    tree: Option<LayoutTree>,
    list: DisplayList,
    viewport: Viewport,
    /// Vrai si le style, les images, le DOM ou la taille ont changé depuis la dernière mise en page.
    dirty: bool,
}

fn attr_has_token(value: &str, token: &str) -> bool {
    value
        .split_ascii_whitespace()
        .any(|t| t.eq_ignore_ascii_case(token))
}

/// Vrai si l'attribut `type` d'un `<script>` désigne un script classique.
fn is_classic_script(kind: Option<&str>) -> bool {
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

/// Vrai si l'attribut `type` d'un `<script>` désigne un module.
fn is_module_script(kind: Option<&str>) -> bool {
    kind.is_some_and(|k| k.trim().eq_ignore_ascii_case("module"))
}

/// Nombre maximal de modules importés par une page.
const MAX_MODULES: usize = 512;

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
    for node in doc.descendants(doc.root()) {
        let Some(el) = doc.element(node) else {
            continue;
        };
        if el.is_html("style") {
            sheets.push(SheetSlot::Inline(parse_stylesheet(&doc.text_content(node))));
        } else if el.is_html("link")
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
        let doc = oxyde_html::parse_document(html);
        let base = doc
            .find_html_element(doc.root(), "base")
            .and_then(|n| doc.element(n)?.attr("href").and_then(|h| url.join(h).ok()))
            .unwrap_or_else(|| url.clone());
        let sheets = collect_sheets(&doc, &base, Vec::new());
        let mut images = HashMap::new();
        let mut scripts = Vec::new();
        for node in doc.descendants(doc.root()) {
            let Some(el) = doc.element(node) else {
                continue;
            };
            if el.is_html("img")
                && images.len() < MAX_IMAGES
                && let Some(target) = el
                    .attr("src")
                    .filter(|s| !s.trim().is_empty())
                    .and_then(|s| base.join(s.trim()).ok())
            {
                images.insert(
                    node,
                    ImageSlot {
                        url: target,
                        resolved: false,
                        image: None,
                    },
                );
            } else if el.is_html("script")
                && (is_classic_script(el.attr("type")) || is_module_script(el.attr("type")))
                && scripts.len() < MAX_SCRIPTS
            {
                let module = is_module_script(el.attr("type"));
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
            }
        }
        Self {
            url,
            base,
            doc,
            sheets,
            images,
            scripts,
            modules: HashMap::new(),
            engine: None,
            styles: StyleMap::default(),
            tree: None,
            list: DisplayList::default(),
            viewport: Viewport {
                width: 0.0,
                height: 0.0,
            },
            dirty: true,
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

    /// Adresses des scripts externes pas encore chargés, dans l'ordre du document.
    #[must_use]
    pub fn pending_scripts(&self) -> Vec<Url> {
        self.scripts
            .iter()
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
        for slot in &mut self.scripts {
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
                if let Some(url) = resolve_module(&referrer, &specifier)
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
        if self.scripts.is_empty() {
            return;
        }
        let entries: Vec<(String, String)> = self
            .module_entries()
            .into_iter()
            .map(|(id, source)| (id.to_string(), source.to_owned()))
            .collect();
        let modules = std::mem::take(&mut self.modules);
        let scripts = std::mem::take(&mut self.scripts);
        let mut engine = Box::new(ScriptEngine::new(&self.url));
        for slot in &scripts {
            let source = match slot {
                ScriptSlot::Inline(source)
                | ScriptSlot::External {
                    source: Some(source),
                    module: false,
                    ..
                } => source,
                _ => continue,
            };
            engine.eval(&mut self.doc, source);
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
        engine.dispatch(&mut self.doc, None, "load");
        engine.take_dirty();
        self.engine = Some(engine);
        self.dom_changed();
    }

    /// À appeler quand un script a pu modifier le DOM : les feuilles de style
    /// en ligne sont relues et la mise en page invalidée.
    fn dom_changed(&mut self) {
        let previous = std::mem::take(&mut self.sheets);
        self.sheets = collect_sheets(&self.doc, &self.base, previous);
        self.dirty = true;
    }

    /// Messages écrits dans la console par les scripts depuis le dernier appel.
    pub fn take_console(&mut self) -> Vec<ConsoleMessage> {
        self.engine
            .as_mut()
            .map(|e| e.take_console())
            .unwrap_or_default()
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
        let Some(engine) = self.engine.as_mut() else {
            return false;
        };
        engine.run_timers(&mut self.doc, now);
        if engine.take_dirty() {
            self.dom_changed();
            return true;
        }
        false
    }

    /// Instant du prochain minuteur à exécuter, s'il y en a un.
    #[must_use]
    pub fn next_wake(&mut self) -> Option<Instant> {
        self.engine.as_mut()?.next_due()
    }

    /// Clic au point (`x`, `y`) de la page : distribue l'événement `click` aux
    /// scripts, puis renvoie la cible du lien à suivre si l'action par défaut
    /// n'a pas été annulée.
    pub fn click(&mut self, x: f32, y: f32) -> Option<Url> {
        let node = self.node_at(x, y)?;
        let link = self.link_from(node);
        let Some(engine) = self.engine.as_mut() else {
            return link;
        };
        let prevented = engine.dispatch(&mut self.doc, Some(node), "click");
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
        let sheets: Vec<Stylesheet> = self
            .sheets
            .iter()
            .filter_map(|s| match s {
                SheetSlot::Inline(sheet)
                | SheetSlot::External {
                    sheet: Some(sheet), ..
                } => Some(sheet.clone()),
                SheetSlot::External { sheet: None, .. } => None,
            })
            .collect();
        self.styles = oxyde_css::compute_styles(&self.doc, &sheets, viewport);
        let images = &self.images;
        let image_of = |node: NodeId| images.get(&node).and_then(|slot| slot.image.clone());
        #[expect(clippy::cast_precision_loss, reason = "côtés d'image bornés à 16 384")]
        let intrinsic = |node: NodeId| {
            images
                .get(&node)
                .and_then(|slot| slot.image.as_ref())
                .map(|image| (image.width() as f32, image.height() as f32))
        };
        let tree =
            oxyde_layout::layout_document(&self.doc, &self.styles, text, viewport, &intrinsic);
        self.list = oxyde_paint::paint(&tree, viewport, &image_of);
        self.tree = Some(tree);
        true
    }

    /// Liste d'affichage de la dernière mise en page (coordonnées de la page).
    #[must_use]
    pub fn display_list(&self) -> &DisplayList {
        &self.list
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
        oxyde_layout::hit_test(&self.doc, self.root_fragment()?, x, y)
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
        // Les refus immédiats n'atteignent jamais l'embarqueur.
        assert_eq!(body_text(&p), "fichier refusé;TRACE refusé;");
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
            "fichier refusé;TRACE refusé;200 true application/json false;json 1+2;ouvert cors bonjour;\
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
             [object HTMLElement] [object Text] 1 2 0 \
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
        assert!(done >= 235, "couverture en régression : {done} membres");
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
        assert_eq!(
            body_text(&p),
            "fetch : une requête GET ne peut pas avoir de corps"
        );
        let requests = p.take_fetches();
        assert_eq!(requests.len(), 3);
        let pair = |n: &str, v: &str| (n.to_owned(), v.to_owned());
        // Même origine : tout est permis, sans pré-vérification.
        assert_eq!(requests[0].method, "POST");
        assert_eq!(
            requests[0].headers,
            [
                pair("X-Jeton", "1"),
                pair("Content-Type", "application/json")
            ]
        );
        assert_eq!(requests[0].body, b"{}");
        assert_eq!(requests[0].origin.as_deref(), Some("https://exemple.test"));
        assert!(!requests[0].needs_preflight);
        // Autre origine, requête simple : type de contenu par défaut.
        assert_eq!(
            requests[1].headers,
            [pair("Content-Type", "text/plain;charset=UTF-8")]
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
}
