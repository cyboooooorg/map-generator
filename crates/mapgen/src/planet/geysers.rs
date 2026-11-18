//! The geysers of an icy body ([`geysers`]): where its ice shell over an
//! ocean is thin enough at a pole for fractures to crack it through, the
//! ocean vents through roughly parallel fractures there, as the "tiger
//! stripes" of Enceladus's south pole (Porco et al. 2006,
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

/// Mean spacing of the fractures, km: Enceladus's tiger stripes are
/// about 35 km apart (Porco et al. 2006, doi:10.1126/science.1123013).
/// Each body's mean is drawn from [`MIN_GAP_KM`] to [`MAX_GAP_KM`]
/// around it.
pub const FRACTURE_SPACING_KM: f64 = 35.0;

/// Most fractures stored, a branch included: a region that would hold
/// more at the drawn gaps gets this many, spaced wider to fill it. A storage and rendering limit, not
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
    /// The fractures, roughly parallel, inside the region; a branch, when
    /// there is one, comes last.
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

/// Signed sideways offset, km, of the drawn centre line of the fracture
/// number `index` (its position in [`Geysers::fractures`], zero-length
/// ones included) from its great circle, at `along` its length from its
/// centre (−0.5 and 0.5 at its ends), positive on the side of the normal
/// centre × strike (see [`Fracture::offset_km`]).
///
/// Look calibration, no source: the stored fractures are straight, while
/// Enceladus's tiger stripes are wavy, slightly bowed, with hooked ends. The sum of
/// - a wave of two sines, at most 6 % of the length and 5 km;
/// - a bow over the whole fracture, at most 5 % of the length and 4 km,
///   its sign and size varying from one fracture to the next so that
///   neighbours are not parallel;
/// - a hook over the last 12 % of the length at each end, at most 8 % of
///   the length and 8 km, curling to opposite sides at the two ends and
///   alternating from one fracture to the next.
///
/// [`fractures`] keeps the drawn lines [`MIN_CLEARANCE_KM`] apart with
/// these meanders (at most [`MAX_MEANDER_KM`] each). The viewer's jets follow the same
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

/// Whether a body of `interior` making `heat` under the ice shell
/// `shell` vents: the seedless criterion of [`geysers`] (see there), its
/// [`vent_strength`] above 0.
pub(crate) fn vents(interior: &Interior, shell: &IceShell, heat: &InternalHeat) -> bool {
    vent_strength(interior, shell, heat) > 0.0
}

/// Polar overburden, as a share of [`CRACK_OVERBURDEN_PA`], at and under
/// which a body vents at full strength ([`vent_strength`]): 0.6.
/// Calibration on Enceladus, which the plume's fallout is calibrated on
/// (`albedo::icy_unit_albedo`): the model's Enceladus carries 0.4 to
/// 0.55 of the threshold at its pole, so it vents in full on every seed.
const FULL_VENT_OVERBURDEN: f64 = 0.6;

/// How strongly a body of `interior` making `heat` under the ice shell
/// `shell` vents, 0 to 1: 0 unless it passes the conditions of
/// [`geysers`] other than the crack (an ocean, a shell that does not
/// convect and that its tides thin at the poles, the tides making
/// [`TIDAL_SHARE`] of its heat), then rising from 0 with its polar shell
/// at the crack threshold (`ρ g h` at [`CRACK_OVERBURDEN_PA`]) to 1 at
/// [`FULL_VENT_OVERBURDEN`] of it, linearly in the overburden: a thinner
/// shell cracks through more of its cap and lets out more plume. The
/// climate reads it for the plume's fallout (`super::vent_strength`), so
/// that the fallout fades to nothing where the geysers stop, with no
/// step for its fixed point to straddle.
pub(crate) fn vent_strength(interior: &Interior, shell: &IceShell, heat: &InternalHeat) -> f64 {
    if shell.convecting
        || shell.ocean_km <= 0.0
        || shell.polar_shell_km >= shell.shell_km
        || heat.tidal_w < TIDAL_SHARE * heat.total_w()
    {
        return 0.0;
    }
    let x = ICE_DENSITY * gravity(interior) * shell.polar_shell_km * 1e3 / CRACK_OVERBURDEN_PA;
    ((1.0 - x) / (1.0 - FULL_VENT_OVERBURDEN)).clamp(0.0, 1.0)
}

