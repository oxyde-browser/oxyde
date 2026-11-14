//! Mise en page en grille (CSS Grid Layout Level 2).
//!
//! Dans l'ordre du standard : grille explicite (répétitions développées,
//! lignes nommées), placement des éléments (§8.5, placement automatique
//! clairsemé ou dense), dimensionnement des pistes (§12 : tailles de base,
//! contributions du contenu, pistes flexibles, étirement), puis alignement
//! des pistes dans le conteneur et des éléments dans leur zone.
//!
//! LIMITES CONNUES : écriture horizontale seulement ; pas de `subgrid` ; une
//! boîte hors flux se place dans le conteneur, pas dans sa zone de grille ;
//! la contribution d'un élément qui couvre plusieurs pistes est répartie à
//! parts égales entre elles ; `align-self: baseline` aligne au début.

use std::sync::Arc;

use oxyde_css::{
    Alignment, Breadth, ComputedStyle, GridAreas, GridLine, Position, Repeat, Size, TrackEntry,
    TrackList, TrackSize,
};
use oxyde_dom::NodeId;

use crate::flex::distribute;
use crate::{
    BoxFragment, Engine, Fragment, OutOfFlow, Rect, Space, Width, edges_px, resolve_height,
};

/// Nombre maximal de pistes sur un axe : borne la mémoire face à
/// `grid-column: 100000`.
const MAX_TRACKS: i32 = 2_000;

/// Élément de grille.
struct Item {
    node: Option<NodeId>,
    text: Vec<NodeId>,
    style: Arc<ComputedStyle>,
    /// Lignes de début et de fin sur chaque axe, à partir de 0 ; `None` tant
    /// que le placement automatique n'a pas tranché.
    rows: Option<(i32, i32)>,
    columns: Option<(i32, i32)>,
    row_span: i32,
    column_span: i32,
    /// Marges résolues : haut, droite, bas, gauche.
    margin: [f32; 4],
    auto_margin: [bool; 4],
    fragment: Option<BoxFragment>,
    out_of_flow: Vec<OutOfFlow>,
    /// Conteneur : le nœud auquel le texte anonyme est attribué.
    parent: NodeId,
}

/// Grille explicite d'un axe : ses pistes et les noms de ses lignes.
struct Explicit {
    tracks: Vec<TrackSize<Size>>,
    /// (nom, rang de ligne à partir de 0).
    names: Vec<(String, i32)>,
    /// Pistes issues d'un `repeat(auto-fit, …)` : repliées si elles restent vides.
    auto_fit: std::ops::Range<usize>,
}

/// Taille d'une piste si elle est fixée par une longueur, pour compter les
/// répétitions automatiques.
fn definite_size(track: TrackSize<Size>, available: Option<f32>) -> f32 {
    let resolve = |b: Breadth<Size>| match b {
        Breadth::Length(size) => match size {
            Size::Px(v) => Some(v),
            other => available.and_then(|a| other.resolve(a)),
        },
        _ => None,
    };
    resolve(track.max)
        .or_else(|| resolve(track.min))
        .unwrap_or(0.0)
}

/// Développe `list` : répétitions fixes, répétition automatique selon
/// l'espace `available`, lignes nommées.
fn expand(list: &TrackList<Size>, available: Option<f32>, gap: f32) -> Explicit {
    // Nombre de répétitions automatiques : autant que l'espace en accepte.
    let (mut fixed_sum, mut fixed_count, mut repeat_sum, mut repeat_len) =
        (0.0, 0usize, 0.0, 0usize);
    for entry in &list.entries {
        match entry {
            TrackEntry::Track(track) => {
                fixed_sum += definite_size(*track, available);
                fixed_count += 1;
            }
            TrackEntry::Repeat(Repeat::Count(n), tracks) => {
                let once: f32 = tracks.iter().map(|t| definite_size(*t, available)).sum();
                #[expect(clippy::cast_precision_loss, reason = "au plus 10 000 répétitions")]
                {
                    fixed_sum += once * *n as f32;
                }
                fixed_count += tracks.len() * *n as usize;
            }
            TrackEntry::Repeat(_, tracks) => {
                repeat_sum = tracks.iter().map(|t| definite_size(*t, available)).sum();
                repeat_len = tracks.len();
            }
            TrackEntry::Names(_) => {}
        }
    }
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "nombres de pistes petits, résultat borné"
    )]
    let auto_count = match available {
        Some(space) if repeat_len > 0 => {
            let per_repeat = repeat_sum + gap * repeat_len as f32;
            let free = space - fixed_sum - gap * fixed_count as f32 + gap;
            if per_repeat > 0.0 {
                ((free / per_repeat).floor().max(1.0) as usize).min(MAX_TRACKS as usize)
            } else {
                1
            }
        }
        _ => 1,
    };
    let mut explicit = Explicit {
        tracks: Vec::new(),
        names: Vec::new(),
        auto_fit: 0..0,
    };
    for entry in &list.entries {
        match entry {
            TrackEntry::Track(track) => explicit.tracks.push(*track),
            TrackEntry::Names(names) => {
                let line = i32::try_from(explicit.tracks.len()).unwrap_or(i32::MAX);
                explicit
                    .names
                    .extend(names.iter().map(|name| (name.clone(), line)));
            }
            TrackEntry::Repeat(repeat, tracks) => {
                let count = match repeat {
                    Repeat::Count(n) => *n as usize,
                    _ => auto_count,
                };
                let start = explicit.tracks.len();
                for _ in 0..count {
                    if explicit.tracks.len() + tracks.len() > MAX_TRACKS as usize {
                        break;
                    }
                    explicit.tracks.extend(tracks.iter().copied());
                }
                if *repeat == Repeat::AutoFit {
                    explicit.auto_fit = start..explicit.tracks.len();
                }
            }
        }
    }
    explicit
}

