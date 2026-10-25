use myriad::{word32, word64};

#[test]
fn host_contains_two_distinct_value_and_heap_layouts() {
    assert_ne!(
        std::any::TypeId::of::<word32::Value>(),
        std::any::TypeId::of::<word64::Value>()
    );
    assert_ne!(
        std::any::TypeId::of::<word32::Heap>(),
        std::any::TypeId::of::<word64::Heap>()
    );
    assert_eq!(std::mem::size_of::<word32::Value>(), 4);
    assert_eq!(std::mem::size_of::<word64::Value>(), 8);
    let mut narrow = word32::Heap::new();
    let mut wide = word64::Heap::new();
    let n = narrow.alloc(65);
    let w = wide.alloc(65);
    assert_eq!(
        std::mem::size_of_val(narrow.cell_data(n.0, n.1).unwrap()),
        65 * 4
    );
    assert_eq!(
        std::mem::size_of_val(wide.cell_data(w.0, w.1).unwrap()),
        65 * 8
    );
    assert_eq!(narrow.cell_mask(n.0, n.1).unwrap().len(), 3);
    assert_eq!(wide.cell_mask(w.0, w.1).unwrap().len(), 2);
    narrow.st(n.0, n.1, 64, u32::MAX, false).unwrap();
    wide.st(w.0, w.1, 64, u64::MAX - 1, false).unwrap();
    assert_eq!(narrow.ld(n.0, n.1, 64), Ok((u32::MAX, false)));
    assert_eq!(wide.ld(w.0, w.1, 64), Ok((u64::MAX - 1, false)));
    narrow.rc_dec(n.0, n.1).unwrap();
    wide.rc_dec(w.0, w.1).unwrap();
    assert_eq!(narrow.live_count(), 0);
    assert_eq!(wide.live_count(), 0);
}

macro_rules! check_graph {
    ($profile:ident) => {{
        let mut heap = $profile::Heap::new();
        let mut regions = $profile::RegionTable::new();
        regions.push();
        let (root, rg) = heap.alloc(65);
        regions.record_alloc(root, rg);
        let mut owned = vec![(root, rg)];
        for offset in [31, 32, 63, 64] {
            let (child, cg) = heap.alloc(1);
            heap.st(
                root,
                rg,
                offset,
                $profile::Value::from_handle(child, cg).raw(),
                true,
            )
            .unwrap();
            owned.push((child, cg));
        }
        heap.rc_inc(root, rg).unwrap();
        regions.pop_and_release(&mut heap).unwrap();
        for (slot, generation) in owned {
            assert!(!heap.is_live(slot, generation));
        }
        assert_eq!(heap.live_count(), 0);
    }};
}

#[test]
fn both_instances_share_the_same_rc_and_region_cleanup_contract() {
    check_graph!(word32);
    check_graph!(word64);
}

#[test]
fn byte_packing_is_local_to_each_heap_instance() {
    let mut narrow = word32::Heap::new();
    let mut wide = word64::Heap::new();
    let bytes = [0, 1, 2, 3, 4, 5, 6, 7, 8];
    let n = word32::alloc_bytes(&mut narrow, &bytes).unwrap();
    let w = word64::alloc_bytes(&mut wide, &bytes).unwrap();
    let (ns, ng) = n.as_handle();
    let (ws, wg) = w.as_handle();
    assert_eq!(narrow.cell_data(ns, ng).unwrap().len(), 4);
    assert_eq!(wide.cell_data(ws, wg).unwrap().len(), 3);
    assert_eq!(word32::read_bytes(&narrow, n), Some(bytes.to_vec()));
    assert_eq!(word64::read_bytes(&wide, w), Some(bytes.to_vec()));
    narrow.rc_dec(ns, ng).unwrap();
    wide.rc_dec(ws, wg).unwrap();
    assert_eq!(narrow.live_count(), 0);
    assert_eq!(wide.live_count(), 0);
}

macro_rules! check_tag_clear {
    ($profile:ident) => {{
        let mut heap = $profile::Heap::new();
        let (root, rg) = heap.alloc(65);
        let mut children = Vec::new();
        for offset in [31, 32, 63, 64] {
            let (slot, generation) = heap.alloc(1);
            let raw = $profile::Value::from_handle(slot, generation).raw();
            heap.st(root, rg, offset, raw, true).unwrap();
            children.push((offset, slot, generation, raw));
        }
        for (cleared, &(offset, slot, generation, raw)) in children.iter().enumerate() {
            assert_eq!(heap.st(root, rg, offset, raw, false), Ok((raw, true)));
            heap.rc_dec(slot, generation).unwrap();
            for (index, &(other, _, _, bits)) in children.iter().enumerate() {
                assert_eq!(heap.ld(root, rg, other), Ok((bits, index > cleared)));
            }
        }
        heap.rc_dec(root, rg).unwrap();
        assert_eq!(heap.live_count(), 0);
    }};
}

#[test]
fn clearing_tags_is_local_to_each_word_and_preserves_scalar_bits() {
    check_tag_clear!(word32);
    check_tag_clear!(word64);
}
