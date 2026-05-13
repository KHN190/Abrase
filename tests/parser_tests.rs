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
    // nested generic return
    assert_eq!(ty("(Int) -> Option<String>"), Type::Function {
        params: vec![Type::Named("Int".into())],
        effects: vec![],
        ret: Box::new(Type::Generic { name: "Option".into(), args: vec![Type::Named("String".into())] }),
    });
}

#[test]
fn test_type_function_effects() {
    // (Int) -> <exn> String
    assert_eq!(ty("(Int) -> <exn> String"), Type::Function {
        params: vec![Type::Named("Int".into())],
        effects: vec![EffectItem { name: vec!["exn".into()], arg: None }],
        ret: Box::new(Type::Named("String".into())),
    });
    // (Int) -> <exn, io> String  — multiple effects
    assert_eq!(ty("(Int) -> <exn, io> String"), Type::Function {
        params: vec![Type::Named("Int".into())],
        effects: vec![
            EffectItem { name: vec!["exn".into()], arg: None },
            EffectItem { name: vec!["io".into()],  arg: None },
        ],
        ret: Box::new(Type::Named("String".into())),
    });
    // (Int) -> <exn<E>> String  — parameterised effect
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
        // Alternative should be another if expression
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
        // First arm should have a guard
        assert!(arms[0].guard.is_some());
        // Second arm should not have a guard
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
        // Pattern should be tuple pattern
        assert!(matches!(pattern.node, Pattern::Tuple(_)));
        // Iterator should be identifier pairs
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
        // Condition should be a binary expression
        assert!(matches!(condition.node, Expr::Binary { .. }));
        // Body should have at least one statement
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
        // If is the return expression of the loop, not a statement
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
fn test_expr_scope() {
    let input = "scope s { 1 }";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::Scope { label, .. } = expr.node {
        assert_eq!(label, Some("s".into()));
    } else {
        panic!("Expected Scope expression");
    }
}

#[test]
fn test_expr_scope_without_label() {
    let input = "scope { 1 }";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::Scope { label, .. } = expr.node {
        assert_eq!(label, None);
    } else {
        panic!("Expected Scope expression");
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
    // Bind pattern
    let mut p = Parser::new(Lexer::new("x"));
    let pat = p.parse_pattern().unwrap();
    assert_eq!(pat.node, Pattern::Bind("x".into()));

    // Wildcard
    let mut p = Parser::new(Lexer::new("_"));
    let pat = p.parse_pattern().unwrap();
    assert_eq!(pat.node, Pattern::Wildcard);

    // Literal
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
        // impl parsed successfully
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
        // Check that pattern is a tuple pattern with 2 elements
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
        // Check outer tuple pattern
        if let Pattern::Tuple(pats) = pattern.node {
            assert_eq!(pats.len(), 2);
            assert_eq!(pats[0].node, Pattern::Bind("x".into()));
            // Check inner tuple pattern
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
    let input = "pub async fn fetch(id: Int) -> String { id }";
    let mut parser = Parser::new(Lexer::new(input));
    let decl = parser.parse_decl().unwrap();

    if let Decl::Fn(fn_decl) = decl {
        assert_eq!(fn_decl.name, "fetch");
        assert!(fn_decl.is_async);
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
    // Function type with single effect
    assert_eq!(
        ty("() -> <io> String"),
        Type::Function {
            params: vec![],
            effects: vec![EffectItem { name: vec!["io".into()], arg: None }],
            ret: Box::new(Type::Named("String".into())),
        }
    );

    // Function type with multiple effects
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

    // Function type with parameterized effect
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

        // Check that effects are captured in the effects field
        assert_eq!(fn_decl.effects.len(), 1);
        assert_eq!(fn_decl.effects[0].name, vec!["io".to_string()]);
        assert_eq!(fn_decl.return_type, Some(Type::Named("String".to_string())));
    } else {
        panic!("Expected Function Declaration");
    }
}
