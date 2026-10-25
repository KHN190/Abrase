extern crate alloc;
#[path = "../src/formatter32.rs"]
mod formatter32;

fn check(value: f32) {
    let actual = formatter32::format_float(value);
    assert_eq!(actual, value.to_string(), "bits={:#010x}", value.to_bits());
    if value.is_finite() {
        assert_eq!(actual.parse::<f32>().unwrap().to_bits(), value.to_bits());
    }
}

#[test]
fn special_values_and_decimal_layout_match_rust_display() {
    for value in [
        0.0,
        -0.0,
        f32::NAN,
        f32::from_bits(0xffc00001),
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(1),
        f32::MIN_POSITIVE,
        f32::MAX,
        -f32::MAX,
        1.0,
        -1.0,
        1e-20,
        1e20,
        0.1,
        1.2345678,
    ] {
        check(value);
    }
}

#[test]
fn every_exponent_and_rounding_neighborhood_matches_exact_text() {
    for exponent in 0..255u32 {
        for mantissa in [0, 1, 2, 0x3fffff, 0x400000, 0x7ffffd, 0x7ffffe, 0x7fffff] {
            let bits = (exponent << 23) | mantissa;
            check(f32::from_bits(bits));
            check(f32::from_bits(bits | 0x80000000));
        }
    }
    for exponent in -45..=38 {
        let center = 10f32.powi(exponent).to_bits();
        for offset in -128..=128 {
            let value = f32::from_bits(center.wrapping_add_signed(offset));
            if value.is_finite() {
                check(value);
                check(-value);
            }
        }
    }
}

#[test]
fn million_random_patterns_match_display_and_round_trip() {
    let mut state = 0x61c8_8647u32;
    for _ in 0..1_000_000 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        check(f32::from_bits(state));
    }
}

#[test]
fn decimal_midpoint_ties_follow_rusts_upward_shortest_choice() {
    let midpoint = 0.000244140625f32;
    assert_eq!(formatter32::format_float(midpoint), "0.00024414063");
    assert_eq!(formatter32::format_float(-midpoint), "-0.00024414063");
    check(midpoint);
    check(-midpoint);
    for exponent in 1..=38 {
        let center = 10f32.powi(exponent).to_bits();
        for offset in -4..=4 {
            check(f32::from_bits(center.wrapping_add_signed(offset)));
        }
    }
}
