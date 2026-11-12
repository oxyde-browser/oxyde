//! `WeakMap`, `WeakSet`, `WeakRef` et `FinalizationRegistry` (ECMA-262 §24.3,
//! §24.4 et §26.1-26.2).
//!
//! LIMITE CONNUE : les références sont fortes. Une clé de `WeakMap`, la cible
//! d'un `WeakRef` ou d'un registre restent en vie tant que leur conteneur
//! l'est, et la fonction de nettoyage d'un `FinalizationRegistry` n'est
//! jamais appelée. Le comportement observable par un script est sinon celui
//! du standard.

use indexmap::IndexMap;

use super::{arg, callable, constructor, is_callable};
use crate::heap::{Kind, MapKey, Object, Property, Registry, Slot};
use crate::internal::same_value;
use crate::value::{ObjId, Value};
use crate::vm::{JsResult, NativeFn, Vm};

/// `CanBeHeldWeakly` (ECMA-262 §9.14) : un objet, ou un symbole qui ne vient
/// pas du registre global (`Symbol.for`).
fn held_weakly(vm: &Vm, value: &Value) -> bool {
    match value {
        Value::Object(_) => true,
        Value::Symbol(symbol) => !vm.symbol_registry.values().any(|known| known == symbol),
        _ => false,
    }
}

/// Prototype des instances de `name` : celui du constructeur global.
fn proto_of(vm: &mut Vm, name: &str) -> JsResult<Option<ObjId>> {
    let ctor = vm.get(&Value::Object(vm.global), name)?;
    if ctor.as_object().is_none() {
        return Ok(None);
    }
    Ok(vm.get(&ctor, "prototype")?.as_object())
}

pub(super) fn require_new(vm: &mut Vm, name: &str) -> JsResult<()> {
    if vm.constructing {
        Ok(())
    } else {
        Err(vm.type_error(&format!("{name} doit être appelé avec « new »")))
    }
}

/// Remplit une collection neuve à partir d'un itérable, en appelant sa
/// méthode `adder` (`set` ou `add`) pour chaque élément.
pub(super) fn fill(
    vm: &mut Vm,
    collection: &Value,
    source: &Value,
    adder: &str,
    pairs: bool,
) -> JsResult<()> {
    if source.is_nullish() {
        return Ok(());
    }
    let method = vm.get(collection, adder)?;
    let method = callable(vm, &method)?;
    let iterator = vm.iterator_for(source)?;
    while let Some(entry) = vm.iterator_next(&iterator)? {
        vm.tick(1)?;
        let added = (|| {
            if !pairs {
                return vm.call(&method, collection, std::slice::from_ref(&entry));
            }
            if entry.as_object().is_none() {
                return Err(vm.type_error("chaque entrée doit être une paire [clé, valeur]"));
            }
            let (key, value) = (vm.get(&entry, "0")?, vm.get(&entry, "1")?);
            vm.call(&method, collection, &[key, value])
        })();
        if let Err(error) = added {
            vm.iterator_close(&iterator, true)?;
            return Err(error);
        }
    }
    Ok(())
}

// ---- WeakMap -------------------------------------------------------------------

fn weak_map<'a>(
    vm: &'a mut Vm,
    this: &Value,
) -> JsResult<&'a mut IndexMap<MapKey, (Value, Value)>> {
    let id = this
        .as_object()
        .filter(|id| matches!(vm.heap.get(*id).kind, Kind::WeakMap(_)));
    match id {
        Some(id) => match &mut vm.heap.get_mut(id).kind {
            Kind::WeakMap(map) => Ok(map),
            _ => unreachable_kind(),
        },
        None => Err(vm.type_error("cette méthode attend une WeakMap comme receveur")),
    }
}

/// Le genre de l'objet vient d'être vérifié : cette branche n'est jamais prise.
fn unreachable_kind<T>() -> JsResult<T> {
    Err(Value::Undefined)
}

fn weak_map_ctor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    require_new(vm, "WeakMap")?;
    let proto = proto_of(vm, "WeakMap")?;
    let map = Value::Object(
        vm.heap
            .alloc(Object::new(proto, Kind::WeakMap(IndexMap::new()))),
    );
    fill(vm, &map, &arg(args, 0), "set", true)?;
    Ok(map)
}

