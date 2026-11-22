//! Reference bodies for the relief tests and previews: Earth, Mars and
//! Venus with their published radius and gravity, on the structure of
//! the timeline's Earth-like fixture. Behind the `fixtures` feature.

use science::constants::{EARTH_MEAN_RADIUS, MARS_SURFACE_GRAVITY, STANDARD_GRAVITY};
use science::units::{EarthRadius, Fraction, Metre, MetrePerSecondSquared};
use timeline::fixtures::{earth_like_fixture, leaf};
use timeline::{PlanetState, TectonicRegime, Trace};

/// Source: Mars's volumetric mean radius, 3389.5 km (NASA Mars Fact
/// Sheet, <https://nssdc.gsfc.nasa.gov/planetary/factsheet/marsfact.html>).
pub const MARS_RADIUS: Metre = Metre(3389.5e3);

/// Source: Venus's volumetric mean radius, 6051.8 km (NASA Venus Fact
/// Sheet, <https://nssdc.gsfc.nasa.gov/planetary/factsheet/venusfact.html>).
const VENUS_RADIUS: Metre = Metre(6051.8e3);

/// Source: Venus's equatorial surface gravity, 8.87 m/s^2 (NASA Venus
/// Fact Sheet).
const VENUS_GRAVITY: MetrePerSecondSquared = MetrePerSecondSquared(8.87);

/// An Earth-like fixture with its radius, surface gravity (m/s^2),
/// tectonic regime and ocean share replaced; `None` removes the liquid.
pub fn relief_body(
    trace: &mut Trace,
    regime: TectonicRegime,
    radius: Metre,
    gravity: f64,
    ocean_share: Option<f64>,
) -> PlanetState {
    let mut state = earth_like_fixture(trace, 4.57);
    state.bulk.radius = leaf(trace, "bulk.radius", EarthRadius::from(radius));
    state.bulk.surface_gravity = leaf(trace, "bulk.gravity", MetrePerSecondSquared(gravity));
    let surface = state.surface.as_mut().unwrap();
    surface.regime = leaf(trace, "surface.regime", regime);
    match ocean_share {
        Some(share) => {
            let liquid = surface.liquid.as_mut().unwrap();
            liquid.ocean_share = leaf(trace, "liquid.share", Fraction::new(share).unwrap());
        }
        None => surface.liquid = None,
    }
    state
}

/// Earth: mean radius 6371.0 km, standard gravity 9.80665 m/s^2 (NASA
/// Planetary Fact Sheet, <https://nssdc.gsfc.nasa.gov/planetary/factsheet/>;
/// the constants of the science crate). Plates, 71 % ocean.
pub fn earth_relief_fixture(trace: &mut Trace) -> PlanetState {
    relief_body(
        trace,
        TectonicRegime::Plates,
        EARTH_MEAN_RADIUS,
        STANDARD_GRAVITY.0,
        Some(0.71),
    )
}

/// Mars: [`MARS_RADIUS`], mean surface gravity [`MARS_SURFACE_GRAVITY`]
/// (NASA Mars Fact Sheet). A stagnant lid.
pub fn mars_relief_fixture(trace: &mut Trace, ocean_share: Option<f64>) -> PlanetState {
    relief_body(
        trace,
        TectonicRegime::Stagnant,
        MARS_RADIUS,
        MARS_SURFACE_GRAVITY.0,
        ocean_share,
    )
}

/// Venus: [`VENUS_RADIUS`] and [`VENUS_GRAVITY`]; no liquid, episodic
/// resurfacing.
pub fn venus_relief_fixture(trace: &mut Trace) -> PlanetState {
    relief_body(
        trace,
        TectonicRegime::Episodic,
        VENUS_RADIUS,
        VENUS_GRAVITY.0,
        None,
    )
}
