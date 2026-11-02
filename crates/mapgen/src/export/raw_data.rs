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
use crate::preset::Preset;
use crate::world::{Biome, World, biome_name};
use image::{GrayImage, ImageBuffer, Luma};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::Path;

/// Bumped whenever the layout of `raw_data/` changes incompatibly.
const FORMAT_VERSION: u32 = 1;

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

    let [elevation, moisture, temperature, flow]: [ScalarLayerInfo; 4] = infos
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
        let dir = std::env::temp_dir().join(format!("map-generator-test-{}", std::process::id()));
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
        std::fs::remove_dir_all(dir).unwrap();
    }
}
