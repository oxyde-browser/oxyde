//! Propriétés de peinture SVG : `fill`, `stroke` et leurs compagnes.
//!
//! Toutes sont héritées ; [`SvgStyle`] est donc partagé tel quel entre un
//! élément et ses descendants tant qu'aucun n'en déclare une.
//!
//! LIMITES CONNUES : pas de `context-fill` ni de `context-stroke` (lus
//! comme `none`) ; `paint-order`, `vector-effect` et les marqueurs ne sont
//! pas lus ; un pourcentage dans `stroke-width` ou `stroke-dasharray` vaut
//! zéro.

use std::sync::Arc;

use crate::syntax::Token;
use crate::values::{Color, ParsedColor, parse_color};

/// Peinture d'un remplissage ou d'un contour.
#[derive(Clone, Debug, PartialEq)]
pub enum SvgPaint {
    /// `none`
    None,
    /// Couleur unie.
    Color(Color),
    /// `currentcolor` : la valeur de `color` de l'élément peint.
    CurrentColor,
    /// `url(#id)` : un serveur de peinture (dégradé) du document, et la
    /// couleur de repli s'il manque.
    Server(Arc<str>, Option<Color>),
}

/// `stroke-linecap`
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SvgLineCap {
    /// `butt`
    #[default]
    Butt,
    /// `round`
    Round,
    /// `square`
    Square,
}

/// `stroke-linejoin`
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SvgLineJoin {
    /// `miter`
    #[default]
    Miter,
    /// `round`
    Round,
    /// `bevel`
    Bevel,
}

/// Valeurs calculées des propriétés de peinture SVG.
#[derive(Clone, Debug, PartialEq)]
pub struct SvgStyle {
    /// `fill`
    pub fill: SvgPaint,
    /// `stroke`
    pub stroke: SvgPaint,
    /// `stroke-width`, en unités de l'utilisateur.
    pub stroke_width: f32,
    /// `fill-opacity` (0–1).
    pub fill_opacity: f32,
    /// `stroke-opacity` (0–1).
    pub stroke_opacity: f32,
    /// `fill-rule: evenodd`
    pub even_odd: bool,
    /// `stroke-linecap`
    pub line_cap: SvgLineCap,
    /// `stroke-linejoin`
    pub line_join: SvgLineJoin,
    /// `stroke-miterlimit`
    pub miter_limit: f32,
    /// `stroke-dasharray` ; vide : trait continu.
    pub dash_array: Arc<[f32]>,
    /// `stroke-dashoffset`
    pub dash_offset: f32,
}

impl Default for SvgStyle {
    fn default() -> Self {
        Self {
            fill: SvgPaint::Color(Color::BLACK),
            stroke: SvgPaint::None,
            stroke_width: 1.0,
            fill_opacity: 1.0,
            stroke_opacity: 1.0,
            even_odd: false,
            line_cap: SvgLineCap::Butt,
            line_join: SvgLineJoin::Miter,
            miter_limit: 4.0,
            dash_array: Arc::from([]),
            dash_offset: 0.0,
        }
    }
}

