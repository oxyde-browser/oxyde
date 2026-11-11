//! Sélecteurs CSS (Selectors Level 4, sous-ensemble) : analyse, spécificité, correspondance.
//!
//! LIMITES CONNUES : pas de pseudo-éléments (`::before`…, la règle est
//! écartée), pas d'espaces de noms, `:hover` / `:focus` / `:active` ne
//! correspondent jamais tant qu'il n'y a pas d'état d'interaction.

use oxyde_dom::{Document, ElementData, NodeId};

use crate::syntax::{Token, matching_close, split_top_level, trim};

/// Combinateur reliant un sélecteur composé à celui qui le précède (à sa gauche).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Combinator {
    /// Espace : descendant.
    Descendant,
    /// `>` : enfant.
    Child,
    /// `+` : frère suivant immédiat.
    NextSibling,
    /// `~` : frère suivant.
    SubsequentSibling,
}

/// Opérateur d'un sélecteur d'attribut.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttrOp {
    /// `[a]`
    Exists,
    /// `[a=v]`
    Equals,
    /// `[a~=v]`
    Includes,
    /// `[a|=v]`
    DashMatch,
    /// `[a^=v]`
    Prefix,
    /// `[a$=v]`
    Suffix,
    /// `[a*=v]`
    Substring,
}

/// Sélecteur d'attribut.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttrSelector {
    /// Nom de l'attribut (minuscules).
    pub name: String,
    /// Opérateur.
    pub op: AttrOp,
    /// Valeur comparée.
    pub value: String,
    /// Drapeau `i` (insensible à la casse ASCII).
    pub case_insensitive: bool,
}

/// Pseudo-classe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pseudo {
    /// `:root`
    Root,
    /// `:empty`
    Empty,
    /// `:first-child`
    FirstChild,
    /// `:last-child`
    LastChild,
    /// `:only-child`
    OnlyChild,
    /// `:nth-child(an+b)`
    NthChild(i32, i32),
    /// `:nth-last-child(an+b)`
    NthLastChild(i32, i32),
    /// `:link` / `:any-link`
    Link,
    /// `:not(...)`
    Not(Vec<Selector>),
    /// `:is(...)`
    Is(Vec<Selector>),
    /// `:where(...)` (spécificité nulle)
    Where(Vec<Selector>),
    /// Pseudo-classe d'état connue mais jamais vraie pour l'instant.
    Never,
}

/// Sélecteur composé (`div.a#b[c]:d`).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Compound {
    /// Nom d'élément (minuscules), `None` pour `*` ou absent.
    pub tag: Option<String>,
    /// `#id`
    pub id: Option<String>,
    /// `.classe`
    pub classes: Vec<String>,
    /// `[attr]`
    pub attrs: Vec<AttrSelector>,
    /// Pseudo-classes.
    pub pseudos: Vec<Pseudo>,
}

/// Sélecteur complexe. `compounds[0]` est le composé le plus à droite (le
/// sujet) ; chaque composé suivant est relié au précédent par son combinateur.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selector {
    subject: Compound,
    /// (combinateur vers la gauche, composé à gauche), de droite à gauche.
    ancestors: Vec<(Combinator, Compound)>,
    specificity: u32,
}

impl Selector {
    /// Spécificité encodée `(a << 20) | (b << 10) | c`.
    #[must_use]
    pub fn specificity(&self) -> u32 {
        self.specificity
    }

    /// Vrai si l'élément `node` de `doc` correspond.
    #[must_use]
    pub fn matches(&self, doc: &Document, node: NodeId) -> bool {
        let Some(el) = doc.element(node) else { return false };
        compound_matches(&self.subject, doc, node, el) && match_ancestors(&self.ancestors, doc, node)
    }
}

fn prev_element(doc: &Document, node: NodeId) -> Option<NodeId> {
    let mut cur = doc.node(node).prev_sibling;
    while let Some(n) = cur {
        if doc.element(n).is_some() {
            return Some(n);
        }
        cur = doc.node(n).prev_sibling;
    }
    None
}

