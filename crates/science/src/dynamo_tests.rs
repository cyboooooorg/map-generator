use super::*;

/// Calibration: the legacy's single dipole share, 0.067 (tag
/// `legacy-final`, `planet/core.rs` l.83), fitted on Earth's surface
/// dipole of about 30 µT; study 21, part 4, splits it into the factors
/// tested here.
const LEGACY_DIPOLE_SHARE: f64 = 0.067;

/// Definition: relative tolerance of the split dipole share against the
/// legacy's 0.067, which the remainder 0.82 (two digits of 0.825)
/// reaches to 0.6 %.
const SHARE_TOLERANCE: f64 = 0.01;

/// Definition: relative tolerance of a law checked against its own
/// formula evaluated by hand (round-off only).
const ROUND_OFF: f64 = 1e-12;

/// Definition: a core density, kg/m³, at which the laws are checked, of
/// the order of an iron core's.
const DENSITY: KilogramPerCubicMetre = KilogramPerCubicMetre(11_000.0);

/// Definition: a heat flux out of a core, W/m², at which the flux scaling
/// is checked.
const FLUX: WattPerSquareMetre = WattPerSquareMetre(0.07);

/// Definition: a core radius, m, of the order of Earth's.
const CORE_RADIUS: Metre = Metre(3.5e6);

/// Definition: a planet radius, m, of the order of Earth's.
const PLANET_RADIUS: Metre = Metre(6.4e6);

/// Definition: a cooling time, Gyr, at which the phases and the field are
/// checked.
const TAU: Gyr = Gyr(4.0);

/// Definition: a small step, in cooling times, on either side of each
/// threshold.
const STEP: f64 = 1e-3;

/// Definition: the number of ages sampled from 0 to 5 cooling times.
const SAMPLES: u32 = 400;

/// Definition: the span sampled, in cooling times, past [`CORE_FROZEN`].
const SPAN: f64 = 5.0;

fn relative(value: f64, expected: f64) -> f64 {
    (value / expected - 1.0).abs()
}

#[test]
fn the_flux_scaling_is_christensens() {
    // Christensen, Holzwarth & Reiners 2009, eq. 2 with f_ohm = 1:
    // B²/2µ₀ = c ρ^(1/3) (F q)^(2/3).
    let mu_0 = 4.0e-7 * std::f64::consts::PI;
    let by_hand = (2.0 * mu_0 * 0.63 * DENSITY.0.cbrt() * (0.52 * FLUX.0).powf(2.0 / 3.0)).sqrt();
    let field = rms_field(DENSITY, FLUX, DYNAMO_EFFICIENCY);
    assert!(
        relative(field.0, by_hand) < ROUND_OFF,
        "{field:?} against {by_hand}"
    );
}

#[test]
fn the_dipole_factors_multiply_to_earths_calibration() {
    let product =
        DYNAMO_EFFICIENCY.cbrt() * DIPOLE_OVER_FIELD * RMS_TO_EQUATORIAL * DIPOLE_REMAINDER;
    assert!(
        relative(product, LEGACY_DIPOLE_SHARE) < SHARE_TOLERANCE,
        "{product}"
    );
}

#[test]
fn the_cooling_time_is_the_lumped_law() {
    // τ = ρ c r_c d / (3 k Nu), in seconds, then in Gyr.
    let mantle = PLANET_RADIUS - CORE_RADIUS;
    let seconds = DENSITY.0 * 840.0 * CORE_RADIUS.0 * mantle.0 / (3.0 * 4.0 * NUSSELT_PLATES);
    let by_hand = seconds / (365.25 * 86_400.0 * 1e9);
    let tau = cooling_time(DENSITY, CORE_RADIUS, mantle, NUSSELT_PLATES);
    assert!(
        relative(tau.0, by_hand) < ROUND_OFF,
        "{tau:?} against {by_hand}"
    );
}

#[test]
fn the_phases_follow_their_thresholds() {
    let at = |t: f64| TAU * t;
    let liquid = |dynamo| CorePhase::Liquid { dynamo };
    // Under a lid: thermal dynamo, then none, then compositional, then frozen.
    assert_eq!(
        core_phase(at(THERMAL_DYNAMO_END - STEP), TAU, false),
        liquid(true)
    );
    assert_eq!(
        core_phase(at(THERMAL_DYNAMO_END + STEP), TAU, false),
        liquid(false)
    );
    assert_eq!(
        core_phase(at(INNER_CORE_START - STEP), TAU, false),
        liquid(false)
    );
    assert_eq!(
        core_phase(at(INNER_CORE_START + STEP), TAU, false),
        liquid(true)
    );
    assert_eq!(core_phase(at(CORE_FROZEN - STEP), TAU, false), liquid(true));
    assert_eq!(
        core_phase(at(CORE_FROZEN + STEP), TAU, false),
        CorePhase::Frozen
    );
    // Under plates the thermal dynamo runs until the core freezes.
    assert_eq!(
        core_phase(at(THERMAL_DYNAMO_END + STEP), TAU, true),
        liquid(true)
    );
    assert_eq!(
        core_phase(at(INNER_CORE_START - STEP), TAU, true),
        liquid(true)
    );
    assert_eq!(core_phase(at(CORE_FROZEN - STEP), TAU, true), liquid(true));
    assert_eq!(
        core_phase(at(CORE_FROZEN + STEP), TAU, true),
        CorePhase::Frozen
    );
}

#[test]
fn a_core_cools_monotonically() {
    // Legacy `cores_freeze_with_age` (`planet/core.rs` l.258), on the
    // phases alone, plus the field falling with age.
    for plates in [false, true] {
        let mut frozen = false;
        for step in 0..=SAMPLES {
            let age = TAU * (f64::from(step) / f64::from(SAMPLES) * SPAN);
            let phase = core_phase(age, TAU, plates);
            if frozen {
                assert_eq!(phase, CorePhase::Frozen, "{plates} {age:?}");
            }
            frozen = phase == CorePhase::Frozen;
        }
        assert!(frozen, "the core freezes within {SPAN} cooling times");
    }
    for deep in [false, true] {
        let mut last = f64::INFINITY;
        for step in 0..=SAMPLES {
            let age = TAU * (f64::from(step) / f64::from(SAMPLES) * CORE_FROZEN);
            let field = surface_field(DENSITY, CORE_RADIUS, PLANET_RADIUS, age, TAU, deep).0;
            assert!(field > 0.0 && field < last, "{deep} {age:?} {field}");
            last = field;
        }
    }
}

#[test]
fn a_deep_dynamo_is_weaker() {
    let dipolar = surface_field(DENSITY, CORE_RADIUS, PLANET_RADIUS, TAU, TAU, false);
    let deep = surface_field(DENSITY, CORE_RADIUS, PLANET_RADIUS, TAU, TAU, true);
    assert!(deep < dipolar, "{deep:?} against {dipolar:?}");
}

#[test]
fn no_core_no_field() {
    let none = Metre(0.0);
    let tau = cooling_time(DENSITY, none, PLANET_RADIUS, NUSSELT_LID);
    for deep in [false, true] {
        assert_eq!(
            surface_field(DENSITY, none, PLANET_RADIUS, TAU, tau, deep),
            Tesla(0.0)
        );
        assert_eq!(
            surface_field(DENSITY, none, PLANET_RADIUS, TAU, TAU, deep),
            Tesla(0.0)
        );
    }
}
