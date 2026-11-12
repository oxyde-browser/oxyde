//! `Iterator` et ses aides (ECMA-262 §27.1) : `map`, `filter`, `take`, `drop`,
//! `flatMap`, `reduce`, `toArray`, `forEach`, `some`, `every`, `find`,
//! `includes`, `join`, `chunks`, `windows`, `Iterator.from`, `Iterator.concat`,
//! `Iterator.zip` et `Iterator.zipKeyed`.
//!
//! Une aide paresseuse rend un objet `Kind::IteratorHelper` qui garde
//! l'itérateur sous-jacent et avance d'un pas à chaque `next()`.

use std::sync::Arc;

use super::{arg, is_callable};
use crate::heap::{HelperKind, IteratorHelper, Kind, Object, Property, Slot, ZipMode, ZipState};
use crate::internal::Descriptor;
use crate::value::{ObjId, PropertyKey, Value};
use crate::vm::{JsResult, NativeFn, Vm};

/// Longueur maximale de la chaîne que produit `join`.
const MAX_JOINED: usize = 1 << 28;

// ---- Protocole ---------------------------------------------------------------

/// `IteratorStepValue` : valeur suivante, ou `None` quand l'itérateur est épuisé.
fn step(vm: &mut Vm, iterator: &Value, next: &Value) -> JsResult<Option<Value>> {
    vm.tick(1)?;
    let result = vm.call(next, iterator, &[])?;
    if result.as_object().is_none() {
        return Err(vm.type_error("le résultat d'un itérateur doit être un objet"));
    }
    if vm.get(&result, "done")?.truthy() {
        return Ok(None);
    }
    vm.get(&result, "value").map(Some)
}

/// `IteratorClose` après une fin normale : les erreurs de `return()` remontent.
fn close(vm: &mut Vm, iterator: &Value) -> JsResult<()> {
    if iterator.as_object().is_none() {
        return Ok(());
    }
    let method = vm.get(iterator, "return")?;
    if method.is_nullish() {
        return Ok(());
    }
    let result = vm.call(&method, iterator, &[])?;
    if result.as_object().is_none() {
        return Err(vm.type_error("le résultat de « return » doit être un objet"));
    }
    Ok(())
}

/// `IteratorClose` après une exception, qui l'emporte sur celles de `return()`.
fn close_quiet(vm: &mut Vm, iterator: &Value) {
    if iterator.as_object().is_some()
        && let Ok(method) = vm.get(iterator, "return")
        && !method.is_nullish()
    {
        let _ = vm.call(&method, iterator, &[]);
    }
}

/// Ferme `iterator` puis rend l'erreur `error` à lever.
fn fail<T>(vm: &mut Vm, iterator: &Value, error: Value) -> JsResult<T> {
    close_quiet(vm, iterator);
    Err(error)
}

/// `IteratorStep` : avance d'un pas sans lire la valeur ; vrai à la fin.
fn step_done(vm: &mut Vm, iterator: &Value, next: &Value) -> JsResult<bool> {
    vm.tick(1)?;
    let result = vm.call(next, iterator, &[])?;
    if result.as_object().is_none() {
        return Err(vm.type_error("le résultat d'un itérateur doit être un objet"));
    }
    Ok(vm.get(&result, "done")?.truthy())
}

/// `IteratorCloseAll` : ferme `iterators` du dernier au premier. L'exception
/// en cours (`error`) l'emporte ; sinon c'est la première que lève un
/// `return()`.
fn close_all(vm: &mut Vm, iterators: &[Value], mut error: Option<Value>) -> JsResult<()> {
    for iterator in iterators.iter().rev() {
        if error.is_some() {
            close_quiet(vm, iterator);
        } else if let Err(raised) = close(vm, iterator) {
            error = Some(raised);
        }
    }
    error.map_or(Ok(()), Err)
}

/// `GetIterator(valeur, sync)` : itérateur de `value` et sa méthode `next`.
fn get_iterator(vm: &mut Vm, value: &Value) -> JsResult<(Value, Value)> {
    let key = PropertyKey::Symbol(vm.well_known.iterator.clone());
    let method = vm.get_key(value, &key)?;
    if !is_callable(vm, &method) {
        return Err(vm.type_error("la valeur n'est pas itérable"));
    }
    let iterator = vm.call(&method, value, &[])?;
    if iterator.as_object().is_none() {
        return Err(vm.type_error("un itérateur doit être un objet"));
    }
    let next = vm.get(&iterator, "next")?;
    Ok((iterator, next))
}

/// `this` d'une méthode de `Iterator.prototype` : doit être un objet.
fn receiver(vm: &mut Vm, this: &Value, method: &str) -> JsResult<Value> {
    if this.as_object().is_none() {
        return Err(vm.type_error(&format!(
            "Iterator.prototype.{method} : le receveur doit être un objet"
        )));
    }
    Ok(this.clone())
}

/// Fonction de rappel d'une aide : si elle n'est pas appelable, l'itérateur
/// est fermé avant de lever `TypeError`.
fn callback(vm: &mut Vm, iterator: &Value, value: &Value, method: &str) -> JsResult<Value> {
    if is_callable(vm, value) {
        return Ok(value.clone());
    }
    let error = vm.type_error(&format!(
        "Iterator.prototype.{method} : une fonction est attendue"
    ));
    fail(vm, iterator, error)
}

