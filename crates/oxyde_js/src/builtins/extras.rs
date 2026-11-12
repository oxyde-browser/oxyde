//! Compléments de la bibliothèque standard : `Symbol.species`, regroupement
//! (`Object.groupBy`, `Map.groupBy`), `Error.isError`, erreurs composées
//! (`AggregateError`, `SuppressedError`), `Function.prototype[Symbol.hasInstance]`
//! et `Symbol.prototype[Symbol.toPrimitive]`.

use std::sync::Arc;

use indexmap::IndexMap;

use super::{arg, callable, constructor};
use crate::heap::{Kind, MapKey, Object, Property, Slot};
use crate::internal::Descriptor;
use crate::value::{ObjId, PropertyKey, Value};
use crate::vm::{JsResult, NativeFn, Vm};

// ---- Symbol.species ------------------------------------------------------------

fn species_getter(_vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    Ok(this.clone())
}

/// Propriété symbole non énumérable, non inscriptible, configurable.
fn frozen_symbol(vm: &mut Vm, target: ObjId, symbol: &crate::value::Symbol, value: Value) {
    vm.heap.get_mut(target).symbols.insert(
        symbol.clone(),
        Property {
            slot: Slot::Data(value),
            writable: false,
            enumerable: false,
            configurable: true,
        },
    );
}

// ---- Regroupement --------------------------------------------------------------

/// `GroupBy` (ECMA-262 §7.3.35) : parcourt `items` et range chaque valeur
/// sous la clé que rend `callback(valeur, index)`. `property` : les clés
/// sont des clés de propriété ; sinon des valeurs quelconques (`-0` devient `+0`).
fn group_by(
    vm: &mut Vm,
    items: &Value,
    callback: &Value,
    property: bool,
) -> JsResult<Vec<(Value, Vec<Value>)>> {
    if items.is_nullish() {
        return Err(vm.type_error("groupBy : la source est null ou undefined"));
    }
    let callback = callable(vm, callback)?;
    let iterator = vm.iterator_for(items)?;
    let mut groups: IndexMap<MapKey, (Value, Vec<Value>)> = IndexMap::new();
    let mut index = 0.0;
    while let Some(value) = vm.iterator_next(&iterator)? {
        vm.tick(1)?;
        let key = vm
            .call(
                &callback,
                &Value::Undefined,
                &[value.clone(), Value::Number(index)],
            )
            .and_then(|key| {
                if property {
                    vm.to_key(&key).map(|key| crate::internal::key_value(&key))
                } else if key == Value::Number(0.0) {
                    Ok(Value::Number(0.0))
                } else {
                    Ok(key)
                }
            });
        let key = match key {
            Ok(key) => key,
            Err(error) => {
                vm.iterator_close(&iterator, true)?;
                return Err(error);
            }
        };
        groups
            .entry(MapKey::from_value(&key))
            .or_insert_with(|| (key, Vec::new()))
            .1
            .push(value);
        index += 1.0;
    }
    Ok(groups.into_values().collect())
}

fn object_group_by(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let groups = group_by(vm, &arg(args, 0), &arg(args, 1), true)?;
    let result = vm.heap.alloc(Object::new(None, Kind::Ordinary));
    for (key, values) in groups {
        let key = vm.to_key(&key)?;
        let values = vm.new_array(values);
        vm.create_data_property(result, &key, values)?;
    }
    Ok(Value::Object(result))
}

fn map_group_by(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let groups = group_by(vm, &arg(args, 0), &arg(args, 1), false)?;
    let mut map = IndexMap::new();
    for (key, values) in groups {
        let values = vm.new_array(values);
        map.insert(MapKey::from_value(&key), (key, values));
    }
    Ok(Value::Object(vm.heap.alloc(Object::new(
        Some(vm.intrinsics.map_proto),
        Kind::Map(map),
    ))))
}

// ---- Erreurs -------------------------------------------------------------------

fn error_is_error(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let is_error = arg(args, 0)
        .as_object()
        .is_some_and(|id| matches!(vm.heap.get(id).kind, Kind::Error));
    Ok(Value::Bool(is_error))
}

/// Complète une erreur : message facultatif, cause facultative (`options.cause`).
fn finish_error(vm: &mut Vm, class: &str, message: &Value, options: &Value) -> JsResult<ObjId> {
    let text = match message {
        Value::Undefined => None,
        other => Some(vm.to_string(other)?),
    };
    let error = vm.error(class, text.as_deref().unwrap_or(""));
    let Some(id) = error.as_object() else {
        return Err(vm.type_error("erreur interne : objet d'erreur attendu"));
    };
    if text.is_none() {
        vm.heap.get_mut(id).props.shift_remove("message");
    }
    if let Some(options_id) = options.as_object() {
        let key = PropertyKey::String(Arc::from("cause"));
        if vm.has_property_op(options_id, &key)? {
            let cause = vm.get(options, "cause")?;
            vm.define_hidden(id, "cause", cause);
        }
    }
    Ok(id)
}

