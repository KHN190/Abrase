use polka::{BytecodeChunk, Chunk, Module, OpCode, Register};
use std::fmt::Write;

mod embed;
mod hybrid;

const STAGE_SLACK: usize = 32;

struct Ctx<'a> {
    reg_count: usize,
    param_counts: &'a [usize],
    is_native: &'a [bool],
    const_is_handle: &'a [bool],
    int32_safe: bool,
    cart: Option<CartCtx>,
}

#[derive(Clone, Copy)]
struct CartCtx { present_id: usize, nregs: usize }

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

// Leaders = every pc the dispatch loop lands on: branch targets, branch
// fallthrough, @cart yield-resume point.
fn block_leaders(code: &[OpCode], present_id: Option<usize>) -> Vec<bool> {
    let len = code.len();
    let mut leader = vec![false; len];
    if len > 0 { leader[0] = true; }
    let mut mark = |t: isize| { if t >= 0 && (t as usize) < len { leader[t as usize] = true; } };
    for (i, op) in code.iter().enumerate() {
        match op {
            OpCode::Jmp(off) => { mark(target(i, *off)); mark(i as isize + 1); }
            OpCode::Jz(_, off) | OpCode::Jnz(_, off) => { mark(target(i, *off)); mark(i as isize + 1); }
            OpCode::Ret(_) => mark(i as isize + 1),
            OpCode::Call(_, fid) if Some(*fid as usize) == present_id => mark(i as isize + 1),
            _ => {}
        }
    }
    leader
}

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
    // Native callees take args by value; the VM drops the staged arg handles
    // after the call (frame.rs). Bytecode callees instead consume args via Move,
    // so only native calls need this caller-side rc release.
    let drop_args = if *ctx.is_native.get(fn_id).unwrap_or(&false) {
        (0..k).map(|j| {
            let r = reg(Register((ctx.reg_count + j) as u8));
            let rh = regh(Register((ctx.reg_count + j) as u8));
            format!("if {rh} {{ h.rc_dec_handle({r})?; }} {r} = u64::MAX; {rh} = false; ")
        }).collect::<String>()
    } else { String::new() };
    Ok(format!(
        "let (__rv, __rh) = f{}(h, host, rt, cs, mt, &[{}], &[{}])?; {}{} = __rv; {} = __rh; pc = {};",
        fn_id, vals.join(", "), hs.join(", "), drop_args, reg(dest), regh(dest), next,
    ))
}

fn rf(r: Register, i32s: bool) -> String {
    if i32s { format!("f32::from_bits({} as u32) as f64", reg(r)) }
    else { format!("f64::from_bits({})", reg(r)) }
}
fn narrowf(expr: String, i32s: bool) -> String {
    if i32s { format!("(({}) as f32).to_bits() as u64", expr) }
    else { format!("({}).to_bits()", expr) }
}
fn fbin(d: Register, a: Register, b: Register, op: &str, next: isize, i32s: bool) -> String {
    scalar(d, narrowf(format!("{} {} {}", rf(a, i32s), op, rf(b, i32s)), i32s), next)
}
fn fcmp(d: Register, a: Register, b: Register, op: &str, next: isize, i32s: bool) -> String {
    scalar(d, format!("{{ let x = {}; let y = {}; if x.is_nan() || y.is_nan() {{ 0u64 }} else if x {} y {{ 1 }} else {{ 0 }} }}", rf(a, i32s), rf(b, i32s), op), next)
}

fn callreg_stmt(dest: Register, fid_reg: Register, next: isize, ctx: &Ctx) -> String {
    let mut arms = String::new();
    for (fid, k) in ctx.param_counts.iter().enumerate() {
        let vals: Vec<String> = (0..*k).map(|j| reg(Register((ctx.reg_count + j) as u8))).collect();
        let hs: Vec<String> = (0..*k).map(|j| regh(Register((ctx.reg_count + j) as u8))).collect();
        let drop_args = if *ctx.is_native.get(fid).unwrap_or(&false) {
            (0..*k).map(|j| {
                let r = reg(Register((ctx.reg_count + j) as u8));
                let rh = regh(Register((ctx.reg_count + j) as u8));
                format!("if {rh} {{ h.rc_dec_handle({r})?; }} {r} = u64::MAX; {rh} = false; ")
            }).collect::<String>()
        } else { String::new() };
        let _ = write!(arms, "{} => {{ let (__rv, __rh) = f{}(h, host, rt, cs, mt, &[{}], &[{}])?; {}{} = __rv; {} = __rh; }} ",
            fid, fid, vals.join(", "), hs.join(", "), drop_args, reg(dest), regh(dest));
    }
    format!(
        "{{ let __fid = {} as i64; if !(0..=0xFFFF).contains(&__fid) {{ return Err(format!(\"call_reg: fn_id {{}} out of u16 range\", __fid)); }} match __fid {{ {} _ => return Err(format!(\"call: unknown fn_id {{}}\", __fid)) }} }} pc = {};",
        reg(fid_reg), arms, next,
    )
}

