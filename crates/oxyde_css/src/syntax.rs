//! Tokenizer CSS (CSS Syntax Module Level 3, §4) et utilitaires sur les jetons.
//!
//! LIMITES CONNUES : `unicode-range`, `<!--` / `-->` et les URL non
//! entre guillemets contenant des échappements ne sont pas traités.

/// Jeton CSS.
#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    /// Identifiant.
    Ident(String),
    /// `nom(` — le nom est en minuscules.
    Function(String),
    /// `@nom`
    AtKeyword(String),
    /// `#nom`
    Hash(String),
    /// Chaîne entre guillemets.
    Str(String),
    /// `url(...)` non quotée.
    Url(String),
    /// Nombre.
    Number(f32),
    /// Pourcentage (valeur avant le `%`).
    Percentage(f32),
    /// Nombre avec unité (unité en minuscules).
    Dimension(f32, String),
    /// Un ou plusieurs blancs / commentaires.
    Whitespace,
    /// `:`
    Colon,
    /// `;`
    Semicolon,
    /// `,`
    Comma,
    /// `{`
    OpenBrace,
    /// `}`
    CloseBrace,
    /// `(`
    OpenParen,
    /// `)`
    CloseParen,
    /// `[`
    OpenBracket,
    /// `]`
    CloseBracket,
    /// Tout autre caractère.
    Delim(char),
}

fn is_name_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || !c.is_ascii()
}

fn is_name(c: char) -> bool {
    is_name_start(c) || c.is_ascii_digit() || c == '-'
}

struct Lexer<'a> {
    chars: &'a [char],
    pos: usize,
}

