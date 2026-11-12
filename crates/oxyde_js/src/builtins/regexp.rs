//! `RegExp` et les méthodes de `String` qui s'appuient dessus (`match`,
//! `matchAll`, `search`, `replace`, `replaceAll`, `split`).
//!
//! Les méthodes désignées par un symbole (`Symbol.match`, `replace`…) sont
//! dans [`super::regexp_symbols`] ; ce module fournit la recherche intégrée
//! et les raccourcis qu'elles prennent pour une expression non modifiée.
//!
//! LIMITE CONNUE : les propriétés statiques héritées (`RegExp.$1`,
//! `lastMatch`…) sont communes à tous les royaumes d'une machine.

use std::sync::Arc;

use super::{arg, constructor, getter, is_callable};
use crate::heap::{Kind, Object, Property, Slot};
use crate::regexp::{Captures, Regex};
use crate::value::{JsStr, ObjId, Value};
use crate::vm::{JsResult, NativeFn, Vm};

/// Longueur maximale d'une chaîne produite par un remplacement.
const MAX_STRING: usize = 1 << 28;
/// Nombre maximal d'expressions compilées gardées en cache.
const CACHE_LIMIT: usize = 512;

/// Texte préparé pour la recherche : points de code, et position de chacun
/// en unités UTF-16 (les index visibles des scripts).
#[derive(Debug)]
pub(crate) struct RegexInput {
    chars: Vec<char>,
    /// Décalage UTF-16 de chaque caractère, plus la longueur totale ;
    /// `None` pour un texte ASCII (les deux numérotations coïncident).
    units: Option<Vec<usize>>,
}

impl RegexInput {
    fn new(text: &str) -> Self {
        let chars: Vec<char> = text.chars().collect();
        let units = (!text.is_ascii()).then(|| {
            let mut offsets = Vec::with_capacity(chars.len() + 1);
            let mut offset = 0;
            for c in &chars {
                offsets.push(offset);
                offset += c.len_utf16();
            }
            offsets.push(offset);
            offsets
        });
        Self { chars, units }
    }

    fn len(&self) -> usize {
        self.chars.len()
    }

    /// Position en unités UTF-16 du caractère d'index `index`.
    fn units_at(&self, index: usize) -> usize {
        match &self.units {
            None => index.min(self.chars.len()),
            Some(offsets) => offsets.get(index).or(offsets.last()).copied().unwrap_or(0),
        }
    }

    /// Index du premier caractère situé à `unit` unités UTF-16 ou au-delà.
    fn char_at_units(&self, unit: usize) -> usize {
        match &self.units {
            None => unit.min(self.chars.len()),
            Some(offsets) => offsets
                .partition_point(|offset| *offset < unit)
                .min(self.chars.len()),
        }
    }

    fn text(&self, start: usize, end: usize) -> String {
        self.chars
            .get(start..end.max(start))
            .unwrap_or(&[])
            .iter()
            .collect()
    }
}

/// Trace de la dernière recherche réussie, lue par les propriétés statiques
/// héritées de `RegExp` (`$1`…`$9`, `input`, `lastMatch`, `lastParen`,
/// `leftContext`, `rightContext`).
#[derive(Debug, Default)]
pub(crate) struct LegacyStatics {
    /// Vrai après la recherche d'une expression qui n'alimente pas ces
    /// propriétés (sous-classe, autre royaume) : les lire lève `TypeError`.
    invalid: bool,
    /// Texte donné par `RegExp.input = …` depuis la dernière recherche.
    input: Option<JsStr>,
    /// Texte, texte préparé et captures de la dernière recherche réussie.
    last: Option<(JsStr, Arc<RegexInput>, Captures)>,
}

/// Ce que lit une propriété statique héritée.
#[derive(Clone, Copy)]
enum Legacy {
    Paren(usize),
    Input,
    LastMatch,
    LastParen,
    LeftContext,
    RightContext,
}

#[expect(
    clippy::cast_precision_loss,
    reason = "position dans une chaîne, petite devant 2^53"
)]
fn number(n: usize) -> Value {
    Value::Number(n as f64)
}

impl Vm {
    /// Crée un objet `RegExp` ; les motifs compilés sont mis en cache.
    ///
    /// # Errors
    /// `SyntaxError` si le motif ou les drapeaux sont invalides.
    pub(crate) fn new_regexp(&mut self, pattern: &JsStr, flags: &JsStr) -> JsResult {
        let regex = self.compile_regexp(pattern, flags)?;
        let mut object = Object::new(
            Some(self.intrinsics.regexp_proto),
            Kind::RegExp {
                regex,
                source: Arc::clone(pattern),
                legacy: true,
            },
        );
        object.props.insert(
            Arc::from("lastIndex"),
            Property {
                slot: Slot::Data(Value::Number(0.0)),
                writable: true,
                enumerable: false,
                configurable: false,
            },
        );
        Ok(Value::Object(self.heap.alloc(object)))
    }

