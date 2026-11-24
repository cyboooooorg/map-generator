//! Tier 1: the body's albedo, the wet balance and the collapse of a CO₂
//! the surface cannot keep.

use super::body::ClimateBody;
use super::fixture::{earth, moved};
use super::wet::{collapsed_climate, wet_balance};
use super::{COLLAPSE_ITERATIONS, COLLAPSE_TOLERANCE, Light, Numerics};
use science::balance::EARTH_DRY_AIR;
use science::radiation::albedo::{ShortwaveCache, shortwave_budget};
use science::radiation::gases::PartialPressures;
use science::surface::{ICE_COVERED_TEMPERATURE, surface_vapour};
use science::units::{Bar, EarthFlux, Fraction, Kelvin};
use science::volatiles::co2_saturation_pressure;

/// Definition: the Bond albedo the tests take the light under, 0.3, a
/// test input (Earth's is 0.306, NASA Earth fact sheet).
pub(super) const TEST_ALBEDO: f64 = 0.3;

/// Source: Earth's average surface temperature, 288 K (NASA Earth fact
/// sheet, nssdc.gsfc.nasa.gov/planetary/factsheet/earthfact.html).
pub(super) const EARTH_SURFACE: Kelvin = Kelvin(288.0);

/// Source: the Sun's nominal effective temperature, 5772 K (IAU 2015
/// Resolution B3).
const SUN: Kelvin = Kelvin(5772.0);

/// Definition: a red dwarf's effective temperature, 3300 K, a test input
/// whose shortwave budget differs from the Sun's.
const RED_DWARF: Kelvin = Kelvin(3300.0);

/// Definition: the flux of the two stars, 0.7 and 0.3 S⊕, a test input
/// summing to Earth's.
const TWO_FLUXES: [f64; 2] = [0.7, 0.3];

/// Definition: the tolerance of an identity of sums, 1e-12, a few
/// roundings of the weights.
const IDENTITY: f64 = 1e-12;

/// Definition: 11 bar of N₂, the thickest air of legacy's test of issue
/// #441 (`climate.rs` l.4180), a test input.
const THICK_N2: Bar = Bar(11.0);

/// Definition: the orbits, au, at which the probe is checked, 1, 0.95 and
/// 0.9 au: a temperate Earth, one near the runaway limit and one past it
/// under [`TEST_ALBEDO`] (test inputs).
const PROBE_ORBITS: [f64; 3] = [1.0, 0.95, 0.9];

/// Definition: 2 au, an orbit past the outer edge where no CO₂ thaws an
/// Earth (legacy `the_thermostat_never_exceeds_the_outgassed_co2`,
/// `climate.rs` l.3209), a test input.
const FAR_ORBIT: f64 = 2.0;

/// Definition: 10 bar of CO₂, a test input, far more than a surface at
/// [`FAR_ORBIT`] keeps.
const THICK_CO2: Bar = Bar(10.0);

/// Definition: 1.6 au, study 25's collapsing Earth (`docs/science/
/// planet-atmospheres.md`: its frozen branch "165 K at 1.6 AU, its CO₂
/// frosted out"), a test input.
pub(super) const COLLAPSE_ORBIT: f64 = 1.6;

/// Earth's bare air with today's CO₂, 400 ppm ([`EARTH_DRY_AIR`]), for the
/// balances of a temperate Earth.
pub(super) fn modern_air(inputs: &super::ClimateInputs) -> PartialPressures {
    PartialPressures {
        co2: EARTH_DRY_AIR.co2,
        ..inputs.air
    }
}

#[test]
fn the_albedo_is_the_flux_weighted_budgets() {
    let mut inputs = earth();
    let base = inputs.sources[0];
    inputs.sources = vec![
        Light {
            flux: EarthFlux(TWO_FLUXES[0]),
            effective_temperature: SUN,
            ..base
        },
        Light {
            flux: EarthFlux(TWO_FLUXES[1]),
            effective_temperature: RED_DWARF,
            ..base
        },
    ];
    let mut body = ClimateBody::new(&inputs, Numerics::DEFAULT);
    let water = body.water_column();
    let air = PartialPressures {
        h2o: surface_vapour(EARTH_SURFACE, water).0,
        ..inputs.air
    };
    let derived = body.derived_albedo(EARTH_SURFACE, &air).get();
    let under = body.underneath(EARTH_SURFACE, &air);
    let mut cache = ShortwaveCache::default();
    let mut budget = |star: Kelvin| {
        shortwave_budget(&air, EARTH_SURFACE, inputs.gravity, star, under, &mut cache)
            .bond_albedo
            .get()
    };
    let (sun, red) = (budget(SUN), budget(RED_DWARF));
    assert_ne!(sun, red, "the two stars must be told apart");
    let expected = TWO_FLUXES[0] * sun + TWO_FLUXES[1] * red;
    assert!(
        (derived - expected).abs() < IDENTITY,
        "{derived} against {expected} ({sun}, {red})"
    );
}

