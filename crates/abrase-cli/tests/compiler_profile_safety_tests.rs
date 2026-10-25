use abrase::{compiler::Compiler, lexer::Lexer, parser::Parser};
use myriad::{word32, word64};

fn execute(source: &str, narrow: bool, disabled: usize) -> (i64, usize) {
    let mut parser = Parser::new(Lexer::new(source)).with_source(source.into());
    let ast = parser.parse_program();
    assert!(parser.errors.is_empty(), "{}", parser.pretty_print_errors());
    let mut compiler = Compiler::new().with_source(source.into()).with_int32_mode(narrow)
        .with_inline(disabled != 1).with_drop_elision(disabled != 2)
        .with_copy_coalesce(disabled != 3).with_tail_resume(disabled != 4);
    let module = compiler.compile_module(&ast).unwrap_or_else(|_| panic!("{}", compiler.pretty_print_errors()));
    macro_rules! run {
        ($profile:ident) => {{
            let mut vm = $profile::VirtualMachine::new().with_heap_check(true).with_step_cap(500_000);
            $profile::Host::default().install_into(&mut vm);
            let value = vm.run_module(&module).unwrap();
            assert!(!vm.last_result_is_handle());
            (value.as_int() as i64, vm.heap_live_count())
        }};
    }
    if narrow { run!(word32) } else { run!(word64) }
}

#[test]
fn existing_closure_effect_and_mutable_capture_fixtures_stay_clean_in_both_profiles() {
    for source in [
        include_str!("scripts/closures_complex.abe"),
        include_str!("scripts/effect_dispatch.abe"),
        include_str!("scripts/effect_resume_paths.abe"),
        include_str!("scripts/handler_let_mut_cell.abe"),
    ] {
        let expected = execute(source, false, 0);
        assert_eq!(expected.1, 0);
        for narrow in [false, true] {
            for disabled in 0..=4 {
                assert_eq!(execute(source, narrow, disabled), expected, "narrow={narrow}, disabled={disabled}");
            }
        }
    }
}

#[test]
fn multi_shot_fixture_is_rejected_in_both_profiles() {
    let source = include_str!("scripts/effect_handlers.abe");
    for narrow in [false, true] {
        let mut parser = Parser::new(Lexer::new(source));
        let ast = parser.parse_program();
        assert!(parser.errors.is_empty());
        let mut compiler = Compiler::new().with_int32_mode(narrow);
        assert!(compiler.compile_module(&ast).is_err());
        assert!(compiler.pretty_print_errors().contains("multi-shot resume"));
    }
}
