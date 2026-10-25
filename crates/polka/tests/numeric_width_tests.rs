use core::mem::size_of;
use polka::numeric::{decode_float, encode_float, Float, Int, Word, WORD_BITS, WORD_BYTES};

#[test]
fn numeric_types_match_the_selected_word_width() {
    #[cfg(all(target_os = "none", target_pointer_width = "32"))]
    let (word, int, float): (u32, i32, f32) = (Word::MAX, Int::MIN, Float::MAX);
    #[cfg(not(all(target_os = "none", target_pointer_width = "32")))]
    let (word, int, float): (u64, i64, f64) = (Word::MAX, Int::MIN, Float::MAX);
    assert_eq!(word, Word::MAX);
    assert_eq!(int, Int::MIN);
    assert_eq!(float, Float::MAX);
    let expected_bytes = if cfg!(all(target_os = "none", target_pointer_width = "32")) { 4 } else { 8 };
    assert_eq!(WORD_BYTES, expected_bytes);
    assert_eq!(WORD_BITS as usize, expected_bytes * 8);
    assert_eq!(size_of::<Word>(), expected_bytes);
    assert_eq!(size_of::<Int>(), expected_bytes);
    assert_eq!(size_of::<Float>(), expected_bytes);
}

#[test]
fn integer_bit_patterns_preserve_signed_boundaries() {
    for int in [Int::MIN, Int::MIN + 1, -1, 0, 1, Int::MAX] {
        assert_eq!((int as Word) as Int, int);
    }
    assert_eq!(Int::MIN as Word, (1 as Word) << (WORD_BITS - 1));
    assert_eq!((-1 as Int) as Word, Word::MAX);
}

#[test]
fn float_codec_preserves_every_selected_bit_pattern() {
    #[cfg(all(target_os = "none", target_pointer_width = "32"))]
    let patterns: &[Word] = &[
        0, 0x8000_0000, 1, 0x007f_ffff, 0x0080_0000,
        0x3f80_0000, 0x7f7f_ffff, 0xff7f_ffff,
        0x7f80_0000, 0xff80_0000, 0x7fc0_1234, 0xffc0_5678, 0x7f80_0001,
    ];
    #[cfg(not(all(target_os = "none", target_pointer_width = "32")))]
    let patterns: &[Word] = &[
        0, 0x8000_0000_0000_0000, 1, 0x000f_ffff_ffff_ffff, 0x0010_0000_0000_0000,
        0x3ff0_0000_0000_0000, 0x7fef_ffff_ffff_ffff, 0xffef_ffff_ffff_ffff,
        0x7ff0_0000_0000_0000, 0xfff0_0000_0000_0000,
        0x7ff8_0000_0000_1234, 0xfff8_0000_0000_5678, 0x7ff0_0000_0000_0001,
    ];
    for &bits in patterns {
        let decoded: Float = decode_float(bits);
        assert_eq!(decoded.to_bits(), bits);
        assert_eq!(encode_float(decoded), bits);
        assert_eq!(encode_float(Float::from_bits(bits)), bits);
    }
}
