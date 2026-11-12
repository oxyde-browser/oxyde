//! Gestion explicite des ressources : `DisposableStack`,
//! `AsyncDisposableStack`, `%IteratorPrototype%[Symbol.dispose]` et
//! `%AsyncIteratorPrototype%[Symbol.asyncDispose]`.
//!
//! L'essentiel est écrit en JavaScript (`disposable.js`) : la libération
//! asynchrone est une fonction async, que le moteur sait déjà exécuter, et
//! les marques internes des piles sont des champs privés.

use crate::value::Value;
use crate::vm::{JsResult, Vm};

/// Emplacement interne de l'objet global où vivent les opérations de `using`.
const USING_SLOT: &str = "usingOperations";

impl Vm {
    /// Opération `name` de `using` (`newStack`, `addResource`, `disposeSync`,
    /// `disposeAsync`) dans le royaume courant.
    pub(crate) fn using_operation(&mut self, name: &str) -> JsResult {
        match self.slot(self.global, USING_SLOT) {
            Some(parts) => self.get(&parts, name),
            None => Err(self.type_error("« using » n'est pas disponible dans ce royaume")),
        }
    }
}

/// Installe les piles de ressources et les méthodes de libération des
/// itérateurs dans le royaume courant.
pub(super) fn install(vm: &mut Vm) {
    let Ok(parts) = vm.eval_script(include_str!("disposable.js")) else {
        return;
    };
    // Les opérations de `using` : gardées dans un emplacement interne de
    // l'objet global, hors de portée des scripts.
    vm.set_slot(vm.global, USING_SLOT, parts.clone());
    let known = vm.well_known.clone();
    for name in ["DisposableStack", "AsyncDisposableStack"] {
        if let Ok(class) = vm.get(&parts, name) {
            vm.define_hidden(vm.global, name, class);
        }
    }
    if let Ok(dispose) = vm.get(&parts, "iteratorDispose") {
        let root = vm.intrinsics.iterator_root;
        vm.define_hidden_symbol(root, &known.dispose, dispose);
    }
    // `%AsyncIteratorPrototype%` est le prototype de celui des générateurs async.
    let async_root = vm.heap.get(vm.intrinsics.async_generator_proto).proto;
    if let (Some(async_root), Ok(dispose)) = (async_root, vm.get(&parts, "asyncIteratorDispose")) {
        vm.define_hidden_symbol(async_root, &known.async_dispose, dispose);
    }
    // Ces fonctions sont des intégrés : `toString` les montre natives, et un
    // constructeur garde son prototype d'origine quand `new.target` n'en
    // fournit pas.
    if let Ok(natives) = vm.get(&parts, "natives") {
        let mut index = 0;
        while let Ok(Value::Object(id)) = vm.get(&natives, &index.to_string()) {
            vm.self_hosted.push(id);
            index += 1;
        }
    }
}
