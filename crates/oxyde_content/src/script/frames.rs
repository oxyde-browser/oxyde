//! Cadres (`<iframe>`) : chaque cadre a son contexte de navigation — un
//! royaume JavaScript, donc un objet global et des interfaces du DOM à lui,
//! et un document, rangé dans la même arène que celui de la page.
//!
//! Le contexte d'un cadre est créé à la première demande (`contentWindow`,
//! `contentDocument`, chargement de son `src`) : un document vide
//! (`about:blank`) que le chargement remplace ensuite.
//!
//! LIMITES CONNUES : un cadre n'est pas dessiné ; faute d'isolation entre
//! processus, seul un cadre de même origine que la page est chargé — un
//! cadre d'une autre origine reste vide ; un document XML est analysé comme
//! du HTML ; un cadre retiré du
//! document garde son royaume en mémoire jusqu'à la fin de la page ; les
//! cadres d'un cadre ne sont pas chargés depuis leur `src`.

use oxyde_dom::{Document, Namespace, NodeId};
use oxyde_js::{JsResult, Value, Vm};

use super::{events, host, install, report, wrap};
use crate::Url;

/// Nombre maximal de cadres par page.
const MAX_FRAMES: usize = 200;

/// Contexte de navigation d'un cadre.
#[derive(Clone, Copy, Debug)]
pub(super) struct FrameContext {
    /// Royaume du cadre.
    pub(super) realm: usize,
    /// Document du cadre.
    pub(super) document: NodeId,
    /// Royaume du document qui contient le cadre.
    pub(super) parent_realm: usize,
}

fn index(node: NodeId) -> u32 {
    u32::try_from(node.index()).unwrap_or(u32::MAX)
}

/// Royaume auquel appartient le document `document` (0 : celui de la page).
pub(super) fn realm_of_document(vm: &mut Vm, document: NodeId) -> usize {
    host(vm).map_or(0, |h| {
        h.doc_realms.get(&index(document)).copied().unwrap_or(0)
    })
}

/// Vrai si `element` est un `<iframe>` relié à un document doté d'une
/// fenêtre (celui de la page ou celui d'un cadre).
fn can_host(vm: &mut Vm, element: NodeId) -> bool {
    let Some(h) = host(vm) else { return false };
    let doc = &h.doc;
    if !doc.element(element).is_some_and(|e| e.is_html("iframe")) {
        return false;
    }
    let root = doc.ancestors(element).last().unwrap_or(element);
    root == doc.root() || h.frames.values().any(|frame| frame.document == root)
}

/// Document vide d'un cadre neuf : `<html><head></head><body></body></html>`.
fn blank_document(doc: &mut Document) -> NodeId {
    let document = doc.create_document();
    let html = doc.create_element("html", Namespace::Html, Vec::new());
    doc.append_child(document, html);
    for name in ["head", "body"] {
        let child = doc.create_element(name, Namespace::Html, Vec::new());
        doc.append_child(html, child);
    }
    document
}

/// Contexte du cadre `element`, créé s'il n'existe pas encore. `None` si
/// l'élément ne peut pas en avoir (il n'est pas dans un document affiché).
pub(super) fn ensure(vm: &mut Vm, element: NodeId) -> Option<FrameContext> {
    let key = index(element);
    if !can_host(vm, element) {
        // Sorti du document : le cadre n'a plus de contexte.
        if let Some(h) = host(vm) {
            h.frames.remove(&key);
        }
        return None;
    }
    if let Some(existing) = host(vm)?.frames.get(&key).copied() {
        return Some(existing);
    }
    if host(vm)?.frames.len() >= MAX_FRAMES {
        return None;
    }
    let parent_document = {
        let doc = &host(vm)?.doc;
        doc.ancestors(element).last().unwrap_or(element)
    };
    let parent_realm = realm_of_document(vm, parent_document);
    let parent_global = Value::Object(vm.realm_global(parent_realm)?);
    let element_wrapper = wrap(vm, element);
    let position = host(vm)?
        .frames
        .values()
        .filter(|frame| frame.parent_realm == parent_realm)
        .count();
    let realm = vm.create_realm()?;
    let document = {
        let h = host(vm)?;
        let document = blank_document(&mut h.doc);
        h.doc_realms.insert(index(document), realm);
        document
    };
    let previous = vm.realm();
    vm.set_realm(realm);
    let installed = (|| -> JsResult<Value> {
        let url = Url::parse("about:blank")
            .map_err(|_| vm.error("TypeError", "adresse de cadre invalide"))?;
        install(vm, &url)?;
        if let Some(h) = host(vm) {
            h.document = document;
        }
        let global = vm.global();
        let wrapper = wrap(vm, document);
        vm.define_hidden(global, "document", wrapper);
        vm.define_hidden(global, "parent", parent_global.clone());
        let top = vm.get(&parent_global, "top")?;
        vm.define_hidden(global, "top", top);
        vm.define_hidden(global, "frameElement", element_wrapper);
        Ok(Value::Object(global))
    })();
    vm.set_realm(previous);
    let window = match installed {
        Ok(window) => window,
        Err(thrown) => {
            report(vm, &thrown);
            return None;
        }
    };
    // `window[rang]` désigne la fenêtre du cadre.
    let _ = vm.set(&parent_global, &position.to_string(), window);
    let context = FrameContext {
        realm,
        document,
        parent_realm,
    };
    host(vm)?.frames.insert(key, context);
    Some(context)
}