/// `GetIteratorFlattenable` (ECMA-262 §27.1.3.2.1) : itérateur et sa méthode `next`.
fn flattenable(vm: &mut Vm, value: &Value, strings: bool) -> JsResult<(Value, Value)> {
    let accepted = value.as_object().is_some() || (strings && matches!(value, Value::String(_)));
    if !accepted {
        return Err(vm.type_error("un itérable ou un itérateur est attendu"));
    }
    let key = PropertyKey::Symbol(vm.well_known.iterator.clone());
    let method = vm.get_key(value, &key)?;
    let iterator = if method.is_nullish() {
        value.clone()
    } else {
        if !is_callable(vm, &method) {
            return Err(vm.type_error("Symbol.iterator n'est pas une fonction"));
        }
        vm.call(&method, value, &[])?
    };
    if iterator.as_object().is_none() {
        return Err(vm.type_error("un itérateur doit être un objet"));
    }
    let next = vm.get(&iterator, "next")?;
    Ok((iterator, next))
}

// ---- Aides paresseuses ---------------------------------------------------------

fn new_helper(kind: HelperKind, iterator: Value, next: Value) -> IteratorHelper {
    IteratorHelper {
        kind,
        iterator,
        next,
        func: Value::Undefined,
        counter: 0.0,
        remaining: 0.0,
        inner: None,
        sources: Vec::new(),
        buffer: Vec::new(),
        zip: None,
        started: false,
        running: false,
        done: false,
    }
}

fn alloc_helper(vm: &mut Vm, helper: IteratorHelper) -> Value {
    let proto = if helper.kind == HelperKind::Wrap {
        vm.intrinsics.wrap_iterator_proto
    } else {
        vm.intrinsics.iterator_helper_proto
    };
    Value::Object(vm.heap.alloc(Object::new(
        Some(proto),
        Kind::IteratorHelper(Box::new(helper)),
    )))
}

fn helper_mut(vm: &mut Vm, id: ObjId) -> Option<&mut IteratorHelper> {
    match &mut vm.heap.get_mut(id).kind {
        Kind::IteratorHelper(helper) => Some(helper),
        _ => None,
    }
}

/// Aide paresseuse désignée par `this` (hors enveloppe de `Iterator.from`).
fn this_helper(vm: &mut Vm, this: &Value, wrap: bool) -> JsResult<ObjId> {
    if let Some(id) = this.as_object()
        && let Kind::IteratorHelper(helper) = &vm.heap.get(id).kind
        && helper.kind != HelperKind::RegExpString
        && (helper.kind == HelperKind::Wrap) == wrap
    {
        return Ok(id);
    }
    Err(vm.type_error("le receveur n'est pas un objet d'aide d'itérateur"))
}

/// Appelle `func(value, compteur)` ; une exception ferme l'itérateur sous-jacent.
fn apply(vm: &mut Vm, id: ObjId, value: &Value) -> JsResult<Value> {
    let Some(helper) = helper_mut(vm, id) else {
        return Ok(Value::Undefined);
    };
    let (func, iterator, counter) = (helper.func.clone(), helper.iterator.clone(), helper.counter);
    helper.counter += 1.0;
    match vm.call(
        &func,
        &Value::Undefined,
        &[value.clone(), Value::Number(counter)],
    ) {
        Ok(result) => Ok(result),
        Err(error) => fail(vm, &iterator, error),
    }
}

