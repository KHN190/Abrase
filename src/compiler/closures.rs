// Lambda lifting and closure environment packing.
use crate::ast::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct CaptureInfo {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub struct ClosureInfo {
    pub lifted_fn: String,
    pub captures: Vec<CaptureInfo>,
    /// If true, the closure was declared with `move |...|` — captures are
    /// moved out of the surrounding scope. Otherwise captures are by-copy.
    pub is_move: bool,
}

pub struct ClosureLowering {
    pub by_span: HashMap<Span, ClosureInfo>,
    pub synthetic_fns: Vec<FnDecl>,
    next_id: usize,
    // Names that are always considered "in scope" globally (top-level fns,
    // types, effect names, variant constructors). Free-variable analysis
    // treats these as non-captures.
    globals: HashSet<String>,
}

impl ClosureLowering {
    pub fn new() -> Self {
        Self {
            by_span: HashMap::new(),
            synthetic_fns: Vec::new(),
            next_id: 0,
            globals: HashSet::new(),
        }
    }

    pub fn lower(&mut self, ast: &[Decl]) {
        // First pass: collect global names so closure bodies can identify
        // them as non-captures.
        for decl in ast {
            match decl {
                Decl::Fn(fn_decl) => { self.globals.insert(fn_decl.name.clone()); }
                Decl::Type { name, body, .. } => {
                    self.globals.insert(name.clone());
                    if let TypeBody::Variant(cases) = body {
                        for case in cases {
                            let case_name = match case {
                                VariantCase::Unit(n) => n.clone(),
                                VariantCase::Tuple(n, _) => n.clone(),
                                VariantCase::Record(n, _) => n.clone(),
                            };
                            self.globals.insert(case_name);
                        }
                    }
                }
                Decl::Effect { name, .. } => { self.globals.insert(name.clone()); }
                Decl::Const { name, .. } => { self.globals.insert(name.clone()); }
                _ => {}
            }
        }
        self.globals.insert("Shared".into());

        // Second pass: walk fn bodies and lift closures.
        // Clone fn_decls to avoid borrowing while we mutate self.
        let fn_decls: Vec<_> = ast.iter().filter_map(|d| match d {
            Decl::Fn(f) => Some(f.clone()),
            _ => None,
        }).collect();
        for fn_decl in &fn_decls {
            let mut env = ParamEnv::from_fn(fn_decl);
            self.walk_block(&fn_decl.body, &mut env);
        }
    }

    fn walk_block(&mut self, block: &Block, env: &mut ParamEnv) {
        env.push_scope();
        for stmt in &block.stmts {
            self.walk_stmt(stmt, env);
        }
        if let Some(ret) = &block.ret {
            self.walk_expr(ret, env);
        }
        env.pop_scope();
    }

    fn walk_stmt(&mut self, stmt: &Spanned<Stmt>, env: &mut ParamEnv) {
        match &stmt.node {
            Stmt::Let { pattern, value, ty, .. } => {
                self.walk_expr(value, env);
                if let Pattern::Bind(name) = &pattern.node {
                    let bind_ty = ty.clone()
                        .or_else(|| infer_type_from_expr(value))
                        .unwrap_or(Type::Named("Unknown".into()));
                    env.bind(name.clone(), bind_ty);
                }
            }
            Stmt::Expr(e) => self.walk_expr(e, env),
            Stmt::Empty => {}
        }
    }

    fn walk_expr(&mut self, expr: &Spanned<Expr>, env: &mut ParamEnv) {
        match &expr.node {
            Expr::Closure { is_move, params, return_type, body, .. } => {
                // Recurse FIRST so any nested closures get their own lifts;
                // the outer closure captures whatever the inner one captures
                // from the outer scope.
                let mut inner_env = env.shadow_with_params(params);
                self.walk_expr(body, &mut inner_env);
                // Collect free variables of this closure
                let mut frees: Vec<String> = Vec::new();
                let mut seen: HashSet<String> = HashSet::new();
                collect_free_vars(body, &param_names(params), &mut seen, &mut frees);

                let captures: Vec<CaptureInfo> = frees.into_iter()
                    .filter_map(|name| {
                        if self.globals.contains(&name) { return None; }
                        env.lookup(&name).map(|ty| CaptureInfo { name, ty })
                    })
                    .collect();

                let lifted_name = format!("__closure_{}", self.next_id);
                self.next_id += 1;

                // Build the lifted fn:
                //   first param: env_ptr (Int handle).
                //   rest:        the closure's own params.
                let mut lifted_params: Vec<Param> = Vec::new();
                lifted_params.push(Param::Named {
                    pattern: Spanned { node: Pattern::Bind("__env".into()), span: expr.span },
                    ty: Type::Named("Int".into()),
                });
                for cp in params {
                    let pty = cp.ty.clone().unwrap_or(Type::Named("Unknown".into()));
                    lifted_params.push(Param::Named {
                        pattern: cp.pattern.clone(),
                        ty: pty,
                    });
                }

                // Rewrite captured-name references in the body to env loads.
                let layout: HashMap<String, usize> = captures.iter()
                    .enumerate()
                    .map(|(i, c)| (c.name.clone(), i))
                    .collect();
                let rewritten_body = rewrite_captures(body, &layout, &param_names(params));

                let lifted = FnDecl {
                    attrs: vec![],
                    is_pub: false,
                    name: lifted_name.clone(),
                    generics: vec![],
                    params: lifted_params,
                    effects: vec![],
                    return_type: return_type.clone(),
                    where_clause: vec![],
                    body: Block { stmts: vec![], ret: Some(Box::new(rewritten_body)) },
                };
                self.synthetic_fns.push(lifted);
                self.by_span.insert(expr.span, ClosureInfo {
                    lifted_fn: lifted_name,
                    captures,
                    is_move: *is_move,
                });
            }
            Expr::Binary { left, right, .. } => {
                self.walk_expr(left, env);
                self.walk_expr(right, env);
            }
            Expr::Unary { right, .. } => self.walk_expr(right, env),
            Expr::Call { callee, args } => {
                self.walk_expr(callee, env);
                for a in args { self.walk_expr(a, env); }
            }
            Expr::If { condition, consequence, alternative } => {
                self.walk_expr(condition, env);
                self.walk_expr(consequence, env);
                if let Some(a) = alternative { self.walk_expr(a, env); }
            }
            Expr::Match { scrutinee, arms } => {
                self.walk_expr(scrutinee, env);
                for arm in arms { self.walk_expr(&arm.body, env); }
            }
            Expr::Block(b) => self.walk_block(b, env),
            Expr::While { condition, body } => {
                self.walk_expr(condition, env);
                self.walk_block(body, env);
            }
            Expr::For { iter, body, pattern } => {
                self.walk_expr(iter, env);
                env.push_scope();
                if let Pattern::Bind(n) = &pattern.node {
                    env.bind(n.clone(), Type::Named("Unknown".into()));
                }
                self.walk_block(body, env);
                env.pop_scope();
            }
            Expr::Loop { body } => self.walk_block(body, env),
            Expr::Region { body, .. } => self.walk_block(body, env),
            Expr::Handle { expr: e, arms } => {
                self.walk_expr(e, env);
                for arm in arms { self.walk_expr(&arm.body, env); }
            }
            Expr::Return(Some(e)) | Expr::Throw(e) | Expr::Question(e) => self.walk_expr(e, env),
            Expr::Resume(Some(e)) => self.walk_expr(e, env),
            Expr::Index { base, index } => {
                self.walk_expr(base, env);
                self.walk_expr(index, env);
            }
            Expr::FieldAccess { base, .. } => self.walk_expr(base, env),
            Expr::Tuple(elems) | Expr::Array(elems) => {
                for e in elems { self.walk_expr(e, env); }
            }
            Expr::ArrayRepeat { elem, count } => {
                self.walk_expr(elem, env);
                self.walk_expr(count, env);
            }
            _ => {}
        }
    }
}

// Bookkeeping for tracking which names are in scope at a given point.
struct ParamEnv {
    scopes: Vec<HashMap<String, Type>>,
}

impl ParamEnv {
    fn from_fn(fn_decl: &FnDecl) -> Self {
        let mut s = HashMap::new();
        for p in &fn_decl.params {
            if let Param::Named { pattern, ty } = p {
                if let Pattern::Bind(n) = &pattern.node {
                    s.insert(n.clone(), ty.clone());
                }
            }
        }
        Self { scopes: vec![s] }
    }
    fn shadow_with_params(&self, params: &[ClosureParam]) -> Self {
        // Closure body sees: outer scopes + its own params on top.
        let mut scopes = self.scopes.clone();
        let mut inner = HashMap::new();
        for cp in params {
            if let Pattern::Bind(n) = &cp.pattern.node {
                inner.insert(n.clone(), cp.ty.clone().unwrap_or(Type::Named("Unknown".into())));
            }
        }
        scopes.push(inner);
        Self { scopes }
    }
    fn push_scope(&mut self) { self.scopes.push(HashMap::new()); }
    fn pop_scope(&mut self) { self.scopes.pop(); }
    fn bind(&mut self, name: String, ty: Type) {
        if let Some(top) = self.scopes.last_mut() { top.insert(name, ty); }
    }
    fn lookup(&self, name: &str) -> Option<Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(t) = scope.get(name) { return Some(t.clone()); }
        }
        None
    }
}

