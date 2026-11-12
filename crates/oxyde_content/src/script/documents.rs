//! Documents supplémentaires et nœuds créés hors de l'analyseur :
//! `DOMImplementation`, les constructeurs `Document`, `DocumentFragment`,
//! `Text` et `Comment`, les sections CDATA et les instructions de traitement
//! (DOM Standard §4.5 à §4.12).
//!
//! Tous les documents d'une page vivent dans la même arène : un nœud passe de
//! l'un à l'autre en changeant de propriétaire, sans copie.

use oxyde_dom::{Namespace, NodeId};
use oxyde_js::{JsResult, Value, Vm};

use super::{arg, dom_exception, host, node_arg, node_of, this_node, wrap, wrap_opt};

/// Emplacement qui relie un objet `DOMImplementation` à son document.
const DOCUMENT: &str = "implémentation.document";
/// Emplacement qui garde l'objet `DOMImplementation` d'un document.
const IMPLEMENTATION: &str = "document.implémentation";

/// Nature d'un document autre que le document HTML de la page.
#[derive(Clone, Debug)]
pub(super) struct DocumentKind {
    /// Type MIME rendu par `contentType`.
    pub(super) content_type: &'static str,
    /// Vrai pour un document HTML (noms d'éléments en minuscules…).
    pub(super) html: bool,
    /// Vrai si l'objet est un `XMLDocument` (`createDocument`).
    pub(super) xml_interface: bool,
}

fn index(node: NodeId) -> u32 {
    u32::try_from(node.index()).unwrap_or(u32::MAX)
}

/// Document auquel appartient `node` (lui-même si c'est un document).
pub(super) fn document_of(vm: &mut Vm, node: NodeId) -> NodeId {
    host(vm).map_or(node, |h| {
        if h.doc.is_document(node) {
            node
        } else {
            h.doc.owner(node)
        }
    })
}

/// Vrai si `document` est un document HTML.
pub(super) fn is_html_document(vm: &mut Vm, document: NodeId) -> bool {
    host(vm).is_none_or(|h| h.kinds.get(&index(document)).is_none_or(|kind| kind.html))
}

fn new_document(vm: &mut Vm, kind: DocumentKind) -> Option<NodeId> {
    let h = host(vm)?;
    let document = h.doc.create_document();
    h.kinds.insert(index(document), kind);
    Some(document)
}

fn require_new(vm: &mut Vm, name: &str) -> JsResult<()> {
    if vm.is_constructing() {
        Ok(())
    } else {
        Err(vm.error(
            "TypeError",
            &format!("{name} doit être appelé avec « new »"),
        ))
    }
}

// ---- Constructeurs ---------------------------------------------------------------------

pub(super) fn document_ctor(vm: &mut Vm, _this: &Value, _args: &[Value]) -> JsResult {
    require_new(vm, "Document")?;
    let document = new_document(
        vm,
        DocumentKind {
            content_type: "application/xml",
            html: false,
            xml_interface: false,
        },
    );
    Ok(wrap_opt(vm, document))
}

pub(super) fn fragment_ctor(vm: &mut Vm, _this: &Value, _args: &[Value]) -> JsResult {
    require_new(vm, "DocumentFragment")?;
    let node = host(vm).map(|h| h.doc.create_fragment());
    Ok(wrap_opt(vm, node))
}

/// Texte passé à un constructeur : chaîne vide si l'argument manque.
fn data_arg(vm: &mut Vm, args: &[Value]) -> JsResult<String> {
    match arg(args, 0) {
        Value::Undefined => Ok(String::new()),
        other => Ok(vm.to_string(&other)?.to_string()),
    }
}

pub(super) fn text_ctor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    require_new(vm, "Text")?;
    let data = data_arg(vm, args)?;
    let node = host(vm).map(|h| h.doc.create_text(&data));
    Ok(wrap_opt(vm, node))
}

pub(super) fn comment_ctor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    require_new(vm, "Comment")?;
    let data = data_arg(vm, args)?;
    let node = host(vm).map(|h| h.doc.create_comment(&data));
    Ok(wrap_opt(vm, node))
}

// ---- Création par un document -------------------------------------------------------------

/// Rattache `node`, tout juste créé, au document `this`.
pub(super) fn created_by(vm: &mut Vm, this: &Value, node: Option<NodeId>) -> Value {
    if let (Some(node), Some(document)) = (node, node_of(vm, this))
        && let Some(h) = host(vm)
        && h.doc.is_document(document)
    {
        h.doc.set_owner(node, document);
    }
    wrap_opt(vm, node)
}

pub(super) fn create_text_node(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let text = vm.to_string(&arg(args, 0))?;
    let node = host(vm).map(|h| h.doc.create_text(&text));
    Ok(created_by(vm, this, node))
}

pub(super) fn create_comment(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let text = vm.to_string(&arg(args, 0))?;
    let node = host(vm).map(|h| h.doc.create_comment(&text));
    Ok(created_by(vm, this, node))
}

pub(super) fn create_document_fragment(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let node = host(vm).map(|h| h.doc.create_fragment());
    Ok(created_by(vm, this, node))
}