/// Surface gravity, m/s², of a body of `interior`, from its layers.
fn gravity(interior: &Interior) -> f64 {
    let radius_m = interior.radius_earth * EARTH_M;
    let mass_kg: f64 = interior.layers.iter().map(|l| l.mass_earth).sum::<f64>() * EARTH_KG;
    G * mass_kg / (radius_m * radius_m)
}

/// The geysers of a body of `interior` making `heat` under the ice shell
/// `shell`, the pole and the fractures' layout drawn from `seed` (a
/// stream of their own); `None` when its shell does not crack through.
///
/// - **Criterion.** The body vents when its shell lies over an ocean,
///   does not convect (a convecting shell is not thinned), is thinned at
///   the poles by the tides (its polar shell under its mean, the tides
///   making at least [`TIDAL_SHARE`] of its heat), and is thin enough at
///   the pole that fractures cross it: the hydrostatic pressure
///   at its base, `ρ g h` with [`ICE_DENSITY`] and the surface gravity, is
///   under [`CRACK_OVERBURDEN_PA`] (Rudolph & Manga 2009,
///   doi:10.1016/j.icarus.2008.10.010).
/// - **Pole.** The tides thin both poles alike; Enceladus vents from its
///   south pole only, so the seed picks one.
/// - **Region.** The fractures fill the cap around the pole where the
///   shell ([`shell_at_m`]) still passes the criterion, up to
///   [`MAX_REGION_DEG`].
/// - **Fractures.** Roughly parallel across the region, as the tiger
///   stripes (Porco et al. 2006, doi:10.1126/science.1123013: four
///   fractures about 130 km long, 35 km apart), and different from one
///   body to the next ([`fractures`]): a mean spacing drawn from
///   [`MIN_GAP_KM`] to [`MAX_GAP_KM`], each gap within 20 % of it and
///   in that range,
///   across a common strike drawn from the seed, each
///   fracture turned off it by up to 15°, 60 to 100 % of its
///   edge-to-edge chord long and shifted along it by up to 15 % of the
///   chord, and on a third of the bodies a short branch splaying off
///   near the end of one. They are straight lines of the gnomonic
///   projection centred on the pole, so great circles on the body, inside
///   the region and [`MIN_CLEARANCE_KM`] apart with their meanders; their
///   count scales with the region (an asked Enceladus gets three to
///   six), at most [`MAX_FRACTURES`], spaced wider when the region
///   would hold more.
pub fn geysers(
    interior: &Interior,
    shell: &IceShell,
    heat: &InternalHeat,
    seed: u32,
) -> Option<Geysers> {
    if !vents(interior, shell, heat) {
        return None;
    }
    let cracks = |shell_m: f64| ICE_DENSITY * gravity(interior) * shell_m <= CRACK_OVERBURDEN_PA;
    let radius_m = interior.radius_earth * EARTH_M;
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
        fractures: fractures(radius_m / 1e3, region_deg, south, strike_deg, &mut rng).0,
    })
}

