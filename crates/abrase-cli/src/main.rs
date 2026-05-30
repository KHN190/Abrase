use std::env;
use std::fs;
use std::process::ExitCode;

use std::path::Path;

use abrase::compiler::Compiler;
use abrase::error::{Error, ErrorCode};
use abrase::loader;
use abrase::typeck::Checker;
use myriad::{Host, Value, VirtualMachine, read_string};

const USAGE: &str = "\
Abrase compiler & Myriad Runtime

usage:
    abrase run    [--debug] <file.abe>    parse, compile, execute main()
    abrase check  <file.abe>               parse and type-check; no execution
    abrase parse  <file.abe>               dump AST and parser errors
    abrase disasm <file.abe>               parse, compile, dump bytecode
    abrase export <file.abe> <out.pk>      compile and write a .pk cartridge
    abrase load   <file.pk>                load a .pk cartridge and execute

flags:
    --debug    dump compile-time lowering/codegen prints and runtime
               instruction trace + handler events to stderr
    --int32    compile in 32-bit mode: reject Int literals outside i32 range
               and Float literals not representable as f32; sets cart header
               INT32_SAFE flag bit so 32-bit runtimes can opt into narrow
               storage. Bytecode encoding itself is unchanged.
    --no-built-in
               skip registering mandatory native imports (print, math,
               conversions, string ops, halt/abort). Any reference to them
               in source becomes an undefined-function error. Produces carts
               whose function table contains only user bytecode chunks, so a
               minimal runtime needs zero native implementations.
";

fn main() -> ExitCode {
    let raw: Vec<String> = env::args().collect();
    let mut debug = false;
    let mut trace_frames = false;
    let mut int32 = false;
    let mut no_built_in = false;
    let mut args: Vec<String> = Vec::with_capacity(raw.len());
    for a in raw {
        match a.as_str() {
            "--debug" => debug = true,
            "--trace-frames" => trace_frames = true,
            "--int32" => int32 = true,
            "--no-built-in" => no_built_in = true,
            _ => args.push(a),
        }
    }
    if args.len() < 3 {
        eprint!("{}", USAGE);
        return ExitCode::from(64);
    }
    let cmd = args[1].as_str();
    let path = &args[2];

    if cmd == "load" {
        return cmd_load(path, debug);
    }
    if cmd == "export" {
        if args.len() < 4 {
            eprint!("{}", USAGE);
            return ExitCode::from(64);
        }
        return cmd_export(path, &args[3], int32, no_built_in);
    }

    let program = match loader::load_program(Path::new(path)) {
        Ok(p) => p,
        Err(e) => { eprintln!("{}", e); return ExitCode::from(66); }
    };

    match cmd {
        "run" => cmd_run(&program, debug, trace_frames, int32, no_built_in),
        "check" => cmd_check(&program, int32, no_built_in),
        "parse" => cmd_parse(&program),
        "disasm" => cmd_disasm(&program, int32, no_built_in),
        _ => {
            eprint!("{}", USAGE);
            ExitCode::from(64)
        }
    }
}

fn cmd_run(program: &loader::LoadedProgram, debug: bool, trace_frames: bool, int32: bool, no_built_in: bool) -> ExitCode {
    if debug {
        eprintln!("# debug fmt:");
        eprintln!("#   compile-time: [lower] [COMPILE] [CALL] [emit_handle_install] [FUNC_MAP] [BYTECODE]");
        eprintln!("#   runtime trace: [<fn_name>#<fn_id>:<pc>] <op>");
        eprintln!("#   handler events: [handle] push ... / [resume] -> ...");
    }
    let ast = &program.decls;
    let source = &program.entry_source;

    let mut compiler = Compiler::new()
        .with_source(source.clone())
        .with_debug(debug)
        .with_int32_mode(int32)
        .with_no_built_in(no_built_in);
    let module = match compiler.compile_module(ast) {
        Ok(m) => m,
        Err(errs) => {
            eprint!("{}", program.render_errors(&errs));
            return ExitCode::from(1);
        }
    };
    let fn_names = compiler.fn_names();

    let mut vm = VirtualMachine::new()
        .with_debug(debug)
        .with_trace_frames(trace_frames)
        .with_fn_names(fn_names);

    Host::default().install_into(&mut vm);
    let main_returns_unit = main_returns_unit(ast);
    match vm.run_module(&module) {
        Ok(v) => {
            if !main_returns_unit { print_result(&vm, v); }
            if debug {
                eprintln!("[heap] live_count after exit: {}", vm.heap_live_count());
            }
            ExitCode::SUCCESS
        }
        Err(e) => { eprintln!("runtime error: {}", e); ExitCode::from(2) }
    }
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
    let mut checker = Checker::new();
    checker.check_program(ast);
    if !checker.errors.is_empty() {
        let errs: Vec<Error> = checker.errors.iter()
            .map(|te| Error::new(ErrorCode::TypeError, te.span, te.message.clone())
                .with_module(te.module.clone()))
            .collect();
        eprint!("{}", program.render_errors(&errs));
        return ExitCode::from(1);
    }
    if int32 || no_built_in {
        let mut compiler = Compiler::new()
            .with_source(source.clone())
            .with_int32_mode(int32)
            .with_no_built_in(no_built_in);
        if let Err(errs) = compiler.compile_module(ast) {
            eprint!("{}", program.render_errors(&errs));
            return ExitCode::from(1);
        }
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
    let names = compiler.fn_names();
    for (i, chunk) in module.functions.iter().enumerate() {
        let entry_marker = if i == module.entry { " <entry>" } else { "" };
        let name = names.get(i).cloned().unwrap_or_default();
        let origin = origins.get(&name).map(|s| format!(" <from: {}>", s)).unwrap_or_default();
        match chunk {
            abrase::bytecode::Chunk::Bytecode(bc) => {
                println!("fn #{} {}{}{} (regs={}, consts={})",
                    i, name, entry_marker, origin, bc.reg_count, bc.constants.len());
                for (j, c) in bc.constants.iter().enumerate() {
                    println!("  const[{}] = {:?}", j, c);
                }
                for (pc, op) in bc.code.iter().enumerate() {
                    println!("  {:>4}: {:?}", pc, op);
                }
            }
            abrase::bytecode::Chunk::Native(n) => {
                println!("fn #{} {}{}{} <native, params={}>",
                    i, name, entry_marker, origin, n.param_count);
            }
        }
    }
    ExitCode::SUCCESS
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
    let module = match compiler.compile_module(ast) {
        Ok(m) => m,
        Err(errs) => { eprint!("{}", program.render_errors(&errs)); return ExitCode::from(1); }
    };
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

fn cmd_load(pk_path: &str, debug: bool) -> ExitCode {
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
    let mut vm = VirtualMachine::new().with_debug(debug);
    Host::default().install_into(&mut vm);
    match vm.run_module(&module) {
        Ok(v) => { print_result(&vm, v); ExitCode::SUCCESS }
        Err(e) => { eprintln!("runtime error: {}", e); ExitCode::from(2) }
    }
}
