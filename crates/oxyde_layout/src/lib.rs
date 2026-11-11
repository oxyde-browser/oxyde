//! Mise en page d'Oxyde.
//!
//! Même principe que LayoutNG dans Blink : chaque nœud est mis en page à
//! partir d'un espace de contraintes (largeur disponible, position) et produit
//! un [`Fragment`] immuable. Les coordonnées des fragments sont absolues, en
//! pixels CSS, depuis le coin supérieur gauche de la page.
//!
//! LIMITES CONNUES (suivies dans `conformance/`) :
//! - `flex` et `grid` sont mis en page comme des blocs ;
//! - tableaux : cellules de largeur égale, pas d'algorithme de tableau automatique ;
//! - pas de flottants ni de positionnement (`float`, `position`) ;
//! - les marges verticales ne fusionnent qu'entre frères, pas entre parent et enfant ;
//! - un élément en ligne contenant un bloc est traité comme un bloc ;
//! - les boîtes en ligne n'ont ni bordure ni marge interne (seul leur fond est peint) ;
//! - les hauteurs en pourcentage sont traitées comme `auto`.

use std::sync::Arc;

use oxyde_css::{
    ComputedStyle, Display, Edges, LineHeight, ListStyle, Size, StyleMap, TextAlign, Viewport,
    WhiteSpace,
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

/// Page mise en page.
#[derive(Clone, Debug)]
pub struct LayoutTree {
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
    };
    let root = doc
        .children(doc.root())
        .find(|n| doc.element(*n).is_some())
        .and_then(|n| Some((n, styles.get(n)?)))
        .filter(|(_, style)| style.display != Display::None)
        .map(|(node, style)| {
            let margin_top = px(style.margin.top, viewport.width);
            engine.layout_box(
                node,
                style,
                viewport.width,
                0.0,
                margin_top,
                Width::Stretch,
                0,
            )
        });
    let height = root.as_ref().map_or(0.0, |b| {
        b.border_box.y + b.border_box.h + px(b.style.margin.bottom, viewport.width)
    });
    LayoutTree {
        root: root.map(Fragment::Box),
        width: viewport.width,
        height,
    }
}

