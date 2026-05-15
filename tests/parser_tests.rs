use ect::ast::*;
use ect::lexer::Lexer;
use ect::parser::{Parser, Precedence};

fn ty(input: &str) -> Type {
    let mut p = Parser::new(Lexer::new(input));
    p.parse_type().expect("type parse failed")
}

#[test]
fn test_type_named() {
    assert_eq!(ty("Int"), Type::Named("Int".into()));
    assert_eq!(ty("Self"), Type::Named("Self".into()));
}

#[test]
fn test_type_generic() {
    assert_eq!(ty("List<Int>"), Type::Generic { name: "List".into(), args: vec![Type::Named("Int".into())] });
    assert_eq!(ty("Result<T, E>"), Type::Generic {
        name: "Result".into(),
        args: vec![Type::Named("T".into()), Type::Named("E".into())],
    });
}

#[test]
fn test_type_qualified() {
    assert_eq!(ty("io.Error"), Type::Qualified(vec!["io".into(), "Error".into()]));
    assert_eq!(ty("a.b.c"), Type::Qualified(vec!["a".into(), "b".into(), "c".into()]));
}

#[test]
fn test_type_array() {
    assert_eq!(ty("[Int; 16]"), Type::Array { elem: Box::new(Type::Named("Int".into())), size: 16 });
    assert_eq!(ty("[Bool; 4]"), Type::Array { elem: Box::new(Type::Named("Bool".into())), size: 4 });
}

#[test]
fn test_type_tuple() {
    assert_eq!(ty("()"), Type::Tuple(vec![]));
    assert_eq!(ty("(Int,)"), Type::Tuple(vec![Type::Named("Int".into())]));
    assert_eq!(ty("(Int, Bool)"), Type::Tuple(vec![Type::Named("Int".into()), Type::Named("Bool".into())]));
    assert_eq!(ty("(Int, Bool, String)"), Type::Tuple(vec![
        Type::Named("Int".into()), Type::Named("Bool".into()), Type::Named("String".into()),
    ]));
}

#[test]
fn test_type_reference() {
    assert_eq!(ty("&Int"), Type::Reference { is_mut: false, inner: Box::new(Type::Named("Int".into())), region: None });
    assert_eq!(ty("&mut String"), Type::Reference { is_mut: true, inner: Box::new(Type::Named("String".into())), region: None });
    assert_eq!(ty("&Int in r"), Type::Reference { is_mut: false, inner: Box::new(Type::Named("Int".into())), region: Some("r".into()) });
    assert_eq!(ty("&mut T in heap"), Type::Reference { is_mut: true, inner: Box::new(Type::Named("T".into())), region: Some("heap".into()) });
}

#[test]
fn test_type_function() {
    assert_eq!(ty("() -> String"), Type::Function {
        params: vec![], effects: vec![], ret: Box::new(Type::Named("String".into())),
    });
    assert_eq!(ty("(Int) -> Bool"), Type::Function {
        params: vec![Type::Named("Int".into())], effects: vec![], ret: Box::new(Type::Named("Bool".into())),
    });
    assert_eq!(ty("(Int, String) -> Bool"), Type::Function {
        params: vec![Type::Named("Int".into()), Type::Named("String".into())],
        effects: vec![],
        ret: Box::new(Type::Named("Bool".into())),
    });
    assert_eq!(ty("(Int) -> Option<String>"), Type::Function {
        params: vec![Type::Named("Int".into())],
        effects: vec![],
        ret: Box::new(Type::Generic { name: "Option".into(), args: vec![Type::Named("String".into())] }),
    });
}

#[test]
fn test_type_function_effects() {
    assert_eq!(ty("(Int) -> <exn> String"), Type::Function {
        params: vec![Type::Named("Int".into())],
        effects: vec![EffectItem { name: vec!["exn".into()], arg: None }],
        ret: Box::new(Type::Named("String".into())),
    });
    assert_eq!(ty("(Int) -> <exn, io> String"), Type::Function {
        params: vec![Type::Named("Int".into())],
        effects: vec![
            EffectItem { name: vec!["exn".into()], arg: None },
            EffectItem { name: vec!["io".into()],  arg: None },
        ],
        ret: Box::new(Type::Named("String".into())),
    });
    assert_eq!(ty("(Int) -> <exn<E>> String"), Type::Function {
        params: vec![Type::Named("Int".into())],
        effects: vec![EffectItem { name: vec!["exn".into()], arg: Some(Box::new(Type::Named("E".into()))) }],
        ret: Box::new(Type::Named("String".into())),
    });
}

