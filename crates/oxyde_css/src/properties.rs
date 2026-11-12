//! Propriétés CSS prises en charge : analyse des déclarations et
//! développement des propriétés raccourcies en propriétés longues.
//!
//! Une valeur invalide fait écarter la déclaration entière, comme l'exige le
//! standard (la déclaration précédente de la cascade reste alors en vigueur).

use crate::syntax::{Token, split_top_level, trim};
use crate::values::{Length, ParsedColor, parse_color, parse_length};

/// Côté d'une boîte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// Haut.
    Top,
    /// Droite.
    Right,
    /// Bas.
    Bottom,
    /// Gauche.
    Left,
}

const SIDES: [Side; 4] = [Side::Top, Side::Right, Side::Bottom, Side::Left];

/// Propriété longue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(
    missing_docs,
    reason = "les variantes portent le nom de la propriété CSS"
)]
pub enum Longhand {
    Display,
    Color,
    BackgroundColor,
    FontSize,
    FontWeight,
    FontStyle,
    FontFamily,
    LineHeight,
    TextAlign,
    WhiteSpace,
    TextDecorationLine,
    Margin(Side),
    Padding(Side),
    BorderWidth(Side),
    BorderColor(Side),
    BorderStyle(Side),
    BorderRadius,
    Width,
    Height,
    MinWidth,
    MaxWidth,
    MinHeight,
    MaxHeight,
    BoxSizing,
    ListStyleType,
    Position,
    Inset(Side),
    Float,
    Clear,
    ZIndex,
    OverflowX,
    OverflowY,
    Visibility,
    Opacity,
    FlexDirection,
    FlexWrap,
    FlexGrow,
    FlexShrink,
    FlexBasis,
    JustifyContent,
    AlignItems,
    AlignSelf,
    AlignContent,
    Order,
    RowGap,
    ColumnGap,
    GridTemplateColumns,
    GridTemplateRows,
    GridAutoColumns,
    GridAutoRows,
    GridAutoFlow,
    GridRowStart,
    GridRowEnd,
    GridColumnStart,
    GridColumnEnd,
    GridTemplateAreas,
    JustifyItems,
    JustifySelf,
    BorderSpacingH,
    BorderSpacingV,
    BorderCollapse,
    TableLayout,
    CaptionSide,
    VerticalAlign,
}

/// Valeur d'une déclaration, déjà validée pour sa propriété.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// Mot-clé (minuscules), valide pour la propriété.
    Ident(String),
    /// Longueur ou pourcentage.
    Length(Length),
    /// Nombre sans unité.
    Number(f32),
    /// Couleur absolue.
    Color(crate::values::Color),
    /// `currentcolor`
    CurrentColor,
    /// Liste de familles de polices.
    FontFamily(Vec<String>),
    /// Liste de pistes de grille.
    Tracks(crate::grid::TrackList<Length>),
    /// Tailles de pistes implicites.
    TrackSizes(Vec<crate::grid::TrackSize<Length>>),
    /// Ligne de placement dans une grille.
    Line(crate::grid::GridLine),
    /// Zones nommées d'une grille.
    Areas(crate::grid::GridAreas),
    /// `inherit`
    Inherit,
    /// `initial`
    Initial,
    /// `unset` / `revert`
    Unset,
}

/// Déclaration d'une propriété longue.
#[derive(Clone, Debug, PartialEq)]
pub struct Declaration {
    /// Propriété.
    pub property: Longhand,
    /// Valeur.
    pub value: Value,
    /// `!important`
    pub important: bool,
}

/// Analyse le contenu d'un bloc de déclarations (`a: b; c: d`).
#[must_use]
pub fn parse_declarations(tokens: &[Token]) -> Vec<Declaration> {
    let mut out = Vec::new();
    for part in split_top_level(tokens, &Token::Semicolon) {
        let part = trim(part);
        let [Token::Ident(name), rest @ ..] = part else {
            continue;
        };
        let rest = trim(rest);
        let [Token::Colon, value @ ..] = rest else {
            continue;
        };
        let mut value = trim(value);
        let mut important = false;
        if let [head @ .., Token::Delim('!'), Token::Ident(word)] = value
            && word.eq_ignore_ascii_case("important")
        {
            important = true;
            value = trim(head);
        } else if let [
            head @ ..,
            Token::Delim('!'),
            Token::Whitespace,
            Token::Ident(word),
        ] = value
            && word.eq_ignore_ascii_case("important")
        {
            important = true;
            value = trim(head);
        }
        if value.is_empty() || value.iter().any(|t| matches!(t, Token::OpenBrace)) {
            continue;
        }
        let name = name.to_ascii_lowercase();
        // Les `calc()` sont évalués avant l'analyse de la propriété.
        let Some(folded) = crate::values::fold_calc(value) else {
            continue;
        };
        if let Some(pairs) = parse_property(&name, &folded) {
            out.extend(pairs.into_iter().map(|(property, value)| Declaration {
                property,
                value,
                important,
            }));
        }
    }
    out
}

type Pairs = Vec<(Longhand, Value)>;

fn css_wide(tokens: &[Token]) -> Option<Value> {
    let [Token::Ident(word)] = tokens else {
        return None;
    };
    match word.to_ascii_lowercase().as_str() {
        "inherit" => Some(Value::Inherit),
        "initial" => Some(Value::Initial),
        "unset" | "revert" | "revert-layer" => Some(Value::Unset),
        _ => None,
    }
}

