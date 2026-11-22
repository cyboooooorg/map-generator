//! Plate boundaries and the relief they build: trenches, arcs, ranges,
//! rifts and mid-ocean ridges, as profiles across the boundary.
//!
//! Values and their citations: the relief study,
//! `docs/superpowers/studies/2026-11-22-relief/plates.md`, and its
//! calibration list (`README.md#final-calibration-list`). Ported from
//! `legacy-final:crates/mapgen/src/plates.rs` (`relief_of`, 751), whose
//! profiles are the bumps `exp(-((d - mu) / sigma)²)` used here; such a
//! bump is `2 √ln 2 ≈ 1.665` sigma wide at half its height.

use crate::constants::{CENTIMETRE_PER_YEAR, DEGREES_PER_HALF_TURN};
use crate::units::{Fraction, Gyr, Metre, MetrePerSecond, Radian, Second, Year};

/// What a boundary is, from how its plates move (closing or parting) and
/// the crust on each side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Boundary {
    /// Two continents closing: a collision range (Tibet).
    Collision,
    /// Oceanic crust sinking under a continent: a trench, an arc and a
    /// plateau behind it (the Andes).
    Andean,
    /// Oceanic crust sinking under oceanic crust: a trench and an island
    /// arc.
    IslandArc,
    /// Two continents parting: a rift.
    Rift,
    /// Two oceanic plates parting: a mid-ocean ridge.
    Ridge,
    /// Plates sliding past each other, or a continent parting from an
    /// ocean: no relief.
    Transform,
}

/// Source: number of plates, from Morra et al.'s (2013,
/// doi:10.1016/j.epsl.2013.04.020) 8 large plates
/// that "cover most of the surface of the Earth" to Bird's (2003,
/// doi:10.1029/2001GC000252) 14 large plates of PB2002. No source scales
/// the count with the planet's size.
pub const PLATE_COUNT: (u32, u32) = (8, 14);

/// Source: exponent of the complementary cumulative distribution of plate
/// areas, µ = 0.25 ± 0.05 on PB2001 (Sornette & Pisarenko 2003,
/// doi:10.1029/2002GL015043); the target the plate draw is fitted to.
pub const PLATE_SIZE_EXPONENT: f64 = 0.25;

/// Source: median RMS speeds over a plate, 2.6 cm/yr for plates with no
/// subducting edge and 7.9 cm/yr for those subducting along more than 25 %
/// of their perimeter (Zahirovic, Müller, Seton & Flament 2015, section
/// 3.5, Fig. 5A, doi:10.1016/j.epsl.2015.02.037). RMS speeds over the plate's
/// area, not the equatorial speed of its Euler pole: the plate map
/// converts them through each plate's geometry.
pub const PLATE_RMS_SPEED: (MetrePerSecond, MetrePerSecond) = (
    MetrePerSecond(2.6 * CENTIMETRE_PER_YEAR.0),
    MetrePerSecond(7.9 * CENTIMETRE_PER_YEAR.0),
);

/// Source: share of its perimeter a plate subducts along to reach the top
/// of [`PLATE_RMS_SPEED`], the class boundary of Zahirovic et al. 2015.
/// An `f64`, not a `Fraction`, to stay a plain constant.
pub const SLAB_FULL_SHARE: f64 = 0.25;

/// Calibration: largest weight of a plate centre, 0.35 rad: a plate of
/// weight `w` reaches `w` farther than an unweighted one (a power
/// diagram), so plates come in varied sizes. The legacy's
/// (`legacy-final:crates/mapgen/src/plates.rs:49`), to fit on Sornette &
/// Pisarenko's 0.25 exponent ([`PLATE_SIZE_EXPONENT`]) and PB2002's areas,
/// not yet fitted; searched and not found: a
/// source for the spread device itself (relief study, calibration 11,
/// `docs/superpowers/studies/2026-11-22-relief/plates.md#plate-count-and-sizes`).
pub const PLATE_WEIGHT: Radian = Radian(0.35);

