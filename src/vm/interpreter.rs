use super::{VirtualMachine, Value};
use crate::bytecode::{BytecodeChunk, Chunk, OpCode, Register, Module, FRAME_REGS};
use crate::vm::frame::Frame;

const MAX_REGISTERS: usize = 1 << 16;
const MAX_RECURSION_DEPTH: usize = 2048;

impl VirtualMachine {
    pub fn run(&mut self, chunk: &Chunk) -> Result<Value, String> {
        let module = Module { functions: vec![chunk.clone()], entry: 0, device_mask: [0; 32] };
        self.run_module(&module)
    }

    pub fn run_module(&mut self, module: &Module) -> Result<Value, String> {
        let r = self.run_module_inner(module);
        r.map_err(|e| format!("[fn {} pc {}] {}", self.current_func, self.pc, e))
    }

    fn run_module_inner(&mut self, module: &Module) -> Result<Value, String> {
        validate_module_register_budget(module)?;
        self.validate_module_devices(module)?;
        self.pc = 0;
        self.base_reg = 0;
        self.current_func = module.entry;
        self.frames.clear();
        self.handlers.clear();
        self.halted = false;
        self.exit_code = None;
        let needed = FRAME_REGS;
        if needed > self.registers.len() {
            self.registers.resize(needed, None);
        }

        'outer: loop {
            if self.halted {
                if let Some(code) = self.exit_code {
                    return Ok(Value::Int(code));
                }
                return self.registers[self.base_reg].clone()
                    .ok_or("Return register is empty".to_string());
            }
            let bc = match &module.functions[self.current_func] {
                Chunk::Bytecode(b) => b,
                Chunk::Native(_) => return Err(format!(
                    "entry fn {} is native; cannot start execution there", self.current_func
                )),
            };
            let entry_func = self.current_func;
            loop {
                if self.pc >= bc.code.len() {
                    if let Some(frame) = self.frames.pop() {
                        let return_val = self.registers[self.base_reg].clone()
                            .ok_or("Return register is empty")?;
                        self.pc = frame.ip;
                        self.base_reg = frame.base_reg;
                        self.current_func = frame.func_id;
                        self.registers[frame.dest_reg] = Some(return_val);
                        continue 'outer;
                    } else {
                        return self.registers[self.base_reg].clone()
                            .ok_or("Return register is empty".to_string());
                    }
                }
                let opcode = &bc.code[self.pc];
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
                self.write(*d, Value::Int(v.wrapping_neg()))
            }
            OpCode::FAdd(d, a, b) => self.bin_f64(*d, *a, *b, |x, y| x + y),
            OpCode::FSub(d, a, b) => self.bin_f64(*d, *a, *b, |x, y| x - y),
            OpCode::FMul(d, a, b) => self.bin_f64(*d, *a, *b, |x, y| x * y),
            OpCode::FDiv(d, a, b) => self.bin_f64(*d, *a, *b, |x, y| x / y),

            OpCode::Eq(d, a, b)  => self.bin_cmp(*d, *a, *b, |x, y| x == y),
            OpCode::Neq(d, a, b) => self.bin_cmp(*d, *a, *b, |x, y| x != y),
            OpCode::Lt(d, a, b)  => self.bin_i64_cmp(*d, *a, *b, |x, y| x < y),
            OpCode::Gt(d, a, b)  => self.bin_i64_cmp(*d, *a, *b, |x, y| x > y),
            OpCode::Lte(d, a, b) => self.bin_i64_cmp(*d, *a, *b, |x, y| x <= y),
            OpCode::Gte(d, a, b) => self.bin_i64_cmp(*d, *a, *b, |x, y| x >= y),
            OpCode::FLt(d, a, b) => {
                let x = self.read_f64(*a)?;
                let y = self.read_f64(*b)?;
                let r = if x.is_nan() || y.is_nan() { false } else { x < y };
                self.write(*d, Value::Bool(r))
            }

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
            OpCode::Ret(reg) => self.do_ret(*reg),

            OpCode::PushConst(reg, pool_idx) => {
                let idx = *pool_idx as usize;
                if idx >= bc.constants.len() {
                    return Err("Constant index out of bounds".to_string());
                }
                self.write(*reg, bc.constants[idx].clone())
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
            OpCode::Lea(_, _, _) => {
                Err("lea: not supported under handle/generation model".to_string())
            }
            OpCode::Ref(d, s) => {
                let v = self.read(*s)?;
                self.value_rc_inc(&v)?;
                let (slot, generation) = self.heap.alloc(1);
                self.heap.st(slot, generation, 0, v)?;
                self.write(*d, Value::Handle { slot, generation })
            }

            OpCode::Alloc(d, size) => {
                let (slot, generation) = self.heap.alloc(*size as usize);
                self.write(*d, Value::Handle { slot, generation })
            }
            OpCode::Free(reg) => {
                let (slot, generation) = self.read_handle(*reg)?;
                self.heap.force_free(slot, generation)
            }
            OpCode::Drop(reg) => {
                let abs = self.abs(*reg)?;
                if let Some(v) = self.registers[abs].take() {
                    self.value_rc_dec(&v)?;
                }
                Ok(())
            }

            OpCode::Dei(d, port_reg) => {
                let port_val = self.read_i64(*port_reg)?;
                let (device_id, port) = split_port(port_val)?;
                if device_id == super::DISPATCH_ID && port == super::DISPATCH_PORT_LOOKUP {
                    let r = self.dispatch_last_result.unwrap_or(super::DISPATCH_NO_MATCH);
                    return self.write(*d, Value::Int(r as i64));
                }
                let dev = self.devices.get_mut(device_id)
                    .ok_or_else(|| format!("dei: device {:#04x} not installed", device_id))?;
                let v = dev.read(port)?;
                self.write(*d, v)
            }
            OpCode::Deo(src, port_reg) => {
                let v = self.read(*src)?;
                let port_val = self.read_i64(*port_reg)?;
                let (device_id, port) = split_port(port_val)?;
                if device_id == 0x00 && port == 0x01 {
                    if let Value::Int(n) = v {
                        self.exit_code = Some(n & 0xFFFF_FFFF);
                    }
                    self.halted = true;
                }
                if device_id == super::DISPATCH_ID && port == super::DISPATCH_PORT_LOOKUP {
                    let key = match v {
                        Value::Int(n) if (0..=0xFFFF).contains(&n) => n as u16,
                        _ => return Err(format!("dispatch.lookup: bad key {:?}", v)),
                    };
                    self.dispatch_last_result = Some(self.resolve_dispatch(key));
                    return Ok(());
                }
                let dev = self.devices.get_mut(device_id)
                    .ok_or_else(|| format!("deo: device {:#04x} not installed", device_id))?;
                dev.write(port, v)
            }
            OpCode::Handle(dest, fn_id) => {
                // Heap-alloc continuation cell; `dest` is the suspended frame's result reg.
                let (slot, generation) = self.heap.alloc(super::cont_slot::SIZE);
                self.heap.st(slot, generation, super::cont_slot::SUSPEND_PC,
                    Value::Int(self.pc as i64))?;
                self.heap.st(slot, generation, super::cont_slot::SUSPEND_BASE,
                    Value::Int(self.base_reg as i64))?;
                self.heap.st(slot, generation, super::cont_slot::DEST_REG,
                    Value::Int(dest.to_usize() as i64))?;
                self.heap.st(slot, generation, super::cont_slot::ALIVE,
                    Value::Int(1))?;
                self.handlers.push(super::HandlerFrame {
                    effect_id: 0,
                    handler_fn: *fn_id as usize,
                    dispatch_table_slot: None,
                    dispatch_table_gen: 0,
                    cell_slot: slot,
                    cell_gen: generation,
                });
                Ok(())
            }
            OpCode::Resume(reg) => {
                // Single-shot: alive check, restore (pc, base, dest), mark dead, free cell.
                let frame = self.handlers.pop()
                    .ok_or("Resume outside an active handler frame")?;
                let alive = self.heap.ld(frame.cell_slot, frame.cell_gen,
                    super::cont_slot::ALIVE)?;
                match alive {
                    Value::Int(1) => {}
                    Value::Int(0) => return Err(
                        "resume: continuation already consumed".to_string()),
                    other => return Err(format!(
                        "resume: continuation cell corrupted (alive slot = {:?})", other)),
                }
                let saved_pc = match self.heap.ld(frame.cell_slot, frame.cell_gen,
                    super::cont_slot::SUSPEND_PC)? {
                    Value::Int(n) if n >= 0 => n as usize,
                    other => return Err(format!(
                        "resume: continuation cell has invalid pc {:?}", other)),
                };
                let saved_base = match self.heap.ld(frame.cell_slot, frame.cell_gen,
                    super::cont_slot::SUSPEND_BASE)? {
                    Value::Int(n) if n >= 0 => n as usize,
                    other => return Err(format!(
                        "resume: continuation cell has invalid base {:?}", other)),
                };
                let dest_reg = match self.heap.ld(frame.cell_slot, frame.cell_gen,
                    super::cont_slot::DEST_REG)? {
                    Value::Int(n) if (0..256).contains(&n) => n as usize,
                    other => return Err(format!(
                        "resume: continuation cell has invalid dest_reg {:?}", other)),
                };
                let val = self.read(*reg)?;
                self.heap.st(frame.cell_slot, frame.cell_gen,
                    super::cont_slot::ALIVE, Value::Int(0))?;
                self.pc = saved_pc;
                self.base_reg = saved_base;
                let abs = saved_base + dest_reg;
                if abs >= self.registers.len() {
                    return Err(format!(
                        "resume: dest r{} (abs {}) out of register window (len {})",
                        dest_reg, abs, self.registers.len()
                    ));
                }
                self.registers[abs] = Some(val);
                self.heap.force_free(frame.cell_slot, frame.cell_gen)?;
                Ok(())
            }
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
            self.registers.resize(needed, None);
        }

        if let Chunk::Native(n) = &module.functions[fn_id] {
            let mut args: Vec<Value> = Vec::with_capacity(n.param_count);
            for i in 0..n.param_count {
                let slot = new_base + i;
                let v = self.registers[slot].clone()
                    .ok_or_else(|| format!("native call: arg {} (r{}) is empty", i, i))?;
                args.push(v);
            }
            let result = (n.func)(&args)?;
            self.registers[dest_abs] = Some(result);
            return Ok(());
        }

        if self.frames.len() >= MAX_RECURSION_DEPTH {
            return Err(format!(
                "Stack overflow: recursion depth {} exceeds limit {}",
                self.frames.len(),
                MAX_RECURSION_DEPTH
            ));
        }
        self.frames.push(Frame {
            func_id: self.current_func,
            ip: self.pc,
            base_reg: self.base_reg,
            dest_reg: dest_abs,
        });
        self.base_reg = new_base;
        self.current_func = fn_id;
        self.pc = 0;
        Ok(())
    }

