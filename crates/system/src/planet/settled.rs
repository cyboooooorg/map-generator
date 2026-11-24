//! The planet chained from its draw to its climate: [`formed`]'s ground,
//! then runs from a water entry to a climate ([`run`]: the surface pass,
//! the air of [`bare_air`], the climate the solver settles on it,
//! [`climate::branches::solve`], recorded in the cause trace,
//! [`climate::record::record`], and the reducing-air fallback that reads
//! it), the boil loop that judges them ([`verdict`], [`after_boil`]) and
//! the steam world ([`steam_world`]).
//! A separate step, so that `bare` and `bare_with` keep their cost and
//! their tests do not run the climate. Ported from
//! `legacy-final:crates/mapgen/src/planet/mod.rs` (`chain`, l.517-604,
//! and `bare`, l.880-893).
//!
//! Not wired into the generation chain yet (2d-4). Not in it: life, the
//! class, the volcanic SO2 frost (#678) and the non-thermal loss (#658).

use super::bare::{Bare, Formed, Ground, bare_air, formed, rock_of, surface_pass};
use super::climate::ClimateInputs;
use super::climate::Rotation;
use super::climate::branches::solve;
use super::climate::record::{
    Climate, record, record_boils, record_hides_surface, record_titan_like,
};
use super::climate::settle::Settle;
use super::climate::wet::boils;
use super::codes;
use super::draws::{LABELS, draw, snowball_draw};
use super::enveloped::Body;
use super::interior::{InteriorError, steam_interior};
use super::water::{boiled, recondensed, water};
use crate::Trial;
use science::units::Bar;
use timeline::{Code, Gas, Seed, Trace, Traced, Water};

use super::draws::Drawn;

/// A planet chained to its climate.
///
/// Past the steam world line the climate is the kept run's, solved on
/// the body before the steam's loss was read on its envelope: its air
/// holds more vapour
/// than the water `bare` kept (study 26, "The steam loss on the inflated
/// body", a documented departure).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Settled {
    /// The planet and its bare air: the air the climate was solved on,
    /// except past the steam world line, where its water, air and body
    /// come from the pass on the envelope ([`steam_world`]).
    pub bare: Bare,
    /// The climate; `None` for a planet that kept an envelope.
    pub climate: Option<Climate>,
    /// Whether a runaway's steam hides its surface: the last
    /// `planet.climate.hides-surface` node read; `None` for a planet that
    /// kept an envelope.
    pub hides_surface: Option<Traced<bool>>,
}

/// The water entry a kept run came from, which a steam world's pass on
/// its envelope enters again ([`steam_world`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Entry {
    /// The water the light leaves ([`water`]).
    Light,
    /// The steam a boil forced ([`boiled`]), from its verdict: the second
    /// entry reuses it, no second verdict.
    Boiled(Traced<bool>),
}

/// One run of the planet from a water entry to its climate: the surface
/// pass, the air, the climate and its reducing-air fallback (legacy
/// `Ran::bare`).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Run {
    /// The planet and the air its climate was solved on.
    pub bare: Bare,
    /// The climate, recorded.
    pub climate: Climate,
    /// What the climate was solved from.
    pub inputs: ClimateInputs,
    /// The settled state the solver found, which [`verdict`] reads.
    pub settle: Settle,
    /// Whether its air held its CO2 back (the fallback's choice).
    pub hold_back: bool,
}

/// The planet `trial` drawn from `seed` ([`draw`]), chained to its bare
/// air and settled ([`settled_with`]), the snowball draw coming from
/// `seed`'s `planet/snowball` sub-seed.
///
/// # Errors
///
/// The errors of [`draw`] and [`settled_with`].
pub(crate) fn settled(
    trial: &Trial,
    seed: Seed,
    rotation: &Rotation,
    trace: &mut Trace,
) -> Result<Settled, InteriorError> {
    let drawn = draw(seed, trial, trace)?;
    settled_with(trial, &drawn, rotation, seed.derive(LABELS.snowball), trace)
}

