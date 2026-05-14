pub mod value;
pub mod frame;
pub mod memory;
pub mod interpreter;
pub mod loader;
pub mod scheduler;

pub use value::Value;

use frame::Frame;

pub struct VirtualMachine {
    pub(crate) registers: [Option<Value>; 256],
    #[allow(dead_code)]
    pub(crate) frames: Vec<Frame>,
    pc: usize,
}

impl VirtualMachine {
    pub fn new() -> Self {
        Self {
            registers: std::array::from_fn(|_| None),
            frames: Vec::new(),
            pc: 0,
        }
    }
}
