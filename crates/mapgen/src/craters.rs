//! Impact craters, from the planet's surface age, gravity, atmosphere and
//! wet era (`docs/analysis/08-geology.md`, section 5).
//!
//! **How many.** The lunar chronology of Neukum gives the density of
//! craters of 1 km or more on a surface of a given age ([`crater_density`]);
//! sizes follow a power law, `N(>D) ∝ D⁻²`, and the density is capped at
//! [`SATURATION`], where each new crater erases as many as it adds. Ages are
//! drawn from the same curve, so most craters of an old surface are very
//! old.
//!
//! **Where.** Diameters are split into octaves `[2^k, 2^(k+1)]` km. Each
//! octave has a grid on the six faces of a cube projected on the sphere;
//! each cell holds a Poisson number of craters, expected from the cell's
//! solid angle, drawn from its own hash. The grids depend on the planet and
//! the seed, not on the map size, so a crater is the same on the base map
//! and in any detail tile that draws it.
//!
//! **Shape.** Simple craters are bowls about `D/5` deep; above a transition
//! diameter that scales as `1/g` (15 km on the Moon) they get a flat floor,
//! a central peak, then a peak ring, shallower than a bowl would be (Pike).
//! A raised rim and an ejecta blanket thinning as `(r/R)⁻³` surround them;
//! rims and ejecta are irregular.
//!
//! **Superposition.** Craters are applied in age order, oldest first. Inside
//! a crater the previous relief is replaced, keeping a small share of it;
//! outside, ejecta bury it. A crater formed in the wet era (older than the
//! end of the era of liquid water) is worn in proportion to the time it
//! spent exposed to water and to the erosion strength, and craters on the
//! floor of a sea that still fills the basins (water, ice-sealed, acid or
//! magma) leave nothing: sediment or lava buries them, and on plate worlds
//! the sea floor is recycled.
//! A thick atmosphere breaks up small impactors: no crater below about
//! `0.1 km × p^0.75`, fewer below ten times that.
use crate::biome::Biome;
use crate::geo::Grid;
use crate::noise::{smoothstep, stream, stream_seed};
use crate::world::WorldParams;
use rayon::prelude::*;
use std::f64::consts::PI;

/// Kilometres per elevation unit, calibrated on Earth-like worlds: their
/// median sea floor (−0.16) is about 4 km deep and their highest peaks
/// (0.30–0.39) about 8–10 km high.
pub const ELEVATION_KM: f32 = 25.0;

/// Cap of the density of craters of 1 km or more, per km²: about a tenth of
/// geometric saturation (1.15 D⁻²), the empirical equilibrium of the
/// lunar highlands. Reached by surfaces older than about 4.1 Ga.
pub const SATURATION: f64 = 0.115;

/// Smallest crater on the base map, in map pixels across.
const MIN_PIXELS: f64 = 4.0;
/// How far ejecta reach, in crater radii: the continuous blanket, about
/// one radius beyond the rim, fading out by three.
const EJECTA_RADII: f64 = 3.0;
/// Simple-to-complex transition on the Moon, km, and the Moon's gravity,
/// m/s²: the transition scales as `1/g`.
const MOON_TRANSITION_KM: f64 = 15.0;
const MOON_GRAVITY: f64 = 1.62;
const EARTH_GRAVITY: f64 = 9.81;
/// Wear of craters formed while water was liquid: a 20 km crater loses
/// `1/e` of its relief in 0.1 Ga of exposure at erosion 1; larger craters
/// last longer in proportion.
const WEAR_GA: f64 = 0.1;
const WEAR_KM: f64 = 20.0;
/// Topographic diffusion of crater profiles by small impacts, m² per Ma
/// (the Moon, Fassett & Thomson 2014): a crater of radius `R` loses `1/e`
/// of its relief once `κ t` reaches [`DIFFUSION_SHARE`] of `R²`. A 500 m
/// crater is subdued in about 0.6 Ga, a 2 km one barely in 4 Ga; craters of
/// the base map (tens of km) are untouched.
const DIFFUSIVITY_M2_PER_MA: f64 = 5.5;
const DIFFUSION_SHARE: f64 = 0.05;
/// Below this share of its relief left, a worn crater is not drawn.
const WORN_OUT: f64 = 0.03;
/// Share of the previous relief kept inside a new crater.
const INHERITANCE: f32 = 0.15;
/// Irregularity of the rim outline, as a share of the radius.
const RIM_JITTER: f64 = 0.06;
/// Irregularity of the ejecta blanket (lobes), as a share of its thickness.
const EJECTA_LOBES: f64 = 0.4;
/// Craters spanning more rows than this compute their rows in parallel.
const PARALLEL_ROWS: usize = 32;
/// Brightness of fresh craters (see [`CraterField::albedo`]): space
/// weathering darkens their ejecta and rays, `1/e` in this time (Ga), so
/// that they are all but gone in 1 Ga (the Moon's Copernican craters).
const RAY_FADE_GA: f64 = 0.25;
/// Below this share of its brightness left, a crater has none.
const RAY_GONE: f64 = 0.02;
/// Smallest crater whose ejecta and rays the base map draws, in pixels
/// across: its rays reach far beyond its rim.
pub const RAY_MIN_PIXELS: f64 = 1.0;
/// Brightness of a fresh crater's floor, of its ejecta at the rim, and of
/// its rays near the rim (albedo shift, see [`CraterField::albedo`]).
const FRESH_FLOOR: f32 = 0.25;
const FRESH_EJECTA: f32 = 0.5;
const FRESH_RAYS: f32 = 0.5;
/// Ray lengths, in crater radii (Tycho's reach about 35).
const RAY_LENGTH: (f64, f64) = (8.0, 30.0);
/// Smallest crater a 1 bar atmosphere lets through, km; it scales as
/// `p^0.75` (Venus: about 3 km at 92 bar).
const AIR_FILTER_KM: f64 = 0.1;

/// Cumulative density of craters of 1 km or more on a surface `age_ga`
/// old, per km² (Neukum, Ivanov & Hartmann 2001): a steep decline until
/// about 3.5 Ga, then a constant flux. 0.063 at 4 Ga.
pub fn crater_density(age_ga: f64) -> f64 {
    5.44e-14 * ((6.93 * age_ga).exp() - 1.0) + 8.38e-4 * age_ga
}

/// The age of the crater at `fraction` of the crater count of a surface
/// `surface_age_ga` old, counting from the youngest: the inverse of the
/// chronology.
fn age_at(fraction: f64, surface_age_ga: f64) -> f64 {
    let target = fraction * crater_density(surface_age_ga);
    let (mut lo, mut hi) = (0.0, surface_age_ga);
    for _ in 0..40 {
        let mid = 0.5 * (lo + hi);
        if crater_density(mid) < target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

/// One impact crater.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Crater {
    /// Centre on the unit sphere.
    pub centre: [f64; 3],
    pub diameter_km: f64,
    /// Age in Ga before present.
    pub age_ga: f64,
    /// Hash of the crater, for its rim and ejecta irregularities.
    shape: u64,
}

/// SplitMix64 finaliser.
fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A small deterministic generator for the draws of one lattice cell.
struct Rng(u64);

impl Rng {
    fn cell(seed: u32, octave: i32, face: u32, i: u32, j: u32) -> Self {
        let mut z = u64::from(seed);
        for v in [
            octave as i64 as u64,
            u64::from(face),
            u64::from(i),
            u64::from(j),
        ] {
            z = mix(z ^ v.wrapping_add(0x9E37_79B9_7F4A_7C15));
        }
        Rng(z)
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        mix(self.0)
    }

    /// Uniform in `[0, 1)`.
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Poisson draw of mean `lambda` (Knuth: means here are a few at most).
    fn poisson(&mut self, lambda: f64) -> u32 {
        let limit = (-lambda).exp();
        let (mut k, mut p) = (0, self.unit());
        while p > limit {
            k += 1;
            p *= self.unit();
        }
        k
    }
}

/// The point of cube face `face` at gnomonic coordinates `(a, b)` in
/// `[-1, 1]²`, on the unit sphere. The six faces cover the sphere once.
fn cube_point(face: u32, a: f64, b: f64) -> [f64; 3] {
    let p = match face {
        0 => [1.0, a, b],
        1 => [-1.0, -a, b],
        2 => [-a, 1.0, b],
        3 => [a, -1.0, b],
        4 => [a, b, 1.0],
        _ => [a, -b, -1.0],
    };
    let n = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
    [p[0] / n, p[1] / n, p[2] / n]
}

/// The cube face holding the unit vector `p` and its gnomonic coordinates
/// on it, in `[-1, 1]²`: the inverse of [`cube_point`].
fn cube_cell(p: [f64; 3]) -> (u32, f64, f64) {
    let [x, y, z] = p;
    let (ax, ay, az) = (x.abs(), y.abs(), z.abs());
    if ax >= ay && ax >= az {
        if x > 0.0 {
            (0, y / x, z / x)
        } else {
            (1, y / x, -z / x)
        }
    } else if ay >= az {
        if y > 0.0 {
            (2, -x / y, z / y)
        } else {
            (3, -x / y, -z / y)
        }
    } else if z > 0.0 {
        (4, x / z, y / z)
    } else {
        (5, -x / z, y / z)
    }
}

/// Craters per km² of an octave of a population: `N(>D) − N(>2D)`.
fn per_km2(population: Population, octave: i32) -> f64 {
    let low = 2f64.powi(octave);
    0.75 * population.density / (low * low)
}

/// Cells per face edge of an octave's grid on a planet of `radius_km`:
/// about two craters per cell, at least one cell per face.
fn cells(population: Population, radius_km: f64, octave: i32) -> u32 {
    let face_area = 4.0 * PI * radius_km * radius_km / 6.0;
    ((per_km2(population, octave) * face_area / 2.0)
        .sqrt()
        .ceil() as u32)
        .max(1)
}

/// Sorts craters oldest first; the shape hash breaks ties, so the order is
/// total.
fn sort_oldest_first(craters: &mut [Crater]) {
    craters.sort_by(|a, b| {
        b.age_ga
            .total_cmp(&a.age_ga)
            .then_with(|| a.shape.cmp(&b.shape))
    });
}

/// Solid angle of the gnomonic rectangle `[a0, a1] × [b0, b1]` of a cube face.
fn solid_angle(a0: f64, a1: f64, b0: f64, b1: f64) -> f64 {
    let corner = |a: f64, b: f64| (a * b).atan2((1.0 + a * a + b * b).sqrt());
    corner(a1, b1) - corner(a0, b1) - corner(a1, b0) + corner(a0, b0)
}

/// The craters of a world, oldest first, with what drawing them needs.
pub struct CraterField {
    pub craters: Vec<Crater>,
    /// The lava floods of large basins formed by the epoch, oldest first.
    pub floods: Vec<Flood>,
    radius_km: f64,
    /// Simple-to-complex transition diameter, km.
    transition_km: f64,
    /// Absolute elevation of the sea surface.
    waterline: f32,
    /// Whether the basins hold a sea today (water, ice-sealed, acid or
    /// magma: the preset's ocean is still a water biome): craters on its
    /// floor vanish.
    sea: bool,
    /// End of the wet era, Ga before present.
    wet_until_ga: f64,
    erosion: f64,
    /// The craters formed so far, and those still to come.
    past: Population,
    future: Population,
    /// Smallest crater the atmosphere lets through, km.
    air_km: f64,
    /// Whether fresh craters keep bright rays: worlds without liquid water
    /// today, where only space weathering darkens them (wind, rain and air
    /// erase them elsewhere).
    rays: bool,
    /// For a planet tidally locked to its star, since when, Ga before
    /// present: craters younger than that favour its leading hemisphere.
    locked_since_ga: Option<f64>,
    /// The epoch shown, Ga before present (negative in the future): only
    /// the craters formed by then exist, worn for the time since.
    epoch_ga: f64,
}

/// Lava flooding the floor of a large basin (a lunar mare): the floor is
/// filled flat up to a level between the floor and the rim.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flood {
    pub basin: Crater,
    /// When the lava came, Ga before present.
    pub age_ga: f64,
    /// Fill level, as a share of the way from the floor to the rim.
    fill: f32,
}

