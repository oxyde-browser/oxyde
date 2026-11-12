# Couverture du DOM

Membres des interfaces décrites par les fichiers Web IDL d'Oxyde (`crates/oxyde_content/idl/`) que les scripts trouvent réellement, mesurés en interrogeant une page d'essai. Généré par `cargo run -p oxyde -- --dom-coverage`.

La mesure dit qu'un membre existe, pas qu'il se comporte comme le standard l'exige : c'est le rôle des Web Platform Tests, pas encore branchés. Les interfaces que l'IDL d'Oxyde ne décrit pas encore (éléments HTML particuliers, `Window`, observateurs, plages…) ne sont pas comptées.

**Total : 209 membres disponibles sur 283 (73.9 %).**

| Interface | Hérite de | Constantes | Attributs | Opérations |
|---|---|---:|---:|---:|
| `Event` | — | 4 / 4 | 7 / 13 | 3 / 5 |
| `EventTarget` | — | — | — | 2 / 3 |
| `NodeList` | — | — | 1 / 1 | 0 / 1 |
| `HTMLCollection` | — | — | 1 / 1 | 0 / 2 |
| `Node` | `EventTarget` | 18 / 18 | 14 / 14 | 12 / 15 |
| `Document` | `Node` | — | 31 / 35 | 16 / 32 |
| `DocumentType` | `Node` | — | 3 / 3 | 4 / 4 |
| `DocumentFragment` | `Node` | — | 0 / 4 | 0 / 7 |
| `Element` | `Node` | — | 16 / 26 | 30 / 42 |
| `CharacterData` | `Node` | — | 4 / 4 | 9 / 9 |
| `Text` | `CharacterData` | — | 1 / 1 | 1 / 1 |
| `Comment` | `CharacterData` | — | — | — |
| `DOMTokenList` | — | — | 2 / 2 | 7 / 7 |
| `HTMLElement` | `Element` | — | 16 / 17 | 3 / 3 |
| `CSSStyleDeclaration` | — | — | 1 / 4 | 3 / 5 |

## Membres absents

- `Event` : srcElement, eventPhase, returnValue, composed, isTrusted, timeStamp, composedPath, initEvent
- `EventTarget` : dispatchEvent
- `NodeList` : item
- `HTMLCollection` : item, namedItem
- `Node` : lookupPrefix, lookupNamespaceURI, isDefaultNamespace
- `Document` : implementation, location, cookie, lastModified, getElementsByTagNameNS, createDocumentFragment, createCDATASection, createProcessingInstruction, createAttribute, createAttributeNS, createEvent, createRange, createNodeIterator, createTreeWalker, open, close, write, writeln, execCommand, moveBefore
- `DocumentFragment` : children, firstElementChild, lastElementChild, childElementCount, getElementById, prepend, append, replaceChildren, moveBefore, querySelector, querySelectorAll
- `Element` : attributes, shadowRoot, scrollTop, scrollLeft, scrollWidth, scrollHeight, clientTop, clientLeft, clientWidth, clientHeight, getAttributeNode, getAttributeNodeNS, setAttributeNode, setAttributeNodeNS, removeAttributeNode, attachShadow, getElementsByTagNameNS, getClientRects, scroll, scrollTo, scrollBy, moveBefore
- `HTMLElement` : dataset
- `CSSStyleDeclaration` : length, parentRule, cssFloat, item, getPropertyPriority
