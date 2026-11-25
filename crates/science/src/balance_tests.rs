//! Tests of the radiative balance, ported from `legacy-final`'s
//! `planet/climate.rs` tests (l.3339 `more_greenhouse_gas_is_warmer`,
//! l.3996 the runaway albedo, l.4032 and l.2739 the steam blend), with
//! the dark body of the plan's Review Focus 1 and the root finders.

use super::*;
use crate::constants::SOLAR_EFFECTIVE_TEMPERATURE;
use crate::orbit::EARTH_EQUILIBRIUM;
use crate::radiation::test_points::{
    EARTH_SURFACE_K, TITAN_EFFECTIVE_K, TITAN_GRAVITY, TITAN_SURFACE_AIR, TITAN_SURFACE_K,
};

/// Definition: the thin air of the greenhouse test, bar: 1 of N₂ and
/// 1e-3 of CO₂ (legacy l.3341).
const THIN_AIR: [f64; 2] = [1.0, 1e-3];

/// Definition: the CO₂, bar, of the greenhouse test's thick air.
const THICK_CO2_BAR: f64 = 1.0;

/// Definition: the internal heat, W/m², that warms the airless body of
/// the greenhouse test (legacy l.3356).
const WARM_HEAT: WattPerSquareMetre = WattPerSquareMetre(100.0);

/// Definition: an airless body against its grey-body temperature, K, the
/// rounding of a fourth root.
const AIRLESS_TOLERANCE_K: f64 = 1e-9;

/// Definition: the starlight, W/m², of a body with almost no light
/// (Review Focus 1 of the plan).
const DARK_STARLIGHT: WattPerSquareMetre = WattPerSquareMetre(1e-9);

/// Definition: the internal heat, W/m², of the dark body (Review Focus 1
/// of the plan).
const DARK_HEAT: WattPerSquareMetre = WattPerSquareMetre(0.05);

/// Definition: the nitrogen, bar, of the dark body's air and of the
/// convergence test's.
const DARK_N2_BAR: f64 = 1.0;

/// Definition: the tolerance of the root finder's test.
const ROOT_TOLERANCE: f64 = 1e-12;

/// Definition: the bracket of the cube root of 2, (1, 2).
const CUBE_BRACKET: (f64, f64) = (1.0, 2.0);

/// Definition: the absorbed flux, W/m², whose black-body crossing
/// [`bracket_up`] brackets, and the temperature, K, it starts from.
const BRACKET_CASE: (f64, f64) = (240.0, 100.0);

/// Source: the effective temperatures, K, that bound Kopparapu et al.
/// 2014's fits ("2600 K ≤ Teff ≤ 7200 K"), the stars the runaway albedo
/// is checked at with the Sun's.
const FIT_ENDS_K: (f64, f64) = (2600.0, 7200.0);

/// Definition: the relative tolerance of the plateau absorbed at the
/// limit, the rounding of the products.
const BIND_TOLERANCE: f64 = 1e-9;

/// Definition: the stars of the runaway albedo's sweep, K, every 100 K
/// over the fit.
const SWEEP_STEP_K: f64 = 100.0;

/// Definition: the steams, bar, the blend's test rises through.
const STEAMS_BAR: [f64; 7] = [1e-9, 1e-3, 0.1, 1.0, 3.0, 10.0, 1e4];

/// Definition: the derived albedo the steam blend starts from, an
/// Earth-like 0.3 (a test input).
const DERIVED_ALBEDO: f64 = 0.3;

/// Definition: a trace of steam, bar (legacy l.4060: one kilogram's).
const TRACE_STEAM: Bar = Bar(1e-9);

/// Definition: how far the albedo under a trace of steam may be from the
/// derived one: the trace's share, 1e-9, rounded up.
const TRACE_TOLERANCE: f64 = 1e-8;

/// Source: the uncertainty, K, of Titan's surface temperature, +-0.25 K
/// (Fulchignoni et al. 2005, *Nature* 438, 785,
/// doi:10.1038/nature04314).
const TITAN_SURFACE_TOLERANCE_K: f64 = 0.25;

/// Definition: the tolerance of `steam_share(1 bar)` against `1 − e⁻¹`,
/// the rounding of an exponential.
const SHARE_TOLERANCE: f64 = 1e-12;

/// The starlight, W/m², a body of equilibrium temperature `t` absorbs.
fn starlight_of(t: Kelvin) -> WattPerSquareMetre {
    WattPerSquareMetre(STEFAN_BOLTZMANN * t.0.powi(4))
}

/// Earth's starlight, that of a body at [`EARTH_EQUILIBRIUM`].
fn earth_light() -> WattPerSquareMetre {
    starlight_of(EARTH_EQUILIBRIUM)
}

/// A black body's emissivity.
fn black() -> Fraction {
    Fraction::new(1.0).expect("one is a fraction")
}

