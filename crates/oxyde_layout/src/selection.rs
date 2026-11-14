//! Sélection de texte dans une page mise en forme.
//!
//! Une sélection va d'un point de la page à un autre : le caractère le plus
//! proche de chacun est retrouvé dans la mise en page, et tout le texte qui
//! les sépare, dans l'ordre du document, est sélectionné.
//!
//! LIMITES CONNUES : la sélection suit l'ordre du document, pas la position
//! à l'écran (un bloc positionné ailleurs que dans le flux est pris à son
//! rang dans le document) ; les rectangles sont ceux de la mise en page, sans
//! les transformations ni le défilement des zones défilantes internes ; les
//! puces de liste, le texte de remplacement des images et la valeur des
//! champs de formulaire ne se sélectionnent pas ; `user-select` est ignoré.

use oxyde_text::parley::Cursor;

use crate::find::range_rects;
use crate::{Fragment, LayoutTree, Rect, TextFragment};

/// Texte sélectionné et rectangles à surligner, en coordonnées de page.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextSelection {
    /// Un rectangle par ligne de chaque bloc de texte touché.
    pub rects: Vec<Rect>,
    /// Le texte, une ligne par bloc de texte.
    pub text: String,
}

/// Blocs de texte du document, dans l'ordre : ceux qui portent du texte.
fn text_fragments(tree: &LayoutTree) -> Vec<&TextFragment> {
    let mut found = Vec::new();
    // Parcours itératif : la profondeur d'un document n'est pas bornée.
    let mut stack: Vec<&Fragment> = tree.root.iter().collect();
    while let Some(fragment) = stack.pop() {
        match fragment {
            Fragment::Box(b) => stack.extend(b.children.iter().rev()),
            Fragment::Text(t) => {
                if !t.text.is_empty() {
                    found.push(&**t);
                }
                // Les boîtes en ligne-bloc portent leur propre texte.
                stack.extend(t.boxes.iter().rev());
            }
        }
    }
    found
}

/// Caractère le plus proche du point (`x`, `y`) : rang du bloc de texte, et
/// octet dans son texte.
fn locate(fragments: &[&TextFragment], x: f32, y: f32) -> Option<(usize, usize)> {
    // Distance du point à la boîte du bloc ; la verticale compte davantage,
    // pour qu'un point dans la marge reste sur sa ligne.
    let distance = |fragment: &TextFragment| {
        let (width, height) = (fragment.layout.width(), fragment.layout.height());
        let dx = (fragment.x - x).max(x - (fragment.x + width)).max(0.0);
        let dy = (fragment.y - y).max(y - (fragment.y + height)).max(0.0);
        dy * 4.0 + dx
    };
    let (index, fragment) = fragments
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| distance(a).total_cmp(&distance(b)))?;
    let cursor = Cursor::from_point(&fragment.layout, x - fragment.x, y - fragment.y);
    // L'indice reste dans le texte, au début d'un caractère.
    let mut byte = cursor.index().min(fragment.text.len());
    while !fragment.text.is_char_boundary(byte) {
        byte -= 1;
    }
    Some((index, byte))
}

/// Sélection entre les positions `start` et `end` (rang du bloc, octet),
/// `start` avant `end`.
fn between(
    fragments: &[&TextFragment],
    start: (usize, usize),
    end: (usize, usize),
) -> TextSelection {
    let mut selection = TextSelection::default();
    for (index, fragment) in fragments
        .iter()
        .enumerate()
        .skip(start.0)
        .take(end.0.saturating_sub(start.0) + 1)
    {
        let from = if index == start.0 { start.1 } else { 0 };
        let to = if index == end.0 {
            end.1
        } else {
            fragment.text.len()
        };
        let Some(piece) = fragment.text.get(from..to) else {
            continue;
        };
        // Le texte copié est celui qu'on lit : hors `white-space: pre`, les
        // blancs du source sont fusionnés et ceux des bords retirés.
        let piece = if fragment.preserves_space {
            piece.to_owned()
        } else {
            piece.split_whitespace().collect::<Vec<_>>().join(" ")
        };
        if piece.is_empty() {
            continue;
        }
        if !selection.text.is_empty() {
            selection.text.push('\n');
        }
        selection.text.push_str(&piece);
        selection.rects.extend(range_rects(fragment, from, to));
    }
    selection
}

/// Texte compris entre le caractère le plus proche de `anchor` et le plus
/// proche de `focus` (points en coordonnées de page), dans un sens ou dans
/// l'autre. Vide si la page n'a pas de texte ou si les deux points désignent
/// le même caractère.
#[must_use]
pub fn select_between(tree: &LayoutTree, anchor: (f32, f32), focus: (f32, f32)) -> TextSelection {
    let fragments = text_fragments(tree);
    let (Some(a), Some(b)) = (
        locate(&fragments, anchor.0, anchor.1),
        locate(&fragments, focus.0, focus.1),
    ) else {
        return TextSelection::default();
    };
    between(&fragments, a.min(b), a.max(b))
}

/// Tout le texte de la page.
#[must_use]
pub fn select_all(tree: &LayoutTree) -> TextSelection {
    let fragments = text_fragments(tree);
    let Some(last) = fragments.last() else {
        return TextSelection::default();
    };
    between(&fragments, (0, 0), (fragments.len() - 1, last.text.len()))
}

