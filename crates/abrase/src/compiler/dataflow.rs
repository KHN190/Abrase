// Bytecode-level register liveness (the "road" for liveness-driven opts:
// move-on-last-use, drop-at-last-use).
//
// Per-pc backward dataflow over the opcode list. live_out[pc] = registers that
// may be read on some path AFTER pc executes. A read of r at pc is its LAST use
// iff r ∉ live_out[pc]. Effect-ops/calls are handled conservatively (a register
// live across them stays live), so the analysis never under-reports liveness —
// which means any opt it enables is sound (never frees a still-live value).

use crate::bytecode::{OpCode, Register};

// Registers are 0..FRAME_REGS (128) → a u128 bitset.
type RegSet = u128;

#[inline]
fn bit(r: Register) -> RegSet { 1u128 << (r.0 as u32) }

fn op_dest(op: &OpCode) -> Option<Register> {
    use OpCode::*;
    match op {
        Add(d,_,_) | Sub(d,_,_) | Mul(d,_,_) | Div(d,_,_) | Mod(d,_,_) | Neg(d,_)
        | FAdd(d,_,_) | FSub(d,_,_) | FMul(d,_,_) | FDiv(d,_,_) | FNeg(d,_) | FLt(d,_,_) | FEq(d,_,_)
        | Eq(d,_,_) | Neq(d,_,_) | Lt(d,_,_) | Gt(d,_,_) | Lte(d,_,_) | Gte(d,_,_)
        | And(d,_,_) | Or(d,_,_) | Xor(d,_,_) | Shl(d,_,_) | Shr(d,_,_)
        | AddImm(d,_,_) | SubImm(d,_,_)
        | PushConst(d,_) | Copy(d,_) | Move(d,_)
        | Ld(d,_,_) | LdIdx(d,_,_) | Alloc(d,_)
        | Dei(d,_) | Call(d,_) | CallReg(d,_) | Resume(d,_) | Raise(d,_,_) => Some(*d),
        _ => None,
    }
}

fn op_uses(op: &OpCode, out: &mut Vec<Register>) {
    use OpCode::*;
    match op {
        Add(_,a,b) | Sub(_,a,b) | Mul(_,a,b) | Div(_,a,b) | Mod(_,a,b)
        | FAdd(_,a,b) | FSub(_,a,b) | FMul(_,a,b) | FDiv(_,a,b) | FLt(_,a,b) | FEq(_,a,b)
        | Eq(_,a,b) | Neq(_,a,b) | Lt(_,a,b) | Gt(_,a,b) | Lte(_,a,b) | Gte(_,a,b)
        | And(_,a,b) | Or(_,a,b) | Xor(_,a,b) | Shl(_,a,b) | Shr(_,a,b)
        | LdIdx(_,a,b) => { out.push(*a); out.push(*b); }
        StIdx(v,b,i) => { out.push(*v); out.push(*b); out.push(*i); }
        Raise(_,k,a) => { out.push(*k); out.push(*a); }
        Neg(_,a) | FNeg(_,a) | Copy(_,a) | Move(_,a) | AddImm(_,a,_) | SubImm(_,a,_)
        | Ld(_,a,_) => out.push(*a),
        St(v,b,_) => { out.push(*v); out.push(*b); }
        Dei(_,p) => out.push(*p),
        Deo(s,p) => { out.push(*s); out.push(*p); }
        Drop(s) | Ret(s) | Jz(s,_) | Jnz(s,_) | Handle(s,_) => out.push(*s),
        CallReg(_,f) => out.push(*f),
        Resume(_,v) => out.push(*v),
        _ => {}
    }
}

fn succs(op: &OpCode, i: usize, len: usize, out: &mut Vec<usize>) {
    use OpCode::*;
    let next = i + 1;
    // target == len means "jump off the end" = frame return (exit), no successor.
    let target = |off: i16| {
        let t = i as isize + 1 + off as isize;
        if t >= 0 && (t as usize) < len { Some(t as usize) } else { None }
    };
    match op {
        Ret(_) => {}
        Jmp(off) => { if let Some(t) = target(*off) { out.push(t); } }
        Jz(_, off) | Jnz(_, off) => {
            if let Some(t) = target(*off) { out.push(t); }
            if next < len { out.push(next); }
        }
        _ => { if next < len { out.push(next); } } // fall off end = frame return (exit)
    }
}

pub fn live_out(code: &[OpCode]) -> Vec<RegSet> {
    let n = code.len();
    let mut live_in = vec![0u128; n];
    let mut live_out = vec![0u128; n];
    let mut uses = vec![0u128; n];
    let mut defs: Vec<RegSet> = vec![0u128; n];
    let mut succ: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut buf: Vec<Register> = Vec::new();
    for i in 0..n {
        buf.clear();
        op_uses(&code[i], &mut buf);
        let mut u = 0u128;
        for r in &buf { u |= bit(*r); }
        uses[i] = u;
        defs[i] = op_dest(&code[i]).map(bit).unwrap_or(0);
        succs(&code[i], i, n, &mut succ[i]);
    }
    let mut changed = true;
    while changed {
        changed = false;
        for i in (0..n).rev() {
            let mut out = 0u128;
            for &s in &succ[i] { out |= live_in[s]; }
            let inn = uses[i] | (out & !defs[i]);
            if out != live_out[i] || inn != live_in[i] {
                live_out[i] = out;
                live_in[i] = inn;
                changed = true;
            }
        }
    }
    live_out
}

