//! Disposition « Onglets horizontaux » (écran `Horizontal` de la maquette).
//!
//! Les onglets sont dans la barre de titre ; dessous, une barre d'outils
//! porte la navigation, l'adresse, la vue scindée, les espaces et les
//! paramètres. La page occupe le reste de la fenêtre.
//!
//! LIMITES CONNUES : la bande d'onglets ne défile pas (les onglets
//! rétrécissent, puis sont coupés) ; les onglets fermés récemment ne sont
//! pas listés ; les boutons Extensions, Profil et Menu de la maquette
//! n'existent pas, les fonctions correspondantes non plus.

use crate::{
    ChromeState, Hit, ICON_BACK, ICON_CLOSE, ICON_FORWARD, ICON_GLOBE, ICON_LOCK, ICON_LOGO,
    ICON_PLUS, ICON_RELOAD, ICON_SETTINGS, ICON_SPLIT, ICON_WIN_CLOSE, ICON_WIN_MAX, ICON_WIN_MIN,
    Label, Painter, Role, panes, rect, tab_title, theme,
};

/// Hauteur de la barre de titre, qui porte les onglets.
pub(crate) const HEADER: f32 = 44.0;
/// Hauteur de la barre d'outils.
pub(crate) const TOOLBAR: f32 = 48.0;

/// Barre de titre : logo, onglets, « nouvel onglet », boutons de la fenêtre.
pub(crate) fn header(p: &mut Painter<'_>, state: &ChromeState<'_>) {
    let w = state.width;
    p.stroke_path(ICON_LOGO, 16.0, 13.0, p.accent, 2.5);
    p.dot(16.0 + 13.6, 13.0 + 4.4, 1.6, theme::AMBER);

    // Onglets épinglés et onglets de l'espace courant, dans l'ordre.
    let shown: Vec<usize> = (0..state.tabs.len())
        .filter(|index| {
            state
                .tabs
                .get(*index)
                .is_some_and(|tab| tab.pinned || tab.space == state.space)
        })
        .collect();
    let left = 46.0;
    let right = w - 138.0 - 8.0 - 36.0 - 4.0;
    #[expect(clippy::cast_precision_loss, reason = "nombre d'onglets")]
    let count = shown.len().max(1) as f32;
    let tab_w = ((right - left - 4.0 * (count - 1.0)) / count).clamp(40.0, 232.0);
    let mut x = left;
    for index in shown {
        let Some(tab) = state.tabs.get(index) else {
            continue;
        };
        if x + tab_w > right + 0.5 {
            break;
        }
        // Le bas de l'onglet actif se fond dans la barre d'outils.
        let r = rect(x, HEADER - 38.0, tab_w, 38.0);
        let hit = Hit::Tab(index);
        let close = Hit::CloseTab(index);
        let active = index == state.active || state.split == Some(index);
        let hovered = p.hover == Some(hit) || p.hover == Some(close);
        if active {
            p.fill(rect(r.x, r.y, r.w, r.h + 10.0), theme::SURFACE, 10.0);
        } else if hovered {
            p.fill(rect(r.x, r.y, r.w, r.h - 4.0), theme::HOVER, 8.0);
        }
        let title = tab_title(tab);
        p.badge(
            rect(r.x + 12.0, r.y + 10.0, 18.0, 18.0),
            5.0,
            tab.host,
            title,
            11.0,
        );
        let mut text_right = r.x + r.w - 10.0;
        if (active || hovered) && tab_w >= 96.0 {
            let button = rect(r.x + r.w - 8.0 - 24.0, r.y + 7.0, 24.0, 24.0);
            if p.hover == Some(close) {
                p.fill(button, theme::HOVER_STRONG, 6.0);
            }
            p.stroke_path(ICON_CLOSE, button.x + 4.0, button.y + 4.0, theme::ICON, 1.5);
            p.target(button, close, Role::Button, "Fermer l'onglet");
            text_right = button.x - 6.0;
        }
        let label = if active {
            Label::new(title, 14.0, theme::WHITE).weight(500.0)
        } else {
            Label::new(title, 14.0, theme::TEXT_TAB)
        };
        let text_x = r.x + 12.0 + 18.0 + 10.0;
        p.label(&label, text_x, r.y, r.h, text_right - text_x);
        p.target(r, hit, Role::Tab, title);
        p.last_widget(|widget| widget.selected = index == state.active);
        x += tab_w + 4.0;
    }
    p.icon_button(
        rect(x + 4.0, 4.0, 36.0, 36.0),
        Hit::NewTab,
        "Nouvel onglet",
        ICON_PLUS,
        16.0,
        theme::ICON,
        1.5,
    );

    let buttons = [
        (Hit::Minimize, ICON_WIN_MIN, "Réduire"),
        (Hit::Maximize, ICON_WIN_MAX, "Agrandir"),
        (Hit::CloseWindow, ICON_WIN_CLOSE, "Fermer"),
    ];
    for (index, (hit, icon, label)) in buttons.into_iter().enumerate() {
        #[expect(clippy::cast_precision_loss, reason = "trois boutons")]
        let r = rect(w - 46.0 * (3 - index) as f32, 0.0, 46.0, 40.0);
        if p.hover == Some(hit) {
            let color = if hit == Hit::CloseWindow {
                theme::DANGER
            } else {
                theme::HOVER
            };
            p.fill(r, color, 0.0);
        }
        p.stroke_path(icon, r.x + 17.0, r.y + 14.0, theme::ICON, 1.0);
        p.target(r, hit, Role::Button, label);
    }
    // Le reste de la barre déplace la fenêtre.
    p.hits
        .push((rect(0.0, 0.0, w - 138.0, HEADER), Hit::TitleBar));
}

