//! The placement loop: turns the asked archetypes and the family's slots
//! into placed, classified planets.
//!
//! Slots run in order: the asked archetypes first (id order, copies 1 to
//! k), then the family's slots in the order its module gives. Each slot
//! reads its own stream, `seed.derive("trial/<label>")`, and its
//! perihelion longitudes `seed.derive("orbit/<label>")`; the label is the
//! archetype id for its first copy, `<id>/<k>` for copy k ≥ 2, and
//! `slot-<n>` for family slot n. A try proposes a mass and an axis,
//! classifies a circular trial, draws the eccentricity from the class,
//! then checks the star, the Roche floor, the spacing to every placed
//! planet and a hot Jupiter's clearing. A drawn slot that spends its
//! [`SLOT_TRIES`] stays empty; an asked one is refused with `NoRoom`.
//! Asked archetypes go first, so they never meet a drawn neighbour and
//! are never moved. Rejected tries keep their nodes in the trace.
//!
//! Every loop is bounded: [`MAX_PLANETS`] is checked before any
//! allocation, each slot has [`SLOT_TRIES`] tries, a compact mass
//! [`SCATTER_REDRAWS`] redraws, and the final re-check removes one planet
//! per pass.

use std::collections::BTreeMap;

use rand_chacha::ChaCha8Rng;
use rand_core::SeedableRng;
use science::orbit::{self, Pair, TWO_PLANET_FLOOR};
use science::units::{
    AstronomicalUnit, Day, Degree, EarthFlux, EarthMass, Fraction, Gyr, Radian, SolarMass,
};
use timeline::{Code, Orbit, PlanetClass, Provenance, Seed, Trace, Traced};

use crate::archetype::{self, ArchetypePlacement, Band};
use crate::error::SystemError;
use crate::families::laws::{
    COMPANION_AXIS, SMALL_MASS_MAX, SMALL_MASS_MIN, eccentricity, ice_scale, k50_sigma_e,
};
use crate::families::table::SLOT_TRIES;
use crate::families::{Family, FamilyContext, Placed, SlotKind, propose};
use crate::random::{log_uniform, normal, uniform};
use crate::sources::{Light, sources};
use crate::star::Star;
use crate::{ArchetypeId, Trial, TrialOutcome};

/// Cause codes of the placement's nodes.
mod codes {
    pub(crate) const ICE_LINE: &str = "system.draw.ice-line";
    pub(crate) const MASS: &str = "system.draw.mass";
    pub(crate) const SEMI_MAJOR_AXIS: &str = "system.draw.semi-major-axis";
    pub(crate) const PERIOD: &str = "system.draw.period";
    pub(crate) const ECCENTRICITY: &str = "system.draw.eccentricity";
    pub(crate) const PERIHELION_LONGITUDE: &str = "system.draw.perihelion-longitude";
    pub(crate) const CLASS: &str = "system.draw.class";
}

/// Definition: the contract's rank limit, `Designation::planet` refuses
/// ranks from 650 (letters `b` to `zz`): a request reaching it is refused
/// before any slot is built.
pub(crate) const MAX_PLANETS: u64 = 650;

/// Source: the scatter of planet radii around their cluster's radius,
/// lognormal of scale σ_R = 0.31 ± 0.07 (He et al. 2019 Table 5, p16, eq.
/// 8 p4; study S 1.4). A compact mass is scattered by `σ_R / slope`, the
/// slope being the planet model's `dln R / dln M`.
const CLUSTER_RADIUS_SIGMA: f64 = 0.31;

/// Definition: the mass ratio of the two trials that probe the model's
/// mass-radius slope (the plan: one probe at M, one at 1.1 M).
const PROBE_STEP: f64 = 1.1;

/// Calibration: the masses (in Earth masses) where the planet model's
/// envelope regime jumps, 0.1, 5, 10 and 20 M⊕ (mapgen1, 2026-11-21): a
/// slope probe must not straddle one.
const ENVELOPE_REGIME_BOUNDS: [f64; 4] = [0.1, 5.0, 10.0, 20.0];

/// Definition: redraws of a compact mass outside `[0.1, 30)` M⊕ before it
/// falls back to the system mass (always inside), so that a very wide
/// scatter cannot loop.
const SCATTER_REDRAWS: u32 = 64;

