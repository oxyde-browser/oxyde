//! Machine virtuelle : interpréteur du bytecode à registres.
//!
//! Les appels de fonction JavaScript à fonction JavaScript n'utilisent pas la
//! pile native : chaque appel empile un [`Frame`]. Seules les fonctions
//! natives qui rappellent du JavaScript (`Array.prototype.map`…) relancent
//! une boucle d'interprétation imbriquée.
//!
//! Le ramasse-miettes ne s'exécute qu'entre deux instructions de la boucle la
//! plus externe : à cet instant, toute valeur vivante est atteignable depuis
//! les registres, les cadres ou les racines déclarées. Une fonction native
//! peut donc garder des identifiants d'objets dans des variables Rust.

use std::any::Any;
use std::collections::VecDeque;
use std::sync::Arc;

use indexmap::IndexMap;

use crate::builtins::promise::Job;
use crate::bytecode::{FunctionKind, FunctionProto, Op, Reg};
use crate::heap::{
    Closure, CoState, Coroutine, Heap, IterState, Kind, Object, Property, SavedFrame, Slot, UpId,
    Upvalue,
};
use crate::value::{JsStr, ObjId, Symbol, Value};

/// Résultat d'une opération qui peut lever une exception JavaScript.
pub type JsResult<T = Value> = Result<T, Value>;

/// Fonction native : reçoit la machine, `this` et les arguments.
pub type NativeFn = fn(&mut Vm, &Value, &[Value]) -> JsResult;

/// Fonction native à données capturées : reçoit la machine, ses données, `this`
/// et les arguments.
pub type NativeDataFn = fn(&mut Vm, &[Value], &Value, &[Value]) -> JsResult;

/// Façon de reprendre une coroutine suspendue.
pub(crate) enum Resume {
    /// `next(v)` : `v` devient la valeur de l'expression `yield` / `await`.
    Next(Value),
    /// `throw(e)` : `e` est levée au point de suspension.
    Throw(Value),
    /// `return(v)` : la coroutine se termine avec `v`.
    Return(Value),
}

/// Issue de la reprise d'une coroutine.
pub(crate) enum CoOutcome {
    /// Suspendue de nouveau, en rendant cette valeur.
    Yield(Value),
    /// Terminée avec cette valeur.
    Return(Value),
}

/// Profondeur maximale de la pile d'appels JavaScript.
const MAX_FRAMES: usize = 2_000;
/// Profondeur maximale de réentrance native → JavaScript.
const MAX_NATIVE_DEPTH: usize = 120;
/// Taille maximale de la pile de registres.
const MAX_STACK: usize = 4_000_000;

/// Prototypes et constructeurs intégrés.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Intrinsics {
    pub(crate) object_proto: ObjId,
    pub(crate) function_proto: ObjId,
    pub(crate) array_proto: ObjId,
    pub(crate) string_proto: ObjId,
    pub(crate) number_proto: ObjId,
    pub(crate) boolean_proto: ObjId,
    pub(crate) error_proto: ObjId,
    pub(crate) map_proto: ObjId,
    pub(crate) set_proto: ObjId,
    pub(crate) symbol_proto: ObjId,
    /// `%IteratorPrototype%` : ancêtre de tous les itérateurs intégrés.
    pub(crate) iterator_proto: ObjId,
    pub(crate) generator_proto: ObjId,
    pub(crate) promise_proto: ObjId,
    pub(crate) regexp_proto: ObjId,
}

/// Symboles connus du langage (`Symbol.iterator`…).
#[derive(Clone, Debug)]
pub(crate) struct WellKnown {
    pub(crate) iterator: Symbol,
    pub(crate) async_iterator: Symbol,
    pub(crate) has_instance: Symbol,
    pub(crate) to_primitive: Symbol,
    pub(crate) to_string_tag: Symbol,
}

struct Frame {
    closure: ObjId,
    proto: Arc<FunctionProto>,
    pc: usize,
    /// Index absolu du registre 0 dans la pile.
    base: usize,
    this: Value,
    new_target: Value,
    /// Arguments reçus (pour `arguments` et les paramètres de reste).
    args: Vec<Value>,
    /// Gestionnaires d'exception actifs : (cible, registre de l'exception).
    handlers: Vec<(u32, Reg)>,
    /// Index absolu du registre qui reçoit le résultat, ou `None` si l'appel
    /// vient d'une fonction native (le résultat est alors rendu par `run`).
    ret: Option<usize>,
    /// Appel par `new` : le résultat est `this` si la fonction ne rend pas un objet.
    construct: bool,
    /// Coroutine dont ce cadre exécute le corps.
    generator: Option<ObjId>,
}

/// Classes d'objets de l'hôte et crochets d'accès à leurs propriétés.
#[derive(Clone, Copy)]
pub struct HostHooks {
    /// Lecture d'une propriété d'un objet hôte ; `None` : propriété ordinaire.
    pub get: fn(&mut Vm, u32, u64, &str) -> Option<JsResult>,
    /// Écriture d'une propriété d'un objet hôte ; `None` : propriété ordinaire.
    pub set: fn(&mut Vm, u32, u64, &str, &Value) -> Option<JsResult<()>>,
}

/// Machine virtuelle JavaScript : un tas, un objet global, une pile.
pub struct Vm {
    pub(crate) heap: Heap,
    stack: Vec<Value>,
    frames: Vec<Frame>,
    /// Cellules de variables capturées encore dans la pile : (index absolu, cellule).
    open_upvalues: Vec<(usize, UpId)>,
    pub(crate) global: ObjId,
    pub(crate) intrinsics: Intrinsics,
    pub(crate) error_protos: IndexMap<&'static str, ObjId>,
    pub(crate) well_known: WellKnown,
    /// Identifiant du prochain symbole créé.
    next_symbol: u64,
    /// Registre de `Symbol.for`.
    pub(crate) symbol_registry: IndexMap<JsStr, Symbol>,
    native_depth: usize,
    /// Expressions rationnelles compilées, par (motif, drapeaux).
    pub(crate) regex_cache: std::collections::HashMap<(JsStr, JsStr), Arc<crate::regexp::Regex>>,
    /// Dernier texte préparé pour une recherche.
    pub(crate) regex_input: Option<(JsStr, Arc<crate::builtins::regexp::RegexInput>)>,
    /// Exception à lever à la prochaine instruction (reprise par `throw()`).
    pending_throw: Option<Value>,
    /// File des microtâches (réactions de promesses).
    pub(crate) jobs: VecDeque<Job>,
    /// Promesses rejetées sans gestionnaire, à signaler en fin de microtâches.
    pub(crate) rejections: Vec<ObjId>,
    /// Vrai pendant l'appel d'un constructeur natif par `new`.
    pub(crate) constructing: bool,
    /// Racines déclarées par l'hôte (écouteurs, minuteurs…).
    roots: Vec<Value>,
    /// Données de l'hôte accessibles aux fonctions natives.
    pub host: Option<Box<dyn Any + Send>>,
    pub(crate) host_hooks: Option<HostHooks>,
    /// Nombre d'instructions encore autorisées ; `None` : illimité.
    budget: Option<u64>,
    /// Vrai une fois le budget épuisé : l'exception ne peut pas être rattrapée.
    interrupted: bool,
    /// État du générateur pseudo-aléatoire de `Math.random`.
    pub(crate) rng: u64,
    /// Messages de `console.*` en attente de lecture par l'hôte.
    pub(crate) console: Vec<crate::builtins::ConsoleMessage>,
}