fn weak_map_get(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let key = MapKey::from_value(&arg(args, 0));
    Ok(weak_map(vm, this)?
        .get(&key)
        .map_or(Value::Undefined, |(_, value)| value.clone()))
}

fn weak_map_has(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let key = MapKey::from_value(&arg(args, 0));
    Ok(Value::Bool(weak_map(vm, this)?.contains_key(&key)))
}

fn weak_map_delete(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let key = MapKey::from_value(&arg(args, 0));
    Ok(Value::Bool(
        weak_map(vm, this)?.shift_remove(&key).is_some(),
    ))
}

fn weak_key(vm: &mut Vm, key: &Value) -> JsResult<()> {
    if held_weakly(vm, key) {
        Ok(())
    } else {
        Err(vm.type_error("clé invalide : un objet ou un symbole non enregistré est attendu"))
    }
}

fn weak_map_set(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    weak_map(vm, this)?;
    let (key, value) = (arg(args, 0), arg(args, 1));
    weak_key(vm, &key)?;
    weak_map(vm, this)?.insert(MapKey::from_value(&key), (key, value));
    Ok(this.clone())
}

fn weak_map_get_or_insert(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    weak_map(vm, this)?;
    let (key, value) = (arg(args, 0), arg(args, 1));
    weak_key(vm, &key)?;
    let entry = weak_map(vm, this)?
        .entry(MapKey::from_value(&key))
        .or_insert((key, value));
    Ok(entry.1.clone())
}

fn weak_map_get_or_insert_computed(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    weak_map(vm, this)?;
    let key = arg(args, 0);
    weak_key(vm, &key)?;
    let callback = callable(vm, &arg(args, 1))?;
    let map_key = MapKey::from_value(&key);
    if let Some((_, present)) = weak_map(vm, this)?.get(&map_key) {
        return Ok(present.clone());
    }
    let value = vm.call(&callback, &Value::Undefined, std::slice::from_ref(&key))?;
    weak_map(vm, this)?.insert(map_key, (key, value.clone()));
    Ok(value)
}

// ---- WeakSet -------------------------------------------------------------------

fn weak_set<'a>(vm: &'a mut Vm, this: &Value) -> JsResult<&'a mut IndexMap<MapKey, Value>> {
    let id = this
        .as_object()
        .filter(|id| matches!(vm.heap.get(*id).kind, Kind::WeakSet(_)));
    match id {
        Some(id) => match &mut vm.heap.get_mut(id).kind {
            Kind::WeakSet(set) => Ok(set),
            _ => unreachable_kind(),
        },
        None => Err(vm.type_error("cette méthode attend un WeakSet comme receveur")),
    }
}

fn weak_set_ctor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    require_new(vm, "WeakSet")?;
    let proto = proto_of(vm, "WeakSet")?;
    let set = Value::Object(
        vm.heap
            .alloc(Object::new(proto, Kind::WeakSet(IndexMap::new()))),
    );
    fill(vm, &set, &arg(args, 0), "add", false)?;
    Ok(set)
}

fn weak_set_add(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    weak_set(vm, this)?;
    let value = arg(args, 0);
    weak_key(vm, &value)?;
    weak_set(vm, this)?.insert(MapKey::from_value(&value), value);
    Ok(this.clone())
}

fn weak_set_has(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let key = MapKey::from_value(&arg(args, 0));
    Ok(Value::Bool(weak_set(vm, this)?.contains_key(&key)))
}

fn weak_set_delete(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let key = MapKey::from_value(&arg(args, 0));
    Ok(Value::Bool(
        weak_set(vm, this)?.shift_remove(&key).is_some(),
    ))
}

// ---- WeakRef -------------------------------------------------------------------

fn weak_ref_ctor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    require_new(vm, "WeakRef")?;
    let target = arg(args, 0);
    if !held_weakly(vm, &target) {
        return Err(
            vm.type_error("WeakRef : la cible doit être un objet ou un symbole non enregistré")
        );
    }
    let proto = proto_of(vm, "WeakRef")?;
    Ok(Value::Object(
        vm.heap.alloc(Object::new(proto, Kind::WeakRef(target))),
    ))
}

