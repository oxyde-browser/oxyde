//! Focus et saisie au clavier dans les contrôles de formulaire.
//!
//! L'élément qui a le focus et la position du curseur vivent dans le
//! document ([`Document::focus`], [`Document::caret`]) : le style (`:focus`)
//! et la mise en page (curseur du champ) les y lisent.
//!
//! LIMITES CONNUES : pas de sélection de texte (le curseur seul) ; la valeur
//! d'un champ vit dans son attribut `value`, donc la frappe se voit aussi
//! par `getAttribute` et par les observateurs de mutation ; dans un
//! `<textarea>`, la frappe se fait en fin de texte ; une liste déroulante ne
//! se déroule pas, les flèches y changent l'option choisie ; `tabindex` rend un
//! élément focalisable mais son ordre n'est pas suivi.

use oxyde_dom::{Document, NodeId};
use oxyde_js::{JsResult, Value, Vm};

use super::events::dispatch_simple;
use super::{host, report, this_node, wrap};

/// Types d'`<input>` qui se saisissent comme du texte.
fn is_text_type(kind: Option<&str>) -> bool {
    !matches!(
        kind.map(str::to_ascii_lowercase).as_deref(),
        Some(
            "checkbox"
                | "radio"
                | "submit"
                | "reset"
                | "button"
                | "image"
                | "file"
                | "hidden"
                | "range"
                | "color"
        )
    )
}

/// Vrai si `node` est un champ de texte d'une ligne.
pub(super) fn is_text_input(doc: &Document, node: NodeId) -> bool {
    doc.element(node)
        .is_some_and(|e| e.is_html("input") && is_text_type(e.attr("type")))
}

/// Vrai si `node` peut recevoir le focus.
pub(super) fn focusable(doc: &Document, node: NodeId) -> bool {
    let Some(element) = doc.element(node) else {
        return false;
    };
    if element.has_attr("disabled") {
        return false;
    }
    match element.name.as_str() {
        "input" => !element
            .attr("type")
            .is_some_and(|kind| kind.eq_ignore_ascii_case("hidden")),
        "select" | "textarea" | "button" => true,
        "a" | "area" => element.has_attr("href"),
        _ => element.has_attr("tabindex") || element.has_attr("contenteditable"),
    }
}

/// Élément qui prend le focus quand on clique sur `node` : lui-même ou son
/// plus proche ancêtre focalisable ; pour une étiquette, son contrôle.
pub(super) fn focus_target(doc: &Document, node: NodeId) -> Option<NodeId> {
    for candidate in std::iter::once(node).chain(doc.ancestors(node)) {
        if focusable(doc, candidate) {
            return Some(candidate);
        }
        let Some(element) = doc.element(candidate) else {
            continue;
        };
        if element.is_html("label") {
            let control = match element.attr("for") {
                Some(id) => doc
                    .descendants(doc.root())
                    .find(|n| doc.element(*n).is_some_and(|e| e.attr("id") == Some(id))),
                None => doc.descendants(candidate).find(|n| {
                    doc.element(*n)
                        .is_some_and(|e| matches!(e.name.as_str(), "input" | "select" | "textarea"))
                }),
            };
            return control.filter(|n| focusable(doc, *n));
        }
    }
    None
}

/// Valeur d'un champ de texte, pour savoir à la perte du focus si elle a changé.
fn text_value(doc: &Document, node: NodeId) -> Option<String> {
    is_text_input(doc, node)
        .then(|| doc.element(node)?.attr("value").map(str::to_owned))
        .flatten()
        .or_else(|| is_text_input(doc, node).then(String::new))
}

/// Donne le focus à `target`, ou le retire (`None`), et distribue les
/// événements : `change` si la valeur du champ quitté a changé, `blur` et
/// `focusout` sur l'ancien élément, `focus` et `focusin` sur le nouveau.
pub(super) fn move_focus(vm: &mut Vm, target: Option<NodeId>) {
    let Some(h) = host(vm) else {
        return;
    };
    let old = h.doc.focus();
    if old == target {
        return;
    }
    let changed = old.is_some_and(|node| text_value(&h.doc, node) != h.focus_value);
    let value = target.and_then(|node| text_value(&h.doc, node));
    // Le curseur se place en fin de valeur.
    let caret = value.as_deref().map_or(0, str::len);
    h.doc.set_focus(target, caret);
    h.focus_value = value;
    h.dirty = true;
    if let Some(old) = old {
        if changed {
            dispatch_simple(vm, old, "change", true);
        }
        dispatch_simple(vm, old, "blur", false);
        dispatch_simple(vm, old, "focusout", true);
    }
    // Un écouteur de `blur` a pu déplacer le focus à son tour.
    if let Some(new) = target
        && host(vm).is_some_and(|h| h.doc.focus() == Some(new))
    {
        dispatch_simple(vm, new, "focus", false);
        dispatch_simple(vm, new, "focusin", true);
    }
}

/// `element.focus()`.
pub(super) fn element_focus(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    if host(vm).is_some_and(|h| focusable(&h.doc, node)) {
        move_focus(vm, Some(node));
    }
    Ok(Value::Undefined)
}

/// `element.blur()`.
pub(super) fn element_blur(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    if host(vm).is_some_and(|h| h.doc.focus() == Some(node)) {
        move_focus(vm, None);
    }
    Ok(Value::Undefined)
}

/// Élément qui a le focus, ou `null` : pour `document.activeElement`.
pub(super) fn focused_element(vm: &mut Vm, _this: &Value, _args: &[Value]) -> JsResult {
    Ok(match host(vm).and_then(|h| h.doc.focus()) {
        Some(node) => wrap(vm, node),
        None => Value::Null,
    })
}