fn next_element(doc: &Document, node: NodeId) -> Option<NodeId> {
    let mut cur = doc.node(node).next_sibling;
    while let Some(n) = cur {
        if doc.element(n).is_some() {
            return Some(n);
        }
        cur = doc.node(n).next_sibling;
    }
    None
}

fn parent_element(doc: &Document, node: NodeId) -> Option<NodeId> {
    doc.node(node).parent.filter(|p| doc.element(*p).is_some())
}

fn match_ancestors(rest: &[(Combinator, Compound)], doc: &Document, node: NodeId) -> bool {
    let Some(((combinator, compound), tail)) = rest.split_first() else { return true };
    let test = |candidate: NodeId| {
        doc.element(candidate).is_some_and(|el| compound_matches(compound, doc, candidate, el))
            && match_ancestors(tail, doc, candidate)
    };
    match combinator {
        Combinator::Child => parent_element(doc, node).is_some_and(test),
        Combinator::NextSibling => prev_element(doc, node).is_some_and(test),
        Combinator::Descendant => {
            let mut cur = parent_element(doc, node);
            while let Some(c) = cur {
                if test(c) {
                    return true;
                }
                cur = parent_element(doc, c);
            }
            false
        }
        Combinator::SubsequentSibling => {
            let mut cur = prev_element(doc, node);
            while let Some(c) = cur {
                if test(c) {
                    return true;
                }
                cur = prev_element(doc, c);
            }
            false
        }
    }
}

fn attr_matches(sel: &AttrSelector, el: &ElementData) -> bool {
    let Some(actual) = el.attr(&sel.name) else { return false };
    if sel.op == AttrOp::Exists {
        return true;
    }
    let (actual, expected) = if sel.case_insensitive {
        (actual.to_ascii_lowercase(), sel.value.to_ascii_lowercase())
    } else {
        (actual.to_owned(), sel.value.clone())
    };
    match sel.op {
        AttrOp::Exists => true,
        AttrOp::Equals => actual == expected,
        AttrOp::Includes => {
            !expected.is_empty() && actual.split_ascii_whitespace().any(|w| w == expected)
        }
        AttrOp::DashMatch => {
            actual == expected
                || actual.strip_prefix(&expected).is_some_and(|rest| rest.starts_with('-'))
        }
        AttrOp::Prefix => !expected.is_empty() && actual.starts_with(&expected),
        AttrOp::Suffix => !expected.is_empty() && actual.ends_with(&expected),
        AttrOp::Substring => !expected.is_empty() && actual.contains(&expected),
    }
}

fn nth_matches(a: i32, b: i32, index: i32) -> bool {
    // Existe-t-il n >= 0 tel que a*n + b == index ?
    // En i64 : `index - b` ne peut pas déborder même avec un `b` hostile.
    let (a, diff) = (i64::from(a), i64::from(index) - i64::from(b));
    if a == 0 { diff == 0 } else { diff % a == 0 && diff / a >= 0 }
}

fn compound_matches(c: &Compound, doc: &Document, node: NodeId, el: &ElementData) -> bool {
    if let Some(tag) = &c.tag
        && !el.name.eq_ignore_ascii_case(tag)
    {
        return false;
    }
    if let Some(id) = &c.id
        && el.id() != Some(id.as_str())
    {
        return false;
    }
    if !c.classes.iter().all(|class| el.classes().any(|x| x == class)) {
        return false;
    }
    if !c.attrs.iter().all(|a| attr_matches(a, el)) {
        return false;
    }
    c.pseudos.iter().all(|p| match p {
        Pseudo::Root => parent_element(doc, node).is_none(),
        Pseudo::Empty => !doc.children(node).any(|n| {
            doc.element(n).is_some() || doc.text(n).is_some_and(|t| !t.is_empty())
        }),
        Pseudo::FirstChild => prev_element(doc, node).is_none(),
        Pseudo::LastChild => next_element(doc, node).is_none(),
        Pseudo::OnlyChild => prev_element(doc, node).is_none() && next_element(doc, node).is_none(),
        Pseudo::NthChild(a, b) => {
            let index = std::iter::successors(prev_element(doc, node), |n| prev_element(doc, *n))
                .count();
            nth_matches(*a, *b, i32::try_from(index).unwrap_or(i32::MAX - 1) + 1)
        }
        Pseudo::NthLastChild(a, b) => {
            let index = std::iter::successors(next_element(doc, node), |n| next_element(doc, *n))
                .count();
            nth_matches(*a, *b, i32::try_from(index).unwrap_or(i32::MAX - 1) + 1)
        }
        Pseudo::Link => matches!(el.name.as_str(), "a" | "area") && el.has_attr("href"),
        Pseudo::Not(list) => !list.iter().any(|s| s.matches(doc, node)),
        Pseudo::Is(list) | Pseudo::Where(list) => list.iter().any(|s| s.matches(doc, node)),
        Pseudo::Never => false,
    })
}

