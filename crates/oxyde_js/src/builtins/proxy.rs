//! `Proxy` (ECMA-262 §10.5 et §28.2) : objet exotique dont chaque méthode
//! interne passe par une trappe du gestionnaire, puis vérifie les invariants
//! que la cible impose.

use super::{arg, is_callable};
use crate::heap::{Kind, Object, ProxyData};
use crate::internal::{Descriptor, key_value, same_value, validate_descriptor};
use crate::value::{ObjId, PropertyKey, Value};
use crate::vm::{JsResult, Vm};

impl Vm {
    /// Cible et gestionnaire du proxy `id` ; `TypeError` s'il est révoqué.
    fn proxy_parts(&mut self, id: ObjId) -> JsResult<(ObjId, ObjId)> {
        match &self.heap.get(id).kind {
            Kind::Proxy(ProxyData {
                target: Some(target),
                handler: Some(handler),
                ..
            }) => Ok((*target, *handler)),
            _ => Err(self.type_error("opération sur un proxy révoqué")),
        }
    }

    /// Trappe `name` du gestionnaire (`GetMethod`) : `None` si elle est absente.
    fn proxy_trap(&mut self, handler: ObjId, name: &str) -> JsResult<Option<Value>> {
        let trap = self.get(&Value::Object(handler), name)?;
        if trap.is_nullish() {
            return Ok(None);
        }
        if !is_callable(self, &trap) {
            return Err(self.type_error(&format!("la trappe « {name} » n'est pas une fonction")));
        }
        Ok(Some(trap))
    }

    fn invariant<T>(&mut self, trap: &str, message: &str) -> JsResult<T> {
        Err(self.type_error(&format!("trappe « {trap} » : {message}")))
    }

    pub(crate) fn proxy_get_prototype_of(&mut self, id: ObjId) -> JsResult<Option<ObjId>> {
        let (target, handler) = self.proxy_parts(id)?;
        let Some(trap) = self.proxy_trap(handler, "getPrototypeOf")? else {
            return self.get_prototype_of(target);
        };
        let result = self.call(&trap, &Value::Object(handler), &[Value::Object(target)])?;
        let proto = match result {
            Value::Object(proto) => Some(proto),
            Value::Null => None,
            _ => return self.invariant("getPrototypeOf", "un objet ou null est attendu"),
        };
        if self.is_extensible(target)? {
            return Ok(proto);
        }
        if self.get_prototype_of(target)? != proto {
            return self.invariant(
                "getPrototypeOf",
                "le prototype d'une cible non extensible ne peut pas différer",
            );
        }
        Ok(proto)
    }

    pub(crate) fn proxy_set_prototype_of(
        &mut self,
        id: ObjId,
        proto: Option<ObjId>,
    ) -> JsResult<bool> {
        let (target, handler) = self.proxy_parts(id)?;
        let Some(trap) = self.proxy_trap(handler, "setPrototypeOf")? else {
            return self.set_prototype_of(target, proto);
        };
        let proto_value = proto.map_or(Value::Null, Value::Object);
        let accepted = self
            .call(
                &trap,
                &Value::Object(handler),
                &[Value::Object(target), proto_value],
            )?
            .truthy();
        if !accepted {
            return Ok(false);
        }
        if self.is_extensible(target)? {
            return Ok(true);
        }
        if self.get_prototype_of(target)? != proto {
            return self.invariant(
                "setPrototypeOf",
                "le prototype d'une cible non extensible ne peut pas changer",
            );
        }
        Ok(true)
    }

    pub(crate) fn proxy_is_extensible(&mut self, id: ObjId) -> JsResult<bool> {
        let (target, handler) = self.proxy_parts(id)?;
        let Some(trap) = self.proxy_trap(handler, "isExtensible")? else {
            return self.is_extensible(target);
        };
        let result = self
            .call(&trap, &Value::Object(handler), &[Value::Object(target)])?
            .truthy();
        if result != self.is_extensible(target)? {
            return self.invariant("isExtensible", "le résultat doit être celui de la cible");
        }
        Ok(result)
    }

    pub(crate) fn proxy_prevent_extensions(&mut self, id: ObjId) -> JsResult<bool> {
        let (target, handler) = self.proxy_parts(id)?;
        let Some(trap) = self.proxy_trap(handler, "preventExtensions")? else {
            return self.prevent_extensions(target);
        };
        let result = self
            .call(&trap, &Value::Object(handler), &[Value::Object(target)])?
            .truthy();
        if result && self.is_extensible(target)? {
            return self.invariant("preventExtensions", "la cible est restée extensible");
        }
        Ok(result)
    }