/// Barre d'outils : navigation, adresse, vue scindée, espaces, paramètres.
pub(crate) fn toolbar(p: &mut Painter<'_>, state: &ChromeState<'_>) {
    let w = state.width;
    p.fill(rect(0.0, HEADER, w, TOOLBAR), theme::SURFACE, 0.0);
    let y = HEADER + 6.0;
    let nav = [
        (Hit::Back, ICON_BACK, state.can_back, "Page précédente"),
        (
            Hit::Forward,
            ICON_FORWARD,
            state.can_forward,
            "Page suivante",
        ),
        (Hit::Reload, ICON_RELOAD, true, "Actualiser"),
    ];
    let mut x = 12.0;
    for (hit, icon, enabled, label) in nav {
        let r = rect(x, y, 36.0, 36.0);
        if enabled {
            p.icon_button(r, hit, label, icon, 16.0, theme::ICON, 1.5);
        } else {
            p.stroke_path(icon, r.x + 10.0, r.y + 10.0, theme::ICON_DISABLED, 1.5);
            p.describe(r, Role::Button, label);
            p.last_widget(|widget| widget.disabled = true);
        }
        x += 38.0;
    }

    // À droite : les espaces, puis les paramètres.
    let mut right = w - 12.0;
    let settings = rect(right - 36.0, y, 36.0, 36.0);
    p.icon_button(
        settings,
        Hit::Settings,
        "Paramètres",
        ICON_SETTINGS,
        16.0,
        theme::ICON,
        1.5,
    );
    right = settings.x - 6.0;
    for (index, space) in state.spaces.iter().enumerate().rev() {
        let r = rect(right - 28.0, y + 4.0, 28.0, 28.0);
        let hit = Hit::Space(index);
        let current = index == state.space;
        if current {
            p.fill(r, theme::PILL, 14.0);
        } else if p.hover == Some(hit) {
            p.fill(r, theme::HOVER_STRONG, 14.0);
        }
        p.dot(r.x + 14.0, r.y + 14.0, 4.0, space.color);
        p.target(r, hit, Role::Button, space.name);
        p.last_widget(|widget| widget.selected = current);
        right = r.x - 2.0;
    }

    // Adresse : une pilule entre les deux groupes de boutons.
    let field = rect(x + 6.0, y, (right - 8.0 - x - 6.0).max(0.0), 36.0);
    let active_is_new_tab = matches!(
        state.tabs.get(state.active).map(|tab| tab.content),
        Some(crate::PaneContent::NewTab)
    );
    let editing = state.editing && !active_is_new_tab;
    p.fill(field, theme::WINDOW, 18.0);
    let border = if editing {
        p.accent
    } else {
        theme::BORDER_SOFT
    };
    p.outline(field, border, 18.0, 1.0);
    p.target(field, Hit::Address, Role::TextInput, "Adresse");
    p.last_widget(|widget| widget.value = Some(state.address.to_owned()));
    let icon = if state.secure { ICON_LOCK } else { ICON_GLOBE };
    p.stroke_path(icon, field.x + 14.0, field.y + 10.0, theme::TEXT_MUTED, 1.3);
    let split = rect(field.x + field.w - 4.0 - 32.0, field.y + 4.0, 32.0, 28.0);
    let text_x = field.x + 14.0 + 16.0 + 8.0;
    let text_w = split.x - 8.0 - text_x;
    if editing {
        panes::edit_field(p, state, text_x, field.y, field.h, text_w, 14.0, "");
    } else {
        let shown = if state.editing { "" } else { state.address };
        p.label(
            &Label::new(shown, 14.0, theme::TEXT),
            text_x,
            field.y,
            field.h,
            text_w,
        );
    }
    // La vue scindée, au bout de la pilule : déclarée après le champ, elle
    // est dessinée dessus et son clic passe avant.
    let color = if state.split.is_some() {
        p.accent
    } else {
        theme::ICON
    };
    if p.hover == Some(Hit::SplitToggle) {
        p.fill(split, theme::HOVER_STRONG, 14.0);
    }
    p.stroke_path(ICON_SPLIT, split.x + 8.0, split.y + 6.0, color, 1.5);
    p.hits
        .insert(p.hits.len().saturating_sub(1), (split, Hit::SplitToggle));
    p.describe(split, Role::Button, "Vue scindée");
    p.last_widget(|widget| {
        widget.hit = Some(Hit::SplitToggle);
        widget.selected = state.split.is_some();
    });
}
