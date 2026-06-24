use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;

const CORE: &str = include_str!("fixtures/core_heap.abe");
const GEN_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../myriad/src/core_heap_gen.rs");

fn generate() -> String {
    let mut p = Parser::new(Lexer::new(CORE)).with_source(CORE.into());
    let ast = p.parse_program();
    assert!(p.errors.is_empty(), "parse: {:?}", p.errors);
    let mut c = Compiler::new().with_source(CORE.into()).with_lib(true);
    let m = c.compile_module(&ast).unwrap_or_else(|e| {
        panic!("compile: {:?}", e.iter().map(|x| &x.message).collect::<Vec<_>>())
    });
    let body = polka_rustc::core::transpile_core(&m).expect("transpile_core");
    format!(
        "// @generated from crates/abrase-cli/tests/fixtures/core_heap.abe — do not edit.\n\
         // Trust archive: the abrase-written heap, AOT-compiled to no_std Rust.\n\
         // Regenerate: `cargo test -p abrase-cli --test core_gen_tests regenerate -- --ignored`.\n\
         #![allow(dead_code, unused, unused_parens)]\n{}",
        body,
    )
}

#[test]
#[ignore = "writes the vendored trust archive; run explicitly to regenerate"]
fn regenerate() {
    std::fs::write(GEN_PATH, generate()).expect("write gen file");
}

#[test]
fn vendored_core_gen_is_in_sync() {
    let current = std::fs::read_to_string(GEN_PATH).unwrap_or_default();
    assert_eq!(current, generate(),
        "myriad/src/core_heap_gen.rs is stale — run `cargo test -p abrase-cli --test core_gen_tests regenerate -- --ignored`");
}
