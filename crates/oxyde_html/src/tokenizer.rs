//! Tokenizer HTML (WHATWG HTML, §13.2.5).
//!
//! Même découpage que `HTMLTokenizer` de Blink : une machine à états qui
//! consomme des caractères et produit des jetons, pilotée par le constructeur
//! d'arbre qui peut changer l'état (RCDATA, RAWTEXT, script, PLAINTEXT).
//!
//! LIMITES CONNUES : les états « script data escaped » (`<!--` dans un script)
//! ne sont pas distingués de l'état script simple ; les erreurs d'analyse ne
//! sont pas rapportées (elles n'influencent pas l'arbre produit).

use std::collections::VecDeque;

use oxyde_dom::Attribute;

use crate::entities;

/// Une balise ouvrante ou fermante.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Tag {
    /// Nom en minuscules.
    pub name: String,
    /// Attributs (vides pour une balise fermante).
    pub attrs: Vec<Attribute>,
    /// Présence du `/` final.
    pub self_closing: bool,
}

impl Tag {
    /// Valeur de l'attribut `name`.
    #[must_use]
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.value.as_str())
    }
}

/// Jeton produit par le tokenizer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    /// `<!DOCTYPE …>`
    Doctype {
        /// Nom, absent si manquant.
        name: Option<String>,
        /// Identifiant public.
        public_id: Option<String>,
        /// Identifiant système.
        system_id: Option<String>,
        /// Drapeau « force-quirks ».
        force_quirks: bool,
    },
    /// Balise ouvrante.
    StartTag(Tag),
    /// Balise fermante.
    EndTag(Tag),
    /// Commentaire.
    Comment(String),
    /// Suite de caractères.
    Characters(String),
    /// Fin du flux.
    Eof,
}

/// États que le constructeur d'arbre peut imposer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentState {
    /// État « data ».
    Data,
    /// Texte avec références de caractères, sans balises (`<title>`, `<textarea>`).
    Rcdata,
    /// Texte brut (`<style>`, `<xmp>`, …).
    Rawtext,
    /// Contenu de `<script>`.
    ScriptData,
    /// Texte brut jusqu'à la fin du flux.
    Plaintext,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Content(ContentState),
    TagOpen,
    EndTagOpen,
    TagName,
    BeforeAttrName,
    AttrName,
    AfterAttrName,
    BeforeAttrValue,
    AttrValueQuoted(char),
    AttrValueUnquoted,
    AfterAttrValueQuoted,
    SelfClosingStartTag,
    BogusComment,
    MarkupDeclarationOpen,
    CommentStart,
    CommentStartDash,
    Comment,
    CommentEndDash,
    CommentEnd,
    CommentEndBang,
    Doctype,
    CdataSection,
}

/// Tokenizer HTML.
#[derive(Debug)]
pub struct Tokenizer {
    input: Vec<char>,
    pos: usize,
    state: State,
    last_start_tag: String,
    pending: VecDeque<Token>,
    text: String,
    tag: Tag,
    tag_is_end: bool,
    attr_name: String,
    attr_value: String,
    has_attr: bool,
    comment: String,
    /// Vrai si le nœud courant ajusté est hors de l'espace de noms HTML
    /// (autorise `<![CDATA[`). Positionné par le constructeur d'arbre.
    pub allow_cdata: bool,
    eof_emitted: bool,
}

fn is_ws(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\x0c' | ' ')
}

