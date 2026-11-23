//! What a planet's hydrogen-helium envelope loses to its stars' X-ray and
//! extreme-ultraviolet (XUV) light over its age, and the body that is
//! left: the energy-limited escape of Owen & Wu 2017 (eq. 19,
//! [`energy_limited_loss`]) on the formed body, capped by the envelope.
//! Ported from `legacy-final:crates/mapgen/src/planet/escape.rs`
//! (`envelope_after_escape`).
//!
//! The loss is taken at the radius of the formed body, before the loss:
//! an upper bound, the planet shrinking as it thins (as in legacy).

use super::codes;
use super::draws::Drawn;
use super::enveloped::Body;
use super::interior::{InteriorError, interior};
use crate::Trial;
use science::escape::{MIN_ENVELOPE, energy_limited_loss};
use science::units::{EarthMass, Fraction, Kilogram, Metre};
use timeline::{Code, NodeId, Trace, Traced};

/// A planet once the escape has taken its share of the envelope.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Escaped {
    /// The trial at the kept mass (`planet.mass.kept`), or the trial as
    /// given when nothing was lost.
    pub trial: Trial,
    /// The composition with the kept envelope fraction
    /// (`planet.envelope.kept-fraction`), or as given when nothing was
    /// lost. The water fraction (of the planet without its envelope)
    /// stays.
    pub drawn: Drawn,
    /// The body built again at the kept mass and envelope, or the formed
    /// body when nothing was lost.
    pub body: Body,
    /// The envelope lost (`planet.envelope.lost`); `None`, and no node
    /// recorded, when nothing was lost.
    pub lost: Option<Traced<Kilogram>>,
}

/// The planet `trial` of composition `drawn` and formed `body` once its
/// envelope `f M` has lost [`energy_limited_loss`] at the body's radius
/// and the trial's mass, under the XUV fluence of all its stars, capped to
/// `[0, f M]`:
///
/// - the kept mass is `M - lost`;
/// - a mini-Neptune keeps the fraction `(f M - lost) / (M - lost)`; under
///   [`MIN_ENVELOPE`] its envelope is gone: the fraction is 0 and the mass
///   `M - f M`, and the body is rebuilt rocky;
/// - a giant keeps `1 - Mz / M'`, from its drawn heavy mass `Mz` and the
///   kept mass `M'` (the same value, with its own cause), with the same
///   floor, tested on that very fraction.
///
/// The floor applies whatever the loss, as in legacy
/// (`envelope_after_escape`): a drawn envelope under [`MIN_ENVELOPE`] is
/// gone even when no XUV light reaches the planet. No loss is recorded
/// then (`lost` is `None`): the floor, not the escape, drops it, so the
/// kept mass and fraction derive from the trial's mass and the drawn
/// fraction alone; a zero-loss node would put the stars' light in the
/// cause of a value it did not change.
///
/// Without an envelope, or when nothing is lost from an envelope at or
/// above the floor, the body is returned as given and nothing is
/// recorded; otherwise [`interior`](super::interior::interior) builds the
/// body again.
///
/// # Errors
///
/// The errors of the trace and of [`interior`](super::interior::interior).
pub(crate) fn escaped(
    trial: &Trial,
    drawn: &Drawn,
    body: Body,
    trace: &mut Trace,
) -> Result<Escaped, InteriorError> {
    let (mass, f) = (trial.mass, drawn.envelope_fraction);
    let total = Kilogram::from(*mass.value());
    let envelope = total * f.value().get();
    let radius = body.bulk().radius;
    let (_, fluence) = trial.sources.band_total(|s| &s.xuv);
    let loss = energy_limited_loss(Metre::from(*radius.value()), total, fluence)
        .0
        .clamp(0.0, envelope.0);
    // The kept mass and fraction, computed once: the floor tests the
    // fraction recorded.
    let kept = EarthMass::from(Kilogram(total.0 - loss));
    let left = match drawn.heavy_mass {
        Some(heavy) => 1.0 - heavy.value().0 / kept.0,
        None => (envelope.0 - loss) / (total.0 - loss),
    };
    let stripped = left < MIN_ENVELOPE;
    if envelope.0 == 0.0 || (loss == 0.0 && !stripped) {
        return Ok(Escaped {
            trial: trial.clone(),
            drawn: *drawn,
            body,
            lost: None,
        });
    }
    let lost = if loss > 0.0 {
        let mut from: Vec<NodeId> = vec![radius.node(), mass.node()];
        from.extend(trial.sources.iter().map(|s| s.xuv.fluence.node()));
        from.push(f.node());
        Some(trace.record(Code::of(codes::ENVELOPE_LOST), Kilogram(loss), &from)?)
    } else {
        None
    };
    // The floor's cause: the mass, the drawn fraction and the loss if any.
    let mut floored: Vec<NodeId> = vec![mass.node(), f.node()];
    floored.extend(lost.map(|l| l.node()));

    let kept_mass = match lost {
        Some(lost) if !stripped => trace.record(
            Code::of(codes::MASS_KEPT),
            kept,
            &[mass.node(), lost.node()],
        )?,
        _ => trace.record(
            Code::of(codes::MASS_KEPT),
            *mass.value() * (1.0 - f.value().get()),
            &floored,
        )?,
    };
    let kept_fraction = match (stripped, drawn.heavy_mass) {
        (true, _) => trace.record(
            Code::of(codes::ENVELOPE_KEPT_FRACTION),
            Fraction::new(0.0).expect("zero is a fraction"),
            &floored,
        )?,
        (false, Some(heavy)) => trace.record(
            Code::of(codes::ENVELOPE_KEPT_FRACTION),
            Fraction::try_from(left).map_err(InteriorError::NotAFraction)?,
            &[kept_mass.node(), heavy.node()],
        )?,
        (false, None) => trace.record(
            Code::of(codes::ENVELOPE_KEPT_FRACTION),
            Fraction::try_from(left).map_err(InteriorError::NotAFraction)?,
            &floored,
        )?,
    };
    let trial = Trial {
        mass: kept_mass,
        ..trial.clone()
    };
    let drawn = Drawn {
        envelope_fraction: kept_fraction,
        ..*drawn
    };
    let body = interior(&trial, &drawn, trace)?;
    Ok(Escaped {
        trial,
        drawn,
        body,
        lost,
    })
}
