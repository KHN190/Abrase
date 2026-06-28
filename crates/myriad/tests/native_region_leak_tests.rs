// Native-returned handle inside a region must be reclaimed at pop (else leaks 1/call).
use myriad::{NativeCtx, Value, VirtualMachine};
use polka::{BytecodeChunk, Chunk, Module, NativeChunk, OpCode, Register};
use std::rc::Rc;

fn r(n: u8) -> Register {
    Register(n)
}
fn raw(c: Vec<Value>) -> Vec<u64> {
    c.into_iter().map(|v| v.raw()).collect()
}
fn region_port(p: u8) -> i64 {
    ((polka::REGION_ID as i64) << 8) | p as i64
}

fn alloc_native() -> myriad::NativeFn {
    Rc::new(|ctx: &mut NativeCtx<'_>, _args: &[Value]| {
        let (slot, gen_) = ctx.heap.try_alloc(2)?;
        Ok((Value::from_handle(slot, gen_), true))
    })
}

fn region_call_module() -> Module {
    let native = NativeChunk { param_count: 0, name: "test_alloc".into() };
    let caller = BytecodeChunk {
        src_file: String::new(),
        lines: vec![],
        code: vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Deo(r(0), r(1)), // region push
            OpCode::Call(r(2), 0),   // r2 = test_alloc() -> owned handle, never dropped
            OpCode::PushConst(r(3), 2),
            OpCode::Deo(r(0), r(3)), // region pop -> must reclaim the recorded cell
            OpCode::PushConst(r(4), 3),
            OpCode::Ret(r(4)),
        ],
        constants: raw(vec![
            Value::from_int(0),
            Value::from_int(region_port(polka::REGION_PORT_PUSH)),
            Value::from_int(region_port(polka::REGION_PORT_POP)),
            Value::from_int(99),
        ]),
        const_mask: Vec::new(),
        string_constants: Vec::new(),
        reg_count: 8,
        param_count: 0,
    };
    Module {
        functions: vec![Chunk::Native(native), Chunk::Bytecode(caller)],
        entry: 1,
        flags: 0,
        exports: vec![],
    }
}

#[test]
fn region_pop_reclaims_native_returned_handle() {
    let module = region_call_module();
    let mut vm = VirtualMachine::new();
    vm.register_native("test_alloc", alloc_native());
    let res = vm.run_module(&module).expect("run");
    assert_eq!(res, Value::from_int(99));
    assert_eq!(
        vm.heap_live_count(),
        0,
        "native-returned handle inside region must be force-freed at pop"
    );
}
