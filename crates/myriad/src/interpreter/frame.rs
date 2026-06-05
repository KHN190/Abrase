use polka::{BytecodeChunk, Chunk, Register, Module, FRAME_REGS, HANDLE_NONE};
use crate::frame::Frame;
use crate::builtins::NativeCtx;
use super::super::{VirtualMachine, Value};
use super::{MAX_REGISTERS, MAX_RECURSION_DEPTH, STAGE_SLACK};


impl VirtualMachine {
    pub(super) fn do_call(&mut self, module: &Module, caller_bc: &BytecodeChunk, dest: Register, fn_id: usize) -> Result<(), String> {
        if fn_id >= module.functions.len() {
            return Err(format!("call: unknown fn_id {}", fn_id));
        }
        let caller_reg_count = caller_bc.reg_count;
        if dest.to_usize() >= caller_reg_count {
            return Err(format!(
                "call: dest r{} out of caller window (reg_count {})",
                dest.0, caller_reg_count
            ));
        }
        let dest_abs = self.base_reg + dest.to_usize();
        let new_base = self.base_reg + caller_reg_count;
        let needed = new_base + FRAME_REGS + STAGE_SLACK;
        if needed > MAX_REGISTERS {
            return Err(format!(
                "Stack overflow: register window {} exceeds limit {}",
                needed, MAX_REGISTERS
            ));
        }
        self.ensure_registers(needed);

        let chunk = unsafe { module.functions.get_unchecked(fn_id) };
        if let Chunk::Native(n) = chunk {
            if n.name == "__frame_present" {
                self.yielded = true;
                self.yield_dest_abs = dest_abs;
                return Ok(());
            }
            let param_count = n.param_count;
            const MAX_NATIVE_ARGS: usize = 8;
            if param_count > MAX_NATIVE_ARGS {
                return Err(format!("native call: param_count {} exceeds buffer size {}", param_count, MAX_NATIVE_ARGS));
            }
            let func = self.resolved_natives[fn_id].as_ref()
                .ok_or_else(|| format!("native call: fn_id {} not resolved", fn_id))?
                .clone();
            let mut buf: [Value; MAX_NATIVE_ARGS] = [Value::ZERO; MAX_NATIVE_ARGS];
            for i in 0..param_count {
                let raw = self.read_abs_raw(new_base + i);
                buf[i] = Value::from_raw(raw);
            }
            let mut ctx = NativeCtx {
                heap: &mut self.heap,
                devices: &mut self.devices,
                halted: &mut self.halted,
                exit_code: &mut self.exit_code,
            };
            let (result, result_is_handle) = func(&mut ctx, &buf[..param_count])?;
            for i in 0..param_count {
                let abs = new_base + i;
                if self.reg_mask_bit(abs) {
                    let raw = self.read_abs_raw(abs);
                    self.rc_dec_handle(raw)?;
                    self.set_reg_mask_bit(abs, false);
                }
                self.write_abs_raw(abs, HANDLE_NONE);
            }
            self.write_abs(dest_abs, result.raw(), result_is_handle);
            return Ok(());
        }

        if self.frames.len() >= MAX_RECURSION_DEPTH {
            return Err(format!(
                "Stack overflow: recursion depth {} exceeds limit {}",
                self.frames.len(),
                MAX_RECURSION_DEPTH
            ));
        }
        self.frames.push(Frame::normal(self.current_func, self.pc, self.base_reg, dest_abs));
        if !self.handlers.is_empty() {
            self.maybe_mark_body_frame();
        }
        self.base_reg = new_base;
        self.current_func = fn_id;
        self.pc = 0;
        self.trace_frame_event("CALL push", format_args!("func={} dest=r{}", fn_id, dest.0));
        Ok(())
    }

