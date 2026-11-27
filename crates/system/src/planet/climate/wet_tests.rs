//! Tier 1: the body's albedo, the wet balance and the collapse of a CO₂
//! the surface cannot keep.

use super::ClimateInputs;
use super::body::ClimateBody;
use super::carbon::thermostat;
use super::fixture::{earth, moved};
use super::wet::{Kept, boils, collapse_search, collapsed_climate, wet_balance};
use super::{COLLAPSE_ITERATIONS, COLLAPSE_STEP, COLLAPSE_TOLERANCE, Light, Numerics};
use science::balance::{EARTH_DRY_AIR, TOLERANCE, absorbed, skin};
use science::habitable::{OCEAN_MASS, runaway_flux};
use science::radiation::albedo::{ShortwaveCache, shortwave_budget};
use science::radiation::column::{Profile, outgoing_longwave};
use science::radiation::gases::PartialPressures;
use science::radiation::ozone::ozone_column;
use science::surface::{ICE_COVERED_TEMPERATURE, surface_vapour};
use science::units::{Bar, EarthFlux, Fraction, Kelvin, Kilogram};
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

/// Definition: 0.05 bar of methane, a test input: in Earth's air, some
/// 6 mbar of it above the 0.1 bar tropopause, against the 1.3 mbar at
/// which its stratosphere takes half its most (`CH4_NEAR_IR_HALF_BAR`),
/// so that the vapour's dilution still moves its share of the light.
const WARM_METHANE: Bar = Bar(0.05);

