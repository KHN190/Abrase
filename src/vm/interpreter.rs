use super::{VirtualMachine, Value};
use crate::bytecode::{Chunk, OpCode, Register, Module};
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
        self.pc = 0;
        self.base_reg = 0;
        self.current_func = module.entry;
        self.frames.clear();
        let needed = 256;
        if needed > self.registers.len() {
            self.registers.resize(needed, None);
        }

        loop {
            let current_chunk = &module.functions[self.current_func];

            if self.pc >= current_chunk.code.len() {
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

            let opcode = current_chunk.code[self.pc].clone();
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

            OpCode::Jmp(off) => { self.branch(*off); Ok(()) }
            OpCode::Jz(r, off) => {
                let v = self.read(*r)?;
                if is_falsy(&v) { self.branch(*off); }
                Ok(())
            }
            OpCode::Jnz(r, off) => {
                let v = self.read(*r)?;
                if !is_falsy(&v) { self.branch(*off); }
                Ok(())
            }
            OpCode::Call(dest, fn_id) => self.do_call(module, *dest, *fn_id as usize),
            OpCode::Ret(reg) => self.do_ret(*reg),

            OpCode::PushConst(reg, pool_idx) => {
                let chunk = &module.functions[self.current_func];
                let idx = *pool_idx as usize;
                if idx >= chunk.constants.len() {
                    return Err("Constant index out of bounds".to_string());
                }
                self.write(*reg, chunk.constants[idx].clone())
            }
            OpCode::Copy(d, s) => {
                let v = self.read(*s)?;
                self.write(*d, v)
            }
            OpCode::Move(d, s) => {
                let v = self.take(*s)?;
                self.write(*d, v)
            }

            OpCode::Ld(d, b, off) => {
                let h = self.read_handle(*b)?;
                let v = self.heap.ld(h, *off as usize)?;
                self.write(*d, v)
            }
            OpCode::St(s, b, off) => {
                let h = self.read_handle(*b)?;
                let v = self.read(*s)?;
                self.heap.st(h, *off as usize, v)
            }
            OpCode::LdIdx(d, b, i) => {
                let h = self.read_handle(*b)?;
                let off = self.read_i64(*i)? as usize;
                let v = self.heap.ld(h, off)?;
                self.write(*d, v)
            }
            OpCode::StIdx(s, b, i) => {
                let h = self.read_handle(*b)?;
                let off = self.read_i64(*i)? as usize;
                let v = self.read(*s)?;
                self.heap.st(h, off, v)
            }
            OpCode::Lea(d, b, off) => {
                let h = self.read_handle(*b)? as i64;
                self.write(*d, Value::Int(h + *off as i64))
            }
            OpCode::Ref(d, s) => {
                let v = self.read(*s)?;
                let h = self.heap.alloc(1);
                self.heap.st(h, 0, v)?;
                self.write(*d, Value::Int(h as i64))
            }

            OpCode::Alloc(d, size) => {
                let h = self.heap.alloc(*size as usize) as i64;
                self.write(*d, Value::Int(h))
            }
            OpCode::Free(reg) => {
                let h = self.read_handle(*reg)?;
                self.heap.free(h)
            }
            OpCode::Drop(reg) => {
                let abs = self.base_reg + reg.to_usize();
                self.registers[abs] = None;
                Ok(())
            }

            OpCode::Dei(_, _) | OpCode::Deo(_, _) => {
                Err("device I/O not yet implemented".to_string())
            }
            OpCode::Handle(dest, fn_id) => {
                // Install a handler frame pointing at the given function. The
                // dispatch wiring (which effect operation goes to which arm) is
                // handled by codegen via direct calls; this opcode just records
                // that a handler is currently active so `Resume` can find it.
                let _ = dest;
                self.handlers.push(super::HandlerFrame {
                    handler_fn: *fn_id as usize,
                    saved_pc: self.pc,
                    saved_base: self.base_reg,
                });
                Ok(())
            }
            OpCode::Resume(reg) => {
                // Single-shot for now: pop the most-recent handler and return
                // its value as the handler-frame's result. The continuation
                // value lives in `reg` (the operation's return value).
                let frame = self.handlers.pop()
                    .ok_or("Resume outside an active handler frame")?;
                let val = self.read(*reg)?;
                self.pc = frame.saved_pc;
                self.base_reg = frame.saved_base;
                self.registers[self.base_reg] = Some(val);
                Ok(())
            }
        }
    }

    fn do_call(&mut self, module: &Module, dest: Register, fn_id: usize) -> Result<(), String> {
        if fn_id >= module.functions.len() {
            return Err(format!("call: unknown fn_id {}", fn_id));
        }
        let caller_chunk = &module.functions[self.current_func];
        let caller_reg_count = caller_chunk.reg_count;
        let dest_abs = self.base_reg + dest.to_usize();
        let new_base = self.base_reg + caller_reg_count;
        let needed = new_base + 256;
        if needed > MAX_REGISTERS {
            return Err(format!(
                "Stack overflow: register window {} exceeds limit {}",
                needed, MAX_REGISTERS
            ));
        }
        if needed > self.registers.len() {
            self.registers.resize(needed, None);
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

    fn do_ret(&mut self, reg: Register) -> Result<(), String> {
        let abs = self.base_reg + reg.to_usize();
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
            self.pc = usize::MAX;
            Ok(())
        }
    }

    fn branch(&mut self, offset: i16) {
        let new_pc = (self.pc as isize) + (offset as isize);
        self.pc = new_pc as usize;
    }

    fn read(&self, r: Register) -> Result<Value, String> {
        self.registers[self.base_reg + r.to_usize()].clone()
            .ok_or_else(|| format!("read: r{} is empty", r.0))
    }

    fn take(&mut self, r: Register) -> Result<Value, String> {
        self.registers[self.base_reg + r.to_usize()].take()
            .ok_or_else(|| format!("move: r{} is empty (already moved?)", r.0))
    }

    fn write(&mut self, r: Register, v: Value) -> Result<(), String> {
        self.registers[self.base_reg + r.to_usize()] = Some(v);
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

    fn read_handle(&self, r: Register) -> Result<usize, String> {
        match self.read(r)? {
            Value::Int(n) => Ok(n as usize),
            v => Err(format!("expected pointer, got {:?}", v)),
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
