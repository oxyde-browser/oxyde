//! `String`, `Number` et `Math`.
//!
//! LIMITES CONNUES : pas de `normalize`, `toPrecision` ni
//! `toExponential` ; `localeCompare` compare les unités UTF-16 sans tenir
//! compte de la langue.

use std::sync::Arc;

use crate::object::char_at;
use crate::value::{JsStr, Value, number_to_string, to_int32, to_uint32, utf16_len};
use crate::vm::{JsResult, NativeFn, Vm};

use super::{
    arg, constructor, global_parse_float, global_parse_int, is_callable, number_to_radix,
    this_primitive,
};

/// Longueur maximale d'une chaîne produite par une méthode.
pub(super) const MAX_STRING: usize = 1 << 28;

fn this_str(vm: &mut Vm, this: &Value) -> JsResult<JsStr> {
    if this.is_nullish() {
        return Err(vm.type_error("méthode de chaîne appelée sur null ou undefined"));
    }
    vm.to_string(this)
}

fn string(s: &str) -> JsResult {
    Ok(Value::string(s))
}

/// `CreateHTML` (ECMA-262 §B.2.2.2.1) : entoure la chaîne d'une balise, avec
/// un attribut éventuel dont les guillemets sont échappés.
fn create_html(
    vm: &mut Vm,
    this: &Value,
    tag: &str,
    attribute: Option<(&str, &Value)>,
) -> JsResult {
    let text = this_str(vm, this)?;
    let mut out = format!("<{tag}");
    if let Some((name, value)) = attribute {
        let value = vm.to_string(value)?;
        out.push_str(&format!(" {name}=\"{}\"", value.replace('"', "&quot;")));
    }
    out.push_str(&format!(">{text}</{tag}>"));
    string(&out)
}

/// Méthodes HTML de `String.prototype` (annexe B) : `balise` sans attribut,
/// ou `balise attribut` dont la valeur est le premier argument.
macro_rules! html_methods {
    ($($name:ident => $tag:literal $(, $attribute:literal)?;)*) => {
        $(
            fn $name(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
                let _ = args;
                let attribute: Option<(&str, Value)> = None $(.or(Some(($attribute, arg(args, 0)))))?;
                create_html(vm, this, $tag, attribute.as_ref().map(|(name, value)| (*name, value)))
            }
        )*
    };
}

html_methods! {
    html_anchor => "a", "name";
    html_big => "big";
    html_blink => "blink";
    html_bold => "b";
    html_fixed => "tt";
    html_fontcolor => "font", "color";
    html_fontsize => "font", "size";
    html_italics => "i";
    html_link => "a", "href";
    html_small => "small";
    html_strike => "strike";
    html_sub => "sub";
    html_sup => "sup";
}

#[expect(
    clippy::cast_precision_loss,
    reason = "longueur de chaîne petite devant 2^53"
)]
fn number(n: usize) -> Value {
    Value::Number(n as f64)
}

/// Position entière d'un argument, ramenée dans `[0, len]`.
fn position(vm: &mut Vm, value: &Value, len: usize, default: usize) -> JsResult<usize> {
    if *value == Value::Undefined {
        return Ok(default);
    }
    let n = vm.to_number(value)?;
    #[expect(clippy::cast_precision_loss, reason = "longueur petite devant 2^53")]
    let clamped = if n.is_nan() {
        0.0
    } else {
        n.trunc().clamp(0.0, len as f64)
    };
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "ramené dans [0, len]"
    )]
    Ok(clamped as usize)
}

/// Sous-chaîne entre deux positions exprimées en unités UTF-16.
fn substring_units(s: &str, start: usize, end: usize) -> String {
    if s.is_ascii() {
        return s.get(start..end.max(start)).unwrap_or("").to_owned();
    }
    let units: Vec<u16> = s
        .encode_utf16()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect();
    String::from_utf16_lossy(&units)
}

/// Position en unités UTF-16 de l'octet `byte` de `s`.
fn units_before(s: &str, byte: usize) -> usize {
    utf16_len(s.get(..byte).unwrap_or(s))
}

