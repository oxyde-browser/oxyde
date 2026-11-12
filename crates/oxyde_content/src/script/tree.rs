//! Algorithmes de mutation de l'arbre (DOM Standard §4.2.3) : validité avant
//! insertion, « pre-insert », « replace », « replace all », « pre-remove »,
//! et les méthodes de `Node`, `ParentNode` et `ChildNode` bâties dessus.
//!
//! Les refus sont des `DOMException` (`HierarchyRequestError`,
//! `NotFoundError`), et les changements d'une même opération forment un seul
//! `MutationRecord` comme le veut le standard.

use oxyde_dom::{Document, NodeData, NodeId};
use oxyde_js::{JsResult, Value, Vm};

use super::{arg, dom_exception, host, node_arg, node_of, this_node};

/// Refus d'une mutation : nom de la `DOMException` et message.
pub(super) type Refusal = (&'static str, &'static str);

const HIERARCHY: &str = "HierarchyRequestError";

fn is_element(doc: &Document, node: NodeId) -> bool {
    matches!(doc.node(node).data, NodeData::Element(_))
}

fn is_doctype(doc: &Document, node: NodeId) -> bool {
    matches!(doc.node(node).data, NodeData::Doctype { .. })
}

fn is_text(doc: &Document, node: NodeId) -> bool {
    matches!(doc.node(node).data, NodeData::Text(_) | NodeData::CData(_))
}

fn is_fragment(doc: &Document, node: NodeId) -> bool {
    matches!(doc.node(node).data, NodeData::Fragment)
}

/// « Ensure pre-insertion validity » de `node` dans `parent` avant `child`.
/// Avec `replacing`, `child` est le nœud que `node` remplace (algorithme
/// « replace », dont les règles pour un document diffèrent un peu).
pub(super) fn validity(
    doc: &Document,
    node: NodeId,
    parent: NodeId,
    child: Option<NodeId>,
    replacing: bool,
) -> Result<(), Refusal> {
    if !matches!(
        doc.node(parent).data,
        NodeData::Document | NodeData::Fragment | NodeData::Element(_)
    ) {
        return Err((HIERARCHY, "ce nœud ne peut pas avoir d'enfants"));
    }
    if doc.contains(node, parent) {
        return Err((
            HIERARCHY,
            "un nœud ne s'insère pas dans son propre sous-arbre",
        ));
    }
    if let Some(c) = child
        && doc.node(c).parent != Some(parent)
    {
        return Err((
            "NotFoundError",
            "le nœud de référence n'est pas un enfant de ce parent",
        ));
    }
    let to_document = doc.is_document(parent);
    match &doc.node(node).data {
        NodeData::Document | NodeData::Attr(_) => {
            return Err((HIERARCHY, "ce nœud ne s'insère pas dans un arbre"));
        }
        NodeData::Text(_) | NodeData::CData(_) if to_document => {
            return Err((HIERARCHY, "un document n'a pas de texte pour enfant"));
        }
        NodeData::Doctype { .. } if !to_document => {
            return Err((HIERARCHY, "un DOCTYPE n'est l'enfant que d'un document"));
        }
        _ => {}
    }
    if !to_document {
        return Ok(());
    }
    // Enfants du document, hors celui qu'on remplace.
    let others = || {
        doc.children(parent)
            .filter(move |c| !(replacing && Some(*c) == child))
    };
    let element_exists = others().any(|c| is_element(doc, c));
    let doctype_exists = others().any(|c| is_doctype(doc, c));
    let doctype_follows = child.is_some_and(|c| {
        std::iter::successors(doc.node(c).next_sibling, |n| doc.node(*n).next_sibling)
            .any(|n| is_doctype(doc, n))
    });
    let element_precedes = child.is_some_and(|c| {
        std::iter::successors(doc.node(c).prev_sibling, |n| doc.node(*n).prev_sibling)
            .any(|n| is_element(doc, n))
    });
    let before_doctype = !replacing && child.is_some_and(|c| is_doctype(doc, c));
    let refused = Err((
        HIERARCHY,
        "un document n'a qu'un DOCTYPE, suivi d'un seul élément",
    ));
    match &doc.node(node).data {
        NodeData::Fragment => {
            let elements = doc.children(node).filter(|c| is_element(doc, *c)).count();
            if elements > 1
                || doc.children(node).any(|c| is_text(doc, c))
                || (elements == 1 && (element_exists || before_doctype || doctype_follows))
            {
                refused
            } else {
                Ok(())
            }
        }
        NodeData::Element(_) if element_exists || before_doctype || doctype_follows => refused,
        NodeData::Doctype { .. }
            if doctype_exists
                || element_precedes
                || (!replacing && child.is_none() && element_exists) =>
        {
            refused
        }
        _ => Ok(()),
    }
}

