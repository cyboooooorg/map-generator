//! The planet chained from its draw to its bare air: the composition, the
//! body, the escape of its envelope, its heat and, for a planet with a
//! surface, its water, its tectonic regime, its core and its air: what it
//! outgasses, what the cosmic shoreline keeps, the methane photolysis, the
//! reducing air (its CO2 held back), the cold trap at the equilibrium
//! temperature and the photochemistry, before any climate. Ported from
//! `legacy-final:crates/mapgen/src/planet/mod.rs` (`chain`, l.480-640,
//! and `bare_air`, l.895-1075), in their order.
//!
//! Not in this chain: the climate, which [`super::settled`] chains after
//! it (the condensation it drives, and the reducing-air fallback that
//! reads it, which calls [`bare_air`] again on the same ground); the
//! boiling loop and life (#659); the non-thermal loss and a giant's
//! magnetosphere over its moons (#658). PR 2d replaces the end of the
//! chain with the climate.

use super::air::air;
use super::core::{CoreRecord, core};
use super::draws::{Drawn, draw};
use super::enveloped::{Body, Exposed};
use super::escape::escaped;
use super::heat::heat;
use super::interior::{Interior, InteriorError, interior};
use super::regime::regime;
use super::volcanism::volcanism;
use super::water::{after_steam, surface_water, water};
use crate::Trial;
use science::escape::MIN_ENVELOPE;
use science::units::Kilogram;
use science::units::{Bar, Fraction};
use timeline::{Atmosphere, Heat, Seed, TectonicRegime, Trace, Traced, Water};

/// A planet from its draw to its bare air.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Bare {
    /// The trial at the mass the planet kept: after the escape of its
    /// envelope and, in a runaway, after the steam it lost.
    pub trial: Trial,
    /// The composition the planet kept, at the same steps as `trial`.
    pub drawn: Drawn,
    /// The body of `trial` and `drawn`.
    pub body: Body,
    /// The internal heat. After a runaway, that of the body the steam's
    /// first loss left: the cosmic shoreline's later loss keeps it
    /// (legacy `mod` l.577-590).
    pub heat: Heat,
    /// The envelope the escape took; `None` when it took none.
    pub envelope_lost: Option<Traced<Kilogram>>,
    /// The surface; `None` for a planet that kept an envelope.
    pub surface: Option<BareSurface>,
}

/// The surface of a planet without an envelope, before its climate.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BareSurface {
    pub regime: Traced<TectonicRegime>,
    /// The volcanic intensity the snowball chance reads
    /// ([`volcanism`]).
    pub volcanism: Traced<Fraction>,
    pub core: timeline::Core,
    /// The water once the air is made: in a runaway, the vapour the
    /// cosmic shoreline kept.
    pub water: Water,
    pub atmosphere: Atmosphere,
    /// The CO2 outgassed before the shoreline and the cold trap
    /// (`planet.air.outgassed.co2`): the carbon inventory of the
    /// climate's carbon cycle.
    pub outgassed_co2: Traced<Bar>,
    /// The CO2 the cold trap froze out, which the climate's carbon cycle
    /// reads.
    pub frozen_co2: Traced<Bar>,
    /// Whether the air is reducing, Titan's: its CO2 was held back. The
    /// climate's fallback reads it.
    pub reducing: Traced<bool>,
}

/// The planet `trial` drawn from `seed` ([`draw`]) and chained to its
/// bare air ([`bare_with`]).
///
/// # Errors
///
/// The errors of [`draw`] and [`bare_with`].
pub(crate) fn bare(trial: &Trial, seed: Seed, trace: &mut Trace) -> Result<Bare, InteriorError> {
    let drawn = draw(seed, trial, trace)?;
    bare_with(trial, &drawn, trace)
}

