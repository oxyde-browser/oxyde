# Feuille de route d'Oxyde

État au 12 novembre 2026, branche `test262-conformite`, commit `73d6cdb`.

- `oxyde_js` réussit **40 768 tests de test262 sur 48 795 (83,5 %)** ; il en reste 8 027.
- Les jalons 1 à 3 du plan (socle, premier pixel, interface de la maquette) sont terminés.
- Le jalon 4 (JavaScript) est en cours : le moteur et le DOM scriptable existent, la conformité monte.
- Les jalons 5 à 10 ne sont pas commencés.

Les nombres de tests des étapes, plus bas, datent de la mesure initiale (77,4 %) ; ce sont des
ordres de grandeur, car corriger un domaine fait souvent passer des tests rangés dans un autre.

### Avancement des étapes

| Étape | Au départ | Reste | État |
|---|---:|---:|---|
| A et B — langage, `eval` direct | 1 839 | 352 | presque terminées |
| C — intégrés courants | 1 743 | 217 | presque terminée |
| D — tampons partagés, tableaux typés | 1 024 | 1 016 | à faire |
| E — expressions rationnelles | 855 | 859 | à faire |
| F — propositions récentes | 938 | 928 | à faire |
| G — Temporal | 4 614 | 4 622 | à faire |
| Royaumes (`$262.createRealm`) | — | 33 | fait pour l'essentiel (108 tests gagnés) |

Ce qui reste dans A, B et C tient à quelques limites de fond :

- **Chaînes UTF-8** : une chaîne ne peut pas porter de substitut isolé (une moitié de paire
  UTF-16), ce qui fausse `codePointAt`, `isWellFormed`, `encodeURI` et `JSON.stringify` sur ces cas.
- **Objets de type `document.all`** (`$262.IsHTMLDDA`) : faux en condition, `typeof` valant
  `"undefined"` ; à faire avec le DOM.
- **`this` capturé par les fonctions fléchées** avant `super()` : la flèche garde la valeur
  vue à sa création.
- **Modules** : zone morte des liaisons importées, exportations ambiguës, `import()` d'un
  module en cours d'attente.
- **Tableaux creux** de plus de 2^26 éléments et itérateurs vivants de `Map` et `Set`.

## Partie 1 — Atteindre 100 % de test262

La suite mesurée couvre les dossiers `language`, `built-ins` et `annexB`. Elle contient des
fonctionnalités récentes ou encore à l'état de proposition (Temporal, `using`, `import defer`,
ShadowRealm) : viser 100 % veut dire les implémenter aussi.

Ordre retenu : d'abord ce qui sert aux sites réels et coûte peu, Temporal en dernier parce qu'il
pèse 42 % du reste à lui seul et ne bloque rien d'autre.

| Étape | Contenu | Tests | Taux visé |
|---|---|---:|---:|
| A | Traîne du langage | 1 617 | 80,7 % |
| B | `eval` direct | 222 | 81,2 % |
| C | Intégrés courants | 1 743 | 84,8 % |
| D | Tampons partagés et tableaux typés | 1 024 | 86,9 % |
| E | Expressions rationnelles | 855 | 88,6 % |
| F | Propositions récentes | 938 | 90,5 % |
| G | Temporal | 4 614 | 100 % |

### A — Traîne du langage (1 617 tests)

Ce sont des écarts dispersés, sans chantier unique :

- **Classes** (418) : membres privés installés avant le retour de `super()`, accesseurs privés
  masqués, décorateurs, champs évalués deux fois sur double `super()`.
- **Générateurs async** (96) : `yield*` doit transmettre `next(v)`, `throw()` et `return()` au
  générateur délégué, et attendre les valeurs.
- **Reste du langage** (1 047) : gabarits étiquetés (non pris en charge), objet `arguments` lié
  aux paramètres, nommage des fonctions anonymes par `&&=`, valeurs d'achèvement des
  instructions, appels terminaux, prototypes distincts pour les fonctions génératrices et async.
- **Annexe B** (56) : cas restants des fonctions dans les blocs.

### B — `eval` direct (222 tests)

