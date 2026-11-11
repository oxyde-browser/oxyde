//! Analyse syntaxique d'ECMAScript : descente récursive et priorités d'opérateurs.
//!
//! LIMITES CONNUES : pas de générateurs async ni de `for await`, pas de modules
//! (`import` / `export`), de gabarits étiquetés ni de `with`. Les noms privés
//! (`#x`) sont traités comme des propriétés ordinaires nommées `#x`. Le mode
//! strict n'est pas distingué du mode normal.

use std::sync::Arc;

use crate::ast::{
    AssignOp, BinaryOp, Class, ClassMember, ClassMemberKind, Expr, ForInit, ForTarget, Function,
    LogicalOp, Param, Pattern, PropKey, PropKind, Property, Stmt, UnaryOp, VarKind,
};
use crate::lexer::{Lexer, SyntaxError, Tok, Token};

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

type PResult<T> = Result<T, SyntaxError>;

struct Parser {
    lexer: Lexer,
    tok: Token,
    depth: usize,
    /// Vrai pendant l'analyse de l'initialisation d'un `for` : `in` n'y est pas un opérateur.
    no_in: bool,
    /// Vrai dans le corps d'une fonction async : `await` y est un opérateur.
    in_async: bool,
    /// Vrai dans le corps d'un générateur : `yield` y est un opérateur.
    in_generator: bool,
}

