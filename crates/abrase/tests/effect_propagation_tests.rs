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

fn warnings(src: &str) -> Vec<String> {
    let mut p = Parser::new(Lexer::new(src)).with_source(src.into());
    let ast = p.parse_program();
    assert!(p.errors.is_empty(), "parse: {:?}", p.errors);
    let mut c = Compiler::new().with_source(src.into());
    let _ = c.compile_module(&ast);
    c.warnings.iter().map(|w| w.message.clone()).collect()
}

fn uses_effect_error(e: &[String]) -> bool {
    e.iter().any(|m| m.contains("does not declare it"))
}

fn unused_effect_warn(w: &[String]) -> bool {
    w.iter().any(|m| m.contains("declares effect") && m.contains("never uses"))
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

// `alloc` is no longer a recognized effect (heap is a runtime property, not a
// surface capability): it carries no obligation.
#[test]
fn alloc_is_not_a_recognized_effect() {
    let e = errors("pub fn r() -> <alloc> Int { 0 }\npub fn c() -> Int { r() }\nfn main() -> Int { 0 }");
    assert!(!uses_effect_error(&e), "alloc carries nothing, caller needs no declaration: {:?}", e);
}

// Declaring `alloc` contributes nothing, so it cannot satisfy a required `io`.
#[test]
fn alloc_declaration_does_not_cover_io() {
    let e = errors("pub fn r() -> <io> Int { 0 }\npub fn c() -> <alloc> Int { r() }\nfn main() -> Int { 0 }");
    assert!(uses_effect_error(&e), "alloc must not cover io: {:?}", e);
}

// --- unused-effect (over-declare) warning ---

#[test]
fn unused_io_declaration_warns() {
    let w = warnings("pub fn f() -> <io> Int { 42 }\nfn main() -> Int { 0 }");
    assert!(unused_effect_warn(&w), "declared-unused io must warn: {:?}", w);
}

#[test]
fn unused_nondet_declaration_warns() {
    let w = warnings("pub fn f() -> <nondet> Int { 42 }\nfn main() -> Int { 0 }");
    assert!(unused_effect_warn(&w), "declared-unused nondet must warn: {:?}", w);
}

#[test]
fn used_io_does_not_warn() {
    let w = warnings("pub fn r() -> <io> Int { 0 }\npub fn f() -> <io> Int { r() }\nfn main() -> Int { 0 }");
    assert!(!w.iter().any(|m| m.contains("'f'") && m.contains("never uses")),
        "caller f uses io (via r), must not warn: {:?}", w);
}

#[test]
fn declared_alloc_never_warns() {
    let w = warnings("pub fn f() -> <alloc> Int { 42 }\nfn main() -> Int { 0 }");
    assert!(!unused_effect_warn(&w), "alloc is ambient, declaring it must not warn: {:?}", w);
}

// --- native effect coverage ---

// frame: callable host capability (effect-op), propagates.
#[test]
fn frame_use_without_declaration_errors() {
    let e = errors("pub fn tick() -> Unit { frame.present() }\nfn main() -> Int { 0 }");
    assert!(uses_effect_error(&e), "undeclared frame use must error: {:?}", e);
}

#[test]
fn frame_use_with_declaration_compiles() {
    let e = errors("pub fn tick() -> <frame> Unit { frame.present() }\nfn main() -> Int { 0 }");
    assert!(!uses_effect_error(&e), "declared frame use must compile: {:?}", e);
}

// Host-provided natives (now/rand live here in real hosts, e.g. posara) carry
// their effect via `register_host_fn` and propagate through the same fn-type
// path as user functions. Registering a fake io native proves it end to end.
fn errors_with_io_native(src: &str) -> Vec<String> {
    use abrase::ty::Type;
    let mut p = Parser::new(Lexer::new(src)).with_source(src.into());
    let ast = p.parse_program();
    assert!(p.errors.is_empty(), "parse: {:?}", p.errors);
    let mut c = Compiler::new().with_source(src.into());
    c.register_host_fn("io_native", vec![], Type::Int,
        vec![abrase::ast::EffectItem { name: vec!["io".into()], arg: None }])
        .expect("register host fn");
    match c.compile_module(&ast) {
        Ok(_) => vec![],
        Err(es) => es.into_iter().map(|e| e.message).collect(),
    }
}

#[test]
fn host_io_native_undeclared_errors() {
    let e = errors_with_io_native("pub fn t() -> Int { io_native() }\nfn main() -> Int { 0 }");
    assert!(uses_effect_error(&e), "undeclared host io native must error: {:?}", e);
}

#[test]
fn host_io_native_declared_compiles() {
    let e = errors_with_io_native("pub fn t() -> <io> Int { io_native() }\nfn main() -> Int { 0 }");
    assert!(!uses_effect_error(&e), "declared host io native must compile: {:?}", e);
}
