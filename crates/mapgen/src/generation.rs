use crate::biome::BiomeInput;
use crate::carve;
use crate::climate::{
    Field, MoistureInput, moisture, regional_variability, row_temperatures, temperature,
    temperature_anomaly,
};
use crate::delta;
use crate::erosion::erode;
use crate::geo::Grid;
use crate::hydrology::hydrology;
use crate::noise::{EARTH_CIRCUMFERENCE_KM, cell_value, fbm, nearest_point, ridged};
use crate::rivers;
use crate::world::*;
use noise::{NoiseFn, Perlin};
use rayon::prelude::*;
use serde::Serialize;
use std::f64::consts::PI;

/// Every intermediate noise value computed for one pixel.
#[derive(Clone, Copy)]
pub struct Sample {
    pub warp_x: f32,
    pub warp_y: f32,
    pub continent: f32,
    pub mountain: f32,
    pub mountain_weight: f32,
    pub elevation: f32,
    pub biome_elevation: f32,
    pub moisture: f32,
    pub volcanic_raw: f32,
    pub volcanic_zone: f32,
    /// Large-scale temperature anomaly noise, roughly in `[-1, 1]`.
    pub temperature_anomaly: f32,
    /// Regional climate variability noise for temperature and moisture.
    pub variability_t: f32,
    pub variability_m: f32,
    /// Highland line jitter noise, roughly in `[-1, 1]`.
    pub highland_jitter: f32,
}

impl Sample {
    /// Local shift of the highland line used by biome selection.
    pub fn highland_offset(&self) -> f32 {
        HIGHLAND_JITTER * self.highland_jitter
    }
}

/// Weight of the highland line jitter (about 0.09 of standard deviation).
const HIGHLAND_JITTER: f32 = 0.50;

/// Lateral displacement of the continent outline, per octave of the coast
/// warp, in units of the octave's wavelength (on the unit sphere).
const COAST_WARP: f64 = 0.5;
/// Ratio of the displacement share from one coast-warp octave to the next:
/// below 1, so the finest octaves fray the outline less than the coarse ones
/// shape it.
const COAST_WARP_DECAY: f64 = 0.8;
/// Height of the "plate" step added to the continents: a piecewise-constant
/// cell noise that gives coasts straight stretches and angular corners.
const PLATE_STEP: f32 = 0.06;
/// Inverse width of the plate borders, in plate cells: about 1/20 of a cell
/// (100–150 km on an Earth-sized planet), a ramp rather than a cliff.
const PLATE_SHARPNESS: f64 = 20.0;
/// Share of the fine continent octaves kept on low ground inland, so plains
/// and deserts are smooth; coasts and mountains keep them all.
const PLAIN_ROUGHNESS: f32 = 0.3;
/// Height above the waterline over which inland ground turns from coastal to
/// plain roughness.
const COAST_BAND: (f32, f32) = (0.03, 0.2);
/// Height above (or depth below) the waterline over which the outline detail
/// fades out: wide enough for every point the displacement can bring to the
/// coast, narrow enough not to wrinkle the lowlands.
const OUTLINE_BAND: (f32, f32) = (0.02, 0.08);
/// Weight of the second and third ridged octaves of the mountains.
const MOUNTAIN_DETAIL: f32 = 0.35;

