use alloc::{boxed::Box, string::{String, ToString}, vec::Vec};
#[path = "frame.rs"]
pub mod frame;
#[path = "devices/mod.rs"]
pub mod devices;
#[path = "interpreter/mod.rs"]
pub mod interpreter;
#[path = "loader.rs"]
pub mod loader;
#[path = "builtins.rs"]
pub mod builtins;
#[path = "debug.rs"]
pub mod debug;
#[path = "host.rs"]
pub mod host;
#[path = "snapshot.rs"]
pub mod snapshot;
#[path = "aot.rs"]
pub mod aot;
pub use devices::{Device, DeviceTable};
pub use builtins::{NativeCtx, NativeFn, NativeRegistry};
pub use debug::{render_fn_label, DebugEvent, DebugSink};
pub use host::Host;
pub use aot::{AotHost, AotNatives, reachable_live_count};
pub use polka::cartridge::read_pk;

use frame::Frame;

pub type AotFn = alloc::rc::Rc<dyn for<'a> Fn(&mut NativeCtx<'a>, &[Value], &[bool]) -> Result<(Value, bool), String>>;

pub fn run(module: polka::Module, host: Host) -> Result<Int, String> {
    let loaded = loader::load(module)?;
    let mut vm = VirtualMachine::new();
    host.install_into(&mut vm);
    let v = vm.run_module(&loaded.module)?;
    Ok(v.as_int())
}

pub struct VirtualMachine {
    pub(crate) registers: Vec<Word>,
    // Bit i (LSB of word i/Word::BITS) = 1 iff registers[i] is a handle.
    pub(crate) register_mask: Vec<Word>,
    pub(crate) frames: Vec<Frame>,
    pub(crate) pc: usize,
    pub(crate) base_reg: usize,
    pub(crate) current_func: usize,
    pub(crate) heap: Heap,
    pub(crate) handlers: Vec<HandlerFrame>,
    pub(crate) halted: bool,
    pub(crate) exit_code: Option<Int>,
    pub(crate) dispatch_last_result: Option<u16>,
    pub(crate) dispatch_last_env: Option<(Word, bool)>,
    pub(crate) devices: DeviceTable,
    // Constants resolved per-fn at module load. Value bits + parallel mask.
    pub(crate) resolved_constants: Vec<Vec<Word>>,
    pub(crate) resolved_const_mask: Vec<Vec<Word>>,
    // Permanent heap handles for string constants; rc=1 module-lifetime.
    pub(crate) string_const_handles: Vec<(u32, u32)>,
    pub(crate) resolved_natives: Vec<Option<NativeFn>>,
    pub(crate) aot_fns: alloc::collections::BTreeMap<alloc::string::String, AotFn>,
    pub(crate) resolved_aot: Vec<Option<AotFn>>,
    pub(crate) region_table: RegionTable,
    pub(crate) core_arena: CoreArena,
    pub(crate) natives: NativeRegistry,
    pub(crate) debug_sink: Option<DebugSink>,
    pub(crate) trace_filter: Option<Vec<bool>>,
    pub(crate) trace_frames: bool,
    pub(crate) fn_names: Vec<String>,
    pub(crate) failing_pc: usize,
    pub(crate) last_result_is_handle: bool,
    pub(crate) int32_safe: bool,
    pub(crate) module_table_raw: Word,
    pub(crate) module_table_is_handle: bool,
    pub(crate) steps: Word,
    pub(crate) step_remaining: Option<Word>,
    pub(crate) static_names: Vec<String>,
    pub(crate) trace_static_filter: Option<String>,
    pub(crate) heap_check: bool,
    pub(crate) profile: bool,
    pub(crate) prof_ops: ProfileMap<&'static str, Word>,
    pub(crate) prof_fns: ProfileMap<usize, Word>,
    pub(crate) prof_fn_ops: ProfileMap<usize, ProfileMap<&'static str, Word>>,
    pub(crate) yielded: bool,
    pub(crate) yield_dest_abs: usize,
    // Text-diagnostic sink for frame/static traces. None = silent (no_std default).
    pub(crate) trace_out: Option<fn(&str)>,
}

