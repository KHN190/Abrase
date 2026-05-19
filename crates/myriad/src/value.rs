use polka::Value;
use crate::memory::Heap;

// String <-> heap cell ABI.
// A String is stored as a heap cell whose first slot is the byte length
// (plain u64) and whose remaining cell-data words pack the UTF-8 bytes 8 per
// word, little-endian. mask is all zeros.

#[inline]
pub fn string_word_count(byte_len: usize) -> usize {
    1 + (byte_len + 7) / 8
}

pub fn alloc_string(heap: &mut Heap, s: &str) -> Result<Value, String> {
    let bytes = s.as_bytes();
    let size = string_word_count(bytes.len());
    let (slot, gen_) = heap.try_alloc(size)?;
    heap.st(slot, gen_, 0, bytes.len() as u64, false)?;
    for (i, chunk) in bytes.chunks(8).enumerate() {
        let mut buf = [0u8; 8];
        buf[..chunk.len()].copy_from_slice(chunk);
        let w = u64::from_le_bytes(buf);
        heap.st(slot, gen_, 1 + i, w, false)?;
    }
    Ok(Value::from_handle(slot, gen_))
}

pub fn read_string(heap: &Heap, val: Value) -> Option<String> {
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
    String::from_utf8(out).ok()
}
