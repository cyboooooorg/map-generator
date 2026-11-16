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
/// 2009 (doi:10.1016/j.icarus.2008.10.010) model cracks opened by 1 to
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

/// Most fractures stored: a region that would hold more at
/// [`FRACTURE_SPACING_KM`] gets this many, spaced wider to fill it, their
/// lengths still from edge to edge. A storage and rendering limit, not
/// physics: the surface map and the viewer draw each fracture, and a hot
/// body with a thin shell would otherwise store over a hundred.
pub const MAX_FRACTURES: usize = 12;

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

/// How far the young terrain around the geysers' pole reaches, as a
/// multiple of the fractures' farthest end from the pole: Enceladus's
/// tiger stripes fill a cap of about 16° and its young south polar
/// terrain, bounded by a ring of cliffs and folds near 55° S, about 35°
/// (Porco et al. 2006, doi:10.1126/science.1123013). Calibration on that
/// one body.
pub const YOUNG_CAP_FACTOR: f64 = 2.0;

/// Widest the young terrain around the geysers' pole gets, degrees from
/// the pole: Enceladus's south polar terrain reaches about 35° from its
/// pole (bounded near 55° S, Porco et al. 2006,
/// doi:10.1126/science.1123013); this bounds a body whose fractures fill
/// a wider cap. Calibration, a little beyond Enceladus's.
pub const YOUNG_CAP_MAX_DEG: f64 = 40.0;

/// The point of the unit sphere at `lat_deg` north and `lon_deg` east, in
/// the frame of [`crate::geo::Grid::unit_vector_at`]: `[cos lat cos lon,
/// cos lat sin lon, sin lat]`.
pub fn unit_at(lat_deg: f64, lon_deg: f64) -> [f64; 3] {
    let (sl, cl) = lat_deg.to_radians().sin_cos();
    let (so, co) = lon_deg.to_radians().sin_cos();
    [cl * co, cl * so, sl]
}

impl Fracture {
    /// Its centre, on the unit sphere (see [`unit_at`]).
    fn centre(&self) -> [f64; 3] {
        unit_at(f64::from(self.lat_deg), f64::from(self.lon_deg))
    }

    /// The unit tangent at its centre along its strike `azimuth_deg`,
    /// clockwise from north (east = north × up). At a pole, north is the
    /// direction of the meridian `lon_deg`.
    fn strike(&self) -> [f64; 3] {
        let c = self.centre();
        let (lat, lon) = (
            f64::from(self.lat_deg).to_radians(),
            f64::from(self.lon_deg).to_radians(),
        );
        let north = if self.lat_deg.abs() >= 90.0 {
            [lon.cos(), lon.sin(), 0.0]
        } else {
            [-lat.sin() * lon.cos(), -lat.sin() * lon.sin(), lat.cos()]
        };
        let east = cross(north, c);
        let (sa, ca) = f64::from(self.azimuth_deg).to_radians().sin_cos();
        normalize(add(scale(north, ca), scale(east, sa)))
    }

    /// Its two ends on the unit sphere of a body of `radius_km`: half its
    /// length from its centre along its strike, then along the opposite
    /// strike, on the great circle through them.
    pub fn ends(&self, radius_km: f64) -> ([f64; 3], [f64; 3]) {
        let (c, d) = (self.centre(), self.strike());
        let half = f64::from(self.length_km) / 2.0 / radius_km;
        let (s, k) = half.sin_cos();
        (
            add(scale(c, k), scale(d, -s)),
            add(scale(c, k), scale(d, s)),
        )
    }

    /// Where the unit vector `p` lies relative to it on a body of
    /// `radius_km`: `(across_km, along)`, its signed distance, km, from the
    /// fracture's great circle (positive on the side of the normal
    /// centre × strike), and the position of its projection on that circle
    /// from the centre, in lengths of the fracture (−0.5 and 0.5 at the
    /// ends, positive towards the strike). `None` when the projection falls
    /// beyond the ends, when `|across_km|` is over [`FRACTURE_SPACING_KM`]
    /// or when the fracture has no length.
    pub fn offset_km(&self, p: [f64; 3], radius_km: f64) -> Option<(f64, f64)> {
        self.frame(radius_km)?.offset_km(p)
    }

    /// Its centre, strike and normal on a body of `radius_km`, worked out
    /// once for many [`FractureFrame::offset_km`] calls; `None` when it has
    /// no length.
    pub fn frame(&self, radius_km: f64) -> Option<FractureFrame> {
        let span = f64::from(self.length_km) / radius_km;
        if span <= 0.0 {
            return None;
        }
        let (c, d) = (self.centre(), self.strike());
        Some(FractureFrame {
            c,
            d,
            n: normalize(cross(c, d)),
            span,
            radius_km,
        })
    }
}