pub struct HandlerFrame {
    pub effect_id: u16,
    pub dispatch_table_slot: Option<u32>,
    pub dispatch_table_gen: u32,
    pub cell_slot: u32,
    pub cell_gen: u32,
    pub cells_allocated: Vec<(u32, u32)>,
    pub body_frame_index: Option<usize>,
    pub pending_return_arm_fn: Option<usize>,
    pub pending_return_arm_env: Word,
    pub pending_return_arm_env_is_handle: bool,
}

impl HandlerFrame {
    pub fn release_cells(
        &self,
        heap: &mut memory::Heap,
        regions: &mut region::RegionTable,
    ) -> Result<(), String> {
        for (slot, generation) in &self.cells_allocated {
            regions.forget(*slot, *generation);
            if heap.is_live(*slot, *generation) {
                heap.rc_dec(*slot, *generation)?;
            }
        }
        Ok(())
    }
}

pub mod cont_slot {
    use super::Word;
    pub const SUSPEND_PC: usize = 0;
    pub const SUSPEND_BASE: usize = 1;
    pub const DEST_REG: usize = 2;
    pub const ALIVE: usize = 3;
    pub const SUSPEND_FUNC: usize = 4;
    pub const DISPATCH_FN_ID: usize = 5;
    pub const DISPATCH_ENV: usize = 6;
    pub const REGS_SNAPSHOT_SLOT: usize = 7;
    pub const REGS_COUNT: usize = 8;
    pub const SIZE: usize = 9;

    // Bits set = the slot holds a handle when written by do_yield.
    pub const INIT_MASK_WORD0: Word =
        ((1 as Word) << DISPATCH_ENV) | ((1 as Word) << REGS_SNAPSHOT_SLOT);
}

impl VirtualMachine {
    pub fn new() -> Self {
        let mut natives = NativeRegistry::new();
        builtins::register_default_builtins(&mut natives);
        Self {
            registers: Vec::new(),
            register_mask: Vec::new(),
            frames: Vec::new(),
            pc: 0,
            base_reg: 0,
            current_func: 0,
            heap: Heap::new(),
            handlers: Vec::new(),
            halted: false,
            exit_code: None,
            dispatch_last_result: None,
            dispatch_last_env: None,
            devices: DeviceTable::new(),
            resolved_constants: Vec::new(),
            resolved_const_mask: Vec::new(),
            string_const_handles: Vec::new(),
            resolved_natives: Vec::new(),
            aot_fns: alloc::collections::BTreeMap::new(),
            resolved_aot: Vec::new(),
            region_table: RegionTable::new(),
            core_arena: CoreArena::new(),
            natives,
            debug_sink: None,
            trace_filter: None,
            trace_frames: false,
            fn_names: Vec::new(),
            failing_pc: 0,
            last_result_is_handle: false,
            int32_safe: false,
            module_table_raw: HANDLE_NONE,
            module_table_is_handle: false,
            steps: 0,
            step_remaining: None,
            static_names: Vec::new(),
            trace_static_filter: None,
            heap_check: false,
            profile: false,
            trace_out: None,
            prof_ops: ProfileMap::new(),
            prof_fns: ProfileMap::new(),
            prof_fn_ops: ProfileMap::new(),
            yielded: false,
            yield_dest_abs: 0,
        }
    }

    pub fn with_static_names(mut self, names: Vec<String>) -> Self {
        self.static_names = names;
        self
    }

    pub fn with_step_cap(mut self, cap: Word) -> Self {
        self.step_remaining = Some(cap.saturating_sub(self.steps));
        self
    }

    pub fn with_heap_check(mut self, on: bool) -> Self {
        self.heap_check = on;
        self
    }

    pub fn with_core_arena(mut self, bytes: usize) -> Self {
        self.core_arena = CoreArena::with_size(bytes);
        self
    }

    pub fn core_arena_ref(&self) -> &CoreArena { &self.core_arena }
    pub fn core_arena_mut(&mut self) -> &mut CoreArena { &mut self.core_arena }

    // Saturating instruction count; execution budgets are tracked independently.
    pub fn steps(&self) -> Word { self.steps }

    pub fn halted(&self) -> bool { self.exit_code.is_some() }

