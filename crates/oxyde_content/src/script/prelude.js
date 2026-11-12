// Prélude des pages : API du web écrites en JavaScript (URL Standard, Fetch,
// HTML Standard, DOM Standard). Le script rend une fabrique, appelée avec
// les opérations que seul le moteur sait faire :
//   parseUrl(texte, base)        -> composants de l'adresse, ou null ;
//   changeUrl(href, rang, valeur) -> composants après modification, ou null.
// Les composants sont, dans l'ordre : href, protocol, username, password,
// host, hostname, port, pathname, search, hash, origin.
//
// LIMITES CONNUES : `TextDecoder` ne connaît que l'UTF-8 ; `structuredClone`
// ne transfère rien et ignore les types exotiques ; `matchMedia` ne comprend
// que les largeurs, les hauteurs et `prefers-color-scheme`.
(function (parseUrl, changeUrl) {
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
})
