use abrase::bytecode::{BytecodeChunk, Chunk, Module, OpCode, Register};
use myriad::{Value, VirtualMachine};
use myriad::devices::{
    BufferConsole, ClockDevice, Console, HostFuncDevice, RandomDevice, SystemDevice,
    CLOCK_ID, CONSOLE_ID, HOSTFUNC_ID, RANDOM_ID, SYSTEM_ID,
};
use std::rc::Rc;

fn r(n: u8) -> Register { Register(n) }

fn module_with(code: Vec<OpCode>, constants: Vec<Value>, reg_count: usize, mask: [u8; 32]) -> Module {
    Module {
        functions: vec![Chunk::Bytecode(BytecodeChunk { code, constants, reg_count, param_count: 0, string_constants: Vec::new() })],
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
fn console_write_byte_to_stdout() {
    let mut vm = VirtualMachine::new();
    let console = BufferConsole::new();
    let (out_handle, _) = console.handles();
    let boxed: Box<dyn Console> = Box::new(console);
    vm.install_device(CONSOLE_ID, Box::new(boxed));

    let module = module_with(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Deo(r(0), r(1)),
            OpCode::PushConst(r(2), 2),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(b'A' as i64), Value::from_int(0x1001), Value::from_int(0)],
        3,
        mask_with(&[CONSOLE_ID]),
    );
    vm.run_module(&module).unwrap();
    assert_eq!(&*out_handle.borrow(), b"A");
}

#[test]
fn console_write_byte_to_stderr() {
    let mut vm = VirtualMachine::new();
    let console = BufferConsole::new();
    let (_, err_handle) = console.handles();
    let boxed: Box<dyn Console> = Box::new(console);
    vm.install_device(CONSOLE_ID, Box::new(boxed));

    let module = module_with(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Deo(r(0), r(1)),
            OpCode::PushConst(r(2), 2),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(b'E' as i64), Value::from_int(0x1002), Value::from_int(0)],
        3,
        mask_with(&[CONSOLE_ID]),
    );
    vm.run_module(&module).unwrap();
    assert_eq!(&*err_handle.borrow(), b"E");
}

#[test]
fn console_stdin_read_returns_minus_one_on_empty() {
    let mut vm = VirtualMachine::new();
    let console: Box<dyn Console> = Box::new(BufferConsole::new());
    vm.install_device(CONSOLE_ID, Box::new(console));

    let module = module_with(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Dei(r(1), r(0)),
            OpCode::Ret(r(1)),
        ],
        vec![Value::from_int(0x1000)],
        2,
        mask_with(&[CONSOLE_ID]),
    );
    assert_eq!(vm.run_module(&module).unwrap(), Value::from_int(-1));
}

#[test]
fn system_halt_returns_exit_code() {
    let mut vm = VirtualMachine::new();
    vm.install_device(SYSTEM_ID, Box::new(SystemDevice::new()));
    let module = module_with(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Deo(r(0), r(1)),
            OpCode::Ret(r(0)),
        ],
        vec![Value::from_int(7), Value::from_int(0x0001)],
        2,
        mask_with(&[SYSTEM_ID]),
    );
    assert_eq!(vm.run_module(&module).unwrap(), Value::from_int(7));
}

#[test]
fn system_panic_traps() {
    let mut vm = VirtualMachine::new();
    vm.install_device(SYSTEM_ID, Box::new(SystemDevice::new()));
    let module = module_with(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Deo(r(0), r(1)),
            OpCode::Ret(r(0)),
        ],
        vec![Value::from_int(0), Value::from_int(0x0002)],
        2,
        mask_with(&[SYSTEM_ID]),
    );
    let result = vm.run_module(&module);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("panic"));
}

#[test]
fn system_version_read() {
    let mut vm = VirtualMachine::new();
    vm.install_device(SYSTEM_ID, Box::new(SystemDevice::new()));
    let module = module_with(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Dei(r(1), r(0)),
            OpCode::Ret(r(1)),
        ],
        vec![Value::from_int(0x0000)],
        2,
        mask_with(&[SYSTEM_ID]),
    );
    let v = vm.run_module(&module).unwrap();
    if let Some(n) = v.as_int() {
        assert!(n >= (1i64 << 32), "version must be at least major=1");
    } else { panic!("expected Int, got {:?}", v); }
}

#[test]
fn missing_device_load_rejected() {
    let mut vm = VirtualMachine::new();
    let module = module_with(
        vec![OpCode::PushConst(r(0), 0), OpCode::Ret(r(0))],
        vec![Value::from_int(0)],
        1,
        mask_with(&[CONSOLE_ID]),
    );
    let result = vm.run_module(&module);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("0x10"));
}

#[test]
fn dei_on_uninstalled_device_traps() {
    let mut vm = VirtualMachine::new();
    let module = module_with(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Dei(r(1), r(0)),
            OpCode::Ret(r(1)),
        ],
        vec![Value::from_int(0x9000)],
        2,
        [0; 32],
    );
    let result = vm.run_module(&module);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not installed"));
}

#[test]
fn hostfunc_round_trip() {
    let mut vm = VirtualMachine::new();
    let mut dev = HostFuncDevice::new();
    dev.register(Rc::new(|_pool: &mut myriad::BoxPool, args: &[Value]| {
        let a = args[0].as_int().ok_or("expected int")?;
        let b = args[1].as_int().ok_or("expected int")?;
        Ok(Value::from_int(a + b))
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
    vm.install_device(CLOCK_ID, Box::new(ClockDevice::new()));
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
    assert!(matches!(v.as_int(), Some(n) if n >= 0));
}

#[test]
fn random_seeded_is_deterministic() {
    let mut vm = VirtualMachine::new();
    vm.install_device(RANDOM_ID, Box::new(RandomDevice::seeded(12345)));
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
    vm2.install_device(RANDOM_ID, Box::new(RandomDevice::seeded(12345)));
    let v2 = vm2.run_module(&module).unwrap();
    assert_eq!(v1, v2);
}

// Runtime ships only `device_in` / `device_out` by default. `println` and
// friends are now optional — this test verifies that calling an unregistered
// fn name surfaces a clean compile error rather than panicking.
#[test]
fn runtime_eval_unregistered_fn_errors_cleanly() {
    use abrase_cli::host::Runtime;
    let (mut rt, _console) = Runtime::new_for_tests();
    let src = r#"
        fn main() -> Int { println("hi"); 0 }
    "#;
    let err = rt.eval(src).expect_err("println isn't registered");
    assert!(err.to_lowercase().contains("println")
            || err.to_lowercase().contains("undefined"),
        "expected an undefined-fn diagnostic; got: {}", err);
}

#[test]
fn runtime_user_registered_host_fn() {
    use abrase_cli::host::Runtime;
    use abrase::ty::Type;
    let (mut rt, _console) = Runtime::new_for_tests();
    rt.register_host("triple", vec![Type::Int], Type::Int, |_pool, args| {
        let n = args[0].as_int().ok_or("Int")?;
        Ok(Value::from_int(n * 3))
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
    assert_eq!(v, Value::from_int(0xFFFF), "dispatch with no handlers must return DISPATCH_NO_MATCH");
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
    assert!(vm.run_module(&module).is_ok(),
        "modules requiring 0xE0 should load without explicit install (it's intrinsic)");
}
