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
                match op {
                    ast::BinaryOp::Assign => {
                        if let ast::Expr::Identifier(name) = &left.node {
                            let rr = self.compile_expr(right)?;
                            let dest_reg = self.var_to_reg.get(name).copied()
                                .ok_or_else(|| format!("Undefined variable: {}", name))?;
                            self.emit(OpCode::Mov(dest_reg, rr));
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

            ast::Expr::If { condition, consequence, alternative } => {
                let cond_reg = self.compile_expr(condition)?;
                let jz_idx = self.code.len();
                self.emit(OpCode::Jz(cond_reg, 0)); // placeholder

                let cons_reg = self.compile_expr(consequence)?;
                let result_reg = self.alloc_register()?;
                self.emit(OpCode::Mov(result_reg, cons_reg));

                let jmp_idx = self.code.len();
                self.emit(OpCode::Jmp(0)); // placeholder

                let else_addr = self.code.len();
                self.code[jz_idx] = OpCode::Jz(cond_reg, else_addr);

                let alt_reg = if let Some(alt) = alternative {
                    self.compile_expr(alt)?
                } else {
                    let r = self.alloc_register()?;
                    let idx = self.add_constant(Value::Unit);
                    self.emit(OpCode::PushConst(r, idx));
                    r
                };
                self.emit(OpCode::Mov(result_reg, alt_reg));

                let end_addr = self.code.len();
                self.code[jmp_idx] = OpCode::Jmp(end_addr);

                Ok(result_reg)
            }

            ast::Expr::While { condition, body } => {
                let loop_addr = self.code.len();
                let cond_reg = self.compile_expr(condition)?;
                let jz_idx = self.code.len();
                self.emit(OpCode::Jz(cond_reg, 0)); // placeholder

                self.compile_block(body)?;
                self.emit(OpCode::Jmp(loop_addr));

                let exit_addr = self.code.len();
                self.code[jz_idx] = OpCode::Jz(cond_reg, exit_addr);

                let r = self.alloc_register()?;
                let idx = self.add_constant(Value::Unit);
                self.emit(OpCode::PushConst(r, idx));
                Ok(r)
            }

            ast::Expr::Block(block) => self.compile_block(block),

            ast::Expr::Match { scrutinee, arms } => {
                // Check exhaustiveness: last arm must be wildcard or bind
                if !arms.is_empty() {
                    let last_arm = &arms[arms.len() - 1];
                    match &last_arm.pattern.node {
                        ast::Pattern::Wildcard | ast::Pattern::Bind(_) => {},
                        _ => return Err("Non-exhaustive match pattern".to_string()),
                    }
                } else {
                    return Err("Empty match expression".to_string());
                }

                let scrutinee_reg = self.compile_expr(scrutinee)?;
                let result_reg = self.alloc_register()?;
                let mut exit_jumps = Vec::new();

                // Try each arm
                for arm in arms {
                    match &arm.pattern.node {
                        ast::Pattern::Wildcard => {
                            // Wildcard always matches - compile body and we're done
                            let body_reg = self.compile_expr(&arm.body)?;
                            self.emit(OpCode::Mov(result_reg, body_reg));
                            break; // Stop processing arms
                        }
                        ast::Pattern::Literal(lit) => {
                            // Compile pattern as constant
                            let pat_val = match lit {
                                ast::Literal::Int(n)    => Value::Int(*n),
                                ast::Literal::Float(f)  => Value::Float(*f),
                                ast::Literal::Bool(b)   => Value::Bool(*b),
                                ast::Literal::String(s) => Value::String(s.clone()),
                                ast::Literal::Unit      => Value::Unit,
                                _ => return Err("Unsupported literal in pattern".to_string()),
                            };
                            let pat_idx = self.add_constant(pat_val);
                            let pat_reg = self.alloc_register()?;
                            self.emit(OpCode::PushConst(pat_reg, pat_idx));

                            // Compare scrutinee with pattern
                            let eq_reg = self.alloc_register()?;
                            self.emit(OpCode::Eq(eq_reg, scrutinee_reg, pat_reg));

                            // Jump to next arm if not equal
                            let jz_idx = self.code.len();
                            self.emit(OpCode::Jz(eq_reg, 0)); // placeholder

                            // Pattern matched: compile body
                            let body_reg = self.compile_expr(&arm.body)?;
                            self.emit(OpCode::Mov(result_reg, body_reg));
                            exit_jumps.push(self.code.len());
                            self.emit(OpCode::Jmp(0)); // placeholder to end

                            // Patch jz to next arm
                            let next_addr = self.code.len();
                            self.code[jz_idx] = OpCode::Jz(eq_reg, next_addr);
                        }
                        ast::Pattern::Bind(name) => {
                            // Bind pattern always matches and binds variable
                            self.var_to_reg.insert(name.clone(), scrutinee_reg);
                            let body_reg = self.compile_expr(&arm.body)?;
                            self.emit(OpCode::Mov(result_reg, body_reg));
                            break; // Stop processing arms
                        }
                        _ => return Err("Unsupported pattern in match".to_string()),
                    }
                }

                // Patch all exit jumps to current position
                let end_addr = self.code.len();
                for &jmp_idx in &exit_jumps {
                    self.code[jmp_idx] = OpCode::Jmp(end_addr);
                }

                Ok(result_reg)
            }

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
