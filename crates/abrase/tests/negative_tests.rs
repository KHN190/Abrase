// Source-level "must reject" tests. 
use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;

fn must_reject(src: &str) -> String {
    let mut parser = Parser::new(Lexer::new(src)).with_source(src.into());
    let ast = parser.parse_program();
    if !parser.errors.is_empty() {
        return parser.pretty_print_errors();
    }
    let mut compiler = Compiler::new().with_source(src.into());
    match compiler.compile_module(&ast) {
        Ok(_) => panic!("expected compile error, got success.\nsource:\n{}", src),
        Err(_) => compiler.pretty_print_errors(),
    }
}

#[test]
fn variant_pattern_too_many_args_rejected() {
    // `Some` has 1 payload; matching with 3 args was silently typed Unknown.
    let src = r#"
        type Opt = Some(Int) | None
        fn main() -> Int {
            let v = Some(1);
            match v {
                Some(a, b, c) => a,
                None => 0,
            }
        }
    "#;
    let err = must_reject(src);
    assert!(err.contains("expects 1 arg") || err.contains("got 3"),
            "expected arg-count error, got: {}", err);
}

#[test]
fn variant_pattern_too_few_args_rejected() {
    let src = r#"
        type Pair = P(Int, Int)
        fn main() -> Int {
            let v = P(1, 2);
            match v {
                P(a) => a,
            }
        }
    "#;
    let err = must_reject(src);
    assert!(err.contains("expects 2 arg") || err.contains("got 1"),
            "expected arg-count error, got: {}", err);
}

#[test]
fn impl_for_reference_type_rejected() {
    // `impl Doubler for &Foo { ... }` previously silently dropped the entire impl.
    let src = r#"
        type Foo = Foo(Int)
        trait Doubler {
            fn double(self) -> Int { 0 }
        }
        impl Doubler for &Foo {
            fn double(self) -> Int { 42 }
        }
        fn main() -> Int { 0 }
    "#;
    let err = must_reject(src);
    assert!(err.contains("not supported") || err.contains("named/qualified"),
            "expected unsupported-target error, got: {}", err);
}

#[test]
fn int_literal_overflow_rejected() {
    // i64 max is 9223372036854775807. Add a digit → overflow.
    let src = r#"fn main() -> Int { 99999999999999999999 }"#;
    let err = must_reject(src);
    assert!(err.contains("out of range") || err.contains("overflow"),
            "expected overflow error, got: {}", err);
}

#[test]
fn unknown_string_escape_rejected() {
    let src = r#"fn main() -> String { "bad\qstuff" }"#;
    let err = must_reject(src);
    assert!(err.contains("unknown escape") || err.contains("\\q"),
            "expected unknown-escape error, got: {}", err);
}

#[test]
fn bad_unicode_escape_rejected() {
    let src = r#"fn main() -> String { "\u{ZZZZ}" }"#;
    let err = must_reject(src);
    assert!(err.contains("not valid hex") || err.contains("unicode"),
            "expected bad-hex error, got: {}", err);
}

#[test]
fn unclosed_string_literal_rejected() {
    // No closing `"` before EOF. Easy to silently produce a String token.
    let src = r#"fn main() -> Int { let s = "hello; 0 }"#;
    let err = must_reject(src);
    assert!(!err.is_empty(), "unclosed string must surface some error");
}

#[test]
fn unclosed_block_comment_rejected() {
    // `/* ... ` with no `*/` — lexer commonly falls off end of input silently.
    let src = "fn main() -> Int { /* unclosed comment\n  0\n}";
    let err = must_reject(src);
    assert!(!err.is_empty(), "unclosed /* */ must surface some error");
}

#[test]
fn reserved_keyword_as_var_name_rejected() {
    // `let fn = 5` — `fn` is a keyword, must not be silently accepted as ident.
    let src = "fn main() -> Int { let fn = 5; 0 }";
    let err = must_reject(src);
    assert!(!err.is_empty(), "reserved keyword as ident must error");
}

#[test]
fn char_literal_multiple_chars_rejected() {
    // `'ab'` is invalid — must not silently truncate to 'a' or 'b'.
    let src = "fn main() -> Char { 'ab' }";
    let err = must_reject(src);
    assert!(!err.is_empty(), "multi-char literal must surface an error");
}

