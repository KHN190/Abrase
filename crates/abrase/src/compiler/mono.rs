use std::collections::{HashMap, HashSet};

use crate::ast::*;
use crate::error::{Error, ErrorCode};

pub fn monomorphize(decls: Vec<Decl>) -> Result<Vec<Decl>, Vec<Error>> {
    Mono::new(decls, HashMap::new()).run()
}

pub fn monomorphize_with_methods(
    decls: Vec<Decl>,
    method_dispatch: HashMap<(String, String), String>,
) -> Result<Vec<Decl>, Vec<Error>> {
    Mono::new(decls, method_dispatch).run()
}

struct Mono {
    generic_fns: HashMap<String, FnDecl>,
    fn_sigs: HashMap<String, (Vec<Type>, Type, Vec<EffectItem>)>,
    /// `(receiver_type_name, method_name) -> mangled fn name`. Empty for
    /// plain `monomorphize` callers; populated when impl-lift has run.
    method_dispatch: HashMap<(String, String), String>,
    out_passthrough: Vec<Decl>,
    out_concrete: Vec<FnDecl>,
    out_specials: Vec<FnDecl>,
    pending: Vec<(String, Vec<Type>)>,
    done: HashSet<String>,
    errors: Vec<Error>,
}

impl Mono {
    fn new(decls: Vec<Decl>, method_dispatch: HashMap<(String, String), String>) -> Self {
        let mut generic_fns = HashMap::new();
        let mut fn_sigs = HashMap::new();
        let mut out_passthrough = Vec::new();
        let mut out_concrete = Vec::new();
        for decl in decls {
            match decl {
                Decl::Fn(fd) if !fd.generics.is_empty() => {
                    generic_fns.insert(fd.name.clone(), fd);
                }
                Decl::Fn(fd) => {
                    fn_sigs.insert(fd.name.clone(), fn_sig(&fd));
                    out_concrete.push(fd);
                }
                other => out_passthrough.push(other),
            }
        }
        Self {
            generic_fns,
            fn_sigs,
            method_dispatch,
            out_passthrough,
            out_concrete,
            out_specials: Vec::new(),
            pending: Vec::new(),
            done: HashSet::new(),
            errors: Vec::new(),
        }
    }

    fn run(mut self) -> Result<Vec<Decl>, Vec<Error>> {
        // Walk every concrete fn
        let concrete: Vec<FnDecl> = std::mem::take(&mut self.out_concrete);
        let mut rewritten_concrete = Vec::with_capacity(concrete.len());
        for mut fd in concrete {
            self.rewrite_fn_body(&mut fd, &HashMap::new());
            rewritten_concrete.push(fd);
        }
        self.out_concrete = rewritten_concrete;

        // Drain the worklist. Each pop emits a single specialization.
        while let Some((fn_name, type_args)) = self.pending.pop() {
            let generic_fd = match self.generic_fns.get(&fn_name) {
                Some(f) => f.clone(),
                None => continue,
            };
            let mangled = mangle(&fn_name, &type_args);
            if !self.done.insert(mangled.clone()) {
                continue;
            }
            if generic_fd.generics.len() != type_args.len() {
                self.errors.push(Error::new(
                    ErrorCode::CodegenError,
                    Span::new(0, 0),
                    format!(
                        "Generic '{}' expects {} type argument(s), got {}",
                        fn_name,
                        generic_fd.generics.len(),
                        type_args.len()
                    ),
                ));
                continue;
            }
            let mut subst: HashMap<String, Type> = HashMap::new();
            for (gp, ta) in generic_fd.generics.iter().zip(type_args.iter()) {
                subst.insert(gp.name.clone(), ta.clone());
            }
            let mut new_fd = generic_fd.clone();
            new_fd.name = mangled.clone();
            new_fd.generics = Vec::new();
            new_fd.where_clause = Vec::new();
            for p in &mut new_fd.params {
                if let Param::Named { ty, .. } = p {
                    *ty = subst_type(ty, &subst);
                }
            }
            if let Some(ret) = &mut new_fd.return_type {
                *ret = subst_type(ret, &subst);
            }
            // Register the specialization's signature before recursion.
            self.fn_sigs.insert(mangled.clone(), fn_sig(&new_fd));
            self.rewrite_fn_body(&mut new_fd, &subst);
            self.out_specials.push(new_fd);
        }

        // Assemble output: always include passthrough and concrete functions.
        let mut out = self.out_passthrough;
        for fd in self.out_concrete { out.push(Decl::Fn(fd)); }

        if !self.errors.is_empty() {
            return Ok(out);
        }

        // Only add specializations if no errors occurred.
        for fd in self.out_specials { out.push(Decl::Fn(fd)); }
        Ok(out)
    }

