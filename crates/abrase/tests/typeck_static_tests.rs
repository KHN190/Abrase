use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;
use abrase::typeck::Checker;

fn compiles(src: &str) -> bool {
    let mut p = Parser::new(Lexer::new(src)).with_source(src.into());
    let ast = p.parse_program();
    if !p.errors.is_empty() { return false; }
    let mut c = Compiler::new().with_source(src.into());
    c.compile_module(&ast).is_ok()
}

fn errors(src: &str) -> Vec<String> {
    let mut parser = Parser::new(Lexer::new(src)).with_source(src.into());
    let ast = parser.parse_program();
    assert!(parser.errors.is_empty(), "parse errors: {:?}", parser.errors);
    let mut checker = Checker::new();
    checker.check_program(&ast);
    checker.errors.into_iter().map(|e| e.message).collect()
}

fn assert_clean(src: &str) {
    let e = errors(src);
    assert!(e.is_empty(), "expected no type errors, got: {:?}", e);
}

#[test]
fn static_and_const_declarations_type_check() {
    assert_clean(
        "const MAX: Int = 100;\n\
         static GREETING: Int = 7;\n\
         fn main() -> Unit { () }\n",
    );
}

#[test]
fn immutable_static_rejects_assignment() {
    let e = errors(
        "static FRAME: Int = 0;\n\
         fn main() -> Unit { () }\n\
         fn tick() -> Unit { FRAME = 1 }\n",
    );
    assert!(
        e.iter().any(|m| m.to_lowercase().contains("immutable")),
        "expected immutable-assignment error, got: {:?}", e
    );
}

#[test]
fn static_initializer_type_mismatch_errors() {
    let e = errors(
        "static FRAME: Int = true;\n\
         fn main() -> Unit { () }\n",
    );
    assert!(!e.is_empty(), "expected a type-mismatch error for `Int = true`");
}

#[test]
fn static_initialized_by_fn_call_is_accepted() {
    assert_clean(
        "fn build() -> Array<Int> { [1, 2, 3] }\n\
         static BH: Array<Int> = build();\n\
         fn main() -> Int { BH[0] }\n",
    );
}

#[test]
fn const_initialized_by_fn_call_is_rejected() {
    let src = "fn seed() -> Int { 7 }\n\
               const C: Int = seed();\n\
               fn main() -> Int { C }\n";
    let mut p = Parser::new(Lexer::new(src)).with_source(src.into());
    let ast = p.parse_program();
    assert!(p.errors.is_empty(), "parse errors: {:?}", p.errors);
    let mut c = Compiler::new().with_source(src.into());
    let r = c.compile_module(&ast);
    assert!(r.is_err(), "const = fn call must fail to compile");
    assert!(
        c.errors.iter().any(|e| e.message.contains("compile-time constant")),
        "expected compile-time-constant error, got: {:?}",
        c.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

#[test]
fn cart_main_compiles_with_frame_loop() {
    let src = r#"
@cart fn main() -> <frame> Unit {
  let mut x = 0;
  loop { x = x + 1; let _ = frame.present() }
}
"#;
    assert!(compiles(src), "@cart main with frame loop must compile");
}

#[test]
fn cart_main_persistent_state_compiles() {
    let src = r#"
@cart fn main() -> <frame> Unit {
  let mut count = 0;
  loop {
    count = count + 1;
    let _ = frame.present()
  }
}
"#;
    assert!(compiles(src), "@cart main with mutable state across yields must compile");
}

#[test]
fn cart_main_allows_frame_effect() {
    let e = errors(
        "@cart fn main() -> <frame> Unit { () }\n",
    );
    assert!(e.is_empty(), "@cart main with frame effect must not error: {:?}", e);
}

#[test]
fn non_cart_main_rejects_frame_effect() {
    let e = errors(
        "fn main() -> <frame> Unit { () }\n",
    );
    assert!(!e.is_empty(), "non-@cart main with frame effect must error");
}

#[test]
fn cart_main_rejects_non_frame_effect() {
    let e = errors(
        "effect E { op tick() -> Unit }\n\
         @cart fn main() -> <E> Unit { () }\n",
    );
    assert!(!e.is_empty(), "@cart main with non-frame effect must still error");
}

#[test]
fn redeclaring_same_static_name_errors() {
    let e = errors(
        "static N: Int = 0;\n\
         static N: Int = 1;\n\
         fn main() -> Unit { () }\n",
    );
    assert!(!e.is_empty(), "expected a redeclaration error for duplicate static `N`");
}
