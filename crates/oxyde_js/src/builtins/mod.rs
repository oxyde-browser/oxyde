//! Bibliothèque standard : objets globaux d'ECMAScript.
//!
//! LIMITES CONNUES : pas d'`Intl` ; `WeakMap` et `WeakSet` retiennent leurs
//! clés.

mod array;
mod async_generator;
pub(crate) mod bigint;
mod collections;
mod date;
mod extras;
mod generator;
mod iterator;
mod json;
pub(crate) mod promise;
pub(crate) mod proxy;
mod reflect;
pub(crate) mod regexp;
mod regexp_symbols;
mod string;
pub(crate) mod symbol;
mod typed_array;
mod weak;

use std::sync::Arc;

use crate::heap::{Kind, Object, Property, Slot};
use crate::internal::{Descriptor, key_value, same_value};
use crate::value::{ObjId, PropertyKey, Value, number_to_string};
use crate::vm::{JsResult, NativeFn, Vm};

/// Argument d'index `i`, ou `undefined`.
pub(crate) fn arg(args: &[Value], i: usize) -> Value {
    args.get(i).cloned().unwrap_or(Value::Undefined)
}

/// Vrai si `value` est une fonction.
pub(crate) fn is_callable(vm: &Vm, value: &Value) -> bool {
    value
        .as_object()
        .is_some_and(|id| vm.heap.get(id).is_callable())
}

/// Exige une fonction en argument.
pub(crate) fn callable(vm: &mut Vm, value: &Value) -> JsResult<Value> {
    if is_callable(vm, value) {
        return Ok(value.clone());
    }
    let shown = vm.display(value);
    Err(vm.type_error(&format!("{shown} n'est pas une fonction")))
}

/// Index relatif (`slice`, `at`…) ramené dans `[0, len]`.
pub(crate) fn relative_index(
    vm: &mut Vm,
    value: &Value,
    len: usize,
    default: usize,
) -> JsResult<usize> {
    if *value == Value::Undefined {
        return Ok(default);
    }
    let n = vm.to_number(value)?;
    if n.is_nan() {
        return Ok(0);
    }
    #[expect(clippy::cast_precision_loss, reason = "longueur petite devant 2^53")]
    let len_f = len as f64;
    let clamped = if n < 0.0 {
        (len_f + n.trunc()).max(0.0)
    } else {
        n.trunc().min(len_f)
    };
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "valeur ramenée dans [0, len]"
    )]
    Ok(clamped as usize)
}

/// Crée un constructeur natif lié à `proto` et l'installe dans l'objet global.
pub(crate) fn constructor(
    vm: &mut Vm,
    name: &str,
    arity: u32,
    func: NativeFn,
    proto: ObjId,
) -> ObjId {
    let ctor = vm.new_native(name, arity, func, true);
    vm.heap.get_mut(ctor).props.insert(
        Arc::from("prototype"),
        Property {
            slot: Slot::Data(Value::Object(proto)),
            writable: false,
            enumerable: false,
            configurable: false,
        },
    );
    vm.define_hidden(proto, "constructor", Value::Object(ctor));
    vm.define_hidden(vm.global, name, Value::Object(ctor));
    ctor
}

/// Définit un accesseur natif (lecture seule) non énumérable.
pub(crate) fn getter(vm: &mut Vm, target: ObjId, name: &str, func: NativeFn) {
    let get = vm.new_function(&format!("get {name}"), 0, func);
    vm.heap.get_mut(target).props.insert(
        Arc::from(name),
        Property {
            slot: Slot::Accessor {
                get: Some(get),
                set: None,
            },
            writable: true,
            enumerable: false,
            configurable: true,
        },
    );
}

pub(crate) fn install(vm: &mut Vm) {
    install_object(vm);
    install_function(vm);
    install_errors(vm);
    install_boolean(vm);
    install_globals(vm);
    array::install(vm);
    string::install(vm);
    json::install(vm);
    collections::install(vm);
    weak::install(vm);
    symbol::install(vm);
    iterator::install(vm);
    bigint::install(vm);
    date::install(vm);
    typed_array::install(vm);
    generator::install(vm);
    async_generator::install(vm);
    promise::install(vm);
    regexp::install(vm);
    regexp_symbols::install(vm);
    reflect::install(vm);
    proxy::install(vm);
    extras::install(vm);
    install_function_kinds(vm);
    array::install_from_async(vm);
    let iterator = PropertyKey::Symbol(vm.well_known.iterator.clone());
    let values = vm.get_key(&Value::Object(vm.intrinsics.array_proto), &iterator);
    let next = vm.get(&Value::Object(vm.intrinsics.iterator_proto), "next");
    if let (Ok(Value::Object(values)), Ok(Value::Object(next))) = (values, next) {
        vm.iteration_intrinsics = Some((values, next));
    }
}

// ---- Object ------------------------------------------------------------------

fn object_arg(vm: &mut Vm, args: &[Value], what: &str) -> JsResult<ObjId> {
    match arg(args, 0).as_object() {
        Some(id) => Ok(id),
        None => Err(vm.type_error(&format!("{what} : un objet est attendu"))),
    }
}

fn object_ctor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    match arg(args, 0) {
        Value::Undefined | Value::Null | Value::Hole => Ok(Value::Object(vm.new_object())),
        other => vm.to_object(&other),
    }
}

fn object_keys(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let target = vm.to_object(&arg(args, 0))?;
    let keys = match target.as_object() {
        Some(id) => vm.enumerable_own_keys(id)?,
        None => Vec::new(),
    };
    Ok(vm.new_array(keys.into_iter().map(Value::String).collect()))
}

/// `EnumerableOwnProperties` (ECMA-262 §7.3.23) : valeurs, ou paires
/// `[clé, valeur]`, des propriétés propres énumérables de clé chaîne.
fn enumerable_own_values(vm: &mut Vm, target: &Value, entries: bool) -> JsResult<Vec<Value>> {
    let Some(id) = target.as_object() else {
        return Ok(Vec::new());
    };
    let mut values = Vec::new();
    for key in vm.own_property_keys(id)? {
        let PropertyKey::String(name) = &key else {
            continue;
        };
        // Relu à chaque clé : une lecture précédente a pu la supprimer.
        if !vm
            .get_own_property(id, &key)?
            .is_some_and(|desc| desc.is_enumerable())
        {
            continue;
        }
        let value = vm.get(target, name)?;
        values.push(if entries {
            vm.new_array(vec![Value::String(Arc::clone(name)), value])
        } else {
            value
        });
    }
    Ok(values)
}

fn object_values(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let target = vm.to_object(&arg(args, 0))?;
    let values = enumerable_own_values(vm, &target, false)?;
    Ok(vm.new_array(values))
}

