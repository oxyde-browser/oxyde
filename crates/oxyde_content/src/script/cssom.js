// CSSOM et CSS Font Loading, côté script : l'objet `CSS`, `document.fonts`
// et `FontFace`. Le script rend une fabrique appelée avec
//   cssSupports(propriété, valeur) -> vrai si le moteur de style accepte la
//   déclaration.
//
// LIMITES CONNUES : les polices d'une page sont chargées avant ses scripts,
// donc `document.fonts.ready` est déjà tenue et `status` vaut « loaded » ;
// une police ajoutée par script (`new FontFace(…)`, `document.fonts.add`)
// n'est pas chargée ; `CSS.supports` ne connaît ni `selector()` ni les
// combinaisons `and`, `or`, `not`.
(function (cssSupports) {
  "use strict";
  const global = globalThis;
  const define = Object.defineProperty;
  const expose = (name, value) => {
    define(global, name, { value, writable: true, enumerable: false, configurable: true });
  };

  // ---- Événements d'animation et de transition -----------------------------------------------
  class AnimationEvent extends global.Event {
    #name; #elapsed; #pseudo;
    constructor(type, init = {}) {
      super(type, init);
      this.#name = String(init?.animationName ?? "");
      this.#elapsed = Number(init?.elapsedTime ?? 0);
      this.#pseudo = String(init?.pseudoElement ?? "");
    }
    get animationName() { return this.#name; }
    get elapsedTime() { return this.#elapsed; }
    get pseudoElement() { return this.#pseudo; }
  }
  class TransitionEvent extends global.Event {
    #name; #elapsed; #pseudo;
    constructor(type, init = {}) {
      super(type, init);
      this.#name = String(init?.propertyName ?? "");
      this.#elapsed = Number(init?.elapsedTime ?? 0);
      this.#pseudo = String(init?.pseudoElement ?? "");
    }
    get propertyName() { return this.#name; }
    get elapsedTime() { return this.#elapsed; }
    get pseudoElement() { return this.#pseudo; }
  }
  for (const constructor of [AnimationEvent, TransitionEvent]) {
    define(constructor.prototype, Symbol.toStringTag, {
      value: constructor.name, configurable: true,
    });
    expose(constructor.name, constructor);
  }

  // ---- CSS -------------------------------------------------------------------------------
  // `CSS.escape` (CSSOM §2.1) : rend un identifiant utilisable dans un sélecteur.
  function escape(value) {
    const text = String(value);
    let out = "";
    for (let i = 0; i < text.length; i++) {
      const unit = text.charCodeAt(i);
      const char = text.charAt(i);
      if (unit === 0) {
        out += "�";
      } else if ((unit >= 1 && unit <= 0x1F) || unit === 0x7F
        || (i === 0 && unit >= 0x30 && unit <= 0x39)
        || (i === 1 && unit >= 0x30 && unit <= 0x39 && text.charCodeAt(0) === 0x2D)) {
        out += "\\" + unit.toString(16) + " ";
      } else if (i === 0 && text.length === 1 && unit === 0x2D) {
        out += "\\" + char;
      } else if (unit >= 0x80 || unit === 0x2D || unit === 0x5F
        || (unit >= 0x30 && unit <= 0x39) || (unit >= 0x41 && unit <= 0x5A)
        || (unit >= 0x61 && unit <= 0x7A)) {
        out += char;
      } else {
        out += "\\" + char;
      }
    }
    return out;
  }
  function supports(property, value) {
    if (arguments.length >= 2) return cssSupports(String(property), String(value));
    // Forme à un argument : « (propriété: valeur) ».
    const text = String(property).trim().replace(/^\((.*)\)$/s, "$1");
    const cut = text.indexOf(":");
    if (cut < 0) return false;
    return cssSupports(text.slice(0, cut).trim(), text.slice(cut + 1).trim());
  }
  const CSS = {};
  define(CSS, Symbol.toStringTag, { value: "CSS", configurable: true });
  for (const [name, value] of [["escape", escape], ["supports", supports]]) {
    define(CSS, name, { value, writable: true, enumerable: true, configurable: true });
  }
  for (const unit of ["px", "em", "rem", "vw", "vh", "percent", "number", "deg", "s", "ms"]) {
    define(CSS, unit, {
      value: (n) => ({ value: Number(n), unit, toString() { return n + (unit === "percent" ? "%" : unit === "number" ? "" : unit); } }),
      writable: true, enumerable: true, configurable: true,
    });
  }
  expose("CSS", CSS);

  // ---- FontFace et document.fonts ----------------------------------------------------------
  class FontFace {
    #family; #source; #status = "unloaded"; #loaded;
    constructor(family, source, descriptors = {}) {
      if (arguments.length < 2) throw new TypeError("FontFace attend deux arguments");
      this.#family = String(family);
      this.#source = source;
      this.style = descriptors.style ?? "normal";
      this.weight = descriptors.weight ?? "normal";
      this.stretch = descriptors.stretch ?? "normal";
      this.unicodeRange = descriptors.unicodeRange ?? "U+0-10FFFF";
      this.display = descriptors.display ?? "auto";
      this.#loaded = new Promise(() => {});
    }
    get family() { return this.#family; }
    set family(value) { this.#family = String(value); }
    get status() { return this.#status; }
    get loaded() { return this.#loaded; }
    load() {
      // Une police ajoutée par script n'est pas chargée (voir les limites).
      this.#status = "error";
      this.#loaded = Promise.reject(new global.DOMException(
        "le chargement d'une police par script n'est pas pris en charge", "NetworkError"));
      this.#loaded.catch(() => {});
      return this.#loaded;
    }
  }
  define(FontFace.prototype, Symbol.toStringTag, { value: "FontFace", configurable: true });
  expose("FontFace", FontFace);

  const faces = new Set();
  const fonts = new global.EventTarget();
  const method = (name, value) => {
    define(fonts, name, { value, writable: true, enumerable: false, configurable: true });
  };
  define(fonts, "ready", { get() { return ready; }, enumerable: true, configurable: true });
  define(fonts, "status", { get() { return "loaded"; }, enumerable: true, configurable: true });
  define(fonts, "size", { get() { return faces.size; }, enumerable: true, configurable: true });
  method("add", (face) => { faces.add(face); return fonts; });
  method("delete", (face) => faces.delete(face));
  method("clear", () => { faces.clear(); });
  method("has", (face) => faces.has(face));
  method("forEach", (callback, receiver) => {
    for (const face of faces) callback.call(receiver, face, face, fonts);
  });
  method("values", () => faces.values());
  method("keys", () => faces.values());
  method("entries", () => faces.entries());
  method(Symbol.iterator, () => faces.values());
  method("check", () => true);
  method("load", () => Promise.resolve([]));
  define(fonts, Symbol.toStringTag, { value: "FontFaceSet", configurable: true });
  const ready = Promise.resolve(fonts);
  define(global.Document.prototype, "fonts", {
    get() { return fonts; }, enumerable: true, configurable: true,
  });
})