/// [`surface_temperature`] on Earth's gravity with no heat, a black body,
/// the default solver and a fresh cache.
fn earth_surface(gases: &PartialPressures, heat: WattPerSquareMetre) -> f64 {
    surface_temperature(
        earth_light(),
        gases,
        heat,
        black(),
        STANDARD_GRAVITY,
        SOLVER,
        &mut ColumnCache::default(),
    )
    .0
}

#[test]
fn more_greenhouse_gas_is_warmer() {
    let [n2, co2] = THIN_AIR;
    let thin = PartialPressures {
        n2,
        co2,
        ..PartialPressures::default()
    };
    let thick = PartialPressures {
        co2: THICK_CO2_BAR,
        ..thin
    };
    let none = WattPerSquareMetre(0.0);
    let (t_thin, t_thick) = (earth_surface(&thin, none), earth_surface(&thick, none));
    println!("thin air {t_thin:.2} K, thick {t_thick:.2} K");
    assert!(
        t_thin > EARTH_EQUILIBRIUM.0 && t_thick > t_thin,
        "{t_thin} {t_thick}"
    );
    let bare = earth_surface(&PartialPressures::default(), none);
    assert!(
        (bare - EARTH_EQUILIBRIUM.0).abs() < AIRLESS_TOLERANCE_K,
        "{bare}"
    );
    // Internal heat warms an airless surface.
    assert!(earth_surface(&PartialPressures::default(), WARM_HEAT) > bare);
}

#[test]
fn a_dark_body_is_warmed_by_its_heat_alone() {
    // Review Focus 1: almost no light, a little internal heat. Without air
    // and under nitrogen the surface sits at least where the heat alone
    // puts a black body.
    let floor = (DARK_HEAT.0 / STEFAN_BOLTZMANN).powf(0.25);
    let nitrogen = PartialPressures {
        n2: DARK_N2_BAR,
        ..PartialPressures::default()
    };
    for gases in [PartialPressures::default(), nitrogen] {
        let t = surface_temperature(
            DARK_STARLIGHT,
            &gases,
            DARK_HEAT,
            black(),
            STANDARD_GRAVITY,
            SOLVER,
            &mut ColumnCache::default(),
        )
        .0;
        println!("{gases:?}: {t} K against {floor} K");
        assert!(t.is_finite() && t >= floor, "{gases:?}: {t} vs {floor}");
    }
}

#[test]
fn regula_falsi_finds_a_crossing() {
    let cube = |x: f64| x.powi(3) - 2.0;
    let (a, b) = CUBE_BRACKET;
    let root = regula_falsi(
        cube,
        Point { x: a, f: cube(a) },
        Point { x: b, f: cube(b) },
        ROOT_TOLERANCE,
        ITERATIONS,
    );
    let expected = 2.0_f64.cbrt();
    println!("{root:?} against {expected}");
    assert!(root.iterations < ITERATIONS, "{root:?}");
    assert!((root.x - expected).abs() < ROOT_TOLERANCE, "{root:?}");
    assert!(cube(root.x) >= 0.0, "{root:?}");

    let (flux, start) = BRACKET_CASE;
    let excess = |t: f64| STEFAN_BOLTZMANN * t.powi(4) - flux;
    let crossing = (flux / STEFAN_BOLTZMANN).powf(0.25);
    let (low, high) = bracket_up(
        excess,
        flux,
        Point {
            x: start,
            f: excess(start),
        },
    );
    println!("{low:?} {high:?} around {crossing} K");
    assert!(low.f < 0.0 && high.f >= 0.0, "{low:?} {high:?}");
    assert!(low.x < crossing && crossing <= high.x, "{low:?} {high:?}");
}

#[test]
fn iterations_is_converged() {
    // `ITERATIONS` is a cap: the reference solves stop by their tolerance
    // under it. Earth's air with its vapour, Titan's column at its
    // gravity, and nitrogen under a dark starlight.
    let earth = PartialPressures {
        h2o: surface_vapour(Kelvin(EARTH_SURFACE_K), Bar(f64::INFINITY)).0,
        ..EARTH_DRY_AIR
    };
    let dark = PartialPressures {
        n2: DARK_N2_BAR,
        ..PartialPressures::default()
    };
    let none = WattPerSquareMetre(0.0);
    for (name, light, gases, heat, gravity) in [
        ("Earth", earth_light(), earth, none, STANDARD_GRAVITY),
        (
            "Titan",
            starlight_of(Kelvin(TITAN_EFFECTIVE_K)),
            TITAN_SURFACE_AIR,
            none,
            TITAN_GRAVITY,
        ),
        ("dark", DARK_STARLIGHT, dark, DARK_HEAT, STANDARD_GRAVITY),
    ] {
        let root = surface_root(
            light,
            &gases,
            heat,
            black(),
            gravity,
            SOLVER,
            &mut ColumnCache::default(),
        );
        println!("{name}: {:.3} K in {} steps", root.x, root.iterations);
        assert!(root.x.is_finite(), "{name}: {root:?}");
        assert!(root.iterations < ITERATIONS, "{name}: {root:?}");
    }
}

