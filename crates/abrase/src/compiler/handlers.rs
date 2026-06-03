// Effect-handler lowering.
use crate::ast::*;
use crate::compiler::closures::{collect_assigned_idents, collect_free_vars, rewrite_captures_with_cells, CaptureInfo};
use std::collections::{HashMap, HashSet};

fn infer_bind_ty(value: &Expr) -> Option<Type> {
    match value {
        Expr::Record { ty, .. } => ty.last().map(|n| Type::Named(n.clone())),
        Expr::Paren(e) => infer_bind_ty(&e.node),
        _ => None,
    }
}

pub struct HandleLowering {
    pub effect_op_to_arm: HashMap<(String, String), String>,
    pub op_call_to_arm: HashMap<Span, String>,
    pub return_arm_by_handle: HashMap<Span, String>,
    pub effect_arms_by_handle: HashMap<Span, HashMap<(String, String), String>>,
    pub synthetic_fns: Vec<FnDecl>,
    pub arm_captures: HashMap<String, Vec<CaptureInfo>>,
    pub cell_vars: HashSet<String>,
    pub effect_ids: HashMap<String, u16>,
    pub op_ids: HashMap<(String, String), u8>,
    pub effect_op_counts: HashMap<String, u8>,
    pub arm_to_handle: HashMap<String, Span>,
    pub arm_resume_counts: HashMap<String, usize>,
    pub arm_resume_in_tail: HashMap<String, bool>,
    pub errors: Vec<String>,
    next_id: usize,
    handle_stack: Vec<HashMap<(String, String), String>>,
    pub debug_sink: Option<crate::compiler::debug::CompileDebugSink>,
}

impl HandleLowering {
    pub fn new() -> Self {
        Self {
            effect_op_to_arm: HashMap::new(),
            op_call_to_arm: HashMap::new(),
            return_arm_by_handle: HashMap::new(),
            effect_arms_by_handle: HashMap::new(),
            synthetic_fns: Vec::new(),
            arm_captures: HashMap::new(),
            cell_vars: HashSet::new(),
            effect_ids: HashMap::new(),
            op_ids: HashMap::new(),
            effect_op_counts: HashMap::new(),
            arm_to_handle: HashMap::new(),
            arm_resume_counts: HashMap::new(),
            arm_resume_in_tail: HashMap::new(),
            errors: Vec::new(),
            next_id: 0,
            handle_stack: Vec::new(),
            debug_sink: None,
        }
    }

    pub fn with_debug_sink(mut self, sink: crate::compiler::debug::CompileDebugSink) -> Self {
        self.debug_sink = Some(sink);
        self
    }

    pub fn lower(&mut self, ast: &[Decl]) {
        let mut op_sigs: HashMap<(String, String), FnSignature> = HashMap::new();
        let mut next_eff: u16 = 1;
        if let Some(sink) = &mut self.debug_sink {
            sink(&format!("[lower] processing {} declarations", ast.len()));
        }
        for decl in ast {
            if let Decl::Effect { name, ops, .. } = decl {
                if let Some(sink) = &mut self.debug_sink {
                    sink(&format!("[lower] found Effect '{}' with {} ops", name, ops.len()));
                }
                if !self.effect_ids.contains_key(name) {
                    self.effect_ids.insert(name.clone(), next_eff);
                    next_eff += 1;
                }
                for (i, op) in ops.iter().enumerate() {
                    op_sigs.insert((name.clone(), op.name.clone()), op.clone());
                    self.op_ids.insert((name.clone(), op.name.clone()), i as u8);
                }
                self.effect_op_counts.insert(name.clone(), ops.len() as u8);
                if let Some(sink) = &mut self.debug_sink {
                    sink(&format!("[lower] effect_op_counts[{}] = {}", name, ops.len()));
                }
            }
        }

        for decl in ast {
            if let Decl::Fn(fn_decl) = decl {
                let mut scope = ScopeStack::from_fn(fn_decl);
                self.walk_block(&fn_decl.body, &op_sigs, &mut scope);
            }
        }
    }

