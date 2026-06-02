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
    pub is_move: bool,
    pub self_name: Option<String>,
}

pub struct ClosureLowering {
    pub by_span: HashMap<Span, ClosureInfo>,
    pub synthetic_fns: Vec<FnDecl>,
    pub fn_value_adapters: HashMap<String, String>,
    next_id: usize,
    globals: HashSet<String>,
    user_fns: HashMap<String, FnDecl>,
    pending_self_name: Option<String>,
}

impl ClosureLowering {
    pub fn new() -> Self {
        Self {
            by_span: HashMap::new(),
            synthetic_fns: Vec::new(),
            fn_value_adapters: HashMap::new(),
            next_id: 0,
            globals: HashSet::new(),
            user_fns: HashMap::new(),
            pending_self_name: None,
        }
    }

    pub fn lower(&mut self, ast: &[Decl]) {
        // First pass: collect global names so closure bodies can identify
        // them as non-captures.
        for decl in ast {
            match decl {
                Decl::Fn(fn_decl) => {
                    self.globals.insert(fn_decl.name.clone());
                    self.user_fns.insert(fn_decl.name.clone(), fn_decl.clone());
                }
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

        // Synthesize adapters for top-level fns referenced as values.
        let mut refs: HashSet<String> = HashSet::new();
        for fn_decl in &fn_decls {
            collect_fn_value_refs_block(&fn_decl.body, &self.user_fns, &mut refs);
        }
        for syn in self.synthetic_fns.clone() {
            collect_fn_value_refs_block(&syn.body, &self.user_fns, &mut refs);
        }
        let mut names: Vec<String> = refs.into_iter().collect();
        names.sort();
        for name in names {
            self.synth_fn_value_adapter(&name);
        }
    }

    fn synth_fn_value_adapter(&mut self, name: &str) {
        let Some(decl) = self.user_fns.get(name).cloned() else { return };
        let mut params: Vec<Param> = vec![Param::Named {
            pattern: Spanned { node: Pattern::Bind("__env".into()), span: Span::new(0, 0) },
            ty: Type::Named("Int".into()),
        }];
        let mut call_args: Vec<Spanned<Expr>> = Vec::new();
        for p in &decl.params {
            let Param::Named { pattern, ty } = p else { return };
            let Pattern::Bind(pn) = &pattern.node else { return };
            params.push(Param::Named { pattern: pattern.clone(), ty: ty.clone() });
            call_args.push(Spanned { node: Expr::Identifier(pn.clone()), span: Span::new(0, 0) });
        }
        let adapter = format!("__fnval_{}", name);
        let body_call = Expr::Call {
            callee: Box::new(Spanned { node: Expr::Identifier(name.to_string()), span: Span::new(0, 0) }),
            args: call_args,
        };
        self.synthetic_fns.push(FnDecl {
            attrs: vec![], is_pub: false, name: adapter.clone(), generics: vec![],
            params, effects: vec![], return_type: decl.return_type.clone(),
            where_clause: vec![],
            body: Block { stmts: vec![], ret: Some(Box::new(Spanned { node: body_call, span: Span::new(0, 0) })) },
        });
        self.fn_value_adapters.insert(name.to_string(), adapter);
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
                let prev = self.pending_self_name.take();
                if let (Pattern::Bind(name), Expr::Closure { .. }) = (&pattern.node, &value.node) {
                    self.pending_self_name = Some(name.clone());
                }
                self.walk_expr(value, env);
                self.pending_self_name = prev;
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
                // Consume self_name BEFORE recursing so nested closures don't
                // inherit our binding name.
                let self_name = self.pending_self_name.take();
                // Recurse first so nested closures get their own lifts before capturing from outer scope.

                let mut inner_env = env.shadow_with_params(params);
                self.walk_expr(body, &mut inner_env);
                // Collect free variables of this closure
                let mut frees: Vec<String> = Vec::new();
                let mut seen: HashSet<String> = HashSet::new();
                collect_free_vars(body, &param_names(params), &mut seen, &mut frees);

                let captures: Vec<CaptureInfo> = frees.into_iter()
                    .filter_map(|name| {
                        if self.globals.contains(&name) { return None; }
                        if self_name.as_deref() == Some(name.as_str()) { return None; }
                        env.lookup(&name).map(|ty| CaptureInfo { name, ty })
                    })
                    .collect();

                let lifted_name = format!("__closure_{}", self.next_id);
                self.next_id += 1;

                // Build lifted fn: first param is env_ptr (Int), rest are closure params.
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
                let rewritten_body = rewrite_captures(
                    body, &layout, &param_names(params),
                    self_name.as_deref(), &lifted_name,
                );

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
                    self_name,
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
            Expr::Return(Some(e)) | Expr::Throw(e) | Expr::Question(e) | Expr::Paren(e) => self.walk_expr(e, env),
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
            Expr::Record { fields, .. } => {
                for f in fields {
                    if let Some(v) = &f.value { self.walk_expr(v, env); }
                }
            }
            Expr::Variant { args, .. } => {
                for a in args { self.walk_expr(a, env); }
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

// Collect unbound identifiers left-to-right (deterministic). `seen` deduplicates.
pub fn collect_free_vars(
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
        Expr::Return(Some(e)) | Expr::Throw(e) | Expr::Question(e) | Expr::Paren(e) => {
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
        Expr::Record { fields, .. } => {
            for f in fields {
                if let Some(v) = &f.value { collect_free_vars(v, bound, seen, out); }
            }
        }
        Expr::Variant { args, .. } => {
            for a in args { collect_free_vars(a, bound, seen, out); }
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

// Collect names of top-level fns used as a VALUE (anywhere except as the direct
// callee of a call). Those need a [fn_id, env] adapter to be callable uniformly.
pub fn collect_fn_value_refs(
    expr: &Spanned<Expr>,
    fns: &HashMap<String, FnDecl>,
    out: &mut HashSet<String>,
) {
    match &expr.node {
        Expr::Identifier(n) => { if fns.contains_key(n) { out.insert(n.clone()); } }
        Expr::Call { callee, args } => {
            // Skip a bare-identifier callee (direct call, not a value use).
            if !matches!(&callee.node, Expr::Identifier(_)) {
                collect_fn_value_refs(callee, fns, out);
            }
            for a in args { collect_fn_value_refs(a, fns, out); }
        }
        Expr::Binary { left, right, .. } => {
            collect_fn_value_refs(left, fns, out);
            collect_fn_value_refs(right, fns, out);
        }
        Expr::Unary { right, .. } => collect_fn_value_refs(right, fns, out),
        Expr::If { condition, consequence, alternative } => {
            collect_fn_value_refs(condition, fns, out);
            collect_fn_value_refs(consequence, fns, out);
            if let Some(a) = alternative { collect_fn_value_refs(a, fns, out); }
        }
        Expr::Match { scrutinee, arms } => {
            collect_fn_value_refs(scrutinee, fns, out);
            for arm in arms {
                if let Some(g) = &arm.guard { collect_fn_value_refs(g, fns, out); }
                collect_fn_value_refs(&arm.body, fns, out);
            }
        }
        Expr::Block(b) => collect_fn_value_refs_block(b, fns, out),
        Expr::While { condition, body } => {
            collect_fn_value_refs(condition, fns, out);
            collect_fn_value_refs_block(body, fns, out);
        }
        Expr::For { iter, body, .. } => {
            collect_fn_value_refs(iter, fns, out);
            collect_fn_value_refs_block(body, fns, out);
        }
        Expr::Loop { body } | Expr::Region { body, .. } => collect_fn_value_refs_block(body, fns, out),
        Expr::Handle { expr: e, arms } => {
            collect_fn_value_refs(e, fns, out);
            for arm in arms { collect_fn_value_refs(&arm.body, fns, out); }
        }
        Expr::Return(Some(e)) | Expr::Throw(e) | Expr::Question(e) | Expr::Paren(e)
            => collect_fn_value_refs(e, fns, out),
        Expr::Resume(Some(e)) | Expr::Break(Some(e)) => collect_fn_value_refs(e, fns, out),
        Expr::Index { base, index } => {
            collect_fn_value_refs(base, fns, out);
            collect_fn_value_refs(index, fns, out);
        }
        Expr::FieldAccess { base, .. } => collect_fn_value_refs(base, fns, out),
        Expr::Tuple(es) | Expr::Array(es) => { for e in es { collect_fn_value_refs(e, fns, out); } }
        Expr::ArrayRepeat { elem, count } => {
            collect_fn_value_refs(elem, fns, out);
            collect_fn_value_refs(count, fns, out);
        }
        Expr::Record { fields, .. } => {
            for f in fields { if let Some(v) = &f.value { collect_fn_value_refs(v, fns, out); } }
        }
        Expr::Variant { args, .. } => { for a in args { collect_fn_value_refs(a, fns, out); } }
        Expr::Closure { body, .. } => collect_fn_value_refs(body, fns, out),
        _ => {}
    }
}

fn collect_fn_value_refs_block(
    block: &Block,
    fns: &HashMap<String, FnDecl>,
    out: &mut HashSet<String>,
) {
    for stmt in &block.stmts {
        match &stmt.node {
            Stmt::Let { value, .. } => collect_fn_value_refs(value, fns, out),
            Stmt::Expr(e) => collect_fn_value_refs(e, fns, out),
            Stmt::Empty => {}
        }
    }
    if let Some(r) = &block.ret { collect_fn_value_refs(r, fns, out); }
}

pub fn collect_assigned_idents(
    expr: &Spanned<Expr>,
    candidates: &HashSet<String>,
    out: &mut HashSet<String>,
) {
    match &expr.node {
        Expr::Binary { op, left, right } => {
            if matches!(op, BinaryOp::Assign | BinaryOp::AddAssign | BinaryOp::SubAssign
                | BinaryOp::MulAssign | BinaryOp::DivAssign | BinaryOp::ModAssign) {
                if let Expr::Identifier(n) = &left.node {
                    if candidates.contains(n) { out.insert(n.clone()); }
                }
                let mut base = left.as_ref();
                loop {
                    match &base.node {
                        Expr::FieldAccess { base: inner, .. } | Expr::Index { base: inner, .. } => base = inner,
                        Expr::Identifier(n) => {
                            if candidates.contains(n) { out.insert(n.clone()); }
                            break;
                        }
                        _ => break,
                    }
                }
            }
            collect_assigned_idents(left, candidates, out);
            collect_assigned_idents(right, candidates, out);
        }
        Expr::Unary { right, .. } => collect_assigned_idents(right, candidates, out),
        Expr::Call { callee, args } => {
            collect_assigned_idents(callee, candidates, out);
            for a in args { collect_assigned_idents(a, candidates, out); }
        }
        Expr::If { condition, consequence, alternative } => {
            collect_assigned_idents(condition, candidates, out);
            collect_assigned_idents(consequence, candidates, out);
            if let Some(a) = alternative { collect_assigned_idents(a, candidates, out); }
        }
        Expr::Match { scrutinee, arms } => {
            collect_assigned_idents(scrutinee, candidates, out);
            for arm in arms { collect_assigned_idents(&arm.body, candidates, out); }
        }
        Expr::Block(b) => {
            for stmt in &b.stmts {
                match &stmt.node {
                    Stmt::Let { value, .. } => collect_assigned_idents(value, candidates, out),
                    Stmt::Expr(e) => collect_assigned_idents(e, candidates, out),
                    Stmt::Empty => {}
                }
            }
            if let Some(r) = &b.ret { collect_assigned_idents(r, candidates, out); }
        }
        Expr::While { condition, body } => {
            collect_assigned_idents(condition, candidates, out);
            collect_assigned_idents(&Spanned { node: Expr::Block(body.clone()), span: expr.span }, candidates, out);
        }
        Expr::For { iter, body, .. } => {
            collect_assigned_idents(iter, candidates, out);
            collect_assigned_idents(&Spanned { node: Expr::Block(body.clone()), span: expr.span }, candidates, out);
        }
        Expr::Loop { body } | Expr::Region { body, .. } => {
            collect_assigned_idents(&Spanned { node: Expr::Block(body.clone()), span: expr.span }, candidates, out);
        }
        Expr::Handle { expr: e, arms } => {
            collect_assigned_idents(e, candidates, out);
            for arm in arms { collect_assigned_idents(&arm.body, candidates, out); }
        }
        Expr::Return(Some(e)) | Expr::Throw(e) | Expr::Question(e) | Expr::Paren(e) => {
            collect_assigned_idents(e, candidates, out);
        }
        Expr::Resume(Some(e)) => collect_assigned_idents(e, candidates, out),
        Expr::Index { base, index } => {
            collect_assigned_idents(base, candidates, out);
            collect_assigned_idents(index, candidates, out);
        }
        Expr::FieldAccess { base, .. } => collect_assigned_idents(base, candidates, out),
        Expr::Tuple(elems) | Expr::Array(elems) => {
            for e in elems { collect_assigned_idents(e, candidates, out); }
        }
        Expr::ArrayRepeat { elem, count } => {
            collect_assigned_idents(elem, candidates, out);
            collect_assigned_idents(count, candidates, out);
        }
        Expr::Closure { body, .. } => collect_assigned_idents(body, candidates, out),
        _ => {}
    }
}

pub(in crate::compiler) fn rewrite_captures(
    expr: &Spanned<Expr>,
    layout: &HashMap<String, usize>,
    params: &HashSet<String>,
    self_name: Option<&str>,
    lifted_fn_name: &str,
) -> Spanned<Expr> {
    rewrite_captures_with_cells(expr, layout, params, self_name, lifted_fn_name, &HashSet::new())
}

pub(in crate::compiler) fn rewrite_captures_with_cells(
    expr: &Spanned<Expr>,
    layout: &HashMap<String, usize>,
    params: &HashSet<String>,
    self_name: Option<&str>,
    lifted_fn_name: &str,
    cells: &HashSet<String>,
) -> Spanned<Expr> {
    Spanned {
        node: rewrite_node(&expr.node, layout, params, expr.span, self_name, lifted_fn_name, cells),
        span: expr.span,
    }
}

fn rw(
    e: &Spanned<Expr>,
    layout: &HashMap<String, usize>,
    params: &HashSet<String>,
    self_name: Option<&str>,
    lifted_fn_name: &str,
    cells: &HashSet<String>,
) -> Spanned<Expr> {
    rewrite_captures_with_cells(e, layout, params, self_name, lifted_fn_name, cells)
}

fn rewrite_node(
    node: &Expr,
    layout: &HashMap<String, usize>,
    params: &HashSet<String>,
    span: Span,
    self_name: Option<&str>,
    lifted_fn_name: &str,
    cells: &HashSet<String>,
) -> Expr {
    match node {
        Expr::Identifier(name) => {
            if params.contains(name) {
                return Expr::Identifier(name.clone());
            }
            let Some(&idx) = layout.get(name) else {
                return Expr::Identifier(name.clone());
            };
            let builtin = if cells.contains(name) { "__cell_load" } else { "__env_load" };
            Expr::Call {
                callee: Box::new(Spanned {
                    node: Expr::Identifier(builtin.into()),
                    span,
                }),
                args: vec![
                    Spanned { node: Expr::Identifier("__env".into()), span },
                    Spanned { node: Expr::Literal(Literal::Int(idx as i64)), span },
                ],
            }
        }
        Expr::Binary { op, left, right } => {
            if matches!(op, BinaryOp::Assign) {
                if let Expr::Identifier(name) = &left.node {
                    if cells.contains(name) && !params.contains(name) {
                        if let Some(&idx) = layout.get(name) {
                            return Expr::Call {
                                callee: Box::new(Spanned {
                                    node: Expr::Identifier("__cell_store".into()),
                                    span,
                                }),
                                args: vec![
                                    Spanned { node: Expr::Identifier("__env".into()), span },
                                    Spanned { node: Expr::Literal(Literal::Int(idx as i64)), span },
                                    rw(right, layout, params, self_name, lifted_fn_name, cells),
                                ],
                            };
                        }
                    }
                }
            }
            Expr::Binary {
                op: op.clone(),
                left: Box::new(rw(left, layout, params, self_name, lifted_fn_name, cells)),
                right: Box::new(rw(right, layout, params, self_name, lifted_fn_name, cells)),
            }
        }
        Expr::Unary { op, right } => Expr::Unary {
            op: op.clone(),
            right: Box::new(rw(right, layout, params, self_name, lifted_fn_name, cells)),
        },
        Expr::Call { callee, args } => {
            if let (Some(sn), Expr::Identifier(name)) = (self_name, &callee.node) {
                if name == sn && !params.contains(name) && !layout.contains_key(name) {
                    let mut new_args = vec![Spanned {
                        node: Expr::Identifier("__env".into()),
                        span: callee.span,
                    }];
                    for a in args {
                        new_args.push(rw(a, layout, params, self_name, lifted_fn_name, cells));
                    }
                    return Expr::Call {
                        callee: Box::new(Spanned {
                            node: Expr::Identifier(lifted_fn_name.into()),
                            span: callee.span,
                        }),
                        args: new_args,
                    };
                }
            }
            Expr::Call {
                callee: Box::new(rw(callee, layout, params, self_name, lifted_fn_name, cells)),
                args: args.iter().map(|a| rw(a, layout, params, self_name, lifted_fn_name, cells)).collect(),
            }
        }
        Expr::If { condition, consequence, alternative } => Expr::If {
            condition: Box::new(rw(condition, layout, params, self_name, lifted_fn_name, cells)),
            consequence: Box::new(rw(consequence, layout, params, self_name, lifted_fn_name, cells)),
            alternative: alternative.as_ref().map(|a| Box::new(rw(a, layout, params, self_name, lifted_fn_name, cells))),
        },
        Expr::Block(b) => Expr::Block(rewrite_block(b, layout, params, self_name, lifted_fn_name, cells)),
        Expr::Return(opt) => Expr::Return(opt.as_ref().map(|e| Box::new(rw(e, layout, params, self_name, lifted_fn_name, cells)))),
        Expr::Throw(e) => Expr::Throw(Box::new(rw(e, layout, params, self_name, lifted_fn_name, cells))),
        Expr::Question(e) => Expr::Question(Box::new(rw(e, layout, params, self_name, lifted_fn_name, cells))),
        Expr::Paren(e) => Expr::Paren(Box::new(rw(e, layout, params, self_name, lifted_fn_name, cells))),
        Expr::Index { base, index } => Expr::Index {
            base: Box::new(rw(base, layout, params, self_name, lifted_fn_name, cells)),
            index: Box::new(rw(index, layout, params, self_name, lifted_fn_name, cells)),
        },
        Expr::FieldAccess { base, field } => Expr::FieldAccess {
            base: Box::new(rw(base, layout, params, self_name, lifted_fn_name, cells)),
            field: field.clone(),
        },
        Expr::Tuple(elems) => Expr::Tuple(elems.iter().map(|e| rw(e, layout, params, self_name, lifted_fn_name, cells)).collect()),
        Expr::Array(elems) => Expr::Array(elems.iter().map(|e| rw(e, layout, params, self_name, lifted_fn_name, cells)).collect()),
        Expr::ArrayRepeat { elem, count } => Expr::ArrayRepeat {
            elem: Box::new(rw(elem, layout, params, self_name, lifted_fn_name, cells)),
            count: Box::new(rw(count, layout, params, self_name, lifted_fn_name, cells)),
        },
        Expr::Match { scrutinee, arms } => Expr::Match {
            scrutinee: Box::new(rw(scrutinee, layout, params, self_name, lifted_fn_name, cells)),
            arms: arms.iter().map(|a| MatchArm {
                pattern: a.pattern.clone(),
                guard: a.guard.as_ref().map(|g| rw(g, layout, params, self_name, lifted_fn_name, cells)),
                body: rw(&a.body, layout, params, self_name, lifted_fn_name, cells),
            }).collect(),
        },
        Expr::While { condition, body } => Expr::While {
            condition: Box::new(rw(condition, layout, params, self_name, lifted_fn_name, cells)),
            body: rewrite_block(body, layout, params, self_name, lifted_fn_name, cells),
        },
        Expr::For { pattern, iter, body } => Expr::For {
            pattern: pattern.clone(),
            iter: Box::new(rw(iter, layout, params, self_name, lifted_fn_name, cells)),
            body: rewrite_block(body, layout, params, self_name, lifted_fn_name, cells),
        },
        Expr::Loop { body } => Expr::Loop {
            body: rewrite_block(body, layout, params, self_name, lifted_fn_name, cells),
        },
        Expr::Region { label, body } => Expr::Region {
            label: label.clone(),
            body: rewrite_block(body, layout, params, self_name, lifted_fn_name, cells),
        },
        Expr::Handle { expr, arms } => Expr::Handle {
            expr: Box::new(rw(expr, layout, params, self_name, lifted_fn_name, cells)),
            arms: arms.iter().map(|a| HandleArm {
                kind: a.kind.clone(),
                pattern: a.pattern.clone(),
                body: rw(&a.body, layout, params, self_name, lifted_fn_name, cells),
            }).collect(),
        },
        Expr::Resume(opt) => Expr::Resume(
            opt.as_ref().map(|e| Box::new(rw(e, layout, params, self_name, lifted_fn_name, cells)))
        ),
        Expr::Break(opt) => Expr::Break(
            opt.as_ref().map(|e| Box::new(rw(e, layout, params, self_name, lifted_fn_name, cells)))
        ),
        _ => node.clone(),
    }
}

fn rewrite_block(
    block: &Block,
    layout: &HashMap<String, usize>,
    params: &HashSet<String>,
    self_name: Option<&str>,
    lifted_fn_name: &str,
    cells: &HashSet<String>,
) -> Block {
    let stmts = block.stmts.iter().map(|s| {
        let node = match &s.node {
            Stmt::Let { pattern, is_mut, ty, value } => Stmt::Let {
                pattern: pattern.clone(),
                is_mut: *is_mut,
                ty: ty.clone(),
                value: rw(value, layout, params, self_name, lifted_fn_name, cells),
            },
            Stmt::Expr(e) => Stmt::Expr(rw(e, layout, params, self_name, lifted_fn_name, cells)),
            Stmt::Empty => Stmt::Empty,
        };
        Spanned { node, span: s.span }
    }).collect();
    let ret = block.ret.as_ref().map(|r| Box::new(rw(r, layout, params, self_name, lifted_fn_name, cells)));
    Block { stmts, ret }
}

fn infer_type_from_expr(expr: &Spanned<Expr>) -> Option<Type> {
    match &expr.node {
        Expr::Literal(Literal::Int(_)) => Some(Type::Named("Int".into())),
        Expr::Literal(Literal::Float(_)) => Some(Type::Named("Float".into())),
        Expr::Literal(Literal::Bool(_)) => Some(Type::Named("Bool".into())),
        Expr::Literal(Literal::String(_)) => Some(Type::Named("String".into())),
        Expr::Literal(Literal::Unit) => Some(Type::Named("Unit".into())),
        Expr::Record { ty, .. } => ty.last().map(|n| Type::Named(n.clone())),
        Expr::Variant { ty, .. } => ty.last().map(|n| Type::Named(n.clone())),
        Expr::Paren(inner) => infer_type_from_expr(inner),
        _ => None,
    }
}
