//! Tier 3: the climate solver against the outer edge of the habitable
//! zone, the convergence of its numerical constants (study 25, "Solver
//! constants", the rows marked tier 3), and the legacy climate tests that
//! solve a bistable Earth or sweep a body's light.
//!
//! Ported from `legacy-final:crates/mapgen/src/planet/climate.rs`
//! (`mod tests`), with the bounds unchanged except where the plan's
//! Decisions changed them. Every test here is `#[ignore = "heavy: …"]`;
//! `mise run test:heavy -- climate` runs them. A bound that fails is
//! reported with its numbers, never retuned.
//!
//! The convergence rows that run under 1 s are kept here on purpose
//! only when they run past about 3 s at opt-level 0 (tier 1's profile):
//! `collapse_tolerance_is_converged` (3.2 s), `scan_ratio_is_converged`
//! (5.8 s), `census_182_d` (13 s), `albedo_damping_is_converged` and
//! `a_planet_just_past_the_runaway_limit_runs_away_hot` (about 20 s);
//! the condensation loop of a Triton-like body runs in 0.1 s and is
//! checked in tier 1 (`the_fixed_points_are_converged`).
//!
//! Covered elsewhere (tier 3, `branches_tests.rs` and `settled_tests.rs`):
//! Earth's snowball share against Hoffman et al. 2017
//! (`earth_snowballs_as_often_as_hoffman_dates`), and "same planet, same
//! answer" on a bistable body (`earth_twice_is_the_same_earth`).

use super::body::ClimateBody;
use super::branches::{Solved, frozen_over, solve, solve_with};
use super::fixture::{EARTH_OBLIQUITY, earth, moved, rotation, titan};
use super::settle::Settle;
use super::settle_tests::{cold_methane_world, settled, triton_like};
use super::wet::{collapsed_climate, wet_balance, wet_climate};
use super::wet_tests::{COLLAPSE_ORBIT, EARTH_SURFACE, TEST_ALBEDO, modern_air};
use super::{
    ALBEDO_DAMPING, ALBEDO_TOLERANCE_K, COLLAPSE_TOLERANCE, ClimateInputs, Numerics, SCAN_RATIO,
    THAW_STEPS,
};
use crate::Trial;
use crate::planet::bare::{Ground, bare_air, bare_with, formed, surface_pass};
use crate::planet::codes;
use crate::planet::draws::{Drawn, LABELS};
use crate::planet::interior_tests::{AU_492, CORE_492, MASS_492, WATER_492};
use crate::planet::settled::{Settled, settled_with};
use crate::planet::settled_tests::{SMALL_MASS, nodes_of};
use crate::planet::tests::{
    EARTH, EARTH_WATER_FRACTION, MARS, ReferenceBody, TITAN, reference_trial, with_fluence,
    with_star,
};
use crate::planet::tests::{derived_from, trial, water_world_trial};
use crate::planet::water::boiled;
use science::balance::{
    EARTH_DRY_AIR, PLATEAU_SURFACES, SIMPSON_NAKAJIMA_W_M2, TOLERANCE, absorbed,
    earth_moist_plateau, moist_plateau, skin,
};
use science::constants::{PASCALS_PER_BAR, STANDARD_GRAVITY, STEFAN_BOLTZMANN};
use science::escape::{WATER_PER_HYDROGEN, energy_limited_loss};
use science::habitable::{SURFACE_WATER_SHARE, maximum_greenhouse_flux, runaway_flux};
use science::outgassing::column_pressure;
use science::radiation::clouds::cloud_cover;
use science::radiation::column::{ColumnCache, Profile, outgoing_longwave};
use science::radiation::gases::PartialPressures;
use science::surface::{
    ICE_COVERED_TEMPERATURE, Liquid, N2_ICE_EMISSIVITY, PHASE_BAND, methane_freezes, surface_vapour,
};
use science::units::{
    Bar, EarthFlux, EarthMass, EarthRadius, Fraction, Kelvin, Kilogram, Metre, Pascal,
};
use science::volatiles::{NITROGEN_TRIPLE_TEMPERATURE, co2_saturation_pressure};
use science::water::{CRITICAL_POINT_TEMPERATURE, melting_temperature};
use timeline::{ClimateState, Code, Gas, LiquidKind, Material, NodeValue, Provenance, Seed, Trace};

/// Source: the Sun's nominal effective temperature, 5772 K (IAU 2015
/// Resolution B3).
const SUN: Kelvin = Kelvin(5772.0);

/// Definition: the cooler stars of the outer-edge test, 4500 K and
/// 3300 K (legacy `the_outer_edge_follows_the_star`), inside the fit of
/// Kopparapu et al. 2014 (2600 to 7200 K).
const COOL_STARS: [Kelvin; 2] = [Kelvin(4500.0), Kelvin(3300.0)];

/// Definition: the bounds of the outer-edge search, 1 and 3 AU (legacy
/// `outer_edge_au`): an Earth wet at the first and frozen at the second.
const EDGE_BRACKET_AU: (f64, f64) = (1.0, 3.0);

/// Definition: the share of its runaway orbit at which the outer-edge
/// search starts for a star whose limit lies past 1 AU, 1.001 (the least
/// step of `science::balance`'s bracket, `LEAST_STEP`).
const PAST_RUNAWAY: f64 = 1.001;

/// Definition: the bisection steps of the outer-edge search, 12 (legacy
/// `outer_edge_au`): `2 / 2¹²` = 0.0005 AU.
const EDGE_STEPS: usize = 12;

/// Definition: the largest move of the Sun's outer edge, AU, that
/// #593's lever 2 and a finer maximum greenhouse may make, 0.001: two
/// steps of [`EDGE_STEPS`] (study 25, "Solver constants" and #593's
/// acceptance condition).
const EDGE_MOVE_AU: f64 = 0.001;

/// Definition: a snowball draw over any chance, 1: the temperate branch
/// wherever there is one.
const WARM_DRAW: f64 = 1.0;

/// Definition: a snowball draw under any chance, 0: the frozen branch
/// wherever there is one.
const COLD_DRAW: f64 = 0.0;

/// Definition: the finer steps of the maximum greenhouse's search, 30
/// (study 25, "Solver constants": at 30 the outer edge moves under
/// 0.001 AU).
const FINER_GREENHOUSE_STEPS: usize = 30;

/// Definition: the finer ratio of the wet balance's scan, 1.01 (study 25,
/// "Solver constants": the same first balance as at [`SCAN_RATIO`]).
const FINER_SCAN_RATIO: f64 = 1.01;

/// Definition: the flux, against the runaway limit `S_rg`, at which the
/// scan ratio is compared, 0.9, 0.95 and 0.99 (study 25, "Solver
/// constants").
const SCAN_FRACTIONS: [f64; 3] = [0.9, 0.95, 0.99];

/// Definition: the finer plateau surfaces, every 10 K from 500 to 700 K
/// (study 25, "Solver constants").
const FINER_PLATEAU: (f64, f64, f64) = (500.0, 700.0, 10.0);

/// Definition: the most the moist plateau may move on finer surfaces,
/// 0.5 W/m² (study 25, "Solver constants").
const PLATEAU_MOVE: f64 = 0.5;

/// Definition: the finer collapse tolerance, 1e-5 (study 25, "Solver
/// constants").
const FINER_COLLAPSE: f64 = 1e-5;

/// Definition: the most the temperature may move at the finer collapse
/// tolerance, 0.01 K (study 25, "Solver constants").
const COLLAPSE_MOVE_K: f64 = 0.01;

/// Definition: the finer thaw bisection, 16 steps (study 25, "Solver
/// constants").
const FINER_THAW_STEPS: usize = 16;

/// Definition: the most Earth's thaw CO₂ may move, as a share of
/// itself, at the finer thaw bisection, 0.1 % (study 25, "Solver
/// constants").
const THAW_MOVE: f64 = 1e-3;

/// Definition: the lighter damping of the albedo's fixed point, 0.25
/// (study 25, "Solver constants").
const LIGHTER_DAMPING: f64 = 0.25;

/// Definition: the least light of the sweeps' three-kelvin span: a sweep
/// must start 3 K under its phase point and end 3 K over it (legacy
/// `settles_across`).
const SPAN_K: f64 = 3.0;

/// Definition: the climates a sweep takes, 200 (legacy
/// `the_fixed_point_converges_across_*`).
const SWEEP_COUNT: usize = 200;

/// Definition: the thin nitrogen air of the methane sweep, 0.1 bar
/// (legacy `thin_titan`): a gas down to nitrogen's triple point, so that
/// the sweep crosses the methane sea's freezing point alone.
const THIN_N2: Bar = Bar(0.1);

/// Definition: the nitrogen air of the nitrogen sweep, 0.5 bar (legacy
/// `nitrogen_world`), a Triton-like body: Titan without its methane.
const NITROGEN_WORLD_N2: Bar = Bar(0.5);

/// Definition: the light, against Titan's, over which the thin Titan's
/// surface spans the methane sea's freezing point ± 3 K. Legacy's
/// 0.7 to 1.3 reached 77.8 K on the rewrite's column, 1 K short of
/// 78.7 K; 1.5 reaches 79.7 K (measured, as legacy widened its nitrogen
/// sweep when its column changed, issue #523).
const METHANE_SWEEP: (f64, f64) = (0.7, 1.5);

/// Definition: the light, against Titan's, over which the nitrogen
/// world's surface spans nitrogen's triple point ± 3 K. Legacy's 0.7 to
/// 1.25 starts at 64.7 K on the rewrite's column, over the 60.1 K the
/// sweep must start under; 0.55 to 0.85 spans 57.9 to 67.5 K (measured).
const NITROGEN_SWEEP: (f64, f64) = (0.55, 0.85);

/// Definition: the most the nitrogen sweep's temperature may step
/// between two neighbours away from its triple point, 1 K (legacy's
/// number, `PHASE_BAND`'s width; the jump across the point is held by
/// [`PHASE_RATIO_TOLERANCE`] instead).
const NITROGEN_JUMP_K: f64 = 1.0;

/// Definition: the relative agreement, 1 %, of the ratio of the light two
/// adjacent climates absorb with the ratio of their `ε T⁴` across the
/// nitrogen triple point: the thin air's column carries the surface's
/// `ε σ T⁴` to space up to the share its own emission changes between
/// the phases, which is far under 1 %. Measured: absorbed ×1.5556
/// against `ε T⁴` ×1.5580, 0.15 % apart.
const PHASE_RATIO_TOLERANCE: f64 = 0.01;

/// Definition: the climates that must fall in the methane sea's melting
/// band, 20 (legacy).
const BAND_CLIMATES: usize = 20;

/// Definition: the steps of the runaway edge sweep, as flux over `S_rg`:
/// 0.98, 0.99 and 1.0 on the temperate branch (wet), 1.001, 1.01 and
/// 1.02 on the steam branch (legacy
/// `the_runaway_edge_steps_only_by_the_branches_albedos`, issue #523).
const EDGE_SWEEP: ([f64; 3], [f64; 3]) = ([0.98, 0.99, 1.0], [1.001, 1.01, 1.02]);

