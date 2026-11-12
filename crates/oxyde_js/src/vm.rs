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
    Closure, CoKind, CoState, Coroutine, Heap, IterState, Kind, Object, Property, SavedFrame, Slot,
    UpId, Upvalue,
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
#[derive(Clone, Debug)]
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
    /// Suspendue par `await`, dans l'attente de cette valeur.
    Await(Value),
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
    /// Prototype des itérateurs internes (porte `next`).
    pub(crate) iterator_proto: ObjId,
    /// `Iterator.prototype`, racine de tous les itérateurs.
    pub(crate) iterator_root: ObjId,
    pub(crate) iterator_helper_proto: ObjId,
    pub(crate) wrap_iterator_proto: ObjId,
    pub(crate) regexp_string_iterator_proto: ObjId,
    /// Le constructeur `Iterator` (provisoire jusqu'à son installation).
    pub(crate) iterator_ctor: ObjId,
    pub(crate) generator_proto: ObjId,
    /// `GeneratorFunction.prototype`, `AsyncFunction.prototype` et
    /// `AsyncGeneratorFunction.prototype` : prototypes des fonctions de ces genres.
    pub(crate) generator_function_proto: ObjId,
    pub(crate) async_function_proto: ObjId,
    pub(crate) async_generator_function_proto: ObjId,
    pub(crate) promise_proto: ObjId,
    pub(crate) regexp_proto: ObjId,
    pub(crate) async_generator_proto: ObjId,
    pub(crate) array_buffer_proto: ObjId,
    pub(crate) data_view_proto: ObjId,
    pub(crate) bigint_proto: ObjId,
    pub(crate) date_proto: ObjId,
}

