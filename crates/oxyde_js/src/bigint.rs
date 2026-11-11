//! Entiers de précision arbitraire (`BigInt`).
//!
//! Représentation signe + magnitude, la magnitude en chiffres de 32 bits du
//! moins significatif au plus significatif. Les algorithmes sont les
//! classiques « de l'école » (multiplication quadratique, division de Knuth) :
//! suffisants pour les tailles que manipulent les scripts, bornées par
//! [`MAX_BITS`].
//!
//! INVARIANT : `digits` n'a jamais de chiffre nul en tête (dernier élément),
//! et zéro s'écrit `digits` vide avec `negative == false`. Toutes les
//! opérations rendent des valeurs normalisées.

// Arithmétique multi-précision : les conversions entre largeurs d'entiers y
// sont le cœur du calcul (retenues, moitiés de produits) et sont vérifiées
// une à une plutôt qu'annotées une à une.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_lossless,
    reason = "retenues et moitiés de produits de l'arithmétique multi-précision"
)]

use std::cmp::Ordering;

/// Taille maximale d'un `BigInt`, en bits. Au-delà, l'opération échoue
/// (`RangeError` côté script) : cela borne le temps et la mémoire.
pub(crate) const MAX_BITS: u64 = 1 << 22;

/// Le résultat dépasserait [`MAX_BITS`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TooBig;

/// Entier de précision arbitraire.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct BigInt {
    negative: bool,
    digits: Vec<u32>,
}

// ---- Magnitudes ------------------------------------------------------------------

fn trim(mut digits: Vec<u32>) -> Vec<u32> {
    while digits.last() == Some(&0) {
        digits.pop();
    }
    digits
}

fn cmp_mag(a: &[u32], b: &[u32]) -> Ordering {
    a.len()
        .cmp(&b.len())
        .then_with(|| a.iter().rev().cmp(b.iter().rev()))
}

fn add_mag(a: &[u32], b: &[u32]) -> Vec<u32> {
    let (long, short) = if a.len() >= b.len() { (a, b) } else { (b, a) };
    let mut out = Vec::with_capacity(long.len() + 1);
    let mut carry = 0_u64;
    for (i, digit) in long.iter().enumerate() {
        let sum = u64::from(*digit) + u64::from(short.get(i).copied().unwrap_or(0)) + carry;
        out.push(sum as u32);
        carry = sum >> 32;
    }
    if carry != 0 {
        out.push(carry as u32);
    }
    out
}

/// `a - b`, avec `a >= b`.
fn sub_mag(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut out = Vec::with_capacity(a.len());
    let mut borrow = 0_i64;
    for (i, digit) in a.iter().enumerate() {
        let diff = i64::from(*digit) - i64::from(b.get(i).copied().unwrap_or(0)) - borrow;
        out.push(diff as u32);
        borrow = i64::from(diff < 0);
    }
    trim(out)
}

fn mul_mag(a: &[u32], b: &[u32]) -> Vec<u32> {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let mut out = vec![0_u32; a.len() + b.len()];
    for (i, x) in a.iter().enumerate() {
        let mut carry = 0_u64;
        for (j, y) in b.iter().enumerate() {
            let slot = &mut out[i + j];
            let product = u64::from(*x) * u64::from(*y) + u64::from(*slot) + carry;
            *slot = product as u32;
            carry = product >> 32;
        }
        out[i + b.len()] = carry as u32;
    }
    trim(out)
}

/// Divise `digits` par `divisor` (non nul) ; rend le quotient et le reste.
fn divrem_small(digits: &[u32], divisor: u32) -> (Vec<u32>, u32) {
    let mut out = vec![0_u32; digits.len()];
    let mut remainder = 0_u64;
    for (i, digit) in digits.iter().enumerate().rev() {
        let current = (remainder << 32) | u64::from(*digit);
        out[i] = (current / u64::from(divisor)) as u32;
        remainder = current % u64::from(divisor);
    }
    (trim(out), remainder as u32)
}

