//! `JSON.parse`, `JSON.stringify`, `JSON.rawJSON` et `JSON.isRawJSON`
//! (ECMA-262 §25.5 et proposition « JSON.parse source text access »).
//!
//! LIMITE CONNUE : une chaîne ne peut pas porter de substitut isolé, que
//! `JSON.stringify` devrait écrire `\udXXX`.

use std::sync::Arc;

use crate::heap::{Kind, Object};
use crate::internal::same_value;
use crate::value::{JsStr, ObjId, PropertyKey, Value, number_to_string};
use crate::vm::{JsResult, Vm};

use super::array::{is_array, length_of};
use super::{arg, is_callable};

/// Profondeur maximale d'imbrication acceptée dans les deux sens.
const MAX_DEPTH: usize = 500;

/// Instantané d'une valeur analysée (`JSON Parse Record`) : sert à donner
/// à la fonction `reviver` le texte source des primitives.
struct Record {
    value: Value,
    /// Texte source d'une primitive.
    source: Option<JsStr>,
    elements: Vec<Record>,
    entries: Vec<(JsStr, Record)>,
}

struct Parser<'a> {
    bytes: &'a [u8],
    text: &'a str,
    pos: usize,
    /// Bâtir les instantanés (une fonction `reviver` est fournie).
    records: bool,
}