`eval` est aujourd'hui toujours évalué dans la portée globale. Un `eval` direct doit voir et
créer les variables locales de l'appelant. Les variables locales vivent dans des registres : il
faut donner aux fonctions qui contiennent `eval` une portée dynamique, comme pour `with`.

### C — Intégrés courants (1 743 tests)

- **Promise** (349) : `Promise.allKeyed`, fermeture des itérateurs en cas d'erreur, capacités
  de promesse construites par une sous-classe.
- **Symbol.species** dans `Array` (194), `Promise`, `Map`, `Set`, `ArrayBuffer`.
- **Function** (191) : `toString` conforme au source, `bind` sur une cible liée, prototypes de
  `GeneratorFunction` et `AsyncFunction`.
- **Array.fromAsync** (95), **Number / Math / BigInt / JSON** (182), **Object / Reflect / Proxy**
  (124), **Map / Set** (95), **String** (93), **Date** (27), divers (263).
- **WeakRef, FinalizationRegistry, WeakMap, WeakSet** (130) : demande des références faibles
  dans le ramasse-miettes ; aujourd'hui `WeakMap` et `WeakSet` retiennent leurs clés.

### D — Tampons partagés et tableaux typés (1 024 tests)

- **SharedArrayBuffer et Atomics** (495) : absents. `Atomics.wait` et `waitAsync` supposent des
  agents concurrents, donc des fils d'exécution ; à faire avec le jalon 5 (multi-processus).
- **TypedArray, ArrayBuffer, DataView** (529) : tampons redimensionnables, `transfer`,
  `Uint8Array.fromBase64` et `toHex`, constructeurs d'espèce.

### E — Expressions rationnelles (855 tests)

- **Propriétés Unicode** `\p{…}` (464) : il faut les tables Unicode (catégories, écritures,
  propriétés binaires). Elles sont déjà prévues au plan par ICU4X, qui est en Rust pur.
- **Drapeau `v`** et **modificateurs** `(?i:…)` (221).
- **Reste** (170) : `RegExp.prototype.compile`, propriétés statiques héritées (`RegExp.$1`),
  drapeau `d` (indices), groupes nommés en double.

### F — Propositions récentes (938 tests)

- **`using` et `DisposableStack`** (338).
- **`import defer`, `import source`, attributs d'importation** (320).
- **Aides d'`Iterator` restantes** (216) : `zip`, `zipKeyed`, `chunks`, `windows`, `includes`,
  `join`.
- **ShadowRealm** (64) : demande plusieurs royaumes dans une même machine.

### G — Temporal (4 614 tests)

Sept types (`Instant`, `PlainDate`, `PlainTime`, `PlainDateTime`, `PlainYearMonth`,
`PlainMonthDay`, `ZonedDateTime`), `Duration` et `Temporal.Now`. Deux dépendances :

- la **base de fuseaux horaires IANA** pour `ZonedDateTime` ;
- les **calendriers** non grégoriens, fournis par ICU4X.

C'est le plus gros chantier isolé du moteur ; il ne bloque aucun site courant aujourd'hui.

### Règles tenues à chaque étape

- Le taux ne régresse jamais : chaque bloc est comparé test par test à la mesure précédente.
- La barrière reste verte : `cargo fmt`, `clippy -D warnings`, tests, `cargo deny`.
- Le rapport [conformance/test262.md](../conformance/test262.md) est republié à chaque palier.

## Partie 2 — La suite du navigateur

Le plan d'origine reste la référence. Ce qui suit le précise à la lumière de l'état actuel.

### Finir le jalon 4 côté web

Le moteur n'est qu'une moitié du jalon ; l'autre est ce que les pages peuvent en faire.
Branche `jalon-4-web`, empilée sur `test262-conformite`.

Fait :

- **DOM** : 241 membres disponibles sur 291 décrits par l'IDL d'Oxyde (82,8 %) ; événements
  conformes (`Event`, `CustomEvent`, capture et bouillonnement, `dispatchEvent`),
  `MutationObserver`, fragments de document, collections, `dataset`, une interface par balise
  (`HTMLInputElement`, `HTMLAnchorElement`…).
