//! Méthodes de `RegExp.prototype` désignées par un symbole (`Symbol.match`,
//! `matchAll`, `replace`, `search`, `split`) et itérateur de `matchAll`
//! (ECMA-262 §22.2.6).
//!
//! Chaque méthode suit l'algorithme générique du standard : elle lit `flags`,
//! `exec`, `lastIndex` et `constructor` sur le receveur, qui peut donc être
//! n'importe quel objet. Une expression rationnelle que le script n'a pas
//! modifiée (voir [`pristine`]) prend un raccourci aux effets identiques.

use std::sync::Arc;

use super::regexp::{match_with, search_with, string_replace, string_split, this_regexp};
use super::{arg, is_callable};
use crate::heap::{HelperKind, IteratorHelper, Kind, Object, Property, Slot};
use crate::regexp::Regex;
use crate::value::{JsStr, ObjId, PropertyKey, Value, to_uint32};
use crate::vm::{JsResult, NativeFn, Vm};

/// Plus grand entier sûr (`2^53 - 1`), borne de `ToLength`.
const MAX_SAFE: f64 = 9_007_199_254_740_991.0;

/// Expression rationnelle que le script n'a pas modifiée : ni propriété
/// ajoutée, ni `lastIndex` figé, ni méthode ou accesseur de
/// `RegExp.prototype` redéfini. Les raccourcis ne valent que pour elle.
fn pristine(vm: &Vm, this: &Value) -> Option<(ObjId, Arc<Regex>)> {
    let (id, regex) = vm.regexp_parts(this)?;
    let object = vm.heap.get(id);
    let proto_id = vm.intrinsics.regexp_proto;
    if object.proto != Some(proto_id) || object.props.len() != 1 || !object.symbols.is_empty() {
        return None;
    }
    if !object.props.get("lastIndex").is_some_and(|p| p.writable) {
        return None;
    }
    let proto = vm.heap.get(proto_id);
    for (name, original) in &vm.regexp_originals {
        let intact = match proto.props.get(name).map(|p| &p.slot) {
            Some(Slot::Data(Value::Object(current))) => current == original,
            Some(Slot::Accessor {
                get: Some(current), ..
            }) => current == original,
            _ => false,
        };
        if !intact {
            return None;
        }
    }
    Some((id, regex))
}

/// Vrai si `RegExp[Symbol.species]` est l'accesseur d'origine : `split` peut
/// alors se passer de construire une copie de l'expression.
fn species_intact(vm: &Vm) -> bool {
    let Some(original) = vm.regexp_species else {
        return false;
    };
    let ctor = vm
        .regexp_originals
        .iter()
        .find(|(name, _)| &**name == "constructor")
        .map(|(_, id)| *id);
    ctor.is_some_and(|ctor| {
        matches!(
            vm.heap.get(ctor).symbols.get(&vm.well_known.species).map(|p| &p.slot),
            Some(Slot::Accessor { get: Some(current), .. }) if *current == original
        )
    })
}

fn receiver(vm: &mut Vm, this: &Value, method: &str) -> JsResult<()> {
    if this.as_object().is_none() {
        return Err(vm.type_error(&format!(
            "RegExp.prototype[{method}] : le receveur doit être un objet"
        )));
    }
    Ok(())
}

/// `Set(O, P, V, true)` : une écriture refusée lève `TypeError`.
fn set_strict(vm: &mut Vm, object: &Value, key: &str, value: Value) -> JsResult<()> {
    if vm.put(object, key, value)? {
        Ok(())
    } else {
        Err(vm.type_error(&format!("impossible d'écrire la propriété « {key} »")))
    }
}

/// `ToLength` (ECMA-262 §7.1.20).
fn to_length(vm: &mut Vm, value: &Value) -> JsResult<f64> {
    let number = vm.to_number(value)?;
    Ok(if number.is_nan() {
        0.0
    } else {
        number.trunc().clamp(0.0, MAX_SAFE)
    })
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "valeur déjà ramenée dans [0, 2^53)"
)]
fn to_index(length: f64) -> usize {
    length as usize
}

