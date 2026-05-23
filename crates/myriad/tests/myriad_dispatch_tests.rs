use polka::{BytecodeChunk, Chunk, NativeChunk, OpCode, Register, Module};
use myriad::{Value, VirtualMachine};
use std::rc::Rc;

fn r(n: u8) -> Register { Register(n) }

fn raw_constants(consts: Vec<Value>) -> Vec<u64> {
    consts.into_iter().map(|v| v.raw()).collect()
}

fn run(ops: Vec<OpCode>, constants: Vec<Value>) -> Result<Value, String> {
    VirtualMachine::new().run(&Chunk::Bytecode(BytecodeChunk {
        code: ops,
        constants: raw_constants(constants),
        const_mask: Vec::new(),
        reg_count: 64,
        param_count: 0,
        string_constants: Vec::new(),
    }))
}

fn dispatch_key(effect_id: u16, op_id: u8) -> i64 {
    (((effect_id as i64) << 8) | (op_id as i64)) & 0xFFFF
}

fn dispatch_port_lookup() -> i64 {
    ((polka::DISPATCH_ID as i64) << 8) | (polka::DISPATCH_PORT_LOOKUP as i64)
}

fn dispatch_port_pop() -> i64 {
    ((polka::DISPATCH_ID as i64) << 8) | (polka::DISPATCH_PORT_POP_HANDLER as i64)
}

#[test]
fn test_call_reg_dispatches_to_bytecode() {
    let callee = BytecodeChunk {
        code: vec![
            OpCode::PushConst(r(1), 0),
            OpCode::Add(r(0), r(0), r(1)),
            OpCode::Ret(r(0)),
        ],
        constants: raw_constants(vec![Value::from_int(1)]),
        const_mask: Vec::new(),
        reg_count: 2,
        param_count: 1, string_constants: Vec::new(),
    };
    let caller = BytecodeChunk {
        code: vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Copy(r(4), r(0)),
            OpCode::CallReg(r(2), r(1)),
            OpCode::Ret(r(2)),
        ],
        constants: raw_constants(vec![Value::from_int(41), Value::from_int(0)]),
        const_mask: Vec::new(),
        reg_count: 4,
        param_count: 0, string_constants: Vec::new(),
    };
    let module = Module {
        functions: vec![Chunk::Bytecode(callee), Chunk::Bytecode(caller)],
        entry: 1,
        flags: 0,

        exports: vec![],
    };
    assert_eq!(VirtualMachine::new().run_module(&module), Ok(Value::from_int(42)));
}

#[test]
fn test_call_reg_dispatches_to_native() {
    let native = NativeChunk {
        param_count: 1,
        name: "test_double".into(),
    };
    let caller = BytecodeChunk {
        code: vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Copy(r(4), r(0)),
            OpCode::CallReg(r(2), r(1)),
            OpCode::Ret(r(2)),
        ],
        constants: raw_constants(vec![Value::from_int(21), Value::from_int(0)]),
        const_mask: Vec::new(),
        reg_count: 4,
        param_count: 0, string_constants: Vec::new(),
    };
    let module = Module {
        functions: vec![Chunk::Native(native), Chunk::Bytecode(caller)],
        entry: 1,
        flags: 0,

        exports: vec![],
    };
    let mut vm = VirtualMachine::new();
    vm.register_native("test_double", Rc::new(|_ctx: &mut myriad::NativeCtx<'_>, args: &[Value]| {
        let n = args[0].as_int();
        Ok((Value::from_int(n * 2), false))
    }));
    assert_eq!(vm.run_module(&module), Ok(Value::from_int(42)));
}

