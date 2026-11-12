//! Analyse de Web IDL (<https://webidl.spec.whatwg.org/>).

use std::fmt;

/// Profondeur maximale d'imbrication des types (`sequence<sequence<…>>`).
const MAX_TYPE_DEPTH: usize = 32;

/// Erreur de syntaxe dans un fichier Web IDL.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdlError {
    /// Numéro de ligne, à partir de 1.
    pub line: usize,
    /// Description de l'erreur.
    pub message: String,
}

impl fmt::Display for IdlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ligne {} : {}", self.line, self.message)
    }
}

impl std::error::Error for IdlError {}

/// Membre d'une interface.
#[derive(Clone, Debug, PartialEq)]
pub enum Member {
    /// `const unsigned short NOM = 1;` (seules les constantes numériques et
    /// booléennes existent en Web IDL ; un booléen vaut 0 ou 1).
    Const {
        /// Nom de la constante.
        name: String,
        /// Valeur.
        value: f64,
    },
    /// `[readonly] attribute Type nom;`
    Attribute {
        /// Nom de l'attribut.
        name: String,
        /// Type, sous forme de texte normalisé.
        ty: String,
        /// Lecture seule.
        readonly: bool,
        /// Membre statique.
        is_static: bool,
    },
    /// `Type nom(arguments);`
    Operation {
        /// Nom de l'opération.
        name: String,
        /// Type de retour, sous forme de texte normalisé.
        returns: String,
        /// Nombre d'arguments requis.
        length: u32,
        /// Membre statique.
        is_static: bool,
    },
    /// `constructor(arguments);`
    Constructor {
        /// Nombre d'arguments requis.
        length: u32,
    },
}

/// Nature d'une définition d'interface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InterfaceKind {
    /// `interface`
    Interface,
    /// `interface mixin`
    Mixin,
    /// `namespace`
    Namespace,
    /// `callback interface`
    Callback,
}

/// Interface, mixin ou espace de noms, éventuellement partiel.
#[derive(Clone, Debug, PartialEq)]
pub struct Interface {
    /// Nom.
    pub name: String,
    /// Interface héritée (`interface A : B`).
    pub parent: Option<String>,
    /// Nature de la définition.
    pub kind: InterfaceKind,
    /// Définition partielle (`partial interface`).
    pub partial: bool,
    /// Membres, dans l'ordre du texte.
    pub members: Vec<Member>,
}

/// Définition de premier niveau.
#[derive(Clone, Debug, PartialEq)]
pub enum Definition {
    /// Interface, mixin, espace de noms ou interface de rappel.
    Interface(Interface),
    /// `Cible includes Mixin;`
    Includes {
        /// Interface qui reçoit les membres.
        target: String,
        /// Mixin inclus.
        mixin: String,
    },
    /// `dictionary`, `enum`, `typedef` ou `callback` : seul le nom est gardé.
    Other {
        /// Nom défini.
        name: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Ident(String),
    Number(f64),
    Str(String),
    Punct(&'static str),
    Eof,
}

struct Lexer<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    line: usize,
    /// Ligne où commence le dernier jeton lu.
    token_line: usize,
}

const PUNCTUATION: &[&str] = &[
    "...", "(", ")", "[", "]", "{", "}", "<", ">", ",", ";", ":", "=", "?", "*", "-",
];

impl Lexer<'_> {
    fn error<T>(&self, message: impl Into<String>) -> Result<T, IdlError> {
        Err(IdlError {
            line: self.line,
            message: message.into(),
        })
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.chars.next()?;
        if c == '\n' {
            self.line += 1;
        }
        Some(c)
    }

