#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Char(char),
    String(String),
    Unit,
    Tuple(Vec<Value>),
    Array(Vec<Value>),
    Record { tag: u32, fields: Vec<Value> },
    Closure { func_id: usize, env: Vec<Value> },
    Reference(Box<Value>),
    Handle { slot: u32, generation: u32 },
}
