// Rust adaptation of newlib's float fdlibm (sf_sin/sf_cos, ef_rem_pio2,
// kf_rem_pio2, kf_sin/kf_cos, ef_sqrt). Float conversion: Ian Lance Taylor.
// Source: https://github.com/mirror/newlib-cygwin/tree/1b7c72fdcc4bde7520407d2d3364146f04fb8312/newlib/libm/math
// Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.
// Developed at SunPro, a Sun Microsystems, Inc. business.
// Permission to use, copy, modify, and distribute this software is freely
// granted, provided that this notice is preserved.

// Base-256 digits of 2/pi for argument reduction, not sampled trig values.
const TWO_OVER_PI: [u8; 198] = [
    0xA2, 0xF9, 0x83, 0x6E, 0x4E, 0x44, 0x15, 0x29, 0xFC, 0x27, 0x57, 0xD1, 0xF5, 0x34, 0xDD, 0xC0,
    0xDB, 0x62, 0x95, 0x99, 0x3C, 0x43, 0x90, 0x41, 0xFE, 0x51, 0x63, 0xAB, 0xDE, 0xBB, 0xC5, 0x61,
    0xB7, 0x24, 0x6E, 0x3A, 0x42, 0x4D, 0xD2, 0xE0, 0x06, 0x49, 0x2E, 0xEA, 0x09, 0xD1, 0x92, 0x1C,
    0xFE, 0x1D, 0xEB, 0x1C, 0xB1, 0x29, 0xA7, 0x3E, 0xE8, 0x82, 0x35, 0xF5, 0x2E, 0xBB, 0x44, 0x84,
    0xE9, 0x9C, 0x70, 0x26, 0xB4, 0x5F, 0x7E, 0x41, 0x39, 0x91, 0xD6, 0x39, 0x83, 0x53, 0x39, 0xF4,
    0x9C, 0x84, 0x5F, 0x8B, 0xBD, 0xF9, 0x28, 0x3B, 0x1F, 0xF8, 0x97, 0xFF, 0xDE, 0x05, 0x98, 0x0F,
    0xEF, 0x2F, 0x11, 0x8B, 0x5A, 0x0A, 0x6D, 0x1F, 0x6D, 0x36, 0x7E, 0xCF, 0x27, 0xCB, 0x09, 0xB7,
    0x4F, 0x46, 0x3F, 0x66, 0x9E, 0x5F, 0xEA, 0x2D, 0x75, 0x27, 0xBA, 0xC7, 0xEB, 0xE5, 0xF1, 0x7B,
    0x3D, 0x07, 0x39, 0xF7, 0x8A, 0x52, 0x92, 0xEA, 0x6B, 0xFB, 0x5F, 0xB1, 0x1F, 0x8D, 0x5D, 0x08,
    0x56, 0x03, 0x30, 0x46, 0xFC, 0x7B, 0x6B, 0xAB, 0xF0, 0xCF, 0xBC, 0x20, 0x9A, 0xF4, 0x36, 0x1D,
    0xA9, 0xE3, 0x91, 0x61, 0x5E, 0xE6, 0x1B, 0x08, 0x65, 0x99, 0x85, 0x5F, 0x14, 0xA0, 0x68, 0x40,
    0x8D, 0xFF, 0xD8, 0x80, 0x4D, 0x73, 0x27, 0x31, 0x06, 0x06, 0x15, 0x56, 0xCA, 0x73, 0xA8, 0xC9,
    0x60, 0xE2, 0x7B, 0xC0, 0x8C, 0x6B,
];
const PIO2: [f32; 11] = [
    f32::from_bits(0x3fc90000),
    f32::from_bits(0x39f00000),
    f32::from_bits(0x37da0000),
    f32::from_bits(0x33a20000),
    f32::from_bits(0x2e840000),
    f32::from_bits(0x2b500000),
    f32::from_bits(0x27c20000),
    f32::from_bits(0x22d00000),
    f32::from_bits(0x1fc40000),
    f32::from_bits(0x1bc60000),
    f32::from_bits(0x17440000),
];