// Is the read of `r` at `code[pc]` its last use (r dead after pc)?
pub fn is_last_use(live_out: &[RegSet], pc: usize, r: Register) -> bool {
    (live_out[pc] & bit(r)) == 0
}

// ── copy coalescing ──────────────────────────────────────────────────────────
// Fuse `producer(dest=s); Copy(d, s)` into `producer(dest=d)` when s is dead
// after the Copy and the Copy is not a branch target (so the producer is its
// only predecessor). Deleting the Copy shifts pcs, so every Jmp/Jz/Jnz offset
// is recomputed against the old→new index map. Runs to fixpoint (chains fuse
// over successive rounds). Returns the number of fused copies.

fn set_dest(op: &mut OpCode, d: Register) -> bool {
    use OpCode::*;
    match op {
        Add(x,_,_) | Sub(x,_,_) | Mul(x,_,_) | Div(x,_,_) | Mod(x,_,_) | Neg(x,_)
        | FAdd(x,_,_) | FSub(x,_,_) | FMul(x,_,_) | FDiv(x,_,_) | FNeg(x,_) | FLt(x,_,_) | FEq(x,_,_)
        | Eq(x,_,_) | Neq(x,_,_) | Lt(x,_,_) | Gt(x,_,_) | Lte(x,_,_) | Gte(x,_,_)
        | And(x,_,_) | Or(x,_,_) | Xor(x,_,_) | Shl(x,_,_) | Shr(x,_,_)
        | AddImm(x,_,_) | SubImm(x,_,_)
        | PushConst(x,_) | Copy(x,_) | Move(x,_)
        | Ld(x,_,_) | LdIdx(x,_,_) | Alloc(x,_)
        | Call(x,_) | CallReg(x,_) => { *x = d; true }
        // Raise/Resume/Handle/Dei: dest is entangled with runtime machinery —
        // never redirect.
        _ => false,
    }
}

fn rewritable_dest(op: &OpCode) -> Option<Register> {
    use OpCode::*;
    match op {
        Raise(..) | Resume(..) | Handle(..) | Dei(..) => None,
        _ => op_dest(op),
    }
}

fn branch_targets(code: &[OpCode]) -> Vec<bool> {
    use OpCode::*;
    let mut t = vec![false; code.len()];
    for (i, op) in code.iter().enumerate() {
        let off = match op { Jmp(o) | Jz(_, o) | Jnz(_, o) => *o, _ => continue };
        let tgt = i as isize + 1 + off as isize;
        if tgt >= 0 && (tgt as usize) < code.len() { t[tgt as usize] = true; }
    }
    t
}

// `reg_count`: the caller's register-window size. Copies whose dest is an
// arg-staging slot (>= reg_count, written by finalize_arg_patches into the
// callee window) are never fused — that region is ABI-special, and a Call
// producer redirected there trips do_call's dest-window check.
pub fn coalesce_copies(code: &mut Vec<OpCode>, reg_count: usize) -> usize {
    use OpCode::*;
    let mut fused = 0usize;
    loop {
        let lo = live_out(code);
        let targets = branch_targets(code);
        // Find the first fusable (producer, Copy) pair.
        let mut found: Option<(usize, Register, Register)> = None;
        for i in 0..code.len().saturating_sub(1) {
            let Copy(d, s) = code[i + 1] else { continue };
            if d == s { continue; }
            if (d.0 as usize) >= reg_count { continue; } // arg-staging slot
            if targets[i + 1] { continue; }                 // another path enters at the Copy
            if rewritable_dest(&code[i]) != Some(s) { continue; }
            if (lo[i + 1] & bit(s)) != 0 { continue; }      // s still read after the Copy
            found = Some((i, d, s));
            break;
        }
        let Some((i, d, _s)) = found else { return fused };
        let k = i + 1; // index of the Copy to delete
        if !set_dest(&mut code[i], d) { return fused; }     // defensive; rewritable_dest agreed
        // Recompute branch offsets against the post-deletion indices.
        let map = |x: usize| if x > k { x - 1 } else { x };
        for j in 0..code.len() {
            if j == k { continue; }
            let off = match &code[j] { Jmp(o) | Jz(_, o) | Jnz(_, o) => *o, _ => continue };
            let old_t = (j as isize + 1 + off as isize) as usize;
            let new_off = (map(old_t) as isize) - (map(j) as isize + 1);
            let new_off = i16::try_from(new_off).expect("coalesce: offset shrank, must fit");
            match &mut code[j] {
                Jmp(o) | Jz(_, o) | Jnz(_, o) => *o = new_off,
                _ => unreachable!(),
            }
        }
        code.remove(k);
        fused += 1;
    }
}
