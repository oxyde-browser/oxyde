//! Taille de la pile du fil principal.
//!
//! La mise en page, la peinture et la recherche de la boîte sous un point
//! descendent dans l'arbre par récursion, jusqu'à 512 niveaux
//! (`oxyde_layout::MAX_DEPTH`), à quelques kilo-octets de pile par niveau.
//! Sous Windows, le fil principal n'a qu'un mébioctet par défaut : une page
//! de 300 éléments imbriqués le faisait déborder. Le binaire est donc lié
//! avec une pile de 16 Mio, valable pour tous ses processus (navigateur,
//! rendu, réseau), qui sont le même exécutable.
//!
//! Ailleurs, la pile du fil principal suit la limite du système (8 Mio en
//! général) : rien à demander.

fn main() {
    const STACK_BYTES: u32 = 16 * 1024 * 1024;
    println!("cargo:rerun-if-changed=build.rs");
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if os == "windows" {
        if env == "msvc" {
            println!("cargo:rustc-link-arg-bins=/STACK:{STACK_BYTES}");
        } else {
            println!("cargo:rustc-link-arg-bins=-Wl,--stack,{STACK_BYTES}");
        }
    }
}
