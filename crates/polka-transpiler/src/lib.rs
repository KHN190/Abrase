use polka::{BytecodeChunk, Chunk, Module, OpCode, Register};
use std::fmt::Write;

const STAGE_SLACK: usize = 32;

struct Ctx<'a> {
    reg_count: usize,
    param_counts: &'a [usize],
    const_is_handle: &'a [bool],
}

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

fn reg(r: Register) -> String { format!("r{}", r.0) }
fn regh(r: Register) -> String { format!("r{}_h", r.0) }

fn target(i: usize, off: i16) -> isize { i as isize + 1 + off as isize }

fn jump_to(i: usize, off: i16, len: usize) -> String {
    let t = target(i, off);
    if t < 0 || t > len as isize {
        "return Err(\"branch out of range\".to_string())".to_string()
    } else {
        format!("pc = {}", t)
    }
}

// scalar (non-handle) dest: set value, clear handle bit, advance.
fn scalar(d: Register, expr: String, next: isize) -> String {
    format!("{} = {}; {} = false; pc = {};", reg(d), expr, regh(d), next)
}

fn bin(a: Register, b: Register, op: &str) -> String {
    format!("({} as i64).{}({} as i64) as u64", reg(a), op, reg(b))
}
fn cmp(a: Register, b: Register, op: &str) -> String {
    format!("if ({} as i64) {} ({} as i64) {{ 1 }} else {{ 0 }}", reg(a), op, reg(b))
}
fn checked(d: Register, a: Register, b: Register, method: &str, msg: &str, next: isize) -> String {
    format!(
        "{} = match ({} as i64).{}({} as i64) {{ Some(v) => v as u64, None => return Err(\"{}\".to_string()) }}; {} = false; pc = {};",
        reg(d), reg(a), method, reg(b), msg, regh(d), next,
    )
}

fn call_stmt(dest: Register, fn_id: usize, next: isize, ctx: &Ctx) -> Result<String, TranspileError> {
    let k = *ctx.param_counts.get(fn_id)
        .ok_or_else(|| TranspileError::Unsupported(format!("call to unknown fn {}", fn_id)))?;
    let vals: Vec<String> = (0..k).map(|j| reg(Register((ctx.reg_count + j) as u8))).collect();
    let hs: Vec<String> = (0..k).map(|j| regh(Register((ctx.reg_count + j) as u8))).collect();
    Ok(format!(
        "let (__rv, __rh) = f{}(h, &[{}], &[{}])?; {} = __rv; {} = __rh; pc = {};",
        fn_id, vals.join(", "), hs.join(", "), reg(dest), regh(dest), next,
    ))
}