    pub(crate) fn proxy_get_own_property(
        &mut self,
        id: ObjId,
        key: &PropertyKey,
    ) -> JsResult<Option<Descriptor>> {
        let (target, handler) = self.proxy_parts(id)?;
        let Some(trap) = self.proxy_trap(handler, "getOwnPropertyDescriptor")? else {
            return self.get_own_property(target, key);
        };
        let result = self.call(
            &trap,
            &Value::Object(handler),
            &[Value::Object(target), key_value(key)],
        )?;
        if result != Value::Undefined && result.as_object().is_none() {
            return self.invariant(
                "getOwnPropertyDescriptor",
                "un objet ou undefined est attendu",
            );
        }
        let target_desc = self.get_own_property(target, key)?;
        if result == Value::Undefined {
            let Some(target_desc) = target_desc else {
                return Ok(None);
            };
            if !target_desc.is_configurable() {
                return self.invariant(
                    "getOwnPropertyDescriptor",
                    "une propriété non configurable ne peut pas être cachée",
                );
            }
            if !self.is_extensible(target)? {
                return self.invariant(
                    "getOwnPropertyDescriptor",
                    "une propriété d'une cible non extensible ne peut pas être cachée",
                );
            }
            return Ok(None);
        }
        let extensible = self.is_extensible(target)?;
        let result_desc = self.to_descriptor(&result)?.completed();
        if validate_descriptor(target_desc.as_ref(), &result_desc, extensible).is_none() {
            return self.invariant(
                "getOwnPropertyDescriptor",
                "descripteur incompatible avec la cible",
            );
        }
        if !result_desc.is_configurable() {
            let Some(target_desc) = &target_desc else {
                return self.invariant(
                    "getOwnPropertyDescriptor",
                    "propriété non configurable absente de la cible",
                );
            };
            if target_desc.is_configurable() {
                return self.invariant(
                    "getOwnPropertyDescriptor",
                    "propriété configurable sur la cible",
                );
            }
            if result_desc.writable == Some(false) && target_desc.is_writable() {
                return self.invariant(
                    "getOwnPropertyDescriptor",
                    "propriété inscriptible sur la cible",
                );
            }
        }
        Ok(Some(result_desc))
    }

    pub(crate) fn proxy_define_own_property(
        &mut self,
        id: ObjId,
        key: &PropertyKey,
        desc: Descriptor,
    ) -> JsResult<bool> {
        let (target, handler) = self.proxy_parts(id)?;
        let Some(trap) = self.proxy_trap(handler, "defineProperty")? else {
            return self.define_own_property(target, key, desc);
        };
        let desc_object = self.from_descriptor(&desc);
        let accepted = self
            .call(
                &trap,
                &Value::Object(handler),
                &[Value::Object(target), key_value(key), desc_object],
            )?
            .truthy();
        if !accepted {
            return Ok(false);
        }
        let target_desc = self.get_own_property(target, key)?;
        let extensible = self.is_extensible(target)?;
        let setting_non_configurable = desc.configurable == Some(false);
        match &target_desc {
            None => {
                if !extensible {
                    return self.invariant("defineProperty", "la cible n'est pas extensible");
                }
                if setting_non_configurable {
                    return self.invariant(
                        "defineProperty",
                        "propriété non configurable absente de la cible",
                    );
                }
            }
            Some(target_desc) => {
                if validate_descriptor(Some(target_desc), &desc, extensible).is_none() {
                    return self
                        .invariant("defineProperty", "descripteur incompatible avec la cible");
                }
                if setting_non_configurable && target_desc.is_configurable() {
                    return self.invariant("defineProperty", "propriété configurable sur la cible");
                }
                if !target_desc.is_accessor()
                    && !target_desc.is_configurable()
                    && target_desc.is_writable()
                    && desc.writable == Some(false)
                {
                    return self.invariant("defineProperty", "propriété inscriptible sur la cible");
                }
            }
        }
        Ok(true)
    }

    pub(crate) fn proxy_has(&mut self, id: ObjId, key: &PropertyKey) -> JsResult<bool> {
        let (target, handler) = self.proxy_parts(id)?;
        let Some(trap) = self.proxy_trap(handler, "has")? else {
            return self.has_property_op(target, key);
        };
        let result = self
            .call(
                &trap,
                &Value::Object(handler),
                &[Value::Object(target), key_value(key)],
            )?
            .truthy();
        if !result && let Some(target_desc) = self.get_own_property(target, key)? {
            if !target_desc.is_configurable() {
                return self.invariant(
                    "has",
                    "une propriété non configurable ne peut pas être cachée",
                );
            }
            if !self.is_extensible(target)? {
                return self.invariant(
                    "has",
                    "une propriété d'une cible non extensible ne peut pas être cachée",
                );
            }
        }
        Ok(result)
    }

