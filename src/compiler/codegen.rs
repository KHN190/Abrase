use super::*;
use crate::ast;
use crate::bytecode::{OpCode, Register};
use crate::vm::Value;

impl Compiler {
    pub(super) fn alloc_register(&mut self) -> Result<Register, String> {
        if self.next_reg >= 255 {
            return Err("Register overflow".to_string());
        }
        let reg = Register(self.next_reg);
        self.next_reg += 1;
        Ok(reg)
    }

    pub(super) fn emit(&mut self, op: OpCode) {
        self.code.push(op);
    }

    pub(super) fn add_constant(&mut self, val: Value) -> usize {
        self.constants.push(val);
        self.constants.len() - 1
    }

    pub(super) fn compile_expr(&mut self, expr: &ast::Spanned<ast::Expr>) -> Result<Register, String> {
        match &expr.node {
            ast::Expr::Literal(lit) => {
                let reg = self.alloc_register()?;
                let val = match lit {
                    ast::Literal::Int(n)    => Value::Int(*n),
                    ast::Literal::Float(f)  => Value::Float(*f),
                    ast::Literal::Bool(b)   => Value::Bool(*b),
                    ast::Literal::String(s) => Value::String(s.clone()),
                    ast::Literal::Unit      => Value::Unit,
                    _ => return Err("Unsupported literal".to_string()),
                };
                let idx = self.add_constant(val);
                self.emit(OpCode::PushConst(reg, idx));
                Ok(reg)
            }

            ast::Expr::Identifier(name) => {
                self.var_to_reg.get(name).copied()
                    .ok_or_else(|| format!("Undefined variable: {}", name))
            }

            ast::Expr::Binary { op, left, right } => {
                let lr = self.compile_expr(left)?;
                let rr = self.compile_expr(right)?;
                let dr = self.alloc_register()?;
                let instr = match op {
                    ast::BinaryOp::Add => OpCode::Add(dr, lr, rr),
                    ast::BinaryOp::Sub => OpCode::Sub(dr, lr, rr),
                    ast::BinaryOp::Mul => OpCode::Mul(dr, lr, rr),
                    ast::BinaryOp::Div => OpCode::Div(dr, lr, rr),
                    ast::BinaryOp::Mod => OpCode::Mod(dr, lr, rr),
                    _ => return Err(format!("Unsupported binary op: {:?}", op)),
                };
                self.emit(instr);
                Ok(dr)
            }

            ast::Expr::Block(block) => self.compile_block(block),

            _ => Err(format!("Unsupported expression: {:?}", expr.node)),
        }
    }

    pub(super) fn compile_stmt(&mut self, stmt: &ast::Spanned<ast::Stmt>) -> Result<(), String> {
        match &stmt.node {
            ast::Stmt::Let { pattern, value, .. } => {
                if let ast::Pattern::Bind(name) = &pattern.node {
                    let reg = self.compile_expr(value)?;
                    self.var_to_reg.insert(name.clone(), reg);
                    Ok(())
                } else {
                    Err("Only simple bindings supported".to_string())
                }
            }
            ast::Stmt::Expr(expr) => { self.compile_expr(expr)?; Ok(()) }
            ast::Stmt::Empty => Ok(()),
        }
    }

    pub(super) fn compile_block(&mut self, block: &ast::Block) -> Result<Register, String> {
        for stmt in &block.stmts {
            self.compile_stmt(stmt)?;
        }
        if let Some(ret) = &block.ret {
            self.compile_expr(ret)
        } else {
            let reg = self.alloc_register()?;
            let idx = self.add_constant(Value::Unit);
            self.emit(OpCode::PushConst(reg, idx));
            Ok(reg)
        }
    }
}
