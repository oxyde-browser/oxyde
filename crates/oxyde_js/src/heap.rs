//! Tas d'objets et ramasse-miettes traçant (marquage puis balayage).
//!
//! Le tas est une arène : les objets vivent dans un `Vec` et sont désignés
//! par leur index ([`ObjId`]). Ce choix garde le moteur en Rust sûr, sans
//! aucun pointeur brut. Les emplacements libérés sont réutilisés.
//!
//! INVARIANT : un `ObjId` atteignable depuis les racines désigne toujours un
//! emplacement occupé. Un identifiant périmé ne peut pas provoquer d'accès
//! mémoire invalide : il désigne au pire un autre objet ou l'objet vide.

use std::sync::Arc;

use indexmap::IndexMap;

use crate::bytecode::{FunctionProto, Reg};
use crate::value::{JsStr, ObjId, PropertyKey, Symbol, Value};
use crate::vm::{NativeDataFn, NativeFn, Resume};

/// Référence à une cellule de variable capturée par une fermeture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct UpId(pub(crate) u32);

/// Variable capturée : encore dans la pile de registres, ou déplacée dans le tas.
#[derive(Clone, Debug)]
pub(crate) enum Upvalue {
    /// Index absolu dans la pile de registres.
    Open(usize),
    Closed(Value),
}

/// Contenu d'une propriété.
#[derive(Clone, Debug)]
pub(crate) enum Slot {
    Data(Value),
    Accessor {
        get: Option<ObjId>,
        set: Option<ObjId>,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct Property {
    pub(crate) slot: Slot,
    pub(crate) writable: bool,
    pub(crate) enumerable: bool,
    pub(crate) configurable: bool,
}

impl Property {
    /// Propriété de données ordinaire (créée par affectation).
    pub(crate) fn data(value: Value) -> Self {
        Self {
            slot: Slot::Data(value),
            writable: true,
            enumerable: true,
            configurable: true,
        }
    }

    /// Propriété de données non énumérable (méthodes des objets natifs).
    pub(crate) fn hidden(value: Value) -> Self {
        Self {
            slot: Slot::Data(value),
            writable: true,
            enumerable: false,
            configurable: true,
        }
    }
}

/// Clé normalisée de `Map` et `Set` (égalité « SameValueZero »).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum MapKey {
    Undefined,
    Null,
    Bool(bool),
    /// Bits du nombre, `-0` ramené à `+0` et `NaN` canonique.
    Number(u64),
    String(JsStr),
    Symbol(Symbol),
    BigInt(Arc<crate::bigint::BigInt>),
    Object(ObjId),
}

impl MapKey {
    pub(crate) fn from_value(value: &Value) -> Self {
        match value {
            Value::Undefined | Value::Hole => Self::Undefined,
            Value::Null => Self::Null,
            Value::Bool(b) => Self::Bool(*b),
            Value::Number(n) => {
                let normalized = if *n == 0.0 {
                    0.0
                } else if n.is_nan() {
                    f64::NAN
                } else {
                    *n
                };
                Self::Number(normalized.to_bits())
            }
            Value::String(s) => Self::String(Arc::clone(s)),
            Value::Symbol(s) => Self::Symbol(s.clone()),
            Value::BigInt(b) => Self::BigInt(Arc::clone(b)),
            Value::Object(id) => Self::Object(*id),
        }
    }
}

/// Fonction JavaScript compilée, avec les variables qu'elle capture.
#[derive(Clone, Debug)]
pub(crate) struct Closure {
    pub(crate) proto: Arc<FunctionProto>,
    pub(crate) upvalues: Vec<UpId>,
    /// `this` lexical d'une fonction fléchée.
    pub(crate) bound_this: Option<Value>,
    /// Objet d'attache d'une méthode, pour résoudre `super`.
    pub(crate) home: Option<ObjId>,
    /// Constructeur de la classe mère (`super(...)`).
    pub(crate) parent: Option<ObjId>,
    /// Fonction d'initialisation des champs d'instance de la classe.
    pub(crate) fields: Option<ObjId>,
    /// Portée de premier niveau où la fonction résout ses noms libres :
    /// celle de son module, ou `None` pour l'objet global.
    pub(crate) scope: Option<ObjId>,
}

/// État d'un itérateur interne (`for…in`, `for…of`).
#[derive(Clone, Debug)]
pub(crate) enum IterState {
    /// Clés énumérables d'un objet, figées au début de la boucle.
    Keys { keys: Vec<JsStr>, index: usize },
    /// Éléments d'un tableau, relus à chaque tour.
    Array { target: ObjId, index: usize },
    /// Valeurs figées (chaînes, `Map`, `Set`, itérables convertis).
    Values { values: Vec<Value>, index: usize },
    /// Objet respectant le protocole d'itération (`next()` → `{ value, done }`).
    /// `done` : épuisé, ou fermé ; plus aucun appel ne lui sera fait.
    Protocol {
        iterator: Value,
        next: Value,
        done: bool,
    },
    /// Propriétés indexées d'un objet de type tableau, relues à chaque tour
    /// (`Array.prototype.keys`, `values`, `entries`).
    Indexed {
        target: Value,
        index: usize,
        kind: IndexedKind,
    },
}

/// Ce que rend un itérateur de tableau.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IndexedKind {
    Keys,
    Values,
    Entries,
}