/// Definition: frequency of the subplate lattice on the unit sphere, 5
/// cells per unit, about 300 subplates whose jagged outlines the plates
/// follow: a discretisation of the plate outlines, the legacy's
/// (`legacy-final:crates/mapgen/src/plates.rs:40`; relief study,
/// plates.md, plate count and sizes).
pub const SUBPLATE_FREQUENCY: f64 = 5.0;

/// Definition: displacement of the subplate lookup on the unit sphere,
/// 0.08, so subplate borders wander: part of the discretisation of the
/// plate outlines, the legacy's (`legacy-final:crates/mapgen/src/plates.rs:44`;
/// relief study, plates.md).
pub const SUBPLATE_WARP: f64 = 0.08;

/// Definition: frequency of the first octave of the subplate warp on the
/// unit sphere, 3; the second is twice as fine and half as strong. Part
/// of the discretisation of the plate outlines, the legacy's
/// (`legacy-final:crates/mapgen/src/plates.rs:45`; relief study, plates.md).
pub const SUBPLATE_WARP_FREQUENCY: f64 = 3.0;

/// Definition: half-width, in grid cells, of the window the direction
/// across a boundary is taken over: one neighbour gives only one of 8
/// directions, which jump from cell to cell along a stepped boundary. A
/// numerical device, the legacy's (`legacy-final:crates/mapgen/src/plates.rs:58`;
/// relief study, plates.md).
pub const BOUNDARY_WINDOW: u32 = 3;

/// Source: depth of the top of the slab under the volcanic arc, 105 km,
/// the global average of 72-173 km over 500 km arc segments (Syracuse &
/// Abers 2006, doi:10.1029/2005GC001045).
pub const ARC_SLAB_DEPTH: Metre = Metre(105e3);

/// Source: mean dip of the slab between 0 and 125 km depth, 32° ± 11° over
/// 159 transects (Lallemand, Heuret & Boutelier 2005, section 4,
/// doi:10.1029/2005GC000917).
pub const SLAB_DIP: Radian = Radian(32.0 * std::f64::consts::PI / DEGREES_PER_HALF_TURN);

/// Definition: depth of a trench below the plate around it, 2.8 km, the
/// middle of the 0.9-4.7 km of Zhang et al.'s (2018) flexural fits to the
/// global array of trenches (W0, Table 3, doi:10.1093/gji/ggx488),
/// as the relief study retains it.
pub const TRENCH_DEPTH: Metre = Metre(2.8e3);

/// Definition: width of a trench, 62 km, the middle of Zhang et al.'s
/// (2018) 31-93 km (X0, from the axis to where the flexure crosses zero).
/// A bump never crosses zero: this is its full width at half depth, so the
/// trench is half as deep 31 km from its axis and nearly flat (2 %) 75 km
/// out.
pub const TRENCH_WIDTH: Metre = Metre(62e3);

/// Calibration: height of an island arc at its crest, 3 km, the legacy's
/// (`0.12` of its 25 km elevation unit), now centred on the arc
/// ([`arc_distance`]) instead of the legacy's 120 km. Searched and not
/// found: arc edifice and forearc height statistics (relief study,
/// calibration 7,
/// `docs/superpowers/studies/2026-11-22-relief/plates.md#island-arcs`).
pub const ISLAND_ARC_UPLIFT: Metre = Metre(3e3);

/// Calibration: sigma of the island arc's bump, 60 km (full width at half
/// height 100 km), the legacy's; also the sigma of the band arc volcanoes
/// stand in, on both kinds of arc (the legacy's Andean 80 km dropped).
/// Searched and not found as [`ISLAND_ARC_UPLIFT`] (relief study,
/// calibration 7).
pub const ISLAND_ARC_SIGMA: Metre = Metre(60e3);

/// Source: mean elevation of the Altiplano-Puna, 3.65 km between 13 and
/// 29°S (Allmendinger, Jordan, Kay & Isacks 1997, p. 142, after Isacks
/// 1988, doi:10.1146/annurev.earth.25.1.139).
pub const ANDEAN_UPLIFT: Metre = Metre(3.65e3);

/// Source: width of the Altiplano-Puna, 375 km, the middle of
/// Allmendinger et al.'s (1997) 350-400 km; the full width of the
/// plateau's bump at half its height. The plateau starts at the arc and
/// spans this width inland ([`andean_centre`]).
pub const ANDEAN_WIDTH: Metre = Metre(375e3);

