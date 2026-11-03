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
//! | `clouds.png`      | 8-bit grey, cloud cover `pixel / 255` (worlds with clouds)  |
//! | `surface.png`     | 8-bit RGB, `world.png` without its overlaid lines           |
//!
//! [`load_world`] (and the hex resampling) refuse a world whose
//! `format_version` is newer than [`FORMAT_VERSION`], or whose layers do not
//! have the size `world.json` announces.
use crate::biome::{biome_color, surface_color};
use crate::climate::{TEMPERATURE_RANGE, celsius};
use crate::generation::{COAST_DISTANCE_CAP_KM, Sampler};
use crate::preset::Preset;
use crate::rivers::{RiverLine, RiversFile};
use crate::world::WorldParams;
use crate::world::{Biome, World, biome_name};
use image::{GrayImage, ImageBuffer, Luma};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::Path;

fn earth_pressure() -> f32 {
    1.0
}

/// Name of the river polylines file.
pub const RIVERS_FILE: &str = "rivers.json";

/// Bumped whenever the layout of `raw_data/` changes incompatibly.
/// Version 2 adds the volcanic zone, coast distance and highland offset
/// layers, and the river polylines; version 3 the floodplain layer;
/// version 4 stores temperatures in °C instead of the former `[0, 1]`
/// scale; version 5 adds the cloud cover; version 6 the surface render
/// without overlaid lines.
pub const FORMAT_VERSION: u32 = 6;

/// First format version whose temperatures are in °C.
const CELSIUS_VERSION: u32 = 4;

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
    /// Surface pressure in bar. Worlds made before it existed had Earth's.
    #[serde(default = "earth_pressure")]
    pub pressure_bar: f32,
    /// Age of the oldest surviving surface, in Ga. Worlds made before it
    /// existed have none (and no craters).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface_age_ga: Option<f32>,
    pub layers: Layers,
    /// River polylines file (`rivers.json`, a `RiversFile`), format version 2.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rivers: Option<String>,
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
    /// Floodplain weight of the river valleys (format version 3 and later).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub floodplain: Option<ScalarLayerInfo>,
    /// Cloud cover in `[0, 1]`, 8 bits (`gray8`: `min + pixel / 255 ×
    /// (max − min)`); format version 5 and later, absent on worlds without
    /// clouds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clouds: Option<ScalarLayerInfo>,
    /// The hillshaded render of `world.png` without its reference and
    /// contour lines, which the viewer draws itself (format version 6 and
    /// later).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface: Option<ImageLayerInfo>,
    pub biomes: BiomeLayerInfo,
}

/// A colour image layer.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct ImageLayerInfo {
    pub file: String,
    /// `rgb8`.
    pub encoding: String,
}

/// A 16-bit layer: pixel `p` decodes to `min + p / 65535 × (max − min)`
/// (`gray8` layers: `min + p / 255 × (max − min)`).
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
    /// Classification colour, `#RRGGBB` (see [`biome_color`]).
    pub color: String,
    /// Surface colour of the hillshaded renders, `#RRGGBB` (see
    /// [`surface_color`]). Empty in worlds written before it existed.
    #[serde(default)]
    pub surface_color: String,
    /// Whether the biome is a water surface.
    pub water: bool,
}

/// A continuous layer quantised to 16 bits over a fixed range.
struct ScalarLayer<'a> {
    name: &'static str,
    /// Encoding name written to world.json.
    encoding: &'static str,
    values: &'a [f32],
    /// Stored as `log10(1 + value)` (encoding `gray16-log10`).
    log: bool,
    range: (f32, f32),
}

/// Ranges the 16-bit layers are quantised over.
const ELEVATION: (f32, f32) = (-1.0, 1.0);
const MOISTURE: (f32, f32) = (-1.0, 1.0);
const TEMPERATURE: (f32, f32) = TEMPERATURE_RANGE;
/// `log10(1 + km²)`: drainage areas span 0 to millions of km².
const FLOW_LOG10: (f32, f32) = (0.0, 8.0);
const VOLCANIC_ZONE: (f32, f32) = (0.0, 1.0);
const COAST_DISTANCE: (f32, f32) = (0.0, COAST_DISTANCE_CAP_KM);
const HIGHLAND_OFFSET: (f32, f32) = (-1.0, 1.0);
const FLOODPLAIN: (f32, f32) = (0.0, 1.0);