/// The fractures across a polar cap of `region_deg` on a body of
/// `radius_km`, around its south pole if `south`, striking about the
/// meridian `strike_deg`, the rest of their layout drawn from `rng` (see
/// [`geysers`]).
///
/// The body's mean spacing is drawn once, from [`MIN_GAP_KM`] to
/// [`MAX_GAP_KM`]; each try draws the gaps, then each fracture's strike, length and
/// shift; a try whose drawn lines (meanders included) come closer than
/// [`MIN_CLEARANCE_KM`] is drawn again, its strikes jittered less, the
/// last one not at all. If no try passes, the one whose lines keep
/// farthest apart is kept. With the fractures comes the index of the
/// branch's parent when the last one is a branch.
fn fractures(
    radius_km: f64,
    region_deg: f64,
    south: bool,
    strike_deg: f64,
    rng: &mut StdRng,
) -> (Vec<Fracture>, Option<usize>) {
    let plane = PolarPlane {
        radius_km,
        sign: if south { -1.0 } else { 1.0 },
        edge_km: radius_km * region_deg.to_radians().tan(),
    };
    let spacing_km = rng.random_range(MIN_GAP_KM..=MAX_GAP_KM);
    let mut best: Option<(f64, Vec<Fracture>)> = None;
    for attempt in 0..LAYOUT_TRIES {
        let jitter = 1.0 - attempt as f64 / (LAYOUT_TRIES - 1) as f64;
        let mut layout = main_fractures(&plane, strike_deg, spacing_km, jitter, rng);
        let clearance = min_clearance_km(&layout, radius_km, None);
        if clearance >= MIN_CLEARANCE_KM {
            let parent = add_branch(&mut layout, radius_km, region_deg, south, rng);
            return (layout, parent);
        }
        if best.as_ref().is_none_or(|(c, _)| clearance > *c) {
            best = Some((clearance, layout));
        }
    }
    (best.map(|(_, layout)| layout).unwrap_or_default(), None)
}

/// Least mean spacing, km, of a body's fractures where they cross the
/// common strike: Enceladus's tiger stripes are about 35 km apart
/// ([`FRACTURE_SPACING_KM`], Porco et al. 2006,
/// doi:10.1126/science.1123013); each body's mean is drawn evenly from
/// this to [`MAX_GAP_KM`], about that, once, so that bodies differ in
/// count, and each gap within [`GAP_JITTER`] of it, kept from this to
/// [`MAX_GAP_KM`] (narrower gaps let the meanders touch). Calibration
/// around the one body.
pub const MIN_GAP_KM: f64 = 25.0;
/// Widest mean spacing, km (see [`MIN_GAP_KM`]).
pub const MAX_GAP_KM: f64 = 45.0;
/// Largest share by which a gap departs from its body's mean spacing.
/// Look calibration.
const GAP_JITTER: f64 = 0.2;
/// Largest turn, degrees, of a fracture's strike from the common one:
/// the tiger stripes are roughly, not exactly, parallel (Porco et al.
/// 2006). Look calibration.
const MAX_STRIKE_JITTER_DEG: f64 = 15.0;
/// Shortest share of its edge-to-edge chord a fracture runs (the longest
/// is the whole chord). Look calibration.
const MIN_LENGTH_SHARE: f64 = 0.6;
/// Largest shift of a fracture's centre along its strike, as a share of
/// its chord, so that the ends are staggered (kept inside the region).
/// Look calibration.
const MAX_SHIFT_SHARE: f64 = 0.15;
/// Odds that a body's fractures have a branch: a short fracture splaying
/// off near one end of a main one, as the tiger stripes' ends fork.
/// Look calibration.
const BRANCH_ODDS: f64 = 1.0 / 3.0;
/// Angle, degrees, between a branch and its parent where it leaves it.
const BRANCH_ANGLE_DEG: std::ops::Range<f64> = 20.0..40.0;
/// Length of a branch, as a share of its parent's.
const BRANCH_LENGTH_SHARE: std::ops::Range<f64> = 0.3..0.5;
/// Where a branch leaves its parent, in lengths from the parent's centre:
/// before the hook of its end (see [`meander_km`]) and its taper.
const BRANCH_ALONG: f64 = 0.4;
/// Least distance, km, between the drawn lines (meanders included) of two
/// fractures, a branch and its parent aside: a trough and its two ridges
/// are 6 km across (see `stripes.rs`); this keeps plains between them, so
/// the fractures never cross. Look calibration.
pub const MIN_CLEARANCE_KM: f64 = 10.0;
/// Layouts drawn before the best is kept (see [`fractures`]).
const LAYOUT_TRIES: usize = 16;
/// Tries at placing a branch inside the region and clear of the others.
const BRANCH_TRIES: usize = 8;
/// Points sampled along a fracture's drawn line for [`min_clearance_km`].
const LINE_SAMPLES: usize = 48;