/// Source: height of a collision range, 5 km, the Tibetan plateau's
/// (Royden, Burchfiel & van der Hilst 2008, p. 1054,
/// doi:10.1126/science.1155371): an upper bound for an orogen.
pub const COLLISION_HEIGHT: Metre = Metre(5e3);

/// Calibration: sigma of the collision range's bump, 350 km (full width
/// at half height 583 km), the legacy's. Bounded by the Altiplano's
/// 350-400 km and Tibet's roughly 1000 km; searched and not found: a
/// published width of collision orogens (relief study, calibration 8,
/// `docs/superpowers/studies/2026-11-22-relief/plates.md#orogens`).
pub const COLLISION_SIGMA: Metre = Metre(350e3);

/// Source: width of a narrow continental rift, under 100 km (Hopper & Buck
/// 1996, doi:10.1029/96JB01644; Buck 2007, "on the order of 100 km"): the
/// full width of the axis trough's bump at half its depth, at the bound
/// (sigma 60 km, the legacy's). With the shoulders, the trough is
/// narrower still.
pub const RIFT_WIDTH: Metre = Metre(100e3);

/// Calibration: depth of a rift's axis, 1.5 km, the legacy's. Bounded by
/// the 3 km Buck (2007) calls the high end of rift relief; searched and
/// not found: axis depth statistics (relief study, calibration 9,
/// `docs/superpowers/studies/2026-11-22-relief/plates.md#rifts`).
pub const RIFT_AXIS_DEPTH: Metre = Metre(1.5e3);

/// Calibration: height of a rift's shoulders, 0.5 km, the legacy's.
/// Bounded by Buck's (2007) 3 km; searched and not found: typical
/// shoulder uplift (relief study, calibration 9).
pub const RIFT_SHOULDER_HEIGHT: Metre = Metre(0.5e3);

/// Calibration: distance of a rift's shoulders from its axis, 120 km, the
/// legacy's (relief study, calibration 9, as [`RIFT_SHOULDER_HEIGHT`]).
pub const RIFT_SHOULDER_DISTANCE: Metre = Metre(120e3);

/// Calibration: sigma of a rift shoulder's bump, 60 km, the legacy's
/// (relief study, calibration 9, as [`RIFT_SHOULDER_HEIGHT`]).
pub const RIFT_SHOULDER_SIGMA: Metre = Metre(60e3);

/// Calibration: convergence over which boundary relief grows from nothing
/// to its full height, 0.5 to 5 cm/yr, the legacy's; parting plates open
/// rifts over the same speeds, and ridges fade in under the first.
/// Searched and not found: a convergence threshold for trenches, orogens
/// or arcs (Jarrard 1986 and Heuret & Lallemand 2005 hold the data, not a
/// threshold; relief study, calibration 10,
/// `docs/superpowers/studies/2026-11-22-relief/plates.md#convergence-thresholds`).
pub const RELIEF_SPEED: (MetrePerSecond, MetrePerSecond) = (
    MetrePerSecond(0.5 * CENTIMETRE_PER_YEAR.0),
    MetrePerSecond(5.0 * CENTIMETRE_PER_YEAR.0),
);

/// Calibration: convergence from which a volcanic arc is at full strength,
/// 2 cm/yr, the legacy's: arcs need subduction, hardly its speed. Searched
/// and not found as [`RELIEF_SPEED`] (relief study, calibration 10).
pub const ARC_SPEED: MetrePerSecond = MetrePerSecond(2.0 * CENTIMETRE_PER_YEAR.0);

/// Source: depth of the ocean floor at a ridge crest, 2.5 km:
/// `d(t) = 2500 + 350 t^½ m`, `t` in Ma, between 0 and 70 Ma (Parsons &
/// Sclater 1977, doi:10.1029/JB082i005p00803).
pub const RIDGE_CREST_DEPTH: Metre = Metre(2.5e3);

