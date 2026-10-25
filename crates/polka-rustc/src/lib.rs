use polka::{BytecodeChunk, Chunk, Module, OpCode, Register};
use std::fmt::Write;

pub mod core;
mod embed;
mod hybrid;

const STAGE_SLACK: usize = 32;

#[derive(Clone, Copy)]
pub(crate) struct TargetWidth {
    narrow: bool,
}

impl TargetWidth {
    pub(crate) fn for_module(module: &Module) -> Self {
        Self {
            narrow: module.flags & polka::CART_FLAG_INT32_SAFE != 0,
        }
    }
    fn for_flag(narrow: bool) -> Self {
        Self { narrow }
    }
    pub(crate) fn namespace(self) -> &'static str {
        if self.narrow {
            "myriad::word32"
        } else {
            "myriad::word64"
        }
    }
    pub(crate) fn aliases(self, out: &mut String) {
        let _ = writeln!(out, "use {} as runtime;", self.namespace());
        let _ = writeln!(
            out,
            "#[allow(unused_imports)] use runtime::{{Word, Int, Float}};"
        );
    }
    pub(crate) fn constant(self, raw: polka::Constant) -> Result<String, TranspileError> {
        let bits = if self.narrow {
            polka::constant_word32(raw)
                .map(u64::from)
                .map_err(|e| TranspileError::Unsupported(e.into()))?
        } else {
            u64::from_le_bytes(raw.to_le_bytes())
        };
        Ok(format!("{bits} as Word"))
    }
}

struct Ctx<'a> {
    reg_count: usize,
    param_counts: &'a [usize],
    is_native: &'a [bool],
    const_is_handle: &'a [bool],
    int32_safe: bool,
    cart: Option<CartCtx>,
}

#[derive(Clone, Copy)]
struct CartCtx {
    present_id: usize,
    nregs: usize,
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
    format!("({} as Int).{}({} as Int) as Word", reg(a), op, reg(b))
}
fn cmp(a: Register, b: Register, op: &str) -> String {
    format!(
        "if ({} as Int) {} ({} as Int) {{ 1 }} else {{ 0 }}",
        reg(a),
        op,
        reg(b)
    )
}
fn checked(d: Register, a: Register, b: Register, method: &str, msg: &str, next: isize) -> String {
    format!(
        "{} = match ({} as Int).{}({} as Int) {{ Some(v) => v as Word, None => return Err(\"{}\".to_string()) }}; {} = false; pc = {};",
        reg(d),
        reg(a),
        method,
        reg(b),
        msg,
        regh(d),
        next,
    )
}

fn call_stmt(
    dest: Register,
    fn_id: usize,
    next: isize,
    ctx: &Ctx,
) -> Result<String, TranspileError> {
    let k = *ctx
        .param_counts
        .get(fn_id)
        .ok_or_else(|| TranspileError::Unsupported(format!("call to unknown fn {}", fn_id)))?;
    let vals: Vec<String> = (0..k)
        .map(|j| reg(Register((ctx.reg_count + j) as u8)))
        .collect();
    let hs: Vec<String> = (0..k)
        .map(|j| regh(Register((ctx.reg_count + j) as u8)))
        .collect();
    // Native callees take args by value; the VM drops the staged arg handles
    // after the call (frame.rs). Bytecode callees instead consume args via Move,
    // so only native calls need this caller-side rc release.
    let drop_args = if *ctx.is_native.get(fn_id).unwrap_or(&false) {
        (0..k)
            .map(|j| {
                let r = reg(Register((ctx.reg_count + j) as u8));
                let rh = regh(Register((ctx.reg_count + j) as u8));
                format!("if {rh} {{ h.rc_dec_handle({r})?; }} {r} = Word::MAX; {rh} = false; ")
            })
            .collect::<String>()
    } else {
        String::new()
    };
    Ok(format!(
        "let (__rv, __rh) = f{}(h, host, rt, cs, mt, &[{}], &[{}])?; {}{} = __rv; {} = __rh; pc = {};",
        fn_id,
        vals.join(", "),
        hs.join(", "),
        drop_args,
        reg(dest),
        regh(dest),
        next,
    ))
}