fn compound_specificity(c: &Compound) -> u32 {
    let mut spec = 0u32;
    if c.id.is_some() {
        spec += 1 << 20;
    }
    let class_like = c.classes.len() + c.attrs.len();
    spec += u32::try_from(class_like).unwrap_or(0).min(1023) << 10;
    if c.tag.is_some() {
        spec += 1;
    }
    for p in &c.pseudos {
        spec += match p {
            Pseudo::Where(_) => 0,
            Pseudo::Not(list) | Pseudo::Is(list) => {
                list.iter().map(Selector::specificity).max().unwrap_or(0)
            }
            _ => 1 << 10,
        };
    }
    spec
}

/// Analyse une liste de sélecteurs séparés par des virgules. `None` si l'un
/// d'eux est invalide ou non pris en charge (la règle entière est alors écartée,
/// comme le veut le standard).
#[must_use]
pub fn parse_selector_list(tokens: &[Token]) -> Option<Vec<Selector>> {
    let parts = split_top_level(tokens, &Token::Comma);
    let mut out = Vec::with_capacity(parts.len());
    for part in parts {
        out.push(parse_selector(trim(part))?);
    }
    Some(out)
}

fn parse_selector(tokens: &[Token]) -> Option<Selector> {
    if tokens.is_empty() {
        return None;
    }
    // Lecture de gauche à droite en (combinateur précédent, composé).
    let mut sequence: Vec<(Option<Combinator>, Compound)> = Vec::new();
    let mut i = 0;
    let mut pending: Option<Combinator> = None;
    while i < tokens.len() {
        match &tokens[i] {
            Token::Whitespace => {
                if pending.is_none() && !sequence.is_empty() {
                    pending = Some(Combinator::Descendant);
                }
                i += 1;
            }
            Token::Delim(c @ ('>' | '+' | '~')) => {
                if sequence.is_empty() {
                    return None;
                }
                let combinator = match c {
                    '>' => Combinator::Child,
                    '+' => Combinator::NextSibling,
                    _ => Combinator::SubsequentSibling,
                };
                if matches!(pending, Some(p) if p != Combinator::Descendant) {
                    return None;
                }
                pending = Some(combinator);
                i += 1;
            }
            _ => {
                let (compound, next) = parse_compound(tokens, i)?;
                if !sequence.is_empty() && pending.is_none() {
                    return None;
                }
                sequence.push((pending.take(), compound));
                i = next;
            }
        }
    }
    if matches!(pending, Some(p) if p != Combinator::Descendant) {
        return None;
    }
    let mut specificity = 0u32;
    for (_, c) in &sequence {
        specificity = specificity.saturating_add(compound_specificity(c));
    }
    let (mut combinator, subject) = sequence.pop()?;
    let mut ancestors = Vec::with_capacity(sequence.len());
    while let Some((prev_combinator, compound)) = sequence.pop() {
        ancestors.push((combinator?, compound));
        combinator = prev_combinator;
    }
    Some(Selector { subject, ancestors, specificity })
}

