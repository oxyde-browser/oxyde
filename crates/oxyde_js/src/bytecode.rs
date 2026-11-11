//! Bytecode à registres, dans l'esprit d'Ignition (V8).
//!
//! Chaque fonction possède sa fenêtre de registres ; les instructions nomment
//! leurs opérandes par numéro de registre. Les sauts portent un index absolu
//! dans le code de la fonction.

use std::sync::Arc;

use crate::ast::BinaryOp;
use crate::value::{JsStr, Value};

/// Numéro de registre dans la fenêtre de la fonction courante.
pub(crate) type Reg = u16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UnaryKind {
    Neg,
    Plus,
    Not,
    BitNot,
    Typeof,
}

/// Origine d'une variable capturée, vue depuis la fonction qui la capture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct UpvalueDesc {
    /// Vrai : registre de la fonction englobante ; faux : variable déjà
    /// capturée par la fonction englobante.
    pub(crate) from_parent_register: bool,
    pub(crate) index: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FunctionKind {
    Normal,
    Arrow,
    Method,
    /// Constructeur de classe ; `derived` : la classe a une classe mère.
    ClassConstructor {
        derived: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Op {
    Nop,
    /// Suspend la coroutine courante en rendant `src` (`yield`, `await`) ;
    /// à la reprise, la valeur transmise arrive dans `dst`.
    Yield {
        dst: Reg,
        src: Reg,
    },
    /// Comme `Yield`, mais pour `await` : la coroutine attend la promesse
    /// `src` au lieu de rendre une valeur à son appelant.
    Await {
        dst: Reg,
        src: Reg,
    },
    /// `dst = src ± 1` (`++`, `--`) ; `src` est déjà numérique (nombre ou
    /// `BigInt`).
    Step {
        dst: Reg,
        src: Reg,
        increment: bool,
    },
    /// Nouvelle clé de membre privé (`#x`), propre à cette évaluation de la classe.
    PrivateSymbol {
        dst: Reg,
        name: u32,
    },
    /// Itérateur asynchrone de `src` (`for await`, `yield*` async).
    AsyncIterStart {
        dst: Reg,
        src: Reg,
    },
    LoadConst {
        dst: Reg,
        index: u32,
    },
    LoadUndefined {
        dst: Reg,
    },
    LoadNull {
        dst: Reg,
    },
    LoadBool {
        dst: Reg,
        value: bool,
    },
    /// Marque une liaison `let` / `const` comme non initialisée.
    LoadHole {
        dst: Reg,
    },
    Move {
        dst: Reg,
        src: Reg,
    },
    LoadThis {
        dst: Reg,
    },
    LoadNewTarget {
        dst: Reg,
    },
    /// `import(src)` : promesse de l'espace de noms du module demandé.
    ImportCall {
        dst: Reg,
        src: Reg,
    },
    /// Fonction en cours d'exécution (nom d'une expression de fonction nommée).
    LoadCallee {
        dst: Reg,
    },
    GetGlobal {
        dst: Reg,
        name: u32,
    },
    SetGlobal {
        name: u32,
        src: Reg,
    },
    /// Définit (ou redéfinit) une variable globale déclarée.
    InitGlobal {
        name: u32,
        src: Reg,
    },
    /// `typeof nom` : ne lève pas d'erreur si le nom n'existe pas.
    TypeofGlobal {
        dst: Reg,
        name: u32,
    },
    GetUpvalue {
        dst: Reg,
        index: u16,
    },
    SetUpvalue {
        index: u16,
        src: Reg,
    },
    /// Ferme les variables capturées situées dans les registres ≥ `from`.
    CloseUpvalues {
        from: Reg,
    },
    /// Précède l'appel `eval(…)` écrit dans du code strict : le code évalué
    /// hérite de ce mode.
    StrictEval,
    /// `{ __proto__: src }` : `src` devient le prototype de `obj` s'il est un
    /// objet ou `null` ; toute autre valeur est ignorée.
    SetProto {
        obj: Reg,
        src: Reg,
    },
    /// `ToObject` : `TypeError` pour `null` et `undefined`.
    ToObject {
        dst: Reg,
        src: Reg,
    },
    /// Dans un `with` : `dst` reçoit vrai si l'objet `obj` fournit la
    /// liaison `name` (propriété présente et non masquée par
    /// `Symbol.unscopables`).
    WithHas {
        dst: Reg,
        obj: Reg,
        name: u32,
    },
    /// Lève `ReferenceError` si le registre est encore non initialisé.
    CheckInit {
        src: Reg,
        name: u32,
    },
    ThrowConstAssign {
        name: u32,
    },
    GetProp {
        dst: Reg,
        obj: Reg,
        name: u32,
    },
    SetProp {
        obj: Reg,
        name: u32,
        src: Reg,
    },
    GetElem {
        dst: Reg,
        obj: Reg,
        key: Reg,
    },
    SetElem {
        obj: Reg,
        key: Reg,
        src: Reg,
    },
    DeleteProp {
        dst: Reg,
        obj: Reg,
        name: u32,
    },
    DeleteElem {
        dst: Reg,
        obj: Reg,
        key: Reg,
    },
    /// `super[key]` : recherche à partir du prototype de l'objet d'attache.
    GetSuper {
        dst: Reg,
        key: Reg,
    },
    NewObject {
        dst: Reg,
    },
    NewArray {
        dst: Reg,
    },
    ArrayPush {
        array: Reg,
        src: Reg,
    },
    ArrayHole {
        array: Reg,
    },
    ArraySpread {
        array: Reg,
        src: Reg,
    },
    /// Copie les propriétés propres énumérables de `src` dans `obj`.
    ObjectSpread {
        obj: Reg,
        src: Reg,
    },
    InitProp {
        obj: Reg,
        name: u32,
        src: Reg,
    },
    InitElem {
        obj: Reg,
        key: Reg,
        src: Reg,
    },
    /// Méthode de classe : propriété non énumérable.
    InitMethod {
        obj: Reg,
        key: Reg,
        func: Reg,
    },
    InitGetter {
        obj: Reg,
        key: Reg,
        func: Reg,
        enumerable: bool,
    },
    InitSetter {
        obj: Reg,
        key: Reg,
        func: Reg,
        enumerable: bool,
    },
    Binary {
        op: BinaryOp,
        dst: Reg,
        left: Reg,
        right: Reg,
    },
    Unary {
        op: UnaryKind,
        dst: Reg,
        src: Reg,
    },
    /// Conversion en nombre (`++`, `--`).
    ToNumber {
        dst: Reg,
        src: Reg,
    },
    /// Conversion en chaîne (gabarits).
    ToString {
        dst: Reg,
        src: Reg,
    },
    /// Conversion en clé de propriété (clés calculées).
    ToPropertyKey {
        dst: Reg,
        src: Reg,
    },
    Jump {
        target: u32,
    },
    JumpIfFalse {
        src: Reg,
        target: u32,
    },
    JumpIfTrue {
        src: Reg,
        target: u32,
    },
    JumpIfNullish {
        src: Reg,
        target: u32,
    },
    JumpIfNotNullish {
        src: Reg,
        target: u32,
    },
    JumpIfNotUndefined {
        src: Reg,
        target: u32,
    },
    Closure {
        dst: Reg,
        index: u32,
    },
    /// Appel : arguments dans `argc` registres consécutifs à partir de `first`.
    Call {
        dst: Reg,
        callee: Reg,
        this: Reg,
        first: Reg,
        argc: u16,
    },
    /// Appel dont les arguments sont dans le tableau `args`.
    CallSpread {
        dst: Reg,
        callee: Reg,
        this: Reg,
        args: Reg,
    },
    New {
        dst: Reg,
        callee: Reg,
        first: Reg,
        argc: u16,
    },
    NewSpread {
        dst: Reg,
        callee: Reg,
        args: Reg,
    },
    SuperCall {
        dst: Reg,
        args: Reg,
    },
    Return {
        src: Reg,
    },
    Throw {
        src: Reg,
    },
    /// Installe un gestionnaire : en cas d'exception, saute à `target` avec
    /// la valeur levée dans `reg`.
    PushHandler {
        target: u32,
        reg: Reg,
    },
    PopHandler,
    /// Itérateur sur les clés énumérables (`for…in`).
    ForInStart {
        dst: Reg,
        obj: Reg,
    },
    /// `IteratorClose` : appelle `return()` sur un itérateur que l'on quitte
    /// avant son épuisement. `quiet` : la sortie est due à une exception, les
    /// erreurs de `return()` sont ignorées.
    IterClose {
        iter: Reg,
        quiet: bool,
    },
    /// Itérateur sur les valeurs (`for…of`, déstructuration, décomposition).
    IterStart {
        dst: Reg,
        src: Reg,
    },
    /// Valeur suivante, ou saut à `done` quand l'itérateur est épuisé.
    IterNext {
        dst: Reg,
        iter: Reg,
        done: u32,
    },
    /// Valeur suivante, ou `undefined` quand l'itérateur est épuisé.
    IterNextOrUndefined {
        dst: Reg,
        iter: Reg,
    },
    /// Tableau des valeurs restantes.
    IterRest {
        dst: Reg,
        iter: Reg,
    },
    /// Tableau des arguments reçus à partir de l'index `from`.
    RestArgs {
        dst: Reg,
        from: u16,
    },
    /// Objet `arguments`.
    Arguments {
        dst: Reg,
    },
    RegExp {
        dst: Reg,
        pattern: u32,
        flags: u32,
    },
    /// Relie un constructeur de classe à son prototype (créé dans `proto`) et
    /// à sa classe mère éventuelle.
    MakeClass {
        ctor: Reg,
        proto: Reg,
        parent: Option<Reg>,
    },
    SetHome {
        func: Reg,
        home: Reg,
    },
    SetFields {
        ctor: Reg,
        func: Reg,
    },
}

impl Op {
    /// Fixe la cible d'un saut émis avec une cible provisoire.
    pub(crate) fn set_target(&mut self, new_target: u32) {
        match self {
            Self::Jump { target }
            | Self::JumpIfFalse { target, .. }
            | Self::JumpIfTrue { target, .. }
            | Self::JumpIfNullish { target, .. }
            | Self::JumpIfNotNullish { target, .. }
            | Self::JumpIfNotUndefined { target, .. }
            | Self::PushHandler { target, .. }
            | Self::IterNext { done: target, .. } => *target = new_target,
            _ => {}
        }
    }
}

/// Fonction compilée, partagée par toutes les fermetures créées à partir d'elle.
#[derive(Debug)]
pub(crate) struct FunctionProto {
    pub(crate) name: JsStr,
    /// Nombre de paramètres formels (propriété `length`).
    pub(crate) param_count: u16,
    /// Taille de la fenêtre de registres.
    pub(crate) registers: u16,
    pub(crate) code: Vec<Op>,
    /// Constantes : nombres et chaînes uniquement (aucune référence au tas).
    pub(crate) constants: Vec<Value>,
    pub(crate) functions: Vec<Arc<FunctionProto>>,
    pub(crate) upvalues: Vec<UpvalueDesc>,
    pub(crate) kind: FunctionKind,
    /// Fonction async : l'appel rend une promesse.
    pub(crate) is_async: bool,
    /// Générateur : l'appel rend un objet générateur.
    pub(crate) is_generator: bool,
    /// Code en mode strict : une écriture refusée lève `TypeError` et `this`
    /// n'est pas converti en objet.
    pub(crate) strict: bool,
}

impl FunctionProto {
    /// Vrai si la fonction peut être appelée avec `new`.
    pub(crate) fn is_constructor(&self) -> bool {
        matches!(
            self.kind,
            FunctionKind::Normal | FunctionKind::ClassConstructor { .. }
        ) && !self.is_async
            && !self.is_generator
    }

    /// Vrai si l'appel crée une coroutine (générateur ou fonction async) au
    /// lieu d'exécuter le corps.
    pub(crate) fn is_coroutine(&self) -> bool {
        self.is_async || self.is_generator
    }
}
