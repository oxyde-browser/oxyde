//! Générateurs async (`async function*`) et itération asynchrone (`for await`).
//!
//! Un générateur async traite ses appels à `next`, `return` et `throw` un par
//! un : chaque appel rend une promesse et rejoint une file ; la coroutine
//! n'est reprise que lorsque la requête précédente est réglée.
//!
//! LIMITES CONNUES : `return()` sur un générateur suspendu n'exécute pas ses
//! blocs `finally` ; `yield*` ne transmet pas au générateur délégué les
//! valeurs de `next(v)`, ni `throw()`, ni `return()` ; un itérateur abandonné
//! par `break` dans un `for await` ne reçoit pas d'appel à `return()`.

use super::{arg, is_callable};
use crate::heap::{AsyncRequest, CoKind, CoState, Kind};
use crate::value::{ObjId, PropertyKey, Value};
use crate::vm::{CoOutcome, JsResult, NativeFn, Resume, Vm};

/// Objet désigné par `data[index]`.
fn data_object(data: &[Value], index: usize) -> Option<ObjId> {
    data.get(index).and_then(Value::as_object)
}

fn rejected(data: &[Value]) -> bool {
    data.get(1) == Some(&Value::Bool(true))
}

impl Vm {
    /// Coroutine de générateur async désignée par `value`.
    fn async_generator_id(&self, value: &Value) -> Option<ObjId> {
        let id = value.as_object()?;
        match &self.heap.get(id).kind {
            Kind::Coroutine(c) if c.kind == CoKind::AsyncGenerator => Some(id),
            _ => None,
        }
    }

    /// Retire la requête en tête de file et rend sa promesse ; le générateur
    /// redevient disponible.
    fn agen_complete(&mut self, generator: ObjId) -> Option<ObjId> {
        let Kind::Coroutine(c) = &mut self.heap.get_mut(generator).kind else {
            return None;
        };
        c.busy = false;
        c.queue.pop_front().map(|request| request.promise)
    }

    /// Traite les requêtes en attente tant que le générateur est disponible.
    fn agen_drain(&mut self, generator: ObjId) {
        loop {
            let Kind::Coroutine(c) = &mut self.heap.get_mut(generator).kind else {
                return;
            };
            if c.busy {
                return;
            }
            let Some(mode) = c.queue.front_mut().and_then(|request| request.mode.take()) else {
                return;
            };
            // Générateur terminé, ou jamais démarré et aussitôt clos : la
            // requête est réglée sans exécuter de code.
            let finished = match &c.state {
                CoState::Done => true,
                CoState::Start { .. } => !matches!(mode, Resume::Next(_)),
                _ => false,
            };
            if finished {
                c.state = CoState::Done;
                let Some(promise) = self.agen_complete(generator) else {
                    return;
                };
                match mode {
                    Resume::Next(_) => {
                        let result = self.iter_result(Value::Undefined, true);
                        self.settle(promise, false, result);
                    }
                    Resume::Throw(thrown) => self.settle(promise, true, thrown),
                    Resume::Return(value) => {
                        // La valeur est attendue avant d'être rendue.
                        let awaited = self.promise_resolve(&value);
                        let data = |rejected| vec![Value::Object(promise), Value::Bool(rejected)];
                        let on_fulfilled = self.new_native_data(agen_settle_request, data(false));
                        let on_rejected = self.new_native_data(agen_settle_request, data(true));
                        self.promise_then(awaited, on_fulfilled, on_rejected, None);
                    }
                }
                continue;
            }
            c.busy = true;
            self.agen_resume(generator, mode);
        }
    }

