// `WebSocket` (WebSockets Standard), écrit en JavaScript. Les connexions
// sont ouvertes par l'embarqueur ; le moteur fait le lien.
//
// Fonction appelée par le moteur avec :
//   socketOpen(adresse, protocoles, rappel) -> identifiant, ou null si la page
//       a trop de connexions ; `rappel(nature, a, b, c)` reçoit ensuite
//       "open" (protocole), "text" (texte), "binary" (chaîne binaire) et
//       "close" (code, raison, dans les règles) ;
//   socketSend(id, données, binaire) ;
//   socketClose(id, code, raison).
//
// LIMITES CONNUES : `bufferedAmount` vaut toujours 0 ; `extensions` est
// toujours vide (aucune extension n'est négociée) ; un `Blob` envoyé part
// après les messages envoyés dans la même tâche.
(function (socketOpen, socketSend, socketClose) {
  "use strict";
  const global = globalThis;
  const define = Object.defineProperty;
  const EventTargetBase = global.EventTarget;
  const later = (task) => global.setTimeout(task, 0);
  const encoder = new global.TextEncoder();
  const CONNECTING = 0;
  const OPEN = 1;
  const CLOSING = 2;
  const CLOSED = 3;
  // Jeton HTTP : ce qu'un nom de sous-protocole peut contenir.
  const TOKEN = /^[!#$%&'*+\-.^_`|~0-9A-Za-z]+$/;

  // Texte sans demi-paire isolée : chacune devient U+FFFD, comme le veut la
  // conversion en chaîne de valeurs scalaires.
  const wellFormed = (text) => {
    let out = "";
    for (let i = 0; i < text.length; i++) {
      const unit = text.charCodeAt(i);
      if (unit >= 0xd800 && unit <= 0xdbff) {
        const next = text.charCodeAt(i + 1);
        if (next >= 0xdc00 && next <= 0xdfff) {
          out += text[i] + text[i + 1];
          i++;
        } else {
          out += "�";
        }
      } else if (unit >= 0xdc00 && unit <= 0xdfff) {
        out += "�";
      } else {
        out += text[i];
      }
    }
    return out;
  };
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

  class WebSocket extends EventTargetBase {
    #id = null;
    #state = CONNECTING;
    #url;
    #protocol = "";
    #binaryType = "blob";
    constructor(url, protocols = []) {
      super();
      if (arguments.length < 1) throw new TypeError("WebSocket : l'adresse est requise");
      let target;
      try {
        target = new global.URL(String(url), global.document?.baseURI ?? global.location.href);
      } catch {
        throw new global.DOMException("WebSocket : adresse invalide", "SyntaxError");
      }
      if (target.protocol === "http:") target.protocol = "ws:";
      else if (target.protocol === "https:") target.protocol = "wss:";
      if (target.protocol !== "ws:" && target.protocol !== "wss:") {
        throw new global.DOMException("WebSocket : le schéma doit être ws ou wss", "SyntaxError");
      }
      if (target.hash !== "") {
        throw new global.DOMException("WebSocket : l'adresse ne peut pas porter de fragment", "SyntaxError");
      }
      const offered = typeof protocols === "string" ? [protocols] : Array.from(protocols, String);
      for (const [rank, protocol] of offered.entries()) {
        if (!TOKEN.test(protocol) || offered.indexOf(protocol) !== rank) {
          throw new global.DOMException("WebSocket : sous-protocole invalide ou répété", "SyntaxError");
        }
      }
      this.#url = target.href;
      this.onopen = null;
      this.onmessage = null;
      this.onerror = null;
      this.onclose = null;
      // Une page sûre n'ouvre pas de connexion en clair, sauf vers cette machine.
      const local = /^(localhost|127\.\d+\.\d+\.\d+|\[::1\])$/.test(target.hostname);
      const refused = global.location.protocol === "https:" && target.protocol === "ws:" && !local;
      const id = refused ? null : socketOpen(target.href, offered.join(","), (kind, a, b, c) => {
        this.#handle(kind, a, b, c);
      });
      if (id === null) {
        // Connexion refusée d'emblée : elle échoue dans une tâche ultérieure.
        later(() => this.#handle("close", 1006, "", false));
      } else {
        this.#id = id;
      }
    }
    #handle(kind, a, b, c) {
      if (this.#state === CLOSED) return;
      if (kind === "open") {
        // Fermée par le script avant même d'être ouverte : rien à annoncer.
        if (this.#state !== CONNECTING) return;
        this.#state = OPEN;
        this.#protocol = a;
        this.dispatchEvent(new global.Event("open"));
      } else if (kind === "text" || kind === "binary") {
        if (this.#state !== OPEN) return;
        let data = a;
        if (kind === "binary") {
          const bytes = fromBinary(a);
          data = this.#binaryType === "arraybuffer" ? bytes.buffer : new global.Blob([bytes]);
        }
        const origin = new global.URL(this.#url).origin;
        this.dispatchEvent(new global.MessageEvent("message", { data, origin }));
      } else if (kind === "close") {
        this.#state = CLOSED;
        if (!c) this.dispatchEvent(new global.Event("error"));
        this.dispatchEvent(new global.CloseEvent("close", { wasClean: c, code: a, reason: b }));
      }
    }
    get url() { return this.#url; }
    get readyState() { return this.#state; }
    get protocol() { return this.#protocol; }
    get extensions() { return ""; }
    get bufferedAmount() { return 0; }
    get binaryType() { return this.#binaryType; }
    set binaryType(value) {
      if (value === "blob" || value === "arraybuffer") this.#binaryType = value;
    }
    send(data) {
      if (arguments.length < 1) throw new TypeError("WebSocket.send : des données sont requises");
      if (this.#state === CONNECTING) {
        throw new global.DOMException("la connexion n'est pas encore ouverte", "InvalidStateError");
      }
      const open = () => this.#state === OPEN && this.#id !== null;
      if (data instanceof global.Blob) {
        data.arrayBuffer().then((buffer) => {
          if (open()) socketSend(this.#id, toBinary(new Uint8Array(buffer)), true);
        });
      } else if (data instanceof ArrayBuffer) {
        if (open()) socketSend(this.#id, toBinary(new Uint8Array(data)), true);
      } else if (ArrayBuffer.isView(data)) {
        const bytes = new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
        if (open()) socketSend(this.#id, toBinary(bytes), true);
      } else if (open()) {
        // Une demi-paire isolée devient U+FFFD, comme pour tout texte envoyé.
        socketSend(this.#id, wellFormed(String(data)), false);
      }
    }
    close(code = undefined, reason = "") {
      if (code !== undefined) {
        code = Number(code) & 0xffff;
        if (code !== 1000 && (code < 3000 || code > 4999)) {
          throw new global.DOMException("code de fermeture non permis", "InvalidAccessError");
        }
      }
      const text = wellFormed(String(reason));
      if (encoder.encode(text).length > 123) {
        throw new global.DOMException("raison de fermeture trop longue", "SyntaxError");
      }
      if (this.#state === CLOSING || this.#state === CLOSED) return;
      this.#state = CLOSING;
      if (this.#id !== null) socketClose(this.#id, code, text);
    }
  }
  for (const [name, value] of [["CONNECTING", CONNECTING], ["OPEN", OPEN], ["CLOSING", CLOSING], ["CLOSED", CLOSED]]) {
    define(WebSocket, name, { value, enumerable: true });
    define(WebSocket.prototype, name, { value, enumerable: true });
  }
  define(WebSocket.prototype, Symbol.toStringTag, { value: "WebSocket", configurable: true });
  define(global, "WebSocket", { value: WebSocket, writable: true, enumerable: false, configurable: true });
})
