//! Types de valeurs CSS : couleurs et longueurs.

use crate::syntax::{Token, matching_close, trim};

/// Couleur sRGB 8 bits par canal, alpha non prémultiplié.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    /// Rouge.
    pub r: u8,
    /// Vert.
    pub g: u8,
    /// Bleu.
    pub b: u8,
    /// Opacité.
    pub a: u8,
}

impl Color {
    /// Noir opaque.
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    /// Blanc opaque.
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    /// Transparent.
    pub const TRANSPARENT: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };

    /// Couleur opaque.
    #[must_use]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
}

/// Longueur telle que spécifiée.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Length {
    /// Pixels CSS.
    Px(f32),
    /// Relatif à la taille de police de l'élément.
    Em(f32),
    /// Relatif à la taille de police de la racine.
    Rem(f32),
    /// Pourcentage.
    Percent(f32),
    /// Centième de la largeur de la fenêtre.
    Vw(f32),
    /// Centième de la hauteur de la fenêtre.
    Vh(f32),
    /// Somme de longueurs d'unités différentes (`calc(100% - 20px)`).
    Calc(Linear),
}

/// Somme pondérée de longueurs : le résultat d'un `calc()` qui mêle des
/// unités. Chaque champ est le coefficient de son unité.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
#[allow(missing_docs, reason = "les champs portent le nom de l'unité CSS")]
pub struct Linear {
    pub px: f32,
    pub em: f32,
    pub rem: f32,
    pub percent: f32,
    pub vw: f32,
    pub vh: f32,
    /// Vrai si l'expression contient un pourcentage, même nul : elle dépend
    /// alors du bloc conteneur (`calc(0px + 0%)` n'est pas `0px`).
    pub has_percent: bool,
}

impl Linear {
    fn of(length: Length) -> Self {
        let mut linear = Self::default();
        match length {
            Length::Px(v) => linear.px = v,
            Length::Em(v) => linear.em = v,
            Length::Rem(v) => linear.rem = v,
            Length::Percent(v) => {
                linear.percent = v;
                linear.has_percent = true;
            }
            Length::Vw(v) => linear.vw = v,
            Length::Vh(v) => linear.vh = v,
            Length::Calc(inner) => linear = inner,
        }
        linear
    }

    fn combine(self, other: Self, sign: f32) -> Self {
        Self {
            px: self.px + sign * other.px,
            em: self.em + sign * other.em,
            rem: self.rem + sign * other.rem,
            percent: self.percent + sign * other.percent,
            vw: self.vw + sign * other.vw,
            vh: self.vh + sign * other.vh,
            has_percent: self.has_percent || other.has_percent,
        }
    }

    fn scale(self, factor: f32) -> Self {
        Self {
            px: self.px * factor,
            em: self.em * factor,
            rem: self.rem * factor,
            percent: self.percent * factor,
            vw: self.vw * factor,
            vh: self.vh * factor,
            has_percent: self.has_percent,
        }
    }

    /// La longueur la plus simple qui vaut cette somme.
    fn simplify(self) -> Length {
        let others = |keep: f32| {
            let total =
                self.px.abs() + self.em.abs() + self.rem.abs() + self.vw.abs() + self.vh.abs();
            (total - keep.abs()).abs() < f32::EPSILON
        };
        if self.has_percent {
            // Un pourcentage seul reste un pourcentage, même nul.
            if others(0.0) {
                Length::Percent(self.percent)
            } else {
                Length::Calc(self)
            }
        } else if others(self.px) {
            Length::Px(self.px)
        } else if others(self.em) {
            Length::Em(self.em)
        } else {
            Length::Calc(self)
        }
    }
}

/// Opérande d'un `calc()` : un nombre sans unité ou une longueur.
#[derive(Clone, Copy)]
enum Operand {
    Number(f32),
    Length(Linear),
}

/// Profondeur maximale d'imbrication d'un `calc()`.
const MAX_CALC_DEPTH: usize = 32;

