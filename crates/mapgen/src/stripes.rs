//! The fractures a body's geysers vent through ([`crate::planet::geysers`]),
//! cut into its world: troughs flanked by two ridges, as Enceladus's tiger
//! stripes (Porco et al. 2006, doi:10.1126/science.1123013), darker than
//! the plains around them.
use crate::geo::Grid;
use crate::planet::geysers::{FractureFrame, MAX_MEANDER_KM, meander_km, unit_at};
use crate::world::WorldParams;
use rayon::prelude::*;

/// Width of a stripe's trough, km, and of each of its two ridges: the
/// tiger stripes are troughs about 2 km wide and 500 m deep flanked by
/// ridges about 100 m high (Porco et al. 2006,
/// doi:10.1126/science.1123013).
const TROUGH_WIDTH_KM: f64 = 2.0;
/// Depth of the trough, km.
const TROUGH_DEPTH_KM: f64 = 0.5;
/// Height of each ridge, km.
const RIDGE_HEIGHT_KM: f64 = 0.1;
/// Share of the trough's (or a ridge's) width at its full depth (or
/// height); its walls fall off as a cosine over the rest. Look constant:
/// a floor about as wide as a pixel when the width is floored at
/// [`MIN_WIDTH_PIXELS`], so that a pixel next to the fracture's line
/// still sees most of the depth.
const FLAT_SHARE: f64 = 0.7;
/// Share of a fracture's length over which the stripe tapers to nothing
/// at each end. Look constant: no measured profile; it only keeps the
/// ends from looking cut off.
const END_TAPER: f64 = 0.1;
/// Narrowest the trough is drawn, in rows of the grid (`Grid::dy_km`) or
/// of a detail tile: averaged over its pixels, a trough narrower than a
/// pixel would only shade it faintly. Its depths are scaled down to keep
/// the area of its cross-section.
const MIN_WIDTH_PIXELS: f64 = 1.5;
/// Albedo shift over the trough (see [`crate::world::World::albedo`]).
/// Look constant: the tiger stripes read darker than the plains around
/// them in Cassini's clear-filter images (Porco et al. 2006).
pub const STRIPE_ALBEDO: f32 = -0.25;

/// Height, km, of the ground of a stripe `across_km` from its fracture's
/// line (either side) and at `along` its length from its centre (−0.5 and
/// 0.5 at its ends): a trough 500 m deep and 2 km wide between two
/// ridges 100 m high and 2 km wide, tapering to nothing over the last
/// tenth of the length at each end.
pub fn stripe_profile_km(across_km: f64, along: f64) -> f64 {
    profile_km(across_km, along, TROUGH_WIDTH_KM)
}

/// [`stripe_profile_km`] with the trough and ridges `width_km` wide, their
/// depths scaled to keep the area of the cross-section.
fn profile_km(across_km: f64, along: f64, width_km: f64) -> f64 {
    let scale = TROUGH_WIDTH_KM / width_km;
    let a = across_km.abs() / width_km;
    let shape = -TROUGH_DEPTH_KM * bump(a) + RIDGE_HEIGHT_KM * bump(a - 1.0);
    shape * scale * taper(along)
}

/// A bump one unit wide centred on 0: 1 over its flat share, falling as a
/// cosine to 0 at ±0.5.
fn bump(t: f64) -> f64 {
    let flat = FLAT_SHARE / 2.0;
    let t = t.abs();
    if t <= flat {
        1.0
    } else if t >= 0.5 {
        0.0
    } else {
        let u = (t - flat) / (0.5 - flat);
        0.5 * (1.0 + (std::f64::consts::PI * u).cos())
    }
}