    #[inline]
    fn do_ret(&mut self, reg: Register) -> Result<(), String> {
        let abs = self.abs(reg)?;
        let return_val = self.registers[abs].take()
            .ok_or("Return register is empty")?;
        if let Some(frame) = self.frames.pop() {
            self.pc = frame.ip;
            self.base_reg = frame.base_reg;
            self.current_func = frame.func_id;
            self.registers[frame.dest_reg] = Some(return_val);
            Ok(())
        } else {
            self.registers[self.base_reg] = Some(return_val);
            self.halted = true;
            Ok(())
        }
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

    // Frame-local r → absolute slot. The calling convention stages outbound
    // args at r{caller_reg_count..}, so the upper bound is FRAME_REGS, not
    // the chunk's own reg_count (S8).
    #[inline(always)]
    fn abs(&self, r: Register) -> Result<usize, String> {
        let abs = self.base_reg + r.to_usize();
        if abs >= self.registers.len() {
            return Err(format!(
                "r{} (abs {}) out of register window (len {})",
                r.0, abs, self.registers.len()
            ));
        }
        Ok(abs)
    }

    #[inline(always)]
    fn read(&self, r: Register) -> Result<Value, String> {
        let abs = self.abs(r)?;
        self.registers[abs].clone()
            .ok_or_else(|| format!("read: r{} is empty", r.0))
    }

    #[inline(always)]
    fn take(&mut self, r: Register) -> Result<Value, String> {
        let abs = self.abs(r)?;
        self.registers[abs].take()
            .ok_or_else(|| format!("move: r{} is empty (already moved?)", r.0))
    }

    #[inline(always)]
    fn write(&mut self, r: Register, v: Value) -> Result<(), String> {
        let abs = self.abs(r)?;
        self.registers[abs] = Some(v);
        Ok(())
    }

    // Hot fast path: scalars take zero refcount edges.
    #[inline(always)]
    fn value_rc_inc(&mut self, v: &Value) -> Result<(), String> {
        match v {
            Value::Int(_) | Value::Float(_) | Value::Bool(_)
            | Value::Char(_) | Value::Unit => Ok(()),
            _ => self.value_rc_inc_slow(v),
        }
    }

    fn value_rc_inc_slow(&mut self, v: &Value) -> Result<(), String> {
        match v {
            Value::Handle { slot, generation } => self.heap.rc_inc(*slot, *generation),
            Value::Closure { env_slot, env_gen, .. } => self.heap.rc_inc(*env_slot, *env_gen),
            _ => Ok(()),
        }
    }

    #[inline(always)]
    fn value_rc_dec(&mut self, v: &Value) -> Result<(), String> {
        match v {
            Value::Int(_) | Value::Float(_) | Value::Bool(_)
            | Value::Char(_) | Value::Unit => Ok(()),
            _ => self.value_rc_dec_slow(v),
        }
    }

    fn value_rc_dec_slow(&mut self, v: &Value) -> Result<(), String> {
        match v {
            Value::Handle { slot, generation } => self.heap.rc_dec(*slot, *generation).map(|_| ()),
            Value::Closure { env_slot, env_gen, .. } => self.heap.rc_dec(*env_slot, *env_gen).map(|_| ()),
            _ => Ok(()),
        }
    }

    #[inline(always)]
    fn read_i64(&self, r: Register) -> Result<i64, String> {
        let abs = self.abs(r)?;
        match &self.registers[abs] {
            Some(Value::Int(n)) => Ok(*n),
            Some(v) => Err(format!("expected i64, got {:?}", v)),
            None => Err(format!("read: r{} is empty", r.0)),
        }
    }

    #[inline(always)]
    fn read_f64(&self, r: Register) -> Result<f64, String> {
        let abs = self.abs(r)?;
        match &self.registers[abs] {
            Some(Value::Float(n)) => Ok(*n),
            Some(v) => Err(format!("expected f64, got {:?}", v)),
            None => Err(format!("read: r{} is empty", r.0)),
        }
    }

    #[inline(always)]
    fn read_handle(&self, r: Register) -> Result<(u32, u32), String> {
        let abs = self.abs(r)?;
        match &self.registers[abs] {
            Some(Value::Handle { slot, generation }) => Ok((*slot, *generation)),
            Some(v) => Err(format!("expected handle, got {:?}", v)),
            None => Err(format!("read: r{} is empty", r.0)),
        }
    }

    #[inline(always)]
    fn bin_i64<F: Fn(i64, i64) -> i64>(&mut self, d: Register, a: Register, b: Register, f: F) -> Result<(), String> {
        let x = self.read_i64(a)?;
        let y = self.read_i64(b)?;
        self.write(d, Value::Int(f(x, y)))
    }

    #[inline(always)]
    fn bin_i64_checked<F: Fn(i64, i64) -> Option<i64>>(&mut self, d: Register, a: Register, b: Register, msg: &str, f: F) -> Result<(), String> {
        let x = self.read_i64(a)?;
        let y = self.read_i64(b)?;
        let r = f(x, y).ok_or_else(|| msg.to_string())?;
        self.write(d, Value::Int(r))
    }

    #[inline(always)]
    fn bin_f64<F: Fn(f64, f64) -> f64>(&mut self, d: Register, a: Register, b: Register, f: F) -> Result<(), String> {
        let x = self.read_f64(a)?;
        let y = self.read_f64(b)?;
        self.write(d, Value::Float(f(x, y)))
    }

    #[inline(always)]
    fn bin_cmp<F: Fn(&Value, &Value) -> bool>(&mut self, d: Register, a: Register, b: Register, f: F) -> Result<(), String> {
        let x = self.read(a)?;
        let y = self.read(b)?;
        self.write(d, Value::Bool(f(&x, &y)))
    }

    #[inline(always)]
    fn bin_i64_cmp<F: Fn(i64, i64) -> bool>(&mut self, d: Register, a: Register, b: Register, f: F) -> Result<(), String> {
        let x = self.read_i64(a)?;
        let y = self.read_i64(b)?;
        self.write(d, Value::Bool(f(x, y)))
    }

    fn validate_module_devices(&self, module: &Module) -> Result<(), String> {
        for id in 0u16..=255 {
            let id = id as u8;
            if id == super::DISPATCH_ID { continue; }
            if module.requires_device(id) && !self.devices.has(id) {
                return Err(format!("module requires device {:#04x} but it is not installed", id));
            }
        }
        Ok(())
    }

    // Dispatch device 0xE0 lookup
    fn resolve_dispatch(&self, key: u16) -> u16 {
        let effect_id = (key >> 8) as u16;
        let op_id = (key & 0xFF) as usize;
        for h in self.handlers.iter().rev() {
            if h.effect_id != effect_id { continue; }
            if let Some(slot) = h.dispatch_table_slot {
                if let Ok(v) = self.heap.ld(slot, h.dispatch_table_gen, op_id) {
                    if let Value::Int(n) = v {
                        if (0..=0xFFFF).contains(&n) { return n as u16; }
                    }
                }
            }
            if op_id == 0 && h.handler_fn <= 0xFFFF {
                return h.handler_fn as u16;
            }
        }
        super::DISPATCH_NO_MATCH
    }
}

fn is_falsy(val: &Value) -> bool {
    match val {
        Value::Bool(b) => !b,
        Value::Int(i) => *i == 0,
        Value::Unit => true,
        _ => false,
    }
}

#[allow(dead_code)]
fn expect_bytecode(module: &Module, fn_id: usize) -> Result<&BytecodeChunk, String> {
    match &module.functions[fn_id] {
        Chunk::Bytecode(b) => Ok(b),
        Chunk::Native(_) => Err(format!("expected bytecode chunk at fn {}, found native", fn_id)),
    }
}

// 16-bit port = (device_id << 8) | port_offset (spec §3.8).
fn split_port(port_val: i64) -> Result<(u8, u8), String> {
    if !(0..=0xFFFF).contains(&port_val) {
        return Err(format!("device port {:#x} out of 16-bit range", port_val));
    }
    let device_id = ((port_val >> 8) & 0xFF) as u8;
    let port = (port_val & 0xFF) as u8;
    Ok((device_id, port))
}

// Reject chunks declaring more than FRAME_REGS registers (spec §2).
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
