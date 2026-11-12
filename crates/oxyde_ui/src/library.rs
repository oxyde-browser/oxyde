//! Bibliothèque : écrans « Historique » et « Favoris » de la maquette.
//!
//! La bibliothèque occupe toute la fenêtre sous la barre de titre : une
//! colonne de rubriques à gauche, la liste à droite. Les onglets ne sont
//! pas affichés ; « Retour aux onglets » y ramène.
//!
//! LIMITES CONNUES : pas de dossiers de favoris, ni d'import ; les rubriques
//! « Téléchargements » et « Onglets d'autres appareils » de la maquette
//! n'existent pas encore.

use oxyde_paint::{DisplayItem, Rect};

use crate::{
    ChromeState, DISPLAY_FONT, Hit, ICON_BACK, ICON_CLOSE, Label, LibrarySection, LibraryView,
    Painter, Role, panes, rect, theme,
};

/// Loupe de 16 px.
const ICON_SEARCH: &str = "M11 7a4 4 0 1 1-8 0 4 4 0 0 1 8 0zM10 10l3.5 3.5";

/// Marge intérieure du panneau, en haut et en bas.
const PANEL_PADDING: f32 = 48.0;

/// Partie de `r` située dans `clip` ; `None` si elle est vide.
fn visible(r: Rect, clip: Rect) -> Option<Rect> {
    let top = r.y.max(clip.y);
    let bottom = (r.y + r.h).min(clip.y + clip.h);
    (bottom > top).then(|| rect(r.x, top, r.w, bottom - top))
}

/// Bouton à texte, de la hauteur de `r` ; `danger` : action destructrice à
/// confirmer.
fn text_button(p: &mut Painter<'_>, r: Rect, hit: Hit, text: &str, danger: bool) {
    let fill = if danger {
        theme::DANGER
    } else if p.hover == Some(hit) {
        theme::HOVER_STRONG
    } else {
        theme::ACTIVE
    };
    p.fill(r, fill, 10.0);
    p.label_centered(&Label::new(text, 14.0, theme::WHITE).weight(500.0), r);
    p.target(r, hit, Role::Button, text);
}

/// Colonne des rubriques.
fn navigation(p: &mut Painter<'_>, view: &LibraryView<'_>) {
    let x = 12.0;
    let w = theme::SIDEBAR - 8.0;
    let mut y = theme::TITLE_BAR + 4.0;
    let back = rect(x, y, w, 40.0);
    if p.hover == Some(Hit::LibraryBack) {
        p.fill(back, theme::HOVER, 8.0);
    }
    p.stroke_path(ICON_BACK, back.x + 10.0, back.y + 12.0, theme::ICON, 1.5);
    p.label(
        &Label::new("Retour aux onglets", 14.0, theme::ICON),
        back.x + 36.0,
        back.y,
        back.h,
        w - 46.0,
    );
    p.target(back, Hit::LibraryBack, Role::Button, "Retour aux onglets");
    y += 40.0 + 12.0 + 2.0;
    let sections = [
        (LibrarySection::History, "Historique"),
        (LibrarySection::Bookmarks, "Favoris"),
    ];
    for (section, name) in sections {
        let r = rect(x, y, w, 40.0);
        let hit = Hit::LibrarySection(section);
        let selected = view.section == section;
        if selected {
            p.fill(r, theme::ACTIVE, 8.0);
        } else if p.hover == Some(hit) {
            p.fill(r, theme::HOVER, 8.0);
        }
        let label = if selected {
            Label::new(name, 14.0, theme::WHITE).weight(500.0)
        } else {
            Label::new(name, 14.0, theme::TEXT_TAB)
        };
        p.label(&label, r.x + 12.0, r.y, r.h, w - 24.0);
        p.target(r, hit, Role::Tab, name);
        p.last_widget(|widget| widget.selected = selected);
        y += 42.0;
    }
}