    /// Le constructeur `RegExp` d'origine du royaume courant.
    fn regexp_constructor(&self) -> Option<ObjId> {
        self.regexp_originals
            .iter()
            .find(|(name, _)| &**name == "constructor")
            .map(|(_, id)| *id)
    }

    /// Compile `pattern` avec `flags` ; les motifs compilés sont mis en cache.
    fn compile_regexp(&mut self, pattern: &JsStr, flags: &JsStr) -> JsResult<Arc<Regex>> {
        let key = (Arc::clone(pattern), Arc::clone(flags));
        Ok(match self.regex_cache.get(&key) {
            Some(regex) => Arc::clone(regex),
            None => {
                let regex = match crate::regexp::compile(pattern, flags) {
                    Ok(regex) => Arc::new(regex),
                    Err(message) => {
                        return Err(self.error(
                            "SyntaxError",
                            &format!(
                                "expression rationnelle /{pattern}/{flags} invalide : {message}"
                            ),
                        ));
                    }
                };
                if self.regex_cache.len() >= CACHE_LIMIT {
                    self.regex_cache.clear();
                }
                self.regex_cache.insert(key, Arc::clone(&regex));
                regex
            }
        })
    }

    /// Objet et expression compilée, si `value` est un `RegExp`.
    pub(crate) fn regexp_parts(&self, value: &Value) -> Option<(ObjId, Arc<Regex>)> {
        let id = value.as_object()?;
        match &self.heap.get(id).kind {
            Kind::RegExp { regex, .. } => Some((id, Arc::clone(regex))),
            _ => None,
        }
    }

    /// Texte préparé pour `text` ; le dernier est gardé, car les boucles
    /// `while (m = re.exec(s))` interrogent sans cesse la même chaîne.
    fn regex_input(&mut self, text: &JsStr) -> Arc<RegexInput> {
        if let Some((cached, input)) = &self.regex_input
            && Arc::ptr_eq(cached, text)
        {
            return Arc::clone(input);
        }
        let input = Arc::new(RegexInput::new(text));
        self.regex_input = Some((Arc::clone(text), Arc::clone(&input)));
        input
    }

    /// Recherche bornée ; son coût est imputé au budget du script.
    fn regex_find(
        &mut self,
        regex: &Regex,
        input: &RegexInput,
        from: usize,
        sticky: bool,
    ) -> JsResult<Option<Captures>> {
        let (result, steps) = regex.find(&input.chars, from, sticky);
        self.charge(steps);
        if let Ok(Some(captures)) = &result
            && let Some((text, prepared)) = &self.regex_input
            && std::ptr::eq(Arc::as_ptr(prepared), input)
        {
            self.regexp_legacy = LegacyStatics {
                invalid: false,
                input: None,
                last: Some((Arc::clone(text), Arc::clone(prepared), captures.clone())),
            };
        }
        result.map_err(|_| {
            self.error(
                "RangeError",
                "expression rationnelle trop coûteuse à évaluer sur ce texte",
            )
        })
    }

    /// Écrit `lastIndex` ; `TypeError` si la propriété est figée.
    fn set_last_index(&mut self, regexp: ObjId, index: usize) -> JsResult<()> {
        if self.put(&Value::Object(regexp), "lastIndex", number(index))? {
            Ok(())
        } else {
            Err(self.type_error("« lastIndex » ne peut pas être écrit"))
        }
    }

