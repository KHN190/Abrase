use polka::{BytecodeChunk, OpCode, Register};
use std::fmt::Write;

#[derive(Debug)]
pub enum TranspileError {
    Unsupported(String),
}

impl std::fmt::Display for TranspileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TranspileError::Unsupported(s) => write!(f, "unsupported: {}", s),
        }
    }
}

impl std::error::Error for TranspileError {}

fn reg(r: Register) -> String {
    format!("r{}", r.0)
}

// Branch target: the interpreter advances pc to opcode_pc+1 *before* exec, so a
// relative offset is measured from there. Target = (i + 1) + off.
fn target(i: usize, off: i16) -> isize {
    i as isize + 1 + off as isize
}

fn bin_i64(d: Register, a: Register, b: Register, op: &str) -> String {
    format!("{} = (({} as i64).{}({} as i64)) as u64;", reg(d), reg(a), op, reg(b))
}

fn cmp_i64(d: Register, a: Register, b: Register, op: &str) -> String {
    format!("{} = if ({} as i64) {} ({} as i64) {{ 1 }} else {{ 0 }};", reg(d), reg(a), op, reg(b))
}

fn checked(d: Register, a: Register, b: Register, method: &str, msg: &str) -> String {
    format!(
        "{} = match ({} as i64).{}({} as i64) {{ Some(v) => v as u64, None => return Err(\"{}\") }};",
        reg(d), reg(a), method, reg(b), msg
    )
}

fn jump_to(i: usize, off: i16, len: usize) -> String {
    let t = target(i, off);
    if t < 0 || t > len as isize {
        "return Err(\"branch out of range\")".to_string()
    } else {
        format!("pc = {}", t)
    }
}

fn op_stmt(i: usize, op: &OpCode, len: usize) -> Result<String, TranspileError> {
    let next = (i + 1) as isize;
    let s = match op {
        OpCode::Add(d, a, b) => format!("{} pc = {};", bin_i64(*d, *a, *b, "wrapping_add"), next),
        OpCode::Sub(d, a, b) => format!("{} pc = {};", bin_i64(*d, *a, *b, "wrapping_sub"), next),
        OpCode::Mul(d, a, b) => format!("{} pc = {};", bin_i64(*d, *a, *b, "wrapping_mul"), next),
        OpCode::Div(d, a, b) => format!("{} pc = {};", checked(*d, *a, *b, "checked_div", "div by zero"), next),
        OpCode::Mod(d, a, b) => format!("{} pc = {};", checked(*d, *a, *b, "checked_rem", "mod by zero"), next),
        OpCode::Neg(d, a) => format!("{} = (({} as i64).wrapping_neg()) as u64; pc = {};", reg(*d), reg(*a), next),
        OpCode::AddImm(d, a, imm) =>
            format!("{} = (({} as i64).wrapping_add({})) as u64; pc = {};", reg(*d), reg(*a), *imm as i64, next),
        OpCode::SubImm(d, a, imm) =>
            format!("{} = (({} as i64).wrapping_sub({})) as u64; pc = {};", reg(*d), reg(*a), *imm as i64, next),

        OpCode::Eq(d, a, b)  => format!("{} pc = {};", cmp_i64(*d, *a, *b, "=="), next),
        OpCode::Neq(d, a, b) => format!("{} pc = {};", cmp_i64(*d, *a, *b, "!="), next),
        OpCode::Lt(d, a, b)  => format!("{} pc = {};", cmp_i64(*d, *a, *b, "<"), next),
        OpCode::Gt(d, a, b)  => format!("{} pc = {};", cmp_i64(*d, *a, *b, ">"), next),
        OpCode::Lte(d, a, b) => format!("{} pc = {};", cmp_i64(*d, *a, *b, "<="), next),
        OpCode::Gte(d, a, b) => format!("{} pc = {};", cmp_i64(*d, *a, *b, ">="), next),

        OpCode::And(d, a, b) => format!("{} = {} & {}; pc = {};", reg(*d), reg(*a), reg(*b), next),
        OpCode::Or(d, a, b)  => format!("{} = {} | {}; pc = {};", reg(*d), reg(*a), reg(*b), next),
        OpCode::Xor(d, a, b) => format!("{} = {} ^ {}; pc = {};", reg(*d), reg(*a), reg(*b), next),
        // Interpreter shifts on i64 (Shr is arithmetic), shift amount (y as u32) & 63.
        OpCode::Shl(d, a, b) =>
            format!("{} = (({} as i64).wrapping_shl(({} as u32) & 63)) as u64; pc = {};", reg(*d), reg(*a), reg(*b), next),
        OpCode::Shr(d, a, b) =>
            format!("{} = (({} as i64).wrapping_shr(({} as u32) & 63)) as u64; pc = {};", reg(*d), reg(*a), reg(*b), next),

        OpCode::PushConst(d, idx) => format!("{} = c{}; pc = {};", reg(*d), idx, next),
        OpCode::Copy(d, a) => format!("{} = {}; pc = {};", reg(*d), reg(*a), next),
        OpCode::Move(d, a) => format!("{} = {}; pc = {};", reg(*d), reg(*a), next),

        OpCode::Jmp(off) => format!("{};", jump_to(i, *off, len)),
        OpCode::Jz(r, off) =>
            format!("if {} == 0 {{ {} }} else {{ pc = {} }};", reg(*r), jump_to(i, *off, len), next),
        OpCode::Jnz(r, off) =>
            format!("if {} != 0 {{ {} }} else {{ pc = {} }};", reg(*r), jump_to(i, *off, len), next),

        OpCode::Ret(a) => format!("return Ok({});", reg(*a)),

        other => return Err(TranspileError::Unsupported(format!("{:?}", other))),
    };
    Ok(s)
}