fn object_entries(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let target = vm.to_object(&arg(args, 0))?;
    let entries = enumerable_own_values(vm, &target, true)?;
    Ok(vm.new_array(entries))
}

fn object_from_entries(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let result = vm.new_object();
    for entry in vm.iterate_to_vec(&arg(args, 0))? {
        if entry.as_object().is_none() {
            return Err(vm.type_error("Object.fromEntries : chaque entrée doit être un objet"));
        }
        let key = vm.get(&entry, "0")?;
        let value = vm.get(&entry, "1")?;
        let key = vm.to_key(&key)?;
        vm.create_data_property(result, &key, value)?;
    }
    Ok(Value::Object(result))
}

fn object_assign(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let target = vm.to_object(&arg(args, 0))?;
    for source in args.iter().skip(1) {
        if source.is_nullish() {
            continue;
        }
        let from = vm.to_object(source)?;
        let Some(id) = from.as_object() else {
            continue;
        };
        for key in vm.own_property_keys(id)? {
            if !vm
                .get_own_property(id, &key)?
                .is_some_and(|desc| desc.is_enumerable())
            {
                continue;
            }
            let value = vm.get_key(&from, &key)?;
            if !vm.put_key(&target, &key, value)? {
                return Err(vm.type_error(&format!(
                    "Object.assign : la propriété « {} » ne peut pas être écrite",
                    key.describe()
                )));
            }
        }
    }
    Ok(target)
}

/// `ObjectDefineProperties` (ECMA-262 §20.1.2.3.1) : tous les descripteurs
/// sont lus avant la première définition.
fn define_properties(vm: &mut Vm, id: ObjId, properties: &Value) -> JsResult<()> {
    let properties = vm.to_object(properties)?;
    let Some(source) = properties.as_object() else {
        return Ok(());
    };
    let mut descriptors = Vec::new();
    for key in vm.own_property_keys(source)? {
        if vm
            .get_own_property(source, &key)?
            .is_some_and(|desc| desc.is_enumerable())
        {
            let descriptor = vm.get_key(&properties, &key)?;
            descriptors.push((key, vm.to_descriptor(&descriptor)?));
        }
    }
    for (key, descriptor) in descriptors {
        vm.define_or_throw(id, &key, descriptor)?;
    }
    Ok(())
}

fn object_create(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let proto = match arg(args, 0) {
        Value::Null => None,
        Value::Object(id) => Some(id),
        _ => return Err(vm.type_error("Object.create : le prototype doit être un objet ou null")),
    };
    let object = vm.heap.alloc(Object::new(proto, Kind::Ordinary));
    let properties = arg(args, 1);
    if properties != Value::Undefined {
        define_properties(vm, object, &properties)?;
    }
    Ok(Value::Object(object))
}

fn object_get_prototype_of(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let target = vm.to_object(&arg(args, 0))?;
    let proto = match target.as_object() {
        Some(id) => vm.get_prototype_of(id)?,
        None => None,
    };
    Ok(proto.map_or(Value::Null, Value::Object))
}

fn object_set_prototype_of(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let target = arg(args, 0);
    if target.is_nullish() {
        return Err(vm.type_error("Object.setPrototypeOf : la cible est null ou undefined"));
    }
    let proto = match arg(args, 1) {
        Value::Null => None,
        Value::Object(id) => Some(id),
        _ => {
            return Err(
                vm.type_error("Object.setPrototypeOf : le prototype doit être un objet ou null")
            );
        }
    };
    if let Some(id) = target.as_object()
        && !vm.set_prototype_of(id, proto)?
    {
        return Err(vm.type_error("Object.setPrototypeOf : le prototype ne peut pas être changé"));
    }
    Ok(target)
}

/// Applique l'objet descripteur `descriptor` (`{ value, writable, get, set… }`)
/// à la propriété `key` de `target`.
pub(super) fn define_from_descriptor(
    vm: &mut Vm,
    target: &Value,
    key: &PropertyKey,
    descriptor: &Value,
) -> JsResult<()> {
    let Some(id) = target.as_object() else {
        return Err(vm.type_error("Object.defineProperty : un objet est attendu"));
    };
    let descriptor = vm.to_descriptor(descriptor)?;
    vm.define_or_throw(id, key, descriptor)
}

fn object_define_property(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let target = arg(args, 0);
    if target.as_object().is_none() {
        return Err(vm.type_error("Object.defineProperty : un objet est attendu"));
    }
    let key = vm.to_key(&arg(args, 1))?;
    define_from_descriptor(vm, &target, &key, &arg(args, 2))?;
    Ok(target)
}

fn object_define_properties(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let target = arg(args, 0);
    let id = object_arg(vm, args, "Object.defineProperties")?;
    define_properties(vm, id, &arg(args, 1))?;
    Ok(target)
}

/// Clés propres de l'argument converti en objet : chaînes ou symboles.
fn own_keys_of(vm: &mut Vm, args: &[Value], symbols: bool) -> JsResult {
    let target = vm.to_object(&arg(args, 0))?;
    let keys = match target.as_object() {
        Some(id) => vm.own_property_keys(id)?,
        None => Vec::new(),
    };
    let keys = keys
        .iter()
        .filter(|key| matches!(key, PropertyKey::Symbol(_)) == symbols)
        .map(key_value)
        .collect();
    Ok(vm.new_array(keys))
}

fn object_get_own_property_names(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    own_keys_of(vm, args, false)
}

fn object_get_own_property_symbols(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    own_keys_of(vm, args, true)
}

fn object_get_own_property_descriptor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let target = vm.to_object(&arg(args, 0))?;
    let key = vm.to_key(&arg(args, 1))?;
    match target.as_object() {
        Some(id) => own_property_descriptor(vm, id, &key),
        None => Ok(Value::Undefined),
    }
}

fn object_get_own_property_descriptors(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let target = vm.to_object(&arg(args, 0))?;
    let result = vm.new_object();
    if let Some(id) = target.as_object() {
        for key in vm.own_property_keys(id)? {
            let descriptor = own_property_descriptor(vm, id, &key)?;
            if descriptor != Value::Undefined {
                vm.create_data_property(result, &key, descriptor)?;
            }
        }
    }
    Ok(Value::Object(result))
}

/// Descripteur de la propriété propre `key` de `id`, ou `undefined`.
pub(super) fn own_property_descriptor(vm: &mut Vm, id: ObjId, key: &PropertyKey) -> JsResult {
    Ok(match vm.get_own_property(id, key)? {
        Some(descriptor) => vm.from_descriptor(&descriptor),
        None => Value::Undefined,
    })
}

/// Fige `id` (`Object.freeze`).
pub(crate) fn freeze(vm: &mut Vm, id: ObjId) -> JsResult<()> {
    set_integrity(vm, id, true, "Object.freeze")
}