/// Résout une ligne de placement en rang de ligne à partir de 0 ; `None`
/// pour `auto`, un `span` ou un nom inconnu.
fn resolve_line(
    line: &GridLine,
    explicit: &Explicit,
    areas: &GridAreas,
    row_axis: bool,
    end: bool,
) -> Option<i32> {
    let last = i32::try_from(explicit.tracks.len()).unwrap_or(i32::MAX);
    match line {
        GridLine::Auto | GridLine::Span(..) => None,
        GridLine::Line(n, None) => Some(if *n > 0 { n - 1 } else { last + 1 + n }),
        GridLine::Line(n, Some(name)) => {
            let mut matching: Vec<i32> = explicit
                .names
                .iter()
                .filter(|(known, _)| known == name)
                .map(|(_, line)| *line)
                .collect();
            if *n < 0 {
                matching.reverse();
            }
            let wanted = usize::try_from(n.unsigned_abs().saturating_sub(1)).ok()?;
            matching.get(wanted).copied()
        }
        GridLine::Named(name) => {
            // Une zone nommée, puis une ligne de ce nom ou du nom suffixé.
            if let Some((top, bottom, left, right)) = areas.area(name) {
                let edge = match (row_axis, end) {
                    (true, false) => top,
                    (true, true) => bottom,
                    (false, false) => left,
                    (false, true) => right,
                };
                return Some(edge - 1);
            }
            let suffixed = format!("{name}-{}", if end { "end" } else { "start" });
            explicit
                .names
                .iter()
                .find(|(known, _)| *known == suffixed || known == name)
                .map(|(_, line)| *line)
        }
    }
}

/// Étendue d'un élément sur un axe : lignes de début et de fin si elles sont
/// fixées, et nombre de pistes couvertes.
fn resolve_span(
    start: &GridLine,
    end: &GridLine,
    explicit: &Explicit,
    areas: &GridAreas,
    row_axis: bool,
) -> (Option<(i32, i32)>, i32) {
    let span_of = |line: &GridLine| match line {
        GridLine::Span(n, _) => i32::try_from(*n).unwrap_or(1).clamp(1, MAX_TRACKS),
        _ => 1,
    };
    let first = resolve_line(start, explicit, areas, row_axis, false);
    let second = resolve_line(end, explicit, areas, row_axis, true);
    let bounded = |a: i32, b: i32| {
        let (a, b) = (
            a.clamp(-MAX_TRACKS, MAX_TRACKS),
            b.clamp(-MAX_TRACKS, MAX_TRACKS),
        );
        match a.cmp(&b) {
            std::cmp::Ordering::Less => (a, b),
            std::cmp::Ordering::Equal => (a, a + 1),
            std::cmp::Ordering::Greater => (b, a),
        }
    };
    match (first, second) {
        (Some(a), Some(b)) => {
            let (a, b) = bounded(a, b);
            (Some((a, b)), b - a)
        }
        (Some(a), None) => {
            let span = span_of(end);
            let (a, b) = bounded(a, a + span);
            (Some((a, b)), b - a)
        }
        (None, Some(b)) => {
            let span = span_of(start);
            let (a, b) = bounded(b - span, b);
            (Some((a, b)), b - a)
        }
        (None, None) => (None, span_of(start).max(span_of(end))),
    }
}

/// Cases occupées de la grille, pour le placement automatique.
#[derive(Default)]
struct Occupancy {
    cells: std::collections::HashSet<(i32, i32)>,
}

impl Occupancy {
    fn fits(&self, major: i32, minor: i32, major_span: i32, minor_span: i32) -> bool {
        (major..major + major_span)
            .all(|a| (minor..minor + minor_span).all(|b| !self.cells.contains(&(a, b))))
    }

    fn mark(&mut self, major: (i32, i32), minor: (i32, i32)) {
        for a in major.0..major.1 {
            for b in minor.0..minor.1 {
                self.cells.insert((a, b));
            }
        }
    }
}

