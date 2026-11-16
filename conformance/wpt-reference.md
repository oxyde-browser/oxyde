# Web Platform Tests : tests de référence

Produit par `cargo run --release -p oxyde_wpt -- --ref --publier css`, sur la suite
`web-platform-tests/wpt` déposée dans `vendor/wpt`. Chaque test est dessiné dans une
fenêtre de 800 × 600 points, puis comparé point par point à la page qu'il désigne
par `<link rel="match">` (ou `mismatch` : elles doivent alors différer).

**Tests réussis : 3406 / 7210 (47,2 %)**, dont 94 sur une page d'une seule couleur.

Une réussite dit que le test et sa référence sont dessinés de la même façon, pas que
ce dessin est juste : deux pages vides sont identiques. Les réussites « sur une page
d'une seule couleur » sont comptées à part pour cette raison ; certaines sont
légitimes (le test attend une page blanche), d'autres non.

7 tests échouent sans avoir été comparés : leur référence n'est pas dans la copie
locale de la suite.

| Dossier | Tests réussis | dont page unie |
|---|---:|---:|
| `css/CSS2` | 8 / 13 (61,5 %) | 0 |
| `css/CSS2/box-display` | 81 / 120 (67,5 %) | 0 |
| `css/CSS2/floats` | 47 / 125 (37,6 %) | 0 |
| `css/CSS2/margin-padding-clear` | 557 / 684 (81,4 %) | 0 |
| `css/CSS2/normal-flow` | 466 / 793 (58,8 %) | 0 |
| `css/CSS2/positioning` | 304 / 520 (58,5 %) | 0 |
| `css/css-align` | 0 / 3 (0,0 %) | 0 |
| `css/css-align/abspos` | 6 / 19 (31,6 %) | 0 |
| `css/css-align/baseline-rules` | 2 / 8 (25,0 %) | 0 |
| `css/css-align/blocks` | 6 / 21 (28,6 %) | 0 |
| `css/css-align/content-distribution` | 1 / 1 (100,0 %) | 0 |
| `css/css-align/distribution-values` | 0 / 1 (0,0 %) | 0 |
| `css/css-align/gaps` | 2 / 2 (100,0 %) | 0 |
| `css/css-align/self-alignment` | 0 / 9 (0,0 %) | 0 |
| `css/css-animations` | 14 / 35 (40,0 %) | 3 |
| `css/css-animations/parsing` | 0 / 1 (0,0 %) | 0 |
| `css/css-box/margin-trim` | 1 / 42 (2,4 %) | 0 |
| `css/css-display` | 34 / 90 (37,8 %) | 0 |
| `css/css-display/run-in` | 53 / 133 (39,8 %) | 0 |
| `css/css-flexbox` | 473 / 908 (52,1 %) | 9 |
| `css/css-flexbox/abspos` | 18 / 32 (56,3 %) | 0 |
| `css/css-flexbox/alignment` | 5 / 8 (62,5 %) | 0 |
| `css/css-flexbox/balance` | 5 / 40 (12,5 %) | 0 |
| `css/css-flexbox/flex-lines` | 0 / 4 (0,0 %) | 0 |
| `css/css-flexbox/intrinsic-size` | 1 / 28 (3,6 %) | 0 |
| `css/css-flexbox/order` | 4 / 4 (100,0 %) | 0 |
| `css/css-grid` | 35 / 58 (60,3 %) | 0 |
| `css/css-grid/abspos` | 26 / 153 (17,0 %) | 0 |
| `css/css-grid/alignment` | 38 / 125 (30,4 %) | 0 |
| `css/css-grid/alignment/self-baseline` | 0 / 42 (0,0 %) | 0 |
| `css/css-grid/animation` | 0 / 2 (0,0 %) | 0 |
| `css/css-grid/grid-definition` | 18 / 42 (42,9 %) | 0 |
| `css/css-grid/grid-items` | 66 / 190 (34,7 %) | 0 |
| `css/css-grid/grid-lanes` | 7 / 10 (70,0 %) | 0 |
| `css/css-grid/grid-lanes/abspos` | 5 / 34 (14,7 %) | 0 |
| `css/css-grid/grid-lanes/alignment` | 22 / 138 (15,9 %) | 0 |
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
| `css/css-grid/grid-lanes/subgrid/grid-subgridded-to-grid-lanes` | 13 / 36 (36,1 %) | 0 |
| `css/css-grid/grid-lanes/subgrid/grid-subgridded-to-grid-lanes/alignment` | 1 / 6 (16,7 %) | 0 |
| `css/css-grid/grid-lanes/subgrid/grid-subgridded-to-grid-lanes/baseline` | 11 / 18 (61,1 %) | 0 |
| `css/css-grid/grid-lanes/subgrid/grid-subgridded-to-grid-lanes/column` | 3 / 11 (27,3 %) | 0 |
| `css/css-grid/grid-lanes/subgrid/grid-subgridded-to-grid-lanes/gap` | 4 / 38 (10,5 %) | 0 |
| `css/css-grid/grid-lanes/subgrid/grid-subgridded-to-grid-lanes/line-names` | 18 / 35 (51,4 %) | 7 |
| `css/css-grid/grid-lanes/subgrid/grid-subgridded-to-grid-lanes/row` | 4 / 10 (40,0 %) | 0 |
| `css/css-grid/grid-lanes/subgrid/grid-subgridded-to-grid-lanes/track-sizing` | 32 / 73 (43,8 %) | 2 |
| `css/css-grid/grid-lanes/tentative/intrinsic-sizing` | 0 / 7 (0,0 %) | 0 |
| `css/css-grid/grid-lanes/tentative/item-placement` | 0 / 38 (0,0 %) | 0 |
| `css/css-grid/grid-lanes/track-sizing` | 1 / 22 (4,5 %) | 0 |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat` | 6 / 40 (15,0 %) | 2 |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat/intrinsic-auto-repeat` | 15 / 98 (15,3 %) | 3 |
| `css/css-grid/grid-model` | 26 / 55 (47,3 %) | 0 |
| `css/css-grid/implicit-grids` | 3 / 3 (100,0 %) | 0 |
| `css/css-grid/layout-algorithm` | 21 / 42 (50,0 %) | 0 |
| `css/css-grid/placement` | 14 / 17 (82,4 %) | 0 |
| `css/css-grid/subgrid` | 31 / 102 (30,4 %) | 2 |
| `css/css-position` | 39 / 114 (34,2 %) | 2 |
| `css/css-position/multicol` | 0 / 8 (0,0 %) | 0 |
| `css/css-position/multicol/static-position` | 0 / 8 (0,0 %) | 0 |
| `css/css-position/overlay` | 4 / 9 (44,4 %) | 2 |
| `css/css-position/static-position` | 3 / 36 (8,3 %) | 0 |
| `css/css-position/sticky` | 37 / 75 (49,3 %) | 1 |
| `css/css-sizing` | 81 / 229 (35,4 %) | 5 |
| `css/css-sizing/aspect-ratio` | 105 / 269 (39,0 %) | 2 |
| `css/css-sizing/contain-intrinsic-size` | 2 / 28 (7,1 %) | 0 |
| `css/css-sizing/responsive-iframe` | 7 / 16 (43,8 %) | 0 |
| `css/css-sizing/stretch` | 0 / 29 (0,0 %) | 0 |
| `css/css-tables` | 67 / 126 (53,2 %) | 0 |
| `css/css-tables/height-distribution` | 1 / 12 (8,3 %) | 0 |
| `css/css-tables/paint` | 2 / 6 (33,3 %) | 0 |
| `css/css-tables/tentative` | 2 / 20 (10,0 %) | 0 |
| `css/css-tables/tentative/paint` | 3 / 6 (50,0 %) | 2 |
| `css/css-transforms` | 201 / 410 (49,0 %) | 35 |
| `css/css-transforms/animation` | 10 / 27 (37,0 %) | 5 |
| `css/css-transforms/document-styles` | 14 / 14 (100,0 %) | 0 |
| `css/css-transforms/external-styles` | 13 / 13 (100,0 %) | 0 |
| `css/css-transforms/gradientTransform` | 3 / 3 (100,0 %) | 0 |
| `css/css-transforms/group` | 32 / 40 (80,0 %) | 0 |
| `css/css-transforms/individual-transform` | 13 / 16 (81,3 %) | 0 |
| `css/css-transforms/individual-transform/animation` | 2 / 2 (100,0 %) | 0 |
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
| `css/css-transitions` | 2 / 5 (40,0 %) | 0 |
| `css/css-transitions/render-blocking` | 1 / 1 (100,0 %) | 0 |
| `css/cssom-view` | 6 / 11 (54,5 %) | 0 |