/// `SetIntegrityLevel` (ECMA-262 §7.3.15) : scelle ou fige `id`.
fn set_integrity(vm: &mut Vm, id: ObjId, frozen: bool, what: &str) -> JsResult<()> {
    if !vm.prevent_extensions(id)? {
        return Err(vm.type_error(&format!(
            "{what} : l'objet ne peut pas être rendu non extensible"
        )));
    }
    for key in vm.own_property_keys(id)? {
        let descriptor = if frozen {
            let Some(current) = vm.get_own_property(id, &key)? else {
                continue;
            };
            Descriptor {
                configurable: Some(false),
                writable: (!current.is_accessor()).then_some(false),
                ..Descriptor::default()
            }
        } else {
            Descriptor {
                configurable: Some(false),
                ..Descriptor::default()
            }
        };
        vm.define_or_throw(id, &key, descriptor)?;
    }
    Ok(())
}

/// `TestIntegrityLevel` (ECMA-262 §7.3.16).
fn test_integrity(vm: &mut Vm, id: ObjId, frozen: bool) -> JsResult<bool> {
    if vm.is_extensible(id)? {
        return Ok(false);
    }
    for key in vm.own_property_keys(id)? {
        if let Some(current) = vm.get_own_property(id, &key)?
            && (current.is_configurable()
                || (frozen && !current.is_accessor() && current.is_writable()))
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn object_freeze(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let target = arg(args, 0);
    if let Some(id) = target.as_object() {
        set_integrity(vm, id, true, "Object.freeze")?;
    }
    Ok(target)
}

fn object_seal(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let target = arg(args, 0);
    if let Some(id) = target.as_object() {
        set_integrity(vm, id, false, "Object.seal")?;
    }
    Ok(target)
}

fn object_is_frozen(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    Ok(Value::Bool(match arg(args, 0).as_object() {
        Some(id) => test_integrity(vm, id, true)?,
        None => true,
    }))
}

fn object_is_sealed(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    Ok(Value::Bool(match arg(args, 0).as_object() {
        Some(id) => test_integrity(vm, id, false)?,
        None => true,
    }))
}

fn object_prevent_extensions(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let target = arg(args, 0);
    if let Some(id) = target.as_object()
        && !vm.prevent_extensions(id)?
    {
        return Err(vm.type_error(
            "Object.preventExtensions : l'objet ne peut pas être rendu non extensible",
        ));
    }
    Ok(target)
}

fn object_is_extensible(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    Ok(Value::Bool(match arg(args, 0).as_object() {
        Some(id) => vm.is_extensible(id)?,
        None => false,
    }))
}

fn object_is(_vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    Ok(Value::Bool(same_value(&arg(args, 0), &arg(args, 1))))
}

fn object_has_own(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let target = vm.to_object(&arg(args, 0))?;
    let key = vm.to_key(&arg(args, 1))?;
    Ok(Value::Bool(match target.as_object() {
        Some(id) => vm.get_own_property(id, &key)?.is_some(),
        None => false,
    }))
}

/// `Object.prototype.toLocaleString` : `this.toString()`.
fn object_proto_to_locale_string(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let method = vm.get(this, "toString")?;
    vm.call(&method, this, &[])
}

fn object_proto_has_own_property(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let key = vm.to_key(&arg(args, 0))?;
    let target = vm.to_object(this)?;
    Ok(Value::Bool(match target.as_object() {
        Some(id) => vm.get_own_property(id, &key)?.is_some(),
        None => false,
    }))
}

fn object_proto_is_prototype_of(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let Some(mut current) = arg(args, 0).as_object() else {
        return Ok(Value::Bool(false));
    };
    let Some(proto) = vm.to_object(this)?.as_object() else {
        return Ok(Value::Bool(false));
    };
    for _ in 0..10_000 {
        match vm.get_prototype_of(current)? {
            Some(p) if p == proto => return Ok(Value::Bool(true)),
            Some(p) => current = p,
            None => break,
        }
    }
    Ok(Value::Bool(false))
}

fn object_proto_property_is_enumerable(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let key = vm.to_key(&arg(args, 0))?;
    let target = vm.to_object(this)?;
    Ok(Value::Bool(match target.as_object() {
        Some(id) => vm
            .get_own_property(id, &key)?
            .is_some_and(|desc| desc.is_enumerable()),
        None => false,
    }))
}

pub(super) fn object_proto_to_string(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let tag = match this {
        Value::Undefined | Value::Hole => "Undefined",
        Value::Null => "Null",
        Value::Bool(_) => "Boolean",
        Value::Number(_) => "Number",
        Value::String(_) => "String",
        Value::Symbol(_) => "Symbol",
        Value::BigInt(_) => "BigInt",
        Value::Object(id) => match &vm.heap.get(*id).kind {
            Kind::Array(_) => "Array",
            Kind::Proxy(_) if array::is_array(vm, this) => "Array",
            Kind::Proxy(proxy) if proxy.callable => "Function",
            Kind::Closure(_)
            | Kind::Native { .. }
            | Kind::NativeData { .. }
            | Kind::Bound { .. } => "Function",
            Kind::Error => "Error",
            Kind::Arguments => "Arguments",
            Kind::Date(_) => "Date",
            Kind::RegExp { .. } => "RegExp",
            Kind::Primitive(Value::Bool(_)) => "Boolean",
            Kind::Primitive(Value::Number(_)) => "Number",
            Kind::Primitive(Value::String(_)) => "String",
            _ => "Object",
        },
    };
    // `Symbol.toStringTag` l'emporte sur l'étiquette par défaut.
    let tag_key = PropertyKey::Symbol(vm.well_known.to_string_tag.clone());
    if !this.is_nullish()
        && let Value::String(custom) = vm.get_key(this, &tag_key)?
    {
        return Ok(Value::string(&format!("[object {custom}]")));
    }
    Ok(Value::string(&format!("[object {tag}]")))
}

fn object_proto_value_of(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    vm.to_object(this)
}

fn object_proto_get_proto(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let target = vm.to_object(this)?;
    let proto = match target.as_object() {
        Some(id) => vm.get_prototype_of(id)?,
        None => None,
    };
    Ok(proto.map_or(Value::Null, Value::Object))
}

fn object_proto_set_proto(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    if this.is_nullish() {
        return Err(vm.type_error("__proto__ : this est null ou undefined"));
    }
    let proto = match arg(args, 0) {
        Value::Null => None,
        Value::Object(proto) => Some(proto),
        _ => return Ok(Value::Undefined),
    };
    if let Some(id) = this.as_object()
        && !vm.set_prototype_of(id, proto)?
    {
        return Err(vm.type_error("__proto__ : le prototype ne peut pas être changé"));
    }
    Ok(Value::Undefined)
}

/// `__defineGetter__` et `__defineSetter__` (annexe B).
fn define_accessor(vm: &mut Vm, this: &Value, args: &[Value], getter: bool) -> JsResult {
    let target = vm.to_object(this)?;
    let function = arg(args, 1);
    if !is_callable(vm, &function) {
        return Err(vm.type_error("une fonction est attendue"));
    }
    let key = vm.to_key(&arg(args, 0))?;
    let mut descriptor = Descriptor {
        enumerable: Some(true),
        configurable: Some(true),
        ..Descriptor::default()
    };
    if getter {
        descriptor.get = Some(function);
    } else {
        descriptor.set = Some(function);
    }
    if let Some(id) = target.as_object() {
        vm.define_or_throw(id, &key, descriptor)?;
    }
    Ok(Value::Undefined)
}

fn object_proto_define_getter(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    define_accessor(vm, this, args, true)
}

fn object_proto_define_setter(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    define_accessor(vm, this, args, false)
}

/// `__lookupGetter__` et `__lookupSetter__` (annexe B).
fn lookup_accessor(vm: &mut Vm, this: &Value, args: &[Value], getter: bool) -> JsResult {
    let target = vm.to_object(this)?;
    let key = vm.to_key(&arg(args, 0))?;
    let mut current = target.as_object();
    for _ in 0..10_000 {
        let Some(id) = current else { break };
        if let Some(descriptor) = vm.get_own_property(id, &key)? {
            let accessor = if getter {
                descriptor.get
            } else {
                descriptor.set
            };
            return Ok(accessor.unwrap_or(Value::Undefined));
        }
        current = vm.get_prototype_of(id)?;
    }
    Ok(Value::Undefined)
}

fn object_proto_lookup_getter(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    lookup_accessor(vm, this, args, true)
}

fn object_proto_lookup_setter(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    lookup_accessor(vm, this, args, false)
}

fn install_object(vm: &mut Vm) {
    let proto = vm.intrinsics.object_proto;
    let ctor = constructor(vm, "Object", 1, object_ctor, proto);
    let statics: &[(&str, u32, NativeFn)] = &[
        ("keys", 1, object_keys),
        ("values", 1, object_values),
        ("entries", 1, object_entries),
        ("fromEntries", 1, object_from_entries),
        ("assign", 2, object_assign),
        ("create", 2, object_create),
        ("getPrototypeOf", 1, object_get_prototype_of),
        ("setPrototypeOf", 2, object_set_prototype_of),
        ("defineProperty", 3, object_define_property),
        ("defineProperties", 2, object_define_properties),
        ("getOwnPropertyNames", 1, object_get_own_property_names),
        ("getOwnPropertySymbols", 1, object_get_own_property_symbols),
        (
            "getOwnPropertyDescriptor",
            2,
            object_get_own_property_descriptor,
        ),
        (
            "getOwnPropertyDescriptors",
            1,
            object_get_own_property_descriptors,
        ),
        ("freeze", 1, object_freeze),
        ("isFrozen", 1, object_is_frozen),
        ("seal", 1, object_seal),
        ("isSealed", 1, object_is_sealed),
        ("preventExtensions", 1, object_prevent_extensions),
        ("isExtensible", 1, object_is_extensible),
        ("is", 2, object_is),
        ("hasOwn", 2, object_has_own),
    ];
    for (name, arity, func) in statics {
        vm.define_method(ctor, name, *arity, *func);
    }
    let methods: &[(&str, u32, NativeFn)] = &[
        ("hasOwnProperty", 1, object_proto_has_own_property),
        ("isPrototypeOf", 1, object_proto_is_prototype_of),
        (
            "propertyIsEnumerable",
            1,
            object_proto_property_is_enumerable,
        ),
        ("toString", 0, object_proto_to_string),
        ("toLocaleString", 0, object_proto_to_locale_string),
        ("valueOf", 0, object_proto_value_of),
        ("__defineGetter__", 2, object_proto_define_getter),
        ("__defineSetter__", 2, object_proto_define_setter),
        ("__lookupGetter__", 1, object_proto_lookup_getter),
        ("__lookupSetter__", 1, object_proto_lookup_setter),
    ];
    for (name, arity, func) in methods {
        vm.define_method(proto, name, *arity, *func);
    }
    let get = vm.new_function("__proto__", 0, object_proto_get_proto);
    let set = vm.new_function("__proto__", 1, object_proto_set_proto);
    vm.heap.get_mut(proto).props.insert(
        Arc::from("__proto__"),
        Property {
            slot: Slot::Accessor {
                get: Some(get),
                set: Some(set),
            },
            writable: true,
            enumerable: false,
            configurable: true,
        },
    );
}

// ---- Function ----------------------------------------------------------------

/// `new Function(p1, p2, …, corps)` : la fonction est créée dans la portée
/// globale, comme le veut le langage.
/// `CreateDynamicFunction` (ECMA-262 §20.2.1.1.1) : la fonction dont les
/// derniers arguments donnent les paramètres puis le corps ; `prefix` est
/// `function`, `function*`, `async function` ou `async function*`.
fn dynamic_function(vm: &mut Vm, prefix: &str, args: &[Value]) -> JsResult {
    let mut texts = Vec::with_capacity(args.len());
    for value in args {
        texts.push(vm.to_string(value)?);
    }
    let body = texts.pop().unwrap_or_else(|| Arc::from(""));
    let params = texts.join(",");
    // Les sauts de ligne isolent un éventuel commentaire en fin de paramètres ou de corps.
    vm.eval_global(&format!("({prefix} anonymous({params}\n) {{\n{body}\n}})"))
}

fn function_ctor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    dynamic_function(vm, "function", args)
}

fn generator_function_ctor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    dynamic_function(vm, "function*", args)
}

