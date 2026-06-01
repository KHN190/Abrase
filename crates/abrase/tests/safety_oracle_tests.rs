use abrase::ast::*;
use abrase::lexer::Lexer;
use abrase::parser::Parser;
use abrase::safety::{CapMode, Facts, Kind};
use std::collections::{HashMap, HashSet};

const STATIC_REGION: u32 = u32::MAX;

struct Program {
    fns: Vec<Block>,
    static_mut: HashSet<String>,
}

fn parse_program(src: &str) -> Program {
    let mut p = Parser::new(Lexer::new(src)).with_source(src.to_string());
    let decls = p.parse_program();
    let mut fns = Vec::new();
    let mut static_mut = HashSet::new();
    for d in decls {
        match d {
            Decl::Fn(f) => fns.push(f.body),
            Decl::Impl { methods, .. } => fns.extend(methods.into_iter().map(|m| m.body)),
            Decl::Static { is_mut: true, name, .. } => {
                static_mut.insert(name);
            }
            _ => {}
        }
    }
    Program { fns, static_mut }
}

struct Extract<'a> {
    facts: Facts,
    static_mut: &'a HashSet<String>,
    scopes: Vec<HashMap<String, u32>>,
    region_stack: Vec<u32>,
    next_var: u32,
    next_region: u32,
    next_closure: u32,
}

impl<'a> Extract<'a> {
    fn new(static_mut: &'a HashSet<String>) -> Self {
        Extract {
            facts: Facts::new(),
            static_mut,
            scopes: vec![HashMap::new()],
            region_stack: Vec::new(),
            next_var: 0,
            next_region: 0,
            next_closure: 0,
        }
    }

    fn fresh_var(&mut self) -> u32 {
        let v = self.next_var;
        self.next_var += 1;
        v
    }

    fn resolve(&self, name: &str) -> Option<u32> {
        self.scopes.iter().rev().find_map(|s| s.get(name).copied())
    }

    fn bind(&mut self, name: String, v: u32) {
        self.scopes.last_mut().unwrap().insert(name, v);
    }

    fn declare(&mut self, name: Option<String>, kind: Kind, is_handle: bool, span: Span) -> u32 {
        let v = self.fresh_var();
        self.facts.binding.insert(v, kind);
        self.facts.span.insert(v, span);
        if is_handle {
            self.facts.is_handle.insert(v);
        }
        if let Some(&r) = self.region_stack.last() {
            self.facts.in_region.insert(v, r);
        }
        if let Some(n) = name {
            self.bind(n, v);
        }
        v
    }

    fn walk_block(&mut self, b: &Block) {
        self.scopes.push(HashMap::new());
        for s in &b.stmts {
            self.walk_stmt(&s.node);
        }
        if let Some(ret) = &b.ret {
            self.walk_expr(&ret.node);
        }
        self.scopes.pop();
    }

    fn walk_stmt(&mut self, s: &Stmt) {
        if let Stmt::Let { pattern, ty, value, .. } = s {
            self.walk_expr(&value.node);
            self.consume_operand(&value.node, value.span);
            let name = match &pattern.node {
                Pattern::Bind(n) => Some(n.clone()),
                _ => None,
            };
            let kind = kind_of(ty, &value.node);
            let handle = is_handle(ty, &value.node);
            let v = self.declare(name, kind, handle, pattern.span);
            if let Expr::Closure { is_move, body, .. } = &value.node {
                let cl = self.register_closure(*is_move, &body.node);
                self.facts.closure_var.insert(cl, v);
            }
        } else if let Stmt::Expr(e) = s {
            self.walk_expr(&e.node);
        }
    }

    fn register_closure(&mut self, is_move: bool, body: &Expr) -> u32 {
        let cl = self.next_closure;
        self.next_closure += 1;
        let mode = if is_move { CapMode::Move } else { CapMode::Copy };
        let mut names = Vec::new();
        collect_idents(body, &mut names);
        for n in names {
            if let Some(v) = self.resolve(&n) {
                self.facts.capture.push((cl, v, mode));
            }
        }
        cl
    }

    fn consume_operand(&mut self, e: &Expr, span: Span) {
        if let Expr::Identifier(n) = e {
            if let Some(v) = self.resolve(n) {
                if matches!(self.facts.binding.get(&v), Some(Kind::Move))
                    && self.facts.is_handle.contains(&v)
                {
                    self.facts.consumed.push((v, span));
                }
            }
        }
    }

