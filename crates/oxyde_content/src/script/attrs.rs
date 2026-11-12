//! Attributs et espaces de noms (DOM Standard §4.9) : les méthodes
//! d'`Element` sur ses attributs et leurs variantes `…NS`, les nœuds `Attr`,
//! `NamedNodeMap`, `createElementNS`, `getElementsByTagName(NS)` et la
//! recherche d'espaces de noms (`lookupNamespaceURI`…).
//!
//! LIMITES CONNUES : les collections rendues par `getElementsByTagName` sont
//! des instantanés, pas des collections vivantes ; les propriétés nommées
//! d'un `NamedNodeMap` ne s'énumèrent pas.

use oxyde_dom::{Attribute, Document, Namespace, NodeData, NodeId, XML_NS, XMLNS_NS};
use oxyde_js::{JsResult, Value, Vm};

use super::documents::{created_by, is_element_local_name, is_html_document};
use super::dom_ops::offset_arg;
use super::{
    CLASS_ATTRS, arg, dom_exception, host, node_arg, string, this_node, wrap, wrap_collection,
    wrap_opt,
};

/// Emplacement qui garde l'objet `attributes` d'un élément.
const ATTRIBUTES: &str = "élément.attributs";

// ---- Noms -------------------------------------------------------------------------------

/// « Valid attribute local name ».
fn is_attribute_local_name(name: &str) -> bool {
    !name.is_empty()
        && !name
            .chars()
            .any(|c| c.is_ascii_whitespace() || matches!(c, '\0' | '/' | '=' | '>'))
}

/// « Valid namespace prefix ».
fn is_namespace_prefix(prefix: &str) -> bool {
    !prefix.is_empty()
        && !prefix
            .chars()
            .any(|c| c.is_ascii_whitespace() || matches!(c, '\0' | '/' | '>'))
}

/// Argument `DOMString?` d'espace de noms : `null` et la chaîne vide valent
/// « aucun ».
fn namespace_arg(vm: &mut Vm, value: &Value) -> JsResult<String> {
    Ok(match value {
        Value::Null | Value::Undefined => String::new(),
        other => vm.to_string(other)?.to_string(),
    })
}

/// Nom extrait d'un nom qualifié : espace de noms, préfixe, nom local.
struct Extracted {
    ns: Namespace,
    prefix: Option<String>,
    local: String,
}

/// « Validate and extract » un espace de noms et un nom qualifié, pour un
/// élément (`element`) ou un attribut.
fn extract(vm: &mut Vm, namespace: &str, qualified: &str, element: bool) -> JsResult<Extracted> {
    let (prefix, local) = match qualified.split_once(':') {
        Some((prefix, local)) => (Some(prefix), local),
        None => (None, qualified),
    };
    let valid = prefix.is_none_or(is_namespace_prefix)
        && if element {
            is_element_local_name(local)
        } else {
            is_attribute_local_name(local)
        };
    if !valid {
        return Err(dom_exception(
            vm,
            "InvalidCharacterError",
            "nom qualifié invalide",
        ));
    }
    let misplaced = (prefix.is_some() && namespace.is_empty())
        || (prefix == Some("xml") && namespace != XML_NS)
        || ((qualified == "xmlns" || prefix == Some("xmlns")) && namespace != XMLNS_NS)
        || (namespace == XMLNS_NS && qualified != "xmlns" && prefix != Some("xmlns"));
    if misplaced {
        return Err(dom_exception(
            vm,
            "NamespaceError",
            "le préfixe ne s'accorde pas avec l'espace de noms",
        ));
    }
    let ns = host(vm).map_or(Namespace::None, |h| h.doc.namespace(namespace));
    Ok(Extracted {
        ns,
        prefix: prefix.map(str::to_owned),
        local: local.to_owned(),
    })
}

/// Élément receveur d'une méthode d'`Element`.
fn this_element(vm: &mut Vm, this: &Value) -> JsResult<NodeId> {
    let node = this_node(vm, this)?;
    if host(vm).is_some_and(|h| h.doc.element(node).is_some()) {
        Ok(node)
    } else {
        Err(vm.error("TypeError", "receveur attendu : un élément"))
    }
}

