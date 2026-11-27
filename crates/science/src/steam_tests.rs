use super::*;
use crate::constants::{EARTH_MEAN_RADIUS, STANDARD_GRAVITY};
use crate::outgassing::column_pressure;
use crate::units::{EarthMass, Kelvin};

/// Source: Turbet et al. 2020 § 3.1, the worked example's core: "a
/// 0.3 M⊕ pure silicate core planet".
const EXAMPLE_CORE_MASS: EarthMass = EarthMass(0.3);

/// Source: the radius of a 0.3 M⊕ MgSiO₃ planet in Zeng's mass-radius
/// table, the one Turbet et al. 2020's footnote 2 names, 0.750 R⊕
/// interpolated (study 26, `docs/science/planet-atmospheres.md`, "The
/// steam envelope"). Zeng's R⊕ is the mean radius, 6371 km (Zeng et al.
/// 2019's 12 175 km is their 1.911 R⊕, `docs/science/planet-interiors.md`).
const EXAMPLE_CORE_RADII: f64 = 0.750;

/// Source: Turbet et al. 2020 § 3.1, "a 5 % water-to-rock ratio".
const EXAMPLE_SHARE: f64 = 0.05;

/// Source: Turbet et al. 2020 § 3.1, the worked example's planet "located
/// just above the runaway greenhouse irradiation threshold", 1 S⊕ at the
/// fit's lower bound.
const EXAMPLE_FLUX: EarthFlux = EarthFlux(1.0);

/// Source: Turbet et al. 2020 § 3.1, "the transit radius lies around
/// 1.2 R⊕"; the window is ±5 % of it (Definition: the test's tolerance
/// for "around"; the laws give 1.23).
const EXAMPLE_RADIUS_WINDOW: (f64, f64) = (1.14, 1.26);

/// Source: Turbet et al. 2020 § 3.1, "∼2 m s⁻²" at the transit radius;
/// the window is ±10 % of it (Definition: the test's tolerance for "∼").
const EXAMPLE_GRAVITY_WINDOW: (f64, f64) = (1.8, 2.2);

/// Source: Turbet et al. 2020 § 2.2, the fit's insolations, 1 to 30 S⊕:
/// one under and one over them.
const UNDER_AND_OVER_FLUX: (EarthFlux, EarthFlux) = (EarthFlux(0.5), EarthFlux(100.0));

/// Source: Turbet et al. 2020 § 2.2, the fit's gravities, 0.2 to 6 g⊕:
/// one under and one over them, in g⊕.
const UNDER_AND_OVER_GRAVITY: (f64, f64) = (0.1, 10.0);

/// Definition: a water share whose steam on a 1 g⊕ core weighs far past
/// the fit's 2.7e10 Pa.
const DEEP_SHARE: f64 = 0.9;

/// Definition: a base pressure under the transit pressure of 0.1 Pa.
const UNDER_TRANSIT: Pascal = Pascal(0.01);

/// Source: insolations of the fit's range (§ 2.2) past the point where
/// the worked example's Eq. 2 thickness reaches the core's radius (about
/// 5.7 S⊕, `docs/science/planet-atmospheres.md`, "Calibration of the
/// thickness cap").
const CAPPED_FLUXES: [EarthFlux; 2] = [EarthFlux(10.0), EarthFlux(30.0)];

/// Definition: a test input, a 5 M⊕ core.
const HEAVY_CORE_MASS: EarthMass = EarthMass(5.0);

/// Definition: a test input, the heavy core's radius, 1.6 R⊕ (mean).
const HEAVY_CORE_RADII: f64 = 1.6;

/// Source: legacy `interior.rs` (`legacy-final`, l.746), the water
/// shares of `steam_inflates_the_radius_with_the_water_share`.
const RISING_SHARES: [f64; 7] = [1e-5, 1e-4, 1e-3, 0.01, 0.05, 0.1, 0.2];

/// Definition: the critical pressure of water in bar, 22.064 MPa.
const CRITICAL_BAR: f64 = 220.64;

/// Definition: a relative tolerance at the rounding of a few operations.
const ROUNDING: f64 = 1e-12;

/// Definition: a step just under the steam world line.
const UNDER_LINE: Bar = Bar(0.01);

/// Definition: a step just under the critical temperature.
const UNDER_CRITICAL: Kelvin = Kelvin(0.01);

/// Definition: a steam column far past the line.
const DEEP_STEAM: Bar = Bar(1e5);