/// Placement automatique (§8.5). L'axe « majeur » est celui que le flux
/// remplit en avançant (les rangées pour `grid-auto-flow: row`).
fn place(items: &mut [Item], minor_count: i32, column_flow: bool, dense: bool) {
    // Accès aux étendues selon l'axe majeur.
    let major_of = |item: &Item| if column_flow { item.columns } else { item.rows };
    let minor_of = |item: &Item| if column_flow { item.rows } else { item.columns };
    let spans_of = |item: &Item| {
        if column_flow {
            (item.column_span, item.row_span)
        } else {
            (item.row_span, item.column_span)
        }
    };
    let set = |item: &mut Item, major: (i32, i32), minor: (i32, i32)| {
        if column_flow {
            item.columns = Some(major);
            item.rows = Some(minor);
        } else {
            item.rows = Some(major);
            item.columns = Some(minor);
        }
    };
    let mut grid = Occupancy::default();
    // Largeur de l'axe mineur : la grille explicite, élargie par les
    // éléments déjà fixés et par le plus large des éléments libres.
    let mut minor_count = minor_count.max(1);
    let mut minor_start = 0;
    for item in items.iter() {
        if let Some((a, b)) = minor_of(item) {
            minor_start = minor_start.min(a);
            minor_count = minor_count.max(b);
        } else {
            minor_count = minor_count.max(spans_of(item).1);
        }
    }
    // 1. Éléments fixés sur les deux axes.
    for item in items.iter() {
        if let (Some(major), Some(minor)) = (major_of(item), minor_of(item)) {
            grid.mark(major, minor);
        }
    }
    // 2. Éléments fixés sur l'axe majeur : première place libre sur l'autre.
    for item in items.iter_mut() {
        let (Some(major), None) = (major_of(item), minor_of(item)) else {
            continue;
        };
        let (_, minor_span) = spans_of(item);
        let mut minor = minor_start;
        while !grid.fits(major.0, minor, major.1 - major.0, minor_span) {
            minor += 1;
            if minor > MAX_TRACKS {
                break;
            }
        }
        minor_count = minor_count.max(minor + minor_span);
        let minor = (minor, minor + minor_span);
        grid.mark(major, minor);
        set(item, major, minor);
    }
    // 3. Les autres, dans l'ordre, derrière un curseur.
    let (mut cursor_major, mut cursor_minor) = (0, minor_start);
    for item in items.iter_mut() {
        if major_of(item).is_some() {
            continue;
        }
        let (major_span, minor_span) = spans_of(item);
        if dense {
            (cursor_major, cursor_minor) = (0, minor_start);
        }
        let placed = if let Some(minor) = minor_of(item) {
            // Fixé sur l'axe mineur : on descend jusqu'à une place libre.
            if minor.0 < cursor_minor {
                cursor_major += 1;
            }
            cursor_minor = minor.0;
            while !grid.fits(cursor_major, minor.0, major_span, minor.1 - minor.0)
                && cursor_major < MAX_TRACKS
            {
                cursor_major += 1;
            }
            ((cursor_major, cursor_major + major_span), minor)
        } else {
            loop {
                if cursor_minor + minor_span > minor_count {
                    cursor_major += 1;
                    cursor_minor = minor_start;
                }
                if grid.fits(cursor_major, cursor_minor, major_span, minor_span)
                    || cursor_major >= MAX_TRACKS
                {
                    break;
                }
                cursor_minor += 1;
            }
            (
                (cursor_major, cursor_major + major_span),
                (cursor_minor, cursor_minor + minor_span),
            )
        };
        grid.mark(placed.0, placed.1);
        set(item, placed.0, placed.1);
    }
}

/// Piste en cours de dimensionnement.
#[derive(Clone, Copy)]
struct Track {
    size: TrackSize<Size>,
    base: f32,
    limit: f32,
}

impl Track {
    fn flex(&self) -> Option<f32> {
        match self.size.max {
            Breadth::Fr(factor) => Some(factor),
            _ => None,
        }
    }
}

/// Contribution d'un élément à ses pistes sur un axe : étendue, puis tailles
/// extérieures minimale et maximale de son contenu.
struct Contribution {
    start: usize,
    end: usize,
    min: f32,
    max: f32,
}

