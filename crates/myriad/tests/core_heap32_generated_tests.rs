#[path = "../src/core_heap32_gen.rs"]
mod core32;
#[path = "support/handle32.rs"]
mod oracle;

fn arena(bytes: usize, slots: u32) -> Vec<u8> {
    let mut arena = vec![0; bytes];
    assert_eq!(core32::core_init(&mut arena, bytes as u32, slots), Ok(0));
    arena
}
fn word(arena: &[u8], off: usize) -> u32 {
    u32::from_le_bytes(arena[off..off + 4].try_into().unwrap())
}

fn assert_no_live_blocks(arena: &[u8]) {
    let mut off = word(arena, 28) as usize;
    let frontier = word(arena, 0) as usize;
    while off < frontier {
        assert_eq!(word(arena, off), 0, "live physical block at {off}");
        let size = word(arena, off + 8) as usize;
        off += 24 + size.div_ceil(32) * 4 + size * 4;
    }
    assert_eq!(off, frontier);
    for slot in 1..=word(arena, 24) as usize {
        assert_eq!(word(arena, 32 + slot * 8 + 4) & (1 << 16), 0);
    }
}

#[test]
fn slot_and_arena_exhaustion_leave_the_other_resource_untouched() {
    let mut a = arena(256, 1);
    let h = core32::alloc(&mut a, 1).unwrap();
    let frontier = word(&a, 0);
    assert_eq!(core32::alloc(&mut a, 1), Ok(0));
    assert_eq!(word(&a, 0), frontier);
    assert_eq!(core32::cell_set(&mut a, h, 0, 73), Ok(0));
    assert_eq!(core32::cell_get(&mut a, h, 0), Ok(73));
    assert_eq!(core32::rc_dec(&mut a, h), Ok(1));
    assert_eq!(core32::alloc(&mut a, 1000), Ok(0));
    assert_eq!(word(&a, 0), frontier);
    let next = core32::alloc(&mut a, 1).unwrap();
    assert_eq!(next & 0xffff, 1);
    assert_eq!(core32::rc_dec(&mut a, next), Ok(1));
    assert_no_live_blocks(&a);
}

#[test]
fn production_generations_retire_and_reuse_physical_block_through_next_slot() {
    let mut a = arena(256, 2);
    let mut oracle = oracle::Handle32Table::new(2, 24).unwrap();
    assert_eq!(oracle.storage_bytes(), 24);
    let mut first = 0;
    let mut offset = 0;
    for generation in 0..=u16::MAX {
        let h = core32::alloc(&mut a, 1).unwrap();
        let off = core32::resolve_offset(&mut a, h).unwrap();
        if generation == 0 {
            first = h;
            offset = off;
        }
        assert_eq!(off, offset);
        assert_eq!(h, oracle.allocate(off).unwrap());
        assert_eq!(core32::rc_dec(&mut a, h), Ok(1));
        assert_eq!(oracle.release(h), Ok(off));
        assert_eq!(core32::resolve_offset(&mut a, first), Ok(u32::MAX));
        assert_eq!(core32::rc_dec(&mut a, h), Ok(u32::MAX));
    }
    let h = core32::alloc(&mut a, 1).unwrap();
    assert_ne!(h >> 16, first >> 16);
    assert_eq!(core32::resolve_offset(&mut a, h), Ok(offset));
    assert_eq!(h, oracle.allocate(offset).unwrap());
    assert_eq!(core32::cell_set(&mut a, h, 0, 91), Ok(0));
    assert_eq!(core32::cell_set(&mut a, first, 0, 0), Ok(u32::MAX));
    assert_eq!(core32::cell_get(&mut a, h, 0), Ok(91));
    assert_eq!(core32::rc_dec(&mut a, h), Ok(1));
    assert_eq!(oracle.release(h), Ok(offset));
    assert_no_live_blocks(&a);
}

