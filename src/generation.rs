use crate::biome::{choose_biome, planet_offsets};
use crate::noise::{EARTH_CIRCUMFERENCE_KM, fbm, ridged};
use crate::world::*;
use noise::{NoiseFn, Perlin};
use rayon::prelude::*;
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
    pub temperature: f32,
    pub volcanic_raw: f32,
    pub volcanic_zone: f32,
}

/// Noise sources and derived constants for one world.
pub struct Sampler {
    width: u32,
    height: u32,
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
}

impl Sampler {
    pub fn new(p: &WorldParams) -> Self {
        let seed = p.seed;
        Self {
            width: p.width,
            height: p.height,
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
        }
    }

    /// Samples every noise layer for the pixel at column `x`, row `y`.
    pub fn sample(&self, x: u32, y: u32) -> Sample {
        let s = self.noise_scale;

        // Spherical mapping: longitude 0..2π, latitude +π/2 (row 0, north) to
        // -π/2 (last row, south). Projects the flat map onto a unit sphere →
        // no seams, no mirror symmetry.
        let lon = (x as f64 / self.width as f64) * 2.0 * PI;
        let lat = PI / 2.0 - (y as f64 / self.height as f64) * PI;
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

        // Temperature: equator warm, poles cold, land colder with altitude.
        // Only terrain above the waterline cools: the sea surface follows the
        // latitude gradient, whatever the depth underneath.
        let latitude_norm = y as f32 / self.height as f32;
        let temp_gradient = 1.0 - (latitude_norm - 0.5).abs() * 2.0;
        let temperature = temp_gradient - biome_elevation.max(0.0) * 0.3;

        Sample {
            warp_x: warp_x as f32,
            warp_y: warp_y as f32,
            continent,
            mountain,
            mountain_weight,
            elevation,
            biome_elevation,
            moisture,
            temperature,
            volcanic_raw,
            volcanic_zone,
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
    ("noise_temperature", Ramp::Unsigned, |s| s.temperature),
    ("noise_volcanic_raw", Ramp::Signed, |s| s.volcanic_raw),
    ("noise_volcanic_zone", Ramp::Unsigned, |s| s.volcanic_zone),
];

/// Generates a world, running every pass in order.
pub fn generate(params: &WorldParams) -> Generated {
    let (width, height) = (params.width, params.height);
    let planet_type = params.planet_type;

    // ── Pass 1: noise ─────────────────────────────────────────────────────────
    // Every pixel is independent: sample rows in parallel. The result stays
    // deterministic because each pixel only depends on its coordinates.
    let sampler = Sampler::new(params);
    let samples: Vec<Sample> = (0..height)
        .into_par_iter()
        .flat_map_iter(|y| (0..width).map(move |x| (x, y)))
        .map(|(x, y)| sampler.sample(x, y))
        .collect();
    let debug: Vec<DebugLayer> = NOISE_LAYERS
        .par_iter()
        .map(|&(name, ramp, value)| DebugLayer {
            name,
            ramp,
            values: samples.par_iter().map(value).collect(),
        })
        .collect();

    // ── Pass 2: biomes ────────────────────────────────────────────────────────
    // Planet-type global offsets shift the whole climate before biome selection.
    let (dt, dm, dvz) = planet_offsets(planet_type);
    let n = params.pixel_count();
    let mut elevation = Vec::with_capacity(n);
    let mut moisture = Vec::with_capacity(n);
    let mut temperature = Vec::with_capacity(n);
    let mut biomes = Vec::with_capacity(n);
    for s in &samples {
        // Store the climate exactly as biome selection sees it, so the
        // exported data explains the biome of every pixel.
        let m = (s.moisture + dm).clamp(-1.0, 1.0);
        let t = (s.temperature + dt).clamp(0.0, 1.0);
        let vz = (s.volcanic_zone + dvz).clamp(0.0, 1.0);
        elevation.push(s.elevation);
        moisture.push(m);
        temperature.push(t);
        biomes.push(choose_biome(s.biome_elevation, m, t, vz, planet_type));
    }

    Generated {
        world: World {
            params: params.clone(),
            width,
            height,
            elevation,
            moisture,
            temperature,
            biomes,
        },
        debug,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn params(seed: u32, planet_type: PlanetType) -> WorldParams {
        WorldParams {
            width: 64,
            height: 32,
            seed,
            planet_type,
            sea_level: 0.1,
            volcanic_intensity: 0.5,
            circumference_km: EARTH_CIRCUMFERENCE_KM,
        }
    }

    fn small_world(seed: u32, planet: PlanetType) -> World {
        generate(&params(seed, planet)).world
    }

    #[test]
    fn same_seed_produces_same_world() {
        let a = small_world(42, PlanetType::Terran);
        let b = small_world(42, PlanetType::Terran);
        assert_eq!(a.biomes, b.biomes);
        assert_eq!(a.elevation, b.elevation);
    }

    #[test]
    fn seed_near_u32_max_does_not_overflow() {
        small_world(u32::MAX, PlanetType::Terran);
    }

    #[test]
    fn every_layer_has_one_value_per_pixel() {
        let w = small_world(7, PlanetType::Frozen);
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
    fn depth_never_warms_the_sea() {
        let sampler = Sampler::new(&WorldParams {
            sea_level: 0.3,
            ..params(3, PlanetType::Terran)
        });
        for x in 0..64 {
            for y in 0..32 {
                let s = sampler.sample(x, y);
                let gradient = 1.0 - (y as f32 / 32.0 - 0.5).abs() * 2.0;
                if s.biome_elevation < 0.0 {
                    assert_eq!(s.temperature, gradient, "ocean pixel ({x}, {y})");
                } else {
                    assert!(s.temperature <= gradient, "land pixel ({x}, {y})");
                }
            }
        }
    }

    #[test]
    fn stored_climate_includes_planet_offsets() {
        let w = small_world(5, PlanetType::Frozen);
        // Frozen offsets cool the planet by 0.55: nothing reaches tropical heat.
        assert!(w.temperature.iter().all(|t| (0.0..=0.45).contains(t)));
        assert!(w.moisture.iter().all(|m| (-1.0..=1.0).contains(m)));
    }

    #[test]
    fn gravity_is_earth_baseline_and_clamped() {
        let g = |circumference_km| {
            WorldParams {
                circumference_km,
                ..params(0, PlanetType::Terran)
            }
            .gravity_modifier()
        };
        assert!((g(EARTH_CIRCUMFERENCE_KM) - 1.0).abs() < 1e-6);
        assert_eq!(g(1.0), 0.1);
        assert_eq!(g(1e9), 5.0);
    }

    #[test]
    fn debug_layers_match_the_world() {
        let g = generate(&params(11, PlanetType::Terran));
        let elevation = g
            .debug
            .iter()
            .find(|l| l.name == "noise_elevation")
            .unwrap();
        assert_eq!(elevation.values, g.world.elevation);
    }
}
