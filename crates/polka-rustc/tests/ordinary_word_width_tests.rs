#[path = "diff/common.rs"]
mod common;
use common::{Outcome, chunk, compile_run_full, fn_chunk, r};
use polka::{CART_FLAG_INT32_SAFE, Chunk, Module, NativeChunk, OpCode as O};

fn module(code: Vec<O>, constants: Vec<u64>, flags: u16) -> Module {
    Module {
        functions: vec![Chunk::Bytecode(chunk(code, constants, 8))],
        entry: 0,
        flags,
        exports: vec![],
    }
}
fn run32(module: &Module) -> String {
    let source = polka_rustc::transpile_module(module).unwrap();
    assert!(source.contains("use myriad::word32 as runtime;"));
    assert!(!source.contains(" as i64"));
    assert!(!source.contains("Vec<u64>"));
    source
}

#[test]
fn ordinary_32_integer_overflow_and_shift_mask_are_real_word_operations() {
    let overflow = module(
        vec![
            O::PushConst(r(0), 0),
            O::PushConst(r(1), 1),
            O::Add(r(2), r(0), r(1)),
            O::PushConst(r(3), 2),
            O::Lt(r(4), r(2), r(3)),
            O::Ret(r(4)),
        ],
        vec![i32::MAX as u64, 1, 0],
        CART_FLAG_INT32_SAFE,
    );
    assert_eq!(compile_run_full(&run32(&overflow)), (Outcome::Ok(1), 0));
    for (shift, expected) in [(32, 1), (63, 0x80000000)] {
        let m = module(
            vec![
                O::PushConst(r(0), 0),
                O::PushConst(r(1), 1),
                O::Shl(r(2), r(0), r(1)),
                O::Ret(r(2)),
            ],
            vec![1, shift],
            CART_FLAG_INT32_SAFE,
        );
        assert_eq!(compile_run_full(&run32(&m)), (Outcome::Ok(expected), 0));
    }
}

#[test]
fn ordinary_32_float_rounding_and_heap_native_abi_use_the_typed_instance() {
    let f = module(
        vec![
            O::PushConst(r(0), 0),
            O::PushConst(r(1), 1),
            O::FAdd(r(2), r(0), r(1)),
            O::FSub(r(3), r(2), r(0)),
            O::Ret(r(3)),
        ],
        vec![16777216f32.to_bits() as u64, 1f32.to_bits() as u64],
        CART_FLAG_INT32_SAFE,
    );
    assert_eq!(compile_run_full(&run32(&f)), (Outcome::Ok(0), 0));
    let m = Module {
        functions: vec![
            Chunk::Native(NativeChunk {
                name: "__int_max".into(),
                param_count: 2,
            }),
            fn_chunk(
                vec![
                    O::PushConst(r(0), 0),
                    O::PushConst(r(1), 1),
                    O::Move(r(4), r(0)),
                    O::Move(r(5), r(1)),
                    O::Alloc(r(3), 65),
                    O::Call(r(2), 0),
                    O::Drop(r(3)),
                    O::Ret(r(2)),
                ],
                vec![0x80000000, 1],
                4,
                0,
            ),
        ],
        entry: 1,
        flags: CART_FLAG_INT32_SAFE,
        exports: vec![],
    };
    assert_eq!(compile_run_full(&run32(&m)), (Outcome::Ok(1), 0));
}

fn effect_module(bridge: bool) -> Module {
    let entry = fn_chunk(
        vec![
            O::PushConst(r(1), 0),
            O::Handle(r(1), 0),
            if bridge {
                O::Call(r(0), 0)
            } else {
                O::PushConst(r(0), 1)
            },
            O::PushConst(r(2), 2),
            O::Deo(r(0), r(2)),
            O::Ret(r(0)),
        ],
        vec![u64::MAX, 42, 0xe001],
        4,
        0,
    );
    let functions = if bridge {
        vec![
            fn_chunk(
                vec![
                    O::PushConst(r(0), 0),
                    O::PushConst(r(1), 1),
                    O::Shl(r(2), r(0), r(1)),
                    O::Ret(r(2)),
                ],
                vec![1, 32],
                3,
                0,
            ),
            entry,
        ]
    } else {
        vec![entry]
    };
    Module {
        entry: functions.len() - 1,
        functions,
        flags: CART_FLAG_INT32_SAFE,
        exports: vec![],
    }
}

#[test]
fn hybrid_and_embed_keep_the_same_word32_vm_boundary() {
    for bridge in [false, true] {
        let m = effect_module(bridge);
        let src = run32(&m);
        assert_eq!(src.contains("register_aot_fn"), bridge);
        assert_eq!(
            compile_run_full(&src),
            (Outcome::Ok(if bridge { 1 } else { 42 }), 0)
        );
        let library = polka_rustc::transpile_module_lib(&m).unwrap();
        assert!(library.contains("runtime::VirtualMachine"));
        let driver = format!(
            "{library}\nfn main() {{ let module=myriad::read_pk(PK).unwrap();let mut vm=myriad::word32::VirtualMachine::new();myriad::word32::Host::default().install_into(&mut vm);register_aot(&mut vm);let v=vm.run_module(&module).unwrap();println!(\"OK {{}} {{}}\",v.raw(),vm.heap_live_count()); }}"
        );
        assert_eq!(
            compile_run_full(&driver),
            (Outcome::Ok(if bridge { 1 } else { 42 }), 0)
        );
    }
}

