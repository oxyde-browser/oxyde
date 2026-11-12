//! Moteur de style d'Oxyde.
//!
//! - [`syntax`] : tokenizer CSS ;
//! - [`selector`] : sélecteurs, spécificité, correspondance ;
//! - [`values`] / [`properties`] : valeurs et déclarations ;
//! - [`stylesheet`] : règles et requêtes média ;
//! - [`style`] : cascade et [`style::ComputedStyle`].
//!
//! ```
//! use oxyde_css::{collect_style_elements, compute_styles, Viewport};
//!
//! let doc = oxyde_html::parse_document("<style>p { color: #4CC9A8 }</style><p>Oxyde");
//! let sheets = collect_style_elements(&doc);
//! let styles = compute_styles(&doc, &sheets, Viewport { width: 1280.0, height: 720.0 });
//! let p = doc.find_html_element(doc.root(), "p").expect("p");
//! assert_eq!(styles.get(p).expect("style").color.g, 0xC9);
//! ```

pub mod properties;
pub mod selector;
pub mod style;
pub mod stylesheet;
pub mod syntax;
pub mod values;

pub use properties::Side;
pub use style::{
    Alignment, BorderStyle, Clear, ComputedStyle, Display, Edges, FlexDirection, FlexWrap, Float,
    LineHeight, ListStyle, Overflow, Position, Size, StyleMap, TextAlign, Visibility, WhiteSpace,
    collect_style_elements, compute_styles,
};
pub use stylesheet::{Stylesheet, Viewport, parse_stylesheet};
pub use values::Color;
