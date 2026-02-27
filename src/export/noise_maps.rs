/// Export every intermediate noise map as a false-colour PNG.
///
/// The maps produced are:
///
/// | File                    | Range    | Description                                         |
/// |-------------------------|----------|-----------------------------------------------------|
/// | noise_warp_x.png        | [-1, 1]  | Domain-warp field, X axis                           |
/// | noise_warp_y.png        | [-1, 1]  | Domain-warp field, Y axis                           |
/// | noise_continent.png     | [-1, 1]  | Low-freq FBM continent shape                        |
/// | noise_mountain.png      | [ 0, 1]  | Ridged noise (mountain peaks)                       |
/// | noise_mountain_wt.png   | [ 0, 1]  | Mountain blend weight (based on continent height)   |
/// | noise_elevation.png     | [-1, 1]  | Final elevation = continent + mountain×weight×blend |
/// | noise_biome_elev.png    | [-1, 1]  | Elevation shifted by sea_level (what biomes see)    |
/// | noise_moisture.png      | [-1, 1]  | Moisture FBM                                        |
/// | noise_temperature.png   | [ 0, 1]  | Temperature (latitude gradient + elevation cooling) |
/// | noise_volcanic_raw.png  | [-1, 1]  | Raw volcanic-zone FBM                               |
/// | noise_volcanic_zone.png | [ 0, 1]  | Processed volcanic zone (threshold applied)         |
///
/// Colour encoding
/// ───────────────
/// All maps share the same "jet" ramp:
///   blue (low) → cyan → green → yellow → red (high)
///
/// Signed maps are linearly rescaled so that 0.0 → green, -1.0 → blue, +1.0 → red.
/// Unsigned maps are rescaled so that 0.0 → blue and 1.0 → red.
use crate::noise::{fbm, ridged, EARTH_CIRCUMFERENCE_KM};
use image::{Rgb, RgbImage};
use noise::Perlin;

// ── Colour map ────────────────────────────────────────────────────────────────

/// "Jet" ramp: blue → cyan → green → yellow → red.
/// `t` ∈ [0.0, 1.0].
fn jet(t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    // Piecewise linear hat functions shifted to R, G, B channels.
    let r = (1.5 - (4.0 * t - 3.0).abs()).clamp(0.0, 1.0);
    let g = (1.5 - (4.0 * t - 2.0).abs()).clamp(0.0, 1.0);
    let b = (1.5 - (4.0 * t - 1.0).abs()).clamp(0.0, 1.0);
    [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]
}

/// Colourize a signed value v ∈ [-1.0, 1.0] → jet(0.0 … 1.0).
#[inline]
fn diverge(v: f32) -> [u8; 3] {
    jet((v.clamp(-1.0, 1.0) + 1.0) * 0.5)
}

/// Colourize an unsigned value v ∈ [0.0, 1.0] → jet(0.0 … 1.0).
#[inline]
fn sequential(v: f32) -> [u8; 3] {
    jet(v.clamp(0.0, 1.0))
}

// ── PNG writer ────────────────────────────────────────────────────────────────