    fn skip_blanks(&mut self) -> Result<(), IdlError> {
        loop {
            match self.chars.peek() {
                Some(c) if c.is_whitespace() => {
                    self.bump();
                }
                Some('/') => {
                    let mut ahead = self.chars.clone();
                    ahead.next();
                    match ahead.next() {
                        Some('/') => {
                            while let Some(c) = self.bump() {
                                if c == '\n' {
                                    break;
                                }
                            }
                        }
                        Some('*') => {
                            self.bump();
                            self.bump();
                            let mut previous = ' ';
                            loop {
                                match self.bump() {
                                    Some('/') if previous == '*' => break,
                                    Some(c) => previous = c,
                                    None => return self.error("commentaire non fermé"),
                                }
                            }
                        }
                        _ => return self.error("« / » inattendu"),
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    fn next(&mut self) -> Result<Token, IdlError> {
        self.skip_blanks()?;
        self.token_line = self.line;
        let Some(&c) = self.chars.peek() else {
            return Ok(Token::Eof);
        };
        if c.is_ascii_alphabetic() || c == '_' {
            let mut text = String::new();
            while let Some(&c) = self.chars.peek() {
                if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                    text.push(c);
                    self.bump();
                } else {
                    break;
                }
            }
            return Ok(Token::Ident(text));
        }
        if c.is_ascii_digit() || c == '.' && self.is_fraction() {
            let mut text = String::new();
            while let Some(&c) = self.chars.peek() {
                if c.is_ascii_alphanumeric() || c == '.' || c == '+' && text.ends_with(['e', 'E']) {
                    text.push(c);
                    self.bump();
                } else {
                    break;
                }
            }
            let value = if let Some(hex) = text.strip_prefix("0x").or(text.strip_prefix("0X")) {
                u64::from_str_radix(hex, 16).ok().map(|v| {
                    #[expect(clippy::cast_precision_loss, reason = "constantes IDL petites")]
                    let value = v as f64;
                    value
                })
            } else {
                text.parse::<f64>().ok()
            };
            return match value {
                Some(value) => Ok(Token::Number(value)),
                None => self.error(format!("nombre invalide : {text}")),
            };
        }
        if c == '"' {
            self.bump();
            let mut text = String::new();
            loop {
                match self.bump() {
                    Some('"') => return Ok(Token::Str(text)),
                    Some(c) => text.push(c),
                    None => return self.error("chaîne non fermée"),
                }
            }
        }
        for punct in PUNCTUATION {
            let mut ahead = self.chars.clone();
            if punct.chars().all(|p| ahead.next() == Some(p)) {
                for _ in 0..punct.len() {
                    self.bump();
                }
                return Ok(Token::Punct(punct));
            }
        }
        self.error(format!("caractère inattendu « {c} »"))
    }

    /// Vrai si le point courant commence un nombre (`.5`) et non `...`.
    fn is_fraction(&self) -> bool {
        let mut ahead = self.chars.clone();
        ahead.next();
        ahead.next().is_some_and(|c| c.is_ascii_digit())
    }
}

struct Parser<'a> {
    lexer: Lexer<'a>,
    token: Token,
    /// Ligne du jeton courant, pour les messages d'erreur.
    line: usize,
}

type PResult<T> = Result<T, IdlError>;

impl Parser<'_> {
    fn error<T>(&self, message: impl Into<String>) -> PResult<T> {
        Err(IdlError {
            line: self.line,
            message: message.into(),
        })
    }

    fn advance(&mut self) -> PResult<Token> {
        let next = self.lexer.next()?;
        self.line = self.lexer.token_line;
        Ok(std::mem::replace(&mut self.token, next))
    }

    fn is_punct(&self, punct: &str) -> bool {
        matches!(&self.token, Token::Punct(p) if *p == punct)
    }

    fn eat_punct(&mut self, punct: &str) -> PResult<bool> {
        if self.is_punct(punct) {
            self.advance()?;
            return Ok(true);
        }
        Ok(false)
    }

    fn expect_punct(&mut self, punct: &str) -> PResult<()> {
        if self.eat_punct(punct)? {
            Ok(())
        } else {
            self.error(format!("« {punct} » attendu"))
        }
    }

    fn is_word(&self, word: &str) -> bool {
        matches!(&self.token, Token::Ident(w) if w == word)
    }

    fn eat_word(&mut self, word: &str) -> PResult<bool> {
        if self.is_word(word) {
            self.advance()?;
            return Ok(true);
        }
        Ok(false)
    }

    /// Identifiant ; un tiret bas initial (`_interface`) échappe un mot réservé.
    fn identifier(&mut self) -> PResult<String> {
        let Token::Ident(name) = &self.token else {
            return self.error("identifiant attendu");
        };
        let name = name.strip_prefix('_').unwrap_or(name).to_owned();
        self.advance()?;
        Ok(name)
    }

    /// Saute des attributs étendus : `[Exposed=Window, SameObject]`.
    fn extended_attributes(&mut self) -> PResult<()> {
        if !self.is_punct("[") {
            return Ok(());
        }
        let mut depth = 0_usize;
        loop {
            match self.advance()? {
                Token::Punct("[" | "(") => depth += 1,
                Token::Punct("]" | ")") => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return Ok(());
                    }
                }
                Token::Eof => return self.error("attributs étendus non fermés"),
                _ => {}
            }
        }
    }

