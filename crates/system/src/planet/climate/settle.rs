//! The fixed points that settle a body's climate: the condensation of its
//! air at the temperature that air gives ([`under_albedo`]), and the
//! albedo and emissivity at the temperature they give ([`settle`]), from
//! a warm start ([`warm_start`]) or an ice-covered one ([`frozen_at`]).
//! Ported from `legacy-final:crates/mapgen/src/planet/climate.rs`
//! (`Body::frozen_at`, l.1749-1763; `Pass` and `under_albedo`,
//! l.1950-2080; `warm_start`, l.2146-2157; `settle` and `settle_steps`,
//! l.2159-2272, with the secant step of #593).

use super::body::ClimateBody;
use super::carbon::thermostat;
use super::wet::{collapsed_climate, wet_climate};
use science::balance::{LEAST_EMISSIVITY, Point, regula_falsi, surface_temperature};
use science::constants::STEFAN_BOLTZMANN;
use science::radiation::gases::PartialPressures;
use science::surface::{
    ICE_COVERED_TEMPERATURE, ICE_FREE_TEMPERATURE, Liquid, condense, condensed_air, liquid,
    substellar_temperature, surface_vapour,
};
use science::units::{Bar, Fraction, Kelvin, WattPerSquareMetre};

/// One solve of [`under_albedo`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Pass {
    /// The air the climate leaves.
    pub air: PartialPressures,
    /// The surface temperature.
    pub temperature: Kelvin,
    /// Whether the carbonate thermostat holds it.
    pub regulated: bool,
    /// The light its surface took, after the runaway's blend of its
    /// albedo.
    pub starlight: WattPerSquareMetre,
    /// Whether the condensation's fixed point settled within the
    /// solver's tolerance.
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
    pub air: PartialPressures,
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
    pub condensate: PartialPressures,
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
/// emissivity a climate was solved under, the push it derives, and that
/// climate.
#[derive(Clone, Copy, Debug)]
struct End {
    albedo: f64,
    emissivity: f64,
    push: f64,
    pass: Pass,
}

/// The bracket of the albedo's fixed point once its push has turned back:
/// the last end that pushes up (`up`, a push of zero included) and the
/// last that pushes down, each with the push the Illinois rule weighs it
/// by (halved each time the other end is replaced twice in a row).
#[derive(Clone, Copy, Debug, Default)]
struct Bracket {
    up: Option<(End, f64)>,
    down: Option<(End, f64)>,
    last_up: Option<bool>,
}

