//! `Promise`, file de microtâches et pilotage des fonctions async.
//!
//! Les réactions (`then`) ne s'exécutent jamais pendant l'appel qui les
//! déclenche : elles sont mises en file et exécutées par
//! [`Vm::run_microtasks`], que l'hôte appelle à la fin de chaque tâche.
//!
//! LIMITE CONNUE : `await` attache ses réactions directement à une promesse
//! ordinaire, sans consulter sa propriété `constructor` ni sa méthode `then`.

use std::sync::Arc;

use super::{arg, constructor, is_callable};
use crate::heap::{Derived, Kind, Object, PromiseData, PromiseState, Property, Reaction, Slot};
use crate::internal::{key_value, same_value};
use crate::value::{JsStr, ObjId, PropertyKey, Value};
use crate::vm::{CoOutcome, JsResult, NativeDataFn, NativeFn, Resume, Vm};

/// Microtâche en attente.
#[derive(Clone, Debug)]
pub(crate) enum Job {
    /// Réaction d'une promesse réglée : appelle `handler(argument)` puis
    /// résout `target` avec le résultat.
    Reaction {
        handler: Value,
        argument: Value,
        rejected: bool,
        target: Derived,
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
                target.trace(out);
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
        let derived = target.map_or(Derived::None, Derived::Promise);
        self.promise_then_derived(promise, on_fulfilled, on_rejected, derived);
    }