/// Definition: a surface far past the critical temperature.
const HOT_SURFACE: Kelvin = Kelvin(2000.0);

fn share(value: f64) -> Fraction {
    Fraction::new(value).unwrap()
}

fn core(mass: EarthMass, radii: f64) -> (Kilogram, Metre) {
    (Kilogram::from(mass), EARTH_MEAN_RADIUS * radii)
}

fn gravity_of(mass: Kilogram, radius: Metre) -> MetrePerSecondSquared {
    MetrePerSecondSquared(GRAVITATIONAL_CONSTANT * mass.0 / (radius.0 * radius.0))
}

/// The water share whose steam weighs `pressure` at `gravity`, the
/// inverse of `x/(1−x) g²/(4πG)`.
fn share_weighing(pressure: Pascal, gravity: MetrePerSecondSquared) -> Fraction {
    let ratio = pressure.0 * 4.0 * PI * GRAVITATIONAL_CONSTANT / (gravity.0 * gravity.0);
    share(ratio / (1.0 + ratio))
}

#[test]
fn turbets_worked_example_is_about_1_2_earth_radii() {
    let (mass, radius) = core(EXAMPLE_CORE_MASS, EXAMPLE_CORE_RADII);
    let steam = envelope(mass, radius, share(EXAMPLE_SHARE), EXAMPLE_FLUX);
    let transit = radius + steam.thickness;
    let radii = transit.0 / EARTH_MEAN_RADIUS.0;
    assert!(
        (EXAMPLE_RADIUS_WINDOW.0..=EXAMPLE_RADIUS_WINDOW.1).contains(&radii),
        "{radii} R⊕"
    );
    // The whole planet's mass, M_core/(1−x) (Appendix D): 2.05 m/s²
    // there; the core's mass alone gives 1.95 (the page says both).
    let planet = mass.0 / (1.0 - EXAMPLE_SHARE);
    let gravity = GRAVITATIONAL_CONSTANT * planet / (transit.0 * transit.0);
    assert!(
        (EXAMPLE_GRAVITY_WINDOW.0..=EXAMPLE_GRAVITY_WINDOW.1).contains(&gravity),
        "{gravity} m/s²"
    );
}

#[test]
fn the_base_pressure_is_the_weight_of_the_steam() {
    let (mass, radius) = core(EXAMPLE_CORE_MASS, EXAMPLE_CORE_RADII);
    let gravity = gravity_of(mass, radius);
    let water = Kilogram(EXAMPLE_SHARE / (1.0 - EXAMPLE_SHARE) * mass.0);
    let weight = Pascal::from(column_pressure(water, gravity, radius));
    let base = base_pressure(share(EXAMPLE_SHARE), gravity);
    assert!(
        (base.0 - weight.0).abs() <= ROUNDING * weight.0,
        "{base:?} against {weight:?}"
    );
    let steam = envelope(mass, radius, share(EXAMPLE_SHARE), EXAMPLE_FLUX);
    assert_eq!(steam.core_gravity, gravity);
    assert_eq!(steam.base_pressure, base);
    assert_eq!(
        steam.temperature,
        steam_temperature(share(EXAMPLE_SHARE), gravity, EXAMPLE_FLUX)
    );
}

#[test]
fn the_temperature_stays_on_the_fit_grid() {
    let (mass, radius) = core(EXAMPLE_CORE_MASS, EXAMPLE_CORE_RADII);
    let g = gravity_of(mass, radius);
    let x = share(EXAMPLE_SHARE);
    assert_eq!(
        steam_temperature(x, g, UNDER_AND_OVER_FLUX.0),
        steam_temperature(x, g, INSOLATION_RANGE.0)
    );
    assert_eq!(
        steam_temperature(x, g, UNDER_AND_OVER_FLUX.1),
        steam_temperature(x, g, INSOLATION_RANGE.1)
    );
    assert_eq!(
        steam_temperature(x, STANDARD_GRAVITY * UNDER_AND_OVER_GRAVITY.0, EXAMPLE_FLUX),
        steam_temperature(x, GRAVITY_RANGE.0, EXAMPLE_FLUX)
    );
    assert_eq!(
        steam_temperature(x, STANDARD_GRAVITY * UNDER_AND_OVER_GRAVITY.1, EXAMPLE_FLUX),
        steam_temperature(x, GRAVITY_RANGE.1, EXAMPLE_FLUX)
    );
    let deepest = share_weighing(STEAM_PRESSURE_RANGE.1, STANDARD_GRAVITY);
    assert_eq!(
        steam_temperature(share(DEEP_SHARE), STANDARD_GRAVITY, EXAMPLE_FLUX),
        steam_temperature(deepest, STANDARD_GRAVITY, EXAMPLE_FLUX)
    );
}

