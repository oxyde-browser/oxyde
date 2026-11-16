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
//! `visible` découpe son contenu ; le décalage d'une zone défilante n'est
//! pas dans la liste, il est appliqué à la composition.

pub mod wire;

pub use kurbo::BezPath;
pub use oxyde_layout::Rect;

use oxyde_css::{Color, Overflow, Overscroll, Position, Side, Size, Viewport, Visibility};
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
    /// Fin du groupe transformé, défilant ou fixe le plus récent.
    PopTransform,
    /// Début du contenu d'une zone défilante : tout ce qui suit, jusqu'au
    /// [`DisplayItem::PopTransform`] correspondant, est décalé du défilement
    /// de la zone `id`, connu à la composition seulement.
    PushScroll {
        /// Identifiant de la zone (voir [`Scroller::id`]).
        id: u32,
    },
    /// Début d'une boîte collante (`position: sticky`) : tout ce qui suit,
    /// jusqu'au [`DisplayItem::PopTransform`] correspondant, est décalé de
    /// [`Sticky::offset`], connu à la composition seulement.
    PushSticky(Sticky),
    /// Début d'un groupe fixe (`position: fixed`) : son contenu ne suit pas
    /// le défilement de la fenêtre.
    PushFixed,
    /// Barre de défilement de la zone `id`, dessinée à la composition.
    Scrollbar {
        /// Identifiant de la zone.
        id: u32,
        /// Glissière.
        track: Rect,
        /// Vrai pour la barre verticale.
        vertical: bool,
        /// Longueur visible du contenu.
        visible: f32,
        /// Longueur totale du contenu.
        total: f32,
    },
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

/// Liste d'affichage du document que contient un cadre (`<iframe>`), dans
/// le repère de ce document ; `None` pour un élément qui n'est pas un
/// cadre chargé.
pub type FrameSource<'a> = &'a dyn Fn(NodeId) -> Option<&'a DisplayList>;

/// Identifiant du défilement de la fenêtre.
pub const ROOT_SCROLLER: u32 = u32::MAX;

/// Boîte collante (`position: sticky`) : de quoi calculer, à la composition,
/// le décalage qui la retient dans la partie visible de sa zone défilante.
/// Les rectangles sont en coordonnées de la page, défilement nul.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sticky {
    /// Zone défilante dont le défilement compte ([`ROOT_SCROLLER`] pour la
    /// fenêtre).
    pub scroller: u32,
    /// Boîte de bordure de l'élément, à sa place dans le flux.
    pub rect: Rect,
    /// Boîte de contenu du bloc conteneur : l'élément n'en sort pas.
    pub limits: Rect,
    /// Partie visible de la zone défilante.
    pub port: Rect,
    /// Distance à tenir du bord haut de la partie visible (`top`) ; `None`
    /// pour `auto`.
    pub top: Option<f32>,
    /// De même pour `right`.
    pub right: Option<f32>,
    /// De même pour `bottom`.
    pub bottom: Option<f32>,
    /// De même pour `left`.
    pub left: Option<f32>,
}

impl Sticky {
    /// Décalage de la boîte quand sa zone défilante a défilé de `scroll`.
    #[must_use]
    pub fn offset(&self, scroll: (f32, f32)) -> (f32, f32) {
        /// Sur un axe : `start` et `end` sont les distances à tenir, `pos` et
        /// `len` la boîte, `limit` le bloc conteneur, `port` la partie
        /// visible une fois défilée. Le bord de départ l'emporte.
        fn axis(
            start: Option<f32>,
            end: Option<f32>,
            (pos, len): (f32, f32),
            limit: (f32, f32),
            port: (f32, f32),
        ) -> f32 {
            let mut offset = 0.0;
            if let Some(end) = end {
                let over = (pos + len) - (port.0 + port.1 - end);
                if over > 0.0 {
                    offset = -over.min((pos - limit.0).max(0.0));
                }
            }
            if let Some(start) = start {
                let under = (port.0 + start) - (pos + offset);
                if under > 0.0 {
                    let room = (limit.0 + limit.1) - (pos + offset + len);
                    offset += under.min(room.max(0.0));
                }
            }
            offset
        }
        (
            axis(
                self.left,
                self.right,
                (self.rect.x, self.rect.w),
                (self.limits.x, self.limits.w),
                (self.port.x + scroll.0, self.port.w),
            ),
            axis(
                self.top,
                self.bottom,
                (self.rect.y, self.rect.h),
                (self.limits.y, self.limits.h),
                (self.port.y + scroll.1, self.port.h),
            ),
        )
    }
}