/// Definition: the most the temperature, taken with the flux's own `S^¼`
/// out, may move between neighbours of a branch, and the most the step
/// at the limit may differ from the albedos' alone, 0.5 K (legacy).
const BRANCH_STEP_K: f64 = 0.5;

/// Definition: the least a runaway's albedo falls under the wet
/// branch's, 0.03 (legacy): the step is real.
const BRANCH_ALBEDO_STEP: f64 = 0.03;

/// Definition: the census case of #443 (legacy `census_182_d`): its
/// equilibrium temperature, K, at the settled albedo.
const CENSUS_EQUILIBRIUM: (f64, f64) = (110.0, 120.0);

/// Definition: the census case's nitrogen over its seas, bar (legacy
/// `census_182_d`).
const CENSUS_N2: (f64, f64) = (4.0, 5.0);

/// Definition: the census case's methane over its seas, bar (legacy
/// `census_182_d`).
const CENSUS_CH4: (f64, f64) = (0.5, 5.0);

/// Definition: the census case's surface, under 150 K (legacy).
const CENSUS_SURFACE_K: f64 = 150.0;

/// Definition: the orbit of the thermostat's CO₂ test, 1.3 AU (legacy
/// `the_thermostat_never_exceeds_the_outgassed_co2`): inside the outer
/// edge.
const THERMOSTAT_ORBIT: f64 = 1.3;

/// Definition: the orbit past the outer edge, 2 AU (legacy).
const PAST_EDGE_ORBIT: f64 = 2.0;

/// Definition: the orbit of the scarce-CO₂ test, 1.6 AU (legacy).
const SCARCE_ORBIT: f64 = 1.6;

/// Definition: the share of its CO₂ the scarce Earth keeps, 1e-4
/// (legacy).
const SCARCE_SHARE: f64 = 1e-4;

/// Definition: the room the CO₂ of a frozen air has over what its ground
/// keeps, 0.1 % (legacy `1.001`).
const KEPT_ROOM: f64 = 1.001;

/// Source: Archean nitrogen, 2 to 3 bar (Goldblatt et al. 2009,
/// doi:10.1038/ngeo692; Charnay et al. 2013, doi:10.1016/j.epsl.2013.
/// 03.047), and the two water inventories of legacy's test, Earth's and
/// 1 % of the mass.
const ARCHEAN_N2: [f64; 2] = [2.0, 3.0];

/// Definition: the second water inventory of the nitrogen test, 1 % of
/// the mass (legacy).
const WET_WATER_FRACTION: f64 = 0.01;

/// Definition: a test tolerance, legacy's window on a thermostat-held
/// Earth under 2 to 3 bar of nitrogen, 282 to 294 K: the thermostat's
/// 288 K ± 6 K (legacy
/// `an_earth_with_two_or_three_bar_of_nitrogen_stays_temperate`). Not a
/// measured range.
const ARCHEAN_WINDOW: (f64, f64) = (282.0, 294.0);

/// Source: the Sun at 2.5 Ga gave 0.82 of today's light (Gough 1981,
/// doi:10.1007/BF00151270).
const FAINT_SUN: f64 = 0.82;

/// Source: the CO₂ of the late Archean test, 10 mbar (Charnay et al.
/// 2013, §5.2).
const ARCHEAN_CO2: f64 = 0.010;

/// Source: the late Archean air, 1 and 2 bar of nitrogen less the 10 mbar
/// of CO₂ (Charnay et al. 2013, §5.2: "from 1 to 2 bar of N₂").
const ARCHEAN_AIRS: [f64; 2] = [0.99, 1.99];

/// Definition: the bounds of the column's temperature search, 200 and
/// 400 K, and its 40 bisection steps (legacy).
const ARCHEAN_SEARCH: (f64, f64, usize) = (200.0, 400.0, 40);

/// Source: the warming from doubling the nitrogen in a 1-D column with
/// its clouds held fixed, 3 to 6 K: the 1-D range of 4 to 5 K (the
/// 4.4 K of the clear-sky 1-D model of Goldblatt et al. 2009,
/// doi:10.1038/ngeo692, as Charnay et al. 2013, §5.2, and
/// `docs/science/planet-atmospheres.md`, "Thick nitrogen", quote it)
/// widened by legacy's 1 K on either side. The +7 K of Charnay et al.
/// 2013's GCM, whose clouds follow the surface, lies outside it on
/// purpose: the test holds the cloud cover fixed.
const ARCHEAN_WARMING: (f64, f64) = (3.0, 6.0);

/// Definition: the orbits of the temperate Earth, as a share of its
/// outer edge, which stand in for legacy's 1.4, 1.5, 1.6, 1.8 and 2.5 AU
/// against the 1.535 AU edge (#523): 0.9, 0.98 inside, 1.02, 1.2 and 1.6
/// outside.
const EDGE_SHARES: [f64; 5] = [0.9, 0.98, 1.02, 1.2, 1.6];

/// Definition: the freezing of the temperate Earth, 273 K (legacy): the
/// least temperature of a wet orbit and the most of a frozen one.
const FREEZING_K: f64 = 273.0;

/// Definition: the room a colder orbit has over a nearer one, 0.05 K
/// (legacy: a numerical tolerance of the monotonic fall).
const FALL_ROOM_K: f64 = 0.05;

/// Definition: the CO₂ of the air before the climate in the build-up
/// test, a tenth of a bar (legacy `co2_builds_up_past_the_outgassed_
/// snapshot`).
const SNAPSHOT_CO2: Bar = Bar(0.1);

/// Definition: a carbon inventory under the thermostat's need, half a
/// bar (legacy).
const SMALL_INVENTORY: Bar = Bar(0.5);

/// Definition: where the build-up test's snowball stands, as a share of
/// the way between the orbits that bound it: 1.2 AU in legacy; here the
/// snowball's orbit is one with a thaw, found by the scan of
/// [`edge_scan`].
const HALF: f64 = 0.5;

/// Definition: the orbits of the dry acid air, from 0.8 AU in 61 steps
/// of 0.02 AU (legacy `a_dry_acid_air_settles_through_the_deck_onset`).
const ACID_ORBITS: (f64, f64, usize) = (0.8, 0.02, 60);

/// Definition: the dry acid air: 3 bar of nitrogen, 0.05 bar of CO₂ and
/// 0.02 bar of SO₂ (legacy).
const ACID_AIR: PartialPressures = PartialPressures {
    n2: 3.0,
    co2: 0.05,
    h2o: 0.0,
    ch4: 0.0,
    nh3: 0.0,
    so2: 0.02,
    o2: 0.0,
    ar: 0.0,
};

/// Definition: the band the deck's onset falls in, 263 to 330 K, and the
/// least orbits near it, 3 (legacy).
const ACID_BAND: (f64, f64, usize) = (263.0, 330.0, 3);

/// Definition: the largest gap between the equilibrium of the derived
/// albedo and of the settled one, 0.1 K (legacy).
const ACID_GAP_K: f64 = 0.1;

/// Definition: the runaway test's water, 1 % of Earth's mass (legacy
/// `a_planet_just_past_the_runaway_limit_runs_away_hot`).
const RUNAWAY_WATER: f64 = 0.01;

/// Definition: the XUV fluence of the runaway test, 0 J/m²: no escape,
/// as legacy's test keeps the water of its inventory in the air (the
/// chain's escape strips this body of all its water).
const NO_ESCAPE: f64 = 0.0;

/// Definition: the flux of the runaway test, 1.01 times its star's
/// runaway limit (legacy).
const RUNAWAY_OVER: f64 = 1.01;

/// Definition: the margin a runaway keeps over water's critical
/// temperature, 100 K (legacy).
const RUNAWAY_MARGIN_K: f64 = 100.0;

/// Definition: the relative agreement of the snowball's CO₂ with what
/// the thermostat reaches, 1 % (legacy).
const REACHED_AGREEMENT: f64 = 0.01;

/// Definition: the snowballs that test must see, 2 (legacy).
const SNOWBALLS_SEEN: usize = 2;

/// Definition: the room the CO₂ of a body frozen for good has over its
/// cap, 1.0001 (legacy).
const CAP_ROOM: f64 = 1.0001;

/// Definition: the margin an inside orbit keeps from its edge, AU, 0.02:
/// forty bisection steps of [`EDGE_STEPS`].
const EDGE_MARGIN_AU: f64 = 0.02;

/// Definition: the shares of the edge at which the snowball tests look
/// for the thaw, 0.65 to 0.977 (legacy's 1.0, 1.2, 1.4 and 1.5 AU over
/// its 1.535 AU edge); 1.0 AU stays its own.
const SNOWBALL_SHARES: [f64; 3] = [0.78, 0.91, 0.977];

/// The climate inputs of `body` rebuilt from its trial at `au` round a
/// star of `teff` (as legacy `reference.rs::derive_at`), not locked, at
/// Earth's obliquity.
fn inputs_at(body: &ReferenceBody, au: f64, teff: Kelvin) -> ClimateInputs {
    inputs_from(body, au, teff, None)
}

/// [`inputs_at`] with the XUV fluence of its star replaced by `fluence`,
/// J/m², when given (`with_fluence`).
fn inputs_from(body: &ReferenceBody, au: f64, teff: Kelvin, fluence: Option<f64>) -> ClimateInputs {
    let body = ReferenceBody { orbit: au, ..*body };
    let mut trace = Trace::new();
    let (t, d) = reference_trial(&mut trace, &body);
    let mut t = with_star(&mut trace, &t, teff);
    if let Some(fluence) = fluence {
        t = with_fluence(&mut trace, &t, fluence);
    }
    let bare = bare_with(&t, &d, &mut trace).unwrap();
    let spin = rotation(&mut trace, EARTH_OBLIQUITY, false);
    ClimateInputs::of(&bare, &spin, Bar(0.0)).expect("the body has a surface")
}

/// Earth's inputs at `au` round the Sun.
fn earth_at(au: f64) -> ClimateInputs {
    inputs_at(&EARTH, au, SUN)
}

/// `inputs` of a body lit by one star, its flux replaced by `flux` and
/// its equilibrium temperature scaled as `flux^¼` (a `Definition:`, the
/// trial's own scaling).
fn lit(inputs: &ClimateInputs, flux: f64) -> ClimateInputs {
    assert_eq!(inputs.sources.len(), 1);
    let mut lit = inputs.clone();
    let source = &mut lit.sources[0];
    let ratio = flux / source.flux.0;
    source.flux = EarthFlux(flux);
    source.equilibrium = Kelvin(source.equilibrium.0 * ratio.powf(0.25));
    lit
}

/// Whether Earth on its temperate branch keeps liquid water at `au`
/// round a star of `teff`.
fn wet_at(teff: Kelvin, au: f64, numerics: &Numerics) -> bool {
    let solved = solve_with(&inputs_at(&EARTH, au, teff), WARM_DRAW, numerics);
    solved.settle.liquid == Some(Liquid::Water)
}

