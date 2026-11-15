//! Scripts insérés par les scripts (`document.createElement("script")`).
//!
//! Les scripts du HTML reçu sont exécutés par la page, dans l'ordre du
//! document. Ceux qu'un script insère ensuite sont pris en charge ici, dès
//! l'insertion (`appendChild`, `append`, `textContent`…) : un script en
//! ligne s'exécute sur le champ, au milieu du script qui l'a inséré ; un
//! script externe est demandé par le circuit de `fetch`, s'exécute à
//! l'arrivée de sa réponse, puis reçoit `load` (ou `error`). Un relevé en
//! fin de tâche rattrape les autres voies d'insertion (`innerHTML`).
//!
//! LIMITES CONNUES : un module en ligne inséré n'est pas exécuté (un module
//! inséré avec `src` est importé, voir `modules`) ;
//! les scripts externes s'exécutent dans l'ordre d'arrivée (`async`), même
//! avec `async = false` ; l'adresse se résout par rapport à celle de la page,
//! sans tenir compte de `<base>` ; un script inséré dans un cadre n'est pas
//! exécuté ; le texte est lu en UTF-8.

use std::collections::HashSet;

use oxyde_dom::{Document, NodeId};
use oxyde_js::{Value, Vm};

use super::events::dispatch_simple;
use super::{MAX_PENDING_FETCHES, NEXT_FETCH_ID, PendingFetch, host};

/// Nombre maximal de scripts en ligne exécutés à la suite d'une même tâche :
/// borne un script qui s'insère lui-même sans fin.
const MAX_CHAINED: usize = 1_000;

fn script_elements(doc: &Document) -> impl Iterator<Item = NodeId> + '_ {
    doc.descendants(doc.root())
        .filter(|node| doc.element(*node).is_some_and(|e| e.is_html("script")))
}

/// Texte d'un script : celui de ses enfants directs, sans celui des
/// éléments qu'il contiendrait.
fn child_text(doc: &Document, node: NodeId) -> String {
    doc.children(node).filter_map(|n| doc.text(n)).collect()
}

/// Relève, à la première tâche, les scripts du HTML reçu : la page les
/// exécute elle-même.
pub(super) fn note_existing(vm: &mut Vm) {
    let Some(h) = host(vm) else {
        return;
    };
    if h.scripts_started.is_none() {
        // Un script vide et sans adresse n'a pas commencé : il s'exécutera
        // quand un script lui donnera son texte.
        let existing: HashSet<usize> = script_elements(&h.doc)
            .filter(|n| {
                h.doc.element(*n).is_some_and(|e| e.has_attr("src"))
                    || !child_text(&h.doc, *n).trim().is_empty()
            })
            .map(|n| n.index())
            .collect();
        h.scripts_started = Some(existing);
        h.scripts_scanned = h.doc.version();
    }
}

/// Exécute le script classique `source` ; `element` est son élément
/// (`document.currentScript`). Une exception est signalée dans la console.
pub(super) fn run_classic(vm: &mut Vm, element: Option<NodeId>, source: &str) {
    let previous = host(vm).and_then(|h| std::mem::replace(&mut h.current_script, element));
    if let Err(error) = vm.eval(source) {
        let mut text = format!("Exception non rattrapée : {error}");
        // Où l'erreur est née, et dans quel script.
        if let oxyde_js::EvalError::Thrown { value, .. } = &error
            && let Some(site) = vm.throw_site(value)
        {
            text.push_str("\n    dans ");
            text.push_str(&site);
        }
        let origin = element.and_then(|node| {
            let h = host(vm)?;
            let src = h.doc.element(node)?.attr("src")?;
            Some(src.to_owned())
        });
        text.push_str("\n    script : ");
        text.push_str(origin.as_deref().unwrap_or("en ligne"));
        let message = Value::string(&text);
        let global = Value::Object(vm.global());
        if let Ok(console) = vm.get(&global, "console")
            && let Ok(log) = vm.get(&console, "error")
        {
            let _ = vm.call(&log, &console, &[message]);
        }
    }
    if let Some(h) = host(vm) {
        h.current_script = previous;
    }
}

/// Ce qu'il faut faire d'un script nouvellement inséré.
enum Action {
    Run(String),
    Fetch(url::Url),
    /// Adresse inutilisable : l'élément reçoit `error`.
    Fail,
}

/// Prend en charge les scripts insérés dans le document depuis le dernier
/// relevé.
pub(super) fn scan(vm: &mut Vm) {
    for _ in 0..MAX_CHAINED {
        let Some(h) = host(vm) else {
            return;
        };
        let version = h.doc.version();
        if h.scripts_scanned == version {
            return;
        }
        h.scripts_scanned = version;
        let Some(started) = h.scripts_started.as_mut() else {
            return;
        };
        let fresh: Vec<NodeId> = script_elements(&h.doc)
            .filter(|node| started.insert(node.index()))
            .collect();
        if fresh.is_empty() {
            return;
        }
        for node in fresh {
            start(vm, node, true);
        }
    }
}

