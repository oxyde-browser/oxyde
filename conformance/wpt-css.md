# Web Platform Tests

Produit par `cargo run --release -p oxyde_wpt -- --publier css/css-flexbox css/css-grid css/css-position css/css-sizing css/css-tables css/css-box css/css-display css/css-align css/CSS2 css/cssom-view`, sur la suite
`web-platform-tests/wpt` déposée dans `vendor/wpt`. Seuls les tests `testharness.js`
destinés à une fenêtre sont exécutés ; un fichier est « entièrement réussi » quand le
harnais va au bout et que tous ses sous-tests passent.

**Sous-tests réussis : 8806 / 28699 (30,7 %)** — fichiers entièrement réussis : 454 / 2267 (20,0 %).

Le taux en sous-tests surestime la conformité : un fichier qui plante ou que le harnais
abandonne ne rapporte aucun sous-test, donc n'en fait échouer aucun. Le taux en fichiers
est le plus honnête des deux.

| Dossier | Fichiers réussis | Sous-tests réussis |
|---|---:|---:|
| `css/CSS2/floats` | 5 / 19 | 10 / 52 (19,2 %) |
| `css/CSS2/normal-flow` | 19 / 31 | 103 / 130 (79,2 %) |
| `css/CSS2/positioning` | 5 / 9 | 8 / 18 (44,4 %) |
| `css/css-align` | 0 / 3 | 4 / 27 (14,8 %) |
| `css/css-align/abspos` | 0 / 69 | 132 / 1142 (11,6 %) |
| `css/css-align/animation` | 0 / 6 | 0 / 0 (—) |
| `css/css-align/baseline-rules` | 1 / 4 | 9 / 20 (45,0 %) |
| `css/css-align/blocks` | 0 / 37 | 55 / 556 (9,9 %) |
| `css/css-align/content-distribution` | 2 / 17 | 145 / 573 (25,3 %) |
| `css/css-align/default-alignment` | 1 / 19 | 140 / 747 (18,7 %) |
| `css/css-align/gaps` | 1 / 20 | 121 / 162 (74,7 %) |
| `css/css-align/multicol` | 0 / 1 | 0 / 32 (0,0 %) |
| `css/css-align/parsing` | 1 / 50 | 214 / 759 (28,2 %) |
| `css/css-align/self-alignment` | 1 / 16 | 160 / 731 (21,9 %) |
| `css/css-box` | 1 / 3 | 27 / 33 (81,8 %) |
| `css/css-box/animation` | 0 / 10 | 0 / 0 (—) |
| `css/css-box/margin-trim` | 2 / 10 | 7 / 42 (16,7 %) |
| `css/css-box/margin-trim/computed-margin-values` | 0 / 1 | 0 / 4 (0,0 %) |
| `css/css-box/parsing` | 10 / 29 | 106 / 255 (41,6 %) |
| `css/css-display` | 10 / 24 | 28 / 55 (50,9 %) |
| `css/css-display/accessibility` | 0 / 1 | 0 / 0 (—) |
| `css/css-display/animations` | 0 / 2 | 0 / 0 (—) |
| `css/css-display/focus` | 0 / 1 | 0 / 5 (0,0 %) |
| `css/css-display/parsing` | 0 / 3 | 49 / 275 (17,8 %) |
| `css/css-display/parsing/tentative` | 0 / 2 | 3 / 16 (18,8 %) |
| `css/css-display/reading-flow` | 3 / 8 | 14 / 46 (30,4 %) |
| `css/css-flexbox` | 62 / 185 | 765 / 1804 (42,4 %) |
| `css/css-flexbox/abspos` | 3 / 38 | 302 / 1389 (21,7 %) |
| `css/css-flexbox/alignment` | 0 / 31 | 78 / 420 (18,6 %) |
| `css/css-flexbox/animation` | 0 / 6 | 0 / 0 (—) |
| `css/css-flexbox/balance` | 1 / 10 | 15 / 68 (22,1 %) |
| `css/css-flexbox/getcomputedstyle` | 51 / 75 | 53 / 78 (67,9 %) |
| `css/css-flexbox/intrinsic-size` | 1 / 16 | 6 / 51 (11,8 %) |
| `css/css-flexbox/parsing` | 8 / 27 | 68 / 215 (31,6 %) |
| `css/css-grid` | 6 / 11 | 60 / 187 (32,1 %) |
| `css/css-grid/abspos` | 4 / 58 | 403 / 3513 (11,5 %) |
| `css/css-grid/alignment` | 59 / 286 | 469 / 1778 (26,4 %) |
| `css/css-grid/alignment/self-baseline` | 0 / 1 | 0 / 12 (0,0 %) |
| `css/css-grid/animation` | 0 / 13 | 0 / 8 (0,0 %) |
| `css/css-grid/grid-definition` | 14 / 51 | 743 / 1318 (56,4 %) |
| `css/css-grid/grid-items` | 15 / 50 | 473 / 984 (48,1 %) |
| `css/css-grid/grid-lanes` | 0 / 2 | 2 / 35 (5,7 %) |
| `css/css-grid/grid-lanes/animation` | 0 / 3 | 0 / 0 (—) |
| `css/css-grid/grid-lanes/intrinsic-sizing` | 0 / 1 | 0 / 7 (0,0 %) |
| `css/css-grid/grid-lanes/invalidation` | 0 / 9 | 2 / 43 (4,7 %) |
| `css/css-grid/grid-lanes/items` | 0 / 19 | 0 / 685 (0,0 %) |
| `css/css-grid/grid-lanes/subgrid/grid-lanes-subgridded-to-grid-lanes/track-sizing` | 0 / 2 | 0 / 3 (0,0 %) |
| `css/css-grid/grid-lanes/subgrid/grid-subgridded-to-grid-lanes` | 1 / 3 | 1 / 3 (33,3 %) |
| `css/css-grid/grid-lanes/subgrid/grid-subgridded-to-grid-lanes/track-sizing` | 0 / 14 | 0 / 54 (0,0 %) |
| `css/css-grid/grid-lanes/tentative/intrinsic-sizing` | 0 / 2 | 0 / 2 (0,0 %) |
| `css/css-grid/grid-lanes/tentative/parsing` | 2 / 13 | 20 / 178 (11,2 %) |
| `css/css-grid/grid-lanes/tentative/parsing/grid-subgridded-to-grid-lanes` | 0 / 2 | 0 / 28 (0,0 %) |
| `css/css-grid/grid-lanes/track-sizing` | 0 / 1 | 0 / 1 (0,0 %) |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat` | 0 / 18 | 0 / 57 (0,0 %) |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat/intrinsic-auto-repeat` | 0 / 13 | 0 / 33 (0,0 %) |
| `css/css-grid/grid-model` | 7 / 22 | 108 / 188 (57,4 %) |
| `css/css-grid/implicit-grids` | 2 / 6 | 42 / 56 (75,0 %) |
| `css/css-grid/layout-algorithm` | 11 / 60 | 297 / 895 (33,2 %) |
| `css/css-grid/parsing` | 4 / 60 | 506 / 1600 (31,6 %) |
| `css/css-grid/placement` | 9 / 24 | 232 / 316 (73,4 %) |
| `css/css-grid/subgrid` | 6 / 26 | 67 / 256 (26,2 %) |
| `css/css-grid/test-plan` | 0 / 1 | 0 / 0 (—) |
| `css/css-position` | 23 / 47 | 128 / 256 (50,0 %) |
| `css/css-position/animations` | 0 / 9 | 0 / 0 (—) |
| `css/css-position/overlay` | 1 / 6 | 8 / 28 (28,6 %) |
| `css/css-position/overlay/animation` | 0 / 1 | 0 / 0 (—) |
| `css/css-position/parsing` | 9 / 22 | 88 / 166 (53,0 %) |
| `css/css-position/sticky` | 4 / 33 | 18 / 82 (22,0 %) |
| `css/css-sizing` | 11 / 53 | 172 / 705 (24,4 %) |
| `css/css-sizing/animation` | 0 / 20 | 0 / 12 (0,0 %) |
| `css/css-sizing/aspect-ratio` | 2 / 24 | 59 / 291 (20,3 %) |
| `css/css-sizing/aspect-ratio/parsing` | 0 / 3 | 7 / 38 (18,4 %) |
| `css/css-sizing/contain-intrinsic-size` | 1 / 27 | 1140 / 1338 (85,2 %) |
| `css/css-sizing/contain-intrinsic-size/animation` | 0 / 1 | 0 / 0 (—) |
| `css/css-sizing/contain-intrinsic-size/parsing` | 0 / 3 | 24 / 88 (27,3 %) |
| `css/css-sizing/parsing` | 2 / 28 | 101 / 261 (38,7 %) |
| `css/css-sizing/responsive-iframe` | 1 / 7 | 1 / 7 (14,3 %) |
| `css/css-sizing/responsive-iframe/parsing` | 1 / 2 | 5 / 10 (50,0 %) |
| `css/css-sizing/stretch` | 12 / 50 | 186 / 1015 (18,3 %) |
| `css/css-tables` | 13 / 82 | 129 / 285 (45,3 %) |
| `css/css-tables/animations` | 0 / 1 | 0 / 0 (—) |
| `css/css-tables/height-distribution` | 0 / 5 | 2 / 10 (20,0 %) |
| `css/css-tables/parsing` | 4 / 15 | 11 / 40 (27,5 %) |
| `css/css-tables/tentative` | 1 / 23 | 46 / 333 (13,8 %) |
| `css/css-tables/width-distribution` | 2 / 15 | 7 / 35 (20,0 %) |
| `css/cssom-view` | 38 / 233 | 625 / 1747 (35,8 %) |
| `css/cssom-view/parsing` | 1 / 3 | 2 / 7 (28,6 %) |

## Fichiers non entièrement réussis

