//! The fixed points that settle a body's climate: the condensation of its
//! air at the temperature that air gives ([`under_albedo`]), and the
//! albedo and emissivity at the temperature they give ([`settle`]), from
//! a warm start ([`warm_start`]) or an ice-covered one ([`frozen_at`]).
//! Ported from `legacy-final:crates/mapgen/src/planet/climate.rs`
//! (`Body::frozen_at`, l.1749-1763; `Pass` and `under_albedo`,
//! l.1950-2080; `warm_start`, l.2146-2157; `settle` and `settle_steps`,
//! l.2159-2272, with the secant step of #593).

use super::Numerics;
use super::body::ClimateBody;
use super::carbon::thermostat;
use super::wet::{collapsed_climate, wet_climate};
use science::balance::{LEAST_EMISSIVITY, Point, regula_falsi, surface_temperature};
use science::constants::STEFAN_BOLTZMANN;
use science::radiation::gases::Weights;
use science::surface::{
    ICE_COVERED_TEMPERATURE, ICE_FREE_TEMPERATURE, Liquid, condensables, condense, condensed_air,
    liquid, substellar_temperature, surface_vapour,
};
use science::units::{Bar, Fraction, Kelvin, WattPerSquareMetre};

/// One solve of [`under_albedo`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Pass {
    /// The air the climate leaves.
    pub air: Weights,
    /// The surface temperature.
    pub temperature: Kelvin,
    /// Whether the carbonate thermostat holds it.
    pub regulated: bool,
    /// The light its surface took, after the runaway's blend of its
    /// albedo.
    pub starlight: WattPerSquareMetre,
    /// Whether the condensation's fixed point settled within the
    /// solver's tolerance, and every collapse of CO₂ it solved settled
    /// by its own (none stopped at `collapse_iterations`).
    pub converged: bool,
    /// The steps of the condensation's fixed point: 0 when nothing
    /// condenses at the black body's temperature, under the numerics'
    /// `condense_iterations` when it stopped by its tolerance.
    pub iterations: usize,
}

/// A settled climate: the fixed point of [`settle`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Settle {
    /// The air the climate leaves.
    pub air: Weights,
    /// The surface temperature.
    pub temperature: Kelvin,
    /// The equilibrium temperature of the light the surface took,
    /// `(starlight / σ)^¼` ([`equilibrium`]).
    pub equilibrium: Kelvin,
    /// The Bond albedo it absorbed its light under: the derived one of
    /// the fixed point after the runaway's blend
    /// ([`ClimateBody::runaway_albedo`]).
    pub bond_albedo: Fraction,
    /// The emissivity its surface radiated under.
    pub emissivity: Fraction,
    /// Whether the carbonate thermostat holds it.
    pub regulated: bool,
    /// Whether the albedo's fixed point and the last condensation in it
    /// settled.
    pub converged: bool,
    /// The N₂, CH₄ and Ar on its ground at its temperature ([`condense`] of
    /// its inventory).
    pub condensate: Weights,
    /// The liquid on its surface ([`liquid`]).
    pub liquid: Option<Liquid>,
    /// The steps of the last condensation's fixed point
    /// ([`Pass::iterations`]).
    pub iterations: usize,
    /// The steps of the albedo's fixed point: under the numerics'
    /// `albedo_iterations` when it settled.
    pub steps: usize,
    /// Whether the albedo's fixed point has none: its bracket closed on a
    /// jump of the climate (#729), which then cycles between the climates
    /// on either side of it; the climate is the colder of the two, and
    /// not converged.
    pub cycles: bool,
}

/// One end of the bracket of the albedo's fixed point: the albedo and
/// emissivity a climate was solved under, the push it derives, its
/// surface temperature, and that climate (`P`, a [`Pass`] in [`settle`]).
/// `used` is the radiative equilibrium `T_eq ε^-¼` of its albedo and
/// emissivity, the line along which a positive push moves the next step.
#[derive(Clone, Copy, Debug)]
pub(crate) struct End<P> {
    pub albedo: f64,
    pub emissivity: f64,
    pub used: f64,
    pub push: f64,
    pub temperature: f64,
    pub climate: P,
}

/// What the bracket of the albedo's fixed point gives next.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Next<P> {
    /// The albedo and emissivity of the next step: the zero of the push on
    /// the line between the two ends, by the Illinois rule.
    Step { albedo: f64, emissivity: f64 },
    /// The bracket is narrower than the width it was asked for: the
    /// climate jumps there, and this is its colder end.
    Closed(End<P>),
}

