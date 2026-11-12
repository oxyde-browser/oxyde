# Oxyde face aux autres navigateurs

Ce document recevra les résultats d'Oxyde sur les bancs d'essai publics, à côté de ceux des
navigateurs établis et des autres moteurs indépendants, en particulier ceux écrits en Rust. **Rien n'y est encore mesuré** : aujourd'hui Oxyde ne fait tourner
aucun de ces bancs jusqu'au bout. Les cases vides le restent tant qu'une mesure réelle ne les
remplit pas ; aucun chiffre n'est estimé ni repris d'ailleurs.

Le plan qui y mène est l'étape R de la [feuille de route](../docs/FEUILLE-DE-ROUTE.md).

## Bancs retenus

| Banc | Ce qu'il mesure | Ce qu'il exige d'Oxyde |
|---|---|---|
| [Speedometer 3](https://browserbench.org/Speedometer3.0/) | la réactivité d'applications web réelles (listes de tâches, éditeurs, graphiques) : DOM, style, mise en page, JavaScript ensemble | saisie dans les pages, éléments personnalisés, `canvas`, SVG, cadres |
| [JetStream](https://browserbench.org/JetStream/) | le débit de JavaScript et de WebAssembly sur des programmes lourds | WebAssembly, workers, JIT |
| [MotionMark](https://browserbench.org/MotionMark/) | le rendu graphique : combien d'éléments animés à 60 images par seconde | `canvas` 2D, SVG, transformations et animations composées |
| [ApexMark](https://apexmark.tech/fr/) | l'appareil vu du navigateur : processeur, carte graphique, WebGPU, mémoire | workers, WebAssembly, WebGL, WebGPU |
| [WebXPRT 4](https://www.principledtechnologies.com/benchmarkxprt/webxprt/) | des tâches de tous les jours (retouche de photos, tableur, chiffrement) | `canvas`, WebAssembly, workers |
| [Basemark Web 3.0](https://web.basemark.com/) | un panorama : DOM, CSS, `canvas`, WebGL | WebGL, `canvas` |

Speedometer est le banc qui ressemble le plus à l'usage réel : c'est lui qui compte d'abord.
ApexMark se dit encore expérimental, et sa méthode peut changer ; ses résultats seront donc
datés et accompagnés de sa version.

## Navigateurs comparés

Deux groupes, parce qu'ils ne répondent pas à la même question.

**Les navigateurs établis** disent où est la barre. Chrome, Edge, Opera, Brave et Vivaldi
partagent le même moteur (Blink et V8, de Chromium) : leurs scores sont proches, et l'écart
entre eux mesure surtout leurs réglages et leurs ajouts. Firefox a le sien.

| Navigateur | Moteur de rendu | Moteur JavaScript |
|---|---|---|
| Chrome | Blink | V8 |
| Edge | Blink | V8 |
| Opera | Blink | V8 |
| Brave | Blink | V8 |
| Vivaldi | Blink | V8 |
| Firefox | Gecko (style et composition en Rust : Stylo, WebRender) | SpiderMonkey |

Safari (WebKit, JavaScriptCore) n'existe pas sous Windows : il rejoindra le tableau avec le
portage macOS.

**Les moteurs indépendants** sont les vrais concurrents d'Oxyde : des projets qui écrivent un
moteur au lieu de reprendre Chromium. C'est face à eux que la comparaison est la plus parlante.

| Projet | Langage | Ce que c'est |
|---|---|---|
| [Servo](https://servo.org/) | Rust | moteur de rendu né chez Mozilla, aujourd'hui à la Linux Foundation Europe ; son moteur JavaScript est SpiderMonkey, en C++ |
| [Verso](https://github.com/versotile-org/verso) | Rust | navigateur construit sur Servo |
| [Blitz](https://github.com/DioxusLabs/blitz) | Rust | moteur HTML et CSS sans JavaScript : comparable sur la mise en page seulement |
| [Gosub](https://gosub.io/) | Rust | moteur complet, à un stade précoce |
| [Ladybird](https://ladybird.org/) | C++ | moteur entièrement indépendant ; pas du Rust, mais la même ambition |

Oxyde semble être le seul de ces projets dont le moteur JavaScript est lui aussi
écrit en Rust et sans dépendance C ou C++ ; c'est à vérifier projet par projet avant de
l'écrire ailleurs que dans ce document. L'état de ces projets change vite : la liste est à
revoir à chaque mesure, et un projet abandonné sort du tableau.

## Méthode

Pour qu'une comparaison vaille quelque chose :

- **Même machine, même jour** pour tous les navigateurs ; le modèle, le processeur, la carte
  graphique, la mémoire et la version de Windows sont notés avec les résultats.
- **Sur secteur**, sans autre programme ouvert, fenêtre de même taille, profil neuf, sans
  extension ; les bloqueurs intégrés (Brave, Opera, Vivaldi) laissés à leur réglage d'origine,
  et ce réglage noté.
- **Cinq passes** par banc et par navigateur ; on publie la médiane et l'écart entre la plus
  faible et la plus forte.
- **Versions exactes** de chaque navigateur et de chaque banc ; pour les moteurs indépendants,
  la version ou le commit de la préversion essayée.
- Un banc qu'un navigateur ne termine pas est noté « ne termine pas », avec la raison ; un score
  partiel n'est jamais publié comme un score. La règle vaut pour Oxyde comme pour les autres.
- Les résultats précédents restent dans `bench/resultats/` : un chiffre ne doit pas reculer d'une
  version à l'autre sans explication.

## Résultats

Machine, date et versions : à renseigner à la première mesure. **Aucune case n'est mesurée.**

### Face aux navigateurs établis

| Banc | Oxyde | Chrome | Edge | Opera | Brave | Vivaldi | Firefox | Oxyde / meilleur |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Speedometer 3 | — | — | — | — | — | — | — | — |
| JetStream | — | — | — | — | — | — | — | — |
| MotionMark | — | — | — | — | — | — | — | — |
| ApexMark | — | — | — | — | — | — | — | — |
| WebXPRT 4 | — | — | — | — | — | — | — | — |
| Basemark Web 3.0 | — | — | — | — | — | — | — | — |

### Face aux moteurs indépendants

| Banc | Oxyde | Servo | Verso | Gosub | Ladybird | Blitz |
|---|---:|---:|---:|---:|---:|---:|
| Speedometer 3 | — | — | — | — | — | sans objet |
| JetStream | — | — | — | — | — | sans objet |
| MotionMark | — | — | — | — | — | — |
| ApexMark | — | — | — | — | — | sans objet |
| WebXPRT 4 | — | — | — | — | — | sans objet |
| Basemark Web 3.0 | — | — | — | — | — | — |

Blitz n'exécute pas JavaScript : les bancs qui en dépendent ne le concernent pas.

### Conformité, pour les mêmes moteurs

Un moteur rapide qui affiche mal ne gagne rien : la vitesse se lit à côté de la conformité.

| Suite | Oxyde | Servo | Gosub | Ladybird |
|---|---:|---:|---:|---:|
| test262 (JavaScript) | 98,6 % | — | — | — |
| Web Platform Tests | voir [conformance/](../conformance/README.md) | — | — | — |

Le chiffre d'Oxyde est mesuré sur ses dossiers `language`, `built-ins` et `annexB`. Ceux des
autres projets seront repris de leurs propres tableaux de bord publics, avec la date et le
lien, en vérifiant qu'ils portent sur le même périmètre ; sinon la case le dira.

Oxyde ne fait aujourd'hui tourner aucun des bancs de vitesse jusqu'au bout : c'est une
déduction de ce qui manque au moteur, pas le résultat d'un essai. La première tâche de
l'étape R est de lancer chaque banc dans chaque navigateur et de noter où il s'arrête.

## Cibles pour la version 1.0

| Banc | Cible |
|---|---|
| Speedometer 3 | au moins la moitié du score du meilleur navigateur établi sur la même machine |
| JetStream | au moins le tiers |
| MotionMark | au moins la moitié |
| ApexMark, WebXPRT 4, Basemark Web 3.0 | terminés sans erreur, scores publiés |

S'y ajoute une cible relative : **devant tous les moteurs indépendants écrits en Rust** sur
chaque banc que l'un d'eux termine.

Ces seuils sont des propositions. Ils sont volontairement en dessous de la parité : les moteurs
en face ont plus de quinze ans d'optimisation, et un chiffre honnête et tenu vaut mieux qu'une
promesse.
