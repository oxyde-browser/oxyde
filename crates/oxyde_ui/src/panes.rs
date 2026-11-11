//! Pages internes dessinées par l'interface : « Nouvel onglet » et pages d'erreur.

use oxyde_paint::Rect;

use crate::{
    ChromeState, DISPLAY_FONT, ErrorStyle, ErrorView, Hit, Label, MONO_FONT, Painter, Role, rect,
    tab_title, theme,
};

pub(crate) const ICON_SEARCH: &str = "M11 7a4 4 0 1 1-8 0 4 4 0 0 1 8 0zM10 10l3.5 3.5";
pub(crate) const ICON_OFFLINE: &str =
    "M6 16c8-8 20-8 28 0M11 22c5-5 13-5 18 0M16 28c2.5-2.5 5.5-2.5 8 0M6 34 34 6";
pub(crate) const ICON_WARNING: &str = "M20 5 36 33H4zM20 16v8M20 28.5v.5";

/// Texte en cours de saisie : sélection ou curseur, sinon texte indicatif.
#[expect(clippy::too_many_arguments, reason = "géométrie explicite du champ")]
pub(crate) fn edit_field(
    p: &mut Painter<'_>,
    state: &ChromeState<'_>,
    x: f32,
    y: f32,
    h: f32,
    max_w: f32,
    size: f32,
    placeholder: &str,
) {
    let caret_h = size + 6.0;
    let caret_y = y + (h - caret_h) / 2.0;
    if state.address.is_empty() {
        p.label(
            &Label::new(placeholder, size, theme::TEXT_MUTED),
            x,
            y,
            h,
            max_w,
        );
        p.fill(rect(x, caret_y, 1.5, caret_h), theme::ACCENT, 0.0);
        return;
    }
    let label = Label::new(state.address, size, theme::TEXT);
    let width = p.measure(&label).min(max_w);
    if state.select_all {
        p.fill(
            rect(x - 2.0, caret_y - 1.0, width + 4.0, caret_h + 2.0),
            theme::SELECTION,
            4.0,
        );
    }
    p.label(&label, x, y, h, max_w);
    if !state.select_all {
        // `get` : une position hors limite ou au milieu d'un caractère se replie en fin de texte.
        let before = state.address.get(..state.caret).unwrap_or(state.address);
        let caret_x = p.measure(&Label::new(before, size, theme::TEXT)).min(max_w);
        p.fill(
            rect(x + caret_x + 1.0, caret_y, 1.5, caret_h),
            theme::ACCENT,
            0.0,
        );
    }
}

/// Fond arrondi d'une page interne ; en vue scindée l'en-tête occupe le haut
/// du volet, donc seuls les coins du bas sont arrondis par la découpe du volet.
fn background(
    p: &mut Painter<'_>,
    page: Rect,
    split: bool,
    fill: oxyde_css::Color,
    border: oxyde_css::Color,
) {
    let radius = if split { 0.0 } else { 12.0 };
    p.fill(page, fill, radius);
    p.outline(page, border, radius, 1.0);
}

/// Titre de section en petites capitales espacées.
fn section_title(p: &mut Painter<'_>, text: &str, x: f32, y: f32, w: f32) {
    p.label(&Label::new(text, 11.0, theme::TEXT_MUTED), x, y, 16.0, w);
    p.describe(rect(x, y, w, 16.0), Role::Heading, text);
}

