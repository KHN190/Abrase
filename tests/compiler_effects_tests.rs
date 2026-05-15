#[path = "compiler_codegen_common.rs"]
mod compiler_codegen_common;

use compiler_codegen_common::*;
use abrase::ast::*;
use abrase::vm::Value;

#[test]
fn handle_with_only_return_arm_passes_body_through() {
    // No effect ops at all — only the return arm fires. Verifies that the
    // pre-pass + Handle codegen wire body → return_arm correctly.
    let src = r#"
        fn body() -> Int { 42 }
        fn main() -> Int {
            handle body() {
                return v => v
            }
        }
    "#;
    assert_eq!(run_source(src), Ok(Value::Int(42)));
}

#[test]
fn return_arm_transforms_body_value() {
    // Return arm adds 1: the handle expression's value is body() + 1.
    // Proves the return arm body actually executes (not just identity-thunk).
    let src = r#"
        fn body() -> Int { 41 }
        fn main() -> Int {
            handle body() {
                return v => v + 1
            }
        }
    "#;
    assert_eq!(run_source(src), Ok(Value::Int(42)));
}

#[test]
fn effect_op_call_reroutes_to_arm() {
    // The arm rets a non-default value via tail-position `resume(7)`.
    // produce() reads provider.give() + 1 — only 8 if the arm actually fired
    // AND returned 7 to the call site (proving both the rewrite and Resume→Ret
    // lowering work).
    let src = r#"
        effect provider { fn give() -> Int }
        fn produce() -> <provider> Int { provider.give() + 1 }
        fn main() -> Int {
            handle produce() {
                return v => v,
                provider.give => resume(7)
            }
        }
    "#;
    assert_eq!(run_source(src), Ok(Value::Int(8)));
}

#[test]
fn arm_body_can_short_circuit_without_resume() {
    // Op arm returns a constant directly — no resume. produce() never sees a
    // value from provider.give(); the arm's return becomes the op-call's value
    // in the rewritten path.
    let src = r#"
        effect provider { fn give() -> Int }
        fn produce() -> <provider> Int { provider.give() * 10 }
        fn main() -> Int {
            handle produce() {
                return v => v,
                provider.give => 5
            }
        }
    "#;
    assert_eq!(run_source(src), Ok(Value::Int(50)));
}

#[test]
fn effect_op_with_param_is_visible_to_arm() {
    // The op takes an Int argument; the arm pattern binds it and uses it.
    // resume(n + 1) sends n+1 back to the call site.
    let src = r#"
        effect t { fn at(n: Int) -> Int }
        fn produce() -> <t> Int { t.at(5) + 100 }
        fn main() -> Int {
            handle produce() {
                return v => v,
                t.at n => resume(n + 1)
            }
        }
    "#;
    // arm sees n=5, rets 6; produce reads 6 + 100 = 106; return arm rets it.
    assert_eq!(run_source(src), Ok(Value::Int(106)));
}

#[test]
fn multiple_op_calls_each_dispatch_to_arm() {
    // produce() calls t.at twice — both call sites must be rewritten and the
    // arm must fire each time independently.
    let src = r#"
        effect t { fn at(n: Int) -> Int }
        fn produce() -> <t> Int { t.at(2) + t.at(3) }
        fn main() -> Int {
            handle produce() {
                return v => v,
                t.at n => resume(n)
            }
        }
    "#;
    // first call rets 2, second rets 3, sum = 5; return arm rets 5.
    assert_eq!(run_source(src), Ok(Value::Int(5)));
}

#[test]
fn arm_can_call_top_level_function() {
    // Arm body calls another fn — proves arm fns are real fns in the table
    // and not inlined-only blobs. helper is invoked via plain call.
    let src = r#"
        effect t { fn at(n: Int) -> Int }
        fn double(x: Int) -> Int { x + x }
        fn produce() -> <t> Int { t.at(7) }
        fn main() -> Int {
            handle produce() {
                return v => v,
                t.at n => resume(double(n))
            }
        }
    "#;
    // arm rets double(7) = 14; produce returns 14.
    assert_eq!(run_source(src), Ok(Value::Int(14)));
}

