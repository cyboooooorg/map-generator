use crate::world::*;
use image::{Rgb, RgbImage};

// Contour lines are drawn whenever a tile and a neighbour straddle one of these levels.
const CONTOUR_LEVELS: &[f32] = &[-0.45, -0.15, 0.0, 0.15, 0.30, 0.45, 0.60, 0.75, 0.90];
// Fraction to darken a pixel by when it sits on a contour line (0.0 = no change, 1.0 = black).
const CONTOUR_DARKNESS: f32 = 0.40;

pub fn export_png(world: &World, path: &str) {
    let w = world.width as u32;
    let h = world.height as u32;
    let mut img = RgbImage::new(w, h);

    // Build a fast elevation lookup indexed by [q * height + r].
    let mut elevation: Vec<f32> = vec![0.0; (w * h) as usize];
    for tile in &world.tiles {
        elevation[(tile.q as u32 * h + tile.r as u32) as usize] = tile.elevation;
    }

    let elev_at = |q: i32, r: i32| -> Option<f32> {
        if q < 0 || r < 0 || q >= world.width || r >= world.height {
            return None;
        }
        Some(elevation[(q as u32 * h + r as u32) as usize])
    };

    // Returns true if the edge between elevations `a` and `b` crosses any contour level.
    let crosses_contour =
        |a: f32, b: f32| -> bool { CONTOUR_LEVELS.iter().any(|&lvl| (a < lvl) != (b < lvl)) };

    for tile in &world.tiles {
        let mut color = biome_color(tile.biome);
        let e = tile.elevation;

        // Check the 4-connected neighbours.
        let is_contour = [
            elev_at(tile.q - 1, tile.r),
            elev_at(tile.q + 1, tile.r),
            elev_at(tile.q, tile.r - 1),
            elev_at(tile.q, tile.r + 1),
        ]
        .iter()
        .filter_map(|n| *n)
        .any(|ne| crosses_contour(e, ne));

        if is_contour {
            // Darken the biome colour proportionally.
            color = color.map(|c| (c as f32 * (1.0 - CONTOUR_DARKNESS)) as u8);
        }

        img.put_pixel(tile.q as u32, tile.r as u32, Rgb(color));
    }

    img.save(path).unwrap();
}

fn biome_color(b: Biome) -> [u8; 3] {
    match b {
        // Water
        Biome::DeepOcean => [10, 20, 140],
        Biome::Ocean => [30, 70, 200],
        // Shore
        Biome::Beach => [220, 210, 120],
        Biome::Wetland => [90, 140, 80],
        // Cold
        Biome::IceCap => [210, 235, 255],
        Biome::Tundra => [160, 185, 155],
        Biome::Taiga => [30, 90, 60],
        // Temperate
        Biome::Shrubland => [170, 180, 80],
        Biome::Plain => [100, 200, 80],
        Biome::Forest => [20, 110, 20],
        // Tropical
        Biome::Savanna => [210, 190, 60],
        Biome::Desert => [240, 200, 100],
        Biome::Jungle => [0, 90, 20],
        // High elevation
        Biome::Mountain => [130, 120, 110],
        Biome::Snow => [245, 245, 250],
    }
}
