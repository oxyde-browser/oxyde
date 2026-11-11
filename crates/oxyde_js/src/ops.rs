//! Conversions de types (ECMA-262 §7.1) et opérateurs (§13).

use std::sync::Arc;

use crate::ast::BinaryOp;
use crate::bytecode::UnaryKind;
use crate::heap::Kind;
use crate::value::{
    JsStr, PropertyKey, Value, number_to_string, string_to_number, to_int32, to_uint32,
};
use crate::vm::{JsResult, Vm};

impl Vm {
    /// Résultat de `typeof`.
    #[must_use]
    pub fn type_of(&self, value: &Value) -> &'static str {
        match value {
            Value::Undefined | Value::Hole => "undefined",
            Value::Null => "object",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Symbol(_) => "symbol",
            Value::Object(id) => {
                if self.heap.get(*id).is_callable() {
                    "function"
                } else {
                    "object"
                }
            }
        }
    }

    /// `ToPrimitive` : `prefer_string` choisit l'ordre `toString` / `valueOf`.
    pub(crate) fn to_primitive(&mut self, value: &Value, prefer_string: bool) -> JsResult {
        let Value::Object(id) = value else {
            return Ok(value.clone());
        };
        // Enveloppes : la valeur primitive est directement disponible.
        if let Kind::Primitive(inner) = &self.heap.get(*id).kind
            && !prefer_string
        {
            return Ok(inner.clone());
        }
        let order = if prefer_string {
            ["toString", "valueOf"]
        } else {
            ["valueOf", "toString"]
        };
        for name in order {
            let method = self.get(value, name)?;
            if method
                .as_object()
                .is_some_and(|m| self.heap.get(m).is_callable())
            {
                let result = self.call(&method, value, &[])?;
                if result.as_object().is_none() {
                    return Ok(result);
                }
            }
        }
        Err(self.type_error("impossible de convertir l'objet en valeur primitive"))
    }

    /// `ToNumber`.
    ///
    /// # Errors
    /// Toute valeur levée par `valueOf` ou `toString` d'un objet.
    pub fn to_number(&mut self, value: &Value) -> JsResult<f64> {
        Ok(match value {
            Value::Undefined | Value::Hole => f64::NAN,
            Value::Null => 0.0,
            Value::Bool(b) => f64::from(u8::from(*b)),
            Value::Number(n) => *n,
            Value::String(s) => string_to_number(s),
            Value::Symbol(_) => {
                return Err(self.type_error("impossible de convertir un symbole en nombre"));
            }
            Value::Object(_) => {
                let primitive = self.to_primitive(value, false)?;
                return self.to_number(&primitive);
            }
        })
    }

    /// `ToString`.
    ///
    /// # Errors
    /// Toute valeur levée par `toString` ou `valueOf` d'un objet.
    pub fn to_string(&mut self, value: &Value) -> JsResult<JsStr> {
        Ok(match value {
            Value::Undefined | Value::Hole => Arc::from("undefined"),
            Value::Null => Arc::from("null"),
            Value::Bool(b) => Arc::from(if *b { "true" } else { "false" }),
            Value::Number(n) => Arc::from(number_to_string(*n)),
            Value::String(s) => Arc::clone(s),
            Value::Symbol(_) => {
                return Err(self.type_error("impossible de convertir un symbole en chaîne"));
            }
            Value::Object(_) => {
                let primitive = self.to_primitive(value, true)?;
                return self.to_string(&primitive);
            }
        })
    }

    /// `ToPropertyKey` : chaîne ou symbole.
    ///
    /// # Errors
    /// Toute valeur levée par la conversion d'un objet en chaîne.
    pub fn to_key(&mut self, value: &Value) -> JsResult<PropertyKey> {
        match value {
            Value::Symbol(s) => Ok(PropertyKey::Symbol(s.clone())),
            other => self.to_property_key(other).map(PropertyKey::String),
        }
    }

    /// `ToPropertyKey` restreint aux clés chaînes (un symbole lève `TypeError`).
    pub(crate) fn to_property_key(&mut self, value: &Value) -> JsResult<JsStr> {
        match value {
            Value::String(s) => Ok(Arc::clone(s)),
            // Chemin rapide : petits entiers (index de tableau).
            Value::Number(n) if *n >= 0.0 && n.fract() == 0.0 && *n < 1e15 => {
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "entier positif borné"
                )]
                let index = *n as u64;
                Ok(Arc::from(index.to_string()))
            }
            other => self.to_string(other),
        }
    }

    /// `ToObject` : enveloppe une primitive.
    pub(crate) fn to_object(&mut self, value: &Value) -> JsResult {
        let proto = match value {
            Value::Object(_) => return Ok(value.clone()),
            Value::Undefined | Value::Null | Value::Hole => {
                return Err(self.type_error("impossible de convertir undefined ou null en objet"));
            }
            Value::Bool(_) => self.intrinsics.boolean_proto,
            Value::Number(_) => self.intrinsics.number_proto,
            Value::String(_) => self.intrinsics.string_proto,
            Value::Symbol(_) => self.intrinsics.symbol_proto,
        };
        let object = crate::heap::Object::new(Some(proto), Kind::Primitive(value.clone()));
        Ok(Value::Object(self.heap.alloc(object)))
    }

    /// Égalité stricte (`===`).
    #[must_use]
    pub fn strict_equals(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Number(x), Value::Number(y)) => x == y,
            (Value::Undefined | Value::Hole, Value::Undefined | Value::Hole) => true,
            _ => a == b,
        }
    }

    /// Égalité « SameValueZero » (`includes`, `Map`, `Set`).
    #[must_use]
    pub fn same_value_zero(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Number(x), Value::Number(y)) => x == y || (x.is_nan() && y.is_nan()),
            _ => Self::strict_equals(a, b),
        }
    }

    /// Égalité lâche (`==`).
    pub(crate) fn loose_equals(&mut self, a: &Value, b: &Value) -> JsResult<bool> {
        Ok(match (a, b) {
            (
                Value::Undefined | Value::Null | Value::Hole,
                Value::Undefined | Value::Null | Value::Hole,
            ) => true,
            (Value::Undefined | Value::Null | Value::Hole, _)
            | (_, Value::Undefined | Value::Null | Value::Hole) => false,
            (Value::Number(_), Value::String(s)) => {
                Self::strict_equals(a, &Value::Number(string_to_number(s)))
            }
            (Value::String(s), Value::Number(_)) => {
                Self::strict_equals(&Value::Number(string_to_number(s)), b)
            }
            (Value::Bool(x), _) => {
                return self.loose_equals(&Value::Number(f64::from(u8::from(*x))), b);
            }
            (_, Value::Bool(y)) => {
                return self.loose_equals(a, &Value::Number(f64::from(u8::from(*y))));
            }
            (Value::Object(_), Value::Number(_) | Value::String(_)) => {
                let primitive = self.to_primitive(a, false)?;
                return self.loose_equals(&primitive, b);
            }
            (Value::Number(_) | Value::String(_), Value::Object(_)) => {
                let primitive = self.to_primitive(b, false)?;
                return self.loose_equals(a, &primitive);
            }
            _ => Self::strict_equals(a, b),
        })
    }

    /// Comparaison `a < b` ; `None` si l'un des opérandes est `NaN`.
    fn less_than(&mut self, a: &Value, b: &Value) -> JsResult<Option<bool>> {
        let (a, b) = (self.to_primitive(a, false)?, self.to_primitive(b, false)?);
        if let (Value::String(x), Value::String(y)) = (&a, &b) {
            // Ordre des unités UTF-16, comme l'exige le langage.
            return Ok(Some(x.encode_utf16().lt(y.encode_utf16())));
        }
        let (x, y) = (self.to_number(&a)?, self.to_number(&b)?);
        Ok(x.partial_cmp(&y).map(std::cmp::Ordering::is_lt))
    }

    /// `value instanceof constructor`.
    pub(crate) fn instance_of(&mut self, value: &Value, constructor: &Value) -> JsResult<bool> {
        let Some(ctor) = constructor
            .as_object()
            .filter(|c| self.heap.get(*c).is_callable())
        else {
            return Err(self.type_error("l'opérande droit de instanceof n'est pas une fonction"));
        };
        // Fonction liée : on teste contre la fonction d'origine.
        if let Kind::Bound { target, .. } = &self.heap.get(ctor).kind {
            let target = Value::Object(*target);
            return self.instance_of(value, &target);
        }
        let Some(proto) = self.get(constructor, "prototype")?.as_object() else {
            return Ok(false);
        };
        let mut current = value.as_object().and_then(|id| self.heap.get(id).proto);
        for _ in 0..10_000 {
            match current {
                Some(p) if p == proto => return Ok(true),
                Some(p) => current = self.heap.get(p).proto,
                None => break,
            }
        }
        Ok(false)
    }

    pub(crate) fn unary(&mut self, op: UnaryKind, value: &Value) -> JsResult {
        Ok(match op {
            UnaryKind::Neg => Value::Number(-self.to_number(value)?),
            UnaryKind::Plus => Value::Number(self.to_number(value)?),
            UnaryKind::Not => Value::Bool(!value.truthy()),
            UnaryKind::BitNot => Value::Number(f64::from(!to_int32(self.to_number(value)?))),
            UnaryKind::Typeof => Value::string(self.type_of(value)),
        })
    }

    pub(crate) fn binary(&mut self, op: BinaryOp, a: &Value, b: &Value) -> JsResult {
        // Chemin rapide : arithmétique et comparaisons sur deux nombres.
        if let (Value::Number(x), Value::Number(y)) = (a, b) {
            let (x, y) = (*x, *y);
            return Ok(match op {
                BinaryOp::Add => Value::Number(x + y),
                BinaryOp::Sub => Value::Number(x - y),
                BinaryOp::Mul => Value::Number(x * y),
                BinaryOp::Div => Value::Number(x / y),
                BinaryOp::Rem => Value::Number(x % y),
                BinaryOp::Exp => Value::Number(js_pow(x, y)),
                BinaryOp::Lt => Value::Bool(x < y),
                BinaryOp::Le => Value::Bool(x <= y),
                BinaryOp::Gt => Value::Bool(x > y),
                BinaryOp::Ge => Value::Bool(x >= y),
                BinaryOp::Eq | BinaryOp::StrictEq => Value::Bool(x == y),
                BinaryOp::Ne | BinaryOp::StrictNe => Value::Bool(x != y),
                _ => return self.binary_slow(op, a, b),
            });
        }
        self.binary_slow(op, a, b)
    }

    fn binary_slow(&mut self, op: BinaryOp, a: &Value, b: &Value) -> JsResult {
        let number = |vm: &mut Self, f: fn(f64, f64) -> f64| -> JsResult {
            let (x, y) = (vm.to_number(a)?, vm.to_number(b)?);
            Ok(Value::Number(f(x, y)))
        };
        let int = |vm: &mut Self, f: fn(i32, u32) -> f64| -> JsResult {
            let (x, y) = (to_int32(vm.to_number(a)?), to_uint32(vm.to_number(b)?));
            Ok(Value::Number(f(x, y)))
        };
        match op {
            BinaryOp::Add => {
                let (x, y) = (self.to_primitive(a, false)?, self.to_primitive(b, false)?);
                if matches!(x, Value::String(_)) || matches!(y, Value::String(_)) {
                    let (x, y) = (self.to_string(&x)?, self.to_string(&y)?);
                    let mut out = String::with_capacity(x.len() + y.len());
                    out.push_str(&x);
                    out.push_str(&y);
                    Ok(Value::String(Arc::from(out)))
                } else {
                    Ok(Value::Number(self.to_number(&x)? + self.to_number(&y)?))
                }
            }
            BinaryOp::Sub => number(self, |x, y| x - y),
            BinaryOp::Mul => number(self, |x, y| x * y),
            BinaryOp::Div => number(self, |x, y| x / y),
            BinaryOp::Rem => number(self, |x, y| x % y),
            BinaryOp::Exp => number(self, js_pow),
            BinaryOp::Shl => int(self, |x, y| f64::from(x.wrapping_shl(y & 31))),
            BinaryOp::Shr => int(self, |x, y| f64::from(x.wrapping_shr(y & 31))),
            BinaryOp::UShr => int(self, |x, y| {
                f64::from(x.cast_unsigned().wrapping_shr(y & 31))
            }),
            BinaryOp::BitAnd => int(self, |x, y| f64::from(x & y.cast_signed())),
            BinaryOp::BitOr => int(self, |x, y| f64::from(x | y.cast_signed())),
            BinaryOp::BitXor => int(self, |x, y| f64::from(x ^ y.cast_signed())),
            BinaryOp::Eq => Ok(Value::Bool(self.loose_equals(a, b)?)),
            BinaryOp::Ne => Ok(Value::Bool(!self.loose_equals(a, b)?)),
            BinaryOp::StrictEq => Ok(Value::Bool(Self::strict_equals(a, b))),
            BinaryOp::StrictNe => Ok(Value::Bool(!Self::strict_equals(a, b))),
            BinaryOp::Lt => Ok(Value::Bool(self.less_than(a, b)? == Some(true))),
            BinaryOp::Gt => Ok(Value::Bool(self.less_than(b, a)? == Some(true))),
            // `a <= b` vaut `!(b < a)`, sauf en présence de NaN.
            BinaryOp::Le => Ok(Value::Bool(self.less_than(b, a)? == Some(false))),
            BinaryOp::Ge => Ok(Value::Bool(self.less_than(a, b)? == Some(false))),
            BinaryOp::In => {
                let Some(id) = b.as_object() else {
                    return Err(self.type_error("l'opérande droit de « in » n'est pas un objet"));
                };
                let key = self.to_key(a)?;
                Ok(Value::Bool(self.has_key(id, &key)))
            }
            BinaryOp::Instanceof => Ok(Value::Bool(self.instance_of(a, b)?)),
        }
    }
}

/// `x ** y` avec les cas particuliers d'ECMAScript (§6.1.6.1.3).
fn js_pow(x: f64, y: f64) -> f64 {
    if y.is_nan() || (x.abs() == 1.0 && y.is_infinite()) {
        return f64::NAN;
    }
    x.powf(y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_and_same_value_zero() {
        let nan = Value::Number(f64::NAN);
        assert!(!Vm::strict_equals(&nan, &nan));
        assert!(Vm::same_value_zero(&nan, &nan));
        assert!(Vm::strict_equals(&Value::Number(0.0), &Value::Number(-0.0)));
        assert!(!Vm::strict_equals(&Value::Null, &Value::Undefined));
        assert!(Vm::strict_equals(&Value::string("a"), &Value::string("a")));
    }

    #[test]
    fn power_special_cases() {
        assert!(js_pow(1.0, f64::INFINITY).is_nan());
        assert!(js_pow(2.0, f64::NAN).is_nan());
        assert_eq!(js_pow(2.0, 10.0), 1024.0);
        assert_eq!(js_pow(f64::NAN, 0.0), 1.0);
    }
}
