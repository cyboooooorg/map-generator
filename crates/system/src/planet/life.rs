//! Life: whether it arose where liquid water lasted, the air it made, and
//! the climate solved again on that air. Ported from
//! `legacy-final:crates/mapgen/src/planet/life.rs` (`life`, `oxygenate`,
//! l.39-70) through `science::life` (study 27: life from 1.17 Gyr, oxygen
//! in two steps, O₂ at the CIPM ratio to N₂). Legacy did not solve the
//! climate again on the oxygenated air (its "known simplification"); the
//! rewrite does (plan 2d-4, B8), so that the class and the surface read
//! the climate under the air life left.

use super::bare::Bare;
use super::climate::Rotation;
use super::climate::weights;
use super::codes;
use super::interior::InteriorError;
use super::settled::{Settled, climate_of};
use science::life::{Stage, oxygenate, stage};
use science::units::{Bar, EarthFlux, Fraction, Gyr, Kelvin};
use timeline::{Biosphere, Code, Gas, LiquidKind, NodeId, Provenance, Trace, Traced};

/// A settled planet and its life.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Living {
    /// The settled planet; its bare air oxygenated when the biosphere
    /// made oxygen, its climate the second solve's then.
    pub settled: Settled,
    /// `None` for a planet that kept an envelope.
    pub life: Option<Traced<Biosphere>>,
}

/// The life of `settled`, `age` old, with the life draw `life`
/// (`draws::life_draw` of the trial seed's `planet/life` sub-seed), and
/// the climate solved again under `rotation` and the snowball draw
/// `snowball` (the value the first solve read) when life changed the air:
///
/// 1. a planet that kept an envelope (no climate) has no life node;
/// 2. the draw is recorded (`planet.life.draw`, drawn), and the stage
///    ([`stage`]) of the first climate's liquid, the age and the draw
///    (`planet.life`, from the liquid node, or the climate's state node
///    when there is no liquid, the age and the draw);
/// 3. an oxidising or modern biosphere oxygenates the bare air
///    ([`oxygenate`]): `planet.life.air.o2` (from the N₂ for a modern
///    one, the bare air's O₂ for an oxidising one, and the life node),
///    `planet.life.air.ch4` (none for an oxidising one, from the life
///    node; the methane a modern biosphere keeps,
///    `science::life_gases::modern_methane`, from the life node, the
///    surface gravity and each source's flux and effective temperature)
///    and
///    `planet.life.air.pressure` (the sum of the partials); its climate is
///    solved again on that air ([`climate_of`]) and replaces the first.
///
/// No boil verdict reads the second solve: the first liquid was water,
/// and `hides_surface` stays the first's.
///
/// # Errors
///
/// The errors of the trace and of the second solve.
pub(crate) fn living(
    settled: Settled,
    age: &Traced<Gyr>,
    life: f64,
    rotation: &Rotation,
    snowball: f64,
    trace: &mut Trace,
) -> Result<Living, InteriorError> {
    let Some(climate) = settled.climate.as_ref() else {
        return Ok(Living {
            settled,
            life: None,
        });
    };
    let draw = trace.leaf(
        Code::of(codes::LIFE_DRAW),
        Fraction::new(life).expect("the life draw is a fraction"),
        Provenance::Drawn,
    )?;
    let water = climate
        .liquid
        .is_some_and(|l| *l.value() == LiquidKind::Water);
    let s = stage(water, *age.value(), life);
    let why = climate
        .liquid
        .map_or(climate.global.state.node(), |l| l.node());
    let node = trace.record(
        Code::of(codes::LIFE),
        biosphere(s),
        &[why, age.node(), draw.node()],
    )?;
    if matches!(s, Stage::Lifeless | Stage::Anoxic) {
        return Ok(Living {
            settled,
            life: Some(node),
        });
    }
    let bare = oxygenated(&settled.bare, s, node, trace)?;
    let (second, ..) = climate_of(&bare, rotation, snowball, trace)?;
    Ok(Living {
        settled: Settled {
            bare,
            climate: Some(second),
            hides_surface: settled.hides_surface,
        },
        life: Some(node),
    })
}

/// The biosphere of the stage `s`.
fn biosphere(s: Stage) -> Biosphere {
    match s {
        Stage::Lifeless => Biosphere::Lifeless,
        Stage::Anoxic => Biosphere::Anoxic,
        Stage::Oxidising => Biosphere::Oxidising,
        Stage::Modern => Biosphere::Modern,
    }
}

/// `bare` with its air oxygenated at the stage `s` of the life node
/// `life` ([`oxygenate`]), its new O₂, CH₄ and pressure recorded.
pub(super) fn oxygenated(
    bare: &Bare,
    s: Stage,
    life: Traced<Biosphere>,
    trace: &mut Trace,
) -> Result<Bare, InteriorError> {
    let mut bare = bare.clone();
    let gravity = bare.body.bulk().surface_gravity;
    let sources = bare.trial.sources.iter();
    let stars: Vec<(Kelvin, EarthFlux)> = sources
        .clone()
        .map(|s| (*s.effective_temperature.value(), *s.flux.value()))
        .collect();
    let lights: Vec<NodeId> = sources
        .flat_map(|s| [s.flux.node(), s.effective_temperature.node()])
        .collect();
    let surface = bare.surface.as_mut().expect("a climate has a surface");
    let air = oxygenate(&weights(&surface.atmosphere), s, &stars, *gravity.value());
    let partial = &mut surface.atmosphere.partial;
    let o2_from = if s == Stage::Modern {
        partial[&Gas::N2].node()
    } else {
        partial[&Gas::O2].node()
    };
    let o2 = trace.record(
        Code::of(codes::LIFE_AIR_O2),
        Bar(air.o2),
        &[o2_from, life.node()],
    )?;
    // A modern biosphere's methane reads the stars' light and the gravity
    // (`science::life_gases::modern_methane`); an oxidising air's none.
    let mut ch4_from = vec![life.node()];
    if s == Stage::Modern {
        ch4_from.push(gravity.node());
        ch4_from.extend(lights);
    }
    let ch4 = trace.record(Code::of(codes::LIFE_AIR_CH4), Bar(air.ch4), &ch4_from)?;
    partial.insert(Gas::O2, o2);
    partial.insert(Gas::Ch4, ch4);
    let from: Vec<NodeId> = partial.values().map(Traced::node).collect();
    surface.atmosphere.surface_pressure = trace.record(
        Code::of(codes::LIFE_AIR_PRESSURE),
        Bar(partial.values().map(|p| p.value().0).sum()),
        &from,
    )?;
    Ok(bare)
}
