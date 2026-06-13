use crate::common::*;
use polka::{OpCode, Chunk};

#[test]
fn string_const_resolves_to_same_handle_and_live_count() {
    let main = str_const_chunk(
        vec![OpCode::PushConst(r(0), 0), OpCode::Ret(r(0))],
        vec![0], vec![0b1], vec!["hello".into()], 4, 0,
    );
    assert_same_heap(vec![main], 0);
}

#[test]
fn string_const_push_then_drop_keeps_module_lifetime_handle() {
    let main = str_const_chunk(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::Drop(r(0)),
            OpCode::PushConst(r(1), 1),
            OpCode::Ret(r(1)),
        ],
        vec![0, myriad::Value::from_int(7).raw()], vec![0b01], vec!["world".into()], 4, 0,
    );
    assert_same_heap(vec![main], 0);
}

#[test]
fn multiple_string_consts_alloc_in_order() {
    let main = str_const_chunk(
        vec![
            OpCode::PushConst(r(0), 0),
            OpCode::PushConst(r(1), 1),
            OpCode::Move(r(2), r(1)),
            OpCode::Ret(r(2)),
        ],
        vec![0, 1], vec![0b11], vec!["aa".into(), "bbbb".into()], 4, 0,
    );
    assert_same_heap(vec![main], 0);
}

// String-const fuzzer: const 0..nstr are handle (string-pool) consts, the rest
// scalars. Body does PushConst (rc_inc on handle consts), Copy (rc_inc), Drop
// (rc_dec), Move (transfer). RC stays well-formed; user-visible live-count must
// agree, catching misplaced rc on module-lifetime string handles.
fn random_strconst_program(rng: &mut Rng) -> Chunk {
    let nstr = 1 + rng.below(3);
    let nscalar = 1 + rng.below(2);
    let nconst = nstr + nscalar;
    let mut constants: Vec<u64> = (0..nstr).map(|i| i as u64).collect();
    for _ in 0..nscalar {
        constants.push(myriad::Value::from_int([0i64, 1, 7, -3][rng.below(4)]).raw());
    }
    let mut mask = 0u64;
    for i in 0..nstr { mask |= 1 << i; }
    let strings: Vec<String> = (0..nstr).map(|i| "x".repeat(1 + i + rng.below(4))).collect();
    let nreg = 6;
    let mut defined: Vec<u8> = Vec::new();
    let mut handle_reg: Vec<bool> = vec![false; nreg];
    let mut code = Vec::new();
    let body = 4 + rng.below(8);
    for _ in 0..body {
        match rng.below(5) {
            0 | 1 => {
                let d = rng.below(nreg);
                let ci = rng.below(nconst);
                if handle_reg[d] { code.push(OpCode::Drop(r(d as u8))); }
                code.push(OpCode::PushConst(r(d as u8), ci as u16));
                handle_reg[d] = ci < nstr;
                if !defined.contains(&(d as u8)) { defined.push(d as u8); }
            }
            2 => {
                if defined.is_empty() { continue; }
                let s = defined[rng.below(defined.len())] as usize;
                let d = rng.below(nreg);
                if handle_reg[d] { code.push(OpCode::Drop(r(d as u8))); }
                code.push(OpCode::Copy(r(d as u8), r(s as u8)));
                handle_reg[d] = handle_reg[s];
                if !defined.contains(&(d as u8)) { defined.push(d as u8); }
            }
            3 => {
                let d = rng.below(nreg);
                if handle_reg[d] { code.push(OpCode::Drop(r(d as u8))); handle_reg[d] = false; }
            }
            _ => {
                if defined.is_empty() { continue; }
                let s = defined[rng.below(defined.len())] as usize;
                let d = rng.below(nreg);
                if d == s { continue; }
                if handle_reg[d] { code.push(OpCode::Drop(r(d as u8))); }
                code.push(OpCode::Move(r(d as u8), r(s as u8)));
                handle_reg[d] = handle_reg[s];
                handle_reg[s] = false;
                if !defined.contains(&(d as u8)) { defined.push(d as u8); }
            }
        }
    }
    for d in 0..nreg {
        if handle_reg[d] { code.push(OpCode::Drop(r(d as u8))); }
    }
    let scalar_idx = nstr as u16;
    code.push(OpCode::PushConst(r(nreg as u8), scalar_idx));
    code.push(OpCode::Ret(r(nreg as u8)));
    str_const_chunk(code, constants, vec![mask], strings, nreg + 1, 0)
}

#[test]
fn fuzz_strconst_programs_balance_module_lifetime_handles() {
    let n: u64 = std::env::var("FUZZ_N").ok().and_then(|s| s.parse().ok()).unwrap_or(30);
    let mut rng = Rng(0x5712E3CA0FF1CE);
    let (mut mods, mut interps) = (Vec::new(), Vec::new());
    for _ in 0..n {
        let main = random_strconst_program(&mut rng);
        let m = polka::Module { functions: vec![main], entry: 0, flags: 0, exports: vec![] };
        interps.push(interp_with_live(&m));
        mods.push(m);
    }
    batch_compare(mods, interps, true);
}