#[test]
fn mixed_width_batch_selects_aliases_inside_each_module() {
    let code = vec![
        O::PushConst(r(0), 0),
        O::PushConst(r(1), 1),
        O::Shl(r(2), r(0), r(1)),
        O::Ret(r(2)),
    ];
    let narrow = module(code.clone(), vec![1, 32], CART_FLAG_INT32_SAFE);
    let wide = module(code, vec![1, 32], 0);
    assert_eq!(
        common::compile_run_batch(&[&narrow, &wide]),
        vec![(Outcome::Ok(1), 0), (Outcome::Ok(1 << 32), 0)]
    );
}

#[test]
fn out_of_width_scalar_constants_are_rejected_before_emission() {
    let m = module(
        vec![O::PushConst(r(0), 0), O::Ret(r(0))],
        vec![1 << 40],
        CART_FLAG_INT32_SAFE,
    );
    assert!(polka_rustc::transpile_module(&m).is_err());
}

#[test]
fn native_trig_and_bytes_pool_preserve_word32_values_and_handle_tags() {
    let mut m = Module {
        functions: vec![
            Chunk::Native(NativeChunk {
                name: "sin".into(),
                param_count: 1,
            }),
            fn_chunk(
                vec![
                    O::PushConst(r(0), 0),
                    O::Move(r(2), r(0)),
                    O::Call(r(1), 0),
                    O::Ret(r(1)),
                ],
                vec![0x3fc90fe0],
                2,
                0,
            ),
        ],
        entry: 1,
        flags: CART_FLAG_INT32_SAFE,
        exports: vec![],
    };
    let mut vm = myriad::word32::VirtualMachine::new();
    myriad::word32::Host::default().install_into(&mut vm);
    let expected = vm.run_module(&m).unwrap().raw();
    assert_eq!(
        compile_run_full(&run32(&m)),
        (Outcome::Ok(expected as u64), 0)
    );
    m.functions[0] = Chunk::Native(NativeChunk {
        name: "__bytes_len".into(),
        param_count: 1,
    });
    let Chunk::Bytecode(bc) = &mut m.functions[1] else {
        unreachable!()
    };
    bc.constants = vec![1 << 63];
    bc.const_mask = vec![1];
    bc.bytes_constants = vec![vec![0, 1, 2, 3, 4, 5, 6, 7, 8]];
    assert_eq!(compile_run_full(&run32(&m)), (Outcome::Ok(9), 0));
}

#[test]
fn word32_library_registration_cannot_accept_the_word64_vm() {
    let library = polka_rustc::transpile_module_lib(&effect_module(true)).unwrap();
    let source = format!(
        "{library}\nfn main() {{ let mut vm=myriad::word64::VirtualMachine::new();register_aot(&mut vm); }}"
    );
    struct TempDirectory(std::path::PathBuf);
    impl Drop for TempDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let temporary = TempDirectory(
        std::env::temp_dir().join(format!("abrase_aot_width_contract_{}", std::process::id())),
    );
    let directory = &temporary.0;
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("wrong_vm.rs");
    std::fs::write(&path, source).unwrap();
    let output = std::process::Command::new("rustc")
        .args(["--edition=2021", "--emit=metadata"])
        .args(common::rustflags())
        .arg("--extern")
        .arg(format!("myriad={}", common::myriad_rlib()))
        .arg("-L")
        .arg(common::deps_dir())
        .arg(&path)
        .arg("-o")
        .arg(directory.join("wrong_vm.rmeta"))
        .output()
        .unwrap();
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(error.contains("E0308"), "{error}");
}

#[test]
fn word32_copy_move_and_deep_forget_keep_rc_and_region_ownership() {
    let m = module(
        vec![
            O::PushConst(r(7), 0),
            O::Deo(r(7), r(7)),
            O::Alloc(r(0), 65),
            O::Alloc(r(1), 1),
            O::St(r(1), r(0), 64),
            O::Copy(r(2), r(0)),
            O::PushConst(r(7), 1),
            O::Deo(r(0), r(7)),
            O::Drop(r(2)),
            O::PushConst(r(7), 2),
            O::Deo(r(7), r(7)),
            O::Ld(r(3), r(0), 64),
            O::Drop(r(3)),
            O::Drop(r(0)),
            O::PushConst(r(4), 3),
            O::Ret(r(4)),
        ],
        vec![0xe100, 0xe102, 0xe101, 42],
        CART_FLAG_INT32_SAFE,
    );
    let expected = common::interp_with_live(&m);
    assert_eq!(expected, (Outcome::Ok(42), 0));
    assert_eq!(compile_run_full(&run32(&m)), expected);
}