    /// Type, rendu sous forme de texte normalisé.
    fn ty(&mut self, depth: usize) -> PResult<String> {
        if depth > MAX_TYPE_DEPTH {
            return self.error("type trop imbriqué");
        }
        self.extended_attributes()?;
        let mut text = if self.eat_punct("(")? {
            let mut members = vec![self.ty(depth + 1)?];
            while self.eat_word("or")? {
                members.push(self.ty(depth + 1)?);
            }
            self.expect_punct(")")?;
            if members.len() < 2 {
                return self.error("un type union a au moins deux membres");
            }
            format!("({})", members.join(" or "))
        } else {
            let mut name = self.identifier()?;
            match name.as_str() {
                "unsigned" | "unrestricted" => {
                    name.push(' ');
                    name.push_str(&self.identifier()?);
                    if self.eat_word("long")? {
                        name.push_str(" long");
                    }
                }
                "long" if self.eat_word("long")? => name.push_str(" long"),
                _ => {}
            }
            if self.eat_punct("<")? {
                let mut parameters = vec![self.ty(depth + 1)?];
                while self.eat_punct(",")? {
                    parameters.push(self.ty(depth + 1)?);
                }
                self.expect_punct(">")?;
                name = format!("{name}<{}>", parameters.join(", "));
            }
            name
        };
        if self.eat_punct("?")? {
            text.push('?');
        }
        Ok(text)
    }

    /// Liste d'arguments (la parenthèse ouvrante est déjà lue) ; renvoie le
    /// nombre d'arguments requis.
    fn arguments(&mut self) -> PResult<u32> {
        let mut required = 0;
        while !self.eat_punct(")")? {
            self.extended_attributes()?;
            let optional = self.eat_word("optional")?;
            self.ty(0)?;
            let variadic = self.eat_punct("...")?;
            self.identifier()?;
            if self.eat_punct("=")? {
                // Valeur par défaut : sautée jusqu'à la fin de l'argument.
                let mut depth = 0_usize;
                loop {
                    match &self.token {
                        Token::Punct("," | ")") if depth == 0 => break,
                        Token::Punct("[" | "{" | "(") => depth += 1,
                        Token::Punct("]" | "}" | ")") => depth = depth.saturating_sub(1),
                        Token::Eof => return self.error("arguments non fermés"),
                        _ => {}
                    }
                    self.advance()?;
                }
            }
            if !optional && !variadic {
                required += 1;
            }
            if !self.is_punct(")") {
                self.expect_punct(",")?;
            }
        }
        Ok(required)
    }

    fn const_value(&mut self) -> PResult<f64> {
        let negative = self.eat_punct("-")?;
        let value = match self.advance()? {
            Token::Number(value) => value,
            Token::Ident(word) => match word.as_str() {
                "true" => 1.0,
                "false" => 0.0,
                "Infinity" => f64::INFINITY,
                "NaN" => f64::NAN,
                _ => return self.error("valeur de constante invalide"),
            },
            _ => return self.error("valeur de constante attendue"),
        };
        Ok(if negative { -value } else { value })
    }

