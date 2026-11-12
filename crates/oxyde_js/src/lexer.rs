//! Analyse lexicale d'ECMAScript (ECMA-262 §12).
//!
//! Le lexer est piloté par l'analyseur syntaxique : c'est lui qui sait si un
//! `/` ouvre une expression rationnelle et quand reprendre un gabarit après
//! une substitution `${…}`.
//!
//! LIMITES CONNUES : pas de commentaires HTML (`<!--`) ; les débuts et suites
//! d'identifiants Unicode sont reconnus par approximation (pas de table).

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
    /// Littéral `BigInt` (`123n`).
    BigInt(std::sync::Arc<crate::bigint::BigInt>),
    /// Identifiant ou mot-clé.
    Ident(String),
    /// Nom privé (`#x`).
    Private(String),
    Num(f64),
    Str(String),
    /// Morceau de gabarit ; `tail` est vrai pour le dernier. `cooked` vaut
    /// `None` si le morceau contient un échappement invalide, ce qui n'est
    /// permis que dans un gabarit étiqueté ; `raw` est le texte du source.
    Template {
        cooked: Option<String>,
        raw: String,
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
    /// Vrai si l'identifiant est écrit avec un échappement : ce n'est alors
    /// jamais un mot-clé.
    pub(crate) escaped: bool,
    /// Vrai si le jeton contient un littéral ou un échappement octal hérité
    /// (`010`, `"\1"`), interdit en mode strict.
    pub(crate) legacy_octal: bool,
}

/// Ponctuations, de la plus longue à la plus courte.
const PUNCTUATORS: &[&str] = &[
    ">>>=", "...", "===", "!==", "**=", "<<=", ">>=", ">>>", "&&=", "||=", "??=", "=>", "==", "!=",
    "<=", ">=", "&&", "||", "??", "?.", "++", "--", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=",
    "<<", ">>", "**", "{", "}", "(", ")", "[", "]", ";", ",", "<", ">", "+", "-", "*", "/", "%",
    "&", "|", "^", "!", "~", "?", ":", "=", ".", "@",
];

/// Caractères de `ID_Start` et `ID_Continue` absents de `XID_Start` et
/// `XID_Continue` (UAX #31, modifications pour la normalisation NFKC).
fn nfkc_exception(c: char, start: bool) -> bool {
    matches!(
        c,
        '\u{37a}' | '\u{309b}' | '\u{309c}' | '\u{fc5e}'
            ..='\u{fc63}'
                | '\u{fdfa}'
                | '\u{fdfb}'
                | '\u{fe70}'
                | '\u{fe72}'
                | '\u{fe74}'
                | '\u{fe76}'
                | '\u{fe78}'
                | '\u{fe7a}'
                | '\u{fe7c}'
                | '\u{fe7e}'
    ) || (start && matches!(c, '\u{e33}' | '\u{eb3}' | '\u{ff9e}' | '\u{ff9f}'))
}

/// Début d'identifiant (`ID_Start`, plus `$` et `_`).
fn is_id_start(c: char) -> bool {
    c == '$' || c == '_' || unicode_ident::is_xid_start(c) || nfkc_exception(c, true)
}

/// Suite d'identifiant (`ID_Continue`, plus `$`, ZWNJ et ZWJ).
fn is_id_part(c: char) -> bool {
    matches!(c, '$' | '\u{200c}' | '\u{200d}')
        || unicode_ident::is_xid_continue(c)
        || nfkc_exception(c, false)
}