/// The outer edge of Earth's habitable zone round a star of `teff`, AU:
/// the largest orbit at which it keeps liquid water, bisected in
/// [`EDGE_STEPS`] steps (legacy `outer_edge_au`).
fn outer_edge_au(teff: Kelvin, numerics: &Numerics) -> f64 {
    // A star cooler than the Sun has its runaway limit under one Earth
    // flux (Kopparapu et al. 2014): Earth rebuilt at 1 AU round it is a
    // runaway with no water, so the search starts just past the orbit of
    // that limit, where the water is kept.
    let runaway_orbit = runaway_flux(teff).0.powf(-0.5) * PAST_RUNAWAY;
    let (mut wet, mut dry) = (EDGE_BRACKET_AU.0.max(runaway_orbit), EDGE_BRACKET_AU.1);
    assert!(wet_at(teff, wet, numerics), "{teff:?}: frozen at {wet} AU");
    assert!(!wet_at(teff, dry, numerics), "{teff:?}: wet at {dry} AU");
    for _ in 0..EDGE_STEPS {
        let mid = 0.5 * (wet + dry);
        if wet_at(teff, mid, numerics) {
            wet = mid;
        } else {
            dry = mid;
        }
    }
    wet
}

/// The orbit-mean flux of an orbit, in units of Earth's.
fn flux_at(au: f64) -> f64 {
    1.0 / (au * au)
}

#[test]
#[ignore = "heavy: three bisections of the outer edge, 14 bistable-Earth climates each"]
fn the_outer_edge_against_kopparapu_2014() {
    // #595: the model's outer edge round the Sun, 4500 K and 3300 K
    // against the maximum greenhouse of Kopparapu et al. 2014 (their
    // Table 1 fit, `science::habitable::maximum_greenhouse_flux`). No
    // bound is frozen on the gap: it goes to the PR and #595.
    let mut last = (f64::INFINITY, f64::INFINITY);
    for teff in [SUN, COOL_STARS[0], COOL_STARS[1]] {
        let edge = outer_edge_au(teff, &Numerics::DEFAULT);
        let flux = flux_at(edge);
        let fit = maximum_greenhouse_flux(teff).0;
        println!(
            "{teff:?}: edge {edge:.4} AU, S {flux:.4} against {fit:.4} (S/S_fit - 1 = {:+.4})",
            flux / fit - 1.0
        );
        assert!(
            edge > EDGE_BRACKET_AU.0 && edge < EDGE_BRACKET_AU.1,
            "{edge}"
        );
        assert!(flux < last.0, "{teff:?}: {flux} after {}", last.0);
        assert!(fit < last.1, "{teff:?}: the fit {fit} after {}", last.1);
        last = (flux, fit);
    }
}

/// The outer edge of `inputs`, a body at 1 AU moved out ([`moved`]), AU:
/// the largest orbit at which it keeps liquid water on its temperate
/// branch, bisected in [`EDGE_STEPS`] steps within [`EDGE_BRACKET_AU`].
fn moved_edge_au(inputs: &ClimateInputs, numerics: &Numerics) -> f64 {
    let wet = |au: f64| {
        solve_with(&moved(inputs, au), WARM_DRAW, numerics)
            .settle
            .liquid
            == Some(Liquid::Water)
    };
    let (mut wet_au, mut dry_au) = EDGE_BRACKET_AU;
    assert!(wet(wet_au), "frozen at {wet_au} AU");
    assert!(!wet(dry_au), "wet at {dry_au} AU");
    for _ in 0..EDGE_STEPS {
        let mid = 0.5 * (wet_au + dry_au);
        if wet(mid) {
            wet_au = mid;
        } else {
            dry_au = mid;
        }
    }
    wet_au
}

#[test]
#[ignore = "heavy: two bisections of a methane Earth's outer edge, and its climate at Titan's orbit twice"]
fn one_greenhouse_search_on_a_condensing_air() {
    // #593 lever 2: the maximum greenhouse searched once on the inventory
    // air, against legacy's search again on each condensed air the
    // thermostat is given (`greenhouse_once: false`), on Earth under
    // Titan's methane, a wet body with plates. A regulated surface sits
    // at `THERMOSTAT_K`, above the critical points of methane and
    // nitrogen, so the air it keeps condenses nothing: a condensed air
    // reaches the thermostat only on the way (the condensation's steps)
    // and where it fails, frozen far out. There the two searches differ;
    // the climate's gap is reported, and the edge held to #593's
    // acceptance condition.
    let like = ClimateInputs {
        air: PartialPressures {
            ch4: titan().air.ch4,
            ..earth().air
        },
        ..earth()
    };
    let each = Numerics {
        greenhouse_once: false,
        ..Numerics::DEFAULT
    };
    let far = moved(&like, TITAN.orbit);
    let (kept, again) = (
        solve_with(&far, WARM_DRAW, &Numerics::DEFAULT),
        solve_with(&far, WARM_DRAW, &each),
    );
    let condensed = kept.settle.air;
    let on_inventory = ClimateBody::new(&far, Numerics::DEFAULT).greenhouse_co2(&condensed);
    let on_condensed = ClimateBody::new(&far, each).greenhouse_co2(&condensed);
    println!(
        "at {} AU: {:.6} K once, {:.6} K at each call (gap {:+.3e} K), CO2 {:.4e} and {:.4e} \
         bar, {:?} and {:?}; methane frost {:.4} bar; the maximum greenhouse {:.4} bar on the \
         inventory, {:.4} bar on the condensed air",
        TITAN.orbit,
        kept.settle.temperature.0,
        again.settle.temperature.0,
        kept.settle.temperature.0 - again.settle.temperature.0,
        kept.settle.air.co2,
        again.settle.air.co2,
        kept.state,
        again.state,
        kept.settle.condensate.ch4,
        on_inventory.0,
        on_condensed.0
    );
    assert!(kept.settle.converged && again.settle.converged);
    assert!(kept.settle.condensate.ch4 > 0.0, "{kept:?}");
    assert!(
        on_inventory != on_condensed,
        "the search at each call reads the inventory air: {on_inventory:?}"
    );
    let (edge, legacy) = (
        moved_edge_au(&like, &Numerics::DEFAULT),
        moved_edge_au(&like, &each),
    );
    println!(
        "outer edge {edge:.4} AU once, {legacy:.4} AU at each call (gap {:+.4} AU)",
        edge - legacy
    );
    assert!(
        (edge - legacy).abs() <= EDGE_MOVE_AU,
        "{edge} against {legacy}"
    );
}

/// The first balance of `inputs` under [`TEST_ALBEDO`] at 288 K's
/// emissivity, with `numerics`, in kelvin.
fn first_balance(inputs: &ClimateInputs, numerics: Numerics) -> Option<f64> {
    let air = modern_air(inputs);
    let mut body = ClimateBody::new(inputs, numerics);
    let light = body.starlight(Fraction::new(TEST_ALBEDO).unwrap());
    let e = body.emissivity_at(EARTH_SURFACE, &air);
    wet_balance(&mut body, light, &air, e).map(|(_, t)| t.0)
}

#[test]
#[ignore = "heavy: wet balances at four orbits, twice"]
fn scan_ratio_is_converged() {
    let limit = runaway_flux(SUN).0;
    let mut bodies = vec![earth()];
    bodies.extend(SCAN_FRACTIONS.map(|f| lit(&earth(), f * limit)));
    let finer = Numerics {
        scan_ratio: FINER_SCAN_RATIO,
        ..Numerics::DEFAULT
    };
    for inputs in &bodies {
        let (a, b) = (
            first_balance(inputs, Numerics::DEFAULT),
            first_balance(inputs, finer),
        );
        match (a, b) {
            (None, None) => {}
            (Some(a), Some(b)) => assert!(
                (a - b).abs() < TOLERANCE.0,
                "flux {:?}: {a} against {b}",
                inputs.sources[0].flux
            ),
            _ => panic!("flux {:?}: {a:?} against {b:?}", inputs.sources[0].flux),
        }
    }
}

#[test]
#[ignore = "heavy: the moist plateau's column on twenty-one surfaces"]
fn plateau_surfaces_are_converged() {
    let (low, high, step) = FINER_PLATEAU;
    let surfaces: Vec<Kelvin> = (0..=((high - low) / step).round() as usize)
        .map(|i| Kelvin(low + step * i as f64))
        .collect();
    assert!(surfaces.len() > PLATEAU_SURFACES.len());
    let finer = moist_plateau(
        &EARTH_DRY_AIR,
        Bar(f64::INFINITY),
        STANDARD_GRAVITY,
        Profile::PLATEAU,
        &surfaces,
        SIMPSON_NAKAJIMA_W_M2,
        &mut ColumnCache::default(),
    );
    let plateau = earth_moist_plateau();
    println!("plateau {plateau:?}, on finer surfaces {finer:?}");
    assert!(
        (finer.0 - plateau.0).abs() < PLATEAU_MOVE,
        "{finer:?} {plateau:?}"
    );
}

#[test]
#[ignore = "heavy: two collapses of an Earth at 1.6 AU"]
fn collapse_tolerance_is_converged() {
    // The tolerance of the collapse (`COLLAPSE_TOLERANCE`): a finer one
    // moves the temperature under `COLLAPSE_MOVE_K`. The iteration cap
    // of the same collapse is held in tier 1
    // (`wet_tests::collapse_iterations_is_converged`).
    let inputs = moved(&earth(), COLLAPSE_ORBIT);
    let collapse = |tolerance: f64| {
        let mut body = ClimateBody::new(
            &inputs,
            Numerics {
                collapse_tolerance: tolerance,
                ..Numerics::DEFAULT
            },
        );
        let air = PartialPressures {
            co2: inputs.carbon.0,
            ..inputs.air
        };
        let albedo = body.derived_albedo(ICE_COVERED_TEMPERATURE, &air);
        let e = body.emissivity_at(ICE_COVERED_TEMPERATURE, &air);
        let light = body.starlight(albedo);
        collapsed_climate(&mut body, light, &air, e)
    };
    let (a, b) = (collapse(COLLAPSE_TOLERANCE), collapse(FINER_COLLAPSE));
    println!("collapse: {:?} against {:?}", a.temperature, b.temperature);
    assert!(
        (a.temperature.0 - b.temperature.0).abs() < COLLAPSE_MOVE_K,
        "{a:?} {b:?}"
    );
}

#[test]
#[ignore = "heavy: two bistable-Earth solves, at 13 and 16 thaw steps"]
fn thaw_steps_is_converged() {
    let inputs = earth();
    let thaw = |steps: usize| {
        solve_with(
            &inputs,
            WARM_DRAW,
            &Numerics {
                thaw_steps: steps,
                ..Numerics::DEFAULT
            },
        )
        .bistable
        .and_then(|b| b.thaw)
        .expect("Earth's CO₂ thaws it")
        .0
    };
    let (a, b) = (thaw(THAW_STEPS), thaw(FINER_THAW_STEPS));
    println!(
        "thaw CO2 {a:.6} bar at {THAW_STEPS} steps, {b:.6} at {FINER_THAW_STEPS} \
         ({:+.4} %)",
        100.0 * (a / b - 1.0)
    );
    assert!((a / b - 1.0).abs() < THAW_MOVE, "{a} against {b}");
}