    fn rewrite_fn_body(&mut self, fd: &mut FnDecl, subst: &HashMap<String, Type>) {
        let mut env: HashMap<String, Type> = HashMap::new();
        for p in &fd.params {
            if let Param::Named { pattern, ty } = p {
                if let Pattern::Bind(name) = &pattern.node {
                    env.insert(name.clone(), ty.clone());
                }
            }
        }
        self.rewrite_block(&mut fd.body, &mut env, subst);
    }

    fn rewrite_block(
        &mut self,
        block: &mut Block,
        env: &mut HashMap<String, Type>,
        subst: &HashMap<String, Type>,
    ) {
        let saved = env.clone();
        for stmt in &mut block.stmts {
            self.rewrite_stmt(stmt, env, subst);
        }
        if let Some(ret) = &mut block.ret {
            self.rewrite_expr(&mut **ret, env, subst);
        }
        *env = saved;
    }

    fn rewrite_stmt(
        &mut self,
        stmt: &mut Spanned<Stmt>,
        env: &mut HashMap<String, Type>,
        subst: &HashMap<String, Type>,
    ) {
        match &mut stmt.node {
            Stmt::Let { pattern, ty, value, .. } => {
                if let Some(t) = ty.as_mut() { *t = subst_type(t, subst); }
                let inferred = self.rewrite_expr(value, env, subst);
                let bound_ty = ty.clone()
                    .or(inferred)
                    .unwrap_or(Type::Named("?".into()));
                if let Pattern::Bind(name) = &pattern.node {
                    env.insert(name.clone(), bound_ty);
                }
            }
            Stmt::Expr(e) => { self.rewrite_expr(e, env, subst); }
            Stmt::Empty => {}
        }
    }

