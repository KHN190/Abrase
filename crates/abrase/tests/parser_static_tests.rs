use abrase::ast::Decl;
use abrase::lexer::Lexer;
use abrase::parser::Parser;

fn parse_one(src: &str) -> Decl {
    let mut p = Parser::new(Lexer::new(src));
    p.parse_decl().unwrap()
}

#[test]
fn parses_immutable_static() {
    let Decl::Static { is_pub, name, ty, .. } = parse_one("static MAX: Int = 100;") else {
        panic!("expected Decl::Static");
    };
    assert!(!is_pub);
    assert_eq!(name, "MAX");
    assert_eq!(ty, abrase::ast::Type::Named("Int".into()));
}

#[test]
fn static_value_expression_is_kept() {
    let Decl::Static { value, .. } = parse_one("static FRAME: Int = 1 + 2;") else {
        panic!("expected Decl::Static");
    };
    assert!(matches!(value.node, abrase::ast::Expr::Binary { .. }));
}

#[test]
fn static_mixes_with_other_decls_in_a_program() {
    let src = "static N: Int = 0;\nfn main() -> Unit { () }\n";
    let mut p = Parser::new(Lexer::new(src)).with_source(src.into());
    let decls = p.parse_program();
    assert!(p.errors.is_empty(), "parse errors: {:?}", p.errors);
    assert_eq!(decls.len(), 2);
    assert!(matches!(decls[0], Decl::Static { .. }));
}
