//! Feuilles de style : règles, `@media`, analyse d'une feuille complète.
//!
//! LIMITES CONNUES : `@import`, `@font-face`, `@supports`, `@keyframes`,
//! `@container` et l'imbrication de règles sont ignorés ; `@layer { … }` est
//! aplati (l'ordre des couches n'est pas appliqué).

use crate::properties::{Declaration, parse_declarations};
use crate::selector::{Selector, parse_selector_list};
use crate::syntax::{Token, matching_close, split_top_level, tokenize, trim};
use crate::values::{Length, parse_length};

/// Dimensions de la fenêtre d'affichage, en pixels CSS.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    /// Largeur.
    pub width: f32,
    /// Hauteur.
    pub height: f32,
}

#[derive(Clone, Debug, PartialEq)]
enum Feature {
    MinWidth(f32),
    MaxWidth(f32),
    MinHeight(f32),
    MaxHeight(f32),
    Orientation {
        portrait: bool,
    },
    /// Caractéristique constante (vraie ou fausse quel que soit l'affichage).
    Const(bool),
}

/// Une requête média (`screen and (min-width: 600px)`).
#[derive(Clone, Debug, PartialEq)]
pub struct MediaQuery {
    negated: bool,
    type_matches: bool,
    features: Vec<Feature>,
}

/// Liste de requêtes média séparées par des virgules (au moins une doit correspondre).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MediaList(Vec<MediaQuery>);

impl MediaList {
    /// Vrai si la liste s'applique à `viewport`.
    #[must_use]
    pub fn matches(&self, viewport: Viewport) -> bool {
        self.0.is_empty()
            || self.0.iter().any(|q| {
                let features = q.features.iter().all(|f| match f {
                    Feature::MinWidth(v) => viewport.width >= *v,
                    Feature::MaxWidth(v) => viewport.width <= *v,
                    Feature::MinHeight(v) => viewport.height >= *v,
                    Feature::MaxHeight(v) => viewport.height <= *v,
                    Feature::Orientation { portrait } => {
                        (viewport.height >= viewport.width) == *portrait
                    }
                    Feature::Const(v) => *v,
                });
                (q.type_matches && features) != q.negated
            })
    }
}

fn media_length(tokens: &[Token]) -> Option<f32> {
    let [token] = tokens else { return None };
    match parse_length(token)? {
        Length::Px(v) => Some(v),
        // Dans une requête média, em et rem valent la taille de police initiale.
        Length::Em(v) | Length::Rem(v) => Some(v * 16.0),
        _ => None,
    }
}

fn parse_feature(tokens: &[Token]) -> Feature {
    let tokens = trim(tokens);
    let Some(Token::Ident(name)) = tokens.first() else {
        return Feature::Const(false);
    };
    let name = name.to_ascii_lowercase();
    let rest = trim(&tokens[1..]);
    // Forme `nom: valeur`.
    if let [Token::Colon, value @ ..] = rest {
        let value = trim(value);
        let ident = match value {
            [Token::Ident(v)] => Some(v.to_ascii_lowercase()),
            _ => None,
        };
        let length = media_length(value);
        return match (name.as_str(), length, ident.as_deref()) {
            ("min-width", Some(v), _) => Feature::MinWidth(v),
            ("max-width", Some(v), _) => Feature::MaxWidth(v),
            ("min-height", Some(v), _) => Feature::MinHeight(v),
            ("max-height", Some(v), _) => Feature::MaxHeight(v),
            ("orientation", _, Some("portrait")) => Feature::Orientation { portrait: true },
            ("orientation", _, Some("landscape")) => Feature::Orientation { portrait: false },
            ("prefers-color-scheme", _, Some(v)) => Feature::Const(v == "light"),
            ("prefers-reduced-motion", _, Some(v)) => Feature::Const(v == "no-preference"),
            ("prefers-contrast", _, Some(v)) => Feature::Const(v == "no-preference"),
            ("hover" | "any-hover", _, Some(v)) => Feature::Const(v == "hover"),
            ("pointer" | "any-pointer", _, Some(v)) => Feature::Const(v == "fine"),
            _ => Feature::Const(false),
        };
    }
    // Forme d'intervalle `width >= 600px`.
    if matches!(name.as_str(), "width" | "height") {
        let (op_len, greater, _strict) = match rest {
            [Token::Delim('>'), Token::Delim('='), ..] => (2, true, false),
            [Token::Delim('<'), Token::Delim('='), ..] => (2, false, false),
            [Token::Delim('>'), ..] => (1, true, true),
            [Token::Delim('<'), ..] => (1, false, true),
            _ => return Feature::Const(false),
        };
        let Some(v) = media_length(trim(&rest[op_len..])) else {
            return Feature::Const(false);
        };
        return match (name.as_str(), greater) {
            ("width", true) => Feature::MinWidth(v),
            ("width", false) => Feature::MaxWidth(v),
            (_, true) => Feature::MinHeight(v),
            (_, false) => Feature::MaxHeight(v),
        };
    }
    // Forme booléenne `(color)`.
    Feature::Const(rest.is_empty() && matches!(name.as_str(), "color" | "hover" | "pointer"))
}

