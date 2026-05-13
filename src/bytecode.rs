use crate::vm::Value;

pub enum OpCode {
    PushConst(usize),
    Add,
    Jump(usize),
    Call(usize),
}
pub struct Chunk {
    pub code: Vec<OpCode>,
    pub constants: Vec<Value>,
}