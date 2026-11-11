//! `Promise`, file de microtâches et pilotage des fonctions async.
//!
//! Les réactions (`then`) ne s'exécutent jamais pendant l'appel qui les
//! déclenche : elles sont mises en file et exécutées par
//! [`Vm::run_microtasks`], que l'hôte appelle à la fin de chaque tâche.
//!
//! LIMITES CONNUES : les combinateurs (`all`, `race`…) et `await` attachent
//! leurs réactions directement, sans passer par une méthode `then` redéfinie ;
//! `Promise.resolve`, `all`… ignorent `this` (pas de sous-classes) ; pas de
//! `Promise.withResolvers` ni `Promise.try`.

use super::{arg, constructor, is_callable};
use crate::heap::{Kind, Object, PromiseData, PromiseState, Reaction};
use crate::value::{JsStr, ObjId, Value};
use crate::vm::{CoOutcome, JsResult, NativeFn, Resume, Vm};

/// Microtâche en attente.
#[derive(Clone, Debug)]
pub(crate) enum Job {
    /// Réaction d'une promesse réglée : appelle `handler(argument)` puis
    /// résout `target` avec le résultat.
    Reaction {
        handler: Value,
        argument: Value,
        rejected: bool,
        target: Option<ObjId>,
    },
    /// Résolution par un objet « thenable » : appelle `then(resolve, reject)`.
    Thenable {
        promise: ObjId,
        thenable: Value,
        then: Value,
    },
    /// `import()` : charge et évalue le module, puis règle `promise`.
    Import {
        promise: ObjId,
        specifier: JsStr,
        referrer: JsStr,
    },
}

impl Job {
    /// Ajoute à `out` les valeurs que la tâche garde en vie.
    pub(crate) fn trace(&self, out: &mut Vec<Value>) {
        match self {
            Self::Reaction {
                handler,
                argument,
                target,
                ..
            } => {
                out.push(handler.clone());
                out.push(argument.clone());
                out.extend(target.map(Value::Object));
            }
            Self::Thenable {
                promise,
                thenable,
                then,
            } => {
                out.push(Value::Object(*promise));
                out.push(thenable.clone());
                out.push(then.clone());
            }
            Self::Import { promise, .. } => out.push(Value::Object(*promise)),
        }
    }
}

/// Nombre maximal de microtâches en attente : borne la mémoire qu'un script
/// peut réserver en empilant des réactions plus vite qu'elles ne s'exécutent.
const MAX_JOBS: usize = 1_000_000;

/// Nombre maximal de rejets non gérés gardés en attente de signalement.
const MAX_PENDING_REJECTIONS: usize = 1_000;

impl Vm {
    /// Met une microtâche en file. Au-delà de [`MAX_JOBS`], la tâche est
    /// abandonnée et l'incident signalé une fois dans la console.
    fn enqueue(&mut self, job: Job) {
        if self.jobs.len() < MAX_JOBS {
            self.jobs.push_back(job);
        } else if self.jobs.len() == MAX_JOBS {
            super::console_message(
                self,
                "error",
                "file de microtâches saturée : des réactions de promesses sont abandonnées"
                    .to_owned(),
            );
            // Dépasse la borne d'un cran pour ne signaler l'incident qu'une fois.
            self.jobs.push_back(job);
        }
    }

    /// Nouvelle promesse en attente.
    pub(crate) fn new_promise(&mut self) -> ObjId {
        let data = PromiseData {
            state: PromiseState::Pending,
            reactions: Vec::new(),
            handled: false,
        };
        self.heap.alloc(Object::new(
            Some(self.intrinsics.promise_proto),
            Kind::Promise(Box::new(data)),
        ))
    }

    fn promise_data(&mut self, promise: ObjId) -> Option<&mut PromiseData> {
        match &mut self.heap.get_mut(promise).kind {
            Kind::Promise(data) => Some(data),
            _ => None,
        }
    }

