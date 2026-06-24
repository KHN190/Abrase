use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;
use myriad::{Value, VirtualMachine};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

const CORE: &str = include_str!("fixtures/core_heap.abe");

fn build() -> abrase::bytecode::Module {
    let mut p = Parser::new(Lexer::new(CORE)).with_source(CORE.into());
    let ast = p.parse_program();
    assert!(p.errors.is_empty(), "parse errors: {:?}", p.errors);
    let mut c = Compiler::new().with_source(CORE.into()).with_lib(true);
    c.compile_module(&ast).unwrap_or_else(|e| {
        panic!("compile: {:?}", e.iter().map(|x| &x.message).collect::<Vec<_>>())
    })
}

// Fixed script run identically on interp and AOT; emits the same i64 list.
const ARENA: i64 = 1024;

fn interp_script() -> Vec<i64> {
    let m = build();
    let mut vm = VirtualMachine::new().with_core_arena(ARENA as usize);
    let mut out = Vec::new();
    let call = |vm: &mut VirtualMachine, name: &str, a: &[i64]| -> i64 {
        let args: Vec<Value> = a.iter().map(|x| Value::from_int(*x)).collect();
        vm.call_export(&m, name, &args).expect(name).as_int()
    };
    call(&mut vm, "core_init", &[ARENA]);
    let a = call(&mut vm, "alloc", &[2]);
    out.push(a);
    out.push(call(&mut vm, "rc_inc", &[a]));
    out.push(call(&mut vm, "cell_set", &[a, 0, 1234]));
    out.push(call(&mut vm, "cell_get", &[a, 0]));
    out.push(call(&mut vm, "rc_dec", &[a]));
    out.push(call(&mut vm, "rc_dec", &[a]));
    let b = call(&mut vm, "alloc", &[2]);
    out.push(b);
    out.push(call(&mut vm, "rc_inc", &[a]));
    out.push(call(&mut vm, "cell_get", &[b, 0]));
    out.push(call(&mut vm, "rc_inc", &[0]));
    out
}

const DRIVER: &str = r#"
fn main() {
    let mut z = vec![0u8; 1024];
    let a = z.as_mut_slice();
    let mut out: Vec<i64> = Vec::new();
    core_init(a, 1024).unwrap();
    let h = alloc(a, 2).unwrap(); out.push(h as i64);
    out.push(rc_inc(a, h).unwrap() as i64);
    out.push(cell_set(a, h, 0, 1234).unwrap() as i64);
    out.push(cell_get(a, h, 0).unwrap() as i64);
    out.push(rc_dec(a, h).unwrap() as i64);
    out.push(rc_dec(a, h).unwrap() as i64);
    let b = alloc(a, 2).unwrap(); out.push(b as i64);
    out.push(rc_inc(a, h).unwrap() as i64);
    out.push(cell_get(a, b, 0).unwrap() as i64);
    out.push(rc_inc(a, 0).unwrap() as i64);
    let s: Vec<String> = out.iter().map(|x| x.to_string()).collect();
    println!("{}", s.join(" "));
}
"#;

static SEQ: AtomicU64 = AtomicU64::new(0);

fn aot_script() -> Vec<i64> {
    let m = build();
    let src = polka_rustc::core::transpile_core(&m).expect("transpile_core");
    let full = format!("{}\n{}", src, DRIVER);
    let id = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("core_aot_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let src_path = dir.join(format!("core_{}.rs", id));
    let bin_path = dir.join(format!("core_{}.bin", id));
    std::fs::write(&src_path, &full).unwrap();
    let status = Command::new("rustc")
        .args(["--edition", "2021", "-A", "warnings"])
        .arg(&src_path).arg("-o").arg(&bin_path)
        .status().expect("rustc");
    assert!(status.success(), "rustc failed on:\n{}", full);
    let out = Command::new(&bin_path).output().expect("run");
    let s = String::from_utf8(out.stdout).unwrap();
    s.trim().split_whitespace().map(|x| x.parse().unwrap()).collect()
}

#[test]
fn core_aot_matches_interpreter() {
    let interp = interp_script();
    let aot = aot_script();
    assert_eq!(aot, interp, "AOT core output must match interpreter");
}
