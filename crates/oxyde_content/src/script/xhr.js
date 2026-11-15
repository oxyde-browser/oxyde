// `XMLHttpRequest` (https://xhr.spec.whatwg.org/), écrit en JavaScript
// par-dessus `fetch` et `EventTarget`. Le script rend une fabrique, appelée
// avec le `fetch` d'origine : une page qui remplace `window.fetch` ne change
// pas le comportement de `XMLHttpRequest`.
//
// LIMITES CONNUES : pas de requête synchrone ; pas d'événements de
// progression intermédiaires ; `withCredentials` est sans effet (pas de
// cookies) ; `responseText` reste lisible quel que soit `responseType` ;
// le texte est lu en UTF-8 sauf encodage annoncé par l'en-tête.
(function (fetch) {
  "use strict";
  const EventTargetBase = EventTarget;
  const EventCtor = Event;
  const startTimer = setTimeout;
  const stopTimer = clearTimeout;
  const parseJSON = JSON.parse;
  const Parser = DOMParser;
  const PARSER_ERROR = "http://www.mozilla.org/newlayout/xml/parsererror.xml";
  // Document de la réponse `text` servie comme `mime` : XML pour un type XML
  // (ou sans type), HTML seulement si le script a demandé un document ;
  // null sinon, ou si le XML est mal formé.
  function parseDocument(text, mime, wanted) {
    const essence = String(mime).split(";")[0].trim().toLowerCase();
    if (essence === "text/html") {
      return wanted ? new Parser().parseFromString(text, "text/html") : null;
    }
    const xml = essence === "" || essence === "text/xml" || essence === "application/xml"
      || essence.endsWith("+xml");
    if (!xml) return null;
    const type = ["text/xml", "application/xhtml+xml", "image/svg+xml"].includes(essence)
      ? essence : "application/xml";
    const doc = new Parser().parseFromString(text, type);
    const top = doc.documentElement;
    return top !== null && top.namespaceURI === PARSER_ERROR ? null : doc;
  }
  const UNSENT = 0;
  const OPENED = 1;
  const HEADERS_RECEIVED = 2;
  const LOADING = 3;
  const DONE = 4;

  function invalidState(message) {
    return new Error("InvalidStateError : " + message);
  }

  class XMLHttpRequest extends EventTargetBase {
    #method = "GET";
    #url = "";
    #headers = [];
    #sent = false;
    #responseHeaders = [];
    #timer = 0;
    // Type MIME imposé par `overrideMimeType`.
    #mime = null;
    // Change à chaque `open()` ou `abort()` : une réponse d'une requête
    // précédente est alors ignorée.
    #generation = 0;

    readyState = UNSENT;
    status = 0;
    statusText = "";
    responseText = "";
    response = "";
    responseXML = null;
    responseType = "";
    responseURL = "";
    timeout = 0;
    withCredentials = false;
    upload = new EventTargetBase();

    open(method, url, async = true) {
      if (arguments.length < 2) {
        throw new TypeError("XMLHttpRequest.open : la méthode et l'adresse sont requises");
      }
      if (async === false) {
        throw new Error("InvalidAccessError : les requêtes synchrones ne sont pas prises en charge");
      }
      this.#generation++;
      stopTimer(this.#timer);
      this.#method = String(method);
      this.#url = String(url);
      this.#headers = [];
      this.#sent = false;
      this.#responseHeaders = [];
      this.status = 0;
      this.statusText = "";
      this.responseText = "";
      this.response = "";
      this.responseXML = null;
      this.responseURL = "";
      this.#state(OPENED);
    }

    setRequestHeader(name, value) {
      if (this.readyState !== OPENED || this.#sent) {
        throw invalidState("setRequestHeader exige une requête ouverte et non envoyée");
      }
      this.#headers.push([String(name), String(value)]);
    }

    send(body = null) {
      if (this.readyState !== OPENED || this.#sent) {
        throw invalidState("send exige une requête ouverte et non envoyée");
      }
      this.#sent = true;
      const generation = this.#generation;
      const current = () => generation === this.#generation;
      const init = { method: this.#method, headers: this.#headers };
      const bodiless = /^(GET|HEAD)$/i.test(this.#method);
      if (body !== null && body !== undefined && !bodiless) init.body = body;
      this.#fire("loadstart");
      if (!current()) return;
      if (this.timeout > 0) {
        this.#timer = startTimer(() => {
          if (current() && this.readyState !== DONE) {
            this.#generation++;
            this.#finish("timeout");
          }
        }, this.timeout);
      }
      fetch(this.#url, init)
        .then((response) => {
          if (!current()) return undefined;
          this.status = response.status;
          this.statusText = response.statusText;
          this.responseURL = response.url;
          const headers = [];
          response.headers.forEach((value, name) => headers.push([name, value]));
          this.#responseHeaders = headers;
          this.#state(HEADERS_RECEIVED);
          const kind = this.responseType;
          if (kind === "arraybuffer" || kind === "blob") {
            const body = kind === "blob" ? response.blob() : response.arrayBuffer();
            return body.then((value) => {
              if (!current()) return;
              this.#state(LOADING);
              if (!current()) return;
              this.response = value;
              this.#finish("load");
            });
          }
          const mime = this.#mime ?? response.headers.get("content-type") ?? "";
          return response.text().then((text) => {
            if (!current()) return;
            this.#state(LOADING);
            if (!current()) return;
            this.responseText = text;
            if (kind === "json") {
              try {
                this.response = parseJSON(text);
              } catch {
                this.response = null;
              }
            } else if (kind === "document") {
              this.responseXML = parseDocument(text, mime, true);
              this.response = this.responseXML;
            } else {
              this.response = text;
              this.responseXML = parseDocument(text, mime, false);
            }
            this.#finish("load");
          });
        })
        .catch(() => {
          if (current()) this.#finish("error");
        });
    }

    abort() {
      this.#generation++;
      stopTimer(this.#timer);
      if (this.#sent && this.readyState !== DONE) {
        this.#sent = false;
        this.status = 0;
        this.statusText = "";
        this.#state(DONE);
        this.#fire("abort");
        this.#fire("loadend");
      }
      // Retour à l'état initial, sans événement.
      this.readyState = UNSENT;
    }

    getResponseHeader(name) {
      const wanted = String(name).toLowerCase();
      const values = [];
      for (const [header, value] of this.#responseHeaders) {
        if (header.toLowerCase() === wanted) values.push(value);
      }
      return values.length > 0 ? values.join(", ") : null;
    }

    getAllResponseHeaders() {
      let text = "";
      for (const [header, value] of this.#responseHeaders) {
        text += header + ": " + value + "\r\n";
      }
      return text;
    }

    overrideMimeType(mime) {
      if (this.readyState >= LOADING) {
        throw invalidState("overrideMimeType exige une réponse pas encore reçue");
      }
      this.#mime = String(mime);
    }

    #state(state) {
      this.readyState = state;
      this.#fire("readystatechange");
    }

    #finish(kind) {
      stopTimer(this.#timer);
      if (kind !== "load") {
        this.status = 0;
        this.statusText = "";
      }
      this.#sent = false;
      this.#state(DONE);
      this.#fire(kind);
      this.#fire("loadend");
    }

    #fire(type) {
      this.dispatchEvent(new EventCtor(type));
    }
  }

  const states = { UNSENT, OPENED, HEADERS_RECEIVED, LOADING, DONE };
  for (const name of Object.keys(states)) {
    const descriptor = { value: states[name], enumerable: true };
    Object.defineProperty(XMLHttpRequest, name, descriptor);
    Object.defineProperty(XMLHttpRequest.prototype, name, descriptor);
  }
  return XMLHttpRequest;
})
