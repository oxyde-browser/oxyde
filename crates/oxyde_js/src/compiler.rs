//! Compilateur : de l'arbre syntaxique au bytecode à registres.
//!
//! Les variables locales vivent dans des registres. Une variable capturée par
//! une fonction interne reste dans son registre tant que sa portée est
//! active ; elle est déplacée dans le tas (« fermée ») à la sortie de la
//! portée, comme les *upvalues* de Lua. Aucune analyse préalable n'est donc
//! nécessaire : la capture est découverte en compilant la fonction interne.
//!
//! LIMITES CONNUES : au niveau du script, `let`, `const` et `class` créent des
//! propriétés de l'objet global (sans zone morte temporelle) ; les clés
//! calculées des champs de classe sont évaluées à chaque instanciation ;
//! `this` vaut `undefined` dans une fonction appelée sans receveur.

use std::collections::HashMap;
use std::sync::Arc;

use crate::ast::{
    AssignOp, BinaryOp, Class, ClassMemberKind, Expr, ForInit, ForTarget, Function, LogicalOp,
    Param, Pattern, PropKey, PropKind, Stmt, UnaryOp, VarKind,
};
use crate::bytecode::{EvalSite, FunctionKind, FunctionProto, Op, Reg, UnaryKind, UpvalueDesc};
use crate::lexer::SyntaxError;
use crate::value::{JsStr, Value};

type CResult<T> = Result<T, SyntaxError>;

/// Préfixe du nom de la liaison cachée qui porte l'objet d'un `with` ; ce
/// n'est pas un identifiant, aucun script ne peut la désigner.
const WITH_PREFIX: &str = "%with";

/// Préfixe de la liaison cachée qui porte, dans une fonction contenant un
/// appel `eval(…)`, l'objet des variables que le code évalué y crée. Il
/// commence par [`WITH_PREFIX`] : les noms s'y cherchent comme dans un `with`.
const EVAL_VARS_PREFIX: &str = "%with-eval";

/// Nom du paramètre de reste du constructeur implicite d'une classe
/// dérivée ; ce n'est pas un identifiant.
const FORWARDED_ARGS: &str = "%args";

