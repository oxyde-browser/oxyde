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
use crate::bytecode::{FunctionKind, FunctionProto, Op, Reg, UnaryKind, UpvalueDesc};
use crate::lexer::SyntaxError;
use crate::value::{JsStr, Value};

type CResult<T> = Result<T, SyntaxError>;

/// Préfixe du nom de la liaison cachée qui porte l'objet d'un `with` ; ce
/// n'est pas un identifiant, aucun script ne peut la désigner.
const WITH_PREFIX: &str = "%with";

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
    is_script: bool,
    code: Vec<Op>,
    constants: Vec<Value>,
    string_constants: HashMap<JsStr, u32>,
    functions: Vec<Arc<FunctionProto>>,
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
    /// Code en mode strict.
    strict: bool,
    uses_arguments: bool,
    param_count: u16,
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
}

/// Compile un script en une fonction sans paramètre dont la valeur de retour
/// est celle de la dernière instruction-expression de premier niveau.
pub(crate) fn compile_script(
    body: &[Stmt],
    strict: bool,
    is_async: bool,
) -> CResult<Arc<FunctionProto>> {
    let mut compiler = Compiler {
        funcs: Vec::new(),
        name_hint: None,
        with_depth: 0,
        with_count: 0,
    };
    compiler.push_function(FunctionKind::Normal, Arc::from("<script>"), true);
    compiler.f().strict = strict;
    compiler.f().is_async = is_async;
    compiler.push_scope();
    // Registre 0 : valeur d'achèvement du script.
    let completion = compiler.alloc()?;
    compiler.emit(Op::LoadUndefined { dst: completion });
    check_declarations(body, true, &[])?;
    compiler.hoist_vars(body)?;
    compiler.declare_block(body)?;
    for stmt in body {
        compiler.statement(stmt)?;
    }
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
/// Erreurs précoces de redéclaration dans une liste d'instructions
/// (ECMA-262 §14.2.1, §15.2.1, §16.1.1). `top` désigne un corps de fonction ou
/// de script, où les fonctions se déclarent comme des `var` ; `outer` porte
/// les noms déjà liés dans la même portée (paramètres, paramètre de `catch`).
///
/// LIMITE : deux fonctions de même nom dans un bloc sont acceptées même en
/// mode strict, faute de connaître ici le mode du code.
fn check_declarations(body: &[Stmt], top: bool, outer: &[JsStr]) -> CResult<()> {
    let mut lexical: Vec<JsStr> = Vec::new();
    let mut functions: Vec<JsStr> = Vec::new();
    for stmt in body {
        match stmt {
            Stmt::VarDecl(VarKind::Let | VarKind::Const, decls) => {
                for (pattern, _) in decls {
                    pattern_names(pattern, &mut lexical);
                }
            }
            Stmt::Class(class) => lexical.extend(class.name.clone()),
            Stmt::Function(func) => functions.extend(func.name.clone()),
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

fn var_names(stmts: &[Stmt], out: &mut Vec<JsStr>) {
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
            uses_arguments: false,
            param_count: 0,
            is_async: false,
            is_generator: false,
        });
    }

    fn pop_function(&mut self) -> CResult<FunctionProto> {
        let Some(state) = self.funcs.pop() else {
            return error("pile de fonctions vide");
        };
        Ok(FunctionProto {
            name: state.name,
            param_count: state.param_count,
            registers: state.max_reg.max(1),
            code: state.code,
            constants: state.constants,
            functions: state.functions,
            upvalues: state.upvalues.into_iter().map(|(desc, _)| desc).collect(),
            kind: state.kind,
            is_async: state.is_async,
            is_generator: state.is_generator,
            strict: state.strict,
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
            return Ok(None);
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
        chain
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
                if kind == BindKind::Const && !init {
                    let name = self.string(name);
                    self.emit(Op::ThrowConstAssign { name });
                } else {
                    self.emit(Op::SetUpvalue { index, src });
                }
            }
            Resolved::Global => {
                let name = self.string(name);
                self.emit(if init {
                    Op::InitGlobal { name, src }
                } else {
                    Op::SetGlobal { name, src }
                });
            }
        }
        Ok(())
    }

    /// Déclare dans la portée de fonction courante les noms `var` de `body`.
    fn hoist_vars(&mut self, body: &[Stmt]) -> CResult<()> {
        let mut names = Vec::new();
        var_names(body, &mut names);
        let script = self.at_script_level();
        for name in names {
            if script {
                // « Trou » : ne crée la variable globale que si elle n'existe pas déjà.
                let mark = self.mark();
                let tmp = self.alloc()?;
                self.emit(Op::LoadHole { dst: tmp });
                let index = self.string(&name);
                self.emit(Op::InitGlobal {
                    name: index,
                    src: tmp,
                });
                self.free_to(mark);
                continue;
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
        if !script {
            for stmt in body {
                match stmt {
                    Stmt::VarDecl(kind @ (VarKind::Let | VarKind::Const), decls) => {
                        let bind = if *kind == VarKind::Const {
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
                    Stmt::Function(func) => {
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
                self.store(name, tmp, true)?;
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
        let name = func.name.clone().or(hint).unwrap_or_else(|| Arc::from(""));
        self.push_function(kind, name, false);
        self.f().is_async = func.is_async;
        self.f().is_generator = func.is_generator;
        self.f().strict = func.strict;
        self.push_scope();
        let entry = self.emit(Op::Nop);

        // Les arguments arrivent dans les premiers registres.
        let mut complex: Vec<(Reg, &Param)> = Vec::new();
        for param in &func.params {
            let reg = self.alloc()?;
            match &param.pattern {
                Pattern::Ident(name) => {
                    self.declare_at(name, BindKind::Var, true, reg);
                    if param.default.is_some() {
                        complex.push((reg, param));
                    }
                }
                _ => complex.push((reg, param)),
            }
        }
        self.f().param_count = u16::try_from(func.params.len()).unwrap_or(u16::MAX);
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
            let reg = self.declare(name, BindKind::Var, true)?;
            self.emit(Op::LoadCallee { dst: reg });
        }
        for (reg, param) in complex {
            if let Some(default) = &param.default {
                let skip = self.emit(Op::JumpIfNotUndefined {
                    src: reg,
                    target: 0,
                });
                self.expr(default, Some(reg))?;
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

        let mut param_names = Vec::new();
        for param in &func.params {
            pattern_names(&param.pattern, &mut param_names);
        }
        if let Some(rest) = &func.rest {
            pattern_names(rest, &mut param_names);
        }
        check_declarations(&func.body, true, &param_names)?;
        self.hoist_vars(&func.body)?;
        self.declare_block(&func.body)?;
        for stmt in &func.body {
            self.statement(stmt)?;
        }
        let tmp = self.alloc()?;
        self.emit(Op::LoadUndefined { dst: tmp });
        self.emit(Op::Return { src: tmp });

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
                };
                &synthesized
            }
        };
        let mut ctor_ast = constructor.clone();
        ctor_ast.name = class.name.clone().or(name_hint);
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
        // Champs et blocs statiques, évalués une fois les méthodes définies
        // et la classe liée à son nom : registre de la clé (champ) et corps.
        let mut statics: Vec<(Option<Reg>, Arc<Function>)> = Vec::new();
        for member in &class.members {
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
                    fields.push(Stmt::Expr(Expr::FieldInit {
                        key: member.key.clone(),
                        value: Box::new(value),
                    }));
                }
            }
            self.free_to(member_mark);
        }
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
            };
            let reg = self.alloc()?;
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
                self.emit(Op::InitElem {
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

    fn statements(&mut self, body: &[Stmt]) -> CResult<()> {
        check_declarations(body, false, &[])?;
        self.push_scope();
        self.declare_block(body)?;
        for stmt in body {
            self.statement(stmt)?;
        }
        self.pop_scope();
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
        let init = kind != VarKind::Var;
        for (pattern, value) in decls {
            let mark = self.mark();
            match (pattern, value) {
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
        match stmt {
            Stmt::Empty | Stmt::Function(_) => {}
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
                    self.store(name, reg, true)?;
                }
            }
            Stmt::Return(value) => {
                if self.f().is_script {
                    return error("« return » en dehors d'une fonction");
                }
                let reg = self.alloc()?;
                match value {
                    Some(value) => {
                        self.expr(value, Some(reg))?;
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
                self.statement(then)?;
                match otherwise {
                    Some(otherwise) => {
                        let to_end = self.emit(Op::Jump { target: 0 });
                        self.patch(to_else);
                        self.statement(otherwise)?;
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
        let labels = std::mem::take(&mut self.f().pending_labels);
        self.push_scope();
        match init {
            Some(ForInit::Decl(kind, decls)) => {
                if *kind != VarKind::Var {
                    let bind = if *kind == VarKind::Const {
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
                    let bind = if *kind == VarKind::Const {
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
        self.statement(body)?;
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
        check_declarations(&all, false, &[])?;
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
                check_declarations(&lexical_only, false, &caught)?;
                check_declarations(handler, false, &[])?;
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
            Pattern::Expr(_) => return error("cible d'affectation invalide"),
            Pattern::Object { props, rest } => {
                // Déstructurer `null` ou `undefined` est une erreur : l'accès ci-dessous la lève.
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
                    let value = self.alloc()?;
                    self.emit(Op::IterNextOrUndefined { dst: value, iter });
                    if let Some((target, default)) = element {
                        self.default_and_assign(target, default.as_ref(), value, init)?;
                    }
                    self.free_to(inner);
                }
                if let Some(rest) = rest {
                    let value = self.alloc()?;
                    self.emit(Op::IterRest { dst: value, iter });
                    self.assign_pattern(rest, value, init)?;
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
                    // `yield* it` dans un générateur async : comme `for await`.
                    let iter = self.alloc()?;
                    self.emit(Op::AsyncIterStart {
                        dst: iter,
                        src: value,
                    });
                    let head = self.here();
                    let item = self.alloc()?;
                    let exit = self.async_iter_next(iter, item)?;
                    self.emit_yield(item, item, None)?;
                    self.emit(Op::Jump { target: head });
                    self.patch(exit);
                    self.emit(Op::LoadUndefined { dst });
                } else if *delegate {
                    // `yield* it` : rend une à une les valeurs de l'itérable.
                    let iter = self.alloc()?;
                    self.emit(Op::IterStart {
                        dst: iter,
                        src: value,
                    });
                    let item = self.alloc()?;
                    let head = self.here();
                    let exit = self.emit(Op::IterNext {
                        dst: item,
                        iter,
                        done: 0,
                    });
                    self.emit_yield(item, item, Some(iter))?;
                    self.emit(Op::Jump { target: head });
                    self.patch(exit);
                    self.emit(Op::LoadUndefined { dst });
                } else {
                    self.emit_yield(dst, value, None)?;
                }
                self.free_to(mark);
                Ok(dst)
            }
            Expr::PrivateName(name) => self.load(name, dst),
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
                self.emit(Op::InitElem {
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
                    matches!(&**callee, Expr::Ident(name) if &**name == "eval") && self.f().strict;
                let arguments = self.arguments(args)?;
                if direct_eval {
                    self.emit(Op::StrictEval);
                }
                match arguments {
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
                let current = self.load(name, None)?;
                self.emit(Op::ToNumber {
                    dst: old,
                    src: current,
                });
                self.emit(Op::Step {
                    dst: new,
                    src: old,
                    increment,
                });
                self.store(name, new, false)?;
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
                self.emit(Op::GetElem { dst: old, obj, key });
                self.emit(Op::ToNumber { dst: old, src: old });
                self.emit(Op::Step {
                    dst: new,
                    src: old,
                    increment,
                });
                self.emit(Op::SetElem { obj, key, src: new });
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
                let src = self.alloc()?;
                self.expr_named(value, name, Some(src))?;
                self.store(name, src, false)?;
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
            (AssignOp::Assign, pattern) => {
                let src = self.expr_to_temp(value)?;
                self.assign_pattern(pattern, src, false)?;
                self.emit(Op::Move { dst, src });
            }
            (_, Pattern::Ident(name)) => {
                let current = self.alloc()?;
                self.load(name, Some(current))?;
                let skip = self.compound(op, current, value)?;
                self.store(name, current, false)?;
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
                let skip = self.compound(op, current, value)?;
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
                let current = self.alloc()?;
                self.emit(Op::GetElem {
                    dst: current,
                    obj,
                    key,
                });
                let skip = self.compound(op, current, value)?;
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
    fn compound(&mut self, op: AssignOp, current: Reg, value: &Expr) -> CResult<Option<usize>> {
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
                self.expr(value, Some(current))?;
                Ok(Some(skip))
            }
            AssignOp::Assign => Ok(None),
        }
    }
}
