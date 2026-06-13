use crate::common::*;
use polka::{Chunk, Module, OpCode};
use polka_rustc::transpile_module;
use myriad::Value;

#[test]
fn call_square_function() {
    let main = fn_chunk(
        vec![OpCode::PushConst(r(4), 0), OpCode::Call(r(0), 1), OpCode::Ret(r(0))],
        vec![Value::from_int(9).raw()], 4, 0,
    );
    let square = fn_chunk(
        vec![OpCode::Mul(r(1), r(0), r(0)), OpCode::Ret(r(1))],
        vec![], 2, 1,
    );
    assert_same_module(vec![main, square], 0);
}

#[test]
fn recursive_factorial() {
    let main = fn_chunk(
        vec![OpCode::PushConst(r(4), 0), OpCode::Call(r(0), 1), OpCode::Ret(r(0))],
        vec![Value::from_int(5).raw()], 4, 0,
    );
    let fact = fn_chunk(
        vec![
            OpCode::PushConst(r(1), 0),
            OpCode::Lte(r(2), r(0), r(1)),
            OpCode::Jz(r(2), 1),
            OpCode::Ret(r(1)),
            OpCode::SubImm(r(3), r(0), 1),
            OpCode::Copy(r(5), r(3)),
            OpCode::Call(r(4), 1),
            OpCode::Mul(r(4), r(0), r(4)),
            OpCode::Ret(r(4)),
        ],
        vec![Value::from_int(1).raw()], 5, 1,
    );
    assert_same_module(vec![main, fact], 0);
}

fn random_leaf(rng: &mut Rng, nparams: usize) -> Chunk {
    let edge = [0i64, 1, -1, 2, i64::MAX, i64::MIN, 7];
    let nconst = 1 + rng.below(3);
    let constants: Vec<u64> = (0..nconst)
        .map(|_| Value::from_int(edge[rng.below(edge.len())]).raw())
        .collect();
    let nreg = (nparams + 4).max(4);
    let mut defined: Vec<u8> = (0..nparams as u8).collect();
    if defined.is_empty() { defined.push(0); }
    let mut code = Vec::new();
    code.push(OpCode::PushConst(r(nparams as u8), 0));
    if !defined.contains(&(nparams as u8)) { defined.push(nparams as u8); }
    let body = 2 + rng.below(6);
    for _ in 0..body {
        let d = r(rng.below(nreg) as u8);
        let a = r(defined[rng.below(defined.len())]);
        let b = r(defined[rng.below(defined.len())]);
        code.push(random_op(rng, d, a, b, nconst));
        if !defined.contains(&d.0) { defined.push(d.0); }
    }
    if rng.below(4) != 0 {
        code.push(OpCode::Ret(r(defined[rng.below(defined.len())])));
    }
    fn_chunk(code, constants, nreg, nparams)
}

#[test]
fn fuzz_call_programs_match_interpreter() {
    let n: u64 = std::env::var("FUZZ_N").ok().and_then(|s| s.parse().ok()).unwrap_or(40);
    let mut rng = Rng(0xD1B54A32D192ED03);
    let (mut mods, mut interps) = (Vec::new(), Vec::new());
    for _ in 0..n {
        let nparams = 1 + rng.below(3);
        let leaf = random_leaf(&mut rng, nparams);
        let mreg = 4;
        let nconst = nparams;
        let constants: Vec<u64> = (0..nconst)
            .map(|_| Value::from_int([0i64, 1, -1, 2, 7, -7][rng.below(6)]).raw())
            .collect();
        let mut code = Vec::new();
        for i in 0..nparams {
            code.push(OpCode::PushConst(r((mreg + i) as u8), i as u16));
        }
        code.push(OpCode::Call(r(0), 1));
        code.push(OpCode::Ret(r(0)));
        let main = fn_chunk(code, constants, mreg, 0);
        let module = Module { functions: vec![main, leaf], entry: 0, flags: 0, exports: vec![] };
        let i = run_module_outcome(&module);
        if let Outcome::Err(e) = &i { if e.contains("step cap") { continue; } }
        interps.push((i, 0));
        mods.push(module);
    }
    batch_compare(mods, interps, false);
}

#[test]
fn two_param_call() {
    let main = fn_chunk(
        vec![
            OpCode::PushConst(r(4), 0),
            OpCode::PushConst(r(5), 1),
            OpCode::Call(r(0), 1),
            OpCode::Ret(r(0)),
        ],
        vec![Value::from_int(20).raw(), Value::from_int(22).raw()], 4, 0,
    );
    let add2 = fn_chunk(
        vec![OpCode::Add(r(2), r(0), r(1)), OpCode::Ret(r(2))],
        vec![], 3, 2,
    );
    assert_same_module(vec![main, add2], 0);
}

#[test]
fn unreferenced_native_fn_transpiles_via_stub() {
    use polka::{Chunk as PChunk, NativeChunk};
    let main = fn_chunk(vec![OpCode::PushConst(r(0), 0), OpCode::Ret(r(0))], vec![Value::from_int(1).raw()], 4, 0);
    let native = PChunk::Native(NativeChunk { name: "print".into(), param_count: 1 });
    assert_same_module(vec![main, native], 0);
}