/// Dimensionne les pistes d'un axe (§12.3 à §12.8). `available` : taille du
/// conteneur sur cet axe si elle est imposée ; `stretch` : les pistes `auto`
/// se partagent l'espace restant.
fn size_tracks(
    sizes: &[TrackSize<Size>],
    contributions: &[Contribution],
    available: Option<f32>,
    gap: f32,
    stretch: bool,
) -> Vec<f32> {
    let resolve = |size: Size| match size {
        Size::Px(v) => Some(v),
        Size::Auto => None,
        other => available.and_then(|a| other.resolve(a)),
    };
    let mut tracks: Vec<Track> = sizes
        .iter()
        .map(|size| {
            let base = match size.min {
                Breadth::Length(l) => resolve(l).unwrap_or(0.0),
                _ => 0.0,
            };
            let limit = match size.max {
                Breadth::Length(l) => resolve(l).unwrap_or(f32::INFINITY),
                _ => f32::INFINITY,
            };
            Track {
                size: *size,
                base,
                limit: limit.max(base),
            }
        })
        .collect();
    // Une longueur en pourcentage sans base vaut `auto`.
    let intrinsic_min = |track: &Track| match track.size.min {
        Breadth::Length(l) => resolve(l).is_none(),
        Breadth::Fr(_) => false,
        _ => true,
    };
    let intrinsic_max = |track: &Track| match track.size.max {
        Breadth::Length(l) => resolve(l).is_none(),
        Breadth::Fr(_) => false,
        _ => true,
    };
    // Contributions du contenu : d'abord les éléments d'une seule piste,
    // puis ceux qui en couvrent plusieurs, par étendue croissante.
    let mut order: Vec<&Contribution> = contributions.iter().collect();
    order.sort_by_key(|c| c.end - c.start);
    for contribution in order {
        let range = contribution.start..contribution.end.min(tracks.len());
        if range.is_empty() {
            continue;
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "nombre de pistes petit devant 2^24"
        )]
        let gaps = gap * (range.len() - 1) as f32;
        // Un élément qui couvre une piste flexible ne compte que pour ses minimums.
        let spans_flex = tracks[range.clone()].iter().any(|t| t.flex().is_some());
        // Tailles de base : ce qui manque au contenu minimal est réparti
        // entre les pistes à minimum intrinsèque.
        let growable: Vec<usize> = range
            .clone()
            .filter(|i| intrinsic_min(&tracks[*i]) || (spans_flex && tracks[*i].flex().is_some()))
            .collect();
        if !growable.is_empty() {
            let wanted = |track: &Track| match track.size.min {
                Breadth::MaxContent => contribution.max,
                _ => contribution.min,
            };
            let target = growable
                .iter()
                .map(|i| wanted(&tracks[*i]))
                .fold(0.0_f32, f32::max);
            let current: f32 = tracks[range.clone()].iter().map(|t| t.base).sum::<f32>() + gaps;
            let missing = target - current;
            if missing > 0.0 {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "nombre de pistes petit devant 2^24"
                )]
                let share = missing / growable.len() as f32;
                for index in &growable {
                    tracks[*index].base += share;
                }
            }
        }
        // Limites de croissance : le contenu maximal, pour les pistes à
        // maximum intrinsèque.
        if !spans_flex {
            let growable: Vec<usize> = range
                .clone()
                .filter(|i| intrinsic_max(&tracks[*i]))
                .collect();
            if !growable.is_empty() {
                let current: f32 = tracks[range.clone()]
                    .iter()
                    .map(|t| if t.limit.is_finite() { t.limit } else { t.base })
                    .sum::<f32>()
                    + gaps;
                let wanted = growable
                    .iter()
                    .map(|i| match tracks[*i].size.max {
                        Breadth::MinContent => contribution.min,
                        _ => contribution.max,
                    })
                    .fold(0.0_f32, f32::max);
                let missing = (wanted - current).max(0.0);
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "nombre de pistes petit devant 2^24"
                )]
                let share = missing / growable.len() as f32;
                for index in growable {
                    let track = &mut tracks[index];
                    let from = if track.limit.is_finite() {
                        track.limit
                    } else {
                        track.base
                    };
                    let mut limit = from + share;
                    if let Breadth::FitContent(cap) = track.size.max {
                        limit = limit.min(resolve(cap).unwrap_or(f32::INFINITY));
                    }
                    track.limit = limit;
                }
            }
        }
    }
    for track in &mut tracks {
        if track.limit < track.base || (!track.limit.is_finite() && track.flex().is_none()) {
            track.limit = track.base;
        }
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "nombre de pistes petit devant 2^24"
    )]
    let gaps = gap * tracks.len().saturating_sub(1) as f32;
    // Maximisation : l'espace libre fait grandir les pistes jusqu'à leur limite.
    if let Some(space) = available {
        let mut free = space - gaps - tracks.iter().map(|t| t.base).sum::<f32>();
        // Chaque tour sature au moins une piste.
        for _ in 0..tracks.len() {
            let open: Vec<usize> = (0..tracks.len())
                .filter(|i| tracks[*i].flex().is_none() && tracks[*i].base < tracks[*i].limit)
                .collect();
            if open.is_empty() || free <= 0.001 {
                break;
            }
            #[expect(
                clippy::cast_precision_loss,
                reason = "nombre de pistes petit devant 2^24"
            )]
            let share = free / open.len() as f32;
            for index in open {
                let track = &mut tracks[index];
                let grown = (track.base + share).min(track.limit);
                free -= grown - track.base;
                track.base = grown;
            }
        }
    } else {
        for track in &mut tracks {
            if track.flex().is_none() {
                track.base = track.limit;
            }
        }
    }
    // Pistes flexibles : la fraction est ce qui reste, divisé par la somme
    // des facteurs ; une piste dont la base dépasse sa part devient fixe.
    let flexible: Vec<usize> = (0..tracks.len())
        .filter(|i| tracks[*i].flex().is_some())
        .collect();
    if !flexible.is_empty() {
        let fraction = match available {
            Some(space) => {
                let mut frozen = vec![false; tracks.len()];
                let mut fraction = 0.0_f32;
                for _ in 0..=flexible.len() {
                    let fixed: f32 = (0..tracks.len())
                        .filter(|i| tracks[*i].flex().is_none() || frozen[*i])
                        .map(|i| tracks[i].base)
                        .sum();
                    let factors: f32 = flexible
                        .iter()
                        .filter(|i| !frozen[**i])
                        .filter_map(|i| tracks[*i].flex())
                        .sum();
                    fraction = (space - gaps - fixed).max(0.0) / factors.max(1.0);
                    let overflowing: Vec<usize> = flexible
                        .iter()
                        .copied()
                        .filter(|i| {
                            !frozen[*i]
                                && tracks[*i].base > fraction * tracks[*i].flex().unwrap_or(0.0)
                        })
                        .collect();
                    if overflowing.is_empty() {
                        break;
                    }
                    for index in overflowing {
                        frozen[index] = true;
                    }
                }
                fraction
            }
            // Taille non imposée : la plus grande part qu'exige un contenu.
            None => flexible
                .iter()
                .map(|i| {
                    let factor = tracks[*i].flex().unwrap_or(0.0);
                    if factor > 0.0 {
                        tracks[*i].base / factor.max(1.0)
                    } else {
                        0.0
                    }
                })
                .fold(0.0_f32, f32::max),
        };
        for index in flexible {
            let factor = tracks[index].flex().unwrap_or(0.0);
            tracks[index].base = tracks[index].base.max(fraction * factor);
        }
    }
    // Étirement : les pistes `auto` se partagent ce qui reste.
    if let (Some(space), true) = (available, stretch) {
        let free = space - gaps - tracks.iter().map(|t| t.base).sum::<f32>();
        let autos: Vec<usize> = (0..tracks.len())
            .filter(|i| matches!(tracks[*i].size.max, Breadth::Auto))
            .collect();
        if free > 0.0 && !autos.is_empty() {
            #[expect(
                clippy::cast_precision_loss,
                reason = "nombre de pistes petit devant 2^24"
            )]
            let share = free / autos.len() as f32;
            for index in autos {
                tracks[index].base += share;
            }
        }
    }
    tracks.iter().map(|t| t.base.max(0.0)).collect()
}