    pub fn exit_code(&self) -> Option<Int> { self.exit_code }

    // Configure heap RC tracing (TRACE_SLOT). slot=None+all=true traces every
    // cell; out is the text sink the host provides (e.g. an eprintln wrapper).
    pub fn with_heap_trace(mut self, slot: Option<u32>, all: bool, out: fn(&str)) -> Self {
        self.heap.set_trace(slot, all, out);
        self
    }

    // Text sink for frame/static diagnostics (TRACE_STATIC, trace_frames).
    pub fn with_trace_out(mut self, out: fn(&str)) -> Self {
        self.trace_out = Some(out);
        self
    }

    pub fn with_profile(mut self, on: bool) -> Self {
        self.profile = on;
        self
    }

    pub fn with_trace_static(mut self, filter: Option<String>) -> Self {
        self.trace_static_filter = filter;
        self
    }

    pub fn with_trace_frames(mut self, on: bool) -> Self {
        self.trace_frames = on;
        self
    }

    // fn_id-indexed bitset; out-of-range fn ids are silenced. None = trace all.
    pub fn with_trace_filter(mut self, bits: Vec<bool>) -> Self {
        self.trace_filter = Some(bits);
        self
    }

    pub fn with_debug_sink(mut self, sink: DebugSink) -> Self {
        self.debug_sink = Some(sink);
        self
    }

    pub fn with_fn_names(mut self, names: Vec<String>) -> Self {
        self.fn_names = names;
        self
    }

    pub(crate) fn emit_debug(&mut self, event: &DebugEvent) {
        if let Some(sink) = &mut self.debug_sink {
            sink(event, &self.fn_names);
        }
    }

    // Opcode + per-fn execution histogram (PROFILE=1). Op counts are
    // pacing-independent: work per run, not wall time. Returns "" when profiling
    // is off; the host prints the report. Kept here so the VM owns the format.
    pub fn profile_report(&self) -> String {
        use core::fmt::Write;
        let mut s = String::new();
        if !self.profile { return s; }
        let mut ops: Vec<_> = self.prof_ops.iter().collect();
        ops.sort_unstable_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
        let total: Word = self.prof_ops.values().fold(0, |sum, n| sum.saturating_add(*n));
        let _ = writeln!(s, "[profile] {} ops executed", total);
        for (name, n) in ops {
            let _ = writeln!(s, "  {:>12} {:>6}%  {}", n, format_diagnostic_float(*n as Float * 100.0 / total.max(1) as Float, 1), name);
        }
        let mut fns: Vec<_> = self.prof_fns.iter().collect();
        fns.sort_unstable_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
        let _ = writeln!(s, "[profile] per-fn opcode breakdown (top 15 fns):");
        for (fid, n) in fns.into_iter().take(15) {
            let _ = writeln!(s, "  {:>12} {} ({}%)", n,
                debug::render_fn_label(*fid, &self.fn_names),
                format_diagnostic_float(*n as Float * 100.0 / total.max(1) as Float, 1));
            if let Some(ops) = self.prof_fn_ops.get(fid) {
                let mut fo: Vec<_> = ops.iter().collect();
                fo.sort_unstable_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
                let line: Vec<String> = fo.into_iter().take(8)
                    .map(|(name, c)| format!("{} {}%", name, format_diagnostic_float(*c as Float * 100.0 / (*n).max(1) as Float, 0)))
                    .collect();
                let _ = writeln!(s, "               {}", line.join("  "));
            }
        }
        s
    }