/// Scripts qu'une insertion de `node` dans `parent` peut mettre en route :
/// `node` et ses descendants, et `parent` lui-même quand c'est un script qui
/// reçoit son texte. À relever avant l'insertion (un fragment se vide).
pub(super) fn within(doc: &Document, node: NodeId, parent: NodeId) -> Vec<NodeId> {
    let is_script = |n: &NodeId| doc.element(*n).is_some_and(|e| e.is_html("script"));
    // Les arbres fantômes des nœuds insérés rejoignent le document avec eux.
    let mut found = vec![parent];
    let mut pending = vec![node];
    while let Some(top) = pending.pop() {
        for inner in std::iter::once(top).chain(doc.descendants(top)) {
            found.push(inner);
            if let Some(shadow) = doc.shadow_root(inner) {
                pending.push(shadow);
            }
        }
    }
    found.into_iter().filter(is_script).collect()
}

/// Met en route, dans l'ordre, ceux des scripts `nodes` qu'une insertion
/// vient de relier au document : un script en ligne s'exécute sur le champ,
/// au milieu du script qui l'a inséré, comme le veut la norme.
pub(super) fn run_inserted(vm: &mut Vm, nodes: &[NodeId]) {
    for node in nodes {
        let Some(h) = host(vm) else {
            return;
        };
        // Hors d'une tâche de script, ou hors du document : rien à faire.
        let Some(started) = h.scripts_started.as_mut() else {
            return;
        };
        if h.doc.composed_root(*node) != h.doc.root() || !started.insert(node.index()) {
            continue;
        }
        // Pas de point de contrôle des micro-tâches : un script est en cours.
        start(vm, *node, false);
    }
}

/// Prend en charge le script `node`, déjà noté comme mis en route.
/// `checkpoint` : les micro-tâches s'exécutent après lui (fin de tâche).
fn start(vm: &mut Vm, node: NodeId, checkpoint: bool) {
    let Some(h) = host(vm) else {
        return;
    };
    let Some(element) = h.doc.element(node) else {
        return;
    };
    // Un module inséré avec `src` est importé.
    if crate::is_module_script(element.attr("type")) {
        let target = element
            .attr("src")
            .map(str::trim)
            .filter(|src| !src.is_empty())
            .and_then(|src| h.url.join(src).ok());
        if let Some(url) = target {
            super::modules::import_for_element(vm, node, &url);
            if checkpoint {
                vm.run_microtasks();
            }
        }
        return;
    }
    // Sinon, seuls les scripts classiques ; `nomodule` est un repli
    // pour les navigateurs sans modules.
    if !crate::is_classic_script(element.attr("type"))
        || element.has_attr("nomodule")
        || !crate::script_event_allows(element)
    {
        return;
    }
    let action = match element.attr("src").map(str::trim) {
        Some(src) if !src.is_empty() => match h.url.join(src) {
            Ok(url) if matches!(url.scheme(), "http" | "https") => Action::Fetch(url),
            _ => Action::Fail,
        },
        Some(_) => Action::Fail,
        None => Action::Run(child_text(&h.doc, node)),
    };
    match action {
        Action::Run(source) => {
            if source.trim().is_empty() {
                // Un script vide n'a pas commencé : il s'exécutera quand il
                // recevra son texte.
                if let Some(started) = h.scripts_started.as_mut() {
                    started.remove(&node.index());
                }
            } else {
                run_classic(vm, Some(node), &source);
                if checkpoint {
                    vm.run_microtasks();
                }
            }
        }
        Action::Fetch(url) => {
            if h.fetches.len() >= MAX_PENDING_FETCHES {
                dispatch_simple(vm, node, "error", false);
                return;
            }
            let id = NEXT_FETCH_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            h.fetch_queue.push(crate::FetchRequest {
                id,
                initiator: h.url.clone(),
                url: url.clone(),
                method: "GET".to_owned(),
                headers: Vec::new(),
                body: Vec::new(),
                origin: None,
                needs_preflight: false,
            });
            h.fetches.insert(
                id,
                PendingFetch {
                    url,
                    resolve: Value::Undefined,
                    reject: Value::Undefined,
                    script: Some(node),
                    module: None,
                },
            );
        }
        Action::Fail => {
            dispatch_simple(vm, node, "error", false);
        }
    }
}

/// La réponse au chargement du script `node` est arrivée : il s'exécute puis
/// reçoit `load` ; en cas d'échec, il reçoit `error`.
pub(super) fn loaded(vm: &mut Vm, node: NodeId, result: Result<crate::FetchResponse, String>) {
    match result {
        Ok(response) if (200..300).contains(&response.status) => {
            let source = String::from_utf8_lossy(&response.body);
            let source = source.strip_prefix('\u{feff}').unwrap_or(&source);
            run_classic(vm, Some(node), source);
            vm.run_microtasks();
            dispatch_simple(vm, node, "load", false);
        }
        _ => {
            dispatch_simple(vm, node, "error", false);
        }
    }
}
