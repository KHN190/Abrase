use abrase::ast::*;
use abrase::lexer::Lexer;
use abrase::parser::Parser;

fn parse_errs(input: &str) -> Vec<String> {
    let mut p = Parser::new(Lexer::new(input));
    let _ = p.parse_program();
    p.errors.into_iter().map(|e| e.message).collect()
}

fn parse_program_no_errors(input: &str) -> Vec<Decl> {
    let mut p = Parser::new(Lexer::new(input));
    let decls = p.parse_program();
    assert!(p.errors.is_empty(), "unexpected parser errors: {:?}", p.errors);
    decls
}

#[test]
fn test_program_effect_with_op_no_stray_token() {
    let input = "effect Gen { op yield(v: Int) -> Unit }";
    let mut p = Parser::new(Lexer::new(input));
    let decls = p.parse_program();
    assert!(p.errors.is_empty(), "unexpected parser errors: {:?}", p.errors);
    assert_eq!(decls.len(), 1);
    if let Decl::Effect { name, ops, .. } = &decls[0] {
        assert_eq!(name, "Gen");
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].name, "yield");
    } else {
        panic!("Expected Effect declaration, got {:?}", decls[0]);
    }
}

#[test]
fn test_program_effect_followed_by_fn() {
    let input = "effect Logger { op log(msg: Int) -> Unit } fn main() -> Int { 0 }";
    let mut p = Parser::new(Lexer::new(input));
    let decls = p.parse_program();
    assert!(p.errors.is_empty(), "unexpected parser errors: {:?}", p.errors);
    assert_eq!(decls.len(), 2);
    assert!(matches!(decls[0], Decl::Effect { .. }));
    assert!(matches!(decls[1], Decl::Fn(_)));
}

#[test]
fn test_program_empty_effect_followed_by_fn() {
    let input = "effect Marker { } fn main() -> Int { 1 }";
    let mut p = Parser::new(Lexer::new(input));
    let decls = p.parse_program();
    assert!(p.errors.is_empty(), "unexpected parser errors: {:?}", p.errors);
    assert_eq!(decls.len(), 2);
}

#[test]
fn test_program_effect_multiple_ops() {
    let input = "effect IO { op read() -> Int op write(n: Int) -> Unit }";
    let mut p = Parser::new(Lexer::new(input));
    let decls = p.parse_program();
    assert!(p.errors.is_empty(), "unexpected parser errors: {:?}", p.errors);
    if let Decl::Effect { ops, .. } = &decls[0] {
        assert_eq!(ops.len(), 2);
        assert_eq!(ops[0].name, "read");
        assert_eq!(ops[1].name, "write");
    } else {
        panic!("Expected Effect declaration");
    }
}

#[test]
fn test_multiple_fn_decls_with_if_else_each() {
    let input = "
        fn is_even(n: Int) -> Int {
            if n == 0 { 1 } else { is_odd(n - 1) }
        }

        fn is_odd(n: Int) -> Int {
            if n == 0 { 0 } else { is_even(n - 1) }
        }

        fn main() -> Int { is_even(6) }
    ";
    let mut p = Parser::new(Lexer::new(input));
    let decls = p.parse_program();
    let err_msgs: Vec<_> = p.errors.iter().map(|e| e.message.clone()).collect();
    assert!(err_msgs.is_empty(), "parse errors: {:?}", err_msgs);
    let fn_names: Vec<_> = decls.iter().filter_map(|d| match d {
        Decl::Fn(f) => Some(f.name.clone()),
        _ => None,
    }).collect();
    assert_eq!(fn_names, vec!["is_even".to_string(), "is_odd".to_string(), "main".to_string()]);
}

#[test]
fn test_error_orphan_else_at_block_start() {
    let errs = parse_errs("fn f() -> Int { else { 1 } }");
    assert!(!errs.is_empty(), "expected error for orphan else, got none");
}

#[test]
fn test_error_missing_arrow_in_match_arm() {
    let errs = parse_errs("fn f() -> Int { match x { 1 1 } }");
    assert!(!errs.is_empty(), "expected error for missing '=>'");
}

#[test]
fn test_error_unclosed_match_brace() {
    let errs = parse_errs("fn f() -> Int { match x { 1 => 1 ");
    assert!(!errs.is_empty(), "expected error for unclosed match");
}

#[test]
fn test_error_let_missing_assign() {
    let errs = parse_errs("fn f() -> Int { let x 5; x }");
    assert!(!errs.is_empty(), "expected error for missing '=' in let");
}

#[test]
fn test_error_duplicate_else_after_if() {
    let errs = parse_errs("fn f() -> Int { if x { 1 } else { 2 } else { 3 } }");
    assert!(!errs.is_empty(), "expected error for duplicate else");
}

