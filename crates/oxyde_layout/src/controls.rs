//! Contenu des contrôles de formulaire : ce qu'un `<input>` ou un `<select>`
//! affiche dans sa boîte, qui n'a pas d'enfants dans le document.
//!
//! - champ de texte : sa valeur (ou son texte indicatif, estompé), et le
//!   curseur quand il a le focus ; un mot de passe est masqué ;
//! - bouton (`submit`, `reset`, `button`) : son libellé, centré ;
//! - case cochée et bouton radio coché : leur marque ;
//! - liste déroulante : l'option choisie et une flèche.
//!
//! LIMITES CONNUES : pas de sélection de texte ; une liste déroulante ne se
//! déroule pas ; les types `range`, `color`, `file` et `date` n'ont pas
//! d'apparence propre ; le contenu d'un `<textarea>` est mis en page comme
//! du texte ordinaire, sans curseur.

use oxyde_text::parley::{Affinity, Cursor};

use super::{
    Alignment, AlignmentOptions, Arc, BoxFragment, ComputedStyle, Engine, Fragment, NodeId, Rect,
    TextFragment, TextLayout, brush, css_font_family, text_style,
};

/// Couleur de la marque d'un bouton radio coché.
const ACCENT: oxyde_css::Color = oxyde_css::Color::rgb(0x00, 0x75, 0xFF);

/// Libellé d'un bouton `<input>` sans attribut `value`.
pub(super) fn default_label(kind: &str) -> &'static str {
    match kind {
        "submit" => "Envoyer",
        "reset" => "Réinitialiser",
        _ => "",
    }
}

/// Option affichée par une liste déroulante : la première qui porte
/// `selected`, sinon la première.
pub(super) fn selected_option(doc: &oxyde_dom::Document, select: NodeId) -> Option<NodeId> {
    let mut first = None;
    for node in doc.descendants(select) {
        let Some(element) = doc.element(node) else {
            continue;
        };
        if !element.is_html("option") {
            continue;
        }
        if element.has_attr("selected") {
            return Some(node);
        }
        first.get_or_insert(node);
    }
    first
}

