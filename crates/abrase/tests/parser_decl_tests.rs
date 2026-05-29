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
fn test_decl_type() {
    let input = "type Point = { x: Int, y: Int }";
    let mut p = Parser::new(Lexer::new(input));
    let decl = p.parse_decl().unwrap();
    if let Decl::Type { name, .. } = decl {
        assert_eq!(name, "Point");
    } else {
        panic!("Expected Type declaration");
    }
}

#[test]
fn test_decl_trait() {
    let input = "trait Show { }";
    let mut p = Parser::new(Lexer::new(input));
    let decl = p.parse_decl().unwrap();
    if let Decl::Trait { name, .. } = decl {
        assert_eq!(name, "Show");
    } else {
        panic!("Expected Trait declaration");
    }
}

#[test]
fn test_decl_impl() {
    let input = "impl Int { }";
    let mut p = Parser::new(Lexer::new(input));
    let decl = p.parse_decl().unwrap();
    if let Decl::Impl { .. } = decl {
    } else {
        panic!("Expected Impl declaration");
    }
}

#[test]
fn test_decl_const() {
    let input = "const PI: Float = 3.14;";
    let mut p = Parser::new(Lexer::new(input));
    let decl = p.parse_decl().unwrap();
    if let Decl::Const { name, .. } = decl {
        assert_eq!(name, "PI");
    } else {
        panic!("Expected Const declaration");
    }
}

#[test]
fn test_decl_use() {
    let input = "use std::io::{Read, Write};";
    let mut p = Parser::new(Lexer::new(input));
    let decl = p.parse_decl().unwrap();
    if let Decl::Use { path, items } = decl {
        assert_eq!(path.len(), 2);
        assert_eq!(path[0], "std");
        assert_eq!(path[1], "io");
        assert_eq!(items.len(), 2);
    } else {
        panic!("Expected Use declaration");
    }
}

#[test]
fn test_decl_use_without_double_colon_before_brace_is_rejected() {
    let input = "use std::io {Read, Write};";
    let mut p = Parser::new(Lexer::new(input));
    let err = p.parse_decl().expect_err("expected '::'-required parse error");
    assert!(err.contains("Expected '::' before '{'"),
        "expected '::'-required error, got: {:?}", err);
}

#[test]
fn test_decl_type_alias() {
    let input = "type alias Result<T, E> = (T, E);";
    let mut p = Parser::new(Lexer::new(input));
    let decl = p.parse_decl().unwrap();
    if let Decl::TypeAlias { name, generics, .. } = decl {
        assert_eq!(name, "Result");
        assert_eq!(generics.len(), 2);
    } else {
        panic!("Expected TypeAlias declaration");
    }
}

#[test]
fn test_decl_effect_alias() {
    let input = "effect alias StdEffect = <io, exn>;";
    let mut p = Parser::new(Lexer::new(input));
    let decl = p.parse_decl().unwrap();
    if let Decl::EffectAlias { name, effects, .. } = decl {
        assert_eq!(name, "StdEffect");
        assert_eq!(effects.len(), 2);
    } else {
        panic!("Expected EffectAlias declaration");
    }
}

#[test]
fn test_decl_effect() {
    let input = "effect Logger { }";
    let mut p = Parser::new(Lexer::new(input));
    let decl = p.parse_decl().unwrap();
    if let Decl::Effect { name, .. } = decl {
        assert_eq!(name, "Logger");
    } else {
        panic!("Expected Effect declaration");
    }
}

#[test]
fn test_type_with_generics() {
    let input = "type Box<T> = { value: T }";
    let mut p = Parser::new(Lexer::new(input));
    let decl = p.parse_decl().unwrap();
    if let Decl::Type { name, generics, .. } = decl {
        assert_eq!(name, "Box");
        assert_eq!(generics.len(), 1);
    } else {
        panic!("Expected Type with generics");
    }
}

#[test]
fn test_fn_declaration() {
    let input = "pub fn fetch(id: Int) -> String { id }";
    let mut parser = Parser::new(Lexer::new(input));
    let decl = parser.parse_decl().unwrap();
    if let Decl::Fn(fn_decl) = decl {
        assert_eq!(fn_decl.name, "fetch");
        assert!(fn_decl.is_pub);
        assert_eq!(fn_decl.params.len(), 1);
        if let Param::Named { pattern, ty } = &fn_decl.params[0] {
            assert_eq!(pattern.node, Pattern::Bind("id".into()));
            assert_eq!(*ty, Type::Named("Int".to_string()));
        } else {
            panic!("Expected named param");
        }
        assert_eq!(fn_decl.return_type, Some(Type::Named("String".to_string())));
        assert_eq!(fn_decl.body.stmts.len(), 0);
        assert_eq!(fn_decl.body.ret.unwrap().node, Expr::Identifier("id".to_string()));
    } else {
        panic!("Expected Function Declaration");
    }
}