/// Basins smaller than this, km, or than [`MARE_TRANSITIONS`] times the
/// simple-to-complex transition, are not flooded (the Moon's maria fill
/// basins of 300 km and more, and a few craters of about 100 km).
const MARE_MIN_KM: f64 = 100.0;
const MARE_TRANSITIONS: f64 = 6.0;
/// Share of the large basins flooded on a world without volcanism, and
/// on a fully volcanic one.
const MARE_SHARE: (f64, f64) = (0.3, 0.9);
/// Fill level range, from the floor (0) to the rim (1).
const MARE_FILL: (f64, f64) = (0.4, 0.75);
/// Albedo shift of the basalt plains: dark (see [`CraterField::albedo`]).
pub const MARE_ALBEDO: f32 = -0.35;
/// Darkens the albedo shift of the pixels flooded by lava (`mare`, from
/// [`CraterField::apply`]; empty for none): basalt plains, with the rays
/// of younger craters still bright over them.
pub fn darken_maria(albedo: &mut [f32], mare: &[bool]) {
    for (a, &m) in albedo.iter_mut().zip(mare) {
        if m {
            *a = (*a + MARE_ALBEDO).clamp(-1.0, 1.0);
        }
    }
}

/// Lava fills the basin out to this share of its radius.
const MARE_EXTENT: f64 = 0.9;

/// Craters per area on the leading hemisphere of a tidally locked planet
/// against the trailing one: it sweeps up more impactors, as the Moon's
/// leading side does (models of near-Earth objects give 1.14 to 1.32 for
/// the Moon, observations more).
const LEADING_RATIO: f64 = 1.3;
/// The apex of a locked planet's orbital motion: on the equator, 90° west
/// of the point under its star (at longitude 180°, the map's centre), as
/// the Moon's apex lies 90° west of the point under the Earth.
const APEX: [f64; 3] = [0.0, 1.0, 0.0];

/// The share of craters kept at an angle `cos_apex` (cosine) from the apex,
/// formed while the planet was locked: `(1 + k cos γ) / (1 + k)` with
/// `k = (ratio − 1) / (ratio + 1)`, so that the leading-to-trailing ratio
/// is [`LEADING_RATIO`] and a density raised by `1 + k` keeps its mean.
fn leading_share(cos_apex: f64) -> f64 {
    let k = (LEADING_RATIO - 1.0) / (LEADING_RATIO + 1.0);
    (1.0 + k * cos_apex) / (1.0 + k)
}

/// How far ahead the craters still to come are drawn, Ga.
pub const FUTURE_GA: f64 = 1.0;

/// Craters of 1 km or more formed per km² and per Ga at today's flux: the
/// constant term of the chronology ([`crater_density`]).
const FLUX_PER_GA: f64 = 8.38e-4;

/// One population of craters, with its own lattice and seed: those formed
/// so far, or those still to come.
#[derive(Clone, Copy, Debug)]
struct Population {
    seed: u32,
    /// Craters of 1 km or more per km².
    density: f64,
    ages: Ages,
}

#[derive(Clone, Copy, Debug)]
enum Ages {
    /// Drawn from the chronology over a surface this old (Ga).
    Past { surface_age_ga: f64 },
    /// Uniform over the next `span_ga` (negative ages), at today's flux.
    Future { span_ga: f64 },
}

/// A latitude-longitude grid of pixel centres, as a detail tile's: row `y`
/// at latitude `lat0 − y × dlat`, column `x` at longitude `lon0 + x × dlon`
/// (radians).
#[derive(Clone, Copy, Debug)]
pub struct Window {
    pub width: usize,
    pub height: usize,
    pub lat0: f64,
    pub dlat: f64,
    pub lon0: f64,
    pub dlon: f64,
}

/// Which craters a pass draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Era {
    /// Formed while water was liquid (older than the end of the wet era):
    /// drawn before the hydraulic erosion.
    Wet,
    /// Formed since: drawn after it, fresh.
    Dry,
}

/// Smallest crater the base map draws, km: [`MIN_PIXELS`] pixels across at
/// the equator.
pub fn base_min_diameter_km(grid: &Grid) -> f64 {
    MIN_PIXELS * grid.circumference_km as f64 / grid.width as f64
}

impl CraterField {
    /// The crater field of the world `p` without any crater yet: what
    /// drawing and finding them needs (see [`Self::near`]).
    pub fn context(p: &WorldParams) -> Self {
        Self::context_at(p, 0.0)
    }

    /// Like [`Self::context`], for the surface at `epoch_ga` (Ga before
    /// present, negative in the future, down to [`FUTURE_GA`]).
    pub fn context_at(p: &WorldParams, epoch_ga: f64) -> Self {
        let radius_km = p.circumference_km as f64 / (2.0 * PI);
        let star = crate::star::StarInfo::new(p.star, p.orbit_au);
        let locked_since_ga = star.tidally_locked.then(|| {
            f64::from((crate::star::AGE_GYR.min(star.lifetime_gyr) - star.locking_gyr).max(0.0))
        });
        // A locked planet draws more craters, and keeps a share of them by
        // their angle to its apex (see `leading_share`): the same mean.
        let raise = if locked_since_ga.is_some() {
            1.0 + (LEADING_RATIO - 1.0) / (LEADING_RATIO + 1.0)
        } else {
            1.0
        };
        let gravity = EARTH_GRAVITY * p.gravity_modifier() as f64;
        CraterField {
            craters: Vec::new(),
            floods: Vec::new(),
            radius_km,
            transition_km: MOON_TRANSITION_KM * MOON_GRAVITY / gravity,
            waterline: p.sea_level + p.planet.rules.waterline,
            sea: [Biome::DeepOcean, Biome::Ocean]
                .iter()
                .any(|&b| p.planet.remap(b).is_water()),
            wet_until_ga: p.planet.geology.wet_until_ga(p.pressure_bar) as f64,
            erosion: p.erosion as f64,
            past: Population {
                seed: stream_seed(p.seed, stream::CRATERS),
                density: crater_density(p.surface_age_ga as f64).min(SATURATION) * raise,
                ages: Ages::Past {
                    surface_age_ga: p.surface_age_ga as f64,
                },
            },
            // A world without craters (surface age 0, or made before them)
            // gets none in the future either.
            future: Population {
                seed: stream_seed(p.seed, stream::FUTURE_CRATERS),
                density: if p.surface_age_ga > 0.0 {
                    FLUX_PER_GA * FUTURE_GA * raise
                } else {
                    0.0
                },
                ages: Ages::Future { span_ga: FUTURE_GA },
            },
            air_km: AIR_FILTER_KM * (p.pressure_bar.max(0.0) as f64).powf(0.75),
            rays: p.pressure_bar < crate::atmosphere::LIQUID_WATER_BAR,
            locked_since_ga,
            epoch_ga: epoch_ga.max(-FUTURE_GA),
        }
    }

    /// The populations whose craters may exist at the epoch: the future
    /// only before the present.
    fn populations(&self) -> impl Iterator<Item = Population> {
        let future = (self.epoch_ga < 0.0).then_some(self.future);
        std::iter::once(self.past)
            .chain(future)
            .filter(|p| p.density > 0.0)
    }

    /// Whether a crater has formed by the epoch.
    fn formed(&self, crater: &Crater) -> bool {
        crater.age_ga >= self.epoch_ga
    }

    /// Every crater of the world `p` of `min_diameter_km` or more that has
    /// survived to the present, oldest first.
    pub fn new(p: &WorldParams, min_diameter_km: f64) -> Self {
        Self::new_at(p, min_diameter_km, 0.0)
    }

    /// Like [`Self::new`], with the craters formed by `epoch_ga` (see
    /// [`Self::context_at`]).
    pub fn new_at(p: &WorldParams, min_diameter_km: f64, epoch_ga: f64) -> Self {
        let mut field = Self::context_at(p, epoch_ga);
        let mut craters = Vec::new();
        for population in field.populations() {
            for octave in field.octaves(min_diameter_km, f64::INFINITY) {
                let cells = cells(population, field.radius_km, octave);
                let found: Vec<Crater> = (0..6 * cells)
                    .into_par_iter()
                    .flat_map_iter(|row| {
                        let (face, i) = (row / cells, row % cells);
                        let field = &field;
                        (0..cells)
                            .flat_map(move |j| field.cell_craters(population, octave, face, i, j))
                    })
                    .filter(|c| c.diameter_km >= min_diameter_km && field.formed(c))
                    .collect();
                craters.extend(found);
            }
        }
        sort_oldest_first(&mut craters);
        field.floods = field.floods_of(p, &craters);
        field.craters = craters;
        field
    }

