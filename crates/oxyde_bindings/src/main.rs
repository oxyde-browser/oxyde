//! Outil en ligne de commande du générateur de liaisons.
//!
//! ```text
//! cargo run -p oxyde_bindings -- <dossier de fichiers .webidl> <fichier Rust produit>
//! ```

use std::path::Path;

use oxyde_bindings::{Definition, generate, parse, resolve};

/// Lit, dans l'ordre alphabétique, tous les fichiers `.webidl` de `directory`.
fn read_definitions(directory: &Path) -> Result<Vec<Definition>, Box<dyn std::error::Error>> {
    let mut paths: Vec<_> = std::fs::read_dir(directory)?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|e| e == "webidl"))
        .collect();
    paths.sort();
    let mut definitions = Vec::new();
    for path in paths {
        let source = std::fs::read_to_string(&path)?;
        let parsed = parse(&source).map_err(|error| format!("{} : {error}", path.display()))?;
        definitions.extend(parsed);
    }
    Ok(definitions)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let (Some(input), Some(output), None) = (args.next(), args.next(), args.next()) else {
        return Err("usage : oxyde_bindings <dossier IDL> <fichier Rust produit>".into());
    };
    let definitions = read_definitions(Path::new(&input))?;
    let interfaces = resolve(&definitions)?;
    std::fs::write(&output, generate(&interfaces))?;
    Ok(())
}