/// Applique `check` au document et lève la `DOMException` du refus.
fn ensure(vm: &mut Vm, check: impl FnOnce(&Document) -> Result<(), Refusal>) -> JsResult<()> {
    let verdict = match host(vm) {
        Some(h) => check(&h.doc),
        None => Ok(()),
    };
    match verdict {
        Ok(()) => Ok(()),
        Err((name, message)) => Err(dom_exception(vm, name, message)),
    }
}

/// Insère `node` dans `parent` avant `reference`, sans contrôle. Les enfants
/// d'un fragment arrivent ensemble : un seul enregistrement de mutation.
pub(super) fn insert(doc: &mut Document, node: NodeId, parent: NodeId, reference: Option<NodeId>) {
    if is_fragment(doc, node) {
        let opened = doc.begin_group();
        doc.insert_before(parent, node, reference);
        if opened {
            doc.end_group();
        }
    } else {
        doc.insert_before(parent, node, reference);
    }
}

/// « Pre-insert » : insère `node` dans `parent` avant `child` (à la fin si
/// `None`) après contrôle de validité.
pub(super) fn pre_insert(
    vm: &mut Vm,
    node: NodeId,
    parent: NodeId,
    child: Option<NodeId>,
) -> JsResult<()> {
    ensure(vm, |doc| validity(doc, node, parent, child, false))?;
    if let Some(h) = host(vm) {
        let reference = if child == Some(node) {
            h.doc.node(node).next_sibling
        } else {
            child
        };
        insert(&mut h.doc, node, parent, reference);
        h.dirty = true;
    }
    Ok(())
}

/// « Replace » : met `node` à la place de `child` dans `parent`.
fn replace(vm: &mut Vm, child: NodeId, node: NodeId, parent: NodeId) -> JsResult<()> {
    ensure(vm, |doc| validity(doc, node, parent, Some(child), true))?;
    let Some(h) = host(vm) else { return Ok(()) };
    let doc = &mut h.doc;
    let mut reference = doc.node(child).next_sibling;
    if reference == Some(node) {
        reference = doc.node(node).next_sibling;
    }
    // Le retrait de `node` de son ancien parent a son propre enregistrement.
    if !is_fragment(doc, node) {
        doc.detach(node);
    }
    let opened = doc.begin_group();
    if child != node {
        doc.detach(child);
    }
    doc.insert_before(parent, node, reference);
    if opened {
        doc.end_group();
    }
    h.dirty = true;
    Ok(())
}

/// « Replace all » : `node` (s'il y en a un) devient le seul contenu de
/// `parent`. Aucun contrôle : l'appelant l'a fait.
pub(super) fn replace_all(doc: &mut Document, node: Option<NodeId>, parent: NodeId) {
    if let Some(node) = node
        && !is_fragment(doc, node)
        && doc.node(node).parent != Some(parent)
    {
        doc.detach(node);
    }
    let opened = doc.begin_group();
    doc.remove_children(parent);
    if let Some(node) = node {
        doc.insert_before(parent, node, None);
    }
    if opened {
        doc.end_group();
    }
}

/// Document auquel appartient `node` (lui-même si c'est un document).
fn document_of(doc: &Document, node: NodeId) -> NodeId {
    if doc.is_document(node) {
        node
    } else {
        doc.owner(node)
    }
}