fn is_line_terminator(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

#[derive(Clone)]
pub(crate) struct Lexer {
    src: std::sync::Arc<[char]>,
    pub(crate) pos: usize,
    /// Le jeton en cours de lecture contient un échappement d'identifiant.
    escaped: bool,
    /// Le jeton en cours de lecture contient une forme octale héritée.
    octal: bool,
    /// Les commentaires `<!--` et `-->` sont reconnus (scripts, pas modules).
    pub(crate) html_comments: bool,
    /// Un jeton a déjà été lu : on n'est plus au début du source.
    started: bool,
}

impl Lexer {
    pub(crate) fn new(source: &str) -> Self {
        Self {
            src: source.chars().collect::<Vec<char>>().into(),
            pos: 0,
            escaped: false,
            octal: false,
            html_comments: true,
            started: false,
        }
    }

    /// Le source analysé, partagé.
    pub(crate) fn source(&self) -> std::sync::Arc<[char]> {
        std::sync::Arc::clone(&self.src)
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
        // `-->` n'ouvre un commentaire qu'en début de ligne ou de source.
        let mut newline = false;
        let at_start = !std::mem::replace(&mut self.started, true);
        // Ligne « hashbang » (`#!/usr/bin/env node`), permise en tête de source.
        if self.pos == 0 && self.peek(0) == Some('#') && self.peek(1) == Some('!') {
            while self.peek(0).is_some_and(|c| !is_line_terminator(c)) {
                self.pos += 1;
            }
        }
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
                Some('<')
                    if self.html_comments
                        && self.peek(1) == Some('!')
                        && self.peek(2) == Some('-')
                        && self.peek(3) == Some('-') =>
                {
                    while self.peek(0).is_some_and(|c| !is_line_terminator(c)) {
                        self.pos += 1;
                    }
                }
                Some('-')
                    if self.html_comments
                        && (newline || at_start)
                        && self.peek(1) == Some('-')
                        && self.peek(2) == Some('>') =>
                {
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
        self.escaped = false;
        self.octal = false;
        let tok = match self.peek(0) {
            None => Tok::Eof,
            Some(c) if is_id_start(c) || c == '\\' => Tok::Ident(self.identifier()?),
            Some('#') if self.peek(1).is_some_and(|c| is_id_start(c) || c == '\\') => {
                self.pos += 1;
                Tok::Private(self.identifier()?)
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
            escaped: self.escaped,
            legacy_octal: self.octal,
        })
    }

    /// Lit un identifiant, échappements `\uXXXX` et `\u{…}` compris.
    ///
    /// Un mot écrit avec un échappement (`l\u0065t`) est signalé par
    /// [`Token::escaped`] : l'analyseur ne le prend jamais pour un mot-clé.
    fn identifier(&mut self) -> Result<String, SyntaxError> {
        let mut out = String::new();
        loop {
            let c = match self.peek(0) {
                Some('\\') => {
                    let start = self.pos;
                    let bad = |lexer: &Self| {
                        lexer.error_at(start, "échappement invalide dans un identifiant")
                    };
                    if self.peek(1) != Some('u') {
                        return Err(bad(self));
                    }
                    self.escaped = true;
                    self.pos += 2;
                    let code = if self.peek(0) == Some('{') {
                        self.pos += 1;
                        let mut value = 0_u32;
                        let mut count = 0;
                        while let Some(digit) = self.peek(0).and_then(|c| c.to_digit(16)) {
                            value = value.saturating_mul(16).saturating_add(digit);
                            self.pos += 1;
                            count += 1;
                        }
                        if count == 0 || self.peek(0) != Some('}') {
                            return Err(bad(self));
                        }
                        self.pos += 1;
                        value
                    } else {
                        self.hex(4).ok_or_else(|| bad(self))?
                    };
                    let c = char::from_u32(code).ok_or_else(|| bad(self))?;
                    let valid = if out.is_empty() {
                        is_id_start(c)
                    } else {
                        is_id_part(c)
                    };
                    if !valid {
                        return Err(bad(self));
                    }
                    c
                }
                Some(c) if out.is_empty() && is_id_start(c) => {
                    self.pos += 1;
                    c
                }
                Some(c) if !out.is_empty() && is_id_part(c) => {
                    self.pos += 1;
                    c
                }
                _ => break,
            };
            out.push(c);
        }
        Ok(out)
    }

    /// Lettres et chiffres qui suivent (drapeaux d'une expression rationnelle).
    fn identifier_part(&mut self) -> String {
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

    /// Chiffres en base `radix`. Un séparateur `_` n'est permis qu'entre
    /// deux chiffres.
    fn digits(&mut self, radix: u32) -> Result<String, SyntaxError> {
        let mut out = String::new();
        let mut after_separator = true;
        loop {
            match self.peek(0) {
                Some(c) if c.is_digit(radix) => {
                    out.push(c);
                    after_separator = false;
                }
                Some('_') if !after_separator => after_separator = true,
                Some('_') => {
                    return Err(self.error_at(self.pos, "séparateur numérique mal placé"));
                }
                _ => break,
            }
            self.pos += 1;
        }
        if after_separator && !out.is_empty() {
            return Err(self.error_at(self.pos, "séparateur numérique mal placé"));
        }
        Ok(out)
    }

    fn number(&mut self) -> Result<Tok, SyntaxError> {
        let start = self.pos;
        let radix = match (self.peek(0), self.peek(1)) {
            (Some('0'), Some('x' | 'X')) => Some(16),
            (Some('0'), Some('o' | 'O')) => Some(8),
            (Some('0'), Some('b' | 'B')) => Some(2),
            _ => None,
        };
        // Chiffres et base de la partie entière, tant que le littéral peut
        // encore être un `BigInt` (ni point, ni exposant, ni zéro de tête).
        let mut integer: Option<(String, u32)> = None;
        let value = if let Some(radix) = radix {
            self.pos += 2;
            let digits = self.digits(radix)?;
            if digits.is_empty() {
                return Err(self.error_at(start, "littéral numérique invalide"));
            }
            integer = Some((digits.clone(), radix));
            // Accumulation en flottant : exacte jusqu'à 2^53, arrondie au-delà comme en JS.
            digits
                .chars()
                .filter_map(|c| c.to_digit(radix))
                .fold(0.0_f64, |acc, d| acc * f64::from(radix) + f64::from(d))
        } else {
            let mut text = self.digits(10)?;
            if text == "0" || !text.starts_with('0') {
                integer = Some((text.clone(), 10));
            }
            // Zéro de tête : forme héritée (`010` vaut 8, `08` vaut 8), sans
            // séparateur, et interdite en mode strict.
            let legacy = text.len() > 1 && text.starts_with('0');
            if legacy {
                if self.pos - start != text.len() {
                    return Err(self.error_at(start, "séparateur numérique mal placé"));
                }
                self.octal = true;
            }
            let legacy_octal = legacy && text.chars().all(|c| c < '8');
            if legacy_octal {
                let value = text
                    .chars()
                    .filter_map(|c| c.to_digit(8))
                    .fold(0.0_f64, |acc, d| acc * 8.0 + f64::from(d));
                text = value.to_string();
            }
            if self.peek(0) == Some('.') && !legacy_octal {
                integer = None;
                self.pos += 1;
                text.push('.');
                text.push_str(&self.digits(10)?);
            }
            if matches!(self.peek(0), Some('e' | 'E')) && !legacy_octal {
                let sign = usize::from(matches!(self.peek(1), Some('+' | '-')));
                if self.peek(1 + sign).is_some_and(|c| c.is_ascii_digit()) {
                    integer = None;
                    text.push('e');
                    if sign == 1 {
                        text.push(self.src[self.pos + 1]);
                    }
                    self.pos += 1 + sign;
                    text.push_str(&self.digits(10)?);
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
                let parsed = integer.and_then(|(digits, radix)| {
                    crate::bigint::BigInt::parse_digits(&digits, radix)
                });
                let Some(big) = parsed else {
                    return Err(self.error_at(start, "littéral BigInt invalide"));
                };
                self.pos += 1;
                if self
                    .peek(0)
                    .is_some_and(|c| is_id_start(c) || c.is_ascii_digit())
                {
                    return Err(self.error_at(self.pos, "identifiant collé à un nombre"));
                }
                Ok(Tok::BigInt(std::sync::Arc::new(big)))
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
            // Échappements octaux hérités : `\1` à `\377`.
            '0'..='7' => {
                self.octal = true;
                let mut value = c.to_digit(8).unwrap_or(0);
                let more = if c <= '3' { 2 } else { 1 };
                for _ in 0..more {
                    let Some(digit) = self.peek(0).and_then(|d| d.to_digit(8)) else {
                        break;
                    };
                    value = value * 8 + digit;
                    self.pos += 1;
                }
                out.push(char::from_u32(value).unwrap_or('\u{fffd}'));
            }
            '8' | '9' => {
                self.octal = true;
                out.push(c);
            }
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
        let mut valid = true;
        let first = self.pos;
        // Texte brut : le source entre les délimiteurs, fins de ligne
        // normalisées en LF.
        let raw = |lexer: &Self, end: usize| -> String {
            let text: String = lexer.src[first..end].iter().collect();
            text.replace("\r\n", "\n").replace('\r', "\n")
        };
        loop {
            match self.peek(0) {
                None => return Err(self.error_at(start, "gabarit non terminé")),
                Some('`') => {
                    let raw = raw(self, self.pos);
                    self.pos += 1;
                    return Ok(Tok::Template {
                        cooked: valid.then_some(cooked),
                        raw,
                        tail: true,
                    });
                }
                Some('$') if self.peek(1) == Some('{') => {
                    let raw = raw(self, self.pos);
                    self.pos += 2;
                    return Ok(Tok::Template {
                        cooked: valid.then_some(cooked),
                        raw,
                        tail: false,
                    });
                }
                Some('\\') => {
                    self.pos += 1;
                    let after_backslash = self.pos;
                    let escaped = self.escape(&mut cooked, start);
                    if escaped.is_err() || std::mem::take(&mut self.octal) {
                        // Échappement invalide : seul le texte brut compte ;
                        // on passe le caractère qui suit la barre oblique.
                        valid = false;
                        self.pos = (after_backslash + 1).min(self.src.len());
                    }
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
            escaped: false,
            legacy_octal: false,
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
        let flags = if self.peek(0).is_some_and(is_id_part) {
            self.identifier_part()
        } else {
            String::new()
        };
        Ok(Token {
            tok: Tok::Regex { pattern, flags },
            start,
            newline_before: false,
            escaped: false,
            legacy_octal: false,
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
                cooked: Some("a".into()),
                raw: "a".into(),
                tail: false
            }
        );
        assert_eq!(lexer.next().expect("x").tok, id("x"));
        assert_eq!(lexer.next().expect("}").tok, Tok::Punct("}"));
        assert_eq!(
            lexer.template_continue().expect("queue").tok,
            Tok::Template {
                cooked: Some("b".into()),
                raw: "b".into(),
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
        assert!(Lexer::new("1.5n").next().is_err());
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
