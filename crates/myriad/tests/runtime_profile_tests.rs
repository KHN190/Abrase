use myriad::{Runtime, RuntimeValue, RuntimeProfile, word32};
use polka::{Module, BytecodeChunk, Chunk, OpCode, Register, CART_FLAG_INT32_SAFE};

fn module(flags: u16) -> Module {
    Module { functions: vec![Chunk::Bytecode(BytecodeChunk {
        code: vec![OpCode::PushConst(Register(0), 0), OpCode::AddImm(Register(1), Register(0), 1), OpCode::Ret(Register(1))],
        constants: vec![i32::MAX as u64], reg_count: 2, ..BytecodeChunk::default()
    })], entry: 0, flags, exports: vec![] }
}

#[test]
fn module_flag_selects_the_real_backend_without_configuration() {
    let small_module = module(CART_FLAG_INT32_SAFE);
    let mut small = Runtime::new(&small_module).unwrap();
    assert_eq!(small.profile(), RuntimeProfile::Word32);
    assert!(small.word64_mut().is_none());
    match small.run_module(&small_module).unwrap() {
        RuntimeValue::Word32(value) => { assert_eq!(std::mem::size_of_val(&value), 4); assert_eq!(value.as_int(), i32::MIN); }
        _ => panic!("narrow module ran in wide storage"),
    }
    let large_module = module(0);
    let mut large = Runtime::new(&large_module).unwrap();
    assert_eq!(large.profile(), RuntimeProfile::Word64);
    assert!(large.word32_mut().is_none());
    match large.run_module(&large_module).unwrap() {
        RuntimeValue::Word64(value) => { assert_eq!(std::mem::size_of_val(&value), 8); assert_eq!(value.as_int(), 2147483648); }
        _ => panic!("wide module ran in narrow storage"),
    }
}

#[test]
fn selected_profile_cannot_change_when_a_live_result_exists() {
    let mut m = module(CART_FLAG_INT32_SAFE);
    if let Chunk::Bytecode(bc) = &mut m.functions[0] { bc.code = vec![OpCode::Alloc(Register(0), 1), OpCode::Ret(Register(0))]; }
    let mut runtime = Runtime::new(&m).unwrap();
    let value = match runtime.run_module(&m).unwrap() { RuntimeValue::Word32(value) => value, _ => unreachable!() };
    let (slot, generation) = value.as_handle();
    assert!(runtime.run_module(&module(0)).is_err());
    assert_eq!(runtime.profile(), RuntimeProfile::Word32);
    assert!(runtime.word32_mut().unwrap().heap_ref().is_live(slot, generation));
    runtime.word32_mut().unwrap().drop_result_for_test(value.raw());
}

#[test]
fn selected_native_registration_uses_the_typed_abi() {
    let m = module(CART_FLAG_INT32_SAFE);
    let mut runtime = Runtime::new(&m).unwrap();
    runtime.word32_mut().unwrap().register_native("typed", std::rc::Rc::new(|_: &mut word32::NativeCtx, _: &[word32::Value]| {
        Ok((word32::Value::UNIT, false))
    }));
    assert!(runtime.word64_mut().is_none());
}
