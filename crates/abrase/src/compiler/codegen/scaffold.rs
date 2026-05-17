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
        Ok(reg)
    }

    pub(in crate::compiler) fn emit(&mut self, op: OpCode) {
        self.code.push(op);
    }

    // Peephole: if the last emitted op writes to `old`, retarget it to `new`.
    pub(in crate::compiler) fn try_redirect_last_dest(&mut self, old: Register, new: Register) -> bool {
        if old == new { return true; }
        let Some(last) = self.code.last_mut() else { return false; };
        let d: &mut Register = match last {
            OpCode::Add(d, _, _) | OpCode::Sub(d, _, _) | OpCode::Mul(d, _, _) |
            OpCode::Div(d, _, _) | OpCode::Mod(d, _, _) | OpCode::Neg(d, _) |
            OpCode::FAdd(d, _, _) | OpCode::FSub(d, _, _) | OpCode::FMul(d, _, _) | OpCode::FDiv(d, _, _) |
            OpCode::Eq(d, _, _) | OpCode::Neq(d, _, _) |
            OpCode::Lt(d, _, _) | OpCode::Gt(d, _, _) | OpCode::Lte(d, _, _) | OpCode::Gte(d, _, _) |
            OpCode::FLt(d, _, _) |
            OpCode::And(d, _, _) | OpCode::Or(d, _, _) | OpCode::Xor(d, _, _) |
            OpCode::Shl(d, _, _) | OpCode::Shr(d, _, _) |
            OpCode::PushConst(d, _) |
            OpCode::Copy(d, _) | OpCode::Move(d, _) |
            OpCode::Ld(d, _, _) | OpCode::LdIdx(d, _, _) |
            OpCode::Ref(d, _) |
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

    pub(in crate::compiler) fn add_constant(&mut self, val: Value) -> Result<u16, String> {
        if let Some(i) = self.constants.iter().position(|c| *c == val) {
            return Ok(i as u16);
        }
        if self.constants.len() >= u16::MAX as usize {
            return Err("Constant pool overflow (max 65535 entries)".to_string());
        }
        self.constants.push(val);
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
        self.add_constant(Value::from_str_const(str_idx))
    }

    // Region push/pop. Tracked in `compiler_region_depth` regardless of who
    // opened the region — user-written `region { ... }` blocks count the same
    // as compiler-inserted ones (loop bodies, statement-position blocks).
    // break/continue/return/throw unwind ALL of them by depth diff, so
    // `loop { region { return ... } }` properly pops the user region too.
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

    // Bare pop that doesn't lower the static depth counter. Used by
    // emit_region_pops_for_exit when the calling control-flow form (break,
    // return, throw, `?`) won't fall through — subsequent codegen still sees
    // itself at the pre-exit depth.
    fn emit_region_pop_unaccounted(&mut self) -> Result<(), String> {
        self.emit_region_marker(crate::bytecode::REGION_PORT_POP)
    }

    // Emit N region pops without changing the static depth tracker. Used by
    // break/continue/return/throw — those instructions don't fall through, so
    // subsequent codegen still thinks of itself at the pre-exit depth.
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

    // Emit a region_forget for `reg`: if at runtime `reg` holds a heap handle,
    // strip it from every active region's record list so the subsequent
    // region_pop force-frees don't reclaim it. No-op for non-handle values.
    // Call this BEFORE emit_region_pops_for_exit on any value being carried
    // out by break/return/throw.
    pub(in crate::compiler) fn emit_region_forget(&mut self, reg: Register) -> Result<(), String> {
        let port_val =
            ((crate::bytecode::REGION_ID as i64) << 8) | (crate::bytecode::REGION_PORT_FORGET as i64);
        let port_idx = self.add_constant(Value::from_int(port_val))?;
        let port_reg = self.alloc_register()?;
        self.emit(OpCode::PushConst(port_reg, port_idx));
        self.emit(OpCode::Deo(reg, port_reg));
        Ok(())
    }

    // Walk a carried value's type structure and emit region_forget for the
    // outer cell plus any heap handles reachable through fields. Records and
    // tuples have known layout — load each field and recurse. Variants do
    // a single-level forget (their tag is only known at runtime; per-case
    // recursive forget would need a tag-switch — deferred). Closures /
    // functions: single-level. Recursive variants (List, Tree) terminate
    // here too — they hit MAX_DEPTH and surface a compile error rather than
    // overflowing the host stack.
    //
    // ORDER INVARIANT (applies to every abnormal exit site):
    //   1. emit_region_forget_typed(carried_reg, &carried_ty)
    //   2. emit_drops_for_exit(blocks_to_unwind, skip = Some(carried_reg))
    //   3. emit_region_pops_for_exit(regions_to_unwind)
    //   4. Jmp / Ret
    // Drops MUST come before pops: pop force-frees cells, after which the
    // Drop's rc_dec would observe a freed slot and err.
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
            // Scalars: not heap, nothing to forget.
            ast::Type::Named(n) if matches!(
                n.as_str(),
                "Int" | "Float" | "Bool" | "Char" | "Unit" | "Never"
            ) => Ok(()),

            // Single-level forget — handle wrapper, but inner not walked.
            ast::Type::Reference { .. }
            | ast::Type::Function { .. } => {
                self.emit_region_forget(reg)
            }

            // Tuples: known offsets. Forget outer, then Ld each field, recurse.
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

            // Named user types (record / variant): single-level forget for
            // now. Per-field recursion requires keeping field TYPES (not just
            // names) on `LayoutCtx`; deferred. The outer cell survives, but
            // inner heap cells reachable through fields will be force-freed
            // by region_pop. A user carrying a record-of-refs out of a
            // region currently risks stale handles inside the record —
            // typeck's `is_heap_typed` still rejects that path.
            ast::Type::Named(_) => self.emit_region_forget(reg),

            // Generics (Array<T>, Vec<T>, etc.): outer-only for now —
            // per-element forget needs runtime length probing.
            ast::Type::Generic { .. } => {
                self.emit_region_forget(reg)
            }

            // Anything else (unknown / future): conservative single forget.
            _ => self.emit_region_forget(reg),
        }
    }

    // Emit OpCode::Drop for every binder reg in the top `n` block scopes,
    // skipping `skip` if Some (the carried value being escaped). Used by
    // every abnormal exit AND by the natural end-of-block path to keep
    // both routes flowing the same Drops.
    pub(in crate::compiler) fn emit_drops_for_exit(
        &mut self,
        n_blocks: usize,
        skip: Option<Register>,
    ) -> Result<(), String> {
        let total = self.block_locals_stack.len();
        let start = total.saturating_sub(n_blocks);
        // iterate top n layers (start..total) WITHOUT removing them
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