#[test]
fn test_expr_if() {
    let input = "if true { 1 } else { 2 }";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::If { condition, consequence: _, alternative } = expr.node {
        assert_eq!(condition.node, Expr::Literal(Literal::Bool(true)));
        assert!(alternative.is_some());
    } else {
        panic!("Expected If expression");
    }
}

#[test]
fn test_expr_if_without_else() {
    let input = "if x > 5 { 10 }";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::If { condition, alternative, .. } = expr.node {
        assert!(matches!(condition.node, Expr::Binary { .. }));
        assert!(alternative.is_none());
    } else {
        panic!("Expected If expression");
    }
}

#[test]
fn test_expr_if_else_if_chain() {
    let input = "if x { 1 } else if y { 2 } else { 3 }";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::If { alternative: Some(alt), .. } = expr.node {
        assert!(matches!(alt.node, Expr::If { .. }));
    } else {
        panic!("Expected If expression with else if");
    }
}

#[test]
fn test_expr_match() {
    let input = "match x { A => 1, B => 2 }";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::Match { scrutinee, arms } = expr.node {
        assert_eq!(scrutinee.node, Expr::Identifier("x".into()));
        assert_eq!(arms.len(), 2);
    } else {
        panic!("Expected Match expression");
    }
}

#[test]
fn test_expr_match_with_guard() {
    let input = "match x { 1 if x > 0 => true, _ => false }";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::Match { scrutinee: _, arms } = expr.node {
        assert_eq!(arms.len(), 2);
        assert!(arms[0].guard.is_some());
        assert!(arms[1].guard.is_none());
    } else {
        panic!("Expected Match expression");
    }
}

#[test]
fn test_expr_match_block_body() {
    let input = "match x { A => { print(1); 1 }, B => 2 }";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::Match { arms, .. } = expr.node {
        assert_eq!(arms.len(), 2);
    } else {
        panic!("Expected Match expression");
    }
}

#[test]
fn test_expr_for() {
    let input = "for x in items { x }";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::For { pattern, iter, body } = expr.node {
        assert_eq!(pattern.node, Pattern::Bind("x".into()));
        assert_eq!(iter.node, Expr::Identifier("items".into()));
        assert_eq!(body.stmts.len(), 0);
    } else {
        panic!("Expected For expression");
    }
}

#[test]
fn test_expr_for_tuple_destructure() {
    let input = "for (x, y) in pairs { x + y }";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::For { pattern, iter, .. } = expr.node {
        assert!(matches!(pattern.node, Pattern::Tuple(_)));
        assert_eq!(iter.node, Expr::Identifier("pairs".into()));
    } else {
        panic!("Expected For expression");
    }
}

#[test]
fn test_expr_while() {
    let input = "while true { 1 }";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::While { condition, body } = expr.node {
        assert_eq!(condition.node, Expr::Literal(Literal::Bool(true)));
        assert_eq!(body.stmts.len(), 0);
    } else {
        panic!("Expected While expression");
    }
}

#[test]
fn test_expr_while_complex_condition() {
    let input = "while x < 10 { x = x + 1 }";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::While { condition, body } = expr.node {
        assert!(matches!(condition.node, Expr::Binary { .. }));
        assert!(body.stmts.len() > 0 || body.ret.is_some());
    } else {
        panic!("Expected While expression");
    }
}

#[test]
fn test_expr_loop() {
    let input = "loop { break }";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::Loop { body } = expr.node {
        assert_eq!(body.stmts.len(), 0);
        assert!(matches!(body.ret, Some(r) if matches!(r.node, Expr::Break(_))));
    } else {
        panic!("Expected Loop expression");
    }
}

#[test]
fn test_expr_loop_with_continue() {
    let input = "loop { if x { continue } }";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::Loop { body } = expr.node {
        assert!(matches!(body.ret, Some(r) if matches!(r.node, Expr::If { .. })));
    } else {
        panic!("Expected Loop expression");
    }
}

#[test]
fn test_expr_closure() {
    let input = "|x| x + 1";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::Closure { params, .. } = expr.node {
        assert_eq!(params.len(), 1);
    } else {
        panic!("Expected Closure expression");
    }
}

