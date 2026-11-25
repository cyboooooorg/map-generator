//! The class of a planet: the first rule of the table that holds, read off
//! the settled planet and never fed back into it. Ported from
//! `legacy-final:crates/mapgen/src/planet/classify.rs` (`classify`,
//! `surface_class`, l.206-259) under the decisions of study 27 (the giants
//! by body kind, no radius rule for the mini-Neptune, the steam line the
//! chain already read); the table is in `docs/science/planet-interiors.md`
//! ("Classes") and `docs/science/planet-atmospheres.md` ("The chain and
//! the surface classes").
//!
//! One code path per value: the class reads `Settled.hides_surface`, the
//! climate's liquid and air, and the water's runaway and inventory; it
//! computes only the basin capacity and the period test.

use super::envelope::MINI_NEPTUNE_MAX_MASS;
use super::enveloped::Body;
use super::heat::ICY_MIN_WATER;
use super::interior::InteriorError;
use super::settled::Settled;
use crate::Trial;
use science::classes::{
    DENSE_AIR_BAR, HOT_GIANT_MAX_PERIOD, ICE_WORLD_MIN_OCEANS, SUPER_EARTH_MIN_RADIUS,
    basin_capacity_oceans,
};
use science::habitable::OCEAN_MASS;
use science::units::{Bar, Day, EarthMass, EarthRadius};
use science::volatiles::methane_triple_pressure;
use timeline::{Code, LiquidKind, NodeId, PlanetClass, TectonicRegime, Trace, Traced};

use super::codes;

/// The kind of body the chain built (legacy read the envelope share and
/// the mass instead; the chain already decides them, study 27).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BodyKind {
    /// A planet without an envelope.
    Rocky,
    /// A core under a hydrogen-helium envelope, below a giant's mass.
    MiniNeptune,
    /// A giant read in the tables.
    Giant,
}

/// What the class reads, as plain values (legacy `classify`'s inputs).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ClassView {
    pub kind: BodyKind,
    /// The bulk's (kept) mass.
    pub mass: EarthMass,
    /// The trial orbit's period.
    pub period: Day,
    /// `None` for a planet that kept an envelope.
    pub surface: Option<SurfaceView>,
}

/// The surface of a [`ClassView`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SurfaceView {
    /// Whether a runaway's steam hides the surface.
    pub hides_surface: bool,
    /// The final climate's liquid.
    pub liquid: Option<LiquidKind>,
    pub runaway: bool,
    /// The final climate's surface pressure.
    pub air: Bar,
    /// The water inventory, in Earth oceans.
    pub oceans: f64,
    /// What the basins hold, in Earth oceans.
    pub capacity: f64,
    /// The kept planet's water share of its mass.
    pub water_fraction: f64,
    pub regime: TectonicRegime,
    pub radius: EarthRadius,
}

/// A value a rule read, to give [`classify`] the nodes of the rules up to
/// the winning one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Read {
    Mass,
    Period,
    Hides,
    Liquid,
    Runaway,
    Air,
    Oceans,
    Basin,
    WaterFraction,
    Regime,
    Radius,
}

/// The class of `view`: the first rule that holds wins. "At least" and
/// "within" are `>=` and `<=`, "over" and "exceed" are `>`.
///
/// 1. a giant: a gas giant (`Giant`) or an ice giant (a `MiniNeptune` of
///    at least [`MINI_NEPTUNE_MAX_MASS`]); hot within
///    [`HOT_GIANT_MAX_PERIOD`]: a hot Jupiter or a hot Neptune;
/// 2. a mini-Neptune: the rest of the `MiniNeptune`s;
/// 3. with a surface: a steam world, a lava world, a runaway greenhouse
///    (runaway, air over [`DENSE_AIR_BAR`]), a hydrocarbon world
///    (hydrocarbons, air over methane's triple point), an ocean world
///    (water, oceans over the basins' capacity);
/// 4. an ice world: [`ICY_MIN_WATER`] of water and ices and
///    [`ICE_WORLD_MIN_OCEANS`] kept;
/// 5. a volcanic world: the heat-pipe regime;
/// 6. a super-Earth from [`SUPER_EARTH_MIN_RADIUS`], rocky below.
#[cfg(test)]
pub(crate) fn class(view: &ClassView) -> PlanetClass {
    decide(view).0
}

/// [`class`] and what its rules read, in order.
fn decide(view: &ClassView) -> (PlanetClass, Vec<Read>) {
    let mut reads = Vec::new();
    let hot = |reads: &mut Vec<Read>| {
        reads.push(Read::Period);
        view.period.0 <= HOT_GIANT_MAX_PERIOD.0
    };
    let class = match view.kind {
        BodyKind::Giant => {
            if hot(&mut reads) {
                PlanetClass::HotJupiter
            } else {
                PlanetClass::GasGiant
            }
        }
        BodyKind::MiniNeptune => {
            reads.push(Read::Mass);
            if view.mass.0 < MINI_NEPTUNE_MAX_MASS {
                PlanetClass::MiniNeptune
            } else if hot(&mut reads) {
                PlanetClass::HotNeptune
            } else {
                PlanetClass::IceGiant
            }
        }
        BodyKind::Rocky => match &view.surface {
            Some(surface) => surface_class(surface, &mut reads),
            // A body without an envelope has a surface: nothing to read.
            None => PlanetClass::Rocky,
        },
    };
    (class, reads)
}

