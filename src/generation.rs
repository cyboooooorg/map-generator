use crate::biome::{choose_biome, planet_offsets};
use crate::noise::{EARTH_CIRCUMFERENCE_KM, fbm, ridged};
use crate::world::*;
use noise::{NoiseFn, Perlin};
use std::f64::consts::PI;

/// Surface gravity relative to Earth for a planet of the given circumference.
///
/// Assumes constant density, so gravity scales linearly with radius (and thus
/// circumference). Clamped to a physically plausible range for rocky planets.
pub fn gravity_modifier(circumference_km: f32) -> f32 {
    (circumference_km / EARTH_CIRCUMFERENCE_KM).clamp(0.1, 5.0)
}

/// Every intermediate value computed for one tile.
///
/// Both [`generate_world`] and the noise-map exporter read from this struct,
/// so the debug maps always match the generated world.
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
    width: i32,
    height: i32,
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
    pub fn new(
        width: i32,
        height: i32,
        seed: u32,
        sea_level: f32,
        volcanic_intensity: f32,
        circumference_km: f32,
    ) -> Self {
        Self {
            width,
            height,
            sea_level,
            // Scale noise frequencies by planet size: a larger circumference
            // stretches the unit-sphere coordinates, producing broader continents.
            // Earth (40 075 km) ≡ scale 1.0.
            noise_scale: (EARTH_CIRCUMFERENCE_KM / circumference_km.max(1.0)) as f64,
            // Stronger gravity suppresses mountain relief; sqrt dampens the effect
            // for extreme values. Baseline 0.35 at Earth gravity.
            mountain_blend: 0.35 / gravity_modifier(circumference_km).sqrt(),
            // Higher intensity lowers the threshold, making more terrain volcanic.
            volcanic_threshold: 1.0 - volcanic_intensity.clamp(0.0, 1.0),
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

    /// Samples every noise layer for the tile at column `q`, row `r`.
    pub fn sample(&self, q: i32, r: i32) -> Sample {
        let s = self.noise_scale;

        // Spherical mapping: longitude 0..2π, latitude -π/2..π/2.
        // Projects the flat map onto a unit sphere → no seams, no mirror symmetry.
        let lon = (q as f64 / self.width as f64) * 2.0 * PI;
        let lat = (r as f64 / self.height as f64) * PI - PI / 2.0;
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

        // Temperature: equator warm, poles cold, high elevation colder.
        let latitude_norm = r as f32 / self.height as f32;
        let temp_gradient = 1.0 - (latitude_norm - 0.5).abs() * 2.0;
        let temperature = temp_gradient - biome_elevation * 0.3;

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

pub fn generate_world(
    width: i32,
    height: i32,
    seed: u32,
    sea_level: f32,
    volcanic_intensity: f32,
    planet_type: PlanetType,
    circumference_km: f32,
) -> World {
    let sampler = Sampler::new(
        width,
        height,
        seed,
        sea_level,
        volcanic_intensity,
        circumference_km,
    );
    // Planet-type global offsets shift the whole climate before biome selection.
    let (dt, dm, dvz) = planet_offsets(planet_type);

    let mut tiles = Vec::with_capacity((width * height) as usize);
    for q in 0..width {
        for r in 0..height {
            let s = sampler.sample(q, r);
            let biome = choose_biome(
                s.biome_elevation,
                (s.moisture + dm).clamp(-1.0, 1.0),
                (s.temperature + dt).clamp(0.0, 1.0),
                (s.volcanic_zone + dvz).clamp(0.0, 1.0),
                planet_type,
            );
            tiles.push(Tile {
                q,
                r,
                elevation: s.elevation,
                moisture: s.moisture,
                temperature: s.temperature,
                biome,
            });
        }
    }

    World {
        width,
        height,
        seed,
        planet_type,
        sea_level,
        volcanic_intensity,
        circumference_km,
        gravity_modifier: gravity_modifier(circumference_km),
        tiles,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small_world(seed: u32, planet: PlanetType) -> World {
        generate_world(64, 32, seed, 0.1, 0.5, planet, 40_075.0)
    }

    #[test]
    fn same_seed_produces_same_world() {
        let a = small_world(42, PlanetType::Terran);
        let b = small_world(42, PlanetType::Terran);
        let biomes = |w: &World| w.tiles.iter().map(|t| t.biome).collect::<Vec<_>>();
        assert_eq!(biomes(&a), biomes(&b));
    }

    #[test]
    fn seed_near_u32_max_does_not_overflow() {
        small_world(u32::MAX, PlanetType::Terran);
    }

    #[test]
    fn world_has_one_tile_per_pixel() {
        let w = small_world(7, PlanetType::Frozen);
        assert_eq!(w.tiles.len(), 64 * 32);
    }

    #[test]
    fn gravity_is_earth_baseline_and_clamped() {
        assert!((gravity_modifier(EARTH_CIRCUMFERENCE_KM) - 1.0).abs() < 1e-6);
        assert_eq!(gravity_modifier(1.0), 0.1);
        assert_eq!(gravity_modifier(1e9), 5.0);
    }
}
