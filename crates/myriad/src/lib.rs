pub mod value;
pub mod frame;
pub mod memory;
pub mod devices;
pub mod interpreter;
pub mod loader;
pub mod region;
pub mod builtins;
pub mod debug;
pub mod host;
pub mod snapshot;

pub use polka::{Value, HANDLE_NONE};
pub use value::{alloc_string, read_string};
pub use devices::{Device, DeviceTable};
pub use memory::Heap;
pub use region::RegionTable;
pub use builtins::{NativeCtx, NativeFn, NativeRegistry};
pub use debug::{DebugEvent, DebugSink};
pub use host::Host;

use frame::Frame;

pub fn run(module: polka::Module, host: Host) -> Result<i64, String> {
    let loaded = loader::load(module)?;
    let mut vm = VirtualMachine::new();
    host.install_into(&mut vm);
    let v = vm.run_module(&loaded.module)?;
    Ok(v.as_int())
}

pub struct VirtualMachine {
    pub(crate) registers: Vec<u64>,
    // Bit i (LSB of word i/64) = 1 iff registers[i] is a handle.
    pub(crate) register_mask: Vec<u64>,
    pub(crate) frames: Vec<Frame>,
    pub(crate) pc: usize,
    pub(crate) base_reg: usize,
    pub(crate) current_func: usize,
    pub(crate) heap: Heap,
    pub(crate) handlers: Vec<HandlerFrame>,
    pub(crate) halted: bool,
    pub(crate) exit_code: Option<i64>,
    pub(crate) dispatch_last_result: Option<u16>,
    pub(crate) dispatch_last_env: Option<(u64, bool)>,
    pub(crate) devices: DeviceTable,
    // Constants resolved per-fn at module load. Value bits + parallel mask.
    pub(crate) resolved_constants: Vec<Vec<u64>>,
    pub(crate) resolved_const_mask: Vec<Vec<u64>>,
    // Permanent heap handles for string constants; rc=1 module-lifetime.
    pub(crate) string_const_handles: Vec<(u32, u32)>,
    pub(crate) resolved_natives: Vec<Option<NativeFn>>,
    pub(crate) region_table: RegionTable,
    pub(crate) natives: NativeRegistry,
    pub(crate) debug_sink: Option<DebugSink>,
    pub(crate) trace_frames: bool,
    pub(crate) fn_names: Vec<String>,
    pub(crate) failing_pc: usize,
    pub(crate) last_result_is_handle: bool,
    pub(crate) int32_safe: bool,
    pub(crate) module_table_raw: u64,
    pub(crate) module_table_is_handle: bool,
    pub(crate) steps: u64,
    pub(crate) step_cap: Option<u64>,
    pub(crate) static_names: Vec<String>,
    pub(crate) trace_static_filter: Option<String>,
    // ABRASE_HEAP_CHECK=1: after every op, assert each handle-tagged register and
    // cell slot points to a live cell (or HANDLE_NONE). Catches dangling tags
    // (UAF precursor) directly instead of waiting for a downstream crash.
    pub(crate) heap_check: bool,
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
    pub pending_return_arm_env: u64,
    pub pending_return_arm_env_is_handle: bool,
}

