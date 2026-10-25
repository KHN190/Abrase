pub const CORE: &str = include_str!("../fixtures/core_heap.abe");

pub fn source32() -> String {
    let source = replace_section(
        CORE,
        "codec",
        include_str!("../fixtures/core_heap32_codec.abe"),
    );
    let source = replace_section(
        &source,
        "init",
        include_str!("../fixtures/core_heap32_init.abe"),
    );
    let source = replace_section(
        &source,
        "word",
        "fn load_word(addr: Addr) -> <core> Int { __peek32(addr) }\nfn store_word(addr: Addr, value: Int) -> <core> Unit { __poke32(addr, value) }\n",
    );
    assert_eq!(source.matches("const WORD_BYTES: Int = 8;").count(), 1);
    assert_eq!(source.matches("const WORD_BITS: Int = 64;").count(), 1);
    source
        .replace("const WORD_BYTES: Int = 8;", "const WORD_BYTES: Int = 4;")
        .replace("const WORD_BITS: Int = 64;", "const WORD_BITS: Int = 32;")
}

fn replace_section(source: &str, name: &str, replacement: &str) -> String {
    let begin = format!("// @{name}:start\n");
    let end = format!("// @{name}:end");
    assert_eq!(source.matches(&begin).count(), 1);
    assert_eq!(source.matches(&end).count(), 1);
    let start = source.find(&begin).unwrap();
    let finish = source.find(&end).unwrap() + end.len();
    assert!(start < finish);
    format!("{}{}{}", &source[..start], replacement, &source[finish..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expansion_uses_existing_word32_intrinsics_only() {
        let source = source32();
        assert!(!source.contains("__peek64"));
        assert!(!source.contains("__poke64"));
        assert!(!source.contains("// @"));
    }

    #[test]
    #[should_panic]
    fn expansion_rejects_missing_section() {
        replace_section("", "codec", "");
    }

    #[test]
    #[should_panic]
    fn expansion_rejects_duplicate_section() {
        replace_section(
            "// @codec:start\n// @codec:start\n// @codec:end",
            "codec",
            "",
        );
    }
}
