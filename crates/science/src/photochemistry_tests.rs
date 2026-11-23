use super::*;
use crate::constants::{PASCALS_PER_BAR, SOLAR_IRRADIANCE};
use crate::units::{Dex, Pascal, Second, SolarMass, Year};
use crate::xuv::{History, LYMAN_ALPHA, history};

/// Source: Titan's mass parameter GM, 8978.137 10 km³/s², in m³/s² (JPL
/// Solar System Dynamics, planetary satellite physical parameters, from
/// Jacobson 2022, AJ 164, 199; read).
const TITAN_GM: f64 = 8.978_137_10e12;

/// Source: Titan's mean radius, 2574.76 km, in m (JPL Solar System
/// Dynamics, planetary satellite physical parameters, from Archinal et
/// al. 2018, Celest. Mech. 130, 22; read).
const TITAN_RADIUS: f64 = 2.574_76e6;

/// Definition: the orbit of `legacy-final`'s Titan cases, AU, the test
/// geometry Yung et al.'s rate is fitted at (Saturn's mean distance is
/// 9.57 AU; `docs/science/planet-atmospheres.md`, calibration of `Y`).
const TITAN_ORBIT: f64 = 9.58;

/// Definition: the age of the Sun the yield is fitted at, Gyr (the
/// calibration of `Y` in `docs/science/planet-atmospheres.md`).
const NOW: Gyr = Gyr(4.6);

/// Source: Titan's present net loss of methane, molecules cm⁻² s⁻¹,
/// averaged over the globe: Yung, Allen & Pinto 1984, ApJS 55, 465,
/// doi:10.1086/190963, Table 5B.
const YUNG_CH4_LOSS: f64 = 1.5e10;

/// Definition: square centimetres in a square metre.
const CM2_PER_M2: f64 = 1e4;

/// Definition: the window over which the present rate is read, Gyr: the
/// Lyman α share changes by 0.72 × 1e-6 / 4.6 of itself across it.
const RATE_WINDOW: Gyr = Gyr(1e-6);

/// Definition: steps of the budget the bisection is checked against (the
/// legacy test's).
const BUDGET_STEPS: u32 = 200_000;

/// Definition: an orbit where the Sun's ultraviolet outruns any supply,
/// AU.
const CLOSE_ORBIT: f64 = 0.05;

/// Definition: the methane outgassed in `legacy-final`'s first Titan
/// case, bar: enough to outlast the photolysis at 9.58 AU.
const TITAN_CH4: Bar = Bar(19.0);

/// Definition: tolerance of the checks that hold to rounding.
const ROUNDING: f64 = 1e-9;

fn titan_gravity() -> MetrePerSecondSquared {
    MetrePerSecondSquared(TITAN_GM / (TITAN_RADIUS * TITAN_RADIUS))
}

/// The present bolometric flux of the model's Sun at `au`.
fn flux_at(au: f64) -> WattPerSquareMetre {
    SOLAR_IRRADIANCE / (au * au)
}

fn sun(age: Gyr) -> History {
    history(SolarMass(1.0), Dex(0.0), age).unwrap()
}

fn relative(a: f64, b: f64) -> f64 {
    (a / b - 1.0).abs()
}

#[test]
fn titan_loses_methane_at_the_rate_of_yung_1984() {
    let h = sun(NOW);
    let g = titan_gravity();
    let source = [LymanSource {
        flux: flux_at(TITAN_ORBIT),
        history: &h,
    }];
    let destroyed = methane_destroyed(g, &source, NOW - RATE_WINDOW, NOW);
    let kg_m2 = Pascal::from(destroyed).0 / g.0;
    let seconds = Second::from(Year::from(RATE_WINDOW)).0;
    let rate = kg_m2 / CH4_MOLECULE_MASS.0 / seconds / CM2_PER_M2;
    assert!(relative(rate, YUNG_CH4_LOSS) < 0.01, "{rate:e}");
}