    /// `PerformPromiseThen` : comme [`Vm::promise_then`], le résultat du
    /// gestionnaire allant à `target`.
    pub(crate) fn promise_then_derived(
        &mut self,
        promise: ObjId,
        on_fulfilled: Value,
        on_rejected: Value,
        target: Derived,
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
            anonymous(self, resolver_resolve, data.clone(), 1.0),
            anonymous(self, resolver_reject, data, 1.0),
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
        self.settle_async_waits();
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
                match target {
                    Derived::None => {}
                    Derived::Promise(target) => match outcome {
                        Ok(value) => self.resolve_promise(target, value),
                        Err(thrown) => self.settle(target, true, thrown),
                    },
                    Derived::Functions(functions) => {
                        let (resolve, reject) = *functions;
                        let _ = match outcome {
                            Ok(value) => self.call(&resolve, &Value::Undefined, &[value]),
                            Err(thrown) => self.call(&reject, &Value::Undefined, &[thrown]),
                        };
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

// ---- Capacités -------------------------------------------------------------------

/// Capacité de promesse (ECMA-262 §27.2.1.1) : une promesse et les fonctions
/// qui la règlent.
pub(crate) struct Capability {
    pub(crate) promise: Value,
    pub(crate) resolve: Value,
    pub(crate) reject: Value,
}

/// Fonction native anonyme (`name` vide) de longueur `length`.
fn anonymous(vm: &mut Vm, func: NativeDataFn, data: Vec<Value>, length: f64) -> Value {
    let function = vm.new_native_data(func, data);
    if let Some(id) = function.as_object() {
        for (key, value) in [
            ("length", Value::Number(length)),
            ("name", Value::string("")),
        ] {
            vm.heap.get_mut(id).props.insert(
                Arc::from(key),
                Property {
                    slot: Slot::Data(value),
                    writable: false,
                    enumerable: false,
                    configurable: true,
                },
            );
        }
    }
    function
}

/// Liste interne, portée par un tableau que le script ne voit jamais.
fn list_mut<'a>(vm: &'a mut Vm, list: &Value) -> Option<&'a mut Vec<Value>> {
    match &mut vm.heap.get_mut(list.as_object()?).kind {
        Kind::Array(items) => Some(items),
        _ => None,
    }
}

fn list_get(vm: &mut Vm, list: &Value, index: usize) -> Value {
    list_mut(vm, list)
        .and_then(|items| items.get(index).cloned())
        .unwrap_or(Value::Undefined)
}

/// Exécuteur passé au constructeur par `NewPromiseCapability` : range les
/// fonctions `resolve` et `reject` reçues dans la liste `data[0]`.
fn capability_executor(vm: &mut Vm, data: &[Value], _this: &Value, args: &[Value]) -> JsResult {
    let holder = data.first().cloned().unwrap_or(Value::Undefined);
    if list_get(vm, &holder, 0) != Value::Undefined || list_get(vm, &holder, 1) != Value::Undefined
    {
        return Err(vm.type_error("l'exécuteur d'une capacité de promesse a déjà été appelé"));
    }
    if let Some(items) = list_mut(vm, &holder) {
        *items = vec![arg(args, 0), arg(args, 1)];
    }
    Ok(Value::Undefined)
}

impl Vm {
    /// Le constructeur `Promise` d'origine.
    fn promise_ctor_value(&self) -> Value {
        self.promise_ctor.map_or(Value::Undefined, Value::Object)
    }

    /// `NewPromiseCapability(C)` (ECMA-262 §27.2.1.5).
    pub(crate) fn new_capability(&mut self, ctor: &Value) -> JsResult<Capability> {
        if self.promise_ctor.is_some() && ctor.as_object() == self.promise_ctor {
            let promise = self.new_promise();
            let (resolve, reject) = self.resolvers(promise);
            return Ok(Capability {
                promise: Value::Object(promise),
                resolve,
                reject,
            });
        }
        if !self.is_constructor(ctor) {
            return Err(self.type_error("un constructeur de promesse est attendu"));
        }
        let holder = self.new_array(vec![Value::Undefined, Value::Undefined]);
        let executor = anonymous(self, capability_executor, vec![holder.clone()], 2.0);
        let promise = self.construct(ctor, &[executor], ctor)?;
        let resolve = list_get(self, &holder, 0);
        let reject = list_get(self, &holder, 1);
        if !is_callable(self, &resolve) || !is_callable(self, &reject) {
            return Err(self.type_error(
                "le constructeur de promesse n'a pas fourni ses fonctions de résolution",
            ));
        }
        Ok(Capability {
            promise,
            resolve,
            reject,
        })
    }
}

/// `Invoke(target, name, args)`.
fn invoke(vm: &mut Vm, target: &Value, name: &str, args: &[Value]) -> JsResult {
    let method = vm.get(target, name)?;
    vm.call(&method, target, args)
}

/// `SpeciesConstructor(promise, %Promise%)`.
fn promise_species(vm: &mut Vm, promise: &Value) -> JsResult {
    let ctor = vm.get(promise, "constructor")?;
    if ctor == Value::Undefined {
        return Ok(vm.promise_ctor_value());
    }
    if ctor.as_object().is_none() {
        return Err(vm.type_error("la propriété « constructor » n'est pas un objet"));
    }
    let key = PropertyKey::Symbol(vm.well_known.species.clone());
    let species = vm.get_key(&ctor, &key)?;
    if species.is_nullish() {
        return Ok(vm.promise_ctor_value());
    }
    if vm.is_constructor(&species) {
        Ok(species)
    } else {
        Err(vm.type_error("l'espèce de la promesse n'est pas un constructeur"))
    }
}

/// `PromiseResolve(C, value)` (ECMA-262 §27.2.4.7.1).
fn promise_resolve_with(vm: &mut Vm, ctor: &Value, value: &Value) -> JsResult {
    let is_promise = value
        .as_object()
        .is_some_and(|id| matches!(vm.heap.get(id).kind, Kind::Promise(_)));
    if is_promise {
        let own = vm.get(value, "constructor")?;
        if same_value(&own, ctor) {
            return Ok(value.clone());
        }
    }
    let capability = vm.new_capability(ctor)?;
    vm.call(
        &capability.resolve,
        &Value::Undefined,
        std::slice::from_ref(value),
    )?;
    Ok(capability.promise)
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

/// `Promise.prototype.then` (ECMA-262 §27.2.5.4).
fn promise_then(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let promise = this_promise(vm, this)?;
    let ctor = promise_species(vm, this)?;
    if ctor.as_object() == vm.promise_ctor {
        let target = vm.new_promise();
        vm.promise_then(promise, arg(args, 0), arg(args, 1), Some(target));
        return Ok(Value::Object(target));
    }
    let capability = vm.new_capability(&ctor)?;
    let derived = Derived::Functions(Box::new((capability.resolve, capability.reject)));
    vm.promise_then_derived(promise, arg(args, 0), arg(args, 1), derived);
    Ok(capability.promise)
}

fn promise_catch(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    invoke(vm, this, "then", &[Value::Undefined, arg(args, 0)])
}

/// `Promise.prototype.finally` (ECMA-262 §27.2.5.3).
fn promise_finally(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    if this.as_object().is_none() {
        return Err(vm.type_error("le receveur de « finally » n'est pas un objet"));
    }
    let ctor = promise_species(vm, this)?;
    let callback = arg(args, 0);
    let (then_finally, catch_finally) = if is_callable(vm, &callback) {
        let data = vec![callback, ctor];
        (
            anonymous(vm, finally_fulfilled, data.clone(), 1.0),
            anonymous(vm, finally_rejected, data, 1.0),
        )
    } else {
        (callback.clone(), callback)
    };
    invoke(vm, this, "then", &[then_finally, catch_finally])
}

/// Appelle le rappel de `finally`, attend son résultat, puis laisse passer
/// l'issue d'origine au moyen de `pass`. `data` : rappel, constructeur.
fn run_finally(vm: &mut Vm, data: &[Value], pass: NativeDataFn, outcome: Value) -> JsResult {
    let callback = data.first().cloned().unwrap_or(Value::Undefined);
    let ctor = data.get(1).cloned().unwrap_or(Value::Undefined);
    let result = vm.call(&callback, &Value::Undefined, &[])?;
    let promise = promise_resolve_with(vm, &ctor, &result)?;
    let pass = anonymous(vm, pass, vec![outcome], 0.0);
    invoke(vm, &promise, "then", &[pass])
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

fn promise_static_resolve(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    if this.as_object().is_none() {
        return Err(vm.type_error("Promise.resolve : le receveur n'est pas un objet"));
    }
    promise_resolve_with(vm, this, &arg(args, 0))
}

fn promise_static_reject(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let capability = vm.new_capability(this)?;
    vm.call(&capability.reject, &Value::Undefined, &[arg(args, 0)])?;
    Ok(capability.promise)
}

/// `Promise.withResolvers()` : `{ promise, resolve, reject }`.
fn promise_with_resolvers(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let capability = vm.new_capability(this)?;
    let object = vm.new_object();
    for (name, value) in [
        ("promise", capability.promise),
        ("resolve", capability.resolve),
        ("reject", capability.reject),
    ] {
        vm.create_data_property(object, &PropertyKey::String(Arc::from(name)), value)?;
    }
    Ok(Value::Object(object))
}

/// `Promise.try(callback, ...args)` : appelle `callback` et rend une
/// promesse de son résultat, rejetée s'il lève une exception.
fn promise_try(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    if this.as_object().is_none() {
        return Err(vm.type_error("Promise.try : le receveur n'est pas un objet"));
    }
    let capability = vm.new_capability(this)?;
    let rest = args.get(1..).unwrap_or(&[]);
    match vm.call(&arg(args, 0), &Value::Undefined, rest) {
        Ok(value) => {
            // Une promesse du même constructeur est rendue telle quelle.
            let is_promise = value
                .as_object()
                .is_some_and(|id| matches!(vm.heap.get(id).kind, Kind::Promise(_)));
            if is_promise {
                let own = vm.get(&value, "constructor")?;
                if same_value(&own, this) {
                    return Ok(value);
                }
            }
            vm.call(&capability.resolve, &Value::Undefined, &[value])?
        }
        Err(thrown) => vm.call(&capability.reject, &Value::Undefined, &[thrown])?,
    };
    Ok(capability.promise)
}

// ---- Combinateurs ------------------------------------------------------------------

/// Combinateur : quelle issue de chaque élément compte, et comment conclure.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Combine {
    All,
    AllSettled,
    Any,
    Race,
}

/// Ce qu'une fonction d'élément range dans la liste des résultats.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Element {
    /// La valeur reçue (`all`).
    Value,
    /// `{ status: "fulfilled", value }` (`allSettled`).
    Fulfilled,
    /// `{ status: "rejected", reason }` (`allSettled`).
    Rejected,
    /// L'erreur reçue ; la conclusion est une `AggregateError` (`any`).
    Error,
}

impl Element {
    fn code(self) -> f64 {
        match self {
            Self::Value => 0.0,
            Self::Fulfilled => 1.0,
            Self::Rejected => 2.0,
            Self::Error => 3.0,
        }
    }

    fn from_value(value: Option<&Value>) -> Self {
        match value {
            Some(Value::Number(n)) if *n == 1.0 => Self::Fulfilled,
            Some(Value::Number(n)) if *n == 2.0 => Self::Rejected,
            Some(Value::Number(n)) if *n == 3.0 => Self::Error,
            _ => Self::Value,
        }
    }
}

/// Cellule interne portant un compteur ou un drapeau.
fn cell(vm: &mut Vm, value: Value) -> Value {
    Value::Object(vm.heap.alloc(Object::new(None, Kind::Primitive(value))))
}

/// Ajoute `delta` au compteur `cell` et rend sa nouvelle valeur.
fn count(vm: &mut Vm, cell: &Value, delta: f64) -> f64 {
    let Some(id) = cell.as_object() else {
        return 0.0;
    };
    match &mut vm.heap.get_mut(id).kind {
        Kind::Primitive(Value::Number(n)) => {
            *n += delta;
            *n
        }
        _ => 0.0,
    }
}

/// Lève le drapeau `cell` ; vrai s'il l'était déjà.
fn already_called(vm: &mut Vm, cell: &Value) -> bool {
    let Some(id) = cell.as_object() else {
        return true;
    };
    match &mut vm.heap.get_mut(id).kind {
        Kind::Primitive(state) => std::mem::replace(state, Value::Bool(true)) == Value::Bool(true),
        _ => true,
    }
}

/// Valeur qui conclut un combinateur : tableau des résultats, objet
/// `clé → résultat` (variantes « Keyed »), ou `AggregateError` (`any`).
fn conclusion(vm: &mut Vm, element: Element, values: &Value, keys: &Value) -> JsResult {
    let items = list_mut(vm, values).cloned().unwrap_or_default();
    if element == Element::Error {
        let error = vm.error("AggregateError", "toutes les promesses ont été rejetées");
        let errors = vm.new_array(items);
        if let Some(id) = error.as_object() {
            vm.define_hidden(id, "errors", errors);
        }
        return Ok(error);
    }
    let Some(keys) = list_mut(vm, keys).cloned() else {
        return Ok(vm.new_array(items));
    };
    let object = vm.heap.alloc(Object::new(None, Kind::Ordinary));
    for (key, value) in keys.into_iter().zip(items) {
        let key = match key {
            Value::Symbol(symbol) => PropertyKey::Symbol(symbol),
            other => PropertyKey::String(vm.to_string(&other)?),
        };
        vm.create_data_property(object, &key, value)?;
    }
    Ok(Value::Object(object))
}

/// Fonction d'élément d'un combinateur (`Promise.all Resolve Element
/// Functions` et ses semblables).
/// `data` : drapeau « déjà appelée », liste des résultats, compteur restant,
/// index, fonction qui conclut, nature de l'élément, liste des clés.
fn combine_element(vm: &mut Vm, data: &[Value], _this: &Value, args: &[Value]) -> JsResult {
    let [flag, values, remaining, index, settle, element, keys] = data else {
        return Ok(Value::Undefined);
    };
    if already_called(vm, flag) {
        return Ok(Value::Undefined);
    }
    let element = Element::from_value(Some(element));
    let value = arg(args, 0);
    let stored = match element {
        Element::Value | Element::Error => value,
        Element::Fulfilled | Element::Rejected => {
            let entry = vm.new_object();
            let (status, key) = if element == Element::Rejected {
                ("rejected", "reason")
            } else {
                ("fulfilled", "value")
            };
            vm.create_data_property(
                entry,
                &PropertyKey::String(Arc::from("status")),
                Value::string(status),
            )?;
            vm.create_data_property(entry, &PropertyKey::String(Arc::from(key)), value)?;
            Value::Object(entry)
        }
    };
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "index rangé par le combinateur : entier positif"
    )]
    let position = match index {
        Value::Number(n) => *n as usize,
        _ => 0,
    };
    if let Some(slot) = list_mut(vm, values).and_then(|items| items.get_mut(position)) {
        *slot = stored;
    }
    if count(vm, remaining, -1.0) == 0.0 {
        let result = conclusion(vm, element, values, keys)?;
        return vm.call(settle, &Value::Undefined, &[result]);
    }
    Ok(Value::Undefined)
}

