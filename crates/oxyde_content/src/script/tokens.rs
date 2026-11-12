//! `DOMTokenList` (DOM Standard §7.1), pour `classList` : un ensemble
//! ordonné de jetons, reflet de l'attribut `class`.
//!
//! LIMITES CONNUES : seule `classList` existe ; `relList`, `sandbox` et les
//! autres listes de jetons ne sont pas servies.

use oxyde_dom::{Document, NodeId};
use oxyde_js::{JsResult, Value, Vm};

use super::dom_ops::offset_arg;
use super::{CLASS_LIST, arg, dom_exception, host, number, owner_node, string, wrap};

/// Emplacement qui garde l'objet `classList` d'un élément.
const CLASS_LIST_SLOT: &str = "élément.classList";

/// Jetons de l'attribut `class` : séparés par des blancs ASCII, sans doublon.
fn tokens_of(doc: &Document, node: NodeId) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    if let Some(element) = doc.element(node) {
        for token in element.classes() {
            if !tokens.iter().any(|t| t == token) {
                tokens.push(token.to_owned());
            }
        }
    }
    tokens
}

/// « Update steps » : écrit l'ensemble dans l'attribut, sauf s'il est vide
/// et que l'attribut n'existe pas.
fn update(doc: &mut Document, node: NodeId, tokens: &[String]) {
    let absent = doc.element(node).is_none_or(|e| !e.has_attr("class"));
    if absent && tokens.is_empty() {
        return;
    }
    doc.set_attr(node, "class", &tokens.join(" "));
}

fn this_list(vm: &mut Vm, this: &Value) -> JsResult<NodeId> {
    match owner_node(vm, this, CLASS_LIST) {
        Some(node) => Ok(node),
        None => Err(vm.error("TypeError", "receveur attendu : un DOMTokenList")),
    }
}

/// Refuse un jeton vide (`SyntaxError`) ou contenant un blanc
/// (`InvalidCharacterError`).
fn validate(vm: &mut Vm, tokens: &[String]) -> JsResult<()> {
    if tokens.iter().any(String::is_empty) {
        return Err(dom_exception(
            vm,
            "SyntaxError",
            "un jeton ne peut pas être vide",
        ));
    }
    if tokens
        .iter()
        .any(|t| t.chars().any(|c| c.is_ascii_whitespace()))
    {
        return Err(dom_exception(
            vm,
            "InvalidCharacterError",
            "un jeton ne contient pas d'espace",
        ));
    }
    Ok(())
}

fn strings(vm: &mut Vm, args: &[Value]) -> JsResult<Vec<String>> {
    let mut out = Vec::with_capacity(args.len());
    for value in args {
        out.push(vm.to_string(value)?.to_string());
    }
    Ok(out)
}

/// Applique `change` à l'ensemble de jetons de `node`, puis l'écrit.
fn change(vm: &mut Vm, node: NodeId, change: impl FnOnce(&mut Vec<String>)) {
    if let Some(h) = host(vm) {
        let mut tokens = tokens_of(&h.doc, node);
        change(&mut tokens);
        update(&mut h.doc, node, &tokens);
        h.dirty = true;
    }
}

/// `element.classList` : toujours le même objet pour un élément.
pub(super) fn class_list(vm: &mut Vm, element: NodeId) -> Value {
    let wrapper = wrap(vm, element);
    let Some(wrapper_id) = wrapper.as_object() else {
        return Value::Null;
    };
    if let Some(existing) = vm.slot(wrapper_id, CLASS_LIST_SLOT) {
        return existing;
    }
    let Some(proto) = host(vm).map(|h| h.class_list_proto) else {
        return Value::Null;
    };
    let index = u64::try_from(element.index()).unwrap_or(u64::MAX);
    let list = Value::Object(vm.new_host_object(proto, CLASS_LIST, index));
    vm.set_slot(wrapper_id, CLASS_LIST_SLOT, list.clone());
    list
}

pub(super) fn add(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_list(vm, this)?;
    let added = strings(vm, args)?;
    validate(vm, &added)?;
    change(vm, node, |tokens| {
        for token in added {
            if !tokens.contains(&token) {
                tokens.push(token);
            }
        }
    });
    Ok(Value::Undefined)
}

pub(super) fn remove(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_list(vm, this)?;
    let removed = strings(vm, args)?;
    validate(vm, &removed)?;
    change(vm, node, |tokens| tokens.retain(|t| !removed.contains(t)));
    Ok(Value::Undefined)
}

