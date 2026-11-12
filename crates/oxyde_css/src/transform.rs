//! `transform` : fonctions de transformation et matrices affines (CSS
//! Transforms 1).
//!
//! LIMITES CONNUES : seules les transformations du plan sont appliquées. Les
//! fonctions 3D sont lues, et ramenées à leur effet dans le plan
//! (`translate3d` à sa translation en x et y, `rotateZ` à une rotation,
//! `scale3d` à son échelle en x et y) ; `perspective`, `rotateX`, `rotateY`
//! et `matrix3d` sont sans effet.

use crate::syntax::{Token, matching_close, split_top_level, trim};
use crate::values::{Length, parse_length};

/// Transformation affine du plan, notée comme `matrix(a, b, c, d, e, f)` :
/// `x' = a·x + c·y + e` et `y' = b·x + d·y + f`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine(pub [f32; 6]);

impl Affine {
    /// La transformation identité.
    pub const IDENTITY: Self = Self([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

    /// Translation de (`x`, `y`).
    #[must_use]
    pub fn translate(x: f32, y: f32) -> Self {
        Self([1.0, 0.0, 0.0, 1.0, x, y])
    }

    /// Mise à l'échelle.
    #[must_use]
    pub fn scale(x: f32, y: f32) -> Self {
        Self([x, 0.0, 0.0, y, 0.0, 0.0])
    }

    /// Rotation de `radians` dans le sens des aiguilles d'une montre.
    #[must_use]
    pub fn rotate(radians: f32) -> Self {
        let (sin, cos) = radians.sin_cos();
        Self([cos, sin, -sin, cos, 0.0, 0.0])
    }

    /// Inclinaison d'angles `x` et `y`, en radians.
    #[must_use]
    pub fn skew(x: f32, y: f32) -> Self {
        Self([1.0, y.tan(), x.tan(), 1.0, 0.0, 0.0])
    }

    /// Composition : applique `inner` d'abord, puis `self`.
    #[must_use]
    pub fn then_inner(self, inner: Self) -> Self {
        let [a, b, c, d, e, f] = self.0;
        let [a2, b2, c2, d2, e2, f2] = inner.0;
        Self([
            a * a2 + c * b2,
            b * a2 + d * b2,
            a * c2 + c * d2,
            b * c2 + d * d2,
            a * e2 + c * f2 + e,
            b * e2 + d * f2 + f,
        ])
    }

    /// Image du point (`x`, `y`).
    #[must_use]
    pub fn apply(self, x: f32, y: f32) -> (f32, f32) {
        let [a, b, c, d, e, f] = self.0;
        (a * x + c * y + e, b * x + d * y + f)
    }

    /// Transformation réciproque ; `None` si la matrice n'est pas inversible
    /// (échelle nulle).
    #[must_use]
    pub fn inverse(self) -> Option<Self> {
        let [a, b, c, d, e, f] = self.0;
        let det = a * d - b * c;
        if det == 0.0 || !det.is_finite() {
            return None;
        }
        Some(Self([
            d / det,
            -b / det,
            -c / det,
            a / det,
            (c * f - d * e) / det,
            (b * e - a * f) / det,
        ]))
    }

    /// Vrai pour l'identité.
    #[must_use]
    pub fn is_identity(self) -> bool {
        self == Self::IDENTITY
    }

    /// Plus petit rectangle (`x`, `y`, largeur, hauteur) aligné sur les axes
    /// qui contient l'image du rectangle donné.
    #[must_use]
    pub fn bounds(self, x: f32, y: f32, w: f32, h: f32) -> (f32, f32, f32, f32) {
        let corners = [
            self.apply(x, y),
            self.apply(x + w, y),
            self.apply(x, y + h),
            self.apply(x + w, y + h),
        ];
        let min_x = corners.iter().map(|c| c.0).fold(f32::INFINITY, f32::min);
        let max_x = corners
            .iter()
            .map(|c| c.0)
            .fold(f32::NEG_INFINITY, f32::max);
        let min_y = corners.iter().map(|c| c.1).fold(f32::INFINITY, f32::min);
        let max_y = corners
            .iter()
            .map(|c| c.1)
            .fold(f32::NEG_INFINITY, f32::max);
        (min_x, min_y, max_x - min_x, max_y - min_y)
    }
}

/// Fonction de transformation ; `L` est le type des longueurs (spécifiées,
/// puis calculées).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TransformFunction<L> {
    /// `translate(x, y)` ; un pourcentage se rapporte à la boîte de bordure.
    Translate(L, L),
    /// `scale(x, y)`
    Scale(f32, f32),
    /// `rotate(angle)`, en radians.
    Rotate(f32),
    /// `skew(x, y)`, en radians.
    Skew(f32, f32),
    /// `matrix(a, b, c, d, e, f)`
    Matrix([f32; 6]),
}

impl<L: Copy> TransformFunction<L> {
    /// La même fonction, ses longueurs converties par `convert`.
    pub fn map<M>(self, convert: impl Fn(L) -> M) -> TransformFunction<M> {
        match self {
            Self::Translate(x, y) => TransformFunction::Translate(convert(x), convert(y)),
            Self::Scale(x, y) => TransformFunction::Scale(x, y),
            Self::Rotate(angle) => TransformFunction::Rotate(angle),
            Self::Skew(x, y) => TransformFunction::Skew(x, y),
            Self::Matrix(m) => TransformFunction::Matrix(m),
        }
    }
}

/// Angle en radians.
fn angle(token: &Token) -> Option<f32> {
    match token {
        Token::Number(v) if *v == 0.0 => Some(0.0),
        Token::Dimension(v, unit) => match unit.as_str() {
            "deg" => Some(v.to_radians()),
            "grad" => Some((v * 0.9).to_radians()),
            "rad" => Some(*v),
            "turn" => Some(v * std::f32::consts::TAU),
            _ => None,
        },
        _ => None,
    }
}

/// Nombre ; un pourcentage vaut sa fraction (`scale(50%)`).
fn number(token: &Token) -> Option<f32> {
    match token {
        Token::Number(v) => Some(*v),
        Token::Percentage(v) => Some(v / 100.0),
        _ => None,
    }
}

/// Arguments d'une fonction : un jeton par argument, séparés par des virgules.
fn arguments(inner: &[Token]) -> Option<Vec<&Token>> {
    split_top_level(inner, &Token::Comma)
        .into_iter()
        .map(|part| match trim(part) {
            [single] => Some(single),
            _ => None,
        })
        .collect()
}

/// Analyse la valeur de `transform` : `none` (liste vide) ou une suite de
/// fonctions.
#[must_use]
pub fn parse_transform(tokens: &[Token]) -> Option<Vec<TransformFunction<Length>>> {
    let tokens = trim(tokens);
    if let [Token::Ident(word)] = tokens
        && word.eq_ignore_ascii_case("none")
    {
        return Some(Vec::new());
    }
    let zero = Length::Px(0.0);
    let mut out = Vec::new();
    let mut at = 0;
    while at < tokens.len() {
        let Token::Function(name) = &tokens[at] else {
            if tokens[at] == Token::Whitespace {
                at += 1;
                continue;
            }
            return None;
        };
        let close = matching_close(tokens, at);
        let args = arguments(tokens.get(at + 1..close)?)?;
        let lengths: Option<Vec<Length>> = args.iter().map(|t| parse_length(t)).collect();
        let numbers: Option<Vec<f32>> = args.iter().map(|t| number(t)).collect();
        let angles: Option<Vec<f32>> = args.iter().map(|t| angle(t)).collect();
        let function = match (name.as_str(), args.len()) {
            ("translate", 1) => TransformFunction::Translate(lengths?[0], zero),
            ("translate", 2) => {
                let l = lengths?;
                TransformFunction::Translate(l[0], l[1])
            }
            ("translatex", 1) => TransformFunction::Translate(lengths?[0], zero),
            ("translatey", 1) => TransformFunction::Translate(zero, lengths?[0]),
            ("translate3d", 3) => {
                let x = parse_length(args[0])?;
                let y = parse_length(args[1])?;
                parse_length(args[2])?;
                TransformFunction::Translate(x, y)
            }
            ("translatez", 1) => {
                lengths?;
                TransformFunction::Scale(1.0, 1.0)
            }
            ("scale", 1) => {
                let n = numbers?[0];
                TransformFunction::Scale(n, n)
            }
            ("scale", 2) | ("scale3d", 3) => {
                let n = numbers?;
                TransformFunction::Scale(n[0], n[1])
            }
            ("scalex", 1) => TransformFunction::Scale(numbers?[0], 1.0),
            ("scaley", 1) => TransformFunction::Scale(1.0, numbers?[0]),
            ("scalez", 1) => {
                numbers?;
                TransformFunction::Scale(1.0, 1.0)
            }
            ("rotate" | "rotatez", 1) => TransformFunction::Rotate(angles?[0]),
            ("rotatex" | "rotatey", 1) => {
                angles?;
                TransformFunction::Scale(1.0, 1.0)
            }
            ("skew", 1) => TransformFunction::Skew(angles?[0], 0.0),
            ("skew", 2) => {
                let a = angles?;
                TransformFunction::Skew(a[0], a[1])
            }
            ("skewx", 1) => TransformFunction::Skew(angles?[0], 0.0),
            ("skewy", 1) => TransformFunction::Skew(0.0, angles?[0]),
            ("matrix", 6) => {
                let n = numbers?;
                TransformFunction::Matrix([n[0], n[1], n[2], n[3], n[4], n[5]])
            }
            ("matrix3d", 16) => {
                // Partie plane d'une matrice 4 × 4 donnée par colonnes.
                let n = numbers?;
                TransformFunction::Matrix([n[0], n[1], n[4], n[5], n[12], n[13]])
            }
            ("perspective", 1) => TransformFunction::Scale(1.0, 1.0),
            _ => return None,
        };
        out.push(function);
        at = close + 1;
    }
    if out.is_empty() { None } else { Some(out) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::tokenize;

    #[test]
    fn parses_function_lists() {
        let list = parse_transform(&tokenize("translate(10px, 50%) rotate(90deg) scale(2)"))
            .expect("liste valide");
        assert_eq!(
            list,
            vec![
                TransformFunction::Translate(Length::Px(10.0), Length::Percent(50.0)),
                TransformFunction::Rotate(std::f32::consts::FRAC_PI_2),
                TransformFunction::Scale(2.0, 2.0),
            ]
        );
        assert_eq!(parse_transform(&tokenize("none")), Some(Vec::new()));
        assert_eq!(parse_transform(&tokenize("rotate(10px)")), None);
        assert_eq!(parse_transform(&tokenize("translate(1px 2px)")), None);
    }

    #[test]
    fn composes_and_inverts() {
        // Translation puis échelle, dans l'ordre CSS : l'échelle s'applique d'abord.
        let m = Affine::translate(10.0, 0.0).then_inner(Affine::scale(2.0, 2.0));
        assert_eq!(m.apply(1.0, 1.0), (12.0, 2.0));
        let back = m.inverse().expect("inversible");
        assert_eq!(back.apply(12.0, 2.0), (1.0, 1.0));
        assert!(Affine::scale(0.0, 1.0).inverse().is_none());
        let quarter = Affine::rotate(std::f32::consts::FRAC_PI_2);
        let (x, y, w, h) = quarter.bounds(0.0, 0.0, 10.0, 20.0);
        assert!((x + 20.0).abs() < 1e-4 && y.abs() < 1e-4);
        assert!((w - 20.0).abs() < 1e-4 && (h - 10.0).abs() < 1e-4);
    }
}