/// The planet `trial` of composition `drawn`, chained to its climate
/// under `rotation`, with the snowball draw of `snowball`, and judged by
/// the boil loop (legacy `chain`, l.517-571):
///
/// 1. the ground ([`formed`]); a planet that kept an envelope stops there;
/// 2. a first [`run`] from the water its light leaves ([`water`]), and
///    its [`verdict`];
/// 3. no boil: that run stands, its `boiled` recorded false from the
///    verdict (the imposed leaf of [`water`] is replaced);
/// 4. a boil: a run from the steam forced on the ground's water
///    ([`boiled`], runaway and boiled from the verdict), then
///    [`after_boil`];
/// 5. the steam world line, read on the kept run and, past it, the pass
///    on the steam envelope ([`steam_world`]).
///
/// Each run's nodes stay in the trace, as 2c's rebuilt bodies do.
///
/// # Errors
///
/// The errors of each step.
pub(crate) fn settled_with(
    trial: &Trial,
    drawn: &Drawn,
    rotation: &Rotation,
    snowball: Seed,
    trace: &mut Trace,
) -> Result<Settled, InteriorError> {
    let f = match formed(trial, drawn, trace)? {
        Ground::Enveloped(bare) => {
            return Ok(Settled {
                bare: *bare,
                climate: None,
                hides_surface: None,
            });
        }
        Ground::Formed(f) => f,
    };
    let draw = snowball_draw(snowball);
    let light = water(&f.trial, &f.body.exposed(), f.surface, trace)?;
    let first = run(&f, light, rotation, draw, trace)?;
    let v = verdict(&first, trace)?;
    let (kept, entry) = if *v.value() {
        let steam = boiled(&f.trial, &f.body.exposed(), f.surface, v, trace)?;
        let steam = run(&f, steam, rotation, draw, trace)?;
        (
            after_boil(&f, steam, rotation, draw, trace)?,
            Entry::Boiled(v),
        )
    } else {
        let mut first = first;
        let not_boiled = trace.record(Code::of(codes::WATER_BOILED), false, &[v.node()])?;
        first
            .bare
            .surface
            .as_mut()
            .expect("a run has a surface")
            .water
            .boiled = not_boiled;
        (first, Entry::Light)
    };
    steam_world(&f, kept, entry, trace)
}

/// The steam world of the run `kept`, entered from `entry` on the ground
/// `formed` (legacy `chain`, l.577-604, and study 26, "The steam loss on
/// the inflated body"):
///
/// 1. the steam world line ([`record_hides_surface`]) on the kept water's
///    runaway, the climate's H₂O and its surface temperature; under it
///    the run stands (a recondensed run is no runaway, so never past it);
/// 2. past it, the steam envelope ([`steam_interior`]) on the water
///    handed to the loss, the ground's surface water, over the formed
///    rock;
/// 3. one more pass from the same entry, its loss read on the inflated
///    body ([`water`] or [`boiled`] with the first verdict, then
///    [`surface_pass`] with the envelope's radius and mass for the
///    shoreline, and [`bare_air`] with the kept run's choice of CO2),
///    without a climate: one pass, no fixed point; its `boiled` is the
///    kept run's node;
/// 4. the line read again on the water that pass kept, its air's H₂O
///    and the climate's temperature; past it the envelope of the water
///    kept on the rock that pass left, under it the rock (a runaway
///    with no envelope).
///
/// The climate is not solved again (study 26, "The climate under the
/// envelope").
///
/// # Errors
///
/// The errors of each step.
pub(super) fn steam_world(
    formed: &Formed,
    kept: Run,
    entry: Entry,
    trace: &mut Trace,
) -> Result<Settled, InteriorError> {
    let climate = kept.climate;
    let temperature = climate.global.surface_temperature;
    let surface = kept.bare.surface.as_ref().expect("a run has a surface");
    let kept_boiled = surface.water.boiled;
    let hides = record_hides_surface(
        surface.water.runaway,
        climate.atmosphere.partial.get(&Gas::H2o).copied(),
        temperature,
        trace,
    )?;
    if !*hides.value() {
        return Ok(Settled {
            bare: kept.bare,
            climate: Some(climate),
            hides_surface: Some(hides),
        });
    }
    let inflated = Body::Rocky(steam_interior(
        rock_of(&formed.body),
        formed.surface,
        &formed.trial,
        hides,
        trace,
    )?);
    let exposed = inflated.exposed();
    let again = match entry {
        Entry::Light => water(&formed.trial, &exposed, formed.surface, trace)?,
        Entry::Boiled(verdict) => boiled(&formed.trial, &exposed, formed.surface, verdict, trace)?,
    };
    let ground = surface_pass(formed, again, Some(&exposed), trace)?;
    let mut bare = bare_air(&ground, kept.hold_back, trace)?;
    let surface = bare.surface.as_mut().expect("a ground has a surface");
    surface.water.boiled = kept_boiled;
    let still = record_hides_surface(
        surface.water.runaway,
        surface.atmosphere.partial.get(&Gas::H2o).copied(),
        temperature,
        trace,
    )?;
    if *still.value() {
        let steam = surface.water.inventory;
        bare.body = Body::Rocky(steam_interior(
            rock_of(&bare.body),
            steam,
            &bare.trial,
            still,
            trace,
        )?);
    }
    Ok(Settled {
        bare,
        climate: Some(climate),
        hides_surface: Some(still),
    })
}

