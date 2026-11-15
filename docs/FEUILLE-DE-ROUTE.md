# Feuille de route d'Oxyde

État au 12 novembre 2026, branche `main`, après la fusion de la PR n° 8.

- `oxyde_js` réussit **48 139 tests de test262 sur 48 795 (98,7 %)** ; il en reste 656.
- Les jalons 1 à 3 du plan (socle, premier pixel, interface de la maquette) sont terminés.
- Les jalons 4 (JavaScript et DOM), 5 (multi-processus), 6 (mise en page) et 7 (fonctions du
  navigateur) sont **entamés et utilisables, aucun n'est terminé** ; ce qu'il leur reste est
  repris dans la partie 3.
- Les jalons 8 à 10 ne sont pas commencés.

**Oxyde n'est pas encore un navigateur de tous les jours.** La partie 3 est la liste de ce
qu'il faut pour qu'il le devienne, dans l'ordre où le faire ; c'est elle qui tient lieu de
liste de tâches.

Les nombres de tests des étapes, plus bas, datent de la mesure initiale (77,4 %) ; ce sont des
ordres de grandeur, car corriger un domaine fait souvent passer des tests rangés dans un autre.

### Avancement des étapes

| Étape | Au départ | Reste | État |
|---|---:|---:|---|
| A et B — langage, `eval` direct | 1 839 | 352 | presque terminées |
| C — intégrés courants | 1 743 | 217 | presque terminée |
| D — tampons partagés, tableaux typés | 1 024 | — | terminée |
| E — expressions rationnelles | 855 | — | terminée |
| F — propositions récentes | 938 | — | terminée |
| G — Temporal | 4 614 | 0 | terminée |
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

### D — Tampons partagés et tableaux typés (1 024 tests) — faite

- **SharedArrayBuffer et Atomics** (495) : `SharedArrayBuffer` est là (104 tests sur 104), sur
  un stockage dont les octets se partagent entre agents, ainsi qu'`Atomics` pour un agent
  et plusieurs (389 sur 389) : `add`, `and`, `compareExchange`, `exchange`, `load`, `or`,
  `store`, `sub`, `xor`, `isLockFree`, `wait`, `waitAsync`, `notify`, `pause`. Le lanceur
  fournit `$262.agent` : chaque agent est un moteur sur son propre fil, et les tests
  vérifient la mémoire réellement partagée et le réveil par `notify`. `waitAsync` inscrit
  une attente et rend une promesse, réglée entre deux tâches par l'agent qui attend ; le
  lanceur a pour cela une boucle d'événements (`setTimeout`, attentes en cours).
- **TypedArray, ArrayBuffer, DataView** (529) : les constructeurs d'espèce sont consultés
  (`slice`, `filter`, `map`, `subarray`, `from`, `of`), les parcours lisent les éléments au
  fil de l'eau, `toLocaleString` appelle celui de chaque élément (TypedArray : 1 405 tests
  sur 1 453). `Uint8Array` lit et écrit le base64 et l'hexadécimal (`fromBase64`,
  `fromHex`, `toBase64`, `toHex`, `setFromBase64`, `setFromHex` : 68 tests sur 70).
  Les tampons immuables sont là (`immutable`, `transferToImmutable`, `sliceToImmutable`) :
  toute écriture y est refusée, par les tableaux typés, `DataView` et `Atomics` (61 tests
  dédiés sur 66 ; les 5 autres demandent les attributs d'importation). Quand `new.target`
  n'a pas de `prototype` objet, le prototype par défaut est pris dans son royaume, pour
  tous les constructeurs natifs (80 tests, dont une cinquantaine hors de cette étape).
  Tableaux typés, `ArrayBuffer`, `DataView` et `SharedArrayBuffer` passent désormais tous
  leurs tests.

### E — Expressions rationnelles (855 tests) — faite

- **Propriétés Unicode** `\p{…}` (464) : faites, sur les tables d'ICU4X (catégories
  générales, écritures et leurs extensions, propriétés binaires). 357 tests de plus. Les
  tables sont en Unicode 18 et la suite vise Unicode 17 : 91 tests échouent sur des
  caractères dont les propriétés ont changé. Les propriétés de chaînes (`RGI_Emoji`…)
  viennent avec le drapeau `v`. Au passage, la répétition d'un atome d'un seul caractère
  (`a*`, `\p{L}+`) ne coûte plus de place de pile par tour.
- **Modificateurs** `(?ims-ims:…)` : faits (66 tests sur 70). Les drapeaux `i`, `m` et `s`
  sont portés par chaque instruction ; avec `u`, la casse suit le repli simple d'Unicode.
  Les 4 tests restants demandent qu'un `.` sans `u` ne prenne qu'une moitié de paire de
  substituts, ce que des chaînes en UTF-8 ne permettent pas.
- **Drapeau `v`** : fait (184 tests). Classes imbriquées, différence `--`, intersection
  `&&`, chaînes `\q{…}`, propriétés de chaînes (`Basic_Emoji`, `Emoji_Keycap_Sequence`,
  `RGI_Emoji_Modifier_Sequence`, `RGI_Emoji_Tag_Sequence`). `RGI_Emoji` et ses séquences
  de drapeaux et à liant manquent, faute de tables (14 tests).
- **Reste** (170) : faits, `RegExp.escape`, `RegExp.prototype.compile`, les propriétés
  statiques héritées (`RegExp.$1`, `lastMatch`…), le drapeau `d` (indices) et le
  constructeur pour les objets qui se disent expressions rationnelles (93 tests), puis
  les groupes nommés en double (permis dans des alternatives distinctes) et les noms de
  groupe en Unicode, échappements compris (27 tests).

Il reste 115 échecs dans `RegExp` : 91 tiennent à la version d'Unicode de la suite, 14
aux tables d'émojis, et une dizaine aux chaînes en UTF-8 (demi-substituts isolés).

### F — Propositions récentes (938 tests)

- **`using` et `DisposableStack`** (338) : les intégrés sont faits (`Symbol.dispose`,
  `Symbol.asyncDispose`, `DisposableStack`, `AsyncDisposableStack`, libération des
  itérateurs : 236 tests), écrits en JavaScript embarqué, ainsi que les déclarations
  `using` et `await using` (131 tests) : dans les blocs, les corps de fonction et de
  module, et les têtes de boucle. Une portée qui en déclare tient une pile de ressources,
  vidée à sa sortie, par un saut comme par une exception. Au passage, les noms liés par
  la tête d'un `for…in` ou `for…of` sont en zone morte pendant l'évaluation de l'objet
  parcouru.
- **`import defer`, `import source`, attributs d'importation** (320) : les formes
  d'appel `import.source(x)` et `import.defer(x)` sont lues (149 tests). `import.source`
  charge le module puis rejette sa promesse, un module JavaScript n'ayant pas de source à
  rendre.
  Les attributs d'importation sont faits (24 tests) : `with { type: "json" }` et
  `with { type: "text" }` donnent un module dont le seul export, `default`, est la
  valeur du fichier ; `import()` les reçoit par son second argument. Le type `bytes`
  manque (5 tests) : le chargeur de modules ne fournit que du texte.
  `import defer * as ns` et `import.defer(x)` sont faits (95 tests) : le module est
  chargé et lié, puis évalué au premier usage d'une propriété de son espace de noms
  différé ; ses parties à `await` de premier niveau sont évaluées d'avance. Restent
  une dizaine de tests sur l'ordre d'évaluation entre modules asynchrones.
