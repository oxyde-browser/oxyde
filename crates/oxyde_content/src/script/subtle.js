// `crypto.subtle` (Web Cryptography API), écrit en JavaScript par-dessus les
// primitives du moteur.
//
// Fonction appelée par le moteur avec (les octets sont des chaînes binaires,
// un caractère de code 0 à 255 par octet) :
//   cryptoDigest(fonction, données)            -> condensat, ou null ;
//   cryptoHmac(fonction, clé, données)         -> signature, ou null ;
//   cryptoEqual(a, b)                          -> égalité, sans arrêt précoce ;
//   cryptoAesGcm(chiffrer, clé, iv, données, donnéesAssociées)
//                                              -> résultat, ou null si l'opération échoue ;
//   cryptoPbkdf2(fonction, motDePasse, sel, itérations, longueur) -> octets, ou null ;
//   cryptoHkdf(fonction, clé, sel, info, longueur)                -> octets, ou null.
//
// Pris en charge : `digest` (SHA-1, SHA-256, SHA-384, SHA-512) ; HMAC
// (`generateKey`, `importKey`, `exportKey`, `sign`, `verify`) ; AES-GCM à clé
// de 128 ou 256 bits (`generateKey`, `importKey`, `exportKey`, `encrypt`,
// `decrypt`) ; PBKDF2 et HKDF (`importKey`, `deriveBits`, `deriveKey`) ;
// formats de clé `raw` et `jwk`.
//
// LIMITES CONNUES : pas de cryptographie à clé publique (RSA, ECDSA, ECDH,
// Ed25519), ni AES-CBC, AES-CTR, AES-KW ; AES-GCM seulement avec un vecteur
// de 12 octets et un sceau de 128 bits ; pas de `wrapKey` ni `unwrapKey` ;
// `crypto.subtle` existe aussi hors d'un contexte sûr ; une clé n'est pas
// clonable par `structuredClone`.
(function (cryptoDigest, cryptoHmac, cryptoEqual, cryptoAesGcm, cryptoPbkdf2, cryptoHkdf) {
  "use strict";
  const global = globalThis;
  const define = Object.defineProperty;
  const DOMException = global.DOMException;
  const fail = (name, message) => new DOMException(message, name);

  const toBinary = (bytes) => {
    let out = "";
    for (let i = 0; i < bytes.length; i += 8192) {
      out += String.fromCharCode.apply(null, bytes.subarray(i, i + 8192));
    }
    return out;
  };
  const fromBinary = (text) => {
    const bytes = new Uint8Array(text.length);
    for (let i = 0; i < text.length; i++) bytes[i] = text.charCodeAt(i);
    return bytes;
  };
  // Copie des octets d'un tampon ou d'une vue.
  const bytesOf = (source, what) => {
    if (source instanceof ArrayBuffer) return new Uint8Array(source.slice(0));
    if (ArrayBuffer.isView(source)) {
      return new Uint8Array(source.buffer.slice(source.byteOffset, source.byteOffset + source.byteLength));
    }
    throw new TypeError(what + " : un tampon d'octets est attendu");
  };
  const base64url = (bytes) => global.btoa(toBinary(bytes)).replace(/\+/g, "-").replace(/\//g, "_")
    .replace(/=+$/, "");
  const fromBase64url = (text) => {
    try {
      return fromBinary(global.atob(String(text).replace(/-/g, "+").replace(/_/g, "/")));
    } catch {
      throw fail("DataError", "clé JWK illisible");
    }
  };

  const HASHES = { "SHA-1": 512, "SHA-256": 512, "SHA-384": 1024, "SHA-512": 1024 };
  const JWK_HMAC = { "SHA-1": "HS1", "SHA-256": "HS256", "SHA-384": "HS384", "SHA-512": "HS512" };
  const NAMES = ["HMAC", "AES-GCM", "PBKDF2", "HKDF", ...Object.keys(HASHES)];
  // Algorithme sous sa forme d'objet, au nom écrit comme la norme l'écrit.
  const normalize = (algorithm) => {
    const given = typeof algorithm === "string" ? { name: algorithm } : algorithm;
    if (given === null || typeof given !== "object" || given.name === undefined) {
      throw new TypeError("algorithme attendu : un nom ou un objet avec `name`");
    }
    const wanted = String(given.name).toUpperCase();
    const name = NAMES.find((known) => known === wanted);
    if (name === undefined) throw fail("NotSupportedError", "algorithme non pris en charge : " + given.name);
    return { ...given, name };
  };
  const hashOf = (algorithm) => {
    if (algorithm.hash === undefined) throw new TypeError("l'algorithme doit nommer sa fonction de hachage");
    const hash = normalize(algorithm.hash).name;
    if (!(hash in HASHES)) throw fail("NotSupportedError", "fonction de hachage non prise en charge");
    return hash;
  };

  // ---- CryptoKey -----------------------------------------------------------------------
  const secrets = new WeakMap();
  let building = false;
  class CryptoKey {
    #type;
    #extractable;
    #algorithm;
    #usages;
    constructor(type, extractable, algorithm, usages) {
      if (!building) throw new TypeError("constructeur illégal");
      this.#type = type;
      this.#extractable = extractable;
      this.#algorithm = Object.freeze(algorithm);
      this.#usages = Object.freeze([...usages]);
    }
    get type() { return this.#type; }
    get extractable() { return this.#extractable; }
    get algorithm() { return this.#algorithm; }
    get usages() { return this.#usages; }
  }
  define(CryptoKey.prototype, Symbol.toStringTag, { value: "CryptoKey", configurable: true });
  const USAGES = {
    HMAC: ["sign", "verify"],
    "AES-GCM": ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
    PBKDF2: ["deriveKey", "deriveBits"],
    HKDF: ["deriveKey", "deriveBits"],
  };
  const makeKey = (bytes, algorithm, extractable, usages) => {
    const list = Array.from(usages ?? [], String);
    const allowed = USAGES[algorithm.name];
    if (list.some((usage) => !allowed.includes(usage))) {
      throw fail("SyntaxError", "usage de clé incompatible avec " + algorithm.name);
    }
    if (list.length === 0) throw fail("SyntaxError", "une clé secrète doit avoir au moins un usage");
    building = true;
    let key;
    try {
      key = new CryptoKey("secret", Boolean(extractable), algorithm, [...new Set(list)]);
    } finally {
      building = false;
    }
    secrets.set(key, bytes);
    return key;
  };
  // Octets de `key`, après contrôle de son algorithme et de son usage.
  const secretOf = (key, name, usage) => {
    const bytes = secrets.get(key);
    if (bytes === undefined) throw new TypeError("une clé (CryptoKey) est attendue");
    if (key.algorithm.name !== name) {
      throw fail("InvalidAccessError", "cette clé n'est pas une clé " + name);
    }
    if (!key.usages.includes(usage)) {
      throw fail("InvalidAccessError", "cette clé ne permet pas « " + usage + " »");
    }
    return bytes;
  };
  // Algorithme enregistré dans une clé, et octets attendus.
  const keyAlgorithm = (algorithm, length) => {
    if (algorithm.name === "HMAC") {
      const hash = hashOf(algorithm);
      const bits = algorithm.length === undefined ? (length ?? HASHES[hash]) : Number(algorithm.length);
      if (!(bits > 0)) throw fail("DataError", "longueur de clé HMAC invalide");
      return { name: "HMAC", hash: { name: hash }, length: bits };
    }
    if (algorithm.name === "AES-GCM") {
      const bits = length ?? Number(algorithm.length);
      if (bits === 192) throw fail("NotSupportedError", "AES à clé de 192 bits non pris en charge");
      if (bits !== 128 && bits !== 256) throw fail("DataError", "une clé AES fait 128 ou 256 bits");
      return { name: "AES-GCM", length: bits };
    }
    if (algorithm.name === "PBKDF2" || algorithm.name === "HKDF") return { name: algorithm.name };
    throw fail("NotSupportedError", algorithm.name + " n'a pas de clé");
  };

  // ---- Opérations ------------------------------------------------------------------------
  const derive = (algorithm, baseKey, length) => {
    const bytes = Math.floor(length / 8);
    if (!(length > 0) || length % 8 !== 0) throw fail("OperationError", "longueur à dériver invalide");
    const hash = hashOf(algorithm);
    let out;
    if (algorithm.name === "PBKDF2") {
      const iterations = Number(algorithm.iterations);
      if (!(iterations >= 1)) throw fail("OperationError", "nombre d'itérations invalide");
      out = cryptoPbkdf2(hash, toBinary(baseKey), toBinary(bytesOf(algorithm.salt, "sel")), iterations, bytes);
    } else {
      out = cryptoHkdf(hash, toBinary(baseKey), toBinary(bytesOf(algorithm.salt, "sel")),
        toBinary(bytesOf(algorithm.info, "info")), bytes);
    }
    if (out === null) throw fail("OperationError", "dérivation refusée");
    return fromBinary(out);
  };
  // Chaque méthode rend une promesse : une faute devient un rejet.
  const promised = (work) => new Promise((resolve) => resolve(work()));

  class SubtleCrypto {
    constructor() {
      if (!building) throw new TypeError("constructeur illégal");
    }
    digest(algorithm, data) {
      return promised(() => {
        const name = normalize(algorithm).name;
        if (!(name in HASHES)) throw fail("NotSupportedError", name + " n'est pas une fonction de hachage");
        return fromBinary(cryptoDigest(name, toBinary(bytesOf(data, "digest")))).buffer;
      });
    }
    generateKey(algorithm, extractable, usages) {
      return promised(() => {
        const wanted = normalize(algorithm);
        if (wanted.name === "PBKDF2" || wanted.name === "HKDF") {
          throw fail("NotSupportedError", wanted.name + " n'engendre pas de clé");
        }
        const stored = keyAlgorithm(wanted);
        const bytes = global.crypto.getRandomValues(new Uint8Array(Math.ceil(stored.length / 8)));
        return makeKey(bytes, stored, extractable, usages);
      });
    }
    importKey(format, keyData, algorithm, extractable, usages) {
      return promised(() => {
        const wanted = normalize(algorithm);
        let bytes;
        if (format === "raw") {
          bytes = bytesOf(keyData, "importKey");
        } else if (format === "jwk") {
          if (keyData === null || typeof keyData !== "object" || keyData.kty !== "oct") {
            throw fail("DataError", "clé JWK attendue, de type « oct »");
          }
          if (wanted.name !== "HMAC" && wanted.name !== "AES-GCM") {
            throw fail("NotSupportedError", "format JWK non pris en charge pour " + wanted.name);
          }
          bytes = fromBase64url(keyData.k);
        } else {
          throw fail("NotSupportedError", "format de clé non pris en charge : " + format);
        }
        if (wanted.name === "PBKDF2" || wanted.name === "HKDF") {
          if (extractable) throw fail("SyntaxError", "une clé " + wanted.name + " n'est pas extractible");
          return makeKey(bytes, { name: wanted.name }, false, usages);
        }
        if (bytes.length === 0) throw fail("DataError", "clé vide");
        return makeKey(bytes, keyAlgorithm(wanted, bytes.length * 8), extractable, usages);
      });
    }
    exportKey(format, key) {
      return promised(() => {
        const bytes = secrets.get(key);
        if (bytes === undefined) throw new TypeError("une clé (CryptoKey) est attendue");
        if (!key.extractable) throw fail("InvalidAccessError", "cette clé n'est pas extractible");
        if (format === "raw") return bytes.slice().buffer;
        if (format !== "jwk") throw fail("NotSupportedError", "format de clé non pris en charge : " + format);
        const alg = key.algorithm.name === "HMAC"
          ? JWK_HMAC[key.algorithm.hash.name]
          : "A" + key.algorithm.length + "GCM";
        return { kty: "oct", k: base64url(bytes), alg, ext: true, key_ops: [...key.usages] };
      });
    }
    sign(algorithm, key, data) {
      return promised(() => {
        const wanted = normalize(algorithm);
        if (wanted.name !== "HMAC") throw fail("NotSupportedError", wanted.name + " ne signe pas");
        const secret = secretOf(key, "HMAC", "sign");
        const mac = cryptoHmac(key.algorithm.hash.name, toBinary(secret), toBinary(bytesOf(data, "sign")));
        return fromBinary(mac).buffer;
      });
    }
    verify(algorithm, key, signature, data) {
      return promised(() => {
        const wanted = normalize(algorithm);
        if (wanted.name !== "HMAC") throw fail("NotSupportedError", wanted.name + " ne vérifie pas");
        const secret = secretOf(key, "HMAC", "verify");
        const mac = cryptoHmac(key.algorithm.hash.name, toBinary(secret), toBinary(bytesOf(data, "verify")));
        return cryptoEqual(mac, toBinary(bytesOf(signature, "verify")));
      });
    }
    #cipher(encrypting, algorithm, key, data) {
      const wanted = normalize(algorithm);
      if (wanted.name !== "AES-GCM") throw fail("NotSupportedError", wanted.name + " ne chiffre pas");
      const secret = secretOf(key, "AES-GCM", encrypting ? "encrypt" : "decrypt");
      const iv = bytesOf(wanted.iv, "iv");
      if (wanted.tagLength !== undefined && Number(wanted.tagLength) !== 128) {
        throw fail("NotSupportedError", "seul un sceau de 128 bits est pris en charge");
      }
      if (iv.length !== 12) throw fail("NotSupportedError", "seul un vecteur de 12 octets est pris en charge");
      const aad = wanted.additionalData === undefined ? new Uint8Array(0) : bytesOf(wanted.additionalData, "aad");
      const out = cryptoAesGcm(encrypting, toBinary(secret), toBinary(iv),
        toBinary(bytesOf(data, encrypting ? "encrypt" : "decrypt")), toBinary(aad));
      if (out === null) throw fail("OperationError", encrypting ? "chiffrement refusé" : "déchiffrement refusé");
      return fromBinary(out).buffer;
    }
    encrypt(algorithm, key, data) {
      return promised(() => this.#cipher(true, algorithm, key, data));
    }
    decrypt(algorithm, key, data) {
      return promised(() => this.#cipher(false, algorithm, key, data));
    }
    deriveBits(algorithm, baseKey, length) {
      return promised(() => {
        const wanted = normalize(algorithm);
        if (wanted.name !== "PBKDF2" && wanted.name !== "HKDF") {
          throw fail("NotSupportedError", wanted.name + " ne dérive pas");
        }
        return derive(wanted, secretOf(baseKey, wanted.name, "deriveBits"), Number(length)).buffer;
      });
    }
    deriveKey(algorithm, baseKey, derivedKeyAlgorithm, extractable, usages) {
      return promised(() => {
        const wanted = normalize(algorithm);
        if (wanted.name !== "PBKDF2" && wanted.name !== "HKDF") {
          throw fail("NotSupportedError", wanted.name + " ne dérive pas");
        }
        const stored = keyAlgorithm(normalize(derivedKeyAlgorithm));
        if (stored.length === undefined) throw fail("NotSupportedError", "clé dérivée sans longueur");
        const bytes = derive(wanted, secretOf(baseKey, wanted.name, "deriveKey"), stored.length);
        return makeKey(bytes, stored, extractable, usages);
      });
    }
  }
  define(SubtleCrypto.prototype, Symbol.toStringTag, { value: "SubtleCrypto", configurable: true });
  building = true;
  const subtle = new SubtleCrypto();
  building = false;
  define(global.crypto, "subtle", { value: subtle, enumerable: true, configurable: true });
  for (const [name, value] of [["CryptoKey", CryptoKey], ["SubtleCrypto", SubtleCrypto]]) {
    define(global, name, { value, writable: true, enumerable: false, configurable: true });
  }
})