/// Zone défilante relevée à la peinture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scroller {
    /// Index du nœud qui porte la zone.
    pub id: u32,
    /// Rang, dans [`DisplayList::scrollers`], de la zone défilante qui la
    /// contient ; `None` si c'est la fenêtre.
    pub parent: Option<usize>,
    /// Partie visible : la boîte de marge intérieure, en coordonnées de la
    /// page (sans le défilement des zones qui la contiennent).
    pub port: Rect,
    /// Largeur et hauteur du contenu.
    pub content: (f32, f32),
    /// Axes que l'utilisateur peut faire défiler (`overflow: scroll` ou
    /// `auto`) ; `overflow: hidden` ne défile que par script.
    pub wheel: (bool, bool),
    /// Axes sur lesquels un défilement arrivé en bout de course se
    /// propage à la zone englobante (`overscroll-behavior: auto`).
    pub chain: (bool, bool),
}

/// Liste d'affichage : primitives dans l'ordre de peinture.
#[derive(Clone, Debug, Default)]
pub struct DisplayList {
    /// Primitives.
    pub items: Vec<DisplayItem>,
    /// Zones défilantes, chacune après celle qui la contient.
    pub scrollers: Vec<Scroller>,
    /// Zones défilantes ouvertes pendant la peinture.
    open_scrollers: Vec<usize>,
    /// Boîtes de contenu des boîtes ouvertes pendant la peinture.
    open_blocks: Vec<Rect>,
    /// Taille de la fenêtre pendant la peinture.
    viewport: (f32, f32),
    /// Nombre de boîtes en cours de peinture.
    depth: usize,
    /// Rang des nœuds dans l'ordre du document pendant la peinture (voir
    /// `LayoutTree::order`).
    order: Vec<u32>,
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
    canvas_background(tree).unwrap_or(Color::WHITE)
}

/// Fond que le document donne au canevas, s'il en donne un.
fn canvas_background(tree: &LayoutTree) -> Option<Color> {
    let Some(Fragment::Box(root)) = &tree.root else {
        return None;
    };
    if root.style.background_color.a > 0 {
        return Some(root.style.background_color);
    }
    root.children.iter().find_map(|c| match c {
        Fragment::Box(b) if b.style.background_color.a > 0 => Some(b.style.background_color),
        _ => None,
    })
}

/// Peint la page. Les coordonnées de la liste sont celles de la page (le
/// défilement est appliqué au rendu).
#[must_use]
pub fn paint(
    tree: &LayoutTree,
    viewport: Viewport,
    images: ImageSource<'_>,
    frames: FrameSource<'_>,
) -> DisplayList {
    paint_canvas(tree, viewport, Some(canvas_color(tree)), images, frames)
}

/// Peint le document d'un cadre (`<iframe>`) : comme [`paint`], mais le
/// canevas reste transparent si le document ne lui donne pas de fond, et
/// laisse voir la page qui contient le cadre. Les cadres imbriqués ne sont
/// pas dessinés.
#[must_use]
pub fn paint_frame(tree: &LayoutTree, viewport: Viewport, images: ImageSource<'_>) -> DisplayList {
    paint_canvas(tree, viewport, canvas_background(tree), images, &|_| None)
}

fn paint_canvas(
    tree: &LayoutTree,
    viewport: Viewport,
    background: Option<Color>,
    images: ImageSource<'_>,
    frames: FrameSource<'_>,
) -> DisplayList {
    let mut list = DisplayList {
        viewport: (viewport.width, viewport.height),
        order: tree.order.clone(),
        ..DisplayList::default()
    };
    if let Some(color) = background {
        list.rect(
            Rect {
                x: 0.0,
                y: 0.0,
                w: tree.width.max(viewport.width),
                h: tree.height.max(viewport.height),
            },
            color,
            0.0,
        );
    }
    if let Some(root) = &tree.root {
        paint_fragment(&mut list, root, images, frames);
    }
    list.order = Vec::new();
    list
}

