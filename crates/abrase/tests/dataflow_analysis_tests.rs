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

#[test]
fn st_reads_value_and_base() {
    // r1=Alloc; r2=const; St(r2, r1, 0); ret r0.  Both r1,r2 live until the St.
    let code = vec![Alloc(R(1), 1), PushConst(R(2), 0), St(R(2), R(1), 0), Ret(R(0))];
    let lo = live_out(&code);
    assert!(live_after(&lo, 1, 1) && live_after(&lo, 1, 2), "St keeps value+base live");
    assert!(is_last_use(&lo, 2, R(1)) && is_last_use(&lo, 2, R(2)), "St is their last use");
}

#[test]
fn stidx_reads_all_three() {
    let code = vec![StIdx(R(1), R(2), R(3)), Ret(R(0))];
    let lo = live_out(&code);
    assert!(is_last_use(&lo, 0, R(1)) && is_last_use(&lo, 0, R(2)) && is_last_use(&lo, 0, R(3)));
}

#[test]
fn ld_reads_base_defs_dest() {
    let code = vec![Ld(R(2), R(1), 0), Ret(R(2))];
    let lo = live_out(&code);
    assert!(is_last_use(&lo, 0, R(1)));
    assert!(live_after(&lo, 0, 2));
}

#[test]
fn move_and_copy_read_source() {
    let code = vec![Move(R(2), R(1)), Copy(R(3), R(2)), Ret(R(3))];
    let lo = live_out(&code);
    assert!(is_last_use(&lo, 0, R(1)), "Move reads r1");
    assert!(is_last_use(&lo, 1, R(2)), "Copy reads r2");
    assert!(live_after(&lo, 1, 3));
}

#[test]
fn deo_reads_both_dei_reads_port_defs_dest() {
    let code = vec![Deo(R(1), R(2)), Dei(R(3), R(4)), Ret(R(3))];
    let lo = live_out(&code);
    assert!(is_last_use(&lo, 0, R(1)) && is_last_use(&lo, 0, R(2)), "Deo reads src+port");
    assert!(is_last_use(&lo, 1, R(4)), "Dei reads port");
    assert!(live_after(&lo, 1, 3), "Dei defs dest, live into Ret");
}

#[test]
fn raise_reads_key_arg_resume_reads_value() {
    // Liveness = "will be read", not "was defined": Raise's dest r3 is read by
    // the final Ret so it stays live (incl. across the Resume); Resume's dest r5
    // is never read → dead def.
    let code = vec![Raise(R(3), R(1), R(2)), Resume(R(5), R(4)), Ret(R(3))];
    let lo = live_out(&code);
    assert!(is_last_use(&lo, 0, R(1)) && is_last_use(&lo, 0, R(2)), "Raise reads key+arg");
    assert!(live_after(&lo, 0, 3), "r3 read later → live across Resume");
    assert!(is_last_use(&lo, 1, R(4)), "Resume reads its value");
    assert!(!live_after(&lo, 1, 5), "r5 defined but never read → dead");
}

#[test]
fn handle_reads_table_callreg_reads_fn() {
    let code = vec![Handle(R(1), 0), CallReg(R(3), R(2)), Ret(R(3))];
    let lo = live_out(&code);
    assert!(is_last_use(&lo, 0, R(1)), "Handle reads the table reg");
    assert!(is_last_use(&lo, 1, R(2)), "CallReg reads the fn-value reg");
    assert!(live_after(&lo, 1, 3), "CallReg defs dest");
}

#[test]
fn float_ops_and_imm_use_def() {
    let code = vec![FAdd(R(3), R(1), R(2)), AddImm(R(4), R(3), 1), Ret(R(4))];
    let lo = live_out(&code);
    assert!(is_last_use(&lo, 0, R(1)) && is_last_use(&lo, 0, R(2)));
    assert!(is_last_use(&lo, 1, R(3)), "AddImm reads its source");
    assert!(live_after(&lo, 1, 4));
}