fn parse_compound(tokens: &[Token], mut i: usize) -> Option<(Compound, usize)> {
    let mut c = Compound::default();
    let start = i;
    match tokens.get(i) {
        Some(Token::Ident(name)) => {
            c.tag = Some(name.to_ascii_lowercase());
            i += 1;
        }
        Some(Token::Delim('*')) => i += 1,
        _ => {}
    }
    loop {
        match tokens.get(i) {
            Some(Token::Hash(id)) => {
                c.id = Some(id.clone());
                i += 1;
            }
            Some(Token::Delim('.')) => {
                let Some(Token::Ident(class)) = tokens.get(i + 1) else { return None };
                c.classes.push(class.clone());
                i += 2;
            }
            Some(Token::OpenBracket) => {
                let close = matching_close(tokens, i);
                c.attrs.push(parse_attr(trim(tokens.get(i + 1..close)?))?);
                i = close + 1;
            }
            Some(Token::Colon) => match tokens.get(i + 1) {
                Some(Token::Ident(name)) => {
                    c.pseudos.push(simple_pseudo(&name.to_ascii_lowercase())?);
                    i += 2;
                }
                Some(Token::Function(name)) => {
                    let close = matching_close(tokens, i + 1);
                    let args = trim(tokens.get(i + 2..close)?);
                    c.pseudos.push(functional_pseudo(name, args)?);
                    i = close + 1;
                }
                // `::pseudo-élément` : non pris en charge.
                _ => return None,
            },
            _ => break,
        }
    }
    if i == start { None } else { Some((c, i)) }
}

fn parse_attr(tokens: &[Token]) -> Option<AttrSelector> {
    let Some(Token::Ident(name)) = tokens.first() else { return None };
    let name = name.to_ascii_lowercase();
    let rest = trim(&tokens[1..]);
    if rest.is_empty() {
        return Some(AttrSelector { name, op: AttrOp::Exists, value: String::new(), case_insensitive: false });
    }
    let (op, used) = match rest {
        [Token::Delim('='), ..] => (AttrOp::Equals, 1),
        [Token::Delim(c), Token::Delim('='), ..] => (
            match c {
                '~' => AttrOp::Includes,
                '|' => AttrOp::DashMatch,
                '^' => AttrOp::Prefix,
                '$' => AttrOp::Suffix,
                '*' => AttrOp::Substring,
                _ => return None,
            },
            2,
        ),
        _ => return None,
    };
    let rest = trim(&rest[used..]);
    let value = match rest.first()? {
        Token::Ident(v) | Token::Str(v) => v.clone(),
        _ => return None,
    };
    let case_insensitive = match trim(&rest[1..]) {
        [] => false,
        [Token::Ident(flag)] if flag.eq_ignore_ascii_case("i") => true,
        [Token::Ident(flag)] if flag.eq_ignore_ascii_case("s") => false,
        _ => return None,
    };
    Some(AttrSelector { name, op, value, case_insensitive })
}

fn simple_pseudo(name: &str) -> Option<Pseudo> {
    Some(match name {
        "root" => Pseudo::Root,
        "empty" => Pseudo::Empty,
        "first-child" => Pseudo::FirstChild,
        "last-child" => Pseudo::LastChild,
        "only-child" => Pseudo::OnlyChild,
        "link" | "any-link" => Pseudo::Link,
        "hover" | "active" | "focus" | "focus-visible" | "focus-within" | "visited" | "target"
        | "checked" | "disabled" | "indeterminate" | "placeholder-shown" | "invalid"
        | "required" | "fullscreen" => Pseudo::Never,
        _ => return None,
    })
}

fn functional_pseudo(name: &str, args: &[Token]) -> Option<Pseudo> {
    Some(match name {
        "not" => Pseudo::Not(parse_selector_list(args)?),
        "is" | "matches" | "-webkit-any" => Pseudo::Is(parse_selector_list(args)?),
        "where" => Pseudo::Where(parse_selector_list(args)?),
        "nth-child" => {
            let (a, b) = parse_nth(args)?;
            Pseudo::NthChild(a, b)
        }
        "nth-last-child" => {
            let (a, b) = parse_nth(args)?;
            Pseudo::NthLastChild(a, b)
        }
        _ => return None,
    })
}