/// Vrai si `element` est un élément HTML d'un document HTML : ses noms
/// d'attributs se comparent en minuscules.
fn is_html_element(vm: &mut Vm, element: NodeId) -> bool {
    let Some(h) = host(vm) else { return true };
    if h.doc
        .element(element)
        .is_none_or(|e| e.ns != Namespace::Html)
    {
        return false;
    }
    let owner = h.doc.owner(element);
    is_html_document(vm, owner)
}

/// Nom qualifié tel que `element` le compare.
fn adjusted(vm: &mut Vm, element: NodeId, name: &str) -> String {
    if is_html_element(vm, element) {
        name.to_ascii_lowercase()
    } else {
        name.to_owned()
    }
}

fn invalid_name(vm: &mut Vm) -> Value {
    dom_exception(vm, "InvalidCharacterError", "nom d'attribut invalide")
}

// ---- Element : attributs par nom --------------------------------------------------------

pub(super) fn get_attribute(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let element = this_element(vm, this)?;
    let name = vm.to_string(&arg(args, 0))?;
    let name = adjusted(vm, element, &name);
    let value = host(vm).and_then(|h| {
        let index = h.doc.attr_index(element, &name)?;
        Some(h.doc.element(element)?.attrs.get(index)?.value.clone())
    });
    Ok(value.map_or(Value::Null, |v| Value::string(&v)))
}

pub(super) fn set_attribute(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let element = this_element(vm, this)?;
    if args.len() < 2 {
        return Err(vm.error("TypeError", "setAttribute attend deux arguments"));
    }
    let name = vm.to_string(&arg(args, 0))?;
    let value = vm.to_string(&arg(args, 1))?;
    if !is_attribute_local_name(&name) {
        return Err(invalid_name(vm));
    }
    let name = adjusted(vm, element, &name);
    if let Some(h) = host(vm) {
        match h.doc.attr_index(element, &name) {
            Some(index) => h.doc.set_attr_at(element, index, &value),
            None => h
                .doc
                .push_attr(element, Attribute::new(name, value.to_string())),
        }
        h.dirty = true;
    }
    Ok(Value::Undefined)
}

pub(super) fn remove_attribute(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let element = this_element(vm, this)?;
    let name = vm.to_string(&arg(args, 0))?;
    let name = adjusted(vm, element, &name);
    if let Some(h) = host(vm)
        && let Some(index) = h.doc.attr_index(element, &name)
    {
        h.doc.remove_attr_at(element, index);
        h.dirty = true;
    }
    Ok(Value::Undefined)
}

pub(super) fn has_attribute(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let element = this_element(vm, this)?;
    let name = vm.to_string(&arg(args, 0))?;
    let name = adjusted(vm, element, &name);
    Ok(Value::Bool(host(vm).is_some_and(|h| {
        h.doc.attr_index(element, &name).is_some()
    })))
}

pub(super) fn toggle_attribute(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let element = this_element(vm, this)?;
    let name = vm.to_string(&arg(args, 0))?;
    if !is_attribute_local_name(&name) {
        return Err(invalid_name(vm));
    }
    let name = adjusted(vm, element, &name);
    let force = args
        .get(1)
        .filter(|v| **v != Value::Undefined)
        .map(Value::truthy);
    let Some(h) = host(vm) else {
        return Ok(Value::Bool(false));
    };
    let present = h.doc.attr_index(element, &name);
    let wanted = force.unwrap_or(present.is_none());
    match (present, wanted) {
        (None, true) => h.doc.push_attr(element, Attribute::new(name, "")),
        (Some(index), false) => {
            h.doc.remove_attr_at(element, index);
        }
        _ => {}
    }
    h.dirty = true;
    Ok(Value::Bool(wanted))
}

pub(super) fn has_attributes(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let element = this_element(vm, this)?;
    Ok(Value::Bool(host(vm).is_some_and(|h| {
        h.doc.element(element).is_some_and(|e| !e.attrs.is_empty())
    })))
}

pub(super) fn get_attribute_names(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let element = this_element(vm, this)?;
    let names: Vec<Value> = host(vm)
        .and_then(|h| h.doc.element(element))
        .map(|e| e.attrs.iter().map(|a| Value::string(&a.name)).collect())
        .unwrap_or_default();
    Ok(vm.new_array(names))
}

// ---- Element : attributs par espace de noms ----------------------------------------------