    pub(super) fn do_raise(
        &mut self,
        module: &Module,
        caller_bc: &BytecodeChunk,
        dest: Register,
        key_reg: Register,
        args_base: Register,
    ) -> Result<(), String> {
        let key_raw = self.read_i64(key_reg)?;
        if !(0..=0xFFFFFF).contains(&key_raw) {
            return Err(format!("raise: bad key {}", key_raw));
        }
        let effect_id = ((key_raw >> 8) & 0xFFFF) as u16;
        let op_id = (key_raw & 0xFF) as usize;

        let (arm_fn_id, tail_arm, env) = self.resolve_dispatch_for(effect_id, op_id);
        if arm_fn_id == polka::DISPATCH_NO_MATCH {
            return Err(format!("raise: unhandled effect {:#04x} op {}", effect_id, op_id));
        }
        let arm_fn_id = arm_fn_id as usize;

        let caller_reg_count = caller_bc.reg_count;
        if dest.to_usize() >= caller_reg_count {
            return Err(format!(
                "raise: dest r{} out of caller window (reg_count {})",
                dest.0, caller_reg_count
            ));
        }
        let dest_abs = self.base_reg + dest.to_usize();

        let arm_chunk = module.functions.get(arm_fn_id)
            .ok_or_else(|| format!("raise: bad arm fn_id {}", arm_fn_id))?;
        let (arm_reg_count, arm_param_count) = match arm_chunk {
            Chunk::Bytecode(b) => (b.reg_count, b.param_count),
            Chunk::Native(_) => return Err(format!("raise: arm fn_id {} is native", arm_fn_id)),
        };
        if arm_param_count < 2 {
            return Err(format!("raise: arm fn_id {} param_count {} < 2", arm_fn_id, arm_param_count));
        }
        let nargs = arm_param_count - 2;

        // Op args are MOVED into the operation: take them out of the caller
        // window (before snapshotting, in the suspension path).
        let args_base_abs = self.base_reg + args_base.to_usize();
        let mut moved_args: Vec<(u64, bool)> = Vec::with_capacity(nargs);
        for i in 0..nargs {
            let src_abs = args_base_abs + i;
            let raw = self.read_abs_raw(src_abs);
            let is_handle = self.reg_mask_bit(src_abs);
            moved_args.push((raw, is_handle));
            if is_handle { self.write_abs(src_abs, HANDLE_NONE, false); }
        }

        let (env_raw, env_is_handle) = env.unwrap_or((HANDLE_NONE, false));
        // Tail-resumptive arm (compiled to end in Ret): plain call, the cont
        // cell and register snapshot would never be read — skip them entirely.
        if !tail_arm {
            let mut init_mask = vec![0u64; (crate::cont_slot::SIZE + 63) / 64];
            init_mask[0] = crate::cont_slot::INIT_MASK_WORD0;
            let (cell_slot, cell_gen) = self.checked_heap_alloc_with_mask(crate::cont_slot::SIZE, &init_mask)?;
            self.region_record_alloc(cell_slot, cell_gen);
            self.heap.st(cell_slot, cell_gen, crate::cont_slot::SUSPEND_PC, (self.pc - 1) as u64, false)?;
            self.heap.st(cell_slot, cell_gen, crate::cont_slot::SUSPEND_BASE, self.base_reg as u64, false)?;
            self.heap.st(cell_slot, cell_gen, crate::cont_slot::SUSPEND_FUNC, self.current_func as u64, false)?;
            self.heap.st(cell_slot, cell_gen, crate::cont_slot::ALIVE, 1, false)?;

            let snapshot = self.snapshot_registers(self.base_reg, caller_reg_count)?;
            self.write_snapshot_into_cell(cell_slot, cell_gen, snapshot)?;

            if let Some(handler_frame) = self.handlers.last_mut() {
                handler_frame.cells_allocated.push((cell_slot, cell_gen));
                handler_frame.cell_slot = cell_slot;
                handler_frame.cell_gen = cell_gen;
            }
            self.heap.st(cell_slot, cell_gen, crate::cont_slot::DISPATCH_FN_ID, arm_fn_id as u64, false)?;
            if env_is_handle { self.rc_inc_handle(env_raw)?; }
            self.heap.st(cell_slot, cell_gen, crate::cont_slot::DISPATCH_ENV, env_raw, env_is_handle)?;
        }

        let new_base = self.base_reg + caller_reg_count;
        let window = arm_reg_count.max(polka::FRAME_REGS);
        let needed = new_base + window + STAGE_SLACK;
        if needed > MAX_REGISTERS {
            return Err(format!(
                "Stack overflow: register window {} exceeds limit {}",
                needed, MAX_REGISTERS
            ));
        }
        self.ensure_registers(needed);

        if env_is_handle { self.rc_inc_handle(env_raw)?; }
        self.write_abs(new_base, env_raw, env_is_handle);
        self.write_abs(new_base + 1, 0, false);

        for (i, (raw, is_handle)) in moved_args.into_iter().enumerate() {
            self.write_abs(new_base + 2 + i, raw, is_handle);
        }

        if self.frames.len() >= MAX_RECURSION_DEPTH {
            return Err(format!(
                "Stack overflow: recursion depth {} exceeds limit {}",
                self.frames.len(),
                MAX_RECURSION_DEPTH
            ));
        }
        self.frames.push(Frame::normal(self.current_func, self.pc, self.base_reg, dest_abs));
        self.maybe_mark_body_frame();
        self.base_reg = new_base;
        self.current_func = arm_fn_id;
        self.pc = 0;
        self.trace_frame_event("RAISE", format_args!("eff={:#04x} op={} arm_fn={}", effect_id, op_id, arm_fn_id));
        Ok(())
    }

