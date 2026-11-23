//! Single-precision `exp` and `ln` for the climate column's inner loops
//! (the column and the spectrum): branch-free, so that a loop
//! over a bin's g-points compiles to vector instructions, which the
//! library's calls do not. Both are good to a few units in the last place
//! of an `f32` (the tests), well under the 0.07 % step of the opacity
//! table's coding.

/// Definition: `log₂ e`, the standard library's constant.
const LOG2_E: f32 = std::f32::consts::LOG2_E;

/// Definition: `ln 2`, the standard library's constant.
const LN_2: f32 = std::f32::consts::LN_2;

/// Source: Cody & Waite, *Software Manual for the Elementary Functions*
/// (1980), the split of `ln 2` into a high part `0.693359375`, exact in
/// nine bits, and the rest ([`LN_2_LOW`]); checked by the audit of the
/// radiative transfer (Wiki study 19).
const LN_2_HIGH: f32 = 0.693_359_4;

/// Source: Cody & Waite (1980), as [`LN_2_HIGH`]: `ln 2 − 0.693359375`.
const LN_2_LOW: f32 = -2.121_944_4e-4;

/// Definition: `1.5 · 2²³`; a float near it holds integers in its last
/// bits (the usual rounding trick of single precision).
const ROUNDER: f32 = 12_582_912.0;

/// Definition: least argument of [`exp`], −87.3, the `ln` of the least
/// normal `f32` (1.175e-38, so −87.34) rounded up, fixed by the IEEE 754
/// binary32 range; `e^x` is then about 1.2e-38, which stands for zero in a
/// transmission or an opacity. It only keeps the result normal.
const EXP_FLOOR: f32 = -87.3;

/// Definition: greatest argument of [`exp`], 88, under the overflow of
/// an `f32` (`ln` of 3.4e38 is 88.72), rounded down, fixed by the IEEE 754
/// binary32 range. It only keeps the result finite.
const EXP_CEILING: f32 = 88.0;

/// `e^x`, relative error under 3e-7, for `x` clamped to
/// [`EXP_FLOOR`]..[`EXP_CEILING`]: `2^n e^f` with `n`
/// the nearest integer to `x log₂ e`, `f = x − n ln 2` (`|f| ≤ ln 2 / 2`)
/// and `e^f` from its Taylor series to the sixth power (remainder under
/// 1.3e-7).
#[inline(always)]
pub(crate) fn exp(x: f32) -> f32 {
    let x = x.clamp(EXP_FLOOR, EXP_CEILING);
    // `x log₂ e` rounded to the nearest integer by adding 1.5 · 2²³, whose
    // last bits then hold it.
    let shifted = x * LOG2_E + ROUNDER;
    let n = shifted - ROUNDER;
    // `x − n ln 2` in two parts (Cody & Waite), exact for `n` to 2⁹.
    let f = (x - n * LN_2_HIGH) - n * LN_2_LOW;
    let p = 1.0
        + f * (1.0
            + f * (1.0 / 2.0
                + f * (1.0 / 6.0 + f * (1.0 / 24.0 + f * (1.0 / 120.0 + f * (1.0 / 720.0))))));
    let power = shifted
        .to_bits()
        .wrapping_sub(ROUNDER.to_bits())
        .wrapping_add(127)
        << 23;
    p * f32::from_bits(power)
}

/// `ln x`, absolute error under 2e-7 plus 1e-7 of the result, for a
/// normal positive `x`; about −87.3 at 0 and the subnormals, whose
/// [`exp`] is the floor. `x = 2^e m` with `m` in `[√½, √2)` (the bits
/// shifted by those of `√½`, so that the exponent comes out without a
/// branch), and `ln m = 2 atanh s`, `s = (m − 1)/(m + 1)`, `|s| ≤ 0.172`,
/// from its series to `s⁹` (remainder under 1e-9).
#[inline(always)]
pub(crate) fn ln(x: f32) -> f32 {
    let shifted = x
        .max(f32::MIN_POSITIVE)
        .to_bits()
        .wrapping_sub(HALF_SQRT_2_BITS);
    let e = (shifted as i32) >> 23;
    let m = f32::from_bits((shifted & 0x007f_ffff).wrapping_add(HALF_SQRT_2_BITS));
    let s = (m - 1.0) / (m + 1.0);
    let s2 = s * s;
    let series =
        s * (2.0 + s2 * (2.0 / 3.0 + s2 * (2.0 / 5.0 + s2 * (2.0 / 7.0 + s2 * (2.0 / 9.0)))));
    e as f32 * LN_2 + series
}

/// Definition: the bits of `√½` as an `f32`.
const HALF_SQRT_2_BITS: u32 = 0x3f35_04f3;

#[cfg(test)]
mod tests {
    use super::*;

    /// Definition: the range and step of the sweep of the `exp` test, a
    /// little inside [`EXP_FLOOR`] and [`EXP_CEILING`], at a step that does
    /// not divide the period of the argument reduction.
    const EXP_SWEEP: (f64, f64, f64) = (-87.0, 87.0, 0.001_37);

    /// Definition: the range and ratio of the sweep of the `ln` test, from
    /// the least normal `f32` (1.175e-38) up to 1e38, at a ratio that does
    /// not divide the powers of two.
    const LN_SWEEP: (f64, f64, f64) = (1.2e-38, 1e38, 1.000_731);

    /// Definition: the relative error [`exp`] promises in its doc, 3e-7:
    /// its Taylor remainder, 1.3e-7, with room for the rounding of single
    /// precision (2⁻²⁴, 6e-8, per operation).
    const EXP_TOLERANCE: f64 = 3e-7;

    /// Definition: the error [`ln`] promises in its doc, 2e-7 relative to
    /// `1 + |ln x|`.
    const LN_TOLERANCE: f64 = 2e-7;

    /// Definition: a bound just above the floor [`exp`] returns, `e^-87.3`
    /// (1.2e-38): below it, a result is the floor.
    const EXP_FLOOR_VALUE: f32 = 2e-38;

    /// Definition: how far `ln 0` may sit from [`EXP_FLOOR`], 0.1: it is
    /// the `ln` of the least normal `f32`, −87.34.
    const LN_ZERO_TOLERANCE: f32 = 0.1;

    #[test]
    fn exp_is_the_librarys() {
        let mut worst = 0.0f64;
        let (mut x, top, step) = EXP_SWEEP;
        while x < top {
            let fast = f64::from(exp(x as f32));
            let exact = (f64::from(x as f32)).exp();
            worst = worst.max((fast / exact - 1.0).abs());
            x += step;
        }
        assert!(worst < EXP_TOLERANCE, "worst relative error {worst:e}");
        assert!(exp(-1e9) > 0.0 && exp(-1e9) < EXP_FLOOR_VALUE);
        assert!(exp(f32::NEG_INFINITY) < EXP_FLOOR_VALUE);
    }

    #[test]
    fn ln_is_the_librarys() {
        let mut worst = 0.0f64;
        let (mut x, top, ratio) = LN_SWEEP;
        while x < top {
            let fast = f64::from(ln(x as f32));
            let exact = (f64::from(x as f32)).ln();
            worst = worst.max((fast - exact).abs() / (1.0 + exact.abs()));
            x *= ratio;
        }
        assert!(worst < LN_TOLERANCE, "worst error {worst:e}");
        assert!((ln(0.0) - EXP_FLOOR).abs() < LN_ZERO_TOLERANCE);
    }
}