/// Boîte de contenu de `b` : bordures et marges internes en pixels retirées.
fn content_rect(b: &BoxFragment) -> Rect {
    let (r, style) = (b.border_box, &b.style);
    let bw = style.border_widths();
    let pad = |s: Size| match s {
        Size::Px(v) => v,
        _ => 0.0,
    };
    let (left, top) = (
        bw.left + pad(style.padding.left),
        bw.top + pad(style.padding.top),
    );
    Rect {
        x: r.x + left,
        y: r.y + top,
        w: (r.w - left - bw.right - pad(style.padding.right)).max(0.0),
        h: (r.h - top - bw.bottom - pad(style.padding.bottom)).max(0.0),
    }
}

/// Boîtes de contenu des éléments de `tree` que `wanted` retient, dans
/// l'ordre de peinture : la taille à laquelle mettre en page le document
/// d'un cadre avant de peindre la page qui le contient.
#[must_use]
pub fn content_boxes(tree: &LayoutTree, wanted: &dyn Fn(NodeId) -> bool) -> Vec<(NodeId, Rect)> {
    let mut found = Vec::new();
    let mut pending: Vec<&Fragment> = tree.root.iter().collect();
    while let Some(fragment) = pending.pop() {
        match fragment {
            Fragment::Box(b) => {
                if let Some(node) = b.node.filter(|node| wanted(*node)) {
                    found.push((node, content_rect(b)));
                }
                pending.extend(b.children.iter().rev());
            }
            Fragment::Text(t) => pending.extend(t.boxes.iter().rev()),
        }
    }
    found
}

/// Insère dans `list` la liste d'affichage `frame` d'un cadre, décalée et
/// découpée à sa boîte de contenu `rect`.
///
/// LIMITE : le contenu d'un cadre ne défile pas ; ses zones défilantes et
/// ses éléments fixes sont dessinés à leur position de départ.
fn embed_frame(list: &mut DisplayList, frame: &DisplayList, rect: Rect) {
    const IDENTITY: [f32; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    if rect.w <= 0.0 || rect.h <= 0.0 || frame.items.is_empty() {
        return;
    }
    list.items.push(DisplayItem::PushClip { rect, radius: 0.0 });
    list.items.push(DisplayItem::PushTransform {
        matrix: [1.0, 0.0, 0.0, 1.0, rect.x, rect.y],
    });
    for item in &frame.items {
        match item {
            // Leur fin est un `PopTransform`, qui reste en place.
            DisplayItem::PushScroll { .. }
            | DisplayItem::PushFixed
            | DisplayItem::PushSticky(_) => {
                list.items
                    .push(DisplayItem::PushTransform { matrix: IDENTITY });
            }
            DisplayItem::Scrollbar { .. } => {}
            other => list.items.push(other.clone()),
        }
    }
    list.items.push(DisplayItem::PopTransform);
    list.items.push(DisplayItem::PopClip);
}

fn paint_fragment(
    list: &mut DisplayList,
    fragment: &Fragment,
    images: ImageSource<'_>,
    frames: FrameSource<'_>,
) {
    match fragment {
        Fragment::Box(b) => paint_box(list, b, images, frames),
        Fragment::Text(t) => paint_text(list, t, images, frames),
    }
}

/// Vrai si la boîte se peint dans la couche des boîtes positionnées du
/// contexte d'empilement qui la contient, et non à sa place dans le flux :
/// une boîte positionnée, ou transformée.
fn hoistable(fragment: &Fragment) -> bool {
    matches!(fragment, Fragment::Box(b) if b.style.is_positioned() || transformed(b))
}

/// Vrai si la boîte porte une transformation, même sans effet (`transform`,
/// `rotate`, `scale`, `translate`) : elle forme un contexte d'empilement.
fn transformed(b: &BoxFragment) -> bool {
    b.style
        .transform_matrix(b.border_box.w, b.border_box.h)
        .is_some()
}

/// Vrai si une rangée ou un groupe de rangées : le bloc conteneur d'une
/// partie de tableau est le tableau, pas eux.
fn bounds_children(style: &oxyde_css::ComputedStyle) -> bool {
    !matches!(
        style.display,
        oxyde_css::Display::TableRow | oxyde_css::Display::TableRowGroup
    )
}

/// Découpe et défilement que la boîte `b` impose à son contenu : la zone de
/// découpe, et l'identifiant de sa zone défilante.
fn content_wrappers(b: &BoxFragment) -> (Option<DisplayItem>, Option<u32>) {
    let style = &b.style;
    let clips = oxyde_layout::clips_content(style);
    if !clips {
        return (None, None);
    }
    let bw = style.border_widths();
    let clip = DisplayItem::PushClip {
        rect: oxyde_layout::padding_box(b),
        radius: (style.border_radius - bw.top).max(0.0),
    };
    // Zone défilante : `overflow: clip` découpe sans défiler.
    let scrollable = |overflow: Overflow| !matches!(overflow, Overflow::Visible | Overflow::Clip);
    let scroller = b
        .node
        .filter(|_| scrollable(style.overflow_x) || scrollable(style.overflow_y))
        .map(|node| u32::try_from(node.index()).unwrap_or(ROOT_SCROLLER - 1));
    (Some(clip), scroller)
}

/// Boîte positionnée relevée dans un contexte d'empilement, avec ce qu'il
/// faut pour la peindre hors de sa place dans l'arbre.
struct Hoisted<'a> {
    /// `z-index`, zéro pour `auto`.
    z: i32,
    /// La boîte.
    fragment: &'a BoxFragment,
    /// Découpes et défilements des boîtes traversées jusqu'à elle.
    wrappers: Vec<DisplayItem>,
    /// Zones défilantes traversées, de la plus large à la plus proche.
    scrollers: Vec<u32>,
    /// Boîte de contenu de son bloc conteneur dans le flux.
    block: Rect,
}