/// Avance l'aide `id` d'un pas : sa prochaine valeur, ou `None` à la fin.
fn advance(vm: &mut Vm, id: ObjId) -> JsResult<Option<Value>> {
    let Some(helper) = helper_mut(vm, id) else {
        return Ok(None);
    };
    let (kind, iterator, next) = (helper.kind, helper.iterator.clone(), helper.next.clone());
    match kind {
        HelperKind::Map => {
            let Some(value) = step(vm, &iterator, &next)? else {
                return Ok(None);
            };
            apply(vm, id, &value).map(Some)
        }
        HelperKind::Filter => loop {
            let Some(value) = step(vm, &iterator, &next)? else {
                return Ok(None);
            };
            if apply(vm, id, &value)?.truthy() {
                return Ok(Some(value));
            }
        },
        HelperKind::Take => {
            let remaining = helper.remaining;
            if remaining == 0.0 {
                close(vm, &iterator)?;
                return Ok(None);
            }
            if remaining.is_finite() {
                helper.remaining -= 1.0;
            }
            step(vm, &iterator, &next)
        }
        HelperKind::Drop => {
            loop {
                let Some(helper) = helper_mut(vm, id) else {
                    return Ok(None);
                };
                if helper.remaining <= 0.0 {
                    break;
                }
                if helper.remaining.is_finite() {
                    helper.remaining -= 1.0;
                }
                if step(vm, &iterator, &next)?.is_none() {
                    return Ok(None);
                }
            }
            step(vm, &iterator, &next)
        }
        HelperKind::FlatMap => loop {
            let inner = helper_mut(vm, id).and_then(|h| h.inner.clone());
            if let Some((inner_iterator, inner_next)) = inner {
                match step(vm, &inner_iterator, &inner_next) {
                    Ok(Some(value)) => return Ok(Some(value)),
                    Ok(None) => {
                        if let Some(helper) = helper_mut(vm, id) {
                            helper.inner = None;
                        }
                    }
                    Err(error) => return fail(vm, &iterator, error),
                }
            }
            let Some(value) = step(vm, &iterator, &next)? else {
                return Ok(None);
            };
            let mapped = apply(vm, id, &value)?;
            let inner = match flattenable(vm, &mapped, false) {
                Ok(inner) => inner,
                Err(error) => return fail(vm, &iterator, error),
            };
            if let Some(helper) = helper_mut(vm, id) {
                helper.inner = Some(inner);
            }
        },
        HelperKind::Concat => loop {
            let inner = helper_mut(vm, id).and_then(|h| h.inner.clone());
            if let Some((inner_iterator, inner_next)) = inner {
                if let Some(value) = step(vm, &inner_iterator, &inner_next)? {
                    return Ok(Some(value));
                }
                if let Some(helper) = helper_mut(vm, id) {
                    helper.inner = None;
                }
            }
            let Some(helper) = helper_mut(vm, id) else {
                return Ok(None);
            };
            if helper.sources.is_empty() {
                return Ok(None);
            }
            let (method, iterable) = helper.sources.remove(0);
            let inner_iterator = vm.call(&method, &iterable, &[])?;
            if inner_iterator.as_object().is_none() {
                return Err(vm.type_error("Iterator.concat : un itérateur doit être un objet"));
            }
            let inner_next = vm.get(&inner_iterator, "next")?;
            if let Some(helper) = helper_mut(vm, id) {
                helper.inner = Some((inner_iterator, inner_next));
            }
        },
        HelperKind::Chunks | HelperKind::Windows => {
            // Taille nulle : l'itérateur sous-jacent est déjà épuisé.
            if helper.remaining < 1.0 {
                return Ok(None);
            }
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "taille validée dans [1, 2^32 - 1]"
            )]
            let size = helper.remaining as usize;
            let partial = helper.counter != 0.0;
            loop {
                let value = step(vm, &iterator, &next)?;
                let Some(helper) = helper_mut(vm, id) else {
                    return Ok(None);
                };
                let ready = match value {
                    None => {
                        helper.remaining = 0.0;
                        let rest = std::mem::take(&mut helper.buffer);
                        // Un reste incomplet : toujours rendu par `chunks`,
                        // et par `windows` si on l'a demandé.
                        let wanted = !rest.is_empty()
                            && (kind == HelperKind::Chunks || (partial && rest.len() < size));
                        if !wanted {
                            return Ok(None);
                        }
                        rest
                    }
                    Some(value) => {
                        if kind == HelperKind::Windows && helper.buffer.len() == size {
                            helper.buffer.remove(0);
                        }
                        helper.buffer.push(value);
                        if helper.buffer.len() < size {
                            continue;
                        }
                        if kind == HelperKind::Chunks {
                            std::mem::take(&mut helper.buffer)
                        } else {
                            helper.buffer.clone()
                        }
                    }
                };
                return Ok(Some(vm.new_array(ready)));
            }
        }
        HelperKind::Zip => {
            // L'état quitte l'aide le temps du pas : `running` interdit d'y
            // revenir entre-temps.
            let Some(mut state) = helper.zip.take() else {
                return Ok(None);
            };
            let outcome = zip_step(vm, &mut state);
            if let Some(helper) = helper_mut(vm, id) {
                helper.zip = Some(state);
            }
            outcome
        }
        HelperKind::Wrap | HelperKind::RegExpString => Ok(None),
    }
}

/// Ferme les itérateurs encore ouverts de `state`, du dernier au premier.
fn zip_close(vm: &mut Vm, state: &mut ZipState, error: Option<Value>) -> JsResult<()> {
    let open: Vec<Value> = state.open.iter_mut().filter_map(Option::take).collect();
    close_all(vm, &open, error)
}

/// Un pas de `Iterator.zip` : le prochain groupe de valeurs, ou `None`.
fn zip_step(vm: &mut Vm, state: &mut ZipState) -> JsResult<Option<Value>> {
    let count = state.iters.len();
    if count == 0 {
        return Ok(None);
    }
    let padding =
        |state: &ZipState, i: usize| state.padding.get(i).cloned().unwrap_or(Value::Undefined);
    let mut results = Vec::with_capacity(count);
    for i in 0..count {
        let Some((iterator, next)) = state.iters.get(i).cloned().flatten() else {
            results.push(padding(state, i));
            continue;
        };
        let value = match step(vm, &iterator, &next) {
            Ok(value) => value,
            Err(error) => {
                // Un itérateur qui lève n'est pas fermé ; les autres le sont.
                if let Some(slot) = state.open.get_mut(i) {
                    *slot = None;
                }
                zip_close(vm, state, Some(error))?;
                return Ok(None);
            }
        };
        let Some(value) = value else {
            if let Some(slot) = state.open.get_mut(i) {
                *slot = None;
            }
            match state.mode {
                ZipMode::Shortest => {
                    zip_close(vm, state, None)?;
                    return Ok(None);
                }
                ZipMode::Strict => {
                    if i != 0 {
                        let error =
                            vm.type_error("Iterator.zip : itérateurs de longueurs inégales");
                        zip_close(vm, state, Some(error))?;
                        return Ok(None);
                    }
                    // Le premier est épuisé : tous les autres doivent l'être.
                    for k in 1..count {
                        let Some((other, other_next)) = state.iters.get(k).cloned().flatten()
                        else {
                            continue;
                        };
                        // Épuisé ou en erreur, il n'est plus à fermer ; encore
                        // en cours, il l'est avec les autres.
                        let error = match step_done(vm, &other, &other_next) {
                            Ok(true) => None,
                            Ok(false) => {
                                let error = vm
                                    .type_error("Iterator.zip : itérateurs de longueurs inégales");
                                zip_close(vm, state, Some(error))?;
                                return Ok(None);
                            }
                            Err(error) => Some(error),
                        };
                        if let Some(slot) = state.open.get_mut(k) {
                            *slot = None;
                        }
                        if let Some(error) = error {
                            zip_close(vm, state, Some(error))?;
                            return Ok(None);
                        }
                    }
                    return Ok(None);
                }
                ZipMode::Longest => {
                    if state.open.iter().all(Option::is_none) {
                        return Ok(None);
                    }
                    if let Some(slot) = state.iters.get_mut(i) {
                        *slot = None;
                    }
                    results.push(padding(state, i));
                    continue;
                }
            }
        };
        results.push(value);
    }
    let Some(keys) = &state.keys else {
        return Ok(Some(vm.new_array(results)));
    };
    // `zipKeyed` : un objet sans prototype, une propriété par clé.
    let object = vm.heap.alloc(Object::new(None, Kind::Ordinary));
    for (key, value) in keys.clone().iter().zip(results) {
        vm.create_data_property(object, key, value)?;
    }
    Ok(Some(Value::Object(object)))
}

