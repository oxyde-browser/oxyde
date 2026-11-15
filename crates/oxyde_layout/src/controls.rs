//! Contenu des contrôles de formulaire : ce qu'un `<input>` ou un `<select>`
//! affiche dans sa boîte, qui n'a pas d'enfants dans le document.
//!
//! - champ de texte : sa valeur (ou son texte indicatif, estompé), et le
//!   curseur quand il a le focus ; un mot de passe est masqué ;
//! - bouton (`submit`, `reset`, `button`) : son libellé, centré ;
//! - case cochée et bouton radio coché : leur marque ;
//! - liste déroulante : l'option choisie et une flèche ; déroulée, la
//!   liste de ses options, peinte par-dessus la page.
//!
//! LIMITES CONNUES : une liste déroulée s'ouvre toujours vers le bas, à
//! l'emplacement de sa liste dans la page (ni transformation ni défilement
//! d'une zone interne pris en compte), et ne défile pas ; les types
//! `range`, `color`, `file` et `date` n'ont pas
//! d'apparence propre ; le contenu d'un `<textarea>` est mis en page comme
//! du texte ordinaire, avec son curseur mais sans surlignage de la
//! sélection ; une zone de texte plus longue que sa boîte ne défile pas
//! pour suivre le curseur.

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

/// Caractère affiché à la place de chaque caractère d'un mot de passe.
pub const PASSWORD_MASK: char = '•';

/// Surlignage du texte sélectionné dans un champ.
const FIELD_SELECTION: oxyde_css::Color = oxyde_css::Color {
    r: 0x33,
    g: 0x90,
    b: 0xFF,
    a: 0x59,
};

/// Position, en octets dans le texte affiché par le champ de texte `node`
/// (sa valeur, ou un [`PASSWORD_MASK`] par caractère), du caractère le plus
/// proche de l'abscisse `x` de la page. `None` si le champ n'est pas mis en
/// page.
#[must_use]
pub fn field_caret_at(tree: &crate::LayoutTree, node: NodeId, x: f32) -> Option<usize> {
    // Parcours itératif : la profondeur d'un document n'est pas bornée.
    let mut stack: Vec<&Fragment> = tree.root.iter().collect();
    while let Some(fragment) = stack.pop() {
        match fragment {
            Fragment::Box(b) if b.node == Some(node) => {
                let line = b.children.iter().find_map(|child| match child {
                    Fragment::Text(text) => Some(text),
                    Fragment::Box(_) => None,
                })?;
                let middle = line.layout.height() / 2.0;
                return Some(Cursor::from_point(&line.layout, x - line.x, middle).index());
            }
            Fragment::Box(b) => stack.extend(b.children.iter()),
            Fragment::Text(text) => stack.extend(text.boxes.iter()),
        }
    }
    None
}

/// Premier texte mis en page parmi `fragments` et leurs descendants, dans
/// l'ordre du document.
fn first_text(fragments: &[Fragment]) -> Option<&TextFragment> {
    // Parcours itératif, en profondeur d'abord.
    let mut stack: Vec<&Fragment> = fragments.iter().rev().collect();
    while let Some(fragment) = stack.pop() {
        match fragment {
            Fragment::Text(text) => return Some(text),
            Fragment::Box(b) => stack.extend(b.children.iter().rev()),
        }
    }
    None
}

/// Position, dans le contenu de la zone de texte `node`, du caractère le
/// plus proche du point (`x`, `y`) ; 0 pour une zone vide, `None` si la
/// zone n'est pas mise en page.
#[must_use]
pub fn area_caret_at(tree: &crate::LayoutTree, node: NodeId, x: f32, y: f32) -> Option<usize> {
    let mut stack: Vec<&Fragment> = tree.root.iter().collect();
    while let Some(fragment) = stack.pop() {
        match fragment {
            Fragment::Box(b) if b.node == Some(node) => {
                return Some(first_text(&b.children).map_or(0, |text| {
                    Cursor::from_point(&text.layout, x - text.x, y - text.y).index()
                }));
            }
            Fragment::Box(b) => stack.extend(b.children.iter()),
            Fragment::Text(text) => stack.extend(text.boxes.iter()),
        }
    }
    None
}

