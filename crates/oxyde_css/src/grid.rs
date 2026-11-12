//! Valeurs de la mise en page en grille (CSS Grid Layout Level 2) : listes de
//! pistes (`grid-template-columns`…), lignes de placement (`grid-row-start`…)
//! et zones nommées (`grid-template-areas`).
//!
//! Les types sont génériques sur la longueur : [`crate::values::Length`] pour
//! la valeur telle qu'écrite, [`crate::style::Size`] une fois calculée.
//!
//! LIMITES CONNUES : `subgrid` et `masonry` ne sont pas reconnus ; les noms
//! de lignes placés dans un `repeat()` sont ignorés.

use crate::syntax::{Token, matching_close, split_top_level, trim};
use crate::values::{Length, parse_length};

/// Borne d'une piste.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Breadth<L> {
    /// Longueur ou pourcentage.
    Length(L),
    /// Fraction de l'espace libre (`1fr`).
    Fr(f32),
    /// `auto`
    Auto,
    /// `min-content`
    MinContent,
    /// `max-content`
    MaxContent,
    /// `fit-content(longueur)` : le contenu, plafonné par la longueur.
    FitContent(L),
}

/// Taille d'une piste : `minmax(min, max)` ; une taille simple a ses deux
/// bornes égales.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrackSize<L> {
    /// Borne inférieure.
    pub min: Breadth<L>,
    /// Borne supérieure.
    pub max: Breadth<L>,
}

impl<L: Copy> TrackSize<L> {
    /// Piste `auto`.
    #[must_use]
    pub fn auto() -> Self {
        Self {
            min: Breadth::Auto,
            max: Breadth::Auto,
        }
    }

    fn single(breadth: Breadth<L>) -> Self {
        match breadth {
            // `1fr` vaut `minmax(auto, 1fr)`, `fit-content(x)` vaut
            // `minmax(auto, fit-content(x))`.
            Breadth::Fr(_) | Breadth::FitContent(_) => Self {
                min: Breadth::Auto,
                max: breadth,
            },
            other => Self {
                min: other,
                max: other,
            },
        }
    }

    fn map<M>(self, f: &impl Fn(L) -> M) -> TrackSize<M> {
        let breadth = |b: Breadth<L>| match b {
            Breadth::Length(l) => Breadth::Length(f(l)),
            Breadth::FitContent(l) => Breadth::FitContent(f(l)),
            Breadth::Fr(v) => Breadth::Fr(v),
            Breadth::Auto => Breadth::Auto,
            Breadth::MinContent => Breadth::MinContent,
            Breadth::MaxContent => Breadth::MaxContent,
        };
        TrackSize {
            min: breadth(self.min),
            max: breadth(self.max),
        }
    }
}

/// Nombre de répétitions d'un `repeat()`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Repeat {
    /// Nombre fixe.
    Count(u32),
    /// `auto-fill` : autant que le conteneur en accepte.
    AutoFill,
    /// `auto-fit` : comme `auto-fill`, les pistes vides étant repliées.
    AutoFit,
}

/// Élément d'une liste de pistes.
#[derive(Clone, Debug, PartialEq)]
pub enum TrackEntry<L> {
    /// Une piste.
    Track(TrackSize<L>),
    /// Noms donnés à la ligne qui se trouve à cet endroit (`[début]`).
    Names(Vec<String>),
    /// `repeat(n, pistes…)`
    Repeat(Repeat, Vec<TrackSize<L>>),
}

/// Liste de pistes (`grid-template-columns`, `grid-template-rows`). Vide :
/// `none`.
#[derive(Clone, Debug, PartialEq)]
pub struct TrackList<L> {
    /// Éléments, dans l'ordre.
    pub entries: Vec<TrackEntry<L>>,
}

impl<L> Default for TrackList<L> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl<L: Copy> TrackList<L> {
    /// La même liste, chaque longueur transformée par `f`.
    pub fn map<M>(&self, f: impl Fn(L) -> M) -> TrackList<M> {
        TrackList {
            entries: self
                .entries
                .iter()
                .map(|entry| match entry {
                    TrackEntry::Track(track) => TrackEntry::Track(track.map(&f)),
                    TrackEntry::Names(names) => TrackEntry::Names(names.clone()),
                    TrackEntry::Repeat(repeat, tracks) => {
                        TrackEntry::Repeat(*repeat, tracks.iter().map(|t| t.map(&f)).collect())
                    }
                })
                .collect(),
        }
    }
}

/// Ligne de placement d'un élément (`grid-row-start`…).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum GridLine {
    /// `auto`
    #[default]
    Auto,
    /// Numéro de ligne (négatif : compté depuis la fin), éventuellement
    /// parmi les lignes d'un nom donné.
    Line(i32, Option<String>),
    /// `span n` : l'élément couvre `n` pistes.
    Span(u32, Option<String>),
    /// Nom de ligne ou de zone.
    Named(String),
}