/// Source: subsidence of the ocean floor with the square root of its age,
/// 350 m per √Ma (Parsons & Sclater 1977, as [`RIDGE_CREST_DEPTH`]).
pub const RIDGE_SUBSIDENCE: f64 = 350.0;

/// Source: the age up to which Parsons & Sclater's (1977) relation holds,
/// 70 Ma; floor this old is the datum ridges stand above (5.43 km deep,
/// 2.93 km below the crest). The legacy's datum, its generator's 4 km
/// median floor (18 Ma), had no source.
pub const RIDGE_FLOOR_AGE: Gyr = Gyr(0.07);

/// Definition: millions of years in a billion (SI prefixes).
const MA_PER_GYR: f64 = 1e3;

/// Depth of the ocean floor of age `age`: `2.5 km + 0.35 km × √(t / Ma)`
/// (Parsons & Sclater 1977). Ported from
/// `legacy-final:crates/mapgen/src/plates.rs:485-491`.
pub fn ridge_depth(age: Gyr) -> Metre {
    let ma = age.0.max(0.0) * MA_PER_GYR;
    RIDGE_CREST_DEPTH + Metre(RIDGE_SUBSIDENCE * ma.sqrt())
}

/// RMS surface speed of a plate subducting along `share` of its
/// perimeter (slab pull): the bottom of [`PLATE_RMS_SPEED`] with no
/// sinking edge, its top from [`SLAB_FULL_SHARE`] on.
///
/// Calibration: the rise between 0 and [`SLAB_FULL_SHARE`] is linear, the
/// legacy's ramp (`legacy-final:crates/mapgen/src/plates.rs:1021`, its
/// `sinking_speeds`). Zahirovic et al. (2015) give classes, not a law; their
/// 4.4 cm/yr median under 25 % subducting is its check point. Searched
/// and not found: a continuous law of speed against subducting share
/// (relief study, calibration 12,
/// `docs/superpowers/studies/2026-11-22-relief/plates.md#plate-speeds-and-slab-pull`).
pub fn slab_pull_speed(share: f64) -> MetrePerSecond {
    let (slow, fast) = PLATE_RMS_SPEED;
    let ramp = (share / SLAB_FULL_SHARE).clamp(0.0, 1.0);
    MetrePerSecond(slow.0 + (fast.0 - slow.0) * ramp)
}

/// Distance of the volcanic arc from the trench, where the slab is
/// [`ARC_SLAB_DEPTH`] deep: `depth / tan(dip)` (England & Katz 2010,
/// Fig. 1; Jarrard 1986), 168 km.
pub fn arc_distance() -> Metre {
    Metre(ARC_SLAB_DEPTH.0 / SLAB_DIP.0.tan())
}

/// Distance of the Andean plateau's middle from the trench: the arc plus
/// half [`ANDEAN_WIDTH`], so the plateau is half its height at the arc,
/// as the Altiplano lies behind the arc of the Western Cordillera.
pub fn andean_centre() -> Metre {
    arc_distance() + ANDEAN_WIDTH / 2.0
}

