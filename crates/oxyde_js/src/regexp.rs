//! Moteur d'expressions rationnelles d'ECMAScript (équivalent d'Irregexp).
//!
//! Le motif est analysé en arbre, compilé en un petit bytecode, puis exécuté
//! par retour sur trace avec une pile explicite : la profondeur de la pile
//! native ne dépend pas de la longueur du texte. Chaque recherche est bornée
//! en nombre de pas et en taille de pile ([`MatchError`]), ce qui neutralise
//! les motifs à coût exponentiel.
//!
//! LIMITES CONNUES : le texte est parcouru par points de code, y compris sans
//! le drapeau `u` (un caractère hors du plan de base compte pour un seul
//! « . ») ; un demi-substitut isolé (`\uD83D` seul) n'est pas représentable ;
//! la comparaison insensible à la casse suit les correspondances simples
//! d'Unicode ; le drapeau `v` n'existe pas et `d` est accepté sans effet.
//!
//! Les propriétés Unicode de `\p{…}` (catégories générales, écritures,
//! propriétés binaires) viennent des tables d'ICU4X.

use icu_properties::props::{GeneralCategory, GeneralCategoryGroup, Script};
use icu_properties::script::ScriptWithExtensions;
use icu_properties::{
    CodePointMapData, CodePointSetData, CodePointSetDataBorrowed, PropertyParser,
};

/// Nombre maximal de pas d'exécution pour une recherche.
const STEP_LIMIT: u64 = 20_000_000;
/// Taille maximale de la pile de retour sur trace.
const STACK_LIMIT: usize = 2_000_000;
/// Taille maximale d'un programme compilé.
const PROGRAM_LIMIT: usize = 200_000;
/// Profondeur maximale d'imbrication des groupes.
const DEPTH_LIMIT: usize = 200;

/// Drapeaux d'une expression rationnelle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Flags {
    pub(crate) indices: bool,
    pub(crate) global: bool,
    pub(crate) ignore_case: bool,
    pub(crate) multiline: bool,
    pub(crate) dot_all: bool,
    pub(crate) unicode: bool,
    pub(crate) sticky: bool,
}

impl Flags {
    /// Analyse une chaîne de drapeaux ; refuse les doublons et les inconnus.
    pub(crate) fn parse(text: &str) -> Result<Self, String> {
        let mut flags = Self::default();
        for c in text.chars() {
            let slot = match c {
                'd' => &mut flags.indices,
                'g' => &mut flags.global,
                'i' => &mut flags.ignore_case,
                'm' => &mut flags.multiline,
                's' => &mut flags.dot_all,
                'u' => &mut flags.unicode,
                'y' => &mut flags.sticky,
                _ => return Err(format!("drapeau « {c} » invalide ou non pris en charge")),
            };
            if std::mem::replace(slot, true) {
                return Err(format!("drapeau « {c} » répété"));
            }
        }
        Ok(flags)
    }

    /// Drapeaux dans l'ordre canonique (`dgimsuy`).
    pub(crate) fn canonical(self) -> String {
        [
            (self.indices, 'd'),
            (self.global, 'g'),
            (self.ignore_case, 'i'),
            (self.multiline, 'm'),
            (self.dot_all, 's'),
            (self.unicode, 'u'),
            (self.sticky, 'y'),
        ]
        .into_iter()
        .filter_map(|(set, c)| set.then_some(c))
        .collect()
    }
}

/// La recherche a dépassé ses bornes de temps ou de mémoire.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MatchError;

/// Positions (en caractères) de la correspondance et de chaque groupe :
/// l'index 0 est la correspondance entière.
pub(crate) type Captures = Vec<Option<(usize, usize)>>;

// ---- Classes de caractères -------------------------------------------------------

/// Propriété Unicode désignée par `\p{…}`.
#[derive(Clone, Copy, Debug)]
enum Property {
    /// Tous les points de code.
    Any,
    Ascii,
    /// Tout sauf la catégorie « non assigné ».
    Assigned,
    /// Une catégorie générale, ou un groupe de catégories (`L`, `Letter`).
    Category(GeneralCategoryGroup),
    Script(Script),
    ScriptExtensions(Script),
    /// Propriété binaire (`Alphabetic`, `Emoji`…).
    Binary(CodePointSetDataBorrowed<'static>),
}

impl Property {
    /// Propriété nommée par le contenu de `\p{…}` : `Nom` ou `Nom=Valeur`,
    /// à la lettre près (ECMA-262 §22.2.2.9). `None` si elle n'existe pas.
    fn parse(text: &str) -> Option<Self> {
        let category = |value: &str| {
            PropertyParser::<GeneralCategoryGroup>::new()
                .get_strict(value)
                .map(Self::Category)
        };
        let script = |value: &str| PropertyParser::<Script>::new().get_strict(value);
        match text.split_once('=') {
            Some(("General_Category" | "gc", value)) => category(value),
            Some(("Script" | "sc", value)) => script(value).map(Self::Script),
            Some(("Script_Extensions" | "scx", value)) => script(value).map(Self::ScriptExtensions),
            Some(_) => None,
            None => match text {
                "Any" => Some(Self::Any),
                "ASCII" => Some(Self::Ascii),
                "Assigned" => Some(Self::Assigned),
                // Alias que la table d'ICU4X ne connaît pas.
                "space" => CodePointSetData::new_for_ecma262(b"White_Space").map(Self::Binary),
                // Une valeur de catégorie générale s'écrit aussi seule.
                _ => category(text).or_else(|| {
                    CodePointSetData::new_for_ecma262(text.as_bytes()).map(Self::Binary)
                }),
            },
        }
    }

