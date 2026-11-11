//! `Array` et `Array.prototype`.
//!
//! LIMITES CONNUES : les méthodes n'opèrent que sur de vrais tableaux (pas
//! sur des objets « de type tableau » passés comme `this`) ; les itérateurs
//! de `keys` et `entries` parcourent une copie prise à leur création.

use crate::heap::{IterState, Kind};
use crate::value::{ObjId, PropertyKey, Value};
use crate::vm::{JsResult, NativeFn, Vm};

use super::{arg, array_like_to_vec, callable, constructor, is_callable, relative_index};

/// Nombre maximal d'éléments qu'une méthode peut produire d'un coup.
const MAX_LENGTH: usize = 1 << 26;

fn this_array(vm: &mut Vm, this: &Value) -> JsResult<ObjId> {
    match this.as_object() {
        Some(id) if matches!(vm.heap.get(id).kind, Kind::Array(_)) => Ok(id),
        _ => Err(vm.type_error("cette méthode attend un tableau comme receveur")),
    }
}

/// Copie des éléments (les trous deviennent `undefined`).
fn items(vm: &Vm, id: ObjId) -> Vec<Value> {
    match &vm.heap.get(id).kind {
        Kind::Array(items) => items
            .iter()
            .map(|v| {
                if *v == Value::Hole {
                    Value::Undefined
                } else {
                    v.clone()
                }
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn items_mut(vm: &mut Vm, id: ObjId) -> Option<&mut Vec<Value>> {
    match &mut vm.heap.get_mut(id).kind {
        Kind::Array(items) => Some(items),
        _ => None,
    }
}

fn len(vm: &Vm, id: ObjId) -> usize {
    match &vm.heap.get(id).kind {
        Kind::Array(items) => items.len(),
        _ => 0,
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "index de tableau petit devant 2^53"
)]
fn number(n: usize) -> Value {
    Value::Number(n as f64)
}

fn array_ctor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    if let [Value::Number(n)] = args {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "validé juste après"
        )]
        let length = *n as usize;
        #[expect(clippy::cast_precision_loss, reason = "comparaison de validation")]
        if *n < 0.0 || n.fract() != 0.0 || length as f64 != *n || length > MAX_LENGTH {
            return Err(vm.error("RangeError", "longueur de tableau invalide"));
        }
        return Ok(vm.new_array(vec![Value::Hole; length]));
    }
    Ok(vm.new_array(args.to_vec()))
}

fn array_is_array(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let is_array = arg(args, 0)
        .as_object()
        .is_some_and(|id| matches!(vm.heap.get(id).kind, Kind::Array(_)));
    Ok(Value::Bool(is_array))
}

fn array_of(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    Ok(vm.new_array(args.to_vec()))
}

fn array_from(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let source = arg(args, 0);
    let map = arg(args, 1);
    let values = match &source {
        Value::Undefined | Value::Null | Value::Hole => {
            return Err(vm.type_error("Array.from : la source n'est pas itérable"));
        }
        Value::String(_) => vm.iterate_to_vec(&source)?,
        Value::Object(id) => match &vm.heap.get(*id).kind {
            Kind::Array(_) | Kind::Map(_) | Kind::Set(_) | Kind::Iterator(_) => {
                vm.iterate_to_vec(&source)?
            }
            // Itérable (`Symbol.iterator`), sinon objet de type tableau (`{ length: n }`).
            _ if vm.has_key(*id, &PropertyKey::Symbol(vm.well_known.iterator.clone())) => {
                vm.iterate_to_vec(&source)?
            }
            _ if vm.has_property(*id, "length") => array_like_to_vec(vm, &source)?,
            _ => Vec::new(),
        },
        _ => Vec::new(),
    };
    if map == Value::Undefined {
        return Ok(vm.new_array(values));
    }
    let map = callable(vm, &map)?;
    let mut out = Vec::with_capacity(values.len());
    for (i, value) in values.into_iter().enumerate() {
        out.push(vm.call(&map, &Value::Undefined, &[value, number(i)])?);
    }
    Ok(vm.new_array(out))
}

fn push(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_array(vm, this)?;
    if let Some(items) = items_mut(vm, id) {
        items.extend_from_slice(args);
    }
    Ok(number(len(vm, id)))
}

fn pop(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let id = this_array(vm, this)?;
    let value = items_mut(vm, id)
        .and_then(Vec::pop)
        .unwrap_or(Value::Undefined);
    Ok(if value == Value::Hole {
        Value::Undefined
    } else {
        value
    })
}

fn shift(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let id = this_array(vm, this)?;
    let Some(items) = items_mut(vm, id) else {
        return Ok(Value::Undefined);
    };
    if items.is_empty() {
        return Ok(Value::Undefined);
    }
    let value = items.remove(0);
    Ok(if value == Value::Hole {
        Value::Undefined
    } else {
        value
    })
}

fn unshift(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_array(vm, this)?;
    if let Some(items) = items_mut(vm, id) {
        items.splice(0..0, args.iter().cloned());
    }
    Ok(number(len(vm, id)))
}

fn slice(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_array(vm, this)?;
    let length = len(vm, id);
    let start = relative_index(vm, &arg(args, 0), length, 0)?;
    let end = relative_index(vm, &arg(args, 1), length, length)?;
    let all = items(vm, id);
    Ok(vm.new_array(all.get(start..end.max(start)).unwrap_or(&[]).to_vec()))
}

fn splice(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_array(vm, this)?;
    let length = len(vm, id);
    let start = relative_index(vm, &arg(args, 0), length, 0)?;
    let count = match args.get(1) {
        None if args.is_empty() => 0,
        None => length - start,
        Some(value) => {
            let n = vm.to_number(value)?;
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "ramené dans [0, length - start]"
            )]
            let count = n.max(0.0).min((length - start) as f64) as usize;
            count
        }
    };
    let inserted = args.get(2..).unwrap_or(&[]).to_vec();
    let removed: Vec<Value> = match items_mut(vm, id) {
        Some(items) => items.splice(start..start + count, inserted).collect(),
        None => Vec::new(),
    };
    Ok(vm.new_array(removed))
}

