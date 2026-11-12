//! Peinture d'Oxyde.
//!
//! Transforme l'arbre de fragments en une [`DisplayList`] : une suite plate de
//! primitives indépendante du moteur de rastérisation, comme les
//! `PaintOp` de Blink/cc. L'interface du navigateur produit la même liste, ce
//! qui donne un seul chemin de rendu vers le GPU.
//!
//! LIMITES CONNUES : bordures pointillées ou tiretées peintes pleines, pas
//! d'ombres ni de dégradés ; l'ordre d'empilement (`z-index`) est établi
//! entre les enfants directs d'une boîte ; une boîte à `overflow` autre que
//! `visible` découpe son contenu mais ne défile pas.

pub use kurbo::BezPath;
pub use oxyde_layout::Rect;

use oxyde_css::{Color, Overflow, Side, Size, Viewport, Visibility};
use oxyde_dom::NodeId;
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
    /// Début d'un calque translucide : tout ce qui suit, jusqu'au
    /// [`DisplayItem::PopClip`] correspondant, est composé avec `alpha`.
    PushOpacity {
        /// Zone couverte par le calque.
        rect: Rect,
        /// Opacité, de 0 à 1.
        alpha: f32,
    },
    /// Fin de la zone de découpe ou du calque le plus récent.
    PopClip,
    /// Début d'un groupe transformé : tout ce qui suit, jusqu'au
    /// [`DisplayItem::PopTransform`] correspondant, subit la transformation
    /// affine `matrix`, notée comme `matrix(a, b, c, d, e, f)` en CSS.
    PushTransform {
        /// Coefficients de la transformation, en coordonnées de page.
        matrix: [f32; 6],
    },
    /// Fin du groupe transformé le plus récent.
    PopTransform,
    /// Image matricielle étirée dans `rect`.
    Image {
        /// Zone de destination.
        rect: Rect,
        /// Pixels.
        image: Image,
    },
}

/// Image décodée, prête pour le rendu.
///
/// INVARIANT : `data` contient exactement `width × height × 4` octets RGBA
/// (alpha non prémultiplié) ; garanti par [`Image::from_rgba`], seul constructeur.
#[derive(Clone, Debug)]
pub struct Image(peniko::ImageData);

impl Image {
    /// Construit une image à partir de pixels RGBA8. `None` si la taille est
    /// nulle ou si `rgba` n'a pas la longueur attendue.
    #[must_use]
    pub fn from_rgba(width: u32, height: u32, rgba: Vec<u8>) -> Option<Self> {
        let expected = (width as usize)
            .checked_mul(height as usize)?
            .checked_mul(4)?;
        if expected == 0 || rgba.len() != expected {
            return None;
        }
        Some(Self(peniko::ImageData {
            data: peniko::Blob::new(std::sync::Arc::new(rgba)),
            format: peniko::ImageFormat::Rgba8,
            alpha_type: peniko::ImageAlphaType::Alpha,
            width,
            height,
        }))
    }

    /// Largeur en pixels.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.0.width
    }

    /// Hauteur en pixels.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.0.height
    }

    /// Données au format attendu par le moteur de rendu.
    #[must_use]
    pub fn data(&self) -> &peniko::ImageData {
        &self.0
    }
}

/// Image associée à un élément remplacé (`<img>`), si elle est chargée.
pub type ImageSource<'a> = &'a dyn Fn(NodeId) -> Option<Image>;

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
pub fn paint(tree: &LayoutTree, viewport: Viewport, images: ImageSource<'_>) -> DisplayList {
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
        paint_fragment(&mut list, root, images);
    }
    list
}

fn paint_fragment(list: &mut DisplayList, fragment: &Fragment, images: ImageSource<'_>) {
    match fragment {
        Fragment::Box(b) => paint_box(list, b, images),
        Fragment::Text(t) => paint_text(list, t, images),
    }
}

