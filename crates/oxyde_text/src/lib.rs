//! Texte d'Oxyde : polices du système, mise en forme (HarfRust), coupure de
//! lignes et bidi, fournis par Parley. Ce crate fixe le type de « pinceau »
//! attaché à chaque suite de glyphes et regroupe les contextes partagés.

pub use parley;

use parley::{FontContext, LayoutContext};

/// Données attachées à chaque suite de glyphes produite par la mise en page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct TextBrush {
    /// Couleur du texte, RGBA non prémultiplié.
    pub color: [u8; 4],
    /// Couleur de fond de la boîte en ligne (alpha nul : aucun fond).
    pub background: [u8; 4],
    /// Index (dans l'arène du document) du nœud qui a produit ce texte.
    pub node: u32,
}

/// Mise en page de texte terminée (lignes coupées et alignées).
pub type TextLayout = parley::Layout<TextBrush>;

/// Contextes partagés par toutes les mises en page de texte d'un processus de
/// rendu : base de polices et tampons réutilisables.
pub struct TextContext {
    /// Base de polices (polices du système + polices enregistrées).
    pub fonts: FontContext,
    /// Tampons de mise en page.
    pub layouts: LayoutContext<TextBrush>,
}

impl Default for TextContext {
    fn default() -> Self {
        Self::new()
    }
}

impl TextContext {
    /// Crée les contextes et charge l'index des polices du système.
    #[must_use]
    pub fn new() -> Self {
        Self {
            fonts: FontContext::new(),
            layouts: LayoutContext::new(),
        }
    }
}

impl std::fmt::Debug for TextContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextContext").finish_non_exhaustive()
    }
}

/// Construit la valeur CSS `font-family` correspondant à `families`, au format
/// attendu par Parley : noms entre guillemets, mots-clés génériques tels quels.
#[must_use]
pub fn css_font_family(families: &[String]) -> String {
    const GENERIC: &[&str] = &[
        "serif",
        "sans-serif",
        "monospace",
        "cursive",
        "fantasy",
        "system-ui",
        "ui-serif",
        "ui-sans-serif",
        "ui-monospace",
        "ui-rounded",
        "emoji",
        "math",
        "fangsong",
    ];
    let mut out = String::new();
    for family in families {
        if !out.is_empty() {
            out.push_str(", ");
        }
        let lower = family.to_ascii_lowercase();
        if GENERIC.contains(&lower.as_str()) {
            out.push_str(&lower);
        } else if lower == "-apple-system" || lower == "blinkmacsystemfont" {
            out.push_str("system-ui");
        } else {
            out.push('"');
            out.extend(family.chars().filter(|c| !matches!(c, '"' | '\\')));
            out.push('"');
        }
    }
    if out.is_empty() {
        out.push_str("serif");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_family_serialization() {
        let list = [
            "Inter".to_owned(),
            "Segoe \"UI".to_owned(),
            "Sans-Serif".to_owned(),
        ];
        assert_eq!(
            css_font_family(&list),
            "\"Inter\", \"Segoe UI\", sans-serif"
        );
        assert_eq!(css_font_family(&[]), "serif");
    }
}