    /// The lava floods of the large basins among `craters` (oldest first),
    /// formed by the epoch: within the preset's window of maria
    /// (`[geology] maria`), after the basin itself, for a share of the
    /// basins that grows with the volcanism.
    fn floods_of(&self, p: &WorldParams, craters: &[Crater]) -> Vec<Flood> {
        let Some([start, end]) = p.planet.geology.maria else {
            return Vec::new();
        };
        let (start, end) = (f64::from(start), f64::from(end));
        let share = MARE_SHARE.0
            + (MARE_SHARE.1 - MARE_SHARE.0) * f64::from(p.volcanic_intensity.clamp(0.0, 1.0));
        let smallest = MARE_MIN_KM.max(MARE_TRANSITIONS * self.transition_km);
        let mut floods: Vec<Flood> = craters
            .iter()
            .filter(|c| c.diameter_km >= smallest && c.age_ga > end)
            .filter_map(|&basin| {
                // A stream of its own, apart from the crater's other draws.
                let mut rng = Rng(mix(basin.shape ^ 0x4D41_5245));
                if rng.unit() >= share {
                    return None;
                }
                let latest = start.min(basin.age_ga);
                let age_ga = end + (latest - end) * rng.unit();
                let fill = (MARE_FILL.0 + (MARE_FILL.1 - MARE_FILL.0) * rng.unit()) as f32;
                Some(Flood {
                    basin,
                    age_ga,
                    fill,
                })
            })
            .filter(|f| f.age_ga >= self.epoch_ga)
            .collect();
        floods.sort_by(|a, b| b.age_ga.total_cmp(&a.age_ga));
        floods
    }

    /// The craters with a diameter in `[min_km, max_km)` whose ejecta reach
    /// the spherical cap of `angle` radians around `centre`, oldest first:
    /// the same craters as [`Self::new`], found from the lattice cells
    /// around the cap only.
    pub fn near(&self, centre: [f64; 3], angle: f64, min_km: f64, max_km: f64) -> Vec<Crater> {
        if min_km >= max_km {
            return Vec::new();
        }
        let mut found = Vec::new();
        for (population, octave) in self
            .populations()
            .flat_map(|p| self.octaves(min_km, max_km).map(move |o| (p, o)))
        {
            let cells = cells(population, self.radius_km, octave);
            // Widest reach of the octave's craters.
            let reach =
                EJECTA_RADII * (1.0 + 2.0 * RIM_JITTER) * 2f64.powi(octave) / self.radius_km;
            let cap = angle + reach;
            // Points of the cap, closer than a quarter of the smallest cell
            // (cells shrink to a third of `2 / cells` at face corners).
            let spacing = (2.0 / f64::from(cells) / 3.0 / 4.0).min(cap / 4.0);
            let mut keys: Vec<(u32, u32, u32)> = Vec::new();
            let lat_c = centre[2].clamp(-1.0, 1.0).asin();
            let lon_c = centre[1].atan2(centre[0]);
            let rows = (2.0 * cap / spacing).ceil() as i64;
            for r in 0..=rows {
                let lat = (lat_c - cap + r as f64 * spacing).clamp(-PI / 2.0, PI / 2.0);
                let cos_lat = lat.cos();
                let half = if lat.abs() + cap >= PI / 2.0 || cap >= PI / 2.0 {
                    PI
                } else {
                    (cap.sin() / cos_lat).min(1.0).asin()
                };
                let step = spacing / cos_lat.max(1e-6);
                let n = ((2.0 * half / step).ceil() as i64).min(1 << 20);
                for k in 0..=n {
                    let lon = lon_c - half + k as f64 * step;
                    let p = [cos_lat * lon.cos(), cos_lat * lon.sin(), lat.sin()];
                    let (face, a, b) = cube_cell(p);
                    let index =
                        |t: f64| (((t + 1.0) / 2.0 * f64::from(cells)) as u32).min(cells - 1);
                    keys.push((face, index(a), index(b)));
                }
            }
            keys.sort_unstable();
            keys.dedup();
            let (cos_angle, sin_angle) = (angle.cos(), angle.sin());
            found.extend(
                keys.into_iter()
                    .flat_map(|(face, i, j)| self.cell_craters(population, octave, face, i, j))
                    .filter(|c| (min_km..max_km).contains(&c.diameter_km) && self.formed(c))
                    .filter(|c| {
                        let reach = self.reach(c);
                        let dot = c.centre[0] * centre[0]
                            + c.centre[1] * centre[1]
                            + c.centre[2] * centre[2];
                        angle + reach >= PI
                            || dot > cos_angle * reach.cos() - sin_angle * reach.sin()
                    }),
            );
        }
        sort_oldest_first(&mut found);
        found
    }

    /// Octaves holding diameters in `[min_km, max_km)`, up to basins of
    /// about the planet's radius (South Pole–Aitken is 1.4 Moon radii,
    /// Hellas 0.7 Mars radii).
    fn octaves(&self, min_km: f64, max_km: f64) -> std::ops::RangeInclusive<i32> {
        let first = min_km.max(1e-3).log2().floor() as i32;
        let largest = self.radius_km.log2().floor() as i32;
        // Octave k holds [2^k, 2^(k+1)): it has diameters below max_km when
        // 2^k < max_km.
        let below_max = if max_km.is_finite() {
            max_km.log2().ceil() as i32 - 1
        } else {
            largest
        };
        first..=largest.min(below_max)
    }

    /// The craters of one lattice cell of a population that survived the
    /// atmosphere, whatever their size within the octave and their age.
    fn cell_craters(
        &self,
        population: Population,
        octave: i32,
        face: u32,
        i: u32,
        j: u32,
    ) -> impl Iterator<Item = Crater> {
        let cells = cells(population, self.radius_km, octave);
        let step = 2.0 / f64::from(cells);
        let (a0, b0) = (-1.0 + step * f64::from(i), -1.0 + step * f64::from(j));
        let omega = solid_angle(a0, a0 + step, b0, b0 + step);
        let mut rng = Rng::cell(population.seed, octave, face, i, j);
        let area = self.radius_km * self.radius_km;
        let count = rng.poisson(per_km2(population, octave) * omega * area);
        let (low, air_km, locked_since) = (2f64.powi(octave), self.air_km, self.locked_since_ga);
        (0..count).filter_map(move |_| {
            let centre = cube_point(face, a0 + step * rng.unit(), b0 + step * rng.unit());
            // Power law within the octave: N(>D) ∝ D⁻².
            let diameter_km = low / (1.0 - 0.75 * rng.unit()).sqrt();
            let age_ga = match population.ages {
                Ages::Past { surface_age_ga } => age_at(rng.unit(), surface_age_ga),
                // In [-span, 0): none forms at the present itself.
                Ages::Future { span_ga } => -span_ga * (1.0 - rng.unit()),
            };
            let survives = air_km <= 0.0
                || rng.unit()
                    < f64::from(smoothstep(
                        air_km as f32,
                        (10.0 * air_km) as f32,
                        diameter_km as f32,
                    ));
            let shape = rng.next();
            // On a locked planet, the share of the craters its leading
            // hemisphere favours (a draw of their own: unlocked worlds keep
            // their craters).
            let survives = match locked_since {
                Some(since) => {
                    let keep = if age_ga < since {
                        let cos_apex =
                            centre[0] * APEX[0] + centre[1] * APEX[1] + centre[2] * APEX[2];
                        leading_share(cos_apex)
                    } else {
                        // Formed while it still rotated: anywhere alike.
                        leading_share(0.0)
                    };
                    rng.unit() < keep && survives
                }
                None => survives,
            };
            survives.then_some(Crater {
                centre,
                diameter_km,
                age_ga,
                shape,
            })
        })
    }

    /// Share of its relief a crater keeps: worn by liquid water for the
    /// time it spent in the wet era (with erosion), and by the slow
    /// diffusion of small impacts since it formed.
    fn wear(&self, crater: &Crater) -> f64 {
        let radius_m = 500.0 * crater.diameter_km;
        // Diffused since it formed, until the epoch shown.
        let since_ga = (crater.age_ga - self.epoch_ga).max(0.0);
        let diffused = DIFFUSIVITY_M2_PER_MA * since_ga * 1000.0;
        let mut wear = (-diffused / (DIFFUSION_SHARE * radius_m * radius_m)).exp();
        let exposure = (crater.age_ga - self.wet_until_ga).max(0.0);
        if exposure > 0.0 && self.erosion > 0.0 {
            let lasts = WEAR_GA * crater.diameter_km / WEAR_KM / self.erosion;
            wear *= (-exposure / lasts).exp();
        }
        wear
    }