    /// Saute une déclaration générique (`iterable<…>;`, `maplike<…>;`).
    fn skip_to_semicolon(&mut self) -> PResult<()> {
        loop {
            match self.advance()? {
                Token::Punct(";") => return Ok(()),
                Token::Eof => return self.error("« ; » attendu"),
                _ => {}
            }
        }
    }

    fn member(&mut self) -> PResult<Option<Member>> {
        self.extended_attributes()?;
        if self.eat_word("const")? {
            self.ty(0)?;
            let name = self.identifier()?;
            self.expect_punct("=")?;
            let value = self.const_value()?;
            self.expect_punct(";")?;
            return Ok(Some(Member::Const { name, value }));
        }
        if self.eat_word("constructor")? {
            self.expect_punct("(")?;
            let length = self.arguments()?;
            self.expect_punct(";")?;
            return Ok(Some(Member::Constructor { length }));
        }
        if ["iterable", "maplike", "setlike", "async"]
            .iter()
            .any(|word| self.is_word(word))
        {
            self.skip_to_semicolon()?;
            return Ok(None);
        }
        let is_static = self.eat_word("static")?;
        let stringifier = self.eat_word("stringifier")?;
        if stringifier && self.eat_punct(";")? {
            return Ok(Some(Member::Operation {
                name: "toString".to_owned(),
                returns: "DOMString".to_owned(),
                length: 0,
                is_static: false,
            }));
        }
        self.eat_word("inherit")?;
        let readonly = self.eat_word("readonly")?;
        if readonly && (self.is_word("maplike") || self.is_word("setlike")) {
            self.skip_to_semicolon()?;
            return Ok(None);
        }
        if self.eat_word("attribute")? {
            let ty = self.ty(0)?;
            let name = self.identifier()?;
            self.expect_punct(";")?;
            return Ok(Some(Member::Attribute {
                name,
                ty,
                readonly,
                is_static,
            }));
        }
        if readonly {
            return self.error("« attribute » attendu après « readonly »");
        }
        // Opération, éventuellement spéciale (`getter`, `setter`, `deleter`).
        let mut special = false;
        while ["getter", "setter", "deleter"]
            .iter()
            .any(|word| self.is_word(word))
        {
            self.advance()?;
            special = true;
        }
        let returns = self.ty(0)?;
        let name = if self.is_punct("(") {
            None
        } else {
            Some(self.identifier()?)
        };
        self.expect_punct("(")?;
        let length = self.arguments()?;
        self.expect_punct(";")?;
        match name {
            Some(name) => Ok(Some(Member::Operation {
                name,
                returns,
                length,
                is_static,
            })),
            None if special => Ok(None),
            None => self.error("nom d'opération attendu"),
        }
    }

    fn interface_body(
        &mut self,
        name: String,
        kind: InterfaceKind,
        partial: bool,
    ) -> PResult<Interface> {
        let parent = if self.eat_punct(":")? {
            Some(self.identifier()?)
        } else {
            None
        };
        self.expect_punct("{")?;
        let mut members = Vec::new();
        while !self.eat_punct("}")? {
            if self.token == Token::Eof {
                return self.error("« } » attendu");
            }
            members.extend(self.member()?);
        }
        self.expect_punct(";")?;
        Ok(Interface {
            name,
            parent,
            kind,
            partial,
            members,
        })
    }

    /// Saute un bloc `{ … };` (dictionnaire, énumération).
    fn skip_block(&mut self) -> PResult<()> {
        let mut depth = 0_usize;
        loop {
            match self.advance()? {
                Token::Punct("{") => depth += 1,
                Token::Punct("}") => depth = depth.saturating_sub(1),
                Token::Punct(";") if depth == 0 => return Ok(()),
                Token::Eof => return self.error("définition non fermée"),
                _ => {}
            }
        }
    }

