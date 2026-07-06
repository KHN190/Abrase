#[path = "compiler_codegen_common.rs"]
mod compiler_codegen_common;

use compiler_codegen_common::*;
use myriad::Value;

#[test]
fn str_len_basic() {
    let r = run_source("fn main() -> Int { \"ab\".len() }").expect("run");
    assert_eq!(r, Value::from_int(2));
}

#[test]
fn str_len_empty() {
    let r = run_source("fn main() -> Int { \"\".len() }").expect("run");
    assert_eq!(r, Value::from_int(0));
}

#[test]
fn str_len_interpolated_fresh_alloc() {
    let src = "fn main() -> Int { let x = \"cd\"; \"ab{x}\".len() }";
    let r = run_source(src).expect("run");
    assert_eq!(r, Value::from_int(4));
}

#[test]
fn str_byte_at_method_first() {
    let r = run_source("fn main() -> Int { \"ab\".byte_at(0) }").expect("run");
    assert_eq!(r, Value::from_int(97));
}

#[test]
fn str_index_first() {
    let r = run_source("fn main() -> Int { \"ab\"[0] }").expect("run");
    assert_eq!(r, Value::from_int(97));
}

#[test]
fn str_index_second() {
    let r = run_source("fn main() -> Int { \"ab\"[1] }").expect("run");
    assert_eq!(r, Value::from_int(98));
}

#[test]
fn str_index_binding() {
    let r = run_source("fn main() -> Int { let s = \"xy\"; s[1] }").expect("run");
    assert_eq!(r, Value::from_int(121));
}

#[test]
fn str_index_equal_len_is_zero() {
    let r = run_source("fn main() -> Int { \"ab\"[2] }").expect("run");
    assert_eq!(r, Value::from_int(0));
}

#[test]
fn str_index_out_of_range_is_zero() {
    let r = run_source("fn main() -> Int { \"ab\"[99] }").expect("run");
    assert_eq!(r, Value::from_int(0));
}

#[test]
fn str_index_negative_is_zero() {
    let r = run_source("fn main() -> Int { \"ab\"[0 - 1] }").expect("run");
    assert_eq!(r, Value::from_int(0));
}

#[test]
fn str_byte_compose_to_char() {
    let src = "fn main() -> Char { \"Az\"[0].to_c() }";
    let r = run_source(src).expect("run");
    assert_eq!(r, Value::from_char('A'));
}

#[test]
fn str_index_in_loop_no_leak() {
    let src = "fn main() -> Int { let mut acc = 0; let mut i = 0; while i < 10 { let x = \"z\"; acc = acc + \"a{x}\"[1]; i = i + 1 }; acc }";
    let (r, live) = run_source_with_heap(src).expect("run");
    assert_eq!(r, Value::from_int(122 * 10));
    assert_eq!(live, 0, "per-iter string temporaries must be freed: {} live", live);
}

#[test]
fn str_ops_no_heap_leak() {
    let src = "fn main() -> Int { let x = \"cd\"; \"ab{x}\".len() + \"ef\"[1] }";
    let (r, live) = run_source_with_heap(src).expect("run");
    assert_eq!(r, Value::from_int(4 + 102));
    assert_eq!(live, 0, "string temporaries must be freed: {} live", live);
}

#[test]
fn str_len_reused_not_consumed() {
    let (v, live) = run_source_with_heap("fn main() -> Int { let s = \"ab\"; s.len() + s.len() }").expect("run");
    assert_eq!(v, Value::from_int(4));
    assert_eq!(live, 0, "receiver must survive read-only method: {live} live");
}

#[test]
fn str_byte_at_reused_not_consumed() {
    let (v, live) = run_source_with_heap("fn main() -> Int { let s = \"ab\"; s.byte_at(0) + s.byte_at(1) }").expect("run");
    assert_eq!(v, Value::from_int(97 + 98));
    assert_eq!(live, 0, "receiver must survive: {live} live");
}

#[test]
fn str_scan_loop_len_and_index() {
    let src = "fn main() -> Int { let s = \"abc\"; let n = s.len(); let mut i = 0; let mut sum = 0; while i < n { sum = sum + s[i]; i = i + 1 }; sum }";
    let (v, live) = run_source_with_heap(src).expect("run");
    assert_eq!(v, Value::from_int(97 + 98 + 99));
    assert_eq!(live, 0, "scan loop must not leak: {live} live");
}

