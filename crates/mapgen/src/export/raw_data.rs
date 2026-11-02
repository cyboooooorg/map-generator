//! Raw world data, meant to be loaded by other programs.
//!
//! Each layer is a lossless PNG with one pixel per map pixel (row 0 = north
//! pole), and `world.json` holds the planet parameters plus everything needed
//! to decode the layers:
//!
//! | File              | Encoding                                                    |
//! |-------------------|-------------------------------------------------------------|
//! | `world.json`      | planet parameters, layer descriptions, biome table           |
//! | `elevation.png`   | 16-bit grey, `value = min + pixel / 65535 * (max - min)`    |
//! | `moisture.png`    | 16-bit grey, same formula                                   |
//! | `temperature.png` | 16-bit grey, same formula                                   |
//! | `flow.png`        | 16-bit grey, `log10(1 + km²)` with the same formula          |
//! | `biomes.png`      | 8-bit grey, pixel = biome id (see the `biomes` table)       |
use crate::biome::biome_color;
use crate::generation::{COAST_DISTANCE_CAP_KM, Sample, Sampler};
use crate::preset::Preset;
use crate::world::WorldParams;
use crate::world::{Biome, World, biome_name};
use image::{GrayImage, ImageBuffer, Luma};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::Path;

/// Bumped whenever the layout of `raw_data/` changes incompatibly.
/// Version 2 adds the volcanic zone, coast distance and highland offset layers.
pub const FORMAT_VERSION: u32 = 2;

/// Content of `raw_data/world.json`.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct WorldMetadata {
    pub format_version: u32,
    pub width: u32,
    pub height: u32,
    pub projection: String,
    pub seed: u32,
    /// Name of the planet preset.
    pub planet_type: String,
    /// The full resolved preset, so the world can be regenerated even if the
    /// preset file changes or disappears.
    pub preset: Preset,
    pub sea_level: f32,
    pub volcanic_intensity: f32,
    pub circumference_km: f32,
    pub gravity_modifier: f32,
    pub axial_tilt_deg: f32,
    pub erosion: f32,
    pub layers: Layers,
    /// Every biome, in id order.
    pub biomes: Vec<BiomeInfo>,
}

/// The layer files of `raw_data/`.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Layers {
    pub elevation: ScalarLayerInfo,
    pub moisture: ScalarLayerInfo,
    pub temperature: ScalarLayerInfo,
    /// `log10(1 + km²)` of the drainage area.
    pub flow: ScalarLayerInfo,
    /// Volcanic zone as biome selection sees it (format version 2 and later).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volcanic_zone: Option<ScalarLayerInfo>,
    /// Distance to open water in km, capped (format version 2 and later).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coast_distance: Option<ScalarLayerInfo>,
    /// Local shift of the highland line (format version 2 and later).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub highland_offset: Option<ScalarLayerInfo>,
    pub biomes: BiomeLayerInfo,
}

/// A 16-bit layer: pixel `p` decodes to `min + p / 65535 × (max − min)`.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct ScalarLayerInfo {
    pub file: String,
    /// `gray16`, or `gray16-log10` when the decoded value is a base-10 logarithm.
    pub encoding: String,
    pub min: f32,
    pub max: f32,
}

/// The 8-bit biome layer: pixel = biome id.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct BiomeLayerInfo {
    pub file: String,
    pub encoding: String,
}

/// One entry of the biome table.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct BiomeInfo {
    pub id: u8,
    pub key: Biome,
    /// Display name.
    pub name: String,
    /// `#RRGGBB`.
    pub color: String,
    /// Whether the biome is a water surface.
    pub water: bool,
}

/// A continuous layer quantised to 16 bits over a fixed range.
struct ScalarLayer<'a> {
    name: &'static str,
    /// Encoding name written to world.json.
    encoding: &'static str,
    values: &'a [f32],
    min: f32,
    max: f32,
}

