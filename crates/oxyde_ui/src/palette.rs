//! Palette de commandes (écran `Palette` de la maquette).
//!
//! Une boîte centrée, posée par-dessus les pages : un champ de saisie, des
//! filtres, puis les résultats groupés par sorte. Le navigateur fournit les
//! lignes ([`PaletteView`]) ; ce module les dessine et déclare leurs zones
//! cliquables, qui passent avant tout le reste de la fenêtre.
//!
//! LIMITES CONNUES : la liste ne défile pas ; les lignes qui ne tiennent pas
//! dans la fenêtre ne sont pas affichées.

use oxyde_css::Color;
use oxyde_paint::DisplayItem;

use crate::{
    ChromeState, Hit, Label, Painter, PaletteGlyph, PaletteView, Role, panes, rect, theme,
};

/// Loupe de 16 px.
const ICON_SEARCH: &str = "M11 7a4 4 0 1 1-8 0 4 4 0 0 1 8 0zM10 10l3.5 3.5";
/// Invite de commande de 16 px.
const ICON_COMMAND: &str = "M3.5 4.5 7 8l-3.5 3.5M8.5 11.5h4";
/// Globe de 16 px.
const ICON_GLOBE: &str =
    "M8 2.5a5.5 5.5 0 1 0 0 11 5.5 5.5 0 0 0 0-11zM2.5 8h11M8 2.5c-2 2-2 9 0 11M8 2.5c2 2 2 9 0 11";

const PLACEHOLDER: &str = "Rechercher, ouvrir ou lancer une commande";

/// Voile posé sur la fenêtre derrière la palette.
const SCRIM: Color = Color {
    r: 0x0E,
    g: 0x11,
    b: 0x16,
    a: 0xA6,
};

/// Indication de touche encadrée, calée à droite en `right` ; renvoie sa largeur.
fn key_box(p: &mut Painter<'_>, text: &str, right: f32, y: f32, h: f32) -> f32 {
    let label = Label::new(text, 11.0, theme::TEXT_MUTED);
    let width = p.measure(&label) + 12.0;
    let frame = rect(right - width, y + (h - 18.0) / 2.0, width, 18.0);
    p.outline(frame, theme::BORDER_HINT, 5.0, 1.0);
    p.label(&label, frame.x + 6.0, frame.y, frame.h, width);
    width
}