/// État partagé par les éléments d'un combinateur.
struct Combinator<'a> {
    ctor: &'a Value,
    capability: &'a Capability,
    /// `C.resolve`, lue une fois.
    resolve: Value,
    mode: Combine,
    values: Value,
    keys: Value,
    remaining: Value,
}

impl Combinator<'_> {
    /// Abonne le combinateur à l'élément `next`, de rang `index`.
    fn subscribe(&self, vm: &mut Vm, next: Value, index: usize) -> JsResult<()> {
        let capability = self.capability;
        if self.mode != Combine::Race
            && let Some(items) = list_mut(vm, &self.values)
        {
            items.push(Value::Undefined);
        }
        let promise = vm.call(&self.resolve, self.ctor, &[next])?;
        let flag = cell(vm, Value::Bool(false));
        #[expect(clippy::cast_precision_loss, reason = "rang petit devant 2^53")]
        let index = Value::Number(index as f64);
        let element = |vm: &mut Vm, element: Element, settle: &Value| {
            let data = vec![
                flag.clone(),
                self.values.clone(),
                self.remaining.clone(),
                index.clone(),
                settle.clone(),
                Value::Number(element.code()),
                self.keys.clone(),
            ];
            anonymous(vm, combine_element, data, 1.0)
        };
        let (on_fulfilled, on_rejected) = match self.mode {
            Combine::Race => (capability.resolve.clone(), capability.reject.clone()),
            Combine::All => (
                element(vm, Element::Value, &capability.resolve),
                capability.reject.clone(),
            ),
            Combine::AllSettled => (
                element(vm, Element::Fulfilled, &capability.resolve),
                element(vm, Element::Rejected, &capability.resolve),
            ),
            Combine::Any => (
                capability.resolve.clone(),
                element(vm, Element::Error, &capability.reject),
            ),
        };
        if self.mode != Combine::Race {
            count(vm, &self.remaining, 1.0);
        }
        invoke(vm, &promise, "then", &[on_fulfilled, on_rejected])?;
        Ok(())
    }

    /// Tous les éléments sont abonnés : conclut si plus aucun n'est en attente.
    fn finish(&self, vm: &mut Vm) -> JsResult<()> {
        if self.mode == Combine::Race || count(vm, &self.remaining, -1.0) != 0.0 {
            return Ok(());
        }
        if self.mode == Combine::Any {
            return Err(conclusion(vm, Element::Error, &self.values, &self.keys)?);
        }
        let result = conclusion(vm, Element::Value, &self.values, &self.keys)?;
        vm.call(&self.capability.resolve, &Value::Undefined, &[result])?;
        Ok(())
    }
}

