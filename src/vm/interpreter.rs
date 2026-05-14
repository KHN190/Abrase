use super::{VirtualMachine, Value};
use crate::bytecode::{Chunk, OpCode, Register};

impl VirtualMachine {
    pub fn run(&mut self, chunk: &Chunk) -> Result<Value, String> {
        self.pc = 0;
        while self.pc < chunk.code.len() {
            let opcode = &chunk.code[self.pc];
            self.pc += 1;
            match opcode {
                OpCode::PushConst(reg, const_idx) => {
                    if *const_idx >= chunk.constants.len() {
                        return Err("Constant index out of bounds".to_string());
                    }
                    self.registers[reg.to_usize()] = Some(chunk.constants[*const_idx].clone());
                }
                OpCode::Mov(dest, src) => {
                    let val = self.registers[src.to_usize()].clone()
                        .ok_or("Source register is empty")?;
                    self.registers[dest.to_usize()] = Some(val);
                }
                OpCode::Add(dest, left, right) => {
                    self.binary_op(*dest, *left, *right, |l, r| match (l, r) {
                        (Value::Int(a), Value::Int(b)) => Value::Int(a + b),
                        _ => Value::Unit,
                    })?;
                }
                OpCode::Sub(dest, left, right) => {
                    self.binary_op(*dest, *left, *right, |l, r| match (l, r) {
                        (Value::Int(a), Value::Int(b)) => Value::Int(a - b),
                        _ => Value::Unit,
                    })?;
                }
                OpCode::Mul(dest, left, right) => {
                    self.binary_op(*dest, *left, *right, |l, r| match (l, r) {
                        (Value::Int(a), Value::Int(b)) => Value::Int(a * b),
                        _ => Value::Unit,
                    })?;
                }
                OpCode::Div(dest, left, right) => {
                    self.binary_op(*dest, *left, *right, |l, r| match (l, r) {
                        (Value::Int(a), Value::Int(b)) if b != 0 => Value::Int(a / b),
                        _ => Value::Unit,
                    })?;
                }
                OpCode::Mod(dest, left, right) => {
                    self.binary_op(*dest, *left, *right, |l, r| match (l, r) {
                        (Value::Int(a), Value::Int(b)) if b != 0 => Value::Int(a % b),
                        _ => Value::Unit,
                    })?;
                }
                OpCode::Eq(dest, left, right) => {
                    self.binary_op(*dest, *left, *right, |l, r| {
                        Value::Bool(l == r)
                    })?;
                }
                OpCode::Neq(dest, left, right) => {
                    self.binary_op(*dest, *left, *right, |l, r| {
                        Value::Bool(l != r)
                    })?;
                }
                OpCode::Lt(dest, left, right) => {
                    self.binary_op(*dest, *left, *right, |l, r| match (l, r) {
                        (Value::Int(a), Value::Int(b)) => Value::Bool(a < b),
                        (Value::Float(a), Value::Float(b)) => Value::Bool(a < b),
                        _ => Value::Bool(false),
                    })?;
                }
                OpCode::Gt(dest, left, right) => {
                    self.binary_op(*dest, *left, *right, |l, r| match (l, r) {
                        (Value::Int(a), Value::Int(b)) => Value::Bool(a > b),
                        (Value::Float(a), Value::Float(b)) => Value::Bool(a > b),
                        _ => Value::Bool(false),
                    })?;
                }
                OpCode::Lte(dest, left, right) => {
                    self.binary_op(*dest, *left, *right, |l, r| match (l, r) {
                        (Value::Int(a), Value::Int(b)) => Value::Bool(a <= b),
                        (Value::Float(a), Value::Float(b)) => Value::Bool(a <= b),
                        _ => Value::Bool(false),
                    })?;
                }
                OpCode::Gte(dest, left, right) => {
                    self.binary_op(*dest, *left, *right, |l, r| match (l, r) {
                        (Value::Int(a), Value::Int(b)) => Value::Bool(a >= b),
                        (Value::Float(a), Value::Float(b)) => Value::Bool(a >= b),
                        _ => Value::Bool(false),
                    })?;
                }
                OpCode::Jz(reg, target) => {
                    let cond = self.registers[reg.to_usize()].clone()
                        .ok_or("Jump register is empty")?;
                    if is_falsy(&cond) {
                        self.pc = *target;
                    }
                }
                OpCode::Jnz(reg, target) => {
                    let cond = self.registers[reg.to_usize()].clone()
                        .ok_or("Jump register is empty")?;
                    if !is_falsy(&cond) {
                        self.pc = *target;
                    }
                }
                OpCode::Jmp(target) => {
                    self.pc = *target;
                }
                OpCode::Ret(reg) => {
                    return self.registers[reg.to_usize()].clone()
                        .ok_or_else(|| "Return register is empty".to_string());
                }
            }
        }
        Ok(Value::Unit)
    }

    fn binary_op<F>(&mut self, dest: Register, left: Register, right: Register, op: F) -> Result<(), String>
    where
        F: Fn(Value, Value) -> Value,
    {
        let lv = self.registers[left.to_usize()].clone().ok_or("Left operand register is empty")?;
        let rv = self.registers[right.to_usize()].clone().ok_or("Right operand register is empty")?;
        self.registers[dest.to_usize()] = Some(op(lv, rv));
        Ok(())
    }
}

fn is_falsy(val: &Value) -> bool {
    match val {
        Value::Bool(b) => !b,
        Value::Int(i) => *i == 0,
        Value::Unit => true,
        _ => false,
    }
}
