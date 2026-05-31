use self::register::{bool_u64, validate_module_register_budget};
use super::{VirtualMachine, Value};
use super::debug::DebugEvent;
use polka::{BytecodeChunk, Chunk, OpCode, Register, Module, FRAME_REGS, HANDLE_NONE};
use crate::frame::Frame;
use crate::memory::mask_bit;

pub mod frame;
pub mod effect;
pub mod register;

pub(self) const MAX_REGISTERS: usize = 1 << 16;
pub(self) const MAX_RECURSION_DEPTH: usize = 2048;
// Slack for materializing param Moves before Call opcode (see stage_call_args).
pub(self) const STAGE_SLACK: usize = 32;

pub const MAX_RAM: usize = 64 * 1024 * 1024;

impl VirtualMachine {
    pub fn run(&mut self, chunk: &Chunk) -> Result<Value, String> {
        let module = Module { functions: vec![chunk.clone()], entry: 0, flags: 0, exports: vec![] };
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

    pub fn call_export(
        &mut self,
        module: &Module,
        name: &str,
        args: &[Value],
    ) -> Result<Value, String> {
        let export = module.exports.iter().find(|e| e.name == name)
            .ok_or_else(|| format!("no export named '{}'", name))?;
        let fn_id = export.fn_id as usize;
        if fn_id >= module.functions.len() {
            return Err(format!("export '{}' fn_id {} out of range", name, fn_id));
        }
        let param_count = module.functions[fn_id].param_count();
        if args.len() != param_count {
            return Err(format!(
                "export '{}' expects {} arg(s), got {}",
                name, param_count, args.len()
            ));
        }
        let r = self.run_export_inner(module, fn_id, args);
        r.map_err(|e| format!(
            "[{}:{}] {}",
            super::debug::render_fn_label(self.current_func, &self.fn_names),
            self.failing_pc, e,
        ))
    }

    fn run_export_inner(
        &mut self,
        module: &Module,
        fn_id: usize,
        args: &[Value],
    ) -> Result<Value, String> {
        if self.exit_code.is_some() {
            return Ok(Value::from_int(self.exit_code.unwrap()));
        }
        validate_module_register_budget(module)?;
        self.int32_safe = (module.flags & polka::CART_FLAG_INT32_SAFE) != 0;
        if self.resolved_constants.is_empty() {
            self.frames.clear();
            self.handlers.clear();
            self.region_table.clear();
            self.heap.clear();
            self.string_const_handles.clear();
            self.resolve_constants(module)?;
            // First entry into this module: build statics once. They persist
            // across later call_export invocations (heap is not cleared below).
            self.module_table_raw = HANDLE_NONE;
            self.module_table_is_handle = false;
            self.run_module_init(module)?;
        } else {
            self.frames.clear();
            self.handlers.clear();
        }
        self.pc = 0;
        self.base_reg = 0;
        self.current_func = fn_id;
        self.halted = false;
        let needed = FRAME_REGS + STAGE_SLACK;
        self.ensure_registers(needed);
        for (i, v) in args.iter().enumerate() {
            self.write_abs_raw(i, v.raw());
            self.set_reg_mask_bit(i, false);
        }
        if self.debug_sink.is_some() {
            self.run_loop::<true>(module)
        } else {
            self.run_loop::<false>(module)
        }
    }

    pub fn reset(&mut self) {
        self.frames.clear();
        self.handlers.clear();
        self.region_table.clear();
        self.heap.clear();
        self.string_const_handles.clear();
        self.resolved_constants.clear();
        self.resolved_const_mask.clear();
        self.resolved_natives.clear();
        self.halted = false;
        self.exit_code = None;
        self.pc = 0;
        self.base_reg = 0;
        self.module_table_raw = HANDLE_NONE;
        self.module_table_is_handle = false;
    }

    fn run_module_inner(&mut self, module: &Module) -> Result<Value, String> {
        validate_module_register_budget(module)?;
        self.int32_safe = (module.flags & polka::CART_FLAG_INT32_SAFE) != 0;
        self.frames.clear();
        self.handlers.clear();
        self.region_table.clear();
        self.heap.clear();
        self.string_const_handles.clear();
        self.resolve_constants(module)?;
        self.module_table_raw = HANDLE_NONE;
        self.module_table_is_handle = false;
        self.run_module_init(module)?;
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

    // Run the synthetic `__module_init` (if the module has statics) exactly
    // once, before the entry/export. It builds the module table and stores it
    // via the MODULE device; its result is discarded.
    fn run_module_init(&mut self, module: &Module) -> Result<(), String> {
        let Some(fn_id) = module.exports.iter()
            .find(|e| e.name == "__module_init")
            .map(|e| e.fn_id as usize)
        else { return Ok(()); };
        self.pc = 0;
        self.base_reg = 0;
        self.current_func = fn_id;
        self.halted = false;
        self.exit_code = None;
        let needed = FRAME_REGS + STAGE_SLACK;
        self.ensure_registers(needed);
        if self.debug_sink.is_some() {
            self.run_loop::<true>(module)?;
        } else {
            self.run_loop::<false>(module)?;
        }
        Ok(())
    }

    fn run_loop<const TRACE: bool>(&mut self, module: &Module) -> Result<Value, String> {
        'outer: loop {
            if self.halted {
                if let Some(code) = self.exit_code {
                    self.last_result_is_handle = false;
                    return Ok(Value::from_int(code));
                }
                let v = self.read_abs_raw(self.base_reg);
                self.last_result_is_handle = self.reg_mask_bit(self.base_reg);
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
                        self.last_result_is_handle = self.reg_mask_bit(self.base_reg);
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
                self.steps = self.steps.wrapping_add(1);
                if let Some(cap) = self.step_cap {
                    if self.steps > cap {
                        self.failing_pc = opcode_pc;
                        return Err(format!("step cap exceeded ({} ops)", cap));
                    }
                }
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
                let bits = self.narrow_float_bits(-x);
                self.write(*d, bits, false)
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

            OpCode::PushConst(reg, pool_idx) => self.exec_push_const(*reg, *pool_idx),
            OpCode::Copy(d, s) => {
                let (v, is_handle) = self.read(*s)?;
                if is_handle { self.rc_inc_handle(v)?; }
                self.write(*d, v, is_handle)
            }
            OpCode::Move(d, s) => {
                let (v, is_handle) = self.take(*s)?;
                self.write(*d, v, is_handle)
            }

            OpCode::Ld(d, b, off)      => self.exec_ld(*d, *b, *off as i64),
            OpCode::St(src, b, off)    => self.exec_st(*src, *b, *off as i64),
            OpCode::LdIdx(d, b, i)     => { let off = self.read_i64(*i)?; if off < 0 { return Err(format!("ldidx: negative index {}", off)); } self.exec_ld(*d, *b, off) }
            OpCode::StIdx(src, b, i)   => { let off = self.read_i64(*i)?; if off < 0 { return Err(format!("stidx: negative index {}", off)); } self.exec_st(*src, *b, off) }
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
            OpCode::Handle(table_reg, effect_id) => self.exec_handler_push(*table_reg, *effect_id),
            OpCode::Resume(dest_reg, val_reg) => self.do_resume(module, *dest_reg, *val_reg),
            OpCode::Raise(dest, key_reg, args_base) => self.do_raise(module, bc, *dest, *key_reg, *args_base),
        }
    }

    #[inline(always)]
    fn exec_push_const(&mut self, reg: Register, pool_idx: u16) -> Result<(), String> {
        let idx = pool_idx as usize;
        let consts = &self.resolved_constants[self.current_func];
        let mask  = &self.resolved_const_mask[self.current_func];
        if idx >= consts.len() {
            return Err("Constant index out of bounds".to_string());
        }
        let raw = consts[idx];
        let is_handle = mask_bit(mask, idx);
        if is_handle { self.rc_inc_handle(raw)?; }
        self.write(reg, raw, is_handle)
    }

    #[inline(always)]
    fn exec_ld(&mut self, d: Register, b: Register, off: i64) -> Result<(), String> {
        let (slot, gen_) = self.read_handle(b)?;
        let (raw, is_handle) = self.heap.ld(slot, gen_, off as usize)?;
        if is_handle { self.rc_inc_handle(raw)?; }
        self.trace_static_access("Ld", slot, off, raw, is_handle);
        self.write(d, raw, is_handle)
    }

    #[inline(always)]
    fn exec_st(&mut self, src: Register, b: Register, off: i64) -> Result<(), String> {
        let (slot, gen_) = self.read_handle(b)?;
        let (raw, is_handle) = self.take(src)?;
        self.trace_static_access("St", slot, off, raw, is_handle);
        let (old_raw, old_is_handle) = self.heap.st(slot, gen_, off as usize, raw, is_handle)?;
        if old_is_handle { self.rc_dec_handle(old_raw)?; }
        Ok(())
    }

    fn trace_static_access(&self, op: &str, slot: u32, off: i64, raw: u64, is_handle: bool) {
        let traced = std::env::var("TRACE_STATIC").ok();
        let traced = match traced.as_deref() { Some(s) if !s.is_empty() => s, _ => return };
        let idx = off as usize;
        let name = self.static_names.get(idx).map(|s| s.as_str()).unwrap_or("");
        if name.is_empty() { return; }
        if traced == "*" || name.contains(traced) {
            let val = if is_handle {
                format!("handle({:#x})", raw)
            } else {
                format!("int({}) / float({:.6})", raw as i64, f64::from_bits(raw))
            };
            eprintln!("[TRACE_STATIC] {} slot={} off={} name={} val={}", op, slot, off, name, val);
        }
    }

    #[inline(always)]
    fn exec_handler_push(&mut self, table_reg: Register, effect_id: u16) -> Result<(), String> {
        let (table_raw, table_is_handle) = self.read_at(table_reg);
        let (table_slot, table_gen) = if table_is_handle && table_raw != HANDLE_NONE {
            let (s, g) = Self::decode_handle(table_raw);
            (Some(s), g)
        } else { (None, 0) };
        self.handlers.push(super::HandlerFrame {
            effect_id,
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
        self.trace_frame_event("HANDLER push", format_args!("effect={:#04x}", effect_id));
        Ok(())
    }
}
