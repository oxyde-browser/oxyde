//! Mise en page flexible (CSS Flexible Box Layout Level 1, §9).
//!
//! L'algorithme suit le standard : tailles de base et tailles hypothétiques
//! des éléments, répartition en lignes, résolution des longueurs flexibles
//! (avec gel des éléments qui butent sur leur minimum ou leur maximum),
//! tailles transversales, étirement, puis alignement sur les deux axes.
//!
//! LIMITES CONNUES : écriture horizontale seulement ; `align-items: baseline`
//! aligne en haut de ligne ; la taille minimale automatique d'un élément
//! (`min-width: auto`) est approchée par son contenu le plus étroit sans
//! tenir compte d'un rapport d'aspect ; les marges d'un élément ne
//! fusionnent pas (c'est conforme) mais son contenu n'est mesuré qu'une fois
//! par passe.

use std::sync::Arc;

use oxyde_css::{Alignment, ComputedStyle, FlexWrap, Overflow, Position, Size};
use oxyde_dom::NodeId;

use crate::{
    BoxFragment, Engine, Fragment, OutOfFlow, Rect, Space, Width, edges_px, resolve_height,
};

/// Élément flexible en cours de mise en page.
struct Item {
    /// Nœud de l'élément ; `None` pour une suite de texte anonyme.
    node: Option<NodeId>,
    /// Nœuds de texte d'un élément anonyme.
    text: Vec<NodeId>,
    style: Arc<ComputedStyle>,
    /// Marges résolues (les marges automatiques valent 0 ici).
    margin: [f32; 4],
    /// Marges automatiques : haut, droite, bas, gauche.
    auto_margin: [bool; 4],
    /// Marge intérieure et bordure sur l'axe principal et sur l'axe transversal.
    main_extras: f32,
    cross_extras: f32,
    /// Taille de base, taille hypothétique et bornes, en boîte de bordure,
    /// sur l'axe principal.
    base: f32,
    hypothetical: f32,
    min: f32,
    max: f32,
    /// Taille principale retenue (boîte de bordure).
    main: f32,
    /// Taille transversale retenue (boîte de bordure).
    cross: f32,
    frozen: bool,
    fragment: Option<BoxFragment>,
    /// Conteneur flexible : le nœud auquel le texte anonyme est attribué.
    parent: NodeId,
    /// Boîtes hors flux rencontrées dans l'élément, en attente de leur bloc
    /// conteneur ; leur position statique suit l'élément quand il est placé.
    out_of_flow: Vec<OutOfFlow>,
}

impl Item {
    /// Somme des marges sur l'axe principal (`row`) ou vertical.
    fn main_margins(&self, row: bool) -> f32 {
        if row {
            self.margin[1] + self.margin[3]
        } else {
            self.margin[0] + self.margin[2]
        }
    }

    fn cross_margins(&self, row: bool) -> f32 {
        self.main_margins(!row)
    }
}

/// Répartition de l'espace libre selon `justify-content` ou `align-content` :
/// décalage initial et espace entre deux éléments.
fn distribute(alignment: Alignment, free: f32, count: usize) -> (f32, f32) {
    #[expect(
        clippy::cast_precision_loss,
        reason = "nombre d'éléments petit devant 2^24"
    )]
    let n = count as f32;
    let positive = free.max(0.0);
    match alignment {
        Alignment::End => (free, 0.0),
        Alignment::Center => (free / 2.0, 0.0),
        Alignment::SpaceBetween if count > 1 => (0.0, positive / (n - 1.0)),
        Alignment::SpaceAround if count > 0 && free > 0.0 => (positive / n / 2.0, positive / n),
        Alignment::SpaceEvenly if count > 0 && free > 0.0 => {
            (positive / (n + 1.0), positive / (n + 1.0))
        }
        // Les répartitions sans espace à distribuer se replient sur le début.
        Alignment::SpaceAround | Alignment::SpaceEvenly => (free / 2.0, 0.0),
        _ => (0.0, 0.0),
    }
}

