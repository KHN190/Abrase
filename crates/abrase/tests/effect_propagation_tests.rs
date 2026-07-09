use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;

fn errors(src: &str) -> Vec<String> {
    let mut p = Parser::new(Lexer::new(src)).with_source(src.into());
    let ast = p.parse_program();
    assert!(p.errors.is_empty(), "parse: {:?}", p.errors);
    match Compiler::new().with_source(src.into()).compile_module(&ast) {
        Ok(_) => vec![],
        Err(es) => es.into_iter().map(|e| e.message).collect(),
    }
}

fn uses_effect_error(e: &[String]) -> bool {
    e.iter().any(|m| m.contains("does not declare it"))
}

// println carries no effect row: pure everywhere, including pure main.
#[test]
fn println_is_pure_in_main() {
    assert!(errors("fn main() -> Unit { println(\"x\") }").is_empty());
}

#[test]
fn println_is_pure_in_helper() {
    assert!(errors("pub fn f() -> Unit { println(\"x\") }\nfn main() -> Int { 0 }").is_empty());
}

// `io` is a tracked capability: propagates to callers, must be declared.
#[test]
fn io_is_tracked_and_must_be_declared() {
    let e = errors("pub fn r() -> <io> Int { 0 }\npub fn c() -> Int { r() }\nfn main() -> Int { 0 }");
    assert!(uses_effect_error(&e), "undeclared io must error: {:?}", e);
}

#[test]
fn io_declared_compiles() {
    let e = errors("pub fn r() -> <io> Int { 0 }\npub fn c() -> <io> Int { r() }\nfn main() -> Int { 0 }");
    assert!(!uses_effect_error(&e), "declared io must compile: {:?}", e);
}

// `alloc` is ambient (heap is a runtime property, not a capability): never
// requires declaration.
#[test]
fn alloc_is_ambient_and_needs_no_declaration() {
    let e = errors("pub fn r() -> <alloc> Int { 0 }\npub fn c() -> Int { r() }\nfn main() -> Int { 0 }");
    assert!(!uses_effect_error(&e), "alloc is ambient, undeclared caller must compile: {:?}", e);
}

// The split is real: declaring `alloc` does not satisfy a required `io`.
#[test]
fn alloc_declaration_does_not_cover_io() {
    let e = errors("pub fn r() -> <io> Int { 0 }\npub fn c() -> <alloc> Int { r() }\nfn main() -> Int { 0 }");
    assert!(uses_effect_error(&e), "alloc must not cover io: {:?}", e);
}