/// Position en octets de l'unité UTF-16 `unit` de `s`.
fn byte_of_unit(s: &str, unit: usize) -> usize {
    if s.is_ascii() {
        return unit.min(s.len());
    }
    let mut units = 0;
    for (byte, c) in s.char_indices() {
        if units >= unit {
            return byte;
        }
        units += c.len_utf16();
    }
    s.len()
}

fn string_ctor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let value = match args.first() {
        None => Value::string(""),
        // Appelé comme fonction, `String` décrit un symbole au lieu de lever.
        Some(Value::Symbol(symbol)) if !vm.constructing => {
            Value::string(&format!("Symbol({})", symbol.description().unwrap_or("")))
        }
        Some(v) => Value::String(vm.to_string(v)?),
    };
    if vm.constructing {
        vm.to_object(&value)
    } else {
        Ok(value)
    }
}

fn from_char_code(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let mut units = Vec::with_capacity(args.len());
    for value in args {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "ToUint16 : troncature voulue"
        )]
        units.push(to_uint32(vm.to_number(value)?) as u16);
    }
    string(&String::from_utf16_lossy(&units))
}

fn from_code_point(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let mut out = String::new();
    for value in args {
        let n = vm.to_number(value)?;
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "validé par la garde"
        )]
        let c = if n >= 0.0 && n.fract() == 0.0 && n <= 1_114_111.0 {
            char::from_u32(n as u32)
        } else {
            None
        };
        match c {
            Some(c) => out.push(c),
            None => return Err(vm.error("RangeError", "point de code invalide")),
        }
    }
    string(&out)
}

fn char_at_method(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let s = this_str(vm, this)?;
    let index = position(vm, &arg(args, 0), usize::MAX >> 12, 0)?;
    string(&char_at(&s, index).unwrap_or_default())
}

fn char_code_at(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let s = this_str(vm, this)?;
    let index = position(vm, &arg(args, 0), usize::MAX >> 12, 0)?;
    Ok(s.encode_utf16()
        .nth(index)
        .map_or(Value::Number(f64::NAN), |u| Value::Number(f64::from(u))))
}

fn code_point_at(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let s = this_str(vm, this)?;
    let index = position(vm, &arg(args, 0), usize::MAX >> 12, 0)?;
    let byte = byte_of_unit(&s, index);
    Ok(s.get(byte..)
        .and_then(|rest| rest.chars().next())
        .map_or(Value::Undefined, |c| Value::Number(f64::from(u32::from(c)))))
}

fn at(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let s = this_str(vm, this)?;
    let n = vm.to_number(&arg(args, 0))?;
    #[expect(clippy::cast_precision_loss, reason = "longueur petite devant 2^53")]
    let index = if n < 0.0 {
        utf16_len(&s) as f64 + n.trunc()
    } else {
        n.trunc()
    };
    if index < 0.0 || index.is_nan() {
        return Ok(Value::Undefined);
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "positif vérifié"
    )]
    Ok(char_at(&s, index as usize).map_or(Value::Undefined, |c| Value::string(&c)))
}

fn index_of(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let s = this_str(vm, this)?;
    let needle = vm.to_string(&arg(args, 0))?;
    let from = position(vm, &arg(args, 1), utf16_len(&s), 0)?;
    let start = byte_of_unit(&s, from);
    let found = s
        .get(start..)
        .and_then(|rest| rest.find(&*needle))
        .map(|i| units_before(&s, start + i));
    Ok(found.map_or(Value::Number(-1.0), number))
}

fn last_index_of(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let s = this_str(vm, this)?;
    let needle = vm.to_string(&arg(args, 0))?;
    Ok(s.rfind(&*needle)
        .map_or(Value::Number(-1.0), |i| number(units_before(&s, i))))
}

fn includes(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let s = this_str(vm, this)?;
    let needle = vm.to_string(&arg(args, 0))?;
    let from = position(vm, &arg(args, 1), utf16_len(&s), 0)?;
    Ok(Value::Bool(
        s.get(byte_of_unit(&s, from)..)
            .is_some_and(|rest| rest.contains(&*needle)),
    ))
}

fn starts_with(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let s = this_str(vm, this)?;
    let needle = vm.to_string(&arg(args, 0))?;
    let from = position(vm, &arg(args, 1), utf16_len(&s), 0)?;
    Ok(Value::Bool(
        s.get(byte_of_unit(&s, from)..)
            .is_some_and(|rest| rest.starts_with(&*needle)),
    ))
}