pub(crate) fn fdec(expr: &str, _i32s: bool) -> String {
    format!("Float::from_bits({})", expr)
}
pub(crate) fn fenc(expr: &str, _i32s: bool) -> String {
    format!("({}).to_bits()", expr)
}
fn rf(r: Register, i32s: bool) -> String { fdec(&reg(r), i32s) }
fn narrowf(expr: String, i32s: bool) -> String { fenc(&expr, i32s) }
fn fbin(d: Register, a: Register, b: Register, op: &str, next: isize, i32s: bool) -> String {
    scalar(
        d,
        narrowf(format!("{} {} {}", rf(a, i32s), op, rf(b, i32s)), i32s),
        next,
    )
}
fn fcmp(d: Register, a: Register, b: Register, op: &str, next: isize, i32s: bool) -> String {
    scalar(
        d,
        format!(
            "{{ let x = {}; let y = {}; if x.is_nan() || y.is_nan() {{ (0 as Word) }} else if x {} y {{ 1 }} else {{ 0 }} }}",
            rf(a, i32s),
            rf(b, i32s),
            op
        ),
        next,
    )
}

fn callreg_stmt(dest: Register, fid_reg: Register, next: isize, ctx: &Ctx) -> String {
    let mut arms = String::new();
    for (fid, k) in ctx.param_counts.iter().enumerate() {
        let vals: Vec<String> = (0..*k)
            .map(|j| reg(Register((ctx.reg_count + j) as u8)))
            .collect();
        let hs: Vec<String> = (0..*k)
            .map(|j| regh(Register((ctx.reg_count + j) as u8)))
            .collect();
        let drop_args = if *ctx.is_native.get(fid).unwrap_or(&false) {
            (0..*k)
                .map(|j| {
                    let r = reg(Register((ctx.reg_count + j) as u8));
                    let rh = regh(Register((ctx.reg_count + j) as u8));
                    format!("if {rh} {{ h.rc_dec_handle({r})?; }} {r} = Word::MAX; {rh} = false; ")
                })
                .collect::<String>()
        } else {
            String::new()
        };
        let _ = write!(
            arms,
            "{} => {{ let (__rv, __rh) = f{}(h, host, rt, cs, mt, &[{}], &[{}])?; {}{} = __rv; {} = __rh; }} ",
            fid,
            fid,
            vals.join(", "),
            hs.join(", "),
            drop_args,
            reg(dest),
            regh(dest)
        );
    }
    format!(
        "{{ let __fid = {} as Int; if !(0..=0xFFFF).contains(&__fid) {{ return Err(format!(\"call_reg: fn_id {{}} out of u16 range\", __fid)); }} match __fid {{ {} _ => return Err(format!(\"call: unknown fn_id {{}}\", __fid)) }} }} pc = {};",
        reg(fid_reg),
        arms,
        next,
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
        OpCode::Neg(d, a) => scalar(
            *d,
            format!("({} as Int).wrapping_neg() as Word", reg(*a)),
            next,
        ),
        OpCode::AddImm(d, a, imm) => scalar(
            *d,
            format!("({} as Int).wrapping_add({}) as Word", reg(*a), *imm),
            next,
        ),
        OpCode::SubImm(d, a, imm) => scalar(
            *d,
            format!("({} as Int).wrapping_sub({}) as Word", reg(*a), *imm),
            next,
        ),

        OpCode::Eq(d, a, b) => scalar(*d, cmp(*a, *b, "=="), next),
        OpCode::Neq(d, a, b) => scalar(*d, cmp(*a, *b, "!="), next),
        OpCode::Lt(d, a, b) => scalar(*d, cmp(*a, *b, "<"), next),
        OpCode::Gt(d, a, b) => scalar(*d, cmp(*a, *b, ">"), next),
        OpCode::Lte(d, a, b) => scalar(*d, cmp(*a, *b, "<="), next),
        OpCode::Gte(d, a, b) => scalar(*d, cmp(*a, *b, ">="), next),

        OpCode::And(d, a, b) => scalar(*d, format!("{} & {}", reg(*a), reg(*b)), next),
        OpCode::Or(d, a, b) => scalar(*d, format!("{} | {}", reg(*a), reg(*b)), next),
        OpCode::Xor(d, a, b) => scalar(*d, format!("{} ^ {}", reg(*a), reg(*b)), next),
        OpCode::Shl(d, a, b) => scalar(
            *d,
            format!(
                "({} as Int).wrapping_shl(({} as u32) & (Word::BITS - 1)) as Word",
                reg(*a),
                reg(*b)
            ),
            next,
        ),
        OpCode::Shr(d, a, b) => scalar(
            *d,
            format!(
                "({} as Int).wrapping_shr(({} as u32) & (Word::BITS - 1)) as Word",
                reg(*a),
                reg(*b)
            ),
            next,
        ),

        OpCode::PushConst(d, idx) => {
            let is_h = ctx
                .const_is_handle
                .get(*idx as usize)
                .copied()
                .unwrap_or(false);
            if is_h {
                format!(
                    "{} = c{}; h.rc_inc_handle({})?; {} = true; pc = {};",
                    reg(*d),
                    idx,
                    reg(*d),
                    regh(*d),
                    next
                )
            } else {
                format!(
                    "{} = c{}; {} = false; pc = {};",
                    reg(*d),
                    idx,
                    regh(*d),
                    next
                )
            }
        }
        OpCode::Copy(d, a) => format!(
            "{} = {}; {} = {}; if {} {{ h.rc_inc_handle({})?; }} pc = {};",
            reg(*d),
            reg(*a),
            regh(*d),
            regh(*a),
            regh(*d),
            reg(*d),
            next,
        ),
        OpCode::Move(d, a) => format!(
            "{{ let t = {}; let th = {}; {} = Word::MAX; {} = false; {} = t; {} = th; }} pc = {};",
            reg(*a),
            regh(*a),
            reg(*a),
            regh(*a),
            reg(*d),
            regh(*d),
            next,
        ),

        OpCode::Alloc(d, size) => format!(
            "{{ let (s, g) = h.try_alloc({}).map_err(|e| e)?; rt.record_alloc(s, g); {} = runtime::Value::from_handle(s, g).raw(); {} = true; }} pc = {};",
            size,
            reg(*d),
            regh(*d),
            next,
        ),
        OpCode::Drop(reg_) => format!(
            "if {} {{ h.rc_dec_handle({})?; }} {} = Word::MAX; {} = false; pc = {};",
            regh(*reg_),
            reg(*reg_),
            reg(*reg_),
            regh(*reg_),
            next,
        ),
        OpCode::Ld(d, b, off) => ld_stmt(*d, *b, format!("{}", off), next),
        OpCode::St(src, b, off) => st_stmt(*src, *b, format!("{}", off), next),
        OpCode::LdIdx(d, b, idx) => {
            let pre = format!(
                "let __o = {} as Int; if __o < 0 {{ return Err(\"ldidx: negative index\".to_string()); }}",
                reg(*idx)
            );
            format!(
                "{{ {} {} }}",
                pre,
                ld_stmt(*d, *b, "__o as usize".to_string(), next)
            )
        }
        OpCode::StIdx(src, b, idx) => {
            let pre = format!(
                "let __o = {} as Int; if __o < 0 {{ return Err(\"stidx: negative index\".to_string()); }}",
                reg(*idx)
            );
            format!(
                "{{ {} {} }}",
                pre,
                st_stmt(*src, *b, "__o as usize".to_string(), next)
            )
        }

        OpCode::FAdd(d, a, b) => fbin(*d, *a, *b, "+", next, ctx.int32_safe),
        OpCode::FSub(d, a, b) => fbin(*d, *a, *b, "-", next, ctx.int32_safe),
        OpCode::FMul(d, a, b) => fbin(*d, *a, *b, "*", next, ctx.int32_safe),
        OpCode::FDiv(d, a, b) => fbin(*d, *a, *b, "/", next, ctx.int32_safe),
        OpCode::FNeg(d, a) => scalar(
            *d,
            narrowf(format!("-({})", rf(*a, ctx.int32_safe)), ctx.int32_safe),
            next,
        ),
        OpCode::FLt(d, a, b) => fcmp(*d, *a, *b, "<", next, ctx.int32_safe),
        OpCode::FEq(d, a, b) => fcmp(*d, *a, *b, "==", next, ctx.int32_safe),

        OpCode::Jmp(off) => format!("{};", jump_to(i, *off, len)),
        OpCode::Jz(r, off) => format!(
            "if {} == 0 {{ {} }} else {{ pc = {} }};",
            reg(*r),
            jump_to(i, *off, len),
            next
        ),
        OpCode::Jnz(r, off) => format!(
            "if {} != 0 {{ {} }} else {{ pc = {} }};",
            reg(*r),
            jump_to(i, *off, len),
            next
        ),

        OpCode::Call(dest, fn_id) => match ctx.cart {
            Some(cc) if *fn_id as usize == cc.present_id => cart_yield(*dest, next, cc.nregs),
            _ => call_stmt(*dest, *fn_id as usize, next, ctx)?,
        },
        OpCode::CallReg(dest, fid_reg) => callreg_stmt(*dest, *fid_reg, next, ctx),
        OpCode::Ret(a) => {
            if ctx.cart.is_some() {
                format!("return Ok(CartStep::Done({}, {}));", reg(*a), regh(*a))
            } else {
                format!("return Ok(({}, {}));", reg(*a), regh(*a))
            }
        }

        OpCode::Deo(src, port_reg) => format!(
            "{{ let __pv = {pv} as Int; let __dev = ((__pv >> 8) & 0xFF) as u8; let __port = (__pv & 0xFF) as u8; \
             if __dev == {rid}u8 {{ match __port {{ \
             {push}u8 => rt.push(), \
             {pop}u8 => rt.pop_and_release(h)?, \
             {forget}u8 => {{ if {sh} && {sr} != Word::MAX {{ let (s, g) = runtime::Value::from_raw({sr}).as_handle(); rt.deep_forget(h, s, g); }} }}, \
             _ => return Err(format!(\"region: unknown port {{}}\", __port)) }} }} \
             else if __dev == {mid}u8 && __port == {mport}u8 {{ \
             if mt.1 && mt.0 != Word::MAX {{ h.rc_dec_handle(mt.0)?; }} *mt = ({sr}, {sh}); }} \
             else {{ return Err(format!(\"deo: unsupported device {{}} in AOT\", __dev)); }} }} pc = {next};",
            pv = reg(*port_reg),
            rid = polka::REGION_ID,
            push = polka::REGION_PORT_PUSH,
            pop = polka::REGION_PORT_POP,
            forget = polka::REGION_PORT_FORGET,
            mid = polka::MODULE_ID,
            mport = polka::MODULE_PORT_TABLE,
            sh = regh(*src),
            sr = reg(*src),
            next = next,
        ),

        OpCode::Dei(dest, port_reg) => format!(
            "{{ let __pv = {pv} as Int; let __dev = ((__pv >> 8) & 0xFF) as u8; let __port = (__pv & 0xFF) as u8; \
             if __dev == {mid}u8 && __port == {mport}u8 {{ \
             let (raw, ish) = *mt; if ish && raw != Word::MAX {{ h.rc_inc_handle(raw)?; }} {dr} = raw; {drh} = ish; }} \
             else {{ return Err(format!(\"dei: unsupported device {{}} in AOT\", __dev)); }} }} pc = {next};",
            pv = reg(*port_reg),
            mid = polka::MODULE_ID,
            mport = polka::MODULE_PORT_TABLE,
            dr = reg(*dest),
            drh = regh(*dest),
            next = next,
        ),

        other => return Err(TranspileError::Unsupported(format!("{:?}", other))),
    };
    Ok(s)
}