/// 1 along most of a fracture, falling smoothly to 0 over the last
/// [`END_TAPER`] of its length at each end.
fn taper(along: f64) -> f64 {
    let t = ((0.5 - along.abs()) / END_TAPER).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Side of the grid of samples a pixel's stripe value is averaged over
/// (see [`Stripes::pixel`]): the mean over its footprint, not the value
/// at its centre, so that a stripe crossing the rows at a slant draws a
/// smooth band instead of a staircase one pixel high per row.
const SUPERSAMPLE: usize = 4;

/// The width the stripes are drawn at on pixels `pixel_km` high, km: their
/// own, floored at `MIN_WIDTH_PIXELS` (1.5) pixels.
pub fn drawn_width_km(pixel_km: f64) -> f64 {
    TROUGH_WIDTH_KM.max(MIN_WIDTH_PIXELS * pixel_km)
}

/// The fractures of a body's geysers, ready to draw their stripes at any
/// resolution: the base map's ([`raster`]) or a detail tile's
/// (`crate::detail`).
pub struct Stripes {
    /// Each fracture's index (zero-length ones counted), length, km, and
    /// frame, for its meander and its offsets.
    frames: Vec<(usize, f64, FractureFrame)>,
    radius_km: f64,
    /// The pole the geysers vent from.
    pub pole: [f64; 3],
    /// Angle, radians, from the pole to the farthest end of a fracture.
    ends_rad: f64,
}

impl Stripes {
    /// The stripes of the geysers of `p`, `None` without geysers.
    pub fn of(p: &WorldParams) -> Option<Self> {
        let geysers = p.model.as_ref()?.geysers.as_ref()?;
        let radius_km = f64::from(p.circumference_km) / std::f64::consts::TAU;
        let pole = unit_at(if geysers.south { -90.0 } else { 90.0 }, 0.0);
        // The farthest point of a fracture from the pole is one of its ends.
        let ends_rad = geysers
            .fractures
            .iter()
            .flat_map(|f| {
                let (a, b) = f.ends(radius_km);
                [a, b]
            })
            .map(|e| angle(pole, e))
            .fold(0.0, f64::max);
        let frames = geysers
            .fractures
            .iter()
            .enumerate()
            .filter_map(|(i, f)| Some((i, f64::from(f.length_km), f.frame(radius_km)?)))
            .collect();
        Some(Self {
            frames,
            radius_km,
            pole,
            ends_rad,
        })
    }

    /// Angle, radians, from the pole beyond which no pixel of footprint
    /// `pixel_km` (its diagonal) is touched by the stripes drawn
    /// `width_km` wide.
    pub fn cap_rad(&self, width_km: f64, pixel_km: f64) -> f64 {
        self.ends_rad + (1.5 * width_km + MAX_MEANDER_KM + pixel_km) / self.radius_km
    }

    /// The stripes drawn `width_km` wide over a pixel, averaged over its
    /// footprint: `at(x, y)` is the point of the unit sphere at the
    /// fractional pixel coordinates `(x, y)` (pixel centres at whole
    /// values), `pixel_km` the pixel's diagonal. Returns the height
    /// change, in elevation units ([`crate::craters::ELEVATION_KM`]), and
    /// the albedo shift; zeros away from the stripes.
    pub fn pixel(
        &self,
        at: impl Fn(f64, f64) -> [f64; 3],
        x: f64,
        y: f64,
        width_km: f64,
        pixel_km: f64,
    ) -> (f32, f32) {
        let reach_km = 1.5 * width_km + MAX_MEANDER_KM;
        let centre = at(x, y);
        let near: Vec<_> = self
            .frames
            .iter()
            .filter(|(_, _, frame)| {
                frame
                    .offset_within_km(centre, reach_km + pixel_km)
                    .is_some()
            })
            .collect();
        if near.is_empty() {
            return (0.0, 0.0);
        }
        let (mut relief_km, mut albedo) = (0.0, 0.0);
        for j in 0..SUPERSAMPLE {
            for i in 0..SUPERSAMPLE {
                let step = |k: usize| (k as f64 + 0.5) / SUPERSAMPLE as f64 - 0.5;
                let v = at(x + step(i), y + step(j));
                for (index, length_km, frame) in &near {
                    let Some((across, along)) = frame.offset_within_km(v, reach_km) else {
                        continue;
                    };
                    let across = across - meander_km(*index, along, *length_km);
                    relief_km += profile_km(across, along, width_km);
                    albedo += bump(across / width_km) * taper(along);
                }
            }
        }
        let n = (SUPERSAMPLE * SUPERSAMPLE) as f64;
        let units = f64::from(crate::craters::ELEVATION_KM);
        (
            (relief_km / n / units) as f32,
            STRIPE_ALBEDO * (albedo / n) as f32,
        )
    }
}

/// Angle, radians, between two unit vectors.
fn angle(a: [f64; 3], b: [f64; 3]) -> f64 {
    (a[0] * b[0] + a[1] * b[1] + a[2] * b[2])
        .clamp(-1.0, 1.0)
        .acos()
}

/// The stripes of the geysers of `p` on `grid` (none without geysers),
/// drawn at [`drawn_width_km`] of its rows and averaged over each pixel's
/// footprint ([`Stripes::pixel`]): `(index, height change, albedo
/// shift)` of every pixel they touch, by index, the height in elevation
/// units ([`crate::craters::ELEVATION_KM`]). Only the rows of the polar
/// cap the fractures lie in are visited. The detail tiles take it off the
/// base layers to draw the stripes at their own resolution.
pub fn raster(grid: &Grid, p: &WorldParams) -> Vec<(usize, f32, f32)> {
    let Some(stripes) = Stripes::of(p) else {
        return Vec::new();
    };
    let width = drawn_width_km(f64::from(grid.dy_km()));
    let w = grid.width as usize;
    (0..grid.height)
        .into_par_iter()
        .filter(|&y| {
            let footprint = f64::from(grid.dx_km(y)).hypot(f64::from(grid.dy_km()));
            let colatitude = angle(stripes.pole, grid.unit_vector_at(0.0, f64::from(y)));
            colatitude <= stripes.cap_rad(width, footprint)
        })
        .flat_map_iter(|y| {
            let stripes = &stripes;
            let footprint = f64::from(grid.dx_km(y)).hypot(f64::from(grid.dy_km()));
            (0..w).filter_map(move |x| {
                let (e, a) = stripes.pixel(
                    |x, y| grid.unit_vector_at(x, y),
                    x as f64,
                    f64::from(y),
                    width,
                    footprint,
                );
                (e != 0.0 || a != 0.0).then_some((y as usize * w + x, e, a))
            })
        })
        .collect()
}

/// Cuts the stripes of the geysers of `p` into `elevation` (a no-op
/// without geysers): [`stripe_profile_km`] near each fracture, averaged
/// over each pixel ([`raster`]), its widths floored at
/// [`MIN_WIDTH_PIXELS`] rows of `grid` and its depths scaled to keep the
/// area of the cross-section; values stay in [−1, 1].
pub fn apply_stripes(elevation: &mut [f32], grid: &Grid, p: &WorldParams) {
    for (i, e, _) in raster(grid, p) {
        elevation[i] = (elevation[i] + e).clamp(-1.0, 1.0);
    }
}

/// Darkens `albedo` by [`STRIPE_ALBEDO`] over the troughs of the stripes
/// of the geysers of `p` (a no-op without geysers), at their width on
/// `grid` (see [`raster`]); values stay in [−1, 1].
pub fn stripe_albedo(albedo: &mut [f32], grid: &Grid, p: &WorldParams) {
    for (i, _, a) in raster(grid, p) {
        albedo[i] = (albedo[i] + a).clamp(-1.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_is_a_trough_between_two_ridges() {
        assert!((stripe_profile_km(0.0, 0.0) + 0.5).abs() < 1e-6);
        assert!((stripe_profile_km(2.0, 0.0) - 0.1).abs() < 0.02);
        assert!((stripe_profile_km(-2.0, 0.0) - 0.1).abs() < 0.02);
        assert_eq!(stripe_profile_km(6.0, 0.0), 0.0);
        assert!(stripe_profile_km(0.0, 0.49).abs() < 0.1);
    }
}