#[test]
fn str_len_then_index_same_binding() {
    let (v, live) = run_source_with_heap("fn main() -> Int { let s = \"xy\"; let n = s.len(); n + s[0] }").expect("run");
    assert_eq!(v, Value::from_int(2 + 120));
    assert_eq!(live, 0, "{live} live");
}

#[test]
fn str_len_temporary_receiver_no_leak() {
    let (v, live) = run_source_with_heap("fn main() -> Int { \"ab\".len() }").expect("run");
    assert_eq!(v, Value::from_int(2));
    assert_eq!(live, 0, "temp receiver must be freed exactly once: {live} live");
}

#[test]
fn str_byte_at_temporary_receiver_no_leak() {
    let (v, live) = run_source_with_heap("fn main() -> Int { \"Az\".byte_at(0) }").expect("run");
    assert_eq!(v, Value::from_int(65));
    assert_eq!(live, 0, "{live} live");
}

#[test]
fn str_receiver_reused_across_statements() {
    let src = "fn main() -> Int { let s = \"abc\"; let a = s.len(); let b = s.byte_at(0); a + b + s[1] }";
    let (v, live) = run_source_with_heap(src).expect("run");
    assert_eq!(v, Value::from_int(3 + 97 + 98));
    assert_eq!(live, 0, "{live} live");
}

#[test]
fn str_len_on_call_temporary_no_leak() {
    let src = "fn mk() -> String { \"hi\" } fn main() -> Int { mk().len() }";
    let (v, live) = run_source_with_heap(src).expect("run");
    assert_eq!(v, Value::from_int(2));
    assert_eq!(live, 0, "call-temp receiver must drop once: {live} live");
}

#[test]
fn str_len_on_array_element_receiver() {
    // receiver is an index projection -> routes through emit_method_call (not mono-lowered);
    // element is an rc-inc'd temp load, move-staging stays balanced, array survives.
    let src = "fn main() -> Int { let a = [\"hi\"]; let n = a[0].len(); n + a[0].byte_at(0) }";
    let (v, live) = run_source_with_heap(src).expect("run");
    assert_eq!(v, Value::from_int(2 + 104));
    assert_eq!(live, 0, "array element read-only method must not leak/corrupt: {live} live");
}

fn oracle_slice(s: &str, off: i64, len: i64) -> String {
    let b = s.as_bytes();
    let start = off.max(0).min(b.len() as i64) as usize;
    let take = len.max(0) as usize;
    let end = start.saturating_add(take).min(b.len());
    String::from_utf8_lossy(&b[start..end]).into_owned()
}

#[test]
fn str_slice_basic() {
    let r = run_source_string("fn main() -> String { \"hello\".slice(1, 3) }").expect("run");
    assert_eq!(r, "ell");
}

#[test]
fn str_slice_from_start() {
    let r = run_source_string("fn main() -> String { \"hello\".slice(0, 2) }").expect("run");
    assert_eq!(r, "he");
}

#[test]
fn str_slice_full() {
    let r = run_source_string("fn main() -> String { \"hello\".slice(0, 5) }").expect("run");
    assert_eq!(r, "hello");
}

#[test]
fn str_slice_zero_len_empty() {
    let r = run_source_string("fn main() -> String { \"hello\".slice(2, 0) }").expect("run");
    assert_eq!(r, "");
}

#[test]
fn str_slice_len_past_end_clamps() {
    let r = run_source_string("fn main() -> String { \"ab\".slice(1, 50) }").expect("run");
    assert_eq!(r, "b");
}

#[test]
fn str_slice_offset_past_end_empty() {
    let r = run_source_string("fn main() -> String { \"ab\".slice(99, 3) }").expect("run");
    assert_eq!(r, "");
}

#[test]
fn str_slice_negative_offset_clamps_zero() {
    let r = run_source_string("fn main() -> String { \"hello\".slice(0 - 1, 2) }").expect("run");
    assert_eq!(r, "he");
}

#[test]
fn str_slice_negative_len_empty() {
    let r = run_source_string("fn main() -> String { \"hello\".slice(1, 0 - 3) }").expect("run");
    assert_eq!(r, "");
}

