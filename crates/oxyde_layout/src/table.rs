//! Mise en page des tableaux (CSS 2 §17, CSS Tables 3 en version simplifiée).
//!
//! Le tableau est d'abord *planifié* — structure, grille des cellules,
//! largeur des colonnes — au moment où sa boîte calcule sa largeur, puis mis
//! en page : cellules, hauteur des rangées, alignement vertical.
//!
//! LIMITES CONNUES : la légende est placée dans la boîte du tableau et non
//! dans une boîte englobante ; `<col>` et `<colgroup>` sont ignorés ; les
//! bordures fusionnées (`border-collapse: collapse`) sont approchées en
//! retirant la bordure en double de chaque arête, sans arbitrage de style ni
//! de largeur ; l'alignement sur la ligne de base revient à un alignement en
//! haut ; un pourcentage porté par une cellule fusionnée est ignoré ; le
//! texte et les boîtes hors flux placés directement dans un tableau ou une
//! rangée ne sont pas mis en page.

use std::ops::Range;
use std::sync::Arc;

use oxyde_css::{BorderStyle, ComputedStyle, Display, Edges, Size, VerticalAlign};
use oxyde_dom::NodeId;

use crate::{BoxFragment, Engine, Fragment, OutOfFlow, Rect, Space, Width, edges_px, px};

/// Nombre maximal de colonnes d'un tableau.
const MAX_COLUMNS: usize = 1000;

/// Rangée telle qu'elle apparaît dans le document.
pub(crate) struct RowSource {
    /// Élément de la rangée ; `None` pour une rangée anonyme.
    pub(crate) node: Option<NodeId>,
    /// Ses cellules, dans l'ordre du document.
    pub(crate) cells: Vec<NodeId>,
}

/// Groupe de rangées tel qu'il apparaît dans le document.
pub(crate) struct GroupSource {
    /// Élément du groupe ; `None` pour un groupe anonyme.
    pub(crate) node: Option<NodeId>,
    /// Ses rangées.
    pub(crate) rows: Vec<RowSource>,
}

/// Contenu d'un tableau : légendes et groupes de rangées, en-tête en premier
/// et pied en dernier.
pub(crate) struct TableSource {
    /// Légendes, dans l'ordre du document.
    pub(crate) captions: Vec<NodeId>,
    /// Groupes de rangées, dans l'ordre d'affichage.
    pub(crate) groups: Vec<GroupSource>,
}

struct Cell {
    node: NodeId,
    style: Arc<ComputedStyle>,
    row: usize,
    col: usize,
    row_span: usize,
    col_span: usize,
}

struct Row {
    node: Option<NodeId>,
    style: Option<Arc<ComputedStyle>>,
}

struct Group {
    node: Option<NodeId>,
    style: Option<Arc<ComputedStyle>>,
    rows: Range<usize>,
}

/// Tableau planifié : sa grille et la largeur de ses colonnes.
pub(crate) struct TablePlan {
    captions: Vec<NodeId>,
    groups: Vec<Group>,
    rows: Vec<Row>,
    cells: Vec<Cell>,
    columns: Vec<f32>,
    /// Espacement horizontal et vertical entre cellules.
    spacing: (f32, f32),
    collapse: bool,
}

/// Largeurs de contenu des colonnes d'un tableau en disposition automatique.
struct ColumnSizes {
    min: Vec<f32>,
    max: Vec<f32>,
    /// Pourcentage demandé ; 0 : aucun.
    percent: Vec<f32>,
    /// Vrai si une cellule de la colonne impose une largeur en pixels.
    fixed: Vec<bool>,
}

/// Répartit `extra` sur `values`, au prorata de `weights` (à parts égales si
/// tous les poids sont nuls).
fn spread(values: &mut [f32], extra: f32, weights: &[f32]) {
    let total: f32 = weights.iter().sum();
    #[expect(
        clippy::cast_precision_loss,
        reason = "nombre de colonnes borné par MAX_COLUMNS"
    )]
    let count = values.len().max(1) as f32;
    for (value, weight) in values.iter_mut().zip(weights) {
        *value += if total > 0.0 {
            extra * weight / total
        } else {
            extra / count
        };
    }
}

/// Nombre converti en flottant ; les grandeurs en jeu (colonnes, rangées)
/// restent très en deçà de 2^24.
#[expect(
    clippy::cast_precision_loss,
    reason = "nombre de colonnes ou de rangées petit devant 2^24"
)]
fn float(count: usize) -> f32 {
    count as f32
}

/// Largeur de boîte de bordure d'une largeur spécifiée `value`.
fn outer_width(style: &ComputedStyle, value: f32, extras: f32) -> f32 {
    if style.border_box {
        value.max(extras)
    } else {
        value + extras
    }
}

