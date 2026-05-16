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
    // First-class callable: lifted-fn id + handle to env heap object.
    Closure { func_id: usize, env_slot: u32, env_gen: u32 },
    Reference(Box<Value>),
    Handle { slot: u32, generation: u32 },
}
