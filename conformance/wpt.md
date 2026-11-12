# Web Platform Tests

Produit par `cargo run --release -p oxyde_wpt -- --publier dom`, sur la suite
`web-platform-tests/wpt` déposée dans `vendor/wpt`. Seuls les tests `testharness.js`
destinés à une fenêtre sont exécutés ; un fichier est « entièrement réussi » quand le
harnais va au bout et que tous ses sous-tests passent.

**Sous-tests réussis : 42934 / 49886 (86,1 %)** — fichiers entièrement réussis : 286 / 693 (41,3 %).

Le taux en sous-tests surestime la conformité : un fichier qui plante ou que le harnais
abandonne ne rapporte aucun sous-test, donc n'en fait échouer aucun. Le taux en fichiers
est le plus honnête des deux.

| Dossier | Fichiers réussis | Sous-tests réussis |
|---|---:|---:|
| `dom` | 5 / 9 | 120 / 142 (84,5 %) |
| `dom/abort` | 3 / 6 | 34 / 37 (91,9 %) |
| `dom/collections` | 4 / 12 | 35 / 60 (58,3 %) |
| `dom/events` | 52 / 100 | 340 / 496 (68,5 %) |
| `dom/events/non-cancelable-when-passive` | 1 / 42 | 9 / 13 (69,2 %) |
| `dom/events/scrolling` | 0 / 39 | 12 / 103 (11,7 %) |
| `dom/lists` | 3 / 5 | 180 / 189 (95,2 %) |
| `dom/nodes` | 155 / 265 | 10324 / 13058 (79,1 %) |
| `dom/nodes/Document-contentType/contentType` | 4 / 16 | 4 / 16 (25,0 %) |
| `dom/nodes/insertion-removing-steps` | 3 / 22 | 6 / 37 (16,2 %) |
| `dom/nodes/moveBefore` | 14 / 64 | 61 / 168 (36,3 %) |
| `dom/observable/tentative` | 0 / 27 | 0 / 242 (0,0 %) |
| `dom/ranges` | 29 / 49 | 30212 / 33528 (90,1 %) |
| `dom/ranges/tentative` | 0 / 19 | 0 / 189 (0,0 %) |
| `dom/traversal` | 13 / 18 | 1597 / 1608 (99,3 %) |

## Fichiers non entièrement réussis