/// `%IteratorHelperPrototype%.next`.
fn helper_next(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let id = this_helper(vm, this, false)?;
    let Some(helper) = helper_mut(vm, id) else {
        return Ok(Value::Undefined);
    };
    if helper.running {
        return Err(vm.type_error("l'aide d'itérateur est déjà en cours d'exécution"));
    }
    if helper.done {
        return Ok(vm.iter_result(Value::Undefined, true));
    }
    helper.running = true;
    helper.started = true;
    let outcome = advance(vm, id);
    if let Some(helper) = helper_mut(vm, id) {
        helper.running = false;
        helper.done = !matches!(outcome, Ok(Some(_)));
    }
    Ok(match outcome? {
        Some(value) => vm.iter_result(value, false),
        None => vm.iter_result(Value::Undefined, true),
    })
}

/// `%IteratorHelperPrototype%.return`.
fn helper_return(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let id = this_helper(vm, this, false)?;
    let Some(helper) = helper_mut(vm, id) else {
        return Ok(Value::Undefined);
    };
    if helper.running {
        return Err(vm.type_error("l'aide d'itérateur est déjà en cours d'exécution"));
    }
    if !std::mem::replace(&mut helper.done, true) {
        let (iterator, inner) = (helper.iterator.clone(), helper.inner.take());
        let zip = helper.zip.take();
        // Une aide déjà entamée est en cours d'exécution tant qu'elle ferme
        // ses itérateurs ; une aide jamais entamée est simplement terminée.
        helper.running = helper.started;
        let closed = (|| {
            if let Some(mut state) = zip {
                return zip_close(vm, &mut state, None);
            }
            // L'itérateur interne (`flatMap`, `concat`) est fermé le premier.
            if let Some((inner_iterator, _)) = inner
                && let Err(error) = close(vm, &inner_iterator)
            {
                return fail(vm, &iterator, error);
            }
            close(vm, &iterator)
        })();
        if let Some(helper) = helper_mut(vm, id) {
            helper.running = false;
        }
        closed?;
    }
    Ok(vm.iter_result(Value::Undefined, true))
}

/// Crée l'aide `kind` sur `this`, avec la fonction de rappel `args[0]`.
fn lazy(vm: &mut Vm, this: &Value, args: &[Value], kind: HelperKind, name: &str) -> JsResult {
    let iterator = receiver(vm, this, name)?;
    let func = callback(vm, &iterator, &arg(args, 0), name)?;
    let next = vm.get(&iterator, "next")?;
    let mut helper = new_helper(kind, iterator, next);
    helper.func = func;
    Ok(alloc_helper(vm, helper))
}

fn iterator_map(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    lazy(vm, this, args, HelperKind::Map, "map")
}

fn iterator_filter(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    lazy(vm, this, args, HelperKind::Filter, "filter")
}

fn iterator_flat_map(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    lazy(vm, this, args, HelperKind::FlatMap, "flatMap")
}

/// `take` et `drop` : la limite est un entier positif ou l'infini.
fn limited(vm: &mut Vm, this: &Value, args: &[Value], kind: HelperKind, name: &str) -> JsResult {
    let iterator = receiver(vm, this, name)?;
    let limit = match vm.to_number(&arg(args, 0)) {
        Ok(limit) => limit,
        Err(error) => return fail(vm, &iterator, error),
    };
    if limit.is_nan()
        || limit.trunc() < 0.0
        || (limit.is_finite() && limit.trunc() > 9_007_199_254_740_991.0)
    {
        let error = vm.error(
            "RangeError",
            &format!("Iterator.prototype.{name} : limite invalide"),
        );
        return fail(vm, &iterator, error);
    }
    let next = vm.get(&iterator, "next")?;
    let mut helper = new_helper(kind, iterator, next);
    helper.remaining = limit.trunc();
    Ok(alloc_helper(vm, helper))
}

fn iterator_take(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    limited(vm, this, args, HelperKind::Take, "take")
}