    fn walk_block(
        &mut self,
        block: &Block,
        op_sigs: &HashMap<(String, String), FnSignature>,
        scope: &mut ScopeStack,
    ) {
        scope.push();
        for stmt in &block.stmts {
            self.walk_stmt(stmt, op_sigs, scope);
        }
        if let Some(ret) = &block.ret {
            self.walk_expr(ret, op_sigs, scope);
        }
        scope.pop();
    }

    fn walk_stmt(
        &mut self,
        stmt: &Spanned<Stmt>,
        op_sigs: &HashMap<(String, String), FnSignature>,
        scope: &mut ScopeStack,
    ) {
        match &stmt.node {
            Stmt::Let { pattern, value, ty, .. } => {
                self.walk_expr(value, op_sigs, scope);
                if let Pattern::Bind(name) = &pattern.node {
                    let bind_ty = ty.clone()
                        .or_else(|| infer_bind_ty(&value.node))
                        .unwrap_or(Type::Named("Unknown".into()));
                    scope.bind(name.clone(), bind_ty);
                }
            }
            Stmt::Expr(e) => self.walk_expr(e, op_sigs, scope),
            Stmt::Empty => {}
        }
    }

    fn walk_expr(
        &mut self,
        expr: &Spanned<Expr>,
        op_sigs: &HashMap<(String, String), FnSignature>,
        scope: &mut ScopeStack,
    ) {
        match &expr.node {
            Expr::Handle { expr: body, arms } => {
                let local_dispatch = self.lift_arms(expr.span, arms, op_sigs, scope);
                self.effect_arms_by_handle.insert(expr.span, local_dispatch.clone());
                self.handle_stack.push(local_dispatch);
                self.walk_expr(body, op_sigs, scope);
                self.handle_stack.pop();
                for arm in arms {
                    self.walk_expr(&arm.body, op_sigs, scope);
                }
            }
            // Recurse into containing forms.
            Expr::Binary { left, right, .. } => {
                self.walk_expr(left, op_sigs, scope);
                self.walk_expr(right, op_sigs, scope);
            }
            Expr::Unary { right, .. } => self.walk_expr(right, op_sigs, scope),
            Expr::Call { callee, args } => {
                if let Expr::FieldAccess { base, field } = &callee.node {
                    if let Expr::Identifier(eff_name) = &base.node {
                        let key = (eff_name.clone(), field.clone());
                        for entry in self.handle_stack.iter().rev() {
                            if let Some(fn_name) = entry.get(&key) {
                                self.op_call_to_arm.insert(expr.span, fn_name.clone());
                                break;
                            }
                        }
                    }
                }
                self.walk_expr(callee, op_sigs, scope);
                for a in args { self.walk_expr(a, op_sigs, scope); }
            }
            Expr::If { condition, consequence, alternative } => {
                self.walk_expr(condition, op_sigs, scope);
                self.walk_expr(consequence, op_sigs, scope);
                if let Some(a) = alternative { self.walk_expr(a, op_sigs, scope); }
            }
            Expr::Match { scrutinee, arms } => {
                self.walk_expr(scrutinee, op_sigs, scope);
                for a in arms { self.walk_expr(&a.body, op_sigs, scope); }
            }
            Expr::Block(b) => self.walk_block(b, op_sigs, scope),
            Expr::While { condition, body } => {
                self.walk_expr(condition, op_sigs, scope);
                self.walk_block(body, op_sigs, scope);
            }
            Expr::For { iter, body, .. } => {
                self.walk_expr(iter, op_sigs, scope);
                self.walk_block(body, op_sigs, scope);
            }
            Expr::Loop { body } => self.walk_block(body, op_sigs, scope),
            Expr::Region { body, .. } => self.walk_block(body, op_sigs, scope),
            Expr::Return(Some(e)) | Expr::Throw(e) | Expr::Question(e) | Expr::Paren(e) => self.walk_expr(e, op_sigs, scope),
            Expr::Resume(Some(e)) => self.walk_expr(e, op_sigs, scope),
            Expr::Index { base, index } => {
                self.walk_expr(base, op_sigs, scope);
                self.walk_expr(index, op_sigs, scope);
            }
            Expr::FieldAccess { base, .. } => self.walk_expr(base, op_sigs, scope),
            Expr::Tuple(elems) | Expr::Array(elems) => {
                for e in elems { self.walk_expr(e, op_sigs, scope); }
            }
            Expr::ArrayRepeat { elem, count } => {
                self.walk_expr(elem, op_sigs, scope);
                self.walk_expr(count, op_sigs, scope);
            }
            _ => {}
        }
    }