#[test]
fn test_fn_decl_with_effects() {
    let input = "fn foo() -> <io> String { \"hello\" }";
    let mut parser = Parser::new(Lexer::new(input));
    let decl = parser.parse_decl().unwrap();
    if let Decl::Fn(fn_decl) = decl {
        assert_eq!(fn_decl.name, "foo");
        assert_eq!(fn_decl.params.len(), 0);
        assert_eq!(fn_decl.effects.len(), 1);
        assert_eq!(fn_decl.effects[0].name, vec!["io".to_string()]);
        assert_eq!(fn_decl.return_type, Some(Type::Named("String".to_string())));
    } else {
        panic!("Expected Function Declaration");
    }
}

#[test]
fn test_fn_decl_with_generics() {
    let mut p = Parser::new(Lexer::new("fn id<T>(x: T) -> T { x }"));
    let decl = p.parse_decl().unwrap();
    if let Decl::Fn(f) = decl {
        assert_eq!(f.generics.len(), 1);
        assert_eq!(f.generics[0].name, "T");
    } else { panic!("expected Fn"); }
}

#[test]
fn test_fn_decl_with_where_clause() {
    let mut p = Parser::new(Lexer::new("fn cmp<T>(a: T, b: T) -> Bool where T: Ord { true }"));
    let decl = p.parse_decl().unwrap();
    if let Decl::Fn(f) = decl {
        assert_eq!(f.generics.len(), 1);
        assert_eq!(f.where_clause.len(), 1);
        assert_eq!(f.where_clause[0].bounds[0], vec!["Ord".to_string()]);
    } else { panic!("expected Fn"); }
}

#[test]
fn test_fn_decl_multiple_generic_params() {
    let mut p = Parser::new(Lexer::new("fn pair<T, U>(x: T, y: U) -> T { x }"));
    let decl = p.parse_decl().unwrap();
    if let Decl::Fn(f) = decl {
        assert_eq!(f.generics.len(), 2);
        assert_eq!(f.generics[0].name, "T");
        assert_eq!(f.generics[1].name, "U");
    } else { panic!("expected Fn"); }
}

#[test]
fn test_const_fn_with_params() {
    let mut p = Parser::new(Lexer::new("const fn add(x: Int, y: Int): Int = x + y;"));
    let decl = p.parse_decl().unwrap();
    if let Decl::Const { is_fn, params, name, .. } = decl {
        assert!(is_fn);
        assert_eq!(name, "add");
        assert_eq!(params.len(), 2);
    } else { panic!("expected Const"); }
}

#[test]
fn test_const_fn_with_generics() {
    let mut p = Parser::new(Lexer::new("const fn id<T>(x: T): T = x;"));
    let decl = p.parse_decl().unwrap();
    if let Decl::Const { is_fn, generics, params, .. } = decl {
        assert!(is_fn);
        assert_eq!(generics.len(), 1);
        assert_eq!(params.len(), 1);
    } else { panic!("expected Const"); }
}

#[test]
fn test_decl_with_attribute() {
    let mut p = Parser::new(Lexer::new("@export fn handler(req: Int) -> Int { req }"));
    let decl = p.parse_decl().unwrap();
    if let Decl::Fn(f) = decl {
        assert_eq!(f.attrs.len(), 1);
        assert_eq!(f.attrs[0].name, "export");
    } else { panic!("expected Fn"); }
}

#[test]
fn test_decl_with_attribute_args() {
    let mut p = Parser::new(Lexer::new("@derive(Eq, Show) type Foo = { x: Int }"));
    let decl = p.parse_decl().unwrap();
    if let Decl::Type { attrs, .. } = decl {
        assert_eq!(attrs.len(), 1);
        assert_eq!(attrs[0].name, "derive");
        assert_eq!(attrs[0].args.len(), 2);
    } else { panic!("expected Type"); }
}

#[test]
fn test_decl_with_multiple_attributes() {
    let mut p = Parser::new(Lexer::new("@inline @export fn foo() -> Int { 0 }"));
    let decl = p.parse_decl().unwrap();
    if let Decl::Fn(f) = decl {
        assert_eq!(f.attrs.len(), 2);
        assert_eq!(f.attrs[0].name, "inline");
        assert_eq!(f.attrs[1].name, "export");
    } else { panic!("expected Fn"); }
}

