//! Moteur JavaScript d'Oxyde, réécriture de V8 en Rust sûr.
//!
//! Même découpage que V8 : analyse ([`lexer`], `parser`), compilation vers un
//! bytecode à registres (`compiler`, à la manière d'Ignition), interprétation
//! (`vm`) et ramasse-miettes traçant (`heap`).
//!
//! ```
//! let mut vm = oxyde_js::Vm::new();
//! let value = vm.eval("[1, 2, 3].map(x => x * 2).join('-')").expect("script valide");
//! assert_eq!(vm.display(&value), "2-4-6");
//! ```
//!
//! ÉTAT : interpréteur seul (pas de compilation à la volée), ramasse-miettes
//! non générationnel. Ce qui manque au langage est listé en tête de
//! `parser`, `compiler` et `builtins`, et dans `conformance/README.md`.

// Les conversions portent le nom des opérations abstraites d'ECMA-262
// (ToNumber, ToString, ToObject…) et ont besoin de la machine en écriture.
#![allow(clippy::wrong_self_convention, reason = "noms repris d'ECMA-262")]

mod ast;
pub mod bigint;
mod builtins;
mod bytecode;
mod compiler;
mod heap;
mod internal;
pub mod lexer;
mod module;
mod object;
mod ops;
mod parser;
mod regexp;
pub mod value;
mod vm;

pub use builtins::ConsoleMessage;
pub use lexer::SyntaxError;
pub use module::{ModuleLoader, module_requests};
pub use value::{JsStr, ObjId, PropertyKey, Symbol, Value};
pub use vm::{HostHooks, JsResult, NativeDataFn, NativeFn, Vm};

/// Échec de l'évaluation d'un script.
#[derive(Clone, Debug, PartialEq)]
pub enum EvalError {
    /// Le source n'est pas un script valide (ou utilise une syntaxe non prise en charge).
    Syntax(SyntaxError),
    /// Le script a levé une exception non rattrapée.
    Thrown {
        /// Valeur levée.
        value: Value,
        /// Sa représentation (« TypeError: … »).
        message: String,
    },
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Syntax(e) => write!(f, "SyntaxError: {e}"),
            Self::Thrown { message, .. } => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for EvalError {}

impl Vm {
    /// Compile et exécute `source` comme un script ; renvoie la valeur de sa
    /// dernière instruction-expression de premier niveau.
    ///
    /// L'analyse et l'exécution sont récursives : à appeler depuis un fil
    /// disposant d'une pile confortable, voir [`with_large_stack`].
    ///
    /// # Errors
    /// [`EvalError::Syntax`] si le source est invalide, [`EvalError::Thrown`]
    /// si le script lève une exception non rattrapée.
    pub fn eval(&mut self, source: &str) -> Result<Value, EvalError> {
        let script = parser::parse_program(source).map_err(EvalError::Syntax)?;
        let proto = compiler::compile_script(&script.body, script.strict, false, false, false)
            .map_err(EvalError::Syntax)?;
        let result = self.run_script(proto);
        // Fin de la tâche : les réactions de promesses en attente s'exécutent.
        self.run_microtasks();
        result.map_err(|value| {
            let message = self.display(&value);
            EvalError::Thrown { value, message }
        })
    }

    /// Compile et exécute `source` comme un script global depuis du code en
    /// cours d'exécution (`eval`, `Function`). Une erreur de syntaxe devient
    /// une exception `SyntaxError`.
    pub(crate) fn eval_global(&mut self, source: &str, strict: bool) -> JsResult {
        let proto = parser::parse_eval(source, strict)
            .and_then(|script| {
                compiler::compile_script(&script.body, script.strict, false, false, true)
            })
            .map_err(|error| self.error("SyntaxError", &error.message))?;
        self.run_script(proto)
    }

    /// Compile et exécute `source` comme un script distinct, depuis du code
    /// en cours d'exécution (une fonction de l'hôte qui évalue un script).
    ///
    /// # Errors
    /// `SyntaxError` si le source est invalide ; toute valeur levée par le script.
    pub fn eval_script(&mut self, source: &str) -> vm::JsResult {
        let proto = parser::parse_program(source)
            .and_then(|script| {
                compiler::compile_script(&script.body, script.strict, false, false, false)
            })
            .map_err(|error| self.error("SyntaxError", &error.message))?;
        self.run_script(proto)
    }

    /// Messages écrits par `console.*` depuis le dernier appel, dans l'ordre.
    pub fn take_console(&mut self) -> Vec<ConsoleMessage> {
        std::mem::take(&mut self.console)
    }
}

/// Taille de la pile du fil d'exécution des scripts (64 Mio, réservés mais
/// engagés seulement à l'usage).
const SCRIPT_STACK_BYTES: usize = 64 * 1024 * 1024;

/// Exécute `work` sur un fil dédié doté d'une grande pile, et attend son résultat.
///
/// L'analyseur, le compilateur et les appels natifs → JavaScript sont
/// récursifs ; leurs profondeurs sont bornées, mais ces bornes supposent
/// davantage de pile que le fil principal d'une application Windows (1 Mio).
///
/// # Panics
/// Propage une panique survenue dans `work`.
pub fn with_large_stack<T: Send>(work: impl FnOnce() -> T + Send) -> T {
    std::thread::scope(|scope| {
        let spawned = std::thread::Builder::new()
            .name("oxyde-js".to_owned())
            .stack_size(SCRIPT_STACK_BYTES)
            .spawn_scoped(scope, work);
        match spawned {
            Ok(handle) => match handle.join() {
                Ok(value) => value,
                Err(panic) => std::panic::resume_unwind(panic),
            },
            // Impossible de créer le fil (ressources épuisées) : on ne peut
            // rien exécuter sans risquer de déborder la pile courante.
            Err(error) => {
                std::panic::panic_any(format!("création du fil JavaScript impossible : {error}"))
            }
        }
    })
}

#[cfg(test)]
mod tests;
