//! Appui natif de `NodeIterator` (DOM Standard §6.1) : le document suit le
//! nœud de référence de chaque itérateur et le déplace quand ce nœud est
//! retiré de l'arbre. Le parcours lui-même est écrit en JavaScript
//! (`traversal.js`).

use oxyde_js::{JsResult, Value, Vm};

use super::{arg, host, node_arg, wrap};

fn iterator_id(vm: &mut Vm, value: &Value) -> JsResult<usize> {
    let n = vm.to_number(value)?;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "identifiant rendu par iterNew : un petit entier positif"
    )]
    Ok(if n.is_finite() && n >= 0.0 {
        n as usize
    } else {
        usize::MAX
    })
}

/// `iterNew(racine)` : identifiant d'un nouvel itérateur.
pub(super) fn iter_new(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let root = node_arg(vm, &arg(args, 0))?;
    match host(vm).and_then(|h| h.doc.add_iterator(root)) {
        #[expect(clippy::cast_precision_loss, reason = "petit entier")]
        Some(id) => Ok(Value::Number(id as f64)),
        None => Err(vm.error("RangeError", "trop d'itérateurs créés par cette page")),
    }
}

/// `iterGet(id)` : `[nœud de référence, pointeur avant ce nœud]`.
pub(super) fn iter_get(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let id = iterator_id(vm, &arg(args, 0))?;
    let Some(state) = host(vm).and_then(|h| h.doc.iterator(id)) else {
        return Err(vm.error("TypeError", "itérateur inconnu"));
    };
    let reference = wrap(vm, state.reference);
    Ok(vm.new_array(vec![reference, Value::Bool(state.before)]))
}

/// `iterSet(id, nœud, avant)` : déplace l'itérateur.
pub(super) fn iter_set(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let id = iterator_id(vm, &arg(args, 0))?;
    let reference = node_arg(vm, &arg(args, 1))?;
    let before = arg(args, 2).truthy();
    if let Some(h) = host(vm) {
        h.doc.set_iterator(id, reference, before);
    }
    Ok(Value::Undefined)
}
