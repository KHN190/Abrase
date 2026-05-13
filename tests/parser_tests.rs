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
    if let Expr::If { condition, consequence, alternative } = expr.node {
        assert_eq!(condition.node, Expr::Literal(Literal::Bool(true)));
        assert!(alternative.is_some());
    } else {
        panic!("Expected If expression");
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
