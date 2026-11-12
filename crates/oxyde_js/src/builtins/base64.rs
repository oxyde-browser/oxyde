//! `Uint8Array` et le texte : `fromBase64`, `fromHex`, `toBase64`, `toHex`,
//! `setFromBase64` et `setFromHex`.
//!
//! Le décodage suit la spécification à la lettre, y compris ce qu'elle rend
//! en cas d'erreur : les octets déjà décodés restent écrits par
//! `setFromBase64` et `setFromHex` avant que l'erreur ne soit levée.

use super::arg;
use super::typed_array::{number, validate};
use crate::heap::ElementKind;
use crate::value::{ObjId, Value};
use crate::vm::{JsResult, NativeFn, Vm};

/// Alphabet du base64 ordinaire.
const STANDARD: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
/// Alphabet du base64 pour adresses (`-` et `_`).
const URL_SAFE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// Traitement d'un dernier groupe incomplet.
#[derive(Clone, Copy, PartialEq, Eq)]
enum LastChunk {
    /// Accepté avec ou sans remplissage.
    Loose,
    /// Remplissage exigé, bits excédentaires nuls.
    Strict,
    /// Laissé de côté : le décodage s'arrête avant lui.
    StopBeforePartial,
}

/// Résultat d'un décodage : ce qui a été lu et décodé, et l'erreur qui l'a
/// arrêté s'il y en a une.
struct Decoded {
    /// Unités de code consommées.
    read: usize,
    bytes: Vec<u8>,
    error: Option<&'static str>,
}

/// Valeur, de 0 à 63, d'un caractère du base64 ordinaire.
fn sextet(unit: u16) -> Option<u8> {
    let byte = u8::try_from(unit).ok()?;
    STANDARD
        .iter()
        .position(|c| *c == byte)
        .and_then(|p| u8::try_from(p).ok())
}

/// Blanc ASCII, ignoré par le décodage.
fn is_whitespace(unit: u16) -> bool {
    matches!(unit, 0x09 | 0x0a | 0x0c | 0x0d | 0x20)
}

fn skip_whitespace(units: &[u16], mut index: usize) -> usize {
    while units.get(index).is_some_and(|u| is_whitespace(*u)) {
        index += 1;
    }
    index
}

/// Décode un groupe de 2 à 4 sextets ; `strict` : refuse des bits non nuls
/// au-delà des octets rendus.
fn decode_chunk(chunk: &[u8], strict: bool) -> Result<Vec<u8>, &'static str> {
    let mut sextets = [0u8; 4];
    for (slot, value) in sextets.iter_mut().zip(chunk) {
        *slot = *value;
    }
    let bytes = [
        (sextets[0] << 2) | (sextets[1] >> 4),
        (sextets[1] << 4) | (sextets[2] >> 2),
        (sextets[2] << 6) | sextets[3],
    ];
    let kept = match chunk.len() {
        2 => 1,
        3 => 2,
        _ => 3,
    };
    if strict && bytes[kept..].iter().any(|b| *b != 0) {
        return Err("bits non nuls après le dernier octet");
    }
    Ok(bytes[..kept].to_vec())
}

