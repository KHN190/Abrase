#![allow(dead_code, unused_imports)]

use polka::{BytecodeChunk, Chunk, Module, OpCode, Register};
use polka_rustc::{transpile_module, transpile_module_lib, transpile_program, transpile_batch};
use myriad::VirtualMachine;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

pub fn r(n: u8) -> Register { Register(n) }

pub fn rustflags() -> Vec<String> {
    if let Ok(enc) = std::env::var("CARGO_ENCODED_RUSTFLAGS") {
        if !enc.is_empty() { return enc.split('\u{1f}').map(|s| s.to_string()).collect(); }
    }
    std::env::var("RUSTFLAGS").ok().filter(|s| !s.is_empty())
        .map(|s| s.split_whitespace().map(|t| t.to_string()).collect())
        .unwrap_or_default()
}

pub fn deps_dir() -> String {
    std::env::current_exe().ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .map(|d| d.to_string_lossy().into_owned())
        .unwrap_or_else(|| format!("{}/target/debug/deps", env!("CARGO_MANIFEST_DIR").trim_end_matches("/crates/polka-rustc")))
}

pub fn myriad_rlib() -> &'static str {
    static RLIB: OnceLock<String> = OnceLock::new();
    RLIB.get_or_init(|| {
        let deps = deps_dir();
        let dir = std::path::Path::new(&deps);
        let probe = std::env::temp_dir().join(format!("polka_probe_{}.rs", std::process::id()));
        std::fs::write(&probe, "fn main() { let _ = myriad::Heap::new(); let _ = myriad::AotHost::new(); }").unwrap();
        let mut rlibs: Vec<std::path::PathBuf> = std::fs::read_dir(dir).expect("deps dir")
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.file_name().map(|n| {
                let n = n.to_string_lossy();
                n.starts_with("libmyriad-") && n.ends_with(".rlib")
            }).unwrap_or(false))
            .collect();
        rlibs.sort_by_key(|p| std::cmp::Reverse(p.metadata().and_then(|m| m.modified()).ok()));
        for p in rlibs {
            let bin = std::env::temp_dir().join(format!("polka_probe_{}.bin", std::process::id()));
            let ok = Command::new("rustc")
                .args(["--edition", "2021"])
                .args(rustflags())
                .arg("--extern").arg(format!("myriad={}", p.display()))
                .arg("-L").arg(&deps)
                .arg(&probe).arg("-o").arg(&bin)
                .stderr(std::process::Stdio::null())
                .status().map(|s| s.success()).unwrap_or(false);
            if ok { return p.display().to_string(); }
        }
        panic!("no linkable myriad rlib in {}", deps);
    })
}

pub fn chunk(code: Vec<OpCode>, constants: Vec<u64>, reg_count: usize) -> BytecodeChunk {
    BytecodeChunk {
        code, constants,
        const_mask: Vec::new(),
        string_constants: Vec::new(),
        reg_count, param_count: 0,
        lines: Vec::new(),
        src_file: String::new(),
    }
}

pub fn fn_chunk(code: Vec<OpCode>, constants: Vec<u64>, reg_count: usize, param_count: usize) -> Chunk {
    Chunk::Bytecode(BytecodeChunk {
        code, constants,
        const_mask: Vec::new(),
        string_constants: Vec::new(),
        reg_count, param_count,
        lines: Vec::new(),
        src_file: String::new(),
    })
}

pub fn str_const_chunk(code: Vec<OpCode>, constants: Vec<u64>, const_mask: Vec<u64>,
                       strings: Vec<String>, reg_count: usize, param_count: usize) -> Chunk {
    Chunk::Bytecode(BytecodeChunk {
        code, constants, const_mask,
        string_constants: strings,
        reg_count, param_count,
        lines: Vec::new(), src_file: String::new(),
    })
}

#[derive(Debug, PartialEq)]
pub enum Outcome { Ok(u64), Err(String) }

pub fn interp(bc: &BytecodeChunk) -> Outcome {
    match VirtualMachine::new().run(&Chunk::Bytecode(bc.clone())) {
        Ok(v) => Outcome::Ok(v.raw()),
        Err(e) => Outcome::Err(e),
    }
}

pub fn run_module_outcome(module: &Module) -> Outcome {
    match VirtualMachine::new().with_step_cap(1_000_000).run_module(module) {
        Ok(v) => Outcome::Ok(v.raw()),
        Err(e) => Outcome::Err(e),
    }
}

static SEQ: AtomicU64 = AtomicU64::new(0);

fn rustc_build_run(src: &str) -> String {
    let id = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("polka_tp_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let src_path = dir.join(format!("prog_{}.rs", id));
    let bin_path = dir.join(format!("prog_{}.bin", id));
    std::fs::write(&src_path, src).unwrap();
    let deps = deps_dir();
    let status = Command::new("rustc")
        .args(["--edition", "2021"])
        .args(rustflags())
        .arg("--extern").arg(format!("myriad={}", myriad_rlib()))
        .arg("-L").arg(&deps)
        .arg(&src_path).arg("-o").arg(&bin_path)
        .status().expect("rustc");
    assert!(status.success(), "rustc failed on:\n{}", src);
    let out = Command::new(&bin_path).output().expect("run binary");
    String::from_utf8(out.stdout).unwrap()
}

fn parse_status(s: &str) -> (Outcome, usize) {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix("OK ") {
        let mut it = rest.split_whitespace();
        let v: u64 = it.next().expect("value").parse().expect("parse u64");
        let live: usize = it.next().expect("live").parse().expect("parse live");
        (Outcome::Ok(v), live)
    } else if let Some(rest) = s.strip_prefix("ERR ") {
        (Outcome::Err(rest.to_string()), 0)
    } else {
        panic!("unexpected program output: {:?}", s);
    }
}

pub fn compile_run_raw(src: &str) -> String { rustc_build_run(src) }

// Returns (outcome, live_cells). Program prints `OK <value> <live>` or `ERR <msg>`.
pub fn compile_run_full(src: &str) -> (Outcome, usize) {
    let full = rustc_build_run(src);
    parse_status(full.trim_end().lines().last().unwrap_or(""))
}

pub fn compile_run(src: &str) -> Outcome { compile_run_full(src).0 }

// Drive the `--lib` emit the way a host does: read_pk(PK), build a VM, install
// devices, register_aot, run_module. Mirrors the standalone main so the output
// contract (`OK <v> <live>` / `ERR`) is identical, letting it diff vs interp.
pub fn compile_run_lib_full(module: &Module) -> (Outcome, usize) {
    let lib = transpile_module_lib(module).expect("transpile lib");
    let driver = format!(
        "{lib}\nfn main() {{\n\
         use std::io::Write;\n\
         let module = myriad::read_pk(PK).expect(\"read_pk\");\n\
         let console = myriad::devices::BufferConsole::new();\n\
         let (cart_out, _) = console.handles();\n\
         let mut vm = myriad::VirtualMachine::new();\n\
         myriad::Host::default().with_console(Box::new(console)).install_into(&mut vm);\n\
         register_aot(&mut vm);\n\
         let r = vm.run_module(&module);\n\
         let live = vm.heap_live_count();\n\
         let _ = std::io::stdout().write_all(&cart_out.borrow());\n\
         match r {{ Ok(v) => println!(\"OK {{}} {{}}\", v.raw(), live), Err(e) => println!(\"ERR {{}}\", e) }}\n\
         }}\n",
        lib = lib);
    parse_status(rustc_build_run(&driver).trim_end().lines().last().unwrap_or(""))
}

pub fn compile_run_batch(modules: &[&polka::Module]) -> Vec<(Outcome, usize)> {
    let src = polka_rustc::transpile_batch(modules).expect("transpile batch");
    let full = rustc_build_run(&src);
    let mut results: Vec<Option<(Outcome, usize)>> = (0..modules.len()).map(|_| None).collect();
    for line in full.lines() {
        let line = line.trim();
        let Some((idx_str, rest)) = line.split_once(' ') else { continue };
        let Ok(i) = idx_str.parse::<usize>() else { continue };
        if i < results.len() && (rest.starts_with("OK ") || rest.starts_with("ERR ")) {
            results[i] = Some(parse_status(rest));
        }
    }
    results.into_iter().enumerate()
        .map(|(i, r)| r.unwrap_or_else(|| panic!("no batch result for program {}", i)))
        .collect()
}

pub fn transpiled(bc: &BytecodeChunk) -> Outcome {
    compile_run(&transpile_program(bc).expect("transpile"))
}

pub fn compare(i: &Outcome, t: &Outcome) {
    match (i, t) {
        (Outcome::Ok(a), Outcome::Ok(b)) => assert_eq!(a, b, "value mismatch"),
        (Outcome::Err(e), Outcome::Err(k)) =>
            assert!(e.contains(k.as_str()), "error category mismatch: interp={:?} transpiled={:?}", e, k),
        _ => panic!("outcome kind mismatch: interp={:?} transpiled={:?}", i, t),
    }
}

pub fn assert_same(code: Vec<OpCode>, constants: Vec<u64>, reg_count: usize) {
    let bc = chunk(code, constants, reg_count);
    compare(&interp(&bc), &transpiled(&bc));
}

pub fn assert_same_module(functions: Vec<Chunk>, entry: usize) {
    let module = Module { functions, entry, flags: 0, exports: vec![] };
    let i = run_module_outcome(&module);
    let t = compile_run(&transpile_module(&module).expect("transpile module"));
    compare(&i, &t);
}

pub fn assert_same_flags(code: Vec<OpCode>, constants: Vec<u64>, reg_count: usize, flags: u16) {
    let main = BytecodeChunk {
        code, constants, const_mask: Vec::new(), string_constants: Vec::new(),
        reg_count, param_count: 0, lines: Vec::new(), src_file: String::new(),
    };
    let module = Module { functions: vec![Chunk::Bytecode(main)], entry: 0, flags, exports: vec![] };
    let i = run_module_outcome(&module);
    let t = compile_run(&transpile_module(&module).expect("transpile"));
    compare(&i, &t);
}

pub fn assert_same_heap(functions: Vec<Chunk>, entry: usize) {
    let module = Module { functions, entry, flags: 0, exports: vec![] };
    let mut vm = VirtualMachine::new().with_step_cap(1_000_000);
    let i = match vm.run_module(&module) {
        Ok(v) => Outcome::Ok(v.raw()),
        Err(e) => Outcome::Err(e),
    };
    let i_live = vm.heap_live_count();
    let (t, t_live) = compile_run_full(&transpile_module(&module).expect("transpile heap"));
    compare(&i, &t);
    if let Outcome::Ok(_) = i {
        assert_eq!(i_live, t_live, "heap live-count mismatch: interp={} transpiled={}", i_live, t_live);
    }
}

pub fn batch_compare(mods: Vec<Module>, interps: Vec<(Outcome, usize)>, check_live: bool) {
    assert_eq!(mods.len(), interps.len());
    if mods.is_empty() { return; }
    let refs: Vec<&Module> = mods.iter().collect();
    let tps = compile_run_batch(&refs);
    for (i, ((io, il), (to, tl))) in interps.iter().zip(tps.iter()).enumerate() {
        match (io, to) {
            (Outcome::Ok(a), Outcome::Ok(b)) => assert_eq!(a, b, "value mismatch prog {}", i),
            (Outcome::Err(e), Outcome::Err(k)) =>
                assert!(e.contains(k.as_str()), "error mismatch prog {}: interp={:?} transpiled={:?}", i, e, k),
            _ => panic!("outcome kind mismatch prog {}: interp={:?} transpiled={:?}", i, io, to),
        }
        if check_live {
            if let Outcome::Ok(_) = io {
                assert_eq!(il, tl, "heap live-count mismatch prog {}: interp={} transpiled={}", i, il, tl);
            }
        }
    }
}

pub fn module_of(bc: BytecodeChunk) -> Module {
    Module { functions: vec![Chunk::Bytecode(bc)], entry: 0, flags: 0, exports: vec![] }
}

pub fn module_with_flags(bc: BytecodeChunk, flags: u16) -> Module {
    Module { functions: vec![Chunk::Bytecode(bc)], entry: 0, flags, exports: vec![] }
}

pub fn interp_with_live(module: &Module) -> (Outcome, usize) {
    let mut vm = VirtualMachine::new().with_step_cap(1_000_000);
    let o = match vm.run_module(module) {
        Ok(v) => Outcome::Ok(v.raw()),
        Err(e) => Outcome::Err(e),
    };
    let live = vm.heap_live_count();
    (o, live)
}

pub struct Rng(pub u64);
impl Rng {
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12; x ^= x << 25; x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn below(&mut self, n: usize) -> usize { (self.next() % n as u64) as usize }
}

pub fn random_op(rng: &mut Rng, d: Register, a: Register, b: Register, nconst: usize) -> OpCode {
    match rng.below(21) {
        0 => OpCode::Add(d, a, b),
        1 => OpCode::Sub(d, a, b),
        2 => OpCode::Mul(d, a, b),
        3 => OpCode::Div(d, a, b),
        4 => OpCode::Mod(d, a, b),
        5 => OpCode::Neg(d, a),
        6 => OpCode::Lt(d, a, b),
        7 => OpCode::Gt(d, a, b),
        8 => OpCode::Lte(d, a, b),
        9 => OpCode::Gte(d, a, b),
        10 => OpCode::Eq(d, a, b),
        11 => OpCode::Neq(d, a, b),
        12 => OpCode::And(d, a, b),
        13 => OpCode::Or(d, a, b),
        14 => OpCode::Xor(d, a, b),
        15 => OpCode::Shl(d, a, b),
        16 => OpCode::Shr(d, a, b),
        17 => OpCode::AddImm(d, a, (rng.next() % 7) as i8 - 3),
        18 => OpCode::SubImm(d, a, (rng.next() % 7) as i8 - 3),
        19 => OpCode::Move(d, a),
        _  => OpCode::PushConst(d, rng.below(nconst) as u16),
    }
}