impl Tokenizer {
    /// Crée un tokenizer sur `source` (les fins de ligne sont normalisées en LF).
    #[must_use]
    pub fn new(source: &str) -> Self {
        let mut input = Vec::with_capacity(source.len());
        let mut chars = source.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\r' {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                input.push('\n');
            } else {
                input.push(c);
            }
        }
        Self {
            input,
            pos: 0,
            state: State::Content(ContentState::Data),
            last_start_tag: String::new(),
            pending: VecDeque::new(),
            text: String::new(),
            tag: Tag::default(),
            tag_is_end: false,
            attr_name: String::new(),
            attr_value: String::new(),
            has_attr: false,
            comment: String::new(),
            allow_cdata: false,
            eof_emitted: false,
        }
    }

    /// Impose l'état de contenu (appelé par le constructeur d'arbre).
    pub fn set_content_state(&mut self, state: ContentState) {
        self.state = State::Content(state);
    }

    /// Jeton suivant. Après [`Token::Eof`], renvoie `Eof` indéfiniment.
    pub fn next_token(&mut self) -> Token {
        loop {
            if let Some(t) = self.pending.pop_front() {
                return t;
            }
            if self.eof_emitted {
                return Token::Eof;
            }
            self.step();
        }
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += 1;
        Some(c)
    }

    /// Vrai si l'entrée, à partir de `pos`, commence par `s` (ASCII, insensible à la casse).
    fn lookahead_ci(&self, s: &str) -> bool {
        let mut i = self.pos;
        for expected in s.chars() {
            match self.input.get(i) {
                Some(c) if c.eq_ignore_ascii_case(&expected) => i += 1,
                _ => return false,
            }
        }
        true
    }

    fn lookahead(&self, s: &str) -> bool {
        let mut i = self.pos;
        for expected in s.chars() {
            match self.input.get(i) {
                Some(c) if *c == expected => i += 1,
                _ => return false,
            }
        }
        true
    }

    fn flush_text(&mut self) {
        if !self.text.is_empty() {
            self.pending
                .push_back(Token::Characters(std::mem::take(&mut self.text)));
        }
    }

    fn emit(&mut self, token: Token) {
        self.flush_text();
        self.pending.push_back(token);
    }

    fn emit_eof(&mut self) {
        self.emit(Token::Eof);
        self.eof_emitted = true;
    }

    fn emit_comment(&mut self) {
        let c = std::mem::take(&mut self.comment);
        self.emit(Token::Comment(c));
    }

    fn start_tag(&mut self, is_end: bool) {
        self.tag = Tag::default();
        self.tag_is_end = is_end;
        self.has_attr = false;
    }

    fn start_attr(&mut self) {
        self.finish_attr();
        self.has_attr = true;
        self.attr_name.clear();
        self.attr_value.clear();
    }

    fn finish_attr(&mut self) {
        if !self.has_attr {
            return;
        }
        self.has_attr = false;
        let name = std::mem::take(&mut self.attr_name);
        let value = std::mem::take(&mut self.attr_value);
        // Un attribut en double est ignoré (le premier gagne).
        if !self.tag.attrs.iter().any(|a| a.name == name) {
            self.tag.attrs.push(Attribute::new(name, value));
        }
    }

    fn emit_tag(&mut self) {
        self.finish_attr();
        let mut tag = std::mem::take(&mut self.tag);
        self.state = State::Content(ContentState::Data);
        if self.tag_is_end {
            tag.attrs.clear();
            tag.self_closing = false;
            self.emit(Token::EndTag(tag));
        } else {
            self.last_start_tag.clone_from(&tag.name);
            self.emit(Token::StartTag(tag));
        }
    }

    /// Dans RCDATA / RAWTEXT / script : vrai si `pos` est sur une « balise
    /// fermante appropriée » (`</nom` suivi d'un blanc, `/` ou `>`).
    fn at_appropriate_end_tag(&self) -> bool {
        if self.last_start_tag.is_empty() || !self.lookahead("</") {
            return false;
        }
        let mut i = self.pos + 2;
        for expected in self.last_start_tag.chars() {
            match self.input.get(i) {
                Some(c) if c.to_ascii_lowercase() == expected => i += 1,
                _ => return false,
            }
        }
        matches!(self.input.get(i), Some(c) if is_ws(*c) || *c == '/' || *c == '>')
    }

    fn step(&mut self) {
        match self.state {
            State::Content(cs) => self.step_content(cs),
            State::TagOpen => self.step_tag_open(),
            State::EndTagOpen => self.step_end_tag_open(),
            State::TagName => self.step_tag_name(),
            State::BeforeAttrName => self.step_before_attr_name(),
            State::AttrName => self.step_attr_name(),
            State::AfterAttrName => self.step_after_attr_name(),
            State::BeforeAttrValue => self.step_before_attr_value(),
            State::AttrValueQuoted(q) => self.step_attr_value_quoted(q),
            State::AttrValueUnquoted => self.step_attr_value_unquoted(),
            State::AfterAttrValueQuoted => self.step_after_attr_value_quoted(),
            State::SelfClosingStartTag => self.step_self_closing(),
            State::BogusComment => self.step_bogus_comment(),
            State::MarkupDeclarationOpen => self.step_markup_declaration_open(),
            State::CommentStart => self.step_comment_start(),
            State::CommentStartDash => self.step_comment_start_dash(),
            State::Comment => self.step_comment(),
            State::CommentEndDash => self.step_comment_end_dash(),
            State::CommentEnd => self.step_comment_end(),
            State::CommentEndBang => self.step_comment_end_bang(),
            State::Doctype => self.step_doctype(),
            State::CdataSection => self.step_cdata(),
        }
    }

    fn step_content(&mut self, cs: ContentState) {
        let Some(c) = self.peek() else {
            self.emit_eof();
            return;
        };
        match (cs, c) {
            (ContentState::Data, '<') => {
                self.pos += 1;
                self.state = State::TagOpen;
            }
            (ContentState::Data | ContentState::Rcdata, '&') => {
                self.pos += 1;
                let s = self.consume_char_ref(false);
                self.text.push_str(&s);
            }
            (ContentState::Rcdata | ContentState::Rawtext | ContentState::ScriptData, '<') => {
                if self.at_appropriate_end_tag() {
                    self.pos += 2;
                    self.start_tag(true);
                    self.state = State::TagName;
                } else {
                    self.pos += 1;
                    self.text.push('<');
                }
            }
            (ContentState::Data, _) => {
                self.pos += 1;
                self.text.push(c);
            }
            (_, '\0') => {
                self.pos += 1;
                self.text.push('\u{fffd}');
            }
            _ => {
                self.pos += 1;
                self.text.push(c);
            }
        }
    }

    fn step_tag_open(&mut self) {
        match self.peek() {
            Some('!') => {
                self.pos += 1;
                self.state = State::MarkupDeclarationOpen;
            }
            Some('/') => {
                self.pos += 1;
                self.state = State::EndTagOpen;
            }
            Some(c) if c.is_ascii_alphabetic() => {
                self.start_tag(false);
                self.state = State::TagName;
            }
            Some('?') => {
                self.comment.clear();
                self.state = State::BogusComment;
            }
            _ => {
                self.text.push('<');
                self.state = State::Content(ContentState::Data);
            }
        }
    }

    fn step_end_tag_open(&mut self) {
        match self.peek() {
            Some(c) if c.is_ascii_alphabetic() => {
                self.start_tag(true);
                self.state = State::TagName;
            }
            Some('>') => {
                self.pos += 1;
                self.state = State::Content(ContentState::Data);
            }
            None => {
                self.text.push_str("</");
                self.emit_eof();
            }
            Some(_) => {
                self.comment.clear();
                self.state = State::BogusComment;
            }
        }
    }

    fn step_tag_name(&mut self) {
        match self.bump() {
            None => self.emit_eof(),
            Some(c) if is_ws(c) => self.state = State::BeforeAttrName,
            Some('/') => self.state = State::SelfClosingStartTag,
            Some('>') => self.emit_tag(),
            Some('\0') => self.tag.name.push('\u{fffd}'),
            Some(c) => self.tag.name.push(c.to_ascii_lowercase()),
        }
    }

    fn step_before_attr_name(&mut self) {
        match self.peek() {
            Some(c) if is_ws(c) => self.pos += 1,
            Some('/' | '>') | None => self.state = State::AfterAttrName,
            Some('=') => {
                self.pos += 1;
                self.start_attr();
                self.attr_name.push('=');
                self.state = State::AttrName;
            }
            Some(_) => {
                self.start_attr();
                self.state = State::AttrName;
            }
        }
    }

    fn step_attr_name(&mut self) {
        match self.peek() {
            Some(c) if is_ws(c) => self.state = State::AfterAttrName,
            Some('/' | '>') | None => self.state = State::AfterAttrName,
            Some('=') => {
                self.pos += 1;
                self.state = State::BeforeAttrValue;
            }
            Some('\0') => {
                self.pos += 1;
                self.attr_name.push('\u{fffd}');
            }
            Some(c) => {
                self.pos += 1;
                self.attr_name.push(c.to_ascii_lowercase());
            }
        }
    }

    fn step_after_attr_name(&mut self) {
        match self.peek() {
            Some(c) if is_ws(c) => self.pos += 1,
            Some('/') => {
                self.pos += 1;
                self.state = State::SelfClosingStartTag;
            }
            Some('=') => {
                self.pos += 1;
                self.state = State::BeforeAttrValue;
            }
            Some('>') => {
                self.pos += 1;
                self.emit_tag();
            }
            None => self.emit_eof(),
            Some(_) => {
                self.start_attr();
                self.state = State::AttrName;
            }
        }
    }

    fn step_before_attr_value(&mut self) {
        match self.peek() {
            Some(c) if is_ws(c) => self.pos += 1,
            Some(q @ ('"' | '\'')) => {
                self.pos += 1;
                self.state = State::AttrValueQuoted(q);
            }
            Some('>') => {
                self.pos += 1;
                self.emit_tag();
            }
            _ => self.state = State::AttrValueUnquoted,
        }
    }

    fn step_attr_value_quoted(&mut self, quote: char) {
        match self.bump() {
            None => self.emit_eof(),
            Some(c) if c == quote => self.state = State::AfterAttrValueQuoted,
            Some('&') => {
                let s = self.consume_char_ref(true);
                self.attr_value.push_str(&s);
            }
            Some('\0') => self.attr_value.push('\u{fffd}'),
            Some(c) => self.attr_value.push(c),
        }
    }

    fn step_attr_value_unquoted(&mut self) {
        match self.bump() {
            None => self.emit_eof(),
            Some(c) if is_ws(c) => self.state = State::BeforeAttrName,
            Some('&') => {
                let s = self.consume_char_ref(true);
                self.attr_value.push_str(&s);
            }
            Some('>') => self.emit_tag(),
            Some('\0') => self.attr_value.push('\u{fffd}'),
            Some(c) => self.attr_value.push(c),
        }
    }

    fn step_after_attr_value_quoted(&mut self) {
        match self.peek() {
            None => self.emit_eof(),
            Some(c) if is_ws(c) => {
                self.pos += 1;
                self.state = State::BeforeAttrName;
            }
            Some('/') => {
                self.pos += 1;
                self.state = State::SelfClosingStartTag;
            }
            Some('>') => {
                self.pos += 1;
                self.emit_tag();
            }
            Some(_) => self.state = State::BeforeAttrName,
        }
    }

    fn step_self_closing(&mut self) {
        match self.peek() {
            None => self.emit_eof(),
            Some('>') => {
                self.pos += 1;
                self.tag.self_closing = true;
                self.emit_tag();
            }
            Some(_) => self.state = State::BeforeAttrName,
        }
    }

    fn step_bogus_comment(&mut self) {
        loop {
            match self.bump() {
                None => {
                    self.emit_comment();
                    self.emit_eof();
                    return;
                }
                Some('>') => {
                    self.emit_comment();
                    self.state = State::Content(ContentState::Data);
                    return;
                }
                Some('\0') => self.comment.push('\u{fffd}'),
                Some(c) => self.comment.push(c),
            }
        }
    }

    fn step_markup_declaration_open(&mut self) {
        self.comment.clear();
        if self.lookahead("--") {
            self.pos += 2;
            self.state = State::CommentStart;
        } else if self.lookahead_ci("doctype") {
            self.pos += 7;
            self.state = State::Doctype;
        } else if self.lookahead("[CDATA[") {
            self.pos += 7;
            if self.allow_cdata {
                self.state = State::CdataSection;
            } else {
                self.comment.push_str("[CDATA[");
                self.state = State::BogusComment;
            }
        } else {
            self.state = State::BogusComment;
        }
    }

    fn step_comment_start(&mut self) {
        match self.peek() {
            Some('-') => {
                self.pos += 1;
                self.state = State::CommentStartDash;
            }
            Some('>') => {
                self.pos += 1;
                self.emit_comment();
                self.state = State::Content(ContentState::Data);
            }
            _ => self.state = State::Comment,
        }
    }

    fn step_comment_start_dash(&mut self) {
        match self.peek() {
            Some('-') => {
                self.pos += 1;
                self.state = State::CommentEnd;
            }
            Some('>') => {
                self.pos += 1;
                self.emit_comment();
                self.state = State::Content(ContentState::Data);
            }
            None => {
                self.emit_comment();
                self.emit_eof();
            }
            Some(_) => {
                self.comment.push('-');
                self.state = State::Comment;
            }
        }
    }

    fn step_comment(&mut self) {
        loop {
            match self.bump() {
                None => {
                    self.emit_comment();
                    self.emit_eof();
                    return;
                }
                Some('-') => {
                    self.state = State::CommentEndDash;
                    return;
                }
                Some('\0') => self.comment.push('\u{fffd}'),
                Some(c) => self.comment.push(c),
            }
        }
    }

    fn step_comment_end_dash(&mut self) {
        match self.peek() {
            Some('-') => {
                self.pos += 1;
                self.state = State::CommentEnd;
            }
            None => {
                self.emit_comment();
                self.emit_eof();
            }
            Some(_) => {
                self.comment.push('-');
                self.state = State::Comment;
            }
        }
    }

    fn step_comment_end(&mut self) {
        match self.peek() {
            Some('>') => {
                self.pos += 1;
                self.emit_comment();
                self.state = State::Content(ContentState::Data);
            }
            Some('!') => {
                self.pos += 1;
                self.state = State::CommentEndBang;
            }
            Some('-') => {
                self.pos += 1;
                self.comment.push('-');
            }
            None => {
                self.emit_comment();
                self.emit_eof();
            }
            Some(_) => {
                self.comment.push_str("--");
                self.state = State::Comment;
            }
        }
    }

    fn step_comment_end_bang(&mut self) {
        match self.peek() {
            Some('-') => {
                self.pos += 1;
                self.comment.push_str("--!");
                self.state = State::CommentEndDash;
            }
            Some('>') => {
                self.pos += 1;
                self.emit_comment();
                self.state = State::Content(ContentState::Data);
            }
            None => {
                self.emit_comment();
                self.emit_eof();
            }
            Some(_) => {
                self.comment.push_str("--!");
                self.state = State::Comment;
            }
        }
    }

    fn skip_ws(&mut self) {
        while self.peek().is_some_and(is_ws) {
            self.pos += 1;
        }
    }

    /// Lit une chaîne entre guillemets d'identifiant DOCTYPE. `Err(())` si elle
    /// est absente ou interrompue (ce qui force le mode quirks).
    fn doctype_quoted(&mut self) -> Result<String, ()> {
        let quote = match self.peek() {
            Some(q @ ('"' | '\'')) => q,
            _ => return Err(()),
        };
        self.pos += 1;
        let mut out = String::new();
        loop {
            match self.peek() {
                None | Some('>') => return Err(()),
                Some(c) if c == quote => {
                    self.pos += 1;
                    return Ok(out);
                }
                Some('\0') => {
                    self.pos += 1;
                    out.push('\u{fffd}');
                }
                Some(c) => {
                    self.pos += 1;
                    out.push(c);
                }
            }
        }
    }

    /// DOCTYPE complet, analysé d'un bloc (les états du standard ne diffèrent
    /// que par les erreurs signalées).
    fn step_doctype(&mut self) {
        self.state = State::Content(ContentState::Data);
        let mut name = None;
        let mut public_id = None;
        let mut system_id = None;
        let mut force_quirks = false;

        self.skip_ws();
        let mut n = String::new();
        while let Some(c) = self.peek() {
            if is_ws(c) || c == '>' {
                break;
            }
            self.pos += 1;
            n.push(if c == '\0' {
                '\u{fffd}'
            } else {
                c.to_ascii_lowercase()
            });
        }
        if n.is_empty() {
            force_quirks = true;
        } else {
            name = Some(n);
        }
        self.skip_ws();

        if self.lookahead_ci("public") {
            self.pos += 6;
            self.skip_ws();
            match self.doctype_quoted() {
                Ok(p) => {
                    public_id = Some(p);
                    self.skip_ws();
                    if matches!(self.peek(), Some('"' | '\'')) {
                        match self.doctype_quoted() {
                            Ok(s) => system_id = Some(s),
                            Err(()) => force_quirks = true,
                        }
                    }
                }
                Err(()) => force_quirks = true,
            }
        } else if self.lookahead_ci("system") {
            self.pos += 6;
            self.skip_ws();
            match self.doctype_quoted() {
                Ok(s) => system_id = Some(s),
                Err(()) => force_quirks = true,
            }
        } else if !matches!(self.peek(), Some('>')) {
            force_quirks = true;
        }

        // « Bogus DOCTYPE » : on saute jusqu'au `>`.
        let mut closed = false;
        while let Some(c) = self.bump() {
            if c == '>' {
                closed = true;
                break;
            }
        }
        if !closed {
            force_quirks = true;
        }
        self.emit(Token::Doctype {
            name,
            public_id,
            system_id,
            force_quirks,
        });
        if !closed {
            self.emit_eof();
        }
    }

    fn step_cdata(&mut self) {
        loop {
            if self.lookahead("]]>") {
                self.pos += 3;
                self.state = State::Content(ContentState::Data);
                return;
            }
            match self.bump() {
                None => {
                    self.emit_eof();
                    return;
                }
                Some(c) => self.text.push(c),
            }
        }
    }

    /// Consomme une référence de caractère (le `&` est déjà consommé) et
    /// renvoie le texte à insérer (`"&"` si ce n'en est pas une).
    fn consume_char_ref(&mut self, in_attr: bool) -> String {
        match self.peek() {
            Some('#') => self.consume_numeric_ref(),
            Some(c) if c.is_ascii_alphanumeric() => self.consume_named_ref(in_attr),
            _ => "&".to_owned(),
        }
    }

    fn consume_numeric_ref(&mut self) -> String {
        let start = self.pos;
        self.pos += 1; // '#'
        let hex = matches!(self.peek(), Some('x' | 'X'));
        if hex {
            self.pos += 1;
        }
        let radix = if hex { 16 } else { 10 };
        let mut value: u32 = 0;
        let mut digits = 0usize;
        while let Some(d) = self.peek().and_then(|c| c.to_digit(radix)) {
            self.pos += 1;
            digits += 1;
            // Saturation : toute valeur > 0x10FFFF devient U+FFFD plus bas.
            value = value.saturating_mul(radix).saturating_add(d);
        }
        if digits == 0 {
            self.pos = start;
            return "&".to_owned();
        }
        if self.peek() == Some(';') {
            self.pos += 1;
        }
        let c = match value {
            0 => '\u{fffd}',
            0x80..=0x9f => WINDOWS_1252[(value - 0x80) as usize],
            _ => char::from_u32(value).unwrap_or('\u{fffd}'),
        };
        c.to_string()
    }

    fn consume_named_ref(&mut self, in_attr: bool) -> String {
        // Plus longue correspondance : on essaie des préfixes de longueur décroissante.
        let mut candidate = String::new();
        let mut i = self.pos;
        while candidate.len() < entities::MAX_NAME_LEN {
            match self.input.get(i) {
                Some(c) if c.is_ascii_alphanumeric() => {
                    candidate.push(*c);
                    i += 1;
                }
                _ => break,
            }
        }
        while !candidate.is_empty() {
            if let Some(value) = entities::lookup(&candidate) {
                let after = self.input.get(self.pos + candidate.len()).copied();
                let terminated = after == Some(';');
                if terminated || entities::is_legacy(&candidate) {
                    if !terminated
                        && in_attr
                        && after.is_some_and(|c| c == '=' || c.is_ascii_alphanumeric())
                    {
                        // Raison historique : `?a=1&copy=2` dans un attribut reste littéral.
                        return "&".to_owned();
                    }
                    self.pos += candidate.len() + usize::from(terminated);
                    return value.to_owned();
                }
            }
            candidate.pop();
        }
        "&".to_owned()
    }
}

