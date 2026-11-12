//! `Range` (DOM Standard §5.5) : plages vivantes sur l'arbre.
//!
//! Les frontières vivent dans le document (`oxyde_dom::LiveRange`), qui les
//! ajuste à chaque mutation ; ce module porte les algorithmes du standard —
//! comparaison de points, extraction, copie, suppression, insertion.
//!
//! LIMITES CONNUES : une plage n'est jamais libérée avant la fin de la page ;
//! `getBoundingClientRect` et `getClientRects` ne sont pas servis (ils
//! dépendent de la mise en page).

use std::cmp::Ordering;

use oxyde_dom::{Boundary, Document, LiveRange, NodeData, NodeId};
use oxyde_js::{JsResult, Value, Vm};

use super::dom_ops::offset_arg;
use super::tree::{self, Refusal};
use super::{CLASS_RANGE, arg, dom_exception, host, node_arg, string, wrap};

/// Profondeur maximale des extractions imbriquées.
const MAX_DEPTH: usize = 2_000;

const HIERARCHY: &str = "HierarchyRequestError";
const INVALID_NODE_TYPE: &str = "InvalidNodeTypeError";

// ---- Géométrie de l'arbre --------------------------------------------------------------

fn root(doc: &Document, node: NodeId) -> NodeId {
    doc.ancestors(node).last().unwrap_or(node)
}

fn is_doctype(doc: &Document, node: NodeId) -> bool {
    matches!(doc.node(node).data, NodeData::Doctype { .. })
}

fn is_text(doc: &Document, node: NodeId) -> bool {
    matches!(doc.node(node).data, NodeData::Text(_) | NodeData::CData(_))
}

fn has_data(doc: &Document, node: NodeId) -> bool {
    doc.character_data(node).is_some()
}

/// Vrai si `a` précède `b` dans l'ordre de l'arbre (un ancêtre précède ses
/// descendants). Faux si les deux nœuds n'ont pas la même racine.
fn precedes(doc: &Document, a: NodeId, b: NodeId) -> bool {
    if a == b {
        return false;
    }
    let path = |n: NodeId| {
        let mut path: Vec<NodeId> = doc.ancestors(n).collect();
        path.reverse();
        path.push(n);
        path
    };
    let (path_a, path_b) = (path(a), path(b));
    if path_a.first() != path_b.first() {
        return false;
    }
    for (x, y) in path_a.iter().zip(&path_b) {
        if x != y {
            return doc.index_of(*x) < doc.index_of(*y);
        }
    }
    path_a.len() < path_b.len()
}

/// Position du point `a` par rapport au point `b` (même racine).
fn position(doc: &Document, a: Boundary, b: Boundary) -> Ordering {
    if a.0 == b.0 {
        return a.1.cmp(&b.1);
    }
    if precedes(doc, b.0, a.0) {
        return position(doc, b, a).reverse();
    }
    if doc.contains(a.0, b.0) {
        let mut child = b.0;
        while let Some(parent) = doc.node(child).parent {
            if parent == a.0 {
                break;
            }
            child = parent;
        }
        if doc.index_of(child) < a.1 {
            return Ordering::Greater;
        }
    }
    Ordering::Less
}

/// Vrai si `node` est contenu dans la plage `start`–`end`.
fn contained(doc: &Document, start: Boundary, end: Boundary, node: NodeId) -> bool {
    root(doc, node) == root(doc, start.0)
        && position(doc, (node, 0), start) == Ordering::Greater
        && position(doc, (node, doc.node_length(node)), end) == Ordering::Less
}

/// Vrai si `node` est partiellement contenu : ancêtre (au sens large) d'une
/// seule des deux extrémités.
fn partially_contained(doc: &Document, start: Boundary, end: Boundary, node: NodeId) -> bool {
    doc.contains(node, start.0) != doc.contains(node, end.0)
}

fn document_of(doc: &Document, node: NodeId) -> NodeId {
    if doc.is_document(node) {
        node
    } else {
        doc.owner(node)
    }
}

// ---- Accès à la plage d'un objet --------------------------------------------------------

