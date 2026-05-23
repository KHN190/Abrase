#[path = "compiler_codegen_common.rs"]
mod compiler_codegen_common;

use compiler_codegen_common::*;
use myriad::Value;

fn expect_int_clean(src: &str, expected: i64) {
    let ast = parse_source(src);
    let (v, live) = compile_module_and_run_with_heap(&ast).expect("run ok");
    assert_eq!(v, Value::from_int(expected));
    assert_eq!(live, 0, "heap leak: live={}", live);
}

#[test]
fn handle_with_only_return_arm_passes_body_through() {
    let src = r#"
        fn body() -> Int { 42 }
        fn main() -> Int {
            handle body() {
                return v => v
            }
        }
    "#;
    expect_int_clean(src, 42);
}

#[test]
fn return_arm_transforms_body_value() {
    let src = r#"
        fn body() -> Int { 41 }
        fn main() -> Int {
            handle body() {
                return v => v + 1
            }
        }
    "#;
    expect_int_clean(src, 42);
}

#[test]
fn effect_op_call_reroutes_to_arm() {
    let src = r#"
        effect provider { op give() -> Int }
        fn produce() -> <provider> Int { provider.give() + 1 }
        fn main() -> Int {
            handle produce() {
                return v => v,
                provider.give => resume(7)
            }
        }
    "#;
    expect_int_clean(src, 8);
}

#[test]
fn arm_body_resumes_with_constant() {
    // typeck rejects bare-value arm bodies (must-resume rule); make the
    // resume explicit. Semantically identical under the current lowering.
    let src = r#"
        effect provider { op give() -> Int }
        fn produce() -> <provider> Int { provider.give() * 10 }
        fn main() -> Int {
            handle produce() {
                return v => v,
                provider.give => resume(5)
            }
        }
    "#;
    expect_int_clean(src, 50);
}

#[test]
fn effect_op_with_param_is_visible_to_arm() {
    let src = r#"
        effect t { op at(n: Int) -> Int }
        fn produce() -> <t> Int { t.at(5) + 100 }
        fn main() -> Int {
            handle produce() {
                return v => v,
                t.at n => resume(n + 1)
            }
        }
    "#;
    expect_int_clean(src, 106);
}

#[test]
fn multiple_op_calls_each_dispatch_to_arm() {
    let src = r#"
        effect t { op at(n: Int) -> Int }
        fn produce() -> <t> Int { t.at(2) + t.at(3) }
        fn main() -> Int {
            handle produce() {
                return v => v,
                t.at n => resume(n)
            }
        }
    "#;
    expect_int_clean(src, 5);
}

#[test]
fn arm_can_call_top_level_function() {
    let src = r#"
        effect t { op at(n: Int) -> Int }
        fn double(x: Int) -> Int { x + x }
        fn produce() -> <t> Int { t.at(7) }
        fn main() -> Int {
            handle produce() {
                return v => v,
                t.at n => resume(double(n))
            }
        }
    "#;
    expect_int_clean(src, 14);
}

#[test]
fn two_handlers_for_different_effects_in_one_module() {
    let src = r#"
        effect a { op one() -> Int }
        effect b { op two() -> Int }
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
    expect_int_clean(src, 42);
}

#[test]
fn return_arm_body_captures_outer_let_binding() {
    let src = r#"
        effect e { op op() -> Int }
        fn produce() -> <e> Int { e.op() }
        fn main() -> Int {
            let bonus = 100;
            handle produce() {
                e.op => resume(5),
                return v => v + bonus
            }
        }
    "#;
    expect_int_clean(src, 105);
}

#[test]
fn op_arm_body_captures_outer_let_binding() {
    let src = r#"
        effect e { op op(n: Int) -> Int }
        fn main() -> Int {
            let mult = 10;
            handle e.op(7) {
                e.op n => resume(n * mult),
                return v => v
            }
        }
    "#;
    expect_int_clean(src, 70);
}

#[test]
fn nested_handlers_same_effect_use_inner_arm() {
    let src = r#"
        effect e { op op() -> Int }
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
    expect_int_clean(src, 110);
}

#[test]
fn handle_compiles_when_body_is_pure() {
    let src = r#"
        fn main() -> Int {
            handle (3 + 4) {
                return v => v * 2
            }
        }
    "#;
    expect_int_clean(src, 14);
}

#[test]
fn lifted_arms_appear_in_function_table() {
    let src = r#"
        effect t { op at() -> Int }
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
    assert!(
        module.functions.len() >= 4,
        "expected ≥4 fns after lifting arms; got {}",
        module.functions.len()
    );
}

#[test]
fn handler_pop_frees_cont_cell_after_resume() {
    let src = r#"
        effect e { op ask() -> Int }
        fn produce() -> <e> Int { e.ask() + 1 }
        fn main() -> Int {
            handle produce() {
                return v => v,
                e.ask => resume(7)
            }
        }
    "#;
    let ast = parse_source(src);
    let (v, live) = compile_module_and_run_with_heap(&ast).expect("run ok");
    assert_eq!(v, Value::from_int(8));
    assert_eq!(live, 0,
        "handler pop must rc_dec each cont cell and cascade-free its snapshot/env; got live={}",
        live);
}

#[test]
#[ignore = "arm body that throws needs exn handler arm; pending proper throw-arm wiring"]
fn handler_pop_frees_cont_cell_when_arm_throws() {
    let src = r#"
        effect e { op ask() -> Int }
        fn produce() -> <e> Int { e.ask() + 1 }
        fn main() -> Int {
            handle produce() {
                return v => v,
                e.ask => throw 42
            }
        }
    "#;
    let ast = parse_source(src);
    let (v, live) = compile_module_and_run_with_heap(&ast).expect("run ok");
    assert_eq!(v, Value::from_int(42));
    assert_eq!(live, 0,
        "arm-throw path must still free cont cell + snapshot at handler pop; got live={}",
        live);
}

#[test]
#[ignore = "multi-suspension cont cell cleanup pending (WIP)"]
fn handler_pop_frees_cont_cells_across_multiple_suspensions() {
    let src = r#"
        effect scale { op apply(x: Int) -> Int }
        fn transform(a: Int, b: Int) -> <scale> Int {
            let x = scale.apply(a);
            let y = scale.apply(b);
            x + y
        }
        fn main() -> Int {
            handle transform(3, 7) {
                scale.apply x => resume(x * 2),
                return v      => v
            }
        }
    "#;
    let ast = parse_source(src);
    let (v, live) = compile_module_and_run_with_heap(&ast).expect("run ok");
    assert_eq!(v, Value::from_int(20));
    assert_eq!(live, 0,
        "two cont cells allocated across two suspensions must both be freed; got live={}",
        live);
}

#[test]
fn handler_arm_mut_capture_counts_effects() {
    let src = r#"
        effect E { op tick() -> Unit }
        fn body() -> <E> Unit { E.tick(); E.tick(); E.tick() }
        fn main() -> Int {
            let mut total = 0;
            handle body() {
                return _ => total,
                E.tick   => { total = total + 1; resume(()) }
            }
        }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_int(3)));
}

#[test]
fn handler_arm_mut_capture_accumulates_payload() {
    let src = r#"
        effect E { op emit(n: Int) -> Unit }
        fn body() -> <E> Unit { E.emit(10); E.emit(20); E.emit(30) }
        fn main() -> Int {
            let mut sum = 0;
            handle body() {
                return _   => sum,
                E.emit v   => { sum = sum + v; resume(()) }
            }
        }
    "#;
    assert_eq!(run_source(src), Ok(Value::from_int(60)));
}

#[test]
fn handler_arm_mut_capture_shared_with_return_arm() {
    // total is mutated by op arm; return arm reads its final value.
    // Both arms must see the same cell.
    let src = r#"
        effect E { op bump() -> Unit }
        fn body() -> <E> Unit { E.bump(); E.bump() }
        fn main() -> Int {
            let mut total = 100;
            handle body() {
                return _ => total + 1,
                E.bump   => { total = total + 10; resume(()) }
            }
        }
    "#;
    // 100 + 10 + 10 = 120, +1 = 121
    assert_eq!(run_source(src), Ok(Value::from_int(121)));
}
