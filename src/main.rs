use ect::ast::{Pattern, Spanned};
use ect::lexer::{Lexer, Token};
use ect::parser::Parser;
use ect::typeck::Checker;
use ect::ast;

fn main() {
    run_lexer();
    run_parser();
    run_typeck();
}

fn run_lexer() {
    println!("═══════════════════════════════════");
    println!(" LEXER");
    println!("═══════════════════════════════════");

    lex_show("keywords",
        "fn let const mut pub async await true false self Self _ where");

    lex_show("operators",
        "+ - * / % == != < <= > >= && || ! = += -= *= /= %= -> => .. ..= ?");

    lex_show("literals",
        r#"42  3.14  3.14e-2  true  'a'  '\n'  '\u{1F600}'  "hello"  ()"#);

    lex_show("string interpolation",
        r#""hello {name}, you are {user.age} years old""#);

    lex_show("comment skipping",
        "let x = 1; // this is ignored\nlet y = 2;");
}

fn lex_show(label: &str, input: &str) {
    print!("\n[{}]\n  input : {}\n  tokens:", label, input);
    let mut lexer = Lexer::new(input);
    loop {
        let (tok, _span) = lexer.next_token();
        let is_eof = tok == Token::Eof;
        print!(" {:?}", tok);
        if is_eof { break; }
    }
    println!();
}

fn run_parser() {
    println!("\n═══════════════════════════════════");
    println!(" PARSER");
    println!("═══════════════════════════════════");

    parse_show("basic function", r#"
        pub fn add(x: Int, y: Int) -> Int {
            x + y
        }
    "#);

    parse_show("async function with let and call", r#"
        async fn fetch(id: Int) -> String {
            let result = get_item(id);
            result
        }
    "#);

    parse_show("nested arithmetic precedence", r#"
        fn calc(a: Int, b: Int, c: Int) -> Int {
            a + b * c
        }
    "#);

    parse_show("self param and field access", r#"
        fn name(self: &Self) -> String {
            self.name
        }
    "#);

    parse_show("await chain", r#"
        async fn load(url: String) -> String {
            let resp = fetch(url).await;
            resp
        }
    "#);
}

fn parse_show(label: &str, input: &str) {
    let mut parser = Parser::new(Lexer::new(input));
    let decls = parser.parse_program();
    print!("\n[{}]", label);
    if parser.errors.is_empty() {
        println!(" ✓  ({} decl(s))", decls.len());
        for d in &decls {
            println!("  {:#?}", d);
        }
    } else {
        println!(" ✗  ({} error(s))", parser.errors.len());
        for e in &parser.errors {
            println!("  [{}:{}] {}", e.span.line, e.span.col, e.message);
        }
    }
}

fn run_typeck() {
    println!("\n═══════════════════════════════════");
    println!(" TYPE CHECKER");
    println!("═══════════════════════════════════");

    typeck_show("primitive inference", |checker| {
        use ast::Expr::Literal as Lit;
        use ast::Literal::*;
        println!("  Int(42)    → {:?}", checker.infer_expr(&sp(Lit(Int(42)))));
        println!("  Float(3.14)→ {:?}", checker.infer_expr(&sp(Lit(Float(3.14)))));
        println!("  Bool(true) → {:?}", checker.infer_expr(&sp(Lit(Bool(true)))));
        println!("  Char('a')  → {:?}", checker.infer_expr(&sp(Lit(Char('a')))));
        println!("  String(..) → {:?}", checker.infer_expr(&sp(Lit(String("hi".into())))));
        println!("  Unit       → {:?}", checker.infer_expr(&sp(Lit(Unit))));
    });

    typeck_show("arithmetic type checking", |checker| {
        let ok = sp(ast::Expr::Binary {
            op: ast::BinaryOp::Add,
            left:  Box::new(sp(ast::Expr::Literal(ast::Literal::Int(1)))),
            right: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(2)))),
        });
        println!("  Int + Int  → {:?}", checker.infer_expr(&ok));

        let bad = sp(ast::Expr::Binary {
            op: ast::BinaryOp::Add,
            left:  Box::new(sp(ast::Expr::Literal(ast::Literal::Int(1)))),
            right: Box::new(sp(ast::Expr::Literal(ast::Literal::Bool(true)))),
        });
        checker.infer_expr(&bad);
        println!("  Int + Bool → error: \"{}\"", checker.errors[0].message);
    });

    typeck_show("move semantics: String is @move", |checker| {
        let d_span = ect::ast::Span::new(0, 0);
        checker.insert_var("s".into(), ect::ty::Type::String, false, d_span);

        // first use: moves
        let use1 = sp(ast::Expr::Identifier("s".into()));
        println!("  use s (1st) → {:?}", checker.infer_expr(&use1));

        // second use: error
        let use2 = sp(ast::Expr::Identifier("s".into()));
        checker.infer_expr(&use2);
        println!("  use s (2nd) → error: \"{}\"", checker.errors[0].message);
    });

    typeck_show("copy semantics: Int is @copy", |checker| {
        let d_span = ect::ast::Span::new(0, 0);
        checker.insert_var("n".into(), ect::ty::Type::Int, false, d_span);

        let use1 = sp(ast::Expr::Identifier("n".into()));
        let use2 = sp(ast::Expr::Identifier("n".into()));
        println!("  use n (1st) → {:?}", checker.infer_expr(&use1));
        println!("  use n (2nd) → {:?} (copy: no error)", checker.infer_expr(&use2));
    });

    typeck_show("if branch type mismatch", |checker| {
        let expr = sp(ast::Expr::If {
            condition:   Box::new(sp(ast::Expr::Literal(ast::Literal::Bool(true)))),
            consequence: Box::new(sp(ast::Expr::Literal(ast::Literal::Int(1)))),
            alternative: Some(Box::new(sp(ast::Expr::Literal(ast::Literal::String("nope".into()))))),
        });
        checker.infer_expr(&expr);
        println!("  if true {{ 1 }} else {{ \"nope\" }} → error: \"{}\"",
            checker.errors[0].message);
    });

    typeck_show("let binding with type annotation", |checker| {
        let stmt = sp(ast::Stmt::Let {
            pattern: sp(Pattern::Bind("x".into())),
            is_mut: false,
            ty: Some(ast::Type::Named("Int".into())),
            value: sp(ast::Expr::Literal(ast::Literal::Int(99))),
        });
        checker.check_stmt(&stmt);
        println!("  let x: Int = 99 → {} error(s)", checker.errors.len());

        let stmt2 = sp(ast::Stmt::Let {
            pattern: sp(Pattern::Bind("y".into())),
            is_mut: false,
            ty: Some(ast::Type::Named("Bool".into())),
            value: sp(ast::Expr::Literal(ast::Literal::Int(0))),
        });
        checker.check_stmt(&stmt2);
        println!("  let y: Bool = 0 → error: \"{}\"", checker.errors[0].message);
    });
}

fn sp<T>(node: T) -> Spanned<T> {
    Spanned { node, span: ast::Span::new(0, 0) }
}

fn typeck_show<F: FnOnce(&mut Checker)>(label: &str, f: F) {
    println!("\n[{}]", label);
    let mut checker = Checker::new();
    f(&mut checker);
}
