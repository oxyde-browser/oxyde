//! Liaison entre le moteur JavaScript et le DOM : objets `window`,
//! `document`, nœuds, événements et minuteurs.
//!
//! Les interfaces (prototypes, constructeurs, constantes, répartition et
//! arité des opérations) viennent de la table produite par `oxyde_bindings` à
//! partir des fichiers Web IDL du dossier `idl/`. Le corps des opérations et
//! les attributs restent écrits à la main dans ce module.
//!
//! LIMITES CONNUES : les modules sont chargés entièrement avant toute
//! exécution et sans contrôle CORS ; `fetch` se limite à `GET` sans en-têtes ni corps, et la
//! réponse est lue d'un bloc (pas de flux, `text()` et `json()` seulement) ;
//! pas de `XMLHttpRequest`, `localStorage`, `MutationObserver`
//! ni `getBoundingClientRect` réel ; `getElementsBy…` et `querySelectorAll`
//! renvoient des tableaux figés ; `document.write` n'existe pas ; les scripts
//! s'exécutent après l'analyse complète du document, dans l'ordre du source ;
//! les enveloppes de nœuds ne sont jamais libérées tant que la page vit.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use oxyde_bindings::InterfaceDescriptor;
use oxyde_css::selector::{Selector, parse_selector_list};
use oxyde_css::syntax::tokenize;
use oxyde_dom::{Document, Namespace, NodeData, NodeId};
use oxyde_js::{ConsoleMessage, HostHooks, JsResult, NativeFn, ObjId, Value, Vm};
use url::Url;

const CLASS_NODE: u32 = 1;
const CLASS_STYLE: u32 = 2;
const CLASS_LIST: u32 = 3;

/// Cible « fenêtre » dans la table des écouteurs.
const WINDOW: u32 = u32::MAX;

/// Instructions accordées à une tâche (script, événement, minuteur) avant interruption.
const TASK_BUDGET: u64 = 200_000_000;
/// Nombre maximal de minuteurs exécutés par appel de [`ScriptEngine::run_timers`].
const MAX_TIMERS_PER_TICK: usize = 500;
/// Nombre maximal de minuteurs en attente.
const MAX_TIMERS: usize = 10_000;
/// Délai minimal d'un minuteur répété, pour ne pas monopoliser le fil.
const MIN_INTERVAL: Duration = Duration::from_millis(4);

struct Listener {
    target: u32,
    kind: String,
    callback: Value,
}

struct Timer {
    id: u32,
    due: Instant,
    callback: Value,
    args: Vec<Value>,
    repeat: Option<Duration>,
}

/// État de l'hôte vu par les fonctions natives.
struct DomHost {
    doc: Document,
    url: Url,
    wrappers: HashMap<u32, ObjId>,
    node_proto: ObjId,
    style_proto: ObjId,
    class_list_proto: ObjId,
    /// Prototype de chaque interface décrite par l'IDL, par nom.
    protos: HashMap<&'static str, ObjId>,
    listeners: Vec<Listener>,
    timers: Vec<Timer>,
    next_timer: u32,
    /// Vrai si le DOM a changé depuis la dernière mise en page.
    dirty: bool,
    viewport: (f32, f32),
    started: Instant,
    /// Requêtes `fetch` que l'embarqueur n'a pas encore prises en charge.
    fetch_queue: Vec<crate::FetchRequest>,
    /// Requêtes `fetch` en attente de réponse, par identifiant.
    fetches: HashMap<u64, PendingFetch>,
    /// Texte des modules importables, par adresse.
    modules: HashMap<String, String>,
}

fn host(vm: &mut Vm) -> Option<&mut DomHost> {
    vm.host.as_mut()?.downcast_mut::<DomHost>()
}

fn arg(args: &[Value], i: usize) -> Value {
    args.get(i).cloned().unwrap_or(Value::Undefined)
}

fn string(s: &str) -> JsResult {
    Ok(Value::string(s))
}

#[expect(clippy::cast_precision_loss, reason = "petits entiers")]
fn number(n: usize) -> Value {
    Value::Number(n as f64)
}

/// Nœud désigné par une enveloppe JavaScript.
fn node_of(vm: &mut Vm, value: &Value) -> Option<NodeId> {
    let (class, data) = vm.host_data(value)?;
    if class != CLASS_NODE {
        return None;
    }
    host(vm)?.doc.id_from_index(usize::try_from(data).ok()?)
}

fn this_node(vm: &mut Vm, this: &Value) -> JsResult<NodeId> {
    match node_of(vm, this) {
        Some(node) => Ok(node),
        None => Err(vm.error(
            "TypeError",
            "cette méthode attend un nœud du DOM comme receveur",
        )),
    }
}

/// Enveloppe JavaScript du nœud `node` (toujours la même pour un nœud donné).
fn wrap(vm: &mut Vm, node: NodeId) -> Value {
    let index = u32::try_from(node.index()).unwrap_or(u32::MAX);
    let Some(h) = host(vm) else {
        return Value::Null;
    };
    if let Some(id) = h.wrappers.get(&index) {
        return Value::Object(*id);
    }
    // L'enveloppe prend le prototype de l'interface du nœud.
    let interface = match &h.doc.node(node).data {
        NodeData::Document => "Document",
        NodeData::Doctype { .. } => "DocumentType",
        NodeData::Element(element) if element.ns == Namespace::Html => "HTMLElement",
        NodeData::Element(_) => "Element",
        NodeData::Text(_) => "Text",
        NodeData::Comment(_) => "Comment",
    };
    let proto = h.protos.get(interface).copied().unwrap_or(h.node_proto);
    let id = vm.new_host_object(proto, CLASS_NODE, u64::from(index));
    vm.add_root(Value::Object(id));
    if let Some(h) = host(vm) {
        h.wrappers.insert(index, id);
    }
    Value::Object(id)
}

fn wrap_opt(vm: &mut Vm, node: Option<NodeId>) -> Value {
    node.map_or(Value::Null, |n| wrap(vm, n))
}

fn wrap_all(vm: &mut Vm, nodes: Vec<NodeId>) -> Value {
    let items = nodes.into_iter().map(|n| wrap(vm, n)).collect();
    vm.new_array(items)
}

// ---- Sérialisation et analyse de fragments -------------------------------------

const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track",
    "wbr",
];

fn escape(text: &str, attribute: bool, out: &mut String) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' if !attribute => out.push_str("&lt;"),
            '>' if !attribute => out.push_str("&gt;"),
            '"' if attribute => out.push_str("&quot;"),
            '\u{a0}' => out.push_str("&nbsp;"),
            c => out.push(c),
        }
    }
}

/// Sérialise `node` (et ses descendants) en HTML.
fn serialize(doc: &Document, node: NodeId, out: &mut String) {
    // Parcours itératif : `Some(nœud)` à ouvrir, `None` + nom à fermer.
    let mut stack: Vec<Result<NodeId, String>> = vec![Ok(node)];
    while let Some(item) = stack.pop() {
        let current = match item {
            Ok(n) => n,
            Err(name) => {
                out.push_str("</");
                out.push_str(&name);
                out.push('>');
                continue;
            }
        };
        match &doc.node(current).data {
            NodeData::Text(text) => {
                let raw = doc
                    .node(current)
                    .parent
                    .and_then(|p| doc.element(p))
                    .is_some_and(|e| matches!(e.name.as_str(), "script" | "style"));
                if raw {
                    out.push_str(text)
                } else {
                    escape(text, false, out)
                }
            }
            NodeData::Comment(text) => {
                out.push_str("<!--");
                out.push_str(text);
                out.push_str("-->");
            }
            NodeData::Element(element) => {
                out.push('<');
                out.push_str(&element.name);
                for attr in &element.attrs {
                    out.push(' ');
                    out.push_str(&attr.name);
                    out.push_str("=\"");
                    escape(&attr.value, true, out);
                    out.push('"');
                }
                out.push('>');
                if VOID_ELEMENTS.contains(&element.name.as_str()) {
                    continue;
                }
                stack.push(Err(element.name.clone()));
                let children: Vec<NodeId> = doc.children(current).collect();
                stack.extend(children.into_iter().rev().map(Ok));
            }
            NodeData::Document | NodeData::Doctype { .. } => {
                let children: Vec<NodeId> = doc.children(current).collect();
                stack.extend(children.into_iter().rev().map(Ok));
            }
        }
    }
}

fn inner_html(doc: &Document, node: NodeId) -> String {
    let mut out = String::new();
    for child in doc.children(node) {
        serialize(doc, child, &mut out);
    }
    out
}

/// Remplace les enfants de `node` par le résultat de l'analyse de `html`.
fn set_inner_html(doc: &mut Document, node: NodeId, html: &str) {
    // LIMITE : le fragment est analysé comme le contenu d'un `<body>`, quel
    // que soit l'élément de contexte.
    let fragment = oxyde_html::parse_document(&format!("<!DOCTYPE html><body>{html}"));
    doc.remove_children(node);
    if let Some(body) = fragment.find_html_element(fragment.root(), "body") {
        let children: Vec<NodeId> = fragment.children(body).collect();
        for child in children {
            let copy = doc.import(&fragment, child);
            doc.append_child(node, copy);
        }
    }
}

