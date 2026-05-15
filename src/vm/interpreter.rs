use super::{VirtualMachine, Value};
use crate::bytecode::{Chunk, OpCode, Register, Module};
use crate::vm::frame::Frame;

// Max 2.5~3 MB
const MAX_REGISTERS: usize = 1 << 16;

impl VirtualMachine {
    pub fn run(&mut self, chunk: &Chunk) -> Result<Value, String> {
        self.pc = 0;
        if chunk.reg_count > self.registers.len() {
            self.registers.resize(chunk.reg_count, None);
        }
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
                OpCode::Call(_, _, _, _) => {
                    return Err("Call opcode not supported in single-chunk mode; use run_module()".to_string());
                }
            }
        }
        Ok(Value::Unit)
    }

    pub fn run_module(&mut self, module: &Module) -> Result<Value, String> {
        self.pc = 0;
        self.base_reg = 0;
        self.current_func = module.entry;
        self.frames.clear();
        let entry_regs = module.functions[module.entry].reg_count;
        if entry_regs > self.registers.len() {
            self.registers.resize(entry_regs, None);
        }

        loop {
            let current_chunk = &module.functions[self.current_func];

            if self.pc >= current_chunk.code.len() {
                if let Some(frame) = self.frames.pop() {
                    let return_val = self.registers[self.base_reg].clone()
                        .ok_or("Return register is empty")?;
                    self.pc = frame.ip;
                    self.base_reg = frame.base_reg;
                    self.current_func = frame.func_id;
                    self.registers[frame.dest_reg] = Some(return_val);
                    continue;
                } else {
                    return self.registers[self.base_reg].clone()
                        .ok_or("Return register is empty".to_string());
                }
            }

            let opcode = &current_chunk.code[self.pc].clone();
            self.pc += 1;

            match opcode {
                OpCode::PushConst(reg, const_idx) => {
                    if *const_idx >= current_chunk.constants.len() {
                        return Err("Constant index out of bounds".to_string());
                    }
                    self.registers[self.base_reg + reg.to_usize()] = Some(current_chunk.constants[*const_idx].clone());
                }
                OpCode::Mov(dest, src) => {
                    let val = self.registers[self.base_reg + src.to_usize()].clone()
                        .ok_or("Source register is empty")?;
                    self.registers[self.base_reg + dest.to_usize()] = Some(val);
                }
                OpCode::Add(dest, left, right) => {
                    self.binary_op_windowed(*dest, *left, *right, |l, r| match (l, r) {
                        (Value::Int(a), Value::Int(b)) => Value::Int(a + b),
                        _ => Value::Unit,
                    })?;
                }
                OpCode::Sub(dest, left, right) => {
                    self.binary_op_windowed(*dest, *left, *right, |l, r| match (l, r) {
                        (Value::Int(a), Value::Int(b)) => Value::Int(a - b),
                        _ => Value::Unit,
                    })?;
                }
                OpCode::Mul(dest, left, right) => {
                    self.binary_op_windowed(*dest, *left, *right, |l, r| match (l, r) {
                        (Value::Int(a), Value::Int(b)) => Value::Int(a * b),
                        _ => Value::Unit,
                    })?;
                }
                OpCode::Div(dest, left, right) => {
                    self.binary_op_windowed(*dest, *left, *right, |l, r| match (l, r) {
                        (Value::Int(a), Value::Int(b)) if b != 0 => Value::Int(a / b),
                        _ => Value::Unit,
                    })?;
                }
                OpCode::Mod(dest, left, right) => {
                    self.binary_op_windowed(*dest, *left, *right, |l, r| match (l, r) {
                        (Value::Int(a), Value::Int(b)) if b != 0 => Value::Int(a % b),
                        _ => Value::Unit,
                    })?;
                }
                OpCode::Eq(dest, left, right) => {
                    self.binary_op_windowed(*dest, *left, *right, |l, r| {
                        Value::Bool(l == r)
                    })?;
                }
                OpCode::Neq(dest, left, right) => {
                    self.binary_op_windowed(*dest, *left, *right, |l, r| {
                        Value::Bool(l != r)
                    })?;
                }
                OpCode::Lt(dest, left, right) => {
                    self.binary_op_windowed(*dest, *left, *right, |l, r| match (l, r) {
                        (Value::Int(a), Value::Int(b)) => Value::Bool(a < b),
                        (Value::Float(a), Value::Float(b)) => Value::Bool(a < b),
                        _ => Value::Bool(false),
                    })?;
                }
                OpCode::Gt(dest, left, right) => {
                    self.binary_op_windowed(*dest, *left, *right, |l, r| match (l, r) {
                        (Value::Int(a), Value::Int(b)) => Value::Bool(a > b),
                        (Value::Float(a), Value::Float(b)) => Value::Bool(a > b),
                        _ => Value::Bool(false),
                    })?;
                }
                OpCode::Lte(dest, left, right) => {
                    self.binary_op_windowed(*dest, *left, *right, |l, r| match (l, r) {
                        (Value::Int(a), Value::Int(b)) => Value::Bool(a <= b),
                        (Value::Float(a), Value::Float(b)) => Value::Bool(a <= b),
                        _ => Value::Bool(false),
                    })?;
                }
                OpCode::Gte(dest, left, right) => {
                    self.binary_op_windowed(*dest, *left, *right, |l, r| match (l, r) {
                        (Value::Int(a), Value::Int(b)) => Value::Bool(a >= b),
                        (Value::Float(a), Value::Float(b)) => Value::Bool(a >= b),
                        _ => Value::Bool(false),
                    })?;
                }
                OpCode::Jz(reg, target) => {
                    let cond = self.registers[self.base_reg + reg.to_usize()].clone()
                        .ok_or("Jump register is empty")?;
                    if is_falsy(&cond) {
                        self.pc = *target;
                    }
                }
                OpCode::Jnz(reg, target) => {
                    let cond = self.registers[self.base_reg + reg.to_usize()].clone()
                        .ok_or("Jump register is empty")?;
                    if !is_falsy(&cond) {
                        self.pc = *target;
                    }
                }
                OpCode::Jmp(target) => {
                    self.pc = *target;
                }
                OpCode::Call(dest, func_id, first_arg_reg, arg_count) => {
                    let dest_abs = self.base_reg + dest.to_usize();
                    let new_base = self.base_reg + current_chunk.reg_count;
                    let callee_reg_count = module.functions[*func_id].reg_count;
                    let needed = new_base + callee_reg_count;
                    if needed > MAX_REGISTERS {
                        return Err(format!(
                            "Stack overflow: register window {} exceeds limit {}",
                            needed, MAX_REGISTERS
                        ));
                    }
                    if needed > self.registers.len() {
                        self.registers.resize(needed, None);
                    }

                    self.frames.push(Frame {
                        func_id: self.current_func,
                        ip: self.pc,
                        base_reg: self.base_reg,
                        dest_reg: dest_abs,
                    });

                    for i in 0..*arg_count as usize {
                        let src_idx = self.base_reg + first_arg_reg.to_usize() + i;
                        if let Some(val) = self.registers[src_idx].clone() {
                            self.registers[new_base + i] = Some(val);
                        }
                    }

                    self.base_reg = new_base;
                    self.current_func = *func_id;
                    self.pc = 0;
                }
                OpCode::Ret(reg) => {
                    let return_val = self.registers[self.base_reg + reg.to_usize()].clone()
                        .ok_or("Return register is empty")?;

                    if let Some(frame) = self.frames.pop() {
                        self.pc = frame.ip;
                        self.base_reg = frame.base_reg;
                        self.current_func = frame.func_id;
                        self.registers[frame.dest_reg] = Some(return_val);
                    } else {
                        return Ok(return_val);
                    }
                }
            }
        }
    }

    fn binary_op_windowed<F>(&mut self, dest: Register, left: Register, right: Register, op: F) -> Result<(), String>
    where
        F: Fn(Value, Value) -> Value,
    {
        let lv = self.registers[self.base_reg + left.to_usize()].clone().ok_or("Left operand register is empty")?;
        let rv = self.registers[self.base_reg + right.to_usize()].clone().ok_or("Right operand register is empty")?;
        self.registers[self.base_reg + dest.to_usize()] = Some(op(lv, rv));
        Ok(())
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
