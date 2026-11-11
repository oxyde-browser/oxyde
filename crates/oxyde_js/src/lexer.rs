//! Analyse lexicale d'ECMAScript (ECMA-262 §12).
//!
//! Le lexer est piloté par l'analyseur syntaxique : c'est lui qui sait si un
//! `/` ouvre une expression rationnelle et quand reprendre un gabarit après
//! une substitution `${…}`.
//!
//! LIMITES CONNUES : pas d'échappements Unicode dans les identifiants, pas de
//! littéraux `BigInt`, pas de commentaires HTML (`<!--`).

/// Erreur de syntaxe, avec sa position dans le source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyntaxError {
    /// Description.
    pub message: String,
    /// Ligne (à partir de 1).
    pub line: u32,
    /// Colonne (à partir de 1).
    pub column: u32,
}

impl std::fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} (ligne {}, colonne {})",
            self.message, self.line, self.column
        )
    }
}

impl std::error::Error for SyntaxError {}

/// Unité lexicale.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Tok {
    /// Identifiant ou mot-clé.
    Ident(String),
    /// Nom privé (`#x`).
    Private(String),
    Num(f64),
    Str(String),
    /// Morceau de gabarit ; `tail` est vrai pour le dernier.
    Template {
        cooked: String,
        tail: bool,
    },
    Regex {
        pattern: String,
        flags: String,
    },
    Punct(&'static str),
    Eof,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Token {
    pub(crate) tok: Tok,
    /// Index du premier caractère dans le source.
    pub(crate) start: usize,
    /// Vrai si un saut de ligne précède le jeton (insertion automatique de `;`).
    pub(crate) newline_before: bool,
}

/// Ponctuations, de la plus longue à la plus courte.
const PUNCTUATORS: &[&str] = &[
    ">>>=", "...", "===", "!==", "**=", "<<=", ">>=", ">>>", "&&=", "||=", "??=", "=>", "==", "!=",
    "<=", ">=", "&&", "||", "??", "?.", "++", "--", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=",
    "<<", ">>", "**", "{", "}", "(", ")", "[", "]", ";", ",", "<", ">", "+", "-", "*", "/", "%",
    "&", "|", "^", "!", "~", "?", ":", "=", ".", "@",
];

fn is_id_start(c: char) -> bool {
    c == '$' || c == '_' || c.is_alphabetic()
}

fn is_id_part(c: char) -> bool {
    is_id_start(c) || c.is_numeric() || c == '\u{200c}' || c == '\u{200d}'
}

fn is_line_terminator(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

#[derive(Clone)]
pub(crate) struct Lexer {
    src: std::sync::Arc<[char]>,
    pub(crate) pos: usize,
}

impl Lexer {
    pub(crate) fn new(source: &str) -> Self {
        Self {
            src: source.chars().collect::<Vec<char>>().into(),
            pos: 0,
        }
    }

    pub(crate) fn error_at(&self, index: usize, message: impl Into<String>) -> SyntaxError {
        let mut line = 1u32;
        let mut column = 1u32;
        for c in self.src.iter().take(index) {
            if *c == '\n' {
                line = line.saturating_add(1);
                column = 1;
            } else {
                column = column.saturating_add(1);
            }
        }
        SyntaxError {
            message: message.into(),
            line,
            column,
        }
    }

    fn peek(&self, offset: usize) -> Option<char> {
        self.src.get(self.pos + offset).copied()
    }

    /// Saute blancs et commentaires ; renvoie vrai si un saut de ligne a été franchi.
    fn skip_trivia(&mut self) -> Result<bool, SyntaxError> {
        let mut newline = false;
        loop {
            match self.peek(0) {
                Some(c) if is_line_terminator(c) => {
                    newline = true;
                    self.pos += 1;
                }
                Some(c) if c.is_whitespace() || c == '\u{feff}' => self.pos += 1,
                Some('/') if self.peek(1) == Some('/') => {
                    while self.peek(0).is_some_and(|c| !is_line_terminator(c)) {
                        self.pos += 1;
                    }
                }
                Some('/') if self.peek(1) == Some('*') => {
                    let start = self.pos;
                    self.pos += 2;
                    loop {
                        match self.peek(0) {
                            None => return Err(self.error_at(start, "commentaire non terminé")),
                            Some('*') if self.peek(1) == Some('/') => {
                                self.pos += 2;
                                break;
                            }
                            Some(c) => {
                                newline |= is_line_terminator(c);
                                self.pos += 1;
                            }
                        }
                    }
                }
                _ => return Ok(newline),
            }
        }
    }

    /// Jeton suivant. Un `/` est toujours rendu comme ponctuation ; voir [`Lexer::regex_at`].
    pub(crate) fn next(&mut self) -> Result<Token, SyntaxError> {
        let newline_before = self.skip_trivia()?;
        let start = self.pos;
        let tok = match self.peek(0) {
            None => Tok::Eof,
            Some(c) if is_id_start(c) => Tok::Ident(self.identifier()),
            Some('#') if self.peek(1).is_some_and(is_id_start) => {
                self.pos += 1;
                Tok::Private(self.identifier())
            }
            Some(c) if c.is_ascii_digit() => self.number()?,
            Some('.') if self.peek(1).is_some_and(|c| c.is_ascii_digit()) => self.number()?,
            Some(q @ ('"' | '\'')) => {
                self.pos += 1;
                Tok::Str(self.string(q, start)?)
            }
            Some('`') => {
                self.pos += 1;
                self.template(start)?
            }
            Some(_) => self.punctuator(start)?,
        };
        Ok(Token {
            tok,
            start,
            newline_before,
        })
    }

    fn identifier(&mut self) -> String {
        let start = self.pos;
        while self.peek(0).is_some_and(is_id_part) {
            self.pos += 1;
        }
        self.src[start..self.pos].iter().collect()
    }

    fn punctuator(&mut self, start: usize) -> Result<Tok, SyntaxError> {
        for p in PUNCTUATORS {
            let matches = p.chars().enumerate().all(|(i, c)| self.peek(i) == Some(c));
            if !matches {
                continue;
            }
            // `a?.5:b` : `?.` suivi d'un chiffre est un `?` puis un nombre.
            if *p == "?." && self.peek(2).is_some_and(|c| c.is_ascii_digit()) {
                continue;
            }
            self.pos += p.chars().count();
            return Ok(Tok::Punct(p));
        }
        Err(self.error_at(
            start,
            format!("caractère inattendu « {} »", self.src[start]),
        ))
    }

    fn digits(&mut self, radix: u32) -> String {
        let mut out = String::new();
        while let Some(c) = self.peek(0) {
            if c.is_digit(radix) {
                out.push(c);
            } else if c != '_' {
                break;
            }
            self.pos += 1;
        }
        out
    }

    fn number(&mut self) -> Result<Tok, SyntaxError> {
        let start = self.pos;
        let radix = match (self.peek(0), self.peek(1)) {
            (Some('0'), Some('x' | 'X')) => Some(16),
            (Some('0'), Some('o' | 'O')) => Some(8),
            (Some('0'), Some('b' | 'B')) => Some(2),
            _ => None,
        };
        let value = if let Some(radix) = radix {
            self.pos += 2;
            let digits = self.digits(radix);
            if digits.is_empty() {
                return Err(self.error_at(start, "littéral numérique invalide"));
            }
            // Accumulation en flottant : exacte jusqu'à 2^53, arrondie au-delà comme en JS.
            digits
                .chars()
                .filter_map(|c| c.to_digit(radix))
                .fold(0.0_f64, |acc, d| acc * f64::from(radix) + f64::from(d))
        } else {
            let mut text = self.digits(10);
            if self.peek(0) == Some('.') {
                self.pos += 1;
                text.push('.');
                text.push_str(&self.digits(10));
            }
            if matches!(self.peek(0), Some('e' | 'E')) {
                let sign = usize::from(matches!(self.peek(1), Some('+' | '-')));
                if self.peek(1 + sign).is_some_and(|c| c.is_ascii_digit()) {
                    text.push('e');
                    if sign == 1 {
                        text.push(self.src[self.pos + 1]);
                    }
                    self.pos += 1 + sign;
                    text.push_str(&self.digits(10));
                }
            }
            if text.starts_with('.') {
                text.insert(0, '0');
            }
            if text.ends_with('.') {
                text.push('0');
            }
            text.parse::<f64>()
                .map_err(|_| self.error_at(start, "littéral numérique invalide"))?
        };
        match self.peek(0) {
            Some('n') => {
                Err(self.error_at(start, "les littéraux BigInt ne sont pas pris en charge"))
            }
            Some(c) if is_id_start(c) || c.is_ascii_digit() => {
                Err(self.error_at(self.pos, "identifiant collé à un nombre"))
            }
            _ => Ok(Tok::Num(value)),
        }
    }

    fn hex(&mut self, count: usize) -> Option<u32> {
        let mut value = 0u32;
        for _ in 0..count {
            value = value * 16 + self.peek(0)?.to_digit(16)?;
            self.pos += 1;
        }
        Some(value)
    }

    /// Lit une séquence d'échappement (le `\` est consommé) et l'ajoute à `out`.
    fn escape(&mut self, out: &mut String, start: usize) -> Result<(), SyntaxError> {
        let bad = |lexer: &Self| lexer.error_at(start, "séquence d'échappement invalide");
        let Some(c) = self.peek(0) else {
            return Err(bad(self));
        };
        self.pos += 1;
        match c {
            'n' => out.push('\n'),
            't' => out.push('\t'),
            'r' => out.push('\r'),
            'b' => out.push('\u{8}'),
            'f' => out.push('\u{c}'),
            'v' => out.push('\u{b}'),
            '0' if !self.peek(0).is_some_and(|c| c.is_ascii_digit()) => out.push('\0'),
            'x' => {
                let code = self.hex(2).ok_or_else(|| bad(self))?;
                out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
            }
            'u' => {
                let unit = self.unicode_escape().ok_or_else(|| bad(self))?;
                // Paire de substituts écrite en deux échappements successifs.
                if (0xD800..0xDC00).contains(&unit)
                    && self.peek(0) == Some('\\')
                    && self.peek(1) == Some('u')
                {
                    let save = self.pos;
                    self.pos += 2;
                    match self.unicode_escape() {
                        Some(low) if (0xDC00..0xE000).contains(&low) => {
                            let code = 0x10000 + ((unit - 0xD800) << 10) + (low - 0xDC00);
                            out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                            return Ok(());
                        }
                        _ => self.pos = save,
                    }
                }
                // LIMITE : un substitut isolé n'est pas représentable (chaînes en UTF-8).
                out.push(char::from_u32(unit).unwrap_or('\u{fffd}'));
            }
            '\r' => {
                if self.peek(0) == Some('\n') {
                    self.pos += 1;
                }
            }
            c if is_line_terminator(c) => {}
            other => out.push(other),
        }
        Ok(())
    }

    fn unicode_escape(&mut self) -> Option<u32> {
        if self.peek(0) == Some('{') {
            self.pos += 1;
            let mut value = 0u32;
            let mut digits = 0;
            while let Some(d) = self.peek(0).and_then(|c| c.to_digit(16)) {
                value = value.checked_mul(16)?.checked_add(d)?;
                digits += 1;
                self.pos += 1;
            }
            if digits == 0 || self.peek(0) != Some('}') || value > 0x10_FFFF {
                return None;
            }
            self.pos += 1;
            Some(value)
        } else {
            self.hex(4)
        }
    }

    fn string(&mut self, quote: char, start: usize) -> Result<String, SyntaxError> {
        let mut out = String::new();
        loop {
            match self.peek(0) {
                None => return Err(self.error_at(start, "chaîne non terminée")),
                Some(c) if c == quote => {
                    self.pos += 1;
                    return Ok(out);
                }
                Some('\n' | '\r') => return Err(self.error_at(start, "chaîne non terminée")),
                Some('\\') => {
                    self.pos += 1;
                    self.escape(&mut out, start)?;
                }
                Some(c) => {
                    self.pos += 1;
                    out.push(c);
                }
            }
        }
    }

    /// Lit un morceau de gabarit jusqu'à `${` ou la fermeture.
    fn template(&mut self, start: usize) -> Result<Tok, SyntaxError> {
        let mut cooked = String::new();
        loop {
            match self.peek(0) {
                None => return Err(self.error_at(start, "gabarit non terminé")),
                Some('`') => {
                    self.pos += 1;
                    return Ok(Tok::Template { cooked, tail: true });
                }
                Some('$') if self.peek(1) == Some('{') => {
                    self.pos += 2;
                    return Ok(Tok::Template {
                        cooked,
                        tail: false,
                    });
                }
                Some('\\') => {
                    self.pos += 1;
                    self.escape(&mut cooked, start)?;
                }
                Some('\r') => {
                    // Les fins de ligne d'un gabarit sont normalisées en LF.
                    self.pos += 1;
                    if self.peek(0) == Some('\n') {
                        self.pos += 1;
                    }
                    cooked.push('\n');
                }
                Some(c) => {
                    self.pos += 1;
                    cooked.push(c);
                }
            }
        }
    }

    /// Reprend la lecture d'un gabarit juste après le `}` d'une substitution.
    pub(crate) fn template_continue(&mut self) -> Result<Token, SyntaxError> {
        let start = self.pos;
        let tok = self.template(start)?;
        Ok(Token {
            tok,
            start,
            newline_before: false,
        })
    }

    /// Relit comme expression rationnelle le jeton `/` ou `/=` commençant en `start`.
    pub(crate) fn regex_at(&mut self, start: usize) -> Result<Token, SyntaxError> {
        self.pos = start + 1;
        let mut pattern = String::new();
        let mut in_class = false;
        loop {
            match self.peek(0) {
                None => return Err(self.error_at(start, "expression rationnelle non terminée")),
                Some(c) if is_line_terminator(c) => {
                    return Err(self.error_at(start, "expression rationnelle non terminée"));
                }
                Some('\\') => {
                    pattern.push('\\');
                    self.pos += 1;
                    if let Some(c) = self.peek(0).filter(|c| !is_line_terminator(*c)) {
                        pattern.push(c);
                        self.pos += 1;
                    }
                }
                Some('/') if !in_class => {
                    self.pos += 1;
                    break;
                }
                Some(c) => {
                    in_class = match c {
                        '[' => true,
                        ']' => false,
                        _ => in_class,
                    };
                    pattern.push(c);
                    self.pos += 1;
                }
            }
        }
        let flags = self.identifier();
        Ok(Token {
            tok: Tok::Regex { pattern, flags },
            start,
            newline_before: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(src: &str) -> Vec<Tok> {
        let mut lexer = Lexer::new(src);
        let mut out = Vec::new();
        loop {
            let t = lexer.next().expect("jeton");
            if t.tok == Tok::Eof {
                return out;
            }
            out.push(t.tok);
        }
    }

    fn id(s: &str) -> Tok {
        Tok::Ident(s.to_owned())
    }

    #[test]
    fn identifiers_numbers_punctuators() {
        assert_eq!(
            toks("let $a_1 = 0x1F + .5e1 - 1_000 >>>= x?.y ?? z?.5:1"),
            vec![
                id("let"),
                id("$a_1"),
                Tok::Punct("="),
                Tok::Num(31.0),
                Tok::Punct("+"),
                Tok::Num(5.0),
                Tok::Punct("-"),
                Tok::Num(1000.0),
                Tok::Punct(">>>="),
                id("x"),
                Tok::Punct("?."),
                id("y"),
                Tok::Punct("??"),
                id("z"),
                Tok::Punct("?"),
                Tok::Num(0.5),
                Tok::Punct(":"),
                Tok::Num(1.0),
            ]
        );
        assert_eq!(
            toks("0b101 0o17 1e3 12.5"),
            vec![
                Tok::Num(5.0),
                Tok::Num(15.0),
                Tok::Num(1000.0),
                Tok::Num(12.5)
            ]
        );
    }

    #[test]
    fn strings_and_escapes() {
        assert_eq!(
            toks(
                r#"'a\n\x41é\u{1F600}' "b\
c" '😀'"#
            ),
            vec![
                Tok::Str("a\nAé😀".into()),
                Tok::Str("bc".into()),
                Tok::Str("😀".into())
            ]
        );
    }

    #[test]
    fn comments_and_newlines() {
        let mut lexer = Lexer::new("a // c\n/* d\n */ b");
        assert!(!lexer.next().expect("a").newline_before);
        assert!(lexer.next().expect("b").newline_before);
    }

    #[test]
    fn templates_and_regex() {
        let mut lexer = Lexer::new("`a${x}b` /[/]\\//gi");
        assert_eq!(
            lexer.next().expect("tête").tok,
            Tok::Template {
                cooked: "a".into(),
                tail: false
            }
        );
        assert_eq!(lexer.next().expect("x").tok, id("x"));
        assert_eq!(lexer.next().expect("}").tok, Tok::Punct("}"));
        assert_eq!(
            lexer.template_continue().expect("queue").tok,
            Tok::Template {
                cooked: "b".into(),
                tail: true
            }
        );
        let slash = lexer.next().expect("/");
        assert_eq!(slash.tok, Tok::Punct("/"));
        assert_eq!(
            lexer.regex_at(slash.start).expect("regex").tok,
            Tok::Regex {
                pattern: "[/]\\/".into(),
                flags: "gi".into()
            }
        );
    }

    #[test]
    fn errors_have_positions() {
        let err = Lexer::new("a\n  'x")
            .next()
            .and_then(|_| Lexer::new("a\n  'x").skip_then_next())
            .expect_err("erreur");
        assert_eq!((err.line, err.column), (2, 3));
        assert!(Lexer::new("1n").next().is_err());
        assert!(Lexer::new("/* x").next().is_err());
        assert!(Lexer::new("3in").next().is_err());
    }

    impl Lexer {
        fn skip_then_next(mut self) -> Result<Token, SyntaxError> {
            self.next()?;
            self.next()
        }
    }

    #[test]
    fn never_panics_on_truncated_input() {
        let src = "let a = `x${1+'\\u{1F600}'}y` /re[/]/g; /* c */ 0x1F .5e-3 '\\x4";
        let chars: Vec<char> = src.chars().collect();
        for n in 0..=chars.len() {
            let prefix: String = chars[..n].iter().collect();
            let mut lexer = Lexer::new(&prefix);
            for _ in 0..200 {
                match lexer.next() {
                    Ok(t) if t.tok == Tok::Eof => break,
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
        }
    }
}