    /// Reprend le générateur et traite ce qu'il produit : valeur rendue,
    /// attente d'une promesse, fin ou exception.
    fn agen_resume(&mut self, generator: ObjId, mode: Resume) {
        let (reaction, awaited): (crate::vm::NativeDataFn, Value) =
            match self.resume(generator, mode) {
                Ok(CoOutcome::Await(value)) => (agen_awaited, value),
                // `yield v` attend `v` avant de le rendre.
                Ok(CoOutcome::Yield(value)) => (agen_yielded, value),
                Ok(CoOutcome::Return(value)) => (agen_returned, value),
                Err(thrown) => {
                    if let Some(promise) = self.agen_complete(generator) {
                        self.settle(promise, true, thrown);
                    }
                    return;
                }
            };
        let awaited = self.promise_resolve(&awaited);
        let data = |rejected| vec![Value::Object(generator), Value::Bool(rejected)];
        let on_fulfilled = self.new_native_data(reaction, data(false));
        let on_rejected = self.new_native_data(reaction, data(true));
        self.promise_then(awaited, on_fulfilled, on_rejected, None);
    }

    /// Itérateur asynchrone de `source` (`for await`, `yield*` async) : celui
    /// que rend `source[Symbol.asyncIterator]()`, sinon son itérateur
    /// synchrone habillé pour rendre des promesses.
    pub(crate) fn async_iterator_for(&mut self, source: &Value) -> JsResult {
        if !source.is_nullish() {
            let key = PropertyKey::Symbol(self.well_known.async_iterator.clone());
            let method = self.get_key(source, &key)?;
            if !method.is_nullish() && !is_callable(self, &method) {
                return Err(self.type_error("Symbol.asyncIterator n'est pas une fonction"));
            }
            if is_callable(self, &method) {
                let iterator = self.call(&method, source, &[])?;
                if iterator.as_object().is_none() {
                    return Err(
                        self.type_error("Symbol.asyncIterator doit renvoyer un objet itérateur")
                    );
                }
                return Ok(iterator);
            }
        }
        // `CreateAsyncFromSyncIterator` : l'itérateur synchrone de `source`,
        // habillé pour rendre des promesses.
        let key = PropertyKey::Symbol(self.well_known.iterator.clone());
        let method = self.get_key(source, &key)?;
        if !is_callable(self, &method) {
            let shown = self.display(source);
            return Err(self.type_error(&format!("{shown} n'est pas itérable")));
        }
        let iterator = self.call(&method, source, &[])?;
        if iterator.as_object().is_none() {
            return Err(self.type_error("Symbol.iterator doit renvoyer un objet itérateur"));
        }
        self.async_from_sync(iterator)
    }

    /// `CreateAsyncFromSyncIterator` : habille l'itérateur synchrone
    /// `iterator` pour qu'il rende des promesses.
    pub(crate) fn async_from_sync(&mut self, iterator: Value) -> JsResult {
        let next = self.get(&iterator, "next")?;
        let wrapper = self.new_object();
        for (name, function) in [
            ("next", sync_next as crate::vm::NativeDataFn),
            ("return", sync_return),
            ("throw", sync_throw),
        ] {
            let function = self.new_native_data(function, vec![iterator.clone(), next.clone()]);
            self.define_hidden(wrapper, name, function);
        }
        Ok(Value::Object(wrapper))
    }
}

/// Règle la promesse `data[0]` d'une requête `return(v)` adressée à un
/// générateur terminé.
fn agen_settle_request(vm: &mut Vm, data: &[Value], _this: &Value, args: &[Value]) -> JsResult {
    if let Some(promise) = data_object(data, 0) {
        if rejected(data) {
            vm.settle(promise, true, arg(args, 0));
        } else {
            let result = vm.iter_result(arg(args, 0), true);
            vm.settle(promise, false, result);
        }
    }
    Ok(Value::Undefined)
}

/// Suite d'un `await` : la coroutine reprend avec la valeur ou l'exception.
fn agen_awaited(vm: &mut Vm, data: &[Value], _this: &Value, args: &[Value]) -> JsResult {
    if let Some(generator) = data_object(data, 0) {
        let mode = if rejected(data) {
            Resume::Throw(arg(args, 0))
        } else {
            Resume::Next(arg(args, 0))
        };
        vm.agen_resume(generator, mode);
        vm.agen_drain(generator);
    }
    Ok(Value::Undefined)
}