impl SvgStyle {
    /// Ce style, transporté d'un contexte d'héritage à un autre : chaque
    /// propriété qui vaut encore celle de `from` (elle en a hérité) prend
    /// celle de `onto`. C'est ainsi que le contenu d'un `<use>` hérite de
    /// lui plutôt que de l'endroit où le modèle est rangé.
    #[must_use]
    pub fn rebased(&self, from: &Self, onto: &Self) -> Self {
        fn pick<T: Clone + PartialEq>(own: &T, from: &T, onto: &T) -> T {
            if own == from {
                onto.clone()
            } else {
                own.clone()
            }
        }
        Self {
            fill: pick(&self.fill, &from.fill, &onto.fill),
            stroke: pick(&self.stroke, &from.stroke, &onto.stroke),
            stroke_width: pick(&self.stroke_width, &from.stroke_width, &onto.stroke_width),
            fill_opacity: pick(&self.fill_opacity, &from.fill_opacity, &onto.fill_opacity),
            stroke_opacity: pick(
                &self.stroke_opacity,
                &from.stroke_opacity,
                &onto.stroke_opacity,
            ),
            even_odd: pick(&self.even_odd, &from.even_odd, &onto.even_odd),
            line_cap: pick(&self.line_cap, &from.line_cap, &onto.line_cap),
            line_join: pick(&self.line_join, &from.line_join, &onto.line_join),
            miter_limit: pick(&self.miter_limit, &from.miter_limit, &onto.miter_limit),
            dash_array: pick(&self.dash_array, &from.dash_array, &onto.dash_array),
            dash_offset: pick(&self.dash_offset, &from.dash_offset, &onto.dash_offset),
        }
    }
}

/// Analyse la valeur de `fill` ou de `stroke`.
#[must_use]
pub fn parse_paint(tokens: &[Token]) -> Option<SvgPaint> {
    let words: Vec<&Token> = tokens
        .iter()
        .filter(|token| **token != Token::Whitespace)
        .collect();
    let plain = |tokens: &[Token]| -> Option<SvgPaint> {
        if let [Token::Ident(word)] = tokens {
            let lower = word.to_ascii_lowercase();
            if matches!(lower.as_str(), "none" | "context-fill" | "context-stroke") {
                return Some(SvgPaint::None);
            }
        }
        let (color, used) = parse_color(tokens)?;
        (used == tokens.len()).then_some(match color {
            ParsedColor::Color(c) => SvgPaint::Color(c),
            ParsedColor::CurrentColor => SvgPaint::CurrentColor,
        })
    };
    // `url(#id)` arrive en un jeton, `url("#id")` en trois.
    let (target, after) = match words.as_slice() {
        [Token::Url(url), ..] => (url.as_str(), 1),
        [
            Token::Function(name),
            Token::Str(url),
            Token::CloseParen,
            ..,
        ] if name == "url" => (url.as_str(), 3),
        _ => return plain(tokens),
    };
    let id = target.trim().strip_prefix('#')?;
    if id.is_empty() {
        return None;
    }
    // Ce qui suit l'adresse est la couleur de repli.
    let position = tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| **token != Token::Whitespace)
        .nth(after - 1)
        .map(|(index, _)| index + 1)?;
    let rest = crate::syntax::trim(tokens.get(position..)?);
    let fallback = if rest.is_empty() {
        None
    } else {
        match plain(rest)? {
            SvgPaint::Color(c) => Some(c),
            SvgPaint::None => Some(Color::TRANSPARENT),
            SvgPaint::CurrentColor | SvgPaint::Server(..) => None,
        }
    };
    Some(SvgPaint::Server(Arc::from(id), fallback))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::tokenize;

    #[test]
    fn parses_paints() {
        assert_eq!(parse_paint(&tokenize("none")), Some(SvgPaint::None));
        assert_eq!(
            parse_paint(&tokenize("currentColor")),
            Some(SvgPaint::CurrentColor)
        );
        assert_eq!(
            parse_paint(&tokenize("#f00")),
            Some(SvgPaint::Color(Color {
                r: 255,
                g: 0,
                b: 0,
                a: 255
            }))
        );
        assert_eq!(
            parse_paint(&tokenize("url(#a)")),
            Some(SvgPaint::Server(Arc::from("a"), None))
        );
        assert_eq!(
            parse_paint(&tokenize("url('#a') blue")),
            Some(SvgPaint::Server(
                Arc::from("a"),
                Some(Color {
                    r: 0,
                    g: 0,
                    b: 255,
                    a: 255
                })
            ))
        );
        assert_eq!(parse_paint(&tokenize("url(a.svg#b)")), None);
        assert_eq!(parse_paint(&tokenize("1px")), None);
    }
}
