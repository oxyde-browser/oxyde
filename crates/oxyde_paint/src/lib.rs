//! Peinture d'Oxyde.
//!
//! Transforme l'arbre de fragments en une [`DisplayList`] : une suite plate de
//! primitives indépendante du moteur de rastérisation, comme les
//! `PaintOp` de Blink/cc. L'interface du navigateur produit la même liste, ce
//! qui donne un seul chemin de rendu vers le GPU.
//!
//! LIMITES CONNUES : bordures pointillées ou tiretées peintes pleines, pas
//! d'images, d'ombres ni de dégradés, pas d'ordre d'empilement (`z-index`).

pub use kurbo::BezPath;
pub use oxyde_layout::Rect;

use oxyde_css::{Color, Side, Viewport};
use oxyde_layout::{BoxFragment, Fragment, LayoutTree, TextFragment};
use oxyde_text::TextLayout;
use oxyde_text::parley::{FontData, PositionedLayoutItem};

/// Glyphe positionné (origine sur la ligne de base).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlyphPos {
    /// Identifiant du glyphe dans la police.
    pub id: u32,
    /// Abscisse.
    pub x: f32,
    /// Ordonnée de la ligne de base.
    pub y: f32,
}

/// Suite de glyphes d'une même police et d'une même couleur.
#[derive(Clone, Debug)]
pub struct GlyphRun {
    /// Données de la police.
    pub font: FontData,
    /// Taille en pixels.
    pub size: f32,
    /// Couleur.
    pub color: Color,
    /// Glyphes positionnés.
    pub glyphs: Vec<GlyphPos>,
    /// Inclinaison synthétique (italique simulé), en degrés.
    pub skew: Option<f32>,
    /// Coordonnées normalisées de police variable.
    pub coords: Vec<i16>,
}

/// Primitive de la liste d'affichage.
#[derive(Clone, Debug)]
pub enum DisplayItem {
    /// Rectangle plein, coins éventuellement arrondis.
    Rect {
        /// Zone.
        rect: Rect,
        /// Couleur de remplissage.
        color: Color,
        /// Rayon des coins.
        radius: f32,
    },
    /// Contour de rectangle, tracé à l'intérieur de `rect`.
    StrokeRect {
        /// Zone extérieure.
        rect: Rect,
        /// Couleur du trait.
        color: Color,
        /// Épaisseur du trait.
        width: f32,
        /// Rayon des coins extérieurs.
        radius: f32,
    },
    /// Texte.
    Glyphs(GlyphRun),
    /// Chemin vectoriel (icônes de l'interface), dans un repère décalé de (`x`, `y`).
    Path {
        /// Tracé.
        path: BezPath,
        /// Décalage.
        x: f32,
        /// Décalage.
        y: f32,
        /// Couleur.
        color: Color,
        /// `Some(épaisseur)` pour un trait, `None` pour un remplissage.
        stroke: Option<f32>,
    },
    /// Début d'une zone de découpe.
    PushClip {
        /// Zone.
        rect: Rect,
        /// Rayon des coins.
        radius: f32,
    },
    /// Fin de la zone de découpe la plus récente.
    PopClip,
}

/// Liste d'affichage : primitives dans l'ordre de peinture.
#[derive(Clone, Debug, Default)]
pub struct DisplayList {
    /// Primitives.
    pub items: Vec<DisplayItem>,
}

impl DisplayList {
    /// Ajoute un rectangle plein (ignoré s'il est transparent ou vide).
    pub fn rect(&mut self, rect: Rect, color: Color, radius: f32) {
        if color.a > 0 && rect.w > 0.0 && rect.h > 0.0 {
            self.items.push(DisplayItem::Rect {
                rect,
                color,
                radius,
            });
        }
    }

    /// Ajoute les glyphes et décorations de `layout`, dont l'origine est (`x`, `y`).
    pub fn text(&mut self, layout: &TextLayout, x: f32, y: f32) {
        for line in layout.lines() {
            let metrics = *line.metrics();
            for item in line.items() {
                let PositionedLayoutItem::GlyphRun(run) = item else {
                    continue;
                };
                let style = run.style();
                let [r, g, b, a] = style.brush.color;
                let color = Color { r, g, b, a };
                let [br, bg, bb, ba] = style.brush.background;
                self.rect(
                    Rect {
                        x: x + run.offset(),
                        y: y + metrics.block_min_coord,
                        w: run.advance(),
                        h: metrics.block_max_coord - metrics.block_min_coord,
                    },
                    Color {
                        r: br,
                        g: bg,
                        b: bb,
                        a: ba,
                    },
                    0.0,
                );
                let inner = run.run();
                let font_metrics = *inner.font_metrics();
                let glyphs: Vec<GlyphPos> = run
                    .positioned_glyphs()
                    .map(|gl| GlyphPos {
                        id: gl.id,
                        x: x + gl.x,
                        y: y + gl.y,
                    })
                    .collect();
                if !glyphs.is_empty() && color.a > 0 {
                    self.items.push(DisplayItem::Glyphs(GlyphRun {
                        font: inner.font().clone(),
                        size: inner.font_size(),
                        color,
                        glyphs,
                        skew: inner.synthesis().skew(),
                        coords: inner
                            .normalized_coords()
                            .iter()
                            .map(|c| c.to_bits())
                            .collect(),
                    }));
                }
                let mut decoration = |offset: f32, size: f32| {
                    self.rect(
                        Rect {
                            x: x + run.offset(),
                            y: y + run.baseline() - offset,
                            w: run.advance(),
                            h: size.max(1.0),
                        },
                        color,
                        0.0,
                    );
                };
                if let Some(d) = &style.underline {
                    decoration(
                        d.offset.unwrap_or(font_metrics.underline_offset),
                        d.size.unwrap_or(font_metrics.underline_size),
                    );
                }
                if let Some(d) = &style.strikethrough {
                    decoration(
                        d.offset.unwrap_or(font_metrics.strikethrough_offset),
                        d.size.unwrap_or(font_metrics.strikethrough_size),
                    );
                }
            }
        }
    }
}

