//! What a young mini-Neptune's hydrogen-helium envelope loses to the
//! boil-off when its disc goes, and the body that is left: the ceiling of
//! Tang, Fortney & Murray-Clay 2024 ([`boil_off_ceiling`], their Table 2)
//! on the formed body, before the XUV escape ([`super::escape`]). The
//! study is [study 30](https://github.com/cyboooooorg/map-generator/wiki/30-core-powered-loss-audit);
//! the page is `docs/science/planet-atmospheres.md`, *Boil-off and
//! core-powered mass loss (study 30)*.
//!
//! One bolometric law: the ceiling stands for the boil-off and the
//! core-powered loss after it (at most 0.1 % more, Tang et al. 2024,
//! § 3.5); Gupta & Schlichting's evolution is not added on top, which
//! would count the same outflow twice.
//!
//! The flux is today's bolometric flux of all the stars, the only one the
//! timeline holds; the boil-off happens about 1 Myr after the disc, before
//! the ZAMS the stellar tracks start at (#700 tracks the flux at disc
//! dispersal).

use super::codes;
use super::draws::Drawn;
use super::enveloped::Body;
use super::interior::{InteriorError, interior};
use crate::Trial;
use science::escape::boil_off_ceiling;
use science::units::{EarthMass, Fraction, Kilogram};
use timeline::{Code, Trace, Traced};

/// A planet once the boil-off has cut its envelope to its ceiling.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BoiledOff {
    /// The trial at the mass left (`planet.mass.after-boil-off`), or the
    /// trial as given when nothing was cut.
    pub trial: Trial,
    /// The composition with the envelope fraction left
    /// (`planet.envelope.after-boil-off`), or as given when nothing was
    /// cut. The water fraction (of the planet without its envelope)
    /// stays.
    pub drawn: Drawn,
    /// The body built again on the envelope left, or the formed body when
    /// nothing was cut.
    pub body: Body,
    /// The envelope boiled off (`planet.envelope.boiled-off`); `None`, and
    /// no node recorded, when nothing was cut.
    pub boiled: Option<Traced<Kilogram>>,
}

/// The planet `trial` of composition `drawn` and formed `body` once the
/// boil-off has cut its envelope fraction `f` to the ceiling `c`
/// ([`boil_off_ceiling`]) read at the mass without the envelope `M_c` and
/// the bolometric flux the body was built under:
///
/// - only a mini-Neptune is read: a rocky body has no envelope, and a
///   giant (from `science::giants::GAS_GIANT_MIN_MASS`, 20 M⊕) lies off
///   Tang et al.'s grid, whose heaviest core is 20 M⊕;
/// - when `f ≤ c` nothing is cut and nothing recorded: a ceiling node
///   would put the flux in the cause of a fraction it did not change (as
///   the XUV escape records no zero loss);
/// - otherwise the fraction left is `c`, the mass `M_c / (1 − c)`, the
///   envelope boiled off the difference with the trial's mass, and the
///   body is built again ([`interior`]).
///
/// The XUV escape and [`science::escape::MIN_ENVELOPE`] follow on what is
/// left ([`super::escape::escaped`]).
///
/// # Errors
///
/// The errors of the trace and of [`interior`].
pub(crate) fn boiled_off(
    trial: &Trial,
    drawn: &Drawn,
    body: Body,
    trace: &mut Trace,
) -> Result<BoiledOff, InteriorError> {
    let unchanged = |body| BoiledOff {
        trial: trial.clone(),
        drawn: *drawn,
        body,
        boiled: None,
    };
    let Body::MiniNeptune { envelope, .. } = &body else {
        return Ok(unchanged(body));
    };
    let (core, flux) = (envelope.mass_without, envelope.flux);
    let f = drawn.envelope_fraction;
    let c = boil_off_ceiling(*core.value(), *flux.value());
    if f.value().get() <= c.get() {
        return Ok(unchanged(body));
    }
    let ceiling = trace.record(
        Code::of(codes::BOIL_OFF_CEILING),
        c,
        &[core.node(), flux.node()],
    )?;
    let left: Traced<Fraction> = trace.record(
        Code::of(codes::ENVELOPE_AFTER_BOIL_OFF),
        c,
        &[f.node(), ceiling.node()],
    )?;
    let mass = trace.record(
        Code::of(codes::MASS_AFTER_BOIL_OFF),
        EarthMass(core.value().0 / (1.0 - c.get())),
        &[core.node(), left.node()],
    )?;
    let boiled = trace.record(
        Code::of(codes::ENVELOPE_BOILED_OFF),
        Kilogram::from(*trial.mass.value()) - Kilogram::from(*mass.value()),
        &[trial.mass.node(), mass.node()],
    )?;
    let trial = Trial {
        mass,
        ..trial.clone()
    };
    let drawn = Drawn {
        envelope_fraction: left,
        ..*drawn
    };
    let body = interior(&trial, &drawn, trace)?;
    Ok(BoiledOff {
        trial,
        drawn,
        body,
        boiled: Some(boiled),
    })
}
