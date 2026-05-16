pub mod value;
pub mod frame;
pub mod memory;
pub mod interpreter;
pub mod loader;

pub use value::Value;

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
}

// `cell_*` point at the 4-slot heap continuation: [pc, base, dest_reg, alive].
pub struct HandlerFrame {
    pub handler_fn: usize,
    pub cell_slot: u32,
    pub cell_gen: u32,
}

pub(crate) mod cont_slot {
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
        }
    }

    pub fn heap_live_count(&self) -> usize {
        self.heap.live_count()
    }
}