/// `FromBase64` : décode `units`, `max_length` octets au plus.
fn from_base64(units: &[u16], url_safe: bool, last_chunk: LastChunk, max_length: usize) -> Decoded {
    let mut decoded = Decoded {
        read: 0,
        bytes: Vec::new(),
        error: None,
    };
    if max_length == 0 {
        return decoded;
    }
    let fail = |mut decoded: Decoded, message: &'static str| {
        decoded.error = Some(message);
        decoded
    };
    let mut chunk: Vec<u8> = Vec::with_capacity(4);
    let mut index = 0;
    loop {
        index = skip_whitespace(units, index);
        let Some(unit) = units.get(index).copied() else {
            // Fin du texte : reste-t-il un groupe incomplet ?
            if !chunk.is_empty() {
                match last_chunk {
                    LastChunk::StopBeforePartial => return decoded,
                    LastChunk::Strict => return fail(decoded, "dernier groupe sans remplissage"),
                    LastChunk::Loose => {
                        if chunk.len() == 1 {
                            return fail(decoded, "dernier groupe d'un seul caractère");
                        }
                        match decode_chunk(&chunk, false) {
                            Ok(bytes) => decoded.bytes.extend(bytes),
                            Err(message) => return fail(decoded, message),
                        }
                    }
                }
            }
            decoded.read = units.len();
            return decoded;
        };
        index += 1;
        if unit == u16::from(b'=') {
            if chunk.len() < 2 {
                return fail(decoded, "remplissage mal placé");
            }
            index = skip_whitespace(units, index);
            if chunk.len() == 2 {
                match units.get(index) {
                    None => {
                        if last_chunk == LastChunk::StopBeforePartial {
                            return decoded;
                        }
                        return fail(decoded, "remplissage incomplet");
                    }
                    Some(next) if *next == u16::from(b'=') => {
                        index = skip_whitespace(units, index + 1);
                    }
                    Some(_) => {}
                }
            }
            if index < units.len() {
                return fail(decoded, "caractères après le remplissage");
            }
            match decode_chunk(&chunk, last_chunk == LastChunk::Strict) {
                Ok(bytes) => decoded.bytes.extend(bytes),
                Err(message) => return fail(decoded, message),
            }
            decoded.read = units.len();
            return decoded;
        }
        // L'alphabet pour adresses se ramène à l'alphabet ordinaire.
        let unit = if url_safe {
            match u8::try_from(unit) {
                Ok(b'+' | b'/') => return fail(decoded, "caractère hors de l'alphabet"),
                Ok(b'-') => u16::from(b'+'),
                Ok(b'_') => u16::from(b'/'),
                _ => unit,
            }
        } else {
            unit
        };
        let Some(value) = sextet(unit) else {
            return fail(decoded, "caractère hors de l'alphabet");
        };
        // Plus assez de place pour les octets du groupe en cours.
        let remaining = max_length - decoded.bytes.len();
        if (remaining == 1 && chunk.len() == 2) || (remaining == 2 && chunk.len() == 3) {
            return decoded;
        }
        chunk.push(value);
        if chunk.len() == 4 {
            match decode_chunk(&chunk, false) {
                Ok(bytes) => decoded.bytes.extend(bytes),
                Err(message) => return fail(decoded, message),
            }
            chunk.clear();
            decoded.read = index;
            if decoded.bytes.len() == max_length {
                return decoded;
            }
        }
    }
}

/// `FromHex` : décode `units`, `max_length` octets au plus.
fn from_hex(units: &[u16], max_length: usize) -> Decoded {
    let mut decoded = Decoded {
        read: 0,
        bytes: Vec::new(),
        error: None,
    };
    if !units.len().is_multiple_of(2) {
        decoded.error = Some("nombre impair de chiffres hexadécimaux");
        return decoded;
    }
    let digit = |unit: u16| {
        char::from_u32(u32::from(unit))
            .and_then(|c| c.to_digit(16))
            .and_then(|d| u8::try_from(d).ok())
    };
    while decoded.read < units.len() && decoded.bytes.len() < max_length {
        let pair = (
            units.get(decoded.read).copied().and_then(digit),
            units.get(decoded.read + 1).copied().and_then(digit),
        );
        let (Some(high), Some(low)) = pair else {
            decoded.error = Some("chiffre hexadécimal invalide");
            return decoded;
        };
        decoded.read += 2;
        decoded.bytes.push((high << 4) | low);
    }
    decoded
}

/// Texte d'un argument qui doit être une chaîne, en unités de code.
fn string_units(vm: &mut Vm, value: &Value) -> JsResult<Vec<u16>> {
    match value {
        Value::String(text) => Ok(text.encode_utf16().collect()),
        _ => Err(vm.type_error("une chaîne est attendue")),
    }
}

/// Objet d'options : `undefined` vaut un objet vide, toute autre valeur qui
/// n'est pas un objet est refusée.
fn options_object(vm: &mut Vm, value: &Value) -> JsResult<Option<Value>> {
    match value {
        Value::Undefined => Ok(None),
        Value::Object(_) => Ok(Some(value.clone())),
        _ => Err(vm.type_error("les options doivent être un objet")),
    }
}

/// Option textuelle `name` : absente, ou l'une des valeurs `allowed`.
fn string_option(
    vm: &mut Vm,
    options: Option<&Value>,
    name: &str,
    allowed: &[&str],
) -> JsResult<Option<usize>> {
    let Some(options) = options else {
        return Ok(None);
    };
    match vm.get(options, name)? {
        Value::Undefined => Ok(None),
        Value::String(text) => match allowed.iter().position(|a| *a == &*text) {
            Some(position) => Ok(Some(position)),
            None => Err(vm.type_error(&format!("valeur inconnue pour l'option « {name} »"))),
        },
        _ => Err(vm.type_error(&format!("l'option « {name} » doit être une chaîne"))),
    }
}

/// Option `alphabet` : vrai pour `base64url`.
fn alphabet_option(vm: &mut Vm, options: Option<&Value>) -> JsResult<bool> {
    Ok(string_option(vm, options, "alphabet", &["base64", "base64url"])? == Some(1))
}