impl Engine<'_> {
    /// Met en page les enfants d'un conteneur flexible dont la boîte de
    /// contenu est en (`x`, `y`), de largeur `width` et de hauteur `height`
    /// si elle est imposée. Renvoie les fragments et la hauteur de contenu.
    #[expect(clippy::too_many_arguments, reason = "boîte de contenu explicite")]
    pub(crate) fn layout_flex(
        &mut self,
        style: &Arc<ComputedStyle>,
        parent: NodeId,
        children: &[NodeId],
        x: f32,
        y: f32,
        width: f32,
        height: Option<f32>,
        depth: usize,
    ) -> (Vec<Fragment>, f32) {
        let row = style.flex_direction.is_row();
        let reverse = style.flex_direction.is_reverse();
        let main_gap = if row { style.column_gap } else { style.row_gap }
            .resolve(width)
            .unwrap_or(0.0);
        let cross_gap = if row { style.row_gap } else { style.column_gap }
            .resolve(width)
            .unwrap_or(0.0);

        let mut items = self.flex_items(style, parent, children, x, y, depth);
        items.sort_by_key(|item| item.style.order);
        for item in &mut items {
            self.flex_base_size(item, row, width, height, depth);
        }

        // Taille principale du conteneur : sa largeur, ou sa hauteur si elle
        // est imposée ; sinon la somme des éléments.
        let container_main = if row { Some(width) } else { height };

        // Répartition en lignes.
        let mut lines: Vec<Vec<usize>> = Vec::new();
        let wraps = style.flex_wrap != FlexWrap::NoWrap;
        let mut current: Vec<usize> = Vec::new();
        let mut used = 0.0_f32;
        for (index, item) in items.iter().enumerate() {
            let outer = item.hypothetical + item.main_margins(row);
            let gap = if current.is_empty() { 0.0 } else { main_gap };
            if wraps
                && !current.is_empty()
                && container_main.is_some_and(|limit| used + gap + outer > limit + 0.01)
            {
                lines.push(std::mem::take(&mut current));
                used = 0.0;
            }
            used += if current.is_empty() {
                outer
            } else {
                main_gap + outer
            };
            current.push(index);
        }
        if !current.is_empty() {
            lines.push(current);
        }

        // Tailles principales, puis mise en page de chaque élément à cette taille.
        let mut line_main_used: Vec<f32> = Vec::with_capacity(lines.len());
        for line in &lines {
            #[expect(
                clippy::cast_precision_loss,
                reason = "nombre d'éléments petit devant 2^24"
            )]
            let gaps = main_gap * (line.len().saturating_sub(1)) as f32;
            match container_main {
                Some(available) => {
                    resolve_flexible_lengths(&mut items, line, available - gaps, row)
                }
                None => {
                    for index in line {
                        items[*index].main = items[*index].hypothetical;
                    }
                }
            }
            let total: f32 = line
                .iter()
                .map(|i| items[*i].main + items[*i].main_margins(row))
                .sum();
            line_main_used.push(total + gaps);
        }
        for item in &mut items {
            self.flex_layout_item(item, row, width, height, None, depth);
        }

        // Tailles transversales des lignes.
        let container_cross = if row { height } else { Some(width) };
        let mut line_cross: Vec<f32> = lines
            .iter()
            .map(|line| {
                line.iter()
                    .map(|i| items[*i].cross + items[*i].cross_margins(row))
                    .fold(0.0_f32, f32::max)
            })
            .collect();
        if let (Some(cross), [only]) = (container_cross, line_cross.as_mut_slice())
            && !wraps
        {
            // Une seule ligne : elle prend toute la taille transversale.
            *only = cross;
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "nombre de lignes petit devant 2^24"
        )]
        let cross_gaps = cross_gap * (lines.len().saturating_sub(1)) as f32;
        let lines_total: f32 = line_cross.iter().sum::<f32>() + cross_gaps;
        // `align-content: stretch` (par défaut) : les lignes se partagent le reste.
        let mut cross_free = container_cross.map_or(0.0, |cross| cross - lines_total);
        if wraps
            && cross_free > 0.0
            && matches!(style.align_content, Alignment::Normal | Alignment::Stretch)
            && !lines.is_empty()
        {
            #[expect(
                clippy::cast_precision_loss,
                reason = "nombre de lignes petit devant 2^24"
            )]
            let share = cross_free / lines.len() as f32;
            for cross in &mut line_cross {
                *cross += share;
            }
            cross_free = 0.0;
        }

        // Étirement : un élément sans taille transversale prend celle de sa ligne.
        for (line, cross) in lines.iter().zip(&line_cross) {
            for index in line {
                let item = &mut items[*index];
                let alignment = match item.style.align_self {
                    Alignment::Normal => style.align_items,
                    own => own,
                };
                let cross_size = if row {
                    item.style.height
                } else {
                    item.style.width
                };
                let auto_cross_margin = if row {
                    item.auto_margin[0] || item.auto_margin[2]
                } else {
                    item.auto_margin[1] || item.auto_margin[3]
                };
                if matches!(alignment, Alignment::Normal | Alignment::Stretch)
                    && cross_size == Size::Auto
                    && !auto_cross_margin
                {
                    let stretched = (*cross - item.cross_margins(row)).max(0.0);
                    let stretched = self.flex_clamp_cross(item, row, stretched, width, height);
                    if (stretched - item.cross).abs() > 0.01 {
                        self.flex_layout_item(item, row, width, height, Some(stretched), depth);
                    }
                }
            }
        }

        // Placement.
        let (content_main, content_cross) = {
            let longest = line_main_used.iter().copied().fold(0.0_f32, f32::max);
            let main = container_main.unwrap_or(longest);
            let cross = container_cross.unwrap_or(line_cross.iter().sum::<f32>() + cross_gaps);
            (main, cross)
        };
        let wrap_reverse = style.flex_wrap == FlexWrap::WrapReverse;
        let (mut cross_cursor, line_spacing) = if wraps {
            distribute(style.align_content, cross_free, lines.len())
        } else {
            (0.0, 0.0)
        };
        let mut fragments: Vec<Fragment> = Vec::with_capacity(items.len());
        let mut placed: Vec<(usize, Fragment)> = Vec::with_capacity(items.len());
        for ((line, cross), used) in lines.iter().zip(&line_cross).zip(&line_main_used) {
            let free = content_main - used;
            // Les marges automatiques de l'axe principal absorbent l'espace libre.
            let autos: usize = line
                .iter()
                .map(|i| {
                    let m = items[*i].auto_margin;
                    if row {
                        usize::from(m[1]) + usize::from(m[3])
                    } else {
                        usize::from(m[0]) + usize::from(m[2])
                    }
                })
                .sum();
            #[expect(
                clippy::cast_precision_loss,
                reason = "nombre de marges petit devant 2^24"
            )]
            let auto_share = if autos > 0 && free > 0.0 {
                free / autos as f32
            } else {
                0.0
            };
            let (mut main_cursor, spacing) = if autos > 0 && free > 0.0 {
                (0.0, 0.0)
            } else {
                distribute(style.justify_content, free, line.len())
            };
            for index in line {
                let item = &mut items[*index];
                let (start_auto, end_auto) = if row {
                    (item.auto_margin[3], item.auto_margin[1])
                } else {
                    (item.auto_margin[0], item.auto_margin[2])
                };
                let (margin_start, margin_end) = if row {
                    (item.margin[3], item.margin[1])
                } else {
                    (item.margin[0], item.margin[2])
                };
                if start_auto {
                    main_cursor += auto_share;
                }
                let main_position = main_cursor + margin_start;
                main_cursor = main_position + item.main + margin_end + main_gap + spacing;
                if end_auto {
                    main_cursor += auto_share;
                }

                // Axe transversal : alignement dans la ligne.
                let (cross_start_margin, cross_end_margin, cross_start_auto, cross_end_auto) =
                    if row {
                        (
                            item.margin[0],
                            item.margin[2],
                            item.auto_margin[0],
                            item.auto_margin[2],
                        )
                    } else {
                        (
                            item.margin[3],
                            item.margin[1],
                            item.auto_margin[3],
                            item.auto_margin[1],
                        )
                    };
                let cross_free_item = *cross - item.cross - cross_start_margin - cross_end_margin;
                let alignment = match item.style.align_self {
                    Alignment::Normal => style.align_items,
                    own => own,
                };
                let offset = match (cross_start_auto, cross_end_auto) {
                    (true, true) => cross_free_item.max(0.0) / 2.0,
                    (true, false) => cross_free_item.max(0.0),
                    (false, true) => 0.0,
                    (false, false) => match alignment {
                        Alignment::End => cross_free_item,
                        Alignment::Center => cross_free_item / 2.0,
                        _ => 0.0,
                    },
                };
                let offset = if wrap_reverse && !cross_start_auto && !cross_end_auto {
                    cross_free_item - offset
                } else {
                    offset
                };
                let cross_position = cross_cursor + cross_start_margin + offset;

                // Sens inversés : la position se lit depuis l'autre bord.
                let main_final = if reverse {
                    content_main - main_position - item.main
                } else {
                    main_position
                };
                let cross_final = if wrap_reverse {
                    content_cross - cross_position - item.cross
                } else {
                    cross_position
                };
                let (target_x, target_y) = if row {
                    (x + main_final, y + cross_final)
                } else {
                    (x + cross_final, y + main_final)
                };
                if let Some(fragment) = item.fragment.take() {
                    let (dx, dy) = (
                        target_x - fragment.border_box.x,
                        target_y - fragment.border_box.y,
                    );
                    let mut fragment = Fragment::Box(fragment);
                    fragment.translate(dx, dy);
                    // Les boîtes hors flux de l'élément le suivent.
                    for mut waiting in item.out_of_flow.drain(..) {
                        waiting.static_x += dx;
                        waiting.static_y += dy;
                        self.out_of_flow.push(waiting);
                    }
                    // Décalage relatif de l'élément lui-même.
                    if item.style.position == Position::Relative {
                        let inset = item.style.inset;
                        let rx = inset
                            .left
                            .resolve(width)
                            .or_else(|| inset.right.resolve(width).map(|r| -r))
                            .unwrap_or(0.0);
                        let ry = resolve_height(inset.top, height)
                            .or_else(|| resolve_height(inset.bottom, height).map(|b| -b))
                            .unwrap_or(0.0);
                        fragment.translate(rx, ry);
                    }
                    placed.push((*index, fragment));
                }
            }
            cross_cursor += *cross + cross_gap + line_spacing;
        }
        // Ordre de peinture : celui du document modifié par `order`.
        placed.sort_by_key(|(index, _)| *index);
        fragments.extend(placed.into_iter().map(|(_, fragment)| fragment));

        let content_height = if row { content_cross } else { content_main };
        (fragments, content_height.max(0.0))
    }

    /// Éléments flexibles de `children` : les éléments, et les suites de
    /// texte non vides, chacune dans un élément anonyme. Les boîtes hors flux
    /// sont mises en attente avec pour position statique l'origine du contenu.
    fn flex_items(
        &mut self,
        style: &Arc<ComputedStyle>,
        parent: NodeId,
        children: &[NodeId],
        x: f32,
        y: f32,
        depth: usize,
    ) -> Vec<Item> {
        let mut items = Vec::new();
        let mut text: Vec<NodeId> = Vec::new();
        let blank = |engine: &Self, nodes: &[NodeId]| {
            nodes.iter().all(|n| {
                engine
                    .doc
                    .text(*n)
                    .is_some_and(|t| t.chars().all(crate::collapsible_ws))
            })
        };
        let new_item = |node: Option<NodeId>, text: Vec<NodeId>, style: Arc<ComputedStyle>| Item {
            node,
            text,
            style,
            margin: [0.0; 4],
            auto_margin: [false; 4],
            main_extras: 0.0,
            cross_extras: 0.0,
            base: 0.0,
            hypothetical: 0.0,
            min: 0.0,
            max: f32::INFINITY,
            main: 0.0,
            cross: 0.0,
            frozen: false,
            fragment: None,
            parent,
            out_of_flow: Vec::new(),
        };
        // Style d'un élément anonyme : celui du conteneur, sans sa boîte.
        let anonymous = Arc::new(ComputedStyle::inherit_from(style));
        for child in children {
            if self.doc.text(*child).is_some() {
                text.push(*child);
                continue;
            }
            if !text.is_empty() {
                let run = std::mem::take(&mut text);
                if !blank(self, &run) {
                    items.push(new_item(None, run, Arc::clone(&anonymous)));
                }
            }
            let Some(child_style) = self.style(*child).cloned() else {
                continue;
            };
            if child_style.is_out_of_flow() {
                self.out_of_flow.push(OutOfFlow {
                    node: *child,
                    style: child_style,
                    static_x: x,
                    static_y: y,
                    depth,
                });
                continue;
            }
            items.push(new_item(Some(*child), Vec::new(), child_style));
        }
        if !text.is_empty() && !blank(self, &text) {
            items.push(new_item(None, text, anonymous));
        }
        items
    }

    /// Marges, taille de base, taille hypothétique et bornes de `item`.
    fn flex_base_size(
        &mut self,
        item: &mut Item,
        row: bool,
        width: f32,
        height: Option<f32>,
        depth: usize,
    ) {
        let style = Arc::clone(&item.style);
        let margin = style.margin;
        let sides = [margin.top, margin.right, margin.bottom, margin.left];
        for (index, side) in sides.into_iter().enumerate() {
            item.auto_margin[index] = side == Size::Auto;
            item.margin[index] = side.resolve(width).unwrap_or(0.0);
        }
        let pad = edges_px(style.padding, width);
        let bw = style.border_widths();
        let horizontal = pad.left + pad.right + bw.left + bw.right;
        let vertical = pad.top + pad.bottom + bw.top + bw.bottom;
        (item.main_extras, item.cross_extras) = if row {
            (horizontal, vertical)
        } else {
            (vertical, horizontal)
        };
        let extras = item.main_extras;
        // Taille de boîte de bordure d'une taille CSS sur l'axe principal.
        let outer = |size: f32| {
            if style.border_box {
                size.max(extras)
            } else {
                size + extras
            }
        };
        let resolve_main = |size: Size| {
            if row {
                size.resolve(width)
            } else {
                resolve_height(size, height)
            }
        };
        let main_size = if row { style.width } else { style.height };
        let specified = match style.flex_basis {
            Some(Size::Auto) => resolve_main(main_size),
            Some(basis) => resolve_main(basis),
            None => None,
        };

        // Mesure du contenu : la plus grande et la plus petite taille possibles.
        let (max_content, min_content) = self.flex_measure(item, row, width, height, depth);
        item.base = specified.map_or(max_content, outer);
        // Minimum automatique : le contenu le plus étroit, plafonné par la
        // taille demandée ; nul si le contenu peut défiler.
        let min_size = if row {
            style.min_width
        } else {
            style.min_height
        };
        let scrollable = if row {
            style.overflow_x
        } else {
            style.overflow_y
        } != Overflow::Visible;
        item.min = match resolve_main(min_size) {
            Some(min) => outer(min),
            None if scrollable => extras,
            None => {
                let content = min_content;
                match resolve_main(main_size) {
                    Some(size) => content.min(outer(size)),
                    None => content,
                }
            }
        };
        let max_size = if row {
            style.max_width
        } else {
            style.max_height
        };
        item.max = resolve_main(max_size).map_or(f32::INFINITY, outer);
        item.hypothetical = item.base.min(item.max).max(item.min);
    }

    /// Tailles de contenu maximale et minimale de `item` sur l'axe principal,
    /// en boîte de bordure.
    fn flex_measure(
        &mut self,
        item: &mut Item,
        row: bool,
        width: f32,
        height: Option<f32>,
        depth: usize,
    ) -> (f32, f32) {
        let Some(node) = item.node else {
            // Texte anonyme.
            let waiting = self.out_of_flow.len();
            let layout =
                self.layout_inline(&item.text, &item.style, item.parent, 0.0, 0.0, width, depth);
            self.out_of_flow.truncate(waiting);
            if row {
                let widths = layout.layout.calculate_content_widths();
                return (widths.max.ceil(), widths.min.ceil());
            }
            let h = layout.layout.height();
            return (h, h);
        };
        let style = Arc::clone(&item.style);
        let waiting = self.out_of_flow.len();
        let space = Space {
            cb_w: width,
            cb_x: 0.0,
            y: 0.0,
            mode: if row || style.width != Size::Auto {
                Width::ShrinkToFit
            } else {
                Width::Stretch
            },
            cb_h: height,
            fixed_h: None,
        };
        // En rangée, la mesure se fait sans limite de largeur : la taille de
        // base est celle du contenu le plus large.
        let space = if row {
            Space {
                cb_w: 1.0e6,
                ..space
            }
        } else {
            space
        };
        let fragment = self.layout_box(node, &style, space, depth);
        // Mesure seulement : les boîtes hors flux seront revues à la passe finale.
        self.out_of_flow.truncate(waiting);
        if !row {
            let h = fragment.border_box.h;
            return (h, h);
        }
        let inner_min = fragment
            .children
            .iter()
            .map(|c| self.min_content(c))
            .fold(0.0_f32, f32::max);
        // Le contenu le plus étroit ne dépend pas de la largeur demandée :
        // c'est lui qui borne le rétrécissement.
        let min = if self.intrinsic_size(node, &style).is_some() {
            // Élément remplacé ou champ de formulaire : sa taille par défaut.
            fragment.border_box.w
        } else {
            inner_min + item.main_extras
        };
        (fragment.border_box.w, min.min(fragment.border_box.w))
    }

    /// Met en page `item` à sa taille principale, et à la taille transversale
    /// `cross` si elle est imposée (étirement).
    fn flex_layout_item(
        &mut self,
        item: &mut Item,
        row: bool,
        width: f32,
        height: Option<f32>,
        cross: Option<f32>,
        depth: usize,
    ) {
        let style = Arc::clone(&item.style);
        let inner_main = (item.main - item.main_extras).max(0.0);
        let inner_cross = cross.map(|c| (c - item.cross_extras).max(0.0));
        let (content_w, fixed_h) = if row {
            (Some(inner_main), inner_cross)
        } else {
            (inner_cross, Some(inner_main))
        };
        let Some(node) = item.node else {
            // Texte anonyme : une boîte sans nœud autour des lignes.
            let w = content_w.unwrap_or(width);
            let waiting = self.out_of_flow.len();
            let layout = self.layout_inline(&item.text, &style, item.parent, 0.0, 0.0, w, depth);
            item.out_of_flow = self.out_of_flow.split_off(waiting);
            let used_w = if row || cross.is_some() {
                w
            } else {
                layout
                    .layout
                    .calculate_content_widths()
                    .max
                    .ceil()
                    .min(width)
            };
            let h = fixed_h.unwrap_or_else(|| layout.layout.height());
            item.fragment = Some(BoxFragment {
                node: None,
                style,
                border_box: Rect {
                    x: 0.0,
                    y: 0.0,
                    w: used_w,
                    h,
                },
                children: vec![Fragment::Text(Box::new(layout))],
            });
            item.cross = if row { h } else { used_w };
            return;
        };
        let waiting = self.out_of_flow.len();
        let mode = match content_w {
            Some(w) => Width::Fixed(w),
            // Colonne sans étirement : largeur demandée, ou ajustée au contenu.
            None if style.width == Size::Auto => Width::ShrinkToFit,
            None => Width::Stretch,
        };
        let space = Space {
            cb_w: width,
            cb_x: 0.0,
            y: 0.0,
            mode,
            cb_h: height,
            fixed_h,
        };
        let fragment = self.layout_box(node, &style, space, depth);
        // Les boîtes hors flux de cette passe remplacent celles de la précédente.
        item.out_of_flow = self.out_of_flow.split_off(waiting);
        item.cross = if row {
            fragment.border_box.h
        } else {
            fragment.border_box.w
        };
        if !row && fixed_h.is_none() {
            item.main = fragment.border_box.h;
        }
        item.fragment = Some(fragment);
    }

    /// Borne la taille transversale étirée `cross` par les minimums et
    /// maximums de l'élément.
    fn flex_clamp_cross(
        &self,
        item: &Item,
        row: bool,
        cross: f32,
        width: f32,
        height: Option<f32>,
    ) -> f32 {
        let style = &item.style;
        let extras = item.cross_extras;
        let outer = |size: f32| {
            if style.border_box {
                size.max(extras)
            } else {
                size + extras
            }
        };
        let (min, max) = if row {
            (
                resolve_height(style.min_height, height),
                resolve_height(style.max_height, height),
            )
        } else {
            (
                style.min_width.resolve(width),
                style.max_width.resolve(width),
            )
        };
        cross
            .min(max.map_or(f32::INFINITY, outer))
            .max(min.map_or(0.0, outer))
    }

    /// Largeur minimale de contenu de `fragment` : celle du mot le plus long
    /// pour un texte, la plus grande de ses enfants pour une boîte.
    fn min_content(&self, fragment: &Fragment) -> f32 {
        match fragment {
            Fragment::Text(t) => t.layout.calculate_content_widths().min.ceil(),
            Fragment::Box(b) => {
                let margins = b.style.margin.left.resolve(0.0).unwrap_or(0.0)
                    + b.style.margin.right.resolve(0.0).unwrap_or(0.0);
                if matches!(b.style.width, Size::Px(_))
                    || b.node
                        .is_some_and(|n| self.intrinsic_size(n, &b.style).is_some())
                {
                    return b.border_box.w + margins;
                }
                let pad = edges_px(b.style.padding, 0.0);
                let bw = b.style.border_widths();
                let inner = b
                    .children
                    .iter()
                    .map(|c| self.min_content(c))
                    .fold(0.0_f32, f32::max);
                let extras = pad.left + pad.right + bw.left + bw.right;
                crate::bounded_width(&b.style, inner, extras) + margins
            }
        }
    }
}

