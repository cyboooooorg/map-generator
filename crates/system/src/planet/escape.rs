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
///   floor.
///
/// When nothing is lost (no envelope, or no fluence) the body is returned
/// as given and nothing is recorded; otherwise
/// [`interior`](super::interior::interior) builds the body again.
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
    if loss == 0.0 {
        return Ok(Escaped {
            trial: trial.clone(),
            drawn: *drawn,
            body,
            lost: None,
        });
    }
    let mut from: Vec<NodeId> = vec![radius.node(), mass.node()];
    from.extend(trial.sources.iter().map(|s| s.xuv.fluence.node()));
    from.push(f.node());
    let lost = trace.record(Code::of(codes::ENVELOPE_LOST), Kilogram(loss), &from)?;

    let left = (envelope.0 - loss) / (total.0 - loss);
    let stripped = left < MIN_ENVELOPE;
    let kept_mass = if stripped {
        trace.record(
            Code::of(codes::MASS_KEPT),
            *mass.value() * (1.0 - f.value().get()),
            &[mass.node(), f.node(), lost.node()],
        )?
    } else {
        trace.record(
            Code::of(codes::MASS_KEPT),
            EarthMass::from(total - *lost.value()),
            &[mass.node(), lost.node()],
        )?
    };
    let kept_fraction = match (stripped, drawn.heavy_mass) {
        (true, _) => trace.record(
            Code::of(codes::ENVELOPE_KEPT_FRACTION),
            Fraction::new(0.0).expect("zero is a fraction"),
            &[mass.node(), f.node(), lost.node()],
        )?,
        (false, Some(heavy)) => trace.record(
            Code::of(codes::ENVELOPE_KEPT_FRACTION),
            Fraction::try_from(1.0 - heavy.value().0 / kept_mass.value().0)
                .map_err(InteriorError::NotAFraction)?,
            &[kept_mass.node(), heavy.node()],
        )?,
        (false, None) => trace.record(
            Code::of(codes::ENVELOPE_KEPT_FRACTION),
            Fraction::try_from(left).map_err(InteriorError::NotAFraction)?,
            &[mass.node(), f.node(), lost.node()],
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
        lost: Some(lost),
    })
}
