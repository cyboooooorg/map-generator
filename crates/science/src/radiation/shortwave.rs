//! The shortwave: the sunlight a column absorbs, reflects and lets
//! through (ported from `legacy-final`'s `planet/shortwave.rs`).

use std::sync::LazyLock;

use super::fast_math;
use super::ktable::Table;
use super::spectrum::DIFFUSIVITY;
use super::starlight::{BANDS, band_edge_m};

/// The shortwave table of CO2's and water's lines
/// (`crates/science/data/shortwave.bin`, see [`super::ktable`]), with
/// MT_CKD 4.3's water and CO2 continua binned on its bins, checked
/// against the star's bands.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the shortwave paths of PR 2b-2 Task 4 read it")
)]
pub(crate) static SHORTWAVE: LazyLock<Table> = LazyLock::new(|| {
    let table = Table::parse(include_bytes!("../../data/shortwave.bin"))
        .expect("crates/science/data/shortwave.bin is a shortwave table of 8 g-points");
    assert!(
        !table.edges_cm.is_empty(),
        "shortwave.bin is a longwave table"
    );
    assert_eq!(table.water_continuum.len(), table.bins);
    let first = BANDS - table.bins;
    for (j, &edge) in table.edges_cm.iter().enumerate() {
        let lambda_m = band_edge_m(BANDS - j);
        assert!(
            (1e-2 / lambda_m / edge - 1.0).abs() < 1e-9,
            "the shortwave table's bins are not the star's bands"
        );
    }
    assert!(first > 0);
    table
});

/// Reflectance and transmission of a homogeneous layer of absorption
/// optical depth `absorption` and Rayleigh scattering optical depth
/// `scattering` (vertical), under diffuse light: the two-stream solution
/// of Meador & Weaver 1980 (*J. Atmos. Sci.* 37, 630,
/// doi:10.1175/1520-0469(1980)037<0630:TSATRT>2.0.CO;2, eqs. 18, 25 and
/// 26, quoted from the ECMWF ecRad code that implements them; the article
/// itself was not opened), `R = γ₂ sinh(λτ) / (λ cosh(λτ) + γ₁ sinh(λτ))`,
/// `T = λ / (λ cosh(λτ) + γ₁ sinh(λτ))`, `λ = √(γ₁² − γ₂²)`, with the
/// hemispheric-mean coefficients of Toon et al. 1989 (*JGR* 94, 16287,
/// doi:10.1029/JD094iD13p16287, Table 1; checked through Heng et al.
/// 2014, arXiv:1405.0026, eq. 67, and ecRad; the table itself not read) at the column's
/// diffusivity `D` = 1.66 in place of their 2: `γ₁ = D (1 − ω/2)`,
/// `γ₂ = D ω/2`, `ω` the single scattering albedo; Rayleigh's phase
/// function scatters half of the light back (`g` = 0). The same `D` as
/// the longwave column's ([`DIFFUSIVITY`], Elsasser's): a pure absorber
/// transmits `e^{−Dτ}`, and a pure scatterer reflects
/// `(D/2) τ / (1 + (D/2) τ)` (see `docs/science/radiative-transfer.md`,
/// "The shortwave half").
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the shortwave paths of PR 2b-2 Task 4 read it")
)]
pub(crate) fn layer(absorption: f64, scattering: f64) -> (f64, f64) {
    let tau = absorption.max(0.0) + scattering.max(0.0);
    if tau <= 0.0 {
        return (0.0, 1.0);
    }
    let d = DIFFUSIVITY;
    let omega = scattering.max(0.0) / tau;
    let (g1, g2) = (d * (1.0 - 0.5 * omega), d * 0.5 * omega);
    // λ = D √(1 − ω), from the absorption so as not to lose it to
    // rounding when the scattering dominates.
    let lambda = d * (absorption.max(0.0) / tau).sqrt();
    let x = lambda * tau;
    // cosh and sinh / λ, both times 2 e^{−x}, so that a thick layer does
    // not overflow.
    let e = (-x).exp();
    let cosh = 0.5 * (1.0 + e * e);
    let sinh_over = if x < 1e-8 {
        tau
    } else if x < 1e-2 {
        -(-2.0 * x).exp_m1() / (2.0 * lambda)
    } else {
        (1.0 - e * e) / (2.0 * lambda)
    };
    let denominator = cosh + g1 * sinh_over;
    (g2 * sinh_over / denominator, e / denominator)
}