/// Couleur de fond du canevas : celle de l'élément racine, ou à défaut celle
/// de `<body>` (propagation prévue par CSS Backgrounds §2.11.2), sinon blanc.
#[must_use]
pub fn canvas_color(tree: &LayoutTree) -> Color {
    let Some(Fragment::Box(root)) = &tree.root else {
        return Color::WHITE;
    };
    if root.style.background_color.a > 0 {
        return root.style.background_color;
    }
    root.children
        .iter()
        .find_map(|c| match c {
            Fragment::Box(b) if b.style.background_color.a > 0 => Some(b.style.background_color),
            _ => None,
        })
        .unwrap_or(Color::WHITE)
}

/// Peint la page. Les coordonnées de la liste sont celles de la page (le
/// défilement est appliqué au rendu).
#[must_use]
pub fn paint(tree: &LayoutTree, viewport: Viewport) -> DisplayList {
    let mut list = DisplayList::default();
    list.rect(
        Rect {
            x: 0.0,
            y: 0.0,
            w: tree.width.max(viewport.width),
            h: tree.height.max(viewport.height),
        },
        canvas_color(tree),
        0.0,
    );
    if let Some(root) = &tree.root {
        paint_fragment(&mut list, root);
    }
    list
}

fn paint_fragment(list: &mut DisplayList, fragment: &Fragment) {
    match fragment {
        Fragment::Box(b) => paint_box(list, b),
        Fragment::Text(t) => paint_text(list, t),
    }
}

fn paint_box(list: &mut DisplayList, b: &BoxFragment) {
    let r = b.border_box;
    let style = &b.style;
    list.rect(r, style.background_color, style.border_radius);

    let bw = style.border_widths();
    let colors =
        [Side::Top, Side::Right, Side::Bottom, Side::Left].map(|s| style.border_color_of(s));
    let uniform = bw.top == bw.right
        && bw.top == bw.bottom
        && bw.top == bw.left
        && colors.iter().all(|c| *c == colors[0]);
    if uniform && bw.top > 0.0 {
        if colors[0].a > 0 {
            list.items.push(DisplayItem::StrokeRect {
                rect: r,
                color: colors[0],
                width: bw.top,
                radius: style.border_radius,
            });
        }
    } else {
        list.rect(
            Rect {
                x: r.x,
                y: r.y,
                w: r.w,
                h: bw.top,
            },
            colors[0],
            0.0,
        );
        list.rect(
            Rect {
                x: r.x + r.w - bw.right,
                y: r.y,
                w: bw.right,
                h: r.h,
            },
            colors[1],
            0.0,
        );
        list.rect(
            Rect {
                x: r.x,
                y: r.y + r.h - bw.bottom,
                w: r.w,
                h: bw.bottom,
            },
            colors[2],
            0.0,
        );
        list.rect(
            Rect {
                x: r.x,
                y: r.y,
                w: bw.left,
                h: r.h,
            },
            colors[3],
            0.0,
        );
    }
    for child in &b.children {
        paint_fragment(list, child);
    }
}

fn paint_text(list: &mut DisplayList, t: &TextFragment) {
    list.text(&t.layout, t.x, t.y);
    for child in &t.boxes {
        paint_fragment(list, child);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paint_html(html: &str) -> DisplayList {
        let doc = oxyde_html::parse_document(html);
        let viewport = Viewport {
            width: 400.0,
            height: 300.0,
        };
        let sheets = oxyde_css::collect_style_elements(&doc);
        let styles = oxyde_css::compute_styles(&doc, &sheets, viewport);
        let mut text = oxyde_text::TextContext::new();
        let tree = oxyde_layout::layout_document(&doc, &styles, &mut text, viewport, &|_| None);
        paint(&tree, viewport)
    }

    #[test]
    fn canvas_takes_body_background() {
        let list = paint_html("<body style='background:#0E1116; margin:0'>");
        let DisplayItem::Rect { rect, color, .. } = &list.items[0] else {
            panic!("fond attendu")
        };
        assert_eq!(*color, Color::rgb(0x0e, 0x11, 0x16));
        assert_eq!((rect.w, rect.h), (400.0, 300.0));
    }

    #[test]
    fn boxes_borders_and_text() {
        let list = paint_html(
            "<body style='margin:0'>\
             <div style='background:red; border:2px solid blue; height:10px'></div>\
             <p style='color:green'>texte <u>souligné</u></p>",
        );
        let rects = list
            .items
            .iter()
            .filter(|i| matches!(i, DisplayItem::Rect { .. }))
            .count();
        let strokes: Vec<_> = list
            .items
            .iter()
            .filter_map(|i| match i {
                DisplayItem::StrokeRect { color, width, .. } => Some((*color, *width)),
                _ => None,
            })
            .collect();
        let glyph_runs: Vec<_> = list
            .items
            .iter()
            .filter_map(|i| match i {
                DisplayItem::Glyphs(g) => Some(g),
                _ => None,
            })
            .collect();
        assert_eq!(strokes, vec![(Color::rgb(0, 0, 255), 2.0)]);
        assert!(rects >= 3, "fond du canevas, fond du div, soulignement");
        assert!(glyph_runs.len() >= 2);
        assert!(
            glyph_runs
                .iter()
                .all(|g| g.color == Color::rgb(0, 128, 0) && !g.glyphs.is_empty())
        );
    }
}
