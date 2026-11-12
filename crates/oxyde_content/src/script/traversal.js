// Parcours de l'arbre et sélection (DOM Standard §6, Selection API) :
// `NodeFilter`, `NodeIterator`, `TreeWalker`, `Selection`. Le script rend une
// fabrique appelée avec les opérations que seul le moteur sait faire :
//   iterNew(racine)          -> identifiant d'un itérateur suivi par le document ;
//   iterGet(id)              -> [nœud de référence, pointeur avant ce nœud] ;
//   iterSet(id, nœud, avant) -> déplace l'itérateur.
// Le document ajuste lui-même la référence d'un itérateur quand son nœud est
// retiré de l'arbre.
//
// LIMITES CONNUES : `Selection` ne garde qu'une plage et ignore le sens de la
// sélection ; rien n'est dessiné à l'écran.
(function (iterNew, iterGet, iterSet) {
  "use strict";
  const global = globalThis;
  const define = Object.defineProperty;
  const expose = (name, value) => {
    define(global, name, { value, writable: true, enumerable: false, configurable: true });
  };
  const tag = (ctor, name) => {
    define(ctor.prototype, Symbol.toStringTag, { value: name, configurable: true });
  };
  const illegal = () => { throw new TypeError("constructeur illégal"); };
  const isNode = (value) => value instanceof global.Node;
  const needNode = (value) => {
    if (!isNode(value)) throw new TypeError("un nœud du DOM est attendu en argument");
    return value;
  };

  // ---- NodeFilter ----------------------------------------------------------------------
  const FILTER_ACCEPT = 1, FILTER_REJECT = 2;
  function NodeFilter() { illegal(); }
  const constants = {
    FILTER_ACCEPT: 1, FILTER_REJECT: 2, FILTER_SKIP: 3,
    SHOW_ALL: 0xFFFFFFFF, SHOW_ELEMENT: 0x1, SHOW_ATTRIBUTE: 0x2, SHOW_TEXT: 0x4,
    SHOW_CDATA_SECTION: 0x8, SHOW_ENTITY_REFERENCE: 0x10, SHOW_ENTITY: 0x20,
    SHOW_PROCESSING_INSTRUCTION: 0x40, SHOW_COMMENT: 0x80, SHOW_DOCUMENT: 0x100,
    SHOW_DOCUMENT_TYPE: 0x200, SHOW_DOCUMENT_FRAGMENT: 0x400, SHOW_NOTATION: 0x800,
  };
  for (const [name, value] of Object.entries(constants)) {
    define(NodeFilter, name, { value, enumerable: true });
  }
  delete NodeFilter.prototype;
  expose("NodeFilter", NodeFilter);

  // « Filter » : ce que le parcours fait du nœud.
  function filterNode(state, node) {
    if (state.active) {
      throw new global.DOMException("le filtre est déjà en cours d'appel", "InvalidStateError");
    }
    if (!((1 << (node.nodeType - 1)) & state.whatToShow)) return 3;
    const filter = state.filter;
    if (filter === null) return FILTER_ACCEPT;
    state.active = true;
    let result;
    try {
      if (typeof filter === "function") {
        result = filter.call(undefined, node);
      } else {
        const accept = filter.acceptNode;
        if (typeof accept !== "function") throw new TypeError("acceptNode n'est pas une fonction");
        result = accept.call(filter, node);
      }
    } finally {
      state.active = false;
    }
    return result & 0xFFFF;
  }
  const filterArg = (value) => {
    if (value === null || value === undefined) return null;
    if (typeof value !== "object" && typeof value !== "function") {
      throw new TypeError("le filtre doit être une fonction ou un objet");
    }
    return value;
  };
  const showArg = (value) => (value === undefined ? 0xFFFFFFFF : value >>> 0);

  // ---- NodeIterator --------------------------------------------------------------------
  const iterators = new WeakMap();
  const iteratorState = (object) => {
    const state = iterators.get(object);
    if (!state) throw new TypeError("receveur attendu : un NodeIterator");
    return state;
  };
  const following = (node, root) => {
    if (node.firstChild) return node.firstChild;
    while (node && node !== root) {
      if (node.nextSibling) return node.nextSibling;
      node = node.parentNode;
    }
    return null;
  };
  const preceding = (node, root) => {
    if (node === root) return null;
    let previous = node.previousSibling;
    if (!previous) return node.parentNode;
    while (previous.lastChild) previous = previous.lastChild;
    return previous;
  };
  function traverse(object, forward) {
    const state = iteratorState(object);
    let [node, before] = iterGet(state.id);
    for (;;) {
      if (forward) {
        if (!before) {
          node = following(node, state.root);
          if (!node) return null;
        } else {
          before = false;
        }
      } else if (before) {
        node = preceding(node, state.root);
        if (!node) return null;
      } else {
        before = true;
      }
      if (filterNode(state, node) === FILTER_ACCEPT) break;
    }
    iterSet(state.id, node, before);
    return node;
  }
  class NodeIterator {
    constructor() { illegal(); }
    get root() { return iteratorState(this).root; }
    get referenceNode() { return iterGet(iteratorState(this).id)[0]; }
    get pointerBeforeReferenceNode() { return iterGet(iteratorState(this).id)[1]; }
    get whatToShow() { return iteratorState(this).whatToShow; }
    get filter() { return iteratorState(this).filter; }
    nextNode() { return traverse(this, true); }
    previousNode() { return traverse(this, false); }
    detach() { iteratorState(this); }
  }
  tag(NodeIterator, "NodeIterator");
  expose("NodeIterator", NodeIterator);

  // ---- TreeWalker ----------------------------------------------------------------------
  const walkers = new WeakMap();
  const walkerState = (object) => {
    const state = walkers.get(object);
    if (!state) throw new TypeError("receveur attendu : un TreeWalker");
    return state;
  };
  function children(state, first) {
    let node = first ? state.current.firstChild : state.current.lastChild;
    while (node) {
      const result = filterNode(state, node);
      if (result === FILTER_ACCEPT) {
        state.current = node;
        return node;
      }
      if (result === 3) {
        const child = first ? node.firstChild : node.lastChild;
        if (child) {
          node = child;
          continue;
        }
      }
      while (node) {
        const sibling = first ? node.nextSibling : node.previousSibling;
        if (sibling) {
          node = sibling;
          break;
        }
        const parent = node.parentNode;
        if (!parent || parent === state.root || parent === state.current) return null;
        node = parent;
      }
    }
    return null;
  }
  function siblings(state, next) {
    let node = state.current;
    if (node === state.root) return null;
    for (;;) {
      let sibling = next ? node.nextSibling : node.previousSibling;
      while (sibling) {
        node = sibling;
        const result = filterNode(state, node);
        if (result === FILTER_ACCEPT) {
          state.current = node;
          return node;
        }
        sibling = next ? node.firstChild : node.lastChild;
        if (result === FILTER_REJECT || !sibling) {
          sibling = next ? node.nextSibling : node.previousSibling;
        }
      }
      node = node.parentNode;
      if (!node || node === state.root) return null;
      if (filterNode(state, node) === FILTER_ACCEPT) return null;
    }
  }
  class TreeWalker {
    constructor() { illegal(); }
    get root() { return walkerState(this).root; }
    get whatToShow() { return walkerState(this).whatToShow; }
    get filter() { return walkerState(this).filter; }
    get currentNode() { return walkerState(this).current; }
    set currentNode(node) { walkerState(this).current = needNode(node); }
    parentNode() {
      const state = walkerState(this);
      let node = state.current;
      while (node && node !== state.root) {
        node = node.parentNode;
        if (node && filterNode(state, node) === FILTER_ACCEPT) {
          state.current = node;
          return node;
        }
      }
      return null;
    }
    firstChild() { return children(walkerState(this), true); }
    lastChild() { return children(walkerState(this), false); }
    previousSibling() { return siblings(walkerState(this), false); }
    nextSibling() { return siblings(walkerState(this), true); }
    previousNode() {
      const state = walkerState(this);
      let node = state.current;
      while (node !== state.root) {
        let sibling = node.previousSibling;
        while (sibling) {
          node = sibling;
          let result = filterNode(state, node);
          while (result !== FILTER_REJECT && node.lastChild) {
            node = node.lastChild;
            result = filterNode(state, node);
          }
          if (result === FILTER_ACCEPT) {
            state.current = node;
            return node;
          }
          sibling = node.previousSibling;
        }
        if (node === state.root || !node.parentNode) return null;
        node = node.parentNode;
        if (filterNode(state, node) === FILTER_ACCEPT) {
          state.current = node;
          return node;
        }
      }
      return null;
    }
    nextNode() {
      const state = walkerState(this);
      let node = state.current;
      let result = FILTER_ACCEPT;
      for (;;) {
        while (result !== FILTER_REJECT && node.firstChild) {
          node = node.firstChild;
          result = filterNode(state, node);
          if (result === FILTER_ACCEPT) {
            state.current = node;
            return node;
          }
        }
        let sibling = null;
        let temporary = node;
        while (temporary) {
          if (temporary === state.root) return null;
          sibling = temporary.nextSibling;
          if (sibling) {
            node = sibling;
            break;
          }
          temporary = temporary.parentNode;
        }
        if (!sibling) return null;
        result = filterNode(state, node);
        if (result === FILTER_ACCEPT) {
          state.current = node;
          return node;
        }
      }
    }
  }
  tag(TreeWalker, "TreeWalker");
  expose("TreeWalker", TreeWalker);

  const method = (target, name, value) => {
    define(target, name, { value, writable: true, enumerable: true, configurable: true });
  };
  method(global.Document.prototype, "createNodeIterator", function createNodeIterator(root, whatToShow, filter) {
    if (arguments.length < 1) throw new TypeError("il manque un argument");
    needNode(root);
    const object = Object.create(NodeIterator.prototype);
    iterators.set(object, {
      id: iterNew(root), root, whatToShow: showArg(whatToShow), filter: filterArg(filter), active: false,
    });
    return object;
  });
  method(global.Document.prototype, "createTreeWalker", function createTreeWalker(root, whatToShow, filter) {
    if (arguments.length < 1) throw new TypeError("il manque un argument");
    needNode(root);
    const object = Object.create(TreeWalker.prototype);
    walkers.set(object, {
      root, current: root, whatToShow: showArg(whatToShow), filter: filterArg(filter), active: false,
    });
    return object;
  });

  // ---- Selection -----------------------------------------------------------------------
  const selections = new WeakMap();
  const selected = (object) => {
    const state = selections.get(object);
    if (!state) throw new TypeError("receveur attendu : un Selection");
    return state;
  };
  const isRange = (value) => value instanceof global.Range;
  class Selection {
    constructor() { illegal(); }
    get rangeCount() { return selected(this).range ? 1 : 0; }
    get anchorNode() { const r = selected(this).range; return r ? r.startContainer : null; }
    get anchorOffset() { const r = selected(this).range; return r ? r.startOffset : 0; }
    get focusNode() { const r = selected(this).range; return r ? r.endContainer : null; }
    get focusOffset() { const r = selected(this).range; return r ? r.endOffset : 0; }
    get isCollapsed() { const r = selected(this).range; return r ? r.collapsed : true; }
    get type() {
      const r = selected(this).range;
      return !r ? "None" : r.collapsed ? "Caret" : "Range";
    }
    get direction() { return selected(this).range ? "forward" : "none"; }
    getRangeAt(index) {
      const r = selected(this).range;
      if (index >>> 0 !== 0 || !r) {
        throw new global.DOMException("aucune plage à ce rang", "IndexSizeError");
      }
      return r;
    }
    addRange(range) {
      if (!isRange(range)) throw new TypeError("une plage est attendue en argument");
      const state = selected(this);
      if (!state.range && range.startContainer.getRootNode() === global.document) state.range = range;
    }
    removeRange(range) {
      if (!isRange(range)) throw new TypeError("une plage est attendue en argument");
      const state = selected(this);
      if (state.range !== range) {
        throw new global.DOMException("cette plage n'est pas sélectionnée", "NotFoundError");
      }
      state.range = null;
    }
    removeAllRanges() { selected(this).range = null; }
    empty() { selected(this).range = null; }
    collapse(node, offset = 0) {
      const state = selected(this);
      if (node === null || node === undefined) {
        state.range = null;
        return;
      }
      const range = global.document.createRange();
      range.setStart(needNode(node), offset);
      range.collapse(true);
      state.range = range;
    }
    setPosition(node, offset = 0) { this.collapse(node, offset); }
    collapseToStart() {
      const r = selected(this).range;
      if (!r) throw new global.DOMException("la sélection est vide", "InvalidStateError");
      this.collapse(r.startContainer, r.startOffset);
    }
    collapseToEnd() {
      const r = selected(this).range;
      if (!r) throw new global.DOMException("la sélection est vide", "InvalidStateError");
      this.collapse(r.endContainer, r.endOffset);
    }
    extend(node, offset = 0) {
      const state = selected(this);
      if (!state.range) throw new global.DOMException("la sélection est vide", "InvalidStateError");
      const range = global.document.createRange();
      range.setStart(state.range.startContainer, state.range.startOffset);
      range.setEnd(needNode(node), offset);
      state.range = range;
    }
    setBaseAndExtent(anchorNode, anchorOffset, focusNode, focusOffset) {
      const range = global.document.createRange();
      range.setStart(needNode(anchorNode), anchorOffset);
      range.setEnd(needNode(focusNode), focusOffset);
      if (range.collapsed && (anchorNode !== focusNode || anchorOffset !== focusOffset)) {
        // Extrémités données à l'envers : la plage va du foyer à l'ancre.
        range.setStart(focusNode, focusOffset);
        range.setEnd(anchorNode, anchorOffset);
      }
      selected(this).range = range;
    }
    selectAllChildren(node) {
      const range = global.document.createRange();
      range.selectNodeContents(needNode(node));
      selected(this).range = range;
    }
    containsNode(node, allowPartialContainment = false) {
      const r = selected(this).range;
      if (!r || !isNode(node)) return false;
      if (allowPartialContainment) return r.intersectsNode(node);
      const parent = node.parentNode;
      if (!parent) return false;
      const index = Array.prototype.indexOf.call(parent.childNodes, node);
      return r.comparePoint(parent, index) >= 0 && r.comparePoint(parent, index + 1) <= 0
        && r.isPointInRange(parent, index) && r.isPointInRange(parent, index + 1);
    }
    deleteFromDocument() {
      const r = selected(this).range;
      if (r) r.deleteContents();
    }
    toString() {
      const r = selected(this).range;
      return r ? r.toString() : "";
    }
  }
  tag(Selection, "Selection");
  expose("Selection", Selection);
  const selection = Object.create(Selection.prototype);
  selections.set(selection, { range: null });
  const getSelection = function getSelection() { return selection; };
  method(global, "getSelection", getSelection);
  method(global.Document.prototype, "getSelection", getSelection);
})