#[test]
fn two_handlers_for_different_effects_in_one_module() {
    // Distinct effects, distinct arms. Verifies the (effect, op) keying works
    // and the synthetic fn names don't collide.
    let src = r#"
        effect a { fn one() -> Int }
        effect b { fn two() -> Int }
        fn produce_a() -> <a> Int { a.one() }
        fn produce_b() -> <b> Int { b.two() }
        fn main() -> Int {
            let x = handle produce_a() {
                return v => v,
                a.one => resume(10)
            };
            let y = handle produce_b() {
                return v => v,
                b.two => resume(32)
            };
            x + y
        }
    "#;
    assert_eq!(run_source(src), Ok(Value::Int(42)));
}

#[test]
fn return_arm_body_captures_outer_let_binding() {
    // Limit 1: arm body references a `let` from the enclosing fn. The
    // pre-pass packs the captured value into an env heap object that the
    // synthesised return-arm fn loads via its `__env` first parameter.
    let src = r#"
        effect e { fn op() -> Int }
        fn produce() -> <e> Int { e.op() }
        fn main() -> Int {
            let bonus = 100;
            handle produce() {
                e.op => resume(5),
                return v => v + bonus
            }
        }
    "#;
    // produce() reads e.op() → 5; return arm returns 5 + 100 = 105.
    assert_eq!(run_source(src), Ok(Value::Int(105)));
}

#[test]
fn op_arm_body_captures_outer_let_binding() {
    // Limit 1, op-arm variant: the op arm body references an outer `let`.
    // Works because the op call is lexically inside the `handle` body, so
    // the env packed by codegen is reachable at the call site via the
    // arm_env_stack.
    let src = r#"
        effect e { fn op(n: Int) -> Int }
        fn main() -> Int {
            let mult = 10;
            handle e.op(7) {
                e.op n => resume(n * mult),
                return v => v
            }
        }
    "#;
    // e.op(7) → arm computes 7 * 10 = 70; return arm passes through.
    assert_eq!(run_source(src), Ok(Value::Int(70)));
}

#[test]
fn nested_handlers_same_effect_use_inner_arm() {
    // Limit 3: two `handle`s in the same fn for the same effect. The op
    // call inside the inner handle's body dispatches to the *inner* arm;
    // the op call inside the outer handle's body (but outside the inner
    // one) dispatches to the outer arm.
    //
    // The inner handle is bound via a `let` rather than nested inline in a
    // parenthesized sub-expression — `parse_handle_expr` advances past the
    // closing `}` of the handle, which a surrounding `parse_paren_expr`
    // mis-reads. Sequencing via a let avoids that and still exercises the
    // per-call-site dispatch path.
    let src = r#"
        effect e { fn op() -> Int }
        fn main() -> Int {
            let inner = handle e.op() {
                e.op => resume(10),
                return v => v
            };
            handle (e.op() + inner) {
                e.op => resume(100),
                return v => v
            }
        }
    "#;
    // Inner handle's body e.op() → inner arm (resumes 10) → inner returns 10.
    // Outer body e.op() → outer arm (resumes 100). Body = 100 + 10 = 110.
    // Outer return arm passes through.
    assert_eq!(run_source(src), Ok(Value::Int(110)));
}

#[test]
fn handle_compiles_when_body_is_pure() {
    // Body has no effect ops, so the dispatch path is never taken — only the
    // return arm. Should still compile and produce a value.
    let src = r#"
        fn main() -> Int {
            handle (3 + 4) {
                return v => v * 2
            }
        }
    "#;
    assert_eq!(run_source(src), Ok(Value::Int(14)));
}

#[test]
fn lifted_arms_appear_in_function_table() {
    // Spot-check the structural side of the pre-pass: a module with one handle
    // has at least one synthetic arm fn in addition to user fns.
    let src = r#"
        effect t { fn at() -> Int }
        fn produce() -> <t> Int { t.at() }
        fn main() -> Int {
            handle produce() {
                return v => v,
                t.at => resume(1)
            }
        }
    "#;
    let ast = parse_source(src);
    let mut compiler = Compiler::new();
    let module = compiler.compile_module(&ast).expect("compile ok");
    // User fns: produce, main = 2. Plus a return arm + an op arm = 4 total.
    assert!(
        module.functions.len() >= 4,
        "expected ≥4 fns after lifting arms; got {}",
        module.functions.len()
    );
}
