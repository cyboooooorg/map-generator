use crate::biome::BiomeInput;
use crate::climate::{
    Field, MoistureInput, moisture, regional_variability, row_temperatures, temperature,
    temperature_anomaly,
};
use crate::erosion::erode;
use crate::geo::Grid;
use crate::hydrology::hydrology;
use crate::noise::{EARTH_CIRCUMFERENCE_KM, fbm, ridged};
use crate::rivers;
use crate::world::*;
use noise::{NoiseFn, Perlin};
use rayon::prelude::*;
use serde::Serialize;
use std::f64::consts::PI;

/// Every intermediate noise value computed for one pixel.
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

/// Noise sources and derived constants for one world.
pub struct Sampler {
    grid: Grid,
    sea_level: f32,
    noise_scale: f64,
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
        Self {
            grid: p.grid(),
            sea_level: p.sea_level,
            // Scale noise frequencies by planet size: a larger circumference
            // stretches the unit-sphere coordinates, producing broader continents.
            // Earth (40 075 km) ≡ scale 1.0.
            noise_scale: (EARTH_CIRCUMFERENCE_KM / p.circumference_km.max(1.0)) as f64,
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

        // Continent shape: low-frequency 3D FBM.
        let continent = fbm(
            &self.continent_noise,
            nx * 0.8 * s,
            ny * 0.8 * s,
            nz * 0.8 * s,
            5,
        );

        // Ridged mountains blended only onto elevated terrain.
        let mountain = ridged(
            &self.elevation_noise,
            wnx * 5.0 * s,
            wny * 5.0 * s,
            nz * 5.0 * s,
        );
        let mountain_weight = ((continent - 0.2) * 2.5).clamp(0.0, 1.0);
        let elevation =
            (continent + mountain * mountain_weight * self.mountain_blend).clamp(-1.0, 1.0);

        // Positive sea_level raises the waterline (more ocean); negative lowers it.
        let biome_elevation = (elevation - self.sea_level).clamp(-1.0, 1.0);

        let moisture = fbm(
            &self.moisture_noise,
            nx * 1.5 * s,
            ny * 1.5 * s,
            nz * 1.5 * s,
            4,
        );

        // volcanic_zone: 0 = inert, > 0 = inside a volcanic chain.
        let volcanic_raw = fbm(&self.volcano_noise, nx * s, ny * s, nz * s, 3);
        let volcanic_zone = ((volcanic_raw - self.volcanic_threshold) * 4.0).clamp(0.0, 1.0);

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

/// Generates a world, running every pass in order.
pub fn generate(params: &WorldParams) -> Generated {
    generate_with_progress(params, |_| {})
}

/// Like [`generate`], calling `progress` as each stage starts.
pub fn generate_with_progress(params: &WorldParams, progress: impl Fn(Stage)) -> Generated {
    let (width, height) = (params.width, params.height);
    let planet = &params.planet;

    // ── Pass 1: noise ─────────────────────────────────────────────────────────
    progress(Stage::Noise);
    // Every pixel is independent: sample rows in parallel. The result stays
    // deterministic because each pixel only depends on its coordinates.
    let sampler = Sampler::new(params);
    let samples: Vec<Sample> = (0..height)
        .into_par_iter()
        .flat_map_iter(|y| (0..width).map(move |x| (x, y)))
        .map(|(x, y)| sampler.sample(x, y))
        .collect();
    let mut debug: Vec<DebugLayer> = NOISE_LAYERS
        .par_iter()
        .map(|&(name, ramp, value)| DebugLayer {
            name,
            ramp,
            values: samples.par_iter().map(value).collect(),
        })
        .collect();

    let rules = &planet.rules;
    let grid = params.grid();
    let n = params.pixel_count();

    // ── Pass 2: hydraulic erosion ─────────────────────────────────────────────
    progress(Stage::Erosion);
    let mut elevation: Vec<f32> = samples.iter().map(|s| s.elevation).collect();
    let erosion = erode(
        &mut elevation,
        width,
        height,
        params.sea_level + rules.waterline,
        params.erosion,
        params.seed,
    );
    debug.push(DebugLayer {
        name: "erosion",
        ramp: Ramp::Signed,
        // ×10: height changes are small next to the [-1, 1] elevation range.
        values: erosion
            .iter()
            .map(|d| (d * 10.0).clamp(-1.0, 1.0))
            .collect(),
    });
    debug.push(DebugLayer {
        name: "elevation",
        ramp: Ramp::Signed,
        values: elevation.clone(),
    });
    // Elevation relative to the shifted sea level: what biome selection sees.
    let relief: Vec<f32> = elevation
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
                + temperature_anomaly(samples[i].temperature_anomaly, above_water >= 0.0)
        })
        .collect();
    debug.push(DebugLayer {
        name: "temperature",
        ramp: Ramp::Unsigned,
        values: raw_temperature.clone(),
    });