    fn lift_arms(
        &mut self,
        handle_span: Span,
        arms: &[HandleArm],
        op_sigs: &HashMap<(String, String), FnSignature>,
        scope: &ScopeStack,
    ) -> HashMap<(String, String), String> {
        let mut local_dispatch: HashMap<(String, String), String> = HashMap::new();
        let outer_names: HashSet<String> = scope.all_names();
        for arm in arms {
            let mut params: HashSet<String> = HashSet::new();
            params.insert("__env".into());
            if let Some(p) = &arm.pattern {
                if let Pattern::Bind(n) = &p.node { params.insert(n.clone()); }
            }
            let mut frees: Vec<String> = Vec::new();
            let mut seen: HashSet<String> = HashSet::new();
            collect_free_vars(&arm.body, &params, &mut seen, &mut frees);
            let caps: HashSet<String> = frees.into_iter()
                .filter(|n| outer_names.contains(n))
                .collect();
            let mut assigned: HashSet<String> = HashSet::new();
            collect_assigned_idents(&arm.body, &caps, &mut assigned);
            for n in assigned { self.cell_vars.insert(n); }
        }
        for arm in arms {
            match &arm.kind {
                HandleArmKind::Return => {
                    let fn_name = format!("__handle_return_{}", self.next_id);
                    self.next_id += 1;
                    let resume_count = self.count_resumes_in_expr(&arm.body);
                    if resume_count > 1 {
                        self.errors.push("Multiple resume calls in one handler arm not yet implemented".to_string());
                    }
                    let resume_in_tail = self.all_resumes_in_tail(&arm.body);
                    let (params, body) = self.build_arm_fn(
                        &fn_name,
                        handle_span,
                        arm.pattern.as_ref(),
                        None,
                        &arm.body,
                        scope,
                    );
                    self.arm_resume_counts.insert(fn_name.clone(), resume_count);
                    self.arm_resume_in_tail.insert(fn_name.clone(), resume_in_tail);
                    self.synthetic_fns.push(FnDecl {
                        attrs: vec![],
                        is_pub: false,
                        name: fn_name.clone(),
                        generics: vec![],
                        params,
                        effects: vec![],
                        return_type: None,
                        where_clause: vec![],
                        body,
                    });
                    self.return_arm_by_handle.insert(handle_span, fn_name.clone());
                    self.arm_to_handle.insert(fn_name, handle_span);
                }
                HandleArmKind::Effect(path) => {
                    let Some(op_name) = path.last().cloned() else { continue };
                    if path.len() < 2 {
                        continue;
                    }
                    let effect_name = path[..path.len()-1].join(".");
                    let fn_name = format!("__handle_op_{}_{}_{}", effect_name, op_name, self.next_id);
                    self.next_id += 1;

                    let key = (effect_name.clone(), op_name.clone());
                    let param_ty = op_sigs.get(&key).and_then(|sig| {
                        sig.params.iter().find_map(|p| match p {
                            Param::Named { ty, .. } => Some(ty.clone()),
                            _ => None,
                        })
                    });
                    let resume_count = self.count_resumes_in_expr(&arm.body);
                    if resume_count > 1 {
                        self.errors.push(format!(
                            "multi-shot resume not yet supported: handler arm for `{}.{}` calls `resume` {} times",
                            effect_name, op_name, resume_count
                        ));
                    }
                    let resume_in_tail = self.all_resumes_in_tail(&arm.body);
                    let (params, body) = self.build_arm_fn(
                        &fn_name,
                        handle_span,
                        arm.pattern.as_ref(),
                        param_ty,
                        &arm.body,
                        scope,
                    );
                    self.arm_resume_counts.insert(fn_name.clone(), resume_count);
                    self.arm_resume_in_tail.insert(fn_name.clone(), resume_in_tail);
                    let arm_effects = if Self::body_contains_throw(&arm.body) {
                        vec![EffectItem { name: vec!["exn".into()], arg: None }]
                    } else { vec![] };
                    self.synthetic_fns.push(FnDecl {
                        attrs: vec![],
                        is_pub: false,
                        name: fn_name.clone(),
                        generics: vec![],
                        params,
                        effects: arm_effects,
                        return_type: None,
                        where_clause: vec![],
                        body,
                    });
                    self.effect_op_to_arm.insert(key.clone(), fn_name.clone());
                    self.arm_to_handle.insert(fn_name.clone(), handle_span);
                    local_dispatch.insert(key, fn_name);
                }
                HandleArmKind::Exn => {
                    // exn handling already lowers through the Result variant
                    // protocol; nothing to lift here.
                }
            }
        }
        local_dispatch
    }

