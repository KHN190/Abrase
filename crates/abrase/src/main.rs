use std::env;
use std::fs;
use std::process::ExitCode;

use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;
use abrase::typeck::Checker;
use abrase::myriad::{BoxedValue, Value, VirtualMachine};

const USAGE: &str = "\
Abrase compiler & Myriad VM

usage:
    abrase run    <file.abe>    parse, compile, execute main()
    abrase check  <file.abe>    parse and type-check; no execution
    abrase parse  <file.abe>    dump AST and parser errors
    abrase disasm <file.abe>    parse, compile, dump bytecode
";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprint!("{}", USAGE);
        return ExitCode::from(64);
    }
    let cmd = args[1].as_str();
    let path = &args[2];

    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ect: cannot read {}: {}", path, e);
            return ExitCode::from(66);
        }
    };

    match cmd {
        "run" => cmd_run(&source),
        "check" => cmd_check(&source),
        "parse" => cmd_parse(&source),
        "disasm" => cmd_disasm(&source),
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

fn frontend(source: &str) -> Result<Vec<abrase::ast::Decl>, ExitCode> {
    let ast = parse(source)?;
    let mut checker = Checker::new();
    checker.check_program(&ast);
    if !checker.errors.is_empty() {
        eprint!("{}", checker.pretty_print_errors(source));
        return Err(ExitCode::from(1));
    }
    Ok(ast)
}

fn cmd_run(source: &str) -> ExitCode {
    let ast = match frontend(source) { Ok(a) => a, Err(c) => return c };

    let mut compiler = Compiler::new().with_source(source.to_string());
    let module = match compiler.compile_module(&ast) {
        Ok(m) => m,
        Err(_) => {
            eprint!("{}", compiler.pretty_print_errors());
            return ExitCode::from(1);
        }
    };

    let mut vm = VirtualMachine::new();
    match vm.run_module(&module) {
        Ok(v) => { print_result(&vm, v); ExitCode::SUCCESS }
        Err(e) => { eprintln!("runtime error: {}", e); ExitCode::from(2) }
    }
}

fn print_result(vm: &VirtualMachine, v: Value) {
    if let Some(idx) = v.as_box() {
        match vm.box_pool().get(idx) {
            Some(BoxedValue::String(s)) => { println!("{}", s); return; }
            Some(b) => { println!("{:?}", b); return; }
            None => {}
        }
    }
    println!("{:?}", v);
}

fn cmd_check(source: &str) -> ExitCode {
    let ast = match parse(source) { Ok(a) => a, Err(c) => return c };
    let mut checker = Checker::new();
    checker.check_program(&ast);
    if !checker.errors.is_empty() {
        eprint!("{}", checker.pretty_print_errors(source));
        return ExitCode::from(1);
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

fn cmd_disasm(source: &str) -> ExitCode {
    let ast = match frontend(source) { Ok(a) => a, Err(c) => return c };
    let mut compiler = Compiler::new().with_source(source.to_string());
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