/// Marges intérieures et bordures horizontales d'une cellule ; un
/// pourcentage de marge intérieure compte pour zéro tant que la colonne
/// n'est pas dimensionnée.
fn horizontal_extras(style: &ComputedStyle) -> f32 {
    let pad = edges_px(style.padding, 0.0);
    let bw = style.border_widths();
    pad.left + pad.right + bw.left + bw.right
}

/// Copie de `style` sans les bordures des côtés marqués dans `drop`
/// (haut, droite, bas, gauche) ; le style d'origine s'il n'y a rien à ôter.
fn without_borders(style: &Arc<ComputedStyle>, drop: [bool; 4]) -> Arc<ComputedStyle> {
    let bw = style.border_widths();
    let present = [bw.top, bw.right, bw.bottom, bw.left];
    if !drop.iter().zip(present).any(|(d, w)| *d && w > 0.0) {
        return Arc::clone(style);
    }
    let mut copy = ComputedStyle::clone(style);
    if drop[0] {
        copy.border_style.top = BorderStyle::None;
    }
    if drop[1] {
        copy.border_style.right = BorderStyle::None;
    }
    if drop[2] {
        copy.border_style.bottom = BorderStyle::None;
    }
    if drop[3] {
        copy.border_style.left = BorderStyle::None;
    }
    Arc::new(copy)
}

/// Déplace une boîte et son contenu.
fn shift(fragment: &mut BoxFragment, dx: f32, dy: f32) {
    fragment.border_box.x += dx;
    fragment.border_box.y += dy;
    for child in &mut fragment.children {
        child.translate(dx, dy);
    }
}

/// Largeur des colonnes d'un tableau automatique de largeur de grille
/// `target` (espacements exclus), d'après CSS 2 §17.5.2.2.
fn distribute(sizes: &ColumnSizes, target: f32) -> Vec<f32> {
    let count = sizes.min.len();
    let mut percent_left = 100.0_f32;
    let mut widths: Vec<f32> = (0..count)
        .map(|i| {
            if sizes.percent[i] > 0.0 {
                let share = sizes.percent[i].min(percent_left);
                percent_left -= share;
                (target * share / 100.0).max(sizes.min[i])
            } else {
                sizes.max[i]
            }
        })
        .collect();
    let total: f32 = widths.iter().sum();
    let is_auto = |i: usize| sizes.percent[i] <= 0.0 && !sizes.fixed[i];
    let is_fixed = |i: usize| sizes.percent[i] <= 0.0 && sizes.fixed[i];
    let is_percent = |i: usize| sizes.percent[i] > 0.0;
    if total < target {
        // La place en trop va aux colonnes automatiques, à défaut aux
        // colonnes de largeur imposée, à défaut aux colonnes en pourcentage.
        let extra = target - total;
        let pick: Vec<usize> = [
            (0..count).filter(|i| is_auto(*i)).collect::<Vec<_>>(),
            (0..count).filter(|i| is_fixed(*i)).collect(),
            (0..count).filter(|i| is_percent(*i)).collect(),
        ]
        .into_iter()
        .find(|group| !group.is_empty())
        .unwrap_or_default();
        let weights: Vec<f32> = pick
            .iter()
            .map(|i| {
                if is_percent(*i) {
                    sizes.percent[*i]
                } else {
                    sizes.max[*i]
                }
            })
            .collect();
        let mut grown: Vec<f32> = pick.iter().map(|i| widths[*i]).collect();
        spread(&mut grown, extra, &weights);
        for (i, value) in pick.into_iter().zip(grown) {
            widths[i] = value;
        }
    } else if total > target {
        // La place manquante est prise aux colonnes automatiques d'abord,
        // sans descendre sous leur largeur minimale.
        let mut over = total - target;
        let groups: [&dyn Fn(usize) -> bool; 3] = [&is_auto, &is_fixed, &is_percent];
        for belongs in groups {
            if over <= 0.0 {
                break;
            }
            let slack: f32 = (0..count)
                .filter(|i| belongs(*i))
                .map(|i| (widths[i] - sizes.min[i]).max(0.0))
                .sum();
            if slack <= 0.0 {
                continue;
            }
            let take = over.min(slack);
            for i in (0..count).filter(|i| belongs(*i)) {
                let own = (widths[i] - sizes.min[i]).max(0.0);
                widths[i] -= take * own / slack;
            }
            over -= take;
        }
    }
    widths
}

