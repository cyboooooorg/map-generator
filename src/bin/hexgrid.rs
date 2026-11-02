//! Resamples a generated world onto a hexagonal grid, for games and other
//! tile-based uses.
//!
//! Reads `<world>/raw_data/` (see the README for the format) and writes
//! `<world>/hex/hex.json` and `<world>/hex/hex.png`. The hexes are regular in
//! map space (the equirectangular projection), not on the sphere: polar hexes
//! cover less real ground than equatorial ones.
use clap::Parser;
use image::{Rgb, RgbImage};
use serde_json::{Value, json};
use std::error::Error;
use std::path::{Path, PathBuf};

/// Resample a generated world onto a pointy-top hexagonal grid.
#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// World directory produced by map-generator (contains raw_data/)
    world: PathBuf,

    /// Number of hex columns around the planet (the grid wraps east-west)
    #[arg(long, default_value_t = 96, value_parser = clap::value_parser!(u32).range(4..=4096))]
    columns: u32,

    /// Land hexes whose largest river drains at least this area (km²) get a river flag
    #[arg(long, default_value_t = 500_000.0)]
    river_flow: f32,
}

/// Pointy-top hex grid laid over a `width × height` map, in "odd-r" offset
/// coordinates: odd rows are shifted half a hex east. Columns wrap around.
#[derive(Clone, Copy, Debug)]
struct HexGrid {
    columns: u32,
    rows: u32,
    /// Hex width in map pixels (flat side to flat side).
    hex_width: f32,
    /// Hex size in map pixels (centre to corner).
    size: f32,
}

impl HexGrid {
    fn new(width: u32, height: u32, columns: u32) -> Self {
        let hex_width = width as f32 / columns as f32;
        let size = hex_width / 3f32.sqrt();
        // Row centres are 1.5 × size apart, the first one a full size below the top.
        let rows = ((height as f32 - size) / (1.5 * size)).ceil().max(0.0) as u32 + 1;
        Self {
            columns,
            rows,
            hex_width,
            size,
        }
    }

    /// Centre of hex `(col, row)` in map pixels.
    fn centre(&self, col: u32, row: u32) -> (f32, f32) {
        let shift = if row % 2 == 1 { 0.5 } else { 0.0 };
        (
            (col as f32 + 0.5 + shift) * self.hex_width,
            self.size + row as f32 * 1.5 * self.size,
        )
    }

    /// The hex containing map point `(px, py)`, as `(col, row)`.
    fn locate(&self, px: f32, py: f32) -> (u32, u32) {
        // Into axial coordinates relative to the centre of hex (0, 0).
        let x = px - 0.5 * self.hex_width;
        let y = py - self.size;
        let q = (3f32.sqrt() / 3.0 * x - y / 3.0) / self.size;
        let r = (2.0 / 3.0 * y) / self.size;
        // Cube rounding.
        let (cx, cz) = (q, r);
        let cy = -cx - cz;
        let (mut rx, ry, mut rz) = (cx.round(), cy.round(), cz.round());
        let (dx, dy, dz) = ((rx - cx).abs(), (ry - cy).abs(), (rz - cz).abs());
        if dx > dy && dx > dz {
            rx = -ry - rz;
        } else if dy <= dz {
            rz = -rx - ry;
        }
        let row = (rz as i64).clamp(0, self.rows as i64 - 1);
        let col = rx as i64 + (rz as i64 - (rz as i64 & 1)) / 2;
        (col.rem_euclid(self.columns as i64) as u32, row as u32)
    }

    fn index(&self, col: u32, row: u32) -> usize {
        (row * self.columns + col) as usize
    }

    fn len(&self) -> usize {
        (self.rows * self.columns) as usize
    }
}