/// The bracket of the albedo's fixed point once its push has turned back:
/// the last end that pushes up (`up`, a push of zero included) and the
/// last that pushes down, each with the push the Illinois rule weighs it
/// by (halved each time the other end is replaced twice in a row).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Bracket<P> {
    up: Option<(End<P>, f64)>,
    down: Option<(End<P>, f64)>,
    last_up: Option<bool>,
}

impl<P: Copy> Default for Bracket<P> {
    fn default() -> Self {
        Bracket {
            up: None,
            down: None,
            last_up: None,
        }
    }
}

impl<P: Copy> Bracket<P> {
    /// Takes `end` as the new end on its side.
    pub(crate) fn take(&mut self, end: End<P>) {
        let up = end.push >= 0.0;
        if self.last_up == Some(up) {
            let kept = if up { &mut self.down } else { &mut self.up };
            if let Some((_, weight)) = kept {
                *weight *= 0.5;
            }
        }
        if up {
            self.up = Some((end, end.push));
        } else {
            self.down = Some((end, end.push));
        }
        self.last_up = Some(up);
    }

    /// Whether both sides are known.
    pub(crate) fn closed_in(&self) -> bool {
        self.up.is_some() && self.down.is_some()
    }

    /// Whether both sides are known and each pushes towards the other: the
    /// end that pushes up (to a warmer radiative equilibrium) is the
    /// colder in `used`. Only such a bracket holds a fixed point the
    /// iteration reaches, or a jump it cycles across; one whose ends push
    /// apart holds an unstable root or jump the iteration leaves.
    pub(crate) fn attracts(&self) -> bool {
        self.up
            .zip(self.down)
            .is_some_and(|((up, _), (down, _))| up.used < down.used)
    }

    /// The next step once both sides are known, or the colder end when
    /// the two ends are under `width` apart in both albedo and emissivity
    /// (each step moves both); `None` before.
    pub(crate) fn next(&self, width: f64) -> Option<Next<P>> {
        let ((up, w_up), (down, w_down)) = self.up.zip(self.down)?;
        if (up.albedo - down.albedo).abs() < width
            && (up.emissivity - down.emissivity).abs() < width
        {
            let cold = if down.temperature < up.temperature {
                down
            } else {
                up
            };
            return Some(Next::Closed(cold));
        }
        let share = w_up / (w_up - w_down);
        Some(Next::Step {
            albedo: up.albedo + share * (down.albedo - up.albedo),
            emissivity: up.emissivity + share * (down.emissivity - up.emissivity),
        })
    }
}

/// Whether the push of the albedo's fixed point falls as the radiative
/// equilibrium it is solved under rises, between a `cold` and a `warm`
/// point (`x` the radiative equilibrium used, `f` the push): the
/// condition under which a root between them attracts the iteration, as
/// [`Bracket::attracts`] asks of a bracket's ends (the end that pushes up
/// is the colder). A root where the push rises instead is an unstable
/// balance, the middle branch of a bistable body's S curve: a disturbed
/// climate leaves it (Cahalan & North 1979: a balance is stable where the
/// light it needs rises with its temperature). A `Definition:` of the
/// solver.
pub(crate) fn attracting(cold: Point, warm: Point) -> bool {
    (warm.f - cold.f) * (warm.x - cold.x) < 0.0
}

/// The albedo that puts the radiative equilibrium at `target` under the
/// same emissivity, from `albedo` at `used`: the radiative equilibrium
/// grows as `(1 − A)^¼` ([`ClimateBody::starlight`], [`equilibrium`]), so
/// `1 − A' = (1 − A) (target / used)⁴` (a `Definition:`).
pub(crate) fn albedo_for(albedo: f64, used: f64, target: f64) -> f64 {
    1.0 - (1.0 - albedo) * (target / used).powi(4)
}

/// The radiative equilibrium `T_eq ε^-¼` of `body` under `albedo` and
/// `emissivity`, K: the coordinate the albedo's fixed point moves along.
fn radiative(body: &mut ClimateBody, albedo: f64, emissivity: f64) -> f64 {
    let light = body.starlight(fraction(albedo));
    equilibrium(light).0 / emissivity.clamp(LEAST_EMISSIVITY, 1.0).powf(0.25)
}