/// Corps d'un combinateur ; une erreur rejette la promesse résultat.
fn perform(
    vm: &mut Vm,
    ctor: &Value,
    source: &Value,
    capability: &Capability,
    mode: Combine,
    keyed: bool,
) -> JsResult<()> {
    // `GetPromiseResolve(C)`.
    let resolve = vm.get(ctor, "resolve")?;
    if !is_callable(vm, &resolve) {
        return Err(vm.type_error("la méthode « resolve » du constructeur n'est pas une fonction"));
    }
    let values = vm.new_array(Vec::new());
    let keys = if keyed {
        vm.new_array(Vec::new())
    } else {
        Value::Undefined
    };
    let combinator = Combinator {
        ctor,
        capability,
        resolve,
        mode,
        values,
        keys,
        remaining: cell(vm, Value::Number(1.0)),
    };
    if keyed {
        let Some(id) = source.as_object() else {
            return Err(vm.type_error("un objet de promesses est attendu"));
        };
        let mut index = 0;
        for key in vm.own_property_keys(id)? {
            let enumerable = vm
                .get_own_property(id, &key)?
                .is_some_and(|descriptor| descriptor.enumerable == Some(true));
            if !enumerable {
                continue;
            }
            let value = vm.get_key(source, &key)?;
            if let Some(items) = list_mut(vm, &combinator.keys) {
                items.push(key_value(&key));
            }
            combinator.subscribe(vm, value, index)?;
            index += 1;
        }
        return combinator.finish(vm);
    }
    let iterator = vm.iterator_for(source)?;
    let mut index = 0;
    let outcome = loop {
        if let Err(error) = vm.tick(1) {
            break Err(error);
        }
        match vm.iterator_next(&iterator) {
            Ok(Some(next)) => {
                if let Err(error) = combinator.subscribe(vm, next, index) {
                    break Err(error);
                }
                index += 1;
            }
            Ok(None) => return combinator.finish(vm),
            // L'itérateur a échoué : il n'est pas fermé.
            Err(error) => return Err(error),
        }
    };
    vm.iterator_close(&iterator, true)?;
    outcome
}