#[test]
fn test_decl_attribute_with_named_arg() {
    let mut p = Parser::new(Lexer::new(r#"@cfg(target = "x86") fn foo() -> Int { 0 }"#));
    let decl = p.parse_decl().unwrap();
    if let Decl::Fn(f) = decl {
        assert_eq!(f.attrs.len(), 1);
        match &f.attrs[0].args[0] {
            AttrArg::Named(k, Literal::String(v)) => {
                assert_eq!(k, "target");
                assert_eq!(v, "x86");
            }
            other => panic!("expected Named, got {:?}", other),
        }
    } else { panic!("expected Fn"); }
}

#[test]
fn test_decl_attribute_with_literal_arg() {
    let mut p = Parser::new(Lexer::new("@version(1) fn foo() -> Int { 0 }"));
    let decl = p.parse_decl().unwrap();
    if let Decl::Fn(f) = decl {
        assert!(matches!(f.attrs[0].args[0], AttrArg::Lit(Literal::Int(1))));
    } else { panic!("expected Fn"); }
}

#[test]
fn test_where_clause_with_plus_bounds() {
    let mut p = Parser::new(Lexer::new("fn f<T>(x: T) -> T where T: Ord + Show { x }"));
    let decl = p.parse_decl().unwrap();
    if let Decl::Fn(f) = decl {
        assert_eq!(f.where_clause.len(), 1);
        assert_eq!(f.where_clause[0].bounds.len(), 2);
        assert_eq!(f.where_clause[0].bounds[0], vec!["Ord".to_string()]);
        assert_eq!(f.where_clause[0].bounds[1], vec!["Show".to_string()]);
    } else { panic!("expected Fn"); }
}

#[test]
fn test_where_clause_multiple_comma_separated_bounds() {
    let mut p = Parser::new(Lexer::new(
        "fn f<T, U>(x: T, y: U) -> T where T: Ord, U: Show { x }"
    ));
    let decl = p.parse_decl().unwrap();
    if let Decl::Fn(f) = decl {
        assert_eq!(f.where_clause.len(), 2);
    } else { panic!("expected Fn"); }
}

#[test]
fn test_effect_decl_no_stray_rbrace_token() {
    let input = "effect E { op op() -> Unit } fn main() -> Int { 0 }";
    let mut p = Parser::new(Lexer::new(input));
    let decls = p.parse_program();
    assert!(p.errors.is_empty(), "unexpected errors: {:?}", p.errors);
    assert_eq!(decls.len(), 2);
    assert!(matches!(decls[0], Decl::Effect { .. }));
    assert!(matches!(decls[1], Decl::Fn(_)));
}

#[test]
fn test_effect_empty_no_stray_rbrace() {
    let input = "effect Empty { } fn main() -> Int { 0 }";
    let mut p = Parser::new(Lexer::new(input));
    let decls = p.parse_program();
    assert!(p.errors.is_empty(), "unexpected errors: {:?}", p.errors);
    assert_eq!(decls.len(), 2);
}

#[test]
fn test_effect_op_as_operation_name() {
    let input = "effect E { op op() -> Int }";
    let mut p = Parser::new(Lexer::new(input));
    let decls = p.parse_program();
    assert!(p.errors.is_empty(), "unexpected errors: {:?}", p.errors);
    assert_eq!(decls.len(), 1);
    if let Decl::Effect { name, ops, .. } = &decls[0] {
        assert_eq!(name, "E");
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].name, "op");
        assert!(ops[0].params.is_empty());
    } else {
        panic!("Expected Effect declaration");
    }
}

#[test]
fn test_effect_op_with_params_as_operation_name() {
    let input = "effect E { op op(x: Int) -> Int }";
    let mut p = Parser::new(Lexer::new(input));
    let decls = p.parse_program();
    assert!(p.errors.is_empty(), "unexpected errors: {:?}", p.errors);
    assert_eq!(decls.len(), 1);
    if let Decl::Effect { name, ops, .. } = &decls[0] {
        assert_eq!(name, "E");
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].name, "op");
        assert_eq!(ops[0].params.len(), 1);
    } else {
        panic!("Expected Effect declaration");
    }
}

#[test]
fn test_effect_multiple_op_operations() {
    let input = "effect E { op op() -> Int op other() -> Unit }";
    let mut p = Parser::new(Lexer::new(input));
    let decls = p.parse_program();
    assert!(p.errors.is_empty(), "unexpected errors: {:?}", p.errors);
    assert_eq!(decls.len(), 1);
    if let Decl::Effect { name, ops, .. } = &decls[0] {
        assert_eq!(name, "E");
        assert_eq!(ops.len(), 2);
        assert_eq!(ops[0].name, "op");
        assert_eq!(ops[1].name, "other");
    } else {
        panic!("Expected Effect declaration");
    }
}

#[test]
fn test_program_effect_alias_with_semicolon_then_fn() {
    let decls = parse_program_no_errors(
        "effect alias E = <exn>; fn main() -> Int { 0 }"
    );
    assert_eq!(decls.len(), 2);
    assert!(matches!(decls[0], Decl::EffectAlias { .. }));
    assert!(matches!(decls[1], Decl::Fn(_)));
}

#[test]
fn test_decl_error_surfaced_via_parser_errors() {
    let errs = parse_errs("@");
    assert!(!errs.is_empty(), "parser must record decl-level errors, got none");
    assert!(errs.iter().any(|e| e.to_lowercase().contains("attribute") || e.contains("@")),
            "expected attribute-related error, got: {:?}", errs);
}

#[test]
fn test_decl_error_does_not_swallow_message_silently() {
    let errs = parse_errs("@foo(\n");
    assert!(!errs.is_empty(), "malformed attribute decl must report an error");
}