fn iterator_drop(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    limited(vm, this, args, HelperKind::Drop, "drop")
}

/// `chunks` et `windows` : la taille est un entier de 1 à 2^32 - 1, sans
/// conversion ; `windows` accepte en plus `"only-full"` ou `"allow-partial"`.
fn sized(vm: &mut Vm, this: &Value, args: &[Value], kind: HelperKind, name: &str) -> JsResult {
    let iterator = receiver(vm, this, name)?;
    let size = match arg(args, 0) {
        Value::Number(size) if size.is_finite() && size.fract() == 0.0 => size,
        _ => {
            let error = vm.type_error(&format!(
                "Iterator.prototype.{name} : la taille doit être un nombre entier"
            ));
            return fail(vm, &iterator, error);
        }
    };
    if !(1.0..=4_294_967_295.0).contains(&size) {
        let error = vm.error(
            "RangeError",
            &format!("Iterator.prototype.{name} : taille hors de l'intervalle permis"),
        );
        return fail(vm, &iterator, error);
    }
    let partial = match arg(args, 1) {
        _ if kind != HelperKind::Windows => false,
        Value::Undefined => false,
        Value::String(mode) if &*mode == "only-full" => false,
        Value::String(mode) if &*mode == "allow-partial" => true,
        _ => {
            let error = vm.type_error(
                "Iterator.prototype.windows : « only-full » ou « allow-partial » attendu",
            );
            return fail(vm, &iterator, error);
        }
    };
    let next = vm.get(&iterator, "next")?;
    let mut helper = new_helper(kind, iterator, next);
    helper.remaining = size;
    helper.counter = f64::from(u8::from(partial));
    Ok(alloc_helper(vm, helper))
}

fn iterator_chunks(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    sized(vm, this, args, HelperKind::Chunks, "chunks")
}

fn iterator_windows(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    sized(vm, this, args, HelperKind::Windows, "windows")
}

// ---- Aides immédiates ----------------------------------------------------------

/// `includes(valeur, sautés)` : vrai si `valeur` paraît après les `sautés`
/// premiers éléments ; l'itérateur est alors fermé.
fn iterator_includes(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let iterator = receiver(vm, this, "includes")?;
    let to_skip = match arg(args, 1) {
        Value::Undefined => 0.0,
        Value::Number(count) if count.is_infinite() || count.fract() == 0.0 => count,
        _ => {
            let error = vm.type_error(
                "Iterator.prototype.includes : le nombre d'éléments à sauter doit être entier",
            );
            return fail(vm, &iterator, error);
        }
    };
    if to_skip < 0.0 || (to_skip.is_finite() && to_skip > 9_007_199_254_740_991.0) {
        let error = vm.error(
            "RangeError",
            "Iterator.prototype.includes : nombre d'éléments à sauter invalide",
        );
        return fail(vm, &iterator, error);
    }
    let target = arg(args, 0);
    let next = vm.get(&iterator, "next")?;
    let mut skipped = 0.0;
    loop {
        let Some(value) = step(vm, &iterator, &next)? else {
            return Ok(Value::Bool(false));
        };
        if skipped < to_skip {
            skipped += 1.0;
        } else if Vm::same_value_zero(&value, &target) {
            close(vm, &iterator)?;
            return Ok(Value::Bool(true));
        }
    }
}

/// `join(séparateur)` : les éléments convertis en chaînes et réunis.
fn iterator_join(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let iterator = receiver(vm, this, "join")?;
    let separator = match arg(args, 0) {
        Value::Undefined => Arc::from(","),
        other => match vm.to_string(&other) {
            Ok(separator) => separator,
            Err(error) => return fail(vm, &iterator, error),
        },
    };
    let next = vm.get(&iterator, "next")?;
    let mut out = String::new();
    let mut first = true;
    while let Some(value) = step(vm, &iterator, &next)? {
        if !std::mem::replace(&mut first, false) {
            out.push_str(&separator);
        }
        if !value.is_nullish() {
            match vm.to_string(&value) {
                Ok(text) => out.push_str(&text),
                Err(error) => return fail(vm, &iterator, error),
            }
        }
        if out.len() > MAX_JOINED {
            let error = vm.error("RangeError", "chaîne trop longue");
            return fail(vm, &iterator, error);
        }
    }
    Ok(Value::string(&out))
}

/// Ce que fait une aide immédiate du résultat de sa fonction de rappel.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Eager {
    ForEach,
    Some,
    Every,
    Find,
}

fn eager(vm: &mut Vm, this: &Value, args: &[Value], mode: Eager, name: &str) -> JsResult {
    let iterator = receiver(vm, this, name)?;
    let func = callback(vm, &iterator, &arg(args, 0), name)?;
    let next = vm.get(&iterator, "next")?;
    let mut counter = 0.0;
    loop {
        let Some(value) = step(vm, &iterator, &next)? else {
            return Ok(match mode {
                Eager::ForEach | Eager::Find => Value::Undefined,
                Eager::Some => Value::Bool(false),
                Eager::Every => Value::Bool(true),
            });
        };
        let result = match vm.call(
            &func,
            &Value::Undefined,
            &[value.clone(), Value::Number(counter)],
        ) {
            Ok(result) => result.truthy(),
            Err(error) => return fail(vm, &iterator, error),
        };
        counter += 1.0;
        let found = match mode {
            Eager::ForEach => None,
            Eager::Some => result.then_some(Value::Bool(true)),
            Eager::Every => (!result).then_some(Value::Bool(false)),
            Eager::Find => result.then_some(value),
        };
        if let Some(found) = found {
            close(vm, &iterator)?;
            return Ok(found);
        }
    }
}