/// What the albedo's fixed point ([`albedo_fixed_point`]) reads of a
/// climate: the climate solved under an albedo and an emissivity, its
/// temperature, whether a thermostat holds it, the albedo and emissivity
/// it derives, and the radiative equilibrium `T_eq ε^-¼` of an albedo and
/// emissivity, K, the coordinate the fixed point moves along. A body
/// ([`ClimateBody`], through [`under_albedo`]) is one; a constructed map
/// is another, which the solver's tests drive (#767).
pub(crate) trait AlbedoMap {
    /// The climate one solve gives.
    type Climate: Copy;
    /// The climate solved under `albedo` and `emissivity`.
    fn solve(&mut self, albedo: f64, emissivity: f64) -> Self::Climate;
    /// Its surface temperature, K.
    fn temperature(climate: &Self::Climate) -> f64;
    /// Whether a thermostat holds its temperature whatever its albedo.
    fn regulated(climate: &Self::Climate) -> bool;
    /// The albedo and emissivity its temperature derives.
    fn derived(&mut self, climate: &Self::Climate) -> (f64, f64);
    /// The radiative equilibrium under `albedo` and `emissivity`, K.
    fn radiative(&mut self, albedo: f64, emissivity: f64) -> f64;
}

impl AlbedoMap for ClimateBody<'_> {
    type Climate = Pass;
    fn solve(&mut self, albedo: f64, emissivity: f64) -> Pass {
        under_albedo(self, fraction(albedo), fraction(emissivity))
    }
    fn temperature(climate: &Pass) -> f64 {
        climate.temperature.0
    }
    fn regulated(climate: &Pass) -> bool {
        climate.regulated
    }
    fn derived(&mut self, climate: &Pass) -> (f64, f64) {
        (
            self.derived_albedo(climate.temperature, &climate.air).get(),
            self.emissivity_at(climate.temperature, &climate.air).get(),
        )
    }
    fn radiative(&mut self, albedo: f64, emissivity: f64) -> f64 {
        radiative(self, albedo, emissivity)
    }
}

/// The push of `map` solved under `albedo` and `emissivity`: the point
/// (the radiative equilibrium used, and the derived one less it) and the
/// climate.
fn push_under<M: AlbedoMap>(map: &mut M, albedo: f64, emissivity: f64) -> (Point, M::Climate) {
    let pass = map.solve(albedo, emissivity);
    let x = map.radiative(albedo, emissivity);
    let (derived, derived_emissivity) = map.derived(&pass);
    let f = map.radiative(derived, derived_emissivity) - x;
    (Point { x, f }, pass)
}

/// Whether the root of `map`'s albedo's fixed point at `albedo` and
/// `emissivity`, where the push is `push`, attracts: `None` when it does,
/// the colder probe's albedo and climate when it does not. The push is
/// taken again at a radiative equilibrium `tolerance` colder (the
/// numerics' `albedo_tolerance`, the albedo moved by [`albedo_for`], the
/// emissivity kept), and the root attracts when the push falls from there
/// to the root ([`attracting`]): a climate disturbed colder is pushed back
/// warmer. One probe, on the cold side, the side an unstable root is left
/// by: a stable root within a probe of its branch's cold fold reads as
/// unstable and the fixed point goes on to the colder branch, which moves
/// that fold by under the tolerance. A body that takes no light (a
/// radiative equilibrium under the tolerance) has no push to weigh: it
/// attracts.
fn probe_root<M: AlbedoMap>(
    map: &mut M,
    albedo: f64,
    emissivity: f64,
    push: f64,
    tolerance: f64,
) -> Option<(f64, M::Climate)> {
    let used = map.radiative(albedo, emissivity);
    if used <= tolerance {
        return None;
    }
    let cold_albedo = albedo_for(albedo, used, used - tolerance).clamp(0.0, 1.0);
    let (cold, cold_pass) = push_under(map, cold_albedo, emissivity);
    let root = Point { x: used, f: push };
    (!attracting(cold, root)).then_some((cold_albedo, cold_pass))
}

/// The albedo's and the emissivity's own residuals of `map` under
/// `albedo` and `emissivity`, given what its climate derives (`derive`,
/// read after the radiative equilibrium used), K: the radiative
/// equilibrium of the derived albedo under the emissivity used, and of
/// the derived emissivity under the albedo used, each less the one used.
fn residuals_of<M: AlbedoMap>(
    map: &mut M,
    albedo: f64,
    emissivity: f64,
    derive: impl FnOnce(&mut M) -> (f64, f64),
) -> (f64, f64) {
    let used = map.radiative(albedo, emissivity);
    let (derived, derived_emissivity) = derive(map);
    (
        map.radiative(derived, emissivity) - used,
        map.radiative(albedo, derived_emissivity) - used,
    )
}