/// Évalue la somme `tokens` (le contenu d'un `calc()` ou d'une parenthèse).
fn calc_sum(tokens: &[Token], depth: usize) -> Option<Operand> {
    if depth > MAX_CALC_DEPTH {
        return None;
    }
    // Découpe au niveau zéro sur « + » et « - » entourés de blancs.
    let mut terms: Vec<(f32, &[Token])> = Vec::new();
    let (mut level, mut start, mut sign) = (0usize, 0usize, 1.0_f32);
    for (i, token) in tokens.iter().enumerate() {
        match token {
            Token::OpenParen | Token::Function(_) => level += 1,
            Token::CloseParen => level = level.saturating_sub(1),
            Token::Delim(op @ ('+' | '-'))
                if level == 0
                    && i > 0
                    && tokens.get(i - 1) == Some(&Token::Whitespace)
                    && tokens.get(i + 1) == Some(&Token::Whitespace) =>
            {
                terms.push((sign, tokens.get(start..i)?));
                sign = if *op == '+' { 1.0 } else { -1.0 };
                start = i + 1;
            }
            _ => {}
        }
    }
    terms.push((sign, tokens.get(start..)?));
    let mut total: Option<Operand> = None;
    for (sign, term) in terms {
        let value = calc_product(term, depth)?;
        total = Some(match (total, value) {
            (None, Operand::Number(n)) => Operand::Number(sign * n),
            (None, Operand::Length(l)) => Operand::Length(l.scale(sign)),
            (Some(Operand::Number(a)), Operand::Number(b)) => Operand::Number(a + sign * b),
            (Some(Operand::Length(a)), Operand::Length(b)) => Operand::Length(a.combine(b, sign)),
            // Un nombre ne s'ajoute pas à une longueur.
            _ => return None,
        });
    }
    total
}

/// Évalue le produit `tokens` : des opérandes séparés par « * » ou « / ».
fn calc_product(tokens: &[Token], depth: usize) -> Option<Operand> {
    let mut factors: Vec<(char, &[Token])> = Vec::new();
    let (mut level, mut start, mut op) = (0usize, 0usize, '*');
    for (i, token) in tokens.iter().enumerate() {
        match token {
            Token::OpenParen | Token::Function(_) => level += 1,
            Token::CloseParen => level = level.saturating_sub(1),
            Token::Delim(next @ ('*' | '/')) if level == 0 => {
                factors.push((op, tokens.get(start..i)?));
                op = *next;
                start = i + 1;
            }
            _ => {}
        }
    }
    factors.push((op, tokens.get(start..)?));
    let mut total: Option<Operand> = None;
    for (op, factor) in factors {
        let value = calc_unit(factor, depth)?;
        total = Some(match (total, op, value) {
            (None, _, value) => value,
            (Some(Operand::Number(a)), '*', Operand::Number(b)) => Operand::Number(a * b),
            (Some(Operand::Number(a)), '*', Operand::Length(l))
            | (Some(Operand::Length(l)), '*', Operand::Number(a)) => Operand::Length(l.scale(a)),
            (Some(Operand::Number(a)), '/', Operand::Number(b)) if b != 0.0 => {
                Operand::Number(a / b)
            }
            (Some(Operand::Length(l)), '/', Operand::Number(b)) if b != 0.0 => {
                Operand::Length(l.scale(1.0 / b))
            }
            _ => return None,
        });
    }
    total
}

/// Évalue un opérande : nombre, longueur, parenthèse ou `calc()` imbriqué.
fn calc_unit(tokens: &[Token], depth: usize) -> Option<Operand> {
    let tokens = crate::syntax::trim(tokens);
    match tokens {
        [Token::Number(n)] => Some(Operand::Number(*n)),
        [Token::Calc(linear)] => Some(Operand::Length(*linear)),
        [single] => parse_length(single).map(|l| Operand::Length(Linear::of(l))),
        [Token::OpenParen, inner @ .., Token::CloseParen] => calc_sum(inner, depth + 1),
        [Token::Function(name), inner @ .., Token::CloseParen] if name == "calc" => {
            calc_sum(inner, depth + 1)
        }
        _ => None,
    }
}