impl Lexer<'_> {
    fn peek(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    fn valid_escape(&self, offset: usize) -> bool {
        self.peek(offset) == Some('\\') && self.peek(offset + 1).is_some_and(|c| c != '\n')
    }

    fn starts_ident(&self) -> bool {
        match self.peek(0) {
            Some('-') => {
                self.peek(1).is_some_and(|c| is_name_start(c) || c == '-') || self.valid_escape(1)
            }
            Some('\\') => self.valid_escape(0),
            Some(c) => is_name_start(c),
            None => false,
        }
    }

    fn starts_number(&self) -> bool {
        match self.peek(0) {
            Some('+' | '-') => match self.peek(1) {
                Some(c) if c.is_ascii_digit() => true,
                Some('.') => self.peek(2).is_some_and(|c| c.is_ascii_digit()),
                _ => false,
            },
            Some('.') => self.peek(1).is_some_and(|c| c.is_ascii_digit()),
            Some(c) => c.is_ascii_digit(),
            None => false,
        }
    }

    fn consume_escape(&mut self) -> char {
        // Le `\` est déjà consommé.
        let mut value: u32 = 0;
        let mut digits = 0;
        while digits < 6
            && let Some(d) = self.peek(0).and_then(|c| c.to_digit(16))
        {
            value = value * 16 + d;
            digits += 1;
            self.pos += 1;
        }
        if digits == 0 {
            return match self.peek(0) {
                Some(c) => {
                    self.pos += 1;
                    c
                }
                None => '\u{fffd}',
            };
        }
        if self.peek(0).is_some_and(|c| c.is_ascii_whitespace()) {
            self.pos += 1;
        }
        match char::from_u32(value) {
            Some(c) if value != 0 => c,
            _ => '\u{fffd}',
        }
    }

    fn consume_name(&mut self) -> String {
        let mut out = String::new();
        loop {
            match self.peek(0) {
                Some(c) if is_name(c) => {
                    self.pos += 1;
                    out.push(c);
                }
                Some('\\') if self.valid_escape(0) => {
                    self.pos += 1;
                    out.push(self.consume_escape());
                }
                _ => return out,
            }
        }
    }

    fn consume_number(&mut self) -> f32 {
        let start = self.pos;
        if matches!(self.peek(0), Some('+' | '-')) {
            self.pos += 1;
        }
        while self.peek(0).is_some_and(|c| c.is_ascii_digit()) {
            self.pos += 1;
        }
        if self.peek(0) == Some('.') && self.peek(1).is_some_and(|c| c.is_ascii_digit()) {
            self.pos += 1;
            while self.peek(0).is_some_and(|c| c.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        if matches!(self.peek(0), Some('e' | 'E')) {
            let sign = usize::from(matches!(self.peek(1), Some('+' | '-')));
            if self.peek(1 + sign).is_some_and(|c| c.is_ascii_digit()) {
                self.pos += 1 + sign;
                while self.peek(0).is_some_and(|c| c.is_ascii_digit()) {
                    self.pos += 1;
                }
            }
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        text.parse().unwrap_or(0.0)
    }

    fn consume_string(&mut self, quote: char) -> Token {
        let mut out = String::new();
        loop {
            match self.peek(0) {
                None => return Token::Str(out),
                Some(c) if c == quote => {
                    self.pos += 1;
                    return Token::Str(out);
                }
                // Chaîne non terminée : jeton « bad-string », rendu comme délimiteur neutre.
                Some('\n') => return Token::Delim('\u{fffd}'),
                Some('\\') => {
                    self.pos += 1;
                    match self.peek(0) {
                        None => {}
                        Some('\n') => self.pos += 1,
                        Some(_) => out.push(self.consume_escape()),
                    }
                }
                Some(c) => {
                    self.pos += 1;
                    out.push(c);
                }
            }
        }
    }

    fn skip_ws(&mut self) {
        while self.peek(0).is_some_and(|c| c.is_ascii_whitespace()) {
            self.pos += 1;
        }
    }

    fn consume_ident_like(&mut self) -> Token {
        let name = self.consume_name();
        if self.peek(0) != Some('(') {
            return Token::Ident(name);
        }
        self.pos += 1;
        let lower = name.to_ascii_lowercase();
        if lower != "url" {
            return Token::Function(lower);
        }
        let save = self.pos;
        self.skip_ws();
        if matches!(self.peek(0), Some('"' | '\'')) {
            self.pos = save;
            return Token::Function(lower);
        }
        let mut url = String::new();
        loop {
            match self.peek(0) {
                None => return Token::Url(url),
                Some(')') => {
                    self.pos += 1;
                    return Token::Url(url);
                }
                Some(c) if c.is_ascii_whitespace() => {
                    self.skip_ws();
                }
                Some('\\') if self.valid_escape(0) => {
                    self.pos += 1;
                    url.push(self.consume_escape());
                }
                Some(c) => {
                    self.pos += 1;
                    url.push(c);
                }
            }
        }
    }

    fn next(&mut self) -> Option<Token> {
        let c = self.peek(0)?;
        // Commentaires.
        if c == '/' && self.peek(1) == Some('*') {
            self.pos += 2;
            while self.pos < self.chars.len()
                && !(self.peek(0) == Some('*') && self.peek(1) == Some('/'))
            {
                self.pos += 1;
            }
            self.pos = (self.pos + 2).min(self.chars.len());
            return Some(Token::Whitespace);
        }
        if c.is_ascii_whitespace() {
            self.skip_ws();
            return Some(Token::Whitespace);
        }
        if self.starts_number() {
            let value = self.consume_number();
            if self.starts_ident() {
                let unit = self.consume_name().to_ascii_lowercase();
                return Some(Token::Dimension(value, unit));
            }
            if self.peek(0) == Some('%') {
                self.pos += 1;
                return Some(Token::Percentage(value));
            }
            return Some(Token::Number(value));
        }
        if self.starts_ident() {
            return Some(self.consume_ident_like());
        }
        self.pos += 1;
        Some(match c {
            '"' | '\'' => self.consume_string(c),
            '#' => {
                if self.peek(0).is_some_and(is_name) || self.valid_escape(0) {
                    Token::Hash(self.consume_name())
                } else {
                    Token::Delim('#')
                }
            }
            '@' => {
                if self.starts_ident() {
                    Token::AtKeyword(self.consume_name().to_ascii_lowercase())
                } else {
                    Token::Delim('@')
                }
            }
            ':' => Token::Colon,
            ';' => Token::Semicolon,
            ',' => Token::Comma,
            '{' => Token::OpenBrace,
            '}' => Token::CloseBrace,
            '(' => Token::OpenParen,
            ')' => Token::CloseParen,
            '[' => Token::OpenBracket,
            ']' => Token::CloseBracket,
            other => Token::Delim(other),
        })
    }
}

/// Découpe `source` en jetons ; les blancs consécutifs sont fusionnés.
#[must_use]
pub fn tokenize(source: &str) -> Vec<Token> {
    let chars: Vec<char> = source.chars().map(|c| if c == '\0' { '\u{fffd}' } else { c }).collect();
    let mut lexer = Lexer { chars: &chars, pos: 0 };
    let mut out = Vec::new();
    while let Some(token) = lexer.next() {
        if token == Token::Whitespace && out.last() == Some(&Token::Whitespace) {
            continue;
        }
        out.push(token);
    }
    out
}

/// Index du jeton fermant correspondant au jeton ouvrant situé en `open`
/// (ou `tokens.len()` si le bloc n'est pas fermé).
#[must_use]
pub fn matching_close(tokens: &[Token], open: usize) -> usize {
    let mut depth = 0usize;
    for (i, t) in tokens.iter().enumerate().skip(open) {
        match t {
            Token::OpenBrace | Token::OpenParen | Token::OpenBracket | Token::Function(_) => {
                depth += 1;
            }
            Token::CloseBrace | Token::CloseParen | Token::CloseBracket => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return i;
                }
            }
            _ => {}
        }
    }
    tokens.len()
}

/// Découpe `tokens` sur le séparateur `sep` au niveau d'imbrication zéro.
#[must_use]
pub fn split_top_level<'a>(tokens: &'a [Token], sep: &Token) -> Vec<&'a [Token]> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            Token::OpenBrace | Token::OpenParen | Token::OpenBracket | Token::Function(_) => {
                i = matching_close(tokens, i);
            }
            t if t == sep => {
                parts.push(&tokens[start..i]);
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    parts.push(&tokens[start.min(tokens.len())..]);
    parts
}

