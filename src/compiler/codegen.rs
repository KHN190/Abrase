use super::*;
use super::effects;
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

    pub(super) fn add_constant(&mut self, val: Value) -> Result<u16, String> {
        if self.constants.len() >= u16::MAX as usize {
            return Err("Constant pool overflow (max 65535 entries)".to_string());
        }
        self.constants.push(val);
        Ok((self.constants.len() - 1) as u16)
    }

    fn rel_offset(&self, target_pc: usize, branch_pc: usize) -> Result<i16, String> {
        let off = target_pc as isize - (branch_pc as isize + 1);
        i16::try_from(off).map_err(|_| format!("Branch offset {} exceeds 16-bit range", off))
    }

    fn patch_jz_at(&mut self, branch_pc: usize, target_pc: usize) -> Result<(), String> {
        let off = self.rel_offset(target_pc, branch_pc)?;
        if let OpCode::Jz(r, _) = self.code[branch_pc] {
            self.code[branch_pc] = OpCode::Jz(r, off);
        }
        Ok(())
    }

    fn patch_jmp_at(&mut self, branch_pc: usize, target_pc: usize) -> Result<(), String> {
        let off = self.rel_offset(target_pc, branch_pc)?;
        if matches!(self.code[branch_pc], OpCode::Jmp(_)) {
            self.code[branch_pc] = OpCode::Jmp(off);
        }
        Ok(())
    }

    pub(super) fn wrap_ok(&mut self, value: Register) -> Result<Register, String> {
        self.wrap_result(value, effects::OK_TAG)
    }

    pub(super) fn wrap_err(&mut self, value: Register) -> Result<Register, String> {
        self.wrap_result(value, effects::ERR_TAG)
    }

    fn wrap_result(&mut self, value: Register, tag: u32) -> Result<Register, String> {
        let dest = self.alloc_register()?;
        self.emit(OpCode::Alloc(dest, 2));
        let tag_reg = self.alloc_register()?;
        let idx = self.add_constant(Value::Int(tag as i64))?;
        self.emit(OpCode::PushConst(tag_reg, idx));
        self.emit(OpCode::St(tag_reg, dest, 0));
        self.emit(OpCode::St(value, dest, 1));
        Ok(dest)
    }

    pub(super) fn compile_expr(&mut self, expr: &ast::Spanned<ast::Expr>) -> Result<Register, String> {
        match &expr.node {
            ast::Expr::Error => Err("Compilation aborted: parser error was not recovered; fix parser errors first".to_string()),
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
                let idx = self.add_constant(val)?;
                self.emit(OpCode::PushConst(reg, idx));
                Ok(reg)
            }

            ast::Expr::Identifier(name) => {
                if let Some(reg) = self.var_to_reg.get(name).copied() {
                    return Ok(reg);
                }
                if let Some(info) = self.layouts.variants.get(name).cloned() {
                    let dest = self.alloc_register()?;
                    self.emit(OpCode::Alloc(dest, 1));
                    let tag_reg = self.alloc_register()?;
                    let idx = self.add_constant(Value::Int(info.tag as i64))?;
                    self.emit(OpCode::PushConst(tag_reg, idx));
                    self.emit(OpCode::St(tag_reg, dest, 0));
                    return Ok(dest);
                }
                Err(format!("Undefined variable: {}", name))
            }

            ast::Expr::Unary { op, right } => {
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

            ast::Expr::Binary { op, left, right } => {
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

            ast::Expr::If { condition, consequence, alternative } => {
                let cond_reg = self.compile_expr(condition)?;
                let jz_idx = self.code.len();
                self.emit(OpCode::Jz(cond_reg, 0));

                let cons_reg = self.compile_expr(consequence)?;
                let result_reg = self.alloc_register()?;
                self.emit(OpCode::Copy(result_reg, cons_reg));

                let jmp_idx = self.code.len();
                self.emit(OpCode::Jmp(0));

                let else_addr = self.code.len();
                self.patch_jz_at(jz_idx, else_addr)?;

                let alt_reg = if let Some(alt) = alternative {
                    self.compile_expr(alt)?
                } else {
                    let r = self.alloc_register()?;
                    let idx = self.add_constant(Value::Unit)?;
                    self.emit(OpCode::PushConst(r, idx));
                    r
                };
                self.emit(OpCode::Copy(result_reg, alt_reg));

                let end_addr = self.code.len();
                self.patch_jmp_at(jmp_idx, end_addr)?;

                Ok(result_reg)
            }

            ast::Expr::While { condition, body } => {
                let loop_addr = self.code.len();
                let cond_reg = self.compile_expr(condition)?;
                let jz_idx = self.code.len();
                self.emit(OpCode::Jz(cond_reg, 0));

                self.compile_block(body)?;
                let back_idx = self.code.len();
                let back_off = self.rel_offset(loop_addr, back_idx)?;
                self.emit(OpCode::Jmp(back_off));

                let exit_addr = self.code.len();
                self.patch_jz_at(jz_idx, exit_addr)?;

                let r = self.alloc_register()?;
                let idx = self.add_constant(Value::Unit)?;
                self.emit(OpCode::PushConst(r, idx));
                Ok(r)
            }

            ast::Expr::Block(block) => self.compile_block(block),

            ast::Expr::Match { scrutinee, arms } => {
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

                for arm in arms {
                    match &arm.pattern.node {
                        ast::Pattern::Wildcard => {
                            let body_reg = self.compile_expr(&arm.body)?;
                            self.emit(OpCode::Copy(result_reg, body_reg));
                            break;
                        }
                        ast::Pattern::Literal(lit) => {
                            let pat_val = match lit {
                                ast::Literal::Int(n)    => Value::Int(*n),
                                ast::Literal::Float(f)  => Value::Float(*f),
                                ast::Literal::Bool(b)   => Value::Bool(*b),
                                ast::Literal::String(s) => Value::String(s.clone()),
                                ast::Literal::Unit      => Value::Unit,
                                _ => return Err("Unsupported literal in pattern".to_string()),
                            };
                            let pat_idx = self.add_constant(pat_val)?;
                            let pat_reg = self.alloc_register()?;
                            self.emit(OpCode::PushConst(pat_reg, pat_idx));

                            let eq_reg = self.alloc_register()?;
                            self.emit(OpCode::Eq(eq_reg, scrutinee_reg, pat_reg));

                            let jz_idx = self.code.len();
                            self.emit(OpCode::Jz(eq_reg, 0));

                            let body_reg = self.compile_expr(&arm.body)?;
                            self.emit(OpCode::Copy(result_reg, body_reg));
                            exit_jumps.push(self.code.len());
                            self.emit(OpCode::Jmp(0));

                            let next_addr = self.code.len();
                            self.patch_jz_at(jz_idx, next_addr)?;
                        }
                        ast::Pattern::Bind(name) => {
                            if let Some(info) = self.layouts.variants.get(name).cloned() {
                                let tag_reg = self.alloc_register()?;
                                self.emit(OpCode::Ld(tag_reg, scrutinee_reg, 0));
                                let expected_tag = self.alloc_register()?;
                                let ti = self.add_constant(Value::Int(info.tag as i64))?;
                                self.emit(OpCode::PushConst(expected_tag, ti));
                                let eq_reg = self.alloc_register()?;
                                self.emit(OpCode::Eq(eq_reg, tag_reg, expected_tag));
                                let jz_idx = self.code.len();
                                self.emit(OpCode::Jz(eq_reg, 0));
                                let body_reg = self.compile_expr(&arm.body)?;
                                self.emit(OpCode::Copy(result_reg, body_reg));
                                exit_jumps.push(self.code.len());
                                self.emit(OpCode::Jmp(0));
                                let next_addr = self.code.len();
                                self.patch_jz_at(jz_idx, next_addr)?;
                            } else {
                                self.var_to_reg.insert(name.clone(), scrutinee_reg);
                                let body_reg = self.compile_expr(&arm.body)?;
                                self.emit(OpCode::Copy(result_reg, body_reg));
                                break;
                            }
                        }
                        ast::Pattern::Variant { ty: vty, args } => {
                            let vname = vty.last().cloned()
                                .ok_or_else(|| "Variant pattern missing name".to_string())?;
                            let info = self.layouts.variants.get(&vname).cloned()
                                .ok_or_else(|| format!("Unknown variant: {}", vname))?;

                            let tag_reg = self.alloc_register()?;
                            self.emit(OpCode::Ld(tag_reg, scrutinee_reg, 0));
                            let expected_tag = self.alloc_register()?;
                            let ti = self.add_constant(Value::Int(info.tag as i64))?;
                            self.emit(OpCode::PushConst(expected_tag, ti));
                            let eq_reg = self.alloc_register()?;
                            self.emit(OpCode::Eq(eq_reg, tag_reg, expected_tag));
                            let jz_idx = self.code.len();
                            self.emit(OpCode::Jz(eq_reg, 0));

                            for (i, arg_pat) in args.iter().enumerate() {
                                if let ast::Pattern::Bind(n) = &arg_pat.node {
                                    let r = self.alloc_register()?;
                                    self.emit(OpCode::Ld(r, scrutinee_reg, (i + 1) as u16));
                                    self.var_to_reg.insert(n.clone(), r);
                                }
                            }

                            let body_reg = self.compile_expr(&arm.body)?;
                            self.emit(OpCode::Copy(result_reg, body_reg));
                            exit_jumps.push(self.code.len());
                            self.emit(OpCode::Jmp(0));

                            let next_addr = self.code.len();
                            self.patch_jz_at(jz_idx, next_addr)?;
                        }
                        ast::Pattern::Record { ty: rty, fields, .. } => {
                            let type_name = rty.last().cloned()
                                .ok_or_else(|| "Record pattern missing type name".to_string())?;
                            let field_order = self.layouts.records.get(&type_name).cloned()
                                .ok_or_else(|| format!("Unknown record type: {}", type_name))?;

                            for fp in fields {
                                if let Some(idx) = field_order.fields.iter().position(|n| n == &fp.name) {
                                    let bind_name = match &fp.pattern {
                                        Some(p) => if let ast::Pattern::Bind(n) = &p.node {
                                            Some(n.clone())
                                        } else { None },
                                        None => Some(fp.name.clone()),
                                    };
                                    if let Some(n) = bind_name {
                                        let r = self.alloc_register()?;
                                        self.emit(OpCode::Ld(r, scrutinee_reg, idx as u16));
                                        self.var_to_reg.insert(n, r);
                                    }
                                }
                            }
                            let body_reg = self.compile_expr(&arm.body)?;
                            self.emit(OpCode::Copy(result_reg, body_reg));
                            break;
                        }
                        _ => return Err("Unsupported pattern in match".to_string()),
                    }
                }

                let end_addr = self.code.len();
                for &jmp_idx in &exit_jumps {
                    self.patch_jmp_at(jmp_idx, end_addr)?;
                }

                Ok(result_reg)
            }

            ast::Expr::Call { callee, args } => {
                if let ast::Expr::Identifier(name) = &callee.node {
                    if name == "Shared" && args.len() == 1 {
                        let src = self.compile_expr(&args[0])?;
                        let dest = self.alloc_register()?;
                        self.emit(OpCode::Alloc(dest, 1));
                        self.emit(OpCode::St(src, dest, 0));
                        return Ok(dest);
                    }
                    if let Some(info) = self.layouts.variants.get(name).cloned() {
                        let dest = self.alloc_register()?;
                        let payload = args.len();
                        self.emit(OpCode::Alloc(dest, (payload + 1) as u16));
                        let tag_reg = self.alloc_register()?;
                        let ti = self.add_constant(Value::Int(info.tag as i64))?;
                        self.emit(OpCode::PushConst(tag_reg, ti));
                        self.emit(OpCode::St(tag_reg, dest, 0));
                        for (i, arg) in args.iter().enumerate() {
                            let v = self.compile_expr(arg)?;
                            self.emit(OpCode::St(v, dest, (i + 1) as u16));
                        }
                        return Ok(dest);
                    }
                    let func_id = self.func_map.get(name).copied()
                        .ok_or_else(|| format!("Undefined function: {}", name))?;
                    if func_id > u16::MAX as usize {
                        return Err(format!("Function id {} exceeds u16 range", func_id));
                    }

                    let mut arg_srcs = Vec::with_capacity(args.len());
                    for arg in args {
                        arg_srcs.push(self.compile_expr(arg)?);
                    }

                    for (i, src) in arg_srcs.iter().enumerate() {
                        if i > u8::MAX as usize {
                            return Err("Too many arguments (>255)".to_string());
                        }
                        let pos = self.code.len();
                        self.emit(OpCode::Copy(Register(0), *src));
                        self.pending_arg_patches.push((pos, i as u8));
                    }

                    let dest = self.alloc_register()?;
                    self.emit(OpCode::Call(dest, func_id as u16));
                    Ok(dest)
                } else {
                    Err("Call target must be a function identifier".to_string())
                }
            }

            ast::Expr::Return(opt_expr) => {
                let r = if let Some(expr) = opt_expr {
                    self.compile_expr(expr)?
                } else {
                    let reg = self.alloc_register()?;
                    let idx = self.add_constant(Value::Unit)?;
                    self.emit(OpCode::PushConst(reg, idx));
                    reg
                };
                let ret_reg = if self.current_fn_fallible { self.wrap_ok(r)? } else { r };
                self.emit(OpCode::Ret(ret_reg));
                Ok(r)
            }

            ast::Expr::Throw(inner) => {
                if !self.current_fn_fallible {
                    return Err("`throw` outside <exn> function".to_string());
                }
                let err_val = self.compile_expr(inner)?;
                let wrapped = self.wrap_err(err_val)?;
                self.emit(OpCode::Ret(wrapped));
                Ok(wrapped)
            }

            ast::Expr::Question(inner) => {
                if !self.current_fn_fallible {
                    return Err("`?` outside <exn> function".to_string());
                }
                let res = self.compile_expr(inner)?;
                let tag = self.alloc_register()?;
                self.emit(OpCode::Ld(tag, res, 0));
                let err_tag = self.alloc_register()?;
                let idx = self.add_constant(Value::Int(effects::ERR_TAG as i64))?;
                self.emit(OpCode::PushConst(err_tag, idx));
                let is_err = self.alloc_register()?;
                self.emit(OpCode::Eq(is_err, tag, err_tag));
                let jz_idx = self.code.len();
                self.emit(OpCode::Jz(is_err, 0));
                self.emit(OpCode::Ret(res));
                let after = self.code.len();
                self.patch_jz_at(jz_idx, after)?;
                let val = self.alloc_register()?;
                self.emit(OpCode::Ld(val, res, 1));
                Ok(val)
            }

            ast::Expr::Record { ty, fields } => {
                let type_name = ty.last().cloned()
                    .ok_or_else(|| "Record literal missing type name".to_string())?;
                let field_order = self.layouts.records.get(&type_name).cloned()
                    .ok_or_else(|| format!("Unknown record type: {}", type_name))?;
                let dest = self.alloc_register()?;
                self.emit(OpCode::Alloc(dest, field_order.fields.len() as u16));
                for (i, fname) in field_order.fields.iter().enumerate() {
                    let init = fields.iter().find(|f| &f.name == fname)
                        .ok_or_else(|| format!("Missing field '{}' in {}", fname, type_name))?;
                    let src = if let Some(v) = &init.value {
                        self.compile_expr(v)?
                    } else {
                        self.var_to_reg.get(&init.name).copied()
                            .ok_or_else(|| format!("Undefined variable: {}", init.name))?
                    };
                    self.emit(OpCode::St(src, dest, i as u16));
                }
                Ok(dest)
            }

            ast::Expr::Variant { ty, args } => {
                let vname = ty.last().cloned()
                    .ok_or_else(|| "Variant constructor missing name".to_string())?;
                let info = self.layouts.variants.get(&vname).cloned()
                    .ok_or_else(|| format!("Unknown variant: {}", vname))?;
                let dest = self.alloc_register()?;
                let payload = args.len();
                self.emit(OpCode::Alloc(dest, (payload + 1) as u16));
                let tag_reg = self.alloc_register()?;
                let ti = self.add_constant(Value::Int(info.tag as i64))?;
                self.emit(OpCode::PushConst(tag_reg, ti));
                self.emit(OpCode::St(tag_reg, dest, 0));
                for (i, arg) in args.iter().enumerate() {
                    let v = self.compile_expr(arg)?;
                    self.emit(OpCode::St(v, dest, (i + 1) as u16));
                }
                Ok(dest)
            }

            ast::Expr::FieldAccess { base, field } => {
                let base_reg = self.compile_expr(base)?;
                let type_name = self.infer_expr_type(base).and_then(|t| match t {
                    ast::Type::Named(n) => Some(n),
                    _ => None,
                }).ok_or_else(|| format!("Cannot determine record type for field access '.{}'", field))?;
                let layout = self.layouts.records.get(&type_name)
                    .ok_or_else(|| format!("Unknown record type: {}", type_name))?;
                let idx = layout.offset_of(field)
                    .ok_or_else(|| format!("No field '{}' in {}", field, type_name))?;
                let dest = self.alloc_register()?;
                self.emit(OpCode::Ld(dest, base_reg, idx));
                Ok(dest)
            }

            ast::Expr::Array(items) => {
                let dest = self.alloc_register()?;
                self.emit(OpCode::Alloc(dest, items.len() as u16));
                for (i, item) in items.iter().enumerate() {
                    let v = self.compile_expr(item)?;
                    self.emit(OpCode::St(v, dest, i as u16));
                }
                Ok(dest)
            }

            ast::Expr::Index { base, index } => {
                let base_reg = self.compile_expr(base)?;
                let idx_reg = self.compile_expr(index)?;
                let dest = self.alloc_register()?;
                self.emit(OpCode::LdIdx(dest, base_reg, idx_reg));
                Ok(dest)
            }

            _ => Err(format!("Unsupported expression: {:?}", expr.node)),
        }
    }

    pub(super) fn compile_stmt(&mut self, stmt: &ast::Spanned<ast::Stmt>) -> Result<(), String> {
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

    pub(super) fn finalize_arg_patches(&mut self) -> Result<(), String> {
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

    pub(super) fn infer_expr_type(&self, expr: &ast::Spanned<ast::Expr>) -> Option<ast::Type> {
        match &expr.node {
            ast::Expr::Literal(lit) => Some(match lit {
                ast::Literal::Int(_) => ast::Type::Named("Int".into()),
                ast::Literal::Float(_) => ast::Type::Named("Float".into()),
                ast::Literal::Bool(_) => ast::Type::Named("Bool".into()),
                ast::Literal::Char(_) => ast::Type::Named("Char".into()),
                ast::Literal::String(_) => ast::Type::Named("String".into()),
                ast::Literal::Unit => ast::Type::Tuple(vec![]),
                _ => return None,
            }),
            ast::Expr::Identifier(name) => self.var_types.get(name).cloned(),
            ast::Expr::Record { ty, .. } => ty.last().map(|n| ast::Type::Named(n.clone())),
            ast::Expr::Variant { ty, .. } => ty.last().and_then(|vname| {
                self.layouts.variants.get(vname).map(|info| ast::Type::Named(info.type_name.clone()))
            }),
            _ => None,
        }
    }
}

pub(super) fn is_move_type(ty: &ast::Type) -> bool {
    use crate::ty::Ownership;
    let rty = ast_type_to_rt(ty);
    matches!(rty.ownership(), Ownership::Move)
}

fn ast_type_to_rt(ty: &ast::Type) -> crate::ty::Type {
    use crate::ty::Type as RTy;
    match ty {
        ast::Type::Named(n) => match n.as_str() {
            "Int" => RTy::Int,
            "Float" => RTy::Float,
            "Bool" => RTy::Bool,
            "Char" => RTy::Char,
            "String" => RTy::String,
            "Unit" => RTy::Unit,
            "Never" => RTy::Never,
            _ => RTy::Named(n.clone()),
        },
        ast::Type::Tuple(ts) if ts.is_empty() => RTy::Unit,
        ast::Type::Tuple(ts) => RTy::Tuple(ts.iter().map(ast_type_to_rt).collect()),
        ast::Type::Generic { name, args } => RTy::Generic {
            name: name.clone(),
            args: args.iter().map(ast_type_to_rt).collect(),
        },
        ast::Type::Function { params, ret, .. } => RTy::Function {
            params: params.iter().map(ast_type_to_rt).collect(),
            effects: vec![],
            ret: Box::new(ast_type_to_rt(ret)),
        },
        ast::Type::Reference { is_mut, inner, .. } => RTy::Reference {
            is_mut: *is_mut,
            inner: Box::new(ast_type_to_rt(inner)),
        },
        _ => RTy::Unknown,
    }
}
