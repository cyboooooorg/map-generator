//! The volcanic intensity of a planet, recorded in the bare chain for the
//! snowball chance of the next pull request (2d-3) to read.

use super::codes;
use super::enveloped::Body;
use super::interior::InteriorError;
use science::outgassing::{LID_SHARE, MOBILE_SHARE};
use science::relief::volcanoes::{heat_flux, volcanic_intensity};
use science::units::{Fraction, Metre, Watt};
use timeline::{Code, Heat, TectonicRegime, Trace, Traced};

/// The volcanic intensity of the planet of body `body`, internal heat
/// `heat` and regime `regime`: [`volcanic_intensity`] of the heat flux
/// through the surface (the flux [`super::regime::regime`] classes the
/// heat pipe by, one [`heat_flux`]), times `LID_SHARE / MOBILE_SHARE`
/// under a stagnant lid, which outgasses that share of what a mobile one
/// does, and times 1 otherwise.
///
/// A heat pipe has no special case (decision B of the 2d-2 plan): the
/// regime classes it from 1 W/m2, the law only reaches 1 at 2.5 W/m2, so
/// a heat pipe reads 0.83 to 1 (issue #670). The snowball chance
/// (`science::snowball::snowball_chance`) divides this value by
/// `EARTH_VOLCANIC_INTENSITY`.
///
/// The node derives from the three heat sources, the radius and the
/// regime.
///
/// # Errors
///
/// The errors of the trace.
pub(crate) fn volcanism(
    body: &Body,
    heat: &Heat,
    regime: &Traced<TectonicRegime>,
    trace: &mut Trace,
) -> Result<Traced<Fraction>, InteriorError> {
    let radius = body.bulk().radius;
    let flux = heat_flux(
        Watt(heat.radiogenic.value().0 + heat.tidal.value().0 + heat.secular.value().0),
        Metre::from(*radius.value()),
    );
    let intensity = volcanic_intensity(flux);
    let scaled = match regime.value() {
        TectonicRegime::Stagnant => intensity.get() * LID_SHARE / MOBILE_SHARE,
        _ => intensity.get(),
    };
    let value = Fraction::try_from(scaled).map_err(InteriorError::NotAFraction)?;
    Ok(trace.record(
        Code::of(codes::SURFACE_VOLCANISM),
        value,
        &[
            heat.radiogenic.node(),
            heat.tidal.node(),
            heat.secular.node(),
            radius.node(),
            regime.node(),
        ],
    )?)
}