fn param_names(params: &[ClosureParam]) -> HashSet<String> {
    params.iter().filter_map(|p| match &p.pattern.node {
        Pattern::Bind(n) => Some(n.clone()),
        _ => None,
    }).collect()
}

// Walk an expr collecting identifiers that are NOT bound by `bound`. Visit
// order is left-to-right so the resulting Vec is deterministic; `seen`
// dedups across the walk.
pub(in crate::compiler) fn collect_free_vars(
    expr: &Spanned<Expr>,
    bound: &HashSet<String>,
    seen: &mut HashSet<String>,
    out: &mut Vec<String>,
) {
    match &expr.node {
        Expr::Identifier(n) => {
            if !bound.contains(n) && !seen.contains(n) {
                seen.insert(n.clone());
                out.push(n.clone());
            }
        }
        Expr::Binary { left, right, .. } => {
            collect_free_vars(left, bound, seen, out);
            collect_free_vars(right, bound, seen, out);
        }
        Expr::Unary { right, .. } => collect_free_vars(right, bound, seen, out),
        Expr::Call { callee, args } => {
            collect_free_vars(callee, bound, seen, out);
            for a in args { collect_free_vars(a, bound, seen, out); }
        }
        Expr::If { condition, consequence, alternative } => {
            collect_free_vars(condition, bound, seen, out);
            collect_free_vars(consequence, bound, seen, out);
            if let Some(a) = alternative { collect_free_vars(a, bound, seen, out); }
        }
        Expr::Match { scrutinee, arms } => {
            collect_free_vars(scrutinee, bound, seen, out);
            for arm in arms { collect_free_vars(&arm.body, bound, seen, out); }
        }
        Expr::Block(b) => {
            // Local lets add to bound within the block; track on a clone.
            let mut local_bound = bound.clone();
            for stmt in &b.stmts {
                match &stmt.node {
                    Stmt::Let { pattern, value, .. } => {
                        collect_free_vars(value, &local_bound, seen, out);
                        if let Pattern::Bind(n) = &pattern.node {
                            local_bound.insert(n.clone());
                        }
                    }
                    Stmt::Expr(e) => collect_free_vars(e, &local_bound, seen, out),
                    Stmt::Empty => {}
                }
            }
            if let Some(r) = &b.ret {
                collect_free_vars(r, &local_bound, seen, out);
            }
        }
        Expr::While { condition, body } => {
            collect_free_vars(condition, bound, seen, out);
            collect_free_vars(&Spanned { node: Expr::Block(body.clone()), span: expr.span }, bound, seen, out);
        }
        Expr::For { iter, body, pattern } => {
            collect_free_vars(iter, bound, seen, out);
            let mut local_bound = bound.clone();
            if let Pattern::Bind(n) = &pattern.node { local_bound.insert(n.clone()); }
            collect_free_vars(&Spanned { node: Expr::Block(body.clone()), span: expr.span }, &local_bound, seen, out);
        }
        Expr::Loop { body } => collect_free_vars(&Spanned { node: Expr::Block(body.clone()), span: expr.span }, bound, seen, out),
        Expr::Region { body, .. } => collect_free_vars(&Spanned { node: Expr::Block(body.clone()), span: expr.span }, bound, seen, out),
        Expr::Handle { expr: e, arms } => {
            collect_free_vars(e, bound, seen, out);
            for arm in arms { collect_free_vars(&arm.body, bound, seen, out); }
        }
        Expr::Return(Some(e)) | Expr::Throw(e) | Expr::Question(e) => {
            collect_free_vars(e, bound, seen, out);
        }
        Expr::Resume(Some(e)) => collect_free_vars(e, bound, seen, out),
        Expr::Index { base, index } => {
            collect_free_vars(base, bound, seen, out);
            collect_free_vars(index, bound, seen, out);
        }
        Expr::FieldAccess { base, .. } => collect_free_vars(base, bound, seen, out),
        Expr::Tuple(elems) | Expr::Array(elems) => {
            for e in elems { collect_free_vars(e, bound, seen, out); }
        }
        Expr::ArrayRepeat { elem, count } => {
            collect_free_vars(elem, bound, seen, out);
            collect_free_vars(count, bound, seen, out);
        }
        // Nested closures contribute their OWN free vars (those that aren't bound
        // by THEIR params or our `bound`).
        Expr::Closure { params, body, .. } => {
            let mut inner_bound = bound.clone();
            for cp in params {
                if let Pattern::Bind(n) = &cp.pattern.node { inner_bound.insert(n.clone()); }
            }
            collect_free_vars(body, &inner_bound, seen, out);
        }
        _ => {}
    }
}