- **Liaisons de module** : les `let`, `const` et classes de premier niveau d'un module
  sont en zone morte dès son chargement, ses `var` valent `undefined` (26 tests, dont
  21 que le lanceur faussait en chargeant deux fois un module qui s'importe lui-même).
  Erreurs précoces des modules (25 tests) : une fonction de premier niveau ne se
  redéclare pas, un nom exporté doit être déclaré, `export default function () {}` est
  une déclaration. Restent les fonctions, créées seulement à l'évaluation, et l'objet
  d'espace de noms, qui n'est pas encore un objet exotique.
- **Aides d'`Iterator` restantes** (216) : faites, `zip`, `zipKeyed`, `chunks`, `windows`,
  `includes` et `join` (210 tests). Restent 3 tests qui écrivent `length = 2^32 - 1` sur
  un tableau : les tableaux d'Oxyde sont denses.
- **ShadowRealm** (64) : fait, les 64 tests passent. `evaluate` exécute le code dans un
  royaume à part ; seules les primitives et les fonctions, enveloppées, passent la
  frontière. Un royaume ainsi créé vit autant que la machine.

### G — Temporal (4 614 tests)

Sept types (`Instant`, `PlainDate`, `PlainTime`, `PlainDateTime`, `PlainYearMonth`,
`PlainMonthDay`, `ZonedDateTime`), `Duration`, `Temporal.Now` et
`Date.prototype.toTemporalInstant`. **Fait** : les 4 614 tests passent. L'objet est écrit en
JavaScript embarqué (`crates/oxyde_js/src/builtins/temporal/`) et n'est construit qu'à son
premier usage dans un royaume.

Limites, hors du périmètre de ces tests (elles relèvent d'`intl402`) :

- pas de **base de fuseaux horaires IANA** : seuls `UTC` et les décalages fixes (`+01:00`)
  existent, et `Temporal.Now.timeZoneId()` vaut `UTC` ;
- seul le **calendrier** `iso8601` existe ;
- `toLocaleString` rend la forme ISO, faute d'`Intl`.

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

- **Web Platform Tests** : le lanceur (`oxyde_wpt`) mesure `dom/` — 309 fichiers de test entièrement réussis sur 693 (44,6 %).
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
- **Liés à la mise en page (jalon 6)** : dessin et saisie des contrôles de formulaire et focus
  (faits, étape H),
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

Branche `jalon-5-multi-processus`, empilée sur `jalon-6-mise-en-page`.

Fait :

- **Messages entre processus** (`oxyde_ipc`) : codec binaire écrit sur la bibliothèque
  standard (`Encode`, `Decode`, macros `ipc_struct!` et `ipc_enum!`), trames bornées, canal
  avec fil de lecture, lancement d'un processus enfant relié par son entrée et sa sortie
  standard — il ne lui faut donc aucun droit sur le système pour parler à son parent. Un
  message reçu est tenu pour hostile : le décodage vérifie chaque longueur avant d'allouer
  et ne panique pas.

- **Liste d'affichage sur le fil** (`oxyde_paint::wire`) : polices et images ne voyagent
  qu'une fois ; le destinataire rejette toute liste incohérente.
- **Processus de rendu par onglet** (`oxyde --type=renderer`) : chaque page du web ou
  fichier local est chargée, scriptée, mise en page et peinte dans son propre processus,
  qui n'ouvre ni fichier ni connexion — il demande ses ressources au navigateur, lequel ne
  lit un fichier que pour un document local. Le navigateur garde la fenêtre, le réseau, les
  cookies et la composition : la molette fait défiler sans attendre le processus de rendu.
  Si ce processus disparaît, seul son onglet affiche « Cet onglet a planté » ; si le
  navigateur disparaît, ses processus de rendu s'arrêtent. `--single-process` garde tout
  dans le navigateur ; `about:crash` fait planter le processus de rendu, pour vérifier.
  Une capture `--screenshot --page-only --multi-process` passe par un processus de rendu :
  les tests d'intégration la comparent, pixel pour pixel, au rendu dans le navigateur.

- **Écran « Cet onglet a planté »** : « Recharger la page » relance un processus de rendu,
  « Fermer l'onglet » ferme l'onglet.
- **Bac à sable Windows** (`oxyde_sandbox`) : le processus de rendu est lancé avec un jeton
  restreint (privilèges retirés ; accès limité à ce qui est ouvert à tous, aux utilisateurs
  ou à la session, donc ni les fichiers du profil ni le registre de l'utilisateur), un
  niveau d'intégrité bas (aucune écriture), un objet de travail (il meurt avec le
  navigateur, ne lance aucun processus, n'a ni presse-papiers ni réglages du système) et
  les seules poignées de ses tubes. Il se durcit ensuite lui-même : pas de code produit à
  l'exécution, pas de points d'extension, pas de bibliothèque distante. Un test lance une
  sonde dans le bac à sable et constate ces refus, témoin hors bac à sable à l'appui.
  `--no-sandbox` y renonce, pour le diagnostic. C'est le seul crate du projet qui contient
  du code `unsafe` (appels Win32), chaque bloc étant justifié.

- **Processus réseau** (`oxyde --type=network`) : HTTP, TLS et cookies quittent le
  navigateur pour un processus à part, auquel il adresse ses requêtes par un canal ; les
  requêtes sont servies en parallèle. Si ce processus disparaît, les requêtes en cours
  échouent et un autre est lancé à la suivante. Son bac à sable est moins serré que celui
  du rendu (`Policy::Networked`) : privilèges retirés, intégrité basse, objet de travail,
  mais pas de restriction par identifiants, que la résolution de noms refuse. Les fichiers
  locaux restent lus par le navigateur.

- **Isolation des sites** : un processus de rendu est voué à un site. Une navigation vers
  un autre site — y compris par redirection — passe par un processus neuf. Le navigateur
  tient ce verrou d'après ce qu'il a lui-même demandé et reçu du réseau, jamais d'après ce
  que le processus affirme : il ne lui sert un document que pour la navigation qu'il lui a
  confiée, et des ressources, des requêtes de script ou des cookies qu'au nom d'un
  document de son site ; l'adresse affichée et les changements d'adresse sans chargement
  restent sur ce site ; une page du web n'ouvre pas un fichier local.

Limites à ce stade : les cadres vivent dans le processus de leur page, quel que soit leur
site ; deux onglets du même site ont chacun leur processus ; le site est approché sans la
liste des suffixes publics ; une redirection vers un autre site est demandée deux fois au
serveur ; l'accessibilité ne voit pas le contenu d'une page distante. Le bac
à sable n'est pas un `AppContainer` : le processus de rendu peut encore lire ce qui est
lisible par tous sur la machine et ouvrir des connexions réseau ; le processus réseau peut
lire ce que l'utilisateur peut lire ; `win32k` n'est pas verrouillé. Les cookies vivent en
mémoire dans le processus réseau et sont perdus s'il s'arrête. Pas de processus GPU : la
rastérisation reste dans le navigateur.

Reste : les workers (`SharedArrayBuffer` et `Atomics` sont faits, étape D de test262) ; le
processus GPU ;
les cadres d'un autre site dans leur propre processus.

### Jalon 6 — Mise en page complète et compositeur

Flex, grille, tableaux, flottants, positionnement, transformations, animations et transitions,
calques composités, défilement hors du fil principal. C'est le jalon qui fait passer la plupart
des sites de « lisible » à « fidèle ». Tableau de bord WPT pour le CSS et la mise en page.

Branche `jalon-6-mise-en-page`, empilée sur `jalon-4-web`.

Fait :

- **Mesure** : le lanceur WPT exécute les 2 267 tests `testharness.js` de `css-flexbox`,
  `css-grid`, `css-position`, `css-sizing`, `css-tables`, `css-box`, `css-display`,
  `css-align`, `CSS2` et `cssom-view`. Point de départ : 213 fichiers entièrement réussis
  (9,4 %), aujourd'hui 569 (25,1 %); une lecture force la
  mise en page si le document a changé.
- **Positionnement** : `position: relative`, `absolute`, `fixed`, décalages `inset`, blocs
  conteneurs, hauteurs en pourcentage, `calc()`, police de test Ahem (`@font-face`).
- **Flex et grille** (`oxyde_layout::flex`, `oxyde_layout::grid`) : algorithme flexible
  complet (lignes, retours à la ligne, `flex-grow`/`shrink`/`basis`, alignements, `order`,
  `gap`) ; grille avec pistes explicites et implicites, `fr`, `minmax()`, `repeat()`,
  placement automatique, zones nommées, alignement des éléments.
- **Peinture** : ordre d'empilement (`z-index`), découpe (`overflow`), `opacity`,
  `visibility`.

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

- **Transformations** : `transform` (fonctions du plan ; les fonctions 3D sont ramenées à leur
  effet dans le plan) et `transform-origin` ; peinture par groupes transformés, test
  d'atteinte par transformation inverse, `getBoundingClientRect` transformé ; une boîte
  transformée est un bloc conteneur et s'empile comme une boîte positionnée. Mesure à part
  ([wpt-animations.md](../conformance/wpt-animations.md)) : 27 fichiers sur 363 au départ.
- **Animations et transitions** (`oxyde_css::animation`) : `@keyframes`, propriétés
  `animation-*` et `transition-*` et leurs raccourcis, fonctions de temporisation
  (`cubic-bezier()`, `steps()`), interpolation des longueurs, couleurs, nombres et
  transformations ; les valeurs animées s'héritent, `!important` l'emporte sur une
  animation. La page et son moteur de script partagent un même animateur et une même
  horloge ; les événements `animationstart`/`iteration`/`end`/`cancel` et
  `transitionrun`/`start`/`end`/`cancel` sont distribués, et une animation en cours demande
  une image toutes les 16 ms. Les minuteurs suivent désormais l'horloge de la page, ce qui
  donne aux tests un temps virtuel cohérent. Mesure : 57 fichiers sur 363 (15,7 %). Restent :
  l'API Web Animations, dont dépendent la plupart des tests, et l'interpolation des
  transformations par décomposition de matrice.

- **Défilement composité** (`oxyde_compositor`) : la peinture désigne chaque zone défilante
  (`overflow` autre que `visible`) par un identifiant et n'en fige pas la position ; le
  décalage est appliqué à la composition, si bien que faire défiler la fenêtre ou un élément
  ne redemande ni style, ni mise en page, ni peinture. L'arbre de défilement borne les
  décalages, route la molette vers la zone sous le pointeur puis vers celles qui la
  contiennent, et dessine les barres de défilement ; `position: fixed` ne suit plus la
  fenêtre. Côté script : `scrollTop`/`scrollLeft`, `scrollTo`/`scrollBy`/`scroll` (éléments
  et fenêtre), `scrollIntoView`, `scrollX`/`scrollY`, `document.scrollingElement`, événement
  `scroll`. Restent : la composition hors du fil principal, les calques mis en cache, les
  animations jouées par le compositeur, le défilement fluide, `position: sticky`.

Reste du jalon : l'API Web Animations ; les tests de référence WPT (comparaison d'images),
qui couvrent la peinture que les tests `testharness.js` ne voient pas ; `<col>`, la ligne de
base dans les tableaux, les modes d'écriture verticaux.

### Jalon 7 — Fonctions du navigateur

Les écrans restants de la maquette : paramètres, historique, favoris, téléchargements, infos du
site, autorisations, recherche dans la page, navigation privée, palette de commandes, mots de
passe, premier lancement, onglets horizontaux, menus contextuels, impression. S'y ajoute la
persistance.

Branche `jalon-7-fonctions`, empilée sur `jalon-5-multi-processus`.

Fait :

- **Profil** (`oxyde_storage`) : un dossier (`%LOCALAPPDATA%\Oxyde\Profil`, ou `--profile`),
  un fichier texte par sorte de donnée, remplacé d'un coup ; l'historique est un journal
  allongé sur place. La lecture tolère un fichier tronqué ou abîmé et borne ce qu'elle accepte.
- **Session** : onglets, onglets épinglés, espaces, vue scindée et onglets fermés reviennent au
  lancement suivant ; seul l'onglet affiché est rechargé, les autres le sont quand on les ouvre.
- **Historique** : chaque nouvelle navigation réussie y est notée, avec son titre (les retours
  en arrière et les rechargements n'en sont pas). 20 000 visites au plus.
- **Favoris** : Ctrl D ajoute ou retire la page.
- **Écrans Historique et Favoris** (bibliothèque, Ctrl H et Ctrl Maj O) : visites groupées par
  jour à l'heure locale, recherche au fil de la frappe, retrait d'une ligne, effacement de
  l'historique après confirmation ; une page s'ouvre dans un nouvel onglet. L'heure locale vient
  d'`oxyde_platform` (API Win32 ; UTC ailleurs).

- **Écran Paramètres** (Ctrl , ou le bouton de la barre latérale) : couleur d'accent, reprise
  de la session au démarrage, moteur de recherche, conservation de l'historique, effacement de
  l'historique. Chaque réglage agit et est conservé dans le profil ; ceux de la maquette qui
  n'ont pas encore d'effet (disposition des onglets, thème clair, performances, traqueurs) n'y
  figurent pas.

- **Palette de commandes** (Ctrl K) : posée sur les pages, elle cherche à la fois dans les
  onglets ouverts, l'historique, les favoris et les commandes du navigateur, avec des filtres
  (Tab) ; sa dernière ligne ouvre l'adresse saisie ou lance la recherche. Ctrl Maj S scinde la
  vue. La liste ne défile pas encore.

- **Navigation privée** (Ctrl Maj N, ou `--private`) : une fenêtre à part, qui est un autre
  processus avec son propre processus réseau. Ni session ni historique n'y sont lus ou écrits ;
  ses cookies, en mémoire, ne se mêlent pas à ceux de la fenêtre principale et disparaissent
  avec elle ; seuls les favoris ajoutés rejoignent le profil, sans écraser ceux des autres
  fenêtres. La teinte violette de la maquette ne couvre que la barre de titre et la page
  « Nouvel onglet ».

- **Recherche dans la page** (Ctrl F) : une barre flottante en haut du volet ; les occurrences
  sont surlignées, la courante d'une autre couleur, et la page défile jusqu'à elle ; Entrée et
  Maj Entrée les parcourent en boucle. La recherche ignore la casse et traverse les éléments en
  ligne. Elle se fait là où vit la page : dans le processus de rendu en mode multi-processus,
  par deux messages de plus. Limites : une occurrence ne traverse pas deux blocs ; les zones
  défilantes internes ne défilent pas jusqu'à l'occurrence ; les accents comptent.

- **Premier lancement** : sur un profil neuf, un accueil en deux étapes — reprendre les favoris
  de Chrome ou d'Edge s'ils sont installés (leur fichier `Bookmarks`, lu comme une entrée non
  fiable, dossiers aplatis), puis choisir la couleur d'accent. La maquette propose aussi
  l'historique, les mots de passe, les onglets, la disposition et le navigateur par défaut :
  rien de cela n'existe encore, donc rien de cela n'est proposé.

- **Onglets horizontaux** : réglage « Disposition des onglets » des paramètres. Les onglets
  passent dans la barre de titre ; une barre d'outils porte la navigation, l'adresse, la vue
  scindée, les espaces et les paramètres. La bande d'onglets ne défile pas et les onglets
  fermés récemment n'y sont pas listés ; la page garde ses marges et ses coins arrondis, là
  où la maquette la colle aux bords.

Reste : les dossiers de favoris ; l'import depuis Firefox ; les écrans (téléchargements, infos du site,
autorisations, mots de passe,
menus contextuels, impression) ; la persistance des
cookies ; un verrou sur le profil.

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

### Ordre suivi jusqu'ici

Étapes A à C de test262, fin du jalon 4 côté web et Web Platform Tests, jalon 6 avant le
jalon 5, puis étapes D à G de test262 et début du jalon 7. La suite est l'objet de la partie 3.

## Dettes connues à ne pas oublier

- Un fil du pilote graphique AMD occupe un cœur au repos après quelques images ; cause non élucidée.
- Les méthodes de `Array.prototype` qui modifient le tableau directement ignorent une longueur figée.
- Un module avec `await` de premier niveau bloque aussi ses modules frères.
- Deux fenêtres ouvertes sur le même profil s'écrasent l'une l'autre : il manque un verrou.
- `Temporal` n'a que le calendrier `iso8601` et les fuseaux `UTC` et à décalage fixe.
- Les PR empilées se fusionnent dans l'ordre, de la base vers le sommet ; sinon, viser `main`.

## Partie 3 — Vers une version pleinement utilisable

But : qu'une personne puisse faire d'Oxyde son navigateur de tous les jours sous Windows.
« Pleinement utilisable » a une définition vérifiable, donnée étape par étape ; tant qu'une
case n'est pas cochée **et mesurée**, elle n'est pas faite.

Trois paliers, à franchir dans l'ordre :

| Palier | Ce qu'il permet | Étapes |
|---|---|---|
| **Alpha** | lire et utiliser les sites courants, sans y passer la journée | H, I, J |
| **Bêta** | s'en servir tous les jours, le confier à des testeurs | K, L, M, N |
| **1.0** | le recommander à quelqu'un, chiffres à l'appui | O, P, Q, R |

L'ordre met d'abord ce sans quoi rien d'autre ne se voit : tant que les sites courants ne se
chargent pas et qu'on ne peut pas y taper, ni les écrans restants ni la vitesse ne comptent.

### H — Les sites courants se chargent (alpha)

Le manque le plus lourd. Mesure : une **liste de 50 sites de référence**, rangée dans
`conformance/sites.md`, essayée à la main et par capture à chaque palier ; pour chacun, la
page s'affiche-t-elle, peut-on s'y connecter, peut-on y faire l'action principale ?

- [x] Dresser la liste (recherche, encyclopédie, presse, messagerie web, vidéo, commerce,
      banque, administration, documentation technique, réseaux sociaux) et noter l'état de départ :
      [conformance/sites.md](../conformance/sites.md). Départ : 14 pages lisibles sur 50 ;
      au 14 novembre 2026 : 20, et plus aucun site ne dépasse 60 secondes. Seul l'affichage de la page d'accueil est mesuré ; la
      connexion et l'action principale restent à essayer.
- [x] Mise en page de conteneurs flex et grille imbriqués en temps linéaire (elle doublait à
      chaque niveau : GitHub ne s'affichait jamais) ; les erreurs de script disent où elles
      sont nées (fonctions en cours, ligne et colonne de leur définition, adresse du script).
- [x] Moteur JavaScript : lecture des chaînes par indice en temps linéaire (elle était
      quadratique : X, Vimeo et Mastodon ne rendaient pas la main), retour de fonction
      indépendant du nombre de variables capturées, chemin rapide pour les instructions
      simples. Mesures face à V8 : [bench/moteur-js.md](../bench/moteur-js.md).
- [x] Pourquoi une page est lente : journal des étapes d'un chargement et des scripts longs
      (variable `OXYDE_JOURNAL`). Il a montré que le temps partait surtout hors du moteur
      JavaScript : calcul des styles (chaque règle essayée sur chaque élément ; les règles sont
      maintenant indexées par identifiant, classe et nom), requêtes sans réponse attendues
      jusqu'au délai en capture, une connexion neuve par ressource (les connexions HTTP/2 sont
      maintenant partagées).
- [ ] Styles : le calcul reste complet à chaque lecture qui suit une écriture (pas de
      recalcul limité aux éléments touchés) ; un script qui alterne écritures et lectures de
      `getComputedStyle` le paie à chaque tour.
- [x] Réseau : les connexions HTTP/1.1 sont réutilisées (une connexion au repos ressert
      pour la requête `GET` ou `HEAD` suivante vers le même hôte ; six au repos par hôte,
      trente secondes au plus) ; cache HTTP en mémoire (réponses `200` aux `GET` que leur
      serveur permet de garder : `max-age`, `Expires`, fraîcheur estimée d'après
      `Last-Modified`, `no-store`, `no-cache`, `Vary`, `Age` ; revalidation par
      `If-None-Match` et `If-Modified-Since`, un `304` ressert le corps gardé), cloisonné
      par site du document demandeur. « Actualiser » oublie ce qui est gardé pour le
      site de la page.
      Limites : rien sur disque (le cache se vide avec le processus réseau) ; 64 Mio
      en tout, 8 Mio par réponse ; ni redirections ni réponses partielles gardées ; pas
      de `stale-while-revalidate` ; « Actualiser » recharge tout le site au lieu de
      revalider ; un `POST` n'invalide pas la réponse gardée pour la même adresse ; le
      site d'un hôte est estimé sans la liste des suffixes publics ; le gain de temps
      n'a pas été mesuré sur de vrais sites.
- [ ] Moteur JavaScript, ce qui reste avant le JIT du jalon 8 : concaténation de chaînes
      (quadratique), coût des appels (allocation des arguments), formes d'objets et cache
      d'accès aux propriétés, compilateur moins naïf. Objectif : l'ordre de grandeur de
      l'interpréteur de V8.
- [x] **Saisie dans les pages** : focus (`focus()`, `blur()`, `document.activeElement`,
      `:focus`, `:checked`, `:disabled`, tabulation), champs de texte éditables avec curseur
      (flèches, Début, Fin, Retour arrière, Suppr, `maxlength`, `readonly`, mot de passe
      masqué, texte indicatif), cases, boutons radio, listes déroulantes (flèches), `<textarea>`,
      bouton d'envoi, Entrée qui envoie le formulaire, événements `input`, `change`, `focus`,
      `blur`. Page d'essai : `tests/pages/formulaires.html`.
      Vérifié par tests et par captures ; pas encore essayé au clavier dans la fenêtre sur des
      sites réels (ce sera fait avec la liste des 50 sites).
- [x] Champs d'une ligne : sélection au clavier (Maj avec les flèches, Début, Fin ; Ctrl A),
      surlignée ; Ctrl C copie, Ctrl X coupe, Ctrl V colle à la place de la sélection (sans
      sauts de ligne, dans la limite de `maxlength`) ; un mot de passe ne se copie pas ; un
      clic place le curseur au caractère le plus proche. Le presse-papiers n'est lu que sur
      Ctrl V. Une zone de texte reçoit le texte collé à sa fin.
      Limites : pas de sélection à la souris dans un champ, pas de déplacement par mot
      (Ctrl flèche), pas d'événements `copy`, `cut`, `paste`, `select` ni de
      `selectionStart` / `setSelectionRange` reliés à cette sélection.
      Vérifié par tests ; pas encore essayé au clavier dans la fenêtre.
- [x] Saisie, suite : une zone de texte (`<textarea>`) s'édite au curseur (flèches, Haut et
      Bas, Début et Fin de ligne, Entrée, Retour arrière et Suppr au milieu du texte,
      sélection au clavier, copier, couper, coller, clic qui place le curseur, curseur
      dessiné) ; Tab suit l'ordre de `tabindex` (positifs d'abord, par valeur, puis
      l'ordre du document ; un `tabindex` négatif en sort) ; `autofocus` donne le focus à
      la fin du chargement.
      Limites : dans une zone de texte, Haut et Bas suivent les lignes du texte, pas les
      lignes affichées quand il est replié ; la sélection n'y est pas surlignée ; elle
      ne défile pas pour suivre le curseur. Non essayé à la main dans la fenêtre.
- [x] Liste déroulante : un `<select>` à choix unique se déroule au clic, à Entrée ou à
      Espace ; ses options se dessinent par-dessus la page, celle qui est choisie en
      évidence ; un clic sur une option la choisit (`input`, `change`) et referme la
      liste ; une option désactivée ne se choisit pas ; Échap, un clic ailleurs ou la
      perte du focus la referment ; les flèches choisissent, liste déroulée ou non.
      Limites : la liste s'ouvre toujours vers le bas et ne défile pas (une longue
      liste sort de la fenêtre) ; elle ignore les transformations et le défilement des
      zones internes ; pas de recherche par frappe d'une lettre ; `<optgroup>` n'y a
      pas de titre ; une liste `multiple` ou de taille supérieure à 1 ne se déroule
      pas. Non essayé à la main dans la fenêtre.
