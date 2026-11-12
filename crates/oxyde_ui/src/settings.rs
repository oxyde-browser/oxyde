//! Écran « Paramètres » de la maquette.
//!
//! Comme la bibliothèque, il occupe la fenêtre sous la barre de titre : les
//! rubriques à gauche, les réglages à droite, rubrique après rubrique. Son
//! contenu est décrit par le navigateur ([`SettingsView`]) ; ce module ne
//! connaît que des sortes de commandes (interrupteur, choix, pastilles,
//! bouton).
//!
//! LIMITES CONNUES : seuls les réglages qui agissent vraiment sont montrés ;
//! la disposition des onglets, le thème clair et les réglages de
//! performances de la maquette n'y figurent pas encore.

use oxyde_paint::{DisplayItem, Rect};

use crate::{
    ChromeState, DISPLAY_FONT, Hit, ICON_BACK, Label, Painter, Role, SettingControl, SettingsView,
    rect, theme,
};

/// Marge intérieure du panneau, en haut et en bas.
const PANEL_PADDING: f32 = 48.0;
/// Espace entre deux rubriques.
const GROUP_GAP: f32 = 40.0;

/// Partie de `r` située dans `clip` ; `None` si elle est vide.
fn visible(r: Rect, clip: Rect) -> Option<Rect> {
    let top = r.y.max(clip.y);
    let bottom = (r.y + r.h).min(clip.y + clip.h);
    (bottom > top).then(|| rect(r.x, top, r.w, bottom - top))
}

/// Hauteur de la commande d'un réglage.
fn control_height(control: &SettingControl<'_>) -> f32 {
    match control {
        SettingControl::Switch { .. } => 24.0,
        SettingControl::Choice { .. }
        | SettingControl::Swatches { .. }
        | SettingControl::Button { .. } => 44.0,
        SettingControl::Value { .. } => 20.0,
    }
}

/// Hauteur d'une ligne de réglage, marges comprises.
fn row_height(description: &str, control: &SettingControl<'_>) -> f32 {
    let text = if description.is_empty() { 20.0 } else { 40.0 };
    control_height(control).max(text) + 28.0
}

/// Position de chaque rubrique dans le contenu (origine : haut du panneau),
/// et hauteur totale du contenu.
fn layout(view: &SettingsView<'_>) -> (Vec<f32>, f32) {
    let mut y = PANEL_PADDING + 40.0 + GROUP_GAP;
    let mut offsets = Vec::with_capacity(view.groups.len());
    for (index, group) in view.groups.iter().enumerate() {
        if index > 0 {
            y += GROUP_GAP;
        }
        offsets.push(y);
        y += 24.0 + 8.0;
        for row in &group.rows {
            y += row_height(row.description, &row.control);
        }
    }
    (offsets, y + PANEL_PADDING)
}

/// Colonne des rubriques.
fn navigation(p: &mut Painter<'_>, view: &SettingsView<'_>) {
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
    for (index, group) in view.groups.iter().enumerate() {
        let r = rect(x, y, w, 40.0);
        let hit = Hit::SettingsGroup(index);
        let selected = view.selected == index;
        if selected {
            p.fill(r, theme::ACTIVE, 8.0);
        } else if p.hover == Some(hit) {
            p.fill(r, theme::HOVER, 8.0);
        }
        let label = if selected {
            Label::new(group.title, 14.0, theme::WHITE).weight(500.0)
        } else {
            Label::new(group.title, 14.0, theme::TEXT_TAB)
        };
        p.label(&label, r.x + 12.0, r.y, r.h, w - 24.0);
        p.target(r, hit, Role::Tab, group.title);
        p.last_widget(|widget| widget.selected = selected);
        y += 42.0;
    }
}