    /// Règle `promise` (tenue ou rejetée) si elle est encore en attente, et
    /// met ses réactions en file.
    pub(crate) fn settle(&mut self, promise: ObjId, rejected: bool, value: Value) {
        let Some(data) = self.promise_data(promise) else {
            return;
        };
        if !matches!(data.state, PromiseState::Pending) {
            return;
        }
        data.state = if rejected {
            PromiseState::Rejected(value.clone())
        } else {
            PromiseState::Fulfilled(value.clone())
        };
        let reactions = std::mem::take(&mut data.reactions);
        let handled = data.handled;
        if rejected && !handled && self.rejections.len() < MAX_PENDING_REJECTIONS {
            self.rejections.push(promise);
        }
        for reaction in reactions {
            self.enqueue(Job::Reaction {
                handler: if rejected {
                    reaction.on_rejected
                } else {
                    reaction.on_fulfilled
                },
                argument: value.clone(),
                rejected,
                target: reaction.target,
            });
        }
    }

    /// Résout `promise` avec `value` : si c'est un objet muni d'une méthode
    /// `then`, la promesse suivra son sort ; sinon elle est tenue.
    pub(crate) fn resolve_promise(&mut self, promise: ObjId, value: Value) {
        if let Value::Object(id) = &value {
            if *id == promise {
                let error = self.type_error("une promesse ne peut pas se résoudre avec elle-même");
                return self.settle(promise, true, error);
            }
            match self.get(&value, "then") {
                Err(thrown) => return self.settle(promise, true, thrown),
                Ok(then) if is_callable(self, &then) => {
                    self.enqueue(Job::Thenable {
                        promise,
                        thenable: value,
                        then,
                    });
                    return;
                }
                Ok(_) => {}
            }
        }
        self.settle(promise, false, value);
    }

    /// Attache des gestionnaires à `promise` ; `target`, s'il existe, est
    /// résolu avec le résultat du gestionnaire appelé.
    pub(crate) fn promise_then(
        &mut self,
        promise: ObjId,
        on_fulfilled: Value,
        on_rejected: Value,
        target: Option<ObjId>,
    ) {
        let Some(data) = self.promise_data(promise) else {
            return;
        };
        data.handled = true;
        let job = match &data.state {
            PromiseState::Pending => {
                data.reactions.push(Reaction {
                    on_fulfilled,
                    on_rejected,
                    target,
                });
                return;
            }
            PromiseState::Fulfilled(value) => Job::Reaction {
                handler: on_fulfilled,
                argument: value.clone(),
                rejected: false,
                target,
            },
            PromiseState::Rejected(value) => Job::Reaction {
                handler: on_rejected,
                argument: value.clone(),
                rejected: true,
                target,
            },
        };
        self.enqueue(job);
    }

    /// `value` si c'est déjà une promesse, sinon une promesse résolue avec.
    pub(crate) fn promise_resolve(&mut self, value: &Value) -> ObjId {
        if let Some(id) = value.as_object()
            && matches!(self.heap.get(id).kind, Kind::Promise(_))
        {
            return id;
        }
        let promise = self.new_promise();
        self.resolve_promise(promise, value.clone());
        promise
    }

    /// Fonctions `resolve` et `reject` d'une promesse ; seul le premier appel
    /// à l'une des deux a un effet.
    fn resolvers(&mut self, promise: ObjId) -> (Value, Value) {
        let flag = self
            .heap
            .alloc(Object::new(None, Kind::Primitive(Value::Bool(false))));
        let data = vec![Value::Object(promise), Value::Object(flag)];
        (
            self.new_native_data(resolver_resolve, data.clone()),
            self.new_native_data(resolver_reject, data),
        )
    }

    /// Nouvelle promesse en attente, avec ses fonctions `resolve` et `reject` :
    /// `(promesse, resolve, reject)`. Sert à l'hôte pour ses opérations
    /// asynchrones (`fetch`…), qu'il conclut en appelant l'une des fonctions.
    pub fn promise_with_resolvers(&mut self) -> (Value, Value, Value) {
        let promise = self.new_promise();
        let (resolve, reject) = self.resolvers(promise);
        (Value::Object(promise), resolve, reject)
    }

    /// Promesse déjà résolue avec `value`.
    pub fn promise_resolved(&mut self, value: Value) -> Value {
        let promise = self.new_promise();
        self.resolve_promise(promise, value);
        Value::Object(promise)
    }

