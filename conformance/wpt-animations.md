# Web Platform Tests

Produit par `cargo run --release -p oxyde_wpt -- --publier css/css-transforms css/css-transitions css/css-animations`, sur la suite
`web-platform-tests/wpt` déposée dans `vendor/wpt`. Seuls les tests `testharness.js`
destinés à une fenêtre sont exécutés ; un fichier est « entièrement réussi » quand le
harnais va au bout et que tous ses sous-tests passent.

**Sous-tests réussis : 337 / 4340 (7,8 %)** — fichiers entièrement réussis : 27 / 363 (7,4 %).

Le taux en sous-tests surestime la conformité : un fichier qui plante ou que le harnais
abandonne ne rapporte aucun sous-test, donc n'en fait échouer aucun. Le taux en fichiers
est le plus honnête des deux.

| Dossier | Fichiers réussis | Sous-tests réussis |
|---|---:|---:|
| `css/css-animations` | 2 / 82 | 7 / 398 (1,8 %) |
| `css/css-animations/parsing` | 7 / 41 | 127 / 574 (22,1 %) |
| `css/css-animations/responsive` | 0 / 4 | 0 / 7 (0,0 %) |
| `css/css-animations/stability` | 0 / 1 | 0 / 1 (0,0 %) |
| `css/css-transforms` | 9 / 43 | 34 / 197 (17,3 %) |
| `css/css-transforms/animation` | 0 / 37 | 0 / 126 (0,0 %) |
| `css/css-transforms/parsing` | 2 / 28 | 113 / 389 (29,0 %) |
| `css/css-transitions` | 5 / 98 | 12 / 2385 (0,5 %) |
| `css/css-transitions/animations` | 0 / 11 | 0 / 11 (0,0 %) |
| `css/css-transitions/parsing` | 2 / 18 | 44 / 252 (17,5 %) |

## Fichiers non entièrement réussis