/// Cadre d'exécution d'une coroutine suspendue, sorti de la pile.
#[derive(Clone, Debug)]
pub(crate) struct SavedFrame {
    pub(crate) closure: ObjId,
    pub(crate) scope: Option<ObjId>,
    pub(crate) proto: Arc<FunctionProto>,
    pub(crate) pc: usize,
    pub(crate) this: Value,
    pub(crate) new_target: Value,
    pub(crate) args: Vec<Value>,
    pub(crate) handlers: Vec<(u32, Reg)>,
    /// Fenêtre de registres au moment de la suspension.
    pub(crate) registers: Vec<Value>,
    /// Variables capturées qui étaient encore dans la pile : (registre, cellule).
    /// Leur valeur vit dans la cellule pendant la suspension.
    pub(crate) upvalues: Vec<(usize, UpId)>,
    /// Registre qui reçoit la valeur transmise à la reprise.
    pub(crate) resume_reg: Reg,
    /// Vrai si la suspension vient d'un `await` (et non d'un `yield`).
    pub(crate) awaiting: bool,
}

/// Nature d'une coroutine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CoKind {
    /// `function*`
    Generator,
    /// `async function`
    Async,
    /// `async function*`
    AsyncGenerator,
}

/// Appel à `next`, `return` ou `throw` d'un générateur async, en attente.
#[derive(Clone, Debug)]
pub(crate) struct AsyncRequest {
    /// Reprise demandée ; `None` une fois transmise au générateur.
    pub(crate) mode: Option<Resume>,
    /// Promesse rendue à l'appelant.
    pub(crate) promise: ObjId,
}

/// État d'une coroutine (générateur ou fonction async).
#[derive(Clone, Debug)]
pub(crate) enum CoState {
    /// Créée mais pas encore démarrée.
    Start {
        closure: ObjId,
        this: Value,
        args: Vec<Value>,
    },
    Suspended(Box<SavedFrame>),
    Running,
    Done,
}

#[derive(Clone, Debug)]
pub(crate) struct Coroutine {
    pub(crate) state: CoState,
    pub(crate) kind: CoKind,
    /// Promesse rendue par l'appel d'une fonction async.
    pub(crate) promise: Option<ObjId>,
    /// Requêtes d'un générateur async, la plus ancienne en tête.
    pub(crate) queue: std::collections::VecDeque<AsyncRequest>,
    /// Générateur async en cours d'exécution ou en attente d'une promesse.
    pub(crate) busy: bool,
}

#[derive(Clone, Debug)]
pub(crate) enum PromiseState {
    Pending,
    Fulfilled(Value),
    Rejected(Value),
}