    /// Adds the relief of `craters` to `relief`, one value per pixel of
    /// `window`, each crater's profile added to whatever is there rather
    /// than replacing it.
    ///
    /// The detail tiles draw the craters too small for the base map this
    /// way: added profiles depend on the position only, not on the order or
    /// on what the window holds, so neighbouring tiles agree at their
    /// borders. Small cavities seldom overlap (about 7 % of the ground per
    /// octave at saturation), so adding them digs no pits. `ground` gives
    /// the elevation at a crater, for the sea rule; `gain` scales its relief.
    pub fn add_relief(
        &self,
        craters: &[Crater],
        window: &Window,
        ground: impl Fn(&Crater) -> f32,
        gain: impl Fn(&Crater) -> f32,
        relief: &mut [f32],
    ) {
        let (w, h) = (window.width, window.height);
        debug_assert_eq!(relief.len(), w * h);
        let lat_of = |y: usize| (window.lat0 - y as f64 * window.dlat).clamp(-PI / 2.0, PI / 2.0);
        let columns: Vec<(f64, f64)> = (0..w)
            .map(|x| {
                let lon = window.lon0 + x as f64 * window.dlon;
                (lon.cos(), lon.sin())
            })
            .collect();
        // What each drawn crater needs, prepared once.
        struct Drawn {
            c: [f64; 3],
            radius: f64,
            reach: f64,
            cos_reach: f64,
            east: [f64; 3],
            north: [f64; 3],
            profile: Profile,
            wear: f32,
            /// Longitude from the window's first column, in (-π, π].
            lon: f64,
            rows: (usize, usize),
        }
        let drawn: Vec<Drawn> = craters
            .iter()
            .filter_map(|crater| {
                let wear = self.wear(crater);
                if wear < WORN_OUT || (self.sea && ground(crater) < self.waterline) {
                    return None;
                }
                let c = crater.centre;
                let reach = self.reach(crater);
                let lat = c[2].clamp(-1.0, 1.0).asin();
                let y0 = ((window.lat0 - (lat + reach)) / window.dlat)
                    .floor()
                    .max(0.0) as usize;
                let y1 = ((window.lat0 - (lat - reach)) / window.dlat).ceil();
                if y1 < 0.0 || y0 >= h {
                    return None;
                }
                let (east, north) = tangent_basis(c);
                Some(Drawn {
                    c,
                    radius: 0.5 * crater.diameter_km / self.radius_km,
                    reach,
                    cos_reach: reach.cos(),
                    east,
                    north,
                    profile: Profile::new(crater, self.transition_km),
                    wear: wear as f32 * gain(crater),
                    lon: (c[1].atan2(c[0]) - window.lon0 + PI).rem_euclid(2.0 * PI) - PI,
                    rows: (y0, (y1 as usize).min(h - 1)),
                })
            })
            .collect();
        // Rows in parallel: a crater only adds to the pixels of its rows.
        relief.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
            let row_lat = lat_of(y);
            let (cos_lat, sin_lat) = (row_lat.cos(), row_lat.sin());
            for d in drawn.iter().filter(|d| (d.rows.0..=d.rows.1).contains(&y)) {
                let (x0, x1) = if row_lat.abs() + d.reach >= PI / 2.0 {
                    (0, w - 1)
                } else {
                    let half = (d.reach.sin() / cos_lat).min(1.0).asin();
                    let x0 = ((d.lon - half) / window.dlon).floor().max(0.0);
                    let x1 = ((d.lon + half) / window.dlon).ceil();
                    if x1 < 0.0 || x0 as usize >= w {
                        continue;
                    }
                    (x0 as usize, (x1 as usize).min(w - 1))
                };
                let c = d.c;
                for (value, &(cos_lon, sin_lon)) in row[x0..=x1].iter_mut().zip(&columns[x0..=x1]) {
                    let v = [cos_lat * cos_lon, cos_lat * sin_lon, sin_lat];
                    let dot = (v[0] * c[0] + v[1] * c[1] + v[2] * c[2]).clamp(-1.0, 1.0);
                    if dot < d.cos_reach {
                        continue;
                    }
                    let r = dot.acos() / d.radius;
                    let along = |b: [f64; 3]| v[0] * b[0] + v[1] * b[1] + v[2] * b[2];
                    let (e, n) = (along(d.east), along(d.north));
                    let len = (e * e + n * n).sqrt();
                    let direction = if len > 1e-12 {
                        (e / len, n / len)
                    } else {
                        (1.0, 0.0)
                    };
                    *value += d.profile.blend(r, direction, 0.0, 0.0, d.wear);
                }
            }
        });
    }

    /// Whether a crater belongs to `era`.
    fn in_era(&self, crater: &Crater, era: Era) -> bool {
        (crater.age_ga > self.wet_until_ga) == (era == Era::Wet)
    }

    /// The brightness of the fresh craters (and their ejecta and rays) on
    /// the base map: an albedo shift per pixel, 0 for none, up to 1 at the
    /// rim of a crater just formed, fading with the time since it formed
    /// until the epoch. Profiles add up, in any order. Empty of craters
    /// (all zeros) on worlds with air enough for liquid water.
    ///
    /// The field holds the craters to light: [`Self::new_at`] down to
    /// [`RAY_MIN_PIXELS`], since even a pixel-wide crater has rays many
    /// pixels long.
    pub fn albedo(&self, grid: &Grid) -> Vec<f32> {
        let (w, h) = (grid.width as usize, grid.height as usize);
        let mut albedo = vec![0.0f32; w * h];
        if !self.rays {
            return albedo;
        }
        let columns: Vec<(f64, f64)> = (0..w)
            .map(|x| {
                let lon = (x as f64 + 0.5) / w as f64 * 2.0 * PI;
                (lon.cos(), lon.sin())
            })
            .collect();
        let lit: Vec<Fresh> = self
            .craters
            .iter()
            .filter(|crater| self.formed(crater))
            .filter_map(|crater| {
                let since = crater.age_ga - self.epoch_ga;
                let fresh = (-since / RAY_FADE_GA).exp();
                (fresh >= RAY_GONE).then(|| Fresh::new(crater, fresh as f32, self.radius_km, h))
            })
            .collect();
        albedo.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
            let lat = grid.latitude(y as u32);
            let (cos_lat, sin_lat) = (lat.cos(), lat.sin());
            for f in lit.iter().filter(|f| (f.rows.0..=f.rows.1).contains(&y)) {
                let columns_of = if lat.abs() + f.reach >= PI / 2.0 {
                    None
                } else {
                    let half = (f.reach.sin() / cos_lat).min(1.0).asin();
                    let to_col = |lon: f64| lon / (2.0 * PI) * w as f64 - 0.5;
                    let x0 = to_col(f.lon - half).floor() as i64;
                    let x1 = to_col(f.lon + half).ceil() as i64;
                    (x1 - x0 + 1 < w as i64).then_some((x0, x1))
                };
                let (x0, x1) = columns_of.unwrap_or((0, w as i64 - 1));
                for xi in x0..=x1 {
                    let x = xi.rem_euclid(w as i64) as usize;
                    let (cos_lon, sin_lon) = columns[x];
                    let v = [cos_lat * cos_lon, cos_lat * sin_lon, sin_lat];
                    row[x] += f.at(v);
                }
            }
        });
        for a in &mut albedo {
            *a = a.clamp(-1.0, 1.0);
        }
        albedo
    }

    /// Angular reach of a crater's ejecta, radians: no pixel farther from
    /// its centre changes.
    fn reach(&self, crater: &Crater) -> f64 {
        EJECTA_RADII * (1.0 + 2.0 * RIM_JITTER) * 0.5 * crater.diameter_km / self.radius_km
    }

    /// Draws the craters of `era` into `elevation`, as if one by one in age
    /// order, oldest first.
    ///
    /// Craters are grouped in waves: each goes in the wave after the latest
    /// older crater it overlaps, so the craters of a wave share no pixel and
    /// read only their own. A wave is computed in parallel, then written:
    /// the result is the same as drawing every crater in turn.
    pub fn apply(&self, elevation: &mut [f32], grid: &Grid, era: Era) -> Vec<bool> {
        let (w, h) = (grid.width as usize, grid.height as usize);
        let rows: Vec<(f64, f64)> = (0..grid.height)
            .map(|y| {
                let lat = grid.latitude(y);
                (lat.cos(), lat.sin())
            })
            .collect();
        let columns: Vec<(f64, f64)> = (0..w)
            .map(|x| {
                let lon = (x as f64 + 0.5) / w as f64 * 2.0 * PI;
                (lon.cos(), lon.sin())
            })
            .collect();
        let pixels = Pixels {
            w,
            h,
            rows: &rows,
            columns: &columns,
        };
        // Impacts and lava floods, in the order they happened. Floods are
        // all drawn after the erosion: lava plains wear little, and their
        // extent can then be found again at any epoch.
        let mut events: Vec<(f64, Event)> = self
            .craters
            .iter()
            .filter(|c| self.in_era(c, era))
            .filter_map(|c| {
                let wear = self.wear(c);
                (wear >= WORN_OUT).then_some((c.age_ga, Event::Impact(c, wear as f32)))
            })
            .collect();
        if era == Era::Dry {
            events.extend(self.floods.iter().map(|f| (f.age_ga, Event::Flood(f))));
        }
        events.sort_by(|a, b| b.0.total_cmp(&a.0));
        // A flood covers less than its basin's ejecta: the basin's reach
        // keeps waves apart for it too.
        let drawn: Vec<(&Crater, f32)> = events
            .iter()
            .map(|(_, e)| match *e {
                Event::Impact(c, wear) => (c, wear),
                Event::Flood(f) => (&f.basin, 1.0),
            })
            .collect();
        let mut mare = Vec::new();
        for wave in self.waves(&drawn) {
            let updates: Vec<(bool, Vec<(u32, f32)>)> = wave
                .par_iter()
                .map(|&i| match events[i].1 {
                    Event::Impact(crater, wear) => {
                        (false, self.draw(crater, wear, elevation, &pixels))
                    }
                    Event::Flood(flood) => (true, self.flood(flood, elevation, &pixels)),
                })
                .collect();
            for (flooded, changes) in updates {
                if flooded && mare.is_empty() {
                    mare = vec![false; w * h];
                }
                for (i, value) in changes {
                    elevation[i as usize] = value;
                    if flooded {
                        mare[i as usize] = true;
                    }
                }
            }
        }
        mare
    }

    /// The pixels a lava flood raises, and their new elevation: the floor
    /// of its basin filled flat up to its level, between the lowest of the
    /// floor and the mean of the rim, as they are when the lava comes.
    fn flood(&self, flood: &Flood, elevation: &[f32], px: &Pixels) -> Vec<(u32, f32)> {
        let c = flood.basin.centre;
        let radius = 0.5 * flood.basin.diameter_km / self.radius_km;
        let (east, north) = tangent_basis(c);
        let ring = |r: f64, n: u32| {
            let (s, k) = ((r * radius).sin(), (r * radius).cos());
            (0..n).map(move |i| {
                let phi = f64::from(i) * 2.0 * PI / f64::from(n);
                let (sp, cp) = phi.sin_cos();
                [
                    k * c[0] + s * (cp * east[0] + sp * north[0]),
                    k * c[1] + s * (cp * east[1] + sp * north[1]),
                    k * c[2] + s * (cp * east[2] + sp * north[2]),
                ]
            })
        };
        let floor = ring(0.3, 8)
            .chain(std::iter::once(c))
            .map(|p| px.sample(elevation, p))
            .fold(f32::MAX, f32::min);
        let rim = ring(1.0, 12).map(|p| px.sample(elevation, p)).sum::<f32>() / 12.0;
        if rim <= floor {
            return Vec::new();
        }
        let level = floor + flood.fill * (rim - floor);
        let reach = MARE_EXTENT * radius;
        let cos_reach = reach.cos();
        let lat = c[2].clamp(-1.0, 1.0).asin();
        let lon = c[1].atan2(c[0]).rem_euclid(2.0 * PI);
        let to_row = |lat: f64| (PI / 2.0 - lat) / PI * px.h as f64 - 0.5;
        let y0 = to_row(lat + reach).floor().max(0.0) as usize;
        let y1 = (to_row(lat - reach).ceil().max(0.0) as usize).min(px.h - 1);
        let mut out = Vec::new();
        for y in y0..=y1 {
            let (cos_lat, sin_lat) = px.rows[y];
            let row_lat = sin_lat.atan2(cos_lat);
            let columns = if row_lat.abs() + reach >= PI / 2.0 {
                None
            } else {
                let half = (reach.sin() / cos_lat).min(1.0).asin();
                let to_col = |lon: f64| lon / (2.0 * PI) * px.w as f64 - 0.5;
                let x0 = to_col(lon - half).floor() as i64;
                let x1 = to_col(lon + half).ceil() as i64;
                (x1 - x0 + 1 < px.w as i64).then_some((x0, x1))
            };
            let (x0, x1) = columns.unwrap_or((0, px.w as i64 - 1));
            for xi in x0..=x1 {
                let x = xi.rem_euclid(px.w as i64) as usize;
                let (cos_lon, sin_lon) = px.columns[x];
                let dot = cos_lat * cos_lon * c[0] + cos_lat * sin_lon * c[1] + sin_lat * c[2];
                let i = y * px.w + x;
                if dot >= cos_reach && elevation[i] < level {
                    out.push((i as u32, level));
                }
            }
        }
        out
    }

    /// Groups `drawn` (oldest first) in waves of craters that share no
    /// pixel, each crater after every older one it overlaps.
    fn waves(&self, drawn: &[(&Crater, f32)]) -> Vec<Vec<usize>> {
        // Craters indexed by the cells of a coarse latitude-longitude grid
        // their reach covers.
        const COLS: usize = 512;
        const ROWS: usize = 256;
        let cell = PI / ROWS as f64;
        let cells_of = |centre: [f64; 3], reach: f64| -> Vec<usize> {
            let lat = centre[2].clamp(-1.0, 1.0).asin();
            let lon = centre[1].atan2(centre[0]).rem_euclid(2.0 * PI);
            let row = |lat: f64| (((PI / 2.0 - lat) / cell) as usize).min(ROWS - 1);
            let (r0, r1) = (
                row((lat + reach).min(PI / 2.0)),
                row((lat - reach).max(-PI / 2.0)),
            );
            let widest = (lat.abs() + reach).min(PI / 2.0);
            let half = if widest >= PI / 2.0 - 1e-9 {
                PI
            } else {
                (reach / widest.cos()).min(PI)
            };
            let (c0, c1) = if half >= PI {
                (0, COLS as i64 - 1)
            } else {
                (
                    ((lon - half) / cell).floor() as i64,
                    ((lon + half) / cell).floor() as i64,
                )
            };
            let mut out = Vec::with_capacity((r1 - r0 + 1) * (c1 - c0 + 1) as usize);
            for r in r0..=r1 {
                for c in c0..=c1 {
                    out.push(r * COLS + c.rem_euclid(COLS as i64) as usize);
                }
            }
            out
        };
        let reaches: Vec<f64> = drawn.iter().map(|(c, _)| self.reach(c)).collect();
        let trig: Vec<(f64, f64)> = reaches.iter().map(|r| (r.cos(), r.sin())).collect();
        // Craters covering many cells are kept out of the index, in a short
        // list every crater checks.
        const LARGE_CELLS: usize = 1024;
        let mut index: Vec<Vec<u32>> = vec![Vec::new(); COLS * ROWS];
        let mut large: Vec<usize> = Vec::new();
        let mut wave_of = vec![0usize; drawn.len()];
        let mut waves: Vec<Vec<usize>> = Vec::new();
        for (i, (crater, _)) in drawn.iter().enumerate() {
            let c = crater.centre;
            let overlaps = |j: usize| {
                let o = drawn[j].0.centre;
                // Closer than the sum of the reaches: cos(a + b) from the
                // cosines and sines of each.
                let ((ca, sa), (cb, sb)) = (trig[i], trig[j]);
                reaches[i] + reaches[j] >= PI
                    || c[0] * o[0] + c[1] * o[1] + c[2] * o[2] > ca * cb - sa * sb
            };
            let cells = cells_of(c, reaches[i]);
            let mut wave = 0;
            for &j in &large {
                if overlaps(j) {
                    wave = wave.max(wave_of[j] + 1);
                }
            }
            for &cell in &cells {
                for &j in &index[cell] {
                    if overlaps(j as usize) {
                        wave = wave.max(wave_of[j as usize] + 1);
                    }
                }
            }
            wave_of[i] = wave;
            if waves.len() <= wave {
                waves.resize_with(wave + 1, Vec::new);
            }
            waves[wave].push(i);
            if cells.len() > LARGE_CELLS {
                large.push(i);
            } else {
                for cell in cells {
                    index[cell].push(i as u32);
                }
            }
        }
        waves
    }

    /// The new elevation of every pixel a crater changes, over the current
    /// relief.
    fn draw(&self, crater: &Crater, wear: f32, elevation: &[f32], px: &Pixels) -> Vec<(u32, f32)> {
        let c = crater.centre;
        let radius = 0.5 * crater.diameter_km / self.radius_km; // radians
        let (east, north) = tangent_basis(c);
        let level = self.reference_level(crater, radius, east, north, elevation, px);
        if self.sea && level < self.waterline {
            return Vec::new();
        }
        let profile = Profile::new(crater, self.transition_km);
        let reach = self.reach(crater);
        let lat = c[2].clamp(-1.0, 1.0).asin();
        let lon = c[1].atan2(c[0]).rem_euclid(2.0 * PI);
        let to_row = |lat: f64| (PI / 2.0 - lat) / PI * px.h as f64 - 0.5;
        let y0 = to_row(lat + reach).floor().max(0.0) as usize;
        let y1 = (to_row(lat - reach).ceil().max(0.0) as usize).min(px.h - 1);
        let cos_reach = reach.cos();
        let row = |y: usize| {
            let mut out = Vec::new();
            let (cos_lat, sin_lat) = px.rows[y];
            let row_lat = sin_lat.atan2(cos_lat);
            let columns = if row_lat.abs() + reach >= PI / 2.0 {
                None
            } else {
                let half = (reach.sin() / cos_lat).min(1.0).asin();
                let to_col = |lon: f64| lon / (2.0 * PI) * px.w as f64 - 0.5;
                let x0 = to_col(lon - half).floor() as i64;
                let x1 = to_col(lon + half).ceil() as i64;
                (x1 - x0 + 1 < px.w as i64).then_some((x0, x1))
            };
            let (x0, x1) = columns.unwrap_or((0, px.w as i64 - 1));
            for xi in x0..=x1 {
                let x = xi.rem_euclid(px.w as i64) as usize;
                let (cos_lon, sin_lon) = px.columns[x];
                let v = [cos_lat * cos_lon, cos_lat * sin_lon, sin_lat];
                let dot = (v[0] * c[0] + v[1] * c[1] + v[2] * c[2]).clamp(-1.0, 1.0);
                if dot < cos_reach {
                    continue;
                }
                let r = dot.acos() / radius;
                let along = |b: [f64; 3]| v[0] * b[0] + v[1] * b[1] + v[2] * b[2];
                let (e, n) = (along(east), along(north));
                let len = (e * e + n * n).sqrt();
                let direction = if len > 1e-12 {
                    (e / len, n / len)
                } else {
                    (1.0, 0.0)
                };
                let i = y * px.w + x;
                out.push((
                    i as u32,
                    profile.blend(r, direction, elevation[i], level, wear),
                ));
            }
            out
        };
        // Large craters (basins) are alone in their wave: their rows in
        // parallel.
        if y1 - y0 >= PARALLEL_ROWS {
            (y0..=y1).into_par_iter().flat_map_iter(row).collect()
        } else {
            (y0..=y1).flat_map(row).collect()
        }
    }

    /// Mean elevation over a crater's site before it forms: its centre and
    /// six points at 0.7 radii.
    fn reference_level(
        &self,
        crater: &Crater,
        radius: f64,
        east: [f64; 3],
        north: [f64; 3],
        elevation: &[f32],
        px: &Pixels,
    ) -> f32 {
        let c = crater.centre;
        let (s, k) = ((0.7 * radius).sin(), (0.7 * radius).cos());
        let mut sum = px.sample(elevation, c);
        for i in 0..6 {
            let phi = f64::from(i) * PI / 3.0;
            let (sp, cp) = phi.sin_cos();
            let p = [
                k * c[0] + s * (cp * east[0] + sp * north[0]),
                k * c[1] + s * (cp * east[1] + sp * north[1]),
                k * c[2] + s * (cp * east[2] + sp * north[2]),
            ];
            sum += px.sample(elevation, p);
        }
        sum / 7.0
    }
}