/// Relève les boîtes positionnées du contexte d'empilement dont `children`
/// est le contenu, dans l'ordre du document. La descente s'arrête à chaque
/// boîte positionnée ou translucide : elle ordonne elle-même son contenu.
fn collect_hoisted<'a>(
    children: &'a [Fragment],
    wrappers: &mut Vec<DisplayItem>,
    scrollers: &mut Vec<u32>,
    block: Rect,
    out: &mut Vec<Hoisted<'a>>,
) {
    for child in children {
        match child {
            Fragment::Box(b) if hoistable(child) => out.push(Hoisted {
                z: b.style.z_index.unwrap_or(0),
                fragment: b,
                wrappers: wrappers.clone(),
                scrollers: scrollers.clone(),
                block,
            }),
            Fragment::Box(b) if b.style.opacity < 1.0 => {}
            Fragment::Box(b) => {
                let (clip, scroller) = content_wrappers(b);
                let (open, scrolled) = (wrappers.len(), scrollers.len());
                wrappers.extend(clip);
                if let Some(id) = scroller {
                    wrappers.push(DisplayItem::PushScroll { id });
                    scrollers.push(id);
                }
                let inner = if bounds_children(&b.style) {
                    content_rect(b)
                } else {
                    block
                };
                collect_hoisted(&b.children, wrappers, scrollers, inner, out);
                wrappers.truncate(open);
                scrollers.truncate(scrolled);
            }
            Fragment::Text(t) => collect_hoisted(&t.boxes, wrappers, scrollers, block, out),
        }
    }
}

/// Peint une boîte positionnée relevée par [`collect_hoisted`], sous les
/// découpes et les défilements des boîtes qui la contiennent.
fn paint_hoisted(
    list: &mut DisplayList,
    hoisted: &Hoisted<'_>,
    images: ImageSource<'_>,
    frames: FrameSource<'_>,
) {
    list.items.extend(hoisted.wrappers.iter().cloned());
    let open = list.open_scrollers.len();
    for id in &hoisted.scrollers {
        if let Some(index) = list.scrollers.iter().position(|s| s.id == *id) {
            list.open_scrollers.push(index);
        }
    }
    list.open_blocks.push(hoisted.block);
    paint_box(list, hoisted.fragment, images, frames);
    list.open_blocks.pop();
    list.open_scrollers.truncate(open);
    for wrapper in hoisted.wrappers.iter().rev() {
        list.items.push(match wrapper {
            DisplayItem::PushClip { .. } => DisplayItem::PopClip,
            _ => DisplayItem::PopTransform,
        });
    }
}

