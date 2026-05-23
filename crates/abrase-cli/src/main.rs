use std::env;
use std::fs;
use std::process::ExitCode;

use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;
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
";

fn main() -> ExitCode {
    let raw: Vec<String> = env::args().collect();
    let mut debug = false;
    let mut int32 = false;
    let mut args: Vec<String> = Vec::with_capacity(raw.len());
    for a in raw {
        match a.as_str() {
            "--debug" => debug = true,
            "--int32" => int32 = true,
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
        return cmd_export(path, &args[3], int32);
    }

    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ect: cannot read {}: {}", path, e);
            return ExitCode::from(66);
        }
    };

    match cmd {
        "run" => cmd_run(&source, debug, int32),
        "check" => cmd_check(&source, int32),
        "parse" => cmd_parse(&source),
        "disasm" => cmd_disasm(&source, int32),
        _ => {
            eprint!("{}", USAGE);
            ExitCode::from(64)
        }
    }
}

fn parse(source: &str) -> Result<Vec<abrase::ast::Decl>, ExitCode> {
    let mut parser = Parser::new(Lexer::new(source)).with_source(source.to_string());
    let ast = parser.parse_program();
    if !parser.errors.is_empty() {
        eprint!("{}", parser.pretty_print_errors());
        return Err(ExitCode::from(1));
    }
    Ok(ast)
}

fn cmd_run(source: &str, debug: bool, int32: bool) -> ExitCode {
    if debug {
        eprintln!("# debug fmt:");
        eprintln!("#   compile-time: [lower] [COMPILE] [CALL] [emit_handle_install] [FUNC_MAP] [BYTECODE]");
        eprintln!("#   runtime trace: [<fn_name>#<fn_id>:<pc>] <op>");
        eprintln!("#   handler events: [handle] push ... / [resume] -> ...");
    }
    let ast = match parse(source) { Ok(a) => a, Err(c) => return c };

    let mut compiler = Compiler::new()
        .with_source(source.to_string())
        .with_debug(debug)
        .with_int32_mode(int32);
    let module = match compiler.compile_module(&ast) {
        Ok(m) => m,
        Err(_) => {
            eprint!("{}", compiler.pretty_print_errors());
            return ExitCode::from(1);
        }
    };
    let fn_names = compiler.fn_names();

    let mut vm = VirtualMachine::new()
        .with_debug(debug)
        .with_fn_names(fn_names);

    Host::default().install_into(&mut vm);
    match vm.run_module(&module) {
        Ok(v) => {
            print_result(&vm, v);
            if debug {
                eprintln!("[heap] live_count after exit: {}", vm.heap_live_count());
            }
            ExitCode::SUCCESS
        }
        Err(e) => { eprintln!("runtime error: {}", e); ExitCode::from(2) }
    }
}

fn print_result(vm: &VirtualMachine, v: Value) {
    if !v.is_handle_none() {
        if let Some(s) = read_string(vm.heap_ref(), v) {
            println!("{}", s);
            return;
        }
    }
    println!("{}", v.as_int());
}

fn cmd_check(source: &str, int32: bool) -> ExitCode {
    let ast = match parse(source) { Ok(a) => a, Err(c) => return c };
    let mut checker = Checker::new();
    checker.check_program(&ast);
    if !checker.errors.is_empty() {
        eprint!("{}", checker.pretty_print_errors(source));
        return ExitCode::from(1);
    }
    if int32 {
        let mut compiler = Compiler::new()
            .with_source(source.to_string())
            .with_int32_mode(true);
        if compiler.compile_module(&ast).is_err() {
            eprint!("{}", compiler.pretty_print_errors());
            return ExitCode::from(1);
        }
    }
    println!("ok");
    ExitCode::SUCCESS
}

fn cmd_parse(source: &str) -> ExitCode {
    let mut parser = Parser::new(Lexer::new(source)).with_source(source.to_string());
    let ast = parser.parse_program();
    for d in &ast {
        println!("{:#?}", d);
    }
    if !parser.errors.is_empty() {
        eprint!("{}", parser.pretty_print_errors());
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

fn cmd_disasm(source: &str, int32: bool) -> ExitCode {
    let ast = match parse(source) { Ok(a) => a, Err(c) => return c };
    let mut compiler = Compiler::new()
        .with_source(source.to_string())
        .with_int32_mode(int32);
    let module = match compiler.compile_module(&ast) {
        Ok(m) => m,
        Err(_) => {
            eprint!("{}", compiler.pretty_print_errors());
            return ExitCode::from(1);
        }
    };
    for (i, chunk) in module.functions.iter().enumerate() {
        let marker = if i == module.entry { " <entry>" } else { "" };
        match chunk {
            abrase::bytecode::Chunk::Bytecode(bc) => {
                println!("fn #{}{} (regs={}, consts={})", i, marker, bc.reg_count, bc.constants.len());
                for (j, c) in bc.constants.iter().enumerate() {
                    println!("  const[{}] = {:?}", j, c);
                }
                for (pc, op) in bc.code.iter().enumerate() {
                    println!("  {:>4}: {:?}", pc, op);
                }
            }
            abrase::bytecode::Chunk::Native(n) => {
                println!("fn #{}{} <native, params={}>", i, marker, n.param_count);
            }
        }
    }
    ExitCode::SUCCESS
}

fn cmd_export(src_path: &str, out_path: &str, int32: bool) -> ExitCode {
    let source = match fs::read_to_string(src_path) {
        Ok(s) => s,
        Err(e) => { eprintln!("ect: cannot read {}: {}", src_path, e); return ExitCode::from(66); }
    };
    let ast = match parse(&source) { Ok(a) => a, Err(c) => return c };
    let mut compiler = Compiler::new()
        .with_source(source.clone())
        .with_int32_mode(int32);
    let module = match compiler.compile_module(&ast) {
        Ok(m) => m,
        Err(_) => { eprint!("{}", compiler.pretty_print_errors()); return ExitCode::from(1); }
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
