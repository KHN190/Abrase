use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;
use polka_rustc::core::{transpile_core, transpile_core_backend, AddrBackend};

const SRC: &str = r#"
fn at(off: Int) -> <core> Addr { __ptr_add(__arena_base(), off) }
pub fn read_status(off: Int) -> <core> Int { __peek32(at(off)) }
pub fn write_ctrl(off: Int, v: Int) -> <core> Unit { __poke32(at(off), v) }
pub fn r64(off: Int) -> <core> Int { __peek64(at(off)) }
pub fn w64(off: Int, v: Int) -> <core> Unit { __poke64(at(off), v) }
pub fn r8(off: Int) -> <core> Int { __peek8(at(off)) }
pub fn w8(off: Int, v: Int) -> <core> Unit { __poke8(at(off), v) }
"#;

fn build() -> abrase::bytecode::Module {
    let mut p = Parser::new(Lexer::new(SRC)).with_source(SRC.into());
    let ast = p.parse_program();
    assert!(p.errors.is_empty(), "parse: {:?}", p.errors);
    let mut c = Compiler::new().with_source(SRC.into()).with_lib(true);
    c.compile_module(&ast)
        .unwrap_or_else(|e| panic!("compile: {:?}", e.iter().map(|x| &x.message).collect::<Vec<_>>()))
}

#[test]
fn arena_backend_bounds_checks_and_never_emits_volatile() {
    let s = transpile_core(&build()).expect("transpile");
    assert!(s.contains("core_peek(arena"), "arena peek indexes the slice");
    assert!(s.contains("arena.len()"), "arena backend bounds-checks");
    assert!(!s.contains("read_volatile"), "arena backend must not emit volatile");
}

#[test]
fn device_backend_emits_volatile_without_bounds() {
    let s = transpile_core_backend(&build(), AddrBackend::Device).expect("transpile");
    assert!(s.contains("read_volatile"), "device peek is volatile");
    assert!(s.contains("write_volatile"), "device poke is volatile");
    assert!(!s.contains("arena.len()"), "device space has no arena bounds");
}

#[test]
fn device_backend_routes_arena_base_through_board_symbol() {
    let s = transpile_core_backend(&build(), AddrBackend::Device).expect("transpile");
    assert!(s.contains("MMIO_BASE"), "arena_base resolves to a board-set base symbol");
}

#[test]
fn device_backend_still_traps_misaligned_access() {
    let s = transpile_core_backend(&build(), AddrBackend::Device).expect("transpile");
    assert!(s.contains("misaligned"), "unaligned MMIO bus-faults, must still trap");
}

#[test]
fn same_core_source_transpiles_under_both_backends() {
    assert!(transpile_core(&build()).is_ok());
    assert!(transpile_core_backend(&build(), AddrBackend::Device).is_ok());
}

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static SEQ: AtomicU64 = AtomicU64::new(0);

const DEVICE_DRIVER: &str = r#"
#[repr(C, align(8))] struct Dev([u8; 64]);
static mut DEV: Dev = Dev([0u8; 64]);
fn main() {
    unsafe { MMIO_BASE = core::ptr::addr_of!(DEV) as u64; }
    let mut dummy = [0u8; 1];
    let a = dummy.as_mut_slice();
    write_ctrl(a, 0, 0xAAAA).unwrap();
    write_ctrl(a, 0, 0x1234).unwrap();
    let r0 = read_status(a, 0).unwrap();
    write_ctrl(a, 4, 0xBEEF).unwrap();
    let r4 = read_status(a, 4).unwrap();
    w64(a, 8, 0x0123456789abcdef).unwrap();
    let q = r64(a, 8).unwrap();
    let lo = r8(a, 8).unwrap();
    w8(a, 16, 0xA5).unwrap();
    let b = r8(a, 16).unwrap();
    let mis = read_status(a, 1).is_err();
    println!("{} {} {} {} {} {}", r0 as i64, r4 as i64, q as i64, lo as i64, b as i64, mis as i64);
}
"#;

#[test]
fn device_backend_volatile_roundtrip_all_widths_and_traps_misaligned() {
    let src = transpile_core_backend(&build(), AddrBackend::Device).expect("transpile");
    let full = format!("{}\n{}", src, DEVICE_DRIVER);
    let id = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("core_mmio_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let sp = dir.join(format!("m_{}.rs", id));
    let bp = dir.join(format!("m_{}.bin", id));
    std::fs::write(&sp, &full).unwrap();
    let st = Command::new("rustc")
        .args(["--edition", "2021", "-A", "warnings"])
        .arg(&sp).arg("-o").arg(&bp)
        .output().expect("rustc");
    assert!(st.status.success(), "rustc failed: {}\n{}", String::from_utf8_lossy(&st.stderr), full);
    let o = Command::new(&bp).output().expect("run");
    let s = String::from_utf8(o.stdout).unwrap();
    assert_eq!(s.trim(), "4660 48879 81985529216486895 239 165 1",
        "32/64/8-bit volatile roundtrip (LE), second write wins, misaligned peek32 traps");
}
