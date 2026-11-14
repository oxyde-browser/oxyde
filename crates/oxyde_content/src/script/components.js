// Éléments personnalisés, `DOMParser` et contenu des gabarits.
//
// Fonction appelée par le moteur avec :
//   parseHtmlDocument(texte) -> nouveau document HTML analysé depuis `texte` ;
//   attachShadowRoot(hôte, ouvert) -> racine du nouvel arbre fantôme, ou null ;
//   shadowRootOf(élément)       -> sa racine fantôme (ouverte ou non), ou null ;
//   shadowHostOf(nœud)          -> l'hôte dont il est la racine fantôme, ou null ;
//   assignedSlotOf(nœud)        -> l'emplacement où il est distribué, ou null ;
//   assignedNodesOf(emplacement) -> tableau des nœuds qui y sont distribués ;
//   deepFocused()               -> l'élément qui a le focus, fût-il dans un arbre fantôme.
//
// LIMITES CONNUES
// - `Path2D` enregistre ses commandes ; rien ne les dessine (pas de canevas).
// - Éléments personnalisés : les rappels `connectedCallback`,
//   `disconnectedCallback` et `attributeChangedCallback` partent à la fin de
//   la tâche (par un observateur de mutations), pas au moment même du
//   changement ; un élément créé par l'analyseur ou par `innerHTML` est mis à
//   niveau à ce moment-là aussi ; pas d'éléments natifs personnalisés
//   (`extends`, attribut `is`), pas de `adoptedCallback`, pas
//   d'`ElementInternals`.
// - Shadow DOM : pas de `adoptedStyleSheets` (les bibliothèques se replient
//   sur des éléments `<style>`), pas d'arbre fantôme déclaratif
//   (`<template shadowrootmode>`), pas de `slot.assign()` ; `innerHTML`
//   d'une racine fantôme analyse son texte comme le contenu d'un `<div>` ;
//   `slotchange` ne part que pour un hôte relié au document, à la fin de
//   la tâche ; `delegatesFocus` est retenu sans effet ; les rappels d'un
//   élément personnalisé d'un arbre fantôme partent dans l'ordre des
//   mutations, parfois avant ceux de son hôte.
// - `DOMParser` : l'analyseur XML est réduit (pas de DTD, pas d'entités
//   déclarées) ; un document XML mal formé rend un document `parsererror`.
// - `template.content` : le fragment appartient au document de la page.
(function (parseHtmlDocument, attachShadowRoot, shadowRootOf, shadowHostOf, assignedSlotOf,
  assignedNodesOf, deepFocused) {
  "use strict";
  const global = globalThis;
  const define = Object.defineProperty;
  const expose = (name, value) => {
    define(global, name, { value, writable: true, enumerable: false, configurable: true });
  };

  const tag = (ctor, name) => {
    define(ctor.prototype, Symbol.toStringTag, { value: name, configurable: true });
  };

  // ---- Interfaces que les pages nomment sans s'en servir --------------------------------------------------
  // Des bibliothèques lisent `Window.prototype`, `SVGElement.prototype`… dès
  // leur chargement : l'interface doit exister, même réduite.
  const illegal = (name) => ({ [name]: function () { throw new TypeError("constructeur illégal"); } }[name]);
  // Interface d'un objet unique (`window`, `navigator`…) : son prototype
  // s'intercale devant celui de l'objet.
  const singleton = (name, instance) => {
    if (name in global || instance === undefined || instance === null) return;
    const Interface = illegal(name);
    try {
      const prototype = Object.create(Object.getPrototypeOf(instance));
      Object.setPrototypeOf(instance, prototype);
      Interface.prototype = prototype;
      define(prototype, "constructor", { value: Interface, writable: true, configurable: true });
    } catch {
      Interface.prototype = Object.getPrototypeOf(instance);
    }
    tag(Interface, name);
    expose(name, Interface);
  };
  // La fenêtre est une cible d'événements : ses méthodes d'écoute se lisent
  // aussi sur `EventTarget.prototype`, où des bibliothèques vont les chercher.
  if (typeof global.EventTarget?.prototype?.addEventListener === "function"
    && Object.getPrototypeOf(global) === Object.prototype) {
    try {
      Object.setPrototypeOf(global, global.EventTarget.prototype);
    } catch {
      // L'objet global garde alors son prototype.
    }
  }
  singleton("Window", global);
  // Accès par nom et par rang dans les collections.
  const collection = global.HTMLCollection?.prototype;
  if (collection !== undefined && typeof collection.namedItem !== "function") {
    define(collection, "namedItem", {
      value: function namedItem(name) {
        const wanted = String(name);
        if (wanted === "") return null;
        for (const element of Array.from(this)) {
          if (element.id === wanted || element.getAttribute("name") === wanted) return element;
        }
        return null;
      },
      writable: true, enumerable: true, configurable: true,
    });
  }
  singleton("Navigator", global.navigator);
  singleton("Performance", global.performance);
  singleton("Screen", global.screen);
  singleton("History", global.history);
  singleton("Location", global.location);
  singleton("Crypto", global.crypto);
  singleton("VisualViewport", global.visualViewport);
  singleton("BarProp", global.locationbar);
  // Interfaces dérivées d'une interface connue.
  const derived = (name, Parent, belongs) => {
    if (name in global || typeof Parent !== "function") return;
    const Interface = illegal(name);
    Interface.prototype = Object.create(Parent.prototype, {
      constructor: { value: Interface, writable: true, configurable: true },
    });
    Object.setPrototypeOf(Interface, Parent);
    if (belongs !== undefined) {
      define(Interface, Symbol.hasInstance, { value: (item) => belongs(item), configurable: true });
    }
    tag(Interface, name);
    expose(name, Interface);
  };
  const SVG_NS = "http://www.w3.org/2000/svg";
  const isSvg = (item) => item instanceof global.Element && item.namespaceURI === SVG_NS;
  derived("SVGElement", global.Element, isSvg);
  derived("SVGGraphicsElement", global.SVGElement, isSvg);
  derived("SVGSVGElement", global.SVGGraphicsElement, (item) => isSvg(item) && item.localName === "svg");
  derived("HTMLUnknownElement", global.HTMLElement, () => false);
  if (!("HTMLDocument" in global)) expose("HTMLDocument", global.Document);
  for (const name of ["MutationRecord", "DOMStringMap", "ValidityState", "FileList", "StyleSheet"]) {
    if (!(name in global)) {
      const Interface = illegal(name);
      tag(Interface, name);
      expose(name, Interface);
    }
  }
  // Tracé enregistré, sans dessin : le canevas n'existe pas encore.
  // LIMITE : `Path2D` garde ses commandes mais rien ne les dessine.
  class Path2D {
    #commands = [];
    constructor(path) {
      if (path instanceof Path2D) this.#commands = path.#commands.slice();
      else if (typeof path === "string") this.#commands.push(["svg", path]);
    }
    addPath(path) {
      if (!(path instanceof Path2D)) throw new TypeError("Path2D.addPath : un tracé est attendu");
      this.#commands.push(...path.#commands);
    }
    static {
      for (const name of ["closePath", "moveTo", "lineTo", "bezierCurveTo", "quadraticCurveTo", "arc",
        "arcTo", "ellipse", "rect", "roundRect"]) {
        define(Path2D.prototype, name, {
          value: { [name](...args) { this.#commands.push([name, args]); } }[name],
          writable: true, enumerable: true, configurable: true,
        });
      }
    }
  }
  tag(Path2D, "Path2D");
  expose("Path2D", Path2D);

  // ---- Élément sous un point ------------------------------------------------------------------------------
  // LIMITE : le dernier élément du document dont la boîte contient le point,
  // sans tenir compte de l'ordre d'empilement (`z-index`) ni de
  // `pointer-events`.
  function elementsAt(doc, x, y) {
    const [px, py] = [Number(x), Number(y)];
    if (!Number.isFinite(px) || !Number.isFinite(py)) {
      throw new TypeError("elementFromPoint : deux coordonnées finies sont attendues");
    }
    if (px < 0 || py < 0 || px > global.innerWidth || py > global.innerHeight) return [];
    const found = [];
    for (const element of doc.querySelectorAll("*")) {
      const box = element.getBoundingClientRect();
      if (box.width > 0 && box.height > 0 && px >= box.left && px < box.right
        && py >= box.top && py < box.bottom) found.push(element);
    }
    return found.reverse();
  }
  for (const [name, pick] of [["elementFromPoint", (list) => list[0] ?? null], ["elementsFromPoint", (list) => list]]) {
    if (typeof global.Document.prototype[name] !== "function") {
      define(global.Document.prototype, name, {
        value: { [name](x, y) { return pick(elementsAt(this, x, y)); } }[name],
        writable: true, enumerable: true, configurable: true,
      });
    }
  }

  // ---- template.content ------------------------------------------------------------------------------------
  // Le contenu d'un gabarit vit dans un fragment à part : il n'est ni
  // affiché ni trouvé par les recherches dans le document.
  const contents = new WeakMap();
  define(global.HTMLTemplateElement.prototype, "content", {
    get() {
      let fragment = contents.get(this);
      if (fragment === undefined) {
        fragment = this.ownerDocument.createDocumentFragment();
        contents.set(this, fragment);
      }
      // Des enfants posés depuis (analyseur, `innerHTML`) remplacent le contenu.
      if (this.firstChild !== null) {
        fragment.replaceChildren(...Array.from(this.childNodes));
      }
      return fragment;
    },
    enumerable: true,
    configurable: true,
  });

  // ---- DOMParser ---------------------------------------------------------------------------------------------
  const XMLNS = "http://www.w3.org/2000/xmlns/";
  const XML = "http://www.w3.org/XML/1998/namespace";
  const ENTITIES = { lt: "<", gt: ">", amp: "&", quot: "\"", apos: "'" };
  const decode = (text) => text.replace(/&(#x[0-9a-fA-F]+|#\d+|\w+);/g, (whole, name) => {
    if (name[0] === "#") {
      const code = name[1] === "x" ? parseInt(name.slice(2), 16) : parseInt(name.slice(1), 10);
      return code > 0 && code <= 0x10ffff ? String.fromCodePoint(code) : whole;
    }
    if (!(name in ENTITIES)) throw new Error("entité inconnue : " + whole);
    return ENTITIES[name];
  });
  function parseXml(source, doc) {
    let at = 0;
    const text = source.replace(/^﻿/, "");
    const fail = (message) => { throw new Error(message + " (position " + at + ")"); };
    // Pile des éléments ouverts, avec leurs déclarations d'espaces de noms.
    const open = [{ node: doc, scope: { xml: XML, xmlns: XMLNS } }];
    const top = () => open[open.length - 1];
    const skip = (end, what) => {
      const stop = text.indexOf(end, at);
      if (stop < 0) fail(what + " non terminé");
      const inner = text.slice(at, stop);
      at = stop + end.length;
      return inner;
    };
    const NAME = /[^\s<>\/=!?"'&]+/y;
    const name = () => {
      NAME.lastIndex = at;
      const found = NAME.exec(text);
      if (found === null) fail("nom attendu");
      at = NAME.lastIndex;
      return found[0];
    };
    const space = () => {
      while (at < text.length && /\s/.test(text[at])) at++;
    };
    let rooted = false;
    while (at < text.length) {
      if (text.startsWith("<!--", at)) {
        at += 4;
        top().node.appendChild(doc.createComment(skip("-->", "commentaire")));
      } else if (text.startsWith("<![CDATA[", at)) {
        at += 9;
        if (open.length === 1) fail("section CDATA hors de l'élément racine");
        top().node.appendChild(doc.createCDATASection(skip("]]>", "section CDATA")));
      } else if (text.startsWith("<?", at)) {
        at += 2;
        const target = name();
        const data = skip("?>", "instruction de traitement").trim();
        if (target.toLowerCase() !== "xml") {
          top().node.appendChild(doc.createProcessingInstruction(target, data));
        }
      } else if (text.startsWith("<!", at)) {
        // DOCTYPE : sauté, sous-ensemble interne compris.
        let depth = 0;
        for (; at < text.length; at++) {
          if (text[at] === "[") depth++;
          else if (text[at] === "]") depth--;
          else if (text[at] === ">" && depth <= 0) break;
        }
        if (at >= text.length) fail("déclaration non terminée");
        at++;
      } else if (text.startsWith("</", at)) {
        at += 2;
        const closing = name();
        space();
        if (text[at] !== ">") fail("« > » attendu");
        at++;
        if (open.length === 1 || top().qualified !== closing) fail("balise fermante inattendue : " + closing);
        open.pop();
      } else if (text[at] === "<") {
        at++;
        if (open.length === 1 && rooted) fail("un seul élément racine est permis");
        const qualified = name();
        const attributes = [];
        const scope = Object.create(top().scope);
        for (;;) {
          space();
          if (text[at] === ">" || text.startsWith("/>", at)) break;
          const key = name();
          space();
          if (text[at] !== "=") fail("« = » attendu après l'attribut " + key);
          at++;
          space();
          const quote = text[at];
          if (quote !== "\"" && quote !== "'") fail("valeur d'attribut entre guillemets attendue");
          at++;
          const value = decode(skip(quote, "valeur d'attribut"));
          if (attributes.some(([other]) => other === key)) fail("attribut répété : " + key);
          attributes.push([key, value]);
          if (key === "xmlns") scope[""] = value;
          else if (key.startsWith("xmlns:")) scope[key.slice(6)] = value;
        }
        const empty = text[at] === "/";
        at += empty ? 2 : 1;
        const prefix = qualified.includes(":") ? qualified.split(":")[0] : "";
        const namespace = scope[prefix];
        if (prefix !== "" && namespace === undefined) fail("préfixe non déclaré : " + prefix);
        const element = doc.createElementNS(namespace === undefined || namespace === "" ? null : namespace, qualified);
        for (const [key, value] of attributes) {
          const colon = key.indexOf(":");
          if (key === "xmlns" || key.startsWith("xmlns:")) element.setAttributeNS(XMLNS, key, value);
          else if (colon < 0) element.setAttribute(key, value);
          else {
            const bound = scope[key.slice(0, colon)];
            if (bound === undefined) fail("préfixe non déclaré : " + key.slice(0, colon));
            element.setAttributeNS(bound, key, value);
          }
        }
        top().node.appendChild(element);
        rooted = true;
        if (!empty) open.push({ node: element, scope, qualified });
      } else {
        const stop = text.indexOf("<", at);
        const raw = text.slice(at, stop < 0 ? text.length : stop);
        at = stop < 0 ? text.length : stop;
        if (open.length === 1) {
          if (raw.trim() !== "") fail("texte hors de l'élément racine");
        } else {
          top().node.appendChild(doc.createTextNode(decode(raw)));
        }
      }
    }
    if (open.length > 1) fail("élément non fermé : " + top().qualified);
    if (!rooted) fail("aucun élément racine");
  }
  const XML_TYPES = ["text/xml", "application/xml", "application/xhtml+xml", "image/svg+xml"];
  class DOMParser {
    parseFromString(text, type) {
      const source = String(text);
      if (type === "text/html") return parseHtmlDocument(source);
      if (!XML_TYPES.includes(type)) {
        throw new TypeError("DOMParser.parseFromString : type « " + type + " » non pris en charge");
      }
      const doc = document.implementation.createDocument(null, null, null);
      try {
        parseXml(source, doc);
      } catch (error) {
        // Document d'erreur, à la manière des autres navigateurs.
        doc.replaceChildren();
        const report = doc.createElementNS("http://www.mozilla.org/newlayout/xml/parsererror.xml", "parsererror");
        report.textContent = "Erreur d'analyse XML : " + error.message;
        doc.appendChild(report);
      }
      return doc;
    }
  }
  define(DOMParser.prototype, Symbol.toStringTag, { value: "DOMParser", configurable: true });
  expose("DOMParser", DOMParser);

  // ---- Éléments personnalisés ------------------------------------------------------------------------------
  const NativeHTMLElement = global.HTMLElement;
  const rawCreateElement = global.Document.prototype.createElement;
  const definitions = new Map();   // nom -> définition
  const byConstructor = new Map(); // constructeur -> définition
  const waiting = new Map();       // nom -> { promise, resolve }
  const upgraded = new WeakSet();
  const connected = new WeakSet();
  // Élément en cours de mise à niveau : c'est lui que rend `super()`.
  let upgrading = null;

  // `HTMLElement` devient constructible : une classe peut l'étendre.
  function HTMLElement() {
    if (new.target === undefined || new.target === HTMLElement) {
      throw new TypeError("constructeur illégal");
    }
    const definition = byConstructor.get(new.target);
    if (definition === undefined) throw new TypeError("constructeur illégal : élément non défini");
    let element = upgrading;
    upgrading = null;
    if (element === null) {
      element = rawCreateElement.call(document, definition.name);
      upgraded.add(element);
    }
    Object.setPrototypeOf(element, new.target.prototype);
    return element;
  }
  HTMLElement.prototype = NativeHTMLElement.prototype;
  Object.setPrototypeOf(HTMLElement, Object.getPrototypeOf(NativeHTMLElement));
  define(NativeHTMLElement.prototype, "constructor", {
    value: HTMLElement, writable: true, enumerable: false, configurable: true,
  });
  expose("HTMLElement", HTMLElement);

  const VALID = /^[a-z][.0-9_a-z]*-[\-.0-9_a-z·À-￿]*$/;
  const RESERVED = new Set(["annotation-xml", "color-profile", "font-face", "font-face-src",
    "font-face-uri", "font-face-format", "font-face-name", "missing-glyph"]);
  const report = (error) => global.reportError(error);
  function react(element, callback, args) {
    const method = element[callback];
    if (typeof method !== "function") return;
    try {
      method.apply(element, args);
    } catch (error) {
      report(error);
    }
  }
  function upgrade(element, definition) {
    if (upgraded.has(element)) return;
    upgraded.add(element);
    upgrading = element;
    try {
      const made = new definition.constructor();
      if (made !== element) throw new TypeError("le constructeur de <" + definition.name + "> a rendu un autre objet");
    } catch (error) {
      upgrading = null;
      report(error);
      return;
    }
    for (const attribute of Array.from(element.attributes)) {
      if (definition.observed.has(attribute.name)) {
        react(element, "attributeChangedCallback", [attribute.name, null, attribute.value, attribute.namespaceURI]);
      }
    }
    if (element.isConnected) {
      connected.add(element);
      react(element, "connectedCallback", []);
    }
  }
  const definitionOf = (node) => (node.nodeType === 1 && node.namespaceURI === "http://www.w3.org/1999/xhtml"
    ? definitions.get(node.localName) : undefined);
  const withDescendants = (node) => {
    if (node.nodeType !== 1 && node.nodeType !== 11 && node.nodeType !== 9) return [];
    const all = node.nodeType === 1 ? [node] : [];
    if (definitions.size > 0) all.push(...node.querySelectorAll("*"));
    // Les arbres fantômes des éléments trouvés suivent leur hôte.
    if (shadowRoots.length > 0) {
      for (let i = 0; i < all.length; i++) {
        const root = shadowRootOf(all[i]);
        if (root !== null) all.push(...root.querySelectorAll("*"));
      }
    }
    return all;
  };
  function entered(node) {
    for (const element of withDescendants(node)) {
      const definition = definitionOf(element);
      if (definition === undefined) continue;
      if (!upgraded.has(element)) {
        upgrade(element, definition);
      } else if (element.isConnected && !connected.has(element)) {
        connected.add(element);
        react(element, "connectedCallback", []);
      }
    }
  }
  function left(node) {
    for (const element of withDescendants(node)) {
      if (connected.has(element) && !element.isConnected) {
        connected.delete(element);
        react(element, "disconnectedCallback", []);
      }
    }
  }
  // Racines fantômes connues : l'observateur les suit comme le document.
  const shadowRoots = [];
  const OBSERVED = { childList: true, subtree: true, attributes: true, attributeOldValue: true };
  // Un seul observateur suit le document, dès la première définition.
  let observer = null;
  function watch() {
    if (observer !== null) return;
    observer = new MutationObserver((records) => {
      for (const record of records) {
        if (record.type === "childList") {
          for (const node of record.removedNodes) left(node);
          for (const node of record.addedNodes) entered(node);
        } else if (record.type === "attributes" && upgraded.has(record.target)) {
          const definition = definitionOf(record.target);
          if (definition !== undefined && definition.observed.has(record.attributeName)) {
            react(record.target, "attributeChangedCallback", [record.attributeName, record.oldValue,
              record.target.getAttribute(record.attributeName), record.attributeNamespace]);
          }
        }
      }
    });
    observer.observe(document, OBSERVED);
    for (const root of shadowRoots) observer.observe(root, OBSERVED);
  }

  // ---- Shadow DOM -------------------------------------------------------------------------------------------
  // L'arbre fantôme lui-même est tenu par le moteur ; ici, ses interfaces.
  const SHADOW_HOSTS = new Set(["article", "aside", "blockquote", "body", "div", "footer", "h1", "h2",
    "h3", "h4", "h5", "h6", "header", "main", "nav", "p", "section", "span"]);
  const shadowInit = new WeakMap();
  const ShadowRoot = illegal("ShadowRoot");
  ShadowRoot.prototype = Object.create(global.DocumentFragment.prototype, {
    constructor: { value: ShadowRoot, writable: true, configurable: true },
  });
  Object.setPrototypeOf(ShadowRoot, global.DocumentFragment);
  tag(ShadowRoot, "ShadowRoot");
  expose("ShadowRoot", ShadowRoot);
  const shadowGetter = (name, get, set) => {
    define(ShadowRoot.prototype, name, { get, set, enumerable: true, configurable: true });
  };
  shadowGetter("host", function () { return shadowHostOf(this); });
  for (const name of ["mode", "delegatesFocus", "slotAssignment", "clonable", "serializable"]) {
    shadowGetter(name, function () { return shadowInit.get(this)?.[name]; });
  }
  const setShadowHtml = function (html) {
    const holder = document.createElement("div");
    holder.innerHTML = String(html);
    this.replaceChildren(...Array.from(holder.childNodes));
  };
  shadowGetter("innerHTML", function () {
    const holder = document.createElement("div");
    for (const child of Array.from(this.childNodes)) holder.appendChild(child.cloneNode(true));
    return holder.innerHTML;
  }, setShadowHtml);
  // L'élément qui a le focus, vu de cet arbre : lui-même s'il y est,
  // sinon l'hôte de l'arbre fantôme imbriqué qui le contient.
  shadowGetter("activeElement", function () {
    let node = deepFocused();
    while (node !== null) {
      const tree = node.getRootNode();
      if (tree === this) return node;
      node = shadowHostOf(tree);
    }
    return null;
  });
  for (const [name, value] of [
    ["setHTMLUnsafe", function setHTMLUnsafe(html) { setShadowHtml.call(this, html); }],
    ["getElementById", function getElementById(id) {
      const wanted = String(id);
      for (const element of this.querySelectorAll("*")) if (element.id === wanted) return element;
      return null;
    }],
  ]) {
    define(ShadowRoot.prototype, name, { value, writable: true, enumerable: true, configurable: true });
  }

  // `slotchange` : après un changement des enfants d'un hôte ou des
  // emplacements d'un arbre fantôme, chaque emplacement dont la
  // distribution a changé le signale.
  const slotSnapshots = new WeakMap();
  const sameNodes = (a, b) => a.length === b.length && a.every((node, i) => node === b[i]);
  function signalSlots(root) {
    for (const slot of root.querySelectorAll("slot")) {
      const now = assignedNodesOf(slot);
      const before = slotSnapshots.get(slot) ?? [];
      slotSnapshots.set(slot, now);
      if (!sameNodes(before, now)) slot.dispatchEvent(new Event("slotchange", { bubbles: true }));
    }
  }
  const SLOT_OBSERVED = { childList: true, subtree: true, attributes: true, attributeFilter: ["slot", "name"] };
  let slotObserver = null;
  function watchSlots(root) {
    if (slotObserver === null) {
      slotObserver = new MutationObserver((records) => {
        const roots = new Set();
        for (const record of records) {
          const target = record.target;
          // Enfant d'un hôte ajouté, retiré, ou qui demande un autre emplacement.
          const parent = record.type === "attributes" ? target.parentNode : target;
          const own = parent !== null && parent.nodeType === 1 ? shadowRootOf(parent) : null;
          if (own !== null) roots.add(own);
          // Emplacement ajouté, retiré ou renommé dans un arbre fantôme.
          const tree = target.getRootNode();
          if (shadowHostOf(tree) !== null) roots.add(tree);
        }
        for (const root of roots) signalSlots(root);
      });
      slotObserver.observe(document, SLOT_OBSERVED);
    }
    slotObserver.observe(root, SLOT_OBSERVED);
  }

  define(global.Element.prototype, "attachShadow", {
    value: function attachShadow(init) {
      if (init === null || typeof init !== "object") {
        throw new TypeError("attachShadow : un dictionnaire { mode } est attendu");
      }
      const mode = String(init.mode);
      if (mode !== "open" && mode !== "closed") {
        throw new TypeError("attachShadow : mode « " + mode + " » inconnu (open ou closed)");
      }
      const name = this.localName;
      const allowed = this.namespaceURI === "http://www.w3.org/1999/xhtml"
        && (SHADOW_HOSTS.has(name) || (VALID.test(name) && !RESERVED.has(name)));
      if (!allowed) {
        throw new DOMException("<" + name + "> ne peut pas porter d'arbre fantôme", "NotSupportedError");
      }
      const root = attachShadowRoot(this, mode === "open");
      if (root === null) {
        throw new DOMException("cet élément porte déjà un arbre fantôme", "NotSupportedError");
      }
      Object.setPrototypeOf(root, ShadowRoot.prototype);
      shadowInit.set(root, {
        mode,
        delegatesFocus: Boolean(init.delegatesFocus),
        slotAssignment: init.slotAssignment === "manual" ? "manual" : "named",
        clonable: Boolean(init.clonable),
        serializable: Boolean(init.serializable),
      });
      shadowRoots.push(root);
      if (observer !== null) observer.observe(root, OBSERVED);
      watchSlots(root);
      return root;
    },
    writable: true, enumerable: true, configurable: true,
  });
  define(global.Element.prototype, "shadowRoot", {
    get() {
      const root = shadowRootOf(this);
      return root !== null && shadowInit.get(root)?.mode === "open" ? root : null;
    },
    enumerable: true, configurable: true,
  });
  // L'emplacement où un nœud est distribué, si son arbre fantôme est ouvert.
  for (const Interface of [global.Element, global.Text]) {
    define(Interface.prototype, "assignedSlot", {
      get() {
        const slot = assignedSlotOf(this);
        return slot !== null && shadowInit.get(slot.getRootNode())?.mode === "open" ? slot : null;
      },
      enumerable: true, configurable: true,
    });
  }
  // Nœuds distribués, emplacements imbriqués et contenu de repli développés.
  const inShadowTree = (node) => shadowHostOf(node.getRootNode()) !== null;
  function flattened(slot) {
    let nodes = assignedNodesOf(slot);
    if (nodes.length === 0 && inShadowTree(slot)) {
      nodes = Array.from(slot.childNodes).filter((node) => node.nodeType === 1 || node.nodeType === 3);
    }
    const out = [];
    for (const node of nodes) {
      if (node.nodeType === 1 && node.localName === "slot" && inShadowTree(node)) out.push(...flattened(node));
      else out.push(node);
    }
    return out;
  }
  const slotMembers = {
    assignedNodes(options) { return options?.flatten ? flattened(this) : assignedNodesOf(this); },
    assignedElements(options) {
      return slotMembers.assignedNodes.call(this, options).filter((node) => node.nodeType === 1);
    },
  };
  for (const [name, value] of Object.entries(slotMembers)) {
    define(global.HTMLSlotElement.prototype, name, { value, writable: true, enumerable: true, configurable: true });
  }

  class CustomElementRegistry {
    define(name, constructor, options) {
      const text = String(name);
      if (typeof constructor !== "function" || constructor.prototype === undefined) {
        throw new TypeError("customElements.define : un constructeur est attendu");
      }
      if (!VALID.test(text) || RESERVED.has(text)) {
        throw new DOMException("« " + text + " » n'est pas un nom d'élément personnalisé valide", "SyntaxError");
      }
      if (definitions.has(text)) {
        throw new DOMException("l'élément <" + text + "> est déjà défini", "NotSupportedError");
      }
      if (byConstructor.has(constructor)) {
        throw new DOMException("ce constructeur définit déjà un autre élément", "NotSupportedError");
      }
      if (options?.extends !== undefined) {
        throw new DOMException("les éléments natifs personnalisés ne sont pas pris en charge", "NotSupportedError");
      }
      const observed = new Set(Array.from(constructor.observedAttributes ?? [], String));
      const definition = { name: text, constructor, observed };
      definitions.set(text, definition);
      byConstructor.set(constructor, definition);
      watch();
      // Les éléments déjà dans le document sont mis à niveau, dans l'ordre.
      for (const element of document.querySelectorAll(text)) upgrade(element, definition);
      for (const root of shadowRoots) {
        for (const element of root.querySelectorAll(text)) upgrade(element, definition);
      }
      const pending = waiting.get(text);
      if (pending !== undefined) {
        waiting.delete(text);
        pending.resolve(constructor);
      }
    }
    get(name) {
      return definitions.get(String(name))?.constructor;
    }
    getName(constructor) {
      return byConstructor.get(constructor)?.name ?? null;
    }
    whenDefined(name) {
      const text = String(name);
      if (!VALID.test(text) || RESERVED.has(text)) {
        return Promise.reject(new DOMException("nom d'élément personnalisé invalide : " + text, "SyntaxError"));
      }
      const known = definitions.get(text);
      if (known !== undefined) return Promise.resolve(known.constructor);
      let pending = waiting.get(text);
      if (pending === undefined) {
        pending = {};
        pending.promise = new Promise((resolve) => { pending.resolve = resolve; });
        waiting.set(text, pending);
      }
      return pending.promise;
    }
    upgrade(root) {
      entered(root);
    }
  }
  define(CustomElementRegistry.prototype, Symbol.toStringTag, { value: "CustomElementRegistry", configurable: true });
  expose("CustomElementRegistry", CustomElementRegistry);
  expose("customElements", new CustomElementRegistry());

  // `createElement` d'un nom défini rend un élément déjà mis à niveau.
  define(global.Document.prototype, "createElement", {
    value: function createElement(name, options) {
      const element = rawCreateElement.call(this, name, options);
      const definition = definitions.size > 0 && this === document ? definitionOf(element) : undefined;
      if (definition !== undefined) upgrade(element, definition);
      return element;
    },
    writable: true, enumerable: true, configurable: true,
  });
})
