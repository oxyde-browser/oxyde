//! `Array` et `Array.prototype`.
//!
//! Les méthodes sont génériques, comme l'exige ECMA-262 : elles opèrent sur
//! tout objet muni d'une propriété `length`, en passant par ses propriétés
//! indexées (trous, accesseurs et prototype compris).
//!
//! LIMITES CONNUES : `Symbol.species` et `Symbol.isConcatSpreadable` ne sont
//! pas consultés (le résultat est toujours un tableau ordinaire, seuls les
//! vrais tableaux sont étalés par `concat`) ; `Array.from` et `Array.of`
//! ignorent leur `this` ; `toLocaleString` vaut `toString` ; une écriture
//! refusée (objet figé) est ignorée au lieu de lever `TypeError`.

use std::sync::Arc;

use crate::heap::{IndexedKind, IterState, Kind};
use crate::value::{ObjId, PropertyKey, Value};
use crate::vm::{JsResult, NativeFn, Vm};

use super::{arg, array_like_to_vec, callable, constructor, is_callable, relative_index};

/// Nombre maximal d'éléments qu'une méthode peut produire d'un coup.
const MAX_LENGTH: usize = 1 << 26;
/// Plus grande longueur d'un objet de type tableau (2^53 - 1).
const MAX_SAFE_LENGTH: f64 = 9_007_199_254_740_991.0;

#[expect(
    clippy::cast_precision_loss,
    reason = "index inférieur à 2^53, exactement représentable"
)]
fn number(n: usize) -> Value {
    Value::Number(n as f64)
}

/// Receveur d'une méthode : l'objet (`ToObject(this)`) et sa longueur.
struct Target {
    object: Value,
    id: ObjId,
    len: usize,
}

/// `LengthOfArrayLike` : `ToLength(obj.length)`.
pub(super) fn length_of(vm: &mut Vm, object: &Value) -> JsResult<usize> {
    let length = vm.get(object, "length")?;
    let length = vm.to_number(&length)?;
    let length = if length.is_nan() {
        0.0
    } else {
        length.trunc().clamp(0.0, MAX_SAFE_LENGTH)
    };
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "entier ramené dans [0, 2^53 - 1]"
    )]
    Ok(length as usize)
}

fn target(vm: &mut Vm, this: &Value) -> JsResult<Target> {
    let object = vm.to_object(this)?;
    let Some(id) = object.as_object() else {
        return Err(vm.type_error("un objet est attendu comme receveur"));
    };
    // Un tableau typé a une longueur interne, que `length` ne peut masquer.
    let len = if vm.typed_array(id).is_some() {
        vm.typed_length(id).unwrap_or(0)
    } else {
        length_of(vm, &object)?
    };
    Ok(Target { object, id, len })
}

fn get(vm: &mut Vm, target: &Target, index: usize) -> JsResult {
    vm.get_element(&target.object, &number(index))
}

fn set(vm: &mut Vm, target: &Target, index: usize, value: Value) -> JsResult<()> {
    vm.set_element(&target.object, &number(index), value)
}

/// Vrai si l'élément `index` existe (propriété propre ou héritée).
fn has(vm: &mut Vm, target: &Target, index: usize) -> JsResult<bool> {
    if let Kind::Array(items) = &vm.heap.get(target.id).kind
        && items.get(index).is_some_and(|v| *v != Value::Hole)
    {
        return Ok(true);
    }
    let key = PropertyKey::String(Arc::from(index.to_string()));
    vm.has_property_op(target.id, &key)
}

fn delete(vm: &mut Vm, target: &Target, index: usize) -> JsResult<()> {
    vm.delete(&target.object, &index.to_string()).map(|_| ())
}

fn set_length(vm: &mut Vm, target: &Target, len: usize) -> JsResult<()> {
    vm.set(&target.object, "length", number(len))
}

/// Refuse de bâtir un résultat démesuré.
fn check_length(vm: &mut Vm, len: usize) -> JsResult<()> {
    if len > MAX_LENGTH {
        return Err(vm.error("RangeError", "tableau trop grand"));
    }
    Ok(())
}

pub(super) fn is_array(vm: &Vm, value: &Value) -> bool {
    let mut current = value.as_object();
    // Un proxy est un tableau si sa cible en est un.
    for _ in 0..10_000 {
        let Some(id) = current else { return false };
        match &vm.heap.get(id).kind {
            Kind::Array(_) => return true,
            Kind::Proxy(proxy) => current = proxy.target,
            _ => return false,
        }
    }
    false
}