/// Enveloppe JavaScript de la plage `id`.
fn wrap_range(vm: &mut Vm, id: usize) -> Value {
    let Some(proto) = host(vm).and_then(|h| h.protos.get("Range").copied()) else {
        return Value::Null;
    };
    Value::Object(vm.new_host_object(proto, CLASS_RANGE, id as u64))
}

/// Crée une plage réduite au point `point`.
fn new_range(vm: &mut Vm, start: Boundary, end: Boundary) -> JsResult {
    match host(vm).and_then(|h| h.doc.add_range(start, end)) {
        Some(id) => Ok(wrap_range(vm, id)),
        None => Err(vm.error("RangeError", "trop de plages créées par cette page")),
    }
}

/// Identifiant et frontières de la plage désignée par `value`.
fn range_of(vm: &mut Vm, value: &Value) -> JsResult<(usize, LiveRange)> {
    let found = vm
        .host_data(value)
        .filter(|(class, _)| *class == CLASS_RANGE)
        .and_then(|(_, data)| usize::try_from(data).ok())
        .and_then(|id| Some((id, host(vm)?.doc.range(id)?)));
    match found {
        Some(found) => Ok(found),
        None => Err(vm.error("TypeError", "receveur attendu : un Range")),
    }
}

/// Applique `operation` au document et lève la `DOMException` d'un refus.
fn apply<T>(
    vm: &mut Vm,
    operation: impl FnOnce(&mut Document) -> Result<T, Refusal>,
) -> JsResult<T> {
    let Some(h) = host(vm) else {
        return Err(vm.error("TypeError", "document indisponible"));
    };
    let outcome = operation(&mut h.doc);
    h.dirty = true;
    match outcome {
        Ok(value) => Ok(value),
        Err((name, message)) => Err(dom_exception(vm, name, message)),
    }
}

fn need(vm: &mut Vm, args: &[Value], count: usize) -> JsResult<()> {
    if args.len() < count {
        return Err(vm.error("TypeError", "il manque un argument"));
    }
    Ok(())
}

fn offset(vm: &mut Vm, value: &Value) -> JsResult<u32> {
    Ok(u32::try_from(offset_arg(vm, value)?).unwrap_or(u32::MAX))
}

// ---- Création et attributs ---------------------------------------------------------------

pub(super) fn range_ctor(vm: &mut Vm, _this: &Value, _args: &[Value]) -> JsResult {
    if !vm.is_constructing() {
        return Err(vm.error("TypeError", "Range doit être appelé avec « new »"));
    }
    let document = host(vm).map_or_else(|| Document::new().root(), |h| h.document);
    new_range(vm, (document, 0), (document, 0))
}

pub(super) fn create_range(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let document = super::this_node(vm, this)?;
    new_range(vm, (document, 0), (document, 0))
}

/// Attribut `key` de la plage `data`, pour l'objet hôte.
pub(super) fn property(vm: &mut Vm, data: u64, key: &str) -> Option<JsResult> {
    let id = usize::try_from(data).ok()?;
    let h = host(vm)?;
    let range = h.doc.range(id)?;
    let doc = &h.doc;
    let number = |n: u32| Ok(Value::Number(f64::from(n)));
    Some(match key {
        "startContainer" => return Some(Ok(wrap(vm, range.start.0))),
        "endContainer" => return Some(Ok(wrap(vm, range.end.0))),
        "startOffset" => number(range.start.1),
        "endOffset" => number(range.end.1),
        "collapsed" => Ok(Value::Bool(range.start == range.end)),
        "commonAncestorContainer" => {
            let mut container = range.start.0;
            while !doc.contains(container, range.end.0) {
                match doc.node(container).parent {
                    Some(parent) => container = parent,
                    None => break,
                }
            }
            return Some(Ok(wrap(vm, container)));
        }
        _ => return None,
    })
}

// ---- Déplacement des frontières -----------------------------------------------------------

