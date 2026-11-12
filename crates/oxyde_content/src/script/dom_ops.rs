//! Compléments du DOM : opérations de `Node`, `Element`, `CharacterData`,
//! `Text`, `Document` et `DOMTokenList` (DOM Standard, WHATWG).
//!
//! LIMITES CONNUES : les erreurs du DOM sont des `Error` dont le message
//! commence par le nom de l'exception (`NotFoundError : …`), pas encore des
//! `DOMException` ; les espaces de noms des attributs sont ignorés ;
//! `insertAdjacentHTML` analyse son fragment comme le contenu d'un `<body>`.

use oxyde_dom::{Document, Namespace, NodeData, NodeId};
use oxyde_js::{JsResult, Value, Vm};

use super::{
    CLASS_LIST, arg, check_insertion, class_tokens, host, node_arg, node_of, number, owner_node,
    string, this_node, wrap, wrap_all, wrap_opt,
};

/// Profondeur maximale explorée par les comparaisons d'arbres.
const MAX_DEPTH: usize = 2_000;

fn dom_error(vm: &mut Vm, name: &str, message: &str) -> Value {
    vm.error("Error", &format!("{name} : {message}"))
}

/// Marque le document comme modifié : la mise en page est à refaire.
fn touch(vm: &mut Vm) {
    if let Some(h) = host(vm) {
        h.dirty = true;
    }
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

pub(super) fn replace_child(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let parent = this_node(vm, this)?;
    let new = node_arg(vm, &arg(args, 0))?;
    let old = node_arg(vm, &arg(args, 1))?;
    let is_child = host(vm).is_some_and(|h| h.doc.node(old).parent == Some(parent));
    if !is_child {
        return Err(dom_error(
            vm,
            "NotFoundError",
            "le nœud à remplacer n'est pas un enfant de ce parent",
        ));
    }
    check_insertion(vm, parent, new)?;
    if new != old
        && let Some(h) = host(vm)
    {
        h.doc.insert_before(parent, new, Some(old));
        h.doc.detach(old);
        h.dirty = true;
    }
    Ok(arg(args, 1))
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
        (NodeData::Document, NodeData::Document) => true,
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

// ---- ChildNode et ParentNode -----------------------------------------------------------

/// Nœuds désignés par des arguments « nœud ou chaîne » : une chaîne devient
/// un nœud texte.
fn nodes_from(vm: &mut Vm, args: &[Value]) -> JsResult<Vec<NodeId>> {
    let mut nodes = Vec::with_capacity(args.len());
    for value in args {
        let node = match node_of(vm, value) {
            Some(node) => node,
            None => {
                let text = vm.to_string(value)?;
                match host(vm) {
                    Some(h) => h.doc.create_text(&text),
                    None => continue,
                }
            }
        };
        nodes.push(node);
    }
    Ok(nodes)
}

/// Insère `nodes` dans `parent`, avant `reference` (à la fin si `None`).
fn insert_all(
    vm: &mut Vm,
    parent: NodeId,
    nodes: &[NodeId],
    reference: Option<NodeId>,
) -> JsResult<()> {
    for node in nodes {
        check_insertion(vm, parent, *node)?;
    }
    if let Some(h) = host(vm) {
        for node in nodes {
            h.doc.insert_before(parent, *node, reference);
        }
        h.dirty = true;
    }
    Ok(())
}

/// Premier frère de `node` dans le sens `forward` qui n'est pas dans `nodes`.
fn viable_sibling(vm: &mut Vm, node: NodeId, nodes: &[NodeId], forward: bool) -> Option<NodeId> {
    let doc = &host(vm)?.doc;
    let step = |n: NodeId| {
        if forward {
            doc.node(n).next_sibling
        } else {
            doc.node(n).prev_sibling
        }
    };
    let mut current = step(node);
    while let Some(candidate) = current {
        if !nodes.contains(&candidate) {
            return Some(candidate);
        }
        current = step(candidate);
    }
    None
}

pub(super) fn before(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let Some(parent) = host(vm).and_then(|h| h.doc.node(node).parent) else {
        return Ok(Value::Undefined);
    };
    let nodes = nodes_from(vm, args)?;
    // Repère : après le frère précédent qui ne bouge pas.
    let reference = match viable_sibling(vm, node, &nodes, false) {
        Some(previous) => host(vm).and_then(|h| h.doc.node(previous).next_sibling),
        None => host(vm).and_then(|h| h.doc.node(parent).first_child),
    };
    insert_all(vm, parent, &nodes, reference)?;
    Ok(Value::Undefined)
}

pub(super) fn after(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let Some(parent) = host(vm).and_then(|h| h.doc.node(node).parent) else {
        return Ok(Value::Undefined);
    };
    let nodes = nodes_from(vm, args)?;
    let reference = viable_sibling(vm, node, &nodes, true);
    insert_all(vm, parent, &nodes, reference)?;
    Ok(Value::Undefined)
}

pub(super) fn replace_with(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let Some(parent) = host(vm).and_then(|h| h.doc.node(node).parent) else {
        return Ok(Value::Undefined);
    };
    let nodes = nodes_from(vm, args)?;
    let reference = viable_sibling(vm, node, &nodes, true);
    for candidate in &nodes {
        check_insertion(vm, parent, *candidate)?;
    }
    if !nodes.contains(&node)
        && let Some(h) = host(vm)
    {
        h.doc.detach(node);
    }
    insert_all(vm, parent, &nodes, reference)?;
    touch(vm);
    Ok(Value::Undefined)
}

pub(super) fn replace_children(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let parent = this_node(vm, this)?;
    let nodes = nodes_from(vm, args)?;
    for candidate in &nodes {
        check_insertion(vm, parent, *candidate)?;
    }
    if let Some(h) = host(vm) {
        // Les nouveaux enfants sont d'abord détachés : certains peuvent être
        // des enfants actuels.
        for node in &nodes {
            h.doc.detach(*node);
        }
        h.doc.remove_children(parent);
    }
    insert_all(vm, parent, &nodes, None)?;
    touch(vm);
    Ok(Value::Undefined)
}

// ---- Attributs d'Element ---------------------------------------------------------------

pub(super) fn has_attributes(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    Ok(Value::Bool(host(vm).is_some_and(|h| {
        h.doc.element(node).is_some_and(|e| !e.attrs.is_empty())
    })))
}

pub(super) fn get_attribute_names(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let names: Vec<Value> = host(vm)
        .and_then(|h| h.doc.element(node))
        .map(|e| e.attrs.iter().map(|a| Value::string(&a.name)).collect())
        .unwrap_or_default();
    Ok(vm.new_array(names))
}

pub(super) fn toggle_attribute(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let name = vm.to_string(&arg(args, 0))?.to_ascii_lowercase();
    let force = args
        .get(1)
        .filter(|v| **v != Value::Undefined)
        .map(Value::truthy);
    let Some(h) = host(vm) else {
        return Ok(Value::Bool(false));
    };
    let present = h.doc.element(node).is_some_and(|e| e.has_attr(&name));
    let wanted = force.unwrap_or(!present);
    if wanted && !present {
        h.doc.set_attr(node, &name, "");
    } else if !wanted && present {
        h.doc.remove_attr(node, &name);
    }
    h.dirty = true;
    Ok(Value::Bool(wanted))
}

/// `getAttributeNS`, `hasAttributeNS`… : l'espace de noms (premier argument)
/// est ignoré, le nom local désigne l'attribut.
pub(super) fn get_attribute_ns(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let name = vm.to_string(&arg(args, 1))?;
    let value = host(vm)
        .and_then(|h| h.doc.element(node))
        .and_then(|e| e.attr(&name).map(str::to_owned));
    Ok(value.map_or(Value::Null, |v| Value::string(&v)))
}

pub(super) fn has_attribute_ns(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let name = vm.to_string(&arg(args, 1))?;
    Ok(Value::Bool(host(vm).is_some_and(|h| {
        h.doc.element(node).is_some_and(|e| e.has_attr(&name))
    })))
}

pub(super) fn set_attribute_ns(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let qualified = vm.to_string(&arg(args, 1))?;
    let value = vm.to_string(&arg(args, 2))?;
    // `préfixe:nom` : seul le nom local est gardé.
    let local = qualified
        .rsplit(':')
        .next()
        .unwrap_or(&qualified)
        .to_owned();
    if let Some(h) = host(vm) {
        h.doc.set_attr(node, &local, &value);
        h.dirty = true;
    }
    Ok(Value::Undefined)
}

pub(super) fn remove_attribute_ns(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let name = vm.to_string(&arg(args, 1))?;
    if let Some(h) = host(vm) {
        h.doc.remove_attr(node, &name);
        h.dirty = true;
    }
    Ok(Value::Undefined)
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
            Some(parent) => insert_all(vm, parent, nodes, Some(element))?,
            None => return Ok(false),
        },
        "afterbegin" => insert_all(vm, element, nodes, first)?,
        "beforeend" => insert_all(vm, element, nodes, None)?,
        "afterend" => match parent {
            Some(parent) => insert_all(vm, parent, nodes, next)?,
            None => return Ok(false),
        },
        _ => {
            return Err(vm.error(
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
        .and_then(|h| h.doc.text(node).map(|t| t.encode_utf16().collect()))
        .unwrap_or_default()
}

fn set_data_units(vm: &mut Vm, node: NodeId, units: &[u16]) {
    if let Some(h) = host(vm) {
        h.doc.set_text(node, &String::from_utf16_lossy(units));
        h.dirty = true;
    }
}

/// Position entière d'un argument (`unsigned long`).
fn offset_arg(vm: &mut Vm, value: &Value) -> JsResult<usize> {
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
    let mut units = data_units(vm, node);
    if offset > units.len() {
        return Err(dom_error(
            vm,
            "IndexSizeError",
            "la position dépasse la longueur des données",
        ));
    }
    let end = offset.saturating_add(count).min(units.len());
    units.splice(offset..end, data.encode_utf16());
    set_data_units(vm, node, &units);
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
    let offset = offset_arg(vm, &arg(args, 0))?;
    let units = data_units(vm, node);
    if offset > units.len() {
        return Err(dom_error(
            vm,
            "IndexSizeError",
            "la position dépasse la longueur du texte",
        ));
    }
    let (head, tail) = units.split_at(offset);
    let Some(h) = host(vm) else {
        return Ok(Value::Null);
    };
    let new = h.doc.create_text(&String::from_utf16_lossy(tail));
    let (parent, next) = {
        let links = h.doc.node(node);
        (links.parent, links.next_sibling)
    };
    if let Some(parent) = parent {
        h.doc.insert_before(parent, new, next);
    }
    h.doc.set_text(node, &String::from_utf16_lossy(head));
    h.dirty = true;
    Ok(wrap(vm, new))
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

pub(super) fn create_comment(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let text = vm.to_string(&arg(args, 0))?;
    let node = host(vm).map(|h| h.doc.create_comment(&text));
    Ok(wrap_opt(vm, node))
}

pub(super) fn create_element_ns(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let namespace = match arg(args, 0) {
        Value::Null | Value::Undefined => String::new(),
        other => vm.to_string(&other)?.to_string(),
    };
    let qualified = vm.to_string(&arg(args, 1))?;
    let local = qualified
        .rsplit(':')
        .next()
        .unwrap_or(&qualified)
        .to_owned();
    let valid = local
        .chars()
        .next()
        .is_some_and(|c| c.is_alphabetic() || c == '_')
        && !local.chars().any(char::is_whitespace);
    if !valid {
        return Err(dom_error(
            vm,
            "InvalidCharacterError",
            "nom d'élément invalide",
        ));
    }
    let ns = match namespace.as_str() {
        "http://www.w3.org/2000/svg" => Namespace::Svg,
        "http://www.w3.org/1998/Math/MathML" => Namespace::MathMl,
        _ => Namespace::Html,
    };
    let node = host(vm).map(|h| h.doc.create_element(&local, ns, Vec::new()));
    Ok(wrap_opt(vm, node))
}

/// `importNode(node, deep)` : une copie de `node` (le document est le même).
pub(super) fn import_node(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let node = node_arg(vm, &arg(args, 0))?;
    let deep = arg(args, 1).truthy();
    let copy = host(vm).map(|h| {
        let source = h.doc.clone();
        let copy = h.doc.import(&source, node);
        if !deep {
            h.doc.remove_children(copy);
        }
        copy
    });
    Ok(wrap_opt(vm, copy))
}

/// `adoptNode(node)` : détache le nœud de son parent et le rend.
pub(super) fn adopt_node(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let node = node_arg(vm, &arg(args, 0))?;
    if let Some(h) = host(vm) {
        if node == h.doc.root() {
            return Err(dom_error(
                vm,
                "NotSupportedError",
                "un document ne peut pas être adopté",
            ));
        }
        h.doc.detach(node);
        h.dirty = true;
    }
    Ok(arg(args, 0))
}

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

pub(super) fn class_list_item(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let index = offset_arg(vm, &arg(args, 0))?;
    let Some(node) = owner_node(vm, this, CLASS_LIST) else {
        return Ok(Value::Null);
    };
    let token = host(vm).and_then(|h| class_tokens(&h.doc, node).get(index).cloned());
    Ok(token.map_or(Value::Null, |t| Value::string(&t)))
}

pub(super) fn class_list_replace(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let old = vm.to_string(&arg(args, 0))?;
    let new = vm.to_string(&arg(args, 1))?;
    for token in [&old, &new] {
        if token.is_empty() {
            return Err(vm.error("SyntaxError", "un jeton de classe ne peut pas être vide"));
        }
        if token.chars().any(char::is_whitespace) {
            return Err(dom_error(
                vm,
                "InvalidCharacterError",
                "un jeton de classe ne contient pas d'espace",
            ));
        }
    }
    let Some(node) = owner_node(vm, this, CLASS_LIST) else {
        return Ok(Value::Bool(false));
    };
    let Some(h) = host(vm) else {
        return Ok(Value::Bool(false));
    };
    let mut tokens = class_tokens(&h.doc, node);
    let Some(position) = tokens.iter().position(|t| **t == *old) else {
        return Ok(Value::Bool(false));
    };
    if tokens.iter().any(|t| **t == *new) {
        // Le nouveau jeton existe déjà : l'ancien disparaît simplement.
        if *old != *new {
            tokens.remove(position);
        }
    } else if let Some(slot) = tokens.get_mut(position) {
        *slot = new.to_string();
    }
    h.doc.set_attr(node, "class", &tokens.join(" "));
    h.dirty = true;
    Ok(Value::Bool(true))
}

/// `supports(token)` : `classList` n'a pas de liste de jetons pris en charge.
pub(super) fn class_list_supports(vm: &mut Vm, _this: &Value, _args: &[Value]) -> JsResult {
    Err(vm.error(
        "TypeError",
        "classList n'a pas de jetons pris en charge à vérifier",
    ))
}
