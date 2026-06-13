use polka::{Chunk, Module, NativeChunk, OpCode};
use std::fmt::Write;

use crate::TranspileError;

fn pure_op(op: &OpCode) -> bool {
    use OpCode::*;
    matches!(op,
        Add(..) | Sub(..) | Mul(..) | Div(..) | Mod(..) | Neg(..)
        | AddImm(..) | SubImm(..)
        | Lt(..) | Gt(..) | Lte(..) | Gte(..) | Eq(..) | Neq(..)
        | And(..) | Or(..) | Xor(..) | Shl(..) | Shr(..)
        | FAdd(..) | FSub(..) | FMul(..) | FDiv(..) | FLt(..) | FEq(..) | FNeg(..)
        | Move(..) | Copy(..) | PushConst(..)
        | Alloc(..) | Drop(..) | Ld(..) | St(..) | LdIdx(..) | StIdx(..)
        | Jmp(..) | Jz(..) | Jnz(..) | Call(..) | Ret(..))
}

fn inline_native_math(name: &str) -> Option<&'static str> {
    Some(match name {
        "sqrt" => "myriad::builtins::fmath::sqrt(f64::from_bits(a[0])).to_bits()",
        "sin" => "myriad::builtins::fmath::sin(f64::from_bits(a[0])).to_bits()",
        "cos" => "myriad::builtins::fmath::cos(f64::from_bits(a[0])).to_bits()",
        "flr" => "myriad::builtins::fmath::floor(f64::from_bits(a[0])).to_bits()",
        "ceil" => "myriad::builtins::fmath::ceil(f64::from_bits(a[0])).to_bits()",
        "__float_abs" => "myriad::builtins::fmath::abs(f64::from_bits(a[0])).to_bits()",
        "__float_max" => "myriad::builtins::fmath::fmax(f64::from_bits(a[0]), f64::from_bits(a[1])).to_bits()",
        "__float_min" => "myriad::builtins::fmath::fmin(f64::from_bits(a[0]), f64::from_bits(a[1])).to_bits()",
        "__int_abs" => "(a[0] as i64).wrapping_abs() as u64",
        "__int_max" => "(a[0] as i64).max(a[1] as i64) as u64",
        "__int_min" => "(a[0] as i64).min(a[1] as i64) as u64",
        "__int_to_f" => "((a[0] as i64) as f64).to_bits()",
        "__float_to_i" => "(f64::from_bits(a[0]) as i64) as u64",
        _ => return None,
    })
}

fn math_native(module: &Module, id: usize) -> bool {
    matches!(&module.functions[id], Chunk::Native(n) if inline_native_math(&n.name).is_some())
}

fn bridgeable_set(module: &Module) -> Vec<bool> {
    let mut called = vec![false; module.functions.len()];
    for c in &module.functions {
        if let Chunk::Bytecode(b) = c {
            for op in &b.code {
                if let OpCode::Call(_, t) = op { called[*t as usize] = true; }
            }
        }
    }
    let mut ok: Vec<bool> = module.functions.iter().enumerate().map(|(i, c)| {
        if !called[i] { return false; }
        let Chunk::Bytecode(b) = c else { return false };
        if (0..b.constants.len()).any(|i| b.const_is_handle(i as u16)) { return false; }
        b.code.iter().all(pure_op)
    }).collect();
    loop {
        let mut changed = false;
        for i in 0..module.functions.len() {
            if !ok[i] { continue; }
            if let Chunk::Bytecode(b) = &module.functions[i] {
                let bad = b.code.iter().any(|op| matches!(op, OpCode::Call(_, t) if !ok[*t as usize] && !math_native(module, *t as usize)));
                if bad { ok[i] = false; changed = true; }
            }
        }
        if !changed { break; }
    }
    ok
}