#[test]
fn test_expr_resume_no_arg() {
    let input = "resume()";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::Resume(arg) = expr.node {
        assert!(arg.is_none());
    } else {
        panic!("Expected Resume expression, got {:?}", expr.node);
    }
}

#[test]
fn test_expr_resume_with_arg() {
    let input = "resume(42)";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::Resume(Some(_)) = expr.node {
        // ok
    } else {
        panic!("Expected Resume expression with arg, got {:?}", expr.node);
    }
}

#[test]
fn test_expr_region() {
    let input = "region r { 1 }";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::Region { label, .. } = expr.node {
        assert_eq!(label, Some("r".into()));
    } else {
        panic!("Expected Region expression");
    }
}

#[test]
fn test_expr_region_without_label() {
    let input = "region { let x = 5; x }";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::Region { label, .. } = expr.node {
        assert_eq!(label, None);
    } else {
        panic!("Expected Region expression");
    }
}

#[test]
fn test_expr_handle() {
    let input = "handle foo { return => 0 }";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::Handle { expr: _, arms } = expr.node {
        assert_eq!(arms.len(), 1);
    } else {
        panic!("Expected Handle expression");
    }
}

#[test]
fn test_expr_handle_multiple_arms() {
    let input = "handle computation { return x => x, exn e => 0 }";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::Handle { expr: _, arms } = expr.node {
        assert_eq!(arms.len(), 2);
    } else {
        panic!("Expected Handle expression");
    }
}

#[test]
fn test_pattern_basic() {
    let mut p = Parser::new(Lexer::new("x"));
    let pat = p.parse_pattern().unwrap();
    assert_eq!(pat.node, Pattern::Bind("x".into()));
    let mut p = Parser::new(Lexer::new("_"));
    let pat = p.parse_pattern().unwrap();
    assert_eq!(pat.node, Pattern::Wildcard);
    let mut p = Parser::new(Lexer::new("42"));
    let pat = p.parse_pattern().unwrap();
    assert_eq!(pat.node, Pattern::Literal(Literal::Int(42)));
}

#[test]
fn test_pattern_tuple() {
    let mut p = Parser::new(Lexer::new("(x, 42)"));
    let pat = p.parse_pattern().unwrap();
    if let Pattern::Tuple(pats) = pat.node {
        assert_eq!(pats.len(), 2);
    } else {
        panic!("Expected tuple pattern");
    }
}