    fn build_arm_fn(
        &mut self,
        fn_name: &str,
        _handle_span: Span,
        pat: Option<&Spanned<Pattern>>,
        pat_ty: Option<Type>,
        body: &Spanned<Expr>,
        scope: &ScopeStack,
    ) -> (Vec<Param>, Block) {
        // Param name set used to shadow captures (arm's own param +
        // synthesised env handle).
        let mut param_names: HashSet<String> = HashSet::new();
        param_names.insert("__env".into());
        if let Some(p) = pat {
            if let Pattern::Bind(n) = &p.node { param_names.insert(n.clone()); }
        }

        // Collect free vars in the arm body, then keep only the ones that
        // resolve to a binding in the surrounding scope.
        let mut frees: Vec<String> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        collect_free_vars(body, &param_names, &mut seen, &mut frees);
        let captures: Vec<CaptureInfo> = frees.into_iter()
            .filter_map(|name| {
                scope.lookup(&name).map(|ty| CaptureInfo { name, ty })
            })
            .collect();

        let layout: HashMap<String, usize> = captures.iter()
            .enumerate()
            .map(|(i, c)| (c.name.clone(), i))
            .collect();
        let capture_set: HashSet<String> = captures.iter().map(|c| c.name.clone()).collect();
        let mut cells: HashSet<String> = HashSet::new();
        collect_assigned_idents(body, &capture_set, &mut cells);
        for n in &cells { self.cell_vars.insert(n.clone()); }
        for n in &capture_set {
            if self.cell_vars.contains(n) { cells.insert(n.clone()); }
        }
        let rewritten_body = rewrite_captures_with_cells(body, &layout, &param_names, None, "", &cells);

        // Build the param list: __env, __return_env, then the arm pattern (if any).
        let mut params: Vec<Param> = Vec::new();
        params.push(Param::Named {
            pattern: Spanned { node: Pattern::Bind("__env".into()), span: body.span },
            ty: Type::Named("Int".into()),
        });
        params.push(Param::Named {
            pattern: Spanned { node: Pattern::Bind("__return_env".into()), span: body.span },
            ty: Type::Named("Int".into()),
        });
        if let Some(p) = pat {
            let ty = pat_ty.unwrap_or(Type::Named("Int".into()));
            let bind_name = if let Pattern::Bind(n) = &p.node {
                n.clone()
            } else {
                "_arg".into()
            };
            params.push(Param::Named {
                pattern: Spanned { node: Pattern::Bind(bind_name), span: p.span },
                ty,
            });
        }

        self.arm_captures.insert(fn_name.to_string(), captures);

        let body = Block {
            stmts: vec![],
            ret: Some(Box::new(rewritten_body)),
        };
        (params, body)
    }