/// Graine du générateur pseudo-aléatoire : tirée de l'horloge, jamais nulle
/// (xorshift ne quitte pas l'état zéro).
fn seed() -> u64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    #[expect(
        clippy::cast_possible_truncation,
        reason = "seuls les bits de poids faible servent de graine"
    )]
    let mixed = (nanos as u64) ^ 0x9E37_79B9_7F4A_7C15;
    mixed | 1
}

impl std::fmt::Debug for Vm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Vm")
            .field("objects", &self.heap.live_objects())
            .field("frames", &self.frames.len())
            .finish_non_exhaustive()
    }
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}

impl Vm {
    /// Crée une machine avec la bibliothèque standard installée.
    #[must_use]
    pub fn new() -> Self {
        let mut heap = Heap::new();
        let object_proto = heap.alloc(Object::new(None, Kind::Ordinary));
        let proto = |heap: &mut Heap, kind: Kind| heap.alloc(Object::new(Some(object_proto), kind));
        let function_proto = proto(&mut heap, Kind::Ordinary);
        let intrinsics = Intrinsics {
            object_proto,
            function_proto,
            array_proto: proto(&mut heap, Kind::Array(Vec::new())),
            string_proto: proto(&mut heap, Kind::Primitive(Value::string(""))),
            number_proto: proto(&mut heap, Kind::Primitive(Value::Number(0.0))),
            boolean_proto: proto(&mut heap, Kind::Primitive(Value::Bool(false))),
            error_proto: proto(&mut heap, Kind::Ordinary),
            map_proto: proto(&mut heap, Kind::Ordinary),
            set_proto: proto(&mut heap, Kind::Ordinary),
            symbol_proto: proto(&mut heap, Kind::Ordinary),
            iterator_proto: proto(&mut heap, Kind::Ordinary),
            generator_proto: proto(&mut heap, Kind::Ordinary),
            promise_proto: proto(&mut heap, Kind::Ordinary),
            regexp_proto: proto(&mut heap, Kind::Ordinary),
        };
        let global = proto(&mut heap, Kind::Ordinary);
        let known = |id: u64, name: &str| Symbol::new(id, Some(Arc::from(name)));
        let well_known = WellKnown {
            iterator: known(1, "Symbol.iterator"),
            async_iterator: known(2, "Symbol.asyncIterator"),
            has_instance: known(3, "Symbol.hasInstance"),
            to_primitive: known(4, "Symbol.toPrimitive"),
            to_string_tag: known(5, "Symbol.toStringTag"),
        };
        let mut vm = Self {
            heap,
            stack: Vec::new(),
            frames: Vec::new(),
            open_upvalues: Vec::new(),
            global,
            intrinsics,
            error_protos: IndexMap::new(),
            well_known,
            next_symbol: 6,
            symbol_registry: IndexMap::new(),
            native_depth: 0,
            regex_cache: std::collections::HashMap::new(),
            regex_input: None,
            pending_throw: None,
            jobs: VecDeque::new(),
            rejections: Vec::new(),
            constructing: false,
            roots: Vec::new(),
            host: None,
            host_hooks: None,
            budget: None,
            interrupted: false,
            rng: seed(),
            console: Vec::new(),
        };
        crate::builtins::install(&mut vm);
        vm
    }

    /// Limite le nombre d'instructions exécutées jusqu'au prochain appel de
    /// cette méthode. Un script qui la dépasse est interrompu par une erreur
    /// qu'il ne peut pas rattraper. `None` : pas de limite.
    pub fn set_budget(&mut self, instructions: Option<u64>) {
        self.budget = instructions;
        self.interrupted = false;
    }

    /// Impute `steps` pas de travail natif (recherche d'expression
    /// rationnelle…) au budget d'instructions du script.
    pub(crate) fn charge(&mut self, steps: u64) {
        if let Some(budget) = &mut self.budget {
            *budget = budget.saturating_sub(steps);
        }
    }

    /// Vrai si le dernier script a été interrompu pour dépassement du budget.
    #[must_use]
    pub fn was_interrupted(&self) -> bool {
        self.interrupted
    }

    /// Déclare `value` comme racine : elle ne sera pas collectée.
    pub fn add_root(&mut self, value: Value) {
        self.roots.push(value);
    }

    /// Retire une racine déclarée par [`Vm::add_root`].
    pub fn remove_root(&mut self, value: &Value) {
        if let Some(index) = self.roots.iter().position(|v| v == value) {
            self.roots.swap_remove(index);
        }
    }

    /// Installe les crochets d'accès aux objets de l'hôte.
    pub fn set_host_hooks(&mut self, hooks: HostHooks) {
        self.host_hooks = Some(hooks);
    }

    /// Nombre d'objets vivants dans le tas.
    #[must_use]
    pub fn live_objects(&self) -> usize {
        self.heap.live_objects()
    }

    /// Nombre de passages du ramasse-miettes depuis la création.
    #[must_use]
    pub fn collections(&self) -> usize {
        self.heap.collections
    }

    /// Force un passage du ramasse-miettes. Sans effet pendant l'exécution
    /// d'une fonction native appelée par un script.
    pub fn collect_garbage(&mut self) {
        if self.native_depth > 0 {
            return;
        }
        let intr = self.intrinsics;
        let fixed = [
            self.global,
            intr.object_proto,
            intr.function_proto,
            intr.array_proto,
            intr.string_proto,
            intr.number_proto,
            intr.boolean_proto,
            intr.error_proto,
            intr.map_proto,
            intr.set_proto,
            intr.symbol_proto,
            intr.iterator_proto,
            intr.generator_proto,
            intr.promise_proto,
            intr.regexp_proto,
        ];
        let frame_values = self.frames.iter().flat_map(|f| {
            [
                Value::Object(f.closure),
                f.this.clone(),
                f.new_target.clone(),
                f.generator.map_or(Value::Undefined, Value::Object),
            ]
            .into_iter()
            .chain(f.args.iter().cloned())
        });
        let mut job_values = Vec::new();
        for job in &self.jobs {
            job.trace(&mut job_values);
        }
        let roots: Vec<Value> = fixed
            .into_iter()
            .map(Value::Object)
            .chain(self.error_protos.values().map(|id| Value::Object(*id)))
            .chain(self.stack.iter().cloned())
            .chain(frame_values)
            .chain(self.roots.iter().cloned())
            .chain(job_values)
            .chain(self.rejections.iter().map(|id| Value::Object(*id)))
            .chain(self.pending_throw.iter().cloned())
            .collect();
        let open: Vec<UpId> = self.open_upvalues.iter().map(|(_, id)| *id).collect();
        self.heap.collect(roots.into_iter(), &open);
    }

    // ---- Création d'objets -------------------------------------------------

    /// Nouvel objet ordinaire.
    pub fn new_object(&mut self) -> ObjId {
        self.heap.alloc(Object::new(
            Some(self.intrinsics.object_proto),
            Kind::Ordinary,
        ))
    }

    /// Nouveau tableau contenant `items`.
    pub fn new_array(&mut self, items: Vec<Value>) -> Value {
        Value::Object(self.heap.alloc(Object::new(
            Some(self.intrinsics.array_proto),
            Kind::Array(items),
        )))
    }

    /// Nouveau symbole, distinct de tous les autres.
    pub fn new_symbol(&mut self, description: Option<&str>) -> Symbol {
        let id = self.next_symbol;
        self.next_symbol = self.next_symbol.saturating_add(1);
        Symbol::new(id, description.map(Arc::from))
    }

    /// Nouvelle fonction native.
    pub fn new_function(&mut self, name: &str, arity: u32, func: NativeFn) -> ObjId {
        self.new_native(name, arity, func, false)
    }