/// A decoded 16-bit layer from raw_data/.
fn read_scalar(dir: &Path, meta: &Value, name: &str) -> Result<Vec<f32>, Box<dyn Error>> {
    let layer = &meta["layers"][name];
    let file = layer["file"].as_str().ok_or("missing layer file")?;
    let (min, max) = (
        layer["min"].as_f64().ok_or("missing min")? as f32,
        layer["max"].as_f64().ok_or("missing max")? as f32,
    );
    let img = image::open(dir.join(file))?.to_luma16();
    Ok(img
        .pixels()
        .map(|p| min + p[0] as f32 / 65535.0 * (max - min))
        .collect())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let raw = args.world.join("raw_data");
    let meta: Value = serde_json::from_str(&std::fs::read_to_string(raw.join("world.json"))?)?;
    let (width, height) = (
        meta["width"].as_u64().ok_or("missing width")? as u32,
        meta["height"].as_u64().ok_or("missing height")? as u32,
    );
    let biome_table = meta["biomes"].as_array().ok_or("missing biome table")?;
    let river_id = biome_table
        .iter()
        .find(|b| b["key"] == "River")
        .and_then(|b| b["id"].as_u64());
    let is_water: Vec<bool> = biome_table
        .iter()
        .map(|b| b["water"].as_bool().unwrap_or(false))
        .collect();
    let colors: Vec<[u8; 3]> = biome_table
        .iter()
        .map(|b| {
            let hex = b["color"].as_str().unwrap_or("#000000");
            let c = u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or(0);
            [(c >> 16) as u8, (c >> 8) as u8, c as u8]
        })
        .collect();

    let biomes = image::open(
        raw.join(
            meta["layers"]["biomes"]["file"]
                .as_str()
                .unwrap_or("biomes.png"),
        ),
    )?
    .to_luma8();
    let elevation = read_scalar(&raw, &meta, "elevation")?;
    let moisture = read_scalar(&raw, &meta, "moisture")?;
    let temperature = read_scalar(&raw, &meta, "temperature")?;
    // Stored as log10(1 + km²).
    let flow: Vec<f32> = read_scalar(&raw, &meta, "flow")?
        .iter()
        .map(|v| 10f32.powf(*v) - 1.0)
        .collect();

    let grid = HexGrid::new(width, height, args.columns);

    // ── Aggregate pixels per hex ─────────────────────────────────────────────
    let mut owner = vec![0usize; (width * height) as usize];
    let mut votes = vec![[0u32; 256]; grid.len()];
    let mut sums = vec![[0.0f64; 3]; grid.len()];
    let mut count = vec![0u32; grid.len()];
    let mut max_flow = vec![0.0f32; grid.len()];
    for y in 0..height {
        for x in 0..width {
            let (col, row) = grid.locate(x as f32 + 0.5, y as f32 + 0.5);
            let h = grid.index(col, row);
            let i = (y * width + x) as usize;
            owner[i] = h;
            let biome = biomes.get_pixel(x, y)[0];
            // A river is a thin line: it never wins a hex, the flow flag shows it.
            if Some(biome as u64) != river_id {
                votes[h][biome as usize] += 1;
            }
            max_flow[h] = max_flow[h].max(flow[i]);
            sums[h][0] += elevation[i] as f64;
            sums[h][1] += moisture[i] as f64;
            sums[h][2] += temperature[i] as f64;
            count[h] += 1;
        }
    }
    let majority: Vec<u8> = votes
        .iter()
        .map(|v| (0..256).max_by_key(|&b| v[b]).unwrap_or(0) as u8)
        .collect();
    let mean = |h: usize, k: usize| (sums[h][k] / count[h].max(1) as f64) as f32;
    let river: Vec<bool> = (0..grid.len())
        .map(|h| {
            let water = is_water.get(majority[h] as usize).copied().unwrap_or(false);
            !water && max_flow[h] >= args.river_flow
        })
        .collect();

    // ── hex.json ─────────────────────────────────────────────────────────────
    let out = args.world.join("hex");
    std::fs::create_dir_all(&out)?;
    let round = |v: f32| (v * 1000.0).round() / 1000.0;
    let hex_json = json!({
        "format_version": 1,
        "layout": "pointy-top, odd-r offset (odd rows shifted half a hex east), columns wrap east-west",
        "columns": grid.columns,
        "rows": grid.rows,
        "hex_width_px": grid.hex_width,
        "hex_size_px": grid.size,
        "source": { "width": width, "height": height, "seed": meta["seed"], "planet_type": meta["planet_type"] },
        "cells": {
            "order": "row-major: index = row * columns + col",
            "biome": majority,
            "elevation": (0..grid.len()).map(|h| round(mean(h, 0))).collect::<Vec<_>>(),
            "moisture": (0..grid.len()).map(|h| round(mean(h, 1))).collect::<Vec<_>>(),
            "temperature": (0..grid.len()).map(|h| round(mean(h, 2))).collect::<Vec<_>>(),
            "max_flow_km2": max_flow.iter().map(|f| f.round()).collect::<Vec<_>>(),
            "river": river,
        },
        "biomes": meta["biomes"],
    });
    std::fs::write(out.join("hex.json"), serde_json::to_string(&hex_json)?)?;

    // ── hex.png: majority colours, outlines, river dots ──────────────────────
    let outline_dark = 0.6;
    let dot_radius = grid.size * 0.3;
    let img = RgbImage::from_fn(width, height, |x, y| {
        let i = (y * width + x) as usize;
        let h = owner[i];
        let mut c = colors
            .get(majority[h] as usize)
            .copied()
            .unwrap_or([0, 0, 0]);
        let (col, row) = (h as u32 % grid.columns, h as u32 / grid.columns);
        let (cx, cy) = grid.centre(col, row);
        let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
        // Wrap the horizontal distance for hexes straddling the seam.
        let dx = (dx + width as f32 / 2.0).rem_euclid(width as f32) - width as f32 / 2.0;
        let east = owner[(y * width + (x + 1) % width) as usize];
        let south = if y + 1 < height {
            owner[i + width as usize]
        } else {
            h
        };
        if east != h || south != h {
            c = c.map(|v| (v as f32 * outline_dark) as u8);
        } else if river[h] && (dx * dx + dy * dy).sqrt() < dot_radius {
            c = [40, 95, 215];
        }
        Rgb(c)
    });
    img.save(out.join("hex.png"))?;

    println!(
        "Hex grid {}×{} (hex ≈ {:.1} px wide, {:.0} km at the equator) → {}/",
        grid.columns,
        grid.rows,
        grid.hex_width,
        meta["circumference_km"].as_f64().unwrap_or(0.0) / grid.columns as f64,
        out.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_centre_maps_to_its_own_hex() {
        let g = HexGrid::new(1920, 1080, 96);
        for row in 0..g.rows {
            for col in 0..g.columns {
                let (x, y) = g.centre(col, row);
                if x < 1920.0 && y < 1080.0 {
                    assert_eq!(g.locate(x, y), (col, row));
                }
            }
        }
    }

    #[test]
    fn grid_wraps_east_west() {
        let g = HexGrid::new(1920, 1080, 96);
        // A point just past the east edge belongs to a hex of the first column.
        let (x, y) = g.centre(0, 0);
        assert_eq!(g.locate(x + 1920.0, y), (0, 0));
    }

    #[test]
    fn rows_cover_the_whole_map() {
        let g = HexGrid::new(1920, 1080, 96);
        let (_, last) = g.locate(10.0, 1079.5);
        assert!(last < g.rows);
        let (_, y) = g.centre(0, g.rows - 1);
        assert!(y + g.size >= 1080.0);
    }
}