#[test]
#[ignore = "measurement probe, prints a report"]
fn report_dead_store_ceiling() {
    use abrase::bytecode::{Chunk, OpCode};
    use abrase::compiler::Compiler;
    use abrase::lexer::Lexer;
    use abrase::parser::Parser;

    fn pure_dest(op: &OpCode) -> Option<abrase::bytecode::Register> {
        use OpCode::*;
        match op {
            PushConst(d,_) | Add(d,_,_) | Sub(d,_,_) | Mul(d,_,_) | Div(d,_,_) | Mod(d,_,_)
            | Neg(d,_) | Eq(d,_,_) | Neq(d,_,_) | Lt(d,_,_) | Gt(d,_,_) | Lte(d,_,_) | Gte(d,_,_)
            | And(d,_,_) | Or(d,_,_) | Xor(d,_,_) | Shl(d,_,_) | Shr(d,_,_)
            | AddImm(d,_,_) | SubImm(d,_,_)
            | FAdd(d,_,_) | FSub(d,_,_) | FMul(d,_,_) | FDiv(d,_,_) | FNeg(d,_)
            | FLt(d,_,_) | FEq(d,_,_) => Some(*d),
            _ => None,
        }
    }

    for name in ["nqueens", "mandelbrot", "ackermann", "merge_sort", "coin_change"] {
        let path = format!("{}/../../examples/{}.abe", env!("CARGO_MANIFEST_DIR"), name);
        let Ok(src) = std::fs::read_to_string(&path) else { continue };
        let mut p = Parser::new(Lexer::new(&src)).with_source(src.clone());
        let ast = p.parse_program();
        let mut c = Compiler::new().with_source(src);
        let Ok(module) = c.compile_module(&ast) else { continue };
        let (mut dead, mut total) = (0usize, 0usize);
        for ch in &module.functions {
            if let Chunk::Bytecode(bc) = ch {
                let lo = live_out(&bc.code);
                for (pc, op) in bc.code.iter().enumerate() {
                    total += 1;
                    if let Some(d) = pure_dest(op) {
                        if (lo[pc] & (1u128 << d.0)) == 0 { dead += 1; }
                    }
                }
            }
        }
        eprintln!("{:<12} dead pure stores: {:>4} / {:>6} static ops ({:.1}%)",
            name, dead, total, dead as f64 * 100.0 / total.max(1) as f64);
    }
}

#[test]
#[ignore = "measurement probe, prints a report"]
fn report_copy_coalesce_ceiling() {
    use abrase::bytecode::{Chunk, OpCode};
    use abrase::compiler::Compiler;
    use abrase::lexer::Lexer;
    use abrase::parser::Parser;

    fn dest_of(op: &OpCode) -> Option<abrase::bytecode::Register> {
        use OpCode::*;
        match op {
            PushConst(d,_) | Add(d,_,_) | Sub(d,_,_) | Mul(d,_,_) | Div(d,_,_) | Mod(d,_,_)
            | Neg(d,_) | Eq(d,_,_) | Neq(d,_,_) | Lt(d,_,_) | Gt(d,_,_) | Lte(d,_,_) | Gte(d,_,_)
            | And(d,_,_) | Or(d,_,_) | Xor(d,_,_) | Shl(d,_,_) | Shr(d,_,_)
            | AddImm(d,_,_) | SubImm(d,_,_) | FAdd(d,_,_) | FSub(d,_,_) | FMul(d,_,_)
            | FDiv(d,_,_) | FNeg(d,_) | FLt(d,_,_) | FEq(d,_,_)
            | Ld(d,_,_) | LdIdx(d,_,_) | Copy(d,_) | Move(d,_) | Call(d,_) | CallReg(d,_) => Some(*d),
            _ => None,
        }
    }

    for name in ["nqueens", "mandelbrot", "ackermann", "merge_sort", "coin_change"] {
        let path = format!("{}/../../examples/{}.abe", env!("CARGO_MANIFEST_DIR"), name);
        let Ok(src) = std::fs::read_to_string(&path) else { continue };
        let mut p = Parser::new(Lexer::new(&src)).with_source(src.clone());
        let ast = p.parse_program();
        let mut c = Compiler::new().with_source(src);
        let Ok(module) = c.compile_module(&ast) else { continue };
        let (mut coalesce, mut copies, mut total) = (0usize, 0usize, 0usize);
        for ch in &module.functions {
            if let Chunk::Bytecode(bc) = ch {
                let lo = live_out(&bc.code);
                for (pc, op) in bc.code.iter().enumerate() {
                    total += 1;
                    if let OpCode::Copy(_, s) = op {
                        copies += 1;
                        if pc > 0
                            && dest_of(&bc.code[pc - 1]) == Some(*s)
                            && (lo[pc] & (1u128 << s.0)) == 0
                        {
                            coalesce += 1;
                        }
                    }
                }
            }
        }
        eprintln!("{:<12} coalescable: {:>4} / {:>4} copies / {:>5} static ops ({:.1}% of ops)",
            name, coalesce, copies, total, coalesce as f64 * 100.0 / total.max(1) as f64);
    }
}

// ── copy coalescing ──────────────────────────────────────────────────────────

use abrase::compiler::dataflow::coalesce_copies;

#[test]
fn coalesce_basic_pair() {
    // PushConst r7; Copy r1<-r7; Ret r1  →  PushConst r1; Ret r1
    let mut code = vec![PushConst(R(7), 0), Copy(R(1), R(7)), Ret(R(1))];
    assert_eq!(coalesce_copies(&mut code, 128), 1);
    assert_eq!(code, vec![PushConst(R(1), 0), Ret(R(1))]);
}

