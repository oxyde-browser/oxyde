// Interfaces des éléments HTML (HTML Standard §4) et formulaires, écrites en
// JavaScript. Le script rend une fabrique qui reçoit :
//   registerInterface(balise, prototype) -> prototype des éléments de cette balise ;
//   parseUrl, changeUrl, locationHref, navigate -> voir le prélude.
// Elle rend les fonctions d'activation que le moteur appelle autour d'un clic.
//
// LIMITES CONNUES : l'état d'un contrôle (valeur, case cochée, option
// choisie) vit dans ses attributs ; un contrôle est rattaché au formulaire
// qui le contient (l'attribut `form` est ignoré) ; pas de fichiers ; la
// validation des contraintes accepte tout ; une liste déroulante ne se
// déroule pas (les flèches changent l'option) ; la frappe dans une zone de
// texte se fait en fin de texte.
(function (registerInterface, parseUrl, changeUrl, locationHref, navigate) {
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
    ["HTMLDialogElement", ["dialog"]], ["HTMLTimeElement", ["time"]],
  ]) {
    elementInterface(name, tags);
  }

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
  for (const kind of ["blur", "error", "focus", "load", "resize", "scroll", "afterprint",
    "beforeprint", "beforeunload", "hashchange", "languagechange", "message", "messageerror",
    "offline", "online", "pagehide", "pageshow", "popstate", "rejectionhandled", "storage",
    "unhandledrejection", "unload"]) {
    const name = "on" + kind;
    windowHandlers[name] = accessor(
      function () { return (this.ownerDocument?.defaultView ?? global)[name] ?? null; },
      function (value) { (this.ownerDocument?.defaultView ?? global)[name] = value; },
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
  const validity = {
    willValidate: accessor(() => true),
    validationMessage: accessor(() => ""),
    checkValidity: method(function checkValidity() { return true; }),
    reportValidity: method(function reportValidity() { return true; }),
    setCustomValidity: method(function setCustomValidity() {}),
  };
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
    if (requestMethod === "dialog") return;
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
      const event = new SubmitEvent("submit", { bubbles: true, cancelable: true, submitter });
      if (this.dispatchEvent(event)) submitForm(this, submitter);
    }),
    reset: method(function reset() {
      this.dispatchEvent(new Event("reset", { bubbles: true, cancelable: true }));
    }),
    checkValidity: method(function checkValidity() { return true; }),
    reportValidity: method(function reportValidity() { return true; }),
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
  function radioGroup(radio) {
    const name = radio.getAttribute("name");
    if (name === null || name === "") return [radio];
    const scope = radio.closest("form") ?? document;
    return Array.from(scope.querySelectorAll("input")).filter(
      (other) => inputType(other) === "radio" && other.getAttribute("name") === name
        && (other.closest("form") ?? document) === scope,
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
    // Frappe dans une zone de texte : en fin de texte.
    typed(control, key) {
      if (control.hasAttribute("readonly") || control.hasAttribute("disabled")) return;
      const text = control.textContent;
      let next = text;
      if (key === "Backspace") next = Array.from(text).slice(0, -1).join("");
      else if (key === "Enter") next = text + "\n";
      else if (Array.from(key).length === 1) next = text + key;
      if (next === text) return;
      control.textContent = next;
      control.dispatchEvent(new Event("input", { bubbles: true }));
    },
    // Texte collé dans une zone de texte : en fin de texte.
    pasted(control, pasted) {
      if (control.hasAttribute("readonly") || control.hasAttribute("disabled")) return;
      const added = String(pasted).replace(/\r\n?/g, "\n");
      if (added === "") return;
      control.textContent += added;
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
