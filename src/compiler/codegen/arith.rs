// Unary and binary expressions.

use crate::ast;
use crate::bytecode::{OpCode, Register};
use crate::compiler::Compiler;
use crate::myriad::Value;

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
                    let idx = self.add_constant(Value::from_int(-n))?;
                    self.emit(OpCode::PushConst(reg, idx));
                    return Ok(reg);
                }
                if let ast::Expr::Literal(ast::Literal::Float(f)) = &right.node {
                    let reg = self.alloc_register()?;
                    let idx = self.add_constant(Value::from_float(-f))?;
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
                let idx = self.add_constant(Value::from_bool(false))?;
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
                    let dest_reg = match self.var_to_reg.get(name).copied() {
                        Some(r) => { self.emit(OpCode::Copy(r, rr)); r }
                        None => {
                            // Var was moved earlier; rebind into a fresh register.
                            let r = self.alloc_register()?;
                            self.emit(OpCode::Copy(r, rr));
                            self.var_to_reg.insert(name.clone(), r);
                            r
                        }
                    };
                    Ok(dest_reg)
                } else {
                    Err("Assignment target must be a variable".to_string())
                }
            }
            _ => {
                // Fuse `x ± i8-literal` into a single AddImm/SubImm.
                if matches!(op, ast::BinaryOp::Add | ast::BinaryOp::Sub) {
                    if let Some(imm) = lit_i8(&right.node) {
                        let lr = self.compile_expr(left)?;
                        let dr = self.alloc_register()?;
                        self.emit(match op {
                            ast::BinaryOp::Add => OpCode::AddImm(dr, lr, imm),
                            _                  => OpCode::SubImm(dr, lr, imm),
                        });
                        return Ok(dr);
                    }
                }
                // Add is commutative: also fuse `i8-literal + x`.
                if matches!(op, ast::BinaryOp::Add) {
                    if let Some(imm) = lit_i8(&left.node) {
                        let rr = self.compile_expr(right)?;
                        let dr = self.alloc_register()?;
                        self.emit(OpCode::AddImm(dr, rr, imm));
                        return Ok(dr);
                    }
                }
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

fn lit_i8(node: &ast::Expr) -> Option<i8> {
    if let ast::Expr::Literal(ast::Literal::Int(n)) = node {
        if (i8::MIN as i64..=i8::MAX as i64).contains(n) {
            return Some(*n as i8);
        }
    }
    None
}
