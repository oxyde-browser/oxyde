//! `Reflect`.
//!
//! Chaque fonction applique la méthode interne de même nom (voir
//! [`crate::internal`]).

use super::{arg, own_property_descriptor};
use crate::internal::key_value;
use crate::value::{ObjId, Value};
use crate::vm::{JsResult, NativeFn, Vm};

fn target(vm: &mut Vm, args: &[Value], method: &str) -> JsResult<ObjId> {
    match arg(args, 0).as_object() {
        Some(id) => Ok(id),
        None => Err(vm.type_error(&format!("Reflect.{method} : la cible doit être un objet"))),
    }
}

/// `CreateListFromArrayLike` : éléments `0..length` d'un objet.
pub(crate) fn list_from_array_like(vm: &mut Vm, value: &Value) -> JsResult<Vec<Value>> {
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
    let id = target(vm, args, "defineProperty")?;
    let key = vm.to_key(&arg(args, 1))?;
    let descriptor = vm.to_descriptor(&arg(args, 2))?;
    vm.define_own_property(id, &key, descriptor)
        .map(Value::Bool)
}

fn reflect_delete_property(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let id = target(vm, args, "deleteProperty")?;
    let key = vm.to_key(&arg(args, 1))?;
    vm.delete_property(id, &key).map(Value::Bool)
}

fn reflect_get(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let id = target(vm, args, "get")?;
    let key = vm.to_key(&arg(args, 1))?;
    let receiver = args.get(2).cloned().unwrap_or_else(|| arg(args, 0));
    vm.get_key_with_receiver(id, &key, &receiver)
}

fn reflect_set(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let id = target(vm, args, "set")?;
    let key = vm.to_key(&arg(args, 1))?;
    let receiver = args.get(3).cloned().unwrap_or_else(|| arg(args, 0));
    vm.set_with_receiver(id, &key, arg(args, 2), &receiver)
        .map(Value::Bool)
}

fn reflect_has(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let id = target(vm, args, "has")?;
    let key = vm.to_key(&arg(args, 1))?;
    vm.has_property_op(id, &key).map(Value::Bool)
}

fn reflect_get_own_property_descriptor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let id = target(vm, args, "getOwnPropertyDescriptor")?;
    let key = vm.to_key(&arg(args, 1))?;
    own_property_descriptor(vm, id, &key)
}

fn reflect_get_prototype_of(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let id = target(vm, args, "getPrototypeOf")?;
    Ok(vm.get_prototype_of(id)?.map_or(Value::Null, Value::Object))
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
    vm.set_prototype_of(id, proto).map(Value::Bool)
}

fn reflect_is_extensible(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let id = target(vm, args, "isExtensible")?;
    vm.is_extensible(id).map(Value::Bool)
}

fn reflect_prevent_extensions(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let id = target(vm, args, "preventExtensions")?;
    vm.prevent_extensions(id).map(Value::Bool)
}

fn reflect_own_keys(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let id = target(vm, args, "ownKeys")?;
    let keys = vm.own_property_keys(id)?.iter().map(key_value).collect();
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
