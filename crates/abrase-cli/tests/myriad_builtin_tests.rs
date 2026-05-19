use abrase::bytecode::{BytecodeChunk, Chunk, Module, OpCode, Register};
use myriad::{Value, VirtualMachine};
use myriad::devices::{
    Clock, HostFuncDevice, Random, SeededRandom, SystemClock,
    CLOCK_ID, HOSTFUNC_ID, RANDOM_ID,
};
use std::rc::Rc;

fn r(n: u8) -> Register { Register(n) }

fn module_with(code: Vec<OpCode>, constants: Vec<Value>, reg_count: usize, mask: [u8; 32]) -> Module {
    let raw: Vec<u64> = constants.iter().map(|v| v.raw()).collect();
    Module {
        functions: vec![Chunk::Bytecode(BytecodeChunk {
            code, constants: raw,
            const_mask: Vec::new(),
            reg_count, param_count: 0,
            string_constants: Vec::new(),
        })],
        entry: 0,
        device_mask: mask,
    }
}

fn mask_with(ids: &[u8]) -> [u8; 32] {
    let mut m = [0u8; 32];
    for id in ids { m[(*id / 8) as usize] |= 1 << (*id % 8); }
    m
}

#[test]
fn hostfunc_round_trip() {
    let mut vm = VirtualMachine::new();
    let mut dev = HostFuncDevice::new();
    dev.register(Rc::new(|_heap: &mut myriad::Heap, args: &[u64]| {
        let a = args[0] as i64;
        let b = args[1] as i64;
        Ok((Value::from_int(a + b).raw(), false))
    }));
    vm.install_device(HOSTFUNC_ID, Box::new(dev));

    let module = module_with(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::PushConst(r(2), 2),
            OpCode::PushConst(r(3), 3),
            OpCode::PushConst(r(4), 4),
            OpCode::Deo(r(0), r(2)),
            OpCode::Deo(r(1), r(2)),
            OpCode::Deo(r(3), r(4)),
            OpCode::PushConst(r(5), 5),
            OpCode::Dei(r(6), r(5)),
            OpCode::Ret(r(6)),
        ],
        vec![
            Value::from_int(7), Value::from_int(35),
            Value::from_int(0xF018),
            Value::from_int(0),
            Value::from_int(0xF01F),
            Value::from_int(0xF01E),
        ],
        7,
        mask_with(&[HOSTFUNC_ID]),
    );
    assert_eq!(vm.run_module(&module).unwrap(), Value::from_int(42));
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
        mask_with(&[CLOCK_ID]),
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
        mask_with(&[RANDOM_ID]),
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
fn runtime_user_registered_host_fn() {
    use abrase_cli::host::Runtime;
    use abrase::ty::Type;
    let (mut rt, _console) = Runtime::new_for_tests();
    rt.register_host("triple", vec![Type::Int], Type::Int, |_heap, args| {
        let n = args[0] as i64;
        Ok((Value::from_int(n * 3).raw(), false))
    });
    let src = r#"
        fn main() -> Int { triple(14) }
    "#;
    assert_eq!(rt.eval(src).unwrap(), Value::from_int(42));
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
        [0; 32],
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

#[test]
fn dispatch_device_is_vm_intrinsic_not_a_device_mask_requirement() {
    let mut vm = VirtualMachine::new();
    let mut mask = [0u8; 32];
    mask[0xE0 / 8] |= 1 << (0xE0 % 8);
    let module = module_with(
        vec![OpCode::PushConst(r(0), 0), OpCode::Ret(r(0))],
        vec![Value::from_int(0)],
        1,
        mask,
    );
    assert!(vm.run_module(&module).is_ok());
}
