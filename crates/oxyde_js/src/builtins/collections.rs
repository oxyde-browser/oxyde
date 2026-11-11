//! `Map` et `Set` (les variantes faibles sont dans `weak`).
//!
//! LIMITE CONNUE : les itérateurs de `keys`, `values` et `entries`
//! parcourent une copie prise à leur création.

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

/// Clé normalisée d'une `Map` (`-0` devient `+0`).
fn map_key(key: Value) -> Value {
    if key == Value::Number(0.0) {
        Value::Number(0.0)
    } else {
        key
    }
}

/// `getOrInsert(key, value)` : la valeur présente, sinon `value`, qui est insérée.
fn map_get_or_insert(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_map(vm, this)?;
    let (key, value) = (map_key(arg(args, 0)), arg(args, 1));
    let Some(map) = map_mut(vm, id) else {
        return Ok(Value::Undefined);
    };
    let entry = map.entry(MapKey::from_value(&key)).or_insert((key, value));
    Ok(entry.1.clone())
}

/// `getOrInsertComputed(key, callback)` : la valeur présente, sinon celle
/// que rend `callback(key)`, qui est insérée.
fn map_get_or_insert_computed(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_map(vm, this)?;
    let callback = callable(vm, &arg(args, 1))?;
    let key = map_key(arg(args, 0));
    let map_key = MapKey::from_value(&key);
    if let Some((_, present)) = map_mut(vm, id).and_then(|map| map.get(&map_key)) {
        return Ok(present.clone());
    }
    let value = vm.call(&callback, &Value::Undefined, std::slice::from_ref(&key))?;
    // La fonction de rappel a pu insérer la clé : sa valeur est remplacée.
    if let Some(map) = map_mut(vm, id) {
        map.insert(map_key, (key, value.clone()));
    }
    Ok(value)
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

// ---- Opérations ensemblistes de Set ------------------------------------------

/// `GetSetRecord` (ECMA-262 §24.2.1.2) : taille, `has` et `keys` d'un objet
/// qui se comporte comme un ensemble.
struct SetRecord {
    object: Value,
    size: f64,
    has: Value,
    keys: Value,
}

fn set_record(vm: &mut Vm, other: &Value) -> JsResult<SetRecord> {
    if other.as_object().is_none() {
        return Err(vm.type_error("un objet de type ensemble est attendu"));
    }
    let size = vm.get(other, "size")?;
    let size = vm.to_number(&size)?;
    if size.is_nan() {
        return Err(vm.type_error("la taille de l'ensemble n'est pas un nombre"));
    }
    let size = size.trunc();
    if size < 0.0 {
        return Err(vm.error("RangeError", "la taille de l'ensemble est négative"));
    }
    let has = vm.get(other, "has")?;
    let has = callable(vm, &has)?;
    let keys = vm.get(other, "keys")?;
    let keys = callable(vm, &keys)?;
    Ok(SetRecord {
        object: other.clone(),
        size,
        has,
        keys,
    })
}

/// Élément normalisé (`-0` devient `+0`).
fn canonical(value: Value) -> Value {
    if value == Value::Number(0.0) {
        Value::Number(0.0)
    } else {
        value
    }
}

fn record_has(vm: &mut Vm, record: &SetRecord, value: &Value) -> JsResult<bool> {
    Ok(vm
        .call(&record.has, &record.object, std::slice::from_ref(value))?
        .truthy())
}

/// Itérateur des clés de l'autre ensemble et sa méthode `next`.
fn record_keys(vm: &mut Vm, record: &SetRecord) -> JsResult<(Value, Value)> {
    let iterator = vm.call(&record.keys, &record.object, &[])?;
    if iterator.as_object().is_none() {
        return Err(vm.type_error("« keys » doit rendre un itérateur"));
    }
    let next = vm.get(&iterator, "next")?;
    Ok((iterator, next))
}

/// Clé suivante de l'autre ensemble ; `None` quand il est épuisé.
fn next_key(vm: &mut Vm, iterator: &Value, next: &Value) -> JsResult<Option<Value>> {
    vm.tick(1)?;
    let result = vm.call(next, iterator, &[])?;
    if result.as_object().is_none() {
        return Err(vm.type_error("le résultat d'un itérateur doit être un objet"));
    }
    if vm.get(&result, "done")?.truthy() {
        return Ok(None);
    }
    vm.get(&result, "value").map(|value| Some(canonical(value)))
}

/// Ferme l'itérateur des clés quand la réponse est connue avant sa fin.
fn close_keys(vm: &mut Vm, iterator: &Value) -> JsResult<()> {
    let method = vm.get(iterator, "return")?;
    if !method.is_nullish() {
        vm.call(&method, iterator, &[])?;
    }
    Ok(())
}

fn set_len(vm: &mut Vm, id: ObjId) -> usize {
    set_mut(vm, id).map_or(0, |set| set.len())
}

/// Élément d'index `index` du Set `id`, relu à chaque tour : une fonction de
/// rappel peut modifier l'ensemble.
fn set_at(vm: &mut Vm, id: ObjId, index: usize) -> Option<Value> {
    set_mut(vm, id)?.get_index(index).map(|(_, v)| v.clone())
}

fn set_contains(vm: &mut Vm, id: ObjId, value: &Value) -> bool {
    set_mut(vm, id).is_some_and(|set| set.contains_key(&MapKey::from_value(value)))
}

fn new_set(vm: &mut Vm, items: IndexMap<MapKey, Value>) -> Value {
    Value::Object(
        vm.heap
            .alloc(Object::new(Some(vm.intrinsics.set_proto), Kind::Set(items))),
    )
}

fn set_copy(vm: &mut Vm, id: ObjId) -> IndexMap<MapKey, Value> {
    set_mut(vm, id).cloned().unwrap_or_default()
}

#[expect(
    clippy::cast_precision_loss,
    reason = "taille d'ensemble petite devant 2^53"
)]
fn this_is_smaller(vm: &mut Vm, id: ObjId, record: &SetRecord) -> bool {
    set_len(vm, id) as f64 <= record.size
}