| Fichier | Sous-tests | Cause |
|---|---:|---|
| `css/css-animations/AnimationEffect-getComputedTiming.tentative.html` | 0 / 43 |  |
| `css/css-animations/AnimationEffect-updateTiming.tentative.html` | 0 / 5 |  |
| `css/css-animations/CSSAnimation-animationName.tentative.html` | 0 / 3 |  |
| `css/css-animations/CSSAnimation-canceling.tentative.html` | 0 / 9 |  |
| `css/css-animations/CSSAnimation-compositeOrder.tentative.html` | 0 / 2 |  |
| `css/css-animations/CSSAnimation-effect.tentative.html` | 0 / 8 |  |
| `css/css-animations/CSSAnimation-finished.tentative.html` | 0 / 3 |  |
| `css/css-animations/CSSAnimation-getCurrentTime.tentative.html` | 0 / 2 |  |
| `css/css-animations/CSSAnimation-id.tentative.html` | 0 / 1 |  |
| `css/css-animations/CSSAnimation-pausing.tentative.html` | 0 / 9 |  |
| `css/css-animations/CSSAnimation-playState.tentative.html` | 0 / 5 |  |
| `css/css-animations/CSSAnimation-ready.tentative.html` | 0 / 5 |  |
| `css/css-animations/CSSAnimation-startTime.tentative.html` | 0 / 3 |  |
| `css/css-animations/Document-getAnimations.tentative.html` | 0 / 18 |  |
| `css/css-animations/Element-getAnimations-dynamic-changes.tentative.html` | 0 / 5 |  |
| `css/css-animations/Element-getAnimations.tentative.html` | 0 / 22 | harnais : délai du harnais dépassé |
| `css/css-animations/KeyframeEffect-getKeyframes.tentative.html` | 0 / 32 |  |
| `css/css-animations/KeyframeEffect-setKeyframes.tentative.html` | 0 / 3 |  |
| `css/css-animations/KeyframeEffect-target.tentative.html` | 0 / 3 |  |
| `css/css-animations/animate-with-background-color-oklch-002.html` | 0 / 2 |  |
| `css/css-animations/animate-with-color-mix-2.html` | 0 / 1 |  |
| `css/css-animations/animate-with-color-mix.html` | 0 / 6 |  |
| `css/css-animations/animate-with-relative-color.html` | 0 / 2 |  |
| `css/css-animations/animation-base-response-001.html` | 0 / 3 |  |
| `css/css-animations/animation-base-response-002.html` | 0 / 1 |  |
| `css/css-animations/animation-base-response-003.html` | 0 / 1 |  |
| `css/css-animations/animation-base-response-004.html` | 0 / 1 |  |
| `css/css-animations/animation-before-initial-box-construction-001.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `css/css-animations/animation-change-underlying-value-changed-in-flight.html` | 0 / 2 |  |
| `css/css-animations/animation-composition-keyframes.html` | 0 / 6 |  |
| `css/css-animations/animation-composition.html` | 0 / 6 |  |
| `css/css-animations/animation-css-variable-dependent-property.html` | 0 / 1 |  |
| `css/css-animations/animation-css-variable-in-keyframe-adjusted.html` | 0 / 2 |  |
| `css/css-animations/animation-important-001.html` | 1 / 7 |  |
| `css/css-animations/animation-important-with-transition.html` | 1 / 5 |  |
| `css/css-animations/animation-in-offscreen-iframe.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `css/css-animations/animation-iteration-count-009.html` | 0 / 4 |  |
| `css/css-animations/animation-iteration-count-010.html` | 2 / 6 |  |
| `css/css-animations/animation-iteration-count-calc.html` | 0 / 1 |  |
| `css/css-animations/animation-iteration-event-001.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `css/css-animations/animation-iteration-event-002.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `css/css-animations/animation-multiple-from-to-keyframes-with-only-timing-function.html` | 0 / 1 |  |
| `css/css-animations/animation-name-inline-style.html` | 0 / 1 |  |
| `css/css-animations/animation-play-state-005.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `css/css-animations/animation-restarted-after-changing-iteration-count-after-completion.html` | 0 / 1 |  |
| `css/css-animations/animation-style-element-replaced-with-keyframes-rule-of-same-name.html` | 0 / 1 |  |
| `css/css-animations/animationevent-interface.html` | 0 / 42 |  |
| `css/css-animations/animationevent-marker-pseudoelement.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `css/css-animations/animationevent-pseudoelement.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `css/css-animations/animationevent-types.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `css/css-animations/cancel-events-on-remove.html` | 0 / 5 | harnais : délai du harnais dépassé |
| `css/css-animations/computed-style-animation-parsing.html` | 0 / 7 |  |
| `css/css-animations/dialog-animation.html` | 0 / 1 |  |
| `css/css-animations/dialog-backdrop-animation.html` | 0 / 1 |  |
| `css/css-animations/display-contents-animates.html` | 0 / 1 |  |
| `css/css-animations/display-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-animations/display-none-dont-cancel-pseudo.tentative.html` | 0 / 1 |  |
| `css/css-animations/display-none-dont-cancel.tentative.html` | 0 / 5 |  |
| `css/css-animations/display-none-prevents-starting-in-subtree.html` | 0 / 1 |  |
| `css/css-animations/display-none-to-display-block-dont-cancel.tentative.html` | 0 / 1 |  |
| `css/css-animations/display-none-to-display-block-while-running.tentative.html` | 0 / 1 |  |
| `css/css-animations/empty-pseudo-class-with-animation.html` | 0 / 1 |  |
| `css/css-animations/event-dispatch.tentative.html` | 0 / 32 |  |
| `css/css-animations/event-order.tentative.html` | 0 / 6 |  |
| `css/css-animations/idlharness.html` | 0 / 1 |  |
| `css/css-animations/inheritance.html` | 0 / 16 |  |
| `css/css-animations/keyframes-rule-caching.html` | 0 / 1 |  |
| `css/css-animations/keyframes-unrelated-custom-property.html` | 0 / 1 |  |
| `css/css-animations/missing-values-first-keyframe.html` | 0 / 1 |  |
| `css/css-animations/missing-values-last-keyframe.html` | 0 / 1 |  |
| `css/css-animations/missing-values-middle-keyframe.html` | 0 / 1 |  |
| `css/css-animations/parsing/animation-composition-computed.tentative.html` | 0 / 1 |  |
| `css/css-animations/parsing/animation-composition-invalid.tentative.html` | 0 / 4 |  |
| `css/css-animations/parsing/animation-computed.html` | 0 / 15 |  |
| `css/css-animations/parsing/animation-delay-computed.html` | 0 / 4 |  |
| `css/css-animations/parsing/animation-delay-invalid.html` | 0 / 5 |  |
| `css/css-animations/parsing/animation-direction-computed.html` | 0 / 1 |  |
| `css/css-animations/parsing/animation-direction-invalid.html` | 0 / 4 |  |
| `css/css-animations/parsing/animation-duration-computed.html` | 0 / 15 |  |
| `css/css-animations/parsing/animation-duration-invalid.html` | 0 / 6 |  |
| `css/css-animations/parsing/animation-fill-mode-computed.html` | 0 / 1 |  |
| `css/css-animations/parsing/animation-fill-mode-invalid.html` | 0 / 4 |  |
| `css/css-animations/parsing/animation-invalid.html` | 0 / 8 |  |
| `css/css-animations/parsing/animation-iteration-count-computed.html` | 0 / 2 |  |
| `css/css-animations/parsing/animation-iteration-count-invalid.html` | 0 / 5 |  |
| `css/css-animations/parsing/animation-name-computed.html` | 0 / 27 |  |
| `css/css-animations/parsing/animation-name-invalid.html` | 0 / 9 |  |
| `css/css-animations/parsing/animation-name-valid.html` | 23 / 27 |  |
| `css/css-animations/parsing/animation-play-state-computed.html` | 0 / 1 |  |
| `css/css-animations/parsing/animation-play-state-invalid.html` | 0 / 4 |  |
| `css/css-animations/parsing/animation-range-end-computed.html` | 0 / 29 |  |
| `css/css-animations/parsing/animation-range-end-invalid.html` | 0 / 14 |  |
| `css/css-animations/parsing/animation-range-end-valid.html` | 20 / 24 |  |
| `css/css-animations/parsing/animation-range-shorthand-end-zero-offset.html` | 3 / 6 |  |
| `css/css-animations/parsing/animation-range-shorthand.html` | 22 / 133 |  |
| `css/css-animations/parsing/animation-range-start-computed.html` | 0 / 30 |  |
| `css/css-animations/parsing/animation-range-start-invalid.html` | 0 / 11 |  |
| `css/css-animations/parsing/animation-range-start-valid.html` | 21 / 26 |  |
| `css/css-animations/parsing/animation-shorthand.html` | 0 / 36 |  |
| `css/css-animations/parsing/animation-timing-function-invalid.html` | 0 / 7 |  |
| `css/css-animations/parsing/animation-valid.html` | 9 / 12 |  |
| `css/css-animations/parsing/keyframe-selectors.html` | 0 / 14 |  |
| `css/css-animations/parsing/keyframes-allowed-properties.html` | 0 / 1 |  |
| `css/css-animations/parsing/keyframes-name-invalid.html` | 0 / 20 |  |
| `css/css-animations/parsing/keyframes-name-valid.html` | 0 / 39 |  |
| `css/css-animations/pending-style-changes-001.html` | 0 / 2 |  |
| `css/css-animations/responsive/column-rule-color-001.html` | 0 / 2 |  |
| `css/css-animations/responsive/column-width-001.html` | 0 / 3 |  |
| `css/css-animations/responsive/fill-forwards-viewport-units.html` | 0 / 1 |  |
| `css/css-animations/responsive/line-height.html` | 0 / 1 |  |
| `css/css-animations/revert-rule-keyframes-dynamic.html` | 0 / 2 |  |
| `css/css-animations/revert-rule-keyframes.html` | 0 / 2 |  |
| `css/css-animations/sample-on-last-keyframe.html` | 0 / 1 |  |
| `css/css-animations/stability/animation-event-destroy-renderer.html` | 0 / 1 |  |
| `css/css-animations/style-animation-parsing.html` | 1 / 5 |  |
| `css/css-animations/svg-color-visited-currentcolor.html` | 0 / 3 |  |
| `css/css-animations/text-decoration-inset-auto.html` | 0 / 1 |  |
| `css/css-animations/transition-properties-not-animatable.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-animations/transition-ready-time-offscreen.html` | 0 / 1 |  |
| `css/css-transforms/2d-rotate-js.html` | 0 / 1 |  |
| `css/css-transforms/2d-transform-inline-js.html` | 0 / 1 |  |
| `css/css-transforms/3d-point-mapping-2-transforminterop.html` | 0 / 12 |  |
| `css/css-transforms/3d-point-mapping-3.html` | 0 / 1 |  |
| `css/css-transforms/3d-point-mapping-4.html` | 0 / 2 |  |
| `css/css-transforms/3d-point-mapping-coplanar.html` | 0 / 4 |  |
| `css/css-transforms/3d-point-mapping-deep.html` | 0 / 8 |  |
| `css/css-transforms/3d-point-mapping-origins.html` | 0 / 12 |  |
| `css/css-transforms/3d-point-mapping-overlapping.html` | 0 / 4 |  |
| `css/css-transforms/3d-point-mapping-preserve-3d.html` | 0 / 20 |  |
| `css/css-transforms/3d-point-mapping.html` | 0 / 12 |  |
| `css/css-transforms/3d-rendering-context-behavior.html` | 3 / 12 |  |
| `css/css-transforms/animation/backface-visibility-no-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transforms/animation/composited-transform.html` | 0 / 1 |  |
| `css/css-transforms/animation/list-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transforms/animation/matrix-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transforms/animation/perspective-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transforms/animation/perspective-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transforms/animation/perspective-origin-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transforms/animation/perspective-origin-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transforms/animation/rotate-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transforms/animation/rotate-interpolation-math-functions-tentative.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transforms/animation/rotate-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transforms/animation/scale-animation-math-functions-tentative.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transforms/animation/scale-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transforms/animation/scale-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transforms/animation/transform-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transforms/animation/transform-interpolation-001.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transforms/animation/transform-interpolation-002.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transforms/animation/transform-interpolation-003.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transforms/animation/transform-interpolation-004.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transforms/animation/transform-interpolation-005.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transforms/animation/transform-interpolation-006.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transforms/animation/transform-interpolation-007.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transforms/animation/transform-interpolation-computed-value.html` | 0 / 82 |  |
| `css/css-transforms/animation/transform-interpolation-inline-value.html` | 0 / 41 |  |
| `css/css-transforms/animation/transform-interpolation-rotate3d-180deg-near-x-axis.html` | 0 / 1 |  |
| `css/css-transforms/animation/transform-interpolation-verify-reftests.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transforms/animation/transform-matrix-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transforms/animation/transform-non-invertible-no-transition.html` | 0 / 1 |  |
| `css/css-transforms/animation/transform-origin-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transforms/animation/transform-origin-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transforms/animation/transform-perspective-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transforms/animation/transform-rotate-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transforms/animation/transform-scale-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transforms/animation/transform-skew-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transforms/animation/transform-translate-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transforms/animation/translate-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transforms/animation/translate-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transforms/css-transform-style-evaluation-validation.html` | 11 / 13 |  |
| `css/css-transforms/hittest-preserve-3d-2.html` | 0 / 1 |  |
| `css/css-transforms/hittest-preserve-3d.html` | 0 / 1 |  |
| `css/css-transforms/inheritance.html` | 4 / 20 |  |
| `css/css-transforms/inline-with-filter-and-hidden-backface.html` | 0 / 1 |  |
| `css/css-transforms/parsing/backface-visibility-computed.html` | 0 / 2 |  |
| `css/css-transforms/parsing/backface-visibility-invalid.html` | 0 / 2 |  |
| `css/css-transforms/parsing/perspective-invalid.html` | 0 / 3 |  |
| `css/css-transforms/parsing/perspective-origin-computed.html` | 0 / 21 |  |
| `css/css-transforms/parsing/perspective-origin-invalid.html` | 0 / 12 |  |
| `css/css-transforms/parsing/perspective-origin-valid.html` | 10 / 18 |  |
| `css/css-transforms/parsing/rotate-parsing-computed.html` | 0 / 23 |  |
| `css/css-transforms/parsing/rotate-parsing-invalid.html` | 0 / 9 |  |
| `css/css-transforms/parsing/rotate-parsing-valid.html` | 7 / 23 |  |
| `css/css-transforms/parsing/scale-parsing-computed.html` | 0 / 38 |  |
| `css/css-transforms/parsing/scale-parsing-invalid.html` | 0 / 8 |  |
| `css/css-transforms/parsing/scale-parsing-valid.html` | 15 / 32 |  |
| `css/css-transforms/parsing/transform-box-computed.html` | 0 / 5 |  |
| `css/css-transforms/parsing/transform-box-invalid.html` | 0 / 3 |  |
| `css/css-transforms/parsing/transform-computed.html` | 2 / 6 |  |
| `css/css-transforms/parsing/transform-invalid.html` | 0 / 20 |  |
| `css/css-transforms/parsing/transform-origin-computed.html` | 18 / 23 |  |
| `css/css-transforms/parsing/transform-origin-invalid.html` | 0 / 10 |  |
| `css/css-transforms/parsing/transform-origin-valid.html` | 5 / 16 |  |
| `css/css-transforms/parsing/transform-valid.html` | 20 / 42 |  |
| `css/css-transforms/parsing/translate-parsing-computed.html` | 0 / 19 |  |
| `css/css-transforms/parsing/translate-parsing-invalid.html` | 0 / 6 |  |
| `css/css-transforms/parsing/translate-parsing-valid.html` | 14 / 20 |  |
| `css/css-transforms/parsing/webkit-perspective-invalid.tentative.html` | 0 / 2 |  |
| `css/css-transforms/parsing/webkit-perspective-valid.tentative.html` | 8 / 10 |  |
| `css/css-transforms/parsing/webkit-transform-valid.tentative.html` | 7 / 9 |  |
| `css/css-transforms/preserve-3d-flat-grouping-properties-containing-block.html` | 0 / 2 |  |
| `css/css-transforms/preserve-3d-flat-grouping-properties.html` | 2 / 10 |  |
| `css/css-transforms/preserve3d-backface-hit-test-1.html` | 0 / 1 |  |
| `css/css-transforms/preserve3d-backface-hit-test-2.html` | 0 / 1 |  |
| `css/css-transforms/scroll-preserve-3d.html` | 0 / 1 |  |
| `css/css-transforms/transform-2d-getComputedStyle-001.html` | 0 / 5 |  |
| `css/css-transforms/transform-and-individual-transform-properties-computed-style.html` | 0 / 1 |  |
| `css/css-transforms/transform-hit-testing.html` | 0 / 5 |  |
| `css/css-transforms/transform-important.html` | 0 / 1 |  |
| `css/css-transforms/transform-origin-014.html` | 0 / 4 |  |
| `css/css-transforms/transform-origin-in-shadow.html` | 0 / 0 | harnais : TypeError: undefined n'est pas une fonction |
| `css/css-transforms/transform-percent-009.html` | 0 / 4 |  |
| `css/css-transforms/transform-scale-hittest.html` | 0 / 2 |  |
| `css/css-transforms/transform-with-sign-function.html` | 0 / 11 |  |
| `css/css-transforms/transformed-table-section-containing-block.html` | 0 / 2 |  |
| `css/css-transforms/transforms-support-calc.html` | 0 / 7 |  |
| `css/css-transforms/translate-getComputedStyle.html` | 0 / 1 |  |
| `css/css-transitions/AnimationEffect-getComputedTiming.tentative.html` | 0 / 22 |  |
| `css/css-transitions/CSSTransition-canceling.tentative.html` | 0 / 11 |  |
| `css/css-transitions/CSSTransition-currentTime.tentative.html` | 0 / 5 |  |
| `css/css-transitions/CSSTransition-effect.tentative.html` | 0 / 10 |  |
| `css/css-transitions/CSSTransition-finished.tentative.html` | 0 / 1 |  |
| `css/css-transitions/CSSTransition-not-canceling.tentative.html` | 0 / 2 |  |
| `css/css-transitions/CSSTransition-ready.tentative.html` | 0 / 2 |  |
| `css/css-transitions/CSSTransition-startTime.tentative.html` | 0 / 5 |  |
| `css/css-transitions/CSSTransition-transitionProperty.tentative.html` | 0 / 1 |  |
| `css/css-transitions/Document-getAnimations.tentative.html` | 0 / 7 |  |
| `css/css-transitions/Element-getAnimations.tentative.html` | 0 / 7 |  |
| `css/css-transitions/KeyframeEffect-getKeyframes-width-and-height-transition.tentative.html` | 0 / 2 |  |
| `css/css-transitions/KeyframeEffect-getKeyframes.tentative.html` | 0 / 5 |  |
| `css/css-transitions/KeyframeEffect-setKeyframes.tentative.html` | 0 / 8 |  |
| `css/css-transitions/KeyframeEffect-target.tentative.html` | 0 / 3 |  |
| `css/css-transitions/after-change-style-inherited-try-fallback.html` | 0 / 1 |  |
| `css/css-transitions/after-change-style-inherited.html` | 0 / 6 |  |
| `css/css-transitions/all-interpolates-same-as-explicit-property.html` | 1 / 32 |  |
| `css/css-transitions/animations/animate-with-color-mix.html` | 0 / 6 |  |
| `css/css-transitions/animations/change-duration-during-transition.html` | 0 / 1 |  |
| `css/css-transitions/animations/color-transition-premultiplied.html` | 0 / 1 |  |
| `css/css-transitions/animations/move-after-transition.html` | 0 / 1 |  |
| `css/css-transitions/animations/text-shadow-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transitions/animations/text-shadow-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transitions/animations/transition-end-event-shorthands.html` | 0 / 1 |  |
| `css/css-transitions/animations/transition-timing-function.html` | 0 / 1 |  |
| `css/css-transitions/animations/vertical-align-composition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transitions/animations/vertical-align-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transitions/animations/z-index-interpolation.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `css/css-transitions/before-load-001.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `css/css-transitions/changing-while-transition-001.html` | 0 / 1 |  |
| `css/css-transitions/changing-while-transition-002.html` | 0 / 1 |  |
| `css/css-transitions/changing-while-transition-003.html` | 0 / 1 |  |
| `css/css-transitions/changing-while-transition-004.html` | 0 / 1 |  |
| `css/css-transitions/custom-property-and-allow-discrete.html` | 0 / 2 |  |
| `css/css-transitions/disconnected-element-001.html` | 3 / 6 | harnais : délai du harnais dépassé |
| `css/css-transitions/display-none-no-animations.html` | 0 / 1 |  |
| `css/css-transitions/dynamic-root-element.html` | 0 / 1 |  |
| `css/css-transitions/event-dispatch.tentative.html` | 0 / 30 |  |
| `css/css-transitions/events-001.html` | 0 / 9 | harnais : délai du harnais dépassé |
| `css/css-transitions/events-002.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `css/css-transitions/events-003.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `css/css-transitions/events-004.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `css/css-transitions/events-005.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `css/css-transitions/events-006.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `css/css-transitions/events-008.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `css/css-transitions/idlharness-2.html` | 0 / 1 |  |
| `css/css-transitions/idlharness.html` | 0 / 1 |  |
| `css/css-transitions/inert-while-transitioning-to-display-none.html` | 0 / 1 |  |
| `css/css-transitions/inherit-height-transition.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `css/css-transitions/inheritance.html` | 0 / 8 |  |
| `css/css-transitions/no-transition-for-unchanged-pointer-valued-properties.html` | 0 / 1 |  |
| `css/css-transitions/non-rendered-element-001.html` | 3 / 4 | harnais : délai du harnais dépassé |
| `css/css-transitions/non-rendered-element-002.html` | 0 / 1 |  |
| `css/css-transitions/non-rendered-element-004.tentative.html` | 0 / 1 |  |
| `css/css-transitions/parsing/starting-style-parsing.html` | 0 / 5 |  |
| `css/css-transitions/parsing/transition-behavior.html` | 3 / 28 |  |
| `css/css-transitions/parsing/transition-computed.html` | 0 / 10 |  |
| `css/css-transitions/parsing/transition-delay-computed.html` | 0 / 1 |  |
| `css/css-transitions/parsing/transition-delay-invalid.html` | 0 / 5 |  |
| `css/css-transitions/parsing/transition-duration-computed.html` | 0 / 3 |  |
| `css/css-transitions/parsing/transition-duration-invalid.html` | 0 / 5 |  |
| `css/css-transitions/parsing/transition-invalid.html` | 0 / 5 |  |
| `css/css-transitions/parsing/transition-property-computed.html` | 0 / 10 |  |
| `css/css-transitions/parsing/transition-property-invalid.html` | 0 / 66 |  |
| `css/css-transitions/parsing/transition-property-valid.html` | 9 / 10 |  |
| `css/css-transitions/parsing/transition-shorthand.html` | 0 / 18 |  |
| `css/css-transitions/parsing/transition-timing-function-computed.html` | 0 / 22 |  |
| `css/css-transitions/parsing/transition-timing-function-invalid.html` | 0 / 25 |  |
| `css/css-transitions/parsing/transition-timing-function-valid.html` | 18 / 22 |  |
| `css/css-transitions/parsing/transition-valid.html` | 7 / 10 |  |
| `css/css-transitions/properties-value-001.html` | 0 / 580 | harnais : délai du harnais dépassé |
| `css/css-transitions/properties-value-002.html` | 0 / 18 | harnais : délai du harnais dépassé |
| `css/css-transitions/properties-value-003.html` | 0 / 122 | harnais : délai du harnais dépassé |
| `css/css-transitions/properties-value-implicit-001.html` | 0 / 62 | harnais : délai du harnais dépassé |
| `css/css-transitions/properties-value-inherit-001.html` | 0 / 580 | harnais : délai du harnais dépassé |
| `css/css-transitions/properties-value-inherit-002.html` | 0 / 580 | harnais : délai du harnais dépassé |
| `css/css-transitions/properties-value-inherit-003.html` | 0 / 62 | harnais : délai du harnais dépassé |
| `css/css-transitions/pseudo-elements-001.html` | 0 / 4 |  |
| `css/css-transitions/pseudo-elements-002.html` | 0 / 1 |  |
| `css/css-transitions/retargetted-transition-with-box-sizing.html` | 0 / 1 |  |
| `css/css-transitions/shadow-root-insertion.html` | 0 / 1 |  |
| `css/css-transitions/starting-of-transitions-001.html` | 0 / 1 |  |
| `css/css-transitions/starting-style-adjustment.html` | 0 / 1 |  |
| `css/css-transitions/starting-style-cascade.html` | 0 / 0 | harnais : ReferenceError: CSSStyleSheet n'est pas défini |
| `css/css-transitions/starting-style-name-defining-rules.html` | 0 / 2 |  |
| `css/css-transitions/starting-style-revert-layer-cascade.html` | 0 / 0 | harnais : ReferenceError: CSSStyleSheet n'est pas défini |
| `css/css-transitions/starting-style-rule-basic.html` | 0 / 3 |  |
| `css/css-transitions/starting-style-rule-none.html` | 0 / 0 | harnais : ReferenceError: CSSStyleSheet n'est pas défini |
| `css/css-transitions/starting-style-rule-pseudo-elements.html` | 0 / 3 |  |
| `css/css-transitions/starting-style-size-container.html` | 0 / 2 |  |
| `css/css-transitions/transition-001.html` | 0 / 13 |  |
| `css/css-transitions/transition-after-animation-001.html` | 0 / 1 |  |
| `css/css-transitions/transition-base-response-001.html` | 0 / 3 |  |
| `css/css-transitions/transition-base-response-002.html` | 0 / 1 |  |
| `css/css-transitions/transition-base-response-003.html` | 0 / 1 |  |
| `css/css-transitions/transition-behavior-events.html` | 0 / 2 |  |
| `css/css-transitions/transition-behavior.html` | 0 / 6 |  |
| `css/css-transitions/transition-cancel-elapsed-time-on-removal.html` | 0 / 1 |  |
| `css/css-transitions/transition-delay-001.html` | 0 / 21 |  |
| `css/css-transitions/transition-duration-001.html` | 0 / 21 |  |
| `css/css-transitions/transition-duration-shorthand.html` | 0 / 1 |  |
| `css/css-transitions/transition-events-with-document-change.html` | 0 / 1 |  |
| `css/css-transitions/transition-important.html` | 0 / 1 |  |
| `css/css-transitions/transition-in-iframe-001.html` | 0 / 0 | le harnais n'a pas terminé |
| `css/css-transitions/transition-property-001.html` | 0 / 5 |  |
| `css/css-transitions/transition-property-002.html` | 0 / 7 |  |
| `css/css-transitions/transition-remove-and-change-immediate.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `css/css-transitions/transition-reparented.html` | 0 / 1 |  |
| `css/css-transitions/transition-zero-duration-with-delay.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `css/css-transitions/transitioncancel-001.html` | 0 / 1 |  |
| `css/css-transitions/transitioncancel-002.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `css/css-transitions/transitioncancel-003.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `css/css-transitions/transitionevent-interface.html` | 0 / 42 |  |
| `css/css-transitions/transitions-retarget.html` | 0 / 1 |  |
| `css/css-transitions/ua-important-001.tentative.html` | 0 / 1 |  |
| `css/css-transitions/ua-important-002.tentative.html` | 0 / 1 |  |
| `css/css-transitions/ua-important-003.tentative.html` | 0 / 1 |  |
| `css/css-transitions/unregistered-custom-property-transition.html` | 0 / 3 |  |
| `css/css-transitions/zero-duration-multiple-transition.html` | 0 / 1 |  |
