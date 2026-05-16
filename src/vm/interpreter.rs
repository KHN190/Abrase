use super::{VirtualMachine, Value};
use crate::bytecode::{BytecodeChunk, Chunk, OpCode, Register, Module, FRAME_REGS};
use crate::vm::frame::Frame;

const MAX_REGISTERS: usize = 1 << 16;
const MAX_RECURSION_DEPTH: usize = 2048;

impl VirtualMachine {
    pub fn run(&mut self, chunk: &Chunk) -> Result<Value, String> {
        let module = Module { functions: vec![chunk.clone()], entry: 0 };
        self.run_module(&module)
    }

    pub fn run_module(&mut self, module: &Module) -> Result<Value, String> {
        let r = self.run_module_inner(module);
        r.map_err(|e| format!("[fn {} pc {}] {}", self.current_func, self.pc, e))
    }

    fn run_module_inner(&mut self, module: &Module) -> Result<Value, String> {
        validate_module_register_budget(module)?;
        self.pc = 0;
        self.base_reg = 0;
        self.current_func = module.entry;
        self.frames.clear();
        self.handlers.clear();
        self.halted = false;
        let needed = FRAME_REGS;
        if needed > self.registers.len() {
            self.registers.resize(needed, None);
        }

        loop {
            if self.halted {
                return self.registers[self.base_reg].clone()
                    .ok_or("Return register is empty".to_string());
            }
            let bc = match &module.functions[self.current_func] {
                Chunk::Bytecode(b) => b,
                Chunk::Native(_) => return Err(format!(
                    "entry fn {} is native; cannot start execution there", self.current_func
                )),
            };

            if self.pc >= bc.code.len() {
                if let Some(frame) = self.frames.pop() {
                    let return_val = self.registers[self.base_reg].clone()
                        .ok_or("Return register is empty")?;
                    self.pc = frame.ip;
                    self.base_reg = frame.base_reg;
                    self.current_func = frame.func_id;
                    self.registers[frame.dest_reg] = Some(return_val);
                    continue;
                } else {
                    return self.registers[self.base_reg].clone()
                        .ok_or("Return register is empty".to_string());
                }
            }

            let opcode = bc.code[self.pc].clone();
            self.pc += 1;
            self.exec(module, &opcode)?;
        }
    }

    fn exec(&mut self, module: &Module, op: &OpCode) -> Result<(), String> {
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

            OpCode::Jmp(off) => self.branch(module, *off),
            OpCode::Jz(r, off) => {
                let v = self.read(*r)?;
                if is_falsy(&v) { self.branch(module, *off) } else { Ok(()) }
            }
            OpCode::Jnz(r, off) => {
                let v = self.read(*r)?;
                if !is_falsy(&v) { self.branch(module, *off) } else { Ok(()) }
            }
            OpCode::Call(dest, fn_id) => self.do_call(module, *dest, *fn_id as usize),
            OpCode::Ret(reg) => self.do_ret(*reg),

            OpCode::PushConst(reg, pool_idx) => {
                let bc = expect_bytecode(module, self.current_func)?;
                let idx = *pool_idx as usize;
                if idx >= bc.constants.len() {
                    return Err("Constant index out of bounds".to_string());
                }
                self.write(*reg, bc.constants[idx].clone())
            }
            OpCode::Copy(d, s) => {
                let v = self.read(*s)?;
                if let Value::Handle { slot, generation } = v {
                    self.heap.rc_inc(slot, generation)?;
                }
                self.write(*d, v)
            }
            OpCode::Move(d, s) => {
                let v = self.take(*s)?;
                self.write(*d, v)
            }

            OpCode::Ld(d, b, off) => {
                let (slot, generation) = self.read_handle(*b)?;
                let v = self.heap.ld(slot, generation, *off as usize)?;
                if let Value::Handle { slot: s2, generation: g2 } = v {
                    self.heap.rc_inc(s2, g2)?;
                }
                self.write(*d, v)
            }
            OpCode::St(src, b, off) => {
                let (slot, generation) = self.read_handle(*b)?;
                let v = self.take(*src)?;
                let old = self.heap.st(slot, generation, *off as usize, v)?;
                if let Value::Handle { slot: s2, generation: g2 } = old {
                    self.heap.rc_dec(s2, g2)?;
                }
                Ok(())
            }
            OpCode::LdIdx(d, b, i) => {
                let (slot, generation) = self.read_handle(*b)?;
                let off = self.read_i64(*i)?;
                if off < 0 {
                    return Err(format!("ldidx: negative index {}", off));
                }
                let v = self.heap.ld(slot, generation, off as usize)?;
                if let Value::Handle { slot: s2, generation: g2 } = v {
                    self.heap.rc_inc(s2, g2)?;
                }
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
                if let Value::Handle { slot: s2, generation: g2 } = old {
                    self.heap.rc_dec(s2, g2)?;
                }
                Ok(())
            }
            OpCode::Lea(_, _, _) => {
                Err("lea: not supported under handle/generation model".to_string())
            }
            OpCode::Ref(d, s) => {
                let v = self.read(*s)?;
                if let Value::Handle { slot: s2, generation: g2 } = v {
                    self.heap.rc_inc(s2, g2)?;
                }
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
                if let Some(Value::Handle { slot, generation }) = self.registers[abs].take() {
                    self.heap.rc_dec(slot, generation)?;
                }
                Ok(())
            }

            OpCode::Dei(_, _) | OpCode::Deo(_, _) => {
                Err("device I/O not yet implemented".to_string())
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
                    handler_fn: *fn_id as usize,
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

    fn do_call(&mut self, module: &Module, dest: Register, fn_id: usize) -> Result<(), String> {
        if fn_id >= module.functions.len() {
            return Err(format!("call: unknown fn_id {}", fn_id));
        }
        let caller_bc = expect_bytecode(module, self.current_func)?;
        let caller_reg_count = caller_bc.reg_count;
        if caller_reg_count > FRAME_REGS {
            return Err(format!(
                "call: caller fn {} has reg_count {} > frame budget {}",
                self.current_func, caller_reg_count, FRAME_REGS
            ));
        }
        let callee_reg_count = match &module.functions[fn_id] {
            Chunk::Bytecode(b) => b.reg_count,
            Chunk::Native(_) => 0,
        };
        if callee_reg_count > FRAME_REGS {
            return Err(format!(
                "call: callee fn {} has reg_count {} > frame budget {}",
                fn_id, callee_reg_count, FRAME_REGS
            ));
        }
        // dest must land within the caller's window
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
            reg_count: caller_reg_count,
        });
        self.base_reg = new_base;
        self.current_func = fn_id;
        self.pc = 0;
        Ok(())
    }

