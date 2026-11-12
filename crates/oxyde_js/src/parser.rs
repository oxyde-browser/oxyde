//! Analyse syntaxique d'ECMAScript : descente récursive et priorités d'opérateurs.
//!
//! Les erreurs précoces qui tiennent à la grammaire (mode strict, mots
//! réservés, noms privés, `super`, `new.target`, étiquettes) sont signalées
//! ici ; les redéclarations le sont par le compilateur.
//!
//! LIMITE CONNUE : Les parenthèses ne sont pas conservées : `(a) = 1` et `([a]) = 1`
//! sont traités de la même façon.

use std::sync::Arc;

use crate::ast::{
    AssignOp, BinaryOp, Class, ClassMember, ClassMemberKind, Expr, ForInit, ForTarget, Function,
    LogicalOp, Param, Pattern, PropKey, PropKind, Property, Stmt, UnaryOp, VarKind,
};
use crate::lexer::{Lexer, SyntaxError, Tok, Token};
use crate::module::{Import, ModuleInfo};

/// Profondeur maximale d'imbrication syntaxique. Borne la récursion de
/// l'analyseur et du compilateur ; voir [`crate::with_large_stack`].
const MAX_DEPTH: usize = 500;

const RESERVED: &[&str] = &[
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "else",
    "enum",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "function",
    "if",
    "import",
    "in",
    "instanceof",
    "new",
    "null",
    "return",
    "super",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "typeof",
    "var",
    "void",
    "while",
    "with",
];

/// Mots réservés en mode strict seulement.
const STRICT_RESERVED: &[&str] = &[
    "implements",
    "interface",
    "let",
    "package",
    "private",
    "protected",
    "public",
    "static",
    "yield",
];

type PResult<T> = Result<T, SyntaxError>;

/// Nature d'un membre privé de classe.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PrivateKind {
    Field,
    Method,
    Getter,
    Setter,
}

/// Noms privés d'un corps de classe en cours d'analyse.
#[derive(Default)]
struct PrivateScope {
    /// Membres déclarés : nom (sans `#`), nature, statique ou non.
    declared: Vec<(Arc<str>, PrivateKind, bool)>,
    /// Noms employés, avec leur position, à vérifier en fin de classe.
    used: Vec<(Arc<str>, usize)>,
}

/// Contexte d'analyse propre à une fonction, mis de côté pendant l'analyse
/// d'une fonction imbriquée.
struct FnState {
    no_in: bool,
    in_async: bool,
    in_generator: bool,
    strict: bool,
    in_function: bool,
    super_property: bool,
    super_call: bool,
    in_field_init: bool,
    in_static_block: bool,
    no_return: bool,
    in_params: bool,
    labels: Vec<Arc<str>>,
    cover_init: Option<usize>,
}

struct Parser {
    lexer: Lexer,
    tok: Token,
    /// Fin du dernier jeton consommé.
    prev_end: usize,
    /// Position d'un `...x` suivi d'une virgule dans un littéral tableau ou
    /// objet : permis dans une expression, interdit si le littéral se
    /// révèle être un motif d'affectation.
    rest_comma: Option<usize>,
    /// Début du texte source de la prochaine fonction analysée.
    fn_start: Option<usize>,
    depth: usize,
    /// Vrai pendant l'analyse de l'initialisation d'un `for` : `in` n'y est pas un opérateur.
    no_in: bool,
    /// Vrai dans le corps d'une fonction async : `await` y est un opérateur.
    in_async: bool,
    /// Vrai dans le corps d'un générateur : `yield` y est un opérateur.
    in_generator: bool,
    /// Vrai pendant l'analyse d'un module : `import.meta` y est permis.
    module: bool,
    /// Vrai dans du code en mode strict.
    strict: bool,
    /// Vrai dans une fonction non fléchée (ou une fléchée qui y est
    /// imbriquée) : `new.target` y est permis.
    in_function: bool,
    /// Vrai dans une méthode : `super.x` y est permis.
    super_property: bool,
    /// Vrai dans le constructeur d'une classe dérivée : `super()` y est permis.
    super_call: bool,
    /// La prochaine fonction analysée est le constructeur d'une classe dérivée.
    derived_ctor: bool,
    /// Vrai dans l'initialiseur d'un champ de classe : `arguments` y est interdit.
    in_field_init: bool,
    /// Vrai dans un bloc `static { }` : `await` y est réservé.
    in_static_block: bool,
    /// Vrai directement dans un bloc `static { }` : `return` y est interdit.
    no_return: bool,
    /// Vrai pendant l'analyse de paramètres : `yield` et `await` y sont interdits.
    in_params: bool,
    /// Vrai là où une déclaration `using` est interdite : au premier niveau
    /// d'un script et directement dans une clause de `switch`.
    no_using: bool,
    /// Étiquettes des instructions englobantes de la fonction courante.
    labels: Vec<Arc<str>>,
    /// Corps de classes en cours d'analyse, du plus externe au plus interne.
    private_scopes: Vec<PrivateScope>,
    /// Le module emploie `await` au premier niveau.
    top_level_await: bool,
    /// Un appel `eval(…)` a été lu dans la fonction en cours d'analyse.
    saw_eval: bool,
    /// Position du premier `{ a = 1 }` (ou `__proto__` en double) qui n'est
    /// valide que si l'objet se révèle être un motif d'affectation.
    cover_init: Option<usize>,
}

/// Nom de la liaison locale qui porte la valeur de `export default <expression>`.
/// Ce n'est pas un identifiant : aucun script ne peut la désigner.
pub(crate) const DEFAULT_EXPORT: &str = "*default*";

/// Script analysé : ses instructions et son mode.
#[derive(Debug)]
pub(crate) struct Script {
    pub(crate) body: Vec<Stmt>,
    /// Le script commence par la directive `"use strict"`.
    pub(crate) strict: bool,
}

/// Analyse un script complet.
pub(crate) fn parse_program(source: &str) -> PResult<Script> {
    parse_script(source, false, false, &[], false, false)
}

/// Analyse le source passé à un `eval` indirect : du code global.
pub(crate) fn parse_eval(source: &str, strict: bool) -> PResult<Script> {
    parse_script(source, false, false, &[], strict, false)
}

/// Analyse le source passé à un `eval` direct. `in_function` : l'appelant
/// est dans une fonction (`new.target` permis) ; `in_method` : dans une
/// méthode (`super.x` permis) ; `private_names` : noms privés visibles.
pub(crate) fn parse_direct_eval(
    source: &str,
    strict: bool,
    in_function: bool,
    in_method: bool,
    in_field_init: bool,
    private_names: &[Arc<str>],
) -> PResult<Script> {
    parse_script(
        source,
        in_function,
        in_method,
        private_names,
        strict,
        in_field_init,
    )
}

fn parse_script(
    source: &str,
    in_function: bool,
    in_method: bool,
    private_names: &[Arc<str>],
    strict: bool,
    in_field_init: bool,
) -> PResult<Script> {
    let mut parser = Parser::new(source, false)?;
    parser.in_function = in_function;
    parser.in_field_init = in_field_init;
    parser.super_property = in_method;
    parser.strict = strict;
    if !private_names.is_empty() {
        // Noms privés des classes qui entourent l'appel à `eval`.
        parser.private_scopes.push(PrivateScope {
            declared: private_names
                .iter()
                .map(|name| (Arc::clone(name), PrivateKind::Field, false))
                .collect(),
            used: Vec::new(),
        });
    }
    let mut body = Vec::new();
    let strict = parser.directives(&mut body)? || strict;
    while parser.tok.tok != Tok::Eof {
        body.push(parser.statement()?);
    }
    if let Some(scope) = parser.private_scopes.pop() {
        parser.close_private_scope(scope)?;
    }
    Ok(Script { body, strict })
}

/// Analyse un module : ses instructions (les déclarations `import` et
/// `export` sont ramenées à des déclarations ordinaires) et ce qu'il importe
/// et exporte.
pub(crate) fn parse_module(source: &str) -> PResult<(Vec<Stmt>, ModuleInfo)> {
    // Un module est toujours du code strict.
    let mut parser = Parser::new(source, true)?;
    let mut body = Vec::new();
    let mut info = ModuleInfo::default();
    while parser.tok.tok != Tok::Eof {
        parser.module_item(&mut body, &mut info)?;
    }
    info.has_top_level_await = parser.top_level_await;
    // Un nom exporté par `export { x }` doit être déclaré dans le module.
    let mut declared: Vec<Arc<str>> = Vec::new();
    crate::compiler::var_names(&body, &mut declared);
    crate::compiler::lexical_names(&body, &mut declared);
    for stmt in &body {
        if let Stmt::Function(func) = stmt {
            declared.extend(func.name.clone());
        }
    }
    declared.extend(info.imports.iter().map(|import| Arc::clone(&import.local)));
    for (_, local) in &info.local_exports {
        if !declared.contains(local) {
            return Err(parser
                .lexer
                .error_at(0, format!("« {local} » est exporté sans être déclaré")));
        }
    }
    Ok((body, info))
}

fn binary_op(p: &str) -> Option<(u8, BinaryOp)> {
    Some(match p {
        "|" => (4, BinaryOp::BitOr),
        "^" => (5, BinaryOp::BitXor),
        "&" => (6, BinaryOp::BitAnd),
        "==" => (7, BinaryOp::Eq),
        "!=" => (7, BinaryOp::Ne),
        "===" => (7, BinaryOp::StrictEq),
        "!==" => (7, BinaryOp::StrictNe),
        "<" => (8, BinaryOp::Lt),
        "<=" => (8, BinaryOp::Le),
        ">" => (8, BinaryOp::Gt),
        ">=" => (8, BinaryOp::Ge),
        "<<" => (9, BinaryOp::Shl),
        ">>" => (9, BinaryOp::Shr),
        ">>>" => (9, BinaryOp::UShr),
        "+" => (10, BinaryOp::Add),
        "-" => (10, BinaryOp::Sub),
        "*" => (11, BinaryOp::Mul),
        "/" => (11, BinaryOp::Div),
        "%" => (11, BinaryOp::Rem),
        "**" => (12, BinaryOp::Exp),
        _ => return None,
    })
}

fn assign_op(p: &str) -> Option<AssignOp> {
    Some(match p {
        "=" => AssignOp::Assign,
        "+=" => AssignOp::Binary(BinaryOp::Add),
        "-=" => AssignOp::Binary(BinaryOp::Sub),
        "*=" => AssignOp::Binary(BinaryOp::Mul),
        "/=" => AssignOp::Binary(BinaryOp::Div),
        "%=" => AssignOp::Binary(BinaryOp::Rem),
        "**=" => AssignOp::Binary(BinaryOp::Exp),
        "<<=" => AssignOp::Binary(BinaryOp::Shl),
        ">>=" => AssignOp::Binary(BinaryOp::Shr),
        ">>>=" => AssignOp::Binary(BinaryOp::UShr),
        "&=" => AssignOp::Binary(BinaryOp::BitAnd),
        "|=" => AssignOp::Binary(BinaryOp::BitOr),
        "^=" => AssignOp::Binary(BinaryOp::BitXor),
        "&&=" => AssignOp::Logical(LogicalOp::And),
        "||=" => AssignOp::Logical(LogicalOp::Or),
        "??=" => AssignOp::Logical(LogicalOp::Nullish),
        _ => return None,
    })
}

impl Parser {
    fn new(source: &str, module: bool) -> PResult<Self> {
        let mut lexer = Lexer::new(source);
        lexer.html_comments = !module;
        let tok = lexer.next()?;
        Ok(Self {
            lexer,
            tok,
            depth: 0,
            no_in: false,
            // Au premier niveau d'un module, `await` est un opérateur.
            in_async: module,
            in_generator: false,
            module,
            top_level_await: false,
            saw_eval: false,
            strict: module,
            in_function: false,
            super_property: false,
            super_call: false,
            derived_ctor: false,
            in_field_init: false,
            in_static_block: false,
            no_return: false,
            in_params: false,
            no_using: !module,
            labels: Vec::new(),
            private_scopes: Vec::new(),
            cover_init: None,
            prev_end: 0,
            rest_comma: None,
            fn_start: None,
        })
    }

    /// Met de côté le contexte de la fonction courante ; les étiquettes et
    /// l'initialiseur en attente repartent de zéro.
    fn take_state(&mut self) -> FnState {
        FnState {
            no_in: self.no_in,
            in_async: self.in_async,
            in_generator: self.in_generator,
            strict: self.strict,
            in_function: self.in_function,
            super_property: self.super_property,
            super_call: self.super_call,
            in_field_init: self.in_field_init,
            in_static_block: self.in_static_block,
            no_return: self.no_return,
            in_params: self.in_params,
            labels: std::mem::take(&mut self.labels),
            cover_init: self.cover_init.take(),
        }
    }

    fn restore_state(&mut self, state: FnState) {
        self.no_in = state.no_in;
        self.in_async = state.in_async;
        self.in_generator = state.in_generator;
        self.strict = state.strict;
        self.in_function = state.in_function;
        self.super_property = state.super_property;
        self.super_call = state.super_call;
        self.in_field_init = state.in_field_init;
        self.in_static_block = state.in_static_block;
        self.no_return = state.no_return;
        self.in_params = state.in_params;
        self.labels = state.labels;
        self.cover_init = state.cover_init.or(self.cover_init);
    }

    // ---- Modules ---------------------------------------------------------

