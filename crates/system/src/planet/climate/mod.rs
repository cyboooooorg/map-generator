//! The climate of a planet with a surface: the temperature at which its
//! column sends out what it absorbs, with the albedo, the frosts, the
//! water vapour and the CO₂ that follow that temperature.
//!
//! Ported from `legacy-final:crates/mapgen/src/planet/climate.rs`, with
//! the values of study
//! [25](https://github.com/cyboooooorg/map-generator/wiki/25-climate-solver-audit)
//! (`docs/science/planet-atmospheres.md`, "The climate solver of the
//! rewrite (study 25)"). The laws live in `science` (the balance of one
//! column, `science::balance`; the cover, the frosts and the condensation,
//! `science::surface`; the shortwave budget, `science::radiation`); this
//! module holds the solver that settles a body on them:
//!
//! - [`ClimateInputs`]: what the climate reads of the bare planet, as
//!   plain values, and the [`Light`] of each of its stars;
//! - [`body::ClimateBody`]: those inputs with their caches, the body's
//!   albedo, emissivity, frost and moist plateau at a temperature;
//! - [`wet`]: the coldest balance of a surface whose vapour follows its
//!   temperature, and the collapse of a CO₂ its surface cannot keep;
//! - [`carbon`]: the carbonate thermostat and the maximum greenhouse;
//! - [`settle`]: the fixed points of the condensation of its air and of
//!   its albedo and emissivity, from a warm or an ice-covered start;
//! - [`branches`]: the temperate and frozen branches, the CO₂ that thaws
//!   a snowball, its chance (`science::snowball`) and the state the
//!   snowball draw finds the body in;
//! - [`record`]: the settled climate in the cause trace, node by node.
//!
//! The solver's own numerical constants (its loops, not its laws) are
//! here, each a `Definition:` or a `Calibration:` held by the convergence
//! test that study 25's table "Solver constants" names; [`Numerics`]
//! carries them so that a test can swap one.
//!
//! Several stars (#592): the body keeps one Bond albedo, the flux-weighted
//! mean `Σ wᵢ Aᵢ` of the shortwave budgets at each star's temperature
//! (`wᵢ = fluxᵢ / Σ flux`), so that `S₀/4 · Σ fluxᵢ (1 − Aᵢ) = S₀/4 · Σ
//! flux · (1 − Σ wᵢ Aᵢ)` exactly; the runaway albedo is weighted the
//! same way. The longwave, the skin and the emissivity do not depend on
//! the star.

pub(crate) mod body;
pub(crate) mod branches;
pub(crate) mod carbon;
pub(crate) mod record;
pub(crate) mod settle;
pub(crate) mod wet;

#[cfg(test)]
mod branches_tests;
#[cfg(test)]
mod carbon_tests;
#[cfg(test)]
pub(crate) mod fixture;
#[cfg(test)]
mod heavy_tests;
#[cfg(test)]
mod settle_tests;
#[cfg(test)]
mod wet_tests;

use super::bare::Bare;
use science::balance::{SOLVER, Solver};
use science::radiation::gases::PartialPressures;
use science::radiation::ozone::ozone_column;
use science::relief::volcanoes::heat_flux;
use science::units::{
    Bar, Degree, Dex, DobsonUnit, EarthFlux, Fraction, Kelvin, Kilogram, Metre,
    MetrePerSecondSquared, Radian, Watt, WattPerSquareMetre,
};
use timeline::{Atmosphere, Gas, TectonicRegime, Traced};

/// Calibration: the largest ratio between two surface temperatures the
/// wet balance tries in turn on its way up, 1.03 (legacy `SCAN_RATIO`):
/// fine enough not to step over a balance of the moist column, whose
/// outgoing radiation turns over within tens of kelvin. Study 25
/// ("Solver constants"): at 1.01 the same first balance, on Earth and a
/// sweep across the runaway limit, a convergence test in tier 3.
pub(crate) const SCAN_RATIO: f64 = 1.03;

/// Definition: the surface temperature at which the wet balance first
/// tries the saturated column for a runaway, 380 K (legacy
/// `RUNAWAY_PROBE_K`), near the column's own maximum (294.6 W/m² at
/// 360 K): a shortcut that changes no result (study 25, "The runaway
/// branch"; `the_probe_changes_no_balance`).
pub(crate) const RUNAWAY_PROBE_K: Kelvin = Kelvin(380.0);

/// Calibration: the share by which the CO₂ of a collapsed climate may
/// exceed what its surface keeps once settled, 1e-4 (legacy
/// `COLLAPSE_TOLERANCE`). Study 25 ("Solver constants"): at 1e-5 the
/// temperature moves under 0.01 K, a convergence test in tier 3.
pub(crate) const COLLAPSE_TOLERANCE: f64 = 1e-4;

