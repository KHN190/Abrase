use abrase::lexer::Lexer;
use abrase::parser::Parser;
use abrase::typeck::Checker;

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
fn redeclaring_same_static_name_errors() {
    let e = errors(
        "static N: Int = 0;\n\
         static N: Int = 1;\n\
         fn main() -> Unit { () }\n",
    );
    assert!(!e.is_empty(), "expected a redeclaration error for duplicate static `N`");
}