    /// `RegExpBuiltinExec` : recherche à partir de `lastIndex` (drapeaux `g`
    /// et `y`) et met `lastIndex` à jour.
    pub(crate) fn regexp_exec(
        &mut self,
        regexp: ObjId,
        regex: &Regex,
        text: &JsStr,
    ) -> JsResult<Option<(Arc<RegexInput>, Captures)>> {
        let input = self.regex_input(text);
        let tracks = regex.flags.global || regex.flags.sticky;
        // `lastIndex` est toujours lu et converti, même quand il ne sert pas.
        let last = self.get(&Value::Object(regexp), "lastIndex")?;
        let last = self.to_number(&last)?;
        let mut from = 0;
        if tracks {
            let last = if last.is_nan() {
                0.0
            } else {
                last.trunc().max(0.0)
            };
            if last > number_of(input.units_at(input.len())) {
                self.set_last_index(regexp, 0)?;
                return Ok(None);
            }
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "entier positif inférieur à la longueur du texte"
            )]
            let last = last as usize;
            from = input.char_at_units(last);
        }
        match self.regex_find(regex, &input, from, regex.flags.sticky)? {
            Some(captures) => {
                // Une sous-classe, ou l'expression d'un autre royaume,
                // n'alimente pas `RegExp.$1` : elle le rend illisible.
                let object = self.heap.get(regexp);
                let feeds = object.realm == self.heap.realm
                    && matches!(object.kind, Kind::RegExp { legacy: true, .. });
                if !feeds {
                    self.regexp_legacy.invalid = true;
                }
                if tracks {
                    let end = captures
                        .first()
                        .copied()
                        .flatten()
                        .map_or(0, |(_, end)| end);
                    self.set_last_index(regexp, input.units_at(end))?;
                }
                Ok(Some((input, captures)))
            }
            None => {
                if tracks {
                    self.set_last_index(regexp, 0)?;
                }
                Ok(None)
            }
        }
    }

    /// Tableau résultat d'une correspondance : textes, `index`, `input`, `groups`.
    pub(crate) fn match_array(
        &mut self,
        regex: &Regex,
        input: &RegexInput,
        text: &JsStr,
        captures: &Captures,
    ) -> JsResult {
        let items: Vec<Value> = captures
            .iter()
            .map(|capture| match capture {
                Some((start, end)) => Value::string(&input.text(*start, *end)),
                None => Value::Undefined,
            })
            .collect();
        let start = captures
            .first()
            .copied()
            .flatten()
            .map_or(0, |(start, _)| start);
        let groups = self.groups_object(regex, &items)?;
        let array = self.new_array(items);
        self.define_field(&array, "index", number(input.units_at(start)))?;
        self.define_field(&array, "input", Value::String(Arc::clone(text)))?;
        self.define_field(&array, "groups", groups)?;
        if regex.flags.indices {
            // Drapeau `d` : bornes de la correspondance et de chaque groupe.
            let mut pairs = Vec::with_capacity(captures.len());
            for capture in captures {
                pairs.push(match capture {
                    Some((start, end)) => self.new_array(vec![
                        number(input.units_at(*start)),
                        number(input.units_at(*end)),
                    ]),
                    None => Value::Undefined,
                });
            }
            let groups = self.groups_object(regex, &pairs)?;
            let indices = self.new_array(pairs);
            self.define_field(&indices, "groups", groups)?;
            self.define_field(&array, "indices", indices)?;
        }
        Ok(array)
    }

    /// Crée la propriété propre `name` de `target` (`CreateDataProperty`) :
    /// ni un accesseur hérité ni `__proto__` ne s'interposent.
    fn define_field(&mut self, target: &Value, name: &str, value: Value) -> JsResult<()> {
        if let Some(id) = target.as_object() {
            let key = crate::value::PropertyKey::String(Arc::from(name));
            self.create_data_property(id, &key, value)?;
        }
        Ok(())
    }

    /// Objet `groups` (groupes nommés), ou `undefined` si le motif n'en a pas.
    fn groups_object(&mut self, regex: &Regex, items: &[Value]) -> JsResult {
        if regex.names.is_empty() {
            return Ok(Value::Undefined);
        }
        let groups = Value::Object(self.heap.alloc(Object::new(None, Kind::Ordinary)));
        for (position, (name, index)) in regex.names.iter().enumerate() {
            // Un nom porté par plusieurs groupes n'a qu'une propriété, à la
            // place du premier : la valeur du groupe qui a participé.
            if regex.names.iter().take(position).any(|(n, _)| n == name) {
                continue;
            }
            let value = regex
                .names
                .iter()
                .skip(position)
                .filter(|(n, _)| n == name)
                .filter_map(|(_, other)| items.get(*other))
                .find(|value| **value != Value::Undefined)
                .or_else(|| items.get(*index))
                .cloned()
                .unwrap_or(Value::Undefined);
            self.define_field(&groups, name, value)?;
        }
        Ok(groups)
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "longueur de chaîne petite devant 2^53"
)]
fn number_of(n: usize) -> f64 {
    n as f64
}

// ---- Constructeur et prototype ---------------------------------------------------

