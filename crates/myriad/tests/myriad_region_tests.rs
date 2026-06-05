// Integration tests for the RegionTable runtime.

use myriad::{Heap, RegionTable, Value, VirtualMachine};
use polka::{BytecodeChunk, Chunk, OpCode, Register};

#[cfg(test)]
mod tests {
    use super::*;

    fn r(n: u8) -> Register { Register(n) }
    fn vm() -> VirtualMachine { VirtualMachine::new() }

    fn raw_constants(consts: Vec<Value>) -> Vec<u64> {
        consts.into_iter().map(|v| v.raw()).collect()
    }

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
        v.region_pop().expect("pop ok");
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
        rt.pop_and_release(&mut heap).unwrap();
        assert_eq!(rt.depth(), 1);
        rt.pop_and_release(&mut heap).unwrap();
        assert_eq!(rt.depth(), 0);
    }

    #[test]
    fn pop_without_push_errs() {
        let mut rt = RegionTable::new();
        let mut heap = Heap::new();
        assert!(rt.pop_and_release(&mut heap).is_err());
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
        let (s1, g1) = heap.alloc(1);
        let (s2, g2) = heap.alloc(1);
        assert_eq!(heap.live_count(), 2);

        let mut rt = RegionTable::new();
        rt.push();
        rt.record_alloc(s1, g1);
        rt.record_alloc(s2, g2);
        rt.pop_and_release(&mut heap).unwrap();

        assert_eq!(heap.live_count(), 0, "region exit must force-free both slots");
    }

    #[test]
    fn region_force_free_cascades_to_nested_handle() {
        // A region-tracked cell holding a child handle must rc_dec the child
        // when force-freed. With child rc=1 (single ref from parent), the
        // cascade reclaims it.
        let mut heap = Heap::new();
        let (child, cg) = heap.alloc(1);
        let (parent, pg) = heap.alloc(1);
        heap.st(parent, pg, 0, Value::from_handle(child, cg).raw(), true).unwrap();

        let mut rt = RegionTable::new();
        rt.push();
        rt.record_alloc(parent, pg);
        rt.pop_and_release(&mut heap).unwrap();

        assert_eq!(heap.live_count(), 0, "parent + child both reclaimed");
    }

    #[test]
    fn rc_dec_cascades_to_handle_child() {
        let mut heap = Heap::new();
        let (child, cg) = heap.alloc(1);
        let (parent, pg) = heap.alloc(1);
        heap.st(parent, pg, 0, Value::from_handle(child, cg).raw(), true).unwrap();

        let _ = heap.rc_dec(parent, pg).unwrap();
        assert_eq!(heap.live_count(), 0, "rc=0 cascade must reclaim handle child");
    }

    #[test]
    fn test_region_pop_force_frees_alloc_inside() {
        let mut vm = VirtualMachine::new();
        let chunk = Chunk::Bytecode(BytecodeChunk {
        src_file: String::new(),
            code: vec![
                OpCode::PushConst(r(0), 0),
                OpCode::PushConst(r(1), 1),
                OpCode::Deo(r(0), r(1)),
                OpCode::Alloc(r(2), 4),
                OpCode::PushConst(r(3), 2),
                OpCode::Deo(r(0), r(3)),
                OpCode::PushConst(r(4), 3),
                OpCode::Ret(r(4)),
            ],
            constants: raw_constants(vec![
                Value::from_int(0),
                Value::from_int(region_port(polka::REGION_PORT_PUSH)),
                Value::from_int(region_port(polka::REGION_PORT_POP)),
                Value::from_int(99),
            ]),
            const_mask: Vec::new(),
            string_constants: vec![],
            reg_count: 8,
            param_count: 0,
            lines: vec![],
        });
        let result = vm.run(&chunk).expect("region push/pop should not error");
        assert_eq!(result, Value::from_int(99));
        assert_eq!(vm.heap_live_count(), 0, "alloc inside region must be force-freed at pop");
    }

    #[test]
    fn test_region_pop_frees_multiple_allocs() {
        let mut vm = VirtualMachine::new();
        let chunk = Chunk::Bytecode(BytecodeChunk {
        src_file: String::new(),
            code: vec![
                OpCode::PushConst(r(0), 0),
                OpCode::PushConst(r(1), 1),
                OpCode::Deo(r(0), r(1)),
                OpCode::Alloc(r(2), 2),
                OpCode::Alloc(r(3), 2),
                OpCode::Alloc(r(4), 2),
                OpCode::PushConst(r(5), 2),
                OpCode::Deo(r(0), r(5)),
                OpCode::PushConst(r(6), 3),
                OpCode::Ret(r(6)),
            ],
            constants: raw_constants(vec![
                Value::from_int(0),
                Value::from_int(region_port(polka::REGION_PORT_PUSH)),
                Value::from_int(region_port(polka::REGION_PORT_POP)),
                Value::from_int(7),
            ]),
            const_mask: Vec::new(),
            string_constants: vec![],
            reg_count: 8,
            param_count: 0,
            lines: vec![],
        });
        vm.run(&chunk).unwrap();
        assert_eq!(vm.heap_live_count(), 0, "all allocs in region must be freed");
    }

    #[test]
    fn test_alloc_outside_region_is_not_force_freed() {
        let mut vm = VirtualMachine::new();
        let chunk = Chunk::Bytecode(BytecodeChunk {
        src_file: String::new(),
            code: vec![
                OpCode::Alloc(r(0), 2),
                OpCode::PushConst(r(1), 0),
                OpCode::Ret(r(1)),
            ],
            constants: raw_constants(vec![Value::from_int(5)]),
            const_mask: Vec::new(),
            string_constants: vec![],
            reg_count: 4,
            param_count: 0,
            lines: vec![],
        });
        vm.run(&chunk).unwrap();
        assert_eq!(vm.heap_live_count(), 1, "alloc outside region survives end of execution");
    }

    #[test]
    fn test_nested_regions_pop_inner_only() {
        let mut vm = VirtualMachine::new();
        let chunk = Chunk::Bytecode(BytecodeChunk {
        src_file: String::new(),
            code: vec![
                OpCode::PushConst(r(0), 0),
                OpCode::PushConst(r(1), 1),
                OpCode::Deo(r(0), r(1)),
                OpCode::Alloc(r(2), 2),
                OpCode::Deo(r(0), r(1)),
                OpCode::Alloc(r(3), 2),
                OpCode::PushConst(r(4), 2),
                OpCode::Deo(r(0), r(4)),
                OpCode::PushConst(r(5), 3),
                OpCode::Ret(r(5)),
            ],
            constants: raw_constants(vec![
                Value::from_int(0),
                Value::from_int(region_port(polka::REGION_PORT_PUSH)),
                Value::from_int(region_port(polka::REGION_PORT_POP)),
                Value::from_int(0),
            ]),
            const_mask: Vec::new(),
            string_constants: vec![],
            reg_count: 8,
            param_count: 0,
            lines: vec![],
        });
        vm.run(&chunk).unwrap();
        assert_eq!(vm.region_depth(), 1, "outer region remains after inner pop");
        assert_eq!(vm.heap_live_count(), 1, "outer alloc survives; only inner alloc was freed");
    }

    #[test]
    fn test_region_pop_without_push_errors() {
        let mut vm = VirtualMachine::new();
        let chunk = Chunk::Bytecode(BytecodeChunk {
        src_file: String::new(),
            code: vec![
                OpCode::PushConst(r(0), 0),
                OpCode::PushConst(r(1), 1),
                OpCode::Deo(r(0), r(1)),
                OpCode::Ret(r(0)),
            ],
            constants: raw_constants(vec![
                Value::from_int(0),
                Value::from_int(region_port(polka::REGION_PORT_POP)),
            ]),
            const_mask: Vec::new(),
            string_constants: vec![],
            reg_count: 4,
            param_count: 0,
            lines: vec![],
        });
        let err = vm.run(&chunk).expect_err("region pop with empty stack must error");
        assert!(err.contains("no active region"), "got error: {}", err);
    }

    // i48 limit is gone; sub-imm of any i64 just wraps as plain u64.
    #[test]
    fn test_sub_imm_full_i64_range() {
        let start = i64::MIN + 5;
        let mut vm = VirtualMachine::new();
        let chunk = Chunk::Bytecode(BytecodeChunk {
        src_file: String::new(),
            code: vec![
                OpCode::PushConst(r(0), 0),
                OpCode::SubImm(r(1), r(0), 5),
                OpCode::Ret(r(1)),
            ],
            constants: raw_constants(vec![Value::from_int(start)]),
            const_mask: Vec::new(),
            string_constants: vec![],
            reg_count: 2,
            param_count: 0,
            lines: vec![],
        });
        let result = vm.run(&chunk).expect("vm should not panic on i64 underflow");
        let expected = start.wrapping_sub(5);
        assert_eq!(result.as_int(), expected);
    }
}