fn ends_with(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let s = this_str(vm, this)?;
    let needle = vm.to_string(&arg(args, 0))?;
    let len = utf16_len(&s);
    let end = position(vm, &arg(args, 1), len, len)?;
    Ok(Value::Bool(
        s.get(..byte_of_unit(&s, end))
            .is_some_and(|head| head.ends_with(&*needle)),
    ))
}

fn slice(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let s = this_str(vm, this)?;
    let len = utf16_len(&s);
    let start = super::relative_index(vm, &arg(args, 0), len, 0)?;
    let end = super::relative_index(vm, &arg(args, 1), len, len)?;
    string(&substring_units(&s, start, end))
}

fn substring(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let s = this_str(vm, this)?;
    let len = utf16_len(&s);
    let a = position(vm, &arg(args, 0), len, 0)?;
    let b = position(vm, &arg(args, 1), len, len)?;
    string(&substring_units(&s, a.min(b), a.max(b)))
}

fn substr(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let s = this_str(vm, this)?;
    let len = utf16_len(&s);
    let start = super::relative_index(vm, &arg(args, 0), len, 0)?;
    let count = position(vm, &arg(args, 1), len - start, len - start)?;
    string(&substring_units(&s, start, start + count))
}

fn to_upper_case(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    string(&this_str(vm, this)?.to_uppercase())
}

fn to_lower_case(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    string(&this_str(vm, this)?.to_lowercase())
}

fn is_js_space(c: char) -> bool {
    c.is_whitespace() || c == '\u{feff}'
}

fn trim(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    string(this_str(vm, this)?.trim_matches(is_js_space))
}

fn trim_start(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    string(this_str(vm, this)?.trim_start_matches(is_js_space))
}

fn trim_end(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    string(this_str(vm, this)?.trim_end_matches(is_js_space))
}

fn pad(vm: &mut Vm, this: &Value, args: &[Value], at_start: bool) -> JsResult {
    let s = this_str(vm, this)?;
    let target = vm.to_number(&arg(args, 0))?;
    let filler = match arg(args, 1) {
        Value::Undefined => Arc::from(" "),
        other => vm.to_string(&other)?,
    };
    let len = utf16_len(&s);
    #[expect(clippy::cast_precision_loss, reason = "longueur petite devant 2^53")]
    if target.is_nan() || target <= len as f64 || filler.is_empty() {
        return Ok(Value::String(s));
    }
    #[expect(clippy::cast_precision_loss, reason = "borne de validation")]
    if target > MAX_STRING as f64 {
        return Err(vm.error("RangeError", "chaîne trop longue"));
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "borné juste au-dessus"
    )]
    let missing = target as usize - len;
    let units: Vec<u16> = filler.encode_utf16().cycle().take(missing).collect();
    let padding = String::from_utf16_lossy(&units);
    string(&if at_start {
        format!("{padding}{s}")
    } else {
        format!("{s}{padding}")
    })
}

fn pad_start(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    pad(vm, this, args, true)
}

fn pad_end(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    pad(vm, this, args, false)
}

fn repeat(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let s = this_str(vm, this)?;
    let count = vm.to_number(&arg(args, 0))?;
    #[expect(clippy::cast_precision_loss, reason = "borne de validation")]
    if count < 0.0 || count.is_infinite() || count * s.len() as f64 > MAX_STRING as f64 {
        return Err(vm.error("RangeError", "nombre de répétitions invalide"));
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "borné juste au-dessus"
    )]
    string(&s.repeat(if count.is_nan() { 0 } else { count as usize }))
}

fn concat(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let mut out = this_str(vm, this)?.to_string();
    for value in args {
        out.push_str(&vm.to_string(value)?);
    }
    string(&out)
}

