pub mod value;
pub mod frame;
pub mod memory;
pub mod device;
pub mod devices;
pub mod interpreter;
pub mod loader;

pub use value::Value;
pub use device::{Device, DeviceTable};

use frame::Frame;
use memory::Heap;

pub struct VirtualMachine {
    pub(crate) registers: Vec<Option<Value>>,
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
}

pub const DISPATCH_ID: u8 = 0xE0;
pub const DISPATCH_PORT_LOOKUP: u8 = 0x00;
pub const DISPATCH_NO_MATCH: u16 = 0xFFFF;

// `cell_*` point at the 4-slot heap continuation: [pc, base, dest_reg, alive].
// `dispatch_table_*` point at a heap array of arm fn_ids (one per op), used by
// Dispatch device (0xE0) lookup. `handler_fn` is the fallback for single-op
// handlers built by the current codegen path.
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
        }
    }

    pub fn heap_live_count(&self) -> usize {
        self.heap.live_count()
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