#[test]
fn str_slice_binding_receiver() {
    let r = run_source_string("fn main() -> String { let s = \"world\"; s.slice(1, 3) }").expect("run");
    assert_eq!(r, "orl");
}

#[test]
fn str_slice_result_is_fresh_string_len() {
    let (v, _) = run_source_with_heap("fn main() -> Int { \"hello\".slice(1, 3).len() }").expect("run");
    assert_eq!(v, Value::from_int(3));
}

#[test]
fn str_slice_chained() {
    let r = run_source_string("fn main() -> String { \"abcdef\".slice(1, 4).slice(1, 2) }").expect("run");
    assert_eq!(r, "cd");
}

#[test]
fn str_slice_temporary_no_leak() {
    let (v, live) = run_source_with_heap("fn main() -> Int { \"hello\".slice(1, 2).len() }").expect("run");
    assert_eq!(v, Value::from_int(2));
    assert_eq!(live, 0, "slice receiver + result temp must drop: {live} live");
}

#[test]
fn str_slice_receiver_reused_not_consumed() {
    let src = "fn main() -> Int { let s = \"hello\"; s.slice(0, 2).len() + s.len() }";
    let (v, live) = run_source_with_heap(src).expect("run");
    assert_eq!(v, Value::from_int(2 + 5));
    assert_eq!(live, 0, "read-only slice must not consume receiver: {live} live");
}

#[test]
fn str_slice_in_loop_no_leak() {
    let src = "fn main() -> Int { let s = \"abcde\"; let mut i = 0; let mut acc = 0; while i < 20 { acc = acc + s.slice(1, 3).len(); i = i + 1 }; acc }";
    let (v, live) = run_source_with_heap(src).expect("run");
    assert_eq!(v, Value::from_int(20 * 3));
    assert_eq!(live, 0, "loop of slice temps must not leak: {live} live");
}

#[test]
fn str_slice_matches_byte_oracle_sample() {
    let s = "hello";
    for &(off, len) in &[(0i64, 5i64), (1, 3), (2, 0), (3, 10), (0, 1)] {
        let src = format!("fn main() -> String {{ \"{s}\".slice({off}, {len}) }}");
        let got = run_source_string(&src).expect("run");
        assert_eq!(got, oracle_slice(s, off, len), "off={off} len={len}");
    }
}

#[test]
fn str_slice_fuzz_vs_byte_oracle() {
    const ALPHA: &[u8] = b"abcdefghijklmnop";
    let mut seed: u64 = 0x9e3779b97f4a7c15;
    let mut next = || { seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17; seed };
    for _ in 0..400 {
        let slen = (next() % 12) as usize;
        let s: String = (0..slen).map(|_| ALPHA[(next() as usize) % ALPHA.len()] as char).collect();
        let off = (next() % 16) as i64 - 3;
        let len = (next() % 16) as i64 - 3;
        let src = format!("fn main() -> String {{ \"{s}\".slice({}, {}) }}",
            if off < 0 { format!("0 - {}", -off) } else { off.to_string() },
            if len < 0 { format!("0 - {}", -len) } else { len.to_string() });
        let got = run_source_string(&src).expect("run");
        assert_eq!(got, oracle_slice(&s, off, len), "s={s:?} off={off} len={len}");
    }
}

#[test]
fn str_slice_fuzz_len_no_leak() {
    const ALPHA: &[u8] = b"abcdefgh";
    let mut seed: u64 = 0x2545f4914f6cdd1d;
    let mut next = || { seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17; seed };
    for _ in 0..200 {
        let slen = 1 + (next() % 8) as usize;
        let s: String = (0..slen).map(|_| ALPHA[(next() as usize) % ALPHA.len()] as char).collect();
        let off = (next() % 10) as i64;
        let len = (next() % 10) as i64;
        let src = format!("fn main() -> Int {{ \"{s}\".slice({off}, {len}).len() }}");
        let (v, live) = run_source_with_heap(&src).expect("run");
        assert_eq!(v, Value::from_int(oracle_slice(&s, off, len).len() as i64), "s={s:?} off={off} len={len}");
        assert_eq!(live, 0, "fuzz slice leak s={s:?} off={off} len={len}: {live} live");
    }
}