fn last_chunk_option(vm: &mut Vm, options: Option<&Value>) -> JsResult<LastChunk> {
    let choice = string_option(
        vm,
        options,
        "lastChunkHandling",
        &["loose", "strict", "stop-before-partial"],
    )?;
    Ok(match choice {
        Some(1) => LastChunk::Strict,
        Some(2) => LastChunk::StopBeforePartial,
        _ => LastChunk::Loose,
    })
}

/// `Uint8Array` désigné par `this` ; `TypeError` pour tout autre objet.
fn this_bytes(vm: &mut Vm, this: &Value) -> JsResult<ObjId> {
    match this.as_object().filter(|id| {
        vm.typed_array(*id)
            .is_some_and(|view| view.kind == ElementKind::Uint8)
    }) {
        Some(id) => Ok(id),
        None => Err(vm.type_error("le receveur n'est pas un Uint8Array")),
    }
}

/// Nouveau `Uint8Array` contenant `bytes`.
fn new_bytes(vm: &mut Vm, bytes: &[u8]) -> JsResult {
    // Le constructeur d'origine, pas celui que le script aurait pu remplacer.
    let constructor = vm.uint8_array_constructor().unwrap_or(Value::Undefined);
    let created = vm.construct(&constructor, &[number(bytes.len())], &constructor)?;
    if let Some(id) = created.as_object() {
        for (index, byte) in bytes.iter().enumerate() {
            vm.typed_set(id, index, &Value::Number(f64::from(*byte)));
        }
    }
    Ok(created)
}

fn from_base64_static(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let units = string_units(vm, &arg(args, 0))?;
    let options = options_object(vm, &arg(args, 1))?;
    let url_safe = alphabet_option(vm, options.as_ref())?;
    let last_chunk = last_chunk_option(vm, options.as_ref())?;
    let decoded = from_base64(&units, url_safe, last_chunk, usize::MAX);
    if let Some(message) = decoded.error {
        return Err(vm.error("SyntaxError", message));
    }
    new_bytes(vm, &decoded.bytes)
}

fn from_hex_static(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let units = string_units(vm, &arg(args, 0))?;
    let decoded = from_hex(&units, usize::MAX);
    if let Some(message) = decoded.error {
        return Err(vm.error("SyntaxError", message));
    }
    new_bytes(vm, &decoded.bytes)
}

/// Octets du `Uint8Array` `id`, qui doit être vivant.
fn bytes_of(vm: &mut Vm, this: &Value, id: ObjId) -> JsResult<Vec<u8>> {
    let (_, length) = validate(vm, this)?;
    Ok((0..length)
        .map(|index| match vm.typed_get(id, index) {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "élément d'un Uint8Array : entier de 0 à 255"
            )]
            Some(Value::Number(n)) => n as u8,
            _ => 0,
        })
        .collect())
}

fn to_base64(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_bytes(vm, this)?;
    let options = options_object(vm, &arg(args, 0))?;
    let url_safe = alphabet_option(vm, options.as_ref())?;
    let omit_padding = match &options {
        Some(options) => vm.get(options, "omitPadding")?.truthy(),
        None => false,
    };
    let bytes = bytes_of(vm, this, id)?;
    let alphabet = if url_safe { URL_SAFE } else { STANDARD };
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for group in bytes.chunks(3) {
        let b = [
            group[0],
            group.get(1).copied().unwrap_or(0),
            group.get(2).copied().unwrap_or(0),
        ];
        let sextets = [
            b[0] >> 2,
            ((b[0] & 0x03) << 4) | (b[1] >> 4),
            ((b[1] & 0x0f) << 2) | (b[2] >> 6),
            b[2] & 0x3f,
        ];
        // Un groupe de n octets donne n + 1 caractères, puis du remplissage.
        for (position, value) in sextets.iter().enumerate() {
            if position <= group.len() {
                out.push(char::from(alphabet[usize::from(*value)]));
            } else if !omit_padding {
                out.push('=');
            }
        }
    }
    Ok(Value::string(&out))
}

fn to_hex(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let id = this_bytes(vm, this)?;
    let bytes = bytes_of(vm, this, id)?;
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    Ok(Value::string(&out))
}