/// Maps `v` from `[min, max]` to the full `u16` range.
fn quantise(v: f32, min: f32, max: f32) -> u16 {
    ((v.clamp(min, max) - min) / (max - min) * u16::MAX as f32).round() as u16
}

/// Writes `world.json` and one PNG per layer into `dir`. `surface`, the
/// map's pixels without overlaid lines (see [`super::render_pixels`]), is
/// written as `surface.png` when given.
pub fn export_raw_data(
    world: &World,
    dir: &Path,
    surface: Option<&[[u8; 3]]>,
) -> Result<(), Box<dyn Error>> {
    let (w, h) = (world.width, world.height);
    let layer = |name, values, range| ScalarLayer {
        name,
        encoding: "gray16",
        values,
        log: false,
        range,
    };
    let scalars = [
        layer("elevation", &world.elevation, ELEVATION),
        layer("moisture", &world.moisture, MOISTURE),
        layer("temperature", &world.temperature, TEMPERATURE),
        ScalarLayer {
            encoding: "gray16-log10",
            log: true,
            ..layer("flow", &world.flow_km2, FLOW_LOG10)
        },
        layer("volcanic_zone", &world.volcanic_zone, VOLCANIC_ZONE),
        layer("coast_distance", &world.coast_distance_km, COAST_DISTANCE),
        layer("highland_offset", &world.highland_offset, HIGHLAND_OFFSET),
        layer("floodplain", &world.floodplain, FLOODPLAIN),
    ];

    let mut infos = Vec::with_capacity(scalars.len());
    for layer in &scalars {
        let (min, max) = layer.range;
        // Quantised on the fly: no full-size copy of the transformed values.
        let pixels: Vec<u16> = layer
            .values
            .iter()
            .map(|&v| quantise(if layer.log { (v + 1.0).log10() } else { v }, min, max))
            .collect();
        let file = format!("{}.png", layer.name);
        ImageBuffer::<Luma<u16>, _>::from_raw(w, h, pixels)
            .ok_or("layer size does not match the world size")?
            .save(dir.join(&file))?;
        infos.push(ScalarLayerInfo {
            file,
            encoding: layer.encoding.to_string(),
            min,
            max,
        });
    }

    let ids: Vec<u8> = world.biomes.iter().map(|b| b.id()).collect();
    GrayImage::from_raw(w, h, ids)
        .ok_or("layer size does not match the world size")?
        .save(dir.join("biomes.png"))?;

    let clouds = if world.clouds.is_empty() {
        None
    } else {
        let cover: Vec<u8> = world
            .clouds
            .iter()
            .map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8)
            .collect();
        GrayImage::from_raw(w, h, cover)
            .ok_or("layer size does not match the world size")?
            .save(dir.join("clouds.png"))?;
        Some(ScalarLayerInfo {
            file: "clouds.png".into(),
            encoding: "gray8".into(),
            min: 0.0,
            max: 1.0,
        })
    };

    let surface = match surface {
        Some(pixels) => {
            if pixels.len() != (w * h) as usize {
                return Err("surface size does not match the world size".into());
            }
            // Saved from the borrowed pixels: no copy of the render.
            image::save_buffer(
                dir.join("surface.png"),
                pixels.as_flattened(),
                w,
                h,
                image::ExtendedColorType::Rgb8,
            )?;
            Some(ImageLayerInfo {
                file: "surface.png".into(),
                encoding: "rgb8".into(),
            })
        }
        None => None,
    };

    let [
        elevation,
        moisture,
        temperature,
        flow,
        volcanic_zone,
        coast_distance,
        highland_offset,
        floodplain,
    ]: [ScalarLayerInfo; 8] = infos
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
        pressure_bar: world.params.pressure_bar,
        surface_age_ga: Some(world.params.surface_age_ga),
        layers: Layers {
            elevation,
            moisture,
            temperature,
            flow,
            volcanic_zone: Some(volcanic_zone),
            coast_distance: Some(coast_distance),
            highland_offset: Some(highland_offset),
            floodplain: Some(floodplain),
            clouds,
            surface,
            biomes: BiomeLayerInfo {
                file: "biomes.png".into(),
                encoding: "gray8-biome-id".into(),
            },
        },
        rivers: Some(RIVERS_FILE.into()),
        biomes: biome_table(),
    };
    let rivers = RiversFile::from_lines(&world.rivers, w, h);
    std::fs::write(dir.join(RIVERS_FILE), serde_json::to_string(&rivers)?)?;
    std::fs::write(
        dir.join("world.json"),
        serde_json::to_string_pretty(&metadata)?,
    )?;
    Ok(())
}

