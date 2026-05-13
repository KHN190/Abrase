use crate::bytecode;

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    String(String),
    Unit,
}

pub struct Frame {
    chunk_index: usize,
    ip: usize,
    base_slot: usize,
}

pub struct VirtualMachine {
    stack: Vec<Value>,
    frames: Vec<Frame>,
    heap: Vec<Value>, 
}

impl VirtualMachine {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            frames: Vec::new(),
            heap: Vec::new(),
        }
    }

    pub fn run(&mut self, _chunk: &bytecode::Chunk) -> Result<Value, String> {
        Ok(Value::Unit)
    }
}