/// Definition: the most steps of a collapse, 300, a guard: a collapse
/// that settles under it does not depend on it, and one that stops at it
/// leaves its climate not converged. Legacy's 100 was a cap a collapsing
/// Earth at 1.6 AU stops under by [`COLLAPSE_TOLERANCE`] (study 25,
/// "Solver constants"; `collapse_iterations_is_converged`), but with
/// #703's pairs a methane Earth just past its outer edge collapses from
/// 14.2 to about 9.3 bar with a gap of only 0.02 to 0.09 bar, and needs
/// up to 91 steps at [`COLLAPSE_STEP`], 140 at half of it and 207 with
/// the plain step (measured at 1.25 to 1.268 AU); stopped at 100, the
/// climate kept depended on the step. 300 is the plain step's 207 with
/// half again as margin.
pub(crate) const COLLAPSE_ITERATIONS: usize = 300;

/// Calibration: the longest step of a collapse, as a share of its CO₂,
/// 0.005 (#731): 0.085 bar at 17 bar. A warm balance whose pocket of CO₂
/// is narrower than one step can be stepped over, where the plain
/// iteration would stop in it; at half of it a methane Earth's climates
/// around its outer edge did not move before #703's pairs, a convergence
/// test in tier 3 (`collapse_step_is_converged`). Since the pairs and
/// #746, those just past the edge (1.261 to 1.268 AU) move by up to
/// 5.6 K, and the test fails: open ("A collapse from the fold (#731)" in
/// `docs/science/planet-atmospheres.md`).
/// Zero steps the collapse plainly, as legacy did.
pub(crate) const COLLAPSE_STEP: f64 = 0.005;

/// Calibration: the bisection steps of the thaw search, 13 (legacy
/// `THAW_STEPS` was 12). Study 25 ("Solver constants"): at 16 steps
/// Earth's thaw CO₂ must move under 0.1 %, held by a convergence test in
/// tier 3 (`thaw_steps_is_converged`). Measured on the reference Earth:
/// 0.180932 bar at 12 steps (+0.165 % against 16), 0.180748 at 13
/// (+0.063 %), 0.180657 at 14, 0.180634 at 16, 0.180625 at 19: 13 is the
/// least count that holds the row.
pub(crate) const THAW_STEPS: usize = 13;

/// Calibration: the move of the surface temperature, K, under which the
/// albedo's fixed point has settled, 0.1 K (legacy `ALBEDO_TOLERANCE_K`).
/// Study 25 ("Solver constants"): at 0.01 K the reference climates move
/// under 0.1 K, a convergence test in tier 1.
pub(crate) const ALBEDO_TOLERANCE_K: Kelvin = Kelvin(0.1);

/// Definition: the most steps of the albedo's fixed point, 200 (legacy
/// `ALBEDO_ITERATIONS`), a cap the reference climates settle under
/// (study 25, "Solver constants").
pub(crate) const ALBEDO_ITERATIONS: usize = 200;

/// Definition: the least share of the way the albedo's fixed point moves
/// at each step, 0.5 (legacy `ALBEDO_DAMPING`): a damping that does not
/// change the converged climate (study 25, "Solver constants": at 0.25
/// the converged climates agree, tier 3).
pub(crate) const ALBEDO_DAMPING: f64 = 0.5;

/// Definition: the width, in Bond albedo and in emissivity (both
/// fractions, which the fixed point moves together), under which the
/// bracket of the albedo's fixed point is closed without a settled
/// climate, 1e-6 (#729):
/// a stop tolerance, not physics. Once the push has turned back, the
/// fixed point lies between an albedo that pushes up and one that pushes
/// down; a bracket this narrow that has not settled holds a jump of the
/// climate (no fixed point, the climate cycles), not a root. At 1e-6 the
/// radiative temperature moves by about `T Δa / (4 (1 − a))`, 1e-4 K at
/// 288 K, a thousandth of [`ALBEDO_TOLERANCE_K`]; the emissivity's,
/// `T Δε / (4 ε)`, is as small.
pub(crate) const ALBEDO_BRACKET: f64 = 1e-6;

/// Definition: the most steps of the condensation's fixed point, 100
/// (legacy `CONDENSE_ITERATIONS`), a cap Titan and Triton settle under
/// (study 25, "Solver constants").
pub(crate) const CONDENSE_ITERATIONS: usize = 100;