// Reduction-only scaling; its proven exponent range stays normal.
fn scale_reduction(x: f32, n: i32) -> f32 {
    debug_assert!((-48..=10).contains(&n));
    x * f32::from_bits(((127 + n) as u32) << 23)
}

fn kernel_reduce(x: &[f32], e0: i32) -> (i32, f32, f32) {
    const JK: usize = 9;
    let jx = x.len() - 1;
    let jv = ((e0 - 3) / 8).max(0) as usize;
    let mut q0 = e0 - 8 * (jv as i32 + 1);
    debug_assert!((-8..=2).contains(&q0));
    let mut f = [0.0f32; 20];
    let mut q = [0.0f32; 20];
    let mut fq = [0.0f32; 20];
    let mut iq = [0i32; 20];
    for (i, item) in f.iter_mut().enumerate().take(jx + JK + 1) {
        let j = jv as i32 - jx as i32 + i as i32;
        *item = if j < 0 {
            0.0
        } else {
            TWO_OVER_PI[j as usize] as f32
        };
    }
    for i in 0..=JK {
        for j in 0..=jx {
            q[i] += x[j] * f[jx + i - j];
        }
    }
    let mut jz = JK;
    let mut n;
    let mut z;
    let mut ih;
    loop {
        z = q[jz];
        for i in 0..jz {
            let fw = (z * (1.0 / 256.0)) as i32 as f32;
            iq[i] = (z - 256.0 * fw) as i32;
            z = q[jz - i - 1] + fw;
        }
        z = scale_reduction(z, q0);
        // All convolution terms are nonnegative, so truncation equals floor here.
        debug_assert!(z >= 0.0);
        z -= 8.0 * (z * 0.125) as i32 as f32;
        n = z as i32;
        z -= n as f32;
        ih = 0;
        if q0 > 0 {
            let i = iq[jz - 1] >> (8 - q0);
            n += i;
            iq[jz - 1] -= i << (8 - q0);
            ih = iq[jz - 1] >> (7 - q0);
        } else if q0 == 0 {
            ih = iq[jz - 1] >> 7;
        } else if z >= 0.5 {
            ih = 2;
        }
        if ih > 0 {
            n += 1;
            let mut carry = false;
            for item in iq.iter_mut().take(jz) {
                if !carry {
                    if *item != 0 {
                        carry = true;
                        *item = 256 - *item;
                    }
                } else {
                    *item = 255 - *item;
                }
            }
            if q0 > 0 {
                iq[jz - 1] &= 255 >> q0;
            }
            if ih == 2 {
                z = 1.0 - z;
                if carry {
                    z -= scale_reduction(1.0, q0);
                }
            }
        }
        if z == 0.0 && iq[JK..jz].iter().all(|&item| item == 0) {
            let mut k = 1;
            while iq[JK - k] == 0 {
                k += 1;
            }
            debug_assert!(k <= 5);
            for i in jz + 1..=jz + k {
                f[jx + i] = TWO_OVER_PI[jv + i] as f32;
                q[i] = 0.0;
                for j in 0..=jx {
                    q[i] += x[j] * f[jx + i - j];
                }
            }
            jz += k;
            continue;
        }
        break;
    }
    debug_assert!(jz <= 14);
    if z == 0.0 {
        jz -= 1;
        q0 -= 8;
        while iq[jz] == 0 {
            jz -= 1;
            q0 -= 8;
        }
    } else {
        z = scale_reduction(z, -q0);
        if z >= 256.0 {
            let fw = (z * (1.0 / 256.0)) as i32 as f32;
            iq[jz] = (z - 256.0 * fw) as i32;
            jz += 1;
            q0 += 8;
            iq[jz] = fw as i32;
        } else {
            iq[jz] = z as i32;
        }
    }
    let mut fw = scale_reduction(1.0, q0);
    for i in (0..=jz).rev() {
        q[i] = fw * iq[i] as f32;
        fw *= 1.0 / 256.0;
    }
    for i in (0..=jz).rev() {
        let mut sum = 0.0;
        for k in 0..=JK.min(jz - i) {
            sum += PIO2[k] * q[i + k];
        }
        fq[jz - i] = sum;
    }
    let mut hi = 0.0;
    for i in (0..=jz).rev() {
        hi += fq[i];
    }
    let mut lo = fq[0] - hi;
    for item in fq.iter().take(jz + 1).skip(1) {
        lo += *item;
    }
    if ih != 0 {
        hi = -hi;
        lo = -lo;
    }
    (n & 7, hi, lo)
}