impl Engine<'_> {
    /// Enfants de `node` qui sont des boîtes dans le flux.
    fn table_boxes(&self, node: NodeId, depth: usize) -> Vec<NodeId> {
        let mut all = Vec::new();
        self.flow_children(node, &mut all, depth);
        all.retain(|c| {
            self.doc.element(*c).is_some() && self.style(*c).is_some_and(|s| !s.is_out_of_flow())
        });
        all
    }

    fn display_of(&self, node: NodeId) -> Display {
        self.style(node).map_or(Display::None, |s| s.display)
    }

    /// Rangée faite des enfants de `node`.
    pub(crate) fn table_row_source(&self, node: NodeId, depth: usize) -> RowSource {
        RowSource {
            node: Some(node),
            cells: self.table_boxes(node, depth),
        }
    }

    /// Rangées d'un groupe ; les cellules orphelines forment des rangées
    /// anonymes.
    fn table_rows_of(&self, group: NodeId, depth: usize) -> Vec<RowSource> {
        let mut rows = Vec::new();
        let mut loose = Vec::new();
        for child in self.table_boxes(group, depth) {
            if self.display_of(child) == Display::TableRow {
                if !loose.is_empty() {
                    rows.push(RowSource {
                        node: None,
                        cells: std::mem::take(&mut loose),
                    });
                }
                rows.push(self.table_row_source(child, depth + 1));
            } else {
                loose.push(child);
            }
        }
        if !loose.is_empty() {
            rows.push(RowSource {
                node: None,
                cells: loose,
            });
        }
        rows
    }

    /// Structure du tableau `node` : légendes, puis groupes de rangées avec
    /// l'en-tête (`<thead>`) en premier et le pied (`<tfoot>`) en dernier.
    fn table_source(&self, node: NodeId, depth: usize) -> TableSource {
        fn flush_cells(cells: &mut Vec<NodeId>, rows: &mut Vec<RowSource>) {
            if !cells.is_empty() {
                rows.push(RowSource {
                    node: None,
                    cells: std::mem::take(cells),
                });
            }
        }
        fn flush_rows(rows: &mut Vec<RowSource>, groups: &mut Vec<GroupSource>) {
            if !rows.is_empty() {
                groups.push(GroupSource {
                    node: None,
                    rows: std::mem::take(rows),
                });
            }
        }
        let mut captions = Vec::new();
        let mut header = None;
        let mut footer = None;
        let mut bodies = Vec::new();
        let mut loose_rows = Vec::new();
        let mut loose_cells = Vec::new();
        for child in self.table_boxes(node, depth) {
            match self.display_of(child) {
                Display::TableCaption => captions.push(child),
                Display::TableRowGroup => {
                    flush_cells(&mut loose_cells, &mut loose_rows);
                    flush_rows(&mut loose_rows, &mut bodies);
                    let group = GroupSource {
                        node: Some(child),
                        rows: self.table_rows_of(child, depth + 1),
                    };
                    let name = self.doc.element(child).map(|e| e.name.as_str());
                    match name {
                        Some("thead") if header.is_none() => header = Some(group),
                        Some("tfoot") if footer.is_none() => footer = Some(group),
                        _ => bodies.push(group),
                    }
                }
                Display::TableRow => {
                    flush_cells(&mut loose_cells, &mut loose_rows);
                    loose_rows.push(self.table_row_source(child, depth + 1));
                }
                _ => loose_cells.push(child),
            }
        }
        flush_cells(&mut loose_cells, &mut loose_rows);
        flush_rows(&mut loose_rows, &mut bodies);
        let groups = header.into_iter().chain(bodies).chain(footer).collect();
        TableSource { captions, groups }
    }

    /// Valeur de l'attribut `colspan` ou `rowspan` de `node` ; 1 à défaut.
    fn span_attribute(&self, node: NodeId, name: &str) -> usize {
        self.doc
            .element(node)
            .and_then(|e| e.attr(name))
            .and_then(|v| v.trim().parse::<usize>().ok())
            .unwrap_or(1)
    }

    /// Largeurs de contenu minimale et maximale d'une cellule, sans ses
    /// marges intérieures ni ses bordures. Le résultat ne dépend pas de la
    /// place disponible : il est gardé d'une passe à l'autre, ce qui évite
    /// de mesurer un tableau imbriqué à chaque niveau.
    fn table_cell_content(
        &mut self,
        node: NodeId,
        style: &Arc<ComputedStyle>,
        depth: usize,
    ) -> (f32, f32) {
        if let Some(sizes) = self.cell_sizes.get(&node) {
            return *sizes;
        }
        let unbounded = 1.0e6;
        let waiting = self.out_of_flow.len();
        let (children, _) = self.layout_children(node, style, 0.0, 0.0, unbounded, None, depth + 1);
        self.out_of_flow.truncate(waiting);
        let max = self.max_content_of(style, &children, unbounded).ceil();
        let min = children
            .iter()
            .map(|c| self.min_content(c))
            .fold(0.0_f32, f32::max)
            .min(max);
        self.cell_sizes.insert(node, (min, max));
        (min, max)
    }

    /// Planifie le tableau porté par l'élément `node` ; voir
    /// [`Engine::plan_table`].
    #[inline(never)]
    pub(crate) fn plan_table_box(
        &mut self,
        node: NodeId,
        style: &Arc<ComputedStyle>,
        wanted: Option<f32>,
        available: f32,
        floor: f32,
        depth: usize,
    ) -> f32 {
        let source = self.table_source(node, depth);
        self.plan_table(node, style, source, wanted, available, floor, depth)
    }

    /// Planifie un tableau : place les cellules sur la grille et dimensionne
    /// les colonnes. `wanted` est la largeur de contenu imposée, s'il y en a
    /// une ; sinon le tableau s'ajuste à son contenu dans la limite de
    /// `available`, sans descendre sous `floor`. Renvoie la largeur de
    /// contenu retenue ; le plan attend [`Engine::layout_table`].
    #[expect(
        clippy::too_many_arguments,
        reason = "contraintes de largeur explicites"
    )]
    pub(crate) fn plan_table(
        &mut self,
        node: NodeId,
        style: &Arc<ComputedStyle>,
        source: TableSource,
        wanted: Option<f32>,
        available: f32,
        floor: f32,
        depth: usize,
    ) -> f32 {
        let TableSource { captions, groups } = source;
        // Une légende de largeur imposée élargit le tableau au besoin.
        let floor = captions
            .iter()
            .filter_map(|c| self.style(*c))
            .filter_map(|s| match s.width {
                Size::Px(v) => Some(
                    outer_width(s, v, horizontal_extras(s))
                        + px(s.margin.left, 0.0)
                        + px(s.margin.right, 0.0),
                ),
                _ => None,
            })
            .fold(floor, f32::max);
        // Grille : `owner[rangée][colonne]` est la cellule qui occupe la case.
        let mut owner: Vec<Vec<Option<usize>>> = Vec::new();
        let mut rows: Vec<Row> = Vec::new();
        let mut cells: Vec<Cell> = Vec::new();
        let mut placed_groups: Vec<Group> = Vec::new();
        for group in groups {
            let start = rows.len();
            let end = start + group.rows.len();
            owner.resize_with(end, Vec::new);
            for (offset, row) in group.rows.into_iter().enumerate() {
                let r = start + offset;
                rows.push(Row {
                    node: row.node,
                    style: row.node.and_then(|n| self.style(n).cloned()),
                });
                let mut col = 0;
                for cell in row.cells {
                    let Some(cell_style) = self.style(cell).cloned() else {
                        continue;
                    };
                    while owner[r].get(col).is_some_and(Option::is_some) {
                        col += 1;
                    }
                    if col >= MAX_COLUMNS {
                        break;
                    }
                    let col_span = self
                        .span_attribute(cell, "colspan")
                        .clamp(1, MAX_COLUMNS - col);
                    // `rowspan=0` : jusqu'à la fin du groupe de rangées.
                    let row_span = match self.span_attribute(cell, "rowspan") {
                        0 => end - r,
                        n => n.min(end - r),
                    };
                    let index = cells.len();
                    for line in &mut owner[r..r + row_span] {
                        if line.len() < col + col_span {
                            line.resize(col + col_span, None);
                        }
                        for slot in &mut line[col..col + col_span] {
                            slot.get_or_insert(index);
                        }
                    }
                    cells.push(Cell {
                        node: cell,
                        style: cell_style,
                        row: r,
                        col,
                        row_span,
                        col_span,
                    });
                    col += col_span;
                }
            }
            placed_groups.push(Group {
                node: group.node,
                style: group.node.and_then(|n| self.style(n).cloned()),
                rows: start..end,
            });
        }
        let count = owner.iter().map(Vec::len).max().unwrap_or(0);
        let collapse = style.border_collapse;
        if collapse {
            Self::collapse_borders(style, &rows, &mut cells, &owner, count);
        }
        let spacing = if collapse {
            (0.0, 0.0)
        } else {
            style.border_spacing
        };
        let gaps = if count == 0 {
            0.0
        } else {
            spacing.0 * float(count + 1)
        };

        let (columns, width) = match wanted {
            Some(wanted) if style.table_layout_fixed => {
                let target = (wanted - gaps).max(0.0);
                let columns = Self::fixed_columns(&cells, count, target, spacing.0);
                let used = columns.iter().sum::<f32>() + gaps;
                self.table_sizes.insert(node, (used, used));
                (columns, wanted.max(used).max(floor))
            }
            _ => {
                let sizes = self.measure_columns(&cells, count, spacing.0, depth);
                let min_total = sizes.min.iter().sum::<f32>() + gaps;
                let max_total = sizes.max.iter().sum::<f32>() + gaps;
                self.table_sizes.insert(node, (min_total, max_total));
                let width = wanted.unwrap_or_else(|| {
                    // Une colonne en pourcentage élargit le tableau jusqu'à
                    // ce que son contenu tienne dans la part demandée.
                    let mut natural = max_total;
                    let mut percent_sum = 0.0_f32;
                    let mut rest = 0.0_f32;
                    for i in 0..count {
                        if sizes.percent[i] > 0.0 {
                            percent_sum += sizes.percent[i];
                            natural = natural.max(sizes.max[i] * 100.0 / sizes.percent[i] + gaps);
                        } else {
                            rest += sizes.max[i];
                        }
                    }
                    if percent_sum > 0.0 && rest > 0.0 {
                        natural = natural.max(if percent_sum < 100.0 {
                            rest * 100.0 / (100.0 - percent_sum) + gaps
                        } else {
                            f32::INFINITY
                        });
                    }
                    natural.min(available).max(floor)
                });
                let width = width.max(min_total).max(floor);
                (distribute(&sizes, (width - gaps).max(0.0)), width)
            }
        };
        self.tables.insert(
            node,
            TablePlan {
                captions,
                groups: placed_groups,
                rows,
                cells,
                columns,
                spacing,
                collapse,
            },
        );
        width
    }

    /// Bordures fusionnées : sur chaque arête partagée, une seule bordure
    /// reste — celle de la cellule de gauche ou du dessus, celle de la rangée
    /// ou celle du tableau sur le pourtour.
    fn collapse_borders(
        table: &ComputedStyle,
        rows: &[Row],
        cells: &mut [Cell],
        owner: &[Vec<Option<usize>>],
        count: usize,
    ) {
        let table_bw = table.border_widths();
        let originals: Vec<Edges<f32>> = cells.iter().map(|c| c.style.border_widths()).collect();
        let row_bw: Vec<Edges<f32>> = rows
            .iter()
            .map(|r| {
                r.style
                    .as_ref()
                    .map_or(Edges::all(0.0), |s| s.border_widths())
            })
            .collect();
        for (index, cell) in cells.iter_mut().enumerate() {
            let neighbour = |r: usize, c: usize| {
                owner
                    .get(r)
                    .and_then(|line| line.get(c))
                    .copied()
                    .flatten()
                    .filter(|other| *other != index)
            };
            let (row, col) = (cell.row, cell.col);
            let last_row = row + cell.row_span - 1;
            let last_col = col + cell.col_span - 1;
            let drop_left = if col == 0 {
                table_bw.left > 0.0
            } else {
                neighbour(row, col - 1).is_some_and(|other| originals[other].right > 0.0)
            };
            let above = if row == 0 {
                table_bw.top > 0.0
            } else {
                neighbour(row - 1, col).is_some_and(|other| originals[other].bottom > 0.0)
                    || row_bw[row - 1].bottom > 0.0
            };
            let drop_top = above || row_bw[row].top > 0.0;
            let drop_right = last_col + 1 == count && table_bw.right > 0.0;
            let drop_bottom = (last_row + 1 == rows.len() && table_bw.bottom > 0.0)
                || row_bw[last_row].bottom > 0.0;
            cell.style =
                without_borders(&cell.style, [drop_top, drop_right, drop_bottom, drop_left]);
        }
    }

    /// Colonnes d'un tableau en disposition fixe (CSS 2 §17.5.2.1) : seules
    /// les cellules de la première rangée comptent.
    fn fixed_columns(cells: &[Cell], count: usize, target: f32, spacing: f32) -> Vec<f32> {
        let mut widths: Vec<Option<f32>> = vec![None; count];
        for cell in cells.iter().filter(|c| c.row == 0) {
            let extras = horizontal_extras(&cell.style);
            let width = match cell.style.width {
                Size::Px(v) => Some(outer_width(&cell.style, v, extras)),
                Size::Percent(p) => Some(target * p / 100.0),
                _ => None,
            };
            if let Some(width) = width {
                let each =
                    ((width - spacing * float(cell.col_span - 1)) / float(cell.col_span)).max(0.0);
                for slot in &mut widths[cell.col..cell.col + cell.col_span] {
                    *slot = Some(each);
                }
            }
        }
        let known: f32 = widths.iter().flatten().sum();
        let unknown = widths.iter().filter(|w| w.is_none()).count();
        if unknown > 0 {
            let share = ((target - known) / float(unknown)).max(0.0);
            widths.into_iter().map(|w| w.unwrap_or(share)).collect()
        } else if known < target && known > 0.0 {
            widths
                .into_iter()
                .map(|w| w.unwrap_or(0.0) * target / known)
                .collect()
        } else {
            widths.into_iter().map(|w| w.unwrap_or(0.0)).collect()
        }
    }

    /// Largeurs minimale et maximale de chaque colonne, d'après le contenu
    /// des cellules (CSS 2 §17.5.2.2).
    fn measure_columns(
        &mut self,
        cells: &[Cell],
        count: usize,
        spacing: f32,
        depth: usize,
    ) -> ColumnSizes {
        let mut sizes = ColumnSizes {
            min: vec![0.0; count],
            max: vec![0.0; count],
            percent: vec![0.0; count],
            fixed: vec![false; count],
        };
        // Cellules fusionnées : (rang, largeur minimale, largeur maximale).
        let mut spanning: Vec<(usize, f32, f32)> = Vec::new();
        for (index, cell) in cells.iter().enumerate() {
            let (content_min, content_max) = self.table_cell_content(cell.node, &cell.style, depth);
            let style = &cell.style;
            let extras = horizontal_extras(style);
            let mut min = content_min + extras;
            let mut max = content_max + extras;
            let mut percent = 0.0;
            let mut fixed = false;
            match style.width {
                // Une largeur imposée fixe la largeur souhaitée ; le contenu
                // insécable reste un plancher.
                Size::Px(v) => {
                    max = outer_width(style, v, extras);
                    fixed = true;
                }
                Size::Percent(p) => percent = p.max(0.0),
                _ => {}
            }
            if let Size::Px(v) = style.min_width {
                min = min.max(outer_width(style, v, extras));
            }
            max = max.max(min);
            if cell.col_span == 1 {
                let c = cell.col;
                sizes.min[c] = sizes.min[c].max(min);
                sizes.max[c] = sizes.max[c].max(max);
                sizes.percent[c] = sizes.percent[c].max(percent);
                sizes.fixed[c] |= fixed;
            } else {
                spanning.push((index, min, max));
            }
        }
        // Les cellules fusionnées élargissent les colonnes qu'elles couvrent
        // si celles-ci ne leur suffisent pas ; les moins étendues d'abord.
        spanning.sort_by_key(|(index, _, _)| cells[*index].col_span);
        for (index, min, max) in spanning {
            let cell = &cells[index];
            let range = cell.col..cell.col + cell.col_span;
            let inner_gaps = spacing * float(cell.col_span - 1);
            let weights: Vec<f32> = sizes.max[range.clone()].to_vec();
            let have_min = sizes.min[range.clone()].iter().sum::<f32>() + inner_gaps;
            if min > have_min {
                spread(&mut sizes.min[range.clone()], min - have_min, &weights);
            }
            let have_max = sizes.max[range.clone()].iter().sum::<f32>() + inner_gaps;
            if max > have_max {
                spread(&mut sizes.max[range], max - have_max, &weights);
            }
        }
        for (max, min) in sizes.max.iter_mut().zip(&sizes.min) {
            *max = max.max(*min);
        }
        sizes
    }

    /// Met en page une légende de tableau sous l'ordonnée `cursor` ; renvoie
    /// l'ordonnée qui la suit.
    #[expect(clippy::too_many_arguments, reason = "boîte de contenu explicite")]
    fn table_caption(
        &mut self,
        node: NodeId,
        style: &Arc<ComputedStyle>,
        x: f32,
        cursor: f32,
        width: f32,
        depth: usize,
        fragments: &mut Vec<Fragment>,
    ) -> f32 {
        let space = Space {
            cb_w: width,
            cb_x: x,
            y: cursor + px(style.margin.top, width),
            mode: Width::Stretch,
            cb_h: None,
            fixed_h: None,
        };
        let fragment = self.layout_box(node, style, space, depth);
        let end = fragment.border_box.y + fragment.border_box.h + px(style.margin.bottom, width);
        fragments.push(Fragment::Box(fragment));
        end
    }

    /// Rangée hors de tout tableau : elle forme un tableau anonyme.
    #[inline(never)]
    pub(crate) fn layout_stray_row(
        &mut self,
        node: NodeId,
        style: &Arc<ComputedStyle>,
        x: f32,
        y: f32,
        width: f32,
        depth: usize,
    ) -> (Vec<Fragment>, f32) {
        let mut row = self.table_row_source(node, depth);
        row.node = None;
        let source = TableSource {
            captions: Vec::new(),
            groups: vec![GroupSource {
                node: None,
                rows: vec![row],
            }],
        };
        self.plan_table(node, style, source, None, width, 0.0, depth);
        self.layout_table(node, x, y, width, None, depth)
    }

    /// Met en page le tableau planifié pour `node` dans la boîte de contenu
    /// d'origine (`x`, `y`) et de largeur `width` ; `height` est sa hauteur
    /// de contenu si elle est imposée. Renvoie les fragments et la hauteur.
    #[inline(never)]
    pub(crate) fn layout_table(
        &mut self,
        node: NodeId,
        x: f32,
        y: f32,
        width: f32,
        height: Option<f32>,
        depth: usize,
    ) -> (Vec<Fragment>, f32) {
        let Some(plan) = self.tables.remove(&node) else {
            return (Vec::new(), 0.0);
        };
        let TablePlan {
            captions,
            groups,
            rows,
            cells,
            columns,
            spacing: (hs, vs),
            collapse,
        } = plan;
        let mut fragments = Vec::new();
        let mut cursor = y;
        let mut bottom_captions = Vec::new();
        for caption in captions {
            let Some(style) = self.style(caption).cloned() else {
                continue;
            };
            if style.caption_bottom {
                bottom_captions.push((caption, style));
            } else {
                cursor =
                    self.table_caption(caption, &style, x, cursor, width, depth, &mut fragments);
            }
        }

        let mut column_x = Vec::with_capacity(columns.len());
        let mut next_x = x + hs;
        for column in &columns {
            column_x.push(next_x);
            next_x += column + hs;
        }
        let grid_w = columns.iter().sum::<f32>() + hs * float(columns.len().saturating_sub(1));

        // Première passe : chaque cellule à la largeur de ses colonnes, pour
        // connaître sa hauteur.
        struct Placed {
            fragment: BoxFragment,
            out_of_flow: Vec<OutOfFlow>,
            /// Hauteur de la cellule à la hauteur de son contenu.
            natural: f32,
            /// Hauteur qu'elle demande à ses rangées.
            wanted: f32,
        }
        let mut placed: Vec<Placed> = Vec::with_capacity(cells.len());
        for cell in &cells {
            let span = cell.col..cell.col + cell.col_span;
            let outer = columns[span].iter().sum::<f32>() + hs * float(cell.col_span - 1);
            let pad = edges_px(cell.style.padding, outer);
            let bw = cell.style.border_widths();
            let inner = (outer - pad.left - pad.right - bw.left - bw.right).max(0.0);
            let waiting = self.out_of_flow.len();
            let space = Space {
                cb_w: outer,
                cb_x: column_x[cell.col],
                y: 0.0,
                mode: Width::Fixed(inner),
                cb_h: None,
                fixed_h: None,
            };
            // La hauteur demandée par la cellule est un minimum pour sa
            // rangée : la cellule est mise en page à la hauteur de son
            // contenu, que l'alignement vertical place ensuite.
            let wanted_h = match cell.style.height {
                Size::Px(h) if cell.style.border_box => h,
                Size::Px(h) => h + pad.top + pad.bottom + bw.top + bw.bottom,
                _ => 0.0,
            };
            let auto_height = if cell.style.height == Size::Auto {
                Arc::clone(&cell.style)
            } else {
                let mut copy = ComputedStyle::clone(&cell.style);
                copy.height = Size::Auto;
                Arc::new(copy)
            };
            let mut fragment = self.layout_box(cell.node, &auto_height, space, depth);
            let mut out_of_flow = self.out_of_flow.split_off(waiting);
            // Une cellule n'a pas de marge : elle occupe toute sa case.
            let dx = column_x[cell.col] - fragment.border_box.x;
            shift(&mut fragment, dx, 0.0);
            for waiting in &mut out_of_flow {
                waiting.static_x += dx;
            }
            let natural = fragment.border_box.h;
            placed.push(Placed {
                fragment,
                out_of_flow,
                natural,
                wanted: natural.max(wanted_h),
            });
        }

        // Hauteur des rangées : la plus haute cellule, ou la hauteur imposée.
        let mut row_h: Vec<f32> = rows
            .iter()
            .map(|row| match row.style.as_ref().map(|s| s.height) {
                Some(Size::Px(h)) => h.max(0.0),
                _ => 0.0,
            })
            .collect();
        for (cell, item) in cells.iter().zip(&placed) {
            if cell.row_span == 1 {
                row_h[cell.row] = row_h[cell.row].max(item.wanted);
            }
        }
        for (cell, item) in cells.iter().zip(&placed) {
            if cell.row_span > 1 {
                let span = cell.row..cell.row + cell.row_span;
                let have = row_h[span.clone()].iter().sum::<f32>() + vs * float(cell.row_span - 1);
                if item.wanted > have {
                    let each = (item.wanted - have) / float(cell.row_span);
                    for h in &mut row_h[span] {
                        *h += each;
                    }
                }
            }
        }

        // Bordures de rangée : peintes seulement en mode fusionné, au-dessus
        // et au-dessous des cellules.
        let row_styles: Vec<Option<Arc<ComputedStyle>>> = rows
            .iter()
            .enumerate()
            .map(|(r, row)| {
                let style = row.style.as_ref()?;
                let doubled = r > 0
                    && rows[r - 1]
                        .style
                        .as_ref()
                        .is_some_and(|s| s.border_widths().bottom > 0.0);
                let drop = [!collapse || doubled, true, !collapse, true];
                Some(without_borders(style, drop))
            })
            .collect();
        let row_border: Vec<(f32, f32)> = row_styles
            .iter()
            .map(|style| {
                style.as_ref().map_or((0.0, 0.0), |s| {
                    let bw = s.border_widths();
                    (bw.top, bw.bottom)
                })
            })
            .collect();

        // Une hauteur imposée au tableau se répartit entre les rangées.
        if let Some(wanted) = height
            && !rows.is_empty()
        {
            let borders: f32 = row_border.iter().map(|(top, bottom)| top + bottom).sum();
            let used =
                (cursor - y) + row_h.iter().sum::<f32>() + borders + vs * float(rows.len() + 1);
            if wanted > used {
                let weights = row_h.clone();
                spread(&mut row_h, wanted - used, &weights);
            }
        }

        let grid_top = cursor;
        let mut row_y = Vec::with_capacity(rows.len());
        if !rows.is_empty() {
            cursor += vs;
        }
        for (h, (top, bottom)) in row_h.iter().zip(&row_border) {
            cursor += top;
            row_y.push(cursor);
            cursor += h + bottom + vs;
        }

        // Seconde passe : chaque cellule prend la hauteur de ses rangées et
        // son contenu s'aligne verticalement.
        let mut row_children: Vec<Vec<Fragment>> = rows.iter().map(|_| Vec::new()).collect();
        for (cell, mut item) in cells.iter().zip(placed) {
            let last = cell.row + cell.row_span - 1;
            let top = row_y[cell.row];
            let total = row_y[last] + row_h[last] - top;
            let free = (total - item.natural).max(0.0);
            let offset = match cell.style.vertical_align {
                VerticalAlign::Middle => free / 2.0,
                VerticalAlign::Bottom => free,
                _ => 0.0,
            };
            shift(&mut item.fragment, 0.0, top);
            for child in &mut item.fragment.children {
                child.translate(0.0, offset);
            }
            item.fragment.border_box.h = total.max(item.natural);
            for mut waiting in item.out_of_flow {
                waiting.static_y += top + offset;
                self.out_of_flow.push(waiting);
            }
            row_children[cell.row].push(Fragment::Box(item.fragment));
        }

        let grid_x = x + hs;
        for group in groups {
            let mut children = Vec::new();
            for r in group.rows.clone() {
                let cells = std::mem::take(&mut row_children[r]);
                match &row_styles[r] {
                    Some(style) => {
                        let (top, bottom) = row_border[r];
                        children.push(Fragment::Box(BoxFragment {
                            node: rows[r].node,
                            style: Arc::clone(style),
                            border_box: Rect {
                                x: grid_x,
                                y: row_y[r] - top,
                                w: grid_w,
                                h: row_h[r] + top + bottom,
                            },
                            children: cells,
                        }));
                    }
                    None => children.extend(cells),
                }
            }
            match &group.style {
                Some(style) if !group.rows.is_empty() => {
                    let (first, last) = (group.rows.start, group.rows.end - 1);
                    let top = row_y[first] - row_border[first].0;
                    let bottom = row_y[last] + row_h[last] + row_border[last].1;
                    fragments.push(Fragment::Box(BoxFragment {
                        node: group.node,
                        style: without_borders(style, [true; 4]),
                        border_box: Rect {
                            x: grid_x,
                            y: top,
                            w: grid_w,
                            h: bottom - top,
                        },
                        children,
                    }));
                }
                _ => fragments.extend(children),
            }
        }

        // La hauteur imposée vaut pour la grille, légendes exclues.
        if let Some(wanted) = height {
            cursor = cursor.max(grid_top + wanted);
        }
        for (caption, style) in bottom_captions {
            cursor = self.table_caption(caption, &style, x, cursor, width, depth, &mut fragments);
        }
        (fragments, (cursor - y).max(0.0))
    }
}