    fn rewrite_expr(
        &mut self,
        expr: &mut Spanned<Expr>,
        env: &mut HashMap<String, Type>,
        subst: &HashMap<String, Type>,
    ) -> Option<Type> {

        self.try_rewrite_method_call(expr, env);

        let call_span = expr.span;
        match &mut expr.node {
            Expr::Literal(lit) => Some(lit_type(lit)),
            Expr::Identifier(name) => env.get(name).cloned()
                .or_else(|| self.fn_sigs.get(name).map(|(p, r, e)| Type::Function {
                    params: p.clone(),
                    effects: e.clone(),
                    ret: Box::new(r.clone()),
                })),
            Expr::Unary { right, .. } => self.rewrite_expr(&mut **right, env, subst),
            Expr::Binary { op, left, right } => {
                let l = self.rewrite_expr(&mut **left, env, subst);
                let _ = self.rewrite_expr(&mut **right, env, subst);
                match op {
                    BinaryOp::Eq | BinaryOp::Neq | BinaryOp::Lt | BinaryOp::Gt
                    | BinaryOp::Lte | BinaryOp::Gte
                    | BinaryOp::And | BinaryOp::Or => Some(Type::Named("Bool".into())),
                    _ => l,
                }
            }
            Expr::Call { callee, args } => {
                let mut arg_types: Vec<Option<Type>> = Vec::with_capacity(args.len());
                for a in args.iter_mut() {
                    arg_types.push(self.rewrite_expr(a, env, subst));
                }
                let callee_name = if let Expr::Identifier(n) = &callee.node {
                    Some(n.clone())
                } else { None };
                if let Some(name) = callee_name {
                    if let Some(generic_fd) = self.generic_fns.get(&name).cloned() {
                        let arg_concrete: Vec<Type> = arg_types.iter()
                            .map(|t| t.clone().unwrap_or(Type::Named("?".into())))
                            .collect();
                        if let Some(type_args) =
                            self.infer_type_args(&generic_fd, &arg_concrete, call_span)
                        {
                            let mangled = mangle(&name, &type_args);
                            if let Expr::Identifier(n) = &mut callee.node {
                                *n = mangled.clone();
                            }
                            if !self.done.contains(&mangled)
                                && !self.pending.iter().any(|(pn, pa)|
                                    pn == &name && pa == &type_args)
                            {
                                self.pending.push((name.clone(), type_args.clone()));
                            }
                            let mut local_subst = HashMap::new();
                            for (gp, ta) in generic_fd.generics.iter().zip(type_args.iter()) {
                                local_subst.insert(gp.name.clone(), ta.clone());
                            }
                            let ret = generic_fd.return_type.clone()
                                .unwrap_or(Type::Tuple(vec![]));
                            return Some(subst_type(&ret, &local_subst));
                        }
                        return None;
                    }
                    if let Some((_, ret, _)) = self.fn_sigs.get(&name) {
                        return Some(ret.clone());
                    }
                }
                self.rewrite_expr(&mut **callee, env, subst);
                None
            }
            Expr::Index { base, index } => {
                self.rewrite_expr(&mut **base, env, subst);
                self.rewrite_expr(&mut **index, env, subst);
                None
            }
            Expr::Block(block) => {
                self.rewrite_block(block, env, subst);
                None
            }
            Expr::If { condition, consequence, alternative } => {
                self.rewrite_expr(&mut **condition, env, subst);
                let c = self.rewrite_expr(&mut **consequence, env, subst);
                if let Some(alt) = alternative {
                    self.rewrite_expr(&mut **alt, env, subst);
                }
                c
            }
            Expr::Match { scrutinee, arms } => {
                self.rewrite_expr(&mut **scrutinee, env, subst);
                let mut last = None;
                for arm in arms {
                    if let Some(g) = &mut arm.guard {
                        self.rewrite_expr(g, env, subst);
                    }
                    last = self.rewrite_expr(&mut arm.body, env, subst);
                }
                last
            }
            Expr::For { iter, body, .. } => {
                self.rewrite_expr(&mut **iter, env, subst);
                self.rewrite_block(body, env, subst);
                None
            }
            Expr::While { condition, body } => {
                self.rewrite_expr(&mut **condition, env, subst);
                self.rewrite_block(body, env, subst);
                None
            }
            Expr::Loop { body } => { self.rewrite_block(body, env, subst); None }
            Expr::Break(opt) | Expr::Return(opt) => {
                if let Some(e) = opt { self.rewrite_expr(&mut **e, env, subst); }
                None
            }
            Expr::Continue | Expr::Error => None,
            Expr::Throw(e) => { self.rewrite_expr(&mut **e, env, subst); None }
            Expr::Question(e) => self.rewrite_expr(&mut **e, env, subst),
            Expr::Tuple(es) | Expr::Array(es) => {
                for e in es.iter_mut() { self.rewrite_expr(e, env, subst); }
                None
            }
            Expr::ArrayRepeat { elem, count } => {
                self.rewrite_expr(&mut **elem, env, subst);
                self.rewrite_expr(&mut **count, env, subst);
                None
            }
            Expr::Record { fields, .. } => {
                for fi in fields.iter_mut() {
                    if let Some(v) = &mut fi.value {
                        self.rewrite_expr(v, env, subst);
                    }
                }
                None
            }
            Expr::Variant { args, .. } => {
                for a in args.iter_mut() { self.rewrite_expr(a, env, subst); }
                None
            }
            Expr::FieldAccess { base, .. } => {
                self.rewrite_expr(&mut **base, env, subst);
                None
            }
            Expr::Closure { params, return_type, body, .. } => {
                for p in params.iter_mut() {
                    if let Some(t) = p.ty.as_mut() { *t = subst_type(t, subst); }
                }
                if let Some(t) = return_type.as_mut() { *t = subst_type(t, subst); }
                self.rewrite_expr(&mut **body, env, subst);
                None
            }
            Expr::Range { start, end, .. } => {
                if let Some(s) = start { self.rewrite_expr(&mut **s, env, subst); }
                if let Some(e) = end { self.rewrite_expr(&mut **e, env, subst); }
                None
            }
            Expr::Region { body, .. } => { self.rewrite_block(body, env, subst); None }
            Expr::Handle { expr: body, arms } => {
                self.rewrite_expr(&mut **body, env, subst);
                for arm in arms {
                    self.rewrite_expr(&mut arm.body, env, subst);
                }
                None
            }
            Expr::Resume(opt) => {
                if let Some(e) = opt { self.rewrite_expr(&mut **e, env, subst); }
                None
            }
        }
    }

