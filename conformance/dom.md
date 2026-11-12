# Couverture du DOM

Membres des interfaces décrites par les fichiers Web IDL d'Oxyde (`crates/oxyde_content/idl/`) que les scripts trouvent réellement, mesurés en interrogeant une page d'essai. Généré par `cargo run -p oxyde -- --dom-coverage`.

La mesure dit qu'un membre existe, pas qu'il se comporte comme le standard l'exige : c'est le rôle des Web Platform Tests (voir `wpt.md`). Les interfaces que l'IDL d'Oxyde ne décrit pas encore (éléments HTML particuliers, `Window`, observateurs, plages…) ne sont pas comptées.

**Total : 313 membres disponibles sur 342 (91.5 %).**

| Interface | Hérite de | Constantes | Attributs | Opérations |
|---|---|---:|---:|---:|
| `Event` | — | 4 / 4 | 13 / 13 | 5 / 5 |
| `CustomEvent` | `Event` | — | 1 / 1 | 1 / 1 |
| `EventTarget` | — | — | — | 3 / 3 |
| `NodeList` | — | — | 1 / 1 | 1 / 1 |
| `HTMLCollection` | — | — | 1 / 1 | 2 / 2 |
| `Node` | `EventTarget` | 18 / 18 | 14 / 14 | 15 / 15 |
| `Document` | `Node` | — | 33 / 35 | 26 / 32 |
| `XMLDocument` | `Document` | — | — | — |
| `DOMImplementation` | — | — | — | 4 / 4 |
| `DocumentType` | `Node` | — | 3 / 3 | 4 / 4 |
| `DocumentFragment` | `Node` | — | 4 / 4 | 6 / 7 |
| `Element` | `Node` | — | 17 / 26 | 36 / 42 |
| `NamedNodeMap` | — | — | 1 / 1 | 7 / 7 |
| `Attr` | `Node` | — | 7 / 7 | — |
| `CharacterData` | `Node` | — | 4 / 4 | 9 / 9 |
| `Text` | `CharacterData` | — | 1 / 1 | 1 / 1 |
| `CDATASection` | `Text` | — | — | — |
| `ProcessingInstruction` | `CharacterData` | — | 1 / 1 | — |
| `Comment` | `CharacterData` | — | — | — |
| `AbstractRange` | — | — | 5 / 5 | — |
| `Range` | `AbstractRange` | 4 / 4 | 1 / 1 | 21 / 21 |
| `DOMTokenList` | — | — | 2 / 2 | 7 / 7 |
| `HTMLElement` | `Element` | — | 17 / 17 | 3 / 3 |
| `CSSStyleDeclaration` | — | — | 1 / 4 | 3 / 5 |
| `Storage` | — | — | 1 / 1 | 5 / 5 |

## Membres absents

- `Document` : cookie, lastModified, open, close, write, writeln, execCommand, moveBefore
- `DocumentFragment` : moveBefore
- `Element` : shadowRoot, scrollTop, scrollLeft, scrollWidth, scrollHeight, clientTop, clientLeft, clientWidth, clientHeight, attachShadow, getClientRects, scroll, scrollTo, scrollBy, moveBefore
- `CSSStyleDeclaration` : length, parentRule, cssFloat, item, getPropertyPriority