fn concat(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_array(vm, this)?;
    let mut out = items(vm, id);
    for value in args {
        match value
            .as_object()
            .filter(|o| matches!(vm.heap.get(*o).kind, Kind::Array(_)))
        {
            Some(other) => out.extend(items(vm, other)),
            None => out.push(value.clone()),
        }
        if out.len() > MAX_LENGTH {
            return Err(vm.error("RangeError", "tableau trop grand"));
        }
    }
    Ok(vm.new_array(out))
}

fn join(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_array(vm, this)?;
    let separator = match arg(args, 0) {
        Value::Undefined => ",".into(),
        other => vm.to_string(&other)?,
    };
    let mut out = String::new();
    for (i, value) in items(vm, id).into_iter().enumerate() {
        if i > 0 {
            out.push_str(&separator);
        }
        if !value.is_nullish() {
            out.push_str(&vm.to_string(&value)?);
        }
    }
    Ok(Value::string(&out))
}

fn reverse(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let id = this_array(vm, this)?;
    if let Some(items) = items_mut(vm, id) {
        items.reverse();
    }
    Ok(this.clone())
}

fn index_of(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_array(vm, this)?;
    let all = items(vm, id);
    let from = relative_index(vm, &arg(args, 1), all.len(), 0)?;
    let needle = arg(args, 0);
    let found = all
        .iter()
        .enumerate()
        .skip(from)
        .find(|(_, v)| Vm::strict_equals(v, &needle));
    Ok(found.map_or(Value::Number(-1.0), |(i, _)| number(i)))
}

