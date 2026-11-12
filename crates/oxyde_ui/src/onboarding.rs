//! Premier lancement (écran `Onboarding` de la maquette).
//!
//! Deux étapes : reprendre les favoris d'un autre navigateur, puis choisir la
//! couleur d'accent. À gauche le titre et les étapes, à droite la carte de
//! l'étape en cours.
//!
//! LIMITES CONNUES : la maquette propose aussi d'importer l'historique, les
//! mots de passe et les onglets, de choisir la disposition des onglets et de
//! devenir le navigateur par défaut ; rien de cela n'existe encore, donc
//! rien de cela n'est proposé.

use oxyde_paint::Rect;

use crate::{
    ChromeState, DISPLAY_FONT, Hit, ICON_LOGO, Label, OnboardingView, Painter, Role, rect, theme,
};

const STEPS: [&str; 2] = ["Reprendre vos favoris", "Choisir la couleur d'accent"];

/// Bouton à texte ; `primary` : sur fond d'accent.
fn button(p: &mut Painter<'_>, right: f32, y: f32, text: &str, hit: Hit, primary: bool) -> f32 {
    let label = if primary {
        Label::new(text, 14.0, theme::ON_ACCENT).weight(600.0)
    } else {
        Label::new(text, 14.0, theme::TEXT_TAB)
    };
    let width = p.measure(&label) + 36.0;
    let r = rect(right - width, y, width, 44.0);
    if primary {
        p.fill(r, p.accent, 10.0);
        if p.hover == Some(hit) {
            p.outline(r, theme::WHITE, 10.0, 1.0);
        }
    } else if p.hover == Some(hit) {
        p.fill(r, theme::HOVER_STRONG, 10.0);
    }
    p.label_centered(&label, r);
    p.target(r, hit, Role::Button, text);
    width
}

/// Colonne de gauche : logo, titre, étapes.
fn introduction(p: &mut Painter<'_>, view: &OnboardingView<'_>, x: f32, y: f32, w: f32) {
    p.stroke_path(ICON_LOGO, x, y, p.accent, 2.5);
    p.dot(x + 13.6, y + 4.4, 1.6, theme::AMBER);
    let title = "Bienvenue dans Oxyde";
    let heading = Label::new(title, 48.0, theme::TEXT)
        .weight(600.0)
        .family(DISPLAY_FONT)
        .line(1.085);
    let mut y = y + 18.0 + 24.0;
    let height = p.paragraph(&heading, x, y, w);
    p.describe(rect(x, y, w, height), Role::Heading, title);
    y += height + 24.0;
    for (index, name) in STEPS.iter().enumerate() {
        let circle = rect(x, y, 28.0, 28.0);
        let number = (index + 1).to_string();
        let current = index == view.step;
        if current {
            p.fill(circle, p.accent, 14.0);
            p.label_centered(
                &Label::new(&number, 14.0, theme::ON_ACCENT).weight(600.0),
                circle,
            );
        } else {
            p.outline(circle, theme::BORDER_HINT, 14.0, 1.0);
            p.label_centered(&Label::new(&number, 14.0, theme::TEXT_SECONDARY), circle);
        }
        let label = if current {
            Label::new(name, 14.0, theme::WHITE).weight(600.0)
        } else {
            Label::new(name, 14.0, theme::TEXT_SECONDARY)
        };
        p.label(&label, x + 40.0, y, 28.0, w - 40.0);
        p.describe(rect(x, y, w, 28.0), Role::Text, name);
        p.last_widget(|widget| widget.selected = current);
        y += 40.0;
    }
}

/// Hauteur de la carte de l'étape en cours.
fn card_height(view: &OnboardingView<'_>) -> f32 {
    let body = if view.step == 0 {
        view.sources.len() as f32 * 48.0 - 4.0
    } else {
        20.0 + 24.0 + 44.0
    };
    32.0 + 28.0 + 12.0 + body + 24.0 + 44.0 + 32.0
}

