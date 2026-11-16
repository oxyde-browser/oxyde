//! Taille de la pile du fil principal du lanceur de tests WPT : la même que
//! celle du navigateur (voir `crates/oxyde/build.rs`), pour que les pages
//! très imbriquées y soient mises en page dans les mêmes conditions.

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
