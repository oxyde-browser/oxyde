//! Liaison entre le moteur JavaScript et le DOM : objets `window`,
//! `document`, nœuds, événements et minuteurs.
//!
//! Les interfaces (prototypes, constructeurs, constantes, répartition et
//! arité des opérations) viennent de la table produite par `oxyde_bindings` à
//! partir des fichiers Web IDL du dossier `idl/`. Le corps des opérations et
//! les attributs restent écrits à la main dans ce module.
//!
//! LIMITES CONNUES : les modules sont chargés entièrement avant toute
//! exécution et sans contrôle CORS ; le corps d'une requête `fetch` est pris pour du texte, et la
//! réponse est lue d'un bloc (pas de flux, `text()` et `json()` seulement) ;
//! pas de `MutationObserver`
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

mod attrs;
mod documents;
mod dom_ops;
mod events;
mod focus;
mod frames;
mod geometry;
mod lists;
mod mutation;
mod range;
mod scripts;
mod storage;
mod tokens;
mod traversal;
mod tree;

use events::{Listener, add_event_listener, dispatch, remove_event_listener};

const CLASS_NODE: u32 = 1;
const CLASS_STYLE: u32 = 2;
const CLASS_LIST: u32 = 3;
const CLASS_STORAGE: u32 = 4;
const CLASS_DATASET: u32 = 5;
const CLASS_RANGE: u32 = 6;
const CLASS_ATTRS: u32 = 7;
const CLASS_LIVE: u32 = 8;
const CLASS_COMPUTED: u32 = 9;

/// Instructions accordées par défaut à une tâche (script, événement,
/// minuteur) avant interruption : un script emballé ne fige pas l'onglet.
/// Réglable par [`ScriptEngine::set_task_budget`].
const TASK_BUDGET: u64 = 200_000_000;
/// Nombre maximal de minuteurs exécutés par appel de [`ScriptEngine::run_timers`].
const MAX_TIMERS_PER_TICK: usize = 500;
/// Nombre maximal de minuteurs en attente.
const MAX_TIMERS: usize = 10_000;
/// Délai minimal d'un minuteur répété, pour ne pas monopoliser le fil.
const MIN_INTERVAL: Duration = Duration::from_millis(4);

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
    /// Nature des documents autres que celui de la page, par index de nœud.
    kinds: HashMap<u32, documents::DocumentKind>,
    /// Collections vivantes créées par les scripts.
    lists: Vec<lists::LiveList>,
    node_proto: ObjId,
    style_proto: ObjId,
    class_list_proto: ObjId,
    /// Prototype des objets `dataset`.
    dataset_proto: ObjId,
    /// Entrées de `localStorage` (0) et `sessionStorage` (1), dans l'ordre
    /// de leur création.
    storage: [Vec<(String, String)>; 2],
    /// Prototype de chaque interface décrite par l'IDL, par nom.
    protos: HashMap<&'static str, ObjId>,
    listeners: Vec<Listener>,
    /// Observateurs de mutation en activité.
    observers: Vec<mutation::Observer>,
    /// Navigations demandées par les scripts, pas encore prises en charge
    /// par l'embarqueur.
    navigations: Vec<crate::Navigation>,
    /// Fonctions du prélude que le moteur appelle en retour.
    hooks: Option<Value>,
    /// Fonctions d'activation au clic (`before`, `after`).
    activation: Option<Value>,
    /// Script en cours d'exécution : `document.currentScript`.
    current_script: Option<NodeId>,
    /// Éléments `<script>` déjà pris en charge (index de nœud) ; `None`
    /// tant que ceux du HTML reçu n'ont pas été relevés.
    scripts_started: Option<std::collections::HashSet<usize>>,
    /// Version du document au dernier relevé des scripts insérés.
    scripts_scanned: u64,
    /// Valeur du champ de texte qui a le focus, telle qu'elle était quand il
    /// l'a reçu : pour l'événement `change` à la perte du focus.
    focus_value: Option<String>,
    /// Prototype des éléments HTML de chaque balise.
    tag_protos: HashMap<String, ObjId>,
    /// Document de la fenêtre du royaume courant (`window.document`).
    document: NodeId,
    /// Royaume dont l'état occupe les champs propres à un royaume (`url`,
    /// `document`, prototypes, fonctions du prélude).
    realm: usize,
    /// État des autres royaumes, par numéro.
    parked: HashMap<usize, RealmHost>,
    /// Royaume des documents qui ne sont pas ceux du royaume 0.
    doc_realms: HashMap<u32, usize>,
    /// Contexte de chaque cadre, par index de son élément `<iframe>`.
    frames: HashMap<u32, frames::FrameContext>,
    /// Source déjà prise en compte pour chaque `<iframe>` (voir
    /// [`frames::fingerprint`]) : un changement relance un chargement.
    frame_sources: HashMap<u32, String>,
    /// Documents de cadres à charger par l'embarqueur : élément et adresse.
    frame_requests: Vec<(NodeId, Url)>,
    /// Version du document au dernier relevé des cadres.
    frames_scanned: Option<u64>,
    /// De quoi mettre le document en page à la demande d'un script.
    layout_env: Option<crate::LayoutEnv>,
    /// Géométrie de la dernière mise en page demandée par un script.
    geometry: Option<geometry::Geometry>,
    /// Polices et tampons de texte de ces mises en page.
    text: Option<oxyde_text::TextContext>,
    /// Prototype des objets rendus par `getComputedStyle`.
    computed_proto: Option<ObjId>,
    /// Texte de `document.cookie`, fourni par l'embarqueur.
    cookies: String,
    /// Écritures dans `document.cookie` à remettre à l'embarqueur.
    cookie_writes: Vec<String>,
    /// Identifiant du prochain écouteur enregistré.
    next_listener: u64,
    timers: Vec<Timer>,
    next_timer: u32,
    /// Vrai si le DOM a changé depuis la dernière mise en page.
    dirty: bool,
    viewport: (f32, f32),
    started: Instant,
    /// Instant le plus avancé communiqué par la page : un lanceur de tests
    /// fait avancer le temps plus vite que l'horloge réelle.
    clock: Instant,
    /// Requêtes `fetch` que l'embarqueur n'a pas encore prises en charge.
    fetch_queue: Vec<crate::FetchRequest>,
    /// Requêtes `fetch` en attente de réponse, par identifiant.
    fetches: HashMap<u64, PendingFetch>,
    /// Texte des modules importables, par adresse.
    modules: HashMap<String, String>,
}

/// État du DOM propre à un royaume : un cadre a sa fenêtre, donc ses
/// prototypes d'interfaces, son adresse et son document.
struct RealmHost {
    url: Url,
    document: NodeId,
    node_proto: ObjId,
    style_proto: ObjId,
    class_list_proto: ObjId,
    dataset_proto: ObjId,
    protos: HashMap<&'static str, ObjId>,
    hooks: Option<Value>,
    activation: Option<Value>,
    tag_protos: HashMap<String, ObjId>,
}

impl DomHost {
    /// Instant courant : l'horloge réelle, ou l'instant le plus avancé que
    /// la page a communiqué s'il la devance (temps virtuel).
    fn now(&self) -> Instant {
        self.clock.max(Instant::now())
    }

    /// Millisecondes écoulées depuis la création du moteur.
    fn elapsed_ms(&self) -> f64 {
        self.now()
            .saturating_duration_since(self.started)
            .as_secs_f64()
            * 1000.0
    }

    /// Échange l'état du royaume courant avec `other`.
    fn swap_realm(&mut self, other: &mut RealmHost) {
        std::mem::swap(&mut self.url, &mut other.url);
        std::mem::swap(&mut self.document, &mut other.document);
        std::mem::swap(&mut self.node_proto, &mut other.node_proto);
        std::mem::swap(&mut self.style_proto, &mut other.style_proto);
        std::mem::swap(&mut self.class_list_proto, &mut other.class_list_proto);
        std::mem::swap(&mut self.dataset_proto, &mut other.dataset_proto);
        std::mem::swap(&mut self.protos, &mut other.protos);
        std::mem::swap(&mut self.hooks, &mut other.hooks);
        std::mem::swap(&mut self.activation, &mut other.activation);
        std::mem::swap(&mut self.tag_protos, &mut other.tag_protos);
    }

    /// Passe dans le royaume `realm`, s'il est connu.
    fn enter(&mut self, realm: usize) {
        if let Some(mut incoming) = self.parked.remove(&realm) {
            self.swap_realm(&mut incoming);
            self.parked.insert(self.realm, incoming);
            self.realm = realm;
        }
    }

    /// Enregistre l'état `fresh` d'un royaume neuf et y passe.
    fn adopt(&mut self, realm: usize, mut fresh: RealmHost) {
        self.swap_realm(&mut fresh);
        self.parked.insert(self.realm, fresh);
        self.realm = realm;
    }
}

/// État de l'hôte, réglé sur le royaume courant de la machine : une fonction
/// native s'exécute dans son royaume, et y trouve ses prototypes.
fn host(vm: &mut Vm) -> Option<&mut DomHost> {
    let realm = vm.realm();
    let h = vm.host.as_mut()?.downcast_mut::<DomHost>()?;
    if h.realm != realm {
        h.enter(realm);
    }
    Some(h)
}

