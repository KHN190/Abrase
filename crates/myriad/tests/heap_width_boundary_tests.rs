use myriad::{Heap, RegionTable, Value};

const BOUNDARIES: [usize; 4] = [31, 32, 63, 64];

#[test]
fn clearing_boundary_handle_tags_preserves_neighbor_tags_and_scalar_bits() {
    let mut heap = Heap::new();
    let (parent, pg) = heap.alloc(65);
    let mut children = Vec::new();
    for offset in BOUNDARIES {
        let (child, cg) = heap.alloc(1);
        let raw = Value::from_handle(child, cg).raw();
        heap.st(parent, pg, offset, raw, true).unwrap();
        children.push((offset, child, cg, raw));
    }
    for (cleared, &(offset, child, cg, raw)) in children.iter().enumerate() {
        // Heap::st returns the old owner; the caller releases it after replacement.
        assert_eq!(
            heap.st(parent, pg, offset, raw, false).unwrap(),
            (raw, true)
        );
        assert!(heap.rc_dec(child, cg).unwrap());
        for (index, &(other, _, _, bits)) in children.iter().enumerate() {
            assert_eq!(heap.ld(parent, pg, other).unwrap(), (bits, index > cleared));
        }
    }
    assert!(heap.rc_dec(parent, pg).unwrap());
    assert_eq!(heap.live_count(), 0);
}

#[test]
fn scalar_handle_bits_at_mask_boundaries_do_not_release_the_referent() {
    let mut heap = Heap::new();
    let (referent, rg) = heap.alloc(1);
    heap.st(referent, rg, 0, 73, false).unwrap();
    let raw = Value::from_handle(referent, rg).raw();
    let (parent, pg) = heap.alloc(65);
    for offset in BOUNDARIES {
        heap.st(parent, pg, offset, raw, false).unwrap();
        assert_eq!(heap.ld(parent, pg, offset).unwrap(), (raw, false));
    }
    assert!(heap.rc_dec(parent, pg).unwrap());
    assert_eq!(heap.live_count(), 1);
    assert_eq!(heap.rc(referent, rg), Some(1));
    assert_eq!(heap.ld(referent, rg, 0).unwrap(), (73, false));
    assert!(heap.rc_dec(referent, rg).unwrap());
    assert_eq!(heap.live_count(), 0);
}

#[test]
fn stale_operations_after_reuse_do_not_mutate_the_new_owner() {
    let mut heap = Heap::new();
    let (old, og) = heap.alloc(65);
    assert!(heap.rc_dec(old, og).unwrap());
    let (new, ng) = heap.alloc(65);
    assert_eq!(old, new, "exercise an actually reused slot");
    assert_ne!(og, ng);
    heap.st(new, ng, 64, 91, false).unwrap();
    assert!(heap.ld(old, og, 64).is_err());
    assert!(heap.st(old, og, 64, 0, false).is_err());
    assert!(heap.rc_inc(old, og).is_err());
    assert!(heap.rc_dec(old, og).is_err());
    heap.force_free(old, og).unwrap();
    assert_eq!(heap.rc(new, ng), Some(1));
    assert_eq!(heap.ld(new, ng, 64).unwrap(), (91, false));
    assert!(heap.rc_dec(new, ng).unwrap());
    assert_eq!(heap.live_count(), 0);
}

#[test]
fn region_force_free_reclaims_nested_boundary_children_but_preserves_scalars() {
    let mut heap = Heap::new();
    let mut regions = RegionTable::new();
    let (outside, og) = heap.alloc(1);
    heap.st(outside, og, 0, 123, false).unwrap();
    let scalar = Value::from_handle(outside, og).raw();
    regions.push();
    let (root, rg) = heap.alloc(65);
    regions.record_alloc(root, rg);
    let mut owned = vec![(root, rg)];
    for (index, offset) in BOUNDARIES.into_iter().enumerate() {
        let (child, cg) = heap.alloc(65);
        let (leaf, lg) = heap.alloc(1);
        heap.st(
            child,
            cg,
            BOUNDARIES[3 - index],
            Value::from_handle(leaf, lg).raw(),
            true,
        )
        .unwrap();
        heap.st(root, rg, offset, Value::from_handle(child, cg).raw(), true)
            .unwrap();
        heap.st(child, cg, 0, scalar, false).unwrap();
        regions.record_alloc(child, cg);
        regions.record_alloc(leaf, lg);
        owned.extend([(child, cg), (leaf, lg)]);
    }
    heap.rc_inc(root, rg).unwrap();
    heap.rc_inc(root, rg).unwrap();
    assert_eq!(heap.rc(root, rg), Some(3));
    assert_eq!(heap.live_count(), owned.len() + 1);
    regions.pop_and_release(&mut heap).unwrap();
    assert_eq!(regions.depth(), 0);
    for (slot, generation) in owned {
        assert!(!heap.is_live(slot, generation));
        assert!(heap.ld(slot, generation, 0).is_err());
    }
    assert_eq!(heap.live_count(), 1);
    assert_eq!(heap.rc(outside, og), Some(1));
    assert_eq!(heap.ld(outside, og, 0).unwrap(), (123, false));
    assert!(heap.rc_dec(outside, og).unwrap());
    assert_eq!(heap.live_count(), 0);
}
