use abrase::bytecode::{BytecodeChunk, Chunk, Module, OpCode, Register};
use abrase::vm::{Value, VirtualMachine};
use abrase::vm::devices::{BufferConsole, Console, SystemDevice, CONSOLE_ID, SYSTEM_ID};

fn r(n: u8) -> Register { Register(n) }

fn module_with(code: Vec<OpCode>, constants: Vec<Value>, reg_count: usize, mask: [u8; 32]) -> Module {
    Module {
        functions: vec![Chunk::Bytecode(BytecodeChunk {
            code, constants, reg_count, param_count: 0,
        })],
        entry: 0,
        device_mask: mask,
    }
}

#[test]
fn deo_writes_byte_to_console() {
    let mut vm = VirtualMachine::new();
    let console: Box<dyn Console> = Box::new(BufferConsole::new());
    vm.install_device(CONSOLE_ID, Box::new(console));

    let mut mask = [0u8; 32];
    mask[CONSOLE_ID as usize / 8] |= 1 << (CONSOLE_ID % 8);
    let module = module_with(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Deo(r(0), r(1)),
            OpCode::PushConst(r(2), 2),
            OpCode::Ret(r(2)),
        ],
        vec![Value::Int(b'A' as i64), Value::Int(0x1018), Value::Int(0)],
        3,
        mask,
    );
    let v = vm.run_module(&module).unwrap();
    assert_eq!(v, Value::Int(0));
}

#[test]
fn deo_writes_string_to_console_buffer() {
    let mut vm = VirtualMachine::new();
    let console = BufferConsole::new();
    let (out_handle, _err_handle) = console.handles();
    let boxed: Box<dyn Console> = Box::new(console);
    vm.install_device(CONSOLE_ID, Box::new(boxed));

    let mut mask = [0u8; 32];
    mask[CONSOLE_ID as usize / 8] |= 1 << (CONSOLE_ID % 8);
    let module = module_with(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Deo(r(0), r(1)),
            OpCode::PushConst(r(2), 2),
            OpCode::Ret(r(2)),
        ],
        vec![Value::String("hi".into()), Value::Int(0x101A), Value::Int(0)],
        3,
        mask,
    );
    vm.run_module(&module).unwrap();
    assert_eq!(&*out_handle.borrow(), b"hi");
}

#[test]
fn missing_device_load_rejected() {
    let vm = VirtualMachine::new();
    let mut mask = [0u8; 32];
    mask[CONSOLE_ID as usize / 8] |= 1 << (CONSOLE_ID % 8);
    let module = module_with(
        vec![OpCode::PushConst(r(0), 0), OpCode::Ret(r(0))],
        vec![Value::Int(0)],
        1,
        mask,
    );
    let mut vm = vm;
    let result = vm.run_module(&module);
    assert!(result.is_err(), "missing console must be rejected");
    let err = result.unwrap_err();
    assert!(err.contains("0x10") && err.contains("not installed"),
            "expected device-not-installed error, got: {}", err);
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
fn system_exit_halts_with_code() {
    let mut vm = VirtualMachine::new();
    vm.install_device(SYSTEM_ID, Box::new(SystemDevice::new()));
    let module = module_with(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Deo(r(0), r(1)),
            OpCode::PushConst(r(2), 2),
            OpCode::Ret(r(2)),
        ],
        vec![Value::Int(7), Value::Int(0x0000), Value::Int(999)],
        3,
        [0; 32],
    );
    let v = vm.run_module(&module).unwrap();
    assert_eq!(v, Value::Int(7), "exit code from r0 should be the returned value");
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
        vec![Value::Int(42), Value::Int(0x0001)],
        2,
        [0; 32],
    );
    let result = vm.run_module(&module);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("panic"));
}

#[test]
fn host_println_lowers_to_console_deo() {
    use abrase::lexer::Lexer;
    use abrase::parser::Parser;
    use abrase::compiler::Compiler;

    let src = r#"
        fn main() -> Int {
            __host_println("hi");
            0
        }
    "#;
    let mut parser = Parser::new(Lexer::new(src)).with_source(src.into());
    let ast = parser.parse_program();
    assert!(parser.errors.is_empty(), "parse errors: {:?}", parser.errors);

    let mut compiler = Compiler::new().with_source(src.into());
    let module = compiler.compile_module(&ast).map_err(|e| format!("{:?}", e)).unwrap();
    assert!(module.requires_device(CONSOLE_ID), "module must declare console requirement");

    let mut vm = VirtualMachine::new();
    let console = BufferConsole::new();
    let (out_handle, _) = console.handles();
    let boxed: Box<dyn Console> = Box::new(console);
    vm.install_device(CONSOLE_ID, Box::new(boxed));

    let v = vm.run_module(&module).unwrap();
    assert_eq!(v, Value::Int(0));
    assert_eq!(&*out_handle.borrow(), b"hi\n");
}