/// [`layer`] in single precision with [`fast_math::exp`], for the many
/// layers of the shortwave paths (within 1e-6 of it).
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the shortwave paths of PR 2b-2 Task 4 read it")
)]
#[inline]
pub(crate) fn layer_fast(absorption: f32, scattering: f32) -> (f32, f32) {
    let tau = absorption.max(0.0) + scattering.max(0.0);
    if tau <= 0.0 {
        return (0.0, 1.0);
    }
    let d = DIFFUSIVITY as f32;
    let omega = scattering.max(0.0) / tau;
    let (g1, g2) = (d * (1.0 - 0.5 * omega), d * 0.5 * omega);
    let lambda = d * (absorption.max(0.0) / tau).sqrt();
    let x = lambda * tau;
    let e = fast_math::exp(-x);
    let cosh = 0.5 * (1.0 + e * e);
    // `1 − e^{−2x}` loses its digits for a thin layer: its series there.
    let sinh_over = if x < 1e-2 {
        tau * (1.0 - x * (1.0 - x * (2.0 / 3.0)))
    } else {
        (1.0 - e * e) / (2.0 * lambda)
    };
    let denominator = cosh + g1 * sinh_over;
    (g2 * sinh_over / denominator, e / denominator)
}

/// The layers above the ground, from the top down, stacked: their
/// reflectance to the light from above (`down`) and from below (`up`),
/// and their transmission.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Stack {
    pub(crate) down: f64,
    pub(crate) up: f64,
    pub(crate) through: f64,
}

