//! Conversions de types (ECMA-262 §7.1) et opérateurs (§13).

use std::sync::Arc;

use crate::ast::BinaryOp;
use crate::bigint::BigInt;
use crate::bytecode::UnaryKind;
use crate::heap::Kind;
use crate::value::{
    JsStr, PropertyKey, Value, number_to_string, string_to_number, to_int32, to_uint32,
};

/// Type préféré d'une conversion en primitive (ECMA-262, « hint »).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Hint {
    /// Pas de préférence (`+`, `==`) : nombre, sauf pour les dates.
    Default,
    Number,
    String,
}
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
            Value::BigInt(_) => "bigint",
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
        let hint = if prefer_string {
            Hint::String
        } else {
            Hint::Number
        };
        self.to_primitive_hint(value, hint)
    }

    /// `ToPrimitive` avec son indication : la méthode `Symbol.toPrimitive` de
    /// l'objet l'emporte, sinon `valueOf` et `toString` sont essayées.
    pub(crate) fn to_primitive_hint(&mut self, value: &Value, hint: Hint) -> JsResult {
        if value.as_object().is_none() {
            return Ok(value.clone());
        }
        let key = PropertyKey::Symbol(self.well_known.to_primitive.clone());
        let exotic = self.get_key(value, &key)?;
        if !exotic.is_nullish() {
            let name = match hint {
                Hint::Default => "default",
                Hint::Number => "number",
                Hint::String => "string",
            };
            // Appeler une valeur non appelable lève la `TypeError` attendue.
            let result = self.call(&exotic, value, &[Value::string(name)])?;
            if result.as_object().is_some() {
                return Err(self.type_error("Symbol.toPrimitive a rendu un objet"));
            }
            return Ok(result);
        }
        self.ordinary_to_primitive(value, hint == Hint::String)
    }

    /// `OrdinaryToPrimitive` : `toString` puis `valueOf`, ou l'inverse.
    pub(crate) fn ordinary_to_primitive(&mut self, value: &Value, prefer_string: bool) -> JsResult {
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
            Value::BigInt(_) => {
                return Err(self.type_error(
                    "impossible de convertir un BigInt en nombre ; utiliser Number(valeur)",
                ));
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
            Value::BigInt(b) => Arc::from(b.to_string()),
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

    /// `ToNumeric` : nombre ou `BigInt`.
    pub(crate) fn to_numeric(&mut self, value: &Value) -> JsResult {
        match value {
            Value::BigInt(_) => Ok(value.clone()),
            Value::Object(_) => {
                let primitive = self.to_primitive(value, false)?;
                self.to_numeric(&primitive)
            }
            other => Ok(Value::Number(self.to_number(other)?)),
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
            Value::BigInt(_) => self.intrinsics.bigint_proto,
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
            (Value::BigInt(x), Value::Number(y)) | (Value::Number(y), Value::BigInt(x)) => {
                x.compare_f64(*y) == Some(std::cmp::Ordering::Equal)
            }
            (Value::BigInt(x), Value::String(s)) | (Value::String(s), Value::BigInt(x)) => {
                BigInt::parse_string(s).is_some_and(|parsed| parsed == **x)
            }
            (
                Value::Object(_),
                Value::Number(_) | Value::String(_) | Value::BigInt(_) | Value::Symbol(_),
            ) => {
                let primitive = self.to_primitive_hint(a, Hint::Default)?;
                return self.loose_equals(&primitive, b);
            }
            (
                Value::Number(_) | Value::String(_) | Value::BigInt(_) | Value::Symbol(_),
                Value::Object(_),
            ) => {
                let primitive = self.to_primitive_hint(b, Hint::Default)?;
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
        match (&a, &b) {
            (Value::BigInt(x), Value::BigInt(y)) => return Ok(Some(x < y)),
            // Une chaîne qui n'est pas un entier rend la comparaison indéfinie.
            (Value::BigInt(x), Value::String(s)) => {
                return Ok(BigInt::parse_string(s).map(|parsed| **x < parsed));
            }
            (Value::String(s), Value::BigInt(y)) => {
                return Ok(BigInt::parse_string(s).map(|parsed| parsed < **y));
            }
            (Value::BigInt(x), _) => {
                let n = self.to_number(&b)?;
                return Ok(x.compare_f64(n).map(std::cmp::Ordering::is_lt));
            }
            (_, Value::BigInt(y)) => {
                let n = self.to_number(&a)?;
                return Ok(y.compare_f64(n).map(std::cmp::Ordering::is_gt));
            }
            _ => {}
        }
        let (x, y) = (self.to_number(&a)?, self.to_number(&b)?);
        Ok(x.partial_cmp(&y).map(std::cmp::Ordering::is_lt))
    }

    /// `value instanceof constructor`.
    pub(crate) fn instance_of(&mut self, value: &Value, constructor: &Value) -> JsResult<bool> {
        if constructor.as_object().is_none() {
            return Err(self.type_error("l'opérande droit de instanceof n'est pas un objet"));
        }
        let key = PropertyKey::Symbol(self.well_known.has_instance.clone());
        let method = self.get_key(constructor, &key)?;
        if !method.is_nullish() {
            if !crate::builtins::is_callable(self, &method) {
                return Err(self.type_error("Symbol.hasInstance n'est pas une fonction"));
            }
            return Ok(self
                .call(&method, constructor, std::slice::from_ref(value))?
                .truthy());
        }
        if !crate::builtins::is_callable(self, constructor) {
            return Err(self.type_error("l'opérande droit de instanceof n'est pas une fonction"));
        }
        self.ordinary_has_instance(value, constructor)
    }

    /// `OrdinaryHasInstance` (ECMA-262 §7.3.21).
    pub(crate) fn ordinary_has_instance(
        &mut self,
        value: &Value,
        constructor: &Value,
    ) -> JsResult<bool> {
        let Some(ctor) = constructor
            .as_object()
            .filter(|c| self.heap.get(*c).is_callable())
        else {
            return Ok(false);
        };
        // Fonction liée : on teste contre la fonction d'origine.
        if let Kind::Bound { target, .. } = &self.heap.get(ctor).kind {
            let target = Value::Object(*target);
            return self.instance_of(value, &target);
        }
        let Some(mut current) = value.as_object() else {
            return Ok(false);
        };
        let Some(proto) = self.get(constructor, "prototype")?.as_object() else {
            return Err(self.type_error("le prototype du constructeur n'est pas un objet"));
        };
        for _ in 0..10_000 {
            match self.get_prototype_of(current)? {
                Some(p) if p == proto => return Ok(true),
                Some(p) => current = p,
                None => break,
            }
        }
        Ok(false)
    }

    pub(crate) fn unary(&mut self, op: UnaryKind, value: &Value) -> JsResult {
        Ok(match op {
            UnaryKind::Neg => match self.to_numeric(value)? {
                Value::BigInt(b) => Value::BigInt(Arc::new(b.neg())),
                other => Value::Number(-self.to_number(&other)?),
            },
            UnaryKind::Plus => Value::Number(self.to_number(value)?),
            UnaryKind::Not => Value::Bool(!value.truthy()),
            UnaryKind::BitNot => match self.to_numeric(value)? {
                Value::BigInt(b) => Value::BigInt(Arc::new(b.bit_not())),
                other => Value::Number(f64::from(!to_int32(self.to_number(&other)?))),
            },
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

    /// Opération entre deux `BigInt`.
    fn bigint_binary(&mut self, op: BinaryOp, x: &BigInt, y: &BigInt) -> JsResult {
        let shift = |amount: &BigInt, left: bool| match amount.to_i64() {
            Some(bits) => x.shift_left(if left { bits } else { bits.saturating_neg() }),
            // Décalage démesuré : zéro ou -1 vers la droite, trop grand vers la gauche.
            None if left != amount.is_negative() && !x.is_zero() => Err(crate::bigint::TooBig),
            None if x.is_negative() => Ok(BigInt::from_i64(-1)),
            None => Ok(BigInt::zero()),
        };
        let result = match op {
            BinaryOp::Add => x.add(y),
            BinaryOp::Sub => x.sub(y),
            BinaryOp::Mul => x.mul(y),
            BinaryOp::Div | BinaryOp::Rem => match x.div_rem(y) {
                Some((quotient, _)) if op == BinaryOp::Div => Ok(quotient),
                Some((_, remainder)) => Ok(remainder),
                None => return Err(self.error("RangeError", "division d'un BigInt par zéro")),
            },
            BinaryOp::Exp => {
                if y.is_negative() {
                    return Err(self.error("RangeError", "exposant BigInt négatif"));
                }
                match y.to_i64() {
                    Some(exponent) => x.pow(exponent.unsigned_abs()),
                    // Exposant démesuré : seuls 0, 1 et -1 restent calculables.
                    None if x.bits() <= 1 => {
                        let odd = !y.bit_and(&BigInt::from_u64(1)).is_zero();
                        Ok(if x.is_negative() && !odd {
                            x.neg()
                        } else {
                            x.clone()
                        })
                    }
                    None => Err(crate::bigint::TooBig),
                }
            }
            BinaryOp::Shl => shift(y, true),
            BinaryOp::Shr => shift(y, false),
            BinaryOp::UShr => {
                return Err(self.type_error("un BigInt n'a pas de décalage non signé (>>>)"));
            }
            BinaryOp::BitAnd => Ok(x.bit_and(y)),
            BinaryOp::BitOr => Ok(x.bit_or(y)),
            BinaryOp::BitXor => Ok(x.bit_xor(y)),
            _ => return Err(self.type_error("opération invalide sur un BigInt")),
        };
        match result {
            Ok(big) => Ok(Value::BigInt(Arc::new(big))),
            Err(_) => Err(self.error("RangeError", "BigInt trop grand")),
        }
    }

    fn binary_slow(&mut self, op: BinaryOp, a: &Value, b: &Value) -> JsResult {
        let arithmetic = matches!(
            op,
            BinaryOp::Add
                | BinaryOp::Sub
                | BinaryOp::Mul
                | BinaryOp::Div
                | BinaryOp::Rem
                | BinaryOp::Exp
                | BinaryOp::Shl
                | BinaryOp::Shr
                | BinaryOp::UShr
                | BinaryOp::BitAnd
                | BinaryOp::BitOr
                | BinaryOp::BitXor
        );
        let special = |v: &Value| matches!(v, Value::Object(_) | Value::BigInt(_));
        if arithmetic && (special(a) || special(b)) {
            // Seul `+` convertit sans préférence (une date y devient une chaîne).
            let hint = if op == BinaryOp::Add {
                Hint::Default
            } else {
                Hint::Number
            };
            // Les objets sont convertis une seule fois ; un `BigInt` ne se
            // mêle à aucun autre type numérique.
            let (x, y) = (
                self.to_primitive_hint(a, hint)?,
                self.to_primitive_hint(b, hint)?,
            );
            if op == BinaryOp::Add
                && (matches!(x, Value::String(_)) || matches!(y, Value::String(_)))
            {
                let (x, y) = (self.to_string(&x)?, self.to_string(&y)?);
                let mut out = String::with_capacity(x.len() + y.len());
                out.push_str(&x);
                out.push_str(&y);
                return Ok(Value::String(Arc::from(out)));
            }
            let (x, y) = (self.to_numeric(&x)?, self.to_numeric(&y)?);
            return match (&x, &y) {
                (Value::BigInt(p), Value::BigInt(q)) => self.bigint_binary(op, p, q),
                (Value::BigInt(_), _) | (_, Value::BigInt(_)) => Err(self.type_error(
                    "impossible de mêler BigInt et autres types ; utiliser des conversions explicites",
                )),
                _ => self.binary(op, &x, &y),
            };
        }
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
                Ok(Value::Bool(self.has_property_op(id, &key)?))
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