/// Nœud du document situé au point (`x`, `y`) de la page, le plus profond.
#[must_use]
pub fn hit_test(doc: &Document, fragment: &Fragment, x: f32, y: f32) -> Option<NodeId> {
    match fragment {
        Fragment::Box(b) => {
            // Les enfants peuvent déborder de leur parent : on les teste toujours.
            b.children
                .iter()
                .rev()
                .find_map(|c| hit_test(doc, c, x, y))
                .or_else(|| b.border_box.contains(x, y).then_some(b.node).flatten())
        }
        Fragment::Text(t) => {
            if let Some(hit) = t.boxes.iter().rev().find_map(|c| hit_test(doc, c, x, y)) {
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

/// Élément d'un flux en ligne, dans l'ordre du document.
enum InlineItem {
    Start {
        style: Arc<ComputedStyle>,
        node: NodeId,
    },
    End,
    Text(String),
    Box(usize),
}

struct Engine<'a> {
    doc: &'a Document,
    styles: &'a StyleMap,
    text: &'a mut TextContext,
    intrinsic: IntrinsicSize<'a>,
    /// Cache : l'élément en ligne contient-il un enfant de niveau bloc ?
    block_child_cache: Vec<Option<bool>>,
    shrink_depth: usize,
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

fn collapsible_ws(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0c')
}

fn brush(style: &ComputedStyle, node: NodeId, with_background: bool) -> TextBrush {
    let c = style.color;
    let b = style.background_color;
    TextBrush {
        color: [c.r, c.g, c.b, c.a],
        background: if with_background {
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

    /// Met en page une boîte dont le bord supérieur de bordure est en `y` et
    /// dont le bloc conteneur a pour boîte de contenu (`cb_x`, largeur `cb_w`).
    #[expect(clippy::too_many_arguments, reason = "espace de contraintes explicite")]
    fn layout_box(
        &mut self,
        node: NodeId,
        style: &Arc<ComputedStyle>,
        cb_w: f32,
        cb_x: f32,
        y: f32,
        mode: Width,
        depth: usize,
    ) -> BoxFragment {
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
        let intrinsic = (self.intrinsic)(node);
        let specified = style.width.resolve(cb_w).map(to_content_w).or_else(|| {
            // Élément remplacé : largeur intrinsèque, ou déduite de la hauteur imposée.
            let (iw, ih) = intrinsic?;
            Some(match style.height {
                Size::Px(h) if ih > 0.0 => iw * h / ih,
                _ => iw,
            })
        });
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
        let mut shrink = false;
        let content_w = match (mode, specified) {
            (Width::Fixed(w), _) => w.max(0.0),
            (_, Some(w)) => clamp_w(w),
            (Width::Stretch, None) => clamp_w(available),
            (Width::ShrinkToFit, None) => {
                shrink = true;
                clamp_w(available)
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

        let content_x = border_x + bw.left + pad.left;
        let content_y = y + bw.top + pad.top;
        let (mut children, mut content_h) =
            self.layout_children(node, style, content_x, content_y, content_w, depth + 1);

        let mut content_w = content_w;
        if shrink && self.shrink_depth < MAX_SHRINK_DEPTH {
            let wanted = children
                .iter()
                .map(|c| self.max_content(c, content_w))
                .fold(0.0_f32, f32::max);
            let fitted = clamp_w(wanted.ceil().min(content_w));
            if fitted < content_w - 0.5 {
                self.shrink_depth += 1;
                content_w = fitted;
                (children, content_h) =
                    self.layout_children(node, style, content_x, content_y, content_w, depth + 1);
                self.shrink_depth -= 1;
            }
        }

        let to_content_h = |outer: f32| {
            if style.border_box {
                (outer - extras_h).max(0.0)
            } else {
                outer
            }
        };
        let specified_h = match style.height {
            Size::Px(h) => Some(to_content_h(h)),
            _ => intrinsic.map(|(iw, ih)| if iw > 0.0 { ih * content_w / iw } else { ih }),
        };
        let mut height = specified_h.unwrap_or(content_h);
        if let Size::Px(max) = style.max_height {
            height = height.min(to_content_h(max));
        }
        if let Size::Px(min) = style.min_height {
            height = height.max(to_content_h(min));
        }

        BoxFragment {
            node: Some(node),
            style: Arc::clone(style),
            border_box: Rect {
                x: border_x,
                y,
                w: content_w + extras_w,
                h: height.max(0.0) + extras_h,
            },
            children,
        }
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
                if b.style.width != Size::Auto
                    || b.node.is_some_and(|n| (self.intrinsic)(n).is_some())
                {
                    return b.border_box.w + margins;
                }
                let pad = edges_px(b.style.padding, available);
                let bw = b.style.border_widths();
                let inner = b
                    .children
                    .iter()
                    .map(|c| self.max_content(c, available))
                    .fold(0.0_f32, f32::max);
                inner + pad.left + pad.right + bw.left + bw.right + margins
            }
        }
    }

    /// Met en page les enfants de `node` dans la boîte de contenu donnée ;
    /// renvoie les fragments et la hauteur de contenu.
    fn layout_children(
        &mut self,
        node: NodeId,
        style: &Arc<ComputedStyle>,
        x: f32,
        y: f32,
        width: f32,
        depth: usize,
    ) -> (Vec<Fragment>, f32) {
        let mut fragments = Vec::new();
        if depth > MAX_DEPTH {
            return (fragments, 0.0);
        }
        let mut children = Vec::new();
        self.flow_children(node, &mut children, depth);

        if style.display == Display::TableRow {
            return self.layout_row(&children, x, y, width, depth);
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
            let gap = if pending_margin >= 0.0 && margin_top >= 0.0 {
                pending_margin.max(margin_top)
            } else {
                pending_margin + margin_top
            };
            let fragment = self.layout_box(
                child,
                &child_style,
                width,
                x,
                cursor + gap,
                Width::Stretch,
                depth,
            );
            cursor = fragment.border_box.y + fragment.border_box.h;
            pending_margin = margin_bottom;
            fragments.push(Fragment::Box(fragment));
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
        // Une suite faite uniquement de blancs fusionnables ne crée aucune ligne.
        let only_ws = style.white_space.collapses()
            && nodes.iter().all(|n| {
                self.doc
                    .text(*n)
                    .is_some_and(|t| t.chars().all(collapsible_ws))
            });
        if only_ws {
            return;
        }
        *cursor += *pending_margin;
        *pending_margin = 0.0;
        let fragment = self.layout_inline(&nodes, style, parent, x, *cursor, width, depth);
        *cursor += fragment.layout.height();
        fragments.push(Fragment::Text(Box::new(fragment)));
    }

    /// Rangée de tableau : cellules côte à côte, de largeur égale.
    fn layout_row(
        &mut self,
        children: &[NodeId],
        x: f32,
        y: f32,
        width: f32,
        depth: usize,
    ) -> (Vec<Fragment>, f32) {
        let cells: Vec<NodeId> = children
            .iter()
            .copied()
            .filter(|c| self.doc.element(*c).is_some())
            .collect();
        if cells.is_empty() {
            return (Vec::new(), 0.0);
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "nombre de cellules petit devant 2^24"
        )]
        let cell_w = width / cells.len() as f32;
        let mut fragments = Vec::with_capacity(cells.len());
        let mut row_h = 0.0_f32;
        let mut cell_x = x;
        for cell in cells {
            let Some(style) = self.style(cell).cloned() else {
                continue;
            };
            let pad = edges_px(style.padding, cell_w);
            let bw = style.border_widths();
            let inner = (cell_w - pad.left - pad.right - bw.left - bw.right).max(0.0);
            let mut fragment =
                self.layout_box(cell, &style, cell_w, cell_x, y, Width::Fixed(inner), depth);
            fragment.border_box.x = cell_x;
            row_h = row_h.max(fragment.border_box.h);
            fragments.push(fragment);
            cell_x += cell_w;
        }
        let fragments = fragments
            .into_iter()
            .map(|mut f| {
                f.border_box.h = row_h;
                Fragment::Box(f)
            })
            .collect();
        (fragments, row_h)
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
            let mut fragment =
                self.layout_box(node, &style, available, 0.0, 0.0, Width::ShrinkToFit, depth);
            // Champs de formulaire sans contenu : taille par défaut.
            if fragment.children.is_empty()
                && style.width == Size::Auto
                && matches!(el.name.as_str(), "input" | "select" | "textarea")
            {
                let rows = if el.name == "textarea" { 2.0 } else { 1.0 };
                fragment.border_box.w += if el.name == "select" { 80.0 } else { 150.0 };
                if style.height == Size::Auto {
                    fragment.border_box.h += style.line_height_px() * rows;
                }
            }
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
        for (item, family) in items.iter().zip(&families) {
            match item {
                InlineItem::Start { style, node } => {
                    builder.push_style_span(text_style(style, family, brush(style, *node, true)));
                }
                InlineItem::End => builder.pop_style_span(),
                InlineItem::Text(text) => builder.push_text(text),
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
        layout.break_all_lines(Some(width));
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
            for item in line.items() {
                if let PositionedLayoutItem::InlineBox(placed) = item
                    && let Some(fragment) = usize::try_from(placed.id)
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
        assert_eq!((c1.x, c1.w, c2.x, c2.w), (0.0, 150.0, 150.0, 150.0));
        assert_eq!(c1.h, c2.h, "les cellules d'une rangée ont la même hauteur");
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
