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

/// Solid angle of the gnomonic rectangle `[a0, a1] × [b0, b1]` of a cube face.
fn solid_angle(a0: f64, a1: f64, b0: f64, b1: f64) -> f64 {
    let corner = |a: f64, b: f64| (a * b).atan2((1.0 + a * a + b * b).sqrt());
    corner(a1, b1) - corner(a0, b1) - corner(a1, b0) + corner(a0, b0)
}

/// The craters of a world, oldest first, with what drawing them needs.
pub struct CraterField {
    pub craters: Vec<Crater>,
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
    /// Every crater of the world `p` of `min_diameter_km` or more that has
    /// survived to the present, oldest first.
    pub fn new(p: &WorldParams, min_diameter_km: f64) -> Self {
        let radius_km = p.circumference_km as f64 / (2.0 * PI);
        let gravity = EARTH_GRAVITY * p.gravity_modifier() as f64;
        let mut field = CraterField {
            craters: Vec::new(),
            radius_km,
            transition_km: MOON_TRANSITION_KM * MOON_GRAVITY / gravity,
            waterline: p.sea_level + p.planet.rules.waterline,
            sea: [Biome::DeepOcean, Biome::Ocean]
                .iter()
                .any(|&b| p.planet.remap(b).is_water()),
            wet_until_ga: p.planet.geology.wet_until_ga(p.pressure_bar) as f64,
            erosion: p.erosion as f64,
        };
        let surface_age = p.surface_age_ga as f64;
        let density = crater_density(surface_age).min(SATURATION);
        if density <= 0.0 {
            return field;
        }
        let seed = stream_seed(p.seed, stream::CRATERS);
        let air_km = AIR_FILTER_KM * (p.pressure_bar.max(0.0) as f64).powf(0.75);
        // Basins up to about the planet's radius (South Pole–Aitken is 1.4
        // Moon radii, Hellas 0.7 Mars radii).
        let largest = radius_km;
        let first = min_diameter_km.max(1e-3).log2().floor() as i32;
        let last = largest.log2().floor() as i32;
        let face_area = 4.0 * PI * radius_km * radius_km / 6.0;
        for octave in first..=last {
            let low = 2f64.powi(octave);
            // N(>D) − N(>2D), per km².
            let per_km2 = 0.75 * density / (low * low);
            // About two craters per cell, at least one cell per face.
            let cells = ((per_km2 * face_area / 2.0).sqrt().ceil() as u32).max(1);
            let step = 2.0 / f64::from(cells);
            let found: Vec<Crater> = (0..6 * cells)
                .into_par_iter()
                .flat_map_iter(|row| {
                    let (face, i) = (row / cells, row % cells);
                    (0..cells).flat_map(move |j| {
                        let a0 = -1.0 + step * f64::from(i);
                        let b0 = -1.0 + step * f64::from(j);
                        let omega = solid_angle(a0, a0 + step, b0, b0 + step);
                        let mut rng = Rng::cell(seed, octave, face, i, j);
                        let count = rng.poisson(per_km2 * omega * radius_km * radius_km);
                        (0..count).filter_map(move |_| {
                            let centre =
                                cube_point(face, a0 + step * rng.unit(), b0 + step * rng.unit());
                            // Power law within the octave: N(>D) ∝ D⁻².
                            let diameter_km = low / (1.0 - 0.75 * rng.unit()).sqrt();
                            let age_ga = age_at(rng.unit(), surface_age);
                            let survives = rng.unit()
                                < f64::from(smoothstep(
                                    air_km as f32,
                                    (10.0 * air_km) as f32,
                                    diameter_km as f32,
                                ))
                                || air_km <= 0.0;
                            let shape = rng.next();
                            (survives && diameter_km >= min_diameter_km).then_some(Crater {
                                centre,
                                diameter_km,
                                age_ga,
                                shape,
                            })
                        })
                    })
                })
                .collect();
            field.craters.extend(found);
        }
        // Oldest first; the shape hash breaks ties, so the order is total.
        field.craters.sort_by(|a, b| {
            b.age_ga
                .total_cmp(&a.age_ga)
                .then_with(|| a.shape.cmp(&b.shape))
        });
        field
    }

    /// Share of its relief a crater keeps after its exposure to liquid
    /// water: 1 when it formed after the wet era or without erosion.
    fn wear(&self, crater: &Crater) -> f64 {
        let exposure = (crater.age_ga - self.wet_until_ga).max(0.0);
        if exposure == 0.0 || self.erosion <= 0.0 {
            return 1.0;
        }
        let lasts = WEAR_GA * crater.diameter_km / WEAR_KM / self.erosion;
        (-exposure / lasts).exp()
    }

    /// Whether a crater belongs to `era`.
    fn in_era(&self, crater: &Crater, era: Era) -> bool {
        (crater.age_ga > self.wet_until_ga) == (era == Era::Wet)
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
    pub fn apply(&self, elevation: &mut [f32], grid: &Grid, era: Era) {
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
        let drawn: Vec<(&Crater, f32)> = self
            .craters
            .iter()
            .filter(|c| self.in_era(c, era))
            .filter_map(|c| {
                let wear = self.wear(c);
                (wear >= WORN_OUT).then_some((c, wear as f32))
            })
            .collect();
        for wave in self.waves(&drawn) {
            let updates: Vec<Vec<(u32, f32)>> = wave
                .par_iter()
                .map(|&i| {
                    let (crater, wear) = drawn[i];
                    self.draw(crater, wear, elevation, &pixels)
                })
                .collect();
            for (i, value) in updates.into_iter().flatten() {
                elevation[i as usize] = value;
            }
        }
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

    fn moon(surface_age_ga: f32) -> WorldParams {
        WorldParams {
            width: 512,
            height: 256,
            seed: 11,
            planet: Preset::load("barren").unwrap(),
            sea_level: 0.2,
            volcanic_intensity: 0.0,
            circumference_km: 10_917.0,
            axial_tilt_deg: 1.5,
            erosion: 0.0,
            pressure_bar: 0.0,
            surface_age_ga,
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
            for (i, v) in field.draw(crater, 1.0, &serial, &px) {
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