| Fichier | Sous-tests | Cause |
|---|---:|---|
| `dom/abort/abort-signal-any.any.js` | 13 / 14 |  |
| `dom/abort/abort-signal-timeout.html` | 0 / 1 |  |
| `dom/abort/event.any.js` | 15 / 16 |  |
| `dom/collections/HTMLCollection-as-prototype.html` | 1 / 2 |  |
| `dom/collections/HTMLCollection-delete.html` | 2 / 4 |  |
| `dom/collections/HTMLCollection-iterator.html` | 3 / 6 |  |
| `dom/collections/HTMLCollection-own-props.html` | 4 / 8 |  |
| `dom/collections/HTMLCollection-supported-property-indices.html` | 3 / 7 |  |
| `dom/collections/HTMLCollection-supported-property-names.html` | 3 / 6 |  |
| `dom/collections/domstringmap-supported-property-names.html` | 0 / 5 |  |
| `dom/collections/namednodemap-supported-property-names.html` | 0 / 3 |  |
| `dom/events/AddEventListenerOptions-signal.any.js` | 4 / 11 |  |
| `dom/events/Body-FrameSet-Event-Handlers.html` | 0 / 5 |  |
| `dom/events/Event-dispatch-click.html` | 11 / 33 | harnais : délai du harnais dépassé |
| `dom/events/Event-dispatch-detached-input-and-change.html` | 4 / 12 |  |
| `dom/events/Event-dispatch-listener-order.window.js` | 0 / 1 |  |
| `dom/events/Event-dispatch-on-disabled-elements.html` | 4 / 9 | harnais : délai du harnais dépassé |
| `dom/events/Event-dispatch-redispatch.html` | 2 / 4 | harnais : délai du harnais dépassé |
| `dom/events/Event-dispatch-single-activation-behavior.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/Event-dispatch-throwing-multiple-globals.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `dom/events/Event-dispatch-throwing.html` | 0 / 2 |  |
| `dom/events/Event-init-while-dispatching.html` | 2 / 5 |  |
| `dom/events/Event-isTrusted.any.js` | 0 / 1 |  |
| `dom/events/Event-stopPropagation-cancel-bubbling.html` | 0 / 1 |  |
| `dom/events/Event-subclasses-constructors.html` | 42 / 49 |  |
| `dom/events/Event-timestamp-high-resolution.https.html` | 0 / 1 |  |
| `dom/events/Event-timestamp-safe-resolution.html` | 0 / 1 |  |
| `dom/events/EventListener-handleEvent-cross-realm.html` | 0 / 5 |  |
| `dom/events/EventListener-handleEvent.html` | 3 / 6 |  |
| `dom/events/EventListener-incumbent-global-1.sub.html` | 0 / 1 |  |
| `dom/events/EventListener-incumbent-global-2.sub.html` | 0 / 1 |  |
| `dom/events/EventListener-invoke-legacy.html` | 3 / 6 | harnais : délai du harnais dépassé |
| `dom/events/EventTarget-add-listener-platform-object.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/events/EventTarget-dispatchEvent.html` | 5 / 25 |  |
| `dom/events/click-on-absolute-pseudo.html` | 0 / 1 |  |
| `dom/events/event-global-event-targets.html` | 0 / 5 |  |
| `dom/events/event-global-extra.window.js` | 0 / 6 |  |
| `dom/events/event-global-is-still-set-when-coercing-beforeunload-result.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `dom/events/event-global-is-still-set-when-reporting-exception-onerror.html` | 0 / 1 |  |
| `dom/events/event-global-set-before-handleEvent-lookup.window.js` | 0 / 1 |  |
| `dom/events/event-global.html` | 0 / 8 |  |
| `dom/events/event-listener-not-fully-active-adopted-node.html` | 3 / 5 |  |
| `dom/events/event-listener-not-fully-active-callback.html` | 1 / 4 |  |
| `dom/events/event-listener-not-fully-active-navigated.html` | 0 / 3 |  |
| `dom/events/event-listener-not-fully-active-target.html` | 3 / 16 |  |
| `dom/events/focus-event-document-move.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/events/handler-count.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `dom/events/mouse-event-retarget.html` | 0 / 1 |  |
| `dom/events/no-focus-events-at-clicking-editable-content-in-link.html` | 0 / 2 |  |
| `dom/events/non-cancelable-when-passive/non-passive-mousewheel-event-listener-on-body.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/non-passive-mousewheel-event-listener-on-div.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/non-passive-mousewheel-event-listener-on-document.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/non-passive-mousewheel-event-listener-on-root.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/non-passive-mousewheel-event-listener-on-window.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/non-passive-touchmove-event-listener-on-body.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/non-passive-touchmove-event-listener-on-div.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/non-passive-touchmove-event-listener-on-document.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/non-passive-touchmove-event-listener-on-root.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/non-passive-touchmove-event-listener-on-window.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/non-passive-touchstart-event-listener-on-body.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/non-passive-touchstart-event-listener-on-div.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/non-passive-touchstart-event-listener-on-document.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/non-passive-touchstart-event-listener-on-root.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/non-passive-touchstart-event-listener-on-window.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/non-passive-wheel-event-listener-on-body.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/non-passive-wheel-event-listener-on-div.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/non-passive-wheel-event-listener-on-document.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/non-passive-wheel-event-listener-on-root.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/non-passive-wheel-event-listener-on-window.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/passive-mousewheel-event-listener-on-body.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/passive-mousewheel-event-listener-on-div.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/passive-mousewheel-event-listener-on-document.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/passive-mousewheel-event-listener-on-root.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/passive-mousewheel-event-listener-on-window.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/passive-touchmove-event-listener-on-body.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/passive-touchmove-event-listener-on-div.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/passive-touchmove-event-listener-on-document.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/passive-touchmove-event-listener-on-root.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/passive-touchmove-event-listener-on-window.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/passive-touchstart-event-listener-on-body.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/passive-touchstart-event-listener-on-div.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/passive-touchstart-event-listener-on-document.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/passive-touchstart-event-listener-on-root.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/passive-touchstart-event-listener-on-window.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/passive-wheel-event-listener-on-body.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/passive-wheel-event-listener-on-div.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/passive-wheel-event-listener-on-document.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/passive-wheel-event-listener-on-root.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/passive-wheel-event-listener-on-window.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/non-cancelable-when-passive/synthetic-events-cancelable.html` | 8 / 12 |  |
| `dom/events/passive-by-default-non-node.html` | 1 / 2 |  |
| `dom/events/pointer-event-document-move.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/preventDefault-during-activation-behavior.html` | 0 / 1 |  |
| `dom/events/relatedTarget.window.js` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/scrolling/iframe-chains.html` | 0 / 1 |  |
| `dom/events/scrolling/input-text-scroll-event-when-using-arrow-keys.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `dom/events/scrolling/save-iframe-scroll-offset-when-display-none.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `dom/events/scrolling/scroll-cross-origin-iframes.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/events/scrolling/scroll-event-fired-to-element.html` | 2 / 4 |  |
| `dom/events/scrolling/scroll-event-fired-to-iframe.html` | 2 / 4 |  |
| `dom/events/scrolling/scroll-event-not-fired-on-ancestor-style-change.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/events/scrolling/scroll-events-fired-once-per-frame-across-display-none.html` | 0 / 2 |  |
| `dom/events/scrolling/scrollIntoView-in-onscroll-to-sticky.html` | 0 / 1 |  |
| `dom/events/scrolling/scrollend-event-fired-after-instant-scroll-in-microtask.html` | 0 / 1 |  |
| `dom/events/scrolling/scrollend-event-fired-after-layout-clamping.html` | 0 / 1 |  |
| `dom/events/scrolling/scrollend-event-fired-after-sequence-of-scrolls.tentative.html` | 0 / 2 |  |
| `dom/events/scrolling/scrollend-event-fired-after-snap.html` | 0 / 2 |  |
| `dom/events/scrolling/scrollend-event-fired-for-mandatory-snap-point-after-load.html` | 0 / 1 |  |
| `dom/events/scrolling/scrollend-event-fired-for-programmatic-scroll.html` | 0 / 8 |  |
| `dom/events/scrolling/scrollend-event-fired-for-scroll-attr-change.html` | 0 / 8 |  |
| `dom/events/scrolling/scrollend-event-fired-for-scrollIntoView.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/scrolling/scrollend-event-fired-to-document.html` | 0 / 2 |  |
| `dom/events/scrolling/scrollend-event-fired-to-element-with-overscroll-behavior.html` | 0 / 1 |  |
| `dom/events/scrolling/scrollend-event-fired-to-window.html` | 0 / 1 |  |
| `dom/events/scrolling/scrollend-event-fires-for-repeat-key-ending-after-scroll-container-end-is-reached.html` | 0 / 1 |  |
| `dom/events/scrolling/scrollend-event-fires-on-visual-viewport.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/scrolling/scrollend-event-fires-to-iframe-window.html` | 6 / 18 | harnais : 6 duplicate test names: "Tests that the target_div within iframe gets scrollend event when touch dragging.", "Tests that |
| `dom/events/scrolling/scrollend-event-for-user-scroll.html` | 2 / 5 |  |
| `dom/events/scrolling/scrollend-event-handler-content-attributes.html` | 0 / 4 |  |
| `dom/events/scrolling/scrollend-event-not-fired-after-removing-scroller.tentative.html` | 0 / 3 |  |
| `dom/events/scrolling/scrollend-event-not-fired-on-no-scroll.html` | 0 / 4 | harnais : Test named 'No scroll via wheel on div shouldn't fire scrollend.' specified 1 'cleanup' function, and 1 failed. |
| `dom/events/scrolling/scrollend-fires-to-text-input.html` | 0 / 5 | harnais : délai du harnais dépassé |
| `dom/events/scrolling/scrollend-with-snap-on-fractional-offset.html` | 0 / 1 |  |
| `dom/events/scrolling/visibility-hidden-scrolls.html` | 0 / 2 |  |
| `dom/events/scrolling/wheel-event-composed.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/events/scrolling/wheel-event-no-scroll-after-prevent-default.html` | 0 / 1 |  |
| `dom/events/scrolling/wheel-event-transactions-basic.html` | 0 / 3 |  |
| `dom/events/scrolling/wheel-event-transactions-multiple-action-chains.html` | 0 / 1 |  |
| `dom/events/scrolling/wheel-event-transactions-target-display-change.html` | 0 / 2 |  |
| `dom/events/scrolling/wheel-event-transactions-target-elements.html` | 0 / 1 |  |
| `dom/events/scrolling/wheel-event-transactions-target-move.html` | 0 / 1 |  |
| `dom/events/scrolling/wheel-event-transactions-target-removal.html` | 0 / 1 |  |
| `dom/events/scrolling/wheel-event-transactions-target-resize.html` | 0 / 2 |  |
| `dom/events/shadow-relatedTarget.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/webkit-animation-end-event.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/webkit-animation-iteration-event.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/webkit-animation-start-event.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/webkit-transition-end-event.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/events/window-event-restored-after-throwing-onerror.html` | 0 / 1 |  |
| `dom/idlharness.window.js` | 0 / 1 |  |
| `dom/lists/DOMTokenList-coverage-for-attributes.html` | 167 / 175 |  |
| `dom/lists/DOMTokenList-stringifier.html` | 0 / 1 |  |
| `dom/nodes/Attr-prefix.html` | 4 / 6 |  |
| `dom/nodes/CharacterData-appendData.html` | 12 / 14 |  |
| `dom/nodes/CharacterData-substringData.html` | 26 / 28 |  |
| `dom/nodes/CharacterData-surrogates.html` | 2 / 8 |  |
| `dom/nodes/DOMImplementation-createDocument-no-browsing-context.html` | 2 / 5 |  |
| `dom/nodes/DOMImplementation-createDocument.html` | 325 / 434 |  |
| `dom/nodes/DOMImplementation-createHTMLDocument.html` | 12 / 13 |  |
| `dom/nodes/Document-URL.html` | 0 / 1 |  |
| `dom/nodes/Document-adoptNode-DocumentFragment-with-host.window.js` | 1 / 7 |  |
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
| `dom/nodes/Document-createElement-namespace.html` | 13 / 51 |  |
| `dom/nodes/Document-createElement.html` | 49 / 147 |  |
| `dom/nodes/Document-createElementNS.html` | 205 / 596 |  |
| `dom/nodes/Document-createEvent.https.html` | 213 / 279 |  |
| `dom/nodes/Document-getElementById.html` | 16 / 18 |  |
| `dom/nodes/Document-getElementsByTagName.html` | 15 / 18 |  |
| `dom/nodes/DocumentFragment-getElementById.html` | 4 / 5 |  |
| `dom/nodes/Element-children.html` | 0 / 2 |  |
| `dom/nodes/Element-classlist.html` | 1415 / 1420 |  |
| `dom/nodes/Element-closest.html` | 24 / 29 |  |
| `dom/nodes/Element-getElementsByTagName-change-document-HTMLNess.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/nodes/Element-getElementsByTagName.html` | 16 / 19 |  |
| `dom/nodes/Element-matches-complex-nth-last-of-type.html` | 0 / 1 |  |
| `dom/nodes/Element-matches-disabled-pseudo-class.html` | 0 / 11 |  |
| `dom/nodes/Element-matches-namespaced-elements.html` | 4 / 6 |  |
| `dom/nodes/Element-matches.html` | 512 / 669 |  |
| `dom/nodes/Element-webkitMatchesSelector.html` | 512 / 669 |  |
| `dom/nodes/MutationObserver-attributes.html` | 39 / 42 |  |
| `dom/nodes/MutationObserver-callback-arguments.html` | 0 / 1 |  |
| `dom/nodes/MutationObserver-callback-order.window.js` | 2 / 3 |  |
| `dom/nodes/MutationObserver-callback-tricky.html` | 0 / 2 |  |
| `dom/nodes/MutationObserver-childList.html` | 33 / 40 |  |
| `dom/nodes/MutationObserver-cross-realm-callback-report-exception.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/nodes/MutationObserver-disconnect.html` | 1 / 2 |  |
| `dom/nodes/MutationObserver-document.html` | 1 / 4 |  |
| `dom/nodes/MutationObserver-inner-outer.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `dom/nodes/MutationObserver-textContent.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `dom/nodes/MutationObserver-transient-registered-observers.window.js` | 3 / 15 |  |
| `dom/nodes/Node-appendChild-cereactions-vs-script.window.js` | 0 / 1 |  |
| `dom/nodes/Node-appendChild.html` | 8 / 11 |  |
| `dom/nodes/Node-cloneNode-document-allow-declarative-shadow-roots.window.js` | 0 / 1 |  |
| `dom/nodes/Node-cloneNode-document-with-doctype.html` | 2 / 3 |  |
| `dom/nodes/Node-cloneNode-svg.html` | 2 / 4 |  |
| `dom/nodes/Node-cloneNode.html` | 113 / 135 |  |
| `dom/nodes/Node-isConnected-shadow-dom.html` | 0 / 2 |  |
| `dom/nodes/Node-isEqualNode.html` | 6 / 9 |  |
| `dom/nodes/Node-normalize-2.html` | 0 / 2 |  |
| `dom/nodes/Node-normalize.html` | 3 / 4 |  |
| `dom/nodes/Node-properties.html` | 724 / 726 |  |
| `dom/nodes/Node-removeChild.html` | 19 / 28 |  |
| `dom/nodes/Node-textContent.html` | 73 / 81 |  |
| `dom/nodes/NodeList-static-length-getter-tampered-1.html` | 0 / 1 |  |
| `dom/nodes/NodeList-static-length-getter-tampered-2.html` | 0 / 1 |  |
| `dom/nodes/NodeList-static-length-getter-tampered-3.html` | 0 / 1 |  |
| `dom/nodes/NodeList-static-length-getter-tampered-indexOf-1.html` | 0 / 1 |  |
| `dom/nodes/NodeList-static-length-getter-tampered-indexOf-2.html` | 0 / 1 |  |
| `dom/nodes/NodeList-static-length-getter-tampered-indexOf-3.html` | 0 / 1 |  |
| `dom/nodes/ParentNode-querySelector-All-scope.html` | 0 / 8 |  |
| `dom/nodes/ParentNode-querySelector-All.html` | 1321 / 1975 |  |
| `dom/nodes/ParentNode-querySelector-basic-has.html` | 0 / 1 |  |
| `dom/nodes/ParentNode-querySelector-dupe-id.html` | 0 / 2 |  |
| `dom/nodes/ParentNode-querySelector-escapes.html` | 63 / 68 |  |
| `dom/nodes/ParentNode-querySelector-host-pseudo-class.html` | 0 / 1 |  |
| `dom/nodes/ParentNode-querySelector-hyphen-prefix.html` | 2 / 4 |  |
| `dom/nodes/ParentNode-querySelector-invalid-comment-like.html` | 0 / 3 |  |
| `dom/nodes/ParentNode-querySelector-namespace-tagname-attribute.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/nodes/ParentNode-querySelector-nth-of-type-complex.html` | 0 / 1 |  |
| `dom/nodes/ParentNode-querySelector-scope-in-not.html` | 0 / 1 |  |
| `dom/nodes/ParentNode-querySelector-scope.html` | 2 / 4 |  |
| `dom/nodes/ParentNode-querySelector-shadowroot-nth-child.html` | 0 / 1 |  |
| `dom/nodes/ParentNode-querySelector-xml-capitalization.window.js` | 0 / 3 |  |
| `dom/nodes/ParentNode-querySelectorAll-namespace.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/nodes/ParentNode-querySelectorAll-nested-not-has.html` | 0 / 1 |  |
| `dom/nodes/ParentNode-querySelectorAll-scope-change.html` | 0 / 1 |  |
| `dom/nodes/ParentNode-querySelectorAll-scope-colon-id-class.html` | 0 / 2 |  |
| `dom/nodes/ParentNode-querySelectors-namespaces.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/nodes/ParentNode-replaceChildren.html` | 25 / 31 | harnais : délai du harnais dépassé |
| `dom/nodes/adoption.window.js` | 2 / 6 |  |
| `dom/nodes/attach-shadow-realm-after-adoption.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/nodes/attributes.html` | 59 / 67 |  |
| `dom/nodes/create-element-realm-after-adoption.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/nodes/getElementById-namespaced-id.html` | 0 / 4 |  |
| `dom/nodes/getElementsByClassName-20.htm` | 0 / 1 |  |
| `dom/nodes/getElementsByClassName-21.htm` | 0 / 1 |  |
| `dom/nodes/getElementsByClassName-22.htm` | 0 / 1 |  |
| `dom/nodes/getElementsByClassName-25.htm` | 0 / 1 |  |
| `dom/nodes/insertion-removing-steps/Node-append-form-and-script-from-fragment.html` | 0 / 1 |  |
| `dom/nodes/insertion-removing-steps/Node-append-meta-referrer-and-script-from-fragment.html` | 0 / 1 |  |
| `dom/nodes/insertion-removing-steps/Node-appendChild-script-and-button-from-div.html` | 0 / 1 |  |
| `dom/nodes/insertion-removing-steps/Node-appendChild-script-and-custom-from-fragment.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/nodes/insertion-removing-steps/Node-appendChild-script-and-default-style-meta-from-fragment.html` | 0 / 1 |  |
| `dom/nodes/insertion-removing-steps/Node-appendChild-script-and-div-from-fragment.html` | 0 / 1 |  |
| `dom/nodes/insertion-removing-steps/Node-appendChild-script-and-iframe.html` | 0 / 4 |  |
| `dom/nodes/insertion-removing-steps/Node-appendChild-script-and-source-from-fragment.html` | 0 / 1 |  |
| `dom/nodes/insertion-removing-steps/Node-appendChild-script-and-style.html` | 0 / 5 |  |
| `dom/nodes/insertion-removing-steps/Node-appendChild-script-in-script.html` | 0 / 1 |  |
| `dom/nodes/insertion-removing-steps/Node-appendChild-script-with-mutation-observer-takeRecords.html` | 0 / 1 |  |
| `dom/nodes/insertion-removing-steps/Node-appendChild-text-and-script-in-style.html` | 0 / 1 |  |
| `dom/nodes/insertion-removing-steps/Node-appendChild-text-in-script.html` | 0 / 1 |  |
| `dom/nodes/insertion-removing-steps/Node-appendChild-three-scripts-from-fragment.html` | 0 / 1 |  |
| `dom/nodes/insertion-removing-steps/Node-appendChild-three-scripts.html` | 0 / 1 |  |
| `dom/nodes/insertion-removing-steps/change-event.window.js` | 0 / 2 |  |
| `dom/nodes/insertion-removing-steps/frame-post-connection-steps.html` | 0 / 4 |  |
| `dom/nodes/insertion-removing-steps/insertion-removing-steps-iframe.window.js` | 1 / 4 |  |
| `dom/nodes/insertion-removing-steps/insertion-removing-steps-script.window.js` | 0 / 1 |  |
| `dom/nodes/live-collections-batched-mutations.window.js` | 0 / 2 |  |
| `dom/nodes/moveBefore/Node-moveBefore.html` | 29 / 32 |  |
| `dom/nodes/moveBefore/continue-css-animation-left.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/nodes/moveBefore/continue-css-animation-transform.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/nodes/moveBefore/continue-css-transition-left-pseudo.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/nodes/moveBefore/continue-css-transition-left.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/nodes/moveBefore/continue-css-transition-transform-pseudo.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/nodes/moveBefore/continue-css-transition-transform.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/nodes/moveBefore/css-animation-commit-styles.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/nodes/moveBefore/css-transition-cross-document.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/nodes/moveBefore/css-transition-cross-shadow.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/nodes/moveBefore/css-transition-to-disconnected-document.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/nodes/moveBefore/css-transition-trigger.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/nodes/moveBefore/custom-element-move-reactions.html` | 0 / 7 |  |
| `dom/nodes/moveBefore/fire-focusin-focusout.html` | 0 / 5 |  |
| `dom/nodes/moveBefore/focus-preserve.html` | 0 / 5 |  |
| `dom/nodes/moveBefore/focus-within.html` | 0 / 5 |  |
| `dom/nodes/moveBefore/form-associated-custom-element-reset.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/nodes/moveBefore/fullscreen-preserve.html` | 0 / 1 |  |
| `dom/nodes/moveBefore/hover-style-update.html` | 0 / 2 |  |
| `dom/nodes/moveBefore/iframe-document-preserve.window.js` | 2 / 7 | harnais : délai du harnais dépassé |
| `dom/nodes/moveBefore/listed-form-element-reset.html` | 0 / 6 |  |
| `dom/nodes/moveBefore/modal-dialog.html` | 0 / 1 |  |
| `dom/nodes/moveBefore/moveBefore-child-list-selector-invalidation.html` | 8 / 9 |  |
| `dom/nodes/moveBefore/moveBefore-custom-element-registry.html` | 0 / 4 |  |
| `dom/nodes/moveBefore/moveBefore-dir.html` | 0 / 1 |  |
| `dom/nodes/moveBefore/moveBefore-disconnected-id-map.html` | 1 / 4 |  |
| `dom/nodes/moveBefore/moveBefore-from-light-to-shadow.html` | 0 / 1 |  |
| `dom/nodes/moveBefore/moveBefore-id-map.html` | 1 / 4 |  |
| `dom/nodes/moveBefore/moveBefore-image-map.html` | 0 / 3 |  |
| `dom/nodes/moveBefore/moveBefore-lang.html` | 0 / 1 |  |
| `dom/nodes/moveBefore/moveBefore-name-map.html` | 0 / 3 |  |
| `dom/nodes/moveBefore/moveBefore-radio-groups.html` | 0 / 8 |  |
| `dom/nodes/moveBefore/moveBefore-shadow-boundary-id-change.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/nodes/moveBefore/moveBefore-shadow-host-id-map.html` | 0 / 4 |  |
| `dom/nodes/moveBefore/moveBefore-shadow-inside.html` | 0 / 1 |  |
| `dom/nodes/moveBefore/moveBefore-shadow-root.html` | 0 / 3 |  |
| `dom/nodes/moveBefore/moveBefore-size-query.html` | 0 / 1 |  |
| `dom/nodes/moveBefore/mutation-observer-moved-subtree.window.js` | 0 / 1 |  |
| `dom/nodes/moveBefore/mutation-observer.html` | 0 / 2 |  |
| `dom/nodes/moveBefore/nonce.html` | 0 / 1 |  |
| `dom/nodes/moveBefore/pointer-events.html` | 0 / 1 |  |
| `dom/nodes/moveBefore/popover-preserve.html` | 0 / 1 |  |
| `dom/nodes/moveBefore/preserve-render-blocking-style.html` | 0 / 1 |  |
| `dom/nodes/moveBefore/relevant-mutations.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `dom/nodes/moveBefore/role-updates-after-move.html` | 0 / 1 |  |
| `dom/nodes/moveBefore/select-option-optgroup.html` | 1 / 2 |  |
| `dom/nodes/moveBefore/selection-preserve.html` | 0 / 6 |  |
| `dom/nodes/moveBefore/slotchange-events.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `dom/nodes/moveBefore/stylesheet-cascade-order.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/nodes/moveBefore/stylesheet-no-refetch.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/nodes/namespaced-attribute-mutations.html` | 14 / 24 |  |
| `dom/nodes/node-creation-realm.html` | 10 / 13 |  |
| `dom/nodes/node-realm-adoption-after-frame-removal.html` | 0 / 3 |  |
| `dom/nodes/node-realm-mixed-across-adoption.html` | 0 / 4 |  |
| `dom/nodes/node-realm-preserved-across-adoption.html` | 4 / 5 |  |
| `dom/nodes/node-realm-preserved-across-frameless-adoption.html` | 0 / 4 |  |
| `dom/nodes/post-connection-steps-shadow-tree.html` | 0 / 1 |  |
| `dom/nodes/processing-instruction-attributes.html` | 0 / 140 |  |
| `dom/nodes/query-target-in-load-event.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `dom/nodes/querySelector-mixed-case.html` | 0 / 1 |  |
| `dom/nodes/remove-and-adopt-thcrash.html` | 0 / 1 |  |
| `dom/nodes/remove-next-sibling-during-replace-with.html` | 0 / 1 |  |
| `dom/nodes/remove-unscopable.html` | 0 / 6 |  |
| `dom/nodes/rootNode.html` | 4 / 5 |  |
| `dom/nodes/svg-template-querySelector.html` | 0 / 3 |  |
| `dom/observable/tentative/idlharness.html` | 0 / 1 |  |
| `dom/observable/tentative/observable-catch.any.js` | 0 / 9 |  |
| `dom/observable/tentative/observable-constructor.any.js` | 0 / 44 |  |
| `dom/observable/tentative/observable-constructor.window.js` | 0 / 4 |  |
| `dom/observable/tentative/observable-drop.any.js` | 0 / 7 |  |
| `dom/observable/tentative/observable-event-target.any.js` | 0 / 3 |  |
| `dom/observable/tentative/observable-event-target.window.js` | 0 / 2 |  |
| `dom/observable/tentative/observable-every.any.js` | 0 / 10 |  |
| `dom/observable/tentative/observable-filter.any.js` | 0 / 6 |  |
| `dom/observable/tentative/observable-finally.any.js` | 0 / 10 |  |
| `dom/observable/tentative/observable-find.any.js` | 0 / 6 |  |
| `dom/observable/tentative/observable-first.any.js` | 0 / 5 |  |
| `dom/observable/tentative/observable-flatMap.any.js` | 0 / 7 |  |
| `dom/observable/tentative/observable-forEach.any.js` | 0 / 6 |  |
| `dom/observable/tentative/observable-forEach.window.js` | 0 / 2 |  |
| `dom/observable/tentative/observable-from.any.js` | 0 / 48 |  |
| `dom/observable/tentative/observable-inspect.any.js` | 0 / 13 |  |
| `dom/observable/tentative/observable-last.any.js` | 0 / 5 |  |
| `dom/observable/tentative/observable-map.any.js` | 0 / 6 |  |
| `dom/observable/tentative/observable-map.window.js` | 0 / 1 |  |
| `dom/observable/tentative/observable-reduce.any.js` | 0 / 8 |  |
| `dom/observable/tentative/observable-some.any.js` | 0 / 7 |  |
| `dom/observable/tentative/observable-switchMap.any.js` | 0 / 6 |  |
| `dom/observable/tentative/observable-take.any.js` | 0 / 6 |  |
| `dom/observable/tentative/observable-takeUntil.any.js` | 0 / 12 |  |
| `dom/observable/tentative/observable-takeUntil.window.js` | 0 / 2 |  |
| `dom/observable/tentative/observable-toArray.any.js` | 0 / 6 |  |
| `dom/ranges/Range-cloneContents-around-shadow.html` | 0 / 9 |  |
| `dom/ranges/Range-cloneContents-around-shadow.tentative.html` | 0 / 3 |  |
| `dom/ranges/Range-cloneContents-in-ShadowRoot.html` | 0 / 4 |  |
| `dom/ranges/Range-cloneContents.html` | 109 / 187 |  |
| `dom/ranges/Range-deleteContents-around-shadow.html` | 0 / 9 |  |
| `dom/ranges/Range-deleteContents-around-shadow.tentative.html` | 0 / 2 |  |
| `dom/ranges/Range-deleteContents-in-ShadowRoot.html` | 0 / 4 |  |
| `dom/ranges/Range-deleteContents.html` | 65 / 125 |  |
| `dom/ranges/Range-expand-shadow.tentative.html` | 0 / 13 |  |
| `dom/ranges/Range-extractContents-around-shadow.html` | 0 / 9 |  |
| `dom/ranges/Range-extractContents-around-shadow.tentative.html` | 0 / 3 |  |
| `dom/ranges/Range-extractContents-in-ShadowRoot.html` | 0 / 4 |  |
| `dom/ranges/Range-extractContents.html` | 123 / 187 |  |
| `dom/ranges/Range-in-shadow-after-the-shadow-removed.html` | 0 / 2 |  |
| `dom/ranges/Range-insertNode.html` | 360 / 1840 |  |
| `dom/ranges/Range-intersectsNode-shadow.html` | 0 / 1 |  |
| `dom/ranges/Range-mutations-normalize.html` | 1 / 4 |  |
| `dom/ranges/Range-set.html` | 0 / 0 | le harnais n'a pas terminé |
| `dom/ranges/Range-surroundContents.html` | 289 / 1840 |  |
| `dom/ranges/StaticRange-constructor.html` | 0 / 17 |  |
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
| `dom/traversal/TreeWalker-nextNode-detached-currentNode.window.js` | 2 / 3 |  |
| `dom/traversal/TreeWalker-realm.html` | 0 / 2 |  |
| `dom/window-extends-event-target.html` | 2 / 3 |  |
| `dom/xpath-evaluation-detached-nodes.html` | 0 / 19 |  |
| `dom/xpath-result-single-node-value-nullable.html` | 0 / 1 |  |