/// Rang de l'attribut désigné par (`namespace`, `local`) : arguments 0 et 1.
fn index_ns(vm: &mut Vm, element: NodeId, args: &[Value]) -> JsResult<Option<usize>> {
    let namespace = namespace_arg(vm, &arg(args, 0))?;
    let local = vm.to_string(&arg(args, 1))?;
    Ok(host(vm).and_then(|h| {
        let ns = h.doc.namespace(&namespace);
        h.doc.attr_index_ns(element, ns, &local)
    }))
}

pub(super) fn get_attribute_ns(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let element = this_element(vm, this)?;
    let index = index_ns(vm, element, args)?;
    let value =
        host(vm).and_then(|h| Some(h.doc.element(element)?.attrs.get(index?)?.value.clone()));
    Ok(value.map_or(Value::Null, |v| Value::string(&v)))
}

pub(super) fn has_attribute_ns(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let element = this_element(vm, this)?;
    Ok(Value::Bool(index_ns(vm, element, args)?.is_some()))
}

pub(super) fn set_attribute_ns(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let element = this_element(vm, this)?;
    if args.len() < 3 {
        return Err(vm.error("TypeError", "setAttributeNS attend trois arguments"));
    }
    let namespace = namespace_arg(vm, &arg(args, 0))?;
    let qualified = vm.to_string(&arg(args, 1))?;
    let value = vm.to_string(&arg(args, 2))?;
    let name = extract(vm, &namespace, &qualified, false)?;
    if let Some(h) = host(vm) {
        match h.doc.attr_index_ns(element, name.ns, &name.local) {
            Some(index) => h.doc.set_attr_at(element, index, &value),
            None => h.doc.push_attr(
                element,
                Attribute {
                    name: qualified.to_string(),
                    value: value.to_string(),
                    ns: name.ns,
                    prefix: name.prefix,
                    node: None,
                },
            ),
        }
        h.dirty = true;
    }
    Ok(Value::Undefined)
}

pub(super) fn remove_attribute_ns(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let element = this_element(vm, this)?;
    if let Some(index) = index_ns(vm, element, args)?
        && let Some(h) = host(vm)
    {
        h.doc.remove_attr_at(element, index);
        h.dirty = true;
    }
    Ok(Value::Undefined)
}

// ---- Nœuds Attr ---------------------------------------------------------------------------

fn wrap_attr(vm: &mut Vm, element: NodeId, index: Option<usize>) -> Value {
    let node = index.and_then(|i| host(vm)?.doc.attr_node(element, i));
    wrap_opt(vm, node)
}

/// Nœud d'attribut passé en argument.
fn attr_arg(vm: &mut Vm, value: &Value) -> JsResult<NodeId> {
    let node = node_arg(vm, value)?;
    if host(vm).is_some_and(|h| h.doc.attr_data(node).is_some()) {
        Ok(node)
    } else {
        Err(vm.error("TypeError", "un nœud Attr est attendu en argument"))
    }
}

pub(super) fn get_attribute_node(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let element = this_element(vm, this)?;
    let name = vm.to_string(&arg(args, 0))?;
    let name = adjusted(vm, element, &name);
    let index = host(vm).and_then(|h| h.doc.attr_index(element, &name));
    Ok(wrap_attr(vm, element, index))
}

pub(super) fn get_attribute_node_ns(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let element = this_element(vm, this)?;
    let index = index_ns(vm, element, args)?;
    Ok(wrap_attr(vm, element, index))
}

/// « Set an attribute » : fait porter `attr` par `element` ; rend l'ancien
/// nœud remplacé, ou `null`.
fn set_attr_node(vm: &mut Vm, element: NodeId, attr: NodeId) -> JsResult {
    let Some(h) = host(vm) else {
        return Ok(Value::Null);
    };
    let Some(data) = h.doc.attr_data(attr).cloned() else {
        return Ok(Value::Null);
    };
    if data.element.is_some_and(|owner| owner != element) {
        return Err(dom_exception(
            vm,
            "InUseAttributeError",
            "cet attribut appartient déjà à un autre élément",
        ));
    }
    let existing = h.doc.attr_index_ns(element, data.ns, &data.local);
    let old = existing.and_then(|index| h.doc.attr_node(element, index));
    if old == Some(attr) {
        return Ok(wrap(vm, attr));
    }
    h.doc.attach_attr(element, attr, existing);
    h.dirty = true;
    Ok(wrap_opt(vm, old))
}

pub(super) fn set_attribute_node(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let element = this_element(vm, this)?;
    let attr = attr_arg(vm, &arg(args, 0))?;
    set_attr_node(vm, element, attr)
}