/// Éléments d'un vrai tableau sans trou, pour les chemins rapides : dans ce
/// cas aucune lecture ne peut atteindre le prototype ni un accesseur.
fn dense_mut(vm: &mut Vm, id: ObjId) -> Option<&mut Vec<Value>> {
    match &mut vm.heap.get_mut(id).kind {
        Kind::Array(items) if !items.contains(&Value::Hole) => Some(items),
        _ => None,
    }
}

/// `ToIntegerOrInfinity`.
fn to_integer(vm: &mut Vm, value: &Value) -> JsResult<f64> {
    let n = vm.to_number(value)?;
    Ok(if n.is_nan() { 0.0 } else { n.trunc() })
}

// ---- Constructeur ----------------------------------------------------------------

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
    Ok(Value::Bool(is_array(vm, &arg(args, 0))))
}

fn array_of(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    Ok(vm.new_array(args.to_vec()))
}

fn array_from(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let source = arg(args, 0);
    let map = arg(args, 1);
    let this_arg = arg(args, 2);
    if map != Value::Undefined && !is_callable(vm, &map) {
        return Err(vm.type_error("Array.from : la fonction de transformation n'est pas appelable"));
    }
    let values = match &source {
        Value::Undefined | Value::Null | Value::Hole => {
            return Err(vm.type_error("Array.from : la source n'est pas itérable"));
        }
        Value::String(_) => vm.iterate_to_vec(&source)?,
        Value::Object(id) => {
            // Itérable (`Symbol.iterator`), sinon objet de type tableau.
            let key = PropertyKey::Symbol(vm.well_known.iterator.clone());
            let builtin = matches!(
                vm.heap.get(*id).kind,
                Kind::Array(_) | Kind::Map(_) | Kind::Set(_) | Kind::Iterator(_)
            );
            if builtin || !vm.get_key(&source, &key)?.is_nullish() {
                vm.iterate_to_vec(&source)?
            } else {
                array_like_to_vec(vm, &source)?
            }
        }
        _ => Vec::new(),
    };
    if map == Value::Undefined {
        return Ok(vm.new_array(values));
    }
    let mut out = Vec::with_capacity(values.len());
    for (i, value) in values.into_iter().enumerate() {
        out.push(vm.call(&map, &this_arg, &[value, number(i)])?);
    }
    Ok(vm.new_array(out))
}

// ---- Modification en place -------------------------------------------------------

fn push(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let t = target(vm, this)?;
    #[expect(clippy::cast_precision_loss, reason = "comparaison à 2^53 - 1")]
    if (t.len + args.len()) as f64 > MAX_SAFE_LENGTH {
        return Err(vm.type_error("longueur de tableau dépassée"));
    }
    for (offset, value) in args.iter().enumerate() {
        set(vm, &t, t.len + offset, value.clone())?;
    }
    let len = t.len + args.len();
    set_length(vm, &t, len)?;
    Ok(number(len))
}

fn pop(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let t = target(vm, this)?;
    if t.len == 0 {
        set_length(vm, &t, 0)?;
        return Ok(Value::Undefined);
    }
    let value = get(vm, &t, t.len - 1)?;
    delete(vm, &t, t.len - 1)?;
    set_length(vm, &t, t.len - 1)?;
    Ok(value)
}

fn shift(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let t = target(vm, this)?;
    if t.len == 0 {
        set_length(vm, &t, 0)?;
        return Ok(Value::Undefined);
    }
    if let Some(items) = dense_mut(vm, t.id)
        && items.len() == t.len
    {
        return Ok(items.remove(0));
    }
    let first = get(vm, &t, 0)?;
    for k in 1..t.len {
        vm.tick(1)?;
        if has(vm, &t, k)? {
            let value = get(vm, &t, k)?;
            set(vm, &t, k - 1, value)?;
        } else {
            delete(vm, &t, k - 1)?;
        }
    }
    delete(vm, &t, t.len - 1)?;
    set_length(vm, &t, t.len - 1)?;
    Ok(first)
}

