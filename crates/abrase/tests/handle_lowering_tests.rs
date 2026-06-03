use abrase::ast::Decl;
use abrase::compiler::handlers::HandleLowering;
use abrase::lexer::Lexer;
use abrase::parser::Parser;

fn parse(source: &str) -> Vec<Decl> {
    let mut parser = Parser::new(Lexer::new(source)).with_source(source.to_string());
    let ast = parser.parse_program();
    assert!(parser.errors.is_empty(), "parse errors: {}", parser.pretty_print_errors());
    ast
}

const SIMPLE_EFFECT: &str = r#"
effect E { op f() -> Int }
fn work() -> <E> Int { E.f() }
fn main() -> Int {
  handle work() {
    E.f => { resume(10) }
    return v => v
  }
}
"#;

#[test]
fn test_effect_call_in_work_function() {
    let ast = parse(SIMPLE_EFFECT);
    let mut lowering = HandleLowering::new();
    lowering.lower(&ast);

    assert!(
        lowering.effect_op_to_arm.contains_key(&("E".to_string(), "f".to_string())),
        "E.f should be in effect_op_to_arm"
    );
    assert!(
        !lowering.synthetic_fns.is_empty(),
        "synthetic_fns should not be empty after lowering"
    );
}

#[test]
fn test_multishot_resume_count() {
    let source = r#"
effect E { op f() -> Int }
fn work() -> <E> Int { E.f() }
fn main() -> Int {
  handle work() {
    E.f => { let a = resume(10); let b = resume(20); a + b }
    return v => v
  }
}
"#;
    let ast = parse(source);
    let mut lowering = HandleLowering::new();
    lowering.lower(&ast);

    let handler_arm = lowering
        .synthetic_fns
        .iter()
        .find(|f| f.name.starts_with("__handle_op_E_f"))
        .expect("handler arm should exist");

    let resume_count = lowering.arm_resume_counts.get(&handler_arm.name).copied().unwrap_or(0);
    assert_eq!(resume_count, 2, "handler arm should have 2 resume calls, got {}", resume_count);
}

#[test]
fn test_single_shot_resume_count() {
    let ast = parse(SIMPLE_EFFECT);
    let mut lowering = HandleLowering::new();
    lowering.lower(&ast);

    let handler_arm = lowering
        .synthetic_fns
        .iter()
        .find(|f| f.name.starts_with("__handle_op_E_f"))
        .expect("handler arm should exist");

    let resume_count = lowering.arm_resume_counts.get(&handler_arm.name).copied().unwrap_or(0);
    assert_eq!(resume_count, 1, "handler arm should have 1 resume call, got {}", resume_count);
}

#[test]
fn test_effect_op_counts_populated() {
    let ast = parse(SIMPLE_EFFECT);
    let mut lowering = HandleLowering::new();
    lowering.lower(&ast);

    assert!(lowering.effect_op_counts.contains_key("E"), "E should be in effect_op_counts");
    let count = lowering.effect_op_counts.get("E").copied().unwrap_or(0);
    assert_eq!(count, 1, "E should have 1 op, got {}", count);
}

#[test]
fn test_work_function_call_chain() {
    let ast = parse(SIMPLE_EFFECT);
    let mut lowering = HandleLowering::new();
    lowering.lower(&ast);

    assert!(
        !lowering.synthetic_fns.iter().any(|f| f.name == "work"),
        "work() should NOT be in synthetic_fns (it's user-defined)"
    );
    assert!(
        lowering.synthetic_fns.iter().any(|f| f.name.starts_with("__handle_op_E_f")),
        "synthetic handler arm for E.f should exist"
    );
}
