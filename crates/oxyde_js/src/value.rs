//! Valeurs JavaScript et conversions qui ne dépendent pas du tas.
//!
//! LIMITE CONNUE : les chaînes sont stockées en UTF-8 (`Arc<str>`). Les
//! longueurs et index sont bien comptés en unités UTF-16 comme l'exige le
//! langage, mais un substitut isolé n'est pas représentable.

use std::sync::Arc;

/// Chaîne JavaScript immuable et partagée.
pub type JsStr = Arc<str>;

/// Référence à un objet du tas.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObjId(pub(crate) u32);

/// Symbole : identité unique et description facultative.
///
/// Deux symboles sont égaux s'ils sont le même symbole, jamais par leur
/// description.
#[derive(Clone, Debug)]
pub struct Symbol(Arc<SymbolData>);

#[derive(Debug)]
struct SymbolData {
    id: u64,
    description: Option<JsStr>,
}

impl Symbol {
    pub(crate) fn new(id: u64, description: Option<JsStr>) -> Self {
        Self(Arc::new(SymbolData { id, description }))
    }

    /// Identifiant unique du symbole dans sa machine virtuelle.
    #[must_use]
    pub fn id(&self) -> u64 {
        self.0.id
    }

    /// Description donnée à la création (`Symbol("description")`).
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.0.description.as_deref()
    }
}

impl PartialEq for Symbol {
    fn eq(&self, other: &Self) -> bool {
        self.0.id == other.0.id
    }
}

impl Eq for Symbol {}

impl std::hash::Hash for Symbol {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.id.hash(state);
    }
}

/// Clé de propriété : chaîne ou symbole.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PropertyKey {
    /// Clé ordinaire.
    String(JsStr),
    /// Clé symbole.
    Symbol(Symbol),
}

impl PropertyKey {
    /// Valeur JavaScript correspondant à la clé.
    #[must_use]
    pub fn to_value(&self) -> Value {
        match self {
            Self::String(s) => Value::String(Arc::clone(s)),
            Self::Symbol(s) => Value::Symbol(s.clone()),
        }
    }

    /// Texte de la clé pour les messages d'erreur.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::String(s) => s.to_string(),
            Self::Symbol(s) => format!("Symbol({})", s.description().unwrap_or("")),
        }
    }
}

/// Valeur JavaScript.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// `undefined`
    Undefined,
    /// `null`
    Null,
    /// Booléen.
    Bool(bool),
    /// Nombre (IEEE 754 double précision).
    Number(f64),
    /// Chaîne.
    String(JsStr),
    /// Symbole.
    Symbol(Symbol),
    /// Objet (y compris fonctions et tableaux).
    Object(ObjId),
    /// Interne : liaison `let` / `const` pas encore initialisée, ou trou de
    /// tableau. N'est jamais visible d'un script.
    Hole,
}

impl Value {
    /// Chaîne JavaScript à partir d'une chaîne Rust.
    #[must_use]
    pub fn string(s: &str) -> Self {
        Self::String(Arc::from(s))
    }

    /// Vrai pour `undefined` et `null`.
    #[must_use]
    pub fn is_nullish(&self) -> bool {
        matches!(self, Self::Undefined | Self::Null | Self::Hole)
    }

    /// Conversion en booléen (ECMA-262 §7.1.2).
    #[must_use]
    pub fn truthy(&self) -> bool {
        match self {
            Self::Undefined | Self::Null | Self::Hole => false,
            Self::Bool(b) => *b,
            Self::Number(n) => *n != 0.0 && !n.is_nan(),
            Self::String(s) => !s.is_empty(),
            Self::Symbol(_) | Self::Object(_) => true,
        }
    }

    /// Identifiant d'objet, si la valeur en est un.
    #[must_use]
    pub fn as_object(&self) -> Option<ObjId> {
        match self {
            Self::Object(id) => Some(*id),
            _ => None,
        }
    }
}