fn split(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let s = this_str(vm, this)?;
    let limit = match arg(args, 1) {
        Value::Undefined => usize::MAX,
        other => to_uint32(vm.to_number(&other)?) as usize,
    };
    if let Some((_, regex)) = vm.regexp_parts(&arg(args, 0)) {
        return super::regexp::string_split(vm, &s, &regex, limit);
    }
    let parts: Vec<Value> = match arg(args, 0) {
        Value::Undefined => vec![Value::String(Arc::clone(&s))],
        separator => {
            let separator = vm.to_string(&separator)?;
            if separator.is_empty() {
                // Découpe en unités UTF-16, comme le prescrit le langage.
                (0..utf16_len(&s))
                    .filter_map(|i| char_at(&s, i))
                    .map(|c| Value::string(&c))
                    .take(limit)
                    .collect()
            } else {
                s.split(&*separator)
                    .take(limit)
                    .map(Value::string)
                    .collect()
            }
        }
    };
    Ok(vm.new_array(parts.into_iter().take(limit).collect()))
}

/// Texte de remplacement pour une occurrence : fonction appelée ou gabarit
/// (`$&`, `$$`, `` $` ``, `$'`). `start` : position en octets de l'occurrence.
fn replacement(
    vm: &mut Vm,
    replacer: &Value,
    matched: &str,
    start: usize,
    whole: &JsStr,
) -> JsResult<String> {
    let position = units_before(whole, start);
    if is_callable(vm, replacer) {
        let result = vm.call(
            replacer,
            &Value::Undefined,
            &[
                Value::string(matched),
                number(position),
                Value::String(Arc::clone(whole)),
            ],
        )?;
        return Ok(vm.to_string(&result)?.to_string());
    }
    let template = vm.to_string(replacer)?;
    let context = |after: bool| {
        if after {
            whole.get(start + matched.len()..).unwrap_or("").to_owned()
        } else {
            whole.get(..start).unwrap_or("").to_owned()
        }
    };
    Ok(super::regexp::expand_template(
        &template,
        matched,
        &[],
        &[],
        &context,
    ))
}

fn replace_impl(vm: &mut Vm, this: &Value, args: &[Value], all: bool) -> JsResult {
    let s = this_str(vm, this)?;
    let replacer = arg(args, 1);
    if let Some((regexp, regex)) = vm.regexp_parts(&arg(args, 0)) {
        return super::regexp::string_replace(vm, &s, regexp, &regex, &replacer, all);
    }
    let pattern = vm.to_string(&arg(args, 0))?;
    let mut out = String::new();
    let mut cursor = 0;
    while let Some(found) = s.get(cursor..).and_then(|rest| rest.find(&*pattern)) {
        let start = cursor + found;
        out.push_str(s.get(cursor..start).unwrap_or(""));
        out.push_str(&replacement(vm, &replacer, &pattern, start, &s)?);
        cursor = start + pattern.len();
        if !all {
            break;
        }
        if pattern.is_empty() {
            // Motif vide : on avance d'un caractère pour ne pas boucler.
            let Some(c) = s.get(cursor..).and_then(|rest| rest.chars().next()) else {
                break;
            };
            out.push(c);
            cursor += c.len_utf8();
        }
        if out.len() > MAX_STRING {
            return Err(vm.error("RangeError", "chaîne trop longue"));
        }
    }
    out.push_str(s.get(cursor..).unwrap_or(""));
    string(&out)
}

fn replace(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    replace_impl(vm, this, args, false)
}

fn replace_all(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    replace_impl(vm, this, args, true)
}

fn match_method(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let s = this_str(vm, this)?;
    super::regexp::string_match(vm, &s, args)
}

fn match_all(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let s = this_str(vm, this)?;
    super::regexp::string_match_all(vm, &s, args)
}

fn search(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let s = this_str(vm, this)?;
    super::regexp::string_search(vm, &s, args)
}

fn locale_compare(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let s = this_str(vm, this)?;
    let other = vm.to_string(&arg(args, 0))?;
    Ok(Value::Number(
        match s.encode_utf16().cmp(other.encode_utf16()) {
            std::cmp::Ordering::Less => -1.0,
            std::cmp::Ordering::Equal => 0.0,
            std::cmp::Ordering::Greater => 1.0,
        },
    ))
}

fn string_value_of(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    this_primitive(
        vm,
        this,
        |v| matches!(v, Value::String(_)),
        "String.prototype.valueOf",
    )
}

// ---- Number --------------------------------------------------------------------

fn number_ctor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let value = match args.first() {
        None => Value::Number(0.0),
        // `Number(1n)` est la conversion explicite d'un BigInt.
        Some(v) => match vm.to_numeric(v)? {
            Value::BigInt(big) => Value::Number(big.to_f64()),
            other => other,
        },
    };
    if vm.constructing {
        vm.to_object(&value)
    } else {
        Ok(value)
    }
}