/// The planet `trial` of composition `drawn`, chained in legacy's order:
///
/// 1. its body ([`interior`]), the escape of its envelope ([`escaped`])
///    and its heat ([`heat`]) on the body that is left;
/// 2. no surface when the planet kept an envelope, at or above
///    [`MIN_ENVELOPE`] (legacy's test); under it the escape left no
///    envelope at all, even without XUV light, and a rocky body;
/// 3. its surface water ([`surface_water`]: a share of a dry planet's
///    bulk water, a water world's whole water) and the water its light
///    leaves of it ([`water`]); after a runaway, the planet its steam
///    left ([`after_steam`]), its body and heat built again;
/// 4. its regime ([`regime`]), its core ([`core`]) and its air ([`air`],
///    a reducing air's CO2 held back), the air reading the trial and
///    composition from before the steam's loss so that the volatiles keep
///    their mass;
/// 5. after a runaway whose cosmic shoreline took some of the vapour, the
///    planet left once more ([`after_steam`] of the planet before the
///    steam with the water kept) and its body built again, keeping the
///    heat of step 3.
///
/// # Errors
///
/// The errors of each step.
pub(crate) fn bare_with(
    trial: &Trial,
    drawn: &Drawn,
    trace: &mut Trace,
) -> Result<Bare, InteriorError> {
    match formed(trial, drawn, trace)? {
        Ground::Enveloped(bare) => Ok(*bare),
        Ground::Formed(f) => {
            let w = water(&f.trial, &f.body.exposed(), f.surface, trace)?;
            let ground = surface_pass(&f, w, None, trace)?;
            bare_air(&ground, true, trace)
        }
    }
}

/// What [`formed`] leaves: a planet that kept an envelope, whole, or a
/// planet with a surface, up to its surface water.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Ground {
    /// A planet that kept an envelope: it has no surface to make an air
    /// for.
    Enveloped(Box<Bare>),
    /// A planet with a surface, before its water.
    Formed(Box<Formed>),
}

/// What runs once for a planet with a surface: steps 1 to 3 of
/// [`bare_with`] up to its surface water. The water it keeps, and what
/// follows, depend on the steam envelope the climate may force
/// ([`surface_pass`]).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Formed {
    /// The trial after the escape of the envelope.
    pub trial: Trial,
    /// The composition at the same step.
    pub drawn: Drawn,
    /// The body of `trial` and `drawn`.
    pub body: Body,
    /// The heat of `body`.
    pub heat: Heat,
    /// The envelope the escape took; `None` when it took none.
    pub envelope_lost: Option<Traced<Kilogram>>,
    /// The surface water before the stars' light.
    pub surface: Traced<Kilogram>,
}

/// The ground of a planet with a surface: everything the chain derives
/// before its air, which [`bare_air`] reads once or twice (the climate's
/// reducing-air fallback runs it again without rebuilding the ground).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Surfaced {
    /// The trial and composition before a runaway's steam loss: the air
    /// reads them so that the volatiles keep their mass.
    before: (Trial, Drawn),
    /// The trial and composition after the steam's first loss.
    after: (Trial, Drawn),
    /// The body of `after`.
    body: Body,
    /// The heat of `after`.
    heat: Heat,
    envelope_lost: Option<Traced<Kilogram>>,
    /// The surface water before the stars' light.
    surface: Traced<Kilogram>,
    /// The water the stars' light leaves of it.
    water: Water,
    regime: Traced<TectonicRegime>,
    volcanism: Traced<Fraction>,
    core: CoreRecord,
    /// What the cosmic shoreline reads of the planet: the radius it
    /// loses its air from and the whole mass that holds it.
    exposed: Exposed,
}

/// The planet `trial` of composition `drawn`, chained in legacy's order
/// up to its surface water (steps 1 to 3 of [`bare_with`]): a planet that
/// kept an envelope whole, the others as [`Formed`].
///
/// # Errors
///
/// The errors of each step.
pub(super) fn formed(
    trial: &Trial,
    drawn: &Drawn,
    trace: &mut Trace,
) -> Result<Ground, InteriorError> {
    let body = interior(trial, drawn, trace)?;
    let escaped = escaped(trial, drawn, body, trace)?;
    let heat = heat(
        &escaped.trial,
        &escaped.body,
        &escaped.drawn.radiogenic_abundance,
        &escaped.drawn.envelope_fraction,
        trace,
    )?;
    if escaped.drawn.envelope_fraction.value().get() >= MIN_ENVELOPE {
        return Ok(Ground::Enveloped(Box::new(Bare {
            trial: escaped.trial,
            drawn: escaped.drawn,
            body: escaped.body,
            heat,
            envelope_lost: escaped.lost,
            surface: None,
        })));
    }
    let surface = surface_water(&escaped.trial, &escaped.drawn, &escaped.body, trace)?;
    Ok(Ground::Formed(Box::new(Formed {
        trial: escaped.trial,
        drawn: escaped.drawn,
        body: escaped.body,
        heat,
        envelope_lost: escaped.lost,
        surface,
    })))
}