/// Relief a boundary of kind `kind` adds at `distance` from it, at Earth's
/// gravity (the world scales it by the relief scale). `distance` is
/// positive on the overriding plate of a subduction (the continent of an
/// Andean margin), negative on the sinking one; the other kinds are
/// symmetric. `convergence` is the speed at which the plates close,
/// negative where they part.
///
/// - Trench (Andean, island arc): [`TRENCH_DEPTH`] at the boundary, the
///   trench's axis, [`TRENCH_WIDTH`] wide.
/// - Island arc: [`ISLAND_ARC_UPLIFT`] over the arc ([`arc_distance`]).
/// - Andean: [`ANDEAN_UPLIFT`] over [`ANDEAN_WIDTH`] behind the arc.
/// - Collision: [`COLLISION_HEIGHT`] over the boundary.
/// - Rift: the axis [`RIFT_AXIS_DEPTH`] down, [`RIFT_WIDTH`] wide, between
///   shoulders [`RIFT_SHOULDER_HEIGHT`] up.
/// - Ridge: the young floor above the floor [`RIDGE_FLOOR_AGE`] old, its
///   age the distance over the speed of one plate (half the spreading).
///
/// Closing relief grows over [`RELIEF_SPEED`]; ridges fade in under its
/// first value. Ported from `legacy-final:crates/mapgen/src/plates.rs`
/// (`relief_of`, 751).
pub fn boundary_height(kind: Boundary, distance: Metre, convergence: MetrePerSecond) -> Metre {
    let d = distance.0;
    let closing = closing(convergence.0);
    let trench = TRENCH_DEPTH * (closing * bump(d, 0.0, sigma(TRENCH_WIDTH)));
    match kind {
        Boundary::Collision => COLLISION_HEIGHT * orogeny_weight(kind, distance, convergence).get(),
        Boundary::Andean => {
            ANDEAN_UPLIFT * orogeny_weight(kind, distance, convergence).get() - trench
        }
        Boundary::IslandArc => {
            let arc = bump(d, arc_distance().0, ISLAND_ARC_SIGMA.0);
            ISLAND_ARC_UPLIFT * (closing * arc) - trench
        }
        Boundary::Rift => {
            let shoulder = bump(d.abs(), RIFT_SHOULDER_DISTANCE.0, RIFT_SHOULDER_SIGMA.0);
            let axis = bump(d, 0.0, sigma(RIFT_WIDTH));
            let parting = smooth(RELIEF_SPEED.0.0, RELIEF_SPEED.1.0, -convergence.0);
            (RIFT_SHOULDER_HEIGHT * shoulder - RIFT_AXIS_DEPTH * axis) * parting
        }
        Boundary::Ridge => ridge_height(d.abs(), -convergence.0),
        Boundary::Transform => Metre(0.0),
    }
}

/// Weight of the volcanic arc at `distance` from a boundary of kind
/// `kind`: full over the arc ([`arc_distance`]) of an Andean margin or an
/// island arc once the plates close at [`ARC_SPEED`], fading over
/// [`ISLAND_ARC_SIGMA`]; zero elsewhere. Arc cones stand where it is
/// strong.
pub fn arc_weight(kind: Boundary, distance: Metre, convergence: MetrePerSecond) -> Fraction {
    let weight = match kind {
        Boundary::Andean | Boundary::IslandArc => {
            smooth(RELIEF_SPEED.0.0, ARC_SPEED.0, convergence.0)
                * bump(distance.0, arc_distance().0, ISLAND_ARC_SIGMA.0)
        }
        _ => 0.0,
    };
    Fraction::new(weight).expect("a bump within 0 and 1")
}

/// Weight of mountain ridges at `distance` from a boundary of kind `kind`:
/// the shape of a collision range or of an Andean plateau, scaled by how
/// fast the plates close ([`RELIEF_SPEED`]); zero elsewhere.
pub fn orogeny_weight(kind: Boundary, distance: Metre, convergence: MetrePerSecond) -> Fraction {
    let shape = match kind {
        Boundary::Collision => bump(distance.0, 0.0, COLLISION_SIGMA.0),
        Boundary::Andean => bump(distance.0, andean_centre().0, sigma(ANDEAN_WIDTH)),
        _ => 0.0,
    };
    Fraction::new(closing(convergence.0) * shape).expect("a bump within 0 and 1")
}

/// Definition: number of levels the strength of a boundary is cut into.
/// A boundary's relief is the integral over strength levels `t` of the
/// full-strength profile at the distance to the boundary cells at least
/// `t` strong, `∫₀¹ profile(d_{s≥t}) dt`; the world sums `RELIEF_LEVELS`
/// terms of it, so a change of the nearest boundary cell moves the relief
/// by at most `1 / RELIEF_LEVELS` of a peak. A discretisation, not a
/// physical value.
pub const RELIEF_LEVELS: u32 = 4;

/// Definition: how many sigmas past its centre a bump of a boundary
/// profile is cut, 3: the bump is then `e⁻⁹`, a ten-thousandth of its
/// peak. A truncation of the distance pass (the legacy's was a fixed
/// 3000 km, `MAX_BOUNDARY_KM`), not a physical value.
pub const RELIEF_REACH_SIGMAS: f64 = 3.0;