    pub(crate) fn proxy_get(&mut self, id: ObjId, key: &PropertyKey, receiver: &Value) -> JsResult {
        let (target, handler) = self.proxy_parts(id)?;
        let Some(trap) = self.proxy_trap(handler, "get")? else {
            return self.get_key_with_receiver(target, key, receiver);
        };
        let value = self.call(
            &trap,
            &Value::Object(handler),
            &[Value::Object(target), key_value(key), receiver.clone()],
        )?;
        if let Some(target_desc) = self.get_own_property(target, key)?
            && !target_desc.is_configurable()
        {
            if target_desc.is_accessor() {
                if target_desc.get == Some(Value::Undefined) && value != Value::Undefined {
                    return self
                        .invariant("get", "un accesseur sans lecture doit donner undefined");
                }
            } else if !target_desc.is_writable()
                && !same_value(
                    &value,
                    target_desc.value.as_ref().unwrap_or(&Value::Undefined),
                )
            {
                return self.invariant(
                    "get",
                    "la valeur d'une propriété figée ne peut pas différer",
                );
            }
        }
        Ok(value)
    }

    pub(crate) fn proxy_set(
        &mut self,
        id: ObjId,
        key: &PropertyKey,
        value: Value,
        receiver: &Value,
    ) -> JsResult<bool> {
        let (target, handler) = self.proxy_parts(id)?;
        let Some(trap) = self.proxy_trap(handler, "set")? else {
            return self.set_with_receiver(target, key, value, receiver);
        };
        let accepted = self
            .call(
                &trap,
                &Value::Object(handler),
                &[
                    Value::Object(target),
                    key_value(key),
                    value.clone(),
                    receiver.clone(),
                ],
            )?
            .truthy();
        if !accepted {
            return Ok(false);
        }
        if let Some(target_desc) = self.get_own_property(target, key)?
            && !target_desc.is_configurable()
        {
            if target_desc.is_accessor() {
                if target_desc.set == Some(Value::Undefined) {
                    return self.invariant("set", "la propriété n'a pas de mutateur");
                }
            } else if !target_desc.is_writable()
                && !same_value(
                    &value,
                    target_desc.value.as_ref().unwrap_or(&Value::Undefined),
                )
            {
                return self
                    .invariant("set", "la valeur d'une propriété figée ne peut pas changer");
            }
        }
        Ok(true)
    }

    pub(crate) fn proxy_delete(&mut self, id: ObjId, key: &PropertyKey) -> JsResult<bool> {
        let (target, handler) = self.proxy_parts(id)?;
        let Some(trap) = self.proxy_trap(handler, "deleteProperty")? else {
            return self.delete_property(target, key);
        };
        let accepted = self
            .call(
                &trap,
                &Value::Object(handler),
                &[Value::Object(target), key_value(key)],
            )?
            .truthy();
        if !accepted {
            return Ok(false);
        }
        let Some(target_desc) = self.get_own_property(target, key)? else {
            return Ok(true);
        };
        if !target_desc.is_configurable() {
            return self.invariant(
                "deleteProperty",
                "une propriété non configurable ne peut pas être supprimée",
            );
        }
        if !self.is_extensible(target)? {
            return self.invariant(
                "deleteProperty",
                "une propriété d'une cible non extensible ne peut pas être supprimée",
            );
        }
        Ok(true)
    }

    pub(crate) fn proxy_own_keys(&mut self, id: ObjId) -> JsResult<Vec<PropertyKey>> {
        let (target, handler) = self.proxy_parts(id)?;
        let Some(trap) = self.proxy_trap(handler, "ownKeys")? else {
            return self.own_property_keys(target);
        };
        let result = self.call(&trap, &Value::Object(handler), &[Value::Object(target)])?;
        let mut keys: Vec<PropertyKey> = Vec::new();
        for element in super::reflect::list_from_array_like(self, &result)? {
            let key = match element {
                Value::String(name) => PropertyKey::String(name),
                Value::Symbol(symbol) => PropertyKey::Symbol(symbol),
                _ => return self.invariant("ownKeys", "chaque clé est une chaîne ou un symbole"),
            };
            if keys.contains(&key) {
                return self.invariant("ownKeys", "clé en double");
            }
            keys.push(key);
        }
        let extensible = self.is_extensible(target)?;
        let mut configurable = Vec::new();
        let mut non_configurable = Vec::new();
        for key in self.own_property_keys(target)? {
            match self.get_own_property(target, &key)? {
                Some(desc) if !desc.is_configurable() => non_configurable.push(key),
                _ => configurable.push(key),
            }
        }
        if extensible && non_configurable.is_empty() {
            return Ok(keys);
        }
        let mut unchecked = keys.clone();
        for key in &non_configurable {
            let Some(position) = unchecked.iter().position(|k| k == key) else {
                return self.invariant("ownKeys", "une clé non configurable de la cible manque");
            };
            unchecked.remove(position);
        }
        if extensible {
            return Ok(keys);
        }
        for key in &configurable {
            let Some(position) = unchecked.iter().position(|k| k == key) else {
                return self.invariant("ownKeys", "une clé d'une cible non extensible manque");
            };
            unchecked.remove(position);
        }
        if !unchecked.is_empty() {
            return self.invariant("ownKeys", "clé absente d'une cible non extensible");
        }
        Ok(keys)
    }