/// Zones nommées (`grid-template-areas`) : une rangée par chaîne, une cellule
/// par mot ; `None` pour une cellule vide (`.`).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct GridAreas {
    /// Cellules, rangée par rangée.
    pub rows: Vec<Vec<Option<String>>>,
}

impl GridAreas {
    /// Rectangle de la zone `name` : (rangée de début, rangée de fin,
    /// colonne de début, colonne de fin), en numéros de lignes à partir de 1.
    #[must_use]
    pub fn area(&self, name: &str) -> Option<(i32, i32, i32, i32)> {
        let (mut top, mut bottom, mut left, mut right) = (usize::MAX, 0usize, usize::MAX, 0usize);
        for (r, row) in self.rows.iter().enumerate() {
            for (c, cell) in row.iter().enumerate() {
                if cell.as_deref() == Some(name) {
                    top = top.min(r);
                    bottom = bottom.max(r + 1);
                    left = left.min(c);
                    right = right.max(c + 1);
                }
            }
        }
        let line = |index: usize| i32::try_from(index + 1).ok();
        (top != usize::MAX).then_some(())?;
        Some((line(top)?, line(bottom)?, line(left)?, line(right)?))
    }
}

fn breadth(token: &Token) -> Option<Breadth<Length>> {
    match token {
        Token::Dimension(value, unit) if unit == "fr" && *value >= 0.0 => Some(Breadth::Fr(*value)),
        Token::Ident(word) => match word.to_ascii_lowercase().as_str() {
            "auto" => Some(Breadth::Auto),
            "min-content" => Some(Breadth::MinContent),
            "max-content" => Some(Breadth::MaxContent),
            _ => None,
        },
        other => parse_length(other).map(Breadth::Length),
    }
}

/// Analyse une taille de piste commençant à `tokens[at]` ; renvoie la taille
/// et le rang du jeton qui la suit.
fn track_size(tokens: &[Token], at: usize) -> Option<(TrackSize<Length>, usize)> {
    match tokens.get(at)? {
        Token::Function(name) if name == "minmax" => {
            let close = matching_close(tokens, at);
            let inner = tokens.get(at + 1..close)?;
            let parts = split_top_level(inner, &Token::Comma);
            let [min, max] = parts.as_slice() else {
                return None;
            };
            let ([min], [max]) = (trim(min), trim(max)) else {
                return None;
            };
            let (min, max) = (breadth(min)?, breadth(max)?);
            // Une fraction n'est pas un minimum.
            if matches!(min, Breadth::Fr(_)) {
                return None;
            }
            Some((TrackSize { min, max }, close + 1))
        }
        Token::Function(name) if name == "fit-content" => {
            let close = matching_close(tokens, at);
            let [limit] = trim(tokens.get(at + 1..close)?) else {
                return None;
            };
            let limit = parse_length(limit)?;
            Some((TrackSize::single(Breadth::FitContent(limit)), close + 1))
        }
        token => Some((TrackSize::single(breadth(token)?), at + 1)),
    }
}

/// Noms de ligne entre crochets commençant à `tokens[at]`.
fn line_names(tokens: &[Token], at: usize) -> Option<(Vec<String>, usize)> {
    let close = matching_close(tokens, at);
    let names = tokens
        .get(at + 1..close)?
        .iter()
        .filter(|t| **t != Token::Whitespace)
        .map(|t| match t {
            Token::Ident(name) => Some(name.clone()),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    Some((names, close + 1))
}

/// Analyse une liste de pistes ; `none` donne une liste vide.
#[must_use]
pub fn parse_track_list(tokens: &[Token]) -> Option<TrackList<Length>> {
    let tokens = trim(tokens);
    if let [Token::Ident(word)] = tokens
        && word.eq_ignore_ascii_case("none")
    {
        return Some(TrackList::default());
    }
    let mut entries = Vec::new();
    let mut auto_repeats = 0;
    let mut at = 0;
    while at < tokens.len() {
        match &tokens[at] {
            Token::Whitespace => at += 1,
            Token::OpenBracket => {
                let (names, next) = line_names(tokens, at)?;
                entries.push(TrackEntry::Names(names));
                at = next;
            }
            Token::Function(name) if name == "repeat" => {
                let close = matching_close(tokens, at);
                let inner = tokens.get(at + 1..close)?;
                let comma = inner.iter().position(|t| *t == Token::Comma)?;
                let repeat = match trim(inner.get(..comma)?) {
                    [Token::Number(n)] if *n >= 1.0 && n.fract() == 0.0 =>
                    {
                        #[expect(
                            clippy::cast_possible_truncation,
                            clippy::cast_sign_loss,
                            reason = "entier positif vérifié"
                        )]
                        Repeat::Count((*n as u32).min(10_000))
                    }
                    [Token::Ident(word)] if word.eq_ignore_ascii_case("auto-fill") => {
                        Repeat::AutoFill
                    }
                    [Token::Ident(word)] if word.eq_ignore_ascii_case("auto-fit") => {
                        Repeat::AutoFit
                    }
                    _ => return None,
                };
                if repeat != Repeat::Count(0) && !matches!(repeat, Repeat::Count(_)) {
                    auto_repeats += 1;
                }
                let body = inner.get(comma + 1..)?;
                let mut tracks = Vec::new();
                let mut inside = 0;
                while inside < body.len() {
                    match &body[inside] {
                        Token::Whitespace => inside += 1,
                        Token::OpenBracket => inside = matching_close(body, inside) + 1,
                        _ => {
                            let (track, next) = track_size(body, inside)?;
                            tracks.push(track);
                            inside = next;
                        }
                    }
                }
                if tracks.is_empty() {
                    return None;
                }
                entries.push(TrackEntry::Repeat(repeat, tracks));
                at = close + 1;
            }
            _ => {
                let (track, next) = track_size(tokens, at)?;
                entries.push(TrackEntry::Track(track));
                at = next;
            }
        }
    }
    // Une seule répétition automatique par liste, et au moins une piste.
    let has_track = entries.iter().any(|e| !matches!(e, TrackEntry::Names(_)));
    (auto_repeats <= 1 && has_track).then_some(TrackList { entries })
}