fn unshift(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let t = target(vm, this)?;
    let count = args.len();
    if let Some(items) = dense_mut(vm, t.id)
        && items.len() == t.len
    {
        items.splice(0..0, args.iter().cloned());
        return Ok(number(t.len + count));
    }
    if count > 0 {
        #[expect(clippy::cast_precision_loss, reason = "comparaison à 2^53 - 1")]
        if (t.len + count) as f64 > MAX_SAFE_LENGTH {
            return Err(vm.type_error("longueur de tableau dépassée"));
        }
        for k in (0..t.len).rev() {
            vm.tick(1)?;
            if has(vm, &t, k)? {
                let value = get(vm, &t, k)?;
                set(vm, &t, k + count, value)?;
            } else {
                delete(vm, &t, k + count)?;
            }
        }
        for (k, value) in args.iter().enumerate() {
            set(vm, &t, k, value.clone())?;
        }
    }
    set_length(vm, &t, t.len + count)?;
    Ok(number(t.len + count))
}

/// Début et nombre d'éléments à retirer d'un `splice`.
fn splice_bounds(vm: &mut Vm, args: &[Value], len: usize) -> JsResult<(usize, usize)> {
    let start = relative_index(vm, &arg(args, 0), len, 0)?;
    let count = match args.get(1) {
        None if args.is_empty() => 0,
        None => len - start,
        Some(value) => {
            let n = to_integer(vm, value)?;
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                clippy::cast_precision_loss,
                reason = "ramené dans [0, len - start]"
            )]
            let count = n.clamp(0.0, (len - start) as f64) as usize;
            count
        }
    };
    Ok((start, count))
}

fn splice(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let t = target(vm, this)?;
    let (start, count) = splice_bounds(vm, args, t.len)?;
    let inserted = args.get(2..).unwrap_or(&[]);
    if let Some(items) = dense_mut(vm, t.id)
        && items.len() == t.len
    {
        let removed: Vec<Value> = items
            .splice(start..start + count, inserted.iter().cloned())
            .collect();
        return Ok(vm.new_array(removed));
    }
    check_length(vm, count)?;
    let mut removed = Vec::with_capacity(count);
    for k in 0..count {
        vm.tick(1)?;
        removed.push(if has(vm, &t, start + k)? {
            get(vm, &t, start + k)?
        } else {
            Value::Hole
        });
    }
    let added = inserted.len();
    let tail = t.len - start - count;
    if added < count {
        for k in 0..tail {
            vm.tick(1)?;
            let (from, to) = (start + count + k, start + added + k);
            if has(vm, &t, from)? {
                let value = get(vm, &t, from)?;
                set(vm, &t, to, value)?;
            } else {
                delete(vm, &t, to)?;
            }
        }
        for k in (t.len - count + added..t.len).rev() {
            vm.tick(1)?;
            delete(vm, &t, k)?;
        }
    } else if added > count {
        for k in (0..tail).rev() {
            vm.tick(1)?;
            let (from, to) = (start + count + k, start + added + k);
            if has(vm, &t, from)? {
                let value = get(vm, &t, from)?;
                set(vm, &t, to, value)?;
            } else {
                delete(vm, &t, to)?;
            }
        }
    }
    for (k, value) in inserted.iter().enumerate() {
        set(vm, &t, start + k, value.clone())?;
    }
    set_length(vm, &t, t.len - count + added)?;
    Ok(vm.new_array(removed))
}

pub(super) fn reverse(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let t = target(vm, this)?;
    for lower in 0..t.len / 2 {
        vm.tick(1)?;
        let upper = t.len - 1 - lower;
        let (has_lower, has_upper) = (has(vm, &t, lower)?, has(vm, &t, upper)?);
        let low = if has_lower {
            Some(get(vm, &t, lower)?)
        } else {
            None
        };
        let high = if has_upper {
            Some(get(vm, &t, upper)?)
        } else {
            None
        };
        match high {
            Some(value) => set(vm, &t, lower, value)?,
            None => delete(vm, &t, lower)?,
        }
        match low {
            Some(value) => set(vm, &t, upper, value)?,
            None => delete(vm, &t, upper)?,
        }
    }
    Ok(t.object)
}

fn fill(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let t = target(vm, this)?;
    let start = relative_index(vm, &arg(args, 1), t.len, 0)?;
    let end = relative_index(vm, &arg(args, 2), t.len, t.len)?;
    let value = arg(args, 0);
    for k in start..end.max(start) {
        vm.tick(1)?;
        set(vm, &t, k, value.clone())?;
    }
    Ok(t.object)
}

