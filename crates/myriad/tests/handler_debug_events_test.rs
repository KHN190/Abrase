use polka::{BytecodeChunk, Chunk, Constant, Module, OpCode, Register};

fn r(n: u8) -> Register {
    Register(n)
}

fn nested_handlers(flags: u16) -> Module {
    let pop = ((polka::DISPATCH_ID as u64) << 8) | polka::DISPATCH_PORT_POP_HANDLER as u64;
    Module {
        functions: vec![Chunk::Bytecode(BytecodeChunk {
            code: vec![
                OpCode::Alloc(r(0), 1),
                OpCode::PushConst(r(1), 0),
                OpCode::St(r(1), r(0), 0),
                OpCode::Handle(r(0), 7),
                OpCode::PushConst(r(2), 1),
                OpCode::Handle(r(2), 9),
                OpCode::PushConst(r(3), 2),
                OpCode::Deo(r(2), r(3)),
                OpCode::Handle(r(0), 11),
                OpCode::Deo(r(2), r(3)),
                OpCode::Deo(r(2), r(3)),
                OpCode::Drop(r(0)),
                OpCode::PushConst(r(0), 0),
                OpCode::Ret(r(0)),
            ],
            constants: [99u64, 0, pop]
                .map(|n| Constant::from_le_bytes(n.to_le_bytes()))
                .to_vec(),
            reg_count: 4,
            ..BytecodeChunk::default()
        })],
        entry: 0,
        flags,
        exports: vec![],
    }
}

macro_rules! check_profile {
    ($name:ident, $runtime:path, $flags:expr) => {
        mod $name {
            use super::*;
            use std::{cell::RefCell, rc::Rc};
            use $runtime as runtime;

            #[test]
            fn each_successful_push_reports_effect_and_post_push_depth() {
                let module = nested_handlers($flags);
                let seen = Rc::new(RefCell::new(Vec::new()));
                let tables = Rc::new(RefCell::new(Vec::new()));
                let capture_tables = Rc::clone(&tables);
                let capture = Rc::clone(&seen);
                let mut traced =
                    runtime::VirtualMachine::new().with_debug_sink(Box::new(move |event, _| {
                        match event {
                            runtime::DebugEvent::HandlePush {
                                effect_id,
                                depth,
                                dispatch_table,
                            } => {
                                capture.borrow_mut().push((*effect_id, *depth));
                                capture_tables.borrow_mut().push(*dispatch_table);
                            }
                            _ => {}
                        }
                    }));
                let mut plain = runtime::VirtualMachine::new();
                let result = traced.run_module(&module).unwrap();
                assert_eq!(result.as_int(), 99);
                assert_eq!(result, plain.run_module(&module).unwrap());
                assert_eq!(traced.steps(), plain.steps());
                assert_eq!(traced.heap_live_count(), 0);
                assert_eq!(plain.heap_live_count(), 0);
                assert_eq!(*seen.borrow(), [(7, 1), (9, 2), (11, 2)]);
                let tables = tables.borrow();
                assert!(tables[0].is_some());
                assert_eq!(tables[1], None);
                assert_eq!(tables[2], tables[0]);
            }

            #[test]
            fn push_events_are_independent_of_opcode_trace_filtering() {
                let pushes = Rc::new(RefCell::new(0));
                let capture = Rc::clone(&pushes);
                let mut vm = runtime::VirtualMachine::new()
                    .with_trace_filter(vec![false])
                    .with_debug_sink(Box::new(move |event, _| match event {
                        runtime::DebugEvent::HandlePush { .. } => *capture.borrow_mut() += 1,
                        runtime::DebugEvent::Trace { .. } => {
                            panic!("opcode trace must be filtered")
                        }
                        _ => {}
                    }));
                assert_eq!(
                    vm.run_module(&nested_handlers($flags)).unwrap().as_int(),
                    99
                );
                assert_eq!(*pushes.borrow(), 3);
            }

            #[test]
            fn rejected_modules_emit_no_push() {
                let seen = Rc::new(RefCell::new(0));
                let capture = Rc::clone(&seen);
                let mut vm =
                    runtime::VirtualMachine::new().with_debug_sink(Box::new(move |event, _| {
                        if let runtime::DebugEvent::HandlePush { .. } = event {
                            *capture.borrow_mut() += 1;
                        }
                    }));
                let module = nested_handlers($flags ^ polka::CART_FLAG_INT32_SAFE);
                assert!(vm.run_module(&module).is_err());
                assert_eq!(*seen.borrow(), 0);
            }
        }
    };
}

check_profile!(word32, myriad::word32, polka::CART_FLAG_INT32_SAFE);
check_profile!(word64, myriad::word64, 0);