/// Source: the Rayleigh scale of mutual inclinations of compact systems,
/// 1.40 (+0.54/-0.39) degrees (He et al. 2019 Table 5 p16; Xie 2016 mean
/// 1.4 degrees; study S 3.1), the `σ_inc` that `orbit::k50` takes.
const MUTUAL_INCLINATION_SIGMA: Degree = Degree(1.4);

/// Source: hot Jupiters stop close to twice the Roche limit (Ford & Rasio
/// 2006, abstract p1; study G 1.2): a hot Jupiter's periapsis stays beyond
/// twice the Roche axis.
const HOT_JUPITER_ROCHE_FACTOR: f64 = 2.0;

/// Definition: a full turn, the perihelion longitude is uniform in
/// `[0°, 360°)`.
const FULL_TURN: Degree = Degree(360.0);

/// What the placement reads of the system: the star, its light, the
/// family's context and the system seed.
pub(crate) struct System<'a> {
    pub star: &'a Star,
    pub light: &'a Light,
    pub ctx: &'a FamilyContext,
    pub seed: Seed,
}

/// Places the `asked` archetypes, then the family's slots, and returns
/// every kept planet sorted by semi-major axis.
///
/// # Errors
///
/// `TooManyPlanets` when the asked counts and the family's slots reach
/// [`MAX_PLANETS`] (checked before any slot is built); `UnknownArchetype`
/// for an id without a placement; `NoRoom` when an asked archetype spends
/// its tries; a classifier error on an asked archetype; trace errors.
pub(crate) fn place(
    system: &System<'_>,
    asked: &BTreeMap<ArchetypeId, u32>,
    trace: &mut Trace,
    classify: &impl Fn(&Trial, &Trace) -> Result<TrialOutcome, SystemError>,
) -> Result<Vec<Placed>, SystemError> {
    let ctx = system.ctx;
    let slots = u64::from(ctx.counts.small) + u64::from(ctx.counts.giants);
    let total = asked.values().map(|&count| u64::from(count)).sum::<u64>() + slots;
    if total >= MAX_PLANETS {
        return Err(SystemError::TooManyPlanets { count: total });
    }
    let state = &system.star.state;
    let ice_line = trace.record(
        Code::of(codes::ICE_LINE),
        ctx.ice_line,
        &[state.mass.node()],
    )?;
    let rules = Rules {
        host: ctx.host,
        age: *system.star.age.value(),
        sigma_e: k50_sigma_e(ctx),
        total,
    };
    let mut placer = Placer {
        system,
        ice_line,
        rules,
        slope: None,
        classify,
    };
    // Below 650 by the check above.
    let mut placed = Vec::with_capacity(total as usize);
    for (id, &count) in asked {
        if count == 0 {
            continue;
        }
        let Some(archetype) = archetype::placement(id) else {
            return Err(SystemError::UnknownArchetype {
                archetype: id.clone(),
            });
        };
        for copy in 1..=count {
            let label = if copy == 1 {
                id.0.as_str().to_owned()
            } else {
                format!("{}/{copy}", id.0.as_str())
            };
            let slot = Slot::Asked(id, archetype);
            match placer.slot(slot, label, &placed, trace)? {
                Some(planet) => placed.push(planet),
                None => {
                    return Err(SystemError::NoRoom {
                        archetype: id.clone(),
                    });
                }
            }
        }
    }
    if ctx.family == Family::Compact && slots > 0 {
        placer.slope = placer.slope(&placed, trace)?;
    }
    for slot in 0..ctx.counts.small + ctx.counts.giants {
        if let Some(planet) =
            placer.slot(Slot::Family(slot), format!("slot-{slot}"), &placed, trace)?
        {
            placed.push(planet);
        }
    }
    recheck(&mut placed, &rules);
    Ok(placed)
}

/// A slot: an asked archetype or a family slot (0-based, family slots
/// only).
#[derive(Clone, Copy)]
enum Slot<'a> {
    Asked(&'a ArchetypeId, &'static ArchetypePlacement),
    Family(u32),
}