/// Symboles connus du langage (`Symbol.iterator`…).
#[derive(Clone, Debug)]
pub(crate) struct WellKnown {
    pub(crate) iterator: Symbol,
    pub(crate) async_iterator: Symbol,
    pub(crate) has_instance: Symbol,
    pub(crate) to_primitive: Symbol,
    pub(crate) to_string_tag: Symbol,
    pub(crate) is_concat_spreadable: Symbol,
    pub(crate) species: Symbol,
    pub(crate) unscopables: Symbol,
    pub(crate) match_: Symbol,
    pub(crate) match_all: Symbol,
    pub(crate) replace: Symbol,
    pub(crate) search: Symbol,
    pub(crate) split: Symbol,
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
    /// Portée de premier niveau du code exécuté (`None` : objet global).
    scope: Option<ObjId>,
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
    /// Méthodes et accesseurs d'origine de `RegExp.prototype` : tant qu'ils
    /// sont en place, les méthodes symboles prennent un raccourci.
    pub(crate) regexp_originals: Vec<(JsStr, ObjId)>,
    /// Accesseur d'origine de `RegExp[Symbol.species]`.
    pub(crate) regexp_species: Option<ObjId>,
    /// Environnement lexical global : liaisons `let`, `const` et `class` de
    /// premier niveau des scripts. Une valeur « trou » : pas encore
    /// initialisée ; non inscriptible : `const`.
    global_lexicals: ObjId,
    /// Noms déclarés par `var` ou par une fonction au premier niveau d'un script.
    global_vars: std::collections::HashSet<JsStr>,
    /// Objets des gabarits étiquetés déjà évalués, par fonction compilée et
    /// index ; la fonction est gardée pour que son adresse reste la sienne.
    template_objects: std::collections::HashMap<(usize, u32), (Arc<FunctionProto>, ObjId)>,
    /// Dernier texte préparé pour une recherche.
    pub(crate) regex_input: Option<(JsStr, Arc<crate::builtins::regexp::RegexInput>)>,
    /// Prototypes des tableaux typés, dans l'ordre de leurs types d'éléments.
    pub(crate) typed_protos: Vec<ObjId>,
    /// Modules chargés, par identifiant.
    pub(crate) modules: IndexMap<JsStr, crate::module::ModuleRecord>,
    /// Fonction de l'hôte qui fournit le texte des modules importés.
    pub(crate) module_loader: Option<crate::module::ModuleLoader>,
    /// `Array.prototype[Symbol.iterator]` et `next` des itérateurs internes,
    /// tels qu'installés : tant qu'ils sont en place, un tableau s'itère
    /// sans passer par le protocole.
    pub(crate) iteration_intrinsics: Option<(ObjId, ObjId)>,
    /// Identifiant des scripts classiques, référent de leurs `import()`.
    pub(crate) script_url: JsStr,
    /// Exception à lever à la prochaine instruction (reprise par `throw()`).
    pending_throw: Option<Value>,
    /// File des microtâches (réactions de promesses).
    pub(crate) jobs: VecDeque<Job>,
    /// Promesses rejetées sans gestionnaire, à signaler en fin de microtâches.
    pub(crate) rejections: Vec<ObjId>,
    /// Vrai pendant l'appel d'un constructeur natif par `new`.
    pub(crate) constructing: bool,
    /// `new.target` du constructeur natif en cours d'appel.
    pub(crate) native_new_target: Value,
    /// `Function.prototype[Symbol.hasInstance]` d'origine : `instanceof` s'en
    /// passe et applique directement la règle ordinaire.
    pub(crate) has_instance_function: Option<ObjId>,
    /// Le constructeur `Promise` d'origine : ses promesses sont créées et
    /// réglées directement, sans passer par un exécuteur.
    pub(crate) promise_ctor: Option<ObjId>,
    /// Intégrés écrits en JavaScript : `toString` les montre comme natifs.
    pub(crate) self_hosted: Vec<ObjId>,
    /// Le constructeur `Array` d'origine : un résultat qu'il bâtirait est
    /// un tableau ordinaire, créé directement.
    pub(crate) array_ctor: Option<ObjId>,
    /// La fonction `eval` d'origine : un appel écrit `eval(…)` qui la
    /// désigne est un « eval direct ».
    pub(crate) eval_function: Option<ObjId>,
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
            iterator_root: proto(&mut heap, Kind::Ordinary),
            iterator_helper_proto: proto(&mut heap, Kind::Ordinary),
            wrap_iterator_proto: proto(&mut heap, Kind::Ordinary),
            regexp_string_iterator_proto: proto(&mut heap, Kind::Ordinary),
            iterator_ctor: object_proto,
            generator_proto: proto(&mut heap, Kind::Ordinary),
            generator_function_proto: proto(&mut heap, Kind::Ordinary),
            async_function_proto: proto(&mut heap, Kind::Ordinary),
            async_generator_function_proto: proto(&mut heap, Kind::Ordinary),
            promise_proto: proto(&mut heap, Kind::Ordinary),
            regexp_proto: proto(&mut heap, Kind::Ordinary),
            async_generator_proto: proto(&mut heap, Kind::Ordinary),
            array_buffer_proto: proto(&mut heap, Kind::Ordinary),
            data_view_proto: proto(&mut heap, Kind::Ordinary),
            bigint_proto: proto(&mut heap, Kind::Ordinary),
            date_proto: proto(&mut heap, Kind::Ordinary),
        };
        let global = proto(&mut heap, Kind::Ordinary);
        let global_lexicals = heap.alloc(Object::new(None, Kind::Ordinary));
        let known = |id: u64, name: &str| Symbol::new(id, Some(Arc::from(name)));
        let well_known = WellKnown {
            iterator: known(1, "Symbol.iterator"),
            async_iterator: known(2, "Symbol.asyncIterator"),
            has_instance: known(3, "Symbol.hasInstance"),
            to_primitive: known(4, "Symbol.toPrimitive"),
            to_string_tag: known(5, "Symbol.toStringTag"),
            is_concat_spreadable: known(6, "Symbol.isConcatSpreadable"),
            species: known(7, "Symbol.species"),
            unscopables: known(8, "Symbol.unscopables"),
            match_: known(9, "Symbol.match"),
            match_all: known(10, "Symbol.matchAll"),
            replace: known(11, "Symbol.replace"),
            search: known(12, "Symbol.search"),
            split: known(13, "Symbol.split"),
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
            next_symbol: 14,
            symbol_registry: IndexMap::new(),
            native_depth: 0,
            regex_cache: std::collections::HashMap::new(),
            regexp_originals: Vec::new(),
            regexp_species: None,
            global_lexicals,
            global_vars: std::collections::HashSet::new(),
            template_objects: std::collections::HashMap::new(),
            regex_input: None,
            typed_protos: Vec::new(),
            modules: IndexMap::new(),
            module_loader: None,
            iteration_intrinsics: None,
            script_url: Arc::from(""),
            pending_throw: None,
            jobs: VecDeque::new(),
            rejections: Vec::new(),
            constructing: false,
            native_new_target: Value::Undefined,
            has_instance_function: None,
            promise_ctor: None,
            self_hosted: Vec::new(),
            array_ctor: None,
            eval_function: None,
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

    /// Impute `steps` pas à une boucle native et l'arrête si le budget est
    /// épuisé : sans cela, une méthode appliquée à un objet de longueur
    /// démesurée (`{ length: 2 ** 53 }`) tournerait sans fin.
    ///
    /// # Errors
    /// `RangeError` que le script ne peut pas rattraper, comme pour le bytecode.
    pub(crate) fn tick(&mut self, steps: u64) -> JsResult<()> {
        let Some(budget) = &mut self.budget else {
            return Ok(());
        };
        if *budget < steps {
            *budget = 0;
            self.interrupted = true;
            return Err(self.error(
                "RangeError",
                "script interrompu : temps d'exécution dépassé",
            ));
        }
        *budget -= steps;
        Ok(())
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
            intr.iterator_root,
            intr.iterator_helper_proto,
            intr.wrap_iterator_proto,
            intr.regexp_string_iterator_proto,
            intr.iterator_ctor,
            intr.generator_proto,
            intr.generator_function_proto,
            intr.async_function_proto,
            intr.async_generator_function_proto,
            intr.promise_proto,
            intr.regexp_proto,
            intr.async_generator_proto,
            intr.array_buffer_proto,
            intr.data_view_proto,
            intr.bigint_proto,
            intr.date_proto,
        ];
        let frame_values = self.frames.iter().flat_map(|f| {
            [
                Value::Object(f.closure),
                f.this.clone(),
                f.new_target.clone(),
                f.generator.map_or(Value::Undefined, Value::Object),
                f.scope.map_or(Value::Undefined, Value::Object),
            ]
            .into_iter()
            .chain(f.args.iter().cloned())
        });
        let mut job_values = Vec::new();
        for job in &self.jobs {
            job.trace(&mut job_values);
        }
        for module in self.modules.values() {
            module.trace(&mut job_values);
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
            .chain(self.typed_protos.iter().map(|id| Value::Object(*id)))
            .chain(self.pending_throw.iter().cloned())
            .chain(std::iter::once(Value::Object(self.global_lexicals)))
            .chain(
                self.template_objects
                    .values()
                    .map(|(_, id)| Value::Object(*id)),
            )
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
            .insert(Arc::from("length"), hidden(Value::Number(f64::from(arity))));
        object
            .props
            .insert(Arc::from("name"), hidden(Value::string(name)));
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
        if !construct && matches!(proto.kind, FunctionKind::ClassConstructor { .. }) {
            return Err(
                self.type_error("le constructeur d'une classe doit être appelé avec « new »")
            );
        }
        let scope = c.scope;
        let lexical_this = c.bound_this.is_some();
        let this = c.bound_this.clone().unwrap_or(this);
        // Hors mode strict, `this` est toujours un objet.
        let this = if proto.strict || lexical_this || construct {
            this
        } else {
            self.sloppy_this(this)
        };
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
            scope,
        });
        Ok(())
    }

    /// Première moitié d'un pas de `yield*` (ECMA-262 §15.5.5) : transmet à
    /// l'itérateur délégué la reprise reçue (`mode` : 0 `next`, 1 `throw`,
    /// 2 `return`). Rend le résultat de l'appel ; le booléen est vrai si
    /// `return` est absent, auquel cas le résultat est la valeur reçue.
    fn delegate_call(
        &mut self,
        iterator: &Value,
        next: &Value,
        received: Value,
        mode: &Value,
    ) -> JsResult<(Value, bool)> {
        let returning = *mode == Value::Number(2.0);
        let result = if *mode == Value::Number(1.0) {
            let throw = self.get(iterator, "throw")?;
            if throw.is_nullish() {
                // Le délégué ne sait pas recevoir d'exception : on le ferme.
                let close = self.get(iterator, "return")?;
                if !close.is_nullish() {
                    let closed = self.call(&close, iterator, &[])?;
                    if closed.as_object().is_none() {
                        return Err(self.type_error("le résultat de « return » doit être un objet"));
                    }
                }
                return Err(self.type_error("l'itérateur délégué n'a pas de méthode « throw »"));
            }
            self.call(&throw, iterator, &[received])?
        } else if returning {
            let close = self.get(iterator, "return")?;
            if close.is_nullish() {
                return Ok((received, true));
            }
            self.call(&close, iterator, &[received])?
        } else {
            self.call(next, iterator, &[received])?
        };
        Ok((result, false))
    }

    /// Seconde moitié : lit `done` et `value` du résultat. Rend la valeur, si
    /// le délégué est terminé, et si le générateur doit lui-même se terminer
    /// (`return` honoré).
    fn delegate_finish(&mut self, result: &Value, mode: &Value) -> JsResult<(Value, bool, bool)> {
        let returning = *mode == Value::Number(2.0);
        if result.as_object().is_none() {
            return Err(self.type_error("le résultat d'un itérateur doit être un objet"));
        }
        let done = self.get(result, "done")?.truthy();
        let value = self.get(result, "value")?;
        Ok((value, done, done && returning))
    }

    /// Donne son nom à une méthode anonyme (clé calculée) : la clé, entre
    /// crochets pour un symbole, précédée de `prefix` pour un accesseur.
    fn name_member(&mut self, func: &Value, key: &crate::value::PropertyKey, prefix: &str) {
        let Some(id) = func.as_object() else { return };
        let object = self.heap.get(id);
        let anonymous = matches!(&object.kind, Kind::Closure(c) if c.proto.name.is_empty())
            && !object.props.contains_key("name");
        if !anonymous {
            return;
        }
        let base = match key {
            crate::value::PropertyKey::String(name) => name.to_string(),
            crate::value::PropertyKey::Symbol(symbol) => symbol
                .description()
                .map_or_else(String::new, |text| format!("[{text}]")),
        };
        let name = if prefix.is_empty() {
            base
        } else {
            format!("{prefix} {base}")
        };
        self.heap.get_mut(id).props.insert(
            Arc::from("name"),
            Property {
                slot: Slot::Data(Value::string(&name)),
                writable: false,
                enumerable: false,
                configurable: true,
            },
        );
    }

    /// Contrôle l'ajout d'un membre privé à `id` : il ne doit pas déjà y
    /// figurer (sauf `merging`, la seconde moitié d'un accesseur), et l'objet
    /// doit être extensible.
    fn check_private_add(&mut self, id: ObjId, symbol: &Symbol, merging: bool) -> JsResult<()> {
        let object = self.heap.get(id);
        let name = symbol.description().unwrap_or("");
        if !merging && object.symbols.contains_key(symbol) {
            return Err(self.type_error(&format!(
                "le membre privé {name} est déjà défini sur cet objet"
            )));
        }
        if !object.extensible {
            return Err(self.type_error(&format!(
                "le membre privé {name} ne peut pas être ajouté à un objet non extensible"
            )));
        }
        Ok(())
    }

    /// Refuse de redéfinir une propriété propre non configurable (membre
    /// statique `prototype` d'une classe).
    fn check_redefinable(&mut self, id: ObjId, key: &crate::value::PropertyKey) -> JsResult<()> {
        if self.heap.get(id).prop(key).is_some_and(|p| !p.configurable) {
            return Err(self.type_error(&format!(
                "la propriété « {} » ne peut pas être redéfinie",
                key.describe()
            )));
        }
        Ok(())
    }

    /// Crée, dans le cadre courant, la fermeture de la fonction `proto` : elle
    /// capture les registres et les captures du cadre que `proto` désigne.
    fn new_closure(&mut self, proto: Arc<FunctionProto>) -> Option<ObjId> {
        let frame = self.frames.last()?;
        let (base, parent, this) = (frame.base, frame.closure, frame.this.clone());
        let scope = frame.scope;
        let mut upvalues = Vec::with_capacity(proto.upvalues.len());
        for desc in &proto.upvalues {
            let id = if desc.from_parent_register {
                self.capture(base + usize::from(desc.index))
            } else {
                match &self.heap.get(parent).kind {
                    Kind::Closure(c) => c.upvalues.get(usize::from(desc.index)).copied(),
                    _ => None,
                }
                .unwrap_or_else(|| self.heap.alloc_upvalue(Upvalue::Closed(Value::Undefined)))
            };
            upvalues.push(id);
        }
        let is_arrow = proto.kind == FunctionKind::Arrow;
        // Une fonction fléchée hérite de l'objet d'attache (pour `super`).
        let home = match (&self.heap.get(parent).kind, is_arrow) {
            (Kind::Closure(c), true) => c.home,
            _ => None,
        };
        let function_proto = match (proto.is_async, proto.is_generator) {
            (false, false) => self.intrinsics.function_proto,
            (false, true) => self.intrinsics.generator_function_proto,
            (true, false) => self.intrinsics.async_function_proto,
            (true, true) => self.intrinsics.async_generator_function_proto,
        };
        let closure = Closure {
            proto,
            upvalues,
            bound_this: is_arrow.then_some(this),
            home,
            parent: None,
            fields: None,
            scope,
        };
        Some(
            self.heap
                .alloc(Object::new(Some(function_proto), Kind::Closure(closure))),
        )
    }

    /// `eval` direct (ECMA-262 §19.2.1.1) : évalue `source` dans la portée
    /// de l'appelant. Le code est compilé comme une fonction fléchée créée
    /// dans le cadre courant, dont elle capture les variables.
    fn direct_eval(&mut self, source: &str, site: u16) -> JsResult {
        let Some(site) = self
            .frames
            .last()
            .and_then(|frame| frame.proto.eval_sites.get(usize::from(site)).cloned())
        else {
            return Ok(Value::Undefined);
        };
        let private_names: Vec<JsStr> = site
            .locals
            .iter()
            .map(|(name, ..)| name)
            .chain(site.upvalues.iter().map(|(name, ..)| name))
            .filter_map(|name| name.strip_prefix('#').map(Arc::from))
            .collect();
        let compiled = crate::parser::parse_direct_eval(
            source,
            site.strict,
            !site.top_level,
            site.in_method,
            site.in_field_init,
            &private_names,
        )
        .and_then(|script| {
            crate::compiler::compile_direct_eval(&script.body, script.strict, &site)
        });
        let proto = match compiled {
            Ok(proto) => proto,
            Err(error) => return Err(self.error("SyntaxError", &error.message)),
        };
        let Some(closure) = self.new_closure(proto) else {
            return Ok(Value::Undefined);
        };
        self.call(&Value::Object(closure), &Value::Undefined, &[])
    }

    /// `GetTemplateObject` (ECMA-262 §13.2.8.4) : tableau figé des morceaux
    /// interprétés, portant le tableau figé `raw` des morceaux bruts. Un même
    /// gabarit rend toujours le même objet.
    fn template_object(&mut self, index: u32) -> JsResult {
        let Some(proto) = self.frames.last().map(|f| Arc::clone(&f.proto)) else {
            return Ok(Value::Undefined);
        };
        let key = (Arc::as_ptr(&proto) as usize, index);
        if let Some((_, id)) = self.template_objects.get(&key) {
            return Ok(Value::Object(*id));
        }
        let Some((cooked, raw)) = proto.templates.get(index as usize) else {
            return Ok(Value::Undefined);
        };
        let cooked: Vec<Value> = cooked
            .iter()
            .map(|part| part.clone().map_or(Value::Undefined, Value::String))
            .collect();
        let raw: Vec<Value> = raw.iter().cloned().map(Value::String).collect();
        let raw = self.new_array(raw);
        let strings = self.new_array(cooked);
        let (Some(raw_id), Some(id)) = (raw.as_object(), strings.as_object()) else {
            return Ok(Value::Undefined);
        };
        crate::builtins::freeze(self, raw_id)?;
        self.heap.get_mut(id).props.insert(
            Arc::from("raw"),
            Property {
                slot: Slot::Data(raw),
                writable: false,
                enumerable: false,
                configurable: false,
            },
        );
        crate::builtins::freeze(self, id)?;
        self.template_objects.insert(key, (proto, id));
        Ok(strings)
    }

    /// `this` d'une fonction en mode non strict (ECMA-262 §10.2.1.2) :
    /// l'objet global pour `undefined` et `null`, un objet enveloppe pour une
    /// primitive.
    fn sloppy_this(&mut self, this: Value) -> Value {
        match this {
            Value::Undefined | Value::Null => Value::Object(self.global),
            Value::Object(_) | Value::Hole => this,
            primitive => self.to_object(&primitive).unwrap_or(primitive),
        }
    }

    /// Valeur de la liaison lexicale globale `name`, vue depuis la portée
    /// `scope` (une liaison propre à un module la masque). `ReferenceError`
    /// si elle n'est pas encore initialisée.
    fn global_lexical(&mut self, scope: ObjId, name: &JsStr) -> JsResult<Option<Value>> {
        let binding = match self.heap.get(self.global_lexicals).props.get(name) {
            Some(binding) => binding,
            None => return Ok(None),
        };
        let Slot::Data(value) = &binding.slot else {
            return Ok(None);
        };
        let value = value.clone();
        if scope != self.global && self.has_own(scope, name) {
            return Ok(None);
        }
        if value == Value::Hole {
            return Err(self.error(
                "ReferenceError",
                &format!("{name} est utilisé avant son initialisation"),
            ));
        }
        Ok(Some(value))
    }

    /// Vrai si le code en cours d'exécution est en mode strict.
    fn strict(&self) -> bool {
        self.frames.last().is_some_and(|f| f.proto.strict)
    }

    /// Erreur d'une écriture refusée en mode strict.
    fn rejected_write(&mut self, key: &str) -> Value {
        self.type_error(&format!(
            "impossible d'écrire la propriété « {key} » (mode strict)"
        ))
    }

    /// Objet où le code courant résout ses noms de premier niveau : la portée
    /// de son module, ou l'objet global. La portée d'un module a l'objet
    /// global pour prototype, si bien qu'une lecture y trouve aussi les globales.
    fn scope(&self) -> ObjId {
        self.frames
            .last()
            .and_then(|f| f.scope)
            .unwrap_or(self.global)
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
            Kind::Proxy(proxy) if proxy.callable => self.proxy_call(id, this, args),
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
                let saved_target =
                    std::mem::replace(&mut self.native_new_target, new_target.clone());
                let result = func(self, &Value::Undefined, args);
                self.native_new_target = saved_target;
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
            Kind::Proxy(proxy) if proxy.constructor => self.proxy_construct(id, args, new_target),
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
            Op::PrivateSymbol { dst, name } => {
                let name = self.name(name);
                let id = self.next_symbol;
                self.next_symbol = self.next_symbol.saturating_add(1);
                self.set_reg(dst, Value::Symbol(Symbol::new_private(id, name)));
            }
            Op::AsyncIterStart { dst, src } => {
                let iterator = self.async_iterator_for(&self.reg(src))?;
                self.set_reg(dst, iterator);
            }
            Op::Yield { dst, src } | Op::Await { dst, src } => {
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
                    scope: frame.scope,
                    proto: frame.proto,
                    pc: frame.pc,
                    this: frame.this,
                    new_target: frame.new_target,
                    args: frame.args,
                    handlers: frame.handlers,
                    registers,
                    upvalues,
                    resume_reg: dst,
                    awaiting: matches!(op, Op::Await { .. }),
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
            Op::ImportCall { dst, src } => {
                let specifier = self.reg(src).clone();
                let scope = self.frames.last().and_then(|f| f.scope);
                let promise = self.dynamic_import(&specifier, scope);
                self.set_reg(dst, promise);
            }
            Op::LoadCallee { dst } => {
                let value = self
                    .frames
                    .last()
                    .map_or(Value::Undefined, |f| Value::Object(f.closure));
                self.set_reg(dst, value);
            }
            Op::GetGlobal { dst, name } => {
                let (name, scope) = (self.name(name), self.scope());
                if let Some(value) = self.global_lexical(scope, &name)? {
                    self.set_reg(dst, value);
                    return Ok(None);
                }
                if !self.has_property(scope, &name) {
                    return Err(self.error("ReferenceError", &format!("{name} n'est pas défini")));
                }
                let value = self.get(&Value::Object(scope), &name)?;
                self.set_reg(dst, value);
            }
            Op::SetGlobal { name, src } => {
                let (name, value) = (self.name(name), self.reg(src));
                let scope = self.scope();
                if self.global_lexical(scope, &name)?.is_some() {
                    let lexicals = self.global_lexicals;
                    match self.heap.get_mut(lexicals).props.get_mut(&name) {
                        Some(binding) if binding.writable => binding.slot = Slot::Data(value),
                        _ => {
                            return Err(
                                self.type_error(&format!("affectation à la constante {name}"))
                            );
                        }
                    }
                    return Ok(None);
                }
                // Dans un module, affecter une variable globale existante ne
                // doit pas créer une liaison locale qui la masquerait.
                let target = if scope != self.global
                    && !self.has_own(scope, &name)
                    && self.has_property(self.global, &name)
                {
                    self.global
                } else {
                    scope
                };
                if self.strict() {
                    // Affecter une variable non déclarée est une erreur.
                    if !self.has_property(target, &name) {
                        return Err(
                            self.error("ReferenceError", &format!("{name} n'est pas défini"))
                        );
                    }
                    if !self.put(&Value::Object(target), &name, value)? {
                        return Err(self.rejected_write(&name));
                    }
                } else {
                    self.set(&Value::Object(target), &name, value)?;
                }
            }
            Op::CheckGlobalDecl { name, lexical } => {
                let name = self.name(name);
                let lexicals = self.heap.get(self.global_lexicals);
                let collides = if lexical {
                    lexicals.props.contains_key(&name)
                        || self.global_vars.contains(&name)
                        || self
                            .heap
                            .get(self.global)
                            .props
                            .get(&name)
                            .is_some_and(|p| !p.configurable)
                } else {
                    lexicals.props.contains_key(&name)
                };
                if collides {
                    return Err(self.error("SyntaxError", &format!("« {name} » est déjà déclaré")));
                }
            }
            Op::DeclareLexical { name } => {
                let name = self.name(name);
                let lexicals = self.global_lexicals;
                self.heap
                    .get_mut(lexicals)
                    .props
                    .insert(name, Property::data(Value::Hole));
            }
            Op::InitLexical {
                name,
                src,
                constant,
            } => {
                let (name, value) = (self.name(name), self.reg(src));
                let lexicals = self.global_lexicals;
                self.heap.get_mut(lexicals).props.insert(
                    name,
                    Property {
                        slot: Slot::Data(value),
                        writable: !constant,
                        enumerable: true,
                        configurable: false,
                    },
                );
            }
            Op::InitGlobal {
                name,
                src,
                declared,
            } => {
                let (name, value) = (self.name(name), self.reg(src));
                let scope = self.scope();
                if declared && scope == self.global {
                    self.global_vars.insert(Arc::clone(&name));
                }
                if value == Value::Hole {
                    // Déclaration `var` : ne remplace pas une variable existante.
                    if !self.heap.get(scope).props.contains_key(&name) {
                        if !self.heap.get(scope).extensible {
                            return Err(self.type_error(&format!(
                                "impossible de déclarer « {name} » : l'objet global n'est pas extensible"
                            )));
                        }
                        // Une variable de script ne se supprime pas ; celle
                        // que crée un `eval`, si.
                        self.heap.get_mut(scope).props.insert(
                            name,
                            Property {
                                configurable: !declared,
                                ..Property::data(Value::Undefined)
                            },
                        );
                    }
                } else {
                    self.heap
                        .get_mut(scope)
                        .props
                        .insert(name, Property::data(value));
                }
            }
            Op::TypeofGlobal { dst, name } => {
                let (name, scope) = (self.name(name), self.scope());
                let value = if let Some(value) = self.global_lexical(scope, &name)? {
                    value
                } else if self.has_property(scope, &name) {
                    self.get(&Value::Object(scope), &name)?
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
            Op::DelegateStart { base, src } => {
                let source = self.reg(src);
                let key = crate::value::PropertyKey::Symbol(self.well_known.iterator.clone());
                let method = self.get_key(&source, &key)?;
                if !crate::builtins::is_callable(self, &method) {
                    let shown = self.display(&source);
                    return Err(self.type_error(&format!("{shown} n'est pas itérable")));
                }
                let iterator = self.call(&method, &source, &[])?;
                if iterator.as_object().is_none() {
                    return Err(self.type_error("Symbol.iterator doit renvoyer un objet itérateur"));
                }
                let next = self.get(&iterator, "next")?;
                self.set_reg(base, iterator);
                self.set_reg(base + 1, next);
            }
            Op::DelegateStartAsync { base, src } => {
                let iterator = self.async_iterator_for(&self.reg(src))?;
                let next = self.get(&iterator, "next")?;
                self.set_reg(base, iterator);
                self.set_reg(base + 1, next);
            }
            Op::DelegateStep { base } => {
                let (iterator, next) = (self.reg(base), self.reg(base + 1));
                let (received, mode) = (self.reg(base + 2), self.reg(base + 3));
                let (result, absent) = self.delegate_call(&iterator, &next, received, &mode)?;
                let (value, done, returning) = if absent {
                    (result, true, true)
                } else {
                    self.delegate_finish(&result, &mode)?
                };
                self.set_reg(base + 4, value);
                self.set_reg(base + 5, Value::Bool(done));
                self.set_reg(base + 6, Value::Bool(returning));
            }
            Op::DelegateCall { base } => {
                let (iterator, next) = (self.reg(base), self.reg(base + 1));
                let (received, mode) = (self.reg(base + 2), self.reg(base + 3));
                let (result, absent) = self.delegate_call(&iterator, &next, received, &mode)?;
                self.set_reg(base + 4, result);
                self.set_reg(base + 5, Value::Bool(absent));
                self.set_reg(base + 6, Value::Bool(absent));
            }
            Op::DelegateFinish { base } => {
                let (result, mode) = (self.reg(base + 4), self.reg(base + 3));
                let (value, done, returning) = self.delegate_finish(&result, &mode)?;
                self.set_reg(base + 4, value);
                self.set_reg(base + 5, Value::Bool(done));
                self.set_reg(base + 6, Value::Bool(returning));
            }
            Op::InitGlobalFunction {
                name,
                src,
                declared,
            } => {
                let (name, value) = (self.name(name), self.reg(src));
                let scope = self.scope();
                if declared && scope == self.global {
                    self.global_vars.insert(Arc::clone(&name));
                }
                // `CreateGlobalFunctionBinding` : une propriété non
                // configurable garde ses attributs, seule sa valeur change.
                let object = self.heap.get_mut(scope);
                match object.props.get_mut(&name) {
                    Some(existing) if !existing.configurable => {
                        existing.slot = Slot::Data(value);
                    }
                    _ => {
                        object.props.insert(
                            name,
                            Property {
                                configurable: !declared,
                                ..Property::data(value)
                            },
                        );
                    }
                }
            }
            Op::DefineVar { obj, name } => {
                let name = self.name(name);
                if let Some(id) = self.reg(obj).as_object() {
                    self.heap
                        .get_mut(id)
                        .props
                        .entry(name)
                        .or_insert_with(|| Property::data(Value::Undefined));
                }
            }
            Op::CheckGlobalFunction { name } => {
                let name = self.name(name);
                let scope = self.scope();
                let object = self.heap.get(scope);
                let allowed = match object.props.get(&name) {
                    None => object.extensible,
                    Some(existing) if existing.configurable => true,
                    Some(existing) => {
                        matches!(existing.slot, Slot::Data(_))
                            && existing.writable
                            && existing.enumerable
                    }
                };
                if !allowed {
                    return Err(self.type_error(&format!(
                        "impossible de déclarer la fonction globale « {name} »"
                    )));
                }
            }
            Op::TemplateObject { dst, index } => {
                let object = self.template_object(index)?;
                self.set_reg(dst, object);
            }
            Op::ToObject { dst, src } => {
                let object = self.to_object(&self.reg(src))?;
                self.set_reg(dst, object);
            }
            Op::WithHas { dst, obj, name } => {
                let (object, name) = (self.reg(obj), self.name(name));
                let mut found = false;
                if let Some(id) = object.as_object() {
                    let key = crate::value::PropertyKey::String(Arc::clone(&name));
                    found = self.has_property_op(id, &key)?;
                    if found {
                        let unscopables =
                            crate::value::PropertyKey::Symbol(self.well_known.unscopables.clone());
                        let blocked = self.get_key(&object, &unscopables)?;
                        if blocked.as_object().is_some() {
                            found = !self.get(&blocked, &name)?.truthy();
                        }
                    }
                }
                self.set_reg(dst, Value::Bool(found));
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
                if !self.put(&target, &name, value)? && self.strict() {
                    return Err(self.rejected_write(&name));
                }
            }
            Op::GetElem { dst, obj, key } => {
                let (target, key) = (self.reg(obj), self.reg(key));
                let value = self.get_element(&target, &key)?;
                self.set_reg(dst, value);
            }
            Op::SetElem { obj, key, src } => {
                let (target, key, value) = (self.reg(obj), self.reg(key), self.reg(src));
                if !self.put_element(&target, &key, value)? && self.strict() {
                    let shown = self.display(&key);
                    return Err(self.rejected_write(&shown));
                }
            }
            Op::DeleteProp { dst, obj, name } => {
                let (target, name) = (self.reg(obj), self.name(name));
                let deleted = self.delete(&target, &name)?;
                if !deleted && self.strict() {
                    return Err(self.type_error(&format!(
                        "impossible de supprimer la propriété « {name} » (mode strict)"
                    )));
                }
                self.set_reg(dst, Value::Bool(deleted));
            }
            Op::DeleteElem { dst, obj, key } => {
                let target = self.reg(obj);
                let key = self.to_key(&self.reg(key))?;
                let deleted = self.delete_key(&target, &key)?;
                if !deleted && self.strict() {
                    return Err(self.type_error(&format!(
                        "impossible de supprimer la propriété « {} » (mode strict)",
                        key.describe()
                    )));
                }
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
            Op::SetSuper { key, src } => {
                let key = self.to_key(&self.reg(key))?;
                let value = self.reg(src);
                let (home, this, strict) = match self.frames.last() {
                    Some(f) => match &self.heap.get(f.closure).kind {
                        Kind::Closure(c) => (c.home, f.this.clone(), c.proto.strict),
                        _ => (None, Value::Undefined, true),
                    },
                    None => (None, Value::Undefined, true),
                };
                let Some(parent) = home.and_then(|h| self.heap.get(h).proto) else {
                    return Err(self.type_error("« super » n'a pas de prototype où écrire"));
                };
                if !self.set_with_receiver(parent, &key, value, &this)? && strict {
                    return Err(self.type_error(&format!(
                        "la propriété « {} » ne peut pas être écrite",
                        key.describe()
                    )));
                }
            }
            Op::KeyOnce { obj, key } => {
                let base = self.reg(obj);
                if base.is_nullish() {
                    let shown = self.display(&base);
                    return Err(
                        self.type_error(&format!("impossible de lire une propriété de {shown}"))
                    );
                }
                let current = self.reg(key);
                if current.as_object().is_some() {
                    let converted = self.to_key(&current)?;
                    self.set_reg(key, converted.to_value());
                }
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
            Op::SetProto { obj, src } => {
                let proto = match self.reg(src) {
                    Value::Object(proto) => Some(Some(proto)),
                    Value::Null => Some(None),
                    _ => None,
                };
                if let (Some(id), Some(proto)) = (self.reg(obj).as_object(), proto) {
                    self.heap.get_mut(id).proto = proto;
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
                self.name_member(&value, &key, "");
                if let Some(id) = self.reg(obj).as_object() {
                    self.check_redefinable(id, &key)?;
                    // Une méthode privée ne se remplace pas.
                    let private =
                        matches!(&key, crate::value::PropertyKey::Symbol(s) if s.is_private());
                    self.heap.get_mut(id).insert_prop(
                        key,
                        Property {
                            writable: !private,
                            ..Property::hidden(value)
                        },
                    );
                }
            }
            Op::DefineField { obj, key, src } => {
                let key = self.to_key(&self.reg(key))?;
                let value = self.reg(src);
                if let Some(id) = self.reg(obj).as_object() {
                    if let crate::value::PropertyKey::Symbol(symbol) = &key
                        && symbol.is_private()
                    {
                        self.check_private_add(id, symbol, false)?;
                        self.heap
                            .get_mut(id)
                            .symbols
                            .insert(symbol.clone(), Property::data(value));
                    } else if !self.create_data_property(id, &key, value)? {
                        return Err(self.type_error(&format!(
                            "le champ « {} » ne peut pas être défini",
                            key.describe()
                        )));
                    }
                }
            }
            Op::InitPrivate {
                obj,
                key,
                func,
                kind,
            } => {
                let key = self.to_key(&self.reg(key))?;
                let function = self.reg(func);
                if let (Some(id), crate::value::PropertyKey::Symbol(symbol)) =
                    (self.reg(obj).as_object(), key)
                {
                    let accessor = function.as_object();
                    let existing = self
                        .heap
                        .get(id)
                        .symbols
                        .get(&symbol)
                        .map(|p| p.slot.clone());
                    let slot = match (kind, existing) {
                        (0, None) => Slot::Data(function),
                        (1, None) => Slot::Accessor {
                            get: accessor,
                            set: None,
                        },
                        (2, None) => Slot::Accessor {
                            get: None,
                            set: accessor,
                        },
                        (1, Some(Slot::Accessor { get: None, set })) => {
                            Slot::Accessor { get: accessor, set }
                        }
                        (2, Some(Slot::Accessor { get, set: None })) => {
                            Slot::Accessor { get, set: accessor }
                        }
                        _ => {
                            self.check_private_add(id, &symbol, false)?;
                            return Ok(None);
                        }
                    };
                    self.check_private_add(id, &symbol, true)?;
                    self.heap.get_mut(id).symbols.insert(
                        symbol,
                        Property {
                            slot,
                            writable: false,
                            enumerable: false,
                            configurable: false,
                        },
                    );
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
                let function = self.reg(func);
                self.name_member(&function, &key, if is_getter { "get" } else { "set" });
                if let Some(id) = self.reg(obj).as_object() {
                    self.check_redefinable(id, &key)?;
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
                // `ToNumeric` : un BigInt reste un BigInt (`x++`).
                let value = self.to_numeric(&self.reg(src))?;
                self.set_reg(dst, value);
            }
            Op::Step {
                dst,
                src,
                increment,
            } => {
                let value = match self.reg(src) {
                    Value::BigInt(big) => {
                        let one = crate::bigint::BigInt::from_i64(if increment { 1 } else { -1 });
                        match big.add(&one) {
                            Ok(result) => Value::BigInt(Arc::new(result)),
                            Err(_) => return Err(self.error("RangeError", "BigInt trop grand")),
                        }
                    }
                    other => {
                        let n = self.to_number(&other)?;
                        Value::Number(if increment { n + 1.0 } else { n - 1.0 })
                    }
                };
                self.set_reg(dst, value);
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
                let proto = self
                    .frames
                    .last()
                    .and_then(|frame| frame.proto.functions.get(index as usize).cloned());
                let Some(id) = proto.and_then(|proto| self.new_closure(proto)) else {
                    return Ok(None);
                };
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
            Op::DirectEval {
                dst,
                callee,
                first,
                argc,
                site,
            } => {
                let callee = self.reg(callee);
                let args: Vec<Value> = (0..argc).map(|i| self.reg(first + i)).collect();
                self.check_callable(&callee)?;
                let is_eval =
                    callee.as_object().is_some() && callee.as_object() == self.eval_function;
                if !is_eval {
                    self.call_from_loop(dst, &callee, Value::Undefined, args)?;
                    return Ok(None);
                }
                let result = match args.into_iter().next() {
                    Some(Value::String(source)) => self.direct_eval(&source, site)?,
                    Some(other) => other,
                    None => Value::Undefined,
                };
                self.set_reg(dst, result);
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
                let (parent, fields) = match &self.heap.get(closure).kind {
                    Kind::Closure(c) => (c.parent, c.fields),
                    _ => (None, None),
                };
                let Some(parent) = parent else {
                    return Err(self.type_error("« super() » hors d'une classe dérivée"));
                };
                let instance = self.construct(&Value::Object(parent), &args, &new_target)?;
                // `this` n'est lié qu'une fois, après le retour du constructeur parent.
                if self.frames.last().is_some_and(|f| f.this != Value::Hole) {
                    return Err(self.error("ReferenceError", "« super() » a déjà été appelé"));
                }
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
                let keys = self.for_in_keys(&self.reg(obj))?;
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
            Op::IterClose { iter, quiet } => {
                let iterator = self.reg(iter);
                self.iterator_close(&iterator, quiet)?;
            }
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
                let (args, callee) = match self.frames.last() {
                    Some(f) => (f.args.clone(), (!f.proto.strict).then_some(f.closure)),
                    None => (Vec::new(), None),
                };
                // LIMITE : les éléments ne sont pas liés aux paramètres.
                let object = self.heap.alloc(Object::new(
                    Some(self.intrinsics.object_proto),
                    Kind::Arguments,
                ));
                #[expect(clippy::cast_precision_loss, reason = "nombre d'arguments petit")]
                let length = Value::Number(args.len() as f64);
                let props = &mut self.heap.get_mut(object).props;
                for (index, value) in args.into_iter().enumerate() {
                    props.insert(Arc::from(index.to_string()), Property::data(value));
                }
                props.insert(Arc::from("length"), Property::hidden(length));
                if let Some(callee) = callee {
                    props.insert(Arc::from("callee"), Property::hidden(Value::Object(callee)));
                }
                let values = self.get(&Value::Object(self.intrinsics.array_proto), "values")?;
                let iterator = self.well_known.iterator.clone();
                self.define_hidden_symbol(object, &iterator, values);
                self.set_reg(dst, Value::Object(object));
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
                        proto_parent = match self.get(value, "prototype")? {
                            Value::Object(id) => Some(id),
                            Value::Null => None,
                            _ => {
                                return Err(self.type_error(
                                    "le prototype de la classe mère n'est ni un objet ni null",
                                ));
                            }
                        };
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
            let derived = frame.proto.kind == (FunctionKind::ClassConstructor { derived: true });
            if derived && value != Value::Undefined {
                return Err(self.type_error(
                    "le constructeur d'une classe dérivée ne peut rendre qu'un objet ou undefined",
                ));
            }
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
        self.run_top_level(proto, None, Value::Object(self.global))
    }

    /// Exécute du code de premier niveau dans la portée `scope` (celle d'un
    /// module, ou `None` pour l'objet global), avec `this` pour receveur.
    pub(crate) fn run_top_level(
        &mut self,
        proto: Arc<FunctionProto>,
        scope: Option<ObjId>,
        this: Value,
    ) -> JsResult {
        let is_async = proto.is_async;
        let closure = Closure {
            proto,
            upvalues: Vec::new(),
            bound_this: None,
            home: None,
            parent: None,
            fields: None,
            scope,
        };
        let id = self.heap.alloc(Object::new(
            Some(self.intrinsics.function_proto),
            Kind::Closure(closure),
        ));
        if is_async {
            // Corps d'un module avec `await` : rend la promesse de sa fin.
            return self.start_coroutine(id, this, Vec::new());
        }
        let depth = self.frames.len();
        self.push_frame(id, this, Vec::new(), Value::Undefined, None, false)?;
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
        let sloppy = matches!(
            &self.heap.get(closure).kind,
            Kind::Closure(c) if !c.proto.strict && c.bound_this.is_none()
        );
        let this = if sloppy { self.sloppy_this(this) } else { this };
        let kind = match &self.heap.get(closure).kind {
            Kind::Closure(c) if c.proto.is_async && c.proto.is_generator => CoKind::AsyncGenerator,
            Kind::Closure(c) if c.proto.is_async => CoKind::Async,
            _ => CoKind::Generator,
        };
        let state = CoState::Start {
            closure,
            this,
            args,
        };
        let coroutine = |promise| Coroutine {
            state,
            kind,
            promise,
            queue: std::collections::VecDeque::new(),
            busy: false,
        };
        if kind != CoKind::Async {
            let proto = if kind == CoKind::Generator {
                self.intrinsics.generator_proto
            } else {
                self.intrinsics.async_generator_proto
            };
            let generator = self.heap.alloc(Object::new(
                Some(proto),
                Kind::Coroutine(Box::new(coroutine(None))),
            ));
            // Les paramètres sont liés dès l'appel : le corps s'exécute
            // jusqu'au `yield` initial que le compilateur place après eux, et
            // une erreur levée par un paramètre remonte à l'appelant.
            self.enter_native()?;
            // Le prototype de l'objet est lu ensuite : `prototype` de la
            // fonction, tel que les paramètres l'ont peut-être modifié.
            let started = self
                .resume(generator, Resume::Next(Value::Undefined))
                .and_then(|_| self.get(&Value::Object(closure), "prototype"));
            self.native_depth -= 1;
            if let Some(proto) = started?.as_object() {
                self.heap.get_mut(generator).proto = Some(proto);
            }
            return Ok(Value::Object(generator));
        }
        let promise = self.new_promise();
        let coroutine = self.heap.alloc(Object::new(
            None,
            Kind::Coroutine(Box::new(coroutine(Some(promise)))),
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
                let mut saved = *saved;
                // `return(v)` reprend le corps sur le chemin de retour que le
                // compilateur place après chaque `yield` (un saut l'enjambe en
                // reprise normale) : les blocs `finally` s'y exécutent.
                let return_path = !saved.awaiting
                    && matches!(saved.proto.code.get(saved.pc), Some(Op::Jump { .. }));
                let mode = match mode {
                    Resume::Return(value) if return_path => {
                        saved.pc += 1;
                        Resume::Next(value)
                    }
                    Resume::Return(value) => {
                        self.set_coroutine_state(coroutine, CoState::Done);
                        return Ok(CoOutcome::Return(value));
                    }
                    other => other,
                };
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
                    scope: saved.scope,
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
        // `Some(true)` : suspendue par `await` ; `Some(false)` : par `yield`.
        let suspended = match &self.heap.get(coroutine).kind {
            Kind::Coroutine(c) => match &c.state {
                CoState::Suspended(frame) => Some(frame.awaiting),
                _ => None,
            },
            _ => None,
        };
        match result {
            Ok(value) if suspended == Some(true) => Ok(CoOutcome::Await(value)),
            Ok(value) if suspended == Some(false) => Ok(CoOutcome::Yield(value)),
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
            Kind::Coroutine(c) if c.kind == CoKind::Generator => Some(id),
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