/// A fresh crater's brightness, prepared once: its floor, ejecta and rays.
struct Fresh {
    c: [f64; 3],
    east: [f64; 3],
    north: [f64; 3],
    /// Crater radius, radians.
    radius: f64,
    /// Farthest reach of its rays, radians.
    reach: f64,
    cos_reach: f64,
    /// Longitude of its centre, in [0, 2π).
    lon: f64,
    /// Rows of the map it lights.
    rows: (usize, usize),
    fresh: f32,
    /// Each ray: its direction in the tangent plane (cos, sin), its width
    /// at the rim (radians of azimuth) and its length (crater radii).
    rays: Vec<((f64, f64), f64, f64)>,
}

impl Fresh {
    fn new(crater: &Crater, fresh: f32, planet_radius_km: f64, height: usize) -> Self {
        let c = crater.centre;
        let (east, north) = tangent_basis(c);
        let radius = 0.5 * crater.diameter_km / planet_radius_km;
        // A stream of its own: the rim and ejecta draws are not disturbed.
        let mut rng = Rng(mix(crater.shape ^ 0x5241_5953));
        let count = 6 + (rng.next() % 9) as usize;
        let rays: Vec<_> = (0..count)
            .map(|_| {
                let phi = 2.0 * PI * rng.unit();
                let width = 0.04 + 0.08 * rng.unit();
                let length = RAY_LENGTH.0 + (RAY_LENGTH.1 - RAY_LENGTH.0) * rng.unit();
                ((phi.cos(), phi.sin()), width, length)
            })
            .collect();
        let longest = rays.iter().map(|r| r.2).fold(3.0, f64::max);
        let reach = (longest * radius).min(PI);
        let lat = c[2].clamp(-1.0, 1.0).asin();
        let to_row = |lat: f64| (PI / 2.0 - lat) / PI * height as f64 - 0.5;
        let y0 = to_row(lat + reach).floor().max(0.0) as usize;
        let y1 = (to_row(lat - reach).ceil().max(0.0) as usize).min(height - 1);
        Fresh {
            c,
            east,
            north,
            radius,
            reach,
            cos_reach: reach.cos(),
            lon: c[1].atan2(c[0]).rem_euclid(2.0 * PI),
            rows: (y0, y1),
            fresh,
            rays,
        }
    }