#[test]
fn no_steam_has_no_envelope() {
    let (mass, radius) = core(EXAMPLE_CORE_MASS, EXAMPLE_CORE_RADII);
    let dry = envelope(mass, radius, share(0.0), EXAMPLE_FLUX);
    assert_eq!(dry.thickness, Metre(0.0));
    let trace = share_weighing(UNDER_TRANSIT, gravity_of(mass, radius));
    assert!(base_pressure(trace, gravity_of(mass, radius)) < TRANSIT_PRESSURE);
    assert_eq!(
        envelope(mass, radius, trace, EXAMPLE_FLUX).thickness,
        Metre(0.0)
    );
}

#[test]
fn an_empty_core_has_no_envelope() {
    let (mass, radius) = core(EXAMPLE_CORE_MASS, EXAMPLE_CORE_RADII);
    for (core_mass, core_radius) in [(Kilogram(0.0), radius), (mass, Metre(0.0))] {
        let steam = envelope(core_mass, core_radius, share(EXAMPLE_SHARE), EXAMPLE_FLUX);
        assert_eq!(steam.thickness, Metre(0.0), "{steam:?}");
        assert!(
            steam.base_pressure.0.is_finite()
                && steam.core_gravity.0.is_finite()
                && steam.temperature.0.is_finite(),
            "{steam:?}"
        );
    }
}

#[test]
fn a_thickness_past_the_core_radius_is_capped() {
    let (mass, radius) = core(EXAMPLE_CORE_MASS, EXAMPLE_CORE_RADII);
    for flux in CAPPED_FLUXES {
        let steam = envelope(mass, radius, share(EXAMPLE_SHARE), flux);
        assert_eq!(steam.thickness, radius * THICKNESS_CAP, "{flux:?}");
    }
}

#[test]
fn the_envelope_thickens_with_the_water() {
    let (mass, radius) = core(HEAVY_CORE_MASS, HEAVY_CORE_RADII);
    let cap = radius * THICKNESS_CAP;
    let mut last = Metre(0.0);
    for x in RISING_SHARES {
        let thickness = envelope(mass, radius, share(x), EXAMPLE_FLUX).thickness;
        assert!(
            thickness.0.is_finite() && thickness >= last,
            "{x}: {thickness:?}"
        );
        if thickness < cap {
            assert!(thickness > last, "{x}: {thickness:?} after {last:?}");
        }
        last = thickness;
    }
}

#[test]
fn the_steam_world_line_is_the_critical_point() {
    assert!((STEAM_WORLD_MIN_BAR.0 - CRITICAL_BAR).abs() <= ROUNDING * CRITICAL_BAR);
    assert!(hides_surface(
        true,
        STEAM_WORLD_MIN_BAR,
        CRITICAL_POINT_TEMPERATURE
    ));
    assert!(!hides_surface(
        true,
        STEAM_WORLD_MIN_BAR - UNDER_LINE,
        CRITICAL_POINT_TEMPERATURE
    ));
    assert!(!hides_surface(
        true,
        STEAM_WORLD_MIN_BAR,
        CRITICAL_POINT_TEMPERATURE - UNDER_CRITICAL
    ));
    assert!(!hides_surface(false, DEEP_STEAM, HOT_SURFACE));
}

/// Source: Turbet et al. 2020's hottest steam profile, K (§ 2.1, "from
/// 300 to 4300 K"), read again here so that the test does not read the
/// constant it checks.
const TURBET_HOTTEST_SURFACE_K: f64 = 4300.0;

/// Definition: a kelvin, a step on either side of the top.
const STEP: Kelvin = Kelvin(1.0);

#[test]
fn surfaces_past_turbets_hottest_profile_are_marked() {
    let top = Kelvin(TURBET_HOTTEST_SURFACE_K);
    assert_eq!(PUBLISHED_SURFACE_TOP, top);
    assert!(!past_published_profiles(top));
    assert!(!past_published_profiles(Kelvin(top.0 - STEP.0)));
    assert!(past_published_profiles(Kelvin(top.0 + STEP.0)));
}