/// Every biome, in id order: the `biomes` table of `world.json`.
pub fn biome_table() -> Vec<BiomeInfo> {
    Biome::ALL
        .iter()
        .map(|&b| {
            let hex = |[r, g, b]: [u8; 3]| format!("#{r:02X}{g:02X}{b:02X}");
            BiomeInfo {
                id: b.id(),
                key: b,
                name: biome_name(b).into(),
                color: hex(biome_color(b)),
                surface_color: hex(surface_color(b)),
                water: b.is_water(),
            }
        })
        .collect()
}

/// Reads and checks `world.json` of a `raw_data/` directory. Worlds written
/// by a newer version of the generator are refused: their layers may not
/// mean what this version expects.
pub(crate) fn read_metadata(raw_dir: &Path) -> Result<WorldMetadata, String> {
    let path = raw_dir.join("world.json");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let meta: WorldMetadata =
        serde_json::from_str(&text).map_err(|e| format!("world.json: {e}"))?;
    if meta.format_version > FORMAT_VERSION {
        return Err(format!(
            "world.json: format version {} is newer than the supported version {FORMAT_VERSION}: \
             update the generator",
            meta.format_version
        ));
    }
    if meta.width == 0 || meta.height == 0 {
        return Err(format!(
            "world.json: invalid size {} × {}",
            meta.width, meta.height
        ));
    }
    Ok(meta)
}

/// Checks that a layer image has the size `world.json` announces.
fn check_size(file: &str, (w, h): (u32, u32), meta: &WorldMetadata) -> Result<(), String> {
    if (w, h) != (meta.width, meta.height) {
        return Err(format!(
            "{file}: {w} × {h} pixels, but world.json announces {} × {}",
            meta.width, meta.height
        ));
    }
    Ok(())
}

/// Decodes the 8-bit biome id layer of a `raw_data/` directory.
pub(crate) fn read_biome_ids(raw_dir: &Path, meta: &WorldMetadata) -> Result<GrayImage, String> {
    let file = &meta.layers.biomes.file;
    let img = image::open(raw_dir.join(file))
        .map_err(|e| format!("{file}: {e}"))?
        .into_luma8();
    check_size(file, img.dimensions(), meta)?;
    Ok(img)
}

/// A 16-bit layer as stored in `raw_data/`, decoded on access: about half
/// the memory of `f32` values, for the same values.
#[derive(Clone, Debug)]
pub struct Layer16 {
    raw: Vec<u16>,
    min: f32,
    /// `(max − min) / 65535`.
    scale: f32,
    /// The decoded value is a base-10 logarithm of `1 + value`.
    log: bool,
}

impl Layer16 {
    /// Quantises `values` over `[min, max]`, as the export does.
    pub fn quantise(values: &[f32], (min, max): (f32, f32)) -> Self {
        Self {
            raw: values.iter().map(|&v| quantise(v, min, max)).collect(),
            min,
            scale: (max - min) / 65535.0,
            log: false,
        }
    }

    /// A layer of zeros (lower bound 0).
    fn zeros(len: usize) -> Self {
        Self {
            raw: vec![0; len],
            min: 0.0,
            scale: 1.0 / 65535.0,
            log: false,
        }
    }

    /// Value of pixel `i` (row-major).
    #[inline]
    pub fn get(&self, i: usize) -> f32 {
        let v = self.min + self.raw[i] as f32 * self.scale;
        if self.log { 10f32.powf(v) - 1.0 } else { v }
    }

    pub fn len(&self) -> usize {
        self.raw.len()
    }

    pub fn is_empty(&self) -> bool {
        self.raw.is_empty()
    }

    /// Every value, decoded.
    pub fn decode(&self) -> Vec<f32> {
        (0..self.raw.len())
            .into_par_iter()
            .map(|i| self.get(i))
            .collect()
    }

    /// Bytes held.
    pub fn bytes(&self) -> usize {
        self.raw.len() * size_of::<u16>()
    }
}

