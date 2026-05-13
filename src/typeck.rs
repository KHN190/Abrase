// src/typeck.rs

use std::collections::HashMap;
use crate::ast;
use crate::ast::{Span, Spanned};
use crate::ty::{Ownership, Type};

fn elem_name(ty: &ast::Type) -> String {
    match ty {
        ast::Type::Named(n) => n.clone(),
        _ => "?".into(),
    }
}

#[derive(Debug, Clone)]
pub struct TypeError {
    pub message: String,
    pub span: Span,
    pub context: Vec<String>,
}

impl TypeError {
    pub fn display(&self) -> String {
        let mut output = format!("TypeError at line {}, col {}: {}", self.span.line, self.span.col, self.message);
        if !self.context.is_empty() {
            output.push_str("\n  Context stack:");
            for (i, ctx) in self.context.iter().enumerate() {
                output.push_str(&format!("\n    {}: {}", i + 1, ctx));
            }
        }
        output
    }
}

#[derive(Clone)]
struct VarMeta {
    ty: Type,
    is_mut: bool,
    is_moved: bool,
    defined_at: Span,
    moved_at: Option<Span>,
    // Phase 7: Ownership & Borrowing
    immut_borrow_count: usize, // count of active immutable borrows
    mut_borrow_active: bool,   // whether a mutable borrow is active
}

#[derive(Clone)]
pub struct Scope {
    vars: HashMap<String, VarMeta>,
}

pub struct Checker {
    scopes: Vec<Scope>,
    pub errors: Vec<TypeError>,
    context_stack: Vec<String>,
    loop_depth: usize,
    in_function: bool,
    fn_return_type: Option<crate::ty::Type>,
    active_effects: Vec<String>, // track active effect handlers
    effect_stack: Vec<Vec<String>>, // stack of effect scopes for region/scope

    // Type Environment (Phase 5)
    fn_registry: HashMap<String, (Vec<Type>, Type)>, // name -> (params, return_type)
    type_registry: HashMap<String, ast::TypeBody>, // name -> type definition
    const_registry: HashMap<String, Type>, // name -> const type

    // Phase 7: Ownership & Borrowing
    borrow_stack: Vec<(String, bool)>, // stack of (var_name, is_mutable)

    // Phase 8: Effects System
    effect_registry: HashMap<String, Vec<String>>, // effect_name -> list of operations
    effect_alias_registry: HashMap<String, Vec<crate::ty::Effect>>, // alias_name -> effects
    current_effects: Vec<crate::ty::Effect>, // effects in current function context
}

impl Checker {
    pub fn new() -> Self {
        Self {
            scopes: vec![Scope { vars: HashMap::new() }],
            errors: Vec::new(),
            context_stack: Vec::new(),
            loop_depth: 0,
            in_function: false,
            fn_return_type: None,
            active_effects: Vec::new(),
            effect_stack: vec![Vec::new()],
            fn_registry: HashMap::new(),
            type_registry: HashMap::new(),
            const_registry: HashMap::new(),
            borrow_stack: Vec::new(),
            effect_registry: HashMap::new(),
            effect_alias_registry: HashMap::new(),
            current_effects: Vec::new(),
        }
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(Scope { vars: HashMap::new() });
    }

    pub fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn display_errors(&self) -> String {
        if self.errors.is_empty() {
            return "No type errors".to_string();
        }
        let mut output = format!("Found {} type error(s):\n", self.errors.len());
        for (i, error) in self.errors.iter().enumerate() {
            output.push_str(&format!("\n{}: {}", i + 1, error.display()));
        }
        output
    }

