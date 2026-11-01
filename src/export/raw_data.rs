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
//! | `biomes.png`      | 8-bit grey, pixel = biome id (see the `biomes` table)       |
use crate::biome::biome_color;
use crate::world::{Biome, World, biome_name};
use image::{GrayImage, ImageBuffer, Luma};
use serde_json::{Value, json};
use std::error::Error;
use std::path::Path;

/// Bumped whenever the layout of `raw_data/` changes incompatibly.
const FORMAT_VERSION: u32 = 1;

/// A continuous layer quantised to 16 bits over a fixed range.
struct ScalarLayer<'a> {
    name: &'static str,
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
    let scalars = [
        ScalarLayer {
            name: "elevation",
            values: &world.elevation,
            min: -1.0,
            max: 1.0,
        },
        ScalarLayer {
            name: "moisture",
            values: &world.moisture,
            min: -1.0,
            max: 1.0,
        },
        ScalarLayer {
            name: "temperature",
            values: &world.temperature,
            min: 0.0,
            max: 1.0,
        },
    ];

    let mut layers = serde_json::Map::new();
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
        layers.insert(
            layer.name.into(),
            json!({ "file": file, "encoding": "gray16", "min": layer.min, "max": layer.max }),
        );
    }

    let ids: Vec<u8> = world.biomes.iter().map(|b| b.id()).collect();
    GrayImage::from_raw(w, h, ids)
        .ok_or("layer size does not match the world size")?
        .save(dir.join("biomes.png"))?;
    layers.insert(
        "biomes".into(),
        json!({ "file": "biomes.png", "encoding": "gray8-biome-id" }),
    );

    let biomes: Vec<Value> = Biome::ALL
        .iter()
        .map(|&b| {
            let [r, g, bl] = biome_color(b);
            json!({
                "id": b.id(),
                "key": format!("{b:?}"),
                "name": biome_name(b),
                "color": format!("#{r:02X}{g:02X}{bl:02X}"),
            })
        })
        .collect();

    let metadata = json!({
        "format_version": FORMAT_VERSION,
        "width": w,
        "height": h,
        "projection": "equirectangular, row 0 = north pole, column 0 = longitude 0",
        "seed": world.seed,
        "planet_type": world.planet_type,
        "sea_level": world.sea_level,
        "volcanic_intensity": world.volcanic_intensity,
        "circumference_km": world.circumference_km,
        "gravity_modifier": world.gravity_modifier,
        "layers": layers,
        "biomes": biomes,
    });
    std::fs::write(
        dir.join("world.json"),
        serde_json::to_string_pretty(&metadata)?,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generation::generate_world;
    use crate::world::PlanetType;

    #[test]
    fn quantise_covers_the_full_range() {
        assert_eq!(quantise(-1.0, -1.0, 1.0), 0);
        assert_eq!(quantise(1.0, -1.0, 1.0), u16::MAX);
        assert_eq!(quantise(5.0, 0.0, 1.0), u16::MAX);
    }

    #[test]
    fn layers_round_trip() {
        let world = generate_world(32, 16, 9, 0.0, 0.5, PlanetType::Caustic, 40_075.0);
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
        let meta: Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("world.json")).unwrap())
                .unwrap();
        assert_eq!(meta["planet_type"], "caustic");
        assert_eq!(meta["biomes"].as_array().unwrap().len(), Biome::ALL.len());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
