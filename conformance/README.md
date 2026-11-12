# Conformance

Ce dossier suit l'écart entre Oxyde et les standards du web. L'objectif est un comportement
identique à Chromium ; l'avancement se mesure avec les suites publiques, pas à l'œil.

## Suites de tests

| Suite | Crate visé | État |
|---|---|---|
| html5lib-tests (tokenizer, tree-construction) | `oxyde_html` | **pas encore branchée** — `--dump-dom` produit déjà le format attendu |
| css-parsing-tests | `oxyde_css` | pas encore branchée |
| Web Platform Tests, dossier `dom/` | `oxyde_dom`, `oxyde_content` | **branchée** : 286 fichiers de test entièrement réussis sur 693 (41,3 %) ; 42936 sous-tests sur 49891, détail dans [wpt.md](wpt.md) |
| Web Platform Tests, mise en page (flex, grille, positionnement, tableaux, tailles, CSS 2, CSSOM View) | `oxyde_css`, `oxyde_layout` | **branchée** pour les tests `testharness.js` : 538 fichiers entièrement réussis sur 2 267 (23,7 %) ; détail dans [wpt-css.md](wpt-css.md). Les tests de référence (comparaison d'images) ne sont pas encore lancés |
| Web Platform Tests, transformations, transitions et animations | `oxyde_css`, `oxyde_layout`, `oxyde_content` | **branchée** pour les tests `testharness.js` : 50 fichiers entièrement réussis sur 363 (13,8 %) ; détail dans [wpt-animations.md](wpt-animations.md). L'API Web Animations (`getAnimations`, `Element.animate`), dont dépendent la plupart de ces tests, reste à faire |
| test262 | `oxyde_js` | **branchée** : 42 992 tests réussis sur 48 795 (88,1 %), détail par dossier dans [test262.md](test262.md) |

Pour les suites qui ne tournent pas encore, aucun taux de réussite ne peut être annoncé : seuls
les tests unitaires de chaque crate (`cargo test --workspace`) les couvrent.

La couverture du DOM (membres des interfaces Web IDL réellement disponibles aux scripts) est
mesurée par `cargo run -p oxyde -- --dom-coverage` et publiée dans [dom.md](dom.md) :
329 membres sur 342 (96,2 %). Elle dit qu'un membre existe, pas qu'il est conforme.

Les Web Platform Tests se lancent avec `cargo run -p oxyde_wpt --release -- --publier dom` après
avoir cloné `web-platform-tests/wpt` dans `vendor/wpt` (non versionné ; les dossiers `resources`,
`common` et `dom` suffisent). `--journal fichier` écrit une ligne par sous-test, pour comparer
deux exécutions : aucun sous-test réussi ne doit être perdu. Ce qui manque le plus : les cadres
(`<iframe>`, un objet global par document), les espaces de noms et les nœuds `Attr`, les tests
pilotés par WebDriver (`testdriver.js`), `Observable`.

test262 se lance avec `cargo run -p oxyde_test262 --release -- --publier` après avoir cloné la
suite dans `vendor/test262` (non versionné). Le taux publié ne doit jamais régresser. Les plus
gros manques qu'il révèle : `Temporal` (4 600 tests à lui seul), les déclarations
`using` et `await using`, les attributs d'importation, `ShadowRealm`.

## Écarts connus

### Analyse HTML (`oxyde_html`)
- Références de caractères nommées : environ 300 noms sur 2 231.
- États « script data escaped » du tokenizer absents.
- `<template>` traité comme un élément ordinaire ; `<frameset>` ignoré.
- Détection du mode quirks limitée aux identifiants publics les plus courants.
- Pas de limite de profondeur à l'analyse (Blink : 512) ; la mise en page, elle, est bornée.
- Encodage supposé UTF-8.

### Style (`oxyde_css`)
- Propriétés prises en charge : `display`, `color`, `background-color`, `font-*`, `line-height`,
  `text-align`, `white-space`, `text-decoration-line`, `margin`, `padding`, `border`,
  `border-radius` (uniforme), `width`/`height` et leurs min/max, `box-sizing`, `list-style-type`.
- Pas de `var()`, `calc()`, pseudo-éléments, `@import`, `@font-face`, `@supports`, `@keyframes`.
- `:hover`, `:focus`, `:active` ne correspondent jamais.
- Pas d'index de règles : coût proportionnel à éléments × règles.

### Mise en page (`oxyde_layout`)
- `flex` et `grid` mis en page comme des blocs.
- Tableaux : cellules de largeur égale.
- Pas de flottants ni de positionnement.
- Fusion des marges entre frères seulement.
- Pas de bordure ni de marge interne sur les boîtes en ligne.
- Hauteurs en pourcentage traitées comme `auto`.

### Peinture et rendu
- Images : `<img src>` seulement (pas de `srcset`, `<picture>`, `object-fit` ni d'images CSS) ; images animées réduites à leur première image.
- Pas de dégradés, d'ombres ni de `z-index`.
- Bordures pointillées et tiretées peintes pleines.

### Réseau (`oxyde_net`)
- Pas de HTTP/3, de cache, de cookies, de réutilisation des connexions ni de décompression.
- Résolution DNS par le système.

### Navigateur
- Un seul processus, pas de bac à sable.
- JavaScript : voir la section dédiée ci-dessous.
- Barre d'adresse : pas de sélection partielle ni de placement du curseur à la souris.
- Onglets épinglés, espaces et onglets fermés ne sont pas conservés entre deux sessions.
- Les polices embarquées pour l'interface (Inter, Space Grotesk, JetBrains Mono) sont aussi visibles des pages web ; Chromium réserve les siennes à l'interface.
- Accessibilité : seul le pourtour est exposé ; une page web apparaît comme un document sans arbre détaillé.
- Le bouton Paramètres est inactif (écran prévu au jalon 7).

### JavaScript (`oxyde_js`) et DOM scriptable
- Absents du langage : `import()` dynamique,
  `Proxy`, gabarits étiquetés, `with`. La syntaxe non prise
  en charge est refusée avec une erreur explicite.
- `eval` et `Function` évaluent toujours dans la portée globale : `eval` ne voit pas les
  variables locales de l'appelant. Le mode strict (`"use strict"`) n'est pas distingué.
- Modules : `import` et `export` sous toutes leurs formes statiques, liaisons vivantes,
  cycles, `import.meta.url`. Pas d'`await` de premier niveau ni d'attributs d'importation ;
  affecter une liaison importée est ignoré au lieu de lever `TypeError` ; deux `export *` qui
  fournissent le même nom ne sont pas signalés comme ambigus. Dans une page, tout le graphe
  est chargé avant l'exécution, sans contrôle CORS, et un spécificateur « nu » (`"lodash"`)
  n'est pas résolu (pas de carte d'importation).
- `BigInt` : arithmétique en précision arbitraire écrite pour Oxyde, bornée à 2^22 bits
  (au-delà, `RangeError`).
- Tableaux typés, `ArrayBuffer` (redimensionnable, transférable, immuable), `DataView`,
  `SharedArrayBuffer` et `Atomics` passent tous leurs tests. Dans une page, la promesse
  d'`Atomics.waitAsync` se règle à la fin de la tâche suivante : rien ne réveille encore
  une page au repos, et il n'y a pas de workers pour partager un tampon.
- `Array` : méthodes génériques (tout objet muni d'une longueur) ; `Symbol.species` et
  `Symbol.isConcatSpreadable` ne sont pas consultés.
- `Symbol` : `Symbol.toPrimitive` et `Symbol.hasInstance` existent mais ne sont pas consultés
  par les conversions ni par `instanceof`.
- Itération : `for…of`, la décomposition et `...` suivent `Symbol.iterator`, sauf pour les
  tableaux, `Map` et `Set`, parcourus directement (redéfinir leur `Symbol.iterator` est sans
  effet) ; un itérateur abandonné en cours de boucle ne reçoit pas d'appel à `return()`.
- Générateurs : `return()` n'exécute pas les blocs `finally` en attente ; `yield*` ne transmet
  pas au générateur délégué les valeurs de `next(v)` ni `throw()` ; les arguments par défaut
  sont évalués au premier `next()` ; des générateurs récursifs imbriqués au-delà d'une
  soixantaine de niveaux lèvent `RangeError`.
- Expressions rationnelles : moteur à retour sur trace écrit pour Oxyde (groupes nommés, y
  compris de même nom dans des alternatives distinctes, références, assertions avant et
  arrière, modificateurs `(?ims-ims:…)`, drapeaux `d g i m s u v y`). `\p{…}` s'appuie sur les
  tables d'ICU4X, en Unicode 18, alors que la suite test262 utilisée vise Unicode 17 : 91 de
  ses tests échouent sur des caractères dont les propriétés ont changé entre les deux
  versions. Avec `v`, les propriétés de chaînes `RGI_Emoji`, `RGI_Emoji_Flag_Sequence` et
  `RGI_Emoji_ZWJ_Sequence` manquent (pas de tables). Le texte est parcouru par points de code
  même sans `u` : un `.` y prend une paire de substituts entière. L'insensibilité à la casse
  en mode Unicode approche le repli simple par la minuscule de la majuscule. Une recherche
  trop coûteuse (motif à coût exponentiel) lève `RangeError`.
- Générateurs async et `for await` : mêmes limites que les générateurs pour `return()` et
  `yield*` ; un itérateur abandonné par `break` ne reçoit pas d'appel à `return()`.
- `Promise` : `then`, `catch`, `finally`, `resolve`, `reject`, `all`, `allSettled`, `any`,
  `race` ; pas de sous-classes (les méthodes statiques ignorent `this`), ni `withResolvers`,
  ni `try`. Un rejet sans gestionnaire est signalé dans la console à la fin de la tâche.
- Chaînes stockées en UTF-8 : longueurs et index comptés en unités UTF-16, mais un substitut
  isolé n'est pas représentable.
- `Date` : le fuseau horaire local n'est pas consulté (l'heure locale est l'heure UTC,
  `getTimezoneOffset()` vaut 0) ; `Date.parse` ne reconnaît que le format ISO et les formats
  de `toString` et `toUTCString`. Pas d'`Intl`.
- Interpréteur seul (pas de compilation à la volée) ; ramasse-miettes marquage-balayage non
  générationnel. Un script est interrompu après 200 millions d'instructions.
- DOM : `getElementById`, `querySelector(All)`, création/insertion/suppression de nœuds,
  `textContent`, `innerHTML`, attributs, `classList`, `style`, événements avec propagation
  (`click`, `load`), `setTimeout`/`setInterval`. Les interfaces (prototypes, constructeurs
  `Node`, `Element`…, constantes, arité et répartition des opérations) sont générées par
  `oxyde_bindings` à partir des fichiers Web IDL de `crates/oxyde_content/idl/` ; le corps des
  opérations et les attributs restent écrits à la main, sans conversion des arguments d'après
  leurs types IDL. Les fichiers IDL sont des transcriptions partielles des standards : les
  éléments HTML particuliers (`HTMLAnchorElement`…) et `Window` n'y figurent pas encore, si
  bien que tous les éléments HTML sont de simples `HTMLElement`.
- `fetch` : `GET` seulement, sans en-têtes ni corps de requête ; la réponse est lue d'un bloc
  (`status`, `ok`, `headers.get/has`, `text()`, `json()`). Politique de même origine : une
  réponse d'une autre origine n'est remise au script que si elle porte
  `Access-Control-Allow-Origin: *` ou l'origine de la page ; la requête ne porte pas d'en-tête
  `Origin`, il n'y a ni requête préalable (`OPTIONS`) ni cookies. Les protocoles autres que
  `http` et `https` sont refusés aux scripts.
- Pas de `XMLHttpRequest`, pas de navigation par `location`, pas de `getBoundingClientRect`,
  pas d'événements clavier ni de formulaires.
- Scripts `type=module` exécutés après les scripts classiques ; `defer` et `async` exécutés
  dans l'ordre du document, comme des scripts classiques.