    /// Promesse déjà rejetée avec `reason`.
    pub fn promise_rejected(&mut self, reason: Value) -> Value {
        let promise = self.new_promise();
        self.settle(promise, true, reason);
        Value::Object(promise)
    }

    /// Exécute les microtâches en attente jusqu'à vider la file, puis signale
    /// dans la console les promesses rejetées sans gestionnaire.
    ///
    /// À appeler par l'hôte à la fin de chaque tâche (script, minuteur,
    /// événement). Sans effet si du code JavaScript est en cours d'exécution.
    pub fn run_microtasks(&mut self) {
        if !self.at_top_level() {
            return;
        }
        while let Some(job) = self.jobs.pop_front() {
            if self.is_interrupted() {
                // Budget épuisé : plus rien ne peut s'exécuter.
                self.jobs.clear();
                break;
            }
            // Pendant une tâche, du code Rust garde des identifiants d'objets :
            // le ramasse-miettes ne passe qu'entre deux tâches, quand tout ce
            // qui vit est atteignable depuis la file ou les racines.
            self.without_gc(|vm| vm.run_job(job));
            if self.heap.should_collect() {
                self.collect_garbage();
            }
        }
        let pending = std::mem::take(&mut self.rejections);
        self.without_gc(|vm| {
            for promise in pending {
                let reason = match &vm.heap.get(promise).kind {
                    Kind::Promise(data) if !data.handled => match &data.state {
                        PromiseState::Rejected(reason) => Some(reason.clone()),
                        _ => None,
                    },
                    _ => None,
                };
                if let Some(reason) = reason {
                    let shown = vm.display(&reason);
                    super::console_message(
                        vm,
                        "error",
                        format!("Promesse rejetée sans gestionnaire : {shown}"),
                    );
                }
            }
        });
    }

    pub(crate) fn run_job(&mut self, job: Job) {
        match job {
            Job::Import {
                promise,
                specifier,
                referrer,
            } => self.run_import(promise, &specifier, &referrer),
            Job::Reaction {
                handler,
                argument,
                rejected,
                target,
            } => {
                let outcome = if is_callable(self, &handler) {
                    self.call(&handler, &Value::Undefined, &[argument])
                } else if rejected {
                    Err(argument)
                } else {
                    Ok(argument)
                };
                if let Some(target) = target {
                    match outcome {
                        Ok(value) => self.resolve_promise(target, value),
                        Err(thrown) => self.settle(target, true, thrown),
                    }
                }
            }
            Job::Thenable {
                promise,
                thenable,
                then,
            } => {
                let (resolve, reject) = self.resolvers(promise);
                if let Err(thrown) = self.call(&then, &thenable, &[resolve, reject.clone()]) {
                    // Sans effet si `then` a déjà appelé `resolve` ou `reject`.
                    let _ = self.call(&reject, &Value::Undefined, &[thrown]);
                }
            }
        }
    }

    /// Fait avancer la fonction async `coroutine` jusqu'à son prochain `await`
    /// ou sa fin, et règle sa promesse en conséquence.
    pub(crate) fn async_step(&mut self, coroutine: ObjId, mode: Resume) {
        let Some(promise) = self.coroutine_promise(coroutine) else {
            return;
        };
        match self.resume(coroutine, mode) {
            Ok(CoOutcome::Return(value)) => self.resolve_promise(promise, value),
            Err(thrown) => self.settle(promise, true, thrown),
            Ok(CoOutcome::Yield(awaited) | CoOutcome::Await(awaited)) => {
                let awaited = self.promise_resolve(&awaited);
                let data = vec![Value::Object(coroutine)];
                let on_fulfilled = self.new_native_data(async_fulfilled, data.clone());
                let on_rejected = self.new_native_data(async_rejected, data);
                self.promise_then(awaited, on_fulfilled, on_rejected, None);
            }
        }
    }
}

/// Objet désigné par `data[index]`.
fn data_object(data: &[Value], index: usize) -> Option<ObjId> {
    data.get(index).and_then(Value::as_object)
}

