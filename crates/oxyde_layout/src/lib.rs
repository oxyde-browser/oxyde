//! Mise en page d'Oxyde.
//!
//! Même principe que LayoutNG dans Blink : chaque nœud est mis en page à
//! partir d'un espace de contraintes (largeur disponible, position) et produit
//! un [`Fragment`] immuable. Les coordonnées des fragments sont absolues, en
//! pixels CSS, depuis le coin supérieur gauche de la page.
//!
//! LIMITES CONNUES (suivies dans `conformance/`) :
//! - pour `flex` et `grid`, voir les modules [`flex`] et [`grid`] ;
//! - tableaux : cellules de largeur égale, pas d'algorithme de tableau automatique ;
//! - pas de flottants (`float`) ; `position: sticky` vaut `relative` sans décalage ;
//! - une boîte positionnée est peinte après le contenu de son bloc conteneur,
//!   sans tenir compte de `z-index` ;
//! - les marges verticales ne fusionnent qu'entre frères, pas entre parent et enfant ;
//! - un élément en ligne contenant un bloc est traité comme un bloc ;
//! - les boîtes en ligne n'ont ni bordure ni marge interne (seul leur fond est peint) ;
//! - une hauteur en pourcentage n'est résolue que si la hauteur du bloc
//!   conteneur est imposée (sinon elle vaut `auto`).

mod find;
mod flex;
mod floats;
mod grid;
mod table;

pub use find::{MAX_MATCHES, find_text};

use std::sync::Arc;

use oxyde_css::{
    ComputedStyle, Display, Edges, LineHeight, ListStyle, Position, Size, SizeKeyword, StyleMap,
    TextAlign, Viewport, Visibility, WhiteSpace,
};
use oxyde_dom::{Document, NodeData, NodeId};
use oxyde_text::parley::{
    Alignment, AlignmentOptions, FontFamily, FontStyle, FontWeight, InlineBox, InlineBoxKind,
    LineHeight as ParleyLineHeight, PositionedLayoutItem, TextStyle, TextWrapMode, VerticalAlign,
    WhiteSpaceCollapse,
};
use oxyde_text::{TextBrush, TextContext, TextLayout, css_font_family};

/// Profondeur maximale de l'arbre mis en page. Au-delà, les descendants sont
/// ignorés : la récursion (donc la pile) reste bornée même face à un document
/// hostile. Blink applique la même borne (512) dès l'analyse.
const MAX_DEPTH: usize = 512;

/// Profondeur maximale d'imbrication de boîtes « ajustées au contenu » pour
/// lesquelles on refait une passe ; borne le coût à 2^8 passes au pire.
const MAX_SHRINK_DEPTH: usize = 8;

/// Rectangle en pixels CSS.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Rect {
    /// Abscisse du bord gauche.
    pub x: f32,
    /// Ordonnée du bord supérieur.
    pub y: f32,
    /// Largeur.
    pub w: f32,
    /// Hauteur.
    pub h: f32,
}

impl Rect {
    /// Vrai si le point est dans le rectangle.
    #[must_use]
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

/// Fragment de boîte (bloc, cellule, boîte en ligne-bloc…).
#[derive(Clone)]
pub struct BoxFragment {
    /// Nœud d'origine (`None` pour une boîte anonyme).
    pub node: Option<NodeId>,
    /// Style calculé.
    pub style: Arc<ComputedStyle>,
    /// Boîte de bordure.
    pub border_box: Rect,
    /// Fragments enfants, dans l'ordre de peinture.
    pub children: Vec<Fragment>,
}

/// Fragment de texte : un contexte de mise en forme en ligne complet.
#[derive(Clone)]
pub struct TextFragment {
    /// Origine (coin supérieur gauche) de la mise en page.
    pub x: f32,
    /// Origine (coin supérieur gauche) de la mise en page.
    pub y: f32,
    /// Lignes coupées et alignées.
    pub layout: TextLayout,
    /// Texte mis en forme, dans les indices d'octets de `layout` ; vide pour
    /// un texte qui n'appartient pas au document (puce de liste).
    pub text: String,
    /// Boîtes en ligne-bloc placées dans ce texte.
    pub boxes: Vec<Fragment>,
}

/// Résultat de la mise en page d'un nœud.
#[derive(Clone)]
pub enum Fragment {
    /// Boîte.
    Box(BoxFragment),
    /// Texte.
    Text(Box<TextFragment>),
}

impl Fragment {
    fn translate(&mut self, dx: f32, dy: f32) {
        match self {
            Self::Box(b) => {
                b.border_box.x += dx;
                b.border_box.y += dy;
                for c in &mut b.children {
                    c.translate(dx, dy);
                }
            }
            Self::Text(t) => {
                t.x += dx;
                t.y += dy;
                for c in &mut t.boxes {
                    c.translate(dx, dy);
                }
            }
        }
    }
}

impl std::fmt::Debug for Fragment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Box(b) => f
                .debug_struct("Box")
                .field("node", &b.node)
                .field("border_box", &b.border_box)
                .field("children", &b.children)
                .finish(),
            Self::Text(t) => f
                .debug_struct("Text")
                .field("x", &t.x)
                .field("y", &t.y)
                .field("width", &t.layout.width())
                .field("height", &t.layout.height())
                .field("lines", &t.layout.len())
                .finish(),
        }
    }
}

/// Pistes d'un conteneur en grille, une fois dimensionnées.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GridTracks {
    /// Largeur de chaque colonne.
    pub columns: Vec<f32>,
    /// Hauteur de chaque rangée.
    pub rows: Vec<f32>,
    /// Largeur de contenu de la grille : ses colonnes sans contrainte.
    pub max_content: f32,
}

/// Page mise en page.
#[derive(Clone, Debug)]
pub struct LayoutTree {
    /// Pistes de chaque conteneur en grille.
    pub grids: std::collections::HashMap<NodeId, GridTracks>,
    /// Fragment de l'élément racine (`None` si le document est vide ou `display: none`).
    pub root: Option<Fragment>,
    /// Largeur du contenu (au moins celle de la fenêtre).
    pub width: f32,
    /// Hauteur du contenu.
    pub height: f32,
}

/// Taille intrinsèque d'un élément remplacé (image), si elle est connue.
pub type IntrinsicSize<'a> = &'a dyn Fn(NodeId) -> Option<(f32, f32)>;

/// Met en page `doc` pour une fenêtre `viewport`.
pub fn layout_document(
    doc: &Document,
    styles: &StyleMap,
    text: &mut TextContext,
    viewport: Viewport,
    intrinsic: IntrinsicSize<'_>,
) -> LayoutTree {
    let mut engine = Engine {
        doc,
        styles,
        text,
        intrinsic,
        block_child_cache: vec![None; doc.len()],
        shrink_depth: 0,
        out_of_flow: Vec::new(),
        inline_origin: (0.0, 0.0),
        grids: std::collections::HashMap::new(),
        floats: Vec::new(),
        tables: std::collections::HashMap::new(),
        table_sizes: std::collections::HashMap::new(),
        cell_sizes: std::collections::HashMap::new(),
    };
    let root = doc
        .children(doc.root())
        .find(|n| doc.element(*n).is_some())
        .and_then(|n| Some((n, styles.get(n)?)))
        .filter(|(_, style)| style.display != Display::None)
        .map(|(node, style)| {
            let margin_top = px(style.margin.top, viewport.width);
            let space = Space {
                cb_w: viewport.width,
                cb_x: 0.0,
                y: margin_top,
                mode: Width::Stretch,
                cb_h: Some(viewport.height),
                fixed_h: None,
            };
            let mut root = engine.layout_box(node, style, space, 0);
            // Bloc conteneur initial : les boîtes fixes, et les boîtes
            // absolues sans ancêtre positionné, se placent dans la fenêtre.
            let initial = Rect {
                x: 0.0,
                y: 0.0,
                w: viewport.width,
                h: viewport.height,
            };
            let pending = std::mem::take(&mut engine.out_of_flow);
            for item in pending {
                let placed = engine.layout_out_of_flow(item, initial);
                root.children.push(placed);
            }
            root
        });
    let height = root.as_ref().map_or(0.0, |b| {
        b.border_box.y + b.border_box.h + px(b.style.margin.bottom, viewport.width)
    });
    LayoutTree {
        grids: engine.grids,
        root: root.map(Fragment::Box),
        width: viewport.width,
        height,
    }
}

/// Enfants d'une boîte dans l'ordre où ils se peignent (CSS 2, annexe E,
/// en version simplifiée) : les boîtes positionnées d'indice `z-index`
/// négatif, puis le contenu ordinaire, puis les boîtes positionnées d'indice
/// nul ou positif ; à indice égal, l'ordre du document.
///
/// LIMITE : l'ordre est établi entre les enfants directs d'une boîte, pas à
/// l'échelle de tout un contexte d'empilement.
#[must_use]
pub fn paint_order(children: &[Fragment]) -> Vec<&Fragment> {
    // Une boîte transformée s'empile comme une boîte positionnée.
    let layer = |fragment: &Fragment| match fragment {
        Fragment::Box(b) if b.style.is_positioned() || b.style.transform.is_some() => {
            Some(b.style.z_index.unwrap_or(0))
        }
        _ => None,
    };
    let mut below: Vec<(i32, &Fragment)> = Vec::new();
    let mut above: Vec<(i32, &Fragment)> = Vec::new();
    let mut ordered: Vec<&Fragment> = Vec::with_capacity(children.len());
    for child in children {
        match layer(child) {
            Some(z) if z < 0 => below.push((z, child)),
            Some(z) => above.push((z, child)),
            None => ordered.push(child),
        }
    }
    // Tris stables : l'ordre du document départage les indices égaux.
    below.sort_by_key(|(z, _)| *z);
    above.sort_by_key(|(z, _)| *z);
    let mut out: Vec<&Fragment> = below.into_iter().map(|(_, f)| f).collect();
    out.append(&mut ordered);
    out.extend(above.into_iter().map(|(_, f)| f));
    out
}

/// Transformation (`transform`) de la boîte `b`, en coordonnées de page ;
/// `None` si elle n'est pas transformée.
#[must_use]
pub fn box_transform(b: &BoxFragment) -> Option<oxyde_css::Affine> {
    use oxyde_css::Affine;
    let rect = b.border_box;
    let local = b.style.transform_matrix(rect.w, rect.h)?;
    Some(
        Affine::translate(rect.x, rect.y)
            .then_inner(local)
            .then_inner(Affine::translate(-rect.x, -rect.y)),
    )
}

/// Décalage de défilement d'un élément ; `None` désigne la fenêtre.
pub type ScrollOffsets<'a> = &'a dyn Fn(Option<NodeId>) -> (f32, f32);

/// Vrai si la boîte découpe son contenu (`overflow` autre que `visible`).
#[must_use]
pub fn clips_content(style: &ComputedStyle) -> bool {
    style.overflow_x != oxyde_css::Overflow::Visible
        || style.overflow_y != oxyde_css::Overflow::Visible
}