/// The ground of the planet `formed` left once its water is `water`
/// (steps 3 to 4 of [`bare_with`], up to the core): after a runaway, the
/// planet its steam left ([`after_steam`]), its body and heat built
/// again; then its regime, its volcanism and its core. Repeatable: the
/// climate's boiling loop runs it again from a forced steam or a
/// recondensed water.
///
/// Escape (the shoreline in [`bare_air`]) reads `exposed` when given,
/// otherwise the body this pass built.
///
/// # Errors
///
/// The errors of each step.
pub(super) fn surface_pass(
    formed: &Formed,
    water: Water,
    exposed: Option<&Exposed>,
    trace: &mut Trace,
) -> Result<Surfaced, InteriorError> {
    let (t, d, body, heat) =
        match after_steam(&formed.trial, &formed.drawn, formed.surface, &water, trace)? {
            Some((t, d)) => {
                let body = interior(&t, &d, trace)?;
                let h = heat(
                    &t,
                    &body,
                    &d.radiogenic_abundance,
                    &d.envelope_fraction,
                    trace,
                )?;
                (t, d, body, h)
            }
            None => (
                formed.trial.clone(),
                formed.drawn,
                formed.body.clone(),
                formed.heat.clone(),
            ),
        };
    let regime = regime(&t, &body, &heat, &water, trace)?;
    let volcanism = volcanism(&body, &heat, &regime, trace)?;
    let core = core(&t, rock_of(&body), &regime, trace)?;
    let exposed = exposed.cloned().unwrap_or_else(|| body.exposed());
    Ok(Surfaced {
        before: (formed.trial.clone(), formed.drawn),
        after: (t, d),
        body,
        heat,
        envelope_lost: formed.envelope_lost,
        surface: formed.surface,
        water,
        regime,
        volcanism,
        core,
        exposed,
    })
}

/// The air of `ground` (the rest of steps 4 and 5 of [`bare_with`]): [`air`] (a
/// reducing air's CO2 held back when `hold_back`), then, after a runaway
/// whose cosmic shoreline took some of the vapour, the planet left once
/// more and its body built again, keeping the heat of the first loss. The
/// ground is not rebuilt: the climate's fallback calls this a second time
/// on the same ground.
///
/// # Errors
///
/// The errors of each step.
pub(super) fn bare_air(
    ground: &Surfaced,
    hold_back: bool,
    trace: &mut Trace,
) -> Result<Bare, InteriorError> {
    let (before_trial, before_drawn) = &ground.before;
    let made = air(
        before_trial,
        before_drawn,
        &ground.body,
        &ground.exposed,
        ground.surface,
        &ground.water,
        &ground.regime,
        &ground.core,
        hold_back,
        trace,
    )?;
    let kept = made.water;
    let (mut t, mut d) = ground.after.clone();
    let mut body = ground.body.clone();
    if *kept.runaway.value()
        && kept.inventory.value().0 < ground.water.inventory.value().0
        && let Some((left, left_drawn)) =
            after_steam(before_trial, before_drawn, ground.surface, &kept, trace)?
    {
        body = interior(&left, &left_drawn, trace)?;
        (t, d) = (left, left_drawn);
    }
    Ok(Bare {
        trial: t,
        drawn: d,
        body,
        heat: ground.heat.clone(),
        envelope_lost: ground.envelope_lost,
        surface: Some(BareSurface {
            regime: ground.regime,
            volcanism: ground.volcanism,
            core: ground.core.core.clone(),
            water: kept,
            atmosphere: made.atmosphere,
            outgassed_co2: made.outgassed_co2,
            frozen_co2: made.frozen_co2,
            reducing: made.reducing,
        }),
    })
}

/// The interior of a body without an envelope: [`interior`] builds a
/// rocky one whenever the envelope fraction is 0, which the escape and
/// [`after_steam`] keep for a planet with a surface.
pub(super) fn rock_of(body: &Body) -> &Interior {
    match body {
        Body::Rocky(interior) => interior,
        _ => unreachable!("a planet with a surface has no envelope, so a rocky body"),
    }
}
