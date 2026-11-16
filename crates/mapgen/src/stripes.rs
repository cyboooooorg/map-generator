//! The fractures a body's geysers vent through ([`crate::planet::geysers`]),
//! cut into its world: troughs flanked by two ridges, as Enceladus's tiger
//! stripes (Porco et al. 2006, doi:10.1126/science.1123013), darker than
//! the plains around them.
use crate::geo::Grid;
use crate::planet::geysers::unit_at;
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
/// Narrowest the trough is drawn, in rows of the grid (`Grid::dy_km`): a
/// trough narrower than a pixel would fall between the pixel centres of
/// the base map. Its depths are scaled down to keep the area of its
/// cross-section.
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

/// The width the stripes are drawn at on `grid`, km.
fn drawn_width_km(grid: &Grid) -> f64 {
    TROUGH_WIDTH_KM.max(MIN_WIDTH_PIXELS * f64::from(grid.dy_km()))
}

/// Adds `f(across_km, along)` to the value of every pixel of `grid` near one
/// of the fractures of the geysers of `p` (none without geysers): within
/// `reach_km` of its line and between its ends. Only the rows of the
/// polar cap the fractures lie in are visited; values stay in [−1, 1].
fn for_each_near(
    grid: &Grid,
    p: &WorldParams,
    reach_km: f64,
    f: impl Fn(f64, f64) -> f32 + Sync,
    values: &mut [f32],
) {
    let Some(geysers) = p.model.as_ref().and_then(|m| m.geysers.as_ref()) else {
        return;
    };
    let radius_km = f64::from(grid.circumference_km) / std::f64::consts::TAU;
    // The farthest point of a fracture from the pole is one of its ends.
    let pole_lat = if geysers.south { -90.0 } else { 90.0 };
    let pole = unit_at(pole_lat, 0.0);
    let cap = geysers
        .fractures
        .iter()
        .flat_map(|f| {
            let (a, b) = f.ends(radius_km);
            [a, b]
        })
        .map(|e| {
            (pole[0] * e[0] + pole[1] * e[1] + pole[2] * e[2])
                .clamp(-1.0, 1.0)
                .acos()
        })
        .fold(0.0, f64::max)
        + reach_km / radius_km
        + std::f64::consts::PI / f64::from(grid.height);
    // Each fracture's centre, strike and normal, once for all the pixels.
    let frames: Vec<_> = geysers
        .fractures
        .iter()
        .filter_map(|f| f.frame(radius_km))
        .collect();
    let w = grid.width as usize;
    values
        .par_chunks_mut(w)
        .enumerate()
        .filter(|(y, _)| {
            let lat = grid.latitude(*y as u32).to_degrees();
            (lat - pole_lat).abs().to_radians() <= cap
        })
        .for_each(|(y, row)| {
            for (x, value) in row.iter_mut().enumerate() {
                let v = grid.unit_vector_at(x as f64, y as f64);
                for fracture in &frames {
                    if let Some((across, along)) = fracture.offset_km(v)
                        && across.abs() <= reach_km
                    {
                        *value = (*value + f(across, along)).clamp(-1.0, 1.0);
                    }
                }
            }
        });
}

/// Cuts the stripes of the geysers of `p` into `elevation` (a no-op
/// without geysers): [`stripe_profile_km`] at each pixel near a fracture,
/// in elevation units ([`crate::craters::ELEVATION_KM`]), its widths
/// floored at [`MIN_WIDTH_PIXELS`] rows of `grid` and its depths scaled
/// to keep the area of the cross-section.
pub fn apply_stripes(elevation: &mut [f32], grid: &Grid, p: &WorldParams) {
    let width = drawn_width_km(grid);
    let units = f64::from(crate::craters::ELEVATION_KM);
    for_each_near(
        grid,
        p,
        1.5 * width,
        |across, along| (profile_km(across, along, width) / units) as f32,
        elevation,
    );
}

/// Darkens `albedo` by [`STRIPE_ALBEDO`] over the troughs of the stripes
/// of the geysers of `p` (a no-op without geysers), at their width on
/// `grid` (see [`apply_stripes`]).
pub fn stripe_albedo(albedo: &mut [f32], grid: &Grid, p: &WorldParams) {
    let width = drawn_width_km(grid);
    for_each_near(
        grid,
        p,
        0.5 * width,
        |across, along| STRIPE_ALBEDO * (bump(across / width) * taper(along)) as f32,
        albedo,
    );
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