#[test]
fn callreg_to_native_releases_staged_handle_args() {
    use polka::{Chunk as PChunk, NativeChunk};
    let main = fn_chunk(
        vec![
            OpCode::Alloc(r(0), 1),
            OpCode::Alloc(r(1), 1),
            OpCode::Copy(r(8), r(0)),
            OpCode::Copy(r(9), r(1)),
            OpCode::PushConst(r(2), 0),
            OpCode::CallReg(r(3), r(2)),
            OpCode::Drop(r(0)),
            OpCode::Drop(r(1)),
            OpCode::Drop(r(3)),
            OpCode::PushConst(r(4), 1),
            OpCode::Ret(r(4)),
        ],
        vec![Value::from_int(1).raw(), Value::from_int(0).raw()], 8, 0,
    );
    let concat = PChunk::Native(NativeChunk { name: "__concat".into(), param_count: 2 });
    assert_same_heap(vec![main, concat], 0);
}

#[test]
fn error_propagates_across_call_frames() {
    let main = fn_chunk(
        vec![
            OpCode::PushConst(r(4), 0),
            OpCode::PushConst(r(5), 1),
            OpCode::Call(r(0), 1),
            OpCode::Ret(r(0)),
        ],
        vec![Value::from_int(10).raw(), Value::from_int(0).raw()], 4, 0,
    );
    let divide = fn_chunk(
        vec![OpCode::Div(r(2), r(0), r(1)), OpCode::Ret(r(2))],
        vec![], 3, 2,
    );
    assert_same_module(vec![main, divide], 0);
}

#[test]
fn nested_call_chain_two_levels() {
    let main = fn_chunk(
        vec![OpCode::PushConst(r(4), 0), OpCode::Call(r(0), 1), OpCode::Ret(r(0))],
        vec![Value::from_int(6).raw()], 4, 0,
    );
    let mid = fn_chunk(
        vec![OpCode::Copy(r(2), r(0)), OpCode::Call(r(1), 2), OpCode::AddImm(r(1), r(1), 1), OpCode::Ret(r(1))],
        vec![], 2, 1,
    );
    let leaf = fn_chunk(
        vec![OpCode::Mul(r(1), r(0), r(0)), OpCode::Ret(r(1))],
        vec![], 2, 1,
    );
    assert_same_module(vec![main, mid, leaf], 0);
}

#[test]
fn deep_recursion_chain_300_levels() {
    let main = fn_chunk(
        vec![OpCode::PushConst(r(4), 0), OpCode::Call(r(0), 1), OpCode::Ret(r(0))],
        vec![Value::from_int(300).raw()], 4, 0,
    );
    let sum = fn_chunk(
        vec![
            OpCode::PushConst(r(1), 0),
            OpCode::Eq(r(2), r(0), r(1)),
            OpCode::Jz(r(2), 1),
            OpCode::Ret(r(1)),
            OpCode::SubImm(r(3), r(0), 1),
            OpCode::Copy(r(6), r(3)),
            OpCode::Call(r(4), 1),
            OpCode::Add(r(4), r(0), r(4)),
            OpCode::Ret(r(4)),
        ],
        vec![Value::from_int(0).raw()], 6, 1,
    );
    assert_same_module(vec![main, sum], 0);
}

#[test]
fn module_with_exports_transpiles_via_entry() {
    use polka::Export;
    let main = fn_chunk(vec![OpCode::PushConst(r(0), 0), OpCode::Ret(r(0))], vec![Value::from_int(9).raw()], 4, 0);
    let helper = fn_chunk(vec![OpCode::Ret(r(0))], vec![], 2, 1);
    let module = Module {
        functions: vec![main, helper],
        entry: 0, flags: 0,
        exports: vec![Export { name: "helper".into(), fn_id: 1 }],
    };
    let i = run_module_outcome(&module);
    let t = compile_run(&transpile_module(&module).expect("transpile"));
    compare(&i, &t);
}

#[test]
fn callreg_dispatches_to_runtime_fn_id() {
    let main = fn_chunk(
        vec![
            OpCode::PushConst(r(5), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::CallReg(r(0), r(1)),
            OpCode::Ret(r(0)),
        ],
        vec![Value::from_int(8).raw(), Value::from_int(1).raw()], 5, 0,
    );
    let square = fn_chunk(
        vec![OpCode::Mul(r(1), r(0), r(0)), OpCode::Ret(r(1))],
        vec![], 2, 1,
    );
    assert_same_module(vec![main, square], 0);
}

#[test]
fn callreg_out_of_u16_range_errors_both_sides() {
    let main = fn_chunk(
        vec![
            OpCode::PushConst(r(1), 0),
            OpCode::CallReg(r(0), r(1)),
            OpCode::Ret(r(0)),
        ],
        vec![Value::from_int(70000).raw()], 5, 0,
    );
    let helper = fn_chunk(vec![OpCode::Ret(r(0))], vec![], 2, 1);
    assert_same_module(vec![main, helper], 0);
}

#[test]
fn callreg_unknown_fn_id_errors_both_sides() {
    let main = fn_chunk(
        vec![
            OpCode::PushConst(r(1), 0),
            OpCode::CallReg(r(0), r(1)),
            OpCode::Ret(r(0)),
        ],
        vec![Value::from_int(9).raw()], 5, 0,
    );
    let helper = fn_chunk(vec![OpCode::Ret(r(0))], vec![], 2, 1);
    assert_same_module(vec![main, helper], 0);
}

#[test]
fn effect_ops_transpile_via_embed() {
    use polka_rustc::transpile_program;
    // Effect ops route to the A1 embed backend (VM-in-binary), so they transpile
    // rather than erroring. End-to-end behavior is covered by tests/diff/effect.rs.
    for op in [
        OpCode::Handle(r(0), 0),
        OpCode::Resume(r(0), r(1)),
        OpCode::Raise(r(0), r(1), r(2)),
    ] {
        let bc = chunk(vec![op, OpCode::Ret(r(0))], vec![], 4);
        assert!(transpile_program(&bc).is_ok(), "effect op should transpile via embed backend");
    }
}
