//! Primitives de `crypto.subtle` : ce que le moteur fournit à `subtle.js`.
//!
//! Les calculs viennent des crates RustCrypto (`sha2`, `hmac`, `aes-gcm`,
//! `hkdf`) ; SHA-1, que ces crates ne fournissent pas ici, est écrit plus
//! bas : il ne sert qu'à la compatibilité (condensats et HMAC hérités), pas
//! à protéger quoi que ce soit de neuf.
//!
//! Les octets voyagent entre le moteur et le script sous forme de « chaînes
//! binaires » : un caractère de code 0 à 255 par octet.
//!
//! LIMITES CONNUES : voir l'en-tête de `subtle.js`. Les clés vivent dans la
//! mémoire de la page ; les calculs ne sont pas à temps constant au-delà de
//! ce que garantissent les crates employés (la comparaison d'une signature
//! HMAC l'est).

use aes_gcm::aead::{Aead, KeyInit, Payload};
use hmac::Mac;
use oxyde_js::{JsResult, Value, Vm};
use sha2::Digest;

use super::arg;

/// Nombre maximal d'itérations PBKDF2 acceptées : au-delà, le calcul
/// figerait la page.
const MAX_PBKDF2_ITERATIONS: u32 = 10_000_000;
/// Longueur maximale d'une dérivation, en octets.
const MAX_DERIVED_BYTES: usize = 1 << 16;

/// Octets d'une chaîne binaire.
fn bytes_of(vm: &mut Vm, value: &Value) -> JsResult<Vec<u8>> {
    let text = vm.to_string(value)?;
    Ok(text
        .chars()
        .filter_map(|c| u8::try_from(u32::from(c)).ok())
        .collect())
}

/// Chaîne binaire de `bytes`.
fn binary(bytes: &[u8]) -> Value {
    let text: String = bytes.iter().map(|byte| char::from(*byte)).collect();
    Value::string(&text)
}

/// Fonction de hachage que désigne un nom WebCrypto.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Hash {
    Sha1,
    Sha256,
    Sha384,
    Sha512,
}

impl Hash {
    fn named(name: &str) -> Option<Self> {
        match name.to_ascii_uppercase().as_str() {
            "SHA-1" => Some(Self::Sha1),
            "SHA-256" => Some(Self::Sha256),
            "SHA-384" => Some(Self::Sha384),
            "SHA-512" => Some(Self::Sha512),
            _ => None,
        }
    }

    /// Taille d'un bloc de la fonction, en octets (pour HMAC).
    fn block_len(self) -> usize {
        match self {
            Self::Sha1 | Self::Sha256 => 64,
            Self::Sha384 | Self::Sha512 => 128,
        }
    }

    /// Taille du condensat, en octets.
    fn output_len(self) -> usize {
        match self {
            Self::Sha1 => 20,
            Self::Sha256 => 32,
            Self::Sha384 => 48,
            Self::Sha512 => 64,
        }
    }

    fn digest(self, data: &[u8]) -> Vec<u8> {
        match self {
            Self::Sha1 => sha1(data).to_vec(),
            Self::Sha256 => sha2::Sha256::digest(data).to_vec(),
            Self::Sha384 => sha2::Sha384::digest(data).to_vec(),
            Self::Sha512 => sha2::Sha512::digest(data).to_vec(),
        }
    }

    /// HMAC de `data` sous la clé `key`.
    fn hmac(self, key: &[u8], data: &[u8]) -> Vec<u8> {
        macro_rules! with {
            ($hash:ty) => {
                match hmac::Hmac::<$hash>::new_from_slice(key) {
                    Ok(mut mac) => {
                        mac.update(data);
                        mac.finalize().into_bytes().to_vec()
                    }
                    // Une clé HMAC peut avoir n'importe quelle longueur.
                    Err(_) => Vec::new(),
                }
            };
        }
        match self {
            Self::Sha1 => self.plain_hmac(key, data),
            Self::Sha256 => with!(sha2::Sha256),
            Self::Sha384 => with!(sha2::Sha384),
            Self::Sha512 => with!(sha2::Sha512),
        }
    }