#[test]
fn photolysis_matches_a_stepwise_budget() {
    // The bisected onset and the band integral against a step-by-step
    // budget: a steady supply, a destruction that follows the Lyman α
    // history, and a stock that never goes below zero.
    let g = titan_gravity();
    for (orbit, mass, age, ch4) in [
        (TITAN_ORBIT, 1.0, 4.6, 19.0),
        (TITAN_ORBIT, 1.0, 4.6, 3.0),
        (5.2, 1.0, 4.6, 0.5),
        (1.0, 0.3, 8.0, 300.0),
        (TITAN_ORBIT, 1.0, 0.05, 19.0),
    ] {
        let h = history(SolarMass(mass), Dex(0.0), Gyr(age)).unwrap();
        let flux = flux_at(orbit);
        let source = [LymanSource { flux, history: &h }];
        let exact = methane_left(Bar(ch4), g, &source, Gyr(age)).0;
        // Bar destroyed per Gyr per unit of the Lyman α share.
        let per_share = CH4_PER_LYMAN_ALPHA_PHOTON * flux.0 / 4.0 / LYMAN_ALPHA_PHOTON_ENERGY
            * CH4_MOLECULE_MASS.0
            * g.0
            / PASCALS_PER_BAR
            * Second::from(Year::from(Gyr(1.0))).0;
        let dt = age / f64::from(BUDGET_STEPS);
        let mut stock = 0.0_f64;
        for i in 0..BUDGET_STEPS {
            let t = (f64::from(i) + 0.5) * dt;
            let rate = per_share * h.ratio_at(LYMAN_ALPHA, Gyr(t));
            stock = (stock + (ch4 / age - rate) * dt).max(0.0);
        }
        assert!(
            (exact - stock).abs() <= 1e-3 * ch4,
            "{orbit} AU, {mass} M☉, {age} Gyr, {ch4} bar: {exact} {stock}"
        );
    }
}

#[test]
fn the_whole_destruction_matches_the_timelines_fluence() {
    let h = sun(NOW);
    let g = titan_gravity();
    let flux = flux_at(TITAN_ORBIT);
    let source = [LymanSource { flux, history: &h }];
    let destroyed = methane_destroyed(g, &source, Gyr(0.0), NOW).0;
    let fluence = h.exposure(LYMAN_ALPHA).fluence(flux).0;
    let expected = CH4_PER_LYMAN_ALPHA_PHOTON * fluence / LYMAN_ALPHA_PHOTON_ENERGY
        * CH4_MOLECULE_MASS.0
        * g.0
        / 4.0
        / PASCALS_PER_BAR;
    assert!(
        relative(destroyed, expected) < ROUNDING,
        "{destroyed} {expected}"
    );
}

#[test]
fn no_methane_or_no_age_changes_nothing() {
    let h = sun(NOW);
    let g = titan_gravity();
    let source = [LymanSource {
        flux: flux_at(TITAN_ORBIT),
        history: &h,
    }];
    assert_eq!(methane_left(Bar(0.0), g, &source, NOW), Bar(0.0));
    let outgassed = TITAN_CH4;
    assert_eq!(methane_left(outgassed, g, &source, Gyr(0.0)), outgassed);
}

#[test]
fn a_close_planet_keeps_no_methane() {
    let h = sun(NOW);
    let source = [LymanSource {
        flux: flux_at(CLOSE_ORBIT),
        history: &h,
    }];
    assert_eq!(
        methane_left(Bar(1.0), titan_gravity(), &source, NOW),
        Bar(0.0)
    );
}

#[test]
fn two_half_sources_photolyse_like_one() {
    let h = sun(NOW);
    let g = titan_gravity();
    let flux = flux_at(TITAN_ORBIT);
    let one = [LymanSource { flux, history: &h }];
    let half = LymanSource {
        flux: flux / 2.0,
        history: &h,
    };
    let two = [half, half];
    let (whole, halves) = (
        methane_destroyed(g, &one, Gyr(0.0), NOW).0,
        methane_destroyed(g, &two, Gyr(0.0), NOW).0,
    );
    assert!(relative(halves, whole) < ROUNDING, "{whole} {halves}");
    let outgassed = TITAN_CH4;
    let (whole, halves) = (
        methane_left(outgassed, g, &one, NOW).0,
        methane_left(outgassed, g, &two, NOW).0,
    );
    assert!(whole > 0.0, "{whole}");
    assert!(relative(halves, whole) < ROUNDING, "{whole} {halves}");
}