/// « Convert nodes into a node » : un seul nœud est rendu tel quel, sinon
/// tous sont rangés dans un fragment neuf ; une chaîne devient un texte.
fn convert(vm: &mut Vm, context: NodeId, args: &[Value]) -> JsResult<NodeId> {
    let mut nodes = Vec::with_capacity(args.len());
    for value in args {
        let node = match node_of(vm, value) {
            Some(node) => node,
            None => {
                let text = vm.to_string(value)?;
                let Some(h) = host(vm) else {
                    return Err(vm.error("TypeError", "document indisponible"));
                };
                let owner = document_of(&h.doc, context);
                let node = h.doc.create_text(&text);
                h.doc.set_owner(node, owner);
                node
            }
        };
        nodes.push(node);
    }
    if let [single] = nodes.as_slice() {
        return Ok(*single);
    }
    let Some(h) = host(vm) else {
        return Err(vm.error("TypeError", "document indisponible"));
    };
    let owner = document_of(&h.doc, context);
    let fragment = h.doc.create_fragment();
    h.doc.set_owner(fragment, owner);
    for node in nodes {
        h.doc.append_child(fragment, node);
    }
    Ok(fragment)
}

/// Argument `Node?` obligatoire : absent, c'est une erreur de type.
fn nullable_node(vm: &mut Vm, args: &[Value], index: usize) -> JsResult<Option<NodeId>> {
    match args.get(index) {
        None => Err(vm.error("TypeError", "il manque un argument")),
        Some(Value::Null | Value::Undefined) => Ok(None),
        Some(value) => node_arg(vm, value).map(Some),
    }
}

// ---- Node ------------------------------------------------------------------------------

pub(super) fn append_child(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let parent = this_node(vm, this)?;
    let node = node_arg(vm, &arg(args, 0))?;
    pre_insert(vm, node, parent, None)?;
    Ok(arg(args, 0))
}

pub(super) fn insert_before(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let parent = this_node(vm, this)?;
    let node = node_arg(vm, &arg(args, 0))?;
    let child = nullable_node(vm, args, 1)?;
    pre_insert(vm, node, parent, child)?;
    Ok(arg(args, 0))
}

pub(super) fn replace_child(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let parent = this_node(vm, this)?;
    let node = node_arg(vm, &arg(args, 0))?;
    let child = node_arg(vm, &arg(args, 1))?;
    replace(vm, child, node, parent)?;
    Ok(arg(args, 1))
}

pub(super) fn remove_child(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let parent = this_node(vm, this)?;
    let child = node_arg(vm, &arg(args, 0))?;
    let is_child = host(vm).is_some_and(|h| h.doc.node(child).parent == Some(parent));
    if !is_child {
        return Err(dom_exception(
            vm,
            "NotFoundError",
            "le nœud n'est pas un enfant de ce parent",
        ));
    }
    if let Some(h) = host(vm) {
        h.doc.detach(child);
        h.dirty = true;
    }
    Ok(arg(args, 0))
}

/// `moveBefore(node, child)` (DOM Standard, « move ») : déplace `node` dans
/// ce parent sans le faire quitter le document.
pub(super) fn move_before(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let parent = this_node(vm, this)?;
    let node = node_arg(vm, &arg(args, 0))?;
    let child = nullable_node(vm, args, 1)?;
    ensure(vm, |doc| {
        let root = |n: NodeId| doc.ancestors(n).last().unwrap_or(n);
        // « Ensure pre-move validity ».
        if !matches!(
            doc.node(parent).data,
            NodeData::Document | NodeData::Fragment | NodeData::Element(_)
        ) || root(parent) != root(node)
            || doc.contains(node, parent)
        {
            return Err((HIERARCHY, "ce nœud ne peut pas être déplacé ici"));
        }
        if let Some(c) = child
            && doc.node(c).parent != Some(parent)
        {
            return Err((
                "NotFoundError",
                "le nœud de référence n'est pas un enfant de ce parent",
            ));
        }
        let movable = matches!(doc.node(node).data, NodeData::Element(_))
            || (doc.character_data(node).is_some() && !doc.is_document(parent));
        if !movable || (is_text(doc, node) && doc.is_document(parent)) {
            return Err((HIERARCHY, "seuls un élément ou un texte se déplacent"));
        }
        if doc.is_document(parent) && doc.node(node).parent != Some(parent) {
            return validity(doc, node, parent, child, false);
        }
        Ok(())
    })?;
    if let Some(h) = host(vm) {
        let reference = if child == Some(node) {
            h.doc.node(node).next_sibling
        } else {
            child
        };
        h.doc.insert_before(parent, node, reference);
        h.dirty = true;
    }
    Ok(Value::Undefined)
}