pub(crate) fn transpile_module(module: &Module) -> Result<String, TranspileError> {
    let bridge = bridgeable_set(module);
    if !bridge.iter().any(|&b| b) {
        return crate::embed::transpile_module(module);
    }

    let mut emb = clone_module(module);
    for (i, b) in bridge.iter().enumerate() {
        if *b {
            let pc = module.functions[i].param_count();
            emb.functions[i] = Chunk::Native(NativeChunk { name: format!("__aot_f{}", i), param_count: pc });
        }
    }
    let pk = polka::cartridge::write_pk(&emb)
        .map_err(|e| TranspileError::Unsupported(format!("write_pk: {:?}", e)))?;

    let param_counts: Vec<usize> = module.functions.iter().map(|c| c.param_count()).collect();
    let is_native: Vec<bool> = module.functions.iter().map(|c| matches!(c, Chunk::Native(_))).collect();
    let int32_safe = (module.flags & polka::CART_FLAG_INT32_SAFE) != 0;

    let mut out = String::new();
    let _ = writeln!(out, "#![allow(unused_mut, unused_variables, dead_code, unused_assignments, unused_parens)]");
    let _ = writeln!(out, "use std::rc::Rc;");
    let _ = writeln!(out, "struct NoHost;");
    let _ = writeln!(out, "impl myriad::AotNatives for NoHost {{ fn call(&mut self, n: &str, _h: &mut myriad::Heap, _a: &[myriad::Value]) -> Result<(u64, bool), String> {{ Err(format!(\"aot pure fn called host {{}}\", n)) }} }}");

    for (i, b) in bridge.iter().enumerate() {
        if *b {
            if let Chunk::Bytecode(bc) = &module.functions[i] {
                crate::emit_pure_fn(&mut out, i, bc, &param_counts, &is_native, int32_safe)?;
            }
        }
    }

    let mut math_emit = vec![false; module.functions.len()];
    for (i, b) in bridge.iter().enumerate() {
        if !*b { continue; }
        if let Chunk::Bytecode(bc) = &module.functions[i] {
            for op in &bc.code {
                if let OpCode::Call(_, t) = op {
                    if math_native(module, *t as usize) { math_emit[*t as usize] = true; }
                }
            }
        }
    }
    for (i, e) in math_emit.iter().enumerate() {
        if !*e { continue; }
        if let Chunk::Native(n) = &module.functions[i] {
            let expr = inline_native_math(&n.name).unwrap();
            let _ = writeln!(out, "fn f{}(_h: &mut myriad::Heap, _host: &mut dyn myriad::AotNatives, _rt: &mut myriad::RegionTable, _cs: &[Vec<u64>], _mt: &mut (u64, bool), a: &[u64], _ah: &[bool]) -> Result<(u64, bool), String> {{ Ok(({}, false)) }}", i, expr);
        }
    }

    let _ = writeln!(out, "fn __cs() -> Vec<Vec<u64>> {{ let mut cs: Vec<Vec<u64>> = Vec::new();");
    for c in module.functions.iter() {
        let _ = write!(out, "    {{ let mut v: Vec<u64> = Vec::new();");
        if let Chunk::Bytecode(b) = c {
            for cv in &b.constants { let _ = write!(out, " v.push({}u64);", cv); }
        }
        let _ = writeln!(out, " cs.push(v); }}");
    }
    let _ = writeln!(out, "    cs }}");

    let _ = writeln!(out, "fn register_aot(vm: &mut myriad::VirtualMachine) {{");
    let _ = writeln!(out, "    let cs = __cs();");
    for (i, b) in bridge.iter().enumerate() {
        if *b {
            let _ = writeln!(out, "    {{ let cs = cs.clone(); vm.register_aot_fn(\"__aot_f{}\", Rc::new(move |ctx: &mut myriad::NativeCtx, args: &[myriad::Value], tags: &[bool]| {{", i);
            let _ = writeln!(out, "        let a: Vec<u64> = args.iter().map(|v| v.raw()).collect(); let ah: Vec<bool> = tags.to_vec();");
            let _ = writeln!(out, "        let mut nohost = NoHost; let mut rt = myriad::RegionTable::new(); let mut mt = (u64::MAX, false);");
            let _ = writeln!(out, "        let (rv, rvh) = f{}(ctx.heap, &mut nohost, &mut rt, &cs, &mut mt, &a, &ah)?;", i);
            let _ = writeln!(out, "        Ok((myriad::Value::from_raw(rv), rvh)) }})); }}");
        }
    }
    let _ = writeln!(out, "}}");

    let _ = write!(out, "const PK: &[u8] = &[");
    for byte in &pk { let _ = write!(out, "{},", byte); }
    let _ = writeln!(out, "];");
    let _ = writeln!(out, "fn main() {{");
    let _ = writeln!(out, "    use std::io::Write;");
    let _ = writeln!(out, "    let module = myriad::read_pk(PK).expect(\"read_pk\");");
    let _ = writeln!(out, "    let console = myriad::devices::BufferConsole::new();");
    let _ = writeln!(out, "    let (cart_out, _) = console.handles();");
    let _ = writeln!(out, "    let mut vm = myriad::VirtualMachine::new().with_step_cap(1_000_000);");
    let _ = writeln!(out, "    myriad::Host::default().with_console(Box::new(console)).install_into(&mut vm);");
    let _ = writeln!(out, "    register_aot(&mut vm);");
    let _ = writeln!(out, "    let r = vm.run_module(&module);");
    let _ = writeln!(out, "    let live = vm.heap_live_count();");
    let _ = writeln!(out, "    let _ = std::io::stdout().write_all(&cart_out.borrow());");
    let _ = writeln!(out, "    match r {{ Ok(v) => println!(\"OK {{}} {{}}\", v.raw(), live), Err(e) => println!(\"ERR {{}}\", e) }}");
    let _ = writeln!(out, "}}");
    Ok(out)
}

fn clone_module(m: &Module) -> Module {
    Module {
        functions: m.functions.clone(),
        entry: m.entry,
        flags: m.flags,
        exports: m.exports.clone(),
    }
}