fn iterator_for_each(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    eager(vm, this, args, Eager::ForEach, "forEach")
}

fn iterator_some(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    eager(vm, this, args, Eager::Some, "some")
}

fn iterator_every(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    eager(vm, this, args, Eager::Every, "every")
}

fn iterator_find(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    eager(vm, this, args, Eager::Find, "find")
}

fn iterator_reduce(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let iterator = receiver(vm, this, "reduce")?;
    let reducer = callback(vm, &iterator, &arg(args, 0), "reduce")?;
    let next = vm.get(&iterator, "next")?;
    let (mut accumulator, mut counter) = if args.len() >= 2 {
        (arg(args, 1), 0.0)
    } else {
        match step(vm, &iterator, &next)? {
            Some(first) => (first, 1.0),
            None => {
                return Err(vm.type_error(
                    "Iterator.prototype.reduce : itérateur vide sans valeur initiale",
                ));
            }
        }
    };
    while let Some(value) = step(vm, &iterator, &next)? {
        accumulator = match vm.call(
            &reducer,
            &Value::Undefined,
            &[accumulator, value, Value::Number(counter)],
        ) {
            Ok(result) => result,
            Err(error) => return fail(vm, &iterator, error),
        };
        counter += 1.0;
    }
    Ok(accumulator)
}

fn iterator_to_array(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let iterator = receiver(vm, this, "toArray")?;
    let next = vm.get(&iterator, "next")?;
    let mut items = Vec::new();
    while let Some(value) = step(vm, &iterator, &next)? {
        items.push(value);
    }
    Ok(vm.new_array(items))
}

// ---- Constructeur et fonctions statiques ----------------------------------------

/// `Iterator` est abstrait : il ne se construit que par une classe dérivée.
fn iterator_ctor(vm: &mut Vm, _this: &Value, _args: &[Value]) -> JsResult {
    let own = Value::Object(vm.intrinsics.iterator_ctor);
    if !vm.constructing || vm.native_new_target == own {
        return Err(
            vm.type_error("Iterator est abstrait : il ne peut pas être construit directement")
        );
    }
    Ok(Value::Object(vm.heap.alloc(Object::new(
        Some(vm.intrinsics.iterator_root),
        Kind::Ordinary,
    ))))
}

fn iterator_from(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let (iterator, next) = flattenable(vm, &arg(args, 0), true)?;
    // Déjà un `Iterator` : rendu tel quel.
    let ctor = Value::Object(vm.intrinsics.iterator_ctor);
    if vm.ordinary_has_instance(&iterator, &ctor)? {
        return Ok(iterator);
    }
    let helper = new_helper(HelperKind::Wrap, iterator, next);
    Ok(alloc_helper(vm, helper))
}

/// `%WrapForValidIteratorPrototype%.next`.
fn wrap_next(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let id = this_helper(vm, this, true)?;
    let Some(helper) = helper_mut(vm, id) else {
        return Ok(Value::Undefined);
    };
    let (iterator, next) = (helper.iterator.clone(), helper.next.clone());
    vm.call(&next, &iterator, &[])
}

/// `%WrapForValidIteratorPrototype%.return`.
fn wrap_return(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let id = this_helper(vm, this, true)?;
    let Some(helper) = helper_mut(vm, id) else {
        return Ok(Value::Undefined);
    };
    let iterator = helper.iterator.clone();
    let method = vm.get(&iterator, "return")?;
    if method.is_nullish() {
        return Ok(vm.iter_result(Value::Undefined, true));
    }
    vm.call(&method, &iterator, &[])
}

fn iterator_concat(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let key = PropertyKey::Symbol(vm.well_known.iterator.clone());
    let mut sources = Vec::with_capacity(args.len());
    for item in args {
        if item.as_object().is_none() {
            return Err(vm.type_error("Iterator.concat : chaque argument doit être un objet"));
        }
        let method = vm.get_key(item, &key)?;
        if !is_callable(vm, &method) {
            return Err(vm.type_error("Iterator.concat : chaque argument doit être itérable"));
        }
        sources.push((method, item.clone()));
    }
    let mut helper = new_helper(HelperKind::Concat, Value::Undefined, Value::Undefined);
    helper.sources = sources;
    Ok(alloc_helper(vm, helper))
}

/// Options de `Iterator.zip` : le mode, et l'option `padding` si le mode est
/// `"longest"` (`undefined` sinon).
fn zip_options(vm: &mut Vm, options: &Value) -> JsResult<(ZipMode, Value)> {
    let options = match options {
        Value::Undefined => None,
        Value::Object(_) => Some(options.clone()),
        _ => return Err(vm.type_error("Iterator.zip : les options doivent être un objet")),
    };
    let mode = match &options {
        Some(options) => vm.get(options, "mode")?,
        None => Value::Undefined,
    };
    let mode = match &mode {
        Value::Undefined => ZipMode::Shortest,
        Value::String(mode) if &**mode == "shortest" => ZipMode::Shortest,
        Value::String(mode) if &**mode == "longest" => ZipMode::Longest,
        Value::String(mode) if &**mode == "strict" => ZipMode::Strict,
        _ => {
            return Err(vm.type_error(
                "Iterator.zip : mode « shortest », « longest » ou « strict » attendu",
            ));
        }
    };
    let padding = match (&options, mode) {
        (Some(options), ZipMode::Longest) => {
            let padding = vm.get(options, "padding")?;
            if padding != Value::Undefined && padding.as_object().is_none() {
                return Err(vm.type_error("Iterator.zip : « padding » doit être un objet"));
            }
            padding
        }
        _ => Value::Undefined,
    };
    Ok((mode, padding))
}