#[expect(clippy::cast_precision_loss, reason = "position petite devant 2^53")]
fn number(n: usize) -> Value {
    Value::Number(n as f64)
}

fn text(units: &[u16]) -> String {
    String::from_utf16_lossy(units)
}

/// `AdvanceStringIndex` (ECMA-262 §22.2.7.3) : position suivante, en sautant
/// une paire de substituts entière en mode Unicode.
fn advance(units: &[u16], index: usize, unicode: bool) -> usize {
    if !unicode || index + 1 >= units.len() {
        return index + 1;
    }
    let pair =
        (0xD800..0xDC00).contains(&units[index]) && (0xDC00..0xE000).contains(&units[index + 1]);
    index + if pair { 2 } else { 1 }
}

/// `RegExpExec` (ECMA-262 §22.2.7.1) : appelle `exec` s'il est redéfini,
/// sinon la recherche intégrée. Rend un objet ou `null`.
fn regexp_exec(vm: &mut Vm, regexp: &Value, string: &JsStr) -> JsResult {
    let exec = vm.get(regexp, "exec")?;
    if is_callable(vm, &exec) {
        let result = vm.call(&exec, regexp, &[Value::String(Arc::clone(string))])?;
        if result.as_object().is_none() && result != Value::Null {
            return Err(vm.type_error("« exec » doit rendre un objet ou null"));
        }
        return Ok(result);
    }
    let (id, regex) = this_regexp(vm, regexp)?;
    match vm.regexp_exec(id, &regex, string)? {
        Some((input, captures)) => vm.match_array(&regex, &input, string, &captures),
        None => Ok(Value::Null),
    }
}

/// Drapeaux lus sur le receveur (`flags`), sous forme de chaîne.
fn flags_of(vm: &mut Vm, regexp: &Value) -> JsResult<JsStr> {
    let flags = vm.get(regexp, "flags")?;
    vm.to_string(&flags)
}

/// Après une correspondance vide, avance `lastIndex` pour ne pas boucler.
fn skip_empty_match(vm: &mut Vm, regexp: &Value, units: &[u16], unicode: bool) -> JsResult<()> {
    let last = vm.get(regexp, "lastIndex")?;
    let last = to_length(vm, &last)?;
    let next = if last >= MAX_SAFE {
        last
    } else {
        #[expect(clippy::cast_precision_loss, reason = "position petite devant 2^53")]
        let next = advance(units, to_index(last), unicode) as f64;
        next
    };
    set_strict(vm, regexp, "lastIndex", Value::Number(next))
}

// ---- Symbol.match et Symbol.search ---------------------------------------------

fn symbol_match(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    receiver(vm, this, "Symbol.match")?;
    let string = vm.to_string(&arg(args, 0))?;
    if let Some((id, regex)) = pristine(vm, this) {
        return match_with(vm, &string, id, &regex);
    }
    let flags = flags_of(vm, this)?;
    if !flags.contains('g') {
        return regexp_exec(vm, this, &string);
    }
    let unicode = flags.contains('u') || flags.contains('v');
    set_strict(vm, this, "lastIndex", Value::Number(0.0))?;
    let units: Vec<u16> = string.encode_utf16().collect();
    let mut matches = Vec::new();
    loop {
        vm.tick(1)?;
        let result = regexp_exec(vm, this, &string)?;
        if result == Value::Null {
            break;
        }
        let matched = vm.get(&result, "0")?;
        let matched = vm.to_string(&matched)?;
        if matched.is_empty() {
            skip_empty_match(vm, this, &units, unicode)?;
        }
        matches.push(Value::String(matched));
    }
    Ok(if matches.is_empty() {
        Value::Null
    } else {
        vm.new_array(matches)
    })
}

