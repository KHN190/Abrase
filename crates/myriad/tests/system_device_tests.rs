use myriad::devices::{SystemDevice, SYSTEM_ID};
use myriad::{Device, Value, VirtualMachine};
use polka::{BytecodeChunk, Chunk, Module, OpCode, Register};

fn r(n: u8) -> Register { Register(n) }

#[test]
fn version_major_port_returns_2() {
    let mut dev = SystemDevice::new();
    assert_eq!(dev.read(0x00).unwrap().as_int(), 2);
}

#[test]
fn version_minor_port_returns_0() {
    let mut dev = SystemDevice::new();
    assert_eq!(dev.read(0x04).unwrap().as_int(), 0);
}

#[test]
fn version_patch_port_returns_1() {
    let mut dev = SystemDevice::new();
    assert_eq!(dev.read(0x05).unwrap().as_int(), 1);
}

#[test]
fn flags_port_reflects_field() {
    let mut dev = SystemDevice::new();
    assert_eq!(dev.read(0x03).unwrap().as_int(), 0);
    dev.flags = 0xABCD;
    assert_eq!(dev.read(0x03).unwrap().as_int(), 0xABCD);
}

#[test]
fn unknown_read_port_returns_zero() {
    let mut dev = SystemDevice::new();
    for port in [0x06, 0x7F, 0xFF] {
        assert_eq!(dev.read(port).unwrap().as_int(), 0, "port {:#x}", port);
    }
}

#[test]
fn write_is_noop_at_device_level() {
    let mut dev = SystemDevice::new();
    for port in [0x00, 0x01, 0x02, 0x03, 0xFF] {
        assert!(dev.write(port, Value::from_int(42)).is_ok(), "write port {:#x}", port);
    }
    assert_eq!(dev.flags, 0);
}

fn run_bytecode(code: Vec<OpCode>, constants: Vec<u64>, reg_count: usize) -> Result<Value, String> {
    let mut vm = VirtualMachine::new();
    vm.install_device(SYSTEM_ID, Box::new(SystemDevice::new()));
    let module = Module {
        functions: vec![Chunk::Bytecode(BytecodeChunk {
            code, constants,
            const_mask: Vec::new(),
            string_constants: Vec::new(),
            reg_count, param_count: 0,
        })],
        entry: 0,
        flags: 0,
    };
    vm.run_module(&module)
}

#[test]
fn halt_port_writes_exit_code() {
    let v = run_bytecode(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Deo(r(0), r(1)),
            OpCode::Ret(r(0)),
        ],
        vec![7, 0x0001],
        2,
    ).expect("halt is a clean exit");
    assert_eq!(v.as_int(), 7);
}

#[test]
fn panic_port_traps_with_message() {
    let r = run_bytecode(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Deo(r(0), r(1)),
            OpCode::Ret(r(0)),
        ],
        vec![42, 0x0002],
        2,
    );
    let err = r.expect_err("panic port must trap");
    assert!(err.contains("panic"), "expected 'panic' in: {}", err);
    assert!(err.contains("42"), "expected payload '42' in: {}", err);
}
