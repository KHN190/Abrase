use myriad::word32::{AotHost, AotNatives, DebugEvent, Heap, Value, VirtualMachine};
use polka::{BytecodeChunk, Chunk, Constant, Export, Module, NativeChunk, OpCode, Register, CART_FLAG_INT32_SAFE};
use std::{cell::RefCell, rc::Rc};

fn r(n: u8) -> Register { Register(n) }
fn constant(word: u32) -> Constant {
    let mut bytes = [0; 8]; bytes[..4].copy_from_slice(&word.to_le_bytes());
    Constant::from_le_bytes(bytes)
}
fn chunk(code: Vec<OpCode>, words: &[u32], regs: usize, params: usize) -> Chunk {
    Chunk::Bytecode(BytecodeChunk { code, constants: words.iter().copied().map(constant).collect(),
        reg_count: regs, param_count: params, ..BytecodeChunk::default() })
}
fn module(code: Vec<OpCode>, words: &[u32], regs: usize) -> Module {
    Module { functions: vec![chunk(code, words, regs, 0)], entry: 0, flags: CART_FLAG_INT32_SAFE,
        exports: vec![Export { name: "entry".into(), fn_id: 0 }] }
}

#[test]
fn storage_and_trace_use_real_four_byte_words_and_masks() {
    assert_eq!(std::mem::size_of::<Value>(), 4);
    let observed = Rc::new(RefCell::new(false));
    let seen = observed.clone();
    let mut vm = VirtualMachine::new().with_debug_sink(Box::new(move |event, _| {
        if let DebugEvent::Trace { window, handle_mask, .. } = event {
            let _: &[u32] = window;
            let _: &[u32; 4] = handle_mask;
            if window.len() == 128 && handle_mask[3] & (1 << 31) != 0 { *seen.borrow_mut() = true; }
        }
    }));
    let m = module(vec![OpCode::Alloc(r(127), 1), OpCode::PushConst(r(0), 0),
        OpCode::St(r(0), r(127), 0), OpCode::Ld(r(1), r(127), 0),
        OpCode::Drop(r(127)), OpCode::Ret(r(1))], &[u32::MAX], 128);
    let value = vm.run_module(&m).unwrap();
    let _: u32 = value.raw();
    let _: u32 = vm.steps();
    let _: u32 = vm.heap_alloc_count();
    assert_eq!(value.as_int(), -1);
    assert!(*observed.borrow());
    assert_eq!(vm.heap_live_count(), 0);
}

#[test]
fn every_module_entry_rejects_the_wide_profile() {
    let mut m = module(vec![OpCode::Ret(r(0))], &[], 1);
    m.flags = 0;
    let mut vm = VirtualMachine::new();
    for error in [vm.run_module(&m).unwrap_err(), vm.call_export(&m, "entry", &[]).unwrap_err(),
        vm.run_to_yield(&m).unwrap_err(), vm.resume(&m, Value::ZERO).unwrap_err(),
        myriad::word32::loader::load(m).err().unwrap()] {
        assert!(error.contains("INT32_SAFE"), "{error}");
    }
}

#[test]
fn raw_chunk_entry_uses_the_compiled_word_profile() {
    let m = module(vec![OpCode::PushConst(r(0), 0), OpCode::AddImm(r(1), r(0), 1), OpCode::Ret(r(1))], &[i32::MAX as u32], 2);
    assert_eq!(VirtualMachine::new().run(&m.functions[0]).unwrap().as_int(), i32::MIN);
}

#[test]
fn native_and_aot_host_abi_use_four_byte_values() {
    let mut m = module(vec![OpCode::PushConst(r(2), 0), OpCode::Call(r(0), 1), OpCode::Ret(r(0))], &[i32::MIN as u32], 2);
    m.functions.push(Chunk::Native(NativeChunk { name: "assert_word32".into(), param_count: 1 }));
    let mut vm = VirtualMachine::new();
    vm.register_native("assert_word32", Rc::new(|ctx, args| {
        assert!(ctx.int32_safe);
        assert_eq!(std::mem::size_of_val(&args[0]), 4);
        let value: i32 = args[0].as_int();
        Ok((Value::from_int(value.wrapping_sub(1)), false))
    }));
    assert_eq!(vm.run_module(&m).unwrap().as_int(), i32::MAX);
    let mut host = AotHost::new(); let mut heap = Heap::new();
    let (word, tag): (u32, bool) = host.call("__int_abs", &mut heap, &[Value::from_int(i32::MIN)]).unwrap();
    assert_eq!(word, i32::MIN as u32); assert!(!tag);
    assert!(AotHost::new().with_int32(false).call("__int_abs", &mut heap, &[Value::ZERO]).unwrap_err().contains("INT32_SAFE"));
}