    /// HMAC écrit d'après la RFC 2104, pour la fonction sans crate HMAC.
    fn plain_hmac(self, key: &[u8], data: &[u8]) -> Vec<u8> {
        let block = self.block_len();
        let mut padded = if key.len() > block {
            self.digest(key)
        } else {
            key.to_vec()
        };
        padded.resize(block, 0);
        let mut inner: Vec<u8> = padded.iter().map(|byte| byte ^ 0x36).collect();
        inner.extend_from_slice(data);
        let mut outer: Vec<u8> = padded.iter().map(|byte| byte ^ 0x5c).collect();
        outer.extend_from_slice(&self.digest(&inner));
        self.digest(&outer)
    }
}

/// Condensat SHA-1 (RFC 3174) de `data`.
fn sha1(data: &[u8]) -> [u8; 20] {
    let mut state: [u32; 5] = [
        0x6745_2301,
        0xEFCD_AB89,
        0x98BA_DCFE,
        0x1032_5476,
        0xC3D2_E1F0,
    ];
    let mut message = data.to_vec();
    let bits = u64::try_from(data.len())
        .unwrap_or(u64::MAX)
        .wrapping_mul(8);
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bits.to_be_bytes());
    let (blocks, _) = message.as_chunks::<64>();
    for block in blocks {
        let mut words = [0u32; 80];
        let (quads, _) = block.as_chunks::<4>();
        for (word, quad) in words.iter_mut().zip(quads) {
            *word = u32::from_be_bytes(*quad);
        }
        for i in 16..80 {
            let mixed = words[i - 3] ^ words[i - 8] ^ words[i - 14] ^ words[i - 16];
            words[i] = mixed.rotate_left(1);
        }
        let [mut a, mut b, mut c, mut d, mut e] = state;
        for (i, word) in words.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | (!b & d), 0x5A82_7999),
                20..=39 => (b ^ c ^ d, 0x6ED9_EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC),
                _ => (b ^ c ^ d, 0xCA62_C1D6_u32),
            };
            let next = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(*word);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = next;
        }
        for (slot, value) in state.iter_mut().zip([a, b, c, d, e]) {
            *slot = slot.wrapping_add(value);
        }
    }
    let mut digest = [0u8; 20];
    for (out, word) in digest.as_chunks_mut::<4>().0.iter_mut().zip(state) {
        out.copy_from_slice(&word.to_be_bytes());
    }
    digest
}

/// Fonction de hachage nommée par l'argument `value`, ou `None`.
fn hash_arg(vm: &mut Vm, value: &Value) -> JsResult<Option<Hash>> {
    Ok(Hash::named(&vm.to_string(value)?))
}

/// `cryptoDigest(fonction, données)` : condensat, ou `null` si la fonction
/// est inconnue.
pub(super) fn digest(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let Some(hash) = hash_arg(vm, &arg(args, 0))? else {
        return Ok(Value::Null);
    };
    let data = bytes_of(vm, &arg(args, 1))?;
    Ok(binary(&hash.digest(&data)))
}

/// `cryptoHmac(fonction, clé, données)` : signature HMAC, ou `null`.
pub(super) fn hmac(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let Some(hash) = hash_arg(vm, &arg(args, 0))? else {
        return Ok(Value::Null);
    };
    let key = bytes_of(vm, &arg(args, 1))?;
    let data = bytes_of(vm, &arg(args, 2))?;
    Ok(binary(&hash.hmac(&key, &data)))
}

/// `cryptoEqual(a, b)` : vrai si les deux chaînes binaires sont égales,
/// comparées sans s'arrêter à la première différence (vérification d'une
/// signature).
pub(super) fn equal(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let a = bytes_of(vm, &arg(args, 0))?;
    let b = bytes_of(vm, &arg(args, 1))?;
    let mut difference = u8::from(a.len() != b.len());
    for (x, y) in a.iter().zip(&b) {
        difference |= x ^ y;
    }
    Ok(Value::Bool(difference == 0))
}