/// Peint une boîte : son fond et ses bordures si elle est visible, puis ses
/// enfants dans l'ordre d'empilement, découpés si elle ne laisse pas
/// déborder son contenu ; le tout dans un calque si elle est translucide,
/// et dans un groupe transformé si elle porte un `transform`.
fn paint_box(list: &mut DisplayList, b: &BoxFragment, images: ImageSource<'_>) {
    let style = &b.style;
    if style.opacity <= 0.0 {
        return;
    }
    let transform = oxyde_layout::box_transform(b);
    if let Some(matrix) = transform {
        list.items
            .push(DisplayItem::PushTransform { matrix: matrix.0 });
    }
    let translucent = style.opacity < 1.0;
    if translucent {
        // Le calque couvre largement la boîte : son contenu peut en déborder.
        let margin = 100_000.0;
        list.items.push(DisplayItem::PushOpacity {
            rect: Rect {
                x: b.border_box.x - margin,
                y: b.border_box.y - margin,
                w: b.border_box.w + 2.0 * margin,
                h: b.border_box.h + 2.0 * margin,
            },
            alpha: style.opacity,
        });
    }
    if style.visibility == Visibility::Visible {
        paint_decorations(list, b, images);
    }
    let clips = style.overflow_x != Overflow::Visible || style.overflow_y != Overflow::Visible;
    if clips {
        // Découpe à la boîte de marge intérieure.
        let bw = style.border_widths();
        list.items.push(DisplayItem::PushClip {
            rect: Rect {
                x: b.border_box.x + bw.left,
                y: b.border_box.y + bw.top,
                w: (b.border_box.w - bw.left - bw.right).max(0.0),
                h: (b.border_box.h - bw.top - bw.bottom).max(0.0),
            },
            radius: (style.border_radius - bw.top).max(0.0),
        });
    }
    for child in oxyde_layout::paint_order(&b.children) {
        paint_fragment(list, child, images);
    }
    if clips {
        list.items.push(DisplayItem::PopClip);
    }
    if translucent {
        list.items.push(DisplayItem::PopClip);
    }
    if transform.is_some() {
        list.items.push(DisplayItem::PopTransform);
    }
}

/// Fond, bordures et image d'une boîte.
fn paint_decorations(list: &mut DisplayList, b: &BoxFragment, images: ImageSource<'_>) {
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
    if let Some(image) = b.node.and_then(images) {
        // Boîte de contenu : bordures et marges internes en pixels retirées.
        let pad = |s: Size| match s {
            Size::Px(v) => v,
            _ => 0.0,
        };
        let (left, top) = (
            bw.left + pad(style.padding.left),
            bw.top + pad(style.padding.top),
        );
        let rect = Rect {
            x: r.x + left,
            y: r.y + top,
            w: (r.w - left - bw.right - pad(style.padding.right)).max(0.0),
            h: (r.h - top - bw.bottom - pad(style.padding.bottom)).max(0.0),
        };
        if rect.w > 0.0 && rect.h > 0.0 {
            // `border-radius` découpe aussi le contenu d'un élément remplacé.
            let rounded = style.border_radius > 0.0;
            if rounded {
                list.items.push(DisplayItem::PushClip {
                    rect: r,
                    radius: style.border_radius,
                });
            }
            list.items.push(DisplayItem::Image { rect, image });
            if rounded {
                list.items.push(DisplayItem::PopClip);
            }
        }
    }
}

fn paint_text(list: &mut DisplayList, t: &TextFragment, images: ImageSource<'_>) {
    list.text(&t.layout, t.x, t.y);
    for child in &t.boxes {
        paint_fragment(list, child, images);
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
        paint(&tree, viewport, &|_| None)
    }

    #[test]
    fn stacking_order_clipping_and_opacity() {
        let list = paint_html(
            "<style>body{margin:0}
               #c{position:relative; overflow:hidden; width:50px; height:50px; opacity:0.5}
               #c div{width:10px; height:10px}
               #front{position:absolute; z-index:2; background:#030303}
               #back{position:absolute; z-index:-1; background:#010101}
               #flow{background:#020202}
               #hidden{visibility:hidden; background:#040404}
             </style>
             <div id=c><div id=front></div><div id=back></div><div id=flow></div>\
             <div id=hidden></div></div>",
        );
        // Les petits carrés, dans l'ordre de peinture : dessous, flux, dessus ;
        // le carré invisible n'est pas peint.
        let squares: Vec<u8> = list
            .items
            .iter()
            .filter_map(|item| match item {
                DisplayItem::Rect { rect, color, .. } if rect.w == 10.0 => Some(color.r),
                _ => None,
            })
            .collect();
        assert_eq!(squares, [1, 2, 3]);
        let layers: Vec<&str> = list
            .items
            .iter()
            .filter_map(|item| match item {
                DisplayItem::PushOpacity { alpha, .. } if (*alpha - 0.5).abs() < 1.0e-6 => {
                    Some("calque")
                }
                DisplayItem::PushClip { rect, .. } if rect.w == 50.0 => Some("découpe"),
                DisplayItem::PopClip => Some("fin"),
                _ => None,
            })
            .collect();
        assert_eq!(layers, ["calque", "découpe", "fin", "fin"]);
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