/// Réaction enregistrée par `then` sur une promesse en attente.
#[derive(Clone, Debug)]
pub(crate) struct Reaction {
    pub(crate) on_fulfilled: Value,
    pub(crate) on_rejected: Value,
    /// Promesse dérivée, résolue avec le résultat du gestionnaire.
    pub(crate) target: Option<ObjId>,
}

#[derive(Clone, Debug)]
pub(crate) struct PromiseData {
    pub(crate) state: PromiseState,
    pub(crate) reactions: Vec<Reaction>,
    /// Vrai dès qu'un gestionnaire a été attaché (rejets non gérés).
    pub(crate) handled: bool,
}

/// Octets d'un `ArrayBuffer`.
#[derive(Clone, Debug)]
pub(crate) struct BufferData {
    pub(crate) bytes: Vec<u8>,
    /// Tampon détaché : il n'a plus d'octets et ses vues sont vides.
    pub(crate) detached: bool,
    /// Taille maximale d'un tampon redimensionnable ; `None` : taille fixe.
    pub(crate) max_len: Option<usize>,
}

/// Type des éléments d'un tableau typé.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ElementKind {
    Int8,
    Uint8,
    Uint8Clamped,
    Int16,
    Uint16,
    Int32,
    Uint32,
    Float16,
    Float32,
    Float64,
    BigInt64,
    BigUint64,
}

/// Vue typée sur un tampon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TypedArrayData {
    pub(crate) buffer: ObjId,
    pub(crate) kind: ElementKind,
    /// Décalage du premier élément, en octets.
    pub(crate) offset: usize,
    /// Nombre d'éléments ; `None` : la vue suit la longueur du tampon.
    pub(crate) length: Option<usize>,
}

/// Nature d'un objet (emplacements internes).
#[derive(Clone, Debug)]
pub(crate) enum Kind {
    Ordinary,
    Array(Vec<Value>),
    Closure(Closure),
    Native {
        func: NativeFn,
        constructor: bool,
    },
    /// Fonction native accompagnée de valeurs capturées.
    NativeData {
        func: NativeDataFn,
        data: Vec<Value>,
    },
    Coroutine(Box<Coroutine>),
    Promise(Box<PromiseData>),
    /// Date : valeur temporelle en millisecondes depuis 1970, ou `NaN`.
    Date(f64),
    ArrayBuffer(Box<BufferData>),
    TypedArray(TypedArrayData),
    /// Vue non typée sur un tampon ; `length` : `None` si elle suit sa longueur.
    DataView {
        buffer: ObjId,
        offset: usize,
        length: Option<usize>,
    },
    /// Expression rationnelle compilée, avec le texte de son motif.
    RegExp {
        regex: Arc<crate::regexp::Regex>,
        source: JsStr,
    },
    Bound {
        target: ObjId,
        this: Value,
        args: Vec<Value>,
    },
    Error,
    /// Objet enveloppe de `Boolean`, `Number` ou `String`.
    Primitive(Value),
    Map(IndexMap<MapKey, (Value, Value)>),
    Set(IndexMap<MapKey, Value>),
    Iterator(IterState),
    /// Objet de l'hôte (nœud du DOM…) : classe et donnée opaques pour le moteur.
    Host {
        class: u32,
        data: u64,
    },
    /// Objet exotique `Proxy`.
    Proxy(ProxyData),
    /// Objet `arguments` : ses éléments et sa longueur sont des propriétés
    /// ordinaires.
    Arguments,
    /// Objet rendu par une aide d'itérateur (`map`, `filter`, `Iterator.from`…).
    IteratorHelper(Box<IteratorHelper>),
}

/// Nature d'une aide d'itérateur.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HelperKind {
    Map,
    Filter,
    Take,
    Drop,
    FlatMap,
    /// `Iterator.concat` : parcourt `sources` l'une après l'autre.
    Concat,
    /// Enveloppe de `Iterator.from` autour d'un itérateur étranger.
    Wrap,
    /// Itérateur de `String.prototype.matchAll` : `iterator` est
    /// l'expression, `next` le texte, `func` le drapeau global.
    RegExpString,
}

