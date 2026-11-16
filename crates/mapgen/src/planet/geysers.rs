//! The geysers of an icy body ([`geysers`]): where its ice shell over an
//! ocean is thin enough at a pole for fractures to crack it through, the
//! ocean vents through parallel fractures there, as the "tiger stripes"
//! of Enceladus's south pole (Porco et al. 2006,
//! doi:10.1126/science.1123013). The fractures are stored on the model so
//! that the surface map and the viewer draw them at the same place.
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use serde::{Deserialize, Serialize};

use super::ice_shell::{ICE_DENSITY, shell_at_m};
use super::{IceShell, Interior, InternalHeat};
use crate::exomoon::{EARTH_KG, EARTH_M, G};

/// Hydrostatic pressure, Pa, at the base of an ice shell above which
/// fractures no longer crack it through to the ocean: Rudolph & Manga
/// 2009 (doi:10.1016/j.icarus.2009.01.015) model cracks opened by 1 to
/// 3 MPa of tension against the overburden that closes them, and find
/// that they cross Europa's shell only when it is under about 2.5 km, and
/// Enceladus's when it is under about 25 km. The overburden `ρ g h` of
/// both limits is about the same, 3.0 MPa for Europa (2.5 km under
/// 1.31 m/s²) and 2.6 MPa for Enceladus (25 km under 0.113 m/s²), which
/// this stands for: calibration on those two limits, the lower of them.
/// The tension itself, from the ocean's pressurisation as it freezes
/// (Manga & Wang 2007, doi:10.1029/2007GL029297) or the tides, is not
/// computed.
const CRACK_OVERBURDEN_PA: f64 = 2.6e6;

/// Spacing of the fractures, km: Enceladus's tiger stripes are about
/// 35 km apart (Porco et al. 2006, doi:10.1126/science.1123013).
pub const FRACTURE_SPACING_KM: f64 = 35.0;

/// Widest the fractured region gets around its pole, degrees of
/// colatitude: the fractures stay in the polar region the tides thin
/// most, and the gnomonic plane they are laid out in does not stretch
/// them much (1.15 times at the edge). Calibration, no source: it bounds
/// the region of a body whose shell cracks through everywhere (a drawn
/// moon hot enough that its shell is under a kilometre); an asked
/// Enceladus's region is 16°.
const MAX_REGION_DEG: f64 = 30.0;

/// Least share of a body's internal heat that its tides must make for it
/// to vent: the tides thin the shell at the poles (Ojakangas & Stevenson
/// 1989, doi:10.1016/0019-1035(89)90052-3) and keep the ocean and the
/// vents open (Enceladus radiates some 15.8 GW, Howett et al. 2011,
/// doi:10.1029/2010JE003718, against a fraction of a GW of radiogenic
/// heat). Calibration, no source: the tides make most of the heat.
const TIDAL_SHARE: f64 = 0.5;

/// Step, degrees of colatitude, of the search for the region's edge.
const REGION_STEP_DEG: f64 = 0.1;

/// The geysers of a body: the pole they vent from and the fractures they
/// vent through.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Geysers {
    /// Whether they vent from the south pole (else the north).
    pub south: bool,
    /// The fractures, parallel, from one side of the region to the other.
    pub fractures: Vec<Fracture>,
}

/// A fracture the geysers vent through: a great-circle segment
/// `length_km` long, centred on (`lat_deg`, `lon_deg`), running along
/// `azimuth_deg` from its centre both ways.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Fracture {
    /// Latitude of its centre, degrees, north positive.
    pub lat_deg: f32,
    /// Longitude of its centre, degrees east, −180 to 180.
    pub lon_deg: f32,
    /// Its strike at its centre, degrees clockwise from north. At a pole,
    /// where north is not defined, 0 means along the meridian `lon_deg`
    /// (through the pole to the opposite meridian).
    pub azimuth_deg: f32,
    /// Length, km, along the surface.
    pub length_km: f32,
}

/// The geysers of a body of `interior` making `heat` under the ice shell
/// `shell`, the pole and the fractures' strike drawn from `seed` (a
/// stream of their own); `None` when its shell does not crack through.
///
/// - **Criterion.** The body vents when its shell lies over an ocean,
///   does not convect (a convecting shell is not thinned), is thinned at
///   the poles by the tides (its polar shell under its mean, the tides
///   making at least [`TIDAL_SHARE`] of its heat), and is thin enough at
///   the pole that fractures cross it: the hydrostatic pressure
///   at its base, `ρ g h` with [`ICE_DENSITY`] and the surface gravity, is
///   at most [`CRACK_OVERBURDEN_PA`] (Rudolph & Manga 2009,
///   doi:10.1016/j.icarus.2009.01.015).
/// - **Pole.** The tides thin both poles alike; Enceladus vents from its
///   south pole only, so the seed picks one.
/// - **Region.** The fractures fill the cap around the pole where the
///   shell ([`shell_at_m`]) still passes the criterion, up to
///   [`MAX_REGION_DEG`].
/// - **Fractures.** Parallel, [`FRACTURE_SPACING_KM`] apart, centred on
///   the pole, each from one edge of the region to the other, as the
///   tiger stripes (Porco et al. 2006, doi:10.1126/science.1123013: four
///   fractures about 130 km long, 35 km apart). They are straight lines
///   of the gnomonic projection centred on the pole, so great circles on
///   the body; their count and lengths scale with the region (an asked
///   Enceladus gets four, about 100 and 140 km long). Their strike is
///   drawn from the seed.
pub fn geysers(
    interior: &Interior,
    shell: &IceShell,
    heat: &InternalHeat,
    seed: u32,
) -> Option<Geysers> {
    if shell.convecting
        || shell.ocean_km <= 0.0
        || shell.polar_shell_km >= shell.shell_km
        || heat.tidal_w < TIDAL_SHARE * heat.total_w()
    {
        return None;
    }
    let radius_m = interior.radius_earth * EARTH_M;
    let mass_kg: f64 = interior.layers.iter().map(|l| l.mass_earth).sum::<f64>() * EARTH_KG;
    let gravity = G * mass_kg / (radius_m * radius_m);
    let cracks = |shell_m: f64| ICE_DENSITY * gravity * shell_m <= CRACK_OVERBURDEN_PA;
    if !cracks(shell.polar_shell_km * 1e3) {
        return None;
    }
    let mut region_deg = 0.0;
    while region_deg + REGION_STEP_DEG <= MAX_REGION_DEG
        && cracks(shell_at_m(
            interior,
            shell,
            heat,
            region_deg + REGION_STEP_DEG,
        ))
    {
        region_deg += REGION_STEP_DEG;
    }
    let mut rng = StdRng::seed_from_u64(u64::from(seed));
    let south = rng.random::<bool>();
    let strike_deg = rng.random_range(-180.0..180.0);
    Some(Geysers {
        south,
        fractures: fractures(radius_m / 1e3, region_deg, south, strike_deg),
    })
}

