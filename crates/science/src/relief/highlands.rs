//! The relief of a world without plates: its highland provinces, the
//! plains of a dry world, and the height of the noise mountains.
//!
//! Ported from `legacy-final:crates/mapgen/src/generation.rs` (the
//! `HIGHLANDS` and `DRY_PLAINS_KM` constants); values and their calibration
//! list: `docs/superpowers/studies/2026-11-22-relief/README.md`, numbers 13
//! and 14; [`MOUNTAIN_RELIEF`] is not in the list yet.

use super::plates::COLLISION_HEIGHT;
use crate::units::Metre;

/// The noise that places the highland provinces of a world without plates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Highlands {
    /// Frequency of the first octave on the unit sphere.
    pub frequency: f64,
    /// Number of octaves.
    pub octaves: u32,
    /// Values of the noise between which the mountains ramp from none to
    /// all their weight.
    pub edge: (f64, f64),
}

/// Calibration: the highland provinces of a world without plates (stagnant,
/// episodic or heat-pipe lids), the only places its noise mountains rise:
/// a three-octave noise at frequency 1.6 on the unit sphere, a few provinces
/// whatever the world's size, ramping the mountains in between 0.23 and
/// 0.33. Fitted, in the legacy, over 40 seeds: the noise passes the middle
/// of the ramp (0.28) on about 9 % of the sphere, Venus's highland share
/// (tesserae about 7-8 % and the highlands with them about a tenth, Ivanov
/// & Head 2011, doi:10.1016/j.pss.2011.07.008). Searched and not found: a
/// law of the provinces of a world without plates (relief study, calibration
/// 13, `docs/superpowers/studies/2026-11-22-relief/README.md`).
pub const HIGHLANDS: Highlands = Highlands {
    frequency: 1.6,
    octaves: 3,
    edge: (0.23, 0.33),
};

/// Share of the mountains' weight at a value `noise` of the highlands'
/// noise: a smooth step from 0 at `edge.0` to 1 at `edge.1`
/// (see [`HIGHLANDS`]).
pub fn highland_weight(noise: f64) -> f64 {
    let (low, high) = HIGHLANDS.edge;
    let t = ((noise - low) / (high - low)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Calibration: relief of a dry world's plains, 3 km per unit of the
/// legacy's continents noise (five Perlin octaves of gain one half divided
/// by their total amplitude, 1.9375; the world converts it to its own
/// unit-variance noise) at Earth's gravity, scaled as 1/g
/// ([`super::spectrum::relief_scale`]). With no sea to level them the
/// plains of a dry world roll: Venus's lowlands and upland plains spread
/// its unimodal hypsometry over about +-1 km, more than 80 % of the surface
/// within 1 km of the mean radius (Ford & Pettengill 1992,
/// doi:10.1029/92JE01085). Fitted in the legacy on its `venus-like`
/// archetype (seed 12345, 2048 x 1024): 3 km puts 79 % of its area within
/// 1 km of its median, 2 km 85 % (relief study, calibration 14).
pub const DRY_PLAINS: Metre = Metre(3000.0);

/// Source: the height of the highest summits above the plateau of their
/// orogen, per unit of the ridged noise at weight 1 and Earth's gravity:
/// Everest's 8848.86 m (China and Nepal survey, 2020; the study's value
/// of `MAX_RELIEF_SCALE`) less the Tibetan plateau's mean height,
/// [`COLLISION_HEIGHT`] (5 km, Royden, Burchfiel & van der Hilst 2008,
/// doi:10.1126/science.1155371): 8848.86 - 5000 = 3848.86 m.
pub const MOUNTAIN_RELIEF: Metre = Metre(EVEREST_HEIGHT.0 - COLLISION_HEIGHT.0);

/// Source: the height of Everest above sea level, 8848.86 m (China and
/// Nepal survey, 2020), the value [`super::spectrum::MAX_RELIEF_SCALE`]
/// is derived from.
const EVEREST_HEIGHT: Metre = Metre(8848.86);

#[cfg(test)]
#[path = "highlands_tests.rs"]
mod tests;