/// Longueur d'une chaîne en unités UTF-16.
#[must_use]
pub fn utf16_len(s: &str) -> usize {
    if s.is_ascii() {
        s.len()
    } else {
        s.encode_utf16().count()
    }
}

/// Conversion d'un nombre en chaîne (ECMA-262 §6.1.6.1.20, base 10).
#[must_use]
pub fn number_to_string(n: f64) -> String {
    if n.is_nan() {
        return "NaN".to_owned();
    }
    if n == 0.0 {
        return "0".to_owned();
    }
    if n.is_infinite() {
        return if n > 0.0 { "Infinity" } else { "-Infinity" }.to_owned();
    }
    // `{:e}` donne les chiffres significatifs les plus courts : « d.ddde±x ».
    let formatted = format!("{:e}", n.abs());
    let (mantissa, exponent) = formatted.split_once('e').unwrap_or((&formatted, "0"));
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let exponent: i32 = exponent.parse().unwrap_or(0);
    // `point` : position de la virgule par rapport au début des chiffres.
    let point = exponent + 1;
    let count = i32::try_from(digits.len()).unwrap_or(i32::MAX);
    let mut out = String::new();
    if n < 0.0 {
        out.push('-');
    }
    if count <= point && point <= 21 {
        out.push_str(&digits);
        out.extend(std::iter::repeat_n(
            '0',
            usize::try_from(point - count).unwrap_or(0),
        ));
    } else if 0 < point && point <= 21 {
        let split = usize::try_from(point).unwrap_or(0);
        out.push_str(digits.get(..split).unwrap_or(&digits));
        out.push('.');
        out.push_str(digits.get(split..).unwrap_or(""));
    } else if -6 < point && point <= 0 {
        out.push_str("0.");
        out.extend(std::iter::repeat_n(
            '0',
            usize::try_from(-point).unwrap_or(0),
        ));
        out.push_str(&digits);
    } else {
        out.push_str(digits.get(..1).unwrap_or("0"));
        if digits.len() > 1 {
            out.push('.');
            out.push_str(digits.get(1..).unwrap_or(""));
        }
        out.push('e');
        out.push(if exponent < 0 { '-' } else { '+' });
        out.push_str(&exponent.unsigned_abs().to_string());
    }
    out
}

fn is_js_whitespace(c: char) -> bool {
    c.is_whitespace() || c == '\u{feff}'
}

/// Conversion d'une chaîne en nombre (ECMA-262 §7.1.4.1).
#[must_use]
pub fn string_to_number(s: &str) -> f64 {
    let s = s.trim_matches(is_js_whitespace);
    if s.is_empty() {
        return 0.0;
    }
    let radix_literal = |prefix: [&str; 2], radix: u32| -> Option<f64> {
        let digits = s
            .strip_prefix(prefix[0])
            .or_else(|| s.strip_prefix(prefix[1]))?;
        if digits.is_empty() {
            return Some(f64::NAN);
        }
        let mut value = 0.0_f64;
        for c in digits.chars() {
            let Some(digit) = c.to_digit(radix) else {
                return Some(f64::NAN);
            };
            value = value * f64::from(radix) + f64::from(digit);
        }
        Some(value)
    };
    if let Some(v) = radix_literal(["0x", "0X"], 16)
        .or_else(|| radix_literal(["0o", "0O"], 8))
        .or_else(|| radix_literal(["0b", "0B"], 2))
    {
        return v;
    }
    match s {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    // Rust accepte « inf », « nan »… que JavaScript refuse : on filtre d'abord.
    if !s
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | '+' | '-'))
    {
        return f64::NAN;
    }
    s.parse::<f64>().unwrap_or(f64::NAN)
}

/// `ToInt32` (ECMA-262 §7.1.6).
#[must_use]
pub fn to_int32(n: f64) -> i32 {
    to_uint32(n).cast_signed()
}

