//! Production du code Rust décrivant les interfaces.

use std::fmt::Write as _;

use crate::parse::{Interface, InterfaceKind, Member};

/// Littéral Rust d'une constante IDL.
fn float_literal(value: f64) -> String {
    if value.is_nan() {
        "f64::NAN".to_owned()
    } else if value.is_infinite() {
        if value > 0.0 {
            "f64::INFINITY".to_owned()
        } else {
            "f64::NEG_INFINITY".to_owned()
        }
    } else {
        // `{:?}` garde toujours un point décimal (« 1.0 ») : c'est un littéral f64.
        format!("{value:?}")
    }
}

/// Code Rust d'une table `INTERFACES` décrivant `interfaces` (voir
/// [`crate::InterfaceDescriptor`]). Les interfaces de rappel et les espaces
/// de noms, qui n'ont ni prototype ni constructeur, sont omis.
///
/// Le texte produit est déterministe : il ne dépend que de son entrée.
#[must_use]
pub fn generate(interfaces: &[Interface]) -> String {
    let mut out = String::new();
    out.push_str(
        "// @generated par oxyde_bindings à partir des fichiers Web IDL. Ne pas modifier à la main :\n\
         // relancer `cargo run -p oxyde_bindings -- <dossier IDL> <ce fichier>`.\n\n\
         use oxyde_bindings::{AttributeDescriptor, InterfaceDescriptor, OperationDescriptor};\n\n\
         /// Interfaces du DOM, mixins et définitions partielles fusionnés.\n\
         #[rustfmt::skip]\n\
         pub(crate) static INTERFACES: &[InterfaceDescriptor] = &[\n",
    );
    for interface in interfaces
        .iter()
        .filter(|i| i.kind == InterfaceKind::Interface)
    {
        let parent = interface
            .parent
            .as_ref()
            .map_or("None".to_owned(), |p| format!("Some({p:?})"));
        let constructor = interface
            .members
            .iter()
            .filter_map(|m| match m {
                Member::Constructor { length } => Some(*length),
                _ => None,
            })
            .min()
            .map_or("None".to_owned(), |length| format!("Some({length})"));
        let _ = writeln!(out, "    InterfaceDescriptor {{");
        let _ = writeln!(out, "        name: {:?},", interface.name);
        let _ = writeln!(out, "        parent: {parent},");
        let _ = writeln!(out, "        constructor: {constructor},");
        let _ = writeln!(out, "        constants: &[");
        for member in &interface.members {
            if let Member::Const { name, value } = member {
                let _ = writeln!(out, "            ({name:?}, {}),", float_literal(*value));
            }
        }
        let _ = writeln!(out, "        ],");
        let _ = writeln!(out, "        attributes: &[");
        for member in &interface.members {
            if let Member::Attribute {
                name,
                readonly,
                is_static,
                ..
            } = member
            {
                let _ = writeln!(
                    out,
                    "            AttributeDescriptor {{ name: {name:?}, readonly: {readonly}, is_static: {is_static} }},"
                );
            }
        }
        let _ = writeln!(out, "        ],");
        let _ = writeln!(out, "        operations: &[");
        for member in &interface.members {
            if let Member::Operation {
                name,
                length,
                is_static,
                ..
            } = member
            {
                let _ = writeln!(
                    out,
                    "            OperationDescriptor {{ name: {name:?}, length: {length}, is_static: {is_static} }},"
                );
            }
        }
        let _ = writeln!(out, "        ],");
        let _ = writeln!(out, "    }},");
    }
    out.push_str("];\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::{parse, resolve};

    #[test]
    fn generated_code_describes_the_interfaces() {
        let definitions = parse(
            "interface A { const short X = 2; const double Y = -Infinity; attribute long a;
               undefined f(long x, optional long y); static undefined g(); };
             interface B : A { constructor(DOMString s); constructor(); readonly attribute long b; };
             callback interface C { undefined handle(); };
             namespace N { undefined n(); };",
        )
        .expect("IDL valide");
        let interfaces = resolve(&definitions).expect("cohérent");
        let code = generate(&interfaces);
        assert!(code.starts_with("// @generated"));
        for expected in [
            "name: \"A\",\n        parent: None,\n        constructor: None,",
            "(\"X\", 2.0),",
            "(\"Y\", f64::NEG_INFINITY),",
            "AttributeDescriptor { name: \"a\", readonly: false, is_static: false },",
            "OperationDescriptor { name: \"f\", length: 1, is_static: false },",
            "OperationDescriptor { name: \"g\", length: 0, is_static: true },",
            "name: \"B\",\n        parent: Some(\"A\"),\n        constructor: Some(0),",
            "AttributeDescriptor { name: \"b\", readonly: true, is_static: false },",
        ] {
            assert!(code.contains(expected), "{expected}\n---\n{code}");
        }
        assert!(!code.contains("\"C\"") && !code.contains("\"N\""));
        assert_eq!(code, generate(&interfaces), "production déterministe");
    }
}
