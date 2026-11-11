//! Page web d'Oxyde : enchaîne analyse HTML, style, mise en page et peinture.
//!
//! C'est le pendant, côté processus de rendu, de `//content/renderer` dans
//! Chromium : une [`Page`] possède le document et sait produire sa liste
//! d'affichage pour une taille de fenêtre donnée.
//!
//! LIMITES CONNUES : pas de JavaScript, pas d'images, encodage supposé UTF-8,
//! pas de mise en page incrémentale (tout est recalculé à chaque changement).

use oxyde_css::{Color, StyleMap, Stylesheet, Viewport, parse_stylesheet};
use oxyde_dom::{Document, NodeId};
use oxyde_layout::{Fragment, LayoutTree};
use oxyde_paint::DisplayList;
use oxyde_text::TextContext;
pub use url::Url;

/// Feuille de style du document, dans l'ordre du source.
#[derive(Debug)]
enum SheetSlot {
    /// `<style>`
    Inline(Stylesheet),
    /// `<link rel=stylesheet>` ; `None` tant que la ressource n'est pas chargée.
    External { url: Url, sheet: Option<Stylesheet> },
}

/// Une page chargée.
#[derive(Debug)]
pub struct Page {
    url: Url,
    doc: Document,
    sheets: Vec<SheetSlot>,
    styles: StyleMap,
    tree: Option<LayoutTree>,
    list: DisplayList,
    viewport: Viewport,
    /// Vrai si le style ou la taille ont changé depuis la dernière mise en page.
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
            }
        }
        Self {
            url,
            doc,
            sheets,
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

    /// Recalcule style, mise en page et peinture si la fenêtre ou le style ont
    /// changé. Renvoie `true` si la liste d'affichage a été reconstruite.
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
        let tree =
            oxyde_layout::layout_document(&self.doc, &self.styles, text, viewport, &|_| None);
        self.list = oxyde_paint::paint(&tree, viewport);
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

    fn page(html: &str) -> Page {
        Page::from_html(
            Url::parse("https://exemple.test/dossier/page.html").expect("url"),
            html,
        )
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
}
