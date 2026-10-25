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

pub(crate) fn inline_native_math(name: &str, i32s: bool) -> Option<String> {
    use crate::{fdec, fenc};
    let a0 = fdec("a[0]", i32s);
    let a1 = fdec("a[1]", i32s);
    let unary = |f: &str| fenc(&format!("runtime::fmath::{}({})", f, a0), i32s);
    let binary = |f: &str| fenc(&format!("runtime::fmath::{}({}, {})", f, a0, a1), i32s);
    Some(match name {
        "sqrt" => unary("sqrt"),
        "sin" => unary("sin"),
        "cos" => unary("cos"),
        "flr" => unary("floor"),
        "ceil" => unary("ceil"),
        "__float_abs" => unary("abs"),
        "__float_max" => binary("fmax"),
        "__float_min" => binary("fmin"),
        "__int_abs" => "(a[0] as Int).wrapping_abs() as Word".to_string(),
        "__int_max" => "(a[0] as Int).max(a[1] as Int) as Word".to_string(),
        "__int_min" => "(a[0] as Int).min(a[1] as Int) as Word".to_string(),
        "__int_to_f" => fenc("(a[0] as Int) as Float", i32s),
        "__float_to_i" => format!("({} as Int) as Word", a0),
        _ => return None,
    })
}

pub(crate) fn inline_native_str(name: &str, i32s: bool) -> Option<String> {
    let a0f = crate::fdec("a[0]", i32s);
    Some(match name {
        "__int_to_s" => "let v = runtime::alloc_string(h, &(a[0] as Int).to_string())?; Ok((v.raw(), true))".to_string(),
        "__float_to_s" => return Some(format!("let v = runtime::alloc_string(h, &({}).to_string())?; Ok((v.raw(), true))", a0f)),
        "__bool_to_s" => "let v = runtime::alloc_string(h, &(a[0] != 0).to_string())?; Ok((v.raw(), true))".to_string(),
        "__unit_to_s" => "let v = runtime::alloc_string(h, \"()\")?; Ok((v.raw(), true))".to_string(),
        "__char_to_s" => "let c = char::from_u32(a[0] as u32).ok_or_else(|| \"__char_to_s: bad codepoint\".to_string())?; let v = runtime::alloc_string(h, &c.to_string())?; Ok((v.raw(), true))".to_string(),
        "__str_len" => "Ok((runtime::read_string(h, runtime::Value::from_raw(a[0])).map(|s| s.len()).unwrap_or(0) as Word, false))".to_string(),
        "__bytes_len" => "Ok((runtime::read_bytes(h, runtime::Value::from_raw(a[0])).map(|b| b.len()).unwrap_or(0) as Word, false))".to_string(),
        "__str_byte_at" => "let i = a[1] as Int; let s = runtime::read_string(h, runtime::Value::from_raw(a[0])).unwrap_or_default(); let by = s.as_bytes(); if i < 0 || i as usize >= by.len() { return Err(format!(\"byte_at: index {} out of bounds for length {}\", i, by.len())); } Ok((by[i as usize] as Word, false))".to_string(),
        "__bytes_byte_at" => "let i = a[1] as Int; let b = runtime::read_bytes(h, runtime::Value::from_raw(a[0])).unwrap_or_default(); if i < 0 || i as usize >= b.len() { return Err(format!(\"byte_at: index {} out of bounds for length {}\", i, b.len())); } Ok((b[i as usize] as Word, false))".to_string(),
        "__str_eq" => "let x = runtime::read_string(h, runtime::Value::from_raw(a[0])); let y = runtime::read_string(h, runtime::Value::from_raw(a[1])); Ok((if x == y { 1 } else { 0 }, false))".to_string(),
        "__str_to_bytes" => "let b = runtime::read_bytes(h, runtime::Value::from_raw(a[0])).unwrap_or_default(); let v = runtime::alloc_bytes(h, &b)?; Ok((v.raw(), true))".to_string(),
        "__str_slice" => "let s = runtime::read_string(h, runtime::Value::from_raw(a[0])).unwrap_or_default(); let by = s.as_bytes(); let off = a[1] as Int; let len = a[2] as Int; let start = off.max(0).min(by.len() as Int) as usize; let take = len.max(0) as usize; let end = start.saturating_add(take).min(by.len()); let out = String::from_utf8_lossy(&by[start..end]); let v = runtime::alloc_string(h, &out)?; Ok((v.raw(), true))".to_string(),
        "__bytes_slice" => "let b = runtime::read_bytes(h, runtime::Value::from_raw(a[0])).unwrap_or_default(); let off = a[1] as Int; let len = a[2] as Int; let start = off.max(0).min(b.len() as Int) as usize; let take = len.max(0) as usize; let end = start.saturating_add(take).min(b.len()); let v = runtime::alloc_bytes(h, &b[start..end])?; Ok((v.raw(), true))".to_string(),
        _ => return None,
    })
}