fn last_index_of(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_array(vm, this)?;
    let all = items(vm, id);
    let needle = arg(args, 0);
    let found = all
        .iter()
        .enumerate()
        .rev()
        .find(|(_, v)| Vm::strict_equals(v, &needle));
    Ok(found.map_or(Value::Number(-1.0), |(i, _)| number(i)))
}

fn includes(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_array(vm, this)?;
    let needle = arg(args, 0);
    Ok(Value::Bool(
        items(vm, id)
            .iter()
            .any(|v| Vm::same_value_zero(v, &needle)),
    ))
}

/// Appelle `callback(élément, index, tableau)` sur chaque élément, en relisant
/// le tableau à chaque tour (le rappel peut le modifier).
fn each(
    vm: &mut Vm,
    this: &Value,
    args: &[Value],
    reverse: bool,
    mut visit: impl FnMut(&mut Vm, usize, Value, Value) -> JsResult<bool>,
) -> JsResult<()> {
    let id = this_array(vm, this)?;
    let callback = callable(vm, &arg(args, 0))?;
    let this_arg = arg(args, 1);
    let length = len(vm, id);
    for step in 0..length {
        let i = if reverse { length - 1 - step } else { step };
        let Some(value) = (match &vm.heap.get(id).kind {
            Kind::Array(items) => items.get(i).cloned(),
            _ => None,
        }) else {
            continue;
        };
        let value = if value == Value::Hole {
            Value::Undefined
        } else {
            value
        };
        let result = vm.call(
            &callback,
            &this_arg,
            &[value.clone(), number(i), this.clone()],
        )?;
        if !visit(vm, i, value, result)? {
            break;
        }
    }
    Ok(())
}

fn for_each(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    each(vm, this, args, false, |_, _, _, _| Ok(true))?;
    Ok(Value::Undefined)
}

fn map(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let mut out = Vec::new();
    each(vm, this, args, false, |_, _, _, result| {
        out.push(result);
        Ok(true)
    })?;
    Ok(vm.new_array(out))
}

fn filter(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let mut out = Vec::new();
    each(vm, this, args, false, |_, _, value, result| {
        if result.truthy() {
            out.push(value);
        }
        Ok(true)
    })?;
    Ok(vm.new_array(out))
}

fn find_impl(
    vm: &mut Vm,
    this: &Value,
    args: &[Value],
    reverse: bool,
    want_index: bool,
) -> JsResult {
    let mut found = if want_index {
        Value::Number(-1.0)
    } else {
        Value::Undefined
    };
    each(vm, this, args, reverse, |_, i, value, result| {
        if result.truthy() {
            found = if want_index { number(i) } else { value };
            return Ok(false);
        }
        Ok(true)
    })?;
    Ok(found)
}

fn find(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    find_impl(vm, this, args, false, false)
}

fn find_index(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    find_impl(vm, this, args, false, true)
}

fn find_last(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    find_impl(vm, this, args, true, false)
}

fn find_last_index(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    find_impl(vm, this, args, true, true)
}

fn some(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let mut any = false;
    each(vm, this, args, false, |_, _, _, result| {
        any = result.truthy();
        Ok(!any)
    })?;
    Ok(Value::Bool(any))
}

fn every(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let mut all = true;
    each(vm, this, args, false, |_, _, _, result| {
        all = result.truthy();
        Ok(all)
    })?;
    Ok(Value::Bool(all))
}

fn reduce_impl(vm: &mut Vm, this: &Value, args: &[Value], reverse: bool) -> JsResult {
    let id = this_array(vm, this)?;
    let callback = callable(vm, &arg(args, 0))?;
    let mut all = items(vm, id);
    if reverse {
        all.reverse();
    }
    let length = all.len();
    let mut iter = all.into_iter().enumerate();
    let mut acc = match args.get(1) {
        Some(initial) => initial.clone(),
        None => match iter.next() {
            Some((_, first)) => first,
            None => return Err(vm.type_error("reduce sur un tableau vide sans valeur initiale")),
        },
    };
    for (step, value) in iter {
        let index = if reverse { length - 1 - step } else { step };
        acc = vm.call(
            &callback,
            &Value::Undefined,
            &[acc, value, number(index), this.clone()],
        )?;
    }
    Ok(acc)
}