    fn body_contains_throw(expr: &Spanned<Expr>) -> bool {
        match &expr.node {
            Expr::Throw(_) => true,
            Expr::Block(b) => b.stmts.iter().any(|s| match &s.node {
                Stmt::Expr(e) => Self::body_contains_throw(e),
                Stmt::Let { value, .. } => Self::body_contains_throw(value),
                _ => false,
            }) || b.ret.as_deref().map_or(false, Self::body_contains_throw),
            Expr::If { condition, consequence, alternative } =>
                Self::body_contains_throw(condition) || Self::body_contains_throw(consequence)
                    || alternative.as_deref().map_or(false, Self::body_contains_throw),
            Expr::Match { scrutinee, arms } =>
                Self::body_contains_throw(scrutinee)
                    || arms.iter().any(|a| Self::body_contains_throw(&a.body)),
            Expr::Binary { left, right, .. } =>
                Self::body_contains_throw(left) || Self::body_contains_throw(right),
            Expr::Unary { right, .. } | Expr::Paren(right) | Expr::Question(right) =>
                Self::body_contains_throw(right),
            Expr::Call { callee, args } =>
                Self::body_contains_throw(callee) || args.iter().any(Self::body_contains_throw),
            Expr::Region { body, .. } => body.ret.as_deref().map_or(false, Self::body_contains_throw)
                || body.stmts.iter().any(|s| match &s.node {
                    Stmt::Expr(e) => Self::body_contains_throw(e),
                    Stmt::Let { value, .. } => Self::body_contains_throw(value),
                    _ => false,
                }),
            _ => false,
        }
    }

    fn count_resumes_in_expr(&self, expr: &Spanned<Expr>) -> usize {
        match &expr.node {
            Expr::Resume(_) => 1,
            Expr::Binary { left, right, .. } => {
                self.count_resumes_in_expr(left) + self.count_resumes_in_expr(right)
            }
            Expr::Unary { right, .. } => self.count_resumes_in_expr(right),
            Expr::Call { callee, args } => {
                self.count_resumes_in_expr(callee) + args.iter().map(|a| self.count_resumes_in_expr(a)).sum::<usize>()
            }
            Expr::Index { base, index } => {
                self.count_resumes_in_expr(base) + self.count_resumes_in_expr(index)
            }
            Expr::Block(b) => {
                b.stmts.iter().map(|s| self.count_resumes_in_stmt(s)).sum::<usize>() +
                b.ret.as_ref().map(|r| self.count_resumes_in_expr(r)).unwrap_or(0)
            }
            Expr::If { condition, consequence, alternative } => {
                self.count_resumes_in_expr(condition) +
                self.count_resumes_in_expr(consequence) +
                alternative.as_ref().map(|a| self.count_resumes_in_expr(a)).unwrap_or(0)
            }
            Expr::Match { scrutinee, arms } => {
                self.count_resumes_in_expr(scrutinee) +
                arms.iter().map(|a| self.count_resumes_in_expr(&a.body)).sum::<usize>()
            }
            Expr::For { iter, body, .. } => {
                self.count_resumes_in_expr(iter) +
                body.stmts.iter().map(|s| self.count_resumes_in_stmt(s)).sum::<usize>() +
                body.ret.as_ref().map(|r| self.count_resumes_in_expr(r)).unwrap_or(0)
            }
            Expr::While { condition, body } => {
                self.count_resumes_in_expr(condition) +
                body.stmts.iter().map(|s| self.count_resumes_in_stmt(s)).sum::<usize>() +
                body.ret.as_ref().map(|r| self.count_resumes_in_expr(r)).unwrap_or(0)
            }
            Expr::Loop { body } => {
                body.stmts.iter().map(|s| self.count_resumes_in_stmt(s)).sum::<usize>() +
                body.ret.as_ref().map(|r| self.count_resumes_in_expr(r)).unwrap_or(0)
            }
            Expr::Break(Some(e)) => self.count_resumes_in_expr(e),
            Expr::Return(Some(e)) | Expr::Throw(e) | Expr::Question(e) | Expr::Paren(e) => self.count_resumes_in_expr(e),
            Expr::Tuple(elems) | Expr::Array(elems) => {
                elems.iter().map(|e| self.count_resumes_in_expr(e)).sum::<usize>()
            }
            Expr::ArrayRepeat { elem, count } => {
                self.count_resumes_in_expr(elem) + self.count_resumes_in_expr(count)
            }
            Expr::Variant { args, .. } => {
                args.iter().map(|a| self.count_resumes_in_expr(a)).sum::<usize>()
            }
            Expr::FieldAccess { base, .. } => self.count_resumes_in_expr(base),
            Expr::Closure { body, .. } => self.count_resumes_in_expr(body),
            Expr::Range { start, end, .. } => {
                start.as_ref().map(|s| self.count_resumes_in_expr(s)).unwrap_or(0) +
                end.as_ref().map(|e| self.count_resumes_in_expr(e)).unwrap_or(0)
            }
            Expr::Region { body, .. } => {
                body.stmts.iter().map(|s| self.count_resumes_in_stmt(s)).sum::<usize>() +
                body.ret.as_ref().map(|r| self.count_resumes_in_expr(r)).unwrap_or(0)
            }
            Expr::Handle { expr, arms } => {
                self.count_resumes_in_expr(expr) +
                arms.iter().map(|a| self.count_resumes_in_expr(&a.body)).sum::<usize>()
            }
            _ => 0,
        }
    }