pub(super) fn remove_attribute_node(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let element = this_element(vm, this)?;
    let attr = attr_arg(vm, &arg(args, 0))?;
    let index = host(vm).and_then(|h| {
        h.doc
            .element(element)?
            .attrs
            .iter()
            .position(|a| a.node == Some(attr))
    });
    let Some(index) = index else {
        return Err(dom_exception(
            vm,
            "NotFoundError",
            "cet attribut n'appartient pas à l'élément",
        ));
    };
    if let Some(h) = host(vm) {
        h.doc.remove_attr_at(element, index);
        h.dirty = true;
    }
    Ok(arg(args, 0))
}

pub(super) fn create_attribute(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let document = this_node(vm, this)?;
    let name = vm.to_string(&arg(args, 0))?;
    if !is_attribute_local_name(&name) {
        return Err(invalid_name(vm));
    }
    let local = if is_html_document(vm, document) {
        name.to_ascii_lowercase()
    } else {
        name.to_string()
    };
    let node = host(vm).map(|h| h.doc.create_attr(&local, Namespace::None, None));
    Ok(created_by(vm, this, node))
}

pub(super) fn create_attribute_ns(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    this_node(vm, this)?;
    if args.len() < 2 {
        return Err(vm.error("TypeError", "createAttributeNS attend deux arguments"));
    }
    let namespace = namespace_arg(vm, &arg(args, 0))?;
    let qualified = vm.to_string(&arg(args, 1))?;
    let name = extract(vm, &namespace, &qualified, false)?;
    let node = host(vm).map(|h| {
        h.doc
            .create_attr(&name.local, name.ns, name.prefix.as_deref())
    });
    Ok(created_by(vm, this, node))
}

/// Attribut `key` du nœud d'attribut `node`, pour l'objet hôte.
pub(super) fn property(vm: &mut Vm, node: NodeId, key: &str) -> Option<JsResult> {
    let h = host(vm)?;
    let data = h.doc.attr_data(node)?;
    let text = |value: Option<&str>| Ok(value.map_or(Value::Null, Value::string));
    Some(match key {
        "name" | "nodeName" => string(&data.qualified()),
        "localName" => string(&data.local),
        "prefix" => text(data.prefix.as_deref()),
        "namespaceURI" => text(h.doc.namespace_uri(data.ns)),
        "value" | "nodeValue" | "textContent" => string(h.doc.attr_value(node).unwrap_or("")),
        "specified" => Ok(Value::Bool(true)),
        "ownerElement" => {
            let element = data.element;
            return Some(Ok(wrap_opt(vm, element)));
        }
        _ => return None,
    })
}

// ---- NamedNodeMap -------------------------------------------------------------------------

/// `element.attributes` : toujours le même objet pour un élément.
pub(super) fn named_node_map(vm: &mut Vm, element: NodeId) -> Value {
    let wrapper = wrap(vm, element);
    let Some(wrapper_id) = wrapper.as_object() else {
        return Value::Null;
    };
    if let Some(existing) = vm.slot(wrapper_id, ATTRIBUTES) {
        return existing;
    }
    let Some(proto) = host(vm).and_then(|h| h.protos.get("NamedNodeMap").copied()) else {
        return Value::Null;
    };
    let index = u64::try_from(element.index()).unwrap_or(u64::MAX);
    let map = Value::Object(vm.new_host_object(proto, CLASS_ATTRS, index));
    vm.set_slot(wrapper_id, ATTRIBUTES, map.clone());
    map
}

/// Élément d'un objet `NamedNodeMap`.
fn map_element(vm: &mut Vm, this: &Value) -> JsResult<NodeId> {
    let element = vm
        .host_data(this)
        .filter(|(class, _)| *class == CLASS_ATTRS)
        .and_then(|(_, data)| host(vm)?.doc.id_from_index(usize::try_from(data).ok()?));
    match element {
        Some(element) => Ok(element),
        None => Err(vm.error("TypeError", "receveur attendu : un NamedNodeMap")),
    }
}