fn symbol_search(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    receiver(vm, this, "Symbol.search")?;
    let string = vm.to_string(&arg(args, 0))?;
    if let Some((_, regex)) = pristine(vm, this) {
        return search_with(vm, &string, &regex);
    }
    let previous = vm.get(this, "lastIndex")?;
    let zero = Value::Number(0.0);
    if !crate::internal::same_value(&previous, &zero) {
        set_strict(vm, this, "lastIndex", zero)?;
    }
    let result = regexp_exec(vm, this, &string)?;
    let current = vm.get(this, "lastIndex")?;
    if !crate::internal::same_value(&current, &previous) {
        set_strict(vm, this, "lastIndex", previous)?;
    }
    if result == Value::Null {
        return Ok(Value::Number(-1.0));
    }
    vm.get(&result, "index")
}

// ---- Symbol.replace --------------------------------------------------------------

/// `GetSubstitution` (ECMA-262 §22.1.3.19.1) pour le chemin générique : les
/// groupes nommés sont lus sur l'objet `named`.
fn substitute(
    vm: &mut Vm,
    template: &str,
    matched: &str,
    units: &[u16],
    position: usize,
    captures: &[Option<String>],
    named: &Value,
) -> JsResult<String> {
    let chars: Vec<char> = template.chars().collect();
    let tail = (position + matched.encode_utf16().count()).min(units.len());
    let mut out = String::new();
    let mut i = 0;
    while let Some(&c) = chars.get(i) {
        i += 1;
        if c != '$' {
            out.push(c);
            continue;
        }
        let group = |index: usize| -> Option<&str> {
            let slot = captures.get(index.checked_sub(1)?)?;
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
                out.push_str(&text(units.get(..position).unwrap_or(&[])));
                i += 1;
            }
            Some('\'') => {
                out.push_str(&text(units.get(tail..).unwrap_or(&[])));
                i += 1;
            }
            Some(first) if first.is_ascii_digit() => {
                let one = first.to_digit(10).unwrap_or(0) as usize;
                let two = chars
                    .get(i + 1)
                    .and_then(|d| d.to_digit(10))
                    .map(|d| one * 10 + d as usize);
                if let Some(found) = two.and_then(group) {
                    out.push_str(found);
                    i += 2;
                } else if let Some(found) = group(one) {
                    out.push_str(found);
                    i += 1;
                } else {
                    out.push('$');
                }
            }
            Some('<') if *named != Value::Undefined => {
                let end = chars
                    .get(i..)
                    .and_then(|rest| rest.iter().position(|c| *c == '>'));
                match end {
                    Some(len) => {
                        let name: String =
                            chars.get(i + 1..i + len).unwrap_or(&[]).iter().collect();
                        let capture = vm.get(named, &name)?;
                        if capture != Value::Undefined {
                            out.push_str(&vm.to_string(&capture)?);
                        }
                        i += len + 1;
                    }
                    None => out.push('$'),
                }
            }
            _ => out.push('$'),
        }
    }
    Ok(out)
}