#[test]
fn high_bit_handle_is_valid_and_reserved_handles_are_rejected() {
    let mut a = arena(2 * 1024 * 1024, 32768);
    let mut high = 0;
    for _ in 0..32768 {
        high = core32::alloc(&mut a, 0).unwrap();
    }
    assert_ne!(high & (1 << 31), 0);
    assert_ne!(core32::resolve_offset(&mut a, high), Ok(u32::MAX));
    assert_eq!(core32::rc_inc(&mut a, high), Ok(0));
    assert_eq!(core32::rc_dec(&mut a, high), Ok(0));
    assert_eq!(core32::rc_dec(&mut a, high), Ok(1));
    for slot in 1..32768 {
        assert_eq!(core32::rc_dec(&mut a, slot << 16), Ok(1));
    }
    assert_no_live_blocks(&a);
    for h in [0, 1, 0xffff, 0xffff0000, u32::MAX] {
        assert_eq!(core32::resolve_offset(&mut a, h), Ok(u32::MAX));
    }
}

#[test]
fn recursive_drop_crosses_mask_boundaries_without_following_scalar_handle_bits() {
    let mut a = arena(8192, 16);
    let root = core32::alloc(&mut a, 65).unwrap();
    let outside = core32::alloc(&mut a, 1).unwrap();
    core32::cell_set(&mut a, outside, 0, 123).unwrap();
    let mut nested = Vec::new();
    for offset in [31, 32, 63, 64] {
        let child = core32::alloc(&mut a, 65).unwrap();
        let leaf = core32::alloc(&mut a, 1).unwrap();
        assert_eq!(core32::cell_set_child(&mut a, root, offset, child), Ok(0));
        assert_eq!(core32::cell_set_child(&mut a, child, offset, leaf), Ok(0));
        assert_eq!(core32::cell_set(&mut a, child, 0, outside), Ok(0));
        nested.extend([child, leaf]);
    }
    assert_eq!(core32::rc_dec(&mut a, root), Ok(1));
    for h in nested {
        assert_eq!(core32::resolve_offset(&mut a, h), Ok(u32::MAX));
    }
    assert_eq!(core32::cell_get(&mut a, outside, 0), Ok(123));
    assert_eq!(core32::rc_dec(&mut a, outside), Ok(1));
    assert_no_live_blocks(&a);
}

#[test]
fn binding_is_invalidated_before_following_self_and_back_edges() {
    let mut a = arena(512, 2);
    let root = core32::alloc(&mut a, 2).unwrap();
    let child = core32::alloc(&mut a, 1).unwrap();
    core32::cell_set_child(&mut a, root, 0, root).unwrap();
    core32::cell_set_child(&mut a, root, 1, child).unwrap();
    core32::cell_set_child(&mut a, child, 0, root).unwrap();
    assert_eq!(core32::rc_inc(&mut a, root), Ok(0));
    assert_eq!(core32::rc_inc(&mut a, root), Ok(0));
    assert_eq!(core32::force_free(&mut a, root), Ok(1));
    assert_eq!(core32::force_free(&mut a, root), Ok(0));
    assert_eq!(core32::resolve_offset(&mut a, root), Ok(u32::MAX));
    assert_eq!(core32::resolve_offset(&mut a, child), Ok(u32::MAX));
    assert_no_live_blocks(&a);
    let replacement = core32::alloc(&mut a, 2).unwrap();
    let replacement_child = core32::alloc(&mut a, 1).unwrap();
    assert_ne!(replacement, 0);
    assert_ne!(replacement_child, 0);
    assert_eq!(core32::rc_dec(&mut a, replacement), Ok(1));
    assert_eq!(core32::rc_dec(&mut a, replacement_child), Ok(1));
    assert_no_live_blocks(&a);
}

#[test]
fn initialization_rejects_invalid_capacity_and_budget_before_writing() {
    for (bytes, slots) in [(32, 1), (256, 0), (256, 65535), (256, u32::MAX)] {
        let mut a = vec![0; bytes];
        assert_eq!(core32::core_init(&mut a, bytes as u32, slots), Ok(u32::MAX));
        assert!(a.iter().all(|b| *b == 0));
    }
}

#[test]
fn claimed_size_larger_than_actual_slice_traps_without_out_of_bounds_panic() {
    for bytes in [0, 4, 32, 40] {
        let mut arena = vec![0; bytes];
        assert!(core32::core_init(&mut arena, 256, 2).is_err());
    }
}
