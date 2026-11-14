//! Scripts insérés par les scripts (`document.createElement("script")`).
//!
//! Les scripts du HTML reçu sont exécutés par la page, dans l'ordre du
//! document. Ceux qu'un script insère ensuite sont pris en charge ici, à la
//! fin de la tâche qui les a insérés : un script en ligne s'exécute aussitôt ;
//! un script externe est demandé par le circuit de `fetch`, s'exécute à
//! l'arrivée de sa réponse, puis reçoit `load` (ou `error`).
//!
//! LIMITES CONNUES : un module inséré (`type="module"`) n'est pas chargé ;
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

/// Relève, à la première tâche, les scripts du HTML reçu : la page les
/// exécute elle-même.
pub(super) fn note_existing(vm: &mut Vm) {
    let Some(h) = host(vm) else {
        return;
    };
    if h.scripts_started.is_none() {
        let existing: HashSet<usize> = script_elements(&h.doc).map(|n| n.index()).collect();
        h.scripts_started = Some(existing);
        h.scripts_scanned = h.doc.version();
    }
}

/// Exécute le script classique `source` ; `element` est son élément
/// (`document.currentScript`). Une exception est signalée dans la console.
pub(super) fn run_classic(vm: &mut Vm, element: Option<NodeId>, source: &str) {
    let previous = host(vm).and_then(|h| std::mem::replace(&mut h.current_script, element));
    if let Err(error) = vm.eval(source) {
        let message = Value::string(&format!("Exception non rattrapée : {error}"));
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
            let Some(h) = host(vm) else {
                return;
            };
            let Some(element) = h.doc.element(node) else {
                continue;
            };
            // Seuls les scripts classiques ; `nomodule` est un repli pour les
            // navigateurs sans modules.
            if !crate::is_classic_script(element.attr("type")) || element.has_attr("nomodule") {
                continue;
            }
            let action = match element.attr("src").map(str::trim) {
                Some(src) if !src.is_empty() => match h.url.join(src) {
                    Ok(url) if matches!(url.scheme(), "http" | "https") => Action::Fetch(url),
                    _ => Action::Fail,
                },
                Some(_) => Action::Fail,
                None => Action::Run(h.doc.text_content(node)),
            };
            match action {
                Action::Run(source) => {
                    if !source.trim().is_empty() {
                        run_classic(vm, Some(node), &source);
                        vm.run_microtasks();
                    }
                }
                Action::Fetch(url) => {
                    if h.fetches.len() >= MAX_PENDING_FETCHES {
                        dispatch_simple(vm, node, "error", false);
                        continue;
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
                        },
                    );
                }
                Action::Fail => {
                    dispatch_simple(vm, node, "error", false);
                }
            }
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
