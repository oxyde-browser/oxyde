//! Objets générateurs : `next`, `return`, `throw`.
//!
//! LIMITES CONNUES : `return()` sur un générateur suspendu n'exécute pas ses
//! blocs `finally` ; `yield*` ne transmet au générateur délégué ni les valeurs
//! envoyées par `next(v)` ni `throw()`, et vaut `undefined` ; tous les
//! générateurs partagent un même prototype (`f.prototype` n'est pas consulté) ;
//! pas de générateurs async.

use super::arg;
use crate::value::Value;
use crate::vm::{CoOutcome, JsResult, NativeFn, Resume, Vm};

fn resume(vm: &mut Vm, this: &Value, mode: Resume) -> JsResult {
    let Some(generator) = vm.generator_id(this) else {
        return Err(vm.type_error("le receveur n'est pas un générateur"));
    };
    Ok(match vm.resume(generator, mode)? {
        CoOutcome::Yield(value) => vm.iter_result(value, false),
        CoOutcome::Return(value) => vm.iter_result(value, true),
    })
}

fn generator_next(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    resume(vm, this, Resume::Next(arg(args, 0)))
}

fn generator_return(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    resume(vm, this, Resume::Return(arg(args, 0)))
}

fn generator_throw(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    resume(vm, this, Resume::Throw(arg(args, 0)))
}

pub(super) fn install(vm: &mut Vm) {
    let proto = vm.intrinsics.generator_proto;
    // Un générateur est un itérateur : il hérite de `%IteratorPrototype%`
    // (et donc de `[Symbol.iterator]() { return this; }`).
    vm.heap.get_mut(proto).proto = Some(vm.intrinsics.iterator_proto);
    let methods: &[(&str, u32, NativeFn)] = &[
        ("next", 1, generator_next),
        ("return", 1, generator_return),
        ("throw", 1, generator_throw),
    ];
    for (name, arity, func) in methods {
        vm.define_method(proto, name, *arity, *func);
    }
    let tag = vm.well_known.to_string_tag.clone();
    vm.define_hidden_symbol(proto, &tag, Value::string("Generator"));
}