/// Decodes one 16-bit layer of a `raw_data/` directory.
fn read_layer(
    raw_dir: &Path,
    layer: &ScalarLayerInfo,
    meta: &WorldMetadata,
) -> Result<Layer16, String> {
    let img = image::open(raw_dir.join(&layer.file))
        .map_err(|e| format!("{}: {e}", layer.file))?
        .into_luma16();
    check_size(&layer.file, img.dimensions(), meta)?;
    Ok(Layer16 {
        raw: img.into_raw(),
        min: layer.min,
        scale: (layer.max - layer.min) / 65535.0,
        log: layer.encoding == "gray16-log10",
    })
}

/// A world reloaded from `raw_data/`, its continuous layers kept as stored
/// (16 bits per value, see [`Layer16`]). [`LoadedWorld::into_world`] decodes
/// them into a [`World`].
pub struct LoadedWorld {
    pub params: WorldParams,
    pub width: u32,
    pub height: u32,
    pub elevation: Layer16,
    pub moisture: Layer16,
    pub temperature: Layer16,
    /// Drainage area in km² (stored on a log scale).
    pub flow_km2: Layer16,
    pub volcanic_zone: Layer16,
    pub coast_distance_km: Layer16,
    pub highland_offset: Layer16,
    pub floodplain: Layer16,
    pub biomes: Vec<Biome>,
    pub rivers: Vec<RiverLine>,
}

impl LoadedWorld {
    /// A generated world as it would be reloaded after its export: every
    /// layer quantised to 16 bits.
    pub fn from_world(world: &World) -> Self {
        let log_flow: Vec<f32> = world.flow_km2.iter().map(|f| (f + 1.0).log10()).collect();
        let mut flow_km2 = Layer16::quantise(&log_flow, FLOW_LOG10);
        flow_km2.log = true;
        Self {
            params: world.params.clone(),
            width: world.width,
            height: world.height,
            elevation: Layer16::quantise(&world.elevation, ELEVATION),
            moisture: Layer16::quantise(&world.moisture, MOISTURE),
            temperature: Layer16::quantise(&world.temperature, TEMPERATURE),
            flow_km2,
            volcanic_zone: Layer16::quantise(&world.volcanic_zone, VOLCANIC_ZONE),
            coast_distance_km: Layer16::quantise(&world.coast_distance_km, COAST_DISTANCE),
            highland_offset: Layer16::quantise(&world.highland_offset, HIGHLAND_OFFSET),
            floodplain: Layer16::quantise(&world.floodplain, FLOODPLAIN),
            biomes: world.biomes.clone(),
            rivers: world.rivers.clone(),
        }
    }

    /// Row-major index of pixel `(x, y)`.
    #[inline]
    pub fn index(&self, x: u32, y: u32) -> usize {
        (y * self.width + x) as usize
    }

    /// Bytes held by the layers (river polylines excluded).
    pub fn bytes(&self) -> usize {
        [
            &self.elevation,
            &self.moisture,
            &self.temperature,
            &self.flow_km2,
            &self.volcanic_zone,
            &self.coast_distance_km,
            &self.highland_offset,
            &self.floodplain,
        ]
        .iter()
        .map(|l| l.bytes())
        .sum::<usize>()
            + self.biomes.len() * size_of::<Biome>()
    }

    /// Decodes every layer into a [`World`].
    pub fn into_world(self) -> World {
        let layers = [
            &self.elevation,
            &self.moisture,
            &self.temperature,
            &self.flow_km2,
            &self.volcanic_zone,
            &self.coast_distance_km,
            &self.highland_offset,
            &self.floodplain,
        ];
        let decoded: Vec<Vec<f32>> = layers.par_iter().map(|l| l.decode()).collect();
        let [
            elevation,
            moisture,
            temperature,
            flow_km2,
            volcanic_zone,
            coast_distance_km,
            highland_offset,
            floodplain,
        ]: [Vec<f32>; 8] = decoded.try_into().expect("eight layers");
        World {
            params: self.params,
            width: self.width,
            height: self.height,
            elevation,
            moisture,
            temperature,
            flow_km2,
            volcanic_zone,
            coast_distance_km,
            floodplain,
            highland_offset,
            biomes: self.biomes,
            rivers: self.rivers,
            clouds: Vec::new(),
        }
    }
}