/// État d'une aide d'itérateur.
#[derive(Clone, Debug)]
pub(crate) struct IteratorHelper {
    pub(crate) kind: HelperKind,
    /// Itérateur sous-jacent et sa méthode `next`.
    pub(crate) iterator: Value,
    pub(crate) next: Value,
    /// Fonction de rappel (`map`, `filter`, `flatMap`).
    pub(crate) func: Value,
    /// Nombre de valeurs déjà passées à la fonction de rappel.
    pub(crate) counter: f64,
    /// Valeurs restant à prendre (`take`) ou à sauter (`drop`).
    pub(crate) remaining: f64,
    /// Itérateur interne en cours (`flatMap`, `concat`) et son `next`.
    pub(crate) inner: Option<(Value, Value)>,
    /// `concat` : (méthode d'itération, itérable) restant à parcourir.
    pub(crate) sources: Vec<(Value, Value)>,
    pub(crate) started: bool,
    pub(crate) running: bool,
    pub(crate) done: bool,
}

/// Cible et gestionnaire d'un `Proxy` ; `None` une fois le proxy révoqué.
#[derive(Clone, Debug)]
pub(crate) struct ProxyData {
    pub(crate) target: Option<ObjId>,
    pub(crate) handler: Option<ObjId>,
    /// La cible était appelable à la création.
    pub(crate) callable: bool,
    /// La cible était un constructeur à la création.
    pub(crate) constructor: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct Object {
    pub(crate) proto: Option<ObjId>,
    pub(crate) props: IndexMap<JsStr, Property>,
    /// Propriétés dont la clé est un symbole (vide pour la plupart des objets).
    pub(crate) symbols: IndexMap<Symbol, Property>,
    pub(crate) kind: Kind,
    pub(crate) extensible: bool,
    /// Propriétés virtuelles d'une fonction (`name`, `length`) supprimées ;
    /// voir [`crate::internal::DELETED_NAME`].
    pub(crate) deleted_virtual: u8,
}

impl Object {
    pub(crate) fn new(proto: Option<ObjId>, kind: Kind) -> Self {
        Self {
            proto,
            props: IndexMap::new(),
            symbols: IndexMap::new(),
            kind,
            extensible: true,
            deleted_virtual: 0,
        }
    }

    /// Propriété propre désignée par `key`.
    pub(crate) fn prop(&self, key: &PropertyKey) -> Option<&Property> {
        match key {
            PropertyKey::String(s) => self.props.get(s),
            PropertyKey::Symbol(s) => self.symbols.get(s),
        }
    }

    /// Crée ou remplace la propriété propre `key`.
    pub(crate) fn insert_prop(&mut self, key: PropertyKey, property: Property) {
        match key {
            PropertyKey::String(s) => {
                self.props.insert(s, property);
            }
            PropertyKey::Symbol(s) => {
                self.symbols.insert(s, property);
            }
        }
    }

    pub(crate) fn is_callable(&self) -> bool {
        matches!(
            self.kind,
            Kind::Closure(_)
                | Kind::Native { .. }
                | Kind::NativeData { .. }
                | Kind::Bound { .. }
                | Kind::Proxy(ProxyData { callable: true, .. })
        )
    }
}

/// Nombre d'allocations entre deux passages du ramasse-miettes, au minimum.
const GC_MIN_THRESHOLD: usize = 4096;

pub(crate) struct Heap {
    objects: Vec<Option<Object>>,
    free: Vec<u32>,
    upvalues: Vec<Option<Upvalue>>,
    free_upvalues: Vec<u32>,
    /// Objet renvoyé en lecture pour un identifiant périmé ; jamais modifié.
    empty: Object,
    /// Reçoit les écritures faites à travers un identifiant périmé.
    scratch: Object,
    allocated_since_gc: usize,
    threshold: usize,
    /// Nombre de passages du ramasse-miettes (pour les tests et le diagnostic).
    pub(crate) collections: usize,
}

impl Heap {
    pub(crate) fn new() -> Self {
        Self {
            objects: Vec::new(),
            free: Vec::new(),
            upvalues: Vec::new(),
            free_upvalues: Vec::new(),
            empty: Object::new(None, Kind::Ordinary),
            scratch: Object::new(None, Kind::Ordinary),
            allocated_since_gc: 0,
            threshold: GC_MIN_THRESHOLD,
            collections: 0,
        }
    }