    fn do_ret(&mut self, reg: Register) -> Result<(), String> {
        let abs = self.abs(reg)?;
        let return_val = self.registers[abs].clone()
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

    fn branch(&mut self, module: &Module, offset: i16) -> Result<(), String> {
        let bc = expect_bytecode(module, self.current_func)?;
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

    fn read(&self, r: Register) -> Result<Value, String> {
        let abs = self.abs(r)?;
        self.registers[abs].clone()
            .ok_or_else(|| format!("read: r{} is empty", r.0))
    }

    fn take(&mut self, r: Register) -> Result<Value, String> {
        let abs = self.abs(r)?;
        self.registers[abs].take()
            .ok_or_else(|| format!("move: r{} is empty (already moved?)", r.0))
    }

    fn write(&mut self, r: Register, v: Value) -> Result<(), String> {
        let abs = self.abs(r)?;
        self.registers[abs] = Some(v);
        Ok(())
    }

    fn read_i64(&self, r: Register) -> Result<i64, String> {
        match self.read(r)? {
            Value::Int(n) => Ok(n),
            v => Err(format!("expected i64, got {:?}", v)),
        }
    }

    fn read_f64(&self, r: Register) -> Result<f64, String> {
        match self.read(r)? {
            Value::Float(n) => Ok(n),
            v => Err(format!("expected f64, got {:?}", v)),
        }
    }

    fn read_handle(&self, r: Register) -> Result<(u32, u32), String> {
        match self.read(r)? {
            Value::Handle { slot, generation } => Ok((slot, generation)),
            v => Err(format!("expected handle, got {:?}", v)),
        }
    }

    fn bin_i64<F: Fn(i64, i64) -> i64>(&mut self, d: Register, a: Register, b: Register, f: F) -> Result<(), String> {
        let x = self.read_i64(a)?;
        let y = self.read_i64(b)?;
        self.write(d, Value::Int(f(x, y)))
    }

    fn bin_i64_checked<F: Fn(i64, i64) -> Option<i64>>(&mut self, d: Register, a: Register, b: Register, msg: &str, f: F) -> Result<(), String> {
        let x = self.read_i64(a)?;
        let y = self.read_i64(b)?;
        let r = f(x, y).ok_or_else(|| msg.to_string())?;
        self.write(d, Value::Int(r))
    }

    fn bin_f64<F: Fn(f64, f64) -> f64>(&mut self, d: Register, a: Register, b: Register, f: F) -> Result<(), String> {
        let x = self.read_f64(a)?;
        let y = self.read_f64(b)?;
        self.write(d, Value::Float(f(x, y)))
    }

    fn bin_cmp<F: Fn(&Value, &Value) -> bool>(&mut self, d: Register, a: Register, b: Register, f: F) -> Result<(), String> {
        let x = self.read(a)?;
        let y = self.read(b)?;
        self.write(d, Value::Bool(f(&x, &y)))
    }

    fn bin_i64_cmp<F: Fn(i64, i64) -> bool>(&mut self, d: Register, a: Register, b: Register, f: F) -> Result<(), String> {
        let x = self.read_i64(a)?;
        let y = self.read_i64(b)?;
        self.write(d, Value::Bool(f(x, y)))
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

fn expect_bytecode(module: &Module, fn_id: usize) -> Result<&BytecodeChunk, String> {
    match &module.functions[fn_id] {
        Chunk::Bytecode(b) => Ok(b),
        Chunk::Native(_) => Err(format!("expected bytecode chunk at fn {}, found native", fn_id)),
    }
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