/// « Set the start or end » : place le début (`start`) ou la fin de la plage
/// au point (`node`, `offset`).
fn set_boundary(
    doc: &mut Document,
    id: usize,
    node: NodeId,
    offset: u32,
    start: bool,
) -> Result<(), Refusal> {
    if is_doctype(doc, node) {
        return Err((INVALID_NODE_TYPE, "un DOCTYPE ne borne pas une plage"));
    }
    if offset > doc.node_length(node) {
        return Err(("IndexSizeError", "la position dépasse la longueur du nœud"));
    }
    let Some(mut range) = doc.range(id) else {
        return Ok(());
    };
    let point = (node, offset);
    let other_root = root(doc, range.start.0) != root(doc, node);
    if start {
        if other_root || position(doc, point, range.end) == Ordering::Greater {
            range.end = point;
        }
        range.start = point;
    } else {
        if other_root || position(doc, point, range.start) == Ordering::Less {
            range.start = point;
        }
        range.end = point;
    }
    doc.set_range(id, range);
    Ok(())
}

fn set_point(vm: &mut Vm, this: &Value, args: &[Value], start: bool) -> JsResult {
    let (id, _) = range_of(vm, this)?;
    need(vm, args, 2)?;
    let node = node_arg(vm, &arg(args, 0))?;
    let at = offset(vm, &arg(args, 1))?;
    apply(vm, |doc| set_boundary(doc, id, node, at, start))?;
    Ok(Value::Undefined)
}

pub(super) fn set_start(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    set_point(vm, this, args, true)
}

pub(super) fn set_end(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    set_point(vm, this, args, false)
}

/// `setStartBefore`, `setStartAfter`, `setEndBefore`, `setEndAfter`.
fn set_beside(vm: &mut Vm, this: &Value, args: &[Value], start: bool, after: bool) -> JsResult {
    let (id, _) = range_of(vm, this)?;
    let node = node_arg(vm, &arg(args, 0))?;
    apply(vm, |doc| {
        let Some(parent) = doc.node(node).parent else {
            return Err((INVALID_NODE_TYPE, "le nœud n'a pas de parent"));
        };
        let index = doc.index_of(node) + u32::from(after);
        set_boundary(doc, id, parent, index, start)
    })?;
    Ok(Value::Undefined)
}

pub(super) fn set_start_before(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    set_beside(vm, this, args, true, false)
}

pub(super) fn set_start_after(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    set_beside(vm, this, args, true, true)
}

pub(super) fn set_end_before(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    set_beside(vm, this, args, false, false)
}

pub(super) fn set_end_after(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    set_beside(vm, this, args, false, true)
}

pub(super) fn collapse(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let (id, mut range) = range_of(vm, this)?;
    if arg(args, 0).truthy() {
        range.end = range.start;
    } else {
        range.start = range.end;
    }
    if let Some(h) = host(vm) {
        h.doc.set_range(id, range);
    }
    Ok(Value::Undefined)
}

/// « Select » : la plage entoure exactement `node`.
fn select(doc: &mut Document, id: usize, node: NodeId) -> Result<(), Refusal> {
    let Some(parent) = doc.node(node).parent else {
        return Err((INVALID_NODE_TYPE, "le nœud n'a pas de parent"));
    };
    let index = doc.index_of(node);
    doc.set_range(
        id,
        LiveRange {
            start: (parent, index),
            end: (parent, index + 1),
        },
    );
    Ok(())
}

pub(super) fn select_node(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let (id, _) = range_of(vm, this)?;
    let node = node_arg(vm, &arg(args, 0))?;
    apply(vm, |doc| select(doc, id, node))?;
    Ok(Value::Undefined)
}

pub(super) fn select_node_contents(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let (id, _) = range_of(vm, this)?;
    let node = node_arg(vm, &arg(args, 0))?;
    apply(vm, |doc| {
        if is_doctype(doc, node) {
            return Err((INVALID_NODE_TYPE, "un DOCTYPE n'a pas de contenu"));
        }
        let length = doc.node_length(node);
        doc.set_range(
            id,
            LiveRange {
                start: (node, 0),
                end: (node, length),
            },
        );
        Ok(())
    })?;
    Ok(Value::Undefined)
}

