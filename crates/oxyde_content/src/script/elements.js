// Interfaces des éléments HTML (HTML Standard §4) et formulaires, écrites en
// JavaScript. Le script rend une fabrique qui reçoit :
//   registerInterface(balise, prototype) -> prototype des éléments de cette balise ;
//   parseUrl, changeUrl, locationHref, navigate -> voir le prélude ;
//   setElementState(élément, nom, actif) -> pose l'état `modal`, `popover-open` ou
//                                           `select-open` (liste déroulée).
// Elle rend les fonctions d'activation que le moteur appelle autour d'un clic.
//
// LIMITES CONNUES : l'état d'un contrôle (valeur, case cochée, option
// choisie) vit dans ses attributs ; un contrôle est rattaché au formulaire
// qui le contient (l'attribut `form` est ignoré) ; pas de fichiers ; la
// validation des contraintes ignore la longueur du texte ; une liste déroulante ne se
// déroule pas (les flèches changent l'option) ; la frappe dans une zone de
// texte se fait en fin de texte.
(function (registerInterface, parseUrl, changeUrl, locationHref, navigate, setElementState) {
  "use strict";
  const global = globalThis;
  const define = Object.defineProperty;
  const HREF = 0;
  const SEARCH = 8;
  const urlRanks = { protocol: 1, username: 2, password: 3, host: 4, hostname: 5, port: 6,
    pathname: 7, search: 8, hash: 9 };

  // ---- Attributs reflétés ------------------------------------------------------------------
  const accessor = (get, set) => ({ get, set, enumerable: true, configurable: true });
  const text = (attribute) => accessor(
    function () { return this.getAttribute(attribute) ?? ""; },
    function (value) { this.setAttribute(attribute, String(value)); },
  );
  const flag = (attribute) => accessor(
    function () { return this.hasAttribute(attribute); },
    function (value) { this.toggleAttribute(attribute, Boolean(value)); },
  );
  const integer = (attribute, fallback) => accessor(
    function () {
      const parsed = parseInt(this.getAttribute(attribute) ?? "", 10);
      return Number.isNaN(parsed) ? fallback : parsed;
    },
    function (value) { this.setAttribute(attribute, String(Math.trunc(Number(value)) || 0)); },
  );
  const address = (attribute) => accessor(
    function () {
      const raw = this.getAttribute(attribute);
      if (raw === null) return "";
      const parts = parseUrl(raw, this.baseURI);
      return parts === null ? raw : parts[HREF];
    },
    function (value) { this.setAttribute(attribute, String(value)); },
  );
  // Mot-clé parmi `allowed`, sinon `fallback`.
  const keyword = (attribute, allowed, fallback) => accessor(
    function () {
      const value = (this.getAttribute(attribute) ?? "").toLowerCase();
      return allowed.includes(value) ? value : fallback;
    },
    function (value) { this.setAttribute(attribute, String(value)); },
  );
  const method = (value) => ({ value, writable: true, enumerable: true, configurable: true });

  // Crée l'interface `name` des éléments de balises `tags`.
  function elementInterface(name, tags, members = {}) {
    const Interface = function () { throw new TypeError("constructeur illégal"); };
    define(Interface, "name", { value: name, configurable: true });
    Interface.prototype = Object.create(HTMLElement.prototype, {
      constructor: { value: Interface, writable: true, configurable: true },
      [Symbol.toStringTag]: { value: name, configurable: true },
    });
    Object.setPrototypeOf(Interface, HTMLElement);
    Object.defineProperties(Interface.prototype, members);
    define(global, name, { value: Interface, writable: true, configurable: true });
    for (const tagName of tags) registerInterface(tagName, Interface.prototype);
    return Interface;
  }

  // ---- Éléments sans membre propre notable ---------------------------------------------------
  for (const [name, tags] of [
    ["HTMLHtmlElement", ["html"]], ["HTMLHeadElement", ["head"]], ["HTMLSlotElement", ["slot"]],
    ["HTMLTitleElement", ["title"]], ["HTMLDivElement", ["div"]], ["HTMLSpanElement", ["span"]],
    ["HTMLParagraphElement", ["p"]], ["HTMLPreElement", ["pre"]], ["HTMLBRElement", ["br"]],
    ["HTMLHRElement", ["hr"]], ["HTMLHeadingElement", ["h1", "h2", "h3", "h4", "h5", "h6"]],
    ["HTMLUListElement", ["ul"]], ["HTMLOListElement", ["ol"]], ["HTMLLIElement", ["li"]],
    ["HTMLDListElement", ["dl"]], ["HTMLQuoteElement", ["blockquote", "q"]],
    ["HTMLTableCaptionElement", ["caption"]],
    ["HTMLFieldSetElement", ["fieldset"]], ["HTMLLegendElement", ["legend"]],
    ["HTMLStyleElement", ["style"]], ["HTMLTemplateElement", ["template"]],
    ["HTMLPictureElement", ["picture"]], ["HTMLDetailsElement", ["details"]],
    ["HTMLTimeElement", ["time"]],
  ]) {
    elementInterface(name, tags);
  }

  // ---- <dialog> et popovers (HTML Standard §4.11.4 et §6.12) ----------------------------------------
  // LIMITES : pas de pseudo-élément `::backdrop` ; le reste de la page n'est
  // pas rendu inerte par une boîte modale ; `closedby`, les popovers `hint`
  // et l'ancrage d'un popover à son bouton ne sont pas pris en charge ; un
  // popover n'est imbriqué dans un autre que s'il en est un descendant.
  const invalidState = (message) => new DOMException(message, "InvalidStateError");
  const modals = [];
  const openPopovers = [];
  const previousFocus = new WeakMap();
  const returnValues = new WeakMap();
  const later = (run) => global.setTimeout(run, 0);
  class ToggleEvent extends global.Event {
    #oldState;
    #newState;
    constructor(type, init = {}) {
      super(type, init);
      this.#oldState = String(init?.oldState ?? "");
      this.#newState = String(init?.newState ?? "");
    }
    get oldState() { return this.#oldState; }
    get newState() { return this.#newState; }
  }
  define(ToggleEvent.prototype, Symbol.toStringTag, { value: "ToggleEvent", configurable: true });
  if (global.ToggleEvent === undefined) {
    define(global, "ToggleEvent", { value: ToggleEvent, writable: true, enumerable: false, configurable: true });
  }
  // Le focus va au premier élément `autofocus` de `container`, à défaut au
  // premier contrôle qui peut le prendre.
  function focusInside(container) {
    const candidates = Array.from(container.querySelectorAll(
      "[autofocus], a[href], button, input, select, textarea, [tabindex]",
    )).filter((element) => !element.matches(":disabled") && element.getAttribute("tabindex") !== "-1"
      && !(element.localName === "input" && inputType(element) === "hidden"));
    const target = candidates.find((element) => element.hasAttribute("autofocus")) ?? candidates[0];
    target?.focus?.();
  }
  const restoreFocus = (element) => {
    const previous = previousFocus.get(element);
    previousFocus.delete(element);
    if (previous && previous.isConnected && previous !== document.body) previous.focus?.();
  };
  function closeDialog(dialog, result) {
    if (!dialog.hasAttribute("open")) return;
    dialog.removeAttribute("open");
    const at = modals.indexOf(dialog);
    if (at >= 0) {
      modals.splice(at, 1);
      setElementState(dialog, "modal", false);
    }
    if (result !== undefined) returnValues.set(dialog, String(result));
    restoreFocus(dialog);
    later(() => dialog.dispatchEvent(new Event("close")));
  }
  // Demande de fermeture (touche Échap, `requestClose()`) : `cancel` peut la refuser.
  function requestClose(dialog, result) {
    if (!dialog.hasAttribute("open")) return;
    if (dialog.dispatchEvent(new Event("cancel", { cancelable: true }))) closeDialog(dialog, result);
  }
  elementInterface("HTMLDialogElement", ["dialog"], {
    open: flag("open"),
    returnValue: accessor(
      function () { return returnValues.get(this) ?? ""; },
      function (value) { returnValues.set(this, String(value)); },
    ),
    show: method(function show() {
      if (this.hasAttribute("open")) {
        if (modals.includes(this)) throw invalidState("la boîte de dialogue est déjà ouverte en mode modal");
        return;
      }
      if (openPopovers.includes(this)) throw invalidState("l'élément est affiché comme popover");
      this.setAttribute("open", "");
      previousFocus.set(this, document.activeElement);
      focusInside(this);
    }),
    showModal: method(function showModal() {
      if (this.hasAttribute("open")) {
        if (modals.includes(this)) return;
        throw invalidState("la boîte de dialogue est déjà ouverte");
      }
      if (!this.isConnected) throw invalidState("la boîte de dialogue n'est pas dans le document");
      if (openPopovers.includes(this)) throw invalidState("l'élément est affiché comme popover");
      this.setAttribute("open", "");
      modals.push(this);
      setElementState(this, "modal", true);
      previousFocus.set(this, document.activeElement);
      focusInside(this);
    }),
    close: method(function close(result) { closeDialog(this, result); }),
    requestClose: method(function requestClose_(result) { requestClose(this, result); }),
  });

  // État d'un popover d'après son attribut : `null` s'il n'en est pas un.
  const popoverKind = (element) => {
    const value = element.getAttribute("popover");
    if (value === null) return null;
    const kind = value.toLowerCase();
    return kind === "" || kind === "auto" ? "auto" : "manual";
  };
  const toggled = (type, showing, cancelable) => new ToggleEvent(type, {
    oldState: showing ? "closed" : "open", newState: showing ? "open" : "closed", cancelable,
  });
  function hidePopover(element, strict) {
    if (popoverKind(element) === null && !openPopovers.includes(element)) {
      throw new DOMException("l'élément n'est pas un popover", "NotSupportedError");
    }
    if (!openPopovers.includes(element)) {
      if (strict) throw invalidState("le popover n'est pas affiché");
      return;
    }
    // Les popovers ouverts depuis celui-ci se ferment avec lui.
    for (const inner of openPopovers.slice(openPopovers.indexOf(element) + 1).reverse()) {
      if (element.contains(inner)) hidePopover(inner, false);
    }
    element.dispatchEvent(toggled("beforetoggle", false, false));
    const at = openPopovers.indexOf(element);
    if (at < 0) return;
    openPopovers.splice(at, 1);
    setElementState(element, "popover-open", false);
    restoreFocus(element);
    later(() => element.dispatchEvent(toggled("toggle", false, false)));
  }
  function showPopover(element) {
    const kind = popoverKind(element);
    if (kind === null) throw new DOMException("l'élément n'est pas un popover", "NotSupportedError");
    if (!element.isConnected) throw invalidState("le popover n'est pas dans le document");
    if (openPopovers.includes(element)) throw invalidState("le popover est déjà affiché");
    if (element.localName === "dialog" && element.hasAttribute("open")) {
      throw invalidState("la boîte de dialogue est déjà ouverte");
    }
    if (!element.dispatchEvent(toggled("beforetoggle", true, true))) return;
    if (openPopovers.includes(element) || !element.isConnected) return;
    // Un popover automatique ferme ceux qui ne le contiennent pas.
    if (kind === "auto") {
      for (const other of openPopovers.slice().reverse()) {
        if (popoverKind(other) === "auto" && !other.contains(element)) hidePopover(other, false);
      }
    }
    previousFocus.set(element, document.activeElement);
    openPopovers.push(element);
    setElementState(element, "popover-open", true);
    if (element.querySelector("[autofocus]") !== null || element.hasAttribute("autofocus")) focusInside(element);
    later(() => element.dispatchEvent(toggled("toggle", true, false)));
  }
  const htmlMembers = {
    popover: accessor(
      function () { return popoverKind(this); },
      function (value) {
        if (value === null || value === undefined) this.removeAttribute("popover");
        else this.setAttribute("popover", String(value));
      },
    ),
    showPopover: method(function showPopover_() { showPopover(this); }),
    hidePopover: method(function hidePopover_() { hidePopover(this, true); }),
    togglePopover: method(function togglePopover(force) {
      const wanted = typeof force === "object" && force !== null ? force.force : force;
      const showing = openPopovers.includes(this);
      if (showing && wanted !== true) hidePopover(this, true);
      else if (!showing && wanted !== false) showPopover(this);
      else if (popoverKind(this) === null) {
        throw new DOMException("l'élément n'est pas un popover", "NotSupportedError");
      }
      return openPopovers.includes(this);
    }),
  };
  for (const [name, descriptor] of Object.entries(htmlMembers)) {
    define(global.HTMLElement.prototype, name, descriptor);
  }
  // ---- Listes déroulantes ------------------------------------------------------------------
  // Une liste à choix unique se déroule au clic : ses options s'affichent
  // au-dessus de la page, un clic sur l'une la choisit. LIMITES : une
  // liste `multiple` ou de taille supérieure à 1 ne se déroule pas ; la
  // liste déroulée ne défile pas si elle dépasse la fenêtre.
  const openSelects = new Set();
  const isDropdown = (select) => !select.hasAttribute("multiple")
    && !(Number(select.getAttribute("size")) > 1) && !select.matches(":disabled");
  const closeSelect = (select) => {
    if (!openSelects.delete(select)) return;
    setElementState(select, "select-open", false);
  };
  const openSelect = (select) => {
    if (!isDropdown(select) || openSelects.has(select)) return;
    for (const other of [...openSelects]) closeSelect(other);
    openSelects.add(select);
    setElementState(select, "select-open", true);
  };
  const toggleSelect = (select) => {
    if (openSelects.has(select)) closeSelect(select);
    else openSelect(select);
  };
  // L'option `option` devient celle que la liste `select` a choisie.
  const pickOption = (select, option) => {
    if (option.hasAttribute("disabled") || isSelected(option)) return;
    for (const other of optionsOf(select)) other.removeAttribute("selected");
    option.setAttribute("selected", "");
    if (!select.isConnected) return;
    select.dispatchEvent(new Event("input", { bubbles: true }));
    select.dispatchEvent(new Event("change", { bubbles: true }));
  };
  global.addEventListener("click", (event) => {
    const target = event.target;
    const inside = target !== null && typeof target.closest === "function";
    const option = inside ? target.closest("option") : null;
    const select = inside ? target.closest("select") : null;
    // Un clic ailleurs referme les listes déroulées.
    for (const other of [...openSelects]) {
      if (other !== select) closeSelect(other);
    }
    if (select === null || event.defaultPrevented || !isDropdown(select)) return;
    if (option !== null && openSelects.has(select)) {
      pickOption(select, option);
      closeSelect(select);
    } else {
      toggleSelect(select);
    }
  });
  // La liste se referme quand elle perd le focus.
  global.addEventListener("focusout", (event) => {
    if (openSelects.has(event.target)) closeSelect(event.target);
  });

  // Bouton `popovertarget` : son clic affiche ou masque le popover visé ;
  // tout autre clic ferme les popovers automatiques qu'il ne touche pas.
  global.addEventListener("click", (event) => {
    const target = event.target;
    if (target === null || typeof target.closest !== "function") return;
    const invoker = target.closest("button[popovertarget], input[popovertarget]");
    let aimed = null;
    if (invoker !== null && !event.defaultPrevented && !invoker.matches(":disabled")) {
      aimed = invoker.ownerDocument.getElementById(invoker.getAttribute("popovertarget"));
      if (aimed !== null && popoverKind(aimed) === null) aimed = null;
    }
    for (const popover of openPopovers.slice().reverse()) {
      if (popoverKind(popover) === "auto" && popover !== aimed && !popover.contains(target)
        && !(aimed !== null && popover.contains(aimed))) hidePopover(popover, false);
    }
    if (aimed === null) return;
    const action = (invoker.getAttribute("popovertargetaction") ?? "toggle").toLowerCase();
    const showing = openPopovers.includes(aimed);
    if (showing && action !== "show") hidePopover(aimed, false);
    else if (!showing && action !== "hide") showPopover(aimed);
  });
  // Échap ferme le popover automatique du dessus, sinon la boîte modale du dessus.
  global.addEventListener("keydown", (event) => {
    if (event.key !== "Escape" || event.defaultPrevented) return;
    // Une liste déroulée se referme d'abord.
    if (openSelects.size > 0) {
      for (const select of [...openSelects]) closeSelect(select);
      return;
    }
    const popover = openPopovers.slice().reverse().find((element) => popoverKind(element) === "auto");
    if (popover !== undefined) {
      hidePopover(popover, false);
      return;
    }
    const dialog = modals.filter((element) => element.isConnected).pop();
    if (dialog !== undefined) requestClose(dialog);
  });

  // ---- Tableaux ------------------------------------------------------------------------------------
  // LIMITE : `rows`, `cells` et `tBodies` rendent une collection figée au
  // moment de la lecture, pas une collection vivante.
  const collection = (nodes) => Object.setPrototypeOf(nodes, global.HTMLCollection.prototype);
  const childrenNamed = (element, names) => Array.from(element.children)
    .filter((child) => names.includes(child.localName));
  // Lignes d'un tableau : celles des en-têtes, puis celles du corps (et les
  // lignes posées directement dans le tableau), puis celles des pieds.
  const tableRows = (table) => {
    const head = [];
    const body = [];
    const foot = [];
    for (const child of Array.from(table.children)) {
      if (child.localName === "tr") body.push(child);
      else if (child.localName === "thead") head.push(...childrenNamed(child, ["tr"]));
      else if (child.localName === "tbody") body.push(...childrenNamed(child, ["tr"]));
      else if (child.localName === "tfoot") foot.push(...childrenNamed(child, ["tr"]));
    }
    return [...head, ...body, ...foot];
  };
  const rank = (index, fallback) => (index === undefined ? fallback : Math.trunc(Number(index)) || 0);
  const outOfRange = () => new DOMException("rang hors du tableau", "IndexSizeError");
  // Insère un nouvel élément `name` au rang `index` parmi `items`, enfants
  // de `parent` ; -1 : à la fin.
  function insertAt(parent, items, index, name) {
    const at = rank(index, -1);
    if (at < -1 || at > items.length) throw outOfRange();
    const created = parent.ownerDocument.createElement(name);
    if (at === -1 || at === items.length) parent.appendChild(created);
    else items[at].parentNode.insertBefore(created, items[at]);
    return created;
  }
  // Retire l'élément de rang `index` parmi `items` ; -1 : le dernier.
  function deleteAt(items, index) {
    let at = rank(index, 0);
    if (at < -1 || at >= items.length) {
      // Retirer le dernier d'une liste vide ne fait rien.
      if (at === -1 && items.length === 0) return;
      throw outOfRange();
    }
    if (at === -1) at = items.length - 1;
    items[at].remove();
  }
  // Première section `name` du tableau (`caption`, `thead`, `tfoot`).
  const section = (name) => accessor(
    function () { return childrenNamed(this, [name])[0] ?? null; },
    function (value) {
      if (value !== null && (value?.nodeType !== 1 || value.localName !== name)) {
        throw new TypeError("un élément <" + name + "> est attendu");
      }
      childrenNamed(this, [name])[0]?.remove();
      if (value === null) return;
      if (name === "caption") this.insertBefore(value, this.firstChild);
      else if (name === "thead") {
        const after = Array.from(this.children)
          .find((child) => child.localName !== "caption" && child.localName !== "colgroup");
        this.insertBefore(value, after ?? null);
      } else this.appendChild(value);
    },
  );
  // `createTHead()`… : la section existante, sinon une nouvelle, à sa place.
  const creator = (name, property) => method(function () {
    const existing = this[property];
    if (existing !== null) return existing;
    this[property] = this.ownerDocument.createElement(name);
    return this[property];
  });
  const remover = (property) => method(function () { this[property]?.remove(); });
  elementInterface("HTMLTableElement", ["table"], {
    caption: section("caption"),
    tHead: section("thead"),
    tFoot: section("tfoot"),
    tBodies: accessor(function () { return collection(childrenNamed(this, ["tbody"])); }),
    rows: accessor(function () { return collection(tableRows(this)); }),
    createCaption: creator("caption", "caption"),
    deleteCaption: remover("caption"),
    createTHead: creator("thead", "tHead"),
    deleteTHead: remover("tHead"),
    createTFoot: creator("tfoot", "tFoot"),
    deleteTFoot: remover("tFoot"),
    createTBody: method(function createTBody() {
      const body = this.ownerDocument.createElement("tbody");
      const bodies = childrenNamed(this, ["tbody"]);
      const last = bodies[bodies.length - 1];
      this.insertBefore(body, last === undefined ? null : last.nextSibling);
      return body;
    }),
    insertRow: method(function insertRow(index) {
      const rows = tableRows(this);
      const at = rank(index, -1);
      if (at < -1 || at > rows.length) throw outOfRange();
      // Dans un tableau sans ligne, la ligne va dans un corps (créé au besoin).
      if (rows.length === 0) {
        const bodies = childrenNamed(this, ["tbody"]);
        const body = bodies[bodies.length - 1] ?? this.appendChild(this.ownerDocument.createElement("tbody"));
        return body.appendChild(this.ownerDocument.createElement("tr"));
      }
      const row = this.ownerDocument.createElement("tr");
      if (at === -1 || at === rows.length) rows[rows.length - 1].parentNode.appendChild(row);
      else rows[at].parentNode.insertBefore(row, rows[at]);
      return row;
    }),
    deleteRow: method(function deleteRow(index) { deleteAt(tableRows(this), index); }),
  });
  elementInterface("HTMLTableSectionElement", ["thead", "tbody", "tfoot"], {
    rows: accessor(function () { return collection(childrenNamed(this, ["tr"])); }),
    insertRow: method(function insertRow(index) {
      return insertAt(this, childrenNamed(this, ["tr"]), index, "tr");
    }),
    deleteRow: method(function deleteRow(index) { deleteAt(childrenNamed(this, ["tr"]), index); }),
  });
  const rowCells = (row) => childrenNamed(row, ["td", "th"]);
  elementInterface("HTMLTableRowElement", ["tr"], {
    cells: accessor(function () { return collection(rowCells(this)); }),
    // Rang de la ligne dans son tableau, et dans sa section ; -1 hors d'eux.
    rowIndex: accessor(function () {
      const parent = this.parentNode;
      const table = parent?.localName === "table" ? parent
        : (["thead", "tbody", "tfoot"].includes(parent?.localName) ? parent.parentNode : null);
      return table?.localName === "table" ? tableRows(table).indexOf(this) : -1;
    }),
    sectionRowIndex: accessor(function () {
      const parent = this.parentNode;
      return ["table", "thead", "tbody", "tfoot"].includes(parent?.localName)
        ? childrenNamed(parent, ["tr"]).indexOf(this) : -1;
    }),
    insertCell: method(function insertCell(index) { return insertAt(this, rowCells(this), index, "td"); }),
    deleteCell: method(function deleteCell(index) { deleteAt(rowCells(this), index); }),
  });
  elementInterface("HTMLTableCellElement", ["td", "th"], {
    colSpan: integer("colspan", 1),
    rowSpan: integer("rowspan", 1),
    headers: text("headers"),
    scope: keyword("scope", ["row", "col", "rowgroup", "colgroup"], ""),
    abbr: text("abbr"),
    cellIndex: accessor(function () {
      return this.parentNode?.localName === "tr" ? rowCells(this.parentNode).indexOf(this) : -1;
    }),
  });

  // ---- Liens, images, scripts ---------------------------------------------------------------------
  const anchorParts = {};
  for (const [name, rank] of Object.entries(urlRanks)) {
    anchorParts[name] = accessor(
      function () {
        const parts = parseUrl(this.getAttribute("href") ?? "", this.baseURI);
        return parts === null ? "" : parts[rank];
      },
      function (value) {
        const parts = parseUrl(this.getAttribute("href") ?? "", this.baseURI);
        if (parts === null) return;
        const changed = changeUrl(parts[HREF], rank, String(value));
        if (changed !== null) this.setAttribute("href", changed[HREF]);
      },
    );
  }
  anchorParts.origin = accessor(function () {
    const parts = parseUrl(this.getAttribute("href") ?? "", this.baseURI);
    return parts === null ? "" : parts[10];
  });
  // `<body>` : ces gestionnaires sont ceux de la fenêtre (`body.onload = …`).
  const windowHandlers = {};
  for (const kind of ["blur", "error", "focus", "load", "resize", "scroll", "scrollend", "afterprint",
    "beforeprint", "beforeunload", "hashchange", "languagechange", "message", "messageerror",
    "offline", "online", "pagehide", "pageshow", "popstate", "rejectionhandled", "storage",
    "unhandledrejection", "unload"]) {
    const name = "on" + kind;
    windowHandlers[name] = accessor(
      // Sans fenêtre (document créé par un script), rien à lire ni à poser.
      function () { return this.ownerDocument?.defaultView?.[name] ?? null; },
      function (value) {
        const view = this.ownerDocument?.defaultView;
        if (view) view[name] = value;
      },
    );
  }
  elementInterface("HTMLBodyElement", ["body"], windowHandlers);
  const linkMembers = {
    ...anchorParts,
    download: text("download"),
    hreflang: text("hreflang"),
    referrerPolicy: text("referrerpolicy"),
    text: accessor(
      function () { return this.textContent; },
      function (value) { this.textContent = String(value); },
    ),
    toString: method(function toString() { return this.href; }),
  };
  elementInterface("HTMLAnchorElement", ["a"], linkMembers);
  elementInterface("HTMLAreaElement", ["area"], linkMembers);
  elementInterface("HTMLImageElement", ["img"], {
    width: integer("width", 0),
    height: integer("height", 0),
    naturalWidth: accessor(() => 0),
    naturalHeight: accessor(() => 0),
    complete: accessor(() => true),
    currentSrc: accessor(function () { return this.src; }),
    loading: keyword("loading", ["lazy", "eager"], "eager"),
    decoding: keyword("decoding", ["sync", "async", "auto"], "auto"),
    crossOrigin: text("crossorigin"),
    srcset: text("srcset"),
    sizes: text("sizes"),
    decode: method(function decode() { return Promise.resolve(); }),
  });
  elementInterface("HTMLScriptElement", ["script"], {
    async: flag("async"),
    defer: flag("defer"),
    noModule: flag("nomodule"),
    crossOrigin: text("crossorigin"),
    integrity: text("integrity"),
    text: accessor(
      function () { return this.textContent; },
      function (value) { this.textContent = String(value); },
    ),
  });
  elementInterface("HTMLLinkElement", ["link"], {
    as: text("as"), media: text("media"), integrity: text("integrity"),
    crossOrigin: text("crossorigin"), hreflang: text("hreflang"), disabled: flag("disabled"),
  });
  elementInterface("HTMLMetaElement", ["meta"], {
    content: text("content"), httpEquiv: text("http-equiv"), media: text("media"),
  });
  elementInterface("HTMLBaseElement", ["base"]);
  elementInterface("HTMLIFrameElement", ["iframe"], {
    width: text("width"), height: text("height"), srcdoc: text("srcdoc"),
    allow: text("allow"), loading: keyword("loading", ["lazy", "eager"], "eager"),
    contentWindow: accessor(() => null), contentDocument: accessor(() => null),
  });
  elementInterface("HTMLCanvasElement", ["canvas"], {
    width: integer("width", 300),
    height: integer("height", 150),
    // LIMITE : pas encore de contexte de dessin.
    getContext: method(function getContext() { return null; }),
  });
  const mediaMembers = {
    autoplay: flag("autoplay"), controls: flag("controls"), loop: flag("loop"),
    muted: flag("muted"), preload: text("preload"),
    paused: accessor(() => true), currentTime: accessor(() => 0, () => {}),
    duration: accessor(() => NaN), readyState: accessor(() => 0),
    // LIMITE : pas encore de lecture audio ou vidéo.
    play: method(function play() {
      return Promise.reject(new DOMException("lecture non prise en charge", "NotSupportedError"));
    }),
    pause: method(function pause() {}),
    load: method(function load() {}),
    canPlayType: method(function canPlayType() { return ""; }),
  };
  elementInterface("HTMLVideoElement", ["video"], {
    ...mediaMembers, width: integer("width", 0), height: integer("height", 0),
    poster: address("poster"),
  });
  elementInterface("HTMLAudioElement", ["audio"], mediaMembers);
  define(global, "Audio", {
    value: function Audio(source) {
      if (!new.target) throw new TypeError("Audio doit être appelé avec « new »");
      const audio = document.createElement("audio");
      if (source !== undefined) audio.setAttribute("src", String(source));
      return audio;
    },
    writable: true,
    configurable: true,
  });

  // ---- Formulaires ---------------------------------------------------------------------------------
  const ownerForm = accessor(function () { return this.closest("form"); });
  const labelsOf = accessor(function () {
    const id = this.getAttribute("id");
    return Array.from(document.querySelectorAll("label")).filter(
      (label) => label.control === this || (id !== null && label.htmlFor === id),
    );
  });
  // ---- Validation des contraintes (HTML Standard §4.10.21) -------------------------------------------
  // LIMITES : `tooLong`, `tooShort` et `badInput` valent toujours `false`
  // (ils dépendent d'une saisie de l'utilisateur, que rien ne suit ici) ;
  // `reportValidity()` donne le focus au contrôle fautif sans afficher de
  // bulle ; les messages ne sont pas traduits selon la langue de la page.
  const customErrors = new WeakMap();
  const typeOf = (control) => (control.localName === "input" ? inputType(control) : control.localName);
  // Contrôles exclus de la validation.
  function barred(control) {
    const name = control.localName;
    if (name === "fieldset" || name === "output" || name === "object") return true;
    if (name === "button") {
      const type = (control.getAttribute("type") ?? "submit").toLowerCase();
      if (type === "reset" || type === "button") return true;
    }
    if (name === "input" && ["hidden", "reset", "button"].includes(inputType(control))) return true;
    if ((name === "input" || name === "textarea") && control.hasAttribute("readonly")) return true;
    if (control.hasAttribute("disabled") || control.closest("fieldset[disabled]") !== null) return true;
    return control.closest("datalist") !== null;
  }
  const NUMBER = /^-?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?$/;
  const pad = (n) => n !== undefined && /^\d+$/.test(n);
  // Instant, en millisecondes, d'une date `aaaa-mm-jj` valide.
  function dateValue(text) {
    const found = /^(\d{4,})-(\d\d)-(\d\d)$/.exec(text);
    if (found === null) return NaN;
    const [year, month, day] = [Number(found[1]), Number(found[2]), Number(found[3])];
    if (year < 1 || month < 1 || month > 12 || day < 1) return NaN;
    const at = new Date(0);
    at.setUTCFullYear(year, month - 1, day);
    return at.getUTCMonth() === month - 1 && at.getUTCDate() === day ? at.getTime() : NaN;
  }
  function timeValue(text) {
    const found = /^(\d\d):(\d\d)(?::(\d\d)(?:\.(\d{1,3}))?)?$/.exec(text);
    if (found === null) return NaN;
    const [hours, minutes, seconds] = [Number(found[1]), Number(found[2]), Number(found[3] ?? 0)];
    if (hours > 23 || minutes > 59 || seconds > 59) return NaN;
    const fraction = pad(found[4]) ? Number(found[4].padEnd(3, "0")) : 0;
    return ((hours * 60 + minutes) * 60 + seconds) * 1000 + fraction;
  }
  // Valeur numérique d'un champ, selon son type ; NaN si le texte ne s'y prête pas.
  function numeric(type, text) {
    switch (type) {
      case "number":
      case "range":
        return NUMBER.test(text) ? Number(text) : NaN;
      case "date":
        return dateValue(text);
      case "time":
        return timeValue(text);
      case "datetime-local": {
        const [date, time] = text.split(/[T ]/);
        return time === undefined ? NaN : dateValue(date) + timeValue(time);
      }
      case "month": {
        const found = /^(\d{4,})-(\d\d)$/.exec(text);
        if (found === null || Number(found[1]) < 1 || Number(found[2]) < 1 || Number(found[2]) > 12) return NaN;
        return (Number(found[1]) - 1970) * 12 + Number(found[2]) - 1;
      }
      case "week": {
        const found = /^(\d{4,})-W(\d\d)$/.exec(text);
        if (found === null) return NaN;
        const [year, week] = [Number(found[1]), Number(found[2])];
        // Le lundi de la semaine 1 est celui de la semaine du 4 janvier.
        const fourth = new Date(0);
        fourth.setUTCFullYear(year, 0, 4);
        const monday = fourth.getTime() - ((fourth.getUTCDay() + 6) % 7) * 86400000;
        const last = new Date(0);
        last.setUTCFullYear(year, 11, 28);
        const weeks = Math.round((last.getTime() - ((last.getUTCDay() + 6) % 7) * 86400000 - monday) / 604800000) + 1;
        if (year < 1 || week < 1 || week > weeks) return NaN;
        return monday + (week - 1) * 604800000;
      }
      default:
        return NaN;
    }
  }
  // Pas par défaut et échelle du pas, par type.
  const STEPS = {
    number: [1, 1], range: [1, 1], date: [1, 86400000], month: [1, 1], week: [1, 604800000],
    time: [60, 1000], "datetime-local": [60, 1000],
  };
  const TEXTUAL = ["text", "search", "url", "tel", "email", "password"];
  const REQUIRABLE = [...TEXTUAL, "date", "month", "week", "time", "datetime-local", "number",
    "checkbox", "radio", "file"];
  const EMAIL = /^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$/;
  const valueOf = (control) => String(control.value ?? "");
  // Chaque défaut de validité, calculé à la demande.
  const FLAWS = {
    valueMissing(control) {
      if (!control.hasAttribute("required")) return false;
      const type = typeOf(control);
      if (type === "select") {
        const options = optionsOf(control);
        const chosen = options.filter(isSelected);
        if (chosen.length === 0) return true;
        // L'option d'invite : la première, vide, d'une liste déroulante simple.
        const first = options[0];
        const single = !control.hasAttribute("multiple") && (Number(control.getAttribute("size")) || 0) <= 1;
        return single && chosen[0] === first && first.parentNode === control && optionValue(first) === "";
      }
      if (type === "textarea") return valueOf(control) === "";
      if (!REQUIRABLE.includes(type)) return false;
      if (type === "checkbox") return !control.hasAttribute("checked");
      if (type === "radio") return !radioGroup(control).some((radio) => radio.hasAttribute("checked"));
      if (type === "file") return true;
      return valueOf(control) === "";
    },
    typeMismatch(control) {
      const type = typeOf(control);
      const value = valueOf(control);
      if (value === "") return false;
      if (type === "email") {
        const parts = control.hasAttribute("multiple") ? value.split(",").map((part) => part.trim()) : [value];
        return parts.some((part) => !EMAIL.test(part));
      }
      if (type === "url") return parseUrl(value.trim(), undefined) === null;
      return false;
    },
    patternMismatch(control) {
      const pattern = control.getAttribute("pattern");
      const value = valueOf(control);
      if (pattern === null || value === "" || !TEXTUAL.includes(typeOf(control))) return false;
      let matcher;
      try {
        matcher = new RegExp("^(?:" + pattern + ")$", "v");
      } catch (error) {
        return false;
      }
      const parts = typeOf(control) === "email" && control.hasAttribute("multiple")
        ? value.split(",").map((part) => part.trim()) : [value];
      return parts.some((part) => !matcher.test(part));
    },
    tooLong() { return false; },
    tooShort() { return false; },
    rangeUnderflow(control) {
      const type = typeOf(control);
      const value = numeric(type, valueOf(control));
      const min = numeric(type, control.getAttribute("min") ?? "");
      return !Number.isNaN(value) && !Number.isNaN(min) && value < min;
    },
    rangeOverflow(control) {
      const type = typeOf(control);
      const value = numeric(type, valueOf(control));
      const max = numeric(type, control.getAttribute("max") ?? "");
      return !Number.isNaN(value) && !Number.isNaN(max) && value > max;
    },
    stepMismatch(control) {
      const type = typeOf(control);
      const rule = STEPS[type];
      const value = numeric(type, valueOf(control));
      if (rule === undefined || Number.isNaN(value)) return false;
      const written = (control.getAttribute("step") ?? "").trim();
      if (written.toLowerCase() === "any") return false;
      let step = NUMBER.test(written) && Number(written) > 0 ? Number(written) : rule[0];
      if (type !== "number" && type !== "range") step = Math.max(1, Math.round(step));
      step *= rule[1];
      let base = numeric(type, control.getAttribute("min") ?? "");
      if (Number.isNaN(base)) base = numeric(type, control.getAttribute("value") ?? "");
      if (Number.isNaN(base)) base = type === "week" ? -259200000 : 0;
      const ratio = (value - base) / step;
      return Math.abs(ratio - Math.round(ratio)) > 1e-7 * Math.max(1, Math.abs(ratio));
    },
    badInput() { return false; },
    customError(control) { return (customErrors.get(control) ?? "") !== ""; },
  };
  const MESSAGES = {
    valueMissing: "Veuillez renseigner ce champ.",
    typeMismatch: "La valeur n'a pas la forme attendue.",
    patternMismatch: "La valeur ne respecte pas le format demandé.",
    rangeUnderflow: "La valeur est trop petite.",
    rangeOverflow: "La valeur est trop grande.",
    stepMismatch: "La valeur ne tombe pas sur un pas autorisé.",
  };
  const flawsOf = (control) => (barred(control) ? []
    : Object.keys(FLAWS).filter((name) => FLAWS[name](control)));
  const states = new WeakMap();
  const owners = new WeakMap();
  const ValidityState = function ValidityState() { throw new TypeError("constructeur illégal"); };
  for (const name of Object.keys(FLAWS)) {
    define(ValidityState.prototype, name, {
      get() { return FLAWS[name](owners.get(this)); }, enumerable: true, configurable: true,
    });
  }
  define(ValidityState.prototype, "valid", {
    get() { return Object.keys(FLAWS).every((name) => !FLAWS[name](owners.get(this))); },
    enumerable: true, configurable: true,
  });
  define(ValidityState.prototype, Symbol.toStringTag, { value: "ValidityState", configurable: true });
  define(global, "ValidityState", { value: ValidityState, writable: true, enumerable: false, configurable: true });
  // Vérifie `control` : faux, après un événement `invalid`, s'il a un défaut.
  // `seen` : appelé avec le contrôle si personne n'a annulé l'événement.
  function check(control, seen) {
    if (flawsOf(control).length === 0) return true;
    const proceed = control.dispatchEvent(new Event("invalid", { cancelable: true }));
    if (proceed && seen !== undefined) seen(control);
    return false;
  }
  const validity = {
    willValidate: accessor(function () { return !barred(this); }),
    validity: accessor(function () {
      let state = states.get(this);
      if (state === undefined) {
        state = Object.create(ValidityState.prototype);
        owners.set(state, this);
        states.set(this, state);
      }
      return state;
    }),
    validationMessage: accessor(function () {
      const flaws = flawsOf(this);
      if (flaws.length === 0) return "";
      if (flaws.includes("customError")) return customErrors.get(this);
      return MESSAGES[flaws[0]] ?? "La valeur est invalide.";
    }),
    checkValidity: method(function checkValidity() { return check(this); }),
    reportValidity: method(function reportValidity() {
      return check(this, (control) => control.focus?.());
    }),
    setCustomValidity: method(function setCustomValidity(message) {
      if (arguments.length === 0) throw new TypeError("setCustomValidity : un message est attendu");
      customErrors.set(this, String(message));
    }),
  };
  // Valide tous les contrôles de `form` ; `report` : le premier fautif dont
  // l'événement `invalid` n'a pas été annulé reçoit le focus.
  function validateForm(form, report) {
    let first = null;
    let valid = true;
    for (const control of controlsOf(form)) {
      if (!check(control, (flawed) => { first ??= flawed; })) valid = false;
    }
    if (report && first !== null) first.focus?.();
    return valid;
  }
  const isButtonType = (type) => ["submit", "image", "button", "reset"].includes(type);
  const inputType = (control) => (control.getAttribute("type") ?? "text").toLowerCase();

  // Contrôles rattachés à `form`, dans l'ordre du document.
  function controlsOf(form) {
    return Array.from(form.querySelectorAll("input, select, textarea, button, fieldset, output"))
      .filter((control) => control.closest("form") === form);
  }
  function optionsOf(select) {
    return Array.from(select.querySelectorAll("option"));
  }
  function isSelected(option) {
    if (option.hasAttribute("selected")) return true;
    const select = option.closest("select");
    if (select === null || select.hasAttribute("multiple")) return false;
    // Liste à choix unique sans option choisie : la première l'est.
    const options = optionsOf(select);
    return !options.some((o) => o.hasAttribute("selected"))
      && options.find((o) => !o.hasAttribute("disabled")) === option;
  }
  function optionValue(option) {
    return option.getAttribute("value") ?? option.textContent.trim().replace(/\s+/g, " ");
  }

  // « Constructing the entry list » (HTML Standard §4.10.21.4).
  function entryList(form, submitter) {
    const list = [];
    for (const control of controlsOf(form)) {
      const tagName = control.localName;
      if (tagName === "fieldset" || tagName === "output") continue;
      if (control.hasAttribute("disabled") || control.closest("fieldset[disabled]") !== null) continue;
      const type = inputType(control);
      const isButton = tagName === "button" || (tagName === "input" && isButtonType(type));
      if (isButton && control !== submitter) continue;
      if (tagName === "input" && (type === "checkbox" || type === "radio")
        && !control.hasAttribute("checked")) continue;
      if (tagName === "input" && type === "file") continue;
      const name = control.getAttribute("name");
      if (name === null || name === "") continue;
      if (tagName === "select") {
        for (const option of optionsOf(control)) {
          if (isSelected(option) && !option.hasAttribute("disabled")) {
            list.push([name, optionValue(option)]);
          }
        }
      } else if (tagName === "input" && (type === "checkbox" || type === "radio")) {
        list.push([name, control.getAttribute("value") ?? "on"]);
      } else {
        list.push([name, String(control.value ?? "")]);
      }
    }
    return list;
  }
  const formEncode = (value) => encodeURIComponent(value)
    .replace(/%20/g, "+")
    .replace(/[!'()~]/g, (c) => "%" + c.charCodeAt(0).toString(16).toUpperCase());
  const urlEncoded = (list) => list.map(([n, v]) => formEncode(n) + "=" + formEncode(v)).join("&");

  // Soumet `form` : navigation vers son `action`, les données dans l'adresse
  // (GET) ou dans le corps (POST).
  function submitForm(form, submitter) {
    const list = entryList(form, submitter);
    const pick = (own, fallback) => (submitter !== null && submitter.hasAttribute(own)
      ? submitter.getAttribute(own)
      : form.getAttribute(fallback));
    const requestMethod = (pick("formmethod", "method") ?? "get").toLowerCase();
    // `method=dialog` : ferme la boîte de dialogue qui contient le formulaire.
    if (requestMethod === "dialog") {
      const dialog = form.closest("dialog");
      if (dialog !== null) {
        closeDialog(dialog, submitter === null ? undefined : submitter.getAttribute("value") ?? "");
      }
      return;
    }
    const action = pick("formaction", "action") ?? "";
    const parts = parseUrl(action === "" ? locationHref() : action, form.baseURI);
    if (parts === null) return;
    if (requestMethod === "post") {
      navigate("post", parts[HREF], urlEncoded(list));
    } else {
      const target = changeUrl(parts[HREF], SEARCH, urlEncoded(list));
      navigate("assign", target === null ? parts[HREF] : target[HREF]);
    }
  }

  const SubmitEvent = global.SubmitEvent;
  elementInterface("HTMLFormElement", ["form"], {
    acceptCharset: text("accept-charset"),
    action: address("action"),
    autocomplete: keyword("autocomplete", ["on", "off"], "on"),
    enctype: keyword("enctype",
      ["application/x-www-form-urlencoded", "multipart/form-data", "text/plain"],
      "application/x-www-form-urlencoded"),
    method: keyword("method", ["get", "post", "dialog"], "get"),
    noValidate: flag("novalidate"),
    // LIMITE : un tableau figé au moment de la lecture, pas une collection
    // vivante ; ses contrôles se lisent aussi par nom ou identifiant.
    elements: accessor(function () {
      const controls = controlsOf(this);
      const named = (name) => controls.find((control) => control.id === name
        || control.getAttribute("name") === name) ?? null;
      for (const control of controls) {
        for (const key of [control.id, control.getAttribute("name")]) {
          if (key && !(key in controls)) controls[key] = control;
        }
      }
      define(controls, "namedItem", { value: (name) => named(String(name)), enumerable: false });
      define(controls, "item", { value: (index) => controls[index] ?? null, enumerable: false });
      return controls;
    }),
    length: accessor(function () { return controlsOf(this).length; }),
    submit: method(function submit() { submitForm(this, null); }),
    requestSubmit: method(function requestSubmit(submitter = null) {
      // Validation avant l'envoi, sauf `novalidate` ou `formnovalidate`.
      const skip = this.hasAttribute("novalidate")
        || (submitter !== null && submitter.hasAttribute?.("formnovalidate"));
      if (!skip && !validateForm(this, true)) return;
      const event = new SubmitEvent("submit", { bubbles: true, cancelable: true, submitter });
      if (this.dispatchEvent(event)) submitForm(this, submitter);
    }),
    reset: method(function reset() {
      this.dispatchEvent(new Event("reset", { bubbles: true, cancelable: true }));
    }),
    checkValidity: method(function checkValidity() { return validateForm(this, false); }),
    reportValidity: method(function reportValidity() { return validateForm(this, true); }),
  });

  elementInterface("HTMLInputElement", ["input"], {
    ...validity,
    form: ownerForm,
    labels: labelsOf,
    checked: flag("checked"),
    defaultChecked: flag("checked"),
    defaultValue: text("value"),
    disabled: flag("disabled"),
    readOnly: flag("readonly"),
    required: flag("required"),
    multiple: flag("multiple"),
    autocomplete: text("autocomplete"),
    accept: text("accept"),
    min: text("min"),
    max: text("max"),
    step: text("step"),
    pattern: text("pattern"),
    maxLength: integer("maxlength", -1),
    minLength: integer("minlength", -1),
    size: integer("size", 20),
    files: accessor(() => null),
    valueAsNumber: accessor(
      function () { return this.value === "" ? NaN : Number(this.value); },
      function (value) { this.value = String(value); },
    ),
    select: method(function select() {}),
    setSelectionRange: method(function setSelectionRange() {}),
  });
  elementInterface("HTMLButtonElement", ["button"], {
    ...validity, form: ownerForm, labels: labelsOf, disabled: flag("disabled"),
    formAction: address("formaction"), formMethod: text("formmethod"),
  });
  elementInterface("HTMLTextAreaElement", ["textarea"], {
    ...validity,
    form: ownerForm,
    labels: labelsOf,
    value: accessor(
      function () { return this.textContent; },
      function (value) { this.textContent = String(value); },
    ),
    defaultValue: accessor(
      function () { return this.textContent; },
      function (value) { this.textContent = String(value); },
    ),
    disabled: flag("disabled"),
    readOnly: flag("readonly"),
    required: flag("required"),
    rows: integer("rows", 2),
    cols: integer("cols", 20),
    maxLength: integer("maxlength", -1),
    textLength: accessor(function () { return this.textContent.length; }),
    select: method(function select() {}),
    setSelectionRange: method(function setSelectionRange() {}),
  });
  elementInterface("HTMLSelectElement", ["select"], {
    ...validity,
    form: ownerForm,
    labels: labelsOf,
    disabled: flag("disabled"),
    multiple: flag("multiple"),
    required: flag("required"),
    size: integer("size", 0),
    options: accessor(function () { return optionsOf(this); }),
    length: accessor(function () { return optionsOf(this).length; }),
    selectedOptions: accessor(function () { return optionsOf(this).filter(isSelected); }),
    selectedIndex: accessor(
      function () { return optionsOf(this).findIndex(isSelected); },
      function (index) {
        optionsOf(this).forEach((option, i) => option.toggleAttribute("selected", i === Number(index)));
      },
    ),
    value: accessor(
      function () {
        const chosen = optionsOf(this).find(isSelected);
        return chosen === undefined ? "" : optionValue(chosen);
      },
      function (value) {
        const wanted = String(value);
        let found = false;
        for (const option of optionsOf(this)) {
          const match = !found && optionValue(option) === wanted;
          option.toggleAttribute("selected", match);
          found ||= match;
        }
      },
    ),
    item: method(function item(index) { return optionsOf(this)[index] ?? null; }),
    add: method(function add(option, before = null) {
      this.insertBefore(option, typeof before === "number" ? optionsOf(this)[before] ?? null : before);
    }),
  });
  elementInterface("HTMLOptionElement", ["option"], {
    form: ownerForm,
    disabled: flag("disabled"),
    defaultSelected: flag("selected"),
    label: accessor(
      function () { return this.getAttribute("label") ?? this.textContent; },
      function (value) { this.setAttribute("label", String(value)); },
    ),
    selected: accessor(
      function () { return isSelected(this); },
      function (value) {
        const select = this.closest("select");
        if (value && select !== null && !select.hasAttribute("multiple")) {
          for (const option of optionsOf(select)) option.removeAttribute("selected");
        }
        this.toggleAttribute("selected", Boolean(value));
      },
    ),
    value: accessor(
      function () { return optionValue(this); },
      function (value) { this.setAttribute("value", String(value)); },
    ),
    text: accessor(
      function () { return this.textContent.trim().replace(/\s+/g, " "); },
      function (value) { this.textContent = String(value); },
    ),
    index: accessor(function () {
      const select = this.closest("select");
      return select === null ? 0 : optionsOf(select).indexOf(this);
    }),
  });
  elementInterface("HTMLOptGroupElement", ["optgroup"], { disabled: flag("disabled"), label: text("label") });
  elementInterface("HTMLLabelElement", ["label"], {
    form: ownerForm,
    htmlFor: text("for"),
    control: accessor(function () {
      const target = this.getAttribute("for");
      if (target !== null) return document.getElementById(target);
      return this.querySelector("input, select, textarea, button");
    }),
  });

  // ---- FormData (XMLHttpRequest Standard §5.3) -----------------------------------------------------------
  class FormData {
    #list = [];
    constructor(form, submitter = null) {
      if (form === undefined) return;
      if (form === null || form.localName !== "form") {
        throw new TypeError("FormData : un élément <form> est attendu");
      }
      this.#list = entryList(form, submitter);
    }
    append(name, value) { this.#list.push([String(name), String(value)]); }
    delete(name) {
      const wanted = String(name);
      this.#list = this.#list.filter(([n]) => n !== wanted);
    }
    get(name) {
      const wanted = String(name);
      const found = this.#list.find(([n]) => n === wanted);
      return found ? found[1] : null;
    }
    getAll(name) {
      const wanted = String(name);
      return this.#list.filter(([n]) => n === wanted).map(([, v]) => v);
    }
    has(name) {
      const wanted = String(name);
      return this.#list.some(([n]) => n === wanted);
    }
    set(name, value) {
      const wanted = String(name);
      const first = this.#list.findIndex(([n]) => n === wanted);
      if (first < 0) {
        this.#list.push([wanted, String(value)]);
      } else {
        this.#list[first][1] = String(value);
        this.#list = this.#list.filter(([n], index) => n !== wanted || index === first);
      }
    }
    forEach(callback, thisArg) {
      for (const [name, value] of this.#list) callback.call(thisArg, value, name, this);
    }
    *entries() { for (const [name, value] of this.#list) yield [name, value]; }
    *keys() { for (const [name] of this.#list) yield name; }
    *values() { for (const [, value] of this.#list) yield value; }
    [Symbol.iterator]() { return this.entries(); }
  }
  define(FormData.prototype, Symbol.toStringTag, { value: "FormData", configurable: true });
  define(global, "FormData", { value: FormData, writable: true, configurable: true });

  // ---- Activation au clic (HTML Standard §6.5) -----------------------------------------------------------------
  // Contrôle activé par un clic sur `target` : lui-même, celui de son
  // étiquette, ou le bouton qui le contient.
  function activated(target) {
    const element = target.nodeType === 1 ? target : target.parentElement;
    if (element === null || element === undefined) return null;
    const found = element.closest("input, button, label");
    if (found === null) return null;
    const control = found.localName === "label" ? found.control : found;
    if (control === null || control.hasAttribute("disabled")) return null;
    return control;
  }
  // Portée d'un groupe de boutons radio : le formulaire du bouton, à défaut
  // son arbre (le document, ou son arbre fantôme).
  const radioScope = (radio) => {
    const form = radio.closest("form");
    if (form !== null) return form;
    const tree = radio.getRootNode();
    return typeof tree.querySelectorAll === "function" ? tree : document;
  };
  function radioGroup(radio) {
    const name = radio.getAttribute("name");
    if (name === null || name === "") return [radio];
    const scope = radioScope(radio);
    return Array.from(scope.querySelectorAll("input")).filter(
      (other) => inputType(other) === "radio" && other.getAttribute("name") === name
        && radioScope(other) === scope,
    );
  }
  // Un contrôle détaché du document n'annonce pas son changement.
  const changed = (control) => {
    if (!control.isConnected) return;
    control.dispatchEvent(new Event("input", { bubbles: true }));
    control.dispatchEvent(new Event("change", { bubbles: true }));
  };
  return {
    // Entrée dans un champ de texte : soumission implicite du formulaire,
    // par son bouton d'envoi s'il en a un.
    enter(control) {
      const form = control.closest("form");
      if (form === null) return;
      const submitter = Array.from(form.querySelectorAll("button, input")).find((candidate) => {
        if (candidate.hasAttribute("disabled")) return false;
        const type = candidate.localName === "button"
          ? (candidate.getAttribute("type") ?? "submit").toLowerCase()
          : inputType(candidate);
        return type === "submit" || type === "image";
      });
      if (submitter !== undefined) submitter.click();
      else form.requestSubmit();
    },
    // Entrée ou Espace sur une liste déroulante : elle se déroule ou se
    // referme.
    toggled(control) {
      toggleSelect(control);
    },
    // Flèche sur une liste déroulante : l'option voisine devient la choisie.
    stepped(control, step) {
      if (control.hasAttribute("disabled")) return;
      const options = optionsOf(control).filter((option) => !option.hasAttribute("disabled"));
      const current = options.findIndex(isSelected);
      const next = Math.min(Math.max(current + step, 0), options.length - 1);
      if (next === current || options.length === 0) return;
      for (const option of options) option.removeAttribute("selected");
      options[next].setAttribute("selected", "");
      changed(control);
    },
    // Le moteur a édité une zone de texte (frappe, collage) : son contenu
    // devient `value`.
    wrote(control, value) {
      control.textContent = value;
      control.dispatchEvent(new Event("input", { bubbles: true }));
    },
    // Avant `click` : une case ou un bouton radio change d'état tout de
    // suite, les écouteurs voient donc le nouvel état.
    before(target) {
      const control = activated(target);
      if (control === null) return null;
      const type = control.localName === "input" ? inputType(control) : "";
      if (type === "checkbox") {
        const was = control.hasAttribute("checked");
        control.toggleAttribute("checked", !was);
        return { control, undo: () => control.toggleAttribute("checked", was) };
      }
      if (type === "radio") {
        const previous = radioGroup(control).filter((radio) => radio.hasAttribute("checked"));
        if (previous.length === 1 && previous[0] === control) return { control, undo: null, same: true };
        for (const radio of previous) radio.removeAttribute("checked");
        control.setAttribute("checked", "");
        return {
          control,
          undo: () => {
            control.removeAttribute("checked");
            for (const radio of previous) radio.setAttribute("checked", "");
          },
        };
      }
      return { control, undo: null, plain: true };
    },
    // Après `click` : l'état est rétabli si l'action a été annulée ; sinon
    // `input` et `change` sont distribués, ou le formulaire est soumis.
    after(state, prevented) {
      if (state === null || state === undefined) return;
      const { control } = state;
      if (!state.plain) {
        if (prevented) {
          if (state.undo !== null) state.undo();
        } else if (!state.same) {
          changed(control);
        }
        return;
      }
      // Un contrôle qu'un écouteur vient de désactiver n'agit plus, et un
      // formulaire détaché du document ne s'envoie pas.
      if (prevented || control.hasAttribute("disabled")) return;
      const form = control.closest("form");
      if (form === null || !form.isConnected) return;
      const type = control.localName === "button"
        ? (control.getAttribute("type") ?? "submit").toLowerCase()
        : inputType(control);
      if (type === "submit" || type === "image") form.requestSubmit(control);
      else if (type === "reset") form.reset();
    },
  };
})
