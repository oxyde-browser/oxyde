//! Analyseur HTML d'Oxyde.
//!
//! Deux étages, comme dans Blink : le [`tokenizer`] (machine à états du
//! standard WHATWG) et le constructeur d'arbre ([`parse_document`]), qui
//! produit un [`oxyde_dom::Document`].
//!
//! ```
//! let doc = oxyde_html::parse_document("<!DOCTYPE html><p>Bonjour <b>Oxyde");
//! let body = doc.find_html_element(doc.root(), "body").expect("body implicite");
//! assert_eq!(doc.text_content(body), "Bonjour Oxyde");
//! ```

mod entities;
pub mod tokenizer;
mod tree_builder;

pub use tree_builder::parse_document;