/// One run of the planet `formed` from the water entry `water` under
/// `rotation` and the snowball draw `draw`: the surface pass
/// ([`surface_pass`]), the air with a reducing air's CO2 held back
/// ([`bare_air`]) and its climate ([`climate_of`]).
///
/// A reducing air whose climate leaves no Titan-like air (a Triton's thin
/// nitrogen, `titan_like_air`) keeps its CO2: the air is made again
/// without holding it back ([`bare_air`], on the same ground) and the
/// climate settled again (legacy `mod` l.889-893). The first pass's
/// nodes stay in the trace; the `planet.climate.titan-like` node records
/// the decision.
///
/// # Errors
///
/// The errors of each step.
pub(super) fn run(
    formed: &Formed,
    water: Water,
    rotation: &Rotation,
    draw: f64,
    trace: &mut Trace,
) -> Result<Run, InteriorError> {
    let ground = surface_pass(formed, water, None, trace)?;
    let held = bare_air(&ground, true, trace)?;
    let (climate, inputs, settle) = climate_of(&held, rotation, draw, trace)?;
    let surface = held.surface.as_ref().expect("a ground has a surface");
    let keeps_held = !*surface.reducing.value()
        || *record_titan_like(&climate, &settle.air, &surface.reducing, trace)?.value();
    if keeps_held {
        return Ok(Run {
            bare: held,
            climate,
            inputs,
            settle,
            hold_back: true,
        });
    }
    let bare = bare_air(&ground, false, trace)?;
    let (climate, inputs, settle) = climate_of(&bare, rotation, draw, trace)?;
    Ok(Run {
        bare,
        climate,
        inputs,
        settle,
        hold_back: false,
    })
}

/// The run's verdict: [`boils`] of its settled state (its air, Bond
/// albedo and emissivity, read from the solver's [`Settle`], not solved
/// again; a regulated climate is judged too), recorded
/// ([`record_boils`]).
///
/// # Errors
///
/// The errors of the trace.
pub(super) fn verdict(run: &Run, trace: &mut Trace) -> Result<Traced<bool>, InteriorError> {
    let s = &run.settle;
    let verdict = boils(&run.inputs, &s.air, s.bond_albedo, s.emissivity);
    record_boils(&run.climate, &run.bare, verdict, trace)
}

/// What follows a boil (legacy `chain`, l.556-570): the `steam` run
/// stands when it kept no water; otherwise its leftover, condensed again
/// ([`recondensed`]), is run, and kept if that run does not boil (its
/// [`verdict`]), the steam standing if it does.
///
/// # Errors
///
/// The errors of each step.
pub(super) fn after_boil(
    formed: &Formed,
    steam: Run,
    rotation: &Rotation,
    draw: f64,
    trace: &mut Trace,
) -> Result<Run, InteriorError> {
    let left = &steam
        .bare
        .surface
        .as_ref()
        .expect("a run has a surface")
        .water;
    if left.inventory.value().0 <= 0.0 {
        return Ok(steam);
    }
    let condensed = recondensed(left, trace)?;
    let wet = run(formed, condensed, rotation, draw, trace)?;
    if *verdict(&wet, trace)?.value() {
        Ok(steam)
    } else {
        Ok(wet)
    }
}

/// The climate of `bare` under `rotation`: solved ([`solve`]) and recorded
/// ([`record`]), with the inputs it was solved from and the settled state
/// the solver found. `ClimateInputs::of` takes no SO2 frost: the volcanic
/// SO2 frost has no port (#678).
///
/// # Errors
///
/// The errors of the trace.
pub(crate) fn climate_of(
    bare: &Bare,
    rotation: &Rotation,
    draw: f64,
    trace: &mut Trace,
) -> Result<(Climate, ClimateInputs, Settle), InteriorError> {
    let inputs = ClimateInputs::of(bare, rotation, Bar(0.0)).expect("a ground has a surface");
    let solved = solve(&inputs, draw);
    let climate = record(&solved, bare, rotation, draw, trace)?;
    Ok((climate, inputs, solved.settle))
}