/// Writes `data` (length == width × height, column-major: index = q*height + r)
/// to a PNG at `path` using the provided colorizer.
fn save_map(data: &[f32], width: u32, height: u32, path: &str, colorize: impl Fn(f32) -> [u8; 3]) {
    assert_eq!(data.len(), (width * height) as usize);
    let mut img = RgbImage::new(width, height);
    for (i, &v) in data.iter().enumerate() {
        let q = (i as u32) / height; // column = x
        let r = (i as u32) % height; // row    = y
        img.put_pixel(q, r, Rgb(colorize(v)));
    }
    img.save(path)
        .unwrap_or_else(|e| eprintln!("[noise] failed to save {path}: {e}"));
    println!("[noise] wrote {path}");
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Re-samples all intermediate noise maps and writes them as false-colour PNGs
/// into `dir/`.  The generation parameters must match the ones used in
/// `generate_world` so the maps correspond to the actual world output.
pub fn export_noise_maps(
    width: i32,
    height: i32,
    seed: u32,
    sea_level: f32,
    volcanic_intensity: f32,
    circumference_km: f32,
    gravity_modifier: f32,
    dir: &str,
) {
    let w = width as u32;
    let h = height as u32;
    let n = (w * h) as usize;

    // Allocate flat buffers (column-major: index = q*height + r)
    let mut warp_x_buf = vec![0.0f32; n];
    let mut warp_y_buf = vec![0.0f32; n];
    let mut continent_buf = vec![0.0f32; n];
    let mut mountain_buf = vec![0.0f32; n];
    let mut mountain_wt_buf = vec![0.0f32; n];
    let mut elevation_buf = vec![0.0f32; n];
    let mut biome_elev_buf = vec![0.0f32; n];
    let mut moisture_buf = vec![0.0f32; n];
    let mut temperature_buf = vec![0.0f32; n];
    let mut volcanic_raw_buf = vec![0.0f32; n];
    let mut volcanic_zone_buf = vec![0.0f32; n];

    // Noise sources — identical seeds to generate_world
    let elevation_noise = Perlin::new(seed);
    let moisture_noise = Perlin::new(seed + 1);
    let continent_noise = Perlin::new(seed + 100);
    let warp_noise_a = Perlin::new(seed + 200);
    let warp_noise_b = Perlin::new(seed + 201);
    let volcano_noise = Perlin::new(seed + 300);

    let noise_scale = (EARTH_CIRCUMFERENCE_KM / circumference_km.max(1.0)) as f64;
    let mountain_blend = 0.35 / gravity_modifier.sqrt();
    let volcanic_threshold = 1.0 - volcanic_intensity.clamp(0.0, 1.0);

    use std::f64::consts::PI;

    for q in 0..width {
        for r in 0..height {
            let idx = (q as u32 * h + r as u32) as usize;

            // Spherical projection
            let lon = (q as f64 / width as f64) * 2.0 * PI;
            let lat = (r as f64 / height as f64) * PI - PI / 2.0;
            let nx = lat.cos() * lon.cos();
            let ny = lat.cos() * lon.sin();
            let nz = lat.sin();

            // Domain warp
            use noise::NoiseFn;
            let wx = warp_noise_a.get([
                nx * 2.0 * noise_scale,
                ny * 2.0 * noise_scale,
                nz * 2.0 * noise_scale,
            ]) as f32;
            let wy = warp_noise_b.get([
                nx * 2.0 * noise_scale + 5.2,
                ny * 2.0 * noise_scale + 1.3,
                nz * 2.0 * noise_scale + 3.7,
            ]) as f32;
            warp_x_buf[idx] = wx;
            warp_y_buf[idx] = wy;

            let wnx = nx + wx as f64 * 0.25;
            let wny = ny + wy as f64 * 0.25;

            // Continent
            let continent = fbm(
                &continent_noise,
                nx * 0.8 * noise_scale,
                ny * 0.8 * noise_scale,
                nz * 0.8 * noise_scale,
                5,
            );
            continent_buf[idx] = continent;

            // Mountain
            let mountain = ridged(
                &elevation_noise,
                wnx * 5.0 * noise_scale,
                wny * 5.0 * noise_scale,
                nz * 5.0 * noise_scale,
            );
            let mountain_weight = ((continent - 0.2) * 2.5).clamp(0.0, 1.0);
            mountain_buf[idx] = mountain;
            mountain_wt_buf[idx] = mountain_weight;

            // Elevation
            let elevation =
                (continent + mountain * mountain_weight * mountain_blend).clamp(-1.0, 1.0);
            let biome_elevation = (elevation - sea_level).clamp(-1.0, 1.0);
            elevation_buf[idx] = elevation;
            biome_elev_buf[idx] = biome_elevation;

            // Moisture
            let moisture = fbm(
                &moisture_noise,
                nx * 1.5 * noise_scale,
                ny * 1.5 * noise_scale,
                nz * 1.5 * noise_scale,
                4,
            );
            moisture_buf[idx] = moisture;

            // Temperature
            let latitude_norm = r as f32 / height as f32;
            let temp_gradient = 1.0 - (latitude_norm - 0.5).abs() * 2.0;
            let temperature = (temp_gradient - biome_elevation * 0.3).clamp(0.0, 1.0);
            temperature_buf[idx] = temperature;

            // Volcanic
            let volcanic_raw = fbm(
                &volcano_noise,
                nx * 1.0 * noise_scale,
                ny * 1.0 * noise_scale,
                nz * 1.0 * noise_scale,
                3,
            );
            let volcanic_zone = ((volcanic_raw - volcanic_threshold) * 4.0).clamp(0.0, 1.0);
            volcanic_raw_buf[idx] = volcanic_raw;
            volcanic_zone_buf[idx] = volcanic_zone;
        }
    }

    // Persist each map
    save_map(&warp_x_buf, w, h, &format!("{dir}/noise_warp_x.png"), diverge);
    save_map(&warp_y_buf, w, h, &format!("{dir}/noise_warp_y.png"), diverge);
    save_map(&continent_buf, w, h, &format!("{dir}/noise_continent.png"), diverge);
    save_map(&mountain_buf, w, h, &format!("{dir}/noise_mountain.png"), sequential);
    save_map(&mountain_wt_buf, w, h, &format!("{dir}/noise_mountain_wt.png"), sequential);
    save_map(&elevation_buf, w, h, &format!("{dir}/noise_elevation.png"), diverge);
    save_map(&biome_elev_buf, w, h, &format!("{dir}/noise_biome_elev.png"), diverge);
    save_map(&moisture_buf, w, h, &format!("{dir}/noise_moisture.png"), diverge);
    save_map(&temperature_buf, w, h, &format!("{dir}/noise_temperature.png"), sequential);
    save_map(&volcanic_raw_buf, w, h, &format!("{dir}/noise_volcanic_raw.png"), diverge);
    save_map(&volcanic_zone_buf, w, h, &format!("{dir}/noise_volcanic_zone.png"), sequential);
}