    pub(crate) fn alloc(&mut self, object: Object) -> ObjId {
        self.allocated_since_gc += 1;
        if let Some(index) = self.free.pop()
            && let Some(slot) = self.objects.get_mut(index as usize)
        {
            *slot = Some(object);
            return ObjId(index);
        }
        let index = u32::try_from(self.objects.len()).unwrap_or(u32::MAX);
        self.objects.push(Some(object));
        ObjId(index)
    }

    pub(crate) fn get(&self, id: ObjId) -> &Object {
        self.objects
            .get(id.0 as usize)
            .and_then(Option::as_ref)
            .unwrap_or(&self.empty)
    }

    pub(crate) fn get_mut(&mut self, id: ObjId) -> &mut Object {
        match self.objects.get_mut(id.0 as usize) {
            Some(Some(object)) => object,
            // Identifiant périmé : les écritures partent dans un objet jetable, remis à neuf.
            _ => {
                self.scratch = Object::new(None, Kind::Ordinary);
                &mut self.scratch
            }
        }
    }

    /// Nombre d'objets vivants.
    pub(crate) fn live_objects(&self) -> usize {
        self.objects.len() - self.free.len()
    }

    pub(crate) fn alloc_upvalue(&mut self, upvalue: Upvalue) -> UpId {
        if let Some(index) = self.free_upvalues.pop()
            && let Some(slot) = self.upvalues.get_mut(index as usize)
        {
            *slot = Some(upvalue);
            return UpId(index);
        }
        let index = u32::try_from(self.upvalues.len()).unwrap_or(u32::MAX);
        self.upvalues.push(Some(upvalue));
        UpId(index)
    }

    pub(crate) fn upvalue(&self, id: UpId) -> Option<&Upvalue> {
        self.upvalues.get(id.0 as usize).and_then(Option::as_ref)
    }

    pub(crate) fn upvalue_mut(&mut self, id: UpId) -> Option<&mut Upvalue> {
        self.upvalues
            .get_mut(id.0 as usize)
            .and_then(Option::as_mut)
    }

    /// Vrai si assez d'objets ont été alloués pour justifier une collecte.
    pub(crate) fn should_collect(&self) -> bool {
        self.allocated_since_gc >= self.threshold
    }

