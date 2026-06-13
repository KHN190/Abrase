mod common;
use common::*;
use polka::{Module, OpCode, Chunk};
use polka_rustc::transpile_module;
use myriad::Value;

#[test]
fn heap_alloc_store_load_drop_no_leak() {
    let main = fn_chunk(
        vec![
            OpCode::Alloc(r(0), 2),
            OpCode::PushConst(r(1), 0),
            OpCode::St(r(1), r(0), 0),
            OpCode::Ld(r(2), r(0), 0),
            OpCode::Drop(r(0)),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(42).raw()], 4, 0,
    );
    assert_same_heap(vec![main], 0);
}

#[test]
fn heap_leak_detected_when_not_dropped() {
    let main = fn_chunk(
        vec![
            OpCode::Alloc(r(0), 2),
            OpCode::PushConst(r(1), 0),
            OpCode::St(r(1), r(0), 0),
            OpCode::Ld(r(2), r(0), 0),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(42).raw()], 4, 0,
    );
    assert_same_heap(vec![main], 0);
}

#[test]
fn nested_handle_drop_cascades_to_zero() {
    let main = fn_chunk(
        vec![
            OpCode::Alloc(r(0), 1),
            OpCode::Alloc(r(1), 1),
            OpCode::St(r(1), r(0), 0),
            OpCode::Drop(r(0)),
            OpCode::PushConst(r(2), 0),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(0).raw()], 4, 0,
    );
    assert_same_heap(vec![main], 0);
}

#[test]
fn copy_handle_increments_refcount() {
    let main = fn_chunk(
        vec![
            OpCode::Alloc(r(0), 1),
            OpCode::Copy(r(1), r(0)),
            OpCode::Drop(r(0)),
            OpCode::Drop(r(1)),
            OpCode::PushConst(r(2), 0),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(0).raw()], 4, 0,
    );
    assert_same_heap(vec![main], 0);
}

#[test]
fn handle_passed_through_call_no_leak() {
    let main = fn_chunk(
        vec![
            OpCode::Alloc(r(0), 1),
            OpCode::PushConst(r(5), 0),
            OpCode::St(r(5), r(0), 0),
            OpCode::Copy(r(4), r(0)),
            OpCode::Call(r(1), 1),
            OpCode::Drop(r(0)),
            OpCode::Ret(r(1)),
        ],
        vec![Value::from_int(99).raw()], 4, 0,
    );
    let reader = fn_chunk(
        vec![
            OpCode::Ld(r(1), r(0), 0),
            OpCode::Drop(r(0)),
            OpCode::Ret(r(1)),
        ],
        vec![], 2, 1,
    );
    assert_same_heap(vec![main, reader], 0);
}

#[test]
fn ldidx_stidx_dynamic_offset_roundtrips() {
    let main = fn_chunk(
        vec![
            OpCode::Alloc(r(0), 3),
            OpCode::PushConst(r(1), 0),
            OpCode::PushConst(r(2), 1),
            OpCode::StIdx(r(1), r(0), r(2)),
            OpCode::LdIdx(r(3), r(0), r(2)),
            OpCode::Drop(r(0)),
            OpCode::Ret(r(3)),
        ],
        vec![Value::from_int(55).raw(), Value::from_int(2).raw()], 4, 0,
    );
    assert_same_heap(vec![main], 0);
}

#[test]
fn stidx_negative_index_errors_both_sides() {
    let main = fn_chunk(
        vec![
            OpCode::Alloc(r(0), 2),
            OpCode::PushConst(r(1), 0),
            OpCode::PushConst(r(2), 1),
            OpCode::StIdx(r(1), r(0), r(2)),
            OpCode::Drop(r(0)),
            OpCode::Ret(r(1)),
        ],
        vec![Value::from_int(7).raw(), Value::from_int(-1).raw()], 4, 0,
    );
    let module = Module { functions: vec![main], entry: 0, flags: 0, exports: vec![] };
    let i = run_module_outcome(&module);
    let t = compile_run(&transpile_module(&module).expect("transpile"));
    compare(&i, &t);
}

// Heap fuzzer: well-formed by construction. Alloc K cells, do random scalar
// St/Ld on valid (handle, offset), drop every handle, return a scalar. RC stays
// balanced so live-count must agree; catches misplaced rc_inc/rc_dec.
fn random_heap_program(rng: &mut Rng) -> Chunk {
    let k = 1 + rng.below(3);
    let size = 1 + rng.below(3);
    let hbase = 0u8;
    let scratch = (hbase as usize + k) as u8;
    let retreg = scratch + 1;
    let nconst = 3;
    let constants: Vec<u64> = (0..nconst)
        .map(|_| Value::from_int([0i64, 1, -1, 42, 7][rng.below(5)]).raw())
        .collect();
    let mut code = Vec::new();
    for i in 0..k { code.push(OpCode::Alloc(r(hbase + i as u8), size as u16)); }
    let ops = 2 + rng.below(6);
    for _ in 0..ops {
        let hreg = r(hbase + rng.below(k) as u8);
        let off = rng.below(size) as u16;
        if rng.below(2) == 0 {
            code.push(OpCode::PushConst(r(scratch), rng.below(nconst) as u16));
            code.push(OpCode::St(r(scratch), hreg, off));
        } else {
            code.push(OpCode::Ld(r(retreg), hreg, off));
        }
    }
    for i in 0..k { code.push(OpCode::Drop(r(hbase + i as u8))); }
    code.push(OpCode::PushConst(r(retreg), 0));
    code.push(OpCode::Ret(r(retreg)));
    fn_chunk(code, constants, (retreg + 1) as usize, 0)
}

#[test]
fn fuzz_heap_programs_balance_refcounts() {
    let n: u64 = std::env::var("FUZZ_N").ok().and_then(|s| s.parse().ok()).unwrap_or(30);
    let mut rng = Rng(0xC0FFEE123456789);
    let (mut mods, mut interps) = (Vec::new(), Vec::new());
    for _ in 0..n {
        let main = random_heap_program(&mut rng);
        let m = Module { functions: vec![main], entry: 0, flags: 0, exports: vec![] };
        interps.push(interp_with_live(&m));
        mods.push(m);
    }
    batch_compare(mods, interps, true);
}