/// The plane tangent to a body at the pole of its geysers, in km: its
/// straight lines are great circles of the body (gnomonic projection).
/// The angle from its x axis is the longitude.
struct PolarPlane {
    radius_km: f64,
    /// −1 around the south pole, 1 around the north.
    sign: f64,
    /// Radius, km, of the fractured region in the plane.
    edge_km: f64,
}

impl PolarPlane {
    /// The unit vector of the point `(x, y)` of the plane.
    fn point(&self, [x, y]: [f64; 2]) -> [f64; 3] {
        normalize([x, y, self.sign * self.radius_km])
    }
}

/// The main fractures of one layout: gaps drawn within [`GAP_JITTER`] of
/// `spacing_km` (and from [`MIN_GAP_KM`] to [`MAX_GAP_KM`]) across the strike `strike_deg` while they fit in the
/// region's width less half [`MIN_GAP_KM`] (at most [`MAX_FRACTURES`], spread to
/// fill the region when it would hold more), the set shifted at random in
/// what is left; each fracture's strike turned by up to `jitter` times
/// [`MAX_STRIKE_JITTER_DEG`], its length a share of its edge-to-edge chord
/// and its centre shifted along it.
fn main_fractures(
    plane: &PolarPlane,
    strike_deg: f64,
    spacing_km: f64,
    jitter: f64,
    rng: &mut StdRng,
) -> Vec<Fracture> {
    let edge = plane.edge_km;
    let span = (2.0 * edge - MIN_GAP_KM / 2.0).max(0.0);
    let mut gaps: Vec<f64> = Vec::new();
    let mut total = 0.0;
    let mut capped = false;
    loop {
        let gap = (spacing_km * rng.random_range(1.0 - GAP_JITTER..=1.0 + GAP_JITTER))
            .clamp(MIN_GAP_KM, MAX_GAP_KM);
        if total + gap > span {
            break;
        }
        if gaps.len() + 1 == MAX_FRACTURES {
            capped = true;
            break;
        }
        gaps.push(gap);
        total += gap;
    }
    let mut offsets = vec![0.0];
    if capped {
        // Spread as evenly as the old regular layout: the outermost
        // `(n − 1)/n` of the way to the edge.
        let n = MAX_FRACTURES as f64;
        let k = 2.0 * edge * (n - 1.0) / n / total;
        gaps.iter_mut().for_each(|g| *g *= k);
        total *= k;
    }
    for g in &gaps {
        offsets.push(offsets.last().unwrap() + g);
    }
    let slack = if capped { 0.0 } else { span - total };
    let start = -total / 2.0 + rng.random_range(-0.5..=0.5) * slack;
    let (sv, cv) = strike_deg.to_radians().sin_cos();
    let v = [-sv, cv];
    offsets
        .iter()
        .map(|o| {
            let o = start + o;
            let turn = jitter * rng.random_range(-MAX_STRIKE_JITTER_DEG..=MAX_STRIKE_JITTER_DEG);
            let (su, cu) = (strike_deg + turn).to_radians().sin_cos();
            let u = [cu, su];
            // The point of the line nearest the pole, and the half chord.
            let p = [o * v[0], o * v[1]];
            let t = p[0] * u[0] + p[1] * u[1];
            let foot = [p[0] - t * u[0], p[1] - t * u[1]];
            let half = (edge * edge - foot[0] * foot[0] - foot[1] * foot[1])
                .max(0.0)
                .sqrt();
            let share = rng.random_range(MIN_LENGTH_SHARE..=1.0);
            let room = half * (1.0 - share);
            let shift =
                (rng.random_range(-1.0..=1.0) * MAX_SHIFT_SHARE * 2.0 * half).clamp(-room, room);
            let at = |s: f64| plane.point([foot[0] + s * u[0], foot[1] + s * u[1]]);
            let (a, b) = (at(shift - share * half), at(shift + share * half));
            segment(a, b, plane.point(foot), plane.radius_km)
        })
        .collect()
}

