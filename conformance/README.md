# Conformance

Ce dossier suit l'écart entre Oxyde et les standards du web. L'objectif est un comportement
identique à Chromium ; l'avancement se mesure avec les suites publiques, pas à l'œil.

## Suites de tests

| Suite | Crate visé | État |
|---|---|---|
| html5lib-tests (tokenizer, tree-construction) | `oxyde_html` | **pas encore branchée** — `--dump-dom` produit déjà le format attendu |
| css-parsing-tests | `oxyde_css` | pas encore branchée |
| Web Platform Tests (sous-ensembles DOM, CSS) | `oxyde_css`, `oxyde_layout` | pas encore branchée — `--screenshot --page-only` servira aux tests de référence |
| test262 | `oxyde_js` | **pas encore branchée** — le moteur exécute un sous-ensemble du langage (voir plus bas) |

Tant que ces suites ne tournent pas, aucun taux de réussite ne peut être annoncé : les tests
actuels sont les tests unitaires de chaque crate (`cargo test --workspace`).

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
- Absents du langage : modules, générateurs async et `for await`,
  `BigInt`, `Proxy`/`Reflect`, tableaux typés, gabarits étiquetés, `eval`. La syntaxe non prise
  en charge est refusée avec une erreur explicite.
- `Symbol` : `Symbol.toPrimitive` et `Symbol.hasInstance` existent mais ne sont pas consultés
  par les conversions ni par `instanceof`.
- Itération : `for…of`, la décomposition et `...` suivent `Symbol.iterator`, sauf pour les
  tableaux, `Map` et `Set`, parcourus directement (redéfinir leur `Symbol.iterator` est sans
  effet) ; un itérateur abandonné en cours de boucle ne reçoit pas d'appel à `return()`.
- Générateurs : `return()` n'exécute pas les blocs `finally` en attente ; `yield*` ne transmet
  pas au générateur délégué les valeurs de `next(v)` ni `throw()` ; les arguments par défaut
  sont évalués au premier `next()` ; des générateurs récursifs imbriqués au-delà d'une
  soixantaine de niveaux lèvent `RangeError`.
- Expressions rationnelles : moteur à retour sur trace écrit pour Oxyde (groupes nommés,
  références, assertions avant et arrière, drapeaux `g i m s u y`). Le texte est parcouru par
  points de code même sans `u` ; `\p{…}` ne connaît que quelques propriétés ; pas de drapeau
  `v` ; `d` est accepté sans effet ; un motif littéral invalide n'est signalé qu'à
  l'exécution ; les méthodes de `String` ne passent pas par `Symbol.match`, `Symbol.replace`…
  Une recherche trop coûteuse (motif à coût exponentiel) lève `RangeError`.
- `Promise` : `then`, `catch`, `finally`, `resolve`, `reject`, `all`, `allSettled`, `any`,
  `race` ; pas de sous-classes (les méthodes statiques ignorent `this`), ni `withResolvers`,
  ni `try`. Un rejet sans gestionnaire est signalé dans la console à la fin de la tâche.
- Chaînes stockées en UTF-8 : longueurs et index comptés en unités UTF-16, mais un substitut
  isolé n'est pas représentable.
- `Date` minimale ; pas d'`Intl`.
- Interpréteur seul (pas de compilation à la volée) ; ramasse-miettes marquage-balayage non
  générationnel. Un script est interrompu après 200 millions d'instructions.
- DOM : `getElementById`, `querySelector(All)`, création/insertion/suppression de nœuds,
  `textContent`, `innerHTML`, attributs, `classList`, `style`, événements avec propagation
  (`click`, `load`), `setTimeout`/`setInterval`. Liaisons écrites à la main : le générateur
  Web IDL (`oxyde_bindings`) n'existe pas encore.
- `fetch` : `GET` seulement, sans en-têtes ni corps de requête ; la réponse est lue d'un bloc
  (`status`, `ok`, `headers.get/has`, `text()`, `json()`). Politique de même origine : une
  réponse d'une autre origine n'est remise au script que si elle porte
  `Access-Control-Allow-Origin: *` ou l'origine de la page ; la requête ne porte pas d'en-tête
  `Origin`, il n'y a ni requête préalable (`OPTIONS`) ni cookies. Les protocoles autres que
  `http` et `https` sont refusés aux scripts.
- Pas de `XMLHttpRequest`, pas de navigation par `location`, pas de `getBoundingClientRect`,
  pas d'événements clavier ni de formulaires.
- Scripts `type=module` ignorés ; `defer` et `async` exécutés dans l'ordre du document, comme
  des scripts classiques.