// ---- ChildNode et ParentNode -----------------------------------------------------------

pub(super) fn remove(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    if let Some(h) = host(vm) {
        h.doc.detach(node);
        h.dirty = true;
    }
    Ok(Value::Undefined)
}

pub(super) fn append(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let parent = this_node(vm, this)?;
    let node = convert(vm, parent, args)?;
    pre_insert(vm, node, parent, None)?;
    Ok(Value::Undefined)
}

pub(super) fn prepend(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let parent = this_node(vm, this)?;
    let node = convert(vm, parent, args)?;
    let first = host(vm).and_then(|h| h.doc.node(parent).first_child);
    pre_insert(vm, node, parent, first)?;
    Ok(Value::Undefined)
}

pub(super) fn replace_children(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let parent = this_node(vm, this)?;
    let node = convert(vm, parent, args)?;
    ensure(vm, |doc| validity(doc, node, parent, None, false))?;
    if let Some(h) = host(vm) {
        replace_all(&mut h.doc, Some(node), parent);
        h.dirty = true;
    }
    Ok(Value::Undefined)
}

/// Premier frère de `node` dans le sens `forward` qui n'est pas un argument.
fn viable_sibling(vm: &mut Vm, node: NodeId, args: &[Value], forward: bool) -> Option<NodeId> {
    let moved: Vec<NodeId> = args.iter().filter_map(|v| node_of(vm, v)).collect();
    let doc = &host(vm)?.doc;
    let step = |n: NodeId| {
        if forward {
            doc.node(n).next_sibling
        } else {
            doc.node(n).prev_sibling
        }
    };
    std::iter::successors(step(node), |n| step(*n)).find(|n| !moved.contains(n))
}

pub(super) fn before(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let Some(parent) = host(vm).and_then(|h| h.doc.node(node).parent) else {
        return Ok(Value::Undefined);
    };
    let previous = viable_sibling(vm, node, args, false);
    let inserted = convert(vm, parent, args)?;
    // Repère : après le frère précédent qui ne bouge pas.
    let reference = match previous {
        Some(previous) => host(vm).and_then(|h| h.doc.node(previous).next_sibling),
        None => host(vm).and_then(|h| h.doc.node(parent).first_child),
    };
    pre_insert(vm, inserted, parent, reference)?;
    Ok(Value::Undefined)
}

pub(super) fn after(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let Some(parent) = host(vm).and_then(|h| h.doc.node(node).parent) else {
        return Ok(Value::Undefined);
    };
    let next = viable_sibling(vm, node, args, true);
    let inserted = convert(vm, parent, args)?;
    pre_insert(vm, inserted, parent, next)?;
    Ok(Value::Undefined)
}

pub(super) fn replace_with(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let Some(parent) = host(vm).and_then(|h| h.doc.node(node).parent) else {
        return Ok(Value::Undefined);
    };
    let next = viable_sibling(vm, node, args, true);
    let inserted = convert(vm, parent, args)?;
    // La conversion a pu sortir `node` de son parent (il était un argument).
    let still_child = host(vm).is_some_and(|h| h.doc.node(node).parent == Some(parent));
    if still_child {
        replace(vm, node, inserted, parent)?;
    } else {
        pre_insert(vm, inserted, parent, next)?;
    }
    Ok(Value::Undefined)
}

/// Insère chacun des `nodes` dans `parent` avant `reference`.
pub(super) fn insert_all(
    vm: &mut Vm,
    parent: NodeId,
    nodes: &[NodeId],
    reference: Option<NodeId>,
) -> JsResult<()> {
    for node in nodes {
        pre_insert(vm, *node, parent, reference)?;
    }
    Ok(())
}