    /// The brightness it adds at the unit vector `v`.
    fn at(&self, v: [f64; 3]) -> f32 {
        let c = self.c;
        let dot = (v[0] * c[0] + v[1] * c[1] + v[2] * c[2]).clamp(-1.0, 1.0);
        if dot < self.cos_reach {
            return 0.0;
        }
        let r = dot.acos() / self.radius;
        if r < 1.0 {
            return self.fresh * FRESH_FLOOR;
        }
        // The continuous blanket, thinning as the ejecta.
        let mut a = FRESH_EJECTA * (r.powi(-3) as f32);
        let along = |b: [f64; 3]| v[0] * b[0] + v[1] * b[1] + v[2] * b[2];
        let (e, n) = (along(self.east), along(self.north));
        let len = (e * e + n * n).sqrt();
        if len > 1e-12 {
            let (ce, cn) = (e / len, n / len);
            for &((rc, rs), width, length) in &self.rays {
                // cos of the angle to the ray: 1 − cos ≈ Δ² / 2, no trigonometry.
                let cos = ce * rc + cn * rs;
                if cos <= 0.0 || r > length {
                    continue;
                }
                // Streaks of about constant width: narrower in azimuth farther out.
                let w = width / r.sqrt();
                let across = 2.0 * (1.0 - cos) / (w * w);
                if across > 9.0 {
                    continue;
                }
                let fade = 1.0 - smoothstep(0.5 * length as f32, length as f32, r as f32);
                a += FRESH_RAYS * ((-across).exp() * r.powf(-0.5)) as f32 * fade;
            }
        }
        self.fresh * a.min(1.0)
    }
}

/// Something that changes the surface, in the order of [`CraterField::apply`].
#[derive(Clone, Copy)]
enum Event<'a> {
    /// An impact, keeping this share of its relief.
    Impact(&'a Crater, f32),
    Flood(&'a Flood),
}

/// Row and column trigonometry of the map, for unit vectors of pixels.
struct Pixels<'a> {
    w: usize,
    h: usize,
    rows: &'a [(f64, f64)],
    columns: &'a [(f64, f64)],
}

impl Pixels<'_> {
    /// Elevation of the pixel holding the point `p` of the unit sphere.
    fn sample(&self, elevation: &[f32], p: [f64; 3]) -> f32 {
        let lat = p[2].clamp(-1.0, 1.0).asin();
        let lon = p[1].atan2(p[0]).rem_euclid(2.0 * PI);
        let x = ((lon / (2.0 * PI) * self.w as f64) as usize).min(self.w - 1);
        let y = (((PI / 2.0 - lat) / PI * self.h as f64) as usize).min(self.h - 1);
        elevation[y * self.w + x]
    }
}

/// East and north unit vectors of the tangent plane at `c` (any pair at
/// the poles).
fn tangent_basis(c: [f64; 3]) -> ([f64; 3], [f64; 3]) {
    let len = (c[0] * c[0] + c[1] * c[1]).sqrt();
    let east = if len < 1e-9 {
        [1.0, 0.0, 0.0]
    } else {
        [-c[1] / len, c[0] / len, 0.0]
    };
    let north = [
        c[1] * east[2] - c[2] * east[1],
        c[2] * east[0] - c[0] * east[2],
        c[0] * east[1] - c[1] * east[0],
    ];
    (east, north)
}

/// The radial profile of one crater, in elevation units.
struct Profile {
    /// Depth from the rim crest to the floor.
    depth: f64,
    /// Rim height above the surrounding ground.
    rim: f64,
    /// Radius of the flat floor, in crater radii (0 for a bowl).
    floor: f64,
    /// Height of the central peak and of the peak ring.
    peak: f64,
    ring: f64,
    /// Rim outline and ejecta lobes: harmonics of the azimuth, each as
    /// `(a cos φ, a sin φ)` for amplitude `a` and phase `φ`.
    outline: [(f64, f64); 4],
    lobes: [(f64, f64); 4],
}

impl Profile {
    fn new(crater: &Crater, transition_km: f64) -> Self {
        let d = crater.diameter_km;
        let t = transition_km;
        let units = f64::from(ELEVATION_KM);
        // Pike (1974, 1977): simple craters 0.2 D deep with a rim of
        // 0.036 D; complex ones grow shallower, continuously at the
        // transition.
        let (depth, rim) = if d < t {
            (0.2 * d, 0.036 * d)
        } else {
            (0.2 * t * (d / t).powf(0.3), 0.036 * t * (d / t).powf(0.4))
        };
        let complexity = ((d / t).log2() / 4.0).clamp(0.0, 1.0);
        let complex = f64::from(smoothstep(1.0, 1.5, (d / t) as f32));
        let ring = f64::from(smoothstep(5.0, 9.0, (d / t) as f32));
        let mut rng = Rng(crater.shape);
        let mut harmonics = || {
            [(); 4].map(|_| {
                let (a, phase) = (2.0 * rng.unit() - 1.0, 2.0 * PI * rng.unit());
                (a * phase.cos(), a * phase.sin())
            })
        };
        Profile {
            depth: depth / units,
            rim: rim / units,
            floor: complex * (0.25 + 0.35 * complexity),
            peak: complex * (1.0 - ring) * 0.3 * depth / units,
            ring: complex * ring * 0.18 * depth / units,
            outline: harmonics(),
            lobes: harmonics(),
        }
    }

    /// Sum of harmonics `k = first..first + 4` (each weighted `2 / k`) at
    /// the azimuth of unit direction `(cos θ, sin θ)`, about `[-1, 1]`.
    /// `cos kθ` and `sin kθ` come from powers of `e^iθ`, without
    /// trigonometric calls.
    fn wave(terms: &[(f64, f64); 4], first: u32, (c1, s1): (f64, f64)) -> f64 {
        let (mut c, mut s) = (1.0, 0.0);
        for _ in 0..first {
            (c, s) = (c * c1 - s * s1, c * s1 + s * c1);
        }
        let mut sum = 0.0;
        for (&(ac, as_), k) in terms.iter().zip(first..) {
            // a cos(kθ + φ) = a cos φ cos kθ − a sin φ sin kθ
            sum += (ac * c - as_ * s) * 2.0 / f64::from(k);
            (c, s) = (c * c1 - s * s1, c * s1 + s * c1);
        }
        sum
    }