fn ld_stmt(d: Register, b: Register, off: String, next: isize) -> String {
    format!(
        "{{ let (s, g) = runtime::Value::from_raw({}).as_handle(); let (v, vh) = h.ld(s, g, {})?; if vh {{ h.rc_inc_handle(v)?; }} {} = v; {} = vh; }} pc = {};",
        reg(b),
        off,
        reg(d),
        regh(d),
        next,
    )
}

fn st_stmt(src: Register, b: Register, off: String, next: isize) -> String {
    format!(
        "{{ let (s, g) = runtime::Value::from_raw({}).as_handle(); let v = {}; let vh = {}; {} = Word::MAX; {} = false; let (old, oldh) = h.st(s, g, {}, v, vh)?; if oldh {{ h.rc_dec_handle(old)?; }} }} pc = {};",
        reg(b),
        reg(src),
        regh(src),
        reg(src),
        regh(src),
        off,
        next,
    )
}

pub(crate) fn emit_pure_fn(
    out: &mut String,
    idx: usize,
    chunk: &BytecodeChunk,
    param_counts: &[usize],
    is_native: &[bool],
    int32_safe: bool,
) -> Result<(), TranspileError> {
    emit_function(out, idx, chunk, param_counts, is_native, int32_safe, None)
}

fn emit_function(
    out: &mut String,
    idx: usize,
    chunk: &BytecodeChunk,
    param_counts: &[usize],
    is_native: &[bool],
    int32_safe: bool,
    cart: Option<CartCtx>,
) -> Result<(), TranspileError> {
    let nregs = chunk.reg_count + STAGE_SLACK;
    if cart.is_some() {
        let _ = writeln!(
            out,
            "#[derive(Default)] struct St{} {{ {} pc: isize }}",
            idx,
            (0..nregs)
                .map(|n| format!("r{}: Word, r{}_h: bool,", n, n))
                .collect::<String>()
        );
        let _ = writeln!(
            out,
            "fn f{}_step(st: &mut St{}, h: &mut runtime::Heap, host: &mut dyn runtime::AotNatives, rt: &mut runtime::RegionTable, cs: &[Vec<Word>], mt: &mut (Word, bool)) -> Result<CartStep, String> {{",
            idx, idx
        );
        for n in 0..nregs {
            let _ = writeln!(
                out,
                "    let mut r{}: Word = st.r{}; let mut r{}_h: bool = st.r{}_h;",
                n, n, n, n
            );
        }
        let _ = writeln!(out, "    let mut pc: isize = st.pc;");
    } else {
        let _ = writeln!(
            out,
            "fn f{}(h: &mut runtime::Heap, host: &mut dyn runtime::AotNatives, rt: &mut runtime::RegionTable, cs: &[Vec<Word>], mt: &mut (Word, bool), a: &[Word], ah: &[bool]) -> Result<(Word, bool), String> {{",
            idx
        );
        for n in 0..nregs {
            if n < chunk.param_count {
                let _ = writeln!(
                    out,
                    "    let mut r{}: Word = a[{}]; let mut r{}_h: bool = ah[{}];",
                    n, n, n, n
                );
            } else {
                let _ = writeln!(
                    out,
                    "    let mut r{}: Word = 0; let mut r{}_h: bool = false;",
                    n, n
                );
            }
        }
        let _ = writeln!(out, "    let mut pc: isize = 0;");
    }
    let const_flags: Vec<bool> = (0..chunk.constants.len())
        .map(|i| chunk.const_is_handle(i as u16))
        .collect();
    for off in 0..chunk.constants.len() {
        if const_flags[off] {
            let _ = writeln!(out, "    let c{}: Word = cs[{}][{}];", off, idx, off);
        } else {
            let literal = TargetWidth::for_flag(int32_safe).constant(chunk.constants[off])?;
            let _ = writeln!(out, "    let c{}: Word = {};", off, literal);
        }
    }
    let ctx = Ctx {
        reg_count: chunk.reg_count,
        param_counts,
        is_native,
        const_is_handle: &const_flags,
        int32_safe,
        cart,
    };
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
            if j >= len || leaders[j] {
                break;
            }
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
    let present_id = module
        .functions
        .iter()
        .position(|c| matches!(c, Chunk::Native(n) if n.name == "__frame_present"))?;
    if let Chunk::Bytecode(bc) = &module.functions[module.entry] {
        if bc
            .code
            .iter()
            .any(|op| matches!(op, OpCode::Call(_, f) if *f as usize == present_id))
        {
            return Some(CartCtx {
                present_id,
                nregs: bc.reg_count + STAGE_SLACK,
            });
        }
    }
    None
}

