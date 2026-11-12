//! Composition d'une image : interface, puis chaque page découpée dans son volet.

use oxyde_css::Viewport;
use oxyde_gpu::{GpuScene, Transform, append_display_list, append_scrolled, pop_clip, push_clip};
use oxyde_text::TextContext;
use oxyde_ui::{Chrome, build_chrome};

use crate::browser::Browser;

/// Rayon des coins d'un volet de contenu.
const PANE_RADIUS: f32 = 12.0;

/// Dessine la fenêtre entière dans `scene` et renvoie l'interface construite
/// (zones cliquables, volets, éléments accessibles) pour le traitement des
/// événements.
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
        let views = browser.views();
        let state = browser.chrome_state(&views, width, height);
        build_chrome(&state, text)
    };
    let to_device = Transform::scale(scale);
    scene.reset();
    append_display_list(scene, &chrome.list, to_device);

    // Seuls les volets de pages web sont listés : les pages internes (nouvel
    // onglet, erreurs) font déjà partie de la liste de l'interface.
    for pane in &chrome.panes {
        let Some(tab) = browser.tab_mut(pane.tab) else {
            continue;
        };
        if pane.rect.w <= 0.0 || pane.rect.h <= 0.0 {
            continue;
        }
        crate::loader::set_viewport_hint(pane.rect.w, pane.rect.h);
        tab.page.update(
            text,
            Viewport {
                width: pane.rect.w,
                height: pane.rect.h,
            },
        );
        // Deux découpes : les coins arrondis du volet, puis la zone de la page
        // (sous l'en-tête en vue scindée).
        push_clip(scene, pane.frame, PANE_RADIUS, to_device);
        push_clip(scene, pane.rect, 0.0, to_device);
        // Le défilement — de la fenêtre comme des zones de la page — est
        // appliqué ici, à la composition : la liste d'affichage n'en dépend pas.
        let (scroll_x, scroll_y) = tab.page.scroll_offset();
        let offset = (
            f64::from(pane.rect.x - scroll_x),
            f64::from(pane.rect.y - scroll_y),
        );
        append_scrolled(
            scene,
            tab.page.display_list(),
            to_device * Transform::translate(offset),
            &|id| tab.page.scroll_of(id),
        );
        pop_clip(scene);
        pop_clip(scene);
    }
    append_display_list(scene, &chrome.overlay, to_device);
    chrome
}