/// Dessine la commande d'un réglage, calée à droite dans la bande `band` ;
/// `row` est l'index du réglage porté par ses zones cliquables.
fn control(
    p: &mut Painter<'_>,
    control: &SettingControl<'_>,
    band: Rect,
    row: usize,
    title: &str,
    panel: Rect,
) {
    let right = band.x + band.w;
    let height = control_height(control);
    let top = band.y + (band.h - height) / 2.0;
    let target =
        |p: &mut Painter<'_>, r: Rect, option: usize, role: Role, name: &str, selected: bool| {
            if let Some(shown) = visible(r, panel) {
                p.target(shown, Hit::Setting { row, option }, role, name);
                p.last_widget(|widget| widget.selected = selected);
            }
        };
    match control {
        SettingControl::Switch { on } => {
            let r = rect(right - 44.0, top, 44.0, 24.0);
            if *on {
                p.fill(r, p.accent, 12.0);
                p.dot(r.x + r.w - 12.0, r.y + 12.0, 9.0, theme::ON_ACCENT);
            } else {
                p.fill(r, theme::SWITCH_OFF, 12.0);
                p.outline(r, theme::ICON_DISABLED, 12.0, 1.0);
                p.dot(r.x + 12.0, r.y + 12.0, 9.0, theme::ICON);
            }
            // La zone cliquable déborde du dessin : 44 px de haut.
            target(
                p,
                rect(r.x, r.y - 10.0, r.w, 44.0),
                0,
                Role::Switch,
                title,
                *on,
            );
        }
        SettingControl::Choice { options, selected } => {
            let widths: Vec<f32> = options
                .iter()
                .map(|option| {
                    p.measure(&Label::new(option, 14.0, theme::WHITE).weight(500.0)) + 28.0
                })
                .collect();
            let total: f32 = widths.iter().sum::<f32>() + 4.0 * (options.len() as f32 + 1.0);
            let frame = rect(right - total, top, total, 44.0);
            p.fill(frame, theme::SURFACE, 10.0);
            let mut x = frame.x + 4.0;
            for (index, (option, width)) in options.iter().zip(&widths).enumerate() {
                let r = rect(x, frame.y + 4.0, *width, 36.0);
                let chosen = index == *selected;
                let hit = Hit::Setting { row, option: index };
                if chosen {
                    p.fill(r, theme::SWITCH_OFF, 7.0);
                } else if p.hover == Some(hit) {
                    p.fill(r, theme::HOVER, 7.0);
                }
                let label = if chosen {
                    Label::new(option, 14.0, theme::WHITE).weight(500.0)
                } else {
                    Label::new(option, 14.0, theme::TEXT_TAB)
                };
                p.label_centered(&label, r);
                target(p, r, index, Role::Button, option, chosen);
                x += width + 4.0;
            }
        }
        SettingControl::Swatches {
            colors,
            names,
            selected,
        } => {
            let count = colors.len() as f32;
            let mut x = right - (44.0 * count + 8.0 * (count - 1.0).max(0.0));
            for (index, color) in colors.iter().enumerate() {
                let r = rect(x, top, 44.0, 44.0);
                let chosen = index == *selected;
                if chosen {
                    p.outline(r, theme::TEXT, 22.0, 2.0);
                } else if p.hover == Some(Hit::Setting { row, option: index }) {
                    p.outline(r, theme::ICON_DISABLED, 22.0, 1.0);
                }
                p.dot(r.x + 22.0, r.y + 22.0, 13.0, *color);
                let name = names.get(index).copied().unwrap_or("");
                target(p, r, index, Role::Button, name, chosen);
                x += 52.0;
            }
        }
        SettingControl::Button { label, danger } => {
            let text = Label::new(label, 14.0, theme::WHITE).weight(500.0);
            let width = p.measure(&text) + 32.0;
            let r = rect(right - width, top, width, 44.0);
            let hit = Hit::Setting { row, option: 0 };
            let fill = if *danger {
                theme::DANGER
            } else if p.hover == Some(hit) {
                theme::HOVER_STRONG
            } else {
                theme::ACTIVE
            };
            p.fill(r, fill, 10.0);
            p.label_centered(&text, r);
            target(p, r, 0, Role::Button, label, false);
        }
        SettingControl::Value { text } => {
            let label = Label::new(text, 14.0, theme::TEXT_MUTED);
            let width = p.measure(&label).min(band.w / 2.0);
            p.label(&label, right - width, top, 20.0, width);
        }
    }
}

