//! Arbre syntaxique abstrait d'ECMAScript.

use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VarKind {
    Var,
    Let,
    Const,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UnaryOp {
    Neg,
    Plus,
    Not,
    BitNot,
    Typeof,
    Void,
    Delete,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Exp,
    Shl,
    Shr,
    UShr,
    BitAnd,
    BitOr,
    BitXor,
    Eq,
    Ne,
    StrictEq,
    StrictNe,
    Lt,
    Le,
    Gt,
    Ge,
    In,
    Instanceof,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LogicalOp {
    And,
    Or,
    Nullish,
}

/// Opérateur d'affectation : simple, composé arithmétique ou composé logique.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AssignOp {
    Assign,
    Binary(BinaryOp),
    Logical(LogicalOp),
}

/// Clé d'une propriété dans un littéral objet, une classe ou un motif.
#[derive(Clone, Debug)]
pub(crate) enum PropKey {
    Name(Arc<str>),
    Computed(Box<Expr>),
}

#[derive(Clone, Debug)]
pub(crate) enum PropKind {
    /// `clé: valeur`, raccourci ou méthode.
    Value(Expr),
    Getter(Arc<Function>),
    Setter(Arc<Function>),
    /// `...objet`
    Spread(Expr),
}

#[derive(Clone, Debug)]
pub(crate) struct Property {
    pub(crate) key: PropKey,
    pub(crate) kind: PropKind,
}

#[derive(Clone, Debug)]
pub(crate) enum Expr {
    Number(f64),
    BigInt(Arc<crate::bigint::BigInt>),
    String(Arc<str>),
    Bool(bool),
    Null,
    Ident(Arc<str>),
    This,
    /// `quasis.len() == exprs.len() + 1`
    Template {
        quasis: Vec<Arc<str>>,
        exprs: Vec<Expr>,
    },
    Regex {
        pattern: Arc<str>,
        flags: Arc<str>,
    },
    /// `None` : trou (`[1, , 3]`).
    Array(Vec<Option<Expr>>),
    Object(Vec<Property>),
    Function(Arc<Function>),
    Class(Box<Class>),
    Unary {
        op: UnaryOp,
        arg: Box<Expr>,
    },
    /// `++x`, `x--`…
    Update {
        increment: bool,
        prefix: bool,
        target: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Logical {
        op: LogicalOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Assign {
        op: AssignOp,
        target: Box<Pattern>,
        value: Box<Expr>,
    },
    Cond {
        test: Box<Expr>,
        then: Box<Expr>,
        otherwise: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        optional: bool,
    },
    New {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Member {
        object: Box<Expr>,
        name: Arc<str>,
        optional: bool,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
        optional: bool,
    },
    /// Borne d'une chaîne optionnelle : un `?.` court-circuite jusqu'ici.
    OptionalChain(Box<Expr>),
    Sequence(Vec<Expr>),
    /// `...x` dans un appel ou un littéral de tableau.
    Spread(Box<Expr>),
    /// `super.nom` ou `super[expr]`
    SuperMember(PropKey),
    /// `super(...)`
    SuperCall(Vec<Expr>),
    NewTarget,
    /// `import(spécificateur)` et son second argument éventuel (options).
    ImportCall(Box<Expr>, Option<Box<Expr>>),
    /// `yield expr`, `yield* expr` (dans un générateur seulement).
    Yield {
        arg: Option<Box<Expr>>,
        delegate: bool,
    },
    /// `await expr` (dans une fonction async seulement).
    Await(Box<Expr>),
    /// Nom privé (`#x`) : vaut la clé du membre privé, liée dans le corps de
    /// la classe qui le déclare. N'apparaît que comme index (`obj.#x`) ou à
    /// gauche de `in`.
    PrivateName(Arc<str>),
    /// Initialisation d'un champ d'instance : définit la propriété `key` de
    /// `this` (sans passer par un mutateur hérité).
    FieldInit {
        key: PropKey,
        value: Box<Expr>,
    },
}

/// Cible d'affectation ou de déclaration.
#[derive(Clone, Debug)]
pub(crate) enum Pattern {
    Ident(Arc<str>),
    /// Cible quelconque (`a.b`, `a[i]`) — affectation seulement.
    Expr(Expr),
    Object {
        props: Vec<(PropKey, Pattern, Option<Expr>)>,
        rest: Option<Box<Pattern>>,
    },
    Array {
        elements: Vec<Option<(Pattern, Option<Expr>)>>,
        rest: Option<Box<Pattern>>,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct Param {
    pub(crate) pattern: Pattern,
    pub(crate) default: Option<Expr>,
}

#[derive(Clone, Debug)]
pub(crate) struct Function {
    pub(crate) name: Option<Arc<str>>,
    pub(crate) params: Vec<Param>,
    pub(crate) rest: Option<Pattern>,
    pub(crate) body: Vec<Stmt>,
    pub(crate) is_arrow: bool,
    /// Méthode, accesseur ou constructeur de classe : n'a pas de `prototype`
    /// (sauf constructeur) et peut utiliser `super`.
    pub(crate) is_method: bool,
    /// `async function`, méthode ou fonction fléchée `async`.
    pub(crate) is_async: bool,
    /// `function*` ou méthode `*nom() {}`.
    pub(crate) is_generator: bool,
    /// Le corps est du code en mode strict.
    pub(crate) strict: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct ClassMember {
    pub(crate) key: PropKey,
    pub(crate) is_static: bool,
    pub(crate) kind: ClassMemberKind,
}

#[derive(Clone, Debug)]
pub(crate) enum ClassMemberKind {
    Method(Arc<Function>),
    Getter(Arc<Function>),
    Setter(Arc<Function>),
    Field(Option<Expr>),
    /// Bloc `static { … }` : une méthode sans paramètre, appelée une fois sur
    /// la classe.
    StaticBlock(Arc<Function>),
}

#[derive(Clone, Debug)]
pub(crate) struct Class {
    pub(crate) name: Option<Arc<str>>,
    pub(crate) parent: Option<Expr>,
    pub(crate) constructor: Option<Arc<Function>>,
    pub(crate) members: Vec<ClassMember>,
}

#[derive(Clone, Debug)]
pub(crate) enum ForInit {
    Decl(VarKind, Vec<(Pattern, Option<Expr>)>),
    Expr(Expr),
}

/// Variable de boucle de `for…in` / `for…of`.
#[derive(Clone, Debug)]
pub(crate) enum ForTarget {
    Decl(VarKind, Pattern),
    Pattern(Pattern),
}

#[derive(Clone, Debug)]
pub(crate) enum Stmt {
    Expr(Expr),
    VarDecl(VarKind, Vec<(Pattern, Option<Expr>)>),
    Function(Arc<Function>),
    Class(Box<Class>),
    Return(Option<Expr>),
    If {
        test: Expr,
        then: Box<Stmt>,
        otherwise: Option<Box<Stmt>>,
    },
    For {
        init: Option<ForInit>,
        test: Option<Expr>,
        update: Option<Expr>,
        body: Box<Stmt>,
    },
    ForIn {
        target: ForTarget,
        object: Expr,
        body: Box<Stmt>,
        of: bool,
        /// `for await (… of …)`.
        is_await: bool,
    },
    While {
        test: Expr,
        body: Box<Stmt>,
    },
    DoWhile {
        body: Box<Stmt>,
        test: Expr,
    },
    Break(Option<Arc<str>>),
    Continue(Option<Arc<str>>),
    Throw(Expr),
    Try {
        block: Vec<Stmt>,
        param: Option<Pattern>,
        handler: Option<Vec<Stmt>>,
        finalizer: Option<Vec<Stmt>>,
    },
    Switch {
        discriminant: Expr,
        cases: Vec<(Option<Expr>, Vec<Stmt>)>,
    },
    Block(Vec<Stmt>),
    Labeled(Arc<str>, Box<Stmt>),
    /// `with (objet) instruction`.
    With(Expr, Box<Stmt>),
    Empty,
}