/// Écrit `decoded` au début du tableau `id` et rend `{ read, written }` ;
/// l'erreur de décodage, s'il y en a une, n'est levée qu'après l'écriture.
fn write_decoded(vm: &mut Vm, id: ObjId, decoded: &Decoded) -> JsResult {
    for (index, byte) in decoded.bytes.iter().enumerate() {
        vm.typed_set(id, index, &Value::Number(f64::from(*byte)));
    }
    if let Some(message) = decoded.error {
        return Err(vm.error("SyntaxError", message));
    }
    let result = Value::Object(vm.new_object());
    vm.set(&result, "read", number(decoded.read))?;
    vm.set(&result, "written", number(decoded.bytes.len()))?;
    Ok(result)
}

fn set_from_base64(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_bytes(vm, this)?;
    let units = string_units(vm, &arg(args, 0))?;
    let options = options_object(vm, &arg(args, 1))?;
    let url_safe = alphabet_option(vm, options.as_ref())?;
    let last_chunk = last_chunk_option(vm, options.as_ref())?;
    let (_, length) = validate(vm, this)?;
    let decoded = from_base64(&units, url_safe, last_chunk, length);
    write_decoded(vm, id, &decoded)
}

fn set_from_hex(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_bytes(vm, this)?;
    let units = string_units(vm, &arg(args, 0))?;
    let (_, length) = validate(vm, this)?;
    let decoded = from_hex(&units, length);
    write_decoded(vm, id, &decoded)
}

/// Installe les méthodes sur le constructeur `Uint8Array` et son prototype.
pub(super) fn install(vm: &mut Vm, constructor: ObjId, prototype: ObjId) {
    vm.define_method(constructor, "fromBase64", 1, from_base64_static);
    vm.define_method(constructor, "fromHex", 1, from_hex_static);
    let methods: &[(&str, u32, NativeFn)] = &[
        ("setFromBase64", 1, set_from_base64),
        ("setFromHex", 1, set_from_hex),
        ("toBase64", 0, to_base64),
        ("toHex", 0, to_hex),
    ];
    for (name, arity, func) in methods {
        vm.define_method(prototype, name, *arity, *func);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn units(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    #[test]
    fn base64_decoding_follows_the_last_chunk_rules() {
        let decode = |text: &str, last: LastChunk| {
            let d = from_base64(&units(text), false, last, usize::MAX);
            (d.bytes, d.read, d.error.is_some())
        };
        assert_eq!(
            decode("Zm9v", LastChunk::Loose),
            (b"foo".to_vec(), 4, false)
        );
        assert_eq!(
            decode(" Zm 9v\n", LastChunk::Loose),
            (b"foo".to_vec(), 7, false)
        );
        // Dernier groupe incomplet.
        assert_eq!(decode("Zm8", LastChunk::Loose), (b"fo".to_vec(), 3, false));
        assert_eq!(
            decode("Zm8=", LastChunk::Strict),
            (b"fo".to_vec(), 4, false)
        );
        assert!(decode("Zm8", LastChunk::Strict).2);
        assert_eq!(
            decode("Zm9vZm8", LastChunk::StopBeforePartial),
            (b"foo".to_vec(), 4, false)
        );
        // Bits excédentaires : tolérés en mode souple, refusés en mode strict.
        assert_eq!(decode("Zm9=", LastChunk::Loose), (b"fo".to_vec(), 4, false));
        assert!(decode("Zm9=", LastChunk::Strict).2);
        // Erreurs : ce qui précède reste décodé.
        assert_eq!(
            decode("Zm9v*", LastChunk::Loose),
            (b"foo".to_vec(), 4, true)
        );
        assert!(decode("Z", LastChunk::Loose).2);
        assert!(decode("Zm9v=", LastChunk::Loose).2);
        // Alphabet pour adresses.
        let url = from_base64(&units("_-8"), true, LastChunk::Loose, usize::MAX);
        assert_eq!((url.bytes, url.error.is_some()), (vec![0xff, 0xef], false));
        assert!(
            from_base64(&units("+/8"), true, LastChunk::Loose, usize::MAX)
                .error
                .is_some()
        );
        // Place limitée : le décodage s'arrête sur un groupe entier.
        let limited = from_base64(&units("Zm9vYmFy"), false, LastChunk::Loose, 4);
        assert_eq!((limited.bytes, limited.read), (b"foo".to_vec(), 4));
    }

    #[test]
    fn hex_decoding() {
        let decode = |text: &str, max: usize| {
            let d = from_hex(&units(text), max);
            (d.bytes, d.read, d.error.is_some())
        };
        assert_eq!(decode("00fF10", usize::MAX), (vec![0, 255, 16], 6, false));
        assert_eq!(decode("00ff10", 2), (vec![0, 255], 4, false));
        assert_eq!(decode("0", usize::MAX), (Vec::new(), 0, true));
        assert_eq!(decode("00zz", usize::MAX), (vec![0], 2, true));
    }
}
