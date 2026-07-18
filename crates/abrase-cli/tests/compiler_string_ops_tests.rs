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
fn str_index_equal_len_is_runtime_error() {
    let r = run_source("fn main() -> Int { \"ab\"[2] }");
    assert!(r.is_err(), "index == len must trap, got {:?}", r);
}

#[test]
fn str_index_out_of_range_is_runtime_error() {
    let r = run_source("fn main() -> Int { \"ab\"[99] }");
    assert!(r.is_err(), "index past end must trap, got {:?}", r);
}

#[test]
fn str_index_negative_is_runtime_error() {
    let r = run_source("fn main() -> Int { \"ab\"[0 - 1] }");
    assert!(r.is_err(), "negative index must trap, got {:?}", r);
}

#[test]
fn str_index_in_range_still_works() {
    let r = run_source("fn main() -> Int { \"ab\"[1] }").expect("run");
    assert_eq!(r, Value::from_int(98));
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

// --- allocation profiling (String Copy/St vs construction) ---
// heap alloc counter = cumulative try_alloc calls (myriad CoreHeap). These are
// metamorphic: vary the loop trip count and assert the alloc delta, isolating
// which ops actually hit the arena.

#[test]
fn string_array_reads_are_alloc_free_regardless_of_iterations() {
    let prog = |n: i32| format!(
        "fn main() -> Int {{ let a = \"aa\"; let b = \"bb\"; let c = \"cc\"; let d = \"dd\"; \
         let arr = [a, b, c, d]; let mut i = 0; let mut s = 0; \
         while i < {n} {{ s = s + arr[0].len() + arr[1].len() + arr[2].len() + arr[3].len(); i = i + 1 }}; s }}");
    let (_, _, a10) = run_source_with_allocs(&prog(10)).expect("run 10");
    let (_, _, a100) = run_source_with_allocs(&prog(100)).expect("run 100");
    assert_eq!(a10, a100,
        "array index-read + len must be alloc-free; if Copy/St/Ld deep-copied Strings, more iterations would allocate ({a10} vs {a100})");
}

#[test]
fn slice_allocates_exactly_one_string_per_call() {
    let prog = |n: i32| format!(
        "fn main() -> Int {{ let src = \"abcdefghij\"; let mut i = 0; let mut s = 0; \
         while i < {n} {{ let piece = src.slice(0, 3); s = s + piece.len(); i = i + 1 }}; s }}");
    let (_, _, a10) = run_source_with_allocs(&prog(10)).expect("run 10");
    let (_, _, a20) = run_source_with_allocs(&prog(20)).expect("run 20");
    assert_eq!(a20 - a10, 10,
        "slice must allocate exactly one owned String per call; 10 extra calls => 10 extra allocs (got {})", a20 - a10);
}

#[test]
fn byte_at_scan_on_parent_is_alloc_free() {
    let prog = |n: i32| format!(
        "fn main() -> Int {{ let src = \"abcdefghij\"; let mut i = 0; let mut s = 0; \
         while i < {n} {{ s = s + src[0]; i = i + 1 }}; s }}");
    let (_, _, a10) = run_source_with_allocs(&prog(10)).expect("run 10");
    let (_, _, a100) = run_source_with_allocs(&prog(100)).expect("run 100");
    assert_eq!(a10, a100,
        "byte_at index scan on the parent string must not allocate ({a10} vs {a100})");
}

// Pattern (b): scan the parent string with byte_at + compare a region against a
// target in place — no per-line slice. Zero runtime allocation regardless of how
// many lines are scanned; only the two string literals are ever allocated. This
// is the allocation-free alternative to slicing every line then comparing.
fn zero_alloc_line_search(n_lines: usize) -> (i64, u64) {
    let mut src = String::new();
    for _ in 0..n_lines - 1 { src.push_str("aa\\n"); }
    src.push_str("zz");
    let prog = format!(
        "fn main() -> Int {{ let src = \"{src}\"; let target = \"zz\"; \
         let tl = target.len(); let n = src.len(); let mut i = 0; let mut ls = 0; let mut found = 0 - 1; \
         while i <= n {{ \
           let boundary = i == n; \
           let nl = if boundary {{ true }} else {{ src[i] == 10 }}; \
           if nl {{ \
             if (i - ls) == tl {{ \
               let mut j = 0; let mut m = 1; \
               while j < tl {{ if src[ls + j] != target[j] {{ m = 0 }}; j = j + 1 }}; \
               if m == 1 {{ found = ls }} \
             }}; ls = i + 1 \
           }}; i = i + 1 \
         }}; found }}");
    let (v, live, alloc) = run_source_with_allocs(&prog).expect("run");
    assert_eq!(live, 0, "no leak");
    (v.as_int(), alloc)
}

#[test]
fn zero_alloc_scan_finds_target_line() {
    // "zz" sits after (n-1) "aa\n" (3 bytes each) => byte offset 3*(n-1).
    let (found4, _) = zero_alloc_line_search(4);
    let (found8, _) = zero_alloc_line_search(8);
    assert_eq!(found4, 9, "zz starts at byte 9 with 4 lines");
    assert_eq!(found8, 21, "zz starts at byte 21 with 8 lines");
}

#[test]
fn zero_alloc_scan_allocation_is_constant_in_line_count() {
    let (_, a4) = zero_alloc_line_search(4);
    let (_, a8) = zero_alloc_line_search(8);
    let (_, a64) = zero_alloc_line_search(64);
    assert_eq!(a4, a8, "scanning more lines must not allocate more ({a4} vs {a8})");
    assert_eq!(a8, a64, "64 lines allocates the same as 8 ({a8} vs {a64})");
    assert_eq!(a4, 2, "only the two string literals are ever allocated, got {a4}");
}

// ---- Bytes: packed byte buffer (String cell layout, no utf8 invariant) ----

#[test]
fn bytes_len_from_string() {
    let r = run_source("fn main() -> Int { \"abc\".to_bytes().len() }").expect("run");
    assert_eq!(r, Value::from_int(3));
}

#[test]
fn bytes_index_byte() {
    let r = run_source("fn main() -> Int { \"Az\".to_bytes()[0] }").expect("run");
    assert_eq!(r, Value::from_int(65));
}

#[test]
fn bytes_byte_at() {
    let r = run_source("fn main() -> Int { \"Az\".to_bytes().byte_at(1) }").expect("run");
    assert_eq!(r, Value::from_int(122));
}

#[test]
fn bytes_byte_at_out_of_range_is_runtime_error() {
    let r = run_source("fn main() -> Int { \"Az\".to_bytes().byte_at(2) }");
    assert!(r.is_err(), "bytes index past end must trap, got {:?}", r);
}

#[test]
fn bytes_byte_at_negative_is_runtime_error() {
    let r = run_source("fn main() -> Int { \"Az\".to_bytes().byte_at(0 - 1) }");
    assert!(r.is_err(), "bytes negative index must trap, got {:?}", r);
}

#[test]
fn bytes_slice_len() {
    let (v, _) = run_source_with_heap("fn main() -> Int { \"hello\".to_bytes().slice(1, 3).len() }").expect("run");
    assert_eq!(v, Value::from_int(3));
}

#[test]
fn bytes_temporary_no_leak() {
    let (v, live) = run_source_with_heap("fn main() -> Int { \"hello\".to_bytes().len() }").expect("run");
    assert_eq!(v, Value::from_int(5));
    assert_eq!(live, 0, "bytes temp receiver must drop: {live} live");
}

#[test]
fn bytes_bound_slice_no_leak() {
    let src = "fn main() -> Int { let b = \"hello\".to_bytes(); b.slice(1, 2).len() }";
    let (v, live) = run_source_with_heap(src).expect("run");
    assert_eq!(v, Value::from_int(2));
    assert_eq!(live, 0, "bound-bytes slice temp must drop: {live} live");
}

// A read-only native must consume a fresh owned-temp receiver (call/literal
// result), not borrow it — else chained read-only calls leak the intermediate.
// Regression for the owned-temp-vs-view distinction in read-only arg staging.
#[test]
fn chained_readonly_temp_receiver_no_leak() {
    let (_, sl) = run_source_with_heap("fn main() -> Int { \"hello\".slice(0, 3).slice(0, 2).len() }").expect("run");
    assert_eq!(sl, 0, "String chained slice leaks {sl}");
    let (_, bl) = run_source_with_heap("fn main() -> Int { \"hello\".to_bytes().slice(1, 2).len() }").expect("run");
    assert_eq!(bl, 0, "Bytes chained slice leaks {bl}");
}

#[test]
fn bytes_binding_reused_not_consumed() {
    let src = "fn main() -> Int { let b = \"hello\".to_bytes(); b.len() + b.byte_at(0) }";
    let (v, live) = run_source_with_heap(src).expect("run");
    assert_eq!(v, Value::from_int(5 + 104));
    assert_eq!(live, 0, "read-only bytes ops must not consume: {live} live");
}

#[test]
fn bytes_byte_at_matches_source_fuzz() {
    const ALPHA: &[u8] = b"abcdefghijklmnop";
    let mut seed: u64 = 0x1234_5678_9abc_def1;
    let mut next = || { seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17; seed };
    for _ in 0..300 {
        let slen = 1 + (next() % 10) as usize;
        let s: String = (0..slen).map(|_| ALPHA[(next() as usize) % ALPHA.len()] as char).collect();
        let k = (next() % (slen as u64 + 3)) as usize;
        let src = format!("fn main() -> Int {{ let b = \"{s}\".to_bytes(); b.byte_at({k}) }}");
        if k >= s.as_bytes().len() {
            assert!(run_source_with_heap(&src).is_err(), "OOB byte_at must trap s={s:?} k={k}");
            continue;
        }
        let (v, live) = run_source_with_heap(&src).expect("run");
        assert_eq!(v, Value::from_int(s.as_bytes()[k] as i64), "s={s:?} k={k}");
        assert_eq!(live, 0, "bound bytes byte_at leak s={s:?} k={k}: {live}");
    }
}

#[test]
fn bytes_slice_len_matches_oracle_fuzz() {
    const ALPHA: &[u8] = b"abcdefgh";
    let mut seed: u64 = 0x0fed_cba9_8765_4321;
    let mut next = || { seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17; seed };
    for _ in 0..300 {
        let slen = (next() % 12) as usize;
        let s: String = (0..slen).map(|_| ALPHA[(next() as usize) % ALPHA.len()] as char).collect();
        let off = (next() % 14) as i64;
        let len = (next() % 14) as i64;
        let src = format!("fn main() -> Int {{ let b = \"{s}\".to_bytes(); b.slice({off}, {len}).len() }}");
        let (v, live) = run_source_with_heap(&src).expect("run");
        assert_eq!(v, Value::from_int(oracle_slice(&s, off, len).len() as i64), "s={s:?} off={off} len={len}");
        assert_eq!(live, 0, "bound bytes slice leak s={s:?} off={off} len={len}: {live}");
    }
}

#[test]
fn to_bytes_roundtrip_len_equals_str_len() {
    for s in ["", "a", "hello", "abcdefgh", "abcdefghi"] {
        let src = format!("fn main() -> Int {{ \"{s}\".to_bytes().len() - \"{s}\".len() }}");
        assert_eq!(run_source(&src), Ok(Value::from_int(0)), "s={s:?}");
    }
}

// Coverage for the shape the main fuzz grammar never generates: chained
// read-only builtin methods on owned temporaries. Random-depth slice chains
// terminated by len/byte_at, on both String and Bytes; asserts value oracle
// AND heap balance (the invariant the borrow/consume bug violated).
#[test]
fn chained_readonly_builtin_fuzz_value_and_no_leak() {
    const ALPHA: &[u8] = b"abcdefghij";
    let mut seed: u64 = 0xa5a5_5a5a_c3c3_3c3c;
    let mut next = || { seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17; seed };
    for _ in 0..500 {
        let slen = (next() % 12) as usize;
        let s: String = (0..slen).map(|_| ALPHA[(next() as usize) % ALPHA.len()] as char).collect();
        let as_bytes = next() & 1 == 0;
        let depth = (next() % 4) as usize;
        // Fold the same slice chain in Rust to get the oracle byte window.
        let mut cur = s.clone().into_bytes();
        let mut chain = if as_bytes {
            format!("\"{s}\".to_bytes()")
        } else {
            format!("\"{s}\"")
        };
        for _ in 0..depth {
            let off = (next() % 10) as i64;
            let len = (next() % 10) as i64;
            chain = format!("{chain}.slice({off}, {len})");
            let start = off.max(0).min(cur.len() as i64) as usize;
            let take = len.max(0) as usize;
            let end = start.saturating_add(take).min(cur.len());
            cur = cur[start..end].to_vec();
        }
        let terminal_len = next() & 1 == 0;
        if !terminal_len {
            let k = (next() % (cur.len() as u64 + 3)) as usize;
            let src = format!("fn main() -> Int {{ {chain}.byte_at({k}) }}");
            if k >= cur.len() {
                assert!(run_source_with_heap(&src).is_err(), "OOB byte_at must trap: {src}");
                continue;
            }
            let (v, live) = run_source_with_heap(&src).unwrap_or_else(|e| panic!("{src}: {e}"));
            assert_eq!(v, Value::from_int(cur[k] as i64), "{src}");
            assert_eq!(live, 0, "heap leak in `{src}`: {live} live");
            continue;
        }
        let src = format!("fn main() -> Int {{ {chain}.len() }}");
        let (v, live) = run_source_with_heap(&src).unwrap_or_else(|e| panic!("{src}: {e}"));
        assert_eq!(v, Value::from_int(cur.len() as i64), "{src}");
        assert_eq!(live, 0, "heap leak in `{src}`: {live} live");
    }
}

// --- byte string literal b"..." (Phase 2: literal -> packed Bytes const) ---

#[test]
fn byte_string_literal_len_is_raw_byte_count() {
    // 3 raw bytes; must NOT UTF-8-expand the high byte 0xc0 (that was the whole point).
    let r = run_source("fn main() -> Int { b\"\\x1f\\xc0\\xff\".len() }").expect("run");
    assert_eq!(r, Value::from_int(3));
}

#[test]
fn byte_string_literal_high_byte_is_exact() {
    let r = run_source("fn main() -> Int { b\"\\xc0\".byte_at(0) }").expect("run");
    assert_eq!(r, Value::from_int(192), "0xc0 must read back as 192, not a UTF-8 lead byte");
}

#[test]
fn byte_string_literal_ascii_bytes() {
    let r = run_source("fn main() -> Int { b\"AB\".byte_at(0) + b\"AB\".byte_at(1) }").expect("run");
    assert_eq!(r, Value::from_int(65 + 66));
}

#[test]
fn byte_string_literal_slice() {
    // slice(offset, length): from index 1, take 2 bytes.
    let r = run_source("fn main() -> Int { b\"\\x00\\x01\\x02\\x03\".slice(1, 2).len() }").expect("run");
    assert_eq!(r, Value::from_int(2));
    let hi = run_source("fn main() -> Int { b\"\\x00\\xc0\\x02\\x03\".slice(1, 2).byte_at(0) }").expect("run");
    assert_eq!(hi, Value::from_int(192), "sliced bytes stay raw");
}

#[test]
fn byte_string_literal_empty() {
    let r = run_source("fn main() -> Int { b\"\".len() }").expect("run");
    assert_eq!(r, Value::from_int(0));
}

#[test]
fn byte_string_literal_no_leak() {
    let src = "fn main() -> Int { let b = b\"\\x01\\x02\\x03\"; b.len() }";
    let (v, live) = run_source_with_heap(src).expect("run");
    assert_eq!(v, Value::from_int(3));
    assert_eq!(live, 0, "byte string literal must not leak");
}

#[test]
fn byte_string_literal_all_256_values_round_trip() {
    // Oracle: b"\x00\x01...\xff", byte_at(i) == i for every i.
    let mut lit = String::new();
    for b in 0u32..=255 { lit.push_str(&format!("\\x{:02x}", b)); }
    for i in [0usize, 1, 127, 128, 200, 255] {
        let src = format!("fn main() -> Int {{ b\"{lit}\".byte_at({i}) }}");
        let r = run_source(&src).unwrap_or_else(|e| panic!("i={i}: {e}"));
        assert_eq!(r, Value::from_int(i as i64), "byte_at({i}) must equal {i}");
    }
    let src = format!("fn main() -> Int {{ b\"{lit}\".len() }}");
    assert_eq!(run_source(&src).unwrap(), Value::from_int(256), "all 256 bytes stored");
}

#[test]
fn byte_string_static_sprite() {
    let src = "static SPR: Bytes = b\"\\x1f\\x2a\\xff\"\nfn main() -> Int { SPR.byte_at(2) }";
    let r = run_source(src).expect("run");
    assert_eq!(r, Value::from_int(255));
}

#[test]
fn byte_string_literal_fuzz_round_trips_raw_bytes() {
    // Acceptance fuzz: random byte arrays -> b"\xNN..." literal -> byte_at/len match
    // the exact source bytes. Guards against any UTF-8/encoding corruption.
    let mut seed: u64 = 0x9E3779B97F4A7C15;
    let mut next = || { seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); (seed >> 33) as u32 };
    for _ in 0..200 {
        let n = (next() % 40) as usize;
        let bytes: Vec<u8> = (0..n).map(|_| (next() & 0xff) as u8).collect();
        let mut lit = String::new();
        for b in &bytes { lit.push_str(&format!("\\x{:02x}", b)); }
        let len_src = format!("fn main() -> Int {{ b\"{lit}\".len() }}");
        assert_eq!(run_source(&len_src).unwrap(), Value::from_int(n as i64), "len mismatch for {bytes:?}");
        if n > 0 {
            let i = (next() as usize) % n;
            let src = format!("fn main() -> Int {{ b\"{lit}\".byte_at({i}) }}");
            assert_eq!(run_source(&src).unwrap(), Value::from_int(bytes[i] as i64),
                "byte_at({i}) mismatch for {bytes:?}");
        }
    }
}