fn emit_fns(out: &mut String, module: &Module) -> Result<(), TranspileError> {
    let param_counts: Vec<usize> = module.functions.iter().map(|c| c.param_count()).collect();
    let is_native: Vec<bool> = module
        .functions
        .iter()
        .map(|c| matches!(c, Chunk::Native(_)))
        .collect();
    let int32_safe = (module.flags & polka::CART_FLAG_INT32_SAFE) != 0;
    let cart = cart_info(module);
    for (idx, chunk) in module.functions.iter().enumerate() {
        let fn_cart = if idx == module.entry { cart } else { None };
        match chunk {
            Chunk::Bytecode(bc) => {
                emit_function(out, idx, bc, &param_counts, &is_native, int32_safe, fn_cart)?
            }
            Chunk::Native(n) => hybrid::emit_native_fn(out, idx, &n.name, int32_safe),
        }
    }
    Ok(())
}

fn emit_setup(out: &mut String, module: &Module) -> Result<(), TranspileError> {
    let width = TargetWidth::for_module(module);
    let _ = writeln!(out, "    let mut cs: Vec<Vec<Word>> = Vec::new();");
    let _ = writeln!(out, "    let mut __sc: Vec<(u32, u32)> = Vec::new();");
    for chunk in &module.functions {
        let _ = writeln!(out, "    {{ let mut v: Vec<Word> = Vec::new();");
        if let Chunk::Bytecode(bc) = chunk {
            for (off, &constant) in bc.constants.iter().enumerate() {
                if bc.const_is_handle(off as u16) {
                    let index = polka::constant_pool_index(constant)
                        .map_err(|e| TranspileError::Unsupported(e.into()))?;
                    let expression = if polka::constant_is_bytes(constant) {
                        let bytes = bc.bytes_constants.get(index).ok_or_else(|| {
                            TranspileError::Unsupported("bytes constant pool index".into())
                        })?;
                        format!("runtime::alloc_bytes(h, &{:?})", bytes)
                    } else {
                        let text = bc.string_constants.get(index).ok_or_else(|| {
                            TranspileError::Unsupported("string constant pool index".into())
                        })?;
                        format!("runtime::alloc_string(h, {:?})", text)
                    };
                    let _ = writeln!(
                        out,
                        "        {{ let sv = {}?; __sc.push(sv.as_handle()); v.push(sv.raw()); }}",
                        expression
                    );
                } else {
                    let _ = writeln!(out, "        v.push({});", width.constant(constant)?);
                }
            }
        }
        let _ = writeln!(out, "        cs.push(v); }}");
    }
    let _ = writeln!(out, "    let mut rt = runtime::RegionTable::new();");
    let _ = writeln!(out, "    let mut mt: (Word, bool) = (Word::MAX, false);");
    if let Some(init) = module.exports.iter().find(|e| e.name == "__module_init") {
        let _ = writeln!(
            out,
            "    let _ = f{}(h, host, &mut rt, &cs, &mut mt, &[], &[])?;",
            init.fn_id
        );
    }
    Ok(())
}

