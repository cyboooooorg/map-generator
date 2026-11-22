//! The k-distribution of a bin: its absorption coefficients sorted, and
//! reduced to a few g-points (Lacis & Oinas 1991, *JGR* 96, 9027).
//!
//! Within a bin the order of the wavenumbers does not matter to a mean
//! transmission, only how much of the bin has each coefficient: sorted,
//! the coefficients form a smooth increasing function `k(g)` of the
//! cumulative share `g` from 0 to 1, and the mean transmission of a path
//! `u` is `∫ exp(−k(g) u) dg`. The g-points split `[0, 1]` into
//! [`BOUNDS`]; each takes the mean coefficient of its interval, with the
//! interval's width as its weight. The intervals narrow towards `g` = 1,
//! where the line cores make `k(g)` steep.

/// Number of g-points per bin.
pub const G: usize = 8;

/// Edges of the g-intervals.
pub const BOUNDS: [f64; G + 1] = [0.0, 0.3, 0.6, 0.8, 0.9, 0.96, 0.99, 0.998, 1.0];

/// Weight of each g-point: its interval's width (science reads them from
/// the table's edges).
#[cfg(test)]
pub fn weights() -> [f64; G] {
    std::array::from_fn(|i| BOUNDS[i + 1] - BOUNDS[i])
}

/// Sorts `samples` (equally spaced coefficients of one bin) and returns
/// the mean coefficient of each g-interval; a sample that straddles an
/// edge is shared in proportion.
pub fn reduce(samples: &mut [f64]) -> [f64; G] {
    samples.sort_unstable_by(f64::total_cmp);
    let n = samples.len() as f64;
    std::array::from_fn(|i| {
        let (a, b) = (BOUNDS[i] * n, BOUNDS[i + 1] * n);
        let mut sum = 0.0;
        let mut m = a.floor() as usize;
        while m < samples.len() && b > m as f64 {
            // The share of sample m (covering [m, m + 1)) inside [a, b).
            let share = (b.min(m as f64 + 1.0) - a.max(m as f64)).max(0.0);
            sum += share * samples[m];
            m += 1;
        }
        if b > a { sum / (b - a) } else { 0.0 }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lbl::tests::synthetic;
    use crate::lbl::{Cutoff, bin_spectrum};

    /// cm²/molecule to m²/kg for CO₂ (44.009 g/mol).
    const TO_MASS: f64 = 1e-4 * 6.022_140_76e23 / 44.009e-3;

    #[test]
    fn the_g_points_reproduce_the_line_by_line_transmission() {
        let cutoff = Cutoff {
            distance_cm: 25.0,
            pedestal: crate::lbl::Pedestal::None,
        };
        let w = weights();
        for lorentz in [0.07, 7e-3, 1e-4] {
            let lines = synthetic(150, 600.0, 620.0, lorentz, 6e-4);
            let mut k: Vec<f64> = bin_spectrum(&lines, 600.0, 620.0, cutoff, 4_000_000)
                .into_iter()
                .map(|x| x * TO_MASS)
                .collect();
            let lbl = |u: f64| k.iter().map(|k| (-k * u).exp()).sum::<f64>() / k.len() as f64;
            let paths: Vec<f64> = (0..=24)
                .map(|i| 1e-4 * 10f64.powf(i as f64 / 4.0))
                .collect();
            let exact: Vec<f64> = paths.iter().map(|&u| lbl(u)).collect();
            let g = reduce(&mut k);
            let mut worst = 0.0f64;
            for (u, t) in paths.iter().zip(&exact) {
                let ck: f64 = g.iter().zip(&w).map(|(k, w)| w * (-k * u).exp()).sum();
                worst = worst.max((ck - t).abs());
                assert!(
                    (ck - t).abs() <= 0.02,
                    "Lorentz {lorentz}, path {u:e} kg/m²: {ck:.4} against {t:.4}"
                );
            }
            println!("Lorentz {lorentz} cm⁻¹: largest error {worst:.4}");
        }
    }

    #[test]
    fn reduce_takes_interval_means() {
        let mut samples: Vec<f64> = (0..1000).rev().map(|i| i as f64).collect();
        let g = reduce(&mut samples);
        // [0, 0.3) holds 0..300: mean 149.5.
        assert!((g[0] - 149.5).abs() < 1e-9, "{g:?}");
        assert!((g[G - 1] - 998.5).abs() < 1e-9, "{g:?}");
        assert!((weights().iter().sum::<f64>() - 1.0).abs() < 1e-15);
    }
}