- [ ] Saisie, ce qui reste : saisie par méthode d'entrée (IME).
- [x] Sélection de texte à la souris et copie : glisser sélectionne, un double clic prend
      le mot, Ctrl A toute la page, Ctrl C copie dans le presse-papiers (un processus de
      rendu n'y écrit que si l'utilisateur vient de copier). Le texte copié est celui
      qu'on lit : blancs fusionnés hors `white-space: pre`, une ligne par bloc.
      Limites : la sélection suit l'ordre du document, ignore `user-select`, les
      transformations et le défilement des zones internes ; elle n'est pas visible des
      scripts (`getSelection()` reste vide, pas de `selectionchange`) ; pas de triple
      clic, de Maj clic ni de défilement automatique en glissant hors du volet.
      Vérifié par tests ; pas encore essayé à la souris dans la fenêtre.
- [x] Menu contextuel de la page (clic droit) : ouvrir le lien dans un nouvel onglet, copier
      son adresse, copier, coller, tout sélectionner, précédent, suivant, actualiser. Le
      lien qu'annonce un processus de rendu passe les mêmes contrôles qu'une navigation
      demandée par la page ; sa réponse n'est acceptée que si un menu a été demandé.
      Limites : pas d'actions sur les images (enregistrer, copier), pas de « Rechercher
      la sélection », pas d'« Inspecter » ; l'événement `contextmenu` n'est pas distribué
      (une page ne peut ni le voir ni l'annuler) ; pas de navigation au clavier dans le
      menu. Vérifié par tests ; ni essayé à la souris ni comparé à la maquette à l'écran.
- [x] Événements de souris dans les pages (ils manquaient tous, sauf un `click` sans
      coordonnées) : `mousedown`, `mouseup`, `click` et `dblclick` en `MouseEvent` de
      confiance (position, bouton, rang du clic, touches tenues), `mousemove`,
      `mouseover` / `mouseout`, `mouseenter` / `mouseleave`, leurs équivalents
      `pointer…`, `contextmenu` (l'annuler retient le menu) et `wheel` (avec l'ancien
      `mousewheel`). La molette n'attend la page que si un écouteur non passif existe ;
      sinon le navigateur défile tout de suite, comme avant.
      Limites : `:hover` ne suit pas le pointeur ; la sortie du pointeur hors de la page
      n'est pas signalée ; annuler `mousedown` n'empêche ni le focus ni la sélection ;
      annuler `pointerdown` ne retient pas les événements `mouse…` ; `offsetX` /
      `offsetY` valent `pageX` / `pageY` au lieu d'être relatifs à la cible ; pas de capture du pointeur, pas de
      glisser-déposer, pas d'événements tactiles ; un écouteur de molette retiré reste
      compté. Vérifié par tests ; pas encore essayé à la souris dans la fenêtre.
- [x] Gestes de `testdriver.js` dans le lanceur Web Platform Tests : souris, molette et
      clavier sont joués sur la page comme le ferait la fenêtre (le rôle de WebDriver
      ailleurs) ; un geste tactile ou au stylet fait échouer la demande au lieu de
      l'attendre. Au passage : `body.onload = …` (et les autres gestionnaires de la
      fenêtre posés sur `<body>`) ; une boîte `position: fixed` reste cliquable hors
      d'un ancêtre qui découpe (`body { overflow: hidden }` sans hauteur : voiles et
      fenêtres modales). `dom/` : 309 → 339 fichiers (44,6 → 48,9 %).
