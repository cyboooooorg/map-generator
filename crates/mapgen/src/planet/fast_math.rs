//! Single-precision `exp` and `ln` for the climate column's inner loops
//! ([`super::column`], [`super::spectrum`]): branch-free, so that a loop
//! over a bin's g-points compiles to vector instructions, which the
//! library's calls do not. Both are good to a few units in the last place
//! of an `f32` (the tests), well under the 0.07 % step of the opacity
//! table's coding.

/// `log₂ e`.
const LOG2_E: f32 = std::f32::consts::LOG2_E;

/// `ln 2`.
const LN_2: f32 = std::f32::consts::LN_2;

/// `ln 2` as `0.693359375` (exact in nine bits) plus the rest.
const LN_2_HIGH: f32 = 0.693_359_4;
const LN_2_LOW: f32 = -2.121_944_4e-4;

/// `1.5 · 2²³`: a float near it holds integers in its last bits.
const ROUNDER: f32 = 12_582_912.0;

/// Least argument of [`exp`]: `e^x` is then the least normal `f32` (about
/// 1.4e-38), which stands for zero in a transmission or an opacity.
const EXP_FLOOR: f32 = -87.3;

/// Greatest argument of [`exp`], under the overflow of an `f32`.
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

/// The bits of `√½` as an `f32`.
const HALF_SQRT_2_BITS: u32 = 0x3f35_04f3;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exp_is_the_librarys() {
        let mut worst = 0.0f64;
        let mut x = -87.0f64;
        while x < 87.0 {
            let fast = f64::from(exp(x as f32));
            let exact = (f64::from(x as f32)).exp();
            worst = worst.max((fast / exact - 1.0).abs());
            x += 0.001_37;
        }
        assert!(worst < 3e-7, "worst relative error {worst:e}");
        assert!(exp(-1e9) > 0.0 && exp(-1e9) < 2e-38);
        assert!(exp(f32::NEG_INFINITY) < 2e-38);
    }

    #[test]
    fn ln_is_the_librarys() {
        let mut worst = 0.0f64;
        let mut x = 1.2e-38f64;
        while x < 1e38 {
            let fast = f64::from(ln(x as f32));
            let exact = (f64::from(x as f32)).ln();
            worst = worst.max((fast - exact).abs() / (1.0 + exact.abs()));
            x *= 1.000_731;
        }
        assert!(worst < 2e-7, "worst error {worst:e}");
        assert!((ln(0.0) + 87.3).abs() < 0.1);
    }
}