#[cfg_attr(all(feature = "ram-math", target_arch = "arm", target_os = "none"), unsafe(link_section = ".data.ram_func"))]
fn reduce(x: f32) -> (i32, f32, f32) {
    let bits = x.to_bits();
    let magnitude = bits & 0x7fff_ffff;
    let negative = bits >> 31 != 0;
    let t = f32::from_bits(magnitude);
    const P1: f32 = f32::from_bits(0x3fc90f80);
    const T1: f32 = f32::from_bits(0x37354443);
    const P2: f32 = f32::from_bits(0x37354400);
    const T2: f32 = f32::from_bits(0x2e85a308);
    const P3: f32 = f32::from_bits(0x2e85a300);
    const T3: f32 = f32::from_bits(0x248d3132);
    let (n, hi, lo) = if magnitude < 0x4016cbe4 {
        let mut z = t - P1;
        if magnitude & 0xfffffff0 != 0x3fc90fd0 {
            let hi = z - T1;
            (1, hi, (z - hi) - T1)
        } else {
            z -= P2;
            let hi = z - T2;
            (1, hi, (z - hi) - T2)
        }
    } else if magnitude <= 0x43490f80 {
        let n = (t * f32::from_bits(0x3f22f984) + 0.5) as i32;
        let multiple = n as f32;
        let mut r = t - multiple * P1;
        let mut w = multiple * T1;
        let mut hi = r - w;
        let exponent = (magnitude >> 23) as i32;
        if exponent - ((hi.to_bits() >> 23) & 255) as i32 > 8 {
            let previous = r;
            w = multiple * P2;
            r = previous - w;
            w = multiple * T2 - ((previous - r) - w);
            hi = r - w;
            if exponent - ((hi.to_bits() >> 23) & 255) as i32 > 25 {
                let previous = r;
                w = multiple * P3;
                r = previous - w;
                w = multiple * T3 - ((previous - r) - w);
                hi = r - w;
            }
        }
        (n, hi, (r - hi) - w)
    } else {
        let e0 = (magnitude >> 23) as i32 - 134;
        let mut z = f32::from_bits(magnitude.wrapping_sub((e0 as u32) << 23));
        let mut chunks = [0.0; 3];
        for chunk in chunks.iter_mut().take(2) {
            *chunk = z as i32 as f32;
            z = (z - *chunk) * 256.0;
        }
        chunks[2] = z;
        let mut len = 3;
        while chunks[len - 1] == 0.0 {
            len -= 1;
        }
        kernel_reduce(&chunks[..len], e0)
    };
    if negative {
        (-n, -hi, -lo)
    } else {
        (n, hi, lo)
    }
}

fn kernel_sin(x: f32, y: f32, tail: bool) -> f32 {
    if x.to_bits() & 0x7fff_ffff < 0x32000000 {
        return x;
    }
    let z = x * x;
    let v = z * x;
    let s1 = f32::from_bits(0xbe2aaaab);
    let r = f32::from_bits(0x3c088889)
        + z * (f32::from_bits(0xb9500d01)
            + z * (f32::from_bits(0x3638ef1b)
                + z * (f32::from_bits(0xb2d72f34) + z * f32::from_bits(0x2f2ec9d3))));
    if !tail {
        x + v * (s1 + z * r)
    } else {
        x - ((z * (0.5 * y - v * r) - y) - v * s1)
    }
}

