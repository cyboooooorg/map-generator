//! The planet chain behind [`super::class_of`] and [`super::model`]: the
//! composition drawn from the trial seed, the ground, the spin, the
//! climate and its boil loop, the steam world, life and the class, then,
//! for `model`, the [`PlanetState`] built from what the chain recorded.
//!
//! Every draw derives from the trial seed (`trial.seed`), never from the
//! trace: the composition ([`draw`]), the snowball and life draws, drawn
//! once here as values and passed down, and the spin `class_of` draws.
//! So the same trial gives the same answer on both paths.

use super::bare::{Ground, formed};
use super::classify::classify;
use super::climate::Rotation;
use super::climate::record::{Climate, sea_share};
use super::codes;
use super::draws::{Drawn, LABELS, draw, life_draw, snowball_draw};
use super::interior::InteriorError;
use super::life::{Living, living};
use super::settled::{enveloped, settled_on};
use crate::archetype;
use crate::spin;
use crate::{Placement, SystemError, Trial};
use science::outgassing::column_mass;
use science::units::{Degree, Fraction, Kilogram, Metre};
use timeline::{
    Bulk, Code, Composition, Gas, InteriorProfile, Layer, LiquidKind, PlanetClass, PlanetState,
    Provenance, RadialSamples, Spin, Surface, SurfaceLiquid, Terrain, Trace, Traced,
};

/// Where the chain's obliquity comes from (B3).
#[derive(Clone, Copy, Debug)]
pub(super) enum Spun<'a> {
    /// `class_of`: the spin is drawn from the trial seed ([`spin::spin`]),
    /// as the draw draws the placement's.
    Trial,
    /// `model`: the placement's spin, whose obliquity node the climate
    /// reads itself.
    Placed(&'a Spin),
}

/// What the chain gives.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Chained {
    /// The settled planet and its life.
    pub living: Living,
    /// The class, recorded.
    pub class: Traced<PlanetClass>,
    /// The rotation the climate read; `None` without a surface.
    pub rotation: Option<Rotation>,
}

/// The chain of `trial`, its composition drawn from the trial seed and its
/// snowball and life draws from the seed's `planet/snowball` and
/// `planet/life` sub-seeds ([`chain_with`]).
///
/// # Errors
///
/// The errors of each step: the planet's as [`SystemError::Planet`], the
/// spin's as they are.
pub(super) fn chain(
    trial: &Trial,
    spun: Spun<'_>,
    trace: &mut Trace,
) -> Result<Chained, SystemError> {
    let drawn = draw(trial.seed, trial, trace)?;
    let snowball = snowball_draw(trial.seed.derive(LABELS.snowball));
    let life = life_draw(trial.seed.derive(LABELS.life));
    chain_with(trial, &drawn, spun, snowball, life, trace)
}

/// The chain of `trial` of composition `drawn`, with the snowball draw
/// `snowball` and the life draw `life` (values):
///
/// 1. the ground ([`formed`]); a planet that kept an envelope has no
///    rotation, and is classified as it is;
/// 2. the rotation: the obliquity drawn from the trial seed with the
///    rocky law (`Spun::Trial`: every class a body with a surface can get
///    takes it; the formed body's radius, the trial's archetype, whose
///    asked obliquity is an input) or the placement's node
///    (`Spun::Placed`), and `locked` an imposed `false` (#604);
/// 3. the climate and its boil loop ([`settled_on`]), life and the
///    second solve ([`living`]), the class ([`classify`]).
///
/// # Errors
///
/// As [`chain`].
pub(super) fn chain_with(
    trial: &Trial,
    drawn: &Drawn,
    spun: Spun<'_>,
    snowball: f64,
    life: f64,
    trace: &mut Trace,
) -> Result<Chained, SystemError> {
    let f = match formed(trial, drawn, trace)? {
        Ground::Enveloped(bare) => {
            let settled = enveloped(*bare);
            let class = classify(trial, &settled, trace)?;
            return Ok(Chained {
                living: Living {
                    settled,
                    life: None,
                },
                class,
                rotation: None,
            });
        }
        Ground::Formed(f) => f,
    };
    let obliquity = match spun {
        Spun::Trial => {
            spin::spin(
                PlanetClass::Rocky,
                &trial.mass,
                *f.body.bulk().radius.value(),
                trial.archetype.as_ref().and_then(archetype::placement),
                trial.seed,
                trace,
            )?
            .obliquity
        }
        Spun::Placed(spin) => spin.obliquity,
    };
    let locked = trace.leaf(Code::of(codes::SPIN_LOCKED), false, Provenance::Imposed)?;
    let rotation = Rotation { obliquity, locked };
    let settled = settled_on(*f, &rotation, snowball, trace)?;
    let living = living(settled, &trial.age, life, &rotation, snowball, trace)?;
    let class = classify(trial, &living.settled, trace)?;
    debug_assert!(
        !spin::is_giant(*class.value()),
        "a planet with a surface takes the rocky spin law: {:?}",
        class.value()
    );
    Ok(Chained {
        living,
        class,
        rotation: Some(rotation),
    })
}