/// A [`Fracture`]'s frame on a body, for the per-pixel loop of the stripes:
/// the trigonometry of its centre, strike and normal is done once.
#[derive(Clone, Copy, Debug)]
pub struct FractureFrame {
    c: [f64; 3],
    d: [f64; 3],
    n: [f64; 3],
    /// Its length in radians.
    span: f64,
    radius_km: f64,
}

impl FractureFrame {
    /// Same as [`Fracture::offset_km`].
    pub fn offset_km(&self, p: [f64; 3]) -> Option<(f64, f64)> {
        self.offset_within_km(p, FRACTURE_SPACING_KM)
    }

    /// [`FractureFrame::offset_km`] with `max_across_km` in place of
    /// [`FRACTURE_SPACING_KM`]: `None` when `|across_km|` is over it.
    pub fn offset_within_km(&self, p: [f64; 3], max_across_km: f64) -> Option<(f64, f64)> {
        let across_km = dot(p, self.n).clamp(-1.0, 1.0).asin() * self.radius_km;
        if across_km.abs() > max_across_km {
            return None;
        }
        let along = dot(p, self.d).atan2(dot(p, self.c)) / self.span;
        (along.abs() <= 0.5).then_some((across_km, along))
    }
}

/// Golden angle, radians: steps the phases of [`meander_km`] from one
/// fracture to the next so that no two neighbours bend alike.
const GOLDEN_ANGLE: f64 = 2.399963229728653;

/// Largest `|meander_km|`, km: the sum of the bounds of its wave (5 km),
/// bow (4 km) and hooks (8 km).
pub const MAX_MEANDER_KM: f64 = 17.0;

// Each line keeps to its side of the midline with its neighbours.
const _: () = assert!(MAX_MEANDER_KM < FRACTURE_SPACING_KM / 2.0);

/// Signed sideways offset, km, of the drawn centre line of the fracture
/// number `index` (its position in [`Geysers::fractures`], zero-length
/// ones included) from its great circle, at `along` its length from its
/// centre (−0.5 and 0.5 at its ends), positive on the side of the normal
/// centre × strike (see [`Fracture::offset_km`]).
///
/// Look calibration, no source: the stored fractures are straight and
/// parallel, while Enceladus's tiger stripes are roughly parallel but
/// wavy, slightly bowed, with hooked ends. The sum of
/// - a wave of two sines, at most 6 % of the length and 5 km;
/// - a bow over the whole fracture, at most 5 % of the length and 4 km,
///   its sign and size varying from one fracture to the next so that
///   neighbours are not parallel;
/// - a hook over the last 12 % of the length at each end, at most 8 % of
///   the length and 8 km, curling to opposite sides at the two ends and
///   alternating from one fracture to the next.
///
/// The bounds keep stripes [`FRACTURE_SPACING_KM`] apart from touching
/// (at most [`MAX_MEANDER_KM`] each). The viewer's jets follow the same
/// line: `meanderKm` in web/src/lib/viewer/jets.ts is its twin.
pub fn meander_km(index: usize, along: f64, length_km: f64) -> f64 {
    use std::f64::consts::TAU;
    let k = index as f64;
    let amp = (0.06 * length_km).min(5.0);
    let wave = amp
        * (0.65 * (TAU * 0.9 * along + k * GOLDEN_ANGLE).sin()
            + 0.35 * (TAU * 2.1 * along + 1.7 + k * GOLDEN_ANGLE * 1.3).sin());
    let bow =
        (0.05 * length_km).min(4.0) * (k * GOLDEN_ANGLE * 0.7).cos() * (1.0 - 4.0 * along * along);
    let u = (along.abs() - 0.38).max(0.0) / 0.12;
    let end = if along > 0.0 { 1.0 } else { -1.0 };
    let parity = if index.is_multiple_of(2) { 1.0 } else { -1.0 };
    let hook = (0.08 * length_km).min(8.0) * u * u * end * parity;
    wave + bow + hook
}

/// The young terrain around the geysers' pole: the cap their fractures
/// resurface, as Enceladus's south polar terrain, where craters are few.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct YoungTerrain {
    /// The pole the geysers vent from, on the unit sphere (see
    /// [`unit_at`]).
    pub pole: [f64; 3],
    /// Its angular radius around the pole, radians.
    pub cap_rad: f64,
}

