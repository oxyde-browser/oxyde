# Web Platform Tests

Produit par `cargo run --release -p oxyde_wpt -- --publier url`, sur la suite
`web-platform-tests/wpt` déposée dans `vendor/wpt`. Seuls les tests `testharness.js`
destinés à une fenêtre sont exécutés ; un fichier est « entièrement réussi » quand le
harnais va au bout et que tous ses sous-tests passent.

**Sous-tests réussis : 6608 / 8717 (75,8 %)** — fichiers entièrement réussis : 16 / 32 (50,0 %).

Le taux en sous-tests surestime la conformité : un fichier qui plante ou que le harnais
abandonne ne rapporte aucun sous-test, donc n'en fait échouer aucun. Le taux en fichiers
est le plus honnête des deux.

| Dossier | Fichiers réussis | Sous-tests réussis |
|---|---:|---:|
| `url` | 16 / 32 | 6608 / 8717 (75,8 %) |

## Fichiers non entièrement réussis

| Fichier | Sous-tests | Cause |
|---|---:|---|
| `url/IdnaTestV2.any.js` | 1919 / 2676 |  |
| `url/a-element-origin.html` | 411 / 422 |  |
| `url/a-element.html` | 567 / 905 |  |
| `url/data-uri-fragment.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `url/failure.html` | 417 / 1177 |  |
| `url/idlharness.any.js` | 0 / 1 |  |
| `url/javascript-urls.window.js` | 0 / 6 | harnais : délai du harnais dépassé |
| `url/percent-encoding.window.js` | 1 / 17 |  |
| `url/toascii.window.js` | 721 / 784 |  |
| `url/url-constructor.any.js` | 841 / 906 |  |
| `url/url-origin.any.js` | 412 / 423 |  |
| `url/url-setters-a-area.window.js` | 519 / 557 |  |
| `url/url-setters.any.js` | 257 / 279 |  |
| `url/urlencoded-parser.any.js` | 88 / 105 |  |
| `url/urlsearchparams-constructor.any.js` | 31 / 32 |  |
| `url/urlsearchparams-delete.any.js` | 6 / 8 |  |