fn this_number(vm: &mut Vm, this: &Value) -> JsResult<f64> {
    match this_primitive(
        vm,
        this,
        |v| matches!(v, Value::Number(_)),
        "Number.prototype",
    )? {
        Value::Number(n) => Ok(n),
        _ => Ok(f64::NAN),
    }
}

fn number_to_string_method(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let n = this_number(vm, this)?;
    let radix = match arg(args, 0) {
        Value::Undefined => 10.0,
        other => vm.to_number(&other)?,
    };
    if !(2.0..=36.0).contains(&radix) || radix.fract() != 0.0 {
        return Err(vm.error("RangeError", "la base doit être comprise entre 2 et 36"));
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "dans [2, 36]"
    )]
    string(&number_to_radix(n, radix as u32))
}

fn number_to_fixed(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let n = this_number(vm, this)?;
    let digits = vm.to_number(&arg(args, 0))?;
    let digits = if digits.is_nan() { 0.0 } else { digits.trunc() };
    if !(0.0..=100.0).contains(&digits) {
        return Err(vm.error(
            "RangeError",
            "toFixed : le nombre de décimales doit être entre 0 et 100",
        ));
    }
    if !n.is_finite() || n.abs() >= 1e21 {
        return string(&number_to_string(n));
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "dans [0, 100]"
    )]
    let digits = digits as usize;
    let formatted = format!("{n:.digits$}");
    // « -0.00 » s'écrit « 0.00 » en JavaScript quand la valeur est nulle.
    string(if n == 0.0 {
        formatted.trim_start_matches('-')
    } else {
        &formatted
    })
}

fn number_value_of(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    Ok(Value::Number(this_number(vm, this)?))
}

fn number_is_nan(_vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    Ok(Value::Bool(
        matches!(arg(args, 0), Value::Number(n) if n.is_nan()),
    ))
}

fn number_is_finite(_vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    Ok(Value::Bool(
        matches!(arg(args, 0), Value::Number(n) if n.is_finite()),
    ))
}

fn number_is_integer(_vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    Ok(Value::Bool(
        matches!(arg(args, 0), Value::Number(n) if n.is_finite() && n.fract() == 0.0),
    ))
}

fn number_is_safe_integer(_vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let safe = matches!(arg(args, 0), Value::Number(n) if n.is_finite() && n.fract() == 0.0 && n.abs() <= 9_007_199_254_740_991.0);
    Ok(Value::Bool(safe))
}

// ---- Math ----------------------------------------------------------------------

macro_rules! math_fn {
    ($name:ident, |$x:ident| $body:expr) => {
        fn $name(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
            let $x = vm.to_number(&arg(args, 0))?;
            Ok(Value::Number($body))
        }
    };
}
math_fn!(math_abs, |x| x.abs());
math_fn!(math_floor, |x| x.floor());
math_fn!(math_ceil, |x| x.ceil());
math_fn!(math_trunc, |x| x.trunc());
math_fn!(math_sqrt, |x| x.sqrt());
math_fn!(math_cbrt, |x| x.cbrt());
math_fn!(math_sin, |x| x.sin());
math_fn!(math_cos, |x| x.cos());
math_fn!(math_tan, |x| x.tan());
math_fn!(math_asin, |x| x.asin());
math_fn!(math_acos, |x| x.acos());
math_fn!(math_atan, |x| x.atan());
math_fn!(math_exp, |x| x.exp());
math_fn!(math_log, |x| x.ln());
math_fn!(math_log2, |x| x.log2());
math_fn!(math_log10, |x| x.log10());
math_fn!(math_fround, |x| {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "arrondi en simple précision voulu"
    )]
    let single = x as f32;
    f64::from(single)
});
math_fn!(math_sign, |x| if x.is_nan() || x == 0.0 {
    x
} else {
    x.signum()
});
// Arrondi « au plus proche, les demis vers +∞ », différent de `f64::round`.
math_fn!(math_round, |x| if x.fract() == -0.5 {
    x.ceil()
} else {
    x.round()
});
math_fn!(math_clz32, |x| f64::from(to_uint32(x).leading_zeros()));