/// Suite d'un `yield` : la valeur attendue règle la requête en cours ; un
/// rejet est levé dans le générateur, au point du `yield`.
fn agen_yielded(vm: &mut Vm, data: &[Value], _this: &Value, args: &[Value]) -> JsResult {
    if let Some(generator) = data_object(data, 0) {
        if rejected(data) {
            vm.agen_resume(generator, Resume::Throw(arg(args, 0)));
        } else if let Some(promise) = vm.agen_complete(generator) {
            let result = vm.iter_result(arg(args, 0), false);
            vm.settle(promise, false, result);
        }
        vm.agen_drain(generator);
    }
    Ok(Value::Undefined)
}

/// Suite de la fin du générateur : sa valeur de retour, une fois attendue,
/// règle la requête en cours.
fn agen_returned(vm: &mut Vm, data: &[Value], _this: &Value, args: &[Value]) -> JsResult {
    if let Some(generator) = data_object(data, 0)
        && let Some(promise) = vm.agen_complete(generator)
    {
        if rejected(data) {
            vm.settle(promise, true, arg(args, 0));
        } else {
            let result = vm.iter_result(arg(args, 0), true);
            vm.settle(promise, false, result);
        }
        vm.agen_drain(generator);
    }
    Ok(Value::Undefined)
}

/// Itérateur synchrone habillé (`data[0]`) et sa méthode `next` (`data[1]`).
fn sync_parts(data: &[Value]) -> (Value, Value) {
    (
        data.first().cloned().unwrap_or(Value::Undefined),
        data.get(1).cloned().unwrap_or(Value::Undefined),
    )
}

/// Appelle `return` sur l'itérateur synchrone, sans tenir compte du résultat.
fn sync_close(vm: &mut Vm, iterator: &Value) {
    if let Ok(method) = vm.get(iterator, "return")
        && !method.is_nullish()
    {
        let _ = vm.call(&method, iterator, &[]);
    }
}

/// `AsyncFromSyncIteratorContinuation` (ECMA-262 §27.1.6.4) : attend la
/// valeur du résultat synchrone et rend la promesse de `{ value, done }`.
/// `close` : fermer l'itérateur synchrone si la valeur est rejetée.
fn sync_continuation(vm: &mut Vm, iterator: &Value, result: JsResult, close: bool) -> Value {
    let step = (|| {
        let result = result?;
        if result.as_object().is_none() {
            return Err(vm.type_error("le résultat d'un itérateur doit être un objet"));
        }
        let done = vm.get(&result, "done")?.truthy();
        let value = vm.get(&result, "value")?;
        Ok((done, value))
    })();
    let (done, value) = match step {
        Ok(step) => step,
        Err(thrown) => return vm.promise_rejected(thrown),
    };
    let awaited = vm.promise_resolve(&value);
    let target = vm.new_promise();
    let wrap = vm.new_native_data(sync_result, vec![Value::Bool(done)]);
    let on_rejected = if close && !done {
        vm.new_native_data(sync_rejected, vec![iterator.clone()])
    } else {
        Value::Undefined
    };
    vm.promise_then(awaited, wrap, on_rejected, Some(target));
    Value::Object(target)
}

/// `next` de l'itérateur synchrone habillé.
fn sync_next(vm: &mut Vm, data: &[Value], _this: &Value, args: &[Value]) -> JsResult {
    let (iterator, next) = sync_parts(data);
    let result = vm.call(&next, &iterator, args.get(..1).unwrap_or(&[]));
    Ok(sync_continuation(vm, &iterator, result, true))
}