    /// Marque tout ce qui est atteignable depuis `roots` (valeurs) et
    /// `upvalue_roots` (cellules encore ouvertes), puis libère le reste.
    pub(crate) fn collect(&mut self, roots: impl Iterator<Item = Value>, upvalue_roots: &[UpId]) {
        let mut marked = vec![false; self.objects.len()];
        let mut marked_upvalues = vec![false; self.upvalues.len()];
        let mut pending: Vec<ObjId> = Vec::new();
        let mut pending_upvalues: Vec<UpId> = upvalue_roots.to_vec();
        let push = |value: &Value, pending: &mut Vec<ObjId>| {
            if let Value::Object(id) = value {
                pending.push(*id);
            }
        };
        for root in roots {
            push(&root, &mut pending);
        }
        loop {
            if let Some(up) = pending_upvalues.pop() {
                let Some(flag) = marked_upvalues.get_mut(up.0 as usize) else {
                    continue;
                };
                if std::mem::replace(flag, true) {
                    continue;
                }
                if let Some(Upvalue::Closed(value)) = self.upvalue(up) {
                    push(value, &mut pending);
                }
                continue;
            }
            let Some(id) = pending.pop() else { break };
            let Some(flag) = marked.get_mut(id.0 as usize) else {
                continue;
            };
            if std::mem::replace(flag, true) {
                continue;
            }
            let Some(Some(object)) = self.objects.get(id.0 as usize) else {
                continue;
            };
            pending.extend(object.proto);
            for property in object.props.values().chain(object.symbols.values()) {
                match &property.slot {
                    Slot::Data(value) => push(value, &mut pending),
                    Slot::Accessor { get, set } => {
                        pending.extend(*get);
                        pending.extend(*set);
                    }
                }
            }
            match &object.kind {
                Kind::Ordinary
                | Kind::Arguments
                | Kind::Native { .. }
                | Kind::Error
                | Kind::Host { .. }
                | Kind::Date(_)
                | Kind::ArrayBuffer(_)
                | Kind::RegExp { .. } => {}
                Kind::TypedArray(view) => pending.push(view.buffer),
                Kind::DataView { buffer, .. } => pending.push(*buffer),
                Kind::Array(items) => items.iter().for_each(|v| push(v, &mut pending)),
                Kind::Closure(closure) => {
                    pending_upvalues.extend(&closure.upvalues);
                    if let Some(this) = &closure.bound_this {
                        push(this, &mut pending);
                    }
                    pending.extend(closure.home);
                    pending.extend(closure.parent);
                    pending.extend(closure.fields);
                    pending.extend(closure.scope);
                }
                Kind::Bound { target, this, args } => {
                    pending.push(*target);
                    push(this, &mut pending);
                    args.iter().for_each(|v| push(v, &mut pending));
                }
                Kind::Primitive(value) => push(value, &mut pending),
                Kind::Proxy(proxy) => {
                    pending.extend(proxy.target);
                    pending.extend(proxy.handler);
                }
                Kind::IteratorHelper(helper) => {
                    for value in [&helper.iterator, &helper.next, &helper.func] {
                        push(value, &mut pending);
                    }
                    for (a, b) in helper.inner.iter().chain(&helper.sources) {
                        push(a, &mut pending);
                        push(b, &mut pending);
                    }
                }
                Kind::NativeData { data, .. } => data.iter().for_each(|v| push(v, &mut pending)),
                Kind::Coroutine(coroutine) => {
                    pending.extend(coroutine.promise);
                    for request in &coroutine.queue {
                        pending.push(request.promise);
                        match &request.mode {
                            Some(Resume::Next(v) | Resume::Throw(v) | Resume::Return(v)) => {
                                push(v, &mut pending);
                            }
                            None => {}
                        }
                    }
                    match &coroutine.state {
                        CoState::Start {
                            closure,
                            this,
                            args,
                        } => {
                            pending.push(*closure);
                            push(this, &mut pending);
                            args.iter().for_each(|v| push(v, &mut pending));
                        }
                        CoState::Suspended(frame) => {
                            pending.push(frame.closure);
                            pending.extend(frame.scope);
                            push(&frame.this, &mut pending);
                            push(&frame.new_target, &mut pending);
                            frame.args.iter().for_each(|v| push(v, &mut pending));
                            frame.registers.iter().for_each(|v| push(v, &mut pending));
                            pending_upvalues.extend(frame.upvalues.iter().map(|(_, id)| *id));
                        }
                        CoState::Running | CoState::Done => {}
                    }
                }
                Kind::Promise(promise) => {
                    match &promise.state {
                        PromiseState::Pending => {}
                        PromiseState::Fulfilled(value) | PromiseState::Rejected(value) => {
                            push(value, &mut pending);
                        }
                    }
                    for reaction in &promise.reactions {
                        push(&reaction.on_fulfilled, &mut pending);
                        push(&reaction.on_rejected, &mut pending);
                        pending.extend(reaction.target);
                    }
                }
                Kind::Map(map) => {
                    for (key, value) in map.values() {
                        push(key, &mut pending);
                        push(value, &mut pending);
                    }
                }
                Kind::Set(set) => set.values().for_each(|v| push(v, &mut pending)),
                Kind::Iterator(state) => match state {
                    IterState::Keys { .. } => {}
                    IterState::Array { target, .. } => pending.push(*target),
                    IterState::Values { values, .. } => {
                        values.iter().for_each(|v| push(v, &mut pending))
                    }
                    IterState::Protocol { iterator, next, .. } => {
                        push(iterator, &mut pending);
                        push(next, &mut pending);
                    }
                    IterState::Indexed { target, .. } => push(target, &mut pending),
                },
            }
        }
        for (index, slot) in self.objects.iter_mut().enumerate() {
            if slot.is_some() && !marked.get(index).copied().unwrap_or(false) {
                *slot = None;
                self.free.push(u32::try_from(index).unwrap_or(u32::MAX));
            }
        }
        for (index, slot) in self.upvalues.iter_mut().enumerate() {
            if slot.is_some() && !marked_upvalues.get(index).copied().unwrap_or(false) {
                *slot = None;
                self.free_upvalues
                    .push(u32::try_from(index).unwrap_or(u32::MAX));
            }
        }
        self.allocated_since_gc = 0;
        // Prochaine collecte quand le tas aura doublé, pour amortir le coût du marquage.
        self.threshold = (self.live_objects() * 2).max(GC_MIN_THRESHOLD);
        self.collections += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ordinary(heap: &mut Heap) -> ObjId {
        heap.alloc(Object::new(None, Kind::Ordinary))
    }

    #[test]
    fn unreachable_objects_are_freed_and_slots_reused() {
        let mut heap = Heap::new();
        let root = ordinary(&mut heap);
        let child = ordinary(&mut heap);
        let garbage = ordinary(&mut heap);
        heap.get_mut(root)
            .props
            .insert(Arc::from("c"), Property::data(Value::Object(child)));
        // Cycle entre deux objets inatteignables.
        let cycle = ordinary(&mut heap);
        heap.get_mut(garbage).proto = Some(cycle);
        heap.get_mut(cycle)
            .props
            .insert(Arc::from("g"), Property::data(Value::Object(garbage)));

        heap.collect(std::iter::once(Value::Object(root)), &[]);
        assert_eq!(heap.live_objects(), 2);
        assert_eq!(heap.collections, 1);
        assert!(heap.get(root).props.contains_key("c"));
        let reused = ordinary(&mut heap);
        assert!(
            reused == garbage || reused == cycle,
            "un emplacement libéré est réutilisé"
        );
    }

    #[test]
    fn closed_upvalues_keep_their_values_alive() {
        let mut heap = Heap::new();
        let captured = ordinary(&mut heap);
        let up = heap.alloc_upvalue(Upvalue::Closed(Value::Object(captured)));
        let dead_up = heap.alloc_upvalue(Upvalue::Closed(Value::Null));
        heap.collect(std::iter::empty(), &[up]);
        assert_eq!(heap.live_objects(), 1);
        assert!(heap.upvalue(up).is_some());
        assert!(heap.upvalue(dead_up).is_none());
    }

    #[test]
    fn stale_ids_are_harmless() {
        let mut heap = Heap::new();
        let id = ordinary(&mut heap);
        heap.collect(std::iter::empty(), &[]);
        assert!(heap.get(id).props.is_empty());
        heap.get_mut(id)
            .props
            .insert(Arc::from("x"), Property::data(Value::Null));
        assert!(heap.get(ObjId(9999)).props.is_empty());
    }

    #[test]
    fn map_keys_follow_same_value_zero() {
        let key = |v: Value| MapKey::from_value(&v);
        assert_eq!(key(Value::Number(0.0)), key(Value::Number(-0.0)));
        assert_eq!(key(Value::Number(f64::NAN)), key(Value::Number(-f64::NAN)));
        assert_ne!(key(Value::Number(1.0)), key(Value::string("1")));
    }
}