/// Analyse une suite de tailles de pistes (`grid-auto-columns`).
#[must_use]
pub fn parse_track_sizes(tokens: &[Token]) -> Option<Vec<TrackSize<Length>>> {
    let tokens = trim(tokens);
    let mut tracks = Vec::new();
    let mut at = 0;
    while at < tokens.len() {
        if tokens[at] == Token::Whitespace {
            at += 1;
            continue;
        }
        let (track, next) = track_size(tokens, at)?;
        tracks.push(track);
        at = next;
    }
    (!tracks.is_empty()).then_some(tracks)
}

/// Analyse une ligne de placement : `auto`, `3`, `-1`, `span 2`, `nom`,
/// `nom 2`, `span nom`.
#[must_use]
pub fn parse_grid_line(tokens: &[Token]) -> Option<GridLine> {
    let parts: Vec<&Token> = tokens.iter().filter(|t| **t != Token::Whitespace).collect();
    let (mut span, mut number, mut name) = (false, None, None);
    for part in &parts {
        match part {
            Token::Ident(word) if word.eq_ignore_ascii_case("auto") && parts.len() == 1 => {
                return Some(GridLine::Auto);
            }
            Token::Ident(word) if word.eq_ignore_ascii_case("span") && !span => span = true,
            Token::Ident(word) if name.is_none() && !word.eq_ignore_ascii_case("auto") => {
                name = Some(word.clone());
            }
            Token::Number(n) if number.is_none() && n.fract() == 0.0 && *n != 0.0 => {
                #[expect(clippy::cast_possible_truncation, reason = "entier vérifié")]
                {
                    number = Some((*n as i32).clamp(-10_000, 10_000));
                }
            }
            _ => return None,
        }
    }
    match (span, number, name) {
        (true, Some(n), name) if n > 0 => Some(GridLine::Span(n.unsigned_abs(), name)),
        (true, None, name @ Some(_)) => Some(GridLine::Span(1, name)),
        (true, _, _) => None,
        (false, Some(n), name) => Some(GridLine::Line(n, name)),
        (false, None, Some(name)) => Some(GridLine::Named(name)),
        (false, None, None) => None,
    }
}

/// Analyse `grid-template-areas` : une chaîne par rangée ; `none` donne des
/// zones vides. Les rangées doivent avoir le même nombre de cellules et
/// chaque zone doit former un rectangle.
#[must_use]
pub fn parse_areas(tokens: &[Token]) -> Option<GridAreas> {
    let tokens = trim(tokens);
    if let [Token::Ident(word)] = tokens
        && word.eq_ignore_ascii_case("none")
    {
        return Some(GridAreas::default());
    }
    let mut rows: Vec<Vec<Option<String>>> = Vec::new();
    for token in tokens {
        match token {
            Token::Whitespace => {}
            Token::Str(row) => {
                let cells: Vec<Option<String>> = row
                    .split_ascii_whitespace()
                    .map(|cell| {
                        if cell.chars().all(|c| c == '.') {
                            None
                        } else {
                            Some(cell.to_owned())
                        }
                    })
                    .collect();
                if cells.is_empty() {
                    return None;
                }
                rows.push(cells);
            }
            _ => return None,
        }
    }
    let width = rows.first()?.len();
    if rows.iter().any(|row| row.len() != width) {
        return None;
    }
    let areas = GridAreas { rows };
    // Chaque zone doit être un rectangle plein.
    let mut names: Vec<&str> = areas
        .rows
        .iter()
        .flatten()
        .filter_map(|cell| cell.as_deref())
        .collect();
    names.sort_unstable();
    names.dedup();
    for name in names {
        let (top, bottom, left, right) = areas.area(name)?;
        let cells = areas
            .rows
            .iter()
            .flatten()
            .filter(|cell| cell.as_deref() == Some(name))
            .count();
        let expected = usize::try_from((bottom - top) * (right - left)).ok()?;
        if cells != expected {
            return None;
        }
    }
    Some(areas)
}
