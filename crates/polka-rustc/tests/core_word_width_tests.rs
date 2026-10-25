#[path = "diff/common.rs"]
mod common;

use polka::{BytecodeChunk, Chunk, Export, Module, NativeChunk, OpCode, Register, CART_FLAG_INT32_SAFE};
use polka_rustc::core::{transpile_core, transpile_core_backend, AddrBackend};
use polka_rustc::TranspileError;

fn compile_run(src: &str) -> String {
    common::compile_run_raw(&(String::from("#![allow(warnings)]\n") + src))
}

fn r(n: u8) -> Register { Register(n) }

fn arithmetic(flags: u16) -> Module {
    let ops = [
        OpCode::Add(r(2), r(0), r(1)), OpCode::Sub(r(2), r(0), r(1)),
        OpCode::Mul(r(2), r(0), r(1)), OpCode::Neg(r(2), r(0)),
        OpCode::AddImm(r(2), r(0), 1), OpCode::SubImm(r(2), r(0), 1),
        OpCode::Shl(r(2), r(0), r(1)), OpCode::Shr(r(2), r(0), r(1)),
        OpCode::Lt(r(2), r(0), r(1)), OpCode::Div(r(2), r(0), r(1)),
        OpCode::Mod(r(2), r(0), r(1)),
    ];
    let count = ops.len();
    Module {
        functions: ops.into_iter().map(|op| Chunk::Bytecode(BytecodeChunk {
            code: vec![op, OpCode::Ret(r(2))], reg_count: 3, param_count: 2,
            ..BytecodeChunk::default()
        })).collect(),
        exports: (0..count).map(|i| Export { name: format!("op{}", i), fn_id: i as u16 }).collect(),
        entry: 0, flags,
    }
}

#[test]
fn core32_compiles_with_u32_abi_and_executes_i32_arithmetic() {
    let src = transpile_core(&arithmetic(CART_FLAG_INT32_SAFE)).unwrap();
    assert!(src.contains("a: &[u32]) -> Result<u32"));
    assert!(src.contains("let mut r0: u32"));
    assert!(!src.contains("u64"));
    assert!(!src.contains("i64"));
    let driver = r#"
fn main() {
    let a = &mut [];
    let _: fn(&mut [u8], u32, u32) -> Result<u32, &'static str> = op0;
    assert_eq!(op0(a, 2147483647, 1), Ok(2147483648));
    assert_eq!(op1(a, 2147483648, 1), Ok(2147483647));
    assert_eq!(op2(a, 65536, 65536), Ok(0));
    assert_eq!(op3(a, 2147483648, 0), Ok(2147483648));
    assert_eq!(op4(a, 2147483647, 0), Ok(2147483648));
    assert_eq!(op5(a, 2147483648, 0), Ok(2147483647));
    assert_eq!(op6(a, 1, 32), Ok(1));
    assert_eq!(op7(a, 2147483648, 32), Ok(2147483648));
    assert_eq!(op7(a, 2147483648, 1), Ok(3221225472));
    assert_eq!(op8(a, 2147483648, 0), Ok(1));
    assert!(op9(a, 2147483648, u32::MAX).is_err());
    assert!(op10(a, 2147483648, u32::MAX).is_err());
    println!("ok");
}
"#;
    assert_eq!(compile_run(&(src + driver)).trim(), "ok");
}

fn intrinsic(name: &str, count: usize, flags: u16) -> Module {
    let mut code: Vec<_> = (0..count).map(|i| OpCode::Copy(r((count + i) as u8), r(i as u8))).collect();
    code.extend([OpCode::Call(r(0), 1), OpCode::Ret(r(0))]);
    Module {
        functions: vec![
            Chunk::Bytecode(BytecodeChunk { code, reg_count: count, param_count: count, ..BytecodeChunk::default() }),
            Chunk::Native(NativeChunk { name: name.into(), param_count: count }),
        ],
        exports: vec![Export { name: "access".into(), fn_id: 0 }], entry: 0, flags,
    }
}

#[test]
fn core32_rejects_explicit_64_bit_memory_access() {
    for backend in [AddrBackend::Arena, AddrBackend::Device] {
        for (name, count) in [("__peek64", 1), ("__poke64", 2)] {
            let err = transpile_core_backend(&intrinsic(name, count, CART_FLAG_INT32_SAFE), backend).unwrap_err();
            assert!(matches!(err, TranspileError::Unsupported(ref msg) if msg.contains(name)));
            assert!(transpile_core_backend(&intrinsic(name, count, 0), backend).is_ok());
        }
    }
}

