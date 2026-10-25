use std::{path::PathBuf, process::Command, sync::atomic::{AtomicUsize, Ordering}};
static SEQUENCE: AtomicUsize = AtomicUsize::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new(source: &str) -> Self {
        let path = std::env::temp_dir().join(format!("abrase_cli_profiles_{}_{}", std::process::id(), SEQUENCE.fetch_add(1, Ordering::Relaxed)));
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join("main.abe"), source).unwrap(); Self(path)
    }
    fn invoke(&self, args: &[&str]) -> String {
        let output = Command::new(env!("CARGO_BIN_EXE_abrase")).current_dir(&self.0).args(args).output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    }
}
impl Drop for Fixture { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }

#[test]
fn run_int32_uses_i32_arithmetic_while_default_keeps_i64() {
    let f = Fixture::new("fn id(x: Int) -> Int { x } fn main() -> Int { id(2147483647) + id(1) }");
    assert_eq!(f.invoke(&["run", "--int32", "main.abe"]), "-2147483648");
    assert_eq!(f.invoke(&["run", "main.abe"]), "2147483648");
}

#[test]
fn load_selects_storage_from_the_cart_header_without_a_runtime_flag() {
    let f = Fixture::new("fn id(x: Int) -> Int { x } fn main() -> Int { id(2147483647) + id(1) }");
    f.invoke(&["export", "--int32", "main.abe", "narrow.pk"]);
    f.invoke(&["export", "main.abe", "wide.pk"]);
    assert_eq!(f.invoke(&["load", "narrow.pk"]), "-2147483648");
    assert_eq!(f.invoke(&["load", "wide.pk"]), "2147483648");
}

#[test]
fn float32_runtime_results_feed_the_real_i32_native_abi() {
    let f = Fixture::new("fn id(x: Float) -> Float { x } fn main() -> Int { (id(16777216.0) + id(1.0)).to_i() }");
    assert_eq!(f.invoke(&["run", "--int32", "main.abe"]), "16777216");
    assert_eq!(f.invoke(&["run", "main.abe"]), "16777217");
}
