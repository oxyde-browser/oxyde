//! Méthodes internes essentielles des objets (ECMA-262 §6.1.7.2 et §10) :
//! `[[GetOwnProperty]]`, `[[DefineOwnProperty]]`, `[[HasProperty]]`, `[[Set]]`,
//! `[[Delete]]`, `[[OwnPropertyKeys]]`, `[[GetPrototypeOf]]`,
//! `[[SetPrototypeOf]]`, `[[IsExtensible]]` et `[[PreventExtensions]]`.
//!
//! Chaque opération délègue aux trappes d'un `Proxy` (voir
//! [`crate::builtins::proxy`]) ou applique le comportement ordinaire, adapté
//! aux objets exotiques du moteur.
//!
//! Représentation des tableaux : les éléments ordinaires (inscriptibles,
//! énumérables, configurables) vivent dans le vecteur dense. Un élément aux
//! attributs particuliers, ou un accesseur, vit dans les propriétés nommées
//! sous son index, l'emplacement dense restant un trou. Une longueur figée
//! est marquée par une propriété nommée `length` non inscriptible.
//!
//! LIMITE CONNUE : les méthodes de `Array.prototype` qui modifient le vecteur
//! dense directement (`push`, `splice`…) ignorent une longueur figée.

use std::sync::Arc;

use crate::builtins::is_callable;
use crate::heap::{Kind, Property, Slot};
use crate::object::{MAX_ARRAY_GAP, char_at};
use crate::value::{
    JsStr, ObjId, PropertyKey, Value, array_index, canonical_numeric_index, to_uint32, utf16_len,
};
use crate::vm::{JsResult, Vm};

/// Bit de [`crate::heap::Object::deleted_virtual`] : `name` d'une fonction supprimé.
pub(crate) const DELETED_NAME: u8 = 1;
/// Bit de [`crate::heap::Object::deleted_virtual`] : `length` d'une fonction supprimé.
pub(crate) const DELETED_LENGTH: u8 = 2;

/// Descripteur de propriété (ECMA-262 §6.2.6). Un champ absent vaut `None`.
#[derive(Clone, Debug, Default)]
pub(crate) struct Descriptor {
    pub(crate) value: Option<Value>,
    pub(crate) writable: Option<bool>,
    pub(crate) get: Option<Value>,
    pub(crate) set: Option<Value>,
    pub(crate) enumerable: Option<bool>,
    pub(crate) configurable: Option<bool>,
}

impl Descriptor {
    /// Descripteur de données complet.
    pub(crate) fn data(value: Value, writable: bool, enumerable: bool, configurable: bool) -> Self {
        Self {
            value: Some(value),
            writable: Some(writable),
            get: None,
            set: None,
            enumerable: Some(enumerable),
            configurable: Some(configurable),
        }
    }

    /// Propriété de données créée par affectation.
    pub(crate) fn plain(value: Value) -> Self {
        Self::data(value, true, true, true)
    }

    /// Descripteur qui ne porte qu'une valeur.
    pub(crate) fn value_only(value: Value) -> Self {
        Self {
            value: Some(value),
            ..Self::default()
        }
    }

    pub(crate) fn is_accessor(&self) -> bool {
        self.get.is_some() || self.set.is_some()
    }

    pub(crate) fn is_data(&self) -> bool {
        self.value.is_some() || self.writable.is_some()
    }

    pub(crate) fn is_enumerable(&self) -> bool {
        self.enumerable == Some(true)
    }

    pub(crate) fn is_configurable(&self) -> bool {
        self.configurable == Some(true)
    }

    pub(crate) fn is_writable(&self) -> bool {
        self.writable == Some(true)
    }

    fn from_property(property: &Property) -> Self {
        match &property.slot {
            Slot::Data(value) => Self::data(
                value.clone(),
                property.writable,
                property.enumerable,
                property.configurable,
            ),
            Slot::Accessor { get, set } => Self {
                value: None,
                writable: None,
                get: Some(get.map_or(Value::Undefined, Value::Object)),
                set: Some(set.map_or(Value::Undefined, Value::Object)),
                enumerable: Some(property.enumerable),
                configurable: Some(property.configurable),
            },
        }
    }