/// `RegExp(motif, drapeaux)` (ECMA-262 §22.2.4.1).
fn regexp_ctor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let (pattern, flags) = (arg(args, 0), arg(args, 1));
    let pattern_is_regexp = super::string::is_regexp(vm, &pattern)?;
    let own = vm.regexp_constructor().map(Value::Object);
    // Appelé comme fonction sur une expression du même constructeur, sans
    // drapeaux : elle est rendue telle quelle.
    if !vm.constructing && pattern_is_regexp && flags == Value::Undefined {
        let constructor = vm.get(&pattern, "constructor")?;
        if own.as_ref() == Some(&constructor) {
            return Ok(pattern);
        }
    }
    let inner = pattern
        .as_object()
        .and_then(|id| match &vm.heap.get(id).kind {
            Kind::RegExp { regex, source, .. } => {
                Some((Arc::clone(source), regex.flags.canonical()))
            }
            _ => None,
        });
    let (source, flags) = if let Some((source, original)) = inner {
        let flags = match flags {
            Value::Undefined => Value::string(&original),
            flags => flags,
        };
        (Value::String(source), flags)
    } else if pattern_is_regexp {
        // Objet qui se dit expression rationnelle : ses propriétés sont lues.
        let source = vm.get(&pattern, "source")?;
        let flags = match flags {
            Value::Undefined => vm.get(&pattern, "flags")?,
            flags => flags,
        };
        (source, flags)
    } else {
        (pattern, flags)
    };
    let source = match source {
        Value::Undefined => Arc::from(""),
        source => vm.to_string(&source)?,
    };
    let flags = match flags {
        Value::Undefined => Arc::from(""),
        flags => vm.to_string(&flags)?,
    };
    let created = vm.new_regexp(&source, &flags)?;
    // Construite pour une sous-classe : pas de propriétés statiques héritées.
    if vm.constructing
        && own.as_ref() != Some(&vm.native_new_target)
        && let Some(id) = created.as_object()
        && let Kind::RegExp { legacy, .. } = &mut vm.heap.get_mut(id).kind
    {
        *legacy = false;
    }
    Ok(created)
}

/// `RegExp.escape(texte)` : le texte, échappé pour se lire littéralement
/// dans un motif, où qu'il y soit inséré.
fn regexp_escape(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let Value::String(text) = arg(args, 0) else {
        return Err(vm.type_error("RegExp.escape : une chaîne est attendue"));
    };
    let mut out = String::with_capacity(text.len());
    let hex = |out: &mut String, c: char| {
        let code = u32::from(c);
        if code <= 0xff {
            out.push_str(&format!("\\x{code:02x}"));
        } else {
            let mut units = [0u16; 2];
            for unit in c.encode_utf16(&mut units) {
                out.push_str(&format!("\\u{unit:04x}"));
            }
        }
    };
    for (index, c) in text.chars().enumerate() {
        // Un chiffre ou une lettre en tête pourrait prolonger un échappement
        // qui précède (`\\0`, `\\c`) : il est écrit en hexadécimal.
        if index == 0 && c.is_ascii_alphanumeric() {
            hex(&mut out, c);
            continue;
        }
        match c {
            '^' | '$' | '\\' | '.' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '|'
            | '/' => {
                out.push('\\');
                out.push(c);
            }
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{b}' => out.push_str("\\v"),
            '\u{c}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            ',' | '-' | '=' | '<' | '>' | '#' | '&' | '!' | '%' | ':' | ';' | '@' | '~' | '\''
            | '`' | '"' => hex(&mut out, c),
            c if crate::regexp::is_space(c) => hex(&mut out, c),
            c => out.push(c),
        }
    }
    Ok(Value::string(&out))
}

/// `RegExp.prototype.compile(motif, drapeaux)` (annexe B) : recompile
/// l'expression sur place.
fn regexp_compile(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let (id, _) = this_regexp(vm, this)?;
    let object = vm.heap.get(id);
    let allowed =
        object.realm == vm.heap.realm && matches!(object.kind, Kind::RegExp { legacy: true, .. });
    if !allowed {
        return Err(
            vm.type_error("compile : expression d'un autre royaume ou d'une sous-classe de RegExp")
        );
    }
    let (pattern, flags) = (arg(args, 0), arg(args, 1));
    let inner = pattern
        .as_object()
        .and_then(|other| match &vm.heap.get(other).kind {
            Kind::RegExp { regex, source, .. } => {
                Some((Arc::clone(source), regex.flags.canonical()))
            }
            _ => None,
        });
    let (source, flags): (JsStr, JsStr) = match inner {
        Some((source, original)) => {
            if flags != Value::Undefined {
                return Err(vm.type_error(
                    "compile : pas de drapeaux quand le motif est une expression rationnelle",
                ));
            }
            (source, Arc::from(original))
        }
        None => {
            let source = match pattern {
                Value::Undefined => Arc::from(""),
                pattern => vm.to_string(&pattern)?,
            };
            let flags = match flags {
                Value::Undefined => Arc::from(""),
                flags => vm.to_string(&flags)?,
            };
            (source, flags)
        }
    };
    let compiled = vm.compile_regexp(&source, &flags)?;
    if let Kind::RegExp {
        regex,
        source: current,
        ..
    } = &mut vm.heap.get_mut(id).kind
    {
        *regex = compiled;
        *current = source;
    }
    vm.set_last_index(id, 0)?;
    Ok(this.clone())
}