/// `DOMException` de nom `name` (`HierarchyRequestError`, `NotFoundError`…).
fn dom_exception(vm: &mut Vm, name: &str, message: &str) -> Value {
    let hooks = host(vm).and_then(|h| h.hooks.clone());
    if let Some(hooks) = hooks
        && let Ok(make) = vm.get(&hooks, "domException")
        && let Ok(exception) = vm.call(
            &make,
            &hooks,
            &[Value::string(name), Value::string(message)],
        )
    {
        return exception;
    }
    // Avant l'installation du prélude : une erreur ordinaire.
    vm.error("Error", &format!("{name} : {message}"))
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
        NodeData::Document if h.kinds.get(&index).is_some_and(|k| k.xml_interface) => "XMLDocument",
        NodeData::Document => "Document",
        NodeData::CData(_) => "CDATASection",
        NodeData::ProcessingInstruction { .. } => "ProcessingInstruction",
        NodeData::Attr(_) => "Attr",
        NodeData::Doctype { .. } => "DocumentType",
        NodeData::Element(element) if element.ns == Namespace::Html => "HTMLElement",
        NodeData::Element(_) => "Element",
        NodeData::Text(_) => "Text",
        NodeData::Comment(_) => "Comment",
        NodeData::Fragment => "DocumentFragment",
    };
    // Un élément HTML prend le prototype de l'interface de sa balise.
    // … dans le royaume de son document.
    let owner = if h.doc.is_document(node) {
        node
    } else {
        h.doc.owner(node)
    };
    let realm = u32::try_from(owner.index())
        .ok()
        .and_then(|i| h.doc_realms.get(&i))
        .copied()
        .unwrap_or(0);
    let (protos, tag_protos, node_proto) = match h.parked.get(&realm) {
        Some(parked) if realm != h.realm => (&parked.protos, &parked.tag_protos, parked.node_proto),
        _ => (&h.protos, &h.tag_protos, h.node_proto),
    };
    let by_tag = match &h.doc.node(node).data {
        NodeData::Element(element) if element.ns == Namespace::Html => {
            tag_protos.get(element.name.as_str()).copied()
        }
        _ => None,
    };
    let proto = by_tag
        .or_else(|| protos.get(interface).copied())
        .unwrap_or(node_proto);
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

/// Liste de nœuds (`NodeList`) : un tableau dont le prototype porte `item`.
fn wrap_all(vm: &mut Vm, nodes: Vec<NodeId>) -> Value {
    wrap_list(vm, nodes, "NodeList")
}

/// Collection d'éléments (`HTMLCollection`) : `item` et `namedItem`.
fn wrap_collection(vm: &mut Vm, nodes: Vec<NodeId>) -> Value {
    wrap_list(vm, nodes, "HTMLCollection")
}

fn wrap_list(vm: &mut Vm, nodes: Vec<NodeId>, interface: &str) -> Value {
    let items = nodes.into_iter().map(|n| wrap(vm, n)).collect();
    let list = vm.new_array(items);
    if let (Some(id), Some(proto)) = (
        list.as_object(),
        host(vm).and_then(|h| h.protos.get(interface).copied()),
    ) {
        // Tableau tout juste créé : son prototype se change toujours.
        let _ = vm.set_prototype(id, Some(proto));
    }
    list
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
            NodeData::CData(text) => escape(text, false, out),
            NodeData::ProcessingInstruction { target, data } => {
                out.push_str("<?");
                out.push_str(target);
                out.push(' ');
                out.push_str(data);
                out.push('>');
            }
            NodeData::Attr(_) => {}
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
            NodeData::Document | NodeData::Doctype { .. } | NodeData::Fragment => {
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

/// `nodeName` de `node` ; `html_document` : son document est un document
/// HTML, où les noms des éléments HTML se lisent en majuscules.
fn node_name(doc: &Document, node: NodeId, html_document: bool) -> String {
    match &doc.node(node).data {
        NodeData::Document => "#document".to_owned(),
        NodeData::Doctype { name, .. } => name.clone(),
        NodeData::Text(_) => "#text".to_owned(),
        NodeData::Comment(_) => "#comment".to_owned(),
        NodeData::Fragment => "#document-fragment".to_owned(),
        NodeData::Element(e) if e.ns == Namespace::Html && html_document => {
            e.qualified().to_ascii_uppercase()
        }
        NodeData::Element(e) => e.qualified().into_owned(),
        NodeData::CData(_) => "#cdata-section".to_owned(),
        NodeData::ProcessingInstruction { target, .. } => target.clone(),
        NodeData::Attr(attr) => attr.qualified(),
    }
}

fn document_title(doc: &Document, document: NodeId) -> String {
    doc.find_html_element(document, "title")
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
    let is_document = doc.is_document(node);
    let html_document = u32::try_from(doc.owner(node).index())
        .ok()
        .and_then(|owner| h.kinds.get(&owner))
        .is_none_or(|kind| kind.html);
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
        "name" | "nodeName" | "localName" | "prefix" | "namespaceURI" | "value" | "nodeValue"
        | "textContent" | "specified" | "ownerElement"
            if matches!(links.data, NodeData::Attr(_)) =>
        {
            return attrs::property(vm, node, key);
        }
        "attributes" if is_element(doc, node) => {
            return Some(Ok(attrs::named_node_map(vm, node)));
        }
        "offsetWidth" | "offsetHeight" | "offsetTop" | "offsetLeft" | "offsetParent"
        | "clientWidth" | "clientHeight" | "clientTop" | "clientLeft" | "scrollWidth"
        | "scrollHeight" | "scrollTop" | "scrollLeft"
            if is_element(doc, node) =>
        {
            return geometry::property(vm, node, key);
        }
        "nodeType" => Ok(Value::Number(match &links.data {
            NodeData::Attr(_) => 2.0,
            NodeData::Element(_) => 1.0,
            NodeData::Text(_) => 3.0,
            NodeData::Comment(_) => 8.0,
            NodeData::CData(_) => 4.0,
            NodeData::ProcessingInstruction { .. } => 7.0,
            NodeData::Document => 9.0,
            NodeData::Doctype { .. } => 10.0,
            NodeData::Fragment => 11.0,
        })),
        "nodeName" => string(&node_name(doc, node, html_document)),
        "tagName" if is_element(doc, node) => string(&node_name(doc, node, html_document)),
        "localName" if is_element(doc, node) => {
            string(doc.element(node).map_or("", |e| e.name.as_str()))
        }
        "textContent" | "innerText" => match &links.data {
            NodeData::Document | NodeData::Doctype { .. } => Ok(Value::Null),
            NodeData::Comment(t) | NodeData::ProcessingInstruction { data: t, .. } => string(t),
            _ => string(&doc.text_content(node)),
        },
        "nodeValue" | "data" => match doc.character_data(node) {
            Some(t) => string(t),
            None if key == "data" => return None,
            None => Ok(Value::Null),
        },
        "target" => match &links.data {
            NodeData::ProcessingInstruction { target, .. } => string(target),
            _ => return None,
        },
        "implementation" if is_document => {
            return Some(Ok(documents::implementation(vm, node)));
        }
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
        // `value` d'une liste ou d'une zone de texte : voir `elements.js`.
        "value"
            if doc
                .element(node)
                .is_some_and(|e| e.is_html("select") || e.is_html("textarea")) =>
        {
            return None;
        }
        name if REFLECTED.contains(&name) && is_element(doc, node) => string(&attr(name)),
        "childElementCount" => Ok(number(
            doc.children(node).filter(|c| is_element(doc, *c)).count(),
        )),
        "isConnected" => Ok(Value::Bool(
            doc.is_document(doc.ancestors(node).last().unwrap_or(node)),
        )),
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
        "children" => return Some(lists::same_object(vm, node, true)),
        "dataset" if is_element(doc, node) => {
            let proto = h.dataset_proto;
            return Some(Ok(Value::Object(vm.new_host_object(
                proto,
                CLASS_DATASET,
                data,
            ))));
        }
        "childNodes" => return Some(lists::same_object(vm, node, false)),
        "ownerDocument" => {
            let root = doc.owner(node);
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
            return Some(Ok(tokens::class_list(vm, node)));
        }
        "body" | "head" if is_document => {
            let found = doc.find_html_element(node, key);
            return Some(Ok(wrap_opt(vm, found)));
        }
        "documentElement" if is_document => {
            let found = doc.children(node).find(|c| is_element(doc, *c));
            return Some(Ok(wrap_opt(vm, found)));
        }
        "title" if is_document => string(&document_title(doc, node)),
        "URL" | "documentURI" if is_document => {
            // Le document d'une fenêtre porte son adresse ; les autres sont vides.
            let realm = u32::try_from(data)
                .ok()
                .and_then(|i| h.doc_realms.get(&i))
                .copied()
                .unwrap_or(0);
            let (document, url) = match h.parked.get(&realm) {
                Some(parked) if realm != h.realm => (parked.document, &parked.url),
                _ => (h.document, &h.url),
            };
            string(if document == node {
                url.as_str()
            } else {
                "about:blank"
            })
        }
        "contentWindow" | "contentDocument"
            if doc.element(node).is_some_and(|e| e.is_html("iframe")) =>
        {
            return Some(frames::property(vm, node, key));
        }
        "readyState" if is_document => string("complete"),
        "baseURI" => string(h.url.as_str()),
        "length" if doc.character_data(node).is_some() => Ok(number(
            doc.character_data(node)
                .map_or(0, |t| t.encode_utf16().count()),
        )),
        "wholeText" if matches!(links.data, NodeData::Text(_)) => {
            string(&dom_ops::whole_text(doc, node))
        }
        "characterSet" | "charset" | "inputEncoding" if is_document => string("UTF-8"),
        "compatMode" if is_document => string("CSS1Compat"),
        "contentType" if is_document => string(
            u32::try_from(data)
                .ok()
                .and_then(|i| h.kinds.get(&i))
                .map_or("text/html", |kind| kind.content_type),
        ),
        "referrer" | "dir" if is_document => string(""),
        "cookie" if is_document => string(&h.cookies),
        "designMode" if is_document => string("off"),
        "visibilityState" if is_document => string("visible"),
        "hidden" if is_document => Ok(Value::Bool(false)),
        "domain" if is_document => string(h.url.host_str().unwrap_or("")),
        "currentScript" if is_document => {
            let current = h.current_script;
            return Some(Ok(match current {
                Some(script) => wrap(vm, script),
                None => Value::Null,
            }));
        }
        "defaultView" if is_document => return Some(Ok(Value::Object(vm.global()))),
        "doctype" if is_document => {
            let found = doc
                .children(node)
                .find(|c| matches!(doc.node(*c).data, NodeData::Doctype { .. }));
            return Some(Ok(wrap_opt(vm, found)));
        }
        "forms" | "images" | "links" | "scripts" | "embeds" | "plugins" if is_document => {
            let wanted: &[&str] = match key {
                "forms" => &["form"],
                "images" => &["img"],
                "links" => &["a", "area"],
                "scripts" => &["script"],
                _ => &["embed"],
            };
            let nodes: Vec<NodeId> = doc
                .descendants(node)
                .filter(|n| {
                    doc.element(*n).is_some_and(|e| {
                        wanted.iter().any(|name| e.is_html(name))
                            // `links` ne retient que les éléments munis de `href`.
                            && (key != "links" || e.has_attr("href"))
                    })
                })
                .collect();
            return Some(Ok(wrap_collection(vm, nodes)));
        }
        "name" | "publicId" | "systemId" if matches!(links.data, NodeData::Doctype { .. }) => {
            match &links.data {
                NodeData::Doctype {
                    name,
                    public_id,
                    system_id,
                } => string(match key {
                    "name" => name,
                    "publicId" => public_id,
                    _ => system_id,
                }),
                _ => return None,
            }
        }
        "namespaceURI" if is_element(doc, node) => Ok(doc
            .element(node)
            .and_then(|e| doc.namespace_uri(e.ns))
            .map_or(Value::Null, Value::string)),
        "prefix" if is_element(doc, node) => Ok(doc
            .element(node)
            .and_then(|e| e.prefix.as_deref())
            .map_or(Value::Null, Value::string)),
        "slot" | "dir" | "accessKey" | "nonce" if is_element(doc, node) => {
            string(&attr(&key.to_ascii_lowercase()))
        }
        "draggable" | "inert" | "autofocus" if is_element(doc, node) => {
            Ok(Value::Bool(doc.element(node).is_some_and(|e| match key {
                "draggable" => e.attr("draggable") == Some("true"),
                _ => e.has_attr(key),
            })))
        }
        "spellcheck" | "translate" if is_element(doc, node) => {
            Ok(Value::Bool(!doc.element(node).is_some_and(|e| {
                matches!(e.attr(key), Some("false" | "no"))
            })))
        }
        "tabIndex" if is_element(doc, node) => Ok(Value::Number(
            doc.element(node)
                .and_then(|e| e.attr("tabindex"))
                .and_then(|v| v.trim().parse::<f64>().ok())
                .unwrap_or(-1.0),
        )),
        "outerText" if is_element(doc, node) => string(&doc.text_content(node)),
        "accessKeyLabel" if is_element(doc, node) => string(""),
        _ => return None,
    })
}

