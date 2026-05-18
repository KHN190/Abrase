use super::{VirtualMachine, Value};
use super::debug::DebugEvent;
use super::memory::HEAP_BYTES_PER_VALUE;
use super::value::{BoxPool, BoxedValue};
use polka::{BytecodeChunk, Chunk, OpCode, Register, Module, FRAME_REGS};
use crate::frame::Frame;

const MAX_REGISTERS: usize = 1 << 16;
const MAX_RECURSION_DEPTH: usize = 2048;

// Hard cap on VM memory (heap cells + boxed values).
pub const MAX_RAM: usize = 64 * 1024 * 1024;

impl VirtualMachine {
    pub fn run(&mut self, chunk: &Chunk) -> Result<Value, String> {
        let module = Module { functions: vec![chunk.clone()], entry: 0, device_mask: [0; 32] };
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
        self.validate_module_devices(module)?;
        self.frames.clear();
        self.handlers.clear();
        self.region_table.clear();
        self.heap.clear();
        self.box_pool.clear();
        self.resolve_constants(module)?;
        self.pc = 0;
        self.base_reg = 0;
        self.current_func = module.entry;
        self.halted = false;
        self.exit_code = None;
        let needed = FRAME_REGS;
        if needed > self.registers.len() {
            self.registers.resize(needed, Value::NONE);
        }

        'outer: loop {
            if self.halted {
                if let Some(code) = self.exit_code {
                    return Ok(Value::from_int(code));
                }
                let v = self.read_abs(self.base_reg);
                return if v.is_none() { Err("Return register is empty".to_string()) } else { Ok(v) };
            }
            debug_assert!(self.current_func < module.functions.len(),
                "current_func {} out of bounds (functions.len {})",
                self.current_func, module.functions.len());

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
                        let return_val = self.read_abs(self.base_reg);
                        if return_val.is_none() { return Err("Return register is empty".to_string()); }
                        self.pc = frame.ip;
                        self.base_reg = frame.base_reg;
                        self.current_func = frame.func_id;
                        self.write_abs(frame.dest_reg, return_val);
                        continue 'outer;
                    } else {
                        let v = self.read_abs(self.base_reg);
                        return if v.is_none() { Err("Return register is empty".to_string()) } else { Ok(v) };
                    }
                }
                let opcode = &bc.code[self.pc];
                self.failing_pc = self.pc;
                if self.debug_sink.is_some() {
                    let event = DebugEvent::Trace {
                        func: self.current_func, pc: self.pc, op: opcode,
                    };
                    self.emit_debug(&event);
                }
                self.pc += 1;
                self.exec(module, bc, opcode)?;
                if self.halted || self.current_func != entry_func {
                    continue 'outer;
                }
            }
        }
    }

    fn exec(&mut self, module: &Module, bc: &BytecodeChunk, op: &OpCode) -> Result<(), String> {
        match op {
            OpCode::Add(d, a, b)  => self.bin_i64(*d, *a, *b, |x, y| x.wrapping_add(y)),
            OpCode::Sub(d, a, b)  => self.bin_i64(*d, *a, *b, |x, y| x.wrapping_sub(y)),
            OpCode::Mul(d, a, b)  => self.bin_i64(*d, *a, *b, |x, y| x.wrapping_mul(y)),
            OpCode::Div(d, a, b)  => self.bin_i64_checked(*d, *a, *b, "div by zero", |x, y| x.checked_div(y)),
            OpCode::Mod(d, a, b)  => self.bin_i64_checked(*d, *a, *b, "mod by zero", |x, y| x.checked_rem(y)),
            OpCode::Neg(d, a)     => {
                let v = self.read_i64(*a)?;
                let r = self.checked_int_or_box(v.wrapping_neg())?;
                self.write(*d, r)
            }

            OpCode::Eq(d, a, b)  => self.bin_cmp(*d, *a, *b, |x, y| x == y),
            OpCode::Neq(d, a, b) => self.bin_cmp(*d, *a, *b, |x, y| x != y),
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
                let v = self.read(*r)?;
                if is_falsy(&v) { self.branch(bc, *off) } else { Ok(()) }
            }
            OpCode::Jnz(r, off) => {
                let v = self.read(*r)?;
                if !is_falsy(&v) { self.branch(bc, *off) } else { Ok(()) }
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
                let resolved = &self.resolved_constants[self.current_func];
                if idx >= resolved.len() {
                    return Err("Constant index out of bounds".to_string());
                }
                let v = resolved[idx];
                self.value_rc_inc(&v)?;
                self.write(*reg, v)
            }
            OpCode::Copy(d, s) => {
                let v = self.read(*s)?;
                self.value_rc_inc(&v)?;
                self.write(*d, v)
            }
            OpCode::Move(d, s) => {
                let v = self.take(*s)?;
                self.write(*d, v)
            }

            OpCode::Ld(d, b, off) => {
                let (slot, generation) = self.read_handle(*b)?;
                let v = self.heap.ld(slot, generation, *off as usize)?;
                self.value_rc_inc(&v)?;
                self.write(*d, v)
            }
            OpCode::St(src, b, off) => {
                let (slot, generation) = self.read_handle(*b)?;
                let v = self.take(*src)?;
                let old = self.heap.st(slot, generation, *off as usize, v)?;
                self.value_rc_dec(&old)?;
                Ok(())
            }
            OpCode::LdIdx(d, b, i) => {
                let (slot, generation) = self.read_handle(*b)?;
                let off = self.read_i64(*i)?;
                if off < 0 {
                    return Err(format!("ldidx: negative index {}", off));
                }
                let v = self.heap.ld(slot, generation, off as usize)?;
                self.value_rc_inc(&v)?;
                self.write(*d, v)
            }
            OpCode::StIdx(src, b, i) => {
                let (slot, generation) = self.read_handle(*b)?;
                let off = self.read_i64(*i)?;
                if off < 0 {
                    return Err(format!("stidx: negative index {}", off));
                }
                let v = self.take(*src)?;
                let old = self.heap.st(slot, generation, off as usize, v)?;
                self.value_rc_dec(&old)?;
                Ok(())
            }
            OpCode::Ref(d, s) => {
                let v = self.read(*s)?;
                self.value_rc_inc(&v)?;
                let (slot, generation) = self.checked_heap_alloc(1)?;
                // Record the enclosing region (if any), same as Alloc
                // and Ref, so per-iter / stmt-position pops force-free the
                // ref cell instead of leaking it.
                self.region_record_alloc(slot, generation);
                self.heap.st(slot, generation, 0, v)?;
                self.write(*d, Value::from_handle(slot, generation))
            }

            OpCode::AddImm(d, s, imm) => {
                let x = self.read_i64(*s)?;
                let v = self.checked_int_or_box(x.wrapping_add(*imm as i64))?;
                self.write(*d, v)
            }
            OpCode::SubImm(d, s, imm) => {
                let x = self.read_i64(*s)?;
                let v = self.checked_int_or_box(x.wrapping_sub(*imm as i64))?;
                self.write(*d, v)
            }

            OpCode::Alloc(d, size) => {
                let (slot, generation) = self.checked_heap_alloc(*size as usize)?;
                self.region_record_alloc(slot, generation);
                self.write(*d, Value::from_handle(slot, generation))
            }
            OpCode::Drop(reg) => {
                let abs = self.abs(*reg);
                let v = self.take_abs(abs);
                if !v.is_none() { self.value_rc_dec(&v)?; }
                Ok(())
            }

            // Read from a device port. Virtual DISPATCH ports are served from the active handler
            // cell instead of the device registry; falls through to the real device otherwise.
            OpCode::Dei(d, port_reg) => self.do_dei(*d, *port_reg),
            // Write to a device port. Handles system exit, effect dispatch (snapshot + cell
            // allocation + handler resolution), handler pop, and region management. All other
            // ports delegate to the device registry.
            OpCode::Deo(src, port_reg) => self.do_deo(module, *src, *port_reg),
            // Push a handler frame bound to effect_id. The dispatch table is a heap array of
            // (fn_id, env) pairs indexed by operation. Continuation cells are allocated lazily
            // on the first DEO dispatch, not here.
            OpCode::Handle(_dest, table_reg, effect_id) => {
                let table_val = self.read_abs(self.abs(*table_reg));
                let (table_slot, table_gen) = match table_val.as_handle() {
                    Some((s, g)) => (Some(s), g),
                    None => (None, 0),
                };
                self.handlers.push(super::HandlerFrame {
                    effect_id: *effect_id,
                    dispatch_table_slot: table_slot,
                    dispatch_table_gen: table_gen,
                    cell_slot: 0,
                    cell_gen: 0,
                    cells_allocated: Vec::new(),
                    body_frame_index: None,
                    pending_return_arm_fn: None,
                    pending_return_arm_env: polka::Value::NONE,
                });
                Ok(())
            }
            // Restore a continuation: write the resume value into the work frame's dest register,
            // push the current (handler) frame so it returns after the work completes, then jump
            // to saved_pc + 2 (the instruction after the suspending DEO).
            OpCode::Resume(dest_reg, val_reg) => self.do_resume(module, *dest_reg, *val_reg),
        }
    }

    fn do_call(&mut self, module: &Module, caller_bc: &BytecodeChunk, dest: Register, fn_id: usize) -> Result<(), String> {
        if fn_id >= module.functions.len() {
            return Err(format!("call: unknown fn_id {}", fn_id));
        }
        let caller_reg_count = caller_bc.reg_count;
        let callee_reg_count = match &module.functions[fn_id] {
            Chunk::Bytecode(b) => b.reg_count,
            Chunk::Native(_) => 0,
        };
        if dest.to_usize() >= caller_reg_count {
            return Err(format!(
                "call: dest r{} out of caller window (reg_count {})",
                dest.0, caller_reg_count
            ));
        }
        let dest_abs = self.base_reg + dest.to_usize();
        let new_base = self.base_reg + caller_reg_count;
        // Reserve at least FRAME_REGS even if the callee declares fewer
        let window = callee_reg_count.max(FRAME_REGS);
        let needed = new_base + window;
        if needed > MAX_REGISTERS {
            return Err(format!(
                "Stack overflow: register window {} exceeds limit {}",
                needed, MAX_REGISTERS
            ));
        }
        if needed > self.registers.len() {
            self.registers.resize(needed, Value::NONE);
        }

        if let Chunk::Native(n) = &module.functions[fn_id] {
            let func = self.natives.get(&n.name)
                .ok_or_else(|| format!("native call: '{}' not registered", n.name))?
                .clone();
            let mut args: Vec<Value> = Vec::with_capacity(n.param_count);
            for i in 0..n.param_count {
                let v = self.read_abs(new_base + i);
                if v.is_none() {
                    return Err(format!("native call: arg {} (r{}) is empty", i, i));
                }
                args.push(v);
            }
            let mut ctx = super::NativeCtx {
                pool: &mut self.box_pool,
                devices: &mut self.devices,
                halted: &mut self.halted,
                exit_code: &mut self.exit_code,
            };
            let result = func(&mut ctx, &args)?;
            self.write_abs(dest_abs, result);
            return Ok(());
        }

        if self.frames.len() >= MAX_RECURSION_DEPTH {
            return Err(format!(
                "Stack overflow: recursion depth {} exceeds limit {}",
                self.frames.len(),
                MAX_RECURSION_DEPTH
            ));
        }
        let new_frame_index = self.frames.len();
        self.frames.push(Frame::normal(self.current_func, self.pc, self.base_reg, dest_abs));
        // The first call after a `Handle` install is the handle body call —
        // record its frame index so do_ret can recognise the boundary.
        if let Some(handler) = self.handlers.last_mut() {
            if handler.body_frame_index.is_none() {
                handler.body_frame_index = Some(new_frame_index);
            }
        }
        self.base_reg = new_base;
        self.current_func = fn_id;
        self.pc = 0;
        Ok(())
    }

    #[inline]
    fn do_ret(&mut self, module: &Module, reg: Register) -> Result<(), String> {
        let abs = self.abs(reg);
        let return_val = self.take_abs(abs);
        if return_val.is_none() { return Err("Return register is empty".to_string()); }
        let frame = match self.frames.pop() {
            Some(f) => f,
            None => {
                self.write_abs(self.base_reg, return_val);
                self.halted = true;
                return Ok(());
            }
        };

        // if the popped frame is a handle body's call frame, or an arm-
        // continuation, and the active handler still has a pending return 
        // arm, route `return_val` through the return arm
        let is_body_frame = self.handlers.last()
            .and_then(|h| h.body_frame_index)
            .map_or(false, |idx| idx == self.frames.len());
        let route_through_return_arm = (frame.is_arm_continuation || is_body_frame)
            && self.handlers.last().map_or(false, |h| h.pending_return_arm_fn.is_some());

        if !route_through_return_arm {
            // Normal Ret.
            if frame.is_arm_continuation && frame.arm_snapshot_count > 0 {
                let snap = crate::snapshot::SnapshotHandle {
                    slot: frame.arm_snapshot_slot,
                    generation: frame.arm_snapshot_gen,
                    count: frame.arm_snapshot_count,
                };
                self.restore_registers(frame.base_reg, snap)?;
                // Single-shot: snapshot consumed, drop the off-region cell.
                self.heap.rc_dec(snap.slot, snap.generation, &mut self.box_pool)?;
            }
            self.pc = frame.ip;
            self.base_reg = frame.base_reg;
            self.current_func = frame.func_id;
            self.write_abs(frame.dest_reg, return_val);
            return Ok(());
        }

        // Restore arm's snapshot before invoking the return arm (so when
        // the return arm Rets back, the arm body sees its own state).
        if frame.is_arm_continuation && frame.arm_snapshot_count > 0 {
            let snap = crate::snapshot::SnapshotHandle {
                slot: frame.arm_snapshot_slot,
                generation: frame.arm_snapshot_gen,
                count: frame.arm_snapshot_count,
            };
            self.restore_registers(frame.base_reg, snap)?;
            self.heap.rc_dec(snap.slot, snap.generation, &mut self.box_pool)?;
        }

        // Take the return arm info; clear pending so subsequent pops in
        // this handler don't re-apply it.
        let (ra_fn, ra_env) = {
            let h = self.handlers.last_mut().unwrap();
            let fn_id = h.pending_return_arm_fn.take().unwrap();
            let env = std::mem::replace(&mut h.pending_return_arm_env, Value::NONE);
            (fn_id, env)
        };

        // Push a "post-return-arm" frame so when the return arm Rets, the
        // runtime restores the popped frame's state and delivers the return
        // arm's result to its dest_reg.
        self.frames.push(super::Frame::normal(
            frame.func_id, frame.ip, frame.base_reg, frame.dest_reg,
        ));

        // Set up the call to the return arm. new_base sits just past the
        // popped frame's caller window — same allocation a regular Call from
        // that function would compute.
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
        let needed = new_base + window;
        if needed > MAX_REGISTERS {
            return Err(format!(
                "Stack overflow setting up return arm: window {} exceeds limit {}",
                needed, MAX_REGISTERS
            ));
        }
        if needed > self.registers.len() {
            self.registers.resize(needed, Value::NONE);
        }

        // Args: (env, return_env, value). Layout matches the handle codegen.
        // Normal callee-arg staging uses Copy (rc_inc); mirror that here so the
        // return arm owns its own refs and balanced drops won't underflow.
        // `ra_env` was stashed without rc_inc at DEO install time; `return_val`
        // was just taken from a register (ref transferred in, no new ref to
        // create). Box and handle values get rc_inc'd; primitives are no-ops.
        self.value_rc_inc(&ra_env)?;
        self.write_abs(new_base, ra_env);
        self.write_abs(new_base + 1, Value::from_int(0));
        self.write_abs(new_base + 2, return_val);

        self.base_reg = new_base;
        self.current_func = ra_fn;
        self.pc = 0;
        Ok(())
    }

    fn do_dei(&mut self, d: Register, port_reg: Register) -> Result<(), String> {
        let port_val = self.read_i64(port_reg)?;
        let (device_id, port) = split_port(port_val)?;
        if device_id == polka::DISPATCH_ID && port == polka::DISPATCH_PORT_LOOKUP {
            let fn_id = if let Some(frame) = self.handlers.last() {
                self.heap.ld(frame.cell_slot, frame.cell_gen, super::cont_slot::DISPATCH_FN_ID)?
            } else {
                // No active handler: return cached result or DISPATCH_NO_MATCH.
                Value::from_int(self.dispatch_last_result.take().unwrap_or(0xFFFF) as i64)
            };
            return self.write(d, fn_id);
        }
        if device_id == polka::DISPATCH_ID && port == polka::DISPATCH_PORT_ENV {
            let env_val = if let Some(frame) = self.handlers.last() {
                self.heap.ld(frame.cell_slot, frame.cell_gen, super::cont_slot::DISPATCH_ENV)?
            } else {
                // No active handler: return cached env or NONE.
                self.dispatch_last_env.take().unwrap_or(Value::NONE)
            };
            // NONE is the VM's uninitialized sentinel; Ret rejects it, so decode to 0.
            let env_val = if env_val.is_none() { Value::from_int(0) } else { env_val };
            return self.write(d, env_val);
        }
        let dev = self.devices.get_mut(device_id)
            .ok_or_else(|| format!("dei: device {:#04x} not installed", device_id))?;
        let v = dev.read(port)?;
        self.write(d, v)
    }

    fn do_deo(&mut self, module: &Module, src: Register, port_reg: Register) -> Result<(), String> {
        let v = self.read(src)?;
        let port_val = self.read_i64(port_reg)?;
        let (device_id, port) = split_port(port_val)?;
        if device_id == 0x00 {
            match port {
                0x01 => {
                    if let Some(n) = v.as_int() {
                        self.exit_code = Some(n & 0xFFFF_FFFF);
                    }
                    self.halted = true;
                    return Ok(());
                }
                0x02 => {
                    let msg = match v.as_int() {
                        Some(n) if n >= 0 => match self.box_pool.get(n as u32) {
                            Some(BoxedValue::String(s)) => s.clone(),
                            _ => format!("(pool idx {})", n),
                        },
                        _ => format!("{:?}", v),
                    };
                    return Err(format!("panic: {}", msg));
                }
                _ => {}
            }
        }
        if device_id == polka::DISPATCH_ID && port == polka::DISPATCH_PORT_LOOKUP {
            let key = decode_dispatch_key(v)?;
            let (cell_slot, cell_gen) = self.checked_heap_alloc(super::cont_slot::SIZE)?;
            self.region_record_alloc(cell_slot, cell_gen);
            self.heap.st(cell_slot, cell_gen, super::cont_slot::SUSPEND_PC,
                Value::from_int((self.pc - 1) as i64))?;
            self.heap.st(cell_slot, cell_gen, super::cont_slot::SUSPEND_BASE,
                Value::from_int(self.base_reg as i64))?;
            self.heap.st(cell_slot, cell_gen, super::cont_slot::SUSPEND_FUNC,
                Value::from_int(self.current_func as i64))?;
            self.heap.st(cell_slot, cell_gen, super::cont_slot::ALIVE,
                Value::from_int(1))?;

            // Snapshot the suspended function's register window
            let reg_count = self.current_fn_reg_count(module);
            let snapshot = self.snapshot_registers(self.base_reg, reg_count)?;
            self.write_snapshot_into_cell(cell_slot, cell_gen, snapshot)?;

            if let Some(handler_frame) = self.handlers.last_mut() {
                handler_frame.cells_allocated.push((cell_slot, cell_gen));
                handler_frame.cell_slot = cell_slot;
                handler_frame.cell_gen = cell_gen;
            }
            let (fn_id, env) = self.resolve_dispatch(key);
            self.heap.st(cell_slot, cell_gen, super::cont_slot::DISPATCH_FN_ID,
                Value::from_int(fn_id as i64))?;
            // The cont cell takes a new ref to env: cascade rc_dec on cell
            // free will balance with this rc_inc on store.
            let env_val = env.unwrap_or(Value::NONE);
            self.value_rc_inc(&env_val)?;
            self.heap.st(cell_slot, cell_gen, super::cont_slot::DISPATCH_ENV, env_val)?;
            self.dispatch_last_result = Some(fn_id);
            self.dispatch_last_env = env;
            return Ok(());
        }
        if device_id == polka::DISPATCH_ID && port == polka::DISPATCH_PORT_POP_HANDLER {
            let frame = self.handlers.pop()
                .ok_or("dispatch.pop_handler: no active handler frame")?;
            for (slot, generation) in frame.cells_allocated.iter() {
                self.region_table.forget(*slot, *generation);
            }
            return Ok(());
        }
        if device_id == polka::DISPATCH_ID && port == polka::DISPATCH_PORT_RETURN_FN {
            let fn_id = v.as_int()
                .ok_or_else(|| format!("dispatch.return_fn: expected int, got {:?}", v))?;
            let handler = self.handlers.last_mut()
                .ok_or("dispatch.return_fn: no active handler frame")?;
            handler.pending_return_arm_fn = Some(fn_id as usize);
            return Ok(());
        }
        if device_id == polka::DISPATCH_ID && port == polka::DISPATCH_PORT_RETURN_ENV {
            let handler = self.handlers.last_mut()
                .ok_or("dispatch.return_env: no active handler frame")?;
            handler.pending_return_arm_env = v;
            return Ok(());
        }
        if device_id == polka::REGION_ID {
            return match port {
                polka::REGION_PORT_PUSH => { self.region_push(); Ok(()) }
                polka::REGION_PORT_POP  => self.region_pop(),
                polka::REGION_PORT_FORGET => {
                    // Callers may emit forget unconditionally; skip non-handle values.
                    if let Some((slot, gen_)) = v.as_handle() {
                        self.region_table.forget(slot, gen_);
                    }
                    Ok(())
                }
                _ => Err(format!("region: unknown port {:#x}", port)),
            };
        }
        let dev = self.devices.get_mut(device_id)
            .ok_or_else(|| format!("deo: device {:#04x} not installed", device_id))?;
        dev.write_with_pool(port, v, &mut self.box_pool)
    }

    fn do_resume(&mut self, module: &Module, dest_reg: Register, val_reg: Register) -> Result<(), String> {
        let (cell_slot, cell_gen) = {
            let h = self.handlers.last()
                .ok_or("Resume outside an active handler frame")?;
            (h.cell_slot, h.cell_gen)
        };
        let alive = self.heap.ld(cell_slot, cell_gen, super::cont_slot::ALIVE)?;
        match alive.as_int() {
            Some(1) => {}
            Some(0) => return Err("resume: continuation already consumed".to_string()),
            _ => return Err(format!(
                "resume: continuation cell corrupted (alive slot = {:?})", alive)),
        }
        let saved_base = match self.heap.ld(cell_slot, cell_gen,
            super::cont_slot::SUSPEND_BASE)?.as_int() {
            Some(n) if n >= 0 => n as usize,
            _ => return Err("resume: continuation cell has invalid base".to_string()),
        };
        let saved_func = match self.heap.ld(cell_slot, cell_gen,
            super::cont_slot::SUSPEND_FUNC)?.as_int() {
            Some(n) if n >= 0 => n as usize,
            _ => return Err("resume: continuation cell has invalid func".to_string()),
        };
        let suspended_snapshot = self.read_snapshot_from_cell(cell_slot, cell_gen)?;

        // Mark the continuation consumed. With single-shot semantics in place,
        // a second resume from the same cell would also fail at "no arm-call
        // frame on stack" below — but flipping ALIVE here gives the clearer
        // diagnostic and matches the cont-slot contract.
        self.heap.st(cell_slot, cell_gen, super::cont_slot::ALIVE,
            Value::from_int(0))?;

        let val = self.read(val_reg)?;

        // NOTE: pop happens before snapshot_registers / push / restore. If any
        // of those allocs fails (OOM in current Myriad spec is fatal), the
        // VM state is left partial. Acceptable today because OOM halts the VM;
        // revisit if recovery becomes a goal.
        let arm_call_frame = self.frames.pop()
            .ok_or("resume: no arm-call frame on stack")?;
        let yield_result_abs = arm_call_frame.dest_reg;
        let resume_ip = arm_call_frame.ip;

        // Snapshot the arm's register window. Off-region so a suspended
        // function's REGION.pop on the path back to its Ret can't free it.
        let arm_reg_count = self.current_fn_reg_count(module);
        let arm_snapshot = self.snapshot_registers_off_region(self.base_reg, arm_reg_count)?;

        // Push the continuation frame. Pop when the continuation Rets.
        let arm_resume_dest_abs = self.base_reg + dest_reg.to_usize();
        self.frames.push(super::Frame {
            func_id: self.current_func,
            ip: self.pc,
            base_reg: self.base_reg,
            dest_reg: arm_resume_dest_abs,
            is_arm_continuation: true,
            arm_snapshot_slot: arm_snapshot.slot,
            arm_snapshot_gen: arm_snapshot.generation,
            arm_snapshot_count: arm_snapshot.count,
        });

        // Restore suspended fn register window.
        self.base_reg = saved_base;
        self.current_func = saved_func;
        self.pc = resume_ip;
        self.restore_registers(saved_base, suspended_snapshot)?;

        if yield_result_abs >= self.registers.len() {
            return Err(format!(
                "resume: yield dest abs {} out of registers (len {})",
                yield_result_abs, self.registers.len()));
        }
        self.write_abs(yield_result_abs, val);

        if self.debug_sink.is_some() {
            let event = DebugEvent::Resume {
                saved_pc: resume_ip,
                saved_base,
                cell_dest: yield_result_abs - saved_base,
                val,
                handler_dest: dest_reg.to_usize(),
                alive,
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
    fn read_abs(&self, abs: usize) -> Value {
        debug_assert!(abs < self.registers.len(),
            "register abs {} out of bounds (len {})", abs, self.registers.len());
        unsafe { *self.registers.get_unchecked(abs) }
    }

    #[inline(always)]
    fn write_abs(&mut self, abs: usize, v: Value) {
        debug_assert!(abs < self.registers.len(),
            "register abs {} out of bounds (len {})", abs, self.registers.len());
        unsafe { *self.registers.get_unchecked_mut(abs) = v; }
    }

    #[inline(always)]
    fn take_abs(&mut self, abs: usize) -> Value {
        debug_assert!(abs < self.registers.len(),
            "register abs {} out of bounds (len {})", abs, self.registers.len());
        unsafe { std::mem::replace(self.registers.get_unchecked_mut(abs), Value::NONE) }
    }

    #[inline(always)]
    fn read(&self, r: Register) -> Result<Value, String> {
        let v = self.read_abs(self.abs(r));
        if v.is_none() { Err(format!("read: r{} is empty", r.0)) } else { Ok(v) }
    }

    #[inline(always)]
    fn take(&mut self, r: Register) -> Result<Value, String> {
        let v = self.take_abs(self.abs(r));
        if v.is_none() {
            Err(format!("move: r{} is empty (already moved?)", r.0))
        } else { Ok(v) }
    }

    #[inline(always)]
    fn write(&mut self, r: Register, v: Value) -> Result<(), String> {
        self.write_abs(self.abs(r), v);
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn value_rc_inc(&mut self, v: &Value) -> Result<(), String> {
        if let Some((slot, generation)) = v.as_handle() {
            self.heap.rc_inc(slot, generation)
        } else if let Some(idx) = v.as_box() {
            self.box_pool.inc(idx)
        } else {
            Ok(())
        }
    }

    #[inline(always)]
    pub(crate) fn value_rc_dec(&mut self, v: &Value) -> Result<(), String> {
        if let Some((slot, generation)) = v.as_handle() {
            self.heap.rc_dec(slot, generation, &mut self.box_pool).map(|_| ())
        } else if let Some(idx) = v.as_box() {
            self.box_pool.dec_cascade(idx, &mut self.heap);
            Ok(())
        } else {
            Ok(())
        }
    }

    #[inline(always)]
    fn read_i64(&self, r: Register) -> Result<i64, String> {
        let v = self.read_abs(self.abs(r));
        if v.is_none() { return Err(format!("read: r{} is empty", r.0)); }
        self.box_pool.read_int(v).ok_or_else(|| format!("expected i64, got {:?}", v))
    }

    #[inline(always)]
    pub(crate) fn mem_used(&self) -> usize {
        self.heap.bytes_used().saturating_add(self.box_pool.bytes_used())
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
        self.mem_charge(size.saturating_mul(HEAP_BYTES_PER_VALUE))?;
        Ok(self.heap.alloc(size))
    }

    #[inline(always)]
    fn checked_int_or_box(&mut self, n: i64) -> Result<Value, String> {
        if Value::fits_i48(n) {
            return Ok(Value::from_int(n));
        }
        self.mem_charge(BoxPool::pending_bytes(&BoxedValue::Int(n)))?;
        Ok(self.box_pool.intern_int(n))
    }

    #[inline(always)]
    fn read_handle(&self, r: Register) -> Result<(u32, u32), String> {
        let v = self.read_abs(self.abs(r));
        if v.is_none() { return Err(format!("read: r{} is empty", r.0)); }
        v.as_handle().ok_or_else(|| format!("expected handle, got {:?}", v))
    }

    #[inline(always)]
    fn bin_i64<F: Fn(i64, i64) -> i64>(&mut self, d: Register, a: Register, b: Register, f: F)
        -> Result<(), String>
    {
        let x = self.read_i64(a)?;
        let y = self.read_i64(b)?;
        let v = self.checked_int_or_box(f(x, y))?;
        self.write(d, v)
    }

    #[inline(always)]
    fn bin_i64_checked<F: Fn(i64, i64) -> Option<i64>>(
        &mut self, d: Register, a: Register, b: Register, msg: &str, f: F,
    ) -> Result<(), String> {
        let x = self.read_i64(a)?;
        let y = self.read_i64(b)?;
        let r = f(x, y).ok_or_else(|| msg.to_string())?;
        let v = self.checked_int_or_box(r)?;
        self.write(d, v)
    }

    #[inline(always)]
    fn bin_cmp<F: Fn(&Value, &Value) -> bool>(&mut self, d: Register, a: Register, b: Register, f: F) -> Result<(), String> {
        let x = self.read(a)?;
        let y = self.read(b)?;
        self.write(d, Value::from_bool(f(&x, &y)))
    }

    #[inline(always)]
    fn bin_i64_cmp<F: Fn(i64, i64) -> bool>(&mut self, d: Register, a: Register, b: Register, f: F) -> Result<(), String> {
        let x = self.read_i64(a)?;
        let y = self.read_i64(b)?;
        self.write(d, Value::from_bool(f(x, y)))
    }

    fn resolve_constants(&mut self, module: &Module) -> Result<(), String> {
        self.resolved_constants.clear();
        self.resolved_constants.reserve(module.functions.len());
        for chunk in &module.functions {
            let resolved: Vec<Value> = match chunk {
                Chunk::Bytecode(bc) => {
                    let mut out = Vec::with_capacity(bc.constants.len());
                    for v in &bc.constants {
                        if let Some(sidx) = v.as_str_const() {
                            let s = bc.string_constants.get(sidx as usize)
                                .cloned().unwrap_or_default();
                            let pending = BoxedValue::String(s);
                            let cost = BoxPool::pending_bytes(&pending);
                            if self.mem_used().saturating_add(cost) > MAX_RAM {
                                return Err(format!(
                                    "out of memory at module load: string constant {} bytes, {} / {} in use",
                                    cost, self.mem_used(), MAX_RAM
                                ));
                            }
                            let bidx = self.box_pool.intern(pending);
                            out.push(Value::from_box(bidx));
                        } else {
                            out.push(*v);
                        }
                    }
                    out
                }
                Chunk::Native(_) => Vec::new(),
            };
            self.resolved_constants.push(resolved);
        }
        Ok(())
    }

    fn validate_module_devices(&self, module: &Module) -> Result<(), String> {
        for id in 0u16..=255 {
            let id = id as u8;
            if id == polka::DISPATCH_ID { continue; }
            if module.requires_device(id) && !self.devices.has(id) {
                return Err(format!("module requires device {:#04x} but it is not installed", id));
            }
        }
        Ok(())
    }

    fn resolve_dispatch(&self, key: u16) -> (u16, Option<Value>) {
        let effect_id = (key >> 8) as u16;
        let op_id = (key & 0xFF) as usize;
        for h in self.handlers.iter().rev() {
            if h.effect_id != effect_id { continue; }
            if let Some(slot) = h.dispatch_table_slot {
                if let Ok(v) = self.heap.ld(slot, h.dispatch_table_gen, op_id * 2) {
                    if let Some(n) = v.as_int() {
                        if (0..=0xFFFF).contains(&n) {
                            let env = self.heap.ld(slot, h.dispatch_table_gen, op_id * 2 + 1).ok();
                            return (n as u16, env);
                        }
                    }
                }
                if let Ok(v) = self.heap.ld(slot, h.dispatch_table_gen, op_id) {
                    if let Some(n) = v.as_int() {
                        if (0..=0xFFFF).contains(&n) {
                            return (n as u16, None);
                        }
                    }
                }
            }
        }
        (polka::DISPATCH_NO_MATCH, None)
    }
}

// Validate and extract a dispatch key (effect_id << 8 | op_id) from a Value.
fn decode_dispatch_key(v: Value) -> Result<u16, String> {
    match v.as_int() {
        Some(n) if (0..=0xFFFF).contains(&n) => Ok(n as u16),
        _ => Err(format!("dispatch.lookup: bad key {:?}", v)),
    }
}

fn is_falsy(val: &Value) -> bool {
    if let Some(b) = val.as_bool() { return !b; }
    if let Some(i) = val.as_int() { return i == 0; }
    val.is_unit()
}

// 16-bit port = (device_id << 8) | port_offset
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
