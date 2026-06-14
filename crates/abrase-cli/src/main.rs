use std::env;
use std::fs;
use std::process::ExitCode;

use std::path::Path;

use abrase::compiler::Compiler;
use abrase::error::{Error, ErrorCode};
use abrase::lexer::Lexer;
use abrase::loader;
use abrase::parser::Parser;
use abrase::typeck::Checker;
use myriad::{Value, VirtualMachine, read_string};
use abrase_cli::host::{install_std_devices, myriad_stderr_sink, eprintln_sink};

const USAGE: &str = "\
Abrase compiler & Myriad Runtime

usage:
    abrase run     <file.abe>  [flags]   compile and execute main()
    abrase check   <file.abe>            type-check only; no execution
    abrase disasm  <file.abe>  [flags]   compile and dump bytecode
    abrase transpile <file.abe> [flags]  compile and emit standalone Rust (trusted-AOT) to stdout
                                         --lib: emit host-injectable items (pub PK + register_aot), no main
    abrase explain <file.abe>            AST → typeck → bytecode chain
    abrase explain --expr '<snippet>'    same for inline code (auto-wrapped)
    abrase export  <file.abe> <out.pk>   compile and write a .pk cartridge
    abrase load    <file.pk>  [flags]    load and execute a .pk cartridge
    abrase --version | -V                print version and exit

