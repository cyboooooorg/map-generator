//! The planet chained from its draw to its bare air: the composition, the
//! body, the escape of its envelope, its heat and, for a planet with a
//! surface, its water, its tectonic regime, its core and the air it
//! outgasses, before any climate. Ported from
//! `legacy-final:crates/mapgen/src/planet/mod.rs` (`chain`, l.480-640,
//! and `bare_air`, l.895-1075), in their order.
//!
//! Not in this chain yet (#659): the climate and its boiling loop, the
//! methane photolysis, the cold trap, the photochemistry and life; nor
//! the non-thermal loss and a giant's magnetosphere over its moons
//! (#658). PR 2d replaces the end of the chain with the climate.

use super::air::air;
use super::core::core;
use super::draws::{Drawn, draw};
use super::enveloped::Body;
use super::escape::escaped;
use super::heat::heat;
use super::interior::{Interior, InteriorError, interior};
use super::regime::regime;
use super::water::{after_steam, water};
use crate::Trial;
use science::escape::MIN_ENVELOPE;
use science::units::Kilogram;
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
    pub core: timeline::Core,
    /// The water once the air is made: in a runaway, the vapour the
    /// cosmic shoreline kept.
    pub water: Water,
    pub atmosphere: Atmosphere,
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
/// 2. no surface when the planet kept an envelope: at or above
///    [`MIN_ENVELOPE`] (legacy's test), or a thinner one no escape took
///    (no XUV light), which keeps its enveloped body; a surface for a
///    rocky body, which after the escape has no envelope at all;
/// 3. its water ([`water`]); after a runaway, the planet its steam left
///    ([`after_steam`]), its body and heat built again;
/// 4. its regime ([`regime`]), its core ([`core`]) and its air ([`air`]),
///    the air reading the trial and composition from before the steam's
///    loss so that the volatiles keep their mass;
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
    let heat_of = |trial: &Trial, drawn: &Drawn, body: &Body, trace: &mut Trace| {
        heat(
            trial,
            body,
            &drawn.radiogenic_abundance,
            &drawn.envelope_fraction,
            trace,
        )
    };
    let formed = interior(trial, drawn, trace)?;
    let escaped = escaped(trial, drawn, formed, trace)?;
    let formed_heat = heat_of(&escaped.trial, &escaped.drawn, &escaped.body, trace)?;
    let enveloped = escaped.drawn.envelope_fraction.value().get() >= MIN_ENVELOPE;
    if enveloped || !matches!(escaped.body, Body::Rocky(_)) {
        return Ok(Bare {
            trial: escaped.trial,
            drawn: escaped.drawn,
            body: escaped.body,
            heat: formed_heat,
            envelope_lost: escaped.lost,
            surface: None,
        });
    }

    let w = water(&escaped.trial, &escaped.drawn, &escaped.body, trace)?;
    let (mut t, mut d, mut body, heat) =
        match after_steam(&escaped.trial, &escaped.drawn, &w, trace)? {
            Some((t, d)) => {
                let body = interior(&t, &d, trace)?;
                let h = heat_of(&t, &d, &body, trace)?;
                (t, d, body, h)
            }
            None => (
                escaped.trial.clone(),
                escaped.drawn,
                escaped.body,
                formed_heat,
            ),
        };
    let r = regime(&t, &body, &heat, &w, trace)?;
    let k = core(&t, rocky(&body), &r, trace)?;
    let (atmosphere, kept) = air(&escaped.trial, &escaped.drawn, &body, &w, &r, &k, trace)?;
    if *kept.runaway.value()
        && kept.inventory.value().0 < w.inventory.value().0
        && let Some((left, left_drawn)) = after_steam(&escaped.trial, &escaped.drawn, &kept, trace)?
    {
        body = interior(&left, &left_drawn, trace)?;
        (t, d) = (left, left_drawn);
    }
    Ok(Bare {
        trial: t,
        drawn: d,
        body,
        heat,
        envelope_lost: escaped.lost,
        surface: Some(BareSurface {
            regime: r,
            core: k.core,
            water: kept,
            atmosphere,
        }),
    })
}

/// The interior of a body without an envelope: [`interior`] builds a
/// rocky one whenever the envelope fraction is 0, which the escape and
/// [`after_steam`] keep for a planet with a surface.
fn rocky(body: &Body) -> &Interior {
    match body {
        Body::Rocky(interior) => interior,
        _ => unreachable!("a planet with a surface has no envelope, so a rocky body"),
    }
}