/// Dessine la palette et déclare ses zones cliquables.
pub(crate) fn build(p: &mut Painter<'_>, state: &ChromeState<'_>, view: &PaletteView<'_>) {
    let (w, h) = (state.width, state.height);
    p.fill(rect(0.0, 0.0, w, h), SCRIM, 0.0);

    let width = (w - 32.0).clamp(0.0, 680.0);
    let x = (w - width) / 2.0;
    let top = (theme::TITLE_BAR + 72.0).min((h - 200.0).max(theme::TITLE_BAR + 8.0));
    let footer = 36.0;
    // Hauteur : saisie, filtres, lignes, pied ; bornée par la fenêtre.
    let mut wanted: f32 = 60.0 + 48.0 + 8.0;
    for (index, row) in view.rows.iter().enumerate() {
        if row.heading.is_some() {
            wanted += if index == 0 { 28.0 } else { 36.0 };
        }
        wanted += 46.0;
    }
    if view.rows.is_empty() {
        wanted += 44.0;
    }
    wanted += 6.0 + footer;
    let height = wanted.min((h - top - 16.0).max(0.0));
    let dialog = rect(x, top, width, height);
    p.fill(dialog, theme::SURFACE, 16.0);
    p.outline(dialog, theme::BORDER, 16.0, 1.0);
    p.list.items.push(DisplayItem::PushClip {
        rect: dialog,
        radius: 16.0,
    });

    // ---- Saisie ----
    let field = rect(x, top, width, 60.0);
    p.stroke_path(ICON_SEARCH, x + 16.0, top + 22.0, theme::TEXT_MUTED, 1.5);
    let hint_w = key_box(p, "Échap", x + width - 16.0, top, 60.0);
    let text_x = x + 16.0 + 16.0 + 12.0;
    let text_w = x + width - 16.0 - hint_w - 12.0 - text_x;
    panes::edit_field(p, state, text_x, top, 60.0, text_w, 16.0, PLACEHOLDER);
    p.describe(field, Role::TextInput, PLACEHOLDER);
    p.last_widget(|widget| widget.value = Some(state.address.to_owned()));
    p.fill(rect(x, top + 60.0, width, 1.0), theme::BORDER_SOFT, 0.0);

    // ---- Filtres ----
    let mut chip_x = x + 16.0;
    let chip_y = top + 60.0 + 12.0;
    for (index, name) in view.filters.iter().enumerate() {
        let chosen = index == view.filter;
        let label = if chosen {
            Label::new(name, 13.0, theme::ON_ACCENT).weight(600.0)
        } else {
            Label::new(name, 13.0, theme::TEXT_TAB)
        };
        let chip = rect(chip_x, chip_y, p.measure(&label) + 24.0, 32.0);
        let hit = Hit::PaletteFilter(index);
        let fill = if chosen {
            p.accent
        } else if p.hover == Some(hit) {
            theme::HOVER_STRONG
        } else {
            theme::CHIP
        };
        p.fill(chip, fill, 16.0);
        p.label_centered(&label, chip);
        p.target(chip, hit, Role::Button, name);
        p.last_widget(|widget| widget.selected = chosen);
        chip_x += chip.w + 6.0;
    }

    // ---- Résultats ----
    let list_bottom = top + height - footer - 6.0;
    let mut y = top + 60.0 + 48.0 + 8.0;
    for (index, row) in view.rows.iter().enumerate() {
        if let Some(heading) = row.heading {
            y += if index == 0 { 6.0 } else { 14.0 };
            if y + 16.0 <= list_bottom {
                let text = heading.to_uppercase();
                p.label(
                    &Label::new(&text, 11.0, theme::TEXT_MUTED),
                    x + 18.0,
                    y,
                    16.0,
                    width - 36.0,
                );
                p.describe(
                    rect(x + 18.0, y, width - 36.0, 16.0),
                    Role::Heading,
                    heading,
                );
            }
            y += 16.0 + 6.0;
        }
        let r = rect(x + 8.0, y, width - 16.0, 44.0);
        y += 46.0;
        if r.y + r.h > list_bottom {
            continue;
        }
        let hit = Hit::PaletteRow(index);
        let chosen = index == view.selected;
        if chosen {
            p.fill(r, theme::ACTIVE, 10.0);
            p.outline(r, p.accent, 10.0, 1.0);
        } else if p.hover == Some(hit) {
            p.fill(r, theme::HOVER, 10.0);
        }
        let glyph = rect(r.x + 12.0, r.y + 12.0, 20.0, 20.0);
        match row.glyph {
            PaletteGlyph::Badge => p.badge(glyph, 5.0, row.host, row.title, 11.0),
            PaletteGlyph::Command => {
                p.stroke_path(ICON_COMMAND, glyph.x + 2.0, glyph.y + 2.0, theme::ICON, 1.5)
            }
            PaletteGlyph::Search => {
                p.stroke_path(ICON_SEARCH, glyph.x + 2.0, glyph.y + 2.0, theme::ICON, 1.5)
            }
            PaletteGlyph::Page => {
                p.stroke_path(ICON_GLOBE, glyph.x + 2.0, glyph.y + 2.0, theme::ICON, 1.3)
            }
        }
        // À droite : un raccourci encadré, ou une indication en clair.
        let right = r.x + r.w - 12.0;
        let hint_w = if row.hint.is_empty() {
            0.0
        } else if row.shortcut {
            key_box(p, row.hint, right, r.y, r.h)
        } else {
            let hint = Label::new(row.hint, 12.0, theme::TEXT_MUTED);
            let hint_w = p.measure(&hint).min(r.w / 3.0);
            p.label(&hint, right - hint_w, r.y, r.h, hint_w);
            hint_w
        };
        let title = if chosen {
            Label::new(row.title, 14.0, theme::WHITE).weight(500.0)
        } else {
            Label::new(row.title, 14.0, theme::TEXT_TAB)
        };
        let title_x = glyph.x + 20.0 + 12.0;
        p.label(&title, title_x, r.y, r.h, right - hint_w - 12.0 - title_x);
        p.target(r, hit, Role::Button, row.title);
        p.last_widget(|widget| widget.selected = chosen);
    }
    if view.rows.is_empty() {
        let note = "Aucun résultat.";
        p.label(
            &Label::new(note, 14.0, theme::TEXT_MUTED),
            x + 20.0,
            y,
            44.0,
            width - 40.0,
        );
        p.describe(rect(x + 20.0, y, width - 40.0, 44.0), Role::Text, note);
    }

    // ---- Pied ----
    let foot_y = top + height - footer;
    p.fill(rect(x, foot_y, width, 1.0), theme::BORDER_SOFT, 0.0);
    let mut hint_x = x + 18.0;
    for hint in [
        "↑ ↓ pour naviguer",
        "Entrée pour ouvrir",
        "Tab pour changer de filtre",
    ] {
        let label = Label::new(hint, 12.0, theme::TEXT_MUTED);
        hint_x += p.label(&label, hint_x, foot_y, footer, width) + 16.0;
    }
    p.list.items.push(DisplayItem::PopClip);

    // Un clic dans la boîte hors d'une ligne ne fait rien ; ailleurs, il la ferme.
    p.hits.push((dialog, Hit::PaletteInput));
    p.hits.push((rect(0.0, 0.0, w, h), Hit::PaletteDismiss));
}
