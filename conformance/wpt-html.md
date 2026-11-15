# Web Platform Tests

Produit par `cargo run --release -p oxyde_wpt -- --publier html`, sur la suite
`web-platform-tests/wpt` déposée dans `vendor/wpt`. Seuls les tests `testharness.js`
destinés à une fenêtre sont exécutés ; un fichier est « entièrement réussi » quand le
harnais va au bout et que tous ses sous-tests passent.

**Sous-tests réussis : 48221 / 123964 (38,9 %)** — fichiers entièrement réussis : 2262 / 7308 (31,0 %).

Le taux en sous-tests surestime la conformité : un fichier qui plante ou que le harnais
abandonne ne rapporte aucun sous-test, donc n'en fait échouer aucun. Le taux en fichiers
est le plus honnête des deux.

| Dossier | Fichiers réussis | Sous-tests réussis |
|---|---:|---:|
| `html/anonymous-iframe` | 0 / 18 | 0 / 24 (0,0 %) |
| `html/browsers/browsing-the-web/back-forward-cache` | 0 / 12 | 0 / 17 (0,0 %) |
| `html/browsers/browsing-the-web/back-forward-cache/broadcastchannel` | 0 / 3 | 0 / 3 (0,0 %) |
| `html/browsers/browsing-the-web/back-forward-cache/eligibility` | 0 / 8 | 0 / 16 (0,0 %) |
| `html/browsers/browsing-the-web/history-traversal` | 3 / 27 | 23 / 53 (43,4 %) |
| `html/browsers/browsing-the-web/history-traversal/event-order` | 3 / 12 | 3 / 12 (25,0 %) |
| `html/browsers/browsing-the-web/history-traversal/pagereveal` | 0 / 5 | 0 / 4 (0,0 %) |
| `html/browsers/browsing-the-web/history-traversal/pageswap` | 0 / 13 | 2 / 14 (14,3 %) |
| `html/browsers/browsing-the-web/history-traversal/persisted-user-state-restoration` | 0 / 6 | 2 / 9 (22,2 %) |
| `html/browsers/browsing-the-web/history-traversal/srcdoc` | 0 / 2 | 0 / 4 (0,0 %) |
| `html/browsers/browsing-the-web/navigating-across-documents` | 6 / 65 | 16 / 143 (11,2 %) |
| `html/browsers/browsing-the-web/navigating-across-documents/initial-empty-document` | 1 / 16 | 13 / 71 (18,3 %) |
| `html/browsers/browsing-the-web/navigating-across-documents/javascript-url-abort` | 0 / 2 | 0 / 2 (0,0 %) |
| `html/browsers/browsing-the-web/navigating-across-documents/multiple-globals` | 0 / 4 | 0 / 4 (0,0 %) |
| `html/browsers/browsing-the-web/navigating-across-documents/refresh` | 1 / 5 | 1 / 7 (14,3 %) |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load` | 0 / 41 | 0 / 51 (0,0 %) |
| `html/browsers/browsing-the-web/navigating-across-documents/source` | 0 / 5 | 0 / 5 (0,0 %) |
| `html/browsers/browsing-the-web/overlapping-navigations-and-traversals` | 3 / 25 | 7 / 47 (14,9 %) |
| `html/browsers/browsing-the-web/read-media` | 0 / 4 | 0 / 4 (0,0 %) |
| `html/browsers/browsing-the-web/read-text` | 0 / 1 | 0 / 3 (0,0 %) |
| `html/browsers/browsing-the-web/remote-context-helper-tests` | 3 / 29 | 3 / 29 (10,3 %) |
| `html/browsers/browsing-the-web/scroll-to-fragid` | 6 / 29 | 11 / 43 (25,6 %) |
| `html/browsers/browsing-the-web/unloading-documents` | 1 / 16 | 4 / 42 (9,5 %) |
| `html/browsers/browsing-the-web/unloading-documents/prompt` | 2 / 4 | 2 / 4 (50,0 %) |
| `html/browsers/browsing-the-web/unloading-documents/unload` | 0 / 10 | 0 / 10 (0,0 %) |
| `html/browsers/history/joint-session-history` | 2 / 3 | 2 / 3 (66,7 %) |
| `html/browsers/history/the-history-interface` | 13 / 63 | 59 / 135 (43,7 %) |
| `html/browsers/history/the-history-interface/joint_session_history` | 0 / 2 | 0 / 10 (0,0 %) |
| `html/browsers/history/the-history-interface/pushstate-replacestate-empty-string` | 4 / 6 | 4 / 6 (66,7 %) |
| `html/browsers/history/the-location-interface` | 14 / 66 | 92 / 282 (32,6 %) |
| `html/browsers/history/the-session-history-of-browsing-contexts` | 0 / 1 | 0 / 1 (0,0 %) |
| `html/browsers/offline/browser-state` | 1 / 1 | 1 / 1 (100,0 %) |
| `html/browsers/origin` | 0 / 1 | 0 / 1 (0,0 %) |
| `html/browsers/origin/api` | 0 / 13 | 0 / 234 (0,0 %) |
| `html/browsers/origin/cross-origin-objects` | 1 / 8 | 9 / 25 (36,0 %) |
| `html/browsers/origin/inheritance` | 2 / 9 | 2 / 29 (6,9 %) |
| `html/browsers/origin/origin-keyed-agent-clusters` | 0 / 12 | 1 / 51 (2,0 %) |
| `html/browsers/origin/origin-keyed-agent-clusters/1-iframe` | 0 / 13 | 0 / 64 (0,0 %) |
| `html/browsers/origin/origin-keyed-agent-clusters/2-iframes` | 0 / 15 | 0 / 146 (0,0 %) |
| `html/browsers/origin/origin-keyed-agent-clusters/getter-special-cases` | 0 / 15 | 0 / 16 (0,0 %) |
| `html/browsers/origin/origin-keyed-agent-clusters/iframe-navigation` | 0 / 8 | 0 / 64 (0,0 %) |
| `html/browsers/origin/origin-keyed-agent-clusters/popups` | 0 / 10 | 0 / 38 (0,0 %) |
| `html/browsers/origin/relaxing-the-same-origin-restriction` | 1 / 5 | 4 / 27 (14,8 %) |
| `html/browsers/sandboxing` | 4 / 20 | 5 / 33 (15,2 %) |
| `html/browsers/the-window-object` | 6 / 33 | 81 / 416 (19,5 %) |
| `html/browsers/the-window-object/accessing-other-browsing-contexts` | 1 / 5 | 7 / 15 (46,7 %) |
| `html/browsers/the-window-object/garbage-collection-and-browsing-contexts` | 0 / 4 | 0 / 4 (0,0 %) |
| `html/browsers/the-window-object/named-access-on-the-window-object` | 6 / 19 | 61 / 97 (62,9 %) |
| `html/browsers/the-window-object/open-close` | 0 / 33 | 12 / 116 (10,3 %) |
| `html/browsers/the-window-object/security-window` | 0 / 1 | 14 / 124 (11,3 %) |
| `html/browsers/the-windowproxy-exotic-object` | 0 / 8 | 15 / 55 (27,3 %) |
| `html/browsers/windows` | 1 / 16 | 5 / 40 (12,5 %) |
| `html/browsers/windows/auxiliary-browsing-contexts` | 0 / 10 | 0 / 19 (0,0 %) |
| `html/browsers/windows/browsing-context-names` | 0 / 18 | 0 / 21 (0,0 %) |
| `html/browsers/windows/consume-user-activation` | 0 / 2 | 0 / 3 (0,0 %) |
| `html/browsers/windows/nested-browsing-contexts` | 1 / 8 | 4 / 42 (9,5 %) |
| `html/canvas` | 3 / 4 | 11 / 18 (61,1 %) |
| `html/canvas/element` | 3 / 5 | 3 / 15 (20,0 %) |
| `html/canvas/element/canvas-context` | 10 / 11 | 10 / 11 (90,9 %) |
| `html/canvas/element/canvas-host` | 36 / 52 | 50 / 66 (75,8 %) |
| `html/canvas/element/color-type` | 2 / 4 | 2 / 4 (50,0 %) |
| `html/canvas/element/compositing` | 59 / 98 | 59 / 98 (60,2 %) |
| `html/canvas/element/conformance-requirements` | 3 / 4 | 3 / 4 (75,0 %) |
| `html/canvas/element/drawing-images-to-the-canvas` | 13 / 37 | 13 / 37 (35,1 %) |
| `html/canvas/element/drawing-rectangles-to-the-canvas` | 30 / 32 | 30 / 32 (93,8 %) |
| `html/canvas/element/fill-and-stroke-styles` | 163 / 257 | 163 / 257 (63,4 %) |
| `html/canvas/element/filters` | 1 / 12 | 1 / 12 (8,3 %) |
| `html/canvas/element/global-hdr-headroom` | 3 / 3 | 10 / 10 (100,0 %) |
| `html/canvas/element/layers` | 4 / 20 | 4 / 25 (16,0 %) |
| `html/canvas/element/line-styles` | 32 / 33 | 32 / 33 (97,0 %) |
| `html/canvas/element/manual/context-attributes` | 1 / 2 | 5 / 17 (29,4 %) |
| `html/canvas/element/manual/draw-element-image` | 1 / 52 | 6 / 134 (4,5 %) |
| `html/canvas/element/manual/draw-element-image/hit-test` | 0 / 15 | 0 / 52 (0,0 %) |
| `html/canvas/element/manual/draw-element-image/privacy` | 0 / 46 | 0 / 40 (0,0 %) |
| `html/canvas/element/manual/draw-element-image/privacy/pseudos` | 0 / 22 | 0 / 22 (0,0 %) |
| `html/canvas/element/manual/draw-element-image/privacy/svg` | 0 / 19 | 0 / 19 (0,0 %) |
| `html/canvas/element/manual/draw-element-image/privacy/visited-link` | 0 / 12 | 0 / 12 (0,0 %) |
| `html/canvas/element/manual/drawing-images-to-the-canvas` | 0 / 5 | 221 / 284 (77,8 %) |
| `html/canvas/element/manual/drawing-paths-to-the-canvas` | 1 / 6 | 1 / 6 (16,7 %) |
| `html/canvas/element/manual/fill-and-stroke-styles` | 0 / 6 | 0 / 6 (0,0 %) |
| `html/canvas/element/manual/filters` | 0 / 7 | 1 / 18 (5,6 %) |
| `html/canvas/element/manual/filters/tentative/idl-conversions` | 0 / 1 | 0 / 1 (0,0 %) |
| `html/canvas/element/manual/image-smoothing` | 0 / 1 | 6 / 9 (66,7 %) |
| `html/canvas/element/manual/imagebitmap` | 2 / 20 | 29 / 281 (10,3 %) |
| `html/canvas/element/manual/line-styles` | 1 / 1 | 2 / 2 (100,0 %) |
| `html/canvas/element/manual/shadows` | 0 / 2 | 0 / 4 (0,0 %) |
| `html/canvas/element/manual/the-canvas-state` | 2 / 2 | 4 / 4 (100,0 %) |
| `html/canvas/element/manual/transformations` | 0 / 1 | 0 / 1 (0,0 %) |
| `html/canvas/element/manual/wide-gamut-canvas` | 0 / 14 | 78 / 1535 (5,1 %) |
| `html/canvas/element/path-objects` | 181 / 210 | 181 / 210 (86,2 %) |
| `html/canvas/element/pixel-manipulation` | 62 / 71 | 62 / 71 (87,3 %) |
| `html/canvas/element/reset` | 23 / 29 | 23 / 29 (79,3 %) |
| `html/canvas/element/shadows` | 23 / 59 | 23 / 59 (39,0 %) |
| `html/canvas/element/text` | 25 / 125 | 25 / 285 (8,8 %) |
| `html/canvas/element/the-canvas-state` | 23 / 23 | 68 / 68 (100,0 %) |
| `html/canvas/element/transformations` | 22 / 22 | 22 / 22 (100,0 %) |
| `html/canvas/element/video` | 0 / 1 | 0 / 1 (0,0 %) |
| `html/canvas/element/wide-gamut-canvas` | 0 / 8 | 0 / 26 (0,0 %) |
| `html/canvas/offscreen` | 1 / 3 | 1 / 3 (33,3 %) |
| `html/canvas/offscreen/canvas-context` | 10 / 15 | 10 / 15 (66,7 %) |
| `html/canvas/offscreen/canvas-host` | 32 / 35 | 43 / 46 (93,5 %) |
| `html/canvas/offscreen/color-type` | 2 / 4 | 2 / 4 (50,0 %) |
| `html/canvas/offscreen/compositing` | 59 / 98 | 59 / 98 (60,2 %) |
| `html/canvas/offscreen/conformance-requirements` | 1 / 2 | 1 / 2 (50,0 %) |
| `html/canvas/offscreen/drawing-images-to-the-canvas` | 7 / 28 | 7 / 28 (25,0 %) |
| `html/canvas/offscreen/drawing-rectangles-to-the-canvas` | 30 / 32 | 30 / 32 (93,8 %) |
| `html/canvas/offscreen/fill-and-stroke-styles` | 165 / 247 | 165 / 247 (66,8 %) |
| `html/canvas/offscreen/filters` | 1 / 12 | 1 / 12 (8,3 %) |
| `html/canvas/offscreen/layers` | 4 / 20 | 4 / 25 (16,0 %) |
| `html/canvas/offscreen/line-styles` | 32 / 33 | 32 / 33 (97,0 %) |
| `html/canvas/offscreen/manual/context-attributes` | 0 / 1 | 4 / 16 (25,0 %) |
| `html/canvas/offscreen/manual/convert-to-blob` | 0 / 2 | 4 / 24 (16,7 %) |
| `html/canvas/offscreen/manual/draw-element-image` | 0 / 5 | 0 / 6 (0,0 %) |
| `html/canvas/offscreen/manual/draw-generic-family` | 2 / 2 | 10 / 10 (100,0 %) |
| `html/canvas/offscreen/manual/filter` | 1 / 2 | 1 / 2 (50,0 %) |
| `html/canvas/offscreen/manual/image-smoothing` | 0 / 1 | 6 / 9 (66,7 %) |
| `html/canvas/offscreen/manual/the-offscreen-canvas` | 0 / 13 | 11 / 60 (18,3 %) |
| `html/canvas/offscreen/manual/transformations` | 0 / 1 | 0 / 1 (0,0 %) |
| `html/canvas/offscreen/manual/wide-gamut-canvas` | 1 / 1 | 1 / 1 (100,0 %) |
| `html/canvas/offscreen/path-objects` | 181 / 211 | 181 / 211 (85,8 %) |
| `html/canvas/offscreen/pixel-manipulation` | 55 / 59 | 55 / 59 (93,2 %) |
| `html/canvas/offscreen/reset` | 23 / 29 | 23 / 29 (79,3 %) |
| `html/canvas/offscreen/shadows` | 19 / 56 | 19 / 56 (33,9 %) |
| `html/canvas/offscreen/text` | 25 / 119 | 25 / 277 (9,0 %) |
| `html/canvas/offscreen/the-canvas-state` | 20 / 20 | 56 / 56 (100,0 %) |
| `html/canvas/offscreen/transformations` | 22 / 22 | 22 / 22 (100,0 %) |
| `html/canvas/offscreen/wide-gamut-canvas` | 0 / 3 | 0 / 8 (0,0 %) |
| `html/capability-delegation` | 0 / 7 | 0 / 17 (0,0 %) |
| `html/cross-origin-embedder-policy` | 1 / 53 | 57 / 406 (14,0 %) |
| `html/cross-origin-embedder-policy/credentialless` | 0 / 22 | 6 / 38 (15,8 %) |
| `html/cross-origin-embedder-policy/multi-globals` | 0 / 1 | 0 / 1 (0,0 %) |
| `html/cross-origin-opener-policy` | 0 / 55 | 6 / 449 (1,3 %) |
| `html/cross-origin-opener-policy/historical` | 0 / 10 | 0 / 76 (0,0 %) |
| `html/cross-origin-opener-policy/reporting/access-reporting` | 0 / 32 | 0 / 72 (0,0 %) |
| `html/cross-origin-opener-policy/reporting/document-reporting` | 0 / 4 | 0 / 6 (0,0 %) |
| `html/cross-origin-opener-policy/reporting/navigation-reporting` | 0 / 15 | 0 / 5 (0,0 %) |
| `html/cross-origin-opener-policy/reporting/tentative` | 0 / 1 | 0 / 2 (0,0 %) |
| `html/cross-origin-opener-policy/tentative/noopener` | 0 / 1 | 0 / 2 (0,0 %) |
| `html/document-isolation-policy` | 0 / 46 | 9 / 253 (3,6 %) |
| `html/dom` | 1 / 23 | 30309 / 51854 (58,5 %) |
| `html/dom/documents/dom-tree-accessors` | 15 / 42 | 99 / 245 (40,4 %) |
| `html/dom/documents/dom-tree-accessors/document.getElementsByName` | 12 / 15 | 31 / 34 (91,2 %) |
| `html/dom/documents/resource-metadata-management` | 6 / 11 | 13 / 23 (56,5 %) |
| `html/dom/elements` | 0 / 1 | 17 / 141 (12,1 %) |
| `html/dom/elements/elements-in-the-dom` | 1 / 2 | 8 / 9 (88,9 %) |
| `html/dom/elements/global-attributes` | 23 / 47 | 158 / 253 (62,5 %) |
| `html/dom/elements/images` | 0 / 1 | 0 / 1 (0,0 %) |
| `html/dom/elements/the-innertext-and-outertext-properties` | 2 / 8 | 253 / 537 (47,1 %) |
| `html/dom/partial-updates/tentative` | 0 / 2 | 0 / 2 (0,0 %) |
| `html/dom/partial-updates/tentative/fragment` | 2 / 14 | 2 / 23 (8,7 %) |
| `html/dom/render-blocking` | 10 / 69 | 30 / 122 (24,6 %) |
| `html/editing/activation` | 0 / 3 | 0 / 6 (0,0 %) |
| `html/editing/dnd` | 1 / 3 | 1 / 3 (33,3 %) |
| `html/editing/dnd/canvas` | 0 / 5 | 0 / 5 (0,0 %) |
| `html/editing/dnd/datastore` | 0 / 11 | 0 / 47 (0,0 %) |
| `html/editing/dnd/dom` | 0 / 3 | 15 / 55 (27,3 %) |
| `html/editing/dnd/drop` | 0 / 8 | 0 / 19 (0,0 %) |
| `html/editing/dnd/events` | 0 / 2 | 0 / 2 (0,0 %) |
| `html/editing/dnd/images` | 6 / 21 | 6 / 19 (31,6 %) |
| `html/editing/dnd/interactiveelements` | 12 / 24 | 26 / 48 (54,2 %) |
| `html/editing/dnd/layoutshift` | 0 / 1 | 0 / 1 (0,0 %) |
| `html/editing/dnd/platform` | 0 / 5 | 0 / 7 (0,0 %) |
| `html/editing/dnd/reload` | 0 / 8 | 0 / 8 (0,0 %) |
| `html/editing/dnd/remove` | 0 / 8 | 0 / 8 (0,0 %) |
| `html/editing/dnd/synthetic` | 0 / 1 | 15 / 16 (93,8 %) |
| `html/editing/dnd/target-origin` | 0 / 1 | 0 / 1 (0,0 %) |
| `html/editing/dnd/the-draggable-attribute` | 0 / 2 | 151 / 303 (49,8 %) |
| `html/editing/dnd/the-dropzone-attribute` | 0 / 1 | 2 / 4 (50,0 %) |
| `html/editing/editing-0/autocapitalization` | 0 / 1 | 0 / 8 (0,0 %) |
| `html/editing/editing-0/autocorrection` | 0 / 1 | 1 / 9 (11,1 %) |
| `html/editing/editing-0/contenteditable` | 1 / 4 | 3 / 36 (8,3 %) |
| `html/editing/editing-0/making-entire-documents-editable-the-designmode-idl-attribute` | 0 / 3 | 1 / 8 (12,5 %) |
| `html/editing/editing-0/spelling-and-grammar-checking` | 0 / 2 | 1 / 4 (25,0 %) |
| `html/editing/editing-0/writing-suggestions` | 0 / 1 | 6 / 80 (7,5 %) |
| `html/editing/focus/focus-management` | 3 / 6 | 4 / 8 (50,0 %) |
| `html/editing/focus/sequential-focus-navigation-and-the-tabindex-attribute` | 1 / 1 | 2 / 2 (100,0 %) |
| `html/editing/the-hidden-attribute` | 0 / 13 | 16 / 47 (34,0 %) |
| `html/infrastructure/common-dom-interfaces/collections` | 0 / 7 | 19 / 104 (18,3 %) |
| `html/infrastructure/conformance-requirements/extensibility` | 1 / 1 | 1 / 1 (100,0 %) |
| `html/infrastructure/fetching-resources` | 0 / 1 | 0 / 2 (0,0 %) |
| `html/infrastructure/safe-passing-of-structured-data` | 2 / 11 | 152 / 248 (61,3 %) |
| `html/infrastructure/safe-passing-of-structured-data/shared-array-buffers` | 1 / 21 | 8 / 55 (14,5 %) |
| `html/infrastructure/terminology/plugins` | 0 / 1 | 0 / 1 (0,0 %) |
| `html/infrastructure/urls/base-url` | 1 / 11 | 2 / 17 (11,8 %) |
| `html/infrastructure/urls/dynamic-changes-to-base-urls` | 0 / 2 | 15 / 39 (38,5 %) |
| `html/infrastructure/urls/resolving-urls/query-encoding` | 1 / 7 | 120 / 323 (37,2 %) |
| `html/infrastructure/urls/terminology-0` | 1 / 5 | 5 / 14 (35,7 %) |
| `html/interaction/focus` | 6 / 13 | 31 / 82 (37,8 %) |
| `html/interaction/focus/document-level-focus-apis` | 0 / 2 | 2 / 5 (40,0 %) |
| `html/interaction/focus/focus-management` | 1 / 3 | 3 / 8 (37,5 %) |
| `html/interaction/focus/focusgroup/tentative` | 1 / 24 | 2 / 133 (1,5 %) |
| `html/interaction/focus/focusgroup/tentative/backward-navigation` | 0 / 13 | 0 / 13 (0,0 %) |
| `html/interaction/focus/focusgroup/tentative/backward-navigation/horizontal` | 0 / 5 | 0 / 5 (0,0 %) |
| `html/interaction/focus/focusgroup/tentative/backward-navigation/vertical` | 0 / 5 | 0 / 5 (0,0 %) |
| `html/interaction/focus/focusgroup/tentative/behavior-tokens` | 0 / 8 | 0 / 83 (0,0 %) |
| `html/interaction/focus/focusgroup/tentative/descendant-navigation` | 0 / 5 | 0 / 15 (0,0 %) |
| `html/interaction/focus/focusgroup/tentative/forward-navigation` | 0 / 14 | 0 / 37 (0,0 %) |
| `html/interaction/focus/focusgroup/tentative/forward-navigation/horizontal` | 0 / 3 | 0 / 5 (0,0 %) |
| `html/interaction/focus/focusgroup/tentative/forward-navigation/vertical` | 0 / 2 | 0 / 2 (0,0 %) |
| `html/interaction/focus/focusgroup/tentative/grid-navigation` | 0 / 20 | 0 / 66 (0,0 %) |
| `html/interaction/focus/focusgroup/tentative/opt-out-barriers` | 0 / 3 | 1 / 11 (9,1 %) |
| `html/interaction/focus/focusgroup/tentative/reading-flow-navigation` | 0 / 6 | 0 / 6 (0,0 %) |
| `html/interaction/focus/focusgroup/tentative/sequential-navigation` | 0 / 27 | 0 / 88 (0,0 %) |
| `html/interaction/focus/focusgroup/tentative/shadow` | 0 / 3 | 0 / 3 (0,0 %) |
| `html/interaction/focus/focusreset` | 0 / 6 | 13 / 64 (20,3 %) |
| `html/interaction/focus/processing-model` | 5 / 10 | 18 / 35 (51,4 %) |
| `html/interaction/focus/sequential-focus-navigation-and-the-tabindex-attribute` | 0 / 7 | 110 / 134 (82,1 %) |
| `html/interaction/focus/the-autofocus-attribute` | 11 / 27 | 11 / 38 (28,9 %) |
| `html/links/icon` | 2 / 2 | 2 / 2 (100,0 %) |
| `html/links/manifest/link-relationship` | 0 / 1 | 0 / 1 (0,0 %) |
| `html/links/stylesheet` | 2 / 3 | 2 / 3 (66,7 %) |
| `html/meta` | 0 / 1 | 0 / 3 (0,0 %) |
| `html/obsolete/requirements-for-implementations/other-elements-attributes-and-apis` | 1 / 10 | 5 / 42 (11,9 %) |
| `html/obsolete/requirements-for-implementations/the-marquee-element-0` | 0 / 5 | 1 / 13 (7,7 %) |
| `html/rendering` | 0 / 3 | 974 / 1866 (52,2 %) |
| `html/rendering/bidi-rendering` | 0 / 3 | 0 / 203 (0,0 %) |
| `html/rendering/non-replaced-elements` | 0 / 2 | 15 / 25 (60,0 %) |
| `html/rendering/non-replaced-elements/flow-content-0` | 2 / 8 | 2 / 14 (14,3 %) |
| `html/rendering/non-replaced-elements/form-controls` | 1 / 3 | 1 / 13 (7,7 %) |
| `html/rendering/non-replaced-elements/lists` | 0 / 4 | 0 / 4 (0,0 %) |
| `html/rendering/non-replaced-elements/margin-collapsing-quirks` | 2 / 2 | 2 / 2 (100,0 %) |
| `html/rendering/non-replaced-elements/phrasing-content-0/font-element-text-decoration-color` | 0 / 2 | 5 / 33 (15,2 %) |
| `html/rendering/non-replaced-elements/sections-and-headings` | 0 / 1 | 0 / 0 (—) |
| `html/rendering/non-replaced-elements/tables` | 5 / 14 | 39 / 144 (27,1 %) |
| `html/rendering/non-replaced-elements/the-fieldset-and-legend-elements` | 15 / 37 | 156 / 300 (52,0 %) |
| `html/rendering/non-replaced-elements/the-frameset-and-frame-elements` | 0 / 3 | 0 / 3 (0,0 %) |
| `html/rendering/non-replaced-elements/the-hr-element-0` | 1 / 2 | 18 / 20 (90,0 %) |
| `html/rendering/non-replaced-elements/the-page` | 3 / 4 | 3 / 6 (50,0 %) |
| `html/rendering/replaced-elements/attributes-for-embedded-content-and-images` | 5 / 18 | 50 / 127 (39,4 %) |
| `html/rendering/replaced-elements/embedded-content` | 3 / 4 | 3 / 16 (18,8 %) |
| `html/rendering/replaced-elements/images` | 1 / 4 | 1 / 3 (33,3 %) |
| `html/rendering/replaced-elements/svg-embedded-sizing` | 0 / 9 | 500 / 609 (82,1 %) |
| `html/rendering/replaced-elements/svg-inline-sizing` | 0 / 1 | 144 / 648 (22,2 %) |
| `html/rendering/replaced-elements/the-textarea-element` | 1 / 1 | 1 / 1 (100,0 %) |
| `html/rendering/the-css-user-agent-style-sheet-and-presentational-hints` | 1 / 4 | 2 / 7 (28,6 %) |
| `html/rendering/the-details-element` | 0 / 5 | 7 / 13 (53,8 %) |
| `html/rendering/widgets` | 6 / 17 | 166 / 246 (67,5 %) |
| `html/rendering/widgets/appearance` | 0 / 1 | 0 / 45 (0,0 %) |
| `html/rendering/widgets/button-layout` | 0 / 5 | 91 / 216 (42,1 %) |
| `html/rendering/widgets/the-select-element` | 0 / 1 | 0 / 0 (—) |
| `html/scripting/the-script-element` | 1 / 3 | 1 / 4 (25,0 %) |
| `html/select` | 0 / 1 | 1 / 5 (20,0 %) |
| `html/semantics` | 0 / 2 | 317 / 442 (71,7 %) |
| `html/semantics/disabled-elements` | 2 / 7 | 158 / 299 (52,8 %) |
| `html/semantics/document-metadata/interactions-of-styling-and-scripting` | 0 / 8 | 0 / 9 (0,0 %) |
| `html/semantics/document-metadata/styling` | 0 / 1 | 1 / 5 (20,0 %) |
| `html/semantics/document-metadata/the-base-element` | 1 / 11 | 1 / 12 (8,3 %) |
| `html/semantics/document-metadata/the-link-element` | 1 / 26 | 1 / 108 (0,9 %) |
| `html/semantics/document-metadata/the-link-element/tentative` | 0 / 5 | 0 / 31 (0,0 %) |
| `html/semantics/document-metadata/the-meta-element` | 1 / 3 | 1 / 3 (33,3 %) |
| `html/semantics/document-metadata/the-meta-element/color-scheme` | 0 / 11 | 1 / 24 (4,2 %) |
| `html/semantics/document-metadata/the-meta-element/pragma-directives` | 1 / 5 | 1 / 5 (20,0 %) |
| `html/semantics/document-metadata/the-meta-element/pragma-directives/attr-meta-http-equiv-refresh` | 1 / 6 | 1 / 138 (0,7 %) |
| `html/semantics/document-metadata/the-style-element` | 2 / 22 | 9 / 40 (22,5 %) |
| `html/semantics/document-metadata/the-style-element/tentative` | 0 / 11 | 1 / 24 (4,2 %) |
| `html/semantics/document-metadata/the-title-element` | 1 / 5 | 1 / 25 (4,0 %) |
| `html/semantics/edits/the-del-element` | 0 / 1 | 0 / 1 (0,0 %) |
| `html/semantics/edits/the-ins-element` | 0 / 1 | 0 / 1 (0,0 %) |
| `html/semantics/embedded-content/bfcache` | 0 / 6 | 0 / 0 (—) |
| `html/semantics/embedded-content/image-maps/image-map-processing-model` | 1 / 3 | 2 / 177 (1,1 %) |
| `html/semantics/embedded-content/media-elements` | 3 / 62 | 145 / 302 (48,0 %) |
| `html/semantics/embedded-content/media-elements/error-codes` | 0 / 1 | 0 / 6 (0,0 %) |
| `html/semantics/embedded-content/media-elements/interfaces/HTMLElement/HTMLMediaElement` | 0 / 3 | 3 / 23 (13,0 %) |
| `html/semantics/embedded-content/media-elements/interfaces/HTMLElement/HTMLTrackElement` | 0 / 7 | 9 / 63 (14,3 %) |
| `html/semantics/embedded-content/media-elements/interfaces/TextTrack` | 0 / 10 | 0 / 17 (0,0 %) |
| `html/semantics/embedded-content/media-elements/interfaces/TextTrackCue` | 0 / 8 | 0 / 2 (0,0 %) |
| `html/semantics/embedded-content/media-elements/interfaces/TextTrackCueList` | 0 / 3 | 0 / 4 (0,0 %) |
| `html/semantics/embedded-content/media-elements/interfaces/TextTrackList` | 0 / 5 | 0 / 5 (0,0 %) |
| `html/semantics/embedded-content/media-elements/interfaces/TrackEvent` | 0 / 2 | 0 / 3 (0,0 %) |
| `html/semantics/embedded-content/media-elements/loading-the-media-resource` | 18 / 52 | 19 / 63 (30,2 %) |
| `html/semantics/embedded-content/media-elements/location-of-the-media-resource` | 0 / 1 | 0 / 18 (0,0 %) |
| `html/semantics/embedded-content/media-elements/media-playback-while-not-visible-permission-policy` | 0 / 4 | 0 / 4 (0,0 %) |
| `html/semantics/embedded-content/media-elements/mime-types` | 0 / 1 | 19 / 60 (31,7 %) |
| `html/semantics/embedded-content/media-elements/offsets-into-the-media-resource` | 1 / 3 | 2 / 6 (33,3 %) |
| `html/semantics/embedded-content/media-elements/playing-the-media-resource` | 0 / 9 | 0 / 18 (0,0 %) |
| `html/semantics/embedded-content/media-elements/ready-states` | 0 / 3 | 0 / 22 (0,0 %) |
| `html/semantics/embedded-content/media-elements/seeking` | 0 / 3 | 0 / 3 (0,0 %) |
| `html/semantics/embedded-content/media-elements/track/track-element` | 1 / 95 | 1 / 131 (0,8 %) |
| `html/semantics/embedded-content/media-elements/track/track-element/cors` | 0 / 44 | 0 / 44 (0,0 %) |
| `html/semantics/embedded-content/media-elements/user-interface` | 0 / 2 | 28 / 57 (49,1 %) |
| `html/semantics/embedded-content/the-area-element` | 2 / 7 | 34 / 79 (43,0 %) |
| `html/semantics/embedded-content/the-audio-element` | 2 / 14 | 7 / 24 (29,2 %) |
| `html/semantics/embedded-content/the-canvas-element` | 34 / 70 | 81 / 149 (54,4 %) |
| `html/semantics/embedded-content/the-embed-element` | 3 / 16 | 5 / 27 (18,5 %) |
| `html/semantics/embedded-content/the-frame-element` | 0 / 2 | 0 / 3 (0,0 %) |
| `html/semantics/embedded-content/the-iframe-element` | 20 / 147 | 46 / 215 (21,4 %) |
| `html/semantics/embedded-content/the-iframe-element/cross-site` | 0 / 6 | 0 / 1 (0,0 %) |
| `html/semantics/embedded-content/the-img-element` | 11 / 89 | 195 / 635 (30,7 %) |
| `html/semantics/embedded-content/the-img-element/current-pixel-density` | 0 / 2 | 3 / 21 (14,3 %) |
| `html/semantics/embedded-content/the-img-element/decode` | 0 / 9 | 17 / 64 (26,6 %) |
| `html/semantics/embedded-content/the-img-element/environment-changes` | 0 / 2 | 0 / 1 (0,0 %) |
| `html/semantics/embedded-content/the-img-element/sizes` | 0 / 11 | 0 / 222 (0,0 %) |
| `html/semantics/embedded-content/the-img-element/srcset` | 1 / 8 | 188 / 303 (62,0 %) |
| `html/semantics/embedded-content/the-img-element/update-the-image-data` | 1 / 6 | 9 / 24 (37,5 %) |
| `html/semantics/embedded-content/the-object-element` | 3 / 17 | 6 / 46 (13,0 %) |
| `html/semantics/embedded-content/the-persistentwidget-element/tentative` | 0 / 1 | 0 / 1 (0,0 %) |
| `html/semantics/embedded-content/the-video-element` | 4 / 25 | 7 / 34 (20,6 %) |
| `html/semantics/forms` | 1 / 5 | 15 / 44 (34,1 %) |
| `html/semantics/forms/attributes-common-to-form-controls` | 0 / 7 | 16 / 42 (38,1 %) |
| `html/semantics/forms/constraints` | 8 / 29 | 798 / 921 (86,6 %) |
| `html/semantics/forms/customizable-combobox` | 0 / 7 | 0 / 12 (0,0 %) |
| `html/semantics/forms/form-control-infrastructure` | 1 / 8 | 17 / 130 (13,1 %) |
| `html/semantics/forms/form-submission-0` | 5 / 39 | 45 / 412 (10,9 %) |
| `html/semantics/forms/form-submission-target` | 0 / 8 | 0 / 35 (0,0 %) |
| `html/semantics/forms/resetting-a-form` | 0 / 4 | 0 / 15 (0,0 %) |
| `html/semantics/forms/textfieldselection` | 0 / 16 | 98 / 540 (18,1 %) |
| `html/semantics/forms/the-button-element` | 9 / 24 | 32 / 115 (27,8 %) |
| `html/semantics/forms/the-datalist-element` | 0 / 2 | 1 / 3 (33,3 %) |
| `html/semantics/forms/the-fieldset-element` | 1 / 11 | 11 / 27 (40,7 %) |
| `html/semantics/forms/the-form-element` | 5 / 21 | 23 / 130 (17,7 %) |
| `html/semantics/forms/the-input-element` | 28 / 130 | 555 / 1986 (27,9 %) |
| `html/semantics/forms/the-label-element` | 2 / 14 | 106 / 169 (62,7 %) |
| `html/semantics/forms/the-legend-element` | 0 / 2 | 0 / 5 (0,0 %) |
| `html/semantics/forms/the-meter-element` | 0 / 2 | 0 / 62 (0,0 %) |
| `html/semantics/forms/the-optgroup-element` | 1 / 1 | 1 / 1 (100,0 %) |
| `html/semantics/forms/the-option-element` | 5 / 11 | 85 / 108 (78,7 %) |
| `html/semantics/forms/the-output-element` | 0 / 4 | 0 / 5 (0,0 %) |
| `html/semantics/forms/the-progress-element` | 1 / 5 | 1 / 19 (5,3 %) |
| `html/semantics/forms/the-select-element` | 15 / 49 | 46 / 164 (28,0 %) |
| `html/semantics/forms/the-select-element/customizable-select` | 9 / 90 | 41 / 318 (12,9 %) |
| `html/semantics/forms/the-select-element/customizable-select-listbox` | 0 / 6 | 0 / 25 (0,0 %) |
| `html/semantics/forms/the-select-element/customizable-select-multiple-popup` | 0 / 5 | 0 / 6 (0,0 %) |
| `html/semantics/forms/the-select-element/filterable-select` | 0 / 3 | 0 / 13 (0,0 %) |
| `html/semantics/forms/the-textarea-element` | 5 / 15 | 20 / 43 (46,5 %) |
| `html/semantics/grouping-content/the-dd-element` | 1 / 1 | 1 / 1 (100,0 %) |
| `html/semantics/grouping-content/the-div-element` | 1 / 1 | 1 / 1 (100,0 %) |
| `html/semantics/grouping-content/the-dl-element` | 1 / 1 | 1 / 1 (100,0 %) |
| `html/semantics/grouping-content/the-dt-element` | 1 / 1 | 1 / 1 (100,0 %) |
| `html/semantics/grouping-content/the-figcaption-element` | 1 / 1 | 1 / 1 (100,0 %) |
| `html/semantics/grouping-content/the-figure-element` | 1 / 1 | 1 / 1 (100,0 %) |
| `html/semantics/grouping-content/the-hr-element` | 1 / 1 | 1 / 1 (100,0 %) |
| `html/semantics/grouping-content/the-li-element` | 0 / 1 | 0 / 10 (0,0 %) |
| `html/semantics/grouping-content/the-ol-element` | 0 / 3 | 4 / 27 (14,8 %) |
| `html/semantics/grouping-content/the-p-element` | 1 / 1 | 1 / 1 (100,0 %) |
| `html/semantics/grouping-content/the-pre-element` | 1 / 1 | 1 / 1 (100,0 %) |
| `html/semantics/grouping-content/the-ul-element` | 1 / 1 | 1 / 1 (100,0 %) |
| `html/semantics/interactive-elements` | 0 / 1 | 9 / 11 (81,8 %) |
| `html/semantics/interactive-elements/commands/option` | 0 / 2 | 0 / 5 (0,0 %) |
| `html/semantics/interactive-elements/the-details-element` | 2 / 9 | 8 / 42 (19,0 %) |
| `html/semantics/interactive-elements/the-dialog-element` | 29 / 87 | 178 / 447 (39,8 %) |
| `html/semantics/interactive-elements/the-summary-element` | 1 / 6 | 3 / 50 (6,0 %) |
| `html/semantics/interestfor` | 1 / 31 | 15 / 136 (11,0 %) |
| `html/semantics/links/downloading-resources` | 0 / 18 | 0 / 18 (0,0 %) |
| `html/semantics/links/following-hyperlinks` | 1 / 2 | 6 / 7 (85,7 %) |
| `html/semantics/links/hyperlink-auditing` | 0 / 1 | 0 / 1 (0,0 %) |
| `html/semantics/links/links-created-by-a-and-area-elements` | 6 / 15 | 141 / 189 (74,6 %) |
| `html/semantics/menu/tentative` | 0 / 25 | 6 / 116 (5,2 %) |
| `html/semantics/permission-element` | 0 / 8 | 0 / 49 (0,0 %) |
| `html/semantics/permission-element/camera` | 0 / 5 | 0 / 7 (0,0 %) |
| `html/semantics/permission-element/geolocation-element` | 0 / 13 | 0 / 52 (0,0 %) |
| `html/semantics/permission-element/install` | 0 / 16 | 0 / 93 (0,0 %) |
| `html/semantics/permission-element/microphone` | 0 / 5 | 0 / 8 (0,0 %) |
| `html/semantics/permission-element/permission-icon` | 0 / 1 | 0 / 1 (0,0 %) |
| `html/semantics/permission-element/usermedia` | 0 / 13 | 6 / 23 (26,1 %) |
| `html/semantics/permission-element/usermedia/legacy-mode` | 0 / 1 | 0 / 4 (0,0 %) |
| `html/semantics/popovers` | 18 / 89 | 1176 / 3900 (30,2 %) |
| `html/semantics/scripting-1/the-noscript-element` | 0 / 1 | 0 / 1 (0,0 %) |
| `html/semantics/scripting-1/the-script-element` | 19 / 62 | 430 / 1499 (28,7 %) |
| `html/semantics/scripting-1/the-script-element/css-module` | 0 / 18 | 2 / 41 (4,9 %) |
| `html/semantics/scripting-1/the-script-element/execution-timing` | 63 / 157 | 74 / 168 (44,0 %) |
| `html/semantics/scripting-1/the-script-element/fetch-src` | 0 / 6 | 1 / 7 (14,3 %) |
| `html/semantics/scripting-1/the-script-element/fetch-src/alpha` | 1 / 1 | 1 / 1 (100,0 %) |
| `html/semantics/scripting-1/the-script-element/import-attributes` | 2 / 5 | 4 / 7 (57,1 %) |
| `html/semantics/scripting-1/the-script-element/json-module` | 1 / 18 | 9 / 159 (5,7 %) |
| `html/semantics/scripting-1/the-script-element/microtasks` | 1 / 8 | 2 / 13 (15,4 %) |
| `html/semantics/scripting-1/the-script-element/module` | 19 / 74 | 58 / 4154 (1,4 %) |
| `html/semantics/scripting-1/the-script-element/module/dynamic-import` | 8 / 32 | 17 / 154 (11,0 %) |
| `html/semantics/scripting-1/the-script-element/module/dynamic-import/alpha` | 0 / 3 | 0 / 0 (—) |
| `html/semantics/scripting-1/the-script-element/module/dynamic-import/microtasks` | 1 / 2 | 3 / 4 (75,0 %) |
| `html/semantics/scripting-1/the-script-element/module/import-meta` | 0 / 3 | 3 / 13 (23,1 %) |
| `html/semantics/scripting-1/the-script-element/module/top-level-await` | 0 / 1 | 0 / 2 (0,0 %) |
| `html/semantics/scripting-1/the-script-element/moving-between-documents` | 0 / 52 | 52 / 260 (20,0 %) |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/ordering` | 0 / 4 | 2 / 6 (33,3 %) |
| `html/semantics/scripting-1/the-script-element/text-module` | 1 / 15 | 13 / 155 (8,4 %) |
| `html/semantics/scripting-1/the-template-element` | 0 / 1 | 267 / 403 (66,3 %) |
| `html/semantics/scripting-1/the-template-element/additions-to-parsing-xhtml-documents` | 1 / 3 | 3 / 10 (30,0 %) |
| `html/semantics/scripting-1/the-template-element/additions-to-serializing-xhtml-documents` | 0 / 1 | 0 / 3 (0,0 %) |
| `html/semantics/scripting-1/the-template-element/additions-to-the-steps-to-clone-a-node` | 2 / 2 | 8 / 8 (100,0 %) |
| `html/semantics/scripting-1/the-template-element/definitions` | 2 / 4 | 13 / 18 (72,2 %) |
| `html/semantics/scripting-1/the-template-element/innerhtml-on-templates` | 0 / 1 | 3 / 4 (75,0 %) |
| `html/semantics/scripting-1/the-template-element/serializing-html-templates` | 0 / 1 | 0 / 3 (0,0 %) |
| `html/semantics/scripting-1/the-template-element/template-element` | 3 / 9 | 229 / 252 (90,9 %) |
| `html/semantics/scripting-1/the-template-element/template-for` | 9 / 53 | 18 / 537 (3,4 %) |
| `html/semantics/sections` | 0 / 2 | 0 / 107 (0,0 %) |
| `html/semantics/selectors/case-sensitivity` | 0 / 1 | 184 / 276 (66,7 %) |
| `html/semantics/selectors/pseudo-classes` | 9 / 33 | 57 / 137 (41,6 %) |
| `html/semantics/tabular-data` | 1 / 1 | 5 / 5 (100,0 %) |
| `html/semantics/tabular-data/attributes-common-to-td-and-th-elements` | 0 / 1 | 5 / 6 (83,3 %) |
| `html/semantics/tabular-data/processing-model-1` | 2 / 4 | 6 / 15 (40,0 %) |
| `html/semantics/tabular-data/the-caption-element` | 1 / 1 | 5 / 5 (100,0 %) |
| `html/semantics/tabular-data/the-table-element` | 6 / 15 | 55 / 72 (76,4 %) |
| `html/semantics/tabular-data/the-tbody-element` | 2 / 3 | 12 / 13 (92,3 %) |
| `html/semantics/tabular-data/the-tfoot-element` | 1 / 1 | 1 / 1 (100,0 %) |
| `html/semantics/tabular-data/the-thead-element` | 1 / 1 | 1 / 1 (100,0 %) |
| `html/semantics/tabular-data/the-tr-element` | 2 / 5 | 36 / 45 (80,0 %) |
| `html/semantics/text-level-semantics` | 1 / 1 | 3 / 3 (100,0 %) |
| `html/semantics/text-level-semantics/the-a-element` | 5 / 7 | 23 / 26 (88,5 %) |
| `html/semantics/text-level-semantics/the-time-element` | 0 / 1 | 4 / 8 (50,0 %) |
| `html/semantics/the-button-element/command-and-commandfor` | 2 / 26 | 212 / 482 (44,0 %) |
| `html/syntax/charset` | 1 / 12 | 4 / 39 (10,3 %) |
| `html/syntax/parsing` | 16 / 47 | 508 / 2416 (21,0 %) |
| `html/syntax/parsing-html-fragments` | 10 / 14 | 12 / 27 (44,4 %) |
| `html/syntax/parsing-xml-fragments` | 0 / 1 | 0 / 5 (0,0 %) |
| `html/syntax/parsing/template/additions-to-foster-parenting` | 2 / 2 | 4 / 4 (100,0 %) |
| `html/syntax/parsing/template/additions-to-the-in-body-insertion-mode` | 6 / 8 | 37 / 42 (88,1 %) |
| `html/syntax/parsing/template/additions-to-the-in-frameset-insertion-mode` | 0 / 1 | 0 / 1 (0,0 %) |
| `html/syntax/parsing/template/additions-to-the-in-head-insertion-mode` | 1 / 2 | 7 / 12 (58,3 %) |
| `html/syntax/parsing/template/additions-to-the-in-table-insertion-mode` | 0 / 1 | 0 / 1 (0,0 %) |
| `html/syntax/parsing/template/appending-to-a-template` | 0 / 1 | 0 / 4 (0,0 %) |
| `html/syntax/parsing/template/clearing-the-stack-back-to-a-given-context` | 0 / 3 | 12 / 42 (28,6 %) |
| `html/syntax/parsing/template/creating-an-element-for-the-token` | 0 / 1 | 205 / 223 (91,9 %) |
| `html/syntax/serializing-html-fragments` | 2 / 8 | 292 / 375 (77,9 %) |
| `html/syntax/serializing-xml-fragments` | 0 / 1 | 1 / 112 (0,9 %) |
| `html/syntax/speculative-charset` | 0 / 1 | 0 / 1 (0,0 %) |
| `html/syntax/speculative-parsing/expect-no-linked-resources` | 0 / 1 | 0 / 1 (0,0 %) |
| `html/syntax/speculative-parsing/generated/document-write` | 0 / 63 | 0 / 63 (0,0 %) |
| `html/syntax/speculative-parsing/generated/page-load` | 0 / 63 | 0 / 63 (0,0 %) |
| `html/syntax/xmldecl` | 0 / 3 | 7 / 88 (8,0 %) |
| `html/the-xhtml-syntax/parsing-xhtml-documents` | 0 / 13 | 3454 / 25607 (13,5 %) |
| `html/user-activation` | 1 / 21 | 5 / 31 (16,1 %) |
| `html/webappapis/animation-frames` | 7 / 12 | 7 / 16 (43,8 %) |
| `html/webappapis/atob` | 0 / 1 | 285 / 286 (99,7 %) |
| `html/webappapis/dynamic-markup-insertion/closing-the-input-stream` | 0 / 3 | 0 / 3 (0,0 %) |
| `html/webappapis/dynamic-markup-insertion/document-write` | 16 / 88 | 18 / 96 (18,8 %) |
| `html/webappapis/dynamic-markup-insertion/document-writeln` | 0 / 3 | 1 / 5 (20,0 %) |
| `html/webappapis/dynamic-markup-insertion/html-unsafe-methods` | 0 / 13 | 10 / 95 (10,5 %) |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream` | 13 / 53 | 61 / 175 (34,9 %) |
| `html/webappapis/dynamic-markup-insertion/the-innerhtml-property` | 0 / 2 | 3 / 16 (18,8 %) |
| `html/webappapis/dynamic-markup-insertion/the-outerhtml-property` | 2 / 2 | 2 / 2 (100,0 %) |
| `html/webappapis/events` | 1 / 2 | 1 / 2 (50,0 %) |
| `html/webappapis/microtask-queuing` | 3 / 4 | 8 / 9 (88,9 %) |
| `html/webappapis/scripting` | 0 / 2 | 2 / 6 (33,3 %) |
| `html/webappapis/scripting/event-loops` | 4 / 8 | 5 / 9 (55,6 %) |
| `html/webappapis/scripting/events` | 6 / 35 | 43 / 109 (39,4 %) |
| `html/webappapis/scripting/events/event-handler-processing-algorithm-error` | 10 / 12 | 24 / 24 (100,0 %) |
| `html/webappapis/scripting/processing-model-2` | 6 / 36 | 11 / 55 (20,0 %) |
| `html/webappapis/scripting/processing-model-2/integration-with-the-javascript-agent-formalism` | 0 / 2 | 0 / 3 (0,0 %) |
| `html/webappapis/scripting/processing-model-2/integration-with-the-javascript-job-queue` | 1 / 4 | 14 / 31 (45,2 %) |
| `html/webappapis/scripting/processing-model-2/unhandled-promise-rejections` | 2 / 14 | 79 / 103 (76,7 %) |
| `html/webappapis/structured-clone` | 0 / 2 | 94 / 156 (60,3 %) |
| `html/webappapis/system-state-and-capabilities/the-navigator-object` | 8 / 19 | 34 / 310 (11,0 %) |
| `html/webappapis/the-windoworworkerglobalscope-mixin` | 0 / 1 | 2 / 6 (33,3 %) |
| `html/webappapis/timers` | 10 / 17 | 16 / 27 (59,3 %) |
| `html/webappapis/update-rendering` | 0 / 1 | 0 / 1 (0,0 %) |
| `html/webappapis/user-prompts` | 1 / 3 | 1 / 3 (33,3 %) |

## Fichiers non entièrement réussis

| Fichier | Sous-tests | Cause |
|---|---:|---|
| `html/anonymous-iframe/anonymous-iframe-popup.tentative.https.window.js` | 0 / 2 |  |
| `html/anonymous-iframe/anonymous-window.tentative.https.window.js` | 0 / 3 |  |
| `html/anonymous-iframe/cache-storage.tentative.https.window.js` | 0 / 1 |  |
| `html/anonymous-iframe/cookie-store.tentative.https.window.js` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/anonymous-iframe/cookie.tentative.https.window.js` | 0 / 1 |  |
| `html/anonymous-iframe/embedding.tentative.https.window.js` | 0 / 0 | boucle sans fin (processus tué) |
| `html/anonymous-iframe/hasStorageAccess.tentative.https.window.js` | 0 / 0 | harnais : Error: assert_implements: requestStorageAccess is not supported. undefined |
| `html/anonymous-iframe/indexeddb.tentative.https.window.js` | 0 / 1 |  |
| `html/anonymous-iframe/initial-empty-document.tentative.https.window.js` | 0 / 2 |  |
| `html/anonymous-iframe/local-storage-initial-empty-document.tentative.https.window.js` | 0 / 1 |  |
| `html/anonymous-iframe/local-storage.tentative.https.window.js` | 0 / 1 |  |
| `html/anonymous-iframe/requestStorageAccess.tentative.https.window.js` | 0 / 0 | harnais : Error: assert_implements: requestStorageAccess is not supported. undefined |
| `html/anonymous-iframe/require-corp-embed-anonymous-iframe.tentative.https.window.js` | 0 / 1 |  |
| `html/anonymous-iframe/serviceworker-partitioning.tentative.https.window.js` | 0 / 1 |  |
| `html/anonymous-iframe/session-storage.tentative.https.window.js` | 0 / 1 |  |
| `html/anonymous-iframe/sharedworker-partitioning.tentative.https.window.js` | 0 / 1 |  |
| `html/anonymous-iframe/web-lock.tentative.https.window.js` | 0 / 1 |  |
| `html/anonymous-iframe/worker-cookies.tentative.https.window.js` | 0 / 3 |  |
| `html/browsers/browsing-the-web/back-forward-cache/broadcastchannel/broadcastchannel.window.js` | 0 / 1 |  |
| `html/browsers/browsing-the-web/back-forward-cache/broadcastchannel/dedicatedworker.tentative.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/back-forward-cache/broadcastchannel/evict-on-message.tentative.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/back-forward-cache/eligibility/broadcast-channel.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/back-forward-cache/eligibility/dedicated-worker.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/back-forward-cache/eligibility/inflight-fetch-1.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/back-forward-cache/eligibility/inflight-fetch-2.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/back-forward-cache/eligibility/inflight-fetch-cors.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/back-forward-cache/eligibility/inflight-fetch-redirects.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/back-forward-cache/eligibility/shared-worker-active-client.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/back-forward-cache/eligibility/shared-worker.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/back-forward-cache/events.html` | 0 / 5 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/back-forward-cache/focus.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/back-forward-cache/pagehide-event-handler-microtasks.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/back-forward-cache/pushstate.https.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/back-forward-cache/service-worker-client-postmessage.https.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/back-forward-cache/service-worker-clients-claim.https.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/back-forward-cache/service-worker-clients-matchall.https.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/back-forward-cache/service-worker-controlled-after-restore.https.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/back-forward-cache/service-worker-unregister.https.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/back-forward-cache/storage-events.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/back-forward-cache/timers.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/back-forward-cache/weblocks-worker.https.window.js` | 0 / 1 |  |
| `html/browsers/browsing-the-web/history-traversal/001.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/history-traversal/PopStateEvent.html` | 3 / 5 |  |
| `html/browsers/browsing-the-web/history-traversal/api-availability.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/history-traversal/browsing_context_name.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/history-traversal/browsing_context_name_cross_origin.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/history-traversal/browsing_context_name_cross_origin_2.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/history-traversal/browsing_context_name_cross_origin_3.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/history-traversal/document-state.https.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/history-traversal/event-order/after-load-hash-twice.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/history-traversal/event-order/after-load-hash.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/history-traversal/event-order/after-load-replaceState.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/history-traversal/event-order/before-load-hash-twice.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/history-traversal/event-order/before-load-hash.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/history-traversal/event-order/pushState-inside-popstate.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/history-traversal/event-order/same-document-traverse-immediate.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/history-traversal/event-order/same-document-traverse-wait.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/history-traversal/event-order/same-url-fragment-link-click.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/history-traversal/hashchange_event.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/history-traversal/history-traversal-navigate-parent-while-child-loading.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/history-traversal/history-traversal-navigates-multiple-frames.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/history-traversal/iframe-history-restore-about-blank.tentative.html` | 0 / 3 | harnais : Test named 'initial about:blank is loaded sync when container traverses back' specified 1 'cleanup' function, and 1 fail |
| `html/browsers/browsing-the-web/history-traversal/iframe-history-restore-on-container-traversal.tentative.html` | 0 / 2 | harnais : Test named 'iframe is restored to JS-navigated URL when container traverses back' specified 1 'cleanup' function, and 1  |
| `html/browsers/browsing-the-web/history-traversal/iframe-navigated-during-container-load.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/history-traversal/pagereveal/order-in-bfcache-restore-iframe.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/history-traversal/pagereveal/order-in-bfcache-restore.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/history-traversal/pagereveal/order-in-new-document-navigation-iframe.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/history-traversal/pagereveal/order-in-new-document-navigation.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/history-traversal/pagereveal/order-in-prerender-activation.https.html` | 0 / 0 | harnais : ReferenceError: assertSpeculationRulesIsSupported n'est pas défini |
| `html/browsers/browsing-the-web/history-traversal/pageswap/pageswap-cross-origin.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/history-traversal/pageswap/pageswap-iframe.html` | 1 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/history-traversal/pageswap/pageswap-initial-navigation.html` | 1 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/history-traversal/pageswap/pageswap-push-from-click.html` | 0 / 2 | harnais : Uncaught TypeError: impossible de lire la propriété « addEventListener » de null (valeur de window) |
| `html/browsers/browsing-the-web/history-traversal/pageswap/pageswap-push-navigation-hidden-document.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/history-traversal/pageswap/pageswap-push-navigation.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/history-traversal/pageswap/pageswap-push-with-cross-origin-redirect.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/history-traversal/pageswap/pageswap-push-with-redirect.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/history-traversal/pageswap/pageswap-reload-navigation.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/history-traversal/pageswap/pageswap-replace-navigation.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/history-traversal/pageswap/pageswap-replace-with-cross-origin-redirect.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/history-traversal/pageswap/pageswap-traverse-navigation-cross-origin-redirect-no-bfcache.https.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/history-traversal/pageswap/pageswap-traverse-navigation-no-bfcache.https.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/history-traversal/persisted-user-state-restoration/resume-timer-on-history-back.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/history-traversal/persisted-user-state-restoration/scroll-restoration-basic.html` | 2 / 3 |  |
| `html/browsers/browsing-the-web/history-traversal/persisted-user-state-restoration/scroll-restoration-fragment-scrolling-cross-origin.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/history-traversal/persisted-user-state-restoration/scroll-restoration-fragment-scrolling-samedoc.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/history-traversal/persisted-user-state-restoration/scroll-restoration-navigation-cross-origin.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/history-traversal/persisted-user-state-restoration/scroll-restoration-navigation-samedoc.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/history-traversal/popstate_event.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/history-traversal/same-url.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/history-traversal/scroll-restoration-order.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/history-traversal/srcdoc/consecutive-srcdoc.html` | 0 / 2 | harnais : Test named 'changing srcdoc does a replace navigation since the URL is still about:srcdoc' specified 1 'cleanup' functio |
| `html/browsers/browsing-the-web/history-traversal/srcdoc/srcdoc-history-entries.html` | 0 / 2 | harnais : Test named 'srcdoc history entries: the iframe itself navigates' specified 1 'cleanup' function, and 1 failed. |
| `html/browsers/browsing-the-web/history-traversal/unset_context_name.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/history-traversal/window-name-after-cross-origin-aux-frame-navigation.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/history-traversal/window-name-after-cross-origin-main-frame-navigation.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/history-traversal/window-name-after-cross-origin-sub-frame-navigation.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/history-traversal/window-name-after-same-origin-aux-frame-navigation.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/history-traversal/window-name-after-same-origin-main-frame-navigation.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/history-traversal/window-name-after-same-origin-sub-frame-navigation.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/003.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/004.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/005.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/006.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/007.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/008.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/011.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/012.html` | 0 / 2 | harnais : Uncaught Error: InvalidAccessError : les requêtes synchrones ne sont pas prises en charge |
| `html/browsers/browsing-the-web/navigating-across-documents/013.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/014.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/015.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/abort-document-load.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/abort-document-window-stop.sub.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/about-srcdoc-navigation-blocked.window.js` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/anchor-fragment-form-submit-longfragment.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/anchor-fragment-form-submit-withpath.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/anchor-fragment-form-submit.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/anchor-jsurl-form-submit.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/child-navigates-parent-cross-origin.window.js` | 0 / 11 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/child-navigates-parent-same-origin.window.js` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/cross-origin-top-navigation-with-user-activation-in-parent.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/cross-origin-top-navigation-with-user-activation.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/cross-origin-top-navigation-without-user-activation-nested.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/cross-origin-top-navigation-without-user-activation.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/failure-check-sequence.https.html` | 0 / 4 | harnais : Test named 'CSP check precedes X-Frame-Options check' specified 1 'cleanup' function, and 1 failed. |
| `html/browsers/browsing-the-web/navigating-across-documents/fragment-replace.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/grandparent_location_aboutsrcdoc.sub.window.js` | 0 / 6 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/grandparent_session_aboutsrcdoc.sub.window.js` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/initial-empty-document/iframe-nosrc.html` | 0 / 7 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/initial-empty-document/iframe-src-204-fragment.html` | 0 / 5 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/initial-empty-document/iframe-src-204-pushState-replaceState.html` | 0 / 2 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/initial-empty-document/iframe-src-204.html` | 1 / 7 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/initial-empty-document/iframe-src-aboutblank-navigate-immediately.html` | 1 / 6 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/initial-empty-document/iframe-src-aboutblank-wait-for-load.html` | 1 / 6 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/initial-empty-document/initial-content-replacement.html` | 5 / 10 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/initial-empty-document/load-event-iframe-element.html` | 0 / 5 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/initial-empty-document/load-pageshow-events-window-open.html` | 0 / 5 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/initial-empty-document/window-open-204-fragment.html` | 0 / 2 | harnais : Test named 'location.href' specified 1 'cleanup' function, and 1 failed. |
| `html/browsers/browsing-the-web/navigating-across-documents/initial-empty-document/window-open-204-pushState-replaceState.html` | 0 / 2 | harnais : Test named 'history.pushState' specified 1 'cleanup' function, and 1 failed. |
| `html/browsers/browsing-the-web/navigating-across-documents/initial-empty-document/window-open-204.html` | 0 / 2 | harnais : Test named 'location.href' specified 1 'cleanup' function, and 1 failed. |
| `html/browsers/browsing-the-web/navigating-across-documents/initial-empty-document/window-open-aboutblank.html` | 0 / 2 | harnais : Test named 'location.href' specified 1 'cleanup' function, and 1 failed. |
| `html/browsers/browsing-the-web/navigating-across-documents/initial-empty-document/window-open-history-length.html` | 0 / 3 | harnais : Test named 'Starting history.length for window.open()' specified 1 'cleanup' function, and 1 failed. |
| `html/browsers/browsing-the-web/navigating-across-documents/initial-empty-document/window-open-nourl.html` | 0 / 2 | harnais : Test named 'location.href' specified 1 'cleanup' function, and 1 failed. |
| `html/browsers/browsing-the-web/navigating-across-documents/javascript-url-abort/javascript-url-abort-return-value-string.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/javascript-url-abort/javascript-url-abort-return-value-undefined.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/javascript-url-global-scope.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/javascript-url-load-as-html.xhtml` | 0 / 1 | harnais : Test named 'javascript: URL navigation to a string must create a HTML document using the correct properties' specified 1 |
| `html/browsers/browsing-the-web/navigating-across-documents/javascript-url-nested-history.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/javascript-url-no-beforeunload.window.js` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/javascript-url-object-embed-history.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/javascript-url-query-fragment-components.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/javascript-url-referrer.window.js` | 0 / 3 | harnais : Test named 'unsafe-url referrer policy used to create the starting page' specified 2 'cleanup' functions, and 1 failed. |
| `html/browsers/browsing-the-web/navigating-across-documents/javascript-url-return-value-handling-dynamic.html` | 0 / 10 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/javascript-url-return-value-handling.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/javascript-url-security-check-multi-globals.sub.html` | 0 / 2 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/javascript-url-security-check-same-origin-domain.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/javascript-url-side-effects.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/javascript-url-src-removes-container.html` | 0 / 2 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/javascript-url-task-queuing.html` | 0 / 5 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/multiple-globals/context-for-location-assign.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/multiple-globals/context-for-location-href.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/multiple-globals/context-for-location.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/multiple-globals/context-for-window-open.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/navigate-cross-origin-iframe-to-same-url-with-fragment-fire-load-event.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/navigate-cross-origin-iframe-to-same-url-with-fragment.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/navigate-cross-origin-iframe-to-same-url.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/navigate-iframe-to-same-url-with-document-domain.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/navigate-to-unparseable-url.html` | 0 / 2 | harnais : Test named 'location.href setter throws a SyntaxError DOMException for unparseable URLs' specified 1 'cleanup' function, |
| `html/browsers/browsing-the-web/navigating-across-documents/navigate_too_many_calls.optional.html` | 0 / 2 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/navigation-unload-data-url.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/navigation-unload-form-submit.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/navigation-unload-javascript-url.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/navigation-unload-same-origin-fragment.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/plugin-document.historical.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/redirect-to-about.window.js` | 0 / 6 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/redirect-to-data.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/redirect-to-unparseable-url.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/refresh/after-same-document-navigation.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/refresh/navigate.window.js` | 0 / 2 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/refresh/same-document-refresh.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/refresh/subresource.any.js` | 0 / 2 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/a-click-during-load.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/a-click-during-pageshow.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/a-click.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/a-fragment-click-during-load.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/a-user-click-during-load.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/a-user-click-during-pageshow.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/a-user-click.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/form-requestsubmit-during-load.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/form-requestsubmit-during-pageshow.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/form-requestsubmit.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/form-submit-button-click-during-load.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/form-submit-button-click-during-pageshow.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/form-submit-button-click.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/form-submit-cross-frame-crossorigin.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/form-submit-cross-frame.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/form-submit-during-load.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/form-submit-during-pageshow.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/form-submit-popup-crossorigin.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/form-submit-popup.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/form-submit.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/history-pushstate-during-load.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/history-pushstate-during-pageshow.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/history-pushstate.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/iframe-src-during-load.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/iframe-src-during-pageshow.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/iframe-src.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/location-assign-during-load.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/location-assign-during-pageshow.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/location-assign-user-click.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/location-assign.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/location-setter-during-load.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/location-setter-during-pageshow.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/location-setter-user-click.html` | 0 / 3 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/location-setter-user-mouseup.html` | 0 / 3 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/location-setter.html` | 0 / 3 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/window-open-popup-during-load.html` | 0 / 1 | harnais : Test named 'No replace before load, triggered by window.open() on a non-_self window' specified 1 'cleanup' function, an |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/window-open-popup-during-pageshow.html` | 0 / 1 | harnais : Test named 'No replace before load, triggered by window.open() on a non-_self window' specified 1 'cleanup' function, an |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/window-open-popup.html` | 0 / 1 | harnais : Test named 'No replace before load, triggered by window.open() on a non-_self window' specified 1 'cleanup' function, an |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/window-open-self-during-load.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/window-open-self-during-pageshow.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/replace-before-load/window-open-self.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/same-origin-top-navigation-without-user-activation.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/source/navigate-child-function-parent-then-fragment.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/source/navigate-child-function-parent.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/source/navigate-child-function-src.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/source/navigate-child-function.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/source/navigate-child-src-about-blank.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/navigating-across-documents/top-level-data-url.window.js` | 4 / 8 |  |
| `html/browsers/browsing-the-web/navigating-across-documents/window-stop-in-image-load-handler.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/overlapping-navigations-and-traversals/anchor-fragment-history-back-on-click.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/overlapping-navigations-and-traversals/cross-document-nav-cross-document-nav.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/overlapping-navigations-and-traversals/cross-document-nav-cross-document-traversal.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/overlapping-navigations-and-traversals/cross-document-nav-same-document-nav.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/overlapping-navigations-and-traversals/cross-document-nav-same-document-traversal.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/overlapping-navigations-and-traversals/cross-document-traversal-cross-document-nav.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/overlapping-navigations-and-traversals/cross-document-traversal-cross-document-traversal.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/overlapping-navigations-and-traversals/cross-document-traversal-same-document-nav.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/overlapping-navigations-and-traversals/cross-document-traversal-same-document-traversal.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/overlapping-navigations-and-traversals/cross-document-traversal-stop.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/overlapping-navigations-and-traversals/forward-to-pruned-entry.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/overlapping-navigations-and-traversals/nav-cancelation-1.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/overlapping-navigations-and-traversals/nav-cancelation-2.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/overlapping-navigations-and-traversals/same-document-nav-cross-document-nav.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/overlapping-navigations-and-traversals/same-document-nav-cross-document-traversal.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/overlapping-navigations-and-traversals/same-document-nav-same-document-traversal.html` | 0 / 2 |  |
| `html/browsers/browsing-the-web/overlapping-navigations-and-traversals/same-document-traversal-cross-document-nav.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/overlapping-navigations-and-traversals/same-document-traversal-cross-document-traversal.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/overlapping-navigations-and-traversals/same-document-traversal-same-document-nav.html` | 0 / 2 |  |
| `html/browsers/browsing-the-web/overlapping-navigations-and-traversals/same-document-traversal-same-document-traversal-hashchange.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/overlapping-navigations-and-traversals/same-document-traversal-same-document-traversal-pushstate.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/overlapping-navigations-and-traversals/same-document-traversal-stop.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/read-media/cross-origin-video.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/read-media/pageload-image-in-popup.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/read-media/pageload-image.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/read-media/pageload-video.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/read-text/load-text-plain.html` | 0 / 3 |  |
| `html/browsers/browsing-the-web/remote-context-helper-tests/addEmbed.window.js` | 0 / 1 |  |
| `html/browsers/browsing-the-web/remote-context-helper-tests/addFrame.window.js` | 0 / 1 |  |
| `html/browsers/browsing-the-web/remote-context-helper-tests/addHTML.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/remote-context-helper-tests/addIframe-srcdoc-startOn.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/remote-context-helper-tests/addIframe-srcdoc.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/remote-context-helper-tests/addIframe-urlType.window.js` | 0 / 1 |  |
| `html/browsers/browsing-the-web/remote-context-helper-tests/addIframe.window.js` | 0 / 1 |  |
| `html/browsers/browsing-the-web/remote-context-helper-tests/addObject.window.js` | 0 / 1 |  |
| `html/browsers/browsing-the-web/remote-context-helper-tests/addScripts.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/remote-context-helper-tests/addSharedWorker.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/remote-context-helper-tests/addSharedWorkerNull.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/remote-context-helper-tests/addWindow-defaults.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/remote-context-helper-tests/addWindow-extra-config.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/remote-context-helper-tests/addWindow-features.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/remote-context-helper-tests/addWindow-from-window.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/remote-context-helper-tests/addWindow-startOn.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/remote-context-helper-tests/addWindow-target.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/remote-context-helper-tests/addWindow-urlType.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/remote-context-helper-tests/addWorker.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/remote-context-helper-tests/addWorkerNull.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/remote-context-helper-tests/constructor.window.js` | 0 / 1 |  |
| `html/browsers/browsing-the-web/remote-context-helper-tests/getRequestHeaders.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/remote-context-helper-tests/navigateToNew.window.js` | 0 / 1 |  |
| `html/browsers/browsing-the-web/remote-context-helper-tests/navigation-bfcache.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/remote-context-helper-tests/navigation-helpers.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/remote-context-helper-tests/navigation-same-document.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/scroll-to-fragid/003.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/scroll-to-fragid/embedded-svg-document-fragment-does-not-scroll-parent.tentative.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/scroll-to-fragid/embedded-svg-document-fragment-does-not-scroll-svg-document.tentative.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/scroll-to-fragid/focus-changes-after-scroll-to-fragment.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/scroll-to-fragid/forward-triggers-hashchange.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/scroll-to-fragid/fragment-and-encoding-2.html` | 0 / 3 |  |
| `html/browsers/browsing-the-web/scroll-to-fragid/fragment-and-encoding.html` | 2 / 4 |  |
| `html/browsers/browsing-the-web/scroll-to-fragid/html-fragment-in-svg-foreign-object-scrolls-ancestors.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/scroll-to-fragid/replacement-enabled.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/scroll-to-fragid/scroll-frag-non-utf8-encoded-document.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/scroll-to-fragid/scroll-frag-percent-encoded.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/scroll-to-fragid/scroll-position-inline-nearest.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/scroll-to-fragid/scroll-position-vertical-lr.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/scroll-to-fragid/scroll-position-vertical-rl.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/scroll-to-fragid/scroll-position.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/scroll-to-fragid/scroll-restore-on-reload-non-ascii-fragment.html` | 0 / 2 |  |
| `html/browsers/browsing-the-web/scroll-to-fragid/scroll-to-anchor-name-quirks-case.html` | 3 / 6 |  |
| `html/browsers/browsing-the-web/scroll-to-fragid/scroll-to-anchor-name.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/scroll-to-fragid/scroll-to-fragid-from-link-is-synchronous.html` | 0 / 2 |  |
| `html/browsers/browsing-the-web/scroll-to-fragid/scroll-to-id-top.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/scroll-to-fragid/scroll-to-top.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/scroll-to-fragid/target-pseudo-after-adoption.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/scroll-to-fragid/target-pseudo-after-reinsertion.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/unloading-documents/001.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/unloading-documents/002.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/unloading-documents/003.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/unloading-documents/004.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/unloading-documents/005.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/unloading-documents/beforeunload-canceling.html` | 3 / 22 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/unloading-documents/beforeunload-on-history-back.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/unloading-documents/beforeunload-on-navigation-of-parent.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/unloading-documents/beforeunload-sticky-activation-iframe.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/unloading-documents/beforeunload-sticky-activation-popup.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/unloading-documents/beforeunload-synchronous.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/unloading-documents/navigation-during-beforeunload.html` | 0 / 6 | harnais : Uncaught SyntaxError: adresse invalide : navigation-target.html |
| `html/browsers/browsing-the-web/unloading-documents/navigation-within-beforeunload.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/unloading-documents/pagehide-on-history-forward.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/unloading-documents/prompt-and-unload-script-closeable.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/unloading-documents/prompt/001.html` | 0 / 1 |  |
| `html/browsers/browsing-the-web/unloading-documents/prompt/004.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/unloading-documents/unload/001.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/unloading-documents/unload/002.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/unloading-documents/unload/003.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/unloading-documents/unload/004.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/unloading-documents/unload/006.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/unloading-documents/unload/007.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/unloading-documents/unload/008.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/unloading-documents/unload/009.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/unloading-documents/unload/unload-main-frame-cross-origin.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/browsing-the-web/unloading-documents/unload/unload-main-frame-same-origin.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/joint-session-history/joint-session-history-only-fully-active.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/001.html` | 4 / 7 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/002.html` | 3 / 5 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/004.html` | 1 / 4 |  |
| `html/browsers/history/the-history-interface/005.html` | 1 / 3 |  |
| `html/browsers/history/the-history-interface/007.html` | 4 / 8 |  |
| `html/browsers/history/the-history-interface/008.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/009.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/010.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/back-pushstate-back-history-state.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/combination_history_004.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/combination_history_005.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/combination_history_006.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/combination_history_007.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/history-associated-with-document.window.js` | 2 / 4 |  |
| `html/browsers/history/the-history-interface/history-state-after-bfcache.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/history_back.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/history_back_1.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/history_back_cross_realm_method.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/history_forward.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/history_forward_1.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/history_forward_cross_realm_method.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/history_go_cross_realm_method.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/history_go_minus.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/history_go_no_argument.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/history_go_plus.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/history_go_to_uri.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/history_go_too_many_calls.optional.html` | 0 / 2 |  |
| `html/browsers/history/the-history-interface/history_go_undefined.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/history_go_zero.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/history_go_zero_which_document.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/history_properties_only_fully_active.html` | 0 / 1 |  |
| `html/browsers/history/the-history-interface/history_pushstate_too_many_calls.optional.html` | 0 / 1 |  |
| `html/browsers/history/the-history-interface/history_pushstate_too_many_calls_ordering.optional.html` | 0 / 4 |  |
| `html/browsers/history/the-history-interface/history_pushstate_url.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/history_pushstate_url_rewriting.html` | 22 / 35 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/history_replacestate_too_many_calls.optional.html` | 0 / 1 |  |
| `html/browsers/history/the-history-interface/history_replacestate_too_many_calls_ordering.optional.html` | 0 / 4 |  |
| `html/browsers/history/the-history-interface/iframe_history_go_0.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/joint_session_history/001.html` | 0 / 7 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/joint_session_history/002.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/pushstate-replacestate-empty-string/pushstate.html` | 0 / 1 |  |
| `html/browsers/history/the-history-interface/pushstate-replacestate-empty-string/replacestate.html` | 0 / 1 |  |
| `html/browsers/history/the-history-interface/traverse-during-beforeunload.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/traverse-during-unload.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/traverse_the_history_1.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/traverse_the_history_2.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/traverse_the_history_3.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/traverse_the_history_4.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/traverse_the_history_5.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/traverse_the_history_unload_1.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/traverse_the_history_write_after_load_1.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/traverse_the_history_write_after_load_2.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/traverse_the_history_write_onload_1.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-history-interface/traverse_the_history_write_onload_2.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-location-interface/allow_prototype_cycle_through_location.sub.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-location-interface/assign-replace-from-iframe.html` | 0 / 2 |  |
| `html/browsers/history/the-location-interface/assign-replace-from-top-to-nested-iframe.html` | 0 / 2 | harnais : Uncaught SyntaxError: adresse invalide : /resources/blank.html |
| `html/browsers/history/the-location-interface/assign-with-nested-iframe.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-location-interface/assign_after_load.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-location-interface/assign_before_load.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-location-interface/create-script-set-location.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-location-interface/document_location.html` | 2 / 4 |  |
| `html/browsers/history/the-location-interface/location-ancestor-origins-inactive-document.sub.html` | 0 / 2 |  |
| `html/browsers/history/the-location-interface/location-ancestor-origins-new-object.html` | 0 / 1 |  |
| `html/browsers/history/the-location-interface/location-ancestor-origins-redirect-still-mask.sub.html` | 0 / 16 |  |
| `html/browsers/history/the-location-interface/location-ancestor-origins-referrerpolicy-snapshot.html` | 0 / 2 |  |
| `html/browsers/history/the-location-interface/location-ancestor-origins-referrerpolicy-unsupported-elements.html` | 1 / 3 |  |
| `html/browsers/history/the-location-interface/location-ancestor-origins.sub.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-location-interface/location-encoding-parse.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-location-interface/location-hash-about-blank.html` | 0 / 1 |  |
| `html/browsers/history/the-location-interface/location-hash-setter-empty-string.html` | 0 / 1 |  |
| `html/browsers/history/the-location-interface/location-non-configurable-toString-valueOf.html` | 0 / 2 |  |
| `html/browsers/history/the-location-interface/location-origin-idna.sub.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-location-interface/location-pathname-setter-question-mark.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-location-interface/location-prevent-extensions.html` | 0 / 2 |  |
| `html/browsers/history/the-location-interface/location-protocol-setter-non-broken.html` | 5 / 10 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-location-interface/location-protocol-setter-sameish.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-location-interface/location-protocol-setter-with-colon.sub.html` | 2 / 3 |  |
| `html/browsers/history/the-location-interface/location-protocol-setter.html` | 0 / 48 |  |
| `html/browsers/history/the-location-interface/location-prototype-setting-cross-origin-domain.sub.html` | 2 / 8 |  |
| `html/browsers/history/the-location-interface/location-prototype-setting-cross-origin.sub.html` | 2 / 8 |  |
| `html/browsers/history/the-location-interface/location-prototype-setting-goes-cross-origin-domain.sub.html` | 3 / 12 |  |
| `html/browsers/history/the-location-interface/location-prototype-setting-same-origin-domain.sub.html` | 4 / 8 |  |
| `html/browsers/history/the-location-interface/location-prototype-setting-same-origin.html` | 4 / 8 |  |
| `html/browsers/history/the-location-interface/location-replace-cross-site.html` | 0 / 10 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-location-interface/location-stringifier.html` | 3 / 6 |  |
| `html/browsers/history/the-location-interface/location-symbol-toprimitive.html` | 0 / 1 |  |
| `html/browsers/history/the-location-interface/location-valueof.html` | 0 / 1 |  |
| `html/browsers/history/the-location-interface/location_assign.html` | 0 / 2 |  |
| `html/browsers/history/the-location-interface/location_hash.html` | 5 / 6 |  |
| `html/browsers/history/the-location-interface/location_port.html` | 0 / 1 |  |
| `html/browsers/history/the-location-interface/location_reload.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-location-interface/location_reload_javascript_url.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-location-interface/location_replace.html` | 0 / 2 |  |
| `html/browsers/history/the-location-interface/location_replace_session_history.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-location-interface/location_search.html` | 1 / 2 |  |
| `html/browsers/history/the-location-interface/no-browsing-context.window.js` | 34 / 46 |  |
| `html/browsers/history/the-location-interface/reload_document_open_write.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-location-interface/reload_document_write.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-location-interface/reload_document_write_onload.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-location-interface/replace-with-nested-iframe.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-location-interface/same-hash.html` | 0 / 12 |  |
| `html/browsers/history/the-location-interface/scripted_click_assign_during_load.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-location-interface/scripted_click_location_assign_during_load.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-location-interface/scripted_form_submit_assign_during_load.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/history/the-location-interface/security_location_0.htm` | 0 / 0 | le harnais n'a pas terminé |
| `html/browsers/history/the-session-history-of-browsing-contexts/navigation-in-onload.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/origin/api/file-scheme-origin.tentative.any.js` | 0 / 1 |  |
| `html/browsers/origin/api/origin-comparison.any.js` | 0 / 4 |  |
| `html/browsers/origin/api/origin-from-global.any.js` | 0 / 1 |  |
| `html/browsers/origin/api/origin-from-hyperlinkelementutils.window.js` | 0 / 129 |  |
| `html/browsers/origin/api/origin-from-location.window.js` | 0 / 2 |  |
| `html/browsers/origin/api/origin-from-messageevent-opaque.window.js` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/browsers/origin/api/origin-from-messageevent.window.js` | 0 / 6 | harnais : délai du harnais dépassé |
| `html/browsers/origin/api/origin-from-origin.any.js` | 0 / 25 |  |
| `html/browsers/origin/api/origin-from-string.any.js` | 0 / 27 |  |
| `html/browsers/origin/api/origin-from-url.any.js` | 0 / 27 |  |
| `html/browsers/origin/api/origin-from-window.window.js` | 0 / 7 | harnais : délai du harnais dépassé |
| `html/browsers/origin/api/origin-from-worker.window.js` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/browsers/origin/api/origin-from.any.js` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/browsers/origin/cross-origin-objects/cross-origin-due-to-document-domain-only.html` | 0 / 1 |  |
| `html/browsers/origin/cross-origin-objects/cross-origin-objects-function-length.html` | 3 / 4 |  |
| `html/browsers/origin/cross-origin-objects/cross-origin-objects-function-name.html` | 2 / 4 |  |
| `html/browsers/origin/cross-origin-objects/cross-origin-objects-on-new-window.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/browsers/origin/cross-origin-objects/cross-origin-objects.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/browsers/origin/cross-origin-objects/location-properties-smoke-test.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/origin/cross-origin-objects/window-location-and-location-href-cross-realm-set.html` | 0 / 11 | harnais : délai du harnais dépassé |
| `html/browsers/origin/inheritance/about-blank-history-state-after-traversal.window.js` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/browsers/origin/inheritance/about-blank-iframe-navigated-by-ancestor.window.js` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/browsers/origin/inheritance/about-blank-iframe-navigated-by-itself.window.js` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/browsers/origin/inheritance/about-blank-popup-navigated-by-non-opener.window.js` | 0 / 7 | harnais : délai du harnais dépassé |
| `html/browsers/origin/inheritance/about-blank-window.html` | 0 / 1 |  |
| `html/browsers/origin/inheritance/document-write.https.window.js` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/browsers/origin/inheritance/javascript-url.html` | 0 / 1 |  |
| `html/browsers/origin/origin-keyed-agent-clusters/1-iframe/parent-no-child-bad-subdomain.sub.https.html` | 0 / 21 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/1-iframe/parent-no-child-yes-cross-site.sub.https.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/1-iframe/parent-no-child-yes-port.sub.https.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/1-iframe/parent-no-child-yes-same.sub.https.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/1-iframe/parent-no-child-yes-subdomain-with-redirect.sub.https.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/1-iframe/parent-no-child-yes-subdomain.sub.https.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/1-iframe/parent-no-child-yeswithparams-subdomain.sub.https.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/1-iframe/parent-yes-child-no-port.sub.https.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/1-iframe/parent-yes-child-no-same.sub.https.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/1-iframe/parent-yes-child-no-subdomain.sub.https.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/1-iframe/parent-yes-child-yes-port.sub.https.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/1-iframe/parent-yes-child-yes-same.sub.https.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/1-iframe/parent-yes-child-yes-subdomain.sub.https.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/2-iframes/parent-no-child1-no-subdomain-child2-yes-subdomain.sub.https.html` | 0 / 7 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/2-iframes/parent-no-child1-no-subdomain-child2-yes-subdomainport.sub.https.html` | 0 / 10 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/2-iframes/parent-no-child1-no-subdomain1-child2-yes-subdomain2.sub.https.html` | 0 / 10 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/2-iframes/parent-no-child1-yes-subdomain-child2-no-port.sub.https.html` | 0 / 10 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/2-iframes/parent-no-child1-yes-subdomain-child2-no-subdomain.sub.https.html` | 0 / 9 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/2-iframes/parent-yes-child1-no-subdomain-child2-no-subdomain.sub.https.html` | 0 / 9 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/2-iframes/parent-yes-child1-no-subdomain-child2-no-subdomain2.sub.https.html` | 0 / 9 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/2-iframes/parent-yes-child1-no-subdomain-child2-yes-subdomain.sub.https.html` | 0 / 9 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/2-iframes/parent-yes-child1-no-subdomain-child2-yes-subdomain2.sub.https.html` | 0 / 11 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/2-iframes/parent-yes-child1-no-subdomain-child2-yes-subdomainport.sub.https.html` | 0 / 11 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/2-iframes/parent-yes-child1-yes-subdomain-child2-no-port.sub.https.html` | 0 / 11 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/2-iframes/parent-yes-child1-yes-subdomain-child2-no-subdomain.sub.https.html` | 0 / 9 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/2-iframes/parent-yes-child1-yes-subdomain-child2-yes-subdomain.sub.https.html` | 0 / 9 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/2-iframes/parent-yes-child1-yes-subdomain-child2-yes-subdomain2.sub.https.html` | 0 / 11 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/2-iframes/parent-yes-child1-yes-subdomain-child2-yes-subdomainport.sub.https.html` | 0 / 11 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/about-blank-popup-insecure-opener.tentative.sub.html` | 0 / 1 | harnais : TypeError: impossible de lire la propriété « document » de null (valeur de popup) |
| `html/browsers/origin/origin-keyed-agent-clusters/about-blank-popup.https.sub.html` | 0 / 1 | harnais : TypeError: impossible de lire la propriété « document » de null (valeur de popup) |
| `html/browsers/origin/origin-keyed-agent-clusters/about-blank.https.sub.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/data-frame-child.https.sub.html` | 0 / 1 |  |
| `html/browsers/origin/origin-keyed-agent-clusters/document-domain.sub.https.html` | 1 / 3 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/getter-special-cases/cross-origin-isolated.sub.https.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/getter-special-cases/csp-sandbox-no.https.html` | 0 / 1 |  |
| `html/browsers/origin/origin-keyed-agent-clusters/getter-special-cases/csp-sandbox-yes.https.html` | 0 / 1 |  |
| `html/browsers/origin/origin-keyed-agent-clusters/getter-special-cases/data-to-javascript-no.https.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/getter-special-cases/data-to-javascript-yes.https.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/getter-special-cases/data-url-no.https.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/getter-special-cases/data-url-yes.https.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/getter-special-cases/javascript-url-no.https.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/getter-special-cases/javascript-url-yes.https.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/getter-special-cases/removed-iframe.sub.https.html` | 0 / 1 |  |
| `html/browsers/origin/origin-keyed-agent-clusters/getter-special-cases/sandboxed-iframe-no.https.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/getter-special-cases/sandboxed-iframe-yes.https.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/getter-special-cases/sandboxed-same-origin-iframe-no.https.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/getter-special-cases/sandboxed-same-origin-iframe-yes.https.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/getter-special-cases/two-sandboxed-iframes.https.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/going-back.sub.https.html` | 0 / 13 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/iframe-navigation/parent-no-1-no-same-2-yes-port.sub.https.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/iframe-navigation/parent-no-1-no-same-2-yes-subdomain.sub.https.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/iframe-navigation/parent-no-1-no-subdomain-2-yes-subdomain.sub.https.html` | 0 / 7 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/iframe-navigation/parent-no-1-no-subdomain-2-yes-subdomain2.sub.https.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/iframe-navigation/parent-no-1-subdomain-yes-2-subdomain2-no.sub.https.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/iframe-navigation/parent-no-1-yes-subdomain-2-no-subdomain.sub.https.html` | 0 / 9 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/iframe-navigation/parent-yes-1-no-same-2-no-port.sub.https.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/iframe-navigation/parent-yes-1-no-same-2-no-subdomain.sub.https.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/insecure-http-opener-https-openee.sub.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/insecure-http-parent-https-children.sub.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/insecure-http.sub.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/no-content-response.sub.https.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/popups/noopener-opener-no-openee-yes-same.https.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/popups/opener-no-openee-yes-port.sub.https.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/popups/opener-no-openee-yes-same.sub.https.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/popups/opener-no-openee-yes-subdomain.sub.https.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/popups/opener-yes-openee-no-port.sub.https.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/popups/opener-yes-openee-no-same.sub.https.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/popups/opener-yes-openee-no-subdomain.sub.https.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/popups/opener-yes-openee-yes-port.sub.https.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/popups/opener-yes-openee-yes-same.sub.https.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/popups/opener-yes-openee-yes-subdomain.sub.https.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-keyed-agent-clusters/regression-1399759.https.sub.html` | 0 / 2 |  |
| `html/browsers/origin/origin-keyed-agent-clusters/removing-iframes.sub.https.html` | 0 / 13 | harnais : délai du harnais dépassé |
| `html/browsers/origin/origin-of-data-document.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/origin/relaxing-the-same-origin-restriction/document_domain_access_details.sub.html` | 0 / 12 | harnais : délai du harnais dépassé |
| `html/browsers/origin/relaxing-the-same-origin-restriction/document_domain_setter.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/browsers/origin/relaxing-the-same-origin-restriction/document_domain_setter_srcdoc.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/browsers/origin/relaxing-the-same-origin-restriction/sandboxed-document_domain.html` | 0 / 5 |  |
| `html/browsers/sandboxing/popup-from-initial-empty-sandboxed-document.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/sandboxing/sandbox-allow-same-origin.html` | 0 / 2 | harnais : Uncaught { message: 'assert_equals: expected 0 but got 1', stack: 'Error' } |
| `html/browsers/sandboxing/sandbox-disallow-forms.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/browsers/sandboxing/sandbox-disallow-popups.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/sandboxing/sandbox-disallow-same-origin.html` | 0 / 1 |  |
| `html/browsers/sandboxing/sandbox-disallow-scripts-via-unsandboxed-popup.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/sandboxing/sandbox-disallow-scripts.html` | 0 / 2 | harnais : Uncaught { message: 'assert_equals: expected 0 but got 1', stack: 'Error' } |
| `html/browsers/sandboxing/sandbox-document-open-mutation.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/sandboxing/sandbox-document-open.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/browsers/sandboxing/sandbox-inherited-from-initiator-frame.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/sandboxing/sandbox-inherited-from-initiator-response.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/sandboxing/sandbox-inherited-from-required-csp.html` | 0 / 7 | harnais : délai du harnais dépassé |
| `html/browsers/sandboxing/sandbox-javascript-window-open.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/sandboxing/sandbox-new-execution-context.html` | 0 / 1 |  |
| `html/browsers/sandboxing/sandbox-window-open-srcdoc.html` | 1 / 2 |  |
| `html/browsers/sandboxing/window-open-blank-from-different-initiator.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/BarProp.window.js` | 0 / 2 |  |
| `html/browsers/the-window-object/Window-document.html` | 0 / 1 |  |
| `html/browsers/the-window-object/accessing-other-browsing-contexts/indexed-browsing-contexts-01.html` | 1 / 3 |  |
| `html/browsers/the-window-object/accessing-other-browsing-contexts/indexed-browsing-contexts-02.html` | 0 / 3 |  |
| `html/browsers/the-window-object/accessing-other-browsing-contexts/indexed-browsing-contexts-03.html` | 0 / 1 |  |
| `html/browsers/the-window-object/accessing-other-browsing-contexts/window_length.html` | 5 / 7 |  |
| `html/browsers/the-window-object/close-method.window.js` | 0 / 2 |  |
| `html/browsers/the-window-object/closed-attribute.window.js` | 0 / 6 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/garbage-collection-and-browsing-contexts/discard_iframe_history_1.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/garbage-collection-and-browsing-contexts/discard_iframe_history_2.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/garbage-collection-and-browsing-contexts/discard_iframe_history_3.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/garbage-collection-and-browsing-contexts/discard_iframe_history_4.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/length-attribute.window.js` | 0 / 1 |  |
| `html/browsers/the-window-object/name-attribute.window.js` | 0 / 1 |  |
| `html/browsers/the-window-object/named-access-on-the-window-object/basics.html` | 5 / 6 |  |
| `html/browsers/the-window-object/named-access-on-the-window-object/cross-global-npo.html` | 0 / 1 |  |
| `html/browsers/the-window-object/named-access-on-the-window-object/cross-origin-named-access.sub.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/named-access-on-the-window-object/id-non-html-elements.html` | 29 / 30 |  |
| `html/browsers/the-window-object/named-access-on-the-window-object/live-collection.html` | 0 / 2 |  |
| `html/browsers/the-window-object/named-access-on-the-window-object/multi-match.html` | 0 / 1 |  |
| `html/browsers/the-window-object/named-access-on-the-window-object/name-attribute-elements.html` | 2 / 3 |  |
| `html/browsers/the-window-object/named-access-on-the-window-object/named-objects.html` | 3 / 6 |  |
| `html/browsers/the-window-object/named-access-on-the-window-object/navigated-named-objects.window.js` | 0 / 12 |  |
| `html/browsers/the-window-object/named-access-on-the-window-object/nested-context.html` | 1 / 2 |  |
| `html/browsers/the-window-object/named-access-on-the-window-object/prototype.html` | 1 / 4 |  |
| `html/browsers/the-window-object/named-access-on-the-window-object/window-named-properties.html` | 1 / 7 |  |
| `html/browsers/the-window-object/named-access-on-the-window-object/window-null-names.html` | 0 / 1 |  |
| `html/browsers/the-window-object/navigate-to-about-blank-while-initial-load-pending.html` | 0 / 1 | harnais : Test named 'Navigating to about:blank while window.open initial load pending.' specified 1 'cleanup' function, and 1 fai |
| `html/browsers/the-window-object/noopener-noreferrer-BarProp.window.js` | 1 / 3 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/noopener-noreferrer-sizing.window.js` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/open-close/close_beforeunload.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/open-close/close_noopener_beforeunload.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/open-close/close_pagehide.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/open-close/close_script_defer.html` | 0 / 1 |  |
| `html/browsers/the-window-object/open-close/close_unload.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/open-close/closed-after-close.html` | 0 / 1 |  |
| `html/browsers/the-window-object/open-close/creating_browsing_context_test_01.html` | 0 / 4 | harnais : Test named 'first argument: absolute url' specified 1 'cleanup' function, and 1 failed. |
| `html/browsers/the-window-object/open-close/declarative-windowfeatures.tentative.html` | 0 / 11 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/open-close/no_window_open_when_term_nesting_level_nonzero.window.js` | 0 / 4 |  |
| `html/browsers/the-window-object/open-close/open-features-is-popup-condition.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/open-close/open-features-negative-innerwidth-innerheight.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/open-close/open-features-negative-screenx-screeny.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/open-close/open-features-negative-top-left.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/open-close/open-features-negative-width-height.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/open-close/open-features-non-integer-height.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/open-close/open-features-non-integer-innerheight.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/open-close/open-features-non-integer-innerwidth.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/open-close/open-features-non-integer-left.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/open-close/open-features-non-integer-screenx.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/open-close/open-features-non-integer-screeny.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/open-close/open-features-non-integer-top.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/open-close/open-features-non-integer-width.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/open-close/open-features-split.tentative.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/open-close/open-features-tokenization-innerheight-innerwidth.html` | 0 / 14 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/open-close/open-features-tokenization-noopener.html` | 6 / 8 |  |
| `html/browsers/the-window-object/open-close/open-features-tokenization-noreferrer.html` | 6 / 8 |  |
| `html/browsers/the-window-object/open-close/open-features-tokenization-screenx-screeny.html` | 0 / 14 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/open-close/open-features-tokenization-top-left.html` | 0 / 16 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/open-close/open-features-tokenization-width-height.html` | 0 / 18 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/open-close/open_fires_resize.tentative.html` | 0 / 3 |  |
| `html/browsers/the-window-object/open-close/open_initial_size.html` | 0 / 1 |  |
| `html/browsers/the-window-object/open-close/script-closable-webdriver.html` | 0 / 2 |  |
| `html/browsers/the-window-object/open-close/script-closable.html` | 0 / 6 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/proxy-getOwnPropertyDescriptor.html` | 5 / 7 |  |
| `html/browsers/the-window-object/security-window/window-security.https.html` | 14 / 124 |  |
| `html/browsers/the-window-object/self-et-al.window.js` | 0 / 8 |  |
| `html/browsers/the-window-object/window-indexed-access-vs-named-access.html` | 4 / 10 |  |
| `html/browsers/the-window-object/window-indexed-properties-delete-no-cache.html` | 0 / 2 |  |
| `html/browsers/the-window-object/window-indexed-properties-strict.html` | 3 / 6 |  |
| `html/browsers/the-window-object/window-indexed-properties.html` | 3 / 7 |  |
| `html/browsers/the-window-object/window-open-defaults.window.js` | 0 / 1 |  |
| `html/browsers/the-window-object/window-open-invalid-url.html` | 0 / 1 |  |
| `html/browsers/the-window-object/window-open-noopener-existing-iframe.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/window-open-noopener.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/window-open-noreferrer.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/window-open-popup-behavior.html` | 0 / 51 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/window-open-windowfeatures-values.html` | 0 / 93 | harnais : délai du harnais dépassé |
| `html/browsers/the-window-object/window-opener-unconfigurable.window.js` | 0 / 1 |  |
| `html/browsers/the-window-object/window-properties.https.html` | 45 / 181 |  |
| `html/browsers/the-window-object/window-prototype-chain.html` | 2 / 5 |  |
| `html/browsers/the-window-object/window-reuse-in-nested-browsing-contexts.tentative.html` | 5 / 7 |  |
| `html/browsers/the-window-object/window-reuse.tentative.html` | 0 / 2 | harnais : Test named 'Initially navigating window to non-about:blank page should reuse synchonously available window' specified 1  |
| `html/browsers/the-windowproxy-exotic-object/document-tree-child-browsing-context-name-property-set.sub.html` | 0 / 1 |  |
| `html/browsers/the-windowproxy-exotic-object/windowproxy-define-own-property-unforgeable-same-origin.html` | 0 / 8 |  |
| `html/browsers/the-windowproxy-exotic-object/windowproxy-prevent-extensions.html` | 0 / 2 |  |
| `html/browsers/the-windowproxy-exotic-object/windowproxy-prototype-setting-cross-origin-domain.sub.html` | 2 / 8 |  |
| `html/browsers/the-windowproxy-exotic-object/windowproxy-prototype-setting-cross-origin.sub.html` | 2 / 8 |  |
| `html/browsers/the-windowproxy-exotic-object/windowproxy-prototype-setting-goes-cross-origin-domain.sub.html` | 3 / 12 |  |
| `html/browsers/the-windowproxy-exotic-object/windowproxy-prototype-setting-same-origin-domain.sub.html` | 4 / 8 |  |
| `html/browsers/the-windowproxy-exotic-object/windowproxy-prototype-setting-same-origin.html` | 4 / 8 |  |
| `html/browsers/windows/auxiliary-browsing-contexts/named-lookup-noopener.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/browsers/windows/auxiliary-browsing-contexts/named-lookup-scoped-to-browsing-context-group.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/windows/auxiliary-browsing-contexts/no-proactive-swap-when-other-contexts-in-group.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/windows/auxiliary-browsing-contexts/opener-closed.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/windows/auxiliary-browsing-contexts/opener-multiple.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/windows/auxiliary-browsing-contexts/opener-noopener.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/windows/auxiliary-browsing-contexts/opener-noreferrer.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/windows/auxiliary-browsing-contexts/opener-setter.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/browsers/windows/auxiliary-browsing-contexts/opener-setter.window.js` | 0 / 7 |  |
| `html/browsers/windows/auxiliary-browsing-contexts/opener.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/browsers/windows/browsing-context-names/choose-_blank-001.html` | 0 / 2 | harnais : Test named 'window.open into `_blank` should create a new browsing context each time' specified 1 'cleanup' function, an |
| `html/browsers/windows/browsing-context-names/choose-_blank-002.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/windows/browsing-context-names/choose-_blank-003.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/windows/browsing-context-names/choose-_current-001.html` | 0 / 2 | harnais : Test named 'window.open into `_current` should create a new browsing context named `_current`' specified 1 'cleanup' fun |
| `html/browsers/windows/browsing-context-names/choose-_parent-001.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/windows/browsing-context-names/choose-_parent-002.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/windows/browsing-context-names/choose-_parent-003.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/windows/browsing-context-names/choose-_parent-004.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/windows/browsing-context-names/choose-_self-001.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/windows/browsing-context-names/choose-_self-002.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/windows/browsing-context-names/choose-_top-001.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/windows/browsing-context-names/choose-_top-002.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/windows/browsing-context-names/choose-_top-003.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/windows/browsing-context-names/choose-default-001.html` | 0 / 2 |  |
| `html/browsers/windows/browsing-context-names/choose-default-002.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/windows/browsing-context-names/choose-existing-001.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/windows/browsing-context-names/choose-existing-unpaired-surrogate.html` | 0 / 1 |  |
| `html/browsers/windows/browsing-context-names/duplicate-name-order.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/windows/browsing-context-window.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/windows/browsing-context.html` | 1 / 3 |  |
| `html/browsers/windows/clear-window-name.https.html` | 0 / 12 |  |
| `html/browsers/windows/consume-user-activation/window-open-multi-global.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/windows/consume-user-activation/window-open.html` | 0 / 2 | harnais : Test named 'Opening a new window should consume user activation' specified 1 'cleanup' function, and 1 failed. |
| `html/browsers/windows/dangling-markup-window-name.html` | 0 / 6 |  |
| `html/browsers/windows/document-domain-nested-navigate.window.js` | 0 / 1 |  |
| `html/browsers/windows/document-domain-nested-set.window.js` | 0 / 1 |  |
| `html/browsers/windows/document-domain-nested.window.js` | 0 / 1 |  |
| `html/browsers/windows/embedded-opener-a-form.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/windows/embedded-opener-remove-frame.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/browsers/windows/embedded-opener.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/windows/nested-browsing-contexts/frameElement-siblings.sub.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/browsers/windows/nested-browsing-contexts/frameElement.sub.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/browsers/windows/nested-browsing-contexts/iframe-referrer.html` | 0 / 2 | harnais : Uncaught { message: 'assert_equals: expected "https://web-platform.test/html/browsers/windows/nested-browsing-contexts/i |
| `html/browsers/windows/nested-browsing-contexts/name-attribute.window.js` | 0 / 24 |  |
| `html/browsers/windows/nested-browsing-contexts/window-parent-null.html` | 0 / 2 |  |
| `html/browsers/windows/nested-browsing-contexts/window-top-null.html` | 0 / 2 |  |
| `html/browsers/windows/nested-browsing-contexts/window-top.html` | 1 / 3 | harnais : délai du harnais dépassé |
| `html/browsers/windows/noreferrer-null-opener.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/windows/noreferrer-window-name.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/browsers/windows/opener-string.window.js` | 0 / 1 | harnais : Test named 'Setting popup.opener to a string' specified 1 'cleanup' function, and 1 failed. |
| `html/browsers/windows/targeting-cross-origin-nested-browsing-contexts.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/browsers/windows/targeting-with-embedded-null-in-target.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/canvas/color.window.js` | 8 / 15 |  |
| `html/canvas/element/2d.conformance.requirements.missingargs.html` | 0 / 1 |  |
| `html/canvas/element/2d.text-root-font-relative-units.html` | 0 / 11 |  |
| `html/canvas/element/canvas-context/2d.canvas.context.invalid.args.html` | 0 / 1 |  |
| `html/canvas/element/canvas-host/2d.canvas.host.initial.reset.pattern.html` | 0 / 1 |  |
| `html/canvas/element/canvas-host/2d.canvas.host.size.attributes.parse.decimal.html` | 0 / 1 |  |
| `html/canvas/element/canvas-host/2d.canvas.host.size.attributes.parse.em.html` | 0 / 1 |  |
| `html/canvas/element/canvas-host/2d.canvas.host.size.attributes.parse.exp.html` | 0 / 1 |  |
| `html/canvas/element/canvas-host/2d.canvas.host.size.attributes.parse.hex.html` | 0 / 1 |  |
| `html/canvas/element/canvas-host/2d.canvas.host.size.attributes.parse.minus.html` | 0 / 1 |  |
| `html/canvas/element/canvas-host/2d.canvas.host.size.attributes.parse.percent.html` | 0 / 1 |  |
| `html/canvas/element/canvas-host/2d.canvas.host.size.attributes.parse.trailingjunk.html` | 0 / 1 |  |
| `html/canvas/element/canvas-host/2d.canvas.host.size.attributes.setAttribute.decimal.html` | 0 / 1 |  |
| `html/canvas/element/canvas-host/2d.canvas.host.size.attributes.setAttribute.em.html` | 0 / 1 |  |
| `html/canvas/element/canvas-host/2d.canvas.host.size.attributes.setAttribute.exp.html` | 0 / 1 |  |
| `html/canvas/element/canvas-host/2d.canvas.host.size.attributes.setAttribute.hex.html` | 0 / 1 |  |
| `html/canvas/element/canvas-host/2d.canvas.host.size.attributes.setAttribute.minus.html` | 0 / 1 |  |
| `html/canvas/element/canvas-host/2d.canvas.host.size.attributes.setAttribute.percent.html` | 0 / 1 |  |
| `html/canvas/element/canvas-host/2d.canvas.host.size.attributes.setAttribute.trailingjunk.html` | 0 / 1 |  |
| `html/canvas/element/canvas-host/2d.canvas.host.size.invalid.attributes.idl.html` | 0 / 1 |  |
| `html/canvas/element/color-type/2d.color.type.u8p3.to.u8srgb.to.u8p3.html` | 0 / 1 |  |
| `html/canvas/element/color-type/2d.color.type.u8srgb.to.u8p3.to.u8srgb.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.canvas.clear.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.canvas.copy.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.canvas.destination-atop.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.canvas.destination-in.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.canvas.destination-out.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.canvas.destination-over.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.canvas.lighter.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.canvas.source-atop.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.canvas.source-in.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.canvas.source-out.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.canvas.source-over.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.canvas.xor.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.globalAlpha.image.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.globalAlpha.imagepattern.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.image.clear.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.image.copy.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.image.destination-atop.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.image.destination-in.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.image.destination-out.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.image.destination-over.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.image.lighter.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.image.source-atop.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.image.source-in.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.image.source-out.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.image.source-over.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.image.xor.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.operation.clear.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.solid.clear.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.transparent.clear.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.uncovered.image.copy.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.uncovered.image.destination-atop.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.uncovered.image.destination-in.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.uncovered.image.source-in.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.uncovered.image.source-out.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.uncovered.pattern.copy.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.uncovered.pattern.destination-atop.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.uncovered.pattern.destination-in.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.uncovered.pattern.source-in.html` | 0 / 1 |  |
| `html/canvas/element/compositing/2d.composite.uncovered.pattern.source-out.html` | 0 / 1 |  |
| `html/canvas/element/conformance-requirements/2d.conformance.requirements.missingargs.html` | 0 / 1 |  |
| `html/canvas/element/drawing-images-to-the-canvas/2d.drawImage.3arg.html` | 0 / 1 |  |
| `html/canvas/element/drawing-images-to-the-canvas/2d.drawImage.5arg.html` | 0 / 1 |  |
| `html/canvas/element/drawing-images-to-the-canvas/2d.drawImage.9arg.basic.html` | 0 / 1 |  |
| `html/canvas/element/drawing-images-to-the-canvas/2d.drawImage.9arg.destpos.html` | 0 / 1 |  |
| `html/canvas/element/drawing-images-to-the-canvas/2d.drawImage.9arg.destsize.html` | 0 / 1 |  |
| `html/canvas/element/drawing-images-to-the-canvas/2d.drawImage.9arg.sourcepos.html` | 0 / 1 |  |
| `html/canvas/element/drawing-images-to-the-canvas/2d.drawImage.9arg.sourcesize.html` | 0 / 1 |  |
| `html/canvas/element/drawing-images-to-the-canvas/2d.drawImage.alpha.html` | 0 / 1 |  |
| `html/canvas/element/drawing-images-to-the-canvas/2d.drawImage.animated.gif.html` | 0 / 1 |  |
| `html/canvas/element/drawing-images-to-the-canvas/2d.drawImage.clip.html` | 0 / 1 |  |
| `html/canvas/element/drawing-images-to-the-canvas/2d.drawImage.composite.html` | 0 / 1 |  |
| `html/canvas/element/drawing-images-to-the-canvas/2d.drawImage.detachedcanvas.html` | 0 / 1 |  |
| `html/canvas/element/drawing-images-to-the-canvas/2d.drawImage.floatsource.html` | 0 / 1 |  |
| `html/canvas/element/drawing-images-to-the-canvas/2d.drawImage.negativedest.html` | 0 / 1 |  |
| `html/canvas/element/drawing-images-to-the-canvas/2d.drawImage.negativedir.html` | 0 / 1 |  |
| `html/canvas/element/drawing-images-to-the-canvas/2d.drawImage.negativesource.html` | 0 / 1 |  |
| `html/canvas/element/drawing-images-to-the-canvas/2d.drawImage.nonexistent.html` | 0 / 1 |  |
| `html/canvas/element/drawing-images-to-the-canvas/2d.drawImage.nonfinite.html` | 0 / 1 |  |
| `html/canvas/element/drawing-images-to-the-canvas/2d.drawImage.nowrap.html` | 0 / 1 |  |
| `html/canvas/element/drawing-images-to-the-canvas/2d.drawImage.path.html` | 0 / 1 |  |
| `html/canvas/element/drawing-images-to-the-canvas/2d.drawImage.svg.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/canvas/element/drawing-images-to-the-canvas/2d.drawImage.transform.html` | 0 / 1 |  |
| `html/canvas/element/drawing-images-to-the-canvas/2d.drawImage.zerosource.html` | 0 / 1 |  |
| `html/canvas/element/drawing-images-to-the-canvas/2d.drawImage.zerosource.image.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/canvas/element/drawing-rectangles-to-the-canvas/2d.fillRect.shadow.html` | 0 / 1 |  |
| `html/canvas/element/drawing-rectangles-to-the-canvas/2d.strokeRect.shadow.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.CSSHSL.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.CSSRGB.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.colorObject.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.colorObject.transparency.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.colormix.currentcolor.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.colormix.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.parse.current.basic.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.parse.current.changed.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.parse.invalid.css-color-4-hsl-1.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.parse.invalid.css-color-4-hsl-2.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.parse.invalid.css-color-4-hsl-3.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.parse.invalid.css-color-4-hsl-4.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.parse.invalid.css-color-4-hsl-5.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.parse.invalid.css-color-4-hsla-1.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.parse.invalid.css-color-4-hsla-2.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.parse.invalid.css-color-4-hsla-3.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.parse.invalid.css-color-4-rgb-1.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.parse.invalid.css-color-4-rgb-2.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.parse.invalid.css-color-4-rgb-3.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.parse.invalid.css-color-4-rgb-4.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.parse.invalid.css-color-4-rgb-5.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.parse.invalid.css-color-4-rgba-1.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.parse.invalid.css-color-4-rgba-2.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.parse.invalid.css-color-4-rgba-3.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.parse.invalid.hsl-3.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.parse.invalid.hsl-4.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.parse.invalid.hsl-6.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.parse.invalid.hsla-2.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.parse.invalid.hsla-3.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.parse.invalid.rgb-1.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.parse.invalid.rgb-2.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.parse.invalid.rgba-1.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.parse.invalid.rgba-4.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.parse.invalid.rgba-5.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.parse.system.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.relativecolor.currentcolor.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.relativecolor.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.fillStyle.toStringFunctionCallback.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.gradient.colormix.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.gradient.conic.invalid.inputs.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.gradient.conic.negative.rotation.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.gradient.conic.positive.rotation.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.gradient.interpolate.zerosize.fill.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.gradient.interpolate.zerosize.fillRect.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.gradient.interpolate.zerosize.stroke.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.gradient.interpolate.zerosize.strokeRect.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.gradient.object.invalidoffset.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.gradient.radial.cone.behind.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.gradient.radial.cone.beside.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.gradient.radial.cone.bottom.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.gradient.radial.cone.cylinder.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.gradient.radial.cone.shape2.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.gradient.radial.equal.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.gradient.radial.inside2.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.gradient.radial.inside3.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.gradient.radial.outside2.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.gradient.radial.outside3.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.gradient.radial.touch1.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.gradient.radial.touch2.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.gradient.radial.touch3.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.gradient.relativecolor.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.pattern.animated.gif.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.pattern.basic.image.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.pattern.basic.type.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.pattern.crosscanvas.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.pattern.image.broken.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.pattern.image.nonexistent-but-loading.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.pattern.image.nonexistent.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.pattern.image.nosrc.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.pattern.modify.canvas1.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.pattern.modify.canvas2.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.pattern.modify.image1.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/canvas/element/fill-and-stroke-styles/2d.pattern.modify.image2.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/canvas/element/fill-and-stroke-styles/2d.pattern.paint.norepeat.basic.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.pattern.paint.norepeat.coord1.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.pattern.paint.norepeat.coord2.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.pattern.paint.orientation.image.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.pattern.paint.repeat.basic.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.pattern.paint.repeat.coord1.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.pattern.paint.repeat.coord2.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.pattern.paint.repeat.coord3.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.pattern.paint.repeat.outside.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.pattern.paint.repeatx.basic.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.pattern.paint.repeaty.basic.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.pattern.repeat.empty.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.pattern.svgimage.nonexistent.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.pattern.svgimage.zeroheight.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.pattern.svgimage.zerowidth.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.pattern.transform.identity.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.pattern.transform.invalid.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.strokeStyle.colorObject.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.strokeStyle.colorObject.transparency.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.strokeStyle.colormix.html` | 0 / 1 |  |
| `html/canvas/element/fill-and-stroke-styles/2d.strokeStyle.relativecolor.html` | 0 / 1 |  |
| `html/canvas/element/filters/2d.filter.canvasFilterObject.blur.exceptions.tentative.html` | 0 / 1 |  |
| `html/canvas/element/filters/2d.filter.canvasFilterObject.colorMatrix.tentative.html` | 0 / 1 |  |
| `html/canvas/element/filters/2d.filter.canvasFilterObject.convolveMatrix.exceptions.tentative.html` | 0 / 1 |  |
| `html/canvas/element/filters/2d.filter.canvasFilterObject.dropShadow.exceptions.tentative.html` | 0 / 1 |  |
| `html/canvas/element/filters/2d.filter.canvasFilterObject.tentative.html` | 0 / 1 |  |
| `html/canvas/element/filters/2d.filter.canvasFilterObject.turbulence.inputTypes.tentative.html` | 0 / 1 |  |
| `html/canvas/element/filters/2d.filter.layers.colorMatrix.tentative.html` | 0 / 1 |  |
| `html/canvas/element/filters/2d.filter.layers.convolveMatrix.exceptions.tentative.html` | 0 / 1 |  |
| `html/canvas/element/filters/2d.filter.layers.dropShadow.exceptions.tentative.html` | 0 / 1 |  |
| `html/canvas/element/filters/2d.filter.layers.turbulence.inputTypes.tentative.html` | 0 / 1 |  |
| `html/canvas/element/filters/2d.filter.value.html` | 0 / 1 |  |
| `html/canvas/element/layers/2d.layer.beginLayer-options.tentative.html` | 0 / 1 |  |
| `html/canvas/element/layers/2d.layer.ctm.getTransform.html` | 0 / 1 |  |
| `html/canvas/element/layers/2d.layer.exceptions-are-no-op.tentative.html` | 0 / 1 |  |
| `html/canvas/element/layers/2d.layer.invalid-calls.beginLayer-reset-endLayer.html` | 0 / 1 |  |
| `html/canvas/element/layers/2d.layer.invalid-calls.beginLayer-restore.html` | 0 / 1 |  |
| `html/canvas/element/layers/2d.layer.invalid-calls.beginLayer-save-endLayer.html` | 0 / 1 |  |
| `html/canvas/element/layers/2d.layer.invalid-calls.endLayer.html` | 0 / 1 |  |
| `html/canvas/element/layers/2d.layer.invalid-calls.save-beginLayer-restore.html` | 0 / 1 |  |
| `html/canvas/element/layers/2d.layer.invalid-calls.save-endLayer.html` | 0 / 1 |  |
| `html/canvas/element/layers/2d.layer.layer-rendering-state-reset-in-layer.html` | 0 / 1 |  |
| `html/canvas/element/layers/2d.layer.malformed-operations-with-promises.html` | 0 / 2 |  |
| `html/canvas/element/layers/2d.layer.malformed-operations.html` | 0 / 5 |  |
| `html/canvas/element/layers/2d.layer.valid-calls.beginLayer-endLayer.html` | 0 / 1 |  |
| `html/canvas/element/layers/2d.layer.valid-calls.beginLayer-save.html` | 0 / 1 |  |
| `html/canvas/element/layers/2d.layer.valid-calls.beginLayer.html` | 0 / 1 |  |
| `html/canvas/element/layers/2d.layer.valid-calls.save-beginLayer.html` | 0 / 1 |  |
| `html/canvas/element/line-styles/2d.line.join.round.html` | 0 / 1 |  |
| `html/canvas/element/manual/context-attributes/getContextAttributes.html` | 4 / 16 |  |
| `html/canvas/element/manual/draw-element-image/animated-gif-pinned-to-element-image.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/animated-gif.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/backdrop-filter-bounds-expansion.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/canvas-transform-get-bounding-client-rect.tentative.html` | 0 / 7 |  |
| `html/canvas/element/manual/draw-element-image/canvas-transform-get-set.tentative.html` | 1 / 13 |  |
| `html/canvas/element/manual/draw-element-image/canvas-transform-update-element-image.tentative.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/draw-element-image/canvas-transform-update.tentative.html` | 0 / 13 | harnais : Test named 'drawElementImage() automatically updates element canvas transform by default' specified 1 'cleanup' function |
| `html/canvas/element/manual/draw-element-image/captureElementImage.tentative.html` | 0 / 6 | harnais : Test named 'getElementImageDefaultSize should return size in canvas pixel grid coordinate space' specified 1 'cleanup' f |
| `html/canvas/element/manual/draw-element-image/containment.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/content-drawable-no-hit-test.tentative.html` | 0 / 3 |  |
| `html/canvas/element/manual/draw-element-image/css-backdrop-filter-with-mix-blend-mode-and-sibling.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/display-contents.tentative.html` | 0 / 2 |  |
| `html/canvas/element/manual/draw-element-image/draw-element-different-canvas.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/draw-element-image-detached.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/draw-element-image-different-canvas.tentative.html` | 0 / 2 | harnais : Test named 'Prevent drawing ElementImage into a different canvas on main thread' specified 1 'cleanup' function, and 1 f |
| `html/canvas/element/manual/draw-element-image/draw-element-image-display-none.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/draw-element-image-empty.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/draw-element-image-get-image-data.tentative.html` | 0 / 2 |  |
| `html/canvas/element/manual/draw-element-image/drawElementImage-dest-rect-non-finite.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/drawElementImage-dest-rect.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/drawElementImage-src-rect-non-finite.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/drawElementImage-src-rect.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/drawElementImage-zero-size.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/drawing-display-none-fails.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/element-image-fractional-size.tentative.html` | 0 / 2 | harnais : Test named 'getElementImageDefaultSize width and height should support fractional values in canvas pixel grid coordinate |
| `html/canvas/element/manual/draw-element-image/error-conditions.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/hit-test/elementsFromPoint-no-content-drawable.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/hit-test/elementsFromPoint-object-fit-contain-tall.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/hit-test/elementsFromPoint-object-fit-contain-wide.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/hit-test/elementsFromPoint-object-fit-cover-tall.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/hit-test/elementsFromPoint-object-fit-cover-wide.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/hit-test/elementsFromPoint-object-fit-fill-tall.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/hit-test/elementsFromPoint-object-fit-fill-wide.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/hit-test/elementsFromPoint-object-fit-none-tall.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/hit-test/elementsFromPoint-object-fit-none-wide.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/hit-test/elementsFromPoint-update-element-geometry-offscreen.tentative.html` | 0 / 10 |  |
| `html/canvas/element/manual/draw-element-image/hit-test/elementsFromPoint-update-element-geometry.tentative.html` | 0 / 21 |  |
| `html/canvas/element/manual/draw-element-image/hit-test/elementsFromPoint.tentative.html` | 0 / 7 |  |
| `html/canvas/element/manual/draw-element-image/hit-test/low-dpi-hit-test.tentative.html` | 0 / 3 |  |
| `html/canvas/element/manual/draw-element-image/hit-test/selection.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/hit-test/z-index.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/intersection-observer-visibility.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/layout-canvas-children.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/moved-nested-content-drawable-canvas.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/moved-webgl-canvas.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/nested-content-drawable-canvas.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/nested-div-content-drawable-canvas.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/onpaint-after-scroll-2.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/onpaint-after-scroll.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/onpaint-changedElements.tentative.html` | 0 / 2 |  |
| `html/canvas/element/manual/draw-element-image/onpaint-css-animation.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/onpaint-display-none.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/onpaint-fires-post-intersection-observation-steps.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/onpaint-fires-post-resize-observer.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/onpaint-iframe-nested-invalidation.tentative.html` | 0 / 8 | harnais : Test named 'calling requestPaint on d1 outside of paint should cause canvas a, b, and c to fire paint' specified 1 'clea |
| `html/canvas/element/manual/draw-element-image/onpaint-iframe-post-tree-order.tentative.html` | 0 / 2 |  |
| `html/canvas/element/manual/draw-element-image/onpaint-iframe.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/onpaint-post-tree-order-shadow-dom.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/onpaint-post-tree-order.tentative.html` | 0 / 5 |  |
| `html/canvas/element/manual/draw-element-image/onpaint-remove-drawable-element.tentative.html` | 0 / 7 |  |
| `html/canvas/element/manual/draw-element-image/onpaint-remove-non-drawable-element.tentative.html` | 0 / 4 |  |
| `html/canvas/element/manual/draw-element-image/onpaint.tentative.html` | 4 / 12 | harnais : Test named 'onpaint should not fire if a canvas child transform changes' specified 1 'cleanup' function, and 1 failed. |
| `html/canvas/element/manual/draw-element-image/privacy/backdrop-filter-png-images-ignored.tentative.https.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/draw-element-image/privacy/background-image-in-iframe-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/background-image-in-iframe-moved-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/background-image-on-body-in-iframe-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/background-image-on-body-in-iframe-moved-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/background-image-on-html-in-iframe-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/background-image-on-html-in-iframe-moved-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/background-png-images-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/background-svg-images-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/border-png-images-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/border-svg-images-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/box-mask-png-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/box-mask-svg-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/box-reflect-images-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/box-reflect-images-in-iframe-moved-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/box-reflect-images-in-iframe-multicol-moved-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/canvas-in-custom-element-images-ignored.tentative.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/draw-element-image/privacy/embed-image-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/filtered-div-drawable-added-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/filtered-div-multi-filters-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/filtered-div-ref-external-svg-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/filtered-div-ref-svg-feimage-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/filtered-div-ref-svg-feimage-svg-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/filtered-div-ref-svg-mixed-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/filtered-div-removed-contain-paint.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/filtered-outside-subtree-ignored.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/filtered-svg-ref-svg-feimage-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/filtered-svg-ref-svg-feimage-svg-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/fragment-mask-png-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/fragment-mask-svg-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/html-images-cors-allowed.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/html-images-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/iframe-sandbox-ignored.tentative.https.sub.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/draw-element-image/privacy/iframes-ignored.tentative.https.sub.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/draw-element-image/privacy/nested-tainted-canvas-ignored.tentative.https.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/draw-element-image/privacy/nested-tainted-html-in-canvas-ignored.tentative.https.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/draw-element-image/privacy/object-image-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/picture-images-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/popover-show-hide-background-image-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/pseudos/before-after-images-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/pseudos/before-after-images-moved-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/pseudos/before-after-visited-color-ignored.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/pseudos/before-background-image-iframe-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/pseudos/before-background-image-iframe-modified-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/pseudos/before-background-image-iframe-moved-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/pseudos/before-images-iframe-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/pseudos/before-images-iframe-moved-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/pseudos/details-content-after-image-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/pseudos/details-content-background-image-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/pseudos/details-content-visited-link-currentcolor-ignored.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/pseudos/details-content-visited-link-ignored.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/pseudos/first-letter-background-images-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/pseudos/first-letter-visited-link-color-ignored.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/pseudos/first-letter-visited-link-currentcolor-ignored.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/pseudos/first-line-background-images-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/pseudos/first-line-background-images-moved-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/pseudos/first-line-visited-link-color-ignored.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/pseudos/first-line-visited-link-color-moved-ignored.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/pseudos/first-line-visited-link-currentcolor-ignored.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/pseudos/marker-images-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/pseudos/marker-visited-link-currentcolor-ignored.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/scrollbar-thumb-images-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/shape-image-png-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/shape-image-svg-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/subframe-cross-origin-background-image.tentative.https.sub.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/draw-element-image/privacy/subframe-cross-origin-frame.tentative.https.sub.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/draw-element-image/privacy/subframe-cross-origin-html-images.tentative.https.sub.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/draw-element-image/privacy/svg/svg-images-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/svg/svg-linear-gradient-href-outside-subtree-ignored.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/svg/svg-linear-gradient-outside-subtree-ignored.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/svg/svg-marker-outside-subtree-ignored.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/draw-element-image/privacy/svg/svg-mask-filter.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/svg/svg-mask-nested-filter.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/svg/svg-mask-on-svg-content-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/svg/svg-mask-visited.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/svg/svg-pattern-cross-origin-image.tentative.https.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/draw-element-image/privacy/svg/svg-pattern-filter-transformed-text.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/svg/svg-pattern-href-outside-subtree-ignored.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/draw-element-image/privacy/svg/svg-pattern-outside-subtree-ignored.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/draw-element-image/privacy/svg/svg-pattern-with-feImage-cross-origin-image.tentative.https.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/draw-element-image/privacy/svg/svg-pattern-with-feImage-transformed-text.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/svg/svg-radial-gradient-href-outside-subtree-ignored.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/svg/svg-radial-gradient-outside-subtree-ignored.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/svg/svg-use-chained-outside-subtree-images-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/svg/svg-use-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/svg/svg-use-outside-subtree-images-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/video-ignored.tentative.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/visited-link/image-map-area-visited-color-ignored.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/visited-link/visited-link-color-ignored.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/visited-link/visited-link-currentcolor-ignored.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/visited-link/visited-link-custom-selection-color-ignored.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/visited-link/visited-link-decoration-color-ignored.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/visited-link/visited-link-slotted-color-ignored.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/visited-link/visited-link-svg-fill-stroke-color-ignored.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/visited-link/visited-link-svg-linear-gradiant-ignored.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/visited-link/visited-link-svg-pattern-ignored.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/visited-link/visited-link-svg-radiant-gradiant-ignored.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/visited-link/visited-link-svg-use-chained-outside-subtree-ignored.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/privacy/visited-link/visited-link-svg-use-outside-subtree-ignored.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/requestPaint.tentative.html` | 0 / 3 |  |
| `html/canvas/element/manual/draw-element-image/triple-nested-content-drawable-canvas.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/draw-element-image/update-element-geometry-get-bounding-client-rect.tentative.html` | 0 / 8 | harnais : Test named 'updateElementGeometry and clearElementGeometry throw InvalidStateError for ineligible ElementImage' specifie |
| `html/canvas/element/manual/draw-element-image/video.tentative.html` | 0 / 1 |  |
| `html/canvas/element/manual/drawing-images-to-the-canvas/drawimage_canvas.html` | 150 / 151 |  |
| `html/canvas/element/manual/drawing-images-to-the-canvas/drawimage_crossorigin.sub.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/drawing-images-to-the-canvas/drawimage_html_image.html` | 71 / 127 |  |
| `html/canvas/element/manual/drawing-images-to-the-canvas/drawimage_svg_image_1.html` | 0 / 1 |  |
| `html/canvas/element/manual/drawing-images-to-the-canvas/drawimage_svg_image_with_foreign_object_does_not_taint.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/drawing-paths-to-the-canvas/drawFocusIfNeeded_001.html` | 0 / 1 |  |
| `html/canvas/element/manual/drawing-paths-to-the-canvas/drawFocusIfNeeded_002.html` | 0 / 1 |  |
| `html/canvas/element/manual/drawing-paths-to-the-canvas/drawFocusIfNeeded_003.html` | 0 / 1 |  |
| `html/canvas/element/manual/drawing-paths-to-the-canvas/drawFocusIfNeeded_004.html` | 0 / 1 |  |
| `html/canvas/element/manual/drawing-paths-to-the-canvas/drawFocusIfNeeded_005.html` | 0 / 1 |  |
| `html/canvas/element/manual/fill-and-stroke-styles/2d.fillStyle.parse.current.notrendered.html` | 0 / 1 |  |
| `html/canvas/element/manual/fill-and-stroke-styles/canvas_colorsandstyles_createlineargradient_001.htm` | 0 / 1 |  |
| `html/canvas/element/manual/fill-and-stroke-styles/pattern-from-canvas-cross-origin-taint.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/fill-and-stroke-styles/pattern-from-image-cross-origin-taint.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/fill-and-stroke-styles/pattern-from-img-cross-origin-taint.https.sub.html` | 0 / 1 |  |
| `html/canvas/element/manual/fill-and-stroke-styles/pattern-from-video-cross-origin-taint.https.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/filters/2d.filter.function-then-svg-reference.html` | 0 / 2 |  |
| `html/canvas/element/manual/filters/svg-diffuse-filter-visited-color-taints.html` | 0 / 1 |  |
| `html/canvas/element/manual/filters/svg-drop-shadow-filter-visited-color-taints.html` | 0 / 1 |  |
| `html/canvas/element/manual/filters/svg-filter-feImage-cross-origin-taint.https.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/filters/svg-filter-visited-link-color-taints.html` | 1 / 11 |  |
| `html/canvas/element/manual/filters/svg-flood-filter-visited-color-taints.html` | 0 / 1 |  |
| `html/canvas/element/manual/filters/svg-specular-filter-visited-color-taints.html` | 0 / 1 |  |
| `html/canvas/element/manual/filters/tentative/idl-conversions/canvas-filter-sequence-conversion.html` | 0 / 1 |  |
| `html/canvas/element/manual/image-smoothing/imagesmoothing.html` | 6 / 9 |  |
| `html/canvas/element/manual/imagebitmap/canvas-ImageBitmap-close.html` | 0 / 1 |  |
| `html/canvas/element/manual/imagebitmap/canvas-createImageBitmap-resize.html` | 0 / 6 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/imagebitmap/canvas-createImageBitmap-video-resize.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/imagebitmap/createImageBitmap-bounds.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/imagebitmap/createImageBitmap-colorSpaceConversion.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/imagebitmap/createImageBitmap-drawImage-closed.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/imagebitmap/createImageBitmap-drawImage.html` | 5 / 55 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/imagebitmap/createImageBitmap-exif-orientation.html` | 0 / 7 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/imagebitmap/createImageBitmap-exif-orientation_none.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/imagebitmap/createImageBitmap-flipY.html` | 2 / 22 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/imagebitmap/createImageBitmap-in-worker-transfer.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/imagebitmap/createImageBitmap-invalid-args.html` | 7 / 93 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/imagebitmap/createImageBitmap-origin.sub.html` | 0 / 27 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/imagebitmap/createImageBitmap-resolves-in-task.any.js` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/imagebitmap/createImageBitmap-serializable.html` | 0 / 12 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/imagebitmap/createImageBitmap-sizeOverflow.html` | 2 / 7 |  |
| `html/canvas/element/manual/imagebitmap/createImageBitmap-transfer.html` | 0 / 12 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/imagebitmap/imagebitmap-replication-exif-orientation.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/shadows/canvas_shadows_001.htm` | 0 / 1 |  |
| `html/canvas/element/manual/shadows/shadowBlur_gaussian_tolerance.1.html` | 0 / 3 |  |
| `html/canvas/element/manual/transformations/2d.transformation.getTransform.html` | 0 / 1 |  |
| `html/canvas/element/manual/wide-gamut-canvas/ImageData-fidelity.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/wide-gamut-canvas/canvas-color-space-settings.html` | 4 / 25 |  |
| `html/canvas/element/manual/wide-gamut-canvas/canvas-display-p3-drawImage-ImageBitmap-Blob.html` | 64 / 416 | harnais : Unhandled rejection: fetch : trop de requêtes en cours |
| `html/canvas/element/manual/wide-gamut-canvas/canvas-display-p3-drawImage-ImageBitmap-ImageBitmap.html` | 0 / 208 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/wide-gamut-canvas/canvas-display-p3-drawImage-ImageBitmap-ImageData.html` | 4 / 16 |  |
| `html/canvas/element/manual/wide-gamut-canvas/canvas-display-p3-drawImage-ImageBitmap-canvas.html` | 4 / 16 |  |
| `html/canvas/element/manual/wide-gamut-canvas/canvas-display-p3-drawImage-ImageBitmap-cloned.html` | 0 / 208 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/wide-gamut-canvas/canvas-display-p3-drawImage-ImageBitmap-image.html` | 0 / 208 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/wide-gamut-canvas/canvas-display-p3-drawImage-ImageBitmap-video.html` | 0 / 32 |  |
| `html/canvas/element/manual/wide-gamut-canvas/canvas-display-p3-drawImage-video.html` | 0 / 32 |  |
| `html/canvas/element/manual/wide-gamut-canvas/canvas-display-p3-drawImage.https.html` | 0 / 240 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/wide-gamut-canvas/canvas-display-p3-pattern-canvas.html` | 2 / 8 |  |
| `html/canvas/element/manual/wide-gamut-canvas/canvas-display-p3-pattern-image.html` | 0 / 104 | harnais : délai du harnais dépassé |
| `html/canvas/element/manual/wide-gamut-canvas/encode-hdr.html` | 0 / 18 |  |
| `html/canvas/element/path-objects/2d.path.arc.scale.1.html` | 0 / 1 |  |
| `html/canvas/element/path-objects/2d.path.isPointInPath.basic.html` | 0 / 1 |  |
| `html/canvas/element/path-objects/2d.path.isPointInPath.edge.html` | 0 / 1 |  |
| `html/canvas/element/path-objects/2d.path.isPointInPath.nobitmap.html` | 0 / 1 |  |
| `html/canvas/element/path-objects/2d.path.isPointInStroke.nobitmap.html` | 0 / 1 |  |
| `html/canvas/element/path-objects/2d.path.isPointInStroke.nobitmap.path2d.html` | 0 / 1 |  |
| `html/canvas/element/path-objects/2d.path.isPointInStroke.nobitmap.transform.html` | 0 / 1 |  |
| `html/canvas/element/path-objects/2d.path.isPointInStroke.scaleddashes.html` | 0 / 1 |  |
| `html/canvas/element/path-objects/2d.path.lineTo.nonfinite.details.html` | 0 / 1 |  |
| `html/canvas/element/path-objects/2d.path.roundrect.1.radius.dompoint.html` | 0 / 1 |  |
| `html/canvas/element/path-objects/2d.path.roundrect.1.radius.dompoint.single.argument.html` | 0 / 1 |  |
| `html/canvas/element/path-objects/2d.path.roundrect.2.radii.1.dompoint.html` | 0 / 1 |  |
| `html/canvas/element/path-objects/2d.path.roundrect.2.radii.2.dompoint.html` | 0 / 1 |  |
| `html/canvas/element/path-objects/2d.path.roundrect.3.radii.1.dompoint.html` | 0 / 1 |  |
| `html/canvas/element/path-objects/2d.path.roundrect.3.radii.2.dompoint.html` | 0 / 1 |  |
| `html/canvas/element/path-objects/2d.path.roundrect.3.radii.3.dompoint.html` | 0 / 1 |  |
| `html/canvas/element/path-objects/2d.path.roundrect.4.radii.1.dompoint.html` | 0 / 1 |  |
| `html/canvas/element/path-objects/2d.path.roundrect.4.radii.2.dompoint.html` | 0 / 1 |  |
| `html/canvas/element/path-objects/2d.path.roundrect.4.radii.3.dompoint.html` | 0 / 1 |  |
| `html/canvas/element/path-objects/2d.path.roundrect.4.radii.4.dompoint.html` | 0 / 1 |  |
| `html/canvas/element/path-objects/2d.path.roundrect.badinput.html` | 0 / 1 |  |
| `html/canvas/element/path-objects/2d.path.roundrect.nonfinite.html` | 0 / 1 |  |
| `html/canvas/element/path-objects/2d.path.roundrect.radius.negative.html` | 0 / 1 |  |
| `html/canvas/element/path-objects/2d.path.roundrect.winding.html` | 0 / 1 |  |
| `html/canvas/element/path-objects/2d.path.stroke.prune.arc.html` | 0 / 1 |  |
| `html/canvas/element/path-objects/2d.path.stroke.prune.closed.html` | 0 / 1 |  |
| `html/canvas/element/path-objects/2d.path.stroke.prune.curve.html` | 0 / 1 |  |
| `html/canvas/element/path-objects/2d.path.stroke.prune.line.html` | 0 / 1 |  |
| `html/canvas/element/path-objects/2d.path.stroke.prune.rect.html` | 0 / 1 |  |
| `html/canvas/element/pixel-manipulation/2d.imageData.create1.this.html` | 0 / 1 |  |
| `html/canvas/element/pixel-manipulation/2d.imageData.create2.this.html` | 0 / 1 |  |
| `html/canvas/element/pixel-manipulation/2d.imageData.createImageBitmap.srgb.rgba.float16.html` | 0 / 1 |  |
| `html/canvas/element/pixel-manipulation/2d.imageData.object.ctor.array.bounds.html` | 0 / 1 |  |
| `html/canvas/element/pixel-manipulation/2d.imageData.object.ctor.basics.html` | 0 / 1 |  |
| `html/canvas/element/pixel-manipulation/2d.imageData.object.ctor.pixelFormat.html` | 0 / 1 |  |
| `html/canvas/element/pixel-manipulation/2d.imageData.object.properties.html` | 0 / 1 |  |
| `html/canvas/element/pixel-manipulation/2d.imageData.put.basic.rgba.float16.html` | 0 / 1 |  |
| `html/canvas/element/pixel-manipulation/2d.imageData.put.nonfinite.html` | 0 / 1 |  |
| `html/canvas/element/reset/2d.reset.state.font_kerning.html` | 0 / 1 |  |
| `html/canvas/element/reset/2d.reset.state.font_stretch.html` | 0 / 1 |  |
| `html/canvas/element/reset/2d.reset.state.font_variant_caps.html` | 0 / 1 |  |
| `html/canvas/element/reset/2d.reset.state.letter_spacing.html` | 0 / 1 |  |
| `html/canvas/element/reset/2d.reset.state.text_rendering.html` | 0 / 1 |  |
| `html/canvas/element/reset/2d.reset.state.word_spacing.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.alpha.2.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.alpha.3.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.alpha.4.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.alpha.5.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.attributes.shadowColor.current.basic.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.attributes.shadowColor.current.changed.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.canvas.alpha.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.canvas.basic.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.canvas.transparent.2.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.clip.1.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.clip.3.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.composite.1.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.composite.2.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.enable.blur.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.enable.x.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.enable.y.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.gradient.alpha.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.gradient.basic.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.gradient.transparent.2.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.image.alpha.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.image.basic.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.image.scale.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.image.transparent.2.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.offset.negativeX.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.offset.negativeY.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.offset.positiveX.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.offset.positiveY.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.outside.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.pattern.alpha.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.pattern.basic.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.pattern.transparent.2.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.stroke.basic.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.stroke.cap.2.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.stroke.join.2.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.transform.1.html` | 0 / 1 |  |
| `html/canvas/element/shadows/2d.shadow.transform.2.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.draw.align.center.complex-shaping.html` | 0 / 2 |  |
| `html/canvas/element/text/2d.text.draw.align.center.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.draw.align.end.ltr.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.draw.align.end.rtl.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.draw.align.left.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.draw.align.right.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.draw.align.start.ltr.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.draw.align.start.rtl.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.draw.baseline.alphabetic.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.draw.baseline.bottom.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.draw.baseline.hanging.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.draw.baseline.ideographic.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.draw.baseline.middle.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.draw.baseline.top.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.draw.fill.maxWidth.bound.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.draw.fontface.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.draw.fontface.notinpage.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.draw.fontface.repeat.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.draw.space.basic.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.draw.space.collapse.end.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.draw.space.collapse.nonspace.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.draw.space.collapse.other.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.draw.space.collapse.space.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.draw.space.collapse.start.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.drawing.style.absolute.spacing.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.drawing.style.font-relative.spacing.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.drawing.style.fontKerning.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.drawing.style.fontKerning.with.uppercase.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.drawing.style.fontStretch.settings.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.drawing.style.fontVariant.settings.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.drawing.style.invalid.spacing.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.drawing.style.letterSpacing.change.font.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.drawing.style.letterSpacing.measure.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.drawing.style.measure.direction.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.drawing.style.measure.textAlign.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.drawing.style.nonfinite.spacing.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.drawing.style.textRendering.settings.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.drawing.style.wordSpacing.change.font.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.drawing.style.wordSpacing.measure.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.font.parse.basic.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.font.parse.complex.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.font.parse.complex2.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.font.parse.family.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.font.parse.invalid.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.font.parse.size.percentage.default.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.font.parse.size.percentage.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.font.relative_size.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.font.weight.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.fontVariantCaps2.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.lang.default.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.measure.actualBoundingBox.combining-mark-offset.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.measure.actualBoundingBox.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.measure.actualBoundingBox.small-font.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.measure.actualBoundingBox.whitespace.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.measure.baselines.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.measure.emHeights-low-ascent.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.measure.emHeights-zero-descent.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.measure.emHeights.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.measure.fillTextCluster-options.tentative.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.measure.fillTextCluster-range.tentative.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.measure.fontBoundingBox-reduced-ascent.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.measure.fontBoundingBox-zero-descent.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.measure.fontBoundingBox.ahem.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.measure.fontBoundingBox.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.measure.getActualBoundingBox-exceptions.tentative.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.measure.getActualBoundingBox-full-text.tentative.html` | 0 / 4 |  |
| `html/canvas/element/text/2d.text.measure.getActualBoundingBox.tentative.html` | 0 / 6 |  |
| `html/canvas/element/text/2d.text.measure.index-from-offset-edge-cases.tentative.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.measure.index-from-offset-edges.tentative.html` | 0 / 10 |  |
| `html/canvas/element/text/2d.text.measure.index-from-offset.tentative.html` | 0 / 40 |  |
| `html/canvas/element/text/2d.text.measure.lang.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.measure.lang.inherit.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.measure.selection-rects-baselines.tentative.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.measure.selection-rects-exceptions.tentative.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.measure.selection-rects.tentative.html` | 0 / 24 |  |
| `html/canvas/element/text/2d.text.measure.strokeTextCluster-options.tentative.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.measure.strokeTextCluster-range.tentative.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.measure.text-clusters-exceptions.tentative.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.measure.text-clusters-position.tentative.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.measure.text-clusters-split.tentative.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.measure.width.basic.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.measure.width.nullCharacter.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.measure.width.space.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.shape.actualBoundingBox-exceptions.tentative.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.shape.actualBoundingBox-full-text.tentative.html` | 0 / 4 |  |
| `html/canvas/element/text/2d.text.shape.actualBoundingBox.tentative.html` | 0 / 6 |  |
| `html/canvas/element/text/2d.text.shape.advance-to-position-edge-cases.tentative.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.shape.advance-to-position-edges.tentative.html` | 0 / 10 |  |
| `html/canvas/element/text/2d.text.shape.advance-to-position.tentative.html` | 0 / 40 |  |
| `html/canvas/element/text/2d.text.shape.fillGlyphRun-range.tentative.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.shape.glyph-clusters-exceptions.tentative.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.shape.glyph-clusters-position.tentative.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.shape.glyph-clusters-split.tentative.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.shape.position-to-advance.tentative.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.shape.selection-rects-baselines.tentative.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.shape.selection-rects-exceptions.tentative.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.shape.selection-rects.tentative.html` | 0 / 24 |  |
| `html/canvas/element/text/2d.text.shape.strokeGlyphRun-range.tentative.html` | 0 / 1 |  |
| `html/canvas/element/text/2d.text.variationSelectors.html` | 0 / 1 |  |
| `html/canvas/element/text/parent-style-relative-units.html` | 0 / 2 |  |
| `html/canvas/element/video/2d.video.invalid.html` | 0 / 1 | harnais : Unhandled rejection: lecture non prise en charge |
| `html/canvas/element/wide-gamut-canvas/2d.color.space.getImageData.convert.html` | 0 / 4 |  |
| `html/canvas/element/wide-gamut-canvas/2d.color.space.p3.fillText.html` | 0 / 2 | harnais : Unhandled rejection: le chargement d'une police par script n'est pas pris en charge |
| `html/canvas/element/wide-gamut-canvas/2d.color.space.p3.fillText.shadow.html` | 0 / 2 | harnais : Unhandled rejection: le chargement d'une police par script n'est pas pris en charge |
| `html/canvas/element/wide-gamut-canvas/2d.color.space.p3.strokeText.html` | 0 / 2 | harnais : Unhandled rejection: le chargement d'une police par script n'est pas pris en charge |
| `html/canvas/element/wide-gamut-canvas/2d.color.space.toBlob.convert.html` | 0 / 4 |  |
| `html/canvas/element/wide-gamut-canvas/2d.color.space.toBlob.toDataURL.jpeg.linear.float.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/canvas/element/wide-gamut-canvas/2d.color.space.toBlob.toDataURL.png.linear.float.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/canvas/element/wide-gamut-canvas/2d.color.space.toDataURL.convert.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/canvas/offscreen/2d.conformance.requirements.missingargs.html` | 0 / 1 |  |
| `html/canvas/offscreen/OffscreenCanvas-ctx-font-sibling-index-invalid.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/canvas-context/2d.canvas.context.arguments.missing.html` | 0 / 1 |  |
| `html/canvas/offscreen/canvas-context/2d.canvas.context.casesensitive.html` | 0 / 1 |  |
| `html/canvas/offscreen/canvas-context/2d.canvas.context.emptystring.html` | 0 / 1 |  |
| `html/canvas/offscreen/canvas-context/2d.canvas.context.invalid.args.html` | 0 / 1 |  |
| `html/canvas/offscreen/canvas-context/2d.getContext.options.any.js` | 0 / 1 |  |
| `html/canvas/offscreen/canvas-host/2d.canvas.host.initial.reset.pattern.html` | 0 / 1 |  |
| `html/canvas/offscreen/canvas-host/2d.canvas.host.size.attributes.parse.minus.html` | 0 / 1 |  |
| `html/canvas/offscreen/canvas-host/2d.canvas.host.size.invalid.attributes.idl.html` | 0 / 1 |  |
| `html/canvas/offscreen/color-type/2d.color.type.u8p3.to.u8srgb.to.u8p3.html` | 0 / 1 |  |
| `html/canvas/offscreen/color-type/2d.color.type.u8srgb.to.u8p3.to.u8srgb.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.canvas.clear.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.canvas.copy.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.canvas.destination-atop.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.canvas.destination-in.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.canvas.destination-out.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.canvas.destination-over.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.canvas.lighter.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.canvas.source-atop.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.canvas.source-in.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.canvas.source-out.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.canvas.source-over.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.canvas.xor.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.globalAlpha.image.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.globalAlpha.imagepattern.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.image.clear.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.image.copy.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.image.destination-atop.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.image.destination-in.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.image.destination-out.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.image.destination-over.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.image.lighter.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.image.source-atop.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.image.source-in.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.image.source-out.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.image.source-over.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.image.xor.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.operation.clear.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.solid.clear.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.transparent.clear.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.uncovered.image.copy.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.uncovered.image.destination-atop.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.uncovered.image.destination-in.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.uncovered.image.source-in.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.uncovered.image.source-out.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.uncovered.pattern.copy.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.uncovered.pattern.destination-atop.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.uncovered.pattern.destination-in.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.uncovered.pattern.source-in.html` | 0 / 1 |  |
| `html/canvas/offscreen/compositing/2d.composite.uncovered.pattern.source-out.html` | 0 / 1 |  |
| `html/canvas/offscreen/conformance-requirements/2d.conformance.requirements.missingargs.html` | 0 / 1 |  |
| `html/canvas/offscreen/drawing-images-to-the-canvas/2d.drawImage.3arg.html` | 0 / 1 |  |
| `html/canvas/offscreen/drawing-images-to-the-canvas/2d.drawImage.5arg.html` | 0 / 1 |  |
| `html/canvas/offscreen/drawing-images-to-the-canvas/2d.drawImage.9arg.basic.html` | 0 / 1 |  |
| `html/canvas/offscreen/drawing-images-to-the-canvas/2d.drawImage.9arg.destpos.html` | 0 / 1 |  |
| `html/canvas/offscreen/drawing-images-to-the-canvas/2d.drawImage.9arg.destsize.html` | 0 / 1 |  |
| `html/canvas/offscreen/drawing-images-to-the-canvas/2d.drawImage.9arg.sourcepos.html` | 0 / 1 |  |
| `html/canvas/offscreen/drawing-images-to-the-canvas/2d.drawImage.9arg.sourcesize.html` | 0 / 1 |  |
| `html/canvas/offscreen/drawing-images-to-the-canvas/2d.drawImage.alpha.html` | 0 / 1 |  |
| `html/canvas/offscreen/drawing-images-to-the-canvas/2d.drawImage.clip.html` | 0 / 1 |  |
| `html/canvas/offscreen/drawing-images-to-the-canvas/2d.drawImage.composite.html` | 0 / 1 |  |
| `html/canvas/offscreen/drawing-images-to-the-canvas/2d.drawImage.floatsource.html` | 0 / 1 |  |
| `html/canvas/offscreen/drawing-images-to-the-canvas/2d.drawImage.negativedest.html` | 0 / 1 |  |
| `html/canvas/offscreen/drawing-images-to-the-canvas/2d.drawImage.negativedir.html` | 0 / 1 |  |
| `html/canvas/offscreen/drawing-images-to-the-canvas/2d.drawImage.negativesource.html` | 0 / 1 |  |
| `html/canvas/offscreen/drawing-images-to-the-canvas/2d.drawImage.nonfinite.html` | 0 / 1 |  |
| `html/canvas/offscreen/drawing-images-to-the-canvas/2d.drawImage.nowrap.html` | 0 / 1 |  |
| `html/canvas/offscreen/drawing-images-to-the-canvas/2d.drawImage.path.html` | 0 / 1 |  |
| `html/canvas/offscreen/drawing-images-to-the-canvas/2d.drawImage.svg.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/canvas/offscreen/drawing-images-to-the-canvas/2d.drawImage.transform.html` | 0 / 1 |  |
| `html/canvas/offscreen/drawing-images-to-the-canvas/2d.drawImage.zerosource.html` | 0 / 1 |  |
| `html/canvas/offscreen/drawing-images-to-the-canvas/2d.drawImage.zerosource.image.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/canvas/offscreen/drawing-rectangles-to-the-canvas/2d.fillRect.shadow.html` | 0 / 1 |  |
| `html/canvas/offscreen/drawing-rectangles-to-the-canvas/2d.strokeRect.shadow.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.fillStyle.CSSHSL.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.fillStyle.CSSRGB.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.fillStyle.colormix.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.fillStyle.parse.invalid.css-color-4-hsl-1.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.fillStyle.parse.invalid.css-color-4-hsl-2.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.fillStyle.parse.invalid.css-color-4-hsl-3.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.fillStyle.parse.invalid.css-color-4-hsl-4.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.fillStyle.parse.invalid.css-color-4-hsl-5.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.fillStyle.parse.invalid.css-color-4-hsla-1.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.fillStyle.parse.invalid.css-color-4-hsla-2.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.fillStyle.parse.invalid.css-color-4-hsla-3.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.fillStyle.parse.invalid.css-color-4-rgb-1.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.fillStyle.parse.invalid.css-color-4-rgb-2.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.fillStyle.parse.invalid.css-color-4-rgb-3.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.fillStyle.parse.invalid.css-color-4-rgb-4.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.fillStyle.parse.invalid.css-color-4-rgb-5.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.fillStyle.parse.invalid.css-color-4-rgba-1.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.fillStyle.parse.invalid.css-color-4-rgba-2.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.fillStyle.parse.invalid.css-color-4-rgba-3.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.fillStyle.parse.invalid.hsl-3.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.fillStyle.parse.invalid.hsl-4.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.fillStyle.parse.invalid.hsl-6.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.fillStyle.parse.invalid.hsla-2.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.fillStyle.parse.invalid.hsla-3.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.fillStyle.parse.invalid.rgb-1.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.fillStyle.parse.invalid.rgb-2.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.fillStyle.parse.invalid.rgba-1.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.fillStyle.parse.invalid.rgba-4.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.fillStyle.parse.invalid.rgba-5.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.fillStyle.parse.system.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.fillStyle.relativecolor.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.fillStyle.toStringFunctionCallback.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.gradient.conic.invalid.inputs.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.gradient.conic.negative.rotation.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.gradient.conic.positive.rotation.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.gradient.interpolate.zerosize.fill.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.gradient.interpolate.zerosize.fillRect.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.gradient.interpolate.zerosize.stroke.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.gradient.interpolate.zerosize.strokeRect.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.gradient.object.invalidoffset.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.gradient.radial.cone.behind.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.gradient.radial.cone.beside.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.gradient.radial.cone.bottom.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.gradient.radial.cone.cylinder.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.gradient.radial.cone.shape2.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.gradient.radial.equal.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.gradient.radial.inside2.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.gradient.radial.inside3.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.gradient.radial.outside2.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.gradient.radial.outside3.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.gradient.radial.touch1.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.gradient.radial.touch2.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.gradient.radial.touch3.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.gradient.relativecolor.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.pattern.basic.image.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.pattern.basic.type.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.pattern.crosscanvas.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.pattern.modify.canvas1.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.pattern.modify.canvas2.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.pattern.paint.norepeat.basic.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.pattern.paint.norepeat.coord1.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.pattern.paint.norepeat.coord2.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.pattern.paint.norepeat.coord3.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.pattern.paint.norepeat.outside.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.pattern.paint.norepeat.outside_arc.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.pattern.paint.orientation.image.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.pattern.paint.repeat.basic.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.pattern.paint.repeat.coord1.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.pattern.paint.repeat.coord2.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.pattern.paint.repeat.coord3.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.pattern.paint.repeat.outside.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.pattern.paint.repeatx.basic.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.pattern.paint.repeatx.coord1.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.pattern.paint.repeatx.outside.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.pattern.paint.repeaty.basic.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.pattern.paint.repeaty.coord1.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.pattern.paint.repeaty.outside.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.pattern.repeat.empty.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.pattern.transform.identity.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.pattern.transform.invalid.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.strokeStyle.colormix.html` | 0 / 1 |  |
| `html/canvas/offscreen/fill-and-stroke-styles/2d.strokeStyle.relativecolor.html` | 0 / 1 |  |
| `html/canvas/offscreen/filters/2d.filter.canvasFilterObject.blur.exceptions.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/filters/2d.filter.canvasFilterObject.colorMatrix.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/filters/2d.filter.canvasFilterObject.convolveMatrix.exceptions.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/filters/2d.filter.canvasFilterObject.dropShadow.exceptions.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/filters/2d.filter.canvasFilterObject.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/filters/2d.filter.canvasFilterObject.turbulence.inputTypes.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/filters/2d.filter.layers.colorMatrix.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/filters/2d.filter.layers.convolveMatrix.exceptions.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/filters/2d.filter.layers.dropShadow.exceptions.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/filters/2d.filter.layers.turbulence.inputTypes.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/filters/2d.filter.value.html` | 0 / 1 |  |
| `html/canvas/offscreen/layers/2d.layer.beginLayer-options.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/layers/2d.layer.ctm.getTransform.html` | 0 / 1 |  |
| `html/canvas/offscreen/layers/2d.layer.exceptions-are-no-op.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/layers/2d.layer.invalid-calls.beginLayer-reset-endLayer.html` | 0 / 1 |  |
| `html/canvas/offscreen/layers/2d.layer.invalid-calls.beginLayer-restore.html` | 0 / 1 |  |
| `html/canvas/offscreen/layers/2d.layer.invalid-calls.beginLayer-save-endLayer.html` | 0 / 1 |  |
| `html/canvas/offscreen/layers/2d.layer.invalid-calls.endLayer.html` | 0 / 1 |  |
| `html/canvas/offscreen/layers/2d.layer.invalid-calls.save-beginLayer-restore.html` | 0 / 1 |  |
| `html/canvas/offscreen/layers/2d.layer.invalid-calls.save-endLayer.html` | 0 / 1 |  |
| `html/canvas/offscreen/layers/2d.layer.layer-rendering-state-reset-in-layer.html` | 0 / 1 |  |
| `html/canvas/offscreen/layers/2d.layer.malformed-operations-with-promises.html` | 0 / 2 |  |
| `html/canvas/offscreen/layers/2d.layer.malformed-operations.html` | 0 / 5 |  |
| `html/canvas/offscreen/layers/2d.layer.valid-calls.beginLayer-endLayer.html` | 0 / 1 |  |
| `html/canvas/offscreen/layers/2d.layer.valid-calls.beginLayer-save.html` | 0 / 1 |  |
| `html/canvas/offscreen/layers/2d.layer.valid-calls.beginLayer.html` | 0 / 1 |  |
| `html/canvas/offscreen/layers/2d.layer.valid-calls.save-beginLayer.html` | 0 / 1 |  |
| `html/canvas/offscreen/line-styles/2d.line.join.round.html` | 0 / 1 |  |
| `html/canvas/offscreen/manual/context-attributes/getContextAttributes.html` | 4 / 16 |  |
| `html/canvas/offscreen/manual/convert-to-blob/offscreencanvas.convert.to.blob.html` | 2 / 8 | harnais : délai du harnais dépassé |
| `html/canvas/offscreen/manual/convert-to-blob/offscreencanvas.convert.to.blob.w.html` | 2 / 16 | harnais : délai du harnais dépassé |
| `html/canvas/offscreen/manual/draw-element-image/animated-gif-worker.tentative.html` | 0 / 2 | harnais : Uncaught TypeError: .transferControlToOffscreen n'est pas une fonction (c'est undefined) |
| `html/canvas/offscreen/manual/draw-element-image/canvas-transform-update-worker.tentative.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/canvas/offscreen/manual/draw-element-image/offscreenCanvas-drawElementImage-worker.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/manual/draw-element-image/offscreenCanvas-drawElementImage.tentative.html` | 0 / 2 |  |
| `html/canvas/offscreen/manual/draw-element-image/video-worker.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/manual/filter/offscreencanvas.filter.w.html` | 0 / 1 |  |
| `html/canvas/offscreen/manual/image-smoothing/image.smoothing.html` | 6 / 9 |  |
| `html/canvas/offscreen/manual/the-offscreen-canvas/offscreencanvas.constructor.html` | 2 / 3 |  |
| `html/canvas/offscreen/manual/the-offscreen-canvas/offscreencanvas.getcontext.html` | 3 / 8 |  |
| `html/canvas/offscreen/manual/the-offscreen-canvas/offscreencanvas.placeholder.size.html` | 0 / 4 |  |
| `html/canvas/offscreen/manual/the-offscreen-canvas/offscreencanvas.resize.html` | 4 / 10 |  |
| `html/canvas/offscreen/manual/the-offscreen-canvas/offscreencanvas.transfer.lowlatency.nocrash.html` | 0 / 1 |  |
| `html/canvas/offscreen/manual/the-offscreen-canvas/offscreencanvas.transfer.to.imagebitmap.html` | 1 / 6 |  |
| `html/canvas/offscreen/manual/the-offscreen-canvas/offscreencanvas.transfer.to.imagebitmap.nocrash.html` | 0 / 1 |  |
| `html/canvas/offscreen/manual/the-offscreen-canvas/offscreencanvas.transfer.to.imagebitmap.w.html` | 1 / 8 |  |
| `html/canvas/offscreen/manual/the-offscreen-canvas/offscreencanvas.transfercontrol.to.offscreen.html` | 0 / 3 |  |
| `html/canvas/offscreen/manual/the-offscreen-canvas/offscreencanvas.transfercontrol.to.offscreen.w.html` | 0 / 4 |  |
| `html/canvas/offscreen/manual/the-offscreen-canvas/offscreencanvas.transferrable.html` | 0 / 5 |  |
| `html/canvas/offscreen/manual/the-offscreen-canvas/offscreencanvas.transferrable.sw.https.window.js` | 0 / 1 | harnais : TypeError: impossible de lire la propriété « getRegistrations » de undefined (valeur de .serviceWorker) |
| `html/canvas/offscreen/manual/the-offscreen-canvas/offscreencanvas.transferrable.w.html` | 0 / 6 |  |
| `html/canvas/offscreen/manual/transformations/2d.transformation.getTransform.html` | 0 / 1 |  |
| `html/canvas/offscreen/path-objects/2d.path.arc.scale.1.html` | 0 / 1 |  |
| `html/canvas/offscreen/path-objects/2d.path.isPointInPath.basic.html` | 0 / 1 |  |
| `html/canvas/offscreen/path-objects/2d.path.isPointInPath.edge.html` | 0 / 1 |  |
| `html/canvas/offscreen/path-objects/2d.path.isPointInPath.nobitmap.html` | 0 / 1 |  |
| `html/canvas/offscreen/path-objects/2d.path.isPointInStroke.basic.html` | 0 / 1 |  |
| `html/canvas/offscreen/path-objects/2d.path.isPointInStroke.nobitmap.html` | 0 / 1 |  |
| `html/canvas/offscreen/path-objects/2d.path.isPointInStroke.nobitmap.path2d.html` | 0 / 1 |  |
| `html/canvas/offscreen/path-objects/2d.path.isPointInStroke.nobitmap.transform.html` | 0 / 1 |  |
| `html/canvas/offscreen/path-objects/2d.path.isPointInStroke.scaleddashes.html` | 0 / 1 |  |
| `html/canvas/offscreen/path-objects/2d.path.lineTo.nonfinite.details.html` | 0 / 1 |  |
| `html/canvas/offscreen/path-objects/2d.path.roundrect.1.radius.dompoint.html` | 0 / 1 |  |
| `html/canvas/offscreen/path-objects/2d.path.roundrect.1.radius.dompoint.single.argument.html` | 0 / 1 |  |
| `html/canvas/offscreen/path-objects/2d.path.roundrect.2.radii.1.dompoint.html` | 0 / 1 |  |
| `html/canvas/offscreen/path-objects/2d.path.roundrect.2.radii.2.dompoint.html` | 0 / 1 |  |
| `html/canvas/offscreen/path-objects/2d.path.roundrect.3.radii.1.dompoint.html` | 0 / 1 |  |
| `html/canvas/offscreen/path-objects/2d.path.roundrect.3.radii.2.dompoint.html` | 0 / 1 |  |
| `html/canvas/offscreen/path-objects/2d.path.roundrect.3.radii.3.dompoint.html` | 0 / 1 |  |
| `html/canvas/offscreen/path-objects/2d.path.roundrect.4.radii.1.dompoint.html` | 0 / 1 |  |
| `html/canvas/offscreen/path-objects/2d.path.roundrect.4.radii.2.dompoint.html` | 0 / 1 |  |
| `html/canvas/offscreen/path-objects/2d.path.roundrect.4.radii.3.dompoint.html` | 0 / 1 |  |
| `html/canvas/offscreen/path-objects/2d.path.roundrect.4.radii.4.dompoint.html` | 0 / 1 |  |
| `html/canvas/offscreen/path-objects/2d.path.roundrect.badinput.html` | 0 / 1 |  |
| `html/canvas/offscreen/path-objects/2d.path.roundrect.nonfinite.html` | 0 / 1 |  |
| `html/canvas/offscreen/path-objects/2d.path.roundrect.radius.negative.html` | 0 / 1 |  |
| `html/canvas/offscreen/path-objects/2d.path.roundrect.winding.html` | 0 / 1 |  |
| `html/canvas/offscreen/path-objects/2d.path.stroke.prune.arc.html` | 0 / 1 |  |
| `html/canvas/offscreen/path-objects/2d.path.stroke.prune.closed.html` | 0 / 1 |  |
| `html/canvas/offscreen/path-objects/2d.path.stroke.prune.curve.html` | 0 / 1 |  |
| `html/canvas/offscreen/path-objects/2d.path.stroke.prune.line.html` | 0 / 1 |  |
| `html/canvas/offscreen/path-objects/2d.path.stroke.prune.rect.html` | 0 / 1 |  |
| `html/canvas/offscreen/pixel-manipulation/2d.imageData.createImageBitmap.srgb.rgba.float16.html` | 0 / 1 |  |
| `html/canvas/offscreen/pixel-manipulation/2d.imageData.object.properties.html` | 0 / 1 |  |
| `html/canvas/offscreen/pixel-manipulation/2d.imageData.put.basic.rgba.float16.html` | 0 / 1 |  |
| `html/canvas/offscreen/pixel-manipulation/2d.imageData.put.nonfinite.html` | 0 / 1 |  |
| `html/canvas/offscreen/reset/2d.reset.state.font_kerning.html` | 0 / 1 |  |
| `html/canvas/offscreen/reset/2d.reset.state.font_stretch.html` | 0 / 1 |  |
| `html/canvas/offscreen/reset/2d.reset.state.font_variant_caps.html` | 0 / 1 |  |
| `html/canvas/offscreen/reset/2d.reset.state.letter_spacing.html` | 0 / 1 |  |
| `html/canvas/offscreen/reset/2d.reset.state.text_rendering.html` | 0 / 1 |  |
| `html/canvas/offscreen/reset/2d.reset.state.word_spacing.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.alpha.2.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.alpha.3.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.alpha.4.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.alpha.5.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.canvas.alpha.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.canvas.basic.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.canvas.transparent.2.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.clip.1.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.clip.3.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.composite.1.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.composite.2.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.enable.blur.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.enable.x.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.enable.y.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.gradient.alpha.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.gradient.basic.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.gradient.transparent.2.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.image.alpha.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.image.basic.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.image.scale.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.image.section.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.image.transparent.1.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.image.transparent.2.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.offset.negativeX.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.offset.negativeY.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.offset.positiveX.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.offset.positiveY.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.outside.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.pattern.alpha.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.pattern.basic.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.pattern.transparent.1.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.pattern.transparent.2.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.stroke.basic.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.stroke.cap.2.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.stroke.join.2.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.transform.1.html` | 0 / 1 |  |
| `html/canvas/offscreen/shadows/2d.shadow.transform.2.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.draw.align.center.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.draw.align.end.ltr.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.draw.align.end.rtl.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.draw.align.left.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.draw.align.right.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.draw.align.start.ltr.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.draw.align.start.rtl.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.draw.baseline.alphabetic.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.draw.baseline.bottom.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.draw.baseline.hanging.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.draw.baseline.ideographic.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.draw.baseline.middle.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.draw.baseline.top.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.draw.fill.maxWidth.bound.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.draw.fontface.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.draw.fontface.notinpage.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.draw.fontface.repeat.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.draw.space.basic.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.draw.space.collapse.end.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.draw.space.collapse.nonspace.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.draw.space.collapse.other.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.draw.space.collapse.space.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.draw.space.collapse.start.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.drawing.style.absolute.spacing.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.drawing.style.font-relative.spacing.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.drawing.style.fontKerning.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.drawing.style.fontKerning.with.uppercase.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.drawing.style.fontStretch.settings.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.drawing.style.fontVariant.settings.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.drawing.style.invalid.spacing.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.drawing.style.letterSpacing.change.font.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.drawing.style.letterSpacing.measure.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.drawing.style.measure.direction.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.drawing.style.measure.textAlign.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.drawing.style.nonfinite.spacing.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.drawing.style.textRendering.settings.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.drawing.style.wordSpacing.change.font.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.drawing.style.wordSpacing.measure.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.font.parse.basic.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.font.parse.complex.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.font.parse.complex2.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.font.parse.family.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.font.parse.invalid.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.font.relative_size.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.font.weight.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.fontVariantCaps2.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.lang.default.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.measure.actualBoundingBox.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.measure.actualBoundingBox.small-font.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.measure.actualBoundingBox.whitespace.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.measure.baselines.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.measure.emHeights-low-ascent.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.measure.emHeights-zero-descent.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.measure.emHeights.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.measure.fillTextCluster-options.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.measure.fillTextCluster-range.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.measure.fontBoundingBox-reduced-ascent.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.measure.fontBoundingBox-zero-descent.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.measure.fontBoundingBox.ahem.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.measure.fontBoundingBox.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.measure.getActualBoundingBox-exceptions.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.measure.getActualBoundingBox-full-text.tentative.html` | 0 / 4 |  |
| `html/canvas/offscreen/text/2d.text.measure.getActualBoundingBox.tentative.html` | 0 / 6 |  |
| `html/canvas/offscreen/text/2d.text.measure.index-from-offset-edge-cases.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.measure.index-from-offset-edges.tentative.html` | 0 / 10 |  |
| `html/canvas/offscreen/text/2d.text.measure.index-from-offset.tentative.html` | 0 / 40 |  |
| `html/canvas/offscreen/text/2d.text.measure.lang.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.measure.lang.inherit.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.measure.selection-rects-baselines.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.measure.selection-rects-exceptions.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.measure.selection-rects.tentative.html` | 0 / 24 |  |
| `html/canvas/offscreen/text/2d.text.measure.strokeTextCluster-options.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.measure.strokeTextCluster-range.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.measure.text-clusters-exceptions.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.measure.text-clusters-position.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.measure.text-clusters-split.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.measure.width.basic.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.measure.width.nullCharacter.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.measure.width.space.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.shape.actualBoundingBox-exceptions.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.shape.actualBoundingBox-full-text.tentative.html` | 0 / 4 |  |
| `html/canvas/offscreen/text/2d.text.shape.actualBoundingBox.tentative.html` | 0 / 6 |  |
| `html/canvas/offscreen/text/2d.text.shape.advance-to-position-edge-cases.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.shape.advance-to-position-edges.tentative.html` | 0 / 10 |  |
| `html/canvas/offscreen/text/2d.text.shape.advance-to-position.tentative.html` | 0 / 40 |  |
| `html/canvas/offscreen/text/2d.text.shape.fillGlyphRun-range.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.shape.glyph-clusters-exceptions.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.shape.glyph-clusters-position.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.shape.glyph-clusters-split.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.shape.position-to-advance.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.shape.selection-rects-baselines.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.shape.selection-rects-exceptions.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/text/2d.text.shape.selection-rects.tentative.html` | 0 / 24 |  |
| `html/canvas/offscreen/text/2d.text.shape.strokeGlyphRun-range.tentative.html` | 0 / 1 |  |
| `html/canvas/offscreen/wide-gamut-canvas/2d.color.space.getImageData.convert.html` | 0 / 4 |  |
| `html/canvas/offscreen/wide-gamut-canvas/2d.color.space.toBlob.toDataURL.jpeg.linear.float.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/canvas/offscreen/wide-gamut-canvas/2d.color.space.toBlob.toDataURL.png.linear.float.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/capability-delegation/delegate-fullscreen-request-popup-cross-origin.https.sub.tentative.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/capability-delegation/delegate-fullscreen-request-popup-same-origin.https.tentative.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/capability-delegation/delegate-fullscreen-request-subframe-cross-origin.https.sub.tentative.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/capability-delegation/delegate-fullscreen-request-subframe-same-origin.https.tentative.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/capability-delegation/delegation-consumes-activation.https.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/capability-delegation/delegation-sender-checks.tentative.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/capability-delegation/frame-capability-delegation-popup-blocked.tentative.html` | 0 / 1 |  |
| `html/cross-origin-embedder-policy/about-blank-popup.https.html` | 0 / 1 | harnais : Test named 'Cross-Origin-Embedder-Policy is inherited by about:blank popup.' specified 1 'cleanup' function, and 1 faile |
| `html/cross-origin-embedder-policy/blob.https.html` | 0 / 9 | harnais : délai du harnais dépassé |
| `html/cross-origin-embedder-policy/block-local-documents-inheriting-none.https.html` | 0 / 1 |  |
| `html/cross-origin-embedder-policy/cache-storage-reporting-dedicated-worker.https.html` | 0 / 2 |  |
| `html/cross-origin-embedder-policy/cache-storage-reporting-document.https.html` | 0 / 2 |  |
| `html/cross-origin-embedder-policy/cache-storage-reporting-service-worker.https.html` | 0 / 2 |  |
| `html/cross-origin-embedder-policy/cache-storage-reporting-shared-worker.https.html` | 0 / 2 |  |
| `html/cross-origin-embedder-policy/coep-frame-javascript.https.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/cross-origin-embedder-policy/coep-on-response-from-service-worker.https.html` | 0 / 13 |  |
| `html/cross-origin-embedder-policy/credentialless/cache-storage.https.window.js` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/cross-origin-embedder-policy/credentialless/cache.window.js` | 0 / 1 | harnais : Error: assert_equals: expected "done" but got "import json\nfrom wptserve.utils import isomorphic_decode\n\n# A server u |
| `html/cross-origin-embedder-policy/credentialless/cross-origin-isolated.window.js` | 0 / 5 |  |
| `html/cross-origin-embedder-policy/credentialless/dedicated-worker.https.window.js` | 0 / 1 |  |
| `html/cross-origin-embedder-policy/credentialless/fetch.https.window.js` | 0 / 1 |  |
| `html/cross-origin-embedder-policy/credentialless/iframe-coep-credentialless.https.window.js` | 0 / 0 | boucle sans fin (processus tué) |
| `html/cross-origin-embedder-policy/credentialless/iframe-coep-none.https.window.js` | 0 / 0 | boucle sans fin (processus tué) |
| `html/cross-origin-embedder-policy/credentialless/iframe-coep-redirect.https.window.js` | 0 / 0 | boucle sans fin (processus tué) |
| `html/cross-origin-embedder-policy/credentialless/iframe-coep-require-corp.https.window.js` | 0 / 0 | boucle sans fin (processus tué) |
| `html/cross-origin-embedder-policy/credentialless/iframe.window.js` | 0 / 1 |  |
| `html/cross-origin-embedder-policy/credentialless/image.https.window.js` | 0 / 1 |  |
| `html/cross-origin-embedder-policy/credentialless/link.https.window.js` | 0 / 1 |  |
| `html/cross-origin-embedder-policy/credentialless/redirect.window.js` | 0 / 1 |  |
| `html/cross-origin-embedder-policy/credentialless/reporting-navigation.https.window.js` | 6 / 13 |  |
| `html/cross-origin-embedder-policy/credentialless/reporting-subresource-corp.https.window.js` | 0 / 1 |  |
| `html/cross-origin-embedder-policy/credentialless/script.https.window.js` | 0 / 1 |  |
| `html/cross-origin-embedder-policy/credentialless/service-worker-coep-credentialless-proxy.https.window.js` | 0 / 1 |  |
| `html/cross-origin-embedder-policy/credentialless/service-worker-coep-none-proxy.https.window.js` | 0 / 1 |  |
| `html/cross-origin-embedder-policy/credentialless/service-worker.https.window.js` | 0 / 1 |  |
| `html/cross-origin-embedder-policy/credentialless/shared-worker.https.window.js` | 0 / 1 |  |
| `html/cross-origin-embedder-policy/credentialless/video.https.window.js` | 0 / 6 | harnais : Error: assert_equals: expected "done" but got "import json\nfrom wptserve.utils import isomorphic_decode\n\n# A server u |
| `html/cross-origin-embedder-policy/credentialless/websocket.https.window.js` | 0 / 1 |  |
| `html/cross-origin-embedder-policy/cross-origin-iframe-navigates-to-own-blob.https.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/cross-origin-embedder-policy/cross-origin-isolated-permission-iframe.https.window.js` | 2 / 8 | harnais : délai du harnais dépassé |
| `html/cross-origin-embedder-policy/cross-origin-isolated-permission-worker.https.window.js` | 4 / 30 | harnais : Unhandled rejection: impossible de lire la propriété « register » de undefined (valeur de .serviceWorker) |
| `html/cross-origin-embedder-policy/data.https.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/cross-origin-embedder-policy/dedicated-worker-cache-storage.https.html` | 0 / 8 |  |
| `html/cross-origin-embedder-policy/dedicated-worker.https.html` | 0 / 14 | harnais : délai du harnais dépassé |
| `html/cross-origin-embedder-policy/header-parsing.https.html` | 17 / 25 |  |
| `html/cross-origin-embedder-policy/iframe-history-none-require-corp.https.html` | 0 / 1 | harnais : Test named '"none" top-level: navigating a frame back from "require-corp" should succeed' specified 1 'cleanup' function |
| `html/cross-origin-embedder-policy/javascript.https.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/cross-origin-embedder-policy/multi-globals/workers-coep-report.https.html` | 0 / 1 |  |
| `html/cross-origin-embedder-policy/no-secure-context-https-iframe.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/cross-origin-embedder-policy/no-secure-context.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/cross-origin-embedder-policy/non-initial-about-blank.https.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/cross-origin-embedder-policy/none-load-from-cache-storage.https.html` | 0 / 29 |  |
| `html/cross-origin-embedder-policy/none-sw-from-none.https.html` | 2 / 13 | harnais : délai du harnais dépassé |
| `html/cross-origin-embedder-policy/none-sw-from-require-corp.https.html` | 2 / 13 | harnais : délai du harnais dépassé |
| `html/cross-origin-embedder-policy/none.https.html` | 1 / 4 | harnais : délai du harnais dépassé |
| `html/cross-origin-embedder-policy/reflection-credentialless.tentative.https.any.js` | 0 / 1 |  |
| `html/cross-origin-embedder-policy/reflection-require-corp.tentative.https.any.js` | 0 / 1 |  |
| `html/cross-origin-embedder-policy/reflection-unsafe-none.tentative.https.any.js` | 0 / 1 |  |
| `html/cross-origin-embedder-policy/report-only-require-corp.https.html` | 2 / 8 | harnais : Test named '"none" top-level: navigating a frame back from "require-corp" should succeed' specified 1 'cleanup' function |
| `html/cross-origin-embedder-policy/reporting-cached-and-service-worker-responses.https.html` | 0 / 12 | harnais : délai du harnais dépassé |
| `html/cross-origin-embedder-policy/reporting-navigation.https.html` | 14 / 30 |  |
| `html/cross-origin-embedder-policy/reporting-report-to-must-be-string.https.window.js` | 0 / 4 |  |
| `html/cross-origin-embedder-policy/reporting-subresource-corp.https.html` | 0 / 26 |  |
| `html/cross-origin-embedder-policy/reporting-to-document-reporting-endpoint.https.window.js` | 0 / 3 |  |
| `html/cross-origin-embedder-policy/reporting-to-endpoint.https.html` | 0 / 5 |  |
| `html/cross-origin-embedder-policy/reporting-to-frame-owner.https.html` | 0 / 9 |  |
| `html/cross-origin-embedder-policy/reporting-to-worker-owner.https.html` | 0 / 9 | harnais : délai du harnais dépassé |
| `html/cross-origin-embedder-policy/require-corp-about-blank.https.html` | 2 / 3 |  |
| `html/cross-origin-embedder-policy/require-corp-about-srcdoc.https.html` | 2 / 3 |  |
| `html/cross-origin-embedder-policy/require-corp-cached-images.https.html` | 0 / 5 | harnais : délai du harnais dépassé |
| `html/cross-origin-embedder-policy/require-corp-load-from-cache-storage.https.html` | 0 / 29 |  |
| `html/cross-origin-embedder-policy/require-corp-revalidated-images.https.html` | 0 / 5 | harnais : délai du harnais dépassé |
| `html/cross-origin-embedder-policy/require-corp-sw-from-none.https.html` | 2 / 13 | harnais : délai du harnais dépassé |
| `html/cross-origin-embedder-policy/require-corp-sw-from-require-corp.https.html` | 2 / 13 | harnais : délai du harnais dépassé |
| `html/cross-origin-embedder-policy/require-corp-sw.https.html` | 0 / 5 | harnais : Unhandled rejection: impossible de lire la propriété « addEventListener » de undefined (valeur de .serviceWorker) |
| `html/cross-origin-embedder-policy/require-corp-worker-script-revalidation.html` | 0 / 1 |  |
| `html/cross-origin-embedder-policy/require-corp.https.html` | 4 / 16 | harnais : délai du harnais dépassé |
| `html/cross-origin-embedder-policy/sandbox.https.html` | 0 / 1 |  |
| `html/cross-origin-embedder-policy/service-worker-cache-storage.https.html` | 0 / 4 |  |
| `html/cross-origin-embedder-policy/shared-workers.https.html` | 0 / 11 |  |
| `html/cross-origin-embedder-policy/srcdoc.https.html` | 0 / 1 |  |
| `html/cross-origin-embedder-policy/worker-inheritance.sub.https.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/blob-popup.https.html` | 0 / 1 | harnais : Timeout while running cleanup for test named "Cross-Origin-Opener-Policy and a blob URL popup". |
| `html/cross-origin-opener-policy/coep-blob-popup.https.html` | 0 / 3 | harnais : Test named 'COOP+COEP blob URL popup: window.open()' specified 1 'cleanup' function, and 1 failed. |
| `html/cross-origin-opener-policy/coep-navigate-popup.https.html` | 0 / 12 | harnais : Timeout while running cleanup for test named "Popup navigating to same-origin with coop/coep". |
| `html/cross-origin-opener-policy/coep-redirect.https.html` | 0 / 6 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/coep-with-cross-origin.https.html` | 0 / 5 |  |
| `html/cross-origin-opener-policy/coep-with-same-origin.https.html` | 0 / 5 |  |
| `html/cross-origin-opener-policy/coep-with-same-site.https.html` | 0 / 5 |  |
| `html/cross-origin-opener-policy/coop-coep-sandbox.https.html` | 2 / 4 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/coop-csp-sandbox-navigate.https.html` | 0 / 4 | harnais : Test named 'CSP: sandbox allow-popups allow-scripts; CSP sandbox popup navigate to Cross-Origin-Opener-Policy document s |
| `html/cross-origin-opener-policy/coop-csp-sandbox.https.html` | 2 / 4 | harnais : Test named 'CSP: sandbox allow-popups allow-scripts; CSP sandboxed Cross-Origin-Opener-Policy popup should result in a n |
| `html/cross-origin-opener-policy/coop-navigate-same-origin-csp-sandbox.html` | 0 / 1 |  |
| `html/cross-origin-opener-policy/coop-navigated-history-popup.https.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/coop-navigated-popup.https.html` | 0 / 1 |  |
| `html/cross-origin-opener-policy/coop-noopener-allow-popups.https.html` | 0 / 9 | harnais : Timeout while running cleanup for test named "     Cross-Origin-Opener-Policy: noopener-allow-popups means that the open |
| `html/cross-origin-opener-policy/coop-popup-opener-navigates.https.html` | 0 / 1 |  |
| `html/cross-origin-opener-policy/coop-same-origin-allow-popups-document-write.html` | 0 / 1 |  |
| `html/cross-origin-opener-policy/coop-sandbox-cuts-opener.https.html` | 0 / 2 |  |
| `html/cross-origin-opener-policy/coop-sandbox-redirects-cuts-opener.https.html` | 0 / 2 |  |
| `html/cross-origin-opener-policy/coop-sandbox.https.html` | 2 / 4 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/header-parsing-failures.https.html` | 0 / 16 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/header-parsing-non-ascii.https.html` | 0 / 1 |  |
| `html/cross-origin-opener-policy/header-parsing-repeated.https.html` | 0 / 1 |  |
| `html/cross-origin-opener-policy/header-parsing-successes.https.html` | 0 / 6 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/historical/coep-navigate-popup-unsafe-inherit.https.html` | 0 / 4 | harnais : Timeout while running cleanup for test named "Popup navigating to same-origin with coop unsafe-inherit/coep". |
| `html/cross-origin-opener-policy/historical/popup-same-origin-unsafe-allow-outgoing-with-cross-origin.https.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/historical/popup-same-origin-unsafe-allow-outgoing-with-same-origin.https.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/historical/popup-same-origin-unsafe-allow-outgoing-with-same-site.https.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/historical/popup-same-site-unsafe-allow-outgoing-with-cross-origin.https.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/historical/popup-same-site-unsafe-allow-outgoing-with-same-origin.https.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/historical/popup-same-site-unsafe-allow-outgoing-with-same-site.https.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/historical/popup-same-site-with-cross-origin.https.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/historical/popup-same-site-with-same-origin.https.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/historical/popup-same-site-with-same-site.https.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/iframe-popup-same-origin-allow-popups-to-same-origin-allow-popups.https.html` | 0 / 27 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/iframe-popup-same-origin-allow-popups-to-same-origin.https.html` | 0 / 27 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/iframe-popup-same-origin-allow-popups-to-unsafe-none.https.html` | 0 / 27 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/iframe-popup-same-origin-to-same-origin.https.html` | 0 / 27 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/iframe-popup-same-origin-to-unsafe-none.https.html` | 0 / 27 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/iframe-popup-unsafe-none-to-same-origin.https.html` | 0 / 27 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/iframe-popup-unsafe-none-to-unsafe-none.https.html` | 0 / 27 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/javascript-url.https.html` | 0 / 18 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/navigate-to-aboutblank.https.html` | 0 / 5 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/navigate-top-to-aboutblank.https.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/no-https.html` | 0 / 2 | harnais : Test named 'Cross-Origin-Opener-Policy only works over secure contexts' specified 1 'cleanup' function, and 1 failed. |
| `html/cross-origin-opener-policy/non-secure-to-secure-context-navigation.https.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/noopener-popup-cross-origin-isolation.https.html` | 0 / 4 |  |
| `html/cross-origin-opener-policy/popup-coop-by-sw.https.html` | 0 / 8 | harnais : Test named 'opener:basic, openee:basic, sw:basic' specified 1 'cleanup' function, and 1 failed. |
| `html/cross-origin-opener-policy/popup-meta-http-equiv.https.html` | 0 / 1 |  |
| `html/cross-origin-opener-policy/popup-redirect-cache.https.html` | 0 / 9 | harnais : Timeout while running cleanup for test named "same-origin document opening popup redirect from https://{{host}}:{{ports[ |
| `html/cross-origin-opener-policy/popup-redirect-same-origin-allow-popups.https.html` | 0 / 2 | harnais : Timeout while running cleanup for test named "Same origin popup redirects to same-origin with same-origin-allow-popups". |
| `html/cross-origin-opener-policy/popup-same-origin-allow-popups-with-cross-origin.https.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/popup-same-origin-allow-popups-with-same-origin.https.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/popup-same-origin-allow-popups-with-same-site.https.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/popup-same-origin-non-initial-about-blank.https.html` | 0 / 1 | harnais : Test named 'Navigating a popup to about:blank' specified 1 'cleanup' function, and 1 failed. |
| `html/cross-origin-opener-policy/popup-same-origin-with-cross-origin.https.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/popup-same-origin-with-same-origin.https.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/popup-same-origin-with-same-site.https.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/popup-unsafe-none-with-cross-origin.https.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/popup-unsafe-none-with-same-origin.https.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/popup-unsafe-none-with-same-site.https.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/popup-unspecified-with-cross-origin.https.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/popup-unspecified-with-same-origin.https.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/popup-unspecified-with-same-site.https.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/popup-with-structured-header.https.html` | 0 / 3 |  |
| `html/cross-origin-opener-policy/reporting/access-reporting/access-from-coop-page-to-openee_coop-ro.https.html` | 0 / 2 | harnais : Timeout while running cleanup for test named "access-from-coop-page-to-openee, same-origin". |
| `html/cross-origin-opener-policy/reporting/access-reporting/access-from-coop-page-to-openee_coop-ro_cross-origin.https.html` | 0 / 2 | harnais : Timeout while running cleanup for test named "access-from-coop-page-to-openee, cross-origin". |
| `html/cross-origin-opener-policy/reporting/access-reporting/access-from-coop-page-to-opener_coop-ro.https.html` | 0 / 2 | harnais : Timeout while running cleanup for test named "access-from-coop-page-to-opener, same-origin". |
| `html/cross-origin-opener-policy/reporting/access-reporting/access-from-coop-page-to-opener_coop-ro_cross-origin.https.html` | 0 / 2 | harnais : Timeout while running cleanup for test named "access-from-coop-page-to-opener, cross-origin". |
| `html/cross-origin-opener-policy/reporting/access-reporting/access-from-coop-page-to-other_coop-ro.https.html` | 0 / 1 |  |
| `html/cross-origin-opener-policy/reporting/access-reporting/access-from-coop-page-to-other_coop-ro_cross-origin.https.html` | 0 / 1 |  |
| `html/cross-origin-opener-policy/reporting/access-reporting/access-to-coop-page-from-openee_coop-ro.https.html` | 0 / 2 | harnais : Timeout while running cleanup for test named "access-to-coop-page-from-openee, same-origin". |
| `html/cross-origin-opener-policy/reporting/access-reporting/access-to-coop-page-from-openee_coop-ro_cross-origin.https.html` | 0 / 2 | harnais : Timeout while running cleanup for test named "access-to-coop-page-from-openee, cross-origin". |
| `html/cross-origin-opener-policy/reporting/access-reporting/access-to-coop-page-from-opener_coop-ro.https.html` | 0 / 2 | harnais : Timeout while running cleanup for test named "access-to-coop-page-from-opener, same-origin". |
| `html/cross-origin-opener-policy/reporting/access-reporting/access-to-coop-page-from-opener_coop-ro_cross-origin.https.html` | 0 / 2 | harnais : Timeout while running cleanup for test named "access-to-coop-page-from-opener, cross-origin". |
| `html/cross-origin-opener-policy/reporting/access-reporting/access-to-coop-page-from-other_coop-ro.https.html` | 0 / 1 |  |
| `html/cross-origin-opener-policy/reporting/access-reporting/access-to-coop-page-from-other_coop-ro_cross-origin.https.html` | 0 / 1 |  |
| `html/cross-origin-opener-policy/reporting/access-reporting/property-blur.https.html` | 0 / 2 |  |
| `html/cross-origin-opener-policy/reporting/access-reporting/property-close.https.html` | 0 / 2 |  |
| `html/cross-origin-opener-policy/reporting/access-reporting/property-closed.https.html` | 0 / 2 |  |
| `html/cross-origin-opener-policy/reporting/access-reporting/property-focus.https.html` | 0 / 2 |  |
| `html/cross-origin-opener-policy/reporting/access-reporting/property-frames.https.html` | 0 / 2 |  |
| `html/cross-origin-opener-policy/reporting/access-reporting/property-indexed-getter.https.html` | 0 / 6 | harnais : Timeout while running cleanup for test named "cross-origin > w => w[0]". |
| `html/cross-origin-opener-policy/reporting/access-reporting/property-length.https.html` | 0 / 2 |  |
| `html/cross-origin-opener-policy/reporting/access-reporting/property-location-get.https.html` | 0 / 2 |  |
| `html/cross-origin-opener-policy/reporting/access-reporting/property-location-set.https.html` | 0 / 2 |  |
| `html/cross-origin-opener-policy/reporting/access-reporting/property-named-getter.https.html` | 0 / 8 | harnais : Timeout while running cleanup for test named "same-site > w => w["iframeName"]". |
| `html/cross-origin-opener-policy/reporting/access-reporting/property-opener-get.https.html` | 0 / 2 |  |
| `html/cross-origin-opener-policy/reporting/access-reporting/property-opener-set.https.html` | 0 / 2 |  |
| `html/cross-origin-opener-policy/reporting/access-reporting/property-postmessage-1.https.html` | 0 / 2 |  |
| `html/cross-origin-opener-policy/reporting/access-reporting/property-postmessage-2.https.html` | 0 / 2 |  |
| `html/cross-origin-opener-policy/reporting/access-reporting/property-self.https.html` | 0 / 2 |  |
| `html/cross-origin-opener-policy/reporting/access-reporting/property-top.https.html` | 0 / 2 |  |
| `html/cross-origin-opener-policy/reporting/access-reporting/property-window.https.html` | 0 / 2 |  |
| `html/cross-origin-opener-policy/reporting/access-reporting/report-to-both_coop-ro.https.html` | 0 / 2 |  |
| `html/cross-origin-opener-policy/reporting/access-reporting/reporting-observer-no-endpoint.tentative.html` | 0 / 2 | harnais : Timeout while running cleanup for test named "ReportingObserver is notified without a reporting endpoint". |
| `html/cross-origin-opener-policy/reporting/access-reporting/reporting-observer.html` | 0 / 4 | harnais : Timeout while running cleanup for test named "Opener COOP". |
| `html/cross-origin-opener-policy/reporting/document-reporting/report-only-four-reports.https.html` | 0 / 0 | boucle sans fin (processus tué) |
| `html/cross-origin-opener-policy/reporting/document-reporting/report-to-both_coop-ro.https.html` | 0 / 2 |  |
| `html/cross-origin-opener-policy/reporting/document-reporting/reporting-redirect-with-same-origin-allow-popups.https.html` | 0 / 2 | harnais : Timeout while running cleanup for test named "Same origin openee redirected to same-origin with same-origin-allow-popups |
| `html/cross-origin-opener-policy/reporting/document-reporting/reporting-redirect-with-unsafe-none.https.html` | 0 / 2 | harnais : Timeout while running cleanup for test named "Same origin openee redirected to same-origin with unsafe-none". |
| `html/cross-origin-opener-policy/reporting/navigation-reporting/report-only-four-reports.https.html` | 0 / 0 | harnais : Unhandled rejection: assert_equals: reports cleared expected 200 but got 404 |
| `html/cross-origin-opener-policy/reporting/navigation-reporting/report-only-from-unsafe-none.https.html` | 0 / 0 | harnais : Unhandled rejection: assert_equals: reports cleared expected 200 but got 404 |
| `html/cross-origin-opener-policy/reporting/navigation-reporting/report-only-same-origin-report-to.https.html` | 0 / 0 | harnais : Unhandled rejection: assert_equals: reports cleared expected 200 but got 404 |
| `html/cross-origin-opener-policy/reporting/navigation-reporting/report-only-same-origin-with-coep-report-only.https.html` | 0 / 0 | harnais : Unhandled rejection: assert_equals: reports cleared expected 200 but got 404 |
| `html/cross-origin-opener-policy/reporting/navigation-reporting/report-only-same-origin-with-coep.https.html` | 0 / 0 | harnais : Unhandled rejection: assert_equals: reports cleared expected 200 but got 404 |
| `html/cross-origin-opener-policy/reporting/navigation-reporting/report-only-same-origin.https.html` | 0 / 0 | harnais : Unhandled rejection: assert_equals: reports cleared expected 200 but got 404 |
| `html/cross-origin-opener-policy/reporting/navigation-reporting/reporting-coop-navigated-opener.https.html` | 0 / 1 | harnais : Timeout while running cleanup for test named "navigation-report-from-opener-navigation". |
| `html/cross-origin-opener-policy/reporting/navigation-reporting/reporting-coop-navigated-popup.https.html` | 0 / 2 | harnais : Timeout while running cleanup for test named "Open a popup to a document without COOP, then navigate it to a document wi |
| `html/cross-origin-opener-policy/reporting/navigation-reporting/reporting-popup-report-to-must-be-string.https.html` | 0 / 0 | harnais : Unhandled rejection: assert_equals: reports cleared expected 200 but got 404 |
| `html/cross-origin-opener-policy/reporting/navigation-reporting/reporting-popup-same-origin-allow-popups-report-to.https.html` | 0 / 0 | harnais : Unhandled rejection: assert_equals: reports cleared expected 200 but got 404 |
| `html/cross-origin-opener-policy/reporting/navigation-reporting/reporting-popup-same-origin-coep-report-to.https.html` | 0 / 0 | harnais : Unhandled rejection: assert_equals: reports cleared expected 200 but got 404 |
| `html/cross-origin-opener-policy/reporting/navigation-reporting/reporting-popup-same-origin-report-to.https.html` | 0 / 0 | harnais : Unhandled rejection: assert_equals: reports cleared expected 200 but got 404 |
| `html/cross-origin-opener-policy/reporting/navigation-reporting/reporting-popup-same-origin.https.html` | 0 / 0 | harnais : Unhandled rejection: assert_equals: reports cleared expected 200 but got 404 |
| `html/cross-origin-opener-policy/reporting/navigation-reporting/reporting-popup-unsafe-none-report-to.https.html` | 0 / 0 | harnais : Unhandled rejection: assert_equals: reports cleared expected 200 but got 404 |
| `html/cross-origin-opener-policy/reporting/navigation-reporting/reporting-redirect-with-same-origin-allow-popups.https.html` | 0 / 2 | harnais : Timeout while running cleanup for test named "Same origin openee redirected to same-origin with same-origin-allow-popups |
| `html/cross-origin-opener-policy/reporting/tentative/access-to-noopener-page-from-no-coop-ro.https.html` | 0 / 2 | harnais : Timeout while running cleanup for test named "access-to-coop-page-from-opener, noopener-allow-popups". |
| `html/cross-origin-opener-policy/resource-popup.https.html` | 0 / 12 | harnais : délai du harnais dépassé |
| `html/cross-origin-opener-policy/tentative/noopener/coop-noopener-allow-popups-restrict-properties.https.html` | 0 / 2 | harnais : Timeout while running cleanup for test named "     Cross-Origin-Opener-Policy: noopener-allow-popups means that the open |
| `html/document-isolation-policy/agent-cluster.tentative.https.html` | 0 / 25 | harnais : Test named 'Navigating the initial about:blank document of a popup to a same-origin document without a policy reuses its |
| `html/document-isolation-policy/back-forward-cache-frames.tentative.https.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/document-isolation-policy/back-forward-cache-same-origin-frames.tentative.https.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/document-isolation-policy/back-forward-cache.tentative.https.html` | 0 / 10 | harnais : délai du harnais dépassé |
| `html/document-isolation-policy/blob-document-inherits-policy.tentative.https.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/document-isolation-policy/broadcast-channel.tentative.https.html` | 0 / 3 | harnais : Test named 'A message without shared memory posted on a BroadcastChannel by a document with a policy reaches same-origin |
| `html/document-isolation-policy/credentialless-cache-storage-from-credentialless.https.tentative.window.js` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/document-isolation-policy/credentialless-cache-storage-from-none.https.tentative.window.js` | 0 / 7 | harnais : délai du harnais dépassé |
| `html/document-isolation-policy/credentialless-cache-storage-from-require-corp.https.tentative.window.js` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/document-isolation-policy/credentialless-cache.tentative.window.js` | 0 / 1 | harnais : Error: assert_equals: expected "done" but got "import json\nfrom wptserve.utils import isomorphic_decode\n\n# A server u |
| `html/document-isolation-policy/credentialless-cross-origin-isolated.tentative.window.js` | 0 / 4 |  |
| `html/document-isolation-policy/credentialless-dedicated-worker.https.tentative.window.js` | 0 / 1 |  |
| `html/document-isolation-policy/credentialless-fetch.https.tentative.window.js` | 0 / 1 |  |
| `html/document-isolation-policy/credentialless-link.https.tentative.window.js` | 0 / 1 |  |
| `html/document-isolation-policy/credentialless-redirect.tentative.window.js` | 0 / 1 |  |
| `html/document-isolation-policy/credentialless-script.https.tentative.window.js` | 0 / 1 |  |
| `html/document-isolation-policy/credentialless-service-worker.https.tentative.window.js` | 0 / 1 |  |
| `html/document-isolation-policy/credentialless-shared-worker.https.tentative.window.js` | 0 / 1 |  |
| `html/document-isolation-policy/credentialless-video.https.tentative.window.js` | 0 / 6 | harnais : Error: assert_equals: expected "done" but got "import json\nfrom wptserve.utils import isomorphic_decode\n\n# A server u |
| `html/document-isolation-policy/credentialless-websocket.https.tentative.window.js` | 0 / 1 |  |
| `html/document-isolation-policy/cross-origin-frame.tentative.https.html` | 0 / 2 | harnais : Test named 'A document with isolate-and-require-corp can embed a cross-origin frame without Cross-Origin-Resource-Policy |
| `html/document-isolation-policy/cross-origin-isolated-permissions-policy-nested.tentative.https.html` | 0 / 2 |  |
| `html/document-isolation-policy/cross-origin-isolated-permissions-policy.tentative.https.html` | 0 / 3 |  |
| `html/document-isolation-policy/cross-origin-isolated.tentative.https.html` | 0 / 12 | harnais : Test named 'A popup with isolate-and-require-corp is cross-origin isolated and exposes SharedArrayBuffer' specified 1 'c |
| `html/document-isolation-policy/dedicated-worker.tentative.https.html` | 0 / 9 | harnais : Test named 'A dedicated worker of a popup with isolate-and-require-corp is cross-origin isolated and exposes SharedArray |
| `html/document-isolation-policy/header-parsing.tentative.https.html` | 8 / 27 |  |
| `html/document-isolation-policy/history-traversal-frames.tentative.https.html` | 0 / 5 | harnais : délai du harnais dépassé |
| `html/document-isolation-policy/isolate-and-require-corp-load-from-cache-storage.tentative.https.html` | 0 / 29 |  |
| `html/document-isolation-policy/isolate-and-require-corp.tentative.https.html` | 1 / 2 |  |
| `html/document-isolation-policy/location-replace-keeps-entry.tentative.https.html` | 0 / 2 | harnais : Test named 'A popup's document replaced by a same-origin one with a policy keeps the session history entry' specified 1  |
| `html/document-isolation-policy/message-channel.tentative.https.html` | 0 / 3 | harnais : Test named 'A message without shared memory posted over a MessageChannel by a document with a policy reaches its same-or |
| `html/document-isolation-policy/navigate-across-policies.tentative.https.html` | 0 / 4 | harnais : Test named 'navigation.navigate() from a document without a policy to one with does not abort the navigation' specified  |
| `html/document-isolation-policy/no-secure-context-https-iframe.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/document-isolation-policy/no-secure-context.tentative.html` | 0 / 2 |  |
| `html/document-isolation-policy/origin-agent-cluster.tentative.https.html` | 0 / 3 | harnais : Timeout while running cleanup for test named "After a document with a policy, a same-origin document without one that se |
| `html/document-isolation-policy/partial-deployment.tentative.https.html` | 0 / 8 | harnais : Test named 'A document without a policy and its same-origin frame with one can post messages to each other' specified 1  |
| `html/document-isolation-policy/reporting-cache-storage-corp.tentative.https.html` | 0 / 9 |  |
| `html/document-isolation-policy/reporting-cached-and-service-worker-responses.tentative.https.html` | 0 / 10 | harnais : délai du harnais dépassé |
| `html/document-isolation-policy/reporting-dedicated-worker.tentative.https.html` | 0 / 2 |  |
| `html/document-isolation-policy/reporting-subresource-corp.tentative.https.html` | 0 / 12 |  |
| `html/document-isolation-policy/reporting.tentative.https.html` | 0 / 5 |  |
| `html/document-isolation-policy/service-worker-coep-credentialless-proxy.https.tentative.window.js` | 0 / 1 |  |
| `html/document-isolation-policy/service-worker-coep-none-proxy.https.tentative.window.js` | 0 / 1 |  |
| `html/document-isolation-policy/shared-array-buffer.tentative.https.html` | 0 / 3 | harnais : Test named 'A SharedArrayBuffer posted by a popup with isolate-and-require-corp to its same-origin frame with isolate-an |
| `html/document-isolation-policy/shared-workers.tentative.https.html` | 0 / 9 |  |
| `html/document-isolation-policy/window-reuse.tentative.https.html` | 0 / 6 | harnais : Test named 'A frame's initial about:blank document created by a document with a policy that navigates to a same-origin d |
| `html/dom/access-key-label.html` | 1 / 2 |  |
| `html/dom/aria-attribute-reflection-enumerated.tentative.html` | 267 / 1761 |  |
| `html/dom/aria-attribute-reflection.html` | 0 / 41 |  |
| `html/dom/aria-attribute-reflection.tentative.html` | 0 / 3 |  |
| `html/dom/aria-element-reflection-disconnected.html` | 0 / 2 |  |
| `html/dom/aria-element-reflection-labelledby.html` | 0 / 4 |  |
| `html/dom/aria-element-reflection.html` | 5 / 27 |  |
| `html/dom/documents/dom-tree-accessors/Document.body.html` | 9 / 26 |  |
| `html/dom/documents/dom-tree-accessors/Document.currentScript.html` | 7 / 18 | harnais : délai du harnais dépassé |
| `html/dom/documents/dom-tree-accessors/document.embeds-document.plugins-01.html` | 0 / 5 |  |
| `html/dom/documents/dom-tree-accessors/document.forms.html` | 2 / 6 |  |
| `html/dom/documents/dom-tree-accessors/document.getElementsByName/document.getElementsByName-liveness.html` | 0 / 1 |  |
| `html/dom/documents/dom-tree-accessors/document.getElementsByName/document.getElementsByName-namespace-xhtml.xhtml` | 0 / 1 |  |
| `html/dom/documents/dom-tree-accessors/document.getElementsByName/document.getElementsByName-namespace.html` | 0 / 1 |  |
| `html/dom/documents/dom-tree-accessors/document.images.html` | 5 / 11 |  |
| `html/dom/documents/dom-tree-accessors/document.links.html` | 0 / 1 |  |
| `html/dom/documents/dom-tree-accessors/document.scripts.html` | 0 / 1 |  |
| `html/dom/documents/dom-tree-accessors/document.title-05.html` | 1 / 21 |  |
| `html/dom/documents/dom-tree-accessors/document.title-06.html` | 0 / 1 |  |
| `html/dom/documents/dom-tree-accessors/document.title-09.html` | 1 / 5 |  |
| `html/dom/documents/dom-tree-accessors/document.title-not-in-html-svg.html` | 1 / 2 |  |
| `html/dom/documents/dom-tree-accessors/nameditem-02.html` | 2 / 12 |  |
| `html/dom/documents/dom-tree-accessors/nameditem-03.html` | 0 / 1 |  |
| `html/dom/documents/dom-tree-accessors/nameditem-04.html` | 3 / 12 |  |
| `html/dom/documents/dom-tree-accessors/nameditem-05.html` | 3 / 12 |  |
| `html/dom/documents/dom-tree-accessors/nameditem-06.html` | 1 / 9 |  |
| `html/dom/documents/dom-tree-accessors/nameditem-07.html` | 0 / 14 |  |
| `html/dom/documents/dom-tree-accessors/nameditem-08.html` | 0 / 2 |  |
| `html/dom/documents/dom-tree-accessors/nameditem-img-name-added-cached-list.html` | 0 / 1 |  |
| `html/dom/documents/dom-tree-accessors/nameditem-img-name-dynamic.html` | 0 / 1 |  |
| `html/dom/documents/dom-tree-accessors/nameditem-img-name-removed.html` | 0 / 1 |  |
| `html/dom/documents/dom-tree-accessors/nameditem-names.html` | 8 / 21 |  |
| `html/dom/documents/dom-tree-accessors/nameditem-no-shadowing.tentative.html` | 7 / 8 |  |
| `html/dom/documents/dom-tree-accessors/nameditem-setattribute-attr-mutation.html` | 2 / 4 |  |
| `html/dom/documents/dom-tree-accessors/nameditem-supported-property-names-order.html` | 0 / 1 |  |
| `html/dom/documents/dom-tree-accessors/nameditem-tree-order.html` | 0 / 1 |  |
| `html/dom/documents/dom-tree-accessors/nameditem-tree-transitions.html` | 1 / 2 |  |
| `html/dom/documents/resource-metadata-management/document-compatmode-03.html` | 0 / 1 |  |
| `html/dom/documents/resource-metadata-management/document-cookie.html` | 2 / 5 |  |
| `html/dom/documents/resource-metadata-management/document-lastModified-01.html` | 0 / 3 |  |
| `html/dom/documents/resource-metadata-management/document-lastModified.html` | 0 / 1 |  |
| `html/dom/documents/resource-metadata-management/document-readyState.html` | 5 / 7 | harnais : délai du harnais dépassé |
| `html/dom/elements/elements-in-the-dom/click-in-progress-flag.html` | 0 / 1 |  |
| `html/dom/elements/global-attributes/cdata-dir_auto.html` | 1 / 4 |  |
| `html/dom/elements/global-attributes/custom-attrs.html` | 0 / 1 |  |
| `html/dom/elements/global-attributes/dataset-binding.window.js` | 0 / 4 |  |
| `html/dom/elements/global-attributes/dataset-delete.html` | 8 / 9 |  |
| `html/dom/elements/global-attributes/dataset-get-invalid-names.html` | 20 / 28 |  |
| `html/dom/elements/global-attributes/dataset-prototype.html` | 1 / 2 |  |
| `html/dom/elements/global-attributes/dataset-set.html` | 9 / 11 |  |
| `html/dom/elements/global-attributes/dataset.html` | 4 / 8 |  |
| `html/dom/elements/global-attributes/dir-assorted.window.js` | 27 / 41 |  |
| `html/dom/elements/global-attributes/dir-auto-div-append-child.html` | 0 / 1 |  |
| `html/dom/elements/global-attributes/dir-auto-dynamic-changes.window.js` | 2 / 21 |  |
| `html/dom/elements/global-attributes/dir-auto-form-associated.window.js` | 11 / 22 |  |
| `html/dom/elements/global-attributes/dir-bdi-script.html` | 0 / 1 |  |
| `html/dom/elements/global-attributes/dir-slots-directionality.html` | 5 / 17 |  |
| `html/dom/elements/global-attributes/document-dir.html` | 0 / 3 |  |
| `html/dom/elements/global-attributes/lang-attribute-shadow.window.js` | 4 / 6 |  |
| `html/dom/elements/global-attributes/lang-attribute.window.js` | 0 / 1 |  |
| `html/dom/elements/global-attributes/the-lang-attribute-003.html` | 0 / 1 |  |
| `html/dom/elements/global-attributes/the-lang-attribute-004.html` | 0 / 1 |  |
| `html/dom/elements/global-attributes/the-lang-attribute-006.html` | 0 / 1 |  |
| `html/dom/elements/global-attributes/the-lang-attribute-011.html` | 0 / 1 |  |
| `html/dom/elements/global-attributes/the-translate-attribute-010.html` | 0 / 1 |  |
| `html/dom/elements/global-attributes/translate-enumerated-ascii-case-insensitive.html` | 0 / 1 |  |
| `html/dom/elements/global-attributes/translate-non-html-translation-mode.html` | 2 / 3 |  |
| `html/dom/elements/images/bypass-cache-revalidation.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/dom/elements/name-content-attribute-and-property.html` | 17 / 141 |  |
| `html/dom/elements/the-innertext-and-outertext-properties/dynamic-getter.html` | 3 / 9 |  |
| `html/dom/elements/the-innertext-and-outertext-properties/getter.html` | 123 / 276 |  |
| `html/dom/elements/the-innertext-and-outertext-properties/innertext-setter.html` | 88 / 126 |  |
| `html/dom/elements/the-innertext-and-outertext-properties/innertext-whitespace-pre-line.html` | 1 / 2 |  |
| `html/dom/elements/the-innertext-and-outertext-properties/innertext-with-white-spaces.html` | 34 / 79 |  |
| `html/dom/elements/the-innertext-and-outertext-properties/outertext-setter.html` | 2 / 43 |  |
| `html/dom/historical.html` | 7 / 13 |  |
| `html/dom/idlharness.https.html` | 0 / 1 |  |
| `html/dom/partial-updates/tentative/fragment/empty-for-streaming.html` | 0 / 2 |  |
| `html/dom/partial-updates/tentative/fragment/sanitize-default.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/dom/partial-updates/tentative/fragment/sanitize-nested.html` | 0 / 3 |  |
| `html/dom/partial-updates/tentative/fragment/sanitize-regular.html` | 0 / 1 |  |
| `html/dom/partial-updates/tentative/fragment/sanitize-unsafe.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/dom/partial-updates/tentative/fragment/src-csp-cors.sub.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/dom/partial-updates/tentative/fragment/src-nonce.sub.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/dom/partial-updates/tentative/fragment/src-parser-blocking.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/dom/partial-updates/tentative/fragment/src-referrerpolicy.sub.html` | 0 / 2 |  |
| `html/dom/partial-updates/tentative/fragment/src-sri.sub.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/dom/partial-updates/tentative/fragment/src-streaming.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/dom/partial-updates/tentative/fragment/streaming-target-marker-removed.html` | 0 / 1 |  |
| `html/dom/partial-updates/tentative/template-for-case-sensitive-dom-inserted.html` | 0 / 1 |  |
| `html/dom/partial-updates/tentative/template-for-case-sensitive.html` | 0 / 1 |  |
| `html/dom/reflection-embedded.html` | 5163 / 9429 |  |
| `html/dom/reflection-forms-weekmonth.html` | 982 / 1618 |  |
| `html/dom/reflection-forms.html` | 5827 / 8817 |  |
| `html/dom/reflection-grouping.html` | 3857 / 5904 |  |
| `html/dom/reflection-metadata.html` | 2128 / 3344 |  |
| `html/dom/reflection-misc.html` | 3483 / 5345 |  |
| `html/dom/reflection-obsolete.html` | 1418 / 2816 |  |
| `html/dom/reflection-sections.html` | 4035 / 6189 |  |
| `html/dom/reflection-tabular.html` | 3124 / 6506 |  |
| `html/dom/reflection-text.html` | 0 / 0 | le harnais n'a pas terminé |
| `html/dom/render-blocking/blocking-idl-attr.html` | 0 / 6 |  |
| `html/dom/render-blocking/element-render-blocking-002.html` | 0 / 1 |  |
| `html/dom/render-blocking/element-render-blocking-004.html` | 0 / 1 |  |
| `html/dom/render-blocking/element-render-blocking-005.html` | 0 / 1 |  |
| `html/dom/render-blocking/element-render-blocking-007.html` | 0 / 1 |  |
| `html/dom/render-blocking/element-render-blocking-009.html` | 0 / 1 |  |
| `html/dom/render-blocking/element-render-blocking-010.html` | 0 / 1 |  |
| `html/dom/render-blocking/element-render-blocking-013.html` | 0 / 1 |  |
| `html/dom/render-blocking/element-render-blocking-014.html` | 0 / 1 |  |
| `html/dom/render-blocking/element-render-blocking-015.html` | 0 / 1 |  |
| `html/dom/render-blocking/element-render-blocking-016.html` | 0 / 1 |  |
| `html/dom/render-blocking/element-render-blocking-017.html` | 0 / 1 |  |
| `html/dom/render-blocking/element-render-blocking-018.html` | 0 / 1 |  |
| `html/dom/render-blocking/element-render-blocking-019.html` | 0 / 1 |  |
| `html/dom/render-blocking/element-render-blocking-020.html` | 0 / 1 |  |
| `html/dom/render-blocking/element-render-blocking-021.html` | 0 / 1 |  |
| `html/dom/render-blocking/element-render-blocking-022.html` | 0 / 1 |  |
| `html/dom/render-blocking/element-render-blocking-023.html` | 0 / 1 |  |
| `html/dom/render-blocking/element-render-blocking-025.html` | 0 / 1 |  |
| `html/dom/render-blocking/element-render-blocking-026.html` | 0 / 1 |  |
| `html/dom/render-blocking/element-render-blocking-028.html` | 0 / 1 |  |
| `html/dom/render-blocking/element-render-blocking-029.html` | 0 / 1 |  |
| `html/dom/render-blocking/element-render-blocking-030.html` | 0 / 1 |  |
| `html/dom/render-blocking/element-render-blocking-031.html` | 0 / 1 |  |
| `html/dom/render-blocking/element-render-blocking-032.html` | 0 / 1 |  |
| `html/dom/render-blocking/element-render-blocking-033.html` | 0 / 1 |  |
| `html/dom/render-blocking/element-render-blocking-034.html` | 0 / 1 |  |
| `html/dom/render-blocking/element-render-blocking-035.html` | 0 / 1 |  |
| `html/dom/render-blocking/element-render-blocking-036.html` | 0 / 1 |  |
| `html/dom/render-blocking/element-render-blocking-037.html` | 0 / 1 |  |
| `html/dom/render-blocking/element-render-blocking-038.html` | 0 / 1 |  |
| `html/dom/render-blocking/element-render-blocking-040.html` | 0 / 1 |  |
| `html/dom/render-blocking/element-render-blocking-041.html` | 0 / 1 |  |
| `html/dom/render-blocking/element-render-blocking-042.html` | 0 / 1 |  |
| `html/dom/render-blocking/element-render-blocking-form-stack-removal.html` | 0 / 1 |  |
| `html/dom/render-blocking/element-render-blocking-insert-named-anchor.html` | 0 / 1 |  |
| `html/dom/render-blocking/expect-parser-associated-form.html` | 0 / 1 |  |
| `html/dom/render-blocking/link-rel-expect-supported.html` | 0 / 1 |  |
| `html/dom/render-blocking/non-render-blocking-scripts.optional.html` | 4 / 14 | harnais : délai du harnais dépassé |
| `html/dom/render-blocking/parser-blocking-script.html` | 1 / 2 |  |
| `html/dom/render-blocking/parser-inserted-async-inline-module-with-import.html` | 0 / 1 |  |
| `html/dom/render-blocking/parser-inserted-async-script.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/dom/render-blocking/parser-inserted-defer-script.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/dom/render-blocking/parser-inserted-inline-module-with-import.html` | 0 / 1 |  |
| `html/dom/render-blocking/parser-inserted-module-script.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/dom/render-blocking/parser-inserted-style-element.html` | 0 / 2 |  |
| `html/dom/render-blocking/parser-inserted-stylesheet-link.html` | 1 / 2 |  |
| `html/dom/render-blocking/remove-attr-script-keeps-blocking.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/dom/render-blocking/remove-attr-style-keeps-blocking.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/dom/render-blocking/remove-attr-stylesheet-link-keeps-blocking.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/dom/render-blocking/remove-attr-unblocks-rendering.optional.html` | 6 / 16 | harnais : délai du harnais dépassé |
| `html/dom/render-blocking/remove-element-unblocks-rendering.optional.html` | 6 / 8 |  |
| `html/dom/render-blocking/remove-pending-async-render-blocking-script.html` | 0 / 1 |  |
| `html/dom/render-blocking/script-inserted-inline-module-with-import.html` | 0 / 1 |  |
| `html/dom/render-blocking/script-inserted-module-script.html` | 1 / 2 |  |
| `html/dom/render-blocking/script-inserted-script.html` | 1 / 2 |  |
| `html/dom/render-blocking/script-inserted-style-element.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/dom/render-blocking/script-inserted-stylesheet-link.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/dom/render-blocking/stylesheet-link-href-change-keeps-blocking.html` | 0 / 1 |  |
| `html/dom/self-origin.any.js` | 0 / 1 |  |
| `html/dom/self-origin.sub.html` | 1 / 11 | harnais : délai du harnais dépassé |
| `html/dom/usvstring-reflection.https.html` | 10 / 19 |  |
| `html/editing/activation/click-bail-on-disabled.html` | 0 / 4 |  |
| `html/editing/activation/click.html` | 0 / 1 |  |
| `html/editing/activation/click_checkbox.html` | 0 / 1 |  |
| `html/editing/dnd/canvas/001.html` | 0 / 1 |  |
| `html/editing/dnd/canvas/002.html` | 0 / 1 |  |
| `html/editing/dnd/canvas/003.html` | 0 / 1 |  |
| `html/editing/dnd/canvas/005.html` | 0 / 1 |  |
| `html/editing/dnd/canvas/007.html` | 0 / 1 |  |
| `html/editing/dnd/datastore/datatransfer-constructor-001.html` | 0 / 1 |  |
| `html/editing/dnd/datastore/datatransfer-getdata-url.html` | 0 / 11 |  |
| `html/editing/dnd/datastore/datatransfer-text-plain-prefix-format.html` | 0 / 10 |  |
| `html/editing/dnd/datastore/datatransfer-types.html` | 0 / 5 |  |
| `html/editing/dnd/datastore/datatransferitem-disabled-mode.html` | 0 / 5 |  |
| `html/editing/dnd/datastore/datatransferitemlist-clear-read-only.html` | 0 / 1 |  |
| `html/editing/dnd/datastore/datatransferitemlist-clear.html` | 0 / 2 |  |
| `html/editing/dnd/datastore/datatransferitemlist-dragstart-after-dispatch.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/editing/dnd/datastore/datatransferitemlist-indexed-getter.html` | 0 / 6 |  |
| `html/editing/dnd/datastore/datatransferitemlist-remove-file.html` | 0 / 3 |  |
| `html/editing/dnd/datastore/datatransferitemlist-remove.html` | 0 / 2 |  |
| `html/editing/dnd/dom/draggable.html` | 15 / 27 |  |
| `html/editing/dnd/dom/events.html` | 0 / 7 |  |
| `html/editing/dnd/dom/specials.html` | 0 / 21 |  |
| `html/editing/dnd/drop/001.xhtml` | 0 / 3 |  |
| `html/editing/dnd/drop/002.xhtml` | 0 / 3 |  |
| `html/editing/dnd/drop/003.xhtml` | 0 / 3 |  |
| `html/editing/dnd/drop/004.xhtml` | 0 / 3 |  |
| `html/editing/dnd/drop/005.xhtml` | 0 / 3 |  |
| `html/editing/dnd/drop/006.xhtml` | 0 / 3 |  |
| `html/editing/dnd/drop/029.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/editing/dnd/drop/030.html` | 0 / 1 |  |
| `html/editing/dnd/events/dragenter-dragleave-related-target.html` | 0 / 1 |  |
| `html/editing/dnd/events/pointer-suppression.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/editing/dnd/images/001.html` | 0 / 1 |  |
| `html/editing/dnd/images/002.html` | 0 / 1 |  |
| `html/editing/dnd/images/004.html` | 0 / 1 |  |
| `html/editing/dnd/images/005.html` | 0 / 1 |  |
| `html/editing/dnd/images/006.html` | 0 / 1 |  |
| `html/editing/dnd/images/007.html` | 0 / 1 |  |
| `html/editing/dnd/images/008.html` | 0 / 1 |  |
| `html/editing/dnd/images/009.html` | 0 / 1 |  |
| `html/editing/dnd/images/010.html` | 0 / 1 |  |
| `html/editing/dnd/images/011.xhtml` | 0 / 1 |  |
| `html/editing/dnd/images/012.xhtml` | 0 / 1 |  |
| `html/editing/dnd/images/013.xhtml` | 0 / 1 |  |
| `html/editing/dnd/images/014.xhtml` | 0 / 1 |  |
| `html/editing/dnd/images/023.html` | 0 / 0 | harnais : Unhandled rejection: impossible de lire la propriété « document » de undefined (valeur de .contentWindow) |
| `html/editing/dnd/images/024.html` | 0 / 0 | harnais : Unhandled rejection: impossible de lire la propriété « document » de undefined (valeur de .contentWindow) |
| `html/editing/dnd/interactiveelements/button_inside_draggable_element.html` | 0 / 2 |  |
| `html/editing/dnd/interactiveelements/draggable_button.html` | 0 / 2 |  |
| `html/editing/dnd/interactiveelements/draggable_input_button.html` | 0 / 2 |  |
| `html/editing/dnd/interactiveelements/draggable_input_image.html` | 1 / 3 | harnais : délai du harnais dépassé |
| `html/editing/dnd/interactiveelements/draggable_input_reset.html` | 0 / 2 |  |
| `html/editing/dnd/interactiveelements/draggable_input_submit.html` | 0 / 2 |  |
| `html/editing/dnd/interactiveelements/draggable_select.html` | 1 / 2 | harnais : délai du harnais dépassé |
| `html/editing/dnd/interactiveelements/input_button_inside_draggable_element.html` | 0 / 2 |  |
| `html/editing/dnd/interactiveelements/input_image_inside_draggable_element.html` | 1 / 3 | harnais : délai du harnais dépassé |
| `html/editing/dnd/interactiveelements/input_reset_inside_draggable_element.html` | 0 / 2 |  |
| `html/editing/dnd/interactiveelements/input_submit_inside_draggable_element.html` | 0 / 2 |  |
| `html/editing/dnd/interactiveelements/select_inside_draggable_element.html` | 1 / 2 | harnais : délai du harnais dépassé |
| `html/editing/dnd/js-file-fallback-text-only.html` | 0 / 1 |  |
| `html/editing/dnd/js-file-image-drag.html` | 0 / 1 |  |
| `html/editing/dnd/layoutshift/001.html` | 0 / 1 |  |
| `html/editing/dnd/platform/mousedown-drag-select-input-display-change.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/editing/dnd/platform/mousedown-move-inner-when-dragging-1.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/editing/dnd/platform/mousedown-move-inner-when-dragging-2.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/editing/dnd/platform/pointerdown-add-display-none.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/editing/dnd/platform/pointerdown-add-overflow-hidden.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/editing/dnd/reload/001.html` | 0 / 1 |  |
| `html/editing/dnd/reload/002.html` | 0 / 1 |  |
| `html/editing/dnd/reload/003.html` | 0 / 1 |  |
| `html/editing/dnd/reload/004.html` | 0 / 1 |  |
| `html/editing/dnd/reload/005.html` | 0 / 1 |  |
| `html/editing/dnd/reload/006.html` | 0 / 1 |  |
| `html/editing/dnd/reload/007.html` | 0 / 1 |  |
| `html/editing/dnd/reload/008.html` | 0 / 1 |  |
| `html/editing/dnd/remove/001.html` | 0 / 1 |  |
| `html/editing/dnd/remove/002.html` | 0 / 1 |  |
| `html/editing/dnd/remove/003.html` | 0 / 1 |  |
| `html/editing/dnd/remove/004.html` | 0 / 1 |  |
| `html/editing/dnd/remove/005.html` | 0 / 1 |  |
| `html/editing/dnd/remove/006.html` | 0 / 1 |  |
| `html/editing/dnd/remove/007.html` | 0 / 1 |  |
| `html/editing/dnd/remove/008.html` | 0 / 1 |  |
| `html/editing/dnd/synthetic/001.html` | 15 / 16 |  |
| `html/editing/dnd/target-origin/202.html` | 0 / 1 |  |
| `html/editing/dnd/the-draggable-attribute/draggable-enumerated-ascii-case-insensitive.html` | 0 / 1 |  |
| `html/editing/dnd/the-draggable-attribute/draggable_attribute.html` | 151 / 302 |  |
| `html/editing/dnd/the-dropzone-attribute/dropzone_attribute.html` | 2 / 4 |  |
| `html/editing/editing-0/autocapitalization/autocapitalize.html` | 0 / 8 |  |
| `html/editing/editing-0/autocorrection/autocorrection.html` | 1 / 9 |  |
| `html/editing/editing-0/contenteditable/contentEditable-slotted-inherit.html` | 0 / 2 |  |
| `html/editing/editing-0/contenteditable/contenteditable-enumerated-ascii-case-insensitive.html` | 2 / 24 |  |
| `html/editing/editing-0/contenteditable/user-interaction-editing-contenteditable.html` | 0 / 9 |  |
| `html/editing/editing-0/making-entire-documents-editable-the-designmode-idl-attribute/contenteditable-false-in-design-mode.html` | 1 / 2 |  |
| `html/editing/editing-0/making-entire-documents-editable-the-designmode-idl-attribute/user-interaction-editing-designMode-svg.svg` | 0 / 3 |  |
| `html/editing/editing-0/making-entire-documents-editable-the-designmode-idl-attribute/user-interaction-editing-designMode.html` | 0 / 3 |  |
| `html/editing/editing-0/spelling-and-grammar-checking/spellcheck-enumerated-ascii-case-insensitive.html` | 0 / 1 |  |
| `html/editing/editing-0/spelling-and-grammar-checking/user-interaction-editing-spellcheck.html` | 1 / 3 |  |
| `html/editing/editing-0/writing-suggestions/writingsuggestions.html` | 6 / 80 |  |
| `html/editing/focus/focus-management/active-element.html` | 1 / 3 |  |
| `html/editing/focus/focus-management/focused-area-removal.html` | 0 / 1 |  |
| `html/editing/focus/focus-management/focusing-disconnected-noop.html` | 0 / 1 |  |
| `html/editing/the-hidden-attribute/beforematch-attribute-removal-001.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/editing/the-hidden-attribute/beforematch-attribute-removal-002.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/editing/the-hidden-attribute/beforematch-element-fragment-navigation.html` | 1 / 10 |  |
| `html/editing/the-hidden-attribute/beforematch-element-removal-001.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/editing/the-hidden-attribute/beforematch-element-removal-002.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/editing/the-hidden-attribute/beforematch-infinite-loop.html` | 0 / 1 |  |
| `html/editing/the-hidden-attribute/beforematch-scroll-to-text-fragment.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/editing/the-hidden-attribute/hidden-idl.html` | 14 / 17 |  |
| `html/editing/the-hidden-attribute/hidden-ua-stylesheet.html` | 1 / 8 |  |
| `html/editing/the-hidden-attribute/hidden-until-found-002.html` | 0 / 1 |  |
| `html/editing/the-hidden-attribute/hidden-until-found-and-details.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/editing/the-hidden-attribute/hidden-until-found-idl.html` | 0 / 1 |  |
| `html/editing/the-hidden-attribute/hidden-until-found-text-fragment.html` | 0 / 1 |  |
| `html/infrastructure/common-dom-interfaces/collections/detached-root-gc.html` | 0 / 11 |  |
| `html/infrastructure/common-dom-interfaces/collections/domstringlist.html` | 0 / 4 |  |
| `html/infrastructure/common-dom-interfaces/collections/historical.html` | 2 / 3 |  |
| `html/infrastructure/common-dom-interfaces/collections/htmlallcollection.html` | 1 / 41 |  |
| `html/infrastructure/common-dom-interfaces/collections/htmlformcontrolscollection.html` | 12 / 16 |  |
| `html/infrastructure/common-dom-interfaces/collections/htmloptionscollection.html` | 4 / 24 |  |
| `html/infrastructure/common-dom-interfaces/collections/radionodelist.html` | 0 / 5 |  |
| `html/infrastructure/fetching-resources/crossorigin-enumerated-ascii-case-insensitive.html` | 0 / 2 |  |
| `html/infrastructure/safe-passing-of-structured-data/cross-origin-transfer-resizable-arraybuffer.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/infrastructure/safe-passing-of-structured-data/messagechannel.any.js` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/infrastructure/safe-passing-of-structured-data/shared-array-buffers/blob-data.https.html` | 2 / 5 | harnais : délai du harnais dépassé |
| `html/infrastructure/safe-passing-of-structured-data/shared-array-buffers/broadcastchannel-success-and-failure.https.html` | 0 / 1 |  |
| `html/infrastructure/safe-passing-of-structured-data/shared-array-buffers/broadcastchannel-success.https.html` | 0 / 1 |  |
| `html/infrastructure/safe-passing-of-structured-data/shared-array-buffers/identity-not-preserved.https.html` | 0 / 3 |  |
| `html/infrastructure/safe-passing-of-structured-data/shared-array-buffers/nested-sharedworker-success.https.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/infrastructure/safe-passing-of-structured-data/shared-array-buffers/nested-worker-success.https.html` | 0 / 2 |  |
| `html/infrastructure/safe-passing-of-structured-data/shared-array-buffers/no-coop-coep.https.any.js` | 0 / 3 |  |
| `html/infrastructure/safe-passing-of-structured-data/shared-array-buffers/no-transferring.https.html` | 2 / 3 |  |
| `html/infrastructure/safe-passing-of-structured-data/shared-array-buffers/serialization-via-idb.any.js` | 0 / 2 |  |
| `html/infrastructure/safe-passing-of-structured-data/shared-array-buffers/serialization-via-notifications-api.any.js` | 0 / 2 |  |
| `html/infrastructure/safe-passing-of-structured-data/shared-array-buffers/window-domain-failure.https.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/infrastructure/safe-passing-of-structured-data/shared-array-buffers/window-failure.https.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/infrastructure/safe-passing-of-structured-data/shared-array-buffers/window-iframe-messagechannel.https.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/infrastructure/safe-passing-of-structured-data/shared-array-buffers/window-iframe-not-cross-origin-isolated.https.html` | 0 / 2 |  |
| `html/infrastructure/safe-passing-of-structured-data/shared-array-buffers/window-messagechannel-success.https.html` | 0 / 1 |  |
| `html/infrastructure/safe-passing-of-structured-data/shared-array-buffers/window-noopener-failure.https.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/infrastructure/safe-passing-of-structured-data/shared-array-buffers/window-opener-success.https.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/infrastructure/safe-passing-of-structured-data/shared-array-buffers/window-serviceworker-failure.https.html` | 0 / 1 |  |
| `html/infrastructure/safe-passing-of-structured-data/shared-array-buffers/window-sharedworker-failure.https.html` | 0 / 1 |  |
| `html/infrastructure/safe-passing-of-structured-data/shared-array-buffers/window-simple-success.https.html` | 0 / 16 |  |
| `html/infrastructure/safe-passing-of-structured-data/structured-cloning-error-stack-optional.sub.window.js` | 16 / 20 |  |
| `html/infrastructure/safe-passing-of-structured-data/structuredclone_0.html` | 31 / 43 |  |
| `html/infrastructure/safe-passing-of-structured-data/transfer-audiodata.html` | 0 / 1 |  |
| `html/infrastructure/safe-passing-of-structured-data/transfer-errors.window.js` | 4 / 17 |  |
| `html/infrastructure/safe-passing-of-structured-data/transfer-mediasourcehandle.html` | 0 / 1 |  |
| `html/infrastructure/safe-passing-of-structured-data/transfer-videoframe.html` | 0 / 1 |  |
| `html/infrastructure/safe-passing-of-structured-data/window-postmessage.window.js` | 89 / 152 |  |
| `html/infrastructure/terminology/plugins/text-plain.html` | 0 / 1 |  |
| `html/infrastructure/urls/base-url/base-url-detached-document-blank.https.window.js` | 0 / 2 | harnais : Uncaught { message: 'assert_equals: expected "https://example.com/" but got "https://web-platform.test/html/infrastructu |
| `html/infrastructure/urls/base-url/base-url-detached-document-srcdoc.https.window.js` | 0 / 2 | harnais : Uncaught { message: 'assert_equals: expected "https://example.com/" but got "https://web-platform.test/html/infrastructu |
| `html/infrastructure/urls/base-url/document-base-uri-synthetic-document.html` | 0 / 2 |  |
| `html/infrastructure/urls/base-url/document-base-url-changes-about-srcdoc-2.https.html` | 1 / 1 | harnais : délai du harnais dépassé |
| `html/infrastructure/urls/base-url/document-base-url-changes-after-nav-about-srcdoc.https.window.js` | 0 / 2 |  |
| `html/infrastructure/urls/base-url/document-base-url-initiated-grand-parent.https.window.js` | 0 / 3 |  |
| `html/infrastructure/urls/base-url/document-base-url-window-initiator-is-not-opener.https.window.js` | 0 / 2 | harnais : Test named 'window.open() gets base url from initiator not opener.' specified 1 'cleanup' function, and 1 failed. |
| `html/infrastructure/urls/base-url/document-base-url-window-open-about-blank.https.window.js` | 0 / 1 |  |
| `html/infrastructure/urls/base-url/initial-about-blank-baseURI.window.js` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/infrastructure/urls/base-url/matches-about-blank-base-url.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/infrastructure/urls/dynamic-changes-to-base-urls/dynamic-urls.sub.html` | 0 / 19 |  |
| `html/infrastructure/urls/dynamic-changes-to-base-urls/historical.sub.xhtml` | 15 / 20 |  |
| `html/infrastructure/urls/resolving-urls/query-encoding/attributes.sub.html` | 14 / 26 |  |
| `html/infrastructure/urls/resolving-urls/query-encoding/location.sub.html` | 0 / 5 | harnais : délai du harnais dépassé |
| `html/infrastructure/urls/resolving-urls/query-encoding/navigation.sub.html` | 0 / 6 |  |
| `html/infrastructure/urls/resolving-urls/query-encoding/utf-8.html` | 35 / 95 | harnais : délai du harnais dépassé |
| `html/infrastructure/urls/resolving-urls/query-encoding/windows-1251.html` | 35 / 95 | harnais : délai du harnais dépassé |
| `html/infrastructure/urls/resolving-urls/query-encoding/windows-1252.html` | 35 / 95 | harnais : délai du harnais dépassé |
| `html/infrastructure/urls/terminology-0/document-base-url-changes-about-srcdoc.https.window.js` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/infrastructure/urls/terminology-0/document-base-url.html` | 3 / 6 |  |
| `html/infrastructure/urls/terminology-0/multiple-base.sub.html` | 0 / 1 |  |
| `html/infrastructure/urls/terminology-0/nontraditional-about-srcdoc.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/interaction/focus/chrome-object-tab-focus-bug.html` | 0 / 1 |  |
| `html/interaction/focus/document-level-focus-apis/document-has-system-focus.html` | 0 / 1 |  |
| `html/interaction/focus/document-level-focus-apis/document-level-apis.html` | 2 / 4 |  |
| `html/interaction/focus/focus-management/focus-event-view.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/interaction/focus/focus-management/focus-events.html` | 1 / 2 |  |
| `html/interaction/focus/focus-selection.html` | 0 / 2 |  |
| `html/interaction/focus/focusgroup/tentative/ax-role-inference-children.html` | 0 / 20 |  |
| `html/interaction/focus/focusgroup/tentative/ax-role-inference-item-elementinternals-nested.html` | 0 / 0 | harnais : Uncaught TypeError: .attachInternals n'est pas une fonction (c'est undefined) |
| `html/interaction/focus/focusgroup/tentative/ax-role-inference-item-elementinternals-owner-implied.html` | 0 / 0 | harnais : Uncaught TypeError: .attachInternals n'est pas une fonction (c'est undefined) |
| `html/interaction/focus/focusgroup/tentative/ax-role-inference-item-elementinternals.html` | 0 / 0 | harnais : Uncaught TypeError: .attachInternals n'est pas une fonction (c'est undefined) |
| `html/interaction/focus/focusgroup/tentative/ax-role-inference-owner-elementinternals.html` | 0 / 0 | harnais : Uncaught TypeError: .attachInternals n'est pas une fonction (c'est undefined) |
| `html/interaction/focus/focusgroup/tentative/ax-role-inference-top-layer.html` | 0 / 2 |  |
| `html/interaction/focus/focusgroup/tentative/backward-navigation/does-not-move-when-on-focusgroup-root.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/backward-navigation/does-not-move-when-on-non-focusgroup-item.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/backward-navigation/does-not-move-when-only-one-item-and-wraps.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/backward-navigation/does-not-move-when-only-one-item.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/backward-navigation/does-not-move-when-outside-focusgroup.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/backward-navigation/does-not-wrap-when-not-supported.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/backward-navigation/horizontal/does-not-ascend-out-of-focusgroup.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/backward-navigation/horizontal/does-not-move-when-axis-not-supported.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/backward-navigation/horizontal/does-not-wrap-in-orthogonal-axis.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/backward-navigation/horizontal/moves-when-only-current-axis-supported.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/backward-navigation/horizontal/wraps-in-axis.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/backward-navigation/moves-to-previous-item-and-skips-focusable-item.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/backward-navigation/moves-to-previous-item.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/backward-navigation/skips-non-focusgroup-subtree.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/backward-navigation/skips-root-focusgroup-complex-case.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/backward-navigation/skips-root-focusgroup.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/backward-navigation/vertical/does-not-ascend-out-of-focusgroup.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/backward-navigation/vertical/does-not-move-when-axis-not-supported.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/backward-navigation/vertical/does-not-wrap-in-orthogonal-axis.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/backward-navigation/vertical/moves-when-only-current-axis-supported.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/backward-navigation/vertical/wraps-in-axis.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/backward-navigation/wraps-successfully-complex-case.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/backward-navigation/wraps-successfully.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/behavior-first-requirement.html` | 0 / 6 |  |
| `html/interaction/focus/focusgroup/tentative/behavior-token-anywhere.html` | 0 / 4 |  |
| `html/interaction/focus/focusgroup/tentative/behavior-tokens/grid.html` | 0 / 8 |  |
| `html/interaction/focus/focusgroup/tentative/behavior-tokens/listbox.html` | 0 / 10 |  |
| `html/interaction/focus/focusgroup/tentative/behavior-tokens/menu.html` | 0 / 15 |  |
| `html/interaction/focus/focusgroup/tentative/behavior-tokens/menubar.html` | 0 / 6 |  |
| `html/interaction/focus/focusgroup/tentative/behavior-tokens/none.html` | 0 / 4 |  |
| `html/interaction/focus/focusgroup/tentative/behavior-tokens/radiogroup.html` | 0 / 14 |  |
| `html/interaction/focus/focusgroup/tentative/behavior-tokens/tablist.html` | 0 / 16 |  |
| `html/interaction/focus/focusgroup/tentative/behavior-tokens/toolbar.html` | 0 / 10 |  |
| `html/interaction/focus/focusgroup/tentative/descendant-navigation/deeply-nested-items.html` | 0 / 3 |  |
| `html/interaction/focus/focusgroup/tentative/descendant-navigation/mixed-content-navigation.html` | 0 / 3 |  |
| `html/interaction/focus/focusgroup/tentative/descendant-navigation/simple-descendant-test.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/descendant-navigation/various-element-types.html` | 0 / 4 |  |
| `html/interaction/focus/focusgroup/tentative/descendant-navigation/wrapping-with-descendants.html` | 0 / 4 |  |
| `html/interaction/focus/focusgroup/tentative/feed-navigation.html` | 0 / 6 |  |
| `html/interaction/focus/focusgroup/tentative/feed-role-inference-item.html` | 0 / 5 |  |
| `html/interaction/focus/focusgroup/tentative/feed-role-inference-owner.html` | 0 / 3 |  |
| `html/interaction/focus/focusgroup/tentative/focusgroupstart-basic.html` | 0 / 7 |  |
| `html/interaction/focus/focusgroup/tentative/focusgroupstart-scope-owners.html` | 0 / 2 |  |
| `html/interaction/focus/focusgroup/tentative/focusgroupstart-tree-pattern.html` | 0 / 5 |  |
| `html/interaction/focus/focusgroup/tentative/forward-navigation/does-not-move-when-on-focusgroup-root.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/forward-navigation/does-not-move-when-on-non-item.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/forward-navigation/does-not-move-when-only-one-item-and-wraps.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/forward-navigation/does-not-move-when-only-one-item.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/forward-navigation/does-not-move-when-outside-focusgroup.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/forward-navigation/does-not-wrap-when-not-supported.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/forward-navigation/horizontal/does-not-move-when-axis-not-supported.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/forward-navigation/horizontal/moves-when-only-current-axis-supported.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/forward-navigation/horizontal/rtl-inline-only.html` | 0 / 3 |  |
| `html/interaction/focus/focusgroup/tentative/forward-navigation/mixed-ltr-rtl-visual-order.html` | 0 / 11 |  |
| `html/interaction/focus/focusgroup/tentative/forward-navigation/moves-to-next-item-and-skips-non-focusable.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/forward-navigation/moves-to-next-item.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/forward-navigation/nested-focusgroup-is-item-of-parent.html` | 0 / 6 |  |
| `html/interaction/focus/focusgroup/tentative/forward-navigation/rtl-direction-reverses-inline.html` | 0 / 4 |  |
| `html/interaction/focus/focusgroup/tentative/forward-navigation/rtl-wraps-correctly.html` | 0 / 2 |  |
| `html/interaction/focus/focusgroup/tentative/forward-navigation/vertical-writing-mode.html` | 0 / 5 |  |
| `html/interaction/focus/focusgroup/tentative/forward-navigation/vertical/does-not-move-when-axis-not-supported.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/forward-navigation/vertical/moves-when-only-current-axis-supported.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/forward-navigation/wraps-successfully.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/grid-navigation/col-flow-only.html` | 0 / 4 |  |
| `html/interaction/focus/focusgroup/tentative/grid-navigation/col-wrap-and-row-flow.html` | 0 / 4 |  |
| `html/interaction/focus/focusgroup/tentative/grid-navigation/col-wrap-only.html` | 0 / 4 |  |
| `html/interaction/focus/focusgroup/tentative/grid-navigation/colspan.html` | 0 / 4 |  |
| `html/interaction/focus/focusgroup/tentative/grid-navigation/css-table-not-focusgroup.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/grid-navigation/css-table.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/grid-navigation/does-not-wrap-or-flow.html` | 0 / 4 |  |
| `html/interaction/focus/focusgroup/tentative/grid-navigation/empty-spaces.html` | 0 / 4 |  |
| `html/interaction/focus/focusgroup/tentative/grid-navigation/flows-in-both-axes.html` | 0 / 4 |  |
| `html/interaction/focus/focusgroup/tentative/grid-navigation/grid-grammar-conflicts.html` | 0 / 4 |  |
| `html/interaction/focus/focusgroup/tentative/grid-navigation/moves-across-table-sections.html` | 0 / 2 |  |
| `html/interaction/focus/focusgroup/tentative/grid-navigation/non-table.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/grid-navigation/relayout-before-navigation.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/grid-navigation/row-flow-only.html` | 0 / 4 |  |
| `html/interaction/focus/focusgroup/tentative/grid-navigation/row-wrap-and-col-flow.html` | 0 / 4 |  |
| `html/interaction/focus/focusgroup/tentative/grid-navigation/row-wrap-only.html` | 0 / 4 |  |
| `html/interaction/focus/focusgroup/tentative/grid-navigation/rowspan.html` | 0 / 4 |  |
| `html/interaction/focus/focusgroup/tentative/grid-navigation/simple-case-with-non-focusable-cell-in-the-center.html` | 0 / 4 |  |
| `html/interaction/focus/focusgroup/tentative/grid-navigation/simple-case.html` | 0 / 4 |  |
| `html/interaction/focus/focusgroup/tentative/grid-navigation/wraps-in-both-axes.html` | 0 / 4 |  |
| `html/interaction/focus/focusgroup/tentative/home-end-keys.optional.html` | 0 / 22 |  |
| `html/interaction/focus/focusgroup/tentative/idl-reflection.html` | 1 / 21 |  |
| `html/interaction/focus/focusgroup/tentative/none-anywhere-opts-out.html` | 0 / 3 |  |
| `html/interaction/focus/focusgroup/tentative/opt-out-barriers/complex-nested-opt-out.html` | 1 / 5 |  |
| `html/interaction/focus/focusgroup/tentative/opt-out-barriers/none-creates-barriers.html` | 0 / 3 |  |
| `html/interaction/focus/focusgroup/tentative/opt-out-barriers/none-opt-out-direct-child.html` | 0 / 3 |  |
| `html/interaction/focus/focusgroup/tentative/reading-flow-navigation/explicit-visual-reordering.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/reading-flow-navigation/fallback-to-dom-order.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/reading-flow-navigation/flex-visual-order-basic.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/reading-flow-navigation/nested-containers.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/reading-flow-navigation/partial-reordering.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/reading-flow-navigation/simple-mixed.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/sequential-navigation/basic-tab-behavior.html` | 0 / 3 |  |
| `html/interaction/focus/focusgroup/tentative/sequential-navigation/directional-input-handler-contenteditable.html` | 0 / 2 |  |
| `html/interaction/focus/focusgroup/tentative/sequential-navigation/directional-input-handler-multi-tab-stop.html` | 0 / 4 |  |
| `html/interaction/focus/focusgroup/tentative/sequential-navigation/directional-input-handler-multiple-in-segment.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/sequential-navigation/directional-input-handler-nested-focusgroup.html` | 0 / 2 |  |
| `html/interaction/focus/focusgroup/tentative/sequential-navigation/directional-input-handler-number.html` | 0 / 2 |  |
| `html/interaction/focus/focusgroup/tentative/sequential-navigation/directional-input-handler-only-item.html` | 0 / 2 |  |
| `html/interaction/focus/focusgroup/tentative/sequential-navigation/directional-input-handler-radio.html` | 0 / 5 |  |
| `html/interaction/focus/focusgroup/tentative/sequential-navigation/directional-input-handler-range.html` | 0 / 2 |  |
| `html/interaction/focus/focusgroup/tentative/sequential-navigation/directional-input-handler-scrollable-container.html` | 0 / 3 |  |
| `html/interaction/focus/focusgroup/tentative/sequential-navigation/directional-input-handler-select.html` | 0 / 2 |  |
| `html/interaction/focus/focusgroup/tentative/sequential-navigation/directional-input-handler-tab-escape-adjacency.html` | 0 / 4 |  |
| `html/interaction/focus/focusgroup/tentative/sequential-navigation/directional-input-handler-tab-escape.html` | 0 / 4 |  |
| `html/interaction/focus/focusgroup/tentative/sequential-navigation/directional-input-handler-text.html` | 0 / 2 |  |
| `html/interaction/focus/focusgroup/tentative/sequential-navigation/directional-input-handler-textarea.html` | 0 / 2 |  |
| `html/interaction/focus/focusgroup/tentative/sequential-navigation/directional-input-handler-with-explicit-optout.html` | 0 / 3 |  |
| `html/interaction/focus/focusgroup/tentative/sequential-navigation/directional-input-handler-with-priority.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/sequential-navigation/dynamic-changes.html` | 0 / 2 |  |
| `html/interaction/focus/focusgroup/tentative/sequential-navigation/empty-and-non-focusable.html` | 0 / 4 |  |
| `html/interaction/focus/focusgroup/tentative/sequential-navigation/focusgroup-segments.html` | 0 / 5 |  |
| `html/interaction/focus/focusgroup/tentative/sequential-navigation/focusgroupstart-shadow-tabindex.html` | 0 / 5 |  |
| `html/interaction/focus/focusgroup/tentative/sequential-navigation/guaranteed-tab-stop-priority.html` | 0 / 5 |  |
| `html/interaction/focus/focusgroup/tentative/sequential-navigation/memory-behavior.html` | 0 / 5 |  |
| `html/interaction/focus/focusgroup/tentative/sequential-navigation/nested-focusgroups.html` | 0 / 2 |  |
| `html/interaction/focus/focusgroup/tentative/sequential-navigation/segment-barrier-tabindex-minus1.html` | 0 / 6 |  |
| `html/interaction/focus/focusgroup/tentative/sequential-navigation/tabindex-minus-one-home-end.optional.html` | 0 / 2 |  |
| `html/interaction/focus/focusgroup/tentative/sequential-navigation/tabindex-minus-one-tab-escape.html` | 0 / 8 |  |
| `html/interaction/focus/focusgroup/tentative/shadow/shadow-items-basic.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/shadow/shadow-nested-scope.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/shadow/shadow-reattach.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/top-layer-dialog-excluded.html` | 0 / 1 |  |
| `html/interaction/focus/focusgroup/tentative/top-layer-dynamic-exclusion.html` | 0 / 6 | harnais : Test named 'Top-layer exclusion is dynamic: show and hide cycles' specified 1 'cleanup' function, and 1 failed. |
| `html/interaction/focus/focusgroup/tentative/top-layer-inner-focusgroup.html` | 0 / 3 |  |
| `html/interaction/focus/focusgroup/tentative/top-layer-popover-excluded.html` | 0 / 6 |  |
| `html/interaction/focus/focusgroup/tentative/top-layer-popover-invoker.html` | 0 / 7 |  |
| `html/interaction/focus/focusgroup/tentative/untraversable-tabindex-minus-one.html` | 0 / 3 |  |
| `html/interaction/focus/focusreset/focusreset-basic.tentative.html` | 4 / 9 |  |
| `html/interaction/focus/focusreset/focusreset-cancelled-by-focus-update.tentative.html` | 5 / 13 |  |
| `html/interaction/focus/focusreset/focusreset-iframe.tentative.html` | 2 / 8 |  |
| `html/interaction/focus/focusreset/focusreset-moveBefore.tentative.html` | 2 / 3 |  |
| `html/interaction/focus/focusreset/focusreset-removal-variants.tentative.html` | 0 / 22 |  |
| `html/interaction/focus/focusreset/focusreset-timing.tentative.html` | 0 / 9 |  |
| `html/interaction/focus/processing-model/focus-fixup-rule-disabled.tentative.html` | 5 / 7 |  |
| `html/interaction/focus/processing-model/focus-fixup-rule-one-no-dialogs.html` | 0 / 8 |  |
| `html/interaction/focus/processing-model/focusVisible.html` | 0 / 5 | harnais : délai du harnais dépassé |
| `html/interaction/focus/processing-model/preventScroll.html` | 6 / 7 |  |
| `html/interaction/focus/processing-model/textarea-scroll-selection.html` | 0 / 1 |  |
| `html/interaction/focus/sequential-focus-navigation-after-disabled.html` | 0 / 1 |  |
| `html/interaction/focus/sequential-focus-navigation-and-the-tabindex-attribute/focus-tabindex-default-value.html` | 1 / 2 |  |
| `html/interaction/focus/sequential-focus-navigation-and-the-tabindex-attribute/focus-tabindex-negative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/interaction/focus/sequential-focus-navigation-and-the-tabindex-attribute/focus-tabindex-order.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/interaction/focus/sequential-focus-navigation-and-the-tabindex-attribute/focus-tabindex-positive.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/interaction/focus/sequential-focus-navigation-and-the-tabindex-attribute/focus-tabindex-zero.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/interaction/focus/sequential-focus-navigation-and-the-tabindex-attribute/tabindex-getter-frame.html` | 3 / 8 |  |
| `html/interaction/focus/sequential-focus-navigation-and-the-tabindex-attribute/tabindex-getter.html` | 106 / 120 |  |
| `html/interaction/focus/sequential-focus-navigation-starting-point.tentative.html` | 0 / 26 | harnais : délai du harnais dépassé |
| `html/interaction/focus/setSequentialFocusStartingPoint.tentative.html` | 0 / 1 |  |
| `html/interaction/focus/tab-table-caption.html` | 2 / 6 | harnais : délai du harnais dépassé |
| `html/interaction/focus/tabindex-focus-flag.html` | 19 / 35 |  |
| `html/interaction/focus/the-autofocus-attribute/autofocus-dialog.html` | 0 / 2 |  |
| `html/interaction/focus/the-autofocus-attribute/autofocus-in-not-fully-active-document.html` | 0 / 1 |  |
| `html/interaction/focus/the-autofocus-attribute/autofocus-on-stable-document.html` | 0 / 1 |  |
| `html/interaction/focus/the-autofocus-attribute/document-with-fragment-empty.html` | 0 / 2 |  |
| `html/interaction/focus/the-autofocus-attribute/document-with-fragment-nonexistent.html` | 0 / 2 |  |
| `html/interaction/focus/the-autofocus-attribute/document-with-fragment-removed-target.html` | 0 / 1 |  |
| `html/interaction/focus/the-autofocus-attribute/document-with-fragment-top.html` | 0 / 2 |  |
| `html/interaction/focus/the-autofocus-attribute/document-with-fragment-valid.html` | 0 / 3 |  |
| `html/interaction/focus/the-autofocus-attribute/first-reconnected.html` | 0 / 1 |  |
| `html/interaction/focus/the-autofocus-attribute/focusable-area-in-top-document.html` | 0 / 1 |  |
| `html/interaction/focus/the-autofocus-attribute/no-cross-origin-autofocus.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/interaction/focus/the-autofocus-attribute/same-origin-autofocus.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/interaction/focus/the-autofocus-attribute/skip-another-top-level-browsing-context.html` | 0 / 1 |  |
| `html/interaction/focus/the-autofocus-attribute/spin-by-blocking-style-sheet.html` | 0 / 1 |  |
| `html/interaction/focus/the-autofocus-attribute/supported-elements.html` | 0 / 6 |  |
| `html/interaction/focus/the-autofocus-attribute/update-the-rendering.html` | 0 / 1 | harnais : Test named '"Flush autofocus candidates" should be happen before a scroll event and animation frame callbacks' specified |
| `html/links/manifest/link-relationship/link-rel-manifest.html` | 0 / 1 |  |
| `html/links/stylesheet/quirk-origin-check-recursive-import.html` | 0 / 1 |  |
| `html/meta/refresh-time.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/obsolete/requirements-for-implementations/other-elements-attributes-and-apis/document-all.html` | 0 / 2 |  |
| `html/obsolete/requirements-for-implementations/other-elements-attributes-and-apis/document-applets.html` | 0 / 1 |  |
| `html/obsolete/requirements-for-implementations/other-elements-attributes-and-apis/document-color-01.html` | 1 / 6 |  |
| `html/obsolete/requirements-for-implementations/other-elements-attributes-and-apis/document-color-02.html` | 1 / 7 |  |
| `html/obsolete/requirements-for-implementations/other-elements-attributes-and-apis/document-color-03.html` | 1 / 7 |  |
| `html/obsolete/requirements-for-implementations/other-elements-attributes-and-apis/document-color-04.html` | 1 / 6 |  |
| `html/obsolete/requirements-for-implementations/other-elements-attributes-and-apis/nothing.html` | 0 / 5 |  |
| `html/obsolete/requirements-for-implementations/other-elements-attributes-and-apis/script-IDL-event-htmlfor.html` | 0 / 5 |  |
| `html/obsolete/requirements-for-implementations/other-elements-attributes-and-apis/window-external.html` | 0 / 2 |  |
| `html/obsolete/requirements-for-implementations/the-marquee-element-0/marquee-events-historical.html` | 1 / 2 |  |
| `html/obsolete/requirements-for-implementations/the-marquee-element-0/marquee-loop.html` | 0 / 3 |  |
| `html/obsolete/requirements-for-implementations/the-marquee-element-0/marquee-overflow.html` | 0 / 1 |  |
| `html/obsolete/requirements-for-implementations/the-marquee-element-0/marquee-scrollamount.html` | 0 / 3 |  |
| `html/obsolete/requirements-for-implementations/the-marquee-element-0/marquee-scrolldelay.html` | 0 / 4 |  |
| `html/rendering/bidi-rendering/slot-direction.window.js` | 0 / 6 |  |
| `html/rendering/bidi-rendering/unicode-bidi-ua-rules-iso-8859-8.html` | 0 / 67 |  |
| `html/rendering/bidi-rendering/unicode-bidi-ua-rules.html` | 0 / 130 |  |
| `html/rendering/dimension-attributes.html` | 913 / 1720 |  |
| `html/rendering/non-replaced-elements/flow-content-0/dialog.html` | 0 / 6 |  |
| `html/rendering/non-replaced-elements/flow-content-0/form-margin-quirk.html` | 0 / 1 |  |
| `html/rendering/non-replaced-elements/flow-content-0/search-styles-iso-8859-8.html` | 0 / 1 |  |
| `html/rendering/non-replaced-elements/flow-content-0/search-styles.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/rendering/non-replaced-elements/flow-content-0/slot-element-focusable.tentative.html` | 0 / 2 |  |
| `html/rendering/non-replaced-elements/flow-content-0/slot-element-tabbable.tentative.html` | 0 / 2 |  |
| `html/rendering/non-replaced-elements/form-controls/input-line-height-computed.html` | 0 / 12 |  |
| `html/rendering/non-replaced-elements/form-controls/resets.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/rendering/non-replaced-elements/hidden-elements.html` | 15 / 20 |  |
| `html/rendering/non-replaced-elements/input-type-hidden-important.html` | 0 / 5 |  |
| `html/rendering/non-replaced-elements/lists/li-text-align.html` | 0 / 2 |  |
| `html/rendering/non-replaced-elements/lists/lists-presentational-hints-ascii-case-insensitive.html` | 0 / 2 |  |
| `html/rendering/non-replaced-elements/lists/lists-styles-quirks.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/rendering/non-replaced-elements/lists/lists-styles.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/rendering/non-replaced-elements/phrasing-content-0/font-element-text-decoration-color/font-face.html` | 1 / 5 |  |
| `html/rendering/non-replaced-elements/phrasing-content-0/font-element-text-decoration-color/font-size.html` | 4 / 28 |  |
| `html/rendering/non-replaced-elements/sections-and-headings/headings-styles.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/rendering/non-replaced-elements/tables/form-in-tables.html` | 0 / 10 |  |
| `html/rendering/non-replaced-elements/tables/hidden-attr.html` | 4 / 10 |  |
| `html/rendering/non-replaced-elements/tables/table-attribute-cellspacing-dynamic-within-display-none.html` | 0 / 1 |  |
| `html/rendering/non-replaced-elements/tables/table-attribute.html` | 16 / 58 | harnais : Test named 'table_tr height attribute percentage is correct' specified 1 'cleanup' function, and 1 failed. |
| `html/rendering/non-replaced-elements/tables/table-border-attributes-computed-style.html` | 2 / 9 |  |
| `html/rendering/non-replaced-elements/tables/table-border-color-not-inherited.html` | 1 / 3 |  |
| `html/rendering/non-replaced-elements/tables/table-bordercolor-default.html` | 0 / 1 |  |
| `html/rendering/non-replaced-elements/tables/table-rules-computed-style.html` | 0 / 35 |  |
| `html/rendering/non-replaced-elements/tables/table-valign-baseline-ascii-case-insensitive.html` | 0 / 1 |  |
| `html/rendering/non-replaced-elements/the-fieldset-and-legend-elements/fieldset-block-formatting-context.html` | 0 / 1 |  |
| `html/rendering/non-replaced-elements/the-fieldset-and-legend-elements/fieldset-block-size.html` | 0 / 1 |  |
| `html/rendering/non-replaced-elements/the-fieldset-and-legend-elements/fieldset-border-radius-hittest.html` | 0 / 1 |  |
| `html/rendering/non-replaced-elements/the-fieldset-and-legend-elements/fieldset-calculating-min-max-content.html` | 1 / 2 |  |
| `html/rendering/non-replaced-elements/the-fieldset-and-legend-elements/fieldset-content-before-legend.html` | 1 / 2 |  |
| `html/rendering/non-replaced-elements/the-fieldset-and-legend-elements/fieldset-display-ruby.html` | 0 / 6 |  |
| `html/rendering/non-replaced-elements/the-fieldset-and-legend-elements/fieldset-display.html` | 7 / 15 |  |
| `html/rendering/non-replaced-elements/the-fieldset-and-legend-elements/fieldset-generated-content.html` | 0 / 1 |  |
| `html/rendering/non-replaced-elements/the-fieldset-and-legend-elements/fieldset-overflow-cssomview.html` | 0 / 1 |  |
| `html/rendering/non-replaced-elements/the-fieldset-and-legend-elements/fieldset-percentage-block-size.html` | 1 / 10 |  |
| `html/rendering/non-replaced-elements/the-fieldset-and-legend-elements/flex-legend-float-abspos.html` | 0 / 4 |  |
| `html/rendering/non-replaced-elements/the-fieldset-and-legend-elements/legend-align-justify-self.html` | 0 / 13 |  |
| `html/rendering/non-replaced-elements/the-fieldset-and-legend-elements/legend-block-formatting-context.html` | 1 / 4 |  |
| `html/rendering/non-replaced-elements/the-fieldset-and-legend-elements/legend-block-position-centering.html` | 0 / 1 |  |
| `html/rendering/non-replaced-elements/the-fieldset-and-legend-elements/legend-display-ruby.html` | 0 / 24 |  |
| `html/rendering/non-replaced-elements/the-fieldset-and-legend-elements/legend-display.html` | 36 / 80 |  |
| `html/rendering/non-replaced-elements/the-fieldset-and-legend-elements/legend-float-abspos.html` | 0 / 4 |  |
| `html/rendering/non-replaced-elements/the-fieldset-and-legend-elements/legend-grid-flex-multicol.html` | 2 / 5 |  |
| `html/rendering/non-replaced-elements/the-fieldset-and-legend-elements/legend-hover.html` | 0 / 1 |  |
| `html/rendering/non-replaced-elements/the-fieldset-and-legend-elements/legend-sans-fieldset-display-ruby.html` | 0 / 2 |  |
| `html/rendering/non-replaced-elements/the-fieldset-and-legend-elements/legend.html` | 47 / 56 |  |
| `html/rendering/non-replaced-elements/the-fieldset-and-legend-elements/min-inline-size.html` | 0 / 6 |  |
| `html/rendering/non-replaced-elements/the-frameset-and-frame-elements/exceed-then-not-exceed.html` | 0 / 1 |  |
| `html/rendering/non-replaced-elements/the-frameset-and-frame-elements/large-cols-relsize.html` | 0 / 1 |  |
| `html/rendering/non-replaced-elements/the-frameset-and-frame-elements/large-rows-relsize.html` | 0 / 1 |  |
| `html/rendering/non-replaced-elements/the-hr-element-0/setting-overflow-visible.html` | 0 / 2 |  |
| `html/rendering/non-replaced-elements/the-page/iframe-body-margin-attributes.html` | 0 / 3 |  |
| `html/rendering/pixel-length-attributes.html` | 0 / 84 |  |
| `html/rendering/replaced-elements/attributes-for-embedded-content-and-images/align.html` | 5 / 23 |  |
| `html/rendering/replaced-elements/attributes-for-embedded-content-and-images/canvas-aspect-ratio.html` | 7 / 13 |  |
| `html/rendering/replaced-elements/attributes-for-embedded-content-and-images/embedded-and-images-presentational-hints-ascii-case-insensitive.html` | 0 / 3 |  |
| `html/rendering/replaced-elements/attributes-for-embedded-content-and-images/img-aspect-ratio-alt-text.html` | 1 / 2 |  |
| `html/rendering/replaced-elements/attributes-for-embedded-content-and-images/img-aspect-ratio-lazy.html` | 0 / 1 |  |
| `html/rendering/replaced-elements/attributes-for-embedded-content-and-images/img-aspect-ratio.html` | 22 / 41 |  |
| `html/rendering/replaced-elements/attributes-for-embedded-content-and-images/img-empty-alt-replaced.html` | 0 / 1 |  |
| `html/rendering/replaced-elements/attributes-for-embedded-content-and-images/img-replaced-box-while-loading.html` | 0 / 1 |  |
| `html/rendering/replaced-elements/attributes-for-embedded-content-and-images/picture-aspect-ratio.html` | 1 / 15 |  |
| `html/rendering/replaced-elements/attributes-for-embedded-content-and-images/textarea-placeholder-scroll.html` | 0 / 1 |  |
| `html/rendering/replaced-elements/attributes-for-embedded-content-and-images/textarea-scrollheight-empty.html` | 0 / 1 |  |
| `html/rendering/replaced-elements/attributes-for-embedded-content-and-images/video-aspect-ratio.html` | 5 / 14 |  |
| `html/rendering/replaced-elements/attributes-for-embedded-content-and-images/video-intrinsic-width-height.html` | 4 / 6 |  |
| `html/rendering/replaced-elements/embedded-content/iframe-frameborder.html` | 0 / 13 |  |
| `html/rendering/replaced-elements/images/img-fallback-baseline-alignment.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/rendering/replaced-elements/images/img-represents-image-size-before-partially-available.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/rendering/replaced-elements/images/img-sizes-auto.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/rendering/replaced-elements/svg-embedded-sizing/svg-in-iframe-auto.html` | 200 / 201 | harnais : délai du harnais dépassé |
| `html/rendering/replaced-elements/svg-embedded-sizing/svg-in-iframe-fixed.html` | 200 / 201 | harnais : délai du harnais dépassé |
| `html/rendering/replaced-elements/svg-embedded-sizing/svg-in-iframe-percentage.html` | 100 / 201 | harnais : délai du harnais dépassé |
| `html/rendering/replaced-elements/svg-embedded-sizing/svg-in-img-auto.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/rendering/replaced-elements/svg-embedded-sizing/svg-in-img-fixed.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/rendering/replaced-elements/svg-embedded-sizing/svg-in-img-percentage.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/rendering/replaced-elements/svg-embedded-sizing/svg-in-object-auto.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/rendering/replaced-elements/svg-embedded-sizing/svg-in-object-fixed.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/rendering/replaced-elements/svg-embedded-sizing/svg-in-object-percentage.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/rendering/replaced-elements/svg-inline-sizing/svg-inline.html` | 144 / 648 |  |
| `html/rendering/the-css-user-agent-style-sheet-and-presentational-hints/mouse-cursor-imagemap.html` | 0 / 1 |  |
| `html/rendering/the-css-user-agent-style-sheet-and-presentational-hints/no-help-cursor-on-links-wrapped-in-svg.historical.html` | 0 / 2 |  |
| `html/rendering/the-css-user-agent-style-sheet-and-presentational-hints/no-help-cursor-on-links.historical.html` | 0 / 2 |  |
| `html/rendering/the-details-element/auto-expand-details-text-fragment.html` | 0 / 1 |  |
| `html/rendering/the-details-element/details-blockification.html` | 0 / 1 |  |
| `html/rendering/the-details-element/details-display.html` | 7 / 8 |  |
| `html/rendering/the-details-element/details-focus-steps.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/rendering/the-details-element/details-pseudo-elements-006.html` | 0 / 1 |  |
| `html/rendering/unmapped-attributes.html` | 61 / 62 | harnais : 30 duplicate test names: "<table hspace> should not be mapped to style marginLeft in CSS1Compat mode", "<table hspace> s |
| `html/rendering/widgets/appearance/default-styles.html` | 0 / 45 |  |
| `html/rendering/widgets/button-layout/computed-style.html` | 73 / 167 |  |
| `html/rendering/widgets/button-layout/display-other.html` | 5 / 18 |  |
| `html/rendering/widgets/button-layout/flex.html` | 3 / 5 |  |
| `html/rendering/widgets/button-layout/grid.html` | 0 / 2 |  |
| `html/rendering/widgets/button-layout/shrink-wrap.html` | 10 / 24 |  |
| `html/rendering/widgets/field-sizing-input-number.html` | 2 / 9 |  |
| `html/rendering/widgets/field-sizing-input-text.html` | 12 / 54 |  |
| `html/rendering/widgets/field-sizing-select.html` | 0 / 2 |  |
| `html/rendering/widgets/field-sizing-textarea.html` | 2 / 10 |  |
| `html/rendering/widgets/input-checkbox-switch.tentative.window.js` | 1 / 4 |  |
| `html/rendering/widgets/input-text-size.html` | 4 / 5 |  |
| `html/rendering/widgets/placeholder-scroll.html` | 0 / 2 |  |
| `html/rendering/widgets/shadow-dom.html` | 0 / 9 |  |
| `html/rendering/widgets/text-control-client-width.html` | 0 / 2 |  |
| `html/rendering/widgets/textarea-cols-rows.html` | 4 / 6 |  |
| `html/rendering/widgets/textarea-scroll-restoration.html` | 0 / 2 |  |
| `html/rendering/widgets/the-select-element/select-as-listbox-default-styles.tentative.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/scripting/the-script-element/attr-script-fetchpriority.html` | 0 / 2 |  |
| `html/scripting/the-script-element/module-script-disabled-scripting.html` | 0 / 1 |  |
| `html/select/options-length-too-large.html` | 1 / 5 |  |
| `html/semantics/disabled-elements/disabled-event-dispatch-additional.tentative.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/semantics/disabled-elements/disabled-event-dispatch.tentative.html` | 0 / 20 | harnais : délai du harnais dépassé |
| `html/semantics/disabled-elements/disabledElement.html` | 7 / 10 |  |
| `html/semantics/disabled-elements/event-propagate-disabled-keyboard.tentative.html` | 0 / 0 | harnais : Uncaught TypeError: .attachInternals n'est pas une fonction (c'est undefined) |
| `html/semantics/disabled-elements/event-propagate-disabled.tentative.html` | 146 / 256 |  |
| `html/semantics/document-metadata/interactions-of-styling-and-scripting/conditionally-block-rendering-on-link-media-attr.html` | 0 / 2 |  |
| `html/semantics/document-metadata/interactions-of-styling-and-scripting/dynamic-render-blocking-link-stylesheet-does-not-block-script.html` | 0 / 1 |  |
| `html/semantics/document-metadata/interactions-of-styling-and-scripting/dynamic-render-blocking-style-element-does-not-block-script.html` | 0 / 1 |  |
| `html/semantics/document-metadata/interactions-of-styling-and-scripting/link-stylesheet-with-non-match-media-does-not-block-render.tentative.html` | 0 / 1 |  |
| `html/semantics/document-metadata/interactions-of-styling-and-scripting/script-created-link-stylesheet-does-not-block-script.html` | 0 / 1 |  |
| `html/semantics/document-metadata/interactions-of-styling-and-scripting/script-created-style-element-does-not-block-script.html` | 0 / 1 |  |
| `html/semantics/document-metadata/interactions-of-styling-and-scripting/style-element-media-match-block-script.html` | 0 / 1 |  |
| `html/semantics/document-metadata/interactions-of-styling-and-scripting/style-element-media-not-match-does-not-block-script.html` | 0 / 1 |  |
| `html/semantics/document-metadata/styling/LinkStyle.html` | 1 / 5 |  |
| `html/semantics/document-metadata/the-base-element/base-data.html` | 0 / 0 | le harnais n'a pas terminé |
| `html/semantics/document-metadata/the-base-element/base-javascript.html` | 0 / 0 | le harnais n'a pas terminé |
| `html/semantics/document-metadata/the-base-element/base_about_blank.html` | 0 / 1 |  |
| `html/semantics/document-metadata/the-base-element/base_href_empty.html` | 0 / 2 |  |
| `html/semantics/document-metadata/the-base-element/base_href_specified.html` | 0 / 2 |  |
| `html/semantics/document-metadata/the-base-element/base_href_unspecified.html` | 0 / 2 |  |
| `html/semantics/document-metadata/the-base-element/base_multiple.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-base-element/base_srcdoc.html` | 0 / 1 |  |
| `html/semantics/document-metadata/the-base-element/base_target_does_not_affect_iframe_src_navigation.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-base-element/base_target_does_not_affect_location_assignment.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-link-element/attr-link-fetchpriority.html` | 0 / 2 |  |
| `html/semantics/document-metadata/the-link-element/document-without-browsing-context.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-link-element/link-error-fired-before-scripting-unblocked.html` | 0 / 1 |  |
| `html/semantics/document-metadata/the-link-element/link-load-error-events.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-link-element/link-load-error-events.https.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-link-element/link-load-event-alt-001.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-link-element/link-load-event-alt-002.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-link-element/link-load-event.html` | 0 / 1 |  |
| `html/semantics/document-metadata/the-link-element/link-load-fired-before-scripting-unblocked.html` | 0 / 1 |  |
| `html/semantics/document-metadata/the-link-element/link-multiple-load-events.html` | 0 / 6 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-link-element/link-rel-attribute-tokenization.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-link-element/link-rel-attribute.html` | 0 / 2 |  |
| `html/semantics/document-metadata/the-link-element/link-rellist.html` | 0 / 1 |  |
| `html/semantics/document-metadata/the-link-element/link-style-data-url-quirks.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-link-element/link-style-error-01.html` | 0 / 26 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-link-element/link-style-error-limited-quirks.html` | 0 / 24 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-link-element/link-style-error-quirks.html` | 0 / 24 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-link-element/link-style-quirks-crossorigin.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-link-element/link-stylesheet-error-status-fires-error.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-link-element/stylesheet-appropriate-time-to-obtain.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-link-element/stylesheet-bad-mime-type.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-link-element/stylesheet-encoding-parse.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-link-element/stylesheet-media-change-no-fetch.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-link-element/stylesheet-non-OK-status.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-link-element/stylesheet-not-removed-until-next-stylesheet-loads.html` | 0 / 1 |  |
| `html/semantics/document-metadata/the-link-element/tentative/module-stylesheet-classification.html` | 0 / 9 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-link-element/tentative/module-stylesheet-document-context.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-link-element/tentative/module-stylesheet-failure-events.html` | 0 / 9 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-link-element/tentative/module-stylesheet-fetch.html` | 0 / 7 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-link-element/tentative/module-stylesheet-module-map-reuse.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-meta-element/color-scheme/meta-color-scheme-attribute-changes.html` | 0 / 8 |  |
| `html/semantics/document-metadata/the-meta-element/color-scheme/meta-color-scheme-empty-content-value.html` | 0 / 1 |  |
| `html/semantics/document-metadata/the-meta-element/color-scheme/meta-color-scheme-first-valid-applies.html` | 0 / 1 |  |
| `html/semantics/document-metadata/the-meta-element/color-scheme/meta-color-scheme-insert.html` | 0 / 3 |  |
| `html/semantics/document-metadata/the-meta-element/color-scheme/meta-color-scheme-no-content-value.html` | 0 / 1 |  |
| `html/semantics/document-metadata/the-meta-element/color-scheme/meta-color-scheme-normal-descendant-change.html` | 1 / 2 |  |
| `html/semantics/document-metadata/the-meta-element/color-scheme/meta-color-scheme-remove-head.html` | 0 / 2 |  |
| `html/semantics/document-metadata/the-meta-element/color-scheme/meta-color-scheme-remove.html` | 0 / 3 |  |
| `html/semantics/document-metadata/the-meta-element/color-scheme/meta-color-scheme-single-value-in-body.html` | 0 / 1 |  |
| `html/semantics/document-metadata/the-meta-element/color-scheme/meta-color-scheme-single-value-in-head.html` | 0 / 1 |  |
| `html/semantics/document-metadata/the-meta-element/color-scheme/meta-color-scheme-single-value-in-shadow-tree.html` | 0 / 1 |  |
| `html/semantics/document-metadata/the-meta-element/http-equiv-and-name-1.html` | 0 / 1 |  |
| `html/semantics/document-metadata/the-meta-element/http-equiv-and-name-2.html` | 0 / 1 |  |
| `html/semantics/document-metadata/the-meta-element/pragma-directives/attr-meta-http-equiv-refresh/allow-scripts-flag-changing-1.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-meta-element/pragma-directives/attr-meta-http-equiv-refresh/allow-scripts-flag-changing-2.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-meta-element/pragma-directives/attr-meta-http-equiv-refresh/dynamic-append.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-meta-element/pragma-directives/attr-meta-http-equiv-refresh/parsing.html` | 0 / 133 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-meta-element/pragma-directives/attr-meta-http-equiv-refresh/remove-from-document.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-meta-element/pragma-directives/http-equiv-enumerated-ascii-case-insensitive.html` | 0 / 1 |  |
| `html/semantics/document-metadata/the-meta-element/pragma-directives/http-equiv-lang-removal.html` | 0 / 1 |  |
| `html/semantics/document-metadata/the-meta-element/pragma-directives/http-equiv-lang-whitespace-end.html` | 0 / 1 |  |
| `html/semantics/document-metadata/the-meta-element/pragma-directives/http-equiv-lang-whitespace-start.html` | 0 / 1 |  |
| `html/semantics/document-metadata/the-style-element/mutations.window.js` | 1 / 5 |  |
| `html/semantics/document-metadata/the-style-element/no-browsing-context-import.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-style-element/no-browsing-context.html` | 1 / 2 |  |
| `html/semantics/document-metadata/the-style-element/non-parser-created-doc.html` | 0 / 1 |  |
| `html/semantics/document-metadata/the-style-element/non-parser-style.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-style-element/sheet-with-disconnected-style.html` | 0 / 2 |  |
| `html/semantics/document-metadata/the-style-element/sheet-with-empty-style.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-style-element/style-error-01.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-style-element/style-load-after-mutate.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-style-element/style-load-mutate-while-parsing.xhtml` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-style-element/style_disabled.html` | 0 / 1 |  |
| `html/semantics/document-metadata/the-style-element/style_events.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-style-element/style_load_async.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-style-element/style_load_event.html` | 0 / 1 |  |
| `html/semantics/document-metadata/the-style-element/style_media.html` | 0 / 1 |  |
| `html/semantics/document-metadata/the-style-element/style_media_change.html` | 0 / 2 |  |
| `html/semantics/document-metadata/the-style-element/style_non_matching_media.html` | 0 / 1 |  |
| `html/semantics/document-metadata/the-style-element/style_type_change.html` | 0 / 3 |  |
| `html/semantics/document-metadata/the-style-element/style_type_html.html` | 3 / 5 |  |
| `html/semantics/document-metadata/the-style-element/style_type_svg.svg` | 2 / 5 |  |
| `html/semantics/document-metadata/the-style-element/tentative/style-element-basic-import.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-style-element/tentative/style-element-csp-allowed.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-style-element/tentative/style-element-csp-blocked.html` | 1 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-style-element/tentative/style-element-csp-both-blocked.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-style-element/tentative/style-element-csp-nonce-allowed.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-style-element/tentative/style-element-csp-nonce-script-only.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-style-element/tentative/style-element-csp-nonce-style-only.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-style-element/tentative/style-element-csp-script-src-allowed.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-style-element/tentative/style-element-csp-script-src-blocked.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-style-element/tentative/style-element-csp-style-src-blocked-script-src-allowed.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-style-element/tentative/style-element-fixed.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/document-metadata/the-title-element/title.text-01.html` | 0 / 1 |  |
| `html/semantics/document-metadata/the-title-element/title.text-02.xhtml` | 0 / 1 |  |
| `html/semantics/document-metadata/the-title-element/title.text-03.html` | 0 / 11 |  |
| `html/semantics/document-metadata/the-title-element/title.text-04.xhtml` | 0 / 11 |  |
| `html/semantics/edits/the-del-element/del_effect.html` | 0 / 1 |  |
| `html/semantics/edits/the-ins-element/ins_effect.html` | 0 / 1 |  |
| `html/semantics/embedded-content/bfcache/embedded-html.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/bfcache/embedded-img.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/bfcache/embedded-js.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/bfcache/embedded-mp4.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/bfcache/embedded-not-found.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/bfcache/embedded-type-only.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/image-maps/image-map-processing-model/hash-name-reference-dynamic.html` | 1 / 8 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/image-maps/image-map-processing-model/hash-name-reference.html` | 0 / 168 | harnais : 6 duplicate test names: "HTML (standards) IMG usemap=undefined", "HTML (standards) OBJECT usemap=undefined", "HTML (quir |
| `html/semantics/embedded-content/media-elements/audio_loop_base.html` | 0 / 2 | harnais : Unhandled rejection: lecture non prise en charge |
| `html/semantics/embedded-content/media-elements/audio_loop_seek_to_eos.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/audio_volume_check.html` | 4 / 6 |  |
| `html/semantics/embedded-content/media-elements/autoplay-allowed-by-permissions-policy-attribute-redirect-on-load.https.sub.html` | 0 / 2 |  |
| `html/semantics/embedded-content/media-elements/autoplay-allowed-by-permissions-policy-attribute.https.sub.html` | 0 / 2 |  |
| `html/semantics/embedded-content/media-elements/autoplay-allowed-by-permissions-policy.https.sub.html` | 0 / 3 |  |
| `html/semantics/embedded-content/media-elements/autoplay-default-permissions-policy.https.sub.html` | 0 / 3 |  |
| `html/semantics/embedded-content/media-elements/autoplay-disabled-by-permissions-policy.https.sub.html` | 0 / 3 |  |
| `html/semantics/embedded-content/media-elements/autoplay-supported-by-permissions-policy.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/autoplay-with-broken-track.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/controlsList.tentative.html` | 0 / 2 |  |
| `html/semantics/embedded-content/media-elements/error-codes/error.html` | 0 / 6 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/error-sequence.html` | 0 / 10 |  |
| `html/semantics/embedded-content/media-elements/event_canplay.html` | 2 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/event_canplay_noautoplay.html` | 2 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/event_canplaythrough.html` | 2 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/event_canplaythrough_noautoplay.html` | 2 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/event_loadeddata.html` | 2 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/event_loadeddata_noautoplay.html` | 2 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/event_loadedmetadata.html` | 2 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/event_loadedmetadata_noautoplay.html` | 2 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/event_loadstart.html` | 2 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/event_loadstart_noautoplay.html` | 2 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/event_order_canplay_canplaythrough.html` | 2 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/event_order_canplay_playing.html` | 2 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/event_order_durationchange_resize_loadedmetadata.html` | 2 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/event_order_loadedmetadata_loadeddata.html` | 2 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/event_order_loadstart_progress.html` | 2 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/event_pause.html` | 2 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/event_pause_noautoplay.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/event_play.html` | 2 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/event_play_noautoplay.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/event_playing.html` | 2 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/event_playing_noautoplay.html` | 4 / 8 | harnais : Unhandled rejection: lecture non prise en charge |
| `html/semantics/embedded-content/media-elements/event_progress.html` | 2 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/event_progress_noautoplay.html` | 2 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/event_timeupdate.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/event_timeupdate_noautoplay.html` | 4 / 8 | harnais : Unhandled rejection: lecture non prise en charge |
| `html/semantics/embedded-content/media-elements/event_volumechange.html` | 10 / 18 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/historical.html` | 53 / 54 |  |
| `html/semantics/embedded-content/media-elements/interfaces/HTMLElement/HTMLMediaElement/addTextTrack.html` | 2 / 11 |  |
| `html/semantics/embedded-content/media-elements/interfaces/HTMLElement/HTMLMediaElement/crossOrigin.html` | 1 / 11 |  |
| `html/semantics/embedded-content/media-elements/interfaces/HTMLElement/HTMLMediaElement/textTracks.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/interfaces/HTMLElement/HTMLTrackElement/default.html` | 0 / 7 |  |
| `html/semantics/embedded-content/media-elements/interfaces/HTMLElement/HTMLTrackElement/kind.html` | 0 / 20 |  |
| `html/semantics/embedded-content/media-elements/interfaces/HTMLElement/HTMLTrackElement/label.html` | 0 / 11 |  |
| `html/semantics/embedded-content/media-elements/interfaces/HTMLElement/HTMLTrackElement/readyState.html` | 0 / 2 |  |
| `html/semantics/embedded-content/media-elements/interfaces/HTMLElement/HTMLTrackElement/src.html` | 9 / 11 |  |
| `html/semantics/embedded-content/media-elements/interfaces/HTMLElement/HTMLTrackElement/srclang.html` | 0 / 11 |  |
| `html/semantics/embedded-content/media-elements/interfaces/HTMLElement/HTMLTrackElement/track.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/interfaces/TextTrack/activeCues.html` | 0 / 0 | harnais : TypeError: .addTextTrack n'est pas une fonction (c'est undefined) |
| `html/semantics/embedded-content/media-elements/interfaces/TextTrack/addCue.html` | 0 / 5 |  |
| `html/semantics/embedded-content/media-elements/interfaces/TextTrack/constants.html` | 0 / 0 | harnais : TypeError: .addTextTrack n'est pas une fonction (c'est undefined) |
| `html/semantics/embedded-content/media-elements/interfaces/TextTrack/cues.html` | 0 / 5 |  |
| `html/semantics/embedded-content/media-elements/interfaces/TextTrack/kind.html` | 0 / 3 |  |
| `html/semantics/embedded-content/media-elements/interfaces/TextTrack/label.html` | 0 / 0 | harnais : TypeError: .addTextTrack n'est pas une fonction (c'est undefined) |
| `html/semantics/embedded-content/media-elements/interfaces/TextTrack/language.html` | 0 / 0 | harnais : TypeError: .addTextTrack n'est pas une fonction (c'est undefined) |
| `html/semantics/embedded-content/media-elements/interfaces/TextTrack/mode.html` | 0 / 2 |  |
| `html/semantics/embedded-content/media-elements/interfaces/TextTrack/oncuechange.html` | 0 / 0 | harnais : TypeError: .addTextTrack n'est pas une fonction (c'est undefined) |
| `html/semantics/embedded-content/media-elements/interfaces/TextTrack/removeCue.html` | 0 / 2 |  |
| `html/semantics/embedded-content/media-elements/interfaces/TextTrackCue/constructor.html` | 0 / 2 |  |
| `html/semantics/embedded-content/media-elements/interfaces/TextTrackCue/endTime.html` | 0 / 0 | harnais : TypeError: .addTextTrack n'est pas une fonction (c'est undefined) |
| `html/semantics/embedded-content/media-elements/interfaces/TextTrackCue/id.html` | 0 / 0 | harnais : TypeError: .addTextTrack n'est pas une fonction (c'est undefined) |
| `html/semantics/embedded-content/media-elements/interfaces/TextTrackCue/onenter.html` | 0 / 0 | harnais : ReferenceError: VTTCue n'est pas défini |
| `html/semantics/embedded-content/media-elements/interfaces/TextTrackCue/onexit.html` | 0 / 0 | harnais : ReferenceError: VTTCue n'est pas défini |
| `html/semantics/embedded-content/media-elements/interfaces/TextTrackCue/pauseOnExit.html` | 0 / 0 | harnais : TypeError: .addTextTrack n'est pas une fonction (c'est undefined) |
| `html/semantics/embedded-content/media-elements/interfaces/TextTrackCue/startTime.html` | 0 / 0 | harnais : TypeError: .addTextTrack n'est pas une fonction (c'est undefined) |
| `html/semantics/embedded-content/media-elements/interfaces/TextTrackCue/track.html` | 0 / 0 | harnais : TypeError: .addTextTrack n'est pas une fonction (c'est undefined) |
| `html/semantics/embedded-content/media-elements/interfaces/TextTrackCueList/getCueById.html` | 0 / 4 |  |
| `html/semantics/embedded-content/media-elements/interfaces/TextTrackCueList/getter.html` | 0 / 0 | harnais : TypeError: .addTextTrack n'est pas une fonction (c'est undefined) |
| `html/semantics/embedded-content/media-elements/interfaces/TextTrackCueList/length.html` | 0 / 0 | harnais : TypeError: .addTextTrack n'est pas une fonction (c'est undefined) |
| `html/semantics/embedded-content/media-elements/interfaces/TextTrackList/getTrackById.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/interfaces/TextTrackList/getter.html` | 0 / 0 | harnais : TypeError: .addTextTrack n'est pas une fonction (c'est undefined) |
| `html/semantics/embedded-content/media-elements/interfaces/TextTrackList/length.html` | 0 / 0 | harnais : TypeError: .addTextTrack n'est pas une fonction (c'est undefined) |
| `html/semantics/embedded-content/media-elements/interfaces/TextTrackList/onaddtrack.html` | 0 / 2 |  |
| `html/semantics/embedded-content/media-elements/interfaces/TextTrackList/onremovetrack.html` | 0 / 2 |  |
| `html/semantics/embedded-content/media-elements/interfaces/TrackEvent/constructor.html` | 0 / 2 |  |
| `html/semantics/embedded-content/media-elements/interfaces/TrackEvent/createEvent.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/loading-the-media-resource/autoplay-overrides-preload.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/loading-the-media-resource/load-events-networkState.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/loading-the-media-resource/load-removes-queued-error-event.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/loading-the-media-resource/media-blob-url-revoked.window.js` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/loading-the-media-resource/resource-selection-candidate-insert-before.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/loading-the-media-resource/resource-selection-candidate-moved.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/loading-the-media-resource/resource-selection-candidate-remove-addEventListener.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/loading-the-media-resource/resource-selection-candidate-remove-onerror.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/loading-the-media-resource/resource-selection-currentSrc.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/loading-the-media-resource/resource-selection-encoding-parse.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/loading-the-media-resource/resource-selection-invoke-audio-constructor.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/loading-the-media-resource/resource-selection-invoke-insert-into-iframe.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/loading-the-media-resource/resource-selection-invoke-insert-source-networkState.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/loading-the-media-resource/resource-selection-invoke-insert-source-not-in-document.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/loading-the-media-resource/resource-selection-invoke-insert-source.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/loading-the-media-resource/resource-selection-invoke-move-source-audio.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/loading-the-media-resource/resource-selection-invoke-move-source-not-in-document.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/loading-the-media-resource/resource-selection-invoke-move-source-same-parent.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/loading-the-media-resource/resource-selection-invoke-move-source.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/loading-the-media-resource/resource-selection-invoke-pause-networkState.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/loading-the-media-resource/resource-selection-invoke-play.html` | 1 / 1 | harnais : Unhandled rejection: lecture non prise en charge |
| `html/semantics/embedded-content/media-elements/loading-the-media-resource/resource-selection-invoke-remove-from-document-networkState.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/loading-the-media-resource/resource-selection-invoke-set-src-networkState.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/loading-the-media-resource/resource-selection-invoke-set-src-not-in-document.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/loading-the-media-resource/resource-selection-invoke-set-src.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/loading-the-media-resource/resource-selection-pointer-control.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/loading-the-media-resource/resource-selection-pointer-insert-br.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/loading-the-media-resource/resource-selection-pointer-insert-source.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/loading-the-media-resource/resource-selection-pointer-insert-text.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/loading-the-media-resource/resource-selection-pointer-remove-source-after.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/loading-the-media-resource/resource-selection-pointer-remove-source.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/loading-the-media-resource/resource-selection-pointer-remove-text.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/loading-the-media-resource/resource-selection-source-media-env-change.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/loading-the-media-resource/resource-selection-source-media.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/location-of-the-media-resource/currentSrc.html` | 0 / 18 |  |
| `html/semantics/embedded-content/media-elements/media-playback-while-not-visible-permission-policy/cant-play-if-hidden.tentative.sub.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/media-playback-while-not-visible-permission-policy/cant-play-when-created-hidden.tentative.sub.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/media-playback-while-not-visible-permission-policy/pause-when-hidden.tentative.sub.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/media-playback-while-not-visible-permission-policy/viewport.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/media_fragment_seek.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/mime-types/canPlayType.html` | 19 / 60 |  |
| `html/semantics/embedded-content/media-elements/networkState_during_loadstart.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/networkState_during_progress.html` | 2 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/no-autoplay-audio-history-back-does-not-play.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/offsets-into-the-media-resource/currentTime-move-within-document.html` | 0 / 2 | harnais : Unhandled rejection: lecture non prise en charge |
| `html/semantics/embedded-content/media-elements/offsets-into-the-media-resource/currentTime.html` | 1 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/paused_false_during_play.html` | 2 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/paused_true_during_pause.html` | 2 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/played-loop.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/playing-the-media-resource/fragmented-mp4-end.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/playing-the-media-resource/loop-from-ended.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/playing-the-media-resource/pause-move-to-other-document.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/playing-the-media-resource/pause-move-within-document.html` | 0 / 2 | harnais : Unhandled rejection: lecture non prise en charge |
| `html/semantics/embedded-content/media-elements/playing-the-media-resource/pause-remove-from-document-different-load.html` | 0 / 2 | harnais : Unhandled rejection: lecture non prise en charge |
| `html/semantics/embedded-content/media-elements/playing-the-media-resource/pause-remove-from-document-networkState.html` | 0 / 1 | harnais : Unhandled rejection: lecture non prise en charge |
| `html/semantics/embedded-content/media-elements/playing-the-media-resource/pause-remove-from-document.html` | 0 / 2 | harnais : Unhandled rejection: lecture non prise en charge |
| `html/semantics/embedded-content/media-elements/playing-the-media-resource/play-in-detached-document.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/playing-the-media-resource/playbackRate.html` | 0 / 7 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/preserves-pitch.html` | 0 / 9 |  |
| `html/semantics/embedded-content/media-elements/ready-states/autoplay-hidden.optional.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/ready-states/autoplay-with-slow-text-tracks.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/ready-states/autoplay.html` | 0 / 20 | harnais : Unhandled rejection: lecture non prise en charge |
| `html/semantics/embedded-content/media-elements/readyState_during_canplay.html` | 2 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/readyState_during_canplaythrough.html` | 2 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/readyState_during_loadeddata.html` | 2 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/readyState_during_loadedmetadata.html` | 2 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/readyState_during_playing.html` | 2 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/readyState_initial.html` | 0 / 2 |  |
| `html/semantics/embedded-content/media-elements/replay.html` | 4 / 12 | harnais : Unhandled rejection: lecture non prise en charge |
| `html/semantics/embedded-content/media-elements/seeking/seek-to-currentTime.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/seeking/seek-to-max-value.htm` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/seeking/seek-to-negative-time.htm` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/src_object_blob.html` | 0 / 2 | harnais : Unhandled rejection: lecture non prise en charge |
| `html/semantics/embedded-content/media-elements/track/track-element/cloneNode.html` | 0 / 4 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/003.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/004.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/005.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/006.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/007.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/008.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/009.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/010.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/011.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/012.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/013.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/014.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/015.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/016.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/017.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/018.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/019.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/020.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/021.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/022.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/023.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/024.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/025.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/026.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/027.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/028.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/029.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/030.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/031.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/032.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/033.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/034.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/035.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/036.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/037.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/038.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/039.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/040.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/041.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/042.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/043.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/044.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/045.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/cors/046.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/no-cuechange-before-play.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/src-clear-cues.html` | 0 / 3 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/src-empty-string.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-active-cues.html` | 0 / 2 | harnais : Unhandled rejection: lecture non prise en charge |
| `html/semantics/embedded-content/media-elements/track/track-element/track-add-remove-cue.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-add-track.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-addtrack-kind.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-api-texttracks.html` | 0 / 3 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-blob-url-revoked.window.js` | 0 / 3 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-change-event.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-css-cue-pseudo-class.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-cue-empty.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-cue-inline.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-cue-mutable-fragment.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-cue-mutable.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-cue-negative-duration.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-cue-negative-timestamp-events.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-cue-negative-timestamp.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-cue-order.html` | 0 / 6 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-cue-rendering-empty-cue.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-cues-add-new-track.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-cues-cuechange-dynamically-created-track-element.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-cues-cuechange.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-cues-enter-exit.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-cues-enter-seeking.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-cues-missed-no-immediate-events.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-cues-missed.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-cues-pause-on-exit.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-cues-seeking.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-cues-sorted-before-dispatch.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-data-url.html` | 0 / 3 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-default-attribute.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-default-load-timer-during-parser-block.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-delete-during-setup.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-disabled-addcue.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-disabled.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-element-dom-change.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-element-src-aborted-load.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-element-src-change-error.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-element-src-change.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-id.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-insert-after-load.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-large-timestamp.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-load-error-readyState.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-load-from-element-readyState.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-load-from-src-readyState.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-mode-disabled.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-mode-not-changed-by-new-track.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-mode-triggers-loading.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-mode.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-node-add-remove.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-remove-active-cue.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-remove-by-setting-innerHTML.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-remove-insert-ready-state.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-remove-track-inband.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-remove-track.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-selection-metadata.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-selection-task-order.html` | 0 / 1 | harnais : Unhandled rejection: lecture non prise en charge |
| `html/semantics/embedded-content/media-elements/track/track-element/track-text-track-cue-list.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-texttracks.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-webvtt-align-positioning.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-webvtt-align-text-line-position.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-webvtt-alignment.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-webvtt-blank-lines.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-webvtt-bom.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-webvtt-class-markup.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-webvtt-cue-identifiers.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-webvtt-cue-no-id.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-webvtt-cue-recovery.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-webvtt-cue-size-align.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-webvtt-cue-size.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-webvtt-degenerate-cues.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-webvtt-empty-cue.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-webvtt-entities.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-webvtt-header-checks.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-webvtt-header-comment.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-webvtt-interspersed-non-cue.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-webvtt-line-position.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-webvtt-magic-header.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-webvtt-markup.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-webvtt-newlines.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-webvtt-no-timings.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-webvtt-positioning.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-webvtt-settings.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-webvtt-timestamp.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-webvtt-timings-hour.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-webvtt-timings-no-hours.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-webvtt-timings-whitespace.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-webvtt-unsupported-markup.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-webvtt-utf8.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/track/track-element/track-webvtt-valign.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/track-webvtt-voice.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/track/track-element/vtt-cue-float-precision.html` | 0 / 1 |  |
| `html/semantics/embedded-content/media-elements/user-interface/muted-playbackrate.tentative.html` | 0 / 6 |  |
| `html/semantics/embedded-content/media-elements/user-interface/muted.html` | 28 / 51 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/media-elements/video_loop_base.html` | 0 / 2 | harnais : Unhandled rejection: lecture non prise en charge |
| `html/semantics/embedded-content/media-elements/video_volume_check.html` | 4 / 6 |  |
| `html/semantics/embedded-content/media-elements/volume_nonfinite.html` | 0 / 6 |  |
| `html/semantics/embedded-content/media-elements/volumechange-ratechange-queue-task.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-area-element/area-coords.html` | 10 / 32 |  |
| `html/semantics/embedded-content/the-area-element/area-image-cursor.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-area-element/area-processing.html` | 7 / 14 |  |
| `html/semantics/embedded-content/the-area-element/area-shape.html` | 4 / 17 |  |
| `html/semantics/embedded-content/the-area-element/area-stringifier.html` | 9 / 11 |  |
| `html/semantics/embedded-content/the-audio-element/audio-loading-autoplay-deferred.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-audio-element/audio-loading-eager.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-audio-element/audio-loading-lazy-autoplay-when-visible.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-audio-element/audio-loading-lazy-in-scroller.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-audio-element/audio-loading-lazy-in-viewport.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-audio-element/audio-loading-lazy-source-deferred.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-audio-element/audio-loading-lazy-to-eager.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-audio-element/audio-loading-lazy-window-onload.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-audio-element/audio-loading-load-deferred.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-audio-element/audio-loading-load-preload-auto-deferred.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-audio-element/audio-loading-load-preload-metadata-deferred.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-audio-element/audio_constructor.html` | 5 / 11 |  |
| `html/semantics/embedded-content/the-canvas-element/2d-getcontext-options.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-canvas-element/canvas-descendants-focusability-002.html` | 0 / 5 |  |
| `html/semantics/embedded-content/the-canvas-element/canvas-descendants-focusability-003.tentative.html` | 5 / 10 |  |
| `html/semantics/embedded-content/the-canvas-element/canvas-descendants-focusability-004.tentative.html` | 0 / 5 |  |
| `html/semantics/embedded-content/the-canvas-element/canvas-descendants-focusability-005.html` | 6 / 11 |  |
| `html/semantics/embedded-content/the-canvas-element/canvas-descendants-focusability-006.html` | 1 / 2 |  |
| `html/semantics/embedded-content/the-canvas-element/context.arguments.missing.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-canvas-element/security.dataURI.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-canvas-element/security.drawImage.canvas.cross.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-canvas-element/security.drawImage.canvas.redirect.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-canvas-element/security.drawImage.image.cross.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-canvas-element/security.drawImage.image.redirect.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-canvas-element/security.pattern.canvas.fillStyle.cross.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-canvas-element/security.pattern.canvas.fillStyle.redirect.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-canvas-element/security.pattern.canvas.strokeStyle.cross.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-canvas-element/security.pattern.canvas.strokeStyle.redirect.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-canvas-element/security.pattern.canvas.timing.cross.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-canvas-element/security.pattern.canvas.timing.redirect.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-canvas-element/security.pattern.create.cross.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-canvas-element/security.pattern.create.redirect.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-canvas-element/security.pattern.cross.cross.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-canvas-element/security.pattern.cross.redirect.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-canvas-element/security.pattern.fillStyle.sub.html` | 0 / 18 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-canvas-element/security.pattern.image.fillStyle.cross.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-canvas-element/security.pattern.image.fillStyle.redirect.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-canvas-element/security.pattern.image.strokeStyle.cross.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-canvas-element/security.pattern.image.strokeStyle.redirect.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-canvas-element/security.reset.cross.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-canvas-element/security.reset.redirect.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-canvas-element/security.resize.cross.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-canvas-element/security.resize.redirect.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-canvas-element/toBlob-cross-realm-callback-report-exception.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-canvas-element/toBlob.jpeg.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-canvas-element/toDataURL.jpeg.primarycolours.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-canvas-element/toDataURL.png.complexcolours.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-canvas-element/toDataURL.png.primarycolours.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-embed-element/document-getters-return-null-for-cross-origin.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-embed-element/embed-allowed-schemas.sub.window.js` | 0 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-embed-element/embed-change-src-2.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-embed-element/embed-change-src.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-embed-element/embed-dimension.html` | 0 / 2 |  |
| `html/semantics/embedded-content/the-embed-element/embed-document-under-content-visibility-focus.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-embed-element/embed-document-under-content-visibility-gbcr.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-embed-element/embed-document.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-embed-element/embed-in-object-fallback-2.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-embed-element/embed-in-object-fallback.html` | 1 / 2 |  |
| `html/semantics/embedded-content/the-embed-element/embed-javascript-url.html` | 1 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-embed-element/embed-network-error.sub.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-embed-element/embed-svg-navigation-resets-size.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-frame-element/document-getters-return-null-for-cross-origin.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-frame-element/frame-javascript-url-load-event.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/change_parentage.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/cross-origin-to-whom-part-2.window.js` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/cross-origin-to-whom.window.js` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/cross-site/iframe-loading-lazy-cross-site.sub.html` | 0 / 0 | le harnais n'a pas terminé |
| `html/semantics/embedded-content/the-iframe-element/cross-site/iframe-loading-lazy-in-scroller-cross-site.sub.html` | 0 / 0 | le harnais n'a pas terminé |
| `html/semantics/embedded-content/the-iframe-element/cross-site/iframe-loading-lazy-in-scroller-far-cross-site.sub.html` | 0 / 0 | le harnais n'a pas terminé |
| `html/semantics/embedded-content/the-iframe-element/cross-site/iframe-loading-lazy-multiple-queued-navigations-cross-site.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/cross-site/iframe-loading-lazy-nested-cross-site.sub.html` | 0 / 0 | le harnais n'a pas terminé |
| `html/semantics/embedded-content/the-iframe-element/cross-site/iframe-loading-lazy-siblings-cross-site.sub.html` | 0 / 0 | le harnais n'a pas terminé |
| `html/semantics/embedded-content/the-iframe-element/cross_origin_parentage.sub.html` | 0 / 3 |  |
| `html/semantics/embedded-content/the-iframe-element/document-getters-return-null-for-cross-origin.html` | 0 / 2 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe-allow.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/iframe-allowfullscreen.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/iframe-append-to-child-document.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe-first-load-canceled-second-load-blank.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/iframe-loading-lazy-adopt-lazy-iframe-from-script-disabled-document.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe-loading-lazy-base-url-2.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/iframe-loading-lazy-base-url-3.html` | 0 / 0 | harnais : Uncaught ReferenceError: below_viewport_iframe est utilisé avant son initialisation |
| `html/semantics/embedded-content/the-iframe-element/iframe-loading-lazy-base-url.html` | 0 / 0 | harnais : Uncaught ReferenceError: below_viewport_iframe est utilisé avant son initialisation |
| `html/semantics/embedded-content/the-iframe-element/iframe-loading-lazy-history-pushState.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/iframe-loading-lazy-history-replaceState.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/iframe-loading-lazy-in-script-disabled-iframe.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe-loading-lazy-in-scroller-far.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe-loading-lazy-in-scroller-horizontal-far.html` | 1 / 1 | harnais : Uncaught ReferenceError: iframe_onload n'est pas défini |
| `html/semantics/embedded-content/the-iframe-element/iframe-loading-lazy-in-scroller-horizontal.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe-loading-lazy-load-event.html` | 0 / 2 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe-loading-lazy-multiple-queued-navigations.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe-loading-lazy-multiple-times.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe-loading-lazy-nav-form-submit.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe-loading-lazy-nav-link-click-fragment.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe-loading-lazy-nav-link-click.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe-loading-lazy-nav-location-assign.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe-loading-lazy-nav-location-replace-cross-origin.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe-loading-lazy-nav-location-replace-set-src.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe-loading-lazy-nav-location-replace.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe-loading-lazy-nav-meta-refresh.optional.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe-loading-lazy-nav-navigation-navigate.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/iframe-loading-lazy-nav-window-open.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe-loading-lazy-referrerpolicy-change.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/iframe-loading-lazy-reload-navigation-reload.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/iframe-loading-lazy-to-eager-srcdoc.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe-loading-lazy-to-eager.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe-loading-lazy.html` | 0 / 0 | le harnais n'a pas terminé |
| `html/semantics/embedded-content/the-iframe-element/iframe-network-error.sub.html` | 2 / 4 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe-src-encoding.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe-synchronously-discard.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe_javascript_url_01.htm` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/iframe_javascript_url_form_submission.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe_javascript_url_in_src.htm` | 14 / 15 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe_javascript_url_initial_insertion.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe_javascript_url_loading_lazy.htm` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe_javascript_url_not_about_blank.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe_javascript_url_remove_srcdoc.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe_javascript_url_xhr.html` | 0 / 4 | harnais : Uncaught { message: 'assert_true: script should have ran expected true got false', stack: 'Error' } |
| `html/semantics/embedded-content/the-iframe-element/iframe_navigate_ancestor-1.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/iframe_navigate_javascript_url.htm` | 1 / 2 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe_remove_src.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe_sandbox_allow_script.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/iframe_sandbox_allow_top_navigation-1.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/iframe_sandbox_allow_top_navigation-2.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/iframe_sandbox_allow_top_navigation-3.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/iframe_sandbox_allow_top_navigation_by_user_activation_with_user_gesture.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe_sandbox_allow_top_navigation_by_user_activation_without_user_gesture.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe_sandbox_anchor_download_allow_downloads.tentative.html` | 0 / 6 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/iframe_sandbox_anchor_download_block_downloads.tentative.html` | 0 / 5 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe_sandbox_block_modals-1.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe_sandbox_block_modals-4.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe_sandbox_navigate_ancestor-1.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/iframe_sandbox_navigate_ancestor-2.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/iframe_sandbox_navigate_descendants.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/iframe_sandbox_navigate_history_go_back-2.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/iframe_sandbox_navigate_history_go_forward.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/iframe_sandbox_navigate_itself.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/iframe_sandbox_navigate_other_frame_popup.sub.html` | 0 / 1 | harnais : Test named 'Sandboxed iframe can not navigate other frame's popup' specified 1 'cleanup' function, and 1 failed. |
| `html/semantics/embedded-content/the-iframe-element/iframe_sandbox_navigation_download_allow_downloads.sub.tentative.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/iframe_sandbox_navigation_download_block_downloads.sub.tentative.html` | 0 / 2 |  |
| `html/semantics/embedded-content/the-iframe-element/iframe_sandbox_popups_escaping-1.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/iframe_sandbox_popups_escaping-2.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/iframe_sandbox_popups_escaping-3.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/iframe_sandbox_popups_nonescaping-1.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/iframe_sandbox_popups_nonescaping-2.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/iframe_sandbox_popups_nonescaping-3.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/iframe_sandbox_window_open_download_allow_downloads.tentative.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/iframe_sandbox_window_open_download_block_downloads.tentative.html` | 0 / 3 |  |
| `html/semantics/embedded-content/the-iframe-element/move-node-local-root-events-still-fire.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/move-node-local-root.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/move_iframe_in_dom_01.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/move_iframe_in_dom_02.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/move_iframe_in_dom_03.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/move_iframe_in_dom_04.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/same-origin-contextual-fragment.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/same_origin_parentage.html` | 0 / 3 |  |
| `html/semantics/embedded-content/the-iframe-element/sandbox-ascii-case-insensitive.html` | 0 / 2 |  |
| `html/semantics/embedded-content/the-iframe-element/sandbox-inherit-to-blank-document-unsandboxed-frame.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/sandbox-inherit-to-blank-document-unsandboxed.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/sandbox-top-close.tentative.sub.window.js` | 0 / 3 |  |
| `html/semantics/embedded-content/the-iframe-element/sandbox-top-navigation-child-cross-origin-frame.tentative.sub.window.js` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/sandbox-top-navigation-child-delivered-both.tentative.sub.window.js` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/sandbox-top-navigation-child-delivered.tentative.sub.window.js` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/sandbox-top-navigation-child-unsandboxed.tentative.sub.window.js` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/sandbox-top-navigation-grandchild-allow-same-origin.tentative.sub.window.js` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/sandbox-top-navigation-grandchild-frame-allow-top.tentative.sub.window.js` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/sandbox-top-navigation-grandchild-sandboxed-cross-origin-parent.tentative.sub.window.js` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/sandbox-top-navigation-grandchild-sandboxed.tentative.sub.window.js` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/sandbox-top-navigation-grandchild-unsandboxed-cross-origin-parent.tentative.sub.window.js` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/sandbox-top-navigation-grandchild-unsandboxed-inherit.tentative.sub.window.js` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/sandbox_001.htm` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/sandbox_002.htm` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/sandbox_005.htm` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/sandbox_011.htm` | 0 / 8 |  |
| `html/semantics/embedded-content/the-iframe-element/sandbox_012.htm` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/sandbox_013.htm` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/sandbox_014.htm` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/sandbox_015.htm` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/sandbox_016.htm` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/sandbox_017.htm` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/sandbox_018.htm` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/sandbox_019.htm` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/sandbox_023.htm` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/sandbox_024.htm` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/sandbox_025.htm` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/sandbox_026.htm` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/sandbox_027.htm` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/sandbox_028.htm` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/sandbox_029.htm` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/sandbox_030.htm` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/sandbox_031.htm` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/sandbox_032.htm` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/src-repeated-in-ancestor.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/srcdoc-attribute-reset.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-iframe-element/srcdoc-lifecycle-crash-crbug-1472607.https.html` | 0 / 2 |  |
| `html/semantics/embedded-content/the-iframe-element/srcdoc_change_hash.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-iframe-element/srcdoc_process_attributes.html` | 1 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/404-response-with-actual-image-data.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/Image-constructor.html` | 3 / 5 |  |
| `html/semantics/embedded-content/the-img-element/adoption.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/already-loaded-image-sync-width.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/attr-img-fetchpriority.html` | 0 / 2 |  |
| `html/semantics/embedded-content/the-img-element/available-images-onload.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/below-viewport-image-loading-lazy-load-event.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/current-pixel-density/basic.html` | 3 / 14 |  |
| `html/semantics/embedded-content/the-img-element/current-pixel-density/error.html` | 0 / 7 |  |
| `html/semantics/embedded-content/the-img-element/currentSrc-blob-cache.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/data-url.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/decode/image-decode-iframe.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/decode/image-decode-image-document.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/decode/image-decode-path-changes-svg.tentative.html` | 0 / 6 |  |
| `html/semantics/embedded-content/the-img-element/decode/image-decode-path-changes.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/decode/image-decode-picture.html` | 5 / 8 |  |
| `html/semantics/embedded-content/the-img-element/decode/image-decode-svg.tentative.html` | 0 / 13 |  |
| `html/semantics/embedded-content/the-img-element/decode/image-decode-with-quick-attach-svg.tentative.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-img-element/decode/image-decode-with-quick-attach.html` | 4 / 9 |  |
| `html/semantics/embedded-content/the-img-element/decode/image-decode.html` | 8 / 15 |  |
| `html/semantics/embedded-content/the-img-element/delay-load-event-detached.html` | 2 / 3 |  |
| `html/semantics/embedded-content/the-img-element/delay-load-event-until-move-to-empty-source.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-img-element/delay-load-event.html` | 2 / 3 |  |
| `html/semantics/embedded-content/the-img-element/dimension-getter-transform.html` | 0 / 2 |  |
| `html/semantics/embedded-content/the-img-element/disconnected-image-loading-lazy.html` | 1 / 3 |  |
| `html/semantics/embedded-content/the-img-element/empty-src-no-current-request.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/environment-changes/picture-resize-complete.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/environment-changes/viewport-change.html` | 0 / 0 | harnais : done() was called without first defining any tests |
| `html/semantics/embedded-content/the-img-element/historical-progress-event.window.js` | 1 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/image-base-url.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/image-loading-eager.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-img-element/image-loading-lazy-available.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-img-element/image-loading-lazy-base-url-2.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/image-loading-lazy-base-url.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/image-loading-lazy-below-viewport-dynamic.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/image-loading-lazy-crossorigin-change.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/image-loading-lazy-different-crossorigin.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/image-loading-lazy-empty-src.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/image-loading-lazy-in-cross-origin-iframe-001.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/image-loading-lazy-in-cross-origin-iframe-002.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/image-loading-lazy-in-scroller-horizontal.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-img-element/image-loading-lazy-in-scroller-nested-2.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-img-element/image-loading-lazy-in-scroller-nested-3.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-img-element/image-loading-lazy-in-scroller-nested-4.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-img-element/image-loading-lazy-in-scroller-nested-5.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-img-element/image-loading-lazy-in-scroller-nested.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-img-element/image-loading-lazy-in-scroller.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-img-element/image-loading-lazy-in-viewport-dynamic.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/image-loading-lazy-move-document.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-img-element/image-loading-lazy-move-into-script-disabled-iframe.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-img-element/image-loading-lazy-multicol.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-img-element/image-loading-lazy-multiple-times.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/image-loading-lazy-referrerpolicy-change.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/image-loading-lazy-srcset.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/image-loading-lazy-to-eager.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/image-loading-lazy-use-list-of-available-images.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/image-loading-lazy-zero-intersection-area.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/image-loading-lazy.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/img-complete-after-error.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-img-element/img-complete-invalid-url.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-img-element/img-picture-ancestor.html` | 1 / 4 |  |
| `html/semantics/embedded-content/the-img-element/img-src-encoding-parse.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/img-src-in-synthetic-document.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-img-element/img.complete-2.html` | 0 / 2 |  |
| `html/semantics/embedded-content/the-img-element/img.complete.html` | 8 / 19 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/invalid-src.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/invisible-image.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-img-element/list-of-available-images-does-not-coalesce-in-flight-requests.sub.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/list-of-available-images-matching.https.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/natural-size-orientation.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/naturalWidth-naturalHeight-width-height.html` | 30 / 258 |  |
| `html/semantics/embedded-content/the-img-element/nonexistent-image.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/not-rendered-dimension-getter.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/null-image-source.html` | 2 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/picture-loading-lazy.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/relevant-mutations-lazy.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/relevant-mutations.html` | 67 / 113 |  |
| `html/semantics/embedded-content/the-img-element/remove-element-and-scroll.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/reuse-img.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/scrolling-below-viewport-image-lazy-loading-in-iframe.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/sizes/implicit-sizes-ignores-width.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/sizes/parse-a-sizes-attribute-display-none.html` | 0 / 0 | harnais : done() was called without first defining any tests |
| `html/semantics/embedded-content/the-img-element/sizes/parse-a-sizes-attribute-quirks-mode.html` | 0 / 0 | harnais : done() was called without first defining any tests |
| `html/semantics/embedded-content/the-img-element/sizes/parse-a-sizes-attribute-standards-mode.html` | 0 / 0 | harnais : done() was called without first defining any tests |
| `html/semantics/embedded-content/the-img-element/sizes/parse-a-sizes-attribute-width-1000px.html` | 0 / 0 | harnais : done() was called without first defining any tests |
| `html/semantics/embedded-content/the-img-element/sizes/sizes-2d.tentative.html` | 0 / 108 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/sizes/sizes-auto-2d.tentative.html` | 0 / 20 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/sizes/sizes-auto-dynamic-attributes.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/sizes/sizes-auto-resize-observer-removes-auto.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/sizes/sizes-auto-unrendered.html` | 0 / 15 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/sizes/sizes-auto.html` | 0 / 74 |  |
| `html/semantics/embedded-content/the-img-element/source-media-outside-doc.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-img-element/source-relevant-mutations.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-img-element/srcset-natural-size-in-dynamic-iframe.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/srcset/avoid-reload-on-resize.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-img-element/srcset/parse-a-srcset-attribute.html` | 187 / 236 |  |
| `html/semantics/embedded-content/the-img-element/srcset/select-an-image-source-cached.optional.html` | 0 / 13 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/srcset/select-an-image-source-zoom-change.optional.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-img-element/srcset/select-an-image-source-zoom.html` | 0 / 4 |  |
| `html/semantics/embedded-content/the-img-element/srcset/select-an-image-source.html` | 0 / 46 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/srcset/srcset-w-reselect-on-resize.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-img-element/svg-image-subpixel-size-loads.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/svg-img-density-vs-zoom.html` | 0 / 9 |  |
| `html/semantics/embedded-content/the-img-element/svg-img-zero-height-with-viewbox.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/update-media.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/update-src-complete.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-img-element/update-the-image-data/current-request-microtask-002.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-img-element/update-the-image-data/current-request-microtask.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-img-element/update-the-image-data/fail-to-resolve.html` | 1 / 4 |  |
| `html/semantics/embedded-content/the-img-element/update-the-image-data/lazy-out-of-band-load.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/update-the-image-data/not-broken-while-load-task-scheduled.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-img-element/update-the-source-set.html` | 55 / 89 |  |
| `html/semantics/embedded-content/the-img-element/usemap-casing.html` | 7 / 8 | harnais : 1 duplicate test name: "Image with usemap of undefined should not match any of the areas" |
| `html/semantics/embedded-content/the-img-element/width-height-attribute-parsing.html` | 0 / 17 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-object-element/document-getters-return-null-for-cross-origin.html` | 0 / 2 |  |
| `html/semantics/embedded-content/the-object-element/historical.html` | 2 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-object-element/object-allowed-schemas.sub.window.js` | 0 / 6 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-object-element/object-attributes.html` | 0 / 4 |  |
| `html/semantics/embedded-content/the-object-element/object-constraint-validation.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-object-element/object-events.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-object-element/object-fallback-failed-cross-origin-navigation.sub.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-object-element/object-handler.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-object-element/object-image-click.sub.html` | 0 / 4 |  |
| `html/semantics/embedded-content/the-object-element/object-image-style-recalc-fallback.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-object-element/object-in-object-fallback-2.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-object-element/object-javascript-url.html` | 1 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-object-element/object-setcustomvalidity.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-object-element/usemap-casing.html` | 0 / 8 | harnais : 1 duplicate test name: "Object with usemap of undefined should not match any of the areas (it does not support usemap)" |
| `html/semantics/embedded-content/the-persistentwidget-element/tentative/idlharness.window.js` | 0 / 1 |  |
| `html/semantics/embedded-content/the-video-element/intrinsic_sizes.htm` | 3 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-video-element/resize-during-playback.html` | 0 / 2 |  |
| `html/semantics/embedded-content/the-video-element/video-loading-autoplay-deferred.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-video-element/video-loading-eager.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-video-element/video-loading-lazy-autoplay-when-visible.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-video-element/video-loading-lazy-below-viewport-load-event.html` | 0 / 3 |  |
| `html/semantics/embedded-content/the-video-element/video-loading-lazy-in-scroller.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-video-element/video-loading-lazy-in-viewport.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-video-element/video-loading-lazy-load-when-visible.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-video-element/video-loading-lazy-poster-when-visible.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-video-element/video-loading-lazy-resume-on-loading-attribute-change.html` | 0 / 3 |  |
| `html/semantics/embedded-content/the-video-element/video-loading-lazy-source-inserted-load-event.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-video-element/video-loading-lazy-source-poster-deferred.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-video-element/video-loading-lazy-to-eager.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-video-element/video-loading-lazy-window-onload.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-video-element/video-loading-load-deferred.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-video-element/video-loading-load-preload-auto-deferred.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-video-element/video-loading-load-preload-metadata-deferred.html` | 0 / 1 |  |
| `html/semantics/embedded-content/the-video-element/video_crash_empty_src.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-video-element/video_size_preserved_after_ended.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/embedded-content/the-video-element/video_timeupdate_on_seek.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/forms/attributes-common-to-form-controls/dirname-ltr.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/forms/attributes-common-to-form-controls/dirname-only-if-applies.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/forms/attributes-common-to-form-controls/dirname-rtl-auto.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/forms/attributes-common-to-form-controls/dirname-rtl-inherited.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/forms/attributes-common-to-form-controls/disabled-elements-01.html` | 12 / 16 |  |
| `html/semantics/forms/attributes-common-to-form-controls/formAction_document_address.html` | 2 / 8 |  |
| `html/semantics/forms/attributes-common-to-form-controls/formaction.html` | 2 / 8 |  |
| `html/semantics/forms/beforeinput.tentative.html` | 0 / 16 | harnais : délai du harnais dépassé |
| `html/semantics/forms/constraints/form-validation-checkValidity.html` | 118 / 130 |  |
| `html/semantics/forms/constraints/form-validation-reportValidity.html` | 118 / 130 |  |
| `html/semantics/forms/constraints/form-validation-validate.html` | 6 / 8 |  |
| `html/semantics/forms/constraints/form-validation-validity-patternMismatch.html` | 78 / 85 |  |
| `html/semantics/forms/constraints/form-validation-validity-rangeOverflow.html` | 47 / 49 |  |
| `html/semantics/forms/constraints/form-validation-validity-rangeUnderflow.html` | 46 / 47 |  |
| `html/semantics/forms/constraints/form-validation-validity-stepMismatch.html` | 20 / 28 |  |
| `html/semantics/forms/constraints/form-validation-validity-textarea-defaultValue.html` | 2 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/forms/constraints/form-validation-validity-typeMismatch.html` | 10 / 11 |  |
| `html/semantics/forms/constraints/form-validation-validity-valid-weekmonth.html` | 4 / 8 |  |
| `html/semantics/forms/constraints/form-validation-validity-valid.html` | 20 / 35 |  |
| `html/semantics/forms/constraints/form-validation-validity-valueMissing-weekmonth.html` | 7 / 19 |  |
| `html/semantics/forms/constraints/form-validation-validity-valueMissing.html` | 45 / 78 |  |
| `html/semantics/forms/constraints/form-validation-willValidate.html` | 70 / 73 |  |
| `html/semantics/forms/constraints/infinite_backtracking.tentative.html` | 0 / 1 |  |
| `html/semantics/forms/constraints/input-maxlength-emoji.html` | 0 / 1 |  |
| `html/semantics/forms/constraints/input-number-validity-dynamic-value-no-change.html` | 0 / 1 |  |
| `html/semantics/forms/constraints/input-pattern-dynamic-value.html` | 0 / 1 |  |
| `html/semantics/forms/constraints/radio-group-valueMissing.html` | 1 / 2 |  |
| `html/semantics/forms/constraints/radio-valueMissing.html` | 4 / 6 |  |
| `html/semantics/forms/constraints/using-form-attribute.html` | 0 / 1 |  |
| `html/semantics/forms/customizable-combobox/customizable-combobox-beforefilter.tentative.html` | 0 / 1 |  |
| `html/semantics/forms/customizable-combobox/customizable-combobox-filtered-pseudo.tentative.html` | 0 / 2 |  |
| `html/semantics/forms/customizable-combobox/customizable-combobox-keyboard-behavior.tentative.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/forms/customizable-combobox/customizable-combobox-option-attributes.html` | 0 / 1 |  |
| `html/semantics/forms/customizable-combobox/customizable-combobox-pointer-behavior.tentative.html` | 0 / 2 |  |
| `html/semantics/forms/customizable-combobox/customizable-combobox-popover-exception.tentative.html` | 0 / 1 |  |
| `html/semantics/forms/customizable-combobox/datalist-popover.tentative.html` | 0 / 1 |  |
| `html/semantics/forms/form-control-infrastructure/form-attribute-tree-mutations.html` | 0 / 7 |  |
| `html/semantics/forms/form-control-infrastructure/form.html` | 7 / 17 |  |
| `html/semantics/forms/form-control-infrastructure/form_attribute.html` | 8 / 100 |  |
| `html/semantics/forms/form-control-infrastructure/form_owner_and_table.html` | 0 / 1 |  |
| `html/semantics/forms/form-control-infrastructure/form_owner_and_table_2.html` | 0 / 1 |  |
| `html/semantics/forms/form-control-infrastructure/form_owner_and_table_3.html` | 0 / 1 |  |
| `html/semantics/forms/form-control-infrastructure/parser-reparented-open-element.html` | 1 / 2 |  |
| `html/semantics/forms/form-submission-0/FormDataEvent.window.js` | 0 / 2 |  |
| `html/semantics/forms/form-submission-0/SubmitEvent.window.js` | 5 / 7 | harnais : délai du harnais dépassé |
| `html/semantics/forms/form-submission-0/action-encoding.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/forms/form-submission-0/constructing-form-data-set.html` | 2 / 13 | harnais : délai du harnais dépassé |
| `html/semantics/forms/form-submission-0/form-data-set-empty-file.window.js` | 0 / 4 |  |
| `html/semantics/forms/form-submission-0/form-data-set-usv.html` | 1 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/forms/form-submission-0/form-double-submit-2.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/forms/form-submission-0/form-double-submit-3.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/forms/form-submission-0/form-double-submit-default-action.html` | 0 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/forms/form-submission-0/form-double-submit-input-type-change.html` | 0 / 20 | harnais : délai du harnais dépassé |
| `html/semantics/forms/form-submission-0/form-double-submit-multiple-targets.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/forms/form-submission-0/form-double-submit-preventdefault-click.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/forms/form-submission-0/form-double-submit-preventdefault.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/forms/form-submission-0/form-double-submit-requestsubmit.html` | 12 / 72 | harnais : Uncaught RangeError: pile d'appels dépassée |
| `html/semantics/forms/form-submission-0/form-double-submit-to-different-origin-frame.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/forms/form-submission-0/form-double-submit.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/forms/form-submission-0/form-submission-algorithm.html` | 4 / 24 | harnais : Uncaught RangeError: pile d'appels dépassée |
| `html/semantics/forms/form-submission-0/form-submit-action.html` | 0 / 10 | harnais : délai du harnais dépassé |
| `html/semantics/forms/form-submission-0/form-submit-iframe-then-location-navigate.html` | 0 / 1 |  |
| `html/semantics/forms/form-submission-0/getactionurl.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/forms/form-submission-0/implicit-submission.optional.html` | 0 / 3 |  |
| `html/semantics/forms/form-submission-0/jsurl-form-submit.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/forms/form-submission-0/jsurl-navigation-then-form-submit.html` | 0 / 1 |  |
| `html/semantics/forms/form-submission-0/multipart-formdata.window.js` | 0 / 62 |  |
| `html/semantics/forms/form-submission-0/newline-normalization.html` | 8 / 12 |  |
| `html/semantics/forms/form-submission-0/reparent-form-during-planned-navigation-task.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/forms/form-submission-0/request-submit-activation.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/forms/form-submission-0/submission-checks.window.js` | 0 / 3 |  |
| `html/semantics/forms/form-submission-0/submit-button-clicked-in-data-document.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/forms/form-submission-0/submit-entity-body.html` | 0 / 13 | harnais : délai du harnais dépassé |
| `html/semantics/forms/form-submission-0/submit-file.sub.html` | 0 / 1 |  |
| `html/semantics/forms/form-submission-0/text-plain.window.js` | 0 / 62 |  |
| `html/semantics/forms/form-submission-0/url-encoded.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/forms/form-submission-0/urlencoded2.window.js` | 0 / 62 |  |
| `html/semantics/forms/form-submission-target/form-target-blank-useractivation-multi-globals.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/forms/form-submission-target/form-target-blank-useractivation.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/forms/form-submission-target/form-target-iframe.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/forms/form-submission-target/form-target-request-header.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/forms/form-submission-target/rel-base-target.html` | 0 / 7 | harnais : délai du harnais dépassé |
| `html/semantics/forms/form-submission-target/rel-button-target.html` | 0 / 7 | harnais : délai du harnais dépassé |
| `html/semantics/forms/form-submission-target/rel-form-target.html` | 0 / 7 | harnais : délai du harnais dépassé |
| `html/semantics/forms/form-submission-target/rel-input-target.html` | 0 / 7 | harnais : délai du harnais dépassé |
| `html/semantics/forms/historical.html` | 13 / 15 | harnais : délai du harnais dépassé |
| `html/semantics/forms/input-change-event-properties.html` | 1 / 11 | harnais : délai du harnais dépassé |
| `html/semantics/forms/resetting-a-form/reset-event.html` | 0 / 1 |  |
| `html/semantics/forms/resetting-a-form/reset-form-2.html` | 0 / 1 |  |
| `html/semantics/forms/resetting-a-form/reset-form-event-realm.html` | 0 / 1 |  |
| `html/semantics/forms/resetting-a-form/reset-form.html` | 0 / 12 |  |
| `html/semantics/forms/setCustomValidity-normalize-newlines.html` | 0 / 1 |  |
| `html/semantics/forms/textfieldselection/defaultSelection.html` | 0 / 6 |  |
| `html/semantics/forms/textfieldselection/select-event.html` | 48 / 270 |  |
| `html/semantics/forms/textfieldselection/selection-after-content-change.html` | 0 / 18 |  |
| `html/semantics/forms/textfieldselection/selection-not-application-textarea.html` | 0 / 1 |  |
| `html/semantics/forms/textfieldselection/selection-not-application.html` | 28 / 71 |  |
| `html/semantics/forms/textfieldselection/selection-preserved-after-text-password-type-change.html` | 0 / 6 |  |
| `html/semantics/forms/textfieldselection/selection-readonly-keyboard.tentative.html` | 0 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/forms/textfieldselection/selection-set-value-after-focus.html` | 0 / 1 |  |
| `html/semantics/forms/textfieldselection/selection-start-end-extra.html` | 1 / 11 |  |
| `html/semantics/forms/textfieldselection/selection-start-end.html` | 3 / 29 | harnais : délai du harnais dépassé |
| `html/semantics/forms/textfieldselection/selection-value-interactions.html` | 2 / 14 |  |
| `html/semantics/forms/textfieldselection/selection.html` | 7 / 18 |  |
| `html/semantics/forms/textfieldselection/setSelectionRange.html` | 0 / 1 |  |
| `html/semantics/forms/textfieldselection/textarea-selection-while-parsing.xhtml` | 0 / 1 |  |
| `html/semantics/forms/textfieldselection/textfieldselection-setRangeText.html` | 9 / 88 | harnais : délai du harnais dépassé |
| `html/semantics/forms/textfieldselection/textfieldselection-setSelectionRange.html` | 0 / 0 | harnais : Error: Unexpected direction |
| `html/semantics/forms/the-button-element/active-onblur.html` | 0 / 1 |  |
| `html/semantics/forms/the-button-element/button-activate.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-button-element/button-click-submits-with-commandfor.html` | 3 / 6 |  |
| `html/semantics/forms/the-button-element/button-click-submits.html` | 9 / 12 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-button-element/button-events.html` | 0 / 9 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-button-element/button-labels.html` | 0 / 1 |  |
| `html/semantics/forms/the-button-element/button-menu-historical.html` | 1 / 2 |  |
| `html/semantics/forms/the-button-element/button-submit-children.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-button-element/button-submit-remove-children-jssubmit.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-button-element/button-submit-remove-children.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-button-element/button-submit-remove-jssubmit.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-button-element/button-type-enumerated-ascii-case-insensitive.html` | 0 / 2 |  |
| `html/semantics/forms/the-button-element/button-type.html` | 0 / 5 |  |
| `html/semantics/forms/the-button-element/button-validation.html` | 3 / 6 |  |
| `html/semantics/forms/the-button-element/controls-inside-link-activation.tentative.html` | 0 / 49 | harnais : Test named '<button>, no form follows the link' specified 1 'cleanup' function, and 1 failed. |
| `html/semantics/forms/the-datalist-element/datalistoptions.html` | 1 / 2 |  |
| `html/semantics/forms/the-datalist-element/input-list-detached.html` | 0 / 1 |  |
| `html/semantics/forms/the-fieldset-element/HTMLFieldSetElement.html` | 1 / 4 |  |
| `html/semantics/forms/the-fieldset-element/disabled-001.html` | 1 / 5 |  |
| `html/semantics/forms/the-fieldset-element/disabled-002.xhtml` | 0 / 1 |  |
| `html/semantics/forms/the-fieldset-element/disabled-003.html` | 0 / 2 |  |
| `html/semantics/forms/the-fieldset-element/disabled-nested-legend.html` | 3 / 4 |  |
| `html/semantics/forms/the-fieldset-element/fieldset-checkvalidity.html` | 0 / 1 |  |
| `html/semantics/forms/the-fieldset-element/fieldset-setcustomvalidity.html` | 0 / 1 |  |
| `html/semantics/forms/the-fieldset-element/fieldset-validationmessage.html` | 0 / 1 |  |
| `html/semantics/forms/the-fieldset-element/fieldset-validity.html` | 0 / 1 |  |
| `html/semantics/forms/the-fieldset-element/fieldset-willvalidate.html` | 0 / 1 |  |
| `html/semantics/forms/the-form-element/form-action-reflection-with-base-url.html` | 1 / 3 |  |
| `html/semantics/forms/the-form-element/form-action-reflection.html` | 2 / 3 |  |
| `html/semantics/forms/the-form-element/form-action-submission-with-base-url.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-form-element/form-action-submission.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-form-element/form-attribute-form-owner.html` | 1 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-form-element/form-autocomplete.html` | 0 / 67 |  |
| `html/semantics/forms/the-form-element/form-elements-filter.html` | 1 / 2 |  |
| `html/semantics/forms/the-form-element/form-elements-form-owner.html` | 0 / 1 |  |
| `html/semantics/forms/the-form-element/form-elements-interfaces-01.html` | 1 / 3 |  |
| `html/semantics/forms/the-form-element/form-elements-matches.html` | 0 / 2 |  |
| `html/semantics/forms/the-form-element/form-elements-nameditem-01.html` | 0 / 3 |  |
| `html/semantics/forms/the-form-element/form-elements-sameobject.html` | 0 / 1 |  |
| `html/semantics/forms/the-form-element/form-indexed-element.html` | 0 / 2 |  |
| `html/semantics/forms/the-form-element/form-indexed-property-names.html` | 0 / 1 |  |
| `html/semantics/forms/the-form-element/form-nameditem.html` | 6 / 17 |  |
| `html/semantics/forms/the-form-element/form-requestsubmit.html` | 6 / 10 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-input-element/button.html` | 4 / 5 |  |
| `html/semantics/forms/the-input-element/change-set-value.html` | 0 / 2 |  |
| `html/semantics/forms/the-input-element/change-to-empty-value.html` | 0 / 1 |  |
| `html/semantics/forms/the-input-element/checkable-active-onblur-with-click.html` | 1 / 7 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-input-element/checkable-active-onblur.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-input-element/checkable-active-space-key-being-disabled.html` | 2 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-input-element/checkable-active-space-key-prevented-default.html` | 1 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-input-element/checkbox-click-events.html` | 3 / 4 |  |
| `html/semantics/forms/the-input-element/checkbox.html` | 1 / 6 |  |
| `html/semantics/forms/the-input-element/click-user-gesture.html` | 2 / 14 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-input-element/clone.html` | 18 / 19 |  |
| `html/semantics/forms/the-input-element/cloning-steps.html` | 23 / 68 |  |
| `html/semantics/forms/the-input-element/color-attributes.window.js` | 0 / 2 |  |
| `html/semantics/forms/the-input-element/color.html` | 1 / 27 |  |
| `html/semantics/forms/the-input-element/color.window.js` | 1 / 196 |  |
| `html/semantics/forms/the-input-element/date-readonly-keyboard.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-input-element/date.html` | 4 / 6 |  |
| `html/semantics/forms/the-input-element/datetime-local-trailing-zeros.html` | 0 / 1 |  |
| `html/semantics/forms/the-input-element/datetime-local-valueasdate.html` | 0 / 1 |  |
| `html/semantics/forms/the-input-element/datetime-local.html` | 2 / 17 |  |
| `html/semantics/forms/the-input-element/datetime-weekmonth.html` | 10 / 14 |  |
| `html/semantics/forms/the-input-element/datetime.html` | 15 / 21 |  |
| `html/semantics/forms/the-input-element/default-type.html` | 44 / 72 |  |
| `html/semantics/forms/the-input-element/defaultValue-clobbering.html` | 1 / 2 |  |
| `html/semantics/forms/the-input-element/email-set-value.html` | 0 / 1 |  |
| `html/semantics/forms/the-input-element/email-value-idn.html` | 0 / 4 |  |
| `html/semantics/forms/the-input-element/email.html` | 4 / 8 |  |
| `html/semantics/forms/the-input-element/files.html` | 21 / 24 |  |
| `html/semantics/forms/the-input-element/hidden-charset-case-sensitive.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-input-element/hidden.html` | 5 / 9 |  |
| `html/semantics/forms/the-input-element/image-click-form-data.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-input-element/image-width-height.html` | 0 / 32 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-input-element/input-disabled-fieldset-dynamic.html` | 0 / 1 |  |
| `html/semantics/forms/the-input-element/input-list.html` | 0 / 6 |  |
| `html/semantics/forms/the-input-element/input-seconds-leading-zeroes.html` | 4 / 12 |  |
| `html/semantics/forms/the-input-element/input-step-float.html` | 0 / 1 |  |
| `html/semantics/forms/the-input-element/input-step-realignment.html` | 0 / 1 |  |
| `html/semantics/forms/the-input-element/input-stepdown-02.html` | 0 / 6 |  |
| `html/semantics/forms/the-input-element/input-stepdown-weekmonth.html` | 0 / 2 |  |
| `html/semantics/forms/the-input-element/input-stepdown.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-input-element/input-stepup.html` | 0 / 53 |  |
| `html/semantics/forms/the-input-element/input-submit-remove-jssubmit.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-input-element/input-type-change-submit.html` | 0 / 20 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-input-element/input-type-change-value.html` | 0 / 2 |  |
| `html/semantics/forms/the-input-element/input-type-checkbox-switch.tentative.window.js` | 0 / 3 |  |
| `html/semantics/forms/the-input-element/input-type-checkbox.html` | 5 / 7 |  |
| `html/semantics/forms/the-input-element/input-value-dirty-flag.html` | 2 / 5 |  |
| `html/semantics/forms/the-input-element/input-value-invalidstateerr.html` | 0 / 1 |  |
| `html/semantics/forms/the-input-element/input-valueasdate-invalidstateerr.html` | 0 / 1 |  |
| `html/semantics/forms/the-input-element/input-valueasdate-null.html` | 0 / 4 |  |
| `html/semantics/forms/the-input-element/input-valueasdate-stepping.html` | 0 / 4 |  |
| `html/semantics/forms/the-input-element/input-valueasdate-typeerror.html` | 0 / 4 |  |
| `html/semantics/forms/the-input-element/input-valueasdate.html` | 4 / 30 |  |
| `html/semantics/forms/the-input-element/input-valueasnumber-invalidstateerr.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-input-element/input-valueasnumber-nan.html` | 0 / 1 |  |
| `html/semantics/forms/the-input-element/input-valueasnumber-stepping.html` | 0 / 7 |  |
| `html/semantics/forms/the-input-element/input-valueasnumber-typeerror.html` | 0 / 4 |  |
| `html/semantics/forms/the-input-element/input-valueasnumber.html` | 33 / 64 |  |
| `html/semantics/forms/the-input-element/maxlength-number.html` | 0 / 1 |  |
| `html/semantics/forms/the-input-element/maxlength.html` | 4 / 5 |  |
| `html/semantics/forms/the-input-element/minlength.html` | 3 / 5 |  |
| `html/semantics/forms/the-input-element/month.html` | 5 / 15 |  |
| `html/semantics/forms/the-input-element/number-constraint-validation.html` | 0 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-input-element/number-invalid-scientific-key-events.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-input-element/number.html` | 16 / 34 |  |
| `html/semantics/forms/the-input-element/password.html` | 2 / 5 |  |
| `html/semantics/forms/the-input-element/pattern_attribute.html` | 3 / 4 |  |
| `html/semantics/forms/the-input-element/pattern_attribute_v_flag.html` | 1 / 3 |  |
| `html/semantics/forms/the-input-element/radio-click-events.html` | 3 / 4 |  |
| `html/semantics/forms/the-input-element/radio-disconnected-group-owner.html` | 3 / 8 |  |
| `html/semantics/forms/the-input-element/radio-focus-navigation-checked.html` | 1 / 5 |  |
| `html/semantics/forms/the-input-element/radio-focus-navigation-disabled.html` | 0 / 1 |  |
| `html/semantics/forms/the-input-element/radio-focus-navigation-group-first-focus.html` | 0 / 1 |  |
| `html/semantics/forms/the-input-element/radio-focus-navigation-group-focusable-focus.html` | 0 / 2 |  |
| `html/semantics/forms/the-input-element/radio-focused-and-scrolled-into-view.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-input-element/radio-form-owner.html` | 0 / 1 |  |
| `html/semantics/forms/the-input-element/radio-keyboard-navigation-order.html` | 0 / 1 |  |
| `html/semantics/forms/the-input-element/radio-morphed.html` | 0 / 1 |  |
| `html/semantics/forms/the-input-element/radio.html` | 4 / 12 |  |
| `html/semantics/forms/the-input-element/range-2.html` | 1 / 5 |  |
| `html/semantics/forms/the-input-element/range-drag-after-selection.html` | 0 / 2 |  |
| `html/semantics/forms/the-input-element/range-restore-oninput-onchange-event.https.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-input-element/range.html` | 10 / 25 |  |
| `html/semantics/forms/the-input-element/reset.html` | 1 / 6 |  |
| `html/semantics/forms/the-input-element/selection-pointer.html` | 0 / 21 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-input-element/selection-weekmonth.html` | 4 / 6 |  |
| `html/semantics/forms/the-input-element/selection.html` | 22 / 42 |  |
| `html/semantics/forms/the-input-element/show-picker-cross-origin-iframe.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-input-element/show-picker-disabled-readonly.html` | 0 / 44 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-input-element/show-picker-does-not-focus.html` | 0 / 22 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-input-element/show-picker-user-gesture.html` | 0 / 63 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-input-element/telephone.html` | 8 / 13 |  |
| `html/semantics/forms/the-input-element/text.html` | 8 / 18 |  |
| `html/semantics/forms/the-input-element/time-2.html` | 10 / 17 |  |
| `html/semantics/forms/the-input-element/time.html` | 8 / 32 |  |
| `html/semantics/forms/the-input-element/type-change-state-weekmonth.html` | 0 / 82 |  |
| `html/semantics/forms/the-input-element/type-change-state.html` | 0 / 380 |  |
| `html/semantics/forms/the-input-element/type-change-text-to-file-to-text.html` | 1 / 3 |  |
| `html/semantics/forms/the-input-element/url.html` | 1 / 4 |  |
| `html/semantics/forms/the-input-element/valueMode-weekmonth.html` | 0 / 4 |  |
| `html/semantics/forms/the-input-element/valueMode.html` | 14 / 40 |  |
| `html/semantics/forms/the-input-element/week.html` | 7 / 16 |  |
| `html/semantics/forms/the-label-element/clicking-noninteractive-labelable-content.html` | 6 / 9 |  |
| `html/semantics/forms/the-label-element/clicking-noninteractive-unlabelable-content.html` | 34 / 51 |  |
| `html/semantics/forms/the-label-element/control-active-with-multiple-clicks.html` | 0 / 1 |  |
| `html/semantics/forms/the-label-element/disabled-labeled-control.html` | 6 / 8 |  |
| `html/semantics/forms/the-label-element/forward-focus-to-associated-element.html` | 2 / 6 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-label-element/label-attributes.sub.html` | 4 / 20 |  |
| `html/semantics/forms/the-label-element/label-forwarded-click-pointer-properties.html` | 0 / 2 |  |
| `html/semantics/forms/the-label-element/labelable-elements.html` | 16 / 26 |  |
| `html/semantics/forms/the-label-element/labels-live-attribute-changes.html` | 0 / 1 |  |
| `html/semantics/forms/the-label-element/labels-live-label-removal.html` | 0 / 1 |  |
| `html/semantics/forms/the-label-element/proxy-click-to-associated-element.html` | 1 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-label-element/proxy-modifier-click-to-associated-element.tentative.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-legend-element/HTMLLegendElement.html` | 0 / 3 |  |
| `html/semantics/forms/the-legend-element/legend-form.html` | 0 / 2 |  |
| `html/semantics/forms/the-meter-element/meter-leading-plus-sign.html` | 0 / 12 |  |
| `html/semantics/forms/the-meter-element/meter.html` | 0 / 50 |  |
| `html/semantics/forms/the-option-element/option-element-constructor.html` | 7 / 11 |  |
| `html/semantics/forms/the-option-element/option-label.html` | 7 / 12 |  |
| `html/semantics/forms/the-option-element/option-selected.html` | 1 / 3 |  |
| `html/semantics/forms/the-option-element/option-text-recurse.html` | 8 / 11 |  |
| `html/semantics/forms/the-option-element/option-text-spaces.html` | 46 / 50 |  |
| `html/semantics/forms/the-option-element/option-value.html` | 7 / 12 |  |
| `html/semantics/forms/the-output-element/mutations.window.js` | 0 / 2 |  |
| `html/semantics/forms/the-output-element/output-setcustomvalidity.html` | 0 / 1 |  |
| `html/semantics/forms/the-output-element/output-validity.html` | 0 / 1 |  |
| `html/semantics/forms/the-output-element/output.html` | 0 / 1 |  |
| `html/semantics/forms/the-progress-element/progress-2.html` | 0 / 4 |  |
| `html/semantics/forms/the-progress-element/progress-max-setting.html` | 0 / 1 |  |
| `html/semantics/forms/the-progress-element/progress.html` | 0 / 11 |  |
| `html/semantics/forms/the-progress-element/progress.window.js` | 0 / 2 |  |
| `html/semantics/forms/the-select-element/common-HTMLOptionsCollection-add.html` | 0 / 3 |  |
| `html/semantics/forms/the-select-element/common-HTMLOptionsCollection-namedItem.html` | 0 / 6 |  |
| `html/semantics/forms/the-select-element/common-HTMLOptionsCollection.html` | 2 / 8 |  |
| `html/semantics/forms/the-select-element/customizable-select-listbox/customizable-select-listbox-keyboard-behavior.optional.html` | 0 / 4 |  |
| `html/semantics/forms/the-select-element/customizable-select-listbox/customizable-select-listbox-mouse-behavior.html` | 0 / 4 |  |
| `html/semantics/forms/the-select-element/customizable-select-listbox/select-listbox-home-end-pagedown-pageup.optional.html` | 0 / 2 |  |
| `html/semantics/forms/the-select-element/customizable-select-listbox/select-listbox-option-optgroup-client-rects.html` | 0 / 12 |  |
| `html/semantics/forms/the-select-element/customizable-select-listbox/select-listbox-typeahead.optional.html` | 0 / 2 |  |
| `html/semantics/forms/the-select-element/customizable-select-listbox/selectedcontent-listbox.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/customizable-select-multiple-popup/select-multiple-popup-keyboard-behavior.tentative.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/customizable-select-multiple-popup/select-multiple-popup-mouse-behavior.tentative.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/customizable-select-multiple-popup/select-multiple-validity-invalidation.tentative.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/customizable-select-multiple-popup/selectedcontent-multiple-restore.optional.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/customizable-select-multiple-popup/selectedcontent-multiple.tentative.html` | 0 / 2 |  |
| `html/semantics/forms/the-select-element/customizable-select/base-appearance-inheritance.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/customizable-select/button-in-popover.html` | 0 / 2 |  |
| `html/semantics/forms/the-select-element/customizable-select/nested-options.html` | 0 / 3 |  |
| `html/semantics/forms/the-select-element/customizable-select/option-disabled-invalid-nesting.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/customizable-select/option-disabled-optgroup-wrapper.html` | 0 / 3 |  |
| `html/semantics/forms/the-select-element/customizable-select/option-disabled-optgroup.html` | 2 / 6 |  |
| `html/semantics/forms/the-select-element/customizable-select/option-form-ancestor-select.html` | 0 / 2 |  |
| `html/semantics/forms/the-select-element/customizable-select/option-list-wrapped-optgroup.html` | 1 / 2 |  |
| `html/semantics/forms/the-select-element/customizable-select/option-list.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/customizable-select/option-selectedness-wrapper.html` | 6 / 32 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-accessibility-minimum-target-size.html` | 0 / 6 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-ask-for-reset-base-appearance.html` | 0 / 2 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-base-appearance-computed-style.html` | 0 / 11 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-button-behaviors.html` | 0 / 2 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-click-drag-option.optional.html` | 0 / 3 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-click-picker-light-dismiss.tentative.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-dialog-mode-focus.optional.html` | 0 / 2 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-disabled.html` | 0 / 2 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-events-2.optional.html` | 1 / 7 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-select-element/customizable-select/select-events.optional.html` | 0 / 2 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-focus-visible-with-mouse.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-home-end-pagedown-pageup-detailed.optional.html` | 0 / 4 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-home-end-pagedown-pageup.optional.html` | 0 / 4 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-initial-focus-display-animation.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-input-parsing.tentative.html` | 2 / 4 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-inside-top-layer.html` | 0 / 4 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-iterate-before-beginning.optional.html` | 0 / 2 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-keyboard-behavior.optional.html` | 0 / 10 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-select-element/customizable-select/select-keyboard-focus-change-for-hidden-options.optional.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-mouse-behavior.html` | 1 / 8 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-select-element/customizable-select/select-mousedown-slot-mutation.optional.html` | 0 / 2 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-optgroup-arrow-keys.optional.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-optgroup-tabindex-focus.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-option-arrow-scroll.optional.html` | 0 / 2 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-option-hover-styles.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-options-id.html` | 0 / 3 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-parsing.html` | 8 / 17 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-pick-option-dirtiness.html` | 0 / 2 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-pick-option-keydown.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-picker-animations.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-picker-exit-animation.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-picker-hover-active-pseudo.html` | 0 / 4 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-picker-hover-after-picking.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-picker-interactive-element-focus.optional.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-picker-option-animation-reopens.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-picker-option-hover-while-dragging.optional.html` | 0 / 2 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-picker-option-optgroup-client-rects.html` | 0 / 8 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-picker-popover-open-invalid.tentative.html` | 0 / 2 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-picker-pseudo-box-type.html` | 2 / 3 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-picker-starting-style.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-pseudo-light-dismiss-invalidation.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-pseudo-open-display.html` | 0 / 2 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-pseudo-open-move-remove.html` | 0 / 2 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-pseudo-open-size.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-select-element/customizable-select/select-pseudo-open-top-layer.html` | 0 / 4 | harnais : Test named 'Showing an unrelated auto popover closes the picker.' specified 1 'cleanup' function, and 1 failed. |
| `html/semantics/forms/the-select-element/customizable-select/select-pseudo-open.html` | 0 / 2 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-synthetic-events.html` | 0 / 2 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-type-to-search.tentative.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-value-selectedOption.html` | 5 / 11 |  |
| `html/semantics/forms/the-select-element/customizable-select/select-vertical-writing-mode-navigation.optional.html` | 0 / 8 |  |
| `html/semantics/forms/the-select-element/customizable-select/selectedcontent-attribute-change.html` | 0 / 7 |  |
| `html/semantics/forms/the-select-element/customizable-select/selectedcontent-cloned-option-is-not-an-option.tentative.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/customizable-select/selectedcontent-custom-element-reactions.html` | 0 / 6 |  |
| `html/semantics/forms/the-select-element/customizable-select/selectedcontent-insertion-removal.html` | 0 / 4 |  |
| `html/semantics/forms/the-select-element/customizable-select/selectedcontent-list-box-pick.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/customizable-select/selectedcontent-movebefore.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/customizable-select/selectedcontent-mutations.html` | 0 / 8 |  |
| `html/semantics/forms/the-select-element/customizable-select/selectedcontent-nested-select.html` | 0 / 2 |  |
| `html/semantics/forms/the-select-element/customizable-select/selectedcontent-nested.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/customizable-select/selectedcontent-omits-scripts-frames-and-plugins.html` | 0 / 2 |  |
| `html/semantics/forms/the-select-element/customizable-select/selectedcontent-option-insertion.html` | 0 / 10 |  |
| `html/semantics/forms/the-select-element/customizable-select/selectedcontent-parser-removal-csp-strict-dynamic.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/customizable-select/selectedcontent-parser-removal-script-runs-once.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/customizable-select/selectedcontent-parser-script-runs-once.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/customizable-select/selectedcontent-queued-update.html` | 0 / 10 |  |
| `html/semantics/forms/the-select-element/customizable-select/selectedcontent-restore.optional.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-select-element/customizable-select/selectedcontent-subtree-mutations.html` | 0 / 3 |  |
| `html/semantics/forms/the-select-element/customizable-select/selectedcontent.html` | 0 / 11 |  |
| `html/semantics/forms/the-select-element/customizable-select/selectedcontentelement-attr.tentative.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/customizable-select/switch-picker-appearance-timing.tentative.html` | 0 / 2 |  |
| `html/semantics/forms/the-select-element/customizable-select/switch-picker-appearance.html` | 0 / 9 |  |
| `html/semantics/forms/the-select-element/filterable-select/filterable-select-filtered-pseudo.tentative.html` | 0 / 7 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-select-element/filterable-select/filterable-select-keyboard-behavior.tentative.html` | 0 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-select-element/filterable-select/select-beforefilter.tentative.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/inserted-or-removed.html` | 0 / 5 |  |
| `html/semantics/forms/the-select-element/option-selectedness-script-mutation.html` | 0 / 5 |  |
| `html/semantics/forms/the-select-element/select-add-optgroup.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/select-ask-for-reset.html` | 1 / 4 |  |
| `html/semantics/forms/the-select-element/select-attribute-change-reset.html` | 5 / 8 |  |
| `html/semantics/forms/the-select-element/select-insertion-inherited-style.html` | 1 / 3 |  |
| `html/semantics/forms/the-select-element/select-label-click.tentative.html` | 0 / 9 | harnais : Test named 'Clicking label opens select picker: appearance:auto with label[for]' specified 1 'cleanup' function, and 1 f |
| `html/semantics/forms/the-select-element/select-listbox-touch.optional.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/select-many-options.tentative.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/select-multiple-resize-does-not-autoscroll.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/select-multiple.html` | 2 / 3 |  |
| `html/semantics/forms/the-select-element/select-named-getter.html` | 0 / 4 |  |
| `html/semantics/forms/the-select-element/select-option-display-inherit.tentative.html` | 1 / 3 |  |
| `html/semantics/forms/the-select-element/select-option-focusable.tentative.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/select-popover-position-with-zoom.tentative.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-select-element/select-remove.html` | 2 / 4 |  |
| `html/semantics/forms/the-select-element/select-required-attribute.tentative.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/select-restore-invalid-option.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-select-element/select-selectedOptions-nesting.window.js` | 0 / 11 |  |
| `html/semantics/forms/the-select-element/select-selectedOptions.html` | 2 / 10 |  |
| `html/semantics/forms/the-select-element/select-setter-dirtiness.html` | 0 / 7 |  |
| `html/semantics/forms/the-select-element/select-user-selection-dirtiness.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-select-element/select-validity.html` | 5 / 6 |  |
| `html/semantics/forms/the-select-element/selected-index.html` | 0 / 14 |  |
| `html/semantics/forms/the-select-element/show-picker-being-cv-hidden.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/show-picker-being-rendered.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/show-picker-cross-origin-iframe.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-select-element/show-picker-disabled.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/show-picker-multiple.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/show-picker-size.html` | 0 / 1 |  |
| `html/semantics/forms/the-select-element/show-picker-user-gesture.html` | 0 / 2 |  |
| `html/semantics/forms/the-textarea-element/change-to-empty-value.html` | 0 / 1 |  |
| `html/semantics/forms/the-textarea-element/cloning-steps.html` | 0 / 2 |  |
| `html/semantics/forms/the-textarea-element/selection-after-whitespace-change.html` | 0 / 1 |  |
| `html/semantics/forms/the-textarea-element/textarea-maxlength.html` | 3 / 5 |  |
| `html/semantics/forms/the-textarea-element/textarea-minlength.html` | 0 / 5 |  |
| `html/semantics/forms/the-textarea-element/textarea-placeholder-lineheight.html` | 1 / 2 |  |
| `html/semantics/forms/the-textarea-element/textarea-type.html` | 0 / 1 |  |
| `html/semantics/forms/the-textarea-element/value-defaultValue-textContent.html` | 6 / 12 |  |
| `html/semantics/forms/the-textarea-element/wrap-enumerated-ascii-case-insensitive.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/forms/the-textarea-element/wrapping-transformation.window.js` | 5 / 8 |  |
| `html/semantics/grouping-content/the-li-element/grouping-li.html` | 0 / 10 |  |
| `html/semantics/grouping-content/the-ol-element/grouping-ol.html` | 4 / 25 |  |
| `html/semantics/grouping-content/the-ol-element/ol.start-reflection-1.html` | 0 / 1 |  |
| `html/semantics/grouping-content/the-ol-element/ol.start-reflection-2.html` | 0 / 1 |  |
| `html/semantics/interactive-elements/commands/option/option-accesskey-dirtiness.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/interactive-elements/commands/option/option-accesskey.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/interactive-elements/contextmenu-historical.html` | 9 / 11 |  |
| `html/semantics/interactive-elements/the-details-element/auto-expand-details-element-fragment.html` | 0 / 1 |  |
| `html/semantics/interactive-elements/the-details-element/details-keyboard-activation.html` | 0 / 1 |  |
| `html/semantics/interactive-elements/the-details-element/details-name-exclusivity-fragment-insertion.html` | 0 / 1 |  |
| `html/semantics/interactive-elements/the-details-element/details-toggle-source-commandfor.tentative.html` | 0 / 1 |  |
| `html/semantics/interactive-elements/the-details-element/details-toggle-source.html` | 0 / 3 |  |
| `html/semantics/interactive-elements/the-details-element/name-attribute.html` | 1 / 18 |  |
| `html/semantics/interactive-elements/the-details-element/toggleEvent.html` | 1 / 11 | harnais : délai du harnais dépassé |
| `html/semantics/interactive-elements/the-dialog-element/abspos-dialog-layout.html` | 8 / 10 |  |
| `html/semantics/interactive-elements/the-dialog-element/centering.html` | 0 / 23 |  |
| `html/semantics/interactive-elements/the-dialog-element/child-sequential-focus.html` | 3 / 5 |  |
| `html/semantics/interactive-elements/the-dialog-element/default-color.html` | 1 / 2 |  |
| `html/semantics/interactive-elements/the-dialog-element/dialog-active-document.html` | 0 / 1 |  |
| `html/semantics/interactive-elements/the-dialog-element/dialog-autofocus-multiple-times.html` | 0 / 1 |  |
| `html/semantics/interactive-elements/the-dialog-element/dialog-autofocus.html` | 0 / 1 |  |
| `html/semantics/interactive-elements/the-dialog-element/dialog-cancel-events.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/interactive-elements/the-dialog-element/dialog-cancel-preventDefault.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/interactive-elements/the-dialog-element/dialog-canceling.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/interactive-elements/the-dialog-element/dialog-close-event-async.html` | 0 / 1 |  |
| `html/semantics/interactive-elements/the-dialog-element/dialog-close-focus-no-previous.html` | 0 / 1 |  |
| `html/semantics/interactive-elements/the-dialog-element/dialog-close-via-attribute.tentative.html` | 0 / 2 |  |
| `html/semantics/interactive-elements/the-dialog-element/dialog-close.html` | 4 / 5 |  |
| `html/semantics/interactive-elements/the-dialog-element/dialog-closedby-bounds-clicking.html` | 2 / 14 | harnais : délai du harnais dépassé |
| `html/semantics/interactive-elements/the-dialog-element/dialog-closedby-corner-cases.html` | 0 / 30 |  |
| `html/semantics/interactive-elements/the-dialog-element/dialog-closedby-show-stacked.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/interactive-elements/the-dialog-element/dialog-closedby-start-open.html` | 0 / 2 |  |
| `html/semantics/interactive-elements/the-dialog-element/dialog-closedby.html` | 0 / 27 | harnais : délai du harnais dépassé |
| `html/semantics/interactive-elements/the-dialog-element/dialog-focus-previous-outside.html` | 2 / 6 |  |
| `html/semantics/interactive-elements/the-dialog-element/dialog-focus-shadow-double-nested.html` | 0 / 2 |  |
| `html/semantics/interactive-elements/the-dialog-element/dialog-focus-shadow.html` | 24 / 50 |  |
| `html/semantics/interactive-elements/the-dialog-element/dialog-focusability.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/interactive-elements/the-dialog-element/dialog-focusing-steps-disconnected.html` | 1 / 2 |  |
| `html/semantics/interactive-elements/the-dialog-element/dialog-focusing-steps-inert.html` | 0 / 2 |  |
| `html/semantics/interactive-elements/the-dialog-element/dialog-form-submission.html` | 5 / 8 | harnais : Uncaught { message: 'assert_equals: dialog's returnValue remains the same expected "" but got "undefined"', stack: 'Erro |
| `html/semantics/interactive-elements/the-dialog-element/dialog-inert.html` | 0 / 1 |  |
| `html/semantics/interactive-elements/the-dialog-element/dialog-light-dismiss-pointer-capture.html` | 0 / 4 | harnais : Uncaught TypeError: .setPointerCapture n'est pas une fonction (c'est undefined) |
| `html/semantics/interactive-elements/the-dialog-element/dialog-light-dismiss-touch.html` | 0 / 1 |  |
| `html/semantics/interactive-elements/the-dialog-element/dialog-open-movebefore-setup.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/interactive-elements/the-dialog-element/dialog-overlay.html` | 0 / 2 |  |
| `html/semantics/interactive-elements/the-dialog-element/dialog-popover-closedby-complex.html` | 0 / 40 | harnais : Test named 'clicking outside all with modeless dialogA and modeless dialogB, normal DOM nesting' specified 1 'cleanup' f |
| `html/semantics/interactive-elements/the-dialog-element/dialog-popover-closedby-simple.html` | 0 / 6 | harnais : Test named 'Dialog closedby=any parent, popover child' specified 1 'cleanup' function, and 1 failed. |
| `html/semantics/interactive-elements/the-dialog-element/dialog-requestclose-2.html` | 0 / 1 |  |
| `html/semantics/interactive-elements/the-dialog-element/dialog-requestclose-3.html` | 0 / 1 |  |
| `html/semantics/interactive-elements/the-dialog-element/dialog-requestclose-recurse.html` | 0 / 1 |  |
| `html/semantics/interactive-elements/the-dialog-element/dialog-showModal.html` | 7 / 10 |  |
| `html/semantics/interactive-elements/the-dialog-element/dialog-toggle-source.html` | 0 / 6 |  |
| `html/semantics/interactive-elements/the-dialog-element/focus-after-close.html` | 3 / 7 | harnais : délai du harnais dépassé |
| `html/semantics/interactive-elements/the-dialog-element/inert-focus-in-frames.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/interactive-elements/the-dialog-element/inert-inlines.html` | 0 / 1 |  |
| `html/semantics/interactive-elements/the-dialog-element/inert-label-focus.html` | 0 / 1 |  |
| `html/semantics/interactive-elements/the-dialog-element/inert-node-is-uneditable.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/interactive-elements/the-dialog-element/inert-node-is-unfocusable.html` | 5 / 16 |  |
| `html/semantics/interactive-elements/the-dialog-element/inert-node-is-unselectable.html` | 0 / 1 |  |
| `html/semantics/interactive-elements/the-dialog-element/inert-svg-hittest.html` | 1 / 2 | harnais : Timeout while running cleanup for test named "Hit-testing can reach contents of a no longer inert SVG". |
| `html/semantics/interactive-elements/the-dialog-element/inertness-with-modal-dialogs-and-iframes.html` | 0 / 8 |  |
| `html/semantics/interactive-elements/the-dialog-element/modal-dialog-ancestor-is-inert.html` | 0 / 1 | harnais : Uncaught { message: 'assert_unreached: body was targeted for an click event Reached unreachable code', stack: 'Error' } |
| `html/semantics/interactive-elements/the-dialog-element/modal-dialog-blocks-mouse-events.html` | 0 / 1 |  |
| `html/semantics/interactive-elements/the-dialog-element/modal-dialog-in-visibility-hidden.html` | 2 / 3 |  |
| `html/semantics/interactive-elements/the-dialog-element/modal-dialog-selection.html` | 1 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/interactive-elements/the-dialog-element/non-modal-canceling.html` | 0 / 1 |  |
| `html/semantics/interactive-elements/the-dialog-element/non-modal-dialog-layout.html` | 0 / 1 |  |
| `html/semantics/interactive-elements/the-dialog-element/remove-dialog-should-unblock-document.html` | 0 / 1 |  |
| `html/semantics/interactive-elements/the-dialog-element/show-modal-focusing-steps.html` | 0 / 1 |  |
| `html/semantics/interactive-elements/the-dialog-element/toggle-events.html` | 2 / 12 |  |
| `html/semantics/interactive-elements/the-dialog-element/top-layer-position.html` | 0 / 1 |  |
| `html/semantics/interactive-elements/the-dialog-element/top-layer-removal-invalidates-ua-styles.html` | 0 / 1 |  |
| `html/semantics/interactive-elements/the-summary-element/activation-behavior.html` | 0 / 9 |  |
| `html/semantics/interactive-elements/the-summary-element/anchor-with-inline-element.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/interactive-elements/the-summary-element/anchor-without-link.html` | 0 / 2 |  |
| `html/semantics/interactive-elements/the-summary-element/click-behavior-optional.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/interactive-elements/the-summary-element/interactive-content.html` | 1 / 34 | harnais : délai du harnais dépassé |
| `html/semantics/interestfor/idlharness.tentative.html` | 0 / 1 |  |
| `html/semantics/interestfor/interestelement-interface.tentative.html` | 0 / 6 |  |
| `html/semantics/interestfor/interestevent-dispatch-shadow.tentative.html` | 0 / 3 |  |
| `html/semantics/interestfor/interestevent-interface.tentative.html` | 0 / 11 |  |
| `html/semantics/interestfor/interestfor-anchor-event-dispatch.tentative.html` | 0 / 3 |  |
| `html/semantics/interestfor/interestfor-area-event-dispatch.tentative.html` | 0 / 2 |  |
| `html/semantics/interestfor/interestfor-basic-behavior.tentative.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/interestfor/interestfor-basic-delays.tentative.html` | 0 / 1 |  |
| `html/semantics/interestfor/interestfor-button-event-dispatch.tentative.html` | 1 / 3 |  |
| `html/semantics/interestfor/interestfor-command-invoker.tentative.html` | 0 / 2 |  |
| `html/semantics/interestfor/interestfor-css-properties.tentative.html` | 7 / 17 |  |
| `html/semantics/interestfor/interestfor-css-shorthands.tentative.html` | 1 / 26 |  |
| `html/semantics/interestfor/interestfor-delay-end.tentative.html` | 0 / 5 |  |
| `html/semantics/interestfor/interestfor-delay-start.tentative.html` | 0 / 8 |  |
| `html/semantics/interestfor/interestfor-event-timing.tentative.html` | 0 / 1 |  |
| `html/semantics/interestfor/interestfor-hide-popover-interest-state.tentative.html` | 0 / 3 |  |
| `html/semantics/interestfor/interestfor-iframe-popover.tentative.html` | 0 / 6 |  |
| `html/semantics/interestfor/interestfor-implicit-anchor.tentative.html` | 0 / 1 |  |
| `html/semantics/interestfor/interestfor-invoker-descendants.tentative.html` | 0 / 8 |  |
| `html/semantics/interestfor/interestfor-keyboard-behavior.tentative.html` | 0 / 1 |  |
| `html/semantics/interestfor/interestfor-on-popover-behavior.tentative.html` | 0 / 2 | harnais : Test named 'hover interest invoking closed popover opens' specified 1 'cleanup' function, and 1 failed. |
| `html/semantics/interestfor/interestfor-pseudo-classes.tentative.html` | 0 / 7 |  |
| `html/semantics/interestfor/interestfor-pseudo-element-block-button.tentative.html` | 0 / 1 |  |
| `html/semantics/interestfor/interestfor-pseudo-element-both-table-display.tentative.html` | 0 / 2 |  |
| `html/semantics/interestfor/interestfor-pseudo-element-pseudo-table-display.tentative.html` | 0 / 2 |  |
| `html/semantics/interestfor/interestfor-pseudo-element-table-cell.tentative.html` | 0 / 2 |  |
| `html/semantics/interestfor/interestfor-pseudo-element.tentative.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/interestfor/interestfor-retrigger-on-esc.tentative.html` | 0 / 1 |  |
| `html/semantics/interestfor/interestfor-svg-a-event-dispatch.tentative.html` | 0 / 2 |  |
| `html/semantics/interestfor/interestfor-user-select.tentative.html` | 0 / 1 |  |
| `html/semantics/interfaces.html` | 317 / 438 |  |
| `html/semantics/links/downloading-resources/header-origin-no-referrer-when-downgrade.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/links/downloading-resources/header-origin-no-referrer.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/links/downloading-resources/header-origin-origin-when-cross-origin.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/links/downloading-resources/header-origin-origin.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/links/downloading-resources/header-origin-same-origin.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/links/downloading-resources/header-origin-strict-origin-when-cross-origin.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/links/downloading-resources/header-origin-strict-origin.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/links/downloading-resources/header-origin-unsafe-url.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/links/downloading-resources/header-origin.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/links/downloading-resources/header-referrer-no-referrer-when-downgrade.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/links/downloading-resources/header-referrer-no-referrer.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/links/downloading-resources/header-referrer-origin-when-cross-origin.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/links/downloading-resources/header-referrer-origin.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/links/downloading-resources/header-referrer-same-origin.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/links/downloading-resources/header-referrer-strict-origin-when-cross-origin.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/links/downloading-resources/header-referrer-strict-origin.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/links/downloading-resources/header-referrer-unsafe-url.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/links/downloading-resources/header-referrer.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/links/following-hyperlinks/activation-behavior.window.js` | 3 / 4 |  |
| `html/semantics/links/hyperlink-auditing/headers.optional.html` | 0 / 1 |  |
| `html/semantics/links/links-created-by-a-and-area-elements/anchor-src-encoding.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/links/links-created-by-a-and-area-elements/htmlanchorelement_noopener.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/links/links-created-by-a-and-area-elements/non-parsable-url-getter-setter.window.js` | 34 / 38 |  |
| `html/semantics/links/links-created-by-a-and-area-elements/rel-attribute-clearing.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/links/links-created-by-a-and-area-elements/rel-opener-prevents-browsing-context-group-change.tentative.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/links/links-created-by-a-and-area-elements/target_blank_implicit_noopener.html` | 0 / 13 | harnais : délai du harnais dépassé |
| `html/semantics/links/links-created-by-a-and-area-elements/target_blank_implicit_noopener_base.html` | 0 / 13 | harnais : délai du harnais dépassé |
| `html/semantics/links/links-created-by-a-and-area-elements/target_blank_useractivation.html` | 0 / 8 | harnais : Uncaught TypeError: impossible de lire la propriété « isActive » de undefined (valeur de .userActivation) |
| `html/semantics/links/links-created-by-a-and-area-elements/target_blank_useractivation_multi_globals.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/menu/tentative/button-invoke-menulist.html` | 0 / 1 |  |
| `html/semantics/menu/tentative/checkable.html` | 2 / 15 |  |
| `html/semantics/menu/tentative/focus-enters-menulist.html` | 0 / 6 |  |
| `html/semantics/menu/tentative/focus-menu-elements-arrowoperations.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/menu/tentative/focus-menu-elements-nested-arrowoperations.html` | 0 / 5 |  |
| `html/semantics/menu/tentative/focus-menu-elements-side-arrowoperations.html` | 0 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/menu/tentative/focus-without-user-activation.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/menu/tentative/idlharness.https.html` | 0 / 1 |  |
| `html/semantics/menu/tentative/menu-content-model-violation.html` | 0 / 36 |  |
| `html/semantics/menu/tentative/menu-elements-pageup-pagedown.optional.html` | 0 / 1 |  |
| `html/semantics/menu/tentative/menu-elements-tab-focus.optional.html` | 0 / 1 |  |
| `html/semantics/menu/tentative/menu-elements-typeahead.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/menu/tentative/menu-safe-triangle.optional.html` | 0 / 2 |  |
| `html/semantics/menu/tentative/menu-violation-mode-focus.html` | 0 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/menu/tentative/menubar-interest-command-invoker.tentative.html` | 0 / 2 |  |
| `html/semantics/menu/tentative/menubar-invoke-menulist.html` | 1 / 6 |  |
| `html/semantics/menu/tentative/menuitem-activate.html` | 0 / 6 |  |
| `html/semantics/menu/tentative/menuitem-drag-click-event.html` | 2 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/menu/tentative/menuitem-implicit-interest-invoker.tentative.html` | 0 / 3 |  |
| `html/semantics/menu/tentative/menuitem-interest-invoker.tentative.html` | 0 / 2 |  |
| `html/semantics/menu/tentative/menuitem-no-interest-button-pseudo.html` | 1 / 2 |  |
| `html/semantics/menu/tentative/menulist-popover-attribute.html` | 0 / 2 |  |
| `html/semantics/menu/tentative/reparent-with-pending-violation-microtask.html` | 0 / 1 |  |
| `html/semantics/menu/tentative/uncheck-menuitem-dismiss-popover.html` | 0 / 2 |  |
| `html/semantics/menu/tentative/valid-submenu-invalid-root.html` | 0 / 1 |  |
| `html/semantics/permission-element/bounded-css-properties.tentative.html` | 0 / 2 |  |
| `html/semantics/permission-element/bounded-sizes.tentative.html` | 0 / 3 |  |
| `html/semantics/permission-element/camera/camera-element-use-cases.https.html` | 0 / 1 |  |
| `html/semantics/permission-element/camera/camera-error-scenarios.https.html` | 0 / 2 |  |
| `html/semantics/permission-element/camera/camera-set-constraints.https.html` | 0 / 2 |  |
| `html/semantics/permission-element/camera/camera-track-attribute.https.html` | 0 / 1 |  |
| `html/semantics/permission-element/camera/idlharness.window.js` | 0 / 1 |  |
| `html/semantics/permission-element/display-values.tentative.html` | 0 / 31 | harnais : délai du harnais dépassé |
| `html/semantics/permission-element/geolocation-element/bounded-css-properties.tentative.html` | 0 / 2 |  |
| `html/semantics/permission-element/geolocation-element/bounded-sizes.tentative.html` | 0 / 3 |  |
| `html/semantics/permission-element/geolocation-element/display-values.tentative.html` | 0 / 31 | harnais : délai du harnais dépassé |
| `html/semantics/permission-element/geolocation-element/get-current-position-error.html` | 0 / 1 | harnais : Error: bidi.permissions.set_permission() is not implemented by testdriver-vendor.js |
| `html/semantics/permission-element/geolocation-element/get-current-position-success.html` | 0 / 1 | harnais : Error: bidi.permissions.set_permission() is not implemented by testdriver-vendor.js |
| `html/semantics/permission-element/geolocation-element/granted-selector.html` | 0 / 1 |  |
| `html/semantics/permission-element/geolocation-element/icon-color-defaults-to-text-color.html` | 0 / 1 |  |
| `html/semantics/permission-element/geolocation-element/idlharness.tentative.window.js` | 0 / 1 |  |
| `html/semantics/permission-element/geolocation-element/invalid-css-properties.tentative.html` | 0 / 1 |  |
| `html/semantics/permission-element/geolocation-element/negative-offset.tentative.html` | 0 / 7 |  |
| `html/semantics/permission-element/geolocation-element/no-children-rendered.tentative.html` | 0 / 1 |  |
| `html/semantics/permission-element/geolocation-element/parent-image-mask.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/permission-element/geolocation-element/parent-mask.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/permission-element/granted-selector.https.html` | 0 / 1 |  |
| `html/semantics/permission-element/install/bounded-css-properties.tentative.html` | 0 / 3 |  |
| `html/semantics/permission-element/install/bounded-sizes.tentative.html` | 0 / 3 |  |
| `html/semantics/permission-element/install/display-values.tentative.html` | 0 / 31 | harnais : délai du harnais dépassé |
| `html/semantics/permission-element/install/iframe-not-allowed.tentative.html` | 0 / 4 |  |
| `html/semantics/permission-element/install/install-element-event-handlers.tentative.html` | 0 / 4 |  |
| `html/semantics/permission-element/install/install-element-feature-detect.tentative.html` | 0 / 3 |  |
| `html/semantics/permission-element/install/install-element-invalid-targets.tentative.https.html` | 0 / 9 |  |
| `html/semantics/permission-element/install/install-element-url-attributes.tentative.html` | 0 / 7 |  |
| `html/semantics/permission-element/install/install-result-event-constructor.tentative.html` | 0 / 13 |  |
| `html/semantics/permission-element/install/invalid-css-properties.tentative.html` | 0 / 1 |  |
| `html/semantics/permission-element/install/negative-offset.tentative.html` | 0 / 7 |  |
| `html/semantics/permission-element/install/no-children-rendered.tentative.html` | 0 / 1 |  |
| `html/semantics/permission-element/install/no-focus.tentative.html` | 0 / 1 |  |
| `html/semantics/permission-element/install/parent-image-mask.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/permission-element/install/parent-mask.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/permission-element/install/sandbox-not-allowed.tentative.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/permission-element/microphone/idlharness.window.js` | 0 / 1 |  |
| `html/semantics/permission-element/microphone/microphone-element-use-cases.https.html` | 0 / 1 |  |
| `html/semantics/permission-element/microphone/microphone-error-scenarios.https.html` | 0 / 2 |  |
| `html/semantics/permission-element/microphone/microphone-set-constraints.https.html` | 0 / 3 |  |
| `html/semantics/permission-element/microphone/microphone-track-attribute.https.html` | 0 / 1 |  |
| `html/semantics/permission-element/negative-offset.tentative.html` | 0 / 7 |  |
| `html/semantics/permission-element/parent-image-mask.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/permission-element/parent-mask.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/permission-element/permission-icon/icon-color-defaults-to-text-color.html` | 0 / 1 |  |
| `html/semantics/permission-element/roles.tentative.html` | 0 / 3 |  |
| `html/semantics/permission-element/usermedia/idlharness.window.js` | 0 / 1 |  |
| `html/semantics/permission-element/usermedia/invalid-css-properties.html` | 0 / 1 |  |
| `html/semantics/permission-element/usermedia/lang-attribute-update.html` | 0 / 1 |  |
| `html/semantics/permission-element/usermedia/legacy-mode/legacy-mode.https.html` | 0 / 4 |  |
| `html/semantics/permission-element/usermedia/no-children-rendered.html` | 0 / 1 |  |
| `html/semantics/permission-element/usermedia/no-focus.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/permission-element/usermedia/oncancel-attribute.html` | 2 / 3 |  |
| `html/semantics/permission-element/usermedia/onerror-attribute.html` | 2 / 3 |  |
| `html/semantics/permission-element/usermedia/onstream-attribute.html` | 2 / 3 |  |
| `html/semantics/permission-element/usermedia/set-constraints-combinations.https.html` | 0 / 4 |  |
| `html/semantics/permission-element/usermedia/usermedia-cancel-prompt.https.html` | 0 / 1 |  |
| `html/semantics/permission-element/usermedia/usermedia-iframe.https.html` | 0 / 1 |  |
| `html/semantics/permission-element/usermedia/usermedia-set-constraints-sanitization.https.html` | 0 / 1 |  |
| `html/semantics/permission-element/usermedia/usermedia-untrusted-click.https.html` | 0 / 1 |  |
| `html/semantics/popovers/button-type-popovertarget.html` | 7 / 15 |  |
| `html/semantics/popovers/hide-other-popover-side-effects.html` | 0 / 1 |  |
| `html/semantics/popovers/imperative-invokers.html` | 1 / 10 | harnais : Test named 'normal opening' specified 1 'cleanup' function, and 1 failed. |
| `html/semantics/popovers/input-type-popovertarget.html` | 6 / 12 |  |
| `html/semantics/popovers/light-dismiss-event-ordering.html` | 0 / 11 |  |
| `html/semantics/popovers/light-dismiss-event-ordering.tentative.html` | 0 / 11 |  |
| `html/semantics/popovers/popover-active-document.html` | 0 / 1 |  |
| `html/semantics/popovers/popover-anchor-display-none.tentative.html` | 0 / 1 |  |
| `html/semantics/popovers/popover-anchor-multicol-display.html` | 0 / 2 |  |
| `html/semantics/popovers/popover-anchor-nesting.tentative.html` | 0 / 1 |  |
| `html/semantics/popovers/popover-attribute-all-elements.html` | 952 / 1101 |  |
| `html/semantics/popovers/popover-attribute-basic.html` | 63 / 249 |  |
| `html/semantics/popovers/popover-change-type.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/popovers/popover-close-request.html` | 0 / 1 |  |
| `html/semantics/popovers/popover-document-open.html` | 0 / 1 |  |
| `html/semantics/popovers/popover-events.html` | 3 / 6 |  |
| `html/semantics/popovers/popover-focus-2.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/popovers/popover-focus-3.html` | 2 / 9 |  |
| `html/semantics/popovers/popover-focus-4.html` | 1 / 3 | harnais : Test named 'Cases where the next focus candidate isn't in the direct parent scope with popovertarget invocation' specifi |
| `html/semantics/popovers/popover-focus-5.html` | 2 / 6 |  |
| `html/semantics/popovers/popover-focus-across-slot.html` | 0 / 1 |  |
| `html/semantics/popovers/popover-focus-form-control.html` | 0 / 1 |  |
| `html/semantics/popovers/popover-focus-inert-invoker.html` | 0 / 1 |  |
| `html/semantics/popovers/popover-focus-inside-shadow-dom.html` | 0 / 1 |  |
| `html/semantics/popovers/popover-focus-inside-slot.html` | 0 / 1 |  |
| `html/semantics/popovers/popover-focus-invoker-inside-popover.html` | 0 / 1 |  |
| `html/semantics/popovers/popover-focus-invoker-with-children.tentative.html` | 0 / 2 |  |
| `html/semantics/popovers/popover-focus-overflow-visible.html` | 0 / 1 |  |
| `html/semantics/popovers/popover-focus-source-contenteditable.html` | 0 / 1 |  |
| `html/semantics/popovers/popover-focus-tabindex.html` | 0 / 1 |  |
| `html/semantics/popovers/popover-focus.html` | 1 / 30 |  |
| `html/semantics/popovers/popover-hint-hierarchy.html` | 0 / 5 |  |
| `html/semantics/popovers/popover-hint-loseinterest-show-child.html` | 0 / 1 |  |
| `html/semantics/popovers/popover-hover-active-ancestors.html` | 0 / 3 |  |
| `html/semantics/popovers/popover-invoking-attribute-hint.html` | 0 / 700 |  |
| `html/semantics/popovers/popover-invoking-attribute.html` | 2 / 1402 |  |
| `html/semantics/popovers/popover-light-dismiss-command-edge-cases.html` | 9 / 11 |  |
| `html/semantics/popovers/popover-light-dismiss-command.html` | 1 / 14 |  |
| `html/semantics/popovers/popover-light-dismiss-disabled-button.html` | 1 / 3 | harnais : Test named 'Clicking on disabled command button should just close popover like normal' specified 1 'cleanup' function, a |
| `html/semantics/popovers/popover-light-dismiss-flat-tree-nested.html` | 0 / 1 |  |
| `html/semantics/popovers/popover-light-dismiss-flat-tree.html` | 0 / 1 |  |
| `html/semantics/popovers/popover-light-dismiss-hint.html` | 0 / 10 |  |
| `html/semantics/popovers/popover-light-dismiss-input-button.html` | 7 / 14 |  |
| `html/semantics/popovers/popover-light-dismiss.html` | 14 / 33 | harnais : Uncaught { message: 'assert_throws_dom: function "() => p17.showPopover()" did not throw', stack: 'Error' } |
| `html/semantics/popovers/popover-manual-hide-does-not-close-auto.html` | 1 / 2 | harnais : Test named 'Hiding a manual popover shown after an auto popover does not close the auto popover' specified 1 'cleanup' f |
| `html/semantics/popovers/popover-minimum-role.html` | 0 / 2 |  |
| `html/semantics/popovers/popover-move-documents-during-hide.html` | 0 / 3 | harnais : Uncaught InvalidStateError: le popover est déjà affiché |
| `html/semantics/popovers/popover-move-documents.html` | 1 / 3 | harnais : Uncaught ReferenceError: p1 n'est pas défini |
| `html/semantics/popovers/popover-nested-in-button.html` | 0 / 4 |  |
| `html/semantics/popovers/popover-open-in-beforetoggle.html` | 0 / 5 | harnais : Uncaught { message: 'assert_throws_dom: function "() => p6.showPopover()" did not throw', stack: 'Error' } |
| `html/semantics/popovers/popover-overlay.html` | 0 / 4 |  |
| `html/semantics/popovers/popover-removal-2.html` | 0 / 1 |  |
| `html/semantics/popovers/popover-removal-hides.html` | 0 / 6 |  |
| `html/semantics/popovers/popover-remove-attribute-during-focusing-steps.html` | 0 / 1 |  |
| `html/semantics/popovers/popover-self-invoke.html` | 1 / 3 |  |
| `html/semantics/popovers/popover-shadow-dom.html` | 1 / 3 |  |
| `html/semantics/popovers/popover-shadowhost-focus-navigation.html` | 0 / 1 |  |
| `html/semantics/popovers/popover-shadowhost-focus.html` | 2 / 4 |  |
| `html/semantics/popovers/popover-stacking.html` | 2 / 6 |  |
| `html/semantics/popovers/popover-target-element-disabled.html` | 2 / 7 |  |
| `html/semantics/popovers/popover-toggle-source.html` | 0 / 7 |  |
| `html/semantics/popovers/popover-top-layer-combinations.html` | 0 / 5 |  |
| `html/semantics/popovers/popover-top-layer-interactions.html` | 3 / 9 |  |
| `html/semantics/popovers/popover-top-layer-nesting-anchor.tentative.html` | 7 / 20 | harnais : Test named 'Nested popover=auto ancestors with dialog' specified 3 'cleanup' functions, and 1 failed. |
| `html/semantics/popovers/popover-top-layer-nesting-hints.html` | 4 / 20 | harnais : Test named 'Nested auto/hint ancestors with dialog' specified 3 'cleanup' functions, and 1 failed. |
| `html/semantics/popovers/popover-top-layer-nesting.html` | 7 / 20 | harnais : Test named 'Nested popover=auto ancestors with dialog' specified 3 'cleanup' functions, and 1 failed. |
| `html/semantics/popovers/popover-types-with-hints.html` | 5 / 7 |  |
| `html/semantics/popovers/popovertarget-disabled-fieldset.html` | 0 / 1 |  |
| `html/semantics/popovers/popovertarget-reflection.html` | 0 / 1 |  |
| `html/semantics/popovers/togglePopover.html` | 2 / 3 |  |
| `html/semantics/popovers/toggleevent-interface.html` | 35 / 40 |  |
| `html/semantics/rellist-feature-detection.html` | 0 / 4 |  |
| `html/semantics/scripting-1/the-noscript-element/non-html-noscript.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/async_001.htm` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/async_003.htm` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/async_004.htm` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/async_005.htm` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/async_006.htm` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/async_007.htm` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/async_008.htm` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/async_009.htm` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/async_010.htm` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/async_011.htm` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/change-src-attr-prepare-a-script.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/css-module/charset-2.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/css-module/charset-bom.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/css-module/charset.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/css-module/content-type-checking.html` | 0 / 6 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/css-module/cors-crossorigin-requests.html` | 0 / 3 |  |
| `html/semantics/scripting-1/the-script-element/css-module/credentials.sub.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/css-module/css-module-parse-error.html` | 0 / 3 |  |
| `html/semantics/scripting-1/the-script-element/css-module/css-module-worker-test.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/css-module/import-css-module-basic.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/css-module/import-css-module-dynamic.html` | 0 / 2 |  |
| `html/semantics/scripting-1/the-script-element/css-module/integrity.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/css-module/load-error-events.html` | 2 / 16 | harnais : Uncaught SyntaxError: module https://web-platform.test/html/semantics/scripting-1/the-script-element/css-module/resource |
| `html/semantics/scripting-1/the-script-element/css-module/redirect-relative-urls.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/css-module/redirect.html` | 0 / 3 |  |
| `html/semantics/scripting-1/the-script-element/css-module/referrer-policies.sub.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/css-module/relative-urls.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/css-module/repeated-import.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/css-module/script-element-css-src.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/data-url.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/emptyish-script-elements.html` | 6 / 7 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/007.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/008.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/009.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/010.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/011.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/012.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/018.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/020.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/026.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/028.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/030.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/031.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/032.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/033.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/035.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/036.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/040.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/041.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/046.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/047.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/049.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/050.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/052.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/053.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/057.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/061.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/062.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/063.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/066.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/067.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/068.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/071.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/execution-timing/073.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/075.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/076.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/077.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/079.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/082.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/083.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/084.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/085.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/086.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/088.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/091.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/092.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/094.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/execution-timing/095.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/096.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/097.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/101.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/execution-timing/102.html` | 0 / 0 | harnais : Uncaught TypeError: impossible de lire la propriété « step » de undefined (valeur de .t) |
| `html/semantics/scripting-1/the-script-element/execution-timing/103.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/104.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/105.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/106-defer-import.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/106-external-import.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/106-external-module-import.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/106-import-xhtml.xhtml` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/106-import.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/106-module-import.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/108.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/109.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/110.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/111.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/112.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/116.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/117.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/118.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/119.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/120.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/122.html` | 0 / 2 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/123.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/124.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/125.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/126.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/130.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/131.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/132.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/133.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/134.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/135.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/136.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/138.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/139.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/140.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/141.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/142.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/143.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/144.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/145.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/146-href.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/150-import-xhtml.xhtml` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/150-import.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/execution-timing/non-external-no-import.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/fetch-src/empty-set-while-connected.html` | 1 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/fetch-src/empty-with-base.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/fetch-src/empty.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/fetch-src/failure.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/fetch-src/whitespace-with-base-module.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/fetch-src/whitespace-with-base.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/import-attributes/invalid-import-errors-order.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/import-attributes/invalid-type-attribute-error.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/import-attributes/unsupported-attribute.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/json-module/charset-2.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/json-module/charset-bom.any.js` | 2 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/json-module/charset.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/json-module/cors-crossorigin-requests.html` | 0 / 3 |  |
| `html/semantics/scripting-1/the-script-element/json-module/credentials.sub.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/json-module/http-accept-header.sub.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/json-module/integrity.html` | 0 / 2 |  |
| `html/semantics/scripting-1/the-script-element/json-module/invalid-content-type.any.js` | 0 / 106 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/json-module/json-module-service-worker-test.https.html` | 0 / 3 |  |
| `html/semantics/scripting-1/the-script-element/json-module/load-error-events.html` | 4 / 16 | harnais : Uncaught SyntaxError: module https://web-platform.test/html/semantics/scripting-1/the-script-element/json-module/load-er |
| `html/semantics/scripting-1/the-script-element/json-module/non-object.any.js` | 2 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/json-module/parse-error-location.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/json-module/parse-error.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/json-module/referrer-policies.sub.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/json-module/repeated-imports.any.js` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/json-module/script-element-json-src.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/json-module/valid-content-type.html` | 0 / 8 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/load-error-events-1.html` | 1 / 11 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/microtasks/checkpoint-after-window-onerror-module.html` | 0 / 2 |  |
| `html/semantics/scripting-1/the-script-element/microtasks/checkpoint-after-window-onerror.html` | 0 / 2 |  |
| `html/semantics/scripting-1/the-script-element/microtasks/checkpoint-after-workerglobalscope-onerror-module.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/microtasks/checkpoint-after-workerglobalscope-onerror.html` | 1 / 2 |  |
| `html/semantics/scripting-1/the-script-element/microtasks/evaluation-order-1.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/microtasks/evaluation-order-2.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/microtasks/evaluation-order-4.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/module/charset-01.html` | 0 / 6 |  |
| `html/semantics/scripting-1/the-script-element/module/charset-02.html` | 2 / 3 |  |
| `html/semantics/scripting-1/the-script-element/module/charset-03.html` | 0 / 4 |  |
| `html/semantics/scripting-1/the-script-element/module/choice-of-error-1.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/module/choice-of-error-2.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/module/choice-of-error-3.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/module/compilation-error-1.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/module/compilation-error-2.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/module/concurrent-failed-imports.html` | 0 / 4 | harnais : Uncaught SyntaxError: module https://web-platform.test/html/semantics/scripting-1/the-script-element/module-fetch-tester |
| `html/semantics/scripting-1/the-script-element/module/credentials.sub.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/module/crossorigin.html` | 0 / 8 |  |
| `html/semantics/scripting-1/the-script-element/module/dynamic-import/alpha/base-url-worker-importScripts.html` | 0 / 0 | harnais : Uncaught SyntaxError: « ; » attendu |
| `html/semantics/scripting-1/the-script-element/module/dynamic-import/alpha/base-url-worker.sub.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/module/dynamic-import/alpha/base-url.sub.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/module/dynamic-import/blob-url-workers.window.js` | 1 / 3 |  |
| `html/semantics/scripting-1/the-script-element/module/dynamic-import/blob-url.any.js` | 1 / 6 |  |
| `html/semantics/scripting-1/the-script-element/module/dynamic-import/code-cache-base-url.html` | 0 / 6 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/module/dynamic-import/code-cache-nonce.html` | 0 / 3 |  |
| `html/semantics/scripting-1/the-script-element/module/dynamic-import/concurrent-failed-imports.any.js` | 0 / 2 |  |
| `html/semantics/scripting-1/the-script-element/module/dynamic-import/cyclic-tla-defer-eval.tentative.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/module/dynamic-import/delay-load-event.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/module/dynamic-import/dynamic-imports-credentials-setTimeout.sub.html` | 0 / 0 | harnais : Unhandled rejection: adresse de requête invalide : « http://{{domains[www2]}}:{{ports[http][0]}}/cookies/resources/set-c |
| `html/semantics/scripting-1/the-script-element/module/dynamic-import/dynamic-imports-credentials.sub.html` | 0 / 0 | harnais : Unhandled rejection: adresse de requête invalide : « http://{{domains[www2]}}:{{ports[http][0]}}/cookies/resources/set-c |
| `html/semantics/scripting-1/the-script-element/module/dynamic-import/dynamic-imports-fetch-error.sub.html` | 0 / 32 | harnais : Unhandled rejection: module https://web-platform.test/html/semantics/scripting-1/the-script-element/module/errorhandling |
| `html/semantics/scripting-1/the-script-element/module/dynamic-import/dynamic-imports-script-error.html` | 3 / 11 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/module/dynamic-import/microtasks/with-import-assertions.any.js` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/module/dynamic-import/repeated-imports.any.js` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/module/dynamic-import/shared-dependency-repeated-import.https.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/module/dynamic-import/string-compilation-base-url-external-classic.html` | 0 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/module/dynamic-import/string-compilation-base-url-external-module.html` | 0 / 10 | harnais : Uncaught TypeError: module « https://web-platform.test/html/semantics/scripting-1/the-script-element/module/dynamic-impo |
| `html/semantics/scripting-1/the-script-element/module/dynamic-import/string-compilation-base-url-inline-classic.html` | 0 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/module/dynamic-import/string-compilation-base-url-inline-module.html` | 0 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/module/dynamic-import/string-compilation-classic.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/module/dynamic-import/string-compilation-module.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/module/dynamic-import/string-compilation-nonce-classic.html` | 0 / 6 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/module/dynamic-import/string-compilation-nonce-module.html` | 0 / 6 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/module/dynamic-import/string-compilation-of-promise-result.html` | 2 / 4 |  |
| `html/semantics/scripting-1/the-script-element/module/dynamic-import/string-compilation-other-document.html` | 0 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/module/dynamic-import/v8-code-cache.html` | 0 / 20 | harnais : Uncaught TypeError: impossible de lire la propriété « then » de undefined (valeur de .promise) |
| `html/semantics/scripting-1/the-script-element/module/error-type-1.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/module/error-type-2.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/module/error-type-3.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/module/errorhandling.html` | 2 / 5 | harnais : Uncaught SyntaxError: module https://web-platform.test/html/semantics/scripting-1/the-script-element/module/errorhandlin |
| `html/semantics/scripting-1/the-script-element/module/evaluation-error-1.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/module/evaluation-error-2.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/module/evaluation-error-3.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/module/evaluation-error-4.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/module/evaluation-error-5.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/module/execorder.html` | 4 / 6 |  |
| `html/semantics/scripting-1/the-script-element/module/fetch-error-1.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/module/fetch-error-2.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/module/import-meta/import-meta-resolve-importmap.html` | 0 / 4 |  |
| `html/semantics/scripting-1/the-script-element/module/import-meta/import-meta-resolve-multiple-scripts.html` | 0 / 3 |  |
| `html/semantics/scripting-1/the-script-element/module/import-meta/import-meta-url.html` | 3 / 6 |  |
| `html/semantics/scripting-1/the-script-element/module/import-subgraph-404.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/module/inline-async-execorder.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/module/inline-async-inserted-execorder.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/module/instantiation-error-1.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/module/instantiation-error-2.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/module/instantiation-error-3.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/module/instantiation-error-4.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/module/instantiation-error-5.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/module/instantiation-error-6.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/module/instantiation-error-7.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/module/instantiation-error-8.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/module/integrity.html` | 1 / 3 |  |
| `html/semantics/scripting-1/the-script-element/module/load-error-events-inline.html` | 2 / 8 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/module/load-error-events.html` | 4 / 16 | harnais : Uncaught SyntaxError: module https://web-platform.test/html/semantics/scripting-1/the-script-element/resources/load-erro |
| `html/semantics/scripting-1/the-script-element/module/module-import-referrer-base-url.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/module/module-import-referrer.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/module/modulepreload-cross-origin-referrerpolicy.sub.html` | 0 / 7 |  |
| `html/semantics/scripting-1/the-script-element/module/modulepreload-inline-referrerpolicy.html` | 0 / 4001 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/module/modulepreload-referrer-check.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/module/modulepreload-referrerpolicy.html` | 0 / 7 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/module/nested-imports.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/module/referrer-no-referrer.sub.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/module/referrer-origin-when-cross-origin.sub.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/module/referrer-origin.sub.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/module/referrer-policy-for-descendants.sub.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/module/referrer-same-origin.sub.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/module/referrer-strict-policies.sub.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/module/referrer-unsafe-url.sub.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/module/shared-dependency-repeated-import.https.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/module/slow-cycle.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/module/specifier-error.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/module/top-level-await/sibling-imports-not-blocked.any.js` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/module/type.html` | 3 / 6 |  |
| `html/semantics/scripting-1/the-script-element/moving-between-documents-during-evaluation.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/after-prepare-createHTMLDocument-fetch-error-external-classic.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/after-prepare-createHTMLDocument-fetch-error-external-module.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/after-prepare-createHTMLDocument-parse-error-external-classic.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/after-prepare-createHTMLDocument-parse-error-external-module.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/after-prepare-createHTMLDocument-success-external-classic.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/after-prepare-createHTMLDocument-success-external-module.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/after-prepare-iframe-fetch-error-external-classic.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/after-prepare-iframe-fetch-error-external-module.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/after-prepare-iframe-parse-error-external-classic.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/after-prepare-iframe-parse-error-external-module.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/after-prepare-iframe-parse-error-inline-classic.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/after-prepare-iframe-success-external-classic.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/after-prepare-iframe-success-external-module.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/after-prepare-iframe-success-inline-classic.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/before-prepare-createHTMLDocument-fetch-error-external-classic.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/before-prepare-createHTMLDocument-fetch-error-external-module.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/before-prepare-createHTMLDocument-parse-error-external-classic.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/before-prepare-createHTMLDocument-parse-error-external-module.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/before-prepare-createHTMLDocument-parse-error-inline-classic.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/before-prepare-createHTMLDocument-parse-error-inline-module.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/before-prepare-createHTMLDocument-success-empty-src-classic.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/before-prepare-createHTMLDocument-success-empty-src-module.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/before-prepare-createHTMLDocument-success-external-classic.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/before-prepare-createHTMLDocument-success-external-module.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/before-prepare-createHTMLDocument-success-inline-classic.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/before-prepare-createHTMLDocument-success-inline-module.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/before-prepare-iframe-fetch-error-external-classic.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/before-prepare-iframe-fetch-error-external-module.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/before-prepare-iframe-parse-error-external-classic.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/before-prepare-iframe-parse-error-external-module.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/before-prepare-iframe-parse-error-inline-classic.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/before-prepare-iframe-parse-error-inline-module.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/before-prepare-iframe-success-empty-src-classic.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/before-prepare-iframe-success-empty-src-module.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/before-prepare-iframe-success-external-classic.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/before-prepare-iframe-success-external-module.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/before-prepare-iframe-success-inline-classic.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/before-prepare-iframe-success-inline-module.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/move-back-createHTMLDocument-fetch-error-external-classic.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/move-back-createHTMLDocument-fetch-error-external-module.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/move-back-createHTMLDocument-parse-error-external-classic.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/move-back-createHTMLDocument-parse-error-external-module.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/move-back-createHTMLDocument-success-external-classic.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/move-back-createHTMLDocument-success-external-module.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/move-back-iframe-fetch-error-external-classic.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/move-back-iframe-fetch-error-external-module.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/move-back-iframe-parse-error-external-classic.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/move-back-iframe-parse-error-external-module.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/move-back-iframe-parse-error-inline-classic.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/move-back-iframe-success-external-classic.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/move-back-iframe-success-external-module.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/move-back-iframe-success-inline-classic.html` | 1 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/ordering/delay-load-event-1.html` | 1 / 2 |  |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/ordering/delay-load-event-2.html` | 1 / 2 |  |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/ordering/in-order.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/moving-between-documents/ordering/parser-blocking.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/muted-errors.sub.html` | 0 / 0 | le harnais n'a pas terminé |
| `html/semantics/scripting-1/the-script-element/nomodule-set-on-async-classic-script.html` | 1 / 2 |  |
| `html/semantics/scripting-1/the-script-element/nomodule-set-on-synchronously-loaded-classic-scripts.html` | 1 / 2 |  |
| `html/semantics/scripting-1/the-script-element/script-charset-01.html` | 0 / 6 |  |
| `html/semantics/scripting-1/the-script-element/script-charset-02.html` | 0 / 3 |  |
| `html/semantics/scripting-1/the-script-element/script-charset-03.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/script-crossorigin-network.sub.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/script-crossorigin.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/script-defer-xhtml.xhtml` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/script-defer.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/script-error-status-fires-error.html` | 0 / 2 |  |
| `html/semantics/scripting-1/the-script-element/script-not-found-not-executed.html` | 1 / 2 |  |
| `html/semantics/scripting-1/the-script-element/script-onerror-insertion-point-1.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/script-onerror-insertion-point-2.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/script-onload-insertion-point.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/script-onload-string.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/script-referrerpolicy-idl.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/script-supports.html` | 0 / 5 |  |
| `html/semantics/scripting-1/the-script-element/script-text-modifications-csp.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/script-text-modifications.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/script-text-xhtml.xhtml` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/script-text.html` | 5 / 6 |  |
| `html/semantics/scripting-1/the-script-element/script-type-and-language-js-svg.svg` | 53 / 444 |  |
| `html/semantics/scripting-1/the-script-element/script-type-and-language-js-xhtml.xhtml` | 150 / 456 |  |
| `html/semantics/scripting-1/the-script-element/script-type-and-language-js.html` | 150 / 456 |  |
| `html/semantics/scripting-1/the-script-element/script-type-whitespace.html` | 1 / 5 |  |
| `html/semantics/scripting-1/the-script-element/src-empty-or-invalid-error-event-queued.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/svg-script-fragment-parsed-not-executed-on-clone.html` | 2 / 4 |  |
| `html/semantics/scripting-1/the-script-element/text-module/charset-2.html` | 1 / 2 |  |
| `html/semantics/scripting-1/the-script-element/text-module/charset-bom.any.js` | 2 / 3 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/text-module/charset.html` | 1 / 5 |  |
| `html/semantics/scripting-1/the-script-element/text-module/cors-crossorigin-requests.html` | 0 / 2 |  |
| `html/semantics/scripting-1/the-script-element/text-module/credentials.sub.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/text-module/http-accept-header.sub.html` | 0 / 2 |  |
| `html/semantics/scripting-1/the-script-element/text-module/ignore-content-type.any.js` | 2 / 106 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/text-module/ignore-extension.any.js` | 2 / 4 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/text-module/integrity.html` | 0 / 2 |  |
| `html/semantics/scripting-1/the-script-element/text-module/load-error-events.html` | 4 / 16 | harnais : Uncaught SyntaxError: module https://web-platform.test/html/semantics/scripting-1/the-script-element/text-module/load-er |
| `html/semantics/scripting-1/the-script-element/text-module/referrer-policies.sub.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/text-module/repeated-imports.any.js` | 0 / 7 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-script-element/text-module/script-element-json-src.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-script-element/text-module/service-worker-test.https.html` | 0 / 3 |  |
| `html/semantics/scripting-1/the-template-element/additions-to-parsing-xhtml-documents/node-document.html` | 2 / 5 |  |
| `html/semantics/scripting-1/the-template-element/additions-to-parsing-xhtml-documents/template-child-nodes.html` | 0 / 4 |  |
| `html/semantics/scripting-1/the-template-element/additions-to-serializing-xhtml-documents/outerhtml.html` | 0 / 3 |  |
| `html/semantics/scripting-1/the-template-element/definitions/template-contents-owner-test-001.html` | 0 / 2 |  |
| `html/semantics/scripting-1/the-template-element/definitions/template-contents-owner-test-002.html` | 0 / 3 |  |
| `html/semantics/scripting-1/the-template-element/innerhtml-on-templates/innerhtml.html` | 3 / 4 |  |
| `html/semantics/scripting-1/the-template-element/reflect-html-for.html` | 267 / 403 |  |
| `html/semantics/scripting-1/the-template-element/serializing-html-templates/outerhtml.html` | 0 / 3 |  |
| `html/semantics/scripting-1/the-template-element/template-element/node-document-changes.html` | 1 / 6 |  |
| `html/semantics/scripting-1/the-template-element/template-element/template-as-a-descendant.html` | 10 / 12 |  |
| `html/semantics/scripting-1/the-template-element/template-element/template-content-hierarcy.html` | 0 / 3 |  |
| `html/semantics/scripting-1/the-template-element/template-element/template-content.html` | 207 / 216 |  |
| `html/semantics/scripting-1/the-template-element/template-element/template-descendant-frameset.html` | 0 / 3 |  |
| `html/semantics/scripting-1/the-template-element/template-element/template-descendant-head.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-template-element/template-for/chained-streaming-templates-with-custom-element.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-template-element/template-for/stream-append-has-invalidation.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-template-element/template-for/stream-append-positional-pseudo-class-invalidation.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-template-element/template-for/stream-append-with-template-for.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-template-element/template-for/stream-head-scope.html` | 0 / 13 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-append-element.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-append-text-and-element.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-case-sensitive-dom-inserted.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-case-sensitive.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-empty-hash.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-empty.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-foster-parenting.html` | 0 / 6 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-gradual-append.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-html-setters.html` | 5 / 28 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-innerHTML-ambiguous.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-innerHTML-within.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-invokes-pagehide-with-end.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-invokes-pagehide-without-end.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-move-cross-document.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-move-end-while-appending.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-move-marker-while-appending.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-move-start-while-appending.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-move-while-appending.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-multiple-cases.html` | 0 / 414 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-multiple.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-mutation-records.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-nested-patch.html` | 0 / 3 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-only-html-001.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-pagehide-dynamic-markup-insertion.html` | 0 / 8 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-prepend-element.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-prepend-text.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-replace-children-with-element.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-replace-children-with-text.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-replace.html` | 1 / 3 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-sanitization.html` | 0 / 4 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-script-children.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-script-replace.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-scripting-nested.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-shadow-nested.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-shadow-root.html` | 0 / 2 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-shadow.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-supersedes-invalid-shadowrootmode.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-title.html` | 0 / 1 |  |
| `html/semantics/scripting-1/the-template-element/template-for/template-for-unclosed-content.html` | 0 / 10 |  |
| `html/semantics/sections/headingoffset-and-headingreset.html` | 0 / 61 |  |
| `html/semantics/sections/headingoffset-mutations.html` | 0 / 46 | harnais : Test named 'manual slot assignment does not change heading levels' specified 1 'cleanup' function, and 1 failed. |
| `html/semantics/selectors/case-sensitivity/values.window.js` | 184 / 276 |  |
| `html/semantics/selectors/pseudo-classes/active-disabled.html` | 0 / 5 | harnais : délai du harnais dépassé |
| `html/semantics/selectors/pseudo-classes/autofill.html` | 0 / 2 |  |
| `html/semantics/selectors/pseudo-classes/checked-indeterminate.window.js` | 0 / 1 |  |
| `html/semantics/selectors/pseudo-classes/checked-type-change.html` | 0 / 1 |  |
| `html/semantics/selectors/pseudo-classes/checked.html` | 1 / 3 |  |
| `html/semantics/selectors/pseudo-classes/default.html` | 0 / 2 |  |
| `html/semantics/selectors/pseudo-classes/dir-dynamic.html` | 0 / 4 |  |
| `html/semantics/selectors/pseudo-classes/dir-html-input-dynamic-text.html` | 0 / 1 |  |
| `html/semantics/selectors/pseudo-classes/dir.html` | 0 / 3 |  |
| `html/semantics/selectors/pseudo-classes/focus-autofocus.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/selectors/pseudo-classes/indeterminate-type-change.html` | 0 / 1 |  |
| `html/semantics/selectors/pseudo-classes/indeterminate.html` | 1 / 6 |  |
| `html/semantics/selectors/pseudo-classes/input-checkbox-switch.tentative.window.js` | 2 / 6 |  |
| `html/semantics/selectors/pseudo-classes/inrange-outofrange-time-reversed.html` | 0 / 4 |  |
| `html/semantics/selectors/pseudo-classes/inrange-outofrange-type-change.html` | 0 / 2 |  |
| `html/semantics/selectors/pseudo-classes/inrange-outofrange.html` | 0 / 6 |  |
| `html/semantics/selectors/pseudo-classes/invalid-after-clone.html` | 0 / 1 |  |
| `html/semantics/selectors/pseudo-classes/placeholder-shown-type-change.html` | 0 / 1 |  |
| `html/semantics/selectors/pseudo-classes/readwrite-readonly-type-change.html` | 0 / 1 |  |
| `html/semantics/selectors/pseudo-classes/readwrite-readonly.html` | 10 / 25 |  |
| `html/semantics/selectors/pseudo-classes/required-optional-hidden.html` | 0 / 1 |  |
| `html/semantics/selectors/pseudo-classes/required-optional.html` | 3 / 6 |  |
| `html/semantics/selectors/pseudo-classes/valid-invalid-fieldset-disconnected.html` | 0 / 2 |  |
| `html/semantics/selectors/pseudo-classes/valid-invalid.html` | 18 / 30 |  |
| `html/semantics/tabular-data/attributes-common-to-td-and-th-elements/cellIndex.html` | 5 / 6 |  |
| `html/semantics/tabular-data/processing-model-1/col-span-limits.html` | 0 / 5 |  |
| `html/semantics/tabular-data/processing-model-1/span-limits.html` | 2 / 6 |  |
| `html/semantics/tabular-data/the-table-element/caption-methods.html` | 15 / 18 |  |
| `html/semantics/tabular-data/the-table-element/createTBody.html` | 14 / 15 |  |
| `html/semantics/tabular-data/the-table-element/delete-caption.html` | 5 / 6 |  |
| `html/semantics/tabular-data/the-table-element/parentless-props.html` | 0 / 3 |  |
| `html/semantics/tabular-data/the-table-element/remove-row.html` | 5 / 6 |  |
| `html/semantics/tabular-data/the-table-element/tFoot.html` | 1 / 2 |  |
| `html/semantics/tabular-data/the-table-element/tHead.html` | 2 / 3 |  |
| `html/semantics/tabular-data/the-table-element/table-rows-live-move.html` | 0 / 1 |  |
| `html/semantics/tabular-data/the-table-element/table-rows.html` | 0 / 5 |  |
| `html/semantics/tabular-data/the-tbody-element/deleteRow.html` | 5 / 6 |  |
| `html/semantics/tabular-data/the-tr-element/cells.html` | 0 / 1 |  |
| `html/semantics/tabular-data/the-tr-element/deleteCell.html` | 5 / 6 |  |
| `html/semantics/tabular-data/the-tr-element/rowIndex.html` | 5 / 12 |  |
| `html/semantics/text-level-semantics/the-a-element/a-stringifier.html` | 9 / 11 |  |
| `html/semantics/text-level-semantics/the-a-element/a-type.historical.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/semantics/text-level-semantics/the-time-element/001.html` | 4 / 8 |  |
| `html/semantics/the-button-element/command-and-commandfor/button-event-dispatch-content-attribute.html` | 0 / 2 |  |
| `html/semantics/the-button-element/command-and-commandfor/button-event-dispatch.html` | 34 / 57 |  |
| `html/semantics/the-button-element/command-and-commandfor/button-type-behavior.html` | 1 / 23 | harnais : Test named 'Button type=reset in form should trigger form reset and not toggle popover' specified 1 'cleanup' function,  |
| `html/semantics/the-button-element/command-and-commandfor/button-type-reflection.html` | 9 / 27 |  |
| `html/semantics/the-button-element/command-and-commandfor/command-reflection.html` | 8 / 16 |  |
| `html/semantics/the-button-element/command-and-commandfor/commandfor-shadow-host.html` | 0 / 5 |  |
| `html/semantics/the-button-element/command-and-commandfor/event-dispatch-shadow.html` | 0 / 1 |  |
| `html/semantics/the-button-element/command-and-commandfor/event-interface.html` | 0 / 23 |  |
| `html/semantics/the-button-element/command-and-commandfor/fullscreen-behavior.tentative.html` | 2 / 11 | harnais : délai du harnais dépassé |
| `html/semantics/the-button-element/command-and-commandfor/interface.html` | 1 / 11 |  |
| `html/semantics/the-button-element/command-and-commandfor/invalid-element-types.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/semantics/the-button-element/command-and-commandfor/on-audio-behavior.tentative.html` | 5 / 14 | harnais : Unhandled rejection: lecture non prise en charge |
| `html/semantics/the-button-element/command-and-commandfor/on-details-behavior.tentative.html` | 11 / 19 |  |
| `html/semantics/the-button-element/command-and-commandfor/on-dialog-behavior.html` | 39 / 104 |  |
| `html/semantics/the-button-element/command-and-commandfor/on-dialog-disconnect.html` | 0 / 1 |  |
| `html/semantics/the-button-element/command-and-commandfor/on-dialog-invalid-behavior.html` | 31 / 40 |  |
| `html/semantics/the-button-element/command-and-commandfor/on-input-number.tentative.html` | 2 / 6 |  |
| `html/semantics/the-button-element/command-and-commandfor/on-popover-behavior.html` | 13 / 28 |  |
| `html/semantics/the-button-element/command-and-commandfor/on-popover-disconnect.html` | 0 / 1 |  |
| `html/semantics/the-button-element/command-and-commandfor/on-popover-invalid-behavior.html` | 8 / 16 |  |
| `html/semantics/the-button-element/command-and-commandfor/on-scroll-behavior.tentative.html` | 19 / 35 |  |
| `html/semantics/the-button-element/command-and-commandfor/on-video-behavior.tentative.html` | 5 / 12 | harnais : Unhandled rejection: lecture non prise en charge |
| `html/semantics/the-button-element/command-and-commandfor/source-attribute-retargeting.html` | 0 / 3 |  |
| `html/semantics/the-button-element/command-and-commandfor/toggleevent-source-attribute-retargeting.html` | 0 / 3 |  |
| `html/syntax/charset/inheritance-bogus-meta-utf-8.html` | 1 / 3 |  |
| `html/syntax/charset/inheritance-bogus-meta.html` | 1 / 4 | harnais : délai du harnais dépassé |
| `html/syntax/charset/meta-charset-double-quotes.html` | 0 / 1 |  |
| `html/syntax/charset/meta-charset-no-quotes.html` | 0 / 1 |  |
| `html/syntax/charset/meta-charset-single-quotes.html` | 0 / 1 |  |
| `html/syntax/charset/meta-http-equiv-attribute-order.html` | 0 / 1 |  |
| `html/syntax/charset/meta-http-equiv-double-quotes.html` | 0 / 1 |  |
| `html/syntax/charset/meta-http-equiv-no-quotes.html` | 0 / 1 |  |
| `html/syntax/charset/meta-http-equiv-single-quotes.html` | 0 / 1 |  |
| `html/syntax/charset/with-inheritance.html` | 0 / 22 |  |
| `html/syntax/charset/xhr.html` | 1 / 2 | harnais : délai du harnais dépassé |
| `html/syntax/parsing-html-fragments/fragment-parse-form-in-template-002.html` | 0 / 1 |  |
| `html/syntax/parsing-html-fragments/fragment-parse-form-owner-001.html` | 2 / 4 |  |
| `html/syntax/parsing-html-fragments/innerHTML-setter-default-namespace.xhtml` | 0 / 2 |  |
| `html/syntax/parsing-html-fragments/tokenizer-modes-001.html` | 0 / 10 |  |
| `html/syntax/parsing-xml-fragments/innerHTML-already-started.html` | 0 / 5 |  |
| `html/syntax/parsing/DOMContentLoaded-defer-task-order.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/parsing/DOMContentLoaded-defer.html` | 0 / 1 |  |
| `html/syntax/parsing/Object-prototype-names.html` | 1 / 2 |  |
| `html/syntax/parsing/adoption-agency-reparenting-cycle.html` | 0 / 1 |  |
| `html/syntax/parsing/adoption-agency-reparenting-document-target.html` | 0 / 1 |  |
| `html/syntax/parsing/adoption-agency-reparenting-inner-loop-cycle.html` | 0 / 1 |  |
| `html/syntax/parsing/adoption-agency-reparenting-pagehide.tentative.html` | 0 / 1 |  |
| `html/syntax/parsing/adoption-agency-reparenting-shadow-cycle.html` | 0 / 1 |  |
| `html/syntax/parsing/adoption-agency-reparenting-template-cycle.html` | 0 / 1 |  |
| `html/syntax/parsing/adoption-agency-reparenting.html` | 2 / 5 |  |
| `html/syntax/parsing/ambiguous-ampersand.html` | 1 / 3 |  |
| `html/syntax/parsing/cdata-in-integration-point-fragment.html` | 2 / 15 |  |
| `html/syntax/parsing/custom-element-reparenting-shadow-cycle.html` | 0 / 0 | boucle sans fin (processus tué) |
| `html/syntax/parsing/custom-element-reparenting-template-cycle.html` | 0 / 1 |  |
| `html/syntax/parsing/doctype-system-identifier-distinction.html` | 2 / 6 |  |
| `html/syntax/parsing/form-element-pointer-in-template.html` | 0 / 3 |  |
| `html/syntax/parsing/foster-parenting-into-document-with-element-child.html` | 0 / 1 |  |
| `html/syntax/parsing/foster-parenting-into-template-element.html` | 0 / 1 |  |
| `html/syntax/parsing/foster-parenting-moved-table.window.js` | 3 / 6 |  |
| `html/syntax/parsing/html5lib_url.html` | 0 / 1 |  |
| `html/syntax/parsing/html5lib_write.html` | 0 / 1 |  |
| `html/syntax/parsing/html5lib_write_single.html` | 0 / 1 |  |
| `html/syntax/parsing/insert-into-nonempty-document.html` | 0 / 4 |  |
| `html/syntax/parsing/math-parse01.html` | 7 / 9 |  |
| `html/syntax/parsing/meta-inhead-insertion-mode.html` | 0 / 1 |  |
| `html/syntax/parsing/misnested-form-in-template.html` | 0 / 1 |  |
| `html/syntax/parsing/named-character-references.html` | 385 / 2231 |  |
| `html/syntax/parsing/noscript-template-querySelector.html` | 0 / 3 |  |
| `html/syntax/parsing/quotes-in-meta.html` | 0 / 1 |  |
| `html/syntax/parsing/template/additions-to-the-in-body-insertion-mode/generating-of-implied-end-tags.html` | 2 / 6 |  |
| `html/syntax/parsing/template/additions-to-the-in-body-insertion-mode/start-tag-html.html` | 0 / 1 |  |
| `html/syntax/parsing/template/additions-to-the-in-frameset-insertion-mode/end-tag-frameset.html` | 0 / 1 |  |
| `html/syntax/parsing/template/additions-to-the-in-head-insertion-mode/generating-of-implied-end-tags.html` | 1 / 6 |  |
| `html/syntax/parsing/template/additions-to-the-in-table-insertion-mode/end-tag-table.html` | 0 / 1 |  |
| `html/syntax/parsing/template/appending-to-a-template/template-child-nodes.html` | 0 / 4 |  |
| `html/syntax/parsing/template/clearing-the-stack-back-to-a-given-context/clearing-stack-back-to-a-table-body-context.html` | 12 / 33 |  |
| `html/syntax/parsing/template/clearing-the-stack-back-to-a-given-context/clearing-stack-back-to-a-table-context.html` | 0 / 6 |  |
| `html/syntax/parsing/template/clearing-the-stack-back-to-a-given-context/clearing-stack-back-to-a-table-row-context.html` | 0 / 3 |  |
| `html/syntax/parsing/template/creating-an-element-for-the-token/template-owner-document.html` | 205 / 223 |  |
| `html/syntax/parsing/the-end.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/syntax/parsing/unclosed-svg-script.html` | 2 / 5 |  |
| `html/syntax/serializing-html-fragments/escaping.html` | 4 / 9 | harnais : délai du harnais dépassé |
| `html/syntax/serializing-html-fragments/outerHTML.html` | 111 / 112 |  |
| `html/syntax/serializing-html-fragments/processing-instructions.html` | 0 / 3 |  |
| `html/syntax/serializing-html-fragments/serializing-lt-gt.html` | 0 / 2 |  |
| `html/syntax/serializing-html-fragments/serializing.html` | 167 / 238 |  |
| `html/syntax/serializing-html-fragments/template.html` | 1 / 2 |  |
| `html/syntax/serializing-xml-fragments/outerHTML.html` | 1 / 112 |  |
| `html/syntax/speculative-charset/speculative-script.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/expect-no-linked-resources/no-speculative-fetch.tentative.optional.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/document-write/base-href-script-src.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/base-in-dsd.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/base-nonexistent.tentative.sub.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/document-write/image-src.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/img-data-src.tentative.sub.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/document-write/img-src-crossorigin.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/img-src-loading-lazy.tentative.sub.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/document-write/img-src-referrerpolicy-no-referrer.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/img-src.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/img-srcset.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/link-no-rel.tentative.sub.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/document-write/link-rel-alternate-stylesheet.tentative.sub.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/document-write/link-rel-preload-as-font-crossorigin.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/link-rel-preload-as-image.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/link-rel-preload-as-script.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/link-rel-preload-as-style.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/link-rel-stylesheet-crossorigin.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/link-rel-stylesheet-disabled.tentative.sub.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/document-write/link-rel-stylesheet-integrity.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/link-rel-stylesheet-nomatch-media.tentative.sub.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/document-write/link-rel-stylesheet-referrerpolicy-no-referrer.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/link-rel-stylesheet-type-text-css.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/link-rel-stylesheet-unsupported-type.tentative.sub.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/document-write/link-rel-stylesheet.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/math-font-face-script-src.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/math-font-script-src.tentative.sub.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/document-write/math-script-src.tentative.sub.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/document-write/meta-charset-script-src.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/meta-csp-img-src-asterisk.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/meta-csp-img-src-in-dsd.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/meta-csp-img-src-none.tentative.sub.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/document-write/meta-referrer-no-referrer-img-src.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/meta-viewport-link-stylesheet-media.tentative.sub.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/document-write/nested-template-shadowrootmode-1.tentative.sub.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/document-write/nested-template-shadowrootmode-2.tentative.sub.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/document-write/picture-source-br-img.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/picture-source-no-img.tentative.sub.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/document-write/picture-source-nomatch-media.tentative.sub.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/document-write/picture-source-unsupported-type.tentative.sub.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/document-write/script-src-async.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/script-src-crossorigin.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/script-src-defer.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/script-src-integrity.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/script-src-module.tentative.sub.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/document-write/script-src-nomodule.tentative.sub.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/document-write/script-src-referrerpolicy-no-referrer.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/script-src-type-application-ecmascript.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/script-src-unsupported-type.tentative.sub.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/document-write/script-src.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/svg-image-href.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/svg-image-src.tentative.sub.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/document-write/svg-image-xlinkhref.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/svg-script-href.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/svg-script-src.tentative.sub.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/document-write/svg-script-xlinkhref.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/template-img-src.tentative.sub.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/document-write/template-link-stylesheet.tentative.sub.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/document-write/template-script-src.tentative.sub.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/document-write/template-shadowrootmode-img-src.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/template-shadowrootmode-link-stylesheet.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/template-shadowrootmode-script-src.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/video-poster.tentative.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/document-write/xmp-script-src.tentative.sub.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/page-load/base-href-script-src.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/base-in-dsd.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/base-nonexistent.tentative.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/page-load/image-src.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/img-data-src.tentative.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/page-load/img-src-crossorigin.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/img-src-loading-lazy.tentative.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/page-load/img-src-referrerpolicy-no-referrer.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/img-src.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/img-srcset.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/link-no-rel.tentative.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/page-load/link-rel-alternate-stylesheet.tentative.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/page-load/link-rel-preload-as-font-crossorigin.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/link-rel-preload-as-image.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/link-rel-preload-as-script.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/link-rel-preload-as-style.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/link-rel-stylesheet-crossorigin.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/link-rel-stylesheet-disabled.tentative.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/page-load/link-rel-stylesheet-integrity.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/link-rel-stylesheet-nomatch-media.tentative.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/page-load/link-rel-stylesheet-referrerpolicy-no-referrer.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/link-rel-stylesheet-type-text-css.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/link-rel-stylesheet-unsupported-type.tentative.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/page-load/link-rel-stylesheet.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/math-font-face-script-src.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/math-font-script-src.tentative.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/page-load/math-script-src.tentative.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/page-load/meta-charset-script-src.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/meta-csp-img-src-asterisk.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/meta-csp-img-src-in-dsd.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/meta-csp-img-src-none.tentative.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/page-load/meta-referrer-no-referrer-img-src.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/meta-viewport-link-stylesheet-media.tentative.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/page-load/nested-template-shadowrootmode-1.tentative.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/page-load/nested-template-shadowrootmode-2.tentative.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/page-load/picture-source-br-img.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/picture-source-no-img.tentative.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/page-load/picture-source-nomatch-media.tentative.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/page-load/picture-source-unsupported-type.tentative.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/page-load/script-src-async.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/script-src-crossorigin.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/script-src-defer.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/script-src-integrity.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/script-src-module.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/script-src-nomodule.tentative.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/page-load/script-src-referrerpolicy-no-referrer.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/script-src-type-application-ecmascript.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/script-src-unsupported-type.tentative.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/page-load/script-src.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/svg-image-href.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/svg-image-src.tentative.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/page-load/svg-image-xlinkhref.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/svg-script-href.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/svg-script-src.tentative.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/page-load/svg-script-xlinkhref.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/template-img-src.tentative.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/page-load/template-link-stylesheet.tentative.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/page-load/template-script-src.tentative.html` | 0 / 1 |  |
| `html/syntax/speculative-parsing/generated/page-load/template-shadowrootmode-img-src.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/template-shadowrootmode-link-stylesheet.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/template-shadowrootmode-script-src.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/video-poster.tentative.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/syntax/speculative-parsing/generated/page-load/xmp-script-src.tentative.html` | 0 / 1 |  |
| `html/syntax/xmldecl/xmldecl-1.html` | 0 / 40 |  |
| `html/syntax/xmldecl/xmldecl-2.html` | 7 / 47 |  |
| `html/syntax/xmldecl/xmldecl-3.html` | 0 / 1 |  |
| `html/the-xhtml-syntax/parsing-xhtml-documents/xhtml-mathml-dtd-entity-1.htm` | 279 / 2125 |  |
| `html/the-xhtml-syntax/parsing-xhtml-documents/xhtml-mathml-dtd-entity-10.htm` | 385 / 2231 |  |
| `html/the-xhtml-syntax/parsing-xhtml-documents/xhtml-mathml-dtd-entity-11.htm` | 279 / 2125 |  |
| `html/the-xhtml-syntax/parsing-xhtml-documents/xhtml-mathml-dtd-entity-12.htm` | 279 / 2125 |  |
| `html/the-xhtml-syntax/parsing-xhtml-documents/xhtml-mathml-dtd-entity-2.htm` | 279 / 2125 |  |
| `html/the-xhtml-syntax/parsing-xhtml-documents/xhtml-mathml-dtd-entity-3.htm` | 279 / 2125 |  |
| `html/the-xhtml-syntax/parsing-xhtml-documents/xhtml-mathml-dtd-entity-4.htm` | 279 / 2125 |  |
| `html/the-xhtml-syntax/parsing-xhtml-documents/xhtml-mathml-dtd-entity-5.htm` | 279 / 2125 |  |
| `html/the-xhtml-syntax/parsing-xhtml-documents/xhtml-mathml-dtd-entity-6.htm` | 279 / 2125 |  |
| `html/the-xhtml-syntax/parsing-xhtml-documents/xhtml-mathml-dtd-entity-7.htm` | 279 / 2125 |  |
| `html/the-xhtml-syntax/parsing-xhtml-documents/xhtml-mathml-dtd-entity-8.htm` | 279 / 2125 |  |
| `html/the-xhtml-syntax/parsing-xhtml-documents/xhtml-mathml-dtd-entity-9.htm` | 279 / 2125 |  |
| `html/the-xhtml-syntax/parsing-xhtml-documents/xml-stylesheet-blocking.xhtml` | 0 / 1 |  |
| `html/user-activation/activation-trigger-keyboard-enter.html` | 0 / 1 |  |
| `html/user-activation/activation-trigger-mouse-left.html` | 0 / 1 |  |
| `html/user-activation/activation-trigger-mouse-right.html` | 0 / 1 |  |
| `html/user-activation/activation-trigger-pointerevent.html` | 0 / 1 |  |
| `html/user-activation/activation-trigger-touch.tentative.html` | 2 / 2 | harnais : Unhandled rejection: impossible de lire la propriété « isActive » de undefined (valeur de .userActivation) |
| `html/user-activation/chained-setTimeout.html` | 1 / 7 |  |
| `html/user-activation/consumption-crossorigin.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/user-activation/consumption-sameorigin.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/user-activation/detached-iframe.html` | 0 / 1 |  |
| `html/user-activation/message-event-activation-api-iframe-cross-origin.sub.tentative.html` | 0 / 1 |  |
| `html/user-activation/message-event-init.tentative.html` | 1 / 2 |  |
| `html/user-activation/navigate-to-crossorigin-redirect.html` | 0 / 1 |  |
| `html/user-activation/navigate-to-sameorigin.html` | 0 / 1 |  |
| `html/user-activation/navigation-state-reset-crossorigin.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/user-activation/navigation-state-reset-sameorigin.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/user-activation/no-activation-thru-escape-key.html` | 0 / 1 | harnais : Unhandled rejection: assert_true: keydown event fired before keyup expected true got false |
| `html/user-activation/propagation-crossorigin.sub.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/user-activation/propagation-same-and-cross-origin.sub.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/user-activation/propagation-sameorigin.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/user-activation/user-activation-interface.html` | 0 / 1 |  |
| `html/webappapis/animation-frames/callback-cross-realm-report-exception.html` | 0 / 1 |  |
| `html/webappapis/animation-frames/callback-timestamp.html` | 0 / 1 |  |
| `html/webappapis/animation-frames/nested-raf-not-sync.html` | 0 / 1 |  |
| `html/webappapis/animation-frames/no-interference-timers.html` | 0 / 5 | harnais : délai du harnais dépassé |
| `html/webappapis/animation-frames/spurious-frame-callbacks-optimization.html` | 0 / 1 |  |
| `html/webappapis/atob/base64.any.js` | 285 / 286 |  |
| `html/webappapis/dynamic-markup-insertion/closing-the-input-stream/document-close-with-pending-script.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/webappapis/dynamic-markup-insertion/closing-the-input-stream/document.close-01.xhtml` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/closing-the-input-stream/load-event-after-location-set-during-write.window.js` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/001.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/003.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/004.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/005.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/007.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/008.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/009.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/010.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/011.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/012.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/013.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/014.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/015.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/017.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/018.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/019.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/020.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/021.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/022.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/023.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/024.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/025.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/026.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/027.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/028.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/029.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/030.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/031.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/033.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/034.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/035.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/036.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/037.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/038.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/039.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/041.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/042.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/043.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/045.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/046.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/047.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/webappapis/dynamic-markup-insertion/document-write/049.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/050.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/051.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/contentType.window.js` | 1 / 4 | harnais : délai du harnais dépassé |
| `html/webappapis/dynamic-markup-insertion/document-write/document.write-01.xhtml` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/document.write-02.html` | 1 / 3 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/iframe_001.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/iframe_002.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/iframe_003.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/iframe_004.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/iframe_005.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/iframe_006.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/iframe_007.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/iframe_008.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/iframe_009.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/module-delayed.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/webappapis/dynamic-markup-insertion/document-write/module-dynamic-import.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/webappapis/dynamic-markup-insertion/document-write/module-static-import-delayed.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/webappapis/dynamic-markup-insertion/document-write/module-static-import.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/module-tla-delayed.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/webappapis/dynamic-markup-insertion/document-write/module-tla-immediate-promise.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/webappapis/dynamic-markup-insertion/document-write/module-tla-import.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/webappapis/dynamic-markup-insertion/document-write/mutation-observer.html` | 0 / 4 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/script_005.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/script_006.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/script_008.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/script_010.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/script_011.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/script_012.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/script_013.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-write/write-active-document.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/webappapis/dynamic-markup-insertion/document-writeln/document.writeln-01.xhtml` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/document-writeln/document.writeln-02.html` | 1 / 3 |  |
| `html/webappapis/dynamic-markup-insertion/document-writeln/document.writeln-03.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/html-unsafe-methods/Document-parseHTMLUnsafe-encoding.html` | 0 / 0 | harnais : Error: assert_equals: the meta charset must be in effect, making the main document windows-1252 expected "windows-1252"  |
| `html/webappapis/dynamic-markup-insertion/html-unsafe-methods/Document-parseHTMLUnsafe-style-attribute.html` | 0 / 0 | harnais : TypeError: .parseHTMLUnsafe n'est pas une fonction (c'est undefined) |
| `html/webappapis/dynamic-markup-insertion/html-unsafe-methods/Document-parseHTMLUnsafe-url-base-pushstate.html` | 0 / 6 |  |
| `html/webappapis/dynamic-markup-insertion/html-unsafe-methods/Document-parseHTMLUnsafe-url-base.html` | 0 / 6 |  |
| `html/webappapis/dynamic-markup-insertion/html-unsafe-methods/Document-parseHTMLUnsafe-url-moretests.html` | 1 / 5 |  |
| `html/webappapis/dynamic-markup-insertion/html-unsafe-methods/Document-parseHTMLUnsafe-url-pushstate.html` | 0 / 6 |  |
| `html/webappapis/dynamic-markup-insertion/html-unsafe-methods/Document-parseHTMLUnsafe-url.html` | 0 / 6 |  |
| `html/webappapis/dynamic-markup-insertion/html-unsafe-methods/Document-parseHTMLUnsafe.html` | 0 / 0 | harnais : TypeError: .parseHTMLUnsafe n'est pas une fonction (c'est undefined) |
| `html/webappapis/dynamic-markup-insertion/html-unsafe-methods/Element-setHTMLUnsafe-04.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/html-unsafe-methods/setHTMLUnsafe-CEReactions.html` | 0 / 2 |  |
| `html/webappapis/dynamic-markup-insertion/html-unsafe-methods/setHTMLUnsafe-runScripts.html` | 8 / 56 | harnais : Uncaught TypeError: impossible de lire la propriété « push » de undefined (valeur de .operations) |
| `html/webappapis/dynamic-markup-insertion/html-unsafe-methods/setHTMLUnsafe-xml.html` | 0 / 2 |  |
| `html/webappapis/dynamic-markup-insertion/html-unsafe-methods/setHTMLUnsafe.html` | 1 / 5 |  |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/015.html` | 1 / 3 |  |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/016.html` | 0 / 4 |  |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/abort-refresh-immediate.window.js` | 2 / 6 | harnais : délai du harnais dépassé |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/abort-refresh-multisecond-header.window.js` | 2 / 3 | harnais : délai du harnais dépassé |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/abort-refresh-multisecond-meta.window.js` | 2 / 3 | harnais : délai du harnais dépassé |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/abort-while-navigating.window.js` | 4 / 9 | harnais : délai du harnais dépassé |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/abort.sub.window.js` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/aborted-parser.window.js` | 0 / 2 |  |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/active.window.js` | 0 / 7 |  |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/bailout-exception-vs-return-origin.sub.window.js` | 0 / 6 | harnais : délai du harnais dépassé |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/bailout-exception-vs-return-xml.window.js` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/bailout-side-effects-ignore-opens-during-unload.window.js` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/bailout-side-effects-same-origin-domain.sub.window.js` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/bailout-side-effects-synchronous-script.window.js` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/bailout-side-effects-xml.window.js` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/crbug-583445-regression.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/custom-element.window.js` | 1 / 2 |  |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/document-open-cancels-javascript-url-navigation.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/document.open-01.xhtml` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/document.open-02.html` | 0 / 2 |  |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/encoding.window.js` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/event-listeners.window.js` | 6 / 18 |  |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/form-control-state.html` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/history.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/ignore-opens-during-unload.window.js` | 0 / 12 | harnais : délai du harnais dépassé |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/location-set-and-document-open.html` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/mutation-observer.window.js` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/open-with-defer-script-during-initial-about-blank-load.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/origin-check-in-document-open-same-origin-domain.sub.html` | 0 / 2 |  |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/quirks.window.js` | 0 / 4 |  |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/readiness.window.js` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/reload.window.js` | 0 / 1 | harnais : Test named 'Reloading a document.open()'d page should reload the URL of the entry realm's responsible document' specifie |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/remove-initial-about-blankness.window.js` | 1 / 2 |  |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/tasks.window.js` | 8 / 10 |  |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/type-argument-plaintext.window.js` | 0 / 8 |  |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/type-argument.window.js` | 0 / 8 |  |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/url-entry-document-sync-call.window.js` | 0 / 3 |  |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/url-entry-document.window.js` | 0 / 1 |  |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/url-fragment.window.js` | 1 / 2 |  |
| `html/webappapis/dynamic-markup-insertion/opening-the-input-stream/url.window.js` | 2 / 5 |  |
| `html/webappapis/dynamic-markup-insertion/the-innerhtml-property/annotation-xml-fragment-parsing.html` | 1 / 2 |  |
| `html/webappapis/dynamic-markup-insertion/the-innerhtml-property/innerhtml-and-xml-namespaces.svg` | 2 / 14 |  |
| `html/webappapis/events/event-handler-object-environment-instantiation.html` | 0 / 1 | harnais : Uncaught ReferenceError: onclickCalled n'est pas défini |
| `html/webappapis/microtask-queuing/queue-microtask-cross-realm-callback-report-exception.html` | 0 / 1 |  |
| `html/webappapis/scripting/event-loops/microtask_before_prepare_the_script_element-01.html` | 0 / 1 |  |
| `html/webappapis/scripting/event-loops/microtask_before_prepare_the_script_element-02.xhtml` | 0 / 1 |  |
| `html/webappapis/scripting/event-loops/new-scroll-event-dispatched-at-next-updating-rendering-time.html` | 0 / 1 |  |
| `html/webappapis/scripting/event-loops/raf-from-animation-event-serviced-in-same-rendering-update.html` | 0 / 1 |  |
| `html/webappapis/scripting/events/compile-event-handler-lexical-scopes-form-owner.html` | 1 / 4 |  |
| `html/webappapis/scripting/events/compile-event-handler-lexical-scopes.html` | 0 / 3 |  |
| `html/webappapis/scripting/events/compile-event-handler-settings-objects.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/webappapis/scripting/events/event-handler-all-global-events.html` | 0 / 0 | harnais : Unhandled rejection: WebIDL2 n'est pas défini |
| `html/webappapis/scripting/events/event-handler-attributes-body-alt.html` | 0 / 0 | harnais : Unhandled rejection: WebIDL2 n'est pas défini |
| `html/webappapis/scripting/events/event-handler-attributes-body-window.html` | 0 / 0 | harnais : Unhandled rejection: WebIDL2 n'est pas défini |
| `html/webappapis/scripting/events/event-handler-attributes-frameset-alt.html` | 0 / 0 | harnais : Unhandled rejection: WebIDL2 n'est pas défini |
| `html/webappapis/scripting/events/event-handler-attributes-frameset-window.html` | 0 / 0 | harnais : Unhandled rejection: WebIDL2 n'est pas défini |
| `html/webappapis/scripting/events/event-handler-attributes-no-bc.html` | 0 / 3 |  |
| `html/webappapis/scripting/events/event-handler-attributes-window-frameset.html` | 0 / 0 | harnais : Unhandled rejection: WebIDL2 n'est pas défini |
| `html/webappapis/scripting/events/event-handler-attributes-window.html` | 0 / 0 | harnais : Unhandled rejection: WebIDL2 n'est pas défini |
| `html/webappapis/scripting/events/event-handler-attributes-windowless-body.html` | 0 / 0 | harnais : Unhandled rejection: WebIDL2 n'est pas défini |
| `html/webappapis/scripting/events/event-handler-handleEvent-ignored.html` | 0 / 2 | harnais : Uncaught TypeError: la valeur appelée n'est pas une fonction |
| `html/webappapis/scripting/events/event-handler-idl-attribute-realm.window.js` | 0 / 2 |  |
| `html/webappapis/scripting/events/event-handler-no-browsing-context.html` | 5 / 12 |  |
| `html/webappapis/scripting/events/event-handler-non-content-document-idl-attributes.html` | 0 / 6 |  |
| `html/webappapis/scripting/events/event-handler-onmove-02.tentative.html` | 0 / 1 |  |
| `html/webappapis/scripting/events/event-handler-processing-algorithm-error/frameset-element-synthetic-errorevent.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/webappapis/scripting/events/event-handler-processing-algorithm-error/frameset-element-synthetic-event.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/webappapis/scripting/events/event-handler-removal.window.js` | 1 / 5 |  |
| `html/webappapis/scripting/events/event-handler-sourcetext.html` | 0 / 5 |  |
| `html/webappapis/scripting/events/event-handler-spec-example.window.js` | 0 / 9 |  |
| `html/webappapis/scripting/events/event-listener-sandboxed-scripts.html` | 3 / 4 |  |
| `html/webappapis/scripting/events/eventhandler-cancellation.html` | 14 / 15 |  |
| `html/webappapis/scripting/events/inline-event-handler-ordering.html` | 0 / 3 |  |
| `html/webappapis/scripting/events/invalid-uncompiled-raw-handler-compiled-late.window.js` | 0 / 1 |  |
| `html/webappapis/scripting/events/invalid-uncompiled-raw-handler-compiled-once.window.js` | 0 / 1 |  |
| `html/webappapis/scripting/events/invalid-uncompiled-raw-handler-keeps-position.window.js` | 0 / 1 |  |
| `html/webappapis/scripting/events/messageevent-constructor.https.html` | 3 / 9 |  |
| `html/webappapis/scripting/events/onerroreventhandler.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/webappapis/scripting/events/uncompiled_event_handler_with_scripting_disabled.html` | 0 / 1 |  |
| `html/webappapis/scripting/processing-model-2/addEventListener.html` | 0 / 1 |  |
| `html/webappapis/scripting/processing-model-2/body-onerror-compile-error-data-url.html` | 1 / 2 |  |
| `html/webappapis/scripting/processing-model-2/body-onerror-compile-error.html` | 1 / 2 |  |
| `html/webappapis/scripting/processing-model-2/body-onerror-runtime-error.html` | 1 / 2 |  |
| `html/webappapis/scripting/processing-model-2/compile-error-cross-origin-setInterval.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/webappapis/scripting/processing-model-2/compile-error-cross-origin-setTimeout.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/webappapis/scripting/processing-model-2/compile-error-cross-origin.html` | 0 / 2 |  |
| `html/webappapis/scripting/processing-model-2/compile-error-data-url.html` | 0 / 2 |  |
| `html/webappapis/scripting/processing-model-2/compile-error-in-setInterval.html` | 0 / 2 |  |
| `html/webappapis/scripting/processing-model-2/compile-error-in-setTimeout.html` | 0 / 2 |  |
| `html/webappapis/scripting/processing-model-2/compile-error-same-origin-with-hash.html` | 0 / 2 |  |
| `html/webappapis/scripting/processing-model-2/compile-error-same-origin.html` | 0 / 2 |  |
| `html/webappapis/scripting/processing-model-2/compile-error.html` | 0 / 2 |  |
| `html/webappapis/scripting/processing-model-2/integration-with-the-javascript-agent-formalism/atomics-wait-async.https.any.js` | 0 / 2 | harnais : délai du harnais dépassé |
| `html/webappapis/scripting/processing-model-2/integration-with-the-javascript-agent-formalism/requires-failure.https.any.js` | 0 / 1 |  |
| `html/webappapis/scripting/processing-model-2/integration-with-the-javascript-job-queue/promise-job-entry-different-function-realm.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/webappapis/scripting/processing-model-2/integration-with-the-javascript-job-queue/promise-job-entry.html` | 0 / 6 | harnais : délai du harnais dépassé |
| `html/webappapis/scripting/processing-model-2/integration-with-the-javascript-job-queue/promise-job-incumbent.html` | 0 / 11 | harnais : délai du harnais dépassé |
| `html/webappapis/scripting/processing-model-2/runtime-error-cross-origin-setInterval.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/webappapis/scripting/processing-model-2/runtime-error-cross-origin-setTimeout.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/webappapis/scripting/processing-model-2/runtime-error-cross-origin.html` | 0 / 2 |  |
| `html/webappapis/scripting/processing-model-2/runtime-error-data-url.html` | 0 / 2 |  |
| `html/webappapis/scripting/processing-model-2/runtime-error-in-body-onerror.html` | 0 / 1 |  |
| `html/webappapis/scripting/processing-model-2/runtime-error-in-setInterval.html` | 0 / 2 |  |
| `html/webappapis/scripting/processing-model-2/runtime-error-in-setTimeout.html` | 0 / 2 |  |
| `html/webappapis/scripting/processing-model-2/runtime-error-in-window-onerror.html` | 0 / 1 |  |
| `html/webappapis/scripting/processing-model-2/runtime-error-same-origin-with-hash.html` | 0 / 2 |  |
| `html/webappapis/scripting/processing-model-2/runtime-error-same-origin.html` | 0 / 2 |  |
| `html/webappapis/scripting/processing-model-2/runtime-error.html` | 0 / 2 |  |
| `html/webappapis/scripting/processing-model-2/unhandled-promise-rejections/allow-crossorigin.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/webappapis/scripting/processing-model-2/unhandled-promise-rejections/disallow-crossorigin.html` | 0 / 3 | harnais : délai du harnais dépassé |
| `html/webappapis/scripting/processing-model-2/unhandled-promise-rejections/promise-rejection-event-constructor.html` | 0 / 1 |  |
| `html/webappapis/scripting/processing-model-2/unhandled-promise-rejections/promise-rejection-event-during-parse.html` | 0 / 1 |  |
| `html/webappapis/scripting/processing-model-2/unhandled-promise-rejections/promise-rejection-events-iframe.html` | 1 / 3 | harnais : délai du harnais dépassé |
| `html/webappapis/scripting/processing-model-2/unhandled-promise-rejections/promise-rejection-events-many-promises.dedicatedworker.html` | 4 / 6 | harnais : délai du harnais dépassé |
| `html/webappapis/scripting/processing-model-2/unhandled-promise-rejections/promise-rejection-events-many-promises.html` | 4 / 6 | harnais : délai du harnais dépassé |
| `html/webappapis/scripting/processing-model-2/unhandled-promise-rejections/promise-rejection-events-onerror.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/webappapis/scripting/processing-model-2/unhandled-promise-rejections/promise-rejection-events.html` | 33 / 43 | harnais : délai du harnais dépassé |
| `html/webappapis/scripting/processing-model-2/unhandled-promise-rejections/promise-rejection-events.serviceworker.https.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/webappapis/scripting/processing-model-2/unhandled-promise-rejections/promise-rejection-events.sharedworker.html` | 0 / 0 | harnais : délai du harnais dépassé |
| `html/webappapis/scripting/processing-model-2/unhandled-promise-rejections/promise-resolution-order.html` | 0 / 1 |  |
| `html/webappapis/scripting/processing-model-2/window-onerror-parse-error.html` | 0 / 1 |  |
| `html/webappapis/scripting/processing-model-2/window-onerror-runtime-error-throw.html` | 0 / 1 |  |
| `html/webappapis/scripting/processing-model-2/window-onerror-runtime-error.html` | 0 / 1 |  |
| `html/webappapis/scripting/processing-model-2/window-onerror-with-cross-frame-event-listeners-1.html` | 0 / 1 |  |
| `html/webappapis/scripting/processing-model-2/window-onerror-with-cross-frame-event-listeners-2.html` | 0 / 1 |  |
| `html/webappapis/scripting/processing-model-2/window-onerror-with-cross-frame-event-listeners-5.html` | 0 / 1 |  |
| `html/webappapis/scripting/reporterror-cross-realm-method.html` | 0 / 1 |  |
| `html/webappapis/scripting/reporterror.any.js` | 2 / 5 |  |
| `html/webappapis/structured-clone/structured-clone-cross-realm-method.html` | 0 / 4 |  |
| `html/webappapis/structured-clone/structured-clone.any.js` | 94 / 152 |  |
| `html/webappapis/system-state-and-capabilities/the-navigator-object/historical.https.window.js` | 4 / 5 |  |
| `html/webappapis/system-state-and-capabilities/the-navigator-object/navigator-window-controls-overlay.tentative.html` | 0 / 7 |  |
| `html/webappapis/system-state-and-capabilities/the-navigator-object/navigator.any.js` | 8 / 12 |  |
| `html/webappapis/system-state-and-capabilities/the-navigator-object/navigator_user_agent.https.tentative.html` | 0 / 2 |  |
| `html/webappapis/system-state-and-capabilities/the-navigator-object/protocol-handler-fragment-nosw.https.html` | 0 / 0 | harnais : Error: unimplemented |
| `html/webappapis/system-state-and-capabilities/the-navigator-object/protocol-handler-fragment.https.html` | 0 / 0 | harnais : Error: unimplemented |
| `html/webappapis/system-state-and-capabilities/the-navigator-object/protocol-handler-path.https.html` | 0 / 0 | harnais : Error: unimplemented |
| `html/webappapis/system-state-and-capabilities/the-navigator-object/protocol-handler-query-nosw.https.html` | 0 / 0 | harnais : Error: unimplemented |
| `html/webappapis/system-state-and-capabilities/the-navigator-object/protocol-handler-query.https.html` | 0 / 0 | harnais : Error: unimplemented |
| `html/webappapis/system-state-and-capabilities/the-navigator-object/protocol.https.html` | 0 / 244 |  |
| `html/webappapis/system-state-and-capabilities/the-navigator-object/protocol.tentative.https.html` | 0 / 18 |  |
| `html/webappapis/the-windoworworkerglobalscope-mixin/Worker_Self_Origin.html` | 2 / 6 | harnais : délai du harnais dépassé |
| `html/webappapis/timers/errors.html` | 0 / 4 | harnais : délai du harnais dépassé |
| `html/webappapis/timers/evil-spec-example.any.js` | 0 / 2 | harnais : Uncaught TypeError: la valeur appelée n'est pas une fonction |
| `html/webappapis/timers/setinterval-cross-realm-callback-report-exception.html` | 0 / 1 |  |
| `html/webappapis/timers/settimeout-cross-realm-callback-report-exception.html` | 0 / 1 |  |
| `html/webappapis/timers/settimeout-detached-iframe.html` | 0 / 1 |  |
| `html/webappapis/timers/type-long-setinterval.any.js` | 0 / 1 |  |
| `html/webappapis/timers/type-long-settimeout.any.js` | 0 / 1 |  |
| `html/webappapis/update-rendering/child-document-raf-order.html` | 0 / 1 |  |
| `html/webappapis/user-prompts/print-during-beforeunload.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `html/webappapis/user-prompts/print-during-unload.html` | 0 / 1 | harnais : délai du harnais dépassé |
