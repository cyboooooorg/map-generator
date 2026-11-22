//! The spectrum of a rocky relief: Hurst exponent, octaves, amplitude, and
//! the scaling of relief with gravity.

use crate::constants::STANDARD_GRAVITY;
use crate::units::{Metre, MetrePerSecondSquared};

/// The body whose measured relief a regime follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Analogue {
    /// Earth, topography of ETOPO 2022.
    Earth,
    /// Mars, topography of MOLA.
    Mars,
}

/// Source: Hurst exponent of the topography at scales above 10 km, Earth
/// 0.479, Mars 0.53 (Landais, Schmidt & Lovejoy 2019, Table 2).
pub fn hurst(analogue: Analogue) -> f64 {
    match analogue {
        Analogue::Earth => 0.479,
        Analogue::Mars => 0.53,
    }
}

/// Definition: ratio of the wavelengths of successive octaves, a sampling
/// choice; with it the persistence `2^-H` keeps the spectrum.
pub const LACUNARITY: f64 = 2.0;

/// Gain of each octave for a Hurst exponent: `LACUNARITY^-hurst`.
pub fn persistence(hurst: f64) -> f64 {
    LACUNARITY.powf(-hurst)
}

/// Definition: the grid's Nyquist limit. Number of octaves from the
/// circumference down to a wavelength of two cells, at least 1.
pub fn octaves(circumference: Metre, cell: Metre) -> u32 {
    let ratio = circumference.0 / (2.0 * cell.0);
    if ratio < 1.0 {
        return 1;
    }
    ratio.log2().floor() as u32 + 1
}

/// Definition: the lag at which [`amplitude`] is measured, 100 km, above
/// the 10-20 km break of the spectrum.
pub const REFERENCE_WAVELENGTH: Metre = Metre(100e3);

/// Source: mean Haar fluctuation of the heights at a lag of
/// [`REFERENCE_WAVELENGTH`]. Earth 219 m (ETOPO 2022, north-south
/// profiles, whole surface), Mars 233 m (MOLA MEGDR, same statistic; Landais
/// et al. 2019, Fig. 2, about 231 m). The noise is scaled to this statistic.
pub fn amplitude(analogue: Analogue) -> Metre {
    match analogue {
        Analogue::Earth => Metre(219.0),
        Analogue::Mars => Metre(233.0),
    }
}

/// Definition: the storage range of a height, 25 km; Olympus Mons, the
/// tallest known relief, stands almost 22 km (Plescia 2004).
pub const HEIGHT_RANGE: Metre = Metre(25_000.0);

/// Source: the largest relief scale, Olympus Mons's relief (almost 22 km,
/// Plescia 2004) over Everest's 8.848 86 km (China-Nepal survey, 2020):
/// 22 / 8.84886 = 2.486, rounded up to 2.49.
pub const MAX_RELIEF_SCALE: f64 = 2.49;

/// Factor on the relief of a body: it stands as one over gravity, up to
/// [`MAX_RELIEF_SCALE`] (Weisskopf 1975; Plescia 2004). Ported from
/// `legacy-final:crates/mapgen/src/generation.rs:407-420`.
pub fn relief_scale(gravity: MetrePerSecondSquared) -> f64 {
    (STANDARD_GRAVITY.0 / gravity.0).min(MAX_RELIEF_SCALE)
}

#[cfg(test)]
#[path = "spectrum_tests.rs"]
mod tests;