/// Analyse le prélude d'une règle `@media` (ou un attribut `media`).
#[must_use]
pub fn parse_media_list(tokens: &[Token]) -> MediaList {
    let tokens = trim(tokens);
    if tokens.is_empty() {
        return MediaList::default();
    }
    let mut queries = Vec::new();
    for part in split_top_level(tokens, &Token::Comma) {
        let part = trim(part);
        let mut query = MediaQuery {
            negated: false,
            type_matches: true,
            features: Vec::new(),
        };
        let mut i = 0;
        while i < part.len() {
            match &part[i] {
                Token::Ident(word) => match word.to_ascii_lowercase().as_str() {
                    "not" => query.negated = true,
                    "only" | "and" => {}
                    "all" | "screen" => query.type_matches = true,
                    _ => query.type_matches = false,
                },
                Token::OpenParen => {
                    let close = matching_close(part, i);
                    query
                        .features
                        .push(parse_feature(part.get(i + 1..close).unwrap_or(&[])));
                    i = close;
                }
                Token::Whitespace => {}
                _ => query.features.push(Feature::Const(false)),
            }
            i += 1;
        }
        queries.push(query);
    }
    MediaList(queries)
}

/// Règle de style.
#[derive(Clone, Debug, PartialEq)]
pub struct Rule {
    /// Sélecteurs (liste séparée par des virgules).
    pub selectors: Vec<Selector>,
    /// Déclarations, propriétés raccourcies développées.
    pub declarations: Vec<Declaration>,
    /// Conditions `@media` englobantes (toutes doivent correspondre).
    pub media: Vec<MediaList>,
}

/// Feuille de style analysée.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Stylesheet {
    /// Règles dans l'ordre du source.
    pub rules: Vec<Rule>,
    /// Polices déclarées par `@font-face`, dans l'ordre du source.
    pub font_faces: Vec<FontFace>,
}

/// Police déclarée par une règle `@font-face`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontFace {
    /// Nom de famille sous lequel la page désigne la police.
    pub family: String,
    /// Adresses des fichiers de police (`src: url(…)`), par ordre de
    /// préférence, telles qu'écrites dans la feuille.
    pub sources: Vec<String>,
}

/// Relève les règles `@font-face` de `tokens`, y compris dans `@media` et
/// `@layer`.
fn collect_font_faces(tokens: &[Token], out: &mut Vec<FontFace>, depth: usize) {
    let mut i = 0;
    while i < tokens.len() {
        let Token::AtKeyword(name) = &tokens[i] else {
            i += 1;
            continue;
        };
        let end = prelude_end(tokens, i + 1);
        if tokens.get(end) != Some(&Token::OpenBrace) {
            i = end + 1;
            continue;
        }
        let close = matching_close(tokens, end);
        let body = tokens.get(end + 1..close).unwrap_or(&[]);
        match name.as_str() {
            "font-face" => out.extend(parse_font_face(body)),
            "media" | "layer" | "supports" if depth < MAX_NESTING => {
                collect_font_faces(body, out, depth + 1);
            }
            _ => {}
        }
        i = close + 1;
    }
}

