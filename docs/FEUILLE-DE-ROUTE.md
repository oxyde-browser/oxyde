# Feuille de route d'Oxyde

État au 12 novembre 2026, branche `test262-conformite`, commit `73d6cdb`.

- `oxyde_js` réussit **40 660 tests de test262 sur 48 795 (83,3 %)** ; il en reste 8 135.
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
| Royaumes (`$262.createRealm`) | — | 141 | à faire |

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

- **DOM** : 220 membres disponibles sur 285 décrits par l'IDL d'Oxyde (77,2 %), et l'IDL ne
  décrit pas encore `Window`, les éléments HTML particuliers, les observateurs ni les plages.
- **Manquent pour les sites réels** : `XMLHttpRequest`, les formulaires, `fetch` autre que
  `GET`, les événements clavier et de saisie, `localStorage`, `history`, `location` complet.
- **Web Platform Tests** : écrire le lanceur (il n'existe pas encore) pour mesurer le DOM comme
  test262 mesure le langage. Sans cette mesure, « ça marche » reste une impression.

### Jalon 5 — Multi-processus

Processus navigateur, réseau, rendu par site et GPU ; `oxyde_ipc` ; bac à sable Windows
(AppContainer, jetons restreints) ; isolation des sites ; écran « Onglet planté ». C'est aussi
le moment de `SharedArrayBuffer` et des workers, qui supposent plusieurs fils.

### Jalon 6 — Mise en page complète et compositeur

Flex, grille, tableaux, flottants, positionnement, transformations, animations et transitions,
calques composités, défilement hors du fil principal. C'est le jalon qui fait passer la plupart
des sites de « lisible » à « fidèle ». Tableau de bord WPT pour le CSS et la mise en page.

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
- La branche `test262-conformite` n'a pas de PR, et repose sur un commit de `jalon-4-javascript`
  qui n'est pas dans `main`.