/// Lève le verrou « déjà résolue » partagé par `resolve` et `reject` ; renvoie
/// la promesse si c'est le premier appel.
fn take_resolver(vm: &mut Vm, data: &[Value]) -> Option<ObjId> {
    let promise = data_object(data, 0)?;
    let flag = data_object(data, 1)?;
    let Kind::Primitive(state) = &mut vm.heap.get_mut(flag).kind else {
        return None;
    };
    if *state == Value::Bool(true) {
        return None;
    }
    *state = Value::Bool(true);
    Some(promise)
}

fn resolver_resolve(vm: &mut Vm, data: &[Value], _this: &Value, args: &[Value]) -> JsResult {
    if let Some(promise) = take_resolver(vm, data) {
        vm.resolve_promise(promise, arg(args, 0));
    }
    Ok(Value::Undefined)
}

fn resolver_reject(vm: &mut Vm, data: &[Value], _this: &Value, args: &[Value]) -> JsResult {
    if let Some(promise) = take_resolver(vm, data) {
        vm.settle(promise, true, arg(args, 0));
    }
    Ok(Value::Undefined)
}

fn async_fulfilled(vm: &mut Vm, data: &[Value], _this: &Value, args: &[Value]) -> JsResult {
    if let Some(coroutine) = data_object(data, 0) {
        vm.async_step(coroutine, Resume::Next(arg(args, 0)));
    }
    Ok(Value::Undefined)
}

fn async_rejected(vm: &mut Vm, data: &[Value], _this: &Value, args: &[Value]) -> JsResult {
    if let Some(coroutine) = data_object(data, 0) {
        vm.async_step(coroutine, Resume::Throw(arg(args, 0)));
    }
    Ok(Value::Undefined)
}

// ---- Constructeur et prototype ---------------------------------------------------

fn promise_ctor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    if !vm.constructing {
        return Err(vm.type_error("Promise doit être appelé avec « new »"));
    }
    let executor = super::callable(vm, &arg(args, 0))?;
    let promise = vm.new_promise();
    let (resolve, reject) = vm.resolvers(promise);
    if let Err(thrown) = vm.call(&executor, &Value::Undefined, &[resolve, reject.clone()]) {
        vm.call(&reject, &Value::Undefined, &[thrown])?;
    }
    Ok(Value::Object(promise))
}

fn this_promise(vm: &mut Vm, this: &Value) -> JsResult<ObjId> {
    match this.as_object() {
        Some(id) if matches!(vm.heap.get(id).kind, Kind::Promise(_)) => Ok(id),
        _ => Err(vm.type_error("le receveur n'est pas une promesse")),
    }
}

fn promise_then(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let promise = this_promise(vm, this)?;
    let target = vm.new_promise();
    vm.promise_then(promise, arg(args, 0), arg(args, 1), Some(target));
    Ok(Value::Object(target))
}

fn promise_catch(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let promise = this_promise(vm, this)?;
    let target = vm.new_promise();
    vm.promise_then(promise, Value::Undefined, arg(args, 0), Some(target));
    Ok(Value::Object(target))
}

fn promise_finally(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let promise = this_promise(vm, this)?;
    let target = vm.new_promise();
    let callback = arg(args, 0);
    if is_callable(vm, &callback) {
        let on_fulfilled = vm.new_native_data(finally_fulfilled, vec![callback.clone()]);
        let on_rejected = vm.new_native_data(finally_rejected, vec![callback]);
        vm.promise_then(promise, on_fulfilled, on_rejected, Some(target));
    } else {
        vm.promise_then(promise, callback.clone(), callback, Some(target));
    }
    Ok(Value::Object(target))
}

/// Appelle le rappel de `finally`, attend son résultat, puis laisse passer
/// l'issue d'origine au moyen de `pass`.
fn run_finally(
    vm: &mut Vm,
    data: &[Value],
    pass: crate::vm::NativeDataFn,
    outcome: Value,
) -> JsResult {
    let callback = data.first().cloned().unwrap_or(Value::Undefined);
    let result = vm.call(&callback, &Value::Undefined, &[])?;
    let awaited = vm.promise_resolve(&result);
    let target = vm.new_promise();
    let pass = vm.new_native_data(pass, vec![outcome]);
    vm.promise_then(awaited, pass, Value::Undefined, Some(target));
    Ok(Value::Object(target))
}

