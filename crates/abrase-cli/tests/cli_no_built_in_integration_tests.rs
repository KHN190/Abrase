use std::process::Command;

const ABRASE: &str = env!("CARGO_BIN_EXE_abrase");

const PURE: &str = "tests/scripts/no_builtin_pure.abe";
const USES_PRINT: &str = "tests/scripts/no_builtin_uses_print.abe";

fn run(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(ABRASE).args(args).output().expect("spawn abrase");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    (out.status.success(), stdout, stderr)
}

#[test]
fn cli_no_built_in_accepts_pure_program() {
    let (ok, stdout, stderr) = run(&["run", "--no-built-in", PURE]);
    assert!(ok, "pure program (no built-in refs) must compile under --no-built-in; stderr: {}", stderr);
    assert_eq!(stdout.trim(), "42");
}

#[test]
fn cli_default_mode_accepts_print() {
    let (ok, stdout, stderr) = run(&["run", USES_PRINT]);
    assert!(ok, "default mode runs print fine; stderr: {}", stderr);
    assert!(stdout.contains("hello"),
        "print should write 'hello'; got stdout: {:?}", stdout);
}

#[test]
fn cli_no_built_in_rejects_print() {
    let (ok, _stdout, stderr) = run(&["run", "--no-built-in", USES_PRINT]);
    assert!(!ok, "--no-built-in must reject reference to mandatory import 'print'");
    assert!(stderr.to_lowercase().contains("print") || stderr.contains("undefined") || stderr.contains("not found"),
        "expected print/undefined error, got stderr: {}", stderr);
}

#[test]
fn cli_no_built_in_check_command_rejects_print() {
    let (ok, _stdout, stderr) = run(&["check", "--no-built-in", USES_PRINT]);
    assert!(!ok, "check --no-built-in must reject reference to 'print'");
    assert!(!stderr.is_empty(), "expected stderr message, got empty");
}

#[test]
fn cli_no_built_in_disasm_shows_no_native_chunks() {
    let (ok, stdout, stderr) = run(&["disasm", "--no-built-in", PURE]);
    assert!(ok, "disasm should succeed on pure program; stderr: {}", stderr);
    assert!(!stdout.contains("<native"),
        "with --no-built-in the function table must contain no Native chunks; got disasm:\n{}", stdout);
}

#[test]
fn cli_default_mode_disasm_shows_native_chunks() {
    // Without --no-built-in, mandatory imports are registered up front and
    // show up at the top of the function table even if the source doesn't
    // touch them. Sanity check the contrast.
    let (ok, stdout, stderr) = run(&["disasm", PURE]);
    assert!(ok, "disasm should succeed; stderr: {}", stderr);
    assert!(stdout.contains("<native"),
        "default mode should register native chunks; got disasm:\n{}", stdout);
}

#[test]
fn cli_no_built_in_export_produces_native_free_cart() {
    let pid = std::process::id();
    let out = std::env::temp_dir().join(format!("abrase_no_builtin_{}.pk", pid));
    let _ = std::fs::remove_file(&out);

    let (ok, _, err) = run(&["export", "--no-built-in", PURE, out.to_str().unwrap()]);
    assert!(ok, "export --no-built-in should succeed; stderr: {}", err);

    let bytes = std::fs::read(&out).expect("read cart");
    // Header is 12 bytes; fn_count is u32 LE at offset 12.
    let fn_count = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
    // Source has `add` + `main` → 2 user fns; no natives.
    assert_eq!(fn_count, 2,
        "cart must contain exactly the two user fns and zero natives, got fn_count={}", fn_count);

    let _ = std::fs::remove_file(&out);
}

#[test]
fn cli_no_built_in_combines_with_int32() {
    // Both flags should compose without conflict.
    let (ok, stdout, stderr) = run(&["run", "--int32", "--no-built-in", PURE]);
    assert!(ok, "--int32 + --no-built-in together must work for a pure i32 program; stderr: {}", stderr);
    assert_eq!(stdout.trim(), "42");
}