/// The fracture along the great circle from `a` to `b` (unit vectors) on
/// a body of `radius_km`; one of no length at `at` when they coincide.
fn segment(a: [f64; 3], b: [f64; 3], at: [f64; 3], radius_km: f64) -> Fracture {
    let angle = dot(a, b).clamp(-1.0, 1.0).acos();
    if angle <= 0.0 {
        return fracture_at(at, None, 0.0);
    }
    let c = normalize(add(a, b));
    let d = normalize(add(b, scale(c, -dot(b, c))));
    fracture_at(c, Some(d), radius_km * angle)
}

/// The fracture centred on the unit vector `c`, striking along the unit
/// tangent `d` there (north when `None`), `length_km` long: its stored
/// strike is measured as [`Fracture::strike`] reads it back, on the
/// stored (rounded) centre.
fn fracture_at(c: [f64; 3], d: Option<[f64; 3]>, length_km: f64) -> Fracture {
    let lat = c[2].clamp(-1.0, 1.0).asin().to_degrees() as f32;
    let lon = c[1].atan2(c[0]).to_degrees() as f32;
    let mut f = Fracture {
        lat_deg: lat,
        lon_deg: lon,
        azimuth_deg: 0.0,
        length_km: length_km as f32,
    };
    if let Some(d) = d {
        let north = f.strike();
        let east = cross(north, f.centre());
        f.azimuth_deg = dot(d, east).atan2(dot(d, north)).to_degrees() as f32;
    }
    f
}

/// With [`BRANCH_ODDS`], adds a branch to the main fractures `layout` of a
/// body of `radius_km`: a short fracture leaving one of them
/// [`BRANCH_ALONG`] from its centre, outwards at [`BRANCH_ANGLE_DEG`] to
/// it, its drawn line starting on its parent's (both meanders
/// accounted). A branch that would leave the region of `region_deg` or
/// come within [`MIN_CLEARANCE_KM`] of another fracture is drawn again,
/// up to [`BRANCH_TRIES`] times, then left out. Gives the parent's index
/// when it adds one.
fn add_branch(
    layout: &mut Vec<Fracture>,
    radius_km: f64,
    region_deg: f64,
    south: bool,
    rng: &mut StdRng,
) -> Option<usize> {
    if !rng.random_bool(BRANCH_ODDS) || layout.len() >= MAX_FRACTURES {
        return None;
    }
    let pole = unit_at(if south { -90.0 } else { 90.0 }, 0.0);
    let index = layout.len();
    for _ in 0..BRANCH_TRIES {
        let parent = rng.random_range(0..index);
        let side = if rng.random_bool(0.5) { 1.0 } else { -1.0 };
        let turn = rng.random_range(BRANCH_ANGLE_DEG).to_radians()
            * if rng.random_bool(0.5) { 1.0 } else { -1.0 };
        let share = rng.random_range(BRANCH_LENGTH_SHARE);
        let p = layout[parent];
        let parent_km = f64::from(p.length_km);
        let Some(frame) = p.frame(radius_km) else {
            continue;
        };
        // The parent's drawn line where the branch leaves it.
        let along = side * BRANCH_ALONG;
        let (s, k) = (along * frame.span).sin_cos();
        let on = add(scale(frame.c, k), scale(frame.d, s));
        let tangent = add(scale(frame.c, -s), scale(frame.d, k));
        let m = meander_km(parent, along, parent_km) / radius_km;
        let junction = add(scale(on, m.cos()), scale(frame.n, m.sin()));
        // Outwards along the parent, turned towards one side.
        let dir = add(
            scale(tangent, side * turn.cos()),
            scale(frame.n, turn.sin()),
        );
        let dir = normalize(add(dir, scale(junction, -dot(dir, junction))));
        // Start off the junction by the branch's own meander at its start,
        // so that its drawn line starts on the parent's.
        let length_km = share * parent_km;
        let mb = meander_km(index, -0.5, length_km) / radius_km;
        let normal = normalize(cross(junction, dir));
        let start = normalize(add(scale(junction, mb.cos()), scale(normal, -mb.sin())));
        let dir = normalize(add(dir, scale(start, -dot(dir, start))));
        let (s, k) = (length_km / radius_km).sin_cos();
        let end = add(scale(start, k), scale(dir, s));
        let inside = |e: [f64; 3]| dot(pole, e).clamp(-1.0, 1.0).acos() <= region_deg.to_radians();
        if !(inside(start) && inside(end)) {
            continue;
        }
        layout.push(segment(start, end, start, radius_km));
        if min_clearance_km(layout, radius_km, Some(parent)) >= MIN_CLEARANCE_KM {
            return Some(parent);
        }
        layout.pop();
    }
    None
}