fn emit_run_body(out: &mut String, module: &Module) -> Result<(), TranspileError> {
    emit_setup(out, module)?;
    if cart_info(module).is_some() {
        let _ = writeln!(out, "    let mut st = St{}::default();", module.entry);
        let _ = writeln!(
            out,
            "    let (v, _) = loop {{ match f{}_step(&mut st, h, host, &mut rt, &cs, &mut mt)? {{",
            module.entry
        );
        let _ = writeln!(
            out,
            "        CartStep::Yield => {{ if host.halted() {{ break ((0 as Word), false); }} }},"
        );
        let _ = writeln!(out, "        CartStep::Done(v, vh) => break (v, vh),");
        let _ = writeln!(out, "    }} }};");
    } else {
        let _ = writeln!(
            out,
            "    let (v, _) = f{}(h, host, &mut rt, &cs, &mut mt, &[], &[])?;",
            module.entry
        );
    }
    let _ = writeln!(
        out,
        "    let __const_live = __sc.iter().filter(|(s, g)| h.is_live(*s, *g)).count();"
    );
    let _ = writeln!(
        out,
        "    let __mt_live = if mt.1 {{ runtime::reachable_live_count(h, mt.0) }} else {{ 0 }};"
    );
    let _ = writeln!(
        out,
        "    let live = h.live_count() - __const_live - __mt_live;"
    );
    let _ = writeln!(out, "    Ok((v, live))");
    Ok(())
}