/// Appelle la fonction `name` des interfaces HTML (`elements.js`) sur `node`.
fn element_hook(vm: &mut Vm, name: &str, node: NodeId, args: &[Value]) {
    let Some(hooks) = host(vm).and_then(|h| h.activation.clone()) else {
        return;
    };
    let mut all = vec![wrap(vm, node)];
    all.extend_from_slice(args);
    if let Err(thrown) = vm.get(&hooks, name).and_then(|f| vm.call(&f, &hooks, &all)) {
        report(vm, &thrown);
    }
}

/// Vrai si la touche est un raccourci plutôt qu'une frappe. Ctrl et Alt
/// ensemble forment AltGr sous Windows : c'est une frappe (`@`, `#`, `€`).
fn is_shortcut(input: &crate::KeyInput<'_>) -> bool {
    input.meta || (input.ctrl != input.alt)
}

/// Frappe dans le champ de texte `node`. Renvoie `true` si la touche a été
/// prise en charge.
fn edit_input(vm: &mut Vm, node: NodeId, input: &crate::KeyInput<'_>) -> bool {
    let Some(h) = host(vm) else {
        return false;
    };
    let Some(element) = h.doc.element(node) else {
        return false;
    };
    let locked = element.has_attr("readonly") || element.has_attr("disabled");
    let limit = element
        .attr("maxlength")
        .and_then(|text| text.trim().parse::<usize>().ok());
    let mut value = element.attr("value").unwrap_or("").to_owned();
    let mut caret = h.doc.caret().min(value.len());
    while !value.is_char_boundary(caret) {
        caret -= 1;
    }
    // Frontières des caractères voisins du curseur.
    let previous = value
        .get(..caret)
        .and_then(|head| head.char_indices().next_back())
        .map(|(index, _)| index);
    let next = value
        .get(caret..)
        .and_then(|tail| tail.chars().next())
        .map(|c| caret + c.len_utf8());
    let shortcut = is_shortcut(input);
    let mut changed = false;
    match input.key {
        "ArrowLeft" => caret = previous.unwrap_or(0),
        "ArrowRight" => caret = next.unwrap_or(value.len()),
        "Home" => caret = 0,
        "End" => caret = value.len(),
        "Backspace" if !locked => {
            if let Some(start) = previous {
                value.replace_range(start..caret, "");
                caret = start;
                changed = true;
            }
        }
        "Delete" if !locked => {
            if let Some(end) = next {
                value.replace_range(caret..end, "");
                changed = true;
            }
        }
        key if !shortcut
            && !locked
            && key.chars().count() == 1
            && !key.contains(char::is_control) =>
        {
            if limit.is_none_or(|max| value.chars().count() < max) {
                value.insert_str(caret, key);
                caret += key.len();
                changed = true;
            }
        }
        _ => return false,
    }
    if changed {
        h.doc.set_attr(node, "value", &value);
    }
    h.doc.set_focus(Some(node), caret);
    h.dirty = true;
    if changed {
        dispatch_simple(vm, node, "input", true);
    }
    true
}

/// Élément focalisable suivant (ou précédent) dans l'ordre du document, en
/// bouclant.
fn next_focusable(doc: &Document, from: Option<NodeId>, backward: bool) -> Option<NodeId> {
    let all: Vec<NodeId> = doc
        .descendants(doc.root())
        .filter(|node| focusable(doc, *node))
        .collect();
    if all.is_empty() {
        return None;
    }
    let position = from.and_then(|node| all.iter().position(|n| *n == node));
    let index = match (position, backward) {
        (Some(index), false) => (index + 1) % all.len(),
        (Some(index), true) => (index + all.len() - 1) % all.len(),
        (None, false) => 0,
        (None, true) => all.len() - 1,
    };
    all.get(index).copied()
}

/// Action par défaut d'une touche enfoncée, quand aucun script ne l'a
/// annulée : frappe dans le champ qui a le focus, Tab, Entrée, Espace.
pub(super) fn default_key(vm: &mut Vm, input: &crate::KeyInput<'_>) {
    let Some(h) = host(vm) else {
        return;
    };
    let focus = h.doc.focus();
    if input.key == "Tab" && !input.ctrl && !input.alt && !input.meta {
        let next = next_focusable(&h.doc, focus, input.shift);
        move_focus(vm, next);
        return;
    }
    let Some(node) = focus else {
        return;
    };
    let Some(element) = h.doc.element(node) else {
        return;
    };
    let name = element.name.clone();
    let text_input = is_text_input(&h.doc, node);
    let activates = match name.as_str() {
        "button" => matches!(input.key, "Enter" | " "),
        "a" => input.key == "Enter",
        "input" => !text_input && matches!(input.key, "Enter" | " "),
        _ => false,
    };
    if text_input {
        if input.key == "Enter" {
            // Soumission implicite du formulaire.
            element_hook(vm, "enter", node, &[]);
        } else {
            edit_input(vm, node, input);
        }
    } else if name == "select" {
        // Les flèches changent l'option choisie, sans dérouler la liste.
        let step = match input.key {
            "ArrowDown" | "ArrowRight" => 1.0,
            "ArrowUp" | "ArrowLeft" => -1.0,
            _ => return,
        };
        element_hook(vm, "stepped", node, &[Value::Number(step)]);
        if let Some(h) = host(vm) {
            h.dirty = true;
        }
    } else if name == "textarea" {
        if !is_shortcut(input) {
            element_hook(vm, "typed", node, &[Value::string(input.key)]);
            if let Some(h) = host(vm) {
                h.dirty = true;
            }
        }
    } else if activates {
        super::activate(vm, node);
    }
}