/// Least distance, km, between the drawn lines (each great circle moved
/// sideways by its [`meander_km`]) of any two of `fractures` on a body of
/// `radius_km`, the last one and the fracture `parent` aside when given
/// (a branch and the fracture it leaves); infinite with fewer than two.
pub fn min_clearance_km(fractures: &[Fracture], radius_km: f64, parent: Option<usize>) -> f64 {
    let lines: Vec<Vec<[f64; 3]>> = fractures
        .iter()
        .enumerate()
        .map(|(i, f)| drawn_line(f, i, radius_km))
        .collect();
    let last = fractures.len().saturating_sub(1);
    let mut least = f64::INFINITY;
    for i in 0..lines.len() {
        for j in i + 1..lines.len() {
            if parent == Some(i) && j == last {
                continue;
            }
            for a in lines[i].windows(2) {
                for b in lines[j].windows(2) {
                    least = least.min(segment_distance(a[0], a[1], b[0], b[1]) * radius_km);
                }
            }
        }
    }
    least
}

/// The drawn line of the fracture `f`, number `index`, on a body of
/// `radius_km`: [`LINE_SAMPLES`] points of its great circle moved sideways
/// by [`meander_km`] (its centre alone when it has no length).
pub fn drawn_line(f: &Fracture, index: usize, radius_km: f64) -> Vec<[f64; 3]> {
    let Some(frame) = f.frame(radius_km) else {
        return vec![f.centre()];
    };
    (0..LINE_SAMPLES)
        .map(|i| {
            let along = i as f64 / (LINE_SAMPLES - 1) as f64 - 0.5;
            let (s, k) = (along * frame.span).sin_cos();
            let on = add(scale(frame.c, k), scale(frame.d, s));
            let m = meander_km(index, along, f64::from(f.length_km)) / radius_km;
            add(scale(on, m.cos()), scale(frame.n, m.sin()))
        })
        .collect()
}