pub(crate) fn emit_native_fn(out: &mut String, idx: usize, name: &str, i32s: bool) {
    if let Some(expr) = inline_native_math(name, i32s) {
        let _ = writeln!(
            out,
            "fn f{}(_h: &mut runtime::Heap, _host: &mut dyn runtime::AotNatives, _rt: &mut runtime::RegionTable, _cs: &[Vec<Word>], _mt: &mut (Word, bool), a: &[Word], _ah: &[bool]) -> Result<(Word, bool), String> {{ Ok(({}, false)) }}",
            idx, expr
        );
    } else if let Some(body) = inline_native_str(name, i32s) {
        let _ = writeln!(
            out,
            "fn f{}(h: &mut runtime::Heap, _host: &mut dyn runtime::AotNatives, _rt: &mut runtime::RegionTable, _cs: &[Vec<Word>], _mt: &mut (Word, bool), a: &[Word], _ah: &[bool]) -> Result<(Word, bool), String> {{ {} }}",
            idx, body
        );
    } else {
        let _ = writeln!(
            out,
            "fn f{}(h: &mut runtime::Heap, host: &mut dyn runtime::AotNatives, _rt: &mut runtime::RegionTable, _cs: &[Vec<Word>], _mt: &mut (Word, bool), a: &[Word], _ah: &[bool]) -> Result<(Word, bool), String> {{ let args: Vec<runtime::Value> = a.iter().map(|&x| runtime::Value::from_raw(x)).collect(); host.call({:?}, h, &args) }}",
            idx, name
        );
    }
}

fn math_native(module: &Module, id: usize) -> bool {
    matches!(&module.functions[id], Chunk::Native(n) if inline_native_math(&n.name, false).is_some())
}

fn str_native(module: &Module, id: usize) -> bool {
    matches!(&module.functions[id], Chunk::Native(n) if inline_native_str(&n.name, false).is_some())
}

fn inlinable_native(module: &Module, id: usize) -> bool {
    math_native(module, id) || str_native(module, id)
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
                let bad = b.code.iter().any(|op| matches!(op, OpCode::Call(_, t) if !ok[*t as usize] && !inlinable_native(module, *t as usize)));
                if bad { ok[i] = false; changed = true; }
            }
        }
        if !changed { break; }
    }
    ok
}

pub(crate) fn transpile_module(module: &Module) -> Result<String, TranspileError> {
    emit(module, false)
}

pub(crate) fn transpile_module_lib(module: &Module) -> Result<String, TranspileError> {
    emit(module, true)
}