    pub(crate) fn new_native(
        &mut self,
        name: &str,
        arity: u32,
        func: NativeFn,
        constructor: bool,
    ) -> ObjId {
        let mut object = Object::new(
            Some(self.intrinsics.function_proto),
            Kind::Native { func, constructor },
        );
        let hidden = |value: Value| Property {
            slot: Slot::Data(value),
            writable: false,
            enumerable: false,
            configurable: true,
        };
        object
            .props
            .insert(Arc::from("name"), hidden(Value::string(name)));
        object
            .props
            .insert(Arc::from("length"), hidden(Value::Number(f64::from(arity))));
        self.heap.alloc(object)
    }

    /// Nouvelle fonction native qui garde `data` à sa disposition.
    pub fn new_native_data(&mut self, func: NativeDataFn, data: Vec<Value>) -> Value {
        Value::Object(self.heap.alloc(Object::new(
            Some(self.intrinsics.function_proto),
            Kind::NativeData { func, data },
        )))
    }

    /// Nouvel objet de l'hôte, de classe `class`, portant la donnée `data`.
    pub fn new_host_object(&mut self, proto: ObjId, class: u32, data: u64) -> ObjId {
        self.heap
            .alloc(Object::new(Some(proto), Kind::Host { class, data }))
    }

    /// Classe et donnée d'un objet de l'hôte.
    #[must_use]
    pub fn host_data(&self, value: &Value) -> Option<(u32, u64)> {
        match self.heap.get(value.as_object()?).kind {
            Kind::Host { class, data } => Some((class, data)),
            _ => None,
        }
    }

    /// Objet global.
    #[must_use]
    pub fn global(&self) -> ObjId {
        self.global
    }

    /// Nouvelle erreur de la classe `class` (`"TypeError"`…), à lever avec `Err`.
    pub fn error(&mut self, class: &str, message: &str) -> Value {
        let proto = self
            .error_protos
            .get(class)
            .copied()
            .unwrap_or(self.intrinsics.error_proto);
        let mut object = Object::new(Some(proto), Kind::Error);
        object.props.insert(
            Arc::from("message"),
            Property::hidden(Value::string(message)),
        );
        Value::Object(self.heap.alloc(object))
    }

    pub(crate) fn type_error(&mut self, message: &str) -> Value {
        self.error("TypeError", message)
    }

    // ---- Registres ---------------------------------------------------------

    // INVARIANT : à l'entrée d'un cadre, la pile est agrandie à
    // `base + proto.registers` ; le compilateur ne produit que des numéros de
    // registre inférieurs à `proto.registers`. Les accès `get` ci-dessous ne
    // peuvent donc échouer que sur un bytecode corrompu, traité comme `undefined`.

    fn base(&self) -> usize {
        self.frames.last().map_or(0, |f| f.base)
    }

    fn reg(&self, r: Reg) -> Value {
        self.stack
            .get(self.base() + usize::from(r))
            .cloned()
            .unwrap_or(Value::Undefined)
    }

    fn set_reg(&mut self, r: Reg, value: Value) {
        let index = self.base() + usize::from(r);
        if let Some(slot) = self.stack.get_mut(index) {
            *slot = value;
        }
    }

    fn constant(&self, index: u32) -> Value {
        self.frames
            .last()
            .and_then(|f| f.proto.constants.get(index as usize))
            .cloned()
            .unwrap_or(Value::Undefined)
    }

    fn name(&self, index: u32) -> JsStr {
        match self.constant(index) {
            Value::String(s) => s,
            _ => Arc::from(""),
        }
    }

    // ---- Variables capturées -----------------------------------------------

    fn capture(&mut self, index: usize) -> UpId {
        if let Some((_, id)) = self.open_upvalues.iter().find(|(i, _)| *i == index) {
            return *id;
        }
        let id = self.heap.alloc_upvalue(Upvalue::Open(index));
        self.open_upvalues.push((index, id));
        id
    }

