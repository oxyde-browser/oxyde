//! Flottants (CSS 2 §9.5).
//!
//! Chaque contexte de formatage de blocs tient la liste de ses flottants, en
//! coordonnées de page. Un flottant se place au plus haut et au plus près du
//! bord demandé ; les lignes de texte se raccourcissent à son niveau, les
//! boîtes qui établissent leur propre contexte se rangent à côté de lui, et
//! `clear` fait passer une boîte dessous.
//!
//! LIMITES CONNUES : un flottant rencontré au milieu d'un texte remonte au
//! début de la suite en ligne en cours ; un flottant imbriqué dans un élément
//! en ligne est traité comme une boîte en ligne-bloc ; `shape-outside` n'est
//! pas pris en charge.

use std::sync::Arc;

use oxyde_css::{Clear, ComputedStyle, Display, Float, Overflow};
use oxyde_dom::NodeId;
use oxyde_text::TextLayout;
use oxyde_text::parley::YieldData;

use crate::{Engine, Fragment, Rect, Space, Width, edges_px};

/// Nombre maximal de reprises pour placer une même ligne à côté de flottants.
const MAX_LINE_RETRIES: usize = 64;

/// Flottant placé dans le contexte de formatage en cours.
pub(crate) struct FloatBox {
    /// Boîte de marge, en coordonnées de page.
    rect: Rect,
    /// Vrai pour `float: left`.
    left: bool,
}

