// Low-level codegen helpers: register allocation, opcodes, constants, jumps, result-wrapping.

use crate::ast;
use crate::bytecode::{OpCode, Register, FRAME_REGS};
use crate::compiler::Compiler;
use crate::compiler::effects;
use crate::bytecode::Value;

pub(in crate::compiler) fn to_u16(n: usize, what: &str) -> Result<u16, String> {
    u16::try_from(n).map_err(|_| format!("{} exceeds u16 range (got {}, max {})", what, n, u16::MAX))
}

pub(in crate::compiler) fn to_u8(n: usize, what: &str) -> Result<u8, String> {
    u8::try_from(n).map_err(|_| format!("{} exceeds u8 range (got {}, max {})", what, n, u8::MAX))
}

impl Compiler {
    pub(in crate::compiler) fn alloc_register(&mut self) -> Result<Register, String> {
        if (self.next_reg as usize) >= FRAME_REGS {
            return Err(format!(
                "Register overflow (per-frame budget is {})",
                FRAME_REGS
            ));
        }
        let reg = Register(self.next_reg as u8);
        self.next_reg += 1;
        if self.next_reg > self.max_reg { self.max_reg = self.next_reg; }
        Ok(reg)
    }

    // Safety: no reg allocated above snapshot must be referenced after restore.
    pub(in crate::compiler) fn snapshot_register_high_water(&self) -> u16 {
        self.next_reg
    }

    pub(in crate::compiler) fn restore_register_high_water(&mut self, mark: u16) {
        if mark < self.next_reg {
            self.next_reg = mark;
        }
    }

    pub(in crate::compiler) fn emit(&mut self, op: OpCode) {
        self.track_dest_handle_bit(&op);
        self.code.push(op);
    }

    fn track_dest_handle_bit(&mut self, op: &OpCode) {
        use OpCode::*;
        let (dest, holds): (Register, bool) = match op {
            Alloc(d, _) => (*d, true),
            Drop(d) => (*d, false),
            Add(d,_,_) | Sub(d,_,_) | Mul(d,_,_) | Div(d,_,_) | Mod(d,_,_) | Neg(d,_) |
            Eq(d,_,_) | Neq(d,_,_) |
            Lt(d,_,_) | Gt(d,_,_) | Lte(d,_,_) | Gte(d,_,_) |
            And(d,_,_) | Or(d,_,_) | Xor(d,_,_) | Shl(d,_,_) | Shr(d,_,_) |
            AddImm(d,_,_) | SubImm(d,_,_) |
            FAdd(d,_,_) | FSub(d,_,_) | FMul(d,_,_) | FDiv(d,_,_) | FNeg(d,_) |
            FLt(d,_,_) | FEq(d,_,_) => (*d, false),
            PushConst(d, idx) => {
                let h = self.const_mask_bits.get(*idx as usize).copied().unwrap_or(false);
                (*d, h)
            }
            Copy(d, s) | Move(d, s) => {
                let h = self.reg_holds_handle.get(s.0 as usize).copied().unwrap_or(false);
                (*d, h)
            }
            // Pessimistic: result might be a handle.
            Ld(d, _, _) | LdIdx(d, _, _) | Dei(d, _) |
            Call(d, _) | CallReg(d, _) | Resume(d, _) => (*d, true),
            _ => return,
        };
        let i = dest.0 as usize;
        if i >= self.reg_holds_handle.len() {
            self.reg_holds_handle.resize(i + 1, false);
        }
        self.reg_holds_handle[i] = holds;
    }