/// Lecture d'une propriété statique héritée : `TypeError` si le receveur
/// n'est pas le constructeur `RegExp` lui-même, ou si la dernière recherche
/// venait d'une expression qui ne les alimente pas.
fn legacy_get(vm: &mut Vm, this: &Value, what: Legacy) -> JsResult {
    let own = vm.regexp_constructor();
    if own.is_none() || this.as_object() != own {
        return Err(vm.type_error("cette propriété ne se lit que sur RegExp lui-même"));
    }
    let statics = &vm.regexp_legacy;
    if statics.invalid {
        return Err(vm.type_error(
            "la dernière recherche n'a pas alimenté les propriétés statiques de RegExp",
        ));
    }
    if let (Legacy::Input, Some(input)) = (what, &statics.input) {
        return Ok(Value::String(Arc::clone(input)));
    }
    let Some((text, input, captures)) = &statics.last else {
        return Ok(Value::string(""));
    };
    let group = |index: usize| match captures.get(index).copied().flatten() {
        Some((start, end)) => input.text(start, end),
        None => String::new(),
    };
    let (start, end) = captures.first().copied().flatten().unwrap_or((0, 0));
    let value = match what {
        Legacy::Input => return Ok(Value::String(Arc::clone(text))),
        Legacy::Paren(index) => group(index),
        Legacy::LastMatch => group(0),
        Legacy::LastParen if captures.len() > 1 => group(captures.len() - 1),
        Legacy::LastParen => String::new(),
        Legacy::LeftContext => input.text(0, start),
        Legacy::RightContext => input.text(end, input.len()),
    };
    Ok(Value::string(&value))
}

macro_rules! legacy_getters {
    ($($name:ident => $what:expr),* $(,)?) => {
        $(
            fn $name(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
                legacy_get(vm, this, $what)
            }
        )*
    };
}
legacy_getters! {
    legacy_paren_1 => Legacy::Paren(1), legacy_paren_2 => Legacy::Paren(2),
    legacy_paren_3 => Legacy::Paren(3), legacy_paren_4 => Legacy::Paren(4),
    legacy_paren_5 => Legacy::Paren(5), legacy_paren_6 => Legacy::Paren(6),
    legacy_paren_7 => Legacy::Paren(7), legacy_paren_8 => Legacy::Paren(8),
    legacy_paren_9 => Legacy::Paren(9),
    legacy_input => Legacy::Input, legacy_last_match => Legacy::LastMatch,
    legacy_last_paren => Legacy::LastParen, legacy_left_context => Legacy::LeftContext,
    legacy_right_context => Legacy::RightContext,
}

/// `RegExp.input = texte` : seul `input` s'écrit.
fn legacy_set_input(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let own = vm.regexp_constructor();
    if own.is_none() || this.as_object() != own {
        return Err(vm.type_error("cette propriété ne s'écrit que sur RegExp lui-même"));
    }
    let text = vm.to_string(&arg(args, 0))?;
    vm.regexp_legacy.input = Some(text);
    Ok(Value::Undefined)
}

pub(super) fn this_regexp(vm: &mut Vm, this: &Value) -> JsResult<(ObjId, Arc<Regex>)> {
    match vm.regexp_parts(this) {
        Some(parts) => Ok(parts),
        None => Err(vm.type_error("le receveur n'est pas une expression rationnelle")),
    }
}

fn regexp_exec(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let (id, regex) = this_regexp(vm, this)?;
    let text = vm.to_string(&arg(args, 0))?;
    match vm.regexp_exec(id, &regex, &text)? {
        Some((input, captures)) => vm.match_array(&regex, &input, &text, &captures),
        None => Ok(Value::Null),
    }
}

fn regexp_test(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let (id, regex) = this_regexp(vm, this)?;
    let text = vm.to_string(&arg(args, 0))?;
    Ok(Value::Bool(vm.regexp_exec(id, &regex, &text)?.is_some()))
}

