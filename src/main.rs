use ect::lexer::Lexer;
use ect::parser::Parser;

fn main() {
    let source_code = r#"
        pub async fn calculate(a: Int, b: Int) -> Int {
            let mut result = a + b;
            result
        }
    "#;

    println!("--- Source Code ---\n{}", source_code);

    let lexer = Lexer::new(source_code);
    let mut parser = Parser::new(lexer);

    let ast = parser.parse_program();

    if !parser.errors.is_empty() {
        println!("--- Parse Errors ({}) ---", parser.errors.len());
        for err in &parser.errors {
            println!("[Line {}:{}] {}", err.span.line, err.span.col, err.message);
        }
    } else {
        println!("--- AST Generated Successfully ---");
        println!("{:#?}", ast);
    }
}