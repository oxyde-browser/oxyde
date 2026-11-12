//! Accès aux propriétés, objets exotiques (tableaux, chaînes, fonctions,
//! objets de l'hôte) et protocole d'itération.

use std::sync::Arc;

use crate::heap::{IndexedKind, IterState, Kind, Object, Property, Slot};
use crate::value::{
    JsStr, ObjId, PropertyKey, Symbol, Value, array_index, canonical_numeric_index,
    number_to_string, utf16_len,
};
use crate::vm::{JsResult, Vm};

/// Écart maximal toléré entre la longueur d'un tableau et un index écrit
/// au-delà : borne la mémoire qu'un script peut réserver d'un coup.
pub(crate) const MAX_ARRAY_GAP: usize = 1 << 24;

/// Unité UTF-16 d'index `index` de `s`, sous forme de chaîne.
pub(crate) fn char_at(s: &str, index: usize) -> Option<String> {
    if s.is_ascii() {
        return s.get(index..=index).map(str::to_owned);
    }
    let unit = s.encode_utf16().nth(index)?;
    // LIMITE : un demi-substitut isolé devient U+FFFD.
    Some(String::from_utf16_lossy(&[unit]))
}

impl Vm {
    /// Lit la propriété `key` de `target`.
    ///
    /// # Errors
    /// `TypeError` si `target` est `null` ou `undefined` ; toute valeur levée
    /// par un accesseur.
    pub fn get(&mut self, target: &Value, key: &str) -> JsResult {
        match target {
            Value::Object(id) => self.get_with_receiver(*id, key, target),
            Value::String(s) => {
                if key == "length" {
                    #[expect(
                        clippy::cast_precision_loss,
                        reason = "longueur de chaîne petite devant 2^53"
                    )]
                    return Ok(Value::Number(utf16_len(s) as f64));
                }
                if let Some(index) = array_index(key) {
                    return Ok(char_at(s, index).map_or(Value::Undefined, |c| Value::string(&c)));
                }
                self.get_with_receiver(self.intrinsics.string_proto, key, target)
            }
            Value::Number(_) => self.get_with_receiver(self.intrinsics.number_proto, key, target),
            Value::Bool(_) => self.get_with_receiver(self.intrinsics.boolean_proto, key, target),
            Value::Symbol(_) => self.get_with_receiver(self.intrinsics.symbol_proto, key, target),
            Value::BigInt(_) => self.get_with_receiver(self.intrinsics.bigint_proto, key, target),
            Value::Undefined | Value::Null | Value::Hole => {
                let shown = if *target == Value::Null {
                    "null"
                } else {
                    "undefined"
                };
                Err(self.type_error(&format!(
                    "impossible de lire la propriété « {key} » de {shown}"
                )))
            }
        }
    }

    /// Prototype de fonction, créé à la première demande.
    fn function_prototype(&mut self, func: ObjId) -> Option<Value> {
        let Kind::Closure(c) = &self.heap.get(func).kind else {
            return None;
        };
        if !c.proto.has_prototype() {
            return None;
        }
        let (is_generator, is_async) = (c.proto.is_generator, c.proto.is_async);
        let proto = self.new_object();
        if is_generator {
            // Prototype des objets générateurs : sans `constructor`.
            self.heap.get_mut(proto).proto = Some(if is_async {
                self.intrinsics.async_generator_proto
            } else {
                self.intrinsics.generator_proto
            });
        } else {
            self.heap.get_mut(proto).props.insert(
                Arc::from("constructor"),
                Property::hidden(Value::Object(func)),
            );
        }
        let value = Value::Object(proto);
        self.heap.get_mut(func).props.insert(
            Arc::from("prototype"),
            Property {
                slot: Slot::Data(value.clone()),
                writable: true,
                enumerable: false,
                configurable: false,
            },
        );
        Some(value)
    }

    /// Propriété propre « virtuelle » d'un objet exotique, s'il en a une pour `key`.
    fn exotic_get(&mut self, id: ObjId, key: &str) -> Option<JsResult> {
        match &self.heap.get(id).kind {
            Kind::Array(items) => {
                if key == "length" {
                    #[expect(
                        clippy::cast_precision_loss,
                        reason = "longueur de tableau petite devant 2^53"
                    )]
                    return Some(Ok(Value::Number(items.len() as f64)));
                }
                let value = items.get(array_index(key)?)?;
                (*value != Value::Hole).then(|| Ok(value.clone()))
            }
            Kind::Primitive(Value::String(s)) => {
                if key == "length" {
                    #[expect(
                        clippy::cast_precision_loss,
                        reason = "longueur de chaîne petite devant 2^53"
                    )]
                    return Some(Ok(Value::Number(utf16_len(s) as f64)));
                }
                char_at(s, array_index(key)?).map(|c| Ok(Value::string(&c)))
            }
            Kind::Closure(c) => match key {
                "name" if self.closure_virtual(id, key) => {
                    Some(Ok(Value::String(Arc::clone(&c.proto.name))))
                }
                "length" if self.closure_virtual(id, key) => {
                    Some(Ok(Value::Number(f64::from(c.proto.length))))
                }
                "prototype" if !self.heap.get(id).props.contains_key("prototype") => {
                    self.function_prototype(id).map(Ok)
                }
                _ => None,
            },
            Kind::Host { class, data } => {
                let (class, data) = (*class, *data);
                let hooks = self.host_hooks?;
                (hooks.get)(self, class, data, key)
            }
            // Tableau typé : une clé numérique désigne un élément ou rien, sans
            // jamais consulter le prototype.
            Kind::TypedArray(_) => {
                let index = canonical_numeric_index(key)?;
                Some(Ok(index
                    .and_then(|i| self.typed_get(id, i))
                    .unwrap_or(Value::Undefined)))
            }
            _ => None,
        }
    }

    /// Vrai si `key` désigne un élément existant du tableau typé `id` ;
    /// `None` si `id` n'est pas un tableau typé ou si la clé n'est pas numérique.
    fn typed_has(&self, id: ObjId, key: &str) -> Option<bool> {
        if !matches!(self.heap.get(id).kind, Kind::TypedArray(_)) {
            return None;
        }
        let index = canonical_numeric_index(key)?;
        Some(index.is_some_and(|i| self.typed_length(id).is_some_and(|len| i < len)))
    }

    /// Lit `key` sur l'objet `id` et sa chaîne de prototypes ; `receiver` est
    /// le `this` passé aux accesseurs.
    pub(crate) fn get_with_receiver(&mut self, id: ObjId, key: &str, receiver: &Value) -> JsResult {
        let mut current = Some(id);
        // Borne : une chaîne de prototypes cyclique ne peut pas être construite
        // (voir `set_prototype`), mais on reste défensif.
        for _ in 0..10_000 {
            let Some(obj) = current else { break };
            if self.is_proxy(obj) {
                return self.proxy_get(obj, &PropertyKey::String(Arc::from(key)), receiver);
            }
            if let Some(result) = self.exotic_get(obj, key) {
                return result;
            }
            let object = self.heap.get(obj);
            match object.props.get(key).map(|p| &p.slot) {
                Some(Slot::Data(value)) => return Ok(value.clone()),
                Some(Slot::Accessor { get, .. }) => {
                    return match *get {
                        Some(getter) => self.call(&Value::Object(getter), receiver, &[]),
                        None => Ok(Value::Undefined),
                    };
                }
                None => current = object.proto,
            }
        }
        Ok(Value::Undefined)
    }

    /// Lit `target[key]`.
    pub(crate) fn get_element(&mut self, target: &Value, key: &Value) -> JsResult {
        // Chemin rapide : tableau indexé par un entier.
        if let (Value::Object(id), Value::Number(n)) = (target, key)
            && let Kind::Array(items) = &self.heap.get(*id).kind
            && *n >= 0.0
            && n.fract() == 0.0
        {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "entier positif vérifié"
            )]
            if let Some(value) = items.get(*n as usize)
                && *value != Value::Hole
            {
                return Ok(value.clone());
            }
        }
        if let (Value::Object(id), Value::Number(n)) = (target, key)
            && matches!(self.heap.get(*id).kind, Kind::TypedArray(_))
            && *n >= 0.0
            && n.fract() == 0.0
            && *n < 9_007_199_254_740_992.0
        {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "entier vérifié dans [0, 2^53)"
            )]
            let index = *n as usize;
            return Ok(self.typed_get(*id, index).unwrap_or(Value::Undefined));
        }
        let key = self.to_key(key)?;
        self.get_key(target, &key)
    }

    /// Lit la propriété `key` (chaîne ou symbole) de `target`.
    ///
    /// # Errors
    /// Comme [`Vm::get`].
    pub fn get_key(&mut self, target: &Value, key: &PropertyKey) -> JsResult {
        match key {
            PropertyKey::String(s) => self.get(target, s),
            PropertyKey::Symbol(symbol) => {
                let start = match target {
                    Value::Object(id) => *id,
                    Value::String(_) => self.intrinsics.string_proto,
                    Value::Number(_) => self.intrinsics.number_proto,
                    Value::Bool(_) => self.intrinsics.boolean_proto,
                    Value::Symbol(_) => self.intrinsics.symbol_proto,
                    Value::BigInt(_) => self.intrinsics.bigint_proto,
                    Value::Undefined | Value::Null | Value::Hole => {
                        return Err(self.type_error(&format!(
                            "impossible de lire la propriété « {} » de null ou undefined",
                            key.describe()
                        )));
                    }
                };
                self.get_symbol_with_receiver(start, symbol, target)
            }
        }
    }

    /// Lit la propriété symbole `symbol` sur `id` et sa chaîne de prototypes.
    pub(crate) fn get_symbol_with_receiver(
        &mut self,
        id: ObjId,
        symbol: &Symbol,
        receiver: &Value,
    ) -> JsResult {
        if symbol.is_private() {
            // Un membre privé appartient à l'objet lui-même, jamais à son
            // prototype, et ne passe par aucune trappe.
            let found = self
                .heap
                .get(id)
                .symbols
                .get(symbol)
                .map(|p| p.slot.clone());
            let name = symbol.description().unwrap_or("").to_owned();
            return match found {
                Some(Slot::Data(value)) => Ok(value),
                Some(Slot::Accessor {
                    get: Some(getter), ..
                }) => self.call(&Value::Object(getter), receiver, &[]),
                Some(Slot::Accessor { get: None, .. }) => Err(self.type_error(&format!(
                    "le membre privé {name} n'a pas d'accesseur de lecture"
                ))),
                None => Err(self.type_error(&format!("l'objet n'a pas de membre privé {name}"))),
            };
        }
        let mut current = Some(id);
        for _ in 0..10_000 {
            let Some(obj) = current else { break };
            if self.is_proxy(obj) {
                return self.proxy_get(obj, &PropertyKey::Symbol(symbol.clone()), receiver);
            }
            let object = self.heap.get(obj);
            match object.symbols.get(symbol).map(|p| &p.slot) {
                Some(Slot::Data(value)) => return Ok(value.clone()),
                Some(Slot::Accessor { get, .. }) => {
                    return match *get {
                        Some(getter) => self.call(&Value::Object(getter), receiver, &[]),
                        None => Ok(Value::Undefined),
                    };
                }
                None => current = object.proto,
            }
        }
        if symbol.is_private() {
            return Err(self.type_error(&format!(
                "l'objet n'a pas de membre privé {}",
                symbol.description().unwrap_or("")
            )));
        }
        Ok(Value::Undefined)
    }

    /// Lit `key` sur `id` et sa chaîne de prototypes, avec `receiver` pour `this`.
    pub(crate) fn get_key_with_receiver(
        &mut self,
        id: ObjId,
        key: &PropertyKey,
        receiver: &Value,
    ) -> JsResult {
        match key {
            PropertyKey::String(s) => self.get_with_receiver(id, s, receiver),
            PropertyKey::Symbol(s) => self.get_symbol_with_receiver(id, s, receiver),
        }
    }

    /// Vrai si `key` est une propriété propre de `id`.
    pub(crate) fn has_own_key(&self, id: ObjId, key: &PropertyKey) -> bool {
        match key {
            PropertyKey::String(s) => self.has_own(id, s),
            PropertyKey::Symbol(s) => self.heap.get(id).symbols.contains_key(s),
        }
    }

    /// Écrit `value` dans la propriété `key` (chaîne ou symbole) de `target`.
    ///
    /// # Errors
    /// Comme [`Vm::set`].
    pub fn set_key(&mut self, target: &Value, key: &PropertyKey, value: Value) -> JsResult<()> {
        self.put_key(target, key, value).map(|_| ())
    }

    /// Comme [`Vm::set_key`], et dit si l'écriture a eu lieu.
    pub(crate) fn put_key(
        &mut self,
        target: &Value,
        key: &PropertyKey,
        value: Value,
    ) -> JsResult<bool> {
        let symbol = match key {
            PropertyKey::String(s) => return self.put(target, s, value),
            PropertyKey::Symbol(symbol) => symbol,
        };
        let id = match target {
            Value::Object(id) => *id,
            Value::Undefined | Value::Null | Value::Hole => {
                return Err(self.type_error(&format!(
                    "impossible d'écrire la propriété « {} » de null ou undefined",
                    key.describe()
                )));
            }
            _ => return Ok(false),
        };
        if symbol.is_private() {
            return self.set_private(id, target, symbol, value).map(|()| true);
        }
        // Chemin rapide : propriété de données propre.
        let object = self.heap.get_mut(id);
        if !matches!(object.kind, Kind::Proxy(_))
            && let Some(property) = object.symbols.get_mut(symbol)
            && let Slot::Data(slot) = &mut property.slot
        {
            let writable = property.writable;
            if writable {
                *slot = value;
            }
            return Ok(writable);
        }
        self.set_with_receiver(id, key, value, target)
    }

    /// Écrit le membre privé `symbol` : il doit déjà exister sur l'objet.
    fn set_private(
        &mut self,
        id: ObjId,
        target: &Value,
        symbol: &Symbol,
        value: Value,
    ) -> JsResult<()> {
        let name = symbol.description().unwrap_or("").to_owned();
        let found = self
            .heap
            .get(id)
            .symbols
            .get(symbol)
            .map(|p| (p.slot.clone(), p.writable));
        match found {
            Some((
                Slot::Accessor {
                    set: Some(setter), ..
                },
                _,
            )) => self
                .call(&Value::Object(setter), target, &[value])
                .map(|_| ()),
            Some((Slot::Accessor { set: None, .. }, _)) => Err(self.type_error(&format!(
                "le membre privé {name} n'a pas d'accesseur d'écriture"
            ))),
            Some((Slot::Data(_), false)) => Err(self.type_error(&format!(
                "la méthode privée {name} ne peut pas être remplacée"
            ))),
            Some((Slot::Data(_), true)) => {
                if let Some(property) = self.heap.get_mut(id).symbols.get_mut(symbol) {
                    property.slot = Slot::Data(value);
                }
                Ok(())
            }
            // Un membre privé n'est jamais créé par affectation.
            None => Err(self.type_error(&format!("l'objet n'a pas de membre privé {name}"))),
        }
    }

    /// Supprime la propriété `key` (chaîne ou symbole).
    pub(crate) fn delete_key(&mut self, target: &Value, key: &PropertyKey) -> JsResult<bool> {
        match (target, key) {
            (_, PropertyKey::String(s)) => self.delete(target, s),
            (Value::Object(id), _) => self.delete_property(*id, key),
            _ => Ok(true),
        }
    }

    /// Symboles servant de clés aux propriétés propres de `id`.
    pub(crate) fn own_symbols(&self, id: ObjId, only_enumerable: bool) -> Vec<Symbol> {
        self.heap
            .get(id)
            .symbols
            .iter()
            .filter(|(s, p)| !s.is_private() && (!only_enumerable || p.enumerable))
            .map(|(s, _)| s.clone())
            .collect()
    }

    /// Définit une constante : ni inscriptible, ni énumérable, ni configurable.
    pub(crate) fn define_constant(&mut self, target: ObjId, name: &str, value: Value) {
        self.heap.get_mut(target).props.insert(
            Arc::from(name),
            Property {
                slot: Slot::Data(value),
                writable: false,
                enumerable: false,
                configurable: false,
            },
        );
    }

    /// Définit une propriété de données non énumérable de clé symbole.
    /// `Symbol.toStringTag` n'est jamais inscriptible sur un intégré.
    pub fn define_hidden_symbol(&mut self, target: ObjId, symbol: &Symbol, value: Value) {
        let writable = *symbol != self.well_known.to_string_tag;
        self.heap.get_mut(target).symbols.insert(
            symbol.clone(),
            Property {
                writable,
                ..Property::hidden(value)
            },
        );
    }

    /// Vrai si `key` existe sur `id` ou sa chaîne de prototypes (opérateur `in`).
    pub(crate) fn has_property(&self, id: ObjId, key: &str) -> bool {
        let mut current = Some(id);
        for _ in 0..10_000 {
            let Some(obj) = current else { return false };
            if let Some(present) = self.typed_has(obj, key) {
                return present;
            }
            if self.has_own(obj, key) {
                return true;
            }
            current = self.heap.get(obj).proto;
        }
        false
    }

    /// Vrai si `key` est une propriété propre de `id`.
    pub(crate) fn has_own(&self, id: ObjId, key: &str) -> bool {
        if let Some(present) = self.typed_has(id, key) {
            return present;
        }
        let object = self.heap.get(id);
        if object.props.contains_key(key) {
            return true;
        }
        match &object.kind {
            Kind::Array(items) => {
                key == "length"
                    || array_index(key)
                        .and_then(|i| items.get(i))
                        .is_some_and(|v| *v != Value::Hole)
            }
            Kind::Primitive(Value::String(s)) => {
                key == "length" || array_index(key).is_some_and(|i| i < utf16_len(s))
            }
            Kind::Closure(c) => {
                self.closure_virtual(id, key) || (key == "prototype" && c.proto.has_prototype())
            }
            _ => false,
        }
    }

    /// Écrit `value` dans la propriété `key` de `target`.
    ///
    /// # Errors
    /// `TypeError` si `target` est `null` ou `undefined` ; toute valeur levée
    /// par un mutateur.
    pub fn set(&mut self, target: &Value, key: &str, value: Value) -> JsResult<()> {
        self.put(target, key, value).map(|_| ())
    }

    /// Comme [`Vm::set`], et dit si l'écriture a eu lieu : `false` pour une
    /// propriété en lecture seule, un accesseur sans mutateur ou un objet non
    /// extensible.
    pub(crate) fn put(&mut self, target: &Value, key: &str, value: Value) -> JsResult<bool> {
        let id = match target {
            Value::Object(id) => *id,
            Value::Undefined | Value::Null | Value::Hole => {
                let shown = if *target == Value::Null {
                    "null"
                } else {
                    "undefined"
                };
                return Err(self.type_error(&format!(
                    "impossible d'écrire la propriété « {key} » de {shown}"
                )));
            }
            // Primitive : seul un mutateur hérité peut agir.
            Value::String(_) => self.intrinsics.string_proto,
            Value::Number(_) => self.intrinsics.number_proto,
            Value::Bool(_) => self.intrinsics.boolean_proto,
            Value::Symbol(_) => self.intrinsics.symbol_proto,
            Value::BigInt(_) => self.intrinsics.bigint_proto,
        };
        if target.as_object().is_some() {
            // Chemins rapides : élément dense d'un tableau, propriété de
            // données propre.
            let object = self.heap.get_mut(id);
            let fast = match &mut object.kind {
                Kind::Array(items) => match array_index(key) {
                    Some(index) => {
                        if let Some(slot) = items.get_mut(index)
                            && *slot != Value::Hole
                        {
                            *slot = value;
                            return Ok(true);
                        }
                        false
                    }
                    None => key != "length",
                },
                Kind::Proxy(_) | Kind::TypedArray(_) | Kind::Host { .. } => false,
                _ => true,
            };
            if fast
                && let Some(property) = object.props.get_mut(key)
                && let Slot::Data(slot) = &mut property.slot
            {
                let writable = property.writable;
                if writable {
                    *slot = value;
                }
                return Ok(writable);
            }
        }
        self.set_with_receiver(id, &PropertyKey::String(Arc::from(key)), value, target)
    }

    /// Écrit `target[key]` et dit si l'écriture a eu lieu.
    pub(crate) fn put_element(
        &mut self,
        target: &Value,
        key: &Value,
        value: Value,
    ) -> JsResult<bool> {
        if let (Value::Object(id), Value::Number(n)) = (target, key)
            && *n >= 0.0
            && n.fract() == 0.0
        {
            let object = self.heap.get_mut(*id);
            // Ajout en fin de tableau : direct si rien sur l'objet ne peut
            // l'intercepter (longueur figée, objet non extensible).
            let appendable = object.extensible && object.props.is_empty();
            if let Kind::Array(items) = &mut object.kind {
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "entier positif vérifié"
                )]
                let index = *n as usize;
                let length = items.len();
                if index == length && appendable {
                    items.push(value);
                    return Ok(true);
                }
                // Un trou peut cacher un accesseur ou un élément figé.
                if let Some(slot) = items.get_mut(index)
                    && *slot != Value::Hole
                {
                    *slot = value;
                    return Ok(true);
                }
            }
        }
        if let (Value::Object(id), Value::Number(n)) = (target, key)
            && matches!(self.heap.get(*id).kind, Kind::TypedArray(_))
            && *n >= 0.0
            && n.fract() == 0.0
            && *n < 9_007_199_254_740_992.0
        {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "entier vérifié dans [0, 2^53)"
            )]
            let index = *n as usize;
            self.typed_store(*id, Some(index), &value)?;
            return Ok(true);
        }
        let key = self.to_key(key)?;
        self.put_key(target, &key, value)
    }

    /// Supprime la propriété `key` ; renvoie `false` si elle n'est pas configurable.
    pub(crate) fn delete(&mut self, target: &Value, key: &str) -> JsResult<bool> {
        match target {
            Value::Object(id) => self.delete_property(*id, &PropertyKey::String(Arc::from(key))),
            Value::Undefined | Value::Null | Value::Hole => {
                Err(self.type_error(&format!("impossible de supprimer la propriété « {key} »")))
            }
            _ => Ok(true),
        }
    }

    /// Clés propres de `id` : index d'abord, puis les autres dans l'ordre de
    /// création. `only_enumerable` écarte les propriétés non énumérables.
    pub(crate) fn own_keys(&self, id: ObjId, only_enumerable: bool) -> Vec<JsStr> {
        let object = self.heap.get(id);
        let mut indices: Vec<(usize, JsStr)> = Vec::new();
        let mut is_array = false;
        match &object.kind {
            Kind::Array(items) => {
                is_array = true;
                indices.extend(
                    items
                        .iter()
                        .enumerate()
                        .filter(|(_, v)| **v != Value::Hole)
                        .map(|(i, _)| (i, Arc::from(i.to_string()))),
                );
            }
            Kind::Primitive(Value::String(s)) => {
                indices.extend((0..utf16_len(s)).map(|i| (i, Arc::from(i.to_string()))));
            }
            Kind::TypedArray(_) => {
                let length = self.typed_length(id).unwrap_or(0);
                indices.extend((0..length).map(|i| (i, Arc::from(i.to_string()))));
            }
            _ => {}
        }
        let dense = indices.len();
        let mut names: Vec<JsStr> = Vec::new();
        for (key, property) in &object.props {
            if only_enumerable && !property.enumerable {
                continue;
            }
            // Marque d'une longueur figée, pas une propriété (voir `internal`).
            if is_array && &**key == "length" {
                continue;
            }
            match array_index(key) {
                Some(index) => indices.push((index, Arc::clone(key))),
                None => names.push(Arc::clone(key)),
            }
        }
        if indices.len() > dense {
            indices.sort_by_key(|(index, _)| *index);
        }
        let mut keys: Vec<JsStr> = indices.into_iter().map(|(_, key)| key).collect();
        keys.extend(names);
        keys
    }

    /// Clés visitées par `for…in` : énumérables, propres puis héritées ; une
    /// propriété propre, même non énumérable, masque celle du prototype.
    pub(crate) fn for_in_keys(&mut self, target: &Value) -> JsResult<Vec<JsStr>> {
        let mut current = match target {
            Value::Object(id) => Some(*id),
            Value::String(s) => {
                return Ok((0..utf16_len(s))
                    .map(|i| Arc::from(i.to_string()))
                    .collect());
            }
            _ => None,
        };
        let mut keys: Vec<JsStr> = Vec::new();
        let mut seen: std::collections::HashSet<JsStr> = std::collections::HashSet::new();
        for _ in 0..10_000 {
            let Some(id) = current else { break };
            for key in self.enumerable_own_keys(id)? {
                if !seen.contains(&key) {
                    keys.push(key);
                }
            }
            current = self.get_prototype_of(id)?;
            if current.is_some() {
                let all = if self.is_proxy(id) {
                    self.own_property_keys(id)?
                        .into_iter()
                        .filter_map(|key| match key {
                            PropertyKey::String(name) => Some(name),
                            PropertyKey::Symbol(_) => None,
                        })
                        .collect()
                } else {
                    self.own_keys(id, false)
                };
                seen.extend(all);
            }
        }
        Ok(keys)
    }

    /// Copie dans `target` les propriétés propres énumérables de `source`
    /// (`{...source}`, `Object.assign`).
    pub(crate) fn copy_properties(&mut self, target: &Value, source: &Value) -> JsResult<()> {
        if let Value::Object(id) = source
            && self.is_proxy(*id)
        {
            for key in self.own_property_keys(*id)? {
                if self
                    .get_own_property(*id, &key)?
                    .is_some_and(|desc| desc.is_enumerable())
                {
                    let value = self.get_key(source, &key)?;
                    self.set_key(target, &key, value)?;
                }
            }
            return Ok(());
        }
        let keys = match source {
            Value::Object(id) => self.own_keys(*id, true),
            Value::String(s) => (0..utf16_len(s))
                .map(|i| Arc::from(i.to_string()))
                .collect(),
            _ => return Ok(()),
        };
        for key in keys {
            let value = self.get(source, &key)?;
            self.set(target, &key, value)?;
        }
        if let Value::Object(id) = source {
            for symbol in self.own_symbols(*id, true) {
                let key = PropertyKey::Symbol(symbol);
                let value = self.get_key(source, &key)?;
                self.set_key(target, &key, value)?;
            }
        }
        Ok(())
    }

    /// Change le prototype de `id`.
    ///
    /// # Errors
    /// `TypeError` si le changement créerait une chaîne de prototypes cyclique.
    pub fn set_prototype(&mut self, id: ObjId, proto: Option<ObjId>) -> JsResult<()> {
        let mut current = proto;
        for _ in 0..10_000 {
            match current {
                Some(p) if p == id => return Err(self.type_error("chaîne de prototypes cyclique")),
                Some(p) => current = self.heap.get(p).proto,
                None => break,
            }
        }
        self.heap.get_mut(id).proto = proto;
        Ok(())
    }

    /// Définit une méthode native non énumérable sur `target`.
    pub fn define_method(
        &mut self,
        target: ObjId,
        name: &str,
        arity: u32,
        func: crate::vm::NativeFn,
    ) {
        let function = self.new_function(name, arity, func);
        self.heap
            .get_mut(target)
            .props
            .insert(Arc::from(name), Property::hidden(Value::Object(function)));
    }

    /// Définit une propriété de données non énumérable sur `target`.
    pub fn define_hidden(&mut self, target: ObjId, name: &str, value: Value) {
        self.heap
            .get_mut(target)
            .props
            .insert(Arc::from(name), Property::hidden(value));
    }

    // ---- Itération -----------------------------------------------------------

    /// Itérateur interne sur les valeurs de `source` (`for…of`, décomposition).
    pub(crate) fn iterator_for(&mut self, source: &Value) -> JsResult {
        let state = match source {
            Value::String(s) => IterState::Values {
                values: s
                    .chars()
                    .map(|c| Value::string(c.encode_utf8(&mut [0; 4])))
                    .collect(),
                index: 0,
            },
            Value::Object(id) => match &self.heap.get(*id).kind {
                Kind::Array(_) if self.array_iteration_intact(*id) => IterState::Array {
                    target: *id,
                    index: 0,
                },
                Kind::Map(map) => {
                    let pairs: Vec<(Value, Value)> = map.values().cloned().collect();
                    let values = pairs
                        .into_iter()
                        .map(|(k, v)| self.new_array(vec![k, v]))
                        .collect();
                    IterState::Values { values, index: 0 }
                }
                Kind::Set(set) => IterState::Values {
                    values: set.values().cloned().collect(),
                    index: 0,
                },
                Kind::Iterator(_) => return Ok(source.clone()),
                _ => {
                    // Protocole d'itération : `source[Symbol.iterator]()` rend
                    // un objet muni d'une méthode `next`.
                    let key = PropertyKey::Symbol(self.well_known.iterator.clone());
                    let method = self.get_key(source, &key)?;
                    if !method
                        .as_object()
                        .is_some_and(|m| self.heap.get(m).is_callable())
                    {
                        let shown = self.display(source);
                        return Err(self.type_error(&format!("{shown} n'est pas itérable")));
                    }
                    return self.iterator_from_method(source, &method);
                }
            },
            _ => {
                let shown = self.display(source);
                return Err(self.type_error(&format!("{shown} n'est pas itérable")));
            }
        };
        Ok(self.new_iterator(state))
    }

    /// `GetIteratorFromMethod` : itérateur interne sur l'objet que rend
    /// `method` appelée sur `source`.
    pub(crate) fn iterator_from_method(&mut self, source: &Value, method: &Value) -> JsResult {
        let iterator = self.call(method, source, &[])?;
        let Some(iterator_id) = iterator.as_object() else {
            return Err(self.type_error("Symbol.iterator doit renvoyer un objet itérateur"));
        };
        if matches!(self.heap.get(iterator_id).kind, Kind::Iterator(_))
            && self.iterator_next_intact()
        {
            return Ok(iterator);
        }
        let next = self.get(&iterator, "next")?;
        Ok(self.new_iterator(IterState::Protocol {
            iterator,
            next,
            done: false,
        }))
    }

    /// Vrai si `next` des itérateurs internes est la fonction d'origine.
    fn iterator_next_intact(&self) -> bool {
        let Some((_, next)) = self.iteration_intrinsics else {
            return true;
        };
        matches!(
            self.heap.get(self.intrinsics.iterator_proto).props.get("next"),
            Some(Property { slot: Slot::Data(Value::Object(current)), .. }) if *current == next
        )
    }

    /// Vrai si itérer le tableau `id` par le protocole donnerait exactement
    /// ses éléments : ni lui ni `Array.prototype` ne redéfinissent
    /// `Symbol.iterator`, et `next` est d'origine.
    pub(crate) fn array_iteration_intact(&self, id: ObjId) -> bool {
        let Some((values, _)) = self.iteration_intrinsics else {
            return true;
        };
        let object = self.heap.get(id);
        let array_proto = self.intrinsics.array_proto;
        object.proto == Some(array_proto)
            && object.symbols.is_empty()
            && matches!(
                self.heap.get(array_proto).symbols.get(&self.well_known.iterator),
                Some(Property { slot: Slot::Data(Value::Object(current)), .. }) if *current == values
            )
            && self.iterator_next_intact()
    }

    /// `IteratorClose` (ECMA-262 §7.4.11) : prévient l'itérateur qu'on le
    /// quitte avant son épuisement. `quiet` : la sortie est due à une
    /// exception, qui l'emporte sur toute erreur de `return()`.
    pub(crate) fn iterator_close(&mut self, iterator: &Value, quiet: bool) -> JsResult<()> {
        let Some(id) = iterator.as_object() else {
            return Ok(());
        };
        let Kind::Iterator(IterState::Protocol { iterator, done, .. }) =
            &mut self.heap.get_mut(id).kind
        else {
            return Ok(());
        };
        if std::mem::replace(done, true) {
            return Ok(());
        }
        let iterator = iterator.clone();
        let method = self.get(&iterator, "return");
        if quiet {
            if let Ok(method) = method
                && !method.is_nullish()
            {
                let _ = self.call(&method, &iterator, &[]);
            }
            return Ok(());
        }
        let method = method?;
        if method.is_nullish() {
            return Ok(());
        }
        let result = self.call(&method, &iterator, &[])?;
        if result.as_object().is_none() {
            return Err(self.type_error("le résultat de « return » doit être un objet"));
        }
        Ok(())
    }

    /// Valeur suivante d'un itérateur interne ; `None` quand il est épuisé.
    pub(crate) fn iterator_next(&mut self, iterator: &Value) -> JsResult<Option<Value>> {
        let Some(id) = iterator.as_object() else {
            return Ok(None);
        };
        let Kind::Iterator(state) = &mut self.heap.get_mut(id).kind else {
            return Ok(None);
        };
        match state {
            IterState::Keys { keys, index } => {
                let key = keys.get(*index).cloned();
                *index += 1;
                Ok(key.map(Value::String))
            }
            IterState::Values { values, index } => {
                let value = values.get(*index).cloned();
                *index += 1;
                Ok(value)
            }
            IterState::Array { target, index } => {
                let (target, i) = (*target, *index);
                *index += 1;
                let Kind::Array(items) = &self.heap.get(target).kind else {
                    return Ok(None);
                };
                match items.get(i).cloned() {
                    // Un trou se lit comme une propriété : accesseur sur
                    // l'index, ou valeur héritée du prototype.
                    Some(Value::Hole) => {
                        #[expect(clippy::cast_precision_loss, reason = "index petit devant 2^53")]
                        let key = Value::Number(i as f64);
                        self.get_element(&Value::Object(target), &key).map(Some)
                    }
                    other => Ok(other),
                }
            }
            IterState::Indexed {
                target,
                index,
                kind,
            } => {
                let (target, i, kind) = (target.clone(), *index, *kind);
                // La longueur est relue à chaque tour : l'objet peut changer.
                let length = self.get(&target, "length")?;
                let length = self.to_number(&length)?;
                #[expect(clippy::cast_precision_loss, reason = "index petit devant 2^53")]
                let position = i as f64;
                if length.is_nan() || position >= length {
                    // Épuisé pour de bon, même si l'objet grandit ensuite.
                    if let Kind::Iterator(IterState::Indexed { index, .. }) =
                        &mut self.heap.get_mut(id).kind
                    {
                        *index = usize::MAX;
                    }
                    return Ok(None);
                }
                if let Kind::Iterator(IterState::Indexed { index, .. }) =
                    &mut self.heap.get_mut(id).kind
                {
                    *index = i.saturating_add(1);
                }
                let key = Value::Number(position);
                Ok(Some(match kind {
                    IndexedKind::Keys => key,
                    IndexedKind::Values => self.get_element(&target, &key)?,
                    IndexedKind::Entries => {
                        let value = self.get_element(&target, &key)?;
                        self.new_array(vec![key, value])
                    }
                }))
            }
            IterState::Protocol {
                iterator,
                next,
                done,
            } => {
                if *done {
                    return Ok(None);
                }
                let (iterator, next) = (iterator.clone(), next.clone());
                let outcome = self.protocol_step(&iterator, &next);
                // Épuisé, ou en erreur : l'itérateur ne sera ni rappelé ni fermé.
                if !matches!(outcome, Ok(Some(_)))
                    && let Kind::Iterator(IterState::Protocol { done, .. }) =
                        &mut self.heap.get_mut(id).kind
                {
                    *done = true;
                }
                outcome
            }
        }
    }

    /// Un pas du protocole d'itération : `next()` puis lecture du résultat.
    fn protocol_step(&mut self, iterator: &Value, next: &Value) -> JsResult<Option<Value>> {
        let result = self.call(next, iterator, &[])?;
        if result.as_object().is_none() {
            return Err(self.type_error("le résultat d'un itérateur doit être un objet"));
        }
        if self.get(&result, "done")?.truthy() {
            return Ok(None);
        }
        self.get(&result, "value").map(Some)
    }

    /// Objet `{ value, done }` rendu par la méthode `next` d'un itérateur.
    pub(crate) fn iter_result(&mut self, value: Value, done: bool) -> Value {
        let id = self.new_object();
        let object = self.heap.get_mut(id);
        object
            .props
            .insert(Arc::from("value"), Property::data(value));
        object
            .props
            .insert(Arc::from("done"), Property::data(Value::Bool(done)));
        Value::Object(id)
    }

    /// Toutes les valeurs d'un itérable.
    pub(crate) fn iterate_to_vec(&mut self, source: &Value) -> JsResult<Vec<Value>> {
        // Chemin rapide : tableau sans trou, itéré de la façon ordinaire.
        if let Some(id) = source.as_object()
            && let Kind::Array(items) = &self.heap.get(id).kind
            && !items.contains(&Value::Hole)
            && self.array_iteration_intact(id)
        {
            return Ok(items.clone());
        }
        let iterator = self.iterator_for(source)?;
        let mut values = Vec::new();
        while let Some(value) = self.iterator_next(&iterator)? {
            values.push(value);
        }
        Ok(values)
    }

    /// Vrai si `value` peut être utilisé avec `new`.
    pub(crate) fn is_constructor(&self, value: &Value) -> bool {
        let Some(id) = value.as_object() else {
            return false;
        };
        match &self.heap.get(id).kind {
            Kind::Native { constructor, .. } => *constructor,
            Kind::Closure(c) => c.proto.is_constructor(),
            Kind::Bound { target, .. } => self.is_constructor(&Value::Object(*target)),
            Kind::Proxy(proxy) => proxy.constructor,
            _ => false,
        }
    }

    /// Représentation lisible d'une valeur, pour les messages et la console.
    pub fn display(&mut self, value: &Value) -> String {
        self.display_depth(value, 0)
    }

    fn display_depth(&mut self, value: &Value, depth: usize) -> String {
        match value {
            Value::Undefined | Value::Hole => "undefined".to_owned(),
            Value::Null => "null".to_owned(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => number_to_string(*n),
            Value::String(s) if depth == 0 => s.to_string(),
            Value::String(s) => format!("'{s}'"),
            Value::Symbol(s) => format!("Symbol({})", s.description().unwrap_or("")),
            Value::BigInt(b) => format!("{b}n"),
            Value::Object(id) => {
                let object: &Object = self.heap.get(*id);
                match &object.kind {
                    Kind::Closure(_)
                    | Kind::Native { .. }
                    | Kind::NativeData { .. }
                    | Kind::Bound { .. } => {
                        let name = self
                            .get(value, "name")
                            .ok()
                            .map(|n| self.display_depth(&n, 0))
                            .unwrap_or_default();
                        format!(
                            "[Function: {}]",
                            if name.is_empty() { "(anonyme)" } else { &name }
                        )
                    }
                    Kind::Error => {
                        let name = self
                            .get(value, "name")
                            .ok()
                            .map_or_else(String::new, |n| self.display_depth(&n, 0));
                        let message = self
                            .get(value, "message")
                            .ok()
                            .map_or_else(String::new, |m| self.display_depth(&m, 0));
                        if message.is_empty() {
                            name
                        } else {
                            format!("{name}: {message}")
                        }
                    }
                    Kind::Array(items) if depth < 3 => {
                        let items = items.clone();
                        let shown: Vec<String> = items
                            .iter()
                            .take(100)
                            .map(|v| self.display_depth(v, depth + 1))
                            .collect();
                        format!("[{}]", shown.join(", "))
                    }
                    Kind::Array(_) => "[Array]".to_owned(),
                    Kind::Host { .. } => "[object Host]".to_owned(),
                    Kind::Promise(_) => "[object Promise]".to_owned(),
                    Kind::Date(_) => self
                        .to_string(value)
                        .map_or_else(|_| "[object Date]".to_owned(), |s| s.to_string()),
                    Kind::RegExp { .. } => self
                        .to_string(value)
                        .map_or_else(|_| "[object RegExp]".to_owned(), |s| s.to_string()),
                    Kind::Coroutine(_) => "[object Generator]".to_owned(),
                    _ if depth < 3 => {
                        let keys = self.own_keys(*id, true);
                        let mut parts = Vec::new();
                        for key in keys.iter().take(50) {
                            let shown =
                                match self.heap.get(*id).props.get(key).map(|p| p.slot.clone()) {
                                    Some(Slot::Data(v)) => self.display_depth(&v, depth + 1),
                                    _ => "[Accesseur]".to_owned(),
                                };
                            parts.push(format!("{key}: {shown}"));
                        }
                        if parts.is_empty() {
                            "{}".to_owned()
                        } else {
                            format!("{{ {} }}", parts.join(", "))
                        }
                    }
                    _ => "[Object]".to_owned(),
                }
            }
        }
    }
}