pub(super) fn copy_within(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let t = target(vm, this)?;
    let to = relative_index(vm, &arg(args, 0), t.len, 0)?;
    let from = relative_index(vm, &arg(args, 1), t.len, 0)?;
    let end = relative_index(vm, &arg(args, 2), t.len, t.len)?;
    let count = end.saturating_sub(from).min(t.len - to);
    // Copie à reculons si les plages se chevauchent vers la droite.
    let backwards = from < to && to < from + count;
    for step in 0..count {
        vm.tick(1)?;
        let offset = if backwards { count - 1 - step } else { step };
        if has(vm, &t, from + offset)? {
            let value = get(vm, &t, from + offset)?;
            set(vm, &t, to + offset, value)?;
        } else {
            delete(vm, &t, to + offset)?;
        }
    }
    Ok(t.object)
}

// ---- Copies ----------------------------------------------------------------------

fn slice(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let t = target(vm, this)?;
    let start = relative_index(vm, &arg(args, 0), t.len, 0)?;
    let end = relative_index(vm, &arg(args, 1), t.len, t.len)?;
    let count = end.saturating_sub(start);
    check_length(vm, count)?;
    let mut out = Vec::with_capacity(count);
    for k in start..start + count {
        vm.tick(1)?;
        out.push(if has(vm, &t, k)? {
            get(vm, &t, k)?
        } else {
            Value::Hole
        });
    }
    Ok(vm.new_array(out))
}

fn concat(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let receiver = vm.to_object(this)?;
    let mut out = Vec::new();
    let spreadable_key = PropertyKey::Symbol(vm.well_known.is_concat_spreadable.clone());
    for value in std::iter::once(&receiver).chain(args) {
        // `IsConcatSpreadable` : la propriété l'emporte sur la nature de l'objet.
        let spreadable = if value.as_object().is_some() {
            match vm.get_key(value, &spreadable_key)? {
                Value::Undefined => is_array(vm, value),
                flag => flag.truthy(),
            }
        } else {
            false
        };
        if spreadable {
            let t = target(vm, value)?;
            check_length(vm, out.len().saturating_add(t.len))?;
            for k in 0..t.len {
                vm.tick(1)?;
                out.push(if has(vm, &t, k)? {
                    get(vm, &t, k)?
                } else {
                    Value::Hole
                });
            }
        } else {
            out.push(value.clone());
        }
        check_length(vm, out.len())?;
    }
    Ok(vm.new_array(out))
}

pub(super) fn join(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let t = target(vm, this)?;
    let separator = match arg(args, 0) {
        Value::Undefined => Arc::from(","),
        other => vm.to_string(&other)?,
    };
    let mut out = String::new();
    for k in 0..t.len {
        vm.tick(1)?;
        if k > 0 {
            out.push_str(&separator);
        }
        let value = get(vm, &t, k)?;
        if !value.is_nullish() {
            out.push_str(&vm.to_string(&value)?);
        }
        if out.len() > super::string::MAX_STRING {
            return Err(vm.error("RangeError", "chaîne trop longue"));
        }
    }
    Ok(Value::string(&out))
}

fn to_string(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let object = vm.to_object(this)?;
    let join_method = vm.get(&object, "join")?;
    if is_callable(vm, &join_method) {
        return vm.call(&join_method, &object, &[]);
    }
    super::object_proto_to_string(vm, &object, &[])
}

/// Copie des éléments `0..len`, trous lus comme `undefined`.
fn snapshot(vm: &mut Vm, t: &Target) -> JsResult<Vec<Value>> {
    check_length(vm, t.len)?;
    let mut out = Vec::with_capacity(t.len);
    for k in 0..t.len {
        vm.tick(1)?;
        out.push(get(vm, t, k)?);
    }
    Ok(out)
}

fn to_reversed(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let t = target(vm, this)?;
    let mut out = snapshot(vm, &t)?;
    out.reverse();
    Ok(vm.new_array(out))
}

fn to_spliced(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let t = target(vm, this)?;
    let (start, count) = splice_bounds(vm, args, t.len)?;
    let inserted = args.get(2..).unwrap_or(&[]);
    check_length(vm, t.len - count + inserted.len())?;
    let mut out = snapshot(vm, &t)?;
    out.splice(start..start + count, inserted.iter().cloned());
    Ok(vm.new_array(out))
}