#[test]
fn yield_resume_preserves_float_and_integer_words() {
    let mut m = module(vec![OpCode::PushConst(r(0), 0), OpCode::PushConst(r(32), 1),
        OpCode::Call(r(1), 1), OpCode::FAdd(r(2), r(0), r(0)),
        OpCode::AddImm(r(33), r(32), 1), OpCode::Ret(r(33))], &[0.5f32.to_bits(), i32::MAX as u32], 34);
    m.functions.push(Chunk::Native(NativeChunk { name: "__frame_present".into(), param_count: 0 }));
    let seen = Rc::new(RefCell::new(None)); let capture = seen.clone();
    let mut vm = VirtualMachine::new().with_debug_sink(Box::new(move |event, _| {
        if let DebugEvent::Trace { op: OpCode::Ret(_), window, .. } = event {
            *capture.borrow_mut() = Some((window[2], window[33]));
        }
    }));
    vm.run_to_yield(&m).unwrap();
    assert!(!vm.resume(&m, Value::ZERO).unwrap());
    assert_eq!(*seen.borrow(), Some((1.0f32.to_bits(), i32::MIN as u32)));
}

#[test]
fn checked_step_cap_never_wraps_or_executes_past_limit() {
    let m = module(vec![OpCode::Jmp(-1)], &[], 1);
    let mut vm = VirtualMachine::new().with_step_cap(3);
    assert!(vm.run_module(&m).unwrap_err().contains("step cap exceeded"));
    assert_eq!(vm.steps(), 3);
}

#[test]
fn invalid_wide_constant_is_rejected_before_execution() {
    let mut m = module(vec![OpCode::PushConst(r(0), 0), OpCode::Ret(r(0))], &[0], 1);
    if let Chunk::Bytecode(bc) = &mut m.functions[0] {
        bc.constants[0] = Constant::from_le_bytes([0,0,0,0,1,0,0,0]);
    }
    assert!(VirtualMachine::new().run_module(&m).unwrap_err().contains("constant exceeds"));
}

#[test]
fn effect_resume_restores_word32_snapshots_and_handle_masks_across_words() {
    let main = chunk(vec![
        OpCode::Alloc(r(0), 2), OpCode::PushConst(r(1), 0), OpCode::St(r(1), r(0), 0),
        OpCode::PushConst(r(1), 1), OpCode::St(r(1), r(0), 1), OpCode::Handle(r(0), 7),
        OpCode::Call(r(1), 1), OpCode::PushConst(r(2), 1), OpCode::PushConst(r(3), 2),
        OpCode::Deo(r(2), r(3)), OpCode::Drop(r(0)), OpCode::Ret(r(1)),
    ], &[2, 0, ((polka::DISPATCH_ID as u32) << 8) | polka::DISPATCH_PORT_POP_HANDLER as u32], 4, 0);
    let body = chunk(vec![
        OpCode::Alloc(r(64), 1), OpCode::PushConst(r(3), 3), OpCode::St(r(3), r(64), 0),
        OpCode::PushConst(r(32), 2), OpCode::PushConst(r(1), 0), OpCode::PushConst(r(2), 1),
        OpCode::Raise(r(4), r(1), r(2)), OpCode::FAdd(r(33), r(32), r(32)),
        OpCode::Ld(r(65), r(64), 0), OpCode::Drop(r(64)), OpCode::AddImm(r(4), r(4), 1),
        OpCode::Ret(r(4)),
    ], &[7 << 8, 41, 0.5f32.to_bits(), i32::MIN as u32], 66, 0);
    let arm = chunk(vec![OpCode::AddImm(r(2), r(2), 1), OpCode::Resume(r(3), r(2)), OpCode::Ret(r(3))], &[], 4, 3);
    let m = Module { functions: vec![main, body, arm], entry: 0, flags: CART_FLAG_INT32_SAFE, exports: vec![] };
    let seen = Rc::new(RefCell::new(false)); let capture = seen.clone();
    let mut vm = VirtualMachine::new().with_heap_check(true).with_debug_sink(Box::new(move |event, _| {
        if let DebugEvent::Trace { func: 1, op: OpCode::Ret(_), window, .. } = event {
            assert_eq!(window[33], 1.0f32.to_bits());
            assert_eq!(window[65], i32::MIN as u32);
            *capture.borrow_mut() = true;
        }
    }));
    assert_eq!(vm.run_module(&m).unwrap().as_int(), 43);
    assert!(*seen.borrow());
    assert_eq!(vm.heap_live_count(), 0);
}

#[test]
fn wire_string_and_bytes_constants_resolve_to_runtime_words() {
    for bytes in [false, true] {
        let mut m = module(vec![OpCode::PushConst(r(0), 0), OpCode::Ret(r(0))], &[0], 1);
        if let Chunk::Bytecode(bc) = &mut m.functions[0] {
            bc.const_mask = vec![1];
            if bytes {
                bc.constants[0] = Constant::from_le_bytes([0,0,0,0,0,0,0,0x80]);
                bc.bytes_constants.push(vec![1,2,3,4,5,6,7]);
            } else { bc.string_constants.push("seven字".into()); }
        }
        let mut vm = VirtualMachine::new(); let value = vm.run_module(&m).unwrap();
        if bytes { assert_eq!(myriad::word32::read_bytes(vm.heap_ref(), value).unwrap(), vec![1,2,3,4,5,6,7]); }
        else { assert_eq!(myriad::word32::read_string(vm.heap_ref(), value).unwrap(), "seven字"); }
        vm.drop_result_for_test(value.raw()); vm.reset(); assert_eq!(vm.heap_live_count(), 0);
    }
}