fn set_union(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_set(vm, this)?;
    let record = set_record(vm, &arg(args, 0))?;
    let (iterator, next) = record_keys(vm, &record)?;
    let mut result = set_copy(vm, id);
    while let Some(key) = next_key(vm, &iterator, &next)? {
        result.entry(MapKey::from_value(&key)).or_insert(key);
    }
    Ok(new_set(vm, result))
}

fn set_intersection(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_set(vm, this)?;
    let record = set_record(vm, &arg(args, 0))?;
    let mut result = IndexMap::new();
    if this_is_smaller(vm, id, &record) {
        let mut index = 0;
        while let Some(value) = set_at(vm, id, index) {
            index += 1;
            if record_has(vm, &record, &value)? {
                result.entry(MapKey::from_value(&value)).or_insert(value);
            }
        }
    } else {
        let (iterator, next) = record_keys(vm, &record)?;
        while let Some(key) = next_key(vm, &iterator, &next)? {
            if set_contains(vm, id, &key) {
                result.entry(MapKey::from_value(&key)).or_insert(key);
            }
        }
    }
    Ok(new_set(vm, result))
}

fn set_difference(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_set(vm, this)?;
    let record = set_record(vm, &arg(args, 0))?;
    let mut result = set_copy(vm, id);
    if this_is_smaller(vm, id, &record) {
        let mut index = 0;
        while let Some(value) = set_at(vm, id, index) {
            index += 1;
            if record_has(vm, &record, &value)? {
                result.shift_remove(&MapKey::from_value(&value));
            }
        }
    } else {
        let (iterator, next) = record_keys(vm, &record)?;
        while let Some(key) = next_key(vm, &iterator, &next)? {
            result.shift_remove(&MapKey::from_value(&key));
        }
    }
    Ok(new_set(vm, result))
}

fn set_symmetric_difference(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_set(vm, this)?;
    let record = set_record(vm, &arg(args, 0))?;
    let (iterator, next) = record_keys(vm, &record)?;
    let mut result = set_copy(vm, id);
    while let Some(key) = next_key(vm, &iterator, &next)? {
        let map_key = MapKey::from_value(&key);
        if set_contains(vm, id, &key) {
            result.shift_remove(&map_key);
        } else {
            result.entry(map_key).or_insert(key);
        }
    }
    Ok(new_set(vm, result))
}

fn set_is_subset_of(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_set(vm, this)?;
    let record = set_record(vm, &arg(args, 0))?;
    if !this_is_smaller(vm, id, &record) {
        return Ok(Value::Bool(false));
    }
    let mut index = 0;
    while let Some(value) = set_at(vm, id, index) {
        index += 1;
        if !record_has(vm, &record, &value)? {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
}

fn set_is_superset_of(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_set(vm, this)?;
    let record = set_record(vm, &arg(args, 0))?;
    #[expect(
        clippy::cast_precision_loss,
        reason = "taille d'ensemble petite devant 2^53"
    )]
    if (set_len(vm, id) as f64) < record.size {
        return Ok(Value::Bool(false));
    }
    let (iterator, next) = record_keys(vm, &record)?;
    while let Some(key) = next_key(vm, &iterator, &next)? {
        if !set_contains(vm, id, &key) {
            close_keys(vm, &iterator)?;
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
}

fn set_is_disjoint_from(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_set(vm, this)?;
    let record = set_record(vm, &arg(args, 0))?;
    if this_is_smaller(vm, id, &record) {
        let mut index = 0;
        while let Some(value) = set_at(vm, id, index) {
            index += 1;
            if record_has(vm, &record, &value)? {
                return Ok(Value::Bool(false));
            }
        }
    } else {
        let (iterator, next) = record_keys(vm, &record)?;
        while let Some(key) = next_key(vm, &iterator, &next)? {
            if set_contains(vm, id, &key) {
                close_keys(vm, &iterator)?;
                return Ok(Value::Bool(false));
            }
        }
    }
    Ok(Value::Bool(true))
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
        ("getOrInsert", 2, map_get_or_insert),
        ("getOrInsertComputed", 2, map_get_or_insert_computed),
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
        ("union", 1, set_union),
        ("intersection", 1, set_intersection),
        ("difference", 1, set_difference),
        ("symmetricDifference", 1, set_symmetric_difference),
        ("isSubsetOf", 1, set_is_subset_of),
        ("isSupersetOf", 1, set_is_superset_of),
        ("isDisjointFrom", 1, set_is_disjoint_from),
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
}