- **Réseau des pages** : `fetch` avec méthode, en-têtes et corps sous contrôle CORS
  (pré-vérification comprise), `XMLHttpRequest`, `URL`, `URLSearchParams`, `Headers`.
- **Navigation** : `location` complet, `history` (`pushState`, `popstate`), soumission des
  formulaires en GET et en POST.
- **Divers** : `localStorage`, événements clavier, `AbortController`, `TextEncoder`,
  `structuredClone`, `matchMedia`, `FormData`.

Reste :

- **Web Platform Tests** : le lanceur (`oxyde_wpt`) mesure `dom/` — 286 fichiers de test entièrement réussis sur 693 (41,3 %).
  La mesure a guidé les algorithmes de mutation du standard, les vraies `DOMException`, les
  documents multiples (`document.implementation`), `Range`, `NodeIterator`, `TreeWalker`,
  `Selection`, les espaces de noms et les nœuds `Attr`, les collections vivantes, `classList`,
  les écouteurs passifs, l'accès nommé sur `window`, `moveBefore`, puis les cadres. Restent,
  par gain décroissant : un analyseur XML (documents XML des cadres, `DOMParser`), les tests
  pilotés par WebDriver (`testdriver.js`), `Observable`, `customElements`, puis `url/` et
  `encoding/`.
- **Cadres (`<iframe>`)** : chaque cadre a son royaume JavaScript (objet global et
  interfaces du DOM à lui) et son document, dans la même arène que la page ;
  `contentWindow`, `contentDocument`, `parent`, `top`, `frameElement`, chargement de `src`
  et de `srcdoc`, gestionnaires en attribut (`onload="…"`). Limites : un cadre n'est pas
  dessiné (jalon 6) ; faute d'isolation entre processus (jalon 5), seul un cadre de même
  origine que la page est chargé ; dans le navigateur, seuls les cadres présents dans le
  HTML reçu sont chargés, pas ceux qu'un script crée ensuite.
- **Liés à la mise en page (jalon 6)** : dessin et saisie des contrôles de formulaire, focus,
  `getComputedStyle`, dimensions (`clientWidth`, `scrollTop`…), `IntersectionObserver`,
  `ResizeObserver`.
- **Stockage et réseau** : faits — `Blob`, `File`, `FileReader`, `Request` et `Response`
  constructibles ; cookies (pot RFC 6265 dans `oxyde_net`, `Secure`, `HttpOnly`,
  `SameSite`, préfixes `__Host-` et `__Secure-`, pas de cookies tiers implicites,
  `document.cookie`). Restent : l'écriture du pot et de `localStorage` sur disque (jalon 7,
  avec l'historique et les favoris) et la liste des suffixes publics.
- **Royaumes** : faits dans `oxyde_js` (108 tests de test262 gagnés) et utilisés par les
  cadres ; restent `ShadowRealm` et les quelques tests qui lisent le royaume d'une fonction
  liée ou d'un `Proxy`.

### Jalon 5 — Multi-processus

Processus navigateur, réseau, rendu par site et GPU ; `oxyde_ipc` ; bac à sable Windows
(AppContainer, jetons restreints) ; isolation des sites ; écran « Onglet planté ». C'est aussi
le moment de `SharedArrayBuffer` et des workers, qui supposent plusieurs fils.

### Jalon 6 — Mise en page complète et compositeur

Flex, grille, tableaux, flottants, positionnement, transformations, animations et transitions,
calques composités, défilement hors du fil principal. C'est le jalon qui fait passer la plupart
des sites de « lisible » à « fidèle ». Tableau de bord WPT pour le CSS et la mise en page.

Branche `jalon-6-mise-en-page`, empilée sur `jalon-4-web`.

Fait :

- **Mesure** : le lanceur WPT exécute les 2 267 tests `testharness.js` de `css-flexbox`,
  `css-grid`, `css-position`, `css-sizing`, `css-tables`, `css-box`, `css-display`,
  `css-align`, `CSS2` et `cssom-view`. Point de départ : 213 fichiers entièrement réussis
  (9,4 %), aujourd'hui 512 (22,6 %); une lecture force la
  mise en page si le document a changé.
