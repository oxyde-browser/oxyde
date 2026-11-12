//! Compléments du DOM : opérations de `Node`, `Element`, `CharacterData`,
//! `Text`, `Document` et `DOMTokenList` (DOM Standard, WHATWG).
//!
//! LIMITES CONNUES : `insertAdjacentHTML` analyse son fragment comme le contenu d'un `<body>`.

use oxyde_dom::{Document, NodeData, NodeId};
use oxyde_js::{JsResult, Value, Vm};

use super::{arg, host, node_arg, node_of, number, string, this_node, wrap, wrap_all, wrap_opt};

/// Profondeur maximale explorée par les comparaisons d'arbres.
const MAX_DEPTH: usize = 2_000;

fn dom_error(vm: &mut Vm, name: &str, message: &str) -> Value {
    super::dom_exception(vm, name, message)
}

// ---- Node ----------------------------------------------------------------------------

pub(super) fn has_child_nodes(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    Ok(Value::Bool(
        host(vm).is_some_and(|h| h.doc.node(node).first_child.is_some()),
    ))
}

pub(super) fn is_same_node(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    Ok(Value::Bool(node_of(vm, &arg(args, 0)) == Some(node)))
}

pub(super) fn get_root_node(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let root = host(vm).map(|h| h.doc.ancestors(node).last().unwrap_or(node));
    Ok(wrap_opt(vm, root))
}

/// Fusionne les nœuds texte voisins du sous-arbre et retire ceux qui sont vides.
pub(super) fn normalize(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let Some(h) = host(vm) else {
        return Ok(Value::Undefined);
    };
    let doc = &mut h.doc;
    let parents: Vec<NodeId> = std::iter::once(node).chain(doc.descendants(node)).collect();
    for parent in parents {
        let mut child = doc.node(parent).first_child;
        while let Some(current) = child {
            let next = doc.node(current).next_sibling;
            let NodeData::Text(text) = &doc.node(current).data else {
                child = next;
                continue;
            };
            if text.is_empty() {
                doc.detach(current);
                child = next;
                continue;
            }
            // Un voisin texte est absorbé, puis le même nœud est réexaminé.
            if let Some(sibling) = next
                && let NodeData::Text(more) = &doc.node(sibling).data
            {
                let merged = format!("{text}{more}");
                doc.set_text(current, &merged);
                doc.detach(sibling);
                continue;
            }
            child = next;
        }
    }
    h.dirty = true;
    Ok(Value::Undefined)
}

/// Égalité structurelle de deux nœuds (DOM Standard §4.4, « equals »).
fn nodes_equal(doc: &Document, a: NodeId, b: NodeId, depth: usize) -> bool {
    if depth > MAX_DEPTH {
        return false;
    }
    let same = match (&doc.node(a).data, &doc.node(b).data) {
        (NodeData::Document, NodeData::Document) | (NodeData::Fragment, NodeData::Fragment) => true,
        (
            NodeData::Doctype {
                name: n1,
                public_id: p1,
                system_id: s1,
            },
            NodeData::Doctype {
                name: n2,
                public_id: p2,
                system_id: s2,
            },
        ) => n1 == n2 && p1 == p2 && s1 == s2,
        (NodeData::Text(x), NodeData::Text(y)) | (NodeData::Comment(x), NodeData::Comment(y)) => {
            x == y
        }
        (NodeData::Element(x), NodeData::Element(y)) => {
            x.ns == y.ns
                && x.name == y.name
                && x.attrs.len() == y.attrs.len()
                && x.attrs
                    .iter()
                    .all(|attr| y.attr(&attr.name) == Some(attr.value.as_str()))
        }
        _ => false,
    };
    if !same {
        return false;
    }
    let (left, right): (Vec<NodeId>, Vec<NodeId>) =
        (doc.children(a).collect(), doc.children(b).collect());
    left.len() == right.len()
        && left
            .iter()
            .zip(&right)
            .all(|(x, y)| nodes_equal(doc, *x, *y, depth + 1))
}

pub(super) fn is_equal_node(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let Some(other) = node_of(vm, &arg(args, 0)) else {
        return Ok(Value::Bool(false));
    };
    Ok(Value::Bool(
        host(vm).is_some_and(|h| nodes_equal(&h.doc, node, other, 0)),
    ))
}

const DISCONNECTED: usize = 1;
const PRECEDING: usize = 2;
const FOLLOWING: usize = 4;
const CONTAINS: usize = 8;
const CONTAINED_BY: usize = 16;
const IMPLEMENTATION_SPECIFIC: usize = 32;