/// Champ de recherche de l'historique.
fn search_field(p: &mut Painter<'_>, state: &ChromeState<'_>, field: Rect) {
    const PLACEHOLDER: &str = "Rechercher dans l'historique";
    p.fill(field, theme::SURFACE, 10.0);
    let border = if state.editing {
        theme::ACCENT
    } else {
        theme::BORDER
    };
    p.outline(field, border, 10.0, 1.0);
    p.stroke_path(
        ICON_SEARCH,
        field.x + 12.0,
        field.y + 14.0,
        theme::TEXT_MUTED,
        1.5,
    );
    let text_x = field.x + 12.0 + 16.0 + 10.0;
    let max_w = field.x + field.w - 12.0 - text_x;
    if state.editing {
        panes::edit_field(p, state, text_x, field.y, field.h, max_w, 14.0, PLACEHOLDER);
    } else {
        let (text, color) = if state.address.is_empty() {
            (PLACEHOLDER, theme::TEXT_MUTED)
        } else {
            (state.address, theme::TEXT)
        };
        p.label(
            &Label::new(text, 14.0, color),
            text_x,
            field.y,
            field.h,
            max_w,
        );
    }
    p.target(field, Hit::LibrarySearch, Role::TextInput, PLACEHOLDER);
    p.last_widget(|widget| widget.value = Some(state.address.to_owned()));
}

