use super::{VirtualMachine, Value};
use super::debug::DebugEvent;
use polka::{BytecodeChunk, Chunk, OpCode, Register, Module, FRAME_REGS, HANDLE_NONE};
use crate::frame::Frame;
use crate::memory::mask_bit;
use crate::value::alloc_string;

const MAX_REGISTERS: usize = 1 << 16;
const MAX_RECURSION_DEPTH: usize = 2048;
// Slack for materializing param Moves before Call opcode (see stage_call_args).
const STAGE_SLACK: usize = 32;

pub const MAX_RAM: usize = 64 * 1024 * 1024;

impl VirtualMachine {
    pub fn run(&mut self, chunk: &Chunk) -> Result<Value, String> {
        let module = Module { functions: vec![chunk.clone()], entry: 0 };
        self.run_module(&module)
    }

    pub fn run_module(&mut self, module: &Module) -> Result<Value, String> {
        let r = self.run_module_inner(module);
        r.map_err(|e| format!(
            "[{}:{}] {}",
            super::debug::render_fn_label(self.current_func, &self.fn_names),
            self.failing_pc, e,
        ))
    }

    fn run_module_inner(&mut self, module: &Module) -> Result<Value, String> {
        validate_module_register_budget(module)?;
        self.frames.clear();
        self.handlers.clear();
        self.region_table.clear();
        self.heap.clear();
        self.string_const_handles.clear();
        self.resolve_constants(module)?;
        self.pc = 0;
        self.base_reg = 0;
        self.current_func = module.entry;
        self.halted = false;
        self.exit_code = None;
        let needed = FRAME_REGS + STAGE_SLACK;
        self.ensure_registers(needed);

        if self.debug_sink.is_some() {
            self.run_loop::<true>(module)
        } else {
            self.run_loop::<false>(module)
        }
    }