impl Stack {
    /// Definition: nothing, a stack that reflects nothing and lets all
    /// the light through.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "the shortwave paths of PR 2b-2 Task 4 read it")
    )]
    pub(crate) const EMPTY: Stack = Stack {
        down: 0.0,
        up: 0.0,
        through: 1.0,
    };

    /// The stack with a symmetric layer of reflectance `r` and
    /// transmission `t` under it, the light reflected back and forth
    /// between them.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "the shortwave paths of PR 2b-2 Task 4 read it")
    )]
    pub(crate) fn over(self, r: f64, t: f64) -> Stack {
        let bounce = 1.0 / (1.0 - self.up * r);
        Stack {
            down: self.down + self.through * self.through * r * bounce,
            up: r + t * t * self.up * bounce,
            through: self.through * t * bounce,
        }
    }

    /// The stack with the stack `below` under it (the adding of two
    /// stacks; each transmits the same both ways).
    #[expect(dead_code, reason = "the shortwave paths of PR 2b-2 Task 4 read it")]
    pub(crate) fn then(self, below: Stack) -> Stack {
        let bounce = 1.0 / (1.0 - self.up * below.down);
        Stack {
            down: self.down + self.through * self.through * below.down * bounce,
            up: below.up + below.through * below.through * self.up * bounce,
            through: self.through * below.through * bounce,
        }
    }

    /// Over a ground of albedo `ground`: the share of the light reflected
    /// to space, and the share the ground absorbs.
    #[expect(dead_code, reason = "the shortwave paths of PR 2b-2 Task 4 read it")]
    pub(crate) fn on(self, ground: f64) -> (f64, f64) {
        let bounce = 1.0 / (1.0 - self.up * ground);
        (
            self.down + self.through * self.through * ground * bounce,
            self.through * (1.0 - ground) * bounce,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_is_on_the_albedo_bands() {
        let table = &*SHORTWAVE;
        assert_eq!(table.bins, 147);
        assert!((table.edges_cm[0] - 2000.0).abs() < 1e-6);
        assert!(table.edges_cm[table.bins] > 14_300.0);
        assert!((table.weights.iter().sum::<f64>() - 1.0).abs() < 1e-12);
    }

    /// The limits of the hemispheric-mean two-stream (the reference page,
    /// "The shortwave half", "In words"): a pure absorber transmits
    /// `e^{-D tau}` and reflects nothing; a pure scatterer (`omega` = 1,
    /// `lambda` = 0 in Meador & Weaver 1980, eqs. 25 and 26) reflects
    /// `(D/2) tau / (1 + (D/2) tau)` and loses nothing. With both, some of
    /// the light is lost, the solution is continuous as the absorption
    /// vanishes, and a thick layer neither overflows nor transmits.
    #[test]
    fn a_layer_absorbs_or_scatters_as_the_two_stream_limits_say() {
        // Pure absorption: e^{-D tau}, nothing reflected.
        let (r, t) = layer(0.7, 0.0);
        assert!(r.abs() < 1e-15, "{r}");
        assert!((t - (-DIFFUSIVITY * 0.7).exp()).abs() < 1e-12, "{t}");
        // Pure scattering: (D/2) tau / (1 + (D/2) tau), and nothing lost.
        for tau in [1e-6, 0.1, 3.0, 300.0] {
            let (r, t) = layer(0.0, tau);
            let a = 0.5 * DIFFUSIVITY * tau;
            assert!((r - a / (1.0 + a)).abs() < 1e-12, "{tau}: {r}");
            assert!((r + t - 1.0).abs() < 1e-12, "{tau}: {r} {t}");
        }
        // Both: some of each lost, continuous as the absorption vanishes,
        // and a thick layer neither overflows nor transmits.
        let (r, t) = layer(0.05, 1.0);
        assert!(r + t < 1.0 && r > 0.0 && t > 0.0, "{r} {t}");
        let (r0, t0) = layer(1e-13, 1.0);
        let (r1, t1) = layer(0.0, 1.0);
        assert!((r0 - r1).abs() < 1e-9 && (t0 - t1).abs() < 1e-9);
        let (r, t) = layer(1e3, 1e3);
        assert!(r.is_finite() && t < 1e-300 && r < 1.0, "{r} {t}");
    }

    /// [`layer_fast`] is [`layer`] in single precision, within 1e-6 of it
    /// over the depths the shortwave paths meet.
    #[test]
    fn the_fast_layer_is_the_layer() {
        for a in [0.0, 1e-7, 1e-4, 3e-3, 0.02, 0.5, 4.0, 80.0] {
            for sc in [0.0, 1e-6, 1e-3, 0.05, 1.0, 20.0] {
                let (r, t) = layer(a, sc);
                let (rf, tf) = layer_fast(a as f32, sc as f32);
                assert!((f64::from(rf) - r).abs() < 1e-6, "{a} {sc}: {rf} {r}");
                assert!((f64::from(tf) - t).abs() < 1e-6, "{a} {sc}: {tf} {t}");
            }
        }
    }

    /// The two-stream solution of a homogeneous layer is the adding of
    /// its halves (the adding method of the two-stream, Meador & Weaver
    /// 1980 eqs. 25 and 26 applied to each half and stacked).
    #[test]
    fn stacking_two_halves_gives_the_whole_layer() {
        for (a, s) in [(0.0, 0.8), (0.3, 0.0), (0.2, 0.5), (2.0, 0.1)] {
            let (r, t) = layer(a, s);
            let (rh, th) = layer(0.5 * a, 0.5 * s);
            let stack = Stack::EMPTY.over(rh, th).over(rh, th);
            assert!((stack.down - r).abs() < 1e-12, "{a} {s}");
            assert!((stack.up - r).abs() < 1e-12, "{a} {s}");
            assert!((stack.through - t).abs() < 1e-12, "{a} {s}");
        }
    }
}