impl Parser<'_> {
    fn error(&self, vm: &mut Vm, what: &str) -> Value {
        vm.error(
            "SyntaxError",
            &format!("JSON invalide : {what} (position {})", self.pos),
        )
    }

    fn skip_ws(&mut self) {
        while matches!(self.bytes.get(self.pos), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    fn literal(&mut self, vm: &mut Vm, word: &str, value: Value) -> JsResult {
        if self
            .text
            .get(self.pos..)
            .is_some_and(|rest| rest.starts_with(word))
        {
            self.pos += word.len();
            return Ok(value);
        }
        Err(self.error(vm, "valeur inattendue"))
    }

    /// Analyse une valeur. Avec `self.records`, rend aussi son instantané.
    fn value(&mut self, vm: &mut Vm, depth: usize) -> JsResult<(Value, Option<Record>)> {
        if depth > MAX_DEPTH {
            return Err(self.error(vm, "imbrication trop profonde"));
        }
        self.skip_ws();
        let start = self.pos;
        let records = self.records;
        let container = |value: &Value, elements, entries| {
            records.then(|| Record {
                value: value.clone(),
                source: None,
                elements,
                entries,
            })
        };
        let value = match self.bytes.get(self.pos) {
            Some(b'{') => {
                self.pos += 1;
                let object = Value::Object(vm.new_object());
                let mut entries: Vec<(JsStr, Record)> = Vec::new();
                self.skip_ws();
                if self.bytes.get(self.pos) == Some(&b'}') {
                    self.pos += 1;
                    let record = container(&object, Vec::new(), entries);
                    return Ok((object, record));
                }
                loop {
                    self.skip_ws();
                    if self.bytes.get(self.pos) != Some(&b'"') {
                        return Err(self.error(vm, "nom de propriété attendu"));
                    }
                    let key: JsStr = Arc::from(self.string(vm)?);
                    self.skip_ws();
                    if self.bytes.get(self.pos) != Some(&b':') {
                        return Err(self.error(vm, "« : » attendu"));
                    }
                    self.pos += 1;
                    let (value, record) = self.value(vm, depth + 1)?;
                    // Définition directe : `__proto__` reste une propriété ordinaire.
                    if let Some(id) = object.as_object() {
                        vm.heap
                            .get_mut(id)
                            .props
                            .insert(Arc::clone(&key), crate::heap::Property::data(value));
                    }
                    if let Some(record) = record {
                        // Une clé répétée : la dernière valeur l'emporte.
                        entries.retain(|(name, _)| *name != key);
                        entries.push((key, record));
                    }
                    self.skip_ws();
                    match self.bytes.get(self.pos) {
                        Some(b',') => self.pos += 1,
                        Some(b'}') => {
                            self.pos += 1;
                            let record = container(&object, Vec::new(), entries);
                            return Ok((object, record));
                        }
                        _ => return Err(self.error(vm, "« , » ou « } » attendu")),
                    }
                }
            }
            Some(b'[') => {
                self.pos += 1;
                let mut items = Vec::new();
                let mut elements = Vec::new();
                self.skip_ws();
                if self.bytes.get(self.pos) != Some(&b']') {
                    loop {
                        let (item, record) = self.value(vm, depth + 1)?;
                        items.push(item);
                        elements.extend(record);
                        self.skip_ws();
                        match self.bytes.get(self.pos) {
                            Some(b',') => self.pos += 1,
                            Some(b']') => break,
                            _ => return Err(self.error(vm, "« , » ou « ] » attendu")),
                        }
                    }
                }
                self.pos += 1;
                let array = vm.new_array(items);
                let record = container(&array, elements, Vec::new());
                return Ok((array, record));
            }
            Some(b'"') => Value::string(&self.string(vm)?),
            Some(b't') => self.literal(vm, "true", Value::Bool(true))?,
            Some(b'f') => self.literal(vm, "false", Value::Bool(false))?,
            Some(b'n') => self.literal(vm, "null", Value::Null)?,
            Some(b'-' | b'0'..=b'9') => self.number(vm)?,
            _ => return Err(self.error(vm, "valeur attendue")),
        };
        // Primitive : son texte source accompagne l'instantané.
        let record = records.then(|| Record {
            value: value.clone(),
            source: self.text.get(start..self.pos).map(Arc::from),
            elements: Vec::new(),
            entries: Vec::new(),
        });
        Ok((value, record))
    }

    fn number(&mut self, vm: &mut Vm) -> JsResult {
        let start = self.pos;
        if self.bytes.get(self.pos) == Some(&b'-') {
            self.pos += 1;
        }
        let digits = |p: &mut Self| {
            let from = p.pos;
            while matches!(p.bytes.get(p.pos), Some(b'0'..=b'9')) {
                p.pos += 1;
            }
            p.pos - from
        };
        let int_start = self.pos;
        let int_len = digits(self);
        // Pas de zéro non significatif : « 01 » est invalide.
        if int_len == 0 || (int_len > 1 && self.bytes.get(int_start) == Some(&b'0')) {
            return Err(self.error(vm, "nombre invalide"));
        }
        if self.bytes.get(self.pos) == Some(&b'.') {
            self.pos += 1;
            if digits(self) == 0 {
                return Err(self.error(vm, "nombre invalide"));
            }
        }
        if matches!(self.bytes.get(self.pos), Some(b'e' | b'E')) {
            self.pos += 1;
            if matches!(self.bytes.get(self.pos), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            if digits(self) == 0 {
                return Err(self.error(vm, "nombre invalide"));
            }
        }
        match self
            .text
            .get(start..self.pos)
            .and_then(|s| s.parse::<f64>().ok())
        {
            Some(n) => Ok(Value::Number(n)),
            None => Err(self.error(vm, "nombre invalide")),
        }
    }

    fn hex4(&mut self) -> Option<u32> {
        let hex = self.text.get(self.pos..self.pos + 4)?;
        let value = u32::from_str_radix(hex, 16).ok()?;
        self.pos += 4;
        Some(value)
    }

    fn string(&mut self, vm: &mut Vm) -> JsResult<String> {
        self.pos += 1; // guillemet ouvrant
        let mut out = String::new();
        loop {
            let start = self.pos;
            // Segment sans échappement, copié d'un bloc.
            while self
                .bytes
                .get(self.pos)
                .is_some_and(|b| *b != b'"' && *b != b'\\' && *b >= 0x20)
            {
                self.pos += 1;
            }
            out.push_str(self.text.get(start..self.pos).unwrap_or(""));
            match self.bytes.get(self.pos) {
                Some(b'"') => {
                    self.pos += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.pos += 1;
                    let escape = self.bytes.get(self.pos).copied();
                    self.pos += 1;
                    match escape {
                        Some(b'"') => out.push('"'),
                        Some(b'\\') => out.push('\\'),
                        Some(b'/') => out.push('/'),
                        Some(b'b') => out.push('\u{8}'),
                        Some(b'f') => out.push('\u{c}'),
                        Some(b'n') => out.push('\n'),
                        Some(b'r') => out.push('\r'),
                        Some(b't') => out.push('\t'),
                        Some(b'u') => {
                            let Some(unit) = self.hex4() else {
                                return Err(self.error(vm, "échappement invalide"));
                            };
                            let mut code = unit;
                            // Paire de substituts écrite en deux échappements.
                            if (0xD800..0xDC00).contains(&unit)
                                && self
                                    .text
                                    .get(self.pos..)
                                    .is_some_and(|r| r.starts_with("\\u"))
                            {
                                let save = self.pos;
                                self.pos += 2;
                                match self.hex4() {
                                    Some(low) if (0xDC00..0xE000).contains(&low) => {
                                        code = 0x10000 + ((unit - 0xD800) << 10) + (low - 0xDC00);
                                    }
                                    _ => self.pos = save,
                                }
                            }
                            out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                        }
                        _ => return Err(self.error(vm, "échappement invalide")),
                    }
                }
                _ => return Err(self.error(vm, "chaîne non terminée")),
            }
        }
    }
}

/// Analyse `text` en entier comme un texte JSON.
fn parse_text(vm: &mut Vm, text: &str, records: bool) -> JsResult<(Value, Option<Record>)> {
    let mut parser = Parser {
        bytes: text.as_bytes(),
        text,
        pos: 0,
        records,
    };
    let parsed = parser.value(vm, 0)?;
    parser.skip_ws();
    if parser.pos != text.len() {
        return Err(parser.error(vm, "texte inattendu après la valeur"));
    }
    Ok(parsed)
}

fn text_key(name: &str) -> PropertyKey {
    PropertyKey::String(Arc::from(name))
}

impl Vm {
    /// Valeur du texte JSON `text` (module JSON) ; `SyntaxError` s'il est
    /// invalide.
    pub(crate) fn parse_json_text(&mut self, text: &str) -> JsResult {
        parse_text(self, text, false).map(|(value, _)| value)
    }
}

/// `JSON.parse(text, reviver)` (ECMA-262 §25.5.1).
fn json_parse(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let text = vm.to_string(&arg(args, 0))?;
    let reviver = arg(args, 1);
    let reviving = is_callable(vm, &reviver);
    let (value, record) = parse_text(vm, &text, reviving)?;
    if !reviving {
        return Ok(value);
    }
    let root = vm.new_object();
    vm.create_data_property(root, &text_key(""), value)?;
    internalize(
        vm,
        &Value::Object(root),
        &Arc::from(""),
        &reviver,
        record.as_ref(),
        0,
    )
}

/// `InternalizeJSONProperty` : fait passer `holder[name]` et, d'abord, tout
/// ce qu'elle contient par la fonction `reviver`.
fn internalize(
    vm: &mut Vm,
    holder: &Value,
    name: &JsStr,
    reviver: &Value,
    record: Option<&Record>,
    depth: usize,
) -> JsResult {
    if depth > MAX_DEPTH {
        return Err(vm.error("RangeError", "JSON.parse : imbrication trop profonde"));
    }
    let value = vm.get(holder, name)?;
    let context = vm.new_object();
    // L'instantané ne vaut que si la valeur n'a pas été remplacée entre-temps.
    let record = record.filter(|record| same_value(&record.value, &value));
    if let Some(record) = record
        && value.as_object().is_none()
        && let Some(source) = &record.source
    {
        vm.create_data_property(
            context,
            &text_key("source"),
            Value::String(Arc::clone(source)),
        )?;
    }
    if let Some(id) = value.as_object() {
        if is_array(vm, &value) {
            let len = length_of(vm, &value)?;
            for index in 0..len {
                vm.tick(1)?;
                let element = record.and_then(|record| record.elements.get(index));
                let key: JsStr = Arc::from(index.to_string());
                revive(vm, &value, id, &key, reviver, element, depth)?;
            }
        } else {
            for key in vm.enumerable_own_keys(id)? {
                let entry = record.and_then(|record| {
                    record
                        .entries
                        .iter()
                        .find(|(name, _)| *name == key)
                        .map(|(_, entry)| entry)
                });
                revive(vm, &value, id, &key, reviver, entry, depth)?;
            }
        }
    }
    vm.call(
        reviver,
        holder,
        &[
            Value::String(Arc::clone(name)),
            value,
            Value::Object(context),
        ],
    )
}

/// Remplace `value[key]` par ce qu'en fait `reviver` ; `undefined` la supprime.
fn revive(
    vm: &mut Vm,
    value: &Value,
    id: ObjId,
    key: &JsStr,
    reviver: &Value,
    record: Option<&Record>,
    depth: usize,
) -> JsResult<()> {
    let revived = internalize(vm, value, key, reviver, record, depth + 1)?;
    let key = PropertyKey::String(Arc::clone(key));
    if revived == Value::Undefined {
        vm.delete_property(id, &key)?;
    } else {
        vm.create_data_property(id, &key, revived)?;
    }
    Ok(())
}

/// `JSON.rawJSON(text)` : objet figé portant un texte JSON à émettre tel quel.
fn json_raw_json(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let text = vm.to_string(&arg(args, 0))?;
    let blank = |c: Option<char>| matches!(c, Some('\t' | '\n' | '\r' | ' '));
    if text.is_empty() || blank(text.chars().next()) || blank(text.chars().next_back()) {
        return Err(vm.error(
            "SyntaxError",
            "JSON.rawJSON : texte vide ou bordé de blancs",
        ));
    }
    let (value, _) = parse_text(vm, &text, false)?;
    if value.as_object().is_some() {
        return Err(vm.error(
            "SyntaxError",
            "JSON.rawJSON : un objet ou un tableau n'est pas accepté",
        ));
    }
    let object = vm.heap.alloc(Object::new(None, Kind::RawJson));
    vm.create_data_property(object, &text_key("rawJSON"), Value::String(text))?;
    super::freeze(vm, object)?;
    Ok(Value::Object(object))
}

fn json_is_raw_json(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    Ok(Value::Bool(arg(args, 0).as_object().is_some_and(|id| {
        matches!(vm.heap.get(id).kind, Kind::RawJson)
    })))
}

fn quote(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if u32::from(c) < 0x20 => out.push_str(&format!("\\u{:04x}", u32::from(c))),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// État de `JSON.stringify` (ECMA-262 §25.5.2).
struct Writer {
    /// Retrait ajouté à chaque niveau ; vide : sortie compacte.
    gap: String,
    /// Retrait du niveau courant.
    indent: String,
    /// Clés retenues, quand `replacer` est un tableau.
    property_list: Option<Vec<JsStr>>,
    /// Fonction `replacer`.
    replacer: Option<Value>,
    /// Objets en cours d'écriture, pour détecter les cycles.
    stack: Vec<ObjId>,
    out: String,
}

impl Writer {
    fn newline(&mut self) {
        if !self.gap.is_empty() {
            self.out.push('\n');
            self.out.push_str(&self.indent);
        }
    }

    /// `SerializeJSONProperty` : écrit `holder[key]` ; renvoie `false` si la
    /// valeur n'a pas de représentation JSON (`undefined`, fonction, symbole).
    fn property(&mut self, vm: &mut Vm, holder: &Value, key: &JsStr) -> JsResult<bool> {
        let mut value = vm.get(holder, key)?;
        if value.as_object().is_some() || matches!(value, Value::BigInt(_)) {
            let to_json = vm.get(&value, "toJSON")?;
            if is_callable(vm, &to_json) {
                value = vm.call(&to_json, &value, &[Value::String(Arc::clone(key))])?;
            }
        }
        if let Some(replacer) = &self.replacer {
            value = vm.call(replacer, holder, &[Value::String(Arc::clone(key)), value])?;
        }
        // Objets enveloppes : la primitive qu'ils portent.
        if let Some(id) = value.as_object() {
            let inner = match &vm.heap.get(id).kind {
                Kind::Primitive(inner) => Some(inner.clone()),
                _ => None,
            };
            match inner {
                Some(Value::Number(_)) => value = Value::Number(vm.to_number(&value)?),
                Some(Value::String(_)) => value = Value::String(vm.to_string(&value)?),
                Some(inner @ (Value::Bool(_) | Value::BigInt(_))) => value = inner,
                _ => {}
            }
        }
        match &value {
            Value::Null => self.out.push_str("null"),
            Value::Bool(b) => self.out.push_str(if *b { "true" } else { "false" }),
            Value::Number(n) if n.is_finite() => self.out.push_str(&number_to_string(*n)),
            Value::Number(_) => self.out.push_str("null"),
            Value::String(s) => quote(s, &mut self.out),
            Value::BigInt(_) => {
                return Err(vm.type_error("JSON.stringify ne sait pas sérialiser un BigInt"));
            }
            Value::Object(id) => {
                let id = *id;
                if vm.heap.get(id).is_callable() {
                    return Ok(false);
                }
                if matches!(vm.heap.get(id).kind, Kind::RawJson) {
                    let raw = vm.get(&value, "rawJSON")?;
                    let raw = vm.to_string(&raw)?;
                    self.out.push_str(&raw);
                } else {
                    self.container(vm, &value, id)?;
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// `SerializeJSONObject` et `SerializeJSONArray`.
    fn container(&mut self, vm: &mut Vm, value: &Value, id: ObjId) -> JsResult<()> {
        if self.stack.contains(&id) {
            return Err(vm.type_error("JSON.stringify : la structure contient un cycle"));
        }
        if self.stack.len() > MAX_DEPTH {
            return Err(vm.error("RangeError", "JSON.stringify : imbrication trop profonde"));
        }
        self.stack.push(id);
        let stepback = self.indent.clone();
        self.indent.push_str(&self.gap);
        let written = self.members(vm, value, id);
        self.stack.pop();
        self.indent = stepback;
        let (close, empty) = written?;
        if !empty {
            self.newline();
        }
        self.out.push(close);
        Ok(())
    }

    /// Écrit l'ouverture et les membres ; rend la fermeture et « aucun membre ».
    fn members(&mut self, vm: &mut Vm, value: &Value, id: ObjId) -> JsResult<(char, bool)> {
        if is_array(vm, value) {
            self.out.push('[');
            let len = length_of(vm, value)?;
            for index in 0..len {
                vm.tick(1)?;
                if index > 0 {
                    self.out.push(',');
                }
                self.newline();
                let key: JsStr = Arc::from(index.to_string());
                if !self.property(vm, value, &key)? {
                    self.out.push_str("null");
                }
            }
            return Ok((']', len == 0));
        }
        self.out.push('{');
        let keys = match &self.property_list {
            Some(list) => list.clone(),
            None => vm.enumerable_own_keys(id)?,
        };
        let mut empty = true;
        for key in keys {
            let checkpoint = self.out.len();
            if !empty {
                self.out.push(',');
            }
            self.newline();
            quote(&key, &mut self.out);
            self.out.push(':');
            if !self.gap.is_empty() {
                self.out.push(' ');
            }
            if self.property(vm, value, &key)? {
                empty = false;
            } else {
                // Propriété sans représentation : on retire ce qui vient d'être écrit.
                self.out.truncate(checkpoint);
            }
        }
        Ok(('}', empty))
    }
}

/// Primitive portée par un objet enveloppe `Number` ou `String`.
fn boxed_text_or_number(vm: &Vm, value: &Value) -> bool {
    value.as_object().is_some_and(|id| {
        matches!(
            vm.heap.get(id).kind,
            Kind::Primitive(Value::Number(_) | Value::String(_))
        )
    })
}

fn json_stringify(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let (value, replacer_arg, mut space) = (arg(args, 0), arg(args, 1), arg(args, 2));
    let mut replacer = None;
    let mut property_list = None;
    if replacer_arg.as_object().is_some() {
        if is_callable(vm, &replacer_arg) {
            replacer = Some(replacer_arg);
        } else if is_array(vm, &replacer_arg) {
            let mut list: Vec<JsStr> = Vec::new();
            for index in 0..length_of(vm, &replacer_arg)? {
                vm.tick(1)?;
                let item = vm.get(&replacer_arg, &index.to_string())?;
                let named = matches!(item, Value::String(_) | Value::Number(_))
                    || boxed_text_or_number(vm, &item);
                if named {
                    let name = vm.to_string(&item)?;
                    if !list.contains(&name) {
                        list.push(name);
                    }
                }
            }
            property_list = Some(list);
        }
    }
    if boxed_text_or_number(vm, &space) {
        let is_number = space
            .as_object()
            .is_some_and(|id| matches!(vm.heap.get(id).kind, Kind::Primitive(Value::Number(_))));
        space = if is_number {
            Value::Number(vm.to_number(&space)?)
        } else {
            Value::String(vm.to_string(&space)?)
        };
    }
    let gap = match &space {
        Value::Number(n) if !n.is_nan() => {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "ramené dans [0, 10]"
            )]
            let count = n.trunc().clamp(0.0, 10.0) as usize;
            " ".repeat(count)
        }
        Value::String(s) => s.chars().take(10).collect(),
        _ => String::new(),
    };
    let wrapper = vm.new_object();
    vm.create_data_property(wrapper, &text_key(""), value)?;
    let mut writer = Writer {
        gap,
        indent: String::new(),
        property_list,
        replacer,
        stack: Vec::new(),
        out: String::new(),
    };
    let wrote = writer.property(vm, &Value::Object(wrapper), &Arc::from(""))?;
    Ok(if wrote {
        Value::string(&writer.out)
    } else {
        Value::Undefined
    })
}

pub(super) fn install(vm: &mut Vm) {
    let json = vm.new_object();
    vm.define_method(json, "parse", 2, json_parse);
    vm.define_method(json, "stringify", 3, json_stringify);
    vm.define_method(json, "rawJSON", 1, json_raw_json);
    vm.define_method(json, "isRawJSON", 1, json_is_raw_json);
    let tag = vm.well_known.to_string_tag.clone();
    vm.define_hidden_symbol(json, &tag, Value::string("JSON"));
    vm.define_hidden(vm.global, "JSON", Value::Object(json));
}
