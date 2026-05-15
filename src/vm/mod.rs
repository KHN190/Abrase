pub mod value;
pub mod frame;
pub mod memory;
pub mod interpreter;
pub mod loader;
pub mod scheduler;

pub use value::Value;

use frame::Frame;

pub struct VirtualMachine {
    pub(crate) registers: Vec<Option<Value>>,
    pub(crate) frames: Vec<Frame>,
    pub(crate) pc: usize,
    pub(crate) base_reg: usize,
    pub(crate) current_func: usize,
}

impl VirtualMachine {
    pub fn new() -> Self {
        Self {
            registers: Vec::new(),
            frames: Vec::new(),
            pc: 0,
            base_reg: 0,
            current_func: 0,
        }
    }
}