/// Reloads a generated world from its directory (the one holding `raw_data/`),
/// keeping its layers as stored (see [`LoadedWorld`]). The layer files are
/// decoded in parallel.
///
/// Worlds of format version 1, which lack the volcanic zone, highland offset
/// and coast distance layers, get them recomputed from their parameters and
/// elevation (then quantised like the stored ones); worlds before version 3
/// get no floodplains.
pub fn load_layers(world_dir: &Path) -> Result<LoadedWorld, String> {
    let raw = world_dir.join("raw_data");
    let meta = read_metadata(&raw)?;
    let params = WorldParams {
        width: meta.width,
        height: meta.height,
        seed: meta.seed,
        planet: meta.preset.clone(),
        sea_level: meta.sea_level,
        volcanic_intensity: meta.volcanic_intensity,
        circumference_km: meta.circumference_km,
        axial_tilt_deg: meta.axial_tilt_deg,
        erosion: meta.erosion,
        pressure_bar: meta.pressure_bar,
        // Worlds made before the surface age have no craters: a fresh surface.
        surface_age_ga: meta.surface_age_ga.unwrap_or(0.0),
    };
    let l = &meta.layers;
    let infos = [
        Some(&l.elevation),
        Some(&l.moisture),
        Some(&l.temperature),
        Some(&l.flow),
        l.volcanic_zone.as_ref(),
        l.coast_distance.as_ref(),
        l.highland_offset.as_ref(),
        l.floodplain.as_ref(),
    ];
    let read_biomes = || -> Result<Vec<Biome>, String> {
        read_biome_ids(&raw, &meta)?
            .into_raw()
            .into_iter()
            .map(|id| {
                Biome::ALL
                    .get(id as usize)
                    .copied()
                    .ok_or_else(|| format!("unknown biome id {id}"))
            })
            .collect()
    };
    // Version 1 worlds have no river polylines.
    let read_rivers = || -> Result<Vec<RiverLine>, String> {
        let Some(file) = &meta.rivers else {
            return Ok(Vec::new());
        };
        let text = std::fs::read_to_string(raw.join(file)).map_err(|e| format!("{file}: {e}"))?;
        let rivers: RiversFile = serde_json::from_str(&text).map_err(|e| format!("{file}: {e}"))?;
        Ok(rivers.to_lines(meta.width, meta.height))
    };
    // PNG decoding is single-threaded: decode the files side by side.
    let (layers, (biomes, rivers)) = rayon::join(
        || {
            infos
                .par_iter()
                .map(|info| info.map(|info| read_layer(&raw, info, &meta)).transpose())
                .collect::<Result<Vec<_>, String>>()
        },
        || rayon::join(read_biomes, read_rivers),
    );
    let [
        elevation,
        moisture,
        temperature,
        flow_km2,
        volcanic_zone,
        coast_distance_km,
        highland_offset,
        floodplain,
    ]: [Option<Layer16>; 8] = layers?.try_into().expect("eight layers");
    let (elevation, moisture, temperature, flow_km2) = (
        elevation.expect("required layer"),
        moisture.expect("required layer"),
        temperature.expect("required layer"),
        flow_km2.expect("required layer"),
    );
    let n = elevation.len();

    // Version 1 worlds: resample the noise-derived layers from the seed, in
    // one parallel pass for both.
    let (volcanic_zone, highland_offset) = match (volcanic_zone, highland_offset) {
        (Some(v), Some(h)) => (v, h),
        (v, h) => {
            let sampler = Sampler::new(&params);
            let shift = params.planet.climate.volcanic;
            let width = params.width as usize;
            let (volcanic, highland): (Vec<f32>, Vec<f32>) = (0..n)
                .into_par_iter()
                .map(|i| {
                    let s = sampler.sample((i % width) as u32, (i / width) as u32);
                    (
                        (s.volcanic_zone + shift).clamp(0.0, 1.0),
                        s.highland_offset(),
                    )
                })
                .unzip();
            (
                v.unwrap_or_else(|| Layer16::quantise(&volcanic, VOLCANIC_ZONE)),
                h.unwrap_or_else(|| Layer16::quantise(&highland, HIGHLAND_OFFSET)),
            )
        }
    };
    let coast_distance_km = coast_distance_km.unwrap_or_else(|| {
        let waterline = params.sea_level + params.planet.rules.waterline;
        let water: Vec<bool> = (0..n).map(|i| elevation.get(i) < waterline).collect();
        let distance = params.grid().distance_to(&water, COAST_DISTANCE_CAP_KM);
        Layer16::quantise(&distance, COAST_DISTANCE)
    });
    let floodplain = floodplain.unwrap_or_else(|| Layer16::zeros(n));
    // Before version 4, temperatures were on a `[0, 1]` scale: convert the
    // layer and the thresholds of the stored preset with the same curve,
    // which keeps every biome.
    let temperature = if meta.format_version < CELSIUS_VERSION {
        let values: Vec<f32> = (0..n)
            .into_par_iter()
            .map(|i| celsius(temperature.get(i)))
            .collect();
        Layer16::quantise(&values, TEMPERATURE)
    } else {
        temperature
    };
    let mut params = params;
    if meta.format_version < CELSIUS_VERSION {
        params.planet.to_celsius();
    }

    Ok(LoadedWorld {
        width: meta.width,
        height: meta.height,
        params,
        elevation,
        moisture,
        temperature,
        flow_km2,
        volcanic_zone,
        coast_distance_km,
        highland_offset,
        floodplain,
        biomes: biomes?,
        rivers: rivers?,
    })
}

