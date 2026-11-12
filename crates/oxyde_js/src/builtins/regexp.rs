//! `RegExp` et les méthodes de `String` qui s'appuient dessus (`match`,
//! `matchAll`, `search`, `replace`, `replaceAll`, `split`).
//!
//! Les méthodes désignées par un symbole (`Symbol.match`, `replace`…) sont
//! dans [`super::regexp_symbols`] ; ce module fournit la recherche intégrée
//! et les raccourcis qu'elles prennent pour une expression non modifiée.
//!
//! LIMITES CONNUES : pas de propriétés statiques héritées (`RegExp.$1`), ni
//! `compile`, ni `indices` (drapeau `d`).

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
        let key = (Arc::clone(pattern), Arc::clone(flags));
        let regex = match self.regex_cache.get(&key) {
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
        };
        let mut object = Object::new(
            Some(self.intrinsics.regexp_proto),
            Kind::RegExp {
                regex,
                source: Arc::clone(pattern),
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
        let mut from = 0;
        if tracks {
            let last = self.get(&Value::Object(regexp), "lastIndex")?;
            let last = self.to_number(&last)?;
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
        self.set(&array, "index", number(input.units_at(start)))?;
        self.set(&array, "input", Value::String(Arc::clone(text)))?;
        self.set(&array, "groups", groups)?;
        Ok(array)
    }

    /// Objet `groups` (groupes nommés), ou `undefined` si le motif n'en a pas.
    fn groups_object(&mut self, regex: &Regex, items: &[Value]) -> JsResult {
        if regex.names.is_empty() {
            return Ok(Value::Undefined);
        }
        let groups = Value::Object(self.heap.alloc(Object::new(None, Kind::Ordinary)));
        for (name, index) in &regex.names {
            let value = items.get(*index).cloned().unwrap_or(Value::Undefined);
            self.set(&groups, name, value)?;
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

fn regexp_ctor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let (pattern, flags) = (arg(args, 0), arg(args, 1));
    let (source, default_flags) = match pattern.as_object().map(|id| &vm.heap.get(id).kind) {
        Some(Kind::RegExp { regex, source }) => {
            (Arc::clone(source), Arc::from(regex.flags.canonical()))
        }
        _ if pattern == Value::Undefined => (Arc::from(""), Arc::from("")),
        _ => (vm.to_string(&pattern)?, Arc::from("")),
    };
    let flags = match flags {
        Value::Undefined => default_flags,
        other => vm.to_string(&other)?,
    };
    vm.new_regexp(&source, &flags)
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
        out.push(c);
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
                        if let Some((_, index)) = names.iter().find(|(n, _)| *n == name)
                            && let Some(text) = group(*index)
                        {
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
    constructor(vm, "RegExp", 2, regexp_ctor, proto);
    let methods: &[(&str, u32, NativeFn)] = &[
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
        ("sticky", regexp_sticky),
        ("hasIndices", regexp_has_indices),
    ];
    for (name, func) in getters {
        getter(vm, proto, name, *func);
    }
}
