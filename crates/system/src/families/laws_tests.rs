use super::*;
use crate::families::tests::{ALL, context};
use rand_core::SeedableRng;
use science::orbit::{Pair, mutual_hill_spacing};
use science::units::{AstronomicalUnit, Day, Fraction, SolarMass};

/// Definition: draws of each statistical test; their tolerances are about
/// four standard errors of this many draws.
const DRAWS: u32 = 50_000;

fn rng(seed: u64) -> ChaCha8Rng {
    ChaCha8Rng::seed_from_u64(seed)
}

/// `∫0^x t^(a-1) (1-t)^(b-1) dt` by Simpson's rule after the substitution
/// `t = s^(1/a)`, which removes the singularity at 0 for `a < 1`.
fn incomplete_beta(a: f64, b: f64, x: f64) -> f64 {
    let steps = 20_000;
    let end = x.powf(a);
    let h = end / f64::from(steps);
    let f = |s: f64| (1.0 - s.powf(1.0 / a)).max(0.0).powf(b - 1.0) / a;
    let inner: f64 = (1..steps)
        .map(|i| f(f64::from(i) * h) * if i % 2 == 1 { 4.0 } else { 2.0 })
        .sum();
    (f(0.0) + inner + f(end)) * h / 3.0
}

#[test]
fn the_medians_are_the_constants() {
    for ((a, b), median) in [
        (SINGLE_GIANT_BETA, SINGLE_GIANT_BETA_MEDIAN),
        (MULTI_GIANT_BETA, MULTI_GIANT_BETA_MEDIAN),
    ] {
        let cdf = incomplete_beta(a, b, median) / incomplete_beta(a, b, 1.0);
        // The constants carry three decimals: 0.0005 of the variable is
        // at most 0.0012 of probability at these densities.
        assert!(
            (cdf - 0.5).abs() < 0.002,
            "Beta({a}, {b}) at {median}: {cdf}"
        );
    }
}

#[test]
fn innermost_period_peaks_near_12_days() {
    let mut rng = rng(40);
    // Bins of 0.1 dex from 0.1 d.
    let mut bins = [0u32; 40];
    for _ in 0..DRAWS {
        let period = innermost_period(&mut rng, Day(0.1), Day(640.0));
        assert!(period.0 >= 0.1 && period.0 < 640.0, "{period:?}");
        let bin = ((period.0 / 0.1).log10() / 0.1) as usize;
        bins[bin.min(39)] += 1;
    }
    let peak = (0..bins.len()).max_by_key(|i| bins[*i]).unwrap();
    let low = 0.1 * 10f64.powf(0.1 * peak as f64);
    let high = low * 10f64.powf(0.1);
    assert!(
        low <= INNERMOST_PEAK.0 && INNERMOST_PEAK.0 < high,
        "peak bin {low}-{high} d"
    );
}

#[test]
fn compact_ratios_stay_above_the_wall() {
    let mut rng = rng(41);
    let mut near = 0;
    let mut wide = Vec::new();
    for _ in 0..DRAWS {
        let (ratio, resonant) = compact_ratio(&mut rng);
        assert!(ratio.is_finite() && ratio >= WEISS_WALL, "{ratio}");
        if resonant {
            near += 1;
            assert!(
                RESONANCES
                    .iter()
                    .any(|r| ratio >= *r && ratio < r * (1.0 + NEAR_RESONANCE_WINDOW)),
                "{ratio}"
            );
        } else {
            wide.push(ratio);
        }
    }
    let share = f64::from(near) / f64::from(DRAWS);
    // Standard error of a 0.29 share over 50 000 draws: 0.0020.
    assert!((share - NEAR_RESONANCE_CHANCE).abs() < 0.008, "{share}");
    // EPOS I: log D = -0.39 gives a median ratio of 1.86 (S 1.5), barely
    // moved by the redraws below the wall (0.2 % of draws).
    wide.sort_by(f64::total_cmp);
    let median = wide[wide.len() / 2];
    assert!((median - 1.86).abs() < 0.02, "{median}");
}

#[test]
fn neil_rogers_median_is_2_7() {
    let mut rng = rng(42);
    let mut masses: Vec<f64> = (0..DRAWS)
        .map(|_| log_normal(&mut rng, NEIL_ROGERS_MU, NEIL_ROGERS_SIGMA))
        .collect();
    masses.sort_by(f64::total_cmp);
    let median = masses[masses.len() / 2];
    // Standard error of the median: 1.2533 σ / sqrt(n) = 0.0092 in ln M,
    // 0.025 M⊕; four of them are 0.1.
    assert!((median - 1f64.exp()).abs() < 0.1, "{median}");
    for _ in 0..DRAWS / 10 {
        let mass = small_planet_mass(&mut rng);
        assert!(mass.0 >= SMALL_MASS_MIN.0 && mass.0 < SMALL_MASS_MAX.0);
    }
}

#[test]
fn fulton_axis_scales_with_the_ice_line() {
    for seed in 0..200 {
        let sun = fulton_axis(&mut rng(seed), AstronomicalUnit(2.7));
        let twice = fulton_axis(&mut rng(seed), AstronomicalUnit(5.4));
        assert!((twice.0 / sun.0 - 2.0).abs() < 1e-12, "{sun:?} {twice:?}");
        assert!(sun.0 >= 2.7 && sun.0 < FULTON_OUTER_SUPPORT.0, "{sun:?}");
    }
}