#[test]
#[ignore = "heavy: the reference climates settled twice"]
fn albedo_damping_is_converged() {
    let lighter = Numerics {
        albedo_damping: LIGHTER_DAMPING,
        ..Numerics::DEFAULT
    };
    let bodies = [
        ("Earth", earth()),
        ("Mars", super::settle_tests::imposed_mars()),
        ("a Triton-like body", triton_like()),
        ("Titan", titan()),
    ];
    for (name, inputs) in &bodies {
        let (a, b) = (settled(inputs, Numerics::DEFAULT), settled(inputs, lighter));
        println!(
            "{name}: {:.3} K against {:.3} K",
            a.temperature.0, b.temperature.0
        );
        assert!(a.converged && b.converged, "{name}: {a:?} {b:?}");
        assert!(
            (a.temperature.0 - b.temperature.0).abs() <= ALBEDO_TOLERANCE_K.0,
            "{name}: {a:?} {b:?}"
        );
    }
}

#[test]
#[ignore = "heavy: two bisections of the Sun's outer edge"]
fn greenhouse_steps_is_converged() {
    let base = outer_edge_au(SUN, &Numerics::DEFAULT);
    let finer = outer_edge_au(
        SUN,
        &Numerics {
            greenhouse_steps: FINER_GREENHOUSE_STEPS,
            ..Numerics::DEFAULT
        },
    );
    println!("edge {base:.4} AU, {finer:.4} AU at {FINER_GREENHOUSE_STEPS} steps");
    assert!(
        (base - finer).abs() <= EDGE_MOVE_AU,
        "{base} against {finer}"
    );
}

/// Titan's body under `air`, its light scaled by each of `count` factors
/// from `low` to `high`: the flux and the settled climate.
fn sweep_of(
    inputs: &ClimateInputs,
    air: PartialPressures,
    (low, high): (f64, f64),
) -> Vec<(f64, Settle)> {
    let base = ClimateInputs {
        air,
        ..inputs.clone()
    };
    let flux = base.sources[0].flux.0;
    (0..SWEEP_COUNT)
        .map(|i| {
            let f = low + (high - low) * i as f64 / (SWEEP_COUNT - 1) as f64;
            let at = lit(&base, flux * f);
            (flux * f, settled(&at, Numerics::DEFAULT))
        })
        .collect()
}

/// A climate of a sweep: its flux and its settle.
type Point<'a> = (f64, &'a Settle);

/// Review Focus 5: across `threshold_k`, every climate of `sweep` settled
/// and its temperature rises with the flux, with no jump over
/// `most_jump_k` between neighbours (legacy `settles_across`), except a
/// jump `explained` says the phases alone account for. Returns how many
/// jumps it let through.
fn settles_across(
    sweep: &[(f64, Settle)],
    threshold_k: f64,
    most_jump_k: f64,
    explained: &dyn Fn(Point<'_>, Point<'_>) -> bool,
) -> usize {
    let (first, last) = (&sweep[0].1, &sweep[sweep.len() - 1].1);
    assert!(
        first.temperature.0 <= threshold_k - SPAN_K && last.temperature.0 >= threshold_k + SPAN_K,
        "the sweep spans {:.2} to {:.2} K, not {threshold_k:.2} ± {SPAN_K} K",
        first.temperature.0,
        last.temperature.0
    );
    for (flux, c) in sweep {
        assert!(c.converged, "{flux:.6}: {c:?}");
    }
    let mut jumps = 0;
    for pair in sweep.windows(2) {
        let ((f0, a), (f1, b)) = (&pair[0], &pair[1]);
        let step = b.temperature.0 - a.temperature.0;
        if step >= most_jump_k && explained((*f0, a), (*f1, b)) {
            jumps += 1;
            continue;
        }
        assert!(
            step > -ALBEDO_TOLERANCE_K.0 && step < most_jump_k,
            "{f0:.6} -> {f1:.6}: {:.3} -> {:.3} K ({a:?} {b:?})",
            a.temperature.0,
            b.temperature.0
        );
    }
    jumps
}

#[test]
#[ignore = "heavy: 200 climates of a methane world across its sea's freezing point"]
fn the_fixed_point_converges_across_the_methane_freezing_point() {
    let reference = titan();
    let air = PartialPressures {
        n2: THIN_N2.0,
        ..reference.air
    };
    let freezes = methane_freezes(THIN_N2).0;
    let sweep = sweep_of(&reference, air, METHANE_SWEEP);
    settles_across(&sweep, freezes, f64::INFINITY, &|_, _| false);
    let melting = sweep
        .iter()
        .filter(|(_, c)| (freezes - PHASE_BAND.0..freezes).contains(&c.temperature.0))
        .count();
    println!("{melting} climates in the melting band under {freezes:.2} K");
    assert!(melting >= BAND_CLIMATES, "{melting} climates in the band");
}

#[test]
#[ignore = "heavy: 200 climates of a nitrogen world across its triple point"]
fn the_fixed_point_converges_until_the_nitrogen_frost_melts() {
    // Every climate of the sweep converges, and away from the triple point
    // adjacent climates stay within NITROGEN_JUMP_K. Across it there is a
    // jump, and it is a bistability, as the runaway edge's step is
    // (#523): nitrogen frost (Bond albedo 0.754, emissivity 0.80) and the
    // melting frost or liquid nitrogen under it (albedo 0.616, emissivity
    // 1.00) are both stable fixed points at one flux, and the branch the
    // fixed point reaches depends on its warm start. Measured on the
    // 200-point sweep: 59.769 K (frost) at flux 0.006861 against 63.153 K
    // (liquid) at 0.006878, +0.24 % of flux, with the same flux settling
    // at 63.094 K from the warm state and the warm flux at 59.804 K from
    // the cold one; the absorbed light rises by a factor 1.55 and so does
    // ε T⁴ ((0.9983 / 0.80) (63.094 / 59.769)⁴). So a jump the sweep
    // lets through must leave the frost (to the melting frost or the
    // liquid), and the absorbed flux must rise as ε T⁴ does, within
    // PHASE_RATIO_TOLERANCE. Under the stellar spectra of #655 the jump
    // lands on the melting frost below the triple point: 59.681 K at
    // 0.006795 against 63.133 K at 0.006812 (albedo 0.7527 to 0.6130,
    // emissivity 0.8000 to 0.9998, absorbed x1.5626 against ε T⁴
    // x1.5650).
    let reference = titan();
    let air = PartialPressures {
        n2: NITROGEN_WORLD_N2.0,
        ch4: 0.0,
        ..reference.air
    };
    let base = ClimateInputs {
        air,
        ..reference.clone()
    };
    let sweep = sweep_of(&reference, air, NITROGEN_SWEEP);
    let absorbed_by = |c: &Settle| {
        let mut body = ClimateBody::new(&base, Numerics::DEFAULT);
        let light = body.starlight(c.bond_albedo);
        absorbed(light, &c.air, base.heat_flux).0
    };
    let explained = |(f0, a): Point<'_>, (f1, b): Point<'_>| {
        // The frost branch: no liquid, and the surface radiates nearer N₂
        // ice's emissivity than the liquid's 1 (the melting frost's climbs
        // from one to the other across the melting band).
        let frost =
            |c: &Settle| c.liquid.is_none() && c.emissivity.get() < 0.5 * (N2_ICE_EMISSIVITY + 1.0);
        let phase = frost(a) != frost(b);
        let light = absorbed_by(b) / absorbed_by(a);
        let emitted = (b.emissivity.get() * b.temperature.0.powi(4))
            / (a.emissivity.get() * a.temperature.0.powi(4));
        println!(
            "jump {f0:.6} -> {f1:.6}: {:.3} -> {:.3} K, albedo {:.4} -> {:.4}, emissivity \
             {:.4} -> {:.4}, absorbed x{light:.4}, e T^4 x{emitted:.4}",
            a.temperature.0,
            b.temperature.0,
            a.bond_albedo.get(),
            b.bond_albedo.get(),
            a.emissivity.get(),
            b.emissivity.get()
        );
        phase && (light / emitted - 1.0).abs() <= PHASE_RATIO_TOLERANCE
    };
    let jumps = settles_across(
        &sweep,
        NITROGEN_TRIPLE_TEMPERATURE.0,
        NITROGEN_JUMP_K,
        &explained,
    );
    assert!(jumps <= 1, "{jumps} explained jumps");
}

#[test]
#[ignore = "heavy: seven settled Earths across the runaway limit"]
fn the_runaway_edge_steps_only_by_the_branches_albedos() {
    // Issue #523, ruling of the climate pass lot 3C-2: Earth's inventory
    // (its water kept, no escape) swept in steps of 1 % of flux across its
    // star's runaway limit `S_rg`, on the temperate branch below (wet) and
    // the steam branch above (its whole ocean in the air). The step at
    // `S_rg` is the bifurcation between the two branches, physics rather
    // than a defect: on each branch the equilibrium temperature is
    // continuous in the flux, and across `S_rg` it jumps by the change of
    // albedo between the branches, `T_eq ∝ (S (1 − A))^¼`, and by nothing
    // else.
    let limit = runaway_flux(SUN).0;
    let base = earth_at((EDGE_SWEEP.0[0] * limit).powf(-0.5));
    assert!(!base.runaway && base.water.0 > 0.0, "{base:?}");
    // (flux over `S_rg`, equilibrium temperature, Bond albedo)
    let at = |f: f64| {
        let mut inputs = lit(&base, f * limit);
        if f > 1.0 {
            inputs.runaway = true;
            inputs.air.h2o = ClimateBody::new(&inputs, Numerics::DEFAULT)
                .water_column()
                .0;
        }
        let c = settled(&inputs, Numerics::DEFAULT);
        (f, c.equilibrium.0, c.bond_albedo.get())
    };
    let temperate: Vec<_> = EDGE_SWEEP.0.into_iter().map(at).collect();
    let steam: Vec<_> = EDGE_SWEEP.1.into_iter().map(at).collect();
    for branch in [&temperate, &steam] {
        for pair in branch.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let (ta, tb) = (a.1 / a.0.powf(0.25), b.1 / b.0.powf(0.25));
            assert!(
                (tb - ta).abs() < BRANCH_STEP_K,
                "{:.3} -> {:.3} S_rg: {ta:.2} -> {tb:.2} K",
                a.0,
                b.0
            );
        }
    }
    let (wet, hot) = (temperate[2], steam[0]);
    let expected = wet.1 * (hot.0 / wet.0 * (1.0 - hot.2) / (1.0 - wet.2)).powf(0.25);
    println!(
        "edge: A {:.4} -> {:.4}, T_eq {:.2} -> {:.2} K (albedo alone {expected:.2} K)",
        wet.2, hot.2, wet.1, hot.1
    );
    assert!(
        (hot.1 - expected).abs() < BRANCH_STEP_K,
        "{:.2} K vs {expected:.2} K",
        hot.1
    );
    // The branches differ in albedo: the step is real, not zero.
    assert!(hot.2 < wet.2 - BRANCH_ALBEDO_STEP, "{wet:?} {hot:?}");
}