#[test]
fn byte_string_packs_denser_than_int_array() {
    // 64 bytes: Bytes packs 8/word (~9 words); Array<Int> is 1 word/elem (~65 words).
    let bytes_prog = "fn main() -> Int { let b = b\"\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\"; b.byte_at(0) }";
    let arr_prog = "fn main() -> Int { let a = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]; a[0] }";
    let bytes_used = run_source_bytes_used(bytes_prog).expect("bytes prog");
    let arr_used = run_source_bytes_used(arr_prog).expect("arr prog");
    assert!(bytes_used * 4 < arr_used,
        "64-byte Bytes ({}) must use <1/4 the heap of a 64-int Array ({})", bytes_used, arr_used);
}

// Indexing through a reference must peel `&` and still dispatch byte_at, not the
// array raw-cell Ld path (which reads a packed word → OOB). Regression.
#[test]
fn index_through_ref_to_string_dispatches_byte_at() {
    let src = "static S: String = \"abc\"; fn main() -> Int { let r = &S; r[1] }";
    assert_eq!(run_source(src), Ok(Value::from_int(98)));
}

#[test]
fn index_through_ref_to_bytes_dispatches_byte_at() {
    let src = "static X: Bytes = b\"\\x01\\x02\\x03\"; fn main() -> Int { let r = &X; r[2] }";
    assert_eq!(run_source(src), Ok(Value::from_int(3)));
}

