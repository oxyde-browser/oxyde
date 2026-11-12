//! Recherche de texte dans une page mise en forme.
//!
//! La recherche se fait dans chaque contexte de mise en forme en ligne
//! (un paragraphe, une cellule) : une occurrence peut traverser des éléments
//! en ligne (`<b>`, `<a>`), pas des blocs. Elle ignore la casse.
//!
//! LIMITES CONNUES : les rectangles sont ceux de la mise en page, sans les
//! transformations ni le défilement des zones défilantes internes ; les
//! puces de liste et le texte de remplacement des images ne sont pas
//! cherchés.

use oxyde_text::parley::{Affinity, Cluster, Cursor, Selection};

use crate::{Fragment, LayoutTree, Rect, TextFragment};

/// Nombre maximal d'occurrences rendues : au-delà, la recherche s'arrête.
pub const MAX_MATCHES: usize = 10_000;

/// Texte en minuscules, avec pour chaque octet le début et la fin, dans le
/// texte d'origine, du caractère dont il provient.
struct Folded {
    text: String,
    starts: Vec<usize>,
    ends: Vec<usize>,
}

fn fold(text: &str) -> Folded {
    let mut folded = Folded {
        text: String::with_capacity(text.len()),
        starts: Vec::with_capacity(text.len()),
        ends: Vec::with_capacity(text.len()),
    };
    for (start, c) in text.char_indices() {
        let end = start + c.len_utf8();
        for lower in c.to_lowercase() {
            folded.text.push(lower);
            for _ in 0..lower.len_utf8() {
                folded.starts.push(start);
                folded.ends.push(end);
            }
        }
    }
    folded
}

/// Intervalles d'octets de `text` où apparaît `needle` (déjà en minuscules),
/// sans chevauchement.
fn occurrences(text: &str, needle: &str) -> Vec<(usize, usize)> {
    let folded = fold(text);
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(offset) = folded.text.get(from..).and_then(|rest| rest.find(needle)) {
        let at = from + offset;
        let last = at + needle.len() - 1;
        if let (Some(start), Some(end)) = (folded.starts.get(at), folded.ends.get(last)) {
            found.push((*start, *end));
        }
        from = at + needle.len();
    }
    found
}

fn search_text(fragment: &TextFragment, needle: &str, out: &mut Vec<Vec<Rect>>) {
    for (start, end) in occurrences(&fragment.text, needle) {
        if out.len() >= MAX_MATCHES {
            return;
        }
        // Corps du texte au début de l'occurrence.
        let size = Cluster::from_byte_index(&fragment.layout, start).map(|c| c.run().font_size());
        let selection = Selection::new(
            Cursor::from_byte_index(&fragment.layout, start, Affinity::Downstream),
            Cursor::from_byte_index(&fragment.layout, end, Affinity::Upstream),
        );
        #[expect(
            clippy::cast_possible_truncation,
            reason = "coordonnées de mise en page, loin des bornes de f32"
        )]
        let rects: Vec<Rect> = selection
            .geometry(&fragment.layout)
            .into_iter()
            .map(|(bounds, line)| {
                // La sélection couvre toute la hauteur de la ligne, qu'une
                // image peut rendre bien plus haute que le texte : le
                // rectangle est ramené autour de la ligne de base.
                let (mut top, mut bottom) = (bounds.y0 as f32, bounds.y1 as f32);
                if let (Some(size), Some(line)) = (size, fragment.layout.get(line)) {
                    let baseline = line.metrics().baseline;
                    top = top.max(baseline - size);
                    bottom = bottom.min(baseline + size * 0.3);
                }
                Rect {
                    x: fragment.x + bounds.x0 as f32,
                    y: fragment.y + top,
                    w: (bounds.x1 - bounds.x0) as f32,
                    h: bottom - top,
                }
            })
            .filter(|rect| rect.w > 0.0 && rect.h > 0.0)
            .collect();
        if !rects.is_empty() {
            out.push(rects);
        }
    }
}

fn search(fragment: &Fragment, needle: &str, out: &mut Vec<Vec<Rect>>) {
    // Parcours itératif : la profondeur d'un document n'est pas bornée.
    let mut stack = vec![fragment];
    while let Some(fragment) = stack.pop() {
        if out.len() >= MAX_MATCHES {
            return;
        }
        match fragment {
            Fragment::Box(b) => stack.extend(b.children.iter().rev()),
            Fragment::Text(t) => {
                search_text(t, needle, out);
                // Les boîtes en ligne-bloc portent leur propre texte.
                stack.extend(t.boxes.iter().rev());
            }
        }
    }
}

/// Occurrences de `needle` dans le texte de la page, dans l'ordre du
/// document : pour chacune, ses rectangles (un par ligne qu'elle occupe), en
/// coordonnées de page. La casse est ignorée ; une recherche vide ne trouve
/// rien ; au plus [`MAX_MATCHES`] occurrences sont rendues.
#[must_use]
pub fn find_text(tree: &LayoutTree, needle: &str) -> Vec<Vec<Rect>> {
    let needle = needle.to_lowercase();
    let mut out = Vec::new();
    if needle.is_empty() {
        return out;
    }
    if let Some(root) = &tree.root {
        search(root, &needle, &mut out);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case_folding_keeps_original_offsets() {
        assert_eq!(occurrences("Rust et RUST", "rust"), [(0, 4), (8, 12)]);
        // Caractères de plusieurs octets : les bornes restent celles du texte d'origine.
        assert_eq!(occurrences("Été, ÉTÉ", "été"), [(0, 5), (7, 12)]);
        // Sans chevauchement.
        assert_eq!(occurrences("aaaa", "aa"), [(0, 2), (2, 4)]);
        // Un caractère dont la minuscule est plus longue (İ → i̇).
        assert_eq!(occurrences("İstanbul", "stan"), [(2, 6)]);
        assert!(occurrences("abc", "abcd").is_empty());
    }

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

    #[test]
    fn matches_are_located_in_document_order() {
        let tree = layout(
            "<body style='margin:0'><p>Le <b>moteur</b> de rendu</p>\
             <ul><li>un MOTEUR</li></ul><p>rien ici</p></body>",
            400.0,
        );
        let found = find_text(&tree, "Moteur");
        assert_eq!(found.len(), 2);
        // Une occurrence qui tient sur une ligne a un seul rectangle, non vide.
        for rects in &found {
            assert_eq!(rects.len(), 1);
            assert!(rects[0].w > 10.0 && rects[0].h > 5.0);
        }
        // La première est au-dessus de la seconde, et à droite du début de ligne.
        assert!(found[0][0].y < found[1][0].y);
        assert!(found[0][0].x > 0.0);
        // Une occurrence peut traverser un élément en ligne, pas un bloc.
        assert_eq!(find_text(&tree, "le moteur de").len(), 1);
        assert!(find_text(&tree, "rendu un").is_empty());
        assert!(find_text(&tree, "").is_empty());
        assert!(find_text(&tree, "absent").is_empty());
    }

    #[test]
    fn a_wrapped_match_has_one_rectangle_per_line() {
        let tree = layout(
            "<body style='margin:0'><p style='width:60px;font-size:16px'>mot mot mot mot</p></body>",
            400.0,
        );
        let found = find_text(&tree, "mot mot mot mot");
        assert_eq!(found.len(), 1);
        assert!(found[0].len() >= 2, "{} rectangle(s)", found[0].len());
    }
}