/// Peint une boîte : son fond et ses bordures si elle est visible, puis ses
/// enfants dans l'ordre d'empilement, découpés si elle ne laisse pas
/// déborder son contenu ; le tout dans un calque si elle est translucide,
/// et dans un groupe transformé si elle porte un `transform`.
fn paint_box(
    list: &mut DisplayList,
    b: &BoxFragment,
    images: ImageSource<'_>,
    frames: FrameSource<'_>,
) {
    let style = &b.style;
    if style.opacity <= 0.0 {
        return;
    }
    let fixed = style.position == Position::Fixed;
    if fixed {
        list.items.push(DisplayItem::PushFixed);
    }
    let sticky = (style.position == Position::Sticky)
        .then(|| sticky_of(list, b))
        .flatten();
    if let Some(sticky) = sticky {
        list.items.push(DisplayItem::PushSticky(sticky));
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
        paint_decorations(list, b, images, frames);
    }
    let clips = oxyde_layout::clips_content(style);
    let port = oxyde_layout::padding_box(b);
    // Zone défilante : `overflow: clip` découpe sans défiler.
    let scrollable = |overflow: Overflow| !matches!(overflow, Overflow::Visible | Overflow::Clip);
    let scroller = b
        .node
        .filter(|_| clips && (scrollable(style.overflow_x) || scrollable(style.overflow_y)))
        .map(|node| u32::try_from(node.index()).unwrap_or(ROOT_SCROLLER - 1));
    let content = oxyde_layout::scrollable_extent(b);
    if clips {
        // Découpe à la boîte de marge intérieure.
        let bw = style.border_widths();
        list.items.push(DisplayItem::PushClip {
            rect: port,
            radius: (style.border_radius - bw.top).max(0.0),
        });
    }
    if let Some(id) = scroller {
        let by_user = |overflow: Overflow| matches!(overflow, Overflow::Scroll | Overflow::Auto);
        let index = list.scrollers.len();
        list.scrollers.push(Scroller {
            id,
            parent: list.open_scrollers.last().copied(),
            port,
            content,
            wheel: (by_user(style.overflow_x), by_user(style.overflow_y)),
            chain: (
                style.overscroll_x == Overscroll::Auto,
                style.overscroll_y == Overscroll::Auto,
            ),
        });
        list.open_scrollers.push(index);
        list.items.push(DisplayItem::PushScroll { id });
    }
    let bounds = bounds_children(style);
    if bounds {
        list.open_blocks.push(content_rect(b));
    }
    // Ordre d'empilement (CSS 2, annexe E, simplifiée) : une boîte
    // positionnée, transformée ou translucide — et la racine — ordonne
    // tout son contenu : les boîtes positionnées d'indice négatif, puis le
    // contenu dans le flux, puis les autres boîtes positionnées, où
    // qu'elles soient dans l'arbre ; à indice égal, l'ordre du document.
    let orders = list.depth == 0 || style.is_positioned() || transformed(b) || style.opacity < 1.0;
    list.depth += 1;
    let mut hoisted = Vec::new();
    if orders {
        let block = list.open_blocks.last().copied().unwrap_or(b.border_box);
        collect_hoisted(
            &b.children,
            &mut Vec::new(),
            &mut Vec::new(),
            block,
            &mut hoisted,
        );
        // À indice égal, l'ordre du document : la mise en page range une
        // boîte hors flux sous son bloc conteneur, pas à sa place.
        let rank = |h: &Hoisted<'_>| {
            h.fragment
                .node
                .and_then(|node| list.order.get(node.index()).copied())
                .unwrap_or(u32::MAX)
        };
        hoisted.sort_by_key(|h| (h.z, rank(h)));
    }
    for below in hoisted.iter().filter(|h| h.z < 0) {
        paint_hoisted(list, below, images, frames);
    }
    for child in b.children.iter().filter(|child| !hoistable(child)) {
        paint_fragment(list, child, images, frames);
    }
    for above in hoisted.iter().filter(|h| h.z >= 0) {
        paint_hoisted(list, above, images, frames);
    }
    list.depth -= 1;
    if bounds {
        list.open_blocks.pop();
    }
    if let Some(id) = scroller {
        list.items.push(DisplayItem::PopTransform);
        list.open_scrollers.pop();
        // Barres de défilement, par-dessus le contenu, là où il déborde.
        let thickness = 8.0_f32.min(port.w).min(port.h);
        let by_user = |overflow: Overflow| matches!(overflow, Overflow::Scroll | Overflow::Auto);
        if by_user(style.overflow_y) && content.1 > port.h + 0.5 {
            list.items.push(DisplayItem::Scrollbar {
                id,
                track: Rect {
                    x: port.x + port.w - thickness,
                    y: port.y,
                    w: thickness,
                    h: port.h,
                },
                vertical: true,
                visible: port.h,
                total: content.1,
            });
        }
        if by_user(style.overflow_x) && content.0 > port.w + 0.5 {
            list.items.push(DisplayItem::Scrollbar {
                id,
                track: Rect {
                    x: port.x,
                    y: port.y + port.h - thickness,
                    w: port.w,
                    h: thickness,
                },
                vertical: false,
                visible: port.w,
                total: content.0,
            });
        }
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
    if sticky.is_some() {
        list.items.push(DisplayItem::PopTransform);
    }
    if fixed {
        list.items.push(DisplayItem::PopTransform);
    }
}