pub(super) fn create_cdata_section(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let document = this_node(vm, this)?;
    let text = vm.to_string(&arg(args, 0))?;
    if is_html_document(vm, document) {
        return Err(dom_exception(
            vm,
            "NotSupportedError",
            "un document HTML n'a pas de section CDATA",
        ));
    }
    if text.contains("]]>") {
        return Err(dom_exception(
            vm,
            "InvalidCharacterError",
            "une section CDATA ne contient pas « ]]> »",
        ));
    }
    let node = host(vm).map(|h| h.doc.create_cdata(&text));
    Ok(created_by(vm, this, node))
}

/// Vrai si `name` est un nom local d'élément valide (DOM Standard,
/// « valid element local name »).
pub(super) fn is_element_local_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() => {
            !chars.any(|c| c.is_ascii_whitespace() || matches!(c, '\0' | '/' | '>'))
        }
        Some(first) if matches!(first, ':' | '_') || first >= '\u{80}' => chars.all(|c| {
            c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | ':' | '_') || c >= '\u{80}'
        }),
        _ => false,
    }
}

/// Vrai si `name` suit la production `Name` de XML.
pub(super) fn is_xml_name(name: &str) -> bool {
    let start = |c: char| {
        c == ':'
            || c == '_'
            || c.is_ascii_alphabetic()
            || matches!(c,
                '\u{C0}'..='\u{D6}' | '\u{D8}'..='\u{F6}' | '\u{F8}'..='\u{2FF}'
                | '\u{370}'..='\u{37D}' | '\u{37F}'..='\u{1FFF}' | '\u{200C}'..='\u{200D}'
                | '\u{2070}'..='\u{218F}' | '\u{2C00}'..='\u{2FEF}' | '\u{3001}'..='\u{D7FF}'
                | '\u{F900}'..='\u{FDCF}' | '\u{FDF0}'..='\u{FFFD}' | '\u{10000}'..='\u{EFFFF}')
    };
    let rest = |c: char| {
        start(c)
            || c == '-'
            || c == '.'
            || c.is_ascii_digit()
            || matches!(c, '\u{B7}' | '\u{300}'..='\u{36F}' | '\u{203F}'..='\u{2040}')
    };
    let mut chars = name.chars();
    chars.next().is_some_and(start) && chars.all(rest)
}

pub(super) fn create_processing_instruction(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let target = vm.to_string(&arg(args, 0))?;
    let data = vm.to_string(&arg(args, 1))?;
    if !is_xml_name(&target) || data.contains("?>") {
        return Err(dom_exception(
            vm,
            "InvalidCharacterError",
            "cible ou contenu invalide pour une instruction de traitement",
        ));
    }
    let node = host(vm).map(|h| h.doc.create_processing_instruction(&target, &data));
    Ok(created_by(vm, this, node))
}

// ---- importNode, adoptNode, cloneNode --------------------------------------------------------

pub(super) fn clone_node(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let deep = arg(args, 0).truthy();
    let owner = document_of(vm, node);
    let copy = clone_into(vm, node, deep, owner);
    Ok(wrap_opt(vm, copy))
}

/// Copie `node` dans le document `owner` ; la copie d'un document garde sa
/// nature (HTML ou XML).
fn clone_into(vm: &mut Vm, node: NodeId, deep: bool, owner: NodeId) -> Option<NodeId> {
    let h = host(vm)?;
    let copy = h.doc.clone_subtree(node, deep, owner);
    if h.doc.is_document(node) {
        let kind = h.kinds.get(&index(node)).cloned().unwrap_or(DocumentKind {
            content_type: "text/html",
            html: true,
            xml_interface: false,
        });
        h.kinds.insert(index(copy), kind);
    }
    Some(copy)
}

/// `importNode(node, deep)` : une copie de `node` appartenant à ce document.
pub(super) fn import_node(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let document = this_node(vm, this)?;
    let node = node_arg(vm, &arg(args, 0))?;
    // Second argument : booléen, ou dictionnaire `{ selfOnly }`.
    let deep = match arg(args, 1) {
        Value::Object(_) => !vm.get(&arg(args, 1), "selfOnly")?.truthy(),
        other => other.truthy(),
    };
    if host(vm).is_some_and(|h| h.doc.is_document(node)) {
        return Err(dom_exception(
            vm,
            "NotSupportedError",
            "un document ne s'importe pas",
        ));
    }
    let copy = clone_into(vm, node, deep, document);
    Ok(wrap_opt(vm, copy))
}

/// `adoptNode(node)` : détache le nœud et le rattache à ce document.
pub(super) fn adopt_node(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let document = this_node(vm, this)?;
    let node = node_arg(vm, &arg(args, 0))?;
    if host(vm).is_some_and(|h| h.doc.is_document(node)) {
        return Err(dom_exception(
            vm,
            "NotSupportedError",
            "un document ne peut pas être adopté",
        ));
    }
    if let Some(h) = host(vm) {
        h.doc.detach(node);
        h.doc.set_owner(node, document);
        h.dirty = true;
    }
    Ok(arg(args, 0))
}

// ---- DOMImplementation --------------------------------------------------------------------

