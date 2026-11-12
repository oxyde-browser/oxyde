// Fetch Standard et File API, écrits en JavaScript : `Blob`, `File`,
// `FileReader`, `Request`, `Response` et la fonction `fetch` qui les relie
// au réseau. Le script rend une fabrique appelée avec le `fetch` du moteur,
// qui prend une adresse et `{ method, headers, binaryBody }` et rend un
// objet brut (`status`, `url`, `headers.forEach`, `binary()`…).
//
// Les octets voyagent entre le moteur et ce script sous forme de « chaînes
// binaires » : un caractère de code 0 à 255 par octet.
//
// LIMITES CONNUES : pas de flux (`body` vaut toujours `null`, `Blob.stream`
// manque) ; `Response.formData()` ne lit que les formulaires encodés dans
// l'adresse ; les requêtes ne sont pas mises en cache ; `Request.signal`
// n'interrompt pas une requête déjà partie.
(function (nativeFetch) {
  "use strict";
  const global = globalThis;
  const define = Object.defineProperty;
  const expose = (name, value) => {
    define(global, name, { value, writable: true, enumerable: false, configurable: true });
  };
  const tag = (ctor, name) => {
    define(ctor.prototype, Symbol.toStringTag, { value: name, configurable: true });
  };
  const encoder = new global.TextEncoder();
  const decoder = new global.TextDecoder();

  // ---- Octets --------------------------------------------------------------------------
  const toBinary = (bytes) => {
    let out = "";
    for (let i = 0; i < bytes.length; i += 8192) {
      out += String.fromCharCode.apply(null, bytes.subarray(i, i + 8192));
    }
    return out;
  };
  const fromBinary = (text) => {
    const bytes = new Uint8Array(text.length);
    for (let i = 0; i < text.length; i++) bytes[i] = text.charCodeAt(i) & 0xFF;
    return bytes;
  };
  const concat = (chunks) => {
    let size = 0;
    for (const chunk of chunks) size += chunk.length;
    const out = new Uint8Array(size);
    let offset = 0;
    for (const chunk of chunks) {
      out.set(chunk, offset);
      offset += chunk.length;
    }
    return out;
  };
  const viewBytes = (value) => {
    if (value instanceof ArrayBuffer) return new Uint8Array(value.slice(0));
    if (ArrayBuffer.isView(value)) {
      return new Uint8Array(value.buffer.slice(value.byteOffset, value.byteOffset + value.byteLength));
    }
    return null;
  };

  // ---- Blob et File --------------------------------------------------------------------
  const blobs = new WeakMap();
  const blobBytes = (blob) => {
    const bytes = blobs.get(blob);
    if (!bytes) throw new TypeError("receveur attendu : un Blob");
    return bytes;
  };
  const cleanType = (type) => {
    const text = String(type);
    return /[^ -~]/.test(text) ? "" : text.toLowerCase();
  };
  class Blob {
    #type;
    constructor(parts = [], options = {}) {
      if (parts === null || typeof parts !== "object" || typeof parts[Symbol.iterator] !== "function") {
        throw new TypeError("Blob : une séquence de parties est attendue");
      }
      const chunks = [];
      for (const part of parts) {
        const bytes = viewBytes(part);
        if (bytes) chunks.push(bytes);
        else if (part instanceof Blob) chunks.push(blobBytes(part));
        else chunks.push(encoder.encode(String(part)));
      }
      blobs.set(this, concat(chunks));
      this.#type = options && options.type !== undefined ? cleanType(options.type) : "";
    }
    get size() { return blobBytes(this).length; }
    get type() { return this.#type; }
    slice(start = 0, end = undefined, contentType = "") {
      const bytes = blobBytes(this);
      const size = bytes.length;
      const clamp = (n) => (n < 0 ? Math.max(size + n, 0) : Math.min(n, size));
      const from = clamp(Math.trunc(Number(start)) || 0);
      const to = end === undefined ? size : clamp(Math.trunc(Number(end)) || 0);
      const blob = new Blob([], { type: contentType });
      blobs.set(blob, bytes.slice(from, Math.max(from, to)));
      return blob;
    }
    text() { return Promise.resolve(decoder.decode(blobBytes(this))); }
    arrayBuffer() { return Promise.resolve(blobBytes(this).slice().buffer); }
    bytes() { return Promise.resolve(blobBytes(this).slice()); }
  }
  tag(Blob, "Blob");
  expose("Blob", Blob);

  class File extends Blob {
    #name;
    #modified;
    constructor(parts, name, options = {}) {
      if (arguments.length < 2) throw new TypeError("File attend deux arguments");
      super(parts, options);
      this.#name = String(name);
      this.#modified = options && options.lastModified !== undefined
        ? Math.trunc(Number(options.lastModified)) || 0
        : Date.now();
    }
    get name() { return this.#name; }
    get lastModified() { return this.#modified; }
    get webkitRelativePath() { return ""; }
  }
  tag(File, "File");
  expose("File", File);

  // ---- FileReader ------------------------------------------------------------------------
  class FileReader extends global.EventTarget {
    #state = 0;
    #result = null;
    #error = null;
    static EMPTY = 0;
    static LOADING = 1;
    static DONE = 2;
    get EMPTY() { return 0; }
    get LOADING() { return 1; }
    get DONE() { return 2; }
    get readyState() { return this.#state; }
    get result() { return this.#result; }
    get error() { return this.#error; }
    #fire(type) {
      const event = new global.Event(type);
      const handler = this["on" + type];
      if (typeof handler === "function") {
        try { handler.call(this, event); } catch (thrown) { global.reportError?.(thrown); }
      }
      this.dispatchEvent(event);
    }
    #read(blob, convert) {
      if (!(blob instanceof Blob)) throw new TypeError("FileReader : un Blob est attendu");
      if (this.#state === 1) {
        throw new global.DOMException("une lecture est déjà en cours", "InvalidStateError");
      }
      this.#state = 1;
      this.#result = null;
      this.#error = null;
      const bytes = blobBytes(blob);
      global.setTimeout(() => {
        if (this.#state !== 1) return;
        this.#fire("loadstart");
        try {
          this.#result = convert(bytes, blob);
          this.#state = 2;
          this.#fire("progress");
          this.#fire("load");
        } catch (thrown) {
          this.#error = thrown;
          this.#state = 2;
          this.#fire("error");
        }
        this.#fire("loadend");
      }, 0);
    }
    readAsText(blob) { this.#read(blob, (bytes) => decoder.decode(bytes)); }
    readAsArrayBuffer(blob) { this.#read(blob, (bytes) => bytes.slice().buffer); }
    readAsBinaryString(blob) { this.#read(blob, (bytes) => toBinary(bytes)); }
    readAsDataURL(blob) {
      this.#read(blob, (bytes, source) => {
        const type = source.type || "application/octet-stream";
        return "data:" + type + ";base64," + global.btoa(toBinary(bytes));
      });
    }
    abort() {
      if (this.#state !== 1) return;
      this.#state = 2;
      this.#result = null;
      this.#fire("abort");
      this.#fire("loadend");
    }
  }
  tag(FileReader, "FileReader");
  expose("FileReader", FileReader);

  // ---- Corps -----------------------------------------------------------------------------
  // Octets et type MIME d'un corps de requête ou de réponse.
  function extractBody(body) {
    if (body === null || body === undefined) return { bytes: null, type: null };
    if (body instanceof Blob) return { bytes: blobBytes(body), type: body.type || null };
    const view = viewBytes(body);
    if (view) return { bytes: view, type: null };
    if (body instanceof global.URLSearchParams) {
      return {
        bytes: encoder.encode(String(body)),
        type: "application/x-www-form-urlencoded;charset=UTF-8",
      };
    }
    if (typeof global.FormData === "function" && body instanceof global.FormData) {
      const boundary = "----OxydeFormBoundary" + Math.random().toString(36).slice(2);
      const chunks = [];
      const quote = (text) => String(text).replace(/\r?\n|\r/g, "%0D%0A").replace(/"/g, "%22");
      for (const [name, value] of body) {
        let head = "--" + boundary + '\r\nContent-Disposition: form-data; name="' + quote(name) + '"';
        if (value instanceof Blob) {
          const filename = value instanceof File ? value.name : "blob";
          head += '; filename="' + quote(filename) + '"\r\nContent-Type: '
            + (value.type || "application/octet-stream") + "\r\n\r\n";
          chunks.push(encoder.encode(head), blobBytes(value), encoder.encode("\r\n"));
        } else {
          chunks.push(encoder.encode(head + "\r\n\r\n" + String(value) + "\r\n"));
        }
      }
      chunks.push(encoder.encode("--" + boundary + "--\r\n"));
      return { bytes: concat(chunks), type: "multipart/form-data; boundary=" + boundary };
    }
    return { bytes: encoder.encode(String(body)), type: "text/plain;charset=UTF-8" };
  }

  // Méthodes de lecture du corps, communes à `Request` et à `Response`.
  const bodies = new WeakMap();
  const bodyOf = (object) => {
    const body = bodies.get(object);
    if (!body) throw new TypeError("receveur attendu : une requête ou une réponse");
    return body;
  };
  function consume(object) {
    const body = bodyOf(object);
    if (body.used) return Promise.reject(new TypeError("le corps a déjà été lu"));
    if (body.bytes !== null) body.used = true;
    return Promise.resolve(body.bytes ?? new Uint8Array(0));
  }
  const bodyMixin = {
    text() { return consume(this).then((bytes) => decoder.decode(bytes)); },
    json() { return consume(this).then((bytes) => JSON.parse(decoder.decode(bytes))); },
    arrayBuffer() { return consume(this).then((bytes) => bytes.slice().buffer); },
    bytes() { return consume(this).then((bytes) => bytes.slice()); },
    blob() {
      const type = this.headers.get("content-type") ?? "";
      return consume(this).then((bytes) => {
        const blob = new Blob([], { type });
        blobs.set(blob, bytes.slice());
        return blob;
      });
    },
    formData() {
      return consume(this).then((bytes) => {
        const data = new global.FormData();
        for (const [name, value] of new global.URLSearchParams(decoder.decode(bytes))) {
          data.append(name, value);
        }
        return data;
      });
    },
  };
  const addBody = (ctor) => {
    for (const [name, value] of Object.entries(bodyMixin)) {
      define(ctor.prototype, name, { value, writable: true, enumerable: true, configurable: true });
    }
    define(ctor.prototype, "bodyUsed", {
      get() { return bodyOf(this).used; }, enumerable: true, configurable: true,
    });
    define(ctor.prototype, "body", {
      get() { bodyOf(this); return null; }, enumerable: true, configurable: true,
    });
  };

  // ---- Request ---------------------------------------------------------------------------
  const METHODS = ["DELETE", "GET", "HEAD", "OPTIONS", "POST", "PUT"];
  const normalizeMethod = (method) => {
    const text = String(method);
    if (!/^[!#$%&'*+\-.^_`|~0-9A-Za-z]+$/.test(text)) {
      throw new TypeError("méthode HTTP invalide : « " + text + " »");
    }
    const upper = text.toUpperCase();
    if (upper === "CONNECT" || upper === "TRACE" || upper === "TRACK") {
      throw new TypeError("la méthode « " + text + " » n'est pas permise");
    }
    return METHODS.includes(upper) ? upper : text;
  };
  const choice = (init, name, allowed, fallback) => {
    if (init[name] === undefined) return fallback;
    const value = String(init[name]);
    if (!allowed.includes(value)) {
      throw new TypeError("valeur invalide pour « " + name + " » : « " + value + " »");
    }
    return value;
  };
  class Request {
    #url; #method; #headers; #signal; #mode; #credentials; #cache; #redirect; #referrer;
    #integrity; #keepalive;
    constructor(input, init = {}) {
      if (arguments.length < 1) throw new TypeError("Request attend au moins un argument");
      if (init === null || init === undefined) init = {};
      const source = input instanceof Request ? input : null;
      let url;
      if (source) {
        url = source.url;
      } else {
        let parsed;
        try {
          parsed = new global.URL(String(input), global.location ? global.location.href : undefined);
        } catch {
          throw new TypeError("adresse de requête invalide : « " + String(input) + " »");
        }
        if (parsed.username || parsed.password) {
          throw new TypeError("une adresse de requête ne porte pas d'identifiants");
        }
        url = parsed.href;
      }
      this.#url = url;
      this.#method = init.method !== undefined ? normalizeMethod(init.method) : source ? source.method : "GET";
      this.#headers = new global.Headers(init.headers !== undefined ? init.headers : source ? source.headers : undefined);
      this.#mode = choice(init, "mode", ["same-origin", "no-cors", "cors", "navigate"], source ? source.mode : "cors");
      if (this.#mode === "navigate") throw new TypeError("le mode « navigate » n'est pas permis");
      this.#credentials = choice(init, "credentials", ["omit", "same-origin", "include"], source ? source.credentials : "same-origin");
      this.#cache = choice(init, "cache", ["default", "no-store", "reload", "no-cache", "force-cache", "only-if-cached"], source ? source.cache : "default");
      this.#redirect = choice(init, "redirect", ["follow", "error", "manual"], source ? source.redirect : "follow");
      this.#referrer = init.referrer !== undefined ? String(init.referrer) : source ? source.referrer : "about:client";
      this.#integrity = init.integrity !== undefined ? String(init.integrity) : source ? source.integrity : "";
      this.#keepalive = init.keepalive !== undefined ? Boolean(init.keepalive) : source ? source.keepalive : false;
      const signal = init.signal !== undefined ? init.signal : source ? source.signal : null;
      if (signal !== null && !(signal instanceof global.AbortSignal)) {
        throw new TypeError("« signal » doit être un AbortSignal");
      }
      this.#signal = signal ?? new global.AbortController().signal;
      let body = { bytes: null, type: null };
      if (init.body !== undefined && init.body !== null) {
        body = extractBody(init.body);
      } else if (source && init.body === undefined) {
        const inherited = bodyOf(source);
        if (inherited.used) throw new TypeError("le corps de la requête a déjà été lu");
        body = { bytes: inherited.bytes, type: null };
        if (inherited.bytes !== null) inherited.used = true;
      }
      if (body.bytes !== null && (this.#method === "GET" || this.#method === "HEAD")) {
        throw new TypeError("une requête " + this.#method + " ne peut pas avoir de corps");
      }
      if (body.type !== null && !this.#headers.has("content-type")) {
        this.#headers.set("content-type", body.type);
      }
      bodies.set(this, { bytes: body.bytes, used: false });
    }
    get url() { return this.#url; }
    get method() { return this.#method; }
    get headers() { return this.#headers; }
    get signal() { return this.#signal; }
    get mode() { return this.#mode; }
    get credentials() { return this.#credentials; }
    get cache() { return this.#cache; }
    get redirect() { return this.#redirect; }
    get referrer() { return this.#referrer; }
    get referrerPolicy() { return ""; }
    get integrity() { return this.#integrity; }
    get keepalive() { return this.#keepalive; }
    get destination() { return ""; }
    get duplex() { return "half"; }
    clone() {
      const body = bodyOf(this);
      if (body.used) throw new TypeError("le corps de la requête a déjà été lu");
      const copy = new Request(this.#url, {
        method: this.#method, headers: this.#headers, mode: this.#mode,
        credentials: this.#credentials, cache: this.#cache, redirect: this.#redirect,
        referrer: this.#referrer, integrity: this.#integrity, keepalive: this.#keepalive,
        signal: this.#signal,
      });
      bodies.set(copy, { bytes: body.bytes, used: false });
      return copy;
    }
  }
  addBody(Request);
  tag(Request, "Request");
  expose("Request", Request);

  // ---- Response --------------------------------------------------------------------------
  const NULL_BODY = [101, 103, 204, 205, 304];
  const REDIRECTS = [301, 302, 303, 307, 308];
  const responses = new WeakMap();
  const stateOf = (response) => {
    const state = responses.get(response);
    if (!state) throw new TypeError("receveur attendu : un Response");
    return state;
  };
  class Response {
    constructor(body = null, init = {}) {
      if (init === null || init === undefined) init = {};
      const status = init.status !== undefined ? Number(init.status) : 200;
      if (!Number.isInteger(status) || status < 200 || status > 599) {
        throw new RangeError("code d'état invalide : " + init.status);
      }
      const statusText = init.statusText !== undefined ? String(init.statusText) : "";
      if (/[^\t -~\u0080-ÿ]/.test(statusText)) {
        throw new TypeError("texte d'état invalide");
      }
      const headers = new global.Headers(init.headers);
      const extracted = extractBody(body);
      if (extracted.bytes !== null && NULL_BODY.includes(status)) {
        throw new TypeError("une réponse " + status + " n'a pas de corps");
      }
      if (extracted.type !== null && !headers.has("content-type")) {
        headers.set("content-type", extracted.type);
      }
      responses.set(this, { status, statusText, headers, type: "default", url: "", redirected: false });
      bodies.set(this, { bytes: extracted.bytes, used: false });
    }
    static error() {
      const response = new Response(null);
      Object.assign(stateOf(response), { status: 0, type: "error" });
      return response;
    }
    static redirect(url, status = 302) {
      const target = new global.URL(String(url), global.location ? global.location.href : undefined);
      if (!REDIRECTS.includes(Number(status))) {
        throw new RangeError("code de redirection invalide : " + status);
      }
      const response = new Response(null, { status: Number(status) });
      response.headers.set("location", target.href);
      return response;
    }
    static json(data, init = {}) {
      const text = JSON.stringify(data);
      if (text === undefined) throw new TypeError("cette valeur ne se sérialise pas en JSON");
      const headers = new global.Headers((init || {}).headers);
      if (!headers.has("content-type")) headers.set("content-type", "application/json");
      return new Response(text, { ...init, headers });
    }
    get status() { return stateOf(this).status; }
    get ok() { const status = stateOf(this).status; return status >= 200 && status <= 299; }
    get statusText() { return stateOf(this).statusText; }
    get headers() { return stateOf(this).headers; }
    get type() { return stateOf(this).type; }
    get url() { return stateOf(this).url; }
    get redirected() { return stateOf(this).redirected; }
    clone() {
      const state = stateOf(this);
      const body = bodyOf(this);
      if (body.used) throw new TypeError("le corps de la réponse a déjà été lu");
      const copy = new Response(null, { status: 200 });
      responses.set(copy, { ...state, headers: new global.Headers(state.headers) });
      bodies.set(copy, { bytes: body.bytes, used: false });
      return copy;
    }
  }
  addBody(Response);
  tag(Response, "Response");
  expose("Response", Response);

  // ---- fetch -----------------------------------------------------------------------------
  function fetch(input, init) {
    let request;
    try {
      if (arguments.length < 1) throw new TypeError("fetch attend au moins un argument");
      request = new Request(input, init);
    } catch (thrown) {
      return Promise.reject(thrown);
    }
    const signal = request.signal;
    if (signal.aborted) {
      return Promise.reject(signal.reason ?? new global.DOMException("requête abandonnée", "AbortError"));
    }
    const body = bodyOf(request);
    const options = { method: request.method, headers: [...request.headers] };
    if (body.bytes !== null) {
      options.binaryBody = toBinary(body.bytes);
      body.used = true;
    }
    const answered = nativeFetch(request.url, options).then((raw) => {
      const headers = new global.Headers();
      raw.headers.forEach((value, name) => headers.append(name, value));
      const response = new Response(null);
      responses.set(response, {
        status: raw.status, statusText: raw.statusText, headers, type: raw.type, url: raw.url,
        redirected: raw.redirected,
      });
      bodies.set(response, { bytes: fromBinary(raw.binary()), used: false });
      return response;
    });
    return new Promise((resolve, reject) => {
      const abort = () => {
        reject(signal.reason ?? new global.DOMException("requête abandonnée", "AbortError"));
      };
      signal.addEventListener("abort", abort, { once: true });
      answered.then(resolve, reject);
    });
  }
  return fetch;
})