    pub(in crate::compiler) fn try_redirect_last_dest(&mut self, old: Register, new: Register) -> bool {
        if old == new { return true; }
        let Some(last) = self.code.last_mut() else { return false; };
        let d: &mut Register = match last {
            OpCode::Add(d, _, _) | OpCode::Sub(d, _, _) | OpCode::Mul(d, _, _) |
            OpCode::Div(d, _, _) | OpCode::Mod(d, _, _) | OpCode::Neg(d, _) |
            OpCode::Eq(d, _, _) | OpCode::Neq(d, _, _) |
            OpCode::Lt(d, _, _) | OpCode::Gt(d, _, _) | OpCode::Lte(d, _, _) | OpCode::Gte(d, _, _) |
            OpCode::And(d, _, _) | OpCode::Or(d, _, _) | OpCode::Xor(d, _, _) |
            OpCode::Shl(d, _, _) | OpCode::Shr(d, _, _) |
            OpCode::PushConst(d, _) |
            OpCode::Copy(d, _) | OpCode::Move(d, _) |
            OpCode::Ld(d, _, _) | OpCode::LdIdx(d, _, _) |
            OpCode::AddImm(d, _, _) | OpCode::SubImm(d, _, _) |
            OpCode::Alloc(d, _) |
            OpCode::Call(d, _) | OpCode::CallReg(d, _) => d,
            _ => return false,
        };
        if *d == old {
            *d = new;
            true
        } else {
            false
        }
    }

    pub(in crate::compiler) fn check_int32_literal(&self, n: i64) -> Result<(), String> {
        if self.int32_mode && (n < i32::MIN as i64 || n > i32::MAX as i64) {
            return Err(format!(
                "Int literal {} out of i32 range; --int32 mode requires values in {}..={}",
                n, i32::MIN, i32::MAX
            ));
        }
        Ok(())
    }

    pub(in crate::compiler) fn check_float32_literal(&self, f: f64) -> Result<(), String> {
        if self.int32_mode && f.is_finite() && (f as f32) as f64 != f {
            return Err(format!(
                "Float literal {} not representable as f32; --int32 mode requires f32-safe values",
                f
            ));
        }
        Ok(())
    }

    pub(in crate::compiler) fn add_constant(&mut self, val: Value) -> Result<u16, String> {
        for (i, c) in self.constants.iter().enumerate() {
            if *c == val && !self.const_mask_bits[i] {
                return Ok(i as u16);
            }
        }
        if self.constants.len() >= u16::MAX as usize {
            return Err("Constant pool overflow (max 65535 entries)".to_string());
        }
        self.constants.push(val);
        self.const_mask_bits.push(false);
        Ok((self.constants.len() - 1) as u16)
    }

    pub(in crate::compiler) fn add_string_constant(&mut self, s: &str) -> Result<u16, String> {
        let str_idx = if let Some(i) = self.string_constants.iter().position(|c| c == s) {
            i as u32
        } else {
            if self.string_constants.len() >= u32::MAX as usize {
                return Err("String constant pool overflow".to_string());
            }
            self.string_constants.push(s.to_string());
            (self.string_constants.len() - 1) as u32
        };
        // Reuse an existing handle-tagged constant pointing to the same string.
        let placeholder = Value::from_raw(str_idx as u64);
        for (i, c) in self.constants.iter().enumerate() {
            if *c == placeholder && self.const_mask_bits[i] {
                return Ok(i as u16);
            }
        }
        if self.constants.len() >= u16::MAX as usize {
            return Err("Constant pool overflow (max 65535 entries)".to_string());
        }
        self.constants.push(placeholder);
        self.const_mask_bits.push(true);
        Ok((self.constants.len() - 1) as u16)
    }

    pub(in crate::compiler) fn emit_region_push(&mut self) -> Result<(), String> {
        self.emit_region_marker(crate::bytecode::REGION_PORT_PUSH)?;
        self.compiler_region_depth += 1;
        Ok(())
    }

    pub(in crate::compiler) fn emit_region_pop(&mut self) -> Result<(), String> {
        self.emit_region_marker(crate::bytecode::REGION_PORT_POP)?;
        debug_assert!(self.compiler_region_depth > 0,
            "emit_region_pop: compiler_region_depth underflow");
        self.compiler_region_depth = self.compiler_region_depth.saturating_sub(1);
        Ok(())
    }

