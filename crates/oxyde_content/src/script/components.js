// Éléments personnalisés, `DOMParser` et contenu des gabarits.
//
// Fonction appelée par le moteur avec :
//   parseHtmlDocument(texte) -> nouveau document HTML analysé depuis `texte` ;
//   attachShadowRoot(hôte, ouvert) -> racine du nouvel arbre fantôme, ou null ;
//   shadowRootOf(élément)       -> sa racine fantôme (ouverte ou non), ou null ;
//   shadowHostOf(nœud)          -> l'hôte dont il est la racine fantôme, ou null ;
//   assignedSlotOf(nœud)        -> l'emplacement où il est distribué, ou null ;
//   assignedNodesOf(emplacement) -> tableau des nœuds qui y sont distribués ;
//   deepFocused()               -> l'élément qui a le focus, fût-il dans un arbre fantôme ;
//   shadowRootList()            -> [racine, ouverte, …] pour chaque arbre fantôme du document ;
//   parseXmlDocument(texte, type) -> nouveau document XML de type `type`, ou, si `texte`
//                                  est mal formé, le message de la faute (une chaîne).
//
// LIMITES CONNUES
// - Éléments personnalisés : les rappels `connectedCallback` et
//   `disconnectedCallback` partent à la fin de la tâche (par un observateur
//   de mutations), pas au moment même du changement ;
//   `attributeChangedCallback` part pendant `setAttribute`,
//   `removeAttribute` et `toggleAttribute`, mais en fin de tâche pour un
//   attribut changé autrement (`el.id = …`, `classList`, `dataset`) ; un élément créé par l'analyseur ou par `innerHTML` est mis à
//   niveau à ce moment-là aussi ; pas d'éléments natifs personnalisés
//   (`extends`, attribut `is`), pas de `adoptedCallback`, pas
//   d'`ElementInternals`.
// - Shadow DOM : `adoptedStyleSheets` est rendu par des éléments `<style>`
//   ajoutés à l'arbre (voir plus bas) ; l'arbre fantôme déclaratif
//   (`<template shadowrootmode>`) n'est pris qu'à l'analyse du document de
//   la page (ni cadres, ni `setHTMLUnsafe`), sans ses options
//   (`shadowrootdelegatesfocus`…) ; pas de `slot.assign()` ; `innerHTML`
//   d'une racine fantôme analyse son texte comme le contenu d'un `<div>` ;
//   `slotchange` ne part que pour un hôte relié au document, à la fin de
//   la tâche ; `delegatesFocus` est retenu sans effet ; les rappels d'un
//   élément personnalisé d'un arbre fantôme partent dans l'ordre des
//   mutations, parfois avant ceux de son hôte.
// - `DOMParser` : un document XML mal formé rend un document `parsererror`
//   (à la manière de Firefox), sans le début du document fautif.
// - `template.content` : le fragment appartient au document de la page.
// - `document.write()` : le document est déjà analysé en entier quand les
//   scripts s'exécutent ; pendant un script, le texte écrit est analysé à
//   part et inséré juste après ce script (dans `<body>` s'il s'agit de
//   contenu écrit depuis `<head>`). Chaque appel est analysé seul : une
//   balise ouverte par un appel et fermée par un autre se referme dès le
//   premier. Un script externe écrit ainsi se charge sans bloquer la suite.
//   Hors d'un script (minuteur, événement), `write()` remplace le document,
//   sans retirer ses écouteurs ni relancer `load`.
(function (parseHtmlDocument, attachShadowRoot, shadowRootOf, shadowHostOf, assignedSlotOf,
  assignedNodesOf, deepFocused, shadowRootList, parseXmlDocument) {
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
  const XML_TYPES = ["text/xml", "application/xml", "application/xhtml+xml", "image/svg+xml"];
  const PARSER_ERROR = "http://www.mozilla.org/newlayout/xml/parsererror.xml";
  // Document XML analysé depuis `source` ; un texte mal formé rend un document
  // d'erreur, à la manière des autres navigateurs.
  const parseXml = (source, type) => {
    const doc = parseXmlDocument(source, type);
    if (typeof doc !== "string") return doc;
    const report = parseXmlDocument("<parsererror xmlns=\"" + PARSER_ERROR + "\"/>", type);
    // Comme un élément créé par `createElementNS` : sans attribut `xmlns`.
    report.documentElement.removeAttribute("xmlns");
    report.documentElement.textContent = "Erreur d'analyse XML : " + doc;
    return report;
  };
  class DOMParser {
    parseFromString(text, type) {
      if (arguments.length < 2) {
        throw new TypeError("DOMParser.parseFromString : deux arguments sont requis");
      }
      const source = String(text);
      if (type === "text/html") return parseHtmlDocument(source);
      if (!XML_TYPES.includes(type)) {
        throw new TypeError("DOMParser.parseFromString : type « " + type + " » non pris en charge");
      }
      return parseXml(source, type);
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
  // `moveBefore()` : un élément personnalisé déplacé sans quitter le
  // document reçoit `connectedMoveCallback`, à défaut
  // `disconnectedCallback` puis `connectedCallback`.
  for (const owner of [global.Element, global.Document, global.DocumentFragment]) {
    const nativeMove = owner.prototype.moveBefore;
    if (typeof nativeMove !== "function") continue;
    define(owner.prototype, "moveBefore", {
      value: function moveBefore(node, child) {
        const result = nativeMove.apply(this, arguments);
        if (definitions.size > 0 && node !== null && typeof node === "object" && node.isConnected) {
          for (const element of withDescendants(node)) {
            if (!connected.has(element)) continue;
            if (typeof element.connectedMoveCallback === "function") {
              react(element, "connectedMoveCallback", []);
            } else {
              react(element, "disconnectedCallback", []);
              react(element, "connectedCallback", []);
            }
          }
        }
        return result;
      },
      writable: true, enumerable: true, configurable: true,
    });
  }
  // `attributeChangedCallback` part pendant l'appel qui change l'attribut
  // (`setAttribute`, `removeAttribute`…), comme le veut le standard : des
  // bibliothèques s'en remettent à cet ordre pour ne pas se répondre sans fin
  // (elles lèvent un drapeau le temps de l'appel). L'observateur verra la même
  // mutation en fin de tâche : `reacted` lui dit de la passer.
  const reacted = new Map();
  for (const [name, at] of [["setAttribute", 0], ["removeAttribute", 0], ["toggleAttribute", 0],
    ["setAttributeNS", 1], ["removeAttributeNS", 1]]) {
    const native = global.Element.prototype[name];
    if (typeof native !== "function") continue;
    define(global.Element.prototype, name, {
      value: { [name]: function (...args) {
        const definition = upgraded.has(this) ? definitionOf(this) : undefined;
        if (definition === undefined || args.length <= at) return native.apply(this, args);
        let attribute = String(args[at]);
        attribute = at === 0 ? attribute.toLowerCase() : attribute.slice(attribute.indexOf(":") + 1);
        if (!definition.observed.has(attribute)) return native.apply(this, args);
        const before = this.getAttribute(attribute);
        const result = native.apply(this, args);
        const after = this.getAttribute(attribute);
        if (before !== null || after !== null) {
          if (observer !== null && this.isConnected) {
            let counts = reacted.get(this);
            if (counts === undefined) reacted.set(this, counts = new Map());
            counts.set(attribute, (counts.get(attribute) ?? 0) + 1);
          }
          react(this, "attributeChangedCallback", [attribute, before, after, at === 1 ? args[0] ?? null : null]);
        }
        return result;
      } }[name],
      writable: true, enumerable: true, configurable: true,
    });
    define(global.Element.prototype[name], "length", { value: native.length, configurable: true });
  }
  // Vrai si la mutation de `name` sur `element` a déjà eu son rappel.
  const alreadyReacted = (element, name) => {
    const counts = reacted.get(element);
    const count = counts?.get(name);
    if (count === undefined) return false;
    if (count > 1) counts.set(name, count - 1);
    else {
      counts.delete(name);
      if (counts.size === 0) reacted.delete(element);
    }
    return true;
  };
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
          if (definition !== undefined && definition.observed.has(record.attributeName)
            && !alreadyReacted(record.target, record.attributeName)) {
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
  // Les arbres fantômes que l'analyseur a posés (`<template
  // shadowrootmode>`) n'ont pas encore leurs interfaces : elles leur sont
  // données à la première occasion.
  let knownRoots = 0;
  function discover() {
    const list = shadowRootList();
    if (list.length / 2 === knownRoots) return;
    knownRoots = list.length / 2;
    for (let i = 0; i < list.length; i += 2) {
      const root = list[i];
      if (shadowInit.has(root)) continue;
      Object.setPrototypeOf(root, ShadowRoot.prototype);
      shadowInit.set(root, {
        mode: list[i + 1] ? "open" : "closed", delegatesFocus: false, slotAssignment: "named",
        clonable: false, serializable: false,
      });
      shadowRoots.push(root);
      if (observer !== null) observer.observe(root, OBSERVED);
      watchSlots(root);
    }
  }
  const initOf = (root) => {
    if (root !== null && typeof root === "object" && !shadowInit.has(root)) discover();
    return shadowInit.get(root);
  };
  const shadowGetter = (name, get, set) => {
    define(ShadowRoot.prototype, name, { get, set, enumerable: true, configurable: true });
  };
  shadowGetter("host", function () { return shadowHostOf(this); });
  for (const name of ["mode", "delegatesFocus", "slotAssignment", "clonable", "serializable"]) {
    shadowGetter(name, function () { return initOf(this)?.[name]; });
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
        // Emplacements sans nœud distribué dont le contenu de repli change.
        const fallbacks = new Set();
        for (const record of records) {
          const target = record.target;
          if (record.type === "childList" && target.nodeType === 1 && target.localName === "slot"
            && shadowHostOf(target.getRootNode()) !== null && assignedNodesOf(target).length === 0) {
            fallbacks.add(target);
          }
          // Enfant d'un hôte ajouté, retiré, ou qui demande un autre emplacement.
          const parent = record.type === "attributes" ? target.parentNode : target;
          const own = parent !== null && parent.nodeType === 1 ? shadowRootOf(parent) : null;
          if (own !== null) roots.add(own);
          // Emplacement ajouté, retiré ou renommé dans un arbre fantôme.
          const tree = target.getRootNode();
          if (shadowHostOf(tree) !== null) roots.add(tree);
        }
        for (const root of roots) signalSlots(root);
        for (const slot of fallbacks) slot.dispatchEvent(new Event("slotchange", { bubbles: true }));
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
      knownRoots += 1;
      if (observer !== null) observer.observe(root, OBSERVED);
      watchSlots(root);
      return root;
    },
    writable: true, enumerable: true, configurable: true,
  });
  define(global.Element.prototype, "shadowRoot", {
    get() {
      const root = shadowRootOf(this);
      return root !== null && initOf(root)?.mode === "open" ? root : null;
    },
    enumerable: true, configurable: true,
  });
  // L'emplacement où un nœud est distribué, si son arbre fantôme est ouvert.
  for (const Interface of [global.Element, global.Text]) {
    define(Interface.prototype, "assignedSlot", {
      get() {
        const slot = assignedSlotOf(this);
        return slot !== null && initOf(slot.getRootNode())?.mode === "open" ? slot : null;
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
      discover();
      // Les éléments déjà dans le document sont mis à niveau, dans l'ordre.
      // Par nom d'élément, pas par sélecteur : un nom peut porter un point.
      for (const element of Array.from(document.getElementsByTagName(text))) upgrade(element, definition);
      for (const root of shadowRoots) {
        for (const element of root.querySelectorAll("*")) {
          if (element.localName === text) upgrade(element, definition);
        }
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
  // ---- document.write(), writeln(), open(), close() ----------------------------------------------------
  // Éléments qui ont leur place dans `<head>`.
  const METADATA = new Set(["base", "link", "meta", "noscript", "script", "style", "template", "title"]);
  // Point d'insertion du script en cours : après le dernier nœud qu'il a écrit.
  let cursor = null;
  // Texte écrit dans un document rouvert, par document.
  const reopened = new WeakMap();
  const inHead = (node) => {
    for (let at = node; at !== null; at = at.parentNode) {
      if (at.nodeType === 1 && at.localName === "head") return true;
    }
    return false;
  };
  // Écrit `text` à la suite du script `script`, comme le ferait l'analyseur.
  function writeAfterScript(doc, script, text) {
    if (cursor === null || cursor.script !== script) {
      cursor = { script, last: script, body: null };
    }
    const holder = rawCreateElement.call(doc, "div");
    holder.innerHTML = text;
    for (const node of Array.from(holder.childNodes)) {
      // Dans `<head>`, seules les métadonnées restent ; le reste ouvre le corps.
      const metadata = node.nodeType === 8 || (node.nodeType === 3 && node.data.trim() === "")
        || (node.nodeType === 1 && METADATA.has(node.localName));
      if (inHead(cursor.script) && !metadata && doc.body !== null) {
        if (cursor.body === null || cursor.body.parentNode !== doc.body) {
          doc.body.insertBefore(node, doc.body.firstChild);
        } else {
          cursor.body.after(node);
        }
        cursor.body = node;
      } else if (cursor.last.parentNode !== null) {
        cursor.last.after(node);
        cursor.last = node;
      }
    }
  }
  // Remplace le contenu de `doc` par l'analyse de `text`.
  function rewrite(doc, text) {
    const parsed = parseHtmlDocument(text);
    doc.replaceChildren();
    for (const node of Array.from(parsed.childNodes)) doc.appendChild(doc.adoptNode(node));
  }
  function write(doc, pieces, ending) {
    const text = pieces.map(String).join("") + ending;
    const script = doc === document ? doc.currentScript : null;
    if (script !== null && script.isConnected && !reopened.has(doc)) {
      writeAfterScript(doc, script, text);
      return;
    }
    // Hors de l'analyse : le document est rouvert, et reçoit ce texte.
    if (!reopened.has(doc)) reopened.set(doc, "");
    const whole = reopened.get(doc) + text;
    reopened.set(doc, whole);
    rewrite(doc, whole);
  }
  const documentMethod = (name, value) => define(global.Document.prototype, name, {
    value, writable: true, enumerable: true, configurable: true,
  });
  documentMethod("write", function write_(...pieces) { write(this, pieces, ""); });
  documentMethod("writeln", function writeln(...pieces) { write(this, pieces, "\n"); });
  documentMethod("open", function open() {
    // `open(adresse, nom, options)` ouvre une fenêtre : c'est `window.open`.
    if (arguments.length >= 3) return global.open?.(...arguments) ?? null;
    // Pendant qu'un script du document s'exécute, l'analyse est en cours : sans effet.
    if (this === document && this.currentScript !== null && !reopened.has(this)) return this;
    reopened.set(this, "");
    this.replaceChildren();
    return this;
  });
  documentMethod("close", function close() {
    reopened.delete(this);
  });

  // `attributes` et les déclarations de style se parcourent par `for…of`
  // comme toute collection indexée.
  for (const name of ["NamedNodeMap", "CSSStyleDeclaration"]) {
    const proto = global[name]?.prototype;
    if (proto !== undefined && !(Symbol.iterator in proto)) {
      define(proto, Symbol.iterator, {
        value: Array.prototype[Symbol.iterator], writable: true, configurable: true,
      });
    }
  }

  // ---- Feuilles de style construites (CSSOM §6.1) et `adoptedStyleSheets` ------------------------------
  // Une feuille construite garde son texte, découpé en règles de premier
  // niveau. Adoptée par un document ou un arbre fantôme, elle y est rendue
  // par un élément `<style data-oxyde-adopted>` ajouté en fin d'arbre, que le
  // moteur de style traite comme tout autre.
  // LIMITES : ces éléments `<style>` sont visibles de la page (`querySelector`,
  // `childNodes`) ; `cssRules` ne donne que `cssText` et `type` ; ni
  // `document.styleSheets` ni `sheet` d'un élément ; `media` est retenu sans effet.
  const splitRules = (text) => {
    const rules = [];
    const keep = (from, to) => {
      const rule = text.slice(from, to).trim();
      if (rule !== "") rules.push(rule);
    };
    let depth = 0, start = 0, quote = null;
    for (let i = 0; i < text.length; i++) {
      const c = text[i];
      if (quote !== null) {
        if (c === "\\") i++;
        else if (c === quote) quote = null;
      } else if (c === '"' || c === "'") {
        quote = c;
      } else if (c === "/" && text[i + 1] === "*") {
        const end = text.indexOf("*/", i + 2);
        i = end < 0 ? text.length : end + 1;
      } else if (c === "{") {
        depth++;
      } else if (c === "}") {
        if (depth > 0 && --depth === 0) { keep(start, i + 1); start = i + 1; }
      } else if (c === ";" && depth === 0) {
        keep(start, i + 1); start = i + 1;
      }
    }
    keep(start, text.length);
    return rules;
  };
  class CSSRule {
    #text;
    constructor(key, text) {
      if (key !== splitRules) throw new TypeError("constructeur illégal");
      this.#text = text;
    }
    get cssText() { return this.#text; }
    get type() { return this.#text.startsWith("@") ? 0 : 1; }
  }
  tag(CSSRule, "CSSRule");
  expose("CSSRule", CSSRule);
  // Éléments `<style>` qui rendent chaque feuille adoptée.
  const rendered = new WeakMap();
  class CSSStyleSheet {
    #rules = []; #disabled; #media;
    constructor(options = undefined) {
      this.#disabled = Boolean(options?.disabled);
      this.#media = String(options?.media ?? "");
    }
    get type() { return "text/css"; }
    get href() { return null; }
    get ownerNode() { return null; }
    get parentStyleSheet() { return null; }
    get title() { return null; }
    get media() { return { mediaText: this.#media, length: this.#media === "" ? 0 : 1 }; }
    get disabled() { return this.#disabled; }
    set disabled(value) { this.#disabled = Boolean(value); this.#refresh(); }
    get cssRules() { return Object.freeze(this.#rules.map((text) => new CSSRule(splitRules, text))); }
    get rules() { return this.cssRules; }
    get cssText() { return this.#disabled ? "" : this.#rules.join("\n"); }
    #refresh() {
      for (const element of rendered.get(this) ?? []) element.textContent = this.cssText;
    }
    replaceSync(text) {
      // Les règles `@import` d'une feuille construite sont ignorées.
      this.#rules = splitRules(String(text)).filter((rule) => !/^@import\b/i.test(rule));
      this.#refresh();
    }
    replace(text) {
      try {
        this.replaceSync(text);
      } catch (error) {
        return Promise.reject(error);
      }
      return Promise.resolve(this);
    }
    insertRule(rule, index = 0) {
      const at = Number(index) >>> 0;
      if (at > this.#rules.length) throw new DOMException("insertRule : rang hors de la feuille", "IndexSizeError");
      const parsed = splitRules(String(rule));
      if (parsed.length !== 1) throw new DOMException("insertRule : une règle et une seule", "SyntaxError");
      this.#rules.splice(at, 0, parsed[0]);
      this.#refresh();
      return at;
    }
    deleteRule(index) {
      const at = Number(index) >>> 0;
      if (at >= this.#rules.length) throw new DOMException("deleteRule : rang hors de la feuille", "IndexSizeError");
      this.#rules.splice(at, 1);
      this.#refresh();
    }
    addRule(selector = "undefined", style = "undefined", index = undefined) {
      this.insertRule(`${selector} { ${style} }`, index === undefined ? this.#rules.length : index);
      return -1;
    }
    removeRule(index = 0) { this.deleteRule(index); }
  }
  tag(CSSStyleSheet, "CSSStyleSheet");
  expose("CSSStyleSheet", CSSStyleSheet);
  expose("StyleSheet", CSSStyleSheet);

  // Feuilles adoptées par arbre : la liste, et les éléments qui la rendent.
  const adopted = new WeakMap();
  function adopt(tree, sheets) {
    const state = adopted.get(tree) ?? { sheets: [], elements: [] };
    adopted.set(tree, state);
    state.elements.forEach((element, i) => {
      rendered.get(state.sheets[i])?.delete(element);
      element.remove();
    });
    state.sheets = sheets;
    const owner = tree.ownerDocument ?? tree;
    const container = tree.nodeType === 9 ? (tree.head ?? tree.documentElement) : tree;
    state.elements = container === null ? [] : sheets.map((sheet) => {
      const element = rawCreateElement.call(owner, "style");
      element.setAttribute("data-oxyde-adopted", "");
      element.textContent = sheet.cssText;
      container.appendChild(element);
      if (!rendered.has(sheet)) rendered.set(sheet, new Set());
      rendered.get(sheet).add(element);
      return element;
    });
  }
  const checkedSheets = (value) => {
    const sheets = Array.from(value);
    for (const sheet of sheets) {
      if (!(sheet instanceof CSSStyleSheet)) throw new TypeError("adoptedStyleSheets : une CSSStyleSheet est attendue");
    }
    return sheets;
  };
  for (const owner of [global.Document.prototype, ShadowRoot.prototype]) {
    define(owner, "adoptedStyleSheets", {
      // Le tableau rendu est observé : `push`, `splice` ou une affectation par
      // indice refont l'adoption.
      get() {
        const tree = this;
        const sheets = (adopted.get(tree)?.sheets ?? []).slice();
        const again = () => adopt(tree, checkedSheets(sheets));
        return new Proxy(sheets, {
          set(target, key, value) {
            target[key] = value;
            // `push` écrit l'élément puis `length` : une seule adoption suffit.
            if (key !== "length" || value !== (adopted.get(tree)?.sheets.length ?? 0)) again();
            return true;
          },
          deleteProperty(target, key) {
            delete target[key];
            return true;
          },
        });
      },
      set(value) { adopt(this, checkedSheets(value)); },
      enumerable: true, configurable: true,
    });
  }
})