/// Remplace chaque expression `calc(…)` de `tokens` par un jeton unique :
/// sa valeur si elle est une longueur, un nombre si elle n'a pas d'unité.
/// `None` si une expression est invalide (la déclaration est alors écartée).
#[must_use]
pub fn fold_calc(tokens: &[Token]) -> Option<Vec<Token>> {
    if !tokens
        .iter()
        .any(|t| matches!(t, Token::Function(name) if name == "calc"))
    {
        return Some(tokens.to_vec());
    }
    let mut out = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            Token::Function(name) if name == "calc" => {
                let close = crate::syntax::matching_close(tokens, i);
                let inner = tokens.get(i + 1..close)?;
                out.push(match calc_sum(inner, 0)? {
                    Operand::Number(n) => Token::Number(n),
                    Operand::Length(linear) => Token::Calc(linear),
                });
                i = close + 1;
            }
            other => {
                out.push(other.clone());
                i += 1;
            }
        }
    }
    Some(out)
}

/// Analyse un jeton en longueur. `allow_unitless_zero` : `0` vaut `0px`.
#[must_use]
pub fn parse_length(token: &Token) -> Option<Length> {
    match token {
        Token::Number(v) if *v == 0.0 => Some(Length::Px(0.0)),
        Token::Percentage(v) => Some(Length::Percent(*v)),
        Token::Calc(linear) => Some(linear.simplify()),
        Token::Dimension(v, unit) => Some(match unit.as_str() {
            "px" => Length::Px(*v),
            "em" => Length::Em(*v),
            "rem" => Length::Rem(*v),
            "vw" | "vi" | "svw" | "lvw" | "dvw" => Length::Vw(*v),
            "vh" | "vb" | "svh" | "lvh" | "dvh" => Length::Vh(*v),
            "pt" => Length::Px(*v * 96.0 / 72.0),
            "pc" => Length::Px(*v * 16.0),
            "in" => Length::Px(*v * 96.0),
            "cm" => Length::Px(*v * 96.0 / 2.54),
            "mm" => Length::Px(*v * 96.0 / 25.4),
            "q" => Length::Px(*v * 96.0 / 101.6),
            "ex" | "ch" => Length::Em(*v * 0.5),
            _ => return None,
        }),
        _ => None,
    }
}

fn clamp_u8(v: f32) -> u8 {
    // `as` sature pour les flottants ; NaN donne 0.
    v.round().clamp(0.0, 255.0) as u8
}

fn hex_color(hex: &str) -> Option<Color> {
    let digits: Option<Vec<u8>> = hex
        .chars()
        .map(|c| c.to_digit(16).and_then(|d| u8::try_from(d).ok()))
        .collect();
    let d = digits?;
    Some(match d.as_slice() {
        [r, g, b] => Color::rgb(r * 17, g * 17, b * 17),
        [r, g, b, a] => Color {
            r: r * 17,
            g: g * 17,
            b: b * 17,
            a: a * 17,
        },
        [r1, r2, g1, g2, b1, b2] => Color::rgb(r1 * 16 + r2, g1 * 16 + g2, b1 * 16 + b2),
        [r1, r2, g1, g2, b1, b2, a1, a2] => Color {
            r: r1 * 16 + r2,
            g: g1 * 16 + g2,
            b: b1 * 16 + b2,
            a: a1 * 16 + a2,
        },
        _ => return None,
    })
}

/// Arguments numériques d'une fonction de couleur, virgules et `/` ignorés.
fn color_args(tokens: &[Token]) -> Option<Vec<&Token>> {
    let mut out = Vec::new();
    for t in tokens {
        match t {
            Token::Whitespace | Token::Comma | Token::Delim('/') => {}
            Token::Number(_) | Token::Percentage(_) | Token::Dimension(..) => out.push(t),
            Token::Ident(s) if s.eq_ignore_ascii_case("none") => out.push(t),
            _ => return None,
        }
    }
    Some(out)
}

