// Côté worker de `Worker` : ce script s'exécute dans le royaume neuf du
// worker, déjà équipé comme une fenêtre (`fetch`, minuteurs, `URL`,
// `TextDecoder`, `EventTarget`…). Il en retire ce qui n'existe pas dans un
// worker (`window`, `document`…), y pose `postMessage`, `close`,
// `importScripts` et les interfaces `WorkerGlobalScope`, puis rend au
// royaume créateur de quoi le piloter.
//
// Fonction appelée par le moteur avec :
//   name                         -> nom du worker (`self.name`) ;
//   toParent(message, ports)     -> remet un message à l'objet `Worker` ;
//   toParentError(message, fichier, ligne) -> y signale une erreur non rattrapée ;
//   runScript(source)            -> exécute `source` comme un script classique de ce royaume.
// Elle rend `{ run(source, imports), deliver(message, ports), close() }`.
//
// LIMITES CONNUES : le worker partage le fil d'exécution de la page (un
// worker qui calcule longtemps la fige, dans la limite du budget d'une
// tâche) ; `importScripts` ne rend que les scripts que le créateur a pu lire
// d'avance dans le source (adresses écrites en toutes lettres) ; l'objet
// global garde les membres d'une fenêtre qui ne touchent pas au document
// (`innerWidth`, `scrollTo`…) ; `self instanceof WorkerGlobalScope` est vrai
// mais le prototype de l'objet global reste celui d'une fenêtre.
(function (name, toParent, toParentError, runScript) {
  "use strict";
  const global = globalThis;
  const define = Object.defineProperty;
  const clone = global.structuredClone;
  const later = (task) => global.setTimeout(task, 0);
  const MessageEvent = global.MessageEvent;
  const DOMException = global.DOMException;
  const URL = global.URL;
  let closed = false;
  // Scripts lus d'avance par le créateur, par adresse.
  const imported = new Map();

  // Ce qu'une fenêtre a et qu'un worker n'a pas.
  for (const key of ["window", "document", "parent", "top", "frames", "frameElement", "opener",
    "alert", "confirm", "prompt", "print", "localStorage", "sessionStorage", "history",
    "customElements", "getComputedStyle", "getSelection", "matchMedia", "requestAnimationFrame",
    "cancelAnimationFrame", "Worker"]) {
    try {
      if (!delete global[key]) throw new TypeError("non supprimable");
    } catch {
      try {
        define(global, key, { value: undefined, writable: true, configurable: true });
      } catch {
        // Membre figé : il reste.
      }
    }
  }

  const expose = (key, value) => {
    define(global, key, { value, writable: true, enumerable: false, configurable: true });
  };
  // Interfaces : l'objet global en est l'unique instance.
  const scopeInterface = (label) => {
    const ctor = function () {
      throw new TypeError("constructeur illégal");
    };
    define(ctor, "name", { value: label });
    define(ctor, Symbol.hasInstance, { value: (value) => value === global });
    expose(label, ctor);
  };
  scopeInterface("WorkerGlobalScope");
  scopeInterface("DedicatedWorkerGlobalScope");
  define(global, Symbol.toStringTag, { value: "DedicatedWorkerGlobalScope", configurable: true });

  expose("name", String(name));
  expose("onmessage", null);
  expose("onmessageerror", null);
  const portsOf = (options) => {
    const list = Array.isArray(options) ? options : options?.transfer;
    return list === undefined || list === null ? [] : Array.from(list);
  };
  expose("postMessage", function postMessage(message, options = undefined) {
    if (arguments.length < 1) throw new TypeError("postMessage : un message est requis");
    if (!closed) toParent(message, portsOf(options));
  });
  expose("close", function close() {
    closed = true;
  });
  expose("importScripts", function importScripts(...urls) {
    for (const url of urls) {
      let address;
      try {
        address = new URL(String(url), global.location.href).href;
      } catch {
        throw new DOMException("importScripts : adresse invalide", "SyntaxError");
      }
      const source = imported.get(address);
      if (source === undefined) {
        throw new DOMException("importScripts : « " + address + " » n'a pas pu être chargé", "NetworkError");
      }
      runScript(source);
    }
  });

  // Une erreur que le worker ne rattrape pas remonte à l'objet `Worker`.
  global.addEventListener("error", (event) => {
    global.queueMicrotask(() => {
      if (!event.defaultPrevented) {
        toParentError(String(event.message), String(event.filename ?? ""), Number(event.lineno) || 0);
      }
    });
  });

  return {
    run(source, imports) {
      for (const [address, text] of imports) imported.set(address, text);
      try {
        runScript(source);
      } catch (thrown) {
        global.reportError(thrown);
      }
    },
    deliver(message, ports) {
      if (closed) return;
      const data = clone(message, { transfer: ports });
      later(() => {
        if (!closed) global.dispatchEvent(new MessageEvent("message", { data, ports }));
      });
    },
    close() {
      closed = true;
    },
  };
})