/// The fractures across a polar cap of `region_deg` on a body of
/// `radius_km`, around its south pole if `south`, parallel to the
/// meridian `strike_deg` (see [`geysers`]).
fn fractures(radius_km: f64, region_deg: f64, south: bool, strike_deg: f64) -> Vec<Fracture> {
    // In the gnomonic plane tangent at the pole, the angle from the x axis
    // is the longitude: the fractures run along `u` (towards the meridian
    // `strike_deg`), offset along `v` (towards `strike_deg + 90`).
    let edge_km = radius_km * region_deg.to_radians().tan();
    let count = ((2.0 * edge_km / FRACTURE_SPACING_KM).round() as usize).max(1);
    let sign = if south { -1.0 } else { 1.0 };
    let (su, cu) = strike_deg.to_radians().sin_cos();
    let (u, v) = ([cu, su], [-su, cu]);
    // The unit vector of the point (x, y) of the plane, km.
    let point = |x: f64, y: f64| {
        let n = (x * x + y * y + radius_km * radius_km).sqrt();
        [x / n, y / n, sign * radius_km / n]
    };
    (0..count)
        .map(|i| {
            let offset = (i as f64 - (count as f64 - 1.0) / 2.0) * FRACTURE_SPACING_KM;
            let half = (edge_km * edge_km - offset * offset).max(0.0).sqrt();
            let at = |along: f64| point(offset * v[0] + along * u[0], offset * v[1] + along * u[1]);
            let (a, b) = (at(-half), at(half));
            let dot: f64 = a.iter().zip(&b).map(|(p, q)| p * q).sum();
            let length_km = radius_km * dot.clamp(-1.0, 1.0).acos();
            let colatitude_deg = (offset.abs() / radius_km).atan().to_degrees();
            let (lon_deg, azimuth_deg) = if offset == 0.0 {
                (strike_deg, 0.0)
            } else {
                // Off the pole, the fracture's centre is the point of its
                // line nearest the pole: it runs east–west there.
                let towards = if offset > 0.0 { 90.0 } else { -90.0 };
                (wrap_deg(strike_deg + towards), 90.0)
            };
            Fracture {
                lat_deg: (sign * (90.0 - colatitude_deg)) as f32,
                lon_deg: lon_deg as f32,
                azimuth_deg: azimuth_deg as f32,
                length_km: length_km as f32,
            }
        })
        .collect()
}

/// `deg` wrapped into −180 to 180.
fn wrap_deg(deg: f64) -> f64 {
    (deg + 180.0).rem_euclid(360.0) - 180.0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Enceladus's tiger stripes: four fractures 35 km apart across a
    /// polar cap of 16° on a 252 km moon, the inner two about 130 km long.
    #[test]
    fn a_polar_cap_holds_parallel_fractures_35_km_apart() {
        let f = fractures(252.0, 16.0, true, 30.0);
        assert_eq!(f.len(), 4, "{f:?}");
        for x in &f {
            assert!(x.lat_deg < -75.0, "{x:?}");
            assert_eq!(x.azimuth_deg, 90.0);
        }
        let inner = f[1].length_km;
        assert!((120.0..150.0).contains(&inner), "{inner}");
        assert!(f[0].length_km < inner && (f[0].length_km - f[3].length_km).abs() < 1e-3);
        // Centres on either side of the pole, across the strike.
        assert!((f[0].lon_deg - -60.0).abs() < 1e-3 && (f[3].lon_deg - 120.0).abs() < 1e-3);
        // Neighbours 35 km apart near the pole.
        let gap = 252.0 * (f[1].lat_deg - f[0].lat_deg).to_radians().abs();
        assert!((gap - 35.0).abs() < 2.0, "{gap}");
    }

    #[test]
    fn an_odd_count_runs_one_fracture_through_the_pole() {
        let f = fractures(252.0, 12.0, false, 170.0);
        assert_eq!(f.len(), 3, "{f:?}");
        assert_eq!(f[1].lat_deg, 90.0);
        assert_eq!((f[1].lon_deg, f[1].azimuth_deg), (170.0, 0.0));
        assert!((f[2].lon_deg - -100.0).abs() < 1e-3, "{f:?}");
    }

    #[test]
    fn a_tiny_region_still_has_one_fracture() {
        let f = fractures(252.0, 0.0, false, 0.0);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].length_km, 0.0);
    }
}