fn reduce(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    reduce_impl(vm, this, args, false)
}

fn reduce_right(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    reduce_impl(vm, this, args, true)
}

/// Tri par fusion stable. Écrit à la main : le comparateur vient du script
/// et peut être incohérent, ce que les tris de la bibliothèque standard de
/// Rust ne garantissent pas de supporter sans paniquer.
fn merge_sort(
    vm: &mut Vm,
    values: Vec<Value>,
    less_or_equal: &mut dyn FnMut(&mut Vm, &Value, &Value) -> JsResult<bool>,
) -> JsResult<Vec<Value>> {
    if values.len() <= 1 {
        return Ok(values);
    }
    let mut runs: Vec<Vec<Value>> = values.into_iter().map(|v| vec![v]).collect();
    while runs.len() > 1 {
        let mut merged = Vec::with_capacity(runs.len().div_ceil(2));
        let mut iter = runs.into_iter();
        while let Some(left) = iter.next() {
            let Some(right) = iter.next() else {
                merged.push(left);
                break;
            };
            let mut out = Vec::with_capacity(left.len() + right.len());
            let (mut l, mut r) = (left.into_iter().peekable(), right.into_iter().peekable());
            loop {
                match (l.peek(), r.peek()) {
                    (Some(a), Some(b)) => {
                        if less_or_equal(vm, a, b)? {
                            out.extend(l.next());
                        } else {
                            out.extend(r.next());
                        }
                    }
                    (Some(_), None) => out.extend(l.next()),
                    (None, Some(_)) => out.extend(r.next()),
                    (None, None) => break,
                }
            }
            merged.push(out);
        }
        runs = merged;
    }
    Ok(runs.pop().unwrap_or_default())
}

fn sort(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_array(vm, this)?;
    let comparator = arg(args, 0);
    if comparator != Value::Undefined && !is_callable(vm, &comparator) {
        return Err(vm.type_error("le comparateur de sort doit être une fonction"));
    }
    let all = items(vm, id);
    // `undefined` est toujours placé à la fin, sans appeler le comparateur.
    let (defined, undefined): (Vec<Value>, Vec<Value>) =
        all.into_iter().partition(|v| *v != Value::Undefined);
    let mut less_or_equal = |vm: &mut Vm, a: &Value, b: &Value| -> JsResult<bool> {
        if comparator == Value::Undefined {
            let (x, y) = (vm.to_string(a)?, vm.to_string(b)?);
            return Ok(x.encode_utf16().le(y.encode_utf16()));
        }
        let result = vm.call(&comparator, &Value::Undefined, &[a.clone(), b.clone()])?;
        // NaN compte pour zéro : les éléments gardent leur ordre.
        Ok(vm.to_number(&result)?.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater))
    };
    let mut sorted = merge_sort(vm, defined, &mut less_or_equal)?;
    sorted.extend(undefined);
    if let Some(items) = items_mut(vm, id) {
        *items = sorted;
    }
    Ok(this.clone())
}

fn flatten(vm: &mut Vm, values: Vec<Value>, depth: usize, out: &mut Vec<Value>) -> JsResult<()> {
    for value in values {
        match value
            .as_object()
            .filter(|o| depth > 0 && matches!(vm.heap.get(*o).kind, Kind::Array(_)))
        {
            Some(inner) => flatten(vm, items(vm, inner), depth - 1, out)?,
            None => out.push(value),
        }
        if out.len() > MAX_LENGTH {
            return Err(vm.error("RangeError", "tableau trop grand"));
        }
    }
    Ok(())
}

