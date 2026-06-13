// modules that use effects (Handle/Raise/Resume) can't lower to the
// native fast path yet, so emit a self-contained Rust program that embeds the
// `.pk` bytecode and runs it on the myriad VM (no_std-capable).
use polka::Module;
use std::fmt::Write;

use crate::TranspileError;

pub(crate) fn is_effectful(module: &Module) -> bool {
    module.functions.iter().any(|c| {
        c.as_bytecode().is_some_and(|b| b.code.iter().any(|op| {
            matches!(op, polka::OpCode::Handle(..) | polka::OpCode::Raise(..) | polka::OpCode::Resume(..))
        }))
    })
}

pub(crate) fn transpile_module(module: &Module) -> Result<String, TranspileError> {
    let pk = polka::cartridge::write_pk(module)
        .map_err(|e| TranspileError::Unsupported(format!("write_pk: {:?}", e)))?;
    let mut out = String::new();
    let _ = writeln!(out, "#![allow(dead_code)]");
    let _ = write!(out, "const PK: &[u8] = &[");
    for b in &pk { let _ = write!(out, "{},", b); }
    let _ = writeln!(out, "];");
    let _ = writeln!(out, "fn main() {{");
    let _ = writeln!(out, "    use std::io::Write;");
    let _ = writeln!(out, "    let module = myriad::read_pk(PK).expect(\"read_pk\");");
    let _ = writeln!(out, "    let console = myriad::devices::BufferConsole::new();");
    let _ = writeln!(out, "    let (cart_out, _) = console.handles();");
    let _ = writeln!(out, "    let mut vm = myriad::VirtualMachine::new().with_step_cap(1_000_000);");
    let _ = writeln!(out, "    myriad::Host::default().with_console(Box::new(console)).install_into(&mut vm);");
    let _ = writeln!(out, "    let r = vm.run_module(&module);");
    let _ = writeln!(out, "    let live = vm.heap_live_count();");
    let _ = writeln!(out, "    let _ = std::io::stdout().write_all(&cart_out.borrow());");
    let _ = writeln!(out, "    match r {{");
    let _ = writeln!(out, "        Ok(v) => println!(\"OK {{}} {{}}\", v.raw(), live),");
    let _ = writeln!(out, "        Err(e) => println!(\"ERR {{}}\", e),");
    let _ = writeln!(out, "    }}");
    let _ = writeln!(out, "}}");
    Ok(out)
}
