//! The internal heat of a planet: radiogenic, secular and tidal power,
//! each recorded with the nodes it comes from.

use super::codes;
use super::enveloped::Body;
use super::interior::Water;
use super::interior::{Interior, InteriorError};
use crate::Trial;
use science::constants::ASTRONOMICAL_UNIT;
use science::heat::{TidalKind, k2_over_q, radiogenic, secular, tidal};
use science::units::{Dex, Fraction, Kilogram, Metre, Watt};
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

/// Definition: a giant's radiogenic power, 0 W: the internal luminosity
/// of the Fortney et al. 2007 tables, its secular heat, already includes
/// whatever its interior radiates (one model for the radius and the heat).
const GIANT_RADIOGENIC: Watt = Watt(0.0);

/// The heat sources of the planet at the age of the trial.
///
/// - A rocky planet, and a mini-Neptune's core: radiogenic and secular
///   power of the interior's mantle (its recorded mass node, which holds a
///   dry planet's water).
/// - A giant: the tables' internal luminosity as the secular power, no
///   radiogenic power apart ([`GIANT_RADIOGENIC`]).
///
/// The tide comes from the primary star on the trial's orbit, with a
/// response set by the composition and the envelope, on the radius of the
/// whole planet.
///
/// # Errors
///
/// The errors of the trace.
pub(crate) fn heat(
    trial: &Trial,
    body: &Body,
    abundance: &Traced<Dex>,
    envelope: &Traced<Fraction>,
    trace: &mut Trace,
) -> Result<Heat, InteriorError> {
    let radius = body.bulk().radius;
    let (radiogenic_power, secular_power, water) = match body {
        Body::Rocky(interior) | Body::MiniNeptune { core: interior, .. } => {
            let (radiogenic, secular) = condensed(trial, interior, abundance, trace)?;
            // A water beyond the laws is not part of the condensed planet
            // that responds to the tide: it does not make the body icy.
            let icy_water = match interior.water {
                Water::BeyondLaw { .. } => 0.0,
                _ => interior.composition.water_fraction.value().get(),
            };
            let water = interior.composition.water_fraction;
            (radiogenic, secular, Some((icy_water, water.node())))
        }
        Body::Giant(record) => {
            let luminosity = record.luminosity;
            let radiogenic = trace.record(
                Code::of(codes::HEAT_RADIOGENIC),
                GIANT_RADIOGENIC,
                &[luminosity.node()],
            )?;
            let secular = trace.record(
                Code::of(codes::HEAT_SECULAR),
                *luminosity.value(),
                &[luminosity.node()],
            )?;
            (radiogenic, secular, None)
        }
    };

    let primary = trial.sources.primary().mass;
    let orbit = &trial.orbit;
    let kind = tidal_kind(envelope.value().get(), water.map_or(0.0, |(w, _)| w));
    let mut from = vec![
        primary.node(),
        radius.node(),
        orbit.semi_major_axis.node(),
        orbit.eccentricity.node(),
    ];
    from.extend(water.map(|(_, node)| node));
    from.push(envelope.node());
    let tidal_power = trace.record(
        Code::of(codes::HEAT_TIDAL),
        tidal(
            Kilogram::from(*primary.value()),
            Metre::from(*radius.value()),
            Metre(orbit.semi_major_axis.value().0 * ASTRONOMICAL_UNIT.0),
            orbit.eccentricity.value().get(),
            k2_over_q(kind),
        ),
        &from,
    )?;
    Ok(Heat {
        radiogenic: radiogenic_power,
        tidal: tidal_power,
        secular: secular_power,
    })
}

/// The radiogenic and secular power of a condensed interior, from its
/// mantle at the age of the trial.
fn condensed(
    trial: &Trial,
    interior: &Interior,
    abundance: &Traced<Dex>,
    trace: &mut Trace,
) -> Result<(Traced<Watt>, Traced<Watt>), InteriorError> {
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
    Ok((radiogenic_power, secular_power))
}