debug flags (run / load):
    --trace      one line per opcode executed  →  [fn#id:pc] op  (stderr)
    --handlers   handler push / resume / pop events              (stderr)
    --leak       dump every live heap cell after exit            (stderr)
    --debug      alias for --trace --handlers
    BREAK_AT=<fn>:<pc> | <file.abe>:<line>  env var: dump register window there (host-side breakpoint)
    TRACE_FN=<fn>[,<fn>]  env var: limit --trace/--debug to these functions

compile flags (run / disasm / export):
    --root <dir> set module root (import paths resolve from here; default: entry file's dir)
    --int32      reject literals outside i32/f32 range; sets INT32_SAFE header
    --no-builtin skip native imports (print, math, conversions, string ops)

env:
    ABRASE_CODEGEN_DEBUG=1   emit compile-time codegen logs (compiler dev use)
";

fn main() -> ExitCode {
    let raw: Vec<String> = env::args().collect();
    let mut trace = false;
    let mut handlers = false;
    let mut int32 = false;
    let mut lib = false;
    let mut no_built_in = false;
    let mut leak = false;
    let mut show_version = false;
    let mut inline_expr: Option<String> = None;
    let mut root_override: Option<std::path::PathBuf> = None;
    let codegen_debug = std::env::var("ABRASE_CODEGEN_DEBUG").map(|v| v == "1").unwrap_or(false);
    let mut args: Vec<String> = Vec::with_capacity(raw.len());
    let mut raw_iter = raw.into_iter();
    while let Some(a) = raw_iter.next() {
        match a.as_str() {
            "--trace"        => trace = true,
            "--handlers"     => handlers = true,
            "--debug"        => { trace = true; handlers = true; }
            "--trace-frames" => handlers = true,
            "--int32"        => int32 = true,
            "--lib"          => lib = true,
            "--no-built-in" | "--no-builtin" => no_built_in = true,
            "--leak"         => leak = true,
            "--version" | "-V" => show_version = true,
            "--expr"         => { inline_expr = raw_iter.next(); }
            "--root"         => { root_override = raw_iter.next().map(std::path::PathBuf::from); }
            _ => args.push(a),
        }
    }

    if show_version {
        println!("abrase {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }

    if args.len() < 2 {
        eprint!("{}", USAGE);
        return ExitCode::from(64);
    }
    let cmd = args[1].as_str();

    if cmd == "explain" {
        if let Some(snippet) = inline_expr {
            return cmd_explain_inline(&snippet);
        }
        if args.len() < 3 {
            eprint!("{}", USAGE);
            return ExitCode::from(64);
        }
        let program = match loader::load_program(Path::new(&args[2])) {
            Ok(p) => p,
            Err(e) => { eprintln!("{}", e); return ExitCode::from(66); }
        };
        return cmd_explain_program(&program);
    }

    if args.len() < 3 {
        eprint!("{}", USAGE);
        return ExitCode::from(64);
    }
    let path = &args[2];

    if cmd == "load" {
        return cmd_load(path, trace, handlers, leak);
    }
    if cmd == "export" {
        if args.len() < 4 {
            eprint!("{}", USAGE);
            return ExitCode::from(64);
        }
        return cmd_export(path, &args[3], int32, no_built_in);
    }

    let program = match loader::load_program_with_root(Path::new(path), root_override.as_deref()) {
        Ok(p) => p,
        Err(e) => { eprintln!("{}", e); return ExitCode::from(66); }
    };

    match cmd {
        "run" => cmd_run(&program, trace, handlers, codegen_debug, int32, no_built_in, leak),
        "check" => cmd_check(&program, int32, no_built_in),
        "parse" => cmd_parse(&program),
        "disasm" => cmd_disasm(&program, int32, no_built_in),
        "transpile" => cmd_transpile(&program, int32, no_built_in, lib),
        _ => {
            eprint!("{}", USAGE);
            ExitCode::from(64)
        }
    }
}

fn print_warnings(program: &loader::LoadedProgram, warnings: &[abrase::lint::Lint]) {
    for w in warnings {
        let src = program.module_sources.get(&w.module)
            .map(|(_, s)| s.as_str())
            .unwrap_or(&program.entry_source);
        eprint!("{}", w.pretty_print(src));
    }
}

fn cmd_run(program: &loader::LoadedProgram, trace: bool, handlers: bool, codegen_debug: bool, int32: bool, no_built_in: bool, leak: bool) -> ExitCode {
    let ast = &program.decls;
    let source = &program.entry_source;

    let mut compiler = Compiler::new()
        .with_source(source.clone())
        .with_debug(codegen_debug)
        .with_int32_mode(int32)
        .with_drop_elision(!std::env::var("ABRASE_NO_ELISION").is_ok())
        .with_inline(!std::env::var("ABRASE_NO_INLINE").is_ok())
        .with_copy_coalesce(!std::env::var("ABRASE_NO_COALESCE").is_ok())
        .with_copy_prop(!std::env::var("ABRASE_NO_COPY_PROP").is_ok())
        .with_typed_ld(!std::env::var("ABRASE_NO_TYPED_LD").is_ok())
        .with_tail_resume(!std::env::var("ABRASE_NO_TAIL_RESUME").is_ok())
        .with_no_built_in(no_built_in);
    let mut module = match compiler.compile_module(ast) {
        Ok(m) => m,
        Err(errs) => {
            eprint!("{}", program.render_errors(&errs));
            return ExitCode::from(1);
        }
    };
    attach_src_files(&mut module, &compiler.fn_names(), &fn_origins(program));
    if !compiler.warnings.is_empty() {
        print_warnings(program, &compiler.warnings);
    }
    let fn_names = compiler.fn_names();
    let static_names = compiler.static_names_by_offset();

    let mut vm = apply_vm_diagnostics(
        VirtualMachine::new()
            .with_trace_frames(handlers)
            .with_fn_names(fn_names)
            .with_static_names(static_names),
        trace,
    );
    if let Ok(spec) = std::env::var("BREAK_AT") {
        vm = vm.with_debug_sink(break_at_sink(spec));
    }
    if let Ok(spec) = std::env::var("TRACE_FN") {
        let names = compiler.fn_names();
        let mut bits = vec![false; names.len()];
        for want in spec.split(',') {
            match names.iter().position(|n| n == want.trim()) {
                Some(i) => bits[i] = true,
                None => eprintln!("TRACE_FN: unknown fn '{}'", want.trim()),
            }
        }
        vm = vm.with_trace_filter(bits);
    }

    install_std_devices(&mut vm);

    let result = if is_cart_main(ast) {
        run_cart(&mut vm, &module)
    } else {
        let main_returns_unit = main_returns_unit(ast);
        vm.run_module(&module).map(|v| {
            if !main_returns_unit { print_result(&vm, v); }
            ()
        })
    };
    eprint!("{}", vm.profile_report());
    match result {
        Ok(()) => {
            if let Some(code) = vm.exit_code() {
                if trace { eprintln!("[heap] live={}", vm.heap_live_count()); }
                if leak { eprint!("{}", vm.live_slots_report()); }
                return ExitCode::from(code as u8);
            }
            if trace { eprintln!("[heap] live={}", vm.heap_live_count()); }
            if leak { eprint!("{}", vm.live_slots_report()); }
            ExitCode::SUCCESS
        }
        Err(e) => { eprintln!("runtime error: {}", e); ExitCode::from(2) }
    }
}

fn run_cart(vm: &mut VirtualMachine, module: &polka::Module) -> Result<(), String> {
    vm.run_to_yield(module)?;
    loop {
        let still_running = vm.resume(module, myriad::Value::from_int(0))?;
        if !still_running { break; }
    }
    Ok(())
}

fn is_cart_main(ast: &[abrase::ast::Decl]) -> bool {
    use abrase::ast::Decl;
    ast.iter().any(|d| {
        if let Decl::Fn(fd) = d {
            fd.name == "main" && fd.attrs.iter().any(|a| a.name == "cart")
        } else { false }
    })
}

fn main_returns_unit(ast: &[abrase::ast::Decl]) -> bool {
    use abrase::ast::{Decl, Type};
    for d in ast {
        if let Decl::Fn(fd) = d {
            if fd.name == "main" {
                return match &fd.return_type {
                    None => true,
                    Some(Type::Named(n)) if n == "Unit" => true,
                    Some(Type::Tuple(t)) if t.is_empty() => true,
                    _ => false,
                };
            }
        }
    }
    false
}

fn print_result(vm: &VirtualMachine, v: Value) {
    if vm.last_result_is_handle() && !v.is_handle_none() {
        if let Some(s) = read_string(vm.heap_ref(), v) {
            println!("{}", s);
            return;
        }
    }
    println!("{}", v.as_int());
}

fn cmd_check(program: &loader::LoadedProgram, int32: bool, no_built_in: bool) -> ExitCode {
    let ast = &program.decls;
    let source = &program.entry_source;
    let mut compiler = Compiler::new()
        .with_source(source.clone())
        .with_int32_mode(int32)
        .with_no_built_in(no_built_in);
    compiler.run_typeck_only(ast);
    if !compiler.warnings.is_empty() {
        print_warnings(program, &compiler.warnings);
    }
    if !compiler.errors.is_empty() {
        eprint!("{}", program.render_errors(&compiler.errors));
        return ExitCode::from(1);
    }
    println!("ok");
    ExitCode::SUCCESS
}

fn cmd_parse(program: &loader::LoadedProgram) -> ExitCode {
    if program.sources.len() > 1 {
        println!("// merged AST from {} files:", program.sources.len());
        for (path, _) in &program.sources {
            println!("//   {}", path.display());
        }
        println!();
    }
    for d in &program.decls {
        println!("{:#?}", d);
    }
    ExitCode::SUCCESS
}

fn cmd_explain_program(program: &loader::LoadedProgram) -> ExitCode {
    explain_chain(&program.decls, &program.entry_source, program)
}

fn cmd_explain_inline(snippet: &str) -> ExitCode {
    let src = if looks_like_decls(snippet) {
        snippet.to_string()
    } else {
        format!("fn main() -> Int {{\n{}\n}}\n", snippet)
    };
    let mut p = Parser::new(Lexer::new(&src)).with_source(src.clone());
    let ast = p.parse_program();
    if !p.errors.is_empty() {
        eprintln!("parse errors:\n{}", p.pretty_print_errors());
        return ExitCode::from(1);
    }
    explain_chain_raw(&ast, &src)
}

fn looks_like_decls(s: &str) -> bool {
    let trimmed = s.trim_start();
    trimmed.starts_with("fn ") || trimmed.starts_with("type ") || trimmed.starts_with("static ")
        || trimmed.starts_with("pub ") || trimmed.starts_with("effect ")
        || trimmed.starts_with("use ")
}

fn explain_chain(
    ast: &[abrase::ast::Decl],
    source: &str,
    program: &loader::LoadedProgram,
) -> ExitCode {
    println!("=== parsed AST ===");
    for d in ast {
        println!("{:#?}", d);
    }

    println!("\n=== typeck ===");
    let mut checker = Checker::new();
    checker.check_program(ast);
    if checker.errors.is_empty() {
        println!("ok  ({} expr types recorded)", checker.expr_types.len());
    } else {
        let errs: Vec<Error> = checker.errors.iter()
            .map(|te| Error::new(ErrorCode::TypeError, te.span, te.message.clone())
                .with_module(te.module.clone()))
            .collect();
        print!("{}", program.render_errors(&errs));
    }

    println!("\n=== bytecode ===");
    let origins = fn_origins(program);
    let mut compiler = Compiler::new().with_source(source.to_string())
        .with_drop_elision(!std::env::var("ABRASE_NO_ELISION").is_ok())
        .with_inline(!std::env::var("ABRASE_NO_INLINE").is_ok())
        .with_copy_coalesce(!std::env::var("ABRASE_NO_COALESCE").is_ok())
        .with_copy_prop(!std::env::var("ABRASE_NO_COPY_PROP").is_ok())
        .with_typed_ld(!std::env::var("ABRASE_NO_TYPED_LD").is_ok())
        .with_tail_resume(!std::env::var("ABRASE_NO_TAIL_RESUME").is_ok());
    let module = match compiler.compile_module(ast) {
        Ok(m) => m,
        Err(errs) => {
            eprint!("{}", program.render_errors(&errs));
            return ExitCode::from(1);
        }
    };
    print_bytecode(&module, &compiler.fn_names(), &compiler.static_names_by_offset(), &origins);
    ExitCode::SUCCESS
}

fn explain_chain_raw(ast: &[abrase::ast::Decl], source: &str) -> ExitCode {
    println!("=== parsed AST ===");
    for d in ast {
        println!("{:#?}", d);
    }

    println!("\n=== typeck ===");
    let mut checker = Checker::new();
    checker.check_program(ast);
    if checker.errors.is_empty() {
        println!("ok  ({} expr types recorded)", checker.expr_types.len());
    } else {
        for e in &checker.errors {
            eprintln!("  type error: {}", e.message);
        }
    }

    println!("\n=== bytecode ===");
    let mut compiler = Compiler::new().with_source(source.to_string()).with_no_built_in(true);
    let module = match compiler.compile_module(ast) {
        Ok(m) => m,
        Err(_) => {
            eprintln!("{}", compiler.pretty_print_errors());
            return ExitCode::from(1);
        }
    };
    let empty_origins = std::collections::HashMap::new();
    print_bytecode(&module, &compiler.fn_names(), &compiler.static_names_by_offset(), &empty_origins);
    ExitCode::SUCCESS
}

fn print_bytecode(
    module: &abrase::bytecode::Module,
    names: &[String],
    static_by_offset: &[String],
    origins: &std::collections::HashMap<String, String>,
) {
    for (i, chunk) in module.functions.iter().enumerate() {
        let entry_marker = if i == module.entry { " <entry>" } else { "" };
        let name = names.get(i).cloned().unwrap_or_default();
        let origin = origins.get(&name).map(|s| format!(" <from: {}>", s)).unwrap_or_default();
        let is_module_init = name == "__module_init";
        match chunk {
            abrase::bytecode::Chunk::Bytecode(bc) => {
                println!("fn #{} {}{}{} (regs={}, consts={})",
                    i, name, entry_marker, origin, bc.reg_count, bc.constants.len());
                for (j, c) in bc.constants.iter().enumerate() {
                    println!("  const[{}] = {:?}", j, c);
                }
                for (pc, op) in bc.code.iter().enumerate() {
                    let ann = if is_module_init {
                        static_init_annotation(op, static_by_offset)
                    } else {
                        let mut a = call_annotation(op, names);
                        if a.is_empty() { a = device_annotation(bc, pc); }
                        a
                    };
                    if ann.is_empty() {
                        println!("  {:>4}: {:?}", pc, op);
                    } else {
                        println!("  {:>4}: {:<50}  ; {}", pc, format!("{:?}", op), ann);
                    }
                }
            }
            abrase::bytecode::Chunk::Native(n) => {
                println!("fn #{} {}{}{} <native, params={}>",
                    i, name, entry_marker, origin, n.param_count);
            }
        }
    }
    if !static_by_offset.is_empty() {
        println!("\nstatic table (offset -> name):");
        for (offset, name) in static_by_offset.iter().enumerate() {
            if !name.is_empty() {
                println!("  [{}] {}", offset, name);
            }
        }
    }
}

fn cmd_disasm(program: &loader::LoadedProgram, int32: bool, no_built_in: bool) -> ExitCode {
    let ast = &program.decls;
    let source = &program.entry_source;
    let origins = fn_origins(program);
    let mut compiler = Compiler::new()
        .with_source(source.clone())
        .with_int32_mode(int32)
        .with_no_built_in(no_built_in);
    let module = match compiler.compile_module(ast) {
        Ok(m) => m,
        Err(errs) => {
            eprint!("{}", program.render_errors(&errs));
            return ExitCode::from(1);
        }
    };
    print_bytecode(&module, &compiler.fn_names(), &compiler.static_names_by_offset(), &origins);
    ExitCode::SUCCESS
}

fn cmd_transpile(program: &loader::LoadedProgram, int32: bool, no_built_in: bool, lib: bool) -> ExitCode {
    let ast = &program.decls;
    let source = &program.entry_source;
    let mut compiler = Compiler::new()
        .with_source(source.clone())
        .with_int32_mode(int32)
        .with_no_built_in(no_built_in);
    let module = match compiler.compile_module(ast) {
        Ok(m) => m,
        Err(errs) => {
            eprint!("{}", program.render_errors(&errs));
            return ExitCode::from(1);
        }
    };
    let result = if lib { polka_rustc::transpile_module_lib(&module) }
                 else { polka_rustc::transpile_module(&module) };
    match result {
        Ok(rust) => { print!("{}", rust); ExitCode::SUCCESS }
        Err(e) => { eprintln!("transpile error: {}", e); ExitCode::from(1) }
    }
}

// Annotate Deo/Dei against the device + port encoded in the port register's
// most recent PushConst. port_val = (device_id << 8) | port.
fn device_annotation(bc: &abrase::bytecode::BytecodeChunk, pc: usize) -> String {
    use abrase::bytecode::OpCode;
    let port_reg = match &bc.code[pc] {
        OpCode::Deo(_, p) | OpCode::Dei(p, _) => *p,
        _ => return String::new(),
    };
    let mut val: Option<i64> = None;
    for prev in bc.code[..pc].iter().rev() {
        if let OpCode::PushConst(d, idx) = prev {
            if *d == port_reg {
                val = bc.constants.get(*idx as usize).map(|c| *c as i64);
                break;
            }
        }
    }
    let Some(v) = val else { return String::new(); };
    let (id, port) = (((v >> 8) & 0xFF) as u8, (v & 0xFF) as u8);
    use abrase::bytecode::*;
    let label = match (id, port) {
        (REGION_ID, REGION_PORT_PUSH)   => "region push",
        (REGION_ID, REGION_PORT_POP)    => "region pop",
        (REGION_ID, REGION_PORT_FORGET) => "region forget",
        (DISPATCH_ID, DISPATCH_PORT_LOOKUP)      => "effect: handler lookup",
        (DISPATCH_ID, DISPATCH_PORT_POP_HANDLER) => "effect: pop handler",
        (DISPATCH_ID, DISPATCH_PORT_ENV)         => "effect: env",
        (DISPATCH_ID, DISPATCH_PORT_RETURN_FN)   => "effect: return-arm fn",
        (DISPATCH_ID, DISPATCH_PORT_RETURN_ENV)  => "effect: return-arm env",
        (MODULE_ID, MODULE_PORT_TABLE)           => "module table",
        _ => return String::new(),
    };
    label.to_string()
}

fn call_annotation(op: &abrase::bytecode::OpCode, names: &[String]) -> String {
    use abrase::bytecode::OpCode;
    match op {
        OpCode::Call(_, fid) => {
            let n = names.get(*fid as usize).cloned().unwrap_or_default();
            let via = if n.starts_with("__closure_") { " (closure body / arm)" }
                      else if n.starts_with("__fnval_") { " (fn-value adapter)" }
                      else { "" };
            format!("-> {}{}", n, via)
        }
        OpCode::CallReg(..) => "-> dynamic via reg (closure / fn-value)".into(),
        _ => String::new(),
    }
}

fn static_init_annotation(op: &abrase::bytecode::OpCode, static_by_offset: &[String]) -> String {
    use abrase::bytecode::OpCode;
    match op {
        OpCode::St(_, _, off) => {
            let idx = *off as usize;
            if let Some(name) = static_by_offset.get(idx) {
                if !name.is_empty() {
                    return format!("static[{}] = {}", idx, name);
                }
            }
            String::new()
        }
        OpCode::Ld(_, _, off) => {
            let idx = *off as usize;
            if let Some(name) = static_by_offset.get(idx) {
                if !name.is_empty() {
                    return format!("static[{}] => {}", idx, name);
                }
            }
            String::new()
        }
        _ => String::new(),
    }
}

fn fn_origins(program: &loader::LoadedProgram) -> std::collections::HashMap<String, String> {
    use abrase::ast::Decl;
    let entry_label = program.sources.last()
        .and_then(|(p, _)| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("<entry>")
        .to_string();
    let mut out = std::collections::HashMap::new();
    let mut stack: Vec<String> = Vec::new();
    for d in &program.decls {
        match d {
            Decl::ModEnter(path) => stack.push(path.join(".")),
            Decl::ModExit => { stack.pop(); }
            Decl::Fn(f) => {
                let label = stack.last().cloned().unwrap_or_else(|| entry_label.clone());
                out.insert(f.name.clone(), label);
            }
            _ => {}
        }
    }
    out
}

fn cmd_export(src_path: &str, out_path: &str, int32: bool, no_built_in: bool) -> ExitCode {
    let program = match loader::load_program(Path::new(src_path)) {
        Ok(p) => p,
        Err(e) => { eprintln!("{}", e); return ExitCode::from(66); }
    };
    let ast = &program.decls;
    let source = &program.entry_source;
    let mut compiler = Compiler::new()
        .with_source(source.clone())
        .with_int32_mode(int32)
        .with_no_built_in(no_built_in);
    let mut module = match compiler.compile_module(ast) {
        Ok(m) => m,
        Err(errs) => { eprint!("{}", program.render_errors(&errs)); return ExitCode::from(1); }
    };
    attach_src_files(&mut module, &compiler.fn_names(), &fn_origins(&program));
    let bytes = match polka::cartridge::write_pk(&module) {
        Ok(b) => b,
        Err(e) => { eprintln!("export: {}", e); return ExitCode::from(2); }
    };
    if let Err(e) = fs::write(out_path, &bytes) {
        eprintln!("export: cannot write {}: {}", out_path, e);
        return ExitCode::from(74);
    }
    eprintln!("wrote {} bytes to {}", bytes.len(), out_path);
    ExitCode::SUCCESS
}

fn cmd_load(pk_path: &str, trace: bool, handlers: bool, leak: bool) -> ExitCode {
    let bytes = match fs::read(pk_path) {
        Ok(b) => b,
        Err(e) => { eprintln!("ect: cannot read {}: {}", pk_path, e); return ExitCode::from(66); }
    };
    let module = match polka::cartridge::read_pk(&bytes) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("load {}: {}", pk_path, e);
            if matches!(e, polka::cartridge::LoadError::NotACartridge) && pk_path.ends_with(".abe") {
                eprintln!("hint: `{}` looks like Abrase source; try `abrase run {}` instead", pk_path, pk_path);
            }
            return ExitCode::from(65);
        }
    };
    let mut vm = apply_vm_diagnostics(
        VirtualMachine::new().with_trace_frames(handlers),
        trace,
    );
    if let Ok(spec) = std::env::var("BREAK_AT") {
        vm = vm.with_debug_sink(break_at_sink(spec));
    }
    install_std_devices(&mut vm);
    match vm.run_module(&module) {
        Ok(v) => {
            print_result(&vm, v);
            if leak { eprint!("{}", vm.live_slots_report()); }
            ExitCode::SUCCESS
        }
        Err(e) => { eprintln!("runtime error: {}", e); ExitCode::from(2) }
    }
}

// BREAK_AT=<fn-name|#id>:<pc> — on the matching trace event, dump the fn's
// register window with handle annotations. The VM only emits events; all
// breakpoint logic lives host-side.
// BREAK_AT=<fn|#id>:<pc> or <file.abe>:<line>.
// Read diagnostic env vars (TRACE_SLOT/TRACE_STATIC/PROFILE/ABRASE_HEAP_CHECK)
// and wire the matching VM sinks. Env reading lives here, not in no_std myriad.
fn apply_vm_diagnostics(mut vm: VirtualMachine, trace: bool) -> VirtualMachine {
    if trace {
        vm = vm.with_debug_sink(myriad_stderr_sink());
    }
    match std::env::var("TRACE_SLOT") {
        Ok(s) if s == "*" => vm = vm.with_heap_trace(None, true, eprintln_sink),
        Ok(s) => if let Ok(n) = s.parse::<u32>() {
            vm = vm.with_heap_trace(Some(n), false, eprintln_sink);
        },
        Err(_) => {}
    }
    if let Ok(s) = std::env::var("TRACE_STATIC") {
        if !s.is_empty() { vm = vm.with_trace_static(Some(s)); }
    }
    if std::env::var("PROFILE").is_ok() { vm = vm.with_profile(true); }
    if std::env::var("ABRASE_HEAP_CHECK").is_ok() { vm = vm.with_heap_check(true); }
    vm.with_trace_out(eprintln_sink)
}

fn break_at_sink(spec: String) -> myriad::DebugSink {
    let (fn_part, pos_part) = match spec.rsplit_once(':') {
        Some(p) => p,
        None => {
            eprintln!("BREAK_AT: expected <fn>:<pc> or <file>:<line>, got '{}'", spec);
            return Box::new(|_, _| {});
        }
    };
    let want_pos: usize = pos_part.parse().unwrap_or(usize::MAX);
    let file_mode = fn_part.contains('.');
    let fn_part = fn_part.to_string();
    Box::new(move |event, names| {
        if let myriad::DebugEvent::Trace { func, pc, op, base_reg, window, handle_mask, line, file } = event {
            let matched = if file_mode {
                *line as usize == want_pos && *file == fn_part
            } else {
                *pc == want_pos
                    && fn_part.strip_prefix('#')
                        .map(|id| id.parse::<usize>() == Ok(*func))
                        .unwrap_or_else(|| names.get(*func).map_or(false, |n| n == &fn_part))
            };
            if !matched { return; }
            eprintln!("[break {}:{}] {:?} (base r{})",
                myriad::render_fn_label(*func, names), pc, op, base_reg);
            for (i, raw) in window.iter().enumerate() {
                let is_h = i < 128 && (handle_mask & (1u128 << i)) != 0;
                if *raw == polka::HANDLE_NONE && !is_h { continue; }
                eprintln!("    r{:<3} = {:#018x}{}", i, raw, if is_h { "  (handle)" } else { "" });
            }
        }
    })
}

// Fill BytecodeChunk.src_file from the loader's fn→file map (compiler stays
// filesystem-agnostic; file knowledge lives host-side).
fn attach_src_files(module: &mut polka::Module, names: &[String],
                    origins: &std::collections::HashMap<String, String>) {
    for (i, chunk) in module.functions.iter_mut().enumerate() {
        if let polka::Chunk::Bytecode(b) = chunk {
            if let Some(f) = names.get(i).and_then(|n| origins.get(n)) {
                b.src_file = f.clone();
            }
        }
    }
}