/// Ferme les itérateurs déjà obtenus, puis rend `error`.
fn zip_abort<T>(vm: &mut Vm, iters: &[(Value, Value)], error: Value) -> JsResult<T> {
    let iterators: Vec<Value> = iters.iter().map(|(iterator, _)| iterator.clone()).collect();
    // Avec une exception en cours, `close_all` la rend toujours.
    match close_all(vm, &iterators, Some(error)) {
        Err(error) => Err(error),
        Ok(()) => Err(vm.type_error("Iterator.zip : itérateurs fermés")),
    }
}

/// Crée l'aide de `zip` ou de `zipKeyed`.
fn zip_helper(
    vm: &mut Vm,
    iters: Vec<(Value, Value)>,
    mode: ZipMode,
    padding: Vec<Value>,
    keys: Option<Vec<PropertyKey>>,
) -> Value {
    let mut helper = new_helper(HelperKind::Zip, Value::Undefined, Value::Undefined);
    helper.zip = Some(Box::new(ZipState {
        open: iters
            .iter()
            .map(|(iterator, _)| Some(iterator.clone()))
            .collect(),
        iters: iters.into_iter().map(Some).collect(),
        mode,
        padding,
        keys,
    }));
    alloc_helper(vm, helper)
}

/// `Iterator.zip(itérables, options)` : des tableaux réunissant, pas à pas,
/// la valeur de chaque itérable.
fn iterator_zip(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let iterables = arg(args, 0);
    if iterables.as_object().is_none() {
        return Err(vm.type_error("Iterator.zip : un objet itérable est attendu"));
    }
    let (mode, padding_option) = zip_options(vm, &arg(args, 1))?;
    let (input, input_next) = get_iterator(vm, &iterables)?;
    let mut iters: Vec<(Value, Value)> = Vec::new();
    loop {
        let item = match step(vm, &input, &input_next) {
            Ok(Some(item)) => item,
            Ok(None) => break,
            Err(error) => return zip_abort(vm, &iters, error),
        };
        match flattenable(vm, &item, false) {
            Ok(pair) => iters.push(pair),
            Err(error) => {
                // L'itérateur des itérables est fermé le dernier.
                let mut all = vec![(input, input_next)];
                all.extend(iters);
                return zip_abort(vm, &all, error);
            }
        }
    }
    let mut padding = Vec::new();
    if mode == ZipMode::Longest {
        if padding_option == Value::Undefined {
            padding = vec![Value::Undefined; iters.len()];
        } else {
            let (source, source_next) = match get_iterator(vm, &padding_option) {
                Ok(pair) => pair,
                Err(error) => return zip_abort(vm, &iters, error),
            };
            let mut using = true;
            for _ in 0..iters.len() {
                if using {
                    match step(vm, &source, &source_next) {
                        Ok(Some(value)) => padding.push(value),
                        Ok(None) => using = false,
                        Err(error) => return zip_abort(vm, &iters, error),
                    }
                }
                if !using {
                    padding.push(Value::Undefined);
                }
            }
            if using && let Err(error) = close(vm, &source) {
                return zip_abort(vm, &iters, error);
            }
        }
    }
    Ok(zip_helper(vm, iters, mode, padding, None))
}

/// `Iterator.zipKeyed(itérables, options)` : comme `zip`, pour un objet
/// dont chaque propriété énumérable est un itérable ; les résultats sont
/// des objets aux mêmes clés.
fn iterator_zip_keyed(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let iterables = arg(args, 0);
    let Some(id) = iterables.as_object() else {
        return Err(vm.type_error("Iterator.zipKeyed : un objet est attendu"));
    };
    let (mode, padding_option) = zip_options(vm, &arg(args, 1))?;
    let all_keys = vm.own_property_keys(id)?;
    let mut keys = Vec::new();
    let mut iters: Vec<(Value, Value)> = Vec::new();
    for key in all_keys {
        let descriptor = match vm.get_own_property(id, &key) {
            Ok(descriptor) => descriptor,
            Err(error) => return zip_abort(vm, &iters, error),
        };
        if !descriptor.is_some_and(|d| d.enumerable == Some(true)) {
            continue;
        }
        let value = match vm.get_key(&iterables, &key) {
            Ok(value) => value,
            Err(error) => return zip_abort(vm, &iters, error),
        };
        if value == Value::Undefined {
            continue;
        }
        keys.push(key);
        match flattenable(vm, &value, false) {
            Ok(pair) => iters.push(pair),
            Err(error) => return zip_abort(vm, &iters, error),
        }
    }
    let mut padding = Vec::new();
    if mode == ZipMode::Longest {
        for key in &keys {
            if padding_option == Value::Undefined {
                padding.push(Value::Undefined);
                continue;
            }
            match vm.get_key(&padding_option, key) {
                Ok(value) => padding.push(value),
                Err(error) => return zip_abort(vm, &iters, error),
            }
        }
    }
    Ok(zip_helper(vm, iters, mode, padding, Some(keys)))
}