    fn infer_type_args(
        &mut self,
        generic_fd: &FnDecl,
        arg_types: &[Type],
        span: Span,
    ) -> Option<Vec<Type>> {
        let gens: Vec<String> = generic_fd.generics.iter().map(|g| g.name.clone()).collect();
        let param_types: Vec<Type> = generic_fd.params.iter().filter_map(|p| match p {
            Param::Named { ty, .. } => Some(ty.clone()),
            _ => None,
        }).collect();
        if param_types.len() != arg_types.len() {
            self.errors.push(Error::new(
                ErrorCode::CodegenError,
                span,
                format!("Call to '{}' has {} argument(s), expected {}",
                    generic_fd.name, arg_types.len(), param_types.len()),
            ));
            return None;
        }
        let mut bindings: HashMap<String, Type> = HashMap::new();
        for (p, a) in param_types.iter().zip(arg_types.iter()) {
            unify(p, a, &gens, &mut bindings);
        }
        let mut out = Vec::with_capacity(gens.len());
        for g in &gens {
            match bindings.get(g) {
                Some(t) => out.push(t.clone()),
                None => {
                    self.errors.push(Error::new(
                        ErrorCode::CodegenError,
                        span,
                        format!("Cannot infer type parameter '{}' for call to '{}'",
                            g, generic_fd.name),
                    ));
                    return None;
                }
            }
        }
        Some(out)
    }

    /// Rewrite method calls to mangled direct calls if receiver is in dispatch table.
    fn try_rewrite_method_call(
        &mut self,
        expr: &mut Spanned<Expr>,
        env: &HashMap<String, Type>,
    ) {
        let mangled_opt: Option<String> = if let Expr::Call { callee, .. } = &expr.node {
            if let Expr::FieldAccess { base, field } = &callee.node {
                let base_ty = peek_type(base, env, &self.fn_sigs);
                base_ty.as_ref()
                    .and_then(|t| receiver_name_of(t))
                    .and_then(|n| self.method_dispatch.get(&(n, field.clone())).cloned())
            } else { None }
        } else { None };

        let mangled = match mangled_opt { Some(m) => m, None => return };

        let span = expr.span;
        let old = std::mem::replace(&mut expr.node, Expr::Error);
        if let Expr::Call { callee, args } = old {
            let callee_span = callee.span;
            if let Expr::FieldAccess { base, .. } = callee.node {
                let mut new_args = Vec::with_capacity(args.len() + 1);
                new_args.push(*base);
                new_args.extend(args);
                expr.node = Expr::Call {
                    callee: Box::new(Spanned {
                        node: Expr::Identifier(mangled),
                        span: callee_span,
                    }),
                    args: new_args,
                };
            } else {
                // Shouldn't happen given the guard above; restore.
                expr.node = Expr::Error;
                let _ = span;
            }
        }
    }
}

fn fn_sig(fd: &FnDecl) -> (Vec<Type>, Type, Vec<EffectItem>) {
    let params: Vec<Type> = fd.params.iter().filter_map(|p| match p {
        Param::Named { ty, .. } => Some(ty.clone()),
        _ => None,
    }).collect();
    let ret = fd.return_type.clone().unwrap_or(Type::Tuple(vec![]));
    let effects = fd.effects.clone();
    (params, ret, effects)
}

fn lit_type(lit: &Literal) -> Type {
    match lit {
        Literal::Int(_) => Type::Named("Int".into()),
        Literal::Float(_) => Type::Named("Float".into()),
        Literal::Bool(_) => Type::Named("Bool".into()),
        Literal::Char(_) => Type::Named("Char".into()),
        Literal::String(_) | Literal::StringInterp(_) => Type::Named("String".into()),
        Literal::Unit => Type::Tuple(vec![]),
    }
}