/// Calibration: the width in `ln CO₂` under which the thermostat's root
/// search stops, 1e-3, a tenth of a percent of the CO₂ (legacy
/// `THERMOSTAT_TOLERANCE`). Study 25 ("Solver constants"): at 1e-4
/// Earth moves under 0.01 K, a convergence test in tier 1
/// (`the_thermostat_tolerance_is_converged`).
pub(crate) const THERMOSTAT_TOLERANCE: f64 = 1e-3;

/// Calibration: the golden-section steps of the maximum greenhouse, 18
/// (legacy `GREENHOUSE_STEPS`), 0.618¹⁸ of its range. Study 25 ("Solver
/// constants"): at 30 steps the outer edge moves under 0.001 AU, a
/// convergence test in tier 3.
pub(crate) const GREENHOUSE_STEPS: usize = 18;

/// Definition: the decades of CO₂ under the most the air can hold that
/// the maximum greenhouse searches, 4 (legacy `GREENHOUSE_DECADES`): a
/// search range the reference maxima lie inside (study 25, "Solver
/// constants"; `the_maximum_greenhouse_lies_inside_its_range`).
pub(crate) const GREENHOUSE_DECADES: f64 = 4.0;

/// The light of one star on the planet, as plain values
/// (`timeline::StellarSource`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Light {
    /// The orbit-mean bolometric flux.
    pub flux: EarthFlux,
    /// The star's effective temperature, which sets the shortwave budget
    /// and the runaway limit.
    pub effective_temperature: Kelvin,
    /// The star's metallicity [M/H], which shapes its spectrum in the
    /// shortwave budget with its temperature (the trial's: the stars of a
    /// system share one, as their activity histories do).
    pub metallicity: Dex,
    /// The equilibrium temperature from this star alone, at the
    /// timeline's reference albedo: the warm start of the fixed point.
    pub equilibrium: Kelvin,
    /// The ozone column the air's O₂ holds under this star's light
    /// ([`ozone_column`] of the bare air's O₂ and the star's
    /// temperature), which its light crosses in the shortwave budget. The
    /// climate never changes the air's O₂ (it does not condense), so the
    /// column holds for every air the solve tries.
    pub ozone: DobsonUnit,
}

/// The rotation the climate reads (the spin, #677): the obliquity, which
/// weights the light on the ice caps, and whether the body is tidally
/// locked, whose ice forms on its night side.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Rotation {
    pub obliquity: Traced<Degree>,
    pub locked: Traced<bool>,
}

/// What the climate reads of a bare planet with a surface, as plain
/// values.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ClimateInputs {
    /// The light of each star.
    pub sources: Vec<Light>,
    /// The bare air: the inventory of what condenses.
    pub air: PartialPressures,
    /// The water kept: liquid and ice, or a runaway's steam.
    pub water: Kilogram,
    /// Whether the water ran away by its stars' light.
    pub runaway: bool,
    /// Whether the planet has plates, the carbonate thermostat.
    pub plates: bool,
    /// The internal heat through the surface.
    pub heat_flux: WattPerSquareMetre,
    /// The surface gravity.
    pub gravity: MetrePerSecondSquared,
    /// The radius.
    pub radius: Metre,
    /// The CO₂ outgassed: the carbon inventory.
    pub carbon: Bar,
    /// The CO₂ the cold trap froze onto the ground.
    pub frozen_co2: Bar,
    /// The SO₂ frost volcanoes lay on the ground.
    pub so2_frost: Bar,
    /// The volcanic intensity the snowball chance reads.
    pub volcanism: Fraction,
    /// The obliquity.
    pub obliquity: Radian,
    /// Whether the body is tidally locked.
    pub locked: bool,
}

impl ClimateInputs {
    /// The inputs of `bare`'s climate under `rotation`, with `so2_frost`
    /// of volcanic SO₂ frost on its ground; `None` for a planet that kept
    /// an envelope. The heat flux is the total internal power over the
    /// bulk's `4 π R²` (`heat_flux`), the gravity and radius the bulk's.
    /// `settled` is the one caller that sets `so2_frost` (#678).
    pub(crate) fn of(bare: &Bare, rotation: &Rotation, so2_frost: Bar) -> Option<ClimateInputs> {
        let surface = bare.surface.as_ref()?;
        let bulk = bare.body.bulk();
        let radius = Metre::from(*bulk.radius.value());
        let heat = &bare.heat;
        let total = Watt(heat.radiogenic.value().0 + heat.tidal.value().0 + heat.secular.value().0);
        let air = partial_pressures(&surface.atmosphere);
        Some(ClimateInputs {
            sources: bare
                .trial
                .sources
                .iter()
                .map(|s| Light {
                    flux: *s.flux.value(),
                    effective_temperature: *s.effective_temperature.value(),
                    metallicity: *bare.trial.metallicity.value(),
                    equilibrium: *s.equilibrium.value(),
                    ozone: ozone_column(Bar(air.o2), *s.effective_temperature.value()),
                })
                .collect(),
            air,
            water: *surface.water.inventory.value(),
            runaway: *surface.water.runaway.value(),
            plates: *surface.regime.value() == TectonicRegime::Plates,
            heat_flux: heat_flux(total, radius),
            gravity: *bulk.surface_gravity.value(),
            radius,
            carbon: *surface.outgassed_co2.value(),
            frozen_co2: *surface.frozen_co2.value(),
            so2_frost,
            volcanism: *surface.volcanism.value(),
            obliquity: Radian::from(*rotation.obliquity.value()),
            locked: *rotation.locked.value(),
        })
    }
}