/// Boîte de marge intérieure de `b` : sa boîte de bordure moins les bordures.
#[must_use]
pub fn padding_box(b: &BoxFragment) -> Rect {
    let bw = b.style.border_widths();
    Rect {
        x: b.border_box.x + bw.left,
        y: b.border_box.y + bw.top,
        w: (b.border_box.w - bw.left - bw.right).max(0.0),
        h: (b.border_box.h - bw.top - bw.bottom).max(0.0),
    }
}

/// Taille du contenu défilable de `b`, mesurée depuis le coin de sa boîte de
/// marge intérieure : au moins cette boîte, agrandie jusqu'au bord des
/// descendants qui en débordent vers la droite ou vers le bas.
///
/// LIMITE : les transformations des descendants ne sont pas prises en compte.
#[must_use]
pub fn scrollable_extent(b: &BoxFragment) -> (f32, f32) {
    fn reach(fragment: &Fragment, right: &mut f32, bottom: &mut f32) {
        match fragment {
            Fragment::Box(b) => {
                // Une boîte fixe tient à la fenêtre, pas à ce contenu.
                if b.style.position == Position::Fixed {
                    return;
                }
                *right = right.max(b.border_box.x + b.border_box.w);
                *bottom = bottom.max(b.border_box.y + b.border_box.h);
                if !clips_content(&b.style) {
                    for child in &b.children {
                        reach(child, right, bottom);
                    }
                }
            }
            Fragment::Text(t) => {
                *right = right.max(t.x + t.layout.width());
                *bottom = bottom.max(t.y + t.layout.height());
                for child in &t.boxes {
                    reach(child, right, bottom);
                }
            }
        }
    }
    let port = padding_box(b);
    let (mut right, mut bottom) = (port.x + port.w, port.y + port.h);
    for child in &b.children {
        reach(child, &mut right, &mut bottom);
    }
    (right - port.x, bottom - port.y)
}

/// Nœud du document situé au point (`x`, `y`) de la page, le plus profond.
#[must_use]
pub fn hit_test(doc: &Document, fragment: &Fragment, x: f32, y: f32) -> Option<NodeId> {
    hit_test_scrolled(doc, fragment, x, y, &|_| (0.0, 0.0))
}