/// A try's mass and axis, before classifying.
struct Proposed {
    mass: EarthMass,
    a: AstronomicalUnit,
}

/// The state shared by every slot of one system.
struct Placer<'a, 's, C> {
    system: &'a System<'s>,
    ice_line: Traced<AstronomicalUnit>,
    rules: Rules,
    /// The model's `dln R / dln M` at the compact system mass; `None`
    /// keeps every compact planet at the system mass.
    slope: Option<f64>,
    classify: &'a C,
}

impl<C> Placer<'_, '_, C>
where
    C: Fn(&Trial, &Trace) -> Result<TrialOutcome, SystemError>,
{
    /// Runs the tries of one slot on its streams; `None` when they are
    /// spent.
    fn slot(
        &self,
        slot: Slot<'_>,
        label: String,
        placed: &[Placed],
        trace: &mut Trace,
    ) -> Result<Option<Placed>, SystemError> {
        let seed = self.system.seed.derive(&format!("trial/{label}"));
        let mut rng = ChaCha8Rng::seed_from_u64(seed.get());
        let mut longitudes =
            ChaCha8Rng::seed_from_u64(self.system.seed.derive(&format!("orbit/{label}")).get());
        for _ in 0..SLOT_TRIES {
            if let Some((trial, outcome)) =
                self.attempt(slot, seed, &mut rng, &mut longitudes, placed, trace)?
            {
                let asked = match slot {
                    Slot::Asked(id, _) => Some(id.clone()),
                    Slot::Family(_) => None,
                };
                return Ok(Some(Placed {
                    trial,
                    outcome,
                    label,
                    asked,
                }));
            }
        }
        Ok(None)
    }

    /// One try: `None` when it is rejected (or, for a drawn slot, when the
    /// classifier fails).
    fn attempt(
        &self,
        slot: Slot<'_>,
        seed: Seed,
        rng: &mut ChaCha8Rng,
        longitudes: &mut ChaCha8Rng,
        placed: &[Placed],
        trace: &mut Trace,
    ) -> Result<Option<(Trial, TrialOutcome)>, SystemError> {
        let ctx = self.system.ctx;
        let (proposed, provenance, archetype) = match slot {
            Slot::Asked(id, placement) => (
                self.band(placement, rng),
                Provenance::Input,
                Some((id, placement)),
            ),
            Slot::Family(n) => {
                let proposal = propose(ctx, n, rng, placed);
                // The rocky proposer's no-room fallback puts the planet past
                // the ice line: the try is spent.
                if ctx.family == Family::SolarLike
                    && proposal.kind == SlotKind::Small
                    && proposal.a.0 > ctx.ice_line.0
                {
                    return Ok(None);
                }
                let mass = if ctx.family == Family::Compact {
                    self.compact_mass(rng)
                } else {
                    proposal.mass
                };
                let proposed = Proposed {
                    mass,
                    a: proposal.a,
                };
                (proposed, Provenance::Drawn, None)
            }
        };
        let circular = self.circular(
            seed,
            &proposed,
            provenance,
            archetype.map(|(id, _)| id),
            trace,
        )?;
        let outcome = match (self.classify)(&circular, trace) {
            Ok(outcome) => outcome,
            Err(error) if archetype.is_some() => return Err(error),
            Err(_) => return Ok(None),
        };
        // An eccentricity is asked only when the archetype gives its range.
        let (e, e_provenance) = match archetype.and_then(|(_, placement)| placement.eccentricity) {
            Some((lo, hi)) => (lo + (hi - lo) * uniform(rng), Provenance::Input),
            None => (eccentricity(ctx, outcome.class, rng), Provenance::Drawn),
        };
        let longitude = Degree(FULL_TURN.0 * uniform(longitudes));
        let Some(e) = Fraction::new(e).filter(|e| e.get() < 1.0) else {
            return Ok(None);
        };
        if !self.clear_of_the_star(proposed.a, e, &outcome) {
            return Ok(None);
        }
        let eccentricity = trace.leaf(Code::of(codes::ECCENTRICITY), e, e_provenance)?;
        let perihelion = trace.leaf(
            Code::of(codes::PERIHELION_LONGITUDE),
            longitude,
            Provenance::Drawn,
        )?;
        let orbit = Orbit {
            eccentricity,
            perihelion_longitude: perihelion,
            ..circular.orbit
        };
        let sources = sources(
            self.system.star,
            self.system.light,
            &orbit.semi_major_axis,
            &orbit.eccentricity,
            trace,
        )?;
        let trial = Trial {
            orbit,
            sources,
            ..circular
        };
        let asked = archetype.is_some();
        if !self.spaced(&trial, outcome.class, asked, placed)
            || !self.cleared(&trial, &outcome, placed)
        {
            return Ok(None);
        }
        let flux = trial
            .sources
            .iter()
            .map(|source| source.flux.node())
            .collect::<Vec<_>>();
        let mut causes = vec![trial.mass.node(), trial.orbit.semi_major_axis.node()];
        causes.extend(flux);
        trace.record(Code::of(codes::CLASS), outcome.class, &causes)?;
        Ok(Some((trial, outcome)))
    }

    /// An asked archetype's band and mass: the flux (or period)
    /// log-uniform in its band, then the mass log-uniform in its range.
    fn band(&self, placement: &ArchetypePlacement, rng: &mut ChaCha8Rng) -> Proposed {
        let state = &self.system.star.state;
        let host = *state.mass.value();
        let a = match placement.band {
            Band::Insolation(lo, hi) => {
                let flux = EarthFlux(log_uniform(rng, lo.0, hi.0));
                AstronomicalUnit((state.luminosity.value().0 / flux.0).sqrt())
            }
            Band::Period(lo, hi) => orbit::semi_major_axis(Day(log_uniform(rng, lo.0, hi.0)), host),
        };
        let (lo, hi) = placement.mass;
        Proposed {
            mass: EarthMass(log_uniform(rng, lo.0, hi.0)),
            a,
        }
    }

    /// A compact planet's mass: the system mass scattered by
    /// `exp(N(0, 1) σ_R / slope)`, drawn again outside `[0.1, 30)` M⊕ up
    /// to [`SCATTER_REDRAWS`] times; the system mass without a slope or
    /// once the redraws are spent.
    fn compact_mass(&self, rng: &mut ChaCha8Rng) -> EarthMass {
        let mass = self.system.ctx.system_mass;
        let Some(slope) = self.slope else {
            return mass;
        };
        for _ in 0..SCATTER_REDRAWS {
            let scattered = mass.0 * (normal(rng) * CLUSTER_RADIUS_SIGMA / slope).exp();
            if (SMALL_MASS_MIN.0..SMALL_MASS_MAX.0).contains(&scattered) {
                return EarthMass(scattered);
            }
        }
        mass
    }

    /// The model's `dln R / dln M` at the compact system mass: two trials
    /// at the first slot's proposal (on a copy of its stream, so its own
    /// first try proposes the same axis), at M and 1.1 M, or at M and
    /// M / 1.1 when 1.1 M crosses an envelope bound. `None` (no scatter)
    /// when both sides cross one, when a trial fails to classify, or when
    /// the slope is not positive.
    fn slope(&self, placed: &[Placed], trace: &mut Trace) -> Result<Option<f64>, SystemError> {
        let mass = self.system.ctx.system_mass;
        let Some(other) = probe_mass(mass) else {
            return Ok(None);
        };
        let seed = self.system.seed.derive("trial/slot-0");
        let mut rng = ChaCha8Rng::seed_from_u64(seed.get());
        let a = propose(self.system.ctx, 0, &mut rng, placed).a;
        let radius = |mass: EarthMass, trace: &mut Trace| {
            let proposed = Proposed { mass, a };
            let trial = self.circular(seed, &proposed, Provenance::Drawn, None, trace)?;
            Ok::<_, SystemError>((self.classify)(&trial, trace).ok().map(|o| o.radius.0))
        };
        let (Some(at), Some(beside)) = (radius(mass, trace)?, radius(other, trace)?) else {
            return Ok(None);
        };
        let slope = (beside / at).ln() / (other.0 / mass.0).ln();
        Ok((slope > 0.0 && slope.is_finite()).then_some(slope))
    }

    /// The trial of a try on a circular orbit: its mass, axis and period,
    /// eccentricity and perihelion longitude imposed at zero, and the
    /// sources at that orbit.
    fn circular(
        &self,
        seed: Seed,
        proposed: &Proposed,
        provenance: Provenance,
        archetype: Option<&ArchetypeId>,
        trace: &mut Trace,
    ) -> Result<Trial, SystemError> {
        let star = self.system.star;
        let host = &star.state.mass;
        let mass = trace.leaf(Code::of(codes::MASS), proposed.mass, provenance)?;
        let a = trace.leaf(Code::of(codes::SEMI_MAJOR_AXIS), proposed.a, provenance)?;
        let period = trace.record(
            Code::of(codes::PERIOD),
            orbit::period(proposed.a, *host.value()),
            &[a.node(), host.node()],
        )?;
        let zero = Fraction::new(0.0).expect("0 is a fraction");
        let eccentricity = trace.leaf(Code::of(codes::ECCENTRICITY), zero, Provenance::Imposed)?;
        let perihelion_longitude = trace.leaf(
            Code::of(codes::PERIHELION_LONGITUDE),
            Degree(0.0),
            Provenance::Imposed,
        )?;
        let sources = sources(star, self.system.light, &a, &eccentricity, trace)?;
        Ok(Trial {
            seed,
            mass,
            orbit: Orbit {
                semi_major_axis: a,
                eccentricity,
                period,
                perihelion_longitude,
            },
            sources,
            age: star.age,
            metallicity: star.state.metallicity,
            ice_line: self.ice_line,
            archetype: archetype.cloned(),
        })
    }

    /// The periapsis stays outside the star and the Roche floor of the
    /// classified density (twice that floor for a hot Jupiter). A NaN
    /// fails every check.
    fn clear_of_the_star(&self, a: AstronomicalUnit, e: Fraction, outcome: &TrialOutcome) -> bool {
        let ctx = self.system.ctx;
        let periapsis = a.0 * (1.0 - e.get());
        let roche = orbit::semi_major_axis(orbit::roche_period(outcome.mean_density), ctx.host);
        let factor = if outcome.class == PlanetClass::HotJupiter {
            HOT_JUPITER_ROCHE_FACTOR
        } else {
            1.0
        };
        periapsis >= ctx.stellar_radius.0 && periapsis >= factor * roche.0
    }

    /// The spacing to every placed planet, by [`Rules`].
    fn spaced(&self, trial: &Trial, class: PlanetClass, asked: bool, placed: &[Placed]) -> bool {
        let k50 = self
            .rules
            .k50(placed.iter().map(|p| &p.trial).chain([trial]));
        placed.iter().all(|planet| {
            let faces = faces_k50(
                (class, asked),
                (planet.outcome.class, planet.asked.is_some()),
            );
            self.rules.holds(&planet.trial, trial, faces, k50)
        })
    }

    /// No planet inside the hot Jupiters' cleared zone, `1 au × ice line /
    /// 2.7 au`: none of Huang 2016's hot Jupiters has a companion inside
    /// the outer companion's band (study G 1.4, row 3).
    fn cleared(&self, trial: &Trial, outcome: &TrialOutcome, placed: &[Placed]) -> bool {
        let zone = COMPANION_AXIS.0.0 * ice_scale(self.system.ctx.ice_line);
        let hot = outcome.class == PlanetClass::HotJupiter;
        placed.iter().all(|planet| {
            let inside = |t: &Trial| t.orbit.semi_major_axis.value().0 < zone;
            !(planet.outcome.class == PlanetClass::HotJupiter && inside(trial)
                || hot && inside(&planet.trial))
        })
    }
}

