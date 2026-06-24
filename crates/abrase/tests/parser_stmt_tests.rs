use abrase::ast::*;
use abrase::lexer::Lexer;
use abrase::parser::Parser;

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

// An expr-with-block (if/while/match) used as a statement needs no trailing `;`
// (BNF 09). A following statement that starts with an identifier — assignment or
// call — must not be misread as a continuation of the block expression.
fn parse_ok(src: &str) -> Vec<String> {
    let mut p = Parser::new(Lexer::new(src)).with_source(src.into());
    let _ = p.parse_program();
    p.errors.iter().map(|e| e.message.clone()).collect()
}

#[test]
fn if_else_statement_followed_by_assignment_no_semicolon() {
    let errs = parse_ok("fn f() -> Int { let mut a = 0; if a == 0 { a = 1; } else { a = 2; } a = a + 1; a }");
    assert!(errs.is_empty(), "if-else then assignment must parse: {:?}", errs);
}

#[test]
fn if_else_statement_followed_by_call_no_semicolon() {
    let errs = parse_ok("fn g() -> Unit { () }\nfn f() -> Unit { if true { () } else { () } g() }");
    assert!(errs.is_empty(), "if-else then call must parse: {:?}", errs);
}

#[test]
fn if_else_statement_followed_by_literal_or_let_still_ok() {
    assert!(parse_ok("fn f() -> Int { if true { 1 } else { 2 } 3 }").is_empty());
    assert!(parse_ok("fn f() -> Int { if true { 1 } else { 2 } let x = 3; x }").is_empty());
}
