use myriad::{word32 as narrow, word64 as wide};
use polka::{BytecodeChunk, Chunk, Constant, Export, Module, NativeChunk, OpCode, Register, CART_FLAG_INT32_SAFE};
use std::rc::Rc;
fn r(n: u8) -> Register { Register(n) }
fn module(flags: u16) -> Module {
    Module { functions: vec![Chunk::Bytecode(BytecodeChunk {
        code: vec![OpCode::PushConst(r(0), 0), OpCode::AddImm(r(1), r(0), 1), OpCode::Ret(r(1))],
        constants: vec![Constant::from_le_bytes((i32::MAX as u64).to_le_bytes())], reg_count: 2,
        ..BytecodeChunk::default()
    })], entry: 0, flags, exports: vec![Export { name: "entry".into(), fn_id: 0 }] }
}
#[test]
fn no_feature_host_executes_both_real_numeric_profiles() {
    let mut small = narrow::VirtualMachine::new();
    let small_value: narrow::Value = small.run_module(&module(CART_FLAG_INT32_SAFE)).unwrap();
    let _: &narrow::Heap = small.heap_ref();
    assert_eq!(small_value.as_int(), i32::MIN);
    assert_eq!(std::mem::size_of_val(&small_value), 4);
    let mut large = wide::VirtualMachine::new();
    let large_value: wide::Value = large.run_module(&module(0)).unwrap();
    let _: &wide::Heap = large.heap_ref();
    assert_eq!(large_value.as_int(), 2147483648);
    assert_eq!(std::mem::size_of_val(&large_value), 8);
}
#[test]
fn typed_instances_reject_wrong_module_profiles_at_every_entry() {
    let mut small = narrow::VirtualMachine::new(); let wide_module = module(0);
    assert!(small.run_module(&wide_module).is_err());
    assert!(small.call_export(&wide_module, "entry", &[]).is_err());
    assert!(small.run_to_yield(&wide_module).is_err());
    assert!(small.resume(&wide_module, narrow::Value::ZERO).is_err());
    let mut large = wide::VirtualMachine::new(); let narrow_module = module(CART_FLAG_INT32_SAFE);
    assert!(large.run_module(&narrow_module).is_err());
    assert!(large.call_export(&narrow_module, "entry", &[]).is_err());
    assert!(large.run_to_yield(&narrow_module).is_err());
    assert!(large.resume(&narrow_module, wide::Value::ZERO).is_err());
}
#[test]
fn native_callbacks_receive_their_instances_nominal_abi() {
    let mut m = module(CART_FLAG_INT32_SAFE);
    m.functions[0] = Chunk::Bytecode(BytecodeChunk { code: vec![OpCode::Call(r(0), 1), OpCode::Ret(r(0))], reg_count: 1, ..BytecodeChunk::default() });
    m.functions.push(Chunk::Native(NativeChunk { name: "typed".into(), param_count: 0 }));
    let mut small = narrow::VirtualMachine::new();
    small.register_native("typed", Rc::new(|_: &mut narrow::NativeCtx, args: &[narrow::Value]| { assert!(args.is_empty()); Ok((narrow::Value::from_int(i32::MIN), false)) }));
    assert_eq!(small.run_module(&m).unwrap().as_int(), i32::MIN);
    m.flags = 0;
    let mut large = wide::VirtualMachine::new();
    large.register_native("typed", Rc::new(|_: &mut wide::NativeCtx, args: &[wide::Value]| { assert!(args.is_empty()); Ok((wide::Value::from_int(i64::MIN), false)) }));
    assert_eq!(large.run_module(&m).unwrap().as_int(), i64::MIN);
}

#[test]
fn convenience_run_uses_the_typed_loader() {
    assert_eq!(narrow::run(module(CART_FLAG_INT32_SAFE), narrow::Host::default()).unwrap(), i32::MIN);
    assert_eq!(wide::run(module(0), wide::Host::default()).unwrap(), 2147483648);
}

#[test]
fn rejected_profile_preserves_a_live_heap_result() {
    let mut m = module(CART_FLAG_INT32_SAFE);
    if let Chunk::Bytecode(bc) = &mut m.functions[0] {
        bc.code = vec![OpCode::Alloc(r(0), 1), OpCode::PushConst(r(1), 0), OpCode::St(r(1), r(0), 0), OpCode::Ret(r(0))];
    }
    let mut vm = narrow::VirtualMachine::new();
    let result = vm.run_module(&m).unwrap();
    let (slot, generation) = result.as_handle();
    let rc = vm.heap_ref().rc(slot, generation);
    assert!(vm.run_module(&module(0)).is_err());
    assert!(vm.call_export(&module(0), "entry", &[]).is_err());
    assert!(vm.run_to_yield(&module(0)).is_err());
    assert_eq!(vm.heap_ref().rc(slot, generation), rc);
    assert_eq!(vm.heap_ref().ld(slot, generation, 0).unwrap().0, i32::MAX as u32);
    vm.drop_result_for_test(result.raw());
    assert_eq!(vm.heap_live_count(), 0);
}