    fn module_item(&mut self, body: &mut Vec<Stmt>, info: &mut ModuleInfo) -> PResult<()> {
        if self.is_kw("import") && !matches!(self.peek()?.tok, Tok::Punct("(" | ".")) {
            return self.import_declaration(info);
        }
        if self.is_kw("export") {
            return self.export_declaration(body, info);
        }
        body.push(self.statement()?);
        Ok(())
    }

    /// Spécificateur de module (`"./chemin.js"`), enregistré dans `info`.
    fn module_request(&mut self, info: &mut ModuleInfo) -> PResult<usize> {
        let Tok::Str(specifier) = &self.tok.tok else {
            return self.error("nom de module attendu (chaîne)");
        };
        let specifier = specifier.clone();
        self.advance()?;
        // Attributs d'importation : seul `type` est reconnu ; il fait
        // partie de la demande (voir `module::split_request`).
        let mut kind: Option<String> = None;
        if self.eat_kw("with")? {
            self.expect_punct("{")?;
            let mut seen: Vec<String> = Vec::new();
            while !self.eat_punct("}")? {
                let key = match &self.tok.tok {
                    Tok::Ident(name) | Tok::Str(name) => name.clone(),
                    _ => return self.error("nom d'attribut d'importation attendu"),
                };
                self.advance()?;
                if seen.contains(&key) {
                    return self.error(format!("attribut d'importation « {key} » répété"));
                }
                self.expect_punct(":")?;
                let Tok::Str(value) = &self.tok.tok else {
                    return self.error("valeur d'attribut d'importation attendue (chaîne)");
                };
                if key == "type" {
                    kind = Some(value.clone());
                } else {
                    return self.error(format!(
                        "attribut d'importation « {key} » non pris en charge"
                    ));
                }
                self.advance()?;
                seen.push(key);
                if !self.is_punct("}") {
                    self.expect_punct(",")?;
                }
            }
        }
        let request = match kind {
            Some(kind) => info.request(&crate::module::typed_request(&specifier, &kind)),
            None => info.request(&specifier),
        };
        Ok(request)
    }

    fn from_clause(&mut self, info: &mut ModuleInfo) -> PResult<usize> {
        if !self.eat_kw("from")? {
            return self.error("« from » attendu");
        }
        self.module_request(info)
    }

    /// Nom d'export : identifiant quelconque (mot réservé compris) ou chaîne.
    /// Le booléen dit si c'est un identifiant utilisable comme liaison.
    fn export_name(&mut self) -> PResult<(Arc<str>, bool)> {
        let name = match &self.tok.tok {
            Tok::Ident(name) => (
                Arc::<str>::from(name.as_str()),
                !RESERVED.contains(&name.as_str()),
            ),
            Tok::Str(name) => (Arc::<str>::from(name.as_str()), false),
            _ => return self.error("nom d'export attendu"),
        };
        self.advance()?;
        Ok(name)
    }

    fn import_declaration(&mut self, info: &mut ModuleInfo) -> PResult<()> {
        self.advance()?;
        // `import "module";` : pour ses seuls effets.
        if matches!(self.tok.tok, Tok::Str(_)) {
            self.module_request(info)?;
            return self.semicolon();
        }
        // `import defer * as ns from …` et `import source x from …`.
        let mut phase = 0u8;
        if !self.tok.escaped {
            let mut probe = self.lexer.clone();
            let next = probe.next()?;
            if self.is_kw("defer") && next.tok == Tok::Punct("*") {
                phase = 2;
            } else if self.is_kw("source")
                && let Tok::Ident(word) = &next.tok
                && (word != "from" || matches!(&probe.next()?.tok, Tok::Ident(w) if w == "from"))
            {
                phase = 1;
            }
            if phase != 0 {
                self.advance()?;
            }
        }
        // (nom dans la source ; `None` : espace de noms, liaison locale).
        let mut bindings: Vec<(Option<Arc<str>>, Arc<str>)> = Vec::new();
        let mut more = true;
        if phase == 1 {
            // Une seule liaison, qui reçoit la source du module.
            bindings.push((Some(Arc::from("default")), self.binding_ident()?));
            more = false;
        } else if phase == 0 && matches!(self.tok.tok, Tok::Ident(_)) {
            bindings.push((Some(Arc::from("default")), self.binding_ident()?));
            more = self.eat_punct(",")?;
        }
        if more {
            if self.eat_punct("*")? {
                if !self.eat_kw("as")? {
                    return self.error("« as » attendu");
                }
                bindings.push((None, self.binding_ident()?));
            } else if self.eat_punct("{")? {
                while !self.eat_punct("}")? {
                    let (imported, is_binding) = self.export_name()?;
                    let local = if self.eat_kw("as")? {
                        self.binding_ident()?
                    } else if is_binding {
                        Arc::clone(&imported)
                    } else {
                        return self.error("« as » attendu");
                    };
                    bindings.push((Some(imported), local));
                    if !self.is_punct("}") {
                        self.expect_punct(",")?;
                    }
                }
            } else {
                return self.error("liaisons d'importation attendues");
            }
        }
        let request = self.from_clause(info)?;
        self.semicolon()?;
        if phase == 2
            && let Some(count) = info.deferred.get_mut(request)
        {
            *count += 1;
        }
        for (name, local) in bindings {
            if info.imports.iter().any(|i| i.local == local) {
                return self.error(format!("« {local} » est importé deux fois"));
            }
            info.imports.push(Import {
                request,
                name,
                local,
                phase,
            });
        }
        Ok(())
    }

    fn add_export(
        &self,
        info: &mut ModuleInfo,
        exported: Arc<str>,
        local: Arc<str>,
    ) -> PResult<()> {
        if info.exports(&exported) {
            return self.error(format!("« {exported} » est exporté deux fois"));
        }
        info.local_exports.push((exported, local));
        Ok(())
    }

    fn export_declaration(&mut self, body: &mut Vec<Stmt>, info: &mut ModuleInfo) -> PResult<()> {
        self.advance()?;
        // `export * from "m"` et `export * as ns from "m"`.
        if self.eat_punct("*")? {
            let alias = if self.eat_kw("as")? {
                Some(self.export_name()?.0)
            } else {
                None
            };
            let request = self.from_clause(info)?;
            self.semicolon()?;
            match alias {
                Some(alias) => {
                    if info.exports(&alias) {
                        return self.error(format!("« {alias} » est exporté deux fois"));
                    }
                    info.indirect_exports.push((alias, request, None));
                }
                None => info.star_exports.push(request),
            }
            return Ok(());
        }
        // `export { a, b as c }`, avec ou sans `from "m"`.
        if self.eat_punct("{")? {
            let mut specifiers = Vec::new();
            while !self.eat_punct("}")? {
                let (local, is_binding) = self.export_name()?;
                let exported = if self.eat_kw("as")? {
                    self.export_name()?.0
                } else {
                    Arc::clone(&local)
                };
                specifiers.push((local, is_binding, exported));
                if !self.is_punct("}") {
                    self.expect_punct(",")?;
                }
            }
            let request = if self.is_kw("from") {
                Some(self.from_clause(info)?)
            } else {
                None
            };
            self.semicolon()?;
            for (local, is_binding, exported) in specifiers {
                match request {
                    Some(request) => {
                        if info.exports(&exported) {
                            return self.error(format!("« {exported} » est exporté deux fois"));
                        }
                        info.indirect_exports.push((exported, request, Some(local)));
                    }
                    None if !is_binding => {
                        return self
                            .error(format!("« {local} » ne désigne pas une liaison locale"));
                    }
                    None => self.add_export(info, exported, local)?,
                }
            }
            return Ok(());
        }
        // `export default …`
        if self.eat_kw("default")? {
            let declaration = self.is_kw("function")
                || self.is_kw("class")
                || (self.is_kw("async") && self.async_function_follows()?);
            // Une déclaration s'arrête à son accolade : `function () {}()`
            // n'est pas un appel.
            let value = if declaration {
                self.primary()?
            } else {
                self.assignment()?
            };
            let local = match value {
                // Fonction ou classe nommée : c'est une déclaration ordinaire.
                Expr::Function(func) if declaration && func.name.is_some() => {
                    let name = func.name.clone().unwrap_or_else(|| Arc::from(""));
                    body.push(Stmt::Function(func));
                    name
                }
                Expr::Class(class) if declaration && class.name.is_some() => {
                    let name = class.name.clone().unwrap_or_else(|| Arc::from(""));
                    body.push(Stmt::Class(class));
                    name
                }
                value => {
                    let local: Arc<str> = Arc::from(DEFAULT_EXPORT);
                    body.push(Stmt::VarDecl(
                        VarKind::Const,
                        vec![(Pattern::Ident(Arc::clone(&local)), Some(value))],
                    ));
                    local
                }
            };
            if declaration {
                self.eat_punct(";")?;
            } else {
                self.semicolon()?;
            }
            return self.add_export(info, Arc::from("default"), local);
        }
        // `export` devant une déclaration.
        let statement = self.statement()?;
        let mut names = Vec::new();
        match &statement {
            Stmt::VarDecl(_, declarations) => {
                for (pattern, _) in declarations {
                    crate::compiler::pattern_names(pattern, &mut names);
                }
            }
            Stmt::Function(func) => names.extend(func.name.clone()),
            Stmt::Class(class) => names.extend(class.name.clone()),
            _ => return self.error("déclaration attendue après « export »"),
        }
        body.push(statement);
        for name in names {
            self.add_export(info, Arc::clone(&name), name)?;
        }
        Ok(())
    }

    // ---- Jetons ----------------------------------------------------------

    fn error<T>(&self, message: impl Into<String>) -> PResult<T> {
        Err(self.lexer.error_at(self.tok.start, message))
    }

    fn unsupported<T>(&self, what: &str) -> PResult<T> {
        self.error(format!(
            "{what} : non pris en charge par Oxyde pour l'instant"
        ))
    }

    /// Texte source qui va de `start` à la fin du dernier jeton consommé.
    fn span(&self, start: Option<usize>) -> Option<crate::ast::SourceSpan> {
        start.map(|start| crate::ast::SourceSpan {
            text: self.lexer.source(),
            start,
            end: self.prev_end.max(start),
        })
    }

    fn advance(&mut self) -> PResult<Token> {
        self.prev_end = self.lexer.pos;
        let next = self.lexer.next()?;
        if self.strict && next.legacy_octal {
            return Err(self
                .lexer
                .error_at(next.start, "forme octale interdite en mode strict"));
        }
        Ok(std::mem::replace(&mut self.tok, next))
    }

    fn is_punct(&self, p: &str) -> bool {
        matches!(&self.tok.tok, Tok::Punct(q) if *q == p)
    }

    fn eat_punct(&mut self, p: &str) -> PResult<bool> {
        if self.is_punct(p) {
            self.advance()?;
            return Ok(true);
        }
        Ok(false)
    }

    fn expect_punct(&mut self, p: &str) -> PResult<()> {
        if self.eat_punct(p)? {
            Ok(())
        } else {
            self.error(format!("« {p} » attendu"))
        }
    }

    /// Vrai si le jeton courant est le mot `k`, écrit sans échappement.
    fn is_kw(&self, k: &str) -> bool {
        !self.tok.escaped && matches!(&self.tok.tok, Tok::Ident(name) if name == k)
    }

    fn eat_kw(&mut self, k: &str) -> PResult<bool> {
        if self.is_kw(k) {
            self.advance()?;
            return Ok(true);
        }
        Ok(false)
    }

    fn expect_kw(&mut self, k: &str) -> PResult<()> {
        if self.eat_kw(k)? {
            Ok(())
        } else {
            self.error(format!("« {k} » attendu"))
        }
    }

    /// Jeton qui suit le jeton courant, sans le consommer.
    fn peek(&self) -> PResult<Token> {
        self.lexer.clone().next()
    }

    fn peek_is_punct(&self, p: &str) -> PResult<bool> {
        Ok(matches!(self.peek()?.tok, Tok::Punct(q) if q == p))
    }

    /// Identifiant de liaison : ni mot réservé, ni `eval` ou `arguments` en
    /// mode strict.
    fn binding_ident(&mut self) -> PResult<Arc<str>> {
        self.identifier(true)
    }

    /// Identifiant (pas un mot réservé) ; `binding` dit s'il nomme une liaison.
    fn identifier(&mut self, binding: bool) -> PResult<Arc<str>> {
        let name: Arc<str> = match &self.tok.tok {
            Tok::Ident(name) if !RESERVED.contains(&name.as_str()) => Arc::from(name.as_str()),
            _ => return self.error("identifiant attendu"),
        };
        self.check_identifier(&name, binding)?;
        self.advance()?;
        Ok(name)
    }

    /// Étiquette : un identifiant, `arguments` compris même là où la
    /// variable est interdite.
    fn label(&mut self) -> PResult<Arc<str>> {
        let saved = std::mem::replace(&mut self.in_field_init, false);
        let label = self.identifier(false);
        self.in_field_init = saved;
        label
    }

    /// Nom facultatif d'une expression de fonction. Il se lit dans le
    /// contexte de la fonction elle-même : `yield` peut nommer une fonction
    /// ordinaire imbriquée dans un générateur, pas un générateur.
    fn function_expression_name(
        &mut self,
        is_async: bool,
        is_generator: bool,
    ) -> PResult<Option<Arc<str>>> {
        if self.is_punct("(") {
            return Ok(None);
        }
        let saved = (self.in_async, self.in_generator);
        (self.in_async, self.in_generator) = (is_async, is_generator);
        let name = self.binding_ident();
        (self.in_async, self.in_generator) = saved;
        name.map(Some)
    }