fn op_stmt(i: usize, op: &OpCode, len: usize, ctx: &Ctx) -> Result<String, TranspileError> {
    let next = (i + 1) as isize;
    let s = match op {
        OpCode::Add(d, a, b) => scalar(*d, bin(*a, *b, "wrapping_add"), next),
        OpCode::Sub(d, a, b) => scalar(*d, bin(*a, *b, "wrapping_sub"), next),
        OpCode::Mul(d, a, b) => scalar(*d, bin(*a, *b, "wrapping_mul"), next),
        OpCode::Div(d, a, b) => checked(*d, *a, *b, "checked_div", "div by zero", next),
        OpCode::Mod(d, a, b) => checked(*d, *a, *b, "checked_rem", "mod by zero", next),
        OpCode::Neg(d, a) => scalar(*d, format!("({} as i64).wrapping_neg() as u64", reg(*a)), next),
        OpCode::AddImm(d, a, imm) => scalar(*d, format!("({} as i64).wrapping_add({}) as u64", reg(*a), *imm as i64), next),
        OpCode::SubImm(d, a, imm) => scalar(*d, format!("({} as i64).wrapping_sub({}) as u64", reg(*a), *imm as i64), next),

        OpCode::Eq(d, a, b)  => scalar(*d, cmp(*a, *b, "=="), next),
        OpCode::Neq(d, a, b) => scalar(*d, cmp(*a, *b, "!="), next),
        OpCode::Lt(d, a, b)  => scalar(*d, cmp(*a, *b, "<"), next),
        OpCode::Gt(d, a, b)  => scalar(*d, cmp(*a, *b, ">"), next),
        OpCode::Lte(d, a, b) => scalar(*d, cmp(*a, *b, "<="), next),
        OpCode::Gte(d, a, b) => scalar(*d, cmp(*a, *b, ">="), next),

        OpCode::And(d, a, b) => scalar(*d, format!("{} & {}", reg(*a), reg(*b)), next),
        OpCode::Or(d, a, b)  => scalar(*d, format!("{} | {}", reg(*a), reg(*b)), next),
        OpCode::Xor(d, a, b) => scalar(*d, format!("{} ^ {}", reg(*a), reg(*b)), next),
        OpCode::Shl(d, a, b) => scalar(*d, format!("({} as i64).wrapping_shl(({} as u32) & 63) as u64", reg(*a), reg(*b)), next),
        OpCode::Shr(d, a, b) => scalar(*d, format!("({} as i64).wrapping_shr(({} as u32) & 63) as u64", reg(*a), reg(*b)), next),

        OpCode::PushConst(d, idx) => {
            let is_h = ctx.const_is_handle.get(*idx as usize).copied().unwrap_or(false);
            if is_h {
                format!("{} = c{}; h.rc_inc_handle({})?; {} = true; pc = {};", reg(*d), idx, reg(*d), regh(*d), next)
            } else {
                format!("{} = c{}; {} = false; pc = {};", reg(*d), idx, regh(*d), next)
            }
        }
        OpCode::Copy(d, a) => format!(
            "{} = {}; {} = {}; if {} {{ h.rc_inc_handle({})?; }} pc = {};",
            reg(*d), reg(*a), regh(*d), regh(*a), regh(*d), reg(*d), next,
        ),
        OpCode::Move(d, a) => format!(
            "{{ let t = {}; let th = {}; {} = u64::MAX; {} = false; {} = t; {} = th; }} pc = {};",
            reg(*a), regh(*a), reg(*a), regh(*a), reg(*d), regh(*d), next,
        ),

        OpCode::Alloc(d, size) => format!(
            "{{ let (s, g) = h.try_alloc({}).map_err(|e| e)?; {} = myriad::Value::from_handle(s, g).raw(); {} = true; }} pc = {};",
            size, reg(*d), regh(*d), next,
        ),
        OpCode::Drop(reg_) => format!(
            "if {} {{ h.rc_dec_handle({})?; }} {} = u64::MAX; {} = false; pc = {};",
            regh(*reg_), reg(*reg_), reg(*reg_), regh(*reg_), next,
        ),
        OpCode::Ld(d, b, off) => ld_stmt(*d, *b, format!("{}", off), next),
        OpCode::St(src, b, off) => st_stmt(*src, *b, format!("{}", off), next),
        OpCode::LdIdx(d, b, idx) => {
            let pre = format!("let __o = {} as i64; if __o < 0 {{ return Err(\"ldidx: negative index\".to_string()); }}", reg(*idx));
            format!("{{ {} {} }}", pre, ld_stmt(*d, *b, "__o as usize".to_string(), next))
        }
        OpCode::StIdx(src, b, idx) => {
            let pre = format!("let __o = {} as i64; if __o < 0 {{ return Err(\"stidx: negative index\".to_string()); }}", reg(*idx));
            format!("{{ {} {} }}", pre, st_stmt(*src, *b, "__o as usize".to_string(), next))
        }

        OpCode::Jmp(off) => format!("{};", jump_to(i, *off, len)),
        OpCode::Jz(r, off) => format!("if {} == 0 {{ {} }} else {{ pc = {} }};", reg(*r), jump_to(i, *off, len), next),
        OpCode::Jnz(r, off) => format!("if {} != 0 {{ {} }} else {{ pc = {} }};", reg(*r), jump_to(i, *off, len), next),

        OpCode::Call(dest, fn_id) => call_stmt(*dest, *fn_id as usize, next, ctx)?,
        OpCode::Ret(a) => format!("return Ok(({}, {}));", reg(*a), regh(*a)),

        other => return Err(TranspileError::Unsupported(format!("{:?}", other))),
    };
    Ok(s)
}

