//! Page web d'Oxyde : enchaîne analyse HTML, style, mise en page et peinture.
//!
//! C'est le pendant, côté processus de rendu, de `//content/renderer` dans
//! Chromium : une [`Page`] possède le document et sait produire sa liste
//! d'affichage pour une taille de fenêtre donnée.
//!
//! LIMITES CONNUES : pas de JavaScript, encodage supposé UTF-8, pas de mise
//! en page incrémentale (tout est recalculé à chaque changement), images
//! limitées à `<img src>` (pas de `srcset`, `<picture>` ni d'images CSS),
//! images animées réduites à leur première image.

use std::collections::HashMap;
use std::io::Cursor;

use oxyde_css::{Color, StyleMap, Stylesheet, Viewport, parse_stylesheet};
use oxyde_dom::{Document, NodeId};
use oxyde_layout::{Fragment, LayoutTree};
use oxyde_paint::DisplayList;
pub use oxyde_paint::Image;
use oxyde_text::TextContext;
pub use url::Url;

/// Côté maximal d'une image décodée, en pixels.
const MAX_IMAGE_SIDE: u32 = 16_384;
/// Mémoire maximale allouée pour décoder une image (256 Mio).
const MAX_IMAGE_ALLOC: u64 = 256 * 1024 * 1024;
/// Nombre maximal d'images chargées par page.
const MAX_IMAGES: usize = 500;

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

/// Une page chargée.
#[derive(Debug)]
pub struct Page {
    url: Url,
    doc: Document,
    sheets: Vec<SheetSlot>,
    images: HashMap<NodeId, ImageSlot>,
    styles: StyleMap,
    tree: Option<LayoutTree>,
    list: DisplayList,
    viewport: Viewport,
    /// Vrai si le style, les images ou la taille ont changé depuis la dernière mise en page.
    dirty: bool,
}

fn attr_has_token(value: &str, token: &str) -> bool {
    value
        .split_ascii_whitespace()
        .any(|t| t.eq_ignore_ascii_case(token))
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
        let mut sheets = Vec::new();
        let mut images = HashMap::new();
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
                sheets.push(SheetSlot::External {
                    url: target,
                    sheet: None,
                });
            } else if el.is_html("img")
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
            }
        }
        Self {
            url,
            doc,
            sheets,
            images,
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

    /// Recalcule style, mise en page et peinture si la fenêtre, le style ou
    /// les images ont changé. Renvoie `true` si la liste d'affichage a été reconstruite.
    pub fn update(&mut self, text: &mut TextContext, viewport: Viewport) -> bool {
        if !self.dirty && viewport == self.viewport {
            return false;
        }
        self.viewport = viewport;
        self.dirty = false;
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

    /// Cible du lien situé au point (`x`, `y`) de la page, s'il y en a un.
    #[must_use]
    pub fn link_at(&self, x: f32, y: f32) -> Option<Url> {
        let node = self.node_at(x, y)?;
        std::iter::once(node)
            .chain(self.doc.ancestors(node))
            .find_map(|n| {
                let el = self.doc.element(n)?;
                if !el.is_html("a") {
                    return None;
                }
                self.url.join(el.attr("href")?).ok()
            })
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
        let viewport = Viewport {
            width: 400.0,
            height: 300.0,
        };
        assert!(p.update(&mut text, viewport));
        assert!(!p.update(&mut text, viewport), "rien n'a changé");
        assert_eq!(p.content_height(), 30.0);
        assert_eq!(
            p.link_at(5.0, 15.0).map(String::from).as_deref(),
            Some("https://exemple.test/dossier/suite.html#x")
        );
        assert_eq!(p.link_at(5.0, 200.0), None);

        assert!(p.set_stylesheet(&pending[0], "p { line-height: 50px !important }"));
        assert!(!p.set_stylesheet(&pending[0], ""), "déjà fournie");
        assert!(p.update(&mut text, viewport));
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
            let pixels = vec![200u8; 4 * 3 * 4];
            // Le JPEG n'a pas de canal alpha : on l'encode depuis du RGB.
            let source = if format == Jpeg {
                image::DynamicImage::ImageRgb8(
                    image::RgbImage::from_raw(4, 3, vec![200u8; 4 * 3 * 3]).expect("tampon"),
                )
            } else {
                image::DynamicImage::ImageRgba8(
                    image::RgbaImage::from_raw(4, 3, pixels).expect("tampon"),
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
        let viewport = Viewport {
            width: 400.0,
            height: 300.0,
        };
        p.update(&mut text, viewport);
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
        assert!(p.update(&mut text, viewport));
        // Taille intrinsèque 2×1, puis largeur imposée à 100 avec proportions conservées.
        assert_eq!(images(&p), vec![(2.0, 1.0), (100.0, 50.0)]);
    }
}