    fn walk_expr(&mut self, e: &Expr) {
        match e {
            Expr::Region { body, .. } => {
                let r = self.next_region;
                self.next_region += 1;
                self.region_stack.push(r);
                self.scopes.push(HashMap::new());
                for s in &body.stmts {
                    self.walk_stmt(&s.node);
                }
                if let Some(ret) = &body.ret {
                    self.escape_tail(&ret.node, ret.span, r);
                }
                self.scopes.pop();
                self.region_stack.pop();
            }
            Expr::Closure { is_move, body, .. } => {
                self.register_closure(*is_move, &body.node);
            }
            Expr::Block(b) => self.walk_block(b),
            Expr::If { condition, consequence, alternative } => {
                self.walk_expr(&condition.node);
                self.walk_expr(&consequence.node);
                if let Some(a) = alternative {
                    self.walk_expr(&a.node);
                }
            }
            Expr::Match { scrutinee, arms } => {
                self.walk_expr(&scrutinee.node);
                for a in arms {
                    self.walk_expr(&a.body.node);
                }
            }
            Expr::Binary { op: BinaryOp::Assign, left, right } => {
                self.walk_expr(&right.node);
                let target = match &left.node {
                    Expr::Identifier(n) => Some(n.clone()),
                    Expr::FieldAccess { base, .. } => match &base.node {
                        Expr::Identifier(n) => Some(n.clone()),
                        _ => None,
                    },
                    _ => None,
                };
                if let Some(n) = target {
                    if self.static_mut.contains(&n) {
                        self.escape_tail(&right.node, right.span, STATIC_REGION);
                    }
                } else {
                    self.walk_expr(&left.node);
                }
                self.consume_operand(&right.node, right.span);
            }
            Expr::Binary { left, right, .. } => {
                self.walk_expr(&left.node);
                self.walk_expr(&right.node);
            }
            Expr::Unary { right, .. } => self.walk_expr(&right.node),
            Expr::Call { callee, args } => {
                self.walk_expr(&callee.node);
                for a in args {
                    self.walk_expr(&a.node);
                    self.consume_operand(&a.node, a.span);
                }
            }
            Expr::Paren(e) | Expr::Question(e) => self.walk_expr(&e.node),
            _ => {}
        }
    }

    fn escape_tail(&mut self, e: &Expr, span: Span, r: u32) {
        match e {
            Expr::Identifier(n) => {
                if let Some(v) = self.resolve(n) {
                    self.facts.flows_outside.insert(v, r);
                }
            }
            Expr::Unary { op: UnaryOp::Ref, .. } => {
                let v = self.declare(None, Kind::Ref, false, span);
                self.facts.flows_outside.insert(v, r);
            }
            Expr::Unary { op: UnaryOp::RefMut, .. } => {
                let v = self.declare(None, Kind::RefMut, false, span);
                self.facts.flows_outside.insert(v, r);
            }
            Expr::Closure { is_move, body, .. } => {
                let v = self.declare(None, Kind::Move, true, span);
                let cl = self.register_closure(*is_move, &body.node);
                self.facts.closure_var.insert(cl, v);
                self.facts.flows_outside.insert(v, r);
            }
            Expr::Paren(inner) => self.escape_tail(&inner.node, span, r),
            _ => self.walk_expr(e),
        }
    }
}

fn collect_idents(e: &Expr, out: &mut Vec<String>) {
    match e {
        Expr::Identifier(n) => out.push(n.clone()),
        Expr::Binary { left, right, .. } => {
            collect_idents(&left.node, out);
            collect_idents(&right.node, out);
        }
        Expr::Unary { right, .. } => collect_idents(&right.node, out),
        Expr::FieldAccess { base, .. } => collect_idents(&base.node, out),
        Expr::Call { callee, args } => {
            collect_idents(&callee.node, out);
            for a in args {
                collect_idents(&a.node, out);
            }
        }
        Expr::Index { base, index } => {
            collect_idents(&base.node, out);
            collect_idents(&index.node, out);
        }
        Expr::Paren(e) | Expr::Question(e) => collect_idents(&e.node, out),
        Expr::If { condition, consequence, alternative } => {
            collect_idents(&condition.node, out);
            collect_idents(&consequence.node, out);
            if let Some(a) = alternative {
                collect_idents(&a.node, out);
            }
        }
        _ => {}
    }
}

