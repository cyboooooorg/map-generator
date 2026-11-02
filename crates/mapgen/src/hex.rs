//! Resampling of a generated world onto a hexagonal grid, for games and other
//! tile-based uses.
//!
//! Reads `<world>/raw_data/` and writes `<world>/hex/hex.json` (a [`HexMap`])
//! and `<world>/hex/hex.png`. The hexes are regular in map space (the
//! equirectangular projection), not on the sphere: polar hexes cover less
//! real ground than equatorial ones.
use crate::export::raw_data::{BiomeInfo, WorldMetadata, read_scalar};
use image::{Rgb, RgbImage};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Land hexes whose largest river drains at least this area (km²) get a river flag.
pub const DEFAULT_RIVER_FLOW_KM2: f32 = 500_000.0;

/// Pointy-top hex grid laid over a `width × height` map, in "odd-r" offset
/// coordinates: odd rows are shifted half a hex east. Columns wrap around.
#[derive(Clone, Copy, Debug)]
pub struct HexGrid {
    pub columns: u32,
    pub rows: u32,
    /// Hex width in map pixels (flat side to flat side).
    pub hex_width: f32,
    /// Hex size in map pixels (centre to corner).
    pub size: f32,
}

impl HexGrid {
    pub fn new(width: u32, height: u32, columns: u32) -> Self {
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
    pub fn centre(&self, col: u32, row: u32) -> (f32, f32) {
        let shift = if row % 2 == 1 { 0.5 } else { 0.0 };
        (
            (col as f32 + 0.5 + shift) * self.hex_width,
            self.size + row as f32 * 1.5 * self.size,
        )
    }

    /// The hex containing map point `(px, py)`, as `(col, row)`.
    pub fn locate(&self, px: f32, py: f32) -> (u32, u32) {
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

    pub fn index(&self, col: u32, row: u32) -> usize {
        (row * self.columns + col) as usize
    }

    pub fn cell_count(&self) -> usize {
        (self.rows * self.columns) as usize
    }
}

/// Content of `hex/hex.json`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct HexMap {
    pub format_version: u32,
    pub layout: String,
    pub columns: u32,
    pub rows: u32,
    /// Hex width in map pixels (flat side to flat side).
    pub hex_width_px: f32,
    /// Hex size in map pixels (centre to corner).
    pub hex_size_px: f32,
    pub source: HexSource,
    pub cells: HexCells,
    /// The biome table of the world.
    pub biomes: Vec<BiomeInfo>,
}

/// The world a hex map was made from.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct HexSource {
    pub width: u32,
    pub height: u32,
    pub seed: u32,
    pub planet_type: String,
}

/// One value per hex for each field, row-major: `index = row × columns + col`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct HexCells {
    pub order: String,
    /// Majority biome id (rivers never win a hex; they set `river` instead).
    pub biome: Vec<u8>,
    pub elevation: Vec<f32>,
    pub moisture: Vec<f32>,
    pub temperature: Vec<f32>,
    /// Largest drainage area (km²) of a pixel in the hex.
    pub max_flow_km2: Vec<f32>,
    /// Land hex crossed by a large river.
    pub river: Vec<bool>,
}