// Reference-dimension coverage: every String/Bytes op must work through `&T`,
// `r = &T`, `(&T)`, and `*r` — codegen dispatch predicates peel Reference (typeck
// already does). The `&static` + `(&Bytes)[i]` bugs both lived in this omitted axis.
#[test]
fn string_bytes_ops_work_through_references() {
    let hdr = "static X: Bytes = b\"\\x01\\x02\\x03\"; static S: String = \"abc\";";
    let cases: [(&str, i64); 11] = [
        ("let r=&X; r.len()", 3),
        ("let r=&X; r[1]", 2),
        ("let r=&X; r.byte_at(2)", 3),
        ("let r=&X; r.slice(0,2).len()", 2),
        ("(&X).len()", 3),
        ("(&X)[0]", 1),
        ("let r=&X; (*r)[2]", 3),
        ("let r=&S; r.len()", 3),
        ("let r=&S; r[1]", 98),
        ("let r=&S; r.slice(1,2).len()", 2),
        ("(&S)[2]", 99),
    ];
    for (body, want) in cases {
        let src = format!("{hdr} fn main() -> Int {{ {body} }}");
        assert_eq!(run_source(&src), Ok(Value::from_int(want)), "`{body}`");
    }
}

#[test]
fn bytes_concat_joins_self_then_parts_in_order() {
    assert_eq!(run_source("fn main() -> Int { b\"\\x01\".concat([b\"\\x02\", b\"\\x03\"]).byte_at(2) }"),
        Ok(Value::from_int(3)));
}
#[test]
fn bytes_concat_length_is_sum() {
    assert_eq!(run_source("fn main() -> Int { b\"\\x01\".concat([b\"\\x02\", b\"\\x03\"]).len() }"),
        Ok(Value::from_int(3)));
}
#[test]
fn bytes_concat_self_first_byte_preserved() {
    assert_eq!(run_source("fn main() -> Int { b\"ab\".concat([b\"cd\"]).byte_at(0) }"),
        Ok(Value::from_int(97)));
}
#[test]
fn bytes_concat_part_byte_at_boundary() {
    assert_eq!(run_source("fn main() -> Int { b\"ab\".concat([b\"cd\"]).byte_at(2) }"),
        Ok(Value::from_int(99)));
}
#[test]
fn bytes_concat_no_leak() {
    let (v, live) = run_source_with_heap("fn main() -> Int { b\"\\x01\".concat([b\"\\x02\"]).byte_at(1) }").expect("run");
    assert_eq!(v, Value::from_int(2));
    assert_eq!(live, 0, "concat leaked");
}
#[test]
fn bytes_concat_empty_parts_rejected() {
    assert!(run_source("fn main() -> Int { b\"a\".concat([]).len() }").is_err());
}
#[test]
fn bytes_concat_wrong_element_type_rejected() {
    assert!(run_source("fn main() -> Int { b\"a\".concat([1, 2]).len() }").is_err());
}
#[test]
fn bytes_concat_consumes_receiver_reuse_rejected() {
    let src = "fn main() -> Int { let x = b\"ab\"; let y = x.concat([b\"cd\"]); y.len() + x.len() }";
    assert!(run_source(src).is_err(), "receiver reused after concat must be rejected");
}