fn combinator(vm: &mut Vm, this: &Value, args: &[Value], mode: Combine, keyed: bool) -> JsResult {
    let capability = vm.new_capability(this)?;
    if let Err(thrown) = perform(vm, this, &arg(args, 0), &capability, mode, keyed) {
        vm.call(&capability.reject, &Value::Undefined, &[thrown])?;
    }
    Ok(capability.promise)
}

fn promise_all(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    combinator(vm, this, args, Combine::All, false)
}

fn promise_all_settled(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    combinator(vm, this, args, Combine::AllSettled, false)
}

fn promise_any(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    combinator(vm, this, args, Combine::Any, false)
}

fn promise_race(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    combinator(vm, this, args, Combine::Race, false)
}

fn promise_all_keyed(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    combinator(vm, this, args, Combine::All, true)
}

fn promise_all_settled_keyed(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    combinator(vm, this, args, Combine::AllSettled, true)
}

pub(super) fn install(vm: &mut Vm) {
    let proto = vm.intrinsics.promise_proto;
    let ctor = constructor(vm, "Promise", 1, promise_ctor, proto);
    vm.promise_ctor = Some(ctor);
    let statics: &[(&str, u32, NativeFn)] = &[
        ("resolve", 1, promise_static_resolve),
        ("reject", 1, promise_static_reject),
        ("all", 1, promise_all),
        ("allSettled", 1, promise_all_settled),
        ("any", 1, promise_any),
        ("race", 1, promise_race),
        ("withResolvers", 0, promise_with_resolvers),
        ("try", 1, promise_try),
        ("allKeyed", 1, promise_all_keyed),
        ("allSettledKeyed", 1, promise_all_settled_keyed),
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