/// Texte d'une option, espaces réduits.
pub(super) fn option_text(doc: &oxyde_dom::Document, option: NodeId) -> String {
    doc.text_content(option)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

impl Engine<'_> {
    /// Met en forme `text` sur une ligne, dans le style `style`.
    fn control_line(&mut self, text: &str, style: &ComputedStyle, node: NodeId) -> TextLayout {
        let family = css_font_family(&style.font_family);
        let root = text_style(style, &family, brush(style, node, false));
        let mut builder = self
            .text
            .layouts
            .tree_builder(&mut self.text.fonts, 1.0, true, &root);
        builder.push_text(text);
        let (mut layout, _) = builder.build();
        layout.break_all_lines(None);
        layout.align(Alignment::Start, AlignmentOptions::default());
        layout
    }

    /// Mesure, avant la mise en page, le libellé des boutons `<input>` et la
    /// plus longue option de chaque liste déroulante : leur boîte en prend
    /// la largeur.
    pub(super) fn measure_controls(&mut self) {
        let doc = self.doc;
        for node in doc.descendants(doc.root()) {
            let (Some(element), Some(style)) = (doc.element(node), self.styles.get(node)) else {
                continue;
            };
            let labels: Vec<String> = if element.is_html("select") {
                doc.descendants(node)
                    .filter(|n| doc.element(*n).is_some_and(|e| e.is_html("option")))
                    .map(|option| option_text(doc, option))
                    .collect()
            } else if element.is_html("input") {
                let kind = element.attr("type").map(str::to_ascii_lowercase);
                match kind.as_deref() {
                    Some(kind @ ("submit" | "reset" | "button")) => vec![
                        element
                            .attr("value")
                            .map_or_else(|| default_label(kind).to_owned(), str::to_owned),
                    ],
                    _ => continue,
                }
            } else {
                continue;
            };
            let style = Arc::clone(style);
            let mut widest = 0.0f32;
            for label in &labels {
                widest = widest.max(self.control_line(label, &style, node).width().ceil());
            }
            self.control_widths.insert(node, widest);
        }
    }

    /// Fragment d'une ligne de texte posée en (`x`, centrée dans `content`).
    fn control_text(layout: TextLayout, x: f32, content: Rect) -> Fragment {
        let y = content.y + (content.h - layout.height()) / 2.0;
        Fragment::Text(Box::new(TextFragment {
            x,
            y,
            layout,
            // Le texte d'un contrôle n'appartient pas au texte de la page.
            text: String::new(),
            boxes: Vec::new(),
        }))
    }

    /// Boîte pleine, sans nœud : curseur ou marque d'un contrôle.
    fn control_mark(rect: Rect, color: oxyde_css::Color, radius: f32) -> Fragment {
        let mut style = ComputedStyle::initial();
        style.background_color = color;
        style.border_radius = radius;
        Fragment::Box(BoxFragment {
            node: None,
            style: Arc::new(style),
            border_box: rect,
            children: Vec::new(),
        })
    }

    /// Champ de texte : valeur ou texte indicatif, et curseur s'il a le focus.
    fn text_field(
        &mut self,
        node: NodeId,
        style: &ComputedStyle,
        content: Rect,
        password: bool,
    ) -> Vec<Fragment> {
        let Some(element) = self.doc.element(node) else {
            return Vec::new();
        };
        let value = element.attr("value").unwrap_or("");
        let focused = self.doc.focus() == Some(node);
        // Position du curseur dans la valeur, ramenée sur un caractère.
        let mut caret = self.doc.caret().min(value.len());
        while !value.is_char_boundary(caret) {
            caret -= 1;
        }
        let (text, caret, muted) = if value.is_empty() {
            let placeholder = element.attr("placeholder").unwrap_or("").to_owned();
            (placeholder, 0, true)
        } else if password {
            // Un point par caractère : le curseur se compte en points.
            let before = value.get(..caret).map_or(0, |head| head.chars().count());
            let mask = "•".repeat(value.chars().count());
            (mask, before * '•'.len_utf8(), false)
        } else {
            (value.to_owned(), caret, false)
        };
        let mut shown = style.clone();
        if muted {
            shown.color.a = shown.color.a / 2 + shown.color.a / 16;
        }
        let layout = self.control_line(&text, &shown, node);
        let height = layout.height();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "abscisse dans une ligne de texte, loin des bornes de f32"
        )]
        let caret_x = if text.is_empty() {
            0.0
        } else {
            Cursor::from_byte_index(&layout, caret, Affinity::Downstream)
                .geometry(&layout, 1.0)
                .x0 as f32
        };
        // Un texte plus long que le champ défile pour garder le curseur visible.
        let scroll = if focused {
            (caret_x - (content.w - 2.0)).max(0.0)
        } else {
            0.0
        };
        let mut fragments = vec![Self::control_text(layout, content.x - scroll, content)];
        if focused {
            let line = if height > 0.0 {
                height
            } else {
                style.line_height_px()
            };
            fragments.push(Self::control_mark(
                Rect {
                    x: content.x + caret_x - scroll,
                    y: content.y + (content.h - line) / 2.0,
                    w: 1.5,
                    h: line,
                },
                style.color,
                0.0,
            ));
        }
        fragments
    }

    /// Fragments à placer dans la boîte de contenu `content` du contrôle
    /// `node` ; aucun si `node` n'en est pas un.
    ///
    /// Jamais fondue dans l'appelant : `layout_box` est récursive, et les
    /// styles copiés ici alourdiraient chacun de ses niveaux de pile.
    #[inline(never)]
    pub(super) fn control_content(
        &mut self,
        node: NodeId,
        style: &ComputedStyle,
        content: Rect,
    ) -> Vec<Fragment> {
        let Some(element) = self.doc.element(node) else {
            return Vec::new();
        };
        if element.is_html("select") {
            let label = selected_option(self.doc, node)
                .map(|option| option_text(self.doc, option))
                .unwrap_or_default();
            let text = self.control_line(&label, style, node);
            let mut fragments = vec![Self::control_text(text, content.x, content)];
            // Flèche : un petit triangle, dessiné ligne à ligne pour ne
            // dépendre d'aucune police.
            let size = (style.font_size * 0.3).round().max(3.0);
            let left = content.x + content.w - 2.0 * size;
            let top = content.y + (content.h - size) / 2.0;
            let mut row = 0.0;
            while row < size {
                fragments.push(Self::control_mark(
                    Rect {
                        x: left + row,
                        y: top + row,
                        w: 2.0 * (size - row),
                        h: 1.0,
                    },
                    style.color,
                    0.0,
                ));
                row += 1.0;
            }
            return fragments;
        }
        if !element.is_html("input") {
            return Vec::new();
        }
        let kind = element
            .attr("type")
            .map_or_else(|| "text".to_owned(), str::to_ascii_lowercase);
        match kind.as_str() {
            "hidden" | "image" | "file" | "range" | "color" => Vec::new(),
            "checkbox" => {
                if !element.has_attr("checked") {
                    return Vec::new();
                }
                let mut mark = style.clone();
                mark.color = oxyde_css::Color::rgb(0xFF, 0xFF, 0xFF);
                mark.font_size = content.h * 0.85;
                mark.font_weight = 700.0;
                let layout = self.control_line("✓", &mark, node);
                let x = content.x + (content.w - layout.width()) / 2.0;
                vec![Self::control_text(layout, x, content)]
            }
            "radio" => {
                if !element.has_attr("checked") {
                    return Vec::new();
                }
                let size = (content.w.min(content.h) - 6.0).max(2.0);
                vec![Self::control_mark(
                    Rect {
                        x: content.x + (content.w - size) / 2.0,
                        y: content.y + (content.h - size) / 2.0,
                        w: size,
                        h: size,
                    },
                    ACCENT,
                    size / 2.0,
                )]
            }
            "submit" | "reset" | "button" => {
                let label = element
                    .attr("value")
                    .map_or_else(|| default_label(&kind).to_owned(), str::to_owned);
                let layout = self.control_line(&label, style, node);
                let x = content.x + ((content.w - layout.width()) / 2.0).max(0.0);
                vec![Self::control_text(layout, x, content)]
            }
            _ => self.text_field(node, style, content, kind == "password"),
        }
    }
}