/// Page « Nouvel onglet » de l'onglet `tab`, dans la zone `page`.
pub(crate) fn new_tab(
    p: &mut Painter<'_>,
    state: &ChromeState<'_>,
    tab: usize,
    page: Rect,
    split: bool,
) {
    background(p, page, split, theme::PANEL, theme::PANEL_BORDER);
    let column_w = (page.w - 48.0).clamp(0.0, 720.0);
    let x = page.x + (page.w - column_w) / 2.0;
    let mut y = page.y + 96.0f32.min(page.h * 0.12).max(32.0);

    let heading = "Où allons-nous ?";
    p.label(
        &Label::new(heading, 32.0, theme::TEXT)
            .weight(600.0)
            .family(DISPLAY_FONT)
            .line(1.25),
        x,
        y,
        40.0,
        column_w,
    );
    p.describe(rect(x, y, column_w, 40.0), Role::Heading, heading);
    y += 40.0 + 12.0;

    // Champ de recherche : c'est la barre d'adresse de cet onglet.
    let field = rect(x, y, column_w, 56.0);
    let focused = state.editing && tab == state.active;
    if focused {
        let ring = rect(field.x - 3.0, field.y - 3.0, field.w + 6.0, field.h + 6.0);
        p.outline(ring, theme::FOCUS_RING, 17.0, 3.0);
    }
    p.fill(field, theme::SURFACE, 14.0);
    p.outline(field, theme::BORDER_HINT, 14.0, 1.0);
    p.stroke_path(
        ICON_SEARCH,
        field.x + 16.0,
        field.y + 19.0,
        theme::TEXT_MUTED,
        1.5,
    );
    let text_x = field.x + 16.0 + 18.0 + 12.0;
    let hint_w = 56.0;
    let max_w = field.x + field.w - 16.0 - hint_w - text_x;
    let placeholder = "Rechercher ou saisir une adresse";
    if focused {
        edit_field(p, state, text_x, field.y, field.h, max_w, 16.0, placeholder);
    } else {
        p.label(
            &Label::new(placeholder, 16.0, theme::TEXT_MUTED),
            text_x,
            field.y,
            field.h,
            max_w,
        );
    }
    p.key_hint("Ctrl K", field.x + field.w - 16.0, field.y, field.h);
    p.target(field, Hit::Address, Role::TextInput, placeholder);
    if focused {
        p.last_widget(|w| w.value = Some(state.address.to_owned()));
    }
    y += 56.0 + 40.0;

    // Épinglés.
    let pinned: Vec<usize> = (0..state.tabs.len())
        .filter(|i| state.tabs[*i].pinned)
        .collect();
    if !pinned.is_empty() {
        section_title(p, "ÉPINGLÉS", x, y, column_w);
        y += 16.0 + 12.0;
        // Autant de colonnes d'au moins 150 px que la largeur le permet.
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "quotient positif borné par la largeur de la colonne (≤ 720)"
        )]
        let columns = (((column_w + 12.0) / 162.0).floor() as usize).clamp(1, pinned.len().max(1));
        #[expect(clippy::cast_precision_loss, reason = "au plus quatre colonnes")]
        let tile_w = (column_w - 12.0 * (columns as f32 - 1.0)) / columns as f32;
        for (n, &i) in pinned.iter().enumerate() {
            let t = &state.tabs[i];
            #[expect(clippy::cast_precision_loss, reason = "peu d'onglets épinglés")]
            let tile = rect(
                x + (tile_w + 12.0) * (n % columns) as f32,
                y + 84.0 * (n / columns) as f32,
                tile_w,
                72.0,
            );
            let hovered = p.hover == Some(Hit::Tab(i));
            p.fill(
                tile,
                if hovered {
                    theme::ACTIVE
                } else {
                    theme::SURFACE
                },
                12.0,
            );
            p.outline(tile, theme::BORDER_SOFT, 12.0, 1.0);
            p.badge(
                rect(tile.x + 16.0, tile.y + 20.0, 32.0, 32.0),
                8.0,
                t.host,
                t.title,
                14.0,
            );
            let title = tab_title(t);
            p.label(
                &Label::new(title, 14.0, theme::TEXT),
                tile.x + 60.0,
                tile.y,
                72.0,
                tile.w - 76.0,
            );
            p.target(tile, Hit::Tab(i), Role::Button, title);
        }
        #[expect(clippy::cast_precision_loss, reason = "peu d'onglets épinglés")]
        let rows = pinned.len().div_ceil(columns) as f32;
        y += rows * 84.0 - 12.0 + 40.0;
    }

    // Fermés récemment.
    if !state.closed.is_empty() {
        section_title(p, "FERMÉS RÉCEMMENT", x, y, column_w);
        y += 16.0 + 8.0;
        for (k, closed) in state.closed.iter().enumerate().take(6) {
            let row = rect(x, y, column_w, 44.0);
            if row.y + row.h > page.y + page.h {
                break;
            }
            if p.hover == Some(Hit::Reopen(k)) {
                p.fill(row, theme::HOVER, 10.0);
            }
            p.badge(
                rect(row.x + 12.0, row.y + 12.0, 20.0, 20.0),
                5.0,
                closed.host,
                closed.title,
                11.0,
            );
            // Le plus récent est celui que rouvre le raccourci clavier.
            let aside = if k == 0 {
                "Ctrl Maj T pour rouvrir"
            } else {
                closed.host
            };
            let aside_label = Label::new(aside, 12.0, theme::TEXT_MUTED);
            let aside_w = p.measure(&aside_label).min(column_w / 2.0);
            p.label(
                &aside_label,
                row.x + row.w - 12.0 - aside_w,
                row.y,
                44.0,
                aside_w + 1.0,
            );
            p.label(
                &Label::new(closed.title, 14.0, theme::TEXT_TAB),
                row.x + 44.0,
                row.y,
                44.0,
                row.w - 44.0 - 24.0 - aside_w,
            );
            p.target(
                row,
                Hit::Reopen(k),
                Role::Button,
                &format!("Rouvrir {}", closed.title),
            );
            y += 44.0 + 4.0;
        }
    }
}