    /// Propriété stockée ; les champs absents prennent leur valeur par défaut.
    fn into_property(self) -> Property {
        let enumerable = self.enumerable.unwrap_or(false);
        let configurable = self.configurable.unwrap_or(false);
        if self.is_accessor() {
            Property {
                slot: Slot::Accessor {
                    get: self.get.as_ref().and_then(Value::as_object),
                    set: self.set.as_ref().and_then(Value::as_object),
                },
                writable: true,
                enumerable,
                configurable,
            }
        } else {
            Property {
                slot: Slot::Data(self.value.unwrap_or(Value::Undefined)),
                writable: self.writable.unwrap_or(false),
                enumerable,
                configurable,
            }
        }
    }

    /// Complète les champs absents par leur valeur par défaut.
    pub(crate) fn completed(mut self) -> Self {
        if self.is_accessor() {
            self.get.get_or_insert(Value::Undefined);
            self.set.get_or_insert(Value::Undefined);
        } else {
            self.value.get_or_insert(Value::Undefined);
            self.writable.get_or_insert(false);
        }
        self.enumerable.get_or_insert(false);
        self.configurable.get_or_insert(false);
        self
    }
}

/// `SameValue` (ECMA-262 §7.2.10) : comme `===`, mais `NaN` est égal à
/// lui-même et `+0` diffère de `-0`.
pub(crate) fn same_value(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            (x.is_nan() && y.is_nan()) || (x == y && x.is_sign_negative() == y.is_sign_negative())
        }
        _ => Vm::strict_equals(a, b),
    }
}

/// Valeur JavaScript d'une clé de propriété.
pub(crate) fn key_value(key: &PropertyKey) -> Value {
    match key {
        PropertyKey::String(s) => Value::String(Arc::clone(s)),
        PropertyKey::Symbol(s) => Value::Symbol(s.clone()),
    }
}

/// `ValidateAndApplyPropertyDescriptor` (ECMA-262 §10.1.6.3), sans
/// l'application : rend la propriété à stocker, ou `None` si la définition
/// est refusée. `current` est complet.
pub(crate) fn validate_descriptor(
    current: Option<&Descriptor>,
    desc: &Descriptor,
    extensible: bool,
) -> Option<Property> {
    let Some(current) = current else {
        return extensible.then(|| desc.clone().into_property());
    };
    let differs = |wanted: &Option<Value>, present: &Option<Value>| {
        wanted.as_ref().is_some_and(|wanted| {
            !same_value(wanted, present.as_ref().unwrap_or(&Value::Undefined))
        })
    };
    let generic = !desc.is_accessor() && !desc.is_data();
    if !current.is_configurable() {
        if desc.configurable == Some(true) {
            return None;
        }
        if desc
            .enumerable
            .is_some_and(|e| e != current.is_enumerable())
        {
            return None;
        }
        if !generic && desc.is_accessor() != current.is_accessor() {
            return None;
        }
        if current.is_accessor() {
            if differs(&desc.get, &current.get) || differs(&desc.set, &current.set) {
                return None;
            }
        } else if !current.is_writable()
            && (desc.writable == Some(true) || differs(&desc.value, &current.value))
        {
            return None;
        }
    }
    let enumerable = desc.enumerable.or(current.enumerable);
    let configurable = desc.configurable.or(current.configurable);
    let merged = if generic || desc.is_accessor() == current.is_accessor() {
        Descriptor {
            value: desc.value.clone().or_else(|| current.value.clone()),
            writable: desc.writable.or(current.writable),
            get: desc.get.clone().or_else(|| current.get.clone()),
            set: desc.set.clone().or_else(|| current.set.clone()),
            enumerable,
            configurable,
        }
    } else {
        // Changement de nature : les champs de l'autre nature repartent de
        // leur valeur par défaut.
        Descriptor {
            value: desc.value.clone(),
            writable: desc.writable,
            get: desc.get.clone(),
            set: desc.set.clone(),
            enumerable,
            configurable,
        }
    };
    Some(merged.into_property())
}

impl Vm {
    pub(crate) fn is_proxy(&self, id: ObjId) -> bool {
        matches!(self.heap.get(id).kind, Kind::Proxy(_))
    }

    // ---- Descripteurs ------------------------------------------------------