#[test]
#[ignore = "heavy: the census case of #443 on its measured inputs"]
fn census_182_d() {
    // The system draw of seed 182 is not wired to the planet chain yet
    // (2d-4): the case is solved on its census inputs, those of
    // `a_methane_greenhouse_cannot_warm_a_cold_world`, and held to
    // legacy's other bounds: T_eq about 114.5 K, 4.4 bar of N₂ and
    // 0.9 bar of methane over its seas.
    let c = settled(&cold_methane_world(), Numerics::DEFAULT);
    println!("{c:?}");
    assert!(
        (CENSUS_EQUILIBRIUM.0..CENSUS_EQUILIBRIUM.1).contains(&c.equilibrium.0),
        "{c:?}"
    );
    assert!((CENSUS_N2.0..CENSUS_N2.1).contains(&c.air.n2), "{c:?}");
    assert!((CENSUS_CH4.0..CENSUS_CH4.1).contains(&c.air.ch4), "{c:?}");
    assert!(c.equilibrium.0 < CENSUS_SURFACE_K && c.temperature.0 < CENSUS_SURFACE_K);
    assert_ne!(c.liquid, Some(Liquid::Water), "{c:?}");
}

/// The air of `c` in bar of CO₂ the ground keeps at its temperature
/// (legacy `co2_saturation_bar`).
fn kept_co2(c: &Settle) -> f64 {
    Bar::from(co2_saturation_pressure(c.temperature)).0
}

#[test]
#[ignore = "heavy: Earth solved at 1.3, 2 and 1.6 AU, the last with a scarce CO₂"]
fn the_thermostat_never_exceeds_the_outgassed_co2() {
    // Inside the outer edge, before where the build-up of a cold planet's
    // CO₂ (issue #522) may take its air past what it outgassed: the
    // thermostat holds 288 K on a few bar of the outgassed.
    let inside = earth_at(THERMOSTAT_ORBIT);
    let c = solve(&inside, WARM_DRAW).settle;
    assert!(c.air.co2 <= inside.air.co2, "{c:?} {:?}", inside.air);
    if !c.regulated {
        // Short of CO₂: all of it is in the air, and the water is liquid
        // only while the surface stays above its melting curve.
        assert_eq!(c.air.co2, inside.air.co2, "{c:?}");
        let melts = melting_temperature(Pascal(c.air.total() * PASCALS_PER_BAR)).0;
        assert_eq!(
            c.liquid == Some(Liquid::Water),
            c.temperature.0 >= melts,
            "{c:?}"
        );
    }
    // Past the outer edge no CO₂ is enough: the planet freezes, and its
    // air keeps only the CO₂ its frozen surface does not frost out (#522).
    let past = earth_at(PAST_EDGE_ORBIT);
    let c = solve(&past, WARM_DRAW).settle;
    assert!(c.air.co2 <= past.air.co2, "{c:?}");
    assert!(c.air.co2 <= kept_co2(&c) * KEPT_ROOM, "{c:?}");
    assert!(!c.regulated, "{c:?}");
    assert_eq!(c.liquid, None, "{c:?}");
    // With a ten-thousandth of its CO₂ the thermostat runs out: it puts
    // all of it in the air, and the water freezes.
    let d = earth_at(SCARCE_ORBIT);
    let scarce = ClimateInputs {
        air: PartialPressures {
            co2: d.air.co2 * SCARCE_SHARE,
            ..d.air
        },
        carbon: Bar(d.air.co2 * SCARCE_SHARE),
        ..d
    };
    let c = settled(&scarce, Numerics::DEFAULT);
    assert!(c.air.co2 <= scarce.air.co2, "{c:?}");
    assert!(!c.regulated, "{c:?}");
    assert_eq!(c.liquid, None, "{c:?}");
}

#[test]
#[ignore = "heavy: four temperate Earths under thick nitrogen, with two water inventories"]
fn an_earth_with_two_or_three_bar_of_nitrogen_stays_temperate() {
    // Archean Earth may have held 2 to 3 bar of N₂ (Goldblatt et al. 2009;
    // Charnay et al. 2013): the thermostat still holds it, with Earth's
    // water and with 1 % of its mass in water.
    for water_fraction in [EARTH_WATER_FRACTION, WET_WATER_FRACTION] {
        for n2 in ARCHEAN_N2 {
            let body = ReferenceBody {
                water_fraction,
                ..EARTH
            };
            let inputs = inputs_at(&body, 1.0, SUN);
            let thick = ClimateInputs {
                air: PartialPressures { n2, ..inputs.air },
                ..inputs
            };
            let c = settled(&thick, Numerics::DEFAULT);
            assert!(c.regulated, "{n2} bar, {water_fraction}: {c:?}");
            assert!(
                (ARCHEAN_WINDOW.0..=ARCHEAN_WINDOW.1).contains(&c.temperature.0),
                "{n2} bar, {water_fraction}: {c:?}"
            );
            assert_eq!(c.liquid, Some(Liquid::Water), "{c:?}");
            assert!(c.air.co2 > 0.0, "{c:?}");
        }
    }
}

#[test]
#[ignore = "heavy: two bisections of a fixed-cloud column, 40 steps each"]
fn doubling_the_nitrogen_warms_a_late_archean_earth_by_a_few_kelvin() {
    // Charnay et al. 2013 (§5.2): the Sun at 2.5 Ga, 10 mbar of CO₂, the
    // air from 1 to 2 bar of N₂: +7 K in their GCM, +4 to 5 K in the 1-D
    // model of Goldblatt et al. 2009. Their CO₂ held fixed, no
    // thermostat, and Earth's albedo (0.3): this checks the column's
    // greenhouse, so the ice-albedo feedback is left out. Goldblatt's
    // clouds are in that albedo and do not change with the surface, so
    // the balance holds the cloud cover fixed, at that of the 1-bar air's
    // own climate: the clear sky with that share of cloudy sky,
    // `(1 − f₀) OLR_clear + f₀ OLR_cloudy`.
    let faint = lit(&earth(), FAINT_SUN * earth().sources[0].flux.0);
    let air = |n2: f64| PartialPressures {
        n2,
        co2: ARCHEAN_CO2,
        ..PartialPressures::default()
    };
    let mut body = ClimateBody::new(&faint, Numerics::DEFAULT);
    let light = body.starlight(Fraction::new(TEST_ALBEDO).unwrap());
    let water = body.water_column();
    let black = Fraction::new(1.0).unwrap();
    let free = wet_climate(&mut body, light, &air(ARCHEAN_AIRS[0]), black).1;
    let cover = |t: f64| cloud_cover(Kelvin(t), surface_vapour(Kelvin(t), water)).get();
    let fixed = cover(free.0);
    let (floor, ceiling, steps) = ARCHEAN_SEARCH;
    let mut cache = ColumnCache::default();
    let mut temperature = |n2: f64| {
        let gases = air(n2);
        let take = absorbed(light, &gases, faint.heat_flux).0;
        let skin = skin(light);
        let mut excess = |t: f64| {
            let wet = PartialPressures {
                h2o: surface_vapour(Kelvin(t), water).0,
                ..gases
            };
            let mut at = |profile| {
                outgoing_longwave(&wet, Kelvin(t), skin, faint.gravity, profile, &mut cache).0
            };
            let (clear, all) = (at(Profile::CLEAR), at(Profile::CLIMATE));
            let f = cover(t);
            let olr = if f > 0.0 {
                clear + fixed / f * (all - clear)
            } else {
                clear
            };
            olr - take
        };
        let (mut low, mut high) = (floor, ceiling);
        assert!(excess(low) < 0.0 && excess(high) > 0.0);
        for _ in 0..steps {
            let mid = 0.5 * (low + high);
            if excess(mid) < 0.0 {
                low = mid;
            } else {
                high = mid;
            }
        }
        0.5 * (low + high)
    };
    let (t1, t2) = (temperature(ARCHEAN_AIRS[0]), temperature(ARCHEAN_AIRS[1]));
    println!(
        "late Archean: {t1:.1} K (cover {fixed:.3} of the {:.1} K climate), +{:.1} K with \
         twice the N₂",
        free.0,
        t2 - t1
    );
    // The 1-bar temperature is printed, not bounded: legacy's 270-290 K
    // window had no published source (Goldblatt et al. 2009 and Charnay
    // et al. 2013 give the warming, not the absolute value). The column
    // puts it about 3.5 K under legacy's 275.4 K because of the IAPWS
    // saturation curve (272.8 K with legacy's constant-L curve swapped in,
    // see the column issue).
    let warming = t2 - t1;
    assert!(
        (ARCHEAN_WARMING.0..=ARCHEAN_WARMING.1).contains(&warming),
        "{t1} -> {t2}"
    );
}

/// The Sun's outer edge for Earth, and the orbits of the temperate-Earth
/// tests measured from it.
fn edge_orbit(edge: f64, share: f64) -> f64 {
    edge * share
}

#[test]
#[ignore = "heavy: the Sun's outer edge, then six Earths around it"]
fn an_earth_track_planet_stays_temperate_to_the_maximum_greenhouse() {
    // The outer edge of the Sun's habitable zone: an Earth on its
    // temperate branch builds up its CO₂ and keeps liquid water to its
    // edge, and freezes beyond it. Colder the farther, with no step but
    // the edge. The orbits measured in the edge test replace legacy's
    // 1.5 AU: 1 AU, 0.9 and 0.98 of the edge inside, 1.02, 1.2 and 1.6
    // of it outside.
    let edge = outer_edge_au(SUN, &Numerics::DEFAULT);
    let mut orbits = vec![1.0];
    orbits.extend(EDGE_SHARES.map(|s| edge_orbit(edge, s)));
    let mut last = f64::INFINITY;
    for au in orbits {
        let solved = solve(&earth_at(au), WARM_DRAW);
        let c = &solved.settle;
        println!(
            "{au:.3} AU: {:.1} K, CO₂ {:.3e} bar, {:?}, regulated {}",
            c.temperature.0, c.air.co2, c.liquid, c.regulated
        );
        if au <= edge - EDGE_MARGIN_AU {
            assert!(c.temperature.0 >= FREEZING_K, "{au} AU: {c:?}");
            assert_eq!(c.liquid, Some(Liquid::Water), "{au} AU: {c:?}");
        } else {
            assert!(c.temperature.0 < FREEZING_K, "{au} AU: {c:?}");
        }
        assert_ne!(solved.state, ClimateState::Snowball, "{au} AU");
        assert!(
            c.temperature.0 <= last + FALL_ROOM_K,
            "{au} AU: {c:?} after {last}"
        );
        last = c.temperature.0;
    }
}