    /// Déplace dans le tas les variables capturées situées à partir de `from`.
    fn close_upvalues(&mut self, from: usize) {
        let mut i = 0;
        while i < self.open_upvalues.len() {
            let (index, id) = self.open_upvalues[i];
            if index >= from {
                let value = self.stack.get(index).cloned().unwrap_or(Value::Undefined);
                if let Some(cell) = self.heap.upvalue_mut(id) {
                    *cell = Upvalue::Closed(value);
                }
                self.open_upvalues.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    fn upvalue_id(&self, index: u16) -> Option<UpId> {
        let frame = self.frames.last()?;
        match &self.heap.get(frame.closure).kind {
            Kind::Closure(c) => c.upvalues.get(usize::from(index)).copied(),
            _ => None,
        }
    }

    fn get_upvalue(&self, index: u16) -> Value {
        match self.upvalue_id(index).and_then(|id| self.heap.upvalue(id)) {
            Some(Upvalue::Open(i)) => self.stack.get(*i).cloned().unwrap_or(Value::Undefined),
            Some(Upvalue::Closed(v)) => v.clone(),
            None => Value::Undefined,
        }
    }

    fn set_upvalue(&mut self, index: u16, value: Value) {
        let Some(id) = self.upvalue_id(index) else {
            return;
        };
        match self.heap.upvalue_mut(id) {
            Some(Upvalue::Open(i)) => {
                let i = *i;
                if let Some(slot) = self.stack.get_mut(i) {
                    *slot = value;
                }
            }
            Some(cell @ Upvalue::Closed(_)) => *cell = Upvalue::Closed(value),
            None => {}
        }
    }

    // ---- Appels --------------------------------------------------------------

    /// Empile le cadre d'une fermeture. Le code s'exécutera dans la boucle `run`.
    fn push_frame(
        &mut self,
        closure: ObjId,
        this: Value,
        args: Vec<Value>,
        new_target: Value,
        ret: Option<usize>,
        construct: bool,
    ) -> JsResult<()> {
        let Kind::Closure(c) = &self.heap.get(closure).kind else {
            return Err(self.type_error("la valeur appelée n'est pas une fonction"));
        };
        let proto = Arc::clone(&c.proto);
        let this = c.bound_this.clone().unwrap_or(this);
        let base = self.stack.len();
        if self.frames.len() >= MAX_FRAMES || base + usize::from(proto.registers) > MAX_STACK {
            return Err(self.error("RangeError", "pile d'appels dépassée"));
        }
        self.stack
            .resize(base + usize::from(proto.registers), Value::Undefined);
        for (i, arg) in args.iter().take(usize::from(proto.param_count)).enumerate() {
            if let Some(slot) = self.stack.get_mut(base + i) {
                *slot = arg.clone();
            }
        }
        self.frames.push(Frame {
            closure,
            proto,
            pc: 0,
            base,
            this,
            new_target,
            args,
            handlers: Vec::new(),
            ret,
            construct,
            generator: None,
        });
        Ok(())
    }

    fn pop_frame(&mut self) -> Option<Frame> {
        let frame = self.frames.pop()?;
        self.close_upvalues(frame.base);
        self.stack.truncate(frame.base);
        Some(frame)
    }

    /// Appelle `callee` avec `this` et `args`, et attend son résultat.
    ///
    /// # Errors
    /// La valeur levée par la fonction, ou une `TypeError` si `callee` n'est
    /// pas appelable.
    pub fn call(&mut self, callee: &Value, this: &Value, args: &[Value]) -> JsResult {
        let Some(id) = callee.as_object() else {
            return Err(self.type_error("la valeur appelée n'est pas une fonction"));
        };
        match &self.heap.get(id).kind {
            Kind::Native { func, .. } => {
                let func = *func;
                self.enter_native()?;
                let saved = std::mem::replace(&mut self.constructing, false);
                let result = func(self, this, args);
                self.constructing = saved;
                self.native_depth -= 1;
                result
            }
            Kind::NativeData { func, data } => {
                let (func, data) = (*func, data.clone());
                self.enter_native()?;
                let saved = std::mem::replace(&mut self.constructing, false);
                let result = func(self, &data, this, args);
                self.constructing = saved;
                self.native_depth -= 1;
                result
            }
            Kind::Bound {
                target,
                this: bound_this,
                args: bound_args,
            } => {
                let (target, bound_this) = (Value::Object(*target), bound_this.clone());
                let mut all = bound_args.clone();
                all.extend_from_slice(args);
                self.call(&target, &bound_this, &all)
            }
            Kind::Closure(c) => {
                if c.proto.is_coroutine() {
                    return self.start_coroutine(id, this.clone(), args.to_vec());
                }
                let depth = self.frames.len();
                self.push_frame(
                    id,
                    this.clone(),
                    args.to_vec(),
                    Value::Undefined,
                    None,
                    false,
                )?;
                self.run_nested(depth)
            }
            _ => Err(self.type_error("la valeur appelée n'est pas une fonction")),
        }
    }

    fn enter_native(&mut self) -> JsResult<()> {
        if self.native_depth >= MAX_NATIVE_DEPTH {
            return Err(self.error("RangeError", "pile d'appels dépassée"));
        }
        self.native_depth += 1;
        Ok(())
    }

    /// Exécute le cadre qui vient d'être empilé jusqu'à son retour.
    fn run_nested(&mut self, depth: usize) -> JsResult {
        // Si un cadre JavaScript est déjà actif, on est dans une fonction
        // native : le ramasse-miettes doit attendre son retour.
        let nested = depth > 0 || self.native_depth > 0;
        if nested {
            self.enter_native()?;
        }
        let result = self.run(depth);
        if nested {
            self.native_depth -= 1;
        }
        result
    }

    /// Prototype à donner aux instances créées par `new_target`.
    fn instance_proto(&mut self, new_target: &Value) -> JsResult<ObjId> {
        let proto = self.get(new_target, "prototype")?;
        Ok(proto.as_object().unwrap_or(self.intrinsics.object_proto))
    }

    /// `new callee(...args)`, avec `new_target` comme constructeur d'origine.
    ///
    /// # Errors
    /// La valeur levée par le constructeur, ou une `TypeError` si `callee`
    /// n'est pas un constructeur.
    pub fn construct(&mut self, callee: &Value, args: &[Value], new_target: &Value) -> JsResult {
        let Some(id) = callee.as_object() else {
            return Err(self.type_error("la valeur n'est pas un constructeur"));
        };
        match &self.heap.get(id).kind {
            Kind::Native {
                func,
                constructor: true,
            } => {
                let func = *func;
                self.enter_native()?;
                let saved = std::mem::replace(&mut self.constructing, true);
                let result = func(self, &Value::Undefined, args);
                self.constructing = saved;
                self.native_depth -= 1;
                let result = result?;
                // Classe dérivée d'un constructeur natif : l'instance prend le
                // prototype de la classe dérivée.
                if new_target != callee
                    && let Some(instance) = result.as_object()
                {
                    let proto = self.instance_proto(new_target)?;
                    self.heap.get_mut(instance).proto = Some(proto);
                }
                Ok(result)
            }
            Kind::Bound {
                target,
                args: bound_args,
                ..
            } => {
                let target = Value::Object(*target);
                let mut all = bound_args.clone();
                all.extend_from_slice(args);
                let new_target = if new_target == callee {
                    target.clone()
                } else {
                    new_target.clone()
                };
                self.construct(&target, &all, &new_target)
            }
            Kind::Closure(c) => {
                let kind = c.proto.kind;
                let fields = c.fields;
                if !c.proto.is_constructor() {
                    return Err(self.type_error("la valeur n'est pas un constructeur"));
                }
                // Dans une classe dérivée, `this` n'existe qu'après `super()`.
                let this = if kind == (FunctionKind::ClassConstructor { derived: true }) {
                    Value::Hole
                } else {
                    let proto = self.instance_proto(new_target)?;
                    let instance = self.heap.alloc(Object::new(Some(proto), Kind::Ordinary));
                    if let Some(fields) = fields {
                        self.call(&Value::Object(fields), &Value::Object(instance), &[])?;
                    }
                    Value::Object(instance)
                };
                let depth = self.frames.len();
                self.push_frame(id, this, args.to_vec(), new_target.clone(), None, true)?;
                self.run_nested(depth)
            }
            _ => Err(self.type_error("la valeur n'est pas un constructeur")),
        }
    }

    /// Lance un appel depuis la boucle d'interprétation : une fermeture est
    /// empilée (son résultat ira dans `dst`), les autres fonctions sont
    /// exécutées immédiatement.
    fn call_from_loop(
        &mut self,
        dst: Reg,
        callee: &Value,
        this: Value,
        args: Vec<Value>,
    ) -> JsResult<()> {
        if let Some(id) = callee.as_object()
            && matches!(&self.heap.get(id).kind, Kind::Closure(c) if !c.proto.is_coroutine())
        {
            let ret = self.base() + usize::from(dst);
            return self.push_frame(id, this, args, Value::Undefined, Some(ret), false);
        }
        let result = self.call(callee, &this, &args)?;
        self.set_reg(dst, result);
        Ok(())
    }

    /// `new` depuis la boucle d'interprétation : le constructeur d'une
    /// fermeture est empilé comme un appel ordinaire, sans récursion native.
    /// Le corps du constructeur s'exécute ainsi dans la boucle externe, où le
    /// ramasse-miettes peut passer.
    fn construct_from_loop(&mut self, dst: Reg, callee: &Value, args: Vec<Value>) -> JsResult<()> {
        if let Some(id) = callee.as_object()
            && let Kind::Closure(c) = &self.heap.get(id).kind
        {
            let (kind, fields) = (c.proto.kind, c.fields);
            if !c.proto.is_constructor() {
                return Err(self.type_error("la valeur n'est pas un constructeur"));
            }
            let this = if kind == (FunctionKind::ClassConstructor { derived: true }) {
                Value::Hole
            } else {
                let proto = self.instance_proto(callee)?;
                let instance = self.heap.alloc(Object::new(Some(proto), Kind::Ordinary));
                if let Some(fields) = fields {
                    self.call(&Value::Object(fields), &Value::Object(instance), &[])?;
                }
                Value::Object(instance)
            };
            let ret = self.base() + usize::from(dst);
            return self.push_frame(id, this, args, callee.clone(), Some(ret), true);
        }
        let value = self.construct(callee, &args, callee)?;
        self.set_reg(dst, value);
        Ok(())
    }

    fn args_from_array(&mut self, array: &Value) -> Vec<Value> {
        match array.as_object().map(|id| &self.heap.get(id).kind) {
            Some(Kind::Array(items)) => items
                .iter()
                .map(|v| {
                    if *v == Value::Hole {
                        Value::Undefined
                    } else {
                        v.clone()
                    }
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    // ---- Boucle d'interprétation ---------------------------------------------

    /// Exécute jusqu'à ce que la pile de cadres redescende à `depth`.
    fn run(&mut self, depth: usize) -> JsResult {
        loop {
            match self.step(depth) {
                Ok(Some(value)) => return Ok(value),
                Ok(None) => {}
                Err(thrown) => {
                    // Déroule la pile jusqu'à un gestionnaire, sans dépasser `depth`.
                    let mut handled = false;
                    while self.frames.len() > depth {
                        let handler = if self.interrupted {
                            None
                        } else {
                            self.frames.last_mut().and_then(|f| f.handlers.pop())
                        };
                        if let Some((target, reg)) = handler {
                            if let Some(frame) = self.frames.last_mut() {
                                frame.pc = target as usize;
                            }
                            self.set_reg(reg, thrown.clone());
                            handled = true;
                            break;
                        }
                        self.pop_frame();
                    }
                    if !handled {
                        return Err(thrown);
                    }
                }
            }
        }
    }

    /// Exécute une instruction. `Ok(Some(v))` : le cadre d'entrée a rendu `v`.
    #[expect(
        clippy::too_many_lines,
        reason = "aiguillage central de l'interpréteur"
    )]
    fn step(&mut self, depth: usize) -> JsResult<Option<Value>> {
        if let Some(thrown) = self.pending_throw.take() {
            return Err(thrown);
        }
        if self.heap.should_collect() && self.native_depth == 0 {
            self.collect_garbage();
        }
        if let Some(budget) = &mut self.budget {
            if *budget == 0 {
                self.interrupted = true;
                return Err(self.error(
                    "RangeError",
                    "script interrompu : temps d'exécution dépassé",
                ));
            }
            *budget -= 1;
        }
        let Some(frame) = self.frames.last_mut() else {
            return Ok(Some(Value::Undefined));
        };
        let Some(op) = frame.proto.code.get(frame.pc).copied() else {
            // Fin de code sans `Return` : ne se produit pas avec le compilateur.
            return self.do_return(Value::Undefined, depth);
        };
        frame.pc += 1;
        match op {
            Op::Nop => {}
            Op::Yield { dst, src } => {
                let value = self.reg(src);
                let Some(coroutine) = self.frames.last().and_then(|f| f.generator) else {
                    return Err(self.error(
                        "SyntaxError",
                        "« yield » ou « await » hors d'un générateur ou d'une fonction async",
                    ));
                };
                // Le cadre quitte la pile ; il y reviendra à la reprise.
                let Some(frame) = self.frames.pop() else {
                    return Ok(None);
                };
                let base = frame.base;
                let upvalues: Vec<(usize, UpId)> = self
                    .open_upvalues
                    .iter()
                    .filter(|(index, _)| *index >= base)
                    .map(|(index, id)| (*index - base, *id))
                    .collect();
                self.close_upvalues(base);
                let registers = self.stack.split_off(base.min(self.stack.len()));
                let saved = SavedFrame {
                    closure: frame.closure,
                    proto: frame.proto,
                    pc: frame.pc,
                    this: frame.this,
                    new_target: frame.new_target,
                    args: frame.args,
                    handlers: frame.handlers,
                    registers,
                    upvalues,
                    resume_reg: dst,
                };
                self.set_coroutine_state(coroutine, CoState::Suspended(Box::new(saved)));
                return Ok(Some(value));
            }
            Op::LoadConst { dst, index } => {
                let value = self.constant(index);
                self.set_reg(dst, value);
            }
            Op::LoadUndefined { dst } => self.set_reg(dst, Value::Undefined),
            Op::LoadNull { dst } => self.set_reg(dst, Value::Null),
            Op::LoadBool { dst, value } => self.set_reg(dst, Value::Bool(value)),
            Op::LoadHole { dst } => self.set_reg(dst, Value::Hole),
            Op::Move { dst, src } => {
                let value = self.reg(src);
                self.set_reg(dst, value);
            }
            Op::LoadThis { dst } => {
                let this = self
                    .frames
                    .last()
                    .map_or(Value::Undefined, |f| f.this.clone());
                if this == Value::Hole {
                    return Err(self.error(
                        "ReferenceError",
                        "« super() » doit être appelé avant d'utiliser « this »",
                    ));
                }
                self.set_reg(dst, this);
            }
            Op::LoadNewTarget { dst } => {
                let value = self
                    .frames
                    .last()
                    .map_or(Value::Undefined, |f| f.new_target.clone());
                self.set_reg(dst, value);
            }
            Op::LoadCallee { dst } => {
                let value = self
                    .frames
                    .last()
                    .map_or(Value::Undefined, |f| Value::Object(f.closure));
                self.set_reg(dst, value);
            }
            Op::GetGlobal { dst, name } => {
                let name = self.name(name);
                if !self.has_property(self.global, &name) {
                    return Err(self.error("ReferenceError", &format!("{name} n'est pas défini")));
                }
                let value = self.get(&Value::Object(self.global), &name)?;
                self.set_reg(dst, value);
            }
            Op::SetGlobal { name, src } => {
                let (name, value) = (self.name(name), self.reg(src));
                self.set(&Value::Object(self.global), &name, value)?;
            }
            Op::InitGlobal { name, src } => {
                let (name, value) = (self.name(name), self.reg(src));
                if value == Value::Hole {
                    // Déclaration `var` : ne remplace pas une variable existante.
                    if !self.heap.get(self.global).props.contains_key(&name) {
                        self.heap
                            .get_mut(self.global)
                            .props
                            .insert(name, Property::data(Value::Undefined));
                    }
                } else {
                    self.heap
                        .get_mut(self.global)
                        .props
                        .insert(name, Property::data(value));
                }
            }
            Op::TypeofGlobal { dst, name } => {
                let name = self.name(name);
                let value = if self.has_property(self.global, &name) {
                    self.get(&Value::Object(self.global), &name)?
                } else {
                    Value::Undefined
                };
                let type_name = self.type_of(&value);
                self.set_reg(dst, Value::string(type_name));
            }
            Op::GetUpvalue { dst, index } => {
                let value = self.get_upvalue(index);
                self.set_reg(dst, value);
            }
            Op::SetUpvalue { index, src } => {
                let value = self.reg(src);
                self.set_upvalue(index, value);
            }
            Op::CloseUpvalues { from } => {
                let from = self.base() + usize::from(from);
                self.close_upvalues(from);
            }
            Op::CheckInit { src, name } => {
                if self.reg(src) == Value::Hole {
                    let name = self.name(name);
                    return Err(self.error(
                        "ReferenceError",
                        &format!("{name} est utilisé avant son initialisation"),
                    ));
                }
            }
            Op::ThrowConstAssign { name } => {
                let name = self.name(name);
                return Err(self.type_error(&format!("affectation à la constante {name}")));
            }
            Op::GetProp { dst, obj, name } => {
                let (target, name) = (self.reg(obj), self.name(name));
                let value = self.get(&target, &name)?;
                self.set_reg(dst, value);
            }
            Op::SetProp { obj, name, src } => {
                let (target, name, value) = (self.reg(obj), self.name(name), self.reg(src));
                self.set(&target, &name, value)?;
            }
            Op::GetElem { dst, obj, key } => {
                let (target, key) = (self.reg(obj), self.reg(key));
                let value = self.get_element(&target, &key)?;
                self.set_reg(dst, value);
            }
            Op::SetElem { obj, key, src } => {
                let (target, key, value) = (self.reg(obj), self.reg(key), self.reg(src));
                self.set_element(&target, &key, value)?;
            }
            Op::DeleteProp { dst, obj, name } => {
                let (target, name) = (self.reg(obj), self.name(name));
                let deleted = self.delete(&target, &name)?;
                self.set_reg(dst, Value::Bool(deleted));
            }
            Op::DeleteElem { dst, obj, key } => {
                let target = self.reg(obj);
                let key = self.to_key(&self.reg(key))?;
                let deleted = self.delete_key(&target, &key)?;
                self.set_reg(dst, Value::Bool(deleted));
            }
            Op::GetSuper { dst, key } => {
                let key = self.to_key(&self.reg(key))?;
                let (home, this) = match self.frames.last() {
                    Some(f) => match &self.heap.get(f.closure).kind {
                        Kind::Closure(c) => (c.home, f.this.clone()),
                        _ => (None, Value::Undefined),
                    },
                    None => (None, Value::Undefined),
                };
                let parent = home.and_then(|h| self.heap.get(h).proto);
                let value = match parent {
                    Some(parent) => self.get_key_with_receiver(parent, &key, &this)?,
                    None => Value::Undefined,
                };
                self.set_reg(dst, value);
            }
            Op::NewObject { dst } => {
                let id = self.new_object();
                self.set_reg(dst, Value::Object(id));
            }
            Op::NewArray { dst } => {
                let array = self.new_array(Vec::new());
                self.set_reg(dst, array);
            }
            Op::ArrayPush { array, src } => {
                let value = self.reg(src);
                if let Some(Kind::Array(items)) = self
                    .reg(array)
                    .as_object()
                    .map(|id| &mut self.heap.get_mut(id).kind)
                {
                    items.push(value);
                }
            }
            Op::ArrayHole { array } => {
                if let Some(Kind::Array(items)) = self
                    .reg(array)
                    .as_object()
                    .map(|id| &mut self.heap.get_mut(id).kind)
                {
                    items.push(Value::Hole);
                }
            }
            Op::ArraySpread { array, src } => {
                let values = self.iterate_to_vec(&self.reg(src))?;
                if let Some(Kind::Array(items)) = self
                    .reg(array)
                    .as_object()
                    .map(|id| &mut self.heap.get_mut(id).kind)
                {
                    items.extend(values);
                }
            }
            Op::ObjectSpread { obj, src } => {
                let (target, source) = (self.reg(obj), self.reg(src));
                self.copy_properties(&target, &source)?;
            }
            Op::InitProp { obj, name, src } => {
                let (name, value) = (self.name(name), self.reg(src));
                if let Some(id) = self.reg(obj).as_object() {
                    self.heap
                        .get_mut(id)
                        .props
                        .insert(name, Property::data(value));
                }
            }
            Op::InitElem { obj, key, src } => {
                let key = self.to_key(&self.reg(key))?;
                let value = self.reg(src);
                if let Some(id) = self.reg(obj).as_object() {
                    self.heap
                        .get_mut(id)
                        .insert_prop(key, Property::data(value));
                }
            }
            Op::InitMethod { obj, key, func } => {
                let key = self.to_key(&self.reg(key))?;
                let value = self.reg(func);
                if let Some(id) = self.reg(obj).as_object() {
                    self.heap
                        .get_mut(id)
                        .insert_prop(key, Property::hidden(value));
                }
            }
            Op::InitGetter {
                obj,
                key,
                func,
                enumerable,
            }
            | Op::InitSetter {
                obj,
                key,
                func,
                enumerable,
            } => {
                let key = self.to_key(&self.reg(key))?;
                let accessor = self.reg(func).as_object();
                let is_getter = matches!(op, Op::InitGetter { .. });
                if let Some(id) = self.reg(obj).as_object() {
                    let props = self.heap.get_mut(id);
                    let (mut get, mut set) = match props.prop(&key).map(|p| &p.slot) {
                        Some(Slot::Accessor { get, set }) => (*get, *set),
                        _ => (None, None),
                    };
                    if is_getter {
                        get = accessor;
                    } else {
                        set = accessor;
                    }
                    props.insert_prop(
                        key,
                        Property {
                            slot: Slot::Accessor { get, set },
                            writable: true,
                            enumerable,
                            configurable: true,
                        },
                    );
                }
            }
            Op::Binary {
                op,
                dst,
                left,
                right,
            } => {
                let (l, r) = (self.reg(left), self.reg(right));
                let value = self.binary(op, &l, &r)?;
                self.set_reg(dst, value);
            }
            Op::Unary { op, dst, src } => {
                let value = self.reg(src);
                let result = self.unary(op, &value)?;
                self.set_reg(dst, result);
            }
            Op::ToNumber { dst, src } => {
                let n = self.to_number(&self.reg(src))?;
                self.set_reg(dst, Value::Number(n));
            }
            Op::ToString { dst, src } => {
                let s = self.to_string(&self.reg(src))?;
                self.set_reg(dst, Value::String(s));
            }
            Op::ToPropertyKey { dst, src } => {
                let key = self.to_key(&self.reg(src))?;
                self.set_reg(dst, key.to_value());
            }
            Op::Jump { target } => self.jump(target),
            Op::JumpIfFalse { src, target } => {
                if !self.reg(src).truthy() {
                    self.jump(target);
                }
            }
            Op::JumpIfTrue { src, target } => {
                if self.reg(src).truthy() {
                    self.jump(target);
                }
            }
            Op::JumpIfNullish { src, target } => {
                if self.reg(src).is_nullish() {
                    self.jump(target);
                }
            }
            Op::JumpIfNotNullish { src, target } => {
                if !self.reg(src).is_nullish() {
                    self.jump(target);
                }
            }
            Op::JumpIfNotUndefined { src, target } => {
                if !matches!(self.reg(src), Value::Undefined | Value::Hole) {
                    self.jump(target);
                }
            }
            Op::Closure { dst, index } => {
                let Some(frame) = self.frames.last() else {
                    return Ok(None);
                };
                let Some(proto) = frame.proto.functions.get(index as usize).cloned() else {
                    return Ok(None);
                };
                let (base, parent, this) = (frame.base, frame.closure, frame.this.clone());
                let mut upvalues = Vec::with_capacity(proto.upvalues.len());
                for desc in &proto.upvalues {
                    let id = if desc.from_parent_register {
                        self.capture(base + usize::from(desc.index))
                    } else {
                        match &self.heap.get(parent).kind {
                            Kind::Closure(c) => c.upvalues.get(usize::from(desc.index)).copied(),
                            _ => None,
                        }
                        .unwrap_or_else(|| {
                            self.heap.alloc_upvalue(Upvalue::Closed(Value::Undefined))
                        })
                    };
                    upvalues.push(id);
                }
                let is_arrow = proto.kind == FunctionKind::Arrow;
                // Une fonction fléchée hérite de l'objet d'attache (pour `super`).
                let home = match (&self.heap.get(parent).kind, is_arrow) {
                    (Kind::Closure(c), true) => c.home,
                    _ => None,
                };
                let closure = Closure {
                    proto,
                    upvalues,
                    bound_this: is_arrow.then_some(this),
                    home,
                    parent: None,
                    fields: None,
                };
                let id = self.heap.alloc(Object::new(
                    Some(self.intrinsics.function_proto),
                    Kind::Closure(closure),
                ));
                self.set_reg(dst, Value::Object(id));
            }
            Op::Call {
                dst,
                callee,
                this,
                first,
                argc,
            } => {
                let (callee, this) = (self.reg(callee), self.reg(this));
                let args: Vec<Value> = (0..argc).map(|i| self.reg(first + i)).collect();
                self.check_callable(&callee)?;
                self.call_from_loop(dst, &callee, this, args)?;
            }
            Op::CallSpread {
                dst,
                callee,
                this,
                args,
            } => {
                let (callee, this) = (self.reg(callee), self.reg(this));
                let args = self.args_from_array(&self.reg(args));
                self.check_callable(&callee)?;
                self.call_from_loop(dst, &callee, this, args)?;
            }
            Op::New {
                dst,
                callee,
                first,
                argc,
            } => {
                let callee = self.reg(callee);
                let args: Vec<Value> = (0..argc).map(|i| self.reg(first + i)).collect();
                self.construct_from_loop(dst, &callee, args)?;
            }
            Op::NewSpread { dst, callee, args } => {
                let callee = self.reg(callee);
                let args = self.args_from_array(&self.reg(args));
                self.construct_from_loop(dst, &callee, args)?;
            }
            Op::SuperCall { dst, args } => {
                let args = self.args_from_array(&self.reg(args));
                let Some(frame) = self.frames.last() else {
                    return Ok(None);
                };
                let (closure, new_target) = (frame.closure, frame.new_target.clone());
                if frame.this != Value::Hole {
                    return Err(self.error("ReferenceError", "« super() » a déjà été appelé"));
                }
                let (parent, fields) = match &self.heap.get(closure).kind {
                    Kind::Closure(c) => (c.parent, c.fields),
                    _ => (None, None),
                };
                let Some(parent) = parent else {
                    return Err(self.type_error("« super() » hors d'une classe dérivée"));
                };
                let instance = self.construct(&Value::Object(parent), &args, &new_target)?;
                if let Some(frame) = self.frames.last_mut() {
                    frame.this = instance.clone();
                }
                if let Some(fields) = fields {
                    self.call(&Value::Object(fields), &instance, &[])?;
                }
                self.set_reg(dst, instance);
            }
            Op::Return { src } => {
                let value = self.reg(src);
                return self.do_return(value, depth);
            }
            Op::Throw { src } => return Err(self.reg(src)),
            Op::PushHandler { target, reg } => {
                if let Some(frame) = self.frames.last_mut() {
                    frame.handlers.push((target, reg));
                }
            }
            Op::PopHandler => {
                if let Some(frame) = self.frames.last_mut() {
                    frame.handlers.pop();
                }
            }
            Op::ForInStart { dst, obj } => {
                let keys = self.for_in_keys(&self.reg(obj));
                let iter = self.new_iterator(IterState::Keys { keys, index: 0 });
                self.set_reg(dst, iter);
            }
            Op::IterStart { dst, src } => {
                let iter = self.iterator_for(&self.reg(src))?;
                self.set_reg(dst, iter);
            }
            Op::IterNext { dst, iter, done } => match self.iterator_next(&self.reg(iter))? {
                Some(value) => self.set_reg(dst, value),
                None => self.jump(done),
            },
            Op::IterNextOrUndefined { dst, iter } => {
                let value = self
                    .iterator_next(&self.reg(iter))?
                    .unwrap_or(Value::Undefined);
                self.set_reg(dst, value);
            }
            Op::IterRest { dst, iter } => {
                let iter = self.reg(iter);
                let mut rest = Vec::new();
                while let Some(value) = self.iterator_next(&iter)? {
                    rest.push(value);
                }
                let array = self.new_array(rest);
                self.set_reg(dst, array);
            }
            Op::RestArgs { dst, from } => {
                let rest: Vec<Value> = self
                    .frames
                    .last()
                    .map(|f| f.args.iter().skip(usize::from(from)).cloned().collect())
                    .unwrap_or_default();
                let array = self.new_array(rest);
                self.set_reg(dst, array);
            }
            Op::Arguments { dst } => {
                let args = self
                    .frames
                    .last()
                    .map(|f| f.args.clone())
                    .unwrap_or_default();
                let array = self.new_array(args);
                self.set_reg(dst, array);
            }
            Op::RegExp {
                dst,
                pattern,
                flags,
            } => {
                let (pattern, flags) = (self.name(pattern), self.name(flags));
                let regexp = self.new_regexp(&pattern, &flags)?;
                self.set_reg(dst, regexp);
            }
            Op::MakeClass {
                ctor,
                proto,
                parent,
            } => {
                let ctor_id = self.reg(ctor).as_object();
                let parent = parent.map(|r| self.reg(r));
                let mut proto_parent = Some(self.intrinsics.object_proto);
                let mut parent_ctor = None;
                match &parent {
                    None => {}
                    Some(Value::Null) => proto_parent = None,
                    Some(value) => {
                        if !self.is_constructor(value) {
                            return Err(self.type_error("la classe mère n'est pas un constructeur"));
                        }
                        proto_parent = self.get(value, "prototype")?.as_object();
                        parent_ctor = value.as_object();
                    }
                }
                let proto_id = self.heap.alloc(Object::new(proto_parent, Kind::Ordinary));
                if let Some(ctor_id) = ctor_id {
                    self.heap.get_mut(proto_id).props.insert(
                        Arc::from("constructor"),
                        Property::hidden(Value::Object(ctor_id)),
                    );
                    let object = self.heap.get_mut(ctor_id);
                    object.props.insert(
                        Arc::from("prototype"),
                        Property {
                            slot: Slot::Data(Value::Object(proto_id)),
                            writable: false,
                            enumerable: false,
                            configurable: false,
                        },
                    );
                    if let Some(parent_ctor) = parent_ctor {
                        object.proto = Some(parent_ctor);
                    }
                    if let Kind::Closure(c) = &mut object.kind {
                        c.parent = parent_ctor;
                    }
                }
                self.set_reg(proto, Value::Object(proto_id));
            }
            Op::SetHome { func, home } => {
                let home = self.reg(home).as_object();
                if let Some(Kind::Closure(c)) = self
                    .reg(func)
                    .as_object()
                    .map(|id| &mut self.heap.get_mut(id).kind)
                {
                    c.home = home;
                }
            }
            Op::SetFields { ctor, func } => {
                let fields = self.reg(func).as_object();
                if let Some(Kind::Closure(c)) = self
                    .reg(ctor)
                    .as_object()
                    .map(|id| &mut self.heap.get_mut(id).kind)
                {
                    c.fields = fields;
                }
            }
        }
        Ok(None)
    }

    fn jump(&mut self, target: u32) {
        if let Some(frame) = self.frames.last_mut() {
            frame.pc = target as usize;
        }
    }

    fn check_callable(&mut self, callee: &Value) -> JsResult<()> {
        if callee
            .as_object()
            .is_some_and(|id| self.heap.get(id).is_callable())
        {
            return Ok(());
        }
        let shown = self.display(callee);
        Err(self.type_error(&format!("{shown} n'est pas une fonction")))
    }

    /// Termine le cadre courant avec `value`.
    fn do_return(&mut self, value: Value, depth: usize) -> JsResult<Option<Value>> {
        let Some(frame) = self.pop_frame() else {
            return Ok(Some(value));
        };
        let value = if frame.construct && value.as_object().is_none() {
            if frame.this == Value::Hole {
                return Err(self.error(
                    "ReferenceError",
                    "le constructeur d'une classe dérivée doit appeler « super() »",
                ));
            }
            frame.this
        } else {
            value
        };
        match frame.ret {
            Some(index) if self.frames.len() > depth => {
                if let Some(slot) = self.stack.get_mut(index) {
                    *slot = value;
                }
                Ok(None)
            }
            _ => Ok(Some(value)),
        }
    }

    /// Exécute une fonction compilée de premier niveau (un script).
    pub(crate) fn run_script(&mut self, proto: Arc<FunctionProto>) -> JsResult {
        let closure = Closure {
            proto,
            upvalues: Vec::new(),
            bound_this: None,
            home: None,
            parent: None,
            fields: None,
        };
        let id = self.heap.alloc(Object::new(
            Some(self.intrinsics.function_proto),
            Kind::Closure(closure),
        ));
        let depth = self.frames.len();
        self.push_frame(
            id,
            Value::Object(self.global),
            Vec::new(),
            Value::Undefined,
            None,
            false,
        )?;
        self.run_nested(depth)
    }

    pub(crate) fn new_iterator(&mut self, state: IterState) -> Value {
        let proto = Some(self.intrinsics.iterator_proto);
        Value::Object(self.heap.alloc(Object::new(proto, Kind::Iterator(state))))
    }
}

// ---- Coroutines : générateurs et fonctions async ---------------------------------
//
// Une coroutine possède un cadre d'exécution qui peut quitter la pile
// (`Op::Yield`) puis y revenir (`Vm::resume`). INVARIANT : le cadre d'une
// coroutine est toujours le cadre d'entrée d'une boucle `run` imbriquée,
// lancée par `resume` ; `Op::Yield` peut donc rendre la main en renvoyant
// simplement une valeur à cette boucle.

impl Vm {
    fn set_coroutine_state(&mut self, coroutine: ObjId, state: CoState) {
        if let Kind::Coroutine(c) = &mut self.heap.get_mut(coroutine).kind {
            c.state = state;
        }
    }

    /// Appel d'un générateur ou d'une fonction async : crée la coroutine.
    /// Un générateur ne démarre qu'au premier `next()` ; une fonction async
    /// s'exécute tout de suite jusqu'à son premier `await`.
    fn start_coroutine(&mut self, closure: ObjId, this: Value, args: Vec<Value>) -> JsResult {
        let is_async = matches!(&self.heap.get(closure).kind, Kind::Closure(c) if c.proto.is_async);
        let state = CoState::Start {
            closure,
            this,
            args,
        };
        if !is_async {
            let coroutine = Coroutine {
                state,
                promise: None,
            };
            return Ok(Value::Object(self.heap.alloc(Object::new(
                Some(self.intrinsics.generator_proto),
                Kind::Coroutine(Box::new(coroutine)),
            ))));
        }
        let promise = self.new_promise();
        let coroutine = self.heap.alloc(Object::new(
            None,
            Kind::Coroutine(Box::new(Coroutine {
                state,
                promise: Some(promise),
            })),
        ));
        // Des identifiants d'objets vivent dans des variables Rust pendant
        // l'étape : le ramasse-miettes doit attendre.
        self.enter_native()?;
        self.async_step(coroutine, Resume::Next(Value::Undefined));
        self.native_depth -= 1;
        Ok(Value::Object(promise))
    }

    /// Reprend l'exécution de `coroutine` jusqu'à sa prochaine suspension ou
    /// sa fin.
    ///
    /// # Errors
    /// La valeur levée par le corps de la coroutine ; `TypeError` si elle est
    /// déjà en cours d'exécution.
    pub(crate) fn resume(&mut self, coroutine: ObjId, mode: Resume) -> JsResult<CoOutcome> {
        let state = match &mut self.heap.get_mut(coroutine).kind {
            Kind::Coroutine(c) => std::mem::replace(&mut c.state, CoState::Running),
            _ => return Err(self.type_error("le receveur n'est pas un générateur")),
        };
        let depth = self.frames.len();
        match state {
            CoState::Running => {
                return Err(self.type_error("le générateur est déjà en cours d'exécution"));
            }
            CoState::Done => {
                self.set_coroutine_state(coroutine, CoState::Done);
                return match mode {
                    Resume::Next(_) => Ok(CoOutcome::Return(Value::Undefined)),
                    Resume::Return(value) => Ok(CoOutcome::Return(value)),
                    Resume::Throw(thrown) => Err(thrown),
                };
            }
            CoState::Start {
                closure,
                this,
                args,
            } => {
                let early = match mode {
                    Resume::Next(_) => None,
                    Resume::Return(value) => Some(Ok(CoOutcome::Return(value))),
                    Resume::Throw(thrown) => Some(Err(thrown)),
                };
                if let Some(outcome) = early {
                    self.set_coroutine_state(coroutine, CoState::Done);
                    return outcome;
                }
                if let Err(thrown) =
                    self.push_frame(closure, this, args, Value::Undefined, None, false)
                {
                    self.set_coroutine_state(coroutine, CoState::Done);
                    return Err(thrown);
                }
                if let Some(frame) = self.frames.last_mut() {
                    frame.generator = Some(coroutine);
                }
            }
            CoState::Suspended(saved) => {
                // LIMITE : `return()` termine la coroutine sans exécuter ses
                // blocs `finally` en attente.
                if let Resume::Return(value) = mode {
                    self.set_coroutine_state(coroutine, CoState::Done);
                    return Ok(CoOutcome::Return(value));
                }
                let saved = *saved;
                let base = self.stack.len();
                if self.frames.len() >= MAX_FRAMES || base + saved.registers.len() > MAX_STACK {
                    self.set_coroutine_state(coroutine, CoState::Done);
                    return Err(self.error("RangeError", "pile d'appels dépassée"));
                }
                self.stack.extend(saved.registers);
                // Les variables capturées reviennent dans la pile : leur
                // valeur, gardée par la cellule pendant la suspension, est
                // recopiée dans le registre, qui redevient la référence.
                for (offset, id) in saved.upvalues {
                    let index = base + offset;
                    let Some(Upvalue::Closed(value)) = self.heap.upvalue(id).cloned() else {
                        continue;
                    };
                    if let Some(slot) = self.stack.get_mut(index) {
                        *slot = value;
                    }
                    if let Some(cell) = self.heap.upvalue_mut(id) {
                        *cell = Upvalue::Open(index);
                    }
                    self.open_upvalues.push((index, id));
                }
                self.frames.push(Frame {
                    closure: saved.closure,
                    proto: saved.proto,
                    pc: saved.pc,
                    base,
                    this: saved.this,
                    new_target: saved.new_target,
                    args: saved.args,
                    handlers: saved.handlers,
                    ret: None,
                    construct: false,
                    generator: Some(coroutine),
                });
                match mode {
                    Resume::Next(value) => {
                        if let Some(slot) = self.stack.get_mut(base + usize::from(saved.resume_reg))
                        {
                            *slot = value;
                        }
                    }
                    Resume::Throw(thrown) => self.pending_throw = Some(thrown),
                    Resume::Return(_) => {}
                }
            }
        }
        let result = self.run_nested(depth);
        // Si la boucle n'a pas pu démarrer (pile native pleine), l'exception en
        // attente et le cadre empilé ne doivent pas survivre à cet appel.
        self.pending_throw = None;
        while self.frames.len() > depth {
            self.pop_frame();
        }
        let suspended = matches!(
            &self.heap.get(coroutine).kind,
            Kind::Coroutine(c) if matches!(c.state, CoState::Suspended(_))
        );
        match result {
            Ok(value) if suspended => Ok(CoOutcome::Yield(value)),
            Ok(value) => {
                self.set_coroutine_state(coroutine, CoState::Done);
                Ok(CoOutcome::Return(value))
            }
            Err(thrown) => {
                self.set_coroutine_state(coroutine, CoState::Done);
                Err(thrown)
            }
        }
    }

    /// Coroutine de générateur désignée par `value` (pas une fonction async).
    pub(crate) fn generator_id(&self, value: &Value) -> Option<ObjId> {
        let id = value.as_object()?;
        match &self.heap.get(id).kind {
            Kind::Coroutine(c) if c.promise.is_none() => Some(id),
            _ => None,
        }
    }

    /// Promesse résultat de la fonction async `coroutine`.
    pub(crate) fn coroutine_promise(&self, coroutine: ObjId) -> Option<ObjId> {
        match &self.heap.get(coroutine).kind {
            Kind::Coroutine(c) => c.promise,
            _ => None,
        }
    }

    /// Exécute `work` sans que le ramasse-miettes puisse passer : à utiliser
    /// quand du code Rust garde des identifiants d'objets non enracinés.
    pub(crate) fn without_gc<T>(&mut self, work: impl FnOnce(&mut Self) -> T) -> T {
        self.native_depth += 1;
        let result = work(self);
        self.native_depth -= 1;
        result
    }

    /// Vrai si aucun code natif n'est en cours : point où les microtâches
    /// peuvent s'exécuter.
    pub(crate) fn at_top_level(&self) -> bool {
        self.native_depth == 0 && self.frames.is_empty()
    }

    /// Vrai si le budget d'instructions est épuisé.
    pub(crate) fn is_interrupted(&self) -> bool {
        self.interrupted
    }
}
