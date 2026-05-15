// Unary and binary expressions.

use crate::ast;
use crate::bytecode::{OpCode, Register};
use crate::compiler::Compiler;
use crate::vm::Value;

impl Compiler {
    pub(in crate::compiler) fn compile_unary(
        &mut self,
        op: &ast::UnaryOp,
        right: &ast::Spanned<ast::Expr>,
    ) -> Result<Register, String> {
        match op {
            ast::UnaryOp::Ref | ast::UnaryOp::RefMut => {
                let src = self.compile_expr(right)?;
                let dest = self.alloc_register()?;
                self.emit(OpCode::Ref(dest, src));
                Ok(dest)
            }
            ast::UnaryOp::Deref => {
                let src = self.compile_expr(right)?;
                let dest = self.alloc_register()?;
                self.emit(OpCode::Ld(dest, src, 0));
                Ok(dest)
            }
            ast::UnaryOp::Neg => {
                if let ast::Expr::Literal(ast::Literal::Int(n)) = &right.node {
                    let reg = self.alloc_register()?;
                    let idx = self.add_constant(Value::Int(-n))?;
                    self.emit(OpCode::PushConst(reg, idx));
                    return Ok(reg);
                }
                if let ast::Expr::Literal(ast::Literal::Float(f)) = &right.node {
                    let reg = self.alloc_register()?;
                    let idx = self.add_constant(Value::Float(-f))?;
                    self.emit(OpCode::PushConst(reg, idx));
                    return Ok(reg);
                }
                let src = self.compile_expr(right)?;
                let dest = self.alloc_register()?;
                self.emit(OpCode::Neg(dest, src));
                Ok(dest)
            }
            ast::UnaryOp::Not => {
                let src = self.compile_expr(right)?;
                let zero = self.alloc_register()?;
                let idx = self.add_constant(Value::Bool(false))?;
                self.emit(OpCode::PushConst(zero, idx));
                let dest = self.alloc_register()?;
                self.emit(OpCode::Eq(dest, src, zero));
                Ok(dest)
            }
        }
    }

    pub(in crate::compiler) fn compile_binary(
        &mut self,
        op: &ast::BinaryOp,
        left: &ast::Spanned<ast::Expr>,
        right: &ast::Spanned<ast::Expr>,
    ) -> Result<Register, String> {
        match op {
            ast::BinaryOp::Assign => {
                if let ast::Expr::Identifier(name) = &left.node {
                    let rr = self.compile_expr(right)?;
                    let dest_reg = self.var_to_reg.get(name).copied()
                        .ok_or_else(|| format!("Undefined variable: {}", name))?;
                    self.emit(OpCode::Copy(dest_reg, rr));
                    Ok(dest_reg)
                } else {
                    Err("Assignment target must be a variable".to_string())
                }
            }
            _ => {
                let lr = self.compile_expr(left)?;
                let rr = self.compile_expr(right)?;
                let dr = self.alloc_register()?;
                let instr = match op {
                    ast::BinaryOp::Add => OpCode::Add(dr, lr, rr),
                    ast::BinaryOp::Sub => OpCode::Sub(dr, lr, rr),
                    ast::BinaryOp::Mul => OpCode::Mul(dr, lr, rr),
                    ast::BinaryOp::Div => OpCode::Div(dr, lr, rr),
                    ast::BinaryOp::Mod => OpCode::Mod(dr, lr, rr),
                    ast::BinaryOp::Eq => OpCode::Eq(dr, lr, rr),
                    ast::BinaryOp::Neq => OpCode::Neq(dr, lr, rr),
                    ast::BinaryOp::Lt => OpCode::Lt(dr, lr, rr),
                    ast::BinaryOp::Gt => OpCode::Gt(dr, lr, rr),
                    ast::BinaryOp::Lte => OpCode::Lte(dr, lr, rr),
                    ast::BinaryOp::Gte => OpCode::Gte(dr, lr, rr),
                    _ => return Err(format!("Unsupported binary op: {:?}", op)),
                };
                self.emit(instr);
                Ok(dr)
            }
        }
    }
}
