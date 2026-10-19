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

// ---- relooper CFG-pattern differential: interp vs AOT, diverse control flow ----

fn compile_lib(src: &str) -> abrase::bytecode::Module {
    let mut p = Parser::new(Lexer::new(src)).with_source(src.into());
    let ast = p.parse_program();
    assert!(p.errors.is_empty(), "parse: {:?}", p.errors);
    let mut c = Compiler::new().with_source(src.into()).with_lib(true);
    c.compile_module(&ast).unwrap_or_else(|e| panic!("compile: {:?}", e.iter().map(|x| &x.message).collect::<Vec<_>>()))
}

fn aot_call(src: &str, fname: &str, arg: i64) -> i64 {
    let m = compile_lib(src);
    let body = polka_rustc::core::transpile_core(&m).expect("transpile_core");
    let driver = format!("fn main() {{ let mut z = vec![0u8; 64]; let r = {}(z.as_mut_slice(), ({}i64) as u64).unwrap(); println!(\"{{}}\", r as i64); }}", fname, arg);
    let full = format!("{}\n{}", body, driver);
    let id = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("core_cfg_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let sp = dir.join(format!("c_{}.rs", id));
    let bp = dir.join(format!("c_{}.bin", id));
    std::fs::write(&sp, &full).unwrap();
    let st = Command::new("rustc").args(["--edition", "2021", "-A", "warnings"]).arg(&sp).arg("-o").arg(&bp).output().expect("rustc");
    assert!(st.status.success(), "rustc failed: {}\n{}", String::from_utf8_lossy(&st.stderr), full);
    let o = Command::new(&bp).output().expect("run");
    String::from_utf8(o.stdout).unwrap().trim().parse().unwrap()
}

fn diff(src: &str, fname: &str, args: &[i64]) {
    let m = compile_lib(src);
    for &a in args {
        let mut vm = VirtualMachine::new().with_core_arena(64);
        let interp = vm.call_export(&m, fname, &[Value::from_int(a)]).expect(fname).as_int();
        let aot = aot_call(src, fname, a);
        assert_eq!(aot, interp, "fn `{}` arg {}: AOT {} != interp {}", fname, a, aot, interp);
    }
}

#[test]
fn cfg_or_short_circuit() {
    let src = "pub fn f(n: Int) -> <core> Int { if n < 0 || n > 100 { 0 } else { 1 } }\nfn main() -> Unit { () }\n";
    diff(src, "f", &[-5, 0, 50, 100, 101]);
}

#[test]
fn cfg_and_short_circuit() {
    let src = "pub fn f(n: Int) -> <core> Int { if n > 0 && n < 100 { 1 } else { 0 } }\nfn main() -> Unit { () }\n";
    diff(src, "f", &[-1, 0, 1, 99, 100]);
}

#[test]
fn cfg_nested_if_in_while() {
    let src = "pub fn f(n: Int) -> <core> Int {\n\
                   let mut acc = 0;\n\
                   let mut i = 0;\n\
                   while i < n { if (i & 1) == 0 { acc = acc + i; } else { acc = acc + 1; }; i = i + 1; }\n\
                   acc\n\
               }\nfn main() -> Unit { () }\n";
    diff(src, "f", &[0, 1, 5, 10]);
}

#[test]
fn cfg_nested_loops() {
    let src = "pub fn f(n: Int) -> <core> Int {\n\
                   let mut acc = 0;\n\
                   let mut i = 0;\n\
                   while i < n { let mut j = 0; while j < i { acc = acc + 1; j = j + 1; } i = i + 1; }\n\
                   acc\n\
               }\nfn main() -> Unit { () }\n";
    diff(src, "f", &[0, 1, 4, 8]);
}

#[test]
fn cfg_if_else_if_chain() {
    let src = "pub fn f(n: Int) -> <core> Int {\n\
                   if n < 0 { 0 } else { if n < 10 { 1 } else { if n < 100 { 2 } else { 3 } } }\n\
               }\nfn main() -> Unit { () }\n";
    diff(src, "f", &[-1, 0, 9, 10, 99, 100]);
}

#[test]
fn cfg_early_return_in_loop() {
    let src = "pub fn f(n: Int) -> <core> Int {\n\
                   let mut i = 0;\n\
                   let mut r = 0 - 1;\n\
                   while i < n { if i * i > n { r = i; i = n; } else { i = i + 1; } }\n\
                   r\n\
               }\nfn main() -> Unit { () }\n";
    diff(src, "f", &[0, 4, 10, 50]);
}


// ---- host-driven AOT differential for String/Bytes natives (slice, byte_at,
//      to_bytes): transpile lib + real myriad host, diff value AND heap-live
//      against the interpreter. Confirms the new read-only natives survive AOT.

fn aot_rustflags() -> Vec<String> {
    if let Ok(enc) = std::env::var("CARGO_ENCODED_RUSTFLAGS") {
        if !enc.is_empty() { return enc.split('\u{1f}').map(|s| s.to_string()).collect(); }
    }
    std::env::var("RUSTFLAGS").ok().filter(|s| !s.is_empty())
        .map(|s| s.split_whitespace().map(|t| t.to_string()).collect())
        .unwrap_or_default()
}

fn aot_deps_dir() -> String {
    std::env::current_exe().ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .map(|d| d.to_string_lossy().into_owned())
        .expect("deps dir")
}

fn aot_myriad_rlib() -> String {
    let deps = aot_deps_dir();
    let mut rlibs: Vec<std::path::PathBuf> = std::fs::read_dir(&deps).expect("deps")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.file_name().map(|n| {
            let n = n.to_string_lossy();
            n.starts_with("libmyriad-") && n.ends_with(".rlib")
        }).unwrap_or(false))
        .collect();
    rlibs.sort_by_key(|p| std::cmp::Reverse(p.metadata().and_then(|m| m.modified()).ok()));
    rlibs.first().expect("myriad rlib").display().to_string()
}

// Returns (value, live) from the transpiled+linked binary. Uses the Native
// (non-lib) transpile so str/bytes natives emit inline Rust (not host.call),
// exercising the inline bodies end-to-end. emit_native prints `OK <v> <live>`.
fn aot_host_run(src: &str) -> (i64, usize) {
    let m = compile_lib(src);
    let driver = polka_rustc::transpile_module(&m).expect("transpile module");
    let id = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("aot_hostdiff_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let sp = dir.join(format!("h_{}.rs", id));
    let bp = dir.join(format!("h_{}.bin", id));
    std::fs::write(&sp, &driver).unwrap();
    let st = Command::new("rustc")
        .args(["--edition", "2021", "-A", "warnings"])
        .args(aot_rustflags())
        .arg("--extern").arg(format!("myriad={}", aot_myriad_rlib()))
        .arg("-L").arg(aot_deps_dir())
        .arg(&sp).arg("-o").arg(&bp)
        .output().expect("rustc");
    assert!(st.status.success(), "rustc failed: {}\n{}", String::from_utf8_lossy(&st.stderr), driver);
    let o = Command::new(&bp).output().expect("run");
    let s = String::from_utf8(o.stdout).unwrap();
    let line = s.trim().lines().last().unwrap_or("");
    let mut it = line.split_whitespace();
    assert_eq!(it.next(), Some("OK"), "AOT run did not print OK: {}", line);
    let v: i64 = it.next().expect("val").parse().unwrap();
    let live: usize = it.next().expect("live").parse().unwrap();
    (v, live)
}

fn aot_host_diff(src: &str) {
    let m = compile_lib(src);
    let mut vm = VirtualMachine::new();
    let iv = vm.run_module(&m).expect("interp").raw() as i64;
    let il = vm.heap_live_count();
    let (av, al) = aot_host_run(src);
    assert_eq!(av, iv, "AOT value {} != interp {} for `{}`", av, iv, src);
    assert_eq!(al, il, "AOT live {} != interp {} for `{}`", al, il, src);
}

#[test]
fn aot_str_bytes_natives_match_interpreter() {
    // slice + len + byte_at + to_bytes + chained read-only, all in one module.
    let src = "fn main() -> Int {\n\
        let a = \"hello\".slice(1, 3).len();\n\
        let b = \"world\".to_bytes().len();\n\
        let c = \"abc\".to_bytes().slice(0, 2).byte_at(1);\n\
        let d = \"xy\".byte_at(0);\n\
        a + b + c + d\n\
    }\nfn main2() -> Unit { () }\n";
    aot_host_diff(src);
}

// The str/bytes natives inline (emit direct Rust, no host.call) on the hybrid
// transpile path — reached when the module is effectful and the str-using fn is
// bridgeable (params, no handle constants). Non-effectful modules embed+interpret
// by design, so this targets the hybrid path to prove the inline whitelist fires.
#[test]
fn aot_str_bytes_natives_inline_on_hybrid_path() {
    // Effectful module (Handle/Raise) routes to the hybrid transpiler; `slen`
    // takes a String param with no handle constants, so it is bridgeable and its
    // `__str_len` call inlines as direct Rust rather than host.call.
    let src = r#"
        effect provider { op give() -> Int }
        fn slen(s: String) -> Int { s.len() }
        fn produce() -> <provider> Int { provider.give() + slen("hi") }
        fn main() -> Int {
            handle produce() {
                return v => v,
                provider.give => resume(3)
            }
        }
    "#;
    // Disable the abrase source inliner so `slen` stays a distinct fn (else it
    // folds into `produce`, which carries the "hi" handle constant and is not
    // bridgeable).
    let mut p = Parser::new(Lexer::new(src)).with_source(src.into());
    let ast = p.parse_program();
    assert!(p.errors.is_empty(), "parse: {:?}", p.errors);
    let mut c = Compiler::new().with_source(src.into()).with_lib(true).with_inline(false);
    let m = c.compile_module(&ast).unwrap_or_else(|e| panic!("compile: {:?}", e.iter().map(|x| &x.message).collect::<Vec<_>>()));
    let lib = polka_rustc::transpile_module_lib(&m).expect("transpile");
    assert!(lib.contains("myriad::read_string"),
        "bridgeable str fn must inline `__str_len` as direct Rust, not host.call:\n{}", lib);
}