| Fichier | Sous-tests | Cause |
|---|---:|---|
| `css/CSS2/floats/computed-float-position-absolute.html` | 0 / 1 |  |
| `css/CSS2/floats/float-dynamic-formatting-context.html` | 0 / 12 |  |
| `css/CSS2/floats/float-in-block-in-inline-position-001.html` | 2 / 6 |  |
| `css/CSS2/floats/float-no-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/CSS2/floats/floats-in-inline-content-intrinsic-size-001.html` | 0 / 16 |  |
| `css/CSS2/floats/hit-test-floats-001.html` | 0 / 1 |  |
| `css/CSS2/floats/hit-test-floats-002.html` | 0 / 1 |  |
| `css/CSS2/floats/hit-test-floats-003.html` | 0 / 1 |  |
| `css/CSS2/floats/hit-test-floats-004.html` | 0 / 1 |  |
| `css/CSS2/floats/hit-test-floats-005.html` | 0 / 1 |  |
| `css/CSS2/floats/zero-space-between-floats-001.html` | 0 / 1 |  |
| `css/CSS2/floats/zero-space-between-floats-002.html` | 0 / 1 |  |
| `css/CSS2/floats/zero-space-between-floats-003.html` | 0 / 1 |  |
| `css/CSS2/floats/zero-space-between-floats-004.html` | 0 / 1 |  |
| `css/CSS2/normal-flow/auto-margins-used-values-with-floats.tentative.html` | 0 / 6 |  |
| `css/CSS2/normal-flow/auto-margins-used-values.html` | 0 / 6 |  |
| `css/CSS2/normal-flow/block-in-inline-client-rects-001.html` | 2 / 3 |  |
| `css/CSS2/normal-flow/block-in-inline-hittest-001.html` | 0 / 1 |  |
| `css/CSS2/normal-flow/block-in-inline-hittest-002.html` | 0 / 2 |  |
| `css/CSS2/normal-flow/block-in-inline-hittest-float-001.html` | 0 / 1 |  |
| `css/CSS2/normal-flow/block-in-inline-hittest-float-002.html` | 0 / 5 |  |
| `css/CSS2/normal-flow/block-in-inline-hittest-margin.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/CSS2/normal-flow/block-in-inline-hittest-relpos-zindex.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/CSS2/normal-flow/block-in-inline-self-collapsing-margin-001.html` | 4 / 6 |  |
| `css/CSS2/normal-flow/hit-test-anonymous-block.html` | 0 / 1 |  |
| `css/CSS2/normal-flow/video-controls-hit-test-order.html` | 0 / 2 |  |
| `css/CSS2/positioning/inline-static-position-001.html` | 2 / 6 |  |
| `css/CSS2/positioning/line-break-after-leading-float.html` | 0 / 1 |  |
| `css/CSS2/positioning/relpos-percentage-offset-anonymous-block.html` | 1 / 5 |  |
| `css/CSS2/positioning/relpos-percentage-top-in-scrollable.html` | 0 / 1 |  |
| `css/css-align/abspos/align-self-default-overflow-htb-ltr-htb.html` | 2 / 14 |  |
| `css/css-align/abspos/align-self-default-overflow-htb-ltr-vrl.html` | 2 / 14 |  |
| `css/css-align/abspos/align-self-default-overflow-htb-rtl-htb.html` | 2 / 14 |  |
| `css/css-align/abspos/align-self-default-overflow-htb-rtl-vrl.html` | 2 / 14 |  |
| `css/css-align/abspos/align-self-default-overflow-vrl-ltr-htb.html` | 0 / 14 |  |
| `css/css-align/abspos/align-self-default-overflow-vrl-ltr-vrl.html` | 0 / 14 |  |
| `css/css-align/abspos/align-self-default-overflow-vrl-rtl-htb.html` | 0 / 14 |  |
| `css/css-align/abspos/align-self-default-overflow-vrl-rtl-vrl.html` | 0 / 14 |  |
| `css/css-align/abspos/align-self-htb-ltr-htb.html` | 2 / 16 |  |
| `css/css-align/abspos/align-self-htb-ltr-vlr.html` | 2 / 16 |  |
| `css/css-align/abspos/align-self-htb-ltr-vrl.html` | 2 / 16 |  |
| `css/css-align/abspos/align-self-htb-rtl-htb.html` | 2 / 16 |  |
| `css/css-align/abspos/align-self-htb-rtl-vlr.html` | 2 / 16 |  |
| `css/css-align/abspos/align-self-htb-rtl-vrl.html` | 2 / 16 |  |
| `css/css-align/abspos/align-self-vlr-ltr-htb.html` | 2 / 16 |  |
| `css/css-align/abspos/align-self-vlr-ltr-vlr.html` | 2 / 16 |  |
| `css/css-align/abspos/align-self-vlr-ltr-vrl.html` | 2 / 16 |  |
| `css/css-align/abspos/align-self-vlr-rtl-htb.html` | 2 / 16 |  |
| `css/css-align/abspos/align-self-vlr-rtl-vlr.html` | 2 / 16 |  |
| `css/css-align/abspos/align-self-vlr-rtl-vrl.html` | 2 / 16 |  |
| `css/css-align/abspos/align-self-vrl-ltr-htb.html` | 2 / 16 |  |
| `css/css-align/abspos/align-self-vrl-ltr-vlr.html` | 2 / 16 |  |
| `css/css-align/abspos/align-self-vrl-ltr-vrl.html` | 2 / 16 |  |
| `css/css-align/abspos/align-self-vrl-rtl-htb.html` | 2 / 16 |  |
| `css/css-align/abspos/align-self-vrl-rtl-vlr.html` | 2 / 16 |  |
| `css/css-align/abspos/align-self-vrl-rtl-vrl.html` | 2 / 16 |  |
| `css/css-align/abspos/default-overflow-with-scroll.html` | 2 / 6 |  |
| `css/css-align/abspos/justify-self-default-overflow-htb-ltr-htb.html` | 2 / 14 |  |
| `css/css-align/abspos/justify-self-default-overflow-htb-ltr-vrl.html` | 2 / 14 |  |
| `css/css-align/abspos/justify-self-default-overflow-htb-rtl-htb.html` | 0 / 14 |  |
| `css/css-align/abspos/justify-self-default-overflow-htb-rtl-vrl.html` | 0 / 14 |  |
| `css/css-align/abspos/justify-self-default-overflow-vrl-ltr-htb.html` | 2 / 14 |  |
| `css/css-align/abspos/justify-self-default-overflow-vrl-ltr-vrl.html` | 2 / 14 |  |
| `css/css-align/abspos/justify-self-default-overflow-vrl-rtl-htb.html` | 0 / 14 |  |
| `css/css-align/abspos/justify-self-default-overflow-vrl-rtl-vrl.html` | 0 / 14 |  |
| `css/css-align/abspos/justify-self-htb-ltr-htb.html` | 2 / 20 |  |
| `css/css-align/abspos/justify-self-htb-ltr-vlr.html` | 2 / 20 |  |
| `css/css-align/abspos/justify-self-htb-ltr-vrl.html` | 2 / 20 |  |
| `css/css-align/abspos/justify-self-htb-rtl-htb.html` | 2 / 20 |  |
| `css/css-align/abspos/justify-self-htb-rtl-vlr.html` | 2 / 20 |  |
| `css/css-align/abspos/justify-self-htb-rtl-vrl.html` | 2 / 20 |  |
| `css/css-align/abspos/justify-self-static-position-htb-ltr-htb.html` | 0 / 14 |  |
| `css/css-align/abspos/justify-self-static-position-htb-rtl-htb.html` | 0 / 14 |  |
| `css/css-align/abspos/justify-self-static-position-vlr-ltr-vlr.html` | 0 / 14 |  |
| `css/css-align/abspos/justify-self-static-position-vrl-ltr-vrl.html` | 0 / 14 |  |
| `css/css-align/abspos/justify-self-vlr-ltr-htb.html` | 2 / 20 |  |
| `css/css-align/abspos/justify-self-vlr-ltr-vlr.html` | 2 / 20 |  |
| `css/css-align/abspos/justify-self-vlr-ltr-vrl.html` | 2 / 20 |  |
| `css/css-align/abspos/justify-self-vlr-rtl-htb.html` | 2 / 20 |  |
| `css/css-align/abspos/justify-self-vlr-rtl-vlr.html` | 2 / 20 |  |
| `css/css-align/abspos/justify-self-vlr-rtl-vrl.html` | 2 / 20 |  |
| `css/css-align/abspos/justify-self-vrl-ltr-htb.html` | 2 / 20 |  |
| `css/css-align/abspos/justify-self-vrl-ltr-vlr.html` | 2 / 20 |  |
| `css/css-align/abspos/justify-self-vrl-ltr-vrl.html` | 2 / 20 |  |
| `css/css-align/abspos/justify-self-vrl-rtl-htb.html` | 2 / 20 |  |
| `css/css-align/abspos/justify-self-vrl-rtl-vlr.html` | 2 / 20 |  |
| `css/css-align/abspos/justify-self-vrl-rtl-vrl.html` | 2 / 20 |  |
| `css/css-align/abspos/safe-align-self-htb.html` | 0 / 24 |  |
| `css/css-align/abspos/safe-align-self-vlr.html` | 0 / 24 |  |
| `css/css-align/abspos/safe-align-self-vrl.html` | 12 / 24 |  |
| `css/css-align/abspos/safe-justify-self-htb.html` | 6 / 24 |  |
| `css/css-align/abspos/safe-justify-self-vlr.html` | 6 / 24 |  |
| `css/css-align/abspos/safe-justify-self-vrl.html` | 6 / 24 |  |
| `css/css-align/abspos/stretch-intrinsic-size-htb-htb.html` | 3 / 12 |  |
| `css/css-align/abspos/stretch-intrinsic-size-htb-vrl.html` | 3 / 12 |  |
| `css/css-align/abspos/stretch-intrinsic-size-vrl-htb.html` | 3 / 12 |  |
| `css/css-align/abspos/stretch-intrinsic-size-vrl-vrl.html` | 3 / 12 |  |
| `css/css-align/abspos/table-align-self-stretch.html` | 0 / 8 |  |
| `css/css-align/abspos/table-justify-self-stretch.html` | 0 / 8 |  |
| `css/css-align/animation/align-no-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-align/animation/column-gap-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-align/animation/column-gap-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-align/animation/justify-no-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-align/animation/row-gap-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-align/animation/row-gap-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-align/baseline-of-single-axis-scroll-container.html` | 0 / 2 |  |
| `css/css-align/baseline-rules/first-baseline-block-in-inline-001.html` | 0 / 3 |  |
| `css/css-align/baseline-rules/synthesized-baseline-flexbox-001.html` | 3 / 7 |  |
| `css/css-align/baseline-rules/synthesized-baseline-grid-001.html` | 3 / 7 |  |
| `css/css-align/blocks/align-content-block-002.html` | 0 / 17 |  |
| `css/css-align/blocks/align-content-block-003.html` | 0 / 17 |  |
| `css/css-align/blocks/align-content-block-004.html` | 0 / 17 |  |
| `css/css-align/blocks/align-content-block-005.html` | 0 / 17 |  |
| `css/css-align/blocks/align-content-block-006.html` | 0 / 17 |  |
| `css/css-align/blocks/align-content-block-007.html` | 0 / 17 |  |
| `css/css-align/blocks/align-content-block-008.html` | 0 / 17 |  |
| `css/css-align/blocks/align-content-block-009.html` | 0 / 17 |  |
| `css/css-align/blocks/align-content-block-010.html` | 0 / 17 |  |
| `css/css-align/blocks/align-content-block-011.html` | 0 / 17 |  |
| `css/css-align/blocks/align-content-block-display-coverage.html` | 1 / 8 |  |
| `css/css-align/blocks/align-content-block-float-001.html` | 0 / 4 |  |
| `css/css-align/blocks/align-content-block-simple-height-change.html` | 0 / 1 |  |
| `css/css-align/blocks/align-content-table-cell.html` | 4 / 8 |  |
| `css/css-align/blocks/justify-self-block-in-inline.html` | 1 / 2 |  |
| `css/css-align/blocks/justify-self-htb-ltr-htb.html` | 2 / 16 |  |
| `css/css-align/blocks/justify-self-htb-ltr-vlr.html` | 2 / 16 |  |
| `css/css-align/blocks/justify-self-htb-ltr-vrl.html` | 2 / 16 |  |
| `css/css-align/blocks/justify-self-htb-rtl-htb.html` | 2 / 16 |  |
| `css/css-align/blocks/justify-self-htb-rtl-vlr.html` | 2 / 16 |  |
| `css/css-align/blocks/justify-self-htb-rtl-vrl.html` | 2 / 16 |  |
| `css/css-align/blocks/justify-self-text-align-2.html` | 1 / 3 |  |
| `css/css-align/blocks/justify-self-vlr-ltr-htb.html` | 0 / 16 |  |
| `css/css-align/blocks/justify-self-vlr-ltr-vlr.html` | 0 / 16 |  |
| `css/css-align/blocks/justify-self-vlr-ltr-vrl.html` | 0 / 16 |  |
| `css/css-align/blocks/justify-self-vlr-rtl-htb.html` | 0 / 16 |  |
| `css/css-align/blocks/justify-self-vlr-rtl-vlr.html` | 0 / 16 |  |
| `css/css-align/blocks/justify-self-vlr-rtl-vrl.html` | 0 / 16 |  |
| `css/css-align/blocks/justify-self-vrl-ltr-htb.html` | 0 / 16 |  |
| `css/css-align/blocks/justify-self-vrl-ltr-vlr.html` | 0 / 16 |  |
| `css/css-align/blocks/justify-self-vrl-ltr-vrl.html` | 0 / 16 |  |
| `css/css-align/blocks/justify-self-vrl-rtl-htb.html` | 0 / 16 |  |
| `css/css-align/blocks/justify-self-vrl-rtl-vlr.html` | 0 / 16 |  |
| `css/css-align/blocks/justify-self-vrl-rtl-vrl.html` | 0 / 16 |  |
| `css/css-align/blocks/safe-justify-self-htb.html` | 12 / 24 |  |
| `css/css-align/blocks/safe-justify-self-vlr.html` | 12 / 24 |  |
| `css/css-align/blocks/safe-justify-self-vrl.html` | 12 / 24 |  |
| `css/css-align/content-distribution/parse-align-content-001.html` | 0 / 18 |  |
| `css/css-align/content-distribution/parse-align-content-002.html` | 0 / 7 |  |
| `css/css-align/content-distribution/parse-align-content-003.html` | 0 / 18 |  |
| `css/css-align/content-distribution/parse-align-content-004.html` | 0 / 40 |  |
| `css/css-align/content-distribution/parse-align-content-005.html` | 0 / 3 |  |
| `css/css-align/content-distribution/parse-justify-content-001.html` | 0 / 17 |  |
| `css/css-align/content-distribution/parse-justify-content-002.html` | 0 / 7 |  |
| `css/css-align/content-distribution/parse-justify-content-003.html` | 0 / 17 |  |
| `css/css-align/content-distribution/parse-justify-content-004.html` | 0 / 41 |  |
| `css/css-align/content-distribution/parse-justify-content-005.html` | 0 / 3 |  |
| `css/css-align/content-distribution/place-content-shorthand-001.html` | 0 / 15 |  |
| `css/css-align/content-distribution/place-content-shorthand-002.html` | 0 / 216 |  |
| `css/css-align/content-distribution/place-content-shorthand-003.html` | 0 / 2 |  |
| `css/css-align/content-distribution/place-content-shorthand-005.html` | 0 / 2 |  |
| `css/css-align/content-distribution/place-content-shorthand-006.html` | 134 / 156 |  |
| `css/css-align/default-alignment/justify-items-legacy-001.html` | 0 / 1 |  |
| `css/css-align/default-alignment/parse-align-items-001.html` | 0 / 17 |  |
| `css/css-align/default-alignment/parse-align-items-002.html` | 0 / 7 |  |
| `css/css-align/default-alignment/parse-align-items-003.html` | 0 / 17 |  |
| `css/css-align/default-alignment/parse-align-items-004.html` | 0 / 29 |  |
| `css/css-align/default-alignment/parse-align-items-005.html` | 0 / 3 |  |
| `css/css-align/default-alignment/parse-justify-items-001.html` | 0 / 22 |  |
| `css/css-align/default-alignment/parse-justify-items-002.html` | 0 / 7 |  |
| `css/css-align/default-alignment/parse-justify-items-003.html` | 0 / 22 |  |
| `css/css-align/default-alignment/parse-justify-items-004.html` | 0 / 32 |  |
| `css/css-align/default-alignment/parse-justify-items-005.html` | 0 / 3 |  |
| `css/css-align/default-alignment/parse-justify-items-006.html` | 0 / 3 |  |
| `css/css-align/default-alignment/place-items-shorthand-001.html` | 0 / 17 |  |
| `css/css-align/default-alignment/place-items-shorthand-002.html` | 0 / 374 |  |
| `css/css-align/default-alignment/place-items-shorthand-003.html` | 0 / 2 |  |
| `css/css-align/default-alignment/place-items-shorthand-005.html` | 0 / 2 |  |
| `css/css-align/default-alignment/place-items-shorthand-006.html` | 132 / 168 |  |
| `css/css-align/default-alignment/shorthand-serialization-001.html` | 4 / 17 |  |
| `css/css-align/flow-start-and-end-serialization.html` | 0 / 9 |  |
| `css/css-align/gaps/column-gap-animation-001.html` | 0 / 1 |  |
| `css/css-align/gaps/column-gap-animation-002.html` | 0 / 1 |  |
| `css/css-align/gaps/column-gap-animation-003.html` | 0 / 1 |  |
| `css/css-align/gaps/column-gap-parsing-001.html` | 16 / 18 |  |
| `css/css-align/gaps/gap-animation-001.html` | 0 / 1 |  |
| `css/css-align/gaps/gap-animation-002.html` | 0 / 1 |  |
| `css/css-align/gaps/gap-animation-003.html` | 0 / 1 |  |
| `css/css-align/gaps/gap-animation-004.html` | 0 / 1 |  |
| `css/css-align/gaps/gap-parsing-001.html` | 22 / 26 |  |
| `css/css-align/gaps/gap-parsing-002.html` | 6 / 16 |  |
| `css/css-align/gaps/gap-serialization-cssom.html` | 1 / 3 |  |
| `css/css-align/gaps/grid-column-gap-parsing-001.html` | 16 / 18 |  |
| `css/css-align/gaps/grid-gap-parsing-001.html` | 22 / 26 |  |
| `css/css-align/gaps/grid-row-gap-parsing-001.html` | 16 / 18 |  |
| `css/css-align/gaps/legacy-gap-aliases-001.html` | 0 / 3 |  |
| `css/css-align/gaps/row-gap-animation-001.html` | 0 / 1 |  |
| `css/css-align/gaps/row-gap-animation-002.html` | 0 / 1 |  |
| `css/css-align/gaps/row-gap-animation-003.html` | 0 / 1 |  |
| `css/css-align/gaps/row-gap-parsing-001.html` | 16 / 18 |  |
| `css/css-align/inheritance.html` | 4 / 16 |  |
| `css/css-align/multicol/align-content-multicol.html` | 0 / 32 |  |
| `css/css-align/parsing/align-content-computed.html` | 0 / 18 |  |
| `css/css-align/parsing/align-content-invalid.html` | 0 / 23 |  |
| `css/css-align/parsing/align-content-valid.html` | 18 / 19 |  |
| `css/css-align/parsing/align-items-computed.html` | 0 / 19 |  |
| `css/css-align/parsing/align-items-invalid.html` | 0 / 26 |  |
| `css/css-align/parsing/align-items-valid.html` | 19 / 20 |  |
| `css/css-align/parsing/align-self-computed.html` | 0 / 20 |  |
| `css/css-align/parsing/align-self-invalid.html` | 0 / 24 |  |
| `css/css-align/parsing/align-self-valid.html` | 20 / 21 |  |
| `css/css-align/parsing/column-gap-computed.html` | 2 / 5 |  |
| `css/css-align/parsing/column-gap-invalid.html` | 0 / 5 |  |
| `css/css-align/parsing/column-gap-valid.html` | 5 / 6 |  |
| `css/css-align/parsing/gap-computed.html` | 0 / 11 |  |
| `css/css-align/parsing/gap-invalid.html` | 0 / 4 |  |
| `css/css-align/parsing/gap-shorthand.html` | 4 / 12 |  |
| `css/css-align/parsing/gap-valid.html` | 5 / 9 |  |
| `css/css-align/parsing/grid-column-gap-computed.html` | 0 / 5 |  |
| `css/css-align/parsing/grid-column-gap-invalid.html` | 0 / 5 |  |
| `css/css-align/parsing/grid-column-gap-valid.html` | 5 / 6 |  |
| `css/css-align/parsing/grid-gap-computed.html` | 0 / 11 |  |
| `css/css-align/parsing/grid-gap-invalid.html` | 0 / 4 |  |
| `css/css-align/parsing/grid-gap-shorthand.html` | 4 / 12 |  |
| `css/css-align/parsing/grid-gap-valid.html` | 5 / 9 |  |
| `css/css-align/parsing/grid-row-gap-computed.html` | 0 / 6 |  |
| `css/css-align/parsing/grid-row-gap-invalid.html` | 0 / 4 |  |
| `css/css-align/parsing/grid-row-gap-valid.html` | 4 / 5 |  |
| `css/css-align/parsing/justify-content-computed.html` | 0 / 18 |  |
| `css/css-align/parsing/justify-content-invalid.html` | 0 / 24 |  |
| `css/css-align/parsing/justify-items-computed.html` | 0 / 26 |  |
| `css/css-align/parsing/justify-items-invalid.html` | 0 / 21 |  |
| `css/css-align/parsing/justify-items-valid.html` | 22 / 26 |  |
| `css/css-align/parsing/justify-self-computed.html` | 0 / 22 |  |
| `css/css-align/parsing/justify-self-invalid.html` | 0 / 22 |  |
| `css/css-align/parsing/justify-self-valid.html` | 22 / 23 |  |
| `css/css-align/parsing/place-content-computed.html` | 0 / 25 |  |
| `css/css-align/parsing/place-content-invalid.html` | 0 / 10 |  |
| `css/css-align/parsing/place-content-shorthand.html` | 2 / 15 |  |
| `css/css-align/parsing/place-content-valid.html` | 18 / 26 |  |
| `css/css-align/parsing/place-items-computed.html` | 0 / 25 |  |
| `css/css-align/parsing/place-items-invalid.html` | 0 / 17 |  |
| `css/css-align/parsing/place-items-shorthand.html` | 1 / 18 |  |
| `css/css-align/parsing/place-items-valid.html` | 16 / 24 |  |
| `css/css-align/parsing/place-self-computed.html` | 0 / 23 |  |
| `css/css-align/parsing/place-self-invalid.html` | 0 / 11 |  |
| `css/css-align/parsing/place-self-shorthand.html` | 1 / 18 |  |
| `css/css-align/parsing/place-self-valid.html` | 16 / 23 |  |
| `css/css-align/parsing/row-gap-computed.html` | 3 / 6 |  |
| `css/css-align/parsing/row-gap-invalid.html` | 0 / 4 |  |
| `css/css-align/parsing/row-gap-valid.html` | 4 / 5 |  |
| `css/css-align/self-alignment/parse-align-self-001.html` | 0 / 18 |  |
| `css/css-align/self-alignment/parse-align-self-002.html` | 0 / 7 |  |
| `css/css-align/self-alignment/parse-align-self-003.html` | 0 / 18 |  |
| `css/css-align/self-alignment/parse-align-self-004.html` | 0 / 28 |  |
| `css/css-align/self-alignment/parse-align-self-005.html` | 0 / 3 |  |
| `css/css-align/self-alignment/parse-justify-self-001.html` | 0 / 20 |  |
| `css/css-align/self-alignment/parse-justify-self-002.html` | 0 / 7 |  |
| `css/css-align/self-alignment/parse-justify-self-003.html` | 0 / 20 |  |
| `css/css-align/self-alignment/parse-justify-self-004.html` | 0 / 26 |  |
| `css/css-align/self-alignment/parse-justify-self-005.html` | 0 / 3 |  |
| `css/css-align/self-alignment/place-self-shorthand-001.html` | 0 / 18 |  |
| `css/css-align/self-alignment/place-self-shorthand-002.html` | 0 / 360 |  |
| `css/css-align/self-alignment/place-self-shorthand-003.html` | 0 / 2 |  |
| `css/css-align/self-alignment/place-self-shorthand-005.html` | 0 / 2 |  |
| `css/css-align/self-alignment/place-self-shorthand-006.html` | 156 / 195 |  |
| `css/css-box/animation/margin-bottom-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-box/animation/margin-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-box/animation/margin-left-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-box/animation/margin-right-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-box/animation/margin-top-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-box/animation/padding-bottom-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-box/animation/padding-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-box/animation/padding-left-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-box/animation/padding-right-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-box/animation/padding-top-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-box/inheritance.html` | 22 / 24 |  |
| `css/css-box/margin-trim/block-container-block-end-last-child-with-border.html` | 0 / 2 |  |
| `css/css-box/margin-trim/block-container-block-end-nested-last-child-with-border.html` | 0 / 3 |  |
| `css/css-box/margin-trim/block-container-block-end-propagation-boundaries.html` | 0 / 10 |  |
| `css/css-box/margin-trim/block-container-block-end-self-collapsing-children-nested-at-bottom.html` | 0 / 7 |  |
| `css/css-box/margin-trim/block-container-block-end-self-collapsing-children-nested-margin-trim.html` | 0 / 1 |  |
| `css/css-box/margin-trim/block-container-block-end-self-collapsing-children-offsets-nested-multiple-times.html` | 0 / 1 |  |
| `css/css-box/margin-trim/block-container-block-end-self-collapsing-children-offsets-vert-lr.html` | 1 / 3 |  |
| `css/css-box/margin-trim/computed-margin-values/block-container-style-change-triggers-layout-block.html` | 0 / 4 |  |
| `css/css-box/margin-trim/margin-trim-computed-value.html` | 0 / 9 |  |
| `css/css-box/negative-margin.html` | 4 / 8 |  |
| `css/css-box/parsing/clear-computed.html` | 4 / 6 |  |
| `css/css-box/parsing/clear-invalid.html` | 0 / 2 |  |
| `css/css-box/parsing/float-computed.html` | 3 / 5 |  |
| `css/css-box/parsing/float-invalid.html` | 0 / 3 |  |
| `css/css-box/parsing/height-invalid.html` | 0 / 10 |  |
| `css/css-box/parsing/margin-computed.html` | 4 / 8 |  |
| `css/css-box/parsing/margin-invalid.html` | 0 / 7 |  |
| `css/css-box/parsing/margin-shorthand.html` | 4 / 20 |  |
| `css/css-box/parsing/margin-trim-computed.html` | 0 / 6 |  |
| `css/css-box/parsing/margin-trim.html` | 4 / 21 |  |
| `css/css-box/parsing/max-height-invalid.html` | 0 / 12 |  |
| `css/css-box/parsing/max-width-invalid.html` | 0 / 12 |  |
| `css/css-box/parsing/overflow-invalid.html` | 0 / 6 |  |
| `css/css-box/parsing/padding-computed.html` | 5 / 13 |  |
| `css/css-box/parsing/padding-invalid.html` | 0 / 10 |  |
| `css/css-box/parsing/padding-shorthand.html` | 4 / 20 |  |
| `css/css-box/parsing/padding-valid.html` | 10 / 11 |  |
| `css/css-box/parsing/visibility-invalid.html` | 0 / 2 |  |
| `css/css-box/parsing/width-invalid.html` | 0 / 13 |  |
| `css/css-display/accessibility/display-contents-role-and-label.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-display/animations/display-interpolation-none.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-display/animations/display-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-display/display-contents-blockify-dynamic.html` | 0 / 3 |  |
| `css/css-display/display-contents-computed-style.html` | 3 / 5 |  |
| `css/css-display/display-contents-focusable-001.html` | 0 / 1 |  |
| `css/css-display/display-contents-pseudo-click-target.html` | 0 / 1 |  |
| `css/css-display/display-contents-svg-tspan-child-character-positions.html` | 0 / 1 |  |
| `css/css-display/display-contents-svg-tspan-child-display-changed.html` | 0 / 1 |  |
| `css/css-display/display-contents-svg-tspan-child-removed.html` | 0 / 1 |  |
| `css/css-display/display-contents-svg-tspan-child-text-inserted.html` | 0 / 1 |  |
| `css/css-display/display-contents-text-line-height-quirks.html` | 2 / 3 |  |
| `css/css-display/display-first-letter-001.html` | 0 / 1 |  |
| `css/css-display/display-first-line-001.html` | 0 / 1 |  |
| `css/css-display/display-initial.html` | 4 / 6 |  |
| `css/css-display/display-math-on-non-mathml-elements.html` | 2 / 10 |  |
| `css/css-display/display-math-on-pseudo-elements-001.html` | 3 / 6 |  |
| `css/css-display/focus/display-contents-focus.html` | 0 / 5 |  |
| `css/css-display/parsing/display-computed.html` | 13 / 112 |  |
| `css/css-display/parsing/display-invalid.html` | 0 / 55 |  |
| `css/css-display/parsing/display-valid.html` | 36 / 108 |  |
| `css/css-display/parsing/tentative/display-computed.html` | 0 / 10 |  |
| `css/css-display/parsing/tentative/display-valid.html` | 3 / 6 |  |
| `css/css-display/reading-flow/reading-flow-computed.html` | 0 / 7 |  |
| `css/css-display/reading-flow/reading-flow-grid-lanes-computed.tentative.html` | 0 / 3 |  |
| `css/css-display/reading-flow/reading-flow-invalid.html` | 0 / 9 |  |
| `css/css-display/reading-flow/reading-order-computed.html` | 0 / 4 |  |
| `css/css-display/reading-flow/reading-order-invalid.html` | 0 / 9 |  |
| `css/css-flexbox/abspos/flex-abspos-staticpos-align-content-001.html` | 0 / 24 |  |
| `css/css-flexbox/abspos/flex-abspos-staticpos-align-self-001.html` | 12 / 24 |  |
| `css/css-flexbox/abspos/flex-abspos-staticpos-align-self-002.html` | 8 / 24 |  |
| `css/css-flexbox/abspos/flex-abspos-staticpos-align-self-003.html` | 0 / 24 |  |
| `css/css-flexbox/abspos/flex-abspos-staticpos-align-self-004.html` | 0 / 24 |  |
| `css/css-flexbox/abspos/flex-abspos-staticpos-align-self-005.html` | 12 / 24 |  |
| `css/css-flexbox/abspos/flex-abspos-staticpos-align-self-006.html` | 8 / 24 |  |
| `css/css-flexbox/abspos/flex-abspos-staticpos-align-self-007.html` | 0 / 24 |  |
| `css/css-flexbox/abspos/flex-abspos-staticpos-align-self-008.html` | 0 / 24 |  |
| `css/css-flexbox/abspos/flex-abspos-staticpos-align-self-rtl-001.html` | 0 / 24 |  |
| `css/css-flexbox/abspos/flex-abspos-staticpos-align-self-rtl-002.html` | 0 / 24 |  |
| `css/css-flexbox/abspos/flex-abspos-staticpos-align-self-rtl-003.html` | 8 / 24 |  |
| `css/css-flexbox/abspos/flex-abspos-staticpos-align-self-rtl-004.html` | 8 / 24 |  |
| `css/css-flexbox/abspos/flex-abspos-staticpos-align-self-vertWM-001.html` | 8 / 28 |  |
| `css/css-flexbox/abspos/flex-abspos-staticpos-align-self-vertWM-002.html` | 8 / 28 |  |
| `css/css-flexbox/abspos/flex-abspos-staticpos-align-self-vertWM-003.html` | 0 / 24 |  |
| `css/css-flexbox/abspos/flex-abspos-staticpos-align-self-vertWM-004.html` | 0 / 24 |  |
| `css/css-flexbox/abspos/flex-abspos-staticpos-justify-content-001.html` | 12 / 24 |  |
| `css/css-flexbox/abspos/flex-abspos-staticpos-justify-content-002.html` | 0 / 24 |  |
| `css/css-flexbox/abspos/flex-abspos-staticpos-justify-content-003.html` | 6 / 24 |  |
| `css/css-flexbox/abspos/flex-abspos-staticpos-justify-content-004.html` | 0 / 24 |  |
| `css/css-flexbox/abspos/flex-abspos-staticpos-justify-content-005.html` | 14 / 24 |  |
| `css/css-flexbox/abspos/flex-abspos-staticpos-justify-content-006.html` | 0 / 24 |  |
| `css/css-flexbox/abspos/flex-abspos-staticpos-justify-content-007.html` | 8 / 24 |  |
| `css/css-flexbox/abspos/flex-abspos-staticpos-justify-content-008.html` | 0 / 24 |  |
| `css/css-flexbox/abspos/flex-abspos-staticpos-justify-content-rtl-001.html` | 6 / 24 |  |
| `css/css-flexbox/abspos/flex-abspos-staticpos-justify-content-rtl-002.html` | 0 / 24 |  |
| `css/css-flexbox/abspos/flex-abspos-staticpos-justify-content-vertWM-001.html` | 0 / 24 |  |
| `css/css-flexbox/abspos/flex-abspos-staticpos-justify-content-vertWM-002.html` | 6 / 24 |  |
| `css/css-flexbox/abspos/position-absolute-001.html` | 21 / 120 |  |
| `css/css-flexbox/abspos/position-absolute-002.html` | 12 / 17 |  |
| `css/css-flexbox/abspos/position-absolute-003.html` | 8 / 16 |  |
| `css/css-flexbox/abspos/position-absolute-012.html` | 24 / 96 |  |
| `css/css-flexbox/abspos/position-absolute-013.html` | 110 / 432 |  |
| `css/css-flexbox/abspos/position-absolute-015.html` | 0 / 1 |  |
| `css/css-flexbox/align-content-horiz-001a.html` | 1 / 72 |  |
| `css/css-flexbox/align-content-horiz-001b.html` | 1 / 72 |  |
| `css/css-flexbox/align-content-horiz-002.html` | 0 / 72 |  |
| `css/css-flexbox/align-content-rounding.html` | 0 / 1 |  |
| `css/css-flexbox/align-content-stretch-rounding.html` | 0 / 1 |  |
| `css/css-flexbox/align-content-vert-001b.html` | 17 / 72 |  |
| `css/css-flexbox/align-content-vert-002.html` | 52 / 72 |  |
| `css/css-flexbox/align-content-wmvert-001.html` | 15 / 72 |  |
| `css/css-flexbox/align-content-wrap-001.html` | 10 / 13 |  |
| `css/css-flexbox/align-content-wrap-002.html` | 4 / 6 |  |
| `css/css-flexbox/align-content-wrap-003.html` | 24 / 44 |  |
| `css/css-flexbox/align-content-wrap-005.html` | 0 / 8 |  |
| `css/css-flexbox/align-items-baseline-column-horz.html` | 1 / 5 |  |
| `css/css-flexbox/align-items-baseline-column-vert-lr-flexbox-item.html` | 1 / 2 |  |
| `css/css-flexbox/align-items-baseline-column-vert-lr-grid-item.html` | 1 / 2 |  |
| `css/css-flexbox/align-items-baseline-column-vert-lr-items.html` | 1 / 2 |  |
| `css/css-flexbox/align-items-baseline-column-vert-lr-table-item.html` | 1 / 2 |  |
| `css/css-flexbox/align-items-baseline-column-vert-rl-flexbox-item.html` | 0 / 2 |  |
| `css/css-flexbox/align-items-baseline-column-vert-rl-grid-item.html` | 0 / 2 |  |
| `css/css-flexbox/align-items-baseline-column-vert-rl-items.html` | 0 / 2 |  |
| `css/css-flexbox/align-items-baseline-column-vert-rl-table-item.html` | 0 / 2 |  |
| `css/css-flexbox/align-items-baseline-column-vert.html` | 1 / 3 |  |
| `css/css-flexbox/align-items-baseline-row-horz.html` | 1 / 3 |  |
| `css/css-flexbox/align-items-baseline-row-vert.html` | 0 / 5 |  |
| `css/css-flexbox/align-items-baseline-vert-lr-column-horz-flexbox-item.html` | 1 / 2 |  |
| `css/css-flexbox/align-items-baseline-vert-lr-column-horz-grid-item.html` | 1 / 2 |  |
| `css/css-flexbox/align-items-baseline-vert-lr-column-horz-items.html` | 1 / 2 |  |
| `css/css-flexbox/align-items-baseline-vert-lr-column-horz-table-item.html` | 0 / 1 |  |
| `css/css-flexbox/align-items-baseline-vert-rl-column-horz-flexbox-item.html` | 1 / 2 |  |
| `css/css-flexbox/align-items-baseline-vert-rl-column-horz-grid-item.html` | 1 / 2 |  |
| `css/css-flexbox/align-items-baseline-vert-rl-column-horz-items.html` | 1 / 2 |  |
| `css/css-flexbox/align-items-baseline-vert-rl-column-horz-table-item.html` | 0 / 1 |  |
| `css/css-flexbox/align-self-baseline-with-flex-wrap.html` | 0 / 16 |  |
| `css/css-flexbox/alignment/flex-align-baseline-001.html` | 1 / 6 |  |
| `css/css-flexbox/alignment/flex-align-baseline-002.html` | 0 / 6 |  |
| `css/css-flexbox/alignment/flex-align-baseline-003.html` | 0 / 6 |  |
| `css/css-flexbox/alignment/flex-align-baseline-004.html` | 1 / 6 |  |
| `css/css-flexbox/alignment/flex-align-baseline-005.html` | 1 / 3 |  |
| `css/css-flexbox/alignment/flex-align-baseline-006.html` | 0 / 3 |  |
| `css/css-flexbox/alignment/flex-align-baseline-007.html` | 0 / 3 |  |
| `css/css-flexbox/alignment/flex-align-baseline-fieldset-001.html` | 2 / 4 |  |
| `css/css-flexbox/alignment/flex-align-baseline-fieldset-002.html` | 0 / 4 |  |
| `css/css-flexbox/alignment/flex-align-baseline-fieldset-003.html` | 0 / 4 |  |
| `css/css-flexbox/alignment/flex-align-baseline-flex-001.html` | 24 / 48 |  |
| `css/css-flexbox/alignment/flex-align-baseline-flex-002.html` | 0 / 48 |  |
| `css/css-flexbox/alignment/flex-align-baseline-flex-003.html` | 8 / 16 |  |
| `css/css-flexbox/alignment/flex-align-baseline-flex-004.html` | 0 / 16 |  |
| `css/css-flexbox/alignment/flex-align-baseline-grid-001.html` | 6 / 12 |  |
| `css/css-flexbox/alignment/flex-align-baseline-grid-002.html` | 0 / 12 |  |
| `css/css-flexbox/alignment/flex-align-baseline-grid-003.html` | 0 / 12 |  |
| `css/css-flexbox/alignment/flex-align-baseline-line-clamp-001.tentative.html` | 13 / 26 |  |
| `css/css-flexbox/alignment/flex-align-baseline-line-clamp-002.tentative.html` | 0 / 24 |  |
| `css/css-flexbox/alignment/flex-align-baseline-line-clamp-003.tentative.html` | 0 / 24 |  |
| `css/css-flexbox/alignment/flex-align-baseline-multicol-001.html` | 6 / 12 |  |
| `css/css-flexbox/alignment/flex-align-baseline-multicol-002.html` | 0 / 12 |  |
| `css/css-flexbox/alignment/flex-align-baseline-multicol-003.html` | 0 / 12 |  |
| `css/css-flexbox/alignment/flex-align-baseline-overflow-001.html` | 6 / 12 |  |
| `css/css-flexbox/alignment/flex-align-baseline-overflow-002.html` | 3 / 12 |  |
| `css/css-flexbox/alignment/flex-align-baseline-overflow-003.html` | 3 / 12 |  |
| `css/css-flexbox/alignment/flex-align-baseline-table-001.html` | 4 / 8 |  |
| `css/css-flexbox/alignment/flex-align-baseline-table-002.html` | 0 / 4 |  |
| `css/css-flexbox/alignment/flex-align-baseline-table-003.html` | 0 / 4 |  |
| `css/css-flexbox/alignment/flex-content-alignment-with-abspos-001.html` | 0 / 1 |  |
| `css/css-flexbox/alignment/multiline-align-self.html` | 0 / 48 |  |
| `css/css-flexbox/animation/discrete-no-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-flexbox/animation/flex-basis-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-flexbox/animation/flex-basis-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-flexbox/animation/flex-grow-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-flexbox/animation/flex-shrink-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-flexbox/animation/order-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-flexbox/balance/balance-negative-margin-002.html` | 1 / 4 |  |
| `css/css-flexbox/balance/flex-flow-invalid.html` | 0 / 6 |  |
| `css/css-flexbox/balance/flex-flow-shorthand.html` | 0 / 9 |  |
| `css/css-flexbox/balance/flex-flow-valid.html` | 4 / 9 |  |
| `css/css-flexbox/balance/flex-line-count-computed.html` | 0 / 4 |  |
| `css/css-flexbox/balance/flex-line-count-invalid.html` | 0 / 5 |  |
| `css/css-flexbox/balance/flex-wrap-computed.html` | 3 / 8 |  |
| `css/css-flexbox/balance/flex-wrap-invalid.html` | 0 / 13 |  |
| `css/css-flexbox/balance/flex-wrap-valid.html` | 5 / 8 |  |
| `css/css-flexbox/canvas-dynamic-change-001.html` | 0 / 1 |  |
| `css/css-flexbox/change-column-flex-width.html` | 0 / 1 |  |
| `css/css-flexbox/checkbox-percentage-width-in-flex.html` | 0 / 4 |  |
| `css/css-flexbox/column-reverse-wrap-auto-margins-001.html` | 0 / 3 |  |
| `css/css-flexbox/flex-aspect-ratio-img-column-011.html` | 4 / 10 |  |
| `css/css-flexbox/flex-aspect-ratio-img-column-017.html` | 0 / 1 |  |
| `css/css-flexbox/flex-aspect-ratio-img-row-005.html` | 1 / 4 |  |
| `css/css-flexbox/flex-aspect-ratio-img-row-013.html` | 0 / 5 |  |
| `css/css-flexbox/flex-base-size-ignores-min-max-width-001.html` | 0 / 4 |  |
| `css/css-flexbox/flex-base-size-ignores-min-max-width-003.html` | 0 / 4 |  |
| `css/css-flexbox/flex-basis-009.html` | 0 / 1 |  |
| `css/css-flexbox/flex-basis-013.html` | 0 / 20 |  |
| `css/css-flexbox/flex-basis-014.html` | 3 / 9 |  |
| `css/css-flexbox/flex-basis-content-percentage-height.html` | 2 / 4 |  |
| `css/css-flexbox/flex-basis-intrinsics-001.html` | 5 / 12 |  |
| `css/css-flexbox/flex-basis-min-content-dynamic-001.html` | 0 / 1 |  |
| `css/css-flexbox/flex-container-max-content-002.tentative.html` | 8 / 24 |  |
| `css/css-flexbox/flex-container-min-content-002.tentative.html` | 4 / 24 |  |
| `css/css-flexbox/flex-factor-less-than-one.html` | 12 / 22 |  |
| `css/css-flexbox/flex-flow-013.html` | 3 / 12 |  |
| `css/css-flexbox/flex-item-compressible-001.html` | 0 / 15 |  |
| `css/css-flexbox/flex-item-compressible-002.html` | 0 / 15 |  |
| `css/css-flexbox/flex-item-contains-strict.html` | 4 / 8 |  |
| `css/css-flexbox/flex-item-min-max-width-intrinsic-contribution-001.html` | 0 / 2 |  |
| `css/css-flexbox/flex-item-percentage-height-img-001.html` | 0 / 2 |  |
| `css/css-flexbox/flex-minimum-height-flex-items-012.html` | 0 / 1 |  |
| `css/css-flexbox/flex-minimum-height-flex-items-025.html` | 0 / 4 |  |
| `css/css-flexbox/flex-minimum-size-002.html` | 0 / 1 |  |
| `css/css-flexbox/flex-minimum-size-003.html` | 0 / 2 |  |
| `css/css-flexbox/flex-minimum-size-single-axis-scroll-container.html` | 2 / 4 |  |
| `css/css-flexbox/flex-minimum-width-flex-items-014.html` | 1 / 6 |  |
| `css/css-flexbox/flex-one-sets-flex-basis-to-zero-px.html` | 9 / 18 |  |
| `css/css-flexbox/flex-shorthand-calc.html` | 0 / 4 |  |
| `css/css-flexbox/flex-wrap-column-grow.html` | 0 / 3 |  |
| `css/css-flexbox/flexbox-baseline-wrap-reverse-unequal-lines.html` | 0 / 1 |  |
| `css/css-flexbox/flexbox-ignores-first-letter.html` | 9 / 12 |  |
| `css/css-flexbox/flexbox-justify-content-wmvert-002.html` | 2 / 4 |  |
| `css/css-flexbox/flexbox-justify-content-wmvert-003.html` | 2 / 4 |  |
| `css/css-flexbox/flexbox-min-height-auto-003.html` | 0 / 4 |  |
| `css/css-flexbox/flexbox-min-height-auto-004.html` | 0 / 4 |  |
| `css/css-flexbox/flexbox_justifycontent-rtl-001.html` | 5 / 12 |  |
| `css/css-flexbox/flexbox_justifycontent-rtl-002.html` | 0 / 12 |  |
| `css/css-flexbox/flexitem-stretch-image.html` | 0 / 1 |  |
| `css/css-flexbox/gap-018.html` | 0 / 1 |  |
| `css/css-flexbox/getcomputedstyle/flexbox_computedstyle_align-content-center.html` | 0 / 1 |  |
| `css/css-flexbox/getcomputedstyle/flexbox_computedstyle_align-content-flex-end.html` | 0 / 1 |  |
| `css/css-flexbox/getcomputedstyle/flexbox_computedstyle_align-content-flex-start.html` | 0 / 1 |  |
| `css/css-flexbox/getcomputedstyle/flexbox_computedstyle_align-content-space-around.html` | 0 / 1 |  |
| `css/css-flexbox/getcomputedstyle/flexbox_computedstyle_align-content-space-between.html` | 0 / 1 |  |
| `css/css-flexbox/getcomputedstyle/flexbox_computedstyle_align-items-baseline.html` | 0 / 1 |  |
| `css/css-flexbox/getcomputedstyle/flexbox_computedstyle_align-items-center.html` | 0 / 1 |  |
| `css/css-flexbox/getcomputedstyle/flexbox_computedstyle_align-items-flex-end.html` | 0 / 1 |  |
| `css/css-flexbox/getcomputedstyle/flexbox_computedstyle_align-items-flex-start.html` | 0 / 1 |  |
| `css/css-flexbox/getcomputedstyle/flexbox_computedstyle_align-items-invalid.html` | 0 / 1 |  |
| `css/css-flexbox/getcomputedstyle/flexbox_computedstyle_align-items-stretch.html` | 0 / 1 |  |
| `css/css-flexbox/getcomputedstyle/flexbox_computedstyle_align-self-baseline.html` | 0 / 1 |  |
| `css/css-flexbox/getcomputedstyle/flexbox_computedstyle_align-self-center.html` | 0 / 1 |  |
| `css/css-flexbox/getcomputedstyle/flexbox_computedstyle_align-self-flex-end.html` | 0 / 1 |  |
| `css/css-flexbox/getcomputedstyle/flexbox_computedstyle_align-self-flex-start.html` | 0 / 1 |  |
| `css/css-flexbox/getcomputedstyle/flexbox_computedstyle_align-self-invalid.html` | 0 / 1 |  |
| `css/css-flexbox/getcomputedstyle/flexbox_computedstyle_align-self-stretch.html` | 0 / 1 |  |
| `css/css-flexbox/getcomputedstyle/flexbox_computedstyle_display-inline.html` | 0 / 1 |  |
| `css/css-flexbox/getcomputedstyle/flexbox_computedstyle_justify-content-center.html` | 0 / 1 |  |
| `css/css-flexbox/getcomputedstyle/flexbox_computedstyle_justify-content-flex-end.html` | 0 / 1 |  |
| `css/css-flexbox/getcomputedstyle/flexbox_computedstyle_justify-content-flex-start.html` | 0 / 1 |  |
| `css/css-flexbox/getcomputedstyle/flexbox_computedstyle_justify-content-space-around.html` | 0 / 1 |  |
| `css/css-flexbox/getcomputedstyle/flexbox_computedstyle_justify-content-space-between.html` | 0 / 1 |  |
| `css/css-flexbox/getcomputedstyle/flexbox_computedstyle_min-auto-size.html` | 2 / 4 |  |
| `css/css-flexbox/hittest-anonymous-box.html` | 0 / 1 |  |
| `css/css-flexbox/hittest-before-pseudo.html` | 0 / 1 |  |
| `css/css-flexbox/hittest-overlapping-margin.html` | 0 / 1 |  |
| `css/css-flexbox/hittest-overlapping-order.html` | 0 / 1 |  |
| `css/css-flexbox/hittest-overlapping-relative.html` | 0 / 1 |  |
| `css/css-flexbox/image-as-flexitem-size-001.html` | 14 / 18 |  |
| `css/css-flexbox/image-as-flexitem-size-001v.html` | 14 / 18 |  |
| `css/css-flexbox/image-as-flexitem-size-002.html` | 13 / 18 |  |
| `css/css-flexbox/image-as-flexitem-size-002v.html` | 13 / 18 |  |
| `css/css-flexbox/image-as-flexitem-size-003.html` | 11 / 18 |  |
| `css/css-flexbox/image-as-flexitem-size-003v.html` | 11 / 18 |  |
| `css/css-flexbox/image-as-flexitem-size-004.html` | 13 / 18 |  |
| `css/css-flexbox/image-as-flexitem-size-004v.html` | 13 / 18 |  |
| `css/css-flexbox/image-as-flexitem-size-006.html` | 8 / 18 |  |
| `css/css-flexbox/image-as-flexitem-size-006v.html` | 8 / 18 |  |
| `css/css-flexbox/image-as-flexitem-size-007.html` | 13 / 18 |  |
| `css/css-flexbox/image-as-flexitem-size-007v.html` | 13 / 18 |  |
| `css/css-flexbox/inheritance.html` | 12 / 20 |  |
| `css/css-flexbox/inline-flexbox-wrap-vertically-width-calculation.html` | 2 / 16 |  |
| `css/css-flexbox/intrinsic-size/col-wrap-004.html` | 0 / 1 |  |
| `css/css-flexbox/intrinsic-size/col-wrap-005.html` | 0 / 1 |  |
| `css/css-flexbox/intrinsic-size/col-wrap-010.html` | 0 / 1 |  |
| `css/css-flexbox/intrinsic-size/col-wrap-018.html` | 0 / 1 |  |
| `css/css-flexbox/intrinsic-size/col-wrap-019.html` | 0 / 1 |  |
| `css/css-flexbox/intrinsic-size/col-wrap-baseline-001.html` | 0 / 4 |  |
| `css/css-flexbox/intrinsic-size/col-wrap-calc-size-001.html` | 0 / 2 |  |
| `css/css-flexbox/intrinsic-size/col-wrap-dynamic-002.html` | 0 / 2 |  |
| `css/css-flexbox/intrinsic-size/col-wrap-percentage-001.html` | 0 / 1 |  |
| `css/css-flexbox/intrinsic-size/percentage-gap-001.html` | 0 / 2 |  |
| `css/css-flexbox/intrinsic-size/row-005.html` | 0 / 9 |  |
| `css/css-flexbox/intrinsic-size/row-008.html` | 0 / 11 |  |
| `css/css-flexbox/intrinsic-size/row-compat-001.html` | 2 / 3 |  |
| `css/css-flexbox/intrinsic-size/row-use-cases-001.html` | 3 / 6 |  |
| `css/css-flexbox/intrinsic-size/row-wrap-001.html` | 0 / 5 |  |
| `css/css-flexbox/intrinsic-width-orthogonal-writing-mode.html` | 1 / 3 |  |
| `css/css-flexbox/justify-content-007.html` | 0 / 1 |  |
| `css/css-flexbox/justify-content-rounding.html` | 0 / 1 |  |
| `css/css-flexbox/main-axis-margin-rounding.html` | 0 / 1 |  |
| `css/css-flexbox/negative-overflow-002.html` | 0 / 72 |  |
| `css/css-flexbox/negative-overflow-003.html` | 0 / 48 |  |
| `css/css-flexbox/negative-overflow-004-no-padding.html` | 0 / 96 |  |
| `css/css-flexbox/negative-overflow.html` | 6 / 11 |  |
| `css/css-flexbox/orthogonal-writing-modes-and-intrinsic-sizing.html` | 1 / 2 |  |
| `css/css-flexbox/parsing/flex-basis-computed.html` | 7 / 12 |  |
| `css/css-flexbox/parsing/flex-basis-invalid.html` | 0 / 7 |  |
| `css/css-flexbox/parsing/flex-computed.html` | 0 / 14 |  |
| `css/css-flexbox/parsing/flex-direction-invalid.html` | 0 / 2 |  |
| `css/css-flexbox/parsing/flex-flow-computed.html` | 0 / 2 |  |
| `css/css-flexbox/parsing/flex-flow-invalid.html` | 0 / 2 |  |
| `css/css-flexbox/parsing/flex-flow-shorthand.html` | 2 / 6 |  |
| `css/css-flexbox/parsing/flex-flow-valid.html` | 1 / 7 |  |
| `css/css-flexbox/parsing/flex-grow-invalid.html` | 0 / 4 |  |
| `css/css-flexbox/parsing/flex-grow-valid.html` | 1 / 4 |  |
| `css/css-flexbox/parsing/flex-invalid.html` | 0 / 6 |  |
| `css/css-flexbox/parsing/flex-shorthand.html` | 13 / 68 |  |
| `css/css-flexbox/parsing/flex-shrink-invalid.html` | 0 / 4 |  |
| `css/css-flexbox/parsing/flex-shrink-valid.html` | 1 / 4 |  |
| `css/css-flexbox/parsing/flex-valid.html` | 4 / 16 |  |
| `css/css-flexbox/parsing/flex-wrap-invalid.html` | 0 / 2 |  |
| `css/css-flexbox/parsing/order-computed.html` | 6 / 7 |  |
| `css/css-flexbox/parsing/order-invalid.html` | 0 / 3 |  |
| `css/css-flexbox/parsing/webkit-aliases.html` | 0 / 12 |  |
| `css/css-flexbox/percent-height-flex-items-cross-sizes-with-mutations.html` | 7 / 21 |  |
| `css/css-flexbox/percentage-heights-000.html` | 3 / 6 |  |
| `css/css-flexbox/percentage-heights-001.html` | 5 / 13 |  |
| `css/css-flexbox/percentage-heights-003.html` | 3 / 7 |  |
| `css/css-flexbox/percentage-heights-012.html` | 9 / 10 |  |
| `css/css-flexbox/percentage-heights-013.html` | 0 / 4 |  |
| `css/css-flexbox/percentage-size-quirks-002.html` | 0 / 2 |  |
| `css/css-flexbox/percentage-size-quirks-004.html` | 2 / 4 |  |
| `css/css-flexbox/position-relative-percentage-top-001.html` | 0 / 1 |  |
| `css/css-flexbox/position-relative-percentage-top-004.html` | 8 / 9 |  |
| `css/css-flexbox/position-relative-percentage-top-005.html` | 7 / 8 |  |
| `css/css-flexbox/position-relative-stretch-height-001.html` | 2 / 4 |  |
| `css/css-flexbox/relayout-image-load.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-flexbox/select-as-flex-item-child-with-overflow.html` | 0 / 1 |  |
| `css/css-flexbox/svg-root-as-flex-item-006.html` | 0 / 1 |  |
| `css/css-flexbox/svg-root-as-flex-item-007.html` | 0 / 2 |  |
| `css/css-flexbox/table-as-item-cross-size.html` | 0 / 1 |  |
| `css/css-flexbox/text-as-flexitem-size-001.html` | 13 / 18 |  |
| `css/css-grid/abspos/absolute-positioning-grid-container-containing-block-001.html` | 0 / 10 |  |
| `css/css-grid/abspos/absolute-positioning-grid-container-parent-001.html` | 0 / 12 |  |
| `css/css-grid/abspos/empty-grid-001.html` | 0 / 9 |  |
| `css/css-grid/abspos/grid-positioned-items-and-autofit-tracks-001.html` | 0 / 1 |  |
| `css/css-grid/abspos/grid-positioned-items-and-autofit-tracks-002.html` | 0 / 1 |  |
| `css/css-grid/abspos/grid-positioned-items-and-autofit-tracks-003.html` | 0 / 1 |  |
| `css/css-grid/abspos/grid-positioned-items-and-autofit-tracks-004.html` | 0 / 1 |  |
| `css/css-grid/abspos/grid-positioned-items-and-autofit-tracks-005.html` | 0 / 1 |  |
| `css/css-grid/abspos/grid-positioned-items-and-autofit-tracks-006.html` | 0 / 1 |  |
| `css/css-grid/abspos/grid-positioned-items-and-autofit-tracks-007.html` | 0 / 1 |  |
| `css/css-grid/abspos/grid-positioned-items-content-alignment-001.html` | 12 / 60 |  |
| `css/css-grid/abspos/grid-positioned-items-content-alignment-rtl-001.html` | 12 / 60 |  |
| `css/css-grid/abspos/grid-positioned-items-gaps-001.html` | 3 / 44 |  |
| `css/css-grid/abspos/grid-positioned-items-gaps-002-rtl.html` | 0 / 2 |  |
| `css/css-grid/abspos/grid-positioned-items-gaps-002.html` | 0 / 2 |  |
| `css/css-grid/abspos/grid-positioned-items-gaps-rtl-001.html` | 3 / 44 |  |
| `css/css-grid/abspos/grid-positioned-items-implicit-grid-001.html` | 0 / 4 |  |
| `css/css-grid/abspos/grid-positioned-items-implicit-grid-line-001.html` | 0 / 4 |  |
| `css/css-grid/abspos/grid-positioned-items-padding-001.html` | 0 / 12 |  |
| `css/css-grid/abspos/grid-positioned-items-unknown-named-grid-line-001.html` | 0 / 2 |  |
| `css/css-grid/abspos/grid-positioned-items-within-grid-implicit-track-001.html` | 0 / 16 |  |
| `css/css-grid/abspos/grid-sizing-positioned-items-001.html` | 0 / 8 |  |
| `css/css-grid/abspos/orthogonal-positioned-grid-descendants-001.html` | 0 / 100 |  |
| `css/css-grid/abspos/orthogonal-positioned-grid-descendants-002.html` | 0 / 100 |  |
| `css/css-grid/abspos/orthogonal-positioned-grid-descendants-003.html` | 0 / 100 |  |
| `css/css-grid/abspos/orthogonal-positioned-grid-descendants-004.html` | 0 / 100 |  |
| `css/css-grid/abspos/orthogonal-positioned-grid-descendants-005.html` | 0 / 100 |  |
| `css/css-grid/abspos/orthogonal-positioned-grid-descendants-006.html` | 0 / 100 |  |
| `css/css-grid/abspos/orthogonal-positioned-grid-descendants-007.html` | 0 / 100 |  |
| `css/css-grid/abspos/orthogonal-positioned-grid-descendants-008.html` | 0 / 100 |  |
| `css/css-grid/abspos/orthogonal-positioned-grid-descendants-009.html` | 0 / 100 |  |
| `css/css-grid/abspos/orthogonal-positioned-grid-descendants-010.html` | 0 / 100 |  |
| `css/css-grid/abspos/orthogonal-positioned-grid-descendants-011.html` | 0 / 100 |  |
| `css/css-grid/abspos/orthogonal-positioned-grid-descendants-012.html` | 0 / 100 |  |
| `css/css-grid/abspos/orthogonal-positioned-grid-descendants-013.html` | 0 / 100 |  |
| `css/css-grid/abspos/orthogonal-positioned-grid-descendants-014.html` | 0 / 100 |  |
| `css/css-grid/abspos/orthogonal-positioned-grid-descendants-015.html` | 0 / 100 |  |
| `css/css-grid/abspos/orthogonal-positioned-grid-descendants-016.html` | 1 / 100 |  |
| `css/css-grid/abspos/positioned-grid-descendants-002.html` | 40 / 100 |  |
| `css/css-grid/abspos/positioned-grid-descendants-003.html` | 40 / 100 |  |
| `css/css-grid/abspos/positioned-grid-descendants-004.html` | 40 / 100 |  |
| `css/css-grid/abspos/positioned-grid-descendants-005.html` | 40 / 100 |  |
| `css/css-grid/abspos/positioned-grid-descendants-006.html` | 16 / 100 |  |
| `css/css-grid/abspos/positioned-grid-descendants-007.html` | 10 / 100 |  |
| `css/css-grid/abspos/positioned-grid-descendants-008.html` | 16 / 100 |  |
| `css/css-grid/abspos/positioned-grid-descendants-009.html` | 16 / 100 |  |
| `css/css-grid/abspos/positioned-grid-descendants-010.html` | 10 / 100 |  |
| `css/css-grid/abspos/positioned-grid-descendants-011.html` | 16 / 100 |  |
| `css/css-grid/abspos/positioned-grid-descendants-012.html` | 4 / 100 |  |
| `css/css-grid/abspos/positioned-grid-descendants-013.html` | 4 / 100 |  |
| `css/css-grid/abspos/positioned-grid-descendants-014.html` | 4 / 100 |  |
| `css/css-grid/abspos/positioned-grid-descendants-015.html` | 4 / 100 |  |
| `css/css-grid/abspos/positioned-grid-descendants-016.html` | 1 / 100 |  |
| `css/css-grid/abspos/positioned-grid-items-should-not-take-up-space-001.html` | 6 / 12 |  |
| `css/css-grid/alignment/grid-align-baseline-001.html` | 0 / 6 |  |
| `css/css-grid/alignment/grid-align-baseline-002.html` | 0 / 6 |  |
| `css/css-grid/alignment/grid-align-baseline-003.html` | 0 / 3 |  |
| `css/css-grid/alignment/grid-align-baseline-004.html` | 1 / 2 |  |
| `css/css-grid/alignment/grid-align-baseline-fieldset-001.html` | 2 / 4 |  |
| `css/css-grid/alignment/grid-align-baseline-fieldset-002.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-align-baseline-fieldset-003.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-align-baseline-flex-001.html` | 24 / 48 |  |
| `css/css-grid/alignment/grid-align-baseline-flex-002.html` | 0 / 48 |  |
| `css/css-grid/alignment/grid-align-baseline-flex-003.html` | 8 / 16 |  |
| `css/css-grid/alignment/grid-align-baseline-flex-004.html` | 0 / 16 |  |
| `css/css-grid/alignment/grid-align-baseline-grid-001.html` | 6 / 12 |  |
| `css/css-grid/alignment/grid-align-baseline-grid-002.html` | 0 / 12 |  |
| `css/css-grid/alignment/grid-align-baseline-grid-003.html` | 0 / 12 |  |
| `css/css-grid/alignment/grid-align-baseline-line-clamp-001.tentative.html` | 13 / 26 |  |
| `css/css-grid/alignment/grid-align-baseline-line-clamp-002.tentative.html` | 0 / 24 |  |
| `css/css-grid/alignment/grid-align-baseline-line-clamp-003.tentative.html` | 0 / 24 |  |
| `css/css-grid/alignment/grid-align-baseline-multicol-001.html` | 6 / 12 |  |
| `css/css-grid/alignment/grid-align-baseline-multicol-002.html` | 0 / 12 |  |
| `css/css-grid/alignment/grid-align-baseline-multicol-003.html` | 0 / 12 |  |
| `css/css-grid/alignment/grid-align-baseline-nested-intrinsic-sizing-001.html` | 0 / 3 |  |
| `css/css-grid/alignment/grid-align-baseline-overflow-001.html` | 6 / 12 |  |
| `css/css-grid/alignment/grid-align-baseline-overflow-002.html` | 2 / 12 |  |
| `css/css-grid/alignment/grid-align-baseline-overflow-003.html` | 2 / 12 |  |
| `css/css-grid/alignment/grid-align-baseline-single-item-001.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-align-baseline-table-001.html` | 4 / 8 |  |
| `css/css-grid/alignment/grid-align-baseline-table-002.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-align-baseline-table-003.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-align-baseline-vertical-mixed.html` | 1 / 11 |  |
| `css/css-grid/alignment/grid-align-baseline-vertical.html` | 1 / 11 |  |
| `css/css-grid/alignment/grid-align-baseline.html` | 2 / 9 |  |
| `css/css-grid/alignment/grid-align-content-distribution-vertical-lr.html` | 0 / 24 |  |
| `css/css-grid/alignment/grid-align-content-distribution-vertical-rl.html` | 0 / 24 |  |
| `css/css-grid/alignment/grid-align-content-distribution.html` | 12 / 24 |  |
| `css/css-grid/alignment/grid-align-content-vertical-lr.html` | 0 / 6 |  |
| `css/css-grid/alignment/grid-align-content-vertical-rl.html` | 0 / 6 |  |
| `css/css-grid/alignment/grid-align-content.html` | 7 / 13 |  |
| `css/css-grid/alignment/grid-align-justify-margin-border-padding-vertical-lr.html` | 0 / 18 |  |
| `css/css-grid/alignment/grid-align-justify-margin-border-padding-vertical-rl.html` | 0 / 18 |  |
| `css/css-grid/alignment/grid-align-justify-margin-border-padding.html` | 0 / 18 |  |
| `css/css-grid/alignment/grid-align-justify-overflow.html` | 0 / 8 |  |
| `css/css-grid/alignment/grid-align-justify-stretch-with-orthogonal-flows.html` | 0 / 8 |  |
| `css/css-grid/alignment/grid-align-justify-stretch.html` | 0 / 16 |  |
| `css/css-grid/alignment/grid-align-stretching-replaced-items.html` | 5 / 6 |  |
| `css/css-grid/alignment/grid-align.html` | 0 / 16 |  |
| `css/css-grid/alignment/grid-alignment-implies-size-change-011.html` | 1 / 2 |  |
| `css/css-grid/alignment/grid-alignment-implies-size-change-012.html` | 1 / 2 |  |
| `css/css-grid/alignment/grid-alignment-implies-size-change-013.html` | 1 / 2 |  |
| `css/css-grid/alignment/grid-alignment-implies-size-change-014.html` | 1 / 2 |  |
| `css/css-grid/alignment/grid-alignment-implies-size-change-017.html` | 1 / 2 |  |
| `css/css-grid/alignment/grid-alignment-implies-size-change-018.html` | 1 / 2 |  |
| `css/css-grid/alignment/grid-alignment-implies-size-change-025.html` | 1 / 2 |  |
| `css/css-grid/alignment/grid-alignment-implies-size-change-026.html` | 1 / 2 |  |
| `css/css-grid/alignment/grid-alignment-implies-size-change-027.html` | 1 / 2 |  |
| `css/css-grid/alignment/grid-alignment-implies-size-change-028.html` | 1 / 2 |  |
| `css/css-grid/alignment/grid-alignment-implies-size-change-029.html` | 1 / 2 |  |
| `css/css-grid/alignment/grid-alignment-implies-size-change-030.html` | 1 / 2 |  |
| `css/css-grid/alignment/grid-alignment-implies-size-change-031.html` | 1 / 2 |  |
| `css/css-grid/alignment/grid-alignment-implies-size-change-032.html` | 1 / 2 |  |
| `css/css-grid/alignment/grid-alignment-implies-size-change-035.html` | 1 / 2 |  |
| `css/css-grid/alignment/grid-alignment-implies-size-change-036.html` | 1 / 2 |  |
| `css/css-grid/alignment/grid-alignment-style-changes-001.html` | 3 / 6 |  |
| `css/css-grid/alignment/grid-alignment-style-changes-002.html` | 3 / 6 |  |
| `css/css-grid/alignment/grid-alignment-style-changes-003.html` | 1 / 6 |  |
| `css/css-grid/alignment/grid-alignment-style-changes-004.html` | 1 / 6 |  |
| `css/css-grid/alignment/grid-alignment-style-changes-005.html` | 3 / 6 |  |
| `css/css-grid/alignment/grid-alignment-style-changes-006.html` | 3 / 6 |  |
| `css/css-grid/alignment/grid-alignment-style-changes-007.html` | 3 / 6 |  |
| `css/css-grid/alignment/grid-alignment-style-changes-008.html` | 3 / 6 |  |
| `css/css-grid/alignment/grid-baseline-004.html` | 0 / 18 |  |
| `css/css-grid/alignment/grid-column-axis-alignment-positioned-items-001.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-column-axis-alignment-positioned-items-002.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-column-axis-alignment-positioned-items-003.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-column-axis-alignment-positioned-items-004.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-column-axis-alignment-positioned-items-005.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-column-axis-alignment-positioned-items-006.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-column-axis-alignment-positioned-items-007.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-column-axis-alignment-positioned-items-008.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-column-axis-alignment-positioned-items-009.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-column-axis-alignment-positioned-items-010.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-column-axis-alignment-positioned-items-011.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-column-axis-alignment-positioned-items-012.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-column-axis-alignment-positioned-items-013.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-column-axis-alignment-positioned-items-014.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-column-axis-alignment-positioned-items-015.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-column-axis-alignment-positioned-items-016.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-column-axis-alignment-positioned-items-017.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-column-axis-alignment-sticky-positioned-items-002.html` | 0 / 2 |  |
| `css/css-grid/alignment/grid-column-axis-self-baseline-synthesized-001.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-column-axis-self-baseline-synthesized-002.html` | 0 / 3 |  |
| `css/css-grid/alignment/grid-column-axis-self-baseline-synthesized-003.html` | 0 / 3 |  |
| `css/css-grid/alignment/grid-column-axis-self-baseline-synthesized-004.html` | 0 / 3 |  |
| `css/css-grid/alignment/grid-column-axis-self-baseline-synthesized-005.html` | 0 / 2 |  |
| `css/css-grid/alignment/grid-container-baseline-001.html` | 0 / 11 |  |
| `css/css-grid/alignment/grid-container-baseline-first-item-in-grid-order.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-content-alignment-and-self-alignment-001.html` | 12 / 24 |  |
| `css/css-grid/alignment/grid-content-alignment-and-self-alignment-002.html` | 16 / 32 |  |
| `css/css-grid/alignment/grid-content-alignment-overflow-001.html` | 2 / 14 |  |
| `css/css-grid/alignment/grid-content-alignment-overflow-002.html` | 1 / 14 |  |
| `css/css-grid/alignment/grid-content-alignment-second-pass-001.html` | 2 / 8 |  |
| `css/css-grid/alignment/grid-content-alignment-second-pass-002.html` | 0 / 8 |  |
| `css/css-grid/alignment/grid-content-alignment-with-abspos-001.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-content-alignment-with-span-001.html` | 4 / 8 |  |
| `css/css-grid/alignment/grid-content-alignment-with-span-vertical-lr-001.html` | 0 / 8 |  |
| `css/css-grid/alignment/grid-content-alignment-with-span-vertical-rl-001.html` | 0 / 8 |  |
| `css/css-grid/alignment/grid-gutters-and-alignment.html` | 9 / 25 |  |
| `css/css-grid/alignment/grid-item-alignment-with-orthogonal-flows-vertical-lr.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-item-alignment-with-orthogonal-flows-vertical-rl.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-item-alignment-with-orthogonal-flows.html` | 1 / 7 |  |
| `css/css-grid/alignment/grid-item-aspect-ratio-justify-self-001.html` | 0 / 20 |  |
| `css/css-grid/alignment/grid-item-aspect-ratio-justify-self-002.html` | 0 / 20 |  |
| `css/css-grid/alignment/grid-item-auto-margins-alignment-vertical-lr.html` | 0 / 8 |  |
| `css/css-grid/alignment/grid-item-auto-margins-alignment-vertical-rl.html` | 0 / 8 |  |
| `css/css-grid/alignment/grid-item-auto-margins-alignment.html` | 4 / 8 |  |
| `css/css-grid/alignment/grid-item-auto-margins-and-stretch-001.html` | 0 / 6 |  |
| `css/css-grid/alignment/grid-item-stretch-dynamic-size-margin-change-001.html` | 3 / 6 |  |
| `css/css-grid/alignment/grid-item-stretch-margin-border-padding-001.html` | 0 / 12 |  |
| `css/css-grid/alignment/grid-item-stretch-margin-border-padding-vertical-lr-001.html` | 0 / 12 |  |
| `css/css-grid/alignment/grid-item-stretch-margin-border-padding-vertical-rl-001.html` | 0 / 12 |  |
| `css/css-grid/alignment/grid-justify-baseline-002.html` | 1 / 6 |  |
| `css/css-grid/alignment/grid-justify-baseline-003.html` | 0 / 3 |  |
| `css/css-grid/alignment/grid-justify-baseline-004.html` | 0 / 3 |  |
| `css/css-grid/alignment/grid-justify-baseline-005.html` | 2 / 4 |  |
| `css/css-grid/alignment/grid-justify-content-001.html` | 8 / 13 |  |
| `css/css-grid/alignment/grid-justify-content-distribution-001.html` | 12 / 24 |  |
| `css/css-grid/alignment/grid-justify-content-distribution-vertical-lr-001.html` | 0 / 24 |  |
| `css/css-grid/alignment/grid-justify-content-distribution-vertical-rl-001.html` | 0 / 24 |  |
| `css/css-grid/alignment/grid-justify-content-vertical-lr-001.html` | 0 / 10 |  |
| `css/css-grid/alignment/grid-justify-content-vertical-rl-001.html` | 0 / 10 |  |
| `css/css-grid/alignment/grid-justify-self-cell-001.html` | 0 / 10 |  |
| `css/css-grid/alignment/grid-place-content-001.html` | 18 / 20 |  |
| `css/css-grid/alignment/grid-row-axis-alignment-positioned-items-001.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-row-axis-alignment-positioned-items-002.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-row-axis-alignment-positioned-items-003.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-row-axis-alignment-positioned-items-004.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-row-axis-alignment-positioned-items-005.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-row-axis-alignment-positioned-items-006.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-row-axis-alignment-positioned-items-007.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-row-axis-alignment-positioned-items-008.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-row-axis-alignment-positioned-items-009.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-row-axis-alignment-positioned-items-010.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-row-axis-alignment-positioned-items-011.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-row-axis-alignment-positioned-items-012.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-row-axis-alignment-positioned-items-013.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-row-axis-alignment-positioned-items-014.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-row-axis-alignment-positioned-items-015.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-row-axis-alignment-positioned-items-016.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-row-axis-alignment-positioned-items-017.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-row-axis-alignment-sticky-positioned-items-002.html` | 0 / 2 |  |
| `css/css-grid/alignment/grid-row-axis-self-baseline-synthesized-001.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-row-axis-self-baseline-synthesized-002.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-row-axis-self-baseline-synthesized-003.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-row-axis-self-baseline-synthesized-004.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-row-axis-self-baseline-synthesized-005.html` | 0 / 2 |  |
| `css/css-grid/alignment/grid-self-alignment-non-static-positioned-items-001.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-self-alignment-non-static-positioned-items-002.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-self-alignment-non-static-positioned-items-003.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-self-alignment-non-static-positioned-items-004.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-self-alignment-non-static-positioned-items-005.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-self-alignment-non-static-positioned-items-006.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-self-alignment-non-static-positioned-items-007.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-self-alignment-non-static-positioned-items-008.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-self-alignment-non-static-positioned-items-009.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-self-alignment-non-static-positioned-items-010.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-self-alignment-non-static-positioned-items-011.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-self-alignment-non-static-positioned-items-012.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-self-alignment-positioned-items-with-margin-border-padding-001.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-self-alignment-positioned-items-with-margin-border-padding-002.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-self-alignment-positioned-items-with-margin-border-padding-003.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-self-alignment-positioned-items-with-margin-border-padding-004.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-positioned-items-with-margin-border-padding-005.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-self-alignment-positioned-items-with-margin-border-padding-006.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-self-alignment-positioned-items-with-margin-border-padding-007.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-self-alignment-positioned-items-with-margin-border-padding-008.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-self-alignment-positioned-items-with-margin-border-padding-009.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-self-alignment-positioned-items-with-margin-border-padding-010.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-self-alignment-positioned-items-with-margin-border-padding-011.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-self-alignment-positioned-items-with-margin-border-padding-012.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-self-alignment-positioned-items-with-margin-border-padding-013.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-self-alignment-positioned-items-with-margin-border-padding-014.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-self-alignment-positioned-items-with-margin-border-padding-015.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-self-alignment-positioned-items-with-margin-border-padding-016.html` | 0 / 4 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-005.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-006.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-007.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-008.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-013.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-014.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-015.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-016.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-vertical-lr-001.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-vertical-lr-002.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-vertical-lr-003.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-vertical-lr-004.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-vertical-lr-005.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-vertical-lr-006.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-vertical-lr-007.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-vertical-lr-008.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-vertical-lr-009.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-vertical-lr-010.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-vertical-lr-011.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-vertical-lr-012.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-vertical-lr-013.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-vertical-lr-014.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-vertical-lr-015.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-vertical-lr-016.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-vertical-rl-001.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-vertical-rl-002.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-vertical-rl-003.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-vertical-rl-004.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-vertical-rl-005.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-vertical-rl-006.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-vertical-rl-007.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-vertical-rl-008.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-vertical-rl-009.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-vertical-rl-010.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-vertical-rl-011.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-vertical-rl-012.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-vertical-rl-013.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-vertical-rl-014.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-vertical-rl-015.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-stretch-vertical-rl-016.html` | 0 / 1 |  |
| `css/css-grid/alignment/grid-self-alignment-unsafe-overflow-001.html` | 8 / 10 |  |
| `css/css-grid/alignment/grid-self-alignment.html` | 24 / 72 |  |
| `css/css-grid/alignment/grid-self-baseline-not-applied-if-sizing-cyclic-dependency-001.html` | 8 / 12 |  |
| `css/css-grid/alignment/grid-self-baseline-not-applied-if-sizing-cyclic-dependency-002.html` | 0 / 24 |  |
| `css/css-grid/alignment/grid-self-baseline-not-applied-if-sizing-cyclic-dependency-003.html` | 0 / 6 |  |
| `css/css-grid/alignment/self-baseline/grid-self-baseline-two-dimensional.html` | 0 / 12 |  |
| `css/css-grid/animation/grid-no-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-grid/animation/grid-template-columns-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-grid/animation/grid-template-columns-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-grid/animation/grid-template-columns-neutral-keyframe-001.html` | 0 / 1 |  |
| `css/css-grid/animation/grid-template-columns-neutral-keyframe-002.html` | 0 / 1 |  |
| `css/css-grid/animation/grid-template-columns-neutral-keyframe-003.html` | 0 / 1 |  |
| `css/css-grid/animation/grid-template-columns-neutral-keyframe-004.html` | 0 / 1 |  |
| `css/css-grid/animation/grid-template-rows-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-grid/animation/grid-template-rows-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-grid/animation/grid-template-rows-neutral-keyframe-001.html` | 0 / 1 |  |
| `css/css-grid/animation/grid-template-rows-neutral-keyframe-002.html` | 0 / 1 |  |
| `css/css-grid/animation/grid-template-rows-neutral-keyframe-003.html` | 0 / 1 |  |
| `css/css-grid/animation/grid-template-rows-neutral-keyframe-004.html` | 0 / 1 |  |
| `css/css-grid/computed-grid-column.html` | 0 / 1 |  |
| `css/css-grid/grid-definition/fit-content-track-sizing.html` | 6 / 13 |  |
| `css/css-grid/grid-definition/flex-content-resolution-columns-002.html` | 5 / 11 |  |
| `css/css-grid/grid-definition/flex-content-resolution-rows-002.html` | 17 / 21 |  |
| `css/css-grid/grid-definition/grid-auto-fill-columns-001.html` | 17 / 33 |  |
| `css/css-grid/grid-definition/grid-auto-fill-rows-001.html` | 17 / 35 |  |
| `css/css-grid/grid-definition/grid-auto-fit-columns-001.html` | 16 / 35 |  |
| `css/css-grid/grid-definition/grid-auto-fit-rows-001.html` | 16 / 33 |  |
| `css/css-grid/grid-definition/grid-auto-repeat-intrinsic-001.html` | 0 / 8 |  |
| `css/css-grid/grid-definition/grid-auto-repeat-max-size-001.tentative.html` | 3 / 20 |  |
| `css/css-grid/grid-definition/grid-auto-repeat-max-size-002.html` | 1 / 4 |  |
| `css/css-grid/grid-definition/grid-auto-repeat-min-max-size-001.html` | 1 / 4 |  |
| `css/css-grid/grid-definition/grid-auto-repeat-min-size-001.html` | 0 / 12 |  |
| `css/css-grid/grid-definition/grid-auto-repeat-min-size-002.html` | 0 / 4 |  |
| `css/css-grid/grid-definition/grid-auto-repeat-min-size-004.html` | 0 / 1 |  |
| `css/css-grid/grid-definition/grid-auto-repeat-rows-dynamic-height-001.html` | 0 / 3 |  |
| `css/css-grid/grid-definition/grid-change-fit-content-argument-001.html` | 6 / 8 |  |
| `css/css-grid/grid-definition/grid-change-intrinsic-size-with-auto-repeat-tracks-001.html` | 0 / 8 |  |
| `css/css-grid/grid-definition/grid-inline-auto-repeat-001.html` | 5 / 12 |  |
| `css/css-grid/grid-definition/grid-inline-support-flexible-lengths-001.html` | 58 / 62 |  |
| `css/css-grid/grid-definition/grid-inline-support-grid-template-areas-001.html` | 0 / 45 |  |
| `css/css-grid/grid-definition/grid-inline-support-named-grid-lines-001.html` | 40 / 80 |  |
| `css/css-grid/grid-definition/grid-inline-support-repeat-001.html` | 26 / 40 |  |
| `css/css-grid/grid-definition/grid-inline-template-columns-rows-resolved-values-001.tentative.html` | 40 / 76 |  |
| `css/css-grid/grid-definition/grid-limits-001.html` | 0 / 2 |  |
| `css/css-grid/grid-definition/grid-minimum-contribution-with-percentages.html` | 8 / 16 |  |
| `css/css-grid/grid-definition/grid-percentage-max-size-flex-rows-001.html` | 0 / 4 |  |
| `css/css-grid/grid-definition/grid-percentage-max-size-flex-rows-002.html` | 0 / 4 |  |
| `css/css-grid/grid-definition/grid-percentage-rows-indefinite-height-001.html` | 16 / 120 |  |
| `css/css-grid/grid-definition/grid-percentage-rows-indefinite-height-002.html` | 0 / 4 |  |
| `css/css-grid/grid-definition/grid-shorthand-001.html` | 0 / 2 |  |
| `css/css-grid/grid-definition/grid-support-flexible-lengths-001.html` | 58 / 62 |  |
| `css/css-grid/grid-definition/grid-support-grid-template-areas-001.html` | 0 / 45 |  |
| `css/css-grid/grid-definition/grid-support-named-grid-lines-001.html` | 40 / 80 |  |
| `css/css-grid/grid-definition/grid-support-repeat-001.html` | 26 / 40 |  |
| `css/css-grid/grid-definition/grid-support-repeat-002.html` | 13 / 20 |  |
| `css/css-grid/grid-definition/grid-template-columns-rows-changes-001.html` | 14 / 21 |  |
| `css/css-grid/grid-definition/grid-template-columns-rows-resolved-values-001.tentative.html` | 40 / 76 |  |
| `css/css-grid/grid-important.html` | 0 / 1 |  |
| `css/css-grid/grid-items/grid-automatic-minimum-intrinsic-aspect-ratio-001.html` | 0 / 14 |  |
| `css/css-grid/grid-items/grid-item-children-percentage-resolution-001.html` | 8 / 14 |  |
| `css/css-grid/grid-items/grid-item-min-auto-size-001.html` | 2 / 4 |  |
| `css/css-grid/grid-items/grid-item-min-contribution-behaves-as-auto-001.html` | 0 / 2 |  |
| `css/css-grid/grid-items/grid-item-min-height-border-box-001.html` | 6 / 14 |  |
| `css/css-grid/grid-items/grid-item-min-width-margin-box-001.html` | 4 / 14 |  |
| `css/css-grid/grid-items/grid-item-minimum-size-single-axis-scroll-container.html` | 2 / 4 |  |
| `css/css-grid/grid-items/grid-item-percentage-resolution-writing-modes-001.html` | 3 / 4 |  |
| `css/css-grid/grid-items/grid-item-percentage-resolution-writing-modes-quirks-001.html` | 3 / 4 |  |
| `css/css-grid/grid-items/grid-items-minimum-width-vertical-lr-001.html` | 0 / 44 |  |
| `css/css-grid/grid-items/grid-items-minimum-width-vertical-lr-002.html` | 0 / 44 |  |
| `css/css-grid/grid-items/grid-items-minimum-width-vertical-rl-001.html` | 0 / 44 |  |
| `css/css-grid/grid-items/grid-items-minimum-width-vertical-rl-002.html` | 0 / 44 |  |
| `css/css-grid/grid-items/grid-items-percentage-margins-001.html` | 0 / 8 |  |
| `css/css-grid/grid-items/grid-items-percentage-margins-002.html` | 0 / 8 |  |
| `css/css-grid/grid-items/grid-items-percentage-margins-vertical-lr-001.html` | 0 / 8 |  |
| `css/css-grid/grid-items/grid-items-percentage-margins-vertical-lr-002.html` | 0 / 8 |  |
| `css/css-grid/grid-items/grid-items-percentage-margins-vertical-rl-001.html` | 0 / 8 |  |
| `css/css-grid/grid-items/grid-items-percentage-margins-vertical-rl-002.html` | 0 / 8 |  |
| `css/css-grid/grid-items/grid-items-percentage-paddings-001.html` | 0 / 8 |  |
| `css/css-grid/grid-items/grid-items-percentage-paddings-002.html` | 0 / 8 |  |
| `css/css-grid/grid-items/grid-items-percentage-paddings-015.html` | 0 / 1 |  |
| `css/css-grid/grid-items/grid-items-percentage-paddings-margins-track-change-001.html` | 0 / 1 |  |
| `css/css-grid/grid-items/grid-items-percentage-paddings-margins-writing-modes-001.html` | 0 / 4 |  |
| `css/css-grid/grid-items/grid-items-percentage-paddings-vertical-lr-001.html` | 0 / 8 |  |
| `css/css-grid/grid-items/grid-items-percentage-paddings-vertical-lr-002.html` | 0 / 8 |  |
| `css/css-grid/grid-items/grid-items-percentage-paddings-vertical-rl-001.html` | 0 / 8 |  |
| `css/css-grid/grid-items/grid-items-percentage-paddings-vertical-rl-002.html` | 0 / 8 |  |
| `css/css-grid/grid-items/grid-items-relative-offsets-001.html` | 1 / 6 |  |
| `css/css-grid/grid-items/grid-items-relative-offsets-002.html` | 0 / 6 |  |
| `css/css-grid/grid-items/grid-minimum-size-grid-items-021.html` | 72 / 144 |  |
| `css/css-grid/grid-items/grid-minimum-size-grid-items-022.html` | 27 / 45 |  |
| `css/css-grid/grid-items/grid-minimum-size-grid-items-023.html` | 40 / 60 |  |
| `css/css-grid/grid-items/grid-minimum-size-grid-items-024.html` | 13 / 45 |  |
| `css/css-grid/grid-items/grid-minimum-size-grid-items-025.html` | 26 / 60 |  |
| `css/css-grid/grid-lanes/animation/fit-tolerance-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-grid/grid-lanes/animation/grid-template-columns-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-grid/grid-lanes/animation/grid-template-rows-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-grid/grid-lanes/grid-lanes-container-minimum-size-single-axis-scroll-container.html` | 2 / 6 |  |
| `css/css-grid/grid-lanes/grid-lanes-grid-template-columns-computed-withcontent.html` | 0 / 29 |  |
| `css/css-grid/grid-lanes/intrinsic-sizing/grid-lanes-contain-intrinsic-size-009.html` | 0 / 7 |  |
| `css/css-grid/grid-lanes/invalidation/grid-lanes-add-child-dynamic-change.html` | 0 / 2 |  |
| `css/css-grid/grid-lanes/invalidation/grid-lanes-change-intrinsic-size-with-auto-repeat-tracks-001.html` | 0 / 3 |  |
| `css/css-grid/grid-lanes/invalidation/grid-lanes-column-tracks-recompute-child-positions-001.html` | 0 / 3 |  |
| `css/css-grid/grid-lanes/invalidation/grid-lanes-direction-dynamic-change-002.html` | 0 / 3 |  |
| `css/css-grid/grid-lanes/invalidation/grid-lanes-direction-dynamic-change-003.html` | 0 / 3 |  |
| `css/css-grid/grid-lanes/invalidation/grid-lanes-grid-template-columns-changes-001.html` | 1 / 12 |  |
| `css/css-grid/grid-lanes/invalidation/grid-lanes-grid-template-rows-changes-001.html` | 1 / 12 |  |
| `css/css-grid/grid-lanes/invalidation/grid-lanes-remove-child-dynamic-change.html` | 0 / 2 |  |
| `css/css-grid/grid-lanes/invalidation/grid-lanes-row-tracks-recompute-child-positions-001.html` | 0 / 3 |  |
| `css/css-grid/grid-lanes/items/column-automatic-minimum-for-auto.html` | 0 / 4 |  |
| `css/css-grid/grid-lanes/items/column-fit-content-percentage.html` | 0 / 20 |  |
| `css/css-grid/grid-lanes/items/column-flex-track-intrinsic-sizes.html` | 0 / 102 |  |
| `css/css-grid/grid-lanes/items/column-intrinsic-track-sizes.html` | 0 / 31 |  |
| `css/css-grid/grid-lanes/items/column-item-percentage-sizes-001.html` | 0 / 49 |  |
| `css/css-grid/grid-lanes/items/column-item-percentage-sizes-002.html` | 0 / 49 |  |
| `css/css-grid/grid-lanes/items/column-item-percentage-sizes-003.html` | 0 / 49 |  |
| `css/css-grid/grid-lanes/items/column-minimum-contribution-baseline-shim-vertical-lr.html` | 0 / 6 |  |
| `css/css-grid/grid-lanes/items/column-minimum-contribution-baseline-shim-vertical-rl.html` | 0 / 6 |  |
| `css/css-grid/grid-lanes/items/column-minimum-contribution-with-percentages.html` | 0 / 8 |  |
| `css/css-grid/grid-lanes/items/column-minimum-size-grid-items-002.html` | 0 / 45 |  |
| `css/css-grid/grid-lanes/items/column-minimum-size-grid-items-003.html` | 0 / 45 |  |
| `css/css-grid/grid-lanes/items/row-automatic-minimum-for-auto.html` | 0 / 4 |  |
| `css/css-grid/grid-lanes/items/row-flex-track-intrinsic-sizes.html` | 0 / 102 |  |
| `css/css-grid/grid-lanes/items/row-intrinsic-track-sizes.html` | 0 / 31 |  |
| `css/css-grid/grid-lanes/items/row-minimum-contribution-baseline-shim.html` | 0 / 6 |  |
| `css/css-grid/grid-lanes/items/row-minimum-contribution-with-percentages.html` | 0 / 8 |  |
| `css/css-grid/grid-lanes/items/row-minimum-size-grid-items-002.html` | 0 / 60 |  |
| `css/css-grid/grid-lanes/items/row-minimum-size-grid-items-003.html` | 0 / 60 |  |
| `css/css-grid/grid-lanes/subgrid/grid-lanes-subgridded-to-grid-lanes/track-sizing/row-subgrid-with-scrollbar-002.html` | 0 / 2 |  |
| `css/css-grid/grid-lanes/subgrid/grid-lanes-subgridded-to-grid-lanes/track-sizing/row-subgrid-with-scrollbar-003.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/subgrid/grid-subgridded-to-grid-lanes/column-subgrid-placement-invalidation-001.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/subgrid/grid-subgridded-to-grid-lanes/row-subgrid-placement-invalidation-001.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/subgrid/grid-subgridded-to-grid-lanes/track-sizing/column-subgrid-auto-fill-006.html` | 0 / 21 |  |
| `css/css-grid/grid-lanes/subgrid/grid-subgridded-to-grid-lanes/track-sizing/column-subgrid-auto-fit-002.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/subgrid/grid-subgridded-to-grid-lanes/track-sizing/column-subgrid-auto-fit-003.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/subgrid/grid-subgridded-to-grid-lanes/track-sizing/column-subgrid-auto-repeat-auto-001.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/subgrid/grid-subgridded-to-grid-lanes/track-sizing/column-subgrid-auto-repeat-auto-002.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/subgrid/grid-subgridded-to-grid-lanes/track-sizing/column-subgrid-auto-repeat-auto-003.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/subgrid/grid-subgridded-to-grid-lanes/track-sizing/column-subgrid-auto-repeat-auto-004.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/subgrid/grid-subgridded-to-grid-lanes/track-sizing/row-subgrid-auto-fill-006.html` | 0 / 21 |  |
| `css/css-grid/grid-lanes/subgrid/grid-subgridded-to-grid-lanes/track-sizing/row-subgrid-auto-fit-002.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/subgrid/grid-subgridded-to-grid-lanes/track-sizing/row-subgrid-auto-fit-003.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/subgrid/grid-subgridded-to-grid-lanes/track-sizing/row-subgrid-auto-repeat-auto-001.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/subgrid/grid-subgridded-to-grid-lanes/track-sizing/row-subgrid-auto-repeat-auto-002.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/subgrid/grid-subgridded-to-grid-lanes/track-sizing/row-subgrid-auto-repeat-auto-003.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/subgrid/grid-subgridded-to-grid-lanes/track-sizing/row-subgrid-auto-repeat-auto-004.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/tentative/intrinsic-sizing/grid-lanes-baseline-alignment-affects-intrinsic-size-006.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/tentative/intrinsic-sizing/grid-lanes-baseline-alignment-affects-intrinsic-size-009.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/tentative/parsing/fit-tolerance-computed.html` | 0 / 7 |  |
| `css/css-grid/grid-lanes/tentative/parsing/fit-tolerance-invalid.html` | 0 / 12 |  |
| `css/css-grid/grid-lanes/tentative/parsing/fit-tolerance-valid.html` | 7 / 8 |  |
| `css/css-grid/grid-lanes/tentative/parsing/grid-lanes-direction-computed.html` | 0 / 11 |  |
| `css/css-grid/grid-lanes/tentative/parsing/grid-lanes-direction-invalid.html` | 0 / 13 |  |
| `css/css-grid/grid-lanes/tentative/parsing/grid-lanes-pack-computed.html` | 0 / 2 |  |
| `css/css-grid/grid-lanes/tentative/parsing/grid-lanes-pack-invalid.html` | 0 / 5 |  |
| `css/css-grid/grid-lanes/tentative/parsing/grid-lanes-shorthand-computed.html` | 0 / 14 |  |
| `css/css-grid/grid-lanes/tentative/parsing/grid-lanes-shorthand-invalid.html` | 0 / 21 |  |
| `css/css-grid/grid-lanes/tentative/parsing/grid-lanes-shorthand-serialization.html` | 0 / 15 |  |
| `css/css-grid/grid-lanes/tentative/parsing/grid-lanes-shorthand-valid.html` | 0 / 57 |  |
| `css/css-grid/grid-lanes/tentative/parsing/grid-subgridded-to-grid-lanes/grid-lanes-column-subgrid-computed.html` | 0 / 14 |  |
| `css/css-grid/grid-lanes/tentative/parsing/grid-subgridded-to-grid-lanes/grid-lanes-row-subgrid-computed.html` | 0 / 14 |  |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat/column-auto-repeat-008.html` | 0 / 12 |  |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat/column-auto-repeat-009.html` | 0 / 3 |  |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat/column-auto-repeat-010.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat/column-auto-repeat-011.html` | 0 / 4 |  |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat/column-auto-repeat-012.html` | 0 / 4 |  |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat/column-auto-repeat-019.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat/column-auto-repeat-020.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat/column-auto-repeat-024.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat/column-auto-repeat-025.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat/intrinsic-auto-repeat/column-auto-repeat-auto-009.html` | 0 / 8 |  |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat/intrinsic-auto-repeat/column-auto-repeat-auto-010.html` | 0 / 4 |  |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat/intrinsic-auto-repeat/column-auto-repeat-auto-015.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat/intrinsic-auto-repeat/column-auto-repeat-auto-016.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat/intrinsic-auto-repeat/column-auto-repeat-auto-020.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat/intrinsic-auto-repeat/column-auto-repeat-auto-021.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat/intrinsic-auto-repeat/column-auto-repeat-auto-027.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat/intrinsic-auto-repeat/row-auto-repeat-auto-009.html` | 0 / 8 |  |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat/intrinsic-auto-repeat/row-auto-repeat-auto-010.html` | 0 / 4 |  |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat/intrinsic-auto-repeat/row-auto-repeat-auto-015.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat/intrinsic-auto-repeat/row-auto-repeat-auto-016.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat/intrinsic-auto-repeat/row-auto-repeat-auto-020.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat/intrinsic-auto-repeat/row-auto-repeat-auto-021.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat/row-auto-repeat-007.html` | 0 / 12 |  |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat/row-auto-repeat-008.html` | 0 / 4 |  |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat/row-auto-repeat-009.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat/row-auto-repeat-010.html` | 0 / 4 |  |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat/row-auto-repeat-011.html` | 0 / 4 |  |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat/row-auto-repeat-017.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat/row-auto-repeat-018.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat/row-auto-repeat-022.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/track-sizing/auto-repeat/row-auto-repeat-023.html` | 0 / 1 |  |
| `css/css-grid/grid-lanes/track-sizing/dynamic-grid-track-direction.html` | 0 / 1 |  |
| `css/css-grid/grid-layout-properties.html` | 35 / 140 |  |
| `css/css-grid/grid-model/compute-intrinsic-widths-scrollbar-001.html` | 0 / 2 |  |
| `css/css-grid/grid-model/grid-box-sizing-001.html` | 22 / 24 |  |
| `css/css-grid/grid-model/grid-computed-value-display-floated-items-001.html` | 2 / 6 |  |
| `css/css-grid/grid-model/grid-container-margin-border-padding-scrollbar-001.html` | 0 / 16 |  |
| `css/css-grid/grid-model/grid-container-padding-border-size-001.html` | 3 / 6 |  |
| `css/css-grid/grid-model/grid-container-sizing-constraints-001.html` | 0 / 15 |  |
| `css/css-grid/grid-model/grid-gutters-and-flex-content-001.html` | 6 / 8 |  |
| `css/css-grid/grid-model/grid-gutters-and-tracks-001.html` | 10 / 17 |  |
| `css/css-grid/grid-model/grid-gutters-as-percentage-001.html` | 6 / 16 |  |
| `css/css-grid/grid-model/grid-item-accepts-first-letter-001.html` | 0 / 2 |  |
| `css/css-grid/grid-model/grid-item-accepts-first-line-001.html` | 0 / 2 |  |
| `css/css-grid/grid-model/grid-item-hit-test.html` | 0 / 1 |  |
| `css/css-grid/grid-model/grid-min-max-width-001.html` | 10 / 20 |  |
| `css/css-grid/grid-model/grid-size-shrink-to-fit-001.html` | 3 / 4 |  |
| `css/css-grid/grid-model/grid-support-display-001.html` | 3 / 6 |  |
| `css/css-grid/grid-template-columns-mixed-calc.html` | 9 / 11 |  |
| `css/css-grid/implicit-grids/grid-auto-columns-rows-auto-flow-resolution-001.html` | 10 / 12 |  |
| `css/css-grid/implicit-grids/grid-auto-columns-rows-resolution-001.html` | 2 / 3 |  |
| `css/css-grid/implicit-grids/grid-implicit-columns-auto-resolution-001.html` | 2 / 3 |  |
| `css/css-grid/implicit-grids/grid-implicit-tracks-before-explicit-001.html` | 21 / 31 |  |
| `css/css-grid/inheritance.html` | 2 / 20 |  |
| `css/css-grid/layout-algorithm/automatic-minimum-size-clamped-to-fixed-max-track-sizing-function.html` | 3 / 13 |  |
| `css/css-grid/layout-algorithm/baseline-alignment-affects-intrinsic-size-001.html` | 0 / 48 |  |
| `css/css-grid/layout-algorithm/baseline-alignment-affects-intrinsic-size-002.html` | 0 / 48 |  |
| `css/css-grid/layout-algorithm/baseline-alignment-affects-intrinsic-size-003.html` | 0 / 24 |  |
| `css/css-grid/layout-algorithm/baseline-alignment-affects-intrinsic-size-004.html` | 0 / 24 |  |
| `css/css-grid/layout-algorithm/baseline-alignment-affects-intrinsic-size-005.html` | 0 / 24 |  |
| `css/css-grid/layout-algorithm/baseline-alignment-affects-intrinsic-size-006.html` | 0 / 24 |  |
| `css/css-grid/layout-algorithm/flex-and-intrinsic-sizes-001.html` | 3 / 8 |  |
| `css/css-grid/layout-algorithm/flex-sizing-columns-min-max-width-001.html` | 5 / 12 |  |
| `css/css-grid/layout-algorithm/flex-sizing-rows-min-max-height-001.html` | 0 / 12 |  |
| `css/css-grid/layout-algorithm/flex-sizing-rows-min-max-height-002.html` | 4 / 14 |  |
| `css/css-grid/layout-algorithm/flex-sizing-rows-min-max-height-border-box-001.html` | 0 / 6 |  |
| `css/css-grid/layout-algorithm/grid-auto-columns-content-resolution-001.html` | 2 / 3 |  |
| `css/css-grid/layout-algorithm/grid-auto-row-regrows-after-container-height-shrinks-001.html` | 0 / 1 |  |
| `css/css-grid/layout-algorithm/grid-automatic-minimum-for-auto-columns-001.html` | 1 / 4 |  |
| `css/css-grid/layout-algorithm/grid-automatic-minimum-for-auto-rows-001.html` | 2 / 4 |  |
| `css/css-grid/layout-algorithm/grid-container-percentage-001.html` | 4 / 20 |  |
| `css/css-grid/layout-algorithm/grid-container-percentage-002.html` | 5 / 24 |  |
| `css/css-grid/layout-algorithm/grid-content-distribution-must-account-for-track-sizing-002.html` | 0 / 1 |  |
| `css/css-grid/layout-algorithm/grid-find-fr-size-gutters-001.html` | 6 / 17 |  |
| `css/css-grid/layout-algorithm/grid-fit-content-percentage.html` | 0 / 20 |  |
| `css/css-grid/layout-algorithm/grid-flex-track-intrinsic-sizes-001.html` | 6 / 30 |  |
| `css/css-grid/layout-algorithm/grid-flex-track-intrinsic-sizes-002.html` | 0 / 5 |  |
| `css/css-grid/layout-algorithm/grid-flex-track-intrinsic-sizes-003.html` | 40 / 102 |  |
| `css/css-grid/layout-algorithm/grid-flex-tracks-item-with-border-001.html` | 1 / 3 |  |
| `css/css-grid/layout-algorithm/grid-intrinsic-size-with-orthogonal-items.html` | 0 / 7 |  |
| `css/css-grid/layout-algorithm/grid-intrinsic-track-sizes-001.html` | 27 / 31 |  |
| `css/css-grid/layout-algorithm/grid-intrinsic-width-minmax-tracks-001.html` | 6 / 35 |  |
| `css/css-grid/layout-algorithm/grid-item-margin-auto-columns-rows-001.html` | 0 / 4 |  |
| `css/css-grid/layout-algorithm/grid-item-margin-auto-columns-rows-vertical-lr-001.html` | 0 / 4 |  |
| `css/css-grid/layout-algorithm/grid-item-margin-auto-columns-rows-vertical-lr-002.html` | 0 / 2 |  |
| `css/css-grid/layout-algorithm/grid-item-margin-auto-columns-rows-vertical-rl-001.html` | 0 / 4 |  |
| `css/css-grid/layout-algorithm/grid-item-margin-auto-columns-rows-vertical-rl-002.html` | 0 / 2 |  |
| `css/css-grid/layout-algorithm/grid-item-positioning-orthogonal-flows-001.html` | 0 / 4 |  |
| `css/css-grid/layout-algorithm/grid-item-sizing-orthogonal-flows-001.html` | 1 / 4 |  |
| `css/css-grid/layout-algorithm/grid-item-stale-block-size-001.html` | 0 / 1 |  |
| `css/css-grid/layout-algorithm/grid-minimum-contribution-baseline-shim-vertical-lr.html` | 0 / 6 |  |
| `css/css-grid/layout-algorithm/grid-minimum-contribution-baseline-shim-vertical-rl.html` | 0 / 6 |  |
| `css/css-grid/layout-algorithm/grid-minimum-contribution-baseline-shim.html` | 0 / 6 |  |
| `css/css-grid/layout-algorithm/grid-minmax-content-tracks-multiple-items-001.html` | 3 / 4 |  |
| `css/css-grid/layout-algorithm/grid-minmax-spanning-items-columns-001.html` | 12 / 26 |  |
| `css/css-grid/layout-algorithm/grid-percentage-of-indefinite-auto-track-size-001.html` | 0 / 6 |  |
| `css/css-grid/layout-algorithm/grid-percentage-of-indefinite-track-size-001.html` | 0 / 6 |  |
| `css/css-grid/layout-algorithm/grid-percentage-tracks-container-size-001.html` | 4 / 12 |  |
| `css/css-grid/layout-algorithm/grid-percentage-tracks-intrinsic-container-001.html` | 0 / 1 |  |
| `css/css-grid/layout-algorithm/grid-relayout-orthogonal-items-same-result-001.html` | 0 / 3 |  |
| `css/css-grid/layout-algorithm/grid-spanning-items-orthogonal-flows-001.html` | 0 / 6 |  |
| `css/css-grid/layout-algorithm/grid-track-sizing-item-addition-001.html` | 22 / 63 |  |
| `css/css-grid/layout-algorithm/grid-track-sizing-item-removal-001.html` | 45 / 66 |  |
| `css/css-grid/parsing/grid-area-computed.html` | 0 / 35 |  |
| `css/css-grid/parsing/grid-area-invalid.html` | 0 / 25 |  |
| `css/css-grid/parsing/grid-area-shorthand.html` | 13 / 53 |  |
| `css/css-grid/parsing/grid-area-valid.html` | 29 / 60 |  |
| `css/css-grid/parsing/grid-auto-columns-computed.html` | 0 / 25 |  |
| `css/css-grid/parsing/grid-auto-columns-invalid.html` | 0 / 16 |  |
| `css/css-grid/parsing/grid-auto-columns-valid.html` | 29 / 30 |  |
| `css/css-grid/parsing/grid-auto-flow-computed.html` | 0 / 7 |  |
| `css/css-grid/parsing/grid-auto-flow-invalid.html` | 0 / 3 |  |
| `css/css-grid/parsing/grid-auto-flow-valid.html` | 4 / 7 |  |
| `css/css-grid/parsing/grid-auto-rows-computed.html` | 0 / 25 |  |
| `css/css-grid/parsing/grid-auto-rows-invalid.html` | 0 / 15 |  |
| `css/css-grid/parsing/grid-auto-rows-valid.html` | 29 / 30 |  |
| `css/css-grid/parsing/grid-column-invalid.html` | 0 / 31 |  |
| `css/css-grid/parsing/grid-column-shortest-serialization.html` | 0 / 16 |  |
| `css/css-grid/parsing/grid-column-shorthand.html` | 16 / 48 |  |
| `css/css-grid/parsing/grid-columns-rows-get-set-multiple.html` | 36 / 40 |  |
| `css/css-grid/parsing/grid-content-sized-columns-resolution.html` | 14 / 42 |  |
| `css/css-grid/parsing/grid-row-invalid.html` | 0 / 31 |  |
| `css/css-grid/parsing/grid-row-shortest-serialization.html` | 0 / 15 |  |
| `css/css-grid/parsing/grid-row-shorthand.html` | 16 / 48 |  |
| `css/css-grid/parsing/grid-shorthand-invalid.html` | 0 / 34 |  |
| `css/css-grid/parsing/grid-shorthand-serialization.html` | 51 / 141 |  |
| `css/css-grid/parsing/grid-shorthand-valid.html` | 32 / 49 |  |
| `css/css-grid/parsing/grid-shorthand.html` | 4 / 63 |  |
| `css/css-grid/parsing/grid-template-areas-computed.html` | 0 / 9 |  |
| `css/css-grid/parsing/grid-template-areas-invalid.html` | 0 / 11 |  |
| `css/css-grid/parsing/grid-template-areas-one-cell.html` | 2 / 6 |  |
| `css/css-grid/parsing/grid-template-areas-valid.html` | 6 / 9 |  |
| `css/css-grid/parsing/grid-template-columns-computed-implicit-track.html` | 5 / 24 |  |
| `css/css-grid/parsing/grid-template-columns-computed-nogrid.html` | 1 / 32 |  |
| `css/css-grid/parsing/grid-template-columns-computed-withcontent.html` | 21 / 30 |  |
| `css/css-grid/parsing/grid-template-columns-computed.html` | 5 / 25 |  |
| `css/css-grid/parsing/grid-template-columns-intrinsic-auto-repeat-computed-implicit-track.tentative.html` | 0 / 12 |  |
| `css/css-grid/parsing/grid-template-columns-intrinsic-auto-repeat-computed-withcontent.tentative.html` | 0 / 8 |  |
| `css/css-grid/parsing/grid-template-columns-intrinsic-auto-repeat-computed.tentative.html` | 0 / 12 |  |
| `css/css-grid/parsing/grid-template-columns-invalid.html` | 0 / 42 |  |
| `css/css-grid/parsing/grid-template-columns-valid.html` | 32 / 34 |  |
| `css/css-grid/parsing/grid-template-important.html` | 0 / 1 |  |
| `css/css-grid/parsing/grid-template-node-not-connected.html` | 0 / 1 |  |
| `css/css-grid/parsing/grid-template-repeat-auto-computed-withcontent-001.html` | 0 / 12 |  |
| `css/css-grid/parsing/grid-template-repeat-auto-computed-withcontent-002.html` | 0 / 17 |  |
| `css/css-grid/parsing/grid-template-rows-computed-implicit-track.html` | 5 / 24 |  |
| `css/css-grid/parsing/grid-template-rows-computed-nogrid.html` | 1 / 24 |  |
| `css/css-grid/parsing/grid-template-rows-computed-withcontent.html` | 21 / 27 |  |
| `css/css-grid/parsing/grid-template-rows-computed.html` | 5 / 24 |  |
| `css/css-grid/parsing/grid-template-rows-intrinsic-auto-repeat-computed-implicit-track.tentative.html` | 0 / 12 |  |
| `css/css-grid/parsing/grid-template-rows-intrinsic-auto-repeat-computed-withcontent.tentative.html` | 0 / 8 |  |
| `css/css-grid/parsing/grid-template-rows-intrinsic-auto-repeat-computed.tentative.html` | 0 / 12 |  |
| `css/css-grid/parsing/grid-template-rows-invalid.html` | 0 / 42 |  |
| `css/css-grid/parsing/grid-template-rows-valid.html` | 32 / 34 |  |
| `css/css-grid/parsing/grid-template-shorthand-areas-valid.html` | 0 / 6 |  |
| `css/css-grid/parsing/grid-template-shorthand-composition.html` | 4 / 11 |  |
| `css/css-grid/parsing/grid-template-shorthand-invalid.html` | 0 / 66 |  |
| `css/css-grid/parsing/grid-template-shorthand-valid.html` | 24 / 42 |  |
| `css/css-grid/parsing/grid-template-shorthand.html` | 3 / 28 |  |
| `css/css-grid/placement/grid-auto-flow-resolution-001.html` | 8 / 13 |  |
| `css/css-grid/placement/grid-auto-placement-automatic-span-001.html` | 12 / 16 |  |
| `css/css-grid/placement/grid-auto-placement-fixed-row-column-001.html` | 8 / 16 |  |
| `css/css-grid/placement/grid-auto-placement-item-addition-001.html` | 18 / 20 |  |
| `css/css-grid/placement/grid-auto-placement-item-removal-001.html` | 19 / 20 |  |
| `css/css-grid/placement/grid-container-change-named-grid-recompute-child-positions-001.html` | 0 / 4 |  |
| `css/css-grid/placement/grid-implicit-columns-for-items-locked-to-row-001.html` | 6 / 8 |  |
| `css/css-grid/placement/grid-placement-dynamic-change-001.html` | 67 / 88 |  |
| `css/css-grid/placement/grid-placement-huge-negative-indexes-001.html` | 0 / 3 |  |
| `css/css-grid/placement/grid-placement-named-grid-area-resolution-001.html` | 1 / 2 |  |
| `css/css-grid/placement/grid-placement-named-lines-and-areas-001.html` | 13 / 22 |  |
| `css/css-grid/placement/grid-placement-negative-integer-explicit-grid-001.html` | 3 / 4 |  |
| `css/css-grid/placement/grid-placement-negative-position-resolution-001.html` | 14 / 16 |  |
| `css/css-grid/placement/grid-placement-unknown-named-area-001.html` | 0 / 4 |  |
| `css/css-grid/placement/grid-placement-unknown-named-grid-line-001.html` | 0 / 17 |  |
| `css/css-grid/subgrid/align-self-baseline-with-subgrid-mbp.html` | 1 / 2 |  |
| `css/css-grid/subgrid/alignment-in-subgridded-axes-001.html` | 0 / 16 |  |
| `css/css-grid/subgrid/grid-template-computed-invalid-subgrid.html` | 0 / 14 |  |
| `css/css-grid/subgrid/grid-template-computed-nogrid.html` | 0 / 50 |  |
| `css/css-grid/subgrid/grid-template-invalid.html` | 0 / 22 |  |
| `css/css-grid/subgrid/nested-grid-subgrid-resize.html` | 0 / 1 |  |
| `css/css-grid/subgrid/placement-implicit-area-line-name-suffix.html` | 0 / 1 |  |
| `css/css-grid/subgrid/placement-invalidation-001.html` | 0 / 1 |  |
| `css/css-grid/subgrid/subgrid-baseline-005.html` | 2 / 12 |  |
| `css/css-grid/subgrid/subgrid-baseline-006.html` | 0 / 12 |  |
| `css/css-grid/subgrid/subgrid-baseline-007.html` | 2 / 12 |  |
| `css/css-grid/subgrid/subgrid-baseline-008.html` | 0 / 12 |  |
| `css/css-grid/subgrid/subgrid-baseline-009.html` | 0 / 12 |  |
| `css/css-grid/subgrid/subgrid-baseline-010.html` | 0 / 2 |  |
| `css/css-grid/subgrid/subgrid-baseline-011.html` | 1 / 2 |  |
| `css/css-grid/subgrid/subgrid-baseline-016.html` | 2 / 6 |  |
| `css/css-grid/subgrid/subgrid-baseline-017.html` | 3 / 8 |  |
| `css/css-grid/subgrid/subgrid-baseline-018.html` | 2 / 6 |  |
| `css/css-grid/subgrid/subgrid-item-aspect-ratio-001.html` | 0 / 3 |  |
| `css/css-grid/subgrid/subgridded-axis-content-distribution-001.html` | 0 / 8 |  |
| `css/css-grid/test-plan/index.html` | 0 / 0 | le harnais n'a pas terminé |
| `css/css-position/animations/bottom-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-position/animations/bottom-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-position/animations/left-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-position/animations/left-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-position/animations/position-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-position/animations/right-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-position/animations/right-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-position/animations/top-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-position/animations/top-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-position/backdrop-inherit-computed.html` | 0 / 0 | harnais : TypeError: undefined n'est pas une fonction |
| `css/css-position/backdrop-tree-abiding-slotted.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-position/backdrop-ua-style.html` | 0 / 1 |  |
| `css/css-position/out-of-flow-in-inline-content-intrinsic-size-001.html` | 1 / 17 |  |
| `css/css-position/overlay/animation/overlay-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-position/overlay/overlay-computed.html` | 0 / 6 |  |
| `css/css-position/overlay/overlay-invalid.html` | 0 / 6 |  |
| `css/css-position/overlay/overlay-transition-property.html` | 2 / 4 |  |
| `css/css-position/overlay/overlay-transition.html` | 0 / 2 |  |
| `css/css-position/overlay/overlay-user-agent-rules.html` | 0 / 4 |  |
| `css/css-position/parsing/bottom-computed.html` | 3 / 5 |  |
| `css/css-position/parsing/bottom-invalid.html` | 0 / 3 |  |
| `css/css-position/parsing/inset-computed.html` | 0 / 20 |  |
| `css/css-position/parsing/inset-invalid.html` | 0 / 22 |  |
| `css/css-position/parsing/inset-valid.html` | 40 / 50 |  |
| `css/css-position/parsing/left-computed.html` | 3 / 5 |  |
| `css/css-position/parsing/left-invalid.html` | 0 / 3 |  |
| `css/css-position/parsing/position-invalid.html` | 0 / 2 |  |
| `css/css-position/parsing/right-computed.html` | 3 / 5 |  |
| `css/css-position/parsing/right-invalid.html` | 0 / 3 |  |
| `css/css-position/parsing/top-computed.html` | 3 / 5 |  |
| `css/css-position/parsing/top-invalid.html` | 0 / 3 |  |
| `css/css-position/parsing/z-index-invalid.html` | 0 / 4 |  |
| `css/css-position/position-absolute-chrome-bug-002.html` | 0 / 1 |  |
| `css/css-position/position-absolute-crash-chrome-005.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `css/css-position/position-absolute-dynamic-containing-block.html` | 4 / 8 |  |
| `css/css-position/position-absolute-fit-content-auto-margin.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-position/position-absolute-in-inline-002.html` | 0 / 1 |  |
| `css/css-position/position-absolute-in-inline-dynamic.html` | 1 / 9 |  |
| `css/css-position/position-absolute-in-inline-vertical-lr.html` | 1 / 16 |  |
| `css/css-position/position-absolute-margin-auto-001.html` | 0 / 1 |  |
| `css/css-position/position-absolute-padding-percentage.html` | 1 / 2 |  |
| `css/css-position/position-absolute-percentage-height.html` | 0 / 2 |  |
| `css/css-position/position-absolute-replaced-minmax.html` | 11 / 43 |  |
| `css/css-position/position-absolute-table-001.html` | 2 / 6 |  |
| `css/css-position/position-absolute-with-negative-sized-imcb.html` | 4 / 30 |  |
| `css/css-position/position-fixed-dynamic-containing-block.html` | 0 / 4 |  |
| `css/css-position/position-relative-015.html` | 19 / 20 |  |
| `css/css-position/position-relative-016.html` | 16 / 18 |  |
| `css/css-position/position-relative-aspect-ratio-001.html` | 1 / 4 |  |
| `css/css-position/position-relative-aspect-ratio-002.html` | 0 / 3 |  |
| `css/css-position/position-sticky-dynamic-ancestor-001.html` | 0 / 1 |  |
| `css/css-position/sticky-dialog.html` | 0 / 1 |  |
| `css/css-position/sticky/position-sticky-block-in-sticky-inline.html` | 0 / 1 |  |
| `css/css-position/sticky/position-sticky-bottom.html` | 1 / 3 |  |
| `css/css-position/sticky/position-sticky-get-bounding-client-rect.html` | 0 / 3 |  |
| `css/css-position/sticky/position-sticky-horizontal-rtl-overconstrained.html` | 0 / 4 |  |
| `css/css-position/sticky/position-sticky-inflow-position.html` | 0 / 1 |  |
| `css/css-position/sticky/position-sticky-inline-block-in-sticky-inline.html` | 0 / 2 |  |
| `css/css-position/sticky/position-sticky-left-and-right-overconstrained.html` | 0 / 4 |  |
| `css/css-position/sticky/position-sticky-left.html` | 1 / 3 |  |
| `css/css-position/sticky/position-sticky-margins.html` | 1 / 3 |  |
| `css/css-position/sticky/position-sticky-nested-bottom.html` | 2 / 5 |  |
| `css/css-position/sticky/position-sticky-nested-left.html` | 1 / 5 |  |
| `css/css-position/sticky/position-sticky-nested-right.html` | 2 / 5 |  |
| `css/css-position/sticky/position-sticky-nested-top.html` | 1 / 5 |  |
| `css/css-position/sticky/position-sticky-offset-overflow.html` | 0 / 1 |  |
| `css/css-position/sticky/position-sticky-offset-top-left.html` | 0 / 2 |  |
| `css/css-position/sticky/position-sticky-overflow-padding.html` | 1 / 3 |  |
| `css/css-position/sticky/position-sticky-right.html` | 1 / 3 |  |
| `css/css-position/sticky/position-sticky-root-scroller.html` | 0 / 1 |  |
| `css/css-position/sticky/position-sticky-scrollIntoView.html` | 0 / 1 |  |
| `css/css-position/sticky/position-sticky-single-axis-basic.html` | 0 / 2 |  |
| `css/css-position/sticky/position-sticky-single-axis-dynamic.html` | 0 / 1 |  |
| `css/css-position/sticky/position-sticky-single-axis-nested.html` | 0 / 1 |  |
| `css/css-position/sticky/position-sticky-single-axis-used-values.html` | 0 / 1 |  |
| `css/css-position/sticky/position-sticky-top-and-bottom-overconstrained.html` | 0 / 4 |  |
| `css/css-position/sticky/position-sticky-top-and-bottom.html` | 0 / 2 |  |
| `css/css-position/sticky/position-sticky-top.html` | 1 / 3 |  |
| `css/css-position/sticky/position-sticky-transforms-translate.html` | 0 / 3 |  |
| `css/css-position/sticky/position-sticky-transforms.html` | 0 / 3 |  |
| `css/css-position/sticky/sticky-after-input.html` | 0 / 1 |  |
| `css/css-sizing/abspos-intrinsic-height-inset-percentage-child.html` | 3 / 6 |  |
| `css/css-sizing/animation/aspect-ratio-interpolation-degenerate.html` | 0 / 12 |  |
| `css/css-sizing/animation/aspect-ratio-interpolation-sideways-lr.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-sizing/animation/aspect-ratio-interpolation-sideways-rl.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-sizing/animation/aspect-ratio-interpolation-vertical-lr.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-sizing/animation/aspect-ratio-interpolation-vertical-rl.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-sizing/animation/aspect-ratio-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-sizing/animation/box-sizing-no-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-sizing/animation/height-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-sizing/animation/height-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-sizing/animation/height-no-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-sizing/animation/max-height-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-sizing/animation/max-height-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-sizing/animation/max-width-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-sizing/animation/max-width-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-sizing/animation/min-height-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-sizing/animation/min-height-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-sizing/animation/min-width-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-sizing/animation/min-width-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-sizing/animation/width-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-sizing/animation/width-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-sizing/aspect-ratio-affects-container-width-when-height-changes.html` | 1 / 2 |  |
| `css/css-sizing/aspect-ratio-automatic-minimum.html` | 1 / 2 |  |
| `css/css-sizing/aspect-ratio.html` | 5 / 11 |  |
| `css/css-sizing/aspect-ratio/abspos-aspect-ratio-border.html` | 0 / 6 |  |
| `css/css-sizing/aspect-ratio/abspos-intrinsic-block-size.html` | 4 / 32 |  |
| `css/css-sizing/aspect-ratio/aspect-ratio-minimum-size-single-axis-scroll-container.html` | 2 / 4 |  |
| `css/css-sizing/aspect-ratio/box-sizing-aspect-ratio-positioned.html` | 0 / 1 |  |
| `css/css-sizing/aspect-ratio/box-sizing-dimensions.html` | 2 / 8 |  |
| `css/css-sizing/aspect-ratio/box-sizing-squashed.html` | 3 / 16 |  |
| `css/css-sizing/aspect-ratio/flex-aspect-ratio-orthogonal-percentage-child.html` | 0 / 4 |  |
| `css/css-sizing/aspect-ratio/flex-aspect-ratio-row-percentage-height-child.html` | 0 / 1 |  |
| `css/css-sizing/aspect-ratio/fractional-aspect-ratio.html` | 0 / 2 |  |
| `css/css-sizing/aspect-ratio/inheritance.html` | 0 / 2 |  |
| `css/css-sizing/aspect-ratio/intrinsic-contribution-transferred-min-width-001.html` | 0 / 3 |  |
| `css/css-sizing/aspect-ratio/parsing/aspect-ratio-computed.html` | 0 / 13 |  |
| `css/css-sizing/aspect-ratio/parsing/aspect-ratio-invalid.html` | 0 / 11 |  |
| `css/css-sizing/aspect-ratio/parsing/aspect-ratio-valid.html` | 7 / 14 |  |
| `css/css-sizing/aspect-ratio/quirks-mode-001.html` | 0 / 1 |  |
| `css/css-sizing/aspect-ratio/quirks-mode-002.html` | 0 / 1 |  |
| `css/css-sizing/aspect-ratio/quirks-mode-003.html` | 0 / 1 |  |
| `css/css-sizing/aspect-ratio/replaced-element-036.html` | 0 / 2 |  |
| `css/css-sizing/aspect-ratio/replaced-element-042.html` | 0 / 4 |  |
| `css/css-sizing/aspect-ratio/replaced-element-043.html` | 38 / 180 |  |
| `css/css-sizing/aspect-ratio/replaced-element-044.html` | 0 / 4 |  |
| `css/css-sizing/aspect-ratio/replaced-element-intrinsic-contribution-transferred-min-width-001.html` | 0 / 3 |  |
| `css/css-sizing/aspect-ratio/sign-function-aspect-ratio.html` | 0 / 1 |  |
| `css/css-sizing/aspect-ratio/table-element-002.html` | 1 / 5 |  |
| `css/css-sizing/aspect-ratio/table-element-intrinsic-contribution-001.html` | 0 / 1 |  |
| `css/css-sizing/available-height-for-replaced-content-001.html` | 0 / 1 |  |
| `css/css-sizing/button-min-width.html` | 0 / 1 |  |
| `css/css-sizing/contain-intrinsic-size/animation/contain-intrinsic-size-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-sizing/contain-intrinsic-size/auto-001.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `css/css-sizing/contain-intrinsic-size/auto-002.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `css/css-sizing/contain-intrinsic-size/auto-003.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `css/css-sizing/contain-intrinsic-size/auto-004.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `css/css-sizing/contain-intrinsic-size/auto-005.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `css/css-sizing/contain-intrinsic-size/auto-006.html` | 0 / 19 | harnais : Test named 'Basic usage' specified 1 'cleanup' function, and 1 failed. |
| `css/css-sizing/contain-intrinsic-size/auto-007.html` | 0 / 0 | boucle sans fin (processus tué) |
| `css/css-sizing/contain-intrinsic-size/auto-008.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `css/css-sizing/contain-intrinsic-size/auto-009.html` | 0 / 2 |  |
| `css/css-sizing/contain-intrinsic-size/auto-010.html` | 0 / 2 |  |
| `css/css-sizing/contain-intrinsic-size/auto-011.html` | 0 / 2 |  |
| `css/css-sizing/contain-intrinsic-size/auto-012.html` | 0 / 7 |  |
| `css/css-sizing/contain-intrinsic-size/auto-013.html` | 0 / 1 |  |
| `css/css-sizing/contain-intrinsic-size/auto-014.html` | 0 / 2 | harnais : Test named 'Basic usage' specified 1 'cleanup' function, and 1 failed. |
| `css/css-sizing/contain-intrinsic-size/auto-015.html` | 0 / 1 | harnais : Test named 'Basic usage' specified 1 'cleanup' function, and 1 failed. |
| `css/css-sizing/contain-intrinsic-size/auto-016.html` | 0 / 1 | harnais : Test named 'Basic usage' specified 1 'cleanup' function, and 1 failed. |
| `css/css-sizing/contain-intrinsic-size/auto-017.html` | 0 / 1 | harnais : Test named 'Basic usage' specified 1 'cleanup' function, and 1 failed. |
| `css/css-sizing/contain-intrinsic-size/auto-018.html` | 0 / 1 | harnais : Test named 'Basic usage' specified 1 'cleanup' function, and 1 failed. |
| `css/css-sizing/contain-intrinsic-size/contain-intrinsic-size-009.html` | 0 / 7 |  |
| `css/css-sizing/contain-intrinsic-size/contain-intrinsic-size-028.html` | 0 / 48 |  |
| `css/css-sizing/contain-intrinsic-size/contain-intrinsic-size-029.html` | 1 / 8 |  |
| `css/css-sizing/contain-intrinsic-size/contain-intrinsic-size-030.html` | 0 / 24 |  |
| `css/css-sizing/contain-intrinsic-size/contain-intrinsic-size-031.html` | 0 / 12 |  |
| `css/css-sizing/contain-intrinsic-size/contain-intrinsic-size-032.html` | 1 / 9 |  |
| `css/css-sizing/contain-intrinsic-size/contain-intrinsic-size-logical-003.html` | 4 / 48 |  |
| `css/css-sizing/contain-intrinsic-size/forget-on-disconnect-in-iframe.html` | 0 / 2 | harnais : 1 duplicate test name: "Forget remembered size inside iframe" |
| `css/css-sizing/contain-intrinsic-size/parsing/contain-intrinsic-size-computed.html` | 0 / 21 |  |
| `css/css-sizing/contain-intrinsic-size/parsing/contain-intrinsic-size-invalid.html` | 0 / 40 |  |
| `css/css-sizing/contain-intrinsic-size/parsing/contain-intrinsic-size-valid.html` | 24 / 27 |  |
| `css/css-sizing/fieldset-intrinsic-size-percent.html` | 0 / 12 |  |
| `css/css-sizing/fit-content-percentage-padding.html` | 0 / 3 |  |
| `css/css-sizing/form-control-in-inline-content-intrinsic-size-001.html` | 0 / 9 |  |
| `css/css-sizing/image-min-max-content-intrinsic-size-change-009.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `css/css-sizing/intrinsic-percent-replaced-033.html` | 0 / 9 |  |
| `css/css-sizing/intrinsic-size-fallback-replaced.html` | 4 / 8 |  |
| `css/css-sizing/intrinsic-size-fallback-video.html` | 0 / 4 |  |
| `css/css-sizing/keyword-min-max-width-with-fixed-width-contribution.html` | 0 / 3 |  |
| `css/css-sizing/keyword-sizes-for-intrinsic-contributions-002.html` | 0 / 8 |  |
| `css/css-sizing/keyword-sizes-for-intrinsic-contributions.html` | 5 / 36 |  |
| `css/css-sizing/keyword-sizes-on-abspos.html` | 9 / 60 |  |
| `css/css-sizing/keyword-sizes-on-flex-item-001.html` | 7 / 74 |  |
| `css/css-sizing/keyword-sizes-on-flex-item-002.html` | 19 / 74 |  |
| `css/css-sizing/keyword-sizes-on-floated-element.html` | 19 / 74 |  |
| `css/css-sizing/keyword-sizes-on-inline-block.html` | 20 / 74 |  |
| `css/css-sizing/keyword-sizes-on-replaced-element.html` | 39 / 84 |  |
| `css/css-sizing/min-max-width-contribution-box-sizing.html` | 0 / 4 |  |
| `css/css-sizing/min-width-min-content-contribution.html` | 2 / 6 |  |
| `css/css-sizing/parsing/box-sizing-invalid.html` | 0 / 7 |  |
| `css/css-sizing/parsing/height-invalid.html` | 0 / 4 |  |
| `css/css-sizing/parsing/height-valid.html` | 9 / 10 |  |
| `css/css-sizing/parsing/max-height-computed.html` | 3 / 12 |  |
| `css/css-sizing/parsing/max-height-invalid.html` | 0 / 4 |  |
| `css/css-sizing/parsing/max-height-valid.html` | 9 / 10 |  |
| `css/css-sizing/parsing/max-size-invalid.html` | 0 / 10 |  |
| `css/css-sizing/parsing/max-size-shorthand.html` | 0 / 15 |  |
| `css/css-sizing/parsing/max-size-valid.html` | 12 / 14 |  |
| `css/css-sizing/parsing/max-width-computed.html` | 3 / 12 |  |
| `css/css-sizing/parsing/max-width-invalid.html` | 0 / 4 |  |
| `css/css-sizing/parsing/max-width-valid.html` | 9 / 10 |  |
| `css/css-sizing/parsing/min-height-computed.html` | 2 / 11 |  |
| `css/css-sizing/parsing/min-height-invalid.html` | 0 / 11 |  |
| `css/css-sizing/parsing/min-height-valid.html` | 9 / 10 |  |
| `css/css-sizing/parsing/min-size-invalid.html` | 0 / 10 |  |
| `css/css-sizing/parsing/min-size-shorthand.html` | 0 / 15 |  |
| `css/css-sizing/parsing/min-size-valid.html` | 12 / 14 |  |
| `css/css-sizing/parsing/min-width-computed.html` | 2 / 11 |  |
| `css/css-sizing/parsing/min-width-invalid.html` | 0 / 11 |  |
| `css/css-sizing/parsing/min-width-valid.html` | 9 / 10 |  |
| `css/css-sizing/parsing/size-invalid.html` | 0 / 6 |  |
| `css/css-sizing/parsing/size-shorthand.html` | 0 / 12 |  |
| `css/css-sizing/parsing/size-valid.html` | 9 / 10 |  |
| `css/css-sizing/parsing/width-invalid.html` | 0 / 4 |  |
| `css/css-sizing/parsing/width-valid.html` | 9 / 10 |  |
| `css/css-sizing/percentage-height-in-flexbox.html` | 0 / 2 |  |
| `css/css-sizing/percentage-height-replaced-content-in-auto-cb.html` | 0 / 1 |  |
| `css/css-sizing/percentage-min-width.html` | 0 / 1 |  |
| `css/css-sizing/replaced-element-max-content-contribution-transferred-size.html` | 0 / 5 |  |
| `css/css-sizing/replaced-element-transferred-size-flex.html` | 0 / 3 |  |
| `css/css-sizing/replaced-fractional-height-from-aspect-ratio-2.html` | 0 / 1 |  |
| `css/css-sizing/replaced-keyword-max-width-contribution.html` | 0 / 3 |  |
| `css/css-sizing/responsive-iframe/parsing/frame-sizing-computed.html` | 0 / 5 |  |
| `css/css-sizing/responsive-iframe/responsive-iframe-dom-content-loaded.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `css/css-sizing/responsive-iframe/responsive-iframe-icb.html` | 0 / 1 |  |
| `css/css-sizing/responsive-iframe/responsive-iframe-navigation-unsized.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `css/css-sizing/responsive-iframe/responsive-iframe-navigation.html` | 0 / 1 |  |
| `css/css-sizing/responsive-iframe/responsive-iframe-request-resize-error.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `css/css-sizing/responsive-iframe/responsive-iframe-value.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-sizing/size-shorthand-serialization.tentative.html` | 3 / 15 |  |
| `css/css-sizing/stretch/aspect-ratio-1.html` | 0 / 4 |  |
| `css/css-sizing/stretch/aspect-ratio-2.html` | 1 / 18 |  |
| `css/css-sizing/stretch/auto-margins-1.html` | 1 / 2 |  |
| `css/css-sizing/stretch/block-height-001.html` | 9 / 18 |  |
| `css/css-sizing/stretch/block-height-002.html` | 1 / 4 |  |
| `css/css-sizing/stretch/block-height-004.html` | 0 / 8 |  |
| `css/css-sizing/stretch/block-height-005.html` | 0 / 4 |  |
| `css/css-sizing/stretch/block-height-007.html` | 5 / 20 |  |
| `css/css-sizing/stretch/block-height-008.html` | 0 / 160 |  |
| `css/css-sizing/stretch/block-height-011.html` | 0 / 8 |  |
| `css/css-sizing/stretch/content-contribution-001.html` | 3 / 5 |  |
| `css/css-sizing/stretch/flex-basis-1.html` | 1 / 5 |  |
| `css/css-sizing/stretch/flex-item-height-001.html` | 0 / 2 |  |
| `css/css-sizing/stretch/flex-item-height-002.html` | 0 / 1 |  |
| `css/css-sizing/stretch/flex-item-height-003.html` | 0 / 1 |  |
| `css/css-sizing/stretch/flex-line-003.html` | 1 / 4 |  |
| `css/css-sizing/stretch/flex-line-004.html` | 3 / 4 |  |
| `css/css-sizing/stretch/flex-stretch-min-max-cross-001.html` | 0 / 1 |  |
| `css/css-sizing/stretch/indefinite-4.html` | 0 / 1 |  |
| `css/css-sizing/stretch/parsing.html` | 13 / 17 |  |
| `css/css-sizing/stretch/replaced-stretch-height-001.html` | 0 / 1 |  |
| `css/css-sizing/stretch/stretch-alias-block-size-001.tentative.html` | 10 / 46 |  |
| `css/css-sizing/stretch/stretch-alias-inline-size-001.tentative.html` | 10 / 46 |  |
| `css/css-sizing/stretch/stretch-alias-max-block-size-001.tentative.html` | 10 / 45 |  |
| `css/css-sizing/stretch/stretch-alias-max-inline-size-001.tentative.html` | 10 / 45 |  |
| `css/css-sizing/stretch/stretch-alias-min-block-size-001.tentative.html` | 10 / 46 |  |
| `css/css-sizing/stretch/stretch-alias-min-inline-size-001.tentative.html` | 10 / 46 |  |
| `css/css-sizing/stretch/stretch-block-size-001.html` | 10 / 46 |  |
| `css/css-sizing/stretch/stretch-block-size-002.html` | 0 / 36 |  |
| `css/css-sizing/stretch/stretch-block-size-003.html` | 0 / 36 |  |
| `css/css-sizing/stretch/stretch-inline-size-001.html` | 10 / 46 |  |
| `css/css-sizing/stretch/stretch-inline-size-002.html` | 0 / 36 |  |
| `css/css-sizing/stretch/stretch-inline-size-003.html` | 0 / 36 |  |
| `css/css-sizing/stretch/stretch-max-block-size-001.html` | 10 / 45 |  |
| `css/css-sizing/stretch/stretch-max-inline-size-001.html` | 10 / 45 |  |
| `css/css-sizing/stretch/stretch-min-block-size-001.html` | 10 / 46 |  |
| `css/css-sizing/stretch/stretch-min-inline-size-001.html` | 10 / 46 |  |
| `css/css-sizing/stretch/stretch-table-001.html` | 0 / 7 |  |
| `css/css-sizing/svg-intrinsic-size-001.html` | 0 / 5 |  |
| `css/css-sizing/svg-intrinsic-size-002.html` | 0 / 5 |  |
| `css/css-sizing/svg-intrinsic-size-003.html` | 0 / 5 |  |
| `css/css-sizing/svg-intrinsic-size-004.html` | 1 / 5 |  |
| `css/css-sizing/svg-intrinsic-size-008.html` | 0 / 5 |  |
| `css/css-sizing/svg-intrinsic-size-009.html` | 0 / 12 |  |
| `css/css-sizing/svg-intrinsic-size-010.html` | 0 / 12 |  |
| `css/css-sizing/svg-intrinsic-size-011.html` | 0 / 4 |  |
| `css/css-sizing/svg-intrinsic-size-zero-img-and-object.html` | 0 / 14 | harnais : délai du harnais dépassé |
| `css/css-sizing/table-cell-definite-height-transferred-size.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-tables/absolute-tables-001.html` | 0 / 8 |  |
| `css/css-tables/absolute-tables-002.html` | 0 / 4 |  |
| `css/css-tables/absolute-tables-003.html` | 0 / 4 |  |
| `css/css-tables/absolute-tables-004.html` | 0 / 4 |  |
| `css/css-tables/absolute-tables-005.html` | 0 / 4 |  |
| `css/css-tables/absolute-tables-height-001.html` | 10 / 11 |  |
| `css/css-tables/absolute-tables-height-quirks-001.html` | 0 / 1 |  |
| `css/css-tables/animations/border-spacing-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-tables/border-spacing-included-in-sizes-001.html` | 2 / 4 |  |
| `css/css-tables/border-writing-mode-dynamic-001.html` | 0 / 1 |  |
| `css/css-tables/bounding-box-computation-1.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-tables/bounding-box-computation-2.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-tables/bounding-box-computation-3.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-tables/caption-orthogonal-margin-in-table-block-axis.html` | 0 / 2 |  |
| `css/css-tables/caption-side-1.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-tables/caption-writing-mode-001.html` | 0 / 2 |  |
| `css/css-tables/chrome-rowspan-bug.html` | 0 / 1 |  |
| `css/css-tables/col_removal.html` | 0 / 1 |  |
| `css/css-tables/colspan-001.html` | 0 / 5 |  |
| `css/css-tables/colspan-002.html` | 0 / 5 |  |
| `css/css-tables/colspan-003.html` | 0 / 5 |  |
| `css/css-tables/column-track-merging.html` | 0 / 13 |  |
| `css/css-tables/dynamic-rowspan-change.html` | 0 / 2 |  |
| `css/css-tables/fixed-layout-1.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-tables/fixed-layout-2.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-tables/fixed-layout-excess-width-distribution-001.html` | 0 / 1 |  |
| `css/css-tables/fractional-percent-width.html` | 0 / 3 |  |
| `css/css-tables/height-distribution/computing-row-measure-0.html` | 2 / 4 |  |
| `css/css-tables/height-distribution/computing-row-measure-1.html` | 0 / 4 |  |
| `css/css-tables/height-distribution/extra-height-given-to-all-row-groups-003.html` | 0 / 1 |  |
| `css/css-tables/height-distribution/extra-height-given-to-all-row-groups-004.html` | 0 / 1 |  |
| `css/css-tables/height-distribution/percentage-sizing-of-table-cell-children.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-tables/html-to-css-mapping-1.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-tables/html-to-css-mapping-2.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-tables/html5-table-formatting-1.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-tables/html5-table-formatting-2.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-tables/html5-table-formatting-3.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-tables/html5-table-formatting-fixed-layout-1.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-tables/inheritance.html` | 0 / 10 |  |
| `css/css-tables/no-overflow-with-table-cell-margins.html` | 0 / 1 |  |
| `css/css-tables/parsing/border-collapse-computed.html` | 0 / 2 |  |
| `css/css-tables/parsing/border-collapse-invalid.html` | 0 / 2 |  |
| `css/css-tables/parsing/border-spacing-computed.html` | 0 / 6 |  |
| `css/css-tables/parsing/border-spacing-invalid.html` | 0 / 4 |  |
| `css/css-tables/parsing/border-spacing-valid.html` | 3 / 4 |  |
| `css/css-tables/parsing/caption-side-computed.html` | 0 / 2 |  |
| `css/css-tables/parsing/caption-side-invalid.html` | 0 / 4 |  |
| `css/css-tables/parsing/empty-cells-computed.html` | 0 / 2 |  |
| `css/css-tables/parsing/empty-cells-invalid.html` | 0 / 2 |  |
| `css/css-tables/parsing/table-layout-computed.html` | 0 / 2 |  |
| `css/css-tables/parsing/table-layout-invalid.html` | 0 / 2 |  |
| `css/css-tables/percent-width-ignored-001.tentative.html` | 0 / 1 |  |
| `css/css-tables/percent-width-ignored-002.tentative.html` | 0 / 1 |  |
| `css/css-tables/percent-width-ignored-003.tentative.html` | 0 / 1 |  |
| `css/css-tables/table-cell-writing-mode-computed.html` | 0 / 1 |  |
| `css/css-tables/table-height-distribution-proportional.html` | 1 / 2 |  |
| `css/css-tables/table-min-max-width-intrinsic-contribution-border-padding.html` | 0 / 5 |  |
| `css/css-tables/table-min-max-width-intrinsic-contribution.html` | 0 / 4 |  |
| `css/css-tables/table-model-fixup-2.html` | 20 / 40 |  |
| `css/css-tables/table-model-fixup.html` | 5 / 11 |  |
| `css/css-tables/table-stretch-in-flex-and-grid.html` | 5 / 6 |  |
| `css/css-tables/tentative/baseline-table.html` | 3 / 15 |  |
| `css/css-tables/tentative/baseline-td.html` | 1 / 5 |  |
| `css/css-tables/tentative/caption.html` | 6 / 13 |  |
| `css/css-tables/tentative/colgroup-col.html` | 0 / 6 |  |
| `css/css-tables/tentative/colspan-redistribution.html` | 0 / 31 |  |
| `css/css-tables/tentative/column-widths.html` | 1 / 33 |  |
| `css/css-tables/tentative/element-sizing.html` | 0 / 2 |  |
| `css/css-tables/tentative/position-sticky-container.html` | 0 / 3 |  |
| `css/css-tables/tentative/rowspan-height-redistribution.html` | 2 / 24 |  |
| `css/css-tables/tentative/table-fixed-minmax.html` | 4 / 8 |  |
| `css/css-tables/tentative/table-height-redistribution.html` | 4 / 31 |  |
| `css/css-tables/tentative/table-limited-quirks.html` | 2 / 7 |  |
| `css/css-tables/tentative/table-minmax.html` | 3 / 16 |  |
| `css/css-tables/tentative/table-quirks.html` | 1 / 7 |  |
| `css/css-tables/tentative/table-rows-with-zero-columns.html` | 0 / 12 |  |
| `css/css-tables/tentative/table-width-redistribution-fixed-padding.html` | 0 / 15 |  |
| `css/css-tables/tentative/table-width-redistribution-fixed.html` | 1 / 26 |  |
| `css/css-tables/tentative/table-width-redistribution.html` | 0 / 22 |  |
| `css/css-tables/tentative/tbody-height-redistribution.html` | 1 / 12 |  |
| `css/css-tables/tentative/td-box-sizing-001.html` | 0 / 14 |  |
| `css/css-tables/tentative/td-box-sizing-002.html` | 11 / 15 |  |
| `css/css-tables/tentative/td-box-sizing-003.html` | 1 / 11 |  |
| `css/css-tables/visibility-collapse-col-001.html` | 1 / 2 |  |
| `css/css-tables/visibility-collapse-col-002.html` | 1 / 2 |  |
| `css/css-tables/visibility-collapse-col-003.html` | 1 / 2 |  |
| `css/css-tables/visibility-collapse-col-004-dynamic.html` | 1 / 2 |  |
| `css/css-tables/visibility-collapse-col-005.html` | 1 / 2 |  |
| `css/css-tables/visibility-collapse-colspan-001.html` | 1 / 2 |  |
| `css/css-tables/visibility-collapse-colspan-002.html` | 1 / 2 |  |
| `css/css-tables/visibility-collapse-row-001.html` | 1 / 2 |  |
| `css/css-tables/visibility-collapse-row-002-dynamic.html` | 1 / 2 |  |
| `css/css-tables/visibility-collapse-row-003-dynamic.html` | 1 / 2 |  |
| `css/css-tables/visibility-collapse-row-004.html` | 0 / 1 |  |
| `css/css-tables/visibility-collapse-row-005.html` | 0 / 2 |  |
| `css/css-tables/visibility-collapse-row-group-001.html` | 2 / 5 |  |
| `css/css-tables/visibility-collapse-row-group-002.html` | 1 / 2 |  |
| `css/css-tables/visibility-collapse-rowcol-001.html` | 0 / 2 |  |
| `css/css-tables/visibility-collapse-rowcol-002.html` | 0 / 2 |  |
| `css/css-tables/visibility-collapse-rowspan-002-border-separate.html` | 4 / 6 |  |
| `css/css-tables/visibility-collapse-rowspan-002.html` | 4 / 6 |  |
| `css/css-tables/visibility-collapse-rowspan-003-border-separate.html` | 5 / 6 |  |
| `css/css-tables/visibility-collapse-rowspan-003.html` | 5 / 6 |  |
| `css/css-tables/visibility-collapse-rowspan-004-dynamic.html` | 14 / 18 |  |
| `css/css-tables/visibility-hidden-col-001.html` | 1 / 2 |  |
| `css/css-tables/visibility-hidden-nested-001.html` | 7 / 8 |  |
| `css/css-tables/visibility-hidden-row-001.html` | 2 / 3 |  |
| `css/css-tables/visibility-hidden-row-002.html` | 2 / 3 |  |
| `css/css-tables/width-distribution/computing-column-measure-0.html` | 0 / 4 |  |
| `css/css-tables/width-distribution/computing-column-measure-1.html` | 0 / 4 |  |
| `css/css-tables/width-distribution/computing-column-measure-2.html` | 0 / 1 |  |
| `css/css-tables/width-distribution/computing-table-width-0.html` | 0 / 1 |  |
| `css/css-tables/width-distribution/computing-table-width-1.html` | 0 / 1 |  |
| `css/css-tables/width-distribution/distribution-algo-1.html` | 0 / 3 |  |
| `css/css-tables/width-distribution/distribution-algo-2.html` | 0 / 3 |  |
| `css/css-tables/width-distribution/distribution-algo-min-content-guess.html` | 0 / 3 |  |
| `css/css-tables/width-distribution/distribution-algo-min-content-specified-guess.1.html` | 0 / 1 |  |
| `css/css-tables/width-distribution/distribution-algo-min-content-specified-guess.html` | 0 / 1 |  |
| `css/css-tables/width-distribution/td-max-width-auto-layout.html` | 3 / 4 |  |
| `css/css-tables/width-distribution/td-min-width-auto-layout.html` | 0 / 4 |  |
| `css/css-tables/width-distribution/td-with-subpixel-padding-vertical-rl.html` | 0 / 1 |  |
| `css/cssom-view/CaretPosition-001.html` | 0 / 2 |  |
| `css/cssom-view/DOMRectList.html` | 0 / 2 |  |
| `css/cssom-view/Element-currentCSSZoom.html` | 0 / 2 |  |
| `css/cssom-view/GetBoundingRect.html` | 0 / 1 |  |
| `css/cssom-view/HTMLBody-ScrollArea_quirksmode.html` | 3 / 10 |  |
| `css/cssom-view/MediaQueryList-addListener-handleEvent.html` | 0 / 6 |  |
| `css/cssom-view/MediaQueryList-addListener-removeListener.html` | 1 / 8 |  |
| `css/cssom-view/MediaQueryList-change-event-matches-value.html` | 0 / 1 |  |
| `css/cssom-view/MediaQueryList-extends-EventTarget-interop.html` | 1 / 8 |  |
| `css/cssom-view/MediaQueryList-extends-EventTarget.html` | 1 / 7 |  |
| `css/cssom-view/MediaQueryListEvent.html` | 0 / 6 |  |
| `css/cssom-view/background-change-during-smooth-scroll.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `css/cssom-view/checkVisibility.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/cssom-view/client-props-input.html` | 0 / 1 |  |
| `css/cssom-view/client-scroll-props-input.html` | 0 / 2 |  |
| `css/cssom-view/cssom-geometryutils-connected-callback.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/cssom-view/cssom-geometryutils-convertPointFromNode.html` | 0 / 20 |  |
| `css/cssom-view/cssom-geometryutils-convertQuadFromNode.html` | 0 / 18 |  |
| `css/cssom-view/cssom-geometryutils-convertRectFromNode.html` | 0 / 18 |  |
| `css/cssom-view/cssom-geometryutils-cross-origin-path.https.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `css/cssom-view/cssom-geometryutils-element-types.html` | 1 / 41 |  |
| `css/cssom-view/cssom-geometryutils-fragmentation.html` | 0 / 4 |  |
| `css/cssom-view/cssom-geometryutils-getBoxQuads-options.html` | 0 / 17 |  |
| `css/cssom-view/cssom-geometryutils-lifecycle.html` | 0 / 4 |  |
| `css/cssom-view/cssom-geometryutils-same-origin-iframe.html` | 0 / 2 |  |
| `css/cssom-view/cssom-geometryutils-shadow-dom.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/cssom-view/cssom-geometryutils-text.html` | 0 / 2 |  |
| `css/cssom-view/cssom-geometryutils-transforms.html` | 0 / 10 |  |
| `css/cssom-view/cssom-getBoundingClientRect-003.html` | 0 / 2 |  |
| `css/cssom-view/cssom-getBoxQuads-001.html` | 0 / 6 |  |
| `css/cssom-view/cssom-getBoxQuads-002.html` | 0 / 1 |  |
| `css/cssom-view/cssom-view-img-attributes-001.html` | 0 / 4 |  |
| `css/cssom-view/dom-element-scroll.html` | 0 / 4 |  |
| `css/cssom-view/element-scroll-arguments.html` | 0 / 12 |  |
| `css/cssom-view/element-scroll-promise-after-removal.html` | 0 / 5 | harnais : TypeError: undefined n'est pas une fonction |
| `css/cssom-view/element-scroll-promise-interruption.html` | 0 / 14 | harnais : TypeError: undefined n'est pas une fonction |
| `css/cssom-view/element-scroll-promises.html` | 0 / 24 | harnais : TypeError: undefined n'est pas une fonction |
| `css/cssom-view/elementFromPoint-001.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/cssom-view/elementFromPoint-002.html` | 0 / 1 |  |
| `css/cssom-view/elementFromPoint-003.html` | 0 / 1 |  |
| `css/cssom-view/elementFromPoint-dynamic-anon-box.html` | 0 / 1 |  |
| `css/cssom-view/elementFromPoint-ellipsis-in-inline-box.html` | 0 / 3 |  |
| `css/cssom-view/elementFromPoint-float-in-relative.html` | 0 / 1 |  |
| `css/cssom-view/elementFromPoint-float-in-table.html` | 0 / 2 |  |
| `css/cssom-view/elementFromPoint-list-001.html` | 0 / 10 |  |
| `css/cssom-view/elementFromPoint-mixed-font-sizes.html` | 0 / 1 |  |
| `css/cssom-view/elementFromPoint-negative-z-index-inline-overflow-clip.html` | 0 / 1 |  |
| `css/cssom-view/elementFromPoint-negative-z-index-inline.html` | 0 / 8 |  |
| `css/cssom-view/elementFromPoint-subpixel.html` | 0 / 4 |  |
| `css/cssom-view/elementFromPoint-visibility-hidden-resizer.html` | 0 / 1 |  |
| `css/cssom-view/elementFromPoint.html` | 0 / 33 | harnais : 11 duplicate test names: "Negative co-ordinates", "co-ordinates larger than the viewport", "co-ordinates larger than the |
| `css/cssom-view/elementFromPosition.html` | 0 / 16 |  |
| `css/cssom-view/elementScroll-002.html` | 0 / 4 |  |
| `css/cssom-view/elementScroll.html` | 0 / 8 |  |
| `css/cssom-view/elementsFromPoint-iframes.html` | 0 / 2 |  |
| `css/cssom-view/elementsFromPoint-inline-htb-ltr.html` | 0 / 1 |  |
| `css/cssom-view/elementsFromPoint-inline-htb-rtl.html` | 0 / 1 |  |
| `css/cssom-view/elementsFromPoint-inline-nested.html` | 0 / 1 |  |
| `css/cssom-view/elementsFromPoint-inline-self-painting.html` | 0 / 4 |  |
| `css/cssom-view/elementsFromPoint-inline-vlr-ltr.html` | 0 / 1 |  |
| `css/cssom-view/elementsFromPoint-inline-vlr-rtl.html` | 0 / 1 |  |
| `css/cssom-view/elementsFromPoint-inline-vrl-ltr.html` | 0 / 1 |  |
| `css/cssom-view/elementsFromPoint-inline-vrl-rtl.html` | 0 / 1 |  |
| `css/cssom-view/elementsFromPoint-invalid-cases.html` | 0 / 3 |  |
| `css/cssom-view/elementsFromPoint-shadowroot.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/cssom-view/elementsFromPoint-simple.html` | 0 / 6 |  |
| `css/cssom-view/elementsFromPoint-svg-text.html` | 0 / 4 |  |
| `css/cssom-view/elementsFromPoint-svg.html` | 0 / 6 |  |
| `css/cssom-view/elementsFromPoint-table.html` | 0 / 4 |  |
| `css/cssom-view/elementsFromPoint.html` | 0 / 27 | harnais : 9 duplicate test names: "Negative co-ordinates", "co-ordinates larger than the viewport", "co-ordinates larger than the  |
| `css/cssom-view/getBoundingClientRect-content-visibility-hidden.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/cssom-view/getBoundingClientRect-empty-inline-002.html` | 2 / 4 |  |
| `css/cssom-view/getBoundingClientRect-empty-inline.html` | 0 / 1 |  |
| `css/cssom-view/getBoundingClientRect-newline.html` | 0 / 1 |  |
| `css/cssom-view/getBoundingClientRect-scroll.html` | 0 / 0 | harnais : TypeError: undefined n'est pas une fonction |
| `css/cssom-view/getBoundingClientRect-shy.html` | 0 / 12 |  |
| `css/cssom-view/getBoundingClientRect-svg.html` | 0 / 3 |  |
| `css/cssom-view/getBoundingClientRect-zoom.html` | 2 / 5 |  |
| `css/cssom-view/getClientRects-br-htb-ltr.html` | 0 / 1 |  |
| `css/cssom-view/getClientRects-br-htb-rtl.html` | 0 / 1 |  |
| `css/cssom-view/getClientRects-br-vlr-ltr.html` | 0 / 1 |  |
| `css/cssom-view/getClientRects-br-vlr-rtl.html` | 0 / 1 |  |
| `css/cssom-view/getClientRects-br-vrl-ltr.html` | 0 / 1 |  |
| `css/cssom-view/getClientRects-br-vrl-rtl.html` | 0 / 1 |  |
| `css/cssom-view/getClientRects-inline-atomic-child.html` | 0 / 3 |  |
| `css/cssom-view/getClientRects-inline-with-block-child.html` | 0 / 1 |  |
| `css/cssom-view/getClientRects-zoom.html` | 2 / 5 |  |
| `css/cssom-view/idlharness.html` | 0 / 1 |  |
| `css/cssom-view/image-x-y-zoom.html` | 0 / 2 |  |
| `css/cssom-view/inheritance.html` | 0 / 2 |  |
| `css/cssom-view/interrupt-hidden-smooth-scroll.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `css/cssom-view/matchMedia-display-none-iframe.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/cssom-view/matchMedia.html` | 6 / 10 |  |
| `css/cssom-view/mouseEvent-offsetXY-svg.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `css/cssom-view/mouseEvent.html` | 2 / 3 |  |
| `css/cssom-view/negativeMargins.html` | 0 / 2 |  |
| `css/cssom-view/offsetParent-body-and-html.html` | 5 / 6 |  |
| `css/cssom-view/offsetParent-fixed.html` | 0 / 2 |  |
| `css/cssom-view/offsetParent_element_test.html` | 0 / 2 |  |
| `css/cssom-view/offsetTop-offsetLeft-nested-offsetParents.html` | 0 / 1 |  |
| `css/cssom-view/offsetTop-offsetLeft-with-zoom.html` | 1 / 2 |  |
| `css/cssom-view/offsetTopLeft-empty-inline-offset.html` | 0 / 6 |  |
| `css/cssom-view/offsetTopLeft-empty-inline.html` | 0 / 3 |  |
| `css/cssom-view/offsetTopLeft-inline-offsetParent.html` | 7 / 19 |  |
| `css/cssom-view/offsetTopLeft-leading-space-inline.html` | 0 / 3 |  |
| `css/cssom-view/offsetTopLeft-trailing-space-inline.html` | 0 / 3 |  |
| `css/cssom-view/parsing/scroll-behavior-computed.html` | 0 / 2 |  |
| `css/cssom-view/parsing/scroll-behavior-invalid.html` | 0 / 3 |  |
| `css/cssom-view/position-sticky-root-scroller-with-scroll-behavior.html` | 0 / 1 |  |
| `css/cssom-view/range-bounding-client-rect-rtl-ligature-split.html` | 0 / 1 |  |
| `css/cssom-view/range-bounding-client-rect-with-display-contents.html` | 0 / 1 |  |
| `css/cssom-view/range-bounding-client-rect-with-nested-text.html` | 0 / 1 |  |
| `css/cssom-view/range-bounding-client-rect-with-shadow-dom.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/cssom-view/range-client-rects-surrogate-indexing.html` | 0 / 1 |  |
| `css/cssom-view/resizeTo-negative.html` | 0 / 1 |  |
| `css/cssom-view/screen-detached-frame.html` | 0 / 1 |  |
| `css/cssom-view/screenLeftTop.html` | 0 / 2 |  |
| `css/cssom-view/scroll-back-to-initial-position.html` | 0 / 2 |  |
| `css/cssom-view/scroll-behavior-default-css.html` | 0 / 3 |  |
| `css/cssom-view/scroll-behavior-element.html` | 0 / 39 |  |
| `css/cssom-view/scroll-behavior-main-frame-root.html` | 1 / 40 |  |
| `css/cssom-view/scroll-behavior-main-frame-window.html` | 1 / 28 |  |
| `css/cssom-view/scroll-behavior-scrollintoview-nested.html` | 0 / 2 |  |
| `css/cssom-view/scroll-behavior-smooth-after-hiding-iframe.html` | 0 / 1 |  |
| `css/cssom-view/scroll-behavior-smooth-navigation.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `css/cssom-view/scroll-behavior-smooth-positions.html` | 0 / 23 |  |
| `css/cssom-view/scroll-behavior-smooth.html` | 0 / 3 | harnais : TypeError: undefined n'est pas une fonction |
| `css/cssom-view/scroll-behavior-subframe-root.html` | 1 / 40 |  |
| `css/cssom-view/scroll-behavior-subframe-window.html` | 1 / 28 |  |
| `css/cssom-view/scroll-clamp-after-transform-shrinks-overflow.html` | 0 / 1 |  |
| `css/cssom-view/scroll-no-layout-box.html` | 0 / 1 |  |
| `css/cssom-view/scroll-offsets-fractional-zoom.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/cssom-view/scroll-overflow-clip-quirks-001.html` | 0 / 1 |  |
| `css/cssom-view/scroll-overflow-clip-quirks-002.html` | 0 / 1 |  |
| `css/cssom-view/scroll-position-root-child-ib-split-insertion.html` | 0 / 1 |  |
| `css/cssom-view/scroll-target-into-view-fragment-in-same-origin-subframe.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `css/cssom-view/scroll-width-with-over-constrained-block-in-inline.html` | 1 / 2 |  |
| `css/cssom-view/scroll-zoom.html` | 1 / 3 |  |
| `css/cssom-view/scrollIntoView-align-scrollport-covering-child.html` | 0 / 1 |  |
| `css/cssom-view/scrollIntoView-container.html` | 0 / 5 |  |
| `css/cssom-view/scrollIntoView-fixed-outside-of-viewport.html` | 0 / 1 |  |
| `css/cssom-view/scrollIntoView-fixed.html` | 0 / 8 | harnais : 4 duplicate test names: "[Box A] scrollIntoView from unscrollable position:fixed", "[Box B] scrollIntoView from unscroll |
| `css/cssom-view/scrollIntoView-horizontal-partially-visible.html` | 0 / 1 |  |
| `css/cssom-view/scrollIntoView-horizontal-tb-writing-mode-and-rtl-direction.html` | 0 / 9 |  |
| `css/cssom-view/scrollIntoView-horizontal-tb-writing-mode.html` | 0 / 9 |  |
| `css/cssom-view/scrollIntoView-iframes.html` | 1 / 4 |  |
| `css/cssom-view/scrollIntoView-inline-image.html` | 0 / 1 |  |
| `css/cssom-view/scrollIntoView-multiple-nested.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `css/cssom-view/scrollIntoView-multiple.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `css/cssom-view/scrollIntoView-nearest-oversized-element.html` | 0 / 1 |  |
| `css/cssom-view/scrollIntoView-root-overflow-clip.html` | 0 / 2 |  |
| `css/cssom-view/scrollIntoView-scrollMargin.html` | 0 / 3 |  |
| `css/cssom-view/scrollIntoView-scrollPadding.html` | 0 / 3 |  |
| `css/cssom-view/scrollIntoView-scrolling-box-with-large-border.html` | 0 / 1 |  |
| `css/cssom-view/scrollIntoView-scrolling-container.html` | 0 / 1 |  |
| `css/cssom-view/scrollIntoView-shadow.html` | 0 / 1 |  |
| `css/cssom-view/scrollIntoView-sideways-lr-writing-mode-and-rtl-direction.html` | 0 / 9 |  |
| `css/cssom-view/scrollIntoView-sideways-lr-writing-mode.html` | 0 / 9 |  |
| `css/cssom-view/scrollIntoView-sideways-rl-writing-mode-and-rtl-direction.html` | 0 / 9 |  |
| `css/cssom-view/scrollIntoView-sideways-rl-writing-mode.html` | 0 / 9 |  |
| `css/cssom-view/scrollIntoView-smooth.html` | 0 / 4 |  |
| `css/cssom-view/scrollIntoView-stuck.tentative.html` | 0 / 1 |  |
| `css/cssom-view/scrollIntoView-svg-shape.html` | 0 / 3 |  |
| `css/cssom-view/scrollIntoView-then-scrollBy.html` | 0 / 1 |  |
| `css/cssom-view/scrollIntoView-vertical-lr-writing-mode-and-rtl-direction.html` | 0 / 9 |  |
| `css/cssom-view/scrollIntoView-vertical-lr-writing-mode.html` | 0 / 9 |  |
| `css/cssom-view/scrollIntoView-vertical-rl-writing-mode.html` | 0 / 9 |  |
| `css/cssom-view/scrollLeft-of-scroller-with-wider-scrollbar.html` | 0 / 1 |  |
| `css/cssom-view/scrollLeftTop.html` | 0 / 6 |  |
| `css/cssom-view/scrollParent-quirks-mode.html` | 0 / 1 |  |
| `css/cssom-view/scrollParent-shadow-tree.html` | 0 / 4 |  |
| `css/cssom-view/scrollParent.html` | 0 / 12 |  |
| `css/cssom-view/scrollWidthHeight-child-border-within-padding.tentative.html` | 0 / 5 |  |
| `css/cssom-view/scrollWidthHeight-flex-column-padding-001.html` | 0 / 2 |  |
| `css/cssom-view/scrollWidthHeight-negative-margin-002.html` | 80 / 200 |  |
| `css/cssom-view/scrolling-no-browsing-context.html` | 0 / 1 |  |
| `css/cssom-view/scrolling-quirks-vs-nonquirks.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `css/cssom-view/scrollingElement.html` | 0 / 8 |  |
| `css/cssom-view/scrollintoview-containingblock-chain.html` | 0 / 1 |  |
| `css/cssom-view/scrollintoview-zero-height-item.html` | 0 / 1 |  |
| `css/cssom-view/scrollintoview.html` | 0 / 40 |  |
| `css/cssom-view/smooth-scroll-in-load-event.html` | 0 / 1 |  |
| `css/cssom-view/smooth-scroll-nonstop.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `css/cssom-view/smooth-scrollIntoView-with-smooth-fragment-scroll.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `css/cssom-view/smooth-scrollIntoView-with-unrelated-gesture-scroll.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `css/cssom-view/subpixel-sizes-and-offsets.tentative.html` | 12 / 15 |  |
| `css/cssom-view/table-client-props.html` | 8 / 15 |  |
| `css/cssom-view/table-offset-props.html` | 9 / 13 |  |
| `css/cssom-view/table-scroll-props.html` | 8 / 13 |  |
| `css/cssom-view/table-with-border-client-width-height.html` | 0 / 1 |  |
| `css/cssom-view/user-scroll-interrupts-smooth-scroll.html` | 1 / 3 |  |
| `css/cssom-view/visual-scrollIntoView-001.html` | 0 / 1 |  |
| `css/cssom-view/visual-scrollIntoView-002.html` | 0 / 1 |  |
| `css/cssom-view/visual-scrollIntoView-003.html` | 0 / 1 |  |
| `css/cssom-view/window-scroll-arguments.html` | 6 / 12 | harnais : 4 duplicate test names: " with 0 arguments", " with 1 non-dictionary argument", " with a valid ScrollToOptions argument" |
| `css/cssom-view/window-scroll-promise-interruption.html` | 0 / 4 | harnais : TypeError: undefined n'est pas une fonction |
| `css/cssom-view/window-scroll-promises.html` | 0 / 18 | harnais : TypeError: undefined n'est pas une fonction |