fn emit(module: &Module, lib: bool) -> Result<String, TranspileError> {
    let bridge = bridgeable_set(module);
    if !bridge.iter().any(|&b| b) {
        return if lib {
            crate::embed::transpile_module_lib(module)
        } else {
            crate::embed::transpile_module(module)
        };
    }

    let mut emb = clone_module(module);
    for (i, b) in bridge.iter().enumerate() {
        if *b {
            let pc = module.functions[i].param_count();
            emb.functions[i] = Chunk::Native(NativeChunk {
                name: format!("__aot_f{}", i),
                param_count: pc,
            });
        }
    }
    let pk = polka::cartridge::write_pk(&emb)
        .map_err(|e| TranspileError::Unsupported(format!("write_pk: {:?}", e)))?;

    let param_counts: Vec<usize> = module.functions.iter().map(|c| c.param_count()).collect();
    let is_native: Vec<bool> = module
        .functions
        .iter()
        .map(|c| matches!(c, Chunk::Native(_)))
        .collect();
    let int32_safe = (module.flags & polka::CART_FLAG_INT32_SAFE) != 0;

    let mut out = String::new();
    let _ = writeln!(
        out,
        "#![allow(unused_mut, unused_variables, dead_code, unused_assignments, unused_parens)]"
    );
    crate::TargetWidth::for_module(module).aliases(&mut out);
    let _ = writeln!(out, "use std::rc::Rc;");
    let _ = writeln!(out, "struct NoHost;");
    let _ = writeln!(
        out,
        "impl runtime::AotNatives for NoHost {{ fn call(&mut self, n: &str, _h: &mut runtime::Heap, _a: &[runtime::Value]) -> Result<(Word, bool), String> {{ Err(format!(\"aot pure fn called host {{}}\", n)) }} }}"
    );

    for (i, b) in bridge.iter().enumerate() {
        if *b {
            if let Chunk::Bytecode(bc) = &module.functions[i] {
                crate::emit_pure_fn(&mut out, i, bc, &param_counts, &is_native, int32_safe)?;
            }
        }
    }

    let mut native_emit = vec![false; module.functions.len()];
    for (i, b) in bridge.iter().enumerate() {
        if !*b {
            continue;
        }
        if let Chunk::Bytecode(bc) = &module.functions[i] {
            for op in &bc.code {
                if let OpCode::Call(_, t) = op {
                    if inlinable_native(module, *t as usize) {
                        native_emit[*t as usize] = true;
                    }
                }
            }
        }
    }
    for (i, e) in native_emit.iter().enumerate() {
        if !*e {
            continue;
        }
        if let Chunk::Native(n) = &module.functions[i] {
            emit_native_fn(&mut out, i, &n.name, int32_safe);
        }
    }

    let _ = writeln!(
        out,
        "fn __cs() -> Vec<Vec<Word>> {{ let mut cs: Vec<Vec<Word>> = Vec::new();"
    );
    let width = crate::TargetWidth::for_module(module);
    for c in module.functions.iter() {
        let _ = write!(out, "    {{ let mut v: Vec<Word> = Vec::new();");
        if let Chunk::Bytecode(b) = c {
            for (index, &constant) in b.constants.iter().enumerate() {
                let literal = if b.const_is_handle(index as u16) {
                    "0 as Word".into()
                } else {
                    width.constant(constant)?
                };
                let _ = write!(out, " v.push({});", literal);
            }
        }
        let _ = writeln!(out, " cs.push(v); }}");
    }
    let _ = writeln!(out, "    cs }}");

    let _ = writeln!(
        out,
        "{}fn register_aot(vm: &mut runtime::VirtualMachine) {{",
        if lib { "pub " } else { "" }
    );
    let _ = writeln!(out, "    let cs = __cs();");
    for (i, b) in bridge.iter().enumerate() {
        if *b {
            let _ = writeln!(
                out,
                "    {{ let cs = cs.clone(); vm.register_aot_fn(\"__aot_f{}\", Rc::new(move |ctx: &mut runtime::NativeCtx, args: &[runtime::Value], tags: &[bool]| {{",
                i
            );
            let _ = writeln!(
                out,
                "        let a: Vec<Word> = args.iter().map(|v| v.raw()).collect(); let ah: Vec<bool> = tags.to_vec();"
            );
            let _ = writeln!(
                out,
                "        let mut nohost = NoHost; let mut rt = runtime::RegionTable::new(); let mut mt = (Word::MAX, false);"
            );
            let _ = writeln!(
                out,
                "        let (rv, rvh) = f{}(ctx.heap, &mut nohost, &mut rt, &cs, &mut mt, &a, &ah)?;",
                i
            );
            let _ = writeln!(
                out,
                "        Ok((runtime::Value::from_raw(rv), rvh)) }})); }}"
            );
        }
    }
    let _ = writeln!(out, "}}");

    let _ = write!(out, "{}const PK: &[u8] = &[", if lib { "pub " } else { "" });
    for byte in &pk {
        let _ = write!(out, "{},", byte);
    }
    let _ = writeln!(out, "];");
    if lib {
        return Ok(out);
    }
    let _ = writeln!(out, "fn main() {{");
    let _ = writeln!(out, "    use std::io::Write;");
    let _ = writeln!(
        out,
        "    let module = runtime::read_pk(PK).expect(\"read_pk\");"
    );
    let _ = writeln!(
        out,
        "    let console = runtime::devices::BufferConsole::new();"
    );
    let _ = writeln!(out, "    let (cart_out, _) = console.handles();");
    let _ = writeln!(out, "    let mut vm = runtime::VirtualMachine::new();");
    let _ = writeln!(
        out,
        "    runtime::Host::default().with_console(Box::new(console)).install_into(&mut vm);"
    );
    let _ = writeln!(out, "    register_aot(&mut vm);");
    let _ = writeln!(out, "    let r = vm.run_module(&module);");
    let _ = writeln!(out, "    let live = vm.heap_live_count();");
    let _ = writeln!(
        out,
        "    let _ = std::io::stdout().write_all(&cart_out.borrow());"
    );
    let _ = writeln!(
        out,
        "    match r {{ Ok(v) => println!(\"OK {{}} {{}}\", v.raw(), live), Err(e) => println!(\"ERR {{}}\", e) }}"
    );
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