    /// `[[Call]]` d'un proxy.
    pub(crate) fn proxy_call(&mut self, id: ObjId, this: &Value, args: &[Value]) -> JsResult {
        let (target, handler) = self.proxy_parts(id)?;
        let Some(trap) = self.proxy_trap(handler, "apply")? else {
            return self.call(&Value::Object(target), this, args);
        };
        let list = self.new_array(args.to_vec());
        self.call(
            &trap,
            &Value::Object(handler),
            &[Value::Object(target), this.clone(), list],
        )
    }

    /// `[[Construct]]` d'un proxy.
    pub(crate) fn proxy_construct(
        &mut self,
        id: ObjId,
        args: &[Value],
        new_target: &Value,
    ) -> JsResult {
        let (target, handler) = self.proxy_parts(id)?;
        let Some(trap) = self.proxy_trap(handler, "construct")? else {
            return self.construct(&Value::Object(target), args, new_target);
        };
        let list = self.new_array(args.to_vec());
        let result = self.call(
            &trap,
            &Value::Object(handler),
            &[Value::Object(target), list, new_target.clone()],
        )?;
        if result.as_object().is_none() {
            return self.invariant("construct", "un objet est attendu");
        }
        Ok(result)
    }

    /// `ProxyCreate` (ECMA-262 §10.5.14).
    fn proxy_create(&mut self, target: &Value, handler: &Value) -> JsResult<ObjId> {
        let (Some(target_id), Some(handler_id)) = (target.as_object(), handler.as_object()) else {
            return Err(
                self.type_error("Proxy : la cible et le gestionnaire doivent être des objets")
            );
        };
        let data = ProxyData {
            target: Some(target_id),
            handler: Some(handler_id),
            callable: is_callable(self, target),
            constructor: self.is_constructor(target),
        };
        Ok(self.heap.alloc(Object::new(None, Kind::Proxy(data))))
    }
}

fn proxy_ctor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    if !vm.constructing {
        return Err(vm.type_error("Proxy doit être appelé avec « new »"));
    }
    vm.proxy_create(&arg(args, 0), &arg(args, 1))
        .map(Value::Object)
}

/// Fonction de révocation : `data[0]` est le proxy, révoqué au premier appel.
fn proxy_revoke(vm: &mut Vm, data: &[Value], _this: &Value, _args: &[Value]) -> JsResult {
    if let Some(id) = data.first().and_then(Value::as_object)
        && let Kind::Proxy(proxy) = &mut vm.heap.get_mut(id).kind
    {
        proxy.target = None;
        proxy.handler = None;
    }
    Ok(Value::Undefined)
}

fn proxy_revocable(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let proxy = Value::Object(vm.proxy_create(&arg(args, 0), &arg(args, 1))?);
    let revoke = vm.new_native_data(proxy_revoke, vec![proxy.clone()]);
    if let Some(id) = revoke.as_object() {
        vm.define_hidden(id, "length", Value::Number(0.0));
        vm.define_hidden(id, "name", Value::string(""));
        for name in ["length", "name"] {
            if let Some(property) = vm.heap.get_mut(id).props.get_mut(name) {
                property.writable = false;
            }
        }
    }
    let result = Value::Object(vm.new_object());
    vm.set(&result, "proxy", proxy)?;
    vm.set(&result, "revoke", revoke)?;
    Ok(result)
}

pub(super) fn install(vm: &mut Vm) {
    let ctor = vm.new_native("Proxy", 2, proxy_ctor, true);
    vm.define_method(ctor, "revocable", 2, proxy_revocable);
    vm.define_hidden(vm.global, "Proxy", Value::Object(ctor));
}
