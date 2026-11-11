//! `Reflect`.
//!
//! LIMITES CONNUES : `Reflect.set` ignore son quatrième argument (receveur) et
//! renvoie toujours `true` ; `Reflect.defineProperty` renvoie `false` pour
//! tout refus, sans distinguer les causes.

use std::sync::Arc;

use super::{arg, define_from_descriptor, own_property_descriptor};
use crate::heap::Kind;
use crate::value::{ObjId, Value};
use crate::vm::{JsResult, NativeFn, Vm};

fn target(vm: &mut Vm, args: &[Value], method: &str) -> JsResult<ObjId> {
    match arg(args, 0).as_object() {
        Some(id) => Ok(id),
        None => Err(vm.type_error(&format!("Reflect.{method} : la cible doit être un objet"))),
    }
}

/// `CreateListFromArrayLike` : éléments `0..length` d'un objet.
pub(super) fn list_from_array_like(vm: &mut Vm, value: &Value) -> JsResult<Vec<Value>> {
    /// Borne la mémoire qu'une « liste » de longueur déclarée peut réserver.
    const MAX_LIST: f64 = 1_000_000.0;
    if value.as_object().is_none() {
        return Err(vm.type_error("une liste d'arguments (objet de type tableau) est attendue"));
    }
    let length = vm.get(value, "length")?;
    let length = vm.to_number(&length)?;
    let length = if length.is_nan() {
        0.0
    } else {
        length.trunc().max(0.0)
    };
    if length > MAX_LIST {
        return Err(vm.error("RangeError", "liste d'arguments trop longue"));
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "entier positif borné juste au-dessus"
    )]
    let length = length as usize;
    let mut list = Vec::with_capacity(length);
    for index in 0..length {
        list.push(vm.get(value, &index.to_string())?);
    }
    Ok(list)
}

fn reflect_apply(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let function = super::callable(vm, &arg(args, 0))?;
    let list = list_from_array_like(vm, &arg(args, 2))?;
    vm.call(&function, &arg(args, 1), &list)
}

fn reflect_construct(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let constructor = arg(args, 0);
    let new_target = args.get(2).cloned().unwrap_or_else(|| constructor.clone());
    if !vm.is_constructor(&constructor) || !vm.is_constructor(&new_target) {
        return Err(vm.type_error("Reflect.construct : un constructeur est attendu"));
    }
    let list = list_from_array_like(vm, &arg(args, 1))?;
    vm.construct(&constructor, &list, &new_target)
}

fn reflect_define_property(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    target(vm, args, "defineProperty")?;
    let key = vm.to_key(&arg(args, 1))?;
    let descriptor = arg(args, 2);
    if descriptor.as_object().is_none() {
        return Err(vm.type_error("Reflect.defineProperty : le descripteur doit être un objet"));
    }
    Ok(Value::Bool(
        define_from_descriptor(vm, &arg(args, 0), &key, &descriptor).is_ok(),
    ))
}

fn reflect_delete_property(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    target(vm, args, "deleteProperty")?;
    let key = vm.to_key(&arg(args, 1))?;
    vm.delete_key(&arg(args, 0), &key).map(Value::Bool)
}

fn reflect_get(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let id = target(vm, args, "get")?;
    let key = vm.to_key(&arg(args, 1))?;
    let receiver = args.get(2).cloned().unwrap_or_else(|| arg(args, 0));
    vm.get_key_with_receiver(id, &key, &receiver)
}

fn reflect_set(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    target(vm, args, "set")?;
    let key = vm.to_key(&arg(args, 1))?;
    vm.set_key(&arg(args, 0), &key, arg(args, 2))?;
    Ok(Value::Bool(true))
}

fn reflect_has(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let id = target(vm, args, "has")?;
    let key = vm.to_key(&arg(args, 1))?;
    Ok(Value::Bool(vm.has_key(id, &key)))
}

fn reflect_get_own_property_descriptor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let id = target(vm, args, "getOwnPropertyDescriptor")?;
    let key = vm.to_key(&arg(args, 1))?;
    own_property_descriptor(vm, id, &key)
}

fn reflect_get_prototype_of(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let id = target(vm, args, "getPrototypeOf")?;
    Ok(vm.heap.get(id).proto.map_or(Value::Null, Value::Object))
}

fn reflect_set_prototype_of(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let id = target(vm, args, "setPrototypeOf")?;
    let proto = match arg(args, 1) {
        Value::Null => None,
        Value::Object(proto) => Some(proto),
        _ => {
            return Err(
                vm.type_error("Reflect.setPrototypeOf : le prototype doit être un objet ou null")
            );
        }
    };
    if vm.heap.get(id).proto == proto {
        return Ok(Value::Bool(true));
    }
    if !vm.heap.get(id).extensible {
        return Ok(Value::Bool(false));
    }
    Ok(Value::Bool(vm.set_prototype(id, proto).is_ok()))
}

fn reflect_is_extensible(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let id = target(vm, args, "isExtensible")?;
    Ok(Value::Bool(vm.heap.get(id).extensible))
}

fn reflect_prevent_extensions(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let id = target(vm, args, "preventExtensions")?;
    vm.heap.get_mut(id).extensible = false;
    Ok(Value::Bool(true))
}

fn reflect_own_keys(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let id = target(vm, args, "ownKeys")?;
    let mut keys: Vec<Value> = vm
        .own_keys(id, false)
        .into_iter()
        .map(Value::String)
        .collect();
    if matches!(vm.heap.get(id).kind, Kind::Array(_)) {
        keys.push(Value::String(Arc::from("length")));
    }
    keys.extend(vm.own_symbols(id, false).into_iter().map(Value::Symbol));
    Ok(vm.new_array(keys))
}

pub(super) fn install(vm: &mut Vm) {
    let reflect = vm.new_object();
    let methods: &[(&str, u32, NativeFn)] = &[
        ("apply", 3, reflect_apply),
        ("construct", 2, reflect_construct),
        ("defineProperty", 3, reflect_define_property),
        ("deleteProperty", 2, reflect_delete_property),
        ("get", 2, reflect_get),
        ("set", 3, reflect_set),
        ("has", 2, reflect_has),
        (
            "getOwnPropertyDescriptor",
            2,
            reflect_get_own_property_descriptor,
        ),
        ("getPrototypeOf", 1, reflect_get_prototype_of),
        ("setPrototypeOf", 2, reflect_set_prototype_of),
        ("isExtensible", 1, reflect_is_extensible),
        ("preventExtensions", 1, reflect_prevent_extensions),
        ("ownKeys", 1, reflect_own_keys),
    ];
    for (name, arity, func) in methods {
        vm.define_method(reflect, name, *arity, *func);
    }
    let tag = vm.well_known.to_string_tag.clone();
    vm.define_hidden_symbol(reflect, &tag, Value::string("Reflect"));
    vm.define_hidden(vm.global, "Reflect", Value::Object(reflect));
}