    pub(crate) fn op_name(op: &polka::OpCode) -> &'static str {
        use polka::OpCode::*;
        match op {
            Add(..) => "Add", Sub(..) => "Sub", Mul(..) => "Mul", Div(..) => "Div", Mod(..) => "Mod",
            Neg(..) => "Neg", FAdd(..) => "FAdd", FSub(..) => "FSub", FMul(..) => "FMul", FDiv(..) => "FDiv",
            FNeg(..) => "FNeg", FLt(..) => "FLt", FEq(..) => "FEq",
            Eq(..) => "Eq", Neq(..) => "Neq", Lt(..) => "Lt", Gt(..) => "Gt", Lte(..) => "Lte", Gte(..) => "Gte",
            And(..) => "And", Or(..) => "Or", Xor(..) => "Xor", Shl(..) => "Shl", Shr(..) => "Shr",
            Jmp(..) => "Jmp", Jz(..) => "Jz", Jnz(..) => "Jnz", Call(..) => "Call", CallReg(..) => "CallReg",
            Ret(..) => "Ret", PushConst(..) => "PushConst", Copy(..) => "Copy", Move(..) => "Move",
            Ld(..) => "Ld", St(..) => "St", LdIdx(..) => "LdIdx", StIdx(..) => "StIdx",
            AddImm(..) => "AddImm", SubImm(..) => "SubImm", Alloc(..) => "Alloc", Drop(..) => "Drop",
            Dei(..) => "Dei", Deo(..) => "Deo", Handle(..) => "Handle", Resume(..) => "Resume", Raise(..) => "Raise",
        }
    }

    #[inline]
    pub(crate) fn trace_frame_event(&self, kind: &str, detail: core::fmt::Arguments<'_>) {
        if !self.trace_frames { return; }
        if let Some(f) = self.trace_out {
            let bfi = self.handlers.last().and_then(|h| h.body_frame_index);
            f(&format!("[{}] {} | frames={} handlers={} bfi={:?}",
                kind, detail, self.frames.len(), self.handlers.len(), bfi));
        }
    }

    pub fn region_push(&mut self) {
        self.region_table.push();
    }

    pub fn region_pop(&mut self) -> Result<(), String> {
        self.region_table.pop_and_release(&mut self.heap)
    }

    pub fn region_depth(&self) -> usize {
        self.region_table.depth()
    }

    #[inline]
    pub fn region_record_alloc(&mut self, slot: u32, generation: u32) {
        if self.region_table.is_active() {
            self.region_table.record_alloc(slot, generation);
        }
    }

    // Refcount of the module static-table root. heap_live_count() hides table
    // leaks (the cell is module-reachable, so always subtracted); this exposes
    // the raw rc so tests can assert per-call Dei/Drop balance on it.
    pub fn module_table_rc(&self) -> Option<u32> {
        if self.module_table_is_handle && self.module_table_raw != HANDLE_NONE {
            let (s, g) = memory::handle_parts(self.module_table_raw);
            self.heap.rc(s, g)
        } else {
            None
        }
    }

    // User-visible live count. Excludes module-lifetime cells owned by the
    // loader/runtime (not by user code): string constants and the module table.
    pub fn heap_live_count(&self) -> usize {
        let total = self.heap.live_count();
        let const_live = self.string_const_handles.iter()
            .filter(|(s, g)| self.heap.is_live(*s, *g))
            .count();
        let mut rt_owned: RuntimeSet<(u32, u32)> = RuntimeSet::new();
        if self.module_table_is_handle && self.module_table_raw != HANDLE_NONE {
            let root = memory::handle_parts(self.module_table_raw);
            self.collect_reachable(root.0, root.1, &mut rt_owned);
        }
        let module_live = rt_owned.iter().filter(|(s, g)| self.heap.is_live(*s, *g)).count();
        total.saturating_sub(const_live).saturating_sub(module_live)
    }

    pub fn heap_alloc_count(&self) -> Word { self.heap.alloc_count() }

    pub fn heap_bytes_used(&self) -> usize { self.heap.bytes_used() }

    fn collect_reachable(&self, slot: u32, generation: u32, visited: &mut RuntimeSet<(u32, u32)>) {
        if !visited.insert((slot, generation)) { return; }
        if !self.heap.is_live(slot, generation) { return; }
        let Ok(data) = self.heap.cell_data(slot, generation) else { return; };
        let Ok(mask) = self.heap.cell_mask(slot, generation) else { return; };
        let n = data.len();
        let data: Vec<Word> = data.to_vec();
        for i in 0..n {
            if memory::mask_bit(mask, i) {
                let child_raw = data[i];
                if child_raw != HANDLE_NONE {
                    let (cs, cg) = memory::handle_parts(child_raw);
                    self.collect_reachable(cs, cg, visited);
                }
            }
        }
    }


    // Debug: render every live heap cell. Host prints the returned report.
    pub fn live_slots_report(&self) -> String {
        use core::fmt::Write;
        let owned: RuntimeSet<(u32, u32)> = {
            let mut s: RuntimeSet<(u32, u32)> =
                self.string_const_handles.iter().copied().collect();
            if self.module_table_is_handle && self.module_table_raw != HANDLE_NONE {
                s.insert(memory::handle_parts(self.module_table_raw));
            }
            s
        };
        let cells = self.heap.live_cells();
        let mut out = String::new();
        let _ = writeln!(out, "[heap] {} live cell(s), {} user:", cells.len(), self.heap_live_count());
        for (slot, gen_, rc, data, handles) in &cells {
            let tag = if owned.contains(&(*slot, *gen_)) { "rt  " } else { "USER" };
            let slots: Vec<String> = data.iter().zip(handles.iter()).map(|(v, h)| {
                if *h { format!("h:{:#x}", v) } else { format!("{}", *v as Int) }
            }).collect();
            let note = self.closure_cell_label(data, handles);
            let _ = writeln!(out, "  [{}] slot={} gen={} rc={} [{}]{}", tag, slot, gen_, rc, slots.join(", "), note);
        }
        out
    }

    fn closure_cell_label(&self, data: &[Word], handles: &[bool]) -> String {
        if data.len() != 2 || handles.first() != Some(&false) { return String::new(); }
        let fid = data[0] as usize;
        match self.fn_names.get(fid) {
            Some(n) if n.starts_with("__closure_") || n.starts_with("__fnval_") =>
                format!("  ; closure({})", n),
            _ => String::new(),
        }
    }

    pub fn heap_ref(&self) -> &Heap { &self.heap }
    pub fn heap_mut(&mut self) -> &mut Heap { &mut self.heap }

    pub fn last_result_is_handle(&self) -> bool { self.last_result_is_handle }

    pub fn install_device(&mut self, id: u8, dev: Box<dyn Device>) {
        self.devices.install(id, dev);
    }

    pub fn register_native<S: Into<String>>(&mut self, name: S, func: NativeFn) {
        self.natives.register(name, func);
    }

    pub fn register_aot_fn<S: Into<String>>(&mut self, name: S, func: AotFn) {
        self.aot_fns.insert(name.into(), func);
    }

    pub fn take_device(&mut self, id: u8) -> Option<Box<dyn Device>> {
        self.devices.take(id)
    }

    pub fn heap_alloc(&mut self, size: usize) -> (u32, u32) {
        self.heap.alloc(size)
    }

    pub fn heap_st(
        &mut self, slot: u32, gen_: u32, offset: usize, val: Word, is_handle: bool,
    ) -> Result<(Word, bool), String> {
        self.heap.st(slot, gen_, offset, val, is_handle)
    }

    pub fn push_handler(&mut self, h: HandlerFrame) {
        let effect_id = h.effect_id;
        let dispatch_table = h.dispatch_table_slot.map(|slot| (slot, h.dispatch_table_gen));
        self.handlers.push(h);
        if self.debug_sink.is_some() {
            self.emit_debug(&DebugEvent::HandlePush {
                effect_id,
                dispatch_table,
                depth: self.handlers.len(),
            });
        }
    }
}

