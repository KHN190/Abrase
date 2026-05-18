pub mod value;
pub mod frame;
pub mod memory;
pub mod device;
pub mod devices;
pub mod interpreter;
pub mod loader;
pub mod region;
pub mod builtins;
pub mod debug;
pub mod host;
pub mod snapshot;

pub use polka::Value;
pub use value::{BoxPool, BoxedValue};
pub use device::{Device, DeviceTable};
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
    Ok(vm.box_pool().read_int(v).unwrap_or(0))
}

pub struct VirtualMachine {
    pub(crate) registers: Vec<Value>,
    pub(crate) frames: Vec<Frame>,
    pub(crate) pc: usize,
    pub(crate) base_reg: usize,
    pub(crate) current_func: usize,
    pub(crate) heap: Heap,
    pub(crate) handlers: Vec<HandlerFrame>,
    pub(crate) halted: bool,
    pub(crate) exit_code: Option<i64>,
    pub(crate) dispatch_last_result: Option<u16>,
    pub(crate) dispatch_last_env: Option<Value>,
    pub(crate) devices: DeviceTable,
    pub(crate) box_pool: BoxPool,
    pub(crate) resolved_constants: Vec<Vec<Value>>,
    pub(crate) resolved_natives: Vec<Option<NativeFn>>,
    pub(crate) region_table: RegionTable,
    pub(crate) natives: NativeRegistry,
    pub(crate) debug_sink: Option<DebugSink>,
    pub(crate) fn_names: Vec<String>,
    // The step loop increments self.pc before exec(), 
    // so a runtime error reporting uses this field instead.
    pub(crate) failing_pc: usize,
}

pub struct HandlerFrame {
    pub effect_id: u16,
    pub dispatch_table_slot: Option<u32>,
    pub dispatch_table_gen: u32,
    pub cell_slot: u32,
    pub cell_gen: u32,
    pub cells_allocated: Vec<(u32, u32)>,
    // Index in `self.frames` of the frame pushed by the handle body's call
    // (e.g. `Call(range)`). Set when do_call fires for the first time after
    // this handler is installed. Used to identify the "boundary" frame whose
    // pop signals the handle body has returned without intermediate Resume.
    pub body_frame_index: Option<usize>,
    // Return-arm function id + env, captured at handle install via the
    // INSTALL_RETURN_ARM dispatch ports. Cleared the first time it's
    // applied (either when the body frame pops without resume, or when the
    // topmost arm-continuation pops via Resume's deep-handler path).
    pub pending_return_arm_fn: Option<usize>,
    pub pending_return_arm_env: Value,
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
}

impl VirtualMachine {
    pub fn new() -> Self {
        let mut natives = NativeRegistry::new();
        builtins::register_default_builtins(&mut natives);
        Self {
            registers: Vec::new(),
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
            box_pool: BoxPool::new(),
            resolved_constants: Vec::new(),
            resolved_natives: Vec::new(),
            region_table: RegionTable::new(),
            natives,
            debug_sink: None,
            fn_names: Vec::new(),
            failing_pc: 0,
        }
    }

    pub fn with_debug(mut self, on: bool) -> Self {
        self.debug_sink = if on { Some(debug::stderr_sink()) } else { None };
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

    pub fn region_push(&mut self) {
        self.region_table.push();
    }

    pub fn region_pop(&mut self) -> Result<(), String> {
        self.region_table.pop_and_release(&mut self.heap, &mut self.box_pool)
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

    pub fn heap_live_count(&self) -> usize {
        self.heap.live_count()
    }

    pub fn box_pool(&self) -> &BoxPool {
        &self.box_pool
    }

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

    pub fn heap_st(&mut self, slot: u32, gen_: u32, offset: usize, val: Value) -> Result<Value, String> {
        self.heap.st(slot, gen_, offset, val)
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
        v.heap.rc_inc(slot, gen_).unwrap(); // rc = 2
        v.heap.rc_inc(slot, gen_).unwrap(); // rc = 3
        v.region_pop().expect("pop ok");
        assert_eq!(v.heap_live_count(), 0, "force_free ignores rc");
    }

    #[test]
    fn region_cascade_frees_handles_inside_cell() {
        // A region-allocated cell that stores another handle to a non-region
        // cell — force_free should cascade rc_dec to the child.
        let mut v = vm();
        let (child_slot, child_gen) = v.heap_alloc(1); // outside region, rc=1
        v.region_push();
        let (parent_slot, parent_gen) = v.heap_alloc(1);
        v.region_record_alloc(parent_slot, parent_gen);
        v.heap.rc_inc(child_slot, child_gen).unwrap(); // child rc=2
        v.heap_st(parent_slot, parent_gen, 0,
                  Value::from_handle(child_slot, child_gen)).unwrap();
        // live = parent + child
        assert_eq!(v.heap_live_count(), 2);
        v.region_pop().expect("pop ok");
        // parent force-freed → cascade rc_dec on child (rc 2→1). Child still live.
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
        v.region_push();        // outer
        v.region_push();        // inner
        let (slot, gen_) = v.heap_alloc(1);
        v.region_record_alloc(slot, gen_);

        v.region_pop().expect("inner pop");
        // Was recorded to inner only — heap is free now.
        assert_eq!(v.heap_live_count(), 0);
        v.region_pop().expect("outer pop");
    }

    #[test]
    fn record_alloc_outside_region_is_noop() {
        let mut v = vm();
        // No region pushed.
        let (slot, gen_) = v.heap_alloc(1);
        v.region_record_alloc(slot, gen_); // silently dropped
        assert_eq!(v.heap_live_count(), 1);
        // No region to pop.
        assert!(v.region_pop().is_err());
    }
}
