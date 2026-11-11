//! `Map`, `Set`, `WeakMap` et `WeakSet`.
//!
//! LIMITES CONNUES : `WeakMap` et `WeakSet` retiennent leurs clés (ce sont des
//! `Map` et `Set` restreints aux clés objets) ; les itérateurs de `keys`,
//! `values` et `entries` parcourent une copie prise à leur création.

use indexmap::IndexMap;

use crate::heap::{IterState, Kind, MapKey, Object};
use crate::value::{ObjId, Value};
use crate::vm::{JsResult, NativeFn, Vm};

use super::{arg, callable, constructor, getter};

fn this_map(vm: &mut Vm, this: &Value) -> JsResult<ObjId> {
    match this.as_object() {
        Some(id) if matches!(vm.heap.get(id).kind, Kind::Map(_)) => Ok(id),
        _ => Err(vm.type_error("cette méthode attend une Map comme receveur")),
    }
}

fn this_set(vm: &mut Vm, this: &Value) -> JsResult<ObjId> {
    match this.as_object() {
        Some(id) if matches!(vm.heap.get(id).kind, Kind::Set(_)) => Ok(id),
        _ => Err(vm.type_error("cette méthode attend un Set comme receveur")),
    }
}

fn map_mut(vm: &mut Vm, id: ObjId) -> Option<&mut IndexMap<MapKey, (Value, Value)>> {
    match &mut vm.heap.get_mut(id).kind {
        Kind::Map(map) => Some(map),
        _ => None,
    }
}

fn set_mut(vm: &mut Vm, id: ObjId) -> Option<&mut IndexMap<MapKey, Value>> {
    match &mut vm.heap.get_mut(id).kind {
        Kind::Set(set) => Some(set),
        _ => None,
    }
}

/// Prototype à donner à une instance : celui du constructeur global `name`.
fn proto_of(vm: &mut Vm, name: &str, fallback: ObjId) -> JsResult<ObjId> {
    let ctor = vm.get(&Value::Object(vm.global), name)?;
    Ok(match ctor {
        Value::Object(_) => vm.get(&ctor, "prototype")?.as_object().unwrap_or(fallback),
        _ => fallback,
    })
}

fn build_map(vm: &mut Vm, proto: ObjId, source: &Value) -> JsResult {
    let mut map = IndexMap::new();
    if !source.is_nullish() {
        for entry in vm.iterate_to_vec(source)? {
            if entry.as_object().is_none() {
                return Err(vm.type_error("Map : chaque entrée doit être une paire [clé, valeur]"));
            }
            let (key, value) = (vm.get(&entry, "0")?, vm.get(&entry, "1")?);
            map.insert(MapKey::from_value(&key), (key, value));
        }
    }
    Ok(Value::Object(
        vm.heap.alloc(Object::new(Some(proto), Kind::Map(map))),
    ))
}

fn build_set(vm: &mut Vm, proto: ObjId, source: &Value) -> JsResult {
    let mut set = IndexMap::new();
    if !source.is_nullish() {
        for value in vm.iterate_to_vec(source)? {
            set.insert(MapKey::from_value(&value), value);
        }
    }
    Ok(Value::Object(
        vm.heap.alloc(Object::new(Some(proto), Kind::Set(set))),
    ))
}

fn map_ctor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let proto = vm.intrinsics.map_proto;
    build_map(vm, proto, &arg(args, 0))
}

fn set_ctor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let proto = vm.intrinsics.set_proto;
    build_set(vm, proto, &arg(args, 0))
}

fn weak_map_ctor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let fallback = vm.intrinsics.map_proto;
    let proto = proto_of(vm, "WeakMap", fallback)?;
    build_map(vm, proto, &arg(args, 0))
}

fn weak_set_ctor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let fallback = vm.intrinsics.set_proto;
    let proto = proto_of(vm, "WeakSet", fallback)?;
    build_set(vm, proto, &arg(args, 0))
}

fn map_get(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_map(vm, this)?;
    let key = MapKey::from_value(&arg(args, 0));
    Ok(map_mut(vm, id)
        .and_then(|m| m.get(&key))
        .map_or(Value::Undefined, |(_, v)| v.clone()))
}

fn map_set(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_map(vm, this)?;
    let (key, value) = (arg(args, 0), arg(args, 1));
    // `-0` est normalisé en `+0`, comme le veut le standard.
    let key = if key == Value::Number(0.0) {
        Value::Number(0.0)
    } else {
        key
    };
    if let Some(map) = map_mut(vm, id) {
        map.insert(MapKey::from_value(&key), (key, value));
    }
    Ok(this.clone())
}

fn map_has(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_map(vm, this)?;
    let key = MapKey::from_value(&arg(args, 0));
    Ok(Value::Bool(
        map_mut(vm, id).is_some_and(|m| m.contains_key(&key)),
    ))
}

fn map_delete(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_map(vm, this)?;
    let key = MapKey::from_value(&arg(args, 0));
    Ok(Value::Bool(
        map_mut(vm, id).is_some_and(|m| m.shift_remove(&key).is_some()),
    ))
}

fn map_clear(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let id = this_map(vm, this)?;
    if let Some(map) = map_mut(vm, id) {
        map.clear();
    }
    Ok(Value::Undefined)
}

#[expect(
    clippy::cast_precision_loss,
    reason = "taille de collection petite devant 2^53"
)]
fn map_size(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let id = this_map(vm, this)?;
    Ok(Value::Number(map_mut(vm, id).map_or(0, |m| m.len()) as f64))
}

