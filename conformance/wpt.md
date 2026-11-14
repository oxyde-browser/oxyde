# Web Platform Tests

Produit par `cargo run --release -p oxyde_wpt -- --publier dom`, sur la suite
`web-platform-tests/wpt` déposée dans `vendor/wpt`. Seuls les tests `testharness.js`
destinés à une fenêtre sont exécutés ; un fichier est « entièrement réussi » quand le
harnais va au bout et que tous ses sous-tests passent.

**Sous-tests réussis : 44543 / 50203 (88,7 %)** — fichiers entièrement réussis : 462 / 693 (66,7 %).

Le taux en sous-tests surestime la conformité : un fichier qui plante ou que le harnais
abandonne ne rapporte aucun sous-test, donc n'en fait échouer aucun. Le taux en fichiers
est le plus honnête des deux.

| Dossier | Fichiers réussis | Sous-tests réussis |
|---|---:|---:|
| `dom` | 6 / 9 | 121 / 142 (85,2 %) |
| `dom/abort` | 5 / 6 | 36 / 37 (97,3 %) |
| `dom/collections` | 10 / 12 | 58 / 60 (96,7 %) |
| `dom/events` | 72 / 100 | 483 / 636 (75,9 %) |
| `dom/events/non-cancelable-when-passive` | 42 / 42 | 53 / 53 (100,0 %) |
| `dom/events/scrolling` | 15 / 39 | 38 / 94 (40,4 %) |
| `dom/lists` | 3 / 5 | 180 / 189 (95,2 %) |
| `dom/nodes` | 195 / 265 | 10914 / 13109 (83,3 %) |
| `dom/nodes/Document-contentType/contentType` | 4 / 16 | 4 / 16 (25,0 %) |
| `dom/nodes/insertion-removing-steps` | 13 / 22 | 20 / 38 (52,6 %) |
| `dom/nodes/moveBefore` | 26 / 64 | 100 / 177 (56,5 %) |
| `dom/observable/tentative` | 19 / 27 | 223 / 242 (92,1 %) |
| `dom/ranges` | 38 / 49 | 30715 / 33613 (91,4 %) |
| `dom/ranges/tentative` | 0 / 19 | 0 / 189 (0,0 %) |
| `dom/traversal` | 14 / 18 | 1598 / 1608 (99,4 %) |

## Fichiers non entièrement réussis