    fn run_loop<const TRACE: bool>(&mut self, module: &Module) -> Result<Value, String> {
        'outer: loop {
            if self.halted {
                if let Some(code) = self.exit_code {
                    return Ok(Value::from_int(code));
                }
                let v = self.read_abs_raw(self.base_reg);
                return Ok(Value::from_raw(v));
            }
            debug_assert!(self.current_func < module.functions.len());

            let bc = match unsafe { module.functions.get_unchecked(self.current_func) } {
                Chunk::Bytecode(b) => b,
                Chunk::Native(_) => return Err(format!(
                    "entry fn {} is native; cannot start execution there", self.current_func
                )),
            };
            let entry_func = self.current_func;
            loop {
                if self.pc >= bc.code.len() {
                    if let Some(frame) = self.frames.pop() {
                        let return_raw = self.read_abs_raw(self.base_reg);
                        let return_is_handle = self.reg_mask_bit(self.base_reg);
                        self.pc = frame.ip;
                        self.base_reg = frame.base_reg;
                        self.current_func = frame.func_id;
                        self.write_abs(frame.dest_reg, return_raw, return_is_handle);
                        continue 'outer;
                    } else {
                        let v = self.read_abs_raw(self.base_reg);
                        return Ok(Value::from_raw(v));
                    }
                }
                let opcode_pc = self.pc;
                let opcode = unsafe { bc.code.get_unchecked(opcode_pc) };
                if TRACE {
                    let event = DebugEvent::Trace {
                        func: self.current_func, pc: opcode_pc, op: opcode,
                    };
                    self.emit_debug(&event);
                }
                self.pc = opcode_pc + 1;
                if let Err(e) = self.exec(module, bc, opcode) {
                    self.failing_pc = opcode_pc;
                    return Err(e);
                }
                if self.halted || self.current_func != entry_func {
                    continue 'outer;
                }
            }
        }
    }

    #[inline(always)]
    fn exec(&mut self, module: &Module, bc: &BytecodeChunk, op: &OpCode) -> Result<(), String> {
        match op {
            OpCode::Add(d, a, b)  => self.bin_i64(*d, *a, *b, |x, y| x.wrapping_add(y)),
            OpCode::Sub(d, a, b)  => self.bin_i64(*d, *a, *b, |x, y| x.wrapping_sub(y)),
            OpCode::Mul(d, a, b)  => self.bin_i64(*d, *a, *b, |x, y| x.wrapping_mul(y)),
            OpCode::Div(d, a, b)  => self.bin_i64_checked(*d, *a, *b, "div by zero", |x, y| x.checked_div(y)),
            OpCode::Mod(d, a, b)  => self.bin_i64_checked(*d, *a, *b, "mod by zero", |x, y| x.checked_rem(y)),
            OpCode::Neg(d, a)     => {
                let v = self.read_i64(*a)?;
                self.write(*d, v.wrapping_neg() as u64, false)
            }

            OpCode::FAdd(d, a, b) => self.bin_f64(*d, *a, *b, |x, y| x + y),
            OpCode::FSub(d, a, b) => self.bin_f64(*d, *a, *b, |x, y| x - y),
            OpCode::FMul(d, a, b) => self.bin_f64(*d, *a, *b, |x, y| x * y),
            OpCode::FDiv(d, a, b) => self.bin_f64(*d, *a, *b, |x, y| x / y),
            OpCode::FNeg(d, a)    => {
                let x = self.read_f64(*a)?;
                self.write(*d, f64::to_bits(-x), false)
            }
            OpCode::FLt(d, a, b)  => {
                let x = self.read_f64(*a)?;
                let y = self.read_f64(*b)?;
                let r = if x.is_nan() || y.is_nan() { false } else { x < y };
                self.write(*d, bool_u64(r), false)
            }
            OpCode::FEq(d, a, b)  => {
                let x = self.read_f64(*a)?;
                let y = self.read_f64(*b)?;
                let r = if x.is_nan() || y.is_nan() { false } else { x == y };
                self.write(*d, bool_u64(r), false)
            }

            OpCode::Eq(d, a, b)  => self.bin_eq(*d, *a, *b, false),
            OpCode::Neq(d, a, b) => self.bin_eq(*d, *a, *b, true),
            OpCode::Lt(d, a, b)  => self.bin_i64_cmp(*d, *a, *b, |x, y| x < y),
            OpCode::Gt(d, a, b)  => self.bin_i64_cmp(*d, *a, *b, |x, y| x > y),
            OpCode::Lte(d, a, b) => self.bin_i64_cmp(*d, *a, *b, |x, y| x <= y),
            OpCode::Gte(d, a, b) => self.bin_i64_cmp(*d, *a, *b, |x, y| x >= y),

            OpCode::And(d, a, b) => self.bin_i64(*d, *a, *b, |x, y| x & y),
            OpCode::Or(d, a, b)  => self.bin_i64(*d, *a, *b, |x, y| x | y),
            OpCode::Xor(d, a, b) => self.bin_i64(*d, *a, *b, |x, y| x ^ y),
            OpCode::Shl(d, a, b) => self.bin_i64(*d, *a, *b, |x, y| x.wrapping_shl((y as u32) & 63)),
            OpCode::Shr(d, a, b) => self.bin_i64(*d, *a, *b, |x, y| x.wrapping_shr((y as u32) & 63)),

            OpCode::Jmp(off) => self.branch(bc, *off),
            OpCode::Jz(r, off) => {
                let v = self.read_raw(*r)?;
                if v == 0 { self.branch(bc, *off) } else { Ok(()) }
            }
            OpCode::Jnz(r, off) => {
                let v = self.read_raw(*r)?;
                if v != 0 { self.branch(bc, *off) } else { Ok(()) }
            }
            OpCode::Call(dest, fn_id) => self.do_call(module, bc, *dest, *fn_id as usize),
            OpCode::CallReg(dest, fn_id_reg) => {
                let fn_id = self.read_i64(*fn_id_reg)?;
                if !(0..=0xFFFF).contains(&fn_id) {
                    return Err(format!("call_reg: fn_id {} out of u16 range", fn_id));
                }
                self.do_call(module, bc, *dest, fn_id as usize)
            }
            OpCode::Ret(reg) => self.do_ret(module, *reg),

            OpCode::PushConst(reg, pool_idx) => {
                let idx = *pool_idx as usize;
                let consts = &self.resolved_constants[self.current_func];
                let mask  = &self.resolved_const_mask[self.current_func];
                if idx >= consts.len() {
                    return Err("Constant index out of bounds".to_string());
                }
                let raw = consts[idx];
                let is_handle = mask_bit(mask, idx);
                if is_handle { self.rc_inc_handle(raw)?; }
                self.write(*reg, raw, is_handle)
            }
            OpCode::Copy(d, s) => {
                let (v, is_handle) = self.read(*s)?;
                if is_handle { self.rc_inc_handle(v)?; }
                self.write(*d, v, is_handle)
            }
            OpCode::Move(d, s) => {
                let (v, is_handle) = self.take(*s)?;
                self.write(*d, v, is_handle)
            }

            OpCode::Ld(d, b, off) => {
                let (slot, gen_) = self.read_handle(*b)?;
                let (raw, is_handle) = self.heap.ld(slot, gen_, *off as usize)?;
                if is_handle { self.rc_inc_handle(raw)?; }
                self.write(*d, raw, is_handle)
            }
            OpCode::St(src, b, off) => {
                let (slot, gen_) = self.read_handle(*b)?;
                let (raw, is_handle) = self.take(*src)?;
                let (old_raw, old_is_handle) = self.heap.st(slot, gen_, *off as usize, raw, is_handle)?;
                if old_is_handle { self.rc_dec_handle(old_raw)?; }
                Ok(())
            }
            OpCode::LdIdx(d, b, i) => {
                let (slot, gen_) = self.read_handle(*b)?;
                let off = self.read_i64(*i)?;
                if off < 0 {
                    return Err(format!("ldidx: negative index {}", off));
                }
                let (raw, is_handle) = self.heap.ld(slot, gen_, off as usize)?;
                if is_handle { self.rc_inc_handle(raw)?; }
                self.write(*d, raw, is_handle)
            }
            OpCode::StIdx(src, b, i) => {
                let (slot, gen_) = self.read_handle(*b)?;
                let off = self.read_i64(*i)?;
                if off < 0 {
                    return Err(format!("stidx: negative index {}", off));
                }
                let (raw, is_handle) = self.take(*src)?;
                let (old_raw, old_is_handle) = self.heap.st(slot, gen_, off as usize, raw, is_handle)?;
                if old_is_handle { self.rc_dec_handle(old_raw)?; }
                Ok(())
            }
            OpCode::AddImm(d, s, imm) => {
                let x = self.read_i64(*s)?;
                self.write(*d, x.wrapping_add(*imm as i64) as u64, false)
            }
            OpCode::SubImm(d, s, imm) => {
                let x = self.read_i64(*s)?;
                self.write(*d, x.wrapping_sub(*imm as i64) as u64, false)
            }

            OpCode::Alloc(d, size) => {
                let (slot, generation) = self.checked_heap_alloc(*size as usize)?;
                self.region_record_alloc(slot, generation);
                let handle = Value::from_handle(slot, generation).raw();
                self.write(*d, handle, true)
            }
            OpCode::Drop(reg) => {
                let abs = self.abs(*reg);
                let (v, is_handle) = self.take_abs(abs);
                if is_handle { self.rc_dec_handle(v)?; }
                Ok(())
            }

            OpCode::Dei(d, port_reg) => self.do_dei(*d, *port_reg),
            OpCode::Deo(src, port_reg) => self.do_deo(module, *src, *port_reg),
            OpCode::Handle(table_reg, effect_id) => {
                let (table_raw, table_is_handle) = self.read_at(*table_reg);
                let (table_slot, table_gen) = if table_is_handle && table_raw != HANDLE_NONE {
                    let (s, g) = Self::decode_handle(table_raw);
                    (Some(s), g)
                } else { (None, 0) };
                self.handlers.push(super::HandlerFrame {
                    effect_id: *effect_id,
                    dispatch_table_slot: table_slot,
                    dispatch_table_gen: table_gen,
                    cell_slot: 0,
                    cell_gen: 0,
                    cells_allocated: Vec::new(),
                    body_frame_index: None,
                    pending_return_arm_fn: None,
                    pending_return_arm_env: HANDLE_NONE,
                    pending_return_arm_env_is_handle: false,
                });
                Ok(())
            }
            OpCode::Resume(dest_reg, val_reg) => self.do_resume(module, *dest_reg, *val_reg),
            OpCode::Raise(dest, key_reg, args_base) => self.do_raise(module, bc, *dest, *key_reg, *args_base),
        }
    }

    fn do_call(&mut self, module: &Module, caller_bc: &BytecodeChunk, dest: Register, fn_id: usize) -> Result<(), String> {
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
            let mut ctx = super::NativeCtx {
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
        Ok(())
    }

    fn do_raise(
        &mut self,
        module: &Module,
        caller_bc: &BytecodeChunk,
        dest: Register,
        key_reg: Register,
        args_base: Register,
    ) -> Result<(), String> {
        let key_raw = self.read_i64(key_reg)?;
        if !(0..=0xFFFF).contains(&key_raw) {
            return Err(format!("raise: bad key {}", key_raw));
        }
        let key = key_raw as u16;
        let effect_id = (key >> 8) as u16;
        let op_id = (key & 0xFF) as usize;

        let (arm_fn_id, env) = self.resolve_dispatch_for(effect_id, op_id);
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

        let mut init_mask = vec![0u64; (super::cont_slot::SIZE + 63) / 64];
        init_mask[0] = super::cont_slot::INIT_MASK_WORD0;
        let (cell_slot, cell_gen) = self.checked_heap_alloc_with_mask(super::cont_slot::SIZE, &init_mask)?;
        self.region_record_alloc(cell_slot, cell_gen);
        self.heap.st(cell_slot, cell_gen, super::cont_slot::SUSPEND_PC, (self.pc - 1) as u64, false)?;
        self.heap.st(cell_slot, cell_gen, super::cont_slot::SUSPEND_BASE, self.base_reg as u64, false)?;
        self.heap.st(cell_slot, cell_gen, super::cont_slot::SUSPEND_FUNC, self.current_func as u64, false)?;
        self.heap.st(cell_slot, cell_gen, super::cont_slot::ALIVE, 1, false)?;

        let snapshot = self.snapshot_registers(self.base_reg, caller_reg_count)?;
        self.write_snapshot_into_cell(cell_slot, cell_gen, snapshot)?;

        if let Some(handler_frame) = self.handlers.last_mut() {
            handler_frame.cells_allocated.push((cell_slot, cell_gen));
            handler_frame.cell_slot = cell_slot;
            handler_frame.cell_gen = cell_gen;
        }
        self.heap.st(cell_slot, cell_gen, super::cont_slot::DISPATCH_FN_ID, arm_fn_id as u64, false)?;
        let (env_raw, env_is_handle) = env.unwrap_or((HANDLE_NONE, false));
        if env_is_handle { self.rc_inc_handle(env_raw)?; }
        self.heap.st(cell_slot, cell_gen, super::cont_slot::DISPATCH_ENV, env_raw, env_is_handle)?;

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

        let args_base_abs = self.base_reg + args_base.to_usize();
        for i in 0..nargs {
            let src_abs = args_base_abs + i;
            let raw = self.read_abs_raw(src_abs);
            let is_handle = self.reg_mask_bit(src_abs);
            if is_handle { self.rc_inc_handle(raw)?; }
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
        Ok(())
    }

    fn resolve_dispatch_for(&self, effect_id: u16, op_id: usize) -> (u16, Option<(u64, bool)>) {
        for h in self.handlers.iter().rev() {
            if h.effect_id != effect_id { continue; }
            if let Some(slot) = h.dispatch_table_slot {
                if let Ok((raw, _)) = self.heap.ld(slot, h.dispatch_table_gen, op_id * 2) {
                    let n = raw as i64;
                    if (0..=0xFFFF).contains(&n) && (n as u16) != polka::DISPATCH_NO_MATCH {
                        let env = self.heap.ld(slot, h.dispatch_table_gen, op_id * 2 + 1).ok();
                        return (n as u16, env);
                    }
                }
            }
        }
        (polka::DISPATCH_NO_MATCH, None)
    }

    #[cold]
    #[inline(never)]
    fn maybe_mark_body_frame(&mut self) {
        let new_frame_index = self.frames.len() - 1;
        let handler = self.handlers.last_mut().unwrap();
        if handler.body_frame_index.is_none() {
            handler.body_frame_index = Some(new_frame_index);
        }
    }

    #[inline]
    fn do_ret(&mut self, module: &Module, reg: Register) -> Result<(), String> {
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
    fn do_ret_slow(&mut self, module: &Module, frame: super::Frame, return_raw: u64, return_is_handle: bool) -> Result<(), String> {
        let is_body_frame = self.handlers.last()
            .and_then(|h| h.body_frame_index)
            .map_or(false, |idx| idx == self.frames.len());
        let route_through_return_arm = (frame.cont.is_some() || is_body_frame)
            && self.handlers.last().map_or(false, |h| h.pending_return_arm_fn.is_some());

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

    fn do_dei(&mut self, d: Register, port_reg: Register) -> Result<(), String> {
        let port_val = self.read_i64(port_reg)?;
        let (device_id, port) = split_port(port_val)?;
        match (device_id, port) {
            (polka::DISPATCH_ID, polka::DISPATCH_PORT_LOOKUP) => {
                let (raw, is_handle) = if let Some(frame) = self.handlers.last() {
                    self.heap.ld(frame.cell_slot, frame.cell_gen, super::cont_slot::DISPATCH_FN_ID)?
                } else {
                    (self.dispatch_last_result.take().unwrap_or(0xFFFF) as u64, false)
                };
                self.write(d, raw, is_handle)
            }
            (polka::DISPATCH_ID, polka::DISPATCH_PORT_ENV) => {
                let (raw, is_handle) = if let Some(frame) = self.handlers.last() {
                    self.heap.ld(frame.cell_slot, frame.cell_gen, super::cont_slot::DISPATCH_ENV)?
                } else {
                    self.dispatch_last_env.take().unwrap_or((HANDLE_NONE, false))
                };
                let (raw, is_handle) = if raw == HANDLE_NONE && !is_handle { (0u64, false) } else { (raw, is_handle) };
                if is_handle { self.rc_inc_handle(raw)?; }
                self.write(d, raw, is_handle)
            }
            _ => {
                let dev = self.devices.get_mut(device_id)
                    .ok_or_else(|| format!("dei: device {:#04x} not installed", device_id))?;
                let v = dev.read(port)?;
                self.write(d, v.raw(), false)
            }
        }
    }

    fn do_deo(&mut self, module: &Module, src: Register, port_reg: Register) -> Result<(), String> {
        let (raw, is_handle) = self.read(src)?;
        let port_val = self.read_i64(port_reg)?;
        let (device_id, port) = split_port(port_val)?;
        match (device_id, port) {
            (0x00, 0x01) => {
                if !is_handle { self.exit_code = Some((raw as i64) & 0xFFFF_FFFF); }
                self.halted = true;
                Ok(())
            }
            (0x00, 0x02) => {
                let msg = if is_handle && raw != HANDLE_NONE {
                    let v = Value::from_raw(raw);
                    crate::value::read_string(&self.heap, v).unwrap_or_else(|| format!("(handle {:?})", v))
                } else {
                    format!("{}", raw as i64)
                };
                Err(format!("panic: {}", msg))
            }
            (polka::DISPATCH_ID, polka::DISPATCH_PORT_LOOKUP) => {
                let key = decode_dispatch_key(raw)?;
                let mut init_mask = vec![0u64; (super::cont_slot::SIZE + 63) / 64];
                init_mask[0] = super::cont_slot::INIT_MASK_WORD0;
                let (cell_slot, cell_gen) = self.checked_heap_alloc_with_mask(super::cont_slot::SIZE, &init_mask)?;
                self.region_record_alloc(cell_slot, cell_gen);
                self.heap.st(cell_slot, cell_gen, super::cont_slot::SUSPEND_PC, (self.pc - 1) as u64, false)?;
                self.heap.st(cell_slot, cell_gen, super::cont_slot::SUSPEND_BASE, self.base_reg as u64, false)?;
                self.heap.st(cell_slot, cell_gen, super::cont_slot::SUSPEND_FUNC, self.current_func as u64, false)?;
                self.heap.st(cell_slot, cell_gen, super::cont_slot::ALIVE, 1, false)?;

                let reg_count = self.current_fn_reg_count(module);
                let snapshot = self.snapshot_registers(self.base_reg, reg_count)?;
                self.write_snapshot_into_cell(cell_slot, cell_gen, snapshot)?;

                if let Some(handler_frame) = self.handlers.last_mut() {
                    handler_frame.cells_allocated.push((cell_slot, cell_gen));
                    handler_frame.cell_slot = cell_slot;
                    handler_frame.cell_gen = cell_gen;
                }
                let (fn_id, env) = self.resolve_dispatch(key);
                self.heap.st(cell_slot, cell_gen, super::cont_slot::DISPATCH_FN_ID, fn_id as u64, false)?;
                let (env_raw, env_is_handle) = env.unwrap_or((HANDLE_NONE, false));
                if env_is_handle { self.rc_inc_handle(env_raw)?; }
                self.heap.st(cell_slot, cell_gen, super::cont_slot::DISPATCH_ENV, env_raw, env_is_handle)?;
                self.dispatch_last_result = Some(fn_id);
                self.dispatch_last_env = env;
                Ok(())
            }
            (polka::DISPATCH_ID, polka::DISPATCH_PORT_POP_HANDLER) => {
                let frame = self.handlers.pop()
                    .ok_or("dispatch.pop_handler: no active handler frame")?;
                for (slot, generation) in frame.cells_allocated.iter() {
                    self.region_table.forget(*slot, *generation);
                    if self.heap.is_live(*slot, *generation) {
                        self.heap.rc_dec(*slot, *generation)?;
                    }
                }
                Ok(())
            }
            (polka::DISPATCH_ID, polka::DISPATCH_PORT_RETURN_FN) => {
                let fn_id = raw as i64;
                let handler = self.handlers.last_mut()
                    .ok_or("dispatch.return_fn: no active handler frame")?;
                handler.pending_return_arm_fn = Some(fn_id as usize);
                Ok(())
            }
            (polka::DISPATCH_ID, polka::DISPATCH_PORT_RETURN_ENV) => {
                let handler = self.handlers.last_mut()
                    .ok_or("dispatch.return_env: no active handler frame")?;
                handler.pending_return_arm_env = raw;
                handler.pending_return_arm_env_is_handle = is_handle;
                Ok(())
            }
            (polka::REGION_ID, polka::REGION_PORT_PUSH) => { self.region_push(); Ok(()) }
            (polka::REGION_ID, polka::REGION_PORT_POP)  => self.region_pop(),
            (polka::REGION_ID, polka::REGION_PORT_FORGET) => {
                if is_handle && raw != HANDLE_NONE {
                    let (slot, gen_) = Self::decode_handle(raw);
                    let mut visited: std::collections::HashSet<(u32, u32)> = std::collections::HashSet::new();
                    self.deep_forget(slot, gen_, &mut visited)?;
                }
                Ok(())
            }
            (polka::REGION_ID, _) => Err(format!("region: unknown port {:#x}", port)),
            _ => {
                let dev = self.devices.get_mut(device_id)
                    .ok_or_else(|| format!("deo: device {:#04x} not installed", device_id))?;
                dev.write(port, Value::from_raw(raw))
            }
        }
    }

    pub(crate) fn deep_forget(
        &mut self,
        slot: u32,
        generation: u32,
        visited: &mut std::collections::HashSet<(u32, u32)>,
    ) -> Result<(), String> {
        if !visited.insert((slot, generation)) { return Ok(()); }
        if !self.region_table.forget(slot, generation) { return Ok(()); }
        let size = match self.heap.size(slot, generation) {
            Ok(n) => n,
            Err(_) => return Ok(()),
        };
        for off in 0..size {
            if let Ok((raw, is_handle)) = self.heap.ld(slot, generation, off) {
                if is_handle && raw != HANDLE_NONE {
                    let (s, g) = Self::decode_handle(raw);
                    self.deep_forget(s, g, visited)?;
                }
            }
        }
        Ok(())
    }

    fn do_resume(&mut self, module: &Module, dest_reg: Register, val_reg: Register) -> Result<(), String> {
        let (cell_slot, cell_gen) = {
            let h = self.handlers.last()
                .ok_or("Resume outside an active handler frame")?;
            (h.cell_slot, h.cell_gen)
        };
        let (alive_raw, saved_base, saved_func, suspended_snapshot) = {
            let data = self.heap.cell_data(cell_slot, cell_gen)?;
            let mask = self.heap.cell_mask(cell_slot, cell_gen)?;
            debug_assert!(data.len() >= super::cont_slot::SIZE);
            let alive = data[super::cont_slot::ALIVE];
            let saved_base = data[super::cont_slot::SUSPEND_BASE] as usize;
            let saved_func = data[super::cont_slot::SUSPEND_FUNC] as usize;
            let snap_raw = data[super::cont_slot::REGS_SNAPSHOT_SLOT];
            let snap_is_handle = mask_bit(mask, super::cont_slot::REGS_SNAPSHOT_SLOT);
            let snap = if !snap_is_handle || snap_raw == HANDLE_NONE {
                crate::snapshot::SnapshotHandle::EMPTY
            } else {
                let (slot, generation) = Self::decode_handle(snap_raw);
                let count = data[super::cont_slot::REGS_COUNT] as usize;
                crate::snapshot::SnapshotHandle { slot, generation, count }
            };
            (alive, saved_base, saved_func, snap)
        };
        match alive_raw {
            1 => {}
            0 => return Err("resume: continuation already consumed".to_string()),
            n => return Err(format!("resume: continuation cell corrupted (alive slot = {})", n)),
        }

        self.heap.st(cell_slot, cell_gen, super::cont_slot::ALIVE, 0, false)?;

        let (val_raw, val_is_handle) = self.read(val_reg)?;

        let arm_call_frame = self.frames.pop()
            .ok_or("resume: no arm-call frame on stack")?;
        let yield_result_abs = arm_call_frame.dest_reg;
        let resume_ip = arm_call_frame.ip;

        let arm_reg_count = self.current_fn_reg_count(module);
        let arm_snapshot = self.snapshot_registers_off_region(self.base_reg, arm_reg_count)?;

        let arm_resume_dest_abs = self.base_reg + dest_reg.to_usize();
        let insert_at = {
            let body_idx = self.handlers.last().and_then(|h| h.body_frame_index)
                .ok_or("resume: active handler has no body frame index")?;
            let mut i = body_idx + 1;
            while i < self.frames.len() && self.frames[i].is_arm_continuation() {
                i += 1;
            }
            i
        };
        let new_frame = super::Frame::arm_continuation(
            self.current_func,
            self.pc,
            self.base_reg,
            arm_resume_dest_abs,
            arm_snapshot.slot,
            arm_snapshot.generation,
            arm_snapshot.count,
        );
        if insert_at == self.frames.len() {
            self.frames.push(new_frame);
        } else {
            self.frames.insert(insert_at, new_frame);
        }

        self.base_reg = saved_base;
        self.current_func = saved_func;
        self.pc = resume_ip;
        self.restore_registers(saved_base, suspended_snapshot)?;

        if !suspended_snapshot.is_empty() {
            self.heap.st(cell_slot, cell_gen, super::cont_slot::REGS_SNAPSHOT_SLOT, HANDLE_NONE, true)?;
            self.heap.rc_dec(suspended_snapshot.slot, suspended_snapshot.generation)?;
        }

        if yield_result_abs >= self.registers.len() {
            return Err(format!(
                "resume: yield dest abs {} out of registers (len {})",
                yield_result_abs, self.registers.len()));
        }
        self.write_abs(yield_result_abs, val_raw, val_is_handle);

        if self.debug_sink.is_some() {
            let event = DebugEvent::Resume {
                saved_pc: resume_ip,
                saved_base,
                cell_dest: yield_result_abs - saved_base,
                val: Value::from_raw(val_raw),
                handler_dest: dest_reg.to_usize(),
                alive: Value::from_raw(alive_raw),
                depth: self.handlers.len(),
            };
            self.emit_debug(&event);
        }
        Ok(())
    }

    fn branch(&mut self, bc: &BytecodeChunk, offset: i16) -> Result<(), String> {
        let new_pc = (self.pc as isize) + (offset as isize);
        if new_pc < 0 || (new_pc as usize) > bc.code.len() {
            return Err(format!(
                "branch: pc {} out of range [0, {}]",
                new_pc, bc.code.len()
            ));
        }
        self.pc = new_pc as usize;
        Ok(())
    }

    #[inline(always)]
    fn abs(&self, r: Register) -> usize {
        let abs = self.base_reg + r.to_usize();
        debug_assert!(abs < self.registers.len(),
            "r{} (abs {}) out of register window (len {})",
            r.0, abs, self.registers.len());
        abs
    }

    #[inline(always)]
    pub(crate) fn ensure_registers(&mut self, needed: usize) {
        if needed > self.registers.len() {
            self.registers.resize(needed, HANDLE_NONE);
        }
        let mask_words_needed = (needed + 63) / 64;
        if mask_words_needed > self.register_mask.len() {
            self.register_mask.resize(mask_words_needed, 0);
        }
    }

    #[inline(always)]
    fn read_abs_raw(&self, abs: usize) -> u64 {
        debug_assert!(abs < self.registers.len());
        unsafe { *self.registers.get_unchecked(abs) }
    }

    #[inline(always)]
    fn write_abs_raw(&mut self, abs: usize, v: u64) {
        debug_assert!(abs < self.registers.len());
        unsafe { *self.registers.get_unchecked_mut(abs) = v; }
    }

    #[inline(always)]
    pub(crate) fn reg_mask_bit(&self, abs: usize) -> bool {
        let w = abs / 64;
        let b = abs % 64;
        self.register_mask.get(w).map_or(false, |x| (x >> b) & 1 == 1)
    }

    #[inline(always)]
    pub(crate) fn set_reg_mask_bit(&mut self, abs: usize, on: bool) {
        let w = abs / 64;
        let b = abs % 64;
        if let Some(x) = self.register_mask.get_mut(w) {
            if on { *x |= 1u64 << b; } else { *x &= !(1u64 << b); }
        }
    }

    #[inline(always)]
    fn write_abs(&mut self, abs: usize, raw: u64, is_handle: bool) {
        self.write_abs_raw(abs, raw);
        self.set_reg_mask_bit(abs, is_handle);
    }

    #[inline(always)]
    fn take_abs(&mut self, abs: usize) -> (u64, bool) {
        let v = self.read_abs_raw(abs);
        let h = self.reg_mask_bit(abs);
        self.write_abs_raw(abs, HANDLE_NONE);
        if h { self.set_reg_mask_bit(abs, false); }
        (v, h)
    }

    #[inline(always)]
    fn read(&self, r: Register) -> Result<(u64, bool), String> {
        let abs = self.abs(r);
        let v = self.read_abs_raw(abs);
        Ok((v, self.reg_mask_bit(abs)))
    }

    #[inline(always)]
    fn read_at(&self, r: Register) -> (u64, bool) {
        let abs = self.abs(r);
        (self.read_abs_raw(abs), self.reg_mask_bit(abs))
    }

    #[inline(always)]
    fn read_raw(&self, r: Register) -> Result<u64, String> {
        Ok(self.read_abs_raw(self.abs(r)))
    }

    #[inline(always)]
    fn take(&mut self, r: Register) -> Result<(u64, bool), String> {
        let abs = self.abs(r);
        let v = self.read_abs_raw(abs);
        let h = self.reg_mask_bit(abs);
        self.write_abs_raw(abs, HANDLE_NONE);
        self.set_reg_mask_bit(abs, false);
        Ok((v, h))
    }

    #[inline(always)]
    fn write(&mut self, r: Register, raw: u64, is_handle: bool) -> Result<(), String> {
        let abs = self.abs(r);
        self.write_abs(abs, raw, is_handle);
        Ok(())
    }

    #[inline(always)]
    fn read_i64(&self, r: Register) -> Result<i64, String> {
        Ok(self.read_raw(r)? as i64)
    }

    #[inline(always)]
    fn read_f64(&self, r: Register) -> Result<f64, String> {
        Ok(f64::from_bits(self.read_raw(r)?))
    }

    #[inline(always)]
    fn read_handle(&self, r: Register) -> Result<(u32, u32), String> {
        let abs = self.abs(r);
        let v = self.read_abs_raw(abs);
        if !self.reg_mask_bit(abs) {
            return Err(format!("expected handle, got plain {:#x} in r{}", v, r.0));
        }
        if v == HANDLE_NONE {
            return Err(format!("expected handle, got None in r{}", r.0));
        }
        Ok(Self::decode_handle(v))
    }

    #[inline(always)]
    fn decode_handle(raw: u64) -> (u32, u32) {
        (((raw >> 24) & 0x00FF_FFFF) as u32, (raw & 0x00FF_FFFF) as u32)
    }

    #[inline(always)]
    fn rc_inc_handle(&mut self, raw: u64) -> Result<(), String> {
        if raw == HANDLE_NONE { return Ok(()); }
        let (s, g) = Self::decode_handle(raw);
        self.heap.rc_inc(s, g)
    }

    #[inline(always)]
    fn rc_dec_handle(&mut self, raw: u64) -> Result<(), String> {
        if raw == HANDLE_NONE { return Ok(()); }
        let (s, g) = Self::decode_handle(raw);
        self.heap.rc_dec(s, g)?;
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn mem_used(&self) -> usize {
        self.heap.bytes_used()
    }

    pub(crate) fn mem_charge(&mut self, bytes: usize) -> Result<(), String> {
        let projected = self.mem_used().saturating_add(bytes);
        if projected <= MAX_RAM { return Ok(()); }
        Err(format!(
            "out of memory: requested {} bytes, {} / {} already in use",
            bytes, self.mem_used(), MAX_RAM
        ))
    }

    #[inline(always)]
    fn checked_heap_alloc(&mut self, size: usize) -> Result<(u32, u32), String> {
        self.mem_charge(size.saturating_mul(8))?;
        self.heap.try_alloc(size)
    }

    #[inline(always)]
    fn checked_heap_alloc_with_mask(&mut self, size: usize, init_mask: &[u64]) -> Result<(u32, u32), String> {
        self.mem_charge(size.saturating_mul(8))?;
        self.heap.try_alloc_with_mask(size, init_mask)
    }

    #[inline(always)]
    fn bin_i64<F: Fn(i64, i64) -> i64>(&mut self, d: Register, a: Register, b: Register, f: F) -> Result<(), String> {
        let x = self.read_i64(a)?;
        let y = self.read_i64(b)?;
        self.write(d, f(x, y) as u64, false)
    }

    #[inline(always)]
    fn bin_f64<F: Fn(f64, f64) -> f64>(&mut self, d: Register, a: Register, b: Register, f: F) -> Result<(), String> {
        let x = self.read_f64(a)?;
        let y = self.read_f64(b)?;
        self.write(d, f64::to_bits(f(x, y)), false)
    }

    #[inline(always)]
    fn bin_i64_checked<F: Fn(i64, i64) -> Option<i64>>(
        &mut self, d: Register, a: Register, b: Register, msg: &str, f: F,
    ) -> Result<(), String> {
        let x = self.read_i64(a)?;
        let y = self.read_i64(b)?;
        let r = f(x, y).ok_or_else(|| msg.to_string())?;
        self.write(d, r as u64, false)
    }

    #[inline(always)]
    fn bin_eq(&mut self, d: Register, a: Register, b: Register, negate: bool) -> Result<(), String> {
        let xa = self.read_raw(a)?;
        let xb = self.read_raw(b)?;
        let r = (xa == xb) ^ negate;
        self.write(d, bool_u64(r), false)
    }

    #[inline(always)]
    fn bin_i64_cmp<F: Fn(i64, i64) -> bool>(&mut self, d: Register, a: Register, b: Register, f: F) -> Result<(), String> {
        let x = self.read_i64(a)?;
        let y = self.read_i64(b)?;
        self.write(d, bool_u64(f(x, y)), false)
    }

    fn resolve_constants(&mut self, module: &Module) -> Result<(), String> {
        self.resolved_constants.clear();
        self.resolved_const_mask.clear();
        self.resolved_constants.reserve(module.functions.len());
        self.resolved_const_mask.reserve(module.functions.len());
        for chunk in &module.functions {
            let (vals, mask): (Vec<u64>, Vec<u64>) = match chunk {
                Chunk::Bytecode(bc) => {
                    let mut vals = bc.constants.clone();
                    let mask = bc.const_mask.clone();
                    // For each handle-bit constant, resolve the string-pool index it
                    // carries into an actual heap handle (rc=1, module-lifetime).
                    for i in 0..vals.len() {
                        if !mask_bit(&mask, i) { continue; }
                        let sidx = vals[i] as usize;
                        let s = bc.string_constants.get(sidx)
                            .cloned().unwrap_or_default();
                        let needed = s.len() + 16; // crude upper bound
                        if self.mem_used().saturating_add(needed) > MAX_RAM {
                            return Err(format!(
                                "out of memory at module load: string constant {} bytes",
                                s.len()
                            ));
                        }
                        let v = alloc_string(&mut self.heap, &s)?;
                        vals[i] = v.raw();
                        let (slot, gen_) = v.as_handle();
                        self.string_const_handles.push((slot, gen_));
                    }
                    (vals, mask)
                }
                Chunk::Native(_) => (Vec::new(), Vec::new()),
            };
            self.resolved_constants.push(vals);
            self.resolved_const_mask.push(mask);
        }
        self.resolved_natives.clear();
        self.resolved_natives.reserve(module.functions.len());
        for chunk in &module.functions {
            let entry = match chunk {
                Chunk::Native(n) => self.natives.get(&n.name).cloned(),
                Chunk::Bytecode(_) => None,
            };
            self.resolved_natives.push(entry);
        }
        Ok(())
    }

    fn resolve_dispatch(&self, key: u16) -> (u16, Option<(u64, bool)>) {
        let effect_id = (key >> 8) as u16;
        let op_id = (key & 0xFF) as usize;
        for h in self.handlers.iter().rev() {
            if h.effect_id != effect_id { continue; }
            if let Some(slot) = h.dispatch_table_slot {
                if let Ok((raw, _)) = self.heap.ld(slot, h.dispatch_table_gen, op_id * 2) {
                    let n = raw as i64;
                    if (0..=0xFFFF).contains(&n) {
                        let env = self.heap.ld(slot, h.dispatch_table_gen, op_id * 2 + 1).ok();
                        return (n as u16, env);
                    }
                }
            }
        }
        (polka::DISPATCH_NO_MATCH, None)
    }
}

#[inline(always)]
fn bool_u64(b: bool) -> u64 { if b { 1 } else { 0 } }

fn decode_dispatch_key(raw: u64) -> Result<u16, String> {
    let n = raw as i64;
    if (0..=0xFFFF).contains(&n) { Ok(n as u16) }
    else { Err(format!("dispatch.lookup: bad key {}", n)) }
}

fn split_port(port_val: i64) -> Result<(u8, u8), String> {
    if !(0..=0xFFFF).contains(&port_val) {
        return Err(format!("device port {:#x} out of 16-bit range", port_val));
    }
    let device_id = ((port_val >> 8) & 0xFF) as u8;
    let port = (port_val & 0xFF) as u8;
    Ok((device_id, port))
}

fn validate_module_register_budget(module: &Module) -> Result<(), String> {
    for (i, chunk) in module.functions.iter().enumerate() {
        if let Chunk::Bytecode(b) = chunk {
            if b.reg_count > FRAME_REGS {
                return Err(format!(
                    "module load: fn {} has reg_count {} > frame budget {}",
                    i, b.reg_count, FRAME_REGS
                ));
            }
            if b.param_count > b.reg_count {
                return Err(format!(
                    "module load: fn {} has param_count {} > reg_count {}",
                    i, b.param_count, b.reg_count
                ));
            }
        }
    }
    Ok(())
}