/// Remplacements des références numériques 0x80–0x9F (table du standard).
const WINDOWS_1252: [char; 32] = [
    '\u{20ac}', '\u{81}', '\u{201a}', '\u{192}', '\u{201e}', '\u{2026}', '\u{2020}', '\u{2021}',
    '\u{2c6}', '\u{2030}', '\u{160}', '\u{2039}', '\u{152}', '\u{8d}', '\u{17d}', '\u{8f}',
    '\u{90}', '\u{2018}', '\u{2019}', '\u{201c}', '\u{201d}', '\u{2022}', '\u{2013}', '\u{2014}',
    '\u{2dc}', '\u{2122}', '\u{161}', '\u{203a}', '\u{153}', '\u{9d}', '\u{17e}', '\u{178}',
];

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(src: &str) -> Vec<Token> {
        let mut t = Tokenizer::new(src);
        let mut out = Vec::new();
        loop {
            let tok = t.next_token();
            if tok == Token::Eof {
                return out;
            }
            out.push(tok);
        }
    }

    fn start(name: &str, attrs: &[(&str, &str)], self_closing: bool) -> Token {
        Token::StartTag(Tag {
            name: name.into(),
            attrs: attrs.iter().map(|(n, v)| Attribute::new(*n, *v)).collect(),
            self_closing,
        })
    }

    fn end(name: &str) -> Token {
        Token::EndTag(Tag {
            name: name.into(),
            ..Tag::default()
        })
    }

    fn chars(s: &str) -> Token {
        Token::Characters(s.into())
    }

    #[test]
    fn tags_and_attributes() {
        assert_eq!(
            tokens("<A HREF='x' b=c d>t</a><br/>"),
            vec![
                start("a", &[("href", "x"), ("b", "c"), ("d", "")], false),
                chars("t"),
                end("a"),
                start("br", &[], true),
            ]
        );
    }

    #[test]
    fn duplicate_attribute_keeps_first() {
        assert_eq!(
            tokens("<p a=1 a=2>"),
            vec![start("p", &[("a", "1")], false)]
        );
    }

    #[test]
    fn character_references() {
        assert_eq!(
            tokens("a&amp;b&lt;&#233;&#x41;&eacute;"),
            vec![chars("a&b<éAé")]
        );
        assert_eq!(
            tokens("&copy 2026 &inconnu; &#128;"),
            vec![chars("© 2026 &inconnu; €")]
        );
        assert_eq!(
            tokens("<a href='?a=1&copy=2&amp;c'>"),
            vec![start("a", &[("href", "?a=1&copy=2&c")], false)]
        );
        assert_eq!(
            tokens("&#0;&#x110000;&#xD800;"),
            vec![chars("\u{fffd}\u{fffd}\u{fffd}")]
        );
    }

    #[test]
    fn comments_and_doctype() {
        assert_eq!(
            tokens("<!DOCTYPE html><!-- a -- b --><!---->"),
            vec![
                Token::Doctype {
                    name: Some("html".into()),
                    public_id: None,
                    system_id: None,
                    force_quirks: false
                },
                Token::Comment(" a -- b ".into()),
                Token::Comment(String::new()),
            ]
        );
        assert_eq!(
            tokens("<!doctype HTML PUBLIC \"-//W3C//DTD HTML 4.01//EN\" 'u'>"),
            vec![Token::Doctype {
                name: Some("html".into()),
                public_id: Some("-//W3C//DTD HTML 4.01//EN".into()),
                system_id: Some("u".into()),
                force_quirks: false
            }]
        );
    }

    #[test]
    fn rawtext_and_rcdata() {
        let mut t = Tokenizer::new("<style>a<b</STYLE >x");
        assert_eq!(t.next_token(), start("style", &[], false));
        t.set_content_state(ContentState::Rawtext);
        assert_eq!(t.next_token(), chars("a<b"));
        assert_eq!(t.next_token(), end("style"));
        assert_eq!(t.next_token(), chars("x"));
        assert_eq!(t.next_token(), Token::Eof);
        assert_eq!(t.next_token(), Token::Eof);

        let mut t = Tokenizer::new("<title>a &amp; <b></title>");
        t.next_token();
        t.set_content_state(ContentState::Rcdata);
        assert_eq!(t.next_token(), chars("a & <b>"));
        assert_eq!(t.next_token(), end("title"));
    }

    #[test]
    fn stray_markup_is_text() {
        assert_eq!(tokens("a < b"), vec![chars("a < b")]);
        assert_eq!(tokens("x</>y"), vec![chars("xy")]);
        assert_eq!(tokens("<?php ?>"), vec![Token::Comment("?php ?".into())]);
    }

    #[test]
    fn never_panics_on_truncated_input() {
        let src =
            "<!DOCTYPE html PUBLIC \"x\" 'y'><a b='c' d=e f><!-- c --><![CDATA[x]]>&amp;&#x41;";
        let chars: Vec<char> = src.chars().collect();
        for n in 0..=chars.len() {
            let prefix: String = chars[..n].iter().collect();
            let _ = tokens(&prefix);
        }
    }
}