#[cfg(test)]
mod region_tests {
    use super::*;

    fn vm() -> VirtualMachine { VirtualMachine::new() }

    #[test]
    fn region_force_frees_even_with_rc_greater_than_one() {
        let mut v = vm();
        v.region_push();
        let (slot, gen_) = v.heap_alloc(1);
        v.region_record_alloc(slot, gen_);
        v.heap.rc_inc(slot, gen_).unwrap();
        v.heap.rc_inc(slot, gen_).unwrap();
        v.region_pop().expect("pop ok");
        assert_eq!(v.heap_live_count(), 0, "force_free ignores rc");
    }

    #[test]
    fn region_cascade_frees_handles_inside_cell() {
        let mut v = vm();
        let (child_slot, child_gen) = v.heap_alloc(1);
        v.region_push();
        let (parent_slot, parent_gen) = v.heap_alloc(1);
        v.region_record_alloc(parent_slot, parent_gen);
        v.heap.rc_inc(child_slot, child_gen).unwrap();
        let child_handle = Value::from_handle(child_slot, child_gen).raw();
        v.heap_st(parent_slot, parent_gen, 0, child_handle, true).unwrap();
        assert_eq!(v.heap_live_count(), 2);
        v.region_pop().expect("pop ok");
        assert_eq!(v.heap_live_count(), 1, "child survives at rc=1; parent freed");
    }

