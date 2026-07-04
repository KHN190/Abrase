#[path = "compiler_codegen_common.rs"]
mod compiler_codegen_common;

use compiler_codegen_common::*;
use myriad::Value;

struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self { Self(seed.wrapping_mul(6364136223846793005).wrapping_add(1)) }
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0
    }
    fn pick(&mut self, n: u64) -> u64 { self.next() % n }
}

const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";

fn gen_ascii(rng: &mut Rng, len: usize) -> String {
    (0..len).map(|_| ALPHABET[rng.pick(ALPHABET.len() as u64) as usize] as char).collect()
}

fn oracle_byte(s: &str, i: i64) -> i64 {
    if i < 0 { 0 } else { s.as_bytes().get(i as usize).copied().unwrap_or(0) as i64 }
}

#[test]
fn str_index_matches_rust_oracle_no_panic_no_leak() {
    let mut rng = Rng::new(0xB17E5);
    for _ in 0..500 {
        let len = rng.pick(9) as usize;
        let s = gen_ascii(&mut rng, len);
        let i = rng.pick((len as u64) + 4) as i64 - 2;
        let src = format!("fn main() -> Int {{ \"{s}\"[{i}] }}");
        let (v, live) = run_source_with_heap(&src).expect("run");
        assert_eq!(v, Value::from_int(oracle_byte(&s, i)),
            "byte mismatch: s={s:?} i={i}");
        assert_eq!(live, 0, "leak: s={s:?} i={i}");
    }
}

#[test]
fn str_len_matches_rust_oracle() {
    let mut rng = Rng::new(0x1E6);
    for _ in 0..300 {
        let len = rng.pick(12) as usize;
        let s = gen_ascii(&mut rng, len);
        let src = format!("fn main() -> Int {{ \"{s}\".len() }}");
        let v = run_source(&src).expect("run");
        assert_eq!(v, Value::from_int(len as i64), "len mismatch: s={s:?}");
    }
}
