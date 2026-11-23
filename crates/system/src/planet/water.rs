//! The water a planet keeps under its stars' light: the water at its
//! surface (#663), the runaway greenhouse threshold summed over every
//! source (#592), the steam a runaway loses to the XUV light, and the
//! planet that is left once that steam is gone. Ported from
//! `legacy-final:crates/mapgen/src/planet/regime.rs` (`water`, `steam`)
//! and `planet/mod.rs` (`after_steam`); the surface share is the
//! rewrite's (`science::habitable::SURFACE_WATER_SHARE`), legacy having
//! drawn the ocean itself.

use super::codes;
use super::draws::Drawn;
use super::enveloped::Body;
use super::interior::{InteriorError, Water as Held};
use crate::Trial;
use science::escape::{WATER_PER_HYDROGEN, energy_limited_loss};
use science::habitable::{SURFACE_WATER_SHARE, runaway_flux};
use science::units::{EarthMass, Fraction, Kilogram, Metre};
use timeline::{Code, NodeId, Provenance, Trace, Traced, Water};

/// The water at the surface of the planet `trial` of composition `drawn`
/// and body `body`, before its stars' light acts on it.
///
/// A planet whose water is bound in its mantle (a dry planet) keeps
/// [`SURFACE_WATER_SHARE`] of its bulk water `w (1 - f) M` at its surface,
/// its ocean (`planet.water.surface`, from the mass and the two
/// fractions); the rest stays in the mantle. A water world's water is at
/// its surface already: its water layer, or its water beyond the laws,
/// whose node is returned as it is (`docs/science/planet-interiors.md`,
/// "Water inside the ice line"). A giant holds no water of its own and is
/// never asked: it is read as a dry planet.
///
/// # Errors
///
/// The errors of the trace.
pub(crate) fn surface_water(
    trial: &Trial,
    drawn: &Drawn,
    body: &Body,
    trace: &mut Trace,
) -> Result<Traced<Kilogram>, InteriorError> {
    let held = match body {
        Body::Rocky(interior) | Body::MiniNeptune { core: interior, .. } => interior.water,
        Body::Giant(_) => Held::InMantle,
    };
    match held {
        Held::Layer { mass, .. } | Held::BeyondLaw { mass, .. } => Ok(mass),
        Held::InMantle => Ok(trace.record(
            Code::of(codes::WATER_SURFACE),
            bulk_water(trial, drawn) * SURFACE_WATER_SHARE,
            &[
                trial.mass.node(),
                drawn.water_fraction.node(),
                drawn.envelope_fraction.node(),
            ],
        )?),
    }
}

/// The water of the planet `trial` of body `body`, whose surface water
/// ([`surface_water`]) is `surface`.
///
/// The planet runs away when `Σ flux_i / runaway_flux(Teff_i) > 1` over
/// its sources, each with its own effective temperature (Kopparapu et al.
/// 2014's limit, [`runaway_flux`], summed as agreed on #592); with one
/// source this is legacy's test. The runaway node derives from every
/// source's flux and temperature.
///
/// Without a runaway it keeps its surface water. In a runaway that water
/// is steam in the air (the mantle's water stays where it is), where the
/// XUV light splits it and the hydrogen escapes at the energy-limited
/// rate ([`energy_limited_loss`] at the body's radius and its whole mass,
/// [`Body::whole_mass`], under the XUV fluence of all its stars) with
/// [`WATER_PER_HYDROGEN`] kg of water for each kg of hydrogen: the steam
/// lost (`planet.water.steam-lost`) is `min(9 H, surface)`, and the
/// inventory what is left. The oxygen left behind is not tracked. For a
/// water beyond the laws the whole mass holds the steam, at the condensed
/// planet's radius: the only radius there is until PR 2d's steam
/// envelope.
///
/// `boiled` is false: the climate's boiling loop is the next pull request's.
///
/// # Errors
///
/// The errors of the trace.
pub(crate) fn water(
    trial: &Trial,
    body: &Body,
    surface: Traced<Kilogram>,
    trace: &mut Trace,
) -> Result<Water, InteriorError> {
    let pressure: f64 = trial
        .sources
        .iter()
        .map(|s| s.flux.value().0 / runaway_flux(*s.effective_temperature.value()).0)
        .sum();
    let light: Vec<NodeId> = trial
        .sources
        .iter()
        .flat_map(|s| [s.flux.node(), s.effective_temperature.node()])
        .collect();
    let runaway = trace.record(Code::of(codes::WATER_RUNAWAY), pressure > 1.0, &light)?;
    let inventory = if *runaway.value() {
        let radius = body.bulk().radius;
        let (mass, mass_from) = body.whole_mass();
        let (_, fluence) = trial.sources.band_total(|s| &s.xuv);
        let hydrogen = energy_limited_loss(Metre::from(*radius.value()), mass, fluence);
        let mut from: Vec<NodeId> = vec![runaway.node(), radius.node()];
        from.extend(mass_from);
        from.extend(trial.sources.iter().map(|s| s.xuv.fluence.node()));
        from.push(surface.node());
        let lost = trace.record(
            Code::of(codes::WATER_STEAM_LOST),
            Kilogram((WATER_PER_HYDROGEN * hydrogen.0).min(surface.value().0)),
            &from,
        )?;
        trace.record(
            Code::of(codes::WATER_INVENTORY),
            Kilogram(surface.value().0 - lost.value().0),
            &[surface.node(), lost.node()],
        )?
    } else {
        trace.record(
            Code::of(codes::WATER_INVENTORY),
            *surface.value(),
            &[surface.node(), runaway.node()],
        )?
    };
    let boiled = trace.leaf(Code::of(codes::WATER_BOILED), false, Provenance::Imposed)?;
    Ok(Water {
        inventory,
        runaway,
        boiled,
    })
}

