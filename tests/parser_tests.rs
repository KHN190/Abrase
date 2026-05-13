use ect::ast::*;
use ect::lexer::Lexer;
use ect::parser::{Parser, Precedence};

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