fn ident_of(token: &Token) -> Option<String> {
    match token {
        Token::Ident(s) => Some(s.to_ascii_lowercase()),
        _ => None,
    }
}

fn keyword(tokens: &[Token], allowed: &[&str]) -> Option<Value> {
    let [token] = tokens else { return None };
    let word = ident_of(token)?;
    allowed
        .contains(&word.as_str())
        .then_some(Value::Ident(word))
}

fn non_negative(length: Length) -> Option<Length> {
    let v = match length {
        Length::Px(v)
        | Length::Em(v)
        | Length::Rem(v)
        | Length::Percent(v)
        | Length::Vw(v)
        | Length::Vh(v) => v,
        // Le signe d'une somme d'unités mêlées dépend du contexte.
        Length::Calc(_) => return Some(length),
    };
    (v >= 0.0).then_some(length)
}

fn length_value(token: &Token, allow_negative: bool) -> Option<Value> {
    let length = parse_length(token)?;
    let length = if allow_negative {
        length
    } else {
        non_negative(length)?
    };
    Some(Value::Length(length))
}

fn length_or_keywords(token: &Token, keywords: &[&str], allow_negative: bool) -> Option<Value> {
    if let Some(word) = ident_of(token) {
        return keywords
            .contains(&word.as_str())
            .then_some(Value::Ident(word));
    }
    length_value(token, allow_negative)
}

fn color_value(tokens: &[Token]) -> Option<Value> {
    let (color, used) = parse_color(tokens)?;
    if used != tokens.len() {
        return None;
    }
    Some(match color {
        ParsedColor::Color(c) => Value::Color(c),
        ParsedColor::CurrentColor => Value::CurrentColor,
    })
}

const BORDER_STYLES: &[&str] = &[
    "none", "hidden", "dotted", "dashed", "solid", "double", "groove", "ridge", "inset", "outset",
];
const BORDER_WIDTHS: &[&str] = &["thin", "medium", "thick"];
const FONT_SIZES: &[&str] = &[
    "xx-small",
    "x-small",
    "small",
    "medium",
    "large",
    "x-large",
    "xx-large",
    "xxx-large",
    "smaller",
    "larger",
];
const DISPLAYS: &[&str] = &[
    "none",
    "block",
    "inline",
    "inline-block",
    "list-item",
    "flex",
    "inline-flex",
    "grid",
    "inline-grid",
    "table",
    "inline-table",
    "table-row",
    "table-cell",
    "table-row-group",
    "table-header-group",
    "table-footer-group",
    "table-column",
    "table-column-group",
    "table-caption",
    "flow-root",
    "contents",
];

/// Composants hors blancs.
fn components(tokens: &[Token]) -> Vec<&Token> {
    tokens.iter().filter(|t| **t != Token::Whitespace).collect()
}

/// Développe 1 à 4 valeurs en haut / droite / bas / gauche.
fn four_sides(values: &[Value]) -> Option<[Value; 4]> {
    Some(match values {
        [a] => [a.clone(), a.clone(), a.clone(), a.clone()],
        [a, b] => [a.clone(), b.clone(), a.clone(), b.clone()],
        [a, b, c] => [a.clone(), b.clone(), c.clone(), b.clone()],
        [a, b, c, d] => [a.clone(), b.clone(), c.clone(), d.clone()],
        _ => return None,
    })
}

fn sides(make: fn(Side) -> Longhand, values: [Value; 4]) -> Pairs {
    SIDES
        .iter()
        .copied()
        .zip(values)
        .map(|(side, v)| (make(side), v))
        .collect()
}

fn all_sides(make: fn(Side) -> Longhand, value: &Value) -> Pairs {
    SIDES
        .iter()
        .map(|side| (make(*side), value.clone()))
        .collect()
}

fn side_of(name: &str) -> Option<Side> {
    Some(match name {
        "top" | "block-start" => Side::Top,
        "right" | "inline-end" => Side::Right,
        "bottom" | "block-end" => Side::Bottom,
        "left" | "inline-start" => Side::Left,
        _ => return None,
    })
}

const OVERFLOWS: &[&str] = &["visible", "hidden", "clip", "scroll", "auto"];
const FLEX_DIRECTIONS: &[&str] = &["row", "row-reverse", "column", "column-reverse"];
const FLEX_WRAPS: &[&str] = &["nowrap", "wrap", "wrap-reverse"];
const CONTENT_ALIGNMENTS: &[&str] = &[
    "normal",
    "stretch",
    "flex-start",
    "start",
    "flex-end",
    "end",
    "center",
    "space-between",
    "space-around",
    "space-evenly",
    "left",
    "right",
];
const ITEM_ALIGNMENTS: &[&str] = &[
    "auto",
    "normal",
    "stretch",
    "flex-start",
    "start",
    "self-start",
    "flex-end",
    "end",
    "self-end",
    "center",
    "baseline",
];

/// Alignements d'un élément dans sa zone (`justify-self`, `justify-items`).
const SELF_ALIGNMENTS: &[&str] = &[
    "auto",
    "normal",
    "stretch",
    "flex-start",
    "start",
    "self-start",
    "flex-end",
    "end",
    "self-end",
    "center",
    "baseline",
    "left",
    "right",
    "legacy",
];