fn kernel_cos(x: f32, y: f32) -> f32 {
    let bits = x.to_bits() & 0x7fff_ffff;
    if bits < 0x32000000 {
        return 1.0;
    }
    let z = x * x;
    let r = z
        * (f32::from_bits(0x3d2aaaab)
            + z * (f32::from_bits(0xbab60b61)
                + z * (f32::from_bits(0x37d00d01)
                    + z * (f32::from_bits(0xb493f27c)
                        + z * (f32::from_bits(0x310f74f6) + z * f32::from_bits(0xad47d74e))))));
    if bits < 0x3e99999a {
        1.0 - (0.5 * z - (z * r - x * y))
    } else {
        let qx = if bits > 0x3f480000 {
            0.28125
        } else {
            f32::from_bits(bits - 0x01000000)
        };
        let hz = 0.5 * z - qx;
        let a = 1.0 - qx;
        a - (hz - (z * r - x * y))
    }
}

#[cfg_attr(all(feature = "ram-math", target_arch = "arm", target_os = "none"), unsafe(link_section = ".data.ram_func"))]
pub fn sin(x: f32) -> f32 {
    let bits = x.to_bits() & 0x7fff_ffff;
    if bits <= 0x3f490fd8 {
        return kernel_sin(x, 0.0, false);
    }
    if bits >= 0x7f800000 {
        return x - x;
    }
    let (n, hi, lo) = reduce(x);
    match n & 3 {
        0 => kernel_sin(hi, lo, true),
        1 => kernel_cos(hi, lo),
        2 => -kernel_sin(hi, lo, true),
        _ => -kernel_cos(hi, lo),
    }
}

#[cfg_attr(all(feature = "ram-math", target_arch = "arm", target_os = "none"), unsafe(link_section = ".data.ram_func"))]
pub fn cos(x: f32) -> f32 {
    let bits = x.to_bits() & 0x7fff_ffff;
    if bits <= 0x3f490fd8 {
        return kernel_cos(x, 0.0);
    }
    if bits >= 0x7f800000 {
        return x - x;
    }
    let (n, hi, lo) = reduce(x);
    match n & 3 {
        0 => kernel_cos(hi, lo),
        1 => -kernel_sin(hi, lo, true),
        2 => -kernel_cos(hi, lo),
        _ => kernel_sin(hi, lo, true),
    }
}

pub fn sqrt(x: f32) -> f32 {
    let mut ix = x.to_bits() as i32;
    let magnitude = ix & 0x7fff_ffff;
    if magnitude >= 0x7f800000 {
        return x * x + x;
    }
    if magnitude == 0 {
        return x;
    }
    if ix < 0 {
        return f32::NAN;
    }
    let mut m = ix >> 23;
    if magnitude < 0x00800000 {
        let mut i = 0;
        while ix & 0x00800000 == 0 {
            ix <<= 1;
            i += 1;
        }
        m -= i - 1;
    }
    m -= 127;
    ix = (ix & 0x007fffff) | 0x00800000;
    if m & 1 != 0 {
        ix += ix;
    }
    m >>= 1;
    ix += ix;
    let mut q = 0;
    let mut s = 0;
    let mut r = 0x01000000;
    while r != 0 {
        let t = s + r;
        if t <= ix {
            s = t + r;
            ix -= t;
            q += r;
        }
        ix += ix;
        r >>= 1;
    }
    // Rust's round-to-nearest, ties-to-even; floating exception flags are not exposed.
    if ix != 0 {
        q += q & 1;
    }
    f32::from_bits(((q >> 1) + 0x3f000000 + (m << 23)) as u32)
}
