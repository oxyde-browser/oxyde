//! `Iterator` et ses aides (ECMA-262 §27.1) : `map`, `filter`, `take`, `drop`,
//! `flatMap`, `reduce`, `toArray`, `forEach`, `some`, `every`, `find`,
//! `Iterator.from` et `Iterator.concat`.
//!
//! Une aide paresseuse rend un objet `Kind::IteratorHelper` qui garde
//! l'itérateur sous-jacent et avance d'un pas à chaque `next()`.
//!
//! LIMITES CONNUES : pas de `Iterator.zip`, `zipKeyed`, `chunks`, `windows`,
//! `includes` ni `join`.

use std::sync::Arc;

use super::{arg, is_callable};
use crate::heap::{HelperKind, IteratorHelper, Kind, Object, Property, Slot};
use crate::internal::Descriptor;
use crate::value::{ObjId, PropertyKey, Value};
use crate::vm::{JsResult, NativeFn, Vm};

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
        HelperKind::Wrap | HelperKind::RegExpString => Ok(None),
    }
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
        // L'itérateur interne (`flatMap`, `concat`) est fermé le premier.
        if let Some((inner_iterator, _)) = inner
            && let Err(error) = close(vm, &inner_iterator)
        {
            return fail(vm, &iterator, error);
        }
        close(vm, &iterator)?;
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
    if limit.is_nan() || limit.trunc() < 0.0 {
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

// ---- Aides immédiates ----------------------------------------------------------

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