/// The orbits at which the snowball tests look: 1 AU and
/// [`SNOWBALL_SHARES`] of the Sun's outer edge `edge`.
fn edge_scan(edge: f64) -> Vec<f64> {
    let mut orbits = vec![1.0];
    orbits.extend(SNOWBALL_SHARES.map(|s| edge_orbit(edge, s)));
    orbits
}

#[test]
#[ignore = "heavy: the Sun's outer edge, then Earth at 1.5 AU and at its snowball orbits"]
fn co2_builds_up_past_the_outgassed_snapshot() {
    // Inside the outer edge, an air that holds a tenth of a bar of CO₂
    // before the climate (what escape and the cold trap left of it):
    // weathering slows and volcanoes give it more, up to the thermostat,
    // within the carbon inventory and the maximum greenhouse, past which
    // CO₂ condensing aloft and its scattering cool more than they warm.
    let edge = outer_edge_au(SUN, &Numerics::DEFAULT);
    // Legacy's orbit, 1.5 AU against a 1.535 AU edge, held the thermostat
    // at 288 K; on the rewrite's column the carbonate thermostat runs
    // out of CO₂ short of the edge (at the edge less 0.02 AU the
    // maximum greenhouse leaves the surface at about 285.6 K, wet but not
    // regulated), so the orbit is the farthest of the scan round the
    // measured edge where it still regulates.
    let (au, snapshot, c) = edge_scan(edge)
        .into_iter()
        .chain([edge - EDGE_MARGIN_AU])
        .rev()
        .map(|au| {
            let near = earth_at(au);
            let snapshot = ClimateInputs {
                air: PartialPressures {
                    co2: SNAPSHOT_CO2.0,
                    ..near.air
                },
                ..near
            };
            let c = settled(&snapshot, Numerics::DEFAULT);
            println!(
                "{au:.3} AU: {:.2} K, regulated {}",
                c.temperature.0, c.regulated
            );
            (au, snapshot, c)
        })
        .find(|(_, _, c)| c.regulated)
        .expect("an orbit whose thermostat holds");
    println!("{au:.3} AU of an edge at {edge:.3} AU");
    let cap = ClimateBody::new(&snapshot, Numerics::DEFAULT)
        .greenhouse_co2(&snapshot.air)
        .0;
    assert!(c.air.co2 > SNAPSHOT_CO2.0, "{c:?}");
    assert!(
        c.air.co2 <= snapshot.carbon.0,
        "{c:?} {:?}",
        snapshot.carbon
    );
    assert!(cap < snapshot.carbon.0, "{cap} {:?}", snapshot.carbon);
    assert!(c.air.co2 <= cap, "{c:?} {cap}");
    // A smaller inventory stops it short of the thermostat.
    let small = ClimateInputs {
        carbon: SMALL_INVENTORY,
        ..snapshot
    };
    let c = settled(&small, Numerics::DEFAULT);
    assert!(c.air.co2 <= SMALL_INVENTORY.0, "{c:?}");
    assert!(!c.regulated, "{c:?}");
    // A snowball builds its CO₂ up past what it froze with, the same
    // whatever draw made it one: the nearest orbit of the scan with a
    // frozen branch that a CO₂ thaws.
    let (orbit, warm) = edge_scan(edge)
        .into_iter()
        .map(|au| (au, solve(&earth_at(au), WARM_DRAW)))
        .find(|(_, s)| s.bistable.is_some_and(|b| b.thaw.is_some()))
        .expect("an orbit with a snowball");
    let inputs = earth_at(orbit);
    let chance = warm.bistable.unwrap().chance.get();
    let frozen = solve(&inputs, HALF * chance);
    let pinned = solve(&inputs, COLD_DRAW);
    assert_eq!(frozen.settle, pinned.settle);
    assert_eq!(frozen.state, ClimateState::Snowball, "{frozen:?}");
    assert!(
        frozen.settle.air.co2 > warm.settle.air.co2,
        "{frozen:?} {warm:?}"
    );
    assert!(frozen.settle.air.co2 <= inputs.carbon.0, "{frozen:?}");
}

#[test]
#[ignore = "heavy: the Sun's outer edge, then the bistable Earths round it"]
fn a_frozen_state_no_co2_thaws_is_no_snowball() {
    // Drawn frozen where no CO₂ up to its maximum greenhouse thaws it, a
    // body stays frozen for good, so it is no snowball (whose card
    // promises a thaw). Where some CO₂ thaws it, a frozen draw is a
    // snowball. The orbits are those of the scan round the measured edge.
    let edge = outer_edge_au(SUN, &Numerics::DEFAULT);
    let mut orbits = edge_scan(edge);
    orbits.push(edge - EDGE_MARGIN_AU);
    let (mut no_thaw, mut thaws) = (None, None);
    for au in orbits {
        let solved = solve(&earth_at(au), COLD_DRAW);
        let Some(b) = solved.bistable else { continue };
        println!(
            "{au:.3} AU: thaw {:?}, froze {:?}, {:?}, {:.1} K",
            b.thaw, b.froze, solved.state, solved.settle.temperature.0
        );
        match b.thaw {
            None => no_thaw = no_thaw.or(Some(solved)),
            Some(_) => thaws = thaws.or(Some(solved)),
        }
    }
    let frozen = no_thaw.expect("a bistable orbit no CO₂ thaws");
    assert!(frozen_over(&frozen.settle), "{frozen:?}");
    assert_eq!(frozen.state, ClimateState::Frozen, "{frozen:?}");
    let snowball = thaws.expect("a bistable orbit a CO₂ thaws");
    assert_eq!(snowball.state, ClimateState::Snowball, "{snowball:?}");
}

#[test]
#[ignore = "heavy: sixty-one dry settles under a sulphuric deck"]
fn a_dry_acid_air_settles_through_the_deck_onset() {
    // A dry air rich in SO₂ makes a sulphuric deck once warm: its albedo
    // climbs from the bare ground's to the deck's as it warms, which must
    // leave a fixed point the albedo settles on (a step would swing
    // between a warm deckless and a cold decked climate).
    let mars = inputs_at(&MARS, 1.0, SUN);
    let dry = ClimateInputs {
        air: ACID_AIR,
        water: science::units::Kilogram(0.0),
        runaway: false,
        carbon: Bar(ACID_AIR.co2),
        ..mars
    };
    let (start, step, count) = ACID_ORBITS;
    let mut near = 0;
    for i in 0..=count {
        let au = start + step * i as f64;
        let inputs = lit(&dry, flux_at(au));
        let c = settled(&inputs, Numerics::DEFAULT);
        if (ACID_BAND.0..=ACID_BAND.1).contains(&c.temperature.0) {
            near += 1;
        }
        let mut body = ClimateBody::new(&inputs, Numerics::DEFAULT);
        let derived = body.derived_albedo(c.temperature, &c.air);
        let mut t_eq = |a: Fraction| (body.starlight(a).0 / STEFAN_BOLTZMANN).powf(0.25);
        let gap = (t_eq(derived) - t_eq(c.bond_albedo)).abs();
        assert!(
            gap < ACID_GAP_K,
            "{au} AU: {c:?}, derived albedo {derived:?}"
        );
    }
    assert!(near >= ACID_BAND.2, "too few orbits near the onset: {near}");
}

#[test]
#[ignore = "heavy: the Sun's outer edge and Earth at four orbits"]
fn snowballs_are_rare_near_earth_and_common_near_the_edge() {
    // Earth spent about a hundredth of its history in snowball states
    // (Hoffman et al. 2017, doi:10.1126/sciadv.1600983): near, in 0.013 to
    // 0.016 (as `earth_snowballs_as_often_as_hoffman_dates`), far above it
    // near the edge, and no temperate state past the edge to be drawn
    // from: it freezes for good whatever the draw.
    let edge = outer_edge_au(SUN, &Numerics::DEFAULT);
    let chance = |au: f64| {
        solve(&earth_at(au), WARM_DRAW)
            .bistable
            .map(|b| b.chance.get())
    };
    let near = chance(1.0).expect("Earth at 1 AU is bistable");
    let far_orbit = edge_orbit(edge, SNOWBALL_SHARES[2]);
    let far = chance(far_orbit).expect("bistable near the edge");
    println!("chance at 1.0 AU {near:.4}, at {far_orbit:.3} AU {far:.4}; edge {edge:.3} AU");
    assert!((0.013..=0.016).contains(&near), "{near}");
    assert!(far > near, "{far} vs {near}");
    let beyond = solve(&earth_at(edge_orbit(edge, EDGE_SHARES[3])), WARM_DRAW);
    assert_eq!(beyond.bistable, None);
    assert_eq!(beyond.state, ClimateState::Frozen, "{beyond:?}");
}

#[test]
#[ignore = "heavy: the Sun's outer edge, then up to four frozen branches"]
fn a_snowball_holds_the_co2_its_thermostat_reaches() {
    // Frozen, weathering stops and volcanoes build the CO₂ up to what
    // thaws the ice, or, where none within reach does, to the most its
    // column lets warm (the maximum greenhouse; issue #522).
    let edge = outer_edge_au(SUN, &Numerics::DEFAULT);
    let mut seen = 0;
    for au in edge_scan(edge) {
        let inputs = earth_at(au);
        let Solved {
            settle: c,
            state,
            bistable,
        } = solve(&inputs, COLD_DRAW);
        let Some(b) = bistable else { continue };
        let cap = ClimateBody::new(&inputs, Numerics::DEFAULT)
            .greenhouse_co2(&inputs.air)
            .0;
        let reached = b.thaw.map_or(cap, |t| t.0.min(cap));
        println!(
            "{au:.3} AU: CO₂ {:.4} bar, thaw {:?}, cap {cap:.4}, {state:?}, {:.1} K",
            c.air.co2, b.thaw, c.temperature.0
        );
        assert!(frozen_over(&c), "{au} AU: {c:?}");
        if state == ClimateState::Snowball {
            seen += 1;
            assert!(
                (c.air.co2 / reached - 1.0).abs() < REACHED_AGREEMENT,
                "{au} AU: {c:?}"
            );
        } else {
            // Frozen for good: what the cap leaves in the air over so
            // cold a surface, the rest frosted onto the ground.
            assert!(c.air.co2 <= cap * CAP_ROOM, "{au} AU: {c:?}");
            assert!(c.air.co2 <= kept_co2(&c) * KEPT_ROOM, "{au} AU: {c:?}");
        }
    }
    assert!(seen >= SNOWBALLS_SEEN, "too few snowballs: {seen}");
}

