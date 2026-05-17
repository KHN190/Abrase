// Integration tests for the RegionTable runtime.

use myriad::{BoxPool, BoxedValue, Heap, RegionTable, Value, VirtualMachine};
use polka::{BytecodeChunk, Chunk, OpCode, Register};

#[cfg(test)]
mod tests {
    use super::*;

    fn r(n: u8) -> Register { Register(n) }
    fn vm() -> VirtualMachine { VirtualMachine::new() }

    fn region_port(port: u8) -> i64 {
        ((polka::REGION_ID as i64) << 8) | port as i64
    }

    #[test]
    fn region_depth_starts_at_zero() {
        assert_eq!(vm().region_depth(), 0);
    }

    #[test]
    fn region_push_increments_depth() {
        let mut v = vm();
        v.region_push();
        assert_eq!(v.region_depth(), 1);
        v.region_push();
        assert_eq!(v.region_depth(), 2);
    }

    #[test]
    fn region_pop_without_push_errors() {
        let mut v = vm();
        assert!(v.region_pop().is_err());
    }

    #[test]
    fn region_pop_ignores_alloc_not_recorded() {
        let mut v = vm();
        v.region_push();
        let _ = v.heap_alloc(2);
         // recorded by VM only when going through OpCode::Alloc
        v.region_pop().expect("pop ok");
        // direct heap_alloc isn't recorded — alloc survives.
        assert_eq!(v.heap_live_count(), 1);
    }

    #[test]
    fn push_pop_balances() {
        let mut rt = RegionTable::new();
        assert_eq!(rt.depth(), 0);
        rt.push();
        rt.push();
        assert_eq!(rt.depth(), 2);
        let mut heap = Heap::new();
        let mut pool = BoxPool::new();
        rt.pop_and_release(&mut heap, &mut pool).unwrap();
        assert_eq!(rt.depth(), 1);
        rt.pop_and_release(&mut heap, &mut pool).unwrap();
        assert_eq!(rt.depth(), 0);
    }

    #[test]
    fn pop_without_push_errs() {
        let mut rt = RegionTable::new();
        let mut heap = Heap::new();
        let mut pool = BoxPool::new();
        assert!(rt.pop_and_release(&mut heap, &mut pool).is_err());
    }

    #[test]
    fn record_outside_region_is_silent() {
        let mut rt = RegionTable::new();
        rt.record_alloc(7, 0);
        assert_eq!(rt.depth(), 0);
    }

    #[test]
    fn region_force_frees_recorded_slots() {
        let mut heap = Heap::new();
        let mut pool = BoxPool::new();
        let (s1, g1) = heap.alloc(1);
        let (s2, g2) = heap.alloc(1);
        assert_eq!(heap.live_count(), 2);

        let mut rt = RegionTable::new();
        rt.push();
        rt.record_alloc(s1, g1);
        rt.record_alloc(s2, g2);
        rt.pop_and_release(&mut heap, &mut pool).unwrap();

        assert_eq!(heap.live_count(), 0, "region exit must force-free both slots");
    }

    #[test]
    fn region_force_free_cascades_to_box_pool() {
        let mut heap = Heap::new();
        let mut pool = BoxPool::new();
        let box_idx = pool.intern(BoxedValue::String("hi".into()));
        assert_eq!(pool.live_count(), 1);

        let (slot, gen_) = heap.alloc(1);
        heap.st(slot, gen_, 0, Value::from_box(box_idx)).unwrap();

        let mut rt = RegionTable::new();
        rt.push();
        rt.record_alloc(slot, gen_);
        rt.pop_and_release(&mut heap, &mut pool).unwrap();

        assert_eq!(heap.live_count(), 0, "cell freed");
        assert_eq!(pool.live_count(), 0, "boxed String inside cell must also be reclaimed");
    }

    #[test]
    fn rc_dec_cascades_to_box_pool() {
        let mut heap = Heap::new();
        let mut pool = BoxPool::new();
        let box_idx = pool.intern(BoxedValue::String("hi".into()));

        let (slot, gen_) = heap.alloc(1);
        heap.st(slot, gen_, 0, Value::from_box(box_idx)).unwrap();
        assert_eq!(pool.live_count(), 1);

        // ordinary rc=0 drop
        let _ = heap.rc_dec(slot, gen_, &mut pool).unwrap();
        assert_eq!(heap.live_count(), 0);
        assert_eq!(pool.live_count(), 0, "rc=0 cascade must reclaim Box child");
    }

    #[test]
    fn test_region_pop_force_frees_alloc_inside() {
        // region push; alloc(4); region pop → heap_live_count should drop to 0
        // even though the binding still has rc=1.
        let mut vm = VirtualMachine::new();
        let chunk = Chunk::Bytecode(BytecodeChunk {
            code: vec![
                OpCode::PushConst(r(0), 0),                  // r0 = 0 (deo value)
                OpCode::PushConst(r(1), 1),                  // r1 = push port
                OpCode::Deo(r(0), r(1)),                     // region push
                OpCode::Alloc(r(2), 4),                      // r2 = handle (rc=1)
                OpCode::PushConst(r(3), 2),                  // r3 = pop port
                OpCode::Deo(r(0), r(3)),                     // region pop → force free r2
                OpCode::PushConst(r(4), 3),                  // r4 = return value 99
                OpCode::Ret(r(4)),
            ],
            constants: vec![
                Value::from_int(0),
                Value::from_int(region_port(polka::REGION_PORT_PUSH)),
                Value::from_int(region_port(polka::REGION_PORT_POP)),
                Value::from_int(99),
            ],
            string_constants: vec![],
            reg_count: 8,
            param_count: 0,
        });
        let result = vm.run(&chunk).expect("region push/pop should not error");
        assert_eq!(result, Value::from_int(99));
        assert_eq!(vm.heap_live_count(), 0, "alloc inside region must be force-freed at pop");
    }