    /// Refuse les mots qui ne sont réservés que selon le contexte.
    fn check_identifier(&self, name: &str, binding: bool) -> PResult<()> {
        if self.strict && STRICT_RESERVED.contains(&name) {
            return self.error(format!("« {name} » est réservé en mode strict"));
        }
        if binding && self.strict && (name == "eval" || name == "arguments") {
            return self.error(format!("« {name} » ne peut pas être lié en mode strict"));
        }
        if name == "yield" && self.in_generator {
            return self.error("« yield » est réservé dans un générateur");
        }
        if name == "await" && (self.in_async || self.module || self.in_static_block) {
            return self.error("« await » est réservé ici");
        }
        if !binding && name == "arguments" && self.in_field_init {
            return self.error("« arguments » est interdit dans un initialiseur de champ");
        }
        Ok(())
    }

    /// Refuse `eval` et `arguments` comme cibles d'affectation en mode strict.
    fn check_assignable(&self, name: &str) -> PResult<()> {
        if self.strict && (name == "eval" || name == "arguments") {
            return self.error(format!(
                "« {name} » ne peut pas être affecté en mode strict"
            ));
        }
        Ok(())
    }

    // ---- Noms privés -------------------------------------------------------

    fn declare_private(
        &mut self,
        name: Arc<str>,
        kind: PrivateKind,
        is_static: bool,
        start: usize,
    ) -> PResult<()> {
        if &*name == "constructor" {
            return Err(self.lexer.error_at(start, "« #constructor » est interdit"));
        }
        let Some(scope) = self.private_scopes.last_mut() else {
            return Ok(());
        };
        // Seule paire permise sous un même nom : un accesseur de chaque sorte.
        let clash = scope
            .declared
            .iter()
            .any(|(other, other_kind, other_static)| {
                *other == name
                    && !(*other_static == is_static
                        && matches!(
                            (*other_kind, kind),
                            (PrivateKind::Getter, PrivateKind::Setter)
                                | (PrivateKind::Setter, PrivateKind::Getter)
                        ))
            });
        if clash {
            return Err(self
                .lexer
                .error_at(start, format!("« #{name} » est déclaré deux fois")));
        }
        scope.declared.push((name, kind, is_static));
        Ok(())
    }

    fn use_private(&mut self, name: &str, start: usize) -> PResult<()> {
        match self.private_scopes.last_mut() {
            Some(scope) => {
                scope.used.push((Arc::from(name), start));
                Ok(())
            }
            None => Err(self
                .lexer
                .error_at(start, format!("nom privé « #{name} » hors d'une classe"))),
        }
    }

    /// Vérifie, en fin de classe, que chaque nom privé employé est déclaré par
    /// elle ou le sera par une classe englobante.
    fn close_private_scope(&mut self, scope: PrivateScope) -> PResult<()> {
        for (name, start) in scope.used {
            if scope
                .declared
                .iter()
                .any(|(declared, ..)| *declared == name)
            {
                continue;
            }
            match self.private_scopes.last_mut() {
                Some(outer) => outer.used.push((name, start)),
                None => {
                    return Err(self
                        .lexer
                        .error_at(start, format!("nom privé « #{name} » non déclaré")));
                }
            }
        }
        Ok(())
    }

    /// Accès `object.name`. Un nom privé (`#x`) devient un accès indexé par
    /// la clé privée, que la classe englobante lie sous ce nom.
    fn member(object: Expr, name: Arc<str>, optional: bool) -> Expr {
        if name.starts_with('#') {
            return Expr::Index {
                object: Box::new(object),
                index: Box::new(Expr::PrivateName(name)),
                optional,
            };
        }
        Expr::Member {
            object: Box::new(object),
            name,
            optional,
        }
    }

    /// Nom de propriété après un point : les mots réservés y sont permis.
    fn property_name(&mut self) -> PResult<Arc<str>> {
        let name: Arc<str> = match &self.tok.tok {
            Tok::Ident(name) => Arc::from(name.as_str()),
            Tok::Private(name) => {
                let name = name.clone();
                self.use_private(&name, self.tok.start)?;
                Arc::from(format!("#{name}"))
            }
            _ => return self.error("nom de propriété attendu"),
        };
        self.advance()?;
        Ok(name)
    }

    /// Insertion automatique de point-virgule (ECMA-262 §12.10).
    fn semicolon(&mut self) -> PResult<()> {
        if self.eat_punct(";")? {
            return Ok(());
        }
        if self.is_punct("}") || self.tok.tok == Tok::Eof || self.tok.newline_before {
            return Ok(());
        }
        self.error("« ; » attendu")
    }

