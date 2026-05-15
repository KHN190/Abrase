// Codegen entry point.
//
// `compile_expr` is a thin dispatcher that routes each AST variant to a
// purpose-built compile_* method living in a sibling submodule. The
// statement and block lowerings are short enough to live alongside it.

pub mod scaffold;
pub mod inference;
pub mod data;
pub mod arith;
pub mod control;
pub mod match_arm;
pub mod calls;
pub mod effects;
pub mod closure_expr;

pub(in crate::compiler) use inference::is_move_type;

use crate::ast;
use crate::bytecode::{OpCode, Register};
use crate::compiler::Compiler;
use crate::vm::Value;

impl Compiler {
    pub(in crate::compiler) fn compile_expr(
        &mut self,
        expr: &ast::Spanned<ast::Expr>,
    ) -> Result<Register, String> {
        match &expr.node {
            ast::Expr::Error => Err(
                "Compilation aborted: parser error was not recovered; fix parser errors first"
                    .to_string()
            ),
            ast::Expr::Literal(lit)            => self.compile_literal(lit),
            ast::Expr::Identifier(name)        => self.compile_identifier(name),
            ast::Expr::Unary { op, right }     => self.compile_unary(op, right),
            ast::Expr::Binary { op, left, right } => self.compile_binary(op, left, right),
            ast::Expr::If { condition, consequence, alternative } => {
                self.compile_if(condition, consequence, alternative.as_deref())
            }
            ast::Expr::While { condition, body } => self.compile_while(condition, body),
            ast::Expr::Block(block)            => self.compile_block(block),
            ast::Expr::Match { scrutinee, arms } => self.compile_match(scrutinee, arms),
            ast::Expr::Call { callee, args }   => self.compile_call(callee, args),
            ast::Expr::Return(opt_expr)        => self.compile_return(opt_expr.as_deref()),
            ast::Expr::Throw(inner)            => self.compile_throw(inner),
            ast::Expr::Question(inner)         => self.compile_question(inner),
            ast::Expr::Record { ty, fields }   => self.compile_record(ty, fields),
            ast::Expr::Variant { ty, args }    => self.compile_variant_expr(ty, args),
            ast::Expr::FieldAccess { base, field } => self.compile_field_access(base, field),
            ast::Expr::Array(items)            => self.compile_array(items),
            ast::Expr::Index { base, index }   => self.compile_index(base, index),
            ast::Expr::Resume(arg)             => self.compile_resume(arg.as_deref()),
            ast::Expr::Handle { expr: body, arms: _ } => self.compile_handle(body, expr.span),
            ast::Expr::Closure { .. }          => self.compile_closure(expr.span),
            _ => Err(format!("Unsupported expression: {:?}", expr.node)),
        }
    }

    pub(in crate::compiler) fn compile_stmt(
        &mut self,
        stmt: &ast::Spanned<ast::Stmt>,
    ) -> Result<(), String> {
        match &stmt.node {
            ast::Stmt::Let { pattern, value, ty, .. } => {
                if let ast::Pattern::Bind(name) = &pattern.node {
                    let inferred_ty = ty.clone().or_else(|| self.infer_expr_type(value));
                    let src_reg = self.compile_expr(value)?;
                    let bound_reg = if let ast::Expr::Identifier(src_name) = &value.node {
                        let dest = self.alloc_register()?;
                        let is_move = inferred_ty.as_ref().map(is_move_type).unwrap_or(false);
                        if is_move {
                            self.emit(OpCode::Move(dest, src_reg));
                            self.var_to_reg.remove(src_name);
                            self.var_types.remove(src_name);
                        } else {
                            self.emit(OpCode::Copy(dest, src_reg));
                        }
                        dest
                    } else {
                        src_reg
                    };
                    self.var_to_reg.insert(name.clone(), bound_reg);
                    if let Some(t) = inferred_ty {
                        self.var_types.insert(name.clone(), t);
                    }
                    // If the RHS was a closure literal, remember which lifted
                    // fn this binding refers to so call sites `name(args)`
                    // can emit a direct call.
                    if let ast::Expr::Closure { .. } = &value.node {
                        if let Some(info) = self.closure_by_span.get(&value.span).cloned() {
                            self.closure_by_var.insert(name.clone(), info);
                        }
                    }
                    Ok(())
                } else {
                    Err("Only simple bindings supported".to_string())
                }
            }
            ast::Stmt::Expr(expr) => { self.compile_expr(expr)?; Ok(()) }
            ast::Stmt::Empty => Ok(()),
        }
    }

    pub(in crate::compiler) fn compile_block(
        &mut self,
        block: &ast::Block,
    ) -> Result<Register, String> {
        let pre_bindings: std::collections::HashSet<String> = self.var_to_reg.keys().cloned().collect();
        for stmt in &block.stmts {
            self.compile_stmt(stmt)?;
        }
        let result = if let Some(ret) = &block.ret {
            self.compile_expr(ret)?
        } else {
            let reg = self.alloc_register()?;
            let idx = self.add_constant(Value::Unit)?;
            self.emit(OpCode::PushConst(reg, idx));
            reg
        };
        let new_bindings: Vec<String> = self.var_to_reg.keys()
            .filter(|k| !pre_bindings.contains(*k))
            .cloned()
            .collect();
        for name in &new_bindings {
            if let (Some(reg), Some(ty)) = (self.var_to_reg.get(name).copied(), self.var_types.get(name)) {
                if is_move_type(ty) && reg != result {
                    self.emit(OpCode::Drop(reg));
                }
            }
            self.var_to_reg.remove(name);
            self.var_types.remove(name);
        }
        Ok(result)
    }

    pub(in crate::compiler) fn finalize_arg_patches(&mut self) -> Result<(), String> {
        let reg_count = self.next_reg as usize;
        for (pos, slot) in std::mem::take(&mut self.pending_arg_patches) {
            let dst_idx = reg_count + slot as usize;
            if dst_idx > u8::MAX as usize {
                return Err(format!("Arg-passing register index {} exceeds u8 range", dst_idx));
            }
            let dst = Register(dst_idx as u8);
            if let OpCode::Copy(_, src) = self.code[pos] {
                self.code[pos] = OpCode::Copy(dst, src);
            }
        }
        Ok(())
    }
}