/// Source du motif telle qu'elle s'écrit entre deux barres obliques.
fn source_text(vm: &Vm, id: ObjId) -> String {
    let Kind::RegExp { source, .. } = &vm.heap.get(id).kind else {
        return "(?:)".to_owned();
    };
    if source.is_empty() {
        return "(?:)".to_owned();
    }
    // Une barre oblique non échappée fermerait le littéral : on l'échappe.
    let mut out = String::with_capacity(source.len());
    let mut escaped = false;
    for c in source.chars() {
        if c == '/' && !escaped {
            out.push('\\');
        }
        escaped = c == '\\' && !escaped;
        // Une fin de ligne couperait le littéral : elle s'écrit échappée.
        match c {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            c => out.push(c),
        }
    }
    out
}

fn regexp_to_string(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let (id, regex) = this_regexp(vm, this)?;
    Ok(Value::string(&format!(
        "/{}/{}",
        source_text(vm, id),
        regex.flags.canonical()
    )))
}

fn regexp_source(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    // `RegExp.prototype` n'est pas une expression, mais ne lève pas.
    if this.as_object() == Some(vm.intrinsics.regexp_proto) {
        return Ok(Value::string("(?:)"));
    }
    let (id, _) = this_regexp(vm, this)?;
    Ok(Value::string(&source_text(vm, id)))
}

/// `get RegExp.prototype.flags` (ECMA-262 §22.2.6.4) : chaque drapeau est lu
/// sur le receveur, qui peut être un objet quelconque.
fn regexp_flags(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    if this.as_object().is_none() {
        return Err(vm.type_error("RegExp.prototype.flags : le receveur doit être un objet"));
    }
    let mut flags = String::new();
    for (name, letter) in [
        ("hasIndices", 'd'),
        ("global", 'g'),
        ("ignoreCase", 'i'),
        ("multiline", 'm'),
        ("dotAll", 's'),
        ("unicode", 'u'),
        ("unicodeSets", 'v'),
        ("sticky", 'y'),
    ] {
        if vm.get(this, name)?.truthy() {
            flags.push(letter);
        }
    }
    Ok(Value::string(&flags))
}

macro_rules! flag_getter {
    ($name:ident, $field:ident) => {
        fn $name(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
            // `RegExp.prototype` n'est pas une expression, mais ne lève pas.
            if this.as_object() == Some(vm.intrinsics.regexp_proto) {
                return Ok(Value::Undefined);
            }
            let (_, regex) = this_regexp(vm, this)?;
            Ok(Value::Bool(regex.flags.$field))
        }
    };
}
flag_getter!(regexp_global, global);
flag_getter!(regexp_ignore_case, ignore_case);
flag_getter!(regexp_multiline, multiline);
flag_getter!(regexp_dot_all, dot_all);
flag_getter!(regexp_unicode, unicode);
flag_getter!(regexp_unicode_sets, unicode_sets);
flag_getter!(regexp_sticky, sticky);
flag_getter!(regexp_has_indices, indices);

// ---- Méthodes de String ----------------------------------------------------------

/// Développe un gabarit de remplacement (`$&`, `$1`, `$<nom>`, `` $` ``, `$'`, `$$`).
///
/// `groups` : textes des groupes 1, 2… (`None` : groupe absent) ; `context`
/// rend le texte qui précède (`false`) ou suit (`true`) la correspondance.
pub(super) fn expand_template(
    template: &str,
    matched: &str,
    groups: &[Option<String>],
    names: &[(String, usize)],
    context: &dyn Fn(bool) -> String,
) -> String {
    let chars: Vec<char> = template.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while let Some(&c) = chars.get(i) {
        i += 1;
        if c != '$' {
            out.push(c);
            continue;
        }
        let group = |index: usize| -> Option<&str> {
            // Les groupes sont numérotés à partir de 1.
            let slot = groups.get(index.checked_sub(1)?)?;
            Some(slot.as_deref().unwrap_or(""))
        };
        match chars.get(i).copied() {
            Some('$') => {
                out.push('$');
                i += 1;
            }
            Some('&') => {
                out.push_str(matched);
                i += 1;
            }
            Some('`') => {
                out.push_str(&context(false));
                i += 1;
            }
            Some('\'') => {
                out.push_str(&context(true));
                i += 1;
            }
            Some(first) if first.is_ascii_digit() => {
                let one = first.to_digit(10).unwrap_or(0) as usize;
                let two = chars
                    .get(i + 1)
                    .and_then(|d| d.to_digit(10))
                    .map(|d| one * 10 + d as usize);
                if let Some(text) = two.and_then(group) {
                    out.push_str(text);
                    i += 2;
                } else if let Some(text) = group(one) {
                    out.push_str(text);
                    i += 1;
                } else {
                    out.push('$');
                }
            }
            Some('<') if !names.is_empty() => {
                let end = chars
                    .get(i..)
                    .and_then(|rest| rest.iter().position(|c| *c == '>'));
                match end {
                    Some(len) => {
                        let name: String =
                            chars.get(i + 1..i + len).unwrap_or(&[]).iter().collect();
                        // Parmi les groupes de ce nom, celui qui a participé.
                        let text = names
                            .iter()
                            .filter(|(n, _)| *n == name)
                            .find_map(|(_, index)| groups.get(index.checked_sub(1)?)?.as_deref());
                        if let Some(text) = text {
                            out.push_str(text);
                        }
                        i += len + 1;
                    }
                    None => out.push('$'),
                }
            }
            _ => out.push('$'),
        }
    }
    out
}