    #[test]
    fn test_region_pop_frees_multiple_allocs() {
        let mut vm = VirtualMachine::new();
        let chunk = Chunk::Bytecode(BytecodeChunk {
            code: vec![
                OpCode::PushConst(r(0), 0),
                OpCode::PushConst(r(1), 1),
                OpCode::Deo(r(0), r(1)),       // push
                OpCode::Alloc(r(2), 2),
                OpCode::Alloc(r(3), 2),
                OpCode::Alloc(r(4), 2),
                OpCode::PushConst(r(5), 2),
                OpCode::Deo(r(0), r(5)),       // pop
                OpCode::PushConst(r(6), 3),
                OpCode::Ret(r(6)),
            ],
            constants: vec![
                Value::from_int(0),
                Value::from_int(region_port(polka::REGION_PORT_PUSH)),
                Value::from_int(region_port(polka::REGION_PORT_POP)),
                Value::from_int(7),
            ],
            string_constants: vec![],
            reg_count: 8,
            param_count: 0,
        });
        vm.run(&chunk).unwrap();
        assert_eq!(vm.heap_live_count(), 0, "all allocs in region must be freed");
    }

    #[test]
    fn test_alloc_outside_region_is_not_force_freed() {
        let mut vm = VirtualMachine::new();
        let chunk = Chunk::Bytecode(BytecodeChunk {
            code: vec![
                OpCode::Alloc(r(0), 2),  // no region active → not tracked
                OpCode::PushConst(r(1), 0),
                OpCode::Ret(r(1)),
            ],
            constants: vec![Value::from_int(5)],
            string_constants: vec![],
            reg_count: 4,
            param_count: 0,
        });
        vm.run(&chunk).unwrap();
        // Handle still has rc=1, never dropped → still live.
        assert_eq!(vm.heap_live_count(), 1, "alloc outside region survives end of execution");
    }

    #[test]
    fn test_nested_regions_pop_inner_only() {
        let mut vm = VirtualMachine::new();
        let chunk = Chunk::Bytecode(BytecodeChunk {
            code: vec![
                OpCode::PushConst(r(0), 0),
                OpCode::PushConst(r(1), 1),
                OpCode::Deo(r(0), r(1)),       // outer push
                OpCode::Alloc(r(2), 2),        // belongs to outer
                OpCode::Deo(r(0), r(1)),       // inner push
                OpCode::Alloc(r(3), 2),        // belongs to inner
                OpCode::PushConst(r(4), 2),
                OpCode::Deo(r(0), r(4)),       // inner pop → frees only r3
                OpCode::PushConst(r(5), 3),
                OpCode::Ret(r(5)),
            ],
            constants: vec![
                Value::from_int(0),
                Value::from_int(region_port(polka::REGION_PORT_PUSH)),
                Value::from_int(region_port(polka::REGION_PORT_POP)),
                Value::from_int(0),
            ],
            string_constants: vec![],
            reg_count: 8,
            param_count: 0,
        });
        vm.run(&chunk).unwrap();
        assert_eq!(vm.region_depth(), 1, "outer region remains after inner pop");
        assert_eq!(vm.heap_live_count(), 1, "outer alloc survives; only inner alloc was freed");
    }

    #[test]
    fn test_region_pop_without_push_errors() {
        let mut vm = VirtualMachine::new();
        let chunk = Chunk::Bytecode(BytecodeChunk {
            code: vec![
                OpCode::PushConst(r(0), 0),
                OpCode::PushConst(r(1), 1),    // pop port
                OpCode::Deo(r(0), r(1)),       // pop without push
                OpCode::Ret(r(0)),
            ],
            constants: vec![
                Value::from_int(0),
                Value::from_int(region_port(polka::REGION_PORT_POP)),
            ],
            string_constants: vec![],
            reg_count: 4,
            param_count: 0,
        });
        let err = vm.run(&chunk).expect_err("region pop with empty stack must error");
        assert!(err.contains("no active region"), "got error: {}", err);
    }

    #[test]
    fn test_sub_imm_boxes_overflow_below_i48() {
        let i48_min: i64 = -(1i64 << 47);
        let start = i48_min + 1;
        let mut vm = VirtualMachine::new();
        let chunk = Chunk::Bytecode(BytecodeChunk {
            code: vec![
                OpCode::PushConst(r(0), 0),
                OpCode::SubImm(r(1), r(0), 5),  // start - 5 = i48_min - 4 → outside i48
                OpCode::Ret(r(1)),
            ],
            constants: vec![Value::from_int(start)],
            string_constants: vec![],
            reg_count: 2,
            param_count: 0,
        });
        let result = vm.run(&chunk).expect("vm should not panic on i48 underflow");
        let expected = start.wrapping_sub(5);
        assert_eq!(vm.box_pool().read_int(result), Some(expected));
    }
}