fn error<T>(message: &str) -> CResult<T> {
    Err(SyntaxError {
        message: message.to_owned(),
        line: 0,
        column: 0,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BindKind {
    Var,
    Let,
    Const,
    /// Nom d'une expression de fonction, visible dans son corps : une
    /// affectation y est ignorée, ou lève `TypeError` en mode strict.
    FnName,
}

impl BindKind {
    /// Code de la nature d'une liaison dans un [`EvalSite`].
    fn code(self) -> u8 {
        match self {
            Self::Var => 0,
            Self::Let => 1,
            Self::Const => 2,
            Self::FnName => 3,
        }
    }

    fn from_code(code: u8) -> Self {
        match code {
            1 => Self::Let,
            2 => Self::Const,
            3 => Self::FnName,
            _ => Self::Var,
        }
    }
}

struct Binding {
    name: JsStr,
    reg: Reg,
    kind: BindKind,
    /// Vrai une fois la déclaration compilée : les lectures suivantes, dans
    /// l'ordre du source, n'ont plus besoin de vérifier la zone morte.
    initialized: bool,
    captured: bool,
}

struct Scope {
    bindings: Vec<Binding>,
    first_reg: Reg,
}

/// Ce qu'il faut défaire quand un saut (`break`, `continue`, `return`) quitte
/// une zone protégée.
#[derive(Clone)]
enum Exit {
    Handler,
    Finally(Arc<Vec<Stmt>>),
    /// Itérateur d'un `for…of` à fermer quand on sort de la boucle.
    IterClose(Reg),
    /// Ressources `using` d'une portée, à libérer quand on en sort : le
    /// registre de leur pile, et si la libération s'attend.
    Dispose(Reg, bool),
}

/// `Some(asynchrone)` si `body` déclare directement des ressources `using`.
fn using_kind(body: &[Stmt]) -> Option<bool> {
    let mut found = None;
    for stmt in body {
        if let Stmt::VarDecl(kind, _) = stmt
            && kind.is_using()
        {
            found = Some(found.unwrap_or(false) || *kind == VarKind::AwaitUsing);
        }
    }
    found
}

/// Cible de `break` / `continue`.
struct JumpTarget {
    labels: Vec<JsStr>,
    is_loop: bool,
    /// Accepte un `break` sans étiquette (boucle ou `switch`).
    breakable: bool,
    breaks: Vec<usize>,
    continues: Vec<usize>,
    exit_len: usize,
    /// Premier registre des portées internes : les variables capturées
    /// au-dessus sont fermées avant de sauter.
    scope_reg: Reg,
}

struct FuncState {
    kind: FunctionKind,
    name: JsStr,
    source: Option<crate::ast::SourceSpan>,
    is_script: bool,
    code: Vec<Op>,
    constants: Vec<Value>,
    string_constants: HashMap<JsStr, u32>,
    functions: Vec<Arc<FunctionProto>>,
    templates: Vec<crate::bytecode::TemplateSite>,
    eval_sites: Vec<Arc<EvalSite>>,
    /// Appelant synthétique d'un `eval` direct : ses captures, par nom.
    known_upvalues: Vec<(JsStr, u16, BindKind)>,
    upvalues: Vec<(UpvalueDesc, BindKind)>,
    scopes: Vec<Scope>,
    next_reg: Reg,
    max_reg: Reg,
    targets: Vec<JumpTarget>,
    pending_labels: Vec<JsStr>,
    exits: Vec<Exit>,
    /// Sauts « valeur nulle » des chaînes optionnelles en cours.
    optional_chains: Vec<Vec<usize>>,
    arguments_reg: Option<Reg>,
    /// Les éléments de `arguments` sont liés aux paramètres.
    mapped_arguments: bool,
    /// Code en mode strict.
    strict: bool,
    /// Annexe B.3.3 : fonctions de blocs qui sont aussi des variables de
    /// cette fonction.
    annex_b: Vec<JsStr>,
    /// Nombre de portées ouvertes au niveau du corps de la fonction.
    top_scopes: usize,
    uses_arguments: bool,
    param_count: u16,
    length: u16,
    /// Paramètres pas encore liés pendant l'évaluation d'une valeur par
    /// défaut : les lire lève `ReferenceError`.
    dead_params: Vec<JsStr>,
    /// Vrai pendant la compilation des valeurs par défaut des paramètres.
    in_params: Vec<JsStr>,
    /// Initialiseur d'un champ de classe ou bloc `static`.
    field_init: bool,
    /// Une déclaration lexicale de premier niveau d'un script est en cours
    /// de compilation ; le booléen dit si c'est un `const`.
    global_lexical: Option<bool>,
    is_async: bool,
    is_generator: bool,
}

enum Resolved {
    Local {
        reg: Reg,
        kind: BindKind,
        needs_check: bool,
    },
    Upvalue {
        index: u16,
        kind: BindKind,
    },
    Global,
}

pub(crate) struct Compiler {
    funcs: Vec<FuncState>,
    /// Nom à donner à la prochaine fonction ou classe anonyme compilée
    /// (`const f = () => {}` nomme la fonction « f »).
    name_hint: Option<JsStr>,
    /// Nombre d'instructions `with` qui entourent le code en cours de
    /// compilation, fonctions englobantes comprises.
    with_depth: usize,
    /// Nombre d'instructions `with` compilées : numérote leurs liaisons.
    with_count: usize,
    /// Le code compilé est un module : ses déclarations de premier niveau
    /// vivent dans la portée du module, pas dans l'environnement global.
    module: bool,
    /// Le code compilé est celui d'un `eval` : ses déclarations lexicales
    /// lui sont locales, et ses `var` aussi en mode strict.
    eval: bool,
    /// Les fonctions déclarées au premier niveau du code d'`eval` sont globales.
    eval_functions_global: bool,
    /// Le code compilé est celui d'un `eval` direct, sous un appelant synthétique.
    direct_eval: bool,
    /// `eval` direct : liaison de l'objet où l'appelant garde les variables
    /// créées par le code évalué.
    eval_vars: Option<JsStr>,
    /// La prochaine fonction compilée est l'initialiseur d'un champ de classe
    /// ou un bloc `static`.
    field_init_next: bool,
}

/// Compile un script en une fonction sans paramètre dont la valeur de retour
/// est celle de la dernière instruction-expression de premier niveau.
pub(crate) fn compile_script(
    body: &[Stmt],
    strict: bool,
    is_async: bool,
    module: bool,
    eval: bool,
) -> CResult<Arc<FunctionProto>> {
    let mut compiler = Compiler {
        funcs: Vec::new(),
        name_hint: None,
        with_depth: 0,
        with_count: 0,
        module,
        eval,
        eval_functions_global: eval && !strict,
        direct_eval: false,
        eval_vars: None,
        field_init_next: false,
    };
    compiler.push_function(FunctionKind::Normal, Arc::from("<script>"), true);
    compiler.f().strict = strict;
    compiler.f().is_async = is_async;
    compiler.push_scope();
    // Registre 0 : valeur d'achèvement du script.
    let completion = compiler.alloc()?;
    compiler.emit(Op::LoadUndefined { dst: completion });
    check_declarations(body, true, &[], strict)?;
    if !module && !(eval && strict) {
        // Collisions avec les scripts précédents, vérifiées avant toute liaison.
        let mut lexical = Vec::new();
        if !eval {
            lexical_names(body, &mut lexical);
        }
        let mut vars = Vec::new();
        var_names(body, &mut vars);
        for stmt in body {
            if let Stmt::Function(func) = stmt {
                vars.extend(func.name.clone());
            }
        }
        for (names, is_lexical) in [(&lexical, true), (&vars, false)] {
            for name in names {
                let name = compiler.string(name);
                compiler.emit(Op::CheckGlobalDecl {
                    name,
                    lexical: is_lexical,
                });
            }
        }
        for name in &lexical {
            let name = compiler.string(name);
            compiler.emit(Op::DeclareLexical { name });
        }
    }
    // Avant toute liaison : chaque fonction globale doit pouvoir être déclarée.
    for stmt in body {
        if let Stmt::Function(func) = stmt
            && let Some(name) = &func.name
            && !(eval && strict)
        {
            let name = compiler.string(name);
            compiler.emit(Op::CheckGlobalFunction { name });
        }
    }
    if eval {
        // Code d'`eval` : ses déclarations lexicales vivent dans une portée
        // qui lui est propre ; ses `var` sont globaux, sauf en mode strict.
        if !strict {
            compiler.hoist_vars(body, &[])?;
        }
        compiler.push_scope();
        if strict {
            compiler.hoist_vars(body, &[])?;
        }
    } else {
        compiler.hoist_vars(body, &[])?;
    }
    compiler.declare_block(body)?;
    compiler.statement_list(body)?;
    compiler.emit(Op::Return { src: completion });
    Ok(Arc::new(compiler.pop_function()?))
}

/// Noms liés par un motif.
pub(crate) fn pattern_names(pattern: &Pattern, out: &mut Vec<JsStr>) {
    match pattern {
        Pattern::Ident(name) => out.push(Arc::clone(name)),
        Pattern::Expr(_) => {}
        Pattern::Object { props, rest } => {
            for (_, p, _) in props {
                pattern_names(p, out);
            }
            if let Some(rest) = rest {
                pattern_names(rest, out);
            }
        }
        Pattern::Array { elements, rest } => {
            for (p, _) in elements.iter().flatten() {
                pattern_names(p, out);
            }
            if let Some(rest) = rest {
                pattern_names(rest, out);
            }
        }
    }
}

/// Noms déclarés par `var` dans `stmts`, sans entrer dans les fonctions.
/// Compile le code d'un `eval` direct comme une fonction fléchée sans
/// paramètre, créée dans la fonction appelante décrite par `site` : elle en
/// capture les liaisons et rend la valeur d'achèvement du code.
pub(crate) fn compile_direct_eval(
    body: &[Stmt],
    strict: bool,
    site: &EvalSite,
) -> CResult<Arc<FunctionProto>> {
    // Objet des variables d'`eval` de la fonction la plus proche.
    let eval_vars = site
        .locals
        .iter()
        .map(|(name, ..)| name)
        .chain(site.upvalues.iter().map(|(name, ..)| name))
        .find(|name| name.starts_with(EVAL_VARS_PREFIX))
        .cloned();
    let with_bindings = site
        .locals
        .iter()
        .map(|(name, ..)| name)
        .chain(site.upvalues.iter().map(|(name, ..)| name))
        .filter(|name| name.starts_with(WITH_PREFIX))
        .count();
    let mut compiler = Compiler {
        funcs: Vec::new(),
        name_hint: None,
        with_depth: with_bindings,
        with_count: 0,
        module: false,
        eval: true,
        eval_functions_global: site.top_level && !strict,
        direct_eval: true,
        eval_vars: eval_vars.filter(|_| !strict),
        field_init_next: false,
    };
    // Appelant synthétique : seules ses liaisons comptent.
    compiler.push_function(FunctionKind::Normal, Arc::from("<appelant>"), false);
    compiler.f().field_init = site.in_field_init;
    compiler.f().known_upvalues = site
        .upvalues
        .iter()
        .map(|(name, index, kind)| (Arc::clone(name), *index, BindKind::from_code(*kind)))
        .collect();
    compiler.push_scope();
    // Les objets de `with` et d'`eval` d'abord : les liaisons propres à
    // l'appelant, déclarées ensuite, passent avant eux.
    let (objects, plain): (Vec<_>, Vec<_>) = site
        .locals
        .iter()
        .partition(|(name, ..)| name.starts_with(WITH_PREFIX));
    for (name, reg, kind) in objects.into_iter().chain(plain) {
        let kind = BindKind::from_code(*kind);
        let initialized = matches!(kind, BindKind::Var | BindKind::FnName);
        compiler.declare_at(name, kind, initialized, *reg);
    }
    compiler.push_function(FunctionKind::Arrow, Arc::from("<eval>"), true);
    compiler.f().strict = strict;
    compiler.push_scope();
    // Registre 0 : valeur d'achèvement.
    let completion = compiler.alloc()?;
    compiler.emit(Op::LoadUndefined { dst: completion });
    check_declarations(body, true, &[], strict)?;
    if !site.params.is_empty() && !strict {
        // Dans l'initialisation des paramètres, un `var` du code évalué ne
        // peut pas redéclarer un paramètre ni l'objet `arguments`.
        let mut vars = Vec::new();
        var_names(body, &mut vars);
        for stmt in body {
            if let Stmt::Function(func) = stmt {
                vars.extend(func.name.clone());
            }
        }
        if let Some(name) = vars.iter().find(|name| site.params.contains(name)) {
            return error(&format!(
                "« {name} » ne peut pas être déclaré par eval dans des paramètres"
            ));
        }
    }
    // Au premier niveau d'un script, hors mode strict, les `var` du code
    // évalué sont globaux ; ailleurs ils lui restent locaux.
    let global_vars = site.top_level && !strict;
    if global_vars {
        let mut vars = Vec::new();
        var_names(body, &mut vars);
        for stmt in body {
            if let Stmt::Function(func) = stmt {
                vars.extend(func.name.clone());
            }
        }
        for name in &vars {
            let name = compiler.string(name);
            compiler.emit(Op::CheckGlobalDecl {
                name,
                lexical: false,
            });
        }
        for stmt in body {
            if let Stmt::Function(func) = stmt
                && let Some(name) = &func.name
            {
                let name = compiler.string(name);
                compiler.emit(Op::CheckGlobalFunction { name });
            }
        }
        compiler.hoist_vars(body, &[])?;
    }
    compiler.push_scope();
    if !global_vars {
        compiler.hoist_vars(body, &[])?;
    }
    compiler.declare_block(body)?;
    compiler.statement_list(body)?;
    compiler.emit(Op::Return { src: completion });
    Ok(Arc::new(compiler.pop_function()?))
}

/// Erreurs précoces de redéclaration dans une liste d'instructions
/// (ECMA-262 §14.2.1, §15.2.1, §16.1.1). `top` désigne un corps de fonction ou
/// de script, où les fonctions se déclarent comme des `var` ; `outer` porte
/// les noms déjà liés dans la même portée (paramètres, paramètre de `catch`).
///
/// LIMITE : deux fonctions de même nom dans un bloc sont acceptées même en
/// mode strict, faute de connaître ici le mode du code.
fn check_declarations(body: &[Stmt], top: bool, outer: &[JsStr], strict: bool) -> CResult<()> {
    let mut lexical: Vec<JsStr> = Vec::new();
    let mut functions: Vec<JsStr> = Vec::new();
    // Dans un bloc, deux fonctions de même nom ne sont tolérées que hors
    // mode strict et si ce sont deux fonctions ordinaires (annexe B.3.2.4).
    let mut plain: Vec<bool> = Vec::new();
    for stmt in body {
        match stmt {
            Stmt::VarDecl(kind, decls) if *kind != VarKind::Var => {
                for (pattern, _) in decls {
                    pattern_names(pattern, &mut lexical);
                }
            }
            Stmt::Class(class) => lexical.extend(class.name.clone()),
            Stmt::Function(func) => {
                if let Some(name) = &func.name {
                    let ordinary = !func.is_async && !func.is_generator;
                    if !top
                        && let Some(previous) = functions.iter().position(|f| f == name)
                        && (strict || !ordinary || !plain[previous])
                    {
                        return error(&format!("« {name} » est déjà déclaré"));
                    }
                    functions.push(Arc::clone(name));
                    plain.push(ordinary);
                }
            }
            _ => {}
        }
    }
    if lexical.is_empty() && (top || functions.is_empty()) {
        return Ok(());
    }
    let mut vars = Vec::new();
    var_names(body, &mut vars);
    for (i, name) in lexical.iter().enumerate() {
        if lexical[..i].contains(name)
            || outer.contains(name)
            || functions.contains(name)
            || vars.contains(name)
        {
            return error(&format!("« {name} » est déjà déclaré"));
        }
    }
    if !top {
        for name in &functions {
            if vars.contains(name) || outer.contains(name) {
                return error(&format!("« {name} » est déjà déclaré"));
            }
        }
    }
    Ok(())
}

/// Noms liés par les déclarations `let`, `const` et `class` directes de `body`.
pub(crate) fn lexical_names(body: &[Stmt], out: &mut Vec<JsStr>) {
    for stmt in body {
        match stmt {
            Stmt::VarDecl(kind, decls) if *kind != VarKind::Var => {
                for (pattern, _) in decls {
                    pattern_names(pattern, out);
                }
            }
            Stmt::Class(class) => out.extend(class.name.clone()),
            _ => {}
        }
    }
}

/// Référence déjà évaluée d'une cible d'affectation : objet et nom, objet
/// et clé, ou clé d'une propriété de `super`.
#[derive(Clone, Copy)]
enum TargetRef {
    Prop(Reg, u32),
    Elem(Reg, Reg),
    Super(Reg),
}

/// Annexe B.3.3 : noms des fonctions déclarées dans un bloc qui, hors mode
/// strict, deviennent aussi des variables de la fonction englobante. Une
/// telle fonction est écartée si un `var` du même nom à sa place entrerait en
/// conflit avec une liaison lexicale d'un bloc englobant (`shadow`).
fn block_functions(stmts: &[Stmt], nested: bool, shadow: &mut Vec<JsStr>, out: &mut Vec<JsStr>) {
    for stmt in stmts {
        block_functions_in(stmt, nested, shadow, out);
    }
}

fn block_functions_in(stmt: &Stmt, nested: bool, shadow: &mut Vec<JsStr>, out: &mut Vec<JsStr>) {
    let block = |body: &[Stmt], shadow: &mut Vec<JsStr>, out: &mut Vec<JsStr>| {
        let mark = shadow.len();
        lexical_names(body, shadow);
        block_functions(body, true, shadow, out);
        shadow.truncate(mark);
    };
    match stmt {
        Stmt::Function(func) if nested && !func.is_async && !func.is_generator => {
            if let Some(name) = &func.name
                && !shadow.contains(name)
                && !out.contains(name)
            {
                out.push(Arc::clone(name));
            }
        }
        Stmt::Block(body) => block(body, shadow, out),
        Stmt::If {
            then, otherwise, ..
        } => {
            block_functions_in(then, true, shadow, out);
            if let Some(otherwise) = otherwise {
                block_functions_in(otherwise, true, shadow, out);
            }
        }
        Stmt::For { init, body, .. } => {
            let mark = shadow.len();
            if let Some(ForInit::Decl(kind, decls)) = init
                && *kind != VarKind::Var
            {
                for (pattern, _) in decls {
                    pattern_names(pattern, shadow);
                }
            }
            block_functions_in(body, true, shadow, out);
            shadow.truncate(mark);
        }
        Stmt::ForIn { target, body, .. } => {
            let mark = shadow.len();
            if let ForTarget::Decl(kind, pattern) = target
                && *kind != VarKind::Var
            {
                pattern_names(pattern, shadow);
            }
            block_functions_in(body, true, shadow, out);
            shadow.truncate(mark);
        }
        Stmt::While { body, .. }
        | Stmt::DoWhile { body, .. }
        | Stmt::Labeled(_, body)
        | Stmt::With(_, body) => block_functions_in(body, true, shadow, out),
        Stmt::Try {
            block: tried,
            param,
            handler,
            finalizer,
        } => {
            block(tried, shadow, out);
            if let Some(handler) = handler {
                // Un paramètre de `catch` déstructuré est une liaison
                // lexicale ; un simple identifiant ne gêne pas (B.3.5).
                let mark = shadow.len();
                if let Some(pattern) = param
                    && !matches!(pattern, Pattern::Ident(_))
                {
                    pattern_names(pattern, shadow);
                }
                block(handler, shadow, out);
                shadow.truncate(mark);
            }
            if let Some(finalizer) = finalizer {
                block(finalizer, shadow, out);
            }
        }
        Stmt::Switch { cases, .. } => {
            let mark = shadow.len();
            for (_, body) in cases {
                lexical_names(body, shadow);
            }
            for (_, body) in cases {
                block_functions(body, true, shadow, out);
            }
            shadow.truncate(mark);
        }
        _ => {}
    }
}

pub(crate) fn var_names(stmts: &[Stmt], out: &mut Vec<JsStr>) {
    for stmt in stmts {
        var_names_in(stmt, out);
    }
}

fn var_names_in(stmt: &Stmt, out: &mut Vec<JsStr>) {
    match stmt {
        Stmt::VarDecl(VarKind::Var, decls) => {
            for (pattern, _) in decls {
                pattern_names(pattern, out);
            }
        }
        Stmt::If {
            then, otherwise, ..
        } => {
            var_names_in(then, out);
            if let Some(otherwise) = otherwise {
                var_names_in(otherwise, out);
            }
        }
        Stmt::For { init, body, .. } => {
            if let Some(ForInit::Decl(VarKind::Var, decls)) = init {
                for (pattern, _) in decls {
                    pattern_names(pattern, out);
                }
            }
            var_names_in(body, out);
        }
        Stmt::ForIn { target, body, .. } => {
            if let ForTarget::Decl(VarKind::Var, pattern) = target {
                pattern_names(pattern, out);
            }
            var_names_in(body, out);
        }
        Stmt::While { body, .. }
        | Stmt::DoWhile { body, .. }
        | Stmt::Labeled(_, body)
        | Stmt::With(_, body) => {
            var_names_in(body, out);
        }
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            var_names(block, out);
            if let Some(handler) = handler {
                var_names(handler, out);
            }
            if let Some(finalizer) = finalizer {
                var_names(finalizer, out);
            }
        }
        Stmt::Switch { cases, .. } => {
            for (_, body) in cases {
                var_names(body, out);
            }
        }
        Stmt::Block(body) => var_names(body, out),
        _ => {}
    }
}

/// Vrai si l'évaluation de `expr` ne peut modifier aucune variable.
fn is_pure(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Number(_)
            | Expr::BigInt(_)
            | Expr::String(_)
            | Expr::Bool(_)
            | Expr::Null
            | Expr::Ident(_)
            | Expr::This
    )
}

impl Compiler {
    // ---- État ------------------------------------------------------------

    fn f(&mut self) -> &mut FuncState {
        // INVARIANT : `funcs` n'est jamais vide pendant la compilation.
        let last = self.funcs.len() - 1;
        &mut self.funcs[last]
    }

    fn push_function(&mut self, kind: FunctionKind, name: JsStr, is_script: bool) {
        self.funcs.push(FuncState {
            kind,
            name,
            is_script,
            code: Vec::new(),
            constants: Vec::new(),
            string_constants: HashMap::new(),
            functions: Vec::new(),
            templates: Vec::new(),
            eval_sites: Vec::new(),
            known_upvalues: Vec::new(),
            upvalues: Vec::new(),
            scopes: Vec::new(),
            next_reg: 0,
            max_reg: 0,
            targets: Vec::new(),
            pending_labels: Vec::new(),
            exits: Vec::new(),
            optional_chains: Vec::new(),
            arguments_reg: None,
            strict: false,
            annex_b: Vec::new(),
            top_scopes: 0,
            uses_arguments: false,
            param_count: 0,
            length: 0,
            dead_params: Vec::new(),
            in_params: Vec::new(),
            mapped_arguments: false,
            field_init: false,
            global_lexical: None,
            is_async: false,
            is_generator: false,
            source: None,
        });
    }

    fn pop_function(&mut self) -> CResult<FunctionProto> {
        let Some(state) = self.funcs.pop() else {
            return error("pile de fonctions vide");
        };
        Ok(FunctionProto {
            source: state.source,
            name: state.name,
            param_count: state.param_count,
            length: state.length,
            registers: state.max_reg.max(1),
            code: state.code,
            constants: state.constants,
            functions: state.functions,
            templates: state.templates,
            eval_sites: state.eval_sites,
            upvalues: state.upvalues.into_iter().map(|(desc, _)| desc).collect(),
            kind: state.kind,
            is_async: state.is_async,
            is_generator: state.is_generator,
            strict: state.strict,
            mapped_arguments: state.mapped_arguments,
        })
    }

    fn emit(&mut self, op: Op) -> usize {
        let f = self.f();
        f.code.push(op);
        f.code.len() - 1
    }

    fn here(&mut self) -> u32 {
        u32::try_from(self.f().code.len()).unwrap_or(u32::MAX)
    }

    /// Fait pointer le saut d'index `jump` sur la prochaine instruction émise.
    fn patch(&mut self, jump: usize) {
        let target = self.here();
        if let Some(op) = self.f().code.get_mut(jump) {
            op.set_target(target);
        }
    }

    fn patch_to(&mut self, jump: usize, target: u32) {
        if let Some(op) = self.f().code.get_mut(jump) {
            op.set_target(target);
        }
    }

    fn alloc(&mut self) -> CResult<Reg> {
        let f = self.f();
        let reg = f.next_reg;
        f.next_reg = match f.next_reg.checked_add(1) {
            Some(next) if next < Reg::MAX => next,
            _ => return error("fonction trop complexe : trop de registres"),
        };
        f.max_reg = f.max_reg.max(f.next_reg);
        Ok(reg)
    }

    fn free_to(&mut self, mark: Reg) {
        self.f().next_reg = mark;
    }

    fn mark(&mut self) -> Reg {
        self.f().next_reg
    }

    fn number(&mut self, value: f64) -> u32 {
        let f = self.f();
        if let Some(i) = f
            .constants
            .iter()
            .position(|c| matches!(c, Value::Number(n) if n.to_bits() == value.to_bits()))
        {
            return u32::try_from(i).unwrap_or(u32::MAX);
        }
        f.constants.push(Value::Number(value));
        u32::try_from(f.constants.len() - 1).unwrap_or(u32::MAX)
    }

    fn string(&mut self, value: &JsStr) -> u32 {
        let f = self.f();
        if let Some(index) = f.string_constants.get(value) {
            return *index;
        }
        f.constants.push(Value::String(Arc::clone(value)));
        let index = u32::try_from(f.constants.len() - 1).unwrap_or(u32::MAX);
        f.string_constants.insert(Arc::clone(value), index);
        index
    }

    // ---- Portées et noms ---------------------------------------------------

    fn push_scope(&mut self) {
        let first_reg = self.f().next_reg;
        self.f().scopes.push(Scope {
            bindings: Vec::new(),
            first_reg,
        });
    }

    /// Quitte la portée courante : ferme ses variables capturées et libère ses registres.
    fn pop_scope(&mut self) {
        let Some(scope) = self.f().scopes.pop() else {
            return;
        };
        if scope.bindings.iter().any(|b| b.captured) {
            self.emit(Op::CloseUpvalues {
                from: scope.first_reg,
            });
        }
        self.free_to(scope.first_reg);
    }

    /// Vrai si les déclarations courantes deviennent des variables globales.
    fn at_script_level(&mut self) -> bool {
        let f = self.f();
        f.is_script && f.scopes.len() == 1
    }

    fn declare(&mut self, name: &JsStr, kind: BindKind, initialized: bool) -> CResult<Reg> {
        let reg = self.alloc()?;
        self.declare_at(name, kind, initialized, reg);
        Ok(reg)
    }

    fn declare_at(&mut self, name: &JsStr, kind: BindKind, initialized: bool, reg: Reg) {
        if let Some(scope) = self.f().scopes.last_mut() {
            scope.bindings.push(Binding {
                name: Arc::clone(name),
                reg,
                kind,
                initialized,
                captured: false,
            });
        }
    }

    fn find_local(
        &mut self,
        func: usize,
        name: &str,
        capture: bool,
    ) -> Option<(Reg, BindKind, bool)> {
        let state = self.funcs.get_mut(func)?;
        for scope in state.scopes.iter_mut().rev() {
            if let Some(binding) = scope.bindings.iter_mut().rev().find(|b| &*b.name == name) {
                binding.captured |= capture;
                if Some(binding.reg) == state.arguments_reg && name == "arguments" {
                    state.uses_arguments = true;
                }
                return Some((binding.reg, binding.kind, !binding.initialized));
            }
        }
        None
    }

    fn add_upvalue(&mut self, func: usize, desc: UpvalueDesc, kind: BindKind) -> CResult<u16> {
        let Some(state) = self.funcs.get_mut(func) else {
            return error("portée introuvable");
        };
        if let Some(i) = state.upvalues.iter().position(|(d, _)| *d == desc) {
            return Ok(u16::try_from(i).unwrap_or(u16::MAX));
        }
        if state.upvalues.len() >= usize::from(u16::MAX) {
            return error("fonction trop complexe : trop de variables capturées");
        }
        state.upvalues.push((desc, kind));
        Ok(u16::try_from(state.upvalues.len() - 1).unwrap_or(u16::MAX))
    }

    /// Cherche `name` dans les fonctions englobant `func` et enregistre la
    /// chaîne de captures nécessaire.
    fn resolve_upvalue(&mut self, func: usize, name: &str) -> CResult<Option<(u16, BindKind)>> {
        let Some(parent) = func.checked_sub(1) else {
            // Appelant synthétique d'un `eval` direct : ses captures sont connues.
            return Ok(self.funcs.get(func).and_then(|state| {
                state
                    .known_upvalues
                    .iter()
                    .find(|(known, ..)| &**known == name)
                    .map(|(_, index, kind)| (*index, *kind))
            }));
        };
        if let Some((reg, kind, _)) = self.find_local(parent, name, true) {
            let desc = UpvalueDesc {
                from_parent_register: true,
                index: reg,
            };
            return Ok(Some((self.add_upvalue(func, desc, kind)?, kind)));
        }
        let Some((index, kind)) = self.resolve_upvalue(parent, name)? else {
            return Ok(None);
        };
        let desc = UpvalueDesc {
            from_parent_register: false,
            index,
        };
        Ok(Some((self.add_upvalue(func, desc, kind)?, kind)))
    }

    fn resolve(&mut self, name: &str) -> CResult<Resolved> {
        let current = self.funcs.len() - 1;
        if let Some((reg, kind, needs_check)) = self.find_local(current, name, false) {
            let needs_check = needs_check && kind != BindKind::Var;
            return Ok(Resolved::Local {
                reg,
                kind,
                needs_check,
            });
        }
        Ok(match self.resolve_upvalue(current, name)? {
            Some((index, kind)) => Resolved::Upvalue { index, kind },
            None => Resolved::Global,
        })
    }

    /// Liaisons cachées des instructions `with` situées entre le code courant
    /// et la déclaration de `name`, de la plus interne à la plus externe :
    /// chacun de ces objets peut fournir `name` à l'exécution.
    fn with_chain(&self, name: &str) -> Vec<JsStr> {
        let mut chain = Vec::new();
        if self.with_depth == 0 {
            return chain;
        }
        for state in self.funcs.iter().rev() {
            for scope in state.scopes.iter().rev() {
                for binding in scope.bindings.iter().rev() {
                    if binding.name.starts_with(WITH_PREFIX) {
                        chain.push(Arc::clone(&binding.name));
                    } else if &*binding.name == name {
                        return chain;
                    }
                }
            }
        }
        // `eval` direct : les objets que l'appelant synthétique a capturés.
        if let Some(caller) = self.funcs.first() {
            for (known, ..) in &caller.known_upvalues {
                if &**known == name {
                    break;
                }
                if known.starts_with(WITH_PREFIX) {
                    chain.push(Arc::clone(known));
                }
            }
        }
        chain
    }

    /// Résout `name` contre les `with` englobants, comme une référence, avant
    /// d'évaluer la valeur qui lui sera affectée : le registre rendu reçoit
    /// l'objet de `with` qui fournit `name`, ou `undefined` si aucun ne le
    /// fournit. `None` hors de tout `with`.
    fn with_base(&mut self, name: &JsStr) -> CResult<Option<Reg>> {
        let chain = self.with_chain(name);
        if chain.is_empty() {
            return Ok(None);
        }
        let base = self.alloc()?;
        let index = self.string(name);
        let mut ends = Vec::new();
        for with in &chain {
            let mark = self.mark();
            let obj = self.load_plain(with, None)?;
            let flag = self.alloc()?;
            self.emit(Op::WithHas {
                dst: flag,
                obj,
                name: index,
            });
            let skip = self.emit(Op::JumpIfFalse {
                src: flag,
                target: 0,
            });
            self.emit(Op::Move {
                dst: base,
                src: obj,
            });
            ends.push(self.emit(Op::Jump { target: 0 }));
            self.patch(skip);
            self.free_to(mark);
        }
        self.emit(Op::LoadUndefined { dst: base });
        for end in ends {
            self.patch(end);
        }
        Ok(Some(base))
    }

    /// Lit `name` par la référence que [`Self::with_base`] a résolue.
    fn load_ref(&mut self, name: &JsStr, base: Option<Reg>, dst: Option<Reg>) -> CResult<Reg> {
        let Some(base) = base else {
            return self.load_plain(name, dst);
        };
        let dst = self.dst_or_alloc(dst)?;
        let index = self.string(name);
        let on_object = self.emit(Op::JumpIfNotUndefined {
            src: base,
            target: 0,
        });
        self.load_plain(name, Some(dst))?;
        let end = self.emit(Op::Jump { target: 0 });
        self.patch(on_object);
        self.emit(Op::GetProp {
            dst,
            obj: base,
            name: index,
        });
        self.patch(end);
        Ok(dst)
    }

    /// Écrit `name` par la référence que [`Self::with_base`] a résolue :
    /// l'objet de `with` reçoit la valeur même s'il a perdu la propriété
    /// entre-temps.
    fn store_ref(&mut self, name: &JsStr, base: Option<Reg>, src: Reg) -> CResult<()> {
        let Some(base) = base else {
            return self.store_plain(name, src, false);
        };
        let index = self.string(name);
        let on_object = self.emit(Op::JumpIfNotUndefined {
            src: base,
            target: 0,
        });
        self.store_plain(name, src, false)?;
        let end = self.emit(Op::Jump { target: 0 });
        self.patch(on_object);
        if self.f().strict {
            // En mode strict, écrire une liaison que l'objet n'a plus est
            // une `ReferenceError`.
            let mark = self.mark();
            let flag = self.alloc()?;
            self.emit(Op::WithHas {
                dst: flag,
                obj: base,
                name: index,
            });
            let present = self.emit(Op::JumpIfTrue {
                src: flag,
                target: 0,
            });
            let message = self.string(&Arc::from(format!("{name} n'est pas défini")));
            self.emit(Op::ThrowReference { message });
            self.patch(present);
            self.free_to(mark);
        }
        self.emit(Op::SetProp {
            obj: base,
            name: index,
            src,
        });
        self.patch(end);
        Ok(())
    }

    /// Charge la variable `name` ; renvoie le registre qui contient sa valeur.
    fn load(&mut self, name: &JsStr, dst: Option<Reg>) -> CResult<Reg> {
        let chain = self.with_chain(name);
        if chain.is_empty() {
            return self.load_plain(name, dst);
        }
        let dst = self.dst_or_alloc(dst)?;
        let mark = self.mark();
        let index = self.string(name);
        let mut ends = Vec::new();
        for with in &chain {
            let obj = self.load_plain(with, None)?;
            let flag = self.alloc()?;
            self.emit(Op::WithHas {
                dst: flag,
                obj,
                name: index,
            });
            let skip = self.emit(Op::JumpIfFalse {
                src: flag,
                target: 0,
            });
            self.emit(Op::GetProp {
                dst,
                obj,
                name: index,
            });
            ends.push(self.emit(Op::Jump { target: 0 }));
            self.patch(skip);
            self.free_to(mark);
        }
        self.load_plain(name, Some(dst))?;
        for end in ends {
            self.patch(end);
        }
        Ok(dst)
    }

    /// Charge `name` sans consulter les objets des `with` englobants.
    fn load_plain(&mut self, name: &JsStr, dst: Option<Reg>) -> CResult<Reg> {
        if self.f().dead_params.contains(name) {
            // Paramètre lu avant sa liaison : toujours une erreur.
            let dst = self.dst_or_alloc(dst)?;
            self.emit(Op::LoadHole { dst });
            let index = self.string(name);
            self.emit(Op::CheckInit {
                src: dst,
                name: index,
            });
            return Ok(dst);
        }
        match self.resolve(name)? {
            Resolved::Local {
                reg, needs_check, ..
            } => {
                if needs_check {
                    let name = self.string(name);
                    self.emit(Op::CheckInit { src: reg, name });
                }
                match dst {
                    Some(dst) if dst != reg => {
                        self.emit(Op::Move { dst, src: reg });
                        Ok(dst)
                    }
                    _ => Ok(reg),
                }
            }
            Resolved::Upvalue { index, kind } => {
                let dst = match dst {
                    Some(dst) => dst,
                    None => self.alloc()?,
                };
                self.emit(Op::GetUpvalue { dst, index });
                if kind != BindKind::Var {
                    let name = self.string(name);
                    self.emit(Op::CheckInit { src: dst, name });
                }
                Ok(dst)
            }
            Resolved::Global => {
                let dst = match dst {
                    Some(dst) => dst,
                    None => self.alloc()?,
                };
                let name = self.string(name);
                self.emit(Op::GetGlobal { dst, name });
                Ok(dst)
            }
        }
    }

    /// Écrit `src` dans la variable `name`. `init` : c'est sa déclaration.
    fn store(&mut self, name: &JsStr, src: Reg, init: bool) -> CResult<()> {
        let chain = if init {
            Vec::new()
        } else {
            self.with_chain(name)
        };
        if chain.is_empty() {
            return self.store_plain(name, src, init);
        }
        let mark = self.mark();
        let index = self.string(name);
        let mut ends = Vec::new();
        for with in &chain {
            let obj = self.load_plain(with, None)?;
            let flag = self.alloc()?;
            self.emit(Op::WithHas {
                dst: flag,
                obj,
                name: index,
            });
            let skip = self.emit(Op::JumpIfFalse {
                src: flag,
                target: 0,
            });
            self.emit(Op::SetProp {
                obj,
                name: index,
                src,
            });
            ends.push(self.emit(Op::Jump { target: 0 }));
            self.patch(skip);
            self.free_to(mark);
        }
        self.store_plain(name, src, init)?;
        for end in ends {
            self.patch(end);
        }
        Ok(())
    }

    /// Écrit dans `name` sans consulter les objets des `with` englobants.
    fn store_plain(&mut self, name: &JsStr, src: Reg, init: bool) -> CResult<()> {
        match self.resolve(name)? {
            Resolved::Local {
                reg,
                kind,
                needs_check,
            } => {
                if kind == BindKind::FnName && !init {
                    // Nom d'une expression de fonction : immuable.
                    if self.f().strict {
                        let name = self.string(name);
                        self.emit(Op::ThrowConstAssign { name });
                    }
                    return Ok(());
                }
                if kind == BindKind::Const && !init {
                    let name = self.string(name);
                    self.emit(Op::ThrowConstAssign { name });
                    return Ok(());
                }
                if needs_check && !init {
                    let name = self.string(name);
                    self.emit(Op::CheckInit { src: reg, name });
                }
                if reg != src {
                    self.emit(Op::Move { dst: reg, src });
                }
                if init {
                    let current = self.funcs.len() - 1;
                    for scope in self.funcs[current].scopes.iter_mut().rev() {
                        if let Some(b) = scope.bindings.iter_mut().rev().find(|b| b.name == *name) {
                            b.initialized = true;
                            break;
                        }
                    }
                }
            }
            Resolved::Upvalue { index, kind } => {
                if kind == BindKind::FnName && !init {
                    if self.f().strict {
                        let name = self.string(name);
                        self.emit(Op::ThrowConstAssign { name });
                    }
                } else if kind == BindKind::Const && !init {
                    let name = self.string(name);
                    self.emit(Op::ThrowConstAssign { name });
                } else {
                    self.emit(Op::SetUpvalue { index, src });
                }
            }
            Resolved::Global => {
                let lexical = self.f().global_lexical;
                let name = self.string(name);
                self.emit(match (init, lexical) {
                    (true, Some(constant)) => Op::InitLexical {
                        name,
                        src,
                        constant,
                    },
                    (true, None) => Op::InitGlobal {
                        name,
                        src,
                        declared: false,
                    },
                    (false, _) => Op::SetGlobal { name, src },
                });
            }
        }
        Ok(())
    }

    /// Déclare dans la portée de fonction courante les noms `var` de `body`.
    fn hoist_vars(&mut self, body: &[Stmt], params: &[JsStr]) -> CResult<()> {
        let mut names = Vec::new();
        var_names(body, &mut names);
        self.f().top_scopes = self.f().scopes.len();
        if !self.f().strict {
            // Annexe B.3.3 : une fonction déclarée dans un bloc est aussi une
            // variable de la fonction englobante.
            let mut shadow: Vec<JsStr> = params.to_vec();
            shadow.push(Arc::from("arguments"));
            lexical_names(body, &mut shadow);
            let mut functions = Vec::new();
            block_functions(body, false, &mut shadow, &mut functions);
            names.extend(functions.iter().cloned());
            self.f().annex_b = functions;
        }
        let script = self.at_script_level();
        for name in names {
            if script {
                // « Trou » : ne crée la variable globale que si elle n'existe pas déjà.
                let mark = self.mark();
                let tmp = self.alloc()?;
                self.emit(Op::LoadHole { dst: tmp });
                let index = self.string(&name);
                let declared = !self.eval;
                self.emit(Op::InitGlobal {
                    name: index,
                    src: tmp,
                    declared,
                });
                self.free_to(mark);
                continue;
            }
            if self.direct_eval && self.funcs.len() == 2 && !self.f().strict {
                // `eval` direct : un `var` déjà lié dans la fonction appelante
                // désigne cette liaison ; sinon il devient une variable de
                // l'appelant, gardée dans son objet des variables d'`eval`.
                let in_caller = self.funcs[0].scopes.iter().any(|scope| {
                    scope
                        .bindings
                        .iter()
                        .any(|b| b.name == name && b.kind == BindKind::Var)
                });
                if in_caller {
                    continue;
                }
                if let Some(vars) = self.eval_vars.clone() {
                    let mark = self.mark();
                    let obj = self.load_plain(&vars, None)?;
                    let index = self.string(&name);
                    self.emit(Op::DefineVar { obj, name: index });
                    self.free_to(mark);
                    continue;
                }
            }
            let current = self.funcs.len() - 1;
            let already = self.funcs[current]
                .scopes
                .last()
                .is_some_and(|s| s.bindings.iter().any(|b| b.name == name));
            if !already {
                let reg = self.declare(&name, BindKind::Var, true)?;
                self.emit(Op::LoadUndefined { dst: reg });
            }
        }
        Ok(())
    }

    /// Déclare les liaisons lexicales (`let`, `const`, `class`, fonctions) des
    /// instructions directes d'un bloc, puis crée ses fonctions.
    fn declare_block(&mut self, body: &[Stmt]) -> CResult<()> {
        let script = self.at_script_level();
        // Au premier niveau d'un `eval` non strict, les fonctions sont globales.
        let global_functions = script
            || (self.eval_functions_global && self.f().is_script && self.f().scopes.len() == 2);
        // `eval` direct dans une fonction : ses fonctions de premier niveau
        // deviennent des variables de l'appelant.
        let caller_functions = self
            .eval_vars
            .clone()
            .filter(|_| self.funcs.len() == 2 && self.f().is_script && self.f().scopes.len() == 2);
        if !script {
            for stmt in body {
                match stmt {
                    Stmt::VarDecl(kind, decls) if *kind != VarKind::Var => {
                        let bind = if *kind != VarKind::Let {
                            BindKind::Const
                        } else {
                            BindKind::Let
                        };
                        let mut names = Vec::new();
                        for (pattern, _) in decls {
                            pattern_names(pattern, &mut names);
                        }
                        for name in names {
                            let reg = self.declare(&name, bind, false)?;
                            self.emit(Op::LoadHole { dst: reg });
                        }
                    }
                    Stmt::Class(class) => {
                        if let Some(name) = &class.name {
                            let reg = self.declare(name, BindKind::Let, false)?;
                            self.emit(Op::LoadHole { dst: reg });
                        }
                    }
                    Stmt::Function(func) if !global_functions && caller_functions.is_none() => {
                        if let Some(name) = &func.name {
                            let reg = self.declare(name, BindKind::Var, true)?;
                            self.emit(Op::LoadUndefined { dst: reg });
                        }
                    }
                    _ => {}
                }
            }
        }
        for stmt in body {
            if let Stmt::Function(func) = stmt
                && let Some(name) = &func.name
            {
                let mark = self.mark();
                let tmp = self.alloc()?;
                let index = self.function(func, FunctionKind::Normal, false)?;
                self.emit(Op::Closure { dst: tmp, index });
                if global_functions {
                    // Fonction globale : la déclaration peut être refusée.
                    let name = self.string(name);
                    let declared = !self.eval;
                    self.emit(Op::InitGlobalFunction {
                        name,
                        src: tmp,
                        declared,
                    });
                } else if let Some(vars) = &caller_functions {
                    let obj = self.load_plain(vars, None)?;
                    let name = self.string(name);
                    self.emit(Op::SetProp {
                        obj,
                        name,
                        src: tmp,
                    });
                } else {
                    self.store(name, tmp, true)?;
                }
                self.free_to(mark);
            }
        }
        Ok(())
    }

    // ---- Fonctions ---------------------------------------------------------

    /// Compile `func` et renvoie son index dans les fonctions de la fonction courante.
    /// Compile `value` ; si c'est une fonction ou une classe anonyme, elle
    /// reçoit le nom `name` (ECMA-262, « NamedEvaluation »).
    fn expr_named(&mut self, value: &Expr, name: &JsStr, dst: Option<Reg>) -> CResult<Reg> {
        let anonymous = match value {
            Expr::Function(func) => func.name.is_none(),
            Expr::Class(class) => class.name.is_none(),
            _ => false,
        };
        if anonymous {
            self.name_hint = Some(Arc::clone(name));
        }
        self.expr(value, dst)
    }

    fn function(&mut self, func: &Function, kind: FunctionKind, bind_self: bool) -> CResult<u32> {
        let hint = self.name_hint.take();
        let field_init = std::mem::take(&mut self.field_init_next);
        let name = func.name.clone().or(hint).unwrap_or_else(|| Arc::from(""));
        self.push_function(kind, name, false);
        self.f().field_init = field_init;
        self.f().is_async = func.is_async;
        self.f().is_generator = func.is_generator;
        self.f().strict = func.strict;
        self.f().source = func.source.clone();
        // Le nom d'une expression de fonction vit dans une portée à part,
        // que les paramètres et les `var` du corps peuvent masquer.
        if bind_self {
            self.push_scope();
        }
        self.push_scope();
        let entry = self.emit(Op::Nop);
        // Une fonction non stricte qui appelle `eval(…)` garde un objet pour
        // les variables que le code évalué y crée.
        let eval_vars = func.has_direct_eval && !func.strict;
        if eval_vars {
            self.with_depth += 1;
        }

        // Les arguments arrivent dans les premiers registres.
        let mut complex: Vec<(usize, Reg, &Param)> = Vec::new();
        for (position, param) in func.params.iter().enumerate() {
            let reg = self.alloc()?;
            match &param.pattern {
                Pattern::Ident(name) => {
                    self.declare_at(name, BindKind::Var, true, reg);
                    if param.default.is_some() {
                        complex.push((position, reg, param));
                    }
                }
                _ => complex.push((position, reg, param)),
            }
        }
        self.f().param_count = u16::try_from(func.params.len()).unwrap_or(u16::MAX);
        // `arguments` lié aux paramètres : hors mode strict, paramètres
        // simples (ni valeur par défaut, ni motif, ni reste) et distincts.
        let mut simple_names: Vec<&JsStr> = Vec::new();
        let distinct = func.params.iter().all(|param| match &param.pattern {
            Pattern::Ident(name) if !simple_names.contains(&name) => {
                simple_names.push(name);
                true
            }
            _ => false,
        });
        self.f().mapped_arguments = !func.strict
            && kind != FunctionKind::Arrow
            && complex.is_empty()
            && func.rest.is_none()
            && distinct;
        let length = func
            .params
            .iter()
            .position(|param| param.default.is_some())
            .unwrap_or(func.params.len());
        self.f().length = u16::try_from(length).unwrap_or(u16::MAX);
        let rest = match &func.rest {
            Some(pattern) => {
                let reg = self.alloc()?;
                self.emit(Op::RestArgs {
                    dst: reg,
                    from: u16::try_from(func.params.len()).unwrap_or(u16::MAX),
                });
                Some((reg, pattern))
            }
            None => None,
        };
        if kind != FunctionKind::Arrow {
            let reg = self.alloc()?;
            self.f().arguments_reg = Some(reg);
            self.declare_at(&Arc::from("arguments"), BindKind::Var, true, reg);
        }
        if bind_self && let Some(name) = &func.name {
            let reg = self.alloc()?;
            self.emit(Op::LoadCallee { dst: reg });
            let depth = self.f().scopes.len();
            if let Some(scope) = depth
                .checked_sub(2)
                .and_then(|i| self.f().scopes.get_mut(i))
            {
                scope.bindings.push(Binding {
                    name: Arc::clone(name),
                    reg,
                    kind: BindKind::FnName,
                    initialized: true,
                    captured: false,
                });
            }
        }
        if eval_vars {
            // Objet sans prototype : seules les variables créées y figurent.
            let reg = self.alloc()?;
            self.emit(Op::NewObject { dst: reg });
            let null = self.alloc()?;
            self.emit(Op::LoadNull { dst: null });
            self.emit(Op::SetProto {
                obj: reg,
                src: null,
            });
            let name: JsStr = Arc::from(format!("{EVAL_VARS_PREFIX}{}", self.with_count));
            self.with_count += 1;
            // Déclarée en tête de portée : les liaisons de la fonction passent avant.
            if let Some(scope) = self.f().scopes.last_mut() {
                scope.bindings.insert(
                    0,
                    Binding {
                        name,
                        reg,
                        kind: BindKind::Var,
                        initialized: true,
                        captured: false,
                    },
                );
            }
        }
        // Un `eval` écrit dans les paramètres ne peut pas déclarer `arguments`
        // si la fonction en a déjà un : l'objet `arguments`, ou un paramètre
        // de ce nom pour une fonction fléchée.
        let mut declared = Vec::new();
        for param in &func.params {
            pattern_names(&param.pattern, &mut declared);
        }
        if let Some(rest) = &func.rest {
            pattern_names(rest, &mut declared);
        }
        if kind != FunctionKind::Arrow {
            declared.push(Arc::from("arguments"));
        }
        self.f().in_params = declared;
        for (position, reg, param) in complex {
            if let Some(default) = &param.default {
                let skip = self.emit(Op::JumpIfNotUndefined {
                    src: reg,
                    target: 0,
                });
                // Ce paramètre et les suivants ne sont pas encore liés.
                let mut dead = Vec::new();
                for later in func.params.iter().skip(position) {
                    pattern_names(&later.pattern, &mut dead);
                }
                self.f().dead_params = dead;
                let compiled = self.expr(default, Some(reg));
                self.f().dead_params.clear();
                compiled?;
                self.patch(skip);
            }
            if !matches!(param.pattern, Pattern::Ident(_)) {
                self.declare_pattern(&param.pattern, BindKind::Let)?;
                self.assign_pattern(&param.pattern, reg, true)?;
            }
        }
        if let Some((reg, pattern)) = rest {
            match pattern {
                Pattern::Ident(name) => self.declare_at(name, BindKind::Var, true, reg),
                _ => {
                    self.declare_pattern(pattern, BindKind::Let)?;
                    self.assign_pattern(pattern, reg, true)?;
                }
            }
        }
        self.f().in_params.clear();

        let mut param_names = Vec::new();
        for param in &func.params {
            pattern_names(&param.pattern, &mut param_names);
        }
        if let Some(rest) = &func.rest {
            pattern_names(rest, &mut param_names);
        }
        check_declarations(&func.body, true, &param_names, func.strict)?;
        self.hoist_vars(&func.body, &param_names)?;
        self.declare_block(&func.body)?;
        if func.is_generator {
            // `yield` initial : l'appel s'arrête ici, paramètres liés ; le
            // corps ne commence qu'au premier `next()`.
            let mark = self.mark();
            let scratch = self.alloc()?;
            self.emit(Op::LoadUndefined { dst: scratch });
            self.emit_yield(scratch, scratch, None)?;
            self.free_to(mark);
        }
        self.statement_list(&func.body)?;
        let tmp = self.alloc()?;
        self.emit(Op::LoadUndefined { dst: tmp });
        self.emit(Op::Return { src: tmp });
        if eval_vars {
            self.with_depth -= 1;
        }

        let state = self.f();
        if state.uses_arguments
            && let Some(dst) = state.arguments_reg
            && let Some(op) = state.code.get_mut(entry)
        {
            *op = Op::Arguments { dst };
        }
        let proto = Arc::new(self.pop_function()?);
        let parent = self.f();
        parent.functions.push(proto);
        Ok(u32::try_from(parent.functions.len() - 1).unwrap_or(u32::MAX))
    }

    /// Déclare (non initialisées) toutes les liaisons d'un motif.
    fn declare_pattern(&mut self, pattern: &Pattern, kind: BindKind) -> CResult<()> {
        let mut names = Vec::new();
        pattern_names(pattern, &mut names);
        for name in names {
            let reg = self.declare(&name, kind, false)?;
            self.emit(Op::LoadHole { dst: reg });
        }
        Ok(())
    }

    fn class(&mut self, class: &Class, dst: Option<Reg>) -> CResult<Reg> {
        // Pris tout de suite : une fonction anonyme de la clause `extends` ne
        // doit pas hériter du nom destiné à la classe.
        let name_hint = self.name_hint.take();
        let dst = match dst {
            Some(dst) => dst,
            None => self.alloc()?,
        };
        // Le nom de la classe est visible dans son corps.
        self.push_scope();
        let inner_name = match &class.name {
            Some(name) => {
                let reg = self.declare(name, BindKind::Const, false)?;
                self.emit(Op::LoadHole { dst: reg });
                Some(name)
            }
            None => None,
        };
        // Une clé par nom privé déclaré, liée sous ce nom dans le corps de la
        // classe : `obj.#x` la retrouve comme une variable ordinaire.
        let mut private_names: Vec<&JsStr> = Vec::new();
        for member in &class.members {
            if let PropKey::Name(name) = &member.key
                && name.starts_with('#')
                && !private_names.contains(&name)
            {
                private_names.push(name);
            }
        }
        for name in private_names {
            let reg = self.declare(name, BindKind::Const, true)?;
            let index = self.string(name);
            self.emit(Op::PrivateSymbol {
                dst: reg,
                name: index,
            });
        }
        let mark = self.mark();
        let parent = match &class.parent {
            Some(parent) => {
                let reg = self.alloc()?;
                self.expr(parent, Some(reg))?;
                Some(reg)
            }
            None => None,
        };
        let derived = parent.is_some();
        let synthesized;
        let constructor = match &class.constructor {
            Some(constructor) => constructor.as_ref(),
            None => {
                // Constructeur implicite : vide, ou `constructor(...args) { super(...args) }`.
                let args: JsStr = Arc::from(FORWARDED_ARGS);
                synthesized = Function {
                    is_async: false,
                    is_generator: false,
                    name: class.name.clone(),
                    params: Vec::new(),
                    rest: derived.then(|| Pattern::Ident(Arc::clone(&args))),
                    body: if derived {
                        vec![Stmt::Expr(Expr::SuperCall(vec![Expr::Spread(Box::new(
                            Expr::Ident(args),
                        ))]))]
                    } else {
                        Vec::new()
                    },
                    is_arrow: false,
                    is_method: true,
                    strict: true,
                    has_direct_eval: false,
                    source: None,
                };
                &synthesized
            }
        };
        let mut ctor_ast = constructor.clone();
        ctor_ast.name = class.name.clone().or(name_hint);
        ctor_ast.source = class.source.clone();
        let ctor = self.alloc()?;
        let index = self.function(&ctor_ast, FunctionKind::ClassConstructor { derived }, false)?;
        self.emit(Op::Closure { dst: ctor, index });
        let proto = self.alloc()?;
        self.emit(Op::MakeClass {
            ctor,
            proto,
            parent,
        });
        self.emit(Op::SetHome {
            func: ctor,
            home: proto,
        });

        let mut fields: Vec<Stmt> = Vec::new();
        // Clé calculée d'un champ d'instance : évaluée une seule fois, à la
        // définition de la classe, et gardée dans une liaison cachée que
        // l'initialiseur relit à chaque construction.
        let mut field_keys: Vec<Option<JsStr>> = Vec::with_capacity(class.members.len());
        for member in &class.members {
            // De même, une méthode ou un accesseur privé d'instance : la
            // fonction est créée une fois, puis installée sur chaque instance.
            let private_method = !matches!(
                member.kind,
                ClassMemberKind::Field(_) | ClassMemberKind::StaticBlock(_)
            ) && matches!(&member.key, PropKey::Name(name) if name.starts_with('#'));
            let kept = !member.is_static
                && (private_method
                    || (matches!(member.kind, ClassMemberKind::Field(_))
                        && matches!(member.key, PropKey::Computed(_))));
            field_keys.push(if kept {
                let name: JsStr = Arc::from(format!("%champ{}", self.with_count));
                self.with_count += 1;
                self.declare(&name, BindKind::Const, true)?;
                Some(name)
            } else {
                None
            });
        }
        // Champs et blocs statiques, évalués une fois les méthodes définies
        // et la classe liée à son nom : registre de la clé (champ) et corps.
        let mut statics: Vec<(Option<Reg>, Arc<Function>)> = Vec::new();
        let mut private_inits: Vec<Stmt> = Vec::new();
        for (member, field_key) in class.members.iter().zip(&field_keys) {
            let member_mark = self.mark();
            let home = if member.is_static { ctor } else { proto };
            let key = self.alloc()?;
            match &member.key {
                PropKey::Name(name) if name.starts_with('#') => {
                    self.load(name, Some(key))?;
                }
                PropKey::Name(name) => {
                    let index = self.string(name);
                    self.emit(Op::LoadConst { dst: key, index });
                }
                PropKey::Computed(expr) => {
                    self.expr(expr, Some(key))?;
                    self.emit(Op::ToPropertyKey { dst: key, src: key });
                }
            }
            match &member.kind {
                ClassMemberKind::Method(func)
                | ClassMemberKind::Getter(func)
                | ClassMemberKind::Setter(func) => {
                    let value = self.alloc()?;
                    let index = self.function(func, FunctionKind::Method, false)?;
                    self.emit(Op::Closure { dst: value, index });
                    self.emit(Op::SetHome { func: value, home });
                    if let (Some(binding), PropKey::Name(name)) = (field_key, &member.key) {
                        self.store(binding, value, true)?;
                        private_inits.push(Stmt::Expr(Expr::PrivateInit {
                            key: Arc::clone(name),
                            binding: Arc::clone(binding),
                            kind: match member.kind {
                                ClassMemberKind::Getter(_) => 1,
                                ClassMemberKind::Setter(_) => 2,
                                _ => 0,
                            },
                        }));
                        self.free_to(member_mark);
                        continue;
                    }
                    self.emit(match member.kind {
                        ClassMemberKind::Getter(_) => Op::InitGetter {
                            obj: home,
                            key,
                            func: value,
                            enumerable: false,
                        },
                        ClassMemberKind::Setter(_) => Op::InitSetter {
                            obj: home,
                            key,
                            func: value,
                            enumerable: false,
                        },
                        _ => Op::InitMethod {
                            obj: home,
                            key,
                            func: value,
                        },
                    });
                }
                ClassMemberKind::Field(value) if member.is_static => {
                    // L'initialiseur s'évalue comme une méthode de la classe :
                    // `this` y désigne la classe. Le registre de la clé, déjà
                    // évaluée, reste réservé jusque-là.
                    let value = value.clone().unwrap_or(Expr::Unary {
                        op: UnaryOp::Void,
                        arg: Box::new(Expr::Number(0.0)),
                    });
                    // Clé littérale : le champ se définit sur `this` comme un
                    // champ d'instance (une fonction anonyme en reçoit le nom).
                    let computed = matches!(member.key, PropKey::Computed(_));
                    let body = if computed {
                        Stmt::Return(Some(value))
                    } else {
                        Stmt::Expr(Expr::FieldInit {
                            key: member.key.clone(),
                            value: Box::new(value),
                        })
                    };
                    let initializer = Function {
                        name: None,
                        params: Vec::new(),
                        rest: None,
                        body: vec![body],
                        is_arrow: false,
                        is_method: true,
                        is_async: false,
                        is_generator: false,
                        strict: true,
                        has_direct_eval: false,
                        source: None,
                    };
                    statics.push((computed.then_some(key), Arc::new(initializer)));
                    if computed {
                        continue;
                    }
                }
                ClassMemberKind::StaticBlock(func) => statics.push((None, Arc::clone(func))),
                ClassMemberKind::Field(value) => {
                    // Champ d'instance : défini sur `this` à la construction.
                    let value = value.clone().unwrap_or(Expr::Unary {
                        op: UnaryOp::Void,
                        arg: Box::new(Expr::Number(0.0)),
                    });
                    let field_key = match field_key {
                        Some(name) => {
                            self.store(name, key, true)?;
                            PropKey::Computed(Box::new(Expr::Ident(Arc::clone(name))))
                        }
                        None => member.key.clone(),
                    };
                    fields.push(Stmt::Expr(Expr::FieldInit {
                        key: field_key,
                        value: Box::new(value),
                    }));
                }
            }
            self.free_to(member_mark);
        }
        // Méthodes et accesseurs privés d'abord, puis les champs dans l'ordre.
        private_inits.append(&mut fields);
        let fields = private_inits;
        if !fields.is_empty() {
            let initializer = Function {
                name: None,
                params: Vec::new(),
                rest: None,
                body: fields,
                is_arrow: false,
                is_method: true,
                is_async: false,
                is_generator: false,
                strict: true,
                has_direct_eval: false,
                source: None,
            };
            let reg = self.alloc()?;
            self.field_init_next = true;
            let index = self.function(&initializer, FunctionKind::Method, false)?;
            self.emit(Op::Closure { dst: reg, index });
            self.emit(Op::SetHome {
                func: reg,
                home: proto,
            });
            self.emit(Op::SetFields { ctor, func: reg });
        }
        if let Some(name) = inner_name {
            self.store(name, ctor, true)?;
        }
        for (key, func) in statics {
            let static_mark = self.mark();
            let callee = self.alloc()?;
            self.field_init_next = true;
            let index = self.function(&func, FunctionKind::Method, false)?;
            self.emit(Op::Closure { dst: callee, index });
            self.emit(Op::SetHome {
                func: callee,
                home: ctor,
            });
            let result = self.alloc()?;
            self.emit(Op::Call {
                dst: result,
                callee,
                this: ctor,
                first: result,
                argc: 0,
            });
            if let Some(key) = key {
                self.emit(Op::DefineField {
                    obj: ctor,
                    key,
                    src: result,
                });
            }
            self.free_to(static_mark);
        }
        self.emit(Op::Move { dst, src: ctor });
        self.free_to(mark);
        self.pop_scope();
        Ok(dst)
    }

    // ---- Instructions ------------------------------------------------------

    /// Branche d'un `if`. Une déclaration de fonction qui en tient lieu se
    /// comporte comme si un bloc l'entourait (annexe B.3.4).
    fn branch(&mut self, stmt: &Stmt) -> CResult<()> {
        if matches!(stmt, Stmt::Function(_)) {
            self.statements(std::slice::from_ref(stmt))
        } else {
            self.statement(stmt)
        }
    }

    fn statements(&mut self, body: &[Stmt]) -> CResult<()> {
        let strict = self.f().strict;
        check_declarations(body, false, &[], strict)?;
        self.push_scope();
        self.declare_block(body)?;
        self.statement_list(body)?;
        self.pop_scope();
        Ok(())
    }

    /// Compile les instructions de `body`, dont les liaisons sont déjà
    /// déclarées. Si certaines sont des `using`, leurs ressources sont
    /// libérées à la sortie, quelle qu'en soit la cause.
    fn statement_list(&mut self, body: &[Stmt]) -> CResult<()> {
        let compile = |compiler: &mut Self| {
            for stmt in body {
                compiler.statement(stmt)?;
            }
            Ok(())
        };
        match using_kind(body) {
            Some(is_async) => self.with_disposal(is_async, compile),
            None => compile(self),
        }
    }

    /// Registre de la pile de ressources de la portée `using` la plus proche.
    fn dispose_stack(&mut self) -> Option<Reg> {
        self.f().exits.iter().rev().find_map(|exit| match exit {
            Exit::Dispose(stack, _) => Some(*stack),
            _ => None,
        })
    }

    /// Émet la libération des ressources de `stack` ; `error` : registre de
    /// l'exception en cours, que la libération relance ou enveloppe.
    fn emit_dispose(&mut self, stack: Reg, error: Option<Reg>, is_async: bool) -> CResult<()> {
        let mark = self.mark();
        let result = self.alloc()?;
        self.emit(Op::UsingDispose {
            dst: result,
            stack,
            error: error.unwrap_or(result),
            has_error: error.is_some(),
            is_async,
        });
        if is_async {
            // `false` : aucune ressource n'était inscrite, rien à attendre.
            let skip = self.emit(Op::JumpIfFalse {
                src: result,
                target: 0,
            });
            self.emit(Op::Await {
                dst: result,
                src: result,
            });
            self.patch(skip);
        }
        self.free_to(mark);
        Ok(())
    }

    /// Compile `body` dans une portée de ressources : une pile reçoit les
    /// `using`, et elle est vidée à la fin du corps, à chaque saut qui en
    /// sort, et quand une exception le traverse.
    fn with_disposal(
        &mut self,
        is_async: bool,
        body: impl FnOnce(&mut Self) -> CResult<()>,
    ) -> CResult<()> {
        let stack = self.alloc()?;
        self.emit(Op::UsingStack { dst: stack });
        let exception = self.alloc()?;
        self.f().exits.push(Exit::Dispose(stack, is_async));
        let handler = self.emit(Op::PushHandler {
            target: 0,
            reg: exception,
        });
        self.f().exits.push(Exit::Handler);
        let result = body(self);
        self.f().exits.pop();
        self.f().exits.pop();
        result?;
        self.emit(Op::PopHandler);
        self.emit_dispose(stack, None, is_async)?;
        let end = self.emit(Op::Jump { target: 0 });
        // Arrivée d'une exception : la libération la relance, enveloppée
        // si une ressource lève à son tour.
        self.patch(handler);
        self.emit_dispose(stack, Some(exception), is_async)?;
        self.emit(Op::Throw { src: exception });
        self.patch(end);
        Ok(())
    }

    fn push_target(&mut self, is_loop: bool, breakable: bool) {
        let f = self.f();
        let labels = std::mem::take(&mut f.pending_labels);
        let (exit_len, scope_reg) = (f.exits.len(), f.next_reg);
        f.targets.push(JumpTarget {
            labels,
            is_loop,
            breakable,
            breaks: Vec::new(),
            continues: Vec::new(),
            exit_len,
            scope_reg,
        });
    }

    /// Termine la cible courante : les `break` arrivent ici, les `continue` en `continue_to`.
    fn pop_target(&mut self, continue_to: u32) {
        let Some(target) = self.f().targets.pop() else {
            return;
        };
        for jump in target.breaks {
            self.patch(jump);
        }
        for jump in target.continues {
            self.patch_to(jump, continue_to);
        }
    }

    /// Émet le code qui défait les zones protégées au-dessus de `down_to`.
    /// Évalue dans `reg` la valeur d'un `return`, en marquant comme terminaux
    /// les appels qui la produisent directement : `f()`, `c ? f() : g()`,
    /// `a && f()`, `(x, f())`.
    fn tail_expr(&mut self, expr: &Expr, reg: Reg) -> CResult<()> {
        match expr {
            Expr::Cond {
                test,
                then,
                otherwise,
            } => {
                let mark = self.mark();
                let cond = self.expr(test, None)?;
                let to_else = self.emit(Op::JumpIfFalse {
                    src: cond,
                    target: 0,
                });
                self.free_to(mark);
                self.tail_expr(then, reg)?;
                let end = self.emit(Op::Jump { target: 0 });
                self.patch(to_else);
                self.tail_expr(otherwise, reg)?;
                self.patch(end);
            }
            Expr::Logical { op, left, right } => {
                self.expr(left, Some(reg))?;
                let skip = self.emit(match op {
                    LogicalOp::And => Op::JumpIfFalse {
                        src: reg,
                        target: 0,
                    },
                    LogicalOp::Or => Op::JumpIfTrue {
                        src: reg,
                        target: 0,
                    },
                    LogicalOp::Nullish => Op::JumpIfNotNullish {
                        src: reg,
                        target: 0,
                    },
                });
                self.tail_expr(right, reg)?;
                self.patch(skip);
            }
            Expr::Sequence(items) => {
                if let Some((last, first)) = items.split_last() {
                    for item in first {
                        let mark = self.mark();
                        self.expr(item, None)?;
                        self.free_to(mark);
                    }
                    self.tail_expr(last, reg)?;
                }
            }
            Expr::Call { .. } => {
                self.expr(expr, Some(reg))?;
                if let Some(last) = self.f().code.last_mut()
                    && let Op::Call {
                        dst,
                        callee,
                        this,
                        first,
                        argc,
                    } = *last
                    && dst == reg
                {
                    *last = Op::TailCall {
                        dst,
                        callee,
                        this,
                        first,
                        argc,
                    };
                }
            }
            _ => {
                self.expr(expr, Some(reg))?;
            }
        }
        Ok(())
    }

    fn unwind_exits(&mut self, down_to: usize) -> CResult<()> {
        let mut index = self.f().exits.len();
        while index > down_to {
            index -= 1;
            match self.f().exits.get(index).cloned() {
                Some(Exit::Handler) => {
                    self.emit(Op::PopHandler);
                }
                Some(Exit::IterClose(iter)) => {
                    self.emit(Op::IterClose { iter, quiet: false });
                }
                Some(Exit::Dispose(stack, is_async)) => {
                    self.emit_dispose(stack, None, is_async)?;
                }
                Some(Exit::Finally(body)) => {
                    // Le bloc `finally` est recopié sur ce chemin de sortie ; pendant sa
                    // compilation, lui-même et les zones internes ne sont plus actifs.
                    let saved = self.f().exits.split_off(index);
                    let result = self.statements(&body);
                    self.f().exits.extend(saved);
                    result?;
                }
                None => {}
            }
        }
        Ok(())
    }

    fn jump_out(&mut self, label: Option<&JsStr>, is_continue: bool) -> CResult<()> {
        let found = self.f().targets.iter().rposition(|t| match label {
            Some(label) => t.labels.contains(label) && (!is_continue || t.is_loop),
            None => {
                if is_continue {
                    t.is_loop
                } else {
                    t.breakable
                }
            }
        });
        let Some(index) = found else {
            return error(if is_continue {
                "« continue » sans boucle correspondante"
            } else {
                "« break » sans cible correspondante"
            });
        };
        let (exit_len, scope_reg) = {
            let target = &self.f().targets[index];
            (target.exit_len, target.scope_reg)
        };
        self.unwind_exits(exit_len)?;
        self.emit(Op::CloseUpvalues { from: scope_reg });
        let jump = self.emit(Op::Jump { target: 0 });
        if let Some(target) = self.f().targets.get_mut(index) {
            if is_continue {
                target.continues.push(jump)
            } else {
                target.breaks.push(jump)
            }
        }
        Ok(())
    }

    fn statement(&mut self, stmt: &Stmt) -> CResult<()> {
        let mark = self.mark();
        let result = self.statement_inner(stmt);
        self.free_to(mark);
        result
    }

    fn var_declaration(&mut self, kind: VarKind, decls: &[(Pattern, Option<Expr>)]) -> CResult<()> {
        // `let` et `const` de premier niveau d'un script vont dans
        // l'environnement lexical global.
        let global = kind != VarKind::Var && !self.module && self.at_script_level();
        if global {
            self.f().global_lexical = Some(kind == VarKind::Const);
        }
        let result = self.var_declaration_inner(kind, decls);
        self.f().global_lexical = None;
        result
    }

    fn var_declaration_inner(
        &mut self,
        kind: VarKind,
        decls: &[(Pattern, Option<Expr>)],
    ) -> CResult<()> {
        let init = kind != VarKind::Var;
        for (pattern, value) in decls {
            let mark = self.mark();
            match (pattern, value) {
                (Pattern::Ident(name), Some(value)) if kind.is_using() => {
                    let reg = self.expr_named(value, name, None)?;
                    let Some(stack) = self.dispose_stack() else {
                        return error("déclaration « using » hors d'une portée de ressources");
                    };
                    self.emit(Op::UsingAdd {
                        stack,
                        src: reg,
                        is_async: kind == VarKind::AwaitUsing,
                    });
                    self.store(name, reg, true)?;
                }
                (Pattern::Ident(name), Some(value))
                    if !init && !self.with_chain(name).is_empty() =>
                {
                    // `var x = v` dans un `with` : une affectation ordinaire,
                    // qui atteint l'objet du `with` s'il fournit `x`.
                    let base = self.with_base(name)?;
                    let reg = self.expr_named(value, name, None)?;
                    self.store_ref(name, base, reg)?;
                }
                (Pattern::Ident(name), Some(value)) => {
                    // `export default <expression>` : la valeur s'appelle « default ».
                    let reg = if &**name == crate::parser::DEFAULT_EXPORT {
                        self.expr_named(value, &Arc::from("default"), None)?
                    } else {
                        self.expr_named(value, name, None)?
                    };
                    self.store(name, reg, init)?;
                }
                (Pattern::Ident(name), None) => {
                    if init {
                        let reg = self.alloc()?;
                        self.emit(Op::LoadUndefined { dst: reg });
                        self.store(name, reg, true)?;
                    }
                }
                (pattern, Some(value)) => {
                    let reg = self.expr(value, None)?;
                    self.assign_pattern(pattern, reg, init)?;
                }
                (_, None) => return error("un motif de déstructuration doit être initialisé"),
            }
            self.free_to(mark);
        }
        Ok(())
    }

    fn statement_inner(&mut self, stmt: &Stmt) -> CResult<()> {
        // Valeur d'achèvement d'un script : une instruction composée part
        // de `undefined`, sans garder la valeur de l'instruction précédente.
        if self.f().is_script
            && matches!(
                stmt,
                Stmt::If { .. }
                    | Stmt::While { .. }
                    | Stmt::DoWhile { .. }
                    | Stmt::For { .. }
                    | Stmt::ForIn { .. }
                    | Stmt::Switch { .. }
                    | Stmt::Try { .. }
                    | Stmt::With(..)
            )
        {
            self.emit(Op::LoadUndefined { dst: 0 });
        }
        match stmt {
            Stmt::Empty => {}
            Stmt::Function(func) => {
                // Annexe B.3.3 : à l'endroit de sa déclaration, la fonction
                // du bloc est recopiée dans la variable de la fonction
                // englobante.
                if let Some(name) = &func.name
                    && self.f().scopes.len() > self.f().top_scopes
                    && self.f().annex_b.contains(name)
                {
                    let mark = self.mark();
                    let value = self.load_plain(name, None)?;
                    let top = self.f().top_scopes;
                    let outer = self
                        .f()
                        .scopes
                        .iter()
                        .take(top)
                        .rev()
                        .find_map(|scope| scope.bindings.iter().rev().find(|b| b.name == *name))
                        .map(|binding| binding.reg);
                    match outer {
                        Some(reg) if reg != value => {
                            self.emit(Op::Move {
                                dst: reg,
                                src: value,
                            });
                        }
                        Some(_) => {}
                        None if self.direct_eval && self.funcs.len() == 2 => {
                            // `eval` direct : la variable est chez l'appelant,
                            // dans une de ses liaisons ou dans son objet des
                            // variables d'`eval` ; à défaut, elle est globale.
                            let in_caller = self.funcs[0].scopes.iter().any(|scope| {
                                scope
                                    .bindings
                                    .iter()
                                    .any(|b| b.name == *name && b.kind == BindKind::Var)
                            });
                            let index = self.string(name);
                            if in_caller
                                && let Some((upvalue, _)) = self.resolve_upvalue(1, name)?
                            {
                                self.emit(Op::SetUpvalue {
                                    index: upvalue,
                                    src: value,
                                });
                            } else if let Some(vars) = self.eval_vars.clone() {
                                let obj = self.load_plain(&vars, None)?;
                                self.emit(Op::SetProp {
                                    obj,
                                    name: index,
                                    src: value,
                                });
                            } else {
                                self.emit(Op::SetGlobal {
                                    name: index,
                                    src: value,
                                });
                            }
                        }
                        None if self.f().is_script => {
                            let index = self.string(name);
                            self.emit(Op::SetGlobal {
                                name: index,
                                src: value,
                            });
                        }
                        None => {}
                    }
                    self.free_to(mark);
                }
            }
            Stmt::Expr(expr) => {
                // Dans un script, toute instruction-expression (même dans un
                // bloc) fixe la valeur d'achèvement, gardée dans le registre 0.
                if self.f().is_script {
                    self.expr(expr, Some(0))?;
                } else {
                    self.expr(expr, None)?;
                }
            }
            Stmt::VarDecl(kind, decls) => self.var_declaration(*kind, decls)?,
            Stmt::Class(class) => {
                let reg = self.class(class, None)?;
                if let Some(name) = &class.name {
                    if !self.module && self.at_script_level() {
                        self.f().global_lexical = Some(false);
                    }
                    let stored = self.store(name, reg, true);
                    self.f().global_lexical = None;
                    stored?;
                }
            }
            Stmt::Return(value) => {
                if self.f().is_script {
                    return error("« return » en dehors d'une fonction");
                }
                let reg = self.alloc()?;
                match value {
                    Some(value) => {
                        // Appel terminal : en mode strict, dans une fonction
                        // ordinaire, hors de tout `try` et de toute boucle
                        // d'itération à fermer.
                        let state = self.f();
                        let tail = state.strict
                            && !state.is_generator
                            && !state.is_async
                            && state.exits.is_empty()
                            && !matches!(state.kind, FunctionKind::ClassConstructor { .. });
                        if tail {
                            self.tail_expr(value, reg)?;
                        } else {
                            self.expr(value, Some(reg))?;
                        }
                    }
                    None => {
                        self.emit(Op::LoadUndefined { dst: reg });
                    }
                }
                self.unwind_exits(0)?;
                self.emit(Op::Return { src: reg });
            }
            Stmt::Throw(value) => {
                let reg = self.expr(value, None)?;
                self.emit(Op::Throw { src: reg });
            }
            Stmt::Break(label) => self.jump_out(label.as_ref(), false)?,
            Stmt::Continue(label) => self.jump_out(label.as_ref(), true)?,
            Stmt::Block(body) => {
                let labels = std::mem::take(&mut self.f().pending_labels);
                if labels.is_empty() {
                    self.statements(body)?;
                } else {
                    // Bloc étiqueté : cible de `break étiquette`.
                    self.f().pending_labels = labels;
                    self.push_target(false, false);
                    self.statements(body)?;
                    self.pop_target(0);
                }
            }
            Stmt::With(object, body) => {
                self.push_scope();
                // L'objet est évalué avant que sa liaison ne soit visible.
                let reg = self.alloc()?;
                self.expr(object, Some(reg))?;
                self.emit(Op::ToObject { dst: reg, src: reg });
                let name: JsStr = Arc::from(format!("{WITH_PREFIX}{}", self.with_count));
                self.with_count += 1;
                self.declare_at(&name, BindKind::Var, true, reg);
                self.with_depth += 1;
                let result = self.statement(body);
                self.with_depth -= 1;
                self.pop_scope();
                result?;
            }
            Stmt::Labeled(label, body) => {
                self.f().pending_labels.push(Arc::clone(label));
                if matches!(
                    **body,
                    Stmt::For { .. }
                        | Stmt::ForIn { .. }
                        | Stmt::While { .. }
                        | Stmt::DoWhile { .. }
                        | Stmt::Block(_)
                        | Stmt::Labeled(..)
                        | Stmt::Switch { .. }
                ) {
                    self.statement(body)?;
                } else {
                    self.push_target(false, false);
                    self.statement(body)?;
                    self.pop_target(0);
                }
                self.f().pending_labels.clear();
            }
            Stmt::If {
                test,
                then,
                otherwise,
            } => {
                let mark = self.mark();
                let cond = self.expr(test, None)?;
                let to_else = self.emit(Op::JumpIfFalse {
                    src: cond,
                    target: 0,
                });
                self.free_to(mark);
                self.branch(then)?;
                match otherwise {
                    Some(otherwise) => {
                        let to_end = self.emit(Op::Jump { target: 0 });
                        self.patch(to_else);
                        self.branch(otherwise)?;
                        self.patch(to_end);
                    }
                    None => self.patch(to_else),
                }
            }
            Stmt::While { test, body } => {
                self.push_target(true, true);
                let head = self.here();
                let cond = self.expr(test, None)?;
                let exit = self.emit(Op::JumpIfFalse {
                    src: cond,
                    target: 0,
                });
                self.statement(body)?;
                self.emit(Op::Jump { target: head });
                self.patch(exit);
                self.pop_target(head);
            }
            Stmt::DoWhile { body, test } => {
                self.push_target(true, true);
                let head = self.here();
                self.statement(body)?;
                let continue_to = self.here();
                let cond = self.expr(test, None)?;
                self.emit(Op::JumpIfTrue {
                    src: cond,
                    target: head,
                });
                self.pop_target(continue_to);
            }
            Stmt::For {
                init,
                test,
                update,
                body,
            } => self.for_statement(init.as_ref(), test.as_ref(), update.as_ref(), body)?,
            Stmt::ForIn {
                target,
                object,
                body,
                of,
                is_await,
            } => self.for_in_statement(target, object, body, *of, *is_await)?,
            Stmt::Switch {
                discriminant,
                cases,
            } => self.switch_statement(discriminant, cases)?,
            Stmt::Try {
                block,
                param,
                handler,
                finalizer,
            } => {
                self.try_statement(
                    block,
                    param.as_ref(),
                    handler.as_deref(),
                    finalizer.as_deref(),
                )?;
            }
        }
        Ok(())
    }

    fn for_statement(
        &mut self,
        init: Option<&ForInit>,
        test: Option<&Expr>,
        update: Option<&Expr>,
        body: &Stmt,
    ) -> CResult<()> {
        // `for (using x = e; …; …)` : la ressource vit autant que la boucle.
        if let Some(ForInit::Decl(kind, decls)) = init
            && kind.is_using()
        {
            let labels = std::mem::take(&mut self.f().pending_labels);
            self.push_scope();
            for (pattern, _) in decls {
                self.declare_pattern(pattern, BindKind::Const)?;
            }
            let is_async = *kind == VarKind::AwaitUsing;
            self.with_disposal(is_async, |compiler| {
                compiler.var_declaration(*kind, decls)?;
                compiler.f().pending_labels = labels;
                compiler.for_statement(None, test, update, body)
            })?;
            self.pop_scope();
            return Ok(());
        }
        let labels = std::mem::take(&mut self.f().pending_labels);
        self.push_scope();
        match init {
            Some(ForInit::Decl(kind, decls)) => {
                if *kind != VarKind::Var {
                    let bind = if *kind != VarKind::Let {
                        BindKind::Const
                    } else {
                        BindKind::Let
                    };
                    for (pattern, _) in decls {
                        self.declare_pattern(pattern, bind)?;
                    }
                }
                self.var_declaration(*kind, decls)?;
            }
            Some(ForInit::Expr(expr)) => {
                let mark = self.mark();
                self.expr(expr, None)?;
                self.free_to(mark);
            }
            None => {}
        }
        let loop_scope_reg = self.f().scopes.last().map_or(0, |s| s.first_reg);
        self.f().pending_labels = labels;
        self.push_target(true, true);
        let head = self.here();
        let exit = match test {
            Some(test) => {
                let mark = self.mark();
                let cond = self.expr(test, None)?;
                let jump = self.emit(Op::JumpIfFalse {
                    src: cond,
                    target: 0,
                });
                self.free_to(mark);
                Some(jump)
            }
            None => None,
        };
        self.statement(body)?;
        let continue_to = self.here();
        // Une fermeture créée dans le corps garde la valeur de la variable de
        // boucle à ce tour : on ferme la cellule avant la mise à jour.
        let captured = self
            .f()
            .scopes
            .last()
            .is_some_and(|s| s.bindings.iter().any(|b| b.captured));
        if captured {
            self.emit(Op::CloseUpvalues {
                from: loop_scope_reg,
            });
        }
        if let Some(update) = update {
            let mark = self.mark();
            self.expr(update, None)?;
            self.free_to(mark);
        }
        self.emit(Op::Jump { target: head });
        if let Some(exit) = exit {
            self.patch(exit);
        }
        self.pop_target(continue_to);
        self.pop_scope();
        Ok(())
    }

    fn for_in_statement(
        &mut self,
        target: &ForTarget,
        object: &Expr,
        body: &Stmt,
        of: bool,
        is_await: bool,
    ) -> CResult<()> {
        let labels = std::mem::take(&mut self.f().pending_labels);
        let iter = self.alloc()?;
        // Les noms d'une déclaration lexicale sont en zone morte temporelle
        // pendant l'évaluation de l'objet parcouru.
        let dead_zone = match target {
            ForTarget::Decl(kind, pattern) if *kind != VarKind::Var => Some(pattern),
            _ => None,
        };
        if let Some(pattern) = dead_zone {
            self.push_scope();
            self.declare_pattern(pattern, BindKind::Let)?;
        }
        let source = self.expr(object, None)?;
        self.emit(if is_await {
            Op::AsyncIterStart {
                dst: iter,
                src: source,
            }
        } else if of {
            Op::IterStart {
                dst: iter,
                src: source,
            }
        } else {
            Op::ForInStart {
                dst: iter,
                obj: source,
            }
        });
        if dead_zone.is_some() {
            self.pop_scope();
        }
        self.free_to(iter + 1);
        self.f().pending_labels = labels;
        // `for…of` : l'itérateur est fermé si l'on quitte la boucle avant son
        // épuisement, par un saut ou par une exception.
        let closing = of && !is_await;
        let mut on_throw = None;
        if closing {
            let exception = self.alloc()?;
            let handler = self.emit(Op::PushHandler {
                target: 0,
                reg: exception,
            });
            self.f().exits.push(Exit::Handler);
            self.f().exits.push(Exit::IterClose(iter));
            on_throw = Some((handler, exception));
        }
        self.push_target(true, true);
        let head = self.here();
        let value = self.alloc()?;
        let exit = if is_await {
            self.async_iter_next(iter, value)?
        } else {
            self.emit(Op::IterNext {
                dst: value,
                iter,
                done: 0,
            })
        };
        // Une portée par tour : chaque itération a ses propres liaisons.
        self.push_scope();
        match target {
            ForTarget::Decl(kind, pattern) => {
                let init = *kind != VarKind::Var;
                if init {
                    let bind = if *kind != VarKind::Let {
                        BindKind::Const
                    } else {
                        BindKind::Let
                    };
                    self.declare_pattern(pattern, bind)?;
                }
                self.assign_pattern(pattern, value, init)?;
            }
            ForTarget::Pattern(pattern) => self.assign_pattern(pattern, value, false)?,
        }
        match target {
            ForTarget::Decl(kind, _) if kind.is_using() => {
                let is_async = *kind == VarKind::AwaitUsing;
                self.with_disposal(is_async, |compiler| {
                    if let Some(stack) = compiler.dispose_stack() {
                        compiler.emit(Op::UsingAdd {
                            stack,
                            src: value,
                            is_async,
                        });
                    }
                    compiler.statement(body)
                })?;
            }
            _ => self.statement(body)?,
        }
        self.pop_scope();
        self.emit(Op::Jump { target: head });
        let Some((handler, exception)) = on_throw else {
            self.patch(exit);
            self.pop_target(head);
            return Ok(());
        };
        // Arrivée d'un `break` : fermer l'itérateur.
        self.pop_target(head);
        self.f().exits.pop();
        self.f().exits.pop();
        self.emit(Op::PopHandler);
        self.emit(Op::IterClose { iter, quiet: false });
        let after_break = self.emit(Op::Jump { target: 0 });
        // Itérateur épuisé : rien à fermer.
        self.patch(exit);
        self.emit(Op::PopHandler);
        let after_done = self.emit(Op::Jump { target: 0 });
        // Exception levée dans la boucle : fermer, puis la relancer.
        self.patch(handler);
        self.emit(Op::IterClose { iter, quiet: true });
        self.emit(Op::Throw { src: exception });
        self.patch(after_break);
        self.patch(after_done);
        Ok(())
    }

    /// Un tour d'itération asynchrone : `value = (await iter.next()).value`.
    /// Renvoie le saut pris quand l'itérateur est épuisé, à compléter.
    fn async_iter_next(&mut self, iter: Reg, value: Reg) -> CResult<usize> {
        let mark = self.mark();
        let next = self.alloc()?;
        let name = self.string(&Arc::from("next"));
        self.emit(Op::GetProp {
            dst: next,
            obj: iter,
            name,
        });
        self.emit(Op::Call {
            dst: value,
            callee: next,
            this: iter,
            first: next,
            argc: 0,
        });
        self.emit(Op::Await {
            dst: value,
            src: value,
        });
        let done = self.string(&Arc::from("done"));
        self.emit(Op::GetProp {
            dst: next,
            obj: value,
            name: done,
        });
        let exit = self.emit(Op::JumpIfTrue {
            src: next,
            target: 0,
        });
        let name = self.string(&Arc::from("value"));
        self.emit(Op::GetProp {
            dst: value,
            obj: value,
            name,
        });
        self.free_to(mark);
        Ok(exit)
    }

    fn switch_statement(
        &mut self,
        discriminant: &Expr,
        cases: &[(Option<Expr>, Vec<Stmt>)],
    ) -> CResult<()> {
        let labels = std::mem::take(&mut self.f().pending_labels);
        let value = self.alloc()?;
        self.expr(discriminant, Some(value))?;
        self.f().pending_labels = labels;
        // Les clauses d'un `switch` partagent une seule portée.
        let all: Vec<Stmt> = cases
            .iter()
            .flat_map(|(_, body)| body.iter().cloned())
            .collect();
        let strict = self.f().strict;
        check_declarations(&all, false, &[], strict)?;
        self.push_target(false, true);
        self.push_scope();
        for (_, body) in cases {
            self.declare_block(body)?;
        }
        let mut entries = Vec::with_capacity(cases.len());
        let mut default_entry = None;
        for (i, (test, _)) in cases.iter().enumerate() {
            match test {
                Some(test) => {
                    let mark = self.mark();
                    let candidate = self.expr(test, None)?;
                    let equal = self.alloc()?;
                    self.emit(Op::Binary {
                        op: BinaryOp::StrictEq,
                        dst: equal,
                        left: value,
                        right: candidate,
                    });
                    entries.push(Some(self.emit(Op::JumpIfTrue {
                        src: equal,
                        target: 0,
                    })));
                    self.free_to(mark);
                }
                None => {
                    default_entry = Some(i);
                    entries.push(None);
                }
            }
        }
        let fallthrough = self.emit(Op::Jump { target: 0 });
        let mut default_target = None;
        for (i, (_, body)) in cases.iter().enumerate() {
            if let Some(Some(jump)) = entries.get(i) {
                self.patch(*jump);
            }
            if default_entry == Some(i) {
                default_target = Some(self.here());
            }
            for stmt in body {
                self.statement(stmt)?;
            }
        }
        match default_target {
            Some(target) => self.patch_to(fallthrough, target),
            None => self.patch(fallthrough),
        }
        self.pop_scope();
        self.pop_target(0);
        Ok(())
    }

    fn try_statement(
        &mut self,
        block: &[Stmt],
        param: Option<&Pattern>,
        handler: Option<&[Stmt]>,
        finalizer: Option<&[Stmt]>,
    ) -> CResult<()> {
        let finalizer = finalizer.map(|f| Arc::new(f.to_vec()));
        let exception = self.alloc()?;
        if let Some(finalizer) = &finalizer {
            self.f().exits.push(Exit::Finally(Arc::clone(finalizer)));
        }
        let to_catch = self.emit(Op::PushHandler {
            target: 0,
            reg: exception,
        });
        self.f().exits.push(Exit::Handler);
        self.statements(block)?;
        self.f().exits.pop();
        self.emit(Op::PopHandler);
        let after_try = self.emit(Op::Jump { target: 0 });

        // Arrivée d'une exception levée dans le bloc `try`.
        self.patch(to_catch);
        let mut rethrow = None;
        match handler {
            Some(handler) => {
                let guard = match &finalizer {
                    // Une exception levée dans `catch` doit encore exécuter `finally`.
                    Some(_) => {
                        let jump = self.emit(Op::PushHandler {
                            target: 0,
                            reg: exception,
                        });
                        self.f().exits.push(Exit::Handler);
                        Some(jump)
                    }
                    None => None,
                };
                self.push_scope();
                let mut caught = Vec::new();
                if let Some(param) = param {
                    pattern_names(param, &mut caught);
                    self.declare_pattern(param, BindKind::Let)?;
                    self.assign_pattern(param, exception, true)?;
                }
                // `catch (e) { function e() {} }` reste permis (annexe B).
                let lexical_only: Vec<Stmt> = handler
                    .iter()
                    .filter(|stmt| !matches!(stmt, Stmt::Function(_)))
                    .cloned()
                    .collect();
                let strict = self.f().strict;
                check_declarations(&lexical_only, false, &caught, strict)?;
                check_declarations(handler, false, &[], strict)?;
                self.declare_block(handler)?;
                for stmt in handler {
                    self.statement(stmt)?;
                }
                self.pop_scope();
                if let Some(guard) = guard {
                    self.f().exits.pop();
                    self.emit(Op::PopHandler);
                    rethrow = Some(guard);
                }
            }
            None => {
                // `try … finally` sans `catch` : exécuter `finally` puis relancer.
                self.f().exits.pop();
                if let Some(finalizer) = &finalizer {
                    self.statements(finalizer)?;
                }
                self.emit(Op::Throw { src: exception });
                self.patch(after_try);
                if let Some(finalizer) = &finalizer {
                    self.statements(finalizer)?;
                }
                return Ok(());
            }
        }
        self.patch(after_try);
        if let Some(finalizer) = &finalizer {
            self.f().exits.pop();
            self.statements(finalizer)?;
            if let Some(guard) = rethrow {
                let to_end = self.emit(Op::Jump { target: 0 });
                self.patch(guard);
                self.statements(finalizer)?;
                self.emit(Op::Throw { src: exception });
                self.patch(to_end);
            }
        }
        Ok(())
    }

    // ---- Motifs ------------------------------------------------------------

    /// Clé de propriété dans un registre.
    fn key_reg(&mut self, key: &PropKey) -> CResult<Reg> {
        let reg = self.alloc()?;
        match key {
            PropKey::Name(name) => {
                let index = self.string(name);
                self.emit(Op::LoadConst { dst: reg, index });
            }
            PropKey::Computed(expr) => {
                self.expr(expr, Some(reg))?;
                self.emit(Op::ToPropertyKey { dst: reg, src: reg });
            }
        }
        Ok(reg)
    }

    /// Affecte la valeur de `src` aux cibles de `pattern`.
    fn assign_pattern(&mut self, pattern: &Pattern, src: Reg, init: bool) -> CResult<()> {
        let mark = self.mark();
        match pattern {
            Pattern::Ident(name) => self.store(name, src, init)?,
            Pattern::Expr(Expr::Member { object, name, .. }) => {
                let obj = self.expr(object, None)?;
                let name = self.string(name);
                self.emit(Op::SetProp { obj, name, src });
            }
            Pattern::Expr(Expr::Index { object, index, .. }) => {
                let obj = self.expr(object, None)?;
                let key = self.expr(index, None)?;
                self.emit(Op::SetElem { obj, key, src });
            }
            Pattern::Expr(Expr::SuperMember(key)) => {
                let key = self.key_reg(key)?;
                self.emit(Op::SetSuper { key, src });
            }
            Pattern::Expr(_) => return error("cible d'affectation invalide"),
            Pattern::Object { props, rest } => {
                // Déstructurer `null` ou `undefined` est une erreur, même sans
                // aucune propriété à lire.
                let coercible = self.alloc()?;
                self.emit(Op::ToObject {
                    dst: coercible,
                    src,
                });
                self.free_to(coercible);
                let mut used_keys = Vec::new();
                for (key, target, default) in props {
                    let inner = self.mark();
                    let key = self.key_reg(key)?;
                    let value = self.alloc()?;
                    self.emit(Op::GetElem {
                        dst: value,
                        obj: src,
                        key,
                    });
                    self.default_and_assign(target, default.as_ref(), value, init)?;
                    if rest.is_some() {
                        // Les clés doivent survivre jusqu'au calcul du reste.
                        used_keys.push(key);
                        self.free_to(key + 1);
                    } else {
                        self.free_to(inner);
                    }
                }
                if let Some(rest) = rest {
                    let copy = self.alloc()?;
                    self.emit(Op::NewObject { dst: copy });
                    self.emit(Op::ObjectSpread { obj: copy, src });
                    let scratch = self.alloc()?;
                    for key in used_keys {
                        self.emit(Op::DeleteElem {
                            dst: scratch,
                            obj: copy,
                            key,
                        });
                    }
                    self.assign_pattern(rest, copy, init)?;
                }
            }
            Pattern::Array { elements, rest } => {
                let iter = self.alloc()?;
                self.emit(Op::IterStart { dst: iter, src });
                // Une exception pendant la déstructuration ferme l'itérateur.
                let exception = self.alloc()?;
                let handler = self.emit(Op::PushHandler {
                    target: 0,
                    reg: exception,
                });
                self.f().exits.push(Exit::Handler);
                for element in elements {
                    let inner = self.mark();
                    // Une cible `obj.x` ou `obj[k]` est évaluée avant de
                    // demander la valeur suivante à l'itérateur.
                    let reference = match element {
                        Some((Pattern::Expr(target), _)) => self.target_ref(target)?,
                        _ => None,
                    };
                    let value = self.alloc()?;
                    self.emit(Op::IterNextOrUndefined { dst: value, iter });
                    match (element, reference) {
                        (Some((_, default)), Some(reference)) => {
                            if let Some(default) = default {
                                let skip = self.emit(Op::JumpIfNotUndefined {
                                    src: value,
                                    target: 0,
                                });
                                self.expr(default, Some(value))?;
                                self.patch(skip);
                            }
                            self.store_target(reference, value);
                        }
                        (Some((target, default)), None) => {
                            self.default_and_assign(target, default.as_ref(), value, init)?;
                        }
                        (None, _) => {}
                    }
                    self.free_to(inner);
                }
                if let Some(rest) = rest {
                    let reference = match &**rest {
                        Pattern::Expr(target) => self.target_ref(target)?,
                        _ => None,
                    };
                    let value = self.alloc()?;
                    self.emit(Op::IterRest { dst: value, iter });
                    match reference {
                        Some(reference) => self.store_target(reference, value),
                        None => self.assign_pattern(rest, value, init)?,
                    }
                }
                self.f().exits.pop();
                self.emit(Op::PopHandler);
                self.emit(Op::IterClose { iter, quiet: false });
                let done = self.emit(Op::Jump { target: 0 });
                self.patch(handler);
                self.emit(Op::IterClose { iter, quiet: true });
                self.emit(Op::Throw { src: exception });
                self.patch(done);
            }
        }
        self.free_to(mark);
        Ok(())
    }

    /// Évalue la référence d'une cible d'affectation `obj.x`, `obj[k]` ou
    /// `super.x` ; `None` pour toute autre cible.
    fn target_ref(&mut self, target: &Expr) -> CResult<Option<TargetRef>> {
        Ok(match target {
            Expr::Member { object, name, .. } => {
                let obj = self.expr_to_temp(object)?;
                Some(TargetRef::Prop(obj, self.string(name)))
            }
            Expr::Index { object, index, .. } => {
                let obj = self.expr_to_temp(object)?;
                let key = self.expr_to_temp(index)?;
                Some(TargetRef::Elem(obj, key))
            }
            Expr::SuperMember(key) => Some(TargetRef::Super(self.key_reg(key)?)),
            _ => None,
        })
    }

    /// Écrit `src` dans la cible que [`Self::target_ref`] a évaluée.
    fn store_target(&mut self, reference: TargetRef, src: Reg) {
        self.emit(match reference {
            TargetRef::Prop(obj, name) => Op::SetProp { obj, name, src },
            TargetRef::Elem(obj, key) => Op::SetElem { obj, key, src },
            TargetRef::Super(key) => Op::SetSuper { key, src },
        });
    }

    fn default_and_assign(
        &mut self,
        target: &Pattern,
        default: Option<&Expr>,
        value: Reg,
        init: bool,
    ) -> CResult<()> {
        if let Some(default) = default {
            let skip = self.emit(Op::JumpIfNotUndefined {
                src: value,
                target: 0,
            });
            match target {
                Pattern::Ident(name) => self.expr_named(default, name, Some(value))?,
                _ => self.expr(default, Some(value))?,
            };
            self.patch(skip);
        }
        self.assign_pattern(target, value, init)
    }

    // ---- Expressions -------------------------------------------------------

    fn dst_or_alloc(&mut self, dst: Option<Reg>) -> CResult<Reg> {
        match dst {
            Some(dst) => Ok(dst),
            None => self.alloc(),
        }
    }

    /// Compile `expr` dans un registre temporaire qui n'est celui d'aucune variable.
    fn expr_to_temp(&mut self, expr: &Expr) -> CResult<Reg> {
        let reg = self.alloc()?;
        self.expr(expr, Some(reg))
    }

    /// Arguments d'un appel dans des registres consécutifs ; `Err(tableau)` s'il y a un `...`.
    fn arguments(&mut self, args: &[Expr]) -> CResult<Result<(Reg, u16), Reg>> {
        if args.iter().any(|a| matches!(a, Expr::Spread(_))) {
            let array = self.alloc()?;
            self.emit(Op::NewArray { dst: array });
            for arg in args {
                let mark = self.mark();
                match arg {
                    Expr::Spread(inner) => {
                        let src = self.expr(inner, None)?;
                        self.emit(Op::ArraySpread { array, src });
                    }
                    other => {
                        let src = self.expr(other, None)?;
                        self.emit(Op::ArrayPush { array, src });
                    }
                }
                self.free_to(mark);
            }
            return Ok(Err(array));
        }
        let first = self.mark();
        for arg in args {
            let reg = self.alloc()?;
            self.expr(arg, Some(reg))?;
            self.free_to(reg + 1);
        }
        Ok(Ok((first, u16::try_from(args.len()).unwrap_or(u16::MAX))))
    }

    /// Compile la fonction appelée et son receveur (`this`).
    fn callee(&mut self, callee: &Expr) -> CResult<(Reg, Reg)> {
        let this = self.alloc()?;
        let func = self.alloc()?;
        match callee {
            Expr::Member {
                object,
                name,
                optional,
            } => {
                self.expr(object, Some(this))?;
                if *optional {
                    self.optional_jump(this);
                }
                let name = self.string(name);
                self.emit(Op::GetProp {
                    dst: func,
                    obj: this,
                    name,
                });
            }
            Expr::Index {
                object,
                index,
                optional,
            } => {
                self.expr(object, Some(this))?;
                if *optional {
                    self.optional_jump(this);
                }
                let key = self.expr(index, None)?;
                self.emit(Op::GetElem {
                    dst: func,
                    obj: this,
                    key,
                });
            }
            Expr::SuperMember(key) => {
                self.emit(Op::LoadThis { dst: this });
                let key = self.key_reg(key)?;
                self.emit(Op::GetSuper { dst: func, key });
            }
            // Dans un `with`, une fonction trouvée sur l'objet est appelée
            // avec cet objet pour `this`.
            Expr::Ident(name) if !self.with_chain(name).is_empty() => {
                let chain = self.with_chain(name);
                let index = self.string(name);
                let mut ends = Vec::new();
                for with in &chain {
                    let obj = self.load_plain(with, None)?;
                    let flag = self.alloc()?;
                    self.emit(Op::WithHas {
                        dst: flag,
                        obj,
                        name: index,
                    });
                    let skip = self.emit(Op::JumpIfFalse {
                        src: flag,
                        target: 0,
                    });
                    self.emit(Op::GetProp {
                        dst: func,
                        obj,
                        name: index,
                    });
                    self.emit(Op::Move {
                        dst: this,
                        src: obj,
                    });
                    ends.push(self.emit(Op::Jump { target: 0 }));
                    self.patch(skip);
                    self.free_to(func + 1);
                }
                self.emit(Op::LoadUndefined { dst: this });
                self.load_plain(name, Some(func))?;
                for end in ends {
                    self.patch(end);
                }
            }
            other => {
                self.emit(Op::LoadUndefined { dst: this });
                self.expr(other, Some(func))?;
            }
        }
        self.free_to(func + 1);
        Ok((func, this))
    }

    /// Décrit, pour un appel `eval(…)`, tout ce que le code évalué pourra voir
    /// de l'appelant. Chaque liaison visible est marquée capturée : le code
    /// évalué y accède comme une fermeture.
    fn eval_site(&mut self) -> CResult<u16> {
        let current = self.funcs.len() - 1;
        let mut names: Vec<JsStr> = Vec::new();
        for state in self.funcs.iter().rev() {
            for scope in state.scopes.iter().rev() {
                for binding in scope.bindings.iter().rev() {
                    if !names.contains(&binding.name) {
                        names.push(Arc::clone(&binding.name));
                    }
                }
            }
        }
        let mut locals = Vec::new();
        let mut upvalues = Vec::new();
        for name in names {
            match self.resolve(&name)? {
                Resolved::Local { reg, kind, .. } => {
                    self.find_local(current, &name, true);
                    locals.push((name, reg, kind.code()));
                }
                Resolved::Upvalue { index, kind } => {
                    upvalues.push((name, index, kind.code()));
                }
                Resolved::Global => {}
            }
        }
        let in_method = self.funcs.iter().any(|f| {
            matches!(
                f.kind,
                FunctionKind::Method | FunctionKind::ClassConstructor { .. }
            )
        });
        let site = EvalSite {
            locals,
            upvalues,
            strict: self.f().strict,
            top_level: self.funcs.len() == 1 && self.f().is_script,
            in_method,
            in_field_init: self
                .funcs
                .iter()
                .rev()
                .find(|f| f.kind != FunctionKind::Arrow)
                .is_some_and(|f| f.field_init),
            params: self.f().in_params.clone(),
        };
        let index = u16::try_from(self.f().eval_sites.len()).unwrap_or(u16::MAX);
        self.f().eval_sites.push(Arc::new(site));
        Ok(index)
    }

    /// Émet un `yield` qui rend `src` ; la valeur de reprise arrive dans
    /// `dst`. Le chemin de retour qui suit sert à `return(v)` : il exécute les
    /// blocs `finally` englobants, ferme l'itérateur délégué `inner`, puis
    /// termine le générateur avec `v`.
    fn emit_yield(&mut self, dst: Reg, src: Reg, inner: Option<Reg>) -> CResult<()> {
        self.emit(Op::Yield { dst, src });
        let resume = self.emit(Op::Jump { target: 0 });
        if let Some(iter) = inner {
            self.emit(Op::IterClose { iter, quiet: false });
        }
        self.unwind_exits(0)?;
        self.emit(Op::Return { src: dst });
        self.patch(resume);
        Ok(())
    }

    /// Dans une chaîne optionnelle : saute à la fin si `src` est `null` ou `undefined`.
    fn optional_jump(&mut self, src: Reg) {
        let jump = self.emit(Op::JumpIfNullish { src, target: 0 });
        if let Some(chain) = self.f().optional_chains.last_mut() {
            chain.push(jump);
        }
    }

    fn expr(&mut self, expr: &Expr, dst: Option<Reg>) -> CResult<Reg> {
        match expr {
            Expr::BigInt(big) => {
                let dst = self.dst_or_alloc(dst)?;
                let f = self.f();
                f.constants.push(Value::BigInt(Arc::clone(big)));
                let index = u32::try_from(f.constants.len() - 1).unwrap_or(u32::MAX);
                self.emit(Op::LoadConst { dst, index });
                Ok(dst)
            }
            Expr::Number(n) => {
                let dst = self.dst_or_alloc(dst)?;
                let index = self.number(*n);
                self.emit(Op::LoadConst { dst, index });
                Ok(dst)
            }
            Expr::String(s) => {
                let dst = self.dst_or_alloc(dst)?;
                let index = self.string(s);
                self.emit(Op::LoadConst { dst, index });
                Ok(dst)
            }
            Expr::Bool(value) => {
                let dst = self.dst_or_alloc(dst)?;
                self.emit(Op::LoadBool { dst, value: *value });
                Ok(dst)
            }
            Expr::Null => {
                let dst = self.dst_or_alloc(dst)?;
                self.emit(Op::LoadNull { dst });
                Ok(dst)
            }
            Expr::Ident(name) => {
                if &**name == "undefined" && matches!(self.resolve(name)?, Resolved::Global) {
                    let dst = self.dst_or_alloc(dst)?;
                    self.emit(Op::LoadUndefined { dst });
                    return Ok(dst);
                }
                self.load(name, dst)
            }
            Expr::This => {
                let dst = self.dst_or_alloc(dst)?;
                self.emit(Op::LoadThis { dst });
                Ok(dst)
            }
            Expr::NewTarget => {
                let dst = self.dst_or_alloc(dst)?;
                self.emit(Op::LoadNewTarget { dst });
                Ok(dst)
            }
            Expr::ImportCall(specifier, options) => {
                let dst = self.dst_or_alloc(dst)?;
                let mark = self.mark();
                let src = self.alloc()?;
                self.expr(specifier, Some(src))?;
                if let Some(options) = options {
                    // Évaluées pour leurs effets ; les attributs sont ignorés.
                    let ignored = self.alloc()?;
                    self.expr(options, Some(ignored))?;
                }
                self.emit(Op::ImportCall { dst, src });
                self.free_to(mark);
                Ok(dst)
            }
            Expr::TaggedTemplate {
                tag,
                cooked,
                raw,
                exprs,
            } => {
                let dst = self.dst_or_alloc(dst)?;
                let mark = self.mark();
                let (func, this) = self.callee(tag)?;
                let index = u32::try_from(self.f().templates.len()).unwrap_or(u32::MAX);
                self.f().templates.push((cooked.clone(), raw.clone()));
                // Arguments : l'objet des morceaux, puis les substitutions.
                let first = self.alloc()?;
                self.emit(Op::TemplateObject { dst: first, index });
                for expr in exprs {
                    let reg = self.alloc()?;
                    self.expr(expr, Some(reg))?;
                    self.free_to(reg + 1);
                }
                self.emit(Op::Call {
                    dst,
                    callee: func,
                    this,
                    first,
                    argc: u16::try_from(exprs.len() + 1).unwrap_or(u16::MAX),
                });
                self.free_to(mark);
                Ok(dst)
            }
            Expr::Template { quasis, exprs } => {
                let dst = self.dst_or_alloc(dst)?;
                let mark = self.mark();
                let acc = self.alloc()?;
                let first = quasis.first().cloned().unwrap_or_else(|| Arc::from(""));
                let index = self.string(&first);
                self.emit(Op::LoadConst { dst: acc, index });
                for (i, part) in exprs.iter().enumerate() {
                    let inner = self.mark();
                    let value = self.expr_to_temp(part)?;
                    self.emit(Op::ToString {
                        dst: value,
                        src: value,
                    });
                    self.emit(Op::Binary {
                        op: BinaryOp::Add,
                        dst: acc,
                        left: acc,
                        right: value,
                    });
                    if let Some(quasi) = quasis.get(i + 1).filter(|q| !q.is_empty()) {
                        let index = self.string(quasi);
                        self.emit(Op::LoadConst { dst: value, index });
                        self.emit(Op::Binary {
                            op: BinaryOp::Add,
                            dst: acc,
                            left: acc,
                            right: value,
                        });
                    }
                    self.free_to(inner);
                }
                self.emit(Op::Move { dst, src: acc });
                self.free_to(mark);
                Ok(dst)
            }
            Expr::Regex { pattern, flags } => {
                let dst = self.dst_or_alloc(dst)?;
                let (pattern, flags) = (self.string(pattern), self.string(flags));
                self.emit(Op::RegExp {
                    dst,
                    pattern,
                    flags,
                });
                Ok(dst)
            }
            Expr::Array(items) => {
                let dst = self.dst_or_alloc(dst)?;
                let mark = self.mark();
                let array = self.alloc()?;
                self.emit(Op::NewArray { dst: array });
                for item in items {
                    let inner = self.mark();
                    match item {
                        None => {
                            self.emit(Op::ArrayHole { array });
                        }
                        Some(Expr::Spread(inner_expr)) => {
                            let src = self.expr(inner_expr, None)?;
                            self.emit(Op::ArraySpread { array, src });
                        }
                        Some(other) => {
                            let src = self.expr(other, None)?;
                            self.emit(Op::ArrayPush { array, src });
                        }
                    }
                    self.free_to(inner);
                }
                self.emit(Op::Move { dst, src: array });
                self.free_to(mark);
                Ok(dst)
            }
            Expr::Object(props) => self.object_literal(props, dst),
            Expr::Function(func) => {
                let dst = self.dst_or_alloc(dst)?;
                let kind = if func.is_arrow {
                    FunctionKind::Arrow
                } else if func.is_method {
                    FunctionKind::Method
                } else {
                    FunctionKind::Normal
                };
                let index = self.function(func, kind, !func.is_arrow && !func.is_method)?;
                self.emit(Op::Closure { dst, index });
                Ok(dst)
            }
            Expr::Class(class) => self.class(class, dst),
            Expr::Yield { arg, delegate } => {
                let dst = self.dst_or_alloc(dst)?;
                let mark = self.mark();
                let value = match arg {
                    Some(arg) => self.expr(arg, None)?,
                    None => {
                        let reg = self.alloc()?;
                        self.emit(Op::LoadUndefined { dst: reg });
                        reg
                    }
                };
                if *delegate && self.f().is_async {
                    // `yield* it` dans un générateur async : comme ci-dessous,
                    // mais chaque résultat de l'itérateur délégué est attendu.
                    let base = self.alloc()?;
                    for _ in 0..6 {
                        self.alloc()?;
                    }
                    let (received, mode, result) = (base + 2, base + 3, base + 4);
                    let (done, returning) = (base + 5, base + 6);
                    self.emit(Op::DelegateStartAsync { base, src: value });
                    self.emit(Op::LoadUndefined { dst: received });
                    let modes = [0.0, 1.0, 2.0].map(|mode| self.number(mode));
                    self.emit(Op::LoadConst {
                        dst: mode,
                        index: modes[0],
                    });
                    let head = self.here();
                    self.emit(Op::DelegateCall { base });
                    // `return` absent : la valeur reçue est attendue puis rendue.
                    let no_return = self.emit(Op::JumpIfTrue {
                        src: returning,
                        target: 0,
                    });
                    self.emit(Op::Await {
                        dst: result,
                        src: result,
                    });
                    self.emit(Op::DelegateFinish { base });
                    let to_return = self.emit(Op::JumpIfTrue {
                        src: returning,
                        target: 0,
                    });
                    let to_done = self.emit(Op::JumpIfTrue {
                        src: done,
                        target: 0,
                    });
                    let handler = self.emit(Op::PushHandler {
                        target: 0,
                        reg: received,
                    });
                    self.emit(Op::Yield {
                        dst: received,
                        src: result,
                    });
                    let resumed = self.emit(Op::Jump { target: 0 });
                    self.emit(Op::PopHandler);
                    self.emit(Op::LoadConst {
                        dst: mode,
                        index: modes[2],
                    });
                    self.emit(Op::Jump { target: head });
                    self.patch(resumed);
                    self.emit(Op::PopHandler);
                    self.emit(Op::LoadConst {
                        dst: mode,
                        index: modes[0],
                    });
                    self.emit(Op::Jump { target: head });
                    self.patch(handler);
                    self.emit(Op::LoadConst {
                        dst: mode,
                        index: modes[1],
                    });
                    self.emit(Op::Jump { target: head });
                    self.patch(no_return);
                    self.emit(Op::Await {
                        dst: result,
                        src: result,
                    });
                    self.patch(to_return);
                    self.unwind_exits(0)?;
                    self.emit(Op::Return { src: result });
                    self.patch(to_done);
                    self.emit(Op::Move { dst, src: result });
                } else if *delegate {
                    // `yield* it` : chaque reprise du générateur (`next`,
                    // `throw`, `return`) est transmise à l'itérateur délégué.
                    let base = self.alloc()?;
                    for _ in 0..6 {
                        self.alloc()?;
                    }
                    let (received, mode, result) = (base + 2, base + 3, base + 4);
                    let (done, returning) = (base + 5, base + 6);
                    self.emit(Op::DelegateStart { base, src: value });
                    self.emit(Op::LoadUndefined { dst: received });
                    let modes = [0.0, 1.0, 2.0].map(|mode| self.number(mode));
                    self.emit(Op::LoadConst {
                        dst: mode,
                        index: modes[0],
                    });
                    let head = self.here();
                    self.emit(Op::DelegateStep { base });
                    let to_return = self.emit(Op::JumpIfTrue {
                        src: returning,
                        target: 0,
                    });
                    let to_done = self.emit(Op::JumpIfTrue {
                        src: done,
                        target: 0,
                    });
                    // Une exception arrivant au `yield` est un `throw()`.
                    let handler = self.emit(Op::PushHandler {
                        target: 0,
                        reg: received,
                    });
                    self.emit(Op::Yield {
                        dst: received,
                        src: result,
                    });
                    let resumed = self.emit(Op::Jump { target: 0 });
                    // Reprise par `return(v)` (voir `Vm::resume`).
                    self.emit(Op::PopHandler);
                    self.emit(Op::LoadConst {
                        dst: mode,
                        index: modes[2],
                    });
                    self.emit(Op::Jump { target: head });
                    // Reprise par `next(v)`.
                    self.patch(resumed);
                    self.emit(Op::PopHandler);
                    self.emit(Op::LoadConst {
                        dst: mode,
                        index: modes[0],
                    });
                    self.emit(Op::Jump { target: head });
                    // Reprise par `throw(e)`.
                    self.patch(handler);
                    self.emit(Op::LoadConst {
                        dst: mode,
                        index: modes[1],
                    });
                    self.emit(Op::Jump { target: head });
                    // Le délégué a honoré `return` : le générateur se termine.
                    self.patch(to_return);
                    self.unwind_exits(0)?;
                    self.emit(Op::Return { src: result });
                    // Le délégué est épuisé : sa valeur finale est celle de `yield*`.
                    self.patch(to_done);
                    self.emit(Op::Move { dst, src: result });
                } else {
                    self.emit_yield(dst, value, None)?;
                }
                self.free_to(mark);
                Ok(dst)
            }
            Expr::PrivateName(name) => self.load(name, dst),
            Expr::PrivateInit { key, binding, kind } => {
                let dst = self.dst_or_alloc(dst)?;
                let mark = self.mark();
                let obj = self.alloc()?;
                self.emit(Op::LoadThis { dst: obj });
                let key = self.load(key, None)?;
                let func = self.load(binding, None)?;
                self.emit(Op::InitPrivate {
                    obj,
                    key,
                    func,
                    kind: *kind,
                });
                self.emit(Op::LoadUndefined { dst });
                self.free_to(mark);
                Ok(dst)
            }
            Expr::FieldInit { key, value } => {
                let dst = self.dst_or_alloc(dst)?;
                let mark = self.mark();
                let object = self.alloc()?;
                self.emit(Op::LoadThis { dst: object });
                let key_reg = self.alloc()?;
                match key {
                    PropKey::Name(name) if name.starts_with('#') => {
                        self.load(name, Some(key_reg))?;
                    }
                    PropKey::Name(name) => {
                        let index = self.string(name);
                        self.emit(Op::LoadConst {
                            dst: key_reg,
                            index,
                        });
                    }
                    PropKey::Computed(expr) => {
                        self.expr(expr, Some(key_reg))?;
                        self.emit(Op::ToPropertyKey {
                            dst: key_reg,
                            src: key_reg,
                        });
                    }
                }
                // Une fonction anonyme prend le nom du champ.
                match key {
                    PropKey::Name(name) => self.expr_named(value, name, Some(dst))?,
                    PropKey::Computed(_) => self.expr(value, Some(dst))?,
                };
                self.emit(Op::DefineField {
                    obj: object,
                    key: key_reg,
                    src: dst,
                });
                self.free_to(mark);
                Ok(dst)
            }
            Expr::Await(arg) => {
                let dst = self.dst_or_alloc(dst)?;
                let mark = self.mark();
                let value = self.expr(arg, None)?;
                self.emit(Op::Await { dst, src: value });
                self.free_to(mark);
                Ok(dst)
            }
            Expr::Unary { op, arg } => self.unary(*op, arg, dst),
            Expr::Update {
                increment,
                prefix,
                target,
            } => self.update(*increment, *prefix, target, dst),
            Expr::Binary { op, left, right } => {
                let dst = self.dst_or_alloc(dst)?;
                let mark = self.mark();
                // L'opérande gauche est figé avant d'évaluer le droit, qui peut le modifier.
                let l = if is_pure(right) {
                    self.expr(left, None)?
                } else {
                    self.expr_to_temp(left)?
                };
                let r = self.expr(right, None)?;
                self.emit(Op::Binary {
                    op: *op,
                    dst,
                    left: l,
                    right: r,
                });
                self.free_to(mark);
                Ok(dst)
            }
            Expr::Logical { op, left, right } => {
                let dst = self.dst_or_alloc(dst)?;
                self.expr(left, Some(dst))?;
                let skip = self.emit(match op {
                    LogicalOp::And => Op::JumpIfFalse {
                        src: dst,
                        target: 0,
                    },
                    LogicalOp::Or => Op::JumpIfTrue {
                        src: dst,
                        target: 0,
                    },
                    LogicalOp::Nullish => Op::JumpIfNotNullish {
                        src: dst,
                        target: 0,
                    },
                });
                self.expr(right, Some(dst))?;
                self.patch(skip);
                Ok(dst)
            }
            Expr::Cond {
                test,
                then,
                otherwise,
            } => {
                let dst = self.dst_or_alloc(dst)?;
                let mark = self.mark();
                let cond = self.expr(test, None)?;
                let to_else = self.emit(Op::JumpIfFalse {
                    src: cond,
                    target: 0,
                });
                self.free_to(mark);
                self.expr(then, Some(dst))?;
                let to_end = self.emit(Op::Jump { target: 0 });
                self.patch(to_else);
                self.expr(otherwise, Some(dst))?;
                self.patch(to_end);
                Ok(dst)
            }
            Expr::Assign { op, target, value } => self.assign(*op, target, value, dst),
            Expr::Sequence(items) => {
                let dst = self.dst_or_alloc(dst)?;
                for (i, item) in items.iter().enumerate() {
                    let mark = self.mark();
                    if i + 1 == items.len() {
                        self.expr(item, Some(dst))?;
                    } else {
                        self.expr(item, None)?;
                    }
                    self.free_to(mark);
                }
                Ok(dst)
            }
            Expr::Member {
                object,
                name,
                optional,
            } => {
                let dst = self.dst_or_alloc(dst)?;
                let mark = self.mark();
                let obj = self.expr(object, None)?;
                if *optional {
                    self.optional_jump(obj);
                }
                let name = self.string(name);
                self.emit(Op::GetProp { dst, obj, name });
                self.free_to(mark);
                Ok(dst)
            }
            Expr::Index {
                object,
                index,
                optional,
            } => {
                let dst = self.dst_or_alloc(dst)?;
                let mark = self.mark();
                let obj = if is_pure(index) {
                    self.expr(object, None)?
                } else {
                    self.expr_to_temp(object)?
                };
                if *optional {
                    self.optional_jump(obj);
                }
                let key = self.expr(index, None)?;
                self.emit(Op::GetElem { dst, obj, key });
                self.free_to(mark);
                Ok(dst)
            }
            Expr::OptionalChain(inner) => {
                let dst = self.dst_or_alloc(dst)?;
                self.f().optional_chains.push(Vec::new());
                let result = self.expr(inner, Some(dst));
                let jumps = self.f().optional_chains.pop().unwrap_or_default();
                result?;
                let to_end = self.emit(Op::Jump { target: 0 });
                for jump in jumps {
                    self.patch(jump);
                }
                self.emit(Op::LoadUndefined { dst });
                self.patch(to_end);
                Ok(dst)
            }
            Expr::Call {
                callee,
                args,
                optional,
            } => {
                let dst = self.dst_or_alloc(dst)?;
                let mark = self.mark();
                let (func, this) = self.callee(callee)?;
                if *optional {
                    self.optional_jump(func);
                }
                let direct_eval =
                    !*optional && matches!(&**callee, Expr::Ident(name) if &**name == "eval");
                let arguments = self.arguments(args)?;
                match arguments {
                    Ok((first, argc)) if direct_eval => {
                        let site = self.eval_site()?;
                        self.emit(Op::DirectEval {
                            dst,
                            callee: func,
                            first,
                            argc,
                            site,
                        })
                    }
                    Ok((first, argc)) => self.emit(Op::Call {
                        dst,
                        callee: func,
                        this,
                        first,
                        argc,
                    }),
                    Err(array) => self.emit(Op::CallSpread {
                        dst,
                        callee: func,
                        this,
                        args: array,
                    }),
                };
                self.free_to(mark);
                Ok(dst)
            }
            Expr::New { callee, args } => {
                let dst = self.dst_or_alloc(dst)?;
                let mark = self.mark();
                let func = self.expr_to_temp(callee)?;
                match self.arguments(args)? {
                    Ok((first, argc)) => self.emit(Op::New {
                        dst,
                        callee: func,
                        first,
                        argc,
                    }),
                    Err(array) => self.emit(Op::NewSpread {
                        dst,
                        callee: func,
                        args: array,
                    }),
                };
                self.free_to(mark);
                Ok(dst)
            }
            Expr::SuperCall(args) => {
                let dst = self.dst_or_alloc(dst)?;
                let mark = self.mark();
                let array = self.alloc()?;
                // Constructeur implicite : les arguments reçus sont transmis
                // tels quels, sans passer par le protocole d'itération.
                let forwarded = match args.as_slice() {
                    [Expr::Spread(inner)] => {
                        matches!(&**inner, Expr::Ident(name) if &**name == FORWARDED_ARGS)
                            .then_some(inner)
                    }
                    _ => None,
                };
                if let Some(inner) = forwarded {
                    self.expr(inner, Some(array))?;
                } else {
                    self.emit(Op::NewArray { dst: array });
                }
                for arg in args.iter().filter(|_| forwarded.is_none()) {
                    let inner = self.mark();
                    match arg {
                        Expr::Spread(inner_expr) => {
                            let src = self.expr(inner_expr, None)?;
                            self.emit(Op::ArraySpread { array, src });
                        }
                        other => {
                            let src = self.expr(other, None)?;
                            self.emit(Op::ArrayPush { array, src });
                        }
                    }
                    self.free_to(inner);
                }
                self.emit(Op::SuperCall { dst, args: array });
                self.free_to(mark);
                Ok(dst)
            }
            Expr::SuperMember(key) => {
                let dst = self.dst_or_alloc(dst)?;
                let mark = self.mark();
                let key = self.key_reg(key)?;
                self.emit(Op::GetSuper { dst, key });
                self.free_to(mark);
                Ok(dst)
            }
            Expr::Spread(_) => error("« ... » inattendu"),
        }
    }

    fn object_literal(&mut self, props: &[crate::ast::Property], dst: Option<Reg>) -> CResult<Reg> {
        let dst = self.dst_or_alloc(dst)?;
        let mark = self.mark();
        let obj = self.alloc()?;
        self.emit(Op::NewObject { dst: obj });
        for prop in props {
            let inner = self.mark();
            match &prop.kind {
                PropKind::Spread(value) => {
                    let src = self.expr(value, None)?;
                    self.emit(Op::ObjectSpread { obj, src });
                }
                PropKind::Proto(value) => {
                    let src = self.expr(value, None)?;
                    self.emit(Op::SetProto { obj, src });
                }
                PropKind::Value(value) => {
                    let is_method = matches!(value, Expr::Function(f) if f.is_method);
                    match &prop.key {
                        PropKey::Name(name) => {
                            let src = self.alloc()?;
                            self.expr_named(value, name, Some(src))?;
                            if is_method {
                                self.emit(Op::SetHome {
                                    func: src,
                                    home: obj,
                                });
                            }
                            let name = self.string(name);
                            self.emit(Op::InitProp { obj, name, src });
                        }
                        PropKey::Computed(_) => {
                            let key = self.key_reg(&prop.key)?;
                            let src = self.expr_to_temp(value)?;
                            // Une fonction anonyme prend le nom de sa clé.
                            let anonymous = match value {
                                Expr::Function(func) => func.name.is_none(),
                                Expr::Class(class) => class.name.is_none(),
                                _ => false,
                            };
                            if anonymous {
                                self.emit(Op::NameMember { func: src, key });
                            }
                            if is_method {
                                self.emit(Op::SetHome {
                                    func: src,
                                    home: obj,
                                });
                            }
                            self.emit(Op::InitElem { obj, key, src });
                        }
                    }
                }
                PropKind::Getter(func) | PropKind::Setter(func) => {
                    let key = self.key_reg(&prop.key)?;
                    let value = self.alloc()?;
                    let index = self.function(func, FunctionKind::Method, false)?;
                    self.emit(Op::Closure { dst: value, index });
                    self.emit(Op::SetHome {
                        func: value,
                        home: obj,
                    });
                    self.emit(if matches!(prop.kind, PropKind::Getter(_)) {
                        Op::InitGetter {
                            obj,
                            key,
                            func: value,
                            enumerable: true,
                        }
                    } else {
                        Op::InitSetter {
                            obj,
                            key,
                            func: value,
                            enumerable: true,
                        }
                    });
                }
            }
            self.free_to(inner);
        }
        self.emit(Op::Move { dst, src: obj });
        self.free_to(mark);
        Ok(dst)
    }

    fn unary(&mut self, op: UnaryOp, arg: &Expr, dst: Option<Reg>) -> CResult<Reg> {
        let dst = self.dst_or_alloc(dst)?;
        let mark = self.mark();
        match op {
            UnaryOp::Void => {
                self.expr(arg, None)?;
                self.emit(Op::LoadUndefined { dst });
            }
            UnaryOp::Delete => match arg {
                Expr::Member { object, name, .. } => {
                    let obj = self.expr(object, None)?;
                    let name = self.string(name);
                    self.emit(Op::DeleteProp { dst, obj, name });
                }
                Expr::Index { object, index, .. } => {
                    let obj = self.expr_to_temp(object)?;
                    let key = self.expr(index, None)?;
                    self.emit(Op::DeleteElem { dst, obj, key });
                }
                Expr::Ident(name) => {
                    // Sous `with` (ou après un `eval` qui a créé la
                    // variable), la liaison est une propriété d'objet.
                    let base = self.with_base(name)?;
                    let index = self.string(name);
                    let on_object = base.map(|base| {
                        self.emit(Op::JumpIfNotUndefined {
                            src: base,
                            target: 0,
                        })
                    });
                    match self.resolve(name)? {
                        Resolved::Global => {
                            self.emit(Op::DeleteGlobal { dst, name: index });
                        }
                        // Une liaison déclarative ne se supprime pas.
                        _ => {
                            self.emit(Op::LoadBool { dst, value: false });
                        }
                    }
                    if let (Some(base), Some(on_object)) = (base, on_object) {
                        let end = self.emit(Op::Jump { target: 0 });
                        self.patch(on_object);
                        self.emit(Op::DeleteProp {
                            dst,
                            obj: base,
                            name: index,
                        });
                        self.patch(end);
                    }
                }
                Expr::SuperMember(key) => {
                    // `delete super.x` : la clé est évaluée, puis c'est une erreur.
                    if let PropKey::Computed(expr) = key {
                        self.expr(expr, None)?;
                    }
                    let message =
                        self.string(&Arc::from("« delete super.propriété » est interdit"));
                    self.emit(Op::ThrowReference { message });
                }
                other => {
                    self.expr(other, None)?;
                    self.emit(Op::LoadBool { dst, value: true });
                }
            },
            UnaryOp::Typeof => {
                // `typeof nomInconnu` ne lève pas d'erreur.
                if let Expr::Ident(name) = arg
                    && matches!(self.resolve(name)?, Resolved::Global)
                {
                    // Dans un `with`, l'objet peut fournir la variable.
                    let chain = self.with_chain(name);
                    let index = self.string(name);
                    let mark = self.mark();
                    let mut ends = Vec::new();
                    for with in &chain {
                        let obj = self.load_plain(with, None)?;
                        let flag = self.alloc()?;
                        self.emit(Op::WithHas {
                            dst: flag,
                            obj,
                            name: index,
                        });
                        let skip = self.emit(Op::JumpIfFalse {
                            src: flag,
                            target: 0,
                        });
                        self.emit(Op::GetProp {
                            dst: flag,
                            obj,
                            name: index,
                        });
                        self.emit(Op::Unary {
                            op: UnaryKind::Typeof,
                            dst,
                            src: flag,
                        });
                        ends.push(self.emit(Op::Jump { target: 0 }));
                        self.patch(skip);
                        self.free_to(mark);
                    }
                    self.emit(Op::TypeofGlobal { dst, name: index });
                    for end in ends {
                        self.patch(end);
                    }
                } else {
                    let src = self.expr(arg, None)?;
                    self.emit(Op::Unary {
                        op: UnaryKind::Typeof,
                        dst,
                        src,
                    });
                }
            }
            UnaryOp::Neg | UnaryOp::Plus | UnaryOp::Not | UnaryOp::BitNot => {
                let src = self.expr(arg, None)?;
                let kind = match op {
                    UnaryOp::Neg => UnaryKind::Neg,
                    UnaryOp::Plus => UnaryKind::Plus,
                    UnaryOp::Not => UnaryKind::Not,
                    _ => UnaryKind::BitNot,
                };
                self.emit(Op::Unary { op: kind, dst, src });
            }
        }
        self.free_to(mark);
        Ok(dst)
    }

    fn update(
        &mut self,
        increment: bool,
        prefix: bool,
        target: &Expr,
        dst: Option<Reg>,
    ) -> CResult<Reg> {
        let dst = self.dst_or_alloc(dst)?;
        let mark = self.mark();
        let (old, new) = (self.alloc()?, self.alloc()?);
        match target {
            Expr::Ident(name) => {
                let base = self.with_base(name)?;
                let current = self.load_ref(name, base, None)?;
                self.emit(Op::ToNumber {
                    dst: old,
                    src: current,
                });
                self.emit(Op::Step {
                    dst: new,
                    src: old,
                    increment,
                });
                self.store_ref(name, base, new)?;
            }
            Expr::Member { object, name, .. } => {
                let obj = self.expr_to_temp(object)?;
                let name = self.string(name);
                self.emit(Op::GetProp {
                    dst: old,
                    obj,
                    name,
                });
                self.emit(Op::ToNumber { dst: old, src: old });
                self.emit(Op::Step {
                    dst: new,
                    src: old,
                    increment,
                });
                self.emit(Op::SetProp {
                    obj,
                    name,
                    src: new,
                });
            }
            Expr::Index { object, index, .. } => {
                let obj = self.expr_to_temp(object)?;
                let key = self.expr_to_temp(index)?;
                self.emit(Op::KeyOnce { obj, key });
                self.emit(Op::GetElem { dst: old, obj, key });
                self.emit(Op::ToNumber { dst: old, src: old });
                self.emit(Op::Step {
                    dst: new,
                    src: old,
                    increment,
                });
                self.emit(Op::SetElem { obj, key, src: new });
            }
            Expr::SuperMember(key) => {
                let key = self.key_reg(key)?;
                self.emit(Op::GetSuper { dst: old, key });
                self.emit(Op::ToNumber { dst: old, src: old });
                self.emit(Op::Step {
                    dst: new,
                    src: old,
                    increment,
                });
                self.emit(Op::SetSuper { key, src: new });
            }
            _ => return error("cible d'incrémentation invalide"),
        }
        self.emit(Op::Move {
            dst,
            src: if prefix { new } else { old },
        });
        self.free_to(mark);
        Ok(dst)
    }

    fn assign(
        &mut self,
        op: AssignOp,
        target: &Pattern,
        value: &Expr,
        dst: Option<Reg>,
    ) -> CResult<Reg> {
        let dst = self.dst_or_alloc(dst)?;
        let mark = self.mark();
        match (op, target) {
            (AssignOp::Assign, Pattern::Ident(name)) => {
                let base = self.with_base(name)?;
                let src = self.alloc()?;
                self.expr_named(value, name, Some(src))?;
                self.store_ref(name, base, src)?;
                self.emit(Op::Move { dst, src });
            }
            (AssignOp::Assign, Pattern::Expr(Expr::Member { object, name, .. })) => {
                let obj = self.expr_to_temp(object)?;
                let src = self.expr_to_temp(value)?;
                let name = self.string(name);
                self.emit(Op::SetProp { obj, name, src });
                self.emit(Op::Move { dst, src });
            }
            (AssignOp::Assign, Pattern::Expr(Expr::Index { object, index, .. })) => {
                let obj = self.expr_to_temp(object)?;
                let key = self.expr_to_temp(index)?;
                let src = self.expr_to_temp(value)?;
                self.emit(Op::SetElem { obj, key, src });
                self.emit(Op::Move { dst, src });
            }
            (AssignOp::Assign, Pattern::Expr(Expr::SuperMember(key))) => {
                let key = self.key_reg(key)?;
                let src = self.expr_to_temp(value)?;
                self.emit(Op::SetSuper { key, src });
                self.emit(Op::Move { dst, src });
            }
            (AssignOp::Assign, pattern) => {
                let src = self.expr_to_temp(value)?;
                self.assign_pattern(pattern, src, false)?;
                self.emit(Op::Move { dst, src });
            }
            (_, Pattern::Expr(Expr::SuperMember(key))) => {
                let key = self.key_reg(key)?;
                let current = self.alloc()?;
                self.emit(Op::GetSuper { dst: current, key });
                let skip = self.compound(op, current, value, None)?;
                self.emit(Op::SetSuper { key, src: current });
                if let Some(skip) = skip {
                    self.patch(skip);
                }
                self.emit(Op::Move { dst, src: current });
            }
            (_, Pattern::Ident(name)) => {
                let base = self.with_base(name)?;
                let current = self.alloc()?;
                self.load_ref(name, base, Some(current))?;
                let skip = self.compound(op, current, value, Some(name))?;
                self.store_ref(name, base, current)?;
                if let Some(skip) = skip {
                    self.patch(skip);
                }
                self.emit(Op::Move { dst, src: current });
            }
            (_, Pattern::Expr(Expr::Member { object, name, .. })) => {
                let obj = self.expr_to_temp(object)?;
                let name = self.string(name);
                let current = self.alloc()?;
                self.emit(Op::GetProp {
                    dst: current,
                    obj,
                    name,
                });
                let skip = self.compound(op, current, value, None)?;
                self.emit(Op::SetProp {
                    obj,
                    name,
                    src: current,
                });
                if let Some(skip) = skip {
                    self.patch(skip);
                }
                self.emit(Op::Move { dst, src: current });
            }
            (_, Pattern::Expr(Expr::Index { object, index, .. })) => {
                let obj = self.expr_to_temp(object)?;
                let key = self.expr_to_temp(index)?;
                self.emit(Op::KeyOnce { obj, key });
                let current = self.alloc()?;
                self.emit(Op::GetElem {
                    dst: current,
                    obj,
                    key,
                });
                let skip = self.compound(op, current, value, None)?;
                self.emit(Op::SetElem {
                    obj,
                    key,
                    src: current,
                });
                if let Some(skip) = skip {
                    self.patch(skip);
                }
                self.emit(Op::Move { dst, src: current });
            }
            _ => return error("cible d'affectation invalide"),
        }
        self.free_to(mark);
        Ok(dst)
    }

    /// Applique `current op= value` dans `current`. Pour un opérateur logique,
    /// renvoie le saut à patcher pour éviter l'écriture quand il court-circuite.
    /// `name` : identifiant affecté, qui nomme une fonction anonyme.
    fn compound(
        &mut self,
        op: AssignOp,
        current: Reg,
        value: &Expr,
        name: Option<&JsStr>,
    ) -> CResult<Option<usize>> {
        match op {
            AssignOp::Binary(op) => {
                let right = self.expr(value, None)?;
                self.emit(Op::Binary {
                    op,
                    dst: current,
                    left: current,
                    right,
                });
                Ok(None)
            }
            AssignOp::Logical(op) => {
                let skip = self.emit(match op {
                    LogicalOp::And => Op::JumpIfFalse {
                        src: current,
                        target: 0,
                    },
                    LogicalOp::Or => Op::JumpIfTrue {
                        src: current,
                        target: 0,
                    },
                    LogicalOp::Nullish => Op::JumpIfNotNullish {
                        src: current,
                        target: 0,
                    },
                });
                match name {
                    Some(name) => self.expr_named(value, name, Some(current))?,
                    None => self.expr(value, Some(current))?,
                };
                Ok(Some(skip))
            }
            AssignOp::Assign => Ok(None),
        }
    }
}