/// Strength of a boundary of kind `kind` whose plates close at
/// `convergence` (negative where they part), in `[0, 1]`: closing kinds
/// over [`RELIEF_SPEED`], rifts and ridges by their spreading over the
/// same speeds, transforms none.
pub fn relief_strength(kind: Boundary, convergence: MetrePerSecond) -> f64 {
    match kind {
        Boundary::Collision | Boundary::Andean | Boundary::IslandArc => closing(convergence.0),
        Boundary::Rift | Boundary::Ridge => closing(-convergence.0),
        Boundary::Transform => 0.0,
    }
}

/// The convergence of a boundary of kind `kind` whose strength
/// ([`relief_strength`]) is `strength`: the inverse of the smooth step,
/// negative for parting kinds. The world gives each boundary cell the
/// level nearest its strength (rounded, not floored, which halves the
/// error) and builds each level's relief at this convergence.
pub fn relief_level(kind: Boundary, strength: f64) -> MetrePerSecond {
    let y = strength.clamp(0.0, 1.0);
    // Inverse of 3t² - 2t³ on [0, 1].
    let t = 0.5 - ((1.0 - 2.0 * y).asin() / 3.0).sin();
    let speed = RELIEF_SPEED.0.0 + t * (RELIEF_SPEED.1.0 - RELIEF_SPEED.0.0);
    match kind {
        Boundary::Rift | Boundary::Ridge => MetrePerSecond(-speed),
        _ => MetrePerSecond(speed),
    }
}

/// Distance from a boundary of kind `kind` closing at `convergence`
/// beyond which its profile ([`boundary_height`]) and weights are
/// negligible: [`RELIEF_REACH_SIGMAS`] sigmas past the farthest bump, and
/// for a ridge the distance at which the floor is [`RIDGE_FLOOR_AGE`] old.
pub fn relief_reach(kind: Boundary, convergence: MetrePerSecond) -> Metre {
    let n = RELIEF_REACH_SIGMAS;
    match kind {
        Boundary::Collision => COLLISION_SIGMA * n,
        Boundary::Andean => andean_centre() + Metre(n * sigma(ANDEAN_WIDTH)),
        Boundary::IslandArc => arc_distance() + ISLAND_ARC_SIGMA * n,
        Boundary::Rift => RIFT_SHOULDER_DISTANCE + RIFT_SHOULDER_SIGMA * n,
        Boundary::Ridge => {
            let age = Second::from(Year::from(RIDGE_FLOOR_AGE));
            Metre(age.0 * convergence.0.abs() / 2.0)
        }
        Boundary::Transform => Metre(0.0),
    }
}

/// How far closing at `convergence` m/s has built its relief, over
/// [`RELIEF_SPEED`].
fn closing(convergence: f64) -> f64 {
    smooth(RELIEF_SPEED.0.0, RELIEF_SPEED.1.0, convergence)
}

/// Height of the floor `d` metres from a ridge spreading at `spreading`
/// m/s (both plates together) above the floor [`RIDGE_FLOOR_AGE`] old.
fn ridge_height(d: f64, spreading: f64) -> Metre {
    if spreading <= 0.0 {
        return Metre(0.0);
    }
    // Each plate carries the new floor away at half the spreading rate.
    let age = Gyr::from(Year::from(Second(d / (spreading / 2.0))));
    if age > RIDGE_FLOOR_AGE {
        return Metre(0.0);
    }
    let fade = smooth(0.0, RELIEF_SPEED.0.0, spreading);
    (ridge_depth(RIDGE_FLOOR_AGE) - ridge_depth(age)) * fade
}

/// The bump of sigma `sigma` centred at `mu`: `exp(-((d - mu) / sigma)²)`.
fn bump(d: f64, mu: f64, sigma: f64) -> f64 {
    let t = (d - mu) / sigma;
    (-t * t).exp()
}

/// Sigma of a bump `width` wide at half its height: `width / (2 √ln 2)`.
fn sigma(width: Metre) -> f64 {
    width.0 / (2.0 * std::f64::consts::LN_2.sqrt())
}

/// Smooth step from 0 at `edge0` to 1 at `edge1` (Hermite cubic).
fn smooth(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
#[path = "plates_tests.rs"]
mod tests;