fn finally_fulfilled(vm: &mut Vm, data: &[Value], _this: &Value, args: &[Value]) -> JsResult {
    run_finally(vm, data, return_value, arg(args, 0))
}

fn finally_rejected(vm: &mut Vm, data: &[Value], _this: &Value, args: &[Value]) -> JsResult {
    run_finally(vm, data, throw_value, arg(args, 0))
}

fn return_value(_vm: &mut Vm, data: &[Value], _this: &Value, _args: &[Value]) -> JsResult {
    Ok(data.first().cloned().unwrap_or(Value::Undefined))
}

fn throw_value(_vm: &mut Vm, data: &[Value], _this: &Value, _args: &[Value]) -> JsResult {
    Err(data.first().cloned().unwrap_or(Value::Undefined))
}

// ---- Méthodes statiques ----------------------------------------------------------

fn promise_static_resolve(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    Ok(Value::Object(vm.promise_resolve(&arg(args, 0))))
}

fn promise_static_reject(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let promise = vm.new_promise();
    vm.settle(promise, true, arg(args, 0));
    Ok(Value::Object(promise))
}

/// Combinateur : quelle issue de chaque élément compte, et comment conclure.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Combine {
    All,
    AllSettled,
    Any,
}

impl Combine {
    fn code(self) -> f64 {
        match self {
            Self::All => 0.0,
            Self::AllSettled => 1.0,
            Self::Any => 2.0,
        }
    }

    fn from_value(value: Option<&Value>) -> Self {
        match value {
            Some(Value::Number(n)) if *n == 1.0 => Self::AllSettled,
            Some(Value::Number(n)) if *n == 2.0 => Self::Any,
            _ => Self::All,
        }
    }
}

/// Conclut un combinateur dont tous les éléments sont réglés.
fn combine_finish(vm: &mut Vm, result: ObjId, values: Value, mode: Combine) {
    if mode == Combine::Any {
        let error = vm.error("AggregateError", "toutes les promesses ont été rejetées");
        if let Some(id) = error.as_object() {
            vm.define_hidden(id, "name", Value::string("AggregateError"));
            vm.define_hidden(id, "errors", values);
        }
        vm.settle(result, true, error);
    } else {
        vm.settle(result, false, values);
    }
}

/// Réaction d'un élément de combinateur.
/// `data` : promesse résultat, tableau des valeurs, compteur restant, index,
/// issue de l'élément (`true` : rejeté), mode.
fn combine_element(vm: &mut Vm, data: &[Value], _this: &Value, args: &[Value]) -> JsResult {
    let (Some(result), Some(values), Some(remaining)) = (
        data_object(data, 0),
        data.get(1).cloned(),
        data_object(data, 2),
    ) else {
        return Ok(Value::Undefined);
    };
    let index = data.get(3).cloned().unwrap_or(Value::Undefined);
    let rejected = data.get(4) == Some(&Value::Bool(true));
    let mode = Combine::from_value(data.get(5));
    let value = arg(args, 0);
    let stored = match (mode, rejected) {
        (Combine::All, true) => {
            vm.settle(result, true, value);
            return Ok(Value::Undefined);
        }
        (Combine::Any, false) => {
            vm.resolve_promise(result, value);
            return Ok(Value::Undefined);
        }
        (Combine::All, false) | (Combine::Any, true) => value,
        (Combine::AllSettled, _) => {
            let entry = Value::Object(vm.new_object());
            let (status, key) = if rejected {
                ("rejected", "reason")
            } else {
                ("fulfilled", "value")
            };
            vm.set(&entry, "status", Value::string(status))?;
            vm.set(&entry, key, value)?;
            entry
        }
    };
    vm.set_element(&values, &index, stored)?;
    let Kind::Primitive(Value::Number(count)) = &mut vm.heap.get_mut(remaining).kind else {
        return Ok(Value::Undefined);
    };
    *count -= 1.0;
    if *count <= 0.0 {
        combine_finish(vm, result, values, mode);
    }
    Ok(Value::Undefined)
}