- [x] Scripts insérés par un script : un script en ligne s'exécute dès son insertion
      (`appendChild`, `append`, `replaceChild`, `textContent`…), au milieu du script qui
      l'insère, et non plus à la fin de la tâche ; un script vide s'exécute quand il reçoit
      son texte, une seule fois. API des tableaux (`rows`, `cells`, `tBodies`, `caption`,
      `tHead`, `tFoot`, `insertRow`, `deleteRow`, `insertCell`, `deleteCell`, `rowIndex`,
      `cellIndex`…), `window.event`, `MutationRecord` pour `instanceof`.
      Limites : les collections des tableaux sont figées à la lecture ; un script posé
      par `innerHTML` s'exécute encore en fin de tâche (il ne devrait pas s'exécuter).
- [x] Exceptions non rattrapées : événement `error` (un `ErrorEvent` annulable) sur la
      fenêtre, `window.onerror(message, fichier, ligne, colonne, erreur)` qui annule en
      rendant `true`, `reportError()` ; promesses rejetées sans gestionnaire : événement
      `unhandledrejection` (`PromiseRejectionEvent`). Un gestionnaire en attribut annule
      l'événement en rendant `false`. Rien n'arrivait à la page jusque-là : seule la
      console recevait l'erreur.
      Limites : `lineno` et `colno` valent 0 (le code compilé ne garde pas les
      positions) ; pas de `rejectionhandled` ; l'erreur d'un cadre est signalée à la
      fenêtre du dernier royaume initialisé.
- [x] `AbortSignal` selon la norme : algorithmes d'abandon, signaux dépendants de
      `AbortSignal.any()` prévenus dans l'ordre après leur source, événement `abort` du
      navigateur (`isTrusted`), option `signal` de `addEventListener`.
      Limite : un signal dépendant reste tenu par ses sources jusqu'à son abandon.
- [x] `Observable` (proposition WICG) : `Observable`, `Subscriber`, producteur partagé
      entre abonnés, `map`, `filter`, `take`, `drop`, `takeUntil`, `flatMap`,
      `switchMap`, `inspect`, `catch`, `finally`, `toArray`, `forEach`, `every`, `some`,
      `find`, `first`, `last`, `reduce`, `Observable.from()` (observable, itérable,
      itérable asynchrone, promesse), `EventTarget.when()`.
      Limite : pas de contrôle « document pleinement actif » (cadre détaché).
- [x] Cadres : l'événement `load` d'un `<iframe>` ne remonte plus jusqu'à la fenêtre
      (`<body onload>` et les écouteurs `load` de la page s'exécutaient une fois par
      cadre chargé) ; un cadre inséré pendant le chargement d'un autre prend son contenu
      dans la même tâche ; un `<img>` inséré ou cloné par un script prend tout de suite
      la taille d'une image déjà reçue.
