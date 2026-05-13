// src/typeck.rs

use std::collections::HashMap;
use crate::ast;
use crate::ast::{Span, Spanned};
use crate::ty::{Effect, Ownership, Type};

#[derive(Debug)]
pub struct TypeError {
    pub message: String,
    pub span: Span,
    pub context: Vec<String>,
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
}

impl Checker {
    pub fn new() -> Self {
        Self {
            scopes: vec![Scope { vars: HashMap::new() }],
            errors: Vec::new(),
            context_stack: Vec::new(),
        }
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(Scope { vars: HashMap::new() });
    }

    pub fn exit_scope(&mut self) {
        self.scopes.pop();
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
            ast::Type::Tuple(tys) => {
                Type::Tuple(tys.iter().map(|t| self.convert_type(t)).collect())
            }
            ast::Type::Reference { is_mut, inner } => Type::Reference {
                is_mut: *is_mut,
                inner: Box::new(self.convert_type(inner)),
            },
            ast::Type::Function { params, ret } => Type::Function {
                params: params.iter().map(|t| self.convert_type(t)).collect(),
                effects: vec![],
                ret: Box::new(self.convert_type(ret)),
            },
        }
    }

    pub fn check_stmt(&mut self, stmt: &Spanned<ast::Stmt>) {
        match &stmt.node {
            ast::Stmt::Let { name, is_mut, ty, value } => {
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
                
                self.insert_var(name.clone(), val_ty, *is_mut, stmt.span);
                self.context_stack.pop();
            }
            ast::Stmt::Expr(expr) => {
                self.infer_expr(expr);
            }
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
                ast::Literal::String(_) => Type::String,
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
                self.context_stack.pop();

                if l_ty == Type::Unknown || r_ty == Type::Unknown {
                    return Type::Unknown;
                }

                if l_ty != r_ty {
                    return self.report_error(format!("Type mismatch: expected {:?}, found {:?}", l_ty, r_ty), right.span);
                }

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
                    ast::BinaryOp::Assign => {
                        Type::Unit
                    }
                }
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
            _ => self.report_error("Expression not supported yet".into(), expr.span),
        }
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
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn verify_binary_operations() {
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
    fn verify_borrow_checking_move_semantics() {
        let mut checker = Checker::new();
        
        let let_stmt = sp(ast::Stmt::Let {
            name: "text".into(),
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
        assert!(checker.errors[0].message.contains("Use of moved value 'text'"));
    }

    #[test]
    fn verify_borrow_checking_copy_semantics() {
        let mut checker = Checker::new();
        
        let let_stmt = sp(ast::Stmt::Let {
            name: "num".into(),
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
}