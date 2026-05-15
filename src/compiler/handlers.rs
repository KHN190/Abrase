// Effect-handler lowering — TODO.
//
// Walks all FnDecls before codegen. For each Expr::Handle, synthesises one
// top-level FnDecl per arm (lifted to module scope) so codegen can `call` them
// like any other function. Builds two maps:
//
//   * `effect_op_to_arm`  : (effect_name, op_name) -> arm fn id
//   * `return_arm_by_handle` : span of the Handle expr -> return arm fn id
//
// Limitations of this TODO:
//   * Arm bodies must not capture outer-scope variables (no closure conversion).
//   * `resume(v)` must be the tail (last) expression of the arm body.
//     The codegen lowers `Expr::Resume(v)` to `Ret(v)`.
//   * One handler per (effect, op) per module (monomorphic dispatch).
//
// These limits are enough to demonstrate end-to-end effect handlers with a
// simple tail-resume test. Removing them needs closure conversion and the heap-
// cell continuation protocol — out of scope for now.

use crate::ast::*;
use std::collections::HashMap;

pub struct HandleLowering {
    /// (effect_name, op_name) -> fn name of the lifted arm.
    pub effect_op_to_arm: HashMap<(String, String), String>,
    /// Span of the Handle expression -> fn name of its return arm.
    pub return_arm_by_handle: HashMap<Span, String>,
    /// Synthetic FnDecls generated for arms. Appended to the module's decls.
    pub synthetic_fns: Vec<FnDecl>,
    next_id: usize,
}

impl HandleLowering {
    pub fn new() -> Self {
        Self {
            effect_op_to_arm: HashMap::new(),
            return_arm_by_handle: HashMap::new(),
            synthetic_fns: Vec::new(),
            next_id: 0,
        }
    }

    /// Walk every top-level FnDecl and lower its Handle expressions.
    /// Also collects effect-op signatures from Decl::Effect so arm params
    /// receive the right types.
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
                self.walk_block(&fn_decl.body, &op_sigs);
            }
        }
    }

    fn walk_block(&mut self, block: &Block, op_sigs: &HashMap<(String, String), FnSignature>) {
        for stmt in &block.stmts {
            self.walk_stmt(stmt, op_sigs);
        }
        if let Some(ret) = &block.ret {
            self.walk_expr(ret, op_sigs);
        }
    }

    fn walk_stmt(&mut self, stmt: &Spanned<Stmt>, op_sigs: &HashMap<(String, String), FnSignature>) {
        match &stmt.node {
            Stmt::Let { value, .. } => self.walk_expr(value, op_sigs),
            Stmt::Expr(e) => self.walk_expr(e, op_sigs),
            Stmt::Empty => {}
        }
    }

    fn walk_expr(&mut self, expr: &Spanned<Expr>, op_sigs: &HashMap<(String, String), FnSignature>) {
        match &expr.node {
            Expr::Handle { expr: body, arms } => {
                self.walk_expr(body, op_sigs);
                self.lift_arms(expr.span, arms, op_sigs);
                for arm in arms {
                    self.walk_expr(&arm.body, op_sigs);
                }
            }
            // Recurse into containing forms.
            Expr::Binary { left, right, .. } => {
                self.walk_expr(left, op_sigs);
                self.walk_expr(right, op_sigs);
            }
            Expr::Unary { right, .. } => self.walk_expr(right, op_sigs),
            Expr::Call { callee, args } => {
                self.walk_expr(callee, op_sigs);
                for a in args { self.walk_expr(a, op_sigs); }
            }
            Expr::If { condition, consequence, alternative } => {
                self.walk_expr(condition, op_sigs);
                self.walk_expr(consequence, op_sigs);
                if let Some(a) = alternative { self.walk_expr(a, op_sigs); }
            }
            Expr::Match { scrutinee, arms } => {
                self.walk_expr(scrutinee, op_sigs);
                for a in arms { self.walk_expr(&a.body, op_sigs); }
            }
            Expr::Block(b) => self.walk_block(b, op_sigs),
            Expr::While { condition, body } => {
                self.walk_expr(condition, op_sigs);
                self.walk_block(body, op_sigs);
            }
            Expr::For { iter, body, .. } => {
                self.walk_expr(iter, op_sigs);
                self.walk_block(body, op_sigs);
            }
            Expr::Loop { body } => self.walk_block(body, op_sigs),
            Expr::Region { body, .. } => self.walk_block(body, op_sigs),
            Expr::Return(Some(e)) | Expr::Throw(e) | Expr::Question(e) => self.walk_expr(e, op_sigs),
            Expr::Resume(Some(e)) => self.walk_expr(e, op_sigs),
            Expr::Index { base, index } => {
                self.walk_expr(base, op_sigs);
                self.walk_expr(index, op_sigs);
            }
            Expr::FieldAccess { base, .. } => self.walk_expr(base, op_sigs),
            Expr::Tuple(elems) | Expr::Array(elems) => {
                for e in elems { self.walk_expr(e, op_sigs); }
            }
            Expr::ArrayRepeat { elem, count } => {
                self.walk_expr(elem, op_sigs);
                self.walk_expr(count, op_sigs);
            }
            _ => {}
        }
    }

    fn lift_arms(
        &mut self,
        handle_span: Span,
        arms: &[HandleArm],
        op_sigs: &HashMap<(String, String), FnSignature>,
    ) {
        for arm in arms {
            match &arm.kind {
                HandleArmKind::Return => {
                    let fn_name = format!("__handle_return_{}", self.next_id);
                    self.next_id += 1;
                    let param = build_param(arm.pattern.as_ref(), None);
                    let body = Block {
                        stmts: vec![],
                        ret: Some(Box::new(arm.body.clone())),
                    };
                    self.synthetic_fns.push(FnDecl {
                        attrs: vec![],
                        is_pub: false,
                        name: fn_name.clone(),
                        generics: vec![],
                        params: param.into_iter().collect(),
                        effects: vec![],
                        return_type: None,
                        where_clause: vec![],
                        body,
                    });
                    self.return_arm_by_handle.insert(handle_span, fn_name);
                }
                HandleArmKind::Effect(path) => {
                    // path is the qualified op name, e.g. ["logger", "note"].
                    if path.len() < 2 {
                        // Not a valid effect.op reference; skip silently.
                        continue;
                    }
                    let effect_name = path[..path.len()-1].join(".");
                    let op_name = path.last().unwrap().clone();
                    let fn_name = format!("__handle_op_{}_{}_{}", effect_name, op_name, self.next_id);
                    self.next_id += 1;

                    // Use op signature to type the parameter(s).
                    let key = (effect_name.clone(), op_name.clone());
                    let param_ty = op_sigs.get(&key).and_then(|sig| {
                        sig.params.iter().find_map(|p| match p {
                            Param::Named { ty, .. } => Some(ty.clone()),
                            _ => None,
                        })
                    });
                    let param = build_param(arm.pattern.as_ref(), param_ty);

                    let body = Block {
                        stmts: vec![],
                        ret: Some(Box::new(arm.body.clone())),
                    };
                    self.synthetic_fns.push(FnDecl {
                        attrs: vec![],
                        is_pub: false,
                        name: fn_name.clone(),
                        generics: vec![],
                        params: param.into_iter().collect(),
                        effects: vec![],
                        return_type: None,
                        where_clause: vec![],
                        body,
                    });
                    self.effect_op_to_arm.insert(key, fn_name);
                }
                HandleArmKind::Exn => {
                    // exn handling already lowers through the Result variant
                    // protocol; nothing to lift here.
                }
            }
        }
    }
}

/// Build a single Named param from an optional arm pattern.
/// Falls back to an underscore-style binder when there is no pattern.
fn build_param(pat: Option<&Spanned<Pattern>>, ty: Option<Type>) -> Option<Param> {
    let ty = ty.unwrap_or(Type::Named("Int".into()));
    match pat {
        Some(p) => {
            if let Pattern::Bind(name) = &p.node {
                Some(Param::Named {
                    pattern: Spanned { node: Pattern::Bind(name.clone()), span: p.span },
                    ty,
                })
            } else {
                // Patterns more complex than a simple bind aren't supported in
                // arm heads yet; treat as anonymous.
                Some(Param::Named {
                    pattern: Spanned { node: Pattern::Bind("_arg".into()), span: p.span },
                    ty,
                })
            }
        }
        None => None,
    }
}
