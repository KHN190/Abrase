use abrase::bytecode::{BytecodeChunk, Chunk, Module, OpCode, Register};
use abrase::vm::{Value, VirtualMachine};
use abrase::vm::devices::{
    BufferConsole, ClockDevice, Console, HostFuncDevice, RandomDevice, SystemDevice,
    CLOCK_ID, CONSOLE_ID, HOSTFUNC_ID, RANDOM_ID, SYSTEM_ID,
};
use std::rc::Rc;

fn r(n: u8) -> Register { Register(n) }

fn module_with(code: Vec<OpCode>, constants: Vec<Value>, reg_count: usize, mask: [u8; 32]) -> Module {
    Module {
        functions: vec![Chunk::Bytecode(BytecodeChunk { code, constants, reg_count, param_count: 0 })],
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
        vec![Value::Int(b'A' as i64), Value::Int(0x1001), Value::Int(0)],
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
        vec![Value::Int(b'E' as i64), Value::Int(0x1002), Value::Int(0)],
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
        vec![Value::Int(0x1000)],
        2,
        mask_with(&[CONSOLE_ID]),
    );
    assert_eq!(vm.run_module(&module).unwrap(), Value::Int(-1));
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
        vec![Value::Int(7), Value::Int(0x0001)],
        2,
        mask_with(&[SYSTEM_ID]),
    );
    assert_eq!(vm.run_module(&module).unwrap(), Value::Int(7));
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
        vec![Value::Int(0), Value::Int(0x0002)],
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
        vec![Value::Int(0x0000)],
        2,
        mask_with(&[SYSTEM_ID]),
    );
    let v = vm.run_module(&module).unwrap();
    if let Value::Int(n) = v {
        assert!(n >= (1i64 << 32), "version must be at least major=1");
    } else { panic!("expected Int, got {:?}", v); }
}

#[test]
fn missing_device_load_rejected() {
    let mut vm = VirtualMachine::new();
    let module = module_with(
        vec![OpCode::PushConst(r(0), 0), OpCode::Ret(r(0))],
        vec![Value::Int(0)],
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
        vec![Value::Int(0x9000)],
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
    dev.register(Rc::new(|args: &[Value]| {
        let (a, b) = match (&args[0], &args[1]) {
            (Value::Int(a), Value::Int(b)) => (*a, *b),
            _ => return Err("expected ints".into()),
        };
        Ok(Value::Int(a + b))
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
            Value::Int(7), Value::Int(35),
            Value::Int(0xF018),
            Value::Int(0),
            Value::Int(0xF01F),
            Value::Int(0xF01E),
        ],
        7,
        mask_with(&[HOSTFUNC_ID]),
    );
    assert_eq!(vm.run_module(&module).unwrap(), Value::Int(42));
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
        vec![Value::Int(0x6001)],
        2,
        mask_with(&[CLOCK_ID]),
    );
    let v = vm.run_module(&module).unwrap();
    assert!(matches!(v, Value::Int(n) if n >= 0));
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
        vec![Value::Int(0x7001)],
        2,
        mask_with(&[RANDOM_ID]),
    );
    let v1 = vm.run_module(&module).unwrap();
    let mut vm2 = VirtualMachine::new();
    vm2.install_device(RANDOM_ID, Box::new(RandomDevice::seeded(12345)));
    let v2 = vm2.run_module(&module).unwrap();
    assert_eq!(v1, v2);
}

#[test]
fn runtime_eval_with_default_println() {
    use abrase::host::Runtime;
    let (mut rt, console) = Runtime::new_for_tests();
    let (out_handle, _) = console.handles();
    let src = r#"
        fn main() -> Int {
            println("hi");
            0
        }
    "#;
    let v = rt.eval(src).unwrap();
    assert_eq!(v, Value::Int(0));
    let _ = out_handle;
}

#[test]
fn runtime_user_registered_host_fn() {
    use abrase::host::Runtime;
    use abrase::ty::Type;
    let (mut rt, _console) = Runtime::new_for_tests();
    rt.register_host("triple", vec![Type::Int], Type::Int, |args| {
        let n = match &args[0] { Value::Int(n) => *n, _ => return Err("Int".into()) };
        Ok(Value::Int(n * 3))
    });
    let src = r#"
        fn main() -> Int { triple(14) }
    "#;
    assert_eq!(rt.eval(src).unwrap(), Value::Int(42));
}