// ---- Comparaisons -------------------------------------------------------------------------

fn ordering(order: Ordering) -> JsResult {
    Ok(Value::Number(match order {
        Ordering::Less => -1.0,
        Ordering::Equal => 0.0,
        Ordering::Greater => 1.0,
    }))
}

pub(super) fn compare_boundary_points(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let (_, range) = range_of(vm, this)?;
    need(vm, args, 2)?;
    // `unsigned short` : la valeur est ramenée dans [0, 65535].
    let how = offset(vm, &arg(args, 0))? % 65_536;
    let (_, source) = range_of(vm, &arg(args, 1))?;
    if how > 3 {
        return Err(dom_exception(
            vm,
            "NotSupportedError",
            "comparaison inconnue : attendu une valeur de 0 à 3",
        ));
    }
    let order = apply(vm, |doc| {
        if root(doc, range.start.0) != root(doc, source.start.0) {
            return Err((
                "WrongDocumentError",
                "les deux plages ne sont pas dans le même arbre",
            ));
        }
        let (ours, theirs) = match how {
            0 => (range.start, source.start),
            1 => (range.end, source.start),
            2 => (range.end, source.end),
            _ => (range.start, source.end),
        };
        Ok(position(doc, ours, theirs))
    })?;
    ordering(order)
}

pub(super) fn is_point_in_range(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let (_, range) = range_of(vm, this)?;
    need(vm, args, 2)?;
    let node = node_arg(vm, &arg(args, 0))?;
    let at = offset(vm, &arg(args, 1))?;
    let inside = apply(vm, |doc| {
        if root(doc, node) != root(doc, range.start.0) {
            return Ok(false);
        }
        if is_doctype(doc, node) {
            return Err((INVALID_NODE_TYPE, "un DOCTYPE n'est pas un point"));
        }
        if at > doc.node_length(node) {
            return Err(("IndexSizeError", "la position dépasse la longueur du nœud"));
        }
        let point = (node, at);
        Ok(position(doc, point, range.start) != Ordering::Less
            && position(doc, point, range.end) != Ordering::Greater)
    })?;
    Ok(Value::Bool(inside))
}

pub(super) fn compare_point(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let (_, range) = range_of(vm, this)?;
    need(vm, args, 2)?;
    let node = node_arg(vm, &arg(args, 0))?;
    let at = offset(vm, &arg(args, 1))?;
    let order = apply(vm, |doc| {
        if root(doc, node) != root(doc, range.start.0) {
            return Err((
                "WrongDocumentError",
                "le point n'est pas dans l'arbre de la plage",
            ));
        }
        if is_doctype(doc, node) {
            return Err((INVALID_NODE_TYPE, "un DOCTYPE n'est pas un point"));
        }
        if at > doc.node_length(node) {
            return Err(("IndexSizeError", "la position dépasse la longueur du nœud"));
        }
        let point = (node, at);
        Ok(if position(doc, point, range.start) == Ordering::Less {
            Ordering::Less
        } else if position(doc, point, range.end) == Ordering::Greater {
            Ordering::Greater
        } else {
            Ordering::Equal
        })
    })?;
    ordering(order)
}

pub(super) fn intersects_node(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let (_, range) = range_of(vm, this)?;
    let node = node_arg(vm, &arg(args, 0))?;
    let Some(h) = host(vm) else {
        return Ok(Value::Bool(false));
    };
    let doc = &h.doc;
    if root(doc, node) != root(doc, range.start.0) {
        return Ok(Value::Bool(false));
    }
    let Some(parent) = doc.node(node).parent else {
        return Ok(Value::Bool(true));
    };
    let index = doc.index_of(node);
    Ok(Value::Bool(
        position(doc, (parent, index), range.end) == Ordering::Less
            && position(doc, (parent, index + 1), range.start) == Ordering::Greater,
    ))
}

// ---- Contenu ------------------------------------------------------------------------------