fn symbol_replace(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    receiver(vm, this, "Symbol.replace")?;
    let string = vm.to_string(&arg(args, 0))?;
    let replace_value = arg(args, 1);
    if let Some((id, regex)) = pristine(vm, this) {
        return string_replace(vm, &string, id, &regex, &replace_value, false);
    }
    let units: Vec<u16> = string.encode_utf16().collect();
    let functional = is_callable(vm, &replace_value);
    let template = if functional {
        Arc::from("")
    } else {
        vm.to_string(&replace_value)?
    };
    let flags = flags_of(vm, this)?;
    let global = flags.contains('g');
    let unicode = flags.contains('u') || flags.contains('v');
    if global {
        set_strict(vm, this, "lastIndex", Value::Number(0.0))?;
    }
    let mut results = Vec::new();
    loop {
        vm.tick(1)?;
        let result = regexp_exec(vm, this, &string)?;
        if result == Value::Null {
            break;
        }
        results.push(result.clone());
        if !global {
            break;
        }
        let matched = vm.get(&result, "0")?;
        if vm.to_string(&matched)?.is_empty() {
            skip_empty_match(vm, this, &units, unicode)?;
        }
    }
    let mut out = String::new();
    let mut next_position = 0;
    for result in &results {
        let length = vm.get(result, "length")?;
        let length = to_index(to_length(vm, &length)?.min(65_536.0));
        let matched = vm.get(result, "0")?;
        let matched = vm.to_string(&matched)?;
        let match_length = matched.encode_utf16().count();
        let position = vm.get(result, "index")?;
        let position = vm.to_number(&position)?;
        let position = if position.is_nan() {
            0.0
        } else {
            position.trunc()
        };
        #[expect(clippy::cast_precision_loss, reason = "longueur petite devant 2^53")]
        let position = to_index(position.clamp(0.0, units.len() as f64));
        let mut captures: Vec<Option<String>> = Vec::new();
        for n in 1..length.max(1) {
            let capture = vm.get(result, &n.to_string())?;
            captures.push(match capture {
                Value::Undefined => None,
                other => Some(vm.to_string(&other)?.to_string()),
            });
        }
        let named = vm.get(result, "groups")?;
        let replacement = if functional {
            let mut call_args = vec![Value::String(Arc::clone(&matched))];
            call_args.extend(
                captures
                    .iter()
                    .map(|c| c.as_deref().map_or(Value::Undefined, Value::string)),
            );
            call_args.push(number(position));
            call_args.push(Value::String(Arc::clone(&string)));
            if named != Value::Undefined {
                call_args.push(named);
            }
            let value = vm.call(&replace_value, &Value::Undefined, &call_args)?;
            vm.to_string(&value)?.to_string()
        } else {
            let named = if named == Value::Undefined {
                named
            } else {
                vm.to_object(&named)?
            };
            substitute(vm, &template, &matched, &units, position, &captures, &named)?
        };
        if position >= next_position {
            out.push_str(&text(units.get(next_position..position).unwrap_or(&[])));
            out.push_str(&replacement);
            next_position = position + match_length;
        }
    }
    if next_position < units.len() {
        out.push_str(&text(units.get(next_position..).unwrap_or(&[])));
    }
    Ok(Value::string(&out))
}

// ---- Symbol.split et Symbol.matchAll ---------------------------------------------

/// `SpeciesConstructor(O, %RegExp%)` (ECMA-262 §7.3.22).
fn species_constructor(vm: &mut Vm, regexp: &Value) -> JsResult {
    let default = vm.get(&Value::Object(vm.global), "RegExp")?;
    let constructor = vm.get(regexp, "constructor")?;
    if constructor == Value::Undefined {
        return Ok(default);
    }
    if constructor.as_object().is_none() {
        return Err(vm.type_error("« constructor » doit être un objet"));
    }
    let key = PropertyKey::Symbol(vm.well_known.species.clone());
    let species = vm.get_key(&constructor, &key)?;
    if species.is_nullish() {
        return Ok(default);
    }
    if !vm.is_constructor(&species) {
        return Err(vm.type_error("Symbol.species doit désigner un constructeur"));
    }
    Ok(species)
}

