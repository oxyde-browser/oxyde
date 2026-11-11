//! Composition d'une image : interface, puis chaque page découpée dans son volet.

use oxyde_css::Viewport;
use oxyde_gpu::{GpuScene, Transform, append_display_list, pop_clip, push_clip};
use oxyde_text::TextContext;
use oxyde_ui::{Chrome, build_chrome};

use crate::browser::Browser;

/// Rayon des coins d'un volet de contenu.
const PANE_RADIUS: f32 = 12.0;

/// Dessine la fenêtre entière dans `scene` et renvoie l'interface construite
/// (zones cliquables et volets) pour le traitement des événements.
///
/// `width` et `height` sont en pixels logiques ; `scale` convertit vers les
/// pixels physiques.
pub(crate) fn compose(
    browser: &mut Browser,
    text: &mut TextContext,
    scene: &mut GpuScene,
    width: f32,
    height: f32,
    scale: f64,
) -> Chrome {
    let chrome = {
        let views = browser.tab_views();
        let state = browser.chrome_state(&views, width, height);
        build_chrome(&state, text)
    };
    let to_device = Transform::scale(scale);
    scene.reset();
    append_display_list(scene, &chrome.list, to_device);

    for pane in &chrome.panes {
        let Some(tab) = browser.tab_mut(pane.tab) else {
            continue;
        };
        if pane.rect.w <= 0.0 || pane.rect.h <= 0.0 {
            continue;
        }
        tab.page.update(
            text,
            Viewport {
                width: pane.rect.w,
                height: pane.rect.h,
            },
        );
        tab.scroll = tab
            .scroll
            .clamp(0.0, (tab.page.content_height() - pane.rect.h).max(0.0));
        // Deux découpes : les coins arrondis du volet, puis la zone de la page
        // (sous l'en-tête en vue scindée).
        push_clip(scene, pane.frame, PANE_RADIUS, to_device);
        push_clip(scene, pane.rect, 0.0, to_device);
        let offset = (f64::from(pane.rect.x), f64::from(pane.rect.y - tab.scroll));
        append_display_list(
            scene,
            tab.page.display_list(),
            to_device * Transform::translate(offset),
        );
        pop_clip(scene);
        pop_clip(scene);
    }
    append_display_list(scene, &chrome.overlay, to_device);
    chrome
}
