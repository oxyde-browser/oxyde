# Couverture du DOM

Membres des interfaces décrites par les fichiers Web IDL d'Oxyde (`crates/oxyde_content/idl/`) que les scripts trouvent réellement, mesurés en interrogeant une page d'essai. Généré par `cargo run -p oxyde -- --dom-coverage`.

La mesure dit qu'un membre existe, pas qu'il se comporte comme le standard l'exige : c'est le rôle des Web Platform Tests, pas encore branchés. Les interfaces que l'IDL d'Oxyde ne décrit pas encore (éléments HTML particuliers, `Window`, observateurs, plages…) ne sont pas comptées.

**Total : 125 membres disponibles sur 283 (44.2 %).**

| Interface | Hérite de | Constantes | Attributs | Opérations |
|---|---|---:|---:|---:|
| `Event` | — | 4 / 4 | 7 / 13 | 3 / 5 |
| `EventTarget` | — | — | — | 2 / 3 |
| `NodeList` | — | — | 1 / 1 | 0 / 1 |
| `HTMLCollection` | — | — | 1 / 1 | 0 / 2 |
| `Node` | `EventTarget` | 18 / 18 | 13 / 14 | 5 / 15 |
| `Document` | `Node` | — | 11 / 35 | 9 / 32 |
| `DocumentType` | `Node` | — | 0 / 3 | 1 / 4 |
| `DocumentFragment` | `Node` | — | 0 / 4 | 0 / 7 |
| `Element` | `Node` | — | 13 / 26 | 15 / 42 |
| `CharacterData` | `Node` | — | 3 / 4 | 1 / 9 |
| `Text` | `CharacterData` | — | 0 / 1 | 0 / 1 |
| `Comment` | `CharacterData` | — | — | — |
| `DOMTokenList` | — | — | 2 / 2 | 4 / 7 |
| `HTMLElement` | `Element` | — | 5 / 17 | 3 / 3 |
| `CSSStyleDeclaration` | — | — | 1 / 4 | 3 / 5 |

## Membres absents

- `Event` : srcElement, eventPhase, returnValue, composed, isTrusted, timeStamp, composedPath, initEvent
- `EventTarget` : dispatchEvent
- `NodeList` : item
- `HTMLCollection` : item, namedItem
- `Node` : baseURI, getRootNode, hasChildNodes, normalize, isEqualNode, isSameNode, compareDocumentPosition, lookupPrefix, lookupNamespaceURI, isDefaultNamespace, replaceChild
- `Document` : implementation, compatMode, characterSet, charset, inputEncoding, contentType, doctype, location, domain, referrer, cookie, lastModified, dir, images, embeds, plugins, links, forms, scripts, currentScript, defaultView, designMode, hidden, visibilityState, getElementsByTagNameNS, createElementNS, createDocumentFragment, createCDATASection, createComment, createProcessingInstruction, importNode, adoptNode, createAttribute, createAttributeNS, createEvent, createRange, createNodeIterator, createTreeWalker, getElementsByName, open, close, write, writeln, hasFocus, execCommand, replaceChildren, moveBefore
- `DocumentType` : name, publicId, systemId, before, after, replaceWith
- `DocumentFragment` : children, firstElementChild, lastElementChild, childElementCount, getElementById, prepend, append, replaceChildren, moveBefore, querySelector, querySelectorAll
- `Element` : namespaceURI, prefix, slot, attributes, shadowRoot, scrollTop, scrollLeft, scrollWidth, scrollHeight, clientTop, clientLeft, clientWidth, clientHeight, hasAttributes, getAttributeNames, getAttributeNS, setAttributeNS, removeAttributeNS, toggleAttribute, hasAttributeNS, getAttributeNode, getAttributeNodeNS, setAttributeNode, setAttributeNodeNS, removeAttributeNode, attachShadow, webkitMatchesSelector, getElementsByTagNameNS, insertAdjacentElement, insertAdjacentText, getClientRects, scroll, scrollTo, scrollBy, insertAdjacentHTML, replaceChildren, moveBefore, before, after, replaceWith
- `CharacterData` : length, substringData, appendData, insertData, deleteData, replaceData, before, after, replaceWith
- `Text` : wholeText, splitText
- `DOMTokenList` : item, replace, supports
- `HTMLElement` : translate, dir, inert, accessKey, accessKeyLabel, draggable, spellcheck, outerText, dataset, nonce, autofocus, tabIndex
- `CSSStyleDeclaration` : length, parentRule, cssFloat, item, getPropertyPriority