pub(super) fn toggle(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_list(vm, this)?;
    let token = vm.to_string(&arg(args, 0))?.to_string();
    validate(vm, std::slice::from_ref(&token))?;
    let force = args
        .get(1)
        .filter(|v| **v != Value::Undefined)
        .map(Value::truthy);
    let present = host(vm).is_some_and(|h| tokens_of(&h.doc, node).contains(&token));
    match (present, force) {
        (true, None | Some(false)) => {
            change(vm, node, |tokens| tokens.retain(|t| *t != token));
            Ok(Value::Bool(false))
        }
        (true, Some(true)) => Ok(Value::Bool(true)),
        (false, None | Some(true)) => {
            change(vm, node, |tokens| tokens.push(token));
            Ok(Value::Bool(true))
        }
        (false, Some(false)) => Ok(Value::Bool(false)),
    }
}

pub(super) fn replace(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_list(vm, this)?;
    let old = vm.to_string(&arg(args, 0))?.to_string();
    let new = vm.to_string(&arg(args, 1))?.to_string();
    validate(vm, &[old.clone(), new.clone()])?;
    let present = host(vm).is_some_and(|h| tokens_of(&h.doc, node).contains(&old));
    if !present {
        return Ok(Value::Bool(false));
    }
    change(vm, node, |tokens| {
        // Le premier des deux jetons prend la nouvelle valeur, l'autre disparaît.
        if let Some(first) = tokens.iter().position(|t| *t == old || *t == new) {
            tokens[first].clone_from(&new);
            let mut index = 0;
            tokens.retain(|t| {
                let keep = index == first || (*t != old && *t != new);
                index += 1;
                keep
            });
        }
    });
    Ok(Value::Bool(true))
}

pub(super) fn contains(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_list(vm, this)?;
    let token = vm.to_string(&arg(args, 0))?;
    Ok(Value::Bool(host(vm).is_some_and(|h| {
        tokens_of(&h.doc, node).iter().any(|t| **t == *token)
    })))
}

pub(super) fn item(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_list(vm, this)?;
    let index = offset_arg(vm, &arg(args, 0))?;
    let token = host(vm).and_then(|h| tokens_of(&h.doc, node).get(index).cloned());
    Ok(token.map_or(Value::Null, |t| Value::string(&t)))
}

/// `supports(token)` : `classList` ne définit pas de jetons pris en charge.
pub(super) fn supports(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    this_list(vm, this)?;
    Err(vm.error(
        "TypeError",
        "classList n'a pas de jetons pris en charge à vérifier",
    ))
}

pub(super) fn to_string(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let node = this_list(vm, this)?;
    let value = host(vm)
        .and_then(|h| h.doc.element(node)?.attr("class").map(str::to_owned))
        .unwrap_or_default();
    string(&value)
}

/// Propriété `key` de la liste de l'élément `data` : `length`, `value` ou
/// un rang.
pub(super) fn property(vm: &mut Vm, data: u64, key: &str) -> Option<JsResult> {
    let h = host(vm)?;
    let node = h.doc.id_from_index(usize::try_from(data).ok()?)?;
    match key {
        "length" => Some(Ok(number(tokens_of(&h.doc, node).len()))),
        "value" => Some(string(
            h.doc
                .element(node)
                .and_then(|e| e.attr("class"))
                .unwrap_or(""),
        )),
        _ => {
            let index = key.parse::<usize>().ok()?;
            let token = tokens_of(&h.doc, node).get(index).cloned()?;
            Some(string(&token))
        }
    }
}

/// Écriture de `value` : l'attribut `class` prend la chaîne telle quelle.
pub(super) fn set_property(
    vm: &mut Vm,
    data: u64,
    key: &str,
    value: &Value,
) -> Option<JsResult<()>> {
    if key != "value" {
        return None;
    }
    let text = match vm.to_string(value) {
        Ok(text) => text,
        Err(thrown) => return Some(Err(thrown)),
    };
    let h = host(vm)?;
    let node = h.doc.id_from_index(usize::try_from(data).ok()?)?;
    h.doc.set_attr(node, "class", &text);
    h.dirty = true;
    Some(Ok(()))
}

/// Clés propres de la liste : ses rangs.
pub(super) fn keys(vm: &mut Vm, data: u64) -> Vec<String> {
    let count = host(vm)
        .and_then(|h| {
            let node = h.doc.id_from_index(usize::try_from(data).ok()?)?;
            Some(tokens_of(&h.doc, node).len())
        })
        .unwrap_or(0);
    (0..count).map(|index| index.to_string()).collect()
}
