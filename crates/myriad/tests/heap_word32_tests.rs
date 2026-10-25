use myriad::word32::core_mem::CoreArena;
use myriad::word32::memory::{make_handle, mask_set};
use myriad::word32::{Heap, RegionTable, Value, alloc_bytes, read_bytes};
use polka::word32::{WORD_BYTES, Word};

#[test]
fn default_capacity_supports_existing_wide_object_graph_and_reports_logical_slots() {
    let mut heap = Heap::new();
    assert_eq!(heap.bytes_used(), (1024 + 1) * 8);
    let (root, rg) = heap.alloc(257);
    for offset in 0..257 {
        let (child, cg) = heap.alloc(1);
        heap.st(child, cg, 0, offset as Word, false).unwrap();
        heap.st(root, rg, offset, make_handle(child, cg), true)
            .unwrap();
    }
    let cells = heap.live_cells();
    assert_eq!(cells.len(), 258);
    assert_eq!(cells[0].0, root);
    assert_eq!(cells[0].1, rg);
    assert_eq!(
        core::mem::size_of_val(heap.cell_data(root, rg).unwrap()),
        257 * 4
    );
    assert!(heap.rc_dec(root, rg).unwrap());
    assert_eq!(heap.live_count(), 0);
}

#[test]
fn explicit_table_budget_and_small_compatibility_capacity_are_bounded() {
    for (bytes, slots) in [(64, 1), (1024, 0), (1024, 65535), (usize::MAX, 1)] {
        assert!(Heap::try_with_capacity_and_slots(bytes, slots).is_err());
    }
    let mut heap = Heap::try_with_capacity_and_slots(128, 1).unwrap();
    let (slot, generation) = heap.alloc(1);
    let charged = heap.bytes_used();
    assert!(
        heap.try_alloc(1)
            .unwrap_err()
            .contains("handle slots exhausted")
    );
    assert_eq!(heap.bytes_used(), charged);
    assert_eq!(heap.rc(slot, generation), Some(1));
    let mut tiny = Heap::with_capacity(0);
    let (s, g) = tiny.alloc(0);
    assert!(tiny.try_alloc(0).is_err());
    tiny.force_free(s, g).unwrap();
    assert_eq!(tiny.live_count(), 0);
}

#[test]
fn word_masks_cross_boundaries_without_mistaking_scalar_bits_for_handles() {
    let mut heap = Heap::new();
    let (outside, og) = heap.alloc(1);
    let mut mask = vec![0 as Word; 3];
    for offset in [31, 32, 63, 64] {
        mask_set(&mut mask, offset, true);
    }
    let (root, rg) = heap.alloc_with_mask(65, &mask);
    assert_eq!(heap.cell_mask(root, rg).unwrap(), mask);
    for offset in [31, 32, 63, 64] {
        let (child, cg) = heap.alloc(1);
        let raw = make_handle(child, cg);
        heap.st(root, rg, offset, raw, true).unwrap();
        assert_eq!(heap.ld(root, rg, offset).unwrap(), (raw, true));
    }
    heap.st(root, rg, 0, make_handle(outside, og), false)
        .unwrap();
    assert!(heap.rc_dec(root, rg).unwrap());
    assert_eq!(heap.rc(outside, og), Some(1));
    heap.rc_dec(outside, og).unwrap();
    assert_eq!(heap.live_count(), 0);
}

#[test]
fn clear_preserves_generations_and_retired_slots() {
    let mut heap = Heap::try_with_capacity_and_slots(128, 2).unwrap();
    let (first, first_gen) = heap.alloc(1);
    let charged = heap.bytes_used();
    for generation in 0..=u16::MAX {
        let (slot, actual_gen) = if generation == 0 {
            (first, first_gen)
        } else {
            heap.alloc(1)
        };
        assert_eq!(slot, first);
        assert_eq!(actual_gen, u32::from(generation));
        heap.clear();
        assert!(!heap.is_live(first, first_gen));
        assert!(heap.ld(slot, actual_gen, 0).is_err());
    }
    let (replacement, rg) = heap.alloc(1);
    assert_ne!(replacement, first);
    assert_eq!(heap.bytes_used(), charged);
    heap.st(replacement, rg, 0, 91, false).unwrap();
    heap.force_free(first, first_gen).unwrap();
    assert!(heap.rc_inc(first, first_gen).is_err());
    assert_eq!(heap.ld(replacement, rg, 0), Ok((91, false)));
    heap.clear();
    assert_eq!(heap.live_count(), 0);
}