#[test]
fn let_binding_type_mismatch_rejected() {
    // `let x: Int = "hi"` — annotation conflicts with RHS type.
    let src = r#"fn main() -> Int { let x: Int = "hi"; 0 }"#;
    let err = must_reject(src);
    assert!(err.to_lowercase().contains("type") || err.contains("mismatch"),
            "expected type error, got: {}", err);
}

#[test]
fn fn_return_type_mismatch_rejected() {
    // Declared `-> Int` but body returns a String.
    let src = r#"fn f() -> Int { "hi" } fn main() -> Int { f() }"#;
    let err = must_reject(src);
    assert!(err.to_lowercase().contains("type") || err.contains("mismatch"),
            "expected return-type error, got: {}", err);
}

#[test]
fn fn_call_wrong_arg_count_rejected() {
    // `fn f(a, b)` called with one arg.
    let src = "fn f(a: Int, b: Int) -> Int { a + b } fn main() -> Int { f(1) }";
    let err = must_reject(src);
    assert!(!err.is_empty(), "wrong arg count must error");
}

#[test]
fn calling_non_function_rejected() {
    // `let x = 1; x()` — Int isn't callable.
    let src = "fn main() -> Int { let x = 1; x() }";
    let err = must_reject(src);
    assert!(!err.is_empty(), "calling non-function must error");
}

#[test]
fn undefined_identifier_rejected() {
    let src = "fn main() -> Int { undefined_var }";
    let err = must_reject(src);
    assert!(err.to_lowercase().contains("undefined") || err.contains("not found"),
            "expected undefined-var error, got: {}", err);
}

#[test]
fn record_literal_missing_field_rejected() {
    // Pt requires both x and y; only x given.
    let src = r#"
        type Pt = { x: Int, y: Int }
        fn main() -> Int { let p = Pt { x: 1 }; p.x }
    "#;
    let err = must_reject(src);
    assert!(!err.is_empty(), "missing record field must error");
}

#[test]
fn record_literal_unknown_field_rejected() {
    let src = r#"
        type Pt = { x: Int, y: Int }
        fn main() -> Int { let p = Pt { x: 1, y: 2, z: 3 }; p.x }
    "#;
    let err = must_reject(src);
    assert!(!err.is_empty(), "unknown record field must error");
}

#[test]
fn field_access_on_non_record_rejected() {
    // `1.foo` — Int has no fields.
    let src = "fn main() -> Int { let x = 1; x.foo }";
    let err = must_reject(src);
    assert!(!err.is_empty(), "field access on non-record must error");
}

#[test]
fn assign_to_immutable_rejected() {
    // `let x = 1; x = 2;` — must demand `let mut`.
    let src = "fn main() -> Int { let x = 1; x = 2; x }";
    let err = must_reject(src);
    assert!(err.to_lowercase().contains("immutable") || err.contains("mut"),
            "expected immutable-binding error, got: {}", err);
}

#[test]
fn if_branch_type_mismatch_rejected() {
    // `if c { 1 } else { "s" }` — branches have incompatible types.
    let src = r#"fn main() -> Int { if true { 1 } else { "s" } }"#;
    let err = must_reject(src);
    assert!(!err.is_empty(), "if-branch type mismatch must error");
}

#[test]
fn resume_outside_handler_rejected() {
    // `resume(x)` is only valid in handler arms.
    let src = "fn main() -> Int { resume(0); 0 }";
    let err = must_reject(src);
    assert!(err.to_lowercase().contains("resume") || err.contains("handler"),
            "expected resume-context error, got: {}", err);
}

#[test]
fn match_wrong_variant_for_scrutinee_rejected() {
    // Matching an Option scrutinee with a Result variant pattern.
    let src = r#"
        type Opt = Some(Int) | None
        type Res = Ok(Int) | Bad(Int)
        fn main() -> Int {
            let v = Some(1);
            match v {
                Ok(a) => a,
                _ => 0,
            }
        }
    "#;
    let err = must_reject(src);
    assert!(!err.is_empty(), "wrong-variant pattern must error");
}