fn async_function_ctor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    dynamic_function(vm, "async function", args)
}

fn async_generator_function_ctor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    dynamic_function(vm, "async function*", args)
}

/// `Function.prototype` est une fonction qui accepte tout et rend `undefined`.
fn function_proto_call(_vm: &mut Vm, _this: &Value, _args: &[Value]) -> JsResult {
    Ok(Value::Undefined)
}

/// Accesseur de `Function.prototype.caller` et `.arguments` : `TypeError`,
/// sauf pour une fonction ordinaire non stricte, où il rend `undefined`
/// (aucune pile d'appels n'est exposée).
fn restricted_property(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let lenient = this
        .as_object()
        .is_some_and(|id| match &vm.heap.get(id).kind {
            Kind::Closure(closure) => {
                let proto = &closure.proto;
                !proto.strict && proto.is_constructor()
            }
            _ => false,
        });
    if lenient {
        Ok(Value::Undefined)
    } else {
        Err(vm.type_error("« caller » et « arguments » ne sont pas accessibles sur cette fonction"))
    }
}

/// `GeneratorFunction`, `AsyncFunction` et `AsyncGeneratorFunction` : des
/// constructeurs sans nom global, atteints par le prototype de leurs fonctions.
fn install_function_kinds(vm: &mut Vm) {
    let function_ctor = vm
        .get(&Value::Object(vm.global), "Function")
        .ok()
        .and_then(|ctor| ctor.as_object());
    let tag = vm.well_known.to_string_tag.clone();
    let intr = &vm.intrinsics;
    let kinds: [(&str, NativeFn, ObjId, Option<ObjId>); 3] = [
        (
            "GeneratorFunction",
            generator_function_ctor,
            intr.generator_function_proto,
            Some(intr.generator_proto),
        ),
        (
            "AsyncFunction",
            async_function_ctor,
            intr.async_function_proto,
            None,
        ),
        (
            "AsyncGeneratorFunction",
            async_generator_function_ctor,
            intr.async_generator_function_proto,
            Some(intr.async_generator_proto),
        ),
    ];
    let fixed = |value: Value| Property {
        slot: Slot::Data(value),
        writable: false,
        enumerable: false,
        configurable: true,
    };
    for (name, func, proto, instances) in kinds {
        let ctor = vm.new_native(name, 1, func, true);
        vm.heap.get_mut(ctor).proto = function_ctor;
        vm.heap.get_mut(ctor).props.insert(
            Arc::from("prototype"),
            Property {
                configurable: false,
                ..fixed(Value::Object(proto))
            },
        );
        let function_proto = vm.intrinsics.function_proto;
        let object = vm.heap.get_mut(proto);
        object.proto = Some(function_proto);
        object
            .props
            .insert(Arc::from("constructor"), fixed(Value::Object(ctor)));
        object
            .symbols
            .insert(tag.clone(), fixed(Value::string(name)));
        if let Some(instances) = instances {
            object
                .props
                .insert(Arc::from("prototype"), fixed(Value::Object(instances)));
            vm.heap
                .get_mut(instances)
                .props
                .insert(Arc::from("constructor"), fixed(Value::Object(proto)));
        }
    }
}

