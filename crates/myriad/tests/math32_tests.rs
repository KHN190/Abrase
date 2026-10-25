#[path = "../src/math32.rs"]
mod math32;

fn ulp(a: f32, b: f32) -> u32 {
    fn ordered(x: f32) -> u32 {
        let bits = x.to_bits();
        if bits >> 31 != 0 {
            !bits
        } else {
            bits | 0x8000_0000
        }
    }
    ordered(a).abs_diff(ordered(b))
}

#[test]
fn special_values_preserve_zero_and_handle_nan_inf_and_subnormals() {
    for x in [0.0f32, -0.0] {
        assert_eq!(math32::sin(x).to_bits(), x.to_bits());
        assert_eq!(math32::cos(x), 1.0);
        assert_eq!(math32::sqrt(x).to_bits(), x.to_bits());
    }
    for x in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(math32::sin(x).is_nan());
        assert!(math32::cos(x).is_nan());
    }
    assert_eq!(math32::sqrt(f32::INFINITY), f32::INFINITY);
    for x in [-1.0, f32::NEG_INFINITY, f32::NAN] {
        assert!(math32::sqrt(x).is_nan());
    }
    for bits in [1, 2, 0x3fffff, 0x7fffff, 0x800000, 0x800001] {
        let x = f32::from_bits(bits);
        assert_eq!(math32::sin(x), x);
        assert_eq!(math32::sin(-x), -x);
        assert_eq!(math32::cos(x), 1.0);
        assert_eq!(math32::sqrt(x).to_bits(), x.sqrt().to_bits());
    }
}

#[test]
fn trig_matches_reference_across_exponents_and_reduction_boundaries() {
    let mut max_ulp = 0;
    let mut max_absolute = 0.0f32;
    let mut witness = 0u32;
    let mut check = |x: f32| {
        for (got, reference) in [
            (math32::sin(x), (x as f64).sin() as f32),
            (math32::cos(x), (x as f64).cos() as f32),
        ] {
            let error = ulp(got, reference);
            if error > max_ulp {
                witness = x.to_bits();
                max_ulp = error;
            }
            max_absolute = max_absolute.max((got - reference).abs());
            assert!(
                error <= 3,
                "x={:#010x} got={got:?} ref={reference:?} ulp={error}",
                x.to_bits()
            );
            assert!((got - reference).abs() <= 2.0e-7);
        }
    };
    for bits in [
        0x3f490fd8u32,
        0x3fc90fd0,
        0x4016cbe4,
        0x43490f80,
        0x7f7fffff,
    ] {
        for delta in -64i32..=64 {
            let x = f32::from_bits(bits.wrapping_add_signed(delta));
            if x.is_finite() {
                check(x);
                check(-x);
            }
        }
    }
    for n in 1..=129 {
        let center = ((n as f64) * core::f64::consts::FRAC_PI_2) as f32;
        for delta in -256..=256 {
            check(f32::from_bits(center.to_bits().wrapping_add_signed(delta)));
        }
    }
    for exponent in 0..255u32 {
        for mantissa in [0, 1, 0x3fffff, 0x400000, 0x7ffffe, 0x7fffff] {
            let x = f32::from_bits((exponent << 23) | mantissa);
            check(x);
            check(-x);
        }
    }
    let mut state = 0x91e1_0da5u32;
    for _ in 0..5_000_000 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        let x = f32::from_bits(state);
        if x.is_finite() {
            check(x);
        }
    }
    eprintln!(
        "trig observed max_ulp={max_ulp}, max_absolute={max_absolute:e}, witness={witness:#010x}"
    );
}

#[test]
fn sqrt_is_correctly_rounded_on_stratified_and_random_positive_inputs() {
    for exponent in 0..255u32 {
        for mantissa in [0, 1, 0x3fffff, 0x400000, 0x7ffffe, 0x7fffff] {
            let x = f32::from_bits((exponent << 23) | mantissa);
            assert_eq!(
                math32::sqrt(x).to_bits(),
                x.sqrt().to_bits(),
                "x={:#x}",
                x.to_bits()
            );
        }
    }
    let mut state = 0x1234_5678u32;
    for _ in 0..1_000_000 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        let x = f32::from_bits(state & 0x7fff_ffff);
        if x.is_finite() {
            assert_eq!(
                math32::sqrt(x).to_bits(),
                x.sqrt().to_bits(),
                "x={:#x}",
                x.to_bits()
            );
        }
    }
}

#[test]
fn symmetry_identity_and_local_continuity_hold() {
    let mut state = 0xdead_beefu32;
    for _ in 0..100_000 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        let x = f32::from_bits(state & 0x7fff_ffff);
        if !x.is_finite() {
            continue;
        }
        let s = math32::sin(x);
        let c = math32::cos(x);
        assert_eq!(math32::sin(-x).to_bits(), s.to_bits() ^ 0x8000_0000);
        assert_eq!(math32::cos(-x).to_bits(), c.to_bits());
        assert!((s * s + c * c - 1.0).abs() <= 3.0e-7);
        let next = f32::from_bits(x.to_bits() + 1);
        if next.is_finite() {
            assert!(math32::sqrt(x) <= math32::sqrt(next));
        }
    }
    for n in 1..=128 {
        let center = (n as f64 * core::f64::consts::FRAC_PI_2) as f32;
        let mut previous_x = f32::from_bits(center.to_bits() - 32);
        let mut previous_s = math32::sin(previous_x);
        let mut previous_c = math32::cos(previous_x);
        for offset in -31..=32 {
            let x = f32::from_bits(center.to_bits().wrapping_add_signed(offset));
            let s = math32::sin(x);
            let c = math32::cos(x);
            assert!((s - previous_s).abs() <= x - previous_x + 2.0e-7);
            assert!((c - previous_c).abs() <= x - previous_x + 2.0e-7);
            if n & 1 == 0 {
                if (n / 2) & 1 == 0 {
                    assert!(s >= previous_s);
                } else {
                    assert!(s <= previous_s);
                }
            } else if ((n + 1) / 2) & 1 == 0 {
                assert!(c >= previous_c);
            } else {
                assert!(c <= previous_c);
            }
            previous_x = x;
            previous_s = s;
            previous_c = c;
        }
    }
}
