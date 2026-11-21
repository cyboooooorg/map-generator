use super::*;
use rand_core::SeedableRng;

/// Definition: draws of each statistical test; their tolerances are about
/// four standard errors of this many draws.
const DRAWS: u32 = 50_000;

fn rng(seed: u64) -> ChaCha8Rng {
    ChaCha8Rng::seed_from_u64(seed)
}

fn mean(values: impl Iterator<Item = f64>) -> f64 {
    let (sum, n) = values.fold((0.0, 0u32), |(s, n), v| (s + v, n + 1));
    sum / f64::from(n)
}

#[test]
fn beta_matches_its_mean() {
    for (seed, (a, b)) in [(0.60, 1.45), (1.06, 3.68), (0.5, 0.5), (2.0, 5.0)]
        .into_iter()
        .enumerate()
    {
        let mut rng = rng(seed as u64);
        let drawn = mean((0..DRAWS).map(|_| beta(&mut rng, a, b)));
        assert!(
            (drawn - a / (a + b)).abs() < 0.005,
            "Beta({a}, {b}): {drawn}"
        );
    }
}

#[test]
fn beta_draws_stay_inside_the_open_interval() {
    let mut rng = rng(11);
    for _ in 0..DRAWS {
        let x = beta(&mut rng, 0.3, 0.3);
        assert!(x > 0.0 && x < 1.0, "{x}");
    }
}

#[test]
fn beta_halves_split_at_the_median() {
    // Beta(1.06, 3.68), median 0.1834 (numeric CDF).
    let (a, b, median) = (1.06, 3.68, 0.1834);
    let mut rng = rng(12);
    let below = (0..DRAWS).filter(|_| beta(&mut rng, a, b) < median).count();
    let share = below as f64 / f64::from(DRAWS);
    // Standard error of a half share over 50 000 draws: 0.0022.
    assert!((share - 0.5).abs() < 0.009, "{share}");
    for _ in 0..DRAWS / 10 {
        assert!(beta_half(&mut rng, a, b, median, false) < median);
        assert!(beta_half(&mut rng, a, b, median, true) >= median);
    }
}

#[test]
fn gamma_matches_its_mean() {
    for (seed, shape) in [0.6, 1.0, 3.68].into_iter().enumerate() {
        let mut rng = rng(20 + seed as u64);
        let drawn = mean((0..DRAWS).map(|_| gamma(&mut rng, shape)));
        // Standard error sqrt(shape / n): 0.0086 at 3.68.
        assert!((drawn - shape).abs() < 4.0 * (shape / f64::from(DRAWS)).sqrt());
    }
}

#[test]
fn normal_has_unit_moments() {
    let mut rng = rng(30);
    let values: Vec<f64> = (0..DRAWS).map(|_| normal(&mut rng)).collect();
    let m = mean(values.iter().copied());
    let var = mean(values.iter().map(|v| (v - m) * (v - m)));
    assert!(m.abs() < 0.018, "{m}");
    assert!((var - 1.0).abs() < 0.025, "{var}");
}

#[test]
fn rayleigh_and_half_gaussian_match_their_means() {
    let mut rng = rng(31);
    let sigma = 0.32;
    let r = mean((0..DRAWS).map(|_| rayleigh(&mut rng, sigma)));
    // Rayleigh mean σ sqrt(π/2), standard deviation 0.655 σ.
    let expected = sigma * (std::f64::consts::PI / 2.0).sqrt();
    assert!((r - expected).abs() < 4.0 * 0.655 * sigma / f64::from(DRAWS).sqrt());
    let h = mean((0..DRAWS).map(|_| half_gaussian(&mut rng, sigma)));
    // Half-Gaussian mean σ sqrt(2/π), standard deviation 0.603 σ.
    let expected = sigma * (2.0 / std::f64::consts::PI).sqrt();
    assert!((h - expected).abs() < 4.0 * 0.603 * sigma / f64::from(DRAWS).sqrt());
}

#[test]
fn log_uniform_spreads_evenly_in_log() {
    let mut rng = rng(32);
    let (lo, hi) = (1.0, 100.0);
    let below = (0..DRAWS)
        .map(|_| log_uniform(&mut rng, lo, hi))
        .inspect(|x| assert!((lo..hi).contains(x), "{x}"))
        .filter(|x| *x < 10.0)
        .count();
    let share = below as f64 / f64::from(DRAWS);
    assert!((share - 0.5).abs() < 0.009, "{share}");
}

#[test]
fn log_normal_median_is_exp_mu() {
    let mut rng = rng(33);
    let mut values: Vec<f64> = (0..DRAWS).map(|_| log_normal(&mut rng, 1.0, 0.5)).collect();
    values.sort_by(f64::total_cmp);
    let median = values[values.len() / 2];
    // Standard error of the median in ln: 1.2533 σ / sqrt(n) = 0.0028.
    assert!((median.ln() - 1.0).abs() < 0.012, "{median}");
}

#[test]
fn broken_power_law_stays_in_its_bounds_and_follows_its_slopes() {
    let mut rng = rng(34);
    let (brk, below, above, lo, hi): (f64, f64, f64, f64, f64) = (12.0, 1.6, -0.9, 0.5, 640.0);
    // Share below the break: the integrals of e^(1.6 u) over
    // [ln(0.5/12), 0] and of e^(-0.9 u) over [0, ln(640/12)].
    let w_low = (1.0 - (below * (lo / brk).ln()).exp()) / below;
    let w_high = ((above * (hi / brk).ln()).exp() - 1.0) / above;
    let expected = w_low / (w_low + w_high);
    let below_break = (0..DRAWS)
        .map(|_| broken_power_law(&mut rng, brk, below, above, lo, hi))
        .inspect(|x| assert!(*x >= lo && *x < hi, "{x}"))
        .filter(|x| *x < brk)
        .count();
    let share = below_break as f64 / f64::from(DRAWS);
    assert!((share - expected).abs() < 0.009, "{share} vs {expected}");
}

#[test]
fn broken_power_law_handles_one_sided_and_empty_supports() {
    let mut rng = rng(35);
    for _ in 0..1_000 {
        let x = broken_power_law(&mut rng, 12.0, 1.6, -0.9, 20.0, 30.0);
        assert!((20.0..30.0).contains(&x));
        let x = broken_power_law(&mut rng, 12.0, 1.6, -0.9, 2.0, 5.0);
        assert!((2.0..5.0).contains(&x));
        let x = broken_power_law(&mut rng, 3.6, 0.0, -0.86, 1.0, 3.0);
        assert!((1.0..3.0).contains(&x));
    }
    assert_eq!(broken_power_law(&mut rng, 12.0, 1.6, -0.9, 5.0, 5.0), 5.0);
    assert_eq!(broken_power_law(&mut rng, 12.0, 1.6, -0.9, 7.0, 5.0), 7.0);
}