    // Pop without updating static depth (for control-flow forms that don't fall through).
    fn emit_region_pop_unaccounted(&mut self) -> Result<(), String> {
        self.emit_region_marker(crate::bytecode::REGION_PORT_POP)
    }

    // Emit N pops without updating static depth tracker (for break/continue/return/throw).
    pub(in crate::compiler) fn emit_region_pops_for_exit(&mut self, n: usize) -> Result<(), String> {
        for _ in 0..n {
            self.emit_region_pop_unaccounted()?;
        }
        Ok(())
    }

    pub(in crate::compiler) fn emit_handler_pop_one(&mut self, table_reg: Register) -> Result<(), String> {
        let unit_reg = self.alloc_register()?;
        let unit_idx = self.add_constant(Value::UNIT)?;
        self.emit(OpCode::PushConst(unit_reg, unit_idx));
        let port_reg = self.alloc_register()?;
        let port_val = ((crate::bytecode::DISPATCH_ID as i64) << 8)
            | (crate::bytecode::DISPATCH_PORT_POP_HANDLER as i64);
        let port_idx = self.add_constant(Value::from_int(port_val))?;
        self.emit(OpCode::PushConst(port_reg, port_idx));
        self.emit(OpCode::Deo(unit_reg, port_reg));
        self.emit(OpCode::Drop(table_reg));
        Ok(())
    }

    pub(in crate::compiler) fn emit_handler_pops_for_exit(&mut self, n: usize) -> Result<(), String> {
        let total = self.handler_table_stack.len();
        for i in 0..n {
            let idx = total - 1 - i;
            let reg = self.handler_table_stack[idx];
            self.emit_handler_pop_one(reg)?;
        }
        Ok(())
    }

    pub(in crate::compiler) fn emit_region_forget(&mut self, reg: Register) -> Result<(), String> {
        let port_val =
            ((crate::bytecode::REGION_ID as i64) << 8) | (crate::bytecode::REGION_PORT_FORGET as i64);
        let port_idx = self.add_constant(Value::from_int(port_val))?;
        let port_reg = self.alloc_register()?;
        self.emit(OpCode::PushConst(port_reg, port_idx));
        self.emit(OpCode::Deo(reg, port_reg));
        Ok(())
    }

    // Walk type structure emitting region_forget for carried value + reachable handles.
    pub(in crate::compiler) fn emit_region_forget_typed(
        &mut self,
        reg: Register,
        ty: &ast::Type,
    ) -> Result<(), String> {
        self.emit_region_forget_typed_inner(reg, ty, 0)
    }

    fn emit_region_forget_typed_inner(
        &mut self,
        reg: Register,
        ty: &ast::Type,
        depth: usize,
    ) -> Result<(), String> {
        const MAX_DEPTH: usize = 32;
        if depth > MAX_DEPTH {
            return Err(format!(
                "region_forget type recursion exceeded {} levels — \
                 carried value is too deeply nested to escape; \
                 bind to a let inside the region instead",
                MAX_DEPTH
            ));
        }
        match ty {
            ast::Type::Named(n) if matches!(
                n.as_str(),
                "Int" | "Float" | "Bool" | "Char" | "Unit" | "Never"
            ) => Ok(()),

            ast::Type::Reference { .. }
            | ast::Type::Function { .. } => {
                self.emit_region_forget(reg)
            }

            ast::Type::Tuple(items) => {
                self.emit_region_forget(reg)?;
                for (i, t) in items.iter().enumerate() {
                    let inner = self.alloc_register()?;
                    let offset = u16::try_from(i)
                        .map_err(|_| "tuple index exceeds u16".to_string())?;
                    self.emit(OpCode::Ld(inner, reg, offset));
                    self.emit_region_forget_typed_inner(inner, t, depth + 1)?;
                }
                Ok(())
            }

            ast::Type::Named(n) => {
                self.emit_region_forget(reg)?;
                if let Some(layout) = self.layouts.records.get(n).cloned() {
                    for (i, fty) in layout.field_types.iter().enumerate() {
                        let inner = self.alloc_register()?;
                        let offset = u16::try_from(i)
                            .map_err(|_| "record field index exceeds u16".to_string())?;
                        self.emit(OpCode::Ld(inner, reg, offset));
                        self.emit_region_forget_typed_inner(inner, fty, depth + 1)?;
                    }
                }
                Ok(())
            }

            ast::Type::Generic { .. } => {
                self.emit_region_forget(reg)
            }

            _ => self.emit_region_forget(reg),
        }
    }