/// The second mass of the slope probe: 1.1 M, or M / 1.1 when an
/// envelope bound lies in `(M, 1.1 M]`; `None` when one lies on each
/// side.
pub(super) fn probe_mass(mass: EarthMass) -> Option<EarthMass> {
    let crosses = |lo: f64, hi: f64| ENVELOPE_REGIME_BOUNDS.iter().any(|&b| lo < b && b <= hi);
    let (up, down) = (mass.0 * PROBE_STEP, mass.0 / PROBE_STEP);
    if !crosses(mass.0, up) {
        Some(EarthMass(up))
    } else if !crosses(down, mass.0) {
        Some(EarthMass(down))
    } else {
        None
    }
}

/// Whether a pair faces K50 (when the system counts 3 or more planets):
/// both planets small (neither a gas giant, a hot Jupiter nor an ice
/// giant) and at least one drawn. Each side is its class and whether it
/// was asked. A pair with a giant keeps the two-planet floor only (user
/// decision, 2026-11-22): Pu & Wu's fit covers planets of 3 to 9 M⊕, and
/// adjacent giants follow their own sourced axis ratio.
fn faces_k50(one: (PlanetClass, bool), other: (PlanetClass, bool)) -> bool {
    let giant = |class: PlanetClass| {
        matches!(
            class,
            PlanetClass::GasGiant | PlanetClass::HotJupiter | PlanetClass::IceGiant
        )
    };
    !(one.1 && other.1) && !giant(one.0) && !giant(other.0)
}