fn grid_line(tokens: &[Token]) -> Option<Value> {
    crate::grid::parse_grid_line(tokens).map(Value::Line)
}

/// Ligne de fin sous-entendue par une ligne de début : le même nom si le
/// début est un nom, `auto` sinon.
fn implied_line(start: &Value) -> Value {
    match start {
        Value::Line(crate::grid::GridLine::Named(name)) => {
            Value::Line(crate::grid::GridLine::Named(name.clone()))
        }
        _ => Value::Line(crate::grid::GridLine::Auto),
    }
}

/// Entier (`z-index`, `order`).
fn integer(token: &Token) -> Option<Value> {
    match token {
        Token::Number(v) if v.fract() == 0.0 => Some(Value::Number(*v)),
        _ => None,
    }
}

fn integer_or_auto(token: &Token) -> Option<Value> {
    match ident_of(token).as_deref() {
        Some("auto") => Some(Value::Ident("auto".to_owned())),
        Some(_) => None,
        None => integer(token),
    }
}

fn non_negative_number(token: &Token) -> Option<Value> {
    match token {
        Token::Number(v) if *v >= 0.0 => Some(Value::Number(*v)),
        _ => None,
    }
}

/// Valeur de `row-gap` ou `column-gap` : `normal` ou une longueur positive.
fn gap_value(token: &Token) -> Option<Value> {
    length_or_keywords(token, &["normal"], false)
}

fn font_weight(token: &Token) -> Option<Value> {
    match token {
        Token::Number(v) if (1.0..=1000.0).contains(v) => Some(Value::Number(*v)),
        Token::Ident(s) => match s.to_ascii_lowercase().as_str() {
            "normal" => Some(Value::Number(400.0)),
            "bold" => Some(Value::Number(700.0)),
            w @ ("bolder" | "lighter") => Some(Value::Ident(w.to_owned())),
            _ => None,
        },
        _ => None,
    }
}

fn font_family(tokens: &[Token]) -> Option<Value> {
    let mut families = Vec::new();
    for part in split_top_level(tokens, &Token::Comma) {
        let part = trim(part);
        match part {
            [Token::Str(name)] => families.push(name.clone()),
            [] => return None,
            idents => {
                let words: Option<Vec<&str>> = idents
                    .iter()
                    .filter(|t| **t != Token::Whitespace)
                    .map(|t| match t {
                        Token::Ident(s) => Some(s.as_str()),
                        _ => None,
                    })
                    .collect();
                families.push(words?.join(" "));
            }
        }
    }
    Some(Value::FontFamily(families))
}

fn line_height(token: &Token) -> Option<Value> {
    match token {
        Token::Number(v) if *v >= 0.0 => Some(Value::Number(*v)),
        Token::Ident(s) if s.eq_ignore_ascii_case("normal") => Some(Value::Ident("normal".into())),
        other => length_value(other, false),
    }
}

/// `border`, `border-top`… : largeur, style et couleur dans un ordre quelconque.
fn border_shorthand(tokens: &[Token]) -> Option<(Value, Value, Value)> {
    let mut width = None;
    let mut style = None;
    let mut color = None;
    let mut i = 0;
    while i < tokens.len() {
        let t = &tokens[i];
        if *t == Token::Whitespace {
            i += 1;
            continue;
        }
        if width.is_none()
            && let Some(v) = length_or_keywords(t, BORDER_WIDTHS, false)
        {
            width = Some(v);
            i += 1;
        } else if style.is_none()
            && let Some(word) = ident_of(t).filter(|w| BORDER_STYLES.contains(&w.as_str()))
        {
            style = Some(Value::Ident(word));
            i += 1;
        } else if color.is_none()
            && let Some((c, used)) = parse_color(&tokens[i..])
        {
            color = Some(match c {
                ParsedColor::Color(c) => Value::Color(c),
                ParsedColor::CurrentColor => Value::CurrentColor,
            });
            i += used;
        } else {
            return None;
        }
    }
    Some((
        width.unwrap_or_else(|| Value::Ident("medium".into())),
        style.unwrap_or_else(|| Value::Ident("none".into())),
        color.unwrap_or(Value::CurrentColor),
    ))
}

fn border_pairs(side_list: &[Side], (width, style, color): (Value, Value, Value)) -> Pairs {
    let mut out = Pairs::new();
    for side in side_list {
        out.push((Longhand::BorderWidth(*side), width.clone()));
        out.push((Longhand::BorderStyle(*side), style.clone()));
        out.push((Longhand::BorderColor(*side), color.clone()));
    }
    out
}

/// `font: [style] [weight] taille[/interligne] famille`.
fn font_shorthand(tokens: &[Token]) -> Option<Pairs> {
    let mut out: Pairs = vec![
        (Longhand::FontStyle, Value::Ident("normal".into())),
        (Longhand::FontWeight, Value::Number(400.0)),
        (Longhand::LineHeight, Value::Ident("normal".into())),
    ];
    let mut i = 0;
    // Préambule : style, graisse, variantes ignorées.
    loop {
        let t = tokens.get(i)?;
        if *t == Token::Whitespace {
            i += 1;
            continue;
        }
        if let Some(word) = ident_of(t) {
            match word.as_str() {
                "italic" | "oblique" => {
                    out.push((Longhand::FontStyle, Value::Ident("italic".into())))
                }
                "normal" | "small-caps" => {}
                "bold" | "bolder" | "lighter" => out.push((Longhand::FontWeight, font_weight(t)?)),
                _ => break,
            }
            i += 1;
        } else if let Token::Number(_) = t {
            out.push((Longhand::FontWeight, font_weight(t)?));
            i += 1;
        } else {
            break;
        }
    }
    let size = length_or_keywords(tokens.get(i)?, FONT_SIZES, false)?;
    out.push((Longhand::FontSize, size));
    i += 1;
    if tokens.get(i) == Some(&Token::Delim('/')) {
        out.push((Longhand::LineHeight, line_height(tokens.get(i + 1)?)?));
        i += 2;
    }
    out.push((Longhand::FontFamily, font_family(trim(tokens.get(i..)?))?));
    Some(out)
}