- [x] Événements tactiles : `Touch`, `TouchList`, `TouchEvent`, `touchstart`,
      `touchmove`, `touchend`, `touchcancel` sur l'élément où le contact a commencé ;
      `touchstart` et `touchmove` ne s'annulent que si un écouteur non passif les
      attend ; un appui bref que rien n'annule se termine par les événements de souris
      et le clic. Gestes tactiles de `testdriver.js` joués par le lanceur WPT.
      Limites : pas d'événements `pointer…` de type `touch` ; la fenêtre du navigateur
      ne transmet pas encore l'écran tactile (ni défilement au doigt).
- [x] Observateurs de mutation prévenus par une micro-tâche, mise en file dès le
      premier enregistrement qui intéresse un observateur, à sa place parmi les
      réactions de promesses (ils attendaient la fin de la tâche). Les rappels des
      éléments personnalisés, qui en dépendent, partent donc plus tôt.
      `moveBefore()` franchit la frontière d'un arbre fantôme du même document et
      déplace un commentaire dans un document.
- [x] `outerHTML` en écriture (il manquait : l'affectation était sans effet) ;
      `innerHTML` et `outerHTML` donnent un seul enregistrement de mutation ; un
      sélecteur invalide lève une `DOMException` « SyntaxError » ; arguments
      obligatoires de `appendData`, `substringData`… ; `textContent = undefined` vide le
      nœud ; `initUIEvent`, `initMouseEvent`, `initKeyboardEvent`,
      `initCompositionEvent` ; `StaticRange` ; `Symbol.unscopables` pour les méthodes
      de `ChildNode` et `ParentNode` ; `isEqualNode` compare préfixes, espaces de noms
      des attributs et instructions de traitement ; `window.event` est une propriété
      propre, intacte pour un écouteur d'arbre fantôme ; anciens noms
      `webkitAnimationEnd`, `webkitTransitionEnd`… ; un script d'un arbre fantôme
      s'exécute quand son hôte rejoint le document ; `performance.now()` et
      `Event.timeStamp` ont une résolution de 0,1 ms.
      Limites : `outerHTML` analyse son texte comme le contenu d'un `<body>` ; un
      identifiant `#-123` est encore accepté dans un sélecteur.
- [x] Collections du DOM : les rangs et les noms d'une `HTMLCollection`, d'une
      `NodeList` vivante et d'une `NamedNodeMap` ne s'écrivent pas, ne se redéfinissent
      pas et ne se suppriment pas (erreur en mode strict) ; ils paraissent dans
      `Object.keys` (les rangs) et `Object.getOwnPropertyNames` (rangs et noms) ; une
      propriété posée par un script passe avant un nom apparu ensuite ; `dataset`
      énumère ses clés et `delete` y retire l'attribut. `NodeList` et `HTMLCollection`
      n'héritent plus de `Array.prototype` : `NodeList` garde `forEach`, `keys`,
      `values`, `entries` et l'itération ; `HTMLCollection` l'itération seule.
      Limites : une `NodeList` figée (`querySelectorAll`) reste un tableau
      (`Array.isArray` vrai) ; `length` lu sur un objet qui hérite d'une collection ne
      lève pas d'erreur.
- [x] `element.style = "…"` écrit l'attribut `style` (l'affectation d'une chaîne était
      sans effet). Événement `scrollend` : une zone qui a défilé et ne bouge plus à
      l'image suivante le reçoit (`onscrollend` sur la fenêtre et les éléments) ; les
      événements de défilement précèdent les minuteurs et `requestAnimationFrame` de
      la même image ; une zone retirée du document ne reçoit rien.
      Limites : pas de défilement doux (`behavior: "smooth"` saute à la cible) ni
      d'accrochage (`scroll-snap`) ; `scrollIntoView` sur la fenêtre et le défilement
      au clavier d'un champ ne déclenchent pas ces événements.
- [x] API Web Animations, en version réduite : `Element.animate()` (images clés en
      liste ou par propriété ; durée, délai, itérations, direction, remplissage,
      temporisation), `Animation` (`play`, `pause`, `finish`, `cancel`, `currentTime`,
      `playState`, `finished`, `ready`, `onfinish`, `oncancel`), `KeyframeEffect`,
      `AnimationPlaybackEvent`, `document.timeline`, `getAnimations()`. Ces animations
      passent par l'animateur des animations CSS et suivent la même horloge.
      Limites : pas de `playbackRate`, de `reverse()`, de `commitStyles()` ni de
      `setKeyframes()` ; `composite`, `iterationStart`, `endDelay` et la temporisation
      propre à une image clé sont ignorés ; `getAnimations()` ne rend pas les
      animations et transitions CSS.
- [x] `overscroll-behavior` (et `-x`, `-y`) : hors de `auto`, un défilement arrivé en
      bout de course ne se propage pas à la zone englobante. Le lanceur WPT fait
      défiler la page sous un doigt qui glisse, comme le ferait une fenêtre tactile.
- [x] Accès nommé : `window.identifiant` (et `typeof identifiant`) désigne l'élément de
      cet identifiant ; `document.nom` désigne l'élément `embed`, `form`, `iframe`,
      `img` ou `object` de ce nom. Seule une variable nue (`identifiant`) y parvenait.
      Sélecteurs `:lang()` et `:dir()` ; `:disabled` et `:enabled` tiennent compte d'un
      `<fieldset disabled>` englobant. Un élément personnalisé déplacé par
      `moveBefore()` reçoit `connectedMoveCallback`, à défaut `disconnectedCallback`
      puis `connectedCallback`.
      Limites : `document.nom` rend le premier élément, jamais une collection, et
      passe après les propriétés du prototype ; chaque accès nommé parcourt le
      document ; `dir=auto` est lu comme `ltr`.
- [x] Arbres fantômes déclaratifs : `<template shadowrootmode="open|closed">` donne un
      arbre fantôme à son parent dès l'analyse du document, même sans script. Un
      groupe de boutons radio est celui de son arbre (document ou arbre fantôme) ;
      `slotchange` part aussi quand le contenu de repli d'un emplacement change.
      Limites : pas dans les cadres ni par `setHTMLUnsafe` ; les options
      (`shadowrootdelegatesfocus`, `shadowrootclonable`…) sont ignorées.
- [x] Analyseur HTML : les attributs des éléments SVG et MathML retrouvent leur casse
      (`viewBox`, `preserveAspectRatio`…) et leur espace de noms (`xlink:href`,
      `xml:lang`, `xmlns`). Sélecteurs : `*|nom`, `[*|attribut]` ; hors du HTML, un nom
      d'attribut se compare tel qu'il est écrit. `String(classList)` rend l'attribut
      tel quel ; `item(rang)` ne rend jamais un élément nommé ; `namedItem` ne lit
      `name` que sur un élément HTML ; `document.location` vaut `null` pour un
      document sans fenêtre ; `createEvent` connaît les interfaces de la norme ;
      `offsetX` et `offsetY` sont relatifs à la cible pendant la distribution.
      Limite : `[viewbox]` ne trouve plus un `<svg viewBox>` (comme dans les
      navigateurs) ; pas de préfixe d'espace de noms déclaré (`ns|nom`).
- [x] `createElement` dans un document XML crée un élément sans espace de noms
      (un `Element`, pas un `HTMLElement`) ; `attributeNamespace` des enregistrements
      de mutation ; `normalize()` selon la norme, plages vivantes comprises ; un nœud
      retiré d'un sous-arbre observé reste suivi jusqu'à la remise suivante ;
      `observe()` et `disconnect()` ne reçoivent plus les modifications antérieures ;
      `defaultView` vaut `null` pour un document créé par un script.
- [x] Web Platform Tests `dom/` : 70 % des fichiers atteints (486 / 693, 70,1 %).
      Les plus gros lots restants : `moveBefore` (33), `events/scrolling` (25 :
      défilement doux, accrochage, cadres), `ranges/tentative` (19),
      `NodeList` figée (6),
      `Document-contentType` (12), royaumes des cadres (~15).
- [x] Encodages de texte : une page, une feuille de style ou un script qui n'est pas en
      UTF-8 est décodé d'après l'en-tête HTTP `Content-Type`, sa marque d'ordre d'octets,
      son `<meta charset>`, et à défaut en UTF-8 s'il en est, sinon en `windows-1252`
      (tout était lu en UTF-8). Encodages lus : UTF-8, UTF-16, les 28 encodages à un
      octet de la norme, `x-user-defined`. `TextDecoder` les lit aussi, flux UTF-16
      compris.
      Limites : la recherche de `<meta charset>` lit les 1024 premiers octets
      sans suivre l'algorithme de pré-analyse de la norme ; pas de `TextEncoder` autre
      qu'UTF-8 (conforme à la norme) ; l'encodage des formulaires reste UTF-8.
- [x] Adresse de base : `a.href`, `img.src` et `baseURI` se résolvent contre le
      `<base href>` du document, même posé par un script ; `URLSearchParams` se
      parcourt en suivant les suppressions en cours de route.
      Limite : les liens suivis au clic gardent la base relevée au chargement.
- [x] Web Platform Tests `url/` (16 fichiers sur 32) et `encoding/` (18 sur 153) branchés
      et mesurés.