fn set_node_property(vm: &mut Vm, data: u64, key: &str, value: &Value) -> Option<JsResult<()>> {
    let node = host(vm)?.doc.id_from_index(usize::try_from(data).ok()?)?;
    // Valeur d'un nœud d'attribut.
    if matches!(key, "value" | "nodeValue" | "textContent")
        && host(vm)?.doc.attr_data(node).is_some()
    {
        let text = match value {
            Value::Null if key != "value" => String::new(),
            other => match vm.to_string(other) {
                Ok(text) => text.to_string(),
                Err(thrown) => return Some(Err(thrown)),
            },
        };
        let h = host(vm)?;
        h.doc.set_attr_value(node, &text);
        h.dirty = true;
        return Some(Ok(()));
    }
    if matches!(key, "scrollTop" | "scrollLeft") && host(vm)?.doc.element(node).is_some() {
        return Some(geometry::set_scroll_property(vm, node, key, value));
    }
    // `value` d'une liste ou d'une zone de texte : voir `elements.js`.
    if key == "value"
        && host(vm)?
            .doc
            .element(node)
            .is_some_and(|e| e.is_html("select") || e.is_html("textarea"))
    {
        return None;
    }
    // `document.cookie = "nom=valeur; …"` : l'embarqueur tient le pot.
    if key == "cookie" && host(vm)?.doc.is_document(node) {
        let text = match vm.to_string(value) {
            Ok(text) => text.to_string(),
            Err(thrown) => return Some(Err(thrown)),
        };
        let h = host(vm)?;
        note_cookie_write(&mut h.cookies, &text);
        if h.cookie_writes.len() < MAX_COOKIE_WRITES {
            h.cookie_writes.push(text);
        }
        return Some(Ok(()));
    }
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
    // `null` vaut la chaîne vide pour le contenu textuel.
    let text: String =
        if matches!(value, Value::Null) && matches!(key, "textContent" | "nodeValue" | "data") {
            String::new()
        } else {
            text.to_string()
        };
    let truthy = value.truthy();
    let h = host(vm)?;
    let doc = &mut h.doc;
    let is_document = doc.is_document(node);
    match key {
        "textContent" | "innerText" => match &doc.node(node).data {
            NodeData::Text(_)
            | NodeData::Comment(_)
            | NodeData::CData(_)
            | NodeData::ProcessingInstruction { .. } => {
                doc.replace_data(node, 0, u32::MAX, &text);
            }
            NodeData::Element(_) | NodeData::Fragment => {
                let owner = doc.owner(node);
                let content = (!text.is_empty()).then(|| {
                    let created = doc.create_text(&text);
                    doc.set_owner(created, owner);
                    created
                });
                tree::replace_all(doc, content, node);
            }
            _ => {}
        },
        "nodeValue" | "data" => {
            doc.replace_data(node, 0, u32::MAX, &text);
        }
        "innerHTML" if !is_document => set_inner_html(doc, node, &text),
        "className" => doc.set_attr(node, "class", &text),
        "title" if is_document => {
            if let Some(title) = doc.find_html_element(node, "title") {
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
        CLASS_LIST => tokens::property(vm, data, key),
        CLASS_RANGE => range::property(vm, data, key),
        CLASS_ATTRS => attrs::map_property(vm, data, key),
        CLASS_LIVE => lists::property(vm, data, key),
        CLASS_COMPUTED => geometry::computed_property(vm, data, key),
        CLASS_STORAGE => {
            let area = usize::from(data != 0);
            match key {
                "length" => Some(Ok(storage::length(vm, area))),
                // Les méthodes ne sont jamais masquées par une entrée.
                "getItem" | "setItem" | "removeItem" | "clear" | "key" | "constructor" => None,
                _ => storage::stored(vm, area, key).map(|value| string(&value)),
            }
        }
        CLASS_DATASET => {
            let node = storage::dataset_owner(vm, data)?;
            storage::dataset_get(vm, node, key).map(|value| string(&value))
        }
        _ => None,
    }
}

/// Accès nommé sur `window` (HTML Standard §7.2.2.3) : une variable globale
/// inconnue désigne l'élément du document qui porte cet identifiant, ou
/// l'élément `embed`, `form`, `img` ou `object` qui porte ce nom.
fn named_global(vm: &mut Vm, name: &str) -> Option<Value> {
    if name.is_empty() {
        return None;
    }
    let h = host(vm)?;
    let doc = &h.doc;
    let found = doc.descendants(h.document).find(|n| {
        doc.element(*n).is_some_and(|e| {
            e.id() == Some(name)
                || (e.attr("name") == Some(name)
                    && ["embed", "form", "img", "object"]
                        .iter()
                        .any(|tag| e.is_html(tag)))
        })
    })?;
    Some(wrap(vm, found))
}

/// `cssSupports(propriété, valeur)` : le moteur de style accepte-t-il cette
/// déclaration ?
fn css_supports(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let property = vm.to_string(&arg(args, 0))?;
    let value = vm.to_string(&arg(args, 1))?;
    Ok(Value::Bool(oxyde_css::supports(&property, &value)))
}

/// Nombre maximal d'écritures de cookies gardées entre deux relevés.
const MAX_COOKIE_WRITES: usize = 1000;

/// Reporte dans `view` (le texte de `document.cookie`) l'écriture `text`,
/// en attendant que l'embarqueur rende la vue exacte de son pot : le nom
/// prend sa nouvelle valeur, ou disparaît si l'écriture le périme.
fn note_cookie_write(view: &mut String, text: &str) {
    let mut parts = text.split(';');
    let pair = parts.next().unwrap_or("").trim();
    let (name, value) = match pair.split_once('=') {
        Some((name, value)) => (name.trim(), value.trim()),
        None => ("", pair),
    };
    let mut expired = false;
    for attribute in parts {
        let (key, setting) = match attribute.split_once('=') {
            Some((key, setting)) => (key.trim().to_ascii_lowercase(), setting.trim()),
            None => (attribute.trim().to_ascii_lowercase(), ""),
        };
        match key.as_str() {
            // Un script ne pose pas de cookie `HttpOnly`.
            "httponly" => return,
            "max-age" => expired = setting.parse::<i64>().is_ok_and(|seconds| seconds <= 0),
            _ => {}
        }
    }
    if name.is_empty() && value.is_empty() {
        return;
    }
    let mut kept: Vec<String> = view
        .split("; ")
        .filter(|entry| !entry.is_empty())
        .filter(|entry| entry.split_once('=').map_or("", |(n, _)| n) != name)
        .map(str::to_owned)
        .collect();
    if !expired {
        kept.push(if name.is_empty() {
            value.to_owned()
        } else {
            format!("{name}={value}")
        });
    }
    *view = kept.join("; ");
}

/// Clés propres que l'objet hôte ajoute aux siennes.
fn host_keys(vm: &mut Vm, class: u32, data: u64) -> Vec<String> {
    match class {
        CLASS_LIVE => lists::keys(vm, data),
        CLASS_LIST => tokens::keys(vm, data),
        _ => Vec::new(),
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
        CLASS_LIST => tokens::set_property(vm, data, key, value),
        CLASS_STORAGE => {
            let text = match vm.to_string(value) {
                Ok(text) => text,
                Err(thrown) => return Some(Err(thrown)),
            };
            Some(storage::store(vm, usize::from(data != 0), key, &text))
        }
        CLASS_DATASET => {
            let text = match vm.to_string(value) {
                Ok(text) => text,
                Err(thrown) => return Some(Err(thrown)),
            };
            let node = storage::dataset_owner(vm, data)?;
            let h = host(vm)?;
            h.doc.set_attr(node, &storage::data_attribute(key), &text);
            h.dirty = true;
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

fn matching(vm: &mut Vm, scope: NodeId, list: &[Selector], first_only: bool) -> Vec<NodeId> {
    let Some(h) = host(vm) else { return Vec::new() };
    let doc = &h.doc;
    let mut found = Vec::new();
    for node in doc.descendants(scope) {
        // `:scope` désigne l'élément sur lequel la recherche est lancée.
        let reference = doc.element(scope).map(|_| scope);
        if list.iter().any(|s| s.matches_in(doc, node, reference)) {
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
        list.iter().any(|s| s.matches_in(&h.doc, node, Some(node)))
    })))
}

fn closest(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let list = selectors(vm, &arg(args, 0))?;
    let found = host(vm).and_then(|h| {
        let doc = &h.doc;
        std::iter::once(node)
            .chain(doc.ancestors(node))
            .find(|n| list.iter().any(|s| s.matches_in(doc, *n, Some(node))))
    });
    Ok(wrap_opt(vm, found))
}

fn get_element_by_id(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let scope = this_node(vm, this)?;
    let id = vm.to_string(&arg(args, 0))?;
    if id.is_empty() {
        return Ok(Value::Null);
    }
    let found = host(vm).and_then(|h| {
        let doc = &h.doc;
        doc.descendants(scope)
            .find(|n| doc.element(*n).is_some_and(|e| e.id() == Some(&*id)))
    });
    Ok(wrap_opt(vm, found))
}

fn create_element(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let raw = vm.to_string(&arg(args, 0))?.to_string();
    let html = node_of(vm, this).is_none_or(|d| documents::is_html_document(vm, d));
    let name = if html { raw.to_ascii_lowercase() } else { raw };
    if !documents::is_element_local_name(&name) {
        return Err(dom_exception(
            vm,
            "InvalidCharacterError",
            "nom d'élément invalide",
        ));
    }
    let node = host(vm).map(|h| h.doc.create_element(&name, Namespace::Html, Vec::new()));
    Ok(documents::created_by(vm, this, node))
}

fn contains(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let other = node_of(vm, &arg(args, 0));
    Ok(Value::Bool(other.is_some_and(|o| {
        host(vm).is_some_and(|h| h.doc.contains(node, o))
    })))
}

// ---- Événements ----------------------------------------------------------------------

/// Signale une exception non rattrapée dans la console de la page.
fn report(vm: &mut Vm, thrown: &Value) {
    let mut message = vm.display(thrown);
    if let Some(site) = vm.throw_site(thrown) {
        message.push_str("\n    dans ");
        message.push_str(&site);
    }
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

/// Octets aléatoires tirés de la source du système, pour `crypto` : un
/// tableau de `n` nombres de 0 à 255 (`n` borné à 65 536).
fn random_bytes(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let wanted = vm.to_number(&arg(args, 0))?;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "valeur bornée juste avant"
    )]
    let count = if wanted.is_finite() {
        wanted.clamp(0.0, 65_536.0) as usize
    } else {
        0
    };
    let mut bytes = vec![0u8; count];
    if getrandom::fill(&mut bytes).is_err() {
        return Err(vm.error("Error", "la source d'aléa du système est indisponible"));
    }
    let items = bytes
        .into_iter()
        .map(|byte| Value::Number(f64::from(byte)))
        .collect();
    Ok(vm.new_array(items))
}

/// Distribue `click` sur `node`, entouré des fonctions d'activation (case
/// cochée, bouton radio, envoi de formulaire). Renvoie `true` si un script
/// a annulé l'action par défaut.
fn activate(vm: &mut Vm, node: NodeId) -> bool {
    let activation = host(vm).and_then(|h| h.activation.clone());
    let target = wrap(vm, node);
    let hook = |vm: &mut Vm, name: &str, args: &[Value]| {
        let hooks = activation.clone()?;
        match vm.get(&hooks, name).and_then(|f| vm.call(&f, &hooks, args)) {
            Ok(value) => Some(value),
            Err(thrown) => {
                report(vm, &thrown);
                None
            }
        }
    };
    let state = hook(vm, "before", &[target]).unwrap_or(Value::Null);
    let prevented = dispatch(vm, Some(node), "click");
    hook(vm, "after", &[state, Value::Bool(prevented)]);
    prevented
}

fn element_click(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    // Comme un clic de l'utilisateur : la case se coche, le bouton envoie.
    activate(vm, node);
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
        due: h.now() + delay,
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
    let elapsed = host(vm).map_or(0.0, |h| h.elapsed_ms());
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
    ("", "appendChild", tree::append_child),
    ("", "insertBefore", tree::insert_before),
    ("", "removeChild", tree::remove_child),
    ("", "remove", tree::remove),
    ("", "append", tree::append),
    ("", "prepend", tree::prepend),
    ("", "getAttribute", attrs::get_attribute),
    ("", "setAttribute", attrs::set_attribute),
    ("", "removeAttribute", attrs::remove_attribute),
    ("", "hasAttribute", attrs::has_attribute),
    ("", "querySelector", query_selector),
    ("", "querySelectorAll", query_selector_all),
    ("", "matches", element_matches),
    ("", "closest", closest),
    ("", "getElementById", get_element_by_id),
    ("", "getElementsByTagName", lists::get_elements_by_tag_name),
    (
        "",
        "getElementsByClassName",
        lists::get_elements_by_class_name,
    ),
    ("", "createElement", create_element),
    ("", "createTextNode", documents::create_text_node),
    ("", "contains", contains),
    ("", "cloneNode", documents::clone_node),
    ("", "addEventListener", add_event_listener),
    ("", "removeEventListener", remove_event_listener),
    ("", "click", element_click),
    ("", "focus", focus::element_focus),
    ("", "blur", focus::element_blur),
    ("", "scrollIntoView", geometry::scroll_into_view),
    ("", "scrollTo", geometry::element_scroll_to),
    ("", "scroll", geometry::element_scroll_to),
    ("", "scrollBy", geometry::element_scroll_by),
    (
        "",
        "getBoundingClientRect",
        geometry::get_bounding_client_rect,
    ),
    ("", "getClientRects", geometry::get_client_rects),
    ("", "dispatchEvent", events::dispatch_event),
    ("Event", "preventDefault", events::prevent_default),
    ("Event", "stopPropagation", events::stop_propagation),
    (
        "Event",
        "stopImmediatePropagation",
        events::stop_immediate_propagation,
    ),
    ("Event", "composedPath", events::composed_path),
    ("Event", "initEvent", events::init_event),
    ("CustomEvent", "initCustomEvent", events::init_custom_event),
    (
        "",
        "createDocumentFragment",
        documents::create_document_fragment,
    ),
    ("", "createCDATASection", documents::create_cdata_section),
    (
        "",
        "createProcessingInstruction",
        documents::create_processing_instruction,
    ),
    (
        "DOMImplementation",
        "createDocumentType",
        documents::create_document_type,
    ),
    (
        "DOMImplementation",
        "createDocument",
        documents::create_document,
    ),
    (
        "DOMImplementation",
        "createHTMLDocument",
        documents::create_html_document,
    ),
    ("DOMImplementation", "hasFeature", documents::has_feature),
    ("", "createRange", range::create_range),
    ("", "moveBefore", tree::move_before),
    (
        "",
        "getElementsByTagNameNS",
        lists::get_elements_by_tag_name_ns,
    ),
    ("", "getAttributeNode", attrs::get_attribute_node),
    ("", "getAttributeNodeNS", attrs::get_attribute_node_ns),
    ("", "setAttributeNode", attrs::set_attribute_node),
    ("", "setAttributeNodeNS", attrs::set_attribute_node),
    ("", "removeAttributeNode", attrs::remove_attribute_node),
    ("", "createAttribute", attrs::create_attribute),
    ("", "createAttributeNS", attrs::create_attribute_ns),
    ("", "lookupNamespaceURI", attrs::lookup_namespace_uri),
    ("", "lookupPrefix", attrs::lookup_prefix),
    ("", "isDefaultNamespace", attrs::is_default_namespace),
    ("NamedNodeMap", "item", attrs::map_item),
    ("NamedNodeMap", "getNamedItem", attrs::map_get),
    ("NamedNodeMap", "getNamedItemNS", attrs::map_get_ns),
    ("NamedNodeMap", "setNamedItem", attrs::map_set),
    ("NamedNodeMap", "setNamedItemNS", attrs::map_set),
    ("NamedNodeMap", "removeNamedItem", attrs::map_remove),
    ("NamedNodeMap", "removeNamedItemNS", attrs::map_remove_ns),
    ("Range", "setStart", range::set_start),
    ("Range", "setEnd", range::set_end),
    ("Range", "setStartBefore", range::set_start_before),
    ("Range", "setStartAfter", range::set_start_after),
    ("Range", "setEndBefore", range::set_end_before),
    ("Range", "setEndAfter", range::set_end_after),
    ("Range", "collapse", range::collapse),
    ("Range", "selectNode", range::select_node),
    ("Range", "selectNodeContents", range::select_node_contents),
    (
        "Range",
        "compareBoundaryPoints",
        range::compare_boundary_points,
    ),
    ("Range", "deleteContents", range::delete_contents),
    ("Range", "extractContents", range::extract_contents),
    ("Range", "cloneContents", range::clone_contents),
    ("Range", "insertNode", range::insert_node),
    ("Range", "surroundContents", range::surround_contents),
    ("Range", "cloneRange", range::clone_range),
    ("Range", "detach", range::detach),
    ("Range", "isPointInRange", range::is_point_in_range),
    ("Range", "comparePoint", range::compare_point),
    ("Range", "intersectsNode", range::intersects_node),
    ("Range", "toString", range::to_string),
    ("Storage", "getItem", storage::get_item),
    ("Storage", "setItem", storage::set_item),
    ("Storage", "removeItem", storage::remove_item),
    ("Storage", "clear", storage::clear),
    ("Storage", "key", storage::key),
    ("NodeList", "item", storage::list_item),
    ("HTMLCollection", "item", storage::list_item),
    ("HTMLCollection", "namedItem", storage::list_named_item),
    ("", "hasChildNodes", dom_ops::has_child_nodes),
    ("", "isSameNode", dom_ops::is_same_node),
    ("", "getRootNode", dom_ops::get_root_node),
    ("", "replaceChild", tree::replace_child),
    ("", "normalize", dom_ops::normalize),
    ("", "isEqualNode", dom_ops::is_equal_node),
    (
        "",
        "compareDocumentPosition",
        dom_ops::compare_document_position,
    ),
    ("", "before", tree::before),
    ("", "after", tree::after),
    ("", "replaceWith", tree::replace_with),
    ("", "replaceChildren", tree::replace_children),
    ("", "hasAttributes", attrs::has_attributes),
    ("", "getAttributeNames", attrs::get_attribute_names),
    ("", "toggleAttribute", attrs::toggle_attribute),
    ("", "getAttributeNS", attrs::get_attribute_ns),
    ("", "hasAttributeNS", attrs::has_attribute_ns),
    ("", "setAttributeNS", attrs::set_attribute_ns),
    ("", "removeAttributeNS", attrs::remove_attribute_ns),
    ("", "webkitMatchesSelector", element_matches),
    (
        "",
        "insertAdjacentElement",
        dom_ops::insert_adjacent_element,
    ),
    ("", "insertAdjacentText", dom_ops::insert_adjacent_text),
    ("", "insertAdjacentHTML", dom_ops::insert_adjacent_html),
    ("", "substringData", dom_ops::substring_data),
    ("", "appendData", dom_ops::append_data),
    ("", "insertData", dom_ops::insert_data),
    ("", "deleteData", dom_ops::delete_data),
    ("", "replaceData", dom_ops::replace_data),
    ("", "splitText", dom_ops::split_text),
    ("", "createComment", documents::create_comment),
    ("", "createElementNS", attrs::create_element_ns),
    ("", "importNode", documents::import_node),
    ("", "adoptNode", documents::adopt_node),
    ("", "getElementsByName", dom_ops::get_elements_by_name),
    ("", "hasFocus", dom_ops::has_focus),
    ("DOMTokenList", "item", tokens::item),
    ("DOMTokenList", "replace", tokens::replace),
    ("DOMTokenList", "supports", tokens::supports),
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
    ("DOMTokenList", "add", tokens::add),
    ("DOMTokenList", "remove", tokens::remove),
    ("DOMTokenList", "toggle", tokens::toggle),
    ("DOMTokenList", "contains", tokens::contains),
    ("DOMTokenList", "toString", tokens::to_string),
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

/// Constructeurs des interfaces qu'un script peut instancier.
const CONSTRUCTORS: &[(&str, NativeFn)] = &[
    ("Event", events::event_ctor),
    ("CustomEvent", events::custom_event_ctor),
    ("EventTarget", events::event_target_ctor),
    ("Document", documents::document_ctor),
    ("DocumentFragment", documents::fragment_ctor),
    ("Text", documents::text_ctor),
    ("Comment", documents::comment_ctor),
    ("Range", range::range_ctor),
];

/// Les autres objets interface ne se construisent pas depuis un script.
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
        let arity = interface.constructor.unwrap_or(0);
        let constructor = match CONSTRUCTORS
            .iter()
            .find(|(name, _)| *name == interface.name)
        {
            Some((_, func)) => vm.new_constructor(interface.name, arity, *func),
            None => vm.new_function(interface.name, arity, illegal_constructor),
        };
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
    // Les listes de nœuds sont des tableaux : leurs prototypes héritent de
    // `Array.prototype`, ce qui leur garde `forEach`, `map`, l'itération…
    let array = vm.get(&Value::Object(vm.global()), "Array")?;
    if let Some(array_proto) = vm.get(&array, "prototype")?.as_object() {
        for name in ["NodeList", "HTMLCollection", "DOMTokenList"] {
            if let Some(proto) = proto_of(name) {
                vm.set_prototype(proto, Some(array_proto))?;
            }
        }
    }
    let dataset_proto = vm.new_object();
    vm.add_root(Value::Object(dataset_proto));
    let storage_proto = proto_of("Storage");
    let blank = Document::new();
    let fresh = RealmHost {
        url: url.clone(),
        document: blank.root(),
        node_proto,
        style_proto,
        class_list_proto,
        dataset_proto,
        protos,
        hooks: None,
        activation: None,
        tag_protos: HashMap::new(),
    };
    let realm = vm.realm();
    if let Some(h) = vm.host.as_mut().and_then(|h| h.downcast_mut::<DomHost>()) {
        // Royaume d'un cadre : l'hôte existe, il gagne un état de plus.
        h.adopt(realm, fresh);
    } else {
        vm.host = Some(Box::new(DomHost {
            doc: blank,
            url: fresh.url,
            wrappers: HashMap::new(),
            kinds: HashMap::new(),
            lists: Vec::new(),
            node_proto: fresh.node_proto,
            style_proto: fresh.style_proto,
            class_list_proto: fresh.class_list_proto,
            dataset_proto: fresh.dataset_proto,
            storage: [Vec::new(), Vec::new()],
            protos: fresh.protos,
            listeners: Vec::new(),
            observers: Vec::new(),
            navigations: Vec::new(),
            hooks: None,
            activation: None,
            tag_protos: HashMap::new(),
            document: fresh.document,
            realm,
            parked: HashMap::new(),
            doc_realms: HashMap::new(),
            frames: HashMap::new(),
            frame_sources: HashMap::new(),
            frame_requests: Vec::new(),
            frames_scanned: None,
            layout_env: None,
            geometry: None,
            text: None,
            computed_proto: None,
            cookies: String::new(),
            cookie_writes: Vec::new(),
            next_listener: 0,
            timers: Vec::new(),
            fetch_queue: Vec::new(),
            fetches: HashMap::new(),
            modules: HashMap::new(),
            next_timer: 0,
            dirty: false,
            focus_value: None,
            current_script: None,
            scripts_started: None,
            scripts_scanned: 0,
            viewport: (0.0, 0.0),
            started: Instant::now(),
            clock: Instant::now(),
        }));
    }
    vm.set_host_hooks(HostHooks {
        get: host_get,
        set: host_set,
        keys: host_keys,
        global: named_global,
    });
    events::install_attributes(vm)?;
    // Prototype des styles calculés : celui de `CSSStyleDeclaration`, avec
    // une lecture de propriété qui lui est propre.
    let computed_proto = vm.new_object();
    vm.set_prototype(computed_proto, Some(style_proto))?;
    vm.define_method(
        computed_proto,
        "getPropertyValue",
        1,
        geometry::computed_get_property_value,
    );
    vm.add_root(Value::Object(computed_proto));
    if let Some(h) = host(vm) {
        h.computed_proto = Some(computed_proto);
    }
    if let Some(proto) = storage_proto {
        for (name, area) in [("localStorage", 0), ("sessionStorage", 1)] {
            let object = vm.new_host_object(proto, CLASS_STORAGE, area);
            vm.define_hidden(vm.global(), name, Value::Object(object));
        }
    }

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
        ("dispatchEvent", 1, events::dispatch_event),
        ("getComputedStyle", 1, geometry::get_computed_style),
        ("scrollTo", 2, geometry::window_scroll_to),
        ("scroll", 2, geometry::window_scroll_to),
        ("scrollBy", 2, geometry::window_scroll_by),
        ("alert", 1, window_alert),
        ("confirm", 1, window_confirm),
        ("prompt", 1, window_prompt),
    ];
    for (name, arity, func) in window_methods {
        vm.define_method(global, name, *arity, *func);
    }
    define_getter(vm, global, "innerWidth", window_inner_width)?;
    define_getter(vm, global, "innerHeight", window_inner_height)?;
    for name in ["scrollX", "pageXOffset"] {
        define_getter(vm, global, name, geometry::window_scroll_x)?;
    }
    for name in ["scrollY", "pageYOffset"] {
        define_getter(vm, global, name, geometry::window_scroll_y)?;
    }

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

    // Prélude : `URL`, `Headers`, `AbortController`… écrits en JavaScript. Il
    // s'appuie sur les objets installés ci-dessus.
    let prelude = vm.eval_script(include_str!("script/prelude.js"))?;
    let helpers = [
        Value::Object(vm.new_function("parseUrl", 2, parse_url)),
        Value::Object(vm.new_function("changeUrl", 3, change_url)),
        Value::Object(vm.new_function("observe", 4, mutation::observe)),
        Value::Object(vm.new_function("disconnect", 1, mutation::disconnect)),
        Value::Object(vm.new_function("takeRecords", 1, mutation::take_records)),
        Value::Object(vm.new_function("locationHref", 0, location_href)),
        Value::Object(vm.new_function("navigate", 2, navigate)),
        Value::Object(vm.new_function("focusedElement", 0, focus::focused_element)),
        Value::Object(vm.new_function("randomBytes", 1, random_bytes)),
    ];
    let hooks = vm.call(&prelude, &Value::Undefined, &helpers)?;
    vm.add_root(hooks.clone());
    if let Some(h) = host(vm) {
        h.hooks = Some(hooks);
    }
    // Interfaces des éléments HTML et formulaires.
    let elements = vm.eval_script(include_str!("script/elements.js"))?;
    let helpers = [
        Value::Object(vm.new_function("registerInterface", 2, register_interface)),
        Value::Object(vm.new_function("parseUrl", 2, parse_url)),
        Value::Object(vm.new_function("changeUrl", 3, change_url)),
        Value::Object(vm.new_function("locationHref", 0, location_href)),
        Value::Object(vm.new_function("navigate", 3, navigate)),
    ];
    let activation = vm.call(&elements, &Value::Undefined, &helpers)?;
    vm.add_root(activation.clone());
    if let Some(h) = host(vm) {
        h.activation = Some(activation);
    }
    // `CSS`, `document.fonts`, `FontFace`.
    let cssom = vm.eval_script(include_str!("script/cssom.js"))?;
    let helpers = [Value::Object(vm.new_function(
        "cssSupports",
        2,
        css_supports,
    ))];
    vm.call(&cssom, &Value::Undefined, &helpers)?;
    // Parcours de l'arbre et sélection.
    let traversal = vm.eval_script(include_str!("script/traversal.js"))?;
    let helpers = [
        Value::Object(vm.new_function("iterNew", 1, traversal::iter_new)),
        Value::Object(vm.new_function("iterGet", 1, traversal::iter_get)),
        Value::Object(vm.new_function("iterSet", 3, traversal::iter_set)),
    ];
    vm.call(&traversal, &Value::Undefined, &helpers)?;
    // `XMLHttpRequest`, écrit en JavaScript : le script rend une fabrique
    // qui reçoit le `fetch` d'origine.
    let factory = vm.eval_script(include_str!("script/xhr.js"))?;
    let original_fetch = vm.get(&window, "fetch")?;
    let xhr = vm.call(
        &factory,
        &Value::Undefined,
        std::slice::from_ref(&original_fetch),
    )?;
    vm.define_hidden(global, "XMLHttpRequest", xhr);
    // `Blob`, `Request`, `Response` et le `fetch` du standard, bâti sur
    // celui du moteur.
    let factory = vm.eval_script(include_str!("script/fetch.js"))?;
    let fetch = vm.call(&factory, &Value::Undefined, &[original_fetch])?;
    vm.define_hidden(global, "fetch", fetch);
    Ok(())
}

pub(crate) use frames::fingerprint as frame_fingerprint;

/// Moteur de script d'une page : la machine JavaScript et ses liaisons au DOM.
pub(crate) struct ScriptEngine {
    vm: Vm,
    /// Instructions accordées à chaque tâche.
    task_budget: u64,
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
        Self {
            vm,
            task_budget: TASK_BUDGET,
        }
    }

    /// Exécute `task` avec le document `doc` prêté à la machine, sur un fil à
    /// grande pile et avec un budget d'instructions.
    /// Charge dans le cadre `element` le document `source`, d'adresse
    /// `url`, et exécute ses scripts classiques dans le royaume du cadre.
    pub(crate) fn load_frame(
        &mut self,
        doc: &mut Document,
        element: NodeId,
        url: &Url,
        source: &Document,
        scripts: &[String],
    ) {
        self.run(doc, |vm| frames::load(vm, element, url, source, scripts));
    }

    /// Signale les cadres que l'embarqueur charge lui-même avec la page :
    /// élément et empreinte de sa source. Ils ne sont pas redemandés.
    pub(crate) fn note_static_frames(&mut self, frames: &[(NodeId, String)]) {
        if let Some(h) = host(&mut self.vm) {
            for (element, print) in frames {
                let key = u32::try_from(element.index()).unwrap_or(u32::MAX);
                h.frame_sources.insert(key, print.clone());
            }
        }
    }

    /// Documents de cadres demandés par les scripts depuis le dernier appel.
    pub(crate) fn take_frame_requests(&mut self) -> Vec<(NodeId, Url)> {
        host(&mut self.vm)
            .map(|h| std::mem::take(&mut h.frame_requests))
            .unwrap_or_default()
    }

    /// Remet au moteur de quoi mettre le document en page à la demande.
    pub(crate) fn set_layout_env(&mut self, env: crate::LayoutEnv) {
        if let Some(h) = host(&mut self.vm) {
            h.layout_env = Some(env);
            h.geometry = None;
        }
    }

    /// Fixe le texte que `document.cookie` rend aux scripts.
    pub(crate) fn set_cookies(&mut self, cookies: &str) {
        if let Some(h) = host(&mut self.vm) {
            cookies.clone_into(&mut h.cookies);
        }
    }

    /// Écritures dans `document.cookie` depuis le dernier appel.
    pub(crate) fn take_cookie_writes(&mut self) -> Vec<String> {
        host(&mut self.vm)
            .map(|h| std::mem::take(&mut h.cookie_writes))
            .unwrap_or_default()
    }

    /// Change le nombre d'instructions accordées à chaque tâche.
    pub(crate) fn set_task_budget(&mut self, budget: u64) {
        self.task_budget = budget;
    }

    fn run<T: Send>(&mut self, doc: &mut Document, task: impl FnOnce(&mut Vm) -> T + Send) -> T {
        let budget = self.task_budget;
        let vm = &mut self.vm;
        // Une tâche commence dans le royaume de la page.
        vm.set_realm(0);
        if let Some(h) = host(vm) {
            std::mem::swap(&mut h.doc, doc);
        }
        scripts::note_existing(vm);
        // Le temps des animations n'avance pas pendant la tâche.
        let clock = host(vm).and_then(|h| {
            h.layout_env
                .as_ref()
                .map(|env| std::sync::Arc::clone(&env.animations))
        });
        if let Some(clock) = &clock {
            clock.freeze(true);
        }
        // `document` désigne la racine du document prêté.
        let document = host(vm).map(|h| h.doc.root());
        if let Some(root) = document {
            let wrapper = wrap(vm, root);
            let global = vm.global();
            vm.define_hidden(global, "document", wrapper);
        }
        vm.set_budget(Some(budget));
        let result = oxyde_js::with_large_stack(|| {
            let result = task(vm);
            // Fin de la tâche : les réactions de promesses en attente
            // s'exécutent, puis les observateurs de mutation sont prévenus.
            vm.run_microtasks();
            mutation::deliver(vm);
            // Les cadres apparus pendant la tâche prennent leur contenu.
            if frames::scan(vm) {
                vm.run_microtasks();
                mutation::deliver(vm);
            }
            // Les scripts insérés pendant la tâche s'exécutent ou se chargent.
            scripts::scan(vm);
            result
        });
        if let Some(clock) = &clock {
            clock.freeze(false);
        }
        if let Some(h) = host(vm) {
            std::mem::swap(&mut h.doc, doc);
        }
        result
    }

    /// Exécute le script classique de l'élément `element`, que désigne
    /// `document.currentScript` pendant l'exécution.
    pub(crate) fn eval_element(
        &mut self,
        doc: &mut Document,
        element: Option<NodeId>,
        source: &str,
    ) {
        self.run(doc, |vm| scripts::run_classic(vm, element, source));
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
        if let Some(h) = host(&mut self.vm) {
            h.clock = h.clock.max(now);
        }
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

    /// Navigations demandées par les scripts depuis le dernier appel.
    pub(crate) fn take_navigations(&mut self) -> Vec<crate::Navigation> {
        host(&mut self.vm)
            .map(|h| std::mem::take(&mut h.navigations))
            .unwrap_or_default()
    }

    /// L'embarqueur a parcouru l'historique de `delta` entrées sans changer
    /// de document : l'adresse devient `url`, et les scripts en sont prévenus
    /// (`popstate`, `hashchange`).
    pub(crate) fn traverse(&mut self, doc: &mut Document, url: &Url, delta: i32) {
        self.run(doc, |vm| {
            let Some(h) = host(vm) else { return };
            let before = Value::string(h.url.as_str());
            h.url = url.clone();
            let Some(hooks) = h.hooks.clone() else { return };
            let outcome = vm
                .get(&hooks, "traverse")
                .and_then(|f| vm.call(&f, &hooks, &[Value::Number(f64::from(delta)), before]));
            if let Err(thrown) = outcome {
                report(vm, &thrown);
            }
        });
    }

    /// Clic sur `node` : comportement d'activation (case cochée, bouton de
    /// formulaire) autour de l'événement `click`. Renvoie `true` si un script
    /// a annulé l'action par défaut.
    pub(crate) fn click(&mut self, doc: &mut Document, node: NodeId) -> bool {
        self.run(doc, |vm| {
            // Le focus va à l'élément cliqué, ou quitte celui qui l'avait.
            let target = host(vm).and_then(|h| focus::focus_target(&h.doc, node));
            focus::move_focus(vm, target);
            activate(vm, node)
        })
    }

    /// Touche enfoncée (`down`) ou relâchée : distribue `keydown` (et
    /// `keypress`) ou `keyup`. Renvoie `true` si un script a annulé l'action
    /// par défaut.
    pub(crate) fn key_event(
        &mut self,
        doc: &mut Document,
        down: bool,
        input: &crate::KeyInput<'_>,
    ) -> bool {
        self.run(doc, |vm| {
            let Some(hooks) = host(vm).and_then(|h| h.hooks.clone()) else {
                return false;
            };
            let args = [
                Value::Bool(down),
                Value::string(input.key),
                Value::string(input.code),
                Value::Bool(input.ctrl),
                Value::Bool(input.shift),
                Value::Bool(input.alt),
                Value::Bool(input.meta),
                Value::Bool(input.repeat),
            ];
            match vm
                .get(&hooks, "key")
                .and_then(|f| vm.call(&f, &hooks, &args))
            {
                Ok(proceed) => {
                    // Aucun script ne s'y oppose : la touche fait son office.
                    if down && proceed.truthy() {
                        focus::default_key(vm, input);
                    }
                    !proceed.truthy()
                }
                Err(thrown) => {
                    report(vm, &thrown);
                    false
                }
            }
        })
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

    /// Distribue l'événement `scroll` aux zones `ids` qui ont défilé : sur
    /// le document pour la fenêtre, sur l'élément sinon.
    pub(crate) fn scroll_events(&mut self, doc: &mut Document, ids: &[u32]) {
        self.run(doc, |vm| {
            for id in ids {
                let Some(h) = host(vm) else { return };
                if *id == oxyde_compositor::ROOT_SCROLLER {
                    let document = h.doc.root();
                    events::dispatch_simple(vm, document, "scroll", true);
                } else if let Some(node) = h.doc.id_from_index(*id as usize) {
                    events::dispatch_simple(vm, node, "scroll", false);
                }
                vm.run_microtasks();
            }
        });
    }

    /// Distribue aux scripts les événements d'animation et de transition.
    pub(crate) fn animation_events(
        &mut self,
        doc: &mut Document,
        events: &[oxyde_css::AnimationEvent],
    ) {
        self.run(doc, |vm| {
            for event in events {
                let transition = event.kind.starts_with("transition");
                let (interface, key) = if transition {
                    ("TransitionEvent", "propertyName")
                } else {
                    ("AnimationEvent", "animationName")
                };
                let global = Value::Object(vm.global());
                let outcome = (|| -> JsResult {
                    let constructor = vm.get(&global, interface)?;
                    let init = Value::Object(vm.new_object());
                    vm.set(&init, key, Value::string(&event.name))?;
                    vm.set(&init, "elapsedTime", Value::Number(event.elapsed))?;
                    vm.set(&init, "bubbles", Value::Bool(true))?;
                    let object = vm.construct(
                        &constructor,
                        &[Value::string(event.kind), init],
                        &constructor,
                    )?;
                    let target = wrap(vm, event.node);
                    let dispatch = vm.get(&target, "dispatchEvent")?;
                    vm.call(&dispatch, &target, &[object])
                })();
                if let Err(thrown) = outcome {
                    report(vm, &thrown);
                }
                vm.run_microtasks();
            }
        });
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
    /// Élément `<script>` dont la réponse est le source : elle n'est pas
    /// remise à un script, donc pas soumise à la politique de même origine.
    script: Option<NodeId>,
}

/// Rejette la promesse d'un `fetch` avec une `TypeError`.
fn reject_fetch(vm: &mut Vm, reject: &Value, message: &str) -> JsResult<()> {
    let error = vm.error("TypeError", &format!("fetch : {message}"));
    vm.call(reject, &Value::Undefined, &[error]).map(|_| ())
}

/// En-têtes d'une requête donnés par un script : un objet `{ nom: valeur }`
/// ou un tableau de paires. Les en-têtes interdits sont écartés sans bruit ;
/// un nom ou une valeur invalide est une erreur.
fn request_headers(vm: &mut Vm, source: &Value) -> JsResult<Result<Vec<(String, String)>, String>> {
    let mut headers = Vec::new();
    if source.as_object().is_none() {
        return Ok(Ok(headers));
    }
    let global = Value::Object(vm.global());
    let array = vm.get(&global, "Array")?;
    // Itérable (tableau de paires, `Headers`, `Map`) : `Array.from` en tire
    // les paires ; sinon ce sont les propriétés de l'objet.
    let symbol = vm.get(&global, "Symbol")?;
    let iterator = vm.get(&symbol, "iterator")?;
    let iterable = match vm.to_key(&iterator) {
        Ok(key) => {
            let method = vm.get_key(source, &key)?;
            vm.type_of(&method) == "function"
        }
        Err(_) => false,
    };
    let pairs = if iterable {
        let from = vm.get(&array, "from")?;
        vm.call(&from, &array, std::slice::from_ref(source))?
    } else {
        let object = vm.get(&global, "Object")?;
        let entries = vm.get(&object, "entries")?;
        vm.call(&entries, &object, std::slice::from_ref(source))?
    };
    let length = vm.get(&pairs, "length")?;
    let count = vm.to_number(&length)?;
    let mut index = 0.0;
    while index < count {
        let pair = vm.get(&pairs, &index.to_string())?;
        index += 1.0;
        let (name, value) = (vm.get(&pair, "0")?, vm.get(&pair, "1")?);
        let name = vm.to_string(&name)?.to_string();
        let value = vm.to_string(&value)?.trim().to_owned();
        if !crate::cors::is_header_name(&name) {
            return Ok(Err(format!("nom d'en-tête invalide : « {name} »")));
        }
        if !crate::cors::is_header_value(&value) {
            return Ok(Err(format!("valeur invalide pour l'en-tête « {name} »")));
        }
        if !crate::cors::is_forbidden_header(&name) {
            headers.push((name, value));
        }
    }
    Ok(Ok(headers))
}

/// Requête décrite par les arguments de `fetch`, ou la raison de son refus.
struct Outgoing {
    address: String,
    method: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

fn outgoing(vm: &mut Vm, args: &[Value]) -> JsResult<Result<Outgoing, String>> {
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
    }
    .to_string();
    let init = arg(args, 1);
    let member = |vm: &mut Vm, name: &str| -> JsResult {
        if init.as_object().is_some() {
            vm.get(&init, name)
        } else {
            Ok(Value::Undefined)
        }
    };
    let method = match member(vm, "method")? {
        Value::Undefined => "GET".to_owned(),
        other => {
            let name = vm.to_string(&other)?;
            if !crate::cors::is_allowed_method(&name) {
                return Ok(Err(format!("la méthode « {name} » n'est pas permise")));
            }
            crate::cors::normalize_method(&name)
        }
    };
    let header_source = member(vm, "headers")?;
    let mut headers = match request_headers(vm, &header_source)? {
        Ok(headers) => headers,
        Err(message) => return Ok(Err(message)),
    };
    let body = match member(vm, "body")? {
        Value::Undefined | Value::Null => Vec::new(),
        other => {
            if matches!(method.as_str(), "GET" | "HEAD") {
                return Ok(Err(format!(
                    "une requête {method} ne peut pas avoir de corps"
                )));
            }
            // LIMITE : le corps est pris pour du texte ; des paramètres
            // d'URL partent comme un formulaire.
            let form = match &other {
                Value::Object(_) => {
                    let constructor = vm.get(&other, "constructor")?;
                    let name = vm.get(&constructor, "name")?;
                    name == Value::string("URLSearchParams")
                }
                _ => false,
            };
            let text = vm.to_string(&other)?;
            if !headers
                .iter()
                .any(|(name, _)| name.eq_ignore_ascii_case("content-type"))
            {
                headers.push((
                    "Content-Type".to_owned(),
                    if form {
                        "application/x-www-form-urlencoded;charset=UTF-8".to_owned()
                    } else {
                        "text/plain;charset=UTF-8".to_owned()
                    },
                ));
            }
            text.as_bytes().to_vec()
        }
    };
    // Corps binaire : une chaîne d'un caractère (0 à 255) par octet.
    let body = match member(vm, "binaryBody")? {
        Value::String(binary) => {
            if matches!(method.as_str(), "GET" | "HEAD") {
                return Ok(Err(format!(
                    "une requête {method} ne peut pas avoir de corps"
                )));
            }
            binary
                .chars()
                .map(|c| u8::try_from(u32::from(c)).unwrap_or(b'?'))
                .collect()
        }
        _ => body,
    };
    Ok(Ok(Outgoing {
        address,
        method,
        headers,
        body,
    }))
}

fn fetch(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let (promise, resolve, reject) = vm.promise_with_resolvers();
    let request = match outgoing(vm, args)? {
        Ok(request) => request,
        Err(message) => {
            reject_fetch(vm, &reject, &message)?;
            return Ok(promise);
        }
    };
    let Some(h) = host(vm) else {
        return Ok(promise);
    };
    let refusal = match h.url.join(&request.address) {
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
            let document = h.url.origin();
            let cross_origin = !document.is_tuple() || document != url.origin();
            // `Origin` accompagne toute requête vers une autre origine, et
            // toute requête qui n'est ni `GET` ni `HEAD`.
            let origin = (cross_origin || !matches!(request.method.as_str(), "GET" | "HEAD"))
                .then(|| document.ascii_serialization());
            let needs_preflight =
                cross_origin && crate::cors::needs_preflight(&request.method, &request.headers);
            h.fetch_queue.push(crate::FetchRequest {
                id,
                initiator: h.url.clone(),
                url: url.clone(),
                method: request.method,
                headers: request.headers,
                body: request.body,
                origin,
                needs_preflight,
            });
            h.fetches.insert(
                id,
                PendingFetch {
                    url,
                    resolve: resolve.clone(),
                    reject: reject.clone(),
                    script: None,
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
    if let Some(script) = pending.script {
        scripts::loaded(vm, script, result);
        return;
    }
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
    let has = vm.new_native_data(headers_has, pairs.clone());
    let for_each = vm.new_native_data(headers_for_each, pairs);
    vm.define_hidden(headers, "get", get);
    vm.define_hidden(headers, "has", has);
    vm.define_hidden(headers, "forEach", for_each);
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
    // Octets du corps, un caractère par octet (voir `fetch.js`).
    let binary: String = response.body.iter().map(|byte| char::from(*byte)).collect();
    let binary_fn = vm.new_native_data(response_binary, vec![Value::string(&binary)]);
    vm.define_hidden(id, "binary", binary_fn);
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

/// `headers.forEach(rappel)` : `rappel(valeur, nom, headers)` pour chaque en-tête.
fn headers_for_each(vm: &mut Vm, data: &[Value], this: &Value, args: &[Value]) -> JsResult {
    let callback = arg(args, 0);
    let this_arg = arg(args, 1);
    for pair in data.chunks(2) {
        if let [name, value] = pair {
            vm.call(
                &callback,
                &this_arg,
                &[value.clone(), name.clone(), this.clone()],
            )?;
        }
    }
    Ok(Value::Undefined)
}

fn response_binary(_vm: &mut Vm, data: &[Value], _this: &Value, _args: &[Value]) -> JsResult {
    Ok(data.first().cloned().unwrap_or(Value::Undefined))
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

// ---- Navigation ------------------------------------------------------------------

/// `registerInterface(balise, prototype)` : les éléments HTML de cette
/// balise prendront ce prototype.
fn register_interface(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let tag = vm.to_string(&arg(args, 0))?.to_string();
    let proto = arg(args, 1);
    if let Some(id) = proto.as_object() {
        vm.add_root(proto);
        if let Some(h) = host(vm) {
            h.tag_protos.insert(tag, id);
        }
    }
    Ok(Value::Undefined)
}

/// `locationHref()` : l'adresse courante du document.
fn location_href(vm: &mut Vm, _this: &Value, _args: &[Value]) -> JsResult {
    Ok(host(vm).map_or(Value::Undefined, |h| Value::string(h.url.as_str())))
}

/// `navigate(nature, valeur)` : consigne une navigation pour l'embarqueur.
/// `pushState` et `replaceState` changent tout de suite l'adresse du
/// document, qui doit rester dans son origine.
fn navigate(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let kind = vm.to_string(&arg(args, 0))?;
    let value = arg(args, 1);
    let request = match &*kind {
        "reload" => crate::Navigation::Reload,
        "traverse" => {
            let delta = vm.to_number(&value)?;
            #[expect(clippy::cast_possible_truncation, reason = "nombre de pages, borné")]
            let delta = delta.clamp(-1_000.0, 1_000.0) as i32;
            crate::Navigation::Traverse(delta)
        }
        _ => {
            let text = vm.to_string(&value)?;
            let Some(h) = host(vm) else {
                return Ok(Value::Undefined);
            };
            let Ok(url) = h.url.join(&text) else {
                return Err(vm.error("SyntaxError", &format!("adresse invalide : {text}")));
            };
            match &*kind {
                "pushState" | "replaceState" => {
                    if url.origin() != h.url.origin() {
                        return Err(dom_exception(
                            vm,
                            "SecurityError",
                            "l'adresse doit rester dans l'origine du document",
                        ));
                    }
                    h.url = url.clone();
                    crate::Navigation::SameDocument {
                        url,
                        replace: &*kind == "replaceState",
                    }
                }
                "post" => {
                    let body = vm.to_string(&arg(args, 2))?;
                    crate::Navigation::Load {
                        url,
                        replace: false,
                        post: Some(crate::FormPost {
                            content_type: "application/x-www-form-urlencoded".to_owned(),
                            body: body.as_bytes().to_vec(),
                        }),
                    }
                }
                _ => crate::Navigation::Load {
                    url,
                    replace: &*kind == "replace",
                    post: None,
                },
            }
        }
    };
    if let Some(h) = host(vm) {
        h.navigations.push(request);
    }
    Ok(Value::Undefined)
}

// ---- URL -------------------------------------------------------------------------

/// Composants de `url`, dans l'ordre attendu par le prélude : href, protocol,
/// username, password, host, hostname, port, pathname, search, hash, origin.
fn url_parts(vm: &mut Vm, url: &Url) -> Value {
    use url::quirks;
    let origin = quirks::origin(url);
    let items = [
        quirks::href(url),
        quirks::protocol(url),
        quirks::username(url),
        quirks::password(url),
        quirks::host(url),
        quirks::hostname(url),
        quirks::port(url),
        quirks::pathname(url),
        quirks::search(url),
        quirks::hash(url),
        origin.as_str(),
    ]
    .map(Value::string)
    .to_vec();
    vm.new_array(items)
}

/// `parseUrl(texte, base)` : les composants de l'adresse, ou `null`.
fn parse_url(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let input = vm.to_string(&arg(args, 0))?;
    let parsed = match arg(args, 1) {
        Value::Undefined => Url::parse(&input),
        base => {
            let base = vm.to_string(&base)?;
            Url::parse(&base).and_then(|base| base.join(&input))
        }
    };
    Ok(match parsed {
        Ok(url) => url_parts(vm, &url),
        Err(_) => Value::Null,
    })
}

/// `changeUrl(href, rang, valeur)` : applique le mutateur du composant de
/// rang `rang` (URL Standard §6.1) et rend les nouveaux composants. Une
/// valeur refusée laisse l'adresse telle quelle.
fn change_url(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    use url::quirks;
    let href = vm.to_string(&arg(args, 0))?;
    let rank = vm.to_number(&arg(args, 1))?;
    let value = vm.to_string(&arg(args, 2))?;
    let Ok(mut url) = Url::parse(&href) else {
        return Ok(Value::Null);
    };
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "rang choisi par le prélude, entre 1 et 9"
    )]
    let rank = rank as u8;
    // Les mutateurs faillibles signalent un refus, sans autre effet.
    let _refused = match rank {
        1 => quirks::set_protocol(&mut url, &value),
        2 => quirks::set_username(&mut url, &value),
        3 => quirks::set_password(&mut url, &value),
        4 => quirks::set_host(&mut url, &value),
        5 => quirks::set_hostname(&mut url, &value),
        6 => quirks::set_port(&mut url, &value),
        7 => {
            quirks::set_pathname(&mut url, &value);
            Ok(())
        }
        8 => {
            quirks::set_search(&mut url, &value);
            Ok(())
        }
        9 => {
            quirks::set_hash(&mut url, &value);
            Ok(())
        }
        _ => Ok(()),
    };
    Ok(url_parts(vm, &url))
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
           const xml = document.implementation.createDocument(null, 'racine');
           const samples = {{
             Event: ev, CustomEvent: new CustomEvent('x'), EventTarget: el, Node: el, Element: el, HTMLElement: el, Document: document,
             DocumentType: kind(document.firstChild) === 10 ? document.firstChild : null,
             DocumentFragment: document.createDocumentFragment(), Storage: localStorage,
             CharacterData: el.firstChild, Text: el.firstChild,
             Comment: kind(el.lastChild) === 8 ? el.lastChild : null,
             DOMTokenList: el.classList, CSSStyleDeclaration: el.style,
             NodeList: document.querySelectorAll('p'), HTMLCollection: el.children,
             Range: document.createRange(), AbstractRange: document.createRange(),
             DOMImplementation: document.implementation, XMLDocument: xml,
             ProcessingInstruction: xml.createProcessingInstruction('cible', 'x'),
             CDATASection: xml.createCDATASection('x'),
             NamedNodeMap: el.attributes, Attr: el.getAttributeNode('id'),
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
         c'est le rôle des Web Platform Tests (voir `wpt.md`). Les interfaces que l'IDL \
         d'Oxyde ne décrit pas encore (éléments HTML particuliers, `Window`, observateurs, plages…) \
         ne sont pas comptées.\n\n\
         **Total : {done} membres disponibles sur {total} ({rate:.1} %).**\n\n\
         | Interface | Hérite de | Constantes | Attributs | Opérations |\n|---|---|---:|---:|---:|\n{rows}\n\
         ## Membres absents\n\n{missing}"
    )
}
