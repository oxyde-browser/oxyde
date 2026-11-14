//! Modules chargés à la demande (`import("…")`, `<script type="module">`
//! inséré par un script).
//!
//! Les modules que le HTML reçu importe statiquement sont fournis d'avance
//! par l'embarqueur (voir `Page::pending_modules`). Un `import()` vers un
//! module encore inconnu passe par ici : son texte est demandé par le
//! circuit de `fetch`, puis celui de chaque module qu'il importe
//! statiquement, et l'import reprend quand tout est arrivé.
//!
//! LIMITES CONNUES : un module d'une autre origine est accepté sans contrôle
//! CORS ; deux imports simultanés du même module le demandent deux fois ; le
//! texte est lu en UTF-8 ; un module en ligne inséré par un script n'est pas
//! exécuté.

use std::collections::HashSet;

use oxyde_dom::NodeId;
use oxyde_js::{Value, Vm};
use url::Url;

use super::{MAX_PENDING_FETCHES, NEXT_FETCH_ID, PendingFetch, host, report};

/// Nombre maximal de modules demandés pour un même `import()`.
const MAX_MODULES_PER_LOAD: usize = 512;

/// Chargement en cours : un `import()` et les modules qu'il attend.
pub(super) struct ModuleLoad {
    /// Fonction `prêt` de l'import, gardée en vie jusqu'à son appel.
    ready: Value,
    /// Adresse du module demandé.
    root: String,
    /// Modules déjà demandés pour cet import.
    seen: HashSet<String>,
    /// Réponses encore attendues.
    waiting: usize,
    /// Raison de l'échec du module demandé, s'il a échoué.
    failed: Option<String>,
}

/// Demande le texte du module `url` pour le chargement `load`.
fn request(vm: &mut Vm, load: usize, url: Url) {
    let Some(h) = host(vm) else {
        return;
    };
    let Some(Some(state)) = h.module_loads.get_mut(load) else {
        return;
    };
    if state.seen.len() >= MAX_MODULES_PER_LOAD
        || h.fetches.len() >= MAX_PENDING_FETCHES * 8
        || !state.seen.insert(url.to_string())
    {
        return;
    }
    state.waiting += 1;
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
            script: None,
            module: Some(load),
        },
    );
}

/// Préparation d'un `import()` (voir [`oxyde_js::ImportPreparer`]) : rend
/// `true` si le module doit d'abord être chargé.
pub(super) fn prepare(vm: &mut Vm, specifier: &str, referrer: &str, ready: &Value) -> bool {
    let Some(h) = host(vm) else {
        return false;
    };
    let Ok(referrer) = Url::parse(referrer) else {
        return false;
    };
    let Some(url) = h.import_map.resolve(&referrer, specifier) else {
        return false;
    };
    // Déjà fourni, ou hors de portée du réseau : le chargeur tranchera.
    if h.modules.contains_key(url.as_str()) || !matches!(url.scheme(), "http" | "https") {
        return false;
    }
    let state = ModuleLoad {
        ready: ready.clone(),
        root: url.to_string(),
        seen: HashSet::new(),
        waiting: 0,
        failed: None,
    };
    let load = match h.module_loads.iter().position(Option::is_none) {
        Some(free) => {
            if let Some(slot) = h.module_loads.get_mut(free) {
                *slot = Some(state);
            }
            free
        }
        None => {
            h.module_loads.push(Some(state));
            h.module_loads.len() - 1
        }
    };
    vm.add_root(ready.clone());
    request(vm, load, url);
    // Rien n'a pu partir (trop de requêtes en cours) : l'import échoue.
    if host(vm).is_some_and(|h| {
        h.module_loads
            .get(load)
            .and_then(Option::as_ref)
            .is_some_and(|state| state.waiting == 0)
    }) {
        finish(vm, load, Some("trop de requêtes en cours".to_owned()));
    }
    true
}

/// Clôt le chargement `load` : l'import reprend, ou échoue avec `failure`.
fn finish(vm: &mut Vm, load: usize, failure: Option<String>) {
    let Some(state) = host(vm).and_then(|h| h.module_loads.get_mut(load)?.take()) else {
        return;
    };
    vm.remove_root(&state.ready);
    let args: Vec<Value> = failure
        .or(state.failed)
        .map(|message| Value::string(&message))
        .into_iter()
        .collect();
    if let Err(thrown) = vm.call(&state.ready, &Value::Undefined, &args) {
        report(vm, &thrown);
    }
    vm.run_microtasks();
}

/// La réponse pour le module `url` du chargement `load` est arrivée.
pub(super) fn loaded(
    vm: &mut Vm,
    load: usize,
    url: &Url,
    result: Result<crate::FetchResponse, String>,
) {
    let text = match result {
        Ok(response) if (200..300).contains(&response.status) => {
            let text = String::from_utf8_lossy(&response.body).into_owned();
            Ok(text
                .strip_prefix('\u{feff}')
                .map_or(text.clone(), str::to_owned))
        }
        Ok(response) => Err(format!("le serveur a répondu {}", response.status)),
        Err(message) => Err(message),
    };
    let mut wanted = Vec::new();
    if let Some(h) = host(vm) {
        match text {
            Ok(text) => {
                // Les modules qu'il importe statiquement sont demandés à leur tour.
                for specifier in oxyde_js::module_requests(&text).unwrap_or_default() {
                    if let Some(dependency) = h.import_map.resolve(url, &specifier)
                        && matches!(dependency.scheme(), "http" | "https")
                        && !h.modules.contains_key(dependency.as_str())
                    {
                        wanted.push(dependency);
                    }
                }
                h.modules.insert(url.to_string(), text);
            }
            Err(message) => {
                if let Some(Some(state)) = h.module_loads.get_mut(load)
                    && state.root == url.as_str()
                {
                    state.failed = Some(message);
                }
            }
        }
    }
    for dependency in wanted {
        request(vm, load, dependency);
    }
    let done = host(vm).is_some_and(|h| {
        h.module_loads
            .get_mut(load)
            .and_then(Option::as_mut)
            .is_some_and(|state| {
                state.waiting = state.waiting.saturating_sub(1);
                state.waiting == 0
            })
    });
    if done {
        finish(vm, load, None);
    }
}

/// Un script a inséré `<script type="module" src>` : le module est importé,
/// puis l'élément reçoit `load` ou `error`.
pub(super) fn import_for_element(vm: &mut Vm, node: NodeId, url: &Url) {
    let Some(h) = host(vm) else {
        return;
    };
    let referrer = h.url.to_string();
    let Some(hooks) = h.hooks.clone() else {
        return;
    };
    let promise = vm.import_module(url.as_str(), &referrer);
    let element = super::wrap(vm, node);
    if let Err(thrown) = vm
        .get(&hooks, "moduleScript")
        .and_then(|notify| vm.call(&notify, &hooks, &[element, promise]))
    {
        report(vm, &thrown);
    }
}