/// Resamples the world in `world_dir` onto a grid of `columns` hex columns,
/// writes `hex/hex.json` and `hex/hex.png` next to its raw data, and returns
/// the hex map.
pub fn resample(world_dir: &Path, columns: u32, river_flow_km2: f32) -> Result<HexMap, String> {
    let raw = world_dir.join("raw_data");
    let text = std::fs::read_to_string(raw.join("world.json"))
        .map_err(|e| format!("{}: {e}", raw.join("world.json").display()))?;
    let meta: WorldMetadata =
        serde_json::from_str(&text).map_err(|e| format!("world.json: {e}"))?;
    let (width, height) = (meta.width, meta.height);
    let river_id = meta
        .biomes
        .iter()
        .find(|b| b.key == crate::biome::Biome::River)
        .map(|b| b.id);
    let biomes = image::open(raw.join(&meta.layers.biomes.file))
        .map_err(|e| format!("{}: {e}", meta.layers.biomes.file))?
        .to_luma8();
    let elevation = read_scalar(&raw, &meta.layers.elevation)?;
    let moisture = read_scalar(&raw, &meta.layers.moisture)?;
    let temperature = read_scalar(&raw, &meta.layers.temperature)?;
    let flow = read_scalar(&raw, &meta.layers.flow)?;

    let grid = HexGrid::new(width, height, columns);
    let n = grid.cell_count();

    // ── Aggregate pixels per hex ──────────────────────────────────────────────
    let mut owner = vec![0usize; (width * height) as usize];
    let mut votes = vec![[0u32; 256]; n];
    let mut sums = vec![[0.0f64; 3]; n];
    let mut count = vec![0u32; n];
    let mut max_flow = vec![0.0f32; n];
    for y in 0..height {
        for x in 0..width {
            let (col, row) = grid.locate(x as f32 + 0.5, y as f32 + 0.5);
            let h = grid.index(col, row);
            let i = (y * width + x) as usize;
            owner[i] = h;
            let biome = biomes.get_pixel(x, y)[0];
            // A river is a thin line: it never wins a hex, the flow flag shows it.
            if Some(biome) != river_id {
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
    let mean = |h: usize, k: usize| {
        let v = (sums[h][k] / count[h].max(1) as f64) as f32;
        (v * 1000.0).round() / 1000.0
    };
    let is_water = |id: u8| meta.biomes.get(id as usize).is_some_and(|b| b.water);
    let river: Vec<bool> = (0..n)
        .map(|h| !is_water(majority[h]) && max_flow[h] >= river_flow_km2)
        .collect();

    let map = HexMap {
        format_version: 1,
        layout:
            "pointy-top, odd-r offset (odd rows shifted half a hex east), columns wrap east-west"
                .into(),
        columns: grid.columns,
        rows: grid.rows,
        hex_width_px: grid.hex_width,
        hex_size_px: grid.size,
        source: HexSource {
            width,
            height,
            seed: meta.seed,
            planet_type: meta.planet_type.clone(),
        },
        cells: HexCells {
            order: "row-major: index = row * columns + col".into(),
            elevation: (0..n).map(|h| mean(h, 0)).collect(),
            moisture: (0..n).map(|h| mean(h, 1)).collect(),
            temperature: (0..n).map(|h| mean(h, 2)).collect(),
            max_flow_km2: max_flow.iter().map(|f| f.round()).collect(),
            biome: majority,
            river,
        },
        biomes: meta.biomes,
    };

    let out = world_dir.join("hex");
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let json = serde_json::to_string(&map).map_err(|e| e.to_string())?;
    std::fs::write(out.join("hex.json"), json).map_err(|e| e.to_string())?;
    render(&map, &grid, &owner)
        .save(out.join("hex.png"))
        .map_err(|e| e.to_string())?;
    Ok(map)
}

/// hex.png: majority colours, darkened outlines, a blue dot on river hexes.
fn render(map: &HexMap, grid: &HexGrid, owner: &[usize]) -> RgbImage {
    let (width, height) = (map.source.width, map.source.height);
    let colors: Vec<[u8; 3]> = map
        .biomes
        .iter()
        .map(|b| {
            let c = u32::from_str_radix(b.color.trim_start_matches('#'), 16).unwrap_or(0);
            [(c >> 16) as u8, (c >> 8) as u8, c as u8]
        })
        .collect();
    let outline_dark = 0.6;
    let dot_radius = grid.size * 0.3;
    RgbImage::from_fn(width, height, |x, y| {
        let i = (y * width + x) as usize;
        let h = owner[i];
        let mut c = colors
            .get(map.cells.biome[h] as usize)
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
        } else if map.cells.river[h] && (dx * dx + dy * dy).sqrt() < dot_radius {
            c = [40, 95, 215];
        }
        Rgb(c)
    })
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

    #[test]
    fn resamples_a_generated_world() {
        use crate::export::{ExportOptions, export_world};
        use crate::generation::generate;
        use crate::request::WorldRequest;
        let request = WorldRequest {
            planet: Some("terran".into()),
            seed: Some(5),
            width: Some(256),
            height: Some(128),
            ..Default::default()
        };
        let params = request.resolve(&mut rand::rng()).unwrap().params;
        let dir = std::env::temp_dir().join(format!("mapgen-hex-{}", std::process::id()));
        export_world(
            &generate(&params),
            &dir,
            ExportOptions {
                raw_data: true,
                noise_maps: false,
            },
        )
        .unwrap();

        let map = resample(&dir, 16, DEFAULT_RIVER_FLOW_KM2).unwrap();
        assert_eq!(map.columns, 16);
        assert_eq!(map.cells.biome.len(), (map.columns * map.rows) as usize);
        assert!(dir.join("hex/hex.json").exists() && dir.join("hex/hex.png").exists());
        let read: HexMap =
            serde_json::from_str(&std::fs::read_to_string(dir.join("hex/hex.json")).unwrap())
                .unwrap();
        assert_eq!(read.cells.biome, map.cells.biome);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
