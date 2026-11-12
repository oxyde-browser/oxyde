//! Texte d'Oxyde : polices du système, mise en forme (HarfRust), coupure de
//! lignes et bidi, fournis par Parley. Ce crate fixe le type de « pinceau »
//! attaché à chaque suite de glyphes et regroupe les contextes partagés.

pub use parley;

use std::sync::Arc;

use parley::fontique::{Blob, FontInfoOverride};
use parley::{FontContext, LayoutContext};

/// Polices de la maquette embarquées dans l'exécutable (licence OFL 1.1, voir
/// `assets/fonts/*/OFL.txt`). Polices variables : un fichier couvre toutes les graisses.
const BUNDLED_FONTS: [&[u8]; 3] = [
    include_bytes!("../../../assets/fonts/inter/Inter.ttf"),
    include_bytes!("../../../assets/fonts/space-grotesk/SpaceGrotesk.ttf"),
    include_bytes!("../../../assets/fonts/jetbrains-mono/JetBrainsMono.ttf"),
];

/// Enregistre les polices embarquées ; renvoie le nombre de familles ajoutées.
fn register_bundled_fonts(fonts: &mut FontContext) -> usize {
    BUNDLED_FONTS
        .iter()
        .map(|bytes| {
            fonts
                .collection
                .register_fonts(Blob::new(Arc::new(*bytes)), None)
                .len()
        })
        .sum()
}

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
    /// Polices de pages déjà enregistrées : (famille, clé).
    web_fonts: std::collections::HashSet<(String, String)>,
}

impl Default for TextContext {
    fn default() -> Self {
        Self::new()
    }
}

impl TextContext {
    /// Crée les contextes, charge l'index des polices du système et
    /// enregistre les polices embarquées (Inter, Space Grotesk, JetBrains Mono).
    ///
    /// ÉCART CONNU : ces polices sont aussi visibles des pages web, alors que
    /// Chromium réserve ses polices d'interface à l'interface.
    #[must_use]
    pub fn new() -> Self {
        let mut fonts = FontContext::new();
        register_bundled_fonts(&mut fonts);
        Self {
            fonts,
            layouts: LayoutContext::new(),
            web_fonts: std::collections::HashSet::new(),
        }
    }

    /// Enregistre une police téléchargée par une page (`@font-face`) sous le
    /// nom de famille `family`. `key` identifie le fichier (son adresse) : le
    /// même fichier n'est enregistré qu'une fois sous un même nom. Renvoie
    /// `false` si les données ne sont pas une police TrueType ou OpenType.
    ///
    /// LIMITES CONNUES : les formats compressés WOFF et WOFF2 ne sont pas
    /// lus ; la police reste enregistrée pour toutes les pages du processus.
    pub fn register_web_font(&mut self, family: &str, key: &str, data: &Arc<Vec<u8>>) -> bool {
        let sfnt = matches!(
            data.get(..4),
            Some(
                [0, 1, 0, 0]
                    | [b'O', b'T', b'T', b'O']
                    | [b't', b'r', b'u', b'e']
                    | [b't', b't', b'c', b'f']
            )
        );
        if !sfnt {
            return false;
        }
        let entry = (family.to_owned(), key.to_owned());
        if self.web_fonts.contains(&entry) {
            return true;
        }
        let info = FontInfoOverride {
            family_name: Some(family),
            width: None,
            style: None,
            weight: None,
            axes: None,
        };
        let added = self
            .fonts
            .collection
            .register_fonts(Blob::new(Arc::clone(data) as _), Some(info));
        if added.is_empty() {
            return false;
        }
        self.web_fonts.insert(entry);
        true
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
    fn bundled_fonts_register_and_are_selected() {
        let mut fonts = FontContext::new();
        assert_eq!(
            register_bundled_fonts(&mut fonts),
            3,
            "une famille par fichier"
        );

        // Une police embarquée demandée par son nom est bien celle utilisée.
        let mut cx = TextContext::new();
        for family in ["Inter", "Space Grotesk", "JetBrains Mono"] {
            let css = css_font_family(&[family.to_owned()]);
            let style = parley::TextStyle {
                font_family: parley::FontFamily::from(css.as_str()),
                font_size: 16.0,
                font_weight: parley::FontWeight::new(600.0),
                ..parley::TextStyle::<TextBrush>::default()
            };
            let mut builder = cx.layouts.tree_builder(&mut cx.fonts, 1.0, true, &style);
            builder.push_text("Oxyde");
            let (mut layout, _) = builder.build();
            layout.break_all_lines(None);
            let line = layout.lines().next().expect("une ligne");
            let run = line.runs().next().expect("une suite de glyphes");
            let used = run.font().data.as_ref();
            assert!(
                BUNDLED_FONTS.contains(&used),
                "{family} doit venir des polices embarquées"
            );
        }
    }

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
