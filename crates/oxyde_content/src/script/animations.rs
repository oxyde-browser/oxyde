//! API Web Animations : les deux opérations que `animations.js` demande au
//! moteur. Les animations créées par `Element.animate()` vivent dans
//! l'animateur du style, à côté des animations CSS, et suivent la même
//! horloge.

use std::sync::Arc;

use oxyde_css::ScriptAnimation;
use oxyde_js::{JsResult, Value, Vm};

use super::{arg, host, node_arg};

/// Nombre maximal d'images clés d'une animation.
const MAX_FRAMES: usize = 1024;

/// Animations de la page, partagées avec le calcul du style.
fn clock(vm: &mut Vm) -> Option<Arc<crate::Animations>> {
    host(vm)?
        .layout_env
        .as_ref()
        .map(|env| Arc::clone(&env.animations))
}

/// Ramène un nombre du script à un `f32` ; l'infini reste l'infini.
#[expect(clippy::cast_possible_truncation, reason = "valeur bornée juste avant")]
fn narrow(value: f64) -> f32 {
    if value.is_infinite() {
        return if value > 0.0 {
            f32::INFINITY
        } else {
            f32::NEG_INFINITY
        };
    }
    value.clamp(-1.0e12, 1.0e12) as f32
}

/// Identifiant d'animation porté par un nombre du script.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "entier positif vérifié juste avant"
)]
fn id_of(value: f64) -> Option<u64> {
    // Un nombre hors de l'intervalle (ou NaN) ne désigne aucune animation.
    (1.0..9.0e15).contains(&value).then_some(value as u64)
}

/// La page doit être restylée : une animation a changé.
fn invalidate(vm: &mut Vm) {
    if let Some(h) = host(vm) {
        h.dirty = true;
    }
}

/// `animate(élément, images, durée, délai, itérations, direction,
/// remplissage, temporisation)` : crée l'animation et rend son identifiant.
/// `images` est une liste de couples `[position, déclarations CSS]`.
pub(super) fn animate(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let node = node_arg(vm, &arg(args, 0))?;
    let list = arg(args, 1);
    let length = vm.get(&list, "length")?;
    let count = vm.to_number(&length)?;
    let mut frames = Vec::new();
    let mut index = 0.0;
    while index < count && frames.len() < MAX_FRAMES {
        let item = vm.get(&list, &index.to_string())?;
        let offset = vm.get(&item, "0")?;
        let offset = vm.to_number(&offset)?;
        let css = vm.get(&item, "1")?;
        let css = vm.to_string(&css)?.to_string();
        frames.push((narrow(offset), css));
        index += 1.0;
    }
    let number = |vm: &mut Vm, at: usize| vm.to_number(&arg(args, at));
    let duration = number(vm, 2)?;
    let delay = number(vm, 3)?;
    let iterations = number(vm, 4)?;
    let spec = ScriptAnimation {
        frames,
        duration: narrow(duration),
        delay: narrow(delay),
        iterations: narrow(iterations),
        direction: vm.to_string(&arg(args, 5))?.to_string(),
        fill: vm.to_string(&arg(args, 6))?.to_string(),
        easing: vm.to_string(&arg(args, 7))?.to_string(),
    };
    let Some(animations) = clock(vm) else {
        return Err(vm.error("TypeError", "animation : la page n'a pas d'horloge"));
    };
    let added = animations.script(|animator, now| animator.add_script_animation(node, &spec, now));
    match added {
        Ok(id) => {
            invalidate(vm);
            // Un identifiant reste bien en deçà de 2^53.
            #[expect(clippy::cast_precision_loss, reason = "compteur d'animations")]
            let id = id as f64;
            Ok(Value::Number(id))
        }
        Err(message) => Err(vm.error("TypeError", &format!("animation : {message}"))),
    }
}

/// `control(identifiant, opération, valeur)` :
/// - `state` : 0 inconnue, 1 en cours, 2 en pause, 3 terminée ;
/// - `time` : temps courant en millisecondes, ou `null` ;
/// - `seek` (valeur : le temps), `pause`, `play`, `finish`, `cancel` :
///   `true` si l'animation existe.
pub(super) fn control(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let id = vm.to_number(&arg(args, 0))?;
    let operation = vm.to_string(&arg(args, 1))?.to_string();
    let value = match arg(args, 2) {
        Value::Undefined => 0.0,
        other => vm.to_number(&other)?,
    };
    let (Some(id), Some(animations)) = (id_of(id), clock(vm)) else {
        return Ok(match operation.as_str() {
            "state" => Value::Number(0.0),
            "time" => Value::Null,
            _ => Value::Bool(false),
        });
    };
    let outcome = animations.script(|animator, now| match operation.as_str() {
        "state" => Value::Number(f64::from(animator.script_animation_state(id, now))),
        "time" => animator
            .script_animation_time(id, now)
            .map_or(Value::Null, Value::Number),
        "seek" => Value::Bool(animator.seek_script_animation(id, value, now)),
        "pause" => Value::Bool(animator.pause_script_animation(id, now)),
        "play" => Value::Bool(animator.play_script_animation(id, now)),
        "finish" => Value::Bool(animator.finish_script_animation(id, now)),
        "cancel" => Value::Bool(animator.cancel_script_animation(id)),
        _ => Value::Undefined,
    });
    if !matches!(operation.as_str(), "state" | "time") {
        invalidate(vm);
    }
    Ok(outcome)
}
