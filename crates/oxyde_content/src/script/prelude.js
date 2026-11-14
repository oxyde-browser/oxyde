// Prélude des pages : API du web écrites en JavaScript (URL Standard, Fetch,
// HTML Standard, DOM Standard). Le script rend une fabrique, appelée avec
// les opérations que seul le moteur sait faire :
//   parseUrl(texte, base)        -> composants de l'adresse, ou null ;
//   changeUrl(href, rang, valeur) -> composants après modification, ou null ;
//   observe, disconnect, takeRecords -> observateurs de mutation ;
//   locationHref()               -> adresse courante du document ;
//   navigate(nature, valeur)     -> demande de navigation à l'embarqueur ;
//   focusedElement()             -> élément qui a le focus, ou null ;
//   randomBytes(n)               -> tableau de n octets aléatoires (source du système).
// Elle rend les fonctions que le moteur appelle en retour (`traverse`, `key`).
// Les composants sont, dans l'ordre : href, protocol, username, password,
// host, hostname, port, pathname, search, hash, origin.
//
// LIMITES CONNUES : `TextDecoder` ne connaît que l'UTF-8 ; `structuredClone`
// ne transfère rien et ignore les types exotiques ; `matchMedia` ne comprend
// que les largeurs, les hauteurs et `prefers-color-scheme`.
(function (parseUrl, changeUrl, observe, disconnect, takeRecords, locationHref, navigate, focusedElement, randomBytes) {
  "use strict";
  const global = globalThis;
  const define = Object.defineProperty;
  const expose = (name, value) => {
    define(global, name, { value, writable: true, enumerable: false, configurable: true });
  };
  const tag = (ctor, name) => {
    define(ctor.prototype, Symbol.toStringTag, { value: name, configurable: true });
  };

  // ---- DOMException --------------------------------------------------------------------
  const exceptionCodes = {
    IndexSizeError: 1, HierarchyRequestError: 3, WrongDocumentError: 4,
    InvalidCharacterError: 5, NoModificationAllowedError: 7, NotFoundError: 8,
    NotSupportedError: 9, InUseAttributeError: 10, InvalidStateError: 11,
    SyntaxError: 12, InvalidModificationError: 13, NamespaceError: 14,
    InvalidAccessError: 15, TypeMismatchError: 17, SecurityError: 18,
    NetworkError: 19, AbortError: 20, URLMismatchError: 21, QuotaExceededError: 22,
    TimeoutError: 23, InvalidNodeTypeError: 24, DataCloneError: 25,
  };
  class DOMException extends Error {
    #name;
    constructor(message = "", name = "Error") {
      super(String(message));
      this.#name = String(name);
    }
    get name() { return this.#name; }
    get code() { return exceptionCodes[this.#name] ?? 0; }
  }
  tag(DOMException, "DOMException");
  expose("DOMException", DOMException);

  // ---- application/x-www-form-urlencoded ------------------------------------------------
  // Décodage tolérant des `%XX` : une séquence invalide reste telle quelle.
  function percentDecode(text) {
    try {
      return decodeURIComponent(text);
    } catch {
      return text.replace(/(?:%[0-9A-Fa-f]{2})+/g, (run) => {
        try { return decodeURIComponent(run); } catch { return run; }
      });
    }
  }
  function parseForm(text) {
    const list = [];
    for (const piece of text.split("&")) {
      if (piece === "") continue;
      const cut = piece.indexOf("=");
      const name = cut < 0 ? piece : piece.slice(0, cut);
      const value = cut < 0 ? "" : piece.slice(cut + 1);
      list.push([
        percentDecode(name.replace(/\+/g, " ")),
        percentDecode(value.replace(/\+/g, " ")),
      ]);
    }
    return list;
  }
  function formEncode(text) {
    return encodeURIComponent(text)
      .replace(/%20/g, "+")
      .replace(/[!'()~]/g, (c) => "%" + c.charCodeAt(0).toString(16).toUpperCase());
  }
  function serializeForm(list) {
    return list.map(([name, value]) => formEncode(name) + "=" + formEncode(value)).join("&");
  }

  // ---- URLSearchParams ---------------------------------------------------------------------
  // Paramètres liés à une URL : fonction qui y reporte chaque modification.
  const owners = new WeakMap();
  class URLSearchParams {
    #list = [];
    constructor(init = "") {
      if (init === null || init === undefined) return;
      if (typeof init === "object" || typeof init === "function") {
        if (typeof init[Symbol.iterator] === "function") {
          for (const pair of init) {
            const items = [...pair];
            if (items.length !== 2) {
              throw new TypeError("URLSearchParams : chaque entrée doit être une paire");
            }
            this.#list.push([String(items[0]), String(items[1])]);
          }
        } else {
          for (const key of Object.keys(init)) this.#list.push([key, String(init[key])]);
        }
      } else {
        const text = String(init);
        this.#list = parseForm(text.startsWith("?") ? text.slice(1) : text);
      }
    }
    #update() {
      const report = owners.get(this);
      if (report) report(serializeForm(this.#list));
    }
    // Remplace le contenu sans prévenir l'URL liée (elle vient de changer).
    static #reset(params, text) { params.#list = parseForm(text); }
    static reset(params, text) { URLSearchParams.#reset(params, text); }
    get size() { return this.#list.length; }
    append(name, value) {
      this.#list.push([String(name), String(value)]);
      this.#update();
    }
    delete(name, value) {
      const wanted = String(name);
      const exact = value === undefined ? undefined : String(value);
      this.#list = this.#list.filter(
        ([n, v]) => !(n === wanted && (exact === undefined || v === exact)),
      );
      this.#update();
    }
    get(name) {
      const wanted = String(name);
      const found = this.#list.find(([n]) => n === wanted);
      return found ? found[1] : null;
    }
    getAll(name) {
      const wanted = String(name);
      return this.#list.filter(([n]) => n === wanted).map(([, v]) => v);
    }
    has(name, value) {
      const wanted = String(name);
      const exact = value === undefined ? undefined : String(value);
      return this.#list.some(([n, v]) => n === wanted && (exact === undefined || v === exact));
    }
    set(name, value) {
      const wanted = String(name);
      const text = String(value);
      const first = this.#list.findIndex(([n]) => n === wanted);
      if (first < 0) {
        this.#list.push([wanted, text]);
      } else {
        this.#list[first][1] = text;
        this.#list = this.#list.filter(([n], index) => n !== wanted || index === first);
      }
      this.#update();
    }
    sort() {
      // Tri stable par nom, en unités de code UTF-16.
      this.#list.sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0));
      this.#update();
    }
    forEach(callback, thisArg) {
      for (const [name, value] of this.#list) callback.call(thisArg, value, name, this);
    }
    *entries() { for (const [name, value] of this.#list) yield [name, value]; }
    *keys() { for (const [name] of this.#list) yield name; }
    *values() { for (const [, value] of this.#list) yield value; }
    [Symbol.iterator]() { return this.entries(); }
    toString() { return serializeForm(this.#list); }
  }
  const resetParams = URLSearchParams.reset;
  delete URLSearchParams.reset;
  tag(URLSearchParams, "URLSearchParams");
  expose("URLSearchParams", URLSearchParams);

  // ---- URL -------------------------------------------------------------------------------------
  const HREF = 0, PROTOCOL = 1, USERNAME = 2, PASSWORD = 3, HOST = 4, HOSTNAME = 5,
    PORT = 6, PATHNAME = 7, SEARCH = 8, HASH = 9, ORIGIN = 10;
  const optional = (base) => (base === undefined ? undefined : String(base));
  class URL {
    #parts;
    #params = null;
    constructor(url, base) {
      if (arguments.length === 0) throw new TypeError("URL : une adresse est requise");
      const parts = parseUrl(String(url), optional(base));
      if (parts === null) throw new TypeError("URL invalide : " + String(url));
      this.#parts = parts;
    }
    static canParse(url, base) {
      return parseUrl(String(url), optional(base)) !== null;
    }
    static parse(url, base) {
      return URL.canParse(url, base) ? new URL(url, base) : null;
    }
    #change(rank, value) {
      const parts = changeUrl(this.#parts[HREF], rank, String(value));
      if (parts === null) return;
      this.#parts = parts;
      if (this.#params !== null) resetParams(this.#params, parts[SEARCH].slice(1));
    }
    get href() { return this.#parts[HREF]; }
    set href(value) {
      const parts = parseUrl(String(value));
      if (parts === null) throw new TypeError("URL invalide : " + String(value));
      this.#parts = parts;
      if (this.#params !== null) resetParams(this.#params, parts[SEARCH].slice(1));
    }
    get origin() { return this.#parts[ORIGIN]; }
    get protocol() { return this.#parts[PROTOCOL]; }
    set protocol(value) { this.#change(PROTOCOL, value); }
    get username() { return this.#parts[USERNAME]; }
    set username(value) { this.#change(USERNAME, value); }
    get password() { return this.#parts[PASSWORD]; }
    set password(value) { this.#change(PASSWORD, value); }
    get host() { return this.#parts[HOST]; }
    set host(value) { this.#change(HOST, value); }
    get hostname() { return this.#parts[HOSTNAME]; }
    set hostname(value) { this.#change(HOSTNAME, value); }
    get port() { return this.#parts[PORT]; }
    set port(value) { this.#change(PORT, value); }
    get pathname() { return this.#parts[PATHNAME]; }
    set pathname(value) { this.#change(PATHNAME, value); }
    get search() { return this.#parts[SEARCH]; }
    set search(value) { this.#change(SEARCH, value); }
    get hash() { return this.#parts[HASH]; }
    set hash(value) { this.#change(HASH, value); }
    get searchParams() {
      if (this.#params === null) {
        this.#params = new URLSearchParams(this.#parts[SEARCH]);
        owners.set(this.#params, (text) => {
          const parts = changeUrl(this.#parts[HREF], SEARCH, text);
          if (parts !== null) this.#parts = parts;
        });
      }
      return this.#params;
    }
    toString() { return this.#parts[HREF]; }
    toJSON() { return this.#parts[HREF]; }
  }
  tag(URL, "URL");
  expose("URL", URL);

  // ---- Headers -----------------------------------------------------------------------------------
  const tokenPattern = /^[!#$%&'*+\-.^_`|~0-9A-Za-z]+$/;
  class Headers {
    #list = [];
    constructor(init) {
      if (init === undefined || init === null) return;
      if (typeof init !== "object" && typeof init !== "function") {
        throw new TypeError("Headers : un objet ou une liste de paires est attendu");
      }
      if (typeof init[Symbol.iterator] === "function") {
        for (const pair of init) {
          const items = [...pair];
          if (items.length !== 2) throw new TypeError("Headers : chaque entrée doit être une paire");
          this.append(items[0], items[1]);
        }
      } else {
        for (const key of Object.keys(init)) this.append(key, init[key]);
      }
    }
    static #name(name) {
      const text = String(name);
      if (!tokenPattern.test(text)) throw new TypeError("nom d'en-tête invalide : « " + text + " »");
      return text.toLowerCase();
    }
    static #value(value) {
      const text = String(value).replace(/^[\t\n\r ]+|[\t\n\r ]+$/g, "");
      if (/[\0\r\n]/.test(text)) throw new TypeError("valeur d'en-tête invalide");
      return text;
    }
    append(name, value) {
      this.#list.push([Headers.#name(name), Headers.#value(value)]);
    }
    delete(name) {
      const wanted = Headers.#name(name);
      this.#list = this.#list.filter(([n]) => n !== wanted);
    }
    get(name) {
      const wanted = Headers.#name(name);
      const values = this.#list.filter(([n]) => n === wanted).map(([, v]) => v);
      return values.length > 0 ? values.join(", ") : null;
    }
    getSetCookie() {
      return this.#list.filter(([n]) => n === "set-cookie").map(([, v]) => v);
    }
    has(name) {
      const wanted = Headers.#name(name);
      return this.#list.some(([n]) => n === wanted);
    }
    set(name, value) {
      const wanted = Headers.#name(name);
      const text = Headers.#value(value);
      this.#list = this.#list.filter(([n]) => n !== wanted);
      this.#list.push([wanted, text]);
    }
    // Triés par nom, les valeurs d'un même nom réunies (sauf `set-cookie`).
    #sorted() {
      const names = [...new Set(this.#list.map(([n]) => n))].sort();
      const out = [];
      for (const name of names) {
        if (name === "set-cookie") {
          for (const value of this.getSetCookie()) out.push([name, value]);
        } else {
          out.push([name, this.get(name)]);
        }
      }
      return out;
    }
    forEach(callback, thisArg) {
      for (const [name, value] of this.#sorted()) callback.call(thisArg, value, name, this);
    }
    *entries() { yield* this.#sorted(); }
    *keys() { for (const [name] of this.#sorted()) yield name; }
    *values() { for (const [, value] of this.#sorted()) yield value; }
    [Symbol.iterator]() { return this.entries(); }
  }
  tag(Headers, "Headers");
  expose("Headers", Headers);

  // ---- MutationObserver (DOM Standard §4.3) -------------------------------------------------------
  class MutationObserver {
    #callback;
    constructor(callback) {
      if (typeof callback !== "function") {
        throw new TypeError("MutationObserver : une fonction de rappel est attendue");
      }
      this.#callback = callback;
    }
    observe(target, options = {}) {
      observe(this, this.#callback, target, options);
    }
    disconnect() {
      disconnect(this);
    }
    takeRecords() {
      return takeRecords(this);
    }
  }
  tag(MutationObserver, "MutationObserver");
  expose("MutationObserver", MutationObserver);

  // ---- Microtâches, erreurs, temps libre ------------------------------------------------------------
  const resolved = Promise.resolve();
  const then = Promise.prototype.then;
  const consoleError = console.error.bind(console);
  function reportError(error) {
    consoleError("Exception non rattrapée : " + String(error));
  }
  expose("reportError", reportError);
  expose("queueMicrotask", function queueMicrotask(callback) {
    if (typeof callback !== "function") {
      throw new TypeError("queueMicrotask : une fonction est attendue");
    }
    then.call(resolved, () => {
      try {
        callback();
      } catch (error) {
        reportError(error);
      }
    });
  });
  const startTimer = setTimeout;
  const stopTimer = clearTimeout;
  expose("requestIdleCallback", function requestIdleCallback(callback) {
    return startTimer(() => callback({ didTimeout: false, timeRemaining: () => 50 }), 1);
  });
  expose("cancelIdleCallback", function cancelIdleCallback(handle) {
    stopTimer(handle);
  });

  // ---- Base64 (HTML Standard §8.3) -----------------------------------------------------------------------
  const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
  function invalidCharacter(what) {
    return new DOMException(what, "InvalidCharacterError");
  }
  expose("btoa", function btoa(data) {
    const text = String(data);
    let out = "";
    for (let i = 0; i < text.length; i += 3) {
      const codes = [text.charCodeAt(i), text.charCodeAt(i + 1), text.charCodeAt(i + 2)];
      for (const code of codes) {
        if (code > 255) throw invalidCharacter("btoa : caractère hors de la plage Latin-1");
      }
      const [a, b, c] = codes;
      out += alphabet[a >> 2];
      out += alphabet[((a & 3) << 4) | (b >> 4 || 0)];
      out += i + 1 < text.length ? alphabet[((b & 15) << 2) | (c >> 6 || 0)] : "=";
      out += i + 2 < text.length ? alphabet[c & 63] : "=";
    }
    return out;
  });
  expose("atob", function atob(data) {
    let text = String(data).replace(/[\t\n\f\r ]/g, "");
    if (text.length % 4 === 0) text = text.replace(/={1,2}$/, "");
    if (text.length % 4 === 1 || /[^A-Za-z0-9+/]/.test(text)) {
      throw invalidCharacter("atob : la chaîne n'est pas du base64 valide");
    }
    let out = "";
    let buffer = 0;
    let bits = 0;
    for (const char of text) {
      buffer = (buffer << 6) | alphabet.indexOf(char);
      bits += 6;
      if (bits >= 8) {
        bits -= 8;
        out += String.fromCharCode((buffer >> bits) & 255);
      }
    }
    return out;
  });

  // ---- TextEncoder et TextDecoder (Encoding Standard, UTF-8 seulement) -----------------------------------------
  function utf8Bytes(text) {
    const bytes = [];
    for (const char of text) {
      const code = char.codePointAt(0);
      if (code < 0x80) {
        bytes.push(code);
      } else if (code < 0x800) {
        bytes.push(0xc0 | (code >> 6), 0x80 | (code & 63));
      } else if (code < 0x10000) {
        bytes.push(0xe0 | (code >> 12), 0x80 | ((code >> 6) & 63), 0x80 | (code & 63));
      } else {
        bytes.push(
          0xf0 | (code >> 18),
          0x80 | ((code >> 12) & 63),
          0x80 | ((code >> 6) & 63),
          0x80 | (code & 63),
        );
      }
    }
    return bytes;
  }
  class TextEncoder {
    get encoding() { return "utf-8"; }
    encode(input = "") { return new Uint8Array(utf8Bytes(String(input))); }
    encodeInto(source, destination) {
      let read = 0;
      let written = 0;
      for (const char of String(source)) {
        const bytes = utf8Bytes(char);
        if (written + bytes.length > destination.length) break;
        for (const byte of bytes) destination[written++] = byte;
        read += char.length;
      }
      return { read, written };
    }
  }
  tag(TextEncoder, "TextEncoder");
  expose("TextEncoder", TextEncoder);

  function bytesOf(input) {
    if (input === undefined) return new Uint8Array(0);
    if (input instanceof ArrayBuffer) return new Uint8Array(input);
    if (ArrayBuffer.isView(input)) {
      return new Uint8Array(input.buffer, input.byteOffset, input.byteLength);
    }
    throw new TypeError("TextDecoder.decode : un tampon d'octets est attendu");
  }
  class TextDecoder {
    #fatal;
    #ignoreBOM;
    constructor(label = "utf-8", options = {}) {
      const name = String(label).trim().toLowerCase();
      if (!["utf-8", "utf8", "unicode-1-1-utf-8"].includes(name)) {
        throw new RangeError("TextDecoder : encodage non pris en charge : " + name);
      }
      this.#fatal = Boolean(options && options.fatal);
      this.#ignoreBOM = Boolean(options && options.ignoreBOM);
    }
    get encoding() { return "utf-8"; }
    get fatal() { return this.#fatal; }
    get ignoreBOM() { return this.#ignoreBOM; }
    decode(input) {
      const bytes = bytesOf(input);
      let out = "";
      let i = 0;
      if (!this.#ignoreBOM && bytes[0] === 0xef && bytes[1] === 0xbb && bytes[2] === 0xbf) i = 3;
      const invalid = () => {
        if (this.#fatal) throw new TypeError("TextDecoder : la suite d'octets n'est pas de l'UTF-8");
        out += "�";
      };
      while (i < bytes.length) {
        const first = bytes[i++];
        if (first < 0x80) {
          out += String.fromCharCode(first);
          continue;
        }
        // Nombre d'octets de suite, et bornes du premier d'entre eux.
        let needed = 0;
        let lower = 0x80;
        let upper = 0xbf;
        let code = 0;
        if (first >= 0xc2 && first <= 0xdf) {
          needed = 1;
          code = first & 0x1f;
        } else if (first >= 0xe0 && first <= 0xef) {
          needed = 2;
          code = first & 0x0f;
          if (first === 0xe0) lower = 0xa0;
          if (first === 0xed) upper = 0x9f;
        } else if (first >= 0xf0 && first <= 0xf4) {
          needed = 3;
          code = first & 0x07;
          if (first === 0xf0) lower = 0x90;
          if (first === 0xf4) upper = 0x8f;
        } else {
          invalid();
          continue;
        }
        let complete = true;
        for (let k = 0; k < needed; k++) {
          const next = bytes[i];
          if (next === undefined || next < lower || next > upper) {
            complete = false;
            break;
          }
          code = (code << 6) | (next & 0x3f);
          lower = 0x80;
          upper = 0xbf;
          i++;
        }
        if (complete) out += String.fromCodePoint(code);
        else invalid();
      }
      return out;
    }
  }
  tag(TextDecoder, "TextDecoder");
  expose("TextDecoder", TextDecoder);

  // ---- AbortController et AbortSignal (DOM Standard §3) -----------------------------------------------------------
  const EventTargetBase = EventTarget;
  const EventCtor = Event;
  const signals = new WeakMap();
  const signalKey = {};
  class AbortSignal extends EventTargetBase {
    constructor(key) {
      if (key !== signalKey) throw new TypeError("AbortSignal : constructeur illégal");
      super();
      signals.set(this, { aborted: false, reason: undefined });
    }
    get aborted() { return signals.get(this).aborted; }
    get reason() { return signals.get(this).reason; }
    throwIfAborted() {
      const state = signals.get(this);
      if (state.aborted) throw state.reason;
    }
    static abort(reason) {
      const signal = new AbortSignal(signalKey);
      abortSignal(signal, reason);
      return signal;
    }
    static timeout(milliseconds) {
      const signal = new AbortSignal(signalKey);
      startTimer(() => {
        abortSignal(signal, new DOMException("le délai est écoulé", "TimeoutError"));
      }, milliseconds);
      return signal;
    }
    static any(list) {
      const signal = new AbortSignal(signalKey);
      for (const source of list) {
        if (source.aborted) {
          abortSignal(signal, source.reason);
          break;
        }
        source.addEventListener("abort", () => abortSignal(signal, source.reason), { once: true });
      }
      return signal;
    }
  }
  function abortSignal(signal, reason) {
    const state = signals.get(signal);
    if (state.aborted) return;
    state.aborted = true;
    state.reason = reason === undefined
      ? new DOMException("l'opération a été abandonnée", "AbortError")
      : reason;
    signal.dispatchEvent(new EventCtor("abort"));
  }
  class AbortController {
    #signal = new AbortSignal(signalKey);
    get signal() { return this.#signal; }
    abort(reason) { abortSignal(this.#signal, reason); }
  }
  tag(AbortSignal, "AbortSignal");
  tag(AbortController, "AbortController");
  expose("AbortSignal", AbortSignal);
  expose("AbortController", AbortController);

  // ---- structuredClone (HTML Standard §2.7) ---------------------------------------------------------------------------
  function cloneValue(value, seen) {
    if (value === null || (typeof value !== "object" && typeof value !== "function")) {
      if (typeof value === "symbol") {
        throw new DOMException("un symbole ne peut pas être cloné", "DataCloneError");
      }
      return value;
    }
    if (typeof value === "function") {
      throw new DOMException("une fonction ne peut pas être clonée", "DataCloneError");
    }
    if (seen.has(value)) return seen.get(value);
    let copy;
    if (value instanceof Date) {
      copy = new Date(value.getTime());
      seen.set(value, copy);
    } else if (value instanceof RegExp) {
      copy = new RegExp(value.source, value.flags);
      seen.set(value, copy);
    } else if (value instanceof Map) {
      copy = new Map();
      seen.set(value, copy);
      for (const [key, item] of value) copy.set(cloneValue(key, seen), cloneValue(item, seen));
    } else if (value instanceof Set) {
      copy = new Set();
      seen.set(value, copy);
      for (const item of value) copy.add(cloneValue(item, seen));
    } else if (value instanceof ArrayBuffer) {
      copy = value.slice(0);
      seen.set(value, copy);
    } else if (ArrayBuffer.isView(value)) {
      copy = value.slice ? value.slice() : new DataView(value.buffer.slice(0));
      seen.set(value, copy);
    } else if (value instanceof Error) {
      copy = new value.constructor(value.message);
      seen.set(value, copy);
    } else if (Array.isArray(value)) {
      copy = [];
      seen.set(value, copy);
      for (let i = 0; i < value.length; i++) {
        if (i in value) copy[i] = cloneValue(value[i], seen);
      }
      copy.length = value.length;
    } else if (typeof value.nodeType === "number") {
      throw new DOMException("un nœud du DOM ne peut pas être cloné", "DataCloneError");
    } else {
      copy = {};
      seen.set(value, copy);
      for (const key of Object.keys(value)) copy[key] = cloneValue(value[key], seen);
    }
    return copy;
  }
  expose("structuredClone", function structuredClone(value) {
    return cloneValue(value, new Map());
  });

  // ---- Événements d'interface (UI Events) ---------------------------------------------------------------------------------
  // Fabrique une interface d'événement : chaque membre de `defaults` devient
  // un attribut en lecture seule, initialisé depuis le dictionnaire reçu et
  // converti au type de sa valeur par défaut.
  function eventInterface(name, Base, defaults) {
    const data = new WeakMap();
    const keys = Object.keys(defaults);
    const Interface = {
      [name]: class extends Base {
        constructor(type, init = {}) {
          super(type, init);
          const values = {};
          for (const key of keys) {
            const given = init === null || init === undefined ? undefined : init[key];
            const fallback = defaults[key];
            if (given === undefined) values[key] = fallback;
            else if (typeof fallback === "boolean") values[key] = Boolean(given);
            else if (typeof fallback === "number") values[key] = Number(given);
            else if (typeof fallback === "string") values[key] = String(given);
            else values[key] = given;
          }
          data.set(this, values);
        }
      },
    }[name];
    for (const key of keys) {
      define(Interface.prototype, key, {
        get() { return data.get(this)[key]; },
        enumerable: true,
        configurable: true,
      });
    }
    tag(Interface, name);
    expose(name, Interface);
    return Interface;
  }
  const modifiers = { ctrlKey: false, shiftKey: false, altKey: false, metaKey: false };
  const UIEvent = eventInterface("UIEvent", EventCtor, { view: null, detail: 0 });
  const MouseEvent = eventInterface("MouseEvent", UIEvent, {
    screenX: 0, screenY: 0, clientX: 0, clientY: 0, ...modifiers,
    button: 0, buttons: 0, relatedTarget: null, movementX: 0, movementY: 0,
  });
  for (const [alias, source] of [["pageX", "clientX"], ["pageY", "clientY"], ["x", "clientX"],
    ["y", "clientY"], ["offsetX", "clientX"], ["offsetY", "clientY"]]) {
    define(MouseEvent.prototype, alias, {
      get() { return this[source]; },
      enumerable: true,
      configurable: true,
    });
  }
  eventInterface("PointerEvent", MouseEvent, {
    pointerId: 0, width: 1, height: 1, pressure: 0, pointerType: "", isPrimary: false,
  });
  eventInterface("WheelEvent", MouseEvent, { deltaX: 0, deltaY: 0, deltaZ: 0, deltaMode: 0 });
  eventInterface("KeyboardEvent", UIEvent, {
    key: "", code: "", location: 0, repeat: false, isComposing: false, ...modifiers,
    keyCode: 0, charCode: 0, which: 0,
  });
  eventInterface("InputEvent", UIEvent, { data: null, inputType: "", isComposing: false });
  eventInterface("FocusEvent", UIEvent, { relatedTarget: null });
  eventInterface("ProgressEvent", EventCtor, { lengthComputable: false, loaded: 0, total: 0 });
  eventInterface("ErrorEvent", EventCtor, {
    message: "", filename: "", lineno: 0, colno: 0, error: undefined,
  });
  eventInterface("PopStateEvent", EventCtor, { state: null });
  eventInterface("SubmitEvent", EventCtor, { submitter: null });
  eventInterface("HashChangeEvent", EventCtor, { oldURL: "", newURL: "" });

  // `document.createEvent(interface)` : un événement à initialiser ensuite.
  const legacyEvents = {
    event: "Event", events: "Event", htmlevents: "Event", customevent: "CustomEvent",
    uievent: "UIEvent", uievents: "UIEvent", mouseevent: "MouseEvent",
    mouseevents: "MouseEvent", keyboardevent: "KeyboardEvent", focusevent: "FocusEvent",
  };
  define(Document.prototype, "createEvent", {
    value: function createEvent(kind) {
      const name = legacyEvents[String(kind).toLowerCase()];
      if (name === undefined) {
        throw new DOMException("interface d'événement inconnue : " + String(kind), "NotSupportedError");
      }
      return new global[name]("");
    },
    writable: true,
    configurable: true,
  });

  // ---- Éléments créés par constructeur, sérialisation -------------------------------------------------------------------------
  expose("Image", function Image(width, height) {
    if (!new.target) throw new TypeError("Image doit être appelé avec « new »");
    const image = document.createElement("img");
    if (width !== undefined) image.setAttribute("width", String(width));
    if (height !== undefined) image.setAttribute("height", String(height));
    return image;
  });
  expose("Option", function Option(text, value) {
    if (!new.target) throw new TypeError("Option doit être appelé avec « new »");
    const option = document.createElement("option");
    if (text !== undefined) option.textContent = String(text);
    if (value !== undefined) option.setAttribute("value", String(value));
    return option;
  });
  class XMLSerializer {
    serializeToString(node) {
      if (node === null || typeof node !== "object" || typeof node.nodeType !== "number") {
        throw new TypeError("XMLSerializer.serializeToString : un nœud est attendu");
      }
      const one = (item) => {
        switch (item.nodeType) {
          case 1: return item.outerHTML;
          case 3: return item.data.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
          case 8: return "<!--" + item.data + "-->";
          case 10: return "<!DOCTYPE " + item.name + ">";
          default: return Array.from(item.childNodes, one).join("");
        }
      };
      return one(node);
    }
  }
  tag(XMLSerializer, "XMLSerializer");
  expose("XMLSerializer", XMLSerializer);

  // ---- Fenêtre, écran, navigateur --------------------------------------------------------------------------------------------------
  const lazy = (target, name, get) => define(target, name, { get, enumerable: true, configurable: true });
  const screen = {};
  for (const [name, source] of [["width", "innerWidth"], ["height", "innerHeight"],
    ["availWidth", "innerWidth"], ["availHeight", "innerHeight"]]) {
    lazy(screen, name, () => global[source]);
  }
  screen.colorDepth = 24;
  screen.pixelDepth = 24;
  expose("screen", screen);
  expose("devicePixelRatio", 1);
  Object.assign(navigator, {
    onLine: true, platform: "Win32", vendor: "", cookieEnabled: false,
    hardwareConcurrency: 1, maxTouchPoints: 0,
    sendBeacon(url, data) {
      try {
        fetch(url, data === undefined || data === null ? { method: "POST" } : { method: "POST", body: data });
        return true;
      } catch {
        return false;
      }
    },
  });
  if (typeof console.table !== "function") console.table = console.log;

  // ---- matchMedia (CSSOM View) ----------------------------------------------------------------------------------------------------------
  function featureMatches(feature) {
    const match = /^\(\s*([a-z-]+)\s*(?::\s*([^)]+?)\s*)?\)$/.exec(feature);
    if (match === null) return false;
    const [, name, value] = match;
    const pixels = value === undefined ? NaN : parseFloat(value) * (/(?:em|rem)$/.test(value) ? 16 : 1);
    switch (name) {
      case "min-width": return global.innerWidth >= pixels;
      case "max-width": return global.innerWidth <= pixels;
      case "min-height": return global.innerHeight >= pixels;
      case "max-height": return global.innerHeight <= pixels;
      case "width": return global.innerWidth === pixels;
      case "height": return global.innerHeight === pixels;
      case "orientation":
        return (global.innerWidth >= global.innerHeight ? "landscape" : "portrait") === value;
      case "prefers-color-scheme": return value === "light";
      case "prefers-reduced-motion": return value === "no-preference";
      case "hover": return value === "hover";
      case "pointer": return value === "fine";
      default: return false;
    }
  }
  function queryMatches(query) {
    return query.split(",").some((alternative) => {
      let text = alternative.trim().toLowerCase();
      let negate = false;
      if (text.startsWith("not ")) {
        negate = true;
        text = text.slice(4).trim();
      } else if (text.startsWith("only ")) {
        text = text.slice(5).trim();
      }
      const result = text.split(/\s+and\s+/).every((part) => {
        if (part === "all" || part === "screen" || part === "") return true;
        if (part === "print" || part === "speech") return false;
        return featureMatches(part);
      });
      return negate ? !result : result;
    });
  }
  class MediaQueryList extends EventTargetBase {
    #media;
    constructor(key, media) {
      if (key !== signalKey) throw new TypeError("MediaQueryList : constructeur illégal");
      super();
      this.#media = media;
    }
    get media() { return this.#media; }
    get matches() { return queryMatches(this.#media); }
    addListener(callback) { this.addEventListener("change", callback); }
    removeListener(callback) { this.removeEventListener("change", callback); }
  }
  tag(MediaQueryList, "MediaQueryList");
  expose("MediaQueryList", MediaQueryList);
  expose("matchMedia", function matchMedia(query) {
    return new MediaQueryList(signalKey, String(query));
  });

  // ---- performance (User Timing, simplifié) ------------------------------------------------------------------------------------------------
  const entries = [];
  const timeOrigin = Date.now() - performance.now();
  Object.assign(performance, {
    timeOrigin,
    timing: { navigationStart: Math.round(timeOrigin) },
    mark(name) {
      const entry = { name: String(name), entryType: "mark", startTime: performance.now(), duration: 0 };
      entries.push(entry);
      return entry;
    },
    measure(name, start, end) {
      const at = (mark, fallback) => {
        if (mark === undefined) return fallback;
        const found = entries.findLast((e) => e.entryType === "mark" && e.name === String(mark));
        if (found === undefined) {
          throw new DOMException("marque inconnue : " + String(mark), "SyntaxError");
        }
        return found.startTime;
      };
      const startTime = at(start, 0);
      const entry = {
        name: String(name), entryType: "measure", startTime,
        duration: at(end, performance.now()) - startTime,
      };
      entries.push(entry);
      return entry;
    },
    getEntries() { return entries.slice(); },
    getEntriesByType(type) { return entries.filter((e) => e.entryType === type); },
    getEntriesByName(name, type) {
      return entries.filter((e) => e.name === name && (type === undefined || e.entryType === type));
    },
    clearMarks(name) {
      for (let i = entries.length - 1; i >= 0; i--) {
        if (entries[i].entryType === "mark" && (name === undefined || entries[i].name === name)) {
          entries.splice(i, 1);
        }
      }
    },
    clearMeasures(name) {
      for (let i = entries.length - 1; i >= 0; i--) {
        if (entries[i].entryType === "measure" && (name === undefined || entries[i].name === name)) {
          entries.splice(i, 1);
        }
      }
    },
  });

  // ---- location (HTML Standard §7.2.4) ----------------------------------------------------------------------------------------------------
  const here = () => parseUrl(locationHref());
  const fireLater = (event) => startTimer(() => global.dispatchEvent(event), 0);
  // Changement de fragment : même document, événement `hashchange`.
  function setHash(value) {
    const before = locationHref();
    const parts = changeUrl(before, HASH, String(value));
    if (parts === null || parts[HREF] === before) return;
    sessionEntries.length = position + 1;
    sessionEntries.push({ state: null, url: parts[HREF] });
    position++;
    navigate("pushState", parts[HREF]);
    fireLater(new global.HashChangeEvent("hashchange", { oldURL: before, newURL: parts[HREF] }));
  }
  const locationObject = {
    assign(url) { navigate("assign", String(url)); },
    replace(url) { navigate("replace", String(url)); },
    reload() { navigate("reload"); },
    toString() { return locationHref(); },
  };
  for (const [name, rank] of [["href", HREF], ["protocol", PROTOCOL], ["host", HOST],
    ["hostname", HOSTNAME], ["port", PORT], ["pathname", PATHNAME], ["search", SEARCH],
    ["hash", HASH], ["origin", ORIGIN]]) {
    define(locationObject, name, {
      get() { return here()[rank]; },
      set(value) {
        if (rank === ORIGIN) return;
        if (rank === HASH) {
          setHash(value);
        } else if (rank === HREF) {
          navigate("assign", String(value));
        } else {
          const parts = changeUrl(locationHref(), rank, String(value));
          if (parts !== null) navigate("assign", parts[HREF]);
        }
      },
      enumerable: true,
      configurable: false,
    });
  }
  expose("location", locationObject);
  define(Document.prototype, "location", {
    get() { return locationObject; },
    set(value) { navigate("assign", String(value)); },
    enumerable: true,
    configurable: true,
  });

  // ---- history (HTML Standard §7.2.5) -------------------------------------------------------------------------------------------------------
  // Entrées de ce document : celle du chargement, puis celles de `pushState`.
  const sessionEntries = [{ state: null, url: locationHref() }];
  let position = 0;
  function changeState(state, url, replace) {
    const current = locationHref();
    let next = current;
    if (url !== undefined && url !== null) {
      const parts = parseUrl(String(url), current);
      if (parts === null || parts[ORIGIN] !== here()[ORIGIN]) {
        throw new DOMException(
          "history : l'adresse doit rester dans l'origine du document",
          "SecurityError",
        );
      }
      next = parts[HREF];
    }
    const entry = { state: cloneValue(state, new Map()), url: next };
    if (replace) {
      sessionEntries[position] = entry;
    } else {
      sessionEntries.length = position + 1;
      sessionEntries.push(entry);
      position++;
    }
    navigate(replace ? "replaceState" : "pushState", next);
  }
  const historyObject = {
    get length() { return sessionEntries.length; },
    get state() { return sessionEntries[position].state; },
    scrollRestoration: "auto",
    pushState(state, unused, url) { changeState(state, url, false); },
    replaceState(state, unused, url) { changeState(state, url, true); },
    back() { navigate("traverse", -1); },
    forward() { navigate("traverse", 1); },
    go(delta = 0) {
      const steps = Math.trunc(Number(delta)) || 0;
      if (steps === 0) navigate("reload");
      else navigate("traverse", steps);
    },
  };
  expose("history", historyObject);

  // L'élément qui a le focus ; à défaut, `<body>`.
  define(Document.prototype, "activeElement", {
    get() { return focusedElement() ?? this.body ?? this.documentElement; },
    enumerable: true,
    configurable: true,
  });
  // Codes hérités (`keyCode`, `which`) des touches courantes.
  const legacyCodes = {
    Backspace: 8, Tab: 9, Enter: 13, Shift: 16, Control: 17, Alt: 18, Escape: 27, " ": 32,
    PageUp: 33, PageDown: 34, End: 35, Home: 36, ArrowLeft: 37, ArrowUp: 38,
    ArrowRight: 39, ArrowDown: 40, Delete: 46,
  };
  function legacyCode(key) {
    if (key in legacyCodes) return legacyCodes[key];
    if (key.length === 1) return key.toUpperCase().charCodeAt(0);
    const functionKey = /^F(\d{1,2})$/.exec(key);
    return functionKey ? 111 + Number(functionKey[1]) : 0;
  }

  // ---- Fenêtre : ce que les pages lisent sans vérifier -----------------------------------------------------------
  // Gestionnaires d'événements de la fenêtre : `null` tant qu'aucun n'est posé.
  for (const kind of ["load", "error", "beforeunload", "unload", "resize", "scroll", "message",
    "popstate", "hashchange", "focus", "blur", "pageshow", "pagehide", "storage",
    "unhandledrejection", "rejectionhandled", "online", "offline", "languagechange",
    "click", "keydown", "keyup", "keypress", "mousedown", "mouseup", "mousemove",
    "DOMContentLoaded", "visibilitychange", "abort", "orientationchange"]) {
    const name = "on" + kind;
    if (!(name in global)) {
      define(global, name, { value: null, writable: true, enumerable: true, configurable: true });
    }
  }
  const fenetre = (name, value) => {
    if (!(name in global)) define(global, name, { value, writable: true, enumerable: true, configurable: true });
  };
  // `frames` est la fenêtre elle-même ; `length` compte ses cadres.
  fenetre("frames", global);
  if (!("length" in global)) {
    lazy(global, "length", () => document.querySelectorAll("iframe, frame").length);
  }
  fenetre("name", "");
  fenetre("status", "");
  fenetre("opener", null);
  fenetre("closed", false);
  fenetre("crossOriginIsolated", false);
  fenetre("clientInformation", global.navigator);
  for (const name of ["screenX", "screenY", "screenLeft", "screenTop"]) fenetre(name, 0);
  for (const name of ["locationbar", "menubar", "personalbar", "scrollbars", "statusbar", "toolbar"]) {
    fenetre(name, { visible: true });
  }
  if (!("origin" in global)) lazy(global, "origin", () => global.location.origin);
  if (!("isSecureContext" in global)) {
    lazy(global, "isSecureContext", () => global.location.protocol === "https:");
  }
  if (!("outerWidth" in global)) lazy(global, "outerWidth", () => global.innerWidth);
  if (!("outerHeight" in global)) lazy(global, "outerHeight", () => global.innerHeight);
  // LIMITE : pas de fenêtre surgissante ni d'impression ; `open` rend `null`,
  // comme un navigateur qui bloque la fenêtre.
  fenetre("open", function open() { return null; });
  for (const name of ["close", "print", "stop", "focus", "blur"]) {
    fenetre(name, { [name]() {} }[name]);
  }
  if (!("visualViewport" in global)) {
    const viewport = new EventTargetBase();
    lazy(viewport, "width", () => global.innerWidth);
    lazy(viewport, "height", () => global.innerHeight);
    lazy(viewport, "pageLeft", () => global.scrollX ?? 0);
    lazy(viewport, "pageTop", () => global.scrollY ?? 0);
    Object.assign(viewport, { scale: 1, offsetLeft: 0, offsetTop: 0 });
    fenetre("visualViewport", viewport);
  }

  // ---- Messages ------------------------------------------------------------------------------------------------------
  eventInterface("MessageEvent", EventCtor, {
    data: null, origin: "", lastEventId: "", source: null, ports: null,
  });
  eventInterface("PromiseRejectionEvent", EventCtor, { promise: null, reason: undefined });
  eventInterface("StorageEvent", EventCtor, {
    key: null, oldValue: null, newValue: null, url: "", storageArea: null,
  });
  eventInterface("ClipboardEvent", EventCtor, { clipboardData: null });
  eventInterface("CloseEvent", EventCtor, { wasClean: false, code: 0, reason: "" });
  eventInterface("PageTransitionEvent", EventCtor, { persisted: false });
  const later = (task) => startTimer(task, 0);
  // `postMessage` vers la fenêtre elle-même : le message arrive dans une
  // tâche ultérieure. LIMITE : rien n'est transféré ; une origine cible autre
  // que celle de la page ou `*` fait ignorer le message.
  fenetre("postMessage", function postMessage(message, target = "*") {
    const wanted = typeof target === "object" && target !== null ? target.targetOrigin ?? "*" : target;
    const data = global.structuredClone(message);
    const origin = global.location.origin;
    if (wanted !== "*" && wanted !== "/" && wanted !== origin) return;
    later(() => global.dispatchEvent(new global.MessageEvent("message", { data, origin, source: global })));
  });
  class MessagePort extends EventTargetBase {
    #other = null;
    #started = false;
    #closed = false;
    #queue = [];
    #handler = null;
    static pair(a, b) {
      a.#other = b;
      b.#other = a;
    }
    postMessage(message) {
      const other = this.#other;
      if (this.#closed || other === null || other.#closed) return;
      other.#receive(global.structuredClone(message));
    }
    #receive(data) {
      if (!this.#started) {
        this.#queue.push(data);
        return;
      }
      later(() => {
        if (!this.#closed) this.dispatchEvent(new global.MessageEvent("message", { data }));
      });
    }
    start() {
      if (this.#started) return;
      this.#started = true;
      for (const data of this.#queue.splice(0)) this.#receive(data);
    }
    close() {
      this.#closed = true;
    }
    // Poser `onmessage` ouvre le port, comme `start()`.
    get onmessage() { return this.#handler; }
    set onmessage(handler) {
      this.#handler = typeof handler === "function" ? handler : null;
      this.start();
    }
  }
  tag(MessagePort, "MessagePort");
  expose("MessagePort", MessagePort);
  class MessageChannel {
    constructor() {
      this.port1 = new MessagePort();
      this.port2 = new MessagePort();
      MessagePort.pair(this.port1, this.port2);
    }
  }
  tag(MessageChannel, "MessageChannel");
  expose("MessageChannel", MessageChannel);
  // LIMITE : un canal ne relie que les objets de cette page ; les autres
  // onglets de la même origine ne le reçoivent pas.
  const channels = new Map();
  class BroadcastChannel extends EventTargetBase {
    #name;
    #closed = false;
    constructor(name) {
      super();
      this.#name = String(name);
      if (!channels.has(this.#name)) channels.set(this.#name, new Set());
      channels.get(this.#name).add(this);
      this.onmessage = null;
    }
    get name() { return this.#name; }
    postMessage(message) {
      if (this.#closed) throw new DOMException("le canal est fermé", "InvalidStateError");
      const data = global.structuredClone(message);
      for (const other of channels.get(this.#name) ?? []) {
        if (other === this) continue;
        later(() => {
          if (!other.#closed) {
            other.dispatchEvent(new global.MessageEvent("message", { data, origin: global.location.origin }));
          }
        });
      }
    }
    close() {
      this.#closed = true;
      channels.get(this.#name)?.delete(this);
    }
  }
  tag(BroadcastChannel, "BroadcastChannel");
  expose("BroadcastChannel", BroadcastChannel);

  // ---- crypto --------------------------------------------------------------------------------------------------------
  // LIMITE : pas de `crypto.subtle`.
  const integerArrays = [Int8Array, Uint8Array, Uint8ClampedArray, Int16Array, Uint16Array,
    Int32Array, Uint32Array, BigInt64Array, BigUint64Array];
  const crypto = {
    getRandomValues(array) {
      if (!integerArrays.some((type) => array instanceof type)) {
        throw new DOMException("un tableau typé d'entiers est attendu", "TypeMismatchError");
      }
      if (array.byteLength > 65536) {
        throw new DOMException("au plus 65536 octets par appel", "QuotaExceededError");
      }
      const bytes = new Uint8Array(array.buffer, array.byteOffset, array.byteLength);
      const random = randomBytes(bytes.length);
      for (let i = 0; i < bytes.length; i++) bytes[i] = random[i];
      return array;
    },
    randomUUID() {
      const bytes = randomBytes(16);
      bytes[6] = (bytes[6] & 0x0f) | 0x40;
      bytes[8] = (bytes[8] & 0x3f) | 0x80;
      const hex = Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
      return [hex.slice(0, 8), hex.slice(8, 12), hex.slice(12, 16), hex.slice(16, 20), hex.slice(20)].join("-");
    },
  };
  define(crypto, Symbol.toStringTag, { value: "Crypto", configurable: true });
  expose("crypto", crypto);

  // ---- Géométrie ------------------------------------------------------------------------------------------------------
  class DOMRectReadOnly {
    #x; #y; #width; #height;
    constructor(x = 0, y = 0, width = 0, height = 0) {
      this.#x = Number(x);
      this.#y = Number(y);
      this.#width = Number(width);
      this.#height = Number(height);
    }
    static fromRect(rect = {}) {
      return new this(rect.x, rect.y, rect.width, rect.height);
    }
    get x() { return this.#x; }
    get y() { return this.#y; }
    get width() { return this.#width; }
    get height() { return this.#height; }
    get left() { return Math.min(this.#x, this.#x + this.#width); }
    get right() { return Math.max(this.#x, this.#x + this.#width); }
    get top() { return Math.min(this.#y, this.#y + this.#height); }
    get bottom() { return Math.max(this.#y, this.#y + this.#height); }
    toJSON() {
      const { x, y, width, height, top, right, bottom, left } = this;
      return { x, y, width, height, top, right, bottom, left };
    }
  }
  tag(DOMRectReadOnly, "DOMRectReadOnly");
  expose("DOMRectReadOnly", DOMRectReadOnly);
  class DOMRect extends DOMRectReadOnly {}
  tag(DOMRect, "DOMRect");
  expose("DOMRect", DOMRect);

  // ---- Observateurs de géométrie ------------------------------------------------------------------------------------
  // `IntersectionObserver` et `ResizeObserver` relisent la géométrie de leurs
  // cibles à intervalle régulier tant qu'ils en observent.
  //
  // LIMITES CONNUES : la géométrie est relue toutes les 100 ms, pas à chaque
  // image ; l'intersection est calculée avec la zone d'affichage ou avec la
  // boîte de `root`, sans le rognage des ancêtres défilants ; `rootMargin`
  // ne comprend que les pixels et les pourcentages.
  const POLL = 100;
  const watching = new Set();
  let polling = false;
  function poll() {
    polling = false;
    for (const observer of Array.from(watching)) {
      try {
        observer[CHECK]();
      } catch (error) {
        reportError(error);
      }
    }
    schedule();
  }
  function schedule() {
    if (!polling && watching.size > 0) {
      polling = true;
      startTimer(poll, POLL);
    }
  }
  const CHECK = Symbol("relevé");
  const rectOf = (x, y, width, height) => new DOMRectReadOnly(x, y, width, height);
  function margins(text) {
    const parts = String(text ?? "0px").trim().split(/\s+/).filter(Boolean);
    if (parts.length === 0) parts.push("0px");
    if (parts.length > 4 || !parts.every((part) => /^-?\d*\.?\d+(px|%)$/.test(part))) {
      throw new SyntaxError("rootMargin : des longueurs en px ou en % sont attendues");
    }
    const [top, right = top, bottom = top, left = right] = parts;
    return [top, right, bottom, left];
  }
  class IntersectionObserver {
    #callback;
    #root;
    #margins;
    #thresholds;
    #targets = new Map();
    constructor(callback, options = {}) {
      if (typeof callback !== "function") throw new TypeError("IntersectionObserver : une fonction est attendue");
      this.#callback = callback;
      this.#root = options?.root ?? null;
      this.#margins = margins(options?.rootMargin);
      const given = options?.threshold ?? 0;
      this.#thresholds = (Array.isArray(given) ? given.map(Number) : [Number(given)]).sort((a, b) => a - b);
      if (this.#thresholds.length === 0) this.#thresholds = [0];
      if (this.#thresholds.some((value) => !(value >= 0 && value <= 1))) {
        throw new RangeError("IntersectionObserver : un seuil est compris entre 0 et 1");
      }
    }
    get root() { return this.#root; }
    get rootMargin() { return this.#margins.join(" "); }
    get thresholds() { return this.#thresholds.slice(); }
    observe(target) {
      if (!(target instanceof Element)) throw new TypeError("IntersectionObserver.observe : un élément est attendu");
      if (!this.#targets.has(target)) this.#targets.set(target, undefined);
      watching.add(this);
      schedule();
    }
    unobserve(target) {
      this.#targets.delete(target);
      if (this.#targets.size === 0) watching.delete(this);
    }
    disconnect() {
      this.#targets.clear();
      watching.delete(this);
    }
    takeRecords() { return []; }
    [CHECK]() {
      const root = this.#root instanceof Element ? this.#root.getBoundingClientRect()
        : { left: 0, top: 0, right: global.innerWidth, bottom: global.innerHeight };
      const [width, height] = [root.right - root.left, root.bottom - root.top];
      const grow = (text, size) => (text.endsWith("%") ? parseFloat(text) * size / 100 : parseFloat(text));
      const [top, right, bottom, left] = this.#margins;
      const bounds = {
        left: root.left - grow(left, width), top: root.top - grow(top, height),
        right: root.right + grow(right, width), bottom: root.bottom + grow(bottom, height),
      };
      const entries = [];
      for (const [target, previous] of this.#targets) {
        const box = target.getBoundingClientRect();
        // Une boîte toute nulle : l'élément n'est pas dessiné.
        const drawn = target.isConnected
          && (box.width !== 0 || box.height !== 0 || box.left !== 0 || box.top !== 0);
        const [x0, y0] = [Math.max(box.left, bounds.left), Math.max(box.top, bounds.top)];
        const [x1, y1] = [Math.min(box.right, bounds.right), Math.min(box.bottom, bounds.bottom)];
        const isIntersecting = drawn && x0 <= x1 && y0 <= y1;
        const area = box.width * box.height;
        const ratio = !isIntersecting ? 0 : area > 0 ? Math.min(((x1 - x0) * (y1 - y0)) / area, 1) : 1;
        const rank = isIntersecting ? this.#thresholds.filter((value) => value <= ratio).length : -1;
        if (rank === previous) continue;
        this.#targets.set(target, rank);
        entries.push({
          time: performance.now(),
          target,
          rootBounds: rectOf(bounds.left, bounds.top, bounds.right - bounds.left, bounds.bottom - bounds.top),
          boundingClientRect: rectOf(box.left, box.top, box.width, box.height),
          intersectionRect: isIntersecting ? rectOf(x0, y0, x1 - x0, y1 - y0) : rectOf(0, 0, 0, 0),
          isIntersecting,
          intersectionRatio: ratio,
        });
      }
      if (entries.length > 0) this.#callback.call(this, entries, this);
    }
  }
  tag(IntersectionObserver, "IntersectionObserver");
  expose("IntersectionObserver", IntersectionObserver);
  expose("IntersectionObserverEntry", class IntersectionObserverEntry {});

  class ResizeObserver {
    #callback;
    #targets = new Map();
    constructor(callback) {
      if (typeof callback !== "function") throw new TypeError("ResizeObserver : une fonction est attendue");
      this.#callback = callback;
    }
    observe(target) {
      if (!(target instanceof Element)) throw new TypeError("ResizeObserver.observe : un élément est attendu");
      if (!this.#targets.has(target)) this.#targets.set(target, "0x0");
      watching.add(this);
      schedule();
    }
    unobserve(target) {
      this.#targets.delete(target);
      if (this.#targets.size === 0) watching.delete(this);
    }
    disconnect() {
      this.#targets.clear();
      watching.delete(this);
    }
    [CHECK]() {
      const entries = [];
      for (const [target, previous] of this.#targets) {
        const box = target.getBoundingClientRect();
        const size = box.width + "x" + box.height;
        if (size === previous) continue;
        this.#targets.set(target, size);
        // LIMITE : la boîte de contenu est prise égale à la boîte de bordure.
        const sizes = [{ inlineSize: box.width, blockSize: box.height }];
        entries.push({
          target,
          contentRect: rectOf(0, 0, box.width, box.height),
          borderBoxSize: sizes,
          contentBoxSize: sizes,
          devicePixelContentBoxSize: sizes,
        });
      }
      if (entries.length > 0) this.#callback.call(this, entries, this);
    }
  }
  tag(ResizeObserver, "ResizeObserver");
  expose("ResizeObserver", ResizeObserver);
  expose("ResizeObserverEntry", class ResizeObserverEntry {});

  // LIMITE : aucune mesure n'est publiée ; l'observateur ne reçoit rien.
  class PerformanceObserver {
    static supportedEntryTypes = Object.freeze([]);
    constructor(callback) {
      if (typeof callback !== "function") throw new TypeError("PerformanceObserver : une fonction est attendue");
    }
    observe() {}
    disconnect() {}
    takeRecords() { return []; }
  }
  tag(PerformanceObserver, "PerformanceObserver");
  expose("PerformanceObserver", PerformanceObserver);

  return {
    // Touche enfoncée (`down`) ou relâchée : `keydown` (puis `keypress` pour
    // un caractère) ou `keyup`, sur l'élément actif. Rend `false` si
    // l'action par défaut a été annulée.
    key(down, key, code, ctrlKey, shiftKey, altKey, metaKey, repeat) {
      const target = document.activeElement ?? document;
      const keyCode = legacyCode(key);
      const init = {
        key, code, ctrlKey, shiftKey, altKey, metaKey, repeat, keyCode, which: keyCode,
        bubbles: true, cancelable: true, view: global,
      };
      let proceed = target.dispatchEvent(new global.KeyboardEvent(down ? "keydown" : "keyup", init));
      if (down && proceed && key.length === 1 && !ctrlKey && !altKey && !metaKey) {
        const charCode = key.charCodeAt(0);
        proceed = target.dispatchEvent(new global.KeyboardEvent("keypress", {
          ...init, keyCode: charCode, charCode, which: charCode,
        }));
      }
      return proceed;
    },
    // Fabrique des exceptions du DOM levées par le moteur.
    domException(name, message) {
      return new DOMException(message, name);
    },
    // L'embarqueur a parcouru l'historique de `delta` entrées sans quitter
    // ce document : `popstate`, et `hashchange` si seul le fragment change.
    traverse(delta, before) {
      position = Math.min(Math.max(position + delta, 0), sessionEntries.length - 1);
      const after = locationHref();
      global.dispatchEvent(new global.PopStateEvent("popstate", { state: sessionEntries[position].state }));
      const strip = (href) => href.replace(/#.*$/, "");
      if (before !== after && strip(before) === strip(after)) {
        global.dispatchEvent(new global.HashChangeEvent("hashchange", { oldURL: before, newURL: after }));
      }
    },
  };
})