fn shl_mag(digits: &[u32], bits: u64) -> Vec<u32> {
    if digits.is_empty() {
        return Vec::new();
    }
    let (words, shift) = ((bits / 32) as usize, (bits % 32) as u32);
    let mut out = vec![0_u32; words];
    let mut carry = 0_u32;
    for digit in digits {
        if shift == 0 {
            out.push(*digit);
        } else {
            out.push((digit << shift) | carry);
            carry = digit >> (32 - shift);
        }
    }
    if carry != 0 {
        out.push(carry);
    }
    out
}

fn shr_mag(digits: &[u32], bits: u64) -> Vec<u32> {
    let words = bits / 32;
    if words >= digits.len() as u64 {
        return Vec::new();
    }
    let shift = (bits % 32) as u32;
    let rest = &digits[words as usize..];
    let mut out = Vec::with_capacity(rest.len());
    for (i, digit) in rest.iter().enumerate() {
        let high = rest.get(i + 1).copied().unwrap_or(0);
        out.push(if shift == 0 {
            *digit
        } else {
            (digit >> shift) | (high << (32 - shift))
        });
    }
    trim(out)
}

/// Division de Knuth (algorithme D) : quotient et reste de `u / v`, `v` non nul.
fn divrem_mag(u: &[u32], v: &[u32]) -> (Vec<u32>, Vec<u32>) {
    if cmp_mag(u, v) == Ordering::Less {
        return (Vec::new(), u.to_vec());
    }
    if let [divisor] = v {
        let (quotient, remainder) = divrem_small(u, *divisor);
        return (quotient, trim(vec![remainder]));
    }
    // Normalisation : le chiffre de tête du diviseur a son bit de poids fort à 1.
    let shift = u64::from(v.last().map_or(0, |d| d.leading_zeros()));
    let vn = shl_mag(v, shift);
    let mut un = shl_mag(u, shift);
    un.resize(u.len() + 1, 0);
    let n = vn.len();
    let m = un.len() - n - 1;
    let mut quotient = vec![0_u32; m + 1];
    let base = 1_u64 << 32;
    let (top, second) = (u64::from(vn[n - 1]), u64::from(vn[n - 2]));
    for j in (0..=m).rev() {
        let numerator = (u64::from(un[j + n]) << 32) | u64::from(un[j + n - 1]);
        let mut qhat = numerator / top;
        let mut rhat = numerator % top;
        while qhat >= base || qhat * second > ((rhat << 32) | u64::from(un[j + n - 2])) {
            qhat -= 1;
            rhat += top;
            if rhat >= base {
                break;
            }
        }
        // Soustraction de `qhat * vn` à la tranche courante du dividende.
        let mut borrow = 0_i64;
        let mut carry = 0_u64;
        for i in 0..n {
            let product = qhat * u64::from(vn[i]) + carry;
            carry = product >> 32;
            let diff = i64::from(un[i + j]) - borrow - (product & 0xffff_ffff) as i64;
            un[i + j] = diff as u32;
            borrow = i64::from(diff < 0);
        }
        let diff = i64::from(un[j + n]) - borrow - carry as i64;
        un[j + n] = diff as u32;
        if diff < 0 {
            // `qhat` était trop grand d'une unité : on rend un diviseur.
            qhat -= 1;
            let mut carry = 0_u64;
            for i in 0..n {
                let sum = u64::from(un[i + j]) + u64::from(vn[i]) + carry;
                un[i + j] = sum as u32;
                carry = sum >> 32;
            }
            un[j + n] = (u64::from(un[j + n]) + carry) as u32;
        }
        quotient[j] = qhat as u32;
    }
    un.truncate(n);
    (trim(quotient), shr_mag(&trim(un), shift))
}

fn bits_mag(digits: &[u32]) -> u64 {
    match digits.last() {
        Some(top) => digits.len() as u64 * 32 - u64::from(top.leading_zeros()),
        None => 0,
    }
}