- **Propriétés CSS** analysées et calculées, pas encore toutes mises en page : `position`,
  `top`/`right`/`bottom`/`left`, `inset`, `float`, `clear`, `z-index`, `overflow`,
  `visibility`, `opacity`, la famille flex (`flex`, `flex-flow`, `justify-content`,
  `align-*`, `order`) et `gap`.

- **Tableaux** (`oxyde_layout::table`) : grille des cellules avec `colspan`/`rowspan`,
  en-tête et pied réordonnés, rangées et tableaux anonymes ; disposition automatique
  (largeurs minimale et maximale du contenu, largeurs en pixels et en pourcentage) et
  `table-layout: fixed` ; `border-spacing`, `border-collapse` (approché : une seule bordure
  par arête), `caption-side`, `vertical-align` des cellules ; attributs HTML `cellspacing`,
  `cellpadding`, `border`, `valign`, `nowrap`. Restent : `<col>`, l'alignement sur la ligne
  de base, l'arbitrage complet des bordures fusionnées.

- **Flottants** (`oxyde_layout::floats`) : `float`, `clear`, lignes de texte raccourcies le
  long des flottants, boîtes à contexte propre rangées à côté, contextes de formatage qui
  contiennent leurs flottants. Restent : les flottants imbriqués dans un élément en ligne,
  `shape-outside`.
- **Tailles définies par le contenu** : `width: min-content`, `max-content`, `fit-content` ;
  `aspect-ratio` pour les boîtes en bloc, flottantes et positionnées. Restent : ces mots-clés
  sur `height`, `min-*` et `max-*`, `aspect-ratio` sur les éléments flex et de grille.

Ordre prévu, chaque étape mesurée par la suite : positionnement et flottants ; `calc()` et
police de test Ahem (`@font-face`), dont beaucoup de tests dépendent ; flex ; tailles
intrinsèques (`min-content`, `max-content`, `fit-content`) ; tableaux ; grille ;
transformations, animations et transitions ; calques composités et défilement.

### Jalon 7 — Fonctions du navigateur

Les écrans restants de la maquette : paramètres, historique, favoris, téléchargements, infos du
site, autorisations, recherche dans la page, navigation privée, palette de commandes, mots de
passe, premier lancement, onglets horizontaux, menus contextuels, impression. S'y ajoute la
persistance : rien n'est aujourd'hui conservé d'une session à l'autre.

### Jalon 8 — Performances de JavaScript

L'interpréteur à registres suffit à la conformité, pas à la vitesse. JIT de base puis
optimisant, ramasse-miettes générationnel et concurrent, caches en ligne pour les accès aux
propriétés, WebAssembly.

### Jalon 9 — Plateforme web étendue

Stockage (IndexedDB, quotas), audio et vidéo, canvas, WebGL et WebGPU, WebRTC, service workers,
protocole DevTools et son écran, extensions, PDF, installateur et mises à jour.

### Jalon 10 — Autres systèmes

Linux, macOS, Android, iOS et iPadOS, par `oxyde_platform`. iOS restreint les moteurs tiers et
le JIT : à trancher le moment venu.

### Ordre proposé

1. Étapes A à C de test262 (jusqu'à environ 85 %) : elles profitent directement aux sites.
2. Fin du jalon 4 côté web et branchement des Web Platform Tests.
3. Jalon 6 avant le jalon 5 : la mise en page est ce qui manque le plus à l'œil, et
   l'isolation des processus ne change rien à ce qu'une page affiche.
4. Jalon 5, avec `SharedArrayBuffer` et Atomics (étape D).
5. Étapes E à G de test262 en tâche de fond, puis jalons 7 à 10 dans l'ordre.

Le point 3 inverse l'ordre du plan d'origine (5 puis 6) : c'est une proposition, à confirmer.

## Dettes connues à ne pas oublier

- Un fil du pilote graphique AMD occupe un cœur au repos après quelques images ; cause non élucidée.
- Les méthodes de `Array.prototype` qui modifient le tableau directement ignorent une longueur figée.
- Un module avec `await` de premier niveau bloque aussi ses modules frères.
- Les branches `test262-conformite` (PR n° 3) et `jalon-4-web` (PR n° 4, empilée sur la
  précédente) attendent leur relecture.
