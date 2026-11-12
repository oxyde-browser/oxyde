//! `ShadowRealm` (proposition « ShadowRealm ») : un royaume à part, où du
//! code s'évalue sans partager d'objets avec le royaume appelant. Seules
//! passent la frontière les valeurs primitives et les fonctions, ces
//! dernières enveloppées : une fonction enveloppée traduit ses arguments
//! vers le royaume de sa cible, et son résultat dans l'autre sens.
//!
//! LIMITES CONNUES : un royaume créé ainsi vit autant que la machine ; les
//! `var` de premier niveau du code évalué deviennent des propriétés de
//! l'objet global du royaume ; `importValue` charge le module sans attendre
//! et règle sa promesse aussitôt.

use std::sync::Arc;

use super::{arg, constructor, is_callable};
use crate::heap::{Kind, Object, Property, Slot};
use crate::value::{PropertyKey, Value};
use crate::vm::{JsResult, NativeFn, Vm};
use crate::{compiler, parser};

/// Emplacement interne d'un objet `ShadowRealm` : le numéro de son royaume.
const REALM_SLOT: &str = "shadowRealm";
/// Emplacement interne de l'objet global : `ShadowRealm.prototype` d'origine.
const PROTO_SLOT: &str = "shadowRealmPrototype";

/// Royaume de l'objet `ShadowRealm` désigné par `this`.
fn this_realm(vm: &mut Vm, this: &Value) -> JsResult<usize> {
    let realm = this
        .as_object()
        .and_then(|id| vm.slot(id, REALM_SLOT))
        .and_then(|value| match value {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "numéro de royaume, petit entier positif"
            )]
            Value::Number(realm) => Some(realm as usize),
            _ => None,
        });
    match realm {
        Some(realm) => Ok(realm),
        None => Err(vm.type_error("le receveur n'est pas un ShadowRealm")),
    }
}

/// `new ShadowRealm()`.
fn shadow_realm_ctor(vm: &mut Vm, _this: &Value, _args: &[Value]) -> JsResult {
    if !vm.constructing {
        return Err(vm.type_error("ShadowRealm doit être appelé avec « new »"));
    }
    let proto = vm
        .slot(vm.global, PROTO_SLOT)
        .and_then(|value| value.as_object());
    let Some(realm) = vm.create_realm() else {
        return Err(vm.error("RangeError", "trop de royaumes"));
    };
    let object = vm.heap.alloc(Object::new(proto, Kind::Ordinary));
    #[expect(
        clippy::cast_precision_loss,
        reason = "numéro de royaume, petit entier"
    )]
    vm.set_slot(object, REALM_SLOT, Value::Number(realm as f64));
    Ok(Value::Object(object))
}

/// `GetWrappedValue` : `value` telle qu'elle paraît dans le royaume `realm`.
/// Une primitive passe telle quelle, une fonction est enveloppée, tout autre
/// objet est refusé.
fn wrapped_value(vm: &mut Vm, realm: usize, value: Value) -> JsResult {
    if value.as_object().is_none() {
        return Ok(value);
    }
    if !is_callable(vm, &value) {
        return Err(vm.type_error(
            "seules les primitives et les fonctions passent la frontière d'un ShadowRealm",
        ));
    }
    // `CopyNameAndLength` : lus sur la cible, dont les erreurs sont masquées.
    let copied = (|| -> JsResult<(f64, Value)> {
        let mut length = 0.0;
        if let Some(id) = value.as_object() {
            let key = PropertyKey::String(Arc::from("length"));
            if vm.get_own_property(id, &key)?.is_some()
                && let Value::Number(target) = vm.get(&value, "length")?
            {
                length = if target == f64::INFINITY {
                    f64::INFINITY
                } else if target.is_nan() || target == f64::NEG_INFINITY {
                    0.0
                } else {
                    target.trunc().max(0.0)
                };
            }
        }
        let name = match vm.get(&value, "name")? {
            name @ Value::String(_) => name,
            _ => Value::string(""),
        };
        Ok((length, name))
    })();
    let Ok((length, name)) = copied else {
        return Err(
            vm.type_error("la fonction à envelopper refuse de donner son nom ou sa longueur")
        );
    };
    // La fonction enveloppée appartient au royaume qui la reçoit.
    let previous = vm.realm();
    vm.set_realm(realm);
    let wrapped = vm.new_native_data(wrapped_call, vec![value]);
    vm.set_realm(previous);
    if let Some(id) = wrapped.as_object() {
        let frozen = |value: Value| Property {
            slot: Slot::Data(value),
            writable: false,
            enumerable: false,
            configurable: true,
        };
        let props = &mut vm.heap.get_mut(id).props;
        props.insert(Arc::from("length"), frozen(Value::Number(length)));
        props.insert(Arc::from("name"), frozen(name));
    }
    Ok(wrapped)
}

