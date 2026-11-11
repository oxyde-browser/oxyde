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
        if let Some(pairs) = parse_property(&name, value) {
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