/// Maps `v` from `[min, max]` to the full `u16` range.
fn quantise(v: f32, min: f32, max: f32) -> u16 {
    ((v.clamp(min, max) - min) / (max - min) * u16::MAX as f32).round() as u16
}

/// Writes `world.json` and one PNG per layer into `dir`.
pub fn export_raw_data(world: &World, dir: &Path) -> Result<(), Box<dyn Error>> {
    let (w, h) = (world.width, world.height);
    // Drainage areas span 0 to millions of km²: store them on a log scale.
    let log_flow: Vec<f32> = world.flow_km2.iter().map(|f| (f + 1.0).log10()).collect();
    let scalars = [
        ScalarLayer {
            name: "elevation",
            encoding: "gray16",
            values: &world.elevation,
            min: -1.0,
            max: 1.0,
        },
        ScalarLayer {
            name: "moisture",
            encoding: "gray16",
            values: &world.moisture,
            min: -1.0,
            max: 1.0,
        },
        ScalarLayer {
            name: "temperature",
            encoding: "gray16",
            values: &world.temperature,
            min: 0.0,
            max: 1.0,
        },
        ScalarLayer {
            name: "flow",
            encoding: "gray16-log10",
            values: &log_flow,
            min: 0.0,
            max: 8.0,
        },
        ScalarLayer {
            name: "volcanic_zone",
            encoding: "gray16",
            values: &world.volcanic_zone,
            min: 0.0,
            max: 1.0,
        },
        ScalarLayer {
            name: "coast_distance",
            encoding: "gray16",
            values: &world.coast_distance_km,
            min: 0.0,
            max: COAST_DISTANCE_CAP_KM,
        },
        ScalarLayer {
            name: "highland_offset",
            encoding: "gray16",
            values: &world.highland_offset,
            min: -1.0,
            max: 1.0,
        },
    ];

    let mut infos = Vec::with_capacity(scalars.len());
    for layer in &scalars {
        let pixels: Vec<u16> = layer
            .values
            .iter()
            .map(|&v| quantise(v, layer.min, layer.max))
            .collect();
        let file = format!("{}.png", layer.name);
        ImageBuffer::<Luma<u16>, _>::from_raw(w, h, pixels)
            .ok_or("layer size does not match the world size")?
            .save(dir.join(&file))?;
        infos.push(ScalarLayerInfo {
            file,
            encoding: layer.encoding.to_string(),
            min: layer.min,
            max: layer.max,
        });
    }

    let ids: Vec<u8> = world.biomes.iter().map(|b| b.id()).collect();
    GrayImage::from_raw(w, h, ids)
        .ok_or("layer size does not match the world size")?
        .save(dir.join("biomes.png"))?;

    let [
        elevation,
        moisture,
        temperature,
        flow,
        volcanic_zone,
        coast_distance,
        highland_offset,
    ]: [ScalarLayerInfo; 7] = infos
        .try_into()
        .map_err(|_| "unexpected number of scalar layers")?;
    let metadata = WorldMetadata {
        format_version: FORMAT_VERSION,
        width: w,
        height: h,
        projection: "equirectangular, row 0 = north pole, column 0 = longitude 0".into(),
        seed: world.params.seed,
        planet_type: world.params.planet.name.clone(),
        preset: world.params.planet.clone(),
        sea_level: world.params.sea_level,
        volcanic_intensity: world.params.volcanic_intensity,
        circumference_km: world.params.circumference_km,
        gravity_modifier: world.params.gravity_modifier(),
        axial_tilt_deg: world.params.axial_tilt_deg,
        erosion: world.params.erosion,
        layers: Layers {
            elevation,
            moisture,
            temperature,
            flow,
            volcanic_zone: Some(volcanic_zone),
            coast_distance: Some(coast_distance),
            highland_offset: Some(highland_offset),
            biomes: BiomeLayerInfo {
                file: "biomes.png".into(),
                encoding: "gray8-biome-id".into(),
            },
        },
        biomes: Biome::ALL
            .iter()
            .map(|&b| {
                let [r, g, bl] = biome_color(b);
                BiomeInfo {
                    id: b.id(),
                    key: b,
                    name: biome_name(b).into(),
                    color: format!("#{r:02X}{g:02X}{bl:02X}"),
                    water: b.is_water(),
                }
            })
            .collect(),
    };
    std::fs::write(
        dir.join("world.json"),
        serde_json::to_string_pretty(&metadata)?,
    )?;
    Ok(())
}