/// Analyse la micro-syntaxe `an+b`.
fn parse_nth(tokens: &[Token]) -> Option<(i32, i32)> {
    let mut text = String::new();
    for t in tokens {
        match t {
            Token::Whitespace => {}
            Token::Ident(s) => text.push_str(&s.to_ascii_lowercase()),
            Token::Number(v) => text.push_str(&format!("{v:+}")),
            Token::Dimension(v, unit) => text.push_str(&format!("{v:+}{unit}")),
            Token::Delim(c @ ('+' | '-')) => text.push(*c),
            _ => return None,
        }
    }
    match text.as_str() {
        "odd" => return Some((2, 1)),
        "even" => return Some((2, 0)),
        _ => {}
    }
    let signed_int = |s: &str, empty_value: i32| -> Option<i32> {
        let mut negative = false;
        let mut digits = s;
        while let Some(rest) = digits.strip_prefix(['+', '-']) {
            if digits.starts_with('-') {
                negative = !negative;
            }
            digits = rest;
        }
        let value = if digits.is_empty() { empty_value } else { digits.parse::<i32>().ok()? };
        Some(if negative { value.checked_neg()? } else { value })
    };
    match text.split_once('n') {
        Some((a, b)) => Some((signed_int(a, 1)?, signed_int(b, 0)?)),
        None if text.is_empty() => None,
        None => Some((0, signed_int(&text, 0)?)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::tokenize;

    fn sel(s: &str) -> Option<Vec<Selector>> {
        parse_selector_list(&tokenize(s))
    }

    fn first_match(html: &str, selector: &str) -> Vec<String> {
        let doc = oxyde_html::parse_document(html);
        let list = sel(selector).expect("sélecteur valide");
        doc.descendants(doc.root())
            .filter(|n| list.iter().any(|s| s.matches(&doc, *n)))
            .filter_map(|n| {
                let e = doc.element(n)?;
                Some(format!("{}{}", e.name, e.id().map(|i| format!("#{i}")).unwrap_or_default()))
            })
            .collect()
    }

    #[test]
    fn specificity() {
        let spec = |s: &str| sel(s).expect("valide")[0].specificity();
        assert_eq!(spec("*"), 0);
        assert_eq!(spec("li"), 1);
        assert_eq!(spec("ul li"), 2);
        assert_eq!(spec("ul ol+li"), 3);
        assert_eq!(spec("h1 + *[rel=up]"), (1 << 10) + 1);
        assert_eq!(spec("li.red.level"), (2 << 10) + 1);
        assert_eq!(spec("#x34y"), 1 << 20);
        assert_eq!(spec("#s12:not(foo)"), (1 << 20) + 1);
        assert_eq!(spec(":where(#a) b"), 1);
    }

    #[test]
    fn invalid_selectors_are_rejected() {
        for s in ["", "a >", "> a", "a,,b", "a::before", "a:unknown", ".", "[=x]", "a > > b"] {
            assert!(sel(s).is_none(), "{s:?} devrait être rejeté");
        }
    }

    #[test]
    fn matching() {
        let html = "<div id=a class='x y'><p id=b>1</p><p id=c lang=fr-CA>2</p><span id=d></span></div><p id=e>";
        assert_eq!(first_match(html, "div p"), ["p#b", "p#c"]);
        assert_eq!(first_match(html, "body > p"), ["p#e"]);
        assert_eq!(first_match(html, ".x.y > :first-child"), ["p#b"]);
        assert_eq!(first_match(html, "p + p"), ["p#c"]);
        assert_eq!(first_match(html, "p ~ span, [lang|=fr]"), ["p#c", "span#d"]);
        assert_eq!(first_match(html, "div :nth-child(2n+1)"), ["p#b", "span#d"]);
        assert_eq!(first_match(html, "div :nth-last-child(-n+2)"), ["p#c", "span#d"]);
        assert_eq!(first_match(html, "p:not(#b, #e)"), ["p#c"]);
        assert_eq!(first_match(html, "span:empty, :root"), ["html", "span#d"]);
        assert_eq!(first_match(html, "p:hover"), Vec::<String>::new());
    }

    #[test]
    fn nth_syntax() {
        let nth = |s: &str| parse_nth(&tokenize(s));
        assert_eq!(nth("odd"), Some((2, 1)));
        assert_eq!(nth("3"), Some((0, 3)));
        assert_eq!(nth("n"), Some((1, 0)));
        assert_eq!(nth("-n + 3"), Some((-1, 3)));
        assert_eq!(nth("2n - 1"), Some((2, -1)));
        assert_eq!(nth("2n-1"), Some((2, -1)));
        assert_eq!(nth("x"), None);
    }
}