/// Reloads a generated world from its directory (the one holding `raw_data/`).
///
/// Values come back quantised to 16 bits; see [`load_layers`] for older
/// formats.
pub fn load_world(world_dir: &Path) -> Result<World, String> {
    load_layers(world_dir).map(LoadedWorld::into_world)
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
            pressure_bar: 1.0,
            surface_age_ga: 0.4,
        })
        .world;
        let world_dir =
            std::env::temp_dir().join(format!("map-generator-test-{}", std::process::id()));
        let dir = world_dir.join("raw_data");
        std::fs::create_dir_all(&dir).unwrap();
        export_raw_data(&world, &dir, None).unwrap();

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
        // The stored layers are exactly the quantised world, and decode to
        // the same values as the full reload.
        let layers = load_layers(&world_dir).unwrap();
        let quantised = LoadedWorld::from_world(&world);
        let decoded = load_world(&world_dir).unwrap();
        for i in 0..world.elevation.len() {
            for (a, b) in [
                (&layers.elevation, &quantised.elevation),
                (&layers.flow_km2, &quantised.flow_km2),
                (&layers.coast_distance_km, &quantised.coast_distance_km),
                (&layers.floodplain, &quantised.floodplain),
            ] {
                assert_eq!(a.get(i), b.get(i));
            }
            assert_eq!(layers.moisture.get(i), decoded.moisture[i]);
            assert!(
                (decoded.flow_km2[i] - world.flow_km2[i]).abs() <= world.flow_km2[i] * 1e-3 + 1e-2
            );
        }
        assert!(layers.bytes() < world.elevation.len() * 18);

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
        assert_eq!(back.params.planet.remap(Biome::Ocean), Biome::AcidSea);
        assert_eq!(back.rivers.len(), world.rivers.len());
        assert!(
            back.floodplain
                .iter()
                .zip(&world.floodplain)
                .all(|(a, b)| (a - b).abs() < 1e-4)
        );

        // A world from a newer generator, or whose layers do not have the
        // announced size, is refused with a clear error.
        let write_meta = |meta: &WorldMetadata| {
            std::fs::write(dir.join("world.json"), serde_json::to_string(meta).unwrap()).unwrap();
        };
        let mut newer: WorldMetadata =
            serde_json::from_str(&serde_json::to_string(&meta).unwrap()).unwrap();
        newer.format_version = FORMAT_VERSION + 1;
        write_meta(&newer);
        let error = load_world(&world_dir).err().unwrap();
        assert!(error.contains("newer"), "{error}");
        newer.format_version = FORMAT_VERSION;
        newer.width += 1;
        write_meta(&newer);
        let error = load_world(&world_dir).err().unwrap();
        assert!(error.contains("announces"), "{error}");

        // A version 1 world (without the new layers) gets them recomputed.
        let mut v1 = meta;
        v1.format_version = 1;
        v1.layers.volcanic_zone = None;
        v1.layers.coast_distance = None;
        v1.layers.highland_offset = None;
        v1.layers.floodplain = None;
        v1.rivers = None;
        std::fs::write(dir.join("world.json"), serde_json::to_string(&v1).unwrap()).unwrap();
        let old = load_world(&world_dir).unwrap();
        assert!(old.rivers.is_empty());
        assert!(old.floodplain.iter().all(|&f| f == 0.0));
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
