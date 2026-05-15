// Effect-handler lowering.
use crate::ast::*;
use crate::compiler::closures::{collect_free_vars, rewrite_captures, CaptureInfo};
use std::collections::{HashMap, HashSet};

pub struct HandleLowering {
    pub effect_op_to_arm: HashMap<(String, String), String>,
    pub op_call_to_arm: HashMap<Span, String>,
    pub return_arm_by_handle: HashMap<Span, String>,
    pub synthetic_fns: Vec<FnDecl>,
    pub arm_captures: HashMap<String, Vec<CaptureInfo>>,
    next_id: usize,
    handle_stack: Vec<HashMap<(String, String), String>>,
}

impl HandleLowering {
    pub fn new() -> Self {
        Self {
            effect_op_to_arm: HashMap::new(),
            op_call_to_arm: HashMap::new(),
            return_arm_by_handle: HashMap::new(),
            synthetic_fns: Vec::new(),
            arm_captures: HashMap::new(),
            next_id: 0,
            handle_stack: Vec::new(),
        }
    }

    pub fn lower(&mut self, ast: &[Decl]) {
        let mut op_sigs: HashMap<(String, String), FnSignature> = HashMap::new();
        for decl in ast {
            if let Decl::Effect { name, ops, .. } = decl {
                for op in ops {
                    op_sigs.insert((name.clone(), op.name.clone()), op.clone());
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
                    let bind_ty = ty.clone().unwrap_or(Type::Named("Unknown".into()));
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
            Expr::Return(Some(e)) | Expr::Throw(e) | Expr::Question(e) => self.walk_expr(e, op_sigs, scope),
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
        for arm in arms {
            match &arm.kind {
                HandleArmKind::Return => {
                    let fn_name = format!("__handle_return_{}", self.next_id);
                    self.next_id += 1;
                    let (params, body) = self.build_arm_fn(
                        &fn_name,
                        arm.pattern.as_ref(),
                        None,
                        &arm.body,
                        scope,
                    );
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
                    self.return_arm_by_handle.insert(handle_span, fn_name);
                }
                HandleArmKind::Effect(path) => {
                    if path.len() < 2 {
                        continue;
                    }
                    let effect_name = path[..path.len()-1].join(".");
                    let op_name = path.last().unwrap().clone();
                    let fn_name = format!("__handle_op_{}_{}_{}", effect_name, op_name, self.next_id);
                    self.next_id += 1;

                    let key = (effect_name.clone(), op_name.clone());
                    let param_ty = op_sigs.get(&key).and_then(|sig| {
                        sig.params.iter().find_map(|p| match p {
                            Param::Named { ty, .. } => Some(ty.clone()),
                            _ => None,
                        })
                    });
                    let (params, body) = self.build_arm_fn(
                        &fn_name,
                        arm.pattern.as_ref(),
                        param_ty,
                        &arm.body,
                        scope,
                    );
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
                    self.effect_op_to_arm.insert(key.clone(), fn_name.clone());
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

        // Rewrite captured-name references in the body to env loads.
        let layout: HashMap<String, usize> = captures.iter()
            .enumerate()
            .map(|(i, c)| (c.name.clone(), i))
            .collect();
        let rewritten_body = rewrite_captures(body, &layout, &param_names);

        // Build the param list: __env first, then the arm pattern (if any).
        let mut params: Vec<Param> = Vec::new();
        params.push(Param::Named {
            pattern: Spanned { node: Pattern::Bind("__env".into()), span: body.span },
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
}