/// The spacing rules of a system: every pair keeps `2√3` mutual Hill
/// radii; a pair of small planets holding a drawn one keeps K50 as well
/// when the system counts 3 or more planets ([`faces_k50`]).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Rules {
    pub host: SolarMass,
    pub age: Gyr,
    /// The family's σ_e for K50.
    pub sigma_e: f64,
    /// Asked counts plus family slots.
    pub total: u64,
}

impl Rules {
    /// Pu & Wu's median-survival spacing at the star's age, with `T1` the
    /// shortest period of `trials`, the family's σ_e and the compact
    /// systems' inclination scale.
    fn k50<'t>(&self, trials: impl Iterator<Item = &'t Trial>) -> f64 {
        let innermost = trials
            .map(|t| t.orbit.period.value().0)
            .fold(f64::INFINITY, f64::min);
        orbit::k50(
            self.age,
            Day(innermost),
            self.sigma_e,
            Radian::from(MUTUAL_INCLINATION_SIGMA),
        )
    }

    /// Whether a pair keeps `2√3` and, when it `faces` K50 ([`faces_k50`])
    /// in a system of 3 or more, `k50`. A NaN spacing fails.
    fn holds(&self, one: &Trial, other: &Trial, faces: bool, k50: f64) -> bool {
        let pair = |t: &Trial| Pair {
            mass: *t.mass.value(),
            a: *t.orbit.semi_major_axis.value(),
            e: *t.orbit.eccentricity.value(),
        };
        let (one, other) = (pair(one), pair(other));
        let (inner, outer) = if one.a.0 <= other.a.0 {
            (one, other)
        } else {
            (other, one)
        };
        let delta = orbit::mutual_hill_spacing(inner, outer, self.host);
        let k50_applies = faces && self.total >= 3;
        delta >= TWO_PLANET_FLOOR && (!k50_applies || delta >= k50)
    }
}