/// The planet `trial` of composition `drawn`, whose surface water was
/// `surface`, once it kept only `water`'s inventory of it: `None` when it
/// lost none; otherwise the trial at the mass less the surface water lost
/// (`planet.mass.after-steam`, `M - (surface - kept)`: the mantle's water
/// stays) and the composition with the water fraction of what is left
/// (`planet.water.kept-fraction`, `(w M - (surface - kept)) / M'`, which
/// is `kept / M'` for a water world, all of whose water is at its
/// surface). The core mass fraction is a share of the refractory mass,
/// which the steam does not touch.
///
/// The planet has a surface, so no envelope: below
/// [`science::escape::MIN_ENVELOPE`] the escape has set its envelope
/// fraction to 0 (`super::escape`). The volatiles' scatter is kept as
/// drawn: the volatile inventory is computed from the composition before
/// the steam loss, never from the water kept, which would drop the ice
/// volatiles with the steam. The hydrogen's oxygen is not tracked.
///
/// The caller builds the body again with
/// [`interior`](super::interior::interior).
///
/// # Errors
///
/// The errors of the trace, and a kept water fraction outside `[0, 1]`.
pub(crate) fn after_steam(
    trial: &Trial,
    drawn: &Drawn,
    surface: Traced<Kilogram>,
    water: &Water,
    trace: &mut Trace,
) -> Result<Option<(Trial, Drawn)>, InteriorError> {
    debug_assert_eq!(
        drawn.envelope_fraction.value().get(),
        0.0,
        "a planet that keeps a surface has no envelope"
    );
    let kept = water.inventory;
    if kept.value().0 >= surface.value().0 {
        return Ok(None);
    }
    let lost = surface.value().0 - kept.value().0;
    let (mass, w) = (trial.mass, drawn.water_fraction);
    let mass_after = trace.record(
        Code::of(codes::MASS_AFTER_STEAM),
        EarthMass::from(Kilogram(Kilogram::from(*mass.value()).0 - lost)),
        &[mass.node(), surface.node(), kept.node()],
    )?;
    let left = bulk_water(trial, drawn).0 - lost;
    let fraction = trace.record(
        Code::of(codes::WATER_KEPT_FRACTION),
        Fraction::try_from(left / Kilogram::from(*mass_after.value()).0)
            .map_err(InteriorError::NotAFraction)?,
        &[
            mass.node(),
            w.node(),
            surface.node(),
            kept.node(),
            mass_after.node(),
        ],
    )?;
    let trial = Trial {
        mass: mass_after,
        ..trial.clone()
    };
    let drawn = Drawn {
        water_fraction: fraction,
        ..*drawn
    };
    Ok(Some((trial, drawn)))
}

/// The bulk water of the planet, `w (1 - f) M`: the water fraction is a
/// share of the planet without its envelope.
fn bulk_water(trial: &Trial, drawn: &Drawn) -> Kilogram {
    Kilogram::from(*trial.mass.value())
        * (drawn.water_fraction.value().get() * (1.0 - drawn.envelope_fraction.value().get()))
}
