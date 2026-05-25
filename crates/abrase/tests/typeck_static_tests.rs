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

// `static` (immutable), `static mut`, and `const` all type-check.
#[test]
fn static_and_const_declarations_type_check() {
    assert_clean(
        "const MAX: Int = 100;\n\
         static GREETING: Int = 7;\n\
         static mut FRAME: Int = 0;\n\
         fn main() -> Unit { () }\n",
    );
}

// A static is readable from a function and carries its declared type.
#[test]
fn static_is_readable_with_its_type() {
    assert_clean(
        "static mut FRAME: Int = 0;\n\
         fn main() -> Unit { () }\n\
         fn current() -> Int { FRAME }\n",
    );
}

// `static mut` is module-level mutable state — assignment is allowed.
#[test]
fn static_mut_is_assignable() {
    assert_clean(
        "static mut FRAME: Int = 0;\n\
         fn main() -> Unit { () }\n\
         fn tick() -> Unit { FRAME = FRAME + 1 }\n",
    );
}

// Immutable `static` (no `mut`) rejects assignment — reuses the binding
// mutability check.
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

// Initializer type must match the declared type.
#[test]
fn static_initializer_type_mismatch_errors() {
    let e = errors(
        "static mut FRAME: Int = true;\n\
         fn main() -> Unit { () }\n",
    );
    assert!(!e.is_empty(), "expected a type-mismatch error for `Int = true`");
}

// Re-declaring a static with a name already taken is an error by design.
#[test]
fn redeclaring_same_static_name_errors() {
    let e = errors(
        "static mut N: Int = 0;\n\
         static mut N: Int = 1;\n\
         fn main() -> Unit { () }\n",
    );
    assert!(!e.is_empty(), "expected a redeclaration error for duplicate static `N`");
}