#[test]
fn a_thick_nitrogen_earth_keeps_a_balance() {
    // Issue #441: the moist limit does not depend on the nitrogen, so even
    // 11 bar keep a balance at 1 au.
    let inputs = earth();
    let mut body = ClimateBody::new(&inputs, Numerics::DEFAULT);
    let light = body.starlight(Fraction::new(TEST_ALBEDO).unwrap());
    let air = PartialPressures {
        n2: THICK_N2.0,
        ..modern_air(&inputs)
    };
    let e = body.emissivity_at(EARTH_SURFACE, &air);
    let balance = wet_balance(&mut body, light, &air, e);
    assert!(balance.is_some(), "{THICK_N2:?} of N₂ runs away");
}

#[test]
fn the_probe_changes_no_balance() {
    let earth = earth();
    for au in PROBE_ORBITS {
        let inputs = moved(&earth, au);
        let air = modern_air(&inputs);
        let solve = |numerics: Numerics| {
            let mut body = ClimateBody::new(&inputs, numerics);
            let light = body.starlight(Fraction::new(TEST_ALBEDO).unwrap());
            let e = body.emissivity_at(EARTH_SURFACE, &air);
            wet_balance(&mut body, light, &air, e)
        };
        let with = solve(Numerics::DEFAULT);
        let without = solve(Numerics {
            probe: None,
            ..Numerics::DEFAULT
        });
        match (with, without) {
            (None, None) => {}
            (Some((a, t)), Some((b, u))) => {
                assert!((t.0 - u.0).abs() < IDENTITY, "{au} au: {t:?} against {u:?}");
                assert!(
                    (a.h2o - b.h2o).abs() < IDENTITY,
                    "{au} au: {a:?} against {b:?}"
                );
            }
            (a, b) => panic!("{au} au: {a:?} with the probe, {b:?} without"),
        }
    }
}

#[test]
fn a_collapse_keeps_what_the_surface_holds() {
    let inputs = moved(&earth(), FAR_ORBIT);
    let mut body = ClimateBody::new(&inputs, Numerics::DEFAULT);
    let light = body.starlight(Fraction::new(TEST_ALBEDO).unwrap());
    let air = PartialPressures {
        co2: THICK_CO2.0,
        ..inputs.air
    };
    let e = body.emissivity_at(EARTH_SURFACE, &air);
    let c = collapsed_climate(&mut body, light, &air, e);
    let kept = Bar::from(co2_saturation_pressure(c.temperature)).0;
    assert!(c.iterations > 0, "nothing collapsed: {c:?}");
    assert!(
        c.air.co2 <= kept * (1.0 + COLLAPSE_TOLERANCE),
        "{c:?} keeps {kept} bar"
    );
}

#[test]
fn collapse_iterations_is_converged() {
    // The frozen branch's start: a body covered in ice, all its carbon in
    // the air.
    let inputs = moved(&earth(), COLLAPSE_ORBIT);
    let mut body = ClimateBody::new(&inputs, Numerics::DEFAULT);
    let air = PartialPressures {
        co2: inputs.carbon.0,
        ..inputs.air
    };
    let albedo = body.derived_albedo(ICE_COVERED_TEMPERATURE, &air);
    let e = body.emissivity_at(ICE_COVERED_TEMPERATURE, &air);
    let light = body.starlight(albedo);
    let c = collapsed_climate(&mut body, light, &air, e);
    let kept = Bar::from(co2_saturation_pressure(c.temperature)).0;
    assert!(c.iterations > 0, "nothing collapsed: {c:?}");
    assert!(c.iterations < COLLAPSE_ITERATIONS, "{c:?}");
    assert!(c.air.co2 <= kept * (1.0 + COLLAPSE_TOLERANCE), "{c:?}");
}
