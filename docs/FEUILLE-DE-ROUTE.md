# Feuille de route d'Oxyde

État au 12 novembre 2026, branche `test262-conformite`, commit `73d6cdb`.

- `oxyde_js` réussit **48 128 tests de test262 sur 48 795 (98,6 %)** ; il en reste 667.
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
  (9,4 %), aujourd'hui 538 (23,7 %); une lecture force la
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
  donne aux tests un temps virtuel cohérent. Mesure : 50 fichiers sur 363 (13,8 %). Restent :
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

Reste : les dossiers de favoris et l'import ; les écrans (téléchargements, infos du site,
autorisations, recherche dans la page, navigation privée, mots de passe,
premier lancement, onglets horizontaux, menus contextuels, impression) ; la persistance des
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