fn combine(vm: &mut Vm, args: &[Value], mode: Combine) -> JsResult {
    let result = vm.new_promise();
    let items = match vm.iterate_to_vec(&arg(args, 0)) {
        Ok(items) => items,
        Err(thrown) => {
            vm.settle(result, true, thrown);
            return Ok(Value::Object(result));
        }
    };
    let values = vm.new_array(vec![Value::Undefined; items.len()]);
    if items.is_empty() {
        combine_finish(vm, result, values, mode);
        return Ok(Value::Object(result));
    }
    let count = f64::from(u32::try_from(items.len()).unwrap_or(u32::MAX));
    let remaining = vm
        .heap
        .alloc(Object::new(None, Kind::Primitive(Value::Number(count))));
    for (index, item) in items.iter().enumerate() {
        let promise = vm.promise_resolve(item);
        let index = f64::from(u32::try_from(index).unwrap_or(u32::MAX));
        let reaction = |vm: &mut Vm, rejected: bool| {
            vm.new_native_data(
                combine_element,
                vec![
                    Value::Object(result),
                    values.clone(),
                    Value::Object(remaining),
                    Value::Number(index),
                    Value::Bool(rejected),
                    Value::Number(mode.code()),
                ],
            )
        };
        let on_fulfilled = reaction(vm, false);
        let on_rejected = reaction(vm, true);
        vm.promise_then(promise, on_fulfilled, on_rejected, None);
    }
    Ok(Value::Object(result))
}

fn promise_all(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    combine(vm, args, Combine::All)
}

fn promise_all_settled(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    combine(vm, args, Combine::AllSettled)
}

fn promise_any(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    combine(vm, args, Combine::Any)
}

/// Règle la promesse `data[0]` avec l'argument ; `data[1]` : rejet.
fn settle_with(vm: &mut Vm, data: &[Value], _this: &Value, args: &[Value]) -> JsResult {
    if let Some(promise) = data_object(data, 0) {
        if data.get(1) == Some(&Value::Bool(true)) {
            vm.settle(promise, true, arg(args, 0));
        } else {
            vm.resolve_promise(promise, arg(args, 0));
        }
    }
    Ok(Value::Undefined)
}

fn promise_race(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let result = vm.new_promise();
    let items = match vm.iterate_to_vec(&arg(args, 0)) {
        Ok(items) => items,
        Err(thrown) => {
            vm.settle(result, true, thrown);
            return Ok(Value::Object(result));
        }
    };
    for item in &items {
        let promise = vm.promise_resolve(item);
        let on_fulfilled =
            vm.new_native_data(settle_with, vec![Value::Object(result), Value::Bool(false)]);
        let on_rejected =
            vm.new_native_data(settle_with, vec![Value::Object(result), Value::Bool(true)]);
        vm.promise_then(promise, on_fulfilled, on_rejected, None);
    }
    Ok(Value::Object(result))
}

pub(super) fn install(vm: &mut Vm) {
    let proto = vm.intrinsics.promise_proto;
    let ctor = constructor(vm, "Promise", 1, promise_ctor, proto);
    let statics: &[(&str, u32, NativeFn)] = &[
        ("resolve", 1, promise_static_resolve),
        ("reject", 1, promise_static_reject),
        ("all", 1, promise_all),
        ("allSettled", 1, promise_all_settled),
        ("any", 1, promise_any),
        ("race", 1, promise_race),
    ];
    for (name, arity, func) in statics {
        vm.define_method(ctor, name, *arity, *func);
    }
    let methods: &[(&str, u32, NativeFn)] = &[
        ("then", 2, promise_then),
        ("catch", 1, promise_catch),
        ("finally", 1, promise_finally),
    ];
    for (name, arity, func) in methods {
        vm.define_method(proto, name, *arity, *func);
    }
    let tag = vm.well_known.to_string_tag.clone();
    vm.define_hidden_symbol(proto, &tag, Value::string("Promise"));
}