fn math_pow(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let (x, y) = (arg(args, 0), arg(args, 1));
    let (x, y) = (
        Value::Number(vm.to_number(&x)?),
        Value::Number(vm.to_number(&y)?),
    );
    vm.binary(crate::ast::BinaryOp::Exp, &x, &y)
}

fn math_atan2(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let (y, x) = (vm.to_number(&arg(args, 0))?, vm.to_number(&arg(args, 1))?);
    Ok(Value::Number(y.atan2(x)))
}

fn math_imul(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let (a, b) = (
        to_int32(vm.to_number(&arg(args, 0))?),
        to_int32(vm.to_number(&arg(args, 1))?),
    );
    Ok(Value::Number(f64::from(a.wrapping_mul(b))))
}

fn math_min_max(vm: &mut Vm, args: &[Value], want_max: bool) -> JsResult {
    let mut result = if want_max {
        f64::NEG_INFINITY
    } else {
        f64::INFINITY
    };
    for value in args {
        let n = vm.to_number(value)?;
        if n.is_nan() {
            return Ok(Value::Number(f64::NAN));
        }
        // `f64::max` ignore le signe de zéro ; JavaScript distingue +0 et -0.
        let replace = if want_max {
            n > result || (n == 0.0 && result == 0.0 && n.is_sign_positive())
        } else {
            n < result || (n == 0.0 && result == 0.0 && n.is_sign_negative())
        };
        if replace {
            result = n;
        }
    }
    Ok(Value::Number(result))
}

fn math_min(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    math_min_max(vm, args, false)
}

fn math_max(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    math_min_max(vm, args, true)
}

fn math_hypot(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let mut sum = 0.0;
    for value in args {
        let n = vm.to_number(value)?;
        sum += n * n;
    }
    Ok(Value::Number(sum.sqrt()))
}

fn math_random(vm: &mut Vm, _this: &Value, _args: &[Value]) -> JsResult {
    // xorshift64* : suffisant pour `Math.random`, qui n'a aucune vocation cryptographique.
    let mut x = vm.rng;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    vm.rng = x;
    let bits = x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11;
    #[expect(
        clippy::cast_precision_loss,
        reason = "53 bits : exactement représentable"
    )]
    Ok(Value::Number(bits as f64 / 9_007_199_254_740_992.0))
}