/// Le mot qui entoure le point (`x`, `y`) : la suite de lettres et de chiffres
/// qui touche le caractère le plus proche. Vide si ce caractère n'en est pas.
#[must_use]
pub fn select_word(tree: &LayoutTree, x: f32, y: f32) -> TextSelection {
    let fragments = text_fragments(tree);
    let Some((index, byte)) = locate(&fragments, x, y) else {
        return TextSelection::default();
    };
    let Some(fragment) = fragments.get(index) else {
        return TextSelection::default();
    };
    let text = fragment.text.as_str();
    let is_word = |c: char| c.is_alphanumeric() || c == '_' || c == '\'' || c == '’';
    let start = text
        .get(..byte)
        .unwrap_or("")
        .char_indices()
        .rev()
        .take_while(|(_, c)| is_word(*c))
        .last()
        .map_or(byte, |(at, _)| at);
    let end = text
        .get(byte..)
        .unwrap_or("")
        .char_indices()
        .find(|(_, c)| !is_word(*c))
        .map_or(text.len(), |(at, _)| byte + at);
    between(&fragments, (index, start), (index, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(html: &str, width: f32) -> LayoutTree {
        let doc = oxyde_html::parse_document(html);
        let viewport = oxyde_css::Viewport {
            width,
            height: 600.0,
        };
        let sheets = oxyde_css::collect_style_elements(&doc);
        let styles = oxyde_css::compute_styles(&doc, &sheets, viewport);
        let mut text = oxyde_text::TextContext::new();
        crate::layout_document(&doc, &styles, &mut text, viewport, &|_| None)
    }

    const PAGE: &str = "<body style='margin:0;font-size:16px'>\
        <p style='margin:0'>Premier <b>paragraphe</b> ici</p>\
        <p style='margin:0'>Second paragraphe</p>\
        <p style='margin:0'>Troisième</p></body>";

    #[test]
    fn a_selection_spans_inline_elements_and_blocks() {
        let tree = layout(PAGE, 400.0);
        // Toute la page : une ligne par paragraphe.
        let all = select_all(&tree);
        assert_eq!(
            all.text,
            "Premier paragraphe ici\nSecond paragraphe\nTroisième"
        );
        assert_eq!(all.rects.len(), 3);
        assert!(all.rects.iter().all(|r| r.w > 10.0 && r.h > 5.0));
        // Du début de la première ligne à la fin de la deuxième, dans les
        // deux sens : le même texte.
        let (top, second) = (all.rects[0], all.rects[1]);
        let from = (0.0, top.y + top.h / 2.0);
        let to = (399.0, second.y + second.h / 2.0);
        let forward = select_between(&tree, from, to);
        assert_eq!(forward.text, "Premier paragraphe ici\nSecond paragraphe");
        assert_eq!(select_between(&tree, to, from), forward);
        // Un point sous la page désigne la fin du texte.
        let below = select_between(&tree, from, (399.0, 5000.0));
        assert_eq!(below.text, all.text);
        // Deux fois le même point : rien.
        assert_eq!(select_between(&tree, from, from), TextSelection::default());
    }

    #[test]
    fn a_partial_selection_stops_at_the_nearest_characters() {
        let tree = layout(PAGE, 400.0);
        let line = select_all(&tree).rects[0];
        let y = line.y + line.h / 2.0;
        // Une moitié de la première ligne : un préfixe du texte, plus étroit.
        let half = select_between(&tree, (0.0, y), (line.w / 2.0, y));
        assert!(
            "Premier paragraphe ici".starts_with(&half.text) && !half.text.is_empty(),
            "{:?}",
            half.text
        );
        assert!(half.text.len() < "Premier paragraphe ici".len());
        assert_eq!(half.rects.len(), 1);
        assert!(half.rects[0].w < line.w && half.rects[0].w > 0.0);
    }

    #[test]
    fn a_word_is_found_around_a_point() {
        let tree = layout(PAGE, 400.0);
        let line = select_all(&tree).rects[0];
        let y = line.y + line.h / 2.0;
        assert_eq!(select_word(&tree, 4.0, y).text, "Premier");
        // Le dernier mot, pris par sa fin.
        assert_eq!(select_word(&tree, line.w - 4.0, y).text, "ici");
        assert_eq!(
            select_word(&layout("<body></body>", 100.0), 1.0, 1.0),
            TextSelection::default()
        );
    }

    #[test]
    fn copied_text_is_the_text_one_reads() {
        // Les blancs du source sont fusionnés ; une suite de blancs entre
        // deux éléments en ligne ne donne ni ligne ni rectangle.
        let tree = layout(
            "<body style='margin:0'><p style='margin:0'>Un   deux\n   trois</p>\n   <span> </span>\n\
             <pre style='margin:0'>a  b\n  c</pre></body>",
            400.0,
        );
        let all = select_all(&tree);
        assert_eq!(all.text, "Un deux trois\na  b\n  c");
    }

    #[test]
    fn a_page_without_text_selects_nothing() {
        let tree = layout("<body><div style='height:50px'></div></body>", 100.0);
        assert_eq!(select_all(&tree), TextSelection::default());
        assert_eq!(
            select_between(&tree, (0.0, 0.0), (50.0, 50.0)),
            TextSelection::default()
        );
    }
}