fn kind_of(ty: &Option<Type>, value: &Expr) -> Kind {
    if let Some(Type::Reference { is_mut, .. }) = ty {
        return if *is_mut { Kind::RefMut } else { Kind::Ref };
    }
    if let Some(Type::Generic { name, .. }) = ty {
        if name == "Shared" {
            return Kind::Shared;
        }
    }
    match value {
        Expr::Closure { .. } => Kind::Move,
        Expr::Literal(Literal::Int(_) | Literal::Float(_) | Literal::Bool(_) | Literal::Char(_) | Literal::Unit) => Kind::Copy,
        Expr::Record { .. } | Expr::Variant { .. } | Expr::Array(_) | Expr::ArrayRepeat { .. } => Kind::Move,
        _ => Kind::Copy,
    }
}

fn is_handle(ty: &Option<Type>, value: &Expr) -> bool {
    if let Some(Type::Reference { .. }) = ty {
        return false;
    }
    matches!(
        value,
        Expr::Closure { .. }
            | Expr::Record { .. }
            | Expr::Variant { .. }
            | Expr::Array(_)
            | Expr::ArrayRepeat { .. }
    )
}

fn analyze_src(src: &str) -> (Vec<String>, usize) {
    let prog = parse_program(src);
    let mut codes = Vec::new();
    let mut forgets = 0;
    for body in &prog.fns {
        let mut ex = Extract::new(&prog.static_mut);
        ex.walk_block(body);
        let (errs, must_forget) = ex.facts.analyze();
        codes.extend(errs.into_iter().map(|e| e.code.to_string()));
        forgets += must_forget.len();
    }
    (codes, forgets)
}

#[test]
fn oracle_copy_value_escaping_region_is_clean() {
    let (errs, mf) = analyze_src("fn main() -> Int { region { let a = 5; a } }");
    assert!(errs.is_empty(), "{errs:?}");
    assert_eq!(mf, 0);
}

#[test]
fn oracle_ref_escaping_region_flagged() {
    let (errs, _) = analyze_src("fn main() -> Int { region { let a = 5; &a } }");
    assert!(errs.iter().any(|c| c == "escape-ref"), "{errs:?}");
}

#[test]
fn oracle_handle_escaping_region_requires_forget() {
    let (errs, mf) = analyze_src(
        "fn main() -> P { region { let p = P { v: 1 }; p } }",
    );
    assert!(errs.is_empty(), "{errs:?}");
    assert_eq!(mf, 1);
}

#[test]
fn oracle_escaping_closure_carries_moved_region_handle() {
    let (errs, mf) = analyze_src(
        "fn main() -> F { region { let p = P { v: 1 }; move |x| x + p } }",
    );
    assert!(errs.is_empty(), "{errs:?}");
    assert_eq!(mf, 2, "closure + captured handle both need forget");
}

#[test]
fn oracle_region_handle_stored_in_static_mut_requires_forget() {
    let (errs, mf) = analyze_src(
        "static mut g: P = P { v: 0 }\nfn main() { region { let p = P { v: 1 }; g = p } }",
    );
    assert!(errs.is_empty(), "{errs:?}");
    assert_eq!(mf, 1, "handle stored into static escapes region");
}

#[test]
fn oracle_region_ref_stored_in_static_mut_flagged() {
    let (errs, _) = analyze_src(
        "static mut g: Int = 0\nfn main() { region { let a = 5; g = &a } }",
    );
    assert!(errs.iter().any(|c| c == "escape-ref"), "{errs:?}");
}

#[test]
fn oracle_multi_module_each_fn_analyzed() {
    let src = "mod a {\nfn f() -> P { region { let p = P { v: 1 }; p } }\n}\nmod b {\nfn g() -> Int { region { let a = 5; &a } }\n}";
    let (errs, mf) = analyze_src(src);
    assert!(errs.iter().any(|c| c == "escape-ref"), "{errs:?}");
    assert_eq!(mf, 1, "module a's handle escape still tracked");
}

#[test]
fn oracle_move_handle_used_twice_is_double_move() {
    let (errs, _) = analyze_src(
        "fn main() { let p = P { v: 1 }; sink(p); sink(p) }",
    );
    assert!(errs.iter().any(|c| c == "double-move"), "{errs:?}");
}

#[test]
fn oracle_move_handle_used_once_is_clean() {
    let (errs, _) = analyze_src("fn main() { let p = P { v: 1 }; sink(p) }");
    assert!(!errs.iter().any(|c| c == "double-move"), "{errs:?}");
}