impl Engine<'_> {
    /// Vrai si la boîte établit un contexte de formatage de blocs : ses
    /// flottants ne débordent pas d'elle et ceux de l'extérieur ne
    /// l'atteignent pas.
    pub(crate) fn establishes_bfc(&self, node: NodeId, style: &ComputedStyle, mode: Width) -> bool {
        if mode != Width::Stretch
            || style.float != Float::None
            || style.is_out_of_flow()
            || style.flex_container
            || style.grid_container
            || style.overflow_x != Overflow::Visible
            || style.overflow_y != Overflow::Visible
            || matches!(
                style.display,
                Display::InlineBlock | Display::Table | Display::TableCell | Display::TableCaption
            )
        {
            return true;
        }
        // Élément racine, ou élément d'un conteneur flexible ou en grille.
        self.doc
            .node(node)
            .parent
            .and_then(|parent| self.style(parent))
            .is_none_or(|parent| parent.flex_container || parent.grid_container)
    }

    /// Vrai si un enfant de niveau bloc doit se ranger à côté des flottants
    /// plutôt que passer dessous.
    pub(crate) fn avoids_floats(&self, node: NodeId, style: &ComputedStyle) -> bool {
        style.flex_container
            || style.grid_container
            || style.display == Display::Table
            || style.overflow_x != Overflow::Visible
            || style.overflow_y != Overflow::Visible
            || self.intrinsic_size(node, style).is_some()
    }

    /// Bande libre entre les flottants sur la tranche verticale
    /// [`top`, `bottom`[ d'une boîte de contenu d'abscisse `x` et de largeur
    /// `width` : abscisses de ses bords gauche et droit.
    pub(crate) fn float_band(&self, top: f32, bottom: f32, x: f32, width: f32) -> (f32, f32) {
        let mut left = x;
        let mut right = x + width;
        for float in &self.floats {
            let rect = float.rect;
            if rect.h <= 0.0 || rect.y >= bottom || rect.y + rect.h <= top {
                continue;
            }
            if float.left {
                left = left.max(rect.x + rect.w);
            } else {
                right = right.min(rect.x);
            }
        }
        (left, right)
    }

    /// Ordonnée du bas des flottants que `clear` demande de dépasser.
    pub(crate) fn float_clearance(&self, clear: Clear) -> Option<f32> {
        self.floats
            .iter()
            .filter(|float| match clear {
                Clear::None => false,
                Clear::Left => float.left,
                Clear::Right => !float.left,
                Clear::Both => true,
            })
            .map(|float| float.rect.y + float.rect.h)
            .reduce(f32::max)
    }

    /// Ordonnée du bas du flottant le plus bas, s'il y en a.
    pub(crate) fn float_bottom(&self) -> Option<f32> {
        self.float_clearance(Clear::Both)
    }

    /// Plus petite ordonnée de bas de flottant située sous `y` : là où la
    /// bande libre peut s'élargir.
    fn float_next_edge(&self, y: f32) -> Option<f32> {
        self.floats
            .iter()
            .map(|float| float.rect.y + float.rect.h)
            .filter(|bottom| *bottom > y)
            .reduce(f32::min)
    }

    /// Ordonnée, à partir de `top`, où une boîte de largeur `needed` tient
    /// entre les flottants, et la bande libre à cet endroit.
    pub(crate) fn float_fit(
        &self,
        mut top: f32,
        height: f32,
        needed: f32,
        x: f32,
        width: f32,
    ) -> (f32, f32, f32) {
        loop {
            let (left, right) = self.float_band(top, top + height.max(1.0), x, width);
            let unobstructed = left <= x && right >= x + width;
            if unobstructed || right - left >= needed - 0.01 {
                return (top, left, right);
            }
            match self.float_next_edge(top) {
                Some(next) => top = next,
                None => return (top, left, right),
            }
        }
    }

    /// Met en page le flottant `node` et le place dans la boîte de contenu
    /// d'abscisse `x` et de largeur `width`, au plus haut à partir de `top`.
    #[expect(clippy::too_many_arguments, reason = "boîte de contenu explicite")]
    #[inline(never)]
    pub(crate) fn place_float(
        &mut self,
        node: NodeId,
        style: &Arc<ComputedStyle>,
        x: f32,
        width: f32,
        top: f32,
        cb_h: Option<f32>,
        depth: usize,
    ) -> Fragment {
        let margin = edges_px(style.margin, width);
        let waiting = self.out_of_flow.len();
        let space = Space {
            cb_w: width,
            cb_x: 0.0,
            y: 0.0,
            mode: Width::ShrinkToFit,
            cb_h,
            fixed_h: None,
        };
        let fragment = self.layout_box(node, style, space, depth);
        let outer_w = fragment.border_box.w + margin.left + margin.right;
        let outer_h = fragment.border_box.h + margin.top + margin.bottom;
        // Un flottant ne monte pas au-dessus de ceux qui le précèdent.
        let mut top = self
            .floats
            .iter()
            .map(|float| float.rect.y)
            .fold(top, f32::max);
        if let Some(clear) = self.float_clearance(style.clear) {
            top = top.max(clear);
        }
        let (top, left, right) = self.float_fit(top, outer_h, outer_w, x, width);
        let is_left = style.float == Float::Left;
        let outer_x = if is_left { left } else { right - outer_w };
        let dx = outer_x + margin.left - fragment.border_box.x;
        let dy = top + margin.top - fragment.border_box.y;
        let mut fragment = Fragment::Box(fragment);
        fragment.translate(dx, dy);
        self.shift_out_of_flow(waiting, dx, dy);
        self.floats.push(FloatBox {
            rect: Rect {
                x: outer_x,
                y: top,
                w: outer_w,
                h: outer_h,
            },
            left: is_left,
        });
        fragment
    }

    /// Coupe les lignes de `layout`, d'origine (`x`, `y`) et de largeur
    /// `width`, en raccourcissant celles qui passent à côté d'un flottant ;
    /// `line_height` est la hauteur de ligne attendue.
    pub(crate) fn break_lines_around_floats(
        &self,
        layout: &mut TextLayout,
        x: f32,
        y: f32,
        width: f32,
        line_height: f32,
    ) {
        let expected = line_height.max(1.0);
        let mut breaker = layout.break_lines();
        breaker.state_mut().set_layout_max_advance(width);
        let mut height = expected;
        let mut retries = 0;
        loop {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "ordonnée de ligne : précision d'un f32 suffisante"
            )]
            let top = y + breaker.state().line_y() as f32;
            let (left, right) = self.float_band(top, top + height, x, width);
            let state = breaker.state_mut();
            state.set_line_x(left - x);
            state.set_line_max_advance((right - left).max(0.0));
            let Some(data) = breaker.break_next() else {
                break;
            };
            let YieldData::LineBreak(line) = data else {
                continue;
            };
            let narrowed = left > x + 0.01 || right < x + width - 0.01;
            if narrowed && retries < MAX_LINE_RETRIES {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "hauteur de ligne : précision d'un f32 suffisante"
                )]
                let real = (line.line_y_end - line.line_y_start) as f32;
                let content = line.metrics.advance - line.metrics.hanging_advance;
                if content > right - left + 0.5
                    && let Some(next) = self.float_next_edge(top)
                {
                    // Le premier mot ne tient pas à côté du flottant : la
                    // ligne descend là où la bande s'élargit.
                    breaker.revert();
                    breaker.state_mut().set_line_y(f64::from(next - y));
                    retries += 1;
                    height = expected;
                    continue;
                }
                if real > height + 0.01 {
                    // Ligne plus haute que prévu : elle peut croiser d'autres
                    // flottants.
                    let (real_left, real_right) = self.float_band(top, top + real, x, width);
                    if real_left > left + 0.01 || real_right < right - 0.01 {
                        breaker.revert();
                        retries += 1;
                        height = real;
                        continue;
                    }
                }
            }
            retries = 0;
            height = expected;
        }
        breaker.finish();
    }
}