| Fichier | Sous-tests | Cause |
|---|---:|---|
| `dom/abort/abort-signal-timeout.html` | 0 / 1 |  |
| `dom/collections/HTMLCollection-as-prototype.html` | 1 / 2 |  |
| `dom/collections/HTMLCollection-supported-property-indices.html` | 6 / 7 |  |
| `dom/events/Body-FrameSet-Event-Handlers.html` | 1 / 5 |  |
| `dom/events/Event-dispatch-click.html` | 18 / 33 | harnais : délai du harnais dépassé |
| `dom/events/Event-dispatch-detached-input-and-change.html` | 8 / 12 |  |
| `dom/events/Event-dispatch-on-disabled-elements.html` | 4 / 9 | harnais : délai du harnais dépassé |
| `dom/events/Event-dispatch-single-activation-behavior.html` | 80 / 132 |  |
| `dom/events/Event-dispatch-throwing-multiple-globals.html` | 0 / 2 |  |
| `dom/events/Event-isTrusted.any.js` | 0 / 1 |  |
| `dom/events/Event-timestamp-high-resolution.https.html` | 0 / 1 |  |
| `dom/events/EventListener-handleEvent-cross-realm.html` | 0 / 5 |  |
| `dom/events/EventListener-incumbent-global-1.sub.html` | 0 / 1 |  |
| `dom/events/EventListener-incumbent-global-2.sub.html` | 0 / 1 |  |
| `dom/events/EventListener-invoke-legacy.html` | 3 / 6 | harnais : délai du harnais dépassé |
| `dom/events/EventTarget-dispatchEvent.html` | 5 / 25 |  |
| `dom/events/click-on-absolute-pseudo.html` | 0 / 1 |  |
| `dom/events/event-global-extra.window.js` | 0 / 6 |  |
| `dom/events/event-global-is-still-set-when-coercing-beforeunload-result.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/events/event-global-is-still-set-when-reporting-exception-onerror.html` | 0 / 1 |  |
| `dom/events/event-listener-not-fully-active-adopted-node.html` | 3 / 5 |  |
| `dom/events/event-listener-not-fully-active-callback.html` | 1 / 4 |  |
| `dom/events/event-listener-not-fully-active-navigated.html` | 0 / 3 |  |
| `dom/events/event-listener-not-fully-active-target.html` | 3 / 16 |  |
| `dom/events/mouse-event-retarget.html` | 0 / 1 |  |
| `dom/events/relatedTarget.window.js` | 0 / 6 |  |
| `dom/events/scrolling/input-text-scroll-event-when-using-arrow-keys.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `dom/events/scrolling/save-iframe-scroll-offset-when-display-none.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `dom/events/scrolling/scroll-cross-origin-iframes.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/events/scrolling/scroll-event-fired-to-iframe.html` | 2 / 4 |  |
| `dom/events/scrolling/scroll-events-fired-once-per-frame-across-display-none.html` | 0 / 2 |  |
| `dom/events/scrolling/scrollend-event-fired-after-layout-clamping.html` | 0 / 1 |  |
| `dom/events/scrolling/scrollend-event-fired-after-sequence-of-scrolls.tentative.html` | 0 / 2 |  |
| `dom/events/scrolling/scrollend-event-fired-for-mandatory-snap-point-after-load.html` | 0 / 1 |  |
| `dom/events/scrolling/scrollend-event-fired-for-programmatic-scroll.html` | 1 / 8 |  |
| `dom/events/scrolling/scrollend-event-fired-for-scroll-attr-change.html` | 2 / 8 | harnais : délai du harnais dépassé |
| `dom/events/scrolling/scrollend-event-fired-for-scrollIntoView.html` | 2 / 5 |  |
| `dom/events/scrolling/scrollend-event-fired-to-document.html` | 0 / 2 | harnais : Uncaught { message: 'assert_true: expected true got false', stack: 'Error' } |
| `dom/events/scrolling/scrollend-event-fires-for-repeat-key-ending-after-scroll-container-end-is-reached.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/events/scrolling/scrollend-event-fires-on-visual-viewport.html` | 0 / 0 | harnais : Unhandled rejection: pinchZoomIn n'est pas défini |
| `dom/events/scrolling/scrollend-event-fires-to-iframe-window.html` | 0 / 6 | harnais : délai du harnais dépassé |
| `dom/events/scrolling/scrollend-event-for-user-scroll.html` | 4 / 5 | harnais : délai du harnais dépassé |
| `dom/events/scrolling/scrollend-event-handler-content-attributes.html` | 2 / 4 | harnais : Uncaught { message: 'assert_true: Scrollend should not be called on: [object Document] expected true got false', stack:  |
| `dom/events/scrolling/scrollend-event-not-fired-on-no-scroll.html` | 1 / 4 | harnais : délai du harnais dépassé |
| `dom/events/scrolling/scrollend-fires-to-text-input.html` | 0 / 5 | harnais : délai du harnais dépassé |
| `dom/events/scrolling/scrollend-with-snap-on-fractional-offset.html` | 0 / 1 |  |
| `dom/events/scrolling/wheel-event-transactions-basic.html` | 1 / 3 |  |
| `dom/events/scrolling/wheel-event-transactions-target-display-change.html` | 1 / 2 |  |
| `dom/events/scrolling/wheel-event-transactions-target-elements.html` | 0 / 1 |  |
| `dom/events/scrolling/wheel-event-transactions-target-resize.html` | 0 / 2 |  |
| `dom/events/shadow-relatedTarget.html` | 0 / 2 |  |
| `dom/events/webkit-animation-end-event.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/webkit-animation-iteration-event.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/webkit-animation-start-event.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/webkit-transition-end-event.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/idlharness.window.js` | 0 / 1 |  |
| `dom/lists/DOMTokenList-coverage-for-attributes.html` | 167 / 175 |  |
| `dom/lists/DOMTokenList-stringifier.html` | 0 / 1 |  |
| `dom/nodes/Attr-prefix.html` | 4 / 6 |  |
| `dom/nodes/CharacterData-surrogates.html` | 2 / 8 |  |
| `dom/nodes/DOMImplementation-createDocument-no-browsing-context.html` | 2 / 5 |  |
| `dom/nodes/DOMImplementation-createDocument.html` | 325 / 434 |  |
| `dom/nodes/DOMImplementation-createHTMLDocument.html` | 12 / 13 |  |
| `dom/nodes/Document-URL.html` | 0 / 1 |  |
| `dom/nodes/Document-adoptNode-DocumentFragment-with-host.window.js` | 5 / 7 |  |
| `dom/nodes/Document-base-url-not-live.window.js` | 0 / 2 |  |
| `dom/nodes/Document-characterSet-normalization-1.html` | 18 / 315 |  |
| `dom/nodes/Document-characterSet-normalization-2.html` | 0 / 339 |  |
| `dom/nodes/Document-cloneNode-about-base-url-relative.window.js` | 0 / 4 |  |
| `dom/nodes/Document-cloneNode-about-base-url.window.js` | 4 / 5 |  |
| `dom/nodes/Document-cloneNode-base-url-xml.window.js` | 0 / 3 |  |
| `dom/nodes/Document-cloneNode-base-url.window.js` | 0 / 11 |  |
| `dom/nodes/Document-cloneNode-srcdoc-base-url.window.js` | 0 / 4 |  |
| `dom/nodes/Document-constructor.html` | 4 / 5 |  |
| `dom/nodes/Document-contentType/contentType/contenttype_bmp.html` | 0 / 1 |  |
| `dom/nodes/Document-contentType/contentType/contenttype_css.html` | 0 / 1 |  |
| `dom/nodes/Document-contentType/contentType/contenttype_datauri_02.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/nodes/Document-contentType/contentType/contenttype_gif.html` | 0 / 1 |  |
| `dom/nodes/Document-contentType/contentType/contenttype_javascripturi.html` | 0 / 1 |  |
| `dom/nodes/Document-contentType/contentType/contenttype_jpg.html` | 0 / 1 |  |
| `dom/nodes/Document-contentType/contentType/contenttype_mimeheader_01.html` | 0 / 1 |  |
| `dom/nodes/Document-contentType/contentType/contenttype_mimeheader_03.html` | 0 / 1 |  |
| `dom/nodes/Document-contentType/contentType/contenttype_png.html` | 0 / 1 |  |
| `dom/nodes/Document-contentType/contentType/contenttype_txt.html` | 0 / 1 |  |
| `dom/nodes/Document-contentType/contentType/contenttype_xml.html` | 0 / 1 |  |
| `dom/nodes/Document-contentType/contentType/xhr_responseType_document.html` | 0 / 1 |  |
| `dom/nodes/Document-createElement-namespace.html` | 14 / 51 |  |
| `dom/nodes/Document-createElement.html` | 49 / 147 |  |
| `dom/nodes/Document-createElementNS.html` | 205 / 596 |  |
| `dom/nodes/Document-createEvent.https.html` | 213 / 279 |  |
| `dom/nodes/Document-getElementsByTagName.html` | 17 / 18 |  |
| `dom/nodes/Element-children.html` | 0 / 2 |  |
| `dom/nodes/Element-classlist.html` | 1415 / 1420 |  |
| `dom/nodes/Element-closest.html` | 28 / 29 |  |
| `dom/nodes/Element-getElementsByTagName-change-document-HTMLNess.html` | 0 / 1 |  |
| `dom/nodes/Element-getElementsByTagName.html` | 18 / 19 |  |
| `dom/nodes/Element-matches-namespaced-elements.html` | 4 / 6 |  |
| `dom/nodes/Element-matches.html` | 594 / 669 |  |
| `dom/nodes/Element-webkitMatchesSelector.html` | 594 / 669 |  |
| `dom/nodes/MutationObserver-attributes.html` | 39 / 42 |  |
| `dom/nodes/MutationObserver-callback-order.window.js` | 2 / 3 |  |
| `dom/nodes/MutationObserver-childList.html` | 33 / 40 |  |
| `dom/nodes/MutationObserver-cross-realm-callback-report-exception.html` | 0 / 1 |  |
| `dom/nodes/MutationObserver-disconnect.html` | 1 / 2 |  |
| `dom/nodes/MutationObserver-document.html` | 1 / 4 |  |
| `dom/nodes/MutationObserver-transient-registered-observers.window.js` | 4 / 15 |  |
| `dom/nodes/Node-appendChild-cereactions-vs-script.window.js` | 0 / 1 |  |
| `dom/nodes/Node-cloneNode-document-allow-declarative-shadow-roots.window.js` | 0 / 1 |  |
| `dom/nodes/Node-cloneNode-svg.html` | 2 / 4 |  |
| `dom/nodes/Node-cloneNode.html` | 114 / 135 |  |
| `dom/nodes/Node-normalize-2.html` | 0 / 2 |  |
| `dom/nodes/Node-properties.html` | 724 / 726 |  |
| `dom/nodes/Node-removeChild.html` | 25 / 28 |  |
| `dom/nodes/NodeList-static-length-getter-tampered-1.html` | 0 / 1 |  |
| `dom/nodes/NodeList-static-length-getter-tampered-2.html` | 0 / 1 |  |
| `dom/nodes/NodeList-static-length-getter-tampered-3.html` | 0 / 1 |  |
| `dom/nodes/NodeList-static-length-getter-tampered-indexOf-1.html` | 0 / 1 |  |
| `dom/nodes/NodeList-static-length-getter-tampered-indexOf-2.html` | 0 / 1 |  |
| `dom/nodes/NodeList-static-length-getter-tampered-indexOf-3.html` | 0 / 1 |  |
| `dom/nodes/ParentNode-querySelector-All.html` | 1581 / 1975 |  |
| `dom/nodes/ParentNode-querySelector-escapes.html` | 63 / 68 |  |
| `dom/nodes/ParentNode-querySelector-hyphen-prefix.html` | 3 / 4 |  |
| `dom/nodes/ParentNode-querySelector-namespace-tagname-attribute.html` | 4 / 8 |  |
| `dom/nodes/ParentNode-querySelector-xml-capitalization.window.js` | 2 / 3 |  |
| `dom/nodes/ParentNode-querySelectors-namespaces.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/nodes/ParentNode-replaceChildren.html` | 50 / 62 | harnais : Uncaught { message: 'assert_equals: mutations.length expected 3 but got 5', stack: 'Error' } |
| `dom/nodes/adoption.window.js` | 2 / 6 |  |
| `dom/nodes/attach-shadow-realm-after-adoption.html` | 2 / 5 |  |
| `dom/nodes/create-element-realm-after-adoption.html` | 2 / 5 |  |
| `dom/nodes/getElementById-namespaced-id.html` | 0 / 4 |  |
| `dom/nodes/insertion-removing-steps/Node-append-form-and-script-from-fragment.html` | 0 / 1 |  |
| `dom/nodes/insertion-removing-steps/Node-append-meta-referrer-and-script-from-fragment.html` | 0 / 1 |  |
| `dom/nodes/insertion-removing-steps/Node-appendChild-script-and-default-style-meta-from-fragment.html` | 0 / 1 |  |
| `dom/nodes/insertion-removing-steps/Node-appendChild-script-and-iframe.html` | 1 / 4 |  |
| `dom/nodes/insertion-removing-steps/Node-appendChild-script-and-source-from-fragment.html` | 0 / 1 |  |
| `dom/nodes/insertion-removing-steps/Node-appendChild-script-and-style.html` | 0 / 5 |  |
| `dom/nodes/insertion-removing-steps/Node-appendChild-text-and-script-in-style.html` | 0 / 1 |  |
| `dom/nodes/insertion-removing-steps/frame-post-connection-steps.html` | 0 / 4 |  |
| `dom/nodes/insertion-removing-steps/insertion-removing-steps-iframe.window.js` | 3 / 4 |  |
| `dom/nodes/moveBefore/continue-css-animation-left.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/nodes/moveBefore/continue-css-animation-transform.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/nodes/moveBefore/continue-css-transition-left-pseudo.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/nodes/moveBefore/continue-css-transition-left.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/nodes/moveBefore/continue-css-transition-transform-pseudo.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/nodes/moveBefore/continue-css-transition-transform.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/nodes/moveBefore/css-animation-commit-styles.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/nodes/moveBefore/css-transition-cross-document.html` | 0 / 0 | le harnais n'a pas terminé |
| `dom/nodes/moveBefore/css-transition-cross-shadow.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/nodes/moveBefore/css-transition-to-disconnected-document.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/nodes/moveBefore/css-transition-trigger.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/nodes/moveBefore/fire-focusin-focusout.html` | 1 / 5 |  |
| `dom/nodes/moveBefore/focus-preserve.html` | 1 / 5 |  |
| `dom/nodes/moveBefore/focus-within.html` | 3 / 5 |  |
| `dom/nodes/moveBefore/form-associated-custom-element-reset.html` | 0 / 4 |  |
| `dom/nodes/moveBefore/fullscreen-preserve.html` | 0 / 1 |  |
| `dom/nodes/moveBefore/hover-style-update.html` | 0 / 2 |  |
| `dom/nodes/moveBefore/iframe-document-preserve.window.js` | 2 / 7 |  |
| `dom/nodes/moveBefore/listed-form-element-reset.html` | 0 / 6 |  |
| `dom/nodes/moveBefore/modal-dialog.html` | 0 / 1 |  |
| `dom/nodes/moveBefore/moveBefore-custom-element-registry.html` | 0 / 4 |  |
| `dom/nodes/moveBefore/moveBefore-image-map.html` | 0 / 3 |  |
| `dom/nodes/moveBefore/moveBefore-name-map.html` | 1 / 3 |  |
| `dom/nodes/moveBefore/moveBefore-radio-groups.html` | 1 / 8 |  |
| `dom/nodes/moveBefore/moveBefore-shadow-root.html` | 1 / 3 |  |
| `dom/nodes/moveBefore/moveBefore-size-query.html` | 0 / 1 |  |
| `dom/nodes/moveBefore/mutation-observer-moved-subtree.window.js` | 0 / 1 |  |
| `dom/nodes/moveBefore/nonce.html` | 0 / 1 |  |
| `dom/nodes/moveBefore/pointer-events.html` | 0 / 1 |  |
| `dom/nodes/moveBefore/popover-preserve.html` | 0 / 1 |  |
| `dom/nodes/moveBefore/preserve-render-blocking-style.html` | 0 / 1 |  |
| `dom/nodes/moveBefore/relevant-mutations.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `dom/nodes/moveBefore/role-updates-after-move.html` | 0 / 1 |  |
| `dom/nodes/moveBefore/select-option-optgroup.html` | 1 / 2 |  |
| `dom/nodes/moveBefore/selection-preserve.html` | 0 / 6 | harnais : délai du harnais dépassé |
| `dom/nodes/moveBefore/slotchange-events.html` | 2 / 4 |  |
| `dom/nodes/moveBefore/stylesheet-cascade-order.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/nodes/moveBefore/stylesheet-no-refetch.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/nodes/namespaced-attribute-mutations.html` | 14 / 24 |  |
| `dom/nodes/node-creation-realm.html` | 11 / 13 |  |
| `dom/nodes/node-realm-adoption-after-frame-removal.html` | 0 / 3 |  |
| `dom/nodes/node-realm-mixed-across-adoption.html` | 0 / 4 |  |
| `dom/nodes/node-realm-preserved-across-adoption.html` | 4 / 5 |  |
| `dom/nodes/node-realm-preserved-across-frameless-adoption.html` | 0 / 4 |  |
| `dom/nodes/processing-instruction-attributes.html` | 10 / 140 |  |
| `dom/nodes/query-target-in-load-event.html` | 0 / 1 |  |
| `dom/nodes/querySelector-mixed-case.html` | 0 / 1 |  |
| `dom/nodes/remove-and-adopt-thcrash.html` | 0 / 1 |  |
| `dom/observable/tentative/idlharness.html` | 0 / 1 |  |
| `dom/observable/tentative/observable-constructor.any.js` | 36 / 44 |  |
| `dom/observable/tentative/observable-constructor.window.js` | 0 / 4 |  |
| `dom/observable/tentative/observable-forEach.window.js` | 1 / 2 |  |
| `dom/observable/tentative/observable-map.window.js` | 0 / 1 |  |
| `dom/observable/tentative/observable-takeUntil.any.js` | 11 / 12 |  |
| `dom/observable/tentative/observable-takeUntil.window.js` | 1 / 2 |  |
| `dom/observable/tentative/observable-toArray.any.js` | 4 / 6 |  |
| `dom/ranges/Range-cloneContents-around-shadow.tentative.html` | 4 / 5 |  |
| `dom/ranges/Range-cloneContents.html` | 117 / 187 |  |
| `dom/ranges/Range-deleteContents.html` | 70 / 125 |  |
| `dom/ranges/Range-expand-shadow.tentative.html` | 0 / 13 |  |
| `dom/ranges/Range-extractContents-around-shadow.tentative.html` | 12 / 13 |  |
| `dom/ranges/Range-extractContents.html` | 129 / 187 |  |
| `dom/ranges/Range-in-shadow-after-the-shadow-removed.html` | 0 / 2 |  |
| `dom/ranges/Range-insertNode.html` | 544 / 1840 |  |
| `dom/ranges/Range-mutations-normalize.html` | 1 / 4 |  |
| `dom/ranges/Range-set.html` | 0 / 0 | le harnais n'a pas terminé |
| `dom/ranges/Range-surroundContents.html` | 441 / 1840 |  |
| `dom/ranges/tentative/OpaqueRange-auto-disconnect.html` | 0 / 17 |  |
| `dom/ranges/tentative/OpaqueRange-basic.html` | 0 / 3 |  |
| `dom/ranges/tentative/OpaqueRange-disconnect.html` | 0 / 5 |  |
| `dom/ranges/tentative/OpaqueRange-display-none.html` | 0 / 18 |  |
| `dom/ranges/tentative/OpaqueRange-geometry-basic.html` | 0 / 14 |  |
| `dom/ranges/tentative/OpaqueRange-geometry-complexity-and-visibility.html` | 0 / 6 |  |
| `dom/ranges/tentative/OpaqueRange-geometry-multiline.html` | 0 / 13 |  |
| `dom/ranges/tentative/OpaqueRange-geometry-mutations.html` | 0 / 12 |  |
| `dom/ranges/tentative/OpaqueRange-highlight.html` | 0 / 8 |  |
| `dom/ranges/tentative/OpaqueRange-highlightsFromPoint.html` | 0 / 11 |  |
| `dom/ranges/tentative/OpaqueRange-interactive-basic.html` | 0 / 16 |  |
| `dom/ranges/tentative/OpaqueRange-interactive-overlap-and-selection.html` | 0 / 10 |  |
| `dom/ranges/tentative/OpaqueRange-offset.html` | 0 / 11 |  |
| `dom/ranges/tentative/OpaqueRange-programmatic-updates.html` | 0 / 28 |  |
| `dom/ranges/tentative/OpaqueRange-range-updates.html` | 0 / 4 |  |
| `dom/ranges/tentative/OpaqueRange-supported-elements.html` | 0 / 6 |  |
| `dom/ranges/tentative/OpaqueRange-unsupported-elements.html` | 0 / 1 |  |
| `dom/ranges/tentative/OpaqueRange-update-event-order.html` | 0 / 4 |  |
| `dom/ranges/tentative/OpaqueRange-validation.html` | 0 / 2 |  |
| `dom/traversal/NodeIterator-removal-during-filtering.html` | 2 / 4 |  |
| `dom/traversal/TreeWalker-acceptNode-filter-cross-realm-null-browsing-context.html` | 0 / 1 |  |
| `dom/traversal/TreeWalker-acceptNode-filter-cross-realm.html` | 0 / 5 |  |
| `dom/traversal/TreeWalker-realm.html` | 0 / 2 |  |
| `dom/xpath-evaluation-detached-nodes.html` | 0 / 19 |  |
| `dom/xpath-result-single-node-value-nullable.html` | 0 / 1 |  |