    #[test]
    fn region_pop_force_frees_recorded_alloc() {
        let mut v = vm();
        v.region_push();
        let (slot, gen_) = v.heap_alloc(4);
        v.region_record_alloc(slot, gen_);
        assert_eq!(v.heap_live_count(), 1);
        v.region_pop().expect("pop ok");
        assert_eq!(v.heap_live_count(), 0, "alloc recorded in region must be force-freed");
    }

    #[test]
    fn nested_region_pop_frees_only_inner() {
        let mut v = vm();
        v.region_push();
        let (outer_slot, outer_gen) = v.heap_alloc(1);
        v.region_record_alloc(outer_slot, outer_gen);

        v.region_push();
        let (inner_slot, inner_gen) = v.heap_alloc(1);
        v.region_record_alloc(inner_slot, inner_gen);
        assert_eq!(v.heap_live_count(), 2);

        v.region_pop().expect("inner pop");
        assert_eq!(v.region_depth(), 1, "outer region still active");
        assert_eq!(v.heap_live_count(), 1, "outer alloc survives inner pop");

        v.region_pop().expect("outer pop");
        assert_eq!(v.region_depth(), 0);
        assert_eq!(v.heap_live_count(), 0);
    }

    #[test]
    fn region_records_only_to_topmost_region() {
        let mut v = vm();
        v.region_push();
        v.region_push();
        let (slot, gen_) = v.heap_alloc(1);
        v.region_record_alloc(slot, gen_);

        v.region_pop().expect("inner pop");
        assert_eq!(v.heap_live_count(), 0);
        v.region_pop().expect("outer pop");
    }

    #[test]
    fn record_alloc_outside_region_is_noop() {
        let mut v = vm();
        let (slot, gen_) = v.heap_alloc(1);
        v.region_record_alloc(slot, gen_);
        assert_eq!(v.heap_live_count(), 1);
        assert!(v.region_pop().is_err());
    }
}

impl VirtualMachine {
    // Structural heap dump for debuggers/hosts. No runtime types: cells render
    // as [v0, v1, …], scalars as decimal, HANDLE_NONE as "none". Read-only;
    // stale handles render as an error string (gen check), never panic.
    pub fn render_value(&self, raw: Word, is_handle: bool, depth: usize) -> String {
        if !is_handle { return (raw as Int).to_string(); }
        if raw == HANDLE_NONE { return "none".into(); }
        if depth == 0 { return "…".into(); }
        let (slot, g) = Self::decode_handle(raw);
        let len = match self.heap.size(slot, g) {
            Ok(n) => n,
            Err(_) => return format!("<stale {:#x}>", raw),
        };
        let mut out = String::from("[");
        for off in 0..len {
            if off > 0 { out.push_str(", "); }
            match self.heap.ld(slot, g, off) {
                Ok((v, h)) => out.push_str(&self.render_value(v, h, depth - 1)),
                Err(_) => { out.push_str("<err>"); }
            }
        }
        out.push(']');
        out
    }

    // Test helper: release one rc on a handle (e.g. a value returned by main).
    pub fn drop_result_for_test(&mut self, raw: Word) {
        let _ = self.heap.rc_dec_handle(raw);
    }
}

#[cfg(test)]
mod vm_api_tests {
    use super::*;
    use polka::{Module, Chunk, BytecodeChunk, OpCode, Register};

