// read_bytes / read_string are host-facing: the host may hand them any handle,
// including a cell that is not a string. A cell is untyped, so the only check
// is consistency: the length word must fit the cell. Anything else is None,
// never a panic or an allocation sized by the garbage length word.

use myriad::{alloc_bytes, read_bytes, read_string, Heap, Value};

fn cell_with_words(heap: &mut Heap, words: &[u64]) -> Value {
    let (slot, gen_) = heap.try_alloc(words.len()).unwrap();
    heap.cell_data_mut(slot, gen_).unwrap().copy_from_slice(words);
    Value::from_handle(slot, gen_)
}

#[test]
fn length_word_beyond_cell_capacity_is_not_a_string() {
    let mut heap = Heap::new();
    for len in [9u64, 17, 1 << 20, u64::MAX, (-1i64) as u64, (-7i64) as u64] {
        let v = cell_with_words(&mut heap, &[len, 0x6161_6161_6161_6161]);
        assert_eq!(read_bytes(&heap, v), None, "len word {:#x}", len);
        assert_eq!(read_string(&heap, v), None, "len word {:#x}", len);
    }
}

#[test]
fn int_array_cell_read_as_bytes_is_none() {
    let mut heap = Heap::new();
    let v = cell_with_words(&mut heap, &[(-1i64) as u64, 2, 3]);
    assert_eq!(read_bytes(&heap, v), None);
}

#[test]
fn length_word_exactly_at_capacity_reads_all_bytes() {
    let mut heap = Heap::new();
    let v = cell_with_words(&mut heap, &[16, 0x0807_0605_0403_0201, 0x100f_0e0d_0c0b_0a09]);
    assert_eq!(read_bytes(&heap, v).unwrap(), (1u8..=16).collect::<Vec<_>>());
}

#[test]
fn single_word_cell_with_zero_length_is_empty_bytes() {
    let mut heap = Heap::new();
    let v = cell_with_words(&mut heap, &[0]);
    assert_eq!(read_bytes(&heap, v).unwrap(), Vec::<u8>::new());
}

#[test]
fn single_word_cell_with_nonzero_length_is_none() {
    let mut heap = Heap::new();
    let v = cell_with_words(&mut heap, &[1]);
    assert_eq!(read_bytes(&heap, v), None);
}

#[test]
fn alloc_bytes_round_trips_every_length_up_to_three_words() {
    let mut heap = Heap::new();
    for n in 0..=24usize {
        let bytes: Vec<u8> = (0..n as u8).map(|b| b.wrapping_mul(37)).collect();
        let v = alloc_bytes(&mut heap, &bytes).unwrap();
        assert_eq!(read_bytes(&heap, v).unwrap(), bytes, "n={}", n);
    }
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0
    }
}

#[test]
fn fuzz_random_cells_never_yield_more_bytes_than_cell_holds() {
    let mut rng = Rng(7);
    let mut heap = Heap::new();
    for _ in 0..2000 {
        let words = (rng.next() % 6 + 1) as usize;
        let mut data: Vec<u64> = (0..words).map(|_| rng.next()).collect();
        if rng.next() % 2 == 0 { data[0] %= 64; }
        let v = cell_with_words(&mut heap, &data);
        let cap = (words - 1) * 8;
        match read_bytes(&heap, v) {
            None => assert!(data[0] as usize > cap || data[0] > usize::MAX as u64, "{:?}", data),
            Some(b) => assert_eq!(b.len() as u64, data[0], "{:?}", data),
        }
    }
}