    /// The new elevation at `r` crater radii from the centre, in the unit
    /// direction `(cos θ, sin θ)` of the tangent plane, over `old`, for a
    /// crater formed on ground at `level` and keeping `wear` of its relief.
    fn blend(&self, r: f64, direction: (f64, f64), old: f32, level: f32, wear: f32) -> f32 {
        let r = r / (1.0 + RIM_JITTER * Self::wave(&self.outline, 2, direction));
        // Share of the cavity, against the ejecta, across the rim.
        let w = 1.0 - smoothstep(0.92, 1.08, r as f32);
        // Inside: the cavity, measured from the rim crest. Most pixels of a
        // crater are outside it, where this is skipped.
        let inside = if w > 0.0 {
            let x = r.min(1.0);
            let wall = if self.floor > 0.0 {
                ((x - self.floor) / (1.0 - self.floor)).max(0.0).powf(1.6)
            } else {
                x * x
            };
            let mut cavity = self.rim - self.depth * (1.0 - wall);
            if self.peak > 0.0 {
                cavity += self.peak * (-(x / 0.13).powi(2)).exp();
            }
            if self.ring > 0.0 {
                cavity += self.ring * (-((x - 0.5) / 0.07).powi(2)).exp();
            }
            level + cavity as f32 + INHERITANCE * (old - level)
        } else {
            0.0
        };
        // Outside: the ejecta blanket over the old relief.
        let outside = if w < 1.0 {
            let lobes = (1.0 + EJECTA_LOBES * Self::wave(&self.lobes, 3, direction)).max(0.0);
            let ejecta = self.rim
                * lobes
                * r.max(1.0).powi(-3)
                * (1.0 - f64::from(smoothstep(2.0, EJECTA_RADII as f32, r as f32)));
            old + ejecta as f32
        } else {
            0.0
        };
        let target = w * inside + (1.0 - w) * outside;
        old + wear * (target - old)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preset::Preset;

    /// A Moon-sized barren world, without the preset's maria (the tests
    /// of floods set them).
    fn moon(surface_age_ga: f32) -> WorldParams {
        let mut planet = Preset::load("barren").unwrap();
        planet.geology.maria = None;
        WorldParams {
            width: 512,
            height: 256,
            seed: 11,
            planet,
            sea_level: 0.2,
            volcanic_intensity: 0.0,
            circumference_km: 10_917.0,
            axial_tilt_deg: 1.5,
            erosion: 0.0,
            pressure_bar: 0.0,
            surface_age_ga,
            star: crate::star::SUN,
            orbit_au: 1.0,
        }
    }

    #[test]
    fn the_chronology_matches_the_lunar_highlands() {
        assert!((crater_density(4.0) - 0.063).abs() < 0.002);
        // Constant flux over the last 3 Ga: linear.
        assert!((crater_density(1.0) - 8.38e-4).abs() < 1e-5);
        assert_eq!(crater_density(0.0), 0.0);
        // Ages invert the curve, and most craters of an old surface are old.
        for f in [0.1, 0.5, 0.9] {
            let a = age_at(f, 4.0);
            assert!((crater_density(a) / crater_density(4.0) - f).abs() < 1e-6);
        }
        assert!(age_at(0.5, 4.0) > 3.8);
    }

    #[test]
    fn cube_cells_cover_the_sphere() {
        let n = 7;
        let step = 2.0 / f64::from(n);
        let mut total = 0.0;
        for i in 0..n {
            for j in 0..n {
                let (a, b) = (-1.0 + step * f64::from(i), -1.0 + step * f64::from(j));
                total += solid_angle(a, a + step, b, b + step);
            }
        }
        assert!((6.0 * total - 4.0 * PI).abs() < 1e-9);
        // Each face holds the points where its axis dominates.
        for face in 0..6 {
            let p = cube_point(face, 0.3, -0.8);
            let axis = (face / 2) as usize;
            assert!(p[axis].abs() >= p[(axis + 1) % 3].abs().max(p[(axis + 2) % 3].abs()));
        }
    }

    #[test]
    fn counts_follow_the_chronology() {
        // The Moon: about 6 000 craters of 20 km or more on a 4 Ga surface
        // (5 185 counted by Head et al. 2010).
        let field = CraterField::new(&moon(4.0), 20.0);
        let radius = 10_917.0 / (2.0 * PI);
        let expected = crater_density(4.0) / 400.0 * 4.0 * PI * radius * radius;
        let n = field.craters.len() as f64;
        assert!((n / expected - 1.0).abs() < 0.05, "{n} vs {expected}");
        assert!(field.craters.iter().all(|c| c.diameter_km >= 20.0));
        // Oldest first.
        assert!(field.craters.windows(2).all(|w| w[0].age_ga >= w[1].age_ga));
        // A fresh surface has none.
        assert!(CraterField::new(&moon(0.0), 20.0).craters.is_empty());
    }

    #[test]
    fn craters_depend_on_the_seed_not_on_the_map() {
        let a = CraterField::new(&moon(3.0), 30.0).craters;
        let b = CraterField::new(
            &WorldParams {
                width: 2048,
                height: 1024,
                ..moon(3.0)
            },
            30.0,
        )
        .craters;
        assert_eq!(a, b);
        let other = CraterField::new(
            &WorldParams {
                seed: 12,
                ..moon(3.0)
            },
            30.0,
        )
        .craters;
        assert_ne!(a, other);
    }

    #[test]
    fn dense_air_filters_small_impactors() {
        let venus = |pressure_bar| {
            CraterField::new(
                &WorldParams {
                    pressure_bar,
                    ..moon(0.5)
                },
                1.0,
            )
            .craters
            .iter()
            .filter(|c| c.diameter_km < 4.0)
            .count()
        };
        let (airless, dense) = (venus(0.0), venus(92.0));
        assert!(airless > 1000, "{airless}");
        assert!(dense * 100 < airless, "{dense} of {airless}");
    }

    #[test]
    fn a_fresh_crater_has_a_cavity_a_rim_and_ejecta() {
        let crater = Crater {
            centre: [1.0, 0.0, 0.0],
            diameter_km: 10.0,
            age_ga: 1.0,
            shape: 0,
        };
        // On the Moon a 10 km crater is a simple bowl, 2 km deep.
        let p = Profile::new(&crater, MOON_TRANSITION_KM);
        let at = |r: f64| p.blend(r, (1.0, 0.0), 0.0, 0.0, 1.0) * ELEVATION_KM;
        let floor = at(0.0);
        assert!((floor - (0.36 - 2.0)).abs() < 1e-3, "{floor}");
        let rim = at(1.0);
        assert!(rim > 0.2 && rim > at(0.5) && rim > at(1.5), "{rim}");
        assert!(at(1.5) > at(2.4) && at(2.4) > 0.0);
        assert_eq!(at(3.5), 0.0);
        // A worn crater keeps a share of its relief.
        let worn = p.blend(0.0, (1.0, 0.0), 0.0, 0.0, 0.25) * ELEVATION_KM;
        assert!((worn - 0.25 * floor).abs() < 1e-4);
    }

    #[test]
    fn large_craters_have_flat_floors_and_peaks() {
        let crater = |diameter_km| Crater {
            centre: [1.0, 0.0, 0.0],
            diameter_km,
            age_ga: 1.0,
            shape: 3,
        };
        let complex = Profile::new(&crater(50.0), MOON_TRANSITION_KM);
        assert!(complex.floor > 0.25 && complex.peak > 0.0 && complex.ring == 0.0);
        // Shallower than a bowl of the same size.
        assert!(complex.depth * f64::from(ELEVATION_KM) < 0.5 * 0.2 * 50.0);
        let basin = Profile::new(&crater(300.0), MOON_TRANSITION_KM);
        assert!(basin.ring > 0.0 && basin.peak == 0.0);
        // Stronger gravity lowers the transition: an 8 km crater on a
        // planet of Earth's gravity is complex.
        assert_eq!(Profile::new(&crater(8.0), MOON_TRANSITION_KM).floor, 0.0);
        let t_earth = MOON_TRANSITION_KM * MOON_GRAVITY / EARTH_GRAVITY;
        assert!(Profile::new(&crater(8.0), t_earth).floor > 0.0);
    }

    #[test]
    fn harmonics_match_their_trigonometric_form() {
        let terms = [(0.3, -0.2), (0.5, 0.1), (-0.4, 0.4), (0.2, 0.0)];
        for theta in [0.0f64, 0.7, 2.0, -2.9] {
            let direct: f64 = terms
                .iter()
                .zip(3u32..)
                .map(|(&(ac, as_), k)| {
                    let k = f64::from(k);
                    (ac * (k * theta).cos() - as_ * (k * theta).sin()) * 2.0 / k
                })
                .sum();
            let fast = Profile::wave(&terms, 3, (theta.cos(), theta.sin()));
            assert!((direct - fast).abs() < 1e-12);
        }
    }

    #[test]
    fn waves_draw_like_one_crater_at_a_time() {
        let p = moon(4.0);
        let grid = p.grid();
        let field = CraterField {
            wet_until_ga: 4.6,
            ..CraterField::new(&p, 2.0 * base_min_diameter_km(&grid))
        };
        let base: Vec<f32> = (0..grid.width * grid.height)
            .map(|i| 0.3 + 0.1 * ((i % 37) as f32 / 37.0))
            .collect();
        let mut waves = base.clone();
        field.apply(&mut waves, &grid, Era::Dry);
        // One at a time, in age order.
        let (w, h) = (grid.width as usize, grid.height as usize);
        let rows: Vec<(f64, f64)> = (0..grid.height)
            .map(|y| (grid.latitude(y).cos(), grid.latitude(y).sin()))
            .collect();
        let columns: Vec<(f64, f64)> = (0..w)
            .map(|x| {
                let lon = (x as f64 + 0.5) / w as f64 * 2.0 * PI;
                (lon.cos(), lon.sin())
            })
            .collect();
        let px = Pixels {
            w,
            h,
            rows: &rows,
            columns: &columns,
        };
        let mut serial = base;
        for crater in &field.craters {
            let wear = field.wear(crater);
            if wear < WORN_OUT {
                continue;
            }
            for (i, v) in field.draw(crater, wear as f32, &serial, &px) {
                serial[i as usize] = v;
            }
        }
        assert_eq!(waves, serial);
        assert!(
            field
                .waves(&field.craters.iter().map(|c| (c, 1.0)).collect::<Vec<_>>())
                .len()
                > 1
        );
    }

    #[test]
    fn epochs_show_the_craters_formed_by_then() {
        let p = moon(4.0);
        let present = CraterField::new(&p, 20.0).craters;
        // In the past: the craters at least that old, and no others.
        let past = CraterField::new_at(&p, 20.0, 3.8).craters;
        let expected: Vec<Crater> = present
            .iter()
            .filter(|c| c.age_ga >= 3.8)
            .copied()
            .collect();
        assert!(!past.is_empty() && past.len() < present.len());
        assert_eq!(past, expected);
        // In the future: every present crater, and new ones at today's flux.
        let future = CraterField::new_at(&p, 20.0, -1.0).craters;
        let new: Vec<&Crater> = future.iter().filter(|c| c.age_ga < 0.0).collect();
        assert!(new.iter().all(|c| c.age_ga >= -1.0));
        assert_eq!(future.len() - new.len(), present.len());
        let radius = 10_917.0 / (2.0 * PI);
        let expected = FLUX_PER_GA / 400.0 * 4.0 * PI * radius * radius;
        let n = new.len() as f64;
        assert!((n / expected - 1.0).abs() < 0.3, "{n} vs {expected}");
        // Halfway, half of them.
        let half = CraterField::new_at(&p, 20.0, -0.5).craters;
        assert!(half.iter().all(|c| c.age_ga >= -0.5));
        // The present has none of the future's.
        assert!(present.iter().all(|c| c.age_ga >= 0.0));
    }

    #[test]
    fn going_back_in_time_freshens_craters() {
        let p = moon(4.0);
        let crater = Crater {
            centre: [1.0, 0.0, 0.0],
            diameter_km: 0.6,
            age_ga: 3.0,
            shape: 0,
        };
        let now = CraterField::context(&p).wear(&crater);
        let then = CraterField::context_at(&p, 2.9).wear(&crater);
        assert!(then > 0.8 && now < 0.1, "{then} {now}");
    }

    #[test]
    fn fresh_craters_have_bright_ejecta_and_rays_that_fade() {
        let p = moon(4.0);
        let grid = p.grid();
        let crater = Crater {
            centre: [1.0, 0.0, 0.0],
            diameter_km: 150.0,
            age_ga: 0.05,
            shape: 9,
        };
        let lit = |field: CraterField| {
            CraterField {
                craters: vec![crater],
                ..field
            }
            .albedo(&grid)
        };
        let fresh = lit(CraterField::context(&p));
        let brightest = fresh.iter().fold(0.0f32, |m, &v| m.max(v));
        assert!(brightest > 0.4, "{brightest}");
        // Rays: some pixels well beyond the ejecta are lit.
        let lit_pixels = fresh.iter().filter(|&&v| v > 0.02).count();
        let crater_px = PI * (75.0f64 / (10_917.0 / 512.0)).powi(2);
        assert!(
            lit_pixels as f64 > 10.0 * crater_px,
            "{lit_pixels} vs {crater_px}"
        );
        // 1 Ga later the rays are gone; before it formed, nothing.
        let old = lit(CraterField::context_at(&p, -1.0));
        assert!(old.iter().all(|&v| v < 0.03));
        assert!(
            lit(CraterField::context_at(&p, 0.1))
                .iter()
                .all(|&v| v == 0.0)
        );
        // Air erases them.
        let venus = CraterField::context(&WorldParams {
            pressure_bar: 92.0,
            ..moon(4.0)
        });
        assert!(lit(venus).iter().all(|&v| v == 0.0));
    }

    #[test]
    fn lava_floods_large_basins_within_the_window() {
        let mut p = moon(4.0);
        p.planet.geology.maria = Some([3.9, 3.1]);
        p.volcanic_intensity = 1.0;
        let field = CraterField::new(&p, 50.0);
        assert!(!field.floods.is_empty());
        for f in &field.floods {
            assert!((3.1..=3.9).contains(&f.age_ga) && f.age_ga <= f.basin.age_ga);
            assert!(f.basin.diameter_km >= MARE_MIN_KM);
        }
        // Before the lava came, no flood; without a window, none at all.
        assert!(CraterField::new_at(&p, 50.0, 3.95).floods.is_empty());
        assert!(CraterField::new(&moon(4.0), 50.0).floods.is_empty());
    }

    #[test]
    fn a_flood_fills_its_basin_flat_and_dark() {
        let p = moon(4.0);
        let grid = p.grid();
        let basin = Crater {
            centre: [1.0, 0.0, 0.0],
            diameter_km: 600.0,
            age_ga: 3.95,
            shape: 4,
        };
        let field = CraterField {
            craters: vec![basin],
            floods: vec![Flood {
                basin,
                age_ga: 3.5,
                fill: 0.6,
            }],
            wet_until_ga: 4.6,
            ..CraterField::context(&p)
        };
        let mut e = vec![0.3; grid.width as usize * grid.height as usize];
        let mare = field.apply(&mut e, &grid, Era::Dry);
        let flooded: Vec<f32> = e
            .iter()
            .zip(&mare)
            .filter(|(_, m)| **m)
            .map(|(v, _)| *v)
            .collect();
        assert!(flooded.len() > 100, "{}", flooded.len());
        // One flat level, between the floor and the ground around.
        let level = flooded[0];
        assert!(flooded.iter().all(|&v| v == level));
        let floor = e.iter().fold(f32::MAX, |m, &v| m.min(v));
        assert!(level > floor && level < 0.3 + 0.05, "{floor} {level}");
        // Dark, with no rays of its own.
        let mut albedo = vec![0.0; e.len()];
        darken_maria(&mut albedo, &mare);
        assert!(albedo.iter().zip(&mare).all(|(a, m)| (*a < 0.0) == *m));
    }

    #[test]
    fn locked_planets_have_a_more_cratered_leading_hemisphere() {
        // A planet of an M dwarf, locked, in the middle of its habitable zone.
        let star = crate::star::parse_type("M5V").unwrap();
        let locked = WorldParams {
            star,
            orbit_au: crate::star::orbit_at(star, 0.5),
            ..moon(4.0)
        };
        let field = CraterField::new(&locked, 20.0);
        let since = field.locked_since_ga.expect("locked");
        let count = |young: bool, leading: bool| {
            field
                .craters
                .iter()
                .filter(|c| (c.age_ga < since) == young)
                .filter(|c| (c.centre[1] > 0.0) == leading)
                .count() as f64
        };
        // Hemispheres facing the apex and away from it: the ratio of
        // `(1 + k/2) / (1 − k/2)`, about 1.14, for craters since the locking.
        let ratio = count(true, true) / count(true, false);
        assert!((1.05..1.25).contains(&ratio), "{ratio}");
        if since < 3.9 {
            // Older craters formed anywhere alike.
            let old = count(false, true) / count(false, false);
            assert!((0.9..1.1).contains(&old), "{old}");
        }
        // The mean density is kept.
        let unlocked = CraterField::new(&moon(4.0), 20.0).craters.len() as f64;
        let n = field.craters.len() as f64;
        assert!((n / unlocked - 1.0).abs() < 0.05, "{n} vs {unlocked}");
        // Unlocked worlds: no favoured side.
        assert!(CraterField::context(&moon(4.0)).locked_since_ga.is_none());
    }

    #[test]
    fn cube_cells_invert_cube_points() {
        for face in 0..6 {
            for (a, b) in [(0.0, 0.0), (0.3, -0.8), (-0.99, 0.99), (0.5, 0.25)] {
                let (f, a2, b2) = cube_cell(cube_point(face, a, b));
                assert_eq!(f, face);
                assert!((a - a2).abs() < 1e-12 && (b - b2).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn craters_near_a_cap_are_those_of_the_whole_sphere() {
        let p = moon(4.0);
        let all = CraterField::new(&p, 4.0);
        let field = CraterField::context(&p);
        for (centre, angle) in [
            ([1.0, 0.0, 0.0], 0.05),
            (cube_point(4, 0.99, -0.99), 0.02),
            ([0.0, 0.0, 1.0], 0.1),
        ] {
            let near = field.near(centre, angle, 4.0, 40.0);
            let expected: Vec<Crater> = all
                .craters
                .iter()
                .filter(|c| c.diameter_km < 40.0)
                .filter(|c| {
                    let dot: f64 = (0..3).map(|k| c.centre[k] * centre[k]).sum();
                    dot.clamp(-1.0, 1.0).acos() < angle + field.reach(c)
                })
                .copied()
                .collect();
            assert!(!expected.is_empty());
            assert_eq!(near, expected);
        }
    }

    #[test]
    fn diffusion_wears_small_old_craters_only() {
        let field = CraterField {
            wet_until_ga: 4.6,
            ..CraterField::context(&moon(4.0))
        };
        let crater = |diameter_km, age_ga| Crater {
            centre: [1.0, 0.0, 0.0],
            diameter_km,
            age_ga,
            shape: 0,
        };
        assert!(field.wear(&crater(0.5, 3.0)) < 0.05);
        assert!(field.wear(&crater(0.5, 0.01)) > 0.9);
        assert!(field.wear(&crater(4.0, 3.9)) > 0.8);
        assert!(field.wear(&crater(40.0, 4.0)) > 0.99);
    }

    #[test]
    fn added_relief_does_not_depend_on_the_window() {
        let p = moon(4.0);
        let field = CraterField::context(&p);
        let centre = [1.0, 0.0, 0.0];
        let craters = field.near(centre, 0.02, 1.0, 8.0);
        assert!(!craters.is_empty());
        let step = 0.0005;
        let window = |x0: usize, y0: usize| Window {
            width: 64,
            height: 64,
            lat0: 0.016 - y0 as f64 * step,
            dlat: step,
            lon0: -0.016 + x0 as f64 * step,
            dlon: step,
        };
        let relief = |win: &Window| {
            let mut r = vec![0.0; 64 * 64];
            field.add_relief(&craters, win, |_| 0.5, |_| 1.0, &mut r);
            r
        };
        let (a, b) = (relief(&window(0, 0)), relief(&window(20, 10)));
        assert!(a.iter().any(|&v| v != 0.0));
        for y in 0..54 {
            for x in 0..44 {
                let (va, vb) = (a[(y + 10) * 64 + x + 20], b[y * 64 + x]);
                assert!((va - vb).abs() < 1e-5, "{va} vs {vb}");
            }
        }
    }

    #[test]
    fn drawing_follows_the_era_the_sea_and_the_wear() {
        let mut p = moon(4.0);
        let grid = p.grid();
        let flat = vec![0.5; grid.width as usize * grid.height as usize];
        let changed = |field: &CraterField, era| {
            let mut e = flat.clone();
            field.apply(&mut e, &grid, era);
            e.iter().filter(|&&v| v != 0.5).count()
        };
        // Never wet: every crater is drawn after the erosion.
        let dry = CraterField {
            wet_until_ga: 4.6,
            ..CraterField::new(&p, base_min_diameter_km(&grid))
        };
        assert!(!dry.craters.is_empty());
        assert_eq!(changed(&dry, Era::Wet), 0);
        let fresh = changed(&dry, Era::Dry);
        assert!(fresh > 1000, "{fresh}");

        // Wet until 3.5 Ga with erosion: the oldest craters are drawn
        // before the erosion, worn.
        p.erosion = 1.0;
        let mars = CraterField {
            wet_until_ga: 3.5,
            ..CraterField::new(&p, base_min_diameter_km(&grid))
        };
        assert!(changed(&mars, Era::Wet) > 0);
        assert!(changed(&mars, Era::Dry) < fresh);

        // A sea fills the basins: craters on its floor leave nothing.
        let sea = CraterField {
            waterline: 1.0,
            sea: true,
            ..CraterField::new(&p, base_min_diameter_km(&grid))
        };
        assert_eq!(changed(&sea, Era::Wet) + changed(&sea, Era::Dry), 0);
        // Barren worlds remap their seas to rock: their basins are cratered.
        assert!(!CraterField::new(&p, 100.0).sea);
        p.planet = Preset::load("frozen").unwrap();
        assert!(CraterField::new(&p, 100.0).sea);
    }
}