pub(super) fn compare_document_position(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let other = node_arg(vm, &arg(args, 0))?;
    let Some(h) = host(vm) else {
        return Ok(number(0));
    };
    let doc = &h.doc;
    if node == other {
        return Ok(number(0));
    }
    let root_of = |n: NodeId| doc.ancestors(n).last().unwrap_or(n);
    let (root, other_root) = (root_of(node), root_of(other));
    if root != other_root {
        // Arbres distincts : l'ordre est arbitraire mais constant.
        let order = if other.index() < node.index() {
            PRECEDING
        } else {
            FOLLOWING
        };
        return Ok(number(DISCONNECTED | IMPLEMENTATION_SPECIFIC | order));
    }
    if doc.ancestors(node).any(|a| a == other) {
        return Ok(number(CONTAINS | PRECEDING));
    }
    if doc.ancestors(other).any(|a| a == node) {
        return Ok(number(CONTAINED_BY | FOLLOWING));
    }
    // Ordre du document : le premier des deux rencontré en parcourant l'arbre.
    let first = doc.descendants(root).find(|n| *n == node || *n == other);
    Ok(number(if first == Some(other) {
        PRECEDING
    } else {
        FOLLOWING
    }))
}

// ---- insertAdjacent… ---------------------------------------------------------------------

/// Insère `nodes` à la position `position` relative à `element`. Renvoie
/// `false` si la position exige un parent que l'élément n'a pas.
fn insert_adjacent(
    vm: &mut Vm,
    element: NodeId,
    position: &str,
    nodes: &[NodeId],
) -> JsResult<bool> {
    let (parent, first, next) = match host(vm) {
        Some(h) => {
            let links = h.doc.node(element);
            (links.parent, links.first_child, links.next_sibling)
        }
        None => return Ok(false),
    };
    match position.to_ascii_lowercase().as_str() {
        "beforebegin" => match parent {
            Some(parent) => super::tree::insert_all(vm, parent, nodes, Some(element))?,
            None => return Ok(false),
        },
        "afterbegin" => super::tree::insert_all(vm, element, nodes, first)?,
        "beforeend" => super::tree::insert_all(vm, element, nodes, None)?,
        "afterend" => match parent {
            Some(parent) => super::tree::insert_all(vm, parent, nodes, next)?,
            None => return Ok(false),
        },
        _ => {
            return Err(dom_error(
                vm,
                "SyntaxError",
                "position attendue : beforebegin, afterbegin, beforeend ou afterend",
            ));
        }
    }
    Ok(true)
}

pub(super) fn insert_adjacent_element(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let element = this_node(vm, this)?;
    let position = vm.to_string(&arg(args, 0))?;
    let node = node_arg(vm, &arg(args, 1))?;
    Ok(if insert_adjacent(vm, element, &position, &[node])? {
        arg(args, 1)
    } else {
        Value::Null
    })
}

pub(super) fn insert_adjacent_text(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let element = this_node(vm, this)?;
    let position = vm.to_string(&arg(args, 0))?;
    let text = vm.to_string(&arg(args, 1))?;
    let Some(node) = host(vm).map(|h| h.doc.create_text(&text)) else {
        return Ok(Value::Undefined);
    };
    insert_adjacent(vm, element, &position, &[node])?;
    Ok(Value::Undefined)
}

pub(super) fn insert_adjacent_html(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let element = this_node(vm, this)?;
    let position = vm.to_string(&arg(args, 0))?;
    let html = vm.to_string(&arg(args, 1))?;
    let Some(h) = host(vm) else {
        return Ok(Value::Undefined);
    };
    let fragment = oxyde_html::parse_document(&format!("<!DOCTYPE html><body>{html}"));
    let mut nodes = Vec::new();
    if let Some(body) = fragment.find_html_element(fragment.root(), "body") {
        let children: Vec<NodeId> = fragment.children(body).collect();
        for child in children {
            nodes.push(h.doc.import(&fragment, child));
        }
    }
    insert_adjacent(vm, element, &position, &nodes)?;
    Ok(Value::Undefined)
}

// ---- CharacterData et Text ---------------------------------------------------------------

/// Texte d'un nœud texte ou commentaire, en unités UTF-16.
fn data_units(vm: &mut Vm, node: NodeId) -> Vec<u16> {
    host(vm)
        .and_then(|h| {
            h.doc
                .character_data(node)
                .map(|t| t.encode_utf16().collect())
        })
        .unwrap_or_default()
}