// `frame.present()`: save the live register window + resume pc into the state
// struct, hand a Unit result to the call's dest, and yield to the host driver.
fn cart_yield(dest: Register, next: isize, nregs: usize) -> String {
    let mut save = String::new();
    for n in 0..nregs {
        let _ = write!(save, "st.r{n} = r{n}; st.r{n}_h = r{n}_h; ", n = n);
    }
    format!("{} st.r{d} = 0; st.r{d}_h = false; st.pc = {}; return Ok(CartStep::Yield);",
        save, next, d = dest.0)
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
            "{{ let (s, g) = h.try_alloc({}).map_err(|e| e)?; rt.record_alloc(s, g); {} = myriad::Value::from_handle(s, g).raw(); {} = true; }} pc = {};",
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

        OpCode::FAdd(d, a, b) => fbin(*d, *a, *b, "+", next, ctx.int32_safe),
        OpCode::FSub(d, a, b) => fbin(*d, *a, *b, "-", next, ctx.int32_safe),
        OpCode::FMul(d, a, b) => fbin(*d, *a, *b, "*", next, ctx.int32_safe),
        OpCode::FDiv(d, a, b) => fbin(*d, *a, *b, "/", next, ctx.int32_safe),
        OpCode::FNeg(d, a) => scalar(*d, narrowf(format!("-({})", rf(*a, ctx.int32_safe)), ctx.int32_safe), next),
        OpCode::FLt(d, a, b) => fcmp(*d, *a, *b, "<", next, ctx.int32_safe),
        OpCode::FEq(d, a, b) => fcmp(*d, *a, *b, "==", next, ctx.int32_safe),

        OpCode::Jmp(off) => format!("{};", jump_to(i, *off, len)),
        OpCode::Jz(r, off) => format!("if {} == 0 {{ {} }} else {{ pc = {} }};", reg(*r), jump_to(i, *off, len), next),
        OpCode::Jnz(r, off) => format!("if {} != 0 {{ {} }} else {{ pc = {} }};", reg(*r), jump_to(i, *off, len), next),

        OpCode::Call(dest, fn_id) => match ctx.cart {
            Some(cc) if *fn_id as usize == cc.present_id => cart_yield(*dest, next, cc.nregs),
            _ => call_stmt(*dest, *fn_id as usize, next, ctx)?,
        },
        OpCode::CallReg(dest, fid_reg) => callreg_stmt(*dest, *fid_reg, next, ctx),
        OpCode::Ret(a) => if ctx.cart.is_some() {
            format!("return Ok(CartStep::Done({}, {}));", reg(*a), regh(*a))
        } else {
            format!("return Ok(({}, {}));", reg(*a), regh(*a))
        },

        OpCode::Deo(src, port_reg) => format!(
            "{{ let __pv = {pv} as i64; let __dev = ((__pv >> 8) & 0xFF) as u8; let __port = (__pv & 0xFF) as u8; \
             if __dev == {rid}u8 {{ match __port {{ \
             {push}u8 => rt.push(), \
             {pop}u8 => rt.pop_and_release(h)?, \
             {forget}u8 => {{ if {sh} && {sr} != u64::MAX {{ let (s, g) = myriad::Value::from_raw({sr}).as_handle(); rt.deep_forget(h, s, g); }} }}, \
             _ => return Err(format!(\"region: unknown port {{}}\", __port)) }} }} \
             else if __dev == {mid}u8 && __port == {mport}u8 {{ \
             if mt.1 && mt.0 != u64::MAX {{ h.rc_dec_handle(mt.0)?; }} *mt = ({sr}, {sh}); }} \
             else {{ return Err(format!(\"deo: unsupported device {{}} in AOT\", __dev)); }} }} pc = {next};",
            pv = reg(*port_reg), rid = polka::REGION_ID, push = polka::REGION_PORT_PUSH,
            pop = polka::REGION_PORT_POP, forget = polka::REGION_PORT_FORGET,
            mid = polka::MODULE_ID, mport = polka::MODULE_PORT_TABLE,
            sh = regh(*src), sr = reg(*src), next = next,
        ),

        OpCode::Dei(dest, port_reg) => format!(
            "{{ let __pv = {pv} as i64; let __dev = ((__pv >> 8) & 0xFF) as u8; let __port = (__pv & 0xFF) as u8; \
             if __dev == {mid}u8 && __port == {mport}u8 {{ \
             let (raw, ish) = *mt; if ish && raw != u64::MAX {{ h.rc_inc_handle(raw)?; }} {dr} = raw; {drh} = ish; }} \
             else {{ return Err(format!(\"dei: unsupported device {{}} in AOT\", __dev)); }} }} pc = {next};",
            pv = reg(*port_reg), mid = polka::MODULE_ID, mport = polka::MODULE_PORT_TABLE,
            dr = reg(*dest), drh = regh(*dest), next = next,
        ),

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

pub(crate) fn emit_pure_fn(out: &mut String, idx: usize, chunk: &BytecodeChunk,
                           param_counts: &[usize], is_native: &[bool], int32_safe: bool)
                           -> Result<(), TranspileError> {
    emit_function(out, idx, chunk, param_counts, is_native, int32_safe, None)
}

fn emit_function(out: &mut String, idx: usize, chunk: &BytecodeChunk, param_counts: &[usize],
                 is_native: &[bool], int32_safe: bool, cart: Option<CartCtx>) -> Result<(), TranspileError> {
    let nregs = chunk.reg_count + STAGE_SLACK;
    if cart.is_some() {
        let _ = writeln!(out, "#[derive(Default)] struct St{} {{ {} pc: isize }}", idx,
            (0..nregs).map(|n| format!("r{}: u64, r{}_h: bool,", n, n)).collect::<String>());
        let _ = writeln!(out, "fn f{}_step(st: &mut St{}, h: &mut myriad::Heap, host: &mut dyn myriad::AotNatives, rt: &mut myriad::RegionTable, cs: &[Vec<u64>], mt: &mut (u64, bool)) -> Result<CartStep, String> {{", idx, idx);
        for n in 0..nregs {
            let _ = writeln!(out, "    let mut r{}: u64 = st.r{}; let mut r{}_h: bool = st.r{}_h;", n, n, n, n);
        }
        let _ = writeln!(out, "    let mut pc: isize = st.pc;");
    } else {
        let _ = writeln!(out, "fn f{}(h: &mut myriad::Heap, host: &mut dyn myriad::AotNatives, rt: &mut myriad::RegionTable, cs: &[Vec<u64>], mt: &mut (u64, bool), a: &[u64], ah: &[bool]) -> Result<(u64, bool), String> {{", idx);
        for n in 0..nregs {
            if n < chunk.param_count {
                let _ = writeln!(out, "    let mut r{}: u64 = a[{}]; let mut r{}_h: bool = ah[{}];", n, n, n, n);
            } else {
                let _ = writeln!(out, "    let mut r{}: u64 = 0; let mut r{}_h: bool = false;", n, n);
            }
        }
        let _ = writeln!(out, "    let mut pc: isize = 0;");
    }
    for off in 0..chunk.constants.len() {
        let _ = writeln!(out, "    let c{}: u64 = cs[{}][{}];", off, idx, off);
    }
    let const_flags: Vec<bool> = (0..chunk.constants.len()).map(|i| chunk.const_is_handle(i as u16)).collect();
    let ctx = Ctx { reg_count: chunk.reg_count, param_counts, is_native, const_is_handle: &const_flags, int32_safe, cart };
    let _ = writeln!(out, "    loop {{");
    let _ = writeln!(out, "        match pc {{");
    let len = chunk.code.len();
    let leaders = block_leaders(&chunk.code, cart.map(|c| c.present_id));
    let mut i = 0;
    while i < len {
        let mut body = String::new();
        let mut j = i;
        loop {
            body.push_str(&op_stmt(j, &chunk.code[j], len, &ctx)?);
            body.push(' ');
            j += 1;
            if j >= len || leaders[j] { break; }
        }
        let _ = writeln!(out, "            {} => {{ {} }}", i, body);
        i = j;
    }
    if cart.is_some() {
        let _ = writeln!(out, "            _ => return Ok(CartStep::Done(r0, r0_h)),");
    } else {
        let _ = writeln!(out, "            _ => return Ok((r0, r0_h)),");
    }
    let _ = writeln!(out, "        }}");
    let _ = writeln!(out, "    }}");
    let _ = writeln!(out, "}}");
    Ok(())
}

// A `@cart` module: entry fn calls the `__frame_present` native. Returns the
// present-native id + entry register window so the entry can yield/resume.
fn cart_info(module: &Module) -> Option<CartCtx> {
    let present_id = module.functions.iter()
        .position(|c| matches!(c, Chunk::Native(n) if n.name == "__frame_present"))?;
    if let Chunk::Bytecode(bc) = &module.functions[module.entry] {
        if bc.code.iter().any(|op| matches!(op, OpCode::Call(_, f) if *f as usize == present_id)) {
            return Some(CartCtx { present_id, nregs: bc.reg_count + STAGE_SLACK });
        }
    }
    None
}

fn emit_fns(out: &mut String, module: &Module) -> Result<(), TranspileError> {
    let param_counts: Vec<usize> = module.functions.iter().map(|c| c.param_count()).collect();
    let is_native: Vec<bool> = module.functions.iter().map(|c| matches!(c, Chunk::Native(_))).collect();
    let int32_safe = (module.flags & polka::CART_FLAG_INT32_SAFE) != 0;
    let cart = cart_info(module);
    for (idx, chunk) in module.functions.iter().enumerate() {
        let fn_cart = if idx == module.entry { cart } else { None };
        match chunk {
            Chunk::Bytecode(bc) => emit_function(out, idx, bc, &param_counts, &is_native, int32_safe, fn_cart)?,
            Chunk::Native(n) => {
                let _ = writeln!(out,
                    "fn f{}(h: &mut myriad::Heap, host: &mut dyn myriad::AotNatives, _rt: &mut myriad::RegionTable, _cs: &[Vec<u64>], _mt: &mut (u64, bool), a: &[u64], _ah: &[bool]) -> Result<(u64, bool), String> {{ let args: Vec<myriad::Value> = a.iter().map(|&x| myriad::Value::from_raw(x)).collect(); host.call({:?}, h, &args) }}",
                    idx, n.name);
            }
        }
    }
    Ok(())
}

fn emit_run_body(out: &mut String, module: &Module) {
    let _ = writeln!(out, "    let mut cs: Vec<Vec<u64>> = Vec::new();");
    let _ = writeln!(out, "    let mut __sc: Vec<(u32, u32)> = Vec::new();");
    for chunk in module.functions.iter() {
        let _ = writeln!(out, "    {{ let mut v: Vec<u64> = Vec::new();");
        if let Chunk::Bytecode(bc) = chunk {
            for (off, c) in bc.constants.iter().enumerate() {
                if bc.const_is_handle(off as u16) {
                    let sidx = *c as usize;
                    let s = bc.string_constants.get(sidx).cloned().unwrap_or_default();
                    let _ = writeln!(out, "        {{ let sv = myriad::alloc_string(h, {:?}).expect(\"string const\"); __sc.push(sv.as_handle()); v.push(sv.raw()); }}", s);
                } else {
                    let _ = writeln!(out, "        v.push({}u64);", c);
                }
            }
        }
        let _ = writeln!(out, "        cs.push(v); }}");
    }
    let _ = writeln!(out, "    let mut rt = myriad::RegionTable::new();");
    let _ = writeln!(out, "    let mut mt: (u64, bool) = (u64::MAX, false);");
    if let Some(init) = module.exports.iter().find(|e| e.name == "__module_init") {
        let _ = writeln!(out, "    let _ = f{}(h, host, &mut rt, &cs, &mut mt, &[], &[])?;", init.fn_id);
    }
    if cart_info(module).is_some() {
        let _ = writeln!(out, "    let mut st = St{}::default();", module.entry);
        let _ = writeln!(out, "    let (v, _) = loop {{ match f{}_step(&mut st, h, host, &mut rt, &cs, &mut mt)? {{", module.entry);
        let _ = writeln!(out, "        CartStep::Yield => {{ if host.halted() {{ break (0u64, false); }} }},");
        let _ = writeln!(out, "        CartStep::Done(v, vh) => break (v, vh),");
        let _ = writeln!(out, "    }} }};");
    } else {
        let _ = writeln!(out, "    let (v, _) = f{}(h, host, &mut rt, &cs, &mut mt, &[], &[])?;", module.entry);
    }
    let _ = writeln!(out, "    let __const_live = __sc.iter().filter(|(s, g)| h.is_live(*s, *g)).count();");
    let _ = writeln!(out, "    let __mt_live = if mt.1 {{ myriad::reachable_live_count(h, mt.0) }} else {{ 0 }};");
    let _ = writeln!(out, "    let live = h.live_count() - __const_live - __mt_live;");
    let _ = writeln!(out, "    Ok((v, live))");
}

pub fn transpile_module(module: &Module) -> Result<String, TranspileError> {
    if embed::is_effectful(module) {
        return hybrid::transpile_module(module);
    }
    emit_native(module, false)
}

fn emit_native(module: &Module, lib: bool) -> Result<String, TranspileError> {
    let mut out = String::new();
    let _ = writeln!(out, "#![allow(unused_mut, unused_variables, dead_code, unused_assignments, unused_parens)]");
    let _ = writeln!(out, "{}enum CartStep {{ Yield, Done(u64, bool) }}", if lib { "pub " } else { "" });
    emit_fns(&mut out, module)?;
    let _ = writeln!(out, "{}fn run(h: &mut myriad::Heap, host: &mut dyn myriad::AotNatives) -> Result<(u64, usize), String> {{", if lib { "pub " } else { "" });
    emit_run_body(&mut out, module);
    let _ = writeln!(out, "}}");
    if lib { return Ok(out); }
    let _ = writeln!(out, "fn main() {{");
    let _ = writeln!(out, "    use std::io::Write;");
    let _ = writeln!(out, "    let mut h = myriad::Heap::new();");
    let _ = writeln!(out, "    let mut host = myriad::AotHost::new();");
    let _ = writeln!(out, "    let r = run(&mut h, &mut host);");
    let _ = writeln!(out, "    let _ = std::io::stdout().write_all(&host.take_stdout());");
    let _ = writeln!(out, "    match r {{");
    let _ = writeln!(out, "        Ok((v, live)) => println!(\"OK {{}} {{}}\", v, live),");
    let _ = writeln!(out, "        Err(e) => println!(\"ERR {{}}\", e),");
    let _ = writeln!(out, "    }}");
    let _ = writeln!(out, "}}");
    Ok(out)
}

pub fn transpile_module_lib(module: &Module) -> Result<String, TranspileError> {
    if embed::is_effectful(module) {
        return hybrid::transpile_module_lib(module);
    }
    emit_native(module, true)
}

pub fn transpile_batch(modules: &[&Module]) -> Result<String, TranspileError> {
    let mut out = String::new();
    let _ = writeln!(out, "#![allow(unused_mut, unused_variables, dead_code, unused_assignments, unused_parens)]");
    for (i, m) in modules.iter().enumerate() {
        let _ = writeln!(out, "mod p{} {{", i);
        emit_fns(&mut out, m)?;
        let _ = writeln!(out, "    pub fn run(h: &mut myriad::Heap, host: &mut myriad::AotHost) -> Result<(u64, usize), String> {{");
        emit_run_body(&mut out, m);
        let _ = writeln!(out, "    }}");
        let _ = writeln!(out, "}}");
    }
    let _ = writeln!(out, "fn main() {{");
    for i in 0..modules.len() {
        let _ = writeln!(out, "    {{ let mut h = myriad::Heap::new(); let mut host = myriad::AotHost::new();");
        let _ = writeln!(out, "      match p{}::run(&mut h, &mut host) {{ Ok((v, live)) => println!(\"{} OK {{}} {{}}\", v, live), Err(e) => println!(\"{} ERR {{}}\", e) }} }}", i, i, i);
    }
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