    pub fn insert_var(&mut self, name: String, ty: Type, is_mut: bool, defined_at: Span) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.vars.insert(name, VarMeta {
                ty,
                is_mut,
                is_moved: false,
                defined_at,
                moved_at: None,
                immut_borrow_count: 0,
                mut_borrow_active: false,
            });
        }
    }

    // Phase 5: Type Environment Management
    pub fn register_function(&mut self, name: String, params: Vec<Type>, ret: Type) {
        self.fn_registry.insert(name, (params, ret));
    }

    pub fn get_function(&self, name: &str) -> Option<(Vec<Type>, Type)> {
        self.fn_registry.get(name).cloned()
    }

    pub fn register_type(&mut self, name: String, body: ast::TypeBody) {
        self.type_registry.insert(name, body);
    }

    pub fn get_type(&self, name: &str) -> Option<ast::TypeBody> {
        self.type_registry.get(name).cloned()
    }

    pub fn register_const(&mut self, name: String, ty: Type) {
        self.const_registry.insert(name, ty);
    }

    pub fn get_const(&self, name: &str) -> Option<Type> {
        self.const_registry.get(name).cloned()
    }

    // Phase 7: Ownership & Borrowing
    pub fn try_immut_borrow(&mut self, var_name: &str, _borrow_span: Span) -> Result<(), String> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(meta) = scope.vars.get_mut(var_name) {
                if meta.mut_borrow_active {
                    return Err(format!("Cannot immutably borrow '{}': mutable borrow already active", var_name));
                }
                meta.immut_borrow_count += 1;
                self.borrow_stack.push((var_name.to_string(), false));
                return Ok(());
            }
        }
        Err(format!("Variable '{}' not found", var_name))
    }

    pub fn try_mut_borrow(&mut self, var_name: &str, _borrow_span: Span) -> Result<(), String> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(meta) = scope.vars.get_mut(var_name) {
                if meta.immut_borrow_count > 0 {
                    return Err(format!("Cannot mutably borrow '{}': immutable borrow already active", var_name));
                }
                if meta.mut_borrow_active {
                    return Err(format!("Cannot mutably borrow '{}': mutable borrow already active", var_name));
                }
                if !meta.is_mut {
                    return Err(format!("Cannot mutably borrow immutable variable '{}'", var_name));
                }
                meta.mut_borrow_active = true;
                self.borrow_stack.push((var_name.to_string(), true));
                return Ok(());
            }
        }
        Err(format!("Variable '{}' not found", var_name))
    }

    pub fn release_borrow(&mut self, var_name: &str) {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(meta) = scope.vars.get_mut(var_name) {
                if meta.immut_borrow_count > 0 {
                    meta.immut_borrow_count = meta.immut_borrow_count.saturating_sub(1);
                }
                if meta.mut_borrow_active && self.borrow_stack.last().map_or(false, |(name, _)| name == var_name) {
                    meta.mut_borrow_active = false;
                }
                return;
            }
        }
    }

    pub fn check_ownership(&self, ty: &Type) -> Ownership {
        ty.ownership()
    }

    // Phase 8: Effects System
    pub fn register_effect(&mut self, name: String, operations: Vec<String>) {
        self.effect_registry.insert(name, operations);
    }

    pub fn get_effect(&self, name: &str) -> Option<Vec<String>> {
        self.effect_registry.get(name).cloned()
    }

    pub fn register_effect_alias(&mut self, alias_name: String, effects: Vec<crate::ty::Effect>) {
        self.effect_alias_registry.insert(alias_name, effects);
    }

    pub fn get_effect_alias(&self, alias_name: &str) -> Option<Vec<crate::ty::Effect>> {
        self.effect_alias_registry.get(alias_name).cloned()
    }

    pub fn push_effect(&mut self, effect: crate::ty::Effect) {
        self.current_effects.push(effect);
    }

    pub fn pop_effect(&mut self) {
        self.current_effects.pop();
    }

    pub fn effects_compatible(&self, expected: &[crate::ty::Effect], actual: &[crate::ty::Effect]) -> bool {
        // All expected effects must be present in actual effects
        expected.iter().all(|exp_effect| {
            actual.iter().any(|act_effect| self.effects_equal(exp_effect, act_effect))
        })
    }

    pub fn effects_equal(&self, e1: &crate::ty::Effect, e2: &crate::ty::Effect) -> bool {
        match (e1, e2) {
            (crate::ty::Effect::Total, crate::ty::Effect::Total) => true,
            (crate::ty::Effect::Async, crate::ty::Effect::Async) => true,
            (crate::ty::Effect::Alloc, crate::ty::Effect::Alloc) => true,
            (crate::ty::Effect::Nondet, crate::ty::Effect::Nondet) => true,
            (crate::ty::Effect::Exn(t1), crate::ty::Effect::Exn(t2)) => t1 == t2,
            _ => false,
        }
    }

    pub fn convert_effect(&self, eff: &ast::EffectItem) -> Option<crate::ty::Effect> {
        let name = eff.name.join(".").to_lowercase();
        match name.as_str() {
            "io" | "alloc" => Some(crate::ty::Effect::Alloc),
            "async" => Some(crate::ty::Effect::Async),
            "exn" => {
                if let Some(arg) = &eff.arg {
                    Some(crate::ty::Effect::Exn(Box::new(self.convert_type(arg))))
                } else {
                    Some(crate::ty::Effect::Exn(Box::new(Type::Named("Exception".into()))))
                }
            },
            "nondet" => Some(crate::ty::Effect::Nondet),
            _ => self.get_effect_alias(&name).and_then(|mut effs| effs.pop()),
        }
    }

    // Phase 5: Pattern Matching Support
    pub fn check_pattern(&mut self, pattern: &Spanned<ast::Pattern>, value_ty: &Type, _pattern_span: Span) {
        match &pattern.node {
            ast::Pattern::Bind(name) => {
                // Bind pattern: extract variable
                self.insert_var(name.clone(), value_ty.clone(), false, pattern.span);
            }
            ast::Pattern::Wildcard => {
                // Wildcard: accept any type
            }
            ast::Pattern::Literal(lit) => {
                // Literal pattern: verify value matches literal type
                let lit_ty = match lit {
                    ast::Literal::Int(_) => Type::Int,
                    ast::Literal::Float(_) => Type::Float,
                    ast::Literal::Bool(_) => Type::Bool,
                    ast::Literal::Char(_) => Type::Char,
                    ast::Literal::String(_) | ast::Literal::StringInterp(_) => Type::String,
                    ast::Literal::Unit => Type::Unit,
                };
                if *value_ty != lit_ty && *value_ty != Type::Unknown {
                    self.report_error(
                        format!("Pattern type mismatch: expected {:?}, found {:?}", lit_ty, value_ty),
                        pattern.span
                    );
                }
            }
            ast::Pattern::Tuple(pats) => {
                // Tuple pattern: recursively check nested patterns
                if let Type::Tuple(elem_types) = value_ty {
                    if pats.len() != elem_types.len() {
                        self.report_error(
                            format!("Tuple pattern length mismatch: expected {}, got {}",
                                elem_types.len(), pats.len()),
                            pattern.span
                        );
                    }
                    for (pat, elem_ty) in pats.iter().zip(elem_types.iter()) {
                        self.check_pattern(pat, elem_ty, pat.span);
                    }
                } else {
                    self.report_error(
                        format!("Expected tuple pattern, got {:?}", value_ty),
                        pattern.span
                    );
                }
            }
            ast::Pattern::Or(pats) => {
                // Or pattern: all branches must be compatible
                for pat in pats {
                    self.check_pattern(pat, value_ty, pat.span);
                }
            }
            ast::Pattern::Range { start: _, end: _, inclusive: _ } => {
                // Range pattern: start and end must be comparable
                if *value_ty != Type::Int && *value_ty != Type::Unknown {
                    self.report_error(
                        format!("Range pattern requires Int, got {:?}", value_ty),
                        pattern.span
                    );
                }
            }
            ast::Pattern::Array(pats) => {
                // Array pattern: all elements must match element type
                match value_ty {
                    Type::Named(name) if name.starts_with("Array") => {
                        for pat in pats {
                            self.check_pattern(pat, &Type::Unknown, pat.span);
                        }
                    }
                    _ => {
                        self.report_error(
                            format!("Expected array pattern, got {:?}", value_ty),
                            pattern.span
                        );
                    }
                }
            }
            ast::Pattern::Record { ty: _, fields, rest: _ } => {
                // Record pattern: verify fields exist (without type registry, skip validation)
                for field in fields {
                    if let Some(pat) = &field.pattern {
                        self.check_pattern(pat, &Type::Unknown, pat.span);
                    }
                }
            }
            ast::Pattern::Variant { ty: _, args } => {
                // Variant pattern: verify variant args (without ADT registry, skip validation)
                for pat in args {
                    self.check_pattern(pat, &Type::Unknown, pat.span);
                }
            }
            ast::Pattern::Ref(pat) => {
                // Reference pattern: unwrap reference type
                if let Type::Reference { inner, .. } = value_ty {
                    self.check_pattern(pat, inner, pattern.span);
                } else {
                    self.report_error(
                        format!("Expected reference pattern, got {:?}", value_ty),
                        pattern.span
                    );
                }
            }
        }
    }

    fn report_error(&mut self, message: String, span: Span) -> Type {
        self.errors.push(TypeError {
            message,
            span,
            context: self.context_stack.clone(),
        });
        Type::Unknown
    }

    pub fn get_var(&mut self, name: &str, is_ref: bool, usage_span: Span) -> Type {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(meta) = scope.vars.get_mut(name) {
                if meta.is_moved {
                    let move_line = meta.moved_at.map_or(0, |s| s.line);
                    let msg = format!("Use of moved value '{}'. It was moved at line {}.", name, move_line);
                    return self.report_error(msg, usage_span);
                }
                if meta.ty.ownership() == Ownership::Move && !is_ref {
                    meta.is_moved = true;
                    meta.moved_at = Some(usage_span);
                }
                return meta.ty.clone();
            }
        }
        self.report_error(format!("Undefined variable: {}", name), usage_span)
    }

    pub fn convert_type(&self, ast_ty: &ast::Type) -> Type {
        match ast_ty {
            ast::Type::Named(name) => match name.as_str() {
                "Int" => Type::Int,
                "Float" => Type::Float,
                "Bool" => Type::Bool,
                "Char" => Type::Char,
                "String" => Type::String,
                "Unit" => Type::Unit,
                _ => Type::Named(name.clone()),
            },
            ast::Type::Qualified(parts) => Type::Named(parts.join(".")),
            ast::Type::Generic { name, args } => {
                // Phase 6: Better generic handling - preserve type arguments
                let arg_strs: Vec<String> = args.iter()
                    .map(|arg| format!("{:?}", self.convert_type(arg)))
                    .collect();
                Type::Named(format!("{}<{}>", name, arg_strs.join(", ")))
            },
            ast::Type::Array { elem, size } => {
                // Phase 6: Track array sizes
                Type::Named(format!("[{}; {}]", elem_name(elem), size))
            },
            ast::Type::Tuple(tys) => {
                Type::Tuple(tys.iter().map(|t| self.convert_type(t)).collect())
            }
            ast::Type::Reference { is_mut, inner, region } => Type::Reference {
                is_mut: *is_mut,
                inner: Box::new(self.convert_type(inner)),
            },
            ast::Type::Function { params, effects, ret } => {
                let converted_effects = effects.iter()
                    .filter_map(|eff| self.convert_effect(eff))
                    .collect();
                Type::Function {
                    params: params.iter().map(|t| self.convert_type(t)).collect(),
                    effects: converted_effects,
                    ret: Box::new(self.convert_type(ret)),
                }
            },
            ast::Type::DynTrait(name) => Type::Named(format!("dyn {}", name)),
        }
    }

    // Phase 6: Type Compatibility & Unification
    pub fn types_compatible(&self, expected: &Type, actual: &Type) -> bool {
        match (expected, actual) {
            // Same types are compatible
            (a, b) if a == b => true,
            // Unknown types are compatible with anything
            (Type::Unknown, _) | (_, Type::Unknown) => true,
            // Named types might be compatible via type definitions
            (Type::Named(exp_name), Type::Named(act_name)) => {
                exp_name == act_name || self.are_types_equivalent(exp_name, act_name)
            },
            // Tuples must have same length and compatible elements
            (Type::Tuple(exp_elems), Type::Tuple(act_elems)) => {
                exp_elems.len() == act_elems.len() &&
                exp_elems.iter().zip(act_elems.iter())
                    .all(|(e, a)| self.types_compatible(e, a))
            },
            // References with compatible inner types
            (Type::Reference { is_mut: e_mut, inner: e_inner },
             Type::Reference { is_mut: a_mut, inner: a_inner }) => {
                // Mutability must match exactly
                e_mut == a_mut && self.types_compatible(e_inner, a_inner)
            },
            // Function types with compatible signatures
            (Type::Function { params: e_params, ret: e_ret, .. },
             Type::Function { params: a_params, ret: a_ret, .. }) => {
                e_params.len() == a_params.len() &&
                e_params.iter().zip(a_params.iter())
                    .all(|(e, a)| self.types_compatible(e, a)) &&
                self.types_compatible(e_ret, a_ret)
            },
            // Different kinds are not compatible
            _ => false,
        }
    }

    // Check if two named types are equivalent (through type definitions)
    fn are_types_equivalent(&self, ty1: &str, ty2: &str) -> bool {
        // TODO: Phase 5 - Use type registry to check equivalence
        // For now, only exact name matches are equivalent
        ty1 == ty2
    }

    // Try to unify two types (for generics)
    pub fn unify_types(&self, ty1: &Type, ty2: &Type) -> Option<Type> {
        match (ty1, ty2) {
            // Same types unify to themselves
            (a, b) if a == b => Some(a.clone()),
            // Unknown unifies with anything
            (Type::Unknown, b) => Some(b.clone()),
            (a, Type::Unknown) => Some(a.clone()),
            // Tuples unify element-wise
            (Type::Tuple(elems1), Type::Tuple(elems2)) if elems1.len() == elems2.len() => {
                let unified: Option<Vec<_>> = elems1.iter().zip(elems2.iter())
                    .map(|(e1, e2)| self.unify_types(e1, e2))
                    .collect();
                unified.map(Type::Tuple)
            },
            // References unify if inner types unify and mutability matches
            (Type::Reference { is_mut: m1, inner: i1 },
             Type::Reference { is_mut: m2, inner: i2 }) if m1 == m2 => {
                self.unify_types(i1, i2).map(|inner| Type::Reference {
                    is_mut: *m1,
                    inner: Box::new(inner),
                })
            },
            // Function types unify if signatures match
            (Type::Function { params: p1, ret: r1, .. },
             Type::Function { params: p2, ret: r2, .. }) if p1.len() == p2.len() => {
                let unified_params: Option<Vec<_>> = p1.iter().zip(p2.iter())
                    .map(|(pp1, pp2)| self.unify_types(pp1, pp2))
                    .collect();
                let unified_ret = self.unify_types(r1, r2);
                match (unified_params, unified_ret) {
                    (Some(params), Some(ret)) => Some(Type::Function {
                        params,
                        effects: vec![],
                        ret: Box::new(ret),
                    }),
                    _ => None,
                }
            },
            // No unification possible
            _ => None,
        }
    }

    // Check if type can be assigned to expected type (subtyping)
    pub fn is_assignable(&self, expected: &Type, actual: &Type) -> bool {
        self.types_compatible(expected, actual)
    }

    pub fn check_stmt(&mut self, stmt: &Spanned<ast::Stmt>) {
        match &stmt.node {
            ast::Stmt::Let { pattern, is_mut: _, ty, value } => {
                let name = match &pattern.node {
                    ast::Pattern::Bind(n) => n.clone(),
                    _ => "<pattern>".into(),
                };
                self.context_stack.push(format!("In let binding for '{}'", name));
                let mut val_ty = self.infer_expr(value);

                if let Some(expected_ast_ty) = ty {
                    let expected_ty = self.convert_type(expected_ast_ty);
                    if expected_ty != val_ty && val_ty != Type::Unknown {
                        self.report_error(
                            format!("Type mismatch: expected {:?}, found {:?}", expected_ty, val_ty),
                            value.span
                        );
                    }
                    val_ty = expected_ty;
                }

                // Use check_pattern for comprehensive pattern matching
                self.check_pattern(pattern, &val_ty, pattern.span);
                self.context_stack.pop();
            }
            ast::Stmt::Expr(expr) => { self.infer_expr(expr); }
            ast::Stmt::Empty => {}
        }
    }

    pub fn infer_expr(&mut self, expr: &Spanned<ast::Expr>) -> Type {
        match &expr.node {
            ast::Expr::Error => Type::Unknown, // Silently propagate unknown to prevent cascades
            ast::Expr::Literal(lit) => match lit {
                ast::Literal::Int(_) => Type::Int,
                ast::Literal::Float(_) => Type::Float,
                ast::Literal::Bool(_) => Type::Bool,
                ast::Literal::Char(_) => Type::Char,
                ast::Literal::String(_) | ast::Literal::StringInterp(_) => Type::String,
                ast::Literal::Unit => Type::Unit,
            },
            ast::Expr::Identifier(name) => self.get_var(name, false, expr.span),
            ast::Expr::Unary { op, right } => {
                match op {
                    ast::UnaryOp::Ref => {
                        if let ast::Expr::Identifier(name) = &right.node {
                            match self.try_immut_borrow(name, expr.span) {
                                Ok(()) => {
                                    let ty = self.get_var(name, true, expr.span);
                                    Type::Reference { is_mut: false, inner: Box::new(ty) }
                                }
                                Err(msg) => self.report_error(msg, expr.span)
                            }
                        } else {
                            self.report_error("Cannot borrow temporary".into(), right.span)
                        }
                    }
                    ast::UnaryOp::RefMut => {
                        if let ast::Expr::Identifier(name) = &right.node {
                            match self.try_mut_borrow(name, expr.span) {
                                Ok(()) => {
                                    let ty = self.get_var(name, true, expr.span);
                                    Type::Reference { is_mut: true, inner: Box::new(ty) }
                                }
                                Err(msg) => self.report_error(msg, expr.span)
                            }
                        } else {
                            self.report_error("Cannot mutably borrow temporary".into(), right.span)
                        }
                    }
                    ast::UnaryOp::Not => {
                        let r_ty = self.infer_expr(right);
                        if r_ty == Type::Bool || r_ty == Type::Unknown { Type::Bool } else { self.report_error("Expected Bool".into(), right.span) }
                    }
                    ast::UnaryOp::Neg => {
                        let r_ty = self.infer_expr(right);
                        if r_ty == Type::Int || r_ty == Type::Float || r_ty == Type::Unknown { r_ty } else { self.report_error("Expected numeric".into(), right.span) }
                    }
                    ast::UnaryOp::Deref => {
                        let r_ty = self.infer_expr(right);
                        if let Type::Reference { inner, .. } = r_ty { *inner } else { self.report_error("Expected reference".into(), right.span) }
                    }
                }
            }
            ast::Expr::Binary { op, left, right } => {
                self.context_stack.push("In binary expression".into());
                let l_ty = self.infer_expr(left);
                let r_ty = self.infer_expr(right);

                let result = if l_ty == Type::Unknown || r_ty == Type::Unknown {
                    Type::Unknown
                } else if l_ty != r_ty {
                    self.report_error(format!("Type mismatch: expected {:?}, found {:?}", l_ty, r_ty), right.span)
                } else {
                    match op {
                        ast::BinaryOp::Add | ast::BinaryOp::Sub | ast::BinaryOp::Mul | ast::BinaryOp::Div | ast::BinaryOp::Mod => {
                            if l_ty == Type::Int || l_ty == Type::Float { l_ty } else { self.report_error("Expected numeric types".into(), expr.span) }
                        }
                        ast::BinaryOp::Eq | ast::BinaryOp::Neq | ast::BinaryOp::Lt | ast::BinaryOp::Gt | ast::BinaryOp::Lte | ast::BinaryOp::Gte => {
                            Type::Bool
                        }
                        ast::BinaryOp::And | ast::BinaryOp::Or => {
                            if l_ty == Type::Bool { Type::Bool } else { self.report_error("Expected Bool".into(), expr.span) }
                        }
                        ast::BinaryOp::Assign
                        | ast::BinaryOp::AddAssign | ast::BinaryOp::SubAssign
                        | ast::BinaryOp::MulAssign | ast::BinaryOp::DivAssign
                        | ast::BinaryOp::ModAssign => {
                            Type::Unit
                        }
                    }
                };
                self.context_stack.pop();
                result
            }
            ast::Expr::Block(block) => {
                self.enter_scope();
                for stmt in &block.stmts {
                    self.check_stmt(stmt);
                }
                let ty = if let Some(ret_expr) = &block.ret {
                    self.infer_expr(ret_expr)
                } else {
                    Type::Unit
                };
                self.exit_scope();
                ty
            }
            ast::Expr::If { condition, consequence, alternative } => {
                self.context_stack.push("In if condition".into());
                let cond_ty = self.infer_expr(condition);
                self.context_stack.pop();
                
                if cond_ty != Type::Bool && cond_ty != Type::Unknown {
                    self.report_error("Condition must be Bool".into(), condition.span);
                }
                
                let cons_ty = self.infer_expr(consequence);
                if let Some(alt) = alternative {
                    let alt_ty = self.infer_expr(alt);
                    if cons_ty != alt_ty && cons_ty != Type::Unknown && alt_ty != Type::Unknown {
                        self.report_error("If branch types do not match".into(), alt.span);
                    }
                }
                cons_ty
            }
            ast::Expr::Match { scrutinee, arms } => {
                self.context_stack.push("In match expression".into());
                let _scrutinee_ty = self.infer_expr(scrutinee);

                let mut arm_types = Vec::new();
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        let guard_ty = self.infer_expr(guard);
                        if guard_ty != Type::Bool && guard_ty != Type::Unknown {
                            self.report_error("Guard must be Bool".into(), guard.span);
                        }
                    }
                    let body_ty = self.infer_expr(&arm.body);
                    arm_types.push(body_ty);
                }

                // All arms must have same type
                if !arm_types.is_empty() {
                    let first = arm_types[0].clone();
                    for ty in arm_types.iter().skip(1) {
                        if *ty != first && first != Type::Unknown && *ty != Type::Unknown {
                            self.report_error("Match arm types do not match".into(), expr.span);
                        }
                    }
                }

                self.context_stack.pop();
                if arm_types.is_empty() { Type::Unknown } else { arm_types[0].clone() }
            }
            ast::Expr::For { pattern, iter, body } => {
                self.context_stack.push("In for loop".into());
                let _iter_ty = self.infer_expr(iter);

                self.enter_scope();
                self.loop_depth += 1;

                // Bind pattern variable
                if let ast::Pattern::Bind(name) = &pattern.node {
                    self.insert_var(name.clone(), Type::Unknown, false, pattern.span);
                }

                let body_ty = self.infer_block(body);

                self.loop_depth -= 1;
                self.exit_scope();
                self.context_stack.pop();
                body_ty
            }
            ast::Expr::While { condition, body } => {
                self.context_stack.push("In while loop".into());
                let cond_ty = self.infer_expr(condition);

                if cond_ty != Type::Bool && cond_ty != Type::Unknown {
                    self.report_error("While condition must be Bool".into(), condition.span);
                }

                self.loop_depth += 1;
                let body_ty = self.infer_block(body);
                self.loop_depth -= 1;

                self.context_stack.pop();
                body_ty
            }
            ast::Expr::Loop { body } => {
                self.context_stack.push("In loop".into());
                self.loop_depth += 1;
                let body_ty = self.infer_block(body);
                self.loop_depth -= 1;
                self.context_stack.pop();
                body_ty
            }
            ast::Expr::Break(_break_val) => {
                if self.loop_depth == 0 {
                    self.report_error("Break outside of loop".into(), expr.span);
                }
                Type::Never
            }
            ast::Expr::Continue => {
                if self.loop_depth == 0 {
                    self.report_error("Continue outside of loop".into(), expr.span);
                }
                Type::Never
            }
            ast::Expr::Return(ret_val) => {
                if let Some(val) = ret_val {
                    let _val_ty = self.infer_expr(val);
                }
                Type::Never
            }
            ast::Expr::Throw(expr_val) => {
                let _ex_ty = self.infer_expr(expr_val);
                Type::Never
            }
            // Phase 2: Complex Expressions
            ast::Expr::Call { callee, args } => {
                let callee_ty = self.infer_expr(callee);
                if let Type::Function { params, ret, .. } = callee_ty {
                    if args.len() != params.len() {
                        self.report_error(
                            format!("Expected {} arguments, got {}", params.len(), args.len()),
                            expr.span
                        );
                    }
                    for (i, (arg, param_ty)) in args.iter().zip(params.iter()).enumerate() {
                        let arg_ty = self.infer_expr(arg);
                        if arg_ty != *param_ty && arg_ty != Type::Unknown && *param_ty != Type::Unknown {
                            self.report_error(
                                format!("Argument {} type mismatch: expected {:?}, got {:?}", i, param_ty, arg_ty),
                                arg.span
                            );
                        }
                    }
                    *ret
                } else {
                    self.report_error("Callee must be function type".into(), callee.span)
                }
            }
            ast::Expr::Tuple(elems) => {
                let elem_types: Vec<_> = elems.iter().map(|e| self.infer_expr(e)).collect();
                Type::Tuple(elem_types)
            }
            ast::Expr::Array(elems) => {
                if elems.is_empty() {
                    Type::Named("Array<Unknown>".into())
                } else {
                    let first_ty = self.infer_expr(&elems[0]);
                    for elem in &elems[1..] {
                        let elem_ty = self.infer_expr(elem);
                        if elem_ty != first_ty && elem_ty != Type::Unknown && first_ty != Type::Unknown {
                            self.report_error("Array elements must have same type".into(), elem.span);
                        }
                    }
                    Type::Named(format!("Array<{:?}>", first_ty))
                }
            }
            ast::Expr::ArrayRepeat { elem, count } => {
                let _elem_ty = self.infer_expr(elem);
                let count_ty = self.infer_expr(count);
                if count_ty != Type::Int && count_ty != Type::Unknown {
                    self.report_error("Array repeat count must be Int".into(), count.span);
                }
                Type::Named("Array<Unknown>".into())
            }
            ast::Expr::Index { base, index } => {
                let base_ty = self.infer_expr(base);
                let index_ty = self.infer_expr(index);

                if index_ty != Type::Int && index_ty != Type::Unknown {
                    self.report_error("Index must be Int".into(), index.span);
                }

                match base_ty {
                    Type::Named(ref name) if name.starts_with("Array") => Type::Unknown,
                    Type::Tuple(ref elems) => {
                        if elems.is_empty() { Type::Unknown }
                        else { elems[0].clone() }
                    }
                    _ => self.report_error("Can only index arrays or tuples".into(), base.span),
                }
            }
            ast::Expr::FieldAccess { base, field: _ } => {
                let _base_ty = self.infer_expr(base);
                Type::Unknown // Would need record type registry
            }
            // Phase 3: Advanced Expressions
            ast::Expr::Closure { is_move: _, params, effects: _, ret_ty, body } => {
                self.enter_scope();
                for param in params {
                    if let Some(param_ty) = &param.ty {
                        let converted_ty = self.convert_type(param_ty);
                        self.insert_var(
                            match &param.pattern.node {
                                ast::Pattern::Bind(n) => n.clone(),
                                _ => "<param>".into(),
                            },
                            converted_ty,
                            false,
                            param.pattern.span
                        );
                    }
                }
                let body_ty = self.infer_expr(body);
                if let Some(expected_ret) = ret_ty {
                    let expected = self.convert_type(expected_ret);
                    if expected != body_ty && expected != Type::Unknown && body_ty != Type::Unknown {
                        self.report_error("Closure body type mismatch".into(), body.span);
                    }
                }
                self.exit_scope();
                Type::Named("Closure".into())
            }
            ast::Expr::Record { ty, fields } => {
                for field in fields {
                    if let Some(value) = &field.value {
                        let _field_ty = self.infer_expr(value);
                    }
                }
                Type::Named(ty.join("."))
            }
            ast::Expr::Variant { ty, args } => {
                for arg in args {
                    let _arg_ty = self.infer_expr(arg);
                }
                Type::Named(ty.join("."))
            }
            ast::Expr::Range { start, end, inclusive: _ } => {
                if let Some(s) = start {
                    let s_ty = self.infer_expr(s);
                    if s_ty != Type::Int && s_ty != Type::Unknown {
                        self.report_error("Range start must be Int".into(), s.span);
                    }
                }
                if let Some(e) = end {
                    let e_ty = self.infer_expr(e);
                    if e_ty != Type::Int && e_ty != Type::Unknown {
                        self.report_error("Range end must be Int".into(), e.span);
                    }
                }
                Type::Named("Range<Int>".into())
            }
            ast::Expr::Question(_inner) => {
                Type::Unknown // Error propagation, would need context
            }
            ast::Expr::Await(_inner) => {
                Type::Unknown // Async, would need effect tracking
            }
            ast::Expr::Scope { label, options: _, body } => {
                self.context_stack.push(format!("In scope{}",
                    label.as_ref().map(|l| format!(" '{}'", l)).unwrap_or_default()));

                // Push new scope effect context
                self.effect_stack.push(self.active_effects.clone());
                let body_ty = self.infer_block(body);
                // Pop scope effect context
                self.effect_stack.pop();

                self.context_stack.pop();
                body_ty
            }
            ast::Expr::Region { label, body } => {
                self.context_stack.push(format!("In region{}",
                    label.as_ref().map(|l| format!(" '{}'", l)).unwrap_or_default()));

                // Push new region effect context
                self.effect_stack.push(self.active_effects.clone());
                let body_ty = self.infer_block(body);
                // Pop region effect context
                self.effect_stack.pop();

                self.context_stack.pop();
                body_ty
            }
            ast::Expr::Handle { expr: handler_expr, arms } => {
                self.context_stack.push("In handle expression".into());

                let _expr_ty = self.infer_expr(handler_expr);

                let mut arm_types = Vec::new();
                for arm in arms {
                    // Type-check the arm body
                    let arm_ty = self.infer_expr(&arm.body);
                    arm_types.push(arm_ty);

                    // Validate arm pattern if present
                    if let Some(pat) = &arm.pattern {
                        match &pat.node {
                            ast::Pattern::Bind(name) => {
                                self.insert_var(name.clone(), Type::Unknown, false, pat.span);
                            }
                            _ => {}
                        }
                    }

                    // Register handled effect based on kind
                    match &arm.kind {
                        ast::HandleArmKind::Return => {
                            // Return handler doesn't remove an effect
                        }
                        ast::HandleArmKind::Exn => {
                            // Exception handler
                            if !self.active_effects.contains(&"exn".to_string()) {
                                self.report_error(
                                    "Handling exn but no exn effect is active".into(),
                                    expr.span
                                );
                            }
                        }
                        ast::HandleArmKind::Effect(effect_path) => {
                            let effect_name = effect_path.join(".");
                            if !self.active_effects.contains(&effect_name) {
                                self.report_error(
                                    format!("Handling effect {} but it is not active", effect_name),
                                    expr.span
                                );
                            }
                        }
                    }
                }

                // All arms must have same type
                if !arm_types.is_empty() {
                    let first = arm_types[0].clone();
                    for ty in arm_types.iter().skip(1) {
                        if *ty != first && first != Type::Unknown && *ty != Type::Unknown {
                            self.report_error("Handle arm types do not match".into(), expr.span);
                        }
                    }
                }

                self.context_stack.pop();
                if arm_types.is_empty() { Type::Unknown } else { arm_types[0].clone() }
            }
            _ => self.report_error("Expression not supported yet".into(), expr.span),
        }
    }

    fn infer_block(&mut self, block: &ast::Block) -> Type {
        self.enter_scope();
        for stmt in &block.stmts {
            self.check_stmt(stmt);
        }
        let ty = if let Some(ret_expr) = &block.ret {
            self.infer_expr(ret_expr)
        } else {
            Type::Unit
        };
        self.exit_scope();
        ty
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d_span() -> Span { Span::new(0, 0) }
    fn sp<T>(node: T) -> Spanned<T> { Spanned { node, span: d_span() } }

    #[test]
    fn verify_type_inference_primitives() {
        let mut checker = Checker::new();
        assert_eq!(checker.infer_expr(&sp(ast::Expr::Literal(ast::Literal::Int(42)))), Type::Int);
        assert_eq!(checker.infer_expr(&sp(ast::Expr::Literal(ast::Literal::Bool(true)))), Type::Bool);
        assert_eq!(checker.infer_expr(&sp(ast::Expr::Literal(ast::Literal::String("test".into())))), Type::String);
        assert_eq!(checker.infer_expr(&sp(ast::Expr::Literal(ast::Literal::Float(3.14)))), Type::Float);
        assert_eq!(checker.infer_expr(&sp(ast::Expr::Literal(ast::Literal::Char('a')))), Type::Char);
        assert_eq!(checker.infer_expr(&sp(ast::Expr::Literal(ast::Literal::Unit))), Type::Unit);
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn verify_binary_add_operations() {
        let mut checker = Checker::new();
        let expr = sp(ast::Expr::Binary {
            op: ast::BinaryOp::Add,
            left: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(10)))),
            right: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(20)))),
        });
        assert_eq!(checker.infer_expr(&expr), Type::Int);
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn verify_binary_float_operations() {
        let mut checker = Checker::new();
        let expr = sp(ast::Expr::Binary {
            op: ast::BinaryOp::Mul,
            left: Box::new(sp(ast::Expr::Literal(ast::Literal::Float(2.5)))),
            right: Box::new(sp(ast::Expr::Literal(ast::Literal::Float(3.0)))),
        });
        assert_eq!(checker.infer_expr(&expr), Type::Float);
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn verify_binary_type_mismatch_error() {
        let mut checker = Checker::new();
        let expr = sp(ast::Expr::Binary {
            op: ast::BinaryOp::Add,
            left: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(10)))),
            right: Box::new(sp(ast::Expr::Literal(ast::Literal::String("test".into())))),
        });
        let result = checker.infer_expr(&expr);
        assert_eq!(result, Type::Unknown);
        assert_eq!(checker.errors.len(), 1);
        assert!(checker.errors[0].message.contains("Type mismatch"), "Error: {}", checker.errors[0].message);
    }

    #[test]
    fn verify_comparison_operations_return_bool() {
        let mut checker = Checker::new();
        let eq_expr = sp(ast::Expr::Binary {
            op: ast::BinaryOp::Eq,
            left: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(5)))),
            right: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(5)))),
        });
        assert_eq!(checker.infer_expr(&eq_expr), Type::Bool);

        let mut checker = Checker::new();
        let lt_expr = sp(ast::Expr::Binary {
            op: ast::BinaryOp::Lt,
            left: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(3)))),
            right: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(7)))),
        });
        assert_eq!(checker.infer_expr(&lt_expr), Type::Bool);
    }

    #[test]
    fn verify_logical_operations() {
        let mut checker = Checker::new();
        let and_expr = sp(ast::Expr::Binary {
            op: ast::BinaryOp::And,
            left: Box::new(sp(ast::Expr::Literal(ast::Literal::Bool(true)))),
            right: Box::new(sp(ast::Expr::Literal(ast::Literal::Bool(false)))),
        });
        assert_eq!(checker.infer_expr(&and_expr), Type::Bool);
    }

    #[test]
    fn verify_logical_operation_type_error() {
        let mut checker = Checker::new();
        let and_expr = sp(ast::Expr::Binary {
            op: ast::BinaryOp::And,
            left: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(1)))),
            right: Box::new(sp(ast::Expr::Literal(ast::Literal::Bool(true)))),
        });
        let result = checker.infer_expr(&and_expr);
        assert_eq!(result, Type::Unknown);
        assert_eq!(checker.errors.len(), 1);
    }

    #[test]
    fn verify_unary_not_operation() {
        let mut checker = Checker::new();
        let not_expr = sp(ast::Expr::Unary {
            op: ast::UnaryOp::Not,
            right: Box::new(sp(ast::Expr::Literal(ast::Literal::Bool(true)))),
        });
        assert_eq!(checker.infer_expr(&not_expr), Type::Bool);
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn verify_unary_not_type_error() {
        let mut checker = Checker::new();
        let not_expr = sp(ast::Expr::Unary {
            op: ast::UnaryOp::Not,
            right: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(42)))),
        });
        let result = checker.infer_expr(&not_expr);
        assert_eq!(result, Type::Unknown);
        assert_eq!(checker.errors.len(), 1);
        assert!(checker.errors[0].message.contains("Expected Bool"));
    }

    #[test]
    fn verify_unary_negation() {
        let mut checker = Checker::new();
        let neg_expr = sp(ast::Expr::Unary {
            op: ast::UnaryOp::Neg,
            right: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(5)))),
        });
        assert_eq!(checker.infer_expr(&neg_expr), Type::Int);

        let mut checker = Checker::new();
        let neg_float = sp(ast::Expr::Unary {
            op: ast::UnaryOp::Neg,
            right: Box::new(sp(ast::Expr::Literal(ast::Literal::Float(3.14)))),
        });
        assert_eq!(checker.infer_expr(&neg_float), Type::Float);
    }

    #[test]
    fn verify_unary_negation_type_error() {
        let mut checker = Checker::new();
        let neg_expr = sp(ast::Expr::Unary {
            op: ast::UnaryOp::Neg,
            right: Box::new(sp(ast::Expr::Literal(ast::Literal::String("hello".into())))),
        });
        let result = checker.infer_expr(&neg_expr);
        assert_eq!(result, Type::Unknown);
        assert_eq!(checker.errors.len(), 1);
    }

    #[test]
    fn verify_reference_operation() {
        let mut checker = Checker::new();
        let ref_expr = sp(ast::Expr::Unary {
            op: ast::UnaryOp::Ref,
            right: Box::new(sp(ast::Expr::Identifier("x".into()))),
        });
        // Setup the variable first
        checker.insert_var("x".into(), Type::Int, false, d_span());
        let result = checker.infer_expr(&ref_expr);
        assert_eq!(result, Type::Reference { is_mut: false, inner: Box::new(Type::Int) });
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn verify_mutable_reference() {
        let mut checker = Checker::new();
        let ref_mut_expr = sp(ast::Expr::Unary {
            op: ast::UnaryOp::RefMut,
            right: Box::new(sp(ast::Expr::Identifier("y".into()))),
        });
        checker.insert_var("y".into(), Type::String, true, d_span());
        let result = checker.infer_expr(&ref_mut_expr);
        assert_eq!(result, Type::Reference { is_mut: true, inner: Box::new(Type::String) });
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn verify_reference_to_temporary_error() {
        let mut checker = Checker::new();
        let ref_expr = sp(ast::Expr::Unary {
            op: ast::UnaryOp::Ref,
            right: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(42)))),
        });
        let result = checker.infer_expr(&ref_expr);
        assert_eq!(result, Type::Unknown);
        assert_eq!(checker.errors.len(), 1);
        assert!(checker.errors[0].message.contains("Cannot borrow temporary"));
    }

    #[test]
    fn verify_dereference_operation() {
        let mut checker = Checker::new();
        let deref_expr = sp(ast::Expr::Unary {
            op: ast::UnaryOp::Deref,
            right: Box::new(sp(ast::Expr::Identifier("ptr".into()))),
        });
        checker.insert_var("ptr".into(), Type::Reference { is_mut: false, inner: Box::new(Type::Float) }, false, d_span());
        let result = checker.infer_expr(&deref_expr);
        assert_eq!(result, Type::Float);
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn verify_dereference_non_reference_error() {
        let mut checker = Checker::new();
        let deref_expr = sp(ast::Expr::Unary {
            op: ast::UnaryOp::Deref,
            right: Box::new(sp(ast::Expr::Literal(ast::Literal::Bool(true)))),
        });
        let result = checker.infer_expr(&deref_expr);
        assert_eq!(result, Type::Unknown);
        assert_eq!(checker.errors.len(), 1);
        assert!(checker.errors[0].message.contains("Expected reference"));
    }

    #[test]
    fn verify_undefined_variable_error() {
        let mut checker = Checker::new();
        let ident_expr = sp(ast::Expr::Identifier("undefined_var".into()));
        let result = checker.infer_expr(&ident_expr);
        assert_eq!(result, Type::Unknown);
        assert_eq!(checker.errors.len(), 1);
        assert!(checker.errors[0].message.contains("Undefined variable"));
    }

    #[test]
    fn verify_let_statement_with_type_annotation() {
        let mut checker = Checker::new();
        let let_stmt = sp(ast::Stmt::Let {
            pattern: sp(ast::Pattern::Bind("x".into())),
            is_mut: false,
            ty: Some(ast::Type::Named("Int".into())),
            value: sp(ast::Expr::Literal(ast::Literal::Int(42))),
        });
        checker.check_stmt(&let_stmt);
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn verify_let_statement_type_mismatch_error() {
        let mut checker = Checker::new();
        let let_stmt = sp(ast::Stmt::Let {
            pattern: sp(ast::Pattern::Bind("x".into())),
            is_mut: false,
            ty: Some(ast::Type::Named("Bool".into())),
            value: sp(ast::Expr::Literal(ast::Literal::Int(42))),
        });
        checker.check_stmt(&let_stmt);
        assert_eq!(checker.errors.len(), 1);
        assert!(checker.errors[0].message.contains("Type mismatch"));
    }

    #[test]
    fn verify_block_expression() {
        let mut checker = Checker::new();
        let block = ast::Block {
            stmts: vec![],
            ret: Some(Box::new(sp(ast::Expr::Literal(ast::Literal::Int(99))))),
        };
        let block_expr = sp(ast::Expr::Block(block));
        let result = checker.infer_expr(&block_expr);
        assert_eq!(result, Type::Int);
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn verify_block_with_statements() {
        let mut checker = Checker::new();
        let let_stmt = sp(ast::Stmt::Let {
            pattern: sp(ast::Pattern::Bind("x".into())),
            is_mut: true,
            ty: None,
            value: sp(ast::Expr::Literal(ast::Literal::Int(10))),
        });
        let block = ast::Block {
            stmts: vec![let_stmt],
            ret: Some(Box::new(sp(ast::Expr::Identifier("x".into())))),
        };
        let block_expr = sp(ast::Expr::Block(block));
        let result = checker.infer_expr(&block_expr);
        assert_eq!(result, Type::Int);
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn verify_if_expression_matching_branches() {
        let mut checker = Checker::new();
        let if_expr = sp(ast::Expr::If {
            condition: Box::new(sp(ast::Expr::Literal(ast::Literal::Bool(true)))),
            consequence: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(1)))),
            alternative: Some(Box::new(sp(ast::Expr::Literal(ast::Literal::Int(2))))),
        });
        let result = checker.infer_expr(&if_expr);
        assert_eq!(result, Type::Int);
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn verify_if_expression_branch_type_mismatch() {
        let mut checker = Checker::new();
        let if_expr = sp(ast::Expr::If {
            condition: Box::new(sp(ast::Expr::Literal(ast::Literal::Bool(true)))),
            consequence: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(1)))),
            alternative: Some(Box::new(sp(ast::Expr::Literal(ast::Literal::String("two".into()))))),
        });
        let result = checker.infer_expr(&if_expr);
        assert_eq!(result, Type::Int);
        assert_eq!(checker.errors.len(), 1);
        assert!(checker.errors[0].message.contains("branch types do not match"));
    }

    #[test]
    fn verify_if_condition_must_be_bool() {
        let mut checker = Checker::new();
        let if_expr = sp(ast::Expr::If {
            condition: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(42)))),
            consequence: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(1)))),
            alternative: None,
        });
        let result = checker.infer_expr(&if_expr);
        assert_eq!(result, Type::Int);
        assert_eq!(checker.errors.len(), 1);
        assert!(checker.errors[0].message.contains("Condition must be Bool"));
    }

    #[test]
    fn verify_borrow_checking_move_semantics() {
        let mut checker = Checker::new();

        let let_stmt = sp(ast::Stmt::Let {
            pattern: sp(ast::Pattern::Bind("text".into())),
            is_mut: false,
            ty: None,
            value: sp(ast::Expr::Literal(ast::Literal::String("hello".into()))),
        });
        checker.check_stmt(&let_stmt);

        let ref_expr = sp(ast::Expr::Unary {
            op: ast::UnaryOp::Ref,
            right: Box::new(sp(ast::Expr::Identifier("text".into()))),
        });
        let ref_ty = checker.infer_expr(&ref_expr);

        assert_eq!(ref_ty, Type::Reference { is_mut: false, inner: Box::new(Type::String) });
        assert!(checker.errors.is_empty(), "Borrowing should not cause an error or move");

        let move_expr = sp(ast::Expr::Identifier("text".into()));
        let _ = checker.infer_expr(&move_expr);
        assert!(checker.errors.is_empty(), "First move should be valid");

        // The second move should fail and push to errors
        let second_move = sp(ast::Expr::Identifier("text".into()));
        let _ = checker.infer_expr(&second_move);

        assert_eq!(checker.errors.len(), 1);
        assert!(checker.errors[0].message.contains("Use of moved value 'text'"), "Error message: {}", checker.errors[0].message);
    }

    #[test]
    fn verify_borrow_checking_copy_semantics() {
        let mut checker = Checker::new();

        let let_stmt = sp(ast::Stmt::Let {
            pattern: sp(ast::Pattern::Bind("num".into())),
            is_mut: false,
            ty: None,
            value: sp(ast::Expr::Literal(ast::Literal::Int(100))),
        });
        checker.check_stmt(&let_stmt);

        let use_one = sp(ast::Expr::Identifier("num".into()));
        assert_eq!(checker.infer_expr(&use_one), Type::Int);

        let use_two = sp(ast::Expr::Identifier("num".into()));
        assert_eq!(checker.infer_expr(&use_two), Type::Int);

        assert!(checker.errors.is_empty());
    }

    #[test]
    fn verify_error_context_stack() {
        let mut checker = Checker::new();
        let expr = sp(ast::Expr::Binary {
            op: ast::BinaryOp::Add,
            left: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(10)))),
            right: Box::new(sp(ast::Expr::Literal(ast::Literal::String("x".into())))),
        });
        checker.infer_expr(&expr);
        assert_eq!(checker.errors.len(), 1);
        assert_eq!(checker.errors[0].context.len(), 1);
        assert!(checker.errors[0].context[0].contains("binary"));
    }
}