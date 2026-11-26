//! Tests of the `Modern` methane against study 37's sources
//! (`docs/science/life-gases.md`).

use super::*;
use crate::constants::EARTH_MASS_PARAMETER;
use crate::radiation::gases::{AR_MOLAR_MASS, CH4_MOLAR_MASS, N2_MOLAR_MASS, O2_MOLAR_MASS};

/// Source: Earth's surface gravity as study 37 takes it, 9.798 m/s²:
/// [`EARTH_MASS_PARAMETER`] over [`EARTH_EQUATORIAL_RADIUS`] squared.
fn earth_gravity() -> MetrePerSecondSquared {
    MetrePerSecondSquared(EARTH_MASS_PARAMETER / EARTH_EQUATORIAL_RADIUS.0.powi(2))
}

/// Source: the reference Earth's dry air by weight, bar, that study 37
/// converts the law's weight to moles in: 0.746 of N₂ (Trenberth & Smith
/// 2005's 5.1352e18 kg of dry air, study 23), 0.228 of O₂ and 0.01294 of
/// argon (the chain's reference Earth, `life-gases.md`, "Effect on
/// Earth").
const REFERENCE_AIR: [(f64, f64); 3] = [
    (0.746, N2_MOLAR_MASS),
    (0.228, O2_MOLAR_MASS),
    (0.012_94, AR_MOLAR_MASS),
];

/// Source: Prather et al. 2012's 1750 tropospheric methane, 700 ± 25
/// ppb, times their fill factor, as a column mean like for like with the
/// law: 680 ± 24 ppb (study 37, "Inputs, options and what must be
/// calibrated"), ppb.
const PRATHER_COLUMN_PPB: (f64, f64) = (680.0, 24.0);

/// Source: study 37's derivation of the law around the Sun on Earth,
/// 3.68e-7 bar by weight (202 Tg/yr over 5.112e14 m², × 9.5 yr × 9.798
/// m/s²), to its three digits.
const STUDY_37_BAR: f64 = 3.68e-7;

/// Source: the K2V and K7V hosts of Rugheimer & Kaltenegger 2018's grid,
/// 5000 and 4250 K (Table 1, as study 37 reads it).
const K2V_K: f64 = 5000.0;
/// Source: see [`K2V_K`].
const K7V_K: f64 = 4250.0;
/// Definition: halfway between [`K2V_K`] and [`K7V_K`].
const K_MIDWAY_K: f64 = 0.5 * (K2V_K + K7V_K);

/// Moles of methane per mole of the reference air, ppb, of `methane`
/// bar by weight in it.
fn ppb_in_reference_air(methane: Bar) -> f64 {
    let air: f64 = REFERENCE_AIR.iter().map(|(w, m)| w / m).sum();
    methane.0 / CH4_MOLAR_MASS / air * 1e9
}

#[test]
fn earths_area_is_study_37s() {
    // Source: study 37, 4 π R² of the IAU radius, 5.112e14 m².
    assert!(
        (earth_area_m2() / 5.112e14 - 1.0).abs() < 1e-3,
        "{}",
        earth_area_m2()
    );
}

#[test]
fn modern_methane_around_the_sun_is_prathers_column() {
    let sun = [(SOLAR_EFFECTIVE_TEMPERATURE, EarthFlux(1.0))];
    let p = modern_methane(&sun, earth_gravity());
    let ppb = ppb_in_reference_air(p);
    println!("{:.4e} bar by weight, {ppb:.1} ppb", p.0);
    assert!((p.0 / STUDY_37_BAR - 1.0).abs() < 5e-3, "{p:?}");
    let (mid, sigma) = PRATHER_COLUMN_PPB;
    assert!(
        (ppb - mid).abs() <= sigma,
        "{ppb} ppb against {mid} ± {sigma}"
    );
}

#[test]
fn methane_scales_with_gravity_and_vanishes_without_it() {
    let sun = [(SOLAR_EFFECTIVE_TEMPERATURE, EarthFlux(1.0))];
    let g = earth_gravity();
    let twice = modern_methane(&sun, MetrePerSecondSquared(2.0 * g.0)).0;
    assert!((twice / modern_methane(&sun, g).0 - 2.0).abs() < 1e-12);
    assert_eq!(modern_methane(&sun, MetrePerSecondSquared(0.0)).0, 0.0);
}

#[test]
fn host_factor_is_the_grids_ratio_at_its_points() {
    // Source: study 37, calibration 1, the ratios to three digits: RK2018
    // over its Sun's 1.73 ppm (F0V, F7V, Sun, G8V, K2V, K7V), then
    // Rugheimer et al. 2015a's active M1, M3, M8 over its Sun's 1.6 ppm.
    let ratios = [
        (7000.0, 2.07),
        (6250.0, 1.42),
        (5772.0, 1.0),
        (5500.0, 243.0),
        (5000.0, 262.0),
        (4250.0, 72.3),
        (3600.0, 231.0),
        (3200.0, 356.0),
        (2400.0, 625.0),
    ];
    for (t, ratio) in ratios {
        let r = methane_host_factor(Kelvin(t));
        assert!((r / ratio - 1.0).abs() < 5e-3, "{t} K: {r} against {ratio}");
    }
    // Held past the grid's ends.
    assert_eq!(
        methane_host_factor(Kelvin(9000.0)),
        methane_host_factor(Kelvin(7000.0))
    );
    assert_eq!(
        methane_host_factor(Kelvin(2000.0)),
        methane_host_factor(Kelvin(2400.0))
    );
}

#[test]
fn host_factor_is_log_linear_between_points() {
    // Halfway between K2V and K7V in temperature is the geometric mean of
    // their ratios.
    let mid = methane_host_factor(Kelvin(K_MIDWAY_K));
    let geometric =
        (methane_host_factor(Kelvin(5000.0)) * methane_host_factor(Kelvin(4250.0))).sqrt();
    assert!(
        (mid / geometric - 1.0).abs() < 1e-12,
        "{mid} against {geometric}"
    );
}

#[test]
fn several_stars_add_their_loss_rates_by_light() {
    let sun = SOLAR_EFFECTIVE_TEMPERATURE;
    let k2 = Kelvin(K2V_K);
    // Two Suns are one.
    let two = methane_lifetime(&[(sun, EarthFlux(0.5)), (sun, EarthFlux(0.5))]).0;
    assert!((two - EARTH_METHANE_LIFETIME.0).abs() < 1e-12);
    // Definition (calibration 2): 1/τ = Σ wᵢ/τᵢ.
    let tau = methane_lifetime(&[(sun, EarthFlux(1.0)), (k2, EarthFlux(1.0))]).0;
    let expected = 1.0 / (0.5 / 9.5 + 0.5 / (9.5 * methane_host_factor(k2)));
    assert!(
        (tau / expected - 1.0).abs() < 1e-12,
        "{tau} against {expected}"
    );
    // An unlit star counts for nothing; no light at all keeps Earth's.
    let unlit = methane_lifetime(&[(sun, EarthFlux(1.0)), (k2, EarthFlux(0.0))]).0;
    assert!((unlit - 9.5).abs() < 1e-12);
    assert_eq!(methane_lifetime(&[]).0, 9.5);
}