/// The albedo's and the emissivity's own residuals of `body` solved
/// under `albedo` and `emissivity` at the temperature `t` with `air`, K:
/// those [`albedo_fixed_point`] settles on ([`residuals_of`]). Only the
/// heavy tests read it: the loop calls [`residuals_of`] itself.
#[cfg(test)]
pub(crate) fn pair_residuals(
    body: &mut ClimateBody,
    albedo: f64,
    emissivity: f64,
    t: Kelvin,
    air: &Weights,
) -> (f64, f64) {
    residuals_of(body, albedo, emissivity, |body| {
        (
            body.derived_albedo(t, air).get(),
            body.emissivity_at(t, air).get(),
        )
    })
}

/// The equilibrium temperature of a surface that takes `starlight`,
/// `(starlight / σ)^¼` (a `Definition:`); a negative light reads as
/// none.
fn equilibrium(starlight: WattPerSquareMetre) -> Kelvin {
    Kelvin((starlight.0.max(0.0) / STEFAN_BOLTZMANN).powf(0.25))
}

/// What condenses of `body`: its inventory air, with the CO₂ its frozen
/// branch holds when it has one ([`ClimateBody::co2`]).
fn inventory(body: &ClimateBody) -> Weights {
    let air = body.inputs.air;
    body.co2.map_or(air, |co2| Weights { co2: co2.0, ..air })
}

/// The air `gases` (the inventory the climate is given: its CO₂ the
/// inventory's, which the thermostat and the collapse start from) with
/// its N₂, CH₄ and Ar condensed at `t` over the other gases of `last`,
/// the air the last climate solved with ([`condense`] of
/// [`condensables`]): the weight that holds a gas at its cap depends on
/// the others (`Weights::weight_at`), and those the climate reads are
/// the solved air's CO₂ and vapour, not the inventory's (#749).
fn condensed_over(gases: &Weights, last: &Weights, t: Kelvin) -> Weights {
    let kept = condense(&condensables(last, gases), t).air;
    Weights {
        n2: kept.n2,
        ch4: kept.ch4,
        ar: kept.ar,
        ..*gases
    }
}

/// The climate of `body` under `starlight` with `air`, its surface
/// radiating with `emissivity`: its air, its temperature and whether the
/// thermostat holds it (legacy `under_albedo`'s `under`):
///
/// - a dry body (no water out of a runaway): the balance of its column
///   ([`surface_temperature`]);
/// - a wet body with plates and no frozen branch's CO₂: the
///   [`thermostat`];
/// - a wet body on its frozen branch: the [`collapsed_climate`] of its
///   CO₂;
/// - any other wet body: the [`wet_climate`].
fn climate_of(
    body: &mut ClimateBody,
    starlight: WattPerSquareMetre,
    air: &Weights,
    emissivity: Fraction,
) -> (Weights, Kelvin, bool) {
    if !body.wet() {
        let i = body.inputs;
        let t = surface_temperature(
            starlight,
            &air.partial_pressures(),
            i.longwave_ozone(),
            i.heat_flux,
            emissivity,
            i.gravity,
            body.numerics.solver,
            i.ceiling(),
            &mut body.columns,
        );
        (*air, t, false)
    } else if body.inputs.plates && body.co2.is_none() {
        thermostat(body, starlight, air, emissivity)
    } else if body.co2.is_some() {
        let c = collapsed_climate(body, starlight, air, emissivity);
        (c.air, c.temperature, false)
    } else {
        let (air, t) = wet_climate(body, starlight, air, emissivity);
        (air, t, false)
    }
}

/// The climate of `body` under its derived Bond `albedo`, before the
/// runaway's blend ([`ClimateBody::runaway_albedo`]), its surface
/// radiating with `emissivity` ([`under_light`] of the light it takes).
pub(crate) fn under_albedo(body: &mut ClimateBody, albedo: Fraction, emissivity: Fraction) -> Pass {
    let absorbing = body.runaway_albedo(albedo);
    let starlight = body.starlight(absorbing);
    under_light(body, starlight, emissivity)
}