/// Position de chaque piste dans le conteneur : `sizes` séparées de `gap`,
/// l'espace libre réparti selon `alignment`.
fn track_offsets(
    sizes: &[f32],
    gap: f32,
    available: Option<f32>,
    alignment: Alignment,
) -> Vec<f32> {
    #[expect(
        clippy::cast_precision_loss,
        reason = "nombre de pistes petit devant 2^24"
    )]
    let used = sizes.iter().sum::<f32>() + gap * sizes.len().saturating_sub(1) as f32;
    let free = available.map_or(0.0, |space| space - used);
    let (mut cursor, spacing) = distribute(alignment, free, sizes.len());
    let mut offsets = Vec::with_capacity(sizes.len());
    for size in sizes {
        offsets.push(cursor);
        cursor += size + gap + spacing;
    }
    offsets
}

impl Engine<'_> {
    /// Met en page les enfants d'un conteneur en grille dont la boîte de
    /// contenu est en (`x`, `y`), de largeur `width` et de hauteur `height`
    /// si elle est imposée. Renvoie les fragments et la hauteur de contenu.
    #[expect(clippy::too_many_arguments, reason = "boîte de contenu explicite")]
    pub(crate) fn layout_grid(
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
        let column_gap = style.column_gap.resolve(width).unwrap_or(0.0);
        let row_gap = match style.row_gap {
            Size::Auto => 0.0,
            Size::Px(v) => v,
            other => height.and_then(|h| other.resolve(h)).unwrap_or(0.0),
        };
        let columns = expand(&style.grid_template_columns, Some(width), column_gap);
        let rows = expand(&style.grid_template_rows, height, row_gap);
        let areas = &style.grid_template_areas;
        // Les zones nommées ajoutent des pistes implicites si la grille
        // explicite en a moins.
        let area_rows = i32::try_from(areas.rows.len()).unwrap_or(0);
        let area_columns = i32::try_from(areas.rows.first().map_or(0, Vec::len)).unwrap_or(0);

        let mut items = self.grid_items(style, parent, children, x, y, depth);
        items.sort_by_key(|item| item.style.order);
        for item in &mut items {
            let s = Arc::clone(&item.style);
            (item.rows, item.row_span) =
                resolve_span(&s.grid_row_start, &s.grid_row_end, &rows, areas, true);
            (item.columns, item.column_span) = resolve_span(
                &s.grid_column_start,
                &s.grid_column_end,
                &columns,
                areas,
                false,
            );
            let margin = s.margin;
            for (index, side) in [margin.top, margin.right, margin.bottom, margin.left]
                .into_iter()
                .enumerate()
            {
                item.auto_margin[index] = side == Size::Auto;
                item.margin[index] = side.resolve(width).unwrap_or(0.0);
            }
        }
        let explicit_columns = i32::try_from(columns.tracks.len())
            .unwrap_or(0)
            .max(area_columns);
        let explicit_rows = i32::try_from(rows.tracks.len()).unwrap_or(0).max(area_rows);
        let column_flow = style.grid_auto_flow_column;
        let minor_count = if column_flow {
            explicit_rows
        } else {
            explicit_columns
        };
        place(
            &mut items,
            minor_count,
            column_flow,
            style.grid_auto_flow_dense,
        );

        // Étendue de la grille : explicite, plus les pistes implicites que le
        // placement a ouvertes avant ou après.
        let (mut first_row, mut last_row) = (0, explicit_rows);
        let (mut first_column, mut last_column) = (0, explicit_columns);
        for item in &items {
            if let Some((a, b)) = item.rows {
                first_row = first_row.min(a);
                last_row = last_row.max(b);
            }
            if let Some((a, b)) = item.columns {
                first_column = first_column.min(a);
                last_column = last_column.max(b);
            }
        }
        let axis_sizes = |explicit: &Explicit,
                          auto: &[TrackSize<Size>],
                          first: i32,
                          last: i32,
                          used: &dyn Fn(i32) -> bool| {
            let pattern = |offset: i32| {
                if auto.is_empty() {
                    TrackSize::auto()
                } else {
                    let len = i32::try_from(auto.len()).unwrap_or(1);
                    auto[usize::try_from(offset.rem_euclid(len)).unwrap_or(0)]
                }
            };
            let explicit_len = i32::try_from(explicit.tracks.len()).unwrap_or(0);
            (first..last)
                .map(|index| {
                    if index < 0 {
                        // Avant la grille explicite : le motif se termine à son bord.
                        let len = i32::try_from(auto.len().max(1)).unwrap_or(1);
                        pattern(index.rem_euclid(len))
                    } else if index < explicit_len {
                        let position = usize::try_from(index).unwrap_or(0);
                        // Piste `auto-fit` restée vide : repliée à zéro.
                        if explicit.auto_fit.contains(&position) && !used(index) {
                            TrackSize {
                                min: Breadth::Length(Size::Px(0.0)),
                                max: Breadth::Length(Size::Px(0.0)),
                            }
                        } else {
                            explicit.tracks[position]
                        }
                    } else {
                        pattern(index - explicit_len)
                    }
                })
                .collect::<Vec<TrackSize<Size>>>()
        };
        let column_used = |index: i32| {
            items
                .iter()
                .any(|item| item.columns.is_some_and(|(a, b)| a <= index && index < b))
        };
        let row_used = |index: i32| {
            items
                .iter()
                .any(|item| item.rows.is_some_and(|(a, b)| a <= index && index < b))
        };
        let column_sizes = axis_sizes(
            &columns,
            &style.grid_auto_columns,
            first_column,
            last_column,
            &column_used,
        );
        let row_sizes = axis_sizes(&rows, &style.grid_auto_rows, first_row, last_row, &row_used);
        let position = |line: i32, first: i32| usize::try_from(line - first).unwrap_or(0);

        // Colonnes : contributions en largeur du contenu de chaque élément.
        let mut contributions = Vec::with_capacity(items.len());
        for item in &mut items {
            let Some((a, b)) = item.columns else { continue };
            let (min, max) = self.grid_measure(item, width, height, depth);
            let margins = item.margin[1] + item.margin[3];
            contributions.push(Contribution {
                start: position(a, first_column),
                end: position(b, first_column),
                min: min + margins,
                max: max + margins,
            });
        }
        let stretch_columns = matches!(
            style.justify_content,
            Alignment::Normal | Alignment::Stretch
        );
        // Largeur de contenu de la grille : ses colonnes sans contrainte.
        #[expect(
            clippy::cast_precision_loss,
            reason = "nombre de pistes petit devant 2^24"
        )]
        let max_content = size_tracks(&column_sizes, &contributions, None, column_gap, false)
            .iter()
            .sum::<f32>()
            + column_gap * column_sizes.len().saturating_sub(1) as f32;
        let column_widths = size_tracks(
            &column_sizes,
            &contributions,
            Some(width),
            column_gap,
            stretch_columns,
        );
        let column_offsets = track_offsets(
            &column_widths,
            column_gap,
            Some(width),
            style.justify_content,
        );
        let span_size = |sizes: &[f32], offsets: &[f32], start: usize, end: usize| {
            let end = end.min(sizes.len());
            if start >= end {
                return (offsets.get(start).copied().unwrap_or(0.0), 0.0);
            }
            let origin = offsets[start];
            (origin, offsets[end - 1] + sizes[end - 1] - origin)
        };

        // Chaque élément est mis en page à la largeur de sa zone : sa hauteur
        // nourrit le dimensionnement des rangées.
        let mut contributions = Vec::with_capacity(items.len());
        for item in &mut items {
            let (Some((ca, cb)), Some((ra, rb))) = (item.columns, item.rows) else {
                continue;
            };
            let (_, area_w) = span_size(
                &column_widths,
                &column_offsets,
                position(ca, first_column),
                position(cb, first_column),
            );
            self.grid_layout_item(item, style, area_w, None, width, depth);
            let h = item.fragment.as_ref().map_or(0.0, |f| f.border_box.h);
            let margins = item.margin[0] + item.margin[2];
            contributions.push(Contribution {
                start: position(ra, first_row),
                end: position(rb, first_row),
                min: h + margins,
                max: h + margins,
            });
        }
        let stretch_rows = matches!(style.align_content, Alignment::Normal | Alignment::Stretch);
        let row_heights = size_tracks(&row_sizes, &contributions, height, row_gap, stretch_rows);
        let row_offsets = track_offsets(&row_heights, row_gap, height, style.align_content);

        // Placement final : chaque élément dans sa zone, étiré ou aligné.
        let mut fragments = Vec::with_capacity(items.len());
        for item in &mut items {
            let (Some((ca, cb)), Some((ra, rb))) = (item.columns, item.rows) else {
                continue;
            };
            let (area_x, area_w) = span_size(
                &column_widths,
                &column_offsets,
                position(ca, first_column),
                position(cb, first_column),
            );
            let (area_y, area_h) = span_size(
                &row_heights,
                &row_offsets,
                position(ra, first_row),
                position(rb, first_row),
            );
            self.grid_layout_item(item, style, area_w, Some(area_h), width, depth);
            let Some(fragment) = item.fragment.take() else {
                continue;
            };
            let (w, h) = (fragment.border_box.w, fragment.border_box.h);
            let justify = match item.style.justify_self {
                Alignment::Normal => style.justify_items,
                own => own,
            };
            let align = match item.style.align_self {
                Alignment::Normal => style.align_items,
                own => own,
            };
            let offset = |free: f32, alignment: Alignment, start_auto: bool, end_auto: bool| match (
                start_auto, end_auto,
            ) {
                (true, true) => free.max(0.0) / 2.0,
                (true, false) => free.max(0.0),
                (false, true) => 0.0,
                (false, false) => match alignment {
                    Alignment::End => free,
                    Alignment::Center => free / 2.0,
                    _ => 0.0,
                },
            };
            let free_w = area_w - w - item.margin[1] - item.margin[3];
            let free_h = area_h - h - item.margin[0] - item.margin[2];
            let target_x = x
                + area_x
                + item.margin[3]
                + offset(free_w, justify, item.auto_margin[3], item.auto_margin[1]);
            let target_y = y
                + area_y
                + item.margin[0]
                + offset(free_h, align, item.auto_margin[0], item.auto_margin[2]);
            let (dx, dy) = (
                target_x - fragment.border_box.x,
                target_y - fragment.border_box.y,
            );
            let mut fragment = Fragment::Box(fragment);
            fragment.translate(dx, dy);
            for mut waiting in item.out_of_flow.drain(..) {
                waiting.static_x += dx;
                waiting.static_y += dy;
                self.out_of_flow.push(waiting);
            }
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
            fragments.push(fragment);
        }

        // Tailles de pistes utilisées, pour `getComputedStyle`.
        self.grid_log.push(parent);
        self.grids.insert(
            parent,
            crate::GridTracks {
                columns: column_widths,
                rows: row_heights.clone(),
                max_content,
            },
        );
        #[expect(
            clippy::cast_precision_loss,
            reason = "nombre de pistes petit devant 2^24"
        )]
        let rows_total =
            row_heights.iter().sum::<f32>() + row_gap * row_heights.len().saturating_sub(1) as f32;
        (fragments, height.unwrap_or(rows_total).max(0.0))
    }

    /// Éléments de grille de `children` ; les boîtes hors flux sont mises en
    /// attente avec pour position statique l'origine du contenu.
    fn grid_items(
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
            rows: None,
            columns: None,
            row_span: 1,
            column_span: 1,
            margin: [0.0; 4],
            auto_margin: [false; 4],
            fragment: None,
            out_of_flow: Vec::new(),
            parent,
        };
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

    /// Largeurs de contenu minimale et maximale de `item`, en boîte de bordure.
    fn grid_measure(
        &mut self,
        item: &Item,
        width: f32,
        height: Option<f32>,
        depth: usize,
    ) -> (f32, f32) {
        let waiting = self.out_of_flow.len();
        let Some(node) = item.node else {
            let layout =
                self.layout_inline(&item.text, &item.style, item.parent, 0.0, 0.0, width, depth);
            self.out_of_flow.truncate(waiting);
            let widths = layout.layout.calculate_content_widths();
            return (widths.min.ceil(), widths.max.ceil());
        };
        let style = Arc::clone(&item.style);
        let space = Space {
            cb_w: 1.0e6,
            cb_x: 0.0,
            y: 0.0,
            mode: Width::ShrinkToFit,
            cb_h: height,
            fixed_h: None,
        };
        let fragment = self.layout_item(node, &style, space, depth);
        self.out_of_flow.truncate(waiting);
        let max = fragment.border_box.w;
        // Une largeur en pourcentage ne se résout qu'une fois la piste connue.
        if matches!(style.width, Size::Px(_)) || self.intrinsic_size(node, &style).is_some() {
            return (max, max);
        }
        let pad = edges_px(style.padding, width);
        let bw = style.border_widths();
        let extras = pad.left + pad.right + bw.left + bw.right;
        let inner = fragment
            .children
            .iter()
            .map(|c| self.min_content(c))
            .fold(0.0_f32, f32::max);
        let min = crate::bounded_width(&style, inner, extras);
        (min.min(max), max)
    }

    /// Met en page `item` dans une zone de largeur `area_w` et, si elle est
    /// connue, de hauteur `area_h`.
    fn grid_layout_item(
        &mut self,
        item: &mut Item,
        container: &ComputedStyle,
        area_w: f32,
        area_h: Option<f32>,
        width: f32,
        depth: usize,
    ) {
        let _ = width;
        let style = Arc::clone(&item.style);
        let waiting = self.out_of_flow.len();
        let Some(node) = item.node else {
            let layout =
                self.layout_inline(&item.text, &style, item.parent, 0.0, 0.0, area_w, depth);
            item.out_of_flow = self.out_of_flow.split_off(waiting);
            let h = area_h.unwrap_or_else(|| layout.layout.height());
            item.fragment = Some(BoxFragment {
                node: None,
                style,
                border_box: Rect {
                    x: 0.0,
                    y: 0.0,
                    w: area_w,
                    h,
                },
                children: vec![Fragment::Text(Box::new(layout))],
            });
            return;
        };
        let pad = edges_px(style.padding, area_w);
        let bw = style.border_widths();
        let extras_w = pad.left + pad.right + bw.left + bw.right;
        let extras_h = pad.top + pad.bottom + bw.top + bw.bottom;
        let justify = match style.justify_self {
            Alignment::Normal => container.justify_items,
            own => own,
        };
        let align = match style.align_self {
            Alignment::Normal => container.align_items,
            own => own,
        };
        let replaced = self.intrinsic_size(node, &style).is_some();
        let stretch_w = matches!(justify, Alignment::Normal | Alignment::Stretch)
            && style.width == Size::Auto
            && !replaced
            && !item.auto_margin[1]
            && !item.auto_margin[3];
        let stretch_h = matches!(align, Alignment::Normal | Alignment::Stretch)
            && style.height == Size::Auto
            && !replaced
            && !item.auto_margin[0]
            && !item.auto_margin[2];
        let outer = |size: f32, extras: f32| {
            if style.border_box {
                size.max(extras)
            } else {
                size + extras
            }
        };
        let mode = if stretch_w {
            let outer_w = (area_w - item.margin[1] - item.margin[3]).max(0.0);
            // Les bornes de l'élément l'emportent sur l'étirement.
            let max = style
                .max_width
                .resolve(area_w)
                .map_or(f32::INFINITY, |v| outer(v, extras_w));
            let min = style
                .min_width
                .resolve(area_w)
                .map_or(0.0, |v| outer(v, extras_w));
            Width::Fixed((outer_w.min(max).max(min) - extras_w).max(0.0))
        } else if style.width == Size::Auto {
            Width::ShrinkToFit
        } else {
            Width::Stretch
        };
        let fixed_h = match (area_h, stretch_h) {
            (Some(area), true) => {
                let outer_h = (area - item.margin[0] - item.margin[2]).max(0.0);
                let max = resolve_height(style.max_height, area_h)
                    .map_or(f32::INFINITY, |v| outer(v, extras_h));
                let min =
                    resolve_height(style.min_height, area_h).map_or(0.0, |v| outer(v, extras_h));
                Some((outer_h.min(max).max(min) - extras_h).max(0.0))
            }
            _ => None,
        };
        let space = Space {
            cb_w: area_w,
            cb_x: 0.0,
            y: 0.0,
            mode,
            cb_h: area_h,
            fixed_h,
        };
        let fragment = self.layout_item(node, &style, space, depth);
        item.out_of_flow = self.out_of_flow.split_off(waiting);
        item.fragment = Some(fragment);
    }
}