/// Empreinte de la source d'un `<iframe>` : son `srcdoc`, son `src`, ou
/// rien. Tant qu'elle ne change pas, le cadre n'est pas rechargé.
pub(crate) fn fingerprint(srcdoc: Option<&str>, src: Option<&str>) -> String {
    match (srcdoc, src.map(str::trim).filter(|s| !s.is_empty())) {
        (Some(srcdoc), _) => format!("srcdoc:{srcdoc}"),
        (None, Some(src)) => format!("src:{src}"),
        (None, None) => "blank".to_owned(),
    }
}

/// Scripts classiques en ligne du document `doc`.
fn inline_scripts(doc: &Document) -> Vec<String> {
    doc.descendants(doc.root())
        .filter(|n| {
            doc.element(*n).is_some_and(|e| {
                e.is_html("script")
                    && !e.has_attr("src")
                    && e.attr("type").is_none_or(|t| {
                        let t = t.trim().to_ascii_lowercase();
                        t.is_empty() || t.contains("javascript") || t.contains("ecmascript")
                    })
            })
        })
        .map(|n| doc.text_content(n))
        .collect()
}

/// Relève les `<iframe>` des documents affichés : un cadre neuf ou dont la
/// source a changé prend son contenu — tout de suite pour `srcdoc` et pour
/// un cadre vide, par une demande à l'embarqueur pour `src`. Renvoie `true`
/// si des scripts ont pu s'exécuter.
pub(super) fn scan(vm: &mut Vm) -> bool {
    let Some(h) = host(vm) else { return false };
    let version = h.doc.version();
    if h.frames_scanned == Some(version) {
        return false;
    }
    h.frames_scanned = Some(version);
    let doc = &h.doc;
    let mut documents = vec![doc.root()];
    documents.extend(h.frames.values().map(|frame| frame.document));
    let mut found: Vec<(NodeId, String)> = Vec::new();
    for document in documents {
        for node in doc.descendants(document) {
            if let Some(element) = doc.element(node)
                && element.is_html("iframe")
            {
                found.push((
                    node,
                    fingerprint(element.attr("srcdoc"), element.attr("src")),
                ));
            }
        }
    }
    let mut ran = false;
    for (element, print) in found {
        let key = index(element);
        let Some(h) = host(vm) else { return ran };
        let first = !h.frame_sources.contains_key(&key);
        if h.frame_sources.get(&key) == Some(&print) {
            continue;
        }
        h.frame_sources.insert(key, print.clone());
        if let Some(srcdoc) = print.strip_prefix("srcdoc:") {
            let parsed = oxyde_html::parse_document(srcdoc);
            let scripts = inline_scripts(&parsed);
            if let Ok(url) = Url::parse("about:srcdoc") {
                load(vm, element, &url, &parsed, &scripts);
                ran = true;
            }
            continue;
        }
        let target = print.strip_prefix("src:").and_then(|src| {
            let h = host(vm)?;
            // L'adresse se résout par rapport au document qui porte le cadre.
            let document = h.doc.ancestors(element).last().unwrap_or(element);
            let realm = h.doc_realms.get(&index(document)).copied().unwrap_or(0);
            let base = match h.parked.get(&realm) {
                Some(parked) if realm != h.realm => &parked.url,
                _ => &h.url,
            };
            let page = h.parked.get(&0).map_or(&h.url, |parked| &parked.url);
            base.join(src)
                .or_else(|_| page.join(src))
                .ok()
                .filter(|url| matches!(url.scheme(), "http" | "https" | "file"))
                .filter(|url| crate::same_origin(url, page))
        });
        match target {
            Some(url) => {
                if let Some(h) = host(vm) {
                    h.frame_requests.push((element, url));
                }
            }
            // Cadre vide : son document `about:blank` est prêt aussitôt.
            None if first && ensure(vm, element).is_some() => {
                events::dispatch(vm, Some(element), "load");
                ran = true;
            }
            None => {}
        }
    }
    ran
}

/// `contentWindow` ou `contentDocument` du cadre `element`.
pub(super) fn property(vm: &mut Vm, element: NodeId, key: &str) -> JsResult {
    let Some(context) = ensure(vm, element) else {
        return Ok(Value::Null);
    };
    Ok(if key == "contentDocument" {
        wrap(vm, context.document)
    } else {
        vm.realm_global(context.realm)
            .map_or(Value::Null, Value::Object)
    })
}

/// Charge dans le cadre `element` le document `source`, d'adresse `url` :
/// son arbre remplace le document vide, ses scripts s'exécutent dans le
/// royaume du cadre, puis `load` est distribué à sa fenêtre et à l'élément.
pub(super) fn load(vm: &mut Vm, element: NodeId, url: &Url, source: &Document, scripts: &[String]) {
    let Some(context) = ensure(vm, element) else {
        return;
    };
    if let Some(h) = host(vm) {
        h.doc.remove_children(context.document);
        let children: Vec<NodeId> = source.children(source.root()).collect();
        for child in children {
            let copy = h.doc.import(source, child);
            h.doc.append_child(context.document, copy);
        }
        h.dirty = true;
    }
    let previous = vm.realm();
    vm.set_realm(context.realm);
    if let Some(h) = host(vm) {
        h.url = url.clone();
    }
    for script in scripts {
        if let Err(thrown) = vm.eval_script(script) {
            report(vm, &thrown);
        }
        vm.run_microtasks();
    }
    events::dispatch(vm, Some(context.document), "DOMContentLoaded");
    events::dispatch(vm, None, "load");
    vm.set_realm(previous);
    events::dispatch(vm, Some(element), "load");
}
