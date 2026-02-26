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

pub fn generate_world(
    width: i32,
    height: i32,
    seed: u32,
    sea_level: f32,
    volcanic_intensity: f32,
) -> World {
    let elevation_noise = Perlin::new(seed);
    let moisture_noise = Perlin::new(seed + 1);
    let continent_noise = Perlin::new(seed + 100);
    let warp_noise_a = Perlin::new(seed + 200);
    let warp_noise_b = Perlin::new(seed + 201);
    // Low-frequency noise that selects which mountain chains turn volcanic.
    let volcano_noise = Perlin::new(seed + 300);

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

            // Volcanic zone: low-frequency noise determines which mountain chains are volcanic.
            // volcanic_intensity 0.0 → no volcanoes; 1.0 → most high mountain chains volcanic.
            // The threshold slides so that higher intensity makes more terrain volcanic.
            let volcanic_raw = fbm(&volcano_noise, nx * 1.0, ny * 1.0, nz * 1.0, 3);
            let volcanic_threshold = 1.0 - volcanic_intensity.clamp(0.0, 1.0);
            // volcanic_zone: 0 = cold/neutral, >0 = inside a volcanic chain
            let volcanic_zone = ((volcanic_raw - volcanic_threshold) * 4.0).clamp(0.0, 1.0);

            // Temperature: equator warm, poles cold, high elevation colder
            let latitude_norm = r as f32 / height as f32; // 0 = south pole, 1 = north pole
            let temp_gradient = 1.0 - (latitude_norm - 0.5).abs() * 2.0;

            let temperature = temp_gradient - biome_elevation * 0.3;

            let biome = choose_biome(biome_elevation, moisture, temperature, volcanic_zone);

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
        volcanic_intensity,
        tiles,
    }
}

/// Biome selection using a Whittaker-style multi-factor diagram.
///
/// Dispatches to one of four altitude bands:
/// ocean → shore → highland → land (further split by temperature zone).
/// The volcanic modifier is applied last and can override any land/highland biome.
///
/// - `e`  elevation    in [-1, 1]
/// - `m`  moisture     in [-1, 1]  (values above ~0 are wet)
/// - `t`  temperature  in [ 0, 1] (0 = polar, 1 = equatorial)
/// - `vz` volcanic_zone in [0, 1] (0 = inert, 1 = fully volcanic)
fn choose_biome(e: f32, m: f32, t: f32, vz: f32) -> Biome {
    let base = if e < -0.15 {
        ocean_biome(e)
    } else if e < 0.0 {
        shore_biome(t, m)
    } else if e > 0.7 {
        highland_biome(e, t)
    } else {
        land_biome(t, m)
    };
    apply_volcanic(base, e, vz)
}

// ── Altitude bands ────────────────────────────────────────────────────────────

fn ocean_biome(e: f32) -> Biome {
    if e < -0.45 {
        Biome::DeepOcean
    } else {
        Biome::Ocean
    }
}

fn shore_biome(t: f32, m: f32) -> Biome {
    if t < 0.15 {
        Biome::IceCap
    }
    // frozen shore / pack ice
    else if m > 0.3 {
        Biome::Wetland
    }
    // mangroves / marshes
    else {
        Biome::Beach
    }
}

fn highland_biome(e: f32, t: f32) -> Biome {
    if t < 0.35 || e > 0.88 {
        Biome::Snow
    } else {
        Biome::Mountain
    }
}

// ── Land: temperature zones, each resolved by moisture ───────────────────────

fn land_biome(t: f32, m: f32) -> Biome {
    if t < 0.15 {
        return Biome::IceCap;
    } // polar
    if t < 0.30 {
        return boreal_biome(m);
    }
    if t < 0.55 {
        return temperate_biome(m);
    }
    tropical_biome(m)
}

fn boreal_biome(m: f32) -> Biome {
    if m > 0.2 { Biome::Taiga } else { Biome::Tundra }
}

fn temperate_biome(m: f32) -> Biome {
    if m < -0.1 {
        Biome::Shrubland
    } else if m > 0.35 {
        Biome::Forest
    } else {
        Biome::Plain
    }
}

fn tropical_biome(m: f32) -> Biome {
    if m < -0.05 {
        Biome::Desert
    } else if m < 0.30 {
        Biome::Savanna
    } else {
        Biome::Jungle
    }
}

// ── Volcanic modifier ─────────────────────────────────────────────────────────

/// Optionally overrides a biome when it sits inside an active volcanic zone.
///
/// - `vz` volcanic_zone in [0, 1]: 0 = inert, higher = stronger volcanic activity.
/// - `e`  biome elevation, used to distinguish caldera from flank from foothill.
///
/// Override ladder (strongest condition wins):
///   Volcano  — summit/caldera : Mountain|Snow, e > 0.80, vz > 0.55
///   LavaField — flanks        : Mountain|Snow,           vz > 0.30
///   AshLand  — lower slopes   : Mountain|Snow|Shrubland|Plain|Tundra, e > 0.30, vz > 0.15
fn apply_volcanic(biome: Biome, e: f32, vz: f32) -> Biome {
    if vz <= 0.0 {
        return biome;
    }
    match biome {
        // Summit / caldera → active vent
        Biome::Mountain | Biome::Snow if e > 0.80 && vz > 0.55 => Biome::Volcano,
        // Volcanic flanks → cooling lava flows
        Biome::Mountain | Biome::Snow if vz > 0.30 => Biome::LavaField,
        // Lower slopes and surrounding terrain → ash wasteland
        Biome::Mountain | Biome::Snow | Biome::Shrubland | Biome::Plain | Biome::Tundra
            if e > 0.30 && vz > 0.15 =>
        {
            Biome::AshLand
        }
        other => other,
    }
}