fn alpha_arg(token: Option<&&Token>) -> Option<u8> {
    Some(match token {
        None => 255,
        Some(Token::Number(v)) => clamp_u8(v * 255.0),
        Some(Token::Percentage(v)) => clamp_u8(v * 2.55),
        Some(Token::Ident(_)) => 0,
        _ => return None,
    })
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let h = h.rem_euclid(360.0) / 360.0;
    let (s, l) = (s.clamp(0.0, 1.0), l.clamp(0.0, 1.0));
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let channel = |mut t: f32| {
        t = t.rem_euclid(1.0);
        let v = if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        };
        clamp_u8(v * 255.0)
    };
    (channel(h + 1.0 / 3.0), channel(h), channel(h - 1.0 / 3.0))
}

fn color_function(name: &str, args: &[Token]) -> Option<Color> {
    let args = color_args(args)?;
    if args.len() != 3 && args.len() != 4 {
        return None;
    }
    let a = alpha_arg(args.get(3))?;
    match name {
        "rgb" | "rgba" => {
            let channel = |t: &Token| match t {
                Token::Number(v) => Some(clamp_u8(*v)),
                Token::Percentage(v) => Some(clamp_u8(v * 2.55)),
                Token::Ident(_) => Some(0),
                _ => None,
            };
            Some(Color {
                r: channel(args[0])?,
                g: channel(args[1])?,
                b: channel(args[2])?,
                a,
            })
        }
        "hsl" | "hsla" => {
            let hue = match args[0] {
                Token::Number(v) => *v,
                Token::Dimension(v, unit) => match unit.as_str() {
                    "deg" => *v,
                    "grad" => v * 0.9,
                    "rad" => v.to_degrees(),
                    "turn" => v * 360.0,
                    _ => return None,
                },
                Token::Ident(_) => 0.0,
                Token::Percentage(_) => return None,
                _ => return None,
            };
            let pct = |t: &Token| match t {
                Token::Percentage(v) | Token::Number(v) => Some(v / 100.0),
                Token::Ident(_) => Some(0.0),
                _ => None,
            };
            let (r, g, b) = hsl_to_rgb(hue, pct(args[1])?, pct(args[2])?);
            Some(Color { r, g, b, a })
        }
        _ => None,
    }
}

/// Résultat de l'analyse d'une couleur.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParsedColor {
    /// Couleur absolue.
    Color(Color),
    /// `currentcolor`
    CurrentColor,
}

/// Analyse une couleur au début de `tokens` ; renvoie la couleur et le nombre
/// de jetons consommés.
#[must_use]
pub fn parse_color(tokens: &[Token]) -> Option<(ParsedColor, usize)> {
    match tokens.first()? {
        Token::Hash(hex) => Some((ParsedColor::Color(hex_color(hex)?), 1)),
        Token::Ident(name) => {
            let lower = name.to_ascii_lowercase();
            if lower == "currentcolor" {
                return Some((ParsedColor::CurrentColor, 1));
            }
            Some((ParsedColor::Color(named_color(&lower)?), 1))
        }
        Token::Function(name) => {
            let close = matching_close(tokens, 0);
            let args = trim(tokens.get(1..close)?);
            Some((
                ParsedColor::Color(color_function(name, args)?),
                (close + 1).min(tokens.len()),
            ))
        }
        _ => None,
    }
}

