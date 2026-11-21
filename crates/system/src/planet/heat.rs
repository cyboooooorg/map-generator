//! The internal heat of a planet: radiogenic, secular and tidal power,
//! each recorded with the nodes it comes from.

use super::codes;
use super::interior::Water;
use super::interior::{Interior, InteriorError};
use crate::Trial;
use science::constants::ASTRONOMICAL_UNIT;
use science::heat::{TidalKind, k2_over_q, radiogenic, secular, tidal};
use science::units::{Dex, Fraction, Kilogram, Metre};
use timeline::{Code, Heat, Trace, Traced};

/// Calibration: least share of the mass in water and ices that makes a
/// body without an envelope respond to tides as an icy one, 0.05. Searched:
/// a published threshold of the tidal response against the composition;
/// none found (study `heat-and-composition.md` section A4). From legacy:
/// Europa (about 8 % water) is icy, Io and Earth (0.02 %) are rocky.
pub(crate) const ICY_MIN_WATER: f64 = 0.05;

/// Calibration: least share of the mass in hydrogen and helium that makes
/// a body respond to tides as an enveloped one, 0.001. Searched: a
/// published threshold; none found (same study section). From legacy: from
/// 0.1 % the envelope lifts a 5 Earth-mass core by about 0.15 Earth radii
/// (Lopez & Fortney 2014, ApJ 792, 1).
pub(crate) const ENVELOPED_MIN_ENVELOPE: f64 = 1e-3;

/// The kind of body that sets its tidal response: enveloped from
/// [`ENVELOPED_MIN_ENVELOPE`], whatever its water; otherwise icy from
/// [`ICY_MIN_WATER`]; otherwise rocky.
pub(crate) fn tidal_kind(envelope_fraction: f64, water_fraction: f64) -> TidalKind {
    if envelope_fraction >= ENVELOPED_MIN_ENVELOPE {
        TidalKind::Enveloped
    } else if water_fraction >= ICY_MIN_WATER {
        TidalKind::Icy
    } else {
        TidalKind::Rocky
    }
}

/// The heat sources of the planet at the age of the trial. The silicate
/// is the interior's mantle (its recorded mass node, which holds a dry
/// planet's water); the tide comes from the primary star on the trial's
/// orbit, with a response set by the composition and the envelope, on the
/// recorded radius.
///
/// # Errors
///
/// The errors of the trace.
pub(crate) fn heat(
    trial: &Trial,
    interior: &Interior,
    abundance: &Traced<Dex>,
    envelope: &Traced<Fraction>,
    trace: &mut Trace,
) -> Result<Heat, InteriorError> {
    let water = interior.composition.water_fraction;
    let silicate = interior.mantle_mass;
    let age = trial.age;

    let radiogenic_power = trace.record(
        Code::of(codes::HEAT_RADIOGENIC),
        radiogenic(
            *silicate.value(),
            10f64.powf(abundance.value().0),
            *age.value(),
        ),
        &[silicate.node(), abundance.node(), age.node()],
    )?;
    let secular_power = trace.record(
        Code::of(codes::HEAT_SECULAR),
        secular(*silicate.value(), *age.value()),
        &[silicate.node(), age.node()],
    )?;

    let primary = trial.sources.primary().mass;
    let orbit = &trial.orbit;
    // A water beyond the laws is not part of the condensed planet that
    // responds to the tide: it does not make the body icy.
    let icy_water = match interior.water {
        Water::BeyondLaw { .. } => 0.0,
        _ => water.value().get(),
    };
    let kind = tidal_kind(envelope.value().get(), icy_water);
    let tidal_power = trace.record(
        Code::of(codes::HEAT_TIDAL),
        tidal(
            Kilogram::from(*primary.value()),
            Metre::from(*interior.bulk.radius.value()),
            Metre(orbit.semi_major_axis.value().0 * ASTRONOMICAL_UNIT.0),
            orbit.eccentricity.value().get(),
            k2_over_q(kind),
        ),
        &[
            primary.node(),
            interior.bulk.radius.node(),
            orbit.semi_major_axis.node(),
            orbit.eccentricity.node(),
            water.node(),
            envelope.node(),
        ],
    )?;
    Ok(Heat {
        radiogenic: radiogenic_power,
        tidal: tidal_power,
        secular: secular_power,
    })
}
