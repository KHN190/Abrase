// Low-level codegen helpers: register allocation, opcodes, constants, jumps, result-wrapping.

use crate::ast;
use crate::bytecode::{OpCode, Register, FRAME_REGS};
use crate::compiler::Compiler;
use crate::compiler::effects;
use crate::bytecode::Value;

fn op_reads_reg(op: &OpCode, r: Register) -> bool {
    use OpCode::*;
    match op {
        Add(_, a, b) | Sub(_, a, b) | Mul(_, a, b) | Div(_, a, b) | Mod(_, a, b)
        | Eq(_, a, b) | Neq(_, a, b) | Lt(_, a, b) | Gt(_, a, b) | Lte(_, a, b) | Gte(_, a, b)
        | And(_, a, b) | Or(_, a, b) | Xor(_, a, b) | Shl(_, a, b) | Shr(_, a, b)
        | FAdd(_, a, b) | FSub(_, a, b) | FMul(_, a, b) | FDiv(_, a, b)
        | FLt(_, a, b) | FEq(_, a, b) => *a == r || *b == r,
        Neg(_, a) | FNeg(_, a) => *a == r,
        Copy(_, s) | Move(_, s) => *s == r,
        Drop(s) => *s == r,
        Jz(c, _) | Jnz(c, _) => *c == r,
        Ret(s) => *s == r,
        AddImm(_, s, _) | SubImm(_, s, _) => *s == r,
        Ld(_, b, _) => *b == r,
        St(v, b, _) => *v == r || *b == r,
        LdIdx(_, b, i) => *b == r || *i == r,
        StIdx(v, b, i) => *v == r || *b == r || *i == r,
        Deo(s, p) | Dei(s, p) => *s == r || *p == r,
        CallReg(_, f) => *f == r,
        Resume(_, v) => *v == r,
        Raise(_, k, a) => *k == r || *a == r,
        Handle(t, _) => *t == r,
        _ => false,
    }
}

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
        if let Some(r) = self.module_table_reg {
            if r.0 as u16 >= self.next_reg { self.module_table_reg = None; }
        }
    }

    pub(in crate::compiler) fn emit(&mut self, op: OpCode) {
        self.track_dest_handle_bit(&op);
        self.line_info.push(self.current_span.line as u32);
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
        if holds && i < 128 { self.ever_handle_mask |= 1u128 << i; }
    }

    pub(in crate::compiler) fn set_reg_handle(&mut self, reg: Register, holds: bool) {
        let i = reg.0 as usize;
        if i >= self.reg_holds_handle.len() {
            self.reg_holds_handle.resize(i + 1, false);
        }
        self.reg_holds_handle[i] = holds;
        if holds && i < 128 { self.ever_handle_mask |= 1u128 << i; }
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

    pub(in crate::compiler) fn try_redirect_alloc_block(&mut self, old: Register, new: Register) -> bool {
        if old == new { return true; }
        let len = self.code.len();
        let mut alloc_pos = None;
        for i in (0..len).rev() {
            match &self.code[i] {
                OpCode::Alloc(d, _) if *d == old => { alloc_pos = Some(i); break; }
                OpCode::St(v, b, _) => {
                    if *v == old { return false; }
                    if *b != old { if op_reads_reg(&self.code[i], old) { return false; } }
                }
                op => { if op_reads_reg(op, old) { return false; } }
            }
        }
        let alloc_pos = match alloc_pos { Some(p) => p, None => return false };
        for i in alloc_pos..len {
            match &mut self.code[i] {
                OpCode::Alloc(d, _) if *d == old => { *d = new; }
                OpCode::St(_, b, _) if *b == old => { *b = new; }
                _ => {}
            }
        }
        let old_i = old.0 as usize;
        if old_i < self.reg_holds_handle.len() { self.reg_holds_handle[old_i] = false; }
        let new_i = new.0 as usize;
        if new_i >= self.reg_holds_handle.len() { self.reg_holds_handle.resize(new_i + 1, false); }
        self.reg_holds_handle[new_i] = true;
        if new_i < 128 { self.ever_handle_mask |= 1u128 << new_i; }
        true
    }

    pub(in crate::compiler) fn peephole_copy_drop(&mut self) {
        let mut i = 0;
        while i < self.code.len() {
            if let OpCode::Copy(dest, src) = self.code[i] {
                let src_is_handle = (src.0 as usize) < self.reg_holds_handle.len()
                    && self.reg_holds_handle[src.0 as usize];
                if src_is_handle {
                    let limit = (self.code.len() - i - 1).min(8);
                    let mut drop_pos = None;
                    let mut blocked = false;
                    for j in 1..=limit {
                        match &self.code[i + j] {
                            OpCode::Drop(r) if *r == src => { drop_pos = Some(i + j); break; }
                            op if op_reads_reg(op, src) => { blocked = true; break; }
                            _ => {}
                        }
                    }
                    if let Some(_dp) = drop_pos {
                        if !blocked {
                            self.code[i] = OpCode::Move(dest, src);
                        }
                    }
                }
            }
            i += 1;
        }
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

    // Forget the carried value so region_pop won't force-free it.
    pub(in crate::compiler) fn emit_region_forget_typed(
        &mut self,
        reg: Register,
        ty: &ast::Type,
    ) -> Result<(), String> {
        let is_scalar = matches!(
            ty,
            ast::Type::Named(n) if matches!(
                n.as_str(),
                "Int" | "Float" | "Bool" | "Char" | "Unit" | "Never"
            )
        );
        if is_scalar { return Ok(()); }
        self.emit_region_forget(reg)
    }

    pub(in crate::compiler) fn emit_drops_for_exit(
        &mut self,
        n_blocks: usize,
        skip: Option<Register>,
    ) -> Result<(), String> {
        let total = self.block_locals_stack.len();
        let start = total.saturating_sub(n_blocks);
        for layer_idx in (start..total).rev() {
            let regs: Vec<(Register, bool)> = self.block_locals_stack[layer_idx].clone();
            for (reg, is_handle) in regs {
                if Some(reg) == skip { continue; }
                // a block-local proven scalar at push time needs no Drop.
                if self.drop_elision && !is_handle { continue; }
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

    pub(in crate::compiler) fn patch_jnz_at(&mut self, branch_pc: usize, target_pc: usize) -> Result<(), String> {
        let off = self.rel_offset(target_pc, branch_pc)?;
        if let OpCode::Jnz(r, _) = self.code[branch_pc] {
            self.code[branch_pc] = OpCode::Jnz(r, off);
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
