use super::core_heap::{BLK_HDR, GLOBAL_HDR, read_u32, read_word};
use super::heap_core;
use super::{WORD_BITS, WORD_BYTES};
use alloc::{format, string::String};

pub(super) const PRESERVE_GENERATIONS: bool = false;

pub(super) fn default_layout(bytes: usize) -> (usize, usize) {
    (bytes.max(GLOBAL_HDR), 0)
}

pub(super) fn validate_layout(bytes: usize, slots: usize) -> Result<(), String> {
    if slots != 0 {
        return Err("heap64 uses direct handles, not a slot table".into());
    }
    if bytes < GLOBAL_HDR {
        return Err("heap: arena cannot fit global header".into());
    }
    Ok(())
}

pub(super) fn init(arena: &mut [u8], _slots: usize) -> Result<(), String> {
    let bytes = arena.len() as super::Word;
    heap_core::core_init(arena, bytes)
        .map(|_| ())
        .map_err(String::from)
}

pub(super) fn binding_available(_arena: &[u8]) -> bool {
    true
}

pub(super) fn resolve(arena: &[u8], slot: u32, generation: u32, op: &str) -> Result<usize, String> {
    let off = slot as usize;
    let frontier = read_word(arena, 0) as usize;
    if off < GLOBAL_HDR || off.checked_add(BLK_HDR).is_none_or(|end| end > arena.len()) {
        return Err(format!(
            "{}: slot {} off {:#x} outside arena",
            op, slot, off
        ));
    }
    if read_u32(arena, off) == 0 {
        return Err(format!(
            "{}: use-after-free slot {} off {:#x} rc=0; frontier {:#x}",
            op, slot, off, frontier
        ));
    }
    if read_u32(arena, off + 4) & 0x00ff_ffff != generation {
        return Err(format!("{}: stale handle for slot {}", op, slot));
    }
    Ok(off)
}

pub(super) fn next_live(arena: &[u8], cursor: &mut usize) -> Option<(u32, u32, usize)> {
    let frontier = read_word(arena, 0) as usize;
    if *cursor == 0 {
        *cursor = GLOBAL_HDR;
    }
    while *cursor < frontier {
        let off = *cursor;
        let size = read_u32(arena, off + 8) as usize;
        *cursor += BLK_HDR + size.div_ceil(WORD_BITS as usize) * WORD_BYTES + size * WORD_BYTES;
        if read_u32(arena, off) != 0 {
            return Some((off as u32, read_u32(arena, off + 4) & 0x00ff_ffff, off));
        }
    }
    None
}