/// Analyse un script complet.
pub(crate) fn parse_program(source: &str) -> PResult<Vec<Stmt>> {
    let mut lexer = Lexer::new(source);
    let tok = lexer.next()?;
    let mut parser = Parser {
        lexer,
        tok,
        depth: 0,
        no_in: false,
        in_async: false,
        in_generator: false,
    };
    let mut body = Vec::new();
    while parser.tok.tok != Tok::Eof {
        body.push(parser.statement()?);
    }
    Ok(body)
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
    // ---- Jetons ----------------------------------------------------------

    fn error<T>(&self, message: impl Into<String>) -> PResult<T> {
        Err(self.lexer.error_at(self.tok.start, message))
    }

    fn unsupported<T>(&self, what: &str) -> PResult<T> {
        self.error(format!(
            "{what} : non pris en charge par Oxyde pour l'instant"
        ))
    }

    fn advance(&mut self) -> PResult<Token> {
        let next = self.lexer.next()?;
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

    fn is_kw(&self, k: &str) -> bool {
        matches!(&self.tok.tok, Tok::Ident(name) if name == k)
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

    /// Identifiant de liaison (pas un mot réservé).
    fn binding_ident(&mut self) -> PResult<Arc<str>> {
        match &self.tok.tok {
            Tok::Ident(name) if !RESERVED.contains(&name.as_str()) => {
                let name: Arc<str> = Arc::from(name.as_str());
                self.advance()?;
                Ok(name)
            }
            _ => self.error("identifiant attendu"),
        }
    }

    /// Nom de propriété après un point : les mots réservés y sont permis.
    fn property_name(&mut self) -> PResult<Arc<str>> {
        let name: Arc<str> = match &self.tok.tok {
            Tok::Ident(name) => Arc::from(name.as_str()),
            Tok::Private(name) => Arc::from(format!("#{name}")),
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
        result
    }

    fn block(&mut self) -> PResult<Vec<Stmt>> {
        self.expect_punct("{")?;
        let mut body = Vec::new();
        while !self.is_punct("}") {
            if self.tok.tok == Tok::Eof {
                return self.error("« } » attendu");
            }
            body.push(self.statement()?);
        }
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
            "function" => {
                self.advance()?;
                let is_generator = self.eat_punct("*")?;
                let name = self.binding_ident()?;
                Ok(Stmt::Function(Arc::new(self.function_with(
                    Some(name),
                    false,
                    false,
                    is_generator,
                )?)))
            }
            "async" if self.async_function_follows()? => {
                self.advance()?;
                self.advance()?;
                if self.is_punct("*") {
                    return self.unsupported("les générateurs async");
                }
                let name = self.binding_ident()?;
                Ok(Stmt::Function(Arc::new(self.function_with(
                    Some(name),
                    false,
                    true,
                    false,
                )?)))
            }
            "class" => {
                self.advance()?;
                Ok(Stmt::Class(Box::new(self.class_rest(true)?)))
            }
            "if" => {
                self.advance()?;
                self.expect_punct("(")?;
                let test = self.expression()?;
                self.expect_punct(")")?;
                let then = Box::new(self.statement()?);
                let otherwise = if self.eat_kw("else")? {
                    Some(Box::new(self.statement()?))
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
                    body: Box::new(self.statement()?),
                })
            }
            "do" => {
                self.advance()?;
                let body = Box::new(self.statement()?);
                self.expect_kw("while")?;
                self.expect_punct("(")?;
                let test = self.expression()?;
                self.expect_punct(")")?;
                self.eat_punct(";")?;
                Ok(Stmt::DoWhile { body, test })
            }
            "return" => {
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
                    Tok::Ident(_) if !self.tok.newline_before => Some(self.binding_ident()?),
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
            "with" => self.unsupported("l'instruction with"),
            _ if !RESERVED.contains(&word.as_str()) && self.peek_is_punct(":")? => {
                let label = self.binding_ident()?;
                self.advance()?;
                Ok(Stmt::Labeled(label, Box::new(self.statement()?)))
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
            let init = if self.eat_punct("=")? {
                Some(self.assignment()?)
            } else {
                None
            };
            if init.is_none() && !self.no_in {
                if kind == VarKind::Const {
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
        if self.is_kw("await") {
            return self.unsupported("for await");
        }
        self.expect_punct("(")?;
        let mut init = None;
        if !self.is_punct(";") {
            let decl_kind = match &self.tok.tok {
                Tok::Ident(w) if w == "var" => Some(VarKind::Var),
                Tok::Ident(w) if w == "const" => Some(VarKind::Const),
                Tok::Ident(w)
                    if w == "let"
                        && matches!(self.peek()?.tok, Tok::Ident(_) | Tok::Punct("[" | "{")) =>
                {
                    Some(VarKind::Let)
                }
                _ => None,
            };
            self.no_in = true;
            let parsed = match decl_kind {
                Some(kind) => {
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
                        if decls.len() != 1 || decls[0].1.is_some() {
                            return self.error("déclaration invalide dans for…in / for…of");
                        }
                        ForTarget::Decl(kind, decls.remove(0).0)
                    }
                    ForInit::Expr(expr) => ForTarget::Pattern(self.to_pattern(expr)?),
                };
                let object = if of {
                    self.assignment()?
                } else {
                    self.expression()?
                };
                self.expect_punct(")")?;
                return Ok(Stmt::ForIn {
                    target,
                    object,
                    body: Box::new(self.statement()?),
                    of,
                });
            }
            if let ForInit::Decl(kind, decls) = &parsed {
                for (pattern, value) in decls {
                    if value.is_none()
                        && (*kind == VarKind::Const || !matches!(pattern, Pattern::Ident(_)))
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
            body: Box::new(self.statement()?),
        })
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
            handler = Some(self.block()?);
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
            let mut body = Vec::new();
            while !self.is_kw("case") && !self.is_kw("default") && !self.is_punct("}") {
                if self.tok.tok == Tok::Eof {
                    return self.error("« } » attendu");
                }
                body.push(self.statement()?);
            }
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
        let saved = (self.in_async, self.in_generator, self.no_in);
        (self.in_async, self.in_generator, self.no_in) = (is_async, is_generator, false);
        let parsed = self
            .params()
            .and_then(|(params, rest)| self.block().map(|body| (params, rest, body)));
        (self.in_async, self.in_generator, self.no_in) = saved;
        let (params, rest, body) = parsed?;
        Ok(Function {
            name,
            params,
            rest,
            body,
            is_arrow: false,
            is_method,
            is_async,
            is_generator,
        })
    }

    /// Vrai si le jeton courant `async` est suivi, sur la même ligne, de `function`.
    fn async_function_follows(&self) -> PResult<bool> {
        let after = self.peek()?;
        Ok(!after.newline_before && matches!(after.tok, Tok::Ident(ref w) if w == "function"))
    }

    /// Modificateurs `async` et `*` placés devant le nom d'une méthode.
    fn method_modifiers(&mut self) -> PResult<(bool, bool)> {
        let mut is_async = false;
        if self.is_kw("async") {
            let after = self.peek()?;
            if !after.newline_before && matches!(after.tok, Tok::Punct("*")) {
                return self.unsupported("les générateurs async");
            }
            if !after.newline_before && self.is_modifier()? {
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
            Tok::Ident(_) | Tok::Private(_) | Tok::Str(_) | Tok::Num(_) | Tok::Punct("[")
        ))
    }

    fn method_name(key: &PropKey) -> Option<Arc<str>> {
        match key {
            PropKey::Name(name) => Some(Arc::clone(name)),
            PropKey::Computed(_) => None,
        }
    }

    fn class_rest(&mut self, name_required: bool) -> PResult<Class> {
        let name = match &self.tok.tok {
            Tok::Ident(w) if w != "extends" => Some(self.binding_ident()?),
            _ if name_required => return self.error("nom de classe attendu"),
            _ => None,
        };
        let parent = if self.eat_kw("extends")? {
            Some(self.left_hand_side()?)
        } else {
            None
        };
        self.expect_punct("{")?;
        let mut constructor = None;
        let mut members = Vec::new();
        while !self.eat_punct("}")? {
            if self.eat_punct(";")? {
                continue;
            }
            let is_static = self.is_kw("static") && self.is_modifier()?;
            if is_static {
                self.advance()?;
            }
            let (is_async, is_generator) = self.method_modifiers()?;
            let accessor = match &self.tok.tok {
                Tok::Ident(w) if (w == "get" || w == "set") && self.is_modifier()? => {
                    Some(w == "get")
                }
                _ => None,
            };
            if accessor.is_some() {
                self.advance()?;
            }
            let key = self.property_key()?;
            let kind = if self.is_punct("(") {
                let func = Arc::new(self.function_with(
                    Self::method_name(&key),
                    true,
                    is_async,
                    is_generator,
                )?);
                let is_constructor = !is_static
                    && accessor.is_none()
                    && matches!(&key, PropKey::Name(n) if &**n == "constructor");
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
                let value = if self.eat_punct("=")? {
                    Some(self.assignment()?)
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
        Ok(Class {
            name,
            parent,
            constructor,
            members,
        })
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
            Expr::Ident(name) => Pattern::Ident(name),
            Expr::Member {
                optional: false, ..
            }
            | Expr::Index {
                optional: false, ..
            } => Pattern::Expr(expr),
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
                        PropKind::Value(value) => {
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
        let saved = (self.in_async, self.in_generator, self.no_in);
        (self.in_async, self.in_generator, self.no_in) = (is_async, false, false);
        let body = if self.is_punct("{") {
            self.block()
        } else {
            self.assignment()
                .map(|value| vec![Stmt::Return(Some(value))])
        };
        (self.in_async, self.in_generator, self.no_in) = saved;
        Ok(Expr::Function(Arc::new(Function {
            name: None,
            params,
            rest,
            body: body?,
            is_arrow: true,
            is_method: false,
            is_async,
            is_generator: false,
        })))
    }

    /// Tente de lire `async x =>` ou `async (params) =>` ; restaure l'état et
    /// renvoie `None` si ce n'en est pas un (`async` est alors un identifiant).
    fn try_async_arrow(&mut self) -> PResult<Option<Expr>> {
        if self.peek()?.newline_before {
            return Ok(None);
        }
        let saved = (self.lexer.clone(), self.tok.clone());
        self.advance()?;
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
                self.arrow_body(params, rest, true).map(Some)
            }
            _ => {
                (self.lexer, self.tok) = saved;
                Ok(None)
            }
        }
    }

    /// `yield`, `yield expr` ou `yield* expr` ; le jeton courant est `yield`.
    fn yield_expression(&mut self) -> PResult<Expr> {
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
        let saved = (self.lexer.clone(), self.tok.clone());
        let attempt = self
            .params()
            .map(|parsed| (self.is_punct("=>") && !self.tok.newline_before).then_some(parsed));
        match attempt {
            Ok(Some((params, rest))) => {
                self.advance()?;
                self.arrow_body(params, rest, false).map(Some)
            }
            _ => {
                (self.lexer, self.tok) = saved;
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
                let name = self.binding_ident()?;
                self.advance()?;
                return self.arrow_body(
                    vec![Param {
                        pattern: Pattern::Ident(name),
                        default: None,
                    }],
                    None,
                    false,
                );
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
        let left = self.conditional()?;
        let Tok::Punct(p) = &self.tok.tok else {
            return Ok(left);
        };
        let Some(op) = assign_op(p) else {
            return Ok(left);
        };
        let target = match (op, left) {
            (AssignOp::Assign, left) => self.to_pattern(left)?,
            (_, Expr::Ident(name)) => Pattern::Ident(name),
            (
                _,
                left @ (Expr::Member {
                    optional: false, ..
                }
                | Expr::Index {
                    optional: false, ..
                }),
            ) => Pattern::Expr(left),
            _ => return self.error("cible d'affectation invalide"),
        };
        self.advance()?;
        let value = self.assignment()?;
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
            self.advance()?;
            let arg = self.unary()?;
            return Ok(Expr::Await(Box::new(arg)));
        }
        let op = match &self.tok.tok {
            Tok::Punct("!") => Some(UnaryOp::Not),
            Tok::Punct("~") => Some(UnaryOp::BitNot),
            Tok::Punct("+") => Some(UnaryOp::Plus),
            Tok::Punct("-") => Some(UnaryOp::Neg),
            Tok::Ident(w) if w == "typeof" => Some(UnaryOp::Typeof),
            Tok::Ident(w) if w == "void" => Some(UnaryOp::Void),
            Tok::Ident(w) if w == "delete" => Some(UnaryOp::Delete),
            _ => None,
        };
        if let Some(op) = op {
            self.advance()?;
            let arg = self.unary()?;
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
        ) {
            return self.error("cible d'incrémentation invalide");
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
                Expr::NewTarget
            } else {
                self.enter()?;
                // Le constructeur est une expression membre sans appel.
                let mut callee = if self.is_kw("new") {
                    self.left_hand_side()?
                } else {
                    self.primary()?
                };
                loop {
                    if self.eat_punct(".")? {
                        callee = Expr::Member {
                            object: Box::new(callee),
                            name: self.property_name()?,
                            optional: false,
                        };
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
                Expr::SuperCall(self.arguments()?)
            } else if self.eat_punct(".")? {
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
                expr = Expr::Member {
                    object: Box::new(expr),
                    name: self.property_name()?,
                    optional: false,
                };
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
                    expr = Expr::Member {
                        object: Box::new(expr),
                        name: self.property_name()?,
                        optional: true,
                    };
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
                let saved_no_in = std::mem::replace(&mut self.no_in, false);
                let args = self.arguments();
                self.no_in = saved_no_in;
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args: args?,
                    optional: false,
                };
            } else if matches!(self.tok.tok, Tok::Template { .. }) {
                return self.unsupported("les gabarits étiquetés");
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
        let mut quasis = Vec::new();
        let mut exprs = Vec::new();
        loop {
            let Tok::Template { cooked, tail } = &self.tok.tok else {
                return self.error("gabarit mal formé");
            };
            quasis.push(Arc::from(cooked.as_str()));
            if *tail {
                self.advance()?;
                return Ok(Expr::Template { quasis, exprs });
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
                    self.advance()?;
                    let is_generator = self.eat_punct("*")?;
                    let name = if self.is_punct("(") {
                        None
                    } else {
                        Some(self.binding_ident()?)
                    };
                    Ok(Expr::Function(Arc::new(self.function_with(
                        name,
                        false,
                        false,
                        is_generator,
                    )?)))
                }
                "async" if self.async_function_follows()? => {
                    self.advance()?;
                    self.advance()?;
                    if self.is_punct("*") {
                        return self.unsupported("les générateurs async");
                    }
                    let name = if self.is_punct("(") {
                        None
                    } else {
                        Some(self.binding_ident()?)
                    };
                    Ok(Expr::Function(Arc::new(
                        self.function_with(name, false, true, false)?,
                    )))
                }
                "class" => {
                    self.advance()?;
                    Ok(Expr::Class(Box::new(self.class_rest(false)?)))
                }
                w if RESERVED.contains(&w) => self.error(format!("« {w} » inattendu")),
                _ => Ok(Expr::Ident(self.binding_ident()?)),
            },
            Tok::Eof => self.error("fin de script inattendue"),
            _ => self.error("expression attendue"),
        }
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
        while !self.eat_punct("}")? {
            if self.eat_punct("...")? {
                let value = self.assignment()?;
                props.push(Property {
                    key: PropKey::Name(Arc::from("")),
                    kind: PropKind::Spread(value),
                });
            } else {
                props.push(self.object_property()?);
            }
            if !self.is_punct("}") {
                self.expect_punct(",")?;
            }
        }
        Ok(props)
    }

    fn object_property(&mut self) -> PResult<Property> {
        let (is_async, is_generator) = self.method_modifiers()?;
        let accessor = match &self.tok.tok {
            Tok::Ident(w) if (w == "get" || w == "set") && self.is_modifier()? => Some(w == "get"),
            _ => None,
        };
        if let Some(is_getter) = accessor {
            self.advance()?;
            let key = self.property_key()?;
            let func = Arc::new(self.function_rest(Self::method_name(&key), true)?);
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
        let key = self.property_key()?;
        if self.is_punct("(") {
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
            return Ok(Property {
                key,
                kind: PropKind::Value(self.assignment()?),
            });
        }
        let Some(name) = shorthand else {
            return self.error("« : » attendu");
        };
        // `{ a }` ou, dans un motif, `{ a = défaut }`.
        let value = if self.eat_punct("=")? {
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
        parse_program(src).unwrap_or_else(|e| panic!("{src}\n→ {e}"))
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
             typeof a === 'undefined'; delete a.b; void 0; -a ** 2 === 0 || !b; a++ + ++b; i in o; a instanceof B;
             [a, b] = [b, a]; ({ a, b: [c] } = o); a ||= b; a.b ??= c; a[b] **= 2;
             (function () {})(); (function named() {}); (class {}); new.target",
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
        for src in [
            "async function* f() {}",
            "class A { async *g() {} }",
            "async function f() { for await (const x of y) {} }",
            "import x from 'y'",
            "export default 1",
            "tag`x`",
        ] {
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