/// The climate of `body` under `starlight` (W/m², after its albedo), its
/// surface radiating with `emissivity`, once the nitrogen and methane
/// that condense on its surface ([`condense`]) keep in the air what its
/// temperature allows and the temperature follows from that air
/// ([`climate_of`]).
///
/// From the grey body's temperature (the coldest surface, but for the
/// methane's anti-greenhouse, which a step down undoes), each step solves
/// the climate with the air the last temperature leaves, which rises
/// towards the coldest balance (more air, more greenhouse). Most bodies
/// condense nothing at their grey body's temperature: one solve,
/// converged. Where a step overshoots (it crossed a phase change that
/// thins the air, as a methane sea freezing), the temperature is solved
/// for by [`regula_falsi`] in the bracket the last two steps make, within
/// the solver's `bracket_tolerance`. It stops when the temperature
/// assumed is within the solver's `tolerance` of the one its air gives,
/// when the air stops changing, or after the numerics'
/// `condense_iterations` steps, not converged. A collapse of CO₂ that
/// stops at the numerics' `collapse_iterations` before settling leaves
/// the pass not converged too.
fn under_light(
    body: &mut ClimateBody,
    starlight: WattPerSquareMetre,
    emissivity: Fraction,
) -> Pass {
    let numerics = body.numerics;
    let solver = numerics.solver;
    let gases = inventory(body);
    let absorbed = starlight.0.max(0.0) + body.inputs.heat_flux.0.max(0.0);
    let e = emissivity.get().clamp(LEAST_EMISSIVITY, 1.0);
    let mut t = (absorbed / (e * STEFAN_BOLTZMANN)).powf(0.25);
    let mut air = condense(&gases, Kelvin(t)).air;
    body.collapse_capped = false;
    let mut pass = climate_of(body, starlight, &air, emissivity);
    if air == gases {
        return Pass {
            air: pass.0,
            temperature: pass.1,
            regulated: pass.2,
            starlight,
            converged: !body.collapse_capped,
            iterations: 0,
        };
    }
    let mut converged = false;
    let mut last: Option<Point> = None;
    let mut iterations = 0;
    while iterations < numerics.condense_iterations {
        iterations += 1;
        // How far the temperature assumed is above the one its air gives.
        let here = Point {
            x: t,
            f: t - pass.1.0,
        };
        if here.f.abs() < solver.tolerance.0 {
            converged = true;
            break;
        }
        if let Some(before) = last.filter(|b| (b.f < 0.0) != (here.f < 0.0)) {
            let (low, high) = if here.f < 0.0 {
                (here, before)
            } else {
                (before, here)
            };
            if low.x < high.x {
                // The other gases are the last climate's: the caps' weights
                // depend on them (`Weights::weight_at`).
                let around = pass.0;
                let excess = |x: f64| {
                    let air = condensed_over(&gases, &around, Kelvin(x));
                    x - climate_of(body, starlight, &air, emissivity).1.0
                };
                t = regula_falsi(
                    excess,
                    low,
                    high,
                    solver.bracket_tolerance.0,
                    solver.iterations,
                )
                .x;
                air = condensed_over(&gases, &around, Kelvin(t));
                pass = climate_of(body, starlight, &air, emissivity);
                converged = (pass.1.0 - t).abs() < solver.tolerance.0;
                break;
            }
        }
        last = Some(here);
        t = pass.1.0;
        // Condensed over the last climate's other gases (its CO₂ and
        // vapour), which set the weight each cap keeps.
        let next = condensed_over(&gases, &pass.0, Kelvin(t));
        if next == air {
            converged = true;
            break;
        }
        air = next;
        pass = climate_of(body, starlight, &air, emissivity);
    }
    Pass {
        air: condensed_air(&pass.0, &gases, pass.1),
        temperature: pass.1,
        regulated: pass.2,
        starlight,
        converged: converged && !body.collapse_capped,
        iterations,
    }
}

