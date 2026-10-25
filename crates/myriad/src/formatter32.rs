// Adapted from Rust 1.91.0 core::num::flt2dec::{decoder,strategy::dragon}.
// https://github.com/rust-lang/rust/tree/1.91.0/library/core/src/num/flt2dec
// Copyright (c) The Rust Project Contributors
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

use alloc::string::String;
use core::cmp::Ordering;

// f32 exp[-151,103], decimal exponent[-45,39], <=9 digits: <=310 working bits.
const LIMBS: usize = 24;
#[derive(Clone)]
struct Big {
    limbs: [u16; LIMBS],
    used: usize,
}
impl Big {
    fn new(value: u32) -> Self {
        let mut result = Self {
            limbs: [0; LIMBS],
            used: 0,
        };
        if value != 0 {
            result.limbs[0] = value as u16;
            result.used = 1;
            if value >> 16 != 0 {
                result.limbs[1] = (value >> 16) as u16;
                result.used = 2;
            }
        }
        result
    }
    fn compare(&self, other: &Self) -> Ordering {
        self.used.cmp(&other.used).then_with(|| {
            for index in (0..self.used).rev() {
                let ordering = self.limbs[index].cmp(&other.limbs[index]);
                if ordering != Ordering::Equal {
                    return ordering;
                }
            }
            Ordering::Equal
        })
    }
    fn push(&mut self, carry: u32) {
        if carry != 0 {
            assert!(self.used < LIMBS, "formatter limb capacity");
            self.limbs[self.used] = carry as u16;
            self.used += 1;
        }
    }
    fn mul_small(&mut self, multiplier: u32) {
        debug_assert!(multiplier <= 10);
        let mut carry = 0;
        for index in 0..self.used {
            let product = self.limbs[index] as u32 * multiplier + carry;
            self.limbs[index] = product as u16;
            carry = product >> 16;
        }
        self.push(carry);
    }
    fn mul_pow10(&mut self, exponent: usize) {
        for _ in 0..exponent {
            self.mul_small(10);
        }
    }
    fn shl(&mut self, bits: usize) {
        if self.used == 0 {
            return;
        }
        let words = bits / 16;
        let shift = bits % 16;
        assert!(self.used + words <= LIMBS, "formatter limb capacity");
        for index in (0..self.used).rev() {
            self.limbs[index + words] = self.limbs[index];
        }
        self.limbs[..words].fill(0);
        self.used += words;
        if shift != 0 {
            let mut carry = 0;
            for index in words..self.used {
                let value = ((self.limbs[index] as u32) << shift) | carry;
                self.limbs[index] = value as u16;
                carry = value >> 16;
            }
            self.push(carry);
        }
    }
    fn add(&mut self, other: &Self) {
        let length = self.used.max(other.used);
        let mut carry = 0;
        for index in 0..length {
            let sum = self.limbs[index] as u32 + other.limbs[index] as u32 + carry;
            self.limbs[index] = sum as u16;
            carry = sum >> 16;
        }
        self.used = length;
        self.push(carry);
    }
    fn sub(&mut self, other: &Self) {
        debug_assert!(self.compare(other) != Ordering::Less);
        let mut borrow = 0;
        for index in 0..self.used {
            let left = self.limbs[index] as u32;
            let right = other.limbs[index] as u32 + borrow;
            self.limbs[index] = left.wrapping_sub(right) as u16;
            borrow = u32::from(left < right);
        }
        debug_assert_eq!(borrow, 0);
        while self.used != 0 && self.limbs[self.used - 1] == 0 {
            self.used -= 1;
        }
    }
}

struct Decoded {
    mant: u32,
    minus: u32,
    plus: u32,
    exponent: i32,
    inclusive: bool,
}
fn decode(bits: u32) -> Decoded {
    let exponent = (bits >> 23) & 255;
    let fraction = bits & 0x7f_ffff;
    let mant = if exponent == 0 {
        fraction << 1
    } else {
        fraction | 0x80_0000
    };
    let inclusive = mant & 1 == 0;
    let exponent = exponent as i32 - 150;
    if bits & 0x7f80_0000 == 0 {
        Decoded {
            mant,
            minus: 1,
            plus: 1,
            exponent,
            inclusive,
        }
    } else if mant == 0x80_0000 {
        Decoded {
            mant: mant << 2,
            minus: 1,
            plus: 2,
            exponent: exponent - 2,
            inclusive,
        }
    } else {
        Decoded {
            mant: mant << 1,
            minus: 1,
            plus: 1,
            exponent: exponent - 1,
            inclusive,
        }
    }
}