/// `document.implementation` : toujours le même objet pour un document.
pub(super) fn implementation(vm: &mut Vm, document: NodeId) -> Value {
    let wrapper = wrap(vm, document);
    let Some(wrapper_id) = wrapper.as_object() else {
        return Value::Null;
    };
    if let Some(existing) = vm.slot(wrapper_id, IMPLEMENTATION) {
        return existing;
    }
    let object = vm.new_object();
    if let Some(proto) = host(vm).and_then(|h| h.protos.get("DOMImplementation").copied()) {
        let _ = vm.set_prototype(object, Some(proto));
    }
    vm.set_slot(object, DOCUMENT, wrapper);
    vm.set_slot(wrapper_id, IMPLEMENTATION, Value::Object(object));
    Value::Object(object)
}

/// Document d'un objet `DOMImplementation`.
fn implementation_document(vm: &mut Vm, this: &Value) -> JsResult<NodeId> {
    let document = this
        .as_object()
        .and_then(|id| vm.slot(id, DOCUMENT))
        .and_then(|wrapper| node_of(vm, &wrapper));
    match document {
        Some(document) => Ok(document),
        None => Err(vm.error("TypeError", "receveur attendu : un DOMImplementation")),
    }
}

pub(super) fn create_document_type(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let document = implementation_document(vm, this)?;
    if args.len() < 3 {
        return Err(vm.error("TypeError", "createDocumentType attend trois arguments"));
    }
    let name = vm.to_string(&arg(args, 0))?;
    let public_id = vm.to_string(&arg(args, 1))?;
    let system_id = vm.to_string(&arg(args, 2))?;
    // Le standard n'exige plus qu'un nom sans espace, « > » ni caractère nul.
    if name
        .chars()
        .any(|c| c.is_ascii_whitespace() || c == '>' || c == '\0')
    {
        return Err(dom_exception(
            vm,
            "InvalidCharacterError",
            "nom de type de document invalide",
        ));
    }
    let node = host(vm).map(|h| {
        let node = h.doc.create_doctype(&name, &public_id, &system_id);
        h.doc.set_owner(node, document);
        node
    });
    Ok(wrap_opt(vm, node))
}

pub(super) fn create_document(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    implementation_document(vm, this)?;
    if args.len() < 2 {
        return Err(vm.error("TypeError", "createDocument attend deux arguments"));
    }
    let namespace = match arg(args, 0) {
        Value::Null | Value::Undefined => String::new(),
        other => vm.to_string(&other)?.to_string(),
    };
    // `[LegacyNullToEmptyString]` : `null` vaut la chaîne vide.
    let qualified = match arg(args, 1) {
        Value::Null => String::new(),
        other => vm.to_string(&other)?.to_string(),
    };
    let doctype = match arg(args, 2) {
        Value::Null | Value::Undefined => None,
        other => Some(node_arg(vm, &other)?),
    };
    let content_type = match namespace.as_str() {
        "http://www.w3.org/1999/xhtml" => "application/xhtml+xml",
        "http://www.w3.org/2000/svg" => "image/svg+xml",
        _ => "application/xml",
    };
    let Some(document) = new_document(
        vm,
        DocumentKind {
            content_type,
            html: false,
            xml_interface: true,
        },
    ) else {
        return Ok(Value::Null);
    };
    let wrapper = wrap(vm, document);
    let element = if qualified.is_empty() {
        None
    } else {
        let created = super::attrs::create_element_ns(
            vm,
            &wrapper,
            &[arg(args, 0), Value::string(&qualified)],
        )?;
        node_of(vm, &created)
    };
    if let Some(h) = host(vm) {
        if let Some(doctype) = doctype {
            h.doc.append_child(document, doctype);
        }
        if let Some(element) = element {
            h.doc.append_child(document, element);
        }
    }
    Ok(wrapper)
}

pub(super) fn create_html_document(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    implementation_document(vm, this)?;
    let title = match arg(args, 0) {
        Value::Undefined => None,
        other => Some(vm.to_string(&other)?.to_string()),
    };
    let Some(document) = new_document(
        vm,
        DocumentKind {
            content_type: "text/html",
            html: true,
            xml_interface: false,
        },
    ) else {
        return Ok(Value::Null);
    };
    if let Some(h) = host(vm) {
        let doc = &mut h.doc;
        let element = |doc: &mut oxyde_dom::Document, name: &str, parent: NodeId| {
            let node = doc.create_element(name, Namespace::Html, Vec::new());
            doc.append_child(parent, node);
            node
        };
        let doctype = doc.create_doctype("html", "", "");
        doc.append_child(document, doctype);
        let html = element(doc, "html", document);
        let head = element(doc, "head", html);
        if let Some(title) = title {
            let node = element(doc, "title", head);
            let text = doc.create_text(&title);
            doc.append_child(node, text);
        }
        element(doc, "body", html);
    }
    Ok(wrap(vm, document))
}

/// `hasFeature()` : toujours vrai, par compatibilité.
pub(super) fn has_feature(_vm: &mut Vm, _this: &Value, _args: &[Value]) -> JsResult {
    Ok(Value::Bool(true))
}