// ---- Complément à deux (opérations bit à bit) ------------------------------------

/// Écriture en complément à deux sur `len` chiffres.
fn to_twos(value: &BigInt, len: usize) -> Vec<u32> {
    let mut out = value.digits.clone();
    out.resize(len, 0);
    if value.negative {
        let mut carry = true;
        for digit in &mut out {
            let (inverted, overflow) = (!*digit).overflowing_add(u32::from(carry));
            *digit = inverted;
            carry = overflow;
        }
    }
    out
}

fn from_twos(mut digits: Vec<u32>) -> BigInt {
    let negative = digits.last().is_some_and(|top| top >> 31 == 1);
    if negative {
        let mut carry = true;
        for digit in &mut digits {
            let (inverted, overflow) = (!*digit).overflowing_add(u32::from(carry));
            *digit = inverted;
            carry = overflow;
        }
    }
    BigInt::from_parts(negative, digits)
}

impl BigInt {
    fn from_parts(negative: bool, digits: Vec<u32>) -> Self {
        let digits = trim(digits);
        Self {
            negative: negative && !digits.is_empty(),
            digits,
        }
    }

    /// Zéro.
    #[must_use]
    pub fn zero() -> Self {
        Self::default()
    }

    /// Entier à partir d'un `i64`.
    #[must_use]
    pub fn from_i64(value: i64) -> Self {
        let magnitude = value.unsigned_abs();
        Self::from_parts(value < 0, vec![magnitude as u32, (magnitude >> 32) as u32])
    }

    /// Entier à partir d'un `u64`.
    #[must_use]
    pub fn from_u64(value: u64) -> Self {
        Self::from_parts(false, vec![value as u32, (value >> 32) as u32])
    }

    /// Entier égal à `value`, si `value` est un entier fini.
    #[must_use]
    pub fn from_f64(value: f64) -> Option<Self> {
        if !value.is_finite() || value.fract() != 0.0 {
            return None;
        }
        let bits = value.abs().to_bits();
        let exponent = ((bits >> 52) & 0x7ff) as i64;
        if exponent == 0 {
            // Zéro ou dénormalisé : seul zéro est entier.
            return Some(Self::zero());
        }
        let significand = (bits & ((1_u64 << 52) - 1)) | (1_u64 << 52);
        let shift = exponent - 1075;
        let magnitude = Self::from_u64(significand).digits;
        let digits = if shift >= 0 {
            shl_mag(&magnitude, shift as u64)
        } else {
            shr_mag(&magnitude, shift.unsigned_abs())
        };
        Some(Self::from_parts(value < 0.0, digits))
    }

    /// Nombre flottant le plus proche (les demis vers le pair).
    #[must_use]
    pub fn to_f64(&self) -> f64 {
        let bits = bits_mag(&self.digits);
        let magnitude = if bits <= 64 {
            let low = u64::from(self.digits.first().copied().unwrap_or(0));
            let high = u64::from(self.digits.get(1).copied().unwrap_or(0));
            ((high << 32) | low) as f64
        } else {
            // 64 bits de tête, plus un bit « collant » si la suite n'est pas
            // nulle : la conversion en flottant arrondit alors correctement.
            let shift = bits - 64;
            let top = shr_mag(&self.digits, shift);
            let mut value = (u64::from(top.get(1).copied().unwrap_or(0)) << 32)
                | u64::from(top.first().copied().unwrap_or(0));
            if shl_mag(&top, shift) != self.digits {
                value |= 1;
            }
            (value as f64) * 2.0_f64.powi(shift.min(4096) as i32)
        };
        if self.negative { -magnitude } else { magnitude }
    }