impl HandlerFrame {
    pub fn release_cells(
        &self,
        heap: &mut crate::memory::Heap,
        regions: &mut crate::region::RegionTable,
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
    pub const INIT_MASK_WORD0: u64 =
        (1u64 << DISPATCH_ENV) | (1u64 << REGS_SNAPSHOT_SLOT);
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
            region_table: RegionTable::new(),
            natives,
            debug_sink: None,
            trace_frames: false,
            fn_names: Vec::new(),
            failing_pc: 0,
            last_result_is_handle: false,
            int32_safe: false,
            module_table_raw: polka::HANDLE_NONE,
            module_table_is_handle: false,
            steps: 0,
            step_cap: None,
            static_names: Vec::new(),
            trace_static_filter: std::env::var("TRACE_STATIC").ok()
                .filter(|s| !s.is_empty()),
            heap_check: std::env::var("ABRASE_HEAP_CHECK").is_ok(),
        }
    }

    pub fn with_static_names(mut self, names: Vec<String>) -> Self {
        self.static_names = names;
        self
    }

    pub fn with_step_cap(mut self, cap: u64) -> Self {
        self.step_cap = Some(cap);
        self
    }

    // N of instructions executed. Monotonic; a profiler reads the per-frame delta.
    pub fn steps(&self) -> u64 { self.steps }

    pub fn halted(&self) -> bool { self.exit_code.is_some() }

    pub fn exit_code(&self) -> Option<i64> { self.exit_code }

    pub fn with_debug(mut self, on: bool) -> Self {
        self.debug_sink = if on { Some(debug::stderr_sink()) } else { None };
        self
    }

    pub fn with_trace_frames(mut self, on: bool) -> Self {
        self.trace_frames = on;
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

    #[inline]
    pub(crate) fn trace_frame_event(&self, kind: &str, detail: std::fmt::Arguments<'_>) {
        if !self.trace_frames { return; }
        let bfi = self.handlers.last().and_then(|h| h.body_frame_index);
        eprintln!("[{}] {} | frames={} handlers={} bfi={:?}",
            kind, detail, self.frames.len(), self.handlers.len(), bfi);
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

    // User-visible live count. Excludes module-lifetime cells owned by the
    // loader/runtime (not by user code): string constants and the module table.
    pub fn heap_live_count(&self) -> usize {
        let total = self.heap.live_count();
        let const_live = self.string_const_handles.iter()
            .filter(|(s, g)| self.heap.is_live(*s, *g))
            .count();
        let module_live = if self.module_table_is_handle && self.module_table_raw != polka::HANDLE_NONE {
            let (s, g) = crate::memory::handle_parts(self.module_table_raw);
            if self.heap.is_live(s, g) { 1 } else { 0 }
        } else { 0 };
        total.saturating_sub(const_live).saturating_sub(module_live)
    }


    // Debug: print every live heap cell
    pub fn dump_live_slots(&self) {
        let owned: std::collections::HashSet<(u32, u32)> = {
            let mut s: std::collections::HashSet<(u32, u32)> =
                self.string_const_handles.iter().copied().collect();
            if self.module_table_is_handle && self.module_table_raw != polka::HANDLE_NONE {
                s.insert(crate::memory::handle_parts(self.module_table_raw));
            }
            s
        };
        let cells = self.heap.live_cells();
        eprintln!("[heap] {} live cell(s), {} user:", cells.len(), self.heap_live_count());
        for (slot, gen_, rc, data, handles) in &cells {
            let tag = if owned.contains(&(*slot, *gen_)) { "rt  " } else { "USER" };
            let slots: Vec<String> = data.iter().zip(handles.iter()).map(|(v, h)| {
                if *h { format!("h:{:#x}", v) } else { format!("{}", *v as i64) }
            }).collect();
            let note = self.closure_cell_label(data, handles);
            eprintln!("  [{}] slot={} gen={} rc={} [{}]{}", tag, slot, gen_, rc, slots.join(", "), note);
        }
    }

    fn closure_cell_label(&self, data: &[u64], handles: &[bool]) -> String {
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

    pub fn take_device(&mut self, id: u8) -> Option<Box<dyn Device>> {
        self.devices.take(id)
    }

    pub fn heap_alloc(&mut self, size: usize) -> (u32, u32) {
        self.heap.alloc(size)
    }

    pub fn heap_st(
        &mut self, slot: u32, gen_: u32, offset: usize, val: u64, is_handle: bool,
    ) -> Result<(u64, bool), String> {
        self.heap.st(slot, gen_, offset, val, is_handle)
    }

    pub fn push_handler(&mut self, h: HandlerFrame) {
        self.handlers.push(h);
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
