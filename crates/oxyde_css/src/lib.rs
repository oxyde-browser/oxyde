//! Moteur de style d'Oxyde.
//!
//! - [`syntax`] : tokenizer CSS ;
//! - [`selector`] : sélecteurs, spécificité, correspondance ;
//! - [`values`] / [`properties`] / [`grid`] : valeurs et déclarations ;
//! - [`transform`] : fonctions de transformation et matrices affines ;
//! - [`animation`] : animations et transitions ;
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

pub mod animation;
pub mod grid;
pub mod properties;
pub mod selector;
pub mod style;
pub mod stylesheet;
pub mod syntax;
pub mod transform;
pub mod values;

pub use animation::{AnimationEvent, AnimationStyle, Animator, Easing, TransitionStyle};
pub use grid::{Breadth, GridAreas, GridLine, Repeat, TrackEntry, TrackList, TrackSize};
pub use properties::Side;
pub use style::{
    Alignment, BorderStyle, Clear, ComputedStyle, Display, Edges, FlexDirection, FlexWrap, Float,
    LineHeight, ListStyle, Overflow, Position, Size, SizeKeyword, StyleMap, TextAlign,
    VerticalAlign, Visibility, WhiteSpace, collect_style_elements, compute_styles,
    compute_styles_at,
};
pub use stylesheet::{FontFace, Stylesheet, Viewport, parse_stylesheet};
pub use transform::{Affine, TransformFunction};

/// Vrai si le moteur de style accepte la déclaration `property: value`
/// (`CSS.supports`).
#[must_use]
pub fn supports(property: &str, value: &str) -> bool {
    let source = format!("{property}:{value}");
    !properties::parse_declarations(&syntax::tokenize(&source)).is_empty()
}
pub use values::Color;