/// `eval(code)`.
///
/// Appelée comme une fonction ordinaire, c'est un « eval indirect » : le
/// code est évalué dans la portée globale. L'appel écrit `eval(…)` est
/// traité par la machine (voir `Op::DirectEval`).
fn global_eval(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    match arg(args, 0) {
        Value::String(source) => vm.eval_global(&source),
        other => Ok(other),
    }
}

fn function_call(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let func = callable(vm, this)?;
    vm.call(&func, &arg(args, 0), args.get(1..).unwrap_or(&[]))
}

fn function_apply(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let func = callable(vm, this)?;
    let list = match arg(args, 1) {
        Value::Undefined | Value::Null | Value::Hole => Vec::new(),
        other => array_like_to_vec(vm, &other)?,
    };
    vm.call(&func, &arg(args, 0), &list)
}

/// Éléments d'un objet « de type tableau » (`length` et index).
pub(crate) fn array_like_to_vec(vm: &mut Vm, value: &Value) -> JsResult<Vec<Value>> {
    if let Some(Kind::Array(_)) = value.as_object().map(|id| &vm.heap.get(id).kind) {
        return vm.iterate_to_vec(value);
    }
    if value.as_object().is_none() {
        return Err(vm.type_error("un objet de type tableau est attendu"));
    }
    let length = vm.get(value, "length")?;
    let length = vm.to_number(&length)?;
    // Borne : un `length` hostile ne doit pas faire réserver des gigaoctets.
    if !(0.0..=16_777_216.0).contains(&length) {
        return Err(vm.error("RangeError", "longueur de liste d'arguments invalide"));
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "borné juste au-dessus"
    )]
    let length = length as usize;
    let mut items = Vec::with_capacity(length);
    for i in 0..length {
        items.push(vm.get(value, &i.to_string())?);
    }
    Ok(items)
}

fn function_bind(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let func = callable(vm, this)?;
    let Some(target) = func.as_object() else {
        return Ok(Value::Undefined);
    };
    let bound_args = args.get(1..).unwrap_or(&[]).to_vec();
    let proto = vm.get_prototype_of(target)?;
    // Longueur : celle de la cible, diminuée des arguments déjà liés.
    let mut length = 0.0;
    let length_key = PropertyKey::String(Arc::from("length"));
    if vm.get_own_property(target, &length_key)?.is_some()
        && let Value::Number(n) = vm.get(&func, "length")?
    {
        #[expect(clippy::cast_precision_loss, reason = "nombre d'arguments, petit")]
        let bound_count = bound_args.len() as f64;
        length = if n == f64::INFINITY {
            n
        } else if n.is_nan() || n == f64::NEG_INFINITY {
            0.0
        } else {
            (n.trunc() - bound_count).max(0.0)
        };
    }
    let name = match vm.get(&func, "name")? {
        Value::String(name) => name,
        _ => Arc::from(""),
    };
    let bound = Object::new(
        proto,
        Kind::Bound {
            target,
            this: arg(args, 0),
            args: bound_args,
        },
    );
    let id = vm.heap.alloc(bound);
    for (key, value) in [
        ("length", Value::Number(length)),
        ("name", Value::string(&format!("bound {name}"))),
    ] {
        vm.heap.get_mut(id).props.insert(
            Arc::from(key),
            Property {
                slot: Slot::Data(value),
                writable: false,
                enumerable: false,
                configurable: true,
            },
        );
    }
    Ok(Value::Object(id))
}

/// Vrai si `name` peut suivre `function` dans l'écriture d'une fonction
/// native : identifiant, `[Symbol.x]`, éventuellement précédé de `get` ou `set`.
fn printable_name(name: &str) -> bool {
    let name = name
        .strip_prefix("get ")
        .or_else(|| name.strip_prefix("set "))
        .unwrap_or(name);
    let word = |text: &str| {
        !text.is_empty()
            && !text.starts_with(|c: char| c.is_ascii_digit())
            && text
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
    };
    match name.strip_prefix('[').and_then(|n| n.strip_suffix(']')) {
        Some(inner) => inner.split('.').all(word),
        None => word(name),
    }
}

/// `Function.prototype.toString` (ECMA-262 §20.2.3.5) : le texte source
/// d'une fonction du script, la forme `function nom() { [native code] }`
/// pour les autres.
fn function_to_string(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let Some(id) = this.as_object().filter(|id| vm.heap.get(*id).is_callable()) else {
        return Err(vm.type_error("Function.prototype.toString attend une fonction"));
    };
    if let Kind::Closure(closure) = &vm.heap.get(id).kind
        && let Some(source) = &closure.proto.source
        && !vm.self_hosted.contains(&id)
    {
        return Ok(Value::string(&source.text()));
    }
    let name = match &vm.heap.get(id).kind {
        Kind::Proxy(_) | Kind::Bound { .. } => None,
        _ => match vm.heap.get(id).props.get("name").map(|p| &p.slot) {
            Some(Slot::Data(Value::String(name))) if printable_name(name) => Some(name.clone()),
            _ => None,
        },
    };
    Ok(Value::string(&format!(
        "function {}() {{ [native code] }}",
        name.as_deref().unwrap_or("")
    )))
}

