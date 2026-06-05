use abrase_wasm::{check_with, run, run_with, Session};
use std::fs;
use std::path::Path;

fn read(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {}", path, e))
}

fn collect_scripts(dir: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {}", dir, e)) {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) == Some("abe") {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            out.push((name, fs::read_to_string(&path).unwrap()));
        }
    }
    assert!(!out.is_empty(), "no .abe scripts found in {}", dir);
    out.sort();
    out
}

const EXAMPLES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples");
const SCRIPTS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../abrase-cli/tests/scripts");

#[test]
fn all_example_scripts_run_to_completion() {
    for (name, src) in collect_scripts(EXAMPLES) {
        if name == "frame_counter.abe" {
            continue;
        }
        let r = run(&src);
        assert!(r.ok, "{} failed:\n{}", name, r.error);
    }
}

#[test]
fn all_cli_integration_scripts_run_to_completion() {
    let special = [
        "int32_in_range.abe",
        "int32_out_of_range.abe",
        "no_builtin_pure.abe",
        "no_builtin_uses_print.abe",
        "effect_handlers.abe",
    ];
    for (name, src) in collect_scripts(SCRIPTS) {
        if special.contains(&name.as_str()) {
            continue;
        }
        let r = run(&src);
        assert!(r.ok, "{} failed:\n{}", name, r.error);
    }
}

#[test]
fn effect_handlers_typecheck_only_fixture_passes_check() {
    let r = abrase_wasm::check(&read(&format!("{}/effect_handlers.abe", SCRIPTS)));
    assert!(r.ok, "{}", r.error);
}

#[test]
fn ackermann_example_prints_expected_value() {
    let r = run(&read(&format!("{}/ackermann.abe", EXAMPLES)));
    assert!(r.ok, "{}", r.error);
    assert!(r.stdout.contains("61"), "ack(3,3)=61, stdout = {:?}", r.stdout);
}

#[test]
fn running_sum_example_folds_via_deep_handler() {
    let r = run(&read(&format!("{}/running_sum.abe", EXAMPLES)));
    assert!(r.ok, "{}", r.error);
    assert!(r.stdout.contains("18"), "stdout = {:?}", r.stdout);
}

#[test]
fn interp_script_returns_string_value() {
    let r = run(&read(&format!("{}/interp.abe", SCRIPTS)));
    assert!(r.ok, "{}", r.error);
    assert!(!r.value.is_empty(), "expected decoded String return value");
}

#[test]
fn built_ins_script_stdout_is_captured() {
    let r = run(&read(&format!("{}/built_ins.abe", SCRIPTS)));
    assert!(r.ok, "{}", r.error);
    assert!(r.stdout.contains("hello, myriad"), "stdout = {:?}", r.stdout);
}

#[test]
fn int32_mode_accepts_in_range_program() {
    let r = run_with(&read(&format!("{}/int32_in_range.abe", SCRIPTS)), true, false);
    assert!(r.ok, "{}", r.error);
}

#[test]
fn int32_mode_rejects_out_of_range_literal() {
    let r = run_with(&read(&format!("{}/int32_out_of_range.abe", SCRIPTS)), true, false);
    assert!(!r.ok);
    assert!(r.error.contains("out of i32 range"), "error = {:?}", r.error);
}

#[test]
fn default_mode_accepts_i64_literal_beyond_i32() {
    let r = run(&read(&format!("{}/int32_out_of_range.abe", SCRIPTS)));
    assert!(r.ok, "{}", r.error);
}

#[test]
fn no_built_in_accepts_pure_program() {
    let r = run_with(&read(&format!("{}/no_builtin_pure.abe", SCRIPTS)), false, true);
    assert!(r.ok, "{}", r.error);
}

#[test]
fn no_built_in_rejects_print() {
    let r = run_with(&read(&format!("{}/no_builtin_uses_print.abe", SCRIPTS)), false, true);
    assert!(!r.ok);
}

#[test]
fn check_no_built_in_rejects_print() {
    let r = check_with(&read(&format!("{}/no_builtin_uses_print.abe", SCRIPTS)), false, true);
    assert!(!r.ok);
}

#[test]
fn cart_session_steps_frames_and_heap_stays_flat() {
    let path = Path::new(EXAMPLES).join("frame_counter.abe");
    let src = fs::read_to_string(&path).unwrap();
    let mut session = Session::new(&src).unwrap_or_else(|e| panic!("{}", e));
    let live0 = session.heap_live_count();
    let mut alive = true;
    let mut frames = 1;
    while alive {
        assert!(frames < 100, "frame_counter should halt within 5 frames");
        alive = session.resume(0).unwrap_or_else(|e| panic!("frame {}: {}", frames, e));
        frames += 1;
    }
    let out = session.take_stdout();
    assert!(out.contains("15"), "total after 5 frames = 15, stdout = {:?}", out);
    assert!(session.heap_live_count() <= live0, "heap grew across frames");
}

#[test]
fn parse_error_is_reported_not_panicked() {
    let r = run("fn main( {");
    assert!(!r.ok);
    assert!(!r.error.is_empty());
}

#[test]
fn runtime_error_carries_partial_stdout() {
    let src = r#"
fn main() -> Int {
  println("before");
  let z = 0;
  1 / z
}
"#;
    let r = run(src);
    assert!(!r.ok);
    assert!(r.error.contains("runtime error"), "error = {:?}", r.error);
    assert!(r.stdout.contains("before"), "stdout = {:?}", r.stdout);
}

#[test]
fn consecutive_runs_are_independent() {
    let src = &read(&format!("{}/region.abe", SCRIPTS));
    let a = run(src);
    let b = run(src);
    assert!(a.ok && b.ok, "{} / {}", a.error, b.error);
    assert_eq!(a.value, b.value);
    assert_eq!(a.stdout, b.stdout);
}
