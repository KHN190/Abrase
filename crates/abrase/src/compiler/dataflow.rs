// Bytecode-level register liveness (the "road" for liveness-driven opts:
// move-on-last-use, drop-at-last-use). Pure analysis — no optimization here.
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

/// Is the read of `r` at `code[pc]` its last use (r dead after pc)?
pub fn is_last_use(live_out: &[RegSet], pc: usize, r: Register) -> bool {
    (live_out[pc] & bit(r)) == 0
}
