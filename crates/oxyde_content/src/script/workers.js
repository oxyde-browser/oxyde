// `Worker` (HTML Standard §10.2), côté créateur. Le worker s'exécute dans un
// royaume JavaScript à lui, que le moteur crée et que `worker_scope.js`
// équipe ; les messages sont clonés d'un royaume à l'autre.
//
// Fonction appelée par le moteur avec :
//   createWorker(adresse, nom, versCréateur, erreurVersCréateur)
//     -> `{ run, deliver, close }` (voir `worker_scope.js`), ou null si la
//        machine ne peut plus créer de royaume.
//
// LIMITES CONNUES : voir `worker_scope.js` ; pas de worker de type module
// (`{ type: "module" }` : l'événement `error` part) ; pas de `SharedWorker`
// ni de service worker ; `terminate()` arrête la remise des messages mais
// pas une tâche en cours, et le royaume n'est pas libéré.
(function (createWorker) {
  "use strict";
  const global = globalThis;
  const define = Object.defineProperty;
  const EventTargetBase = global.EventTarget;
  const later = (task) => global.setTimeout(task, 0);
  const fetch = global.fetch;
  // Nombre de scripts lus d'avance pour `importScripts`, et profondeur.
  const MAX_IMPORTS = 32;
  const MAX_IMPORT_DEPTH = 4;

  // Adresses écrites en toutes lettres dans les appels `importScripts(…)` de
  // `source`, résolues contre `base`.
  function importedAddresses(source, base) {
    const found = [];
    const calls = /importScripts\s*\(([^)]*)\)/g;
    for (let call = calls.exec(source); call !== null; call = calls.exec(source)) {
      const literals = /(["'`])((?:\\.|(?!\1)[^\\\n])*)\1/g;
      for (let item = literals.exec(call[1]); item !== null; item = literals.exec(call[1])) {
        try {
          found.push(new global.URL(item[2], base).href);
        } catch {
          // Adresse invalide : `importScripts` le dira.
        }
      }
    }
    return found;
  }

  // Lit le script `address` et, de proche en proche, ceux qu'il importe.
  function load(address) {
    const imports = new Map();
    const read = (target) => fetch(target).then((response) => {
      if (!response.ok) throw new TypeError("script de worker indisponible");
      return response.text();
    });
    const follow = (source, base, depth) => {
      if (depth >= MAX_IMPORT_DEPTH) return Promise.resolve();
      const wanted = importedAddresses(source, base).filter((target) => !imports.has(target));
      return Promise.all(wanted.map((target) => {
        if (imports.size >= MAX_IMPORTS || imports.has(target)) return undefined;
        // Réservé tout de suite : une adresse n'est lue qu'une fois.
        imports.set(target, undefined);
        return read(target).then(
          (text) => {
            imports.set(target, text);
            return follow(text, target, depth + 1);
          },
          () => {
            imports.delete(target);
          },
        );
      }));
    };
    return read(address).then((source) => follow(source, address, 0).then(() => {
      for (const [target, text] of [...imports]) {
        if (text === undefined) imports.delete(target);
      }
      return { source, imports };
    }));
  }

  const portsOf = (options) => {
    const list = Array.isArray(options) ? options : options?.transfer;
    return list === undefined || list === null ? [] : Array.from(list);
  };

  // Seuls les ports changent de mains d'un royaume à l'autre : un tampon
  // transféré est déjà passé dans la copie.
  const isPort = (value) => Object.prototype.toString.call(value) === "[object MessagePort]";

  class Worker extends EventTargetBase {
    #control = null;
    #pending = [];
    #terminated = false;
    constructor(scriptURL, options = undefined) {
      super();
      if (arguments.length < 1) throw new TypeError("Worker : l'adresse du script est requise");
      this.onmessage = null;
      this.onmessageerror = null;
      this.onerror = null;
      let address;
      try {
        const base = global.document?.baseURI ?? global.location.href;
        address = new global.URL(String(scriptURL), base);
      } catch {
        throw new global.DOMException("Worker : adresse invalide", "SyntaxError");
      }
      const local = address.protocol === "blob:" || address.protocol === "data:";
      if (!local && address.origin !== global.location.origin) {
        throw new global.DOMException("Worker : le script doit être de même origine", "SecurityError");
      }
      const fail = () => later(() => {
        if (!this.#terminated) this.dispatchEvent(new global.Event("error", { cancelable: true }));
      });
      if (options !== undefined && options !== null && options.type === "module") {
        fail();
        return;
      }
      const control = createWorker(
        address.href,
        String(options?.name ?? ""),
        (message, ports) => this.#receive(message, ports),
        (message, filename, lineno) => this.#report(message, filename, lineno),
      );
      if (control === null) {
        fail();
        return;
      }
      load(address.href).then(
        ({ source, imports }) => {
          if (this.#terminated) return;
          control.run(source, imports);
          this.#control = control;
          for (const [message, ports] of this.#pending.splice(0)) control.deliver(message, ports);
        },
        fail,
      );
    }
    // Message venu du worker : cloné dans ce royaume, remis dans une tâche.
    #receive(message, ports) {
      if (this.#terminated) return;
      const data = global.structuredClone(message, { transfer: ports });
      const moved = ports.filter(isPort);
      later(() => {
        if (!this.#terminated) {
          this.dispatchEvent(new global.MessageEvent("message", { data, ports: moved }));
        }
      });
    }
    #report(message, filename, lineno) {
      later(() => {
        if (this.#terminated) return;
        this.dispatchEvent(new global.ErrorEvent("error", { message, filename, lineno, cancelable: true }));
      });
    }
    postMessage(message, options = undefined) {
      if (arguments.length < 1) throw new TypeError("postMessage : un message est requis");
      const ports = portsOf(options);
      // Cloné tout de suite : une valeur non clonable lève ici, et le
      // worker reçoit la valeur telle qu'elle était à l'envoi.
      const copy = global.structuredClone(message, { transfer: ports });
      if (this.#terminated) return;
      const moved = ports.filter(isPort);
      if (this.#control === null) this.#pending.push([copy, moved]);
      else this.#control.deliver(copy, moved);
    }
    terminate() {
      this.#terminated = true;
      this.#pending.length = 0;
      this.#control?.close();
    }
  }
  define(Worker.prototype, Symbol.toStringTag, { value: "Worker", configurable: true });
  define(global, "Worker", { value: Worker, writable: true, enumerable: false, configurable: true });
})
