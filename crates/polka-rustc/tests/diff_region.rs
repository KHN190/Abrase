mod common;
use common::*;
use polka::{Module, OpCode};
use myriad::Value;

const PUSH: i64 = ((0xE1u16 as i64) << 8) | 0x00;
const POP: i64 = ((0xE1u16 as i64) << 8) | 0x01;
const FORGET: i64 = ((0xE1u16 as i64) << 8) | 0x02;

fn region_module(code: Vec<OpCode>, constants: Vec<u64>, reg_count: usize) -> Module {
    Module { functions: vec![fn_chunk(code, constants, reg_count, 0)], entry: 0, flags: 0, exports: vec![] }
}

#[test]
fn region_push_alloc_pop_frees_all() {
    let m = region_module(
        vec![
            OpCode::PushConst(r(3), 0),
            OpCode::Deo(r(3), r(3)),
            OpCode::Alloc(r(0), 1),
            OpCode::Alloc(r(1), 1),
            OpCode::PushConst(r(3), 1),
            OpCode::Deo(r(3), r(3)),
            OpCode::PushConst(r(2), 2),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(PUSH).raw(), Value::from_int(POP).raw(), Value::from_int(0).raw()], 4,
    );
    let i = interp_with_live(&m);
    batch_compare(vec![m], vec![i], true);
}

#[test]
fn region_forget_excludes_from_pop() {
    let m = region_module(
        vec![
            OpCode::PushConst(r(3), 0),
            OpCode::Deo(r(3), r(3)),
            OpCode::Alloc(r(0), 1),
            OpCode::PushConst(r(3), 2),
            OpCode::Deo(r(0), r(3)),
            OpCode::PushConst(r(3), 1),
            OpCode::Deo(r(3), r(3)),
            OpCode::Drop(r(0)),
            OpCode::PushConst(r(2), 3),
            OpCode::Ret(r(2)),
        ],
        vec![
            Value::from_int(PUSH).raw(), Value::from_int(POP).raw(),
            Value::from_int(FORGET).raw(), Value::from_int(0).raw(),
        ], 4,
    );
    let i = interp_with_live(&m);
    batch_compare(vec![m], vec![i], true);
}

#[test]
fn region_nested_push_pop() {
    let m = region_module(
        vec![
            OpCode::PushConst(r(3), 0),
            OpCode::Deo(r(3), r(3)),
            OpCode::Alloc(r(0), 1),
            OpCode::PushConst(r(3), 0),
            OpCode::Deo(r(3), r(3)),
            OpCode::Alloc(r(1), 1),
            OpCode::PushConst(r(3), 1),
            OpCode::Deo(r(3), r(3)),
            OpCode::PushConst(r(3), 1),
            OpCode::Deo(r(3), r(3)),
            OpCode::PushConst(r(2), 2),
            OpCode::Ret(r(2)),
        ],
        vec![Value::from_int(PUSH).raw(), Value::from_int(POP).raw(), Value::from_int(0).raw()], 4,
    );
    let i = interp_with_live(&m);
    batch_compare(vec![m], vec![i], true);
}
