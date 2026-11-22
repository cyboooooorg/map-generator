//! The spectrum of a rocky relief: Hurst exponent, octaves, amplitude, and
//! the scaling of relief with gravity.

use crate::constants::{MARS_SURFACE_GRAVITY, STANDARD_GRAVITY};
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

/// The two provinces of a crust that floats at two thicknesses
/// ([`super::crust::Crust`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Province {
    /// The thick crust: continents, or highlands.
    Thick,
    /// The thin crust: ocean floor, or lowlands.
    Thin,
}

/// Source: mean Haar fluctuation (the mean of `|M2 − M1|`, the two
/// half-window means of a north-south profile) of the heights at a lag of
/// [`REFERENCE_WAVELENGTH`], within one province, on the analogue body at
/// its own gravity. Earth, measured on ETOPO 2022 in windows wholly on land
/// (the thick crust) and wholly on the sea floor deeper than 200 m (the
/// thin crust), interpolated log-log to 100 km between 89 and 111 km: land
/// 131 m (124 and 138 m), sea floor 246 m (236 and 255 m). Mars, the whole
/// surface of the MOLA MEGDR, 233 m (Landais et al. 2019, Fig. 2, about
/// 231 m), for both provinces. The study's `scripts/etopo_haar_split.py`
/// and `scripts/mola_stats.py`,
/// `docs/superpowers/studies/2026-11-22-relief/topography.md`. The noise is
/// scaled to this statistic, measured on the noise itself.
pub fn amplitude(analogue: Analogue, province: Province) -> Metre {
    match (analogue, province) {
        (Analogue::Earth, Province::Thick) => Metre(131.0),
        (Analogue::Earth, Province::Thin) => Metre(246.0),
        (Analogue::Mars, _) => Metre(233.0),
    }
}

/// The surface gravity at which the analogue's relief was measured.
/// Definition: Earth's is [`STANDARD_GRAVITY`], the gravity every
/// Earth-measured height is referred to (its relief scale is then 1).
/// Mars's is [`MARS_SURFACE_GRAVITY`], its mean surface gravity.
pub fn analogue_gravity(analogue: Analogue) -> MetrePerSecondSquared {
    match analogue {
        Analogue::Earth => STANDARD_GRAVITY,
        Analogue::Mars => MARS_SURFACE_GRAVITY,
    }
}

/// Factor on a height measured on the analogue body, for a body of surface
/// gravity `gravity`: `relief_scale(g) / relief_scale(g_analogue)`, so
/// that gravity is counted once. 1 at the analogue's own gravity; for
/// Earth, [`relief_scale`] itself.
pub fn analogue_relief_scale(gravity: MetrePerSecondSquared, analogue: Analogue) -> f64 {
    relief_scale(gravity) / relief_scale(analogue_gravity(analogue))
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