/// Comme [`hit_test`], le contenu des boîtes défilantes étant décalé de
/// `scroll` ; (`x`, `y`) est un point de la page, défilement de la fenêtre
/// compris.
#[must_use]
pub fn hit_test_scrolled(
    doc: &Document,
    fragment: &Fragment,
    x: f32,
    y: f32,
    scroll: ScrollOffsets<'_>,
) -> Option<NodeId> {
    match fragment {
        Fragment::Box(b) => {
            // Une boîte fixe ne suit pas le défilement de la fenêtre.
            let (x, y) = if b.style.position == Position::Fixed {
                let (sx, sy) = scroll(None);
                (x - sx, y - sy)
            } else {
                (x, y)
            };
            // Boîte transformée : le point est ramené dans son repère d'origine.
            let (x, y) = match box_transform(b) {
                Some(matrix) => matrix.inverse()?.apply(x, y),
                None => (x, y),
            };
            let own = || b.border_box.contains(x, y).then_some(b.node).flatten();
            // Boîte qui découpe : son contenu n'est atteint qu'à travers elle,
            // décalé de son défilement.
            let (cx, cy) = if clips_content(&b.style) {
                if !padding_box(b).contains(x, y) {
                    return own();
                }
                let (sx, sy) = b.node.map_or((0.0, 0.0), |node| scroll(Some(node)));
                (x + sx, y + sy)
            } else {
                // Les enfants peuvent déborder de leur parent : on les teste toujours.
                (x, y)
            };
            paint_order(&b.children)
                .into_iter()
                .rev()
                .find_map(|c| hit_test_scrolled(doc, c, cx, cy, scroll))
                .or_else(own)
        }
        Fragment::Text(t) => {
            if let Some(hit) = t
                .boxes
                .iter()
                .rev()
                .find_map(|c| hit_test_scrolled(doc, c, x, y, scroll))
            {
                return Some(hit);
            }
            let (lx, ly) = (x - t.x, y - t.y);
            for line in t.layout.lines() {
                let m = line.metrics();
                if ly < m.block_min_coord || ly >= m.block_max_coord {
                    continue;
                }
                for item in line.items() {
                    if let PositionedLayoutItem::GlyphRun(run) = item
                        && lx >= run.offset()
                        && lx < run.offset() + run.advance()
                    {
                        return doc.id_from_index(run.style().brush.node as usize);
                    }
                }
            }
            None
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Width {
    /// Occupe la largeur disponible (bloc dans le flux).
    Stretch,
    /// S'ajuste au contenu (ligne-bloc).
    ShrinkToFit,
    /// Largeur de contenu imposée par le parent (cellule de tableau).
    Fixed(f32),
}

/// Espace de contraintes d'une boîte : son bloc conteneur et ce que le
/// parent lui impose.
#[derive(Clone, Copy)]
struct Space {
    /// Largeur de la boîte de contenu du bloc conteneur.
    cb_w: f32,
    /// Abscisse de la boîte de contenu du bloc conteneur.
    cb_x: f32,
    /// Ordonnée du bord supérieur de bordure de la boîte.
    y: f32,
    /// Règle de calcul de la largeur.
    mode: Width,
    /// Hauteur du bloc conteneur, si elle est imposée (pour les pourcentages).
    cb_h: Option<f32>,
    /// Hauteur de contenu imposée par le parent (boîte positionnée étirée).
    fixed_h: Option<f32>,
}

/// Boîte hors flux (`position: absolute` ou `fixed`) en attente de son bloc
/// conteneur.
struct OutOfFlow {
    node: NodeId,
    style: Arc<ComputedStyle>,
    /// Position statique : celle qu'aurait la boîte dans le flux.
    static_x: f32,
    static_y: f32,
    depth: usize,
}

/// Résout une hauteur : un pourcentage n'a de sens que si la hauteur du bloc
/// conteneur est imposée.
fn resolve_height(size: Size, cb_h: Option<f32>) -> Option<f32> {
    match size {
        Size::Auto => None,
        Size::Px(v) => Some(v),
        relative => cb_h.and_then(|h| relative.resolve(h)),
    }
}

/// Élément d'un flux en ligne, dans l'ordre du document.
enum InlineItem {
    Start {
        style: Arc<ComputedStyle>,
        node: NodeId,
    },
    End,
    Text(String),
    Box(usize),
    /// Emplacement, dans la ligne, d'une boîte hors flux (rang dans la file
    /// d'attente) : il donne sa position statique.
    Anchor(usize),
}

/// Identifiant Parley du premier emplacement de boîte hors flux ; en dessous,
/// ce sont les boîtes en ligne-bloc.
const ANCHOR_BASE: u64 = 1 << 40;

struct Engine<'a> {
    doc: &'a Document,
    styles: &'a StyleMap,
    text: &'a mut TextContext,
    intrinsic: IntrinsicSize<'a>,
    /// Cache : l'élément en ligne contient-il un enfant de niveau bloc ?
    block_child_cache: Vec<Option<bool>>,
    shrink_depth: usize,
    /// Boîtes hors flux rencontrées, en attente de leur bloc conteneur.
    out_of_flow: Vec<OutOfFlow>,
    /// Origine du contexte en ligne en cours : position statique des boîtes
    /// hors flux qu'il contient.
    inline_origin: (f32, f32),
    /// Pistes des conteneurs en grille déjà mis en page.
    grids: std::collections::HashMap<NodeId, GridTracks>,
    /// Flottants du contexte de formatage de blocs en cours.
    floats: Vec<floats::FloatBox>,
    /// Tableaux planifiés, en attente de leur mise en page.
    tables: std::collections::HashMap<NodeId, table::TablePlan>,
    /// Largeurs de contenu minimale et maximale des tableaux planifiés.
    table_sizes: std::collections::HashMap<NodeId, (f32, f32)>,
    /// Largeurs de contenu minimale et maximale des cellules déjà mesurées.
    cell_sizes: std::collections::HashMap<NodeId, (f32, f32)>,
}

fn px(size: Size, base: f32) -> f32 {
    size.resolve(base).unwrap_or(0.0)
}

fn edges_px(edges: Edges<Size>, base: f32) -> Edges<f32> {
    Edges {
        top: px(edges.top, base),
        right: px(edges.right, base),
        bottom: px(edges.bottom, base),
        left: px(edges.left, base),
    }
}

fn is_block_display(display: Display) -> bool {
    !matches!(
        display,
        Display::Inline | Display::InlineBlock | Display::None | Display::Contents
    )
}

/// Ordonnée de la ligne de base de la dernière ligne de texte de `fragments`.
fn last_baseline(fragments: &[Fragment]) -> Option<f32> {
    fragments.iter().rev().find_map(|f| match f {
        Fragment::Text(t) => t.layout.last_baseline().map(|b| t.y + b),
        Fragment::Box(b) => last_baseline(&b.children),
    })
}

/// Largeur de boîte de bordure d'un contenu de largeur `inner`, une fois
/// appliqués les `min-width` et `max-width` en pixels de `style`.
fn bounded_width(style: &ComputedStyle, inner: f32, extras: f32) -> f32 {
    let outer = |size: f32| {
        if style.border_box {
            size.max(extras)
        } else {
            size + extras
        }
    };
    let mut width = inner + extras;
    if let Size::Px(max) = style.max_width {
        width = width.min(outer(max));
    }
    if let Size::Px(min) = style.min_width {
        width = width.max(outer(min));
    }
    width
}

/// Marge résultant de la fusion de deux marges adjacentes (CSS 2 §8.3.1) :
/// la plus grande de deux marges positives, la plus négative de deux marges
/// négatives, leur somme si elles sont de signes contraires.
fn collapse(a: f32, b: f32) -> f32 {
    if a >= 0.0 && b >= 0.0 {
        a.max(b)
    } else if a < 0.0 && b < 0.0 {
        a.min(b)
    } else {
        a + b
    }
}

fn collapsible_ws(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0c')
}

fn brush(style: &ComputedStyle, node: NodeId, with_background: bool) -> TextBrush {
    let c = style.color;
    let b = style.background_color;
    // `visibility: hidden` : le texte garde sa place mais ne se peint pas.
    let visible = style.visibility == Visibility::Visible;
    TextBrush {
        color: [c.r, c.g, c.b, if visible { c.a } else { 0 }],
        background: if with_background && visible {
            [b.r, b.g, b.b, b.a]
        } else {
            [0; 4]
        },
        node: u32::try_from(node.index()).unwrap_or(0),
    }
}

/// Style Parley correspondant à un style calculé. `family` doit provenir de
/// [`css_font_family`] et vivre aussi longtemps que le résultat.
fn text_style<'f>(
    style: &ComputedStyle,
    family: &'f str,
    brush: TextBrush,
) -> TextStyle<'f, 'static, TextBrush> {
    TextStyle {
        font_family: FontFamily::from(family),
        font_size: style.font_size,
        font_style: if style.italic {
            FontStyle::Italic
        } else {
            FontStyle::Normal
        },
        font_weight: FontWeight::new(style.font_weight),
        brush,
        has_underline: style.underline,
        has_strikethrough: style.line_through,
        line_height: match style.line_height {
            LineHeight::Normal => ParleyLineHeight::NORMAL,
            LineHeight::Number(n) => ParleyLineHeight::FontSizeRelative(n),
            LineHeight::Px(v) => ParleyLineHeight::Absolute(v),
        },
        text_wrap_mode: if style.white_space.wraps() {
            TextWrapMode::Wrap
        } else {
            TextWrapMode::NoWrap
        },
        white_space_collapse: match style.white_space {
            WhiteSpace::Normal | WhiteSpace::Nowrap => WhiteSpaceCollapse::Collapse,
            WhiteSpace::Pre | WhiteSpace::PreWrap => WhiteSpaceCollapse::Preserve,
            WhiteSpace::PreLine => WhiteSpaceCollapse::PreserveBreaks,
        },
        ..TextStyle::default()
    }
}

impl Engine<'_> {
    fn style(&self, node: NodeId) -> Option<&Arc<ComputedStyle>> {
        self.styles.get(node)
    }

    /// Enfants à mettre en page : `display: none` et commentaires écartés,
    /// `display: contents` aplati.
    fn flow_children(&self, node: NodeId, out: &mut Vec<NodeId>, depth: usize) {
        if depth > MAX_DEPTH {
            return;
        }
        // Une liste déroulante est dessinée d'un bloc : ses options ne sont
        // pas mises en page.
        if self.doc.element(node).is_some_and(|e| e.is_html("select")) {
            return;
        }
        for child in self.doc.children(node) {
            match &self.doc.node(child).data {
                NodeData::Text(_) => out.push(child),
                NodeData::Element(_) => match self.style(child).map(|s| s.display) {
                    None | Some(Display::None) => {}
                    Some(Display::Contents) => self.flow_children(child, out, depth + 1),
                    Some(_) => out.push(child),
                },
                _ => {}
            }
        }
    }

    /// Vrai si `node` participe au flux de blocs de son parent.
    fn is_block_level(&mut self, node: NodeId, depth: usize) -> bool {
        let Some(style) = self.style(node) else {
            return false;
        };
        match style.display {
            Display::Inline => self.has_block_child(node, depth),
            d => is_block_display(d),
        }
    }

    /// Un élément en ligne contenant un bloc est promu en bloc (voir limites).
    fn has_block_child(&mut self, node: NodeId, depth: usize) -> bool {
        if depth > MAX_DEPTH {
            return false;
        }
        if let Some(Some(cached)) = self.block_child_cache.get(node.index()) {
            return *cached;
        }
        let mut children = Vec::new();
        self.flow_children(node, &mut children, depth);
        let result = children
            .into_iter()
            .any(|c| self.is_block_level(c, depth + 1));
        if let Some(slot) = self.block_child_cache.get_mut(node.index()) {
            *slot = Some(result);
        }
        result
    }

    /// Met en page une boîte dans l'espace de contraintes `space`.
    fn layout_box(
        &mut self,
        node: NodeId,
        style: &Arc<ComputedStyle>,
        space: Space,
        depth: usize,
    ) -> BoxFragment {
        let Space {
            cb_w,
            cb_x,
            y,
            mode,
            cb_h,
            fixed_h,
        } = space;
        let pad = edges_px(style.padding, cb_w);
        let bw = style.border_widths();
        let extras_w = pad.left + pad.right + bw.left + bw.right;
        let extras_h = pad.top + pad.bottom + bw.top + bw.bottom;
        let margin_left = style.margin.left.resolve(cb_w);
        let margin_right = style.margin.right.resolve(cb_w);
        let (ml, mr) = (margin_left.unwrap_or(0.0), margin_right.unwrap_or(0.0));

        let to_content_w = |outer: f32| {
            if style.border_box {
                (outer - extras_w).max(0.0)
            } else {
                outer
            }
        };
        let dimensions = self.intrinsic_dimensions(node, style);
        let intrinsic = dimensions.map(|(w, h, _)| (w, h));
        // Une image garde ses proportions ; un champ de formulaire, non.
        let keeps_ratio = dimensions.is_some_and(|(_, _, ratio)| ratio);
        // `aspect-ratio` ne vaut pas pour un élément remplacé, qui a le sien.
        let ratio = style.aspect_ratio.filter(|_| intrinsic.is_none());
        let specified = style
            .width
            .resolve(cb_w)
            .map(to_content_w)
            .or_else(|| {
                // Élément remplacé : largeur intrinsèque, ou déduite de la hauteur imposée.
                let (iw, ih) = intrinsic?;
                Some(match style.height {
                    Size::Px(h) if ih > 0.0 && keeps_ratio => iw * h / ih,
                    _ => iw,
                })
            })
            .or_else(|| {
                // `aspect-ratio` : une boîte ajustée à son contenu tire sa
                // largeur de sa hauteur imposée.
                let ratio = ratio.filter(|_| mode == Width::ShrinkToFit)?;
                let own_h = fixed_h.or_else(|| {
                    resolve_height(style.height, cb_h).map(|h| {
                        if style.border_box {
                            (h - extras_h).max(0.0)
                        } else {
                            h
                        }
                    })
                })?;
                Some(if style.border_box {
                    ((own_h + extras_h) * ratio - extras_w).max(0.0)
                } else {
                    own_h * ratio
                })
            });
        // Hauteur de contenu qu'`aspect-ratio` donne à une largeur de contenu.
        let ratio_h = |content_w: f32| {
            ratio.map(|ratio| {
                if style.border_box {
                    ((content_w + extras_w) / ratio - extras_h).max(0.0)
                } else {
                    content_w / ratio
                }
            })
        };
        let clamp_w = |w: f32| {
            let max = style
                .max_width
                .resolve(cb_w)
                .map(to_content_w)
                .unwrap_or(f32::INFINITY);
            let min = style
                .min_width
                .resolve(cb_w)
                .map(to_content_w)
                .unwrap_or(0.0);
            w.min(max).max(min).max(0.0)
        };
        let available = (cb_w - ml - mr - extras_w).max(0.0);
        let keyword = style.width_keyword;
        let mut shrink = false;
        // Un tableau fixe lui-même sa largeur : celle de ses colonnes.
        let planned = (style.display == Display::Table).then(|| {
            let wanted = match mode {
                Width::Fixed(w) => Some(w.max(0.0)),
                _ => specified.map(clamp_w),
            };
            let (limit, floor) = (clamp_w(available), clamp_w(0.0));
            self.plan_table_box(node, style, wanted, limit, floor, depth + 1)
        });
        let content_w = match (planned, mode, specified) {
            (Some(w), _, _) => w,
            (None, Width::Fixed(w), _) => w.max(0.0),
            (None, _, Some(w)) => clamp_w(w),
            (None, Width::Stretch, None) if keyword == SizeKeyword::None => clamp_w(available),
            // Largeur définie par le contenu : `fit-content` part de la place
            // disponible, `min-content` et `max-content` d'une largeur sans
            // contrainte ; la seconde passe ramène la boîte à son contenu.
            (None, _, None) => {
                shrink = true;
                let unbounded =
                    matches!(keyword, SizeKeyword::MinContent | SizeKeyword::MaxContent)
                        && self.shrink_depth < MAX_SHRINK_DEPTH;
                if unbounded { 1.0e6 } else { clamp_w(available) }
            }
        };

        // Marges automatiques : centrage horizontal d'un bloc de largeur imposée.
        let mut border_x = cb_x + ml;
        if mode == Width::Stretch {
            let free = (cb_w - content_w - extras_w - ml - mr).max(0.0);
            border_x += match (margin_left, margin_right) {
                (None, None) => free / 2.0,
                (None, Some(_)) => free,
                _ => 0.0,
            };
        }

        let mut content_x = border_x + bw.left + pad.left;
        let content_y = y + bw.top + pad.top;
        let to_content_h = |outer: f32| {
            if style.border_box {
                (outer - extras_h).max(0.0)
            } else {
                outer
            }
        };
        // Hauteur imposée : elle sert de base aux pourcentages des enfants.
        let own_h = fixed_h.or_else(|| resolve_height(style.height, cb_h).map(to_content_h));
        // `aspect-ratio` : la hauteur découle de la largeur, si elle est déjà connue.
        let definite_h = own_h.or_else(|| if shrink { None } else { ratio_h(content_w) });
        // Les boîtes hors flux rencontrées plus bas attendent ce bloc conteneur.
        let out_of_flow_start = self.out_of_flow.len();
        // Contexte de formatage de blocs : ses flottants lui sont propres.
        let outer_floats = self
            .establishes_bfc(node, style, mode)
            .then(|| std::mem::take(&mut self.floats));
        let floats_start = self.floats.len();
        let (mut children, mut content_h) = self.layout_children(
            node,
            style,
            content_x,
            content_y,
            content_w,
            definite_h,
            depth + 1,
        );

        let mut content_w = content_w;
        if shrink && self.shrink_depth < MAX_SHRINK_DEPTH {
            // Grille : la somme de ses colonnes sans contrainte.
            let wanted = match self.grids.get(&node).filter(|_| style.grid_container) {
                Some(tracks) => tracks.max_content,
                None if keyword == SizeKeyword::MinContent => children
                    .iter()
                    .map(|c| self.min_content(c))
                    .fold(0.0_f32, f32::max),
                None => self.max_content_of(style, &children, content_w),
            };
            let fitted = clamp_w(wanted.ceil().min(content_w));
            if fitted < content_w - 0.5 {
                self.shrink_depth += 1;
                content_w = fitted;
                if mode == Width::Stretch {
                    // Les marges automatiques centrent la boîte une fois sa
                    // largeur connue.
                    let free = (cb_w - content_w - extras_w - ml - mr).max(0.0);
                    border_x = cb_x
                        + ml
                        + match (margin_left, margin_right) {
                            (None, None) => free / 2.0,
                            (None, Some(_)) => free,
                            _ => 0.0,
                        };
                    content_x = border_x + bw.left + pad.left;
                }
                // La seconde passe remet en attente les mêmes boîtes hors flux
                // et replace les mêmes flottants.
                self.out_of_flow.truncate(out_of_flow_start);
                self.floats.truncate(floats_start);
                (children, content_h) = self.layout_children(
                    node,
                    style,
                    content_x,
                    content_y,
                    content_w,
                    definite_h,
                    depth + 1,
                );
                self.shrink_depth -= 1;
            }
        }

        if let Some(outer) = outer_floats {
            // La boîte s'étend jusqu'au bas de ses flottants.
            if let Some(bottom) = self.float_bottom() {
                content_h = content_h.max(bottom - content_y);
            }
            self.floats = outer;
        }
        let specified_h = definite_h.or_else(|| ratio_h(content_w)).or_else(|| {
            intrinsic.map(|(iw, ih)| {
                if iw > 0.0 && keeps_ratio {
                    ih * content_w / iw
                } else {
                    ih
                }
            })
        });
        let mut height = specified_h.unwrap_or(content_h);
        // Une hauteur tirée d'`aspect-ratio` cède devant un contenu plus haut.
        if own_h.is_none() && ratio.is_some() && style.overflow_y == oxyde_css::Overflow::Visible {
            height = height.max(content_h);
        }
        // La hauteur d'une cellule est un minimum ; celle d'un tableau vaut
        // pour sa grille, et sa mise en page l'a déjà prise en compte.
        match style.display {
            Display::Table => height = content_h,
            Display::TableCell => height = height.max(content_h),
            _ => {}
        }
        if fixed_h.is_none() {
            if let Some(max) = resolve_height(style.max_height, cb_h) {
                height = height.min(to_content_h(max));
            }
            if let Some(min) = resolve_height(style.min_height, cb_h) {
                height = height.max(to_content_h(min));
            }
        }
        let border_box = Rect {
            x: border_x,
            y,
            w: content_w + extras_w,
            h: height.max(0.0) + extras_h,
        };

        // Une boîte positionnée est le bloc conteneur de ses descendants
        // absolus : sa boîte de marge intérieure (la bordure exclue). Une
        // boîte transformée l'est aussi, y compris pour les boîtes fixes.
        let transformed = style.transform.is_some();
        if style.is_positioned() || transformed {
            let padding_box = Rect {
                x: border_box.x + bw.left,
                y: border_box.y + bw.top,
                w: border_box.w - bw.left - bw.right,
                h: border_box.h - bw.top - bw.bottom,
            };
            let mut waiting = self.out_of_flow.split_off(out_of_flow_start);
            // Les boîtes fixes remontent jusqu'à la fenêtre.
            let fixed: Vec<OutOfFlow> = {
                let (fixed, absolute) = waiting
                    .drain(..)
                    .partition(|item| item.style.position == Position::Fixed && !transformed);
                waiting = absolute;
                fixed
            };
            for item in waiting {
                let placed = self.layout_out_of_flow(item, padding_box);
                children.push(placed);
            }
            self.out_of_flow.extend(fixed);
        }

        BoxFragment {
            node: Some(node),
            style: Arc::clone(style),
            border_box,
            children,
        }
    }

    /// Décale de (`dx`, `dy`) la position statique des boîtes hors flux mises
    /// en attente depuis le rang `from` : elles suivent la boîte qui les
    /// contient quand celle-ci est déplacée après sa mise en page.
    fn shift_out_of_flow(&mut self, from: usize, dx: f32, dy: f32) {
        for waiting in self.out_of_flow.iter_mut().skip(from) {
            waiting.static_x += dx;
            waiting.static_y += dy;
        }
    }

    /// Met en page une boîte hors flux dans son bloc conteneur `cb` (CSS 2
    /// §10.3.7 et §10.6.4).
    fn layout_out_of_flow(&mut self, item: OutOfFlow, cb: Rect) -> Fragment {
        let waiting = self.out_of_flow.len();
        let style = &item.style;
        let inset = style.inset;
        let (left, right) = (inset.left.resolve(cb.w), inset.right.resolve(cb.w));
        let (top, bottom) = (inset.top.resolve(cb.h), inset.bottom.resolve(cb.h));
        let margin_left = style.margin.left.resolve(cb.w);
        let margin_right = style.margin.right.resolve(cb.w);
        let margin_top = style.margin.top.resolve(cb.w);
        let margin_bottom = style.margin.bottom.resolve(cb.w);
        let (ml, mr) = (margin_left.unwrap_or(0.0), margin_right.unwrap_or(0.0));
        let (mt, mb) = (margin_top.unwrap_or(0.0), margin_bottom.unwrap_or(0.0));
        let pad = edges_px(style.padding, cb.w);
        let bw = style.border_widths();
        let extras_w = pad.left + pad.right + bw.left + bw.right;
        let extras_h = pad.top + pad.bottom + bw.top + bw.bottom;
        let content = |outer: f32, extras: f32| {
            if style.border_box {
                (outer - extras).max(0.0)
            } else {
                outer
            }
        };
        let clamp_w = |w: f32| {
            let max = style
                .max_width
                .resolve(cb.w)
                .map_or(f32::INFINITY, |v| content(v, extras_w));
            let min = style
                .min_width
                .resolve(cb.w)
                .map_or(0.0, |v| content(v, extras_w));
            w.min(max).max(min).max(0.0)
        };
        let has_intrinsic = (self.intrinsic)(item.node).is_some();
        // Largeur : imposée, étirée entre `left` et `right`, ou ajustée au contenu.
        let (mode, available) = match (style.width.resolve(cb.w), left, right) {
            (Some(w), _, _) => (Width::Fixed(clamp_w(content(w, extras_w))), cb.w),
            (None, Some(l), Some(r)) if !has_intrinsic => (
                Width::Fixed(clamp_w((cb.w - l - r - ml - mr - extras_w).max(0.0))),
                cb.w,
            ),
            (None, l, r) => (
                Width::ShrinkToFit,
                (cb.w - l.unwrap_or(0.0) - r.unwrap_or(0.0)).max(0.0),
            ),
        };
        // Hauteur : imposée, ou étirée entre `top` et `bottom`.
        let clamp_h = |h: f32| {
            let max = resolve_height(style.max_height, Some(cb.h))
                .map_or(f32::INFINITY, |v| content(v, extras_h));
            let min =
                resolve_height(style.min_height, Some(cb.h)).map_or(0.0, |v| content(v, extras_h));
            h.min(max).max(min).max(0.0)
        };
        let fixed_h = match (resolve_height(style.height, Some(cb.h)), top, bottom) {
            (Some(h), _, _) => Some(clamp_h(content(h, extras_h))),
            (None, Some(t), Some(b)) if !has_intrinsic => {
                Some(clamp_h((cb.h - t - b - mt - mb - extras_h).max(0.0)))
            }
            _ => None,
        };
        let space = Space {
            cb_w: available,
            cb_x: 0.0,
            y: 0.0,
            mode,
            cb_h: Some(cb.h),
            fixed_h,
        };
        let fragment = self.layout_box(item.node, style, space, item.depth);
        let (w, h) = (fragment.border_box.w, fragment.border_box.h);
        // Position : les décalages l'emportent ; à défaut, la position statique.
        // Des marges automatiques centrent une boîte dont tout le reste est fixé.
        let x = match (left, right) {
            (Some(l), Some(r)) => {
                let free = cb.w - l - r - w;
                match (margin_left, margin_right) {
                    (None, None) => cb.x + l + (free / 2.0).max(0.0),
                    (None, Some(m)) => cb.x + l + (free - m).max(0.0),
                    _ => cb.x + l + ml,
                }
            }
            (Some(l), None) => cb.x + l + ml,
            (None, Some(r)) => cb.x + cb.w - r - mr - w,
            (None, None) => item.static_x + ml,
        };
        let y = match (top, bottom) {
            (Some(t), Some(b)) => {
                let free = cb.h - t - b - h;
                match (margin_top, margin_bottom) {
                    (None, None) => cb.y + t + (free / 2.0).max(0.0),
                    (None, Some(m)) => cb.y + t + (free - m).max(0.0),
                    _ => cb.y + t + mt,
                }
            }
            (Some(t), None) => cb.y + t + mt,
            (None, Some(b)) => cb.y + cb.h - b - mb - h,
            (None, None) => item.static_y + mt,
        };
        let mut fragment = Fragment::Box(fragment);
        if let Fragment::Box(b) = &fragment {
            let (dx, dy) = (x - b.border_box.x, y - b.border_box.y);
            fragment.translate(dx, dy);
            self.shift_out_of_flow(waiting, dx, dy);
        }
        fragment
    }

    /// Largeur de contenu la plus grande d'une boîte de style `style` dont
    /// les enfants sont `children` : la plus large des boîtes empilées, ou
    /// la somme des éléments d'une rangée flexible.
    fn max_content_of(&self, style: &ComputedStyle, children: &[Fragment], available: f32) -> f32 {
        if style.flex_container && style.flex_direction.is_row() {
            let in_flow = children
                .iter()
                .filter(|c| !matches!(c, Fragment::Box(b) if b.style.is_out_of_flow()))
                .count();
            let gap = style.column_gap.resolve(available).unwrap_or(0.0);
            #[expect(
                clippy::cast_precision_loss,
                reason = "nombre d'éléments petit devant 2^24"
            )]
            let gaps = gap * in_flow.saturating_sub(1) as f32;
            children
                .iter()
                .filter(|c| !matches!(c, Fragment::Box(b) if b.style.is_out_of_flow()))
                .map(|c| self.max_content(c, available))
                .sum::<f32>()
                + gaps
        } else {
            // Les flottants se rangent côte à côte, à côté du reste.
            let is_float = |c: &Fragment| {
                matches!(c, Fragment::Box(b)
                    if b.style.float != oxyde_css::Float::None && !b.style.is_out_of_flow())
            };
            let floats: f32 = children
                .iter()
                .filter(|c| is_float(c))
                .map(|c| self.max_content(c, available))
                .sum();
            let others = children
                .iter()
                .filter(|c| !is_float(c))
                .map(|c| self.max_content(c, available))
                .fold(0.0_f32, f32::max);
            floats + others
        }
    }

    /// Taille intrinsèque de `node` : celle de son image, ou la taille par
    /// défaut d'un champ de formulaire.
    fn intrinsic_size(&self, node: NodeId, style: &ComputedStyle) -> Option<(f32, f32)> {
        self.intrinsic_dimensions(node, style)
            .map(|(w, h, _)| (w, h))
    }

    /// Comme [`Engine::intrinsic_size`], avec un troisième terme : vrai si
    /// l'élément garde ses proportions quand une seule dimension est imposée.
    fn intrinsic_dimensions(
        &self,
        node: NodeId,
        style: &ComputedStyle,
    ) -> Option<(f32, f32, bool)> {
        if let Some((w, h)) = (self.intrinsic)(node) {
            return Some((w, h, true));
        }
        let element = self.doc.element(node)?;
        // `<canvas>` : ses attributs `width` et `height`, 300 × 150 à défaut.
        let attribute = |name: &str, default: f32| {
            element
                .attr(name)
                .and_then(|v| v.trim().parse::<f32>().ok())
                .filter(|v| *v >= 0.0)
                .unwrap_or(default)
        };
        if element.is_html("canvas") {
            return Some((attribute("width", 300.0), attribute("height", 150.0), true));
        }
        if element.is_html("iframe") || element.is_html("video") {
            return Some((attribute("width", 300.0), attribute("height", 150.0), false));
        }
        if !element.is_html("input") && !element.is_html("select") && !element.is_html("textarea") {
            return None;
        }
        self.control_size(element, style)
            .map(|(w, h)| (w, h, false))
    }

    /// Taille par défaut d'un champ de formulaire.
    fn control_size(
        &self,
        element: &oxyde_dom::ElementData,
        style: &ComputedStyle,
    ) -> Option<(f32, f32)> {
        let line = style.line_height_px();
        Some(match element.name.as_str() {
            "input" => match element.attr("type").map(str::to_ascii_lowercase).as_deref() {
                Some("checkbox" | "radio") => (13.0, 13.0),
                Some("hidden") => (0.0, 0.0),
                _ => (150.0, line),
            },
            "select" => (80.0, line),
            _ => (150.0, line * 2.0),
        })
    }

    /// Largeur extérieure qu'occuperait `fragment` sans contrainte de largeur.
    fn max_content(&self, fragment: &Fragment, available: f32) -> f32 {
        match fragment {
            Fragment::Text(t) => t
                .layout
                .calculate_content_widths()
                .max
                .min(available + 1.0e6),
            Fragment::Box(b) => {
                let margins =
                    px(b.style.margin.left, available) + px(b.style.margin.right, available);
                // Une largeur en pourcentage ne dit rien de la place que
                // le contenu demande.
                if matches!(b.style.width, Size::Px(_))
                    || b.node
                        .is_some_and(|n| self.intrinsic_size(n, &b.style).is_some())
                {
                    return b.border_box.w + margins;
                }
                let pad = edges_px(b.style.padding, available);
                let bw = b.style.border_widths();
                let extras = pad.left + pad.right + bw.left + bw.right;
                let grid = b
                    .node
                    .filter(|_| b.style.grid_container)
                    .and_then(|n| self.grids.get(&n));
                let table = b
                    .node
                    .filter(|_| b.style.display == Display::Table)
                    .and_then(|n| self.table_sizes.get(&n));
                let inner = match (grid, table) {
                    (Some(tracks), _) => tracks.max_content,
                    (None, Some((_, max))) => *max,
                    (None, None) => self.max_content_of(&b.style, &b.children, available),
                };
                // Le contenu ne descend pas sous `min-width` ni ne dépasse
                // un `max-width` en pixels.
                let outer = bounded_width(&b.style, inner, extras);
                outer + margins
            }
        }
    }

    /// Met en page les enfants de `node` dans la boîte de contenu donnée ;
    /// renvoie les fragments et la hauteur de contenu. `height` : hauteur de
    /// cette boîte de contenu si elle est imposée.
    #[expect(clippy::too_many_arguments, reason = "boîte de contenu explicite")]
    fn layout_children(
        &mut self,
        node: NodeId,
        style: &Arc<ComputedStyle>,
        x: f32,
        y: f32,
        width: f32,
        height: Option<f32>,
        depth: usize,
    ) -> (Vec<Fragment>, f32) {
        let mut fragments = Vec::new();
        if depth > MAX_DEPTH {
            return (fragments, 0.0);
        }
        let mut children = Vec::new();
        self.flow_children(node, &mut children, depth);

        if style.display == Display::Table {
            return self.layout_table(node, x, y, width, height, depth);
        }
        if style.display == Display::TableRow {
            return self.layout_stray_row(node, style, x, y, width, depth);
        }
        if style.flex_container {
            return self.layout_flex(style, node, &children, x, y, width, height, depth);
        }
        if style.grid_container {
            return self.layout_grid(style, node, &children, x, y, width, height, depth);
        }

        if style.display == Display::ListItem
            && let Some(marker) = self.list_marker(node, style, x, y)
        {
            fragments.push(marker);
        }

        let mut cursor = y;
        let mut pending_margin = 0.0_f32;
        let mut run: Vec<NodeId> = Vec::new();
        for child in children {
            // Une boîte hors flux ne prend pas de place : elle attend son bloc
            // conteneur, avec pour position statique le point où elle serait.
            if let Some(child_style) = self.style(child).filter(|s| s.is_out_of_flow()) {
                // Boîte en ligne à l'origine : sa position statique est dans
                // la ligne en cours, elle suit donc le texte qui l'entoure.
                if matches!(child_style.display, Display::Inline | Display::InlineBlock) {
                    run.push(child);
                    continue;
                }
                let item = OutOfFlow {
                    node: child,
                    style: Arc::clone(child_style),
                    static_x: x,
                    static_y: cursor + pending_margin.max(0.0),
                    depth,
                };
                self.out_of_flow.push(item);
                continue;
            }
            if let Some(child_style) = self
                .style(child)
                .filter(|s| s.float != oxyde_css::Float::None)
                .cloned()
            {
                // Flottant : placé au niveau du flux en cours — en haut de la
                // suite en ligne s'il y en a une —, il ne prend pas de place.
                let top = cursor + pending_margin.max(0.0);
                let placed = self.place_float(child, &child_style, x, width, top, height, depth);
                fragments.push(placed);
                continue;
            }
            if !self.is_block_level(child, depth) {
                run.push(child);
                continue;
            }
            self.flush_inline(
                &mut run,
                style,
                node,
                x,
                &mut cursor,
                &mut pending_margin,
                width,
                depth,
                &mut fragments,
            );
            let Some(child_style) = self.style(child).cloned() else {
                continue;
            };
            let margin_top = px(child_style.margin.top, width);
            let margin_bottom = px(child_style.margin.bottom, width);
            // Fusion des marges entre frères adjacents.
            let gap = collapse(pending_margin, margin_top);
            let mut top = cursor + gap;
            let (mut band_x, mut band_w) = (x, width);
            if !self.floats.is_empty() {
                // `clear` : la boîte passe sous les flottants désignés.
                if let Some(clear) = self.float_clearance(child_style.clear) {
                    top = top.max(clear);
                }
                // Une boîte qui établit son propre contexte se range à côté
                // des flottants, ou dessous si elle n'y tient pas.
                if self.avoids_floats(child, &child_style) {
                    let needed = match child_style.width {
                        Size::Px(w) => w,
                        _ => 0.0,
                    };
                    let (fit_top, left, right) = self.float_fit(top, 1.0, needed, x, width);
                    top = fit_top;
                    band_x = left;
                    band_w = (right - left).max(0.0);
                }
            }
            let space = Space {
                cb_w: band_w,
                cb_x: band_x,
                y: top,
                mode: Width::Stretch,
                cb_h: height,
                fixed_h: None,
            };
            let waiting = self.out_of_flow.len();
            let fragment = self.layout_box(child, &child_style, space, depth);
            if fragment.border_box.h == 0.0 && fragment.children.is_empty() {
                // Bloc vide de hauteur nulle : ses marges haute et basse se
                // rejoignent et fusionnent avec celles de ses voisins.
                pending_margin = collapse(gap, margin_bottom);
            } else {
                cursor = fragment.border_box.y + fragment.border_box.h;
                pending_margin = margin_bottom;
            }
            let mut fragment = Fragment::Box(fragment);
            // Positionnement relatif : la boîte est décalée après coup, sans
            // effet sur le flux.
            if child_style.position == Position::Relative {
                let inset = child_style.inset;
                let dx = inset
                    .left
                    .resolve(width)
                    .or_else(|| inset.right.resolve(width).map(|r| -r))
                    .unwrap_or(0.0);
                let dy = resolve_height(inset.top, height)
                    .or_else(|| resolve_height(inset.bottom, height).map(|b| -b))
                    .unwrap_or(0.0);
                fragment.translate(dx, dy);
                self.shift_out_of_flow(waiting, dx, dy);
            }
            fragments.push(fragment);
        }
        self.flush_inline(
            &mut run,
            style,
            node,
            x,
            &mut cursor,
            &mut pending_margin,
            width,
            depth,
            &mut fragments,
        );
        cursor += pending_margin;
        (fragments, (cursor - y).max(0.0))
    }

    /// Met en page la suite en ligne accumulée dans `run`, s'il y en a une.
    #[expect(clippy::too_many_arguments, reason = "état du flux de blocs en cours")]
    fn flush_inline(
        &mut self,
        run: &mut Vec<NodeId>,
        style: &Arc<ComputedStyle>,
        parent: NodeId,
        x: f32,
        cursor: &mut f32,
        pending_margin: &mut f32,
        width: f32,
        depth: usize,
        fragments: &mut Vec<Fragment>,
    ) {
        if run.is_empty() {
            return;
        }
        let nodes = std::mem::take(run);
        // Une suite faite uniquement de blancs fusionnables (et de boîtes
        // hors flux) ne crée aucune ligne.
        let out_of_flow =
            |engine: &Self, n: NodeId| engine.style(n).is_some_and(|s| s.is_out_of_flow());
        let only_ws = style.white_space.collapses()
            && nodes.iter().all(|n| {
                out_of_flow(self, *n)
                    || self
                        .doc
                        .text(*n)
                        .is_some_and(|t| t.chars().all(collapsible_ws))
            });
        if only_ws {
            // Les boîtes hors flux de la suite se placent comme des blocs.
            for n in nodes {
                if let Some(child_style) = self.style(n).filter(|s| s.is_out_of_flow()).cloned() {
                    self.out_of_flow.push(OutOfFlow {
                        node: n,
                        style: child_style,
                        static_x: x,
                        static_y: *cursor + pending_margin.max(0.0),
                        depth,
                    });
                }
            }
            return;
        }
        *cursor += *pending_margin;
        *pending_margin = 0.0;
        let fragment = self.layout_inline(&nodes, style, parent, x, *cursor, width, depth);
        *cursor += fragment.layout.height();
        fragments.push(Fragment::Text(Box::new(fragment)));
    }

    /// Puce ou numéro d'un élément de liste, placé à gauche de son contenu.
    fn list_marker(
        &mut self,
        node: NodeId,
        style: &Arc<ComputedStyle>,
        x: f32,
        y: f32,
    ) -> Option<Fragment> {
        let text = match style.list_style {
            ListStyle::None => return None,
            ListStyle::Disc => "\u{2022}".to_owned(),
            ListStyle::Circle => "\u{25e6}".to_owned(),
            ListStyle::Square => "\u{25aa}".to_owned(),
            ListStyle::Decimal => {
                let mut index = 1usize;
                let mut cur = self.doc.node(node).prev_sibling;
                while let Some(n) = cur {
                    if self.doc.element(n).is_some_and(|e| e.is_html("li")) {
                        index += 1;
                    }
                    cur = self.doc.node(n).prev_sibling;
                }
                format!("{index}.")
            }
        };
        let family = css_font_family(&style.font_family);
        let root = text_style(style, &family, brush(style, node, false));
        let mut builder = self
            .text
            .layouts
            .tree_builder(&mut self.text.fonts, 1.0, true, &root);
        builder.push_text(&text);
        let (mut layout, _) = builder.build();
        layout.break_all_lines(None);
        layout.align(Alignment::Start, AlignmentOptions::default());
        let marker_x = x - layout.width() - style.font_size * 0.4;
        Some(Fragment::Text(Box::new(TextFragment {
            x: marker_x,
            y,
            layout,
            text: String::new(),
            boxes: Vec::new(),
        })))
    }

    /// Aplati le sous-arbre en ligne de `node` en éléments de flux.
    fn collect_inline(
        &mut self,
        node: NodeId,
        available: f32,
        items: &mut Vec<InlineItem>,
        boxes: &mut Vec<Fragment>,
        depth: usize,
    ) {
        if depth > MAX_DEPTH {
            return;
        }
        if let Some(text) = self.doc.text(node) {
            items.push(InlineItem::Text(text.to_owned()));
            return;
        }
        let Some(el) = self.doc.element(node) else {
            return;
        };
        let Some(style) = self.style(node).cloned() else {
            return;
        };
        if style.is_out_of_flow() {
            // Hors flux au milieu d'un texte : sa position statique sera celle
            // de son emplacement dans la ligne ; en attendant, l'origine du
            // contexte en ligne.
            let (static_x, static_y) = self.inline_origin;
            items.push(InlineItem::Anchor(self.out_of_flow.len()));
            self.out_of_flow.push(OutOfFlow {
                node,
                style,
                static_x,
                static_y,
                depth,
            });
            return;
        }
        if el.is_html("br") {
            // U+2028 : saut de ligne obligatoire, non fusionné avec les blancs.
            items.push(InlineItem::Text("\u{2028}".to_owned()));
            return;
        }
        let replaced_without_image = el.is_html("img") && (self.intrinsic)(node).is_none();
        if replaced_without_image && style.width == Size::Auto {
            // Image indisponible : son texte alternatif est affiché en ligne.
            if let Some(alt) = el.attr("alt").filter(|a| !a.is_empty()) {
                items.push(InlineItem::Start { style, node });
                items.push(InlineItem::Text(alt.to_owned()));
                items.push(InlineItem::End);
            }
            return;
        }
        if style.display == Display::InlineBlock {
            let margin = edges_px(style.margin, available);
            let space = Space {
                cb_w: available,
                cb_x: 0.0,
                y: 0.0,
                mode: Width::ShrinkToFit,
                cb_h: None,
                fixed_h: None,
            };
            let fragment = self.layout_box(node, &style, space, depth);
            // La boîte en ligne réservée inclut les marges. `layout_box` a déjà
            // décalé le fragment de la marge gauche ; reste la marge haute.
            let mut fragment = Fragment::Box(fragment);
            fragment.translate(0.0, margin.top);
            items.push(InlineItem::Box(boxes.len()));
            boxes.push(fragment);
            return;
        }
        items.push(InlineItem::Start { style, node });
        let mut children = Vec::new();
        self.flow_children(node, &mut children, depth);
        for child in children {
            self.collect_inline(child, available, items, boxes, depth + 1);
        }
        items.push(InlineItem::End);
    }

    /// Contexte de mise en forme en ligne : construit le texte stylé, coupe
    /// les lignes à `width` et place les boîtes en ligne-bloc.
    #[expect(clippy::too_many_arguments, reason = "espace de contraintes explicite")]
    fn layout_inline(
        &mut self,
        nodes: &[NodeId],
        style: &Arc<ComputedStyle>,
        parent: NodeId,
        x: f32,
        y: f32,
        width: f32,
        depth: usize,
    ) -> TextFragment {
        let mut items = Vec::new();
        let mut boxes = Vec::new();
        self.inline_origin = (x, y);
        for node in nodes {
            self.collect_inline(*node, width, &mut items, &mut boxes, depth);
        }

        // Les chaînes de familles doivent survivre au constructeur Parley.
        let root_family = css_font_family(&style.font_family);
        let families: Vec<String> = items
            .iter()
            .map(|item| match item {
                InlineItem::Start { style, .. } => css_font_family(&style.font_family),
                _ => String::new(),
            })
            .collect();

        let root = text_style(style, &root_family, brush(style, parent, false));
        let mut builder = self
            .text
            .layouts
            .tree_builder(&mut self.text.fonts, 1.0, true, &root);
        let mut source = String::new();
        for (item, family) in items.iter().zip(&families) {
            match item {
                InlineItem::Start { style, node } => {
                    builder.push_style_span(text_style(style, family, brush(style, *node, true)));
                }
                InlineItem::End => builder.pop_style_span(),
                InlineItem::Text(text) => {
                    source.push_str(text);
                    builder.push_text(text);
                }
                InlineItem::Anchor(index) => builder.push_inline_box(InlineBox {
                    id: ANCHOR_BASE + *index as u64,
                    kind: InlineBoxKind::InFlow,
                    index: 0,
                    width: 0.0,
                    height: 0.0,
                    baseline: None,
                    vertical_align: VerticalAlign::BASELINE,
                }),
                InlineItem::Box(index) => {
                    let Some(Fragment::Box(b)) = boxes.get(*index) else {
                        continue;
                    };
                    let margin = edges_px(b.style.margin, width);
                    // Ligne de base d'une boîte en ligne-bloc : celle de sa dernière
                    // ligne de texte (CSS 2 §10.8.1) ; à défaut, son bord inférieur.
                    // À ce stade le fragment est en (marge gauche, marge haute), donc
                    // les ordonnées sont déjà relatives au haut de la boîte de marge.
                    builder.push_inline_box(InlineBox {
                        id: *index as u64,
                        kind: InlineBoxKind::InFlow,
                        index: 0,
                        width: b.border_box.w + margin.left + margin.right,
                        height: b.border_box.h + margin.top + margin.bottom,
                        baseline: last_baseline(&b.children),
                        vertical_align: VerticalAlign::BASELINE,
                    });
                }
            }
        }
        let (mut layout, _) = builder.build();
        if self.floats.is_empty() {
            layout.break_all_lines(Some(width));
        } else {
            self.break_lines_around_floats(&mut layout, x, y, width, style.line_height_px());
        }
        let alignment = match style.text_align {
            TextAlign::Start => Alignment::Start,
            TextAlign::End => Alignment::End,
            TextAlign::Left => Alignment::Left,
            TextAlign::Right => Alignment::Right,
            TextAlign::Center => Alignment::Center,
            TextAlign::Justify => Alignment::Justify,
        };
        layout.align(alignment, AlignmentOptions::default());

        for line in layout.lines() {
            let line_top = line.metrics().block_min_coord;
            for item in line.items() {
                let PositionedLayoutItem::InlineBox(placed) = item else {
                    continue;
                };
                if placed.id >= ANCHOR_BASE {
                    // Position statique : ce point de la ligne, en haut de sa boîte.
                    if let Some(waiting) = usize::try_from(placed.id - ANCHOR_BASE)
                        .ok()
                        .and_then(|i| self.out_of_flow.get_mut(i))
                    {
                        waiting.static_x = x + placed.x;
                        waiting.static_y = y + line_top;
                    }
                } else if let Some(fragment) = usize::try_from(placed.id)
                    .ok()
                    .and_then(|i| boxes.get_mut(i))
                {
                    fragment.translate(x + placed.x, y + placed.y);
                }
            }
        }
        TextFragment {
            x,
            y,
            layout,
            text: source,
            boxes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(html: &str, width: f32) -> (Document, LayoutTree) {
        let doc = oxyde_html::parse_document(html);
        let viewport = Viewport {
            width,
            height: 600.0,
        };
        let sheets = oxyde_css::collect_style_elements(&doc);
        let styles = oxyde_css::compute_styles(&doc, &sheets, viewport);
        let mut text = TextContext::new();
        let tree = layout_document(&doc, &styles, &mut text, viewport, &|_| None);
        (doc, tree)
    }

    fn find<'a>(doc: &Document, fragment: &'a Fragment, tag_id: &str) -> Option<&'a BoxFragment> {
        match fragment {
            Fragment::Box(b) => {
                let is_target = b
                    .node
                    .and_then(|n| doc.element(n))
                    .is_some_and(|e| e.id() == Some(tag_id) || e.name == tag_id);
                if is_target {
                    return Some(b);
                }
                b.children.iter().find_map(|c| find(doc, c, tag_id))
            }
            Fragment::Text(t) => t.boxes.iter().find_map(|c| find(doc, c, tag_id)),
        }
    }

    fn rect(doc: &Document, tree: &LayoutTree, tag_id: &str) -> Rect {
        find(doc, tree.root.as_ref().expect("racine"), tag_id)
            .expect("fragment")
            .border_box
    }

    const RESET: &str = "<style>body{margin:0} div{height:10px}</style>";

    #[test]
    fn block_stacking_and_width() {
        let (doc, tree) = layout(&format!("{RESET}<div id=a></div><div id=b></div>"), 800.0);
        assert_eq!(
            rect(&doc, &tree, "a"),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 800.0,
                h: 10.0
            }
        );
        assert_eq!(
            rect(&doc, &tree, "b"),
            Rect {
                x: 0.0,
                y: 10.0,
                w: 800.0,
                h: 10.0
            }
        );
        assert_eq!(tree.height, 20.0);
    }

    #[test]
    fn box_model() {
        let (doc, tree) = layout(
            &format!(
                "{RESET}<div id=a style='margin:5px 10px; padding:3px; border:2px solid; width:50%'></div>\
                 <div id=b style='width:100px; padding:10px; box-sizing:border-box; margin:0 auto'></div>"
            ),
            400.0,
        );
        assert_eq!(
            rect(&doc, &tree, "a"),
            Rect {
                x: 10.0,
                y: 5.0,
                w: 210.0,
                h: 20.0
            }
        );
        // Largeur de bordure 100, centré : (400 - 100) / 2. Hauteur : border-box 10px, mais
        // la marge interne (20px) l'emporte sur une boîte de contenu négative.
        assert_eq!(
            rect(&doc, &tree, "b"),
            Rect {
                x: 150.0,
                y: 30.0,
                w: 100.0,
                h: 20.0
            }
        );
    }

    #[test]
    fn absolute_relative_and_fixed_positioning() {
        let (doc, tree) = layout(
            "<style>body{margin:0}
               #cb{position:relative; width:200px; height:100px; margin:10px; border:5px solid}
               #abs{position:absolute; left:10px; top:20px; width:30px; height:40px}
               #fill{position:absolute; left:10px; right:20px; top:0; bottom:50%}
               #br{position:absolute; right:0; bottom:0; width:10px; height:10px}
               #rel{position:relative; left:7px; top:3px; height:10px}
               #after{height:10px}
               #fixed{position:fixed; right:0; top:0; width:50px; height:20px}
             </style>
             <div id=cb><div id=abs></div><div id=fill></div><div id=br></div>\
             <div id=rel></div><div id=after></div><div id=fixed></div></div>",
            400.0,
        );
        let at = |id: &str| {
            let r = rect(&doc, &tree, id);
            (r.x, r.y, r.w, r.h)
        };
        // Bloc conteneur : la boîte de marge intérieure de #cb, en (15, 15), 200 × 100.
        assert_eq!(at("cb"), (10.0, 10.0, 210.0, 110.0));
        assert_eq!(at("abs"), (25.0, 35.0, 30.0, 40.0));
        assert_eq!(at("fill"), (25.0, 15.0, 170.0, 50.0));
        assert_eq!(at("br"), (205.0, 105.0, 10.0, 10.0));
        // Décalage relatif : la boîte bouge, pas le flux qui la suit.
        assert_eq!(at("rel"), (22.0, 18.0, 200.0, 10.0));
        assert_eq!(at("after"), (15.0, 25.0, 200.0, 10.0));
        // Fixe : par rapport à la fenêtre (400 × 600).
        assert_eq!(at("fixed"), (350.0, 0.0, 50.0, 20.0));
    }

    #[test]
    fn flex_row_grows_stretches_and_aligns() {
        let (doc, tree) = layout(
            "<style>body{margin:0} #f{display:flex; width:300px; height:100px}
               #a{flex:1; height:20px} #b{flex:2} #c{width:60px; height:40px; align-self:center}
             </style>
             <div id=f><div id=a></div><div id=b></div><div id=c></div></div>",
            800.0,
        );
        let at = |id: &str| {
            let r = rect(&doc, &tree, id);
            (r.x, r.y, r.w, r.h)
        };
        // 240 px libres partagés 1 : 2 ; #b, sans hauteur, s'étire sur la ligne.
        assert_eq!(at("a"), (0.0, 0.0, 80.0, 20.0));
        assert_eq!(at("b"), (80.0, 0.0, 160.0, 100.0));
        assert_eq!(at("c"), (240.0, 30.0, 60.0, 40.0));
    }

    #[test]
    fn flex_column_wrap_and_justify() {
        let (doc, tree) = layout(
            "<style>body{margin:0}
               #col{display:flex; flex-direction:column; width:100px; height:200px}
               #top{height:50px} #rest{flex:1}
               #wrap{display:flex; flex-wrap:wrap; justify-content:space-between; width:100px}
               #wrap div{width:40px; height:10px}
               #shrink{display:flex; width:100px}
               #shrink div{width:80px; height:10px}
             </style>
             <div id=col><div id=top></div><div id=rest></div></div>
             <div id=wrap><div id=w1></div><div id=w2></div><div id=w3></div></div>
             <div id=shrink><div id=s1></div><div id=s2></div></div>",
            800.0,
        );
        let at = |id: &str| {
            let r = rect(&doc, &tree, id);
            (r.x, r.y, r.w, r.h)
        };
        assert_eq!(at("top"), (0.0, 0.0, 100.0, 50.0));
        assert_eq!(at("rest"), (0.0, 50.0, 100.0, 150.0));
        // Deux éléments par ligne, écartés ; le troisième passe à la ligne.
        assert_eq!(at("w1"), (0.0, 200.0, 40.0, 10.0));
        assert_eq!(at("w2"), (60.0, 200.0, 40.0, 10.0));
        assert_eq!(at("w3"), (0.0, 210.0, 40.0, 10.0));
        // Trop larges pour le conteneur : chacun rétrécit de moitié du dépassement.
        assert_eq!(at("s1"), (0.0, 220.0, 50.0, 10.0));
        assert_eq!(at("s2"), (50.0, 220.0, 50.0, 10.0));
    }

    #[test]
    fn grid_tracks_placement_and_spans() {
        let (doc, tree) = layout(
            "<style>body{margin:0}
               #g{display:grid; width:300px; gap:10px;
                  grid-template-columns:100px 1fr 2fr; grid-template-rows:50px auto}
               #a{grid-column:1 / 3} #c{grid-row:2; grid-column:3; height:30px} #d{height:20px}
               #areas{display:grid; width:200px; grid-template-columns:repeat(2, 1fr);
                      grid-template-areas:'tete tete' 'menu corps'; grid-auto-rows:40px}
               #tete{grid-area:tete} #corps{grid-area:corps; justify-self:end; width:30px}
             </style>
             <div id=g><div id=a></div><div id=b></div><div id=c></div><div id=d></div></div>
             <div id=areas><div id=tete></div><div id=corps></div></div>",
            800.0,
        );
        let at = |id: &str| {
            let r = rect(&doc, &tree, id);
            (r.x, r.y, r.w, r.h)
        };
        // Colonnes : 100, puis 180 px libres partagés 1 : 2 ; rangées : 50 et 30.
        assert_eq!(at("g"), (0.0, 0.0, 300.0, 90.0));
        assert_eq!(at("a"), (0.0, 0.0, 170.0, 50.0));
        assert_eq!(at("b"), (180.0, 0.0, 120.0, 50.0));
        assert_eq!(at("c"), (180.0, 60.0, 120.0, 30.0));
        assert_eq!(at("d"), (0.0, 60.0, 100.0, 20.0));
        let tracks = tree
            .grids
            .values()
            .find(|t| t.columns.len() == 3)
            .expect("pistes de la grille");
        assert_eq!(tracks.columns, [100.0, 60.0, 120.0]);
        assert_eq!(tracks.rows, [50.0, 30.0]);
        // Zones nommées : l'en-tête couvre les deux colonnes.
        assert_eq!(at("tete"), (0.0, 90.0, 200.0, 40.0));
        assert_eq!(at("corps"), (170.0, 130.0, 30.0, 40.0));
    }

    #[test]
    fn percentage_height_needs_a_definite_container() {
        let (doc, tree) = layout(
            "<style>body{margin:0}</style>\
             <div style='height:200px'><div id=half style='height:50%'></div></div>\
             <div><div id=auto style='height:50%'></div></div>",
            400.0,
        );
        assert_eq!(rect(&doc, &tree, "half").h, 100.0);
        assert_eq!(rect(&doc, &tree, "auto").h, 0.0);
    }

    #[test]
    fn sibling_margins_collapse() {
        let (doc, tree) = layout(
            &format!(
                "{RESET}<div id=a style='margin-bottom:30px'></div><div id=b style='margin-top:20px'></div>"
            ),
            400.0,
        );
        assert_eq!(rect(&doc, &tree, "b").y, 40.0);
    }

    #[test]
    fn text_wraps_and_grows_the_block() {
        let (doc, tree) = layout(
            "<style>body{margin:0} p{margin:0; font-size:20px; line-height:30px}</style>\
             <p id=one>mot</p><p id=many>mot mot mot mot mot mot mot mot mot mot mot mot</p>",
            120.0,
        );
        assert_eq!(rect(&doc, &tree, "one").h, 30.0);
        let many = rect(&doc, &tree, "many");
        assert!(
            many.h >= 90.0,
            "le texte doit passer sur plusieurs lignes : {many:?}"
        );
        assert_eq!(many.h % 30.0, 0.0);
    }

    #[test]
    fn whitespace_between_blocks_creates_no_line() {
        let (doc, tree) = layout(
            &format!("{RESET}<div id=a></div>\n   \n<div id=b></div>"),
            300.0,
        );
        assert_eq!(rect(&doc, &tree, "b").y, 10.0);
    }

    #[test]
    fn inline_block_shrinks_to_fit() {
        let (doc, tree) = layout(
            "<style>body{margin:0}</style>\
             <p style='margin:0'>a <span id=ib style='display:inline-block; padding:0 5px'>bc</span> d</p>",
            500.0,
        );
        let ib = rect(&doc, &tree, "ib");
        assert!(ib.w > 10.0 && ib.w < 100.0, "ajusté au contenu : {ib:?}");
        assert!(ib.x > 0.0, "placé après le texte qui le précède : {ib:?}");
    }

    #[test]
    fn table_row_cells_side_by_side() {
        let (doc, tree) = layout(
            "<style>body{margin:0} td{padding:0}</style>\
             <table><tr><td id=c1>a</td><td id=c2>b<br>c</td></tr></table>",
            300.0,
        );
        let (c1, c2) = (rect(&doc, &tree, "c1"), rect(&doc, &tree, "c2"));
        assert_eq!(c1.x, 2.0, "espacement de bordure par défaut");
        assert_eq!(c2.x, c1.x + c1.w + 2.0);
        assert!(c1.w < 40.0, "la colonne s'ajuste à son contenu : {}", c1.w);
        assert_eq!(c1.h, c2.h, "les cellules d'une rangée ont la même hauteur");
    }

    const TABLE: &str = "<style>body{margin:0} table{border-spacing:0} td,th{padding:0}</style>";

    #[test]
    fn table_columns_follow_specified_widths() {
        let (doc, tree) = layout(
            &format!(
                "{TABLE}<table id=t style='width:400px'><tr>\
                 <td id=a style='width:100px'></td><td id=b></td><td id=c style='width:25%'></td>\
                 </tr></table>"
            ),
            800.0,
        );
        let (a, b, c) = (
            rect(&doc, &tree, "a"),
            rect(&doc, &tree, "b"),
            rect(&doc, &tree, "c"),
        );
        assert_eq!(rect(&doc, &tree, "t").w, 400.0);
        assert_eq!((a.x, a.w), (0.0, 100.0));
        assert_eq!((b.x, b.w), (100.0, 200.0));
        assert_eq!((c.x, c.w), (300.0, 100.0));
    }

    #[test]
    fn table_spans_and_row_heights() {
        let (doc, tree) = layout(
            &format!(
                "{TABLE}<table style='width:300px'>\
                 <tr><td id=a rowspan=2 style='height:80px'></td>\
                     <td id=b style='height:30px;width:100px'></td>\
                     <td id=c style='width:100px'></td></tr>\
                 <tr><td id=d colspan=2 style='height:20px'></td></tr></table>"
            ),
            800.0,
        );
        let (a, b, d) = (
            rect(&doc, &tree, "a"),
            rect(&doc, &tree, "b"),
            rect(&doc, &tree, "d"),
        );
        assert_eq!((a.y, a.h), (0.0, 80.0), "la cellule couvre deux rangées");
        assert_eq!(b.h + d.h, 80.0, "les rangées se partagent la hauteur");
        assert_eq!(d.y, b.h);
        assert_eq!((d.x, d.w), (b.x, 200.0), "la cellule couvre deux colonnes");
    }

    #[test]
    fn table_shrinks_and_centers() {
        let (doc, tree) = layout(
            &format!(
                "{TABLE}<table id=t style='margin:0 auto'><tr>\
                 <td><div style='width:120px;height:10px'></div></td>\
                 <td><div style='width:80px;height:10px'></div></td></tr></table>"
            ),
            800.0,
        );
        let t = rect(&doc, &tree, "t");
        assert_eq!((t.x, t.w), (300.0, 200.0));
    }

    #[test]
    fn table_fixed_layout_and_header_order() {
        let (doc, tree) = layout(
            &format!(
                "{TABLE}<table style='width:300px;table-layout:fixed'>\
                 <tbody><tr><td id=b style='height:10px;width:60px'>un-mot-vraiment-très-long</td>\
                 <td id=c></td></tr></tbody>\
                 <thead><tr><th id=h style='height:20px'></th><th></th></tr></thead></table>"
            ),
            800.0,
        );
        let (h, b, c) = (
            rect(&doc, &tree, "h"),
            rect(&doc, &tree, "b"),
            rect(&doc, &tree, "c"),
        );
        assert_eq!(h.y, 0.0, "l'en-tête passe avant le corps");
        assert_eq!(b.y, 20.0);
        // La première rangée affichée est l'en-tête, sans largeur : parts égales.
        assert_eq!((b.w, c.w), (150.0, 150.0));
    }

    #[test]
    fn table_vertical_align_and_collapse() {
        let (doc, tree) = layout(
            "<style>body{margin:0} table{border-collapse:collapse} \
             td{padding:0;border:2px solid black}</style>\
             <table><tr><td id=a style='height:100px'></td>\
             <td id=b><div id=i style='width:30px;height:20px'></div></td></tr></table>",
            800.0,
        );
        let (a, b, i) = (
            rect(&doc, &tree, "a"),
            rect(&doc, &tree, "b"),
            rect(&doc, &tree, "i"),
        );
        assert_eq!(b.x, a.x + a.w, "pas d'espacement en mode fusionné");
        assert_eq!(b.w, 32.0, "la bordure commune n'est comptée qu'une fois");
        assert_eq!(i.y - b.y, 2.0 + 40.0, "contenu centré verticalement");
    }

    #[test]
    fn floats_stack_side_by_side_and_wrap() {
        let (doc, tree) = layout(
            "<style>body{margin:0} .f{float:left;width:100px;height:40px}</style>\
             <div id=c style='width:250px'><div class=f id=a></div><div class=f id=b></div>\
             <div class=f id=d></div><div id=r style='float:right;width:30px;height:10px'></div></div>",
            800.0,
        );
        let (a, b, d, r) = (
            rect(&doc, &tree, "a"),
            rect(&doc, &tree, "b"),
            rect(&doc, &tree, "d"),
            rect(&doc, &tree, "r"),
        );
        assert_eq!((a.x, a.y), (0.0, 0.0));
        assert_eq!((b.x, b.y), (100.0, 0.0));
        assert_eq!(
            (d.x, d.y),
            (0.0, 40.0),
            "le troisième flottant passe à la ligne"
        );
        assert_eq!(
            (r.x, r.y),
            (220.0, 40.0),
            "à droite, pas plus haut que le précédent"
        );
        assert_eq!(
            rect(&doc, &tree, "c").h,
            0.0,
            "un bloc ordinaire ne contient pas ses flottants"
        );
        assert_eq!(
            tree.height, 80.0,
            "la racine s'étend jusqu'au bas des flottants"
        );
    }

    #[test]
    fn clear_and_flow_root_contain_floats() {
        let (doc, tree) = layout(
            "<style>body{margin:0}</style>\
             <div id=c style='overflow:hidden'><div style='float:left;width:50px;height:70px'></div></div>\
             <div style='float:right;width:20px;height:30px'></div>\
             <div id=n style='clear:both;height:10px'></div>",
            400.0,
        );
        assert_eq!(
            rect(&doc, &tree, "c").h,
            70.0,
            "overflow: hidden contient ses flottants"
        );
        assert_eq!(
            rect(&doc, &tree, "n").y,
            100.0,
            "clear passe sous le flottant"
        );
    }

    #[test]
    fn text_and_flow_roots_avoid_floats() {
        let (doc, tree) = layout(
            "<style>body{margin:0;font-size:10px;line-height:20px}</style>\
             <div style='float:left;width:100px;height:30px'></div>\
             <p id=p style='margin:0'>un deux trois quatre cinq six sept huit neuf dix onze douze</p>\
             <div id=o style='overflow:hidden;height:10px'></div>",
            200.0,
        );
        let root = tree.root.as_ref().expect("racine");
        // La première ligne commence à droite du flottant.
        let hit = hit_test(&doc, root, 105.0, 10.0).expect("texte à côté du flottant");
        let in_paragraph = doc
            .ancestors(hit)
            .chain([hit])
            .any(|n| doc.element(n).is_some_and(|e| e.id() == Some("p")));
        assert!(in_paragraph, "du texte à droite du flottant");
        let p = rect(&doc, &tree, "p");
        assert_eq!(p.x, 0.0, "le bloc lui-même passe sous le flottant");
        assert!(p.h >= 60.0, "le texte est coupé plus court : {}", p.h);
        let o = rect(&doc, &tree, "o");
        assert_eq!(
            o.x, 0.0,
            "sous le flottant, la boîte retrouve toute la largeur"
        );
        assert_eq!(o.w, 200.0);
    }

    #[test]
    fn shrink_to_fit_sums_floats() {
        let (doc, tree) = layout(
            "<style>body{margin:0}</style>\
             <div id=c style='float:left'><div style='float:left;width:60px;height:10px'></div>\
             <div style='float:left;width:40px;height:10px'></div></div>",
            400.0,
        );
        let c = rect(&doc, &tree, "c");
        assert_eq!((c.w, c.h), (100.0, 10.0));
    }

    #[test]
    fn width_keywords_follow_content() {
        let (doc, tree) = layout(
            "<style>body{margin:0} i{display:inline-block;width:40px;height:10px}</style>\
             <div id=max style='width:max-content'><i></i> <i></i> <i></i></div>\
             <div id=min style='width:min-content'><i></i> <i></i> <i></i></div>\
             <div id=fit style='width:fit-content;margin:0 auto'><i></i></div>\
             <div style='width:60px'><div id=big style='width:max-content'><i></i> <i></i></div></div>",
            400.0,
        );
        let max = rect(&doc, &tree, "max");
        assert!(
            max.w > 120.0 && max.w < 140.0,
            "trois boîtes sur une ligne : {}",
            max.w
        );
        assert_eq!(rect(&doc, &tree, "min").w, 40.0);
        let fit = rect(&doc, &tree, "fit");
        assert_eq!(
            (fit.x, fit.w),
            (180.0, 40.0),
            "ajusté au contenu puis centré"
        );
        let big = rect(&doc, &tree, "big");
        assert!(
            big.w > 80.0,
            "max-content dépasse la place disponible : {}",
            big.w
        );
    }

    #[test]
    fn aspect_ratio_links_width_and_height() {
        let (doc, tree) = layout(
            "<style>body{margin:0}</style>\
             <div id=a style='width:200px;aspect-ratio:2/1'></div>\
             <div id=b style='float:left;height:50px;aspect-ratio:3'></div>\
             <div id=c style='clear:both;width:100px;aspect-ratio:1'>\
             <div style='height:150px'></div></div>\
             <div id=d style='width:100px;aspect-ratio:1;box-sizing:border-box;padding:10px'></div>",
            400.0,
        );
        assert_eq!(rect(&doc, &tree, "a").h, 100.0);
        assert_eq!(rect(&doc, &tree, "b").w, 150.0);
        assert_eq!(
            rect(&doc, &tree, "c").h,
            150.0,
            "le contenu l'emporte s'il est plus haut"
        );
        assert_eq!(
            rect(&doc, &tree, "d").h,
            100.0,
            "le rapport vaut pour la boîte de bordure"
        );
    }

    #[test]
    fn transforms_move_hit_testing_not_layout() {
        let (doc, tree) = layout(
            "<style>body{margin:0}</style>\
             <div id=a style='width:100px;height:50px;transform:translate(200px,100px)'></div>\
             <div id=b style='height:10px'></div>",
            800.0,
        );
        assert_eq!(
            rect(&doc, &tree, "a").x,
            0.0,
            "la mise en page ignore la transformation"
        );
        assert_eq!(rect(&doc, &tree, "b").y, 50.0);
        let root = tree.root.as_ref().expect("racine");
        let id_at = |x: f32, y: f32| {
            hit_test(&doc, root, x, y)
                .and_then(|n| doc.element(n))
                .and_then(|e| e.id().map(str::to_owned))
        };
        assert_eq!(
            id_at(250.0, 120.0).as_deref(),
            Some("a"),
            "la boîte est là où elle se peint"
        );
        assert_ne!(
            id_at(50.0, 20.0).as_deref(),
            Some("a"),
            "et plus à sa place d'origine"
        );
        let a = find(&doc, root, "a").expect("fragment");
        let matrix = box_transform(a).expect("transformation");
        assert_eq!(matrix.apply(0.0, 0.0), (200.0, 100.0));
    }

    #[test]
    fn scroll_containers_measure_and_offset_their_content() {
        let (doc, tree) = layout(
            "<style>body{margin:0}</style>\
             <div id=s style='width:100px;height:50px;overflow:auto;border:5px solid black'>\
             <div id=a style='height:40px'></div><div id=b style='width:300px;height:40px'></div>\
             </div><div id=after style='height:20px'></div>",
            400.0,
        );
        let root = tree.root.as_ref().expect("racine");
        let scroller = find(&doc, root, "s").expect("fragment");
        assert_eq!(scrollable_extent(scroller), (300.0, 80.0));
        assert_eq!(padding_box(scroller).h, 50.0);
        let id_at = |x: f32, y: f32, top: f32| {
            let node = scroller.node;
            hit_test_scrolled(&doc, root, x, y, &|n| {
                if n.is_some() && n == node {
                    (0.0, top)
                } else {
                    (0.0, 0.0)
                }
            })
            .and_then(|n| doc.element(n))
            .and_then(|e| e.id().map(str::to_owned))
        };
        assert_eq!(id_at(20.0, 20.0, 0.0).as_deref(), Some("a"));
        assert_eq!(
            id_at(20.0, 20.0, 30.0).as_deref(),
            Some("b"),
            "le contenu a défilé"
        );
        // Le contenu qui dépasse est découpé : on atteint ce qui est dessous.
        assert_eq!(id_at(20.0, 70.0, 0.0).as_deref(), Some("after"));
    }

    #[test]
    fn hit_testing_finds_deepest_node() {
        let (doc, tree) = layout(
            "<style>body{margin:0} p{margin:0; font-size:20px; line-height:30px}</style>\
             <p><a id=lien href=x>lien cliquable</a></p>",
            500.0,
        );
        let root = tree.root.as_ref().expect("racine");
        let hit = hit_test(&doc, root, 10.0, 15.0).expect("nœud touché");
        let link = doc
            .ancestors(hit)
            .chain([hit])
            .find(|n| doc.element(*n).is_some_and(|e| e.id() == Some("lien")));
        assert!(link.is_some(), "le clic doit tomber dans le lien");
        assert!(hit_test(&doc, root, 10.0, 500.0).is_none());
    }

    #[test]
    fn deep_nesting_is_bounded() {
        let html = format!("{}x", "<div>".repeat(5000));
        let (_, tree) = layout(&html, 300.0);
        assert!(tree.root.is_some());
    }
}
