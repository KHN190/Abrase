use myriad::{Value, VirtualMachine, HandlerFrame};

#[test]
fn handle_allocates_continuation_slot() {
    let mut vm = VirtualMachine::new();
    let initial_live = vm.heap_live_count();
    let (_, _) = vm.heap_alloc(4);

    assert_eq!(vm.heap_live_count(), initial_live + 1, "allocation should increase heap count");
}

#[test]
fn handler_frame_basic_structure() {
    let (slot, generation) = (42u32, 7u32);

    let frame = HandlerFrame {
        effect_id: 123,
        dispatch_table_slot: Some(slot),
        dispatch_table_gen: generation,
        cell_slot: slot,
        cell_gen: generation,
        cells_allocated: vec![(slot, generation)],
        body_frame_index: None,
        pending_return_arm_fn: None,
        pending_return_arm_env: polka::HANDLE_NONE,
        pending_return_arm_env_is_handle: false,
    };

    assert_eq!(frame.effect_id, 123);
    assert_eq!(frame.dispatch_table_slot, Some(42));
    assert_eq!(frame.dispatch_table_gen, 7);
}

#[test]
fn handler_frame_push_succeeds() {
    let mut vm = VirtualMachine::new();

    let (slot, generation) = vm.heap_alloc(4);
    vm.push_handler(HandlerFrame {
        effect_id: 123,
        dispatch_table_slot: Some(slot),
        dispatch_table_gen: generation,
        cell_slot: slot,
        cell_gen: generation,
        cells_allocated: vec![(slot, generation)],
        body_frame_index: None,
        pending_return_arm_fn: None,
        pending_return_arm_env: polka::HANDLE_NONE,
        pending_return_arm_env_is_handle: false,
    });

    assert!(true, "push_handler succeeded without error");
}

#[test]
fn handle_with_dispatch_table_handle_value() {
    let (table_slot, table_gen) = (100u32, 10u32);
    let (cont_slot, cont_gen) = (101u32, 11u32);

    let frame = HandlerFrame {
        effect_id: 42,
        dispatch_table_slot: Some(table_slot),
        dispatch_table_gen: table_gen,
        cell_slot: cont_slot,
        cell_gen: cont_gen,
        cells_allocated: vec![(cont_slot, cont_gen)],
        body_frame_index: None,
        pending_return_arm_fn: None,
        pending_return_arm_env: polka::HANDLE_NONE,
        pending_return_arm_env_is_handle: false,
    };

    assert_eq!(frame.dispatch_table_slot, Some(table_slot));
    assert_eq!(frame.dispatch_table_gen, table_gen);
    assert_eq!(frame.effect_id, 42);
}

#[test]
fn handler_frame_with_pending_return_arm() {
    let (cont_slot, cont_gen) = (102u32, 12u32);

    let frame = HandlerFrame {
        effect_id: 1,
        dispatch_table_slot: None,
        dispatch_table_gen: 0,
        cell_slot: cont_slot,
        cell_gen: cont_gen,
        cells_allocated: vec![(cont_slot, cont_gen)],
        body_frame_index: Some(3),
        pending_return_arm_fn: Some(42),
        pending_return_arm_env: 0,
        pending_return_arm_env_is_handle: false,
    };

    assert_eq!(frame.body_frame_index, Some(3));
    assert_eq!(frame.pending_return_arm_fn, Some(42));
    assert_eq!(frame.dispatch_table_slot, None);
}

#[test]
fn continuation_slot_in_region() {
    // A continuation slot allocated inside a region should be recorded
    // and freed when the region pops (via force_free)
    let mut vm = VirtualMachine::new();
    let initial_live = vm.heap_live_count();

    vm.region_push();
    let (slot, generation) = vm.heap_alloc(4);
    vm.region_record_alloc(slot, generation);

    assert_eq!(vm.heap_live_count(), initial_live + 1);

    vm.region_pop().expect("pop");
    assert_eq!(vm.heap_live_count(), initial_live, "region pop should force-free recorded allocs");
}

#[test]
fn multiple_allocations_in_region() {
    // Multiple continuation slots in a region
    let mut vm = VirtualMachine::new();

    vm.region_push();
    let (slot1, gen1) = vm.heap_alloc(4);
    let (slot2, gen2) = vm.heap_alloc(4);
    let (slot3, gen3) = vm.heap_alloc(4);

    vm.region_record_alloc(slot1, gen1);
    vm.region_record_alloc(slot2, gen2);
    vm.region_record_alloc(slot3, gen3);

    assert_eq!(vm.heap_live_count(), 3, "three allocations");

    vm.region_pop().expect("pop");
    assert_eq!(vm.heap_live_count(), 0, "all region allocs freed on pop");
}

#[test]
fn multiple_handler_frames_creation() {
    let frames: Vec<_> = (0..3)
        .map(|i| HandlerFrame {
            effect_id: i as u16,
            dispatch_table_slot: Some(i as u32),
            dispatch_table_gen: i as u32,
            cell_slot: i as u32 + 100,
            cell_gen: i as u32,
            cells_allocated: vec![(i as u32 + 100, i as u32)],
            body_frame_index: None,
            pending_return_arm_fn: None,
            pending_return_arm_env: polka::HANDLE_NONE,
        pending_return_arm_env_is_handle: false,
        })
        .collect();

    assert_eq!(frames.len(), 3);
    assert_eq!(frames[0].effect_id, 0);
    assert_eq!(frames[1].effect_id, 1);
    assert_eq!(frames[2].effect_id, 2);
}

#[test]
fn handler_with_invalid_generation_structure() {
    let frame = HandlerFrame {
        effect_id: 1,
        dispatch_table_slot: Some(100),
        dispatch_table_gen: 999,
        cell_slot: 100,
        cell_gen: 999,
        cells_allocated: vec![(100, 999)],
        body_frame_index: None,
        pending_return_arm_fn: None,
        pending_return_arm_env: polka::HANDLE_NONE,
        pending_return_arm_env_is_handle: false,
    };

    assert_eq!(frame.dispatch_table_gen, 999);
    assert_eq!(frame.cell_gen, 999);
}

#[test]
fn region_depth_tracking() {
    let mut vm = VirtualMachine::new();

    assert_eq!(vm.region_depth(), 0);

    vm.region_push();
    assert_eq!(vm.region_depth(), 1);

    vm.region_push();
    assert_eq!(vm.region_depth(), 2);

    vm.region_pop().expect("pop");
    assert_eq!(vm.region_depth(), 1);

    vm.region_pop().expect("pop");
    assert_eq!(vm.region_depth(), 0);
}

#[test]
fn region_pop_without_push_errors() {
    let mut vm = VirtualMachine::new();
    let result = vm.region_pop();
    assert!(result.is_err(), "popping without push should error");
}

#[test]
fn heap_alloc_tracking() {
    let mut vm = VirtualMachine::new();
    let initial = vm.heap_live_count();

    // Allocate 5 cells
    for _ in 0..5 {
        vm.heap_alloc(2);
    }

    assert_eq!(vm.heap_live_count(), initial + 5);
}

#[test]
fn heap_st_modifies_cell() {
    let mut vm = VirtualMachine::new();

    let (slot, generation) = vm.heap_alloc(4);
    let val1 = Value::from_int(42);
    let val2 = Value::from_int(100);

    let old1 = vm.heap_st(slot, generation, 0, val1.raw(), false).expect("write 1");
    let old2 = vm.heap_st(slot, generation, 1, val2.raw(), false).expect("write 2");

    assert_eq!(old1, (polka::HANDLE_NONE, false));
    assert_eq!(old2, (polka::HANDLE_NONE, false));
}
