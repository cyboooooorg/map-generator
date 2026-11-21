//! Tier 1: the primordial envelope draw, by mass regime.

use super::envelope::{EnvelopeDraw, envelope_fraction};
use science::giants::core_bounds;
use science::units::EarthMass;
use timeline::Seed;

/// The seeds each mass is drawn with.
const SEEDS: u64 = 500;

fn draws(mass: f64) -> Vec<EnvelopeDraw> {
    (0..SEEDS)
        .map(|s| envelope_fraction(Seed::new(s), EarthMass(mass)).unwrap())
        .collect()
}

fn assert_within(mass: f64, low: f64, high: f64) {
    for d in draws(mass) {
        assert!(
            d.fraction >= low * (1.0 - 1e-9) && d.fraction <= high * (1.0 + 1e-9),
            "{mass} M_E: {} outside [{low}, {high}]",
            d.fraction
        );
    }
}

// Expected bounds: Lammer et al. 2014 Tables 1 to 3 (arXiv:1401.2765), read
// in `draw-bounds.md` section 4a, capped at 0.20 (the Lopez & Fortney 2014
// tables' edge); Lopez & Fortney 2014 and Helled et al. 2011 for the
// regimes above (sections 4b, 4c).
#[test]
fn envelope_bounds_by_mass() {
    assert_within(0.05, 0.0, 0.0);
    assert_within(0.1, 4.4e-8, 1.0e-4);
    assert_within(1.0, 8.1e-6, 0.025);
    assert_within(3.0, 2.0e-4, 0.20);
    assert_within(5.0, 8.7e-4, 0.20);
    assert_within(7.0, 8.7e-4, 0.20);
    assert_within(10.0, 0.09, 0.20);
    assert_within(15.0, 0.09, 0.20);
    assert_within(19.9, 0.09, 0.20);
}

#[test]
fn between_the_grid_masses_the_bounds_stay_between_the_neighbours() {
    // 0.2 M_E lies between the 0.1 and 0.5 nodes: its bounds lie between
    // theirs (log-linear interpolation, monotone nodes).
    assert_within(0.2, 4.4e-8, 5.9e-3);
    assert_within(2.5, 6.1e-5, 0.20);
}

#[test]
fn a_gas_giant_core_stays_in_the_tables() {
    // Thorngren et al. 2016 (arXiv:1511.07854), Sect. V.1 p. 9: the
    // one-sigma scatter around the fit is the factor 10^sigma = 1.82; the
    // draw spans two sigma either side, 1.82^2.
    let band = 1.82_f64.powi(2);
    for mass in [20.0, 50.0, 318.0, 3_000.0] {
        let (low, high) = core_bounds(EarthMass(mass)).unwrap();
        // Thorngren et al. 2016, Mz = 57.9 (M / M_J)^0.61 M_E, M_J = 317.83.
        let fit = 57.9 * (mass / 317.828_4).powf(0.61);
        for d in draws(mass) {
            let z = d.heavy_mass.unwrap().0;
            assert!(
                z >= low.0 * (1.0 - 1e-9) && z <= high.0 * (1.0 + 1e-9),
                "{mass}: {z}"
            );
            assert!(z >= fit / band * (1.0 - 1e-9) && z <= fit * band * (1.0 + 1e-9));
            let envelope = 1.0 - z / mass;
            assert!((d.fraction - envelope).abs() < 1e-12);
            assert!(d.fraction > 0.0 && d.fraction < 1.0);
        }
    }
}

#[test]
fn only_a_gas_giant_has_a_heavy_mass() {
    for mass in [0.05, 1.0, 7.0, 15.0, 19.9] {
        assert!(draws(mass).iter().all(|d| d.heavy_mass.is_none()));
    }
    assert!(draws(20.0).iter().all(|d| d.heavy_mass.is_some()));
}

#[test]
fn the_fraction_is_monotone_in_its_uniform() {
    // One uniform per call in every regime: the draws of two masses are
    // monotone in it, within a regime and across regimes.
    // The fraction is a monotone function of the single uniform within a
    // regime: seeds ordered by the 5 to 10 M_E draw are ordered the same
    // way at another mass of that regime.
    let mut pairs: Vec<(f64, f64)> = (0..SEEDS)
        .map(|s| {
            let a = envelope_fraction(Seed::new(s), EarthMass(6.0))
                .unwrap()
                .fraction;
            let b = envelope_fraction(Seed::new(s), EarthMass(9.0))
                .unwrap()
                .fraction;
            (a, b)
        })
        .collect();
    pairs.sort_by(|x, y| x.0.total_cmp(&y.0));
    assert!(pairs.windows(2).all(|w| w[0].1 <= w[1].1));
    // The same holds across regimes: the uniform behind the 15 M_E draw
    // (uniform 0.09 to 0.20) orders the 7 M_E draw the same way.
    let mut across: Vec<(f64, f64)> = (0..SEEDS)
        .map(|s| {
            let a = envelope_fraction(Seed::new(s), EarthMass(7.0))
                .unwrap()
                .fraction;
            let b = envelope_fraction(Seed::new(s), EarthMass(15.0))
                .unwrap()
                .fraction;
            (a, b)
        })
        .collect();
    across.sort_by(|x, y| x.0.total_cmp(&y.0));
    assert!(across.windows(2).all(|w| w[0].1 <= w[1].1));
}

#[test]
fn the_draw_follows_its_seed() {
    let a = envelope_fraction(Seed::new(1), EarthMass(318.0)).unwrap();
    let b = envelope_fraction(Seed::new(2), EarthMass(318.0)).unwrap();
    assert_ne!(a, b);
}

#[test]
fn above_the_highest_core_the_heavy_mass_is_that_core() {
    // At 6 000 M_E (18.9 M_J, inside Thorngren et al. 2016's 20 M_J), the
    // fit's low end, 57.9 (6000 / 317.83)^0.61 / 1.82^2 = 104.9 M_E, lies
    // above the tables' highest core, 100 M_E (Fortney et al. 2007): the
    // heavy-element mass is that core, and the draw never fails.
    let (_, high) = core_bounds(EarthMass(6_000.0)).unwrap();
    assert_eq!(high.0, 100.0);
    for d in draws(6_000.0) {
        assert_eq!(d.heavy_mass, Some(high));
        assert!((d.fraction - (1.0 - 100.0 / 6_000.0)).abs() < 1e-12);
    }
}
