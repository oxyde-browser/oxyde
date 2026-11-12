# Couverture du DOM

Membres des interfaces décrites par les fichiers Web IDL d'Oxyde (`crates/oxyde_content/idl/`) que les scripts trouvent réellement, mesurés en interrogeant une page d'essai. Généré par `cargo run -p oxyde -- --dom-coverage`.

La mesure dit qu'un membre existe, pas qu'il se comporte comme le standard l'exige : c'est le rôle des Web Platform Tests, pas encore branchés. Les interfaces que l'IDL d'Oxyde ne décrit pas encore (éléments HTML particuliers, `Window`, observateurs, plages…) ne sont pas comptées.

**Total : 246 membres disponibles sur 296 (83.1 %).**

| Interface | Hérite de | Constantes | Attributs | Opérations |
|---|---|---:|---:|---:|
| `Event` | — | 4 / 4 | 13 / 13 | 5 / 5 |
| `CustomEvent` | `Event` | — | 1 / 1 | 1 / 1 |
| `EventTarget` | — | — | — | 3 / 3 |
| `NodeList` | — | — | 1 / 1 | 1 / 1 |
| `HTMLCollection` | — | — | 1 / 1 | 2 / 2 |
| `Node` | `EventTarget` | 18 / 18 | 14 / 14 | 12 / 15 |
| `Document` | `Node` | — | 33 / 35 | 20 / 32 |
| `XMLDocument` | `Document` | — | — | — |
| `DOMImplementation` | — | — | — | 0 / 4 |
| `DocumentType` | `Node` | — | 3 / 3 | 4 / 4 |
| `DocumentFragment` | `Node` | — | 4 / 4 | 6 / 7 |
| `Element` | `Node` | — | 16 / 26 | 30 / 42 |
| `CharacterData` | `Node` | — | 4 / 4 | 9 / 9 |
| `Text` | `CharacterData` | — | 1 / 1 | 1 / 1 |
| `CDATASection` | `Text` | — | — | — |
| `ProcessingInstruction` | `CharacterData` | — | 0 / 1 | — |
| `Comment` | `CharacterData` | — | — | — |
| `DOMTokenList` | — | — | 2 / 2 | 7 / 7 |
| `HTMLElement` | `Element` | — | 17 / 17 | 3 / 3 |
| `CSSStyleDeclaration` | — | — | 1 / 4 | 3 / 5 |
| `Storage` | — | — | 1 / 1 | 5 / 5 |

## Membres absents

- `Node` : lookupPrefix, lookupNamespaceURI, isDefaultNamespace
- `Document` : cookie, lastModified, getElementsByTagNameNS, createAttribute, createAttributeNS, createRange, createNodeIterator, createTreeWalker, open, close, write, writeln, execCommand, moveBefore
- `DOMImplementation` : createDocumentType, createDocument, createHTMLDocument, hasFeature
- `DocumentFragment` : moveBefore
- `Element` : attributes, shadowRoot, scrollTop, scrollLeft, scrollWidth, scrollHeight, clientTop, clientLeft, clientWidth, clientHeight, getAttributeNode, getAttributeNodeNS, setAttributeNode, setAttributeNodeNS, removeAttributeNode, attachShadow, getElementsByTagNameNS, getClientRects, scroll, scrollTo, scrollBy, moveBefore
- `ProcessingInstruction` : target
- `CSSStyleDeclaration` : length, parentRule, cssFloat, item, getPropertyPriority