#[test]
fn test_handle_records_dispatch_table() {
    let mut vm = VirtualMachine::new();
    let module = Module {
        functions: vec![Chunk::Bytecode(BytecodeChunk {
            code: vec![
                OpCode::Alloc(r(0), 1),
                OpCode::PushConst(r(1), 0),
                OpCode::St(r(1), r(0), 0),
                OpCode::Handle(r(0), 7),
                OpCode::PushConst(r(2), 1),
                OpCode::PushConst(r(3), 2),
                OpCode::Deo(r(2), r(3)),
                OpCode::Dei(r(4), r(3)),
                OpCode::Ret(r(4)),
            ],
            constants: raw_constants(vec![
                Value::from_int(99),
                Value::from_int(dispatch_key(7, 0)),
                Value::from_int(dispatch_port_lookup()),
            ]),
            const_mask: Vec::new(),
            reg_count: 8, param_count: 0, string_constants: Vec::new(),
        })],
        entry: 0,
        flags: 0,

        exports: vec![],
    };
    let v = vm.run_module(&module).expect("dispatch must succeed");
    assert_eq!(v, Value::from_int(99));
}

#[test]
fn test_dispatch_no_match_returns_sentinel() {
    let mut vm = VirtualMachine::new();
    let module = Module {
        functions: vec![Chunk::Bytecode(BytecodeChunk {
            code: vec![
                OpCode::PushConst(r(0), 0),
                OpCode::PushConst(r(1), 1),
                OpCode::Deo(r(0), r(1)),
                OpCode::Dei(r(2), r(1)),
                OpCode::Ret(r(2)),
            ],
            constants: raw_constants(vec![
                Value::from_int(dispatch_key(99, 0)),
                Value::from_int(dispatch_port_lookup()),
            ]),
            const_mask: Vec::new(),
            reg_count: 4, param_count: 0, string_constants: Vec::new(),
        })],
        entry: 0,
        flags: 0,

        exports: vec![],
    };
    let v = vm.run_module(&module).expect("must run");
    assert_eq!(v, Value::from_int(polka::DISPATCH_NO_MATCH as i64));
}

#[test]
fn test_pop_handler_clears_frame_and_cell() {
    let mut vm = VirtualMachine::new();
    let module = Module {
        functions: vec![Chunk::Bytecode(BytecodeChunk {
            code: vec![
                OpCode::Alloc(r(0), 1),
                OpCode::Handle(r(0), 5),
                OpCode::PushConst(r(2), 0),
                OpCode::PushConst(r(3), 1),
                OpCode::Deo(r(2), r(3)),
                OpCode::Ret(r(0)),
            ],
            constants: raw_constants(vec![
                Value::UNIT,
                Value::from_int(dispatch_port_pop()),
            ]),
            const_mask: Vec::new(),
            reg_count: 4, param_count: 0, string_constants: Vec::new(),
        })],
        entry: 0,
        flags: 0,

        exports: vec![],
    };
    let _ = vm.run_module(&module).expect("must run");
    assert_eq!(vm.heap_live_count(), 1);
}

#[test]
fn test_pop_handler_without_frame_traps() {
    let result = run(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Deo(r(0), r(1)),
            OpCode::Ret(r(0)),
        ],
        vec![
            Value::UNIT,
            Value::from_int(dispatch_port_pop()),
        ],
    );
    assert!(result.is_err());
}

#[test]
fn test_nested_handlers_innermost_wins() {
    let mut vm = VirtualMachine::new();
    let module = Module {
        functions: vec![Chunk::Bytecode(BytecodeChunk {
            code: vec![
                OpCode::Alloc(r(0), 1),
                OpCode::PushConst(r(1), 0),
                OpCode::St(r(1), r(0), 0),
                OpCode::Handle(r(0), 3),
                OpCode::Alloc(r(2), 1),
                OpCode::PushConst(r(3), 1),
                OpCode::St(r(3), r(2), 0),
                OpCode::Handle(r(2), 3),
                OpCode::PushConst(r(4), 2),
                OpCode::PushConst(r(5), 3),
                OpCode::Deo(r(4), r(5)),
                OpCode::Dei(r(6), r(5)),
                OpCode::Ret(r(6)),
            ],
            constants: raw_constants(vec![
                Value::from_int(11),
                Value::from_int(22),
                Value::from_int(dispatch_key(3, 0)),
                Value::from_int(dispatch_port_lookup()),
            ]),
            const_mask: Vec::new(),
            reg_count: 16, param_count: 0, string_constants: Vec::new(),
        })],
        entry: 0,
        flags: 0,

        exports: vec![],
    };
    let v = vm.run_module(&module).expect("must run");
    assert_eq!(v, Value::from_int(22));
}