/// Ce qui retient la boîte collante `b`, ou `None` si aucun de ses quatre
/// décalages n'est fixé.
///
/// LIMITES : les marges de la boîte ne comptent pas dans la place qui lui
/// reste dans son bloc conteneur ; le clic et les mesures des scripts voient
/// la boîte à sa place dans le flux.
fn sticky_of(list: &DisplayList, b: &BoxFragment) -> Option<Sticky> {
    let (scroller, port) = match list
        .open_scrollers
        .last()
        .and_then(|i| list.scrollers.get(*i))
    {
        Some(scroller) => (scroller.id, scroller.port),
        None => (
            ROOT_SCROLLER,
            Rect {
                x: 0.0,
                y: 0.0,
                w: list.viewport.0,
                h: list.viewport.1,
            },
        ),
    };
    let inset = |size: Size, whole: f32| match size {
        Size::Px(v) => Some(v),
        Size::Percent(p) => Some(whole * p / 100.0),
        Size::Calc { px, percent } => Some(px + whole * percent / 100.0),
        _ => None,
    };
    let edges = &b.style.inset;
    let sticky = Sticky {
        scroller,
        rect: b.border_box,
        limits: list.open_blocks.last().copied().unwrap_or(b.border_box),
        port,
        top: inset(edges.top, port.h),
        right: inset(edges.right, port.w),
        bottom: inset(edges.bottom, port.h),
        left: inset(edges.left, port.w),
    };
    [sticky.top, sticky.right, sticky.bottom, sticky.left]
        .iter()
        .any(Option::is_some)
        .then_some(sticky)
}