fn symbol_split(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    receiver(vm, this, "Symbol.split")?;
    let string = vm.to_string(&arg(args, 0))?;
    let limit_arg = arg(args, 1);
    if let Some((_, regex)) = pristine(vm, this)
        && species_intact(vm)
    {
        let limit = match &limit_arg {
            Value::Undefined => usize::MAX,
            other => to_uint32(vm.to_number(other)?) as usize,
        };
        return string_split(vm, &string, &regex, limit);
    }
    let constructor = species_constructor(vm, this)?;
    let flags = flags_of(vm, this)?;
    let unicode = flags.contains('u') || flags.contains('v');
    let new_flags: JsStr = if flags.contains('y') {
        flags
    } else {
        Arc::from(format!("{flags}y"))
    };
    let splitter = vm.construct(
        &constructor,
        &[this.clone(), Value::String(new_flags)],
        &constructor,
    )?;
    let limit = match &limit_arg {
        Value::Undefined => u32::MAX as usize,
        other => to_uint32(vm.to_number(other)?) as usize,
    };
    let mut parts: Vec<Value> = Vec::new();
    if limit == 0 {
        return Ok(vm.new_array(parts));
    }
    let units: Vec<u16> = string.encode_utf16().collect();
    if units.is_empty() {
        if regexp_exec(vm, &splitter, &string)? == Value::Null {
            parts.push(Value::String(string));
        }
        return Ok(vm.new_array(parts));
    }
    let size = units.len();
    let (mut p, mut q) = (0, 0);
    while q < size {
        vm.tick(1)?;
        set_strict(vm, &splitter, "lastIndex", number(q))?;
        let result = regexp_exec(vm, &splitter, &string)?;
        if result == Value::Null {
            q = advance(&units, q, unicode);
            continue;
        }
        let end = vm.get(&splitter, "lastIndex")?;
        #[expect(clippy::cast_precision_loss, reason = "longueur petite devant 2^53")]
        let end = to_index(to_length(vm, &end)?.min(size as f64));
        if end == p {
            q = advance(&units, q, unicode);
            continue;
        }
        parts.push(Value::string(&text(units.get(p..q).unwrap_or(&[]))));
        if parts.len() == limit {
            return Ok(vm.new_array(parts));
        }
        p = end;
        let length = vm.get(&result, "length")?;
        let length = to_index(to_length(vm, &length)?.min(65_536.0));
        for n in 1..length.max(1) {
            parts.push(vm.get(&result, &n.to_string())?);
            if parts.len() == limit {
                return Ok(vm.new_array(parts));
            }
        }
        q = p;
    }
    parts.push(Value::string(&text(units.get(p..).unwrap_or(&[]))));
    Ok(vm.new_array(parts))
}

fn symbol_match_all(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    receiver(vm, this, "Symbol.matchAll")?;
    let string = vm.to_string(&arg(args, 0))?;
    let constructor = species_constructor(vm, this)?;
    let flags = flags_of(vm, this)?;
    let matcher = vm.construct(
        &constructor,
        &[this.clone(), Value::String(Arc::clone(&flags))],
        &constructor,
    )?;
    let last = vm.get(this, "lastIndex")?;
    let last = to_length(vm, &last)?;
    set_strict(vm, &matcher, "lastIndex", Value::Number(last))?;
    let unicode = flags.contains('u') || flags.contains('v');
    // État de l'itérateur : expression, texte, drapeau global, mode Unicode.
    let helper = IteratorHelper {
        kind: HelperKind::RegExpString,
        iterator: matcher,
        next: Value::String(string),
        func: Value::Bool(flags.contains('g')),
        counter: f64::from(u8::from(unicode)),
        remaining: 0.0,
        inner: None,
        sources: Vec::new(),
        buffer: Vec::new(),
        zip: None,
        started: false,
        running: false,
        done: false,
    };
    Ok(Value::Object(vm.heap.alloc(Object::new(
        Some(vm.intrinsics.regexp_string_iterator_proto),
        Kind::IteratorHelper(Box::new(helper)),
    ))))
}