/// Position entière d'un argument (`unsigned long`).
pub(super) fn offset_arg(vm: &mut Vm, value: &Value) -> JsResult<usize> {
    let n = vm.to_number(value)?;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "ramené dans [0, 2^32 - 1]"
    )]
    Ok(if n.is_finite() {
        n.trunc().rem_euclid(4_294_967_296.0) as usize
    } else {
        0
    })
}

/// `replaceData(offset, count, data)` (DOM Standard §4.10) : la base de
/// toutes les modifications de `CharacterData`.
fn replace_units(
    vm: &mut Vm,
    node: NodeId,
    offset: usize,
    count: usize,
    data: &str,
) -> JsResult<()> {
    let narrow = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    let done = host(vm).is_some_and(|h| {
        h.dirty = true;
        h.doc
            .replace_data(node, narrow(offset), narrow(count), data)
    });
    if !done {
        return Err(dom_error(
            vm,
            "IndexSizeError",
            "la position dépasse la longueur des données",
        ));
    }
    Ok(())
}

pub(super) fn substring_data(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let offset = offset_arg(vm, &arg(args, 0))?;
    let count = offset_arg(vm, &arg(args, 1))?;
    let units = data_units(vm, node);
    if offset > units.len() {
        return Err(dom_error(
            vm,
            "IndexSizeError",
            "la position dépasse la longueur des données",
        ));
    }
    let end = offset.saturating_add(count).min(units.len());
    string(&String::from_utf16_lossy(
        units.get(offset..end).unwrap_or(&[]),
    ))
}

pub(super) fn append_data(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let data = vm.to_string(&arg(args, 0))?;
    let length = data_units(vm, node).len();
    replace_units(vm, node, length, 0, &data)?;
    Ok(Value::Undefined)
}

pub(super) fn insert_data(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let offset = offset_arg(vm, &arg(args, 0))?;
    let data = vm.to_string(&arg(args, 1))?;
    replace_units(vm, node, offset, 0, &data)?;
    Ok(Value::Undefined)
}

pub(super) fn delete_data(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let offset = offset_arg(vm, &arg(args, 0))?;
    let count = offset_arg(vm, &arg(args, 1))?;
    replace_units(vm, node, offset, count, "")?;
    Ok(Value::Undefined)
}

pub(super) fn replace_data(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let offset = offset_arg(vm, &arg(args, 0))?;
    let count = offset_arg(vm, &arg(args, 1))?;
    let data = vm.to_string(&arg(args, 2))?;
    replace_units(vm, node, offset, count, &data)?;
    Ok(Value::Undefined)
}

pub(super) fn split_text(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let offset = u32::try_from(offset_arg(vm, &arg(args, 0))?).unwrap_or(u32::MAX);
    let new = host(vm).and_then(|h| {
        h.dirty = true;
        h.doc.split_text(node, offset)
    });
    match new {
        Some(new) => Ok(wrap(vm, new)),
        None => Err(dom_error(
            vm,
            "IndexSizeError",
            "la position dépasse la longueur du texte",
        )),
    }
}

/// `wholeText` : le texte de ce nœud et de ses voisins texte contigus.
pub(super) fn whole_text(doc: &Document, node: NodeId) -> String {
    let is_text = |n: NodeId| matches!(doc.node(n).data, NodeData::Text(_));
    let mut first = node;
    while let Some(previous) = doc.node(first).prev_sibling.filter(|p| is_text(*p)) {
        first = previous;
    }
    let mut out = String::new();
    let mut current = Some(first);
    while let Some(n) = current.filter(|n| is_text(*n)) {
        out.push_str(doc.text(n).unwrap_or(""));
        current = doc.node(n).next_sibling;
    }
    out
}

// ---- Document ------------------------------------------------------------------------------

pub(super) fn get_elements_by_name(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let scope = this_node(vm, this)?;
    let name = vm.to_string(&arg(args, 0))?;
    let found: Vec<NodeId> = host(vm)
        .map(|h| {
            let doc = &h.doc;
            doc.descendants(scope)
                .filter(|n| {
                    doc.element(*n)
                        .is_some_and(|e| e.attr("name") == Some(&*name))
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(wrap_all(vm, found))
}

pub(super) fn has_focus(_vm: &mut Vm, _this: &Value, _args: &[Value]) -> JsResult {
    Ok(Value::Bool(true))
}

// ---- DOMTokenList ----------------------------------------------------------------------------