    fn count_resumes_in_stmt(&self, stmt: &Spanned<Stmt>) -> usize {
        match &stmt.node {
            Stmt::Let { value, .. } => self.count_resumes_in_expr(value),
            Stmt::Expr(e) => self.count_resumes_in_expr(e),
            Stmt::Empty => 0,
        }
    }

    // True when every `resume(...)` inside `expr` is in tail position of the
    // arm body. Tail-position resume can lower to a direct `Ret`; non-tail
    // resume must capture the continuation so the arm body can keep computing
    // with resume's return value (e.g. `v + resume(())`).
    fn all_resumes_in_tail(&self, expr: &Spanned<Expr>) -> bool {
        match &expr.node {
            Expr::Resume(_) => true,
            Expr::Block(b) => {
                b.stmts.iter().all(|s| self.count_resumes_in_stmt(s) == 0)
                    && b.ret.as_ref().map_or(true, |r| self.all_resumes_in_tail(r))
            }
            Expr::If { condition, consequence, alternative } => {
                self.count_resumes_in_expr(condition) == 0
                    && self.all_resumes_in_tail(consequence)
                    && alternative.as_ref().map_or(true, |a| self.all_resumes_in_tail(a))
            }
            Expr::Match { scrutinee, arms } => {
                self.count_resumes_in_expr(scrutinee) == 0
                    && arms.iter().all(|a| self.all_resumes_in_tail(&a.body))
            }
            Expr::Region { body, .. } => {
                body.stmts.iter().all(|s| self.count_resumes_in_stmt(s) == 0)
                    && body.ret.as_ref().map_or(true, |r| self.all_resumes_in_tail(r))
            }
            _ => self.count_resumes_in_expr(expr) == 0,
        }
    }
}

struct ScopeStack {
    scopes: Vec<HashMap<String, Type>>,
}

impl ScopeStack {
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
    fn push(&mut self) { self.scopes.push(HashMap::new()); }
    fn pop(&mut self) { self.scopes.pop(); }
    fn bind(&mut self, name: String, ty: Type) {
        if let Some(top) = self.scopes.last_mut() { top.insert(name, ty); }
    }
    fn lookup(&self, name: &str) -> Option<Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(t) = scope.get(name) { return Some(t.clone()); }
        }
        None
    }
    fn all_names(&self) -> HashSet<String> {
        self.scopes.iter().flat_map(|s| s.keys().cloned()).collect()
    }
}

