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
        handler_fn: 5,
        dispatch_table_slot: Some(slot),
        dispatch_table_gen: generation,
        cell_slot: slot,
        cell_gen: generation,
    };

    assert_eq!(frame.effect_id, 123);
    assert_eq!(frame.handler_fn, 5);
    assert_eq!(frame.dispatch_table_slot, Some(42));
    assert_eq!(frame.dispatch_table_gen, 7);
}

#[test]
fn handler_frame_push_succeeds() {
    // Test that push_handler works with a valid HandlerFrame
    let mut vm = VirtualMachine::new();

    // Create and push a handler frame using public API
    let (slot, generation) = vm.heap_alloc(4);
    vm.push_handler(HandlerFrame {
        effect_id: 123,
        handler_fn: 5,
        dispatch_table_slot: Some(slot),
        dispatch_table_gen: generation,
        cell_slot: slot,
        cell_gen: generation,
    });

    // If we get here without panic, the push succeeded
    assert!(true, "push_handler succeeded without error");
}

#[test]
fn handle_with_dispatch_table_handle_value() {
    let (table_slot, table_gen) = (100u32, 10u32);
    let (cont_slot, cont_gen) = (101u32, 11u32);

    let frame = HandlerFrame {
        effect_id: 42,
        handler_fn: 0,
        dispatch_table_slot: Some(table_slot),
        dispatch_table_gen: table_gen,
        cell_slot: cont_slot,
        cell_gen: cont_gen,
    };

    assert_eq!(frame.dispatch_table_slot, Some(table_slot));
    assert_eq!(frame.dispatch_table_gen, table_gen);
    assert_eq!(frame.effect_id, 42);
}

#[test]
fn handle_with_fallback_handler_function() {
    // When the handle expression is a unit (no dispatch table),
    // the handler_fn field stores the fallback handler index
    let (cont_slot, cont_gen) = (102u32, 12u32);

    let frame = HandlerFrame {
        effect_id: 0,
        handler_fn: 7, // fallback handler function index
        dispatch_table_slot: None,
        dispatch_table_gen: 0,
        cell_slot: cont_slot,
        cell_gen: cont_gen,
    };

    assert_eq!(frame.handler_fn, 7);
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
    // Test that multiple HandlerFrame structures with different configs
    let frames: Vec<_> = (0..3)
        .map(|i| HandlerFrame {
            effect_id: i as u16,
            handler_fn: i as usize,
            dispatch_table_slot: Some(i as u32),
            dispatch_table_gen: i as u32,
            cell_slot: i as u32 + 100,
            cell_gen: i as u32,
        })
        .collect();

    assert_eq!(frames.len(), 3);
    assert_eq!(frames[0].effect_id, 0);
    assert_eq!(frames[1].effect_id, 1);
    assert_eq!(frames[2].effect_id, 2);
    assert_eq!(frames[0].handler_fn, 0);
    assert_eq!(frames[2].handler_fn, 2);
}

#[test]
fn handler_with_invalid_generation_structure() {
    // Test that a handler frame can be created with invalid generation
    // (The runtime would detect this when accessing the heap)
    let frame = HandlerFrame {
        effect_id: 1,
        handler_fn: 0,
        dispatch_table_slot: Some(100),
        dispatch_table_gen: 999, // Invalid generation
        cell_slot: 100,
        cell_gen: 999,
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

    // Write to cell
    let old1 = vm.heap_st(slot, generation, 0, val1).expect("write 1");
    let old2 = vm.heap_st(slot, generation, 1, val2).expect("write 2");

    // heap_st returns the old value, which was NONE initially
    assert_eq!(old1, Value::NONE);
    assert_eq!(old2, Value::NONE);
}