impl Bracket {
    /// Takes `end` as the new end on its side.
    fn take(&mut self, end: End) {
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

    /// The two ends with their weights, once both sides are known.
    fn ends(&self) -> Option<((End, f64), (End, f64))> {
        self.up.zip(self.down)
    }
}

/// The equilibrium temperature of a surface that takes `starlight`,
/// `(starlight / σ)^¼` (a `Definition:`); a negative light reads as
/// none.
fn equilibrium(starlight: WattPerSquareMetre) -> Kelvin {
    Kelvin((starlight.0.max(0.0) / STEFAN_BOLTZMANN).powf(0.25))
}

/// What condenses of `body`: its inventory air, with the CO₂ its frozen
/// branch holds when it has one ([`ClimateBody::co2`]).
fn inventory(body: &ClimateBody) -> PartialPressures {
    let air = body.inputs.air;
    body.co2
        .map_or(air, |co2| PartialPressures { co2: co2.0, ..air })
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
    air: &PartialPressures,
    emissivity: Fraction,
) -> (PartialPressures, Kelvin, bool) {
    if !body.wet() {
        let i = body.inputs;
        let t = surface_temperature(
            starlight,
            air,
            i.heat_flux,
            emissivity,
            i.gravity,
            body.numerics.solver,
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
/// `condense_iterations` steps, not converged.
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
    let mut pass = climate_of(body, starlight, &air, emissivity);
    if air == gases {
        return Pass {
            air: pass.0,
            temperature: pass.1,
            regulated: pass.2,
            starlight,
            converged: true,
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
                let excess = |x: f64| {
                    let air = condense(&gases, Kelvin(x)).air;
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
                air = condense(&gases, Kelvin(t)).air;
                pass = climate_of(body, starlight, &air, emissivity);
                converged = (pass.1.0 - t).abs() < solver.tolerance.0;
                break;
            }
        }
        last = Some(here);
        t = pass.1.0;
        let next = condense(&gases, Kelvin(t)).air;
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
        converged,
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
        PartialPressures {
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
/// about half the steps). Once the push has turned back, the fixed point
/// lies between the last albedo that pushes up and the last that pushes
/// down, and each step goes to the zero of the push the Illinois rule of
/// [`regula_falsi`] puts between them (#729: the secant alone, at the
/// full way, kept a climate cycling between a warm and a collapsed side
/// for all its steps). It goes on until the temperature moves less than
/// the numerics' `albedo_tolerance` and the push is under it too, at most
/// `albedo_iterations` steps, or until the bracket is narrower than the
/// numerics' `albedo_bracket` without settling: the climate jumps there
/// and has no fixed point, so it cycles (Menou 2015, Haqq-Misra et al.
/// 2016 find such cycles near the outer edge); the climate is then the
/// colder end's (the one a cooling planet reaches), not converged, and
/// [`Settle::cycles`].
///
/// The climate keeps the albedo and emissivity it was solved under, so
/// that it reproduces; it is not a snowball (the frozen branch is drawn
/// by the caller). It is converged when this fixed point and the last
/// condensation in it settled. The liquid reads the substellar
/// temperature of the light it took ([`substellar_temperature`]) when the
/// body is tidally locked.
pub(crate) fn settle(body: &mut ClimateBody, start: (Fraction, Fraction)) -> Settle {
    let numerics = body.numerics;
    let tolerance = numerics.albedo_tolerance.0;
    let radiative = |body: &mut ClimateBody, albedo: f64, emissivity: f64| {
        let light = body.starlight(fraction(albedo));
        equilibrium(light).0 / emissivity.clamp(LEAST_EMISSIVITY, 1.0).powf(0.25)
    };
    let (mut albedo, mut emissivity) = (start.0.get(), start.1.get());
    let mut last = f64::NAN;
    let mut pass = under_albedo(body, fraction(albedo), fraction(emissivity));
    let mut settled = false;
    let mut steps = 0;
    let mut damping = numerics.albedo_damping;
    let mut moved = 0.0;
    let mut before: Option<Point> = None;
    let mut bracket = Bracket::default();
    let mut previous: Option<End> = None;
    let mut cycles = false;
    while steps < numerics.albedo_iterations {
        let t = pass.temperature;
        let derived = body.derived_albedo(t, &pass.air).get();
        let derived_emissivity = body.emissivity_at(t, &pass.air).get();
        // Settled when the temperature has stopped moving and the light
        // it derives would not move it by more either (a thermostat
        // holds the temperature whatever the albedo).
        let used = radiative(body, albedo, emissivity);
        let push = radiative(body, derived, derived_emissivity) - used;
        if (t.0 - last).abs() < tolerance && push.abs() < tolerance {
            settled = true;
            break;
        }
        let end = End {
            albedo,
            emissivity,
            push,
            pass,
        };
        // The push turned back: the step before and this one bracket the
        // fixed point; from then on every step is an end of the bracket.
        if bracket.ends().is_some() {
            bracket.take(end);
        } else if before.is_some_and(|b| (b.f >= 0.0) != (push >= 0.0)) {
            if let Some(first) = previous {
                bracket.take(first);
            }
            bracket.take(end);
        }
        previous = Some(end);
        if let Some(((up, w_up), (down, w_down))) = bracket.ends() {
            if (up.albedo - down.albedo).abs() < numerics.albedo_bracket {
                let cold = if down.pass.temperature.0 < up.pass.temperature.0 {
                    down
                } else {
                    up
                };
                (albedo, emissivity, pass) = (cold.albedo, cold.emissivity, cold.pass);
                cycles = true;
                break;
            }
            // The zero of the push on the line between the two ends.
            let share = w_up / (w_up - w_down);
            last = t.0;
            albedo = up.albedo + share * (down.albedo - up.albedo);
            emissivity = up.emissivity + share * (down.emissivity - up.emissivity);
            pass = under_albedo(body, fraction(albedo), fraction(emissivity));
            steps += 1;
            continue;
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
        last = t.0;
        albedo += share * (derived - albedo);
        emissivity += share * (derived_emissivity - emissivity);
        pass = under_albedo(body, fraction(albedo), fraction(emissivity));
        steps += 1;
    }
    let i = body.inputs;
    let t = pass.temperature;
    let condensate = condense(&inventory(body), t).condensate;
    let substellar = i.locked.then(|| substellar_temperature(pass.starlight));
    Settle {
        air: pass.air,
        temperature: t,
        equilibrium: equilibrium(pass.starlight),
        bond_albedo: body.runaway_albedo(fraction(albedo)),
        emissivity: fraction(emissivity),
        regulated: pass.regulated,
        converged: settled && pass.converged,
        condensate,
        liquid: liquid(t, &pass.air, i.water, i.runaway, &condensate, substellar),
        iterations: pass.iterations,
        steps,
        cycles,
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