// Rewrite occurrences of captured names in `expr` to a load from the env
// handle. Treats `params` as locals that shadow captures.
pub(in crate::compiler) fn rewrite_captures(
    expr: &Spanned<Expr>,
    layout: &HashMap<String, usize>,
    params: &HashSet<String>,
) -> Spanned<Expr> {
    Spanned { node: rewrite_node(&expr.node, layout, params, expr.span), span: expr.span }
}

fn rewrite_node(
    node: &Expr,
    layout: &HashMap<String, usize>,
    params: &HashSet<String>,
    span: Span,
) -> Expr {
    match node {
        Expr::Identifier(name) => {
            if params.contains(name) {
                return Expr::Identifier(name.clone());
            }
            // Captured idx (or pass through if not captured).
            let Some(&idx) = layout.get(name) else {
                return Expr::Identifier(name.clone());
            };
            Expr::Call {
                callee: Box::new(Spanned {
                    node: Expr::Identifier("__env_load".into()),
                    span,
                }),
                args: vec![
                    Spanned { node: Expr::Identifier("__env".into()), span },
                    Spanned { node: Expr::Literal(Literal::Int(idx as i64)), span },
                ],
            }
        }
        Expr::Binary { op, left, right } => Expr::Binary {
            op: op.clone(),
            left: Box::new(rewrite_captures(left, layout, params)),
            right: Box::new(rewrite_captures(right, layout, params)),
        },
        Expr::Unary { op, right } => Expr::Unary {
            op: op.clone(),
            right: Box::new(rewrite_captures(right, layout, params)),
        },
        Expr::Call { callee, args } => Expr::Call {
            callee: Box::new(rewrite_captures(callee, layout, params)),
            args: args.iter().map(|a| rewrite_captures(a, layout, params)).collect(),
        },
        Expr::If { condition, consequence, alternative } => Expr::If {
            condition: Box::new(rewrite_captures(condition, layout, params)),
            consequence: Box::new(rewrite_captures(consequence, layout, params)),
            alternative: alternative.as_ref().map(|a| Box::new(rewrite_captures(a, layout, params))),
        },
        Expr::Block(b) => Expr::Block(rewrite_block(b, layout, params)),
        Expr::Return(opt) => Expr::Return(opt.as_ref().map(|e| Box::new(rewrite_captures(e, layout, params)))),
        Expr::Throw(e) => Expr::Throw(Box::new(rewrite_captures(e, layout, params))),
        Expr::Question(e) => Expr::Question(Box::new(rewrite_captures(e, layout, params))),
        Expr::Index { base, index } => Expr::Index {
            base: Box::new(rewrite_captures(base, layout, params)),
            index: Box::new(rewrite_captures(index, layout, params)),
        },
        Expr::FieldAccess { base, field } => Expr::FieldAccess {
            base: Box::new(rewrite_captures(base, layout, params)),
            field: field.clone(),
        },
        Expr::Tuple(elems) => Expr::Tuple(elems.iter().map(|e| rewrite_captures(e, layout, params)).collect()),
        Expr::Array(elems) => Expr::Array(elems.iter().map(|e| rewrite_captures(e, layout, params)).collect()),
        Expr::ArrayRepeat { elem, count } => Expr::ArrayRepeat {
            elem: Box::new(rewrite_captures(elem, layout, params)),
            count: Box::new(rewrite_captures(count, layout, params)),
        },
        Expr::Match { scrutinee, arms } => Expr::Match {
            scrutinee: Box::new(rewrite_captures(scrutinee, layout, params)),
            arms: arms.iter().map(|a| MatchArm {
                pattern: a.pattern.clone(),
                guard: a.guard.as_ref().map(|g| rewrite_captures(g, layout, params)),
                body: rewrite_captures(&a.body, layout, params),
            }).collect(),
        },
        Expr::While { condition, body } => Expr::While {
            condition: Box::new(rewrite_captures(condition, layout, params)),
            body: rewrite_block(body, layout, params),
        },
        Expr::For { pattern, iter, body } => Expr::For {
            pattern: pattern.clone(),
            iter: Box::new(rewrite_captures(iter, layout, params)),
            body: rewrite_block(body, layout, params),
        },
        Expr::Loop { body } => Expr::Loop {
            body: rewrite_block(body, layout, params),
        },
        Expr::Region { label, body } => Expr::Region {
            label: label.clone(),
            body: rewrite_block(body, layout, params),
        },
        Expr::Handle { expr, arms } => Expr::Handle {
            expr: Box::new(rewrite_captures(expr, layout, params)),
            arms: arms.iter().map(|a| HandleArm {
                kind: a.kind.clone(),
                pattern: a.pattern.clone(),
                body: rewrite_captures(&a.body, layout, params),
            }).collect(),
        },
        Expr::Resume(opt) => Expr::Resume(
            opt.as_ref().map(|e| Box::new(rewrite_captures(e, layout, params)))
        ),
        Expr::Break(opt) => Expr::Break(
            opt.as_ref().map(|e| Box::new(rewrite_captures(e, layout, params)))
        ),
        _ => node.clone(),
    }
}