#[test]
fn coalesce_skips_when_source_still_read() {
    // r7 read again after the Copy → must NOT fuse.
    let mut code = vec![Add(R(7), R(1), R(2)), Copy(R(3), R(7)), Ret(R(7))];
    assert_eq!(coalesce_copies(&mut code, 128), 0);
}

#[test]
fn coalesce_skips_branch_targeted_copy() {
    // Jz targets the Copy: another path reaches it without the producer → no fuse.
    let code0 = vec![
        Jz(R(0), 1),          // target = 0+1+1 = 2 = the Copy
        Add(R(7), R(1), R(1)),
        Copy(R(2), R(7)),
        Ret(R(2)),
    ];
    let mut code = code0.clone();
    assert_eq!(coalesce_copies(&mut code, 128), 0);
    assert_eq!(code, code0, "untouched");
}

#[test]
fn coalesce_fixes_branch_offsets_across_deletion() {
    // Jz at 0 jumps over the fused pair to Ret r0 at 4.
    let mut code = vec![
        Jz(R(0), 3),          // target = 0+1+3 = 4
        PushConst(R(7), 0),
        Copy(R(1), R(7)),
        Ret(R(1)),
        Ret(R(0)),
    ];
    assert_eq!(coalesce_copies(&mut code, 128), 1);
    assert_eq!(code, vec![
        Jz(R(0), 2),          // target = 0+1+2 = 3 (Ret r0, shifted)
        PushConst(R(1), 0),
        Ret(R(1)),
        Ret(R(0)),
    ]);
}

#[test]
fn coalesce_chains_to_fixpoint() {
    let mut code = vec![
        PushConst(R(7), 0),
        Copy(R(6), R(7)),
        Copy(R(1), R(6)),
        Ret(R(1)),
    ];
    assert_eq!(coalesce_copies(&mut code, 128), 2);
    assert_eq!(code, vec![PushConst(R(1), 0), Ret(R(1))]);
}

#[test]
fn coalesce_never_redirects_raise_or_resume() {
    let code0 = vec![Raise(R(7), R(1), R(2)), Copy(R(3), R(7)), Ret(R(3))];
    let mut code = code0.clone();
    assert_eq!(coalesce_copies(&mut code, 128), 0);
    assert_eq!(code, code0);
}

#[test]
fn coalesce_never_fuses_into_arg_staging_slot() {
    // Copy dest >= reg_count is an arg-staging slot (callee window). Fusing a
    // Call producer there trips do_call's dest-window check — found by
    // fuzz_coalesce_equivalence. Must stay untouched.
    let code0 = vec![Call(R(3), 0), Copy(R(11), R(3)), Call(R(4), 1), Ret(R(4))];
    let mut code = code0.clone();
    assert_eq!(coalesce_copies(&mut code, 11), 0, "reg_count=11 → r11 is an arg slot");
    assert_eq!(code, code0);
    // With a window that covers r11, the same pair fuses fine.
    let mut code = code0.clone();
    assert_eq!(coalesce_copies(&mut code, 64), 1);
}

#[test]
#[ignore = "measurement probe: run with -- --ignored --nocapture"]
fn report_register_reuse_ceiling() {
    use abrase::bytecode::Chunk;
    use abrase::compiler::Compiler;
    use abrase::lexer::Lexer;
    use abrase::parser::Parser;

    for name in ["nqueens", "mandelbrot", "ackermann", "merge_sort", "coin_change", "stress_dispatch", "primes_gen"] {
        let path = format!("{}/../../examples/{}.abe", env!("CARGO_MANIFEST_DIR"), name);
        let Ok(src) = std::fs::read_to_string(&path) else { continue };
        let mut p = Parser::new(Lexer::new(&src)).with_source(src.clone());
        let ast = p.parse_program();
        let mut c = Compiler::new().with_source(src);
        let Ok(module) = c.compile_module(&ast) else { continue };
        println!("== {}", name);
        for (fid, ch) in module.functions.iter().enumerate() {
            if let Chunk::Bytecode(bc) = ch {
                if bc.code.is_empty() { continue; }
                let lo = live_out(&bc.code);
                let max_live = lo.iter().map(|w| w.count_ones() as usize).max().unwrap_or(0);
                let max_live = max_live.max(bc.param_count);
                if bc.reg_count > 2 {
                    println!("  fn#{:<3} reg_count={:<3} max_live={:<3} slack={}",
                        fid, bc.reg_count, max_live, bc.reg_count.saturating_sub(max_live));
                }
            }
        }
    }
}