fn map_pairs(vm: &mut Vm, this: &Value) -> JsResult<Vec<(Value, Value)>> {
    let id = this_map(vm, this)?;
    Ok(map_mut(vm, id)
        .map(|m| m.values().cloned().collect())
        .unwrap_or_default())
}

fn map_for_each(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let callback = callable(vm, &arg(args, 0))?;
    for (key, value) in map_pairs(vm, this)? {
        vm.call(&callback, &arg(args, 1), &[value, key, this.clone()])?;
    }
    Ok(Value::Undefined)
}

fn map_keys(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let values = map_pairs(vm, this)?.into_iter().map(|(k, _)| k).collect();
    Ok(vm.new_iterator(IterState::Values { values, index: 0 }))
}

fn map_values(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let values = map_pairs(vm, this)?.into_iter().map(|(_, v)| v).collect();
    Ok(vm.new_iterator(IterState::Values { values, index: 0 }))
}

fn map_entries(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let pairs = map_pairs(vm, this)?;
    let entries = pairs
        .into_iter()
        .map(|(k, v)| vm.new_array(vec![k, v]))
        .collect();
    Ok(vm.new_iterator(IterState::Values {
        values: entries,
        index: 0,
    }))
}

fn set_add(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_set(vm, this)?;
    let value = arg(args, 0);
    let value = if value == Value::Number(0.0) {
        Value::Number(0.0)
    } else {
        value
    };
    if let Some(set) = set_mut(vm, id) {
        set.entry(MapKey::from_value(&value)).or_insert(value);
    }
    Ok(this.clone())
}

fn set_has(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_set(vm, this)?;
    let key = MapKey::from_value(&arg(args, 0));
    Ok(Value::Bool(
        set_mut(vm, id).is_some_and(|s| s.contains_key(&key)),
    ))
}

fn set_delete(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_set(vm, this)?;
    let key = MapKey::from_value(&arg(args, 0));
    Ok(Value::Bool(
        set_mut(vm, id).is_some_and(|s| s.shift_remove(&key).is_some()),
    ))
}

fn set_clear(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let id = this_set(vm, this)?;
    if let Some(set) = set_mut(vm, id) {
        set.clear();
    }
    Ok(Value::Undefined)
}

#[expect(
    clippy::cast_precision_loss,
    reason = "taille de collection petite devant 2^53"
)]
fn set_size(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let id = this_set(vm, this)?;
    Ok(Value::Number(set_mut(vm, id).map_or(0, |s| s.len()) as f64))
}

fn set_items(vm: &mut Vm, this: &Value) -> JsResult<Vec<Value>> {
    let id = this_set(vm, this)?;
    Ok(set_mut(vm, id)
        .map(|s| s.values().cloned().collect())
        .unwrap_or_default())
}

fn set_for_each(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let callback = callable(vm, &arg(args, 0))?;
    for value in set_items(vm, this)? {
        vm.call(
            &callback,
            &arg(args, 1),
            &[value.clone(), value, this.clone()],
        )?;
    }
    Ok(Value::Undefined)
}

fn set_values(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let values = set_items(vm, this)?;
    Ok(vm.new_iterator(IterState::Values { values, index: 0 }))
}

fn set_entries(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let values = set_items(vm, this)?;
    let entries = values
        .into_iter()
        .map(|v| vm.new_array(vec![v.clone(), v]))
        .collect();
    Ok(vm.new_iterator(IterState::Values {
        values: entries,
        index: 0,
    }))
}

pub(super) fn install(vm: &mut Vm) {
    let map_methods: &[(&str, u32, NativeFn)] = &[
        ("get", 1, map_get),
        ("set", 2, map_set),
        ("has", 1, map_has),
        ("delete", 1, map_delete),
        ("clear", 0, map_clear),
        ("forEach", 1, map_for_each),
        ("keys", 0, map_keys),
        ("values", 0, map_values),
        ("entries", 0, map_entries),
    ];
    let set_methods: &[(&str, u32, NativeFn)] = &[
        ("add", 1, set_add),
        ("has", 1, set_has),
        ("delete", 1, set_delete),
        ("clear", 0, set_clear),
        ("forEach", 1, set_for_each),
        ("keys", 0, set_values),
        ("values", 0, set_values),
        ("entries", 0, set_entries),
    ];
    let map_proto = vm.intrinsics.map_proto;
    constructor(vm, "Map", 0, map_ctor, map_proto);
    for (name, arity, func) in map_methods {
        vm.define_method(map_proto, name, *arity, *func);
    }
    getter(vm, map_proto, "size", map_size);

    let set_proto = vm.intrinsics.set_proto;
    constructor(vm, "Set", 0, set_ctor, set_proto);
    for (name, arity, func) in set_methods {
        vm.define_method(set_proto, name, *arity, *func);
    }
    getter(vm, set_proto, "size", set_size);

    // Variantes « faibles » : mêmes opérations de base, sans énumération.
    let weak_map_proto = vm.new_object();
    constructor(vm, "WeakMap", 0, weak_map_ctor, weak_map_proto);
    for (name, arity, func) in map_methods.iter().take(4) {
        vm.define_method(weak_map_proto, name, *arity, *func);
    }
    let weak_set_proto = vm.new_object();
    constructor(vm, "WeakSet", 0, weak_set_ctor, weak_set_proto);
    for (name, arity, func) in set_methods.iter().take(3) {
        vm.define_method(weak_set_proto, name, *arity, *func);
    }
}