#[test]
fn fulton_axis_rises_to_its_break() {
    let mut rng = rng(43);
    let axes: Vec<f64> = (0..DRAWS)
        .map(|_| fulton_axis(&mut rng, AstronomicalUnit(2.7)).0)
        .collect();
    // Shares per ln a: ∫ e^(0.72 u) over [ln(2.7/3.6), 0] and
    // ∫ e^(-0.86 u) over [0, ln(30/3.6)].
    let low = (1.0 - (FULTON_INSIDE * (2.7f64 / 3.6).ln()).exp()) / FULTON_INSIDE;
    let high = ((FULTON_BEYOND * (30.0f64 / 3.6).ln()).exp() - 1.0) / FULTON_BEYOND;
    let expected = low / (low + high);
    let share = axes.iter().filter(|a| **a < 3.6).count() as f64 / f64::from(DRAWS);
    assert!((share - expected).abs() < 0.008, "{share} vs {expected}");
}

#[test]
fn next_giants_stay_within_their_ratio() {
    let mut rng = rng(44);
    for _ in 0..DRAWS / 10 {
        let a = next_giant_axis(&mut rng, AstronomicalUnit(5.0));
        let ratio = a.0 / 5.0;
        assert!(ratio >= 0.5f64.exp() && ratio < 10.0, "{ratio}");
    }
}

#[test]
fn next_rocky_planets_stay_inside_their_limit() {
    let mut rng = rng(45);
    let host = SolarMass(1.0);
    let mut spacings = Vec::new();
    for _ in 0..DRAWS / 10 {
        let a = next_rocky_axis(
            &mut rng,
            (EarthMass(1.0), AstronomicalUnit(0.5)),
            EarthMass(1.0),
            host,
            AstronomicalUnit(2.7),
        );
        assert!(a.0 > 0.5 && a.0 < 2.7, "{a:?}");
        let pair = |a| Pair {
            mass: EarthMass(1.0),
            a,
            e: Fraction::new(0.0).unwrap(),
        };
        let delta = mutual_hill_spacing(pair(AstronomicalUnit(0.5)), pair(a), host);
        assert!(delta >= HILL_OFFSET - 1e-9, "{delta}");
        spacings.push(delta);
    }
    // Far from the limit (0.5 to 2.7 au is 130 Hill radii for two Earths),
    // the truncation barely bites: the mean is 3.5 + 14.5 sqrt(π/2) = 21.7
    // (Fang & Margot), standard error 9.5 / sqrt(5000) = 0.13.
    let mean = spacings.iter().sum::<f64>() / spacings.len() as f64;
    assert!((mean - 21.7).abs() < 0.6, "{mean}");
}

#[test]
fn a_rocky_planet_with_no_room_goes_at_the_offset() {
    let mut rng = rng(46);
    let host = SolarMass(1.0);
    let a = next_rocky_axis(
        &mut rng,
        (EarthMass(1.0), AstronomicalUnit(2.69)),
        EarthMass(1.0),
        host,
        AstronomicalUnit(2.7),
    );
    let pair = |a| Pair {
        mass: EarthMass(1.0),
        a,
        e: Fraction::new(0.0).unwrap(),
    };
    let delta = mutual_hill_spacing(pair(AstronomicalUnit(2.69)), pair(a), host);
    assert!((delta - HILL_OFFSET).abs() < 1e-9, "{a:?}: {delta}");
}

#[test]
fn eccentricities_stay_below_one() {
    use timeline::PlanetClass::*;
    let mut rng = rng(47);
    for family in ALL {
        for class in [
            Rocky,
            SuperEarth,
            MiniNeptune,
            IceGiant,
            GasGiant,
            HotJupiter,
        ] {
            for giants in [0, 1, 2] {
                for stirred in [false, true] {
                    let rules = context(family, giants, stirred);
                    for _ in 0..500 {
                        let e = eccentricity(&rules, class, &mut rng);
                        assert!((0.0..1.0).contains(&e), "{family} {class:?}: {e}");
                    }
                }
            }
        }
    }
}

#[test]
fn giant_eccentricities_take_their_family_half() {
    use timeline::PlanetClass::GasGiant;
    let mut rng = rng(48);
    let rules = context;
    for _ in 0..2_000 {
        let calm = rules(Family::SolarLike, 3, false);
        assert!(eccentricity(&calm, GasGiant, &mut rng) < MULTI_GIANT_BETA_MEDIAN);
        let stirred = rules(Family::SolarLike, 3, true);
        assert!(eccentricity(&stirred, GasGiant, &mut rng) >= MULTI_GIANT_BETA_MEDIAN);
        let single = rules(Family::EccentricGiants, 1, false);
        assert!(eccentricity(&single, GasGiant, &mut rng) >= SINGLE_GIANT_BETA_MEDIAN);
        let pair = rules(Family::EccentricGiants, 2, false);
        assert!(eccentricity(&pair, GasGiant, &mut rng) >= MULTI_GIANT_BETA_MEDIAN);
        let hot = rules(Family::HotJupiter, 2, false);
        assert!(eccentricity(&hot, timeline::PlanetClass::HotJupiter, &mut rng) < 0.1);
    }
}

#[test]
fn the_k50_scale_stays_in_the_fit() {
    for family in ALL {
        let sigma = k50_sigma_e(&context(family, 1, false));
        assert!((0.0..=K50_SIGMA_E_MAX).contains(&sigma));
        let expected = if family == Family::Compact {
            0.03
        } else {
            0.05
        };
        assert_eq!(sigma, expected, "{family}");
    }
}