pub fn transpile_module(module: &Module) -> Result<String, TranspileError> {
    if embed::is_effectful(module) {
        return hybrid::transpile_module(module);
    }
    emit_native(module)
}

fn emit_native(module: &Module) -> Result<String, TranspileError> {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "#![allow(unused_mut, unused_variables, dead_code, unused_assignments, unused_parens)]"
    );
    TargetWidth::for_module(module).aliases(&mut out);
    let _ = writeln!(out, "enum CartStep {{ Yield, Done(Word, bool) }}");
    emit_fns(&mut out, module)?;
    let _ = writeln!(
        out,
        "fn run(h: &mut runtime::Heap, host: &mut dyn runtime::AotNatives) -> Result<(Word, usize), String> {{"
    );
    emit_run_body(&mut out, module)?;
    let _ = writeln!(out, "}}");
    let _ = writeln!(out, "fn main() {{");
    let _ = writeln!(out, "    use std::io::Write;");
    let _ = writeln!(out, "    let mut h = runtime::Heap::new();");
    let _ = writeln!(out, "    let mut host = runtime::AotHost::new();");
    let _ = writeln!(out, "    let r = run(&mut h, &mut host);");
    let _ = writeln!(
        out,
        "    let _ = std::io::stdout().write_all(&host.take_stdout());"
    );
    let _ = writeln!(out, "    match r {{");
    let _ = writeln!(
        out,
        "        Ok((v, live)) => println!(\"OK {{}} {{}}\", v, live),"
    );
    let _ = writeln!(out, "        Err(e) => println!(\"ERR {{}}\", e),");
    let _ = writeln!(out, "    }}");
    let _ = writeln!(out, "}}");
    Ok(out)
}