    pub(super) fn resolve_dispatch_for(&self, effect_id: u16, op_id: usize) -> (u16, bool, Option<(u64, bool)>) {
        for h in self.handlers.iter().rev() {
            if h.effect_id != effect_id { continue; }
            if let Some(slot) = h.dispatch_table_slot {
                if let Ok((raw, _)) = self.heap.ld(slot, h.dispatch_table_gen, op_id * 2) {
                    let tail = (raw & polka::DISPATCH_TAIL_FLAG) != 0;
                    let n = raw & 0xFFFF;
                    if raw & !(polka::DISPATCH_TAIL_FLAG | 0xFFFF) == 0 && (n as u16) != polka::DISPATCH_NO_MATCH {
                        let env = self.heap.ld(slot, h.dispatch_table_gen, op_id * 2 + 1).ok();
                        return (n as u16, tail, env);
                    }
                }
            }
        }
        (polka::DISPATCH_NO_MATCH, false, None)
    }

    #[cold]
    #[inline(never)]
    pub(super) fn maybe_mark_body_frame(&mut self) {
        let new_frame_index = self.frames.len() - 1;
        let handler = self.handlers.last_mut().unwrap();
        if handler.body_frame_index.is_none() {
            handler.body_frame_index = Some(new_frame_index);
        }
    }

    #[inline]
    pub(super) fn do_ret(&mut self, module: &Module, reg: Register) -> Result<(), String> {
        let abs = self.abs(reg);
        let (return_raw, return_is_handle) = self.take_abs(abs);
        let frame = match self.frames.pop() {
            Some(f) => f,
            None => {
                self.write_abs(self.base_reg, return_raw, return_is_handle);
                self.halted = true;
                return Ok(());
            }
        };
        self.trace_frame_event("RET pop",
            format_args!("func={} ret({})={:#x}", frame.func_id,
                if return_is_handle {"handle"} else {"scalar"}, return_raw));

        if self.handlers.is_empty() && frame.cont.is_none() {
            self.pc = frame.ip;
            self.base_reg = frame.base_reg;
            self.current_func = frame.func_id;
            self.write_abs(frame.dest_reg, return_raw, return_is_handle);
            return Ok(());
        }

        self.do_ret_slow(module, frame, return_raw, return_is_handle)
    }