#[test]
fn test_error_stray_keyword_after_if_consequence() {
    let errs = parse_errs("fn f() -> Int { if x { 1 } banana { 2 } }");
    assert!(!errs.is_empty(), "expected error for stray ident between if and following block");
}

#[test]
fn test_error_unclosed_fn_body() {
    let errs = parse_errs("fn f() -> Int { 1");
    assert!(!errs.is_empty(), "expected error for unclosed fn body");
}

#[test]
fn test_error_two_exprs_no_separator() {
    let errs = parse_errs("fn f() -> Int { 1 2 }");
    assert!(!errs.is_empty(), "expected error for two stmts without separator");
}

#[test]
fn test_error_extra_else_in_chain() {
    let errs = parse_errs(
        "fn f() -> Int { if a { 1 } else if b { 2 } else { 3 } else { 4 } }"
    );
    assert!(!errs.is_empty(), "expected error for extra else after terminal else");
}

#[test]
fn test_error_match_garbage_between_arms() {
    let errs = parse_errs("fn f() -> Int { match x { 1 => 1 banana 2 => 2 } }");
    assert!(!errs.is_empty(), "expected error for garbage between match arms");
}

#[test]
fn test_error_extra_token_after_complete_decl() {
    let errs = parse_errs("fn f() -> Int { 1 }  banana  fn g() -> Int { 2 }");
    assert!(!errs.is_empty(), "expected error for stray top-level token");
}

#[test]
fn test_program_mod_keyword_is_rejected() {
    let src = "mod foo fn main() -> Int { 0 }";
    let mut p = abrase::parser::Parser::new(abrase::lexer::Lexer::new(src)).with_source(src.into());
    let _ = p.parse_program();
    assert!(!p.errors.is_empty(),
        "`mod` is removed — file-path-based modules only; expected parse error");
}

#[test]
fn test_program_trait_then_fn() {
    let decls = parse_program_no_errors("trait Foo { } fn main() -> Int { 0 }");
    assert_eq!(decls.len(), 2);
    assert!(matches!(decls[0], Decl::Trait { .. }));
    assert!(matches!(decls[1], Decl::Fn(_)));
}

#[test]
fn test_program_impl_then_fn() {
    let decls = parse_program_no_errors("impl Foo { } fn main() -> Int { 0 }");
    assert_eq!(decls.len(), 2);
    assert!(matches!(decls[0], Decl::Impl { .. }));
    assert!(matches!(decls[1], Decl::Fn(_)));
}

#[test]
fn test_program_type_alias_then_fn() {
    let decls = parse_program_no_errors("type alias Pair = (Int, Int) fn main() -> Int { 0 }");
    assert_eq!(decls.len(), 2);
    assert!(matches!(decls[0], Decl::TypeAlias { .. }));
    assert!(matches!(decls[1], Decl::Fn(_)));
}

#[test]
fn test_program_type_alias_with_semicolon_then_fn() {
    let decls = parse_program_no_errors("type alias Pair = (Int, Int); fn main() -> Int { 0 }");
    assert_eq!(decls.len(), 2);
}

#[test]
fn test_program_effect_alias_then_fn() {
    let decls = parse_program_no_errors("effect alias E = <exn> fn main() -> Int { 0 }");
    assert_eq!(decls.len(), 2);
    assert!(matches!(decls[0], Decl::EffectAlias { .. }));
    assert!(matches!(decls[1], Decl::Fn(_)));
}

#[test]
fn test_program_const_then_fn() {
    let decls = parse_program_no_errors("const X: Int = 42 fn main() -> Int { X }");
    assert_eq!(decls.len(), 2);
    assert!(matches!(decls[0], Decl::Const { .. }));
}

#[test]
fn test_program_const_with_semicolon_then_fn() {
    let decls = parse_program_no_errors("const X: Int = 42; fn main() -> Int { X }");
    assert_eq!(decls.len(), 2);
}

#[test]
fn test_program_const_block_value_then_fn() {
    let decls = parse_program_no_errors(
        "const X: Int = if true { 1 } else { 0 } fn main() -> Int { X }",
    );
    assert_eq!(decls.len(), 2);
}

#[test]
fn test_program_import_then_fn() {
    let decls = parse_program_no_errors("use std::io::{Read, Write}; fn main() -> Int { 0 }");
    assert_eq!(decls.len(), 2);
    assert!(matches!(decls[0], Decl::Use { .. }));
}

#[test]
fn test_program_import_dot_brace_syntax() {
    let decls = parse_program_no_errors("use io::{File, Read}; fn main() -> Int { 0 }");
    assert_eq!(decls.len(), 2);
    if let Decl::Use { path, items } = &decls[0] {
        assert_eq!(path, &vec!["io".to_string()]);
        assert_eq!(items.len(), 2);
    } else { panic!("expected Import"); }
}