#[test]
fn titans_column_sits_at_its_measured_surface() {
    // Issue #646: the sourced column of `test_points` (1.467 bar, 5.65 %
    // of methane, an effective temperature of 82 K, Titan's gravity, no
    // heat, a black surface) under the anti-greenhouse of its
    // stratosphere sits at Titan's measured surface.
    let t = surface_temperature(
        starlight_of(Kelvin(TITAN_EFFECTIVE_K)),
        &TITAN_SURFACE_AIR,
        WattPerSquareMetre(0.0),
        black(),
        TITAN_GRAVITY,
        SOLVER,
        &mut ColumnCache::default(),
    )
    .0;
    println!("Titan's column: {t:.3} K against {TITAN_SURFACE_K} K");
    assert!(
        (t - TITAN_SURFACE_K).abs() <= TITAN_SURFACE_TOLERANCE_K,
        "{t} K against {TITAN_SURFACE_K} +- {TITAN_SURFACE_TOLERANCE_K} K"
    );
}

#[test]
fn the_runaway_albedo_binds_the_plateau_at_the_limit() {
    let plateau = earth_moist_plateau();
    let (cool, hot) = FIT_ENDS_K;
    for star in [cool, SOLAR_EFFECTIVE_TEMPERATURE.0, hot] {
        let albedo = runaway_albedo(Kelvin(star), plateau).get();
        let limit = WattPerSquareMetre::from(runaway_flux(Kelvin(star))).0;
        let taken = (1.0 - albedo) * limit / 4.0;
        println!("{star} K: A_rg {albedo:.4}, absorbs {taken:.3} W/m²");
        assert!(
            (taken / plateau.0 - 1.0).abs() < BIND_TOLERANCE,
            "{star} K: {taken} vs {}",
            plateau.0
        );
    }
    // A_rg stays inside its guard over the fit's range.
    let (least, greatest) = RUNAWAY_ALBEDO;
    let mut star = cool;
    while star <= hot {
        let albedo = runaway_albedo(Kelvin(star), plateau).get();
        assert!(albedo > least && albedo < greatest, "{star} K: {albedo}");
        star += SWEEP_STEP_K;
    }
}

#[test]
fn a_runaways_light_follows_its_steam_down_to_none() {
    assert_eq!(steam_share(Bar(0.0)).get(), 0.0);
    let one = steam_share(STEAM_ALBEDO_BAR).get();
    assert!(
        (one - (1.0 - (-1.0_f64).exp())).abs() < SHARE_TOLERANCE,
        "{one}"
    );
    let mut last = 0.0;
    for steam in STEAMS_BAR {
        let share = steam_share(Bar(steam)).get();
        assert!(share > last, "{steam} bar: {share} after {last}");
        last = share;
    }
    let moist = runaway_albedo(SOLAR_EFFECTIVE_TEMPERATURE, earth_moist_plateau()).get();
    let blend = DERIVED_ALBEDO + (moist - DERIVED_ALBEDO) * steam_share(TRACE_STEAM).get();
    assert!((blend - DERIVED_ALBEDO).abs() < TRACE_TOLERANCE, "{blend}");
}

/// Source: how much Earth's argon may warm it, 0.2 K: study 29's bound,
/// over its +0.07 to +0.11 K scaled from Goldblatt et al. 2009's doubled
/// N₂ (argon, 0.0127 bar, broadens less than N₂). An upper bound only:
/// that window is for a wet climate, and this dry column, without the
/// water's feedback, is about 12 times less sensitive (+0.005 K for the
/// argon, +0.37 K for doubled N₂; the wet Earth at 280 ppm gives +0.125 K
/// for its argon, `docs/science/planet-atmospheres.md`, "Argon").
const EARTH_ARGON_WARMING_K: f64 = 0.2;

/// Source: Earth's argon as a weight, 0.0127 bar (study 29, derived from
/// the NASA Earth fact sheet's 9340 ppm and Trenberth & Smith 2005).
const EARTH_ARGON_BAR: f64 = 0.0127;

#[test]
fn earths_argon_warms_its_column_little() {
    // Earth's dry air under Earth's light, with and without its argon:
    // the argon weighs, broadens as air and steepens the dry slope.
    let without = earth_surface(&EARTH_DRY_AIR, WattPerSquareMetre(0.0));
    let with = earth_surface(
        &PartialPressures {
            ar: EARTH_ARGON_BAR,
            ..EARTH_DRY_AIR
        },
        WattPerSquareMetre(0.0),
    );
    let warming = with - without;
    println!("Earth's dry column: {without:.3} K, with its argon {with:.3} K ({warming:+.3} K)");
    // An upper bound only (see `EARTH_ARGON_WARMING_K`).
    assert!(warming < EARTH_ARGON_WARMING_K, "{warming}");
}