/// Sorts `placed` by axis and re-checks adjacent pairs with the final
/// `T1`, dropping the drawn planet of the outermost violating pair (its
/// outer one when drawn), one per pass, until none violates. Asked
/// planets are never dropped; two asked neighbours face only `2√3`,
/// already met when placed. Each pass removes a planet or ends.
pub(crate) fn recheck(placed: &mut Vec<Placed>, rules: &Rules) {
    placed.sort_by(|a, b| {
        let axis = |p: &Placed| p.trial.orbit.semi_major_axis.value().0;
        axis(a).total_cmp(&axis(b))
    });
    for _ in 0..=placed.len() {
        let k50 = rules.k50(placed.iter().map(|p| &p.trial));
        let victim = (1..placed.len()).rev().find_map(|outer| {
            let (inner, outer_planet) = (&placed[outer - 1], &placed[outer]);
            let faces = faces_k50(
                (inner.outcome.class, inner.asked.is_some()),
                (outer_planet.outcome.class, outer_planet.asked.is_some()),
            );
            if rules.holds(&inner.trial, &outer_planet.trial, faces, k50) {
                None
            } else if outer_planet.asked.is_none() {
                Some(outer)
            } else if inner.asked.is_none() {
                Some(outer - 1)
            } else {
                None
            }
        });
        match victim {
            Some(index) => {
                placed.remove(index);
            }
            None => return,
        }
    }
}