/// Chiffre ou déchiffre en AES-GCM ; `None` si la clé n'a pas 16 ou 32
/// octets, si le vecteur n'en a pas 12, ou si le texte chiffré est altéré.
fn aes_gcm(encrypt: bool, key: &[u8], iv: &[u8], data: &[u8], aad: &[u8]) -> Option<Vec<u8>> {
    let payload = Payload { msg: data, aad };
    macro_rules! with {
        ($cipher:ty) => {{
            let cipher = <$cipher>::new_from_slice(key).ok()?;
            let nonce = aes_gcm::Nonce::try_from(iv).ok()?;
            if encrypt {
                cipher.encrypt(&nonce, payload).ok()
            } else {
                cipher.decrypt(&nonce, payload).ok()
            }
        }};
    }
    match key.len() {
        16 => with!(aes_gcm::Aes128Gcm),
        32 => with!(aes_gcm::Aes256Gcm),
        _ => None,
    }
}

/// `cryptoAesGcm(chiffrer, clé, vecteur, données, donnéesAssociées)` : le
/// texte chiffré suivi de son sceau de 16 octets, ou le texte clair ; `null`
/// si l'opération échoue.
pub(super) fn aes_gcm_native(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let encrypt = arg(args, 0).truthy();
    let key = bytes_of(vm, &arg(args, 1))?;
    let iv = bytes_of(vm, &arg(args, 2))?;
    let data = bytes_of(vm, &arg(args, 3))?;
    let aad = bytes_of(vm, &arg(args, 4))?;
    Ok(match aes_gcm(encrypt, &key, &iv, &data, &aad) {
        Some(out) => binary(&out),
        None => Value::Null,
    })
}

/// Longueur demandée par le script, bornée.
fn length_arg(vm: &mut Vm, value: &Value) -> JsResult<Option<usize>> {
    let number = vm.to_number(value)?;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "valeur bornée par le filtre"
    )]
    Ok(Some(number)
        .filter(|n| n.is_finite() && (0.0..=MAX_DERIVED_BYTES as f64).contains(n))
        .map(|n| n as usize))
}

/// `cryptoPbkdf2(fonction, motDePasse, sel, itérations, longueur)` : clé
/// dérivée (RFC 8018), ou `null` si un paramètre est refusé.
pub(super) fn pbkdf2(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let Some(hash) = hash_arg(vm, &arg(args, 0))? else {
        return Ok(Value::Null);
    };
    let password = bytes_of(vm, &arg(args, 1))?;
    let salt = bytes_of(vm, &arg(args, 2))?;
    let iterations = vm.to_number(&arg(args, 3))?;
    let Some(length) = length_arg(vm, &arg(args, 4))? else {
        return Ok(Value::Null);
    };
    if !(1.0..=f64::from(MAX_PBKDF2_ITERATIONS)).contains(&iterations) {
        return Ok(Value::Null);
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "valeur bornée juste au-dessus"
    )]
    let iterations = iterations as u32;
    let mut derived = Vec::with_capacity(length + hash.output_len());
    let mut block: u32 = 1;
    while derived.len() < length {
        let mut seed = salt.clone();
        seed.extend_from_slice(&block.to_be_bytes());
        let mut round = hash.hmac(&password, &seed);
        let mut folded = round.clone();
        for _ in 1..iterations {
            round = hash.hmac(&password, &round);
            for (acc, byte) in folded.iter_mut().zip(&round) {
                *acc ^= byte;
            }
        }
        derived.extend_from_slice(&folded);
        block = block.wrapping_add(1);
    }
    derived.truncate(length);
    Ok(binary(&derived))
}

