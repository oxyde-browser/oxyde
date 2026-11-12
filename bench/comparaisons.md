# Oxyde face aux autres navigateurs

Ce document recevra les résultats d'Oxyde sur les bancs d'essai publics, à côté de ceux des
navigateurs de référence. **Rien n'y est encore mesuré** : aujourd'hui Oxyde ne fait tourner
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

## Méthode

Pour qu'une comparaison vaille quelque chose :

- **Même machine, même jour** pour tous les navigateurs ; le modèle, le processeur, la carte
  graphique, la mémoire et la version de Windows sont notés avec les résultats.
- **Sur secteur**, sans autre programme ouvert, fenêtre de même taille, profil neuf, sans
  extension.
- **Cinq passes** par banc et par navigateur ; on publie la médiane et l'écart entre la plus
  faible et la plus forte.
- **Versions exactes** de chaque navigateur et de chaque banc.
- Un banc qu'Oxyde ne termine pas est noté « ne termine pas », avec la raison ; un score partiel
  n'est jamais publié comme un score.
- Les résultats précédents restent dans `bench/resultats/` : un chiffre ne doit pas reculer d'une
  version à l'autre sans explication.

## Résultats

Machine, date et versions : à renseigner à la première mesure.

| Banc | Oxyde | Chrome | Edge | Firefox | Oxyde / meilleur |
|---|---:|---:|---:|---:|---:|
| Speedometer 3 | ne termine pas | — | — | — | — |
| JetStream | ne termine pas | — | — | — | — |
| MotionMark | ne termine pas | — | — | — | — |
| ApexMark | ne termine pas | — | — | — | — |
| WebXPRT 4 | ne termine pas | — | — | — | — |
| Basemark Web 3.0 | ne termine pas | — | — | — | — |

« ne termine pas » est ici une déduction de ce qui manque au moteur (voir la colonne de droite
du premier tableau), pas le résultat d'un essai : la première tâche de l'étape R est de lancer
chaque banc et de noter où il s'arrête.

## Cibles pour la version 1.0

| Banc | Cible |
|---|---|
| Speedometer 3 | au moins la moitié du score du meilleur navigateur sur la même machine |
| JetStream | au moins le tiers |
| MotionMark | au moins la moitié |
| ApexMark, WebXPRT 4, Basemark Web 3.0 | terminés sans erreur, scores publiés |

Ces seuils sont des propositions. Ils sont volontairement en dessous de la parité : les moteurs
en face ont plus de quinze ans d'optimisation, et un chiffre honnête et tenu vaut mieux qu'une
promesse.
