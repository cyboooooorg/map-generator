//! The uniform number every draw of the engine builds on, and the
//! samplers built on it. Each sampler reads its own stream only.

use rand_chacha::ChaCha8Rng;
use rand_core::Rng;

/// Definition: bits of the mantissa of an `f64`, so that 53 random bits
/// make a uniform number of the grid `k / 2^53`.
const MANTISSA_BITS: u32 = 53;

/// A uniform number in the open interval (0, 1).
pub(crate) fn uniform(rng: &mut ChaCha8Rng) -> f64 {
    let grid = 1.0 / (1u64 << MANTISSA_BITS) as f64;
    ((rng.next_u64() >> (u64::BITS - MANTISSA_BITS)) as f64 + 0.5) * grid
}

/// A standard normal number, by the Box-Muller transform (Box & Muller
/// 1958, Ann. Math. Stat. 29, 610), two uniforms per number: the same
/// arithmetic as the planet model's own copy (`planet::draws`), to be
/// unified with it later.
pub(crate) fn normal(rng: &mut ChaCha8Rng) -> f64 {
    let radius = (-2.0 * uniform(rng).ln()).sqrt();
    radius * (std::f64::consts::TAU * uniform(rng)).cos()
}

/// A number whose logarithm is uniform between `ln lo` and `ln hi`, in
/// `[lo, hi)`; one uniform.
pub(crate) fn log_uniform(rng: &mut ChaCha8Rng, lo: f64, hi: f64) -> f64 {
    (lo * (hi / lo).powf(uniform(rng))).clamp(lo, hi.max(lo))
}

/// A Rayleigh number of scale `sigma` (density `x/σ² exp(-x²/2σ²)`), by
/// its inverse CDF; one uniform.
pub(crate) fn rayleigh(rng: &mut ChaCha8Rng, sigma: f64) -> f64 {
    sigma * (-2.0 * uniform(rng).ln()).sqrt()
}

/// The absolute value of a normal number of mean 0 and scale `sigma`.
pub(crate) fn half_gaussian(rng: &mut ChaCha8Rng, sigma: f64) -> f64 {
    (sigma * normal(rng)).abs()
}

/// `exp(mu + sigma z)`, `z` standard normal: a log-normal number whose
/// median is `exp(mu)`.
pub(crate) fn log_normal(rng: &mut ChaCha8Rng, mu: f64, sigma: f64) -> f64 {
    (mu + sigma * normal(rng)).exp()
}

/// A Gamma number of `shape` and unit scale, by Marsaglia & Tsang 2000
/// (ACM TOMS 26, 363, section 4: `d = shape - 1/3`, `c = 1/sqrt(9d)`,
/// accept `d v` when `ln u < x²/2 + d - d v + d ln v`); below a shape of
/// 1, their section 6 boost: `Gamma(shape + 1) × u^(1/shape)`.
pub(crate) fn gamma(rng: &mut ChaCha8Rng, shape: f64) -> f64 {
    if shape < 1.0 {
        let boosted = gamma(rng, shape + 1.0);
        return boosted * uniform(rng).powf(1.0 / shape);
    }
    let d = shape - 1.0 / 3.0;
    let c = 1.0 / (9.0 * d).sqrt();
    loop {
        let x = normal(rng);
        let root = 1.0 + c * x;
        if root <= 0.0 {
            continue;
        }
        let v = root * root * root;
        if uniform(rng).ln() < 0.5 * x * x + d - d * v + d * v.ln() {
            return d * v;
        }
    }
}

/// A Beta(`a`, `b`) number, `X / (X + Y)` with `X`, `Y` Gamma of shapes
/// `a` and `b`, in the open interval (0, 1): a result that rounds to
/// exactly 0 or 1 is drawn again.
pub(crate) fn beta(rng: &mut ChaCha8Rng, a: f64, b: f64) -> f64 {
    loop {
        let x = gamma(rng, a);
        let y = gamma(rng, b);
        let value = x / (x + y);
        if value > 0.0 && value < 1.0 {
            return value;
        }
    }
}

/// One half of a Beta(`a`, `b`) law: below `median` (`upper` false) or
/// at and above it (`upper` true), by drawing the whole law again until
/// the number falls on the asked side (half the draws on average).
pub(crate) fn beta_half(rng: &mut ChaCha8Rng, a: f64, b: f64, median: f64, upper: bool) -> f64 {
    loop {
        let value = beta(rng, a, b);
        if (value >= median) == upper {
            return value;
        }
    }
}

/// A number of `[lo, hi)` whose density per logarithm is
/// `(x / brk)^below` under `brk` and `(x / brk)^above` from it on: a
/// broken power law in `dN/d ln x`, continuous at the break. Drawn by the
/// inverse CDF of each segment, the segment picked by its weight; two
/// uniforms. An empty support (`hi <= lo`) returns `lo`.
pub(crate) fn broken_power_law(
    rng: &mut ChaCha8Rng,
    brk: f64,
    below: f64,
    above: f64,
    lo: f64,
    hi: f64,
) -> f64 {
    let pick = uniform(rng);
    let within = uniform(rng);
    if hi <= lo {
        return lo;
    }
    // In u = ln(x / brk) the density is e^(k u) on each segment.
    let (start, end) = ((lo / brk).ln(), (hi / brk).ln());
    let low = (start < 0.0).then(|| (below, start, end.min(0.0)));
    let high = (end > 0.0).then(|| (above, start.max(0.0), end));
    let weight = |segment: Option<(f64, f64, f64)>| {
        segment.map_or(0.0, |(k, u1, u2)| exp_integral(k, u1, u2))
    };
    let (w_low, w_high) = (weight(low), weight(high));
    let segment = match (low, high) {
        (Some(low), Some(high)) => {
            if pick * (w_low + w_high) < w_low {
                low
            } else {
                high
            }
        }
        (Some(only), None) | (None, Some(only)) => only,
        (None, None) => return lo,
    };
    let (k, u1, u2) = segment;
    let u = if k == 0.0 {
        u1 + within * (u2 - u1)
    } else {
        let (e1, e2) = ((k * u1).exp(), (k * u2).exp());
        (e1 + within * (e2 - e1)).ln() / k
    };
    (brk * u.exp()).clamp(lo, hi)
}

/// `∫ e^(k u) du` from `u1` to `u2`.
fn exp_integral(k: f64, u1: f64, u2: f64) -> f64 {
    if k == 0.0 {
        u2 - u1
    } else {
        ((k * u2).exp() - (k * u1).exp()) / k
    }
}

#[cfg(test)]
#[path = "random_tests.rs"]
mod tests;