/// `cryptoHkdf(fonction, clé, sel, info, longueur)` : clé dérivée (RFC
/// 5869), ou `null` si un paramètre est refusé.
pub(super) fn hkdf(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let Some(hash) = hash_arg(vm, &arg(args, 0))? else {
        return Ok(Value::Null);
    };
    let key = bytes_of(vm, &arg(args, 1))?;
    let salt = bytes_of(vm, &arg(args, 2))?;
    let info = bytes_of(vm, &arg(args, 3))?;
    let Some(length) = length_arg(vm, &arg(args, 4))? else {
        return Ok(Value::Null);
    };
    // La norme borne la sortie à 255 blocs de la fonction.
    if length > 255 * hash.output_len() {
        return Ok(Value::Null);
    }
    // Extraction, puis expansion : écrites avec le HMAC de ce module pour
    // couvrir aussi SHA-1.
    let salt = if salt.is_empty() {
        vec![0; hash.output_len()]
    } else {
        salt
    };
    let pseudo_random = hash.hmac(&salt, &key);
    let mut derived = Vec::with_capacity(length + hash.output_len());
    let mut previous: Vec<u8> = Vec::new();
    let mut counter: u8 = 1;
    while derived.len() < length {
        let mut input = previous.clone();
        input.extend_from_slice(&info);
        input.push(counter);
        previous = hash.hmac(&pseudo_random, &input);
        derived.extend_from_slice(&previous);
        counter = counter.wrapping_add(1);
    }
    derived.truncate(length);
    Ok(binary(&derived))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn digests_match_published_vectors() {
        assert_eq!(
            hex(&Hash::Sha1.digest(b"abc")),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        assert_eq!(
            hex(&Hash::Sha256.digest(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            hex(&Hash::Sha384.digest(b"abc")),
            "cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed\
             8086072ba1e7cc2358baeca134c825a7"
        );
        assert_eq!(
            hex(&Hash::Sha512.digest(b"abc")),
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a\
             2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
        );
    }

    #[test]
    fn hmac_matches_rfc_vectors() {
        // RFC 4231, cas 2, et RFC 2202, cas 2.
        let (key, data) = (b"Jefe", b"what do ya want for nothing?");
        assert_eq!(
            hex(&Hash::Sha256.hmac(key, data)),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
        assert_eq!(
            hex(&Hash::Sha1.hmac(key, data)),
            "effcdf6ae5eb2fa2d27416d5f184df9c259a7c79"
        );
        // Le HMAC écrit ici et celui du crate s'accordent, clé longue comprise.
        let long = [0xaa_u8; 131];
        assert_eq!(
            Hash::Sha256.plain_hmac(&long, data),
            Hash::Sha256.hmac(&long, data)
        );
        assert_eq!(
            Hash::Sha512.plain_hmac(key, data),
            Hash::Sha512.hmac(key, data)
        );
    }

    #[test]
    fn aes_gcm_matches_the_standard_vector_and_detects_tampering() {
        // Cas d'essai 2 de la spécification de GCM : clé et bloc nuls.
        let sealed = aes_gcm(true, &[0; 16], &[0; 12], &[0; 16], b"").expect("chiffrement");
        assert_eq!(
            hex(&sealed),
            "0388dace60b6a392f328c2b971b2fe78ab6e47d42cec13bdf53a67b21257bddf"
        );
        assert_eq!(
            aes_gcm(false, &[0; 16], &[0; 12], &sealed, b""),
            Some(vec![0; 16])
        );
        let mut altered = sealed.clone();
        altered[0] ^= 1;
        assert_eq!(aes_gcm(false, &[0; 16], &[0; 12], &altered, b""), None);
        assert_eq!(aes_gcm(false, &[0; 16], &[0; 12], &sealed, b"autre"), None);
        // Clé de 256 bits ; clé ou vecteur de longueur refusée.
        let sealed = aes_gcm(true, &[7; 32], &[1; 12], b"secret", b"aad").expect("chiffrement");
        assert_eq!(
            aes_gcm(false, &[7; 32], &[1; 12], &sealed, b"aad").as_deref(),
            Some(&b"secret"[..])
        );
        assert_eq!(aes_gcm(true, &[0; 24], &[0; 12], b"x", b""), None);
        assert_eq!(aes_gcm(true, &[0; 16], &[0; 16], b"x", b""), None);
    }
}
