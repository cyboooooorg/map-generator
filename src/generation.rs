use crate::world::*;
use noise::{NoiseFn, Perlin};

/// Fractional Brownian Motion - combines multiple octaves of noise for more detail
///
/// Description:
///
/// * `noise`: The Perlin noise generator to use for sampling.
/// * `x`, `y`: The coordinates to sample the noise at.
/// * `octaves`: The number of noise layers to combine. More octaves add detail but increase computation time.
///
/// Returns:
///
/// A single f32 value representing the combined noise at the given coordinates, normalized to the range
/// 3D FBM — samples all three sphere-surface axes so there is no mirror symmetry
fn fbm(noise: &Perlin, x: f64, y: f64, z: f64, octaves: u32) -> f32 {
    let mut value = 0.0f64;
    let mut amplitude = 1.0f64;
    let mut frequency = 1.0f64;
    let mut max_value = 0.0f64;

    for _ in 0..octaves {
        value += noise.get([x * frequency, y * frequency, z * frequency]) * amplitude;
        max_value += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    (value / max_value) as f32
}

/// Ridged noise - inverts the absolute value to produce sharp mountain peaks instead of smooth hills
fn ridged(noise: &Perlin, x: f64, y: f64, z: f64) -> f32 {
    let v = noise.get([x, y, z]) as f32;
    1.0 - v.abs()
}

pub fn generate_world(width: i32, height: i32, seed: u32, sea_level: f32) -> World {
    let elevation_noise = Perlin::new(seed);
    let moisture_noise = Perlin::new(seed + 1);
    let continent_noise = Perlin::new(seed + 100);
    let warp_noise_a = Perlin::new(seed + 200);
    let warp_noise_b = Perlin::new(seed + 201);

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
            let warp_x = warp_noise_a.get([nx * 2.0, ny * 2.0, nz * 2.0]);
            let warp_y = warp_noise_b.get([nx * 2.0 + 5.2, ny * 2.0 + 1.3, nz * 2.0 + 3.7]);
            let wnx = nx + warp_x * 0.25;
            let wny = ny + warp_y * 0.25;

            // Continent shape: low-frequency 3D FBM — all three axes used, no symmetry
            let continent = fbm(&continent_noise, nx * 0.8, ny * 0.8, nz * 0.8, 5);

            // Ridged mountains blended only onto elevated terrain
            let mountain = ridged(&elevation_noise, wnx * 5.0, wny * 5.0, nz * 5.0);
            let mountain_weight = ((continent - 0.2) * 2.5).clamp(0.0, 1.0);
            let elevation = (continent + mountain * mountain_weight * 0.35).clamp(-1.0, 1.0);

            // Shift elevation by sea_level before biome selection.
            // Positive sea_level raises the waterline (more ocean);
            // negative sea_level lowers it (more land).
            let biome_elevation = (elevation - sea_level).clamp(-1.0, 1.0);

            // Moisture uses 3D sphere coords so it also wraps seamlessly
            let moisture = fbm(&moisture_noise, nx * 1.5, ny * 1.5, nz * 1.5, 4);

            // Temperature: equator warm, poles cold, high elevation colder
            let latitude_norm = r as f32 / height as f32; // 0 = south pole, 1 = north pole
            let temp_gradient = 1.0 - (latitude_norm - 0.5).abs() * 2.0;

            let temperature = temp_gradient - biome_elevation * 0.3;

            let biome = choose_biome(biome_elevation, moisture, temperature);

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
        sea_level,
        tiles,
    }
}

/// Biome selection using a Whittaker-style multi-factor diagram.
///
/// - `e` elevation  in [-1, 1]
/// - `m` moisture   in [-1, 1]  (normalised: values above ~0 are wet)
/// - `t` temperature in [0, 1]  (0 = polar, 1 = equatorial)
fn choose_biome(e: f32, m: f32, t: f32) -> Biome {
    // ── Water bodies ─────────────────────────────────────────────────────────
    if e < -0.45 {
        return Biome::DeepOcean;
    }
    if e < -0.15 {
        return Biome::Ocean;
    }

    // ── Shoreline ─────────────────────────────────────────────────────────────
    if e < 0.0 {
        if t < 0.15 {
            return Biome::IceCap; // frozen shore / pack ice
        }
        if m > 0.3 {
            return Biome::Wetland; // mangroves / marshes
        }
        return Biome::Beach;
    }

    // ── High elevation (mountains & snowfields) ───────────────────────────────
    if e > 0.7 {
        if t < 0.35 || e > 0.88 {
            return Biome::Snow;
        }
        return Biome::Mountain;
    }

    // ── Land — branch first on temperature then on moisture ───────────────────

    // Polar / sub-polar
    if t < 0.15 {
        return Biome::IceCap;
    }

    if t < 0.30 {
        // Boreal zone
        if m > 0.2 {
            return Biome::Taiga;
        }
        return Biome::Tundra;
    }

    if t < 0.55 {
        // Temperate zone
        if m < -0.1 {
            return Biome::Shrubland;
        }
        if m > 0.35 {
            return Biome::Forest;
        }
        return Biome::Plain;
    }

    // Tropical zone
    if m < -0.05 {
        return Biome::Desert;
    }
    if m < 0.30 {
        return Biome::Savanna;
    }
    Biome::Jungle
}