#[test]
#[ignore = "heavy: a wet Earth just past its runaway limit"]
fn a_planet_just_past_the_runaway_limit_runs_away_hot() {
    // An Earth with 1 % of its mass in water at 1.01 times its star's
    // runaway limit: its water runs away and its surface settles well
    // above water's critical point, not on it.
    let body = ReferenceBody {
        water_fraction: RUNAWAY_WATER,
        ..EARTH
    };
    let au = (RUNAWAY_OVER * runaway_flux(SUN).0).powf(-0.5);
    let inputs = inputs_from(&body, au, SUN, Some(NO_ESCAPE));
    assert!(inputs.runaway, "{inputs:?}");
    assert!(inputs.water.0 > 0.0, "{inputs:?}");
    let solved = solve(&inputs, WARM_DRAW);
    let t = solved.settle.temperature.0;
    println!("runaway at {au:.4} AU: {t:.1} K, {:?}", solved.state);
    assert_eq!(solved.state, ClimateState::Runaway, "{solved:?}");
    assert!(
        t > CRITICAL_POINT_TEMPERATURE.0 + RUNAWAY_MARGIN_K,
        "{solved:?}"
    );
}

/// Source: an F0V star's effective temperature, 7220 K (Pecaut &
/// Mamajek 2013, ApJS 208, 9, doi:10.1088/0067-0049/208/1/9, Table 5;
/// legacy `classify.rs` `f0v_at`), past the 7200 K hot end of Kopparapu
/// et al. 2014's fit, which `runaway_flux` clamps to.
const F0V: Kelvin = Kelvin(7220.0);

/// Definition: the flux of the boil tests, S⊕, round an [`F0V`] star,
/// 1.2963: a test input just under its runaway limit
/// (`runaway_flux(F0V)`, 1.29636 S⊕), in the band where the model boils
/// under that limit. The escaping boil's planet there (3e-4 of water, a
/// hundredth of its N₂, [`HUNDREDTH_OF_N2`]) absorbs 291.03 W/m² (Bond
/// albedo 0.340) against its saturated column's peak of 289.94 W/m², and
/// boils (a margin of 1.1 W/m²), while its thermostat still balances at
/// 288 K on the climate's column. The flux cannot rise (the runaway
/// limit caps it), so the band is held by the N₂; legacy's 1.28 S⊕ lies
/// under the model's boil
/// threshold (study 26, "The boil": the threshold sits about 15 % above
/// Kopparapu et al. 2014's for the Sun, #680). The band widens when #680
/// lowers the threshold.
const F0V_BOILING_FLUX: f64 = 1.2963;

/// Definition: the water share of the boil that escapes all its water,
/// 3e-4 of the mass (legacy `a_boil_that_escapes_all_its_water_ends_dry`).
const ESCAPING_WATER: f64 = 3.0e-4;

/// Definition: the volatile scatter that cuts the N₂ to a hundredth, dex,
/// in the order CO₂, N₂, S, CH₄, NH₃: `log10(0.01) = -2` on N₂, a test
/// input that puts both boil tests' first runs in the boil band. Legacy
/// `recondensed()` cut it to a tenth (`N2_SHARE`, 0.1, so that the F
/// star's Rayleigh scattering does not keep the first run from boiling,
/// #522). The absorbed light hardly moves with the N₂ (291.0 W/m² at a
/// tenth and at a hundredth): the dry gas sets the saturated column's
/// peak instead, through the air's mean molar mass, which sets how much
/// vapour a column of given pressure holds. Earth's 0.0129 bar of argon
/// (#694, 40 g/mol) on a tenth of the N₂ raises that peak from 290.85 to
/// 292.31 W/m² (the same pressure of N₂ only to 291.12; with argon given
/// N₂'s molar mass, argon and N₂ match, and with N₂'s heat capacity the
/// argon still raises it by 1.46 W/m²), above the 290.88 W/m² the planet
/// absorbs: a tenth no longer boils. Measured on the merged branch
/// (stellar spectra of #655, argon of #694), the first runs' peak against
/// their absorbed light, W/m²:
///
/// | N₂ cut | escaping boil | recondensed boil |
/// | --- | --- | --- |
/// | a tenth (−1) | 292.31 against 290.88: no boil | 293.16 against 291.72: no boil |
/// | 10^−1.3 | 290.72 against 291.10: boils | 291.43 against 292.09: boils |
/// | 10^−1.6 | 290.15 against 291.08: boils | 290.68 against 292.13: boils |
/// | a hundredth (−2) | 289.94 against 291.03: boils | 290.32 against 292.12: boils |
///
/// A hundredth gives margins of 1.1 and 1.8 W/m², near the most the cut
/// can give (the column's peak with no N₂ but its argon tends to about
/// 289.7 W/m²). With the rule's N₂ the escaping boil's planet has a Bond
/// albedo of 0.368 and absorbs 278.7 W/m² (measured before the argon,
/// which only raises the peak), far under the moist limit, and does not
/// boil.
const HUNDREDTH_OF_N2: [f64; 5] = [0.0, -2.0, 0.0, 0.0, 0.0];

/// Definition: the leftover the steam run hands its recondensed water,
/// 0.03 bar on the formed body (legacy `recondensed()`'s `left_bar`).
const LEFTOVER: Bar = Bar(0.03);

/// Definition: the XUV fluence of the recondensed boil, J/m², 1e14: a
/// test input about 73 times under the Sun's scaled to
/// [`F0V_BOILING_FLUX`] (7.3e15). The water is sized so that the steam
/// run's `9 H` (about 6.5e21 kg at this fluence) leaves [`LEFTOVER`]:
/// 3.6e-4 of the mass. The planet's first run then lies in the boil band:
/// Bond albedo 0.338, 292.12 W/m² absorbed against its saturated
/// column's peak of 290.32 W/m² under [`HUNDREDTH_OF_N2`], and it boils;
/// its recondensed run (Bond albedo 0.353, 285.4 W/m²) keeps too little
/// water to boil. A larger fluence
/// sizes more water, with more ice volatiles and a higher albedo, and
/// takes the first run out of the band. It was 2e14 (7.2e-4 of water)
/// until two changes each took that planet out of the band, and each
/// found it boiling again at 1e14: the stellar spectra of #655 (they
/// raise this F star's planet's albedo by about 0.007, 0.324 to 0.330 at
/// 2e14, and its first run absorbed 295.3 W/m² and did not boil), and
/// the argon of #694 (Earth's 0.013 bar of argon on the case's tenth of
/// its N₂; checked by zeroing the argon). After the two met, 1e14 no
/// longer boiled at a tenth of the N₂ either: the cut to a hundredth
/// ([`HUNDREDTH_OF_N2`]) holds the band instead of a smaller fluence.
const RECONDENSED_FLUENCE: f64 = 1.0e14;

/// Definition: the rounds of the recondensed boil's water sizing, 3
/// (legacy `recondensed()`): the body barely moves with its water.
const SIZING_ROUNDS: usize = 3;

/// Definition: the share of its formed water a recondensed boil keeps at
/// most, 1e-3 (legacy `a_boil_whose_leftover_recondenses_keeps_its_ocean`).
const RECONDENSED_SHARE: f64 = 1e-3;

/// Definition: the cool small planet's orbit, 1.5 AU, and its water,
/// 1e-3 of the mass (legacy `a_cool_small_planet_does_not_boil`).
const COOL_SMALL: (f64, f64) = (1.5, 1.0e-3);

/// Earth's composition with a water share `water` and a volatile
/// `scatter`, at [`F0V_BOILING_FLUX`] round an [`F0V`] star (its orbit
/// `flux^-½` AU), not migrated.
fn f0v_earth(trace: &mut Trace, water: f64, scatter: Option<[f64; 5]>) -> (Trial, Drawn) {
    let au = F0V_BOILING_FLUX.powf(-0.5);
    let t = trial(trace, 1, EARTH.mass, au, 0.0);
    let t = with_star(trace, &t, F0V);
    let drawn = Drawn::fixed(
        &t,
        trace,
        EARTH.core_mass_fraction,
        water,
        0.0,
        Some(false),
        0.0,
        None,
        scatter,
    );
    (t, drawn)
}

/// The planet `t` of composition `d`, settled at Earth's obliquity with
/// the snowball draw of seed 1.
fn settle(trace: &mut Trace, t: &Trial, d: &Drawn) -> Settled {
    let rot = rotation(trace, EARTH_OBLIQUITY, false);
    settled_with(t, d, &rot, Seed::new(1).derive(LABELS.snowball), trace).unwrap()
}

#[test]
#[ignore = "heavy: a full boil loop, the first and the steam runs, 10.8 s in release"]
fn a_boil_that_escapes_all_its_water_ends_dry() {
    assert!(F0V_BOILING_FLUX < runaway_flux(F0V).0);
    let mut trace = Trace::new();
    let (t, d) = f0v_earth(&mut trace, ESCAPING_WATER, Some(HUNDREDTH_OF_N2));
    let s = settle(&mut trace, &t, &d);
    let verdicts = nodes_of(&trace, codes::CLIMATE_BOILS);
    assert_eq!(verdicts[0].1, NodeValue::Flag(true), "{verdicts:?}");
    let water = &s.bare.surface.as_ref().unwrap().water;
    assert!(*water.runaway.value() && *water.boiled.value(), "{water:?}");
    assert_eq!(derived_from(&trace, water.boiled.node()), [verdicts[0].0]);
    let c = s.climate.as_ref().unwrap();
    assert_ne!(
        c.liquid.map(|l| *l.value()),
        Some(LiquidKind::Water),
        "{c:?}"
    );
    assert_eq!(water.inventory.value().0, 0.0, "{water:?}");
    assert_eq!(verdicts.len(), 1, "{verdicts:?}");
}

/// The bulk water share, under [`RECONDENSED_FLUENCE`], whose steam run hands [`LEFTOVER`] to its
/// recondensed water: the surface water `9 H + L / k` (`L` the mass of
/// [`LEFTOVER`] on the formed body, `H` its energy-limited hydrogen loss,
/// `k` the cosmic shoreline's share of a first steam run) over
/// [`SURFACE_WATER_SHARE`] of the mass, in [`SIZING_ROUNDS`] rounds.
fn recondensing_water() -> f64 {
    let mut water = ESCAPING_WATER;
    for _ in 0..SIZING_ROUNDS {
        let mut trace = Trace::new();
        let (t, d) = f0v_earth(&mut trace, water, Some(HUNDREDTH_OF_N2));
        let t = with_fluence(&mut trace, &t, RECONDENSED_FLUENCE);
        let Ground::Formed(f) = formed(&t, &d, &mut trace).unwrap() else {
            panic!("an Earth has a surface");
        };
        let exposed = f.body.exposed();
        let (_, fluence) = f.trial.sources.band_total(|s| &s.xuv);
        let hydrogen =
            energy_limited_loss(Metre::from(*exposed.radius.value()), exposed.mass, fluence);
        let verdict = trace
            .leaf(Code::of("test.boils"), true, Provenance::Imposed)
            .unwrap();
        let steam = boiled(&f.trial, &exposed, f.surface, verdict, &mut trace).unwrap();
        let ground = surface_pass(&f, steam, None, &mut trace).unwrap();
        bare_air(&ground, true, &mut trace).unwrap();
        let kept = nodes_of(&trace, codes::AIR_SHORELINE_KEPT);
        let NodeValue::Number { value: share, .. } = kept.last().unwrap().1 else {
            panic!("a share: {kept:?}");
        };
        let bulk = f.body.bulk();
        let per_kg = column_pressure(
            Kilogram(1.0),
            *bulk.surface_gravity.value(),
            Metre::from(*bulk.radius.value()),
        );
        let left = LEFTOVER.0 / per_kg.0;
        let surface = WATER_PER_HYDROGEN * hydrogen.0 + left / share;
        water = surface / (SURFACE_WATER_SHARE * Kilogram::from(EarthMass(EARTH.mass)).0);
    }
    water
}