// ---- Accesseurs de `Iterator.prototype` -------------------------------------------

fn tag_getter(_vm: &mut Vm, _this: &Value, _args: &[Value]) -> JsResult {
    Ok(Value::string("Iterator"))
}

fn constructor_getter(vm: &mut Vm, _this: &Value, _args: &[Value]) -> JsResult {
    Ok(Value::Object(vm.intrinsics.iterator_ctor))
}

/// `SetterThatIgnoresPrototypeProperties` (ECMA-262 §27.1.4.1) : écrire sur
/// `Iterator.prototype` lui-même est refusé ; sur un objet qui en hérite, la
/// propriété est créée sur cet objet.
fn weak_setter(vm: &mut Vm, this: &Value, key: &PropertyKey, value: Value) -> JsResult {
    let Some(id) = this.as_object() else {
        return Err(vm.type_error("le receveur doit être un objet"));
    };
    if id == vm.intrinsics.iterator_root {
        return Err(vm.type_error("cette propriété de Iterator.prototype ne peut pas être écrite"));
    }
    if vm.get_own_property(id, key)?.is_none() {
        vm.create_data_property(id, key, value)?;
    } else if !vm.put_key(this, key, value)? {
        return Err(vm.type_error("la propriété ne peut pas être écrite"));
    }
    Ok(Value::Undefined)
}

fn tag_setter(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let key = PropertyKey::Symbol(vm.well_known.to_string_tag.clone());
    weak_setter(vm, this, &key, arg(args, 0))
}

fn constructor_setter(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let key = PropertyKey::String(Arc::from("constructor"));
    weak_setter(vm, this, &key, arg(args, 0))
}

fn accessor(vm: &mut Vm, name: &str, get: NativeFn, set: NativeFn) -> Descriptor {
    let getter = vm.new_function(&format!("get {name}"), 0, get);
    let setter = vm.new_function(&format!("set {name}"), 1, set);
    Descriptor {
        get: Some(Value::Object(getter)),
        set: Some(Value::Object(setter)),
        enumerable: Some(false),
        configurable: Some(true),
        ..Descriptor::default()
    }
}

pub(super) fn install(vm: &mut Vm) {
    let root = vm.intrinsics.iterator_root;
    let ctor = vm.new_native("Iterator", 0, iterator_ctor, true);
    vm.intrinsics.iterator_ctor = ctor;
    vm.heap.get_mut(ctor).props.insert(
        Arc::from("prototype"),
        Property {
            slot: Slot::Data(Value::Object(root)),
            writable: false,
            enumerable: false,
            configurable: false,
        },
    );
    vm.define_hidden(vm.global, "Iterator", Value::Object(ctor));
    vm.define_method(ctor, "from", 1, iterator_from);
    vm.define_method(ctor, "concat", 0, iterator_concat);
    vm.define_method(ctor, "zip", 1, iterator_zip);
    vm.define_method(ctor, "zipKeyed", 1, iterator_zip_keyed);

    let methods: &[(&str, u32, NativeFn)] = &[
        ("map", 1, iterator_map),
        ("filter", 1, iterator_filter),
        ("take", 1, iterator_take),
        ("drop", 1, iterator_drop),
        ("flatMap", 1, iterator_flat_map),
        ("reduce", 1, iterator_reduce),
        ("toArray", 0, iterator_to_array),
        ("forEach", 1, iterator_for_each),
        ("some", 1, iterator_some),
        ("every", 1, iterator_every),
        ("find", 1, iterator_find),
        ("includes", 1, iterator_includes),
        ("join", 1, iterator_join),
        ("chunks", 1, iterator_chunks),
        ("windows", 1, iterator_windows),
    ];
    for (name, arity, func) in methods {
        vm.define_method(root, name, *arity, *func);
    }
    let tag = vm.well_known.to_string_tag.clone();
    let descriptor = accessor(vm, "[Symbol.toStringTag]", tag_getter, tag_setter);
    let _ = vm.define_own_property(root, &PropertyKey::Symbol(tag.clone()), descriptor);
    let descriptor = accessor(vm, "constructor", constructor_getter, constructor_setter);
    let _ = vm.define_own_property(
        root,
        &PropertyKey::String(Arc::from("constructor")),
        descriptor,
    );

    let frozen_tag = |text: &str| Property {
        slot: Slot::Data(Value::string(text)),
        writable: false,
        enumerable: false,
        configurable: true,
    };
    let helper_proto = vm.intrinsics.iterator_helper_proto;
    vm.heap.get_mut(helper_proto).proto = Some(root);
    vm.define_method(helper_proto, "next", 0, helper_next);
    vm.define_method(helper_proto, "return", 0, helper_return);
    vm.heap
        .get_mut(helper_proto)
        .symbols
        .insert(tag, frozen_tag("Iterator Helper"));

    let wrap_proto = vm.intrinsics.wrap_iterator_proto;
    vm.heap.get_mut(wrap_proto).proto = Some(root);
    vm.define_method(wrap_proto, "next", 0, wrap_next);
    vm.define_method(wrap_proto, "return", 0, wrap_return);
}
