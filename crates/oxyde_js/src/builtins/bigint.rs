//! `BigInt` : fonction de conversion, méthodes statiques et prototype.
//!
//! LIMITES CONNUES : un `BigInt` est borné à 2^22 bits (au-delà,
//! `RangeError`) ; `toLocaleString` vaut `toString`.

use std::sync::Arc;

use super::{arg, constructor};
use crate::bigint::BigInt;
use crate::heap::Kind;
use crate::value::Value;
use crate::vm::{JsResult, NativeFn, Vm};

/// `ToBigInt` : les nombres ne sont pas convertis implicitement.
pub(crate) fn to_bigint(vm: &mut Vm, value: &Value) -> JsResult<Arc<BigInt>> {
    match vm.to_primitive(value, false)? {
        Value::BigInt(big) => Ok(big),
        Value::Bool(b) => Ok(Arc::new(BigInt::from_u64(u64::from(b)))),
        Value::String(text) => match BigInt::parse_string(&text) {
            Some(big) => Ok(Arc::new(big)),
            None => Err(vm.error(
                "SyntaxError",
                &format!("« {text} » ne peut pas être converti en BigInt"),
            )),
        },
        other => {
            let shown = vm.display(&other);
            Err(vm.type_error(&format!("{shown} ne peut pas être converti en BigInt")))
        }
    }
}

fn too_big(vm: &mut Vm) -> Value {
    vm.error("RangeError", "BigInt trop grand")
}

fn bigint_fn(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    if vm.constructing {
        return Err(vm.type_error("BigInt n'est pas un constructeur"));
    }
    let primitive = vm.to_primitive(&arg(args, 0), false)?;
    if let Value::Number(n) = primitive {
        return match BigInt::from_f64(n) {
            Some(big) => Ok(Value::BigInt(Arc::new(big))),
            None => Err(vm.error(
                "RangeError",
                "seul un nombre entier peut être converti en BigInt",
            )),
        };
    }
    to_bigint(vm, &primitive).map(Value::BigInt)
}

/// `ToIndex` : nombre de bits demandé.
fn bit_count(vm: &mut Vm, value: &Value) -> JsResult<u64> {
    if *value == Value::Undefined {
        return Ok(0);
    }
    let n = vm.to_number(value)?;
    let n = if n.is_nan() { 0.0 } else { n.trunc() };
    if !(0.0..=9_007_199_254_740_991.0).contains(&n) {
        return Err(vm.error("RangeError", "nombre de bits invalide"));
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "entier vérifié dans [0, 2^53 - 1]"
    )]
    Ok(n as u64)
}

fn bigint_as_int_n(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let bits = bit_count(vm, &arg(args, 0))?;
    let big = to_bigint(vm, &arg(args, 1))?;
    match big.as_int_n(bits) {
        Ok(result) => Ok(Value::BigInt(Arc::new(result))),
        Err(_) => Err(too_big(vm)),
    }
}

fn bigint_as_uint_n(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let bits = bit_count(vm, &arg(args, 0))?;
    let big = to_bigint(vm, &arg(args, 1))?;
    match big.as_uint_n(bits) {
        Ok(result) => Ok(Value::BigInt(Arc::new(result))),
        Err(_) => Err(too_big(vm)),
    }
}

/// Valeur désignée par `this` : primitive ou objet enveloppe.
fn this_bigint(vm: &mut Vm, this: &Value) -> JsResult<Arc<BigInt>> {
    match this {
        Value::BigInt(big) => Ok(Arc::clone(big)),
        Value::Object(id) => match &vm.heap.get(*id).kind {
            Kind::Primitive(Value::BigInt(big)) => Ok(Arc::clone(big)),
            _ => Err(vm.type_error("un BigInt est attendu")),
        },
        _ => Err(vm.type_error("un BigInt est attendu")),
    }
}

fn bigint_to_string(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let big = this_bigint(vm, this)?;
    let radix = match arg(args, 0) {
        Value::Undefined => 10.0,
        other => {
            let n = vm.to_number(&other)?;
            if n.is_nan() { 0.0 } else { n.trunc() }
        }
    };
    if !(2.0..=36.0).contains(&radix) {
        return Err(vm.error("RangeError", "la base doit être comprise entre 2 et 36"));
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "base vérifiée dans [2, 36]"
    )]
    Ok(Value::string(&big.to_string_radix(radix as u32)))
}

fn bigint_to_locale_string(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let big = this_bigint(vm, this)?;
    Ok(Value::string(&big.to_string()))
}

fn bigint_value_of(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    this_bigint(vm, this).map(Value::BigInt)
}

pub(super) fn install(vm: &mut Vm) {
    let proto = vm.intrinsics.bigint_proto;
    let ctor = constructor(vm, "BigInt", 1, bigint_fn, proto);
    vm.define_method(ctor, "asIntN", 2, bigint_as_int_n);
    vm.define_method(ctor, "asUintN", 2, bigint_as_uint_n);
    let methods: &[(&str, u32, NativeFn)] = &[
        ("toString", 0, bigint_to_string),
        ("toLocaleString", 0, bigint_to_locale_string),
        ("valueOf", 0, bigint_value_of),
    ];
    for (name, arity, func) in methods {
        vm.define_method(proto, name, *arity, *func);
    }
    let tag = vm.well_known.to_string_tag.clone();
    vm.define_hidden_symbol(proto, &tag, Value::string("BigInt"));
}
