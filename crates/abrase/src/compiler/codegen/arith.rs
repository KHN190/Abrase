// Unary and binary expressions.

use crate::ast;
use crate::bytecode::{OpCode, Register};
use crate::compiler::Compiler;
use crate::bytecode::Value;
use super::is_move_type;

impl Compiler {
    pub(in crate::compiler) fn compile_unary(
        &mut self,
        op: &ast::UnaryOp,
        right: &ast::Spanned<ast::Expr>,
    ) -> Result<Register, String> {
        match op {
            ast::UnaryOp::Ref | ast::UnaryOp::RefMut => {
                let inner_ty = self.infer_expr_type(right);
                let cell_typed = inner_ty.as_ref().map(is_move_type).unwrap_or(false);
                let src = self.compile_expr(right)?;
                let dest = self.alloc_register()?;
                if cell_typed {
                    self.emit(OpCode::Copy(dest, src));
                } else {
                    let tmp = self.alloc_register()?;
                    self.emit(OpCode::Copy(tmp, src));
                    self.emit(OpCode::Alloc(dest, 1));
                    self.emit(OpCode::St(tmp, dest, 0));
                }
                Ok(dest)
            }
            ast::UnaryOp::Deref => {
                let inner_ty = self.infer_expr_type(right);
                let inner_is_cell = matches!(
                    inner_ty.as_ref(),
                    Some(ast::Type::Reference { inner, .. }) if is_move_type(inner)
                );
                let pointee_unboxed = match inner_ty.as_ref() {
                    Some(ast::Type::Reference { inner, .. }) => super::data::type_is_unboxed(inner),
                    Some(ast::Type::Generic { name, args }) if name == "Shared" =>
                        args.first().map_or(false, super::data::type_is_unboxed),
                    _ => false,
                };
                let src = self.compile_expr(right)?;
                let dest = self.alloc_register()?;
                if inner_is_cell {
                    self.emit(OpCode::Copy(dest, src));
                } else {
                    self.emit(OpCode::Ld(dest, src, 0));
                }
                if pointee_unboxed { self.set_reg_handle(dest, false); }
                Ok(dest)
            }
            ast::UnaryOp::Neg => {
                if let ast::Expr::Literal(ast::Literal::Int(n)) = &right.node {
                    let v = -n;
                    self.check_int32_literal(v)?;
                    let reg = self.alloc_register()?;
                    let idx = self.add_constant(Value::from_int(v))?;
                    self.emit(OpCode::PushConst(reg, idx));
                    return Ok(reg);
                }
                if let ast::Expr::Literal(ast::Literal::Float(f)) = &right.node {
                    let v = -f;
                    self.check_float32_literal(v)?;
                    let encoded = if self.int32_mode { Value::from_float_f32(v) } else { Value::from_float(v) };
                    let reg = self.alloc_register()?;
                    let idx = self.add_constant(encoded)?;
                    self.emit(OpCode::PushConst(reg, idx));
                    return Ok(reg);
                }
                let is_float = matches!(self.infer_expr_type(right),
                    Some(ast::Type::Named(ref n)) if n == "Float");
                let src = self.compile_expr(right)?;
                let dest = self.alloc_register()?;
                if is_float {
                    self.emit(OpCode::FNeg(dest, src));
                } else {
                    self.emit(OpCode::Neg(dest, src));
                }
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
                match &left.node {
                    ast::Expr::Identifier(name) => {
                        if let Some(offset) = self.resolve_static_offset(name) {
                            let rr = self.compile_expr(right)?;
                            let table = self.load_module_table()?;
                            let tmp = self.alloc_register()?;
                            self.emit(OpCode::Copy(tmp, rr));
                            self.emit(OpCode::St(tmp, table, offset));
                            return Ok(rr);
                        }
                        let rr = self.compile_expr(right)?;
                        let ty = self.var_types.get(name).cloned();
                        let is_heap = ty.as_ref().map(is_move_type).unwrap_or(false);
                        if self.cell_bindings.contains(name) {
                            let cell = *self.var_to_reg.get(name)
                                .ok_or_else(|| format!("cell binding '{}' missing register", name))?;
                            if is_heap {
                                let old = self.alloc_register()?;
                                self.emit(OpCode::Ld(old, cell, 0));
                                self.emit(OpCode::Drop(old));
                            }
                            self.emit(OpCode::St(rr, cell, 0));
                            return Ok(rr);
                        }
                        let bound_depth = self.var_bound_at_region.get(name).copied().unwrap_or(0);
                        if is_heap && bound_depth < self.compiler_region_depth {
                            if let Some(t) = ty.as_ref() {
                                self.emit_region_forget_typed(rr, t)?;
                            }
                        }
                        let dest_reg = match self.var_to_reg.get(name).copied() {
                            Some(r) => {
                                if is_heap { self.emit(OpCode::Drop(r)); }
                                self.emit(OpCode::Copy(r, rr));
                                r
                            }
                            None => {
                                let r = self.alloc_register()?;
                                self.emit(OpCode::Copy(r, rr));
                                self.var_to_reg.insert(name.clone(), r);
                                self.var_bound_at_region.insert(name.clone(), self.compiler_region_depth);
                                r
                            }
                        };
                        Ok(dest_reg)
                    }
                    ast::Expr::Index { base, index } => {
                        let arr_reg = self.compile_expr(base)?;
                        let idx_reg = self.compile_expr(index)?;
                        let val_reg = self.compile_expr(right)?;
                        let tmp = self.alloc_register()?;
                        self.emit(OpCode::Copy(tmp, val_reg));
                        self.emit(OpCode::StIdx(tmp, arr_reg, idx_reg));
                        Ok(val_reg)
                    }
                    ast::Expr::Unary { op: ast::UnaryOp::Deref, right: target } => {
                        let cell_reg = self.compile_expr(target)?;
                        let val_reg = self.compile_expr(right)?;
                        let tmp = self.alloc_register()?;
                        self.emit(OpCode::Copy(tmp, val_reg));
                        self.emit(OpCode::St(tmp, cell_reg, 0));
                        Ok(val_reg)
                    }
                    ast::Expr::FieldAccess { base, field } => {
                        let type_name = self.infer_expr_type(base).and_then(|t| match t {
                            ast::Type::Named(n) => Some(n),
                            ast::Type::Reference { inner, .. } => match *inner {
                                ast::Type::Named(n) => Some(n),
                                _ => None,
                            },
                            _ => None,
                        }).ok_or_else(|| format!("Cannot determine record type for field assignment '.{}'", field))?;
                        let (offset, field_ty) = {
                            let layout = self.layouts.records.get(&type_name)
                                .ok_or_else(|| format!("Unknown record type: {}", type_name))?;
                            let idx = layout.offset_of(field)
                                .ok_or_else(|| format!("No field '{}' in {}", field, type_name))?;
                            let pos = layout.fields.iter().position(|n| n == field)
                                .ok_or_else(|| format!("No field '{}' in {}", field, type_name))?;
                            let fty = layout.field_types.get(pos).cloned();
                            (idx, fty)
                        };
                        let val_reg = self.compile_expr(right)?;
                        let base_reg = self.compile_expr(base)?;
                        let field_is_heap = field_ty.as_ref().map(is_move_type).unwrap_or(false);
                        if field_is_heap {
                            let old = self.alloc_register()?;
                            self.emit(OpCode::Ld(old, base_reg, offset));
                            self.emit(OpCode::Drop(old));
                        }
                        let want_move = self.arg_should_move(right);
                        self.emit_store_field(val_reg, want_move, base_reg, offset)?;
                        Ok(val_reg)
                    }
                    _ => Err("Assignment target must be a variable".to_string())
                }
            }
            ast::BinaryOp::And | ast::BinaryOp::Or => {
                let lr = self.compile_expr(left)?;
                let dest = self.alloc_register()?;
                self.emit(OpCode::Copy(dest, lr));
                let branch_pc = self.code.len();
                if matches!(op, ast::BinaryOp::And) {
                    self.emit(OpCode::Jz(dest, 0));
                } else {
                    self.emit(OpCode::Jnz(dest, 0));
                }
                let rr = self.compile_expr(right)?;
                self.emit(OpCode::Copy(dest, rr));
                let end_pc = self.code.len();
                if matches!(op, ast::BinaryOp::And) {
                    self.patch_jz_at(branch_pc, end_pc)?;
                } else {
                    self.patch_jnz_at(branch_pc, end_pc)?;
                }
                Ok(dest)
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
                let is_float = matches!(self.infer_expr_type(left),
                                Some(ast::Type::Named(ref n)) if n == "Float")
                    && matches!(self.infer_expr_type(right),
                                Some(ast::Type::Named(ref n)) if n == "Float");
                let lr = self.compile_expr(left)?;
                let rr = self.compile_expr(right)?;
                if is_float {
                    let dest = self.alloc_register()?;
                    let opcode = match op {
                        ast::BinaryOp::Add => Some(OpCode::FAdd(dest, lr, rr)),
                        ast::BinaryOp::Sub => Some(OpCode::FSub(dest, lr, rr)),
                        ast::BinaryOp::Mul => Some(OpCode::FMul(dest, lr, rr)),
                        ast::BinaryOp::Div => Some(OpCode::FDiv(dest, lr, rr)),
                        ast::BinaryOp::Lt  => Some(OpCode::FLt(dest, lr, rr)),
                        ast::BinaryOp::Gt  => Some(OpCode::FLt(dest, rr, lr)),
                        ast::BinaryOp::Eq  => Some(OpCode::FEq(dest, lr, rr)),
                        _ => None,
                    };
                    if let Some(o) = opcode {
                        self.emit(o);
                        return Ok(dest);
                    }
                }
                let dr = self.alloc_register()?;
                let instr = match op {
                    ast::BinaryOp::Add => OpCode::Add(dr, lr, rr),
                    ast::BinaryOp::Sub => OpCode::Sub(dr, lr, rr),
                    ast::BinaryOp::Mul => OpCode::Mul(dr, lr, rr),
                    ast::BinaryOp::Div => OpCode::Div(dr, lr, rr),
                    ast::BinaryOp::Mod => OpCode::Mod(dr, lr, rr),
                    ast::BinaryOp::Eq  => OpCode::Eq(dr, lr, rr),
                    ast::BinaryOp::Neq => OpCode::Neq(dr, lr, rr),
                    ast::BinaryOp::Lt  => OpCode::Lt(dr, lr, rr),
                    ast::BinaryOp::Gt  => OpCode::Gt(dr, lr, rr),
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