    fn enter(&mut self) -> PResult<()> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return self.error("imbrication trop profonde");
        }
        Ok(())
    }

    // ---- Instructions ----------------------------------------------------

    fn statement(&mut self) -> PResult<Stmt> {
        self.enter()?;
        let result = self.statement_inner();
        self.depth -= 1;
        if result.is_ok()
            && let Some(start) = self.cover_init.take()
        {
            return Err(self
                .lexer
                .error_at(start, "littéral objet invalide hors d'un motif"));
        }
        result
    }

    /// Prologue de directives d'un script ou d'un corps de fonction : passe en
    /// mode strict sur `"use strict"` et dit si cette directive est présente.
    fn directives(&mut self, body: &mut Vec<Stmt>) -> PResult<bool> {
        let mut found = false;
        let mut octal = false;
        while let Tok::Str(text) = &self.tok.tok {
            let next = self.peek()?;
            let ends = match &next.tok {
                Tok::Eof | Tok::Punct(";" | "}") => true,
                Tok::Punct(_) | Tok::Template { .. } => false,
                Tok::Ident(word) if word == "in" || word == "instanceof" => false,
                _ => next.newline_before,
            };
            if !ends {
                break;
            }
            octal |= self.tok.legacy_octal;
            // Sans échappement ni continuation : dix caractères entre guillemets.
            if text == "use strict" && self.lexer.pos - self.tok.start == 12 {
                found = true;
                self.strict = true;
            }
            body.push(self.statement()?);
        }
        if found && octal {
            return self.error("forme octale interdite en mode strict");
        }
        Ok(found)
    }

    /// Corps de fonction entre accolades ; l'accolade fermante reste le jeton
    /// courant, pour être consommée une fois le mode strict du corps quitté.
    /// Le booléen dit si le corps porte la directive `"use strict"`.
    fn function_body(&mut self) -> PResult<(Vec<Stmt>, bool)> {
        self.expect_punct("{")?;
        let no_using = std::mem::replace(&mut self.no_using, false);
        let mut body = Vec::new();
        let directive = self.directives(&mut body)?;
        while !self.is_punct("}") {
            if self.tok.tok == Tok::Eof {
                return self.error("« } » attendu");
            }
            body.push(self.statement()?);
        }
        self.no_using = no_using;
        Ok((body, directive))
    }

    /// Vrai si `import` (jeton courant) est suivi de `.source(` ou `.defer(`.
    fn import_phase_follows(&self) -> PResult<bool> {
        let mut probe = self.lexer.clone();
        if probe.next()?.tok != Tok::Punct(".") {
            return Ok(false);
        }
        let word = probe.next()?;
        let phase = matches!(&word.tok, Tok::Ident(w) if w == "source" || w == "defer");
        Ok(phase && !word.escaped && probe.next()?.tok == Tok::Punct("("))
    }

    /// Vrai si le jeton courant ouvre une déclaration `using` : le mot
    /// `using`, suivi sur la même ligne d'un nom de liaison.
    fn using_follows(&self) -> PResult<bool> {
        if !self.is_kw("using") || self.tok.escaped {
            return Ok(false);
        }
        let next = self.lexer.clone().next()?;
        Ok(!next.newline_before
            && matches!(&next.tok, Tok::Ident(word) if word != "in" && word != "instanceof"))
    }

    /// Vrai si le jeton courant ouvre une déclaration `await using`, là où
    /// `await` est un opérateur.
    /// Comme [`Self::using_follows`], dans la tête d'un `for` : `using of`
    /// n'y ouvre pas une déclaration.
    fn for_using_follows(&self) -> PResult<bool> {
        if !self.using_follows()? {
            return Ok(false);
        }
        let mut probe = self.lexer.clone();
        if !matches!(&probe.next()?.tok, Tok::Ident(word) if word == "of") {
            return Ok(true);
        }
        // `using of = …` déclare une liaison nommée `of`.
        Ok(probe.next()?.tok == Tok::Punct("="))
    }

    fn await_using_follows(&self) -> PResult<bool> {
        if !self.is_kw("await") || self.tok.escaped || !self.in_async {
            return Ok(false);
        }
        let mut probe = self.lexer.clone();
        let using = probe.next()?;
        if using.newline_before
            || using.escaped
            || !matches!(&using.tok, Tok::Ident(word) if word == "using")
        {
            return Ok(false);
        }
        let name = probe.next()?;
        Ok(!name.newline_before
            && matches!(&name.tok, Tok::Ident(word) if word != "in" && word != "instanceof"))
    }

    /// `using x = …;` ou `await using x = …;` ; le jeton courant est `using`
    /// ou `await`.
    fn using_declaration(&mut self, kind: VarKind) -> PResult<Stmt> {
        if self.no_using {
            return self.error("déclaration « using » interdite ici");
        }
        if kind == VarKind::AwaitUsing {
            self.top_level_await |= self.module && !self.in_function;
            self.advance()?;
        }
        self.advance()?;
        let decls = self.declarators(kind)?;
        self.semicolon()?;
        Ok(Stmt::VarDecl(kind, decls))
    }

    /// Instruction en position de corps (`if`, boucle) : les déclarations y
    /// sont interdites, sauf une fonction ordinaire après `if` hors mode strict.
    fn substatement(&mut self, allow_function: bool) -> PResult<Stmt> {
        if let Tok::Ident(word) = &self.tok.tok
            && !self.tok.escaped
        {
            match word.as_str() {
                "class" | "const" => return self.error("déclaration interdite ici"),
                "function" => {
                    if !allow_function || self.strict || self.peek_is_punct("*")? {
                        return self.error("déclaration de fonction interdite ici");
                    }
                }
                "async" if self.async_function_follows()? => {
                    return self.error("déclaration de fonction interdite ici");
                }
                "using" if self.using_follows()? => {
                    return self.error("déclaration interdite ici");
                }
                "await" if self.await_using_follows()? => {
                    return self.error("déclaration interdite ici");
                }
                "let" => {
                    let next = self.peek()?;
                    match next.tok {
                        Tok::Punct("[") => return self.error("déclaration interdite ici"),
                        // `let` puis un saut de ligne : l'identifiant `let`.
                        Tok::Ident(_) | Tok::Punct("{") if next.newline_before => {
                            return self.expression_statement();
                        }
                        Tok::Ident(_) | Tok::Punct("{") => {
                            return self.error("déclaration interdite ici");
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        let statement = self.statement()?;
        let mut inner = &statement;
        let mut labeled = false;
        while let Stmt::Labeled(_, body) = inner {
            inner = body;
            labeled = true;
        }
        if labeled && matches!(inner, Stmt::Function(_)) {
            return self.error("déclaration de fonction interdite ici");
        }
        Ok(statement)
    }

    /// `étiquette: instruction` ; le jeton courant est l'étiquette.
    fn labeled_statement(&mut self) -> PResult<Stmt> {
        let label = self.label()?;
        if self.labels.contains(&label) {
            return self.error(format!("étiquette « {label} » déjà employée"));
        }
        self.advance()?;
        if self.is_kw("function") && (self.strict || self.peek_is_punct("*")?) {
            return self.error("déclaration de fonction interdite ici");
        }
        // Une étiquette précède une instruction, jamais une déclaration.
        let lexical = self.is_kw("const")
            || self.is_kw("class")
            || (self.is_kw("async") && self.async_function_follows()?)
            || self.using_follows()?
            || self.await_using_follows()?
            || (self.is_kw("let") && {
                let next = self.peek()?;
                matches!(next.tok, Tok::Punct("["))
                    || (!next.newline_before && matches!(next.tok, Tok::Ident(_) | Tok::Punct("{")))
            });
        if lexical && !self.tok.escaped {
            return self.error("déclaration interdite après une étiquette");
        }
        self.labels.push(Arc::clone(&label));
        let body = self.statement();
        self.labels.pop();
        Ok(Stmt::Labeled(label, Box::new(body?)))
    }

    fn block(&mut self) -> PResult<Vec<Stmt>> {
        self.expect_punct("{")?;
        let no_using = std::mem::replace(&mut self.no_using, false);
        let mut body = Vec::new();
        while !self.is_punct("}") {
            if self.tok.tok == Tok::Eof {
                return self.error("« } » attendu");
            }
            body.push(self.statement()?);
        }
        self.no_using = no_using;
        self.advance()?;
        Ok(body)
    }

    fn statement_inner(&mut self) -> PResult<Stmt> {
        let Tok::Ident(word) = &self.tok.tok else {
            if self.is_punct("{") {
                return Ok(Stmt::Block(self.block()?));
            }
            if self.eat_punct(";")? {
                return Ok(Stmt::Empty);
            }
            return self.expression_statement();
        };
        if self.tok.escaped {
            // Un mot écrit avec un échappement n'est jamais un mot-clé.
            if !RESERVED.contains(&word.as_str()) && self.peek_is_punct(":")? {
                return self.labeled_statement();
            }
            return self.expression_statement();
        }
        match word.as_str() {
            "var" | "const" => {
                let kind = if word == "var" {
                    VarKind::Var
                } else {
                    VarKind::Const
                };
                self.advance()?;
                let decls = self.declarators(kind)?;
                self.semicolon()?;
                Ok(Stmt::VarDecl(kind, decls))
            }
            "let" if matches!(self.peek()?.tok, Tok::Ident(_) | Tok::Punct("[" | "{")) => {
                self.advance()?;
                let decls = self.declarators(VarKind::Let)?;
                self.semicolon()?;
                Ok(Stmt::VarDecl(VarKind::Let, decls))
            }
            "using" if self.using_follows()? => self.using_declaration(VarKind::Using),
            "await" if self.await_using_follows()? => self.using_declaration(VarKind::AwaitUsing),
            "function" => {
                let start = self.tok.start;
                self.advance()?;
                let is_generator = self.eat_punct("*")?;
                let name = self.binding_ident()?;
                self.fn_start = Some(start);
                Ok(Stmt::Function(Arc::new(self.function_with(
                    Some(name),
                    false,
                    false,
                    is_generator,
                )?)))
            }
            "async" if self.async_function_follows()? => {
                let start = self.tok.start;
                self.advance()?;
                self.advance()?;
                let is_generator = self.eat_punct("*")?;
                let name = self.binding_ident()?;
                self.fn_start = Some(start);
                Ok(Stmt::Function(Arc::new(self.function_with(
                    Some(name),
                    false,
                    true,
                    is_generator,
                )?)))
            }
            "class" => {
                let start = self.tok.start;
                self.advance()?;
                Ok(Stmt::Class(Box::new(self.class_rest(true, start)?)))
            }
            "if" => {
                self.advance()?;
                self.expect_punct("(")?;
                let test = self.expression()?;
                self.expect_punct(")")?;
                let then = Box::new(self.substatement(true)?);
                let otherwise = if self.eat_kw("else")? {
                    Some(Box::new(self.substatement(true)?))
                } else {
                    None
                };
                Ok(Stmt::If {
                    test,
                    then,
                    otherwise,
                })
            }
            "for" => self.for_statement(),
            "while" => {
                self.advance()?;
                self.expect_punct("(")?;
                let test = self.expression()?;
                self.expect_punct(")")?;
                Ok(Stmt::While {
                    test,
                    body: Box::new(self.substatement(false)?),
                })
            }
            "do" => {
                self.advance()?;
                let body = Box::new(self.substatement(false)?);
                self.expect_kw("while")?;
                self.expect_punct("(")?;
                let test = self.expression()?;
                self.expect_punct(")")?;
                self.eat_punct(";")?;
                Ok(Stmt::DoWhile { body, test })
            }
            "return" => {
                if self.no_return {
                    return self.error("« return » interdit dans un bloc statique");
                }
                self.advance()?;
                let value = if self.is_punct(";")
                    || self.is_punct("}")
                    || self.tok.tok == Tok::Eof
                    || self.tok.newline_before
                {
                    None
                } else {
                    Some(self.expression()?)
                };
                self.semicolon()?;
                Ok(Stmt::Return(value))
            }
            "break" | "continue" => {
                let is_break = word == "break";
                self.advance()?;
                let label = match &self.tok.tok {
                    Tok::Ident(_) if !self.tok.newline_before => Some(self.label()?),
                    _ => None,
                };
                self.semicolon()?;
                Ok(if is_break {
                    Stmt::Break(label)
                } else {
                    Stmt::Continue(label)
                })
            }
            "throw" => {
                self.advance()?;
                if self.tok.newline_before {
                    return self.error("saut de ligne interdit après « throw »");
                }
                let value = self.expression()?;
                self.semicolon()?;
                Ok(Stmt::Throw(value))
            }
            "try" => self.try_statement(),
            "switch" => self.switch_statement(),
            "debugger" => {
                self.advance()?;
                self.semicolon()?;
                Ok(Stmt::Empty)
            }
            "import" if !matches!(self.peek()?.tok, Tok::Punct("(" | ".")) => {
                self.unsupported("les modules (import)")
            }
            "export" => self.unsupported("les modules (export)"),
            "with" => {
                if self.strict {
                    return self.error("« with » est interdit en mode strict");
                }
                self.advance()?;
                self.expect_punct("(")?;
                let object = self.expression()?;
                self.expect_punct(")")?;
                Ok(Stmt::With(object, Box::new(self.substatement(false)?)))
            }
            _ if !RESERVED.contains(&word.as_str()) && self.peek_is_punct(":")? => {
                self.labeled_statement()
            }
            _ => self.expression_statement(),
        }
    }

    fn expression_statement(&mut self) -> PResult<Stmt> {
        let expr = self.expression()?;
        self.semicolon()?;
        Ok(Stmt::Expr(expr))
    }

    fn declarators(&mut self, kind: VarKind) -> PResult<Vec<(Pattern, Option<Expr>)>> {
        let mut decls = Vec::new();
        loop {
            let pattern = self.binding_pattern()?;
            if kind.is_using() && !matches!(pattern, Pattern::Ident(_)) {
                return self.error("« using » ne lie que des identifiants");
            }
            if kind != VarKind::Var {
                let mut names = Vec::new();
                crate::compiler::pattern_names(&pattern, &mut names);
                if names.iter().any(|name| &**name == "let") {
                    return self.error("« let » ne peut pas nommer une liaison lexicale");
                }
            }
            let init = if self.eat_punct("=")? {
                Some(self.assignment()?)
            } else {
                None
            };
            if init.is_none() && !self.no_in {
                if kind == VarKind::Const || kind.is_using() {
                    return self.error("une constante doit être initialisée");
                }
                if !matches!(pattern, Pattern::Ident(_)) {
                    return self.error("un motif de déstructuration doit être initialisé");
                }
            }
            decls.push((pattern, init));
            if !self.eat_punct(",")? {
                return Ok(decls);
            }
        }
    }

    fn for_statement(&mut self) -> PResult<Stmt> {
        self.advance()?;
        let is_await = self.is_kw("await");
        if is_await {
            if !self.in_async {
                return self.error("« for await » n'est permis que dans une fonction async");
            }
            self.top_level_await |= self.module && !self.in_function;
            self.advance()?;
        }
        self.expect_punct("(")?;
        let mut init = None;
        let mut for_in_init = None;
        if !self.is_punct(";") {
            let decl_kind = match &self.tok.tok {
                Tok::Ident(_) if self.tok.escaped => None,
                Tok::Ident(w) if w == "var" => Some(VarKind::Var),
                Tok::Ident(w) if w == "const" => Some(VarKind::Const),
                Tok::Ident(w)
                    if w == "let"
                        && matches!(self.peek()?.tok, Tok::Ident(_) | Tok::Punct("[" | "{")) =>
                {
                    Some(VarKind::Let)
                }
                // `for (using of …)` : `using` y reste un identifiant.
                Tok::Ident(w) if w == "using" && self.for_using_follows()? => Some(VarKind::Using),
                Tok::Ident(w) if w == "await" && self.await_using_follows()? => {
                    Some(VarKind::AwaitUsing)
                }
                _ => None,
            };
            // `for (async of …)` est interdit : il se lirait comme le début
            // d'une fonction fléchée (`async of => …` reste permis).
            if decl_kind.is_none() && !is_await && self.is_kw("async") && !self.tok.escaped {
                let mut probe = self.lexer.clone();
                let of = matches!(probe.next()?.tok, Tok::Ident(w) if w == "of");
                if of && probe.next()?.tok != Tok::Punct("=>") {
                    return self.error("« for (async of » est interdit");
                }
            }
            self.rest_comma = None;
            self.no_in = true;
            let parsed = match decl_kind {
                Some(kind) => {
                    if kind == VarKind::AwaitUsing {
                        self.top_level_await |= self.module && !self.in_function;
                        self.advance()?;
                    }
                    self.advance()?;
                    self.declarators(kind).map(|d| ForInit::Decl(kind, d))
                }
                None => self.expression().map(ForInit::Expr),
            };
            self.no_in = false;
            let parsed = parsed?;
            let of = self.is_kw("of");
            if of || self.is_kw("in") {
                self.advance()?;
                let target = match parsed {
                    ForInit::Decl(kind, mut decls) => {
                        if decls.len() != 1 || (kind.is_using() && !of) {
                            return self.error("déclaration invalide dans for…in / for…of");
                        }
                        let (pattern, init) = decls.remove(0);
                        if let Some(init) = init {
                            // Annexe B.3.5 : `for (var x = init in obj)`, hors
                            // mode strict seulement ; l'initialiseur s'évalue
                            // avant la boucle.
                            let allowed = !of
                                && !self.strict
                                && kind == VarKind::Var
                                && matches!(pattern, Pattern::Ident(_));
                            if !allowed {
                                return self.error("déclaration invalide dans for…in / for…of");
                            }
                            for_in_init = Some(Stmt::VarDecl(
                                VarKind::Var,
                                vec![(pattern.clone(), Some(init))],
                            ));
                        }
                        ForTarget::Decl(kind, pattern)
                    }
                    ForInit::Expr(expr) => {
                        // `for ({ a = 1 } of xs)` : l'objet est bien un motif.
                        if matches!(expr, Expr::Object(_) | Expr::Array(_)) {
                            self.cover_init = None;
                            if let Some(position) = self.rest_comma.take() {
                                return Err(self.lexer.error_at(
                                    position,
                                    "l'élément de reste doit être le dernier du motif",
                                ));
                            }
                        }
                        ForTarget::Pattern(self.to_pattern(expr)?)
                    }
                };
                let object = if of {
                    self.assignment()?
                } else {
                    self.expression()?
                };
                self.expect_punct(")")?;
                if is_await && !of {
                    return self.error("« for await » exige la forme « of »");
                }
                let body = self.substatement(false)?;
                if let ForTarget::Decl(kind, pattern) = &target
                    && *kind != VarKind::Var
                {
                    let mut names = Vec::new();
                    crate::compiler::pattern_names(pattern, &mut names);
                    self.check_loop_names(&names, &body)?;
                }
                let looped = Stmt::ForIn {
                    target,
                    object,
                    body: Box::new(body),
                    of,
                    is_await,
                };
                return Ok(match for_in_init {
                    Some(init) => Stmt::Block(vec![init, looped]),
                    None => looped,
                });
            }
            if is_await {
                return self.error("« for await » exige la forme « of »");
            }
            if let ForInit::Decl(kind, decls) = &parsed {
                for (pattern, value) in decls {
                    if value.is_none()
                        && (*kind == VarKind::Const
                            || kind.is_using()
                            || !matches!(pattern, Pattern::Ident(_)))
                    {
                        return self.error("déclaration sans initialisation");
                    }
                }
            }
            init = Some(parsed);
        }
        self.expect_punct(";")?;
        let test = if self.is_punct(";") {
            None
        } else {
            Some(self.expression()?)
        };
        self.expect_punct(";")?;
        let update = if self.is_punct(")") {
            None
        } else {
            Some(self.expression()?)
        };
        self.expect_punct(")")?;
        Ok(Stmt::For {
            init,
            test,
            update,
            body: Box::new(self.substatement(false)?),
        })
    }

    /// Noms liés par la tête `let` ou `const` d'une boucle : sans doublon,
    /// et distincts des `var` que déclare le corps.
    fn check_loop_names(&self, names: &[Arc<str>], body: &Stmt) -> PResult<()> {
        let mut vars = Vec::new();
        crate::compiler::var_names(std::slice::from_ref(body), &mut vars);
        for (i, name) in names.iter().enumerate() {
            if names.get(..i).is_some_and(|seen| seen.contains(name)) || vars.contains(name) {
                return self.error(format!("« {name} » est déjà déclaré"));
            }
        }
        Ok(())
    }

    fn try_statement(&mut self) -> PResult<Stmt> {
        self.advance()?;
        let block = self.block()?;
        let mut param = None;
        let mut handler = None;
        if self.eat_kw("catch")? {
            if self.eat_punct("(")? {
                param = Some(self.binding_pattern()?);
                self.expect_punct(")")?;
            }
            let body = self.block()?;
            if let Some(pattern) = &param {
                // Le paramètre ne peut pas être redéclaré par une liaison
                // lexicale du bloc, ni lier deux fois le même nom.
                let mut names = Vec::new();
                crate::compiler::pattern_names(pattern, &mut names);
                let mut lexical = Vec::new();
                crate::compiler::lexical_names(&body, &mut lexical);
                for stmt in &body {
                    if let Stmt::Function(func) = stmt {
                        lexical.extend(func.name.clone());
                    }
                }
                for (i, name) in names.iter().enumerate() {
                    if names.get(..i).is_some_and(|seen| seen.contains(name))
                        || lexical.contains(name)
                    {
                        return self.error(format!("« {name} » est déjà déclaré"));
                    }
                }
            }
            handler = Some(body);
        }
        let finalizer = if self.eat_kw("finally")? {
            Some(self.block()?)
        } else {
            None
        };
        if handler.is_none() && finalizer.is_none() {
            return self.error("« catch » ou « finally » attendu");
        }
        Ok(Stmt::Try {
            block,
            param,
            handler,
            finalizer,
        })
    }

    fn switch_statement(&mut self) -> PResult<Stmt> {
        self.advance()?;
        self.expect_punct("(")?;
        let discriminant = self.expression()?;
        self.expect_punct(")")?;
        self.expect_punct("{")?;
        let mut cases = Vec::new();
        let mut has_default = false;
        while !self.eat_punct("}")? {
            let test = if self.eat_kw("default")? {
                if std::mem::replace(&mut has_default, true) {
                    return self.error("plusieurs « default » dans un switch");
                }
                None
            } else {
                self.expect_kw("case")?;
                Some(self.expression()?)
            };
            self.expect_punct(":")?;
            // Pas de `using` directement dans une clause.
            let no_using = std::mem::replace(&mut self.no_using, true);
            let mut body = Vec::new();
            while !self.is_kw("case") && !self.is_kw("default") && !self.is_punct("}") {
                if self.tok.tok == Tok::Eof {
                    return self.error("« } » attendu");
                }
                body.push(self.statement()?);
            }
            self.no_using = no_using;
            cases.push((test, body));
        }
        Ok(Stmt::Switch {
            discriminant,
            cases,
        })
    }

    // ---- Fonctions et classes --------------------------------------------

    /// Paramètres entre parenthèses (la parenthèse ouvrante est le jeton courant).
    fn params(&mut self) -> PResult<(Vec<Param>, Option<Pattern>)> {
        let saved = std::mem::replace(&mut self.in_params, true);
        let result = self.params_inner();
        self.in_params = saved;
        result
    }

    fn params_inner(&mut self) -> PResult<(Vec<Param>, Option<Pattern>)> {
        self.expect_punct("(")?;
        let mut params = Vec::new();
        let mut rest = None;
        while !self.eat_punct(")")? {
            if self.eat_punct("...")? {
                rest = Some(self.binding_pattern()?);
                self.expect_punct(")")?;
                break;
            }
            let pattern = self.binding_pattern()?;
            let default = if self.eat_punct("=")? {
                Some(self.assignment()?)
            } else {
                None
            };
            params.push(Param { pattern, default });
            if !self.is_punct(")") {
                self.expect_punct(",")?;
            }
        }
        Ok((params, rest))
    }

    /// Paramètres et corps d'une fonction ordinaire dont le nom a déjà été lu.
    fn function_rest(&mut self, name: Option<Arc<str>>, is_method: bool) -> PResult<Function> {
        self.function_with(name, is_method, false, false)
    }

    /// Paramètres et corps d'une fonction, éventuellement async ou génératrice.
    fn function_with(
        &mut self,
        name: Option<Arc<str>>,
        is_method: bool,
        is_async: bool,
        is_generator: bool,
    ) -> PResult<Function> {
        let start = self.fn_start.take();
        let super_call = std::mem::take(&mut self.derived_ctor);
        let saved = self.take_state();
        let was_strict = saved.strict;
        self.no_in = false;
        self.in_async = is_async;
        self.in_generator = is_generator;
        self.in_function = true;
        self.super_property = is_method;
        self.super_call = super_call;
        self.in_field_init = false;
        self.in_static_block = false;
        self.no_return = false;
        self.in_params = false;
        let checked_name = if is_method { None } else { name.as_deref() };
        let outer_eval = std::mem::replace(&mut self.saw_eval, false);
        let parsed = self.function_parts(checked_name, is_method, was_strict);
        let has_direct_eval = std::mem::replace(&mut self.saw_eval, outer_eval);
        let strict = self.strict;
        self.restore_state(saved);
        let (params, rest, body) = parsed?;
        // L'accolade fermante, lue hors du mode strict éventuel du corps.
        self.advance()?;
        Ok(Function {
            source: self.span(start),
            name,
            params,
            rest,
            body,
            is_arrow: false,
            is_method,
            is_async,
            is_generator,
            strict,
            has_direct_eval,
        })
    }

    /// Paramètres puis corps d'une fonction ; l'accolade fermante reste le
    /// jeton courant.
    fn function_parts(
        &mut self,
        name: Option<&str>,
        unique: bool,
        was_strict: bool,
    ) -> PResult<(Vec<Param>, Option<Pattern>, Vec<Stmt>)> {
        let (params, rest) = self.params()?;
        let (body, directive) = self.function_body()?;
        self.check_params(name, &params, rest.as_ref(), unique, directive, was_strict)?;
        Ok((params, rest, body))
    }

    /// Erreurs précoces sur les paramètres, connues une fois le corps lu :
    /// doublons, noms interdits par un corps strict, directive `"use strict"`
    /// avec des paramètres non simples.
    fn check_params(
        &self,
        name: Option<&str>,
        params: &[Param],
        rest: Option<&Pattern>,
        unique: bool,
        directive: bool,
        was_strict: bool,
    ) -> PResult<()> {
        let simple = rest.is_none()
            && params
                .iter()
                .all(|p| p.default.is_none() && matches!(p.pattern, Pattern::Ident(_)));
        if directive && !simple {
            return self.error("« use strict » interdit avec des paramètres non simples");
        }
        let mut names = Vec::new();
        for param in params {
            crate::compiler::pattern_names(&param.pattern, &mut names);
        }
        if let Some(rest) = rest {
            crate::compiler::pattern_names(rest, &mut names);
        }
        if unique || !simple || self.strict {
            for (i, name) in names.iter().enumerate() {
                if names[..i].contains(name) {
                    return self.error(format!("paramètre « {name} » en double"));
                }
            }
        }
        if self.strict && !was_strict {
            // Le corps vient de passer en mode strict : ses règles valent
            // aussi pour le nom et les paramètres, déjà lus.
            for bound in names.iter().map(|n| &**n).chain(name) {
                if bound == "eval" || bound == "arguments" || STRICT_RESERVED.contains(&bound) {
                    return self.error(format!("« {bound} » ne peut pas être lié en mode strict"));
                }
            }
        }
        Ok(())
    }

    /// Vrai si le jeton courant `async` est suivi, sur la même ligne, de `function`.
    fn async_function_follows(&self) -> PResult<bool> {
        if self.tok.escaped {
            return Ok(false);
        }
        let after = self.peek()?;
        Ok(!after.newline_before
            && !after.escaped
            && matches!(after.tok, Tok::Ident(ref w) if w == "function"))
    }

    /// Un accesseur `get` n'a aucun paramètre, un `set` exactement un.
    fn check_accessor(&self, func: &Function, is_getter: bool) -> PResult<()> {
        let valid = if is_getter {
            func.params.is_empty() && func.rest.is_none()
        } else {
            func.params.len() == 1 && func.rest.is_none()
        };
        if valid {
            Ok(())
        } else if is_getter {
            self.error("un accesseur « get » n'a pas de paramètre")
        } else {
            self.error("un accesseur « set » a exactement un paramètre")
        }
    }

    /// Bloc `static { … }` d'une classe ; le jeton courant est l'accolade ouvrante.
    fn static_block(&mut self) -> PResult<Function> {
        let saved = self.take_state();
        self.no_in = false;
        self.in_async = false;
        self.in_generator = false;
        self.in_function = true;
        self.super_property = true;
        self.super_call = false;
        self.in_field_init = true;
        self.in_static_block = true;
        self.no_return = true;
        self.in_params = false;
        let body = self.function_body();
        self.restore_state(saved);
        let (body, _) = body?;
        self.advance()?;
        Ok(Function {
            source: None,
            name: None,
            params: Vec::new(),
            rest: None,
            body,
            is_arrow: false,
            is_method: true,
            is_async: false,
            is_generator: false,
            strict: true,
            has_direct_eval: false,
        })
    }

    /// Initialiseur d'un champ de classe : s'évalue comme le corps d'une méthode.
    fn field_initializer(&mut self) -> PResult<Expr> {
        let saved = self.take_state();
        self.no_in = false;
        self.in_async = false;
        self.in_generator = false;
        self.in_function = true;
        self.super_property = true;
        self.super_call = false;
        self.in_field_init = true;
        self.in_static_block = false;
        self.in_params = false;
        let value = self.assignment();
        self.restore_state(saved);
        value
    }

    /// Modificateurs `async` et `*` placés devant le nom d'une méthode.
    fn method_modifiers(&mut self) -> PResult<(bool, bool)> {
        let mut is_async = false;
        if self.is_kw("async") {
            let after = self.peek()?;
            let modifier = matches!(after.tok, Tok::Punct("*")) || self.is_modifier()?;
            if !after.newline_before && modifier {
                self.advance()?;
                is_async = true;
            }
        }
        let is_generator = self.eat_punct("*")?;
        Ok((is_async, is_generator))
    }

    fn property_key(&mut self) -> PResult<PropKey> {
        let key = match &self.tok.tok {
            Tok::Ident(name) => PropKey::Name(Arc::from(name.as_str())),
            Tok::Private(name) => PropKey::Name(Arc::from(format!("#{name}"))),
            Tok::Str(s) => PropKey::Name(Arc::from(s.as_str())),
            Tok::Num(n) => PropKey::Name(Arc::from(crate::value::number_to_string(*n))),
            Tok::BigInt(big) => PropKey::Name(Arc::from(big.to_string())),
            Tok::Punct("[") => {
                self.advance()?;
                let expr = self.assignment()?;
                self.expect_punct("]")?;
                return Ok(PropKey::Computed(Box::new(expr)));
            }
            _ => return self.error("nom de propriété attendu"),
        };
        self.advance()?;
        Ok(key)
    }

    /// Vrai si le jeton courant (`get`, `set`, `static`…) est un modificateur
    /// et non le nom d'une propriété : il est alors suivi d'une clé.
    fn is_modifier(&self) -> PResult<bool> {
        Ok(matches!(
            self.peek()?.tok,
            Tok::Ident(_)
                | Tok::Private(_)
                | Tok::Str(_)
                | Tok::Num(_)
                | Tok::BigInt(_)
                | Tok::Punct("[")
        ))
    }

    /// Nom d'une méthode ou d'un accesseur (`get x`, `set x`).
    fn accessor_name(key: &PropKey, accessor: Option<bool>) -> Option<Arc<str>> {
        let name = Self::method_name(key)?;
        Some(match accessor {
            Some(true) => Arc::from(format!("get {name}")),
            Some(false) => Arc::from(format!("set {name}")),
            None => name,
        })
    }

    fn method_name(key: &PropKey) -> Option<Arc<str>> {
        match key {
            PropKey::Name(name) => Some(Arc::clone(name)),
            PropKey::Computed(_) => None,
        }
    }

    fn class_rest(&mut self, name_required: bool, start: usize) -> PResult<Class> {
        // Une définition de classe est toujours du code strict.
        let was_strict = std::mem::replace(&mut self.strict, true);
        let parsed = self.class_tail(name_required);
        self.strict = was_strict;
        let mut class = parsed?;
        // L'accolade fermante, lue hors du mode strict de la classe.
        self.advance()?;
        class.source = self.span(Some(start));
        Ok(class)
    }

    /// Nom, parent et corps d'une classe ; l'accolade fermante reste le jeton courant.
    fn class_tail(&mut self, name_required: bool) -> PResult<Class> {
        let name = match &self.tok.tok {
            Tok::Ident(w) if w != "extends" || self.tok.escaped => Some(self.binding_ident()?),
            _ if name_required => return self.error("nom de classe attendu"),
            _ => None,
        };
        let parent = if self.eat_kw("extends")? {
            Some(self.left_hand_side()?)
        } else {
            None
        };
        self.expect_punct("{")?;
        self.private_scopes.push(PrivateScope::default());
        let body = self.class_body(parent.is_some());
        let scope = self.private_scopes.pop().unwrap_or_default();
        let (constructor, members) = body?;
        self.close_private_scope(scope)?;
        Ok(Class {
            source: None,
            name,
            parent,
            constructor,
            members,
        })
    }

    fn class_body(&mut self, derived: bool) -> PResult<(Option<Arc<Function>>, Vec<ClassMember>)> {
        let mut constructor = None;
        let mut members = Vec::new();
        while !self.is_punct("}") {
            if self.eat_punct(";")? {
                continue;
            }
            if self.is_kw("static") && self.peek_is_punct("{")? {
                self.advance()?;
                members.push(ClassMember {
                    key: PropKey::Name(Arc::from("")),
                    is_static: true,
                    kind: ClassMemberKind::StaticBlock(Arc::new(self.static_block()?)),
                });
                continue;
            }
            let is_static =
                self.is_kw("static") && (self.is_modifier()? || self.peek_is_punct("*")?);
            if is_static {
                self.advance()?;
            }
            let member_start = self.tok.start;
            let (is_async, is_generator) = self.method_modifiers()?;
            let accessor = match &self.tok.tok {
                Tok::Ident(w)
                    if !self.tok.escaped && (w == "get" || w == "set") && self.is_modifier()? =>
                {
                    Some(w == "get")
                }
                _ => None,
            };
            if accessor.is_some() {
                self.advance()?;
            }
            let private = match &self.tok.tok {
                Tok::Private(name) => Some((Arc::<str>::from(name.as_str()), self.tok.start)),
                _ => None,
            };
            let key = self.property_key()?;
            let named = |wanted: &str| matches!(&key, PropKey::Name(n) if &**n == wanted);
            let named_constructor = named("constructor");
            let named_prototype = named("prototype");
            let is_method = self.is_punct("(");
            if let Some((name, start)) = private {
                let kind = match (is_method, accessor) {
                    (false, _) => PrivateKind::Field,
                    (true, Some(true)) => PrivateKind::Getter,
                    (true, Some(false)) => PrivateKind::Setter,
                    (true, None) => PrivateKind::Method,
                };
                self.declare_private(name, kind, is_static, start)?;
            }
            let kind = if is_method {
                if is_static && named_prototype {
                    return self.error("une méthode statique ne peut pas se nommer « prototype »");
                }
                let is_constructor = !is_static && named_constructor;
                if is_constructor && (accessor.is_some() || is_async || is_generator) {
                    return self.error("le constructeur est une méthode ordinaire");
                }
                self.derived_ctor = is_constructor && derived;
                self.fn_start = Some(member_start);
                let func = Arc::new(self.function_with(
                    Self::accessor_name(&key, accessor),
                    true,
                    is_async,
                    is_generator,
                )?);
                if let Some(is_getter) = accessor {
                    self.check_accessor(&func, is_getter)?;
                }
                if is_constructor {
                    if constructor.replace(func).is_some() {
                        return self.error("une classe n'a qu'un constructeur");
                    }
                    continue;
                }
                match accessor {
                    Some(true) => ClassMemberKind::Getter(func),
                    Some(false) => ClassMemberKind::Setter(func),
                    None => ClassMemberKind::Method(func),
                }
            } else {
                if accessor.is_some() || is_async || is_generator {
                    return self.error("« ( » attendu");
                }
                if named_constructor || (is_static && named_prototype) {
                    return self.error("nom de champ interdit");
                }
                let value = if self.eat_punct("=")? {
                    Some(self.field_initializer()?)
                } else {
                    None
                };
                self.semicolon()?;
                ClassMemberKind::Field(value)
            };
            members.push(ClassMember {
                key,
                is_static,
                kind,
            });
        }
        Ok((constructor, members))
    }

    // ---- Motifs ------------------------------------------------------------

    fn binding_pattern(&mut self) -> PResult<Pattern> {
        self.enter()?;
        let result = self.binding_pattern_inner();
        self.depth -= 1;
        result
    }

    fn binding_pattern_inner(&mut self) -> PResult<Pattern> {
        if self.eat_punct("{")? {
            let mut props = Vec::new();
            let mut rest = None;
            while !self.eat_punct("}")? {
                if self.eat_punct("...")? {
                    rest = Some(Box::new(Pattern::Ident(self.binding_ident()?)));
                    self.expect_punct("}")?;
                    break;
                }
                let is_plain_ident = matches!(&self.tok.tok, Tok::Ident(_));
                let key = self.property_key()?;
                let pattern = if self.eat_punct(":")? {
                    self.binding_pattern()?
                } else {
                    match &key {
                        PropKey::Name(name) if is_plain_ident && !RESERVED.contains(&&**name) => {
                            self.check_identifier(name, true)?;
                            Pattern::Ident(Arc::clone(name))
                        }
                        _ => return self.error("« : » attendu"),
                    }
                };
                let default = if self.eat_punct("=")? {
                    Some(self.assignment()?)
                } else {
                    None
                };
                props.push((key, pattern, default));
                if !self.is_punct("}") {
                    self.expect_punct(",")?;
                }
            }
            return Ok(Pattern::Object { props, rest });
        }
        if self.eat_punct("[")? {
            let mut elements = Vec::new();
            let mut rest = None;
            while !self.eat_punct("]")? {
                if self.eat_punct(",")? {
                    elements.push(None);
                    continue;
                }
                if self.eat_punct("...")? {
                    rest = Some(Box::new(self.binding_pattern()?));
                    self.expect_punct("]")?;
                    break;
                }
                let pattern = self.binding_pattern()?;
                let default = if self.eat_punct("=")? {
                    Some(self.assignment()?)
                } else {
                    None
                };
                elements.push(Some((pattern, default)));
                if !self.is_punct("]") {
                    self.expect_punct(",")?;
                }
            }
            return Ok(Pattern::Array { elements, rest });
        }
        Ok(Pattern::Ident(self.binding_ident()?))
    }

    /// Réinterprète une expression comme cible d'affectation (grammaire de
    /// couverture : `[a, b] = …`, `({x} = …)`, paramètres de fonction fléchée).
    fn to_pattern(&self, expr: Expr) -> PResult<Pattern> {
        Ok(match expr {
            Expr::Ident(name) => {
                self.check_assignable(&name)?;
                Pattern::Ident(name)
            }
            Expr::Member {
                optional: false, ..
            }
            | Expr::Index {
                optional: false, ..
            }
            | Expr::SuperMember(_) => Pattern::Expr(expr),
            Expr::Array(items) => {
                let mut elements = Vec::new();
                let mut rest = None;
                let count = items.len();
                for (i, item) in items.into_iter().enumerate() {
                    match item {
                        None => elements.push(None),
                        Some(Expr::Spread(inner)) if i + 1 == count => {
                            rest = Some(Box::new(self.to_pattern(*inner)?));
                        }
                        Some(other) => elements.push(Some(self.to_pattern_with_default(other)?)),
                    }
                }
                Pattern::Array { elements, rest }
            }
            Expr::Object(properties) => {
                let mut props = Vec::new();
                let mut rest = None;
                for property in properties {
                    match property.kind {
                        PropKind::Value(value) | PropKind::Proto(value) => {
                            let (pattern, default) = self.to_pattern_with_default(value)?;
                            props.push((property.key, pattern, default));
                        }
                        PropKind::Spread(inner) => rest = Some(Box::new(self.to_pattern(inner)?)),
                        _ => return self.error("cible d'affectation invalide"),
                    }
                }
                Pattern::Object { props, rest }
            }
            _ => return self.error("cible d'affectation invalide"),
        })
    }

    fn to_pattern_with_default(&self, expr: Expr) -> PResult<(Pattern, Option<Expr>)> {
        match expr {
            Expr::Assign {
                op: AssignOp::Assign,
                target,
                value,
            } => Ok((*target, Some(*value))),
            other => Ok((self.to_pattern(other)?, None)),
        }
    }

    // ---- Expressions -------------------------------------------------------

    fn expression(&mut self) -> PResult<Expr> {
        let first = self.assignment()?;
        if !self.is_punct(",") {
            return Ok(first);
        }
        let mut items = vec![first];
        while self.eat_punct(",")? {
            items.push(self.assignment()?);
        }
        Ok(Expr::Sequence(items))
    }

    fn arrow_body(
        &mut self,
        params: Vec<Param>,
        rest: Option<Pattern>,
        is_async: bool,
    ) -> PResult<Expr> {
        let start = self.fn_start.take();
        // Une fonction fléchée hérite de `super`, `new.target` et du mode
        // strict de la fonction qui l'entoure.
        let saved = self.take_state();
        let was_strict = saved.strict;
        self.no_in = false;
        self.in_async = is_async;
        self.in_generator = false;
        self.in_params = false;
        self.no_return = false;
        let block = self.is_punct("{");
        let body = if block {
            self.function_body().and_then(|(body, directive)| {
                self.check_params(None, &params, rest.as_ref(), true, directive, was_strict)?;
                Ok(body)
            })
        } else {
            self.check_params(None, &params, rest.as_ref(), true, false, was_strict)
                .and_then(|()| self.assignment())
                .map(|value| vec![Stmt::Return(Some(value))])
        };
        let strict = self.strict;
        self.restore_state(saved);
        let body = body?;
        if block {
            // L'accolade fermante, lue hors du mode strict éventuel du corps.
            self.advance()?;
        }
        Ok(Expr::Function(Arc::new(Function {
            source: self.span(start),
            name: None,
            params,
            rest,
            body,
            is_arrow: true,
            is_method: false,
            is_async,
            is_generator: false,
            strict,
            has_direct_eval: self.saw_eval,
        })))
    }

    /// Tente de lire `async x =>` ou `async (params) =>` ; restaure l'état et
    /// renvoie `None` si ce n'en est pas un (`async` est alors un identifiant).
    fn try_async_arrow(&mut self) -> PResult<Option<Expr>> {
        if self.peek()?.newline_before {
            return Ok(None);
        }
        let saved = (self.lexer.clone(), self.tok.clone(), self.cover_init);
        let start = self.tok.start;
        self.advance()?;
        let outer_eval = std::mem::replace(&mut self.saw_eval, false);
        let attempt: PResult<Option<(Vec<Param>, Option<Pattern>)>> = match &self.tok.tok {
            Tok::Ident(name) if !RESERVED.contains(&name.as_str()) => {
                match self.peek_is_punct("=>") {
                    Ok(true) => self.binding_ident().map(|name| {
                        Some((
                            vec![Param {
                                pattern: Pattern::Ident(name),
                                default: None,
                            }],
                            None,
                        ))
                    }),
                    Ok(false) => Ok(None),
                    Err(e) => Err(e),
                }
            }
            Tok::Punct("(") => self
                .params()
                .map(|parsed| (self.is_punct("=>") && !self.tok.newline_before).then_some(parsed)),
            _ => Ok(None),
        };
        match attempt {
            Ok(Some((params, rest))) => {
                self.advance()?;
                self.fn_start = Some(start);
                let arrow = self.arrow_body(params, rest, true);
                self.saw_eval = outer_eval;
                arrow.map(Some)
            }
            _ => {
                (self.lexer, self.tok, self.cover_init) = saved;
                self.saw_eval = outer_eval;
                Ok(None)
            }
        }
    }

    /// `yield`, `yield expr` ou `yield* expr` ; le jeton courant est `yield`.
    fn yield_expression(&mut self) -> PResult<Expr> {
        if self.in_params {
            return self.error("« yield » est interdit dans des paramètres");
        }
        self.advance()?;
        let delegate = !self.tok.newline_before && self.eat_punct("*")?;
        let ends = self.tok.newline_before
            || matches!(
                &self.tok.tok,
                Tok::Eof | Tok::Punct(")" | "]" | "}" | "," | ";" | ":")
            );
        let arg = if delegate || !ends {
            Some(Box::new(self.assignment()?))
        } else {
            None
        };
        Ok(Expr::Yield { arg, delegate })
    }

    /// Tente de lire `(params) =>` ; restaure l'état et renvoie `None` si ce n'en est pas un.
    fn try_arrow(&mut self) -> PResult<Option<Expr>> {
        let saved = (self.lexer.clone(), self.tok.clone(), self.cover_init);
        let start = self.tok.start;
        // Un `eval(…)` lu dans les paramètres ou le corps est celui de la
        // fonction fléchée, pas de la fonction qui l'entoure.
        let outer_eval = std::mem::replace(&mut self.saw_eval, false);
        let attempt = self
            .params()
            .map(|parsed| (self.is_punct("=>") && !self.tok.newline_before).then_some(parsed));
        match attempt {
            Ok(Some((params, rest))) => {
                self.advance()?;
                self.fn_start = Some(start);
                let arrow = self.arrow_body(params, rest, false);
                self.saw_eval = outer_eval;
                arrow.map(Some)
            }
            _ => {
                (self.lexer, self.tok, self.cover_init) = saved;
                self.saw_eval = outer_eval;
                Ok(None)
            }
        }
    }

    fn assignment(&mut self) -> PResult<Expr> {
        self.enter()?;
        let result = self.assignment_inner();
        self.depth -= 1;
        result
    }

    fn assignment_inner(&mut self) -> PResult<Expr> {
        if self.in_generator && self.is_kw("yield") {
            return self.yield_expression();
        }
        // Fonctions fléchées async : `async x => …`, `async (a, b) => …`.
        if self.is_kw("async")
            && let Some(arrow) = self.try_async_arrow()?
        {
            return Ok(arrow);
        }
        // Fonctions fléchées : `x => …`, `(a, b) => …`.
        match &self.tok.tok {
            Tok::Ident(name)
                if !RESERVED.contains(&name.as_str()) && self.peek_is_punct("=>")? =>
            {
                let start = self.tok.start;
                let name = self.binding_ident()?;
                if self.tok.newline_before {
                    return self.error("saut de ligne interdit avant « => »");
                }
                self.advance()?;
                self.fn_start = Some(start);
                let outer_eval = std::mem::replace(&mut self.saw_eval, false);
                let arrow = self.arrow_body(
                    vec![Param {
                        pattern: Pattern::Ident(name),
                        default: None,
                    }],
                    None,
                    false,
                );
                self.saw_eval = outer_eval;
                return arrow;
            }
            Tok::Ident(name)
                if (name == "yield" && !self.in_generator)
                    || (name == "await" && !self.in_async) =>
            {
                // Hors de leur contexte, ce sont de simples identifiants : les
                // faire suivre d'une expression est une erreur.
                if !matches!(self.peek()?.tok, Tok::Punct(_) | Tok::Eof) {
                    return self.error(format!(
                        "« {name} » n'est permis que dans {}",
                        if name == "yield" {
                            "un générateur"
                        } else {
                            "une fonction async"
                        }
                    ));
                }
            }
            Tok::Punct("(") => {
                if let Some(arrow) = self.try_arrow()? {
                    return Ok(arrow);
                }
            }
            _ => {}
        }
        // Un `{ a = 1 }` lu ici n'est valide que si l'expression se révèle
        // être un motif d'affectation ; sinon il reste en attente d'une erreur.
        let outer_cover = self.cover_init.take();
        let outer_rest = self.rest_comma.take();
        let left = self.conditional()?;
        let op = match &self.tok.tok {
            Tok::Punct(p) => assign_op(p),
            _ => None,
        };
        let Some(op) = op else {
            self.cover_init = outer_cover.or(self.cover_init);
            self.rest_comma = outer_rest.or(self.rest_comma);
            return Ok(left);
        };
        if op == AssignOp::Assign && matches!(left, Expr::Object(_) | Expr::Array(_)) {
            self.cover_init = None;
            if let Some(position) = self.rest_comma {
                return Err(self
                    .lexer
                    .error_at(position, "l'élément de reste doit être le dernier du motif"));
            }
        }
        self.rest_comma = None;
        self.cover_init = outer_cover.or(self.cover_init);
        let target = match (op, left) {
            (AssignOp::Assign, left) => self.to_pattern(left)?,
            (_, Expr::Ident(name)) => {
                self.check_assignable(&name)?;
                Pattern::Ident(name)
            }
            (
                _,
                left @ (Expr::Member {
                    optional: false, ..
                }
                | Expr::Index {
                    optional: false, ..
                }
                | Expr::SuperMember(_)),
            ) => Pattern::Expr(left),
            _ => return self.error("cible d'affectation invalide"),
        };
        self.advance()?;
        let value = self.assignment()?;
        // La valeur affectée n'est pas un motif : ses littéraux sont libres.
        self.rest_comma = outer_rest;
        Ok(Expr::Assign {
            op,
            target: Box::new(target),
            value: Box::new(value),
        })
    }

    fn conditional(&mut self) -> PResult<Expr> {
        let test = self.binary(0)?;
        if !self.eat_punct("?")? {
            return Ok(test);
        }
        let saved_no_in = std::mem::replace(&mut self.no_in, false);
        let then = self.assignment();
        self.no_in = saved_no_in;
        let then = then?;
        self.expect_punct(":")?;
        let otherwise = self.assignment()?;
        Ok(Expr::Cond {
            test: Box::new(test),
            then: Box::new(then),
            otherwise: Box::new(otherwise),
        })
    }

    /// Opérateur binaire ou logique au jeton courant, avec sa priorité.
    fn current_operator(&self) -> Option<(u8, Result<BinaryOp, LogicalOp>)> {
        match &self.tok.tok {
            Tok::Punct("??") => Some((1, Err(LogicalOp::Nullish))),
            Tok::Punct("||") => Some((2, Err(LogicalOp::Or))),
            Tok::Punct("&&") => Some((3, Err(LogicalOp::And))),
            Tok::Punct(p) => binary_op(p).map(|(prec, op)| (prec, Ok(op))),
            Tok::Ident(_) if self.tok.escaped => None,
            Tok::Ident(w) if w == "instanceof" => Some((8, Ok(BinaryOp::Instanceof))),
            Tok::Ident(w) if w == "in" && !self.no_in => Some((8, Ok(BinaryOp::In))),
            _ => None,
        }
    }

    fn binary(&mut self, min_prec: u8) -> PResult<Expr> {
        let mut left = self.unary()?;
        while let Some((prec, op)) = self.current_operator() {
            if prec < min_prec {
                break;
            }
            self.advance()?;
            // `**` est associatif à droite, les autres à gauche.
            let next_min = if op == Ok(BinaryOp::Exp) {
                prec
            } else {
                prec + 1
            };
            let right = self.binary(next_min)?;
            left = match op {
                Ok(op) => Expr::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                Err(op) => Expr::Logical {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            };
        }
        Ok(left)
    }

    fn unary(&mut self) -> PResult<Expr> {
        self.enter()?;
        let result = self.unary_inner();
        self.depth -= 1;
        result
    }

    fn unary_inner(&mut self) -> PResult<Expr> {
        if self.in_async && self.is_kw("await") {
            if self.in_params {
                return self.error("« await » est interdit dans des paramètres");
            }
            self.top_level_await |= self.module && !self.in_function;
            self.advance()?;
            let arg = self.unary()?;
            return Ok(Expr::Await(Box::new(arg)));
        }
        let op = match &self.tok.tok {
            Tok::Punct("!") => Some(UnaryOp::Not),
            Tok::Punct("~") => Some(UnaryOp::BitNot),
            Tok::Punct("+") => Some(UnaryOp::Plus),
            Tok::Punct("-") => Some(UnaryOp::Neg),
            Tok::Ident(_) if self.tok.escaped => None,
            Tok::Ident(w) if w == "typeof" => Some(UnaryOp::Typeof),
            Tok::Ident(w) if w == "void" => Some(UnaryOp::Void),
            Tok::Ident(w) if w == "delete" => Some(UnaryOp::Delete),
            _ => None,
        };
        if let Some(op) = op {
            self.advance()?;
            let arg = self.unary()?;
            if op == UnaryOp::Delete {
                match &arg {
                    Expr::Ident(_) if self.strict => {
                        return self.error("« delete » d'une variable interdit en mode strict");
                    }
                    Expr::Index { index, .. } if matches!(**index, Expr::PrivateName(_)) => {
                        return self.error("« delete » d'un membre privé interdit");
                    }
                    _ => {}
                }
            }
            if self.is_punct("**") {
                return self.error("opérande unaire interdit à gauche de « ** »");
            }
            return Ok(Expr::Unary {
                op,
                arg: Box::new(arg),
            });
        }
        if self.is_punct("++") || self.is_punct("--") {
            let increment = self.is_punct("++");
            self.advance()?;
            let target = self.unary()?;
            return self.update(increment, true, target);
        }
        let expr = self.left_hand_side()?;
        if (self.is_punct("++") || self.is_punct("--")) && !self.tok.newline_before {
            let increment = self.is_punct("++");
            self.advance()?;
            return self.update(increment, false, expr);
        }
        Ok(expr)
    }

    fn update(&self, increment: bool, prefix: bool, target: Expr) -> PResult<Expr> {
        if !matches!(
            target,
            Expr::Ident(_)
                | Expr::Member {
                    optional: false,
                    ..
                }
                | Expr::Index {
                    optional: false,
                    ..
                }
                | Expr::SuperMember(_)
        ) {
            return self.error("cible d'incrémentation invalide");
        }
        if let Expr::Ident(name) = &target {
            self.check_assignable(name)?;
        }
        Ok(Expr::Update {
            increment,
            prefix,
            target: Box::new(target),
        })
    }

    fn arguments(&mut self) -> PResult<Vec<Expr>> {
        self.expect_punct("(")?;
        let mut args = Vec::new();
        while !self.eat_punct(")")? {
            if self.eat_punct("...")? {
                args.push(Expr::Spread(Box::new(self.assignment()?)));
            } else {
                args.push(self.assignment()?);
            }
            if !self.is_punct(")") {
                self.expect_punct(",")?;
            }
        }
        Ok(args)
    }

    /// Expression « membre gauche » : primaire suivi d'accès, d'appels et de `new`.
    fn left_hand_side(&mut self) -> PResult<Expr> {
        let mut expr = if self.is_kw("new") {
            self.advance()?;
            if self.eat_punct(".")? {
                self.expect_kw("target")?;
                if !self.in_function {
                    return self.error("« new.target » hors d'une fonction");
                }
                Expr::NewTarget
            } else {
                self.enter()?;
                // Le constructeur est une expression membre sans appel.
                let mut callee = if self.is_kw("new") {
                    self.left_hand_side()?
                } else if self.is_kw("import")
                    && (self.peek_is_punct("(")? || self.import_phase_follows()?)
                {
                    return self.error("« new import() » est interdit");
                } else {
                    self.primary()?
                };
                loop {
                    if self.eat_punct(".")? {
                        callee = Self::member(callee, self.property_name()?, false);
                    } else if self.eat_punct("[")? {
                        let index = self.expression()?;
                        self.expect_punct("]")?;
                        callee = Expr::Index {
                            object: Box::new(callee),
                            index: Box::new(index),
                            optional: false,
                        };
                    } else {
                        break;
                    }
                }
                self.depth -= 1;
                let args = if self.is_punct("(") {
                    self.arguments()?
                } else {
                    Vec::new()
                };
                Expr::New {
                    callee: Box::new(callee),
                    args,
                }
            }
        } else if self.is_kw("super") {
            self.advance()?;
            if self.is_punct("(") {
                if !self.super_call {
                    return self.error("« super() » hors du constructeur d'une classe dérivée");
                }
                Expr::SuperCall(self.arguments()?)
            } else if !self.super_property {
                return self.error("« super » hors d'une méthode");
            } else if self.eat_punct(".")? {
                if matches!(self.tok.tok, Tok::Private(_)) {
                    return self.error("nom privé interdit après « super. »");
                }
                Expr::SuperMember(PropKey::Name(self.property_name()?))
            } else if self.eat_punct("[")? {
                let index = self.expression()?;
                self.expect_punct("]")?;
                Expr::SuperMember(PropKey::Computed(Box::new(index)))
            } else {
                return self.error("« super » doit être suivi d'un appel ou d'un accès");
            }
        } else {
            self.primary()?
        };
        let mut in_optional_chain = false;
        loop {
            if self.eat_punct(".")? {
                expr = Self::member(expr, self.property_name()?, false);
            } else if self.eat_punct("?.")? {
                in_optional_chain = true;
                if self.is_punct("(") {
                    expr = Expr::Call {
                        callee: Box::new(expr),
                        args: self.arguments()?,
                        optional: true,
                    };
                } else if self.eat_punct("[")? {
                    let index = self.expression()?;
                    self.expect_punct("]")?;
                    expr = Expr::Index {
                        object: Box::new(expr),
                        index: Box::new(index),
                        optional: true,
                    };
                } else {
                    expr = Self::member(expr, self.property_name()?, true);
                }
            } else if self.eat_punct("[")? {
                let saved_no_in = std::mem::replace(&mut self.no_in, false);
                let index = self.expression();
                self.no_in = saved_no_in;
                self.expect_punct("]")?;
                expr = Expr::Index {
                    object: Box::new(expr),
                    index: Box::new(index?),
                    optional: false,
                };
            } else if self.is_punct("(") {
                if matches!(&expr, Expr::Ident(name) if &**name == "eval") {
                    self.saw_eval = true;
                }
                let saved_no_in = std::mem::replace(&mut self.no_in, false);
                let args = self.arguments();
                self.no_in = saved_no_in;
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args: args?,
                    optional: false,
                };
            } else if matches!(self.tok.tok, Tok::Template { .. }) {
                if in_optional_chain {
                    return self.error("gabarit étiqueté interdit dans une chaîne optionnelle");
                }
                let (cooked, raw, exprs) = self.template_parts()?;
                expr = Expr::TaggedTemplate {
                    tag: Box::new(expr),
                    cooked,
                    raw,
                    exprs,
                };
            } else {
                break;
            }
        }
        Ok(if in_optional_chain {
            Expr::OptionalChain(Box::new(expr))
        } else {
            expr
        })
    }

    fn template(&mut self) -> PResult<Expr> {
        let start = self.tok.start;
        let (cooked, _, exprs) = self.template_parts()?;
        // Sans étiquette, tout échappement doit être valide.
        let Some(quasis) = cooked.into_iter().collect::<Option<Vec<_>>>() else {
            return Err(self
                .lexer
                .error_at(start, "séquence d'échappement invalide dans un gabarit"));
        };
        Ok(Expr::Template { quasis, exprs })
    }

    /// Morceaux d'un gabarit (interprétés et bruts) et ses substitutions.
    #[expect(clippy::type_complexity, reason = "triplet interne à l'analyseur")]
    fn template_parts(&mut self) -> PResult<(Vec<Option<Arc<str>>>, Vec<Arc<str>>, Vec<Expr>)> {
        let mut cooked_parts = Vec::new();
        let mut raw_parts = Vec::new();
        let mut exprs = Vec::new();
        loop {
            let Tok::Template { cooked, raw, tail } = &self.tok.tok else {
                return self.error("gabarit mal formé");
            };
            cooked_parts.push(cooked.as_deref().map(Arc::from));
            raw_parts.push(Arc::from(raw.as_str()));
            if *tail {
                self.advance()?;
                return Ok((cooked_parts, raw_parts, exprs));
            }
            self.advance()?;
            exprs.push(self.expression()?);
            if !self.is_punct("}") {
                return self.error("« } » attendu dans le gabarit");
            }
            // Le lexer est positionné juste après le `}` : on reprend le gabarit.
            self.tok = self.lexer.template_continue()?;
        }
    }

    fn primary(&mut self) -> PResult<Expr> {
        match &self.tok.tok {
            Tok::Num(n) => {
                let n = *n;
                self.advance()?;
                Ok(Expr::Number(n))
            }
            Tok::BigInt(big) => {
                let big = Arc::clone(big);
                self.advance()?;
                Ok(Expr::BigInt(big))
            }
            // `#x in obj` : seul emploi d'un nom privé hors d'un accès.
            Tok::Private(name) => {
                let bare = name.clone();
                self.use_private(&bare, self.tok.start)?;
                let name: Arc<str> = Arc::from(format!("#{bare}"));
                self.advance()?;
                if !self.is_kw("in") {
                    return self.error("nom privé inattendu");
                }
                Ok(Expr::PrivateName(name))
            }
            Tok::Str(s) => {
                let s: Arc<str> = Arc::from(s.as_str());
                self.advance()?;
                Ok(Expr::String(s))
            }
            Tok::Template { .. } => self.template(),
            Tok::Punct("/" | "/=") => {
                self.tok = self.lexer.regex_at(self.tok.start)?;
                let Tok::Regex { pattern, flags } = &self.tok.tok else {
                    return self.error("expression rationnelle mal formée");
                };
                // Une expression rationnelle mal formée est une erreur
                // précoce ; ce que le moteur ne sait pas encore traiter n'est
                // signalé qu'à l'exécution.
                if let Err(message) = crate::regexp::compile(pattern, flags)
                    && !message.contains("non pris en charge")
                {
                    return self.error(message);
                }
                let expr = Expr::Regex {
                    pattern: Arc::from(pattern.as_str()),
                    flags: Arc::from(flags.as_str()),
                };
                self.advance()?;
                Ok(expr)
            }
            Tok::Punct("(") => {
                self.advance()?;
                let saved_no_in = std::mem::replace(&mut self.no_in, false);
                let expr = self.expression();
                self.no_in = saved_no_in;
                self.expect_punct(")")?;
                expr
            }
            Tok::Punct("[") => {
                self.advance()?;
                let saved_no_in = std::mem::replace(&mut self.no_in, false);
                let items = self.array_items();
                self.no_in = saved_no_in;
                Ok(Expr::Array(items?))
            }
            Tok::Punct("{") => {
                self.advance()?;
                let saved_no_in = std::mem::replace(&mut self.no_in, false);
                let props = self.object_properties();
                self.no_in = saved_no_in;
                Ok(Expr::Object(props?))
            }
            Tok::Ident(word) => match word.as_str() {
                w if self.tok.escaped && RESERVED.contains(&w) => {
                    self.error(format!("mot-clé « {w} » écrit avec un échappement"))
                }
                "true" | "false" => {
                    let value = word == "true";
                    self.advance()?;
                    Ok(Expr::Bool(value))
                }
                "null" => {
                    self.advance()?;
                    Ok(Expr::Null)
                }
                "this" => {
                    self.advance()?;
                    Ok(Expr::This)
                }
                "function" => {
                    let start = self.tok.start;
                    self.advance()?;
                    let is_generator = self.eat_punct("*")?;
                    let name = self.function_expression_name(false, is_generator)?;
                    self.fn_start = Some(start);
                    Ok(Expr::Function(Arc::new(self.function_with(
                        name,
                        false,
                        false,
                        is_generator,
                    )?)))
                }
                "async" if self.async_function_follows()? => {
                    let start = self.tok.start;
                    self.advance()?;
                    self.advance()?;
                    let is_generator = self.eat_punct("*")?;
                    let name = self.function_expression_name(true, is_generator)?;
                    self.fn_start = Some(start);
                    Ok(Expr::Function(Arc::new(self.function_with(
                        name,
                        false,
                        true,
                        is_generator,
                    )?)))
                }
                "class" => {
                    let start = self.tok.start;
                    self.advance()?;
                    Ok(Expr::Class(Box::new(self.class_rest(false, start)?)))
                }
                "import" if self.peek_is_punct("(")? => {
                    self.advance()?;
                    self.advance()?;
                    let saved_no_in = std::mem::replace(&mut self.no_in, false);
                    let parsed = self.import_arguments();
                    self.no_in = saved_no_in;
                    let (specifier, options) = parsed?;
                    Ok(Expr::ImportCall(
                        Box::new(specifier),
                        options.map(Box::new),
                        0,
                    ))
                }
                "import" if self.peek_is_punct(".")? => {
                    self.advance()?;
                    self.advance()?;
                    // `import.source(x)` et `import.defer(x)` : un seul argument.
                    let phase = match &self.tok.tok {
                        Tok::Ident(word) if !self.tok.escaped && word == "source" => Some(1),
                        Tok::Ident(word) if !self.tok.escaped && word == "defer" => Some(2),
                        _ => None,
                    };
                    if let Some(source) = phase
                        && self.peek_is_punct("(")?
                    {
                        self.advance()?;
                        self.advance()?;
                        let saved_no_in = std::mem::replace(&mut self.no_in, false);
                        let specifier = self.assignment();
                        self.no_in = saved_no_in;
                        let specifier = specifier?;
                        self.expect_punct(")")?;
                        return Ok(Expr::ImportCall(Box::new(specifier), None, source));
                    }
                    if !self.module || !self.eat_kw("meta")? {
                        return self.error("« meta » attendu après « import. »");
                    }
                    // Lu comme une liaison de la portée du module (voir `module.rs`).
                    Ok(Expr::Ident(Arc::from("import.meta")))
                }
                w if RESERVED.contains(&w) => self.error(format!("« {w} » inattendu")),
                _ => Ok(Expr::Ident(self.identifier(false)?)),
            },
            Tok::Eof => self.error("fin de script inattendue"),
            _ => self.error("expression attendue"),
        }
    }

    /// Arguments de `import(…)` après la parenthèse ouvrante : le
    /// spécificateur, puis des options facultatives.
    fn import_arguments(&mut self) -> PResult<(Expr, Option<Expr>)> {
        let specifier = self.assignment()?;
        let mut options = None;
        if self.eat_punct(",")? && !self.is_punct(")") {
            options = Some(self.assignment()?);
            self.eat_punct(",")?;
        }
        self.expect_punct(")")?;
        Ok((specifier, options))
    }

    fn array_items(&mut self) -> PResult<Vec<Option<Expr>>> {
        let mut items = Vec::new();
        while !self.eat_punct("]")? {
            if self.eat_punct(",")? {
                items.push(None);
                continue;
            }
            if self.eat_punct("...")? {
                items.push(Some(Expr::Spread(Box::new(self.assignment()?))));
                if self.is_punct(",") {
                    self.rest_comma.get_or_insert(self.tok.start);
                }
            } else {
                items.push(Some(self.assignment()?));
            }
            if !self.is_punct("]") {
                self.expect_punct(",")?;
            }
        }
        Ok(items)
    }

    fn object_properties(&mut self) -> PResult<Vec<Property>> {
        let mut props = Vec::new();
        let mut has_proto = false;
        while !self.eat_punct("}")? {
            if self.eat_punct("...")? {
                let value = self.assignment()?;
                if self.is_punct(",") {
                    self.rest_comma.get_or_insert(self.tok.start);
                }
                props.push(Property {
                    key: PropKey::Name(Arc::from("")),
                    kind: PropKind::Spread(value),
                });
            } else {
                // `__proto__: valeur` ne peut figurer qu'une fois, sauf dans
                // un motif d'affectation.
                let is_proto = matches!(&self.tok.tok, Tok::Ident(n) | Tok::Str(n) if n == "__proto__")
                    && self.peek_is_punct(":")?;
                if is_proto && std::mem::replace(&mut has_proto, true) {
                    self.cover_init.get_or_insert(self.tok.start);
                }
                props.push(self.object_property()?);
            }
            if !self.is_punct("}") {
                self.expect_punct(",")?;
            }
        }
        Ok(props)
    }

    fn object_property(&mut self) -> PResult<Property> {
        let start = self.tok.start;
        let (is_async, is_generator) = self.method_modifiers()?;
        let accessor = match &self.tok.tok {
            Tok::Ident(w)
                if !self.tok.escaped && (w == "get" || w == "set") && self.is_modifier()? =>
            {
                Some(w == "get")
            }
            _ => None,
        };
        if let Some(is_getter) = accessor {
            self.advance()?;
            let key = self.property_key()?;
            self.fn_start = Some(start);
            let name = Self::accessor_name(&key, accessor);
            let func = Arc::new(self.function_rest(name, true)?);
            self.check_accessor(&func, is_getter)?;
            let kind = if is_getter {
                PropKind::Getter(func)
            } else {
                PropKind::Setter(func)
            };
            return Ok(Property { key, kind });
        }
        let shorthand = match &self.tok.tok {
            Tok::Ident(name) if !RESERVED.contains(&name.as_str()) => {
                Some(Arc::<str>::from(name.as_str()))
            }
            _ => None,
        };
        let key_start = self.tok.start;
        if matches!(self.tok.tok, Tok::Private(_)) {
            return self.error("nom privé interdit dans un littéral objet");
        }
        let key = self.property_key()?;
        if self.is_punct("(") {
            self.fn_start = Some(start);
            let func = Arc::new(self.function_with(
                Self::method_name(&key),
                true,
                is_async,
                is_generator,
            )?);
            return Ok(Property {
                key,
                kind: PropKind::Value(Expr::Function(func)),
            });
        }
        if is_async || is_generator {
            return self.error("« ( » attendu");
        }
        if self.eat_punct(":")? {
            let value = self.assignment()?;
            let is_proto = matches!(&key, PropKey::Name(name) if &**name == "__proto__");
            let kind = if is_proto {
                PropKind::Proto(value)
            } else {
                PropKind::Value(value)
            };
            return Ok(Property { key, kind });
        }
        let Some(name) = shorthand else {
            return self.error("« : » attendu");
        };
        self.check_identifier(&name, false)?;
        // `{ a }` ou, dans un motif, `{ a = défaut }`.
        let value = if self.eat_punct("=")? {
            self.check_assignable(&name)?;
            self.cover_init.get_or_insert(key_start);
            let default = self.assignment()?;
            Expr::Assign {
                op: AssignOp::Assign,
                target: Box::new(Pattern::Ident(Arc::clone(&name))),
                value: Box::new(default),
            }
        } else {
            Expr::Ident(name)
        };
        Ok(Property {
            key,
            kind: PropKind::Value(value),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> Vec<Stmt> {
        parse_program(src)
            .unwrap_or_else(|e| panic!("{src}\n→ {e}"))
            .body
    }

    fn fails(src: &str) -> bool {
        parse_program(src).is_err()
    }

    #[test]
    fn statements_parse() {
        let program = parse(
            "var a = 1, b; let [c, , ...d] = [1, 2, 3]; const {e, f: {g = 2}, ...h} = o;
             function f(x, y = 2, ...z) { return x + y }
             if (a) b = 1; else if (c) { b = 2 } else b = 3
             for (let i = 0; i < 3; i++) continue;
             for (const k in o) {} for (const v of xs) {} for (x of xs) ; for (;;) break
             outer: while (true) { do { break outer } while (0) }
             switch (a) { case 1: case 2: b; break; default: c }
             try { throw new Error('x') } catch ({message}) { } finally { }
             try { } catch { }
             class A extends B { static s = 1; #p = 2; constructor() { super(); } get x() { return this.#p } static m() {} ['k']() {} }",
        );
        assert_eq!(program.len(), 15);
    }

    #[test]
    fn expressions_parse() {
        parse(
            "a = b ? c : d ? e : f; x = y ?? z || w && v | u ^ t & s == r < q << p + o * n ** m ** l;
             f(a, ...b)(c)[d].e?.f?.[g]?.(h); new A.B(c).d; new A; new new A()();
             (a, b) => a + b; x => ({ y: x }); () => {}; ({ a, b = 1, ...c }) => a; ([a, [b]]) => b;
             `a${b}c${`d${e}`}`; /re[/]+/gi.test(s); a / b / c; x = /=/;
             o = { a, b: 1, 'c': 2, 3: 4, [k]: 5, m() {}, get g() { return 1 }, set g(v) {}, get: 1, set, ...p };
             typeof a === 'undefined'; delete a.b; void 0; (-a) ** 2 === 0 || !b; a++ + ++b; i in o; a instanceof B;
             [a, b] = [b, a]; ({ a, b: [c] } = o); a ||= b; a.b ??= c; a[b] **= 2;
             (function () {})(); (function named() {}); (class {}); function nt() { new.target }",
        );
    }

    #[test]
    fn automatic_semicolon_insertion() {
        let program = parse("let a = 1\nlet b = 2\na\n++b\nreturn\n1");
        assert_eq!(program.len(), 6);
        assert!(matches!(&program[2], Stmt::Expr(Expr::Ident(_))));
        assert!(matches!(
            &program[3],
            Stmt::Expr(Expr::Update { prefix: true, .. })
        ));
        assert!(matches!(&program[4], Stmt::Return(None)));
        assert!(fails("a b"));
        assert!(fails("throw\n1"));
    }

    #[test]
    fn precedence_and_associativity() {
        let Stmt::Expr(Expr::Binary {
            op: BinaryOp::Add,
            right,
            ..
        }) = &parse("1 + 2 * 3")[0]
        else {
            panic!("addition attendue à la racine");
        };
        assert!(matches!(
            **right,
            Expr::Binary {
                op: BinaryOp::Mul,
                ..
            }
        ));
        let Stmt::Expr(Expr::Binary {
            op: BinaryOp::Exp,
            right,
            ..
        }) = &parse("2 ** 3 ** 2")[0]
        else {
            panic!("puissance attendue à la racine");
        };
        assert!(
            matches!(
                **right,
                Expr::Binary {
                    op: BinaryOp::Exp,
                    ..
                }
            ),
            "associative à droite"
        );
        let Stmt::Expr(Expr::Binary {
            op: BinaryOp::Sub,
            left,
            ..
        }) = &parse("1 - 2 - 3")[0]
        else {
            panic!("soustraction attendue à la racine");
        };
        assert!(
            matches!(
                **left,
                Expr::Binary {
                    op: BinaryOp::Sub,
                    ..
                }
            ),
            "associative à gauche"
        );
    }

    #[test]
    fn syntax_errors_are_reported() {
        for src in [
            "const a",
            "a +",
            "(a",
            "f(",
            "{",
            "1 = 2",
            "a++ = 1",
            "for (;;",
            "if (a",
            "x = {a:}",
            "class { constructor(){} constructor(){} }",
            "try {}",
            "switch (a) { default: default: }",
            "let [a]",
            "break\nfoo bar",
            "`a${",
            "super",
            "new.x",
            "a => \n",
            "({a:1} = 1) => 1",
        ] {
            assert!(fails(src), "{src:?} devrait être refusé");
        }
    }

    #[test]
    fn unsupported_syntax_says_so() {
        for src in ["import x from 'y'", "export default 1"] {
            let message = parse_program(src).expect_err("non pris en charge").message;
            assert!(
                message.contains("non pris en charge"),
                "{src:?} → {message}"
            );
        }
    }

    #[test]
    fn nesting_is_bounded() {
        assert!(fails(&format!("{}1{}", "(".repeat(5000), ")".repeat(5000))));
        assert!(fails(&"[".repeat(5000)));
        assert!(fails(&format!("x = {}", "a ? b : ".repeat(5000))));
        assert!(fails(&"{".repeat(5000)));
    }

    #[test]
    fn never_panics_on_truncated_input() {
        let src = "class A extends B { static #x = `a${1}`; get y() { return super.y?.[0] ?? /r/g } }
                   for (const [k, {v = 1}] of m) try { o = {...p, [k]: (a, ...b) => a ** -b} } catch { }";
        let chars: Vec<char> = src.chars().collect();
        for n in 0..=chars.len() {
            let prefix: String = chars[..n].iter().collect();
            let _ = parse_program(&prefix);
        }
    }
}