/// « Resolving flexible lengths » (§9.7) pour les éléments `line`, dans
/// l'espace principal `available` (espaces entre éléments déjà retirés).
fn resolve_flexible_lengths(items: &mut [Item], line: &[usize], available: f32, row: bool) {
    let outer_hypothetical: f32 = line
        .iter()
        .map(|i| items[*i].hypothetical + items[*i].main_margins(row))
        .sum();
    let growing = outer_hypothetical < available;
    let factor = |item: &Item| {
        if growing {
            item.style.flex_grow
        } else {
            item.style.flex_shrink
        }
    };
    // Gel initial : les éléments inflexibles, et ceux que leur taille
    // hypothétique place déjà du mauvais côté de leur taille de base.
    for index in line {
        let item = &mut items[*index];
        item.frozen = factor(item) == 0.0
            || (growing && item.base > item.hypothetical)
            || (!growing && item.base < item.hypothetical);
        item.main = if item.frozen {
            item.hypothetical
        } else {
            item.base
        };
    }
    let used = |items: &[Item]| -> f32 {
        line.iter()
            .map(|i| {
                let item = &items[*i];
                let size = if item.frozen { item.main } else { item.base };
                size + item.main_margins(row)
            })
            .sum()
    };
    let initial_free = available - used(items);
    // Chaque tour gèle au moins un élément.
    for _ in 0..=line.len() {
        let unfrozen: Vec<usize> = line.iter().copied().filter(|i| !items[*i].frozen).collect();
        if unfrozen.is_empty() {
            break;
        }
        let mut free = available - used(items);
        let sum: f32 = unfrozen.iter().map(|i| factor(&items[*i])).sum();
        if sum < 1.0 {
            let scaled = initial_free * sum;
            if scaled.abs() < free.abs() {
                free = scaled;
            }
        }
        if growing {
            for index in &unfrozen {
                let item = &mut items[*index];
                item.main = item.base
                    + if sum > 0.0 {
                        free * item.style.flex_grow / sum
                    } else {
                        0.0
                    };
            }
        } else {
            // Le rétrécissement est proportionnel à la taille de base du contenu.
            let scaled_sum: f32 = unfrozen
                .iter()
                .map(|i| {
                    items[*i].style.flex_shrink * (items[*i].base - items[*i].main_extras).max(0.0)
                })
                .sum();
            for index in &unfrozen {
                let item = &mut items[*index];
                let scaled = item.style.flex_shrink * (item.base - item.main_extras).max(0.0);
                item.main = item.base
                    + if scaled_sum > 0.0 {
                        free.min(0.0) * scaled / scaled_sum
                    } else {
                        0.0
                    };
            }
        }
        // Bornes : la somme des dépassements dit qui geler.
        let mut violation = 0.0_f32;
        let mut clamped: Vec<(usize, f32)> = Vec::with_capacity(unfrozen.len());
        for index in &unfrozen {
            let item = &items[*index];
            let bounded = item.main.min(item.max).max(item.min).max(item.main_extras);
            violation += bounded - item.main;
            clamped.push((*index, bounded));
        }
        for (index, bounded) in clamped {
            let item = &mut items[index];
            let delta = bounded - item.main;
            let freeze = violation.abs() < 0.001
                || (violation > 0.0 && delta > 0.0)
                || (violation < 0.0 && delta < 0.0);
            if freeze {
                item.main = bounded;
                item.frozen = true;
            }
        }
    }
    for index in line {
        let item = &mut items[*index];
        if !item.frozen {
            item.main = item.main.min(item.max).max(item.min);
        }
    }
}
