//! Barre de recherche dans la page.
//!
//! Une barre flottante, en haut à droite du volet actif : le texte cherché,
//! le rang de l'occurrence courante, puis « précédente », « suivante » et
//! « fermer ». Elle est dessinée par-dessus la page et ses zones cliquables
//! passent avant celles du volet.

use oxyde_paint::Rect;

use crate::{ChromeState, FindView, Hit, ICON_CLOSE, Label, Painter, Role, panes, rect, theme};

/// Loupe de 16 px.
const ICON_SEARCH: &str = "M11 7a4 4 0 1 1-8 0 4 4 0 0 1 8 0zM10 10l3.5 3.5";
const ICON_UP: &str = "M4 10l4-4 4 4";
const ICON_DOWN: &str = "M4 6l4 4 4-4";

const PLACEHOLDER: &str = "Rechercher dans la page";

/// Dessine la barre de recherche dans le volet `pane`.
pub(crate) fn build(p: &mut Painter<'_>, state: &ChromeState<'_>, view: &FindView<'_>, pane: Rect) {
    let width = (pane.w - 32.0).clamp(0.0, 400.0);
    if width < 160.0 {
        return;
    }
    let bar = rect(pane.x + pane.w - 16.0 - width, pane.y + 12.0, width, 48.0);
    p.fill(bar, theme::SURFACE, 12.0);
    let border = if view.focused {
        p.accent
    } else {
        theme::BORDER
    };
    p.outline(bar, border, 12.0, 1.0);
    p.stroke_path(
        ICON_SEARCH,
        bar.x + 14.0,
        bar.y + 16.0,
        theme::TEXT_MUTED,
        1.5,
    );

    // Boutons, de droite à gauche.
    let buttons = [
        (Hit::FindClose, ICON_CLOSE, "Fermer la recherche"),
        (Hit::FindNext, ICON_DOWN, "Occurrence suivante"),
        (Hit::FindPrevious, ICON_UP, "Occurrence précédente"),
    ];
    let mut right = bar.x + bar.w - 8.0;
    let mut targets = Vec::with_capacity(buttons.len());
    for (hit, icon, name) in buttons {
        let button = rect(right - 32.0, bar.y + 8.0, 32.0, 32.0);
        if p.hover == Some(hit) {
            p.fill(button, theme::HOVER_STRONG, 8.0);
        }
        p.stroke_path(icon, button.x + 8.0, button.y + 8.0, theme::ICON, 1.5);
        targets.push((button, hit, name));
        right -= 34.0;
    }

    // Rang de l'occurrence courante.
    let query = if view.focused {
        state.address
    } else {
        view.query
    };
    let status = if query.is_empty() {
        String::new()
    } else if view.count == 0 {
        "Aucun résultat".to_owned()
    } else {
        format!("{} sur {}", view.current, view.count)
    };
    let status_label = Label::new(&status, 13.0, theme::TEXT_MUTED);
    let status_w = p.measure(&status_label);
    let status_x = right - 6.0 - status_w;
    if !status.is_empty() {
        p.label(&status_label, status_x, bar.y, bar.h, status_w + 1.0);
    }

    // Champ.
    let text_x = bar.x + 14.0 + 16.0 + 10.0;
    let text_w = (status_x - 10.0 - text_x).max(0.0);
    let field = rect(text_x, bar.y, text_w, bar.h);
    if view.focused {
        panes::edit_field(p, state, text_x, bar.y, bar.h, text_w, 14.0, PLACEHOLDER);
    } else {
        let (text, color) = if query.is_empty() {
            (PLACEHOLDER, theme::TEXT_MUTED)
        } else {
            (query, theme::TEXT)
        };
        p.label(&Label::new(text, 14.0, color), text_x, bar.y, bar.h, text_w);
    }
    p.target(field, Hit::FindInput, Role::TextInput, PLACEHOLDER);
    p.last_widget(|widget| widget.value = Some(query.to_owned()));
    if !status.is_empty() {
        p.describe(rect(status_x, bar.y, status_w, bar.h), Role::Text, &status);
    }
    // Dans l'ordre de lecture : précédente, suivante, fermer.
    for (button, hit, name) in targets.into_iter().rev() {
        p.target(button, hit, Role::Button, name);
    }
    // Un clic dans la barre hors d'un bouton lui donne la saisie, et
    // n'atteint pas la page.
    p.hits.push((bar, Hit::FindInput));
}