fn ld_stmt(d: Register, b: Register, off: String, next: isize) -> String {
    format!(
        "{{ let (s, g) = myriad::Value::from_raw({}).as_handle(); let (v, vh) = h.ld(s, g, {})?; if vh {{ h.rc_inc_handle(v)?; }} {} = v; {} = vh; }} pc = {};",
        reg(b), off, reg(d), regh(d), next,
    )
}

fn st_stmt(src: Register, b: Register, off: String, next: isize) -> String {
    format!(
        "{{ let (s, g) = myriad::Value::from_raw({}).as_handle(); let v = {}; let vh = {}; {} = u64::MAX; {} = false; let (old, oldh) = h.st(s, g, {}, v, vh)?; if oldh {{ h.rc_dec_handle(old)?; }} }} pc = {};",
        reg(b), reg(src), regh(src), reg(src), regh(src), off, next,
    )
}

fn emit_function(out: &mut String, idx: usize, chunk: &BytecodeChunk, param_counts: &[usize]) -> Result<(), TranspileError> {
    let _ = writeln!(out, "fn f{}(h: &mut myriad::Heap, a: &[u64], ah: &[bool]) -> Result<(u64, bool), String> {{", idx);
    let nregs = chunk.reg_count + STAGE_SLACK;
    for n in 0..nregs {
        if n < chunk.param_count {
            let _ = writeln!(out, "    let mut r{}: u64 = a[{}]; let mut r{}_h: bool = ah[{}];", n, n, n, n);
        } else {
            let _ = writeln!(out, "    let mut r{}: u64 = 0; let mut r{}_h: bool = false;", n, n);
        }
    }
    for (off, c) in chunk.constants.iter().enumerate() {
        let _ = writeln!(out, "    let c{}: u64 = {}u64;", off, c);
    }
    let const_flags: Vec<bool> = (0..chunk.constants.len()).map(|i| chunk.const_is_handle(i as u16)).collect();
    let ctx = Ctx { reg_count: chunk.reg_count, param_counts, const_is_handle: &const_flags };
    let _ = writeln!(out, "    let mut pc: isize = 0;");
    let _ = writeln!(out, "    loop {{");
    let _ = writeln!(out, "        match pc {{");
    let len = chunk.code.len();
    for (i, op) in chunk.code.iter().enumerate() {
        let _ = writeln!(out, "            {} => {{ {} }}", i, op_stmt(i, op, len, &ctx)?);
    }
    let _ = writeln!(out, "            _ => return Ok((r0, r0_h)),");
    let _ = writeln!(out, "        }}");
    let _ = writeln!(out, "    }}");
    let _ = writeln!(out, "}}");
    Ok(())
}

/// Transpile a module to a standalone Rust program linking `myriad` for the
/// heap/RC runtime. `main` prints `OK <value> <live_cells>` or `ERR <msg>` so
/// the differential harness can compare both the result and heap leaks.
pub fn transpile_module(module: &Module) -> Result<String, TranspileError> {
    let param_counts: Vec<usize> = module.functions.iter().map(|c| c.param_count()).collect();
    let mut out = String::new();
    let _ = writeln!(out, "#![allow(unused_mut, unused_variables, dead_code, unused_assignments, unused_parens)]");
    for (idx, chunk) in module.functions.iter().enumerate() {
        match chunk {
            Chunk::Bytecode(bc) => emit_function(&mut out, idx, bc, &param_counts)?,
            Chunk::Native(n) => return Err(TranspileError::Unsupported(format!("native fn {}", n.name))),
        }
    }
    let _ = writeln!(out, "fn main() {{");
    let _ = writeln!(out, "    let mut h = myriad::Heap::new();");
    let _ = writeln!(out, "    match f{}(&mut h, &[], &[]) {{", module.entry);
    let _ = writeln!(out, "        Ok((v, _)) => println!(\"OK {{}} {{}}\", v, h.live_count()),");
    let _ = writeln!(out, "        Err(e) => println!(\"ERR {{}}\", e),");
    let _ = writeln!(out, "    }}");
    let _ = writeln!(out, "}}");
    Ok(out)
}

pub fn transpile_program(chunk: &BytecodeChunk) -> Result<String, TranspileError> {
    let module = Module {
        functions: vec![Chunk::Bytecode(chunk.clone())],
        entry: 0, flags: 0, exports: vec![],
    };
    transpile_module(&module)
}
