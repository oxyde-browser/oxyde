# Oxyde

Navigateur web écrit en Rust : une réécriture de l'architecture de Chromium (Blink, V8, cc, //net…)
avec du code Rust et des bibliothèques Rust uniquement. Cible actuelle : Windows.

Maquette de référence : « Oxyde — navigateur Rust » (Claude Design).

## Lancer

```bash
cargo run -p oxyde -- https://example.com
```

```bash
cargo run -p oxyde -- tests/pages/demo.html --split https://example.com
```

Rendu hors écran (capture de la fenêtre, ou de la page seule avec `--page-only`) :

```bash
cargo run -p oxyde -- --screenshot capture.png --size 1440x900 tests/pages/demo.html
```

Arbre DOM au format des tests html5lib :

```bash
cargo run -p oxyde -- --dump-dom tests/pages/demo.html
```

Raccourcis : Ctrl L (adresse), Ctrl T (nouvel onglet), Ctrl W (fermer), Ctrl R / F5 (actualiser),
Ctrl Tab (onglet suivant), Alt ← / Alt → (historique).

## Architecture

Un crate par bloc de Chromium. La chaîne de rendu d'une page :

```
octets ─ oxyde_net ─▶ oxyde_html ─▶ oxyde_dom ─▶ oxyde_css ─▶ oxyde_layout ─▶ oxyde_paint ─▶ oxyde_gpu
         (//net)      (analyseur)   (arbre)      (cascade)    (LayoutNG)      (liste          (Vello, wgpu,
                                                               + oxyde_text    d'affichage)    Direct3D 12)
                                                                 (Parley)
```

| Crate | Rôle | Équivalent Chromium | État |
|---|---|---|---|
| `oxyde` | exécutable : fenêtre, onglets, chargements | `chrome/` | fonctionne, mono-processus |
| `oxyde_ui` | interface dessinée d'après la maquette | Views | écran « Fenêtre principale » |
| `oxyde_content` | page : du HTML à la liste d'affichage | `content/renderer` | fonctionne |
| `oxyde_net` | HTTP/1.1, HTTP/2, TLS | `//net`, BoringSSL | GET, redirections ; voir limites |
| `oxyde_html` | tokenizer + construction d'arbre WHATWG | analyseur HTML de Blink | l'essentiel du standard |
| `oxyde_dom` | arbre de nœuds en arène | DOM de Blink | lecture seule (pas de script) |
| `oxyde_css` | syntaxe, sélecteurs, cascade | moteur de style de Blink | ~40 propriétés |
| `oxyde_layout` | blocs, flux en ligne, fragments | LayoutNG | bloc + en ligne ; voir limites |
| `oxyde_text` | polices, mise en forme, coupure de lignes | HarfBuzz, ICU | via Parley / HarfRust / ICU4X |
| `oxyde_paint` | liste d'affichage | paint de Blink | rectangles, bordures, texte |
| `oxyde_gpu` | rastérisation | Skia, Dawn | Vello sur wgpu |
| `oxyde_js` | moteur JavaScript | V8 | **réservé — jalon 4** |
| `oxyde_bindings` | liaisons Web IDL | bindings de Blink | **réservé — jalon 4** |
| `oxyde_ipc` | messages entre processus | Mojo | **réservé — jalon 5** |
| `oxyde_sandbox` | bac à sable Windows | `//sandbox/win` | **réservé — jalon 5** |
| `oxyde_compositor` | calques, défilement, animations | cc, Viz | **réservé — jalon 6** |
| `oxyde_platform` | abstraction du système | `//ui/base`, `//base` | **réservé — jalon 10** |

## Avertissements

- **Pas de bac à sable ni d'isolation des sites** : tout tourne dans un seul processus. Ne pas
  utiliser Oxyde pour naviguer sur des sites non fiables ni pour des données sensibles.
- **TLS** : le fournisseur cryptographique `rustls-rustcrypto` est une préversion non auditée,
  retenue pour rester en Rust pur. À réévaluer avant toute diffusion.
- **Pas de JavaScript** : les pages qui construisent leur contenu par script s'affichent vides
  ou partiellement.

Les écarts connus par rapport aux standards sont listés dans [conformance/README.md](conformance/README.md)
et en tête de chaque crate (« LIMITES CONNUES »).

## Règle « 100 % Rust »

Aucune dépendance C ou C++ compilée. Seules exceptions : les API du système appelées via
`windows-rs` / `windows-sys` (Direct3D 12, fenêtres, sockets). `unsafe` est interdit dans tous les
crates du workspace (`unsafe_code = "forbid"`).

## Vérifications

```bash
cargo fmt --all -- --check
```

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

```bash
cargo test --workspace
```

## Jalons

1. Socle — fait
2. Premier pixel — fait, sauf les images
3. Interface de la maquette — fenêtre principale faite ; nouvel onglet, pages d'erreur et
   accessibilité (AccessKit) à compléter
4. JavaScript (`oxyde_js`, Web IDL, DOM scriptable)
5. Multi-processus et bac à sable
6. Mise en page complète (flex, grille, tableaux, flottants, positionnement) et compositeur
7. Fonctions du navigateur (paramètres, historique, favoris, téléchargements, mots de passe…)
8. Performances JavaScript (compilateurs à la volée, ramasse-miettes générationnel, WebAssembly)
9. Plateforme web étendue (stockage, média, WebGL/WebGPU, WebRTC, DevTools, extensions, PDF)
10. Linux, macOS, Android, iOS/iPadOS