#[test]
#[ignore = "heavy: a full boil loop, three runs and the water sizing, 4.3 s in release"]
fn a_boil_whose_leftover_recondenses_keeps_its_ocean() {
    let mut trace = Trace::new();
    let (t, d) = f0v_earth(&mut trace, recondensing_water(), Some(HUNDREDTH_OF_N2));
    let t = with_fluence(&mut trace, &t, RECONDENSED_FLUENCE);
    let Ground::Formed(f) = formed(&t, &d, &mut trace.clone()).unwrap() else {
        panic!("an Earth has a surface");
    };
    let all = f.surface.value().0;
    let s = settle(&mut trace, &t, &d);
    let verdicts: Vec<_> = nodes_of(&trace, codes::CLIMATE_BOILS)
        .into_iter()
        .map(|(_, v)| v)
        .collect();
    assert_eq!(verdicts, [NodeValue::Flag(true), NodeValue::Flag(false)]);
    let water = &s.bare.surface.as_ref().unwrap().water;
    assert!(
        *water.boiled.value() && !*water.runaway.value(),
        "{water:?}"
    );
    let kept = water.inventory.value().0;
    assert!(
        kept > 0.0 && kept < RECONDENSED_SHARE * all,
        "{kept} of {all}"
    );
    let c = s.climate.as_ref().unwrap();
    assert_eq!(
        c.liquid.map(|l| *l.value()),
        Some(LiquidKind::Water),
        "{c:?}"
    );
    assert!(derived_from(&trace, water.runaway.node()).contains(&water.inventory.node()));
}

#[test]
#[ignore = "heavy: a full boil loop, 6.3 s at opt-level 0, 0.3 s in release"]
fn a_cool_small_planet_does_not_boil() {
    let mut trace = Trace::new();
    let (au, w) = COOL_SMALL;
    let (t, d) = water_world_trial(&mut trace, SMALL_MASS, EARTH.core_mass_fraction, w, au);
    let Ground::Formed(f) = formed(&t, &d, &mut trace.clone()).unwrap() else {
        panic!("a small planet has a surface");
    };
    let s = settle(&mut trace, &t, &d);
    let water = &s.bare.surface.as_ref().unwrap().water;
    assert!(
        !*water.runaway.value() && !*water.boiled.value(),
        "{water:?}"
    );
    assert_eq!(*water.inventory.value(), *f.surface.value());
}

const _: () = assert!(FINER_SCAN_RATIO < SCAN_RATIO);
const _: () = assert!(FINER_THAW_STEPS > THAW_STEPS);
const _: () = assert!(LIGHTER_DAMPING < ALBEDO_DAMPING);

/// Source: an M9V star's effective temperature, 2400 K (Pecaut & Mamajek
/// 2013, ApJS 208, 9, doi:10.1088/0067-0049/208/1/9, Table 5), the star of
/// the #492 body; the cool end of Kopparapu et al. 2014's fit (2600 K),
/// which `runaway_flux` clamps to.
const M9V: Kelvin = Kelvin(2400.0);

/// Definition: the steam over which a column is a deep supercritical one,
/// 1e5 bar (legacy `the_492_super_earth_is_a_steam_world` and #438: the
/// census's empty band runs from 1e3 to 1e5 bar).
const DEEP_STEAM: Bar = Bar(1.0e5);

/// Source: a steam world's surface, over 1000 K, under every source's
/// post-runaway surface temperature: Barnes et al. 2013 (Astrobiology 13,
/// 225, arXiv:1203.5104, about 1400 K), Kasting 1988 (Icarus 74, 472,
/// about 1500 K) and Goldblatt et al. 2013 (Nature Geosci. 6, 661, about
/// 1600 K), as study 26 § 4 cites them (legacy
/// `the_492_super_earth_is_a_steam_world`'s bound: past the moist
/// plateau, not held at water's critical point).
const STEAM_WORLD_SURFACE: Kelvin = Kelvin(1000.0);

/// Definition: the thick steam runaway of #438 (legacy `climate.rs`
/// `a_thick_steam_runaway_settles_past_the_end_of_the_plateau`): 1 M⊕,
/// a core share of 0.325, 30 % water, at 0.7 AU of the Sun.
const THICK_STEAM: (f64, f64, f64, f64) = (1.0, 0.325, 0.3, 0.7);

/// Source: the surface of a thick steam runaway past the plateau, 1500 to
/// 2500 K: Kasting 1988 (*Icarus* 74, 472, doi:10.1016/0019-1035(88)90116-9;
/// over 1500 K for a full ocean), Goldblatt et al. 2013 (*Nat. Geosci.*
/// 6, 661, doi:10.1038/ngeo1892; a stable steam state from about
/// 1600 K), Boukrouche et al. 2021 § 4.3 (1835 K to 2451 K); study 26's
/// window.
const THICK_STEAM_WINDOW: (Kelvin, Kelvin) = (Kelvin(1500.0), Kelvin(2500.0));

/// Whether `settled` stands under a steam envelope: its last shell is
/// `Steam`.
fn under_steam(settled: &Settled) -> bool {
    settled
        .bare
        .body
        .layers()
        .last()
        .is_some_and(|s| *s.material.value() == Material::Steam)
}

#[test]
#[ignore = "heavy: a steam world settled, its pass on the envelope, 8.4 s at opt-level 0, 0.35 s in release"]
fn the_492_super_earth_is_a_steam_world() {
    let mut trace = Trace::new();
    let (t, d) = water_world_trial(&mut trace, MASS_492, CORE_492, WATER_492, AU_492);
    let t = with_star(&mut trace, &t, M9V);
    assert!(t.sources.total_flux().0 > runaway_flux(M9V).0);
    let s = settle(&mut trace, &t, &d);
    let hides = s.hides_surface.expect("a body with a surface is judged");
    assert!(*hides.value());
    let c = s.climate.as_ref().unwrap();
    let h2o = *c.atmosphere.partial[&Gas::H2o].value();
    let temperature = *c.global.surface_temperature.value();
    assert!(h2o > DEEP_STEAM, "{h2o:?}");
    assert!(temperature > STEAM_WORLD_SURFACE, "{temperature:?}");
    assert!(under_steam(&s));
    // Over the dry core under the steam (Turbet et al. 2020 § 3.1); the
    // condensed twin is not a bound outside their grid (#684).
    let layers = s.bare.body.layers();
    let core = layers[layers.len() - 2];
    assert!(EarthRadius::from(*core.outer_radius.value()).0 < s.bare.body.bulk().radius.value().0);
    let code_of = |id| trace.node(id).unwrap().code.as_str().to_owned();
    let lost = nodes_of(&trace, codes::WATER_STEAM_LOST);
    let (last, _) = lost.last().expect("a runaway loses steam");
    assert!(
        derived_from(&trace, *last)
            .into_iter()
            .any(|id| code_of(id) == codes::RADIUS_STEAM)
    );
    let water = &s.bare.surface.as_ref().unwrap().water;
    assert!(
        derived_from(&trace, water.boiled.node())
            .into_iter()
            .any(|id| code_of(id) == codes::CLIMATE_BOILS)
    );
    let lines: Vec<_> = nodes_of(&trace, codes::CLIMATE_HIDES_SURFACE)
        .into_iter()
        .map(|(_, v)| v)
        .collect();
    assert_eq!(lines, [NodeValue::Flag(true), NodeValue::Flag(true)]);
}

#[test]
#[ignore = "heavy: a thick steam runaway settled, its pass on the envelope, 7.4 s at opt-level 0, 0.34 s in release"]
fn a_thick_steam_runaway_settles_past_the_end_of_the_plateau() {
    let mut trace = Trace::new();
    let (mass, core, water, au) = THICK_STEAM;
    let (t, d) = water_world_trial(&mut trace, mass, core, water, au);
    let s = settle(&mut trace, &t, &d);
    let w = &s.bare.surface.as_ref().unwrap().water;
    assert!(*w.runaway.value(), "{w:?}");
    let c = s.climate.as_ref().unwrap();
    let h2o = *c.atmosphere.partial[&Gas::H2o].value();
    let temperature = *c.global.surface_temperature.value();
    assert!(h2o > DEEP_STEAM, "{h2o:?}");
    let (low, high) = THICK_STEAM_WINDOW;
    assert!((low.0..=high.0).contains(&temperature.0), "{temperature:?}");
    assert!(c.liquid.is_none(), "{c:?}");
    assert!(*s.hides_surface.expect("judged").value());
    assert!(under_steam(&s));
}

/// An orbit just past a methane Earth's outer edge (1.4355 AU, measured by
/// `one_greenhouse_search_on_a_condensing_air` on #729's branch) where the
/// albedo's fixed point has none (#729): a warm climate at its maximum
/// greenhouse derives a Rayleigh albedo that collapses its CO₂, and the
/// collapsed climate derives one that warms it back.
const CYCLING_AU: f64 = 1.44;

/// Definition: the most albedo steps the cycling orbit may take before
/// its bracket closes, 40: the bracket narrows from the swing (about
/// 0.02 in albedo) to [`super::ALBEDO_BRACKET`] in about 15 halvings;
/// before #729 it ran to the 200 of [`super::ALBEDO_ITERATIONS`].
const CYCLING_STEPS: usize = 40;

#[test]
#[ignore = "heavy: a methane Earth just past its outer edge, a cycling climate"]
fn a_climate_with_no_fixed_point_stops_on_its_cold_side() {
    let like = ClimateInputs {
        air: PartialPressures {
            ch4: titan().air.ch4,
            ..earth().air
        },
        ..earth()
    };
    let solved = solve_with(&moved(&like, CYCLING_AU), WARM_DRAW, &Numerics::DEFAULT);
    let c = &solved.settle;
    println!(
        "{CYCLING_AU} AU: {:.3} K, CO2 {:.4} bar, {} steps, cycles {}, converged {}",
        c.temperature.0, c.air.co2, c.steps, c.cycles, c.converged
    );
    assert!(c.cycles && !c.converged, "{c:?}");
    assert!(c.steps <= CYCLING_STEPS, "{} steps", c.steps);
    // The colder side: frozen over, its thermostat lost.
    assert!(frozen_over(c) && !c.regulated, "{c:?}");
}