#[test]
fn too_many_function_args_rejected() {
    // >255 args must surface a codegen error, not silently truncate the slot byte.
    let params: String = (0..260).map(|i| format!("a{}: Int", i)).collect::<Vec<_>>().join(", ");
    let args: String = (0..260).map(|i| i.to_string()).collect::<Vec<_>>().join(", ");
    let src = format!("fn f({}) -> Int {{ 0 }} fn main() -> Int {{ f({}) }}", params, args);
    let err = must_reject(&src);
    assert!(err.contains("u8 range") || err.contains("Argument") || err.contains("255")
            || err.to_lowercase().contains("too many"),
            "expected arg-slot overflow error, got: {}", err);
}

#[test]
fn prefix_op_at_eof_does_not_panic() {
    // `!` / `-` / `&` / `*` alone at EOF previously had a path
    // that could hit `unreachable!()` if the inner match drifted out of sync.
    // Must surface a normal parse error, not panic.
    for src in ["fn main() -> Int { ! }", "fn main() -> Int { - }",
                "fn main() -> Int { & }", "fn main() -> Int { * }"] {
        let mut parser = Parser::new(Lexer::new(src)).with_source(src.into());
        let _ = parser.parse_program();
        assert!(!parser.errors.is_empty(), "expected parse error for {:?}", src);
    }
}

#[test]
fn let_pattern_destructure_length_mismatch() {
    // Destructuring a 3-element variant pattern with 2 binders, etc.
    let src = r#"
        type Triple = T(Int, Int, Int)
        fn main() -> Int {
            let t = T(1, 2, 3);
            match t {
                T(a, b) => a,
            }
        }
    "#;
    let err = must_reject(src);
    assert!(err.contains("expects") || err.contains("arg"),
            "expected arg-count error, got: {}", err);
}

// --- Typeck rejection: for / loop / break / continue / tuple / array-repeat / range ---

#[test]
fn for_over_non_iterable_rejected() {
    // `for x in 42 {}` — Int is not iterable.
    let src = "fn main() -> Int { for x in 42 {}; 0 }";
    let err = must_reject(src);
    assert!(err.contains("not iterable") || err.contains("iterate"),
            "expected 'not iterable' error, got: {}", err);
}

#[test]
fn loop_break_type_mismatch_rejected() {
    // `loop { break "wrong" }` used as `Int` — break value String ≠ declared return Int.
    let src = r#"fn main() -> Int { loop { break "wrong" } }"#;
    let err = must_reject(src);
    assert!(!err.is_empty(), "break with wrong-type value must error");
}

#[test]
fn break_outside_loop_rejected() {
    // `break` at top-level of a function — no enclosing loop.
    let src = "fn main() -> Int { break; 0 }";
    let err = must_reject(src);
    assert!(err.contains("Break outside") || err.contains("loop"),
            "expected 'Break outside of loop' error, got: {}", err);
}

#[test]
fn continue_outside_loop_rejected() {
    // `continue` at top-level of a function — no enclosing loop.
    let src = "fn main() -> Int { continue; 0 }";
    let err = must_reject(src);
    assert!(err.contains("Continue outside") || err.contains("loop"),
            "expected 'Continue outside of loop' error, got: {}", err);
}

#[test]
fn tuple_element_type_mismatch_rejected() {
    // Passing (Int, String) where (Int, Bool) is expected.
    let src = r#"
        fn first(t: (Int, Bool)) -> Int { 0 }
        fn main() -> Int { first((1, "not_bool")) }
    "#;
    let err = must_reject(src);
    assert!(!err.is_empty(), "tuple element type mismatch must error");
}

#[test]
fn array_repeat_non_int_count_rejected() {
    // `[0; true]` — repeat count must be Int, not Bool.
    let src = "fn main() -> Int { let _ = [0; true]; 0 }";
    let err = must_reject(src);
    assert!(err.contains("Array repeat count must be Int") || err.contains("count"),
            "expected count type error, got: {}", err);
}

#[test]
fn range_non_int_end_rejected() {
    // `0..true` — range end must be Int, not Bool.
    let src = "fn main() -> Int { let _ = 0..true; 0 }";
    let err = must_reject(src);
    assert!(err.contains("Range end must be Int") || err.contains("range") || err.contains("Int"),
            "expected range end type error, got: {}", err);
}