/// Appel d'une fonction enveloppée : `data[0]` est la cible. Arguments et
/// receveur sont traduits vers le royaume de la cible, le résultat vers
/// celui de l'appelant ; toute exception devient une `TypeError`.
fn wrapped_call(vm: &mut Vm, data: &[Value], this: &Value, args: &[Value]) -> JsResult {
    let target = data.first().cloned().unwrap_or(Value::Undefined);
    let caller = vm.realm();
    let target_realm = usize::from(vm.function_realm(&target)?);
    let mut wrapped_args = Vec::with_capacity(args.len());
    for value in args {
        wrapped_args.push(wrapped_value(vm, target_realm, value.clone())?);
    }
    let wrapped_this = wrapped_value(vm, target_realm, this.clone())?;
    match vm.call(&target, &wrapped_this, &wrapped_args) {
        Ok(result) => wrapped_value(vm, caller, result),
        Err(_) => {
            Err(vm.type_error("la fonction enveloppée a levé une exception dans son royaume"))
        }
    }
}

/// `ShadowRealm.prototype.evaluate(source)`.
fn shadow_realm_evaluate(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let realm = this_realm(vm, this)?;
    let Value::String(source) = arg(args, 0) else {
        return Err(vm.type_error("ShadowRealm.prototype.evaluate : une chaîne est attendue"));
    };
    let compiled = parser::parse_eval(&source, false).and_then(|script| {
        compiler::compile_script(&script.body, script.strict, false, false, true)
    });
    let proto = match compiled {
        Ok(proto) => proto,
        Err(error) => return Err(vm.error("SyntaxError", &error.message)),
    };
    let caller = vm.realm();
    vm.set_realm(realm);
    let result = vm.run_script(proto);
    vm.set_realm(caller);
    match result {
        Ok(value) => wrapped_value(vm, caller, value),
        Err(_) => Err(vm.type_error("le code évalué dans le ShadowRealm a levé une exception")),
    }
}

/// `ShadowRealm.prototype.importValue(spécificateur, nom)` : promesse de la
/// valeur exportée sous `nom` par le module, chargé dans le royaume.
fn shadow_realm_import_value(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let realm = this_realm(vm, this)?;
    let specifier = vm.to_string(&arg(args, 0))?;
    let Value::String(export_name) = arg(args, 1) else {
        return Err(vm.type_error(
            "ShadowRealm.prototype.importValue : le nom d'export doit être une chaîne",
        ));
    };
    let caller = vm.realm();
    let promise = vm.new_promise();
    let referrer = Arc::clone(&vm.script_url);
    vm.set_realm(realm);
    let exported = vm
        .import_namespace(&specifier, &referrer)
        .and_then(|namespace| {
            let key = PropertyKey::String(Arc::clone(&export_name));
            match vm.get_own_property(namespace, &key)? {
                Some(_) => vm.get(&Value::Object(namespace), &export_name),
                None => Err(Value::Undefined),
            }
        });
    vm.set_realm(caller);
    let settled = exported.and_then(|value| wrapped_value(vm, caller, value));
    match settled {
        Ok(value) => vm.resolve_promise(promise, value),
        Err(_) => {
            let error = vm.type_error("ShadowRealm.prototype.importValue : importation impossible");
            vm.settle(promise, true, error);
        }
    }
    Ok(Value::Object(promise))
}

pub(super) fn install(vm: &mut Vm) {
    let proto = vm.new_object();
    constructor(vm, "ShadowRealm", 0, shadow_realm_ctor, proto);
    vm.set_slot(vm.global, PROTO_SLOT, Value::Object(proto));
    let methods: &[(&str, u32, NativeFn)] = &[
        ("evaluate", 1, shadow_realm_evaluate),
        ("importValue", 2, shadow_realm_import_value),
    ];
    for (name, arity, func) in methods {
        vm.define_method(proto, name, *arity, *func);
    }
    let tag = vm.well_known.to_string_tag.clone();
    vm.define_hidden_symbol(proto, &tag, Value::string("ShadowRealm"));
}