/// Toutes les correspondances de `regex` dans `input`, de gauche à droite.
fn all_matches(vm: &mut Vm, regex: &Regex, input: &RegexInput) -> JsResult<Vec<Captures>> {
    let mut found = Vec::new();
    let mut from = 0;
    while from <= input.len() {
        let Some(captures) = vm.regex_find(regex, input, from, regex.flags.sticky)? else {
            break;
        };
        let (start, end) = captures.first().copied().flatten().unwrap_or((from, from));
        // Une correspondance vide fait avancer d'un caractère pour ne pas boucler.
        from = if end == start { end + 1 } else { end };
        found.push(captures);
    }
    Ok(found)
}

/// `regexp[Symbol.match](text)` pour une expression non modifiée.
pub(super) fn match_with(vm: &mut Vm, text: &JsStr, id: ObjId, regex: &Regex) -> JsResult {
    if !regex.flags.global {
        return match vm.regexp_exec(id, regex, text)? {
            Some((input, captures)) => vm.match_array(regex, &input, text, &captures),
            None => Ok(Value::Null),
        };
    }
    vm.set_last_index(id, 0)?;
    let input = vm.regex_input(text);
    let matches = all_matches(vm, regex, &input)?;
    if matches.is_empty() {
        return Ok(Value::Null);
    }
    let texts = matches
        .iter()
        .filter_map(|captures| captures.first().copied().flatten())
        .map(|(start, end)| Value::string(&input.text(start, end)))
        .collect();
    Ok(vm.new_array(texts))
}

/// `regexp[Symbol.search](text)` pour une expression non modifiée.
pub(super) fn search_with(vm: &mut Vm, text: &JsStr, regex: &Regex) -> JsResult {
    let input = vm.regex_input(text);
    Ok(match vm.regex_find(regex, &input, 0, regex.flags.sticky)? {
        Some(captures) => {
            let start = captures
                .first()
                .copied()
                .flatten()
                .map_or(0, |(start, _)| start);
            number(input.units_at(start))
        }
        None => Value::Number(-1.0),
    })
}

/// `regexp[Symbol.replace](text, replacer)` pour une expression non modifiée.
pub(super) fn string_replace(
    vm: &mut Vm,
    text: &JsStr,
    regexp: ObjId,
    regex: &Regex,
    replacer: &Value,
    all: bool,
) -> JsResult {
    if all && !regex.flags.global {
        return Err(
            vm.type_error("replaceAll exige une expression rationnelle globale (drapeau g)")
        );
    }
    let input = vm.regex_input(text);
    let matches = if regex.flags.global {
        vm.set_last_index(regexp, 0)?;
        all_matches(vm, regex, &input)?
    } else {
        vm.regexp_exec(regexp, regex, text)?
            .map(|(_, captures)| vec![captures])
            .unwrap_or_default()
    };
    let template = if is_callable(vm, replacer) {
        None
    } else {
        Some(vm.to_string(replacer)?)
    };
    let mut out = String::new();
    let mut cursor = 0;
    for captures in &matches {
        let Some((start, end)) = captures.first().copied().flatten() else {
            continue;
        };
        out.push_str(&input.text(cursor, start));
        let matched = input.text(start, end);
        let groups: Vec<Option<String>> = captures
            .iter()
            .skip(1)
            .map(|capture| capture.map(|(start, end)| input.text(start, end)))
            .collect();
        match &template {
            Some(template) => {
                let context = |after: bool| {
                    if after {
                        input.text(end, input.len())
                    } else {
                        input.text(0, start)
                    }
                };
                out.push_str(&expand_template(
                    template,
                    &matched,
                    &groups,
                    &regex.names,
                    &context,
                ));
            }
            None => {
                let mut call_args = vec![Value::string(&matched)];
                call_args.extend(
                    groups
                        .iter()
                        .map(|g| g.as_deref().map_or(Value::Undefined, Value::string)),
                );
                call_args.push(number(input.units_at(start)));
                call_args.push(Value::String(Arc::clone(text)));
                if !regex.names.is_empty() {
                    let named = vm.groups_object(regex, &call_args[..=regex.groups])?;
                    call_args.push(named);
                }
                let result = vm.call(replacer, &Value::Undefined, &call_args)?;
                out.push_str(&vm.to_string(&result)?);
            }
        }
        cursor = end;
        if out.len() > MAX_STRING {
            return Err(vm.error("RangeError", "chaîne trop longue"));
        }
    }
    out.push_str(&input.text(cursor, input.len()));
    Ok(Value::string(&out))
}

