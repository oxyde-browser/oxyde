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
    /// Définit (ou redéfinit) une variable globale déclarée ; `declared` :
    /// par le `var` d'un script, et non par un `eval`.
    InitGlobal {
        name: u32,
        src: Reg,
        declared: bool,
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
    /// Début d'un `yield*` : `base` reçoit l'itérateur de `src` et `base + 1`
    /// sa méthode `next`.
    DelegateStart {
        base: Reg,
        src: Reg,
    },
    /// Comme `DelegateStart`, pour un générateur async : l'itérateur est
    /// asynchrone (ou un itérateur synchrone adapté).
    DelegateStartAsync {
        base: Reg,
        src: Reg,
    },
    /// Première moitié d'un pas de `yield*` async : appelle la méthode de
    /// l'itérateur délégué ; le résultat (une promesse) va dans `base + 4`.
    /// `base + 5` et `base + 6` sont vrais si `return` est absent.
    DelegateCall {
        base: Reg,
    },
    /// Seconde moitié : interprète le résultat attendu, comme `DelegateStep`.
    DelegateFinish {
        base: Reg,
    },
    /// Un pas de `yield*`, sur sept registres consécutifs à partir de `base` :
    /// itérateur, `next`, valeur reçue, mode de reprise (0 `next`, 1 `throw`,
    /// 2 `return`), résultat, « délégué terminé », « retour demandé ».
    DelegateStep {
        base: Reg,
    },
    /// Avant toute liaison d'un script : `SyntaxError` si la déclaration
    /// globale `name` entre en collision avec une déclaration d'un script
    /// précédent. `lexical` : `let`, `const` ou `class` ; sinon `var` ou fonction.
    CheckGlobalDecl {
        name: u32,
        lexical: bool,
    },
    /// Crée la liaison lexicale globale `name`, non initialisée.
    DeclareLexical {
        name: u32,
    },
    /// Initialise la liaison lexicale globale `name` ; `constant` : `const`.
    InitLexical {
        name: u32,
        src: Reg,
        constant: bool,
    },
    /// `CanDeclareGlobalFunction` : `TypeError` si l'objet global ne peut pas
    /// recevoir la fonction `name`.
    CheckGlobalFunction {
        name: u32,
    },
    /// `var name` créé par un `eval` direct chez l'appelant : définit la
    /// propriété `name` de l'objet des variables d'`eval`, si elle manque.
    DefineVar {
        obj: Reg,
        name: u32,
    },
    /// Déclaration d'une fonction globale ; `declared` : par un script, et
    /// non par un `eval`.
    InitGlobalFunction {
        name: u32,
        src: Reg,
        declared: bool,
    },
    /// Objet des morceaux d'un gabarit étiqueté : le même à chaque
    /// évaluation du même gabarit.
    TemplateObject {
        dst: Reg,
        index: u32,
    },
    /// Appel écrit `eval(…)` : si la fonction appelée est bien `eval`, le
    /// code est évalué dans la portée de l'appelant décrite par le site
    /// `site` ; sinon c'est un appel ordinaire.
    DirectEval {
        dst: Reg,
        callee: Reg,
        first: Reg,
        argc: u16,
        site: u16,
    },
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
    /// `delete nom` sur une liaison globale : supprime la propriété de
    /// l'objet global (`false` si elle n'est pas configurable, ou si le nom
    /// est une liaison lexicale).
    DeleteGlobal {
        dst: Reg,
        name: u32,
    },
    /// Lève une `ReferenceError` dont le message est la constante `message`.
    ThrowReference {
        message: u32,
    },
    /// Appel en position terminale (`return f(…)` en mode strict) : comme
    /// `Call`, mais le cadre courant est remplacé par celui de la fonction
    /// appelée quand c'est possible (ECMA-262 §15.10, appels terminaux).
    TailCall {
        dst: Reg,
        callee: Reg,
        this: Reg,
        first: Reg,
        argc: u16,
    },
    /// Champ de classe : définit `obj[key]` (un champ privé ne se définit
    /// qu'une fois ; un champ public refusé lève `TypeError`).
    DefineField {
        obj: Reg,
        key: Reg,
        src: Reg,
    },
    /// Installe sur `obj` la méthode privée `func` (`kind` 0), ou
    /// l'accesseur privé de lecture (1) ou d'écriture (2).
    InitPrivate {
        obj: Reg,
        key: Reg,
        func: Reg,
        kind: u8,
    },
    /// `super[key] = src` : écriture à partir du prototype de l'objet
    /// d'attache, avec `this` pour receveur.
    SetSuper {
        key: Reg,
        src: Reg,
    },
    /// Prépare `obj[key] op= …` : `obj` ne doit être ni `null` ni
    /// `undefined`, et une clé objet est convertie une seule fois.
    KeyOnce {
        obj: Reg,
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

/// Site d'un appel `eval(…)` : ce que le code évalué voit de l'appelant.
/// Chaque liaison porte son nom, son emplacement et sa nature (un code
/// propre au compilateur).
#[derive(Clone, Debug)]
pub(crate) struct EvalSite {
    /// Liaisons de la fonction appelante : registre.
    pub(crate) locals: Vec<(JsStr, Reg, u8)>,
    /// Liaisons des fonctions englobantes : index de capture de l'appelant.
    pub(crate) upvalues: Vec<(JsStr, u16, u8)>,
    /// L'appel est écrit dans du code strict.
    pub(crate) strict: bool,
    /// L'appelant est le code de premier niveau d'un script : les `var` du
    /// code évalué sont globaux.
    pub(crate) top_level: bool,
    /// L'appelant est dans une méthode : `super.x` est permis.
    pub(crate) in_method: bool,
    /// L'appel est écrit dans l'initialiseur d'un champ de classe ou un bloc
    /// `static` : `arguments` y est interdit.
    pub(crate) in_field_init: bool,
    /// L'appel est écrit dans l'initialisation des paramètres d'une
    /// fonction : le code évalué ne peut pas y déclarer `arguments`.
    pub(crate) in_params: bool,
}

/// Morceaux d'un gabarit étiqueté : interprétés, puis bruts.
pub(crate) type TemplateSite = (Vec<Option<JsStr>>, Vec<JsStr>);

/// Fonction compilée, partagée par toutes les fermetures créées à partir d'elle.
#[derive(Debug)]
pub(crate) struct FunctionProto {
    /// Texte source (`Function.prototype.toString`).
    pub(crate) source: Option<crate::ast::SourceSpan>,
    pub(crate) name: JsStr,
    /// Nombre de paramètres formels.
    pub(crate) param_count: u16,
    /// Propriété `length` : paramètres qui précèdent le premier paramètre à
    /// valeur par défaut.
    pub(crate) length: u16,
    /// Taille de la fenêtre de registres.
    pub(crate) registers: u16,
    pub(crate) code: Vec<Op>,
    /// Constantes : nombres et chaînes uniquement (aucune référence au tas).
    pub(crate) constants: Vec<Value>,
    pub(crate) functions: Vec<Arc<FunctionProto>>,
    /// Gabarits étiquetés : morceaux interprétés (`None` : échappement
    /// invalide) et bruts.
    pub(crate) templates: Vec<TemplateSite>,
    /// Sites des appels `eval(…)` de la fonction.
    pub(crate) eval_sites: Vec<Arc<EvalSite>>,
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

    /// Vrai si la fonction porte une propriété `prototype` : constructeurs
    /// et générateurs (prototype de leurs objets générateurs).
    pub(crate) fn has_prototype(&self) -> bool {
        self.is_constructor() || self.is_generator
    }

    /// Vrai si l'appel crée une coroutine (générateur ou fonction async) au
    /// lieu d'exécuter le corps.
    pub(crate) fn is_coroutine(&self) -> bool {
        self.is_async || self.is_generator
    }
}