/// Least distance between the segments `p0`–`p1` and `q0`–`q1` of space
/// (Ericson 2005, "Real-Time Collision Detection", 5.1.9); a chord of the
/// unit sphere stands for its short arc.
fn segment_distance(p0: [f64; 3], p1: [f64; 3], q0: [f64; 3], q1: [f64; 3]) -> f64 {
    let sub = |a: [f64; 3], b: [f64; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let (d1, d2, r) = (sub(p1, p0), sub(q1, q0), sub(p0, q0));
    let (a, e, f) = (dot(d1, d1), dot(d2, d2), dot(d2, r));
    let eps = 1e-18;
    let (s, t) = if a <= eps && e <= eps {
        (0.0, 0.0)
    } else if a <= eps {
        (0.0, (f / e).clamp(0.0, 1.0))
    } else {
        let c = dot(d1, r);
        if e <= eps {
            ((-c / a).clamp(0.0, 1.0), 0.0)
        } else {
            let b = dot(d1, d2);
            let denom = a * e - b * b;
            let mut s = if denom > eps {
                ((b * f - c * e) / denom).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let mut t = (b * s + f) / e;
            if t < 0.0 {
                t = 0.0;
                s = (-c / a).clamp(0.0, 1.0);
            } else if t > 1.0 {
                t = 1.0;
                s = ((b - c) / a).clamp(0.0, 1.0);
            }
            (s, t)
        }
    };
    let gap = sub(add(p0, scale(d1, s)), add(q0, scale(d2, t)));
    dot(gap, gap).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

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
            fractures: draw(252.1, 16.0, 30.0, 1).0,
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

    /// The fractures of a 16° cap round the south pole of a body of
    /// `radius_km` (an asked Enceladus's on 252.1 km), striking about the
    /// meridian `strike_deg`, laid out from `seed`.
    fn draw(
        radius_km: f64,
        region_deg: f64,
        strike_deg: f64,
        seed: u64,
    ) -> (Vec<Fracture>, Option<usize>) {
        let mut rng = StdRng::seed_from_u64(seed);
        fractures(radius_km, region_deg, true, strike_deg, &mut rng)
    }

    /// Angle, degrees, of the fracture `f`'s line in the plane tangent at
    /// the south pole of a body of `radius_km`, from `strike_deg`, in −90
    /// to 90.
    fn turn_deg(f: &Fracture, radius_km: f64, strike_deg: f64) -> f64 {
        let (a, b) = f.ends(radius_km);
        let flat = |e: [f64; 3]| [e[0] / -e[2], e[1] / -e[2]];
        let (a, b) = (flat(a), flat(b));
        let deg = (b[1] - a[1]).atan2(b[0] - a[0]).to_degrees();
        (deg - strike_deg + 90.0).rem_euclid(180.0) - 90.0
    }

    /// An asked Enceladus's fractures, over 300 seeds: three to six main
    /// ones, each turned by at most 15° off the common
    /// strike, all inside the 16° cap, their drawn lines at least
    /// [`MIN_CLEARANCE_KM`] apart (a branch and its parent aside), all in
    /// the young terrain; a branch on about a third of the bodies, and
    /// every count from three to six met, none on most seeds.
    #[test]
    fn enceladus_fractures_vary_from_one_seed_to_the_next() {
        let (r, region, strike) = (252.1, 16.0, 30.0);
        let pole = unit_at(-90.0, 0.0);
        let mut counts = [0usize; MAX_FRACTURES + 1];
        let (mut branches, mut turned) = (0, 0);
        for seed in 0..300 {
            let (f, parent) = draw(r, region, strike, seed);
            let mains = f.len() - usize::from(parent.is_some());
            assert!((3..=6).contains(&mains), "seed {seed}: {f:?}");
            counts[mains] += 1;
            branches += usize::from(parent.is_some());
            for x in &f {
                let (a, b) = x.ends(r);
                for e in [a, b] {
                    assert!(
                        angle(pole, e).to_degrees() <= region + 1e-3,
                        "seed {seed}: {x:?}"
                    );
                }
            }
            let turns: Vec<f64> = f[..mains].iter().map(|x| turn_deg(x, r, strike)).collect();
            for t in &turns {
                assert!(
                    t.abs() <= MAX_STRIKE_JITTER_DEG + 1e-3,
                    "seed {seed}: {turns:?}"
                );
            }
            let spread = turns.iter().fold(f64::MIN, |m, t| m.max(*t))
                - turns.iter().fold(f64::MAX, |m, t| m.min(*t));
            turned += usize::from(spread > 2.0);
            let clearance = min_clearance_km(&f, r, parent);
            assert!(clearance >= MIN_CLEARANCE_KM, "seed {seed}: {clearance} km");
            let young = YoungTerrain::of(
                &Geysers {
                    south: true,
                    fractures: f.clone(),
                },
                r,
            );
            for (i, x) in f.iter().enumerate() {
                assert!(
                    drawn_line(x, i, r).iter().all(|p| young.covers(*p)),
                    "seed {seed}"
                );
            }
            if seed < 6 {
                println!(
                    "seed {seed}: {mains} main, branch {}, turns {:?}°, clearance {clearance:.1} km",
                    parent.is_some(),
                    turns.iter().map(|t| t.round()).collect::<Vec<_>>()
                );
            }
        }
        println!(
            "main counts {:?}, branches {branches}/300, turned {turned}/300",
            &counts[3..=6]
        );
        assert!(counts[3..=6].iter().all(|&c| c > 0), "{counts:?}");
        assert!(counts.iter().all(|&c| c < 180), "{counts:?}");
        assert!((60..=140).contains(&branches), "{branches}");
        assert!(turned > 240, "{turned}");
    }

    /// The layout is the seed's: the same seed draws the same fractures,
    /// another seed others.
    #[test]
    fn the_fractures_follow_the_seed() {
        assert_eq!(draw(252.1, 16.0, 30.0, 7), draw(252.1, 16.0, 30.0, 7));
        assert_ne!(draw(252.1, 16.0, 30.0, 7).0, draw(252.1, 16.0, 30.0, 8).0);
    }

    /// A branch leaves its parent: its drawn line starts on the parent's,
    /// near one of its ends, and turns 20 to 40° off it.
    #[test]
    fn a_branch_starts_on_its_parent() {
        let r = 252.1;
        let (f, parent) = (0..)
            .map(|seed| draw(r, 16.0, 30.0, seed))
            .find(|(_, p)| p.is_some())
            .unwrap();
        let (p, b) = (parent.unwrap(), f.len() - 1);
        let start = drawn_line(&f[b], b, r)[0];
        let line = drawn_line(&f[p], p, r);
        let near = line
            .windows(2)
            .map(|w| segment_distance(w[0], w[1], start, start) * r)
            .fold(f64::INFINITY, f64::min);
        assert!(near < 1.0, "{near} km: {f:?}");
        let turn =
            (turn_deg(&f[b], r, 0.0) - turn_deg(&f[p], r, 0.0) + 90.0).rem_euclid(180.0) - 90.0;
        assert!((15.0..45.0).contains(&turn.abs()), "{turn}°");
        assert!(f[b].length_km < 0.55 * f[p].length_km, "{f:?}");
    }

    /// A hot body with a shell thin everywhere vents over the whole 30°
    /// cap: 5000 km in radius, it would hold over a hundred fractures; it
    /// stores at most twelve, spread across the cap, apart from each
    /// other.
    #[test]
    fn a_wide_region_stores_at_most_twelve_fractures_across_it() {
        let radius = 5000.0;
        let pole = unit_at(-90.0, 0.0);
        for seed in 0..5 {
            let (f, parent) = draw(radius, MAX_REGION_DEG, 0.0, seed);
            assert!(
                (MAX_FRACTURES - 1..=MAX_FRACTURES).contains(&f.len()),
                "{f:?}"
            );
            assert!(min_clearance_km(&f, radius, parent) >= MIN_CLEARANCE_KM);
            // The outermost centres reach most of the way to the edge.
            let outer = f
                .iter()
                .map(|x| angle(pole, x.centre()).to_degrees())
                .fold(0.0, f64::max);
            assert!(
                outer > 0.7 * MAX_REGION_DEG && outer < MAX_REGION_DEG,
                "{outer}"
            );
        }
    }

    #[test]
    fn a_tiny_region_still_has_one_fracture() {
        let (f, parent) = draw(252.0, 0.0, 0.0, 1);
        assert_eq!((f.len(), parent), (1, None));
        assert_eq!(f[0].length_km, 0.0);
    }
}