    /// `ToPropertyDescriptor` (ECMA-262 §6.2.6.5).
    pub(crate) fn to_descriptor(&mut self, value: &Value) -> JsResult<Descriptor> {
        let Some(id) = value.as_object() else {
            return Err(self.type_error("un descripteur de propriété doit être un objet"));
        };
        let field = |vm: &mut Self, name: &str| -> JsResult<Option<Value>> {
            let key = PropertyKey::String(Arc::from(name));
            if vm.has_property_op(id, &key)? {
                Ok(Some(vm.get(value, name)?))
            } else {
                Ok(None)
            }
        };
        let mut desc = Descriptor {
            enumerable: field(self, "enumerable")?.map(|v| v.truthy()),
            ..Descriptor::default()
        };
        desc.configurable = field(self, "configurable")?.map(|v| v.truthy());
        desc.value = field(self, "value")?;
        desc.writable = field(self, "writable")?.map(|v| v.truthy());
        desc.get = field(self, "get")?;
        desc.set = field(self, "set")?;
        for accessor in [&desc.get, &desc.set].into_iter().flatten() {
            if *accessor != Value::Undefined && !is_callable(self, accessor) {
                return Err(self.type_error("get et set doivent être des fonctions ou undefined"));
            }
        }
        if desc.is_accessor() && desc.is_data() {
            return Err(self.type_error(
                "un descripteur ne peut pas être à la fois un accesseur et une donnée",
            ));
        }
        Ok(desc)
    }

    /// `FromPropertyDescriptor` (ECMA-262 §6.2.6.4) : objet portant les champs présents.
    pub(crate) fn from_descriptor(&mut self, desc: &Descriptor) -> Value {
        let object = self.new_object();
        let fields = [
            ("value", desc.value.clone()),
            ("writable", desc.writable.map(Value::Bool)),
            ("get", desc.get.clone()),
            ("set", desc.set.clone()),
            ("enumerable", desc.enumerable.map(Value::Bool)),
            ("configurable", desc.configurable.map(Value::Bool)),
        ];
        let props = &mut self.heap.get_mut(object).props;
        for (name, value) in fields {
            if let Some(value) = value {
                props.insert(Arc::from(name), Property::data(value));
            }
        }
        Value::Object(object)
    }

    // ---- [[GetOwnProperty]] ------------------------------------------------

    /// Vrai si la fonction `id` porte encore la propriété virtuelle `key`
    /// (`name` ou `length`), ni redéfinie ni supprimée.
    pub(crate) fn closure_virtual(&self, id: ObjId, key: &str) -> bool {
        let object = self.heap.get(id);
        let bit = match key {
            "name" => DELETED_NAME,
            "length" => DELETED_LENGTH,
            _ => return false,
        };
        matches!(object.kind, Kind::Closure(_))
            && object.deleted_virtual & bit == 0
            && !object.props.contains_key(key)
    }

    /// `[[GetOwnProperty]]` : descripteur complet de la propriété propre
    /// `key`, ou `None`.
    pub(crate) fn get_own_property(
        &mut self,
        id: ObjId,
        key: &PropertyKey,
    ) -> JsResult<Option<Descriptor>> {
        if self.is_proxy(id) {
            return self.proxy_get_own_property(id, key);
        }
        // Objet hôte : une clé que l'hôte annonce est une propriété propre,
        // en lecture seule, dont il sert la valeur.
        if let PropertyKey::String(name) = key
            && let Kind::Host { class, data } = &self.heap.get(id).kind
        {
            let (class, data) = (*class, *data);
            if let Some(hooks) = self.host_hooks
                && (hooks.keys)(self, class, data)
                    .iter()
                    .any(|own| **own == **name)
                && let Some(value) = (hooks.get)(self, class, data, name)
            {
                return Ok(Some(Descriptor::data(value?, false, true, true)));
            }
        }
        Ok(self.ordinary_get_own(id, key))
    }