/// Carte de l'étape en cours.
fn card(p: &mut Painter<'_>, view: &OnboardingView<'_>, frame: Rect) {
    p.fill(frame, theme::SURFACE, 16.0);
    p.outline(frame, theme::BORDER, 16.0, 1.0);
    let x = frame.x + 32.0;
    let w = frame.w - 64.0;
    let mut y = frame.y + 32.0;
    let legend = if view.step == 0 {
        "Importer depuis un autre navigateur"
    } else {
        "Couleur d'accent"
    };
    p.label(
        &Label::new(legend, 20.0, theme::TEXT)
            .weight(600.0)
            .family(DISPLAY_FONT),
        x,
        y,
        28.0,
        w,
    );
    p.describe(rect(x, y, w, 28.0), Role::Heading, legend);
    y += 28.0 + 12.0;

    if view.step == 0 {
        for (index, name) in view.sources.iter().enumerate() {
            let r = rect(x, y, w, 44.0);
            let hit = Hit::OnboardingSource(index);
            let chosen = index == view.selected;
            if chosen {
                p.fill(r, theme::ACTIVE, 10.0);
            } else if p.hover == Some(hit) {
                p.fill(r, theme::HOVER, 10.0);
            }
            // Bouton radio.
            let (cx, cy) = (r.x + 12.0 + 9.0, r.y + 22.0);
            if chosen {
                p.dot(cx, cy, 9.0, p.accent);
                p.dot(cx, cy, 4.0, theme::ON_ACCENT);
            } else {
                p.outline(
                    rect(cx - 9.0, cy - 9.0, 18.0, 18.0),
                    theme::ICON_DISABLED,
                    9.0,
                    1.5,
                );
            }
            p.label(
                &Label::new(name, 14.0, theme::TEXT),
                r.x + 42.0,
                r.y,
                r.h,
                w - 54.0,
            );
            p.target(r, hit, Role::Button, name);
            p.last_widget(|widget| widget.selected = chosen);
            y += 48.0;
        }
        y -= 4.0;
    } else {
        let note = match view.imported {
            Some(0) => "Aucun favori n'a été trouvé à importer.".to_owned(),
            Some(1) => "1 favori importé.".to_owned(),
            Some(count) => format!("{count} favoris importés."),
            None => "Elle colore les éléments actifs de l'interface.".to_owned(),
        };
        p.label(
            &Label::new(&note, 14.0, theme::TEXT_SECONDARY),
            x,
            y,
            20.0,
            w,
        );
        p.describe(rect(x, y, w, 20.0), Role::Text, &note);
        y += 20.0 + 24.0;
        let mut swatch_x = x;
        for (index, color) in view.accents.iter().enumerate() {
            let r = rect(swatch_x, y, 44.0, 44.0);
            let hit = Hit::OnboardingAccent(index);
            let chosen = index == view.accent;
            if chosen {
                p.outline(r, theme::TEXT, 22.0, 2.0);
            } else if p.hover == Some(hit) {
                p.outline(r, theme::ICON_DISABLED, 22.0, 1.0);
            }
            p.dot(r.x + 22.0, r.y + 22.0, 13.0, *color);
            p.target(
                r,
                hit,
                Role::Button,
                view.accent_names.get(index).copied().unwrap_or(""),
            );
            p.last_widget(|widget| widget.selected = chosen);
            swatch_x += 52.0;
        }
        y += 44.0;
    }

    // Boutons, calés à droite.
    y += 24.0;
    let right = frame.x + frame.w - 32.0;
    let importing = view.step == 0 && view.selected + 1 < view.sources.len();
    let primary = match (view.step, importing) {
        (0, true) => "Importer et continuer",
        (0, false) => "Continuer",
        _ => "Terminer",
    };
    let width = button(p, right, y, primary, Hit::OnboardingNext, true);
    if view.step == 0 {
        button(
            p,
            right - width - 8.0,
            y,
            "Passer",
            Hit::OnboardingSkip,
            false,
        );
    }
}

/// Dessine le premier lancement sous la barre de titre.
pub(crate) fn build(p: &mut Painter<'_>, state: &ChromeState<'_>, view: &OnboardingView<'_>) {
    let top = theme::TITLE_BAR;
    let (w, h) = (state.width, state.height - top);
    let card_w = (w - 48.0).clamp(0.0, 480.0);
    let card_h = card_height(view);
    // Les deux colonnes côte à côte si la fenêtre le permet, sinon la carte seule.
    let wide = w >= 380.0 + 64.0 + 480.0 + 48.0;
    let total = if wide { 380.0 + 64.0 + card_w } else { card_w };
    let x = (w - total) / 2.0;
    let card_y = top + ((h - card_h) / 2.0).max(16.0);
    if wide {
        let intro_h = 18.0 + 24.0 + 104.0 + 24.0 + 68.0;
        introduction(p, view, x, top + ((h - intro_h) / 2.0).max(16.0), 380.0);
        card(p, view, rect(x + 380.0 + 64.0, card_y, card_w, card_h));
    } else {
        card(p, view, rect(x, card_y, card_w, card_h));
    }
}
