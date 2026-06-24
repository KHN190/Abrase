use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;

fn compiles(src: &str) -> bool {
    let mut p = Parser::new(Lexer::new(src)).with_source(src.into());
    let ast = p.parse_program();
    if !p.errors.is_empty() {
        return false;
    }
    let mut c = Compiler::new().with_source(src.into());
    c.compile_module(&ast).is_ok()
}

#[test]
fn core_helper_with_addr_and_peek_typechecks() {
    let src = "fn load(p: Addr) -> <core> Int { __peek64(p) }\n\
               fn main() -> Unit { () }\n";
    assert!(compiles(src), "Addr param + __peek64 declaring <core> must compile");
}

#[test]
fn arena_base_produces_addr() {
    let src = "fn origin() -> <core> Int { __peek64(__arena_base()) }\n\
               fn main() -> Unit { () }\n";
    assert!(compiles(src), "__arena_base() returns an Addr usable by peek");
}

#[test]
fn arena_base_addr_threads_through_ptr_add() {
    let src = "fn slot(n: Int) -> <core> Addr { __ptr_add(__arena_base(), n * 8) }\n\
               fn main() -> Unit { () }\n";
    assert!(compiles(src), "__arena_base() seed flows into ptr_add producing Addr");
}

#[test]
fn ptr_add_and_poke_typecheck() {
    let src = "fn store(p: Addr, v: Int) -> <core> Unit { __poke32(__ptr_add(p, 8), v) }\n\
               fn main() -> Unit { () }\n";
    assert!(compiles(src), "__ptr_add returns Addr; __poke32 takes Addr,Int");
}

#[test]
fn all_widths_typecheck() {
    let src = "fn t(p: Addr) -> <core> Int {\n\
                   __poke8(p, 1);\n\
                   __poke32(p, 2);\n\
                   __poke64(p, 3);\n\
                   __peek8(p) + __peek32(p) + __peek64(p)\n\
               }\n\
               fn main() -> Unit { () }\n";
    assert!(compiles(src), "all peek/poke widths must typecheck");
}

#[test]
fn core_fn_with_handle_param_is_rejected() {
    let src = "fn bad(s: String) -> <core> Int { 0 }\n\
               fn main() -> Unit { () }\n";
    assert!(!compiles(src), "a <core> fn taking a handle (String) must be rejected");
}

#[test]
fn core_fn_returning_handle_is_rejected() {
    let src = "fn bad(p: Addr) -> <core> String { \"x\" }\n\
               fn main() -> Unit { () }\n";
    assert!(!compiles(src), "a <core> fn returning a handle (String) must be rejected");
}

#[test]
fn core_body_string_literal_rejected() {
    let src = "fn bad(p: Addr) -> <core> Int { let s = \"x\"; __peek64(p) }\n\
               fn main() -> Unit { () }\n";
    assert!(!compiles(src), "<core> body creating a String handle must fail");
}

#[test]
fn core_body_array_rejected() {
    let src = "fn bad(p: Addr) -> <core> Int { let a = [1, 2, 3]; __peek64(p) }\n\
               fn main() -> Unit { () }\n";
    assert!(!compiles(src), "<core> body creating an array must fail");
}

#[test]
fn core_body_handle_returning_call_rejected() {
    let src = "fn bad(p: Addr) -> <core> Int { let s = __int_to_s(5); __peek64(p) }\n\
               fn main() -> Unit { () }\n";
    assert!(!compiles(src), "<core> body calling a handle-returning builtin must fail");
}

#[test]
fn core_body_pure_scalar_still_compiles() {
    let src = "fn ok(p: Addr) -> <core> Int {\n\
                   let mut x = __peek64(p);\n\
                   let mut i = 0;\n\
                   while i < 4 { x = x + __peek32(__ptr_add(p, i * 8)); i = i + 1; }\n\
                   x\n\
               }\n\
               fn main() -> Unit { () }\n";
    assert!(compiles(src), "<core> body using only Int/Addr must compile");
}

#[test]
fn plain_main_declaring_core_is_rejected() {
    let src = "fn main() -> <core> Unit { () }\n";
    assert!(!compiles(src), "a non-@cart main must be pure; <core> on main must fail");
}

#[test]
fn cart_main_declaring_core_is_rejected() {
    let src = "@cart fn main() -> <core> Unit { () }\n";
    assert!(!compiles(src), "<core> is not a runtime capability; @cart main declaring it must fail");
}