fn with(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let t = target(vm, this)?;
    let relative = to_integer(vm, &arg(args, 0))?;
    #[expect(clippy::cast_precision_loss, reason = "longueur petite devant 2^53")]
    let index = if relative < 0.0 {
        t.len as f64 + relative
    } else {
        relative
    };
    #[expect(clippy::cast_precision_loss, reason = "longueur petite devant 2^53")]
    if index < 0.0 || index >= t.len as f64 {
        return Err(vm.error("RangeError", "index hors du tableau"));
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "index vérifié dans [0, len)"
    )]
    let replaced = index as usize;
    check_length(vm, t.len)?;
    let mut out = Vec::with_capacity(t.len);
    for k in 0..t.len {
        vm.tick(1)?;
        // L'élément remplacé n'est pas lu.
        out.push(if k == replaced {
            arg(args, 1)
        } else {
            get(vm, &t, k)?
        });
    }
    Ok(vm.new_array(out))
}

// ---- Recherche -------------------------------------------------------------------

pub(super) fn index_of(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let t = target(vm, this)?;
    if t.len == 0 {
        return Ok(Value::Number(-1.0));
    }
    let from = relative_index(vm, &arg(args, 1), t.len, 0)?;
    let needle = arg(args, 0);
    for k in from..t.len {
        vm.tick(1)?;
        if has(vm, &t, k)? && Vm::strict_equals(&get(vm, &t, k)?, &needle) {
            return Ok(number(k));
        }
    }
    Ok(Value::Number(-1.0))
}

pub(super) fn last_index_of(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let t = target(vm, this)?;
    if t.len == 0 {
        return Ok(Value::Number(-1.0));
    }
    #[expect(clippy::cast_precision_loss, reason = "longueur petite devant 2^53")]
    let len = t.len as f64;
    let from = match args.get(1) {
        Some(value) => to_integer(vm, value)?,
        None => len - 1.0,
    };
    let last = if from >= 0.0 {
        from.min(len - 1.0)
    } else {
        len + from
    };
    if last < 0.0 {
        return Ok(Value::Number(-1.0));
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "ramené dans [0, len - 1]"
    )]
    let last = last as usize;
    let needle = arg(args, 0);
    for k in (0..=last).rev() {
        vm.tick(1)?;
        if has(vm, &t, k)? && Vm::strict_equals(&get(vm, &t, k)?, &needle) {
            return Ok(number(k));
        }
    }
    Ok(Value::Number(-1.0))
}

pub(super) fn includes(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let t = target(vm, this)?;
    if t.len == 0 {
        return Ok(Value::Bool(false));
    }
    let from = relative_index(vm, &arg(args, 1), t.len, 0)?;
    let needle = arg(args, 0);
    for k in from..t.len {
        vm.tick(1)?;
        if Vm::same_value_zero(&get(vm, &t, k)?, &needle) {
            return Ok(Value::Bool(true));
        }
    }
    Ok(Value::Bool(false))
}

pub(super) fn at(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let t = target(vm, this)?;
    let relative = to_integer(vm, &arg(args, 0))?;
    #[expect(clippy::cast_precision_loss, reason = "longueur petite devant 2^53")]
    let index = if relative < 0.0 {
        t.len as f64 + relative
    } else {
        relative
    };
    #[expect(clippy::cast_precision_loss, reason = "longueur petite devant 2^53")]
    if index < 0.0 || index >= t.len as f64 {
        return Ok(Value::Undefined);
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "index vérifié dans [0, len)"
    )]
    get(vm, &t, index as usize)
}

// ---- Parcours avec rappel --------------------------------------------------------

/// Appelle `callback(élément, index, objet)` sur chaque élément. `skip_holes`
/// : les éléments absents ne sont pas visités (`forEach`, `map`…) ; sinon ils
/// sont lus comme `undefined` (`find`…). `visit` reçoit l'index, l'élément et
/// le résultat du rappel, et renvoie `false` pour arrêter.
fn each(
    vm: &mut Vm,
    this: &Value,
    args: &[Value],
    reverse: bool,
    skip_holes: bool,
    mut visit: impl FnMut(&mut Vm, usize, Value, Value) -> JsResult<bool>,
) -> JsResult<Target> {
    let t = target(vm, this)?;
    let callback = callable(vm, &arg(args, 0))?;
    let this_arg = arg(args, 1);
    for step in 0..t.len {
        vm.tick(1)?;
        let k = if reverse { t.len - 1 - step } else { step };
        if skip_holes && !has(vm, &t, k)? {
            continue;
        }
        let value = get(vm, &t, k)?;
        let result = vm.call(
            &callback,
            &this_arg,
            &[value.clone(), number(k), t.object.clone()],
        )?;
        if !visit(vm, k, value, result)? {
            break;
        }
    }
    Ok(t)
}

pub(super) fn for_each(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    each(vm, this, args, false, true, |_, _, _, _| Ok(true))?;
    Ok(Value::Undefined)
}

fn map(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let mut out: Vec<(usize, Value)> = Vec::new();
    let t = each(vm, this, args, false, true, |_, k, _, result| {
        out.push((k, result));
        Ok(true)
    })?;
    check_length(vm, t.len)?;
    // Les éléments absents de la source restent absents du résultat.
    let mut items = vec![Value::Hole; t.len];
    for (k, value) in out {
        if let Some(slot) = items.get_mut(k) {
            *slot = value;
        }
    }
    Ok(vm.new_array(items))
}

fn filter(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let mut out = Vec::new();
    each(vm, this, args, false, true, |_, _, value, result| {
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
    each(vm, this, args, reverse, false, |_, k, value, result| {
        if result.truthy() {
            found = if want_index { number(k) } else { value };
            return Ok(false);
        }
        Ok(true)
    })?;
    Ok(found)
}

pub(super) fn find(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    find_impl(vm, this, args, false, false)
}

pub(super) fn find_index(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    find_impl(vm, this, args, false, true)
}

pub(super) fn find_last(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    find_impl(vm, this, args, true, false)
}

pub(super) fn find_last_index(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    find_impl(vm, this, args, true, true)
}

pub(super) fn some(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let mut any = false;
    each(vm, this, args, false, true, |_, _, _, result| {
        any = result.truthy();
        Ok(!any)
    })?;
    Ok(Value::Bool(any))
}

pub(super) fn every(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let mut all = true;
    each(vm, this, args, false, true, |_, _, _, result| {
        all = result.truthy();
        Ok(all)
    })?;
    Ok(Value::Bool(all))
}

fn reduce_impl(vm: &mut Vm, this: &Value, args: &[Value], reverse: bool) -> JsResult {
    let t = target(vm, this)?;
    let callback = callable(vm, &arg(args, 0))?;
    let index = |step: usize| if reverse { t.len - 1 - step } else { step };
    let mut step = 0;
    let mut acc = match args.get(1) {
        Some(initial) => initial.clone(),
        None => {
            // Premier élément présent.
            let mut first = None;
            while step < t.len && first.is_none() {
                vm.tick(1)?;
                let k = index(step);
                step += 1;
                if has(vm, &t, k)? {
                    first = Some(get(vm, &t, k)?);
                }
            }
            match first {
                Some(value) => value,
                None => {
                    return Err(vm.type_error("reduce sur un tableau vide sans valeur initiale"));
                }
            }
        }
    };
    while step < t.len {
        vm.tick(1)?;
        let k = index(step);
        step += 1;
        if has(vm, &t, k)? {
            let value = get(vm, &t, k)?;
            acc = vm.call(
                &callback,
                &Value::Undefined,
                &[acc, value, number(k), t.object.clone()],
            )?;
        }
    }
    Ok(acc)
}

pub(super) fn reduce(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    reduce_impl(vm, this, args, false)
}

pub(super) fn reduce_right(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    reduce_impl(vm, this, args, true)
}

// ---- Tri -------------------------------------------------------------------------

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

fn comparator_arg(vm: &mut Vm, args: &[Value]) -> JsResult<Value> {
    let comparator = arg(args, 0);
    if comparator != Value::Undefined && !is_callable(vm, &comparator) {
        return Err(vm.type_error("le comparateur de tri doit être une fonction"));
    }
    Ok(comparator)
}

/// Trie `values` : les `undefined` vont à la fin sans appeler le comparateur.
fn sort_values(vm: &mut Vm, values: Vec<Value>, comparator: &Value) -> JsResult<Vec<Value>> {
    let (defined, undefined): (Vec<Value>, Vec<Value>) =
        values.into_iter().partition(|v| *v != Value::Undefined);
    let mut less_or_equal = |vm: &mut Vm, a: &Value, b: &Value| -> JsResult<bool> {
        if *comparator == Value::Undefined {
            let (x, y) = (vm.to_string(a)?, vm.to_string(b)?);
            return Ok(x.encode_utf16().le(y.encode_utf16()));
        }
        let result = vm.call(comparator, &Value::Undefined, &[a.clone(), b.clone()])?;
        // NaN compte pour zéro : les éléments gardent leur ordre.
        Ok(vm.to_number(&result)?.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater))
    };
    let mut sorted = merge_sort(vm, defined, &mut less_or_equal)?;
    sorted.extend(undefined);
    Ok(sorted)
}

fn sort(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let comparator = comparator_arg(vm, args)?;
    let t = target(vm, this)?;
    check_length(vm, t.len)?;
    // Seuls les éléments présents sont triés ; les trous finissent à la fin.
    let mut values = Vec::new();
    for k in 0..t.len {
        vm.tick(1)?;
        if has(vm, &t, k)? {
            values.push(get(vm, &t, k)?);
        }
    }
    let sorted = sort_values(vm, values, &comparator)?;
    let count = sorted.len();
    for (k, value) in sorted.into_iter().enumerate() {
        set(vm, &t, k, value)?;
    }
    for k in count..t.len {
        vm.tick(1)?;
        if has(vm, &t, k)? {
            delete(vm, &t, k)?;
        }
    }
    Ok(t.object)
}

fn to_sorted(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let comparator = comparator_arg(vm, args)?;
    let t = target(vm, this)?;
    let values = snapshot(vm, &t)?;
    let sorted = sort_values(vm, values, &comparator)?;
    Ok(vm.new_array(sorted))
}

// ---- Aplatissement ---------------------------------------------------------------

fn flatten(vm: &mut Vm, source: &Value, depth: usize, out: &mut Vec<Value>) -> JsResult<()> {
    let t = target(vm, source)?;
    for k in 0..t.len {
        vm.tick(1)?;
        if !has(vm, &t, k)? {
            continue;
        }
        let value = get(vm, &t, k)?;
        if depth > 0 && is_array(vm, &value) {
            flatten(vm, &value, depth - 1, out)?;
        } else {
            out.push(value);
        }
        check_length(vm, out.len())?;
    }
    Ok(())
}

fn flat(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let object = vm.to_object(this)?;
    let depth = match arg(args, 0) {
        Value::Undefined => 1.0,
        other => to_integer(vm, &other)?,
    };
    // Borne la récursion native : au-delà, le résultat ne change plus en pratique.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "ramené dans [0, 64]"
    )]
    let depth = depth.clamp(0.0, 64.0) as usize;
    let mut out = Vec::new();
    flatten(vm, &object, depth, &mut out)?;
    Ok(vm.new_array(out))
}

