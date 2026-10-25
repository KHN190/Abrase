use super::core_heap::{BLK_HDR, GLOBAL_HDR, read_u32, read_word};
use super::heap_core;
use super::{HANDLE_SLOT_MAX, Word};
use alloc::{format, string::String};

pub(super) const PRESERVE_GENERATIONS: bool = true;
const DEFAULT_SLOTS: usize = 1024;
const HDR_FRONTIER: usize = 0;
const HDR_SLOT_HEAD: usize = 20;
const HDR_SLOT_COUNT: usize = 24;
const HDR_HEAP_START: usize = 28;
const ENTRY_BYTES: usize = 8;
const ENTRY_ACTIVE: u32 = 1 << 16;

pub(super) fn default_layout(bytes: usize) -> (usize, usize) {
    let bytes = bytes.max(GLOBAL_HDR + 2 * ENTRY_BYTES + BLK_HDR);
    let slots = ((bytes - GLOBAL_HDR - BLK_HDR) / ENTRY_BYTES - 1).min(DEFAULT_SLOTS);
    (bytes, slots)
}

pub(super) fn validate_layout(bytes: usize, slots: usize) -> Result<(), String> {
    if !(1..=HANDLE_SLOT_MAX as usize).contains(&slots) {
        return Err("heap: invalid handle slot capacity".into());
    }
    let required = slots
        .checked_add(1)
        .and_then(|n| n.checked_mul(ENTRY_BYTES))
        .and_then(|n| n.checked_add(GLOBAL_HDR))
        .and_then(|n| n.checked_add(BLK_HDR))
        .ok_or_else(|| String::from("heap: handle table size overflow"))?;
    if bytes < required {
        return Err("heap: arena cannot fit handle table and block header".into());
    }
    Ok(())
}

pub(super) fn init(arena: &mut [u8], slots: usize) -> Result<(), String> {
    let bytes = arena.len() as Word;
    let result = heap_core::core_init(arena, bytes, slots as Word).map_err(String::from)?;
    if result != 0 {
        return Err("heap: core32 initialization failed".into());
    }
    Ok(())
}

pub(super) fn binding_available(arena: &[u8]) -> bool {
    read_u32(arena, HDR_SLOT_HEAD) != 0
}

pub(super) fn resolve(arena: &[u8], slot: u32, generation: u32, op: &str) -> Result<usize, String> {
    if slot == 0 || slot > read_u32(arena, HDR_SLOT_COUNT) || generation > u16::MAX as u32 {
        return Err(format!(
            "{}: invalid slot {} generation {}",
            op, slot, generation
        ));
    }
    let entry = GLOBAL_HDR + slot as usize * ENTRY_BYTES;
    let state = read_u32(arena, entry + 4);
    if state & ENTRY_ACTIVE == 0 || state & u16::MAX as u32 != generation {
        return Err(format!(
            "{}: stale handle for slot {} generation {}",
            op, slot, generation
        ));
    }
    let off = read_u32(arena, entry) as usize;
    let frontier = read_word(arena, HDR_FRONTIER) as usize;
    if off < read_u32(arena, HDR_HEAP_START) as usize
        || off
            .checked_add(BLK_HDR)
            .is_none_or(|end| end > frontier || end > arena.len())
        || read_u32(arena, off) == 0
    {
        return Err(format!("{}: invalid block for slot {}", op, slot));
    }
    Ok(off)
}

pub(super) fn next_live(arena: &[u8], cursor: &mut usize) -> Option<(u32, u32, usize)> {
    let count = read_u32(arena, HDR_SLOT_COUNT) as usize;
    while *cursor < count {
        *cursor += 1;
        let slot = *cursor as u32;
        let state = read_u32(arena, GLOBAL_HDR + *cursor * ENTRY_BYTES + 4);
        if state & ENTRY_ACTIVE != 0 {
            let generation = state & u16::MAX as u32;
            if let Ok(off) = resolve(arena, slot, generation, "scan") {
                return Some((slot, generation, off));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::super::{
        core_heap::CoreHeap,
        memory::{handle_parts, make_handle},
    };
    use super::*;
    use alloc::vec::Vec;

    #[test]
    fn readonly_resolver_matches_generated_resolver_across_lifetimes() {
        let mut heap = CoreHeap::try_with_capacity_and_slots(4096, 4).unwrap();
        let mut handles = Vec::new();
        for _ in 0..4 {
            let (slot, generation) = heap.alloc(1);
            handles.push(make_handle(slot, generation));
        }
        handles.extend([0, 1, 0xffff, 0xffff0000, Word::MAX]);
        for round in 0..3 {
            for &handle in &handles {
                let expected = heap_core::resolve_offset(heap.bytes_mut(), handle).unwrap();
                let (slot, generation) = handle_parts(handle);
                assert_eq!(
                    heap.valid(slot, generation, "test").ok(),
                    (expected != Word::MAX).then_some(expected as usize)
                );
            }
            heap.clear();
            if round != 2 {
                let (slot, generation) = heap.alloc(1);
                handles.push(make_handle(slot, generation));
            }
        }
        assert!(heap.valid(u32::MAX, 0, "invalid").is_err());
        assert!(heap.valid(1, u32::MAX, "invalid").is_err());
    }
}
