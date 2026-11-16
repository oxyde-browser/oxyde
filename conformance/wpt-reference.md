# Web Platform Tests : tests de référence

Produit par `cargo run --release -p oxyde_wpt -- --ref --publier css`, sur la suite
`web-platform-tests/wpt` déposée dans `vendor/wpt`. Chaque test est dessiné dans une
fenêtre de 800 × 600 points, puis comparé point par point à la page qu'il désigne
par `<link rel="match">` (ou `mismatch` : elles doivent alors différer).

**Tests réussis : 2448 / 7210 (34,0 %)**, dont 84 sur une page d'une seule couleur.

Une réussite dit que le test et sa référence sont dessinés de la même façon, pas que
ce dessin est juste : deux pages vides sont identiques. Les réussites « sur une page
d'une seule couleur » sont comptées à part pour cette raison ; certaines sont
légitimes (le test attend une page blanche), d'autres non.

2008 tests échouent sans avoir été comparés : leur référence n'est pas dans la copie
locale de la suite.

| Dossier | Tests réussis | dont page unie |
|---|---:|---:|
| `css/CSS2` | 2 / 13 (15,4 %) | 0 |
| `css/CSS2/box-display` | 69 / 120 (57,5 %) | 0 |
| `css/CSS2/floats` | 28 / 125 (22,4 %) | 0 |
| `css/CSS2/margin-padding-clear` | 427 / 684 (62,4 %) | 0 |
| `css/CSS2/normal-flow` | 301 / 793 (38,0 %) | 0 |
| `css/CSS2/positioning` | 203 / 520 (39,0 %) | 0 |
| `css/css-align` | 0 / 3 (0,0 %) | 0 |
| `css/css-align/abspos` | 4 / 19 (21,1 %) | 0 |
| `css/css-align/baseline-rules` | 2 / 8 (25,0 %) | 0 |
| `css/css-align/blocks` | 4 / 21 (19,0 %) | 0 |
| `css/css-align/content-distribution` | 1 / 1 (100,0 %) | 0 |
| `css/css-align/distribution-values` | 0 / 1 (0,0 %) | 0 |
| `css/css-align/gaps` | 0 / 2 (0,0 %) | 0 |
| `css/css-align/self-alignment` | 0 / 9 (0,0 %) | 0 |
| `css/css-animations` | 11 / 35 (31,4 %) | 3 |
| `css/css-animations/parsing` | 0 / 1 (0,0 %) | 0 |
| `css/css-box/margin-trim` | 0 / 42 (0,0 %) | 0 |
| `css/css-display` | 30 / 90 (33,3 %) | 0 |
| `css/css-display/run-in` | 41 / 133 (30,8 %) | 0 |
| `css/css-flexbox` | 350 / 908 (38,5 %) | 1 |
| `css/css-flexbox/abspos` | 7 / 32 (21,9 %) | 0 |
| `css/css-flexbox/alignment` | 5 / 8 (62,5 %) | 0 |
| `css/css-flexbox/balance` | 3 / 40 (7,5 %) | 0 |
| `css/css-flexbox/flex-lines` | 0 / 4 (0,0 %) | 0 |
| `css/css-flexbox/intrinsic-size` | 0 / 28 (0,0 %) | 0 |
| `css/css-flexbox/order` | 2 / 4 (50,0 %) | 0 |
| `css/css-grid` | 20 / 58 (34,5 %) | 0 |
| `css/css-grid/abspos` | 10 / 153 (6,5 %) | 0 |
| `css/css-grid/alignment` | 33 / 125 (26,4 %) | 0 |
| `css/css-grid/alignment/self-baseline` | 0 / 42 (0,0 %) | 0 |
| `css/css-grid/animation` | 0 / 2 (0,0 %) | 0 |
| `css/css-grid/grid-definition` | 8 / 42 (19,0 %) | 0 |
| `css/css-grid/grid-items` | 37 / 190 (19,5 %) | 0 |
| `css/css-grid/grid-lanes` | 7 / 10 (70,0 %) | 0 |
| `css/css-grid/grid-lanes/abspos` | 5 / 34 (14,7 %) | 0 |
| `css/css-grid/grid-lanes/alignment` | 21 / 138 (15,2 %) | 0 |
| `css/css-grid/grid-lanes/animation` | 2 / 2 (100,0 %) | 0 |
| `css/css-grid/grid-lanes/baseline` | 0 / 40 (0,0 %) | 0 |
| `css/css-grid/grid-lanes/gap` | 0 / 6 (0,0 %) | 0 |
| `css/css-grid/grid-lanes/grid-placement` | 0 / 14 (0,0 %) | 0 |
| `css/css-grid/grid-lanes/intrinsic-sizing` | 4 / 68 (5,9 %) | 1 |
| `css/css-grid/grid-lanes/invalidation` | 0 / 3 (0,0 %) | 0 |
| `css/css-grid/grid-lanes/item-placement` | 1 / 23 (4,3 %) | 1 |
| `css/css-grid/grid-lanes/item-placement/dense-packing` | 1 / 45 (2,2 %) | 0 |
| `css/css-grid/grid-lanes/item-placement/fit-tolerance` | 3 / 17 (17,6 %) | 2 |
| `css/css-grid/grid-lanes/item-placement/flow-tolerance` | 0 / 1 (0,0 %) | 0 |
| `css/css-grid/grid-lanes/items` | 2 / 15 (13,3 %) | 0 |
| `css/css-grid/grid-lanes/order` | 0 / 11 (0,0 %) | 0 |
| `css/css-grid/grid-lanes/overflow` | 9 / 22 (40,9 %) | 0 |
| `css/css-grid/grid-lanes/subgrid/grid-lanes-subgridded-to-grid-lanes/column` | 0 / 5 (0,0 %) | 0 |
| `css/css-grid/grid-lanes/subgrid/grid-lanes-subgridded-to-grid-lanes/row` | 0 / 6 (0,0 %) | 0 |
| `css/css-grid/grid-lanes/subgrid/grid-lanes-subgridded-to-grid-lanes/track-sizing` | 9 / 28 (32,1 %) | 0 |
| `css/css-grid/grid-lanes/subgrid/grid-lanes-subgridded-to-grid/column` | 0 / 3 (0,0 %) | 0 |
| `css/css-grid/grid-lanes/subgrid/grid-lanes-subgridded-to-grid/row` | 0 / 3 (0,0 %) | 0 |
| `css/css-grid/grid-lanes/subgrid/grid-lanes-subgridded-to-grid/track-sizing` | 0 / 3 (0,0 %) | 0 |
| `css/css-grid/grid-lanes/subgrid/grid-subgridded-to-grid-lanes` | 8 / 36 (22,2 %) | 0 |
| `css/css-grid/grid-lanes/subgrid/grid-subgridded-to-grid-lanes/alignment` | 1 / 6 (16,7 %) | 0 |
| `css/css-grid/grid-lanes/subgrid/grid-subgridded-to-grid-lanes/baseline` | 11 / 18 (61,1 %) | 0 |
| `css/css-grid/grid-lanes/subgrid/grid-subgridded-to-grid-lanes/column` | 3 / 11 (27,3 %) | 0 |
| `css/css-grid/grid-lanes/subgrid/grid-subgridded-to-grid-lanes/gap` | 4 / 38 (10,5 %) | 0 |
| `css/css-grid/grid-lanes/subgrid/grid-subgridded-to-grid-lanes/line-names` | 15 / 35 (42,9 %) | 7 |
| `css/css-grid/grid-lanes/subgrid/grid-subgridded-to-grid-lanes/row` | 4 / 10 (40,0 %) | 0 |
| `css/css-grid/grid-lanes/subgrid/grid-subgridded-to-grid-lanes/track-sizing` | 19 / 73 (26,0 %) | 2 |
| `css/css-grid/grid-lanes/tentative/intrinsic-sizing` | 0 / 7 (0,0 %) | 0 |
| `css/css-grid/grid-lanes/tentative/item-placement` | 0 / 38 (0,0 %) | 0 |
| `css/css-grid/grid-lanes/track-sizing` | 1 / 22 (4,5 %) | 0 |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat` | 3 / 40 (7,5 %) | 2 |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat/intrinsic-auto-repeat` | 12 / 98 (12,2 %) | 2 |
| `css/css-grid/grid-model` | 27 / 55 (49,1 %) | 0 |
| `css/css-grid/implicit-grids` | 3 / 3 (100,0 %) | 0 |
| `css/css-grid/layout-algorithm` | 9 / 42 (21,4 %) | 0 |
| `css/css-grid/placement` | 6 / 17 (35,3 %) | 0 |
| `css/css-grid/subgrid` | 18 / 102 (17,6 %) | 2 |
| `css/css-position` | 19 / 114 (16,7 %) | 2 |
| `css/css-position/multicol` | 0 / 8 (0,0 %) | 0 |
| `css/css-position/multicol/static-position` | 0 / 8 (0,0 %) | 0 |
| `css/css-position/overlay` | 3 / 9 (33,3 %) | 1 |
| `css/css-position/static-position` | 0 / 36 (0,0 %) | 0 |
| `css/css-position/sticky` | 10 / 75 (13,3 %) | 1 |
| `css/css-sizing` | 58 / 229 (25,3 %) | 5 |
| `css/css-sizing/aspect-ratio` | 8 / 269 (3,0 %) | 2 |
| `css/css-sizing/contain-intrinsic-size` | 1 / 28 (3,6 %) | 0 |
| `css/css-sizing/responsive-iframe` | 7 / 16 (43,8 %) | 0 |
| `css/css-sizing/stretch` | 0 / 29 (0,0 %) | 0 |
| `css/css-tables` | 43 / 126 (34,1 %) | 0 |
| `css/css-tables/height-distribution` | 1 / 12 (8,3 %) | 0 |
| `css/css-tables/paint` | 2 / 6 (33,3 %) | 0 |
| `css/css-tables/tentative` | 0 / 20 (0,0 %) | 0 |
| `css/css-tables/tentative/paint` | 3 / 6 (50,0 %) | 2 |
| `css/css-transforms` | 192 / 410 (46,8 %) | 35 |
| `css/css-transforms/animation` | 9 / 27 (33,3 %) | 5 |
| `css/css-transforms/document-styles` | 14 / 14 (100,0 %) | 0 |
| `css/css-transforms/external-styles` | 13 / 13 (100,0 %) | 0 |
| `css/css-transforms/gradientTransform` | 3 / 3 (100,0 %) | 0 |
| `css/css-transforms/group` | 32 / 40 (80,0 %) | 0 |
| `css/css-transforms/individual-transform` | 1 / 16 (6,3 %) | 0 |
| `css/css-transforms/individual-transform/animation` | 0 / 2 (0,0 %) | 0 |
| `css/css-transforms/inline-styles` | 14 / 14 (100,0 %) | 0 |
| `css/css-transforms/matrix` | 69 / 69 (100,0 %) | 0 |
| `css/css-transforms/patternTransform` | 0 / 2 (0,0 %) | 0 |
| `css/css-transforms/rotate` | 11 / 14 (78,6 %) | 0 |
| `css/css-transforms/scale` | 2 / 16 (12,5 %) | 0 |
| `css/css-transforms/skewX` | 1 / 7 (14,3 %) | 0 |
| `css/css-transforms/skewY` | 1 / 6 (16,7 %) | 0 |
| `css/css-transforms/transform-box` | 8 / 34 (23,5 %) | 8 |
| `css/css-transforms/transform-list-separation` | 11 / 11 (100,0 %) | 0 |
| `css/css-transforms/transform-origin` | 81 / 81 (100,0 %) | 0 |
| `css/css-transforms/translate` | 19 / 19 (100,0 %) | 0 |
| `css/css-transitions` | 1 / 5 (20,0 %) | 0 |
| `css/css-transitions/render-blocking` | 1 / 1 (100,0 %) | 0 |
| `css/cssom-view` | 4 / 11 (36,4 %) | 0 |
