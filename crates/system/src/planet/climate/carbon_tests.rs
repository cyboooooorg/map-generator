//! Tier 1: the carbonate thermostat and the maximum greenhouse.

use super::body::ClimateBody;
use super::carbon::{maximum_greenhouse, thermostat};
use super::fixture::earth;
use super::{GREENHOUSE_DECADES, Numerics, THERMOSTAT_TOLERANCE};
use science::units::{Bar, Fraction, Kelvin, WattPerSquareMetre};

/// Definition: the Bond albedo the tests take the light under, 0.3, a
/// test input (Earth's is 0.306, NASA Earth fact sheet).
const TEST_ALBEDO: f64 = 0.3;

/// Source: Earth's average surface temperature, 288 K (NASA Earth fact
/// sheet, nssdc.gsfc.nasa.gov/planetary/factsheet/earthfact.html).
const EARTH_SURFACE: Kelvin = Kelvin(288.0);

/// Definition: the window around Earth's 288 K the thermostat holds it
/// in, 0.05 K, five times the solver's tolerance (`science::balance::
/// TOLERANCE`).
const HELD: Kelvin = Kelvin(0.05);

/// Definition: the finer thermostat tolerance, a tenth of
/// [`THERMOSTAT_TOLERANCE`] (study 25, "Solver constants").
const FINER_THERMOSTAT: f64 = THERMOSTAT_TOLERANCE / 10.0;

/// Definition: how far the finer thermostat may move the temperature,
/// 0.01 K, the solver's tolerance (study 25, "Solver constants").
const THERMOSTAT_MOVE: Kelvin = Kelvin(0.01);

/// Definition: a wider search, one decade more than
/// [`GREENHOUSE_DECADES`].
const WIDER_DECADES: f64 = GREENHOUSE_DECADES + 1.0;

/// Definition: how far the wider search may move the maximum, 0.2 % of
/// the CO₂: the golden section's resolution, 0.618¹⁸ of five decades.
const GREENHOUSE_MOVE: f64 = 2e-3;

/// Earth at 1 au under [`TEST_ALBEDO`]: its body, the light and the
/// emissivity of its surface at 288 K.
fn earth_under_test_albedo(
    inputs: &super::ClimateInputs,
    numerics: Numerics,
) -> (ClimateBody<'_>, WattPerSquareMetre, Fraction) {
    let mut body = ClimateBody::new(inputs, numerics);
    let light = body.starlight(Fraction::new(TEST_ALBEDO).unwrap());
    let e = body.emissivity_at(EARTH_SURFACE, &inputs.air);
    (body, light, e)
}

#[test]
fn the_thermostat_holds_earth_at_288_k() {
    let inputs = earth();
    let (mut body, light, e) = earth_under_test_albedo(&inputs, Numerics::DEFAULT);
    let (air, t, regulated) = thermostat(&mut body, light, &inputs.air, e);
    assert!(regulated, "{air:?} at {t:?}");
    assert!((t.0 - EARTH_SURFACE.0).abs() < HELD.0, "{t:?}");
    let most = body.greenhouse_co2(&inputs.air);
    assert!(air.co2 <= most.0, "{air:?} over {most:?}");
    assert!(
        most.0 <= inputs.carbon.0,
        "{most:?} over {:?}",
        inputs.carbon
    );
}

#[test]
fn the_thermostat_tolerance_is_converged() {
    let inputs = earth();
    let (mut body, light, e) = earth_under_test_albedo(&inputs, Numerics::DEFAULT);
    let (_, t, _) = thermostat(&mut body, light, &inputs.air, e);
    // The maximum greenhouse does not read the thermostat's tolerance:
    // the finer solve keeps the one found.
    let finer = Numerics {
        thermostat_tolerance: FINER_THERMOSTAT,
        ..Numerics::DEFAULT
    };
    let (mut fine, light, e) = earth_under_test_albedo(&inputs, finer);
    fine.greenhouse = body.greenhouse;
    let (_, u, _) = thermostat(&mut fine, light, &inputs.air, e);
    assert!((t.0 - u.0).abs() < THERMOSTAT_MOVE.0, "{t:?} against {u:?}");
}

#[test]
fn the_maximum_greenhouse_lies_inside_its_range() {
    let inputs = earth();
    let most = inputs.carbon.0.max(inputs.air.co2);
    let search = |numerics: Numerics| {
        let mut body = ClimateBody::new(&inputs, numerics);
        maximum_greenhouse(&mut body, &inputs.air)
    };
    let co2 = search(Numerics::DEFAULT);
    let wider = search(Numerics {
        greenhouse_decades: WIDER_DECADES,
        ..Numerics::DEFAULT
    });
    let floor = most * 10f64.powf(-GREENHOUSE_DECADES);
    assert!(
        co2.0 > floor && co2.0 < most,
        "{co2:?} in {floor} to {most}"
    );
    assert!(
        (wider.0 / co2.0 - 1.0).abs() < GREENHOUSE_MOVE,
        "{co2:?} against {wider:?}"
    );
    let _: Bar = co2;
}
