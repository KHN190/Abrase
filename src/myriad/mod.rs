pub mod value;
pub mod frame;
pub mod memory;
pub mod device;
pub mod devices;
pub mod interpreter;
pub mod loader;
pub mod host;
pub mod region;

pub use value::{Value, BoxPool, BoxedValue};
pub use device::{Device, DeviceTable};
pub use memory::Heap;
pub use region::RegionTable;

use frame::Frame;

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
    pub(crate) devices: DeviceTable,
    pub(crate) box_pool: BoxPool,
    pub(crate) resolved_constants: Vec<Vec<Value>>,
    pub(crate) region_table: RegionTable,
}

pub const DISPATCH_ID: u8 = 0xE0;
pub const DISPATCH_PORT_LOOKUP: u8 = 0x00;
pub const DISPATCH_NO_MATCH: u16 = 0xFFFF;

pub const REGION_ID: u8 = 0xE1;
pub const REGION_PORT_PUSH: u8 = 0x00;
pub const REGION_PORT_POP: u8 = 0x01;

// Heap continuation slots [pc, base, dest_reg, alive]; dispatch table for arm fns; fallback handler_fn.
pub struct HandlerFrame {
    pub effect_id: u16,
    pub handler_fn: usize,
    pub dispatch_table_slot: Option<u32>,
    pub dispatch_table_gen: u32,
    pub cell_slot: u32,
    pub cell_gen: u32,
}

pub mod cont_slot {
    pub const SUSPEND_PC: usize = 0;
    pub const SUSPEND_BASE: usize = 1;
    pub const DEST_REG: usize = 2;
    pub const ALIVE: usize = 3;
    pub const SIZE: usize = 4;
}

impl VirtualMachine {
    pub fn new() -> Self {
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
            devices: DeviceTable::new(),
            box_pool: BoxPool::new(),
            resolved_constants: Vec::new(),
            region_table: RegionTable::new(),
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

    pub(crate) fn region_record_alloc(&mut self, slot: u32, generation: u32) {
        self.region_table.record_alloc(slot, generation);
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
    fn region_depth_starts_at_zero() {
        assert_eq!(vm().region_depth(), 0);
    }

    #[test]
    fn region_push_increments_depth() {
        let mut v = vm();
        v.region_push();
        assert_eq!(v.region_depth(), 1);
        v.region_push();
        assert_eq!(v.region_depth(), 2);
    }

    #[test]
    fn region_pop_without_push_errors() {
        let mut v = vm();
        assert!(v.region_pop().is_err());
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
    fn region_pop_ignores_alloc_not_recorded() {
        let mut v = vm();
        v.region_push();
        let _ = v.heap_alloc(2); // recorded by VM only when going through OpCode::Alloc
        v.region_pop().expect("pop ok");
        // direct heap_alloc isn't recorded — alloc survives.
        assert_eq!(v.heap_live_count(), 1);
    }

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
}