/// `return` de l'itérateur synchrone habillé.
fn sync_return(vm: &mut Vm, data: &[Value], _this: &Value, args: &[Value]) -> JsResult {
    let (iterator, _) = sync_parts(data);
    let method = match vm.get(&iterator, "return") {
        Ok(method) => method,
        Err(thrown) => return Ok(vm.promise_rejected(thrown)),
    };
    if method.is_nullish() {
        let result = vm.iter_result(arg(args, 0), true);
        return Ok(vm.promise_resolved(result));
    }
    let result = vm.call(&method, &iterator, args.get(..1).unwrap_or(&[]));
    Ok(sync_continuation(vm, &iterator, result, false))
}

/// `throw` de l'itérateur synchrone habillé.
fn sync_throw(vm: &mut Vm, data: &[Value], _this: &Value, args: &[Value]) -> JsResult {
    let (iterator, _) = sync_parts(data);
    let method = match vm.get(&iterator, "throw") {
        Ok(method) => method,
        Err(thrown) => return Ok(vm.promise_rejected(thrown)),
    };
    if method.is_nullish() {
        // L'itérateur ne sait pas recevoir d'exception : on le ferme.
        sync_close(vm, &iterator);
        let error = vm.type_error("l'itérateur n'a pas de méthode « throw »");
        return Ok(vm.promise_rejected(error));
    }
    let result = vm.call(&method, &iterator, args.get(..1).unwrap_or(&[]));
    Ok(sync_continuation(vm, &iterator, result, true))
}

/// Résultat `{ value, done }` une fois la valeur attendue ; `data[0]` : `done`.
fn sync_result(vm: &mut Vm, data: &[Value], _this: &Value, args: &[Value]) -> JsResult {
    let done = data.first() == Some(&Value::Bool(true));
    Ok(vm.iter_result(arg(args, 0), done))
}

/// La valeur attendue est rejetée : ferme l'itérateur synchrone `data[0]`
/// et propage le rejet.
fn sync_rejected(vm: &mut Vm, data: &[Value], _this: &Value, args: &[Value]) -> JsResult {
    let iterator = data.first().cloned().unwrap_or(Value::Undefined);
    sync_close(vm, &iterator);
    Err(arg(args, 0))
}

/// Met une requête en file et rend sa promesse.
fn request(vm: &mut Vm, this: &Value, mode: Resume) -> JsResult {
    let promise = vm.new_promise();
    let Some(generator) = vm.async_generator_id(this) else {
        let error = vm.type_error("le receveur n'est pas un générateur async");
        vm.settle(promise, true, error);
        return Ok(Value::Object(promise));
    };
    if let Kind::Coroutine(c) = &mut vm.heap.get_mut(generator).kind {
        c.queue.push_back(AsyncRequest {
            mode: Some(mode),
            promise,
        });
    }
    vm.agen_drain(generator);
    Ok(Value::Object(promise))
}

fn async_generator_next(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    request(vm, this, Resume::Next(arg(args, 0)))
}

fn async_generator_return(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    request(vm, this, Resume::Return(arg(args, 0)))
}

fn async_generator_throw(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    request(vm, this, Resume::Throw(arg(args, 0)))
}

fn async_iterator_self(_vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    Ok(this.clone())
}

pub(super) fn install(vm: &mut Vm) {
    let proto = vm.intrinsics.async_generator_proto;
    let methods: &[(&str, u32, NativeFn)] = &[
        ("next", 1, async_generator_next),
        ("return", 1, async_generator_return),
        ("throw", 1, async_generator_throw),
    ];
    for (name, arity, func) in methods {
        vm.define_method(proto, name, *arity, *func);
    }
    let known = vm.well_known.clone();
    let this_fn = vm.new_function("[Symbol.asyncIterator]", 0, async_iterator_self);
    vm.define_hidden_symbol(proto, &known.async_iterator, Value::Object(this_fn));
    vm.define_hidden_symbol(proto, &known.to_string_tag, Value::string("AsyncGenerator"));
}
