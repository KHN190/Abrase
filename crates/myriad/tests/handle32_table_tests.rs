#[path = "support/handle32.rs"]
mod handle32;
use handle32::{Handle32Error, Handle32Table};

#[test]
fn table_budget_counts_reserved_slot_and_rejects_oversized_requests() {
    assert!(matches!(
        Handle32Table::new(2, 23),
        Err(Handle32Error::BudgetExceeded)
    ));
    let table = Handle32Table::new(2, 24).unwrap();
    assert_eq!(table.storage_bytes(), 24);
    for slots in [0, 65_535, usize::MAX] {
        assert!(matches!(
            Handle32Table::new(slots, usize::MAX),
            Err(Handle32Error::InvalidCapacity)
        ));
    }
}

#[test]
fn full_table_failure_preserves_live_bindings_and_reuses_released_slot() {
    let mut table = Handle32Table::new(2, 24).unwrap();
    let first = table.allocate(0).unwrap();
    let second = table.allocate(u32::MAX).unwrap();
    assert_eq!(table.allocate(48), Err(Handle32Error::Exhausted));
    assert_eq!(table.resolve(first), Ok(0));
    assert_eq!(table.resolve(second), Ok(u32::MAX));
    assert_eq!(table.release(first), Ok(0));
    let replacement = table.allocate(48).unwrap();
    assert_ne!(first, replacement);
    assert_eq!(table.resolve(replacement), Ok(48));
    assert_eq!(table.resolve(second), Ok(u32::MAX));
}

#[test]
fn stale_resolve_and_release_do_not_touch_the_replacement() {
    let mut table = Handle32Table::new(1, 16).unwrap();
    let stale = table.allocate(32).unwrap();
    assert_eq!(table.release(stale), Ok(32));
    let live = table.allocate(32).unwrap();
    for _ in 0..3 {
        assert_eq!(table.resolve(stale), Err(Handle32Error::InvalidHandle));
        assert_eq!(table.release(stale), Err(Handle32Error::InvalidHandle));
        assert_eq!(table.resolve(live), Ok(32));
    }
    assert_eq!(table.release(live), Ok(32));
    assert_eq!(table.release(live), Err(Handle32Error::InvalidHandle));
}

#[test]
fn final_generation_retires_slot_without_reviving_any_old_handle() {
    let mut table = Handle32Table::new(1, 16).unwrap();
    let mut issued = Vec::new();
    for generation in 0..=u16::MAX {
        let handle = table.allocate(64).unwrap();
        assert_eq!(handle >> 16, 1);
        assert_eq!(handle & 0xffff, u32::from(generation));
        issued.push(handle);
        assert_eq!(table.release(handle), Ok(64));
    }
    assert_eq!(table.allocate(64), Err(Handle32Error::Exhausted));
    for stale in issued {
        assert_eq!(table.resolve(stale), Err(Handle32Error::InvalidHandle));
        assert_eq!(table.release(stale), Err(Handle32Error::InvalidHandle));
    }
    assert_eq!(table.storage_bytes(), 16);
}

#[test]
fn retired_slot_does_not_prevent_physical_offset_reuse_by_another_slot() {
    let mut table = Handle32Table::new(2, 24).unwrap();
    let first = table.allocate(128).unwrap();
    table.release(first).unwrap();
    for _ in 1..=u16::MAX {
        let handle = table.allocate(128).unwrap();
        assert_eq!(handle >> 16, first >> 16);
        table.release(handle).unwrap();
    }
    let next_slot = table.allocate(128).unwrap();
    assert_ne!(next_slot >> 16, first >> 16);
    assert_eq!(table.resolve(next_slot), Ok(128));
    assert_eq!(table.release(first), Err(Handle32Error::InvalidHandle));
    assert_eq!(table.resolve(next_slot), Ok(128));
}

#[test]
fn reserved_unallocated_and_free_handles_are_rejected() {
    let mut table = Handle32Table::new(1, 16).unwrap();
    for invalid in [0, 1, 0xffff, 0xffff_0000, u32::MAX, 2 << 16, 1 << 16] {
        assert_eq!(table.resolve(invalid), Err(Handle32Error::InvalidHandle));
        assert_eq!(table.release(invalid), Err(Handle32Error::InvalidHandle));
    }
    let live = table.allocate(96).unwrap();
    assert_eq!(table.resolve(live ^ 1), Err(Handle32Error::InvalidHandle));
    assert_eq!(table.resolve(live), Ok(96));
}

#[test]
fn independent_table_does_not_change_the_existing_64_bit_codec() {
    let expected = (0x100_0001u64 << 24) | 0xff_ffff;
    let value = myriad::Value::from_handle(0x100_0001, 0xff_ffff);
    assert_eq!(value.raw(), expected);
    assert_eq!(value.as_handle(), (0x100_0001, 0xff_ffff));
    assert_eq!(myriad::memory::make_handle(0x100_0001, 0xff_ffff), expected);
    assert_eq!(myriad::memory::handle_parts(expected), value.as_handle());
}