    /// `[[GetOwnProperty]]` d'un objet qui n'est pas un `Proxy`.
    pub(crate) fn ordinary_get_own(&mut self, id: ObjId, key: &PropertyKey) -> Option<Descriptor> {
        if let PropertyKey::String(name) = key
            && let Some(cell) = self.mapped_argument(id, name)
            && let Some(property) = self.heap.get(id).props.get(name)
        {
            // Élément de `arguments` lié : la valeur est celle du paramètre.
            let (writable, enumerable, configurable) = (
                property.writable,
                property.enumerable,
                property.configurable,
            );
            return Some(Descriptor::data(
                self.cell_value(cell),
                writable,
                enumerable,
                configurable,
            ));
        }
        if let PropertyKey::String(name) = key {
            match &self.heap.get(id).kind {
                Kind::Array(items) => {
                    if &**name == "length" {
                        #[expect(
                            clippy::cast_precision_loss,
                            reason = "longueur de tableau petite devant 2^53"
                        )]
                        let length = Value::Number(items.len() as f64);
                        let writable = !self.heap.get(id).props.contains_key("length");
                        return Some(Descriptor::data(length, writable, false, false));
                    }
                    if let Some(value) = array_index(name).and_then(|i| items.get(i))
                        && *value != Value::Hole
                    {
                        return Some(Descriptor::plain(value.clone()));
                    }
                }
                Kind::Primitive(Value::String(s)) => {
                    if &**name == "length" {
                        #[expect(
                            clippy::cast_precision_loss,
                            reason = "longueur de chaîne petite devant 2^53"
                        )]
                        let length = Value::Number(utf16_len(s) as f64);
                        return Some(Descriptor::data(length, false, false, false));
                    }
                    if let Some(c) = array_index(name).and_then(|i| char_at(s, i)) {
                        return Some(Descriptor::data(Value::string(&c), false, true, false));
                    }
                }
                Kind::TypedArray(_) => {
                    if let Some(index) = canonical_numeric_index(name) {
                        let immutable = self.typed_immutable(id);
                        return index.and_then(|i| self.typed_get(id, i)).map(|value| {
                            if immutable {
                                Descriptor::data(value, false, true, false)
                            } else {
                                Descriptor::plain(value)
                            }
                        });
                    }
                }
                Kind::Closure(c) => {
                    if self.closure_virtual(id, name) {
                        let value = if &**name == "name" {
                            Value::String(Arc::clone(&c.proto.name))
                        } else {
                            Value::Number(f64::from(c.proto.length))
                        };
                        return Some(Descriptor::data(value, false, false, true));
                    }
                    if &**name == "prototype" {
                        // Créé à la première demande, puis propriété ordinaire.
                        let _ = self.get_with_receiver(id, "prototype", &Value::Object(id));
                    }
                }
                _ => {}
            }
        }
        self.heap.get(id).prop(key).map(Descriptor::from_property)
    }

    // ---- [[DefineOwnProperty]] ---------------------------------------------

    /// `[[DefineOwnProperty]]` : `false` si la définition est refusée.
    pub(crate) fn define_own_property(
        &mut self,
        id: ObjId,
        key: &PropertyKey,
        desc: Descriptor,
    ) -> JsResult<bool> {
        if self.is_proxy(id) {
            return self.proxy_define_own_property(id, key, desc);
        }
        if let PropertyKey::String(name) = key
            && let Some(cell) = self.mapped_argument(id, name)
        {
            // `arguments` lié (ECMA-262 §10.4.4.2) : la définition ordinaire
            // se fait sur la valeur courante du paramètre ; le lien est rompu
            // si l'élément devient un accesseur ou cesse d'être inscriptible.
            let current = self.cell_value(cell);
            if let Some(property) = self.heap.get_mut(id).props.get_mut(name)
                && let Slot::Data(slot) = &mut property.slot
            {
                *slot = current;
            }
            let accessor = desc.get.is_some() || desc.set.is_some();
            let read_only = desc.writable == Some(false);
            let new_value = desc.value.clone();
            self.map_argument(id, name, None);
            if !self.define_own_property(id, key, desc)? {
                self.map_argument(id, name, Some(cell));
                return Ok(false);
            }
            if !accessor {
                if let Some(value) = new_value {
                    self.set_cell_value(cell, value);
                }
                if !read_only {
                    self.map_argument(id, name, Some(cell));
                }
            }
            return Ok(true);
        }
        if let PropertyKey::String(name) = key {
            match &self.heap.get(id).kind {
                Kind::Array(_) if &**name == "length" => return self.array_set_length(id, desc),
                Kind::Array(_) => {
                    if let Some(index) = array_index(name) {
                        return self.array_define_index(id, index, name, &desc);
                    }
                }
                Kind::TypedArray(_) => {
                    if let Some(index) = canonical_numeric_index(name) {
                        return self.typed_define(id, index, &desc);
                    }
                }
                // Longueur et caractères d'une chaîne : figés, seule une
                // redéfinition à l'identique est acceptée.
                Kind::Primitive(Value::String(s))
                    if &**name == "length"
                        || array_index(name).is_some_and(|i| i < utf16_len(s)) =>
                {
                    let current = self.ordinary_get_own(id, key);
                    return Ok(validate_descriptor(current.as_ref(), &desc, false).is_some());
                }
                _ => {}
            }
        }
        let current = self.ordinary_get_own(id, key);
        let extensible = self.heap.get(id).extensible;
        let Some(property) = validate_descriptor(current.as_ref(), &desc, extensible) else {
            return Ok(false);
        };
        let object = self.heap.get_mut(id);
        if let PropertyKey::String(name) = key {
            // Une propriété virtuelle redéfinie devient ordinaire.
            match &**name {
                "name" => object.deleted_virtual &= !DELETED_NAME,
                "length" => object.deleted_virtual &= !DELETED_LENGTH,
                _ => {}
            }
        }
        object.insert_prop(key.clone(), property);
        Ok(true)
    }

    /// `DefinePropertyOrThrow` : `TypeError` si la définition est refusée.
    pub(crate) fn define_or_throw(
        &mut self,
        id: ObjId,
        key: &PropertyKey,
        desc: Descriptor,
    ) -> JsResult<()> {
        if self.define_own_property(id, key, desc)? {
            Ok(())
        } else {
            Err(self.type_error(&format!(
                "la propriété « {} » ne peut pas être redéfinie",
                key.describe()
            )))
        }
    }

    /// `CreateDataProperty` : propriété de données ordinaire.
    pub(crate) fn create_data_property(
        &mut self,
        id: ObjId,
        key: &PropertyKey,
        value: Value,
    ) -> JsResult<bool> {
        self.define_own_property(id, key, Descriptor::plain(value))
    }

    fn array_length(&self, id: ObjId) -> usize {
        match &self.heap.get(id).kind {
            Kind::Array(items) => items.len(),
            _ => 0,
        }
    }

    /// Définition d'un élément de tableau (ECMA-262 §10.4.2.1).
    fn array_define_index(
        &mut self,
        id: ObjId,
        index: usize,
        name: &JsStr,
        desc: &Descriptor,
    ) -> JsResult<bool> {
        let length = self.array_length(id);
        let frozen_length = self.heap.get(id).props.contains_key("length");
        if index >= length && frozen_length {
            return Ok(false);
        }
        let current = self.ordinary_get_own(id, &PropertyKey::String(Arc::clone(name)));
        let extensible = self.heap.get(id).extensible;
        let Some(property) = validate_descriptor(current.as_ref(), desc, extensible) else {
            return Ok(false);
        };
        if index >= length && index - length > MAX_ARRAY_GAP {
            return Err(self.error("RangeError", "index de tableau trop grand"));
        }
        let object = self.heap.get_mut(id);
        let Kind::Array(items) = &mut object.kind else {
            return Ok(false);
        };
        if index >= items.len() {
            items.resize(index + 1, Value::Hole);
        }
        let plain = property.writable && property.enumerable && property.configurable;
        match property.slot {
            Slot::Data(value) if plain => {
                items[index] = value;
                object.props.shift_remove(&**name);
            }
            slot => {
                items[index] = Value::Hole;
                object.props.insert(
                    Arc::clone(name),
                    Property {
                        slot,
                        writable: property.writable,
                        enumerable: property.enumerable,
                        configurable: property.configurable,
                    },
                );
            }
        }
        Ok(true)
    }

    /// `ArraySetLength` (ECMA-262 §10.4.2.4).
    fn array_set_length(&mut self, id: ObjId, mut desc: Descriptor) -> JsResult<bool> {
        let key = PropertyKey::String(Arc::from("length"));
        let freeze = desc.writable == Some(false);
        let mut new_length = None;
        if let Some(value) = &desc.value {
            let number = self.to_number(value)?;
            let length = to_uint32(number);
            // Seconde conversion, observable, exigée par le standard.
            if f64::from(length) != self.to_number(value)? {
                return Err(self.error("RangeError", "longueur de tableau invalide"));
            }
            desc.value = Some(Value::Number(f64::from(length)));
            new_length = Some(length as usize);
        }
        let current = self.ordinary_get_own(id, &key);
        if validate_descriptor(current.as_ref(), &desc, true).is_none() {
            return Ok(false);
        }
        let mut complete = true;
        if let Some(new_length) = new_length {
            let old_length = self.array_length(id);
            if new_length > old_length && new_length - old_length > MAX_ARRAY_GAP {
                return Err(self.error("RangeError", "longueur de tableau invalide"));
            }
            let object = self.heap.get_mut(id);
            let mut final_length = new_length;
            if new_length < old_length {
                // Un élément non configurable arrête la troncature.
                for (name, property) in &object.props {
                    if let Some(index) = array_index(name)
                        && index >= final_length
                        && !property.configurable
                    {
                        final_length = index + 1;
                        complete = false;
                    }
                }
                object
                    .props
                    .retain(|name, _| array_index(name).is_none_or(|index| index < final_length));
            }
            if let Kind::Array(items) = &mut object.kind {
                items.resize(final_length, Value::Hole);
            }
        }
        if freeze {
            self.heap.get_mut(id).props.insert(
                Arc::from("length"),
                Property {
                    slot: Slot::Data(Value::Undefined),
                    writable: false,
                    enumerable: false,
                    configurable: false,
                },
            );
        }
        Ok(complete)
    }

    /// Définition d'un élément de tableau typé (ECMA-262 §10.4.5.3).
    fn typed_define(
        &mut self,
        id: ObjId,
        index: Option<usize>,
        desc: &Descriptor,
    ) -> JsResult<bool> {
        let valid = index.is_some_and(|i| self.typed_length(id).is_some_and(|len| i < len));
        if valid && self.typed_immutable(id) {
            // Élément figé : seule une redéfinition à l'identique passe.
            let current = index.and_then(|i| self.typed_get(id, i));
            let same = match (&desc.value, &current) {
                (Some(wanted), Some(current)) => same_value(wanted, current),
                _ => true,
            };
            return Ok(same
                && desc.configurable != Some(true)
                && desc.enumerable != Some(false)
                && !desc.is_accessor()
                && desc.writable != Some(true));
        }
        if !valid
            || desc.configurable == Some(false)
            || desc.enumerable == Some(false)
            || desc.is_accessor()
            || desc.writable == Some(false)
        {
            return Ok(false);
        }
        if let Some(value) = &desc.value {
            self.typed_store(id, index, value)?;
        }
        Ok(true)
    }

    // ---- [[HasProperty]] ---------------------------------------------------

    /// `[[HasProperty]]` : propriété propre ou héritée.
    pub(crate) fn has_property_op(&mut self, id: ObjId, key: &PropertyKey) -> JsResult<bool> {
        let mut current = Some(id);
        for _ in 0..10_000 {
            let Some(obj) = current else { break };
            if self.is_proxy(obj) {
                return self.proxy_has(obj, key);
            }
            if let PropertyKey::String(name) = key
                && matches!(self.heap.get(obj).kind, Kind::TypedArray(_))
                && let Some(index) = canonical_numeric_index(name)
            {
                // Clé numérique d'un tableau typé : jamais cherchée plus haut.
                return Ok(index.is_some_and(|i| self.typed_length(obj).is_some_and(|len| i < len)));
            }
            if self.has_own_key(obj, key) {
                return Ok(true);
            }
            // Objet hôte : une propriété que l'hôte sert existe.
            if let PropertyKey::String(name) = key
                && let Kind::Host { class, data } = &self.heap.get(obj).kind
            {
                let (class, data) = (*class, *data);
                if let Some(hooks) = self.host_hooks
                    && (hooks.get)(self, class, data, name).is_some()
                {
                    return Ok(true);
                }
            }
            current = self.heap.get(obj).proto;
        }
        Ok(false)
    }

    // ---- [[Set]] -----------------------------------------------------------

    /// `[[Set]]` (ECMA-262 §10.1.9) : écrit `key` en partant de `id`, avec
    /// `receiver` pour destinataire. `false` si l'écriture est refusée.
    pub(crate) fn set_with_receiver(
        &mut self,
        id: ObjId,
        key: &PropertyKey,
        value: Value,
        receiver: &Value,
    ) -> JsResult<bool> {
        let mut current = Some(id);
        for _ in 0..10_000 {
            let Some(obj) = current else { break };
            match &self.heap.get(obj).kind {
                Kind::Proxy(_) => return self.proxy_set(obj, key, value, receiver),
                Kind::TypedArray(_) => {
                    if let PropertyKey::String(name) = key
                        && let Some(index) = canonical_numeric_index(name)
                    {
                        if receiver.as_object() == Some(obj) {
                            if self.typed_immutable(obj) {
                                return Ok(false);
                            }
                            self.typed_store(obj, index, &value)?;
                            return Ok(true);
                        }
                        let valid =
                            index.is_some_and(|i| self.typed_length(obj).is_some_and(|l| i < l));
                        if !valid {
                            return Ok(true);
                        }
                    }
                }
                Kind::Host { class, data } => {
                    let (class, data) = (*class, *data);
                    if let PropertyKey::String(name) = key
                        && let Some(hooks) = self.host_hooks
                        && let Some(result) = (hooks.set)(self, class, data, name, &value)
                    {
                        return result.map(|()| true);
                    }
                }
                _ => {}
            }
            match self.ordinary_get_own(obj, key) {
                None => current = self.heap.get(obj).proto,
                Some(own) if own.is_accessor() => {
                    return match own.set {
                        Some(setter) if setter != Value::Undefined => {
                            self.call(&setter, receiver, &[value])?;
                            Ok(true)
                        }
                        _ => Ok(false),
                    };
                }
                Some(own) => {
                    if !own.is_writable() {
                        return Ok(false);
                    }
                    break;
                }
            }
        }
        let Some(target) = receiver.as_object() else {
            return Ok(false);
        };
        match self.get_own_property(target, key)? {
            Some(existing) if existing.is_accessor() || !existing.is_writable() => Ok(false),
            Some(_) => self.define_own_property(target, key, Descriptor::value_only(value)),
            None => self.create_data_property(target, key, value),
        }
    }

    // ---- [[Delete]] --------------------------------------------------------

    /// `[[Delete]]` : `false` si la propriété n'est pas configurable.
    pub(crate) fn delete_property(&mut self, id: ObjId, key: &PropertyKey) -> JsResult<bool> {
        if self.is_proxy(id) {
            return self.proxy_delete(id, key);
        }
        let name = match key {
            PropertyKey::String(name) => name,
            PropertyKey::Symbol(symbol) => {
                let object = self.heap.get_mut(id);
                return Ok(match object.symbols.get(symbol) {
                    Some(property) if !property.configurable => false,
                    Some(_) => {
                        object.symbols.shift_remove(symbol);
                        true
                    }
                    None => true,
                });
            }
        };
        // Supprimer un élément lié de `arguments` rompt le lien.
        if self.mapped_argument(id, name).is_some()
            && self
                .heap
                .get(id)
                .props
                .get(name)
                .is_none_or(|property| property.configurable)
        {
            self.map_argument(id, name, None);
        }
        if self.closure_virtual(id, name) {
            self.heap.get_mut(id).deleted_virtual |= if &**name == "name" {
                DELETED_NAME
            } else {
                DELETED_LENGTH
            };
            return Ok(true);
        }
        match &mut self.heap.get_mut(id).kind {
            Kind::Array(items) => {
                if &**name == "length" {
                    return Ok(false);
                }
                if let Some(slot) = array_index(name).and_then(|i| items.get_mut(i))
                    && *slot != Value::Hole
                {
                    *slot = Value::Hole;
                    return Ok(true);
                }
            }
            Kind::Primitive(Value::String(s)) => {
                if &**name == "length" || array_index(name).is_some_and(|i| i < utf16_len(s)) {
                    return Ok(false);
                }
            }
            Kind::TypedArray(_) => {
                if let Some(index) = canonical_numeric_index(name) {
                    // Un élément existant ne se supprime pas.
                    let present =
                        index.is_some_and(|i| self.typed_length(id).is_some_and(|l| i < l));
                    return Ok(!present);
                }
            }
            _ => {}
        }
        let object = self.heap.get_mut(id);
        Ok(match object.props.get(&**name) {
            Some(property) if !property.configurable => false,
            Some(_) => {
                object.props.shift_remove(&**name);
                true
            }
            None => true,
        })
    }

    // ---- [[OwnPropertyKeys]] -----------------------------------------------

    /// `[[OwnPropertyKeys]]` : index croissants, puis chaînes et symboles
    /// dans l'ordre de création.
    pub(crate) fn own_property_keys(&mut self, id: ObjId) -> JsResult<Vec<PropertyKey>> {
        if self.is_proxy(id) {
            return self.proxy_own_keys(id);
        }
        let own = self.ordinary_own_keys(id);
        // Objet hôte : les clés que l'hôte ajoute viennent en tête.
        if let Kind::Host { class, data } = &self.heap.get(id).kind {
            let (class, data) = (*class, *data);
            if let Some(hooks) = self.host_hooks {
                let mut keys: Vec<PropertyKey> = (hooks.keys)(self, class, data)
                    .iter()
                    .map(|name| PropertyKey::String(Arc::from(name.as_str())))
                    .collect();
                keys.extend(own);
                return Ok(keys);
            }
        }
        Ok(own)
    }

    /// `[[OwnPropertyKeys]]` d'un objet qui n'est pas un `Proxy`.
    pub(crate) fn ordinary_own_keys(&mut self, id: ObjId) -> Vec<PropertyKey> {
        let has_prototype = match &self.heap.get(id).kind {
            Kind::Closure(c) => c.proto.has_prototype(),
            _ => false,
        };
        if has_prototype {
            // Matérialise `prototype`, créé à la première demande.
            let _ = self.get_with_receiver(id, "prototype", &Value::Object(id));
        }
        let mut keys: Vec<PropertyKey> = self
            .own_keys(id, false)
            .into_iter()
            .map(PropertyKey::String)
            .collect();
        // Les propriétés virtuelles qui ne sont pas des index viennent juste
        // après eux.
        let indices = keys
            .iter()
            .take_while(
                |key| matches!(key, PropertyKey::String(name) if array_index(name).is_some()),
            )
            .count();
        let mut virtuals: Vec<PropertyKey> = Vec::new();
        match &self.heap.get(id).kind {
            Kind::Array(_) | Kind::Primitive(Value::String(_)) => {
                virtuals.push(PropertyKey::String(Arc::from("length")));
            }
            Kind::Closure(_) => {
                for name in ["length", "name"] {
                    if self.closure_virtual(id, name) {
                        virtuals.push(PropertyKey::String(Arc::from(name)));
                    }
                }
            }
            _ => {}
        }
        keys.splice(indices..indices, virtuals);
        keys.extend(
            self.own_symbols(id, false)
                .into_iter()
                .map(PropertyKey::Symbol),
        );
        keys
    }

    /// Clés propres énumérables de `id` qui sont des chaînes (`Object.keys`,
    /// `for…in`), dans l'ordre de `[[OwnPropertyKeys]]`.
    pub(crate) fn enumerable_own_keys(&mut self, id: ObjId) -> JsResult<Vec<JsStr>> {
        if !self.is_proxy(id) {
            return Ok(self.own_keys(id, true));
        }
        let mut keys = Vec::new();
        for key in self.own_property_keys(id)? {
            if let PropertyKey::String(name) = &key
                && self
                    .get_own_property(id, &key)?
                    .is_some_and(|desc| desc.is_enumerable())
            {
                keys.push(Arc::clone(name));
            }
        }
        Ok(keys)
    }

    // ---- Prototype et extensibilité ----------------------------------------

    /// `[[GetPrototypeOf]]`.
    pub(crate) fn get_prototype_of(&mut self, id: ObjId) -> JsResult<Option<ObjId>> {
        if self.is_proxy(id) {
            return self.proxy_get_prototype_of(id);
        }
        Ok(self.heap.get(id).proto)
    }

    /// `[[SetPrototypeOf]]` : `false` si le changement est refusé.
    pub(crate) fn set_prototype_of(&mut self, id: ObjId, proto: Option<ObjId>) -> JsResult<bool> {
        if self.is_proxy(id) {
            return self.proxy_set_prototype_of(id, proto);
        }
        let object = self.heap.get(id);
        if object.proto == proto {
            return Ok(true);
        }
        // `Object.prototype` a un prototype immuable (ECMA-262 §10.4.7).
        if !object.extensible || id == self.intrinsics.object_proto {
            return Ok(false);
        }
        let mut current = proto;
        for _ in 0..10_000 {
            match current {
                Some(p) if p == id => return Ok(false),
                // Un `Proxy` dans la chaîne : son prototype n'est pas connu.
                Some(p) if self.is_proxy(p) => break,
                Some(p) => current = self.heap.get(p).proto,
                None => break,
            }
        }
        self.heap.get_mut(id).proto = proto;
        Ok(true)
    }

    /// `[[IsExtensible]]`.
    pub(crate) fn is_extensible(&mut self, id: ObjId) -> JsResult<bool> {
        if self.is_proxy(id) {
            return self.proxy_is_extensible(id);
        }
        Ok(self.heap.get(id).extensible)
    }

    /// `[[PreventExtensions]]`.
    pub(crate) fn prevent_extensions(&mut self, id: ObjId) -> JsResult<bool> {
        if self.is_proxy(id) {
            return self.proxy_prevent_extensions(id);
        }
        self.heap.get_mut(id).extensible = false;
        Ok(true)
    }
}