/// Largeur que prend la commande d'un réglage, pour borner le texte à sa gauche.
fn control_width(p: &mut Painter<'_>, control: &SettingControl<'_>) -> f32 {
    match control {
        SettingControl::Switch { .. } => 44.0,
        SettingControl::Choice { options, .. } => {
            let labels: f32 = options
                .iter()
                .map(|option| {
                    p.measure(&Label::new(option, 14.0, theme::WHITE).weight(500.0)) + 28.0
                })
                .sum();
            labels + 4.0 * (options.len() as f32 + 1.0)
        }
        SettingControl::Swatches { colors, .. } => 52.0 * colors.len() as f32,
        SettingControl::Button { label, .. } => {
            p.measure(&Label::new(label, 14.0, theme::WHITE).weight(500.0)) + 32.0
        }
        SettingControl::Value { text } => p.measure(&Label::new(text, 14.0, theme::TEXT_MUTED)),
    }
}

/// Dessine les paramètres sous la barre de titre. Renvoie le défilement
/// maximal, et le défilement appliqué quand l'état demandait de sauter à une
/// rubrique.
pub(crate) fn build(
    p: &mut Painter<'_>,
    state: &ChromeState<'_>,
    view: &SettingsView<'_>,
) -> (f32, Option<f32>) {
    navigation(p, view);

    let panel_x = 8.0 + theme::SIDEBAR + 8.0;
    let panel = rect(
        panel_x,
        theme::TITLE_BAR,
        (state.width - panel_x - 8.0).max(0.0),
        (state.height - theme::TITLE_BAR - 8.0).max(0.0),
    );
    let (offsets, content) = layout(view);
    let scroll_max = (content - panel.h).max(0.0);
    // Sauter à une rubrique : son titre vient en haut du panneau, à la marge.
    let jumped = view
        .jump
        .and_then(|group| offsets.get(group))
        .map(|offset| (offset - PANEL_PADDING).clamp(0.0, scroll_max));
    let scroll = jumped.unwrap_or(view.scroll);

    p.fill(panel, theme::PANEL, 12.0);
    p.outline(panel, theme::PANEL_BORDER, 12.0, 1.0);
    p.list.items.push(DisplayItem::PushClip {
        rect: panel,
        radius: 12.0,
    });
    let inner_w = (panel.w - 48.0).clamp(0.0, 720.0);
    let x = panel.x + (panel.w - inner_w) / 2.0;
    let origin = panel.y - scroll;

    let heading = Label::new("Paramètres", 32.0, theme::TEXT)
        .weight(600.0)
        .family(DISPLAY_FONT)
        .line(1.25);
    p.label(&heading, x, origin + PANEL_PADDING, 40.0, inner_w);
    p.describe(
        rect(x, origin + PANEL_PADDING, inner_w, 40.0),
        Role::Heading,
        "Paramètres",
    );

    let mut row_index = 0;
    for (group, offset) in view.groups.iter().zip(&offsets) {
        let mut y = origin + offset;
        let title = Label::new(group.title, 18.0, theme::TEXT)
            .weight(600.0)
            .family(DISPLAY_FONT)
            .line(1.33);
        if visible(rect(x, y, inner_w, 24.0), panel).is_some() {
            p.label(&title, x, y, 24.0, inner_w);
            p.describe(rect(x, y, inner_w, 24.0), Role::Heading, group.title);
        }
        y += 24.0 + 8.0;
        for row in &group.rows {
            let height = row_height(row.description, &row.control);
            let band = rect(x, y, inner_w, height);
            if visible(band, panel).is_some() {
                p.fill(rect(x, y, inner_w, 1.0), theme::PANEL_BORDER, 0.0);
                let text_w = inner_w - control_width(p, &row.control) - 24.0;
                let block = if row.description.is_empty() {
                    20.0
                } else {
                    40.0
                };
                let text_y = y + (height - block) / 2.0;
                p.label(
                    &Label::new(row.title, 14.0, theme::TEXT),
                    x,
                    text_y,
                    20.0,
                    text_w,
                );
                if !row.description.is_empty() {
                    p.label(
                        &Label::new(row.description, 13.0, theme::TEXT_SECONDARY),
                        x,
                        text_y + 20.0,
                        20.0,
                        text_w,
                    );
                }
                control(p, &row.control, band, row_index, row.title, panel);
            }
            row_index += 1;
            y += height;
        }
    }
    p.list.items.push(DisplayItem::PopClip);
    (scroll_max, jumped)
}