- [x] Encodages à plusieurs octets : `Shift_JIS`, `EUC-JP`, `ISO-2022-JP`, `EUC-KR`,
      `Big5`, `GBK` et `gb18030` se lisent (pages, feuilles de style, scripts,
      `TextDecoder`), ainsi que `replacement` ; tous les encodages de la norme sont
      donc lus. `TextDecoder` suit `stream: true` pour tous, UTF-8 compris (un
      caractère coupé entre deux morceaux d'une réponse devenait U+FFFD) : marque
      d'ordre d'octets en plusieurs morceaux, caractère inachevé gardé pour la suite.
      Un cadre de même origine sans encodage déclaré hérite de celui de la page.
      Web Platform Tests `encoding/` : 18 -> 44 fichiers sur 153, 11 559 -> 88 371
      sous-tests.
      Le lanceur WPT accorde désormais son délai en temps processeur du fil du test,
      plus en temps d'horloge : la mesure dépend moins de la charge de la machine
      (les tests qui mesurent eux-mêmes le temps réel y restent sensibles). Il décode
      les tests et leurs cadres comme le navigateur (`Content-Type` du fichier
      `.headers`, marque d'ordre d'octets, `<meta charset>`, héritage).
      Limites : aucun encodeur autre qu'UTF-8 (formulaires et URL d'une page en
      `Shift_JIS` partent en UTF-8 ; 71 fichiers `encoding/` en dépendent) ;
      `document.characterSet` vaut toujours `UTF-8` ; un document sans encodage
      déclaré mais valable en UTF-8 est lu en UTF-8 (la norme veut `windows-1252`) ;
      18 fichiers de décodage de plus de dix mille sous-tests dépassent le délai du
      lanceur (vitesse : étape L) ; `TextDecoderStream` reste incomplet.
- [x] Web Platform Tests `html/` branché et mesuré : 711 fichiers sur 7 254 (9,8 %) à la
      première mesure. Plus gros lots : `canvas` (2 426 fichiers), contenus embarqués
      (759), scripts (525), formulaires (514), navigation (343), focus (206),
      `document.write` (159). Trois fichiers très longs (`svg-in-iframe-*`) atteignent
      parfois le délai du lanceur dans une mesure complète.
- [x] Validation des formulaires : `validity` (`valueMissing`, `typeMismatch`,
      `patternMismatch`, `rangeUnderflow`, `rangeOverflow`, `stepMismatch`,
      `customError`), `willValidate`, `validationMessage`, `checkValidity()`,
      `reportValidity()`, `setCustomValidity()`, événement `invalid` ; un formulaire
      invalide n'est plus envoyé (sauf `novalidate` et `formnovalidate`) ; `:invalid` et
      `:valid`.
      Limites : `tooLong`, `tooShort` et `badInput` valent toujours `false` ;
      `reportValidity()` donne le focus au champ fautif sans afficher de bulle ;
      `:invalid` ne juge que `required`.
- [x] `<dialog>` : masqué tant qu'il n'est pas ouvert (il s'affichait comme un bloc
      ordinaire), `show()`, `showModal()`, `close()`, `requestClose()`, `returnValue`,
      événements `cancel` et `close`, touche Échap, `<form method=dialog>`, focus rendu
      à la fermeture, `:modal` et `:open`. Popovers : attribut `popover`,
      `showPopover()`, `hidePopover()`, `togglePopover()`, boutons `popovertarget`,
      fermeture au clic ailleurs et à Échap, `beforetoggle` et `toggle`,
      `:popover-open`.
      Limites : pas de `::backdrop` ; une boîte modale ne rend pas le reste de la page
      inerte ; pas de `closedby`, de popover `hint` ni d'ancrage au bouton ; boîtes et
      popovers passent devant par un `z-index` maximal, pas par une vraie couche
      supérieure.
- [x] L'état d'activation d'un clic (case cochée, bouton) est protégé du
      ramasse-miettes pendant la distribution de l'événement : il pouvait être recyclé
      avant d'être relu.
- [x] `document.write()`, `writeln()`, `open()`, `close()` : pendant un script, le texte
      écrit est inséré juste après ce script (dans `<body>` pour du contenu écrit depuis
      `<head>`), et les scripts qu'il contient s'exécutent ; hors d'un script, le
      document est remplacé. Un `<script for=… event=…>` ne s'exécute que pour
      `for=window event=onload`.
      Limites : chaque appel est analysé seul (une balise ouverte par un appel et
      fermée par un autre se referme dès le premier) ; un script externe écrit ainsi
      ne bloque pas la suite ; `open()` ne retire pas les écouteurs.
- [ ] Web Platform Tests `html/` : de 11,8 % vers 30 % des fichiers (cadres, `canvas`,
      contenus embarqués, scripts).
