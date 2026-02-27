use crate::biome::{choose_biome, planet_offsets};
use crate::noise::{EARTH_CIRCUMFERENCE_KM, fbm, ridged};
use crate::world::*;
use noise::{NoiseFn, Perlin};

pub fn generate_world(
    width: i32,
    height: i32,
    seed: u32,
    sea_level: f32,
    volcanic_intensity: f32,
    planet_type: PlanetType,
    circumference_km: f32,
) -> World {
    let elevation_noise = Perlin::new(seed);
    let moisture_noise = Perlin::new(seed + 1);
    let continent_noise = Perlin::new(seed + 100);
    let warp_noise_a = Perlin::new(seed + 200);
    let warp_noise_b = Perlin::new(seed + 201);
    // Low-frequency noise that selects which mountain chains turn volcanic.
    let volcano_noise = Perlin::new(seed + 300);

    // Scale noise frequencies by planet size: a larger circumference stretches
    // the unit-sphere coordinates, producing broader continents and ocean basins.
    // Earth (40 075 km) ≡ scale 1.0, preserving the original noise frequencies.
    let noise_scale = (EARTH_CIRCUMFERENCE_KM / circumference_km.max(1.0)) as f64;

    // Gravity modifier: assuming constant density, surface gravity scales linearly
    // with radius (and thus circumference).  Earth ≡ 1.0.
    // Clamped to a physically plausible range (≈ Moon-mass to super-Jupiter rocky).
    // Stronger gravity suppresses mountain relief; weaker gravity amplifies it.
    let gravity_modifier = (circumference_km / EARTH_CIRCUMFERENCE_KM).clamp(0.1, 5.0);
    // Mountain blend coefficient: baseline 0.35 at Earth gravity, compressed or
    // stretched proportionally.  sqrt dampens the effect for extreme values.
    let mountain_blend = 0.35 / gravity_modifier.sqrt();

    let mut tiles = Vec::new();

    for q in 0..width {
        for r in 0..height {
            use std::f64::consts::PI;

            // Proper spherical mapping: longitude 0..2π, latitude -π/2..π/2
            // Projects the flat map onto a unit sphere → no seams, no mirror symmetry
            let lon = (q as f64 / width as f64) * 2.0 * PI;
            let lat = (r as f64 / height as f64) * PI - PI / 2.0;

            let nx = lat.cos() * lon.cos();
            let ny = lat.cos() * lon.sin();
            let nz = lat.sin();

            // Domain warping: twist coordinates before sampling for organic coastlines
            let warp_x = warp_noise_a.get([
                nx * 2.0 * noise_scale,
                ny * 2.0 * noise_scale,
                nz * 2.0 * noise_scale,
            ]);
            let warp_y = warp_noise_b.get([
                nx * 2.0 * noise_scale + 5.2,
                ny * 2.0 * noise_scale + 1.3,
                nz * 2.0 * noise_scale + 3.7,
            ]);
            let wnx = nx + warp_x * 0.25;
            let wny = ny + warp_y * 0.25;

            // Continent shape: low-frequency 3D FBM — all three axes used, no symmetry
            let continent = fbm(
                &continent_noise,
                nx * 0.8 * noise_scale,
                ny * 0.8 * noise_scale,
                nz * 0.8 * noise_scale,
                5,
            );

            // Ridged mountains blended only onto elevated terrain
            let mountain = ridged(
                &elevation_noise,
                wnx * 5.0 * noise_scale,
                wny * 5.0 * noise_scale,
                nz * 5.0 * noise_scale,
            );
            let mountain_weight = ((continent - 0.2) * 2.5).clamp(0.0, 1.0);
            let elevation =
                (continent + mountain * mountain_weight * mountain_blend).clamp(-1.0, 1.0);

            // Shift elevation by sea_level before biome selection.
            // Positive sea_level raises the waterline (more ocean);
            // negative sea_level lowers it (more land).
            let biome_elevation = (elevation - sea_level).clamp(-1.0, 1.0);

            // Moisture uses 3D sphere coords so it also wraps seamlessly
            let moisture = fbm(
                &moisture_noise,
                nx * 1.5 * noise_scale,
                ny * 1.5 * noise_scale,
                nz * 1.5 * noise_scale,
                4,
            );

            // Volcanic zone: low-frequency noise determines which mountain chains are volcanic.
            // volcanic_intensity 0.0 → no volcanoes; 1.0 → most high mountain chains volcanic.
            // The threshold slides so that higher intensity makes more terrain volcanic.
            let volcanic_raw = fbm(
                &volcano_noise,
                nx * 1.0 * noise_scale,
                ny * 1.0 * noise_scale,
                nz * 1.0 * noise_scale,
                3,
            );
            let volcanic_threshold = 1.0 - volcanic_intensity.clamp(0.0, 1.0);
            // volcanic_zone: 0 = cold/neutral, >0 = inside a volcanic chain
            let volcanic_zone = ((volcanic_raw - volcanic_threshold) * 4.0).clamp(0.0, 1.0);

            // Temperature: equator warm, poles cold, high elevation colder
            let latitude_norm = r as f32 / height as f32; // 0 = south pole, 1 = north pole
            let temp_gradient = 1.0 - (latitude_norm - 0.5).abs() * 2.0;

            let temperature = temp_gradient - biome_elevation * 0.3;

            // Planet-type global offsets applied to temperature, moisture and volcanic zone.
            // These shift the entire planet climate before biome selection.
            let (dt, dm, dvz) = planet_offsets(planet_type);
            let eff_temperature = (temperature + dt).clamp(0.0, 1.0);
            let eff_moisture = (moisture + dm).clamp(-1.0, 1.0);
            let eff_volcanic_zone = (volcanic_zone + dvz).clamp(0.0, 1.0);

            let biome = choose_biome(
                biome_elevation,
                eff_moisture,
                eff_temperature,
                eff_volcanic_zone,
                planet_type,
            );

            tiles.push(Tile {
                q,
                r,
                elevation,
                moisture,
                temperature,
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
        gravity_modifier,
        tiles,
    }
}