    fn const_module(val: Word) -> Module {
        let val = {
            let mut bytes = [0; 8];
            let native = val.to_le_bytes();
            bytes[..native.len()].copy_from_slice(&native);
            polka::Constant::from_le_bytes(bytes)
        };
        Module {
            functions: vec![Chunk::Bytecode(BytecodeChunk {
                code: vec![OpCode::PushConst(Register(0), 0), OpCode::Ret(Register(0))],
                constants: vec![val],
                const_mask: Vec::new(),
                string_constants: Vec::new(),
                bytes_constants: Vec::new(),
                reg_count: 1,
                param_count: 0,
                lines: Vec::new(),
                src_file: String::new(),
            })],
            entry: 0,
            flags: if IS_INT32 { polka::CART_FLAG_INT32_SAFE } else { 0 },
            exports: Vec::new(),
        }
    }

    #[test]
    fn run_returns_entry_value_as_int() {
        assert_eq!(run(const_module(42), Host::default()).unwrap(), 42);
    }

    #[test]
    fn profile_and_step_counters_populate_after_run() {
        let mut vm = VirtualMachine::new().with_profile(true);
        let loaded = loader::load(const_module(7)).unwrap();
        let v = vm.run_module(&loaded.module).unwrap();
        assert_eq!(v.as_int(), 7);
        assert!(vm.steps() > 0);
        assert!(!vm.profile_report().is_empty());
    }

    #[test]
    fn region_depth_tracks_push_pop() {
        let mut vm = VirtualMachine::new();
        assert_eq!(vm.region_depth(), 0);
        vm.region_push();
        assert_eq!(vm.region_depth(), 1);
        vm.region_pop().unwrap();
        assert_eq!(vm.region_depth(), 0);
    }

    #[test]
    fn fresh_vm_not_halted_no_exit_code() {
        let vm = VirtualMachine::new();
        assert!(!vm.halted());
        assert_eq!(vm.exit_code(), None);
        assert_eq!(vm.module_table_rc(), None);
    }

    #[test]
    fn render_value_formats_int_and_opaque_handle() {
        let vm = VirtualMachine::new();
        assert_eq!(vm.render_value(42, false, 0), "42");
        assert_eq!(vm.render_value(HANDLE_NONE, true, 4), "none");
        assert_eq!(vm.render_value(0, true, 0), "…");
    }

    #[test]
    fn take_device_absent_is_none() {
        let mut vm = VirtualMachine::new();
        assert!(vm.take_device(0x7e).is_none());
    }

    #[test]
    fn profiling_counts_and_report_total_saturate_at_word_max() {
        let mut vm = VirtualMachine::new().with_profile(true);
        vm.prof_ops.insert("Ret", Word::MAX);
        vm.prof_fns.insert(0, Word::MAX);
        vm.prof_fn_ops.entry(0).or_default().insert("Ret", Word::MAX);
        vm.run_module(&const_module(7)).unwrap();
        assert_eq!(vm.prof_ops["Ret"], Word::MAX);
        assert_eq!(vm.prof_fns[&0], Word::MAX);
        assert_eq!(vm.prof_fn_ops[&0]["Ret"], Word::MAX);
        assert!(vm.profile_report().contains(&format!("{} ops executed", Word::MAX)));
    }

    #[test]
    fn diagnostic_float_text_uses_the_profile_formatter() {
        assert_eq!(format_diagnostic_float(1.25, 6), if IS_INT32 { "1.25" } else { "1.250000" });
        let mut vm = VirtualMachine::new().with_profile(true);
        vm.prof_ops.insert("Ret", 1);
        vm.prof_fns.insert(0, 1);
        vm.prof_fn_ops.entry(0).or_default().insert("Ret", 1);
        let report = vm.profile_report();
        assert!(report.contains(if IS_INT32 { "(100%)" } else { "(100.0%)" }), "{report}");
        assert!(report.contains("Ret 100%"), "{report}");
    }

    #[test]
    fn profiling_ties_use_name_and_function_id_order() {
        let mut vm = VirtualMachine::new().with_profile(true);
        vm.prof_ops.insert("Zulu", 1);
        vm.prof_ops.insert("Alpha", 1);
        for id in (0..20).rev() {
            vm.prof_fns.insert(id, 1);
            vm.prof_fn_ops.entry(id).or_default().insert("Zulu", 1);
            vm.prof_fn_ops.entry(id).or_default().insert("Alpha", 1);
        }
        let report = vm.profile_report();
        assert!(report.find("Alpha\n").unwrap() < report.find("Zulu\n").unwrap());
        let mut previous = 0;
        for id in 0..15 {
            let index = report.find(&format!("#{id} (")).unwrap();
            assert!(index > previous);
            previous = index;
        }
        assert!(!report.contains("#15 ("));
        assert!(report.contains("Alpha 100%  Zulu 100%"));
    }