    fn definition(&mut self) -> PResult<Definition> {
        self.extended_attributes()?;
        let partial = self.eat_word("partial")?;
        if self.eat_word("callback")? {
            if self.eat_word("interface")? {
                let name = self.identifier()?;
                return self
                    .interface_body(name, InterfaceKind::Callback, partial)
                    .map(Definition::Interface);
            }
            let name = self.identifier()?;
            self.skip_to_semicolon()?;
            return Ok(Definition::Other { name });
        }
        if self.eat_word("interface")? {
            let kind = if self.eat_word("mixin")? {
                InterfaceKind::Mixin
            } else {
                InterfaceKind::Interface
            };
            let name = self.identifier()?;
            return self
                .interface_body(name, kind, partial)
                .map(Definition::Interface);
        }
        if self.eat_word("namespace")? {
            let name = self.identifier()?;
            return self
                .interface_body(name, InterfaceKind::Namespace, partial)
                .map(Definition::Interface);
        }
        if self.eat_word("dictionary")? || self.eat_word("enum")? {
            let name = self.identifier()?;
            self.skip_block()?;
            return Ok(Definition::Other { name });
        }
        if self.eat_word("typedef")? {
            self.ty(0)?;
            let name = self.identifier()?;
            self.expect_punct(";")?;
            return Ok(Definition::Other { name });
        }
        // `Cible includes Mixin;`
        let target = self.identifier()?;
        if !self.eat_word("includes")? {
            return self.error(format!("définition inconnue : « {target} »"));
        }
        let mixin = self.identifier()?;
        self.expect_punct(";")?;
        Ok(Definition::Includes { target, mixin })
    }
}

/// Analyse un fichier Web IDL.
///
/// # Errors
/// [`IdlError`] à la première erreur de syntaxe.
pub fn parse(source: &str) -> Result<Vec<Definition>, IdlError> {
    let mut lexer = Lexer {
        chars: source.chars().peekable(),
        line: 1,
        token_line: 1,
    };
    let token = lexer.next()?;
    let line = lexer.token_line;
    let mut parser = Parser { lexer, token, line };
    let mut definitions = Vec::new();
    while parser.token != Token::Eof {
        definitions.push(parser.definition()?);
    }
    Ok(definitions)
}

/// Ajoute `member` à `members`. Les surcharges d'une opération sont
/// fusionnées en gardant le plus petit nombre d'arguments requis.
fn add_member(members: &mut Vec<Member>, member: &Member) {
    if let Member::Operation {
        name,
        length,
        is_static,
        ..
    } = member
    {
        for existing in members.iter_mut() {
            if let Member::Operation {
                name: n,
                length: l,
                is_static: s,
                ..
            } = existing
                && n == name
                && s == is_static
            {
                *l = (*l).min(*length);
                return;
            }
        }
    }
    members.push(member.clone());
}

