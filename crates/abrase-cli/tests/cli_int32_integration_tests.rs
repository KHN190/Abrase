use std::process::Command;

const ABRASE: &str = env!("CARGO_BIN_EXE_abrase");

const IN_RANGE: &str = "tests/scripts/int32_in_range.abe";
const OUT_OF_RANGE: &str = "tests/scripts/int32_out_of_range.abe";

fn run(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(ABRASE).args(args).output().expect("spawn abrase");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    (out.status.success(), stdout, stderr)
}

#[test]
fn cli_default_mode_runs_i32_range_program() {
    let (ok, stdout, stderr) = run(&["run", IN_RANGE]);
    assert!(ok, "default mode should run a program with i32-range literals; stderr: {}", stderr);
    assert_eq!(stdout.trim(), "-1", "2147483647 + (-2147483648) = -1; got: {:?}", stdout);
}

#[test]
fn cli_int32_mode_accepts_i32_range_program() {
    let (ok, stdout, stderr) = run(&["run", "--int32", IN_RANGE]);
    assert!(ok, "--int32 should accept program whose literals fit i32; stderr: {}", stderr);
    assert_eq!(stdout.trim(), "-1");
}

#[test]
fn cli_default_mode_accepts_i64_literal_beyond_i32() {
    let (ok, stdout, stderr) = run(&["run", OUT_OF_RANGE]);
    assert!(ok, "default mode must accept i64 literals beyond i32 range; stderr: {}", stderr);
    assert_eq!(stdout.trim(), "2147483648");
}

#[test]
fn cli_int32_mode_rejects_literal_beyond_i32() {
    let (ok, _stdout, stderr) = run(&["run", "--int32", OUT_OF_RANGE]);
    assert!(!ok, "--int32 must reject literal > i32::MAX");
    assert!(stderr.contains("out of i32 range"),
        "expected i32 range error, got stderr: {}", stderr);
}

#[test]
fn cli_int32_mode_check_command_rejects_out_of_range() {
    let (ok, _stdout, stderr) = run(&["check", "--int32", OUT_OF_RANGE]);
    assert!(!ok, "check --int32 must reject out-of-range literal");
    assert!(stderr.contains("out of i32 range"),
        "expected i32 range error from check, got stderr: {}", stderr);
}

#[test]
fn cli_export_int32_flag_sets_cart_header_bit() {
    let pid = std::process::id();
    let dir = std::env::temp_dir();
    let out_int32 = dir.join(format!("abrase_int32_flag_set_{}.pk", pid));
    let out_default = dir.join(format!("abrase_int32_flag_clear_{}.pk", pid));

    let _ = std::fs::remove_file(&out_int32);
    let _ = std::fs::remove_file(&out_default);

    let (ok1, _, err1) = run(&["export", "--int32", IN_RANGE, out_int32.to_str().unwrap()]);
    assert!(ok1, "export --int32 should succeed; stderr: {}", err1);

    let (ok2, _, err2) = run(&["export", IN_RANGE, out_default.to_str().unwrap()]);
    assert!(ok2, "export default should succeed; stderr: {}", err2);

    let bytes_int32 = std::fs::read(&out_int32).expect("read int32 cart");
    let bytes_default = std::fs::read(&out_default).expect("read default cart");

    // Header layout: magic[0..4], version[4..6], flags[6..8].
    let flags_int32 = u16::from_le_bytes([bytes_int32[6], bytes_int32[7]]);
    let flags_default = u16::from_le_bytes([bytes_default[6], bytes_default[7]]);

    assert_eq!(flags_int32 & 0x0001, 0x0001,
        "INT32_SAFE bit must be set with --int32");
    assert_eq!(flags_default & 0x0001, 0x0000,
        "INT32_SAFE bit must be clear without --int32");

    let _ = std::fs::remove_file(&out_int32);
    let _ = std::fs::remove_file(&out_default);
}