    // ── Pass 4: distance to the coast ─────────────────────────────────────────
    progress(Stage::Coast);
    // Shores are the land pixels close to open water.
    let water: Vec<bool> = relief.iter().map(|&r| r < rules.waterline).collect();
    let coast_distance_km = grid.distance_to(&water, COAST_DISTANCE_CAP_KM);
    debug.push(DebugLayer {
        name: "coast_distance",
        ramp: Ramp::Unsigned,
        values: coast_distance_km
            .iter()
            .map(|d| d / COAST_DISTANCE_CAP_KM)
            .collect(),
    });

    // ── Pass 5: moisture ──────────────────────────────────────────────────────
    progress(Stage::Moisture);
    // Winds carry ocean evaporation inland; it rains out with distance and on
    // windward slopes, leaving rain shadows behind mountains.
    let height_above_water: Vec<f32> = relief.iter().map(|r| r - rules.waterline).collect();
    let moisture_noise: Vec<f32> = samples.iter().map(|s| s.moisture).collect();
    let (raw_moisture, precipitation) = moisture(&MoistureInput {
        grid,
        height_above_water: &height_above_water,
        temperature: &raw_temperature,
        noise: &moisture_noise,
    });
    debug.push(DebugLayer {
        name: "precipitation",
        ramp: Ramp::Signed,
        values: precipitation,
    });
    debug.push(DebugLayer {
        name: "moisture",
        ramp: Ramp::Signed,
        values: raw_moisture.clone(),
    });

    // Planet-type global offsets shift the whole climate before rivers and
    // biomes. Store the climate exactly as biome selection sees it, so the
    // exported data explains the biome of every pixel.
    let shift = planet.climate;
    let (dt, dm, dvz) = (shift.temperature, shift.moisture, shift.volcanic);
    // Regional variability is part of the stored climate, so the exported
    // layers still explain every biome.
    let moisture: Vec<f32> = raw_moisture
        .iter()
        .zip(&samples)
        .map(|(m, s)| {
            (m + dm + regional_variability(s.variability_m, Field::Moisture)).clamp(-1.0, 1.0)
        })
        .collect();
    let temperature: Vec<f32> = raw_temperature
        .iter()
        .zip(&samples)
        .map(|(t, s)| {
            (t + dt + regional_variability(s.variability_t, Field::Temperature)).clamp(0.0, 1.0)
        })
        .collect();

    // ── Pass 6: rivers and lakes ──────────────────────────────────────────────
    progress(Stage::Rivers);
    let water = hydrology(&grid, &height_above_water, &moisture, params.seed);
    let river_lines = rivers::shape(
        &rivers::trace(&grid, &water),
        &grid,
        &water.filled,
        params.seed,
    );
    debug.push(DebugLayer {
        name: "flow",
        ramp: Ramp::Unsigned,
        values: water
            .flow_km2
            .iter()
            .map(|f| (f + 1.0).log10() / FLOW_LOG_MAX)
            .collect(),
    });

    // ── Pass 7: biomes ────────────────────────────────────────────────────────
    progress(Stage::Biomes);
    let volcanic_zone: Vec<f32> = samples
        .iter()
        .map(|s| (s.volcanic_zone + dvz).clamp(0.0, 1.0))
        .collect();
    let highland_offset: Vec<f32> = samples.iter().map(Sample::highland_offset).collect();
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
                river: water.river[i],
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
        generate_with_progress(&params(1, "terran"), |s| seen.lock().unwrap().push(s));
        assert_eq!(seen.into_inner().unwrap(), Stage::ALL[..7]);
    }

    #[test]
    fn debug_layers_match_the_world() {
        let g = generate(&params(11, "terran"));
        let elevation = g.debug.iter().find(|l| l.name == "elevation").unwrap();
        assert_eq!(elevation.values, g.world.elevation);
    }
}