/// The partial pressures of `atmosphere`, bar; a gas it lacks is none.
fn partial_pressures(atmosphere: &Atmosphere) -> PartialPressures {
    let p = |gas: Gas| atmosphere.partial.get(&gas).map_or(0.0, |p| p.value().0);
    PartialPressures {
        n2: p(Gas::N2),
        co2: p(Gas::Co2),
        h2o: p(Gas::H2o),
        ch4: p(Gas::Ch4),
        nh3: p(Gas::Nh3),
        so2: p(Gas::So2),
        o2: p(Gas::O2),
        ar: p(Gas::Ar),
    }
}

/// The numerical settings of a climate solve: one field per constant of
/// the solver, so that a convergence test swaps one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Numerics {
    /// The balance solves' tolerance and steps (`science::balance`).
    pub solver: Solver,
    /// [`SCAN_RATIO`].
    pub scan_ratio: f64,
    /// [`COLLAPSE_TOLERANCE`].
    pub collapse_tolerance: f64,
    /// [`COLLAPSE_ITERATIONS`].
    pub collapse_iterations: usize,
    /// [`COLLAPSE_STEP`]; zero steps the collapse plainly.
    pub collapse_step: f64,
    /// [`THAW_STEPS`].
    pub thaw_steps: usize,
    /// [`ALBEDO_TOLERANCE_K`].
    pub albedo_tolerance: Kelvin,
    /// [`ALBEDO_ITERATIONS`].
    pub albedo_iterations: usize,
    /// [`ALBEDO_DAMPING`].
    pub albedo_damping: f64,
    /// [`ALBEDO_BRACKET`].
    pub albedo_bracket: f64,
    /// [`CONDENSE_ITERATIONS`].
    pub condense_iterations: usize,
    /// [`THERMOSTAT_TOLERANCE`].
    pub thermostat_tolerance: f64,
    /// [`GREENHOUSE_STEPS`].
    pub greenhouse_steps: usize,
    /// [`GREENHOUSE_DECADES`].
    pub greenhouse_decades: f64,
    /// The temperature at which the wet balance first tries the
    /// saturated column ([`RUNAWAY_PROBE_K`]); `None` always scans it.
    pub probe: Option<Kelvin>,
    /// Whether the maximum greenhouse is searched once per body, on its
    /// inventory air, and kept (#593 lever 2); `false` searches it again
    /// on each condensed air the thermostat is given, as legacy did, for
    /// the tier-3 test that measures the departure.
    pub greenhouse_once: bool,
}

impl Numerics {
    /// The solver's constants: `science::balance::SOLVER` and those of
    /// this module, the probe on and the maximum greenhouse searched
    /// once.
    pub(crate) const DEFAULT: Numerics = Numerics {
        solver: SOLVER,
        scan_ratio: SCAN_RATIO,
        collapse_tolerance: COLLAPSE_TOLERANCE,
        collapse_iterations: COLLAPSE_ITERATIONS,
        collapse_step: COLLAPSE_STEP,
        thaw_steps: THAW_STEPS,
        albedo_tolerance: ALBEDO_TOLERANCE_K,
        albedo_iterations: ALBEDO_ITERATIONS,
        albedo_damping: ALBEDO_DAMPING,
        albedo_bracket: ALBEDO_BRACKET,
        condense_iterations: CONDENSE_ITERATIONS,
        thermostat_tolerance: THERMOSTAT_TOLERANCE,
        greenhouse_steps: GREENHOUSE_STEPS,
        greenhouse_decades: GREENHOUSE_DECADES,
        probe: Some(RUNAWAY_PROBE_K),
        greenhouse_once: true,
    };
}
