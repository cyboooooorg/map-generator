//! The thresholds of the planet classes: the hot giants' period, the
//! super-Earth's radius, the ocean world's basins, the runaway's dense air
//! and the ice world's surface water. The class itself is read in
//! `system::planet`, off the chain's result.
//!
//! Values of study
//! [27](https://github.com/cyboooooorg/map-generator/wiki/27-life-and-class-audit),
//! decided on 2026-11-24. Pages: `docs/science/planet-interiors.md`,
//! "Classes", and `docs/science/planet-atmospheres.md`, "The chain and the
//! surface classes".

use crate::constants::STANDARD_GRAVITY;
use crate::units::{Bar, Day, EarthRadius, MetrePerSecondSquared};

/// Source: a hot Jupiter has "P < 10 d" (Wright et al. 2012, *ApJ* 753,
/// 160, doi:10.1088/0004-637X/753/2/160, § 3); Dawson & Johnson 2018
/// ("Origins of hot Jupiters") use the same line. A hot giant, Jupiter or
/// Neptune, has a period of at most 10 days; the families draw their hot
/// Jupiters within it too. Page: `docs/science/planet-interiors.md`,
/// "Classes".
pub const HOT_GIANT_MAX_PERIOD: Day = Day(10.0);

/// Source: "Earth-size (Rp < 1.25 R⊕)" (Borucki et al. 2011, *ApJ* 736,
/// 19, doi:10.1088/0004-637X/736/1/19): a rocky planet of 1.25 R⊕ or more
/// is a super-Earth. Page: `docs/science/planet-interiors.md`, "Classes".
pub const SUPER_EARTH_MIN_RADIUS: EarthRadius = EarthRadius(1.25);

/// Source: an Earth-size planet's basins hold at most "5.2x10^18 m^3, or
/// a mass of 3.7 M_o" Earth oceans (Cowan & Abbot 2014, *ApJ* 781, 27,
/// doi:10.1088/0004-637X/781/1/27, § 2.4): past that no land shows. Page:
/// `docs/science/planet-atmospheres.md`, "The ocean world".
pub const BASIN_OCEANS: f64 = 3.7;

/// The Earth oceans a body of `radius` and surface `gravity` holds in its
/// basins before all its land drowns: `BASIN_OCEANS · (R/R⊕)² · (g⊕/g)`
/// (Cowan & Abbot 2014: the basins' depth scales as `1/g`, Eq. 11, and
/// their area as `R²`). `g⊕` is [`STANDARD_GRAVITY`], which stands for
/// Earth's surface gravity (a `Definition:`). Takes a positive gravity.
pub fn basin_capacity_oceans(radius: EarthRadius, gravity: MetrePerSecondSquared) -> f64 {
    BASIN_OCEANS * radius.0 * radius.0 * (STANDARD_GRAVITY.0 / gravity.0)
}

/// Calibration: the air over which a planet in runaway is a runaway
/// greenhouse rather than a bare rock, 1 bar (Venus has 92; a runaway
/// that lost its air is a bare rock). Searched and not found: a pressure
/// that parts a runaway greenhouse from a bare rock; the published
/// criteria are the absorbed light (Goldblatt et al. 2013,
/// doi:10.1038/ngeo1892) or the distance to the star (Hamano et al.
/// 2013). Kept from legacy (study 27). Page:
/// `docs/science/planet-atmospheres.md`, "The dense air".
pub const DENSE_AIR_BAR: Bar = Bar(1.0);

/// Calibration: the water kept at the surface, in Earth oceans, from which
/// a body of at least `system::planet::heat::ICY_MIN_WATER` water and ices is an ice
/// world, 0.01 (issue #452): a little over twice Mars's 0.004, its polar
/// deposits and ground ice of about 34 m of global layer (Carr & Head
/// 2015, doi:10.1002/2014GL062464, not reached: the figure as the page
/// quotes it). Searched and not found: a surface water that makes an ice
/// world (study 27). Page: `docs/science/planet-interiors.md`, "Classes".
pub const ICE_WORLD_MIN_OCEANS: f64 = 0.01;

#[cfg(test)]
#[path = "classes_tests.rs"]
mod tests;