/// `ToUint32` (ECMA-262 §7.1.7).
#[must_use]
pub fn to_uint32(n: f64) -> u32 {
    if !n.is_finite() {
        return 0;
    }
    // Modulo 2^32 du nombre tronqué ; `rem_euclid` garde le résultat positif.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "valeur ramenée dans [0, 2^32) juste avant"
    )]
    let wrapped = n.trunc().rem_euclid(4_294_967_296.0) as u32;
    wrapped
}

/// Index de tableau canonique désigné par une clé de propriété, s'il y en a un.
#[must_use]
pub fn array_index(key: &str) -> Option<usize> {
    if key.is_empty() || key.len() > 10 || (key.len() > 1 && key.starts_with('0')) {
        return None;
    }
    if !key.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let index: u64 = key.parse().ok()?;
    // 2^32 - 1 n'est pas un index valide (c'est la longueur maximale).
    (index < u64::from(u32::MAX))
        .then(|| usize::try_from(index).ok())
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_format_like_javascript() {
        let cases: &[(f64, &str)] = &[
            (0.0, "0"),
            (-0.0, "0"),
            (1.0, "1"),
            (-1.5, "-1.5"),
            (123456789.0, "123456789"),
            (0.1, "0.1"),
            (0.000001, "0.000001"),
            (0.0000001, "1e-7"),
            (1.5e-10, "1.5e-10"),
            (1e21, "1e+21"),
            (1e20, "100000000000000000000"),
            (123e25, "1.23e+27"),
            (0.1 + 0.2, "0.30000000000000004"),
            (f64::NAN, "NaN"),
            (f64::INFINITY, "Infinity"),
            (f64::NEG_INFINITY, "-Infinity"),
            (4294967296.0, "4294967296"),
            (1.7976931348623157e308, "1.7976931348623157e+308"),
            (5e-324, "5e-324"),
        ];
        for (n, expected) in cases {
            assert_eq!(number_to_string(*n), *expected);
        }
    }

    #[test]
    fn strings_convert_like_javascript() {
        assert_eq!(string_to_number(""), 0.0);
        assert_eq!(string_to_number("  42\n"), 42.0);
        assert_eq!(string_to_number("-1.5e2"), -150.0);
        assert_eq!(string_to_number(".5"), 0.5);
        assert_eq!(string_to_number("0x1F"), 31.0);
        assert_eq!(string_to_number("0b11"), 3.0);
        assert_eq!(string_to_number("Infinity"), f64::INFINITY);
        for bad in [
            "abc", "1px", "inf", "nan", "infinity", "0x", "0xZ", "1e", "--1", "1 2",
        ] {
            assert!(string_to_number(bad).is_nan(), "{bad:?}");
        }
    }

    #[test]
    fn integer_conversions() {
        assert_eq!(to_int32(4294967295.0), -1);
        assert_eq!(to_int32(2147483648.0), -2147483648);
        assert_eq!(to_int32(-1.9), -1);
        assert_eq!(to_uint32(-1.0), 4294967295);
        assert_eq!(to_uint32(f64::NAN), 0);
        assert_eq!(to_uint32(1e300), 0);
        assert_eq!(to_int32(f64::INFINITY), 0);
    }

    #[test]
    fn array_indices() {
        assert_eq!(array_index("0"), Some(0));
        assert_eq!(array_index("42"), Some(42));
        assert_eq!(array_index("4294967294"), Some(4294967294));
        for bad in [
            "",
            "01",
            "-1",
            "1.5",
            "4294967295",
            "99999999999",
            "a",
            " 1",
        ] {
            assert_eq!(array_index(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn truthiness_and_lengths() {
        assert!(!Value::Number(f64::NAN).truthy());
        assert!(!Value::string("").truthy());
        assert!(Value::string("0").truthy());
        assert_eq!(utf16_len("aé😀"), 4);
    }
}
