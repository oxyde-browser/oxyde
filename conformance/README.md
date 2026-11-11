# Conformance

Ce dossier suit l'écart entre Oxyde et les standards du web. L'objectif est un comportement
identique à Chromium ; l'avancement se mesure avec les suites publiques, pas à l'œil.

## Suites de tests

| Suite | Crate visé | État |
|---|---|---|
| html5lib-tests (tokenizer, tree-construction) | `oxyde_html` | **pas encore branchée** — `--dump-dom` produit déjà le format attendu |
| css-parsing-tests | `oxyde_css` | pas encore branchée |
| Web Platform Tests (sous-ensembles DOM, CSS) | `oxyde_css`, `oxyde_layout` | pas encore branchée — `--screenshot --page-only` servira aux tests de référence |
| test262 | `oxyde_js` | moteur non commencé |

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
- Pas de JavaScript.
- Barre d'adresse : pas de sélection partielle ni de placement du curseur à la souris.
- Onglets épinglés, espaces et onglets fermés ne sont pas conservés entre deux sessions.
- Les polices embarquées pour l'interface (Inter, Space Grotesk, JetBrains Mono) sont aussi visibles des pages web ; Chromium réserve les siennes à l'interface.
- Accessibilité : seul le pourtour est exposé ; une page web apparaît comme un document sans arbre détaillé.
- Le bouton Paramètres est inactif (écran prévu au jalon 7).
