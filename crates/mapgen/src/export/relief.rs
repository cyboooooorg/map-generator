//! Relief normals: the orientation of the ground, for the viewer to light.
//!
//! The viewer lights the relief itself, by the sun or its headlight, from
//! a normal per pixel in the tangent frame of the map: east, north and up.
//! The slopes are those of the hillshade the flat images keep (see
//! [`super::hillshade`]): central differences of the elevation, the
//! longitude wrapping and the latitude clamped at the poles, at the same
//! exaggeration ([`super::relief_km`]).
//!
//! A normal is stored in 8 bits per channel, `(n + 1) / 2 · 255`: up is
//! `(128, 128, 255)`.

use crate::geo::Grid;
use crate::world::World;

/// The encoded normal of flat ground (and of water).
pub const FLAT: [u8; 3] = [128, 128, 255];

/// Slopes of the exaggerated elevation at pixel `(x, y)`, rise over run:
/// towards the east and towards the north.
pub(crate) fn slopes(world: &World, grid: &Grid, x: u32, y: u32) -> (f32, f32) {
    let (w, h) = (world.width, world.height);
    let relief = super::relief_km(world.params.circumference_km);
    let elev = |x: u32, y: u32| world.elevation[world.index(x, y)] * relief;
    let (west, east) = ((x + w - 1) % w, (x + 1) % w);
    let (north, south) = (y.saturating_sub(1), (y + 1).min(h - 1));
    let dz_east = (elev(east, y) - elev(west, y)) / (2.0 * grid.dx_km(y));
    let dz_north =
        (elev(x, north) - elev(x, south)) / ((south - north).max(1) as f32 * grid.dy_km());
    (dz_east, dz_north)
}

/// The unit normal (east, north, up) of the ground at pixel `(x, y)`.
pub fn relief_normal(world: &World, grid: &Grid, x: u32, y: u32) -> [f32; 3] {
    let (dz_east, dz_north) = slopes(world, grid, x, y);
    normal_of(dz_east, dz_north)
}

/// The unit normal of ground with these slopes (see [`slopes`]).
pub(crate) fn normal_of(dz_east: f32, dz_north: f32) -> [f32; 3] {
    normalise([-dz_east, -dz_north, 1.0])
}

fn normalise(n: [f32; 3]) -> [f32; 3] {
    let len = n.iter().map(|c| c * c).sum::<f32>().sqrt();
    n.map(|c| c / len)
}

/// A unit normal with its slopes scaled by `share` (0: flat, 1: as is),
/// for surfaces whose relief shows less (lava).
pub fn soften(n: [f32; 3], share: f32) -> [f32; 3] {
    if share == 1.0 {
        return n;
    }
    // Back to slopes (rise over run), scaled, then normalised again.
    let up = n[2].max(1e-6);
    normalise([n[0] / up * share, n[1] / up * share, 1.0])
}

/// A normal in 8 bits per channel.
pub fn encode_normal(n: [f32; 3]) -> [u8; 3] {
    n.map(|c| ((c.clamp(-1.0, 1.0) + 1.0) * 0.5 * 255.0).round() as u8)
}

/// The unit normal an encoded one stands for.
pub fn decode_normal(c: [u8; 3]) -> [f32; 3] {
    normalise(c.map(|v| f32::from(v) / 255.0 * 2.0 - 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world_with(elevation: impl Fn(u32, u32) -> f32) -> World {
        let p = crate::generation::tests::params(3, "terran");
        let mut world = crate::generation::generate(&p).world;
        for y in 0..world.height {
            for x in 0..world.width {
                let i = world.index(x, y);
                world.elevation[i] = elevation(x, y);
            }
        }
        world
    }

    fn length(n: [f32; 3]) -> f32 {
        n.iter().map(|c| c * c).sum::<f32>().sqrt()
    }

    #[test]
    fn flat_ground_faces_up() {
        let world = world_with(|_, _| 0.3);
        let grid = world.params.grid();
        for (x, y) in [(0, 0), (10, 5), (63, 31), (32, 16)] {
            assert_eq!(encode_normal(relief_normal(&world, &grid, x, y)), FLAT);
        }
    }

    #[test]
    fn a_slope_rising_east_tilts_the_normal_west() {
        // 0.05 elevation units a pixel: 75 km of exaggerated rise over the
        // 626 km of a pixel of this 64-pixel Earth, a slope of 0.12.
        let world = world_with(|x, _| 0.05 * x as f32);
        let grid = world.params.grid();
        let n = relief_normal(&world, &grid, 40, 16);
        assert!(n[0] < -0.05, "{n:?}");
        assert!(n[1].abs() < 0.01, "{n:?}");
        assert!(n[2] > 0.0, "{n:?}");
        assert!((length(n) - 1.0).abs() < 1e-3, "{n:?}");
    }

    #[test]
    fn the_longitude_wraps() {
        let w = 64.0;
        let world = world_with(|x, _| 0.002 * (std::f32::consts::TAU * x as f32 / w).sin());
        let grid = world.params.grid();
        // sin is steepest upwards at x = 0 and downwards at x = w / 2: the
        // seam column sees its wrapped neighbour, not a cliff.
        let seam = relief_normal(&world, &grid, 0, 16);
        let half = relief_normal(&world, &grid, 32, 16);
        assert!(seam[0] < 0.0 && half[0] > 0.0, "{seam:?} {half:?}");
        assert!((seam[0] + half[0]).abs() < 1e-3, "{seam:?} {half:?}");
    }

    #[test]
    fn polar_rows_stay_bounded() {
        let world = world_with(|x, y| 0.01 * ((x * 7 + y * 13) % 11) as f32);
        let grid = world.params.grid();
        for y in [0, world.height - 1] {
            for x in 0..world.width {
                let n = relief_normal(&world, &grid, x, y);
                assert!(n[2] >= 0.05, "row {y} column {x}: {n:?}");
            }
        }
    }

    #[test]
    fn softening_scales_the_slopes() {
        let n = [-0.6, 0.0, 0.8];
        assert_eq!(soften(n, 1.0), n);
        let flat = soften(n, 0.0);
        assert!(flat[0].abs() < 1e-6 && (flat[2] - 1.0).abs() < 1e-6);
        let half = soften(n, 0.4);
        assert!(half[0] > n[0] && half[0] < 0.0 && (length(half) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn encoding_round_trips() {
        assert_eq!(encode_normal([0.0, 0.0, 1.0]), FLAT);
        for n in [[0.6, 0.0, 0.8], [-0.36, 0.48, 0.8], [0.0, -1.0, 0.0]] {
            let back = decode_normal(encode_normal(n));
            for k in 0..3 {
                assert!((back[k] - n[k]).abs() < 0.01, "{n:?} -> {back:?}");
            }
        }
    }
}