fn parse_property(name: &str, tokens: &[Token]) -> Option<Pairs> {
    let wide = css_wide(tokens);
    let single = |property: Longhand, parsed: Option<Value>| -> Option<Pairs> {
        Some(vec![(property, wide.clone().or(parsed)?)])
    };
    let one = || -> Option<&Token> {
        let [t] = tokens else { return None };
        Some(t)
    };
    let size = |keywords: &[&str]| one().and_then(|t| length_or_keywords(t, keywords, false));

    match name {
        "display" => single(Longhand::Display, keyword(tokens, DISPLAYS)),
        "color" => single(Longhand::Color, color_value(tokens)),
        "background-color" => single(Longhand::BackgroundColor, color_value(tokens)),
        "background" => {
            // Seule la couleur est retenue ; une image ou un dégradé donne `transparent`.
            if let Some(w) = wide {
                return Some(vec![(Longhand::BackgroundColor, w)]);
            }
            let mut found = Value::Color(crate::values::Color::TRANSPARENT);
            let mut i = 0;
            while i < tokens.len() {
                if let Some((c, used)) = parse_color(&tokens[i..]) {
                    found = match c {
                        ParsedColor::Color(c) => Value::Color(c),
                        ParsedColor::CurrentColor => Value::CurrentColor,
                    };
                    i += used;
                } else if matches!(tokens[i], Token::Function(_)) {
                    i = crate::syntax::matching_close(tokens, i) + 1;
                } else {
                    i += 1;
                }
            }
            Some(vec![(Longhand::BackgroundColor, found)])
        }
        "font-size" => single(Longhand::FontSize, size(FONT_SIZES)),
        "font-weight" => single(Longhand::FontWeight, one().and_then(font_weight)),
        "font-style" => single(
            Longhand::FontStyle,
            keyword(tokens, &["normal", "italic", "oblique"]),
        ),
        "font-family" => single(Longhand::FontFamily, font_family(tokens)),
        "font" => match wide {
            Some(w) => Some(
                [
                    Longhand::FontStyle,
                    Longhand::FontWeight,
                    Longhand::FontSize,
                    Longhand::LineHeight,
                    Longhand::FontFamily,
                ]
                .into_iter()
                .map(|p| (p, w.clone()))
                .collect(),
            ),
            None => font_shorthand(tokens),
        },
        "line-height" => single(Longhand::LineHeight, one().and_then(line_height)),
        "text-align" => single(
            Longhand::TextAlign,
            keyword(
                tokens,
                &[
                    "left",
                    "right",
                    "center",
                    "justify",
                    "start",
                    "end",
                    "-webkit-center",
                ],
            ),
        ),
        "white-space" => single(
            Longhand::WhiteSpace,
            keyword(
                tokens,
                &[
                    "normal",
                    "nowrap",
                    "pre",
                    "pre-wrap",
                    "pre-line",
                    "break-spaces",
                ],
            ),
        ),
        "text-decoration" | "text-decoration-line" => {
            if let Some(w) = wide {
                return Some(vec![(Longhand::TextDecorationLine, w)]);
            }
            // Seule la ligne est retenue (couleur, style et épaisseur ignorés).
            let mut lines = Vec::new();
            for t in components(tokens) {
                if let Some(word) = ident_of(t)
                    && matches!(
                        word.as_str(),
                        "none" | "underline" | "line-through" | "overline"
                    )
                {
                    lines.push(word);
                }
            }
            if lines.is_empty() {
                return None;
            }
            Some(vec![(
                Longhand::TextDecorationLine,
                Value::Ident(lines.join(" ")),
            )])
        }
        "margin" | "padding" => {
            let (make, keywords, negative): (fn(Side) -> Longhand, &[&str], bool) =
                if name == "margin" {
                    (Longhand::Margin, &["auto"], true)
                } else {
                    (Longhand::Padding, &[], false)
                };
            if let Some(w) = wide {
                return Some(all_sides(make, &w));
            }
            let values: Option<Vec<Value>> = components(tokens)
                .into_iter()
                .map(|t| length_or_keywords(t, keywords, negative))
                .collect();
            Some(sides(make, four_sides(&values?)?))
        }
        "border-width" | "border-style" | "border-color" => {
            let make: fn(Side) -> Longhand = match name {
                "border-width" => Longhand::BorderWidth,
                "border-style" => Longhand::BorderStyle,
                _ => Longhand::BorderColor,
            };
            if let Some(w) = wide {
                return Some(all_sides(make, &w));
            }
            let mut values = Vec::new();
            let mut i = 0;
            while i < tokens.len() {
                if tokens[i] == Token::Whitespace {
                    i += 1;
                    continue;
                }
                match name {
                    "border-width" => {
                        values.push(length_or_keywords(&tokens[i], BORDER_WIDTHS, false)?);
                        i += 1;
                    }
                    "border-style" => {
                        values.push(keyword(&tokens[i..=i], BORDER_STYLES)?);
                        i += 1;
                    }
                    _ => {
                        let (c, used) = parse_color(&tokens[i..])?;
                        values.push(match c {
                            ParsedColor::Color(c) => Value::Color(c),
                            ParsedColor::CurrentColor => Value::CurrentColor,
                        });
                        i += used;
                    }
                }
            }
            Some(sides(make, four_sides(&values)?))
        }
        "border" | "outline-ignored" => match wide {
            Some(w) => Some(border_pairs(&SIDES, (w.clone(), w.clone(), w))),
            None => Some(border_pairs(&SIDES, border_shorthand(tokens)?)),
        },
        "border-radius" => {
            // Rayon uniforme : la première valeur fait foi.
            let first = components(tokens).into_iter().next()?;
            single(Longhand::BorderRadius, length_value(first, false))
        }
        "width" => single(Longhand::Width, size(&["auto"])),
        "height" => single(Longhand::Height, size(&["auto"])),
        "min-width" => single(Longhand::MinWidth, size(&["auto"])),
        "min-height" => single(Longhand::MinHeight, size(&["auto"])),
        "max-width" => single(Longhand::MaxWidth, size(&["none"])),
        "max-height" => single(Longhand::MaxHeight, size(&["none"])),
        "box-sizing" => single(
            Longhand::BoxSizing,
            keyword(tokens, &["content-box", "border-box"]),
        ),
        "position" => single(
            Longhand::Position,
            keyword(
                tokens,
                &["static", "relative", "absolute", "fixed", "sticky"],
            ),
        ),
        "top" | "right" | "bottom" | "left" => single(
            Longhand::Inset(side_of(name)?),
            one().and_then(|t| length_or_keywords(t, &["auto"], true)),
        ),
        "inset" => {
            if let Some(w) = wide {
                return Some(all_sides(Longhand::Inset, &w));
            }
            let values: Option<Vec<Value>> = components(tokens)
                .into_iter()
                .map(|t| length_or_keywords(t, &["auto"], true))
                .collect();
            Some(sides(Longhand::Inset, four_sides(&values?)?))
        }
        "float" => single(
            Longhand::Float,
            keyword(
                tokens,
                &["none", "left", "right", "inline-start", "inline-end"],
            ),
        ),
        "clear" => single(
            Longhand::Clear,
            keyword(
                tokens,
                &[
                    "none",
                    "left",
                    "right",
                    "both",
                    "inline-start",
                    "inline-end",
                ],
            ),
        ),
        "z-index" => single(Longhand::ZIndex, one().and_then(integer_or_auto)),
        "order" => single(Longhand::Order, one().and_then(integer)),
        "overflow" => {
            if let Some(w) = wide {
                return Some(vec![
                    (Longhand::OverflowX, w.clone()),
                    (Longhand::OverflowY, w),
                ]);
            }
            let words: Option<Vec<Value>> = components(tokens)
                .into_iter()
                .map(|t| keyword(std::slice::from_ref(t), OVERFLOWS))
                .collect();
            match words?.as_slice() {
                [both] => Some(vec![
                    (Longhand::OverflowX, both.clone()),
                    (Longhand::OverflowY, both.clone()),
                ]),
                [x, y] => Some(vec![
                    (Longhand::OverflowX, x.clone()),
                    (Longhand::OverflowY, y.clone()),
                ]),
                _ => None,
            }
        }
        "overflow-x" => single(Longhand::OverflowX, keyword(tokens, OVERFLOWS)),
        "overflow-y" => single(Longhand::OverflowY, keyword(tokens, OVERFLOWS)),
        "visibility" => single(
            Longhand::Visibility,
            keyword(tokens, &["visible", "hidden", "collapse"]),
        ),
        "opacity" => single(
            Longhand::Opacity,
            one().and_then(|t| match t {
                Token::Number(v) => Some(Value::Number(v.clamp(0.0, 1.0))),
                Token::Percentage(v) => Some(Value::Number((v / 100.0).clamp(0.0, 1.0))),
                _ => None,
            }),
        ),
        "flex-direction" => single(Longhand::FlexDirection, keyword(tokens, FLEX_DIRECTIONS)),
        "flex-wrap" => single(Longhand::FlexWrap, keyword(tokens, FLEX_WRAPS)),
        "flex-flow" => {
            if let Some(w) = wide {
                return Some(vec![
                    (Longhand::FlexDirection, w.clone()),
                    (Longhand::FlexWrap, w),
                ]);
            }
            let mut pairs = Vec::new();
            for token in components(tokens) {
                let one = std::slice::from_ref(token);
                if let Some(direction) = keyword(one, FLEX_DIRECTIONS) {
                    pairs.push((Longhand::FlexDirection, direction));
                } else {
                    pairs.push((Longhand::FlexWrap, keyword(one, FLEX_WRAPS)?));
                }
            }
            (!pairs.is_empty() && pairs.len() <= 2).then_some(pairs)
        }
        "flex-grow" => single(Longhand::FlexGrow, one().and_then(non_negative_number)),
        "flex-shrink" => single(Longhand::FlexShrink, one().and_then(non_negative_number)),
        "flex-basis" => single(
            Longhand::FlexBasis,
            one().and_then(|t| length_or_keywords(t, &["auto", "content"], false)),
        ),
        "flex" => {
            if let Some(w) = wide {
                return Some(vec![
                    (Longhand::FlexGrow, w.clone()),
                    (Longhand::FlexShrink, w.clone()),
                    (Longhand::FlexBasis, w),
                ]);
            }
            let parts = components(tokens);
            let auto = || Value::Ident("auto".to_owned());
            // Base omise : 0 %, qui vaut `content` quand la taille principale
            // du conteneur n'est pas imposée.
            let zero = || Value::Length(Length::Percent(0.0));
            let (grow, shrink, basis) = match parts.as_slice() {
                [Token::Ident(word)] if word.eq_ignore_ascii_case("none") => {
                    (Value::Number(0.0), Value::Number(0.0), auto())
                }
                [Token::Ident(word)] if word.eq_ignore_ascii_case("auto") => {
                    (Value::Number(1.0), Value::Number(1.0), auto())
                }
                [one] => match non_negative_number(one) {
                    // `flex: 2` vaut `2 1 0`.
                    Some(grow) => (grow, Value::Number(1.0), zero()),
                    None => (
                        Value::Number(1.0),
                        Value::Number(1.0),
                        length_or_keywords(one, &["auto", "content"], false)?,
                    ),
                },
                [first, second] => {
                    let grow = non_negative_number(first)?;
                    match non_negative_number(second) {
                        Some(shrink) => (grow, shrink, zero()),
                        None => (
                            grow,
                            Value::Number(1.0),
                            length_or_keywords(second, &["auto", "content"], false)?,
                        ),
                    }
                }
                [first, second, third] => (
                    non_negative_number(first)?,
                    non_negative_number(second)?,
                    // Une base nulle peut s'écrire `0`, sans unité.
                    match third {
                        Token::Number(v) if *v == 0.0 => Value::Length(Length::Px(0.0)),
                        other => length_or_keywords(other, &["auto", "content"], false)?,
                    },
                ),
                _ => return None,
            };
            Some(vec![
                (Longhand::FlexGrow, grow),
                (Longhand::FlexShrink, shrink),
                (Longhand::FlexBasis, basis),
            ])
        }
        "justify-content" => single(
            Longhand::JustifyContent,
            keyword(tokens, CONTENT_ALIGNMENTS),
        ),
        "align-content" => single(Longhand::AlignContent, keyword(tokens, CONTENT_ALIGNMENTS)),
        "align-items" => single(Longhand::AlignItems, keyword(tokens, ITEM_ALIGNMENTS)),
        "align-self" => single(Longhand::AlignSelf, keyword(tokens, ITEM_ALIGNMENTS)),
        "grid-template-columns" => single(
            Longhand::GridTemplateColumns,
            crate::grid::parse_track_list(tokens).map(Value::Tracks),
        ),
        "grid-template-rows" => single(
            Longhand::GridTemplateRows,
            crate::grid::parse_track_list(tokens).map(Value::Tracks),
        ),
        "grid-auto-columns" => single(
            Longhand::GridAutoColumns,
            crate::grid::parse_track_sizes(tokens).map(Value::TrackSizes),
        ),
        "grid-auto-rows" => single(
            Longhand::GridAutoRows,
            crate::grid::parse_track_sizes(tokens).map(Value::TrackSizes),
        ),
        "grid-auto-flow" => {
            let words: Vec<String> = components(tokens)
                .into_iter()
                .filter_map(ident_of)
                .collect();
            let valid = !words.is_empty()
                && words.len() == components(tokens).len()
                && words.len() <= 2
                && words
                    .iter()
                    .all(|w| matches!(w.as_str(), "row" | "column" | "dense"))
                && words.iter().filter(|w| *w != "dense").count() <= 1;
            let parsed = valid.then(|| {
                let column = words.iter().any(|w| w == "column");
                let dense = words.iter().any(|w| w == "dense");
                Value::Ident(
                    match (column, dense) {
                        (false, false) => "row",
                        (false, true) => "row dense",
                        (true, false) => "column",
                        (true, true) => "column dense",
                    }
                    .to_owned(),
                )
            });
            single(Longhand::GridAutoFlow, parsed)
        }
        "grid-row-start" => single(Longhand::GridRowStart, grid_line(tokens)),
        "grid-row-end" => single(Longhand::GridRowEnd, grid_line(tokens)),
        "grid-column-start" => single(Longhand::GridColumnStart, grid_line(tokens)),
        "grid-column-end" => single(Longhand::GridColumnEnd, grid_line(tokens)),
        "grid-row" | "grid-column" => {
            let (start, end) = if name == "grid-row" {
                (Longhand::GridRowStart, Longhand::GridRowEnd)
            } else {
                (Longhand::GridColumnStart, Longhand::GridColumnEnd)
            };
            if let Some(w) = wide {
                return Some(vec![(start, w.clone()), (end, w)]);
            }
            let parts: Vec<&[Token]> = tokens.split(|t| *t == Token::Delim('/')).collect();
            let first = grid_line(parts.first()?)?;
            let second = match parts.as_slice() {
                [_] => implied_line(&first),
                [_, second] => grid_line(second)?,
                _ => return None,
            };
            Some(vec![(start, first), (end, second)])
        }
        "grid-area" => {
            let properties = [
                Longhand::GridRowStart,
                Longhand::GridColumnStart,
                Longhand::GridRowEnd,
                Longhand::GridColumnEnd,
            ];
            if let Some(w) = wide {
                return Some(properties.iter().map(|p| (*p, w.clone())).collect());
            }
            let parts: Vec<&[Token]> = tokens.split(|t| *t == Token::Delim('/')).collect();
            if parts.is_empty() || parts.len() > 4 {
                return None;
            }
            let given: Option<Vec<Value>> = parts.iter().map(|part| grid_line(part)).collect();
            let given = given?;
            // Valeurs omises : le nom de la valeur dont elles dépendent, sinon `auto`.
            let row_start = given.first()?.clone();
            let column_start = given
                .get(1)
                .cloned()
                .unwrap_or_else(|| implied_line(&row_start));
            let row_end = given
                .get(2)
                .cloned()
                .unwrap_or_else(|| implied_line(&row_start));
            let column_end = given
                .get(3)
                .cloned()
                .unwrap_or_else(|| implied_line(&column_start));
            Some(
                properties
                    .into_iter()
                    .zip([row_start, column_start, row_end, column_end])
                    .collect(),
            )
        }
        "grid-template-areas" => single(
            Longhand::GridTemplateAreas,
            crate::grid::parse_areas(tokens).map(Value::Areas),
        ),
        "grid-template" | "grid" => {
            // Forme simple : `none`, ou `rangées / colonnes`.
            let none = || Value::Tracks(crate::grid::TrackList::default());
            let no_areas = || Value::Areas(crate::grid::GridAreas::default());
            if let Some(w) = wide {
                return Some(vec![
                    (Longhand::GridTemplateRows, w.clone()),
                    (Longhand::GridTemplateColumns, w.clone()),
                    (Longhand::GridTemplateAreas, w),
                ]);
            }
            let parts: Vec<&[Token]> = tokens.split(|t| *t == Token::Delim('/')).collect();
            let (rows, columns) = match parts.as_slice() {
                [only] if keyword(crate::syntax::trim(only), &["none"]).is_some() => {
                    (none(), none())
                }
                [rows, columns] => (
                    Value::Tracks(crate::grid::parse_track_list(rows)?),
                    Value::Tracks(crate::grid::parse_track_list(columns)?),
                ),
                _ => return None,
            };
            Some(vec![
                (Longhand::GridTemplateRows, rows),
                (Longhand::GridTemplateColumns, columns),
                (Longhand::GridTemplateAreas, no_areas()),
            ])
        }
        "border-spacing" => {
            if let Some(w) = wide {
                return Some(vec![
                    (Longhand::BorderSpacingH, w.clone()),
                    (Longhand::BorderSpacingV, w),
                ]);
            }
            let values: Option<Vec<Value>> = components(tokens)
                .into_iter()
                .map(|t| length_value(t, false))
                .collect();
            match values?.as_slice() {
                [both] => Some(vec![
                    (Longhand::BorderSpacingH, both.clone()),
                    (Longhand::BorderSpacingV, both.clone()),
                ]),
                [horizontal, vertical] => Some(vec![
                    (Longhand::BorderSpacingH, horizontal.clone()),
                    (Longhand::BorderSpacingV, vertical.clone()),
                ]),
                _ => None,
            }
        }
        "border-collapse" => single(
            Longhand::BorderCollapse,
            keyword(tokens, &["separate", "collapse"]),
        ),
        "table-layout" => single(Longhand::TableLayout, keyword(tokens, &["auto", "fixed"])),
        "caption-side" => single(Longhand::CaptionSide, keyword(tokens, &["top", "bottom"])),
        "vertical-align" => single(
            Longhand::VerticalAlign,
            one().and_then(|t| {
                length_or_keywords(
                    t,
                    &[
                        "baseline",
                        "top",
                        "middle",
                        "bottom",
                        "text-top",
                        "text-bottom",
                        "sub",
                        "super",
                    ],
                    true,
                )
            }),
        ),
        "justify-items" => single(Longhand::JustifyItems, keyword(tokens, SELF_ALIGNMENTS)),
        "justify-self" => single(Longhand::JustifySelf, keyword(tokens, SELF_ALIGNMENTS)),
        "place-items" | "place-self" | "place-content" => {
            let (block, inline, allowed) = match name {
                "place-items" => (
                    Longhand::AlignItems,
                    Longhand::JustifyItems,
                    SELF_ALIGNMENTS,
                ),
                "place-self" => (Longhand::AlignSelf, Longhand::JustifySelf, SELF_ALIGNMENTS),
                _ => (
                    Longhand::AlignContent,
                    Longhand::JustifyContent,
                    CONTENT_ALIGNMENTS,
                ),
            };
            if let Some(w) = wide {
                return Some(vec![(block, w.clone()), (inline, w)]);
            }
            let words: Option<Vec<Value>> = components(tokens)
                .into_iter()
                .map(|t| keyword(std::slice::from_ref(t), allowed))
                .collect();
            match words?.as_slice() {
                [both] => Some(vec![(block, both.clone()), (inline, both.clone())]),
                [first, second] => Some(vec![(block, first.clone()), (inline, second.clone())]),
                _ => None,
            }
        }
        "row-gap" | "grid-row-gap" => single(Longhand::RowGap, one().and_then(gap_value)),
        "column-gap" | "grid-column-gap" => single(Longhand::ColumnGap, one().and_then(gap_value)),
        "gap" | "grid-gap" => {
            if let Some(w) = wide {
                return Some(vec![
                    (Longhand::RowGap, w.clone()),
                    (Longhand::ColumnGap, w),
                ]);
            }
            let values: Option<Vec<Value>> =
                components(tokens).into_iter().map(gap_value).collect();
            match values?.as_slice() {
                [both] => Some(vec![
                    (Longhand::RowGap, both.clone()),
                    (Longhand::ColumnGap, both.clone()),
                ]),
                [row, column] => Some(vec![
                    (Longhand::RowGap, row.clone()),
                    (Longhand::ColumnGap, column.clone()),
                ]),
                _ => None,
            }
        }
        "list-style-type" | "list-style" => {
            if let Some(w) = wide {
                return Some(vec![(Longhand::ListStyleType, w)]);
            }
            let word = components(tokens).into_iter().find_map(|t| {
                ident_of(t).filter(|w| {
                    matches!(
                        w.as_str(),
                        "none" | "disc" | "circle" | "square" | "decimal"
                    )
                })
            })?;
            Some(vec![(Longhand::ListStyleType, Value::Ident(word))])
        }
        _ => {
            // margin-top, padding-inline-start, border-left, border-top-width, …
            if let Some(side) = name.strip_prefix("margin-").and_then(side_of) {
                return single(
                    Longhand::Margin(side),
                    one().and_then(|t| length_or_keywords(t, &["auto"], true)),
                );
            }
            if let Some(side) = name.strip_prefix("padding-").and_then(side_of) {
                return single(
                    Longhand::Padding(side),
                    one().and_then(|t| length_value(t, false)),
                );
            }
            let rest = name.strip_prefix("border-")?;
            if let Some(side) = side_of(rest) {
                return match wide {
                    Some(w) => Some(border_pairs(&[side], (w.clone(), w.clone(), w))),
                    None => Some(border_pairs(&[side], border_shorthand(tokens)?)),
                };
            }
            if let Some(side) = rest.strip_suffix("-width").and_then(side_of) {
                return single(
                    Longhand::BorderWidth(side),
                    one().and_then(|t| length_or_keywords(t, BORDER_WIDTHS, false)),
                );
            }
            if let Some(side) = rest.strip_suffix("-style").and_then(side_of) {
                return single(Longhand::BorderStyle(side), keyword(tokens, BORDER_STYLES));
            }
            if let Some(side) = rest.strip_suffix("-color").and_then(side_of) {
                return single(Longhand::BorderColor(side), color_value(tokens));
            }
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::tokenize;
    use crate::values::Color;

    fn decls(s: &str) -> Vec<Declaration> {
        parse_declarations(&tokenize(s))
    }

    #[test]
    fn longhands_and_important() {
        let d = decls("color: red !important; display:block; width: 50%; bogus: 1; color: 12px");
        assert_eq!(d.len(), 3);
        assert_eq!(d[0].property, Longhand::Color);
        assert_eq!(d[0].value, Value::Color(Color::rgb(255, 0, 0)));
        assert!(d[0].important);
        assert_eq!(d[1].value, Value::Ident("block".into()));
        assert_eq!(d[2].value, Value::Length(Length::Percent(50.0)));
    }

    #[test]
    fn margin_shorthand() {
        let d = decls("margin: 1px auto 3px");
        let got: Vec<_> = d.iter().map(|d| (d.property, d.value.clone())).collect();
        assert_eq!(
            got,
            vec![
                (Longhand::Margin(Side::Top), Value::Length(Length::Px(1.0))),
                (Longhand::Margin(Side::Right), Value::Ident("auto".into())),
                (
                    Longhand::Margin(Side::Bottom),
                    Value::Length(Length::Px(3.0))
                ),
                (Longhand::Margin(Side::Left), Value::Ident("auto".into())),
            ]
        );
        assert!(decls("padding: -1px").is_empty());
        assert!(decls("margin: 1px 2px 3px 4px 5px").is_empty());
    }

    #[test]
    fn border_and_font_shorthands() {
        let d = decls("border: 1px solid #2A313B");
        assert_eq!(d.len(), 12);
        assert_eq!(d[2].property, Longhand::BorderColor(Side::Top));
        assert_eq!(d[2].value, Value::Color(Color::rgb(0x2a, 0x31, 0x3b)));

        let d = decls("font: italic 600 14px/20px Inter, 'Segoe UI', sans-serif");
        let last = d.last().expect("famille");
        assert_eq!(
            last.value,
            Value::FontFamily(vec!["Inter".into(), "Segoe UI".into(), "sans-serif".into()])
        );
        assert!(d.iter().any(
            |d| d.property == Longhand::FontSize && d.value == Value::Length(Length::Px(14.0))
        ));
        assert!(
            d.iter()
                .any(|d| d.property == Longhand::FontWeight && d.value == Value::Number(600.0))
        );
    }

    #[test]
    fn background_keeps_only_color() {
        let d = decls("background: url(x.png) no-repeat #0E1116");
        assert_eq!(d[0].value, Value::Color(Color::rgb(0x0e, 0x11, 0x16)));
        let d = decls("background: linear-gradient(red, blue)");
        assert_eq!(d[0].value, Value::Color(Color::TRANSPARENT));
    }
}