/// Integer-only bytecode function -> a pc-driven state machine mirroring the
/// interpreter's run loop. Returns `Result<u64, &str>` for checked-op errors.
pub fn transpile_function(chunk: &BytecodeChunk) -> Result<String, TranspileError> {
    let mut out = String::new();
    for n in 0..chunk.reg_count {
        let _ = writeln!(out, "    let mut r{}: u64 = 0;", n);
    }
    for (off, c) in chunk.constants.iter().enumerate() {
        let _ = writeln!(out, "    let c{}: u64 = {}u64;", off, c);
    }
    let _ = writeln!(out, "    let mut pc: isize = 0;");
    let _ = writeln!(out, "    loop {{");
    let _ = writeln!(out, "        match pc {{");
    let len = chunk.code.len();
    for (i, op) in chunk.code.iter().enumerate() {
        let _ = writeln!(out, "            {} => {{ {} }}", i, op_stmt(i, op, len)?);
    }
    // pc == len (one past end): interpreter halts, yields base register r0.
    let _ = writeln!(out, "            _ => return Ok(r0),");
    let _ = writeln!(out, "        }}");
    let _ = writeln!(out, "    }}");
    Ok(out)
}

/// Standalone program printing `OK <n>` or `ERR <msg>` for the differential harness.
pub fn transpile_program(chunk: &BytecodeChunk) -> Result<String, TranspileError> {
    let body = transpile_function(chunk)?;
    let mut out = String::new();
    let _ = writeln!(out, "#![allow(unused_mut, unused_variables, dead_code, unused_assignments)]");
    let _ = writeln!(out, "fn entry() -> Result<u64, &'static str> {{");
    out.push_str(&body);
    let _ = writeln!(out, "}}");
    let _ = writeln!(out, "fn main() {{");
    let _ = writeln!(out, "    match entry() {{");
    let _ = writeln!(out, "        Ok(v) => println!(\"OK {{}}\", v),");
    let _ = writeln!(out, "        Err(e) => println!(\"ERR {{}}\", e),");
    let _ = writeln!(out, "    }}");
    let _ = writeln!(out, "}}");
    Ok(out)
}
