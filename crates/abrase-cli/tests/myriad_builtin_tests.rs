use abrase::bytecode::{BytecodeChunk, Chunk, Module, OpCode, Register};
use myriad::{Value, VirtualMachine};

fn r(n: u8) -> Register { Register(n) }

fn module_with(code: Vec<OpCode>, constants: Vec<Value>, reg_count: usize) -> Module {
    let raw: Vec<u64> = constants.iter().map(|v| v.raw()).collect();
    Module {
        functions: vec![Chunk::Bytecode(BytecodeChunk {
            code, constants: raw,
            const_mask: Vec::new(),
            reg_count, param_count: 0,
            string_constants: Vec::new(),
        })],
        entry: 0,
        flags: 0,

        exports: vec![],
    }
}

#[test]
fn runtime_eval_unregistered_fn_errors_cleanly() {
    use abrase_cli::host::Runtime;
    let (mut rt, _console) = Runtime::new_for_tests();
    let src = r#"
        fn main() -> Int { frobnicate(1); 0 }
    "#;
    let err = rt.eval(src).expect_err("frobnicate isn't registered");
    assert!(err.to_lowercase().contains("frobnicate")
            || err.to_lowercase().contains("undefined"),
        "expected an undefined-fn diagnostic; got: {}", err);
}

#[test]
fn dispatch_no_matching_handler_returns_no_match() {
    let mut vm = VirtualMachine::new();
    let module = module_with(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Deo(r(0), r(1)),
            OpCode::Dei(r(2), r(1)),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(0x0500), Value::from_int(0xE000)],
        3,
    );
    let v = vm.run_module(&module).unwrap();
    assert_eq!(v, Value::from_int(0xFFFF));
}

#[test]
fn device_out_reads_back_dispatch_env() {
    let src = r#"
        fn main() -> Int { device_out(57346) }
    "#;
    let mut rt = abrase_cli::host::Runtime::new();
    let result = rt.eval(src).expect("dispatch env without prior lookup returns NONE");
    assert_eq!(result, Value::from_int(0));
}