fn named_color(name: &str) -> Option<Color> {
    let (r, g, b) = match name {
        "transparent" => return Some(Color::TRANSPARENT),
        "aliceblue" => (240, 248, 255),
        "antiquewhite" => (250, 235, 215),
        "aqua" | "cyan" => (0, 255, 255),
        "aquamarine" => (127, 255, 212),
        "azure" => (240, 255, 255),
        "beige" => (245, 245, 220),
        "bisque" => (255, 228, 196),
        "black" => (0, 0, 0),
        "blanchedalmond" => (255, 235, 205),
        "blue" => (0, 0, 255),
        "blueviolet" => (138, 43, 226),
        "brown" => (165, 42, 42),
        "burlywood" => (222, 184, 135),
        "cadetblue" => (95, 158, 160),
        "chartreuse" => (127, 255, 0),
        "chocolate" => (210, 105, 30),
        "coral" => (255, 127, 80),
        "cornflowerblue" => (100, 149, 237),
        "cornsilk" => (255, 248, 220),
        "crimson" => (220, 20, 60),
        "darkblue" => (0, 0, 139),
        "darkcyan" => (0, 139, 139),
        "darkgoldenrod" => (184, 134, 11),
        "darkgray" | "darkgrey" => (169, 169, 169),
        "darkgreen" => (0, 100, 0),
        "darkkhaki" => (189, 183, 107),
        "darkmagenta" => (139, 0, 139),
        "darkolivegreen" => (85, 107, 47),
        "darkorange" => (255, 140, 0),
        "darkorchid" => (153, 50, 204),
        "darkred" => (139, 0, 0),
        "darksalmon" => (233, 150, 122),
        "darkseagreen" => (143, 188, 143),
        "darkslateblue" => (72, 61, 139),
        "darkslategray" | "darkslategrey" => (47, 79, 79),
        "darkturquoise" => (0, 206, 209),
        "darkviolet" => (148, 0, 211),
        "deeppink" => (255, 20, 147),
        "deepskyblue" => (0, 191, 255),
        "dimgray" | "dimgrey" => (105, 105, 105),
        "dodgerblue" => (30, 144, 255),
        "firebrick" => (178, 34, 34),
        "floralwhite" => (255, 250, 240),
        "forestgreen" => (34, 139, 34),
        "fuchsia" | "magenta" => (255, 0, 255),
        "gainsboro" => (220, 220, 220),
        "ghostwhite" => (248, 248, 255),
        "gold" => (255, 215, 0),
        "goldenrod" => (218, 165, 32),
        "gray" | "grey" => (128, 128, 128),
        "green" => (0, 128, 0),
        "greenyellow" => (173, 255, 47),
        "honeydew" => (240, 255, 240),
        "hotpink" => (255, 105, 180),
        "indianred" => (205, 92, 92),
        "indigo" => (75, 0, 130),
        "ivory" => (255, 255, 240),
        "khaki" => (240, 230, 140),
        "lavender" => (230, 230, 250),
        "lavenderblush" => (255, 240, 245),
        "lawngreen" => (124, 252, 0),
        "lemonchiffon" => (255, 250, 205),
        "lightblue" => (173, 216, 230),
        "lightcoral" => (240, 128, 128),
        "lightcyan" => (224, 255, 255),
        "lightgoldenrodyellow" => (250, 250, 210),
        "lightgray" | "lightgrey" => (211, 211, 211),
        "lightgreen" => (144, 238, 144),
        "lightpink" => (255, 182, 193),
        "lightsalmon" => (255, 160, 122),
        "lightseagreen" => (32, 178, 170),
        "lightskyblue" => (135, 206, 250),
        "lightslategray" | "lightslategrey" => (119, 136, 153),
        "lightsteelblue" => (176, 196, 222),
        "lightyellow" => (255, 255, 224),
        "lime" => (0, 255, 0),
        "limegreen" => (50, 205, 50),
        "linen" => (250, 240, 230),
        "maroon" => (128, 0, 0),
        "mediumaquamarine" => (102, 205, 170),
        "mediumblue" => (0, 0, 205),
        "mediumorchid" => (186, 85, 211),
        "mediumpurple" => (147, 112, 219),
        "mediumseagreen" => (60, 179, 113),
        "mediumslateblue" => (123, 104, 238),
        "mediumspringgreen" => (0, 250, 154),
        "mediumturquoise" => (72, 209, 204),
        "mediumvioletred" => (199, 21, 133),
        "midnightblue" => (25, 25, 112),
        "mintcream" => (245, 255, 250),
        "mistyrose" => (255, 228, 225),
        "moccasin" => (255, 228, 181),
        "navajowhite" => (255, 222, 173),
        "navy" => (0, 0, 128),
        "oldlace" => (253, 245, 230),
        "olive" => (128, 128, 0),
        "olivedrab" => (107, 142, 35),
        "orange" => (255, 165, 0),
        "orangered" => (255, 69, 0),
        "orchid" => (218, 112, 214),
        "palegoldenrod" => (238, 232, 170),
        "palegreen" => (152, 251, 152),
        "paleturquoise" => (175, 238, 238),
        "palevioletred" => (219, 112, 147),
        "papayawhip" => (255, 239, 213),
        "peachpuff" => (255, 218, 185),
        "peru" => (205, 133, 63),
        "pink" => (255, 192, 203),
        "plum" => (221, 160, 221),
        "powderblue" => (176, 224, 230),
        "purple" => (128, 0, 128),
        "rebeccapurple" => (102, 51, 153),
        "red" => (255, 0, 0),
        "rosybrown" => (188, 143, 143),
        "royalblue" => (65, 105, 225),
        "saddlebrown" => (139, 69, 19),
        "salmon" => (250, 128, 114),
        "sandybrown" => (244, 164, 96),
        "seagreen" => (46, 139, 87),
        "seashell" => (255, 245, 238),
        "sienna" => (160, 82, 45),
        "silver" => (192, 192, 192),
        "skyblue" => (135, 206, 235),
        "slateblue" => (106, 90, 205),
        "slategray" | "slategrey" => (112, 128, 144),
        "snow" => (255, 250, 250),
        "springgreen" => (0, 255, 127),
        "steelblue" => (70, 130, 180),
        "tan" => (210, 180, 140),
        "teal" => (0, 128, 128),
        "thistle" => (216, 191, 216),
        "tomato" => (255, 99, 71),
        "turquoise" => (64, 224, 208),
        "violet" => (238, 130, 238),
        "wheat" => (245, 222, 179),
        "white" => (255, 255, 255),
        "whitesmoke" => (245, 245, 245),
        "yellow" => (255, 255, 0),
        "yellowgreen" => (154, 205, 50),
        // Couleurs système utilisées par la feuille de style par défaut.
        "canvas" => (255, 255, 255),
        "canvastext" => (0, 0, 0),
        "linktext" => (0, 0, 238),
        "visitedtext" => (85, 26, 139),
        "graytext" => (128, 128, 128),
        _ => return None,
    };
    Some(Color::rgb(r, g, b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::tokenize;

    fn color(s: &str) -> Option<Color> {
        match parse_color(&tokenize(s))? {
            (ParsedColor::Color(c), _) => Some(c),
            (ParsedColor::CurrentColor, _) => None,
        }
    }

    #[test]
    fn colors() {
        assert_eq!(color("#0E1116"), Some(Color::rgb(0x0e, 0x11, 0x16)));
        assert_eq!(color("#fff"), Some(Color::WHITE));
        assert_eq!(
            color("#0008"),
            Some(Color {
                r: 0,
                g: 0,
                b: 0,
                a: 0x88
            })
        );
        assert_eq!(color("rgb(76, 201, 168)"), Some(Color::rgb(76, 201, 168)));
        assert_eq!(
            color("rgb(100% 0% 0% / 50%)"),
            Some(Color {
                r: 255,
                g: 0,
                b: 0,
                a: 128
            })
        );
        assert_eq!(
            color("rgba(0,0,0,0.45)"),
            Some(Color {
                r: 0,
                g: 0,
                b: 0,
                a: 115
            })
        );
        assert_eq!(color("hsl(120, 100%, 25%)"), Some(Color::rgb(0, 128, 0)));
        assert_eq!(color("RebeccaPurple"), Some(Color::rgb(102, 51, 153)));
        assert_eq!(color("transparent"), Some(Color::TRANSPARENT));
        assert_eq!(color("#ggg"), None);
        assert_eq!(color("rgb(1,2)"), None);
        assert_eq!(color("rgb(999, -5, 1e9)"), Some(Color::rgb(255, 0, 255)));
    }

    #[test]
    fn lengths() {
        let len = |s: &str| parse_length(&tokenize(s)[0]);
        assert_eq!(len("12px"), Some(Length::Px(12.0)));
        assert_eq!(len("0"), Some(Length::Px(0.0)));
        assert_eq!(len("5"), None);
        assert_eq!(len("1.5em"), Some(Length::Em(1.5)));
        assert_eq!(len("50%"), Some(Length::Percent(50.0)));
        assert_eq!(len("72pt"), Some(Length::Px(96.0)));
        assert_eq!(len("3parsec"), None);
    }
}
