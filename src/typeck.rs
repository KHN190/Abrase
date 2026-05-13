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
                moved_at: None 
            });
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
            ast::Type::Generic { name, .. } => Type::Named(name.clone()),
            ast::Type::Array { elem, .. } => Type::Named(format!("[{}]", elem_name(elem))),
            ast::Type::Tuple(tys) => {
                Type::Tuple(tys.iter().map(|t| self.convert_type(t)).collect())
            }
            ast::Type::Reference { is_mut, inner, .. } => Type::Reference {
                is_mut: *is_mut,
                inner: Box::new(self.convert_type(inner)),
            },
            ast::Type::Function { params, ret, .. } => Type::Function {
                params: params.iter().map(|t| self.convert_type(t)).collect(),
                effects: vec![],
                ret: Box::new(self.convert_type(ret)),
            },
            ast::Type::DynTrait(name) => Type::Named(format!("dyn {}", name)),
        }
    }

    pub fn check_stmt(&mut self, stmt: &Spanned<ast::Stmt>) {
        match &stmt.node {
            ast::Stmt::Let { pattern, is_mut, ty, value } => {
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

                self.insert_var(name, val_ty, *is_mut, stmt.span);
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
                            let ty = self.get_var(name, true, expr.span);
                            Type::Reference { is_mut: false, inner: Box::new(ty) }
                        } else {
                            self.report_error("Cannot borrow temporary".into(), right.span)
                        }
                    }
                    ast::UnaryOp::RefMut => {
                        if let ast::Expr::Identifier(name) = &right.node {
                            let ty = self.get_var(name, true, expr.span);
                            Type::Reference { is_mut: true, inner: Box::new(ty) }
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