use super::runtime;
use runtime::{Word, Int, Float, HANDLE_NONE};
use alloc::{string::{String, ToString}, vec::Vec};
use self::register::{validate_module_register_budget, validate_module_profile};
use super::{VirtualMachine, Value};
use super::debug::DebugEvent;
use polka::{BytecodeChunk, Chunk, OpCode, Register, Module, FRAME_REGS};
use runtime::frame::Frame;
use runtime::memory::mask_bit;

pub mod frame;
pub mod effect;
pub mod register;

pub(self) const MAX_REGISTERS: usize = 1 << 16;
pub(self) const MAX_RECURSION_DEPTH: usize = 2048;
// Slack for materializing param Moves before Call opcode (see stage_call_args).
pub(self) const STAGE_SLACK: usize = 32;

pub const MAX_RAM: usize = 64 * 1024 * 1024;

fn diagnostic_text_contains(text: &str, needle: &str) -> bool {
    needle.is_empty() || text.as_bytes().windows(needle.len()).any(|window| window == needle.as_bytes())
}

#[cfg(test)]
mod trace_format_tests {
    extern crate std;
    use super::*;
    use core::cell::RefCell;
    std::thread_local! { static TRACE: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) }; }
    fn capture(text: &str) { TRACE.with(|trace| trace.borrow_mut().push(text.into())); }

    #[test]
    fn diagnostic_filter_keeps_exact_utf8_substring_semantics() {
        for text in ["", "abc", "aaaab", "你好🌍", "name::value"] {
            for needle in ["", "a", "ab", "aaaab", "你好", "好🌍", "🌍", "value", "missing"] {
                assert_eq!(diagnostic_text_contains(text, needle), text.contains(needle));
            }
        }
    }

    #[test]
    fn static_trace_callback_uses_profile_float_text() {
        TRACE.with(|trace| trace.borrow_mut().clear());
        let vm = VirtualMachine::new().with_static_names(alloc::vec![String::from("value")])
            .with_trace_static(Some(String::from("*"))).with_trace_out(capture);
        let raw = (1.25 as Float).to_bits();
        vm.trace_static_access("Ld", 3, 0, raw, false);
        TRACE.with(|trace| {
            let trace = trace.borrow();
            assert_eq!(trace.len(), 1);
            let value = if runtime::IS_INT32 { "1.25" } else { "1.250000" };
            assert_eq!(trace[0], format!("[TRACE_STATIC] Ld slot=3 off=0 name=value val=int({}) / float({value})", raw as Int));
        });
    }
}

impl VirtualMachine {
    pub fn run(&mut self, chunk: &Chunk) -> Result<Value, String> {
        let module = Module { functions: vec![chunk.clone()], entry: 0, flags: if runtime::IS_INT32 { polka::CART_FLAG_INT32_SAFE } else { 0 }, exports: vec![] };
        self.run_module(&module)
    }

    pub fn run_module(&mut self, module: &Module) -> Result<Value, String> {
        let r = self.run_module_inner(module);
        r.map_err(|e| {
            let line = match module.functions.get(self.current_func) {
                Some(polka::Chunk::Bytecode(bc)) => bc.lines.get(self.failing_pc).copied().unwrap_or(0),
                _ => 0,
            };
            let at = if line > 0 { format!(" @{}", line) } else { String::new() };
            format!(
                "[{}:{}{}] {}",
                super::debug::render_fn_label(self.current_func, &self.fn_names),
                self.failing_pc, at, e,
            )
        })
    }

    pub fn call_export(
        &mut self,
        module: &Module,
        name: &str,
        args: &[Value],
    ) -> Result<Value, String> {
        validate_module_profile(module)?;
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
        validate_module_profile(module)?;
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
        self.enter_run_loop(module)
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
        self.resolved_aot.clear();
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

        self.enter_run_loop(module)
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
        self.enter_run_loop(module)?;
        Ok(())
    }

    fn enter_run_loop(&mut self, module: &Module) -> Result<Value, String> {
        match (self.debug_sink.is_some(), self.profile) {
            (false, false) => self.run_loop::<false, false>(module),
            (true,  false) => self.run_loop::<true,  false>(module),
            (false, true)  => self.run_loop::<false, true>(module),
            (true,  true)  => self.run_loop::<true,  true>(module),
        }
    }

    fn run_loop<const TRACE: bool, const PROF: bool>(&mut self, module: &Module) -> Result<Value, String> {
        'outer: loop {
            if self.yielded {
                return Ok(Value::from_int(0));
            }
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
                if let Some(remaining) = &mut self.step_remaining {
                    if *remaining == 0 {
                        self.failing_pc = opcode_pc;
                        return Err(String::from("step cap exceeded"));
                    }
                    *remaining -= 1;
                }
                if TRACE {
                    let pass = self.trace_filter.as_ref()
                        .map_or(true, |f| f.get(self.current_func).copied().unwrap_or(false));
                    // take() the sink so the event may borrow self.registers.
                    if pass { if let Some(mut sink) = self.debug_sink.take() {
                        let base = self.base_reg;
                        let rc = bc.reg_count.min(128);
                        let end = (base + rc).min(self.registers.len());
                        let mut handle_mask = runtime::empty_trace_mask();
                        for i in 0..end.saturating_sub(base) {
                            if self.reg_mask_bit(base + i) {
                                runtime::set_trace_mask(&mut handle_mask, i);
                            }
                        }
                        let event = DebugEvent::Trace {
                            func: self.current_func, pc: opcode_pc, op: opcode,
                            base_reg: base, window: &self.registers[base..end], handle_mask,
                            line: bc.lines.get(opcode_pc).copied().unwrap_or(0),
                            file: &bc.src_file,
                        };
                        sink(&event, &self.fn_names);
                        self.debug_sink = Some(sink);
                    } }
                }
                if PROF {
                    let name = Self::op_name(opcode);
                    let count = self.prof_ops.entry(name).or_insert(0);
                    *count = count.saturating_add(1);
                    let count = self.prof_fns.entry(self.current_func).or_insert(0);
                    *count = count.saturating_add(1);
                    let count = self.prof_fn_ops.entry(self.current_func).or_default().entry(name).or_insert(0);
                    *count = count.saturating_add(1);
                }
                self.pc = opcode_pc + 1;
                self.steps = self.steps.saturating_add(1);
                if self.heap_check {
                    self.heap.trace_pc = opcode_pc;
                }
                if let Err(e) = self.exec(module, bc, opcode) {
                    self.failing_pc = opcode_pc;
                    return Err(e);
                }
                if self.heap_check {
                    if let Err(e) = self.check_handle_tags(&format!("after {:?}", opcode)) {
                        self.failing_pc = opcode_pc;
                        return Err(e);
                    }
                }
                if self.halted || self.yielded || self.current_func != entry_func {
                    continue 'outer;
                }
            }
        }
    }

    pub fn run_to_yield(&mut self, module: &Module) -> Result<(), String> {
        validate_module_profile(module)?;
        self.yielded = false;
        validate_module_register_budget(module)?;
        self.int32_safe = (module.flags & polka::CART_FLAG_INT32_SAFE) != 0;
        if self.resolved_constants.is_empty() {
            self.frames.clear();
            self.handlers.clear();
            self.region_table.clear();
            self.heap.clear();
            self.string_const_handles.clear();
            self.resolve_constants(module)?;
            self.module_table_raw = HANDLE_NONE;
            self.module_table_is_handle = false;
            self.run_module_init(module)?;
        }
        self.pc = 0;
        self.base_reg = 0;
        self.current_func = module.entry as usize;
        self.halted = false;
        let needed = polka::FRAME_REGS + STAGE_SLACK;
        self.ensure_registers(needed);
        self.enter_run_loop(module)?;
        if !self.yielded {
            return Err("run_to_yield: main returned without calling __frame_present".into());
        }
        Ok(())
    }

    pub fn resume(&mut self, module: &Module, input: Value) -> Result<bool, String> {
        validate_module_profile(module)?;
        if !self.yielded {
            return Err("resume: VM is not suspended".into());
        }
        self.yielded = false;
        self.write_abs_raw(self.yield_dest_abs, input.raw());
        self.set_reg_mask_bit(self.yield_dest_abs, false);
        self.enter_run_loop(module)?;
        Ok(self.yielded)
    }

    #[inline(always)]
    fn exec(&mut self, module: &Module, bc: &BytecodeChunk, op: &OpCode) -> Result<(), String> {
        match op {
            OpCode::Add(..)|OpCode::Sub(..)|OpCode::Mul(..)|OpCode::Div(..)|OpCode::Mod(..)
            |OpCode::Neg(..)|OpCode::AddImm(..)|OpCode::SubImm(..)
            |OpCode::Lt(..)|OpCode::Gt(..)|OpCode::Lte(..)|OpCode::Gte(..)
            |OpCode::And(..)|OpCode::Or(..)|OpCode::Xor(..)|OpCode::Shl(..)|OpCode::Shr(..)
                => self.exec_integer_op(op),
            OpCode::FAdd(..)|OpCode::FSub(..)|OpCode::FMul(..)|OpCode::FDiv(..)
            |OpCode::FNeg(..)|OpCode::FLt(..)|OpCode::FEq(..) => self.exec_float_op(op),
            OpCode::Eq(d, a, b) => self.bin_eq(*d, *a, *b, false),
            OpCode::Neq(d, a, b) => self.bin_eq(*d, *a, *b, true),

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
                let fn_id = self.read_int(*fn_id_reg)?;
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

            OpCode::Ld(d, b, off)      => self.exec_ld(*d, *b, *off as Int),
            OpCode::St(src, b, off)    => self.exec_st(*src, *b, *off as Int),
            OpCode::LdIdx(d, b, i)     => { let off = self.read_int(*i)?; if off < 0 { return Err(format!("ldidx: negative index {}", off)); } self.exec_ld(*d, *b, off) }
            OpCode::StIdx(src, b, i)   => { let off = self.read_int(*i)?; if off < 0 { return Err(format!("stidx: negative index {}", off)); } self.exec_st(*src, *b, off) }


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
    fn exec_ld(&mut self, d: Register, b: Register, off: Int) -> Result<(), String> {
        let (slot, gen_) = self.read_handle(b)?;
        let (raw, is_handle) = self.heap.ld(slot, gen_, off as usize)?;
        if is_handle { self.rc_inc_handle(raw)?; }
        self.trace_static_access("Ld", slot, off, raw, is_handle);
        self.write(d, raw, is_handle)
    }

    #[inline(always)]
    fn exec_st(&mut self, src: Register, b: Register, off: Int) -> Result<(), String> {
        let (slot, gen_) = self.read_handle(b)?;
        let (raw, is_handle) = self.take(src)?;
        self.trace_static_access("St", slot, off, raw, is_handle);
        let (old_raw, old_is_handle) = self.heap.st(slot, gen_, off as usize, raw, is_handle)?;
        if old_is_handle { self.rc_dec_handle(old_raw)?; }
        Ok(())
    }

    fn trace_static_access(&self, op: &str, slot: u32, off: Int, raw: Word, is_handle: bool) {
        let traced = match self.trace_static_filter.as_deref() { Some(s) => s, None => return };
        let idx = off as usize;
        let name = self.static_names.get(idx).map(|s| s.as_str()).unwrap_or("");
        if name.is_empty() { return; }
        if traced == "*" || diagnostic_text_contains(name, traced) {
            let val = if is_handle {
                format!("handle({:#x})", raw)
            } else {
                format!("int({}) / float({})", raw as Int, runtime::format_diagnostic_float(Float::from_bits(raw), 6))
            };
            if let Some(f) = self.trace_out {
                f(&format!("[TRACE_STATIC] {} slot={} off={} name={} val={}", op, slot, off, name, val));
            }
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