/// Propriété `key` d'un `NamedNodeMap` : `length`, un rang, ou un nom.
pub(super) fn map_property(vm: &mut Vm, data: u64, key: &str) -> Option<JsResult> {
    let element = host(vm)?.doc.id_from_index(usize::try_from(data).ok()?)?;
    if key == "length" {
        let count = host(vm)?.doc.element(element)?.attrs.len();
        return Some(Ok(super::number(count)));
    }
    if let Ok(index) = key.parse::<usize>() {
        let present = host(vm)?.doc.element(element)?.attrs.len() > index;
        return present.then(|| Ok(wrap_attr(vm, element, Some(index))));
    }
    // Les méthodes du prototype ne sont jamais masquées par un attribut.
    if matches!(
        key,
        "item"
            | "getNamedItem"
            | "getNamedItemNS"
            | "setNamedItem"
            | "setNamedItemNS"
            | "removeNamedItem"
            | "removeNamedItemNS"
            | "constructor"
            | "toString"
            | "valueOf"
    ) {
        return None;
    }
    let name = adjusted(vm, element, key);
    let index = host(vm)?.doc.attr_index(element, &name)?;
    Some(Ok(wrap_attr(vm, element, Some(index))))
}

pub(super) fn map_item(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let element = map_element(vm, this)?;
    let index = offset_arg(vm, &arg(args, 0))?;
    let present = host(vm).is_some_and(|h| {
        h.doc
            .element(element)
            .is_some_and(|e| e.attrs.len() > index)
    });
    Ok(wrap_attr(vm, element, present.then_some(index)))
}

pub(super) fn map_get(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let element = map_element(vm, this)?;
    let name = vm.to_string(&arg(args, 0))?;
    let name = adjusted(vm, element, &name);
    let index = host(vm).and_then(|h| h.doc.attr_index(element, &name));
    Ok(wrap_attr(vm, element, index))
}

pub(super) fn map_get_ns(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let element = map_element(vm, this)?;
    let index = index_ns(vm, element, args)?;
    Ok(wrap_attr(vm, element, index))
}

pub(super) fn map_set(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let element = map_element(vm, this)?;
    let attr = attr_arg(vm, &arg(args, 0))?;
    set_attr_node(vm, element, attr)
}

/// Retire l'attribut de rang `index` et rend son nœud.
fn map_take(vm: &mut Vm, element: NodeId, index: Option<usize>) -> JsResult {
    let Some(index) = index else {
        return Err(dom_exception(
            vm,
            "NotFoundError",
            "l'élément n'a pas cet attribut",
        ));
    };
    let removed = wrap_attr(vm, element, Some(index));
    if let Some(h) = host(vm) {
        h.doc.remove_attr_at(element, index);
        h.dirty = true;
    }
    Ok(removed)
}

pub(super) fn map_remove(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let element = map_element(vm, this)?;
    let name = vm.to_string(&arg(args, 0))?;
    let name = adjusted(vm, element, &name);
    let index = host(vm).and_then(|h| h.doc.attr_index(element, &name));
    map_take(vm, element, index)
}

pub(super) fn map_remove_ns(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let element = map_element(vm, this)?;
    let index = index_ns(vm, element, args)?;
    map_take(vm, element, index)
}

// ---- Éléments par nom ---------------------------------------------------------------------

pub(super) fn create_element_ns(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    if args.len() < 2 {
        return Err(vm.error("TypeError", "createElementNS attend deux arguments"));
    }
    let namespace = namespace_arg(vm, &arg(args, 0))?;
    let qualified = vm.to_string(&arg(args, 1))?;
    let name = extract(vm, &namespace, &qualified, true)?;
    let node = host(vm).map(|h| {
        h.doc
            .create_element_ns(&name.local, name.ns, name.prefix.as_deref())
    });
    Ok(created_by(vm, this, node))
}