#[test]
fn test_pattern_or() {
    let mut p = Parser::new(Lexer::new("A | B"));
    let pat = p.parse_pattern().unwrap();
    if let Pattern::Or(pats) = pat.node {
        assert_eq!(pats.len(), 2);
    } else {
        panic!("Expected or pattern");
    }
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
fn test_decl_import() {
    let input = "import std.io { Read, Write };";
    let mut p = Parser::new(Lexer::new(input));
    let decl = p.parse_decl().unwrap();
    if let Decl::Import { path, items } = decl {
        assert_eq!(path.len(), 2);
        assert_eq!(path[0], "std");
        assert_eq!(path[1], "io");
        assert_eq!(items.len(), 2);
    } else {
        panic!("Expected Import declaration");
    }
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
fn test_program_effect_with_op_no_stray_token() {
    // Regression: parse_effect_decl used to leave current_token on the
    // closing '}', causing parse_program to report a stray RBrace.
    let input = "effect Gen { fn yield(v: Int) -> Unit }";
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
    // Two declarations in sequence — the parser must advance past the effect's
    // '}' so the following 'fn' is recognised as the next top-level decl.
    let input = "effect Logger { fn log(msg: Int) -> Unit } fn main() -> Int { 0 }";
    let mut p = Parser::new(Lexer::new(input));
    let decls = p.parse_program();
    assert!(p.errors.is_empty(), "unexpected parser errors: {:?}", p.errors);
    assert_eq!(decls.len(), 2);
    assert!(matches!(decls[0], Decl::Effect { .. }));
    assert!(matches!(decls[1], Decl::Fn(_)));
}

#[test]
fn test_program_empty_effect_followed_by_fn() {
    // Same path for the empty-ops case.
    let input = "effect Marker { } fn main() -> Int { 1 }";
    let mut p = Parser::new(Lexer::new(input));
    let decls = p.parse_program();
    assert!(p.errors.is_empty(), "unexpected parser errors: {:?}", p.errors);
    assert_eq!(decls.len(), 2);
}

#[test]
fn test_program_effect_multiple_ops() {
    let input = "effect IO { fn read() -> Int fn write(n: Int) -> Unit }";
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
fn test_let_statements() {
    let input = "let mut x: Int = 10;";
    let mut parser = Parser::new(Lexer::new(input));

    let spanned_stmt = parser.parse_stmt().unwrap();

    if let Stmt::Let { pattern, is_mut, ty, value } = spanned_stmt.node {
        assert_eq!(pattern.node, Pattern::Bind("x".into()));
        assert!(is_mut);
        assert_eq!(ty, Some(Type::Named("Int".to_string())));
        assert_eq!(value.node, Expr::Literal(Literal::Int(10)));
    } else {
        panic!("Expected Let statement");
    }
}

#[test]
fn test_let_tuple_pattern() {
    let input = "let (x, y) = (1, 2);";
    let mut parser = Parser::new(Lexer::new(input));

    let spanned_stmt = parser.parse_stmt().unwrap();

    if let Stmt::Let { pattern, is_mut, ty, .. } = spanned_stmt.node {
        assert!(!is_mut);
        assert!(ty.is_none());
        if let Pattern::Tuple(pats) = pattern.node {
            assert_eq!(pats.len(), 2);
            assert_eq!(pats[0].node, Pattern::Bind("x".into()));
            assert_eq!(pats[1].node, Pattern::Bind("y".into()));
        } else {
            panic!("Expected tuple pattern");
        }
    } else {
        panic!("Expected Let statement");
    }
}

#[test]
fn test_let_wildcard_pattern() {
    let input = "let _ = 42;";
    let mut parser = Parser::new(Lexer::new(input));

    let spanned_stmt = parser.parse_stmt().unwrap();

    if let Stmt::Let { pattern, is_mut, ty, value } = spanned_stmt.node {
        assert!(!is_mut);
        assert!(ty.is_none());
        assert_eq!(pattern.node, Pattern::Wildcard);
        assert_eq!(value.node, Expr::Literal(Literal::Int(42)));
    } else {
        panic!("Expected Let statement");
    }
}

#[test]
fn test_let_nested_tuple_pattern() {
    let input = "let (x, (a, b)): (Int, (Bool, String)) = (1, (true, \"hi\"));";
    let mut parser = Parser::new(Lexer::new(input));

    let spanned_stmt = parser.parse_stmt().unwrap();

    if let Stmt::Let { pattern, is_mut, ty, value: _ } = spanned_stmt.node {
        assert!(!is_mut);
        assert!(ty.is_some());

        if let Pattern::Tuple(pats) = pattern.node {
            assert_eq!(pats.len(), 2);
            assert_eq!(pats[0].node, Pattern::Bind("x".into()));

            if let Pattern::Tuple(inner_pats) = &pats[1].node {
                assert_eq!(inner_pats.len(), 2);
                assert_eq!(inner_pats[0].node, Pattern::Bind("a".into()));
                assert_eq!(inner_pats[1].node, Pattern::Bind("b".into()));
            } else {
                panic!("Expected nested tuple pattern");
            }
        } else {
            panic!("Expected tuple pattern");
        }
    } else {
        panic!("Expected Let statement");
    }
}

#[test]
fn test_operator_precedence() {
    let input = "1 + 2 * 3";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);

    if let Expr::Binary { op, left, right } = expr.node {
        assert_eq!(op, BinaryOp::Add);
        assert_eq!(left.node, Expr::Literal(Literal::Int(1)));
        
        if let Expr::Binary { op: op_inner, left: l_inner, right: r_inner } = &right.node {
            assert_eq!(*op_inner, BinaryOp::Mul);
            assert_eq!(l_inner.node, Expr::Literal(Literal::Int(2)));
            assert_eq!(r_inner.node, Expr::Literal(Literal::Int(3)));
        } else {
            panic!("Right side of addition should be multiplication");
        }
    } else {
        panic!("Expected Binary expression");
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
fn test_fn_type_with_effects() {
    assert_eq!(
        ty("() -> <io> String"),
        Type::Function {
            params: vec![],
            effects: vec![EffectItem { name: vec!["io".into()], arg: None }],
            ret: Box::new(Type::Named("String".into())),
        }
    );

    assert_eq!(
        ty("(Int) -> <io, exn> Bool"),
        Type::Function {
            params: vec![Type::Named("Int".into())],
            effects: vec![
                EffectItem { name: vec!["io".into()], arg: None },
                EffectItem { name: vec!["exn".into()], arg: None },
            ],
            ret: Box::new(Type::Named("Bool".into())),
        }
    );

    assert_eq!(
        ty("() -> <exn<String>> Int"),
        Type::Function {
            params: vec![],
            effects: vec![EffectItem { name: vec!["exn".into()], arg: Some(Box::new(Type::Named("String".into()))) }],
            ret: Box::new(Type::Named("Int".into())),
        }
    );
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

fn fn_body_expr(input: &str) -> Expr {
    let mut p = Parser::new(Lexer::new(input));
    let decls = p.parse_program();
    assert!(p.errors.is_empty(), "parse errors: {:?}", p.errors.iter().map(|e| &e.message).collect::<Vec<_>>());
    let fn_decl = decls.into_iter().find_map(|d| match d {
        Decl::Fn(f) => Some(f),
        _ => None,
    }).expect("expected a function declaration");

    fn_decl.body.ret.map(|b| b.node).or_else(|| {
        fn_decl.body.stmts.last().and_then(|s| match &s.node {
            Stmt::Expr(e) => Some(e.node.clone()),
            _ => None,
        })
    }).expect("expected an expression in fn body")
}

#[test]
fn test_if_else_inside_fn_body_keeps_alternative() {
    let input = "fn f(n: Int) -> Int { if n <= 1 { n } else { n - 1 } }";
    let body = fn_body_expr(input);
    let Expr::If { alternative, .. } = body else {
        panic!("expected If at fn body, got {:?}", body);
    };
    assert!(alternative.is_some(), "else branch was dropped");
}

#[test]
fn test_recursive_fn_with_else_preserves_base_case() {
    // Test no recursive call leaking out of the else
    let input = "
        fn fib(n: Int) -> Int {
            if n <= 1 { n } else { fib(n - 1) + fib(n - 2) }
        }
    ";
    let body = fn_body_expr(input);
    let Expr::If { consequence, alternative, .. } = body else {
        panic!("expected If at fn body, got {:?}", body);
    };
    // Consequence is the base case `n`.
    let Expr::Block(cons_block) = &consequence.node else {
        panic!("expected Block consequence");
    };
    let Some(cons_ret) = &cons_block.ret else {
        panic!("expected consequence to have a tail expression");
    };
    assert!(matches!(cons_ret.node, Expr::Identifier(_)));

    let alt = alternative.expect("else branch missing — base case would be lost");
    let Expr::Block(alt_block) = &alt.node else {
        panic!("expected Block alternative");
    };
    let alt_ret = alt_block.ret.as_ref().expect("expected alternative tail expr");
    assert!(matches!(alt_ret.node, Expr::Binary { op: BinaryOp::Add, .. }));
}

#[test]
fn test_nested_if_else_chain_inside_fn_body() {
    let input = "
        fn classify(n: Int) -> Int {
            if n < 0 {
                0
            } else {
                if n == 0 { 1 } else { if n < 10 { 2 } else { 3 } }
            }
        }
    ";
    let body = fn_body_expr(input);
    let mut current = body;
    for depth in 0..3 {
        let Expr::If { alternative, .. } = current else {
            panic!("expected If at depth {}", depth);
        };
        let alt = alternative.unwrap_or_else(|| panic!("else dropped at depth {}", depth));
        // Unwrap the wrapping Block(s) from `else { ... }`.
        let mut inner = alt.node;
        while let Expr::Block(b) = inner {
            inner = b.ret.expect("expected tail expr").node;
        }
        current = inner;
    }
}

#[test]
fn test_match_newline_separated_arms() {
    let input = "
        fn pick(x: Int) -> Int {
            match x {
                0 => 10
                1 => 20
                _ => 30
            }
        }
    ";
    let body = fn_body_expr(input);
    let Expr::Match { arms, .. } = body else {
        panic!("expected Match at fn body");
    };
    assert_eq!(arms.len(), 3);
}

#[test]
fn test_nested_match_block_body_arm() {
    let input = "
        fn quadrant(x: Int, y: Int) -> Int {
            match x {
                0 => 0
                1 => match y {
                    1 => 1
                    _ => 0
                }
                _ => 0
            }
        }
    ";
    let body = fn_body_expr(input);
    let Expr::Match { arms, .. } = body else {
        panic!("expected outer Match");
    };
    assert_eq!(arms.len(), 3, "outer match should keep all three arms");
    assert!(matches!(arms[1].body.node, Expr::Match { .. }),
        "expected nested Match as arm body, got {:?}", arms[1].body.node);
}

#[test]
fn test_match_block_body_with_following_arm_no_comma() {
    let input = "
        fn pick(x: Int) -> Int {
            match x {
                0 => { let a = 1; a }
                _ => 0
            }
        }
    ";
    let body = fn_body_expr(input);
    let Expr::Match { arms, .. } = body else {
        panic!("expected Match");
    };
    assert_eq!(arms.len(), 2);
    assert!(matches!(arms[0].body.node, Expr::Block(_)));
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

fn parse_errs(input: &str) -> Vec<String> {
    let mut p = Parser::new(Lexer::new(input));
    let _ = p.parse_program();
    p.errors.into_iter().map(|e| e.message).collect()
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
    assert!(!errs.is_empty(),
        "expected error for duplicate else");
}

#[test]
fn test_error_stray_keyword_after_if_consequence() {
    let errs = parse_errs("fn f() -> Int { if x { 1 } banana { 2 } }");
    assert!(!errs.is_empty(),
        "expected error for stray ident between if and following block");
}

#[test]
fn test_error_unclosed_fn_body() {
    let errs = parse_errs("fn f() -> Int { 1");
    assert!(!errs.is_empty(),
        "expected error for unclosed fn body");
}

#[test]
fn test_error_two_exprs_no_separator() {
    let errs = parse_errs("fn f() -> Int { 1 2 }");
    assert!(!errs.is_empty(),
        "expected error for two stmts without separator");
}

#[test]
fn test_error_extra_else_in_chain() {
    let errs = parse_errs(
        "fn f() -> Int { if a { 1 } else if b { 2 } else { 3 } else { 4 } }"
    );
    assert!(!errs.is_empty(),
        "expected error for extra else after terminal else");
}

#[test]
fn test_error_match_garbage_between_arms() {
    let errs = parse_errs("fn f() -> Int { match x { 1 => 1 banana 2 => 2 } }");
    assert!(!errs.is_empty(),
        "expected error for garbage between match arms");
}

#[test]
fn test_error_extra_token_after_complete_decl() {
    let errs = parse_errs("fn f() -> Int { 1 }  banana  fn g() -> Int { 2 }");
    assert!(!errs.is_empty(),
        "expected error for stray top-level token");
}

fn parse_program_no_errors(input: &str) -> Vec<Decl> {
    let mut p = Parser::new(Lexer::new(input));
    let decls = p.parse_program();
    assert!(p.errors.is_empty(), "unexpected parser errors: {:?}", p.errors);
    decls
}

#[test]
fn test_program_mod_then_fn() {
    let decls = parse_program_no_errors("mod foo fn main() -> Int { 0 }");
    assert_eq!(decls.len(), 2);
    assert!(matches!(decls[0], Decl::Mod(ref s) if s == "foo"));
    assert!(matches!(decls[1], Decl::Fn(_)));
}

#[test]
fn test_program_mod_dotted_path() {
    let decls = parse_program_no_errors("mod a.b.c fn main() -> Int { 0 }");
    assert_eq!(decls.len(), 2);
    assert!(matches!(decls[0], Decl::Mod(ref s) if s == "a.b.c"));
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
    let decls = parse_program_no_errors("import std.io { Read, Write }; fn main() -> Int { 0 }");
    assert_eq!(decls.len(), 2);
    assert!(matches!(decls[0], Decl::Import { .. }));
}

#[test]
fn test_program_import_dot_brace_syntax() {
    // BNF canonical: `.{ ... }` for the import list.
    let decls = parse_program_no_errors("import io.{File, Read}; fn main() -> Int { 0 }");
    assert_eq!(decls.len(), 2);
    if let Decl::Import { path, items } = &decls[0] {
        assert_eq!(path, &vec!["io".to_string()]);
        assert_eq!(items.len(), 2);
    } else { panic!("expected Import"); }
}

#[test]
fn test_handle_missing_comma_between_atom_arms_reports_error() {
    let errs = parse_errs(
        "fn main() -> Int { handle f() { return v => v exn e => 0 } }",
    );
    assert!(errs.iter().any(|m| m.contains("Expected ',' or '}'")),
        "expected comma-required error, got: {:?}", errs);
}

#[test]
fn test_plus_assign_desugars_to_assign_of_add() {
    // a += 1  --->  a = a + 1
    let input = "a += 1";
    let mut p = Parser::new(Lexer::new(input));
    let expr = p.parse_expr(Precedence::Lowest);
    if let Expr::Binary { op: BinaryOp::Assign, left, right } = expr.node {
        assert!(matches!(left.node, Expr::Identifier(ref n) if n == "a"));
        if let Expr::Binary { op: BinaryOp::Add, .. } = right.node {
            // ok
        } else { panic!("expected Add on RHS, got {:?}", right.node); }
    } else { panic!("expected Assign at top, got {:?}", expr.node); }
}

#[test]
fn test_minus_assign_desugars_to_assign_of_sub() {
    let input = "a -= 1";
    let mut p = Parser::new(Lexer::new(input));
    let expr = p.parse_expr(Precedence::Lowest);
    if let Expr::Binary { op: BinaryOp::Assign, right, .. } = expr.node {
        assert!(matches!(right.node, Expr::Binary { op: BinaryOp::Sub, .. }));
    } else { panic!("expected Assign with Sub RHS"); }
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
fn test_array_repeat_literal() {
    let mut p = Parser::new(Lexer::new("[0; 4]"));
    let expr = p.parse_expr(Precedence::Lowest);
    if let Expr::ArrayRepeat { elem, count } = expr.node {
        assert!(matches!(elem.node, Expr::Literal(Literal::Int(0))));
        assert!(matches!(count.node, Expr::Literal(Literal::Int(4))));
    } else { panic!("expected ArrayRepeat, got {:?}", expr.node); }
}

#[test]
fn test_array_list_literal_still_works() {
    let mut p = Parser::new(Lexer::new("[1, 2, 3]"));
    let expr = p.parse_expr(Precedence::Lowest);
    if let Expr::Array(items) = expr.node {
        assert_eq!(items.len(), 3);
    } else { panic!("expected Array, got {:?}", expr.node); }
}

#[test]
fn test_resume_expr_no_arg_in_function() {
    let input = "fn f() -> Int { resume() }";
    let mut p = Parser::new(Lexer::new(input));
    let decls = p.parse_program();
    assert!(p.errors.is_empty(), "unexpected errors: {:?}", p.errors);
    assert_eq!(decls.len(), 1);
    if let Decl::Fn(fn_decl) = &decls[0] {
        // Check that the body contains a resume expression
        let expr = fn_decl.body.ret.as_ref().expect("expected return expr");
        assert!(matches!(expr.node, Expr::Resume(None)));
    } else { panic!("expected Fn"); }
}

#[test]
fn test_resume_expr_with_arg_in_function() {
    let input = "fn f() -> Int { resume(5) }";
    let mut p = Parser::new(Lexer::new(input));
    let decls = p.parse_program();
    assert!(p.errors.is_empty(), "unexpected errors: {:?}", p.errors);
    assert_eq!(decls.len(), 1);
    if let Decl::Fn(fn_decl) = &decls[0] {
        let expr = fn_decl.body.ret.as_ref().expect("expected return expr");
        if let Expr::Resume(Some(arg)) = &expr.node {
            assert!(matches!(arg.node, Expr::Literal(Literal::Int(5))));
        } else { panic!("expected Resume with Some arg"); }
    } else { panic!("expected Fn"); }
}

#[test]
fn test_resume_with_complex_arg() {
    let input = "fn f() -> Int { resume(x + 1) }";
    let mut p = Parser::new(Lexer::new(input));
    let decls = p.parse_program();
    assert!(p.errors.is_empty(), "unexpected errors: {:?}", p.errors);
    if let Decl::Fn(fn_decl) = &decls[0] {
        let expr = fn_decl.body.ret.as_ref().expect("expected return expr");
        if let Expr::Resume(Some(arg)) = &expr.node {
            assert!(matches!(arg.node, Expr::Binary { op: BinaryOp::Add, .. }));
        } else { panic!("expected Resume with binary expr arg"); }
    } else { panic!("expected Fn"); }
}

#[test]
fn test_effect_decl_no_stray_rbrace_token() {
    let input = "effect E { fn op() -> Unit } fn main() -> Int { 0 }";
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
fn test_resume_paren_required() {
    // resume must have parentheses
    let errs = parse_errs("fn f() -> Int { resume }");
    assert!(!errs.is_empty(),
        "resume without parens should fail");
}

#[test]
fn test_resume_closing_paren_required() {
    let errs = parse_errs("fn f() -> Int { resume(1 }");
    assert!(!errs.is_empty(),
        "resume with missing closing paren should fail");
}

#[test]
fn test_paren_unit_literal() {
    let mut p = Parser::new(Lexer::new("()"));
    let expr = p.parse_expr(Precedence::Lowest);
    assert_eq!(expr.node, Expr::Literal(Literal::Unit));
}

#[test]
fn test_paren_single_expr_strips_parens() {
    // (1 + 2) should not become Tuple — it is a parenthesised expression and
    // parse_paren_expr unwraps it to the inner Expr::Binary.
    let mut p = Parser::new(Lexer::new("(1 + 2)"));
    let expr = p.parse_expr(Precedence::Lowest);
    assert!(matches!(expr.node, Expr::Binary { op: BinaryOp::Add, .. }),
        "expected Binary Add, got {:?}", expr.node);
}

#[test]
fn test_paren_two_element_tuple() {
    let mut p = Parser::new(Lexer::new("(1, 2)"));
    let expr = p.parse_expr(Precedence::Lowest);
    if let Expr::Tuple(elems) = expr.node {
        assert_eq!(elems.len(), 2);
    } else { panic!("expected Tuple, got {:?}", expr.node); }
}

#[test]
fn test_paren_three_element_tuple() {
    let mut p = Parser::new(Lexer::new("(1, 2, 3)"));
    let expr = p.parse_expr(Precedence::Lowest);
    if let Expr::Tuple(elems) = expr.node {
        assert_eq!(elems.len(), 3);
    } else { panic!("expected Tuple, got {:?}", expr.node); }
}

#[test]
fn test_paren_tuple_trailing_comma() {
    let mut p = Parser::new(Lexer::new("(1, 2,)"));
    let expr = p.parse_expr(Precedence::Lowest);
    if let Expr::Tuple(elems) = expr.node {
        assert_eq!(elems.len(), 2);
    } else { panic!("expected Tuple, got {:?}", expr.node); }
}

#[test]
fn test_decl_with_multiple_attributes() {
    // The `while self.current_token == Token::At` loop must accumulate.
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
    // `@cfg(key = "val")` exercises the AttrArg::Named branch.
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
    // Bare-literal AttrArg path: `@version(1)`.
    let mut p = Parser::new(Lexer::new("@version(1) fn foo() -> Int { 0 }"));
    let decl = p.parse_decl().unwrap();
    if let Decl::Fn(f) = decl {
        assert!(matches!(f.attrs[0].args[0], AttrArg::Lit(Literal::Int(1))));
    } else { panic!("expected Fn"); }
}

#[test]
fn test_where_clause_with_plus_bounds() {
    // `T: A + B` — single bound entry with two trait paths.
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
    // `where T: Ord, U: Show` — two distinct WhereBound entries.
    let mut p = Parser::new(Lexer::new(
        "fn f<T, U>(x: T, y: U) -> T where T: Ord, U: Show { x }"
    ));
    let decl = p.parse_decl().unwrap();
    if let Decl::Fn(f) = decl {
        assert_eq!(f.where_clause.len(), 2);
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
fn test_const_fn_with_generics() {
    // const fn with both generics and params — exercises both optional branches
    // in parse_const_decl.
    let mut p = Parser::new(Lexer::new("const fn id<T>(x: T): T = x;"));
    let decl = p.parse_decl().unwrap();
    if let Decl::Const { is_fn, generics, params, .. } = decl {
        assert!(is_fn);
        assert_eq!(generics.len(), 1);
        assert_eq!(params.len(), 1);
    } else { panic!("expected Const"); }
}

#[test]
fn test_program_effect_alias_with_semicolon_then_fn() {
    // Symmetric to test_program_type_alias_with_semicolon_then_fn.
    let decls = parse_program_no_errors(
        "effect alias E = <exn>; fn main() -> Int { 0 }"
    );
    assert_eq!(decls.len(), 2);
    assert!(matches!(decls[0], Decl::EffectAlias { .. }));
    assert!(matches!(decls[1], Decl::Fn(_)));
}

#[test]
fn test_array_list_trailing_comma() {
    let mut p = Parser::new(Lexer::new("[1, 2, 3,]"));
    let expr = p.parse_expr(Precedence::Lowest);
    if let Expr::Array(items) = expr.node {
        assert_eq!(items.len(), 3);
    } else { panic!("expected Array"); }
}