fn estimate(mantissa: u32, exponent: i32) -> i32 {
    let bits = 32 - (mantissa - 1).leading_zeros() as i32;
    let n = bits + exponent;
    debug_assert!((-151..=130).contains(&n));
    (n * 78913) >> 18
}
fn allowed(ordering: Ordering, inclusive: bool) -> bool {
    ordering == Ordering::Less || (inclusive && ordering == Ordering::Equal)
}
fn upper_compare(scale: &Big, mant: &Big, plus: &Big) -> Ordering {
    let mut upper = mant.clone();
    upper.add(plus);
    scale.compare(&upper)
}
fn digit(mant: &mut Big, scales: &[Big; 4]) -> u8 {
    let mut value = 0;
    for index in (0..4).rev() {
        if mant.compare(&scales[index]) != Ordering::Less {
            mant.sub(&scales[index]);
            value += 1 << index;
        }
    }
    debug_assert!(value < 10);
    value
}
fn shortest(decoded: Decoded) -> ([u8; 10], usize, i32) {
    let mut exponent = estimate(decoded.mant + decoded.plus, decoded.exponent);
    let mut mant = Big::new(decoded.mant);
    let mut minus = Big::new(decoded.minus);
    let mut plus = Big::new(decoded.plus);
    let mut scale = Big::new(1);
    if decoded.exponent < 0 {
        scale.shl((-decoded.exponent) as usize);
    } else {
        mant.shl(decoded.exponent as usize);
        minus.shl(decoded.exponent as usize);
        plus.shl(decoded.exponent as usize);
    }
    if exponent >= 0 {
        scale.mul_pow10(exponent as usize);
    } else {
        mant.mul_pow10((-exponent) as usize);
        minus.mul_pow10((-exponent) as usize);
        plus.mul_pow10((-exponent) as usize);
    }
    if allowed(upper_compare(&scale, &mant, &plus), decoded.inclusive) {
        exponent += 1;
    } else {
        mant.mul_small(10);
        minus.mul_small(10);
        plus.mul_small(10);
    }
    let scales = core::array::from_fn(|index| {
        let mut value = scale.clone();
        value.shl(index);
        value
    });
    let mut digits = [0; 10];
    let mut length = 0;
    loop {
        assert!(length < 9, "f32 shortest digit bound");
        digits[length] = b'0' + digit(&mut mant, &scales);
        length += 1;
        let down = allowed(mant.compare(&minus), decoded.inclusive);
        let up = allowed(upper_compare(&scale, &mant, &plus), decoded.inclusive);
        if down || up {
            mant.shl(1);
            if up && (!down || mant.compare(&scale) != Ordering::Less) {
                let mut index = length;
                while index != 0 && digits[index - 1] == b'9' {
                    index -= 1;
                    digits[index] = b'0';
                }
                if index != 0 {
                    digits[index - 1] += 1;
                } else {
                    digits[0] = b'1';
                    digits[length] = b'0';
                    length += 1;
                    exponent += 1;
                }
            }
            break;
        }
        mant.mul_small(10);
        minus.mul_small(10);
        plus.mul_small(10);
    }
    debug_assert!(length <= 9);
    (digits, length, exponent)
}

pub(crate) fn format_float(value: f32) -> String {
    let bits = value.to_bits();
    let magnitude = bits & 0x7fff_ffff;
    if magnitude > 0x7f80_0000 {
        return String::from("NaN");
    }
    let mut output = String::with_capacity(64);
    if bits >> 31 != 0 {
        output.push('-');
    }
    if magnitude == 0x7f80_0000 {
        output.push_str("inf");
        return output;
    }
    if magnitude == 0 {
        output.push('0');
        return output;
    }
    let (digits, length, exponent) = shortest(decode(magnitude));
    if exponent <= 0 {
        output.push_str("0.");
        for _ in 0..-exponent {
            output.push('0');
        }
        for &byte in &digits[..length] {
            output.push(byte as char);
        }
    } else {
        for index in 0..length.max(exponent as usize) {
            if index == exponent as usize {
                output.push('.');
            }
            output.push(if index < length {
                digits[index] as char
            } else {
                '0'
            });
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn conservative_f32_working_bounds_fit_every_intermediate() {
        fn bits(value: &Big) -> usize {
            if value.used == 0 {
                0
            } else {
                (value.used - 1) * 16 + 16 - value.limbs[value.used - 1].leading_zeros() as usize
            }
        }
        // Independent upper bounds overestimate combinations that cannot coexist:
        // mant <2^26, exp<=103, |negative decimal scale|<=45, <=9 digit factors.
        let mut mant = Big::new((1 << 26) - 1);
        mant.shl(103);
        mant.mul_pow10(45 + 9);
        let mut upper = mant.clone();
        upper.add(&mant);
        mant.shl(1);
        assert!(bits(&upper) <= 310 && bits(&mant) <= 310);
        // scale <=2^151 *10^39; cached scale8 adds three bits.
        let mut scale = Big::new(1);
        scale.shl(151);
        scale.mul_pow10(39);
        scale.shl(3);
        assert!(bits(&scale) <= 284);
        assert!(310 < LIMBS * 16);
    }

    #[test]
    fn top_limb_fits_without_growth() {
        let mut value = Big::new(1);
        value.shl(383);
        assert_eq!(value.used, 24);
        assert_eq!(value.limbs[23], 0x8000);
        value.mul_small(1);
    }
    #[test]
    #[should_panic(expected = "formatter limb capacity")]
    fn overflowing_internal_limb_is_rejected() {
        Big::new(1).shl(384);
    }
    #[test]
    fn bounded_estimator_matches_the_upstream_integer_formula() {
        for n in -151i32..=130 {
            assert_eq!((n * 78913) >> 18, ((n as i64 * 1292913986) >> 32) as i32);
        }
    }
}