fn install_function(vm: &mut Vm) {
    let proto = vm.intrinsics.function_proto;
    constructor(vm, "Function", 1, function_ctor, proto);
    vm.define_method(proto, "call", 1, function_call);
    vm.define_method(proto, "apply", 2, function_apply);
    vm.define_method(proto, "bind", 1, function_bind);
    vm.define_method(proto, "toString", 0, function_to_string);
    vm.heap.get_mut(proto).kind = Kind::Native {
        func: function_proto_call,
        constructor: false,
    };
    // `caller` et `arguments` : un même accesseur, en lecture et en écriture.
    let thrower = vm.new_function("", 0, restricted_property);
    for name in ["caller", "arguments"] {
        vm.heap.get_mut(proto).props.insert(
            Arc::from(name),
            Property {
                slot: Slot::Accessor {
                    get: Some(thrower),
                    set: Some(thrower),
                },
                writable: true,
                enumerable: false,
                configurable: true,
            },
        );
    }
    // `Function.prototype` est lui-même une fonction : longueur 0, nom vide.
    for (name, value) in [("length", Value::Number(0.0)), ("name", Value::string(""))] {
        vm.heap.get_mut(proto).props.insert(
            Arc::from(name),
            Property {
                slot: Slot::Data(value),
                writable: false,
                enumerable: false,
                configurable: true,
            },
        );
    }
}

// ---- Erreurs -------------------------------------------------------------------

/// Construit une erreur dont le prototype est celui de la classe `class`.
fn make_error(vm: &mut Vm, class: &str, args: &[Value]) -> JsResult {
    let message = match arg(args, 0) {
        Value::Undefined => None,
        other => Some(vm.to_string(&other)?),
    };
    let error = vm.error(class, message.as_deref().unwrap_or(""));
    if message.is_none()
        && let Some(id) = error.as_object()
    {
        vm.heap.get_mut(id).props.shift_remove("message");
    }
    if let Value::Object(options) = arg(args, 1)
        && vm.has_property_op(options, &PropertyKey::String(Arc::from("cause")))?
    {
        let cause = vm.get(&Value::Object(options), "cause")?;
        if let Some(id) = error.as_object() {
            vm.define_hidden(id, "cause", cause);
        }
    }
    Ok(error)
}

macro_rules! error_ctor {
    ($name:ident, $class:literal) => {
        fn $name(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
            make_error(vm, $class, args)
        }
    };
}
error_ctor!(error_ctor_error, "Error");
error_ctor!(error_ctor_type, "TypeError");
error_ctor!(error_ctor_range, "RangeError");
error_ctor!(error_ctor_reference, "ReferenceError");
error_ctor!(error_ctor_syntax, "SyntaxError");
error_ctor!(error_ctor_eval, "EvalError");
error_ctor!(error_ctor_uri, "URIError");

fn error_to_string(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    if this.as_object().is_none() {
        return Err(vm.type_error("Error.prototype.toString : un objet est attendu"));
    }
    let name = match vm.get(this, "name")? {
        Value::Undefined => Arc::from("Error"),
        other => vm.to_string(&other)?,
    };
    let message = match vm.get(this, "message")? {
        Value::Undefined => Arc::from(""),
        other => vm.to_string(&other)?,
    };
    Ok(Value::string(
        &match (name.is_empty(), message.is_empty()) {
            (true, _) => message.to_string(),
            (_, true) => name.to_string(),
            _ => format!("{name}: {message}"),
        },
    ))
}

/// Propriété de données `key` de `id` ou de ses prototypes, lue sans
/// appeler aucun accesseur ni piège.
fn plain_text(vm: &Vm, id: ObjId, key: &str) -> Option<crate::value::JsStr> {
    let mut current = Some(id);
    for _ in 0..1_000 {
        let object = vm.heap.get(current?);
        if let Some(property) = object.props.get(key) {
            return match &property.slot {
                Slot::Data(Value::String(text)) => Some(Arc::clone(text)),
                _ => None,
            };
        }
        current = object.proto;
    }
    None
}

/// Lecture de `Error.prototype.stack` : une description de l'erreur (la
/// pile d'appels n'est pas enregistrée), `undefined` pour un autre objet.
fn error_stack_get(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let Some(id) = this.as_object() else {
        return Err(vm.type_error("« stack » attend un objet comme receveur"));
    };
    if !matches!(vm.heap.get(id).kind, Kind::Error) {
        return Ok(Value::Undefined);
    }
    let name = plain_text(vm, id, "name").unwrap_or_else(|| Arc::from("Error"));
    let message = plain_text(vm, id, "message").unwrap_or_else(|| Arc::from(""));
    Ok(Value::string(&if message.is_empty() {
        name.to_string()
    } else {
        format!("{name}: {message}")
    }))
}

/// Écriture de `Error.prototype.stack` : crée la propriété sur le receveur,
/// jamais sur `Error.prototype`.
fn error_stack_set(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let Some(id) = this.as_object() else {
        return Err(vm.type_error("« stack » attend un objet comme receveur"));
    };
    let value = arg(args, 0);
    if !matches!(value, Value::String(_)) {
        return Err(vm.type_error("« stack » attend une chaîne"));
    }
    if id == vm.intrinsics.error_proto {
        return Err(vm.type_error("« stack » ne peut pas être écrite sur Error.prototype"));
    }
    let key = PropertyKey::String(Arc::from("stack"));
    let written = if vm.get_own_property(id, &key)?.is_none() {
        vm.create_data_property(id, &key, value)?
    } else {
        vm.put_key(this, &key, value)?
    };
    if written {
        Ok(Value::Undefined)
    } else {
        Err(vm.type_error("« stack » ne peut pas être écrite sur cet objet"))
    }
}