/// Fusionne les définitions partielles et les mixins dans leurs interfaces.
/// Renvoie les interfaces (et espaces de noms) complets, dans l'ordre de leur
/// première définition.
///
/// # Errors
/// [`IdlError`] (ligne 0) si une définition partielle, un `includes` ou un
/// héritage désigne une interface absente, ou si une interface est définie
/// deux fois.
pub fn resolve(definitions: &[Definition]) -> Result<Vec<Interface>, IdlError> {
    let error = |message: String| IdlError { line: 0, message };
    let interfaces = || {
        definitions.iter().filter_map(|d| match d {
            Definition::Interface(interface) => Some(interface),
            _ => None,
        })
    };
    let mut resolved: Vec<Interface> = Vec::new();
    for interface in interfaces().filter(|i| !i.partial && i.kind != InterfaceKind::Mixin) {
        if resolved.iter().any(|r| r.name == interface.name) {
            return Err(error(format!(
                "interface {} définie deux fois",
                interface.name
            )));
        }
        let mut merged = interface.clone();
        merged.members.clear();
        for member in &interface.members {
            add_member(&mut merged.members, member);
        }
        resolved.push(merged);
    }
    // Membres d'un mixin, définitions partielles comprises.
    let mixin_members = |name: &str| -> Option<Vec<Member>> {
        let mut found = false;
        let mut members = Vec::new();
        for mixin in interfaces().filter(|i| i.kind == InterfaceKind::Mixin && i.name == name) {
            found |= !mixin.partial;
            members.extend(mixin.members.iter().cloned());
        }
        found.then_some(members)
    };
    for partial in interfaces().filter(|i| i.partial && i.kind != InterfaceKind::Mixin) {
        let Some(target) = resolved.iter_mut().find(|r| r.name == partial.name) else {
            return Err(error(format!(
                "définition partielle d'une interface inconnue : {}",
                partial.name
            )));
        };
        for member in &partial.members {
            add_member(&mut target.members, member);
        }
    }
    for definition in definitions {
        let Definition::Includes { target, mixin } = definition else {
            continue;
        };
        let Some(members) = mixin_members(mixin) else {
            return Err(error(format!("mixin inconnu : {mixin}")));
        };
        let Some(target) = resolved.iter_mut().find(|r| r.name == *target) else {
            return Err(error(format!(
                "« {target} includes {mixin} » : interface inconnue"
            )));
        };
        for member in &members {
            add_member(&mut target.members, member);
        }
    }
    for interface in &resolved {
        if let Some(parent) = &interface.parent
            && !resolved.iter().any(|r| r.name == *parent)
        {
            return Err(error(format!(
                "{} hérite d'une interface inconnue : {parent}",
                interface.name
            )));
        }
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
        // Extrait inspiré du standard DOM.
        [Exposed=*]
        interface EventTarget {
          constructor();
          undefined addEventListener(DOMString type, EventListener? callback,
                                     optional (AddEventListenerOptions or boolean) options = {});
          boolean dispatchEvent(Event event);
        };
        callback interface EventListener { undefined handleEvent(Event event); };
        dictionary AddEventListenerOptions : EventListenerOptions { boolean passive; };
        enum Mode { "open", "closed" };
        typedef (sequence<DOMString> or record<DOMString, long long>)? Bag;
        callback Visitor = undefined (Node node, optional unsigned long index = 0);

        [Exposed=Window]
        interface Node : EventTarget {
          const unsigned short ELEMENT_NODE = 1;
          const long NEGATIVE = -0x10;
          readonly attribute DOMString nodeName;
          [CEReactions] attribute DOMString? textContent;
          static readonly attribute boolean supported;
          [CEReactions] Node appendChild(Node node);
          Node cloneNode(optional boolean subtree = false);
          undefined log(any... values);
          getter Node? (unsigned long index);
          stringifier;
          iterable<Node>;
        };
        partial interface Node { undefined extra(long a, long b); };
        interface mixin ChildNode {
          [CEReactions, Unscopable] undefined remove();
          undefined before((Node or DOMString)... nodes);
        };
        partial interface mixin ChildNode { readonly attribute long position; };
        Node includes ChildNode;
        interface Text : Node { constructor(optional DOMString data = ""); };
        namespace CSS { boolean supports(DOMString property, DOMString value); boolean supports(DOMString text); };
    "#;

    fn interface<'a>(interfaces: &'a [Interface], name: &str) -> &'a Interface {
        interfaces
            .iter()
            .find(|i| i.name == name)
            .expect("interface présente")
    }

    fn operation(interface: &Interface, wanted: &str) -> Option<u32> {
        interface.members.iter().find_map(|m| match m {
            Member::Operation { name, length, .. } if name == wanted => Some(*length),
            _ => None,
        })
    }

    #[test]
    fn definitions_are_parsed_and_merged() {
        let definitions = parse(SAMPLE).expect("IDL valide");
        let interfaces = resolve(&definitions).expect("définitions cohérentes");
        let names: Vec<&str> = interfaces.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(
            names,
            ["EventTarget", "EventListener", "Node", "Text", "CSS"]
        );

        let target = interface(&interfaces, "EventTarget");
        assert_eq!(target.parent, None);
        assert!(target.members.contains(&Member::Constructor { length: 0 }));
        assert_eq!(operation(target, "addEventListener"), Some(2));

        let node = interface(&interfaces, "Node");
        assert_eq!(node.parent.as_deref(), Some("EventTarget"));
        assert!(node.members.contains(&Member::Const {
            name: "ELEMENT_NODE".to_owned(),
            value: 1.0
        }));
        assert!(node.members.contains(&Member::Const {
            name: "NEGATIVE".to_owned(),
            value: -16.0
        }));
        assert!(node.members.contains(&Member::Attribute {
            name: "textContent".to_owned(),
            ty: "DOMString?".to_owned(),
            readonly: false,
            is_static: false
        }));
        assert!(node.members.contains(&Member::Attribute {
            name: "supported".to_owned(),
            ty: "boolean".to_owned(),
            readonly: true,
            is_static: true
        }));
        assert_eq!(operation(node, "appendChild"), Some(1));
        assert_eq!(operation(node, "cloneNode"), Some(0));
        assert_eq!(operation(node, "log"), Some(0));
        assert_eq!(operation(node, "toString"), Some(0));
        // Définition partielle et mixin (avec sa propre définition partielle).
        assert_eq!(operation(node, "extra"), Some(2));
        assert_eq!(operation(node, "remove"), Some(0));
        assert_eq!(operation(node, "before"), Some(0));
        assert!(node.members.iter().any(
            |m| matches!(m, Member::Attribute { name, readonly: true, .. } if name == "position")
        ));

        assert!(
            interface(&interfaces, "Text")
                .members
                .contains(&Member::Constructor { length: 0 })
        );
        // Surcharges : le plus petit nombre d'arguments requis.
        let css = interface(&interfaces, "CSS");
        assert_eq!(css.kind, InterfaceKind::Namespace);
        assert_eq!(operation(css, "supports"), Some(1));
    }

    #[test]
    fn types_are_normalized() {
        let definitions = parse(
            "interface A { attribute (sequence<unsigned long long>? or Promise<undefined>) x;
               attribute unrestricted double y; attribute record<DOMString, [Clamp] long> z; };",
        )
        .expect("IDL valide");
        let Definition::Interface(a) = &definitions[0] else {
            panic!("interface attendue");
        };
        let types: Vec<&str> = a
            .members
            .iter()
            .filter_map(|m| match m {
                Member::Attribute { ty, .. } => Some(ty.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            types,
            [
                "(sequence<unsigned long long>? or Promise<undefined>)",
                "unrestricted double",
                "record<DOMString, long>"
            ]
        );
    }

    #[test]
    fn errors_are_reported_with_their_line() {
        let error = parse("interface A {\n  attribute long;\n};").expect_err("nom manquant");
        assert_eq!(error.line, 2);
        for source in [
            "interface A {",
            "interface A { readonly long x; };",
            "interface { };",
            "interface A { attribute (long) x; };",
            "truc A;",
            "interface A { const long X = \"a\"; };",
            "/* jamais fermé",
            "interface A { long f(long a };",
        ] {
            assert!(parse(source).is_err(), "{source:?}");
        }
        let nested = format!(
            "interface A {{ attribute {}long{} x; }};",
            "sequence<".repeat(64),
            ">".repeat(64)
        );
        assert!(parse(&nested).is_err());
        for source in [
            "partial interface A { };",
            "interface A { }; A includes M;",
            "interface mixin M { }; A includes M;",
            "interface A : B { };",
            "interface A { }; interface A { };",
        ] {
            let definitions = parse(source).expect("syntaxe valide");
            assert!(resolve(&definitions).is_err(), "{source:?}");
        }
    }
}