/// `%RegExpStringIteratorPrototype%.next` (ECMA-262 §22.2.9.2.1).
fn string_iterator_next(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let state = this
        .as_object()
        .and_then(|id| match &mut vm.heap.get_mut(id).kind {
            Kind::IteratorHelper(helper) if helper.kind == HelperKind::RegExpString => Some((
                id,
                helper.iterator.clone(),
                helper.next.clone(),
                helper.func == Value::Bool(true),
                helper.counter != 0.0,
                helper.done,
            )),
            _ => None,
        });
    let Some((id, matcher, Value::String(string), global, unicode, done)) = state else {
        return Err(vm.type_error("le receveur n'est pas un itérateur de matchAll"));
    };
    if done {
        return Ok(vm.iter_result(Value::Undefined, true));
    }
    let finish = |vm: &mut Vm| {
        if let Kind::IteratorHelper(helper) = &mut vm.heap.get_mut(id).kind {
            helper.done = true;
        }
    };
    let result = match regexp_exec(vm, &matcher, &string) {
        Ok(result) => result,
        Err(error) => {
            finish(vm);
            return Err(error);
        }
    };
    if result == Value::Null {
        finish(vm);
        return Ok(vm.iter_result(Value::Undefined, true));
    }
    if !global {
        finish(vm);
        return Ok(vm.iter_result(result, false));
    }
    let matched = vm.get(&result, "0")?;
    if vm.to_string(&matched)?.is_empty() {
        let units: Vec<u16> = string.encode_utf16().collect();
        skip_empty_match(vm, &matcher, &units, unicode)?;
    }
    Ok(vm.iter_result(result, false))
}

fn species_getter(_vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    Ok(this.clone())
}

pub(super) fn install(vm: &mut Vm) {
    let proto = vm.intrinsics.regexp_proto;
    let known = vm.well_known.clone();
    let methods: &[(&str, &crate::value::Symbol, u32, NativeFn)] = &[
        ("[Symbol.match]", &known.match_, 1, symbol_match),
        ("[Symbol.matchAll]", &known.match_all, 1, symbol_match_all),
        ("[Symbol.replace]", &known.replace, 2, symbol_replace),
        ("[Symbol.search]", &known.search, 1, symbol_search),
        ("[Symbol.split]", &known.split, 2, symbol_split),
    ];
    for (name, symbol, arity, func) in methods {
        let function = vm.new_function(name, *arity, *func);
        vm.define_hidden_symbol(proto, symbol, Value::Object(function));
    }

    // `RegExp[Symbol.species]` : accesseur qui rend le constructeur lui-même.
    if let Ok(Value::Object(ctor)) = vm.get(&Value::Object(vm.global), "RegExp") {
        let getter = vm.new_function("get [Symbol.species]", 0, species_getter);
        vm.heap.get_mut(ctor).symbols.insert(
            known.species.clone(),
            Property {
                slot: Slot::Accessor {
                    get: Some(getter),
                    set: None,
                },
                writable: true,
                enumerable: false,
                configurable: true,
            },
        );
        vm.regexp_species = Some(getter);
    }

    // Itérateur de `matchAll`.
    let iterator_proto = vm.intrinsics.regexp_string_iterator_proto;
    vm.heap.get_mut(iterator_proto).proto = Some(vm.intrinsics.iterator_root);
    vm.define_method(iterator_proto, "next", 0, string_iterator_next);
    vm.heap.get_mut(iterator_proto).symbols.insert(
        known.to_string_tag.clone(),
        Property {
            slot: Slot::Data(Value::string("RegExp String Iterator")),
            writable: false,
            enumerable: false,
            configurable: true,
        },
    );

    // Fonctions d'origine de `RegExp.prototype` : tant qu'elles sont en
    // place, les raccourcis sont permis.
    let names = [
        "exec",
        "flags",
        "global",
        "ignoreCase",
        "multiline",
        "dotAll",
        "unicode",
        "sticky",
        "hasIndices",
        "constructor",
    ];
    let mut originals = Vec::with_capacity(names.len());
    for name in names {
        let original = match vm.heap.get(proto).props.get(name).map(|p| &p.slot) {
            Some(Slot::Data(Value::Object(id))) => Some(*id),
            Some(Slot::Accessor { get: Some(id), .. }) => Some(*id),
            _ => None,
        };
        if let Some(original) = original {
            originals.push((Arc::from(name), original));
        }
    }
    vm.regexp_originals = originals;
}