/// Fond, bordures et image d'une boîte.
fn paint_decorations(
    list: &mut DisplayList,
    b: &BoxFragment,
    images: ImageSource<'_>,
    frames: FrameSource<'_>,
) {
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
    if let Some(frame) = b.node.and_then(frames) {
        embed_frame(list, frame, content_rect(b));
    }
    if let Some(image) = b.node.and_then(images) {
        let rect = content_rect(b);
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

fn paint_text(
    list: &mut DisplayList,
    t: &TextFragment,
    images: ImageSource<'_>,
    frames: FrameSource<'_>,
) {
    list.text(&t.layout, t.x, t.y);
    // Les boîtes positionnées sont peintes par leur contexte d'empilement.
    for child in t.boxes.iter().filter(|child| !hoistable(child)) {
        paint_fragment(list, child, images, frames);
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
        paint(&tree, viewport, &|_| None, &|_| None)
    }

    #[test]
    fn positioned_boxes_are_ordered_across_the_stacking_context() {
        let list = paint_html(
            "<style>body{margin:0} div{width:10px; height:10px}
               #tot{position:absolute; background:#010101}
               #flux{background:#020202}
               #profond{position:relative; background:#030303}
               #zone{overflow:hidden; width:20px; height:20px}
               #dessous{position:relative; z-index:-1; background:#040404}
               #apres{background:#050505}
             </style>
             <div id=tot></div>
             <div id=zone><div id=flux></div><div><div id=profond></div></div></div>
             <div id=dessous></div>
             <div id=apres></div>",
        );
        let grey = |item: &DisplayItem| match item {
            DisplayItem::Rect { color, .. } if color.r == color.g && color.r < 10 => Some(color.r),
            _ => None,
        };
        // Le premier rectangle est le fond du canevas.
        let order: Vec<u8> = list.items.iter().skip(1).filter_map(grey).collect();
        // Indice négatif, puis le flux, puis les boîtes positionnées dans
        // l'ordre du document, même enfouies dans une boîte du flux.
        assert_eq!(order, [4, 2, 5, 1, 3], "{:?}", list.items);
        // La boîte enfouie reste découpée par la zone qui la contient.
        let deep = list
            .items
            .iter()
            .position(|item| grey(item) == Some(3))
            .expect("boîte enfouie");
        let clips = list.items[..deep]
            .iter()
            .filter(|item| matches!(item, DisplayItem::PushClip { rect, .. } if rect.w == 20.0))
            .count();
        assert_eq!(clips, 2, "{:?}", list.items);
        let pushes = list
            .items
            .iter()
            .filter(|item| {
                matches!(
                    item,
                    DisplayItem::PushClip { .. } | DisplayItem::PushOpacity { .. }
                )
            })
            .count();
        let pops = list
            .items
            .iter()
            .filter(|item| matches!(item, DisplayItem::PopClip))
            .count();
        assert_eq!(pushes, pops);
    }

    #[test]
    fn sticky_boxes_are_held_in_the_visible_part() {
        let list = paint_html(
            "<style>body{margin:0}
               #bloc{height:500px; margin-top:40px}
               #haut{position:sticky; top:10px; height:20px}
               #bas{position:sticky; bottom:0; height:20px; margin-top:800px}
               #libre{position:sticky; height:20px}
               #zone{overflow:auto; height:100px; margin-top:50px}
               #dedans{position:sticky; top:0; height:10px; margin-top:30px}
             </style>
             <div id=bloc><div id=haut></div><div id=libre></div></div>
             <div id=zone><div id=dedans></div><div style='height:400px'></div></div>
             <div id=bas></div>",
        );
        let stickies: Vec<Sticky> = list
            .items
            .iter()
            .filter_map(|item| match item {
                DisplayItem::PushSticky(sticky) => Some(*sticky),
                _ => None,
            })
            .collect();
        // Sans décalage fixé, rien ne retient la boîte.
        assert_eq!(stickies.len(), 3, "{stickies:?}");
        let (top, inner, bottom) = (stickies[0], stickies[1], stickies[2]);

        // `top` : la boîte suit le haut de la fenêtre, sans sortir de son bloc.
        assert_eq!(top.scroller, ROOT_SCROLLER);
        assert_eq!(
            (top.rect.y, top.limits.y, top.limits.h),
            (40.0, 40.0, 500.0)
        );
        assert_eq!(top.offset((0.0, 0.0)), (0.0, 0.0));
        assert_eq!(top.offset((0.0, 100.0)), (0.0, 70.0));
        assert_eq!(top.offset((0.0, 5000.0)), (0.0, 480.0));

        // Dans une zone défilante, c'est son défilement et sa partie visible
        // qui comptent.
        assert_ne!(inner.scroller, ROOT_SCROLLER);
        assert_eq!(
            (inner.port.y, inner.port.h, inner.rect.y),
            (590.0, 100.0, 620.0)
        );
        assert_eq!(inner.offset((0.0, 0.0)), (0.0, 0.0));
        assert_eq!(inner.offset((0.0, 50.0)), (0.0, 20.0));

        // `bottom` : la boîte, plus bas que la fenêtre, remonte à son bord.
        let (_, lifted) = bottom.offset((0.0, 0.0));
        assert_eq!(bottom.rect.y + lifted + bottom.rect.h, 300.0);
        assert_eq!(bottom.offset((0.0, 5000.0)), (0.0, 0.0));
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