pub fn transpile_module_lib(module: &Module) -> Result<String, TranspileError> {
    if embed::is_effectful(module) {
        hybrid::transpile_module_lib(module)
    } else {
        embed::transpile_module_lib(module)
    }
}

pub fn transpile_batch(modules: &[&Module]) -> Result<String, TranspileError> {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "#![allow(unused_mut, unused_variables, dead_code, unused_assignments, unused_parens)]"
    );
    for (i, m) in modules.iter().enumerate() {
        let _ = writeln!(out, "mod p{} {{", i);
        TargetWidth::for_module(m).aliases(&mut out);
        emit_fns(&mut out, m)?;
        let _ = writeln!(
            out,
            "    pub fn run(h: &mut runtime::Heap, host: &mut runtime::AotHost) -> Result<(Word, usize), String> {{"
        );
        emit_run_body(&mut out, m)?;
        let _ = writeln!(out, "    }}");
        let _ = writeln!(out, "}}");
    }
    let _ = writeln!(out, "fn main() {{");
    for i in 0..modules.len() {
        let width = TargetWidth::for_module(modules[i]);
        let namespace = width.namespace();
        let _ = writeln!(
            out,
            "    {{ let mut h = {namespace}::Heap::new(); let mut host = {namespace}::AotHost::new();"
        );
        let _ = writeln!(
            out,
            "      match p{}::run(&mut h, &mut host) {{ Ok((v, live)) => println!(\"{} OK {{}} {{}}\", v, live), Err(e) => println!(\"{} ERR {{}}\", e) }} }}",
            i, i, i
        );
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
