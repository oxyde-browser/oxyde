//! Menu contextuel d'une page.
//!
//! Un panneau flottant ouvert au point du clic droit, ramené dans la fenêtre
//! s'il en déborde : une ligne par action, son raccourci à droite, un filet
//! entre les groupes. Il est dessiné par-dessus tout et ses zones cliquables
//! passent avant toutes les autres.

use oxyde_css::Color;

use crate::{ChromeState, Hit, Label, MenuView, Painter, Role, rect, theme};

/// Hauteur d'une ligne.
const ROW: f32 = 32.0;
/// Marge intérieure du panneau.
const PADDING: f32 = 6.0;
/// Hauteur prise par un filet de séparation.
const SEPARATOR: f32 = 9.0;
/// Largeur minimale du panneau.
const MIN_WIDTH: f32 = 220.0;
/// Écart gardé avec les bords de la fenêtre.
const MARGIN: f32 = 8.0;

/// Dessine le menu `view`.
pub(crate) fn build(p: &mut Painter<'_>, state: &ChromeState<'_>, view: &MenuView<'_>) {
    if view.items.is_empty() {
        return;
    }
    // Largeur : la ligne la plus longue, raccourci compris.
    let mut width = MIN_WIDTH;
    let mut height = 2.0 * PADDING;
    for item in view.items {
        let label = p.measure(&Label::new(item.label, 13.0, theme::TEXT));
        let shortcut = p.measure(&Label::new(item.shortcut, 12.0, theme::TEXT_MUTED));
        width = width.max(label + shortcut + 2.0 * PADDING + 44.0);
        height += ROW + if item.separator { SEPARATOR } else { 0.0 };
    }
    let width = width
        .ceil()
        .min((state.width - 2.0 * MARGIN).max(MIN_WIDTH));
    // Le panneau reste dans la fenêtre.
    let x = view.x.min(state.width - width - MARGIN).max(MARGIN).round();
    let y = view
        .y
        .min(state.height - height - MARGIN)
        .max(MARGIN)
        .round();
    let panel = rect(x, y, width, height);
    p.fill(panel, theme::SURFACE, 10.0);
    p.outline(panel, theme::BORDER, 10.0, 1.0);

    let mut top = y + PADDING;
    for (index, item) in view.items.iter().enumerate() {
        if item.separator {
            p.fill(
                rect(
                    x + PADDING + 4.0,
                    top + 4.0,
                    width - 2.0 * PADDING - 8.0,
                    1.0,
                ),
                theme::BORDER,
                0.0,
            );
            top += SEPARATOR;
        }
        let row = rect(x + PADDING, top, width - 2.0 * PADDING, ROW);
        let hit = Hit::MenuItem(index);
        if item.enabled && p.hover == Some(hit) {
            p.fill(row, theme::HOVER_STRONG, 6.0);
        }
        let color = if item.enabled {
            theme::TEXT
        } else {
            Color {
                a: 0x80,
                ..theme::TEXT_MUTED
            }
        };
        let shortcut = Label::new(item.shortcut, 12.0, theme::TEXT_MUTED);
        let shortcut_w = p.measure(&shortcut);
        if !item.shortcut.is_empty() {
            p.label(
                &shortcut,
                row.x + row.w - 10.0 - shortcut_w,
                top,
                ROW,
                shortcut_w + 1.0,
            );
        }
        p.label(
            &Label::new(item.label, 13.0, color),
            row.x + 10.0,
            top,
            ROW,
            (row.w - 30.0 - shortcut_w).max(0.0),
        );
        if item.enabled {
            p.target(row, hit, Role::Button, item.label);
        } else {
            p.describe(row, Role::Button, item.label);
            p.last_widget(|widget| widget.disabled = true);
        }
        top += ROW;
    }
    // Un clic dans le panneau hors d'une action n'atteint pas la page.
    p.hits.push((panel, Hit::Menu));
}