pub(super) fn install(vm: &mut Vm) {
    let proto = vm.intrinsics.string_proto;
    let ctor = constructor(vm, "String", 1, string_ctor, proto);
    vm.define_method(ctor, "fromCharCode", 1, from_char_code);
    vm.define_method(ctor, "fromCodePoint", 1, from_code_point);
    let methods: &[(&str, u32, NativeFn)] = &[
        ("charAt", 1, char_at_method),
        ("charCodeAt", 1, char_code_at),
        ("codePointAt", 1, code_point_at),
        ("at", 1, at),
        ("indexOf", 1, index_of),
        ("lastIndexOf", 1, last_index_of),
        ("includes", 1, includes),
        ("startsWith", 1, starts_with),
        ("endsWith", 1, ends_with),
        ("slice", 2, slice),
        ("substring", 2, substring),
        ("substr", 2, substr),
        ("toUpperCase", 0, to_upper_case),
        ("toLowerCase", 0, to_lower_case),
        ("toLocaleUpperCase", 0, to_upper_case),
        ("toLocaleLowerCase", 0, to_lower_case),
        ("trim", 0, trim),
        ("trimStart", 0, trim_start),
        ("trimEnd", 0, trim_end),
        ("anchor", 1, html_anchor),
        ("big", 0, html_big),
        ("blink", 0, html_blink),
        ("bold", 0, html_bold),
        ("fixed", 0, html_fixed),
        ("fontcolor", 1, html_fontcolor),
        ("fontsize", 1, html_fontsize),
        ("italics", 0, html_italics),
        ("link", 1, html_link),
        ("small", 0, html_small),
        ("strike", 0, html_strike),
        ("sub", 0, html_sub),
        ("sup", 0, html_sup),
        ("padStart", 2, pad_start),
        ("padEnd", 2, pad_end),
        ("repeat", 1, repeat),
        ("concat", 1, concat),
        ("split", 2, split),
        ("replace", 2, replace),
        ("replaceAll", 2, replace_all),
        ("match", 1, match_method),
        ("matchAll", 1, match_all),
        ("search", 1, search),
        ("localeCompare", 1, locale_compare),
        ("toString", 0, string_value_of),
        ("valueOf", 0, string_value_of),
    ];
    for (name, arity, func) in methods {
        vm.define_method(proto, name, *arity, *func);
    }
    // `trimLeft` et `trimRight` sont les fonctions mêmes de `trimStart` et
    // `trimEnd` (annexe B).
    for (alias, name) in [("trimLeft", "trimStart"), ("trimRight", "trimEnd")] {
        if let Ok(function) = vm.get(&Value::Object(proto), name) {
            vm.define_hidden(proto, alias, function);
        }
    }

    let proto = vm.intrinsics.number_proto;
    let ctor = constructor(vm, "Number", 1, number_ctor, proto);
    vm.define_method(proto, "toString", 1, number_to_string_method);
    vm.define_method(proto, "toLocaleString", 0, number_to_string_method);
    vm.define_method(proto, "toFixed", 1, number_to_fixed);
    vm.define_method(proto, "valueOf", 0, number_value_of);
    let statics: &[(&str, u32, NativeFn)] = &[
        ("isNaN", 1, number_is_nan),
        ("isFinite", 1, number_is_finite),
        ("isInteger", 1, number_is_integer),
        ("isSafeInteger", 1, number_is_safe_integer),
        ("parseFloat", 1, global_parse_float),
        ("parseInt", 2, global_parse_int),
    ];
    for (name, arity, func) in statics {
        vm.define_method(ctor, name, *arity, *func);
    }
    let constants: &[(&str, f64)] = &[
        ("EPSILON", f64::EPSILON),
        ("MAX_SAFE_INTEGER", 9_007_199_254_740_991.0),
        ("MIN_SAFE_INTEGER", -9_007_199_254_740_991.0),
        ("MAX_VALUE", f64::MAX),
        ("MIN_VALUE", 5e-324),
        ("POSITIVE_INFINITY", f64::INFINITY),
        ("NEGATIVE_INFINITY", f64::NEG_INFINITY),
        ("NaN", f64::NAN),
    ];
    for (name, value) in constants {
        vm.define_hidden(ctor, name, Value::Number(*value));
    }

    let math = vm.new_object();
    let functions: &[(&str, u32, NativeFn)] = &[
        ("abs", 1, math_abs),
        ("floor", 1, math_floor),
        ("ceil", 1, math_ceil),
        ("round", 1, math_round),
        ("trunc", 1, math_trunc),
        ("sign", 1, math_sign),
        ("sqrt", 1, math_sqrt),
        ("cbrt", 1, math_cbrt),
        ("pow", 2, math_pow),
        ("min", 2, math_min),
        ("max", 2, math_max),
        ("hypot", 2, math_hypot),
        ("random", 0, math_random),
        ("sin", 1, math_sin),
        ("cos", 1, math_cos),
        ("tan", 1, math_tan),
        ("asin", 1, math_asin),
        ("acos", 1, math_acos),
        ("atan", 1, math_atan),
        ("atan2", 2, math_atan2),
        ("exp", 1, math_exp),
        ("log", 1, math_log),
        ("log2", 1, math_log2),
        ("log10", 1, math_log10),
        ("fround", 1, math_fround),
        ("clz32", 1, math_clz32),
        ("imul", 2, math_imul),
    ];
    for (name, arity, func) in functions {
        vm.define_method(math, name, *arity, *func);
    }
    let constants: &[(&str, f64)] = &[
        ("PI", std::f64::consts::PI),
        ("E", std::f64::consts::E),
        ("LN2", std::f64::consts::LN_2),
        ("LN10", std::f64::consts::LN_10),
        ("LOG2E", std::f64::consts::LOG2_E),
        ("LOG10E", std::f64::consts::LOG10_E),
        ("SQRT2", std::f64::consts::SQRT_2),
        ("SQRT1_2", std::f64::consts::FRAC_1_SQRT_2),
    ];
    for (name, value) in constants {
        vm.define_hidden(math, name, Value::Number(*value));
    }
    vm.define_hidden(vm.global, "Math", Value::Object(math));
}