/// Dessine la bibliothèque sous la barre de titre ; renvoie le défilement
/// maximal de la liste.
pub(crate) fn build(p: &mut Painter<'_>, state: &ChromeState<'_>, view: &LibraryView<'_>) -> f32 {
    navigation(p, view);

    let history = view.section == LibrarySection::History;
    let panel_x = 8.0 + theme::SIDEBAR + 8.0;
    let panel = rect(
        panel_x,
        theme::TITLE_BAR,
        (state.width - panel_x - 8.0).max(0.0),
        (state.height - theme::TITLE_BAR - 8.0).max(0.0),
    );
    p.fill(panel, theme::PANEL, 12.0);
    p.outline(panel, theme::PANEL_BORDER, 12.0, 1.0);
    p.list.items.push(DisplayItem::PushClip {
        rect: panel,
        radius: 12.0,
    });

    let inner_w = (panel.w - 48.0)
        .min(if history { 760.0 } else { 880.0 })
        .max(0.0);
    let x = panel.x + (panel.w - inner_w) / 2.0;
    let mut y = panel.y + PANEL_PADDING - view.scroll;

    // Titre, et bouton d'effacement de l'historique.
    let title = if history { "Historique" } else { "Favoris" };
    let heading = Label::new(title, 32.0, theme::TEXT)
        .weight(600.0)
        .family(DISPLAY_FONT)
        .line(1.25);
    p.label(&heading, x, y, 44.0, inner_w);
    p.describe(rect(x, y, inner_w, 44.0), Role::Heading, title);
    if history && (!view.rows.is_empty() || !state.address.is_empty()) {
        let text = if view.confirm_clear {
            "Confirmer l'effacement"
        } else {
            "Effacer l'historique…"
        };
        let width = p.measure(&Label::new(text, 14.0, theme::WHITE).weight(500.0)) + 32.0;
        let button = rect(x + inner_w - width, y, width, 44.0);
        if visible(button, panel).is_some() {
            text_button(p, button, Hit::LibraryClear, text, view.confirm_clear);
        }
    }
    y += 44.0 + 24.0;

    if history {
        let field = rect(x, y, inner_w, 44.0);
        if visible(field, panel).is_some() {
            search_field(p, state, field);
        }
        y += 44.0 + 24.0;
    }

    let row_h = if history { 44.0 } else { 52.0 };
    for (index, row) in view.rows.iter().enumerate() {
        if let Some(heading) = row.heading {
            if index > 0 {
                y += 22.0;
            }
            if visible(rect(x, y, inner_w, 16.0), panel).is_some() {
                let text = heading.to_uppercase();
                p.label(
                    &Label::new(&text, 11.0, theme::TEXT_MUTED),
                    x,
                    y,
                    16.0,
                    inner_w,
                );
                p.describe(rect(x, y, inner_w, 16.0), Role::Heading, heading);
            }
            y += 16.0 + 6.0;
        }
        let r = rect(x, y, inner_w, row_h);
        y += row_h + 2.0;
        let Some(shown) = visible(r, panel) else {
            continue;
        };
        let open = Hit::LibraryRow(index);
        let remove = Hit::LibraryRemove(index);
        let hovered = p.hover == Some(open) || p.hover == Some(remove);
        if hovered {
            p.fill(r, theme::HOVER, 10.0);
        }
        // Bouton de retrait, au bout de la ligne survolée ; déclaré avant la
        // ligne pour que le clic lui revienne.
        let right = r.x + r.w - 12.0;
        let mut text_right = right;
        if hovered {
            let button = rect(right - 28.0, r.y + (row_h - 28.0) / 2.0, 28.0, 28.0);
            if p.hover == Some(remove) {
                p.fill(button, theme::HOVER_STRONG, 6.0);
            }
            p.stroke_path(ICON_CLOSE, button.x + 6.0, button.y + 6.0, theme::ICON, 1.5);
            let name = if history {
                "Retirer de l'historique"
            } else {
                "Retirer des favoris"
            };
            if let Some(shown) = visible(button, panel) {
                p.target(shown, remove, Role::Button, name);
            }
            text_right = button.x - 12.0;
        }
        if history {
            // Heure, pastille, titre, puis le site à droite.
            p.label(
                &Label::new(row.detail, 13.0, theme::TEXT_MUTED),
                r.x + 12.0,
                r.y,
                row_h,
                44.0,
            );
            let badge = rect(r.x + 12.0 + 44.0 + 12.0, r.y + 12.0, 20.0, 20.0);
            p.badge(badge, 5.0, row.host, row.title, 11.0);
            let host = Label::new(row.host, 13.0, theme::TEXT_MUTED);
            let host_w = p.measure(&host).min(inner_w / 3.0);
            p.label(&host, text_right - host_w, r.y, row_h, host_w);
            let title_x = badge.x + 20.0 + 12.0;
            let title_w = text_right - host_w - 12.0 - title_x;
            p.label(
                &Label::new(row.title, 14.0, theme::TEXT),
                title_x,
                r.y,
                row_h,
                title_w,
            );
        } else {
            // Pastille, puis le titre au-dessus de l'adresse.
            let badge = rect(r.x + 12.0, r.y + 14.0, 24.0, 24.0);
            p.badge(badge, 6.0, row.host, row.title, 12.0);
            let text_x = badge.x + 24.0 + 12.0;
            let text_w = text_right - text_x;
            p.label(
                &Label::new(row.title, 14.0, theme::TEXT),
                text_x,
                r.y + 6.0,
                20.0,
                text_w,
            );
            p.label(
                &Label::new(row.detail, 13.0, theme::TEXT_MUTED),
                text_x,
                r.y + 27.0,
                18.0,
                text_w,
            );
        }
        p.target(shown, open, Role::Button, row.title);
    }

    // Liste vide ou tronquée : le dire.
    let note = if !view.rows.is_empty() {
        view.truncated.then_some(
            "Seules les visites les plus récentes sont affichées : précisez la recherche.",
        )
    } else if !history {
        Some("Aucun favori pour l'instant. Ctrl D ajoute la page affichée aux favoris.")
    } else if state.address.is_empty() {
        Some("Aucune page visitée pour l'instant.")
    } else {
        Some("Aucune visite ne correspond à cette recherche.")
    };
    if let Some(note) = note {
        if !view.rows.is_empty() {
            y += 14.0;
        }
        let label = Label::new(note, 14.0, theme::TEXT_MUTED);
        let height = p.paragraph(&label, x, y, inner_w);
        p.describe(rect(x, y, inner_w, height), Role::Text, note);
        y += height;
    }
    p.list.items.push(DisplayItem::PopClip);

    let content = y + view.scroll - panel.y + PANEL_PADDING;
    (content - panel.h).max(0.0)
}
