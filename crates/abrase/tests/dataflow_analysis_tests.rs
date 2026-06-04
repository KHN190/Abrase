// Tests for the bytecode register-liveness analysis (compiler::dataflow).

use abrase::bytecode::{OpCode::*, Register as R};
use abrase::compiler::dataflow::{is_last_use, live_out};

// r live in live_out[pc] (i.e. may be read after code[pc]).
fn live_after(lo: &[u128], pc: usize, r: u8) -> bool {
    (lo[pc] & (1u128 << r)) != 0
}

#[test]
fn straight_line_last_use_and_def() {
    // r1 = 5; r2 = r1 + r1; ret r2
    let code = vec![
        PushConst(R(1), 0),
        Add(R(2), R(1), R(1)),
        Ret(R(2)),
    ];
    let lo = live_out(&code);
    assert!(is_last_use(&lo, 1, R(1)), "r1 dead after the Add (its only use)");
    assert!(!is_last_use(&lo, 1, R(2)), "r2 read by the following Ret");
    assert!(live_after(&lo, 0, 1), "r1 live between its def and the Add");
}

#[test]
fn dead_store_dest_never_live() {
    // r1 = 5 (never read); r2 = 9; ret r2.  r1 is a dead store.
    let code = vec![
        PushConst(R(1), 0),
        PushConst(R(2), 1),
        Ret(R(2)),
    ];
    let lo = live_out(&code);
    assert!(!live_after(&lo, 0, 1), "r1 dead immediately after its def = dead store");
    assert!(live_after(&lo, 1, 2), "r2 live into Ret");
}

#[test]
fn first_use_not_last_when_used_again() {
    // r1 = 5; r2 = r1 + r1; r3 = r1 + r2; ret r3.  r1 used at op1 and op2.
    let code = vec![
        PushConst(R(1), 0),
        Add(R(2), R(1), R(1)),
        Add(R(3), R(1), R(2)),
        Ret(R(3)),
    ];
    let lo = live_out(&code);
    assert!(!is_last_use(&lo, 1, R(1)), "r1 still read at op2 → op1 is NOT its last use");
    assert!(is_last_use(&lo, 2, R(1)), "op2 is r1's last use");
}

#[test]
fn redef_makes_old_value_dead() {
    // r1 = 1; r1 = 2 (redef, old value never read); ret r1
    let code = vec![
        PushConst(R(1), 0),
        PushConst(R(1), 1),
        Ret(R(1)),
    ];
    let lo = live_out(&code);
    // After the first def, r1 is overwritten before any read → dead there.
    assert!(!live_after(&lo, 0, 1), "first r1 value dead (redefined before use)");
    assert!(live_after(&lo, 1, 1), "second r1 value live into Ret");
}

#[test]
fn drop_counts_as_use() {
    // r1 = Alloc; drop r1; ret r0.  Drop reads r1 → r1's last use is the Drop.
    let code = vec![
        Alloc(R(1), 1),
        Drop(R(1)),
        Ret(R(0)),
    ];
    let lo = live_out(&code);
    assert!(live_after(&lo, 0, 1), "r1 live from Alloc to its Drop");
    assert!(is_last_use(&lo, 1, R(1)), "Drop is r1's last use");
}

#[test]
fn branch_keeps_var_live_on_one_path() {
    // 0: Jz r0 -> fallthrough only reads r1; r1 live after the branch via that path.
    let code = vec![
        Jz(R(0), 2),  // target = 0+1+2 = 3 = off-end (exit) → only fallthrough succ
        Ret(R(1)),    // 1: reads r1
        Ret(R(0)),    // 2
    ];
    let lo = live_out(&code);
    assert!(live_after(&lo, 0, 1), "r1 live after the conditional (read on a successor path)");
}

#[test]
fn live_across_call_is_conservative() {
    // r1 = 1; r3 = call f (defs r3, our model reads no reg); r2 = r1 + r3; ret r2.
    // r1 must stay live ACROSS the Call (read afterwards).
    let code = vec![
        PushConst(R(1), 0),
        Call(R(3), 0),
        Add(R(2), R(1), R(3)),
        Ret(R(2)),
    ];
    let lo = live_out(&code);
    assert!(live_after(&lo, 1, 1), "r1 live across the Call (used after)");
    assert!(is_last_use(&lo, 2, R(1)), "r1 last used at the Add after the call");
}

#[test]
fn loop_back_edge_keeps_live() {
    // 0: Add r2,r1,r1   1: Jnz r0 -> back to 0.  r1 read each iteration.
    let code = vec![
        Add(R(2), R(1), R(1)),
        Jnz(R(0), -2), // target = 1+1-2 = 0 (back-edge)
    ];
    let lo = live_out(&code);
    assert!(live_after(&lo, 1, 1), "r1 live across the loop back-edge");
    assert!(!is_last_use(&lo, 0, R(1)), "r1 not last-used inside the loop body");
}

#[test]
fn empty_and_single_op() {
    assert_eq!(live_out(&[]).len(), 0);
    let lo = live_out(&[Ret(R(0))]);
    assert_eq!(lo.len(), 1);
    assert!(!live_after(&lo, 0, 0), "nothing live after the only Ret");
}