fn install_errors(vm: &mut Vm) {
    let base = vm.intrinsics.error_proto;
    vm.error_protos.insert("Error", base);
    constructor(vm, "Error", 1, error_ctor_error, base);
    vm.define_hidden(base, "name", Value::string("Error"));
    vm.define_hidden(base, "message", Value::string(""));
    vm.define_method(base, "toString", 0, error_to_string);
    let stack_get = vm.new_function("get stack", 0, error_stack_get);
    let stack_set = vm.new_function("set stack", 1, error_stack_set);
    vm.heap.get_mut(base).props.insert(
        Arc::from("stack"),
        Property {
            slot: Slot::Accessor {
                get: Some(stack_get),
                set: Some(stack_set),
            },
            writable: true,
            enumerable: false,
            configurable: true,
        },
    );
    let classes: &[(&'static str, NativeFn)] = &[
        ("TypeError", error_ctor_type),
        ("RangeError", error_ctor_range),
        ("ReferenceError", error_ctor_reference),
        ("SyntaxError", error_ctor_syntax),
        ("EvalError", error_ctor_eval),
        ("URIError", error_ctor_uri),
    ];
    for (name, func) in classes {
        let proto = vm.heap.alloc(Object::new(Some(base), Kind::Ordinary));
        vm.error_protos.insert(name, proto);
        let ctor = constructor(vm, name, 1, *func, proto);
        // Les constructeurs d'erreur héritent de `Error` (méthodes statiques).
        if let Some(Value::Object(error_ctor)) =
            vm.heap
                .get(vm.global)
                .props
                .get("Error")
                .map(|p| match &p.slot {
                    Slot::Data(v) => v.clone(),
                    Slot::Accessor { .. } => Value::Undefined,
                })
        {
            vm.heap.get_mut(ctor).proto = Some(error_ctor);
        }
        vm.define_hidden(proto, "name", Value::string(name));
    }
}

// ---- Boolean -------------------------------------------------------------------

fn boolean_ctor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let value = Value::Bool(arg(args, 0).truthy());
    if vm.constructing {
        vm.to_object(&value)
    } else {
        Ok(value)
    }
}

/// Valeur primitive de `this` s'il est du type attendu (ou son enveloppe).
pub(crate) fn this_primitive(
    vm: &mut Vm,
    this: &Value,
    matches: fn(&Value) -> bool,
    what: &str,
) -> JsResult {
    if matches(this) {
        return Ok(this.clone());
    }
    if let Some(Kind::Primitive(inner)) = this.as_object().map(|id| &vm.heap.get(id).kind)
        && matches(inner)
    {
        return Ok(inner.clone());
    }
    Err(vm.type_error(&format!("{what} : receveur incompatible")))
}

fn boolean_value_of(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    this_primitive(
        vm,
        this,
        |v| matches!(v, Value::Bool(_)),
        "Boolean.prototype.valueOf",
    )
}

fn boolean_to_string(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let value = this_primitive(
        vm,
        this,
        |v| matches!(v, Value::Bool(_)),
        "Boolean.prototype.toString",
    )?;
    Ok(Value::string(if value.truthy() { "true" } else { "false" }))
}

fn install_boolean(vm: &mut Vm) {
    let proto = vm.intrinsics.boolean_proto;
    constructor(vm, "Boolean", 1, boolean_ctor, proto);
    vm.define_method(proto, "valueOf", 0, boolean_value_of);
    vm.define_method(proto, "toString", 0, boolean_to_string);
}

// ---- Fonctions et objets globaux -------------------------------------------------

fn global_is_nan(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    Ok(Value::Bool(vm.to_number(&arg(args, 0))?.is_nan()))
}

fn global_is_finite(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    Ok(Value::Bool(vm.to_number(&arg(args, 0))?.is_finite()))
}

pub(crate) fn global_parse_float(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let text = vm.to_string(&arg(args, 0))?;
    let text = text.trim_start();
    // Plus long préfixe qui forme un littéral décimal.
    let mut end = 0;
    let mut seen_digit = false;
    let mut seen_dot = false;
    let mut seen_exp = false;
    let bytes = text.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        let ok = match b {
            b'0'..=b'9' => {
                seen_digit = true;
                true
            }
            b'+' | b'-' => i == 0 || matches!(bytes.get(i - 1), Some(b'e' | b'E')),
            b'.' => {
                !seen_dot && !seen_exp && {
                    seen_dot = true;
                    true
                }
            }
            b'e' | b'E' => {
                seen_digit && !seen_exp && {
                    seen_exp = true;
                    true
                }
            }
            _ => false,
        };
        if !ok {
            break;
        }
        end = i + 1;
    }
    let mut prefix = text.get(..end).unwrap_or("");
    for infinity in ["Infinity", "+Infinity", "-Infinity"] {
        if text.starts_with(infinity) {
            return Ok(Value::Number(if infinity.starts_with('-') {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            }));
        }
    }
    // Un préfixe comme « 1e » ou « 1e+ » se réduit à sa partie valide.
    while !prefix.is_empty() {
        if let Ok(n) = prefix.parse::<f64>()
            && seen_digit
        {
            return Ok(Value::Number(n));
        }
        prefix = prefix.get(..prefix.len() - 1).unwrap_or("");
    }
    Ok(Value::Number(f64::NAN))
}

pub(crate) fn global_parse_int(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let text = vm.to_string(&arg(args, 0))?;
    let mut text = text.trim_start();
    let negative = text.starts_with('-');
    if text.starts_with(['+', '-']) {
        text = text.get(1..).unwrap_or("");
    }
    let radix = crate::value::to_int32(vm.to_number(&arg(args, 1))?);
    let mut radix = if radix == 0 { 10 } else { radix };
    let is_hex = text.starts_with("0x") || text.starts_with("0X");
    if is_hex
        && (radix == 16
            || arg(args, 1) == Value::Undefined
            || radix == 10 && vm.to_number(&arg(args, 1))? == 0.0)
    {
        text = text.get(2..).unwrap_or("");
        radix = 16;
    }
    let Ok(radix) = u32::try_from(radix) else {
        return Ok(Value::Number(f64::NAN));
    };
    if !(2..=36).contains(&radix) {
        return Ok(Value::Number(f64::NAN));
    }
    let mut value = 0.0_f64;
    let mut digits = 0;
    for c in text.chars() {
        let Some(d) = c.to_digit(radix) else { break };
        value = value * f64::from(radix) + f64::from(d);
        digits += 1;
    }
    if digits == 0 {
        return Ok(Value::Number(f64::NAN));
    }
    Ok(Value::Number(if negative { -value } else { value }))
}

/// Caractères laissés tels quels par `encodeURIComponent`.
fn is_unreserved(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte)
}