/// Point où la plage se referme après une extraction ou une suppression.
fn collapse_point(doc: &Document, start: Boundary, end: Boundary) -> Boundary {
    if doc.contains(start.0, end.0) {
        return start;
    }
    let mut reference = start.0;
    while let Some(parent) = doc.node(reference).parent {
        if doc.contains(parent, end.0) {
            break;
        }
        reference = parent;
    }
    match doc.node(reference).parent {
        Some(parent) => (parent, doc.index_of(reference) + 1),
        None => start,
    }
}

/// Copie sans enfants de `node` dont les données sont `text`.
fn data_copy(doc: &mut Document, node: NodeId, owner: NodeId, text: &str) -> NodeId {
    let copy = doc.clone_subtree(node, false, owner);
    doc.replace_data(copy, 0, u32::MAX, text);
    copy
}

/// « Extract » (ou « clone the contents » si `clone`) : le contenu de la
/// plage `start`–`end` dans un fragment neuf. Avec `clone`, l'arbre n'est
/// pas modifié.
fn contents(
    doc: &mut Document,
    start: Boundary,
    end: Boundary,
    clone: bool,
    depth: usize,
) -> Result<NodeId, Refusal> {
    let owner = document_of(doc, start.0);
    let fragment = doc.create_fragment();
    doc.set_owner(fragment, owner);
    if start == end {
        return Ok(fragment);
    }
    if depth > MAX_DEPTH {
        return Err((HIERARCHY, "arbre trop profond pour cette opération"));
    }
    let ((first_node, first_offset), (last_node, last_offset)) = (start, end);
    if first_node == last_node && has_data(doc, first_node) {
        let count = last_offset.saturating_sub(first_offset);
        let text = doc
            .substring_data(first_node, first_offset, count)
            .unwrap_or_default();
        let copy = data_copy(doc, first_node, owner, &text);
        doc.append_child(fragment, copy);
        if !clone {
            doc.replace_data(first_node, first_offset, count, "");
        }
        return Ok(fragment);
    }
    let mut common = first_node;
    while !doc.contains(common, last_node) {
        match doc.node(common).parent {
            Some(parent) => common = parent,
            None => return Ok(fragment),
        }
    }
    let partial = |doc: &Document, node: NodeId| partially_contained(doc, start, end, node);
    let first_partial = if doc.contains(first_node, last_node) {
        None
    } else {
        doc.children(common).find(|c| partial(doc, *c))
    };
    let last_partial = if doc.contains(last_node, first_node) {
        None
    } else {
        doc.children(common).filter(|c| partial(doc, *c)).last()
    };
    let inner: Vec<NodeId> = doc
        .children(common)
        .filter(|c| contained(doc, start, end, *c))
        .collect();
    if inner.iter().any(|c| is_doctype(doc, *c)) {
        return Err((HIERARCHY, "un DOCTYPE ne s'extrait pas d'un document"));
    }
    match first_partial {
        Some(first) if has_data(doc, first) => {
            let count = doc.data_len(first_node).saturating_sub(first_offset);
            let text = doc
                .substring_data(first_node, first_offset, count)
                .unwrap_or_default();
            let copy = data_copy(doc, first_node, owner, &text);
            doc.append_child(fragment, copy);
            if !clone {
                doc.replace_data(first_node, first_offset, count, "");
            }
        }
        Some(first) => {
            let copy = doc.clone_subtree(first, false, owner);
            doc.append_child(fragment, copy);
            let inner_end = (first, doc.node_length(first));
            let sub = contents(doc, start, inner_end, clone, depth + 1)?;
            doc.append_child(copy, sub);
        }
        None => {}
    }
    for child in inner {
        let moved = if clone {
            doc.clone_subtree(child, true, owner)
        } else {
            child
        };
        doc.append_child(fragment, moved);
    }
    match last_partial {
        Some(last) if has_data(doc, last) => {
            let text = doc
                .substring_data(last_node, 0, last_offset)
                .unwrap_or_default();
            let copy = data_copy(doc, last_node, owner, &text);
            doc.append_child(fragment, copy);
            if !clone {
                doc.replace_data(last_node, 0, last_offset, "");
            }
        }
        Some(last) => {
            let copy = doc.clone_subtree(last, false, owner);
            doc.append_child(fragment, copy);
            let sub = contents(doc, (last, 0), end, clone, depth + 1)?;
            doc.append_child(copy, sub);
        }
        None => {}
    }
    Ok(fragment)
}