/// Definition: 0.7 au, a test input: Earth's air with [`WARM_METHANE`]
/// there keeps a warm, wet balance (some 296 K and 20 mbar of vapour)
/// under [`TEST_ALBEDO`], just short of its runaway (at 0.65 au).
const WARM_METHANE_ORBIT: f64 = 0.7;

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
    // Earth's air with its O₂, so that each star's light crosses the
    // ozone that star's ultraviolet makes of it.
    let mut inputs = earth();
    inputs.air.o2 = EARTH_DRY_AIR.o2;
    let base = inputs.sources[0];
    let o2 = Bar(inputs.air.o2);
    inputs.sources = vec![
        Light {
            flux: EarthFlux(TWO_FLUXES[0]),
            effective_temperature: SUN,
            ozone: ozone_column(o2, SUN),
            ..base
        },
        Light {
            flux: EarthFlux(TWO_FLUXES[1]),
            effective_temperature: RED_DWARF,
            ozone: ozone_column(o2, RED_DWARF),
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
        shortwave_budget(
            &air,
            ozone_column(Bar(air.o2), star),
            EARTH_SURFACE,
            inputs.gravity,
            star,
            base.metallicity,
            under,
            &mut cache,
        )
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
fn a_wet_methane_balance_is_its_own_airs() {
    // #681: a warm, wet Earth under methane, whose stratosphere takes a
    // share of the light that the vapour dilutes. The balance's
    // definition: on the air it returns, its vapour held, the column sends
    // out at least what that air's surface absorbs at the temperature it
    // returns, and less one solver tolerance below.
    let inputs = moved(&earth(), WARM_METHANE_ORBIT);
    let start = PartialPressures {
        ch4: WARM_METHANE.0,
        ..modern_air(&inputs)
    };
    let mut body = ClimateBody::new(&inputs, Numerics::DEFAULT);
    let light = body.starlight(Fraction::new(TEST_ALBEDO).unwrap());
    let e = body.emissivity_at(EARTH_SURFACE, &start);
    let (air, t) = wet_balance(&mut body, light, &start, e).expect("a balance");
    let mut excess = |x: f64| {
        e.get()
            * outgoing_longwave(
                &air,
                Kelvin(x),
                skin(light),
                inputs.gravity,
                Profile::CLIMATE,
                &mut body.columns,
            )
            .0
            - absorbed(light, &air, inputs.heat_flux).0
    };
    let (at, below) = (excess(t.0), excess(t.0 - TOLERANCE.0));
    assert!(
        at >= 0.0 && below < 0.0,
        "{t:?} under {air:?}: {at} and {below} W/m²"
    );
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
    // the air. This holds the iteration cap (`COLLAPSE_ITERATIONS`); the
    // tolerance is held in tier 3 (`collapse_tolerance_is_converged`).
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
    assert!(c.held && c.iterations < COLLAPSE_ITERATIONS, "{c:?}");
    assert!(c.air.co2 <= kept * (1.0 + COLLAPSE_TOLERANCE), "{c:?}");
}

/// Source: Earth's Bond albedo, 0.306 (NASA Earth fact sheet,
/// nssdc.gsfc.nasa.gov/planetary/factsheet/earthfact.html).
const EARTH_BOND: f64 = 0.306;

/// Definition: the water of the trace test, 1e-4 bar of vapour, a test
/// input far under what a wet balance needs to hold the moist limit
/// (Kasting 1988 section 3a; Boukrouche et al. 2021 Fig. 9).
const TRACE_WATER: Bar = Bar(1e-4);

/// Definition: the tolerance of the column of the trace test, 1e-9 bar,
/// the roundings of the inverse of `column_pressure`.
const TRACE_TOLERANCE: f64 = 1e-9;

/// Source: Venus's semi-major axis, 0.723 au (NASA Venus fact sheet,
/// nssdc.gsfc.nasa.gov/planetary/factsheet/venusfact.html): where a wet
/// Earth runs away (Kasting 1988, Icarus 74, 472, section 3a and Fig. 7:
/// the saturated column's outgoing radiation levels out near 310 W/m²,
/// far under what a wet Venus absorbs).
const VENUS_ORBIT: f64 = 0.723;

/// The verdict on `inputs` of Earth's air, Bond albedo and a surface
/// radiating as a black body.
fn boils_earth(inputs: &ClimateInputs) -> bool {
    boils(
        inputs,
        &EARTH_DRY_AIR,
        Fraction::new(EARTH_BOND).unwrap(),
        Fraction::new(1.0).unwrap(),
    )
}

/// `inputs` of Earth keeping its present ocean, not run away.
fn with_ocean(mut inputs: ClimateInputs) -> ClimateInputs {
    inputs.water = OCEAN_MASS;
    inputs.runaway = false;
    inputs
}

#[test]
fn earth_keeps_its_ocean_at_one_au() {
    // An observation: Earth's oceans have not boiled.
    assert!(!boils_earth(&with_ocean(earth())));
}

#[test]
fn a_wet_venus_boils() {
    let inputs = with_ocean(moved(&earth(), VENUS_ORBIT));
    // Past Kopparapu et al. 2014's limit for the Sun (1.107 S⊕).
    let flux: f64 = inputs.sources.iter().map(|s| s.flux.0).sum();
    let limit = runaway_flux(inputs.sources[0].effective_temperature).0;
    assert!(flux > limit, "{flux} S⊕ against the limit {limit} S⊕");
    assert!(boils_earth(&inputs));
}

#[test]
fn a_runaway_does_not_boil_again() {
    let mut inputs = with_ocean(moved(&earth(), VENUS_ORBIT));
    inputs.runaway = true;
    assert!(!boils_earth(&inputs));
}

#[test]
fn a_regulated_climate_with_a_wet_balance_does_not_boil() {
    // Earth at 1 AU under its carbonate thermostat: the verdict reads the
    // air the thermostat left, and a wet balance exists under it. Being
    // regulated is not what keeps it from boiling: a regulated climate
    // whose saturated column cannot send out what it absorbs boils
    // (`heavy_tests::a_boil_that_escapes_all_its_water_ends_dry`).
    let inputs = with_ocean(earth());
    let mut body = ClimateBody::new(&inputs, Numerics::DEFAULT);
    let bond = Fraction::new(EARTH_BOND).unwrap();
    let black = Fraction::new(1.0).unwrap();
    let light = body.starlight(bond);
    let (air, _, regulated) = thermostat(&mut body, light, &inputs.air, black);
    assert!(regulated, "{air:?}");
    assert!(!boils(&inputs, &air, bond, black));
}

#[test]
fn a_trace_of_water_does_not_boil() {
    // Kasting 1988 section 3a: too little vapour cannot hold the
    // outgoing radiation at the moist limit.
    let mut inputs = with_ocean(moved(&earth(), VENUS_ORBIT));
    let area = 4.0 * std::f64::consts::PI * inputs.radius.0 * inputs.radius.0;
    inputs.water = Kilogram(TRACE_WATER.0 * 1e5 * area / inputs.gravity.0);
    let column = ClimateBody::new(&inputs, Numerics::DEFAULT)
        .water_column()
        .0;
    assert!((column - TRACE_WATER.0).abs() < TRACE_TOLERANCE, "{column}");
    assert!(!boils_earth(&inputs));
}

#[test]
fn two_half_suns_boil_as_one() {
    // The verdict reads the sum of the fluxes (`ClimateBody::starlight`)
    // under an albedo that is an input: this guards that sum.
    let split = |au: f64| {
        let mut inputs = with_ocean(moved(&earth(), au));
        let half = Light {
            flux: EarthFlux(inputs.sources[0].flux.0 / 2.0),
            ..inputs.sources[0]
        };
        inputs.sources = vec![half, half];
        inputs
    };
    assert!(boils_earth(&split(VENUS_ORBIT)));
    assert!(!boils_earth(&split(1.0)));
}

/// A synthetic collapse in the shape measured in #731, CO₂ in bar: from
/// the fold at 17.065 bar, `g(c) = c − φ(c)` rises from 0.0045 bar by a
/// factor e every 0.3 bar, until the climate falls to its cold balance
/// at 0.6 bar. Each `dip` (depth, middle, half-width, bar) takes from it.
fn synthetic_gap(c: f64, dips: &[(f64, f64, f64)]) -> f64 {
    let rise = (c - 0.6).min(0.0045 * (3.3 * (FOLD - c)).exp());
    dips.iter().fold(rise, |g, &(depth, middle, half)| {
        g - depth * (-((c - middle) / half).powi(2)).exp()
    })
}

/// Definition: the fold the synthetic collapse starts from, bar.
const FOLD: f64 = 17.065;

/// Definition: the synthetic collapse's noise, 0.005 bar at the fold, as
/// measured there (#731).
const SYNTHETIC_NOISE: f64 = 0.005;

/// The collapse of the synthetic gap from the fold, plain (`widest` 0)
/// or not: its CO₂ and its evaluations.
fn synthetic_collapse(dips: &[(f64, f64, f64)], widest: f64) -> (f64, usize) {
    let (c, (), n, _) = collapse_search(
        FOLD,
        |c| Kept {
            kept: c - synthetic_gap(c, dips),
            noise: SYNTHETIC_NOISE * c / FOLD,
            state: (),
        },
        COLLAPSE_TOLERANCE,
        COLLAPSE_ITERATIONS * 10,
        widest,
    );
    (c, n)
}

#[test]
fn a_collapse_from_the_fold_reaches_the_cold_balance_in_fewer_steps() {
    let (plain, plain_steps) = synthetic_collapse(&[], 0.0);
    let (fast, fast_steps) = synthetic_collapse(&[], COLLAPSE_STEP);
    assert!((plain - 0.6).abs() < 1e-3, "{plain}");
    assert!((fast - 0.6).abs() < 1e-3, "{fast}");
    assert!(
        fast_steps * 3 < plain_steps,
        "{fast_steps} against {plain_steps}"
    );
}

#[test]
fn a_collapse_stops_at_a_warm_balance_wider_than_its_step() {
    // A warm balance from 16.515 down to 15.110 bar, wider than a rising
    // step there (`COLLAPSE_STEP` of the CO₂, 0.83 bar), and away from the
    // fold: the plain iteration stops at its upper root, and so must the
    // search.
    let dips = [(4.0, 15.4, 0.5)];
    let negative = (0..4000)
        .map(|i| FOLD - 0.001 * f64::from(i))
        .filter(|&c| synthetic_gap(c, &dips) < 0.0)
        .count();
    assert!(
        0.001 * negative as f64 > COLLAPSE_STEP * 16.515,
        "the balance is {negative} thousandths of a bar wide"
    );
    let (plain, plain_steps) = synthetic_collapse(&dips, 0.0);
    let (fast, fast_steps) = synthetic_collapse(&dips, COLLAPSE_STEP);
    assert!((plain - 16.525).abs() < 0.01, "{plain}");
    assert!((fast - plain).abs() < 0.01, "{fast} against {plain}");
    // At most within its noise below the root, where it holds.
    assert!(
        synthetic_gap(fast, &dips) >= -SYNTHETIC_NOISE,
        "{fast} below the root"
    );
    assert!(
        fast_steps * 3 < plain_steps,
        "{fast_steps} against {plain_steps}"
    );
}

#[test]
fn a_collapse_steps_over_a_warm_balance_narrower_than_its_step() {
    // A warm balance between 16.53 and 16.67 bar, narrower than a rising
    // step (0.83 bar there): the plain iteration stops at its upper root,
    // the search steps over it to the cold balance. This is the limit the
    // user accepted on 2026-11-27 for the faster collapse (#731).
    let dips = [(0.15, 16.6, 0.05)];
    let (plain, _) = synthetic_collapse(&dips, 0.0);
    let (fast, _) = synthetic_collapse(&dips, COLLAPSE_STEP);
    assert!((plain - 16.67).abs() < 0.01, "{plain}");
    assert!((fast - 0.6).abs() < 1e-3, "{fast}");
}

#[test]
fn a_falling_gap_closes_on_its_root_by_the_secant() {
    // A collapse in the shape traced round a 3300 K star (#731): from
    // 45.9 bar, `g` rises from 0.026 bar to 2.9 bar near 24 bar, then
    // falls to its root at 2.2 bar. Once `g` falls, the secant's zero is
    // not capped by `COLLAPSE_STEP`: 21 evaluations, against 49 with the
    // capped secant and 62 plain (simulated).
    let gap = |c: f64| 0.00605 * (c - 2.2) * (46.0 - c);
    let (c, (), n, held) = collapse_search(
        45.9,
        |c| Kept {
            kept: c - gap(c),
            noise: 0.0016 + 0.0097 * c / 45.9,
            state: (),
        },
        COLLAPSE_TOLERANCE,
        COLLAPSE_ITERATIONS,
        COLLAPSE_STEP,
    );
    assert!(held, "{c} after {n}");
    assert!((c - 2.2).abs() < 0.01, "{c}");
    assert!(n <= 25, "{n} evaluations");
}

#[test]
fn a_collapse_steps_over_a_zero_within_its_noise() {
    // A dip of 0.007 bar at 17.03 bar takes `g` 0.002 bar under zero,
    // within the noise: the plain iteration stops there (legacy's
    // lottery); the search steps on to the cold balance.
    let dips = [(0.007, 17.03, 0.004)];
    let (plain, _) = synthetic_collapse(&dips, 0.0);
    let (fast, _) = synthetic_collapse(&dips, COLLAPSE_STEP);
    assert!(plain > 17.0, "{plain}");
    assert!((fast - 0.6).abs() < 1e-3, "{fast}");
}

#[test]
fn a_collapse_whose_air_already_holds_does_not_step() {
    let (c, n) = {
        let (c, (), n, _) = collapse_search(
            1.0,
            |c| Kept {
                kept: 2.0 * c,
                noise: 0.0,
                state: (),
            },
            COLLAPSE_TOLERANCE,
            COLLAPSE_ITERATIONS,
            COLLAPSE_STEP,
        );
        (c, n)
    };
    assert_eq!((c, n), (1.0, 0));
}

#[test]
fn a_collapse_stopped_at_its_cap_says_it_did_not_settle() {
    // The synthetic crawl from the fold takes more than five steps to its
    // cold balance, plain or not: stopped at five, neither has settled.
    for widest in [0.0, COLLAPSE_STEP] {
        let (_, (), n, held) = collapse_search(
            FOLD,
            |c| Kept {
                kept: c - synthetic_gap(c, &[]),
                noise: SYNTHETIC_NOISE * c / FOLD,
                state: (),
            },
            COLLAPSE_TOLERANCE,
            5,
            widest,
        );
        assert!(n == 5 && !held, "widest {widest}: {n} steps, held {held}");
    }
    let (_, (), _, held) = collapse_search(
        FOLD,
        |c| Kept {
            kept: c - synthetic_gap(c, &[]),
            noise: SYNTHETIC_NOISE * c / FOLD,
            state: (),
        },
        COLLAPSE_TOLERANCE,
        COLLAPSE_ITERATIONS,
        COLLAPSE_STEP,
    );
    assert!(held);
}

#[test]
fn a_collapse_does_not_crawl_onto_a_zero_within_its_noise() {
    // A dip that brings `g` to zero within the noise: the secant would
    // close in on it by ever shorter steps; the steps stay at least the
    // noise.
    let dips = [(0.0047, 17.06, 0.003)];
    let (fast, steps) = synthetic_collapse(&dips, COLLAPSE_STEP);
    assert!((fast - 0.6).abs() < 1e-3, "{fast}");
    let (_, plain_cold) = synthetic_collapse(&[], 0.0);
    assert!(steps * 2 < plain_cold, "{steps} against {plain_cold}");
}

#[test]
fn a_collapse_stops_where_its_gap_jumps_across_a_root() {
    // A solve with a 0.01 K tolerance makes `g` piecewise smooth, with
    // jumps up to its noise: here +4e-4 above the root at 2 bar and
    // −6e-4 below it, no point within the tolerance above. The search
    // stops within the noise of the root, well under its cap.
    let jump = |c: f64| 0.1 * (c - 2.0) + if c > 2.0 { 4e-4 } else { -6e-4 };
    let collapse = |start: f64, widest: f64| {
        collapse_search(
            start,
            |c| Kept {
                kept: c - jump(c),
                noise: 6.6e-4,
                state: (),
            },
            COLLAPSE_TOLERANCE,
            COLLAPSE_ITERATIONS,
            widest,
        )
    };
    for start in [2.5, 4.0, 10.0] {
        let (c, (), n, held) = collapse(start, COLLAPSE_STEP);
        let (_, (), plain, _) = collapse(start, 0.0);
        assert!((c - 2.0).abs() < 0.01, "from {start}: {c}");
        assert!(
            held && n <= plain && n < COLLAPSE_ITERATIONS,
            "from {start}: {n} steps against {plain}"
        );
    }
}
