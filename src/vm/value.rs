// Sized so clone is a small memcpy: all heap payloads are Box'd.
#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Char(char),
    Unit,
    Handle { slot: u32, generation: u32 },
    Closure { func_id: usize, env_slot: u32, env_gen: u32 },
    String(Box<String>),
    Tuple(Box<Vec<Value>>),
    Array(Box<Vec<Value>>),
    Record { tag: u32, fields: Box<Vec<Value>> },
    Reference(Box<Value>),
}