#[test]
fn core32_memory_access_compiles_and_preserves_bounds_and_bits() {
    for (name, count) in [("__peek8", 1), ("__peek32", 1), ("__poke8", 2), ("__poke32", 2)] {
        let module = intrinsic(name, count, CART_FLAG_INT32_SAFE);
        let src = transpile_core(&module).unwrap();
        let assertion = match name {
            "__peek8" => "assert_eq!(access(&mut arena, 1), Ok(0x56));",
            "__peek32" => "assert_eq!(access(&mut arena, 0), Ok(0x12345678));",
            "__poke8" => "access(&mut arena, 1, 0xab).unwrap(); assert_eq!(arena[1], 0xab);",
            _ => "access(&mut arena, 0, 0xfedcba98).unwrap(); assert_eq!(arena, [0x98, 0xba, 0xdc, 0xfe]);",
        };
        let driver = format!(r#"
fn main() {{
    let mut arena = [0x78, 0x56, 0x34, 0x12];
    {assertion}
    assert!(core_peek(&arena, 1, 4).is_err());
    assert!(core_peek(&arena, 4, 4).is_err());
    assert!(core_peek(&arena, u32::MAX - 3, 4).is_err());
    assert!(core_poke(&mut arena, 1, 0, 4).is_err());
    assert!(core_poke(&mut arena, 4, 0, 4).is_err());
    println!("ok");
}}
"#);
        assert_eq!(compile_run(&(src + &driver)).trim(), "ok");
        let src = transpile_core_backend(&module, AddrBackend::Device).unwrap();
        assert!(!src.contains("u64"));
        let params = if count == 1 { "u32" } else { "u32, u32" };
        let driver = format!(r#"fn main() {{ let _: fn(&mut [u8], {params}) -> Result<u32, &'static str> = access; println!("ok"); }}"#);
        assert_eq!(compile_run(&(src + &driver)).trim(), "ok");
    }
}

fn constant(raw: u64, flags: u16) -> Module {
    Module {
        functions: vec![Chunk::Bytecode(BytecodeChunk {
            code: vec![OpCode::PushConst(r(0), 0), OpCode::Ret(r(0))],
            constants: vec![raw], reg_count: 1, ..BytecodeChunk::default()
        })],
        exports: vec![Export { name: "value".into(), fn_id: 0 }], entry: 0, flags,
    }
}

#[test]
fn core32_accepts_word_bits_and_sign_extended_negative_constants() {
    for (raw, expected) in [(0, 0), (u32::MAX as u64, u32::MAX), (i32::MIN as i64 as u64, 0x80000000), (u64::MAX, u32::MAX)] {
        let src = transpile_core(&constant(raw, CART_FLAG_INT32_SAFE)).unwrap();
        let driver = format!(r#"fn main() {{ assert_eq!(value(&mut []), Ok({expected}u32)); println!("ok"); }}"#);
        assert_eq!(compile_run(&(src + &driver)).trim(), "ok");
    }
}

#[test]
fn core32_rejects_constants_with_unrepresentable_high_bits() {
    for raw in [0x100000000, 0xffffffff00000000, 0xffffffff7fffffff] {
        assert!(matches!(transpile_core(&constant(raw, CART_FLAG_INT32_SAFE)), Err(TranspileError::Unsupported(_))));
        assert!(transpile_core(&constant(raw, 0)).is_ok());
    }
}

#[test]
fn default_core_executes_64_bit_arithmetic() {
    let src = transpile_core(&arithmetic(0)).unwrap();
    let driver = r#"
fn main() {
    let a = &mut [];
    let _: fn(&mut [u8], u64, u64) -> Result<u64, &'static str> = op0;
    assert_eq!(op0(a, 2147483647, 1), Ok(2147483648));
    assert_eq!(op2(a, 65536, 65536), Ok(4294967296));
    assert_eq!(op6(a, 1, 32), Ok(4294967296));
    assert_eq!(op7(a, (-2147483648i64) as u64, 32), Ok(u64::MAX));
    println!("ok");
}
"#;
    assert_eq!(compile_run(&(src + driver)).trim(), "ok");
}

#[test]
fn core32_bounds_comparison_keeps_large_host_lengths() {
    let src = transpile_core(&constant(0, CART_FLAG_INT32_SAFE)).unwrap();
    let condition = src.lines().find(|line| line.contains("if ") && line.contains("arena.len()"))
        .unwrap().trim().strip_prefix("if ").unwrap().split(" { ").next().unwrap();
    let condition = condition.replace("arena.len()", "arena_len");
    let driver = format!(r#"
fn main() {{
    let out_of_bounds = |end: u32, arena_len: usize| {{ {condition} }};
    assert!(!out_of_bounds(4, 4));
    assert!(out_of_bounds(4, 3));
    #[cfg(target_pointer_width = "64")]
    {{
        assert!(!out_of_bounds(4, (u32::MAX as usize) + 1));
        assert!(!out_of_bounds(u32::MAX, (u32::MAX as usize) + 1));
    }}
    println!("ok");
}}
"#);
    assert_eq!(compile_run(&(src + &driver)).trim(), "ok");
}

#[test]
fn core_div_and_mod_trap_zero_and_signed_overflow_at_each_word_width() {
    for (flags, word, minimum) in [(0, "u64", "1u64 << 63"), (CART_FLAG_INT32_SAFE, "u32", "1u32 << 31")] {
        let src = transpile_core(&arithmetic(flags)).unwrap();
        let driver = format!(r#"
fn main() {{
    let arena = &mut [];
    let minimum = {minimum};
    assert_eq!(op9(arena, minimum, {word}::MAX), Err("div by zero"));
    assert_eq!(op10(arena, minimum, {word}::MAX), Err("mod by zero"));
    assert_eq!(op9(arena, 1, 0), Err("div by zero"));
    assert_eq!(op10(arena, 1, 0), Err("mod by zero"));
    assert_eq!(op9(arena, minimum, 1), Ok(minimum));
    assert_eq!(op10(arena, minimum, 1), Ok(0));
    println!("ok");
}}
"#);
        assert_eq!(compile_run(&(src + &driver)).trim(), "ok");
    }
}
