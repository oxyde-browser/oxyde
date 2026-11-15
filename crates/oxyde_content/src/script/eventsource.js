// `EventSource` (HTML Standard §9.2, événements envoyés par le serveur),
// écrit en JavaScript. Le flux est ouvert par l'embarqueur, par le même canal
// que les connexions `WebSocket` : une adresse `http(s)` y désigne un flux
// d'événements.
//
// Fonction appelée par le moteur avec :
//   socketOpen(adresse, dernierId, rappel) -> identifiant, ou null ;
//       `rappel(nature, a, b, c)` reçoit "open" quand la réponse est un flux
//       d'événements, "binary" (chaîne binaire) pour chaque morceau, et
//       "close" (code, raison, dans les règles) à la fin : code 1000 (flux
//       fini) ou 1006 (coupure) pour une fin après laquelle se reconnecter,
//       tout autre code pour un refus définitif (réponse qui n'est pas un
//       flux d'événements, autre origine non autorisée) ;
//   socketClose(id, code, raison).
//
// LIMITES CONNUES : `withCredentials` est retenu sans effet (les cookies
// suivent les règles ordinaires du site) ; le délai de reconnexion ne
// s'allonge pas après des échecs répétés.
(function (socketOpen, socketClose) {
  "use strict";
  const global = globalThis;
  const define = Object.defineProperty;
  const EventTargetBase = global.EventTarget;
  const CONNECTING = 0;
  const OPEN = 1;
  const CLOSED = 2;
  // Délai avant de se reconnecter, tant que le serveur n'en donne pas.
  const DEFAULT_RETRY = 3000;

  const fromBinary = (text) => {
    const bytes = new Uint8Array(text.length);
    for (let i = 0; i < text.length; i++) bytes[i] = text.charCodeAt(i);
    return bytes;
  };

  class EventSource extends EventTargetBase {
    #url;
    #withCredentials;
    #state = CONNECTING;
    #id = null;
    #timer = 0;
    #retry = DEFAULT_RETRY;
    #lastEventId = "";
    // Lecture du flux : décodeur, texte pas encore découpé en lignes, champs
    // de l'événement en cours.
    #decoder = null;
    #pending = "";
    #data = "";
    #type = "";
    #candidateId = "";
    constructor(url, init = undefined) {
      super();
      if (arguments.length < 1) throw new TypeError("EventSource : l'adresse est requise");
      let target;
      try {
        target = new global.URL(String(url), global.document?.baseURI ?? global.location.href);
      } catch {
        throw new global.DOMException("EventSource : adresse invalide", "SyntaxError");
      }
      this.#url = target.href;
      this.#withCredentials = Boolean(init && init.withCredentials);
      this.onopen = null;
      this.onmessage = null;
      this.onerror = null;
      this.#connect();
    }
    #connect() {
      if (this.#state === CLOSED) return;
      this.#decoder = new global.TextDecoder();
      this.#pending = "";
      this.#data = "";
      this.#type = "";
      this.#candidateId = this.#lastEventId;
      const scheme = new global.URL(this.#url).protocol;
      const id = scheme === "http:" || scheme === "https:"
        ? socketOpen(this.#url, this.#lastEventId, (kind, a) => this.#handle(kind, a))
        : null;
      if (id === null) {
        // Adresse qui ne se lit pas ainsi, ou trop de connexions : échec
        // définitif, annoncé dans une tâche ultérieure.
        global.setTimeout(() => this.#handle("close", 0), 0);
      } else {
        this.#id = id;
      }
    }
    #handle(kind, a) {
      if (this.#state === CLOSED) return;
      if (kind === "open") {
        this.#state = OPEN;
        this.dispatchEvent(new global.Event("open"));
      } else if (kind === "binary") {
        if (this.#state === OPEN) this.#feed(this.#decoder.decode(fromBinary(a), { stream: true }));
      } else if (kind === "close") {
        this.#id = null;
        if (a === 1000 || a === 1006) {
          // Flux fini ou coupé : nouvelle tentative après le délai.
          this.#state = CONNECTING;
          this.dispatchEvent(new global.Event("error"));
          if (this.#state === CONNECTING) {
            this.#timer = global.setTimeout(() => this.#connect(), this.#retry);
          }
        } else {
          this.#state = CLOSED;
          this.dispatchEvent(new global.Event("error"));
        }
      }
    }
    // Découpe le texte reçu en lignes (finies par CR LF, LF ou CR) et les
    // interprète ; la dernière, peut-être incomplète, attend la suite.
    #feed(text) {
      this.#pending += text;
      for (;;) {
        const found = /\r\n|\n|\r/.exec(this.#pending);
        if (found === null) return;
        // Un CR en toute fin de texte peut être la moitié d'un CR LF.
        if (found[0] === "\r" && found.index === this.#pending.length - 1) return;
        const line = this.#pending.slice(0, found.index);
        this.#pending = this.#pending.slice(found.index + found[0].length);
        this.#line(line);
        if (this.#state !== OPEN) return;
      }
    }
    #line(line) {
      if (line === "") {
        this.#dispatch();
        return;
      }
      if (line[0] === ":") return;
      const colon = line.indexOf(":");
      const field = colon < 0 ? line : line.slice(0, colon);
      let value = colon < 0 ? "" : line.slice(colon + 1);
      if (value[0] === " ") value = value.slice(1);
      if (field === "event") this.#type = value;
      else if (field === "data") this.#data += value + "\n";
      else if (field === "id") {
        if (!value.includes("\0")) this.#candidateId = value;
      } else if (field === "retry") {
        if (/^[0-9]+$/.test(value)) this.#retry = Number(value);
      }
    }
    #dispatch() {
      this.#lastEventId = this.#candidateId;
      const data = this.#data;
      const type = this.#type;
      this.#data = "";
      this.#type = "";
      if (data === "") return;
      this.dispatchEvent(new global.MessageEvent(type === "" ? "message" : type, {
        data: data.slice(0, -1),
        origin: new global.URL(this.#url).origin,
        lastEventId: this.#lastEventId,
      }));
    }
    get url() { return this.#url; }
    get withCredentials() { return this.#withCredentials; }
    get readyState() { return this.#state; }
    close() {
      this.#state = CLOSED;
      global.clearTimeout(this.#timer);
      if (this.#id !== null) {
        socketClose(this.#id, undefined, "");
        this.#id = null;
      }
    }
  }
  for (const [name, value] of [["CONNECTING", CONNECTING], ["OPEN", OPEN], ["CLOSED", CLOSED]]) {
    define(EventSource, name, { value, enumerable: true });
    define(EventSource.prototype, name, { value, enumerable: true });
  }
  define(EventSource.prototype, Symbol.toStringTag, { value: "EventSource", configurable: true });
  define(global, "EventSource", { value: EventSource, writable: true, enumerable: false, configurable: true });
})