- [x] Analyseur XML 1.0 avec espaces de noms (`oxyde_html::parse_xml`) : documents
      bien formés seulement (première faute signalée avec sa ligne et sa colonne),
      DOCTYPE et entités internes de son sous-ensemble, entités du HTML pour les
      DOCTYPE XHTML, développement des entités borné. Il sert `DOMParser` (à la place
      de l'analyseur réduit écrit en JavaScript), `XMLHttpRequest.responseXML` et
      `responseType` `document`, `arraybuffer` et `blob`, et les pages et cadres
      servis en XML (`.xhtml`, `.svg`, `.xml`) : leur document est un `XMLDocument`
      dont `contentType` dit le type, et `createElement` n'y met pas les noms en
      minuscules. Un document mal formé laisse place à une page qui dit la faute.
      L'import d'un nœud d'un document dans un autre traduit ses espaces de noms.
      Le lanceur WPT exécute aussi les tests `.xhtml`, `.xht` et `.svg` : `dom/` passe de
      489 fichiers sur 693 à 533 sur 736 (72,4 %), `html/` de 826 sur 7 254 à 847 sur
      7 308 (11,6 %).
      Limites : aucune ressource externe (DTD, entité externe) ; valeurs par défaut
      des `<!ATTLIST>` ignorées ; pas de validation ; le navigateur reconnaît un
      document XML à l'extension de son adresse, pas à son en-tête `Content-Type`
      (qui n'arrive pas jusqu'au processus de rendu) ; une page mal formée n'affiche
      pas le début du document ; l'affichage d'un document SVG ou XML sans
      élément HTML n'a pas été vérifié ; `XMLHttpRequest` envoie ses noms
      d'en-têtes en minuscules.
- [x] Shadow DOM : `attachShadow` (ouvert ou fermé), `ShadowRoot` (`host`, `mode`,
      `innerHTML`, `getElementById`, `activeElement`), emplacements `<slot>` nommés et par
      défaut avec contenu de repli, `assignedSlot`, `assignedNodes`, `assignedElements`,
      `slotchange`, `getRootNode({ composed })`, `isConnected` à travers les arbres. Les
      styles et la mise en page suivent l'arbre aplati ; une feuille ne vaut que dans son
      arbre, avec `:host`, `:host(...)` et `::slotted(...)` (le document l'emporte sur
      eux). Les événements composés sortent de l'arbre fantôme et sont reciblés sur
      l'hôte ; les touches visent l'élément qui a vraiment le focus ; les éléments
      personnalisés imbriqués sont mis à niveau et reliés.
      Limites : pas de `adoptedStyleSheets` (les bibliothèques se replient sur
      `<style>`), pas d'arbre fantôme déclaratif (`<template shadowrootmode>`), pas de
      `slot.assign()`, pas de `:host-context()` ni de `::part()` ; `<link
      rel=stylesheet>` ignoré dans un arbre fantôme ; `relatedTarget` et
      `composedPath()` non reciblés pour un arbre fermé ; `delegatesFocus` sans effet ;
      un hôte glissé dans son propre arbre fantôme n'est pas refusé ; la distribution
      dans les emplacements est recalculée à chaque lecture (à reprendre à l'étape L).
      Le dossier `shadow-dom/` de Web Platform Tests n'est pas dans la copie locale :
      non mesuré. YouTube reste sur son squelette (une autre erreur de script le
      retient).
- [x] `IntersectionObserver`, `ResizeObserver`, `requestIdleCallback` : faits, par relevé de
      la géométrie toutes les 100 ms (pas à chaque image) ; l'intersection ne tient pas compte
      du rognage des ancêtres défilants.
- [x] Scripts insérés par les scripts (`createElement("script")`), `document.currentScript`,
      images et feuilles de style ajoutées après le chargement, `<noscript>` inerte,
      décompression `gzip` et `deflate`, sélecteurs `:has()`, `:scope`, `:nth-of-type`,
      `crypto.getRandomValues`, `MessageChannel`, `postMessage` (vers la fenêtre elle-même).
- [x] `Intl` réduit (français et anglais : nombres, dates, pluriels, listes, durées relatives,
      comparaison, découpage), `String.prototype.normalize`, modules chargés à la demande
      (`import()`, module inséré par script), cartes d'import (table `imports`).
- [x] `DOMParser` (HTML et XML), `template.content`, éléments
      personnalisés (`customElements`, classes qui étendent `HTMLElement` ; rappels à la fin de
      la tâche), flux (`ReadableStream`, `WritableStream`, `TransformStream`),
      `document.elementFromPoint`, interfaces `Window`, `Navigator`, `SVGElement`… ; les
      messages d'erreur nomment la méthode ou la valeur fautive.
- [ ] Vus à la mesure des sites et encore absents : Shadow DOM, canevas (`Path2D` existe sans
      rien dessiner), `document.write`,
      `postMessage` entre fenêtres (la source n'est pas connue), `crypto.subtle`, Brotli
      (demande un crate à télécharger), chargement des ressources en parallèle.
- [x] Workers : `new Worker(adresse)` exécute son script dans un royaume JavaScript à lui
      (ni `window` ni `document` ; `self`, `postMessage`, `onmessage`, `close`,
      `importScripts`, `name`, `WorkerGlobalScope`, plus `fetch`, minuteurs, `URL`…), les
      erreurs non rattrapées remontent à l'objet `Worker`. Adresses `blob:`
      (`URL.createObjectURL`, `revokeObjectURL`), lisibles par `fetch`, `XMLHttpRequest`
      et `Worker` (workers en ligne). `structuredClone` clone entre royaumes (nature lue
      à l'étiquette de l'objet), garde les tampons partagés par plusieurs tableaux,
      refuse ce qui n'est pas clonable et transfère `ArrayBuffer` (l'original est
      détaché) et `MessagePort` ; `postMessage` des ports et des workers prend une liste
      de transfert et l'événement porte `ports`.
      Limites : un worker partage le fil de la page (un calcul long la fige, dans la
      limite du budget d'une tâche) ; `importScripts` ne rend que les scripts dont
      l'adresse est écrite en toutes lettres dans le source (lus d'avance) ; pas de
      worker de type module, de `SharedWorker` ni de service worker ; `terminate()`
      ne libère pas le royaume (32 workers au plus par page) ; un `Blob` cloné est le
      même objet ; une adresse `blob:` ne se lit pas par `<img src>` ; les tests WPT
      `.worker.js` ne sont pas exécutés par le lanceur.
- [x] `WebSocket` : client RFC 6455 dans `oxyde_net` (poignée de main HTTP/1.1 sur TCP ou
      TLS, trames, masquage, fragments, ping et pong, fermeture dans les règles, cookies
      du site, sous-protocoles) ; classe `WebSocket` des pages (`send` de texte, de
      tampons et de `Blob`, `binaryType`, `close`, événements `open`, `message`, `error`,
      `close`), aussi dans les cadres et les workers. Les connexions sont ouvertes par
      le service réseau : la page donne ses ordres (`Page::take_socket_commands`), le
      processus de rendu les passe au navigateur, qui vérifie qu'ils viennent bien du
      site affiché, puis au processus réseau ; les nouvelles reviennent par le même
      chemin. Une page quittée perd ses connexions.
      Limites : aucune extension (pas de compression `permessage-deflate`) ; pas de
      WebSocket sur HTTP/2 ; `bufferedAmount` vaut toujours 0 ; les captures d'écran
      (`--screenshot`) et le lanceur WPT n'ouvrent pas de connexion ; la chaîne
      complète entre processus n'a pas de test de bout en bout (chaque maillon a les
      siens, contre un serveur local) et n'a pas été essayée sur un vrai site.
- [x] `EventSource` : `oxyde_net` sait lire une réponse au fil de l'eau
      (`Client::stream` : en-tête, morceaux du corps décompressés à mesure, fin ou
      échec, abandon possible d'un flux muet). La classe `EventSource` des pages
      découpe le flux (champs `data`, `event`, `id`, `retry`, commentaires, fins de
      ligne CR, LF et CR LF, caractères coupés entre deux morceaux), se reconnecte
      après une coupure avec `Last-Event-ID`, et renonce si la réponse n'est pas un
      flux d'événements (code autre que 200, type autre que `text/event-stream`, autre
      origine non autorisée). Elle passe par le canal des connexions `WebSocket`.
      Limites : `withCredentials` est sans effet ; le délai de reconnexion ne s'allonge
      pas après des échecs répétés ; chaque flux occupe un fil du service réseau ;
      seuls `gzip` et `deflate` sont décompressés au fil de l'eau ; `fetch` ne lit
      toujours pas ses réponses au fil de l'eau (`Response.body` reste `null`) ; comme
      pour `WebSocket`, pas de test de bout en bout entre processus ni d'essai sur un
      vrai site.
- [ ] Liste des suffixes publics (frontière des sites, cookies).
- [ ] Les 656 tests de test262 restants, par leurs causes de fond : chaînes UTF-16 (substituts
      isolés), tableaux denses, objet de type `document.all`.

Terminé quand 40 des 50 sites s'affichent correctement et que 25 permettent leur action
principale.

### I — La page est fidèle (alpha)

Fin du jalon 6. Mesure : les tests de référence WPT (comparaison d'images), à brancher.

- [ ] Lanceur des tests de référence WPT ; point de départ publié dans `conformance/`.
- [ ] Dessin des cadres (`<iframe>`), et cadres d'un autre site dans leur propre processus.
- [ ] `position: sticky`, modes d'écriture verticaux, `<col>`, ligne de base des tableaux.
- [ ] API Web Animations ; animations et défilement fluide joués par le compositeur, hors du
      fil principal ; calques mis en cache.
- [ ] Web Platform Tests de mise en page : de 24 % à 60 % des fichiers.
- [ ] SVG (formes, tracés, texte, dégradés) : presque tous les sites en affichent.
- [ ] Polices : repli par écriture, emoji en couleur, `font-display`, `unicode-range`.
- [ ] Images : GIF animé, AVIF, `srcset` et `<picture>`, chargement paresseux.
- [ ] `<canvas>` 2D.

Terminé quand les 50 sites de référence sont visuellement proches de leur rendu dans
Chromium, écarts notés un par un.

### J — Rien ne se perd (alpha)

- [ ] **Cookies sur disque** (pot du processus réseau), `localStorage` et `sessionStorage`
      sur disque : sans eux, aucune connexion à un site ne survit à la fermeture.
- [ ] **Téléchargements** : reconnaître une réponse à enregistrer (`Content-Disposition`,
      type non affichable), annuler la navigation sans perdre la page affichée, écrire dans
      le dossier Téléchargements sous un nom assaini et jamais écrasé, panneau et rubrique
      de la bibliothèque (maquette `Panels`), progression et annulation — ce qui demande à
      `oxyde_net` de rendre le corps au fil de l'eau.
- [ ] Cache HTTP sur disque (validation, expiration, quota).
- [ ] Verrou sur le profil : une seule fenêtre normale y écrit.
- [ ] Reprise après un arrêt brutal : la session écrite est toujours relisible.
- [ ] Dossiers de favoris ; import depuis Firefox.

### K — On peut lui faire confiance (bêta)

Fin du jalon 5. Un navigateur lit du contenu hostile : sans ce bloc, pas de testeurs.

- [ ] Bac à sable **`AppContainer`** pour les processus de rendu ; `win32k` verrouillé ;
      le processus réseau privé de ce dont il n'a pas besoin.
- [ ] Processus GPU séparé.
- [ ] Un processus par site, cadres compris ; deux onglets d'un même site partagent le leur.
- [ ] Erreurs de certificat complètes (révocation, HSTS, liste de préchargement) ; contenu
      mixte bloqué ; mode « HTTPS seulement ».
- [ ] CSP, `X-Frame-Options`, `Referrer-Policy`, CORP, COOP et COEP.
- [ ] Autorisations par site (localisation, notifications, caméra, micro, presse-papiers),
      avec l'écran « Infos du site » de la maquette.
- [ ] **Fuzzing** des analyseurs qui lisent le réseau (HTML, CSS, JavaScript, images,
      polices, HTTP, IPC) ; Miri sur les crates qui ont du code `unsafe`.
- [ ] Audit des dépendances (`cargo audit`, `cargo vet`) dans le contrôle de chaque commit.
- [ ] Intégration continue sur le dépôt : le contrôle n'est aujourd'hui lancé qu'à la main.

### L — Il est assez rapide (bêta)

ORDRE : décidé le 14 novembre 2026, cette étape se fait **après l'étape P**. Les étapes
H à K puis M à P passent d'abord : qu'il ne manque plus rien au navigateur, ensuite la vitesse.
Un défaut qui empêche une page de se charger (coût quadratique, boucle sans fin) se corrige
dans l'étape en cours ; le réglage de vitesse attend.

Jalon 8. Mesure : `oxyde_bench`, puis les bancs publics de l'étape R dès que le moteur les
fait tourner ; chiffres publiés, jamais en régression.

- [ ] Caches en ligne pour les accès aux propriétés ; formes cachées des objets.
- [ ] Chaînes UTF-16 ou à double représentation (règle aussi une cause d'échec de test262).
- [ ] JIT de base, puis JIT optimisant.
- [ ] Ramasse-miettes générationnel, puis concurrent.
- [ ] WebAssembly.
- [ ] Style, mise en page et peinture incrémentaux : ne refaire que ce qui a changé.
- [ ] Analyse du HTML au fil de l'eau ; chargement des ressources en parallèle ; HTTP/2 et
      HTTP/3 vérifiés sur les sites de référence.
- [ ] Mémoire : mise en veille des onglets inactifs, plafond par processus de rendu.
- [ ] Élucider le cœur occupé au repos avec le pilote AMD.

Terminé quand les 50 sites de référence se chargent en moins de deux fois le temps de
Chromium sur la même machine.

### M — Le reste du navigateur (bêta)

Fin du jalon 7 : les écrans de la maquette qui manquent.

- [ ] Infos du site et autorisations (avec K).
- [ ] Menus contextuels (page, lien, image, onglet) et invites (`alert`, `confirm`,
      `prompt`, `beforeunload`, sélecteur de fichier).
- [ ] **Mots de passe** : enregistrement à la connexion, remplissage, écran de gestion,
      chiffrement par DPAPI, déverrouillage par Windows Hello avant tout affichage ou copie.
- [ ] **Codes à usage unique (TOTP)**, à côté du mot de passe (maquettes « Mots de passe,
      codes et clés d'accès » et « Proposition · Connexion ») :
  - calcul selon la RFC 6238 (HMAC-SHA-1, SHA-256 et SHA-512 ; 6 ou 8 chiffres ; période
    de 30 s par défaut), vérifié sur les vecteurs de test de la RFC ;
  - enregistrement d'un secret : adresse `otpauth://`, QR code de configuration repéré
    dans la page, ou saisie à la main ;
  - **saisie à la place de l'utilisateur** : le champ du code est reconnu
    (`autocomplete="one-time-code"`, champs à six cases) et rempli, avec une bulle qui dit
    ce qui a été saisi et quand le code expire ;
  - le code n'est saisi que sur le site où il a été enregistré, jamais sur un autre ni
    dans un cadre d'un autre site ;
  - le secret est chiffré comme les mots de passe et ne quitte pas le processus du
    navigateur : le processus de rendu ne reçoit que le code du moment ;
  - réglage « Demander Windows Hello avant de saisir un code », éteint par défaut : garder
    le code à côté du mot de passe réduit les deux facteurs à un seul coffre, et ce
    réglage est le moyen de garder un geste distinct ;
  - export et import des secrets avec les mots de passe.
- [ ] **Clés d'accès (passkeys)** :
  - API WebAuthn des pages (`navigator.credentials.create` et `get`,
    `PublicKeyCredential`, remplissage conditionnel), mesurée par les Web Platform Tests
    `webauthn/` ;
  - d'abord par l'authentificateur de Windows (Windows Hello, clés de sécurité USB et
    NFC), dont les clés restent dans le module de sécurité de la machine ;
  - puis un coffre propre à Oxyde (clés ES256, chiffrées comme les mots de passe), qui
    permettra la synchronisation entre appareils ;
  - une clé n'est proposée que pour le site qui l'a créée : c'est ce qui la rend
    insensible à l'hameçonnage, et la vérification se fait dans le processus du navigateur ;
  - invites de la maquette : choisir une clé pour se connecter, créer une clé à la place
    du mot de passe ; gestion dans l'écran des mots de passe (détails, suppression).
- [ ] Impression : pagination, aperçu, envoi au spouleur Windows, export PDF.
- [ ] Réglages encore absents : thème clair et « comme Windows », zoom par site, langue,
      police par défaut, page d'accueil.
- [ ] Zoom (Ctrl +, Ctrl −), plein écran (F11), glisser-déposer des onglets, défilement de
      la bande d'onglets et de la palette.
- [ ] Teinte violette complète de la fenêtre privée.
- [ ] **Icônes des sites** : télécharger l'icône déclarée par la page (`<link rel="icon">`,
      sinon `/favicon.ico`), la conserver dans le profil, l'afficher dans les onglets,
      l'historique, les favoris et la palette à la place de l'initiale ; en tirer la couleur
      dominante.
- [ ] **Écran de chargement** (maquette « Proposition · Chargement d'une page ») : quand une
      page tarde, le volet montre l'icône du site entourée d'un anneau animé, son nom, son
      hôte avec le cadenas et l'étape en cours, sur un fond teinté par l'icône, plutôt
      qu'un écran blanc. Dépend des icônes des sites. Règles :
  - l'ancienne page reste affichée pendant les 400 premières millisecondes : un site
    rapide ne montre jamais l'écran ;
  - l'écran s'efface dès le premier contenu de la page, pas à la fin du chargement ;
  - c'est une surface du navigateur : l'hôte réel y figure toujours, et une page ne peut
    ni le dessiner ni le modifier ;
  - il occupe le volet de la page, pas la fenêtre : on peut toujours changer d'onglet ou
    arrêter (Échap) ; l'onglet porte le même anneau, « Actualiser » devient « Arrêter » ;
  - première visite : l'initiale et la couleur de l'onglet, remplacées par l'icône dès
    qu'elle arrive ;
  - après 8 secondes : « Ce site met du temps à répondre », avec « Continuer d'attendre »
    et « Arrêter » ;
  - animations réduites dans Windows : icône fixe et barre de progression.

  Les seuils de 400 ms et de 8 s sont des propositions, à régler à l'usage.
- [ ] Accessibilité du contenu des pages (arbre AccessKit des pages distantes), contraste
      et navigation au clavier de toute l'interface.
- [ ] Fuseaux horaires IANA et calendriers pour `Temporal`, `Date` à l'heure locale, `Intl`.

### N — On peut le distribuer (bêta)

- [ ] **Installateur** Windows signé ; désinstallation propre.
- [ ] **Mises à jour automatiques** signées, avec retour en arrière : sans elles, une faille
      découverte chez les testeurs ne peut pas être corrigée.
- [ ] Devenir le navigateur par défaut (associations de fichiers et de protocoles).
- [ ] Rapports de plantage, envoyés seulement avec l'accord de l'utilisateur.
- [ ] Canaux (nocturne, bêta, stable) et numéro de version réel.
- [ ] Politique de confidentialité et page « À propos » avec les licences des dépendances.

**Bêta** quand H à N sont cochés et qu'une personne a tenu une semaine avec Oxyde seul.

### O — Médias (1.0)

- [ ] `<audio>` et `<video>` : conteneurs MP4 et WebM, AV1 et VP9 (décodeurs Rust), H.264
      par le décodeur de Windows, AAC et Opus, sous-titres, plein écran.
- [ ] Web Audio ; `MediaSource` (lecture adaptative) ; capture (`getUserMedia`).
- [ ] WebGL, puis WebGPU.
- [ ] WebRTC.
- [ ] Contenu protégé (EME) : à trancher, il n'existe pas de module libre.

### P — Plateforme étendue (1.0)

- [ ] IndexedDB, Cache API, quotas ; service workers et notifications.
- [ ] Protocole DevTools et écran DevTools de la maquette.
- [ ] Extensions (WebExtensions, Manifest V3) et leur écran.
- [ ] Lecteur PDF.
- [ ] Synchronisation des favoris et des onglets entre appareils (« Onglets d'autres
      appareils » de la maquette).

### Q — Autres systèmes (1.0 et après)

Jalon 10, par `oxyde_platform` : Linux, macOS, puis Android ; iOS et iPadOS restreignent les
moteurs tiers et le JIT, à trancher le moment venu.

### R — Les bancs d'essai publics (1.0)

La version 1.0 ne se contente pas de fonctionner : elle doit tenir son rang sur les bancs
d'essai que tout le monde peut lancer. La comparaison est publiée dans
[bench/comparaisons.md](../bench/comparaisons.md), avec sa méthode, face à deux groupes :

- les **navigateurs établis** — Chrome, Edge, Opera, Brave, Vivaldi (tous sur le moteur de
  Chromium) et Firefox —, qui disent où est la barre ;
- les **moteurs indépendants**, vrais concurrents d'Oxyde — Servo, Verso, Gosub et Blitz,
  écrits en Rust, et Ladybird, en C++.

Bancs retenus : [Speedometer 3](https://browserbench.org/Speedometer3.0/) (réactivité
d'applications web réelles, le plus proche de l'usage), JetStream (JavaScript et
WebAssembly), MotionMark (rendu graphique), [ApexMark](https://apexmark.tech/fr/)
(processeur, carte graphique, WebGPU et mémoire vus du navigateur), WebXPRT 4 (tâches de
tous les jours) et Basemark Web 3.0 (panorama).

- [ ] Lancer chaque banc dans Oxyde et noter où il s'arrête : c'est l'état de départ, et
      la liste de ce qui manque en découle (elle recoupe H, I, L, O et P).
- [ ] Mesurer les navigateurs établis sur la machine de référence, selon la méthode du
      document (même jour, cinq passes, médiane, versions exactes).
- [ ] Mesurer les moteurs indépendants de la même façon, en notant la préversion essayée
      et les bancs qu'ils ne terminent pas ; reprendre leur conformité publiée (test262,
      Web Platform Tests) à côté de la nôtre.
- [ ] **Devant tous les moteurs indépendants écrits en Rust** sur chaque banc que l'un
      d'eux termine.
- [ ] **Speedometer 3 se termine**, puis atteint la moitié du meilleur score.
- [ ] **JetStream se termine** (WebAssembly et JIT, étape L), puis atteint le tiers.
- [ ] **MotionMark se termine** (`canvas`, SVG, compositeur, étape I), puis atteint la moitié.
- [ ] **ApexMark se termine** (workers, WebGL et WebGPU, étape O) ; score publié avec la
      version du banc, qui se dit encore expérimental.
- [ ] WebXPRT 4 et Basemark Web 3.0 se terminent ; scores publiés.
- [ ] Chaque version publiée relance les bancs ; un score qui recule est expliqué ou corrigé
      avant la sortie.

Les seuils sont des propositions, volontairement sous la parité : un chiffre honnête et
tenu vaut mieux qu'une promesse. Un banc qui ne se termine pas est noté comme tel, jamais
par un score partiel.

**1.0** quand O à R sont cochés.

### Règles tenues jusqu'au bout

- Une case n'est cochée qu'avec sa mesure : suite publique, site de référence, ou test.
- Aucune suite ne régresse ; le contrôle (`fmt`, `clippy`, tests, `cargo deny`) reste vert.
- Chaque palier se termine par un essai à la main, noté dans `conformance/` : ce que les
  tests ne voient pas se voit là.
- Ce qui n'agit pas n'est pas montré : pas de réglage ni de bouton sans effet.
