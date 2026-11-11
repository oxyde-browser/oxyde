//! `JSON.parse` et `JSON.stringify` (ECMA-262 §25.5).
//!
//! LIMITES CONNUES : `JSON.parse` ignore la fonction de rappel `reviver` ;
//! `JSON.stringify` n'accepte comme `replacer` qu'une liste de clés.

use std::sync::Arc;

use crate::heap::Kind;
use crate::value::{JsStr, ObjId, Value, number_to_string};
use crate::vm::{JsResult, Vm};

use super::{arg, is_callable};

/// Profondeur maximale d'imbrication acceptée dans les deux sens.
const MAX_DEPTH: usize = 500;

struct Parser<'a> {
    bytes: &'a [u8],
    text: &'a str,
    pos: usize,
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

    fn value(&mut self, vm: &mut Vm, depth: usize) -> JsResult {
        if depth > MAX_DEPTH {
            return Err(self.error(vm, "imbrication trop profonde"));
        }
        self.skip_ws();
        match self.bytes.get(self.pos) {
            Some(b'{') => {
                self.pos += 1;
                let object = Value::Object(vm.new_object());
                self.skip_ws();
                if self.bytes.get(self.pos) == Some(&b'}') {
                    self.pos += 1;
                    return Ok(object);
                }
                loop {
                    self.skip_ws();
                    if self.bytes.get(self.pos) != Some(&b'"') {
                        return Err(self.error(vm, "nom de propriété attendu"));
                    }
                    let key = self.string(vm)?;
                    self.skip_ws();
                    if self.bytes.get(self.pos) != Some(&b':') {
                        return Err(self.error(vm, "« : » attendu"));
                    }
                    self.pos += 1;
                    let value = self.value(vm, depth + 1)?;
                    // Définition directe : `__proto__` reste une propriété ordinaire.
                    if let Some(id) = object.as_object() {
                        vm.heap
                            .get_mut(id)
                            .props
                            .insert(Arc::from(key), crate::heap::Property::data(value));
                    }
                    self.skip_ws();
                    match self.bytes.get(self.pos) {
                        Some(b',') => self.pos += 1,
                        Some(b'}') => {
                            self.pos += 1;
                            return Ok(object);
                        }
                        _ => return Err(self.error(vm, "« , » ou « } » attendu")),
                    }
                }
            }
            Some(b'[') => {
                self.pos += 1;
                let mut items = Vec::new();
                self.skip_ws();
                if self.bytes.get(self.pos) == Some(&b']') {
                    self.pos += 1;
                    return Ok(vm.new_array(items));
                }
                loop {
                    items.push(self.value(vm, depth + 1)?);
                    self.skip_ws();
                    match self.bytes.get(self.pos) {
                        Some(b',') => self.pos += 1,
                        Some(b']') => {
                            self.pos += 1;
                            return Ok(vm.new_array(items));
                        }
                        _ => return Err(self.error(vm, "« , » ou « ] » attendu")),
                    }
                }
            }
            Some(b'"') => Ok(Value::string(&self.string(vm)?)),
            Some(b't') => self.literal(vm, "true", Value::Bool(true)),
            Some(b'f') => self.literal(vm, "false", Value::Bool(false)),
            Some(b'n') => self.literal(vm, "null", Value::Null),
            Some(b'-' | b'0'..=b'9') => self.number(vm),
            _ => Err(self.error(vm, "valeur attendue")),
        }
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

fn json_parse(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let text = vm.to_string(&arg(args, 0))?;
    let mut parser = Parser {
        bytes: text.as_bytes(),
        text: &text,
        pos: 0,
    };
    let value = parser.value(vm, 0)?;
    parser.skip_ws();
    if parser.pos != text.len() {
        return Err(parser.error(vm, "texte inattendu après la valeur"));
    }
    Ok(value)
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

struct Writer {
    indent: String,
    allowed: Option<Vec<JsStr>>,
    /// Objets en cours d'écriture, pour détecter les cycles.
    stack: Vec<ObjId>,
    out: String,
}

impl Writer {
    fn newline(&mut self, depth: usize) {
        if !self.indent.is_empty() {
            self.out.push('\n');
            for _ in 0..depth {
                self.out.push_str(&self.indent);
            }
        }
    }

    /// Écrit `value` ; renvoie `false` si elle n'a pas de représentation JSON
    /// (`undefined`, fonction).
    fn value(
        &mut self,
        vm: &mut Vm,
        holder: &Value,
        key: &str,
        value: Value,
        depth: usize,
    ) -> JsResult<bool> {
        let mut value = value;
        // `toJSON` (dates, objets personnalisés).
        if value.as_object().is_some() {
            let to_json = vm.get(&value, "toJSON")?;
            if is_callable(vm, &to_json) {
                value = vm.call(&to_json, &value, &[Value::string(key)])?;
            }
        }
        let _ = holder;
        match &value {
            Value::Undefined | Value::Hole | Value::Symbol(_) => return Ok(false),
            Value::BigInt(_) => {
                return Err(vm.type_error("JSON.stringify ne sait pas sérialiser un BigInt"));
            }
            Value::Null => self.out.push_str("null"),
            Value::Bool(b) => self.out.push_str(if *b { "true" } else { "false" }),
            Value::Number(n) => {
                if n.is_finite() {
                    self.out.push_str(&number_to_string(*n));
                } else {
                    self.out.push_str("null");
                }
            }
            Value::String(s) => quote(s, &mut self.out),
            Value::Object(id) => {
                let id = *id;
                match &vm.heap.get(id).kind {
                    Kind::Closure(_)
                    | Kind::Native { .. }
                    | Kind::NativeData { .. }
                    | Kind::Bound { .. } => {
                        return Ok(false);
                    }
                    Kind::Primitive(inner) => {
                        let inner = inner.clone();
                        return self.value(vm, holder, key, inner, depth);
                    }
                    _ => {}
                }
                if self.stack.contains(&id) {
                    return Err(vm.type_error("JSON.stringify : la structure contient un cycle"));
                }
                if depth > MAX_DEPTH {
                    return Err(
                        vm.error("RangeError", "JSON.stringify : imbrication trop profonde")
                    );
                }
                self.stack.push(id);
                let result = self.container(vm, &value, id, depth);
                self.stack.pop();
                result?;
            }
        }
        Ok(true)
    }

    fn container(&mut self, vm: &mut Vm, value: &Value, id: ObjId, depth: usize) -> JsResult<()> {
        if let Kind::Array(items) = &vm.heap.get(id).kind {
            let items = items.clone();
            self.out.push('[');
            for (i, item) in items.into_iter().enumerate() {
                if i > 0 {
                    self.out.push(',');
                }
                self.newline(depth + 1);
                let item = if item == Value::Hole {
                    Value::Undefined
                } else {
                    item
                };
                if !self.value(vm, value, &i.to_string(), item, depth + 1)? {
                    self.out.push_str("null");
                }
            }
            if !matches!(&vm.heap.get(id).kind, Kind::Array(items) if items.is_empty()) {
                self.newline(depth);
            }
            self.out.push(']');
            return Ok(());
        }
        let keys = match &self.allowed {
            Some(allowed) => allowed.clone(),
            None => vm.own_keys(id, true),
        };
        self.out.push('{');
        let mut first = true;
        for key in keys {
            let property = vm.get(value, &key)?;
            let checkpoint = self.out.len();
            if !first {
                self.out.push(',');
            }
            self.newline(depth + 1);
            quote(&key, &mut self.out);
            self.out.push(':');
            if !self.indent.is_empty() {
                self.out.push(' ');
            }
            if self.value(vm, value, &key, property, depth + 1)? {
                first = false;
            } else {
                // Propriété sans représentation : on retire ce qui vient d'être écrit.
                self.out.truncate(checkpoint);
            }
        }
        if !first {
            self.newline(depth);
        }
        self.out.push('}');
        Ok(())
    }
}

fn json_stringify(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let indent = match arg(args, 2) {
        Value::Number(n) => {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "ramené dans [0, 10]"
            )]
            let count = n.clamp(0.0, 10.0) as usize;
            " ".repeat(count)
        }
        Value::String(s) => s.chars().take(10).collect(),
        _ => String::new(),
    };
    let allowed = match arg(args, 1)
        .as_object()
        .filter(|id| matches!(vm.heap.get(*id).kind, Kind::Array(_)))
    {
        Some(id) => {
            let mut keys = Vec::new();
            for item in vm.iterate_to_vec(&Value::Object(id))? {
                if matches!(item, Value::String(_) | Value::Number(_)) {
                    keys.push(vm.to_property_key(&item)?);
                }
            }
            Some(keys)
        }
        None => None,
    };
    let mut writer = Writer {
        indent,
        allowed,
        stack: Vec::new(),
        out: String::new(),
    };
    let wrote = writer.value(vm, &Value::Undefined, "", arg(args, 0), 0)?;
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
    vm.define_hidden(vm.global, "JSON", Value::Object(json));
}