fn flat_map(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let mut mapped = Vec::new();
    each(vm, this, args, false, true, |_, _, _, result| {
        mapped.push(result);
        Ok(true)
    })?;
    let mut out = Vec::new();
    for value in mapped {
        if is_array(vm, &value) {
            flatten(vm, &value, 0, &mut out)?;
        } else {
            out.push(value);
        }
        check_length(vm, out.len())?;
    }
    Ok(vm.new_array(out))
}

// ---- Itérateurs ------------------------------------------------------------------

fn indexed_iterator(vm: &mut Vm, this: &Value, kind: IndexedKind) -> JsResult {
    let target = vm.to_object(this)?;
    Ok(vm.new_iterator(IterState::Indexed {
        target,
        index: 0,
        kind,
    }))
}

fn keys(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    indexed_iterator(vm, this, IndexedKind::Keys)
}

fn values(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    indexed_iterator(vm, this, IndexedKind::Values)
}

fn entries(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    indexed_iterator(vm, this, IndexedKind::Entries)
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
        ("copyWithin", 2, copy_within),
        ("at", 1, at),
        ("keys", 0, keys),
        ("values", 0, values),
        ("entries", 0, entries),
        ("toReversed", 0, to_reversed),
        ("toSorted", 1, to_sorted),
        ("toSpliced", 2, to_spliced),
        ("with", 2, with),
        ("toString", 0, to_string),
        ("toLocaleString", 0, to_string),
    ];
    for (name, arity, func) in methods {
        vm.define_method(proto, name, *arity, *func);
    }
}