#[test]
fn rejected_profile_preserves_suspended_execution_and_live_handles() {
    let mut m = module(CART_FLAG_INT32_SAFE);
    if let Chunk::Bytecode(bc) = &mut m.functions[0] {
        bc.code = vec![OpCode::Alloc(r(64), 1), OpCode::PushConst(r(0), 0), OpCode::St(r(0), r(64), 0),
            OpCode::Call(r(1), 1), OpCode::Ld(r(2), r(64), 0), OpCode::Drop(r(64)), OpCode::Ret(r(2))];
        bc.reg_count = 65;
    }
    m.functions.push(Chunk::Native(NativeChunk { name: "__frame_present".into(), param_count: 0 }));
    let result = std::rc::Rc::new(std::cell::Cell::new(None)); let capture = result.clone();
    let mut vm = narrow::VirtualMachine::new().with_debug_sink(Box::new(move |event, _| {
        if let narrow::DebugEvent::Trace { op: OpCode::Ret(_), window, .. } = event { capture.set(Some(window[2])); }
    }));
    vm.run_to_yield(&m).unwrap();
    assert_eq!(vm.heap_live_count(), 1);
    assert!(vm.run_module(&module(0)).is_err());
    assert!(vm.run_to_yield(&module(0)).is_err());
    assert!(vm.resume(&module(0), narrow::Value::ZERO).is_err());
    assert_eq!(vm.heap_live_count(), 1);
    assert!(!vm.resume(&m, narrow::Value::ZERO).unwrap());
    assert_eq!(result.get(), Some(i32::MAX as u32));
    assert_eq!(vm.heap_live_count(), 0);
}

#[test]
fn malformed_high_constant_words_are_rejected_without_truncation() {
    let mut m = module(CART_FLAG_INT32_SAFE);
    if let Chunk::Bytecode(bc) = &mut m.functions[0] {
        bc.constants[0] = Constant::from_le_bytes([0,0,0,0,1,0,0,0]);
        bc.code = vec![OpCode::PushConst(r(0), 0), OpCode::Ret(r(0))];
    }
    assert!(narrow::VirtualMachine::new().run_module(&m).unwrap_err().contains("constant exceeds"));
    m.flags = 0;
    assert_eq!(wide::VirtualMachine::new().run_module(&m).unwrap().raw(), 1u64 << 32);
}

#[test]
fn device_and_aot_interfaces_keep_the_narrow_abi() {
    struct Echo(u32);
    impl narrow::Device for Echo {
        fn read(&mut self, _: u8) -> Result<(narrow::Value, bool), String> { Ok((narrow::Value::from_raw(self.0), false)) }
        fn write(&mut self, _: u8, value: narrow::Value, tag: bool, _: &mut narrow::Heap) -> Result<(), String> {
            assert!(!tag); self.0 = value.raw(); Ok(())
        }
    }
    let mut m = module(CART_FLAG_INT32_SAFE);
    if let Chunk::Bytecode(bc) = &mut m.functions[0] {
        bc.constants.push(Constant::from_le_bytes(0x0300u64.to_le_bytes()));
        bc.code = vec![OpCode::PushConst(r(0), 0), OpCode::PushConst(r(1), 1), OpCode::Deo(r(0), r(1)), OpCode::Dei(r(2), r(1)), OpCode::Ret(r(2))];
        bc.reg_count = 3;
    }
    let mut vm = narrow::VirtualMachine::new(); vm.install_device(3, Box::new(Echo(0)));
    assert_eq!(vm.run_module(&m).unwrap().raw(), i32::MAX as u32);
    use narrow::AotNatives;
    let mut host = narrow::AotHost::new(); let mut heap = narrow::Heap::new();
    let (word, tag): (u32, bool) = host.call("__int_abs", &mut heap, &[narrow::Value::from_int(i32::MIN)]).unwrap();
    assert_eq!(word, i32::MIN as u32); assert!(!tag);
    assert!(host.with_int32(false).call("__int_abs", &mut heap, &[narrow::Value::ZERO]).is_err());
}