fn global_encode_uri_component(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let text = vm.to_string(&arg(args, 0))?;
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if is_unreserved(byte) {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    Ok(Value::string(&out))
}

fn global_encode_uri(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let text = vm.to_string(&arg(args, 0))?;
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if is_unreserved(byte) || b";/?:@&=+$,#".contains(&byte) {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    Ok(Value::string(&out))
}

/// `escape` (annexe B) : `%XX` pour une unité de code sous 256, `%uXXXX`
/// au-delà ; lettres, chiffres et `@*_+-./` sont conservés.
fn global_escape(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let text = vm.to_string(&arg(args, 0))?;
    let mut out = String::with_capacity(text.len());
    for unit in text.encode_utf16() {
        match u8::try_from(unit) {
            Ok(byte) if byte.is_ascii_alphanumeric() || b"@*_+-./".contains(&byte) => {
                out.push(char::from(byte));
            }
            Ok(byte) => out.push_str(&format!("%{byte:02X}")),
            Err(_) => out.push_str(&format!("%u{unit:04X}")),
        }
    }
    Ok(Value::string(&out))
}

/// `unescape` (annexe B) : inverse de `escape` ; une séquence mal formée est
/// laissée telle quelle.
fn global_unescape(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let text = vm.to_string(&arg(args, 0))?;
    let units: Vec<u16> = text.encode_utf16().collect();
    let hex = |slice: &[u16]| -> Option<u16> {
        let mut value = 0_u16;
        for unit in slice {
            let digit = char::from_u32(u32::from(*unit))?.to_digit(16)?;
            value = value * 16 + u16::try_from(digit).ok()?;
        }
        Some(value)
    };
    let mut out: Vec<u16> = Vec::with_capacity(units.len());
    let mut index = 0;
    while let Some(unit) = units.get(index) {
        if *unit == u16::from(b'%') {
            let long = (units.get(index + 1) == Some(&u16::from(b'u')))
                .then(|| units.get(index + 2..index + 6))
                .flatten()
                .and_then(hex);
            if let Some(code) = long {
                out.push(code);
                index += 6;
                continue;
            }
            if let Some(code) = units.get(index + 1..index + 3).and_then(hex) {
                out.push(code);
                index += 3;
                continue;
            }
        }
        out.push(*unit);
        index += 1;
    }
    // LIMITE : un demi-substitut isolé devient U+FFFD (chaînes en UTF-8).
    Ok(Value::string(&String::from_utf16_lossy(&out)))
}

fn global_decode_uri_component(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let text = vm.to_string(&arg(args, 0))?;
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while let Some(&byte) = bytes.get(i) {
        if byte == b'%' {
            let hex = bytes
                .get(i + 1..i + 3)
                .and_then(|h| std::str::from_utf8(h).ok());
            match hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                Some(decoded) => {
                    out.push(decoded);
                    i += 3;
                    continue;
                }
                None => return Err(vm.error("URIError", "séquence d'échappement d'URI mal formée")),
            }
        }
        out.push(byte);
        i += 1;
    }
    match String::from_utf8(out) {
        Ok(decoded) => Ok(Value::string(&decoded)),
        Err(_) => Err(vm.error("URIError", "séquence d'échappement d'URI mal formée")),
    }
}

/// Messages écrits par `console.*`, à vider par l'hôte.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConsoleMessage {
    /// Niveau : `"log"`, `"info"`, `"warn"`, `"error"` ou `"debug"`.
    pub level: &'static str,
    /// Texte.
    pub text: String,
}

/// Nombre maximal de messages de console gardés en attente.
const MAX_CONSOLE_MESSAGES: usize = 10_000;

/// Ajoute un message à la console (au plus [`MAX_CONSOLE_MESSAGES`] en attente).
pub(crate) fn console_message(vm: &mut Vm, level: &'static str, text: String) {
    if vm.console.len() < MAX_CONSOLE_MESSAGES {
        vm.console.push(ConsoleMessage { level, text });
    }
}

fn console_write(vm: &mut Vm, level: &'static str, args: &[Value]) -> JsResult {
    let parts: Vec<String> = args.iter().map(|a| vm.display(a)).collect();
    console_message(vm, level, parts.join(" "));
    Ok(Value::Undefined)
}

macro_rules! console_fn {
    ($name:ident, $level:literal) => {
        fn $name(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
            console_write(vm, $level, args)
        }
    };
}
console_fn!(console_log, "log");
console_fn!(console_info, "info");
console_fn!(console_warn, "warn");
console_fn!(console_error, "error");
console_fn!(console_debug, "debug");

fn install_globals(vm: &mut Vm) {
    let global = vm.global;
    vm.define_hidden(global, "globalThis", Value::Object(global));
    vm.define_constant(global, "NaN", Value::Number(f64::NAN));
    vm.define_constant(global, "Infinity", Value::Number(f64::INFINITY));
    vm.define_constant(global, "undefined", Value::Undefined);
    let functions: &[(&str, u32, NativeFn)] = &[
        ("eval", 1, global_eval),
        ("isNaN", 1, global_is_nan),
        ("isFinite", 1, global_is_finite),
        ("parseFloat", 1, global_parse_float),
        ("parseInt", 2, global_parse_int),
        ("encodeURIComponent", 1, global_encode_uri_component),
        ("encodeURI", 1, global_encode_uri),
        ("decodeURIComponent", 1, global_decode_uri_component),
        ("decodeURI", 1, global_decode_uri_component),
        ("escape", 1, global_escape),
        ("unescape", 1, global_unescape),
    ];
    for (name, arity, func) in functions {
        vm.define_method(global, name, *arity, *func);
    }
    if let Ok(Value::Object(eval)) = vm.get(&Value::Object(global), "eval") {
        vm.eval_function = Some(eval);
    }
    let console = vm.new_object();
    let levels: &[(&str, NativeFn)] = &[
        ("log", console_log),
        ("info", console_info),
        ("warn", console_warn),
        ("error", console_error),
        ("debug", console_debug),
        ("trace", console_debug),
    ];
    for (name, func) in levels {
        vm.define_method(console, name, 0, *func);
    }
    vm.define_hidden(global, "console", Value::Object(console));
}

/// Représentation d'un nombre dans la base `radix` (2 à 36), partie entière seulement
/// exacte ; la partie fractionnaire est rendue sur 20 chiffres au plus.
pub(crate) fn number_to_radix(n: f64, radix: u32) -> String {
    if radix == 10 || !n.is_finite() {
        return number_to_string(n);
    }
    let negative = n < 0.0;
    let mut integer = n.abs().trunc();
    let mut fraction = n.abs() - integer;
    let digit = |d: u32| char::from_digit(d, radix).unwrap_or('0');
    let mut int_digits = Vec::new();
    if integer == 0.0 {
        int_digits.push('0');
    }
    while integer > 0.0 && int_digits.len() < 1100 {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "reste dans [0, radix)"
        )]
        int_digits.push(digit((integer % f64::from(radix)) as u32));
        integer = (integer / f64::from(radix)).trunc();
    }
    let mut out: String = int_digits.into_iter().rev().collect();
    if fraction > 0.0 {
        out.push('.');
        for _ in 0..20 {
            fraction *= f64::from(radix);
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "partie entière dans [0, radix)"
            )]
            out.push(digit(fraction.trunc() as u32));
            fraction -= fraction.trunc();
            if fraction == 0.0 {
                break;
            }
        }
    }
    if negative { format!("-{out}") } else { out }
}
