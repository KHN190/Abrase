pub mod memory;
pub mod interpreter;
pub mod loader;
pub mod scheduler;

pub use memory::value::Value;

use memory::frame::Frame;

pub struct VirtualMachine {
    pub(crate) registers: [Option<Value>; 256],
    #[allow(dead_code)]
    pub(crate) frames: Vec<Frame>,
}

impl VirtualMachine {
    pub fn new() -> Self {
        Self {
            registers: std::array::from_fn(|_| None),
            frames: Vec::new(),
        }
    }
}