fn unify(param: &Type, arg: &Type, gens: &[String], subst: &mut HashMap<String, Type>) {
    match (param, arg) {
        (Type::Named(n), arg_ty) if gens.contains(n) => {
            subst.entry(n.clone()).or_insert_with(|| arg_ty.clone());
        }
        (Type::Generic { name: pn, args: pa }, Type::Generic { name: an, args: aa })
            if pn == an && pa.len() == aa.len() =>
        {
            for (p, a) in pa.iter().zip(aa.iter()) { unify(p, a, gens, subst); }
        }
        (Type::Tuple(ps), Type::Tuple(as_)) if ps.len() == as_.len() => {
            for (p, a) in ps.iter().zip(as_.iter()) { unify(p, a, gens, subst); }
        }
        (Type::Reference { inner: pi, .. }, Type::Reference { inner: ai, .. }) => {
            unify(pi, ai, gens, subst);
        }
        (Type::Function { params: pp, ret: pr, .. },
         Type::Function { params: ap, ret: ar, .. }) if pp.len() == ap.len() =>
        {
            for (p, a) in pp.iter().zip(ap.iter()) { unify(p, a, gens, subst); }
            unify(pr, ar, gens, subst);
        }
        _ => {}
    }
}

fn subst_type(ty: &Type, subst: &HashMap<String, Type>) -> Type {
    match ty {
        Type::Named(n) => subst.get(n).cloned().unwrap_or_else(|| ty.clone()),
        Type::Generic { name, args } => Type::Generic {
            name: name.clone(),
            args: args.iter().map(|a| subst_type(a, subst)).collect(),
        },
        Type::Tuple(ts) => Type::Tuple(ts.iter().map(|t| subst_type(t, subst)).collect()),
        Type::Reference { is_mut, inner, region } => Type::Reference {
            is_mut: *is_mut,
            inner: Box::new(subst_type(inner, subst)),
            region: region.clone(),
        },
        Type::Function { params, effects, ret } => Type::Function {
            params: params.iter().map(|p| subst_type(p, subst)).collect(),
            effects: effects.clone(),
            ret: Box::new(subst_type(ret, subst)),
        },
        Type::Array { elem, size } => Type::Array {
            elem: Box::new(subst_type(elem, subst)),
            size: *size,
        },
        Type::Qualified(_) => ty.clone(),
    }
}

fn mangle(base: &str, args: &[Type]) -> String {
    let mut s = String::from(base);
    s.push_str("__");
    for (i, arg) in args.iter().enumerate() {
        if i > 0 { s.push('_'); }
        s.push_str(&mangle_type(arg));
    }
    s
}

/// Non-mutating type peek for method-call rewriting (no env/AST mutations).
fn peek_type(
    expr: &Spanned<Expr>,
    env: &HashMap<String, Type>,
    fn_sigs: &HashMap<String, (Vec<Type>, Type, Vec<EffectItem>)>,
) -> Option<Type> {
    match &expr.node {
        Expr::Literal(lit) => Some(lit_type(lit)),
        Expr::Identifier(n) => env.get(n).cloned()
            .or_else(|| fn_sigs.get(n).map(|(_, r, _)| r.clone())),
        Expr::Unary { op, right } => {
            let inner = peek_type(right, env, fn_sigs)?;
            match op {
                UnaryOp::Ref => Some(Type::Reference {
                    is_mut: false,
                    inner: Box::new(inner),
                    region: None,
                }),
                UnaryOp::RefMut => Some(Type::Reference {
                    is_mut: true,
                    inner: Box::new(inner),
                    region: None,
                }),
                UnaryOp::Deref => match inner {
                    Type::Reference { inner, .. } => Some(*inner),
                    _ => None,
                },
                _ => Some(inner),
            }
        }
        _ => None,
    }
}

fn receiver_name_of(ty: &Type) -> Option<String> {
    match ty {
        Type::Named(n) => Some(n.clone()),
        Type::Reference { inner, .. } => receiver_name_of(inner),
        _ => None,
    }
}

fn mangle_type(ty: &Type) -> String {
    match ty {
        Type::Named(n) => n.clone(),
        Type::Qualified(parts) => parts.join("_"),
        Type::Generic { name, args } => {
            let mut s = name.clone();
            for a in args { s.push('_'); s.push_str(&mangle_type(a)); }
            s
        }
        Type::Tuple(ts) => {
            let mut s = String::from("Tup");
            for t in ts { s.push('_'); s.push_str(&mangle_type(t)); }
            s
        }
        Type::Reference { is_mut, inner, .. } => {
            let prefix = if *is_mut { "RefMut_" } else { "Ref_" };
            format!("{}{}", prefix, mangle_type(inner))
        }
        Type::Array { elem, size } => format!("Arr_{}_{}", mangle_type(elem), size),
        Type::Function { .. } => "Fn".to_string(),
    }
}