pub(super) fn get_elements_by_tag_name(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let scope = this_node(vm, this)?;
    let name = vm.to_string(&arg(args, 0))?;
    let lowered = name.to_ascii_lowercase();
    let document = super::documents::document_of(vm, scope);
    let html = is_html_document(vm, document);
    let found: Vec<NodeId> = host(vm)
        .map(|h| {
            let doc = &h.doc;
            doc.descendants(scope)
                .filter(|n| {
                    doc.element(*n).is_some_and(|e| {
                        *name == *"*"
                            || if html && e.ns == Namespace::Html {
                                e.qualified() == lowered
                            } else {
                                e.qualified() == *name
                            }
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(wrap_collection(vm, found))
}

pub(super) fn get_elements_by_tag_name_ns(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let scope = this_node(vm, this)?;
    if args.len() < 2 {
        return Err(vm.error("TypeError", "getElementsByTagNameNS attend deux arguments"));
    }
    let namespace = namespace_arg(vm, &arg(args, 0))?;
    let local = vm.to_string(&arg(args, 1))?;
    let found: Vec<NodeId> = host(vm)
        .map(|h| {
            let ns = (namespace != "*").then(|| h.doc.namespace(&namespace));
            let doc = &h.doc;
            doc.descendants(scope)
                .filter(|n| {
                    doc.element(*n).is_some_and(|e| {
                        ns.is_none_or(|ns| e.ns == ns) && (*local == *"*" || e.name == *local)
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(wrap_collection(vm, found))
}

// ---- Recherche d'espaces de noms ----------------------------------------------------------

/// Élément d'où part la recherche d'un espace de noms pour `node`.
fn lookup_start(doc: &Document, node: NodeId) -> Option<NodeId> {
    match &doc.node(node).data {
        NodeData::Element(_) => Some(node),
        NodeData::Document => doc.children(node).find(|c| doc.element(*c).is_some()),
        NodeData::Doctype { .. } | NodeData::Fragment => None,
        NodeData::Attr(attr) => attr.element,
        _ => doc
            .node(node)
            .parent
            .filter(|parent| doc.element(*parent).is_some()),
    }
}

fn parent_element(doc: &Document, node: NodeId) -> Option<NodeId> {
    doc.node(node)
        .parent
        .filter(|parent| doc.element(*parent).is_some())
}

/// « Locate a namespace » : adresse de l'espace de noms que `prefix` désigne
/// (ou de l'espace par défaut si `prefix` est `None`) vu de `element`.
fn locate_namespace(doc: &Document, element: NodeId, prefix: Option<&str>) -> Option<String> {
    match prefix {
        Some("xml") => return Some(XML_NS.to_owned()),
        Some("xmlns") => return Some(XMLNS_NS.to_owned()),
        _ => {}
    }
    let mut current = Some(element);
    while let Some(node) = current {
        let data = doc.element(node)?;
        if data.ns != Namespace::None && data.prefix.as_deref() == prefix {
            return doc.namespace_uri(data.ns).map(str::to_owned);
        }
        let declared = data.attrs.iter().find(|a| {
            a.ns == Namespace::Xmlns
                && match prefix {
                    Some(prefix) => a.prefix.as_deref() == Some("xmlns") && a.local() == prefix,
                    None => a.prefix.is_none() && a.local() == "xmlns",
                }
        });
        if let Some(attribute) = declared {
            return (!attribute.value.is_empty()).then(|| attribute.value.clone());
        }
        current = parent_element(doc, node);
    }
    None
}

pub(super) fn lookup_namespace_uri(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let prefix = namespace_arg(vm, &arg(args, 0))?;
    let prefix = (!prefix.is_empty()).then_some(prefix);
    let found = host(vm).and_then(|h| {
        let start = lookup_start(&h.doc, node)?;
        locate_namespace(&h.doc, start, prefix.as_deref())
    });
    Ok(found.map_or(Value::Null, |uri| Value::string(&uri)))
}

pub(super) fn is_default_namespace(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let namespace = namespace_arg(vm, &arg(args, 0))?;
    let default = host(vm).and_then(|h| {
        let start = lookup_start(&h.doc, node)?;
        locate_namespace(&h.doc, start, None)
    });
    Ok(Value::Bool(default.unwrap_or_default() == namespace))
}

pub(super) fn lookup_prefix(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let namespace = namespace_arg(vm, &arg(args, 0))?;
    if namespace.is_empty() {
        return Ok(Value::Null);
    }
    let found = host(vm).and_then(|h| {
        let doc = &h.doc;
        let mut current = lookup_start(doc, node);
        while let Some(element) = current {
            let data = doc.element(element)?;
            if doc.namespace_uri(data.ns) == Some(namespace.as_str())
                && let Some(prefix) = &data.prefix
            {
                return Some(prefix.clone());
            }
            let declared = data
                .attrs
                .iter()
                .find(|a| a.prefix.as_deref() == Some("xmlns") && a.value == namespace);
            if let Some(attribute) = declared {
                return Some(attribute.local().to_owned());
            }
            current = parent_element(doc, element);
        }
        None
    });
    Ok(found.map_or(Value::Null, |prefix| Value::string(&prefix)))
}
