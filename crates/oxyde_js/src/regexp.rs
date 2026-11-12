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
//! d'Unicode ; avec le drapeau `v`, les propriétés de chaînes `RGI_Emoji`,
//! `RGI_Emoji_Flag_Sequence` et `RGI_Emoji_ZWJ_Sequence` ne sont pas fournies
//! (leurs tables manquent).
//!
//! Les propriétés Unicode de `\p{…}` (catégories générales, écritures,
//! propriétés binaires) viennent des tables d'ICU4X.

use icu_properties::props::{
    BasicEmoji, EmojiModifierBase, GeneralCategory, GeneralCategoryGroup, IdContinue, IdStart,
    Script,
};
use icu_properties::script::ScriptWithExtensions;
use icu_properties::{
    CodePointMapData, CodePointSetData, CodePointSetDataBorrowed, EmojiSetData, PropertyParser,
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
    /// Drapeau `v` : mode Unicode, avec les classes étendues.
    pub(crate) unicode_sets: bool,
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
                'v' => &mut flags.unicode_sets,
                'y' => &mut flags.sticky,
                _ => return Err(format!("drapeau « {c} » invalide ou non pris en charge")),
            };
            if std::mem::replace(slot, true) {
                return Err(format!("drapeau « {c} » répété"));
            }
        }
        if flags.unicode && flags.unicode_sets {
            return Err("les drapeaux « u » et « v » s'excluent".to_owned());
        }
        Ok(flags)
    }

    /// Vrai en mode Unicode : drapeau `u` ou `v`.
    pub(crate) fn unicode_mode(self) -> bool {
        self.unicode || self.unicode_sets
    }

    /// Drapeaux dans l'ordre canonique (`dgimsuvy`).
    pub(crate) fn canonical(self) -> String {
        [
            (self.indices, 'd'),
            (self.global, 'g'),
            (self.ignore_case, 'i'),
            (self.multiline, 'm'),
            (self.dot_all, 's'),
            (self.unicode, 'u'),
            (self.unicode_sets, 'v'),
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

/// Lettres qu'un repli de casse Unicode ramène à une lettre ASCII : avec les
/// drapeaux `i` et `u`, elles comptent parmi les caractères de mot.
fn folds_to_word(c: char) -> bool {
    matches!(c, '\u{17f}' | '\u{212a}')
}

impl Builtin {
    /// `wide` : drapeaux `i` et `u` ensemble, qui élargissent `\w`.
    fn matches(self, c: char, wide: bool) -> bool {
        match self {
            Self::Digit => c.is_ascii_digit(),
            Self::Word => is_word(c) || (wide && folds_to_word(c)),
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
    /// Classe du mode `v` : une expression d'ensembles, qui remplace le reste.
    expr: Option<Box<SetExpr>>,
}

/// Expression d'ensembles de caractères d'une classe du mode `v`.
#[derive(Clone, Debug)]
enum SetExpr {
    /// Intervalles et classes prédéfinies, sans négation.
    Leaf(ClassSet),
    Union(Vec<SetExpr>),
    Intersection(Vec<SetExpr>),
    /// Le premier ensemble, privé des suivants.
    Difference(Vec<SetExpr>),
    Complement(Box<SetExpr>),
}

impl SetExpr {
    /// Vrai si `c` appartient à l'ensemble. Insensible à la casse (`fold`),
    /// tout se décide sur la forme repliée de `c` : une feuille le contient si
    /// elle contient une de ses variantes de casse, et le complément se prend
    /// ensuite (ECMA-262 §22.2.2.9.7, `CharacterComplement`).
    fn contains(&self, c: char, fold: bool) -> bool {
        let folded = if fold { canonical(c, true) } else { c };
        self.holds(folded, c, fold)
    }

    fn holds(&self, c: char, original: char, fold: bool) -> bool {
        match self {
            Self::Leaf(set) => {
                set.contains(c, fold)
                    || (fold
                        && (set.contains(original, true) || set.contains(simple_upper(c), true)))
            }
            Self::Union(sets) => sets.iter().any(|s| s.holds(c, original, fold)),
            Self::Intersection(sets) => sets.iter().all(|s| s.holds(c, original, fold)),
            Self::Difference(sets) => {
                let mut sets = sets.iter();
                sets.next().is_some_and(|s| s.holds(c, original, fold))
                    && !sets.any(|s| s.holds(c, original, fold))
            }
            Self::Complement(set) => !set.holds(c, original, fold),
        }
    }
}

/// Valeur d'une classe du mode `v`, ou d'un de ses opérandes : un ensemble
/// de caractères et des chaînes (de longueur autre que 1).
#[derive(Clone, Debug)]
struct Operand {
    chars: SetExpr,
    strings: Vec<Vec<char>>,
    /// `MayContainStrings` : interdit la négation de la classe.
    may_contain_strings: bool,
}

impl Operand {
    fn chars(chars: SetExpr) -> Self {
        Self {
            chars,
            strings: Vec::new(),
            may_contain_strings: false,
        }
    }

    fn range(low: char, high: char) -> Self {
        Self::chars(SetExpr::Leaf(ClassSet {
            ranges: vec![(low, high)],
            ..ClassSet::default()
        }))
    }

    fn builtin(builtin: Builtin, negated: bool) -> Self {
        let leaf = SetExpr::Leaf(ClassSet::single(builtin, false));
        Self::chars(if negated {
            SetExpr::Complement(Box::new(leaf))
        } else {
            leaf
        })
    }

    /// Ensemble de caractères (`ranges`) et de chaînes donnés en extension.
    fn listed(ranges: Vec<(char, char)>, strings: Vec<Vec<char>>, may: bool) -> Self {
        let mut operand = Self::chars(SetExpr::Leaf(ClassSet {
            ranges,
            ..ClassSet::default()
        }));
        for string in strings {
            if !operand.strings.contains(&string) {
                operand.strings.push(string);
            }
        }
        operand.may_contain_strings = may;
        operand
    }

    fn union(items: Vec<Self>) -> Self {
        let mut strings: Vec<Vec<char>> = Vec::new();
        let mut may = false;
        let mut sets = Vec::with_capacity(items.len());
        for item in items {
            may |= item.may_contain_strings;
            for string in item.strings {
                if !strings.contains(&string) {
                    strings.push(string);
                }
            }
            sets.push(item.chars);
        }
        Self {
            chars: SetExpr::Union(sets),
            strings,
            may_contain_strings: may,
        }
    }

    fn intersection(items: Vec<Self>) -> Self {
        let may = items.iter().all(|item| item.may_contain_strings);
        let strings = items.first().map_or_else(Vec::new, |first| {
            first
                .strings
                .iter()
                .filter(|s| items.iter().all(|item| item.strings.contains(s)))
                .cloned()
                .collect()
        });
        Self {
            chars: SetExpr::Intersection(items.into_iter().map(|item| item.chars).collect()),
            strings,
            may_contain_strings: may,
        }
    }

    fn difference(items: Vec<Self>) -> Self {
        let may = items.first().is_some_and(|item| item.may_contain_strings);
        let strings = items.first().map_or_else(Vec::new, |first| {
            first
                .strings
                .iter()
                .filter(|s| !items.iter().skip(1).any(|item| item.strings.contains(s)))
                .cloned()
                .collect()
        });
        Self {
            chars: SetExpr::Difference(items.into_iter().map(|item| item.chars).collect()),
            strings,
            may_contain_strings: may,
        }
    }

    /// Nœud qui reconnaît la classe : les chaînes d'abord, de la plus longue
    /// à la plus courte, la classe de caractères prenant rang parmi elles.
    fn into_node(self) -> Node {
        let set = ClassSet {
            expr: Some(Box::new(self.chars)),
            ..ClassSet::default()
        };
        if self.strings.is_empty() {
            return Node::Class(set);
        }
        let mut strings = self.strings;
        strings.sort_by_key(|string| std::cmp::Reverse(string.len()));
        let mut alternatives = Vec::with_capacity(strings.len() + 1);
        let mut set = Some(set);
        for string in strings {
            if string.is_empty() {
                // La chaîne vide vient après les caractères seuls.
                alternatives.extend(set.take().map(Node::Class));
                alternatives.push(Node::Empty);
            } else {
                alternatives.push(Node::Seq(string.into_iter().map(Node::Char).collect()));
            }
        }
        alternatives.extend(set.take().map(Node::Class));
        Node::Group(Box::new(Node::Alt(alternatives)), None)
    }
}

/// Propriété de chaînes du mode `v` (`\p{Emoji_Keycap_Sequence}`…), donnée
/// en extension. `Ok(None)` : `name` n'en est pas une.
fn string_property(name: &str) -> PResult<Option<Operand>> {
    let scalar = |code: u32| char::from_u32(code).unwrap_or('\u{fffd}');
    let strings: Vec<Vec<char>> = match name {
        "Emoji_Keycap_Sequence" => "#*0123456789"
            .chars()
            .map(|c| vec![c, '\u{fe0f}', '\u{20e3}'])
            .collect(),
        "RGI_Emoji_Tag_Sequence" => ["eng", "sct", "wls"]
            .iter()
            .map(|region| {
                // Drapeau noir, « gb » et la région en lettres-étiquettes, fin.
                let mut sequence = vec!['\u{1f3f4}'];
                sequence.extend(
                    "gb".chars()
                        .chain(region.chars())
                        .map(|c| scalar(0xe0000 + u32::from(c))),
                );
                sequence.push('\u{e007f}');
                sequence
            })
            .collect(),
        "RGI_Emoji_Modifier_Sequence" => CodePointSetData::new::<EmojiModifierBase>()
            .iter_ranges()
            .flatten()
            .flat_map(|base| (0x1f3fb..=0x1f3ff).map(move |tone| vec![scalar(base), scalar(tone)]))
            .collect(),
        "Basic_Emoji" => {
            let data = EmojiSetData::new::<BasicEmoji>().static_to_owned();
            let list = data.to_code_point_inversion_list_string_list();
            let ranges = list
                .code_points()
                .iter_ranges()
                .map(|range| (scalar(*range.start()), scalar(*range.end())))
                .collect();
            let strings = list.strings().iter().map(|s| s.chars().collect()).collect();
            return Ok(Some(Operand::listed(ranges, strings, true)));
        }
        "RGI_Emoji" | "RGI_Emoji_Flag_Sequence" | "RGI_Emoji_ZWJ_Sequence" => {
            return Err(format!(
                "propriété de chaînes « {name} » non prise en charge par Oxyde pour l'instant"
            ));
        }
        _ => return Ok(None),
    };
    Ok(Some(Operand::listed(Vec::new(), strings, true)))
}

/// Forme canonique d'un caractère pour la comparaison insensible à la casse
/// (ECMA-262 §22.2.5.6). Avec le drapeau `u` (`unicode`), c'est le repli de
/// casse simple d'Unicode, approché par la minuscule de la majuscule ; sans
/// lui, la majuscule, qui ne ramène jamais une lettre non ASCII à l'ASCII.
fn canonical(c: char, unicode: bool) -> char {
    if unicode {
        return simple_lower(simple_upper(c));
    }
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

fn simple_upper(c: char) -> char {
    let mut upper = c.to_uppercase();
    match (upper.next(), upper.next()) {
        (Some(u), None) => u,
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
            expr: None,
        }
    }

    fn contains(&self, c: char, wide: bool) -> bool {
        self.ranges.iter().any(|(lo, hi)| *lo <= c && c <= *hi)
            || self
                .builtins
                .iter()
                .any(|(builtin, negated)| builtin.matches(c, wide) != *negated)
    }

    /// `ignore_case` : insensible à la casse ; `unicode` : drapeau `u`.
    fn matches(&self, c: char, ignore_case: bool, unicode: bool) -> bool {
        if let Some(expr) = &self.expr {
            return expr.contains(c, ignore_case);
        }
        let wide = ignore_case && unicode;
        let hit = self.contains(c, wide)
            || (ignore_case
                && (self.contains(canonical(c, unicode), wide)
                    || self.contains(simple_lower(c), wide)
                    || (unicode && self.contains(simple_upper(c), wide))));
        hit != self.negated
    }
}

// ---- Arbre syntaxique ------------------------------------------------------------

/// Drapeaux qu'un groupe peut changer pour son contenu : `i`, `m` et `s`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Modifiers {
    ignore_case: bool,
    multiline: bool,
    dot_all: bool,
}

impl Modifiers {
    /// Champ du drapeau `c`, s'il peut être modifié.
    fn slot(&mut self, c: char) -> Option<&mut bool> {
        match c {
            'i' => Some(&mut self.ignore_case),
            'm' => Some(&mut self.multiline),
            's' => Some(&mut self.dot_all),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
enum Node {
    Empty,
    Char(char),
    Any,
    Class(ClassSet),
    Seq(Vec<Node>),
    Alt(Vec<Node>),
    /// Groupe à modificateurs `(?ims-ims:…)` : drapeaux allumés (`on`) et
    /// éteints (`off`) pour son contenu.
    Modified {
        node: Box<Node>,
        on: Modifiers,
        off: Modifiers,
    },
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
    /// Référence arrière : au groupe qui a participé, parmi ceux de même nom.
    BackRef(Vec<usize>),
    Look {
        node: Box<Node>,
        behind: bool,
        negative: bool,
    },
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
    /// Mode Unicode : drapeau `u` ou `v`.
    unicode: bool,
    /// Drapeau `v` : classes étendues.
    unicode_sets: bool,
    /// Nombre total de groupes capturants du motif (connu d'avance).
    total_groups: usize,
    /// Noms des groupes, avec leur numéro.
    names: Vec<(String, usize)>,
    next_group: usize,
    depth: usize,
    /// Nombre de disjonctions rencontrées : chacune a son numéro.
    disjunctions: usize,
    /// Disjonctions qui entourent la position courante, avec le rang de
    /// l'alternative où l'on se trouve dans chacune.
    path: Vec<(usize, usize)>,
    /// Groupes nommés déjà définis, avec leur chemin.
    defined: Vec<(String, Vec<(usize, usize)>)>,
}

type PResult<T> = Result<T, String>;

/// Lit quatre chiffres hexadécimaux à partir de `pos`.
fn hex4(chars: &[char], pos: usize) -> Option<u32> {
    let digits = chars.get(pos..pos + 4)?;
    digits
        .iter()
        .try_fold(0u32, |value, c| Some(value * 16 + c.to_digit(16)?))
}

/// Lit un nom de groupe à partir de `pos` (juste après `<`), `>` compris
/// (ECMA-262 §22.2.1, `RegExpIdentifierName`) : lettres d'identifiant, `$`,
/// `_`, et échappements `\uXXXX` ou `\u{X}`, quel que soit le drapeau `u`.
/// Renvoie le nom et la position qui suit `>`.
fn read_group_name(chars: &[char], mut pos: usize) -> PResult<(String, usize)> {
    let invalid = || "nom de groupe invalide".to_owned();
    let mut name = String::new();
    loop {
        let c = match chars.get(pos).copied() {
            Some('>') if !name.is_empty() => return Ok((name, pos + 1)),
            Some('\\') if chars.get(pos + 1) == Some(&'u') => {
                pos += 2;
                let code = if chars.get(pos) == Some(&'{') {
                    let close = chars
                        .get(pos..)
                        .and_then(|rest| rest.iter().position(|c| *c == '}'))
                        .ok_or_else(invalid)?;
                    let digits = chars.get(pos + 1..pos + close).unwrap_or(&[]);
                    if digits.is_empty() || digits.len() > 8 {
                        return Err(invalid());
                    }
                    let code = digits
                        .iter()
                        .try_fold(0u32, |value, c| Some(value * 16 + c.to_digit(16)?))
                        .ok_or_else(invalid)?;
                    pos += close + 1;
                    code
                } else {
                    let lead = hex4(chars, pos).ok_or_else(invalid)?;
                    pos += 4;
                    // Deux échappements qui se suivent forment une paire.
                    let trail = (0xd800..0xdc00)
                        .contains(&lead)
                        .then(|| {
                            (chars.get(pos) == Some(&'\\') && chars.get(pos + 1) == Some(&'u'))
                                .then(|| hex4(chars, pos + 2))
                                .flatten()
                                .filter(|trail| (0xdc00..0xe000).contains(trail))
                        })
                        .flatten();
                    match trail {
                        Some(trail) => {
                            pos += 6;
                            0x10000 + ((lead - 0xd800) << 10) + (trail - 0xdc00)
                        }
                        None => lead,
                    }
                };
                char::from_u32(code).ok_or_else(invalid)?
            }
            Some(c) => {
                pos += 1;
                c
            }
            None => return Err(invalid()),
        };
        let allowed = if name.is_empty() {
            c == '$' || c == '_' || CodePointSetData::new::<IdStart>().contains(c)
        } else {
            matches!(c, '$' | '\u{200c}' | '\u{200d}')
                || CodePointSetData::new::<IdContinue>().contains(c)
        };
        if !allowed {
            return Err(invalid());
        }
        name.push(c);
    }
}

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
                    // Un même nom peut revenir dans des alternatives distinctes :
                    // l'analyse complète le vérifie.
                    let (name, _) = read_group_name(chars, i + 3)?;
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
        let id = self.disjunctions;
        self.disjunctions += 1;
        let mut alternatives = Vec::new();
        loop {
            self.path.push((id, alternatives.len()));
            let alternative = self.alternative();
            self.path.pop();
            alternatives.push(alternative?);
            if !self.eat('|') {
                break;
            }
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
            '[' if self.unicode_sets => (self.class_v()?.into_node(), true),
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
        if self.peek() == Some('?') && matches!(self.peek_at(1), Some('i' | 'm' | 's' | '-')) {
            let (on, off) = self.modifiers()?;
            let node = self.disjunction()?;
            self.close_group()?;
            return Ok((
                Node::Modified {
                    node: Box::new(node),
                    on,
                    off,
                },
                true,
            ));
        }
        let index = if self.looking_at("?:") {
            self.pos += 2;
            None
        } else if self.looking_at("?<") {
            let (name, next) = read_group_name(&self.chars, self.pos + 2)?;
            self.pos = next;
            // Deux groupes de même nom ne doivent pas pouvoir participer
            // ensemble : il faut qu'une disjonction les sépare.
            let clash = self.defined.iter().any(|(other, path)| {
                *other == name
                    && !path
                        .iter()
                        .zip(&self.path)
                        .take_while(|(a, b)| a.0 == b.0)
                        .any(|(a, b)| a.1 != b.1)
            });
            if clash {
                return Err(format!("nom de groupe « {name} » répété"));
            }
            self.defined.push((name, self.path.clone()));
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

    /// Lit `?ims-ims:` : les drapeaux à allumer, puis ceux à éteindre. Aucun
    /// ne se répète, et `(?-:` ne modifie rien : c'est une erreur.
    fn modifiers(&mut self) -> PResult<(Modifiers, Modifiers)> {
        let invalid = || "modificateurs de groupe invalides".to_owned();
        self.pos += 1;
        let (mut on, mut off) = (Modifiers::default(), Modifiers::default());
        let mut removing = false;
        loop {
            let c = self.peek().ok_or_else(invalid)?;
            self.pos += 1;
            match c {
                ':' => break,
                '-' if !removing => removing = true,
                c => {
                    // Un drapeau ne paraît qu'une fois, d'un côté ou de l'autre.
                    let seen = on.slot(c).is_some_and(|slot| *slot)
                        || off.slot(c).is_some_and(|slot| *slot);
                    let target = if removing { &mut off } else { &mut on };
                    match target.slot(c) {
                        Some(slot) if !seen => *slot = true,
                        _ => return Err(invalid()),
                    }
                }
            }
        }
        if removing && on == Modifiers::default() && off == Modifiers::default() {
            return Err(invalid());
        }
        Ok((on, off))
    }

    fn close_group(&mut self) -> PResult<()> {
        if self.eat(')') {
            Ok(())
        } else {
            Err("parenthèse non fermée".to_owned())
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

    /// Lit `{Nom}` ou `{Nom=Valeur}` après `\p`.
    fn property_name(&mut self) -> PResult<String> {
        if !self.eat('{') {
            return Err("propriété Unicode invalide".to_owned());
        }
        let mut name = String::new();
        loop {
            match self.peek() {
                Some('}') => {
                    self.pos += 1;
                    return Ok(name);
                }
                Some(c) => {
                    name.push(c);
                    self.pos += 1;
                }
                None => return Err("propriété Unicode invalide".to_owned()),
            }
        }
    }

    /// Classe prédéfinie du mode `v` : `\d`, `\W`, `\p{…}`… (`c`, la lettre,
    /// est déjà consommée). Une négation est un complément, et une propriété
    /// de chaînes ne se nie pas.
    fn class_escape_v(&mut self, c: char) -> PResult<Operand> {
        Ok(match c {
            'd' | 'D' => Operand::builtin(Builtin::Digit, c == 'D'),
            'w' | 'W' => Operand::builtin(Builtin::Word, c == 'W'),
            's' | 'S' => Operand::builtin(Builtin::Space, c == 'S'),
            _ => {
                let name = self.property_name()?;
                if let Some(strings) = string_property(&name)? {
                    if c == 'P' {
                        return Err(format!("la propriété de chaînes « {name} » ne se nie pas"));
                    }
                    return Ok(strings);
                }
                let Some(property) = Property::parse(&name) else {
                    return Err(format!("propriété Unicode « {name} » inconnue"));
                };
                Operand::builtin(Builtin::Property(property), c == 'P')
            }
        })
    }

    /// Opérande d'une classe du mode `v` : classe imbriquée, `\q{…}`, classe
    /// prédéfinie ou caractère. Le caractère est rendu à part quand
    /// l'opérande en est un : il peut alors borner un intervalle.
    fn class_operand_v(&mut self) -> PResult<(Operand, Option<char>)> {
        let single = |c: char| Ok((Operand::range(c, c), Some(c)));
        let Some(c) = self.peek() else {
            return Err("classe de caractères non fermée".to_owned());
        };
        self.pos += 1;
        match c {
            '[' => Ok((self.class_v()?, None)),
            '\\' => {
                let Some(escaped) = self.peek() else {
                    return Err("« \\ » en fin de motif".to_owned());
                };
                self.pos += 1;
                match escaped {
                    'q' if self.peek() == Some('{') => {
                        self.pos += 1;
                        Ok((self.class_strings_v()?, None))
                    }
                    'd' | 'D' | 'w' | 'W' | 's' | 'S' | 'p' | 'P' => {
                        Ok((self.class_escape_v(escaped)?, None))
                    }
                    'b' => single('\u{8}'),
                    '&' | '-' | '!' | '#' | '%' | ',' | ':' | ';' | '<' | '=' | '>' | '@' | '`'
                    | '~' => single(escaped),
                    _ => single(self.char_escape(escaped)?),
                }
            }
            '(' | ')' | '{' | '}' | '/' | '-' | '|' | ']' => {
                Err(format!("« {c} » doit être échappé dans une classe"))
            }
            _ => {
                // Ponctuation doublée : réservée à de futurs opérateurs.
                if "&!#$%*+,.:;<=>?@^`~".contains(c) && self.peek() == Some(c) {
                    return Err(format!("« {c}{c} » est réservé dans une classe"));
                }
                single(c)
            }
        }
    }

    /// `\q{abc|d|}` : les chaînes séparées par `|` (l'accolade ouvrante est
    /// déjà consommée). Une chaîne d'un seul caractère est ce caractère.
    fn class_strings_v(&mut self) -> PResult<Operand> {
        let mut ranges = Vec::new();
        let mut strings = Vec::new();
        let mut current: Vec<char> = Vec::new();
        let mut may = false;
        loop {
            let closing = match self.peek() {
                Some('}') => true,
                Some('|') => false,
                Some(_) => {
                    let (_, Some(c)) = self.class_operand_v()? else {
                        return Err("« \\q{…} » ne contient que des caractères".to_owned());
                    };
                    current.push(c);
                    continue;
                }
                None => return Err("« \\q{ » non fermé".to_owned()),
            };
            self.pos += 1;
            let string = std::mem::take(&mut current);
            match string.as_slice() {
                [c] => ranges.push((*c, *c)),
                _ => {
                    may = true;
                    strings.push(string);
                }
            }
            if closing {
                return Ok(Operand::listed(ranges, strings, may));
            }
        }
    }

    /// Classe du mode `v`, après son `[` : union d'opérandes et
    /// d'intervalles, intersection (`&&`) ou différence (`--`) d'opérandes,
    /// sans mélange à un même niveau.
    fn class_v(&mut self) -> PResult<Operand> {
        #[derive(Clone, Copy, PartialEq)]
        enum Op {
            Union,
            Intersection,
            Difference,
        }
        self.depth += 1;
        if self.depth > DEPTH_LIMIT {
            return Err("classes trop imbriquées".to_owned());
        }
        let negated = self.eat('^');
        let mut items: Vec<Operand> = Vec::new();
        let mut op = None;
        let mut first_is_range = false;
        loop {
            if self.eat(']') {
                break;
            }
            if !items.is_empty() {
                let found = if self.looking_at("&&") {
                    Some(Op::Intersection)
                } else if self.looking_at("--") {
                    Some(Op::Difference)
                } else {
                    None
                };
                match (op, found) {
                    (None, Some(found)) if !first_is_range => {
                        op = Some(found);
                        self.pos += 2;
                    }
                    (Some(current), Some(found)) if current == found => self.pos += 2,
                    (_, Some(_)) => {
                        return Err("opérateurs de classe mélangés".to_owned());
                    }
                    (None | Some(Op::Union), None) => op = Some(Op::Union),
                    (Some(_), None) => {
                        return Err("opérateur de classe attendu".to_owned());
                    }
                }
            }
            let (operand, single) = self.class_operand_v()?;
            // Intervalle `a-z` : un tiret seul entre deux caractères.
            if let Some(low) = single
                && self.peek() == Some('-')
                && self.peek_at(1) != Some('-')
            {
                if matches!(op, Some(Op::Intersection | Op::Difference)) {
                    return Err("intervalle parmi des opérandes".to_owned());
                }
                self.pos += 1;
                let (_, Some(high)) = self.class_operand_v()? else {
                    return Err("intervalle de classe invalide".to_owned());
                };
                if low > high {
                    return Err("intervalle de classe dans le désordre".to_owned());
                }
                first_is_range |= items.is_empty();
                items.push(Operand::range(low, high));
                continue;
            }
            items.push(operand);
        }
        self.depth -= 1;
        let mut class = match op {
            None | Some(Op::Union) => Operand::union(items),
            Some(Op::Intersection) => Operand::intersection(items),
            Some(Op::Difference) => Operand::difference(items),
        };
        if negated {
            if class.may_contain_strings {
                return Err("une classe niée ne contient pas de chaînes".to_owned());
            }
            class.chars = SetExpr::Complement(Box::new(class.chars));
        }
        Ok(class)
    }

    fn property(&mut self, negated: bool) -> PResult<ClassSet> {
        let name = self.property_name()?;
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
        if self.unicode_sets && matches!(c, 'd' | 'D' | 'w' | 'W' | 's' | 'S' | 'p' | 'P') {
            return Ok((self.class_escape_v(c)?.into_node(), true));
        }
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
                let (name, next) = read_group_name(&self.chars, self.pos)?;
                self.pos = next;
                let groups: Vec<usize> = self
                    .names
                    .iter()
                    .filter(|(n, _)| *n == name)
                    .map(|(_, index)| *index)
                    .collect();
                if groups.is_empty() {
                    return Err(format!("groupe « {name} » inconnu"));
                }
                return Ok((Node::BackRef(groups), true));
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
                    return Ok((Node::BackRef(vec![value]), true));
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
    /// `fold` : comparaison insensible à la casse (`c` est déjà canonique).
    Char {
        c: char,
        back: bool,
        fold: bool,
    },
    Any {
        back: bool,
        dot_all: bool,
    },
    Class {
        set: usize,
        back: bool,
        fold: bool,
    },
    Save(usize),
    /// Essaie `first`, puis `second` en cas d'échec.
    Split {
        first: usize,
        second: usize,
    },
    Jmp(usize),
    Start {
        multiline: bool,
    },
    End {
        multiline: bool,
    },
    WordBoundary {
        negated: bool,
        /// Drapeaux `i` et `u` ensemble : `\w` élargi.
        wide: bool,
    },
    BackRef {
        /// Groupes candidats : celui qui a participé est comparé.
        groups: Vec<usize>,
        back: bool,
        fold: bool,
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
    /// Drapeaux en vigueur à l'endroit que l'on compile : ceux de
    /// l'expression, modifiés par les groupes `(?ims-ims:…)` traversés.
    modifiers: Modifiers,
    /// Drapeau `u` : il change la comparaison insensible à la casse.
    unicode: bool,
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
                let fold = self.modifiers.ignore_case;
                let c = if fold {
                    canonical(*c, self.unicode)
                } else {
                    *c
                };
                self.emit(Inst::Char { c, back, fold })?;
            }
            Node::Any => {
                self.emit(Inst::Any {
                    back,
                    dot_all: self.modifiers.dot_all,
                })?;
            }
            Node::Class(set) => {
                self.sets.push(set.clone());
                let set = self.sets.len() - 1;
                self.emit(Inst::Class {
                    set,
                    back,
                    fold: self.modifiers.ignore_case,
                })?;
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
            Node::Modified { node, on, off } => {
                let outer = self.modifiers;
                let inner = &mut self.modifiers;
                inner.ignore_case = (inner.ignore_case || on.ignore_case) && !off.ignore_case;
                inner.multiline = (inner.multiline || on.multiline) && !off.multiline;
                inner.dot_all = (inner.dot_all || on.dot_all) && !off.dot_all;
                let compiled = self.node(node, back);
                self.modifiers = outer;
                compiled?;
            }
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
                self.emit(Inst::Start {
                    multiline: self.modifiers.multiline,
                })?;
            }
            Node::End => {
                self.emit(Inst::End {
                    multiline: self.modifiers.multiline,
                })?;
            }
            Node::WordBoundary { negated } => {
                self.emit(Inst::WordBoundary {
                    negated: *negated,
                    wide: self.unicode && self.modifiers.ignore_case,
                })?;
            }
            Node::BackRef(groups) => {
                self.emit(Inst::BackRef {
                    groups: groups.clone(),
                    back,
                    fold: self.modifiers.ignore_case,
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
        unicode: flags.unicode_mode(),
        unicode_sets: flags.unicode_sets,
        total_groups,
        names,
        next_group: 0,
        depth: 0,
        disjunctions: 0,
        path: Vec::new(),
        defined: Vec::new(),
    };
    let node = parser.disjunction()?;
    if parser.pos < parser.chars.len() {
        return Err("parenthèse fermante en trop".to_owned());
    }
    let mut compiler = Compiler {
        insts: Vec::new(),
        sets: Vec::new(),
        registers: 0,
        unicode: flags.unicode_mode(),
        modifiers: Modifiers {
            ignore_case: flags.ignore_case,
            multiline: flags.multiline,
            dot_all: flags.dot_all,
        },
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
            Some(Inst::Char {
                c: wanted, fold, ..
            }) => {
                let found = if *fold {
                    canonical(c, self.regex.flags.unicode_mode())
                } else {
                    c
                };
                found == *wanted
            }
            Some(Inst::Any { dot_all, .. }) => *dot_all || !is_line_terminator(c),
            Some(Inst::Class { set, fold, .. }) => self
                .regex
                .sets
                .get(*set)
                .is_some_and(|s| s.matches(c, *fold, self.regex.flags.unicode_mode())),
            _ => false,
        }
    }

    fn word_at(&self, index: Option<usize>, wide: bool) -> bool {
        index
            .and_then(|i| self.input.get(i))
            .is_some_and(|c| is_word(*c) || (wide && folds_to_word(*c)))
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
                Inst::Char { c, back, fold } => match self.next_char(pos, *back) {
                    Some((found, next)) => {
                        let found = if *fold {
                            canonical(found, self.regex.flags.unicode_mode())
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
                Inst::Any { back, dot_all } => match self.next_char(pos, *back) {
                    Some((found, next)) if *dot_all || !is_line_terminator(found) => {
                        pos = next;
                        true
                    }
                    _ => false,
                },
                Inst::Class { set, back, fold } => match self.next_char(pos, *back) {
                    Some((found, next))
                        if self.regex.sets.get(*set).is_some_and(|s| {
                            s.matches(found, *fold, self.regex.flags.unicode_mode())
                        }) =>
                    {
                        pos = next;
                        true
                    }
                    _ => false,
                },
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
                Inst::Start { multiline } => {
                    pos == 0
                        || (*multiline
                            && self
                                .input
                                .get(pos - 1)
                                .is_some_and(|c| is_line_terminator(*c)))
                }
                Inst::End { multiline } => {
                    pos == self.input.len()
                        || (*multiline
                            && self.input.get(pos).is_some_and(|c| is_line_terminator(*c)))
                }
                Inst::WordBoundary { negated, wide } => {
                    let boundary =
                        self.word_at(pos.checked_sub(1), *wide) != self.word_at(Some(pos), *wide);
                    boundary != *negated
                }
                Inst::BackRef { groups, back, fold } => {
                    let bounds = groups.iter().find_map(|group| {
                        let start = self.caps.get(group * 2).copied().flatten()?;
                        let end = self.caps.get(group * 2 + 1).copied().flatten()?;
                        Some((start, end))
                    });
                    match (bounds.map(|b| b.0), bounds.map(|b| b.1)) {
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
                                            (Some(a), Some(b)) => {
                                                let unicode = self.regex.flags.unicode_mode();
                                                a == b
                                                    || (*fold
                                                        && canonical(*a, unicode)
                                                            == canonical(*b, unicode))
                                            }
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
    fn group_names() {
        // Un même nom dans des alternatives distinctes ; `\k` suit le groupe
        // qui a participé.
        let regex = compile(r"(?:(?<x>a)|(?<x>b))\k<x>", "").expect("motif valide");
        assert_eq!(regex.names, [("x".to_owned(), 1), ("x".to_owned(), 2)]);
        assert_eq!(
            find(r"(?:(?<x>a)|(?<x>b))\k<x>", "", "abbaa").as_deref(),
            Some("bb|-|b")
        );
        assert_eq!(
            find(r"(?<y>.)(?:(?<x>a)|(?<x>b)|c)\k<x>", "", "zc").as_deref(),
            Some("zc|z|-|-")
        );
        // Deux groupes de même nom qui peuvent participer ensemble : refusés.
        for pattern in [
            r"(?<x>a)(?<x>b)",
            r"(?<x>a)|(?:b(?<x>c)(?<x>d))",
            r"(?:(?<x>a)|b)(?<x>c)",
            r"(?<x>a(?<x>b)|c)",
        ] {
            assert!(compile(pattern, "").is_err(), "{pattern}");
        }
        assert!(compile(r"(?<x>a)|(?:b|(?<x>c))|(?<x>d)", "").is_ok());
        // Noms : lettres d'identifiant Unicode et échappements, avec ou sans `u`.
        for flags in ["", "u"] {
            for pattern in [
                r"(?<π>a)\k<π>",
                r"(?<\u{03C0}>a)\k<π>",
                r"(?<\u03C0>a)",
                r"(?<$_a1>a)",
                r"(?<𝒜>a)",
                r"(?<\uD835\uDC9C>a)\k<𝒜>",
                "(?<a\u{200d}b>.)",
            ] {
                assert!(compile(pattern, flags).is_ok(), "{pattern} /{flags}");
            }
            for pattern in [
                r"(?<>a)",
                r"(?<1a>a)",
                r"(?<a-b>a)",
                r"(?<a",
                r"(?<\u{110000}>a)",
                r"(?<a>.)\k<b>",
            ] {
                assert!(compile(pattern, flags).is_err(), "{pattern} /{flags}");
            }
        }
    }

    #[test]
    fn modifiers() {
        // Un groupe allume ou éteint `i`, `m`, `s` pour son seul contenu.
        assert_eq!(find("a(?i:b)c", "", "aBc").as_deref(), Some("aBc"));
        assert_eq!(find("a(?i:b)c", "", "aBC"), None);
        assert_eq!(find("a(?-i:b)c", "i", "AbC").as_deref(), Some("AbC"));
        assert_eq!(find("a(?-i:b)c", "i", "ABC"), None);
        assert_eq!(find("(?s:.)+", "", "a\nb").as_deref(), Some("a\nb"));
        assert_eq!(find("(?-s:.)+", "s", "a\nb").as_deref(), Some("a"));
        assert_eq!(find("(?m:^b)", "", "a\nb").as_deref(), Some("b"));
        assert_eq!(find("(?-m:^b)", "m", "a\nb"), None);
        assert_eq!(find("(?im-s:^B.)", "s", "a\nbc").as_deref(), Some("bc"));
        // Imbrication, répétition, références et classes.
        assert_eq!(
            find("(?i:a(?-i:b)c)+", "", "AbCabc").as_deref(),
            Some("AbCabc")
        );
        assert_eq!(find("(a)(?i:\\1)", "", "aA").as_deref(), Some("aA|a"));
        assert_eq!(find("(?i:[a-c]+)d", "", "xABCd").as_deref(), Some("ABCd"));
        // Avec `u`, le repli de casse d'Unicode : `ſ` et `K` valent `s` et `k`.
        assert_eq!(find("(?i:s)", "u", "\u{17f}").as_deref(), Some("\u{17f}"));
        assert_eq!(
            find("(?i:\\w)", "u", "\u{212a}").as_deref(),
            Some("\u{212a}")
        );
        assert_eq!(find("\\w", "u", "\u{17f}"), None);
        assert_eq!(find("(?i:\\W)", "u", "\u{17f}"), None);
        assert_eq!(find("(?i:\\b)s", "u", "\u{17f}s"), None);
        for pattern in ["(?:a)", "(?i:)", "(?ims:a)", "(?-ims:a)", "(?i-ms:a)"] {
            assert!(compile(pattern, "").is_ok(), "{pattern}");
        }
        for pattern in [
            "(?-:a)",
            "(?ii:a)",
            "(?i-i:a)",
            "(?i-mm:a)",
            "(?x:a)",
            "(?i",
            "(?i)",
            "(?i-s-m:a)",
            "(?g:a)",
            "(?-u:a)",
        ] {
            assert!(compile(pattern, "").is_err(), "{pattern}");
        }
    }

    #[test]
    fn unicode_sets() {
        // Union, différence, intersection, imbrication et complément.
        assert_eq!(
            find(r"[[a-z]--[aeiou]]+", "v", "eau bcd").as_deref(),
            Some("bcd")
        );
        assert_eq!(
            find(r"[\p{L}&&\p{ASCII}]+", "v", "éabcé").as_deref(),
            Some("abc")
        );
        assert_eq!(
            find(r"[[a-c][x-z]\d]+", "v", "-a9z-").as_deref(),
            Some("a9z")
        );
        assert_eq!(
            find(r"[^[a-z]--[aeiou]]+", "v", "bcd eau!f").as_deref(),
            Some(" eau!")
        );
        assert_eq!(
            find(r"[\w--[\d_]--[a-f]]+", "v", "a1_gz").as_deref(),
            Some("gz")
        );
        assert_eq!(find(r"[]", "v", "a"), None);
        assert_eq!(find(r"[^]", "v", "a").as_deref(), Some("a"));
        // Chaînes : la plus longue d'abord, avec retour sur les plus courtes.
        assert_eq!(
            find(r"[\q{abc|d|}x]+", "v", "xabcdy").as_deref(),
            Some("xabcd")
        );
        assert_eq!(find(r"[\q{ab|a}]b", "v", "ab").as_deref(), Some("ab"));
        assert_eq!(find(r"^[\q{ab|a}]$", "v", "ab").as_deref(), Some("ab"));
        assert_eq!(
            find(r"[\q{ab|cd}--\q{ab}]", "v", "abcd").as_deref(),
            Some("cd")
        );
        assert_eq!(
            find(r"[\q{ab|cd}&&\q{cd|ef}]", "v", "abcd").as_deref(),
            Some("cd")
        );
        assert_eq!(find(r"(?<=[\q{ab}])c", "v", "abc").as_deref(), Some("c"));
        // Propriétés de chaînes.
        assert_eq!(
            find(r"^\p{Emoji_Keycap_Sequence}$", "v", "1\u{fe0f}\u{20e3}").as_deref(),
            Some("1\u{fe0f}\u{20e3}")
        );
        assert_eq!(
            find(
                r"[\p{Emoji_Keycap_Sequence}--\q{1\uFE0F\u20E3}]",
                "v",
                "1\u{fe0f}\u{20e3}"
            ),
            None
        );
        assert_eq!(
            find(
                r"\p{RGI_Emoji_Modifier_Sequence}",
                "v",
                "x\u{1f44d}\u{1f3fd}"
            )
            .as_deref(),
            Some("\u{1f44d}\u{1f3fd}")
        );
        assert!(compile(r"\p{Basic_Emoji}", "v").is_ok());
        // Insensible à la casse : le complément se prend après le repli.
        assert_eq!(find(r"[^\P{Lu}]", "vi", "a").as_deref(), Some("a"));
        assert_eq!(find(r"\P{Lu}", "vi", "aA"), None);
        assert_eq!(find(r"[\q{AB}]", "vi", "xab").as_deref(), Some("ab"));
        // Syntaxe propre au mode `v`.
        for pattern in [
            r"[a-z&&[aeiou]]",
            r"[a&&b--c]",
            r"[ab&&c]",
            r"[a--b&&c]",
            r"[a&&&b]",
            r"[(]",
            r"[a-]",
            r"[a|b]",
            r"[a!!b]",
            r"[^\q{ab}]",
            r"[^\p{Emoji_Keycap_Sequence}]",
            r"\P{Emoji_Keycap_Sequence}",
            r"[\q{a[b]}]",
            r"[z-a]",
            r"[\d-a]",
        ] {
            assert!(compile(pattern, "v").is_err(), "{pattern}");
        }
        for pattern in [
            r"[\(\)\-\|\&]",
            r"[a&&b&&c]",
            r"[^[a]&&[b]]",
            r"[\q{}]",
            r"[a\-b]",
        ] {
            assert!(compile(pattern, "v").is_ok(), "{pattern}");
        }
        assert!(compile(r"\p{Emoji_Keycap_Sequence}", "u").is_err());
        assert!(Flags::parse("uv").is_err());
        assert_eq!(Flags::parse("yvg").expect("valide").canonical(), "gvy");
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
        assert!(compile("a", "uv").is_err());
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