fn rewrite_block(
    block: &Block,
    layout: &HashMap<String, usize>,
    params: &HashSet<String>,
) -> Block {
    let stmts = block.stmts.iter().map(|s| {
        let node = match &s.node {
            Stmt::Let { pattern, is_mut, ty, value } => Stmt::Let {
                pattern: pattern.clone(),
                is_mut: *is_mut,
                ty: ty.clone(),
                value: rewrite_captures(value, layout, params),
            },
            Stmt::Expr(e) => Stmt::Expr(rewrite_captures(e, layout, params)),
            Stmt::Empty => Stmt::Empty,
        };
        Spanned { node, span: s.span }
    }).collect();
    let ret = block.ret.as_ref().map(|r| Box::new(rewrite_captures(r, layout, params)));
    Block { stmts, ret }
}

fn infer_type_from_expr(expr: &Spanned<Expr>) -> Option<Type> {
    match &expr.node {
        Expr::Literal(Literal::Int(_)) => Some(Type::Named("Int".into())),
        Expr::Literal(Literal::Float(_)) => Some(Type::Named("Float".into())),
        Expr::Literal(Literal::Bool(_)) => Some(Type::Named("Bool".into())),
        Expr::Literal(Literal::String(_)) => Some(Type::Named("String".into())),
        Expr::Literal(Literal::Unit) => Some(Type::Named("Unit".into())),
        _ => None,
    }
}
