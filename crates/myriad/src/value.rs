use polka::Value;
use alloc::{string::String, vec::Vec};
use crate::memory::Heap;

// String <-> heap cell ABI.
// A String is stored as a heap cell whose first slot is the byte length
// (plain u64) and whose remaining cell-data words pack the UTF-8 bytes 8 per
// word, little-endian. mask is all zeros.

#[inline]
pub fn string_word_count(byte_len: usize) -> usize {
    1 + (byte_len + 7) / 8
}

pub fn alloc_bytes(heap: &mut Heap, bytes: &[u8]) -> Result<Value, String> {
    let size = string_word_count(bytes.len());
    let (slot, gen_) = heap.try_alloc(size)?;
    let dst = heap.cell_data_mut(slot, gen_)?;
    dst[0] = bytes.len() as u64;
    if !bytes.is_empty() {
        let dst_ptr = dst[1..].as_mut_ptr() as *mut u8;
        unsafe { core::ptr::copy_nonoverlapping(bytes.as_ptr(), dst_ptr, bytes.len()); }
    }
    Ok(Value::from_handle(slot, gen_))
}

pub fn read_bytes(heap: &Heap, val: Value) -> Option<Vec<u8>> {
    if val.is_handle_none() { return None; }
    let (slot, gen_) = val.as_handle();
    let data = heap.cell_data(slot, gen_).ok()?;
    if data.is_empty() { return None; }
    let len = data[0] as usize;
    let mut out = Vec::with_capacity(len);
    let mut remaining = len;
    for i in 1..data.len() {
        if remaining == 0 { break; }
        let word = data[i].to_le_bytes();
        let take = remaining.min(8);
        out.extend_from_slice(&word[..take]);
        remaining -= take;
    }
    Some(out)
}

pub fn alloc_string(heap: &mut Heap, s: &str) -> Result<Value, String> {
    alloc_bytes(heap, s.as_bytes())
}

pub fn read_string(heap: &Heap, val: Value) -> Option<String> {
    read_bytes(heap, val).and_then(|b| String::from_utf8(b).ok())
}
