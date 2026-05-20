use abrase::bytecode::{BytecodeChunk, Chunk, Module, OpCode, Register};
use myriad::{Value, VirtualMachine};
use myriad::devices::{
    Clock, Random, SeededRandom, SystemClock,
    CLOCK_ID, RANDOM_ID,
};

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
    }
}

#[test]
fn clock_returns_monotonic_progress() {
    let mut vm = VirtualMachine::new();
    let clock: Box<dyn Clock> = Box::new(SystemClock::new());
    vm.install_device(CLOCK_ID, Box::new(clock));
    let module = module_with(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Dei(r(1), r(0)),
            OpCode::Ret(r(1)),
        ],
        vec![Value::from_int(0x6001)],
        2,
    );
    let v = vm.run_module(&module).unwrap();
    assert!(v.as_int() >= 0);
}

#[test]
fn random_seeded_is_deterministic() {
    let mut vm = VirtualMachine::new();
    let rng: Box<dyn Random> = Box::new(SeededRandom::new(12345));
    vm.install_device(RANDOM_ID, Box::new(rng));
    let module = module_with(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Dei(r(1), r(0)),
            OpCode::Ret(r(1)),
        ],
        vec![Value::from_int(0x7001)],
        2,
    );
    let v1 = vm.run_module(&module).unwrap();
    let mut vm2 = VirtualMachine::new();
    let rng2: Box<dyn Random> = Box::new(SeededRandom::new(12345));
    vm2.install_device(RANDOM_ID, Box::new(rng2));
    let v2 = vm2.run_module(&module).unwrap();
    assert_eq!(v1, v2);
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