/// The class of a body with a surface (rules 3 to 6 of [`class`]).
fn surface_class(s: &SurfaceView, reads: &mut Vec<Read>) -> PlanetClass {
    reads.push(Read::Hides);
    if s.hides_surface {
        return PlanetClass::SteamWorld;
    }
    reads.push(Read::Liquid);
    if s.liquid == Some(LiquidKind::Lava) {
        return PlanetClass::LavaWorld;
    }
    reads.push(Read::Runaway);
    if s.runaway {
        reads.push(Read::Air);
        if s.air.0 > DENSE_AIR_BAR.0 {
            return PlanetClass::RunawayGreenhouse;
        }
    }
    if s.liquid == Some(LiquidKind::Hydrocarbons) {
        reads.push(Read::Air);
        if s.air.0 > methane_triple_pressure().0 {
            return PlanetClass::HydrocarbonWorld;
        }
    }
    if s.liquid == Some(LiquidKind::Water) {
        reads.extend([Read::Oceans, Read::Basin]);
        if s.oceans > s.capacity {
            return PlanetClass::OceanWorld;
        }
    }
    reads.push(Read::WaterFraction);
    if s.water_fraction >= ICY_MIN_WATER {
        reads.push(Read::Oceans);
        if s.oceans >= ICE_WORLD_MIN_OCEANS {
            return PlanetClass::IceWorld;
        }
    }
    reads.push(Read::Regime);
    if s.regime == TectonicRegime::HeatPipe {
        return PlanetClass::VolcanicWorld;
    }
    reads.push(Read::Radius);
    if s.radius.0 >= SUPER_EARTH_MIN_RADIUS.0 {
        PlanetClass::SuperEarth
    } else {
        PlanetClass::Rocky
    }
}

/// The class of the settled planet `settled` of `trial`, recorded as
/// `planet.class` from the nodes the rules up to the winning one read.
///
/// # Errors
///
/// The errors of the trace.
pub(crate) fn classify(
    trial: &Trial,
    settled: &Settled,
    trace: &mut Trace,
) -> Result<Traced<PlanetClass>, InteriorError> {
    let body = &settled.bare.body;
    let bulk = body.bulk();
    let kind = match body {
        Body::Rocky(_) => BodyKind::Rocky,
        Body::MiniNeptune { .. } => BodyKind::MiniNeptune,
        Body::Giant(_) => BodyKind::Giant,
    };
    let surface = settled
        .climate
        .as_ref()
        .zip(settled.bare.surface.as_ref())
        .map(|(climate, ground)| SurfaceView {
            hides_surface: settled.hides_surface.is_some_and(|hides| *hides.value()),
            liquid: climate.liquid.map(|liquid| *liquid.value()),
            runaway: *ground.water.runaway.value(),
            air: *climate.atmosphere.surface_pressure.value(),
            oceans: ground.water.inventory.value().0 / OCEAN_MASS.0,
            capacity: basin_capacity_oceans(*bulk.radius.value(), *bulk.surface_gravity.value()),
            water_fraction: settled.bare.drawn.water_fraction.value().get(),
            regime: *ground.regime.value(),
            radius: *bulk.radius.value(),
        });
    let view = ClassView {
        kind,
        mass: *bulk.mass.value(),
        period: *trial.orbit.period.value(),
        surface,
    };
    let (class, reads) = decide(&view);
    let mut from: Vec<NodeId> = Vec::new();
    for read in reads {
        let nodes: Vec<NodeId> = match read {
            Read::Mass => vec![bulk.mass.node()],
            Read::Period => vec![trial.orbit.period.node()],
            Read::Hides => settled.hides_surface.iter().map(Traced::node).collect(),
            Read::Liquid => settled.climate.iter().map(liquid_node).collect(),
            Read::Runaway => settled
                .bare
                .surface
                .iter()
                .map(|s| s.water.runaway.node())
                .collect(),
            Read::Air => settled
                .climate
                .iter()
                .map(|c| c.atmosphere.surface_pressure.node())
                .collect(),
            Read::Oceans => settled
                .bare
                .surface
                .iter()
                .map(|s| s.water.inventory.node())
                .collect(),
            Read::Basin => vec![bulk.radius.node(), bulk.surface_gravity.node()],
            Read::WaterFraction => vec![settled.bare.drawn.water_fraction.node()],
            Read::Regime => settled
                .bare
                .surface
                .iter()
                .map(|s| s.regime.node())
                .collect(),
            Read::Radius => vec![bulk.radius.node()],
        };
        for node in nodes {
            if !from.contains(&node) {
                from.push(node);
            }
        }
    }
    Ok(trace.record(Code::of(codes::CLASS), class, &from)?)
}

/// The node of the liquid a climate reads: the liquid's, or the climate's
/// state when there is none (as `life` does).
fn liquid_node(climate: &super::climate::record::Climate) -> NodeId {
    climate
        .liquid
        .map_or(climate.global.state.node(), |liquid| liquid.node())
}