/// The [`PlanetState`] of `placement` from the chain `chained` run on its
/// trial (B5, B6, B13): the kept planet's composition, bulk and heat, the
/// trial's sources, age and orbit, the placement's spin, the body's
/// layers, and the surface of the climate life left.
///
/// # Errors
///
/// The errors of the trace.
pub(super) fn state_of(
    placement: &Placement,
    chained: Chained,
    trace: &mut Trace,
) -> Result<PlanetState, InteriorError> {
    let Chained { living, class, .. } = chained;
    let bare = &living.settled.bare;
    let trial = &placement.trial;
    let bulk: Bulk = bare.body.bulk().clone();
    let layers = bare
        .body
        .layers()
        .into_iter()
        .map(|shell| Layer {
            outer_radius: shell.outer_radius,
            mass: shell.mass,
            material: shell.material,
            state: shell.state,
            transport: None,
        })
        .collect();
    let samples = trace.leaf(
        Code::of(codes::INTERIOR_SAMPLES),
        RadialSamples(Vec::new()),
        Provenance::Imposed,
    )?;
    let surface = match (living.settled.climate.as_ref(), bare.surface.as_ref()) {
        (Some(climate), Some(ground)) => Some(Surface {
            regime: ground.regime,
            core: ground.core.clone(),
            water: ground.water.clone(),
            atmosphere: Some(climate.atmosphere.clone()),
            climate: climate.global.clone(),
            liquid: liquid(climate, &ground.water, &bulk, trace)?,
            terrain: Terrain { units: Vec::new() },
            life: living.life.expect("a climate has a life node"),
        }),
        _ => None,
    };
    Ok(PlanetState {
        sources: trial.sources.clone(),
        age: trial.age,
        orbit: trial.orbit.clone(),
        spin: placement.spin.clone(),
        bulk,
        composition: Composition {
            core_mass_fraction: bare.drawn.core_mass_fraction,
            water_fraction: bare.drawn.water_fraction,
        },
        class,
        heat: bare.heat.clone(),
        interior: InteriorProfile { layers, samples },
        envelope_fraction: bare.drawn.envelope_fraction,
        surface,
        targets: Vec::new(),
    })
}

/// The liquid at the surface of `climate` (B6): water's inventory is the
/// water's node and its share under water [`sea_share`]'s; a condensate's
/// inventory is the column mass of its partial on the ground
/// ([`column_mass`]), its share an imposed 0, as lava's inventory and
/// share (no sea level reads them yet).
fn liquid(
    climate: &Climate,
    water: &timeline::Water,
    bulk: &Bulk,
    trace: &mut Trace,
) -> Result<Option<SurfaceLiquid>, InteriorError> {
    let Some(kind) = climate.liquid else {
        return Ok(None);
    };
    let zero = Fraction::new(0.0).expect("0 is a fraction");
    let imposed_share = |trace: &mut Trace| {
        trace.leaf(
            Code::of(codes::SURFACE_OCEAN_SHARE),
            zero,
            Provenance::Imposed,
        )
    };
    let condensate = match kind.value() {
        LiquidKind::Water => {
            let share = sea_share(water, Some(&kind), trace)?.expect("the liquid is water");
            return Ok(Some(SurfaceLiquid {
                kind,
                inventory: water.inventory,
                ocean_share: share,
            }));
        }
        LiquidKind::Hydrocarbons => climate.condensate.get(&Gas::Ch4),
        LiquidKind::Nitrogen => climate.condensate.get(&Gas::N2),
        // Lava, and a kind the contract adds later: an imposed 0.
        _ => None,
    };
    let inventory = match condensate {
        Some(pressure) => trace.record(
            Code::of(codes::LIQUID_INVENTORY),
            column_mass(
                *pressure.value(),
                *bulk.surface_gravity.value(),
                Metre::from(*bulk.radius.value()),
            ),
            &[
                pressure.node(),
                bulk.surface_gravity.node(),
                bulk.radius.node(),
                kind.node(),
            ],
        )?,
        None => trace.leaf(
            Code::of(codes::LIQUID_INVENTORY),
            Kilogram(0.0),
            Provenance::Imposed,
        )?,
    };
    Ok(Some(SurfaceLiquid {
        kind,
        inventory,
        ocean_share: imposed_share(trace)?,
    }))
}

/// The obliquity the climate of `chained` read, `None` without a surface:
/// the one [`super::ClassRead`] carries (D2).
pub(super) fn obliquity_of(chained: &Chained) -> Option<Degree> {
    chained.rotation.map(|rotation| *rotation.obliquity.value())
}