    #[cold]
    #[inline(never)]
    pub(super) fn do_ret_slow(&mut self, module: &Module, frame: super::Frame, return_raw: u64, return_is_handle: bool) -> Result<(), String> {
        let is_body_frame = self.handlers.last()
            .and_then(|h| h.body_frame_index)
            .map_or(false, |idx| idx == self.frames.len());
        let route_through_return_arm = (frame.cont.is_some() || is_body_frame)
            && self.handlers.last().map_or(false, |h| h.pending_return_arm_fn.is_some());

        let is_arm_ret = self.handlers.last()
            .and_then(|h| h.body_frame_index)
            .map_or(false, |idx| self.frames.len() == idx + 1);
        if is_arm_ret && frame.cont.is_none() && return_is_handle && return_raw != HANDLE_NONE {
            let (s, g) = Self::decode_handle(return_raw);
            let tag_is_err = self.heap.ld(s, g, 0).ok()
                .map(|(t, _)| (t as u32) == 1)
                .unwrap_or(false);

            if tag_is_err {
                let body_call_frame = self.frames.pop()
                    .ok_or("arm-throw: missing body-call frame to unwind into")?;
                if let Some(handler) = self.handlers.pop() {
                    handler.release_cells(&mut self.heap, &mut self.region_table)?;
                }
                let inner_base = body_call_frame.base_reg;
                let inner_func = body_call_frame.func_id;
                let inner_reg_count = match module.functions.get(inner_func) {
                    Some(Chunk::Bytecode(b)) => b.reg_count,
                    _ => 0,
                };
                for i in 0..inner_reg_count {
                    let abs = inner_base + i;
                    if self.reg_mask_bit(abs) {
                        let raw = self.read_abs_raw(abs);
                        if raw != HANDLE_NONE && raw != return_raw {
                            self.rc_dec_handle(raw)?;
                        }
                        self.set_reg_mask_bit(abs, false);
                        self.write_abs_raw(abs, HANDLE_NONE);
                    }
                }
                let outer_frame = self.frames.pop()
                    .ok_or("arm-throw: missing outer (handle-expr caller) frame")?;
                self.pc = outer_frame.ip;
                self.base_reg = outer_frame.base_reg;
                self.current_func = outer_frame.func_id;
                let dest_abs = outer_frame.dest_reg;
                if self.reg_mask_bit(dest_abs) {
                    let prior = self.read_abs_raw(dest_abs);
                    if prior != HANDLE_NONE && prior != return_raw {
                        self.rc_dec_handle(prior)?;
                    }
                }
                self.write_abs(dest_abs, return_raw, true);
                return Ok(());
            }
        }

        if !route_through_return_arm {
            if let Some(cont) = frame.cont.as_ref() {
                if cont.snapshot_count > 0 {
                    let snap = crate::snapshot::SnapshotHandle {
                        slot: cont.snapshot_slot,
                        generation: cont.snapshot_gen,
                        count: cont.snapshot_count,
                    };
                    self.restore_registers(frame.base_reg, snap)?;
                    self.heap.rc_dec(snap.slot, snap.generation)?;
                }
            }
            self.pc = frame.ip;
            self.base_reg = frame.base_reg;
            self.current_func = frame.func_id;
            self.write_abs(frame.dest_reg, return_raw, return_is_handle);
            return Ok(());
        }

        if let Some(cont) = frame.cont.as_ref() {
            if cont.snapshot_count > 0 {
                let snap = crate::snapshot::SnapshotHandle {
                    slot: cont.snapshot_slot,
                    generation: cont.snapshot_gen,
                    count: cont.snapshot_count,
                };
                self.restore_registers(frame.base_reg, snap)?;
                self.heap.rc_dec(snap.slot, snap.generation)?;
            }
        }

        let (ra_fn, ra_env_raw, ra_env_is_handle) = {
            let h = self.handlers.last_mut().unwrap();
            let fn_id = h.pending_return_arm_fn.take().unwrap();
            let env = std::mem::replace(&mut h.pending_return_arm_env, HANDLE_NONE);
            let env_h = std::mem::replace(&mut h.pending_return_arm_env_is_handle, false);
            (fn_id, env, env_h)
        };

        self.frames.push(super::Frame::normal(
            frame.func_id, frame.ip, frame.base_reg, frame.dest_reg,
        ));

        let caller_reg_count = match module.functions.get(frame.func_id) {
            Some(Chunk::Bytecode(b)) => b.reg_count,
            _ => return Err(format!("return-arm: popped frame func {} is not bytecode", frame.func_id)),
        };
        let callee_reg_count = match module.functions.get(ra_fn) {
            Some(Chunk::Bytecode(b)) => b.reg_count,
            Some(Chunk::Native(_)) => return Err("return-arm: native return arm not supported".into()),
            None => return Err(format!("return-arm: unknown fn_id {}", ra_fn)),
        };
        let new_base = frame.base_reg + caller_reg_count;
        let window = callee_reg_count.max(polka::FRAME_REGS);
        let needed = new_base + window + STAGE_SLACK;
        if needed > MAX_REGISTERS {
            return Err(format!(
                "Stack overflow setting up return arm: window {} exceeds limit {}",
                needed, MAX_REGISTERS
            ));
        }
        self.ensure_registers(needed);

        if ra_env_is_handle { self.rc_inc_handle(ra_env_raw)?; }
        self.write_abs(new_base, ra_env_raw, ra_env_is_handle);
        self.write_abs(new_base + 1, 0, false);
        self.write_abs(new_base + 2, return_raw, return_is_handle);

        self.base_reg = new_base;
        self.current_func = ra_fn;
        self.pc = 0;
        Ok(())
    }
}