impl Engine<'_> {
    /// Liste déroulée du `<select>` `node`, dont la boîte de contenu est
    /// `content` : une ligne par option, celle qui est choisie en
    /// évidence. Chaque ligne porte le nœud de son option, pour qu'un clic
    /// la désigne.
    #[inline(never)]
    fn dropdown(&mut self, node: NodeId, style: &ComputedStyle, content: Rect) -> Fragment {
        let doc = self.doc;
        let chosen = selected_option(doc, node);
        let options: Vec<NodeId> = doc
            .descendants(node)
            .filter(|n| doc.element(*n).is_some_and(|e| e.is_html("option")))
            .collect();
        let row = style.line_height_px() + 4.0;
        let widest = self.control_widths.get(&node).copied().unwrap_or(24.0);
        let width = (widest + 12.0).max(content.w + 8.0);
        #[expect(
            clippy::cast_precision_loss,
            reason = "nombre d'options, loin des bornes de f32"
        )]
        let frame = Rect {
            x: content.x - 4.0,
            y: content.y + content.h + 4.0,
            w: width + 2.0,
            h: row * options.len() as f32 + 2.0,
        };
        let mut rows = Vec::with_capacity(options.len());
        let mut top = frame.y + 1.0;
        for option in options {
            let picked = chosen == Some(option);
            let disabled = doc.element(option).is_some_and(|e| e.has_attr("disabled"));
            let mut shown = style.clone();
            shown.background_color = if picked {
                ACCENT
            } else {
                oxyde_css::Color::rgb(0xFF, 0xFF, 0xFF)
            };
            shown.color = if picked {
                oxyde_css::Color::rgb(0xFF, 0xFF, 0xFF)
            } else if disabled {
                oxyde_css::Color::rgb(0x8A, 0x8A, 0x8A)
            } else {
                oxyde_css::Color::rgb(0x00, 0x00, 0x00)
            };
            let rect = Rect {
                x: frame.x + 1.0,
                y: top,
                w: width,
                h: row,
            };
            let label = option_text(doc, option);
            let layout = self.control_line(&label, &shown, option);
            let mut backdrop = ComputedStyle::initial();
            backdrop.background_color = shown.background_color;
            rows.push(Fragment::Box(BoxFragment {
                node: Some(option),
                style: Arc::new(backdrop),
                border_box: rect,
                children: vec![Self::control_text(layout, rect.x + 4.0, rect)],
            }));
            top += row;
        }
        // Cadre gris d'un pixel, peint au-dessus de tout le reste.
        let mut border = ComputedStyle::initial();
        border.background_color = oxyde_css::Color::rgb(0x76, 0x76, 0x76);
        border.position = oxyde_css::Position::Relative;
        border.z_index = Some(i32::MAX);
        Fragment::Box(BoxFragment {
            node: Some(node),
            style: Arc::new(border),
            border_box: frame,
            children: rows,
        })
    }

    /// Si `node` est une zone de texte, ajoute son curseur à `children`, son
    /// contenu déjà mis en page dans la boîte `border_box`.
    ///
    /// Jamais fondue dans l'appelant : `layout_box` est récursive, et ce
    /// code alourdirait chacun de ses niveaux de pile.
    #[inline(never)]
    pub(super) fn push_area_caret(
        &self,
        node: NodeId,
        style: &ComputedStyle,
        border_box: Rect,
        children: &mut Vec<Fragment>,
    ) {
        if !self
            .doc
            .element(node)
            .is_some_and(|e| e.is_html("textarea"))
        {
            return;
        }
        let pad = super::edges_px(style.padding, border_box.w);
        let bw = style.border_widths();
        let content = Rect {
            x: border_box.x + bw.left + pad.left,
            y: border_box.y + bw.top + pad.top,
            w: (border_box.w - bw.left - bw.right - pad.left - pad.right).max(0.0),
            h: (border_box.h - bw.top - bw.bottom - pad.top - pad.bottom).max(0.0),
        };
        let caret = self.area_caret(style, content, children);
        children.push(caret);
    }

    /// Curseur de la zone de texte qui a le focus : un trait à la position
    /// du curseur dans son texte `children` déjà mis en page, ou au début
    /// de sa boîte de contenu `content` si elle est vide.
    fn area_caret(&self, style: &ComputedStyle, content: Rect, children: &[Fragment]) -> Fragment {
        let rect = match first_text(children).filter(|text| !text.text.is_empty()) {
            Some(text) => {
                let mut caret = self.doc.caret().min(text.text.len());
                while !text.text.is_char_boundary(caret) {
                    caret -= 1;
                }
                let geometry = Cursor::from_byte_index(&text.layout, caret, Affinity::Downstream)
                    .geometry(&text.layout, 1.0);
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "coordonnées dans un texte, loin des bornes de f32"
                )]
                Rect {
                    x: text.x + geometry.x0 as f32,
                    y: text.y + geometry.y0 as f32,
                    w: 1.5,
                    h: (geometry.y1 - geometry.y0) as f32,
                }
            }
            None => Rect {
                x: content.x,
                y: content.y,
                w: 1.5,
                h: style.line_height_px(),
            },
        };
        Self::control_mark(rect, style.color, 0.0)
    }

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
            preserves_space: false,
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
        let mut anchor = self.doc.selection_anchor().min(value.len());
        while !value.is_char_boundary(anchor) {
            anchor -= 1;
        }
        let (text, caret, anchor, muted) = if value.is_empty() {
            let placeholder = element.attr("placeholder").unwrap_or("").to_owned();
            (placeholder, 0, 0, true)
        } else if password {
            // Un point par caractère : le curseur se compte en points.
            let points = |at: usize| {
                value.get(..at).map_or(0, |head| head.chars().count()) * PASSWORD_MASK.len_utf8()
            };
            let mask = String::from(PASSWORD_MASK).repeat(value.chars().count());
            (mask, points(caret), points(anchor), false)
        } else {
            (value.to_owned(), caret, anchor, false)
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
        let x_of = |index: usize| {
            if text.is_empty() {
                0.0
            } else {
                Cursor::from_byte_index(&layout, index, Affinity::Downstream)
                    .geometry(&layout, 1.0)
                    .x0 as f32
            }
        };
        let (caret_x, anchor_x) = (x_of(caret), x_of(anchor));
        // Un texte plus long que le champ défile pour garder le curseur visible.
        let scroll = if focused {
            (caret_x - (content.w - 2.0)).max(0.0)
        } else {
            0.0
        };
        let line = if height > 0.0 {
            height
        } else {
            style.line_height_px()
        };
        let mut fragments = Vec::new();
        if focused && (anchor_x - caret_x).abs() > f32::EPSILON {
            // Le texte sélectionné, surligné sous les lettres.
            fragments.push(Self::control_mark(
                Rect {
                    x: content.x + anchor_x.min(caret_x) - scroll,
                    y: content.y + (content.h - line) / 2.0,
                    w: (anchor_x - caret_x).abs(),
                    h: line,
                },
                FIELD_SELECTION,
                0.0,
            ));
        }
        fragments.push(Self::control_text(layout, content.x - scroll, content));
        if focused {
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
            if self.doc.has_state(node, oxyde_dom::STATE_SELECT_OPEN) {
                let list = self.dropdown(node, style, content);
                self.overlays.push(list);
            }
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