/// Analyse le corps d'une règle `@font-face` ; `None` s'il manque la
/// famille ou une adresse.
fn parse_font_face(body: &[Token]) -> Option<FontFace> {
    let (mut family, mut sources) = (None, Vec::new());
    for part in crate::syntax::split_top_level(body, &Token::Semicolon) {
        let [Token::Ident(name), rest @ ..] = trim(part) else {
            continue;
        };
        let [Token::Colon, value @ ..] = trim(rest) else {
            continue;
        };
        let value = trim(value);
        match name.to_ascii_lowercase().as_str() {
            "font-family" => {
                family = Some(match value {
                    [Token::Str(quoted)] => quoted.clone(),
                    words => words
                        .iter()
                        .filter_map(|t| match t {
                            Token::Ident(word) => Some(word.as_str()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join(" "),
                });
            }
            "src" => {
                let mut tokens = value.iter().peekable();
                while let Some(token) = tokens.next() {
                    match token {
                        Token::Url(url) => sources.push(url.clone()),
                        Token::Function(function) if function == "url" => {
                            if let Some(Token::Str(url)) = tokens.peek() {
                                sources.push(url.clone());
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    let family = family.filter(|f| !f.is_empty())?;
    (!sources.is_empty()).then_some(FontFace { family, sources })
}

/// Profondeur maximale d'imbrication des règles `@` (borne la récursion).
const MAX_NESTING: usize = 16;

/// Analyse une feuille de style. Ne peut pas échouer : le CSS invalide est
/// écarté règle par règle, comme le prescrit le standard.
#[must_use]
pub fn parse_stylesheet(source: &str) -> Stylesheet {
    let tokens = tokenize(source);
    let mut sheet = Stylesheet::default();
    parse_rules(&tokens, &[], &mut sheet.rules, 0);
    collect_font_faces(&tokens, &mut sheet.font_faces, 0);
    sheet
}

/// Index du premier `{` ou `;` de niveau zéro à partir de `start`.
fn prelude_end(tokens: &[Token], start: usize) -> usize {
    let mut i = start;
    while i < tokens.len() {
        match &tokens[i] {
            Token::OpenBrace | Token::Semicolon => return i,
            Token::OpenParen | Token::OpenBracket | Token::Function(_) => {
                i = matching_close(tokens, i);
            }
            _ => {}
        }
        i += 1;
    }
    tokens.len()
}

fn parse_rules(tokens: &[Token], media: &[MediaList], out: &mut Vec<Rule>, depth: usize) {
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            Token::Whitespace | Token::Semicolon | Token::CloseBrace => i += 1,
            Token::AtKeyword(name) => {
                let end = prelude_end(tokens, i + 1);
                if tokens.get(end) != Some(&Token::OpenBrace) {
                    i = end + 1;
                    continue;
                }
                let close = matching_close(tokens, end);
                let body = tokens.get(end + 1..close).unwrap_or(&[]);
                if depth < MAX_NESTING {
                    match name.as_str() {
                        "media" => {
                            let mut nested = media.to_vec();
                            nested.push(parse_media_list(&tokens[i + 1..end]));
                            parse_rules(body, &nested, out, depth + 1);
                        }
                        "layer" => parse_rules(body, media, out, depth + 1),
                        _ => {}
                    }
                }
                i = close + 1;
            }
            _ => {
                let end = prelude_end(tokens, i);
                if tokens.get(end) != Some(&Token::OpenBrace) {
                    i = end + 1;
                    continue;
                }
                let close = matching_close(tokens, end);
                if let Some(selectors) = parse_selector_list(trim(&tokens[i..end])) {
                    let declarations =
                        parse_declarations(tokens.get(end + 1..close).unwrap_or(&[]));
                    if !declarations.is_empty() {
                        out.push(Rule {
                            selectors,
                            declarations,
                            media: media.to_vec(),
                        });
                    }
                }
                i = close + 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIDE: Viewport = Viewport {
        width: 1440.0,
        height: 900.0,
    };
    const NARROW: Viewport = Viewport {
        width: 390.0,
        height: 844.0,
    };

    #[test]
    fn rules_and_recovery() {
        let sheet = parse_stylesheet(
            "@charset 'utf-8'; @import url(x.css);
             a, b > c { color: red }
             d::before { color: blue }
             e { }
             f { bogus }
             @font-face { font-family: X }
             g { margin: 0",
        );
        assert_eq!(sheet.rules.len(), 2);
        assert_eq!(sheet.rules[0].selectors.len(), 2);
        assert_eq!(sheet.rules[1].declarations.len(), 4);
    }

    #[test]
    fn media_queries() {
        let sheet = parse_stylesheet(
            "@media screen and (min-width: 600px) { a { color: red } }
             @media (max-width: 599px), print { b { color: red } }
             @media not screen { c { color: red } }
             @media (width >= 40em) { @media (prefers-color-scheme: dark) { d { color: red } } }
             @layer base { e { color: red } }",
        );
        let active = |v: Viewport| -> Vec<bool> {
            sheet
                .rules
                .iter()
                .map(|r| r.media.iter().all(|m| m.matches(v)))
                .collect()
        };
        assert_eq!(active(WIDE), [true, false, false, false, true]);
        assert_eq!(active(NARROW), [false, true, false, false, true]);
    }

    #[test]
    fn never_panics_on_truncated_input() {
        let src = "@media (min-width:1px){a[b=\"c\"]:not(d)>e{f:g(h,i) !important;j:k}}@x y;l{m:n}";
        let chars: Vec<char> = src.chars().collect();
        for n in 0..=chars.len() {
            let prefix: String = chars[..n].iter().collect();
            let _ = parse_stylesheet(&prefix);
        }
        let _ = parse_stylesheet(&"@media x{".repeat(100));
    }
}
