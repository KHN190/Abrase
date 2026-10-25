"""Reproducible certificate; Python standard-library fractions only.

Machin's alternating series brackets pi. Common continued-fraction terms
bracket alpha = 2**(exponent-150) * 2/pi. The best-approximation-of-second-kind
theorem applies to Q=2**24 < q_next: for 0 < q <= Q and integer p,
abs(q*alpha-p) >= abs(qn*alpha-pn) > 1/(qn+q_next) > 2**-30.
This bounds every finite f32 mantissa for all 121 large-input exponents;
it is a finite-index certificate, not an enumeration of arbitrary reals.

Each base-256 convolution has <=3 products of bytes: q[i] <=195075.
Including carry, z <=195840 <2**18, so these integer f32 operations are exact.
q0 is in [-8,2]. The omitted tail is bounded by
195075 * 2**(2-8*10)/(1-1/256) <2**-60.
Earlier omitted terms are multiples of 8 and do not change the quadrant.
Thus the represented distance to an integer remains >2**-31.
If the first five fractional limbs were zero, the remaining tail would be
<2**(q0-40) <=2**-38: contradiction. Hence k<=5; one recomputation moves
the existing significant limb into iq[9..jz], ending recomputation.
Therefore jz<=14, jx<=2, f index<=16; q/iq/fq index<=15 after splitting z.
The significant limb also stops zero-chop before jz can underflow.
"""
from fractions import Fraction as F
from pathlib import Path
import re


def atan_interval(denominator):
    x = F(1, denominator)
    lower = sum(((-1 if n & 1 else 1) * x ** (2*n+1) / (2*n+1)
                 for n in range(80)), F(0))
    return lower, lower + x ** 161 / 161


lo5, hi5 = atan_interval(5)
lo239, hi239 = atan_interval(239)
pi_lo, pi_hi = 16*lo5 - 4*hi239, 16*hi5 - 4*lo239
source = (Path(__file__).parents[2] / "src/math32.rs").read_text()
limbs = [int(token, 16) for token in re.findall(r"0x[0-9a-fA-F]+", source.split("const TWO_OVER_PI")[1].split("];")[0])][:29]
expected = 0
for limb in limbs: expected = expected*256 + limb
scale = 1 << (8*len(limbs))
assert (2*scale/pi_hi).__floor__() == expected == (2*scale/pi_lo).__floor__()
assert F(195075) * F(2)**(2-80) / (1-F(1,256)) < F(2)**-60
worst = (0, 0)
for exponent in range(134, 255):
    factor = F(2) ** (exponent - 150) * 2
    lo, hi = factor / pi_hi, factor / pi_lo
    previous, current = 1, 0
    while True:
        a, b = lo.numerator // lo.denominator, hi.numerator // hi.denominator
        assert a == b, "pi enclosure too wide for this continued fraction"
        following = a*current + previous
        if following > 1 << 24:
            worst = max(worst, (following + current, exponent))
            break
        previous, current = current, following
        lo, hi = 1/(hi-a), 1/(lo-a)
assert worst == (983782263, 222)
assert worst[0] < 1 << 30
print("121 exponent certificates: distance > 2**-30; max denominator sum", worst)