    #[test]
    fn unlimited_execution_continues_when_statistics_saturate() {
        let mut vm = VirtualMachine::new();
        vm.steps = Word::MAX - 1;
        assert_eq!(vm.run_module(&const_module(7)).unwrap().as_int(), 7);
        assert_eq!(vm.steps(), Word::MAX);
        assert_eq!(vm.run_module(&const_module(8)).unwrap().as_int(), 8);
        assert_eq!(vm.steps(), Word::MAX);
    }

    #[test]
    fn explicit_maximum_budget_stops_at_the_limit() {
        let mut vm = VirtualMachine::new();
        vm.steps = Word::MAX - 1;
        let mut vm = vm.with_step_cap(Word::MAX);
        assert!(vm.run_module(&const_module(7)).unwrap_err().contains("step cap exceeded"));
        assert_eq!(vm.steps(), Word::MAX);
        assert!(vm.run_module(&const_module(7)).unwrap_err().contains("step cap exceeded"));
    }

    #[test]
    fn explicit_zero_budget_has_no_execution_observations() {
        let traces = alloc::rc::Rc::new(core::cell::Cell::new(0));
        let captured = traces.clone();
        let mut vm = VirtualMachine::new().with_step_cap(0).with_profile(true)
            .with_debug_sink(Box::new(move |event, _| {
                if matches!(event, DebugEvent::Trace { .. }) { captured.set(captured.get() + 1); }
            }));
        assert!(vm.run_module(&const_module(7)).unwrap_err().contains("step cap exceeded"));
        assert_eq!(vm.steps(), 0);
        assert_eq!(vm.pc, 0);
        assert_eq!(traces.get(), 0);
        assert!(vm.prof_ops.is_empty());
        assert!(vm.prof_fns.is_empty());
        assert!(vm.prof_fn_ops.is_empty());
    }

    #[test]
    fn finite_budget_observes_exactly_the_attempted_instructions() {
        let traces = alloc::rc::Rc::new(core::cell::Cell::new(0));
        let captured = traces.clone();
        let mut vm = VirtualMachine::new().with_step_cap(3).with_profile(true)
            .with_debug_sink(Box::new(move |event, _| {
                if matches!(event, DebugEvent::Trace { .. }) { captured.set(captured.get() + 1); }
            }));
        let mut m = const_module(7);
        if let polka::Chunk::Bytecode(bc) = &mut m.functions[0] { bc.code = vec![polka::OpCode::Jmp(-1)]; }
        assert!(vm.run_module(&m).unwrap_err().contains("step cap exceeded"));
        assert_eq!(vm.steps(), 3);
        assert_eq!(traces.get(), 3);
        assert_eq!(vm.prof_ops["Jmp"], 3);
        assert_eq!(vm.prof_fns[&0], 3);
    }

    #[test]
    fn explicit_budget_is_cumulative_across_calls_and_reset() {
        let mut vm = VirtualMachine::new().with_step_cap(4);
        assert_eq!(vm.run_module(&const_module(7)).unwrap().as_int(), 7);
        vm.reset();
        assert_eq!(vm.run_module(&const_module(8)).unwrap().as_int(), 8);
        assert_eq!(vm.steps(), 4);
        assert!(vm.run_module(&const_module(9)).unwrap_err().contains("step cap exceeded"));
        assert_eq!(vm.steps(), 4);
    }

    #[test]
    fn setting_budget_after_execution_keeps_the_cumulative_contract() {
        let mut vm = VirtualMachine::new();
        vm.run_module(&const_module(7)).unwrap();
        let mut vm = vm.with_step_cap(3);
        assert!(vm.run_module(&const_module(8)).unwrap_err().contains("step cap exceeded"));
        assert_eq!(vm.steps(), 3);
    }

}