/// The albedo and emissivity the fixed point of `body` starts from warm:
/// those of the climate under its stars' own light at the timeline's
/// reference albedo (a pass under `σ Σ T_eq,ᵢ⁴`, each source's
/// [`Light::equilibrium`], with a black surface), at least as warm as a
/// body with no ice ([`ICE_FREE_TEMPERATURE`]), so that it settles on the
/// warmest climate. A wet body's air holds the vapour its water keeps at
/// that temperature ([`surface_vapour`]): its clouds follow their
/// humidity, and the vapour of a colder surface in a warmer air would
/// start the fixed point under too thin a cloud cover. Earth's 255 K is
/// where this start begins, not the balance it settles on.
///
/// A departure from legacy (`climate.rs` l.2146-2157), in the starting
/// point only: legacy passed the reference albedo through
/// `under_albedo`, so that a runaway that keeps steam took it blended
/// towards its runaway albedo ([`ClimateBody::runaway_albedo`]); this
/// start takes the stars' light at the reference albedo as it is. The
/// fixed point ([`settle`]) blends every step after it, so the converged
/// climate does not depend on it.
///
/// [`Light::equilibrium`]: super::Light::equilibrium
pub(crate) fn warm_start(body: &mut ClimateBody) -> (Fraction, Fraction) {
    let light: f64 = body
        .inputs
        .sources
        .iter()
        .map(|s| STEFAN_BOLTZMANN * s.equilibrium.0.max(0.0).powi(4))
        .sum();
    let pass = under_light(body, WattPerSquareMetre(light), fraction(1.0));
    let warm = Kelvin(pass.temperature.0.max(ICE_FREE_TEMPERATURE.0));
    let air = if body.wet() {
        let water = body.water_column();
        Weights {
            h2o: surface_vapour(warm, water).0,
            ..pass.air
        }
    } else {
        pass.air
    };
    (
        body.derived_albedo(warm, &air),
        body.emissivity_at(warm, &air),
    )
}

/// Where the albedo's fixed point ([`albedo_fixed_point`]) stopped: the
/// albedo and emissivity of its last climate, that climate, whether it
/// settled, its steps, and whether it has no fixed point and cycles
/// ([`Settle::cycles`]).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Fixed<C> {
    pub albedo: f64,
    pub emissivity: f64,
    pub climate: C,
    pub settled: bool,
    pub steps: usize,
    pub cycles: bool,
}

/// The albedo's fixed point of `map` from the albedo and emissivity
/// `start`, under `numerics` (its `albedo_*` constants): the loop of
/// [`settle`], which reads its climate only through [`AlbedoMap`], so
/// that a constructed map drives it in the solver's tests (#767).
pub(crate) fn albedo_fixed_point<M: AlbedoMap>(
    map: &mut M,
    start: (f64, f64),
    numerics: &Numerics,
) -> Fixed<M::Climate> {
    let tolerance = numerics.albedo_tolerance.0;
    let (mut albedo, mut emissivity) = start;
    let mut last = f64::NAN;
    let mut pass = map.solve(albedo, emissivity);
    let mut settled = false;
    let mut steps = 0;
    let mut damping = numerics.albedo_damping;
    let mut moved = 0.0;
    let mut before: Option<Point> = None;
    let mut bracket = Bracket::default();
    let mut previous: Option<End<M::Climate>> = None;
    let mut cycles = false;
    // Whether the root the push settled on was found stable (kept until
    // the push leaves the tolerance).
    let mut stable = false;
    // The radiative equilibrium of the last root left as unstable.
    let mut rejected: Option<f64> = None;
    // Whether the last step was the bracket's.
    let mut bracketed = false;
    while steps < numerics.albedo_iterations {
        let t = M::temperature(&pass);
        let (derived, derived_emissivity) = map.derived(&pass);
        // Settled when the temperature has stopped moving and the light
        // it derives would not move it by more either (a thermostat
        // holds the temperature whatever the albedo).
        let used = map.radiative(albedo, emissivity);
        let push = map.radiative(derived, derived_emissivity) - used;
        let root = (t - last).abs() < tolerance && push.abs() < tolerance;
        // A push out of the tolerance has left the root found stable: the
        // next root is weighed again.
        if push.abs() >= tolerance {
            stable = false;
        }
        if root && !stable {
            // Is the root stable? On an unstable one, go on from the
            // colder probe. A root the plain step came back to from there
            // attracts from its cold side after all: it is kept.
            let back = !bracketed && rejected.is_some_and(|x: f64| (used - x).abs() < tolerance);
            // A plain step leaves an unstable root by itself: a root it
            // reached with the push falling from its last step is stable
            // without probes (the bracket's steps carry no such slope).
            let plain = !bracketed
                && before.is_some_and(|b| b.x != used && attracting(b, Point { x: used, f: push }));
            stable = M::regulated(&pass) || back || plain || {
                let probe = probe_root(map, albedo, emissivity, push, tolerance);
                if let Some((cold_albedo, cold_pass)) = probe {
                    rejected = Some(used);
                    albedo = cold_albedo;
                    pass = cold_pass;
                    bracket = Bracket::default();
                    (before, previous) = (None, None);
                    (moved, last, damping) = (0.0, f64::NAN, numerics.albedo_damping);
                    bracketed = false;
                    steps += 1;
                    continue;
                }
                true
            };
        }
        if root {
            let (albedo_off, emissivity_off) =
                residuals_of(map, albedo, emissivity, |map| map.derived(&pass));
            if albedo_off.abs() < tolerance && emissivity_off.abs() < tolerance {
                settled = true;
                break;
            }
        }
        let end = End {
            albedo,
            emissivity,
            used,
            push,
            temperature: t,
            climate: pass,
        };
        // The push turned back: the step before and this one bracket the
        // fixed point; from then on every step is an end of the bracket.
        if bracket.closed_in() {
            bracket.take(end);
        } else if before.is_some_and(|b| (b.f >= 0.0) != (push >= 0.0)) {
            if let Some(first) = previous {
                bracket.take(first);
            }
            bracket.take(end);
        }
        previous = Some(end);
        // A bracket whose ends push apart holds a root or a jump the
        // iteration leaves (a bistable body's unstable state): the plain
        // step goes on from here, as it did before #729.
        if bracket.closed_in() && !bracket.attracts() {
            bracket = Bracket::default();
        }
        // A settled push on a pair that is not the derived one: the plain
        // step below moves the pair, not the bracket.
        let next = if root {
            None
        } else {
            bracket.next(numerics.albedo_bracket)
        };
        match next {
            Some(Next::Closed(cold)) => {
                (albedo, emissivity, pass) = (cold.albedo, cold.emissivity, cold.climate);
                cycles = true;
                break;
            }
            Some(Next::Step {
                albedo: a,
                emissivity: e,
            }) => {
                before = Some(Point { x: used, f: push });
                moved = push;
                last = t;
                (albedo, emissivity) = (a, e);
                pass = map.solve(albedo, emissivity);
                steps += 1;
                bracketed = true;
                continue;
            }
            None => {}
        }
        // A push back the way the last step came: the fixed point is
        // between them, and the next steps go half as far.
        if push * moved < 0.0 {
            damping *= 0.5;
        }
        let here = Point { x: used, f: push };
        let share = match before {
            Some(b) if here.x != b.x => {
                let slope = (here.f - b.f) / (here.x - b.x);
                if slope < 0.0 {
                    (-1.0 / slope).clamp(damping, 1.0)
                } else {
                    damping
                }
            }
            _ => damping,
        };
        before = Some(here);
        moved = push;
        last = t;
        albedo += share * (derived - albedo);
        emissivity += share * (derived_emissivity - emissivity);
        pass = map.solve(albedo, emissivity);
        steps += 1;
        bracketed = false;
    }
    Fixed {
        albedo,
        emissivity,
        climate: pass,
        settled,
        steps,
        cycles,
    }
}

/// The fixed point of the albedo, the emissivity and the climate of
/// `body` from the derived albedo and emissivity `start`: each step
/// solves the climate under them ([`under_albedo`]) and moves part of the
/// way to the ones its temperature derives
/// ([`ClimateBody::derived_albedo`], [`ClimateBody::emissivity_at`]): as
/// far as the secant through its last two steps puts the zero of the
/// push (the radiative equilibrium `T_eq ε^-¼` derived less the one
/// used), but at least the numerics' `albedo_damping` of the way and
/// never past the derived light, the damping halving after each step
/// whose push turns back, so that a swing settles (#593: the secant saves
/// about half the steps). Once the push has turned back with each end
/// pushing towards the other ([`Bracket::attracts`]), the fixed point
/// lies between the last albedo that pushes up and the last that pushes
/// down, and each step goes to the zero of the push the Illinois rule
/// (Dowell & Jarratt 1971, as [`regula_falsi`] applies it) puts between
/// them (#729: the secant alone, at the full way, kept a climate cycling
/// between a warm and a collapsed side for all its steps); a bracket
/// whose ends push apart is dropped and the plain step goes on. It goes on until the temperature moves less than
/// the numerics' `albedo_tolerance` and the push is under it too, at most
/// `albedo_iterations` steps, or until the bracket is narrower than the
/// numerics' `albedo_bracket` in both the albedo and the emissivity
/// without settling: the climate jumps there
/// and has no fixed point, so it cycles, the static analogue of the
/// limit cycles in time that Kadoya and Tajika 2014, Menou 2015 and
/// Haqq-Misra et al. 2016 find near the outer edge
/// (`docs/science/planet-atmospheres.md`, #729); the climate is then the
/// colder end's (the one a cooling planet reaches), not converged, and
/// [`Settle::cycles`].
///
/// A root is kept only where it attracts (study 38,
/// `docs/science/nitrogen-phases.md`): where the
/// temperature and the push have settled, [`probe_root`] takes the push
/// a little colder. A regulated climate is stable
/// whatever its albedo (the thermostat holds its temperature), and a root
/// the plain step reached with the push falling from its last step to it
/// is stable without probes (the plain step leaves an unstable root by
/// itself, so only the bracket's steps need them). The
/// bracket's Illinois steps can land on the unstable
/// root of a bistable body's S curve, between its two stable branches
/// (the frost and the liquid of a nitrogen world): that root is
/// rejected, and the fixed point goes on from the colder probe with the
/// plain step and no bracket, so that it reaches the stable branch on
/// the cold side, the side #729 stores a jump on. A root the plain step
/// comes back to from that colder probe (within the tolerance) attracts
/// from its cold side and is kept. The plain step from
/// the warm start leaves an unstable root by itself, so a stable root it
/// reaches first is kept.
///
/// The push alone does not settle the pair: a step moves the albedo and
/// the emissivity together, and the push, one residual, vanishes on a
/// whole line of pairs whose `(1 − A) / ε` is the derived one. So it is
/// settled only once the albedo's own residual (the radiative
/// equilibrium of the derived albedo under the emissivity used, less the
/// one used) and the emissivity's (the derived emissivity under the
/// albedo used) are each under the tolerance too; while the push has
/// settled and they have not, the plain step moves the pair towards the
/// derived one ([`residuals_of`]). The climate keeps the albedo and
/// emissivity it was solved under, so that it reproduces, and they are the
/// derived ones within that tolerance; it is not a snowball (the frozen
/// branch is drawn by the caller). It is converged when this fixed point
/// and the last condensation in it settled. The liquid reads the substellar
/// temperature of the light it took ([`substellar_temperature`]) when the
/// body is tidally locked.
pub(crate) fn settle(body: &mut ClimateBody, start: (Fraction, Fraction)) -> Settle {
    let numerics = body.numerics;
    let fixed = albedo_fixed_point(body, (start.0.get(), start.1.get()), &numerics);
    let (albedo, emissivity, pass) = (fixed.albedo, fixed.emissivity, fixed.climate);
    let i = body.inputs;
    let t = pass.temperature;
    let condensate = condense(&condensables(&pass.air, &inventory(body)), t).condensate;
    let substellar = i.locked.then(|| substellar_temperature(pass.starlight));
    Settle {
        air: pass.air,
        temperature: t,
        equilibrium: equilibrium(pass.starlight),
        bond_albedo: body.runaway_albedo(fraction(albedo)),
        emissivity: fraction(emissivity),
        regulated: pass.regulated,
        converged: fixed.settled && pass.converged,
        condensate,
        liquid: liquid(t, &pass.air, i.water, i.runaway, &condensate, substellar),
        iterations: pass.iterations,
        steps: fixed.steps,
        cycles: fixed.cycles,
    }
}

/// The frozen branch of `body` with `co2` of CO₂ in its air: the fixed
/// point ([`settle`]) started from a body covered in ice, at
/// [`ICE_COVERED_TEMPERATURE`], with that CO₂ in place of its own
/// ([`ClimateBody::co2`], restored after). The maximum greenhouse and the
/// moist plateau found on the inventory air are kept (#593 lever 2).
pub(crate) fn frozen_at(body: &mut ClimateBody, co2: Bar) -> Settle {
    let held = body.co2.replace(co2);
    let air = inventory(body);
    let start = (
        body.derived_albedo(ICE_COVERED_TEMPERATURE, &air),
        body.emissivity_at(ICE_COVERED_TEMPERATURE, &air),
    );
    let settled = settle(body, start);
    body.co2 = held;
    settled
}

/// `value` as a fraction, held within 0 to 1: the albedo and emissivity
/// move between fractions, so only a rounding leaves the range.
fn fraction(value: f64) -> Fraction {
    Fraction::new(value.clamp(0.0, 1.0)).expect("a value held within 0 to 1 is a fraction")
}
