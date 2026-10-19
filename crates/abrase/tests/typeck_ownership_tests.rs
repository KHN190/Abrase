use abrase::ty::{Type, Ownership};
use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;

fn errs(src: &str) -> Vec<String> {
    let mut p = Parser::new(Lexer::new(src)).with_source(src.into());
    let ast = p.parse_program();
    assert!(p.errors.is_empty(), "parse: {:?}", p.errors);
    match Compiler::new().with_source(src.into()).compile_module(&ast) {
        Ok(_) => vec![],
        Err(es) => es.into_iter().map(|e| e.message).collect(),
    }
}

#[test]
fn repeated_mut_borrow_at_fn_top_level_ok() {
    let src = "fn push(xs: &mut Array<Int>, v: Int) -> Unit { }\nfn main() -> Unit {\n  let mut buf = [0];\n  push(&mut buf, 1);\n  push(&mut buf, 2);\n}";
    assert_eq!(errs(src), Vec::<String>::new(), "top-level repeated &mut should compile");
}

#[test]
fn repeated_mut_borrow_inside_loop_ok() {
    let src = "fn push(xs: &mut Array<Int>, v: Int) -> Unit { }\nfn main() -> Unit {\n  let mut buf = [0];\n  for i in [1,2,3] {\n    push(&mut buf, i);\n    push(&mut buf, i);\n  }\n}";
    assert_eq!(errs(src), Vec::<String>::new(), "in-loop repeated &mut should compile");
}

#[test]
fn repeated_mut_borrow_inside_if_ok() {
    let src = "fn push(xs: &mut Array<Int>, v: Int) -> Unit { }\nfn main() -> Unit {\n  let mut buf = [0];\n  if true {\n    push(&mut buf, 1);\n    push(&mut buf, 2);\n  }\n}";
    assert_eq!(errs(src), Vec::<String>::new(), "in-if repeated &mut should compile");
}

#[test]
fn repeated_mut_borrow_inside_region_ok() {
    let src = "fn push(xs: &mut Array<Int>, v: Int) -> Unit { }\nfn main() -> Unit {\n  let mut buf = [0];\n  region {\n    push(&mut buf, 1);\n    push(&mut buf, 2);\n  }\n}";
    assert_eq!(errs(src), Vec::<String>::new(), "in-region repeated &mut should compile");
}

#[test]
fn conflicting_mut_borrow_in_single_statement_still_errors_in_if() {
    let src = "fn take(a: &mut Array<Int>, b: &mut Array<Int>) -> Unit { }\nfn main() -> Unit {\n  let mut buf = [0];\n  if true {\n    take(&mut buf, &mut buf);\n  }\n}";
    assert!(!errs(src).is_empty(), "aliased &mut in one stmt must still error inside if");
}

#[test]
fn verify_ownership_derivation() {
    assert_eq!(Type::Int.ownership(), Ownership::Copy);
    assert_eq!(Type::String.ownership(), Ownership::Move);
    assert_eq!(
        Type::Tuple(vec![Type::Int, Type::Bool]).ownership(),
        Ownership::Copy
    );
    assert_eq!(
        Type::Tuple(vec![Type::Int, Type::String]).ownership(),
        Ownership::Move
    );
    assert_eq!(
        Type::Reference { is_mut: false, inner: Box::new(Type::String) }.ownership(),
        Ownership::Copy
    );
}
