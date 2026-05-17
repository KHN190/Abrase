#[path = "compiler_codegen_common.rs"]
mod compiler_codegen_common;

use compiler_codegen_common::*;
use myriad::Value;

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
    assert_eq!(run_source(src), Ok(Value::from_int(42)));
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
    assert_eq!(run_source(src), Ok(Value::from_int(42)));
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
    assert_eq!(run_source(src), Ok(Value::from_int(8)));
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
    assert_eq!(run_source(src), Ok(Value::from_int(50)));
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
    assert_eq!(run_source(src), Ok(Value::from_int(106)));
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
    assert_eq!(run_source(src), Ok(Value::from_int(5)));
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
    assert_eq!(run_source(src), Ok(Value::from_int(14)));
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
    assert_eq!(run_source(src), Ok(Value::from_int(42)));
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
    assert_eq!(run_source(src), Ok(Value::from_int(105)));
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
    assert_eq!(run_source(src), Ok(Value::from_int(70)));
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
    assert_eq!(run_source(src), Ok(Value::from_int(110)));
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
    assert_eq!(run_source(src), Ok(Value::from_int(14)));
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