impl YoungTerrain {
    /// The young terrain of `geysers` on a body of `radius_km`:
    /// [`YOUNG_CAP_FACTOR`] times the angle from the pole to the
    /// fractures' farthest end, at most [`YOUNG_CAP_MAX_DEG`].
    pub fn of(geysers: &Geysers, radius_km: f64) -> YoungTerrain {
        let pole = unit_at(if geysers.south { -90.0 } else { 90.0 }, 0.0);
        let farthest = geysers
            .fractures
            .iter()
            .flat_map(|f| {
                let (a, b) = f.ends(radius_km);
                [a, b]
            })
            .map(|e| dot(pole, e).clamp(-1.0, 1.0).acos())
            .fold(0.0, f64::max);
        YoungTerrain {
            pole,
            cap_rad: (YOUNG_CAP_FACTOR * farthest).min(YOUNG_CAP_MAX_DEG.to_radians()),
        }
    }

    /// Whether the unit vector `p` lies in it.
    pub fn covers(&self, p: [f64; 3]) -> bool {
        dot(self.pole, p) >= self.cap_rad.cos()
    }
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale(a: [f64; 3], k: f64) -> [f64; 3] {
    [a[0] * k, a[1] * k, a[2] * k]
}

fn normalize(a: [f64; 3]) -> [f64; 3] {
    scale(a, 1.0 / dot(a, a).sqrt())
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
///   doi:10.1016/j.icarus.2008.10.010).
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
///   Enceladus gets four, about 100 and 140 km long), at most
///   [`MAX_FRACTURES`], spaced wider when the region would hold more.
///   Their strike is drawn from the seed.
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
    let wanted = ((2.0 * edge_km / FRACTURE_SPACING_KM).round() as usize).max(1);
    let count = wanted.min(MAX_FRACTURES);
    let spacing_km = if wanted > MAX_FRACTURES {
        2.0 * edge_km / MAX_FRACTURES as f64
    } else {
        FRACTURE_SPACING_KM
    };
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
            let offset = (i as f64 - (count as f64 - 1.0) / 2.0) * spacing_km;
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

    /// A hot body with a shell thin everywhere vents over the whole 30°
    /// cap: 5000 km in radius, it would hold 165 fractures 35 km apart; it
    /// stores twelve, spread across the cap, still from edge to edge.
    #[test]
    fn a_wide_region_stores_at_most_twelve_fractures_across_it() {
        let radius = 5000.0;
        let f = fractures(radius, MAX_REGION_DEG, false, 0.0);
        assert_eq!(f.len(), MAX_FRACTURES, "{f:?}");
        // The outermost reach most of the way to the cap's edge.
        let edge = radius * MAX_REGION_DEG.to_radians().tan();
        let outer = radius * (90.0 - f64::from(f[0].lat_deg)).to_radians().tan();
        assert!(outer > 0.85 * edge && outer < edge, "{outer} of {edge}");
        // The middle ones run nearly the whole cap across.
        let middle = f64::from(f[5].length_km);
        assert!(
            middle > 0.95 * 2.0 * radius * MAX_REGION_DEG.to_radians(),
            "{middle}"
        );
    }

    /// Angle, radians, between two unit vectors.
    fn angle(a: [f64; 3], b: [f64; 3]) -> f64 {
        dot(a, b).clamp(-1.0, 1.0).acos()
    }

    /// Whether the unit vector `e` lies at longitude `lon` degrees, modulo 360.
    fn lon_close(e: [f64; 3], lon: f64) -> bool {
        let d = (e[1].atan2(e[0]).to_degrees() - lon).rem_euclid(360.0);
        d < 1e-6 || d > 360.0 - 1e-6
    }

    /// The fractures of an asked Enceladus: a 16° cap around its south pole.
    fn enceladus_geysers() -> Geysers {
        Geysers {
            south: true,
            fractures: fractures(252.1, 16.0, true, 30.0),
        }
    }

    #[test]
    fn ends_are_half_a_length_from_the_centre() {
        let f = Fracture {
            lat_deg: -80.0,
            lon_deg: 30.0,
            azimuth_deg: 90.0,
            length_km: 130.0,
        };
        let r = 252.1;
        let (a, b) = f.ends(r);
        let c = unit_at(-80.0, 30.0);
        for e in [a, b] {
            assert!((angle(c, e) * r - 65.0).abs() < 0.1);
        }
    }

    #[test]
    fn offset_is_zero_on_the_fracture_and_spacing_away_beside_it() {
        // the centre is on it; a point 10 km across the strike is 10 km off
        let f = Fracture {
            lat_deg: -80.0,
            lon_deg: 30.0,
            azimuth_deg: 90.0,
            length_km: 130.0,
        };
        let r = 252.1;
        let (across, along) = f.offset_km(unit_at(-80.0, 30.0), r).unwrap();
        assert!(across.abs() < 1e-6 && along.abs() < 1e-6);
        let north = unit_at(-80.0 + (10.0 / r).to_degrees(), 30.0);
        assert!((f.offset_km(north, r).unwrap().0.abs() - 10.0).abs() < 0.05);
        assert!(f.offset_km(unit_at(-80.0, 30.0 + 90.0), r).is_none());
    }

    #[test]
    fn segment_through_the_pole() {
        let f = Fracture {
            lat_deg: -90.0,
            lon_deg: 45.0,
            azimuth_deg: 0.0,
            length_km: 130.0,
        };
        let r = 252.1;
        let (a, b) = f.ends(r);
        // runs along the 45°/225° meridians
        for (e, lon) in [(a, 45.0_f64), (b, 225.0)] {
            assert!(lon_close(e, lon) || lon_close(e, lon - 180.0));
        }
        assert!(f.offset_km(unit_at(-89.0, 45.0), r).unwrap().0.abs() < 0.05);
    }

    #[test]
    fn segment_across_the_seam() {
        let f = Fracture {
            lat_deg: -82.0,
            lon_deg: 179.0,
            azimuth_deg: 90.0,
            length_km: 130.0,
        };
        let r = 252.1;
        for lon in [178.0, -179.5] {
            assert!(f.offset_km(unit_at(-82.0, lon), r).is_some());
        }
    }

    /// The stored fractures run from edge to edge of their cap: their ends
    /// on the sphere are those of the gnomonic layout, 16° from the pole.
    #[test]
    fn ends_of_drawn_fractures_lie_on_the_region_edge() {
        let pole = unit_at(-90.0, 0.0);
        for strike in [30.0, -170.0] {
            for f in fractures(252.1, 16.0, true, strike) {
                let (a, b) = f.ends(252.1);
                for e in [a, b] {
                    let deg = angle(pole, e).to_degrees();
                    assert!((deg - 16.0).abs() < 0.01, "{f:?} {deg}");
                }
            }
        }
    }

    #[test]
    fn young_terrain_of_enceladus_reaches_about_55_south() {
        // the fractures part A draws for an Enceladus-like moon
        let g = enceladus_geysers();
        let y = YoungTerrain::of(&g, 252.1);
        assert!((25.0..=40.0).contains(&y.cap_rad.to_degrees()));
        assert!(y.covers(unit_at(-75.0, 0.0)) && !y.covers(unit_at(-45.0, 0.0)));
    }

    /// The twin of `meanderKm` in web/src/lib/viewer/jets.ts: the same
    /// values are pinned in jets.test.ts.
    #[test]
    fn meander_matches_its_viewer_twin() {
        for (index, along, length, want) in [
            (0, 0.0, 130.0, 5.73541341829182),
            (1, 0.25, 130.0, -0.6613252465646292),
            (2, -0.45, 130.0, 0.6434323178558201),
            (3, 0.5, 130.0, -11.46808168745059),
            (5, -0.1, 40.0, -2.635000805293958),
            (11, 0.49, 3000.0, -10.857832404480899),
        ] {
            let got = meander_km(index, along, length);
            assert!((got - want).abs() < 1e-9, "{index} {along} {length}: {got}");
        }
    }

    /// The meander stays within [`MAX_MEANDER_KM`], is zero without
    /// length, and bends neighbours differently.
    #[test]
    fn meander_is_bounded_and_differs_between_neighbours() {
        for index in 0..MAX_FRACTURES {
            assert_eq!(meander_km(index, 0.3, 0.0), 0.0);
            for length in [10.0, 130.0, 5000.0] {
                for i in 0..=100 {
                    let along = -0.5 + f64::from(i) / 100.0;
                    let m = meander_km(index, along, length);
                    assert!(m.abs() <= MAX_MEANDER_KM, "{index} {along} {length}: {m}");
                }
            }
        }
        let diff = (0..=20)
            .map(|i| {
                let along = -0.5 + f64::from(i) / 20.0;
                (meander_km(1, along, 130.0) - meander_km(2, along, 130.0)).abs()
            })
            .fold(0.0, f64::max);
        assert!(diff > 2.0, "{diff}");
    }

    #[test]
    fn a_tiny_region_still_has_one_fracture() {
        let f = fractures(252.0, 0.0, false, 0.0);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].length_km, 0.0);
    }
}
