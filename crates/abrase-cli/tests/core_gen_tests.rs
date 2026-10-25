use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;

#[path = "support/core_heap_source.rs"]
mod source;
const GEN_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../myriad/src/core_heap_gen.rs"
);

const GEN32_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../myriad/src/core_heap32_gen.rs"
);

fn generate(word32: bool) -> String {
    let core = if word32 {
        source::source32()
    } else {
        source::CORE.into()
    };
    let core = core.as_str();
    let mut p = Parser::new(Lexer::new(core)).with_source(core.into());
    let ast = p.parse_program();
    assert!(p.errors.is_empty(), "parse: {:?}", p.errors);
    let mut c = Compiler::new()
        .with_source(core.into())
        .with_lib(true)
        .with_int32_mode(word32);
    let m = c.compile_module(&ast).unwrap_or_else(|e| {
        panic!(
            "compile: {:?}",
            e.iter().map(|x| &x.message).collect::<Vec<_>>()
        )
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
    std::fs::write(GEN_PATH, generate(false)).expect("write gen file");
    std::fs::write(GEN32_PATH, generate(true)).expect("write gen32 file");
}

#[test]
fn vendored_core_gen_is_in_sync() {
    let current = std::fs::read_to_string(GEN_PATH).unwrap_or_default();
    assert_eq!(
        current,
        generate(false),
        "myriad/src/core_heap_gen.rs is stale — run `cargo test -p abrase-cli --test core_gen_tests regenerate -- --ignored`"
    );
}

#[test]
fn vendored_core32_gen_is_in_sync() {
    let current = std::fs::read_to_string(GEN32_PATH).unwrap_or_default();
    assert!(current == generate(true), "core_heap32_gen.rs is stale");
}
