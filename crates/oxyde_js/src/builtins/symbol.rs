//! `Symbol` et protocole d'itération (`Symbol.iterator`, `%IteratorPrototype%`).
//!
//! LIMITES CONNUES : `Symbol.toPrimitive` et `Symbol.hasInstance` existent
//! mais ne sont pas consultés par les conversions ni par `instanceof` ; les
//! tableaux, `Map` et `Set` sont parcourus par `for…of` sans consulter leur
//! `Symbol.iterator` (le redéfinir est sans effet) ; un itérateur abandonné en
//! cours de boucle ne reçoit pas d'appel à `return()`.

use std::sync::Arc;

use super::{arg, constructor, getter};
use crate::heap::{IterState, Kind, Property, Slot};
use crate::value::{Symbol, Value};
use crate::vm::{JsResult, NativeFn, Vm};

fn symbol_ctor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    if vm.constructing {
        return Err(vm.type_error("Symbol n'est pas un constructeur"));
    }
    let description = match arg(args, 0) {
        Value::Undefined => None,
        other => Some(vm.to_string(&other)?),
    };
    Ok(Value::Symbol(vm.new_symbol(description.as_deref())))
}

fn symbol_for(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let key = vm.to_string(&arg(args, 0))?;
    if let Some(symbol) = vm.symbol_registry.get(&key) {
        return Ok(Value::Symbol(symbol.clone()));
    }
    let symbol = vm.new_symbol(Some(&key));
    vm.symbol_registry.insert(key, symbol.clone());
    Ok(Value::Symbol(symbol))
}

fn symbol_key_for(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let Value::Symbol(symbol) = arg(args, 0) else {
        return Err(vm.type_error("Symbol.keyFor : un symbole est attendu"));
    };
    Ok(vm
        .symbol_registry
        .iter()
        .find(|(_, s)| **s == symbol)
        .map_or(Value::Undefined, |(key, _)| Value::String(Arc::clone(key))))
}

/// Symbole désigné par `this` : primitive ou objet enveloppe.
fn this_symbol(vm: &mut Vm, this: &Value) -> JsResult<Symbol> {
    match this {
        Value::Symbol(symbol) => Ok(symbol.clone()),
        Value::Object(id) => match &vm.heap.get(*id).kind {
            Kind::Primitive(Value::Symbol(symbol)) => Ok(symbol.clone()),
            _ => Err(vm.type_error("un symbole est attendu")),
        },
        _ => Err(vm.type_error("un symbole est attendu")),
    }
}

fn symbol_to_string(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let symbol = this_symbol(vm, this)?;
    Ok(Value::string(&format!(
        "Symbol({})",
        symbol.description().unwrap_or("")
    )))
}

fn symbol_value_of(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    this_symbol(vm, this).map(Value::Symbol)
}

fn symbol_description(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let symbol = this_symbol(vm, this)?;
    Ok(symbol.description().map_or(Value::Undefined, Value::string))
}

// ---- Itérateurs ------------------------------------------------------------------

/// `%IteratorPrototype%.next` des itérateurs intégrés.
fn iterator_next(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let is_iterator = this
        .as_object()
        .is_some_and(|id| matches!(vm.heap.get(id).kind, Kind::Iterator(_)));
    if !is_iterator {
        return Err(vm.type_error("next : le receveur n'est pas un itérateur"));
    }
    let next = vm.iterator_next(this)?;
    let done = next.is_none();
    Ok(vm.iter_result(next.unwrap_or(Value::Undefined), done))
}

/// `%IteratorPrototype%[Symbol.iterator]` : un itérateur est son propre itérable.
fn iterator_self(_vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    Ok(this.clone())
}

/// `String.prototype[Symbol.iterator]` : parcours par points de code.
fn string_iterator(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let text = vm.to_string(this)?;
    let values = text
        .chars()
        .map(|c| Value::string(c.encode_utf8(&mut [0; 4])))
        .collect();
    Ok(vm.new_iterator(IterState::Values { values, index: 0 }))
}

pub(super) fn install(vm: &mut Vm) {
    let proto = vm.intrinsics.symbol_proto;
    let ctor = constructor(vm, "Symbol", 0, symbol_ctor, proto);
    vm.define_method(ctor, "for", 1, symbol_for);
    vm.define_method(ctor, "keyFor", 1, symbol_key_for);
    let known = vm.well_known.clone();
    for (name, symbol) in [
        ("iterator", &known.iterator),
        ("asyncIterator", &known.async_iterator),
        ("hasInstance", &known.has_instance),
        ("toPrimitive", &known.to_primitive),
        ("toStringTag", &known.to_string_tag),
        ("isConcatSpreadable", &known.is_concat_spreadable),
        ("species", &known.species),
        ("unscopables", &known.unscopables),
        ("match", &known.match_),
        ("matchAll", &known.match_all),
        ("replace", &known.replace),
        ("search", &known.search),
        ("split", &known.split),
        ("dispose", &known.dispose),
        ("asyncDispose", &known.async_dispose),
    ] {
        // Les symboles connus sont des propriétés figées de `Symbol`.
        vm.heap.get_mut(ctor).props.insert(
            Arc::from(name),
            Property {
                slot: Slot::Data(Value::Symbol(symbol.clone())),
                writable: false,
                enumerable: false,
                configurable: false,
            },
        );
    }
    let methods: &[(&str, u32, NativeFn)] = &[
        ("toString", 0, symbol_to_string),
        ("valueOf", 0, symbol_value_of),
    ];
    for (name, arity, func) in methods {
        vm.define_method(proto, name, *arity, *func);
    }
    getter(vm, proto, "description", symbol_description);
    vm.define_hidden_symbol(proto, &known.to_string_tag, Value::string("Symbol"));

    // Protocole d'itération.
    let iterator_proto = vm.intrinsics.iterator_proto;
    vm.define_method(iterator_proto, "next", 0, iterator_next);
    // `[Symbol.iterator]() { return this; }` vit sur `Iterator.prototype`,
    // dont héritent tous les itérateurs.
    let root = vm.intrinsics.iterator_root;
    vm.heap.get_mut(iterator_proto).proto = Some(root);
    let this_fn = vm.new_function("[Symbol.iterator]", 0, iterator_self);
    vm.define_hidden_symbol(root, &known.iterator, Value::Object(this_fn));

    let intr = vm.intrinsics;
    for (proto, method) in [
        (intr.array_proto, "values"),
        (intr.map_proto, "entries"),
        (intr.set_proto, "values"),
    ] {
        if let Ok(function) = vm.get(&Value::Object(proto), method) {
            vm.define_hidden_symbol(proto, &known.iterator, function);
        }
    }
    let chars = vm.new_function("[Symbol.iterator]", 0, string_iterator);
    vm.define_hidden_symbol(intr.string_proto, &known.iterator, Value::Object(chars));
}