#[test]
fn region_force_free_and_deep_forget_keep_the_existing_ownership_boundary() {
    let mut heap = Heap::new();
    let mut regions = RegionTable::new();
    regions.push();
    let (outer, og) = heap.alloc(65);
    regions.record_alloc(outer, og);
    regions.push();
    let (moved, mg) = heap.alloc(65);
    let (leaf, lg) = heap.alloc(1);
    let (discarded, dg) = heap.alloc(65);
    regions.record_alloc(moved, mg);
    regions.record_alloc(leaf, lg);
    regions.record_alloc(discarded, dg);
    heap.st(moved, mg, 32, make_handle(leaf, lg), true).unwrap();
    heap.rc_inc(outer, og).unwrap();
    heap.st(moved, mg, 64, make_handle(outer, og), true)
        .unwrap();
    regions.deep_forget(&heap, moved, mg);
    regions.pop_and_release(&mut heap).unwrap();
    assert!(!heap.is_live(discarded, dg));
    assert!(heap.is_live(leaf, lg));
    assert!(heap.is_live(outer, og));
    heap.rc_dec(moved, mg).unwrap();
    assert!(!heap.is_live(leaf, lg));
    assert_eq!(heap.rc(outer, og), Some(1));
    regions.pop_and_release(&mut heap).unwrap();
    assert_eq!(heap.live_count(), 0);
}

#[test]
fn force_free_invalidates_cycles_before_traversing_children() {
    let mut heap = Heap::new();
    let (root, rg) = heap.alloc(65);
    let (child, cg) = heap.alloc(65);
    heap.rc_inc(root, rg).unwrap();
    heap.rc_inc(root, rg).unwrap();
    heap.st(root, rg, 31, make_handle(root, rg), true).unwrap();
    heap.st(root, rg, 64, make_handle(child, cg), true).unwrap();
    heap.st(child, cg, 32, make_handle(root, rg), true).unwrap();
    heap.force_free(root, rg).unwrap();
    assert_eq!(heap.live_count(), 0);
    assert!(!heap.is_live(root, rg));
    assert!(!heap.is_live(child, cg));
}

#[test]
fn high_bit_logical_slot_survives_the_wrapper_codec() {
    let mut heap = Heap::try_with_capacity_and_slots(2 << 20, 32768).unwrap();
    let mut last = (0, 0);
    for _ in 0..32768 {
        last = heap.alloc(0);
    }
    assert_ne!(Value::from_handle(last.0, last.1).raw() & (1 << 31), 0);
    heap.rc_inc(last.0, last.1).unwrap();
    assert!(!heap.rc_dec(last.0, last.1).unwrap());
    heap.clear();
    assert_eq!(heap.live_count(), 0);
}

#[test]
fn byte_cells_pack_four_bytes_and_raw_core_rejects_eight_byte_access() {
    assert_eq!(WORD_BYTES, 4);
    let mut heap = Heap::new();
    for len in [0usize, 3, 4, 5, 8, 9, 17] {
        let bytes: Vec<u8> = (0..len).map(|i| i as u8).collect();
        let value = alloc_bytes(&mut heap, &bytes).unwrap();
        let (slot, generation) = value.as_handle();
        assert_eq!(heap.size(slot, generation).unwrap(), 1 + len.div_ceil(4));
        assert_eq!(read_bytes(&heap, value), Some(bytes));
        heap.rc_dec_handle(value.raw()).unwrap();
    }
    assert_eq!(heap.live_count(), 0);
    let mut arena = CoreArena::with_size(32);
    assert!(arena.peek(0, 8).is_err());
    assert!(arena.poke(0, 8, Word::MAX).is_err());
    arena.poke(0, 4, Word::MAX).unwrap();
    assert_eq!(arena.peek(0, 4), Ok(Word::MAX));
    assert_eq!(CoreArena::ptr_add(Word::MAX, 1), 0);
}
