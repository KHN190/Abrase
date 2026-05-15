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
    // Stack of installed handler frames (parallel to value frames).
    // Each entry holds the function id of the handler so a `Resume` can
    // unwind back to it. Empty when no handler is active.
    pub(crate) handlers: Vec<HandlerFrame>,
}

pub struct HandlerFrame {
    pub handler_fn: usize,
    pub saved_pc: usize,
    pub saved_base: usize,
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
}