    /// Vrai si l'entier est nul.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.digits.is_empty()
    }

    /// Vrai si l'entier est strictement négatif.
    #[must_use]
    pub fn is_negative(&self) -> bool {
        self.negative
    }

    /// Nombre de bits de la magnitude.
    pub(crate) fn bits(&self) -> u64 {
        bits_mag(&self.digits)
    }

    /// Les 64 bits de poids faible de l'écriture en complément à deux.
    pub(crate) fn low_u64(&self) -> u64 {
        let twos = to_twos(self, self.digits.len().max(2) + 1);
        (u64::from(twos.get(1).copied().unwrap_or(0)) << 32)
            | u64::from(twos.first().copied().unwrap_or(0))
    }

    /// Valeur en `i64`, si elle y tient.
    pub(crate) fn to_i64(&self) -> Option<i64> {
        if self.digits.len() > 2 {
            return None;
        }
        let magnitude = (u64::from(self.digits.get(1).copied().unwrap_or(0)) << 32)
            | u64::from(self.digits.first().copied().unwrap_or(0));
        if self.negative {
            (magnitude <= 1 << 63).then(|| (magnitude as i64).wrapping_neg())
        } else {
            i64::try_from(magnitude).ok()
        }
    }

    fn checked(self) -> Result<Self, TooBig> {
        if self.bits() > MAX_BITS {
            Err(TooBig)
        } else {
            Ok(self)
        }
    }

    /// Analyse des chiffres en base `radix` (2 à 36), sans signe ni préfixe.
    #[must_use]
    pub fn parse_digits(text: &str, radix: u32) -> Option<Self> {
        if text.is_empty() {
            return None;
        }
        let mut digits: Vec<u32> = Vec::new();
        for c in text.chars() {
            let digit = c.to_digit(radix)?;
            // digits = digits * radix + digit
            let mut carry = u64::from(digit);
            for slot in &mut digits {
                let product = u64::from(*slot) * u64::from(radix) + carry;
                *slot = product as u32;
                carry = product >> 32;
            }
            if carry != 0 {
                digits.push(carry as u32);
            }
            if digits.len() as u64 * 32 > MAX_BITS + 64 {
                return None;
            }
        }
        Some(Self::from_parts(false, digits))
    }

    /// `StringToBigInt` : blancs tolérés autour, chaîne vide = 0, préfixes
    /// `0x`, `0o`, `0b` (sans signe), décimal avec signe facultatif.
    #[must_use]
    pub fn parse_string(text: &str) -> Option<Self> {
        let text = text.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}');
        if text.is_empty() {
            return Some(Self::zero());
        }
        for (prefixes, radix) in [(["0x", "0X"], 16), (["0o", "0O"], 8), (["0b", "0B"], 2)] {
            if let Some(rest) = prefixes.iter().find_map(|p| text.strip_prefix(p)) {
                return Self::parse_digits(rest, radix);
            }
        }
        let (negative, rest) = match text.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, text.strip_prefix('+').unwrap_or(text)),
        };
        let magnitude = Self::parse_digits(rest, 10)?;
        Some(Self::from_parts(negative, magnitude.digits))
    }

    /// Écriture en base `radix` (2 à 36).
    #[must_use]
    pub fn to_string_radix(&self, radix: u32) -> String {
        if self.is_zero() {
            return "0".to_owned();
        }
        let radix = radix.clamp(2, 36);
        // Plus grande puissance de la base tenant dans un chiffre de 32 bits.
        let (mut chunk, mut width) = (radix, 1_usize);
        while let Some(next) = chunk.checked_mul(radix) {
            chunk = next;
            width += 1;
        }
        let mut groups = Vec::new();
        let mut rest = self.digits.clone();
        while !rest.is_empty() {
            let (quotient, remainder) = divrem_small(&rest, chunk);
            groups.push(remainder);
            rest = quotient;
        }
        let mut out = String::new();
        if self.negative {
            out.push('-');
        }
        for (i, group) in groups.iter().rev().enumerate() {
            let mut text = String::new();
            let mut value = *group;
            while value > 0 {
                text.insert(0, char::from_digit(value % radix, radix).unwrap_or('0'));
                value /= radix;
            }
            if i > 0 {
                // Les groupes suivants gardent leurs zéros de tête.
                for _ in text.len()..width {
                    out.push('0');
                }
            }
            out.push_str(&text);
        }
        out
    }

    /// Opposé.
    #[must_use]
    pub fn neg(&self) -> Self {
        Self::from_parts(!self.negative, self.digits.clone())
    }

    /// Somme.
    pub(crate) fn add(&self, other: &Self) -> Result<Self, TooBig> {
        let result = if self.negative == other.negative {
            Self::from_parts(self.negative, add_mag(&self.digits, &other.digits))
        } else {
            match cmp_mag(&self.digits, &other.digits) {
                Ordering::Less => {
                    Self::from_parts(other.negative, sub_mag(&other.digits, &self.digits))
                }
                _ => Self::from_parts(self.negative, sub_mag(&self.digits, &other.digits)),
            }
        };
        result.checked()
    }

    /// Différence.
    pub(crate) fn sub(&self, other: &Self) -> Result<Self, TooBig> {
        self.add(&other.neg())
    }

    /// Produit.
    pub(crate) fn mul(&self, other: &Self) -> Result<Self, TooBig> {
        if self.bits() + other.bits() > MAX_BITS + 1 {
            return Err(TooBig);
        }
        Self::from_parts(
            self.negative != other.negative,
            mul_mag(&self.digits, &other.digits),
        )
        .checked()
    }

    /// Quotient et reste de la division tronquée vers zéro ; `None` si
    /// `other` est nul. Le reste a le signe du dividende.
    #[must_use]
    pub fn div_rem(&self, other: &Self) -> Option<(Self, Self)> {
        if other.is_zero() {
            return None;
        }
        let (quotient, remainder) = divrem_mag(&self.digits, &other.digits);
        Some((
            Self::from_parts(self.negative != other.negative, quotient),
            Self::from_parts(self.negative, remainder),
        ))
    }

    /// Puissance ; `exponent` positif ou nul.
    pub(crate) fn pow(&self, exponent: u64) -> Result<Self, TooBig> {
        if self.bits().saturating_mul(exponent) > MAX_BITS + 64 && self.bits() > 1 {
            return Err(TooBig);
        }
        let mut result = Self::from_u64(1);
        let mut base = self.clone();
        let mut remaining = exponent;
        while remaining > 0 {
            if remaining & 1 == 1 {
                result = result.mul(&base)?;
            }
            remaining >>= 1;
            if remaining > 0 {
                base = base.mul(&base)?;
            }
        }
        Ok(result)
    }

    /// Décalage à gauche de `bits` (à droite si négatif), arrondi vers moins
    /// l'infini comme l'exige le langage.
    pub(crate) fn shift_left(&self, bits: i64) -> Result<Self, TooBig> {
        if bits >= 0 {
            if self.is_zero() {
                return Ok(Self::zero());
            }
            if self.bits().saturating_add(bits as u64) > MAX_BITS {
                return Err(TooBig);
            }
            return Ok(Self::from_parts(
                self.negative,
                shl_mag(&self.digits, bits as u64),
            ));
        }
        let amount = bits.unsigned_abs();
        if !self.negative {
            return Ok(Self::from_parts(false, shr_mag(&self.digits, amount)));
        }
        // Négatif : -((|x| - 1) >> n) - 1.
        let minus_one = sub_mag(&self.digits, &[1]);
        let shifted = shr_mag(&minus_one, amount);
        Ok(Self::from_parts(true, add_mag(&shifted, &[1])))
    }

    fn bitwise(&self, other: &Self, op: impl Fn(u32, u32) -> u32) -> Self {
        let len = self.digits.len().max(other.digits.len()) + 1;
        let (a, b) = (to_twos(self, len), to_twos(other, len));
        from_twos(a.iter().zip(&b).map(|(x, y)| op(*x, *y)).collect())
    }

    /// ET bit à bit (complément à deux infini).
    #[must_use]
    pub fn bit_and(&self, other: &Self) -> Self {
        self.bitwise(other, |x, y| x & y)
    }

    /// OU bit à bit.
    #[must_use]
    pub fn bit_or(&self, other: &Self) -> Self {
        self.bitwise(other, |x, y| x | y)
    }

    /// OU exclusif bit à bit.
    #[must_use]
    pub fn bit_xor(&self, other: &Self) -> Self {
        self.bitwise(other, |x, y| x ^ y)
    }

    /// Complément bit à bit : `-x - 1`.
    #[must_use]
    pub fn bit_not(&self) -> Self {
        if self.negative {
            Self::from_parts(false, sub_mag(&self.digits, &[1]))
        } else {
            Self::from_parts(true, add_mag(&self.digits, &[1]))
        }
    }

    /// `BigInt.asUintN(bits, x)` : `x` modulo `2^bits`.
    pub(crate) fn as_uint_n(&self, bits: u64) -> Result<Self, TooBig> {
        if bits == 0 {
            return Ok(Self::zero());
        }
        if !self.negative && self.bits() <= bits {
            return Ok(self.clone());
        }
        if bits > MAX_BITS {
            return Err(TooBig);
        }
        let len = (bits / 32) as usize + 2;
        let mut twos = to_twos(self, len.max(self.digits.len() + 1));
        twos.truncate(bits.div_ceil(32) as usize);
        if !bits.is_multiple_of(32)
            && let Some(top) = twos.last_mut()
        {
            *top &= (1_u32 << (bits % 32)) - 1;
        }
        Ok(Self::from_parts(false, twos))
    }

    /// `BigInt.asIntN(bits, x)` : `x` ramené dans `[-2^(bits-1), 2^(bits-1))`.
    pub(crate) fn as_int_n(&self, bits: u64) -> Result<Self, TooBig> {
        if bits == 0 {
            return Ok(Self::zero());
        }
        if self.bits() < bits {
            return Ok(self.clone());
        }
        let unsigned = self.as_uint_n(bits)?;
        if unsigned.bits() == bits {
            // Bit de signe à 1 : on retranche 2^bits.
            let modulus = Self::from_u64(1).shift_left(bits as i64)?;
            return unsigned.sub(&modulus);
        }
        Ok(unsigned)
    }

    /// Comparaison avec un nombre flottant ; `None` si `number` est `NaN`.
    #[must_use]
    pub fn compare_f64(&self, number: f64) -> Option<Ordering> {
        if number.is_nan() {
            return None;
        }
        if number == f64::INFINITY {
            return Some(Ordering::Less);
        }
        if number == f64::NEG_INFINITY {
            return Some(Ordering::Greater);
        }
        // Compare à la partie entière, puis départage par la partie fractionnaire.
        let integer = Self::from_f64(number.trunc())?;
        Some(self.cmp(&integer).then_with(|| {
            let fraction = number - number.trunc();
            if fraction > 0.0 {
                Ordering::Less
            } else if fraction < 0.0 {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        }))
    }
}