/// `text.split(regexp, limit)`.
pub(super) fn string_split(vm: &mut Vm, text: &JsStr, regex: &Regex, limit: usize) -> JsResult {
    let input = vm.regex_input(text);
    let mut parts: Vec<Value> = Vec::new();
    if limit == 0 {
        return Ok(vm.new_array(parts));
    }
    if input.len() == 0 {
        // Chaîne vide : aucun morceau si le motif correspond au vide.
        if vm.regex_find(regex, &input, 0, true)?.is_none() {
            parts.push(Value::String(Arc::clone(text)));
        }
        return Ok(vm.new_array(parts));
    }
    let mut last = 0;
    let mut from = 0;
    while from < input.len() {
        let Some(captures) = vm.regex_find(regex, &input, from, false)? else {
            break;
        };
        let Some((start, end)) = captures.first().copied().flatten() else {
            break;
        };
        if start >= input.len() {
            break;
        }
        // Une correspondance vide à la position du dernier découpage est ignorée.
        if end == last {
            from = start + 1;
            continue;
        }
        parts.push(Value::string(&input.text(last, start)));
        if parts.len() >= limit {
            return Ok(vm.new_array(parts));
        }
        for capture in captures.iter().skip(1) {
            parts.push(match capture {
                Some((start, end)) => Value::string(&input.text(*start, *end)),
                None => Value::Undefined,
            });
            if parts.len() >= limit {
                return Ok(vm.new_array(parts));
            }
        }
        last = end;
        from = end;
    }
    parts.push(Value::string(&input.text(last, input.len())));
    Ok(vm.new_array(parts))
}

pub(super) fn install(vm: &mut Vm) {
    let proto = vm.intrinsics.regexp_proto;
    let ctor = constructor(vm, "RegExp", 2, regexp_ctor, proto);
    vm.define_method(ctor, "escape", 1, regexp_escape);
    // Propriétés statiques héritées : des accesseurs sur le constructeur.
    let statics: &[(&[&str], NativeFn, bool)] = &[
        (&["$1"], legacy_paren_1, false),
        (&["$2"], legacy_paren_2, false),
        (&["$3"], legacy_paren_3, false),
        (&["$4"], legacy_paren_4, false),
        (&["$5"], legacy_paren_5, false),
        (&["$6"], legacy_paren_6, false),
        (&["$7"], legacy_paren_7, false),
        (&["$8"], legacy_paren_8, false),
        (&["$9"], legacy_paren_9, false),
        (&["input", "$_"], legacy_input, true),
        (&["lastMatch", "$&"], legacy_last_match, false),
        (&["lastParen", "$+"], legacy_last_paren, false),
        (&["leftContext", "$`"], legacy_left_context, false),
        (&["rightContext", "$'"], legacy_right_context, false),
    ];
    for (names, get, writable) in statics {
        for name in *names {
            let get = vm.new_function(&format!("get {name}"), 0, *get);
            let set =
                writable.then(|| vm.new_function(&format!("set {name}"), 1, legacy_set_input));
            vm.heap.get_mut(ctor).props.insert(
                Arc::from(*name),
                Property {
                    slot: Slot::Accessor {
                        get: Some(get),
                        set,
                    },
                    writable: false,
                    enumerable: false,
                    configurable: true,
                },
            );
        }
    }
    let methods: &[(&str, u32, NativeFn)] = &[
        ("compile", 2, regexp_compile),
        ("exec", 1, regexp_exec),
        ("test", 1, regexp_test),
        ("toString", 0, regexp_to_string),
    ];
    for (name, arity, func) in methods {
        vm.define_method(proto, name, *arity, *func);
    }
    let getters: &[(&str, NativeFn)] = &[
        ("source", regexp_source),
        ("flags", regexp_flags),
        ("global", regexp_global),
        ("ignoreCase", regexp_ignore_case),
        ("multiline", regexp_multiline),
        ("dotAll", regexp_dot_all),
        ("unicode", regexp_unicode),
        ("unicodeSets", regexp_unicode_sets),
        ("sticky", regexp_sticky),
        ("hasIndices", regexp_has_indices),
    ];
    for (name, func) in getters {
        getter(vm, proto, name, *func);
    }
}
