use abrase::bytecode::{BytecodeChunk, Chunk, Module, OpCode, Register};
use myriad::{Value, VirtualMachine};
use myriad::devices::{BufferConsole, Console, SystemDevice, CONSOLE_ID, SYSTEM_ID};

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
    );
    let v = vm.run_module(&module).unwrap();
    assert!(v.as_int() >= (2i64 << 48), "version must be at least major=2");
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
    );
    let result = vm.run_module(&module);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not installed"));
}