/// Retire les blancs en tête et en queue.
#[must_use]
pub fn trim(tokens: &[Token]) -> &[Token] {
    let start = tokens.iter().position(|t| *t != Token::Whitespace).unwrap_or(tokens.len());
    let end = tokens.iter().rposition(|t| *t != Token::Whitespace).map_or(start, |i| i + 1);
    &tokens[start..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_tokens() {
        assert_eq!(
            tokenize("a.b#c{color:#fff;margin:-1.5em 10% 0}"),
            vec![
                Token::Ident("a".into()),
                Token::Delim('.'),
                Token::Ident("b".into()),
                Token::Hash("c".into()),
                Token::OpenBrace,
                Token::Ident("color".into()),
                Token::Colon,
                Token::Hash("fff".into()),
                Token::Semicolon,
                Token::Ident("margin".into()),
                Token::Colon,
                Token::Dimension(-1.5, "em".into()),
                Token::Whitespace,
                Token::Percentage(10.0),
                Token::Whitespace,
                Token::Number(0.0),
                Token::CloseBrace,
            ]
        );
    }

    #[test]
    fn strings_urls_comments_escapes() {
        assert_eq!(
            tokenize("/* c */ 'a\\'b' url( x.png ) url(\"y\") \\31 a @media"),
            vec![
                Token::Whitespace,
                Token::Str("a'b".into()),
                Token::Whitespace,
                Token::Url("x.png".into()),
                Token::Whitespace,
                Token::Function("url".into()),
                Token::Str("y".into()),
                Token::CloseParen,
                Token::Whitespace,
                Token::Ident("1a".into()),
                Token::Whitespace,
                Token::AtKeyword("media".into()),
            ]
        );
    }

    #[test]
    fn nesting_helpers() {
        let t = tokenize("a(b,c),d{e;f};g");
        let parts = split_top_level(&t, &Token::Comma);
        assert_eq!(parts.len(), 2);
        let parts = split_top_level(&t, &Token::Semicolon);
        assert_eq!(parts.len(), 2);
        assert_eq!(matching_close(&t, 0), 4);
    }

    #[test]
    fn never_panics_on_truncated_input() {
        let src = "@media (min-width: 10px) { a[b='c\\\n'] > d::e { f: url(g) 1e3px -x \\";
        let chars: Vec<char> = src.chars().collect();
        for n in 0..=chars.len() {
            let prefix: String = chars[..n].iter().collect();
            let t = tokenize(&prefix);
            let _ = split_top_level(&t, &Token::Comma);
        }
    }
}