fn flat(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_array(vm, this)?;
    let depth = match arg(args, 0) {
        Value::Undefined => 1.0,
        other => vm.to_number(&other)?,
    };
    // Borne la récursion : au-delà de 1000 niveaux, le résultat ne change plus en pratique.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "ramené dans [0, 1000]"
    )]
    let depth = depth.clamp(0.0, 1000.0) as usize;
    let mut out = Vec::new();
    flatten(vm, items(vm, id), depth, &mut out)?;
    Ok(vm.new_array(out))
}

fn flat_map(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let mut mapped = Vec::new();
    each(vm, this, args, false, |_, _, _, result| {
        mapped.push(result);
        Ok(true)
    })?;
    let mut out = Vec::new();
    flatten(vm, mapped, 1, &mut out)?;
    Ok(vm.new_array(out))
}

fn fill(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_array(vm, this)?;
    let length = len(vm, id);
    let start = relative_index(vm, &arg(args, 1), length, 0)?;
    let end = relative_index(vm, &arg(args, 2), length, length)?;
    let value = arg(args, 0);
    if let Some(slice) = items_mut(vm, id).and_then(|items| items.get_mut(start..end.max(start))) {
        slice.fill(value);
    }
    Ok(this.clone())
}

fn at(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_array(vm, this)?;
    let all = items(vm, id);
    let n = vm.to_number(&arg(args, 0))?;
    #[expect(clippy::cast_precision_loss, reason = "longueur petite devant 2^53")]
    let index = if n < 0.0 {
        all.len() as f64 + n.trunc()
    } else {
        n.trunc()
    };
    if index < 0.0 || index.is_nan() {
        return Ok(Value::Undefined);
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "positif vérifié"
    )]
    Ok(all.get(index as usize).cloned().unwrap_or(Value::Undefined))
}

fn keys(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let id = this_array(vm, this)?;
    let values = (0..len(vm, id)).map(number).collect();
    Ok(vm.new_iterator(IterState::Values { values, index: 0 }))
}

fn values(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let id = this_array(vm, this)?;
    Ok(vm.new_iterator(IterState::Array {
        target: id,
        index: 0,
    }))
}

fn entries(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let id = this_array(vm, this)?;
    let pairs = items(vm, id)
        .into_iter()
        .enumerate()
        .map(|(i, v)| vm.new_array(vec![number(i), v]))
        .collect();
    Ok(vm.new_iterator(IterState::Values {
        values: pairs,
        index: 0,
    }))
}

fn to_string(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    join(vm, this, &[])
}

pub(super) fn install(vm: &mut Vm) {
    let proto = vm.intrinsics.array_proto;
    let ctor = constructor(vm, "Array", 1, array_ctor, proto);
    vm.define_method(ctor, "isArray", 1, array_is_array);
    vm.define_method(ctor, "of", 0, array_of);
    vm.define_method(ctor, "from", 1, array_from);
    let methods: &[(&str, u32, NativeFn)] = &[
        ("push", 1, push),
        ("pop", 0, pop),
        ("shift", 0, shift),
        ("unshift", 1, unshift),
        ("slice", 2, slice),
        ("splice", 2, splice),
        ("concat", 1, concat),
        ("join", 1, join),
        ("reverse", 0, reverse),
        ("indexOf", 1, index_of),
        ("lastIndexOf", 1, last_index_of),
        ("includes", 1, includes),
        ("forEach", 1, for_each),
        ("map", 1, map),
        ("filter", 1, filter),
        ("find", 1, find),
        ("findIndex", 1, find_index),
        ("findLast", 1, find_last),
        ("findLastIndex", 1, find_last_index),
        ("some", 1, some),
        ("every", 1, every),
        ("reduce", 1, reduce),
        ("reduceRight", 1, reduce_right),
        ("sort", 1, sort),
        ("flat", 0, flat),
        ("flatMap", 1, flat_map),
        ("fill", 1, fill),
        ("at", 1, at),
        ("keys", 0, keys),
        ("values", 0, values),
        ("entries", 0, entries),
        ("toString", 0, to_string),
    ];
    for (name, arity, func) in methods {
        vm.define_method(proto, name, *arity, *func);
    }
}