/// Page d'erreur de l'onglet `tab`, dans la zone `page`.
pub(crate) fn error(
    p: &mut Painter<'_>,
    error: &ErrorView<'_>,
    tab: usize,
    page: Rect,
    split: bool,
) {
    let certificate = error.style == ErrorStyle::Certificate;
    let (fill, border, icon_color, body_color, code_color) = if certificate {
        (
            theme::DANGER_PANEL,
            theme::DANGER_BORDER,
            theme::DANGER_SOFT,
            theme::DANGER_TEXT,
            theme::DANGER_MUTED,
        )
    } else {
        (
            theme::PANEL,
            theme::PANEL_BORDER,
            theme::TEXT_MUTED,
            theme::ICON,
            theme::TEXT_MUTED,
        )
    };
    background(p, page, split, fill, border);
    let x = page.x + 40.0;
    let content_w = (page.w - 80.0).clamp(0.0, 640.0);
    let mut y = page.y + 96.0f32.min(page.h * 0.12).max(32.0);

    let icon = if certificate {
        ICON_WARNING
    } else {
        ICON_OFFLINE
    };
    p.stroke_path(icon, x, y, icon_color, 2.5);
    y += 40.0 + 16.0;

    let title = Label::new(error.title, 28.0, theme::TEXT)
        .weight(600.0)
        .family(DISPLAY_FONT)
        .line(36.0 / 28.0);
    let title_h = p.paragraph(&title, x, y, content_w);
    p.describe(rect(x, y, content_w, title_h), Role::Heading, error.title);
    y += title_h + 16.0;

    let message = Label::new(error.message, 16.0, body_color).line(26.0 / 16.0);
    let message_h = p.paragraph(&message, x, y, content_w);
    p.describe(rect(x, y, content_w, message_h), Role::Text, error.message);
    y += message_h + 16.0 + 8.0;

    // Boutons d'action.
    let (primary, secondary) = match error.style {
        ErrorStyle::Network => ("Réessayer", Some("Diagnostic réseau de Windows")),
        ErrorStyle::Certificate => (
            "Revenir en lieu sûr",
            Some(if error.detail.is_some() {
                "Masquer les détails"
            } else {
                "Afficher les détails"
            }),
        ),
        ErrorStyle::Other => ("Réessayer", None),
    };
    let (primary_fill, primary_text, secondary_fill) = if certificate {
        (
            theme::DANGER_SOFT,
            theme::DANGER_ON_SOFT,
            theme::DANGER_BUTTON,
        )
    } else {
        (theme::ACCENT, theme::ON_ACCENT, theme::ACTIVE)
    };
    let mut button_x = x;
    let mut button = |p: &mut Painter<'_>, text: &str, hit: Hit, fill, color, weight: f32| {
        let label = Label::new(text, 14.0, color).weight(weight);
        let w = p.measure(&label) + 36.0;
        let r = rect(button_x, y, w, 44.0);
        p.fill(r, fill, 10.0);
        if p.hover == Some(hit) {
            p.outline(r, theme::WHITE, 10.0, 1.0);
        }
        p.label_centered(&label, r);
        p.target(r, hit, Role::Button, text);
        button_x += w + 8.0;
    };
    button(
        p,
        primary,
        Hit::ErrorPrimary(tab),
        primary_fill,
        primary_text,
        600.0,
    );
    if let Some(text) = secondary {
        button(
            p,
            text,
            Hit::ErrorSecondary(tab),
            secondary_fill,
            theme::WHITE,
            500.0,
        );
    }
    y += 44.0 + 24.0;

    if let Some(detail) = error.detail {
        let label = Label::new(detail, 13.0, code_color)
            .family(MONO_FONT)
            .line(1.5);
        let h = p.paragraph(&label, x, y, content_w);
        p.describe(rect(x, y, content_w, h), Role::Text, detail);
    }

    // Code d'erreur, ancré en bas du volet.
    let code_y = page.y + page.h - 40.0 - 16.0;
    if code_y > y {
        p.label(
            &Label::new(error.code, 12.0, code_color).family(MONO_FONT),
            x,
            code_y,
            16.0,
            content_w,
        );
        p.describe(rect(x, code_y, content_w, 16.0), Role::Text, error.code);
    }
}