fn selectors(vm: &mut Vm, source: &Value) -> JsResult<Vec<Selector>> {
    let text = vm.to_string(source)?;
    match parse_selector_list(&tokenize(&text)) {
        Some(list) => Ok(list),
        None => Err(vm.error("SyntaxError", &format!("sélecteur invalide : {text}"))),
    }
}

fn is_element(doc: &Document, node: NodeId) -> bool {
    doc.element(node).is_some()
}

fn element_sibling(doc: &Document, node: NodeId, forward: bool) -> Option<NodeId> {
    let step = |n: NodeId| {
        if forward {
            doc.node(n).next_sibling
        } else {
            doc.node(n).prev_sibling
        }
    };
    std::iter::successors(step(node), |n| step(*n)).find(|n| is_element(doc, *n))
}

// ---- Attributs de style et de classe ---------------------------------------------

/// `backgroundColor` → `background-color`.
fn css_name(property: &str) -> String {
    if property.contains('-') {
        return property.to_owned();
    }
    let mut out = String::new();
    for c in property.chars() {
        if c.is_ascii_uppercase() {
            out.push('-');
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    if out == "css-float" {
        "float".to_owned()
    } else {
        out
    }
}

fn style_declarations(doc: &Document, node: NodeId) -> Vec<(String, String)> {
    let text = doc
        .element(node)
        .and_then(|e| e.attr("style"))
        .unwrap_or("");
    text.split(';')
        .filter_map(|decl| {
            let (name, value) = decl.split_once(':')?;
            let (name, value) = (name.trim().to_ascii_lowercase(), value.trim());
            (!name.is_empty() && !value.is_empty()).then(|| (name, value.to_owned()))
        })
        .collect()
}

fn write_style(doc: &mut Document, node: NodeId, declarations: &[(String, String)]) {
    let text: Vec<String> = declarations
        .iter()
        .map(|(n, v)| format!("{n}: {v}"))
        .collect();
    if text.is_empty() {
        doc.remove_attr(node, "style");
    } else {
        doc.set_attr(node, "style", &text.join("; "));
    }
}

fn set_style_property(vm: &mut Vm, node: NodeId, name: &str, value: &str) {
    let Some(h) = host(vm) else { return };
    let mut declarations = style_declarations(&h.doc, node);
    declarations.retain(|(n, _)| n != name);
    // Une valeur contenant `;` romprait la liste de déclarations.
    if !value.trim().is_empty() && !value.contains(';') {
        declarations.push((name.to_owned(), value.trim().to_owned()));
    }
    write_style(&mut h.doc, node, &declarations);
    h.dirty = true;
}

/// Nœud d'un objet `style` ou `classList`.
fn owner_node(vm: &mut Vm, value: &Value, class: u32) -> Option<NodeId> {
    let (found, data) = vm.host_data(value)?;
    if found != class {
        return None;
    }
    host(vm)?.doc.id_from_index(usize::try_from(data).ok()?)
}

fn class_tokens(doc: &Document, node: NodeId) -> Vec<String> {
    doc.element(node)
        .map(|e| e.classes().map(str::to_owned).collect())
        .unwrap_or_default()
}

fn class_list_op(
    vm: &mut Vm,
    this: &Value,
    args: &[Value],
    op: fn(&mut Vec<String>, &str) -> bool,
) -> JsResult {
    let Some(node) = owner_node(vm, this, CLASS_LIST) else {
        return Err(vm.error("TypeError", "receveur incompatible avec classList"));
    };
    let mut names = Vec::with_capacity(args.len());
    for value in args {
        names.push(vm.to_string(value)?);
    }
    let Some(h) = host(vm) else {
        return Ok(Value::Undefined);
    };
    let mut tokens = class_tokens(&h.doc, node);
    let mut result = false;
    for name in &names {
        result = op(&mut tokens, name);
    }
    h.doc.set_attr(node, "class", &tokens.join(" "));
    h.dirty = true;
    Ok(Value::Bool(result))
}

fn class_list_add(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    class_list_op(vm, this, args, |tokens, name| {
        if !tokens.iter().any(|t| t == name) {
            tokens.push(name.to_owned());
        }
        true
    })?;
    Ok(Value::Undefined)
}

fn class_list_remove(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    class_list_op(vm, this, args, |tokens, name| {
        tokens.retain(|t| t != name);
        false
    })?;
    Ok(Value::Undefined)
}

fn class_list_toggle(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    class_list_op(vm, this, args.get(..1).unwrap_or(&[]), |tokens, name| {
        if tokens.iter().any(|t| t == name) {
            tokens.retain(|t| t != name);
            false
        } else {
            tokens.push(name.to_owned());
            true
        }
    })
}

fn class_list_contains(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let name = vm.to_string(&arg(args, 0))?;
    let Some(node) = owner_node(vm, this, CLASS_LIST) else {
        return Ok(Value::Bool(false));
    };
    let found = host(vm).is_some_and(|h| class_tokens(&h.doc, node).iter().any(|t| **t == *name));
    Ok(Value::Bool(found))
}

fn style_set_property(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let (name, value) = (vm.to_string(&arg(args, 0))?, vm.to_string(&arg(args, 1))?);
    if let Some(node) = owner_node(vm, this, CLASS_STYLE) {
        set_style_property(vm, node, &name.to_ascii_lowercase(), &value);
    }
    Ok(Value::Undefined)
}

fn style_get_property_value(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let name = vm.to_string(&arg(args, 0))?.to_ascii_lowercase();
    let Some(node) = owner_node(vm, this, CLASS_STYLE) else {
        return string("");
    };
    let value = host(vm)
        .and_then(|h| {
            style_declarations(&h.doc, node)
                .into_iter()
                .rev()
                .find(|(n, _)| *n == name)
        })
        .map(|(_, v)| v)
        .unwrap_or_default();
    string(&value)
}

fn style_remove_property(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let name = vm.to_string(&arg(args, 0))?.to_ascii_lowercase();
    if let Some(node) = owner_node(vm, this, CLASS_STYLE) {
        set_style_property(vm, node, &name, "");
    }
    Ok(Value::Undefined)
}

// ---- Crochets d'accès aux propriétés ---------------------------------------------

/// Attributs reflétés tels quels par une propriété du même nom.
const REFLECTED: &[&str] = &[
    "id",
    "value",
    "type",
    "name",
    "alt",
    "title",
    "placeholder",
    "lang",
    "rel",
    "target",
];

fn node_name(doc: &Document, node: NodeId) -> String {
    match &doc.node(node).data {
        NodeData::Document => "#document".to_owned(),
        NodeData::Doctype { name, .. } => name.clone(),
        NodeData::Text(_) => "#text".to_owned(),
        NodeData::Comment(_) => "#comment".to_owned(),
        NodeData::Element(e) if e.ns == Namespace::Html => e.name.to_ascii_uppercase(),
        NodeData::Element(e) => e.name.clone(),
    }
}

fn document_title(doc: &Document) -> String {
    doc.find_html_element(doc.root(), "title")
        .map(|n| {
            doc.text_content(n)
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default()
}

fn get_node_property(vm: &mut Vm, data: u64, key: &str) -> Option<JsResult> {
    let h = host(vm)?;
    let node = h.doc.id_from_index(usize::try_from(data).ok()?)?;
    let doc = &h.doc;
    let is_document = node == doc.root();
    let links = doc.node(node);
    let (parent, first, last, next, prev) = (
        links.parent,
        links.first_child,
        links.last_child,
        links.next_sibling,
        links.prev_sibling,
    );
    let attr = |name: &str| {
        doc.element(node)
            .and_then(|e| e.attr(name))
            .unwrap_or("")
            .to_owned()
    };
    Some(match key {
        "nodeType" => Ok(Value::Number(match &links.data {
            NodeData::Element(_) => 1.0,
            NodeData::Text(_) => 3.0,
            NodeData::Comment(_) => 8.0,
            NodeData::Document => 9.0,
            NodeData::Doctype { .. } => 10.0,
        })),
        "nodeName" => string(&node_name(doc, node)),
        "tagName" | "localName" if is_element(doc, node) => {
            let name = node_name(doc, node);
            string(&if key == "localName" {
                name.to_ascii_lowercase()
            } else {
                name
            })
        }
        "textContent" | "innerText" => {
            if is_document {
                Ok(Value::Null)
            } else {
                string(&doc.text_content(node))
            }
        }
        "nodeValue" | "data" => match &links.data {
            NodeData::Text(t) | NodeData::Comment(t) => string(t),
            _ => Ok(Value::Null),
        },
        "innerHTML" => string(&inner_html(doc, node)),
        "outerHTML" => {
            let mut out = String::new();
            serialize(doc, node, &mut out);
            string(&out)
        }
        "className" => string(&attr("class")),
        "href" | "src" if is_element(doc, node) => {
            let raw = attr(key);
            // Comme dans un navigateur, la propriété donne l'adresse absolue.
            string(&if raw.is_empty() {
                raw
            } else {
                h.url.join(&raw).map_or(raw, String::from)
            })
        }
        "hidden" if is_element(doc, node) => Ok(Value::Bool(
            doc.element(node).is_some_and(|e| e.has_attr("hidden")),
        )),
        name if REFLECTED.contains(&name) && is_element(doc, node) => string(&attr(name)),
        "childElementCount" => Ok(number(
            doc.children(node).filter(|c| is_element(doc, *c)).count(),
        )),
        "isConnected" => Ok(Value::Bool(doc.contains(doc.root(), node))),
        "parentNode" => return Some(Ok(wrap_opt(vm, parent))),
        "parentElement" => {
            let parent = parent.filter(|p| is_element(doc, *p));
            return Some(Ok(wrap_opt(vm, parent)));
        }
        "firstChild" => return Some(Ok(wrap_opt(vm, first))),
        "lastChild" => return Some(Ok(wrap_opt(vm, last))),
        "nextSibling" => return Some(Ok(wrap_opt(vm, next))),
        "previousSibling" => return Some(Ok(wrap_opt(vm, prev))),
        "firstElementChild" => {
            let found = doc.children(node).find(|c| is_element(doc, *c));
            return Some(Ok(wrap_opt(vm, found)));
        }
        "lastElementChild" => {
            let found = doc.children(node).filter(|c| is_element(doc, *c)).last();
            return Some(Ok(wrap_opt(vm, found)));
        }
        "nextElementSibling" => {
            let found = element_sibling(doc, node, true);
            return Some(Ok(wrap_opt(vm, found)));
        }
        "previousElementSibling" => {
            let found = element_sibling(doc, node, false);
            return Some(Ok(wrap_opt(vm, found)));
        }
        "children" => {
            let nodes: Vec<NodeId> = doc.children(node).filter(|c| is_element(doc, *c)).collect();
            return Some(Ok(wrap_all(vm, nodes)));
        }
        "childNodes" => {
            let nodes: Vec<NodeId> = doc.children(node).collect();
            return Some(Ok(wrap_all(vm, nodes)));
        }
        "ownerDocument" => {
            let root = doc.root();
            return Some(Ok(if is_document {
                Value::Null
            } else {
                wrap(vm, root)
            }));
        }
        "style" if is_element(doc, node) => {
            let proto = h.style_proto;
            return Some(Ok(Value::Object(vm.new_host_object(
                proto,
                CLASS_STYLE,
                data,
            ))));
        }
        "classList" if is_element(doc, node) => {
            let proto = h.class_list_proto;
            return Some(Ok(Value::Object(
                vm.new_host_object(proto, CLASS_LIST, data),
            )));
        }
        "body" | "head" if is_document => {
            let found = doc.find_html_element(doc.root(), key);
            return Some(Ok(wrap_opt(vm, found)));
        }
        "documentElement" if is_document => {
            let found = doc.children(doc.root()).find(|c| is_element(doc, *c));
            return Some(Ok(wrap_opt(vm, found)));
        }
        "title" if is_document => string(&document_title(doc)),
        "URL" | "documentURI" if is_document => string(h.url.as_str()),
        "readyState" if is_document => string("complete"),
        _ => return None,
    })
}

fn set_node_property(vm: &mut Vm, data: u64, key: &str, value: &Value) -> Option<JsResult<()>> {
    let node = host(vm)?.doc.id_from_index(usize::try_from(data).ok()?)?;
    let is_text_key = matches!(
        key,
        "textContent"
            | "innerText"
            | "innerHTML"
            | "nodeValue"
            | "data"
            | "className"
            | "href"
            | "src"
    ) || REFLECTED.contains(&key);
    if !is_text_key && key != "hidden" {
        return None;
    }
    let text = match vm.to_string(value) {
        Ok(text) => text,
        Err(thrown) => return Some(Err(thrown)),
    };
    let truthy = value.truthy();
    let h = host(vm)?;
    let doc = &mut h.doc;
    let is_document = node == doc.root();
    match key {
        "textContent" | "innerText" => match &doc.node(node).data {
            NodeData::Text(_) | NodeData::Comment(_) => doc.set_text(node, &text),
            NodeData::Element(_) => {
                doc.remove_children(node);
                doc.insert_text(node, None, &text);
            }
            _ => {}
        },
        "nodeValue" | "data" => doc.set_text(node, &text),
        "innerHTML" if !is_document => set_inner_html(doc, node, &text),
        "className" => doc.set_attr(node, "class", &text),
        "title" if is_document => {
            if let Some(title) = doc.find_html_element(doc.root(), "title") {
                doc.remove_children(title);
                doc.insert_text(title, None, &text);
            }
        }
        "hidden" => {
            if truthy {
                doc.set_attr(node, "hidden", "");
            } else {
                doc.remove_attr(node, "hidden");
            }
        }
        name => doc.set_attr(node, name, &text),
    }
    h.dirty = true;
    Some(Ok(()))
}

fn host_get(vm: &mut Vm, class: u32, data: u64, key: &str) -> Option<JsResult> {
    match class {
        CLASS_NODE => get_node_property(vm, data, key),
        CLASS_STYLE => {
            let h = host(vm)?;
            let node = h.doc.id_from_index(usize::try_from(data).ok()?)?;
            if key == "cssText" {
                return Some(string(
                    h.doc
                        .element(node)
                        .and_then(|e| e.attr("style"))
                        .unwrap_or(""),
                ));
            }
            // Les méthodes (`setProperty`…) vivent sur le prototype.
            if matches!(
                key,
                "setProperty"
                    | "getPropertyValue"
                    | "removeProperty"
                    | "constructor"
                    | "toString"
                    | "valueOf"
            ) {
                return None;
            }
            let name = css_name(key);
            let value = style_declarations(&h.doc, node)
                .into_iter()
                .rev()
                .find(|(n, _)| *n == name);
            Some(string(&value.map(|(_, v)| v).unwrap_or_default()))
        }
        CLASS_LIST => {
            let h = host(vm)?;
            let node = h.doc.id_from_index(usize::try_from(data).ok()?)?;
            match key {
                "length" => Some(Ok(number(class_tokens(&h.doc, node).len()))),
                "value" => Some(string(&class_tokens(&h.doc, node).join(" "))),
                _ => None,
            }
        }
        _ => None,
    }
}

fn host_set(vm: &mut Vm, class: u32, data: u64, key: &str, value: &Value) -> Option<JsResult<()>> {
    match class {
        CLASS_NODE => set_node_property(vm, data, key, value),
        CLASS_STYLE => {
            let text = match vm.to_string(value) {
                Ok(text) => text,
                Err(thrown) => return Some(Err(thrown)),
            };
            let node = host(vm)?.doc.id_from_index(usize::try_from(data).ok()?)?;
            if key == "cssText" {
                let h = host(vm)?;
                h.doc.set_attr(node, "style", &text);
                h.dirty = true;
            } else {
                set_style_property(vm, node, &css_name(key), &text);
            }
            Some(Ok(()))
        }
        _ => None,
    }
}

// ---- Méthodes des nœuds ------------------------------------------------------------

fn node_arg(vm: &mut Vm, value: &Value) -> JsResult<NodeId> {
    match node_of(vm, value) {
        Some(node) => Ok(node),
        None => Err(vm.error("TypeError", "un nœud du DOM est attendu en argument")),
    }
}

/// Refuse une insertion qui créerait un cycle (un nœud dans son propre sous-arbre).
fn check_insertion(vm: &mut Vm, parent: NodeId, child: NodeId) -> JsResult<()> {
    let cyclic = host(vm).is_some_and(|h| h.doc.contains(child, parent) || child == h.doc.root());
    if cyclic {
        return Err(vm.error(
            "Error",
            "HierarchyRequestError : insertion impossible à cet endroit",
        ));
    }
    Ok(())
}

fn append_child(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let parent = this_node(vm, this)?;
    let child = node_arg(vm, &arg(args, 0))?;
    check_insertion(vm, parent, child)?;
    if let Some(h) = host(vm) {
        h.doc.append_child(parent, child);
        h.dirty = true;
    }
    Ok(arg(args, 0))
}

fn insert_before(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let parent = this_node(vm, this)?;
    let child = node_arg(vm, &arg(args, 0))?;
    let reference = node_of(vm, &arg(args, 1));
    check_insertion(vm, parent, child)?;
    if let Some(h) = host(vm) {
        h.doc.insert_before(parent, child, reference);
        h.dirty = true;
    }
    Ok(arg(args, 0))
}

fn remove_child(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let parent = this_node(vm, this)?;
    let child = node_arg(vm, &arg(args, 0))?;
    let is_child = host(vm).is_some_and(|h| h.doc.node(child).parent == Some(parent));
    if !is_child {
        return Err(vm.error(
            "Error",
            "NotFoundError : le nœud n'est pas un enfant de ce parent",
        ));
    }
    if let Some(h) = host(vm) {
        h.doc.detach(child);
        h.dirty = true;
    }
    Ok(arg(args, 0))
}

fn remove(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    if let Some(h) = host(vm) {
        h.doc.detach(node);
        h.dirty = true;
    }
    Ok(Value::Undefined)
}

/// `append(...)` et `prepend(...)` : nœuds ou chaînes.
fn append_many(vm: &mut Vm, this: &Value, args: &[Value], at_start: bool) -> JsResult {
    let parent = this_node(vm, this)?;
    let reference = if at_start {
        host(vm).and_then(|h| h.doc.node(parent).first_child)
    } else {
        None
    };
    for value in args {
        let child = match node_of(vm, value) {
            Some(node) => node,
            None => {
                let text = vm.to_string(value)?;
                match host(vm) {
                    Some(h) => h.doc.create_text(&text),
                    None => return Ok(Value::Undefined),
                }
            }
        };
        check_insertion(vm, parent, child)?;
        if let Some(h) = host(vm) {
            h.doc.insert_before(parent, child, reference);
            h.dirty = true;
        }
    }
    Ok(Value::Undefined)
}

fn append(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    append_many(vm, this, args, false)
}

fn prepend(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    append_many(vm, this, args, true)
}

fn get_attribute(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let name = vm.to_string(&arg(args, 0))?.to_ascii_lowercase();
    let value = host(vm).and_then(|h| h.doc.element(node)?.attr(&name).map(str::to_owned));
    Ok(value.map_or(Value::Null, |v| Value::string(&v)))
}

fn set_attribute(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let name = vm.to_string(&arg(args, 0))?.to_ascii_lowercase();
    let value = vm.to_string(&arg(args, 1))?;
    // Un nom d'attribut vide ou contenant des blancs n'est pas sérialisable.
    if name.is_empty()
        || name.contains(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '>' | '/' | '='))
    {
        return Err(vm.error("Error", "InvalidCharacterError : nom d'attribut invalide"));
    }
    if let Some(h) = host(vm) {
        h.doc.set_attr(node, &name, &value);
        h.dirty = true;
    }
    Ok(Value::Undefined)
}

fn remove_attribute(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let name = vm.to_string(&arg(args, 0))?.to_ascii_lowercase();
    if let Some(h) = host(vm) {
        h.doc.remove_attr(node, &name);
        h.dirty = true;
    }
    Ok(Value::Undefined)
}

fn has_attribute(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let name = vm.to_string(&arg(args, 0))?.to_ascii_lowercase();
    Ok(Value::Bool(host(vm).is_some_and(|h| {
        h.doc.element(node).is_some_and(|e| e.has_attr(&name))
    })))
}

fn matching(vm: &mut Vm, scope: NodeId, list: &[Selector], first_only: bool) -> Vec<NodeId> {
    let Some(h) = host(vm) else { return Vec::new() };
    let doc = &h.doc;
    let mut found = Vec::new();
    for node in doc.descendants(scope) {
        if list.iter().any(|s| s.matches(doc, node)) {
            found.push(node);
            if first_only {
                break;
            }
        }
    }
    found
}

fn query_selector(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let scope = this_node(vm, this)?;
    let list = selectors(vm, &arg(args, 0))?;
    let found = matching(vm, scope, &list, true).into_iter().next();
    Ok(wrap_opt(vm, found))
}

fn query_selector_all(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let scope = this_node(vm, this)?;
    let list = selectors(vm, &arg(args, 0))?;
    let found = matching(vm, scope, &list, false);
    Ok(wrap_all(vm, found))
}

fn element_matches(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let list = selectors(vm, &arg(args, 0))?;
    Ok(Value::Bool(host(vm).is_some_and(|h| {
        list.iter().any(|s| s.matches(&h.doc, node))
    })))
}

fn closest(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let list = selectors(vm, &arg(args, 0))?;
    let found = host(vm).and_then(|h| {
        let doc = &h.doc;
        std::iter::once(node)
            .chain(doc.ancestors(node))
            .find(|n| list.iter().any(|s| s.matches(doc, *n)))
    });
    Ok(wrap_opt(vm, found))
}

fn get_element_by_id(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let scope = this_node(vm, this)?;
    let id = vm.to_string(&arg(args, 0))?;
    let found = host(vm).and_then(|h| {
        let doc = &h.doc;
        doc.descendants(scope)
            .find(|n| doc.element(*n).is_some_and(|e| e.id() == Some(&*id)))
    });
    Ok(wrap_opt(vm, found))
}

fn get_elements_by_tag_name(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let scope = this_node(vm, this)?;
    let name = vm.to_string(&arg(args, 0))?.to_ascii_lowercase();
    let found: Vec<NodeId> = host(vm)
        .map(|h| {
            let doc = &h.doc;
            doc.descendants(scope)
                .filter(|n| {
                    doc.element(*n)
                        .is_some_and(|e| name == "*" || e.name == name)
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(wrap_all(vm, found))
}

fn get_elements_by_class_name(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let scope = this_node(vm, this)?;
    let names = vm.to_string(&arg(args, 0))?;
    let wanted: Vec<&str> = names.split_ascii_whitespace().collect();
    let found: Vec<NodeId> = host(vm)
        .map(|h| {
            let doc = &h.doc;
            doc.descendants(scope)
                .filter(|n| {
                    doc.element(*n).is_some_and(|e| {
                        !wanted.is_empty() && wanted.iter().all(|w| e.classes().any(|c| c == *w))
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(wrap_all(vm, found))
}

fn create_element(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let name = vm.to_string(&arg(args, 0))?.to_ascii_lowercase();
    let valid = name.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    if !valid {
        return Err(vm.error("Error", "InvalidCharacterError : nom d'élément invalide"));
    }
    let node = host(vm).map(|h| h.doc.create_element(&name, Namespace::Html, Vec::new()));
    Ok(wrap_opt(vm, node))
}

fn create_text_node(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let text = vm.to_string(&arg(args, 0))?;
    let node = host(vm).map(|h| h.doc.create_text(&text));
    Ok(wrap_opt(vm, node))
}

fn contains(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let other = node_of(vm, &arg(args, 0));
    Ok(Value::Bool(other.is_some_and(|o| {
        host(vm).is_some_and(|h| h.doc.contains(node, o))
    })))
}

fn clone_node(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let deep = arg(args, 0).truthy();
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

fn no_op(_vm: &mut Vm, _this: &Value, _args: &[Value]) -> JsResult {
    Ok(Value::Undefined)
}

/// LIMITE : la géométrie de la mise en page n'est pas encore exposée aux scripts.
fn get_bounding_client_rect(vm: &mut Vm, _this: &Value, _args: &[Value]) -> JsResult {
    let rect = Value::Object(vm.new_object());
    for key in [
        "x", "y", "width", "height", "top", "left", "right", "bottom",
    ] {
        vm.set(&rect, key, Value::Number(0.0))?;
    }
    Ok(rect)
}

// ---- Événements ----------------------------------------------------------------------

fn listener_target(vm: &mut Vm, this: &Value) -> u32 {
    match node_of(vm, this) {
        Some(node) => u32::try_from(node.index()).unwrap_or(WINDOW),
        None => WINDOW,
    }
}

fn add_event_listener(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let kind = vm.to_string(&arg(args, 0))?.to_string();
    let callback = arg(args, 1);
    if callback.as_object().is_none() {
        return Ok(Value::Undefined);
    }
    let target = listener_target(vm, this);
    let Some(h) = host(vm) else {
        return Ok(Value::Undefined);
    };
    let exists = h
        .listeners
        .iter()
        .any(|l| l.target == target && l.kind == kind && l.callback == callback);
    if !exists {
        h.listeners.push(Listener {
            target,
            kind,
            callback: callback.clone(),
        });
        vm.add_root(callback);
    }
    Ok(Value::Undefined)
}

fn remove_event_listener(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let kind = vm.to_string(&arg(args, 0))?.to_string();
    let callback = arg(args, 1);
    let target = listener_target(vm, this);
    let Some(h) = host(vm) else {
        return Ok(Value::Undefined);
    };
    let before = h.listeners.len();
    h.listeners
        .retain(|l| !(l.target == target && l.kind == kind && l.callback == callback));
    if h.listeners.len() != before {
        vm.remove_root(&callback);
    }
    Ok(Value::Undefined)
}

fn event_prevent_default(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    vm.set(this, "defaultPrevented", Value::Bool(true))?;
    Ok(Value::Undefined)
}

fn event_stop_propagation(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    vm.set(this, "cancelBubble", Value::Bool(true))?;
    Ok(Value::Undefined)
}

/// Distribue un événement `kind` sur `target` puis ses ancêtres et la fenêtre
/// (phase de bouillonnement). Renvoie `true` si `preventDefault()` a été appelé.
fn dispatch(vm: &mut Vm, target: Option<NodeId>, kind: &str) -> bool {
    let event = Value::Object(vm.new_object());
    let target_value = match target {
        Some(node) => wrap(vm, node),
        None => Value::Object(vm.global()),
    };
    let prevent = Value::Object(vm.new_function("preventDefault", 0, event_prevent_default));
    let stop = Value::Object(vm.new_function("stopPropagation", 0, event_stop_propagation));
    for (key, value) in [
        ("type", Value::string(kind)),
        ("target", target_value),
        ("bubbles", Value::Bool(true)),
        ("cancelable", Value::Bool(true)),
        ("defaultPrevented", Value::Bool(false)),
        ("cancelBubble", Value::Bool(false)),
        ("preventDefault", prevent),
        ("stopPropagation", stop.clone()),
        ("stopImmediatePropagation", stop),
    ] {
        // Écriture sur un objet ordinaire tout juste créé : ne peut pas échouer.
        let _ = vm.set(&event, key, value);
    }
    // Chemin de propagation : la cible, ses ancêtres, puis la fenêtre.
    let mut path: Vec<u32> = Vec::new();
    if let (Some(node), Some(h)) = (target, host(vm)) {
        path.extend(
            std::iter::once(node)
                .chain(h.doc.ancestors(node))
                .map(|n| u32::try_from(n.index()).unwrap_or(WINDOW)),
        );
    }
    path.push(WINDOW);
    let handler_name = format!("on{kind}");
    for current in path {
        let current_value = match host(vm).and_then(|h| h.doc.id_from_index(current as usize)) {
            Some(node) if current != WINDOW => wrap(vm, node),
            _ => Value::Object(vm.global()),
        };
        let _ = vm.set(&event, "currentTarget", current_value.clone());
        // Copie de la liste : un écouteur peut en ajouter ou en retirer.
        let mut callbacks: Vec<Value> = host(vm)
            .map(|h| {
                h.listeners
                    .iter()
                    .filter(|l| l.target == current && l.kind == kind)
                    .map(|l| l.callback.clone())
                    .collect()
            })
            .unwrap_or_default();
        // Gestionnaire posé par affectation : `element.onclick = …`.
        if let Ok(handler) = vm.get(&current_value, &handler_name)
            && handler.as_object().is_some()
        {
            callbacks.push(handler);
        }
        for callback in callbacks {
            if let Err(thrown) = vm.call(&callback, &current_value, std::slice::from_ref(&event)) {
                report(vm, &thrown);
            }
            if vm.was_interrupted() {
                return false;
            }
        }
        if vm.get(&event, "cancelBubble").is_ok_and(|v| v.truthy()) {
            break;
        }
    }
    vm.get(&event, "defaultPrevented").is_ok_and(|v| v.truthy())
}

/// Signale une exception non rattrapée dans la console de la page.
fn report(vm: &mut Vm, thrown: &Value) {
    let message = vm.display(thrown);
    let console = Value::Object(vm.global());
    if let Ok(console) = vm.get(&console, "console")
        && let Ok(error) = vm.get(&console, "error")
    {
        let _ = vm.call(
            &error,
            &console,
            &[Value::string(&format!(
                "Exception non rattrapée : {message}"
            ))],
        );
    }
}

fn element_click(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    dispatch(vm, Some(node), "click");
    Ok(Value::Undefined)
}

// ---- Minuteurs et fenêtre --------------------------------------------------------------

fn add_timer(vm: &mut Vm, args: &[Value], repeat: bool, default_delay: f64) -> JsResult {
    let callback = arg(args, 0);
    if callback.as_object().is_none() {
        // Une chaîne de code à évaluer n'est pas prise en charge.
        return Ok(Value::Number(0.0));
    }
    let delay = match args.get(1) {
        Some(value) => vm.to_number(value)?,
        None => default_delay,
    };
    // Délai ramené dans [0, un jour] ; NaN compte pour zéro.
    let delay = Duration::from_secs_f64(
        if delay.is_nan() {
            0.0
        } else {
            delay.clamp(0.0, 86_400_000.0)
        } / 1000.0,
    );
    let extra = args.get(2..).unwrap_or(&[]).to_vec();
    let Some(h) = host(vm) else {
        return Ok(Value::Number(0.0));
    };
    if h.timers.len() >= MAX_TIMERS {
        return Err(vm.error("RangeError", "trop de minuteurs en attente"));
    }
    h.next_timer = h.next_timer.wrapping_add(1).max(1);
    let id = h.next_timer;
    h.timers.push(Timer {
        id,
        due: Instant::now() + delay,
        callback: callback.clone(),
        args: extra.clone(),
        repeat: repeat.then(|| delay.max(MIN_INTERVAL)),
    });
    vm.add_root(callback);
    for value in extra {
        vm.add_root(value);
    }
    Ok(Value::Number(f64::from(id)))
}

fn set_timeout(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    add_timer(vm, args, false, 0.0)
}

fn set_interval(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    add_timer(vm, args, true, 0.0)
}

fn request_animation_frame(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    add_timer(vm, args.get(..1).unwrap_or(&[]), false, 16.0)
}

fn unroot_timer(vm: &mut Vm, timer: &Timer) {
    vm.remove_root(&timer.callback);
    for value in &timer.args {
        vm.remove_root(value);
    }
}

fn clear_timer(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let id = vm.to_number(&arg(args, 0))?;
    let Some(h) = host(vm) else {
        return Ok(Value::Undefined);
    };
    if let Some(index) = h.timers.iter().position(|t| f64::from(t.id) == id) {
        let timer = h.timers.swap_remove(index);
        unroot_timer(vm, &timer);
    }
    Ok(Value::Undefined)
}

fn performance_now(vm: &mut Vm, _this: &Value, _args: &[Value]) -> JsResult {
    let elapsed = host(vm).map_or(0.0, |h| h.started.elapsed().as_secs_f64() * 1000.0);
    Ok(Value::Number(elapsed))
}

fn window_alert(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    // LIMITE : pas encore de boîte de dialogue ; le message part dans la console.
    let text = vm.to_string(&arg(args, 0))?;
    let global = Value::Object(vm.global());
    let console = vm.get(&global, "console")?;
    let log = vm.get(&console, "log")?;
    vm.call(&log, &console, &[Value::string(&format!("alert : {text}"))])
}

fn window_confirm(_vm: &mut Vm, _this: &Value, _args: &[Value]) -> JsResult {
    Ok(Value::Bool(false))
}

fn window_prompt(_vm: &mut Vm, _this: &Value, _args: &[Value]) -> JsResult {
    Ok(Value::Null)
}

fn window_inner_width(vm: &mut Vm, _this: &Value, _args: &[Value]) -> JsResult {
    Ok(Value::Number(
        host(vm).map_or(0.0, |h| f64::from(h.viewport.0)),
    ))
}

fn window_inner_height(vm: &mut Vm, _this: &Value, _args: &[Value]) -> JsResult {
    Ok(Value::Number(
        host(vm).map_or(0.0, |h| f64::from(h.viewport.1)),
    ))
}

fn define_getter(vm: &mut Vm, target: ObjId, name: &str, func: NativeFn) -> JsResult<()> {
    let global = Value::Object(vm.global());
    let object = vm.get(&global, "Object")?;
    let define = vm.get(&object, "defineProperty")?;
    let descriptor = Value::Object(vm.new_object());
    let getter = Value::Object(vm.new_function(name, 0, func));
    vm.set(&descriptor, "get", getter)?;
    vm.set(&descriptor, "configurable", Value::Bool(true))?;
    vm.call(
        &define,
        &object,
        &[Value::Object(target), Value::string(name), descriptor],
    )?;
    Ok(())
}

// ---- Interfaces décrites par l'IDL -----------------------------------------------

/// Opérations implémentées : (interface, nom, fonction). Une interface vide
/// désigne une implémentation valable pour toute interface de nœud qui
/// déclare l'opération ; l'IDL décide sur quels prototypes elle est installée
/// et avec quelle arité.
const OPERATIONS: &[(&str, &str, NativeFn)] = &[
    ("", "appendChild", append_child),
    ("", "insertBefore", insert_before),
    ("", "removeChild", remove_child),
    ("", "remove", remove),
    ("", "append", append),
    ("", "prepend", prepend),
    ("", "getAttribute", get_attribute),
    ("", "setAttribute", set_attribute),
    ("", "removeAttribute", remove_attribute),
    ("", "hasAttribute", has_attribute),
    ("", "querySelector", query_selector),
    ("", "querySelectorAll", query_selector_all),
    ("", "matches", element_matches),
    ("", "closest", closest),
    ("", "getElementById", get_element_by_id),
    ("", "getElementsByTagName", get_elements_by_tag_name),
    ("", "getElementsByClassName", get_elements_by_class_name),
    ("", "createElement", create_element),
    ("", "createTextNode", create_text_node),
    ("", "contains", contains),
    ("", "cloneNode", clone_node),
    ("", "addEventListener", add_event_listener),
    ("", "removeEventListener", remove_event_listener),
    ("", "click", element_click),
    ("", "focus", no_op),
    ("", "blur", no_op),
    ("", "scrollIntoView", no_op),
    ("", "getBoundingClientRect", get_bounding_client_rect),
    ("CSSStyleDeclaration", "setProperty", style_set_property),
    (
        "CSSStyleDeclaration",
        "getPropertyValue",
        style_get_property_value,
    ),
    (
        "CSSStyleDeclaration",
        "removeProperty",
        style_remove_property,
    ),
    ("DOMTokenList", "add", class_list_add),
    ("DOMTokenList", "remove", class_list_remove),
    ("DOMTokenList", "toggle", class_list_toggle),
    ("DOMTokenList", "contains", class_list_contains),
];

fn interface(name: &str) -> Option<&'static InterfaceDescriptor> {
    crate::dom_idl::INTERFACES.iter().find(|i| i.name == name)
}

/// Vrai si `name` est `EventTarget` ou en hérite : ses instances sont des
/// nœuds, auxquels s'appliquent les implémentations génériques.
fn is_node_interface(name: &str) -> bool {
    let mut current = interface(name);
    // Borne : une table générée à partir d'un IDL valide n'a pas de cycle.
    for _ in 0..64 {
        match current {
            Some(i) if i.name == "EventTarget" => return true,
            Some(i) => current = i.parent.and_then(interface),
            None => return false,
        }
    }
    false
}

/// Implémentation de l'opération `name` déclarée par l'interface `interface`.
fn implementation(interface: &str, name: &str) -> Option<NativeFn> {
    let find = |wanted: &str| {
        OPERATIONS
            .iter()
            .find(|(i, n, _)| *i == wanted && *n == name)
            .map(|(_, _, func)| *func)
    };
    find(interface).or_else(|| is_node_interface(interface).then(|| find("")).flatten())
}

/// Les objets interface ne se construisent pas depuis un script.
fn illegal_constructor(vm: &mut Vm, _this: &Value, _args: &[Value]) -> JsResult {
    Err(vm.error("TypeError", "constructeur illégal"))
}

/// Crée, pour chaque interface de la table générée, son prototype et son
/// objet interface global (`Node`, `Element`…), avec la chaîne d'héritage,
/// les constantes et les opérations implémentées. Renvoie les prototypes.
fn install_interfaces(vm: &mut Vm) -> JsResult<HashMap<&'static str, ObjId>> {
    let global = vm.global();
    let symbol = vm.get(&Value::Object(global), "Symbol")?;
    let tag = vm.get(&symbol, "toStringTag")?;
    let mut protos = HashMap::new();
    let mut constructors = HashMap::new();
    for interface in crate::dom_idl::INTERFACES {
        let proto = vm.new_object();
        let constructor = vm.new_function(
            interface.name,
            interface.constructor.unwrap_or(0),
            illegal_constructor,
        );
        vm.define_hidden(constructor, "prototype", Value::Object(proto));
        vm.define_hidden(proto, "constructor", Value::Object(constructor));
        if let Value::Symbol(tag) = &tag {
            vm.define_hidden_symbol(proto, tag, Value::string(interface.name));
        }
        for (name, value) in interface.constants {
            vm.define_hidden(constructor, name, Value::Number(*value));
            vm.define_hidden(proto, name, Value::Number(*value));
        }
        vm.define_hidden(global, interface.name, Value::Object(constructor));
        protos.insert(interface.name, proto);
        constructors.insert(interface.name, constructor);
    }
    for interface in crate::dom_idl::INTERFACES {
        let (Some(proto), Some(constructor)) = (
            protos.get(interface.name).copied(),
            constructors.get(interface.name).copied(),
        ) else {
            continue;
        };
        if let Some(parent) = interface.parent
            && let (Some(parent_proto), Some(parent_constructor)) = (
                protos.get(parent).copied(),
                constructors.get(parent).copied(),
            )
        {
            vm.set_prototype(proto, Some(parent_proto))?;
            vm.set_prototype(constructor, Some(parent_constructor))?;
        }
        for operation in interface.operations {
            if let Some(func) = implementation(interface.name, operation.name) {
                let target = if operation.is_static {
                    constructor
                } else {
                    proto
                };
                vm.define_method(target, operation.name, operation.length, func);
            }
        }
    }
    Ok(protos)
}

/// Installe `window`, `document` et leurs méthodes dans une machine neuve.
fn install(vm: &mut Vm, url: &Url) -> JsResult<()> {
    let protos = install_interfaces(vm)?;
    let proto_of = |name: &str| protos.get(name).copied();
    let (Some(node_proto), Some(style_proto), Some(class_list_proto)) = (
        proto_of("Node"),
        proto_of("CSSStyleDeclaration"),
        proto_of("DOMTokenList"),
    ) else {
        return Err(vm.error(
            "TypeError",
            "interfaces du DOM absentes de la table générée",
        ));
    };
    vm.host = Some(Box::new(DomHost {
        doc: Document::new(),
        url: url.clone(),
        wrappers: HashMap::new(),
        node_proto,
        style_proto,
        class_list_proto,
        protos,
        listeners: Vec::new(),
        timers: Vec::new(),
        fetch_queue: Vec::new(),
        fetches: HashMap::new(),
        modules: HashMap::new(),
        next_timer: 0,
        dirty: false,
        viewport: (0.0, 0.0),
        started: Instant::now(),
    }));
    vm.set_host_hooks(HostHooks {
        get: host_get,
        set: host_set,
    });

    let global = vm.global();
    let window = Value::Object(global);
    for name in ["window", "self", "top", "parent"] {
        vm.define_hidden(global, name, window.clone());
    }
    let window_methods: &[(&str, u32, NativeFn)] = &[
        ("fetch", 1, fetch),
        ("setTimeout", 2, set_timeout),
        ("setInterval", 2, set_interval),
        ("clearTimeout", 1, clear_timer),
        ("clearInterval", 1, clear_timer),
        ("requestAnimationFrame", 1, request_animation_frame),
        ("cancelAnimationFrame", 1, clear_timer),
        ("addEventListener", 2, add_event_listener),
        ("removeEventListener", 2, remove_event_listener),
        ("alert", 1, window_alert),
        ("confirm", 1, window_confirm),
        ("prompt", 1, window_prompt),
    ];
    for (name, arity, func) in window_methods {
        vm.define_method(global, name, *arity, *func);
    }
    define_getter(vm, global, "innerWidth", window_inner_width)?;
    define_getter(vm, global, "innerHeight", window_inner_height)?;

    let performance = vm.new_object();
    vm.define_method(performance, "now", 0, performance_now);
    vm.define_hidden(global, "performance", Value::Object(performance));

    let navigator = Value::Object(vm.new_object());
    vm.set(
        &navigator,
        "userAgent",
        Value::string("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Oxyde/0.1"),
    )?;
    vm.set(&navigator, "language", Value::string("fr-FR"))?;
    let languages = vm.new_array(vec![Value::string("fr-FR"), Value::string("fr")]);
    vm.set(&navigator, "languages", languages)?;
    vm.define_hidden(global, "navigator", navigator);

    // LIMITE : `location` est en lecture seule (l'affecter ne navigue pas).
    let location = Value::Object(vm.new_object());
    let origin = url.origin().ascii_serialization();
    let fields = [
        ("href", url.as_str().to_owned()),
        ("protocol", format!("{}:", url.scheme())),
        (
            "host",
            url.host_str().map_or(String::new(), |h| {
                url.port().map_or(h.to_owned(), |p| format!("{h}:{p}"))
            }),
        ),
        ("hostname", url.host_str().unwrap_or("").to_owned()),
        ("pathname", url.path().to_owned()),
        (
            "search",
            url.query().map_or(String::new(), |q| format!("?{q}")),
        ),
        (
            "hash",
            url.fragment().map_or(String::new(), |f| format!("#{f}")),
        ),
        ("origin", origin),
    ];
    for (key, value) in fields {
        vm.set(&location, key, Value::string(&value))?;
    }
    vm.define_hidden(global, "location", location);
    Ok(())
}

/// Moteur de script d'une page : la machine JavaScript et ses liaisons au DOM.
pub(crate) struct ScriptEngine {
    vm: Vm,
}

impl std::fmt::Debug for ScriptEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScriptEngine")
            .field("vm", &self.vm)
            .finish()
    }
}

impl ScriptEngine {
    pub(crate) fn new(url: &Url) -> Self {
        let mut vm = Vm::new();
        if let Err(thrown) = install(&mut vm, url) {
            report(&mut vm, &thrown);
        }
        Self { vm }
    }

    /// Exécute `task` avec le document `doc` prêté à la machine, sur un fil à
    /// grande pile et avec un budget d'instructions.
    fn run<T: Send>(&mut self, doc: &mut Document, task: impl FnOnce(&mut Vm) -> T + Send) -> T {
        let vm = &mut self.vm;
        if let Some(h) = host(vm) {
            std::mem::swap(&mut h.doc, doc);
        }
        // `document` désigne la racine du document prêté.
        let document = host(vm).map(|h| h.doc.root());
        if let Some(root) = document {
            let wrapper = wrap(vm, root);
            let global = vm.global();
            vm.define_hidden(global, "document", wrapper);
        }
        vm.set_budget(Some(TASK_BUDGET));
        let result = oxyde_js::with_large_stack(|| {
            let result = task(vm);
            // Fin de la tâche : les réactions de promesses en attente s'exécutent.
            vm.run_microtasks();
            result
        });
        if let Some(h) = host(vm) {
            std::mem::swap(&mut h.doc, doc);
        }
        result
    }

    /// Exécute un script classique ; une exception est signalée dans la console.
    pub(crate) fn eval(&mut self, doc: &mut Document, source: &str) {
        self.run(doc, |vm| {
            if let Err(error) = vm.eval(source) {
                let message = Value::string(&format!("Exception non rattrapée : {error}"));
                let global = Value::Object(vm.global());
                if let Ok(console) = vm.get(&global, "console")
                    && let Ok(log) = vm.get(&console, "error")
                {
                    let _ = vm.call(&log, &console, &[message]);
                }
            }
        });
    }

    /// Fournit le texte des modules que les scripts pourront importer.
    pub(crate) fn set_modules(&mut self, modules: HashMap<String, String>) {
        self.vm.set_module_loader(load_module);
        if let Some(h) = host(&mut self.vm) {
            h.modules = modules;
        }
    }

    /// Exécute un script de type module, d'identifiant (adresse) `id` ; une
    /// erreur est signalée dans la console.
    pub(crate) fn eval_module(&mut self, doc: &mut Document, source: &str, id: &str) {
        self.run(doc, |vm| {
            if let Err(error) = vm.eval_module(source, id) {
                let message = Value::string(&format!("Exception non rattrapée : {error}"));
                let global = Value::Object(vm.global());
                if let Ok(console) = vm.get(&global, "console")
                    && let Ok(log) = vm.get(&console, "error")
                {
                    let _ = vm.call(&log, &console, &[message]);
                }
            }
        });
    }

    /// Distribue l'événement `kind` sur `target` (`None` : la fenêtre).
    /// Renvoie `true` si l'action par défaut a été annulée.
    pub(crate) fn dispatch(
        &mut self,
        doc: &mut Document,
        target: Option<NodeId>,
        kind: &str,
    ) -> bool {
        self.run(doc, |vm| dispatch(vm, target, kind))
    }

    /// Exécute les minuteurs arrivés à échéance à l'instant `now`.
    pub(crate) fn run_timers(&mut self, doc: &mut Document, now: Instant) {
        if !host(&mut self.vm).is_some_and(|h| h.timers.iter().any(|t| t.due <= now)) {
            return;
        }
        self.run(doc, |vm| {
            for _ in 0..MAX_TIMERS_PER_TICK {
                let Some(h) = host(vm) else { return };
                // Le minuteur échu le plus ancien d'abord.
                let Some(index) = h
                    .timers
                    .iter()
                    .enumerate()
                    .filter(|(_, t)| t.due <= now)
                    .min_by_key(|(_, t)| (t.due, t.id))
                    .map(|(i, _)| i)
                else {
                    return;
                };
                let (callback, args) = (
                    h.timers[index].callback.clone(),
                    h.timers[index].args.clone(),
                );
                match h.timers[index].repeat {
                    Some(period) => h.timers[index].due = now + period,
                    None => {
                        let timer = h.timers.swap_remove(index);
                        unroot_timer(vm, &timer);
                    }
                }
                if let Err(thrown) = vm.call(&callback, &Value::Undefined, &args) {
                    report(vm, &thrown);
                }
                // Point de contrôle des microtâches après chaque rappel.
                vm.run_microtasks();
                if vm.was_interrupted() {
                    // Un script qui boucle perd ses minuteurs : la page reste utilisable.
                    if let Some(h) = host(vm) {
                        h.timers.clear();
                    }
                    return;
                }
            }
        });
    }

    /// Requêtes `fetch` émises par les scripts depuis le dernier appel.
    pub(crate) fn take_fetches(&mut self) -> Vec<crate::FetchRequest> {
        host(&mut self.vm)
            .map(|h| std::mem::take(&mut h.fetch_queue))
            .unwrap_or_default()
    }

    /// Conclut la requête `fetch` d'identifiant `id` et exécute la suite du
    /// script qui l'attendait.
    pub(crate) fn complete_fetch(
        &mut self,
        doc: &mut Document,
        id: u64,
        result: Result<crate::FetchResponse, String>,
    ) {
        self.run(doc, |vm| settle_fetch(vm, id, result));
    }

    /// Échéance du prochain minuteur.
    pub(crate) fn next_due(&mut self) -> Option<Instant> {
        host(&mut self.vm)?.timers.iter().map(|t| t.due).min()
    }

    /// Vrai si le DOM a changé depuis le dernier appel.
    pub(crate) fn take_dirty(&mut self) -> bool {
        host(&mut self.vm).is_some_and(|h| std::mem::take(&mut h.dirty))
    }

    pub(crate) fn set_viewport(&mut self, width: f32, height: f32) {
        if let Some(h) = host(&mut self.vm) {
            h.viewport = (width, height);
        }
    }

    pub(crate) fn take_console(&mut self) -> Vec<ConsoleMessage> {
        self.vm.take_console()
    }
}

// ---- Modules ---------------------------------------------------------------------

/// Chargeur de modules de la page : les textes ont été fournis d'avance par
/// l'embarqueur (voir `Page::pending_modules`).
fn load_module(vm: &mut Vm, specifier: &str, referrer: &str) -> Result<(String, String), String> {
    let referrer = Url::parse(referrer).map_err(|e| e.to_string())?;
    let url = crate::resolve_module(&referrer, specifier)
        .ok_or("ce spécificateur ne désigne ni un chemin ni une adresse")?;
    let id = url.to_string();
    let source = host(vm)
        .and_then(|h| h.modules.get(&id))
        .ok_or_else(|| format!("{id} n'a pas pu être chargé"))?
        .clone();
    Ok((id, source))
}

// ---- fetch -----------------------------------------------------------------------
//
// La page ne fait pas de réseau elle-même : `fetch` enregistre une requête que
// l'embarqueur exécute (voir `Page::take_fetches`), puis la réponse revient
// par `Page::complete_fetch`. La politique de même origine est appliquée ici,
// au moment de livrer la réponse au script.

/// Nombre maximal de requêtes `fetch` en cours pour une page.
const MAX_PENDING_FETCHES: usize = 64;

/// Identifiants de requêtes, uniques dans tout le processus : une réponse
/// tardive ne peut pas être prise pour celle d'une autre page.
static NEXT_FETCH_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

struct PendingFetch {
    /// Adresse demandée (avant redirections).
    url: Url,
    resolve: Value,
    reject: Value,
}

/// Rejette la promesse d'un `fetch` avec une `TypeError`.
fn reject_fetch(vm: &mut Vm, reject: &Value, message: &str) -> JsResult<()> {
    let error = vm.error("TypeError", &format!("fetch : {message}"));
    vm.call(reject, &Value::Undefined, &[error]).map(|_| ())
}

fn fetch(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let (promise, resolve, reject) = vm.promise_with_resolvers();
    // Adresse : chaîne, ou objet muni d'une propriété `url`.
    let input = arg(args, 0);
    let address = match &input {
        Value::Object(_) => {
            let url = vm.get(&input, "url")?;
            if url == Value::Undefined {
                vm.to_string(&input)?
            } else {
                vm.to_string(&url)?
            }
        }
        other => vm.to_string(other)?,
    };
    let init = arg(args, 1);
    let method = match &init {
        Value::Object(_) => match vm.get(&init, "method")? {
            Value::Undefined => "GET".to_owned(),
            other => vm.to_string(&other)?.to_ascii_uppercase(),
        },
        _ => "GET".to_owned(),
    };
    let Some(h) = host(vm) else {
        return Ok(promise);
    };
    let refusal = match h.url.join(&address) {
        _ if method != "GET" => Some(format!("la méthode {method} n'est pas prise en charge")),
        Err(error) => Some(format!("adresse invalide ({error})")),
        Ok(url) if !matches!(url.scheme(), "http" | "https") => Some(format!(
            "le protocole « {} » n'est pas accessible aux scripts",
            url.scheme()
        )),
        Ok(_) if h.fetches.len() >= MAX_PENDING_FETCHES => {
            Some("trop de requêtes en cours".to_owned())
        }
        Ok(url) => {
            let id = NEXT_FETCH_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            h.fetch_queue.push(crate::FetchRequest {
                id,
                url: url.clone(),
            });
            h.fetches.insert(
                id,
                PendingFetch {
                    url,
                    resolve: resolve.clone(),
                    reject: reject.clone(),
                },
            );
            None
        }
    };
    match refusal {
        Some(message) => reject_fetch(vm, &reject, &message)?,
        None => {
            // Les fonctions de résolution doivent survivre jusqu'à la réponse.
            vm.add_root(resolve);
            vm.add_root(reject);
        }
    }
    Ok(promise)
}

/// Politique de même origine : une réponse d'une autre origine n'est lisible
/// que si son serveur l'autorise par `Access-Control-Allow-Origin`.
///
/// LIMITE : la requête ne porte pas d'en-tête `Origin` ; un serveur qui
/// n'autorise que les origines qu'il reconnaît répondra donc sans autorisation.
fn check_same_origin(document: &Url, response: &crate::FetchResponse) -> Result<(), String> {
    let origin = document.origin();
    if origin.is_tuple() && origin == response.url.origin() {
        return Ok(());
    }
    let allowed = response
        .headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("access-control-allow-origin"))
        .any(|(_, value)| {
            let value = value.trim();
            value == "*" || (origin.is_tuple() && value == origin.ascii_serialization())
        });
    if allowed {
        Ok(())
    } else {
        Err(format!(
            "réponse de {} bloquée par la politique de même origine (pas d'en-tête Access-Control-Allow-Origin qui l'autorise)",
            response.url.host_str().unwrap_or("l'hôte distant")
        ))
    }
}

fn settle_fetch(vm: &mut Vm, id: u64, result: Result<crate::FetchResponse, String>) {
    let Some(h) = host(vm) else { return };
    let Some(pending) = h.fetches.remove(&id) else {
        // Requête inconnue : réponse destinée à une page déjà remplacée.
        return;
    };
    let document = h.url.clone();
    vm.remove_root(&pending.resolve);
    vm.remove_root(&pending.reject);
    let checked = result.and_then(|response| {
        check_same_origin(&document, &response)?;
        Ok(response)
    });
    let outcome = match checked {
        Ok(response) => make_response(vm, &document, &pending.url, response)
            .and_then(|object| vm.call(&pending.resolve, &Value::Undefined, &[object])),
        Err(message) => reject_fetch(vm, &pending.reject, &message).map(|()| Value::Undefined),
    };
    if let Err(thrown) = outcome {
        report(vm, &thrown);
    }
}

/// Objet `Response` remis au script.
fn make_response(
    vm: &mut Vm,
    document: &Url,
    requested: &Url,
    response: crate::FetchResponse,
) -> JsResult {
    let object = Value::Object(vm.new_object());
    let same_origin = document.origin() == response.url.origin();
    vm.set(
        &object,
        "ok",
        Value::Bool((200..300).contains(&response.status)),
    )?;
    vm.set(&object, "status", Value::Number(f64::from(response.status)))?;
    vm.set(&object, "statusText", Value::string(""))?;
    vm.set(&object, "url", Value::string(response.url.as_str()))?;
    vm.set(
        &object,
        "redirected",
        Value::Bool(*requested != response.url),
    )?;
    vm.set(
        &object,
        "type",
        Value::string(if same_origin { "basic" } else { "cors" }),
    )?;

    // En-têtes : paires (nom, valeur) à plat dans les données des fonctions.
    let pairs: Vec<Value> = response
        .headers
        .iter()
        .flat_map(|(name, value)| [Value::string(name), Value::string(value)])
        .collect();
    let headers = vm.new_object();
    let get = vm.new_native_data(headers_get, pairs.clone());
    let has = vm.new_native_data(headers_has, pairs);
    vm.define_hidden(headers, "get", get);
    vm.define_hidden(headers, "has", has);
    vm.set(&object, "headers", Value::Object(headers))?;

    let text = String::from_utf8_lossy(&response.body);
    let text = Value::string(text.strip_prefix('\u{feff}').unwrap_or(&text));
    let Some(id) = object.as_object() else {
        return Ok(object);
    };
    let text_fn = vm.new_native_data(response_text, vec![text.clone()]);
    let json_fn = vm.new_native_data(response_json, vec![text]);
    vm.define_hidden(id, "text", text_fn);
    vm.define_hidden(id, "json", json_fn);
    Ok(object)
}

/// Valeurs de l'en-tête `name` parmi les paires `data`, jointes par « , ».
fn header_value(vm: &mut Vm, data: &[Value], name: &Value) -> JsResult<Option<String>> {
    let name = vm.to_string(name)?;
    let values: Vec<&str> = data
        .chunks(2)
        .filter_map(|pair| match pair {
            [Value::String(n), Value::String(v)] if n.eq_ignore_ascii_case(&name) => Some(&**v),
            _ => None,
        })
        .collect();
    Ok((!values.is_empty()).then(|| values.join(", ")))
}

fn headers_get(vm: &mut Vm, data: &[Value], _this: &Value, args: &[Value]) -> JsResult {
    Ok(header_value(vm, data, &arg(args, 0))?.map_or(Value::Null, |v| Value::string(&v)))
}

fn headers_has(vm: &mut Vm, data: &[Value], _this: &Value, args: &[Value]) -> JsResult {
    Ok(Value::Bool(
        header_value(vm, data, &arg(args, 0))?.is_some(),
    ))
}

fn response_text(vm: &mut Vm, data: &[Value], _this: &Value, _args: &[Value]) -> JsResult {
    let text = data.first().cloned().unwrap_or(Value::Undefined);
    Ok(vm.promise_resolved(text))
}

fn response_json(vm: &mut Vm, data: &[Value], _this: &Value, _args: &[Value]) -> JsResult {
    let text = data.first().cloned().unwrap_or(Value::Undefined);
    let json = vm.get(&Value::Object(vm.global()), "JSON")?;
    let parse = vm.get(&json, "parse")?;
    Ok(match vm.call(&parse, &json, &[text]) {
        Ok(value) => vm.promise_resolved(value),
        Err(thrown) => vm.promise_rejected(thrown),
    })
}

// ---- Couverture du DOM -----------------------------------------------------------

/// Voir [`crate::dom_coverage_report`].
pub(crate) fn dom_coverage_report() -> String {
    use std::fmt::Write as _;

    let Ok(url) = Url::parse("https://oxyde.test/") else {
        return String::new();
    };
    let mut doc = oxyde_html::parse_document(
        "<!DOCTYPE html><title>t</title><p id=p class=c style='color: red'>texte<!--c--></p>",
    );
    let mut engine = ScriptEngine::new(&url);

    // Un membre par ligne : interface, nature (c, a, o), nom, statique.
    let mut members = String::new();
    for interface in crate::dom_idl::INTERFACES {
        let name = interface.name;
        for (constant, _) in interface.constants {
            let _ = write!(members, "['{name}','c','{constant}',1],");
        }
        for attribute in interface.attributes {
            let is_static = u8::from(attribute.is_static);
            let _ = write!(members, "['{name}','a','{}',{is_static}],", attribute.name);
        }
        for operation in interface.operations {
            let is_static = u8::from(operation.is_static);
            let _ = write!(members, "['{name}','o','{}',{is_static}],", operation.name);
        }
    }
    // Pour chaque interface, un objet d'essai ; un attribut IDL ne vaut jamais
    // `undefined`, ce qui distingue un attribut servi d'un attribut absent.
    // `style` répond à tout nom de propriété CSS : seul `cssText` y est un
    // attribut IDL réellement servi.
    let probe = format!(
        "(function () {{
           const el = document.getElementById('p'); let ev = null;
           el.addEventListener('click', e => {{ ev = e; }}); el.click();
           const kind = n => n && n.nodeType;
           const samples = {{
             Event: ev, EventTarget: el, Node: el, Element: el, HTMLElement: el, Document: document,
             DocumentType: kind(document.firstChild) === 10 ? document.firstChild : null,
             DocumentFragment: null,
             CharacterData: el.firstChild, Text: el.firstChild,
             Comment: kind(el.lastChild) === 8 ? el.lastChild : null,
             DOMTokenList: el.classList, CSSStyleDeclaration: el.style,
             NodeList: document.querySelectorAll('p'), HTMLCollection: el.children,
           }};
           let out = '';
           for (const [iface, nature, name, isStatic] of [{members}]) {{
             let ok = false;
             try {{
               const target = isStatic ? globalThis[iface] : samples[iface];
               if (target !== null && target !== undefined) {{
                 if (nature === 'c') ok = typeof target[name] === 'number';
                 else if (nature === 'o') ok = typeof target[name] === 'function';
                 else if (iface === 'CSSStyleDeclaration') ok = name === 'cssText';
                 else ok = target[name] !== undefined;
               }}
             }} catch (e) {{}}
             out += ok ? '1' : '0';
           }}
           return out;
         }})()"
    );
    let flags = engine.run(&mut doc, |vm| {
        vm.eval(&probe)
            .map(|value| vm.display(&value))
            .unwrap_or_default()
    });
    let mut flags = flags.chars().map(|c| c == '1');

    let mut rows = String::new();
    let mut missing = String::new();
    let (mut total, mut done) = (0_usize, 0_usize);
    for interface in crate::dom_idl::INTERFACES {
        let mut absent: Vec<&str> = Vec::new();
        let mut count = |names: &mut dyn Iterator<Item = &'static str>| {
            let (mut all, mut ok) = (0_usize, 0_usize);
            for name in names {
                all += 1;
                if flags.next().unwrap_or(false) {
                    ok += 1;
                } else {
                    absent.push(name);
                }
            }
            (ok, all)
        };
        let constants = count(&mut interface.constants.iter().map(|(name, _)| *name));
        let attributes = count(&mut interface.attributes.iter().map(|a| a.name));
        let operations = count(&mut interface.operations.iter().map(|o| o.name));
        total += constants.1 + attributes.1 + operations.1;
        done += constants.0 + attributes.0 + operations.0;
        let cell = |(ok, all): (usize, usize)| {
            if all == 0 {
                "—".to_owned()
            } else {
                format!("{ok} / {all}")
            }
        };
        let _ = writeln!(
            rows,
            "| `{}` | {} | {} | {} | {} |",
            interface.name,
            interface
                .parent
                .map_or("—".to_owned(), |p| format!("`{p}`")),
            cell(constants),
            cell(attributes),
            cell(operations)
        );
        if !absent.is_empty() {
            let _ = writeln!(missing, "- `{}` : {}", interface.name, absent.join(", "));
        }
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "nombres de membres petits devant 2^53"
    )]
    let rate = if total == 0 {
        0.0
    } else {
        done as f64 * 100.0 / total as f64
    };
    format!(
        "# Couverture du DOM\n\n\
         Membres des interfaces décrites par les fichiers Web IDL d'Oxyde (`crates/oxyde_content/idl/`) \
         que les scripts trouvent réellement, mesurés en interrogeant une page d'essai. \
         Généré par `cargo run -p oxyde -- --dom-coverage`.\n\n\
         La mesure dit qu'un membre existe, pas qu'il se comporte comme le standard l'exige : \
         c'est le rôle des Web Platform Tests, pas encore branchés. Les interfaces que l'IDL \
         d'Oxyde ne décrit pas encore (éléments HTML particuliers, `Window`, observateurs, plages…) \
         ne sont pas comptées.\n\n\
         **Total : {done} membres disponibles sur {total} ({rate:.1} %).**\n\n\
         | Interface | Hérite de | Constantes | Attributs | Opérations |\n|---|---|---:|---:|---:|\n{rows}\n\
         ## Membres absents\n\n{missing}"
    )
}
