use std::{path::PathBuf, process::Command, sync::atomic::{AtomicUsize, Ordering}};

static SEQUENCE: AtomicUsize = AtomicUsize::new(0);

struct CompileFixture(PathBuf);

impl CompileFixture {
    fn new() -> Self {
        let id = SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("myriad_dual_abi_{}_{}", std::process::id(), id));
        std::fs::create_dir_all(dir.join("src")).unwrap();
        let fixture = Self(dir);
        std::fs::write(fixture.0.join("Cargo.toml"), format!(
            "[package]\nname = \"width-contract\"\nversion = \"0.0.0\"\nedition = \"2021\"\n[workspace]\n[dependencies]\nmyriad = {{ package = \"myriad-rs\", path = {:?} }}\n",
            env!("CARGO_MANIFEST_DIR")
        )).unwrap();
        fixture
    }

    fn compile(&self, source: &str) -> std::process::Output {
        std::fs::write(self.0.join("src/main.rs"), source).unwrap();
        Command::new(env!("CARGO"))
            .args(["check", "--offline", "--quiet", "--manifest-path"])
            .arg(self.0.join("Cargo.toml"))
            .env("CARGO_TARGET_DIR", self.0.join("target"))
            .env("RUSTC_WRAPPER", "")
            .output().expect("compile width contract")
    }
}

impl Drop for CompileFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn nominal_values_and_heaps_cannot_cross_profile_boundaries() {
    let fixture = CompileFixture::new();
    let imports = "use myriad::{word32 as narrow, word64 as wide};";
    let positive = fixture.compile(&format!("{imports}
fn take32(_: narrow::Value, _: narrow::Heap) {{}}
fn take64(_: wide::Value, _: wide::Heap) {{}}
fn main() {{ take32(narrow::Value::ZERO, narrow::Heap::new()); take64(wide::Value::ZERO, wide::Heap::new()); }}"));
    assert!(positive.status.success(), "{}", String::from_utf8_lossy(&positive.stderr));
    for body in [
        "fn takes32(_: narrow::Value) {} fn main() { takes32(wide::Value::ZERO); }",
        "fn takes32(_: narrow::Heap) {} fn main() { takes32(wide::Heap::new()); }",
    ] {
        let output = fixture.compile(&format!("{imports}\n{body}"));
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success());
        assert!(error.contains("E0308"), "expected nominal type mismatch, got {error}");
    }
}