fn weak_ref_deref(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    match this.as_object().map(|id| &vm.heap.get(id).kind) {
        Some(Kind::WeakRef(target)) => Ok(target.clone()),
        _ => Err(vm.type_error("deref attend un WeakRef comme receveur")),
    }
}

// ---- FinalizationRegistry ------------------------------------------------------

fn registry<'a>(vm: &'a mut Vm, this: &Value) -> JsResult<&'a mut Registry> {
    let id = this
        .as_object()
        .filter(|id| matches!(vm.heap.get(*id).kind, Kind::FinalizationRegistry(_)));
    match id {
        Some(id) => match &mut vm.heap.get_mut(id).kind {
            Kind::FinalizationRegistry(registry) => Ok(registry),
            _ => unreachable_kind(),
        },
        None => Err(vm.type_error("cette méthode attend un FinalizationRegistry comme receveur")),
    }
}

fn registry_ctor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    require_new(vm, "FinalizationRegistry")?;
    let cleanup = arg(args, 0);
    if !is_callable(vm, &cleanup) {
        return Err(vm.type_error("FinalizationRegistry : une fonction de nettoyage est attendue"));
    }
    let proto = proto_of(vm, "FinalizationRegistry")?;
    let data = Registry {
        cleanup,
        cells: Vec::new(),
    };
    Ok(Value::Object(vm.heap.alloc(Object::new(
        proto,
        Kind::FinalizationRegistry(Box::new(data)),
    ))))
}

fn registry_register(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    registry(vm, this)?;
    let (target, held, token) = (arg(args, 0), arg(args, 1), arg(args, 2));
    if !held_weakly(vm, &target) {
        return Err(
            vm.type_error("register : la cible doit être un objet ou un symbole non enregistré")
        );
    }
    if same_value(&target, &held) {
        return Err(vm.type_error("register : la valeur retenue ne peut pas être la cible"));
    }
    if token != Value::Undefined && !held_weakly(vm, &token) {
        return Err(vm.type_error("register : jeton de désinscription invalide"));
    }
    registry(vm, this)?.cells.push((target, held, token));
    Ok(Value::Undefined)
}

fn registry_unregister(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    registry(vm, this)?;
    let token = arg(args, 0);
    if !held_weakly(vm, &token) {
        return Err(vm.type_error("unregister : jeton de désinscription invalide"));
    }
    let cells = &mut registry(vm, this)?.cells;
    let before = cells.len();
    cells.retain(|(_, _, registered)| !same_value(registered, &token));
    Ok(Value::Bool(cells.len() != before))
}

/// Méthodes d'un prototype : nom, arité, fonction.
type Methods = &'static [(&'static str, u32, NativeFn)];

pub(super) fn install(vm: &mut Vm) {
    let tag = vm.well_known.to_string_tag.clone();
    let classes: &[(&str, NativeFn, Methods)] = &[
        (
            "WeakMap",
            weak_map_ctor,
            &[
                ("get", 1, weak_map_get),
                ("set", 2, weak_map_set),
                ("has", 1, weak_map_has),
                ("delete", 1, weak_map_delete),
                ("getOrInsert", 2, weak_map_get_or_insert),
                ("getOrInsertComputed", 2, weak_map_get_or_insert_computed),
            ],
        ),
        (
            "WeakSet",
            weak_set_ctor,
            &[
                ("add", 1, weak_set_add),
                ("has", 1, weak_set_has),
                ("delete", 1, weak_set_delete),
            ],
        ),
        ("WeakRef", weak_ref_ctor, &[("deref", 0, weak_ref_deref)]),
        (
            "FinalizationRegistry",
            registry_ctor,
            &[
                ("register", 2, registry_register),
                ("unregister", 1, registry_unregister),
            ],
        ),
    ];
    for (name, ctor, methods) in classes {
        let proto = vm.new_object();
        let arity = u32::from(matches!(*name, "WeakRef" | "FinalizationRegistry"));
        constructor(vm, name, arity, *ctor, proto);
        for (method, arity, func) in *methods {
            vm.define_method(proto, method, *arity, *func);
        }
        vm.heap.get_mut(proto).symbols.insert(
            tag.clone(),
            Property {
                slot: Slot::Data(Value::string(name)),
                writable: false,
                enumerable: false,
                configurable: true,
            },
        );
    }
}