impl Ord for BigInt {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.negative, other.negative) {
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (false, false) => cmp_mag(&self.digits, &other.digits),
            (true, true) => cmp_mag(&other.digits, &self.digits),
        }
    }
}

impl PartialOrd for BigInt {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl std::fmt::Display for BigInt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_string_radix(10))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn big(text: &str) -> BigInt {
        BigInt::parse_string(text).expect("entier valide")
    }

    #[test]
    fn parsing_and_printing() {
        for text in [
            "0",
            "1",
            "-1",
            "4294967296",
            "-18446744073709551616",
            "123456789012345678901234567890",
        ] {
            assert_eq!(big(text).to_string(), text);
        }
        assert_eq!(big("0xff").to_string(), "255");
        assert_eq!(big("  0b101 ").to_string(), "5");
        assert_eq!(big("0o17").to_string(), "15");
        assert_eq!(big("").to_string(), "0");
        assert_eq!(big("-0").to_string(), "0");
        assert!(!big("-0").is_negative());
        assert_eq!(big("255").to_string_radix(16), "ff");
        assert_eq!(big("-255").to_string_radix(2), "-11111111");
        assert_eq!(
            big("1000000000000000000000").to_string_radix(36),
            "5v1j4f4ds79m9s"
        );
        // Zéros internes d'un groupe de chiffres.
        assert_eq!(
            big("1000000000000000000").to_string(),
            "1000000000000000000"
        );
        for bad in ["1.5", "abc", "0x", "-0x1", "1n", "+", "1 2", "--1"] {
            assert_eq!(BigInt::parse_string(bad), None, "{bad}");
        }
    }

    #[test]
    fn arithmetic() {
        let a = big("123456789012345678901234567890");
        let b = big("987654321098765432109876543210");
        assert_eq!(
            a.add(&b).expect("somme").to_string(),
            "1111111110111111111011111111100"
        );
        assert_eq!(
            a.sub(&b).expect("différence").to_string(),
            "-864197532086419753208641975320"
        );
        assert_eq!(
            a.mul(&b).expect("produit").to_string(),
            "121932631137021795226185032733622923332237463801111263526900"
        );
        let (q, r) = b.div_rem(&a).expect("division");
        assert_eq!(
            (q.to_string(), r.to_string()),
            ("8".to_owned(), "9000000000900000000090".to_owned())
        );
        // Division tronquée vers zéro, reste du signe du dividende.
        for (x, y, q, r) in [
            ("7", "2", "3", "1"),
            ("-7", "2", "-3", "-1"),
            ("7", "-2", "-3", "1"),
            ("-7", "-2", "3", "-1"),
        ] {
            let (quotient, remainder) = big(x).div_rem(&big(y)).expect("division");
            assert_eq!(
                (quotient.to_string(), remainder.to_string()),
                (q.to_owned(), r.to_owned())
            );
        }
        assert_eq!(big("1").div_rem(&BigInt::zero()), None);
        // Division longue à plusieurs chiffres : q * d + r == n.
        let n = big("0xfedcba9876543210fedcba9876543210fedcba9876543210");
        let d = big("0x123456789abcdef0123456789");
        let (q, r) = n.div_rem(&d).expect("division");
        assert_eq!(
            q.mul(&d).and_then(|p| p.add(&r)).expect("reconstitution"),
            n
        );
        assert!(r < d);
        // Cas où l'estimation du chiffre du quotient doit être corrigée.
        let n = big("0x800000000000000000000000");
        let d = big("0x800000000000000000000001");
        assert_eq!(
            n.div_rem(&d).map(|(q, r)| (q.to_string(), r == n)),
            Some(("0".to_owned(), true))
        );
        let n = big("0x7fffffff800000010000000000000000");
        let d = big("0x800000008000000200000005");
        let (q, r) = n.div_rem(&d).expect("division");
        assert_eq!(
            q.mul(&d).and_then(|p| p.add(&r)).expect("reconstitution"),
            n
        );
        assert_eq!(
            big("2").pow(100).expect("puissance").to_string(),
            "1267650600228229401496703205376"
        );
        assert_eq!(big("-3").pow(3).expect("puissance").to_string(), "-27");
        assert_eq!(big("5").pow(0).expect("puissance").to_string(), "1");
        assert_eq!(big("2").pow(1 << 40), Err(TooBig));
    }

    #[test]
    fn shifts_and_bitwise() {
        assert_eq!(
            big("1").shift_left(70).expect("décalage").to_string(),
            "1180591620717411303424"
        );
        assert_eq!(
            big("1180591620717411303424")
                .shift_left(-69)
                .expect("décalage")
                .to_string(),
            "2"
        );
        // Décalage à droite d'un négatif : arrondi vers moins l'infini.
        assert_eq!(
            big("-5").shift_left(-1).expect("décalage").to_string(),
            "-3"
        );
        assert_eq!(
            big("-1").shift_left(-100).expect("décalage").to_string(),
            "-1"
        );
        assert_eq!(
            big("-8").shift_left(-3).expect("décalage").to_string(),
            "-1"
        );
        assert_eq!(big("5").shift_left(-10).expect("décalage").to_string(), "0");
        assert_eq!(big("1").shift_left(1 << 40), Err(TooBig));
        assert_eq!(big("12").bit_and(&big("10")).to_string(), "8");
        assert_eq!(big("12").bit_or(&big("-10")).to_string(), "-2");
        assert_eq!(big("-12").bit_xor(&big("10")).to_string(), "-2");
        assert_eq!(
            big("-1").bit_and(&big("0xffffffffffffffffff")).to_string(),
            "4722366482869645213695"
        );
        assert_eq!(big("0").bit_not().to_string(), "-1");
        assert_eq!(big("-1").bit_not().to_string(), "0");
        assert_eq!(big("255").bit_not().to_string(), "-256");
        assert_eq!(big("-1").as_uint_n(8).expect("modulo").to_string(), "255");
        assert_eq!(big("256").as_uint_n(8).expect("modulo").to_string(), "0");
        assert_eq!(big("255").as_int_n(8).expect("modulo").to_string(), "-1");
        assert_eq!(big("128").as_int_n(8).expect("modulo").to_string(), "-128");
        assert_eq!(big("127").as_int_n(8).expect("modulo").to_string(), "127");
        assert_eq!(big("-129").as_int_n(8).expect("modulo").to_string(), "127");
        assert_eq!(
            big("-1").as_uint_n(64).expect("modulo").to_string(),
            "18446744073709551615"
        );
        assert_eq!(
            big("18446744073709551615")
                .as_int_n(64)
                .expect("modulo")
                .to_string(),
            "-1"
        );
        assert_eq!(big("5").as_int_n(0).expect("modulo").to_string(), "0");
    }

    #[test]
    fn floats_and_comparisons() {
        assert_eq!(
            BigInt::from_f64(1e21).map(|b| b.to_string()).as_deref(),
            Some("1000000000000000000000")
        );
        assert_eq!(
            BigInt::from_f64(-0.0).map(|b| b.to_string()).as_deref(),
            Some("0")
        );
        assert_eq!(
            BigInt::from_f64(9_007_199_254_740_993.0)
                .map(|b| b.to_string())
                .as_deref(),
            Some("9007199254740992")
        );
        assert_eq!(BigInt::from_f64(1.5), None);
        assert_eq!(BigInt::from_f64(f64::NAN), None);
        assert_eq!(
            BigInt::from_f64(f64::MAX).map(|b| b.to_f64()),
            Some(f64::MAX)
        );
        assert_eq!(big("9007199254740993").to_f64(), 9_007_199_254_740_992.0);
        assert_eq!(big("9007199254740995").to_f64(), 9_007_199_254_740_996.0);
        assert_eq!(
            big("-18446744073709551617").to_f64(),
            -18_446_744_073_709_551_616.0
        );
        assert_eq!(
            big("1").shift_left(2000).expect("décalage").to_f64(),
            f64::INFINITY
        );
        assert!(big("-5") < big("3") && big("-5") < big("-3") && big("10") > big("9"));
        assert_eq!(big("5").compare_f64(5.5), Some(Ordering::Less));
        assert_eq!(big("-5").compare_f64(-5.5), Some(Ordering::Greater));
        assert_eq!(big("5").compare_f64(5.0), Some(Ordering::Equal));
        assert_eq!(big("5").compare_f64(f64::INFINITY), Some(Ordering::Less));
        assert_eq!(big("5").compare_f64(f64::NAN), None);
        assert_eq!(BigInt::from_i64(i64::MIN).to_i64(), Some(i64::MIN));
        assert_eq!(BigInt::from_i64(-42).to_string(), "-42");
        assert_eq!(big("9223372036854775808").to_i64(), None);
    }
}