/// « Extract » sur la plage `id` : rend le fragment et referme la plage.
fn extract(doc: &mut Document, id: usize) -> Result<NodeId, Refusal> {
    let Some(range) = doc.range(id) else {
        return Err((HIERARCHY, "plage inconnue"));
    };
    let same_data = range.start.0 == range.end.0 && has_data(doc, range.start.0);
    let point = collapse_point(doc, range.start, range.end);
    let fragment = contents(doc, range.start, range.end, false, 0)?;
    if range.start != range.end && !same_data {
        doc.set_range(
            id,
            LiveRange {
                start: point,
                end: point,
            },
        );
    }
    Ok(fragment)
}

pub(super) fn extract_contents(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let (id, _) = range_of(vm, this)?;
    let fragment = apply(vm, |doc| extract(doc, id))?;
    Ok(wrap(vm, fragment))
}

pub(super) fn clone_contents(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let (_, range) = range_of(vm, this)?;
    let fragment = apply(vm, |doc| contents(doc, range.start, range.end, true, 0))?;
    Ok(wrap(vm, fragment))
}

/// Nœuds contenus dans la plage dont le parent ne l'est pas, dans l'ordre.
fn outermost_contained(
    doc: &Document,
    start: Boundary,
    end: Boundary,
    node: NodeId,
    out: &mut Vec<NodeId>,
) {
    // Parcours itératif : seuls les ancêtres d'une extrémité sont explorés.
    let mut pending = vec![node];
    while let Some(current) = pending.pop() {
        let children: Vec<NodeId> = doc.children(current).collect();
        for child in children.into_iter().rev() {
            if contained(doc, start, end, child) {
                out.push(child);
            } else if doc.contains(child, start.0) || doc.contains(child, end.0) {
                pending.push(child);
            }
        }
    }
}

pub(super) fn delete_contents(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let (id, range) = range_of(vm, this)?;
    if range.start == range.end {
        return Ok(Value::Undefined);
    }
    apply(vm, |doc| {
        let ((first_node, first_offset), (last_node, last_offset)) = (range.start, range.end);
        if first_node == last_node && has_data(doc, first_node) {
            let count = last_offset.saturating_sub(first_offset);
            doc.replace_data(first_node, first_offset, count, "");
            return Ok(());
        }
        let mut common = first_node;
        while !doc.contains(common, last_node) {
            match doc.node(common).parent {
                Some(parent) => common = parent,
                None => return Ok(()),
            }
        }
        let mut doomed = Vec::new();
        outermost_contained(doc, range.start, range.end, common, &mut doomed);
        let point = collapse_point(doc, range.start, range.end);
        if has_data(doc, first_node) {
            let count = doc.data_len(first_node).saturating_sub(first_offset);
            doc.replace_data(first_node, first_offset, count, "");
        }
        for node in doomed {
            doc.detach(node);
        }
        if has_data(doc, last_node) {
            doc.replace_data(last_node, 0, last_offset, "");
        }
        doc.set_range(
            id,
            LiveRange {
                start: point,
                end: point,
            },
        );
        Ok(())
    })?;
    Ok(Value::Undefined)
}