/// `AggregateError(errors, message, options)`.
fn aggregate_error_ctor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let id = finish_error(vm, "AggregateError", &arg(args, 1), &arg(args, 2))?;
    let errors = vm.iterate_to_vec(&arg(args, 0))?;
    let errors = vm.new_array(errors);
    vm.define_hidden(id, "errors", errors);
    Ok(Value::Object(id))
}

/// `SuppressedError(error, suppressed, message)`.
fn suppressed_error_ctor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let id = finish_error(vm, "SuppressedError", &arg(args, 2), &Value::Undefined)?;
    vm.define_hidden(id, "error", arg(args, 0));
    vm.define_hidden(id, "suppressed", arg(args, 1));
    Ok(Value::Object(id))
}

// ---- Function et Symbol ----------------------------------------------------------

fn function_has_instance(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    vm.ordinary_has_instance(&arg(args, 0), this)
        .map(Value::Bool)
}

fn symbol_to_primitive(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    super::this_primitive(
        vm,
        this,
        |v| matches!(v, Value::Symbol(_)),
        "Symbol.prototype[Symbol.toPrimitive]",
    )
}

pub(super) fn install(vm: &mut Vm) {
    let known = vm.well_known.clone();
    let global = Value::Object(vm.global);

    // `Symbol.species` : accesseur qui rend le constructeur lui-même.
    for name in [
        "Array",
        "Map",
        "Set",
        "Promise",
        "ArrayBuffer",
        "SharedArrayBuffer",
    ] {
        let Ok(Value::Object(ctor)) = vm.get(&global, name) else {
            continue;
        };
        let getter = vm.new_function("get [Symbol.species]", 0, species_getter);
        let descriptor = Descriptor {
            get: Some(Value::Object(getter)),
            set: Some(Value::Undefined),
            enumerable: Some(false),
            configurable: Some(true),
            ..Descriptor::default()
        };
        let _ = vm.define_own_property(
            ctor,
            &PropertyKey::Symbol(known.species.clone()),
            descriptor,
        );
    }

    if let Ok(Value::Object(object)) = vm.get(&global, "Object") {
        vm.define_method(object, "groupBy", 2, object_group_by);
    }
    if let Ok(Value::Object(map)) = vm.get(&global, "Map") {
        vm.define_method(map, "groupBy", 2, map_group_by);
    }
    let error_ctor = vm.get(&global, "Error").ok().and_then(|v| v.as_object());
    if let Some(error) = error_ctor {
        vm.define_method(error, "isError", 1, error_is_error);
    }

    // Erreurs composées : elles héritent de `Error`.
    let classes: &[(&'static str, u32, NativeFn)] = &[
        ("AggregateError", 2, aggregate_error_ctor),
        ("SuppressedError", 3, suppressed_error_ctor),
    ];
    for (name, arity, func) in classes {
        let proto = vm
            .heap
            .alloc(Object::new(Some(vm.intrinsics.error_proto), Kind::Ordinary));
        vm.error_protos.insert(name, proto);
        let ctor = constructor(vm, name, *arity, *func, proto);
        vm.heap.get_mut(ctor).proto = error_ctor;
        vm.define_hidden(proto, "name", Value::string(name));
        vm.define_hidden(proto, "message", Value::string(""));
    }

    // `Function.prototype[Symbol.hasInstance]` : figé.
    let function_proto = vm.intrinsics.function_proto;
    let has_instance = vm.new_function("[Symbol.hasInstance]", 1, function_has_instance);
    vm.has_instance_function = Some(has_instance);
    vm.heap.get_mut(function_proto).symbols.insert(
        known.has_instance.clone(),
        Property {
            slot: Slot::Data(Value::Object(has_instance)),
            writable: false,
            enumerable: false,
            configurable: false,
        },
    );

    let symbol_proto = vm.intrinsics.symbol_proto;
    let to_primitive = vm.new_function("[Symbol.toPrimitive]", 1, symbol_to_primitive);
    frozen_symbol(
        vm,
        symbol_proto,
        &known.to_primitive,
        Value::Object(to_primitive),
    );

    if let Ok(Value::Object(math)) = vm.get(&global, "Math") {
        frozen_symbol(vm, math, &known.to_string_tag, Value::string("Math"));
    }
}
