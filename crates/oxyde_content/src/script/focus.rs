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
pub(crate) fn is_text_input(doc: &Document, node: NodeId) -> bool {
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

/// Champ de texte d'une ligne en cours d'édition : sa valeur, le curseur et
/// l'autre bout de la sélection, sur des frontières de caractères.
struct Field {
    value: String,
    caret: usize,
    anchor: usize,
    /// `readonly` ou `disabled` : la valeur ne se modifie pas.
    locked: bool,
    /// Mot de passe : la valeur ne se copie pas.
    secret: bool,
    /// `maxlength`, en caractères.
    limit: Option<usize>,
}

impl Field {
    fn read(doc: &Document, node: NodeId) -> Option<Self> {
        let element = doc.element(node)?;
        let value = element.attr("value").unwrap_or("").to_owned();
        let clamp = |at: usize| {
            let mut at = at.min(value.len());
            while !value.is_char_boundary(at) {
                at -= 1;
            }
            at
        };
        let (caret, anchor) = (clamp(doc.caret()), clamp(doc.selection_anchor()));
        Some(Self {
            caret,
            anchor,
            locked: element.has_attr("readonly") || element.has_attr("disabled"),
            secret: element
                .attr("type")
                .is_some_and(|kind| kind.eq_ignore_ascii_case("password")),
            limit: element
                .attr("maxlength")
                .and_then(|text| text.trim().parse::<usize>().ok()),
            value,
        })
    }

    /// Début et fin de la sélection.
    fn range(&self) -> (usize, usize) {
        (self.caret.min(self.anchor), self.caret.max(self.anchor))
    }

    /// Texte sélectionné ; `None` s'il n'y en a pas ou s'il est secret.
    fn selected(&self) -> Option<String> {
        let (start, end) = self.range();
        if start == end || self.secret {
            return None;
        }
        self.value.get(start..end).map(str::to_owned)
    }

    /// Remplace la sélection par `text`, coupé à ce que `maxlength` laisse
    /// entrer ; le curseur se place après. Renvoie `true` si la valeur a
    /// changé.
    fn replace_selection(&mut self, text: &str) -> bool {
        if self.locked {
            return false;
        }
        let (start, end) = self.range();
        let kept = self.value.chars().count()
            - self
                .value
                .get(start..end)
                .map_or(0, |cut| cut.chars().count());
        let room = self
            .limit
            .map_or(usize::MAX, |max| max.saturating_sub(kept));
        let inserted: String = text.chars().take(room).collect();
        if start == end && inserted.is_empty() {
            return false;
        }
        self.value.replace_range(start..end, &inserted);
        self.caret = start + inserted.len();
        self.anchor = self.caret;
        true
    }

    /// Écrit la valeur (si `changed`), le curseur et la sélection dans le
    /// document, et prévient les scripts d'une valeur changée.
    fn commit(self, vm: &mut Vm, node: NodeId, changed: bool) {
        let Some(h) = host(vm) else {
            return;
        };
        if changed {
            h.doc.set_attr(node, "value", &self.value);
        }
        h.doc
            .set_field_selection(Some(node), self.anchor, self.caret);
        h.dirty = true;
        if changed {
            dispatch_simple(vm, node, "input", true);
        }
    }
}

/// Frappe dans le champ de texte `node`. Renvoie `true` si la touche a été
/// prise en charge.
///
/// Maj avec une touche de déplacement étend la sélection ; Ctrl A
/// sélectionne tout, Ctrl C copie, Ctrl X coupe (voir `Host::copied`).
fn edit_input(vm: &mut Vm, node: NodeId, input: &crate::KeyInput<'_>) -> bool {
    let Some(h) = host(vm) else {
        return false;
    };
    let Some(mut field) = Field::read(&h.doc, node) else {
        return false;
    };
    let (start, end) = field.range();
    let selected = start != end;
    let caret = field.caret;
    // Frontières des caractères voisins du curseur.
    let previous = field
        .value
        .get(..caret)
        .and_then(|head| head.char_indices().next_back())
        .map_or(0, |(index, _)| index);
    let next = field
        .value
        .get(caret..)
        .and_then(|tail| tail.chars().next())
        .map_or(field.value.len(), |c| caret + c.len_utf8());
    let shortcut = is_shortcut(input);
    let mut changed = false;
    // Vrai si la touche déplace le curseur en gardant l'autre bout.
    let mut extends = false;
    match input.key {
        "ArrowLeft" | "ArrowRight" | "Home" | "End" => {
            let left = input.key == "ArrowLeft";
            field.caret = match input.key {
                "Home" => 0,
                "End" => field.value.len(),
                // Sans Maj, une flèche referme la sélection sur son bord.
                _ if selected && !input.shift => {
                    if left {
                        start
                    } else {
                        end
                    }
                }
                _ if left => previous,
                _ => next,
            };
            extends = input.shift;
        }
        "Backspace" | "Delete" if !field.locked => {
            if !selected {
                field.anchor = if input.key == "Backspace" {
                    previous
                } else {
                    next
                };
            }
            changed = field.replace_selection("");
        }
        key if shortcut => {
            if !input.ctrl || input.alt || input.meta {
                return false;
            }
            match key.to_ascii_lowercase().as_str() {
                "a" => {
                    field.anchor = 0;
                    field.caret = field.value.len();
                    extends = true;
                }
                "c" => {
                    if let Some(text) = field.selected() {
                        h.copied = Some(text);
                    }
                    extends = true;
                }
                "x" => {
                    if let Some(text) = field.selected().filter(|_| !field.locked) {
                        h.copied = Some(text);
                        changed = field.replace_selection("");
                    } else {
                        extends = true;
                    }
                }
                _ => return false,
            }
        }
        key if !field.locked && key.chars().count() == 1 && !key.contains(char::is_control) => {
            changed = field.replace_selection(key);
        }
        _ => return false,
    }
    if !extends {
        field.anchor = field.caret;
    }
    field.commit(vm, node, changed);
    true
}

/// Colle `text` dans le champ qui a le focus, à la place de la sélection.
/// Dans un champ d'une ligne, les sauts de ligne et les caractères de
/// commande n'entrent pas.
///
/// LIMITE : l'événement `paste` n'est pas distribué.
pub(super) fn paste(vm: &mut Vm, text: &str) {
    let Some(h) = host(vm) else {
        return;
    };
    let Some(node) = h.doc.focus() else {
        return;
    };
    if is_text_input(&h.doc, node) {
        let Some(mut field) = Field::read(&h.doc, node) else {
            return;
        };
        let line: String = text.chars().filter(|c| !c.is_control()).collect();
        let changed = field.replace_selection(&line);
        field.commit(vm, node, changed);
    } else if h.doc.element(node).is_some_and(|e| e.is_html("textarea")) {
        element_hook(vm, "pasted", node, &[Value::string(text)]);
        if let Some(h) = host(vm) {
            h.dirty = true;
        }
    }
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
        super::activate(vm, node, None);
    }
}