/// Decodes one 16-bit layer of a `raw_data/` directory.
pub(crate) fn read_scalar(raw_dir: &Path, layer: &ScalarLayerInfo) -> Result<Vec<f32>, String> {
    let img = image::open(raw_dir.join(&layer.file))
        .map_err(|e| format!("{}: {e}", layer.file))?
        .to_luma16();
    let scale = (layer.max - layer.min) / 65535.0;
    let log = layer.encoding == "gray16-log10";
    Ok(img
        .pixels()
        .map(|p| {
            let v = layer.min + p[0] as f32 * scale;
            if log { 10f32.powf(v) - 1.0 } else { v }
        })
        .collect())
}

/// Reloads a generated world from its directory (the one holding `raw_data/`).
///
/// Values come back quantised to 16 bits. Worlds of format version 1, which
/// lack the volcanic zone and coast distance layers, get them recomputed from
/// their parameters and elevation.
pub fn load_world(world_dir: &Path) -> Result<World, String> {
    let raw = world_dir.join("raw_data");
    let path = raw.join("world.json");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let meta: WorldMetadata =
        serde_json::from_str(&text).map_err(|e| format!("world.json: {e}"))?;
    let params = WorldParams {
        width: meta.width,
        height: meta.height,
        seed: meta.seed,
        planet: meta.preset,
        sea_level: meta.sea_level,
        volcanic_intensity: meta.volcanic_intensity,
        circumference_km: meta.circumference_km,
        axial_tilt_deg: meta.axial_tilt_deg,
        erosion: meta.erosion,
    };
    let elevation = read_scalar(&raw, &meta.layers.elevation)?;
    let biome_ids = image::open(raw.join(&meta.layers.biomes.file))
        .map_err(|e| format!("{}: {e}", meta.layers.biomes.file))?
        .to_luma8();
    let biomes = biome_ids
        .pixels()
        .map(|p| {
            Biome::ALL
                .get(p[0] as usize)
                .copied()
                .ok_or_else(|| format!("unknown biome id {}", p[0]))
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Version 1 worlds: resample the noise-derived layers from the seed.
    let resampled = |pick: &dyn Fn(&Sample) -> f32| -> Vec<f32> {
        let sampler = Sampler::new(&params);
        (0..params.height)
            .flat_map(|y| (0..params.width).map(move |x| (x, y)))
            .map(|(x, y)| pick(&sampler.sample(x, y)))
            .collect()
    };
    let shift = params.planet.climate.volcanic;
    let volcanic_zone = match &meta.layers.volcanic_zone {
        Some(info) => read_scalar(&raw, info)?,
        None => resampled(&|s| (s.volcanic_zone + shift).clamp(0.0, 1.0)),
    };
    let highland_offset = match &meta.layers.highland_offset {
        Some(info) => read_scalar(&raw, info)?,
        None => resampled(&Sample::highland_offset),
    };
    let coast_distance_km = match &meta.layers.coast_distance {
        Some(info) => read_scalar(&raw, info)?,
        None => {
            let waterline = params.sea_level + params.planet.rules.waterline;
            let water: Vec<bool> = elevation.iter().map(|&e| e < waterline).collect();
            params.grid().distance_to(&water, COAST_DISTANCE_CAP_KM)
        }
    };

    Ok(World {
        width: meta.width,
        height: meta.height,
        moisture: read_scalar(&raw, &meta.layers.moisture)?,
        temperature: read_scalar(&raw, &meta.layers.temperature)?,
        flow_km2: read_scalar(&raw, &meta.layers.flow)?,
        params,
        elevation,
        volcanic_zone,
        coast_distance_km,
        highland_offset,
        biomes,
        rivers: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generation::generate;
    use crate::preset::Preset;
    use crate::world::WorldParams;

    #[test]
    fn quantise_covers_the_full_range() {
        assert_eq!(quantise(-1.0, -1.0, 1.0), 0);
        assert_eq!(quantise(1.0, -1.0, 1.0), u16::MAX);
        assert_eq!(quantise(5.0, 0.0, 1.0), u16::MAX);
    }

    #[test]
    fn layers_round_trip() {
        let world = generate(&WorldParams {
            width: 32,
            height: 16,
            seed: 9,
            planet: Preset::load("caustic").unwrap(),
            sea_level: 0.0,
            volcanic_intensity: 0.5,
            circumference_km: 40_075.0,
            axial_tilt_deg: 23.44,
            erosion: 1.0,
        })
        .world;
        let world_dir =
            std::env::temp_dir().join(format!("map-generator-test-{}", std::process::id()));
        let dir = world_dir.join("raw_data");
        std::fs::create_dir_all(&dir).unwrap();
        export_raw_data(&world, &dir).unwrap();

        let biomes = image::open(dir.join("biomes.png")).unwrap().to_luma8();
        let elevation = image::open(dir.join("elevation.png")).unwrap().to_luma16();
        for y in 0..world.height {
            for x in 0..world.width {
                let i = world.index(x, y);
                assert_eq!(biomes.get_pixel(x, y)[0], world.biomes[i].id());
                let decoded = -1.0 + elevation.get_pixel(x, y)[0] as f32 / 65535.0 * 2.0;
                assert!((decoded - world.elevation[i]).abs() < 1e-4);
            }
        }
        // world.json reads back into its typed form, preset included.
        let meta: WorldMetadata =
            serde_json::from_str(&std::fs::read_to_string(dir.join("world.json")).unwrap())
                .unwrap();
        assert_eq!(meta.planet_type, "caustic");
        assert_eq!(meta.preset.name, "caustic");
        assert_eq!(meta.biomes.len(), Biome::ALL.len());
        assert_eq!(meta.layers.flow.encoding, "gray16-log10");

        // The world reloads with every layer, and its preset can remap.
        let back = load_world(&world_dir).unwrap();
        assert_eq!(back.biomes, world.biomes);
        assert_eq!(back.params.planet.name, "caustic");
        assert!(
            back.volcanic_zone
                .iter()
                .zip(&world.volcanic_zone)
                .all(|(a, b)| (a - b).abs() < 1e-4)
        );
        assert!(
            back.coast_distance_km
                .iter()
                .zip(&world.coast_distance_km)
                .all(|(a, b)| (a - b).abs() < 0.05)
        );
        assert_eq!(back.params.planet.remap(Biome::Ocean), Biome::CausticLake);

        // A version 1 world (without the new layers) gets them recomputed.
        let mut v1 = meta;
        v1.format_version = 1;
        v1.layers.volcanic_zone = None;
        v1.layers.coast_distance = None;
        v1.layers.highland_offset = None;
        std::fs::write(dir.join("world.json"), serde_json::to_string(&v1).unwrap()).unwrap();
        let old = load_world(&world_dir).unwrap();
        assert!(
            old.volcanic_zone
                .iter()
                .zip(&world.volcanic_zone)
                .all(|(a, b)| (a - b).abs() < 1e-3)
        );
        assert!(
            old.coast_distance_km
                .iter()
                .zip(&world.coast_distance_km)
                .all(|(a, b)| (a - b).abs() < 1.0)
        );
        assert!(
            old.highland_offset
                .iter()
                .zip(&world.highland_offset)
                .all(|(a, b)| (a - b).abs() < 1e-3)
        );
        std::fs::remove_dir_all(world_dir).unwrap();
    }
}