    fn matches(self, c: char) -> bool {
        let category = || CodePointMapData::<GeneralCategory>::new().get(c);
        match self {
            Self::Any => true,
            Self::Ascii => c.is_ascii(),
            Self::Assigned => category() != GeneralCategory::Unassigned,
            Self::Category(group) => group.contains(category()),
            Self::Script(script) => CodePointMapData::<Script>::new().get(c) == script,
            Self::ScriptExtensions(script) => ScriptWithExtensions::new().has_script(c, script),
            Self::Binary(set) => set.contains(c),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Builtin {
    Digit,
    Word,
    Space,
    Property(Property),
}

fn is_line_terminator(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

pub(crate) fn is_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

impl Builtin {
    fn matches(self, c: char) -> bool {
        match self {
            Self::Digit => c.is_ascii_digit(),
            Self::Word => is_word(c),
            Self::Space => is_space(c),
            Self::Property(property) => property.matches(c),
        }
    }
}

#[derive(Clone, Debug, Default)]
struct ClassSet {
    negated: bool,
    ranges: Vec<(char, char)>,
    /// Classes prédéfinies ; le booléen les inverse (`\D`, `\P{…}`).
    builtins: Vec<(Builtin, bool)>,
}

/// Forme canonique d'un caractère pour la comparaison insensible à la casse
/// (ECMA-262 §22.2.5.6, sans le drapeau `u`).
fn canonical(c: char) -> char {
    if c.is_ascii() {
        return c.to_ascii_uppercase();
    }
    let mut upper = c.to_uppercase();
    match (upper.next(), upper.next()) {
        // Une lettre non ASCII ne se replie jamais sur une lettre ASCII.
        (Some(u), None) if !u.is_ascii() => u,
        _ => c,
    }
}

fn simple_lower(c: char) -> char {
    let mut lower = c.to_lowercase();
    match (lower.next(), lower.next()) {
        (Some(l), None) => l,
        _ => c,
    }
}

impl ClassSet {
    fn single(builtin: Builtin, negated: bool) -> Self {
        Self {
            negated: false,
            ranges: Vec::new(),
            builtins: vec![(builtin, negated)],
        }
    }

    fn contains(&self, c: char) -> bool {
        self.ranges.iter().any(|(lo, hi)| *lo <= c && c <= *hi)
            || self
                .builtins
                .iter()
                .any(|(builtin, negated)| builtin.matches(c) != *negated)
    }

    fn matches(&self, c: char, ignore_case: bool) -> bool {
        let hit = self.contains(c)
            || (ignore_case && (self.contains(canonical(c)) || self.contains(simple_lower(c))));
        hit != self.negated
    }
}

// ---- Arbre syntaxique ------------------------------------------------------------

#[derive(Clone, Debug)]
enum Node {
    Empty,
    Char(char),
    Any,
    Class(ClassSet),
    Seq(Vec<Node>),
    Alt(Vec<Node>),
    /// Groupe ; `Some(n)` : capturant, de numéro `n`.
    Group(Box<Node>, Option<usize>),
    Repeat {
        node: Box<Node>,
        min: u32,
        max: Option<u32>,
        greedy: bool,
        /// Groupes capturants contenus : remis à zéro à chaque tour.
        groups: (usize, usize),
    },
    Start,
    End,
    WordBoundary {
        negated: bool,
    },
    BackRef(usize),
    Look {
        node: Box<Node>,
        behind: bool,
        negative: bool,
    },
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
    unicode: bool,
    /// Nombre total de groupes capturants du motif (connu d'avance).
    total_groups: usize,
    /// Noms des groupes, avec leur numéro.
    names: Vec<(String, usize)>,
    next_group: usize,
    depth: usize,
}

type PResult<T> = Result<T, String>;

/// Compte les groupes capturants et relève leurs noms, sans analyser le reste.
fn scan_groups(chars: &[char]) -> PResult<(usize, Vec<(String, usize)>)> {
    let mut count = 0;
    let mut names: Vec<(String, usize)> = Vec::new();
    let mut in_class = false;
    let mut i = 0;
    while let Some(&c) = chars.get(i) {
        match c {
            '\\' => i += 1,
            '[' => in_class = true,
            ']' => in_class = false,
            '(' if !in_class => {
                if chars.get(i + 1) != Some(&'?') {
                    count += 1;
                } else if chars.get(i + 2) == Some(&'<')
                    && !matches!(chars.get(i + 3), Some('=' | '!'))
                {
                    count += 1;
                    let name: String = chars
                        .get(i + 3..)
                        .unwrap_or(&[])
                        .iter()
                        .take_while(|c| **c != '>')
                        .collect();
                    if names.iter().any(|(n, _)| *n == name) {
                        return Err(format!("nom de groupe « {name} » répété"));
                    }
                    names.push((name, count));
                }
            }
            _ => {}
        }
        i += 1;
    }
    Ok((count, names))
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    fn eat(&mut self, c: char) -> bool {
        if self.peek() == Some(c) {
            self.pos += 1;
            return true;
        }
        false
    }

    fn looking_at(&self, text: &str) -> bool {
        text.chars()
            .enumerate()
            .all(|(i, c)| self.peek_at(i) == Some(c))
    }

    fn disjunction(&mut self) -> PResult<Node> {
        self.depth += 1;
        if self.depth > DEPTH_LIMIT {
            return Err("groupes trop imbriqués".to_owned());
        }
        let mut alternatives = vec![self.alternative()?];
        while self.eat('|') {
            alternatives.push(self.alternative()?);
        }
        self.depth -= 1;
        Ok(if alternatives.len() == 1 {
            alternatives.pop().unwrap_or(Node::Empty)
        } else {
            Node::Alt(alternatives)
        })
    }

    fn alternative(&mut self) -> PResult<Node> {
        let mut terms = Vec::new();
        while let Some(c) = self.peek() {
            if c == '|' || c == ')' {
                break;
            }
            terms.push(self.term()?);
        }
        Ok(match terms.len() {
            0 => Node::Empty,
            1 => terms.pop().unwrap_or(Node::Empty),
            _ => Node::Seq(terms),
        })
    }

    fn term(&mut self) -> PResult<Node> {
        let first_group = self.next_group;
        let (node, quantifiable) = self.atom()?;
        let Some((min, max)) = self.quantifier()? else {
            return Ok(node);
        };
        if !quantifiable {
            return Err("rien à répéter".to_owned());
        }
        let greedy = !self.eat('?');
        Ok(Node::Repeat {
            node: Box::new(node),
            min,
            max,
            greedy,
            groups: (first_group, self.next_group),
        })
    }

    /// Lit un quantificateur s'il y en a un : (minimum, maximum).
    fn quantifier(&mut self) -> PResult<Option<(u32, Option<u32>)>> {
        let bounds = match self.peek() {
            Some('*') => (0, None),
            Some('+') => (1, None),
            Some('?') => (0, Some(1)),
            Some('{') => {
                let Some((bounds, len)) = self.braces() else {
                    if self.unicode {
                        return Err("quantificateur incomplet".to_owned());
                    }
                    return Ok(None);
                };
                self.pos += len - 1;
                bounds
            }
            _ => return Ok(None),
        };
        self.pos += 1;
        if let (min, Some(max)) = bounds
            && min > max
        {
            return Err("quantificateur dans le désordre".to_owned());
        }
        Ok(Some(bounds))
    }

    /// Reconnaît `{n}`, `{n,}` ou `{n,m}` à la position courante, sans avancer :
    /// bornes et longueur du texte lu.
    fn braces(&self) -> Option<((u32, Option<u32>), usize)> {
        let mut i = 1;
        let number = |i: &mut usize| -> Option<u32> {
            let start = *i;
            let mut value: u32 = 0;
            while let Some(d) = self.peek_at(*i).and_then(|c| c.to_digit(10)) {
                // Au-delà de 2^32, la borne est de toute façon inatteignable.
                value = value.saturating_mul(10).saturating_add(d);
                *i += 1;
            }
            (*i > start).then_some(value)
        };
        let min = number(&mut i)?;
        let max = if self.peek_at(i) == Some(',') {
            i += 1;
            if self.peek_at(i) == Some('}') {
                None
            } else {
                Some(number(&mut i)?)
            }
        } else {
            Some(min)
        };
        (self.peek_at(i) == Some('}')).then_some(((min, max), i + 1))
    }

    /// Atome ou assertion ; le booléen dit s'il peut porter un quantificateur.
    fn atom(&mut self) -> PResult<(Node, bool)> {
        let Some(c) = self.peek() else {
            return Ok((Node::Empty, false));
        };
        self.pos += 1;
        Ok(match c {
            '^' => (Node::Start, false),
            '$' => (Node::End, false),
            '.' => (Node::Any, true),
            '(' => return self.group(),
            '[' => (Node::Class(self.class()?), true),
            '\\' => return self.atom_escape(),
            '*' | '+' | '?' => return Err("rien à répéter".to_owned()),
            '{' => {
                self.pos -= 1;
                let is_quantifier = self.braces().is_some();
                self.pos += 1;
                if self.unicode || is_quantifier {
                    return Err("rien à répéter".to_owned());
                }
                (Node::Char('{'), true)
            }
            '}' | ']' if self.unicode => return Err(format!("« {c} » isolé")),
            _ => (Node::Char(c), true),
        })
    }

    fn group(&mut self) -> PResult<(Node, bool)> {
        let look = [
            ("?=", false, false),
            ("?!", false, true),
            ("?<=", true, false),
            ("?<!", true, true),
        ]
        .into_iter()
        .find(|(prefix, _, _)| self.looking_at(prefix));
        if let Some((prefix, behind, negative)) = look {
            self.pos += prefix.len();
            let node = self.disjunction()?;
            self.close_group()?;
            return Ok((
                Node::Look {
                    node: Box::new(node),
                    behind,
                    negative,
                },
                // Seule une assertion avant peut être répétée (annexe B).
                !behind && !self.unicode,
            ));
        }
        let index = if self.looking_at("?:") {
            self.pos += 2;
            None
        } else if self.looking_at("?<") {
            self.pos += 2;
            let name = self.group_name()?;
            if name.is_empty() {
                return Err("nom de groupe vide".to_owned());
            }
            self.next_group += 1;
            Some(self.next_group)
        } else if self.peek() == Some('?') {
            return Err("groupe invalide".to_owned());
        } else {
            self.next_group += 1;
            Some(self.next_group)
        };
        let node = self.disjunction()?;
        self.close_group()?;
        Ok((Node::Group(Box::new(node), index), true))
    }

    fn close_group(&mut self) -> PResult<()> {
        if self.eat(')') {
            Ok(())
        } else {
            Err("parenthèse non fermée".to_owned())
        }
    }

    /// Lit `nom>` (le `<` est déjà consommé).
    fn group_name(&mut self) -> PResult<String> {
        let mut name = String::new();
        loop {
            match self.peek() {
                Some('>') => {
                    self.pos += 1;
                    return Ok(name);
                }
                Some(c) if c.is_alphanumeric() || c == '_' || c == '$' => {
                    name.push(c);
                    self.pos += 1;
                }
                _ => return Err("nom de groupe invalide".to_owned()),
            }
        }
    }

    fn hex(&mut self, digits: usize) -> Option<u32> {
        let mut value = 0;
        for i in 0..digits {
            value = value * 16 + self.peek_at(i)?.to_digit(16)?;
        }
        self.pos += digits;
        Some(value)
    }

    /// Suite d'un `\u` : `XXXX` (paire de substituts comprise) ou `{X…}`.
    fn unicode_escape(&mut self) -> PResult<Option<char>> {
        if self.unicode && self.peek() == Some('{') {
            let mut value: u32 = 0;
            let mut i = 1;
            while let Some(d) = self.peek_at(i).and_then(|c| c.to_digit(16)) {
                value = value.saturating_mul(16).saturating_add(d);
                i += 1;
            }
            if i == 1 || self.peek_at(i) != Some('}') {
                return Err("échappement Unicode invalide".to_owned());
            }
            self.pos += i + 1;
            return char::from_u32(value)
                .map(Some)
                .ok_or_else(|| "point de code hors limites".to_owned());
        }
        let Some(unit) = self.hex(4) else {
            if self.unicode {
                return Err("échappement Unicode invalide".to_owned());
            }
            return Ok(None);
        };
        if (0xD800..0xDC00).contains(&unit) && self.looking_at("\\u") {
            let saved = self.pos;
            self.pos += 2;
            if let Some(low) = self.hex(4)
                && (0xDC00..0xE000).contains(&low)
            {
                let code = 0x10000 + ((unit - 0xD800) << 10) + (low - 0xDC00);
                return Ok(char::from_u32(code));
            }
            self.pos = saved;
        }
        // LIMITE : un demi-substitut isolé devient U+FFFD.
        Ok(Some(char::from_u32(unit).unwrap_or('\u{fffd}')))
    }

    fn property(&mut self, negated: bool) -> PResult<ClassSet> {
        if !self.eat('{') {
            return Err("propriété Unicode invalide".to_owned());
        }
        let mut name = String::new();
        loop {
            match self.peek() {
                Some('}') => {
                    self.pos += 1;
                    break;
                }
                Some(c) => {
                    name.push(c);
                    self.pos += 1;
                }
                None => return Err("propriété Unicode invalide".to_owned()),
            }
        }
        let Some(property) = Property::parse(&name) else {
            return Err(format!("propriété Unicode « {name} » inconnue"));
        };
        Ok(ClassSet::single(Builtin::Property(property), negated))
    }

    /// Échappement commun aux atomes et aux classes ; `None` : pas un caractère.
    fn char_escape(&mut self, c: char) -> PResult<char> {
        Ok(match c {
            't' => '\t',
            'n' => '\n',
            'v' => '\u{b}',
            'f' => '\u{c}',
            'r' => '\r',
            'c' => match self.peek() {
                Some(letter) if letter.is_ascii_alphabetic() => {
                    self.pos += 1;
                    char::from_u32(u32::from(letter) % 32).unwrap_or('\0')
                }
                _ if self.unicode => return Err("échappement de contrôle invalide".to_owned()),
                _ => {
                    // Annexe B : `\c` suivi d'autre chose est une barre oblique littérale.
                    self.pos -= 1;
                    '\\'
                }
            },
            'x' => match self.hex(2) {
                Some(value) => char::from_u32(value).unwrap_or('\0'),
                None if self.unicode => return Err("échappement invalide".to_owned()),
                None => 'x',
            },
            'u' => self.unicode_escape()?.unwrap_or('u'),
            '0'..='7' if !self.unicode => {
                // Annexe B : échappement octal, au plus trois chiffres et 0o377.
                let mut value = c.to_digit(8).unwrap_or(0);
                for _ in 0..2 {
                    match self.peek().and_then(|d| d.to_digit(8)) {
                        Some(d) if value * 8 + d <= 0o377 => {
                            value = value * 8 + d;
                            self.pos += 1;
                        }
                        _ => break,
                    }
                }
                char::from_u32(value).unwrap_or('\0')
            }
            '0' if !self.peek().is_some_and(|d| d.is_ascii_digit()) => '\0',
            // Avec `u`, seuls les caractères de syntaxe peuvent être échappés.
            _ if self.unicode && !r"^$\.*+?()[]{}|/".contains(c) => {
                return Err(format!("échappement « \\{c} » invalide"));
            }
            _ => c,
        })
    }

    fn atom_escape(&mut self) -> PResult<(Node, bool)> {
        let Some(c) = self.peek() else {
            return Err("« \\ » en fin de motif".to_owned());
        };
        self.pos += 1;
        let class = |builtin, negated| Ok((Node::Class(ClassSet::single(builtin, negated)), true));
        match c {
            'b' => return Ok((Node::WordBoundary { negated: false }, false)),
            'B' => return Ok((Node::WordBoundary { negated: true }, false)),
            'd' => return class(Builtin::Digit, false),
            'D' => return class(Builtin::Digit, true),
            'w' => return class(Builtin::Word, false),
            'W' => return class(Builtin::Word, true),
            's' => return class(Builtin::Space, false),
            'S' => return class(Builtin::Space, true),
            'p' | 'P' if self.unicode => {
                return Ok((Node::Class(self.property(c == 'P')?), true));
            }
            'k' if self.unicode || !self.names.is_empty() => {
                if !self.eat('<') {
                    return Err("référence nommée invalide".to_owned());
                }
                let name = self.group_name()?;
                let Some((_, index)) = self.names.iter().find(|(n, _)| *n == name) else {
                    return Err(format!("groupe « {name} » inconnu"));
                };
                return Ok((Node::BackRef(*index), true));
            }
            '1'..='9' => {
                // Référence arrière si le groupe existe ; sinon (annexe B)
                // échappement octal ou caractère littéral.
                let start = self.pos - 1;
                let mut value = c.to_digit(10).unwrap_or(0) as usize;
                while let Some(d) = self.peek().and_then(|d| d.to_digit(10)) {
                    value = value.saturating_mul(10).saturating_add(d as usize);
                    self.pos += 1;
                }
                if value <= self.total_groups {
                    return Ok((Node::BackRef(value), true));
                }
                if self.unicode {
                    return Err("référence à un groupe inexistant".to_owned());
                }
                self.pos = start + 1;
            }
            _ => {}
        }
        Ok((Node::Char(self.char_escape(c)?), true))
    }

    /// Élément d'une classe : caractère seul ou classe prédéfinie.
    fn class_atom(&mut self) -> PResult<Result<char, ClassSet>> {
        let Some(c) = self.peek() else {
            return Err("classe de caractères non fermée".to_owned());
        };
        self.pos += 1;
        if c != '\\' {
            return Ok(Ok(c));
        }
        let Some(escaped) = self.peek() else {
            return Err("« \\ » en fin de motif".to_owned());
        };
        self.pos += 1;
        let class = |builtin, negated| Ok(Err(ClassSet::single(builtin, negated)));
        match escaped {
            'd' => class(Builtin::Digit, false),
            'D' => class(Builtin::Digit, true),
            'w' => class(Builtin::Word, false),
            'W' => class(Builtin::Word, true),
            's' => class(Builtin::Space, false),
            'S' => class(Builtin::Space, true),
            'p' | 'P' if self.unicode => Ok(Err(self.property(escaped == 'P')?)),
            'b' => Ok(Ok('\u{8}')),
            '-' => Ok(Ok('-')),
            _ => self.char_escape(escaped).map(Ok),
        }
    }

    fn class(&mut self) -> PResult<ClassSet> {
        let mut set = ClassSet {
            negated: self.eat('^'),
            ..ClassSet::default()
        };
        loop {
            if self.eat(']') {
                return Ok(set);
            }
            let low = match self.class_atom()? {
                Ok(c) => c,
                Err(inner) => {
                    // Une classe prédéfinie ne borne pas un intervalle.
                    if self.unicode
                        && self.peek() == Some('-')
                        && !matches!(self.peek_at(1), Some(']') | None)
                    {
                        return Err("intervalle de classe invalide".to_owned());
                    }
                    set.builtins.extend(inner.builtins);
                    continue;
                }
            };
            // Intervalle `a-z` : le tiret doit être suivi d'un caractère seul.
            if self.peek() == Some('-') && !matches!(self.peek_at(1), Some(']') | None) {
                let saved = self.pos;
                self.pos += 1;
                match self.class_atom()? {
                    Ok(high) => {
                        if low > high {
                            return Err("intervalle de classe dans le désordre".to_owned());
                        }
                        set.ranges.push((low, high));
                        continue;
                    }
                    Err(_) if self.unicode => {
                        return Err("intervalle de classe invalide".to_owned());
                    }
                    // Annexe B : `[a-\d]` contient `a`, `-` et les chiffres.
                    Err(_) => self.pos = saved,
                }
            }
            set.ranges.push((low, low));
        }
    }
}

// ---- Bytecode --------------------------------------------------------------------

#[derive(Clone, Debug)]
enum Inst {
    Char {
        c: char,
        back: bool,
    },
    Any {
        back: bool,
    },
    Class {
        set: usize,
        back: bool,
    },
    Save(usize),
    /// Essaie `first`, puis `second` en cas d'échec.
    Split {
        first: usize,
        second: usize,
    },
    Jmp(usize),
    Start,
    End,
    WordBoundary {
        negated: bool,
    },
    BackRef {
        group: usize,
        back: bool,
    },
    SetReg {
        reg: usize,
        value: usize,
    },
    /// Répétition d'un atome d'un seul caractère (`a*`, `.+`, `\d{2,}`) :
    /// l'atome est l'instruction suivante, la suite du motif vient après.
    /// Les tours ne s'empilent pas : un seul point de choix les représente.
    Span {
        min: u32,
        max: Option<u32>,
        greedy: bool,
        back: bool,
    },
    /// Tête de boucle : décide, d'après le compteur `reg`, de refaire un tour
    /// (instruction suivante) ou de sortir (`exit`).
    LoopHead {
        reg: usize,
        min: u32,
        max: Option<u32>,
        greedy: bool,
        exit: usize,
    },
    /// Efface les captures des groupes `from..to` (exclus).
    ClearCaps {
        from: usize,
        to: usize,
    },
    /// Note la position courante dans `reg`.
    Mark {
        reg: usize,
    },
    /// Échoue si le tour n'a rien consommé alors que le minimum est atteint.
    EmptyCheck {
        mark: usize,
        count: usize,
        min: u32,
    },
    IncReg {
        reg: usize,
    },
    /// Assertion avant ou arrière : le corps suit et se termine par `Match` ;
    /// l'exécution reprend à `end`.
    Look {
        negative: bool,
        end: usize,
    },
    Match,
}

/// Expression rationnelle compilée.
#[derive(Debug)]
pub(crate) struct Regex {
    insts: Vec<Inst>,
    sets: Vec<ClassSet>,
    registers: usize,
    /// Nombre de groupes capturants (sans compter la correspondance entière).
    pub(crate) groups: usize,
    /// Groupes nommés : (nom, numéro).
    pub(crate) names: Vec<(String, usize)>,
    pub(crate) flags: Flags,
}

struct Compiler {
    insts: Vec<Inst>,
    sets: Vec<ClassSet>,
    registers: usize,
    ignore_case: bool,
}

impl Compiler {
    fn emit(&mut self, inst: Inst) -> PResult<usize> {
        if self.insts.len() >= PROGRAM_LIMIT {
            return Err("expression rationnelle trop grande".to_owned());
        }
        self.insts.push(inst);
        Ok(self.insts.len() - 1)
    }

    fn register(&mut self) -> usize {
        self.registers += 1;
        self.registers - 1
    }

    /// Compile `node` ; `back` : le texte est parcouru vers la gauche
    /// (corps d'une assertion arrière).
    fn node(&mut self, node: &Node, back: bool) -> PResult<()> {
        match node {
            Node::Empty => {}
            Node::Char(c) => {
                let c = if self.ignore_case { canonical(*c) } else { *c };
                self.emit(Inst::Char { c, back })?;
            }
            Node::Any => {
                self.emit(Inst::Any { back })?;
            }
            Node::Class(set) => {
                self.sets.push(set.clone());
                let set = self.sets.len() - 1;
                self.emit(Inst::Class { set, back })?;
            }
            Node::Seq(nodes) => {
                if back {
                    for node in nodes.iter().rev() {
                        self.node(node, back)?;
                    }
                } else {
                    for node in nodes {
                        self.node(node, back)?;
                    }
                }
            }
            Node::Alt(alternatives) => {
                let mut exits = Vec::new();
                let last = alternatives.len().saturating_sub(1);
                for (i, alternative) in alternatives.iter().enumerate() {
                    if i == last {
                        self.node(alternative, back)?;
                        break;
                    }
                    let split = self.emit(Inst::Split {
                        first: 0,
                        second: 0,
                    })?;
                    self.node(alternative, back)?;
                    exits.push(self.emit(Inst::Jmp(0))?);
                    let next = self.insts.len();
                    if let Some(inst) = self.insts.get_mut(split) {
                        *inst = Inst::Split {
                            first: split + 1,
                            second: next,
                        };
                    }
                }
                let end = self.insts.len();
                for exit in exits {
                    if let Some(inst) = self.insts.get_mut(exit) {
                        *inst = Inst::Jmp(end);
                    }
                }
            }
            Node::Group(inner, None) => self.node(inner, back)?,
            Node::Group(inner, Some(index)) => {
                // À reculons, la fin du groupe est rencontrée la première.
                let (first, second) = if back {
                    (index * 2 + 1, index * 2)
                } else {
                    (index * 2, index * 2 + 1)
                };
                self.emit(Inst::Save(first))?;
                self.node(inner, back)?;
                self.emit(Inst::Save(second))?;
            }
            Node::Repeat {
                node,
                min,
                max,
                greedy,
                groups,
            } => {
                if matches!(**node, Node::Char(_) | Node::Any | Node::Class(_)) {
                    self.emit(Inst::Span {
                        min: *min,
                        max: *max,
                        greedy: *greedy,
                        back,
                    })?;
                    return self.node(node, back);
                }
                let count = self.register();
                let mark = self.register();
                self.emit(Inst::SetReg {
                    reg: count,
                    value: 0,
                })?;
                let head = self.emit(Inst::LoopHead {
                    reg: count,
                    min: *min,
                    max: *max,
                    greedy: *greedy,
                    exit: 0,
                })?;
                if groups.0 < groups.1 {
                    self.emit(Inst::ClearCaps {
                        from: groups.0 + 1,
                        to: groups.1 + 1,
                    })?;
                }
                self.emit(Inst::Mark { reg: mark })?;
                self.node(node, back)?;
                self.emit(Inst::EmptyCheck {
                    mark,
                    count,
                    min: *min,
                })?;
                self.emit(Inst::IncReg { reg: count })?;
                self.emit(Inst::Jmp(head))?;
                let exit = self.insts.len();
                if let Some(Inst::LoopHead { exit: target, .. }) = self.insts.get_mut(head) {
                    *target = exit;
                }
            }
            Node::Start => {
                self.emit(Inst::Start)?;
            }
            Node::End => {
                self.emit(Inst::End)?;
            }
            Node::WordBoundary { negated } => {
                self.emit(Inst::WordBoundary { negated: *negated })?;
            }
            Node::BackRef(group) => {
                self.emit(Inst::BackRef {
                    group: *group,
                    back,
                })?;
            }
            Node::Look {
                node,
                behind,
                negative,
            } => {
                let look = self.emit(Inst::Look {
                    negative: *negative,
                    end: 0,
                })?;
                self.node(node, *behind)?;
                self.emit(Inst::Match)?;
                let end = self.insts.len();
                if let Some(Inst::Look { end: target, .. }) = self.insts.get_mut(look) {
                    *target = end;
                }
            }
        }
        Ok(())
    }
}

/// Compile `pattern` avec les drapeaux `flags`.
///
/// # Errors
/// Message décrivant l'erreur de syntaxe du motif ou des drapeaux.
pub(crate) fn compile(pattern: &str, flags: &str) -> Result<Regex, String> {
    let flags = Flags::parse(flags)?;
    let chars: Vec<char> = pattern.chars().collect();
    let (total_groups, names) = scan_groups(&chars)?;
    let mut parser = Parser {
        chars,
        pos: 0,
        unicode: flags.unicode,
        total_groups,
        names,
        next_group: 0,
        depth: 0,
    };
    let node = parser.disjunction()?;
    if parser.pos < parser.chars.len() {
        return Err("parenthèse fermante en trop".to_owned());
    }
    let mut compiler = Compiler {
        insts: Vec::new(),
        sets: Vec::new(),
        registers: 0,
        ignore_case: flags.ignore_case,
    };
    compiler.emit(Inst::Save(0))?;
    compiler.node(&node, false)?;
    compiler.emit(Inst::Save(1))?;
    compiler.emit(Inst::Match)?;
    Ok(Regex {
        insts: compiler.insts,
        sets: compiler.sets,
        registers: compiler.registers,
        groups: parser.next_group,
        names: parser.names,
        flags,
    })
}

// ---- Exécution -------------------------------------------------------------------

/// Entrée de la pile de retour sur trace.
enum Undo {
    /// Point de choix : reprendre à `pc`, position `pos`.
    Branch {
        pc: usize,
        pos: usize,
    },
    Cap {
        index: usize,
        old: Option<usize>,
    },
    Reg {
        index: usize,
        old: usize,
    },
    /// Captures d'avant une assertion réussie.
    Caps(Vec<Option<usize>>),
    /// Répétition gourmande : reprendre à `pc` avec un caractère de moins,
    /// tant que `pos` n'a pas rejoint `limit`.
    Shorter {
        pc: usize,
        pos: usize,
        limit: usize,
        back: bool,
    },
    /// Répétition paresseuse d'instruction `span` : essayer un caractère de
    /// plus à partir de `pos`, `count` étant déjà pris.
    Longer {
        span: usize,
        pos: usize,
        count: u32,
    },
}

struct Machine<'a> {
    regex: &'a Regex,
    input: &'a [char],
    steps: u64,
    caps: Vec<Option<usize>>,
    regs: Vec<usize>,
}

impl Machine<'_> {
    fn same(&self, a: char, b: char) -> bool {
        a == b || (self.regex.flags.ignore_case && canonical(a) == canonical(b))
    }

    /// Caractère à consommer à `pos` dans le sens demandé, et position suivante.
    fn next_char(&self, pos: usize, back: bool) -> Option<(char, usize)> {
        if back {
            let before = pos.checked_sub(1)?;
            Some((*self.input.get(before)?, before))
        } else {
            Some((*self.input.get(pos)?, pos + 1))
        }
    }

    /// Vrai si l'atome d'un seul caractère `inst` accepte `c`.
    fn unit_matches(&self, inst: Option<&Inst>, c: char) -> bool {
        match inst {
            Some(Inst::Char { c: wanted, .. }) => {
                let found = if self.regex.flags.ignore_case {
                    canonical(c)
                } else {
                    c
                };
                found == *wanted
            }
            Some(Inst::Any { .. }) => self.regex.flags.dot_all || !is_line_terminator(c),
            Some(Inst::Class { set, .. }) => self
                .regex
                .sets
                .get(*set)
                .is_some_and(|s| s.matches(c, self.regex.flags.ignore_case)),
            _ => false,
        }
    }

    fn word_at(&self, index: Option<usize>) -> bool {
        index
            .and_then(|i| self.input.get(i))
            .is_some_and(|c| is_word(*c))
    }

    /// Exécute le programme à partir de `pc` et `pos`. `Ok(true)` : `Match`
    /// atteint, les captures sont dans `self.caps`. `Ok(false)` : échec, les
    /// captures et registres sont revenus à leur état d'entrée.
    fn run(&mut self, mut pc: usize, mut pos: usize) -> Result<bool, MatchError> {
        let mut stack: Vec<Undo> = Vec::new();
        loop {
            self.steps += 1;
            if self.steps > STEP_LIMIT || stack.len() > STACK_LIMIT {
                return Err(MatchError);
            }
            let Some(inst) = self.regex.insts.get(pc) else {
                return Ok(false);
            };
            let ok = match inst {
                Inst::Char { c, back } => match self.next_char(pos, *back) {
                    Some((found, next)) => {
                        let found = if self.regex.flags.ignore_case {
                            canonical(found)
                        } else {
                            found
                        };
                        if found == *c {
                            pos = next;
                            true
                        } else {
                            false
                        }
                    }
                    None => false,
                },
                Inst::Any { back } => match self.next_char(pos, *back) {
                    Some((found, next))
                        if self.regex.flags.dot_all || !is_line_terminator(found) =>
                    {
                        pos = next;
                        true
                    }
                    _ => false,
                },
                Inst::Class { set, back } => {
                    match self.next_char(pos, *back) {
                        Some((found, next))
                            if self.regex.sets.get(*set).is_some_and(|s| {
                                s.matches(found, self.regex.flags.ignore_case)
                            }) =>
                        {
                            pos = next;
                            true
                        }
                        _ => false,
                    }
                }
                Inst::Save(index) => {
                    if let Some(slot) = self.caps.get_mut(*index) {
                        stack.push(Undo::Cap {
                            index: *index,
                            old: slot.replace(pos),
                        });
                    }
                    true
                }
                Inst::Split { first, second } => {
                    stack.push(Undo::Branch { pc: *second, pos });
                    pc = *first;
                    continue;
                }
                Inst::Jmp(target) => {
                    pc = *target;
                    continue;
                }
                Inst::Start => {
                    pos == 0
                        || (self.regex.flags.multiline
                            && self
                                .input
                                .get(pos - 1)
                                .is_some_and(|c| is_line_terminator(*c)))
                }
                Inst::End => {
                    pos == self.input.len()
                        || (self.regex.flags.multiline
                            && self.input.get(pos).is_some_and(|c| is_line_terminator(*c)))
                }
                Inst::WordBoundary { negated } => {
                    let boundary = self.word_at(pos.checked_sub(1)) != self.word_at(Some(pos));
                    boundary != *negated
                }
                Inst::BackRef { group, back } => {
                    let start = self.caps.get(group * 2).copied().flatten();
                    let end = self.caps.get(group * 2 + 1).copied().flatten();
                    match (start, end) {
                        // Un groupe qui n'a pas participé correspond au vide.
                        (Some(start), Some(end)) if start <= end => {
                            let len = end - start;
                            let from = if *back {
                                pos.checked_sub(len)
                            } else {
                                Some(pos)
                            };
                            let matched = from.is_some_and(|from| {
                                from + len <= self.input.len()
                                    && (0..len).all(|i| {
                                        match (self.input.get(start + i), self.input.get(from + i))
                                        {
                                            (Some(a), Some(b)) => self.same(*a, *b),
                                            _ => false,
                                        }
                                    })
                            });
                            if matched {
                                pos = if *back { pos - len } else { pos + len };
                            }
                            matched
                        }
                        _ => true,
                    }
                }
                Inst::SetReg { reg, value } => {
                    if let Some(slot) = self.regs.get_mut(*reg) {
                        stack.push(Undo::Reg {
                            index: *reg,
                            old: std::mem::replace(slot, *value),
                        });
                    }
                    true
                }
                Inst::Span {
                    min,
                    max,
                    greedy,
                    back,
                } => {
                    let (min, max, back) = (*min, *max, *back);
                    let unit = self.regex.insts.get(pc + 1);
                    // Gourmande : tout ce qui se présente ; paresseuse : le minimum.
                    let wanted = if *greedy { max } else { Some(min) };
                    let mut count = 0u32;
                    let mut end = pos;
                    let mut least = pos;
                    while wanted.is_none_or(|wanted| count < wanted) {
                        match self.next_char(end, back) {
                            Some((c, next)) if self.unit_matches(unit, c) => {
                                end = next;
                                count += 1;
                                if count == min {
                                    least = end;
                                }
                            }
                            _ => break,
                        }
                    }
                    self.steps += u64::from(count);
                    if count < min {
                        false
                    } else {
                        if *greedy && end != least {
                            stack.push(Undo::Shorter {
                                pc: pc + 2,
                                pos: end,
                                limit: least,
                                back,
                            });
                        } else if !*greedy && max.is_none_or(|max| count < max) {
                            stack.push(Undo::Longer {
                                span: pc,
                                pos: end,
                                count,
                            });
                        }
                        pos = end;
                        pc += 2;
                        continue;
                    }
                }
                Inst::LoopHead {
                    reg,
                    min,
                    max,
                    greedy,
                    exit,
                } => {
                    let count = self.regs.get(*reg).copied().unwrap_or(0);
                    let count = u32::try_from(count).unwrap_or(u32::MAX);
                    if count < *min {
                        pc += 1;
                    } else if max.is_some_and(|max| count >= max) {
                        pc = *exit;
                    } else if *greedy {
                        stack.push(Undo::Branch { pc: *exit, pos });
                        pc += 1;
                    } else {
                        stack.push(Undo::Branch { pc: pc + 1, pos });
                        pc = *exit;
                    }
                    continue;
                }
                Inst::ClearCaps { from, to } => {
                    for index in from * 2..to * 2 {
                        if let Some(slot) = self.caps.get_mut(index)
                            && slot.is_some()
                        {
                            stack.push(Undo::Cap {
                                index,
                                old: slot.take(),
                            });
                        }
                    }
                    true
                }
                Inst::Mark { reg } => {
                    if let Some(slot) = self.regs.get_mut(*reg) {
                        stack.push(Undo::Reg {
                            index: *reg,
                            old: std::mem::replace(slot, pos),
                        });
                    }
                    true
                }
                Inst::EmptyCheck { mark, count, min } => {
                    let started = self.regs.get(*mark).copied().unwrap_or(usize::MAX);
                    let done = self.regs.get(*count).copied().unwrap_or(0);
                    !(started == pos && done >= *min as usize)
                }
                Inst::IncReg { reg } => {
                    if let Some(slot) = self.regs.get_mut(*reg) {
                        let old = *slot;
                        *slot = old.saturating_add(1);
                        stack.push(Undo::Reg { index: *reg, old });
                    }
                    true
                }
                Inst::Look { negative, end } => {
                    let (negative, end) = (*negative, *end);
                    let before = self.caps.clone();
                    let matched = self.run(pc + 1, pos)?;
                    if matched && !negative {
                        // Les captures de l'assertion sont gardées, et rendues
                        // si l'on revient en arrière au-delà d'elle.
                        stack.push(Undo::Caps(before));
                    } else if matched {
                        self.caps = before;
                    }
                    if matched != negative {
                        pc = end;
                        continue;
                    }
                    false
                }
                Inst::Match => return Ok(true),
            };
            if ok {
                pc += 1;
                continue;
            }
            // Échec : défaire jusqu'au dernier point de choix.
            loop {
                match stack.pop() {
                    Some(Undo::Branch {
                        pc: target,
                        pos: saved,
                    }) => {
                        pc = target;
                        pos = saved;
                        break;
                    }
                    Some(Undo::Cap { index, old }) => {
                        if let Some(slot) = self.caps.get_mut(index) {
                            *slot = old;
                        }
                    }
                    Some(Undo::Reg { index, old }) => {
                        if let Some(slot) = self.regs.get_mut(index) {
                            *slot = old;
                        }
                    }
                    Some(Undo::Caps(old)) => self.caps = old,
                    Some(Undo::Shorter {
                        pc: target,
                        pos: longest,
                        limit,
                        back,
                    }) => {
                        // Un caractère de moins ; `longest` diffère de `limit`.
                        let shorter = if back {
                            longest.saturating_add(1)
                        } else {
                            longest.saturating_sub(1)
                        };
                        if shorter != limit {
                            stack.push(Undo::Shorter {
                                pc: target,
                                pos: shorter,
                                limit,
                                back,
                            });
                        }
                        self.steps += 1;
                        pc = target;
                        pos = shorter;
                        break;
                    }
                    Some(Undo::Longer {
                        span,
                        pos: shortest,
                        count,
                    }) => {
                        let Some(Inst::Span { max, back, .. }) = self.regex.insts.get(span) else {
                            continue;
                        };
                        let unit = self.regex.insts.get(span + 1);
                        self.steps += 1;
                        // Un caractère de plus, s'il convient ; sinon l'échec continue.
                        if let Some((c, next)) = self.next_char(shortest, *back)
                            && self.unit_matches(unit, c)
                        {
                            let count = count.saturating_add(1);
                            if max.is_none_or(|max| count < max) {
                                stack.push(Undo::Longer {
                                    span,
                                    pos: next,
                                    count,
                                });
                            }
                            pc = span + 2;
                            pos = next;
                            break;
                        }
                    }
                    None => return Ok(false),
                }
            }
        }
    }
}

impl Regex {
    /// Cherche une correspondance dans `input` à partir de la position `from`
    /// (en caractères). `sticky` : seule la position `from` est essayée.
    ///
    /// Renvoie aussi le nombre de pas dépensés, à imputer au budget du script.
    ///
    /// # Errors
    /// [`MatchError`] si la recherche dépasse ses bornes.
    pub(crate) fn find(
        &self,
        input: &[char],
        from: usize,
        sticky: bool,
    ) -> (Result<Option<Captures>, MatchError>, u64) {
        let mut machine = Machine {
            regex: self,
            input,
            steps: 0,
            caps: Vec::new(),
            regs: Vec::new(),
        };
        let mut start = from;
        let result = loop {
            if start > input.len() {
                break Ok(None);
            }
            machine.caps = vec![None; (self.groups + 1) * 2];
            machine.regs = vec![0; self.registers];
            match machine.run(0, start) {
                Err(error) => break Err(error),
                Ok(true) => {
                    let captures = machine
                        .caps
                        .chunks(2)
                        .map(|pair| match pair {
                            [Some(start), Some(end)] if start <= end => Some((*start, *end)),
                            _ => None,
                        })
                        .collect();
                    break Ok(Some(captures));
                }
                Ok(false) if sticky => break Ok(None),
                Ok(false) => start += 1,
            }
        };
        (result, machine.steps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Texte de la correspondance et de chaque groupe (`-` : groupe absent).
    fn find(pattern: &str, flags: &str, input: &str) -> Option<String> {
        let regex = compile(pattern, flags).expect("motif valide");
        let chars: Vec<char> = input.chars().collect();
        let (result, _) = regex.find(&chars, 0, flags.contains('y'));
        result.expect("recherche bornée").map(|captures| {
            captures
                .iter()
                .map(|c| match c {
                    Some((start, end)) => chars[*start..*end].iter().collect::<String>(),
                    None => "-".to_owned(),
                })
                .collect::<Vec<_>>()
                .join("|")
        })
    }

    #[test]
    fn literals_classes_and_anchors() {
        assert_eq!(find("abc", "", "xxabcx").as_deref(), Some("abc"));
        assert_eq!(find("a.c", "", "a\nc abc"), Some("abc".to_owned()));
        assert_eq!(find("a.c", "s", "a\nc").as_deref(), Some("a\nc"));
        assert_eq!(find("[a-c]+", "", "xxbcaz").as_deref(), Some("bca"));
        assert_eq!(find("[^a-c]+", "", "abxyc").as_deref(), Some("xy"));
        assert_eq!(find(r"\d+\.\d*", "", "pi = 3.14").as_deref(), Some("3.14"));
        assert_eq!(find(r"[\w-]+", "", "  a-b_c  ").as_deref(), Some("a-b_c"));
        assert_eq!(find(r"\s\S", "", "a b").as_deref(), Some(" b"));
        assert_eq!(find("^b", "", "a\nb"), None);
        assert_eq!(find("^b", "m", "a\nb").as_deref(), Some("b"));
        assert_eq!(find("a$", "m", "a\nb").as_deref(), Some("a"));
        assert_eq!(find(r"\bfoo\b", "", "a foo b").as_deref(), Some("foo"));
        assert_eq!(find(r"\Boo", "", "foo").as_deref(), Some("oo"));
        assert_eq!(find(r"\x41B\u{43}", "u", "ABC").as_deref(), Some("ABC"));
        assert_eq!(find(r"[\]\\]+", "", r"a]\b").as_deref(), Some(r"]\"));
        assert_eq!(find("é+", "", "créée").as_deref(), Some("éé"));
        assert_eq!(find(".", "", "😀").as_deref(), Some("😀"));
    }

    #[test]
    fn quantifiers() {
        assert_eq!(find("a*", "", "aaab").as_deref(), Some("aaa"));
        assert_eq!(find("a*?b", "", "aaab").as_deref(), Some("aaab"));
        assert_eq!(find("a+?", "", "aaa").as_deref(), Some("a"));
        assert_eq!(find("a{2,3}", "", "aaaa").as_deref(), Some("aaa"));
        assert_eq!(find("a{2,3}?", "", "aaaa").as_deref(), Some("aa"));
        assert_eq!(find("a{2}", "", "a aa").as_deref(), Some("aa"));
        assert_eq!(find("a{2,}", "", "aaaaa").as_deref(), Some("aaaaa"));
        assert_eq!(find("colou?r", "", "color").as_deref(), Some("color"));
        assert_eq!(find("(ab)*c", "", "ababc").as_deref(), Some("ababc|ab"));
        // Une accolade qui n'est pas un quantificateur est littérale.
        assert_eq!(find("a{,2}", "", "a{,2}").as_deref(), Some("a{,2}"));
        // Boucles sur le vide : terminent.
        assert_eq!(find("(a*)*", "", "b").as_deref(), Some("|-"));
        assert_eq!(find("(a*)+", "", "b").as_deref(), Some("|"));
        assert_eq!(find("(?:a|)*b", "", "aab").as_deref(), Some("aab"));
        // Les captures d'un tour précédent sont effacées.
        assert_eq!(find("(?:(a)|b)*", "", "ab").as_deref(), Some("ab|-"));
        assert_eq!(
            find("(z)((a+)?(b+)?(c))*", "", "zaacbbbcac").as_deref(),
            Some("zaacbbbcac|z|ac|a|-|c")
        );
    }

    #[test]
    fn groups_alternation_and_backreferences() {
        assert_eq!(find("(a)|(b)", "", "b").as_deref(), Some("b|-|b"));
        assert_eq!(find("a|ab", "", "abc").as_deref(), Some("a"));
        assert_eq!(find("(?:x)(y)", "", "xy").as_deref(), Some("xy|y"));
        assert_eq!(find(r"(\w)\1", "", "abccd").as_deref(), Some("cc|c"));
        assert_eq!(
            find(r"(?<q>['\x22]).*?\k<q>", "", "a 'b' c").as_deref(),
            Some("'b'|'")
        );
        assert_eq!(find(r"(a)\1", "i", "aA").as_deref(), Some("aA|a"));
        // Référence à un groupe qui n'a pas participé : correspond au vide.
        assert_eq!(find(r"(?:(a)|b)\1", "", "b").as_deref(), Some("b|-"));
        // Référence vers l'avant.
        assert_eq!(find(r"\1(a)", "", "a").as_deref(), Some("a|a"));
        // Sans groupe 8, `\8` est le caractère « 8 » ; `\101` est « A ».
        assert_eq!(find(r"\8\101", "", "8A").as_deref(), Some("8A"));
        let regex = compile("(?<an>\\d{4})-(?<mois>\\d\\d)", "").expect("valide");
        assert_eq!(regex.groups, 2);
        assert_eq!(regex.names, [("an".to_owned(), 1), ("mois".to_owned(), 2)]);
    }

    #[test]
    fn lookaround() {
        assert_eq!(find("a(?=b)", "", "ac ab").as_deref(), Some("a"));
        assert_eq!(find("a(?!b)", "", "ab ac").as_deref(), Some("a"));
        assert_eq!(find(r"(?<=\$)\d+", "", "€5 $42").as_deref(), Some("42"));
        assert_eq!(find(r"(?<!\$)\b\d+", "", "$5 42").as_deref(), Some("42"));
        assert_eq!(find("(?=(a+))a*b", "", "aaab").as_deref(), Some("aaab|aaa"));
        assert_eq!(find("(?!(a))b", "", "b").as_deref(), Some("b|-"));
        // Une assertion arrière lit vers la gauche : le groupe prend tout.
        assert_eq!(
            find(r"(?<=(\d+)(\d+))$", "", "1053").as_deref(),
            Some("|1|053")
        );
        assert_eq!(find(r"(?<=\1(a))b", "", "aab").as_deref(), Some("b|a"));
        assert_eq!(find("(?<=ab)c", "", "abc").as_deref(), Some("c"));
    }

    #[test]
    fn case_insensitivity_and_unicode() {
        assert_eq!(find("abc", "i", "xABCx").as_deref(), Some("ABC"));
        assert_eq!(find("[a-z]+", "i", "Hello").as_deref(), Some("Hello"));
        assert_eq!(find("É", "i", "é").as_deref(), Some("é"));
        assert_eq!(find(r"\p{Lu}\p{Ll}+", "u", "x Été").as_deref(), Some("Été"));
        assert_eq!(find(r"\P{L}+", "u", "ab12cd").as_deref(), Some("12"));
        assert_eq!(find(r"[\p{N}_]+", "u", "a_1٣b").as_deref(), Some("_1٣"));
    }

    #[test]
    fn unicode_properties() {
        // Catégories générales, seules ou nommées, et groupes de catégories.
        assert_eq!(find(r"\p{Lu}+", "u", "abcDÉFghi").as_deref(), Some("DÉF"));
        assert_eq!(
            find(r"\p{General_Category=Letter}+", "u", "12abc34").as_deref(),
            Some("abc")
        );
        assert_eq!(find(r"\P{L}+", "u", "ab12cd").as_deref(), Some("12"));
        // Écritures, avec et sans extensions.
        assert_eq!(
            find(r"\p{Script=Greek}+", "u", "abc αβγ").as_deref(),
            Some("αβγ")
        );
        assert_eq!(
            find(r"\p{sc=Han}+", "u", "ab漢字cd").as_deref(),
            Some("漢字")
        );
        assert_eq!(
            find(r"\p{scx=Hira}+", "u", "ab ーあ").as_deref(),
            Some("ーあ")
        );
        assert_eq!(find(r"\p{sc=Hira}+", "u", "ab ーあ").as_deref(), Some("あ"));
        // Propriétés binaires et pseudo-propriétés.
        assert_eq!(
            find(r"\p{Emoji_Presentation}", "u", "a😀b").as_deref(),
            Some("😀")
        );
        assert_eq!(
            find(r"\p{White_Space}+", "u", "a \t\u{a0}b").as_deref(),
            Some(" \t\u{a0}")
        );
        assert_eq!(find(r"\p{space}", "u", "a b").as_deref(), Some(" "));
        assert_eq!(find(r"[\p{ASCII}-]+", "u", "é-a-é").as_deref(), Some("-a-"));
        assert_eq!(
            find(r"\P{Assigned}", "u", "a\u{378}b").as_deref(),
            Some("\u{378}")
        );
        assert_eq!(find(r"\p{Any}+", "u", "a\n😀").as_deref(), Some("a\n😀"));
        // Les noms se lisent à la lettre : ni casse libre, ni valeur inconnue.
        for pattern in [
            r"\p{lu}",
            r"\p{letter}",
            r"\p{Script=greek}",
            r"\p{Script}",
            r"\p{Latin}",
            r"\p{General_Category=Greek}",
            r"\p{Nope=Lu}",
            r"\p{}",
            r"[\p{Lu}-z]",
        ] {
            assert!(compile(pattern, "u").is_err(), "{pattern}");
        }
    }

    #[test]
    fn single_character_loops_do_not_stack() {
        // Un tour par caractère ne coûte ni pas ni place de pile en proportion.
        let text: Vec<char> = "a".repeat(3_000_000).chars().collect();
        let regex = compile(r"^\p{L}+$", "u").expect("motif valide");
        let (result, steps) = regex.find(&text, 0, false);
        assert_eq!(result, Ok(Some(vec![Some((0, 3_000_000))])));
        assert!(steps < STEP_LIMIT);
        // Le retour sur trace rend les caractères un à un.
        assert_eq!(find("a*ab", "", "aaab").as_deref(), Some("aaab"));
        assert_eq!(find("a{2,}?b|a+", "", "aaa").as_deref(), Some("aaa"));
        assert_eq!(find(".*?b", "", "aabab").as_deref(), Some("aab"));
        assert_eq!(find("x[ab]{1,2}?a", "", "xbaa").as_deref(), Some("xba"));
        assert_eq!(find("(?<=a+)b", "", "caab").as_deref(), Some("b"));
        assert_eq!(find("(?<=(a+?))b", "", "caab").as_deref(), Some("b|a"));
        assert_eq!(find("a{2}", "", "a"), None);
    }

    #[test]
    fn sticky_and_start_position() {
        let regex = compile("b", "y").expect("valide");
        let chars: Vec<char> = "abc".chars().collect();
        assert_eq!(regex.find(&chars, 0, true).0, Ok(None));
        assert_eq!(regex.find(&chars, 1, true).0, Ok(Some(vec![Some((1, 2))])));
        assert_eq!(regex.find(&chars, 2, false).0, Ok(None));
        assert_eq!(regex.find(&chars, 9, false).0, Ok(None));
    }

    #[test]
    fn syntax_errors() {
        for pattern in [
            "(",
            ")",
            "a)",
            "[a",
            "*",
            "a**",
            "+a",
            "(?<n>a)(?<n>b)",
            "[z-a]",
            "a{2,1}",
            "\\",
            "(?<>a)",
            "(?<=a)*",
            "(?",
            "\\k<x>(?<y>a)",
        ] {
            assert!(compile(pattern, "").is_err(), "{pattern:?}");
        }
        for pattern in [
            "a{",
            "}",
            "]",
            r"\p{Inconnu}",
            r"\u{110000}",
            r"\-",
            "(?=a)*",
            r"\2(a)",
        ] {
            assert!(compile(pattern, "u").is_err(), "{pattern:?} avec u");
        }
        assert!(compile("a", "gg").is_err());
        assert!(compile("a", "x").is_err());
        assert!(compile("a", "v").is_err());
        assert_eq!(Flags::parse("ygi").expect("valide").canonical(), "giy");
        assert!(compile(&"(".repeat(500), "").is_err());
    }

    #[test]
    fn catastrophic_patterns_are_bounded() {
        let regex = compile("(a+)+$", "").expect("valide");
        let chars: Vec<char> = format!("{}b", "a".repeat(40)).chars().collect();
        let (result, steps) = regex.find(&chars, 0, false);
        assert_eq!(result, Err(MatchError));
        assert!(steps > STEP_LIMIT);
        // Un long texte ne fait pas déborder la pile native.
        let regex = compile("(?:a|b)*c", "").expect("valide");
        let chars: Vec<char> = format!("{}c", "ab".repeat(50_000)).chars().collect();
        assert!(matches!(regex.find(&chars, 0, false).0, Ok(Some(_))));
    }
}