/// « Insert » : place `node` au début de la plage `id`.
fn insert(doc: &mut Document, id: usize, node: NodeId) -> Result<(), Refusal> {
    let Some(range) = doc.range(id) else {
        return Ok(());
    };
    let (start_node, start_offset) = range.start;
    let text_start = is_text(doc, start_node);
    let refused = matches!(
        doc.node(start_node).data,
        NodeData::ProcessingInstruction { .. } | NodeData::Comment(_)
    ) || (text_start && doc.node(start_node).parent.is_none())
        || start_node == node;
    if refused {
        return Err((HIERARCHY, "rien ne s'insère à cet endroit de la plage"));
    }
    let mut reference = if text_start {
        Some(start_node)
    } else {
        doc.child_at(start_node, start_offset)
    };
    let parent = match reference {
        Some(reference) => doc.node(reference).parent.unwrap_or(start_node),
        None => start_node,
    };
    tree::validity(doc, node, parent, reference, false)?;
    if text_start {
        reference = doc.split_text(start_node, start_offset);
    }
    if reference == Some(node) {
        reference = doc.node(node).next_sibling;
    }
    doc.detach(node);
    let mut new_offset = match reference {
        Some(reference) => doc.index_of(reference),
        None => doc.node_length(parent),
    };
    new_offset += if matches!(doc.node(node).data, NodeData::Fragment) {
        doc.node_length(node)
    } else {
        1
    };
    tree::insert(doc, node, parent, reference);
    if let Some(mut now) = doc.range(id)
        && now.start == now.end
    {
        now.end = (parent, new_offset);
        doc.set_range(id, now);
    }
    Ok(())
}

pub(super) fn insert_node(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let (id, _) = range_of(vm, this)?;
    let node = node_arg(vm, &arg(args, 0))?;
    apply(vm, |doc| insert(doc, id, node))?;
    Ok(Value::Undefined)
}

pub(super) fn surround_contents(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let (id, range) = range_of(vm, this)?;
    let new_parent = node_arg(vm, &arg(args, 0))?;
    apply(vm, |doc| {
        // Un nœud partiellement contenu qui n'est pas un texte interdit l'opération.
        let (start, end) = (range.start.0, range.end.0);
        let lopsided = |doc: &Document, from: NodeId, other: NodeId| {
            std::iter::once(from)
                .chain(doc.ancestors(from))
                .any(|n| !doc.contains(n, other) && !is_text(doc, n))
        };
        if lopsided(doc, start, end) || lopsided(doc, end, start) {
            return Err((
                "InvalidStateError",
                "la plage coupe un nœud qui n'est pas un texte",
            ));
        }
        if matches!(
            doc.node(new_parent).data,
            NodeData::Document | NodeData::Doctype { .. } | NodeData::Fragment
        ) {
            return Err((INVALID_NODE_TYPE, "ce nœud ne peut pas entourer un contenu"));
        }
        let fragment = extract(doc, id)?;
        if doc.node(new_parent).first_child.is_some() {
            tree::replace_all(doc, None, new_parent);
        }
        insert(doc, id, new_parent)?;
        tree::validity(doc, fragment, new_parent, None, false)?;
        tree::insert(doc, fragment, new_parent, None);
        select(doc, id, new_parent)
    })?;
    Ok(Value::Undefined)
}

pub(super) fn clone_range(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let (_, range) = range_of(vm, this)?;
    new_range(vm, range.start, range.end)
}

/// `detach()` : sans effet, gardé par compatibilité.
pub(super) fn detach(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    range_of(vm, this)?;
    Ok(Value::Undefined)
}

pub(super) fn to_string(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let (_, range) = range_of(vm, this)?;
    let Some(h) = host(vm) else {
        return string("");
    };
    let doc = &h.doc;
    let ((first_node, first_offset), (last_node, last_offset)) = (range.start, range.end);
    if first_node == last_node && is_text(doc, first_node) {
        let count = last_offset.saturating_sub(first_offset);
        return string(
            &doc.substring_data(first_node, first_offset, count)
                .unwrap_or_default(),
        );
    }
    let mut out = String::new();
    if is_text(doc, first_node) {
        out.push_str(
            &doc.substring_data(first_node, first_offset, u32::MAX)
                .unwrap_or_default(),
        );
    }
    let top = root(doc, first_node);
    for node in doc.descendants(top) {
        if is_text(doc, node)
            && contained(doc, range.start, range.end, node)
            && let Some(text) = doc.character_data(node)
        {
            out.push_str(text);
        }
    }
    if is_text(doc, last_node) {
        out.push_str(
            &doc.substring_data(last_node, 0, last_offset)
                .unwrap_or_default(),
        );
    }
    string(&out)
}