    pub(in crate::compiler) fn emit_drops_for_exit(
        &mut self,
        n_blocks: usize,
        skip: Option<Register>,
    ) -> Result<(), String> {
        let total = self.block_locals_stack.len();
        let start = total.saturating_sub(n_blocks);
        for layer_idx in (start..total).rev() {
            let regs: Vec<Register> = self.block_locals_stack[layer_idx].clone();
            for reg in regs {
                if Some(reg) == skip { continue; }
                self.emit(OpCode::Drop(reg));
            }
        }
        Ok(())
    }

    fn emit_region_marker(&mut self, port: u8) -> Result<(), String> {
        let port_val = ((crate::bytecode::REGION_ID as i64) << 8) | (port as i64);
        let val_idx = self.add_constant(Value::from_int(0))?;
        let port_idx = self.add_constant(Value::from_int(port_val))?;
        let val_reg = self.alloc_register()?;
        let port_reg = self.alloc_register()?;
        self.emit(OpCode::PushConst(val_reg, val_idx));
        self.emit(OpCode::PushConst(port_reg, port_idx));
        self.emit(OpCode::Deo(val_reg, port_reg));
        Ok(())
    }

    pub(in crate::compiler) fn rel_offset(&self, target_pc: usize, branch_pc: usize) -> Result<i16, String> {
        let off = target_pc as isize - (branch_pc as isize + 1);
        i16::try_from(off).map_err(|_| format!("Branch offset {} exceeds 16-bit range", off))
    }

    pub(in crate::compiler) fn patch_jz_at(&mut self, branch_pc: usize, target_pc: usize) -> Result<(), String> {
        let off = self.rel_offset(target_pc, branch_pc)?;
        if let OpCode::Jz(r, _) = self.code[branch_pc] {
            self.code[branch_pc] = OpCode::Jz(r, off);
        }
        Ok(())
    }

    pub(in crate::compiler) fn patch_jmp_at(&mut self, branch_pc: usize, target_pc: usize) -> Result<(), String> {
        let off = self.rel_offset(target_pc, branch_pc)?;
        if matches!(self.code[branch_pc], OpCode::Jmp(_)) {
            self.code[branch_pc] = OpCode::Jmp(off);
        }
        Ok(())
    }

    pub(in crate::compiler) fn wrap_ok(&mut self, value: Register) -> Result<Register, String> {
        self.wrap_result(value, effects::OK_TAG)
    }

    pub(in crate::compiler) fn wrap_err(&mut self, value: Register) -> Result<Register, String> {
        self.wrap_result(value, effects::ERR_TAG)
    }

    fn wrap_result(&mut self, value: Register, tag: u32) -> Result<Register, String> {
        let dest = self.alloc_register()?;
        self.emit(OpCode::Alloc(dest, 2));
        let tag_reg = self.alloc_register()?;
        let idx = self.add_constant(Value::from_int(tag as i64))?;
        self.emit(OpCode::PushConst(tag_reg, idx));
        self.emit(OpCode::St(tag_reg, dest, 0));
        self.emit(OpCode::St(value, dest, 1));
        Ok(dest)
    }
}
