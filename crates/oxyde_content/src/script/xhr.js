// `XMLHttpRequest` (https://xhr.spec.whatwg.org/), écrit en JavaScript
// par-dessus `fetch` et `EventTarget`. Le script rend une fabrique, appelée
// avec le `fetch` d'origine : une page qui remplace `window.fetch` ne change
// pas le comportement de `XMLHttpRequest`.
//
// LIMITES CONNUES : pas de requête synchrone ; les types de réponse
// `arraybuffer`, `blob` et `document` rendent le texte ; pas d'événements de
// progression intermédiaires ; `withCredentials` est sans effet (pas de
// cookies).
(function (fetch) {
  "use strict";
  const EventTargetBase = EventTarget;
  const EventCtor = Event;
  const startTimer = setTimeout;
  const stopTimer = clearTimeout;
  const parseJSON = JSON.parse;
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
    // Change à chaque `open()` ou `abort()` : une réponse d'une requête
    // précédente est alors ignorée.
    #generation = 0;

    readyState = UNSENT;
    status = 0;
    statusText = "";
    responseText = "";
    response = "";
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
          return response.text().then((text) => {
            if (!current()) return;
            this.#state(LOADING);
            if (!current()) return;
            this.responseText = text;
            if (this.responseType === "json") {
              try {
                this.response = parseJSON(text);
              } catch {
                this.response = null;
              }
            } else {
              this.response = text;
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

    overrideMimeType() {}

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