/// Octaves of the coast warp: from the finest continent octave down to about
/// two pixels of the map, so the outline is fractal at the map's resolution
/// without aliasing below it.
fn coast_octaves(noise_scale: f64, grid: &Grid) -> u32 {
    let radius_km = grid.circumference_km as f64 / (2.0 * PI);
    let pixel_km = grid.circumference_km as f64 / grid.width as f64;
    // The finest continent octave has frequency 0.8 × s × 16.
    let mut wavelength = radius_km / (0.8 * noise_scale * 16.0);
    let mut octaves = 0;
    while wavelength > 2.0 * pixel_km && octaves < 10 {
        wavelength /= 2.0;
        octaves += 1;
    }
    octaves
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Moisture added on a floodplain (weight 1): rivers green their valleys.
const RIPARIAN_MOISTURE: f32 = 0.15;

/// Volcanic zone away from cones, as a share of the broad zone: below the
/// ash threshold (0.15), so the zone is ordinary land dotted with volcanoes,
/// each with its vent, lava flows and an ash halo.
const VOLCANIC_BACKGROUND: f32 = 0.12;
/// Cone lattice cell, km: at most one volcano per cell.
const CONE_CELL_KM: f64 = 900.0;
/// Share of the cells with a volcano.
const CONE_PRESENCE: f64 = 0.5;
/// Cone radius range, km. In a fully volcanic zone the vent biome reaches
/// about 1.2 radii, lava flows 1.8 and ash 2.6.
const CONE_KM: (f64, f64) = (50.0, 120.0);
/// Height of a cone above the terrain at its vent, in a fully volcanic zone.
const CONE_HEIGHT: f32 = 0.25;

/// Noise sources and derived constants for one world.
pub struct Sampler {
    grid: Grid,
    sea_level: f32,
    /// Absolute elevation of the sea surface.
    waterline: f32,
    noise_scale: f64,
    coast_octaves: u32,
    coast_noise: Perlin,
    plate_seed: u32,
    cone_seed: u32,
    mountain_blend: f32,
    volcanic_threshold: f32,
    elevation_noise: Perlin,
    moisture_noise: Perlin,
    continent_noise: Perlin,
    warp_noise_a: Perlin,
    warp_noise_b: Perlin,
    volcano_noise: Perlin,
    /// Continental-scale temperature anomalies (currents, monsoons, continentality).
    anomaly_noise: Perlin,
    /// Regional climate variability (soils, aspect, local circulation).
    variability_noise: Perlin,
    highland_noise: Perlin,
}

impl Sampler {
    pub fn new(p: &WorldParams) -> Self {
        let seed = p.seed;
        // Scale noise frequencies by planet size: a larger circumference
        // stretches the unit-sphere coordinates, producing broader continents.
        // Earth (40 075 km) ≡ scale 1.0.
        let noise_scale = (EARTH_CIRCUMFERENCE_KM / p.circumference_km.max(1.0)) as f64;
        let grid = p.grid();
        Self {
            coast_octaves: coast_octaves(noise_scale, &grid),
            grid,
            sea_level: p.sea_level,
            waterline: p.sea_level + p.planet.rules.waterline,
            noise_scale,
            coast_noise: Perlin::new(seed.wrapping_add(900)),
            plate_seed: seed.wrapping_add(950),
            cone_seed: seed.wrapping_add(960),
            // Stronger gravity suppresses mountain relief; sqrt dampens the effect
            // for extreme values. Baseline 0.35 at Earth gravity.
            mountain_blend: 0.35 / p.gravity_modifier().sqrt(),
            // Higher intensity lowers the threshold, making more terrain volcanic.
            volcanic_threshold: 1.0 - p.volcanic_intensity.clamp(0.0, 1.0),
            // wrapping_add: a random seed close to u32::MAX must not overflow.
            elevation_noise: Perlin::new(seed),
            moisture_noise: Perlin::new(seed.wrapping_add(1)),
            continent_noise: Perlin::new(seed.wrapping_add(100)),
            warp_noise_a: Perlin::new(seed.wrapping_add(200)),
            warp_noise_b: Perlin::new(seed.wrapping_add(201)),
            // Low-frequency noise that selects which mountain chains turn volcanic.
            volcano_noise: Perlin::new(seed.wrapping_add(300)),
            anomaly_noise: Perlin::new(seed.wrapping_add(400)),
            variability_noise: Perlin::new(seed.wrapping_add(500)),
            highland_noise: Perlin::new(seed.wrapping_add(700)),
        }
    }

    /// Volcanic cone profile at a point of the unit sphere: 1 at a cone's
    /// vent, falling with distance over a few cone radii, with lobed lava
    /// flows. Cones are one per [`CONE_CELL_KM`] cell at most, sized in km,
    /// so their size does not depend on the planet's.
    fn cone(&self, p: [f64; 3]) -> f32 {
        let radius_km = self.grid.circumference_km as f64 / (2.0 * PI);
        let f = radius_km / CONE_CELL_KM;
        let Some((d, h)) =
            nearest_point(self.cone_seed, p[0] * f, p[1] * f, p[2] * f, CONE_PRESENCE)
        else {
            return 0.0;
        };
        let size = CONE_KM.0 + (CONE_KM.1 - CONE_KM.0) * ((h >> 8) & 0xFF) as f64 / 255.0;
        // Flows reach further in some directions than others.
        let g = radius_km / (2.0 * size);
        let lobe = self
            .volcano_noise
            .get([p[0] * g + 71.0, p[1] * g - 13.0, p[2] * g + 29.0]);
        let r = d * CONE_CELL_KM / size * (1.0 + 0.4 * lobe);
        (-0.5 * r * r).exp() as f32
    }

    /// Samples every noise layer for the pixel at column `x`, row `y`.
    pub fn sample(&self, x: u32, y: u32) -> Sample {
        let s = self.noise_scale;

        // Spherical mapping of the pixel centre: longitude 0..2π, latitude
        // +π/2 (row 0, north) to -π/2 (last row, south). Projects the flat map
        // onto a unit sphere → no seams, no mirror symmetry.
        let lon = (x as f64 + 0.5) / self.grid.width as f64 * 2.0 * PI;
        let lat = self.grid.latitude(y);
        let nx = lat.cos() * lon.cos();
        let ny = lat.cos() * lon.sin();
        let nz = lat.sin();

        // Domain warping: twist coordinates before sampling for organic coastlines.
        let warp_x = self
            .warp_noise_a
            .get([nx * 2.0 * s, ny * 2.0 * s, nz * 2.0 * s]);
        let warp_y =
            self.warp_noise_b
                .get([nx * 2.0 * s + 5.2, ny * 2.0 * s + 1.3, nz * 2.0 * s + 3.7]);
        let wnx = nx + warp_x * 0.25;
        let wny = ny + warp_y * 0.25;

        // Continent shape: 5 octaves, split into the 2 coarse ones (continents,
        // basins) and the 3 fine ones (hills).
        let c = 0.8 * s;
        // Same normalisation as `fbm` with 5 octaves.
        let norm = 1.0 + 0.5 + 0.25 + 0.125 + 0.0625;
        let shape = |x: f64, y: f64, z: f64| {
            let octave = |k: u32| {
                let f = c * f64::from(1u32 << k);
                self.continent_noise.get([x * f, y * f, z * f]) as f32 / (1u32 << k) as f32
            };
            (
                (octave(0) + octave(1)) / norm,
                (octave(2) + octave(3) + octave(4)) / norm,
            )
        };
        let (coarse, fine) = shape(nx, ny, nz);
        // Continent outline: near the coast the noise is sampled at a
        // position displaced sideways by every octave finer than its own,
        // each by a share of its wavelength (thinning at finer octaves), and
        // a plate step is added, so coasts get capes, bays, islands, straight
        // stretches and corners at every scale down to the pixel. The
        // displacement fades out away from the coast, where it would only
        // wrinkle the relief.
        let coastal = 1.0
            - smoothstep(
                OUTLINE_BAND.0,
                OUTLINE_BAND.1,
                (coarse + fine - self.waterline).abs(),
            );
        let (coarse, fine) = if coastal > 0.0 {
            let (mut qx, mut qy, mut qz) = (nx, ny, nz);
            let mut f = 0.8 * s * 16.0;
            let mut share = COAST_WARP * coastal as f64;
            for k in 0..self.coast_octaves {
                let a = share / f;
                let o = 17.0 * k as f64;
                qx += a * self.coast_noise.get([nx * f + o, ny * f, nz * f]);
                qy += a * self.coast_noise.get([nx * f, ny * f + o, nz * f + 3.1]);
                qz += a * self.coast_noise.get([nx * f + 5.7, ny * f, nz * f + o]);
                f *= 2.0;
                share *= COAST_WARP_DECAY;
            }
            let (coarse, fine) = shape(qx, qy, qz);
            let pf = 2.5 * s;
            let plate = PLATE_STEP
                * cell_value(self.plate_seed, qx * pf, qy * pf, qz * pf, PLATE_SHARPNESS);
            (coarse + coastal * plate, fine)
        } else {
            (coarse, fine)
        };
        let mountain_weight = ((coarse + fine - 0.2) * 2.5).clamp(0.0, 1.0);
        // Low ground inland keeps a share of the fine octaves: plains and
        // deserts are smooth, coasts keep their outline and mountains their
        // foothills.
        let inland = smoothstep(COAST_BAND.0, COAST_BAND.1, coarse + fine - self.waterline);
        let roughness = (1.0 - (1.0 - PLAIN_ROUGHNESS) * inland).max(mountain_weight);
        let continent = coarse + fine * roughness;

        // Ridged mountains blended only onto elevated terrain: a main ridge,
        // with two finer ridged octaves concentrated on it.
        let ridge = |f: f64, o: f64| {
            ridged(
                &self.elevation_noise,
                wnx * f + o,
                wny * f - o,
                nz * f + 0.5 * o,
            )
        };
        let main = ridge(5.0 * s, 0.0);
        let detail = 0.6 * ridge(10.7 * s, 23.0) + 0.4 * ridge(22.1 * s, 41.0);
        let mountain = main * (1.0 - MOUNTAIN_DETAIL + MOUNTAIN_DETAIL * detail);

        // Volcanic zones are broad; volcanoes are cones within them, which
        // rise above the terrain (islands at sea). Away from a cone the zone
        // stays below the ash threshold.
        let volcanic_raw = fbm(&self.volcano_noise, nx * s, ny * s, nz * s, 3);
        let broad = ((volcanic_raw - self.volcanic_threshold) * 4.0).clamp(0.0, 1.0);
        let cone = if broad > 0.0 {
            self.cone([nx, ny, nz])
        } else {
            0.0
        };
        let volcanic_zone = broad * (VOLCANIC_BACKGROUND + (1.0 - VOLCANIC_BACKGROUND) * cone);

        let elevation = (continent
            + mountain * mountain_weight * self.mountain_blend
            + CONE_HEIGHT * broad * cone)
            .clamp(-1.0, 1.0);

        // Positive sea_level raises the waterline (more ocean); negative lowers it.
        let biome_elevation = (elevation - self.sea_level).clamp(-1.0, 1.0);

        let moisture = fbm(
            &self.moisture_noise,
            nx * 1.5 * s,
            ny * 1.5 * s,
            nz * 1.5 * s,
            4,
        );

        // Features of ~3 000 km: whole zones shift between continents.
        let temperature_anomaly = fbm(
            &self.anomaly_noise,
            nx * 1.2 * s,
            ny * 1.2 * s,
            nz * 1.2 * s,
            3,
        );

        // Features from ~1 000 km down to ~60 km; two independent samples.
        let f = 6.0 * s;
        let variability_t = fbm(&self.variability_noise, nx * f, ny * f, nz * f, 5);
        let variability_m = fbm(
            &self.variability_noise,
            nx * f + 17.3,
            ny * f - 4.1,
            nz * f + 9.7,
            5,
        );

        let h = 4.0 * s;
        let highland_jitter = fbm(&self.highland_noise, nx * h, ny * h, nz * h, 4);

        Sample {
            warp_x: warp_x as f32,
            warp_y: warp_y as f32,
            continent,
            mountain,
            mountain_weight,
            elevation,
            biome_elevation,
            moisture,
            volcanic_raw,
            volcanic_zone,
            temperature_anomaly,
            variability_t,
            variability_m,
            highland_jitter,
        }
    }
}

/// How a debug layer is coloured: signed values span `[-1, 1]`, unsigned `[0, 1]`.
#[derive(Clone, Copy)]
pub enum Ramp {
    Signed,
    Unsigned,
}

/// An intermediate layer kept for the diagnostic maps.
pub struct DebugLayer {
    pub name: &'static str,
    pub ramp: Ramp,
    pub values: Vec<f32>,
}

/// The output of [`generate`]: the world and the intermediate layers that
/// produced it.
pub struct Generated {
    pub world: World,
    pub debug: Vec<DebugLayer>,
}

/// A noise layer exported as a diagnostic map: name, colour ramp, extractor.
type NoiseLayer = (&'static str, Ramp, fn(&Sample) -> f32);

const NOISE_LAYERS: &[NoiseLayer] = &[
    ("noise_warp_x", Ramp::Signed, |s| s.warp_x),
    ("noise_warp_y", Ramp::Signed, |s| s.warp_y),
    ("noise_continent", Ramp::Signed, |s| s.continent),
    ("noise_mountain", Ramp::Unsigned, |s| s.mountain),
    ("noise_mountain_wt", Ramp::Unsigned, |s| s.mountain_weight),
    ("noise_elevation", Ramp::Signed, |s| s.elevation),
    ("noise_biome_elev", Ramp::Signed, |s| s.biome_elevation),
    ("noise_moisture", Ramp::Signed, |s| s.moisture),
    ("noise_volcanic_raw", Ramp::Signed, |s| s.volcanic_raw),
    ("noise_volcanic_zone", Ramp::Unsigned, |s| s.volcanic_zone),
    ("noise_temperature_anomaly", Ramp::Signed, |s| {
        s.temperature_anomaly
    }),
    ("noise_variability_t", Ramp::Signed, |s| s.variability_t),
    ("noise_variability_m", Ramp::Signed, |s| s.variability_m),
    ("noise_highland_jitter", Ramp::Signed, |s| s.highland_jitter),
];

/// Coast distances are computed up to this value; further pixels get the cap.
/// Also the full scale of the `coast_distance` debug map.
pub const COAST_DISTANCE_CAP_KM: f32 = 2_000.0;

/// Full scale of the `flow` debug map: log10 of the drainage area in km².
const FLOW_LOG_MAX: f32 = 7.0;

/// A generation step, reported to progress callbacks as it starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Stage {
    Noise,
    Erosion,
    Temperature,
    Coast,
    Moisture,
    Rivers,
    Biomes,
    /// Writing the output files (reported by the caller of [`export_world`](crate::export::export_world)).
    Export,
}

impl Stage {
    /// Every stage, in the order they run.
    pub const ALL: [Stage; 8] = [
        Stage::Noise,
        Stage::Erosion,
        Stage::Temperature,
        Stage::Coast,
        Stage::Moisture,
        Stage::Rivers,
        Stage::Biomes,
        Stage::Export,
    ];
}

/// What [`generate_with_progress`] keeps besides the world.
#[derive(Clone, Copy, Debug)]
pub struct GenerateOptions {
    /// Keep every intermediate layer for the diagnostic maps. They cost about
    /// twenty map-sized layers of memory: skip them when not exported.
    pub debug_layers: bool,
}

impl Default for GenerateOptions {
    fn default() -> Self {
        Self { debug_layers: true }
    }
}

/// The noise fields later passes need, one value per pixel (much smaller
/// than keeping every [`Sample`]).
struct NoiseFields {
    elevation: Vec<f32>,
    moisture: Vec<f32>,
    volcanic_zone: Vec<f32>,
    temperature_anomaly: Vec<f32>,
    variability_t: Vec<f32>,
    variability_m: Vec<f32>,
    highland_offset: Vec<f32>,
}

impl NoiseFields {
    fn sample(sampler: &Sampler, width: u32, height: u32) -> Self {
        let n = (width * height) as usize;
        let w = width as usize;
        let mut f = Self {
            elevation: vec![0.0; n],
            moisture: vec![0.0; n],
            volcanic_zone: vec![0.0; n],
            temperature_anomaly: vec![0.0; n],
            variability_t: vec![0.0; n],
            variability_m: vec![0.0; n],
            highland_offset: vec![0.0; n],
        };
        (
            f.elevation.par_chunks_mut(w),
            f.moisture.par_chunks_mut(w),
            f.volcanic_zone.par_chunks_mut(w),
            f.temperature_anomaly.par_chunks_mut(w),
            f.variability_t.par_chunks_mut(w),
            f.variability_m.par_chunks_mut(w),
            f.highland_offset.par_chunks_mut(w),
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(y, (e, m, v, a, vt, vm, h))| {
                for x in 0..w {
                    let s = sampler.sample(x as u32, y as u32);
                    (e[x], m[x], v[x], a[x]) = (
                        s.elevation,
                        s.moisture,
                        s.volcanic_zone,
                        s.temperature_anomaly,
                    );
                    (vt[x], vm[x], h[x]) = (s.variability_t, s.variability_m, s.highland_offset());
                }
            });
        f
    }

    fn from_samples(samples: impl ParallelIterator<Item = Sample>) -> Self {
        let picked: Vec<[f32; 7]> = samples
            .map(|s| {
                [
                    s.elevation,
                    s.moisture,
                    s.volcanic_zone,
                    s.temperature_anomaly,
                    s.variability_t,
                    s.variability_m,
                    s.highland_offset(),
                ]
            })
            .collect();
        let column = |k: usize| picked.par_iter().map(|v| v[k]).collect::<Vec<f32>>();
        Self {
            elevation: column(0),
            moisture: column(1),
            volcanic_zone: column(2),
            temperature_anomaly: column(3),
            variability_t: column(4),
            variability_m: column(5),
            highland_offset: column(6),
        }
    }
}

/// Generates a world, running every pass in order, with every debug layer.
pub fn generate(params: &WorldParams) -> Generated {
    generate_with_progress(params, GenerateOptions::default(), |_| {})
}

/// Like [`generate`], with options, calling `progress` as each stage starts.
pub fn generate_with_progress(
    params: &WorldParams,
    options: GenerateOptions,
    progress: impl Fn(Stage),
) -> Generated {
    let (width, height) = (params.width, params.height);
    let planet = &params.planet;
    let keep = options.debug_layers;

    // ── Pass 1: noise ─────────────────────────────────────────────────────────
    progress(Stage::Noise);
    // Every pixel is independent: sample rows in parallel. The result stays
    // deterministic because each pixel only depends on its coordinates.
    let sampler = Sampler::new(params);
    let pixels = || {
        (0..height)
            .into_par_iter()
            .flat_map_iter(move |y| (0..width).map(move |x| (x, y)))
    };
    let (fields, mut debug) = if keep {
        let samples: Vec<Sample> = pixels().map(|(x, y)| sampler.sample(x, y)).collect();
        let debug: Vec<DebugLayer> = NOISE_LAYERS
            .par_iter()
            .map(|&(name, ramp, value)| DebugLayer {
                name,
                ramp,
                values: samples.par_iter().map(value).collect(),
            })
            .collect();
        (NoiseFields::from_samples(samples.into_par_iter()), debug)
    } else {
        // Without debug layers, never hold every sample at once: write each
        // row's fields straight into their layers.
        (NoiseFields::sample(&sampler, width, height), Vec::new())
    };

    let rules = &planet.rules;
    let grid = params.grid();
    let n = params.pixel_count();

    // ── Pass 2: hydraulic erosion ─────────────────────────────────────────────
    progress(Stage::Erosion);
    let NoiseFields {
        mut elevation,
        moisture: moisture_noise,
        volcanic_zone: raw_volcanic_zone,
        temperature_anomaly: anomaly,
        variability_t,
        variability_m,
        highland_offset,
    } = fields;
    let erosion = erode(
        &mut elevation,
        width,
        height,
        params.sea_level + rules.waterline,
        params.erosion,
        params.seed,
    );
    if keep {
        debug.push(DebugLayer {
            name: "erosion",
            ramp: Ramp::Signed,
            // ×10: height changes are small next to the [-1, 1] elevation range.
            values: erosion
                .iter()
                .map(|d| (d * 10.0).clamp(-1.0, 1.0))
                .collect(),
        });
    }
    if keep {
        debug.push(DebugLayer {
            name: "elevation",
            ramp: Ramp::Signed,
            values: elevation.clone(),
        });
    }
    // Elevation relative to the shifted sea level: what biome selection sees.
    let mut relief: Vec<f32> = elevation
        .iter()
        .map(|e| (e - params.sea_level).clamp(-1.0, 1.0))
        .collect();

    // ── Pass 3: temperature ───────────────────────────────────────────────────
    progress(Stage::Temperature);
    let row_temperature = row_temperatures(&grid, params.axial_tilt_deg);
    let raw_temperature: Vec<f32> = relief
        .par_iter()
        .enumerate()
        .map(|(i, &r)| {
            let above_water = r - rules.waterline;
            temperature(row_temperature[i / width as usize], above_water)
                + temperature_anomaly(anomaly[i], above_water >= 0.0)
        })
        .collect();
    if keep {
        debug.push(DebugLayer {
            name: "temperature",
            ramp: Ramp::Unsigned,
            values: raw_temperature.clone(),
        });
    }

    // ── Pass 4: distance to the coast ─────────────────────────────────────────
    progress(Stage::Coast);
    // Shores are the land pixels close to open water.
    let water: Vec<bool> = relief.iter().map(|&r| r < rules.waterline).collect();
    let coast_distance_km = grid.distance_to(&water, COAST_DISTANCE_CAP_KM);
    if keep {
        debug.push(DebugLayer {
            name: "coast_distance",
            ramp: Ramp::Unsigned,
            values: coast_distance_km
                .iter()
                .map(|d| d / COAST_DISTANCE_CAP_KM)
                .collect(),
        });
    }

    // ── Pass 5: moisture ──────────────────────────────────────────────────────
    progress(Stage::Moisture);
    // Winds carry ocean evaporation inland; it rains out with distance and on
    // windward slopes, leaving rain shadows behind mountains.
    let height_above_water: Vec<f32> = relief.iter().map(|r| r - rules.waterline).collect();
    let (raw_moisture, precipitation) = moisture(&MoistureInput {
        grid,
        height_above_water: &height_above_water,
        temperature: &raw_temperature,
        noise: &moisture_noise,
    });
    if keep {
        debug.push(DebugLayer {
            name: "precipitation",
            ramp: Ramp::Signed,
            values: precipitation,
        });
    }
    if keep {
        debug.push(DebugLayer {
            name: "moisture",
            ramp: Ramp::Signed,
            values: raw_moisture.clone(),
        });
    }

    // Planet-type global offsets shift the whole climate before rivers and
    // biomes. Store the climate exactly as biome selection sees it, so the
    // exported data explains the biome of every pixel.
    let shift = planet.climate;
    let (dt, dm, dvz) = (shift.temperature, shift.moisture, shift.volcanic);
    // Regional variability is part of the stored climate, so the exported
    // layers still explain every biome.
    let mut moisture: Vec<f32> = raw_moisture
        .iter()
        .zip(&variability_m)
        .map(|(m, v)| (m + dm + regional_variability(*v, Field::Moisture)).clamp(-1.0, 1.0))
        .collect();
    let temperature: Vec<f32> = raw_temperature
        .iter()
        .zip(&variability_t)
        .map(|(t, v)| (t + dt + regional_variability(*v, Field::Temperature)).clamp(0.0, 1.0))
        .collect();
    drop((variability_t, variability_m));

    // ── Pass 6: rivers and lakes ──────────────────────────────────────────────
    progress(Stage::Rivers);
    let water = hydrology(&grid, &height_above_water, &moisture, params.seed);
    let mut river_lines = rivers::shape(
        &rivers::trace(&grid, &water),
        &grid,
        &water.filled,
        params.seed,
    );
    // Deltas and estuaries at the mouths of the large rivers.
    let deltas = delta::outlets(&mut river_lines, &grid, &height_above_water, params.seed);
    // Valleys and floodplains along the rivers. Carving never creates water,
    // so the coast distance and the drainage stay valid.
    let waterline = params.sea_level + rules.waterline;
    let mut above: Vec<f32> = elevation.iter().map(|e| e - waterline).collect();
    let mut floodplain = carve::carve(&river_lines, &grid, &water.filled, &water.lake, &mut above);
    let delta_plain = delta::plains(&deltas, &river_lines, &grid, &mut above, &mut moisture);
    for (f, d) in floodplain.iter_mut().zip(delta_plain) {
        *f = f.max(d);
    }
    for ((e, a), r) in elevation.iter_mut().zip(&above).zip(relief.iter_mut()) {
        *e = a + waterline;
        *r = (*e - params.sea_level).clamp(-1.0, 1.0);
    }
    drop(above);
    // Rivers water their floodplain: green strips across dry land.
    for (m, f) in moisture.iter_mut().zip(&floodplain) {
        *m = (*m + RIPARIAN_MOISTURE * f).clamp(-1.0, 1.0);
    }
    if keep {
        if let Some(layer) = debug.iter_mut().find(|l| l.name == "elevation") {
            layer.values.clone_from(&elevation);
        }
        debug.push(DebugLayer {
            name: "floodplain",
            ramp: Ramp::Unsigned,
            values: floodplain.clone(),
        });
    }
    // The biome layer marks rivers where they are drawn, so it matches the map.
    let river_coverage =
        rivers::coverage(&river_lines, width, height, params.circumference_km as f64);
    if keep {
        debug.push(DebugLayer {
            name: "flow",
            ramp: Ramp::Unsigned,
            values: water
                .flow_km2
                .iter()
                .map(|f| (f + 1.0).log10() / FLOW_LOG_MAX)
                .collect(),
        });
    }

    // ── Pass 7: biomes ────────────────────────────────────────────────────────
    progress(Stage::Biomes);
    let volcanic_zone: Vec<f32> = raw_volcanic_zone
        .iter()
        .map(|v| (v + dvz).clamp(0.0, 1.0))
        .collect();
    drop(raw_volcanic_zone);
    let biomes: Vec<Biome> = (0..n)
        .into_par_iter()
        .map(|i| {
            let input = BiomeInput {
                elevation: relief[i],
                moisture: moisture[i],
                temperature: temperature[i],
                volcanic_zone: volcanic_zone[i],
                coast_distance_km: coast_distance_km[i],
                lake: water.lake[i],
                river: !water.lake[i] && river_coverage[i] >= rivers::RIVER_COVERAGE,
                highland_offset: highland_offset[i],
            };
            planet.remap(rules.choose(input))
        })
        .collect();

    Generated {
        world: World {
            params: params.clone(),
            width,
            height,
            elevation,
            moisture,
            temperature,
            flow_km2: water.flow_km2,
            volcanic_zone,
            coast_distance_km,
            floodplain,
            highland_offset,
            biomes,
            rivers: river_lines,
        },
        debug,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::climate::EARTH_AXIAL_TILT_DEG;
    use crate::preset::Preset;

    pub fn params(seed: u32, planet: &str) -> WorldParams {
        WorldParams {
            width: 64,
            height: 32,
            seed,
            planet: Preset::load(planet).unwrap(),
            sea_level: 0.1,
            volcanic_intensity: 0.5,
            circumference_km: EARTH_CIRCUMFERENCE_KM,
            axial_tilt_deg: EARTH_AXIAL_TILT_DEG,
            erosion: 1.0,
        }
    }

    fn small_world(seed: u32, planet: &str) -> World {
        generate(&params(seed, planet)).world
    }

    #[test]
    fn same_seed_produces_same_world() {
        let a = small_world(42, "terran");
        let b = small_world(42, "terran");
        assert_eq!(a.biomes, b.biomes);
        assert_eq!(a.elevation, b.elevation);
    }

    #[test]
    fn seed_near_u32_max_does_not_overflow() {
        small_world(u32::MAX, "terran");
    }

    #[test]
    fn every_layer_has_one_value_per_pixel() {
        let w = small_world(7, "frozen");
        for len in [
            w.elevation.len(),
            w.moisture.len(),
            w.temperature.len(),
            w.biomes.len(),
        ] {
            assert_eq!(len, 64 * 32);
        }
    }

    #[test]
    fn stored_climate_includes_planet_offsets() {
        let frozen = small_world(5, "frozen");
        let terran = small_world(5, "terran");
        // The frozen preset cools every pixel by 0.55; both are clamped to [0, 1].
        for (f, t) in frozen.temperature.iter().zip(&terran.temperature) {
            if *t < 1.0 {
                assert!((f - (t - 0.55).max(0.0)).abs() < 1e-5, "{f} vs {t}");
            } else {
                assert!(*f >= 0.45 - 1e-5, "{f} vs {t}");
            }
        }
        assert!(frozen.moisture.iter().all(|m| (-1.0..=1.0).contains(m)));
    }

    #[test]
    fn gravity_is_earth_baseline_and_clamped() {
        let g = |circumference_km| {
            WorldParams {
                circumference_km,
                ..params(0, "terran")
            }
            .gravity_modifier()
        };
        assert!((g(EARTH_CIRCUMFERENCE_KM) - 1.0).abs() < 1e-6);
        assert_eq!(g(1.0), 0.1);
        assert_eq!(g(1e9), 5.0);
    }

    #[test]
    fn progress_reports_every_generation_stage_in_order() {
        let seen = std::sync::Mutex::new(Vec::new());
        generate_with_progress(&params(1, "terran"), GenerateOptions::default(), |s| {
            seen.lock().unwrap().push(s)
        });
        assert_eq!(seen.into_inner().unwrap(), Stage::ALL[..7]);
    }

    #[test]
    fn skipping_debug_layers_gives_the_same_world() {
        let p = params(13, "volcanic");
        let full = generate(&p);
        let lean = generate_with_progress(
            &p,
            GenerateOptions {
                debug_layers: false,
            },
            |_| {},
        );
        assert!(lean.debug.is_empty());
        assert_eq!(full.world.biomes, lean.world.biomes);
        assert_eq!(full.world.elevation, lean.world.elevation);
        assert_eq!(full.world.moisture, lean.world.moisture);
        assert_eq!(full.world.rivers.len(), lean.world.rivers.len());
    }

    #[test]
    fn debug_layers_match_the_world() {
        let g = generate(&params(11, "terran"));
        let elevation = g.debug.iter().find(|l| l.name == "elevation").unwrap();
        assert_eq!(elevation.values, g.world.elevation);
    }
}
