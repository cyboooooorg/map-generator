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
//! `format_version` is newer than [`FORMAT_VERSION`] or older than
//! [`MIN_FORMAT_VERSION`], or whose layers do not have the size
//! `world.json` announces. The generator keeps no code to re-derive a world
//! of an older format: delete it (`mise run clean-systems`) and draw it again.
use crate::biome::{biome_color, surface_color};
use crate::climate::TEMPERATURE_RANGE;
use crate::generation::COAST_DISTANCE_CAP_KM;
use crate::preset::Preset;
use crate::rivers::{RiverLine, RiversFile};
use crate::star::StarInfo;
use crate::world::WorldParams;
use crate::world::{Biome, World, biome_name};
use image::{GrayImage, ImageBuffer, Luma};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::Path;

/// Name of the river polylines file.
pub const RIVERS_FILE: &str = "rivers.json";

/// Bumped whenever the layout of `raw_data/` changes incompatibly, or the
/// generator changes what a stored world means (its biomes, seasons or
/// temperatures): a world is read as the current format or not at all, see
/// [`MIN_FORMAT_VERSION`]. Version 14 dropped the code that re-derived
/// older worlds. Version 15 samples the smooth climate noise fields on a
/// coarser grid and interpolates them (see [`crate::coarse`]): the same
/// seed gives slightly different worlds from then on; version 14 worlds
/// are still read as they are.
pub const FORMAT_VERSION: u32 = 15;

/// The oldest format [`load_layers`] reads. Older worlds were made with
/// algorithms the generator no longer carries (temperatures on a `[0, 1]`
/// scale, biomes without seasons, a linear moisture, layers computed on
/// load): they are refused with a message suggesting `mise run
/// clean-systems`.
pub const MIN_FORMAT_VERSION: u32 = 14;

/// Range of the shape layer: the radius as a share of the mean one.
const SHAPE: (f32, f32) = (0.0, 2.0);

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
    /// Surface pressure in bar.
    pub pressure_bar: f32,
    /// Age of the oldest surviving surface, in Ga.
    pub surface_age_ga: f32,
    /// The host star and the orbit.
    pub star: StarInfo,
    /// Epoch of the plates, Ma from the present (negative in the past).
    /// Left out at the present.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub drift_ma: f32,
    /// The crater rate its planetary system set (see
    /// `WorldParams::impact_factor`); left out for every other world.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub impact_factor: Option<f32>,
    /// The moons and what bounds them; left out when the request turned
    /// them off.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moons: Option<crate::moons::MoonSystem>,
    /// The giant planets of the system and what they do to the world; left
    /// out when the request turned them off.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub giants: Option<crate::giants::GiantSystem>,
    /// What a giant world is (its mass, radius, spin, winds, colours and
    /// rings); left out for a rocky world.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub giant: Option<crate::giant_world::GiantInfo>,
    /// The giant the world orbits, as a moon of it (its world in `host/`);
    /// left out for a world orbiting its star.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<crate::exomoon::HostGiant>,
    /// The semi-axes of an irregular body, as shares of its mean radius
    /// (see `WorldParams::shape`); left out for a round one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<[f32; 3]>,
    /// The height of an equatorial ridge of accreted ring material on an
    /// irregular ring moon, as a share of its radius (see
    /// `WorldParams::ridge`); left out for a body without one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ridge: Option<f32>,
    pub layers: Layers,
    /// River polylines file (`rivers.json`, a `RiversFile`).
    pub rivers: String,
    /// Every biome, in id order.
    pub biomes: Vec<BiomeInfo>,
}

/// The layer files of `raw_data/`.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Layers {
    pub elevation: ScalarLayerInfo,
    /// The elevation without the impact craters younger than the wet era,
    /// same encoding as `elevation`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elevation_base: Option<ScalarLayerInfo>,
    pub moisture: ScalarLayerInfo,
    pub temperature: ScalarLayerInfo,
    /// `log10(1 + km²)` of the drainage area.
    pub flow: ScalarLayerInfo,
    /// Volcanic zone as biome selection sees it.
    pub volcanic_zone: ScalarLayerInfo,
    /// Distance to open water in km, capped.
    pub coast_distance: ScalarLayerInfo,
    /// Local shift of the highland line.
    pub highland_offset: ScalarLayerInfo,
    /// Floodplain weight of the river valleys.
    pub floodplain: ScalarLayerInfo,
    /// Brightness shift of the ground in `[-1, 1]`, 8 bits (`gray8`, as
    /// `clouds`): fresh impact craters, their ejecta and rays.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub albedo: Option<ScalarLayerInfo>,
    /// Radius of an irregular body as a share of its mean radius, 16 bits
    /// in `[0, 2]`: the globe's shape, which the elevation details (irregular
    /// moons only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<ScalarLayerInfo>,
    /// Cloud cover in `[0, 1]`, 8 bits (`gray8`: `min + pixel / 255 ×
    /// (max − min)`); absent on worlds without clouds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clouds: Option<ScalarLayerInfo>,
    /// The hillshaded render of `world.png` without its reference and
    /// contour lines, which the viewer draws itself.
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
    /// [`surface_color`]).
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
pub(crate) const ELEVATION: (f32, f32) = (-1.0, 1.0);
const MOISTURE: (f32, f32) = (-1.0, 1.0);
pub(crate) const TEMPERATURE: (f32, f32) = TEMPERATURE_RANGE;
/// `log10(1 + km²)`: drainage areas span 0 to millions of km².
const FLOW_LOG10: (f32, f32) = (0.0, 8.0);
const VOLCANIC_ZONE: (f32, f32) = (0.0, 1.0);
const COAST_DISTANCE: (f32, f32) = (0.0, COAST_DISTANCE_CAP_KM);
const HIGHLAND_OFFSET: (f32, f32) = (-1.0, 1.0);
const FLOODPLAIN: (f32, f32) = (0.0, 1.0);

/// Maps `v` from `[min, max]` to the full `u16` range.
/// One file of `raw_data/` to write.
type Job<'a> = Box<dyn FnOnce() -> Result<(), String> + Send + 'a>;

/// Files written at once by [`write_jobs`]: enough to hide most of their
/// encoding behind each other, few enough that their buffers (up to 64 MiB
/// each at 8192 × 4096) stay a small share of the export's memory.
const PARALLEL_FILES: usize = 4;

/// Runs `jobs` on [`PARALLEL_FILES`] threads, each taking the next job of
/// the queue, and returns the first error.
fn write_jobs(jobs: Vec<Job>) -> Result<(), String> {
    let queue = std::sync::Mutex::new(jobs.into_iter());
    (0..PARALLEL_FILES).into_par_iter().try_for_each(|_| {
        loop {
            let Some(job) = queue.lock().expect("job queue").next() else {
                return Ok(());
            };
            job()?;
        }
    })
}

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

    // Every PNG below is an independent single-threaded encode of the
    // whole map (about 0.1 s each at 8192 × 4096): they are queued as jobs
    // and written by a few threads at once (see `write_jobs`). The layers
    // are quantised inside their job, so only those being written hold a
    // buffer.
    let mut jobs: Vec<Job> = Vec::new();
    let save16 = move |file: String, pixels: Vec<u16>| -> Result<(), String> {
        ImageBuffer::<Luma<u16>, _>::from_raw(w, h, pixels)
            .ok_or("layer size does not match the world size")?
            .save(dir.join(file))
            .map_err(|e| e.to_string())
    };
    let save8 = move |file: &str, pixels: Vec<u8>| -> Result<(), String> {
        GrayImage::from_raw(w, h, pixels)
            .ok_or("layer size does not match the world size")?
            .save(dir.join(file))
            .map_err(|e| e.to_string())
    };

    let mut infos = Vec::with_capacity(scalars.len());
    for layer in scalars {
        let (min, max) = layer.range;
        let file = format!("{}.png", layer.name);
        infos.push(ScalarLayerInfo {
            file: file.clone(),
            encoding: layer.encoding.to_string(),
            min,
            max,
        });
        jobs.push(Box::new(move || {
            // Quantised on the fly: no full-size copy of the transformed values.
            let pixels: Vec<u16> = layer
                .values
                .iter()
                .map(|&v| quantise(if layer.log { (v + 1.0).log10() } else { v }, min, max))
                .collect();
            save16(file, pixels)
        }));
    }

    let elevation_base = match &world.elevation_base {
        Some(values) => {
            let (min, max) = ELEVATION;
            jobs.push(Box::new(move || {
                let pixels: Vec<u16> = values.iter().map(|&v| quantise(v, min, max)).collect();
                save16("elevation_base.png".into(), pixels)
            }));
            Some(ScalarLayerInfo {
                file: "elevation_base.png".into(),
                encoding: "gray16".into(),
                min,
                max,
            })
        }
        None => None,
    };

    let shape = match crate::moons::shape_layer(&world.params) {
        Some(values) => {
            let (min, max) = SHAPE;
            jobs.push(Box::new(move || {
                let pixels: Vec<u16> = values.iter().map(|&v| quantise(v, min, max)).collect();
                save16("shape.png".into(), pixels)
            }));
            Some(ScalarLayerInfo {
                file: "shape.png".into(),
                encoding: "gray16".into(),
                min,
                max,
            })
        }
        None => None,
    };

    let albedo = if world.albedo.is_empty() {
        None
    } else {
        jobs.push(Box::new(move || {
            save8("albedo.png", albedo_bytes(&world.albedo))
        }));
        Some(ScalarLayerInfo {
            file: "albedo.png".into(),
            encoding: "gray8".into(),
            min: ALBEDO.0,
            max: ALBEDO.1,
        })
    };

    jobs.push(Box::new(move || {
        save8("biomes.png", world.biomes.iter().map(|b| b.id()).collect())
    }));

    let clouds = if world.clouds.is_empty() {
        None
    } else {
        jobs.push(Box::new(move || {
            let cover: Vec<u8> = world
                .clouds
                .iter()
                .map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8)
                .collect();
            save8("clouds.png", cover)
        }));
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
            // Saved from the borrowed pixels: no copy of the render. The
            // largest file: first in the queue.
            jobs.insert(
                0,
                Box::new(move || {
                    image::save_buffer(
                        dir.join("surface.png"),
                        pixels.as_flattened(),
                        w,
                        h,
                        image::ExtendedColorType::Rgb8,
                    )
                    .map_err(|e| e.to_string())
                }),
            );
            Some(ImageLayerInfo {
                file: "surface.png".into(),
                encoding: "rgb8".into(),
            })
        }
        None => None,
    };
    write_jobs(jobs)?;

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
        surface_age_ga: world.params.surface_age_ga,
        star: StarInfo {
            metallicity: world.params.metallicity,
            ..crate::spin::star_info(&world.params)
        },
        drift_ma: world.params.drift_ma,
        impact_factor: world.params.impact_factor,
        moons: crate::moons::MoonSystem::draw(&world.params),
        giants: crate::giants::GiantSystem::draw(&world.params),
        giant: crate::giant_world::info(&world.params),
        host: crate::exomoon::HostGiant::draw(&world.params),
        shape: world.params.shape,
        ridge: world.params.ridge,
        layers: Layers {
            elevation,
            elevation_base,
            moisture,
            temperature,
            flow,
            volcanic_zone,
            coast_distance,
            highland_offset,
            floodplain,
            clouds,
            albedo,
            shape,
            surface,
            biomes: BiomeLayerInfo {
                file: "biomes.png".into(),
                encoding: "gray8-biome-id".into(),
            },
        },
        rivers: RIVERS_FILE.into(),
        biomes: biome_table_for(&world.params),
    };
    let rivers = RiversFile::from_lines(&world.rivers, w, h);
    std::fs::write(dir.join(RIVERS_FILE), serde_json::to_string(&rivers)?)?;
    std::fs::write(
        dir.join("world.json"),
        serde_json::to_string_pretty(&metadata)?,
    )?;
    Ok(())
}

/// The biome table of the world `p`: [`biome_table`], with a giant's own
/// colours for its band biomes.
pub fn biome_table_for(p: &WorldParams) -> Vec<BiomeInfo> {
    let mut table = biome_table();
    if let Some(colours) = crate::giant_world::biome_colours(p) {
        let hex = |[r, g, b]: [u8; 3]| format!("#{r:02X}{g:02X}{b:02X}");
        for (biome, colour) in colours {
            let entry = &mut table[biome.id() as usize];
            entry.color = hex(colour);
            entry.surface_color = hex(colour);
        }
    }
    table
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

/// The message of a refused world when the `world.json` text says a format
/// older than [`MIN_FORMAT_VERSION`], `None` otherwise (or when the text
/// has no version). The server answers it with 410 Gone.
pub fn old_format_error(world_json: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct Head {
        format_version: u32,
    }
    let head: Head = serde_json::from_str(world_json).ok()?;
    (head.format_version < MIN_FORMAT_VERSION).then(|| {
        format!(
            "world.json: format version {} is older than the oldest supported version \
             {MIN_FORMAT_VERSION}: the generator no longer re-derives older worlds, delete it \
             (`mise run clean-systems`) and draw it again",
            head.format_version
        )
    })
}

/// Reads and checks `world.json` of a `raw_data/` directory. Worlds written
/// by a newer version of the generator are refused (their layers may not
/// mean what this version expects), and so are worlds older than
/// [`MIN_FORMAT_VERSION`].
pub(crate) fn read_metadata(raw_dir: &Path) -> Result<WorldMetadata, String> {
    let path = raw_dir.join("world.json");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    // The version first: an older world may not even parse as today's.
    if let Some(error) = old_format_error(&text) {
        return Err(error);
    }
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

/// Range of the albedo layer.
const ALBEDO: (f32, f32) = (-1.0, 1.0);

/// The albedo shift quantised to 8 bits, as `albedo.png` stores it.
pub fn albedo_bytes(values: &[f32]) -> Vec<u8> {
    values
        .iter()
        .map(|&v| ((v.clamp(-1.0, 1.0) + 1.0) / 2.0 * 255.0).round() as u8)
        .collect()
}

/// An albedo byte of `albedo.png`, decoded.
#[inline]
pub fn albedo_of(byte: u8) -> f32 {
    f32::from(byte) / 255.0 * 2.0 - 1.0
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

    /// The stored 16-bit values, as in the layer's PNG.
    pub fn raw(&self) -> &[u16] {
        &self.raw
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
#[derive(Clone)]
pub struct LoadedWorld {
    pub params: WorldParams,
    pub width: u32,
    pub height: u32,
    pub elevation: Layer16,
    pub moisture: Layer16,
    pub temperature: Layer16,
    /// The seasonal cycle, from the parameters.
    pub seasons: crate::climate::Seasons,
    /// Cooling of the land per elevation unit the temperatures were made
    /// with, from the parameters.
    pub lapse_rate: f32,
    /// Drainage area in km² (stored on a log scale).
    pub flow_km2: Layer16,
    pub volcanic_zone: Layer16,
    pub coast_distance_km: Layer16,
    pub highland_offset: Layer16,
    pub floodplain: Layer16,
    pub biomes: Vec<Biome>,
    /// Albedo shift as stored (see [`albedo_of`]).
    pub albedo: Vec<u8>,
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
            seasons: world.seasons.clone(),
            lapse_rate: world.lapse_rate,
            flow_km2,
            volcanic_zone: Layer16::quantise(&world.volcanic_zone, VOLCANIC_ZONE),
            coast_distance_km: Layer16::quantise(&world.coast_distance_km, COAST_DISTANCE),
            highland_offset: Layer16::quantise(&world.highland_offset, HIGHLAND_OFFSET),
            floodplain: Layer16::quantise(&world.floodplain, FLOODPLAIN),
            biomes: world.biomes.clone(),
            albedo: albedo_bytes(&world.albedo),
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
            + self.albedo.len()
    }

    /// Decodes every layer into a [`World`], keeping this one. The stored
    /// layers are read in place, not copied first: only the decoded world
    /// (and the biomes and rivers it owns) is added to this one's memory.
    pub fn to_world(&self) -> World {
        let biomes = self.biomes.clone();
        let rivers = self.rivers.clone();
        self.decoded(biomes, rivers)
    }

    /// Decodes every layer into a [`World`].
    pub fn into_world(mut self) -> World {
        let biomes = std::mem::take(&mut self.biomes);
        let rivers = std::mem::take(&mut self.rivers);
        self.decoded(biomes, rivers)
    }

    /// The [`World`] of these layers, decoded in parallel, with `biomes`
    /// and `rivers`.
    fn decoded(&self, biomes: Vec<Biome>, rivers: Vec<RiverLine>) -> World {
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
            params: self.params.clone(),
            width: self.width,
            height: self.height,
            elevation,
            moisture,
            temperature,
            seasons: self.seasons.clone(),
            lapse_rate: self.lapse_rate,
            flow_km2,
            volcanic_zone,
            coast_distance_km,
            floodplain,
            highland_offset,
            elevation_base: None,
            albedo: self.albedo.iter().map(|&b| albedo_of(b)).collect(),
            biomes,
            rivers,
            clouds: Vec::new(),
            surface: Vec::new(),
        }
    }
}

/// The elevation without the young impact craters of a world (see
/// [`World::elevation_base`]), decoded from its `raw_data/`: `None` for a
/// world without such a layer.
pub fn read_elevation_base(world_dir: &Path) -> Result<Option<Vec<f32>>, String> {
    let raw = world_dir.join("raw_data");
    let meta = read_metadata(&raw)?;
    match &meta.layers.elevation_base {
        Some(layer) => Ok(Some(read_layer(&raw, layer, &meta)?.decode())),
        None => Ok(None),
    }
}

fn is_zero(v: &f32) -> bool {
    *v == 0.0
}

/// Reloads a generated world from its directory (the one holding `raw_data/`),
/// keeping its layers as stored (see [`LoadedWorld`]). The layer files are
/// decoded in parallel.
///
/// Worlds older than [`MIN_FORMAT_VERSION`] are refused (see
/// [`read_metadata`]).
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
        surface_age_ga: meta.surface_age_ga,
        star: meta.star.code(),
        orbit_au: meta.star.orbit_au,
        eccentricity: meta.star.eccentricity,
        drift_ma: meta.drift_ma,
        moons: match &meta.moons {
            Some(system) => system.count_asked.map_or(
                crate::moons::MoonsMode::Auto,
                crate::moons::MoonsMode::Count,
            ),
            None => crate::moons::MoonsMode::None,
        },
        shape: meta.shape,
        metallicity: meta.star.metallicity,
        giants: if meta.giants.is_some() {
            crate::giants::GiantsMode::Auto
        } else {
            crate::giants::GiantsMode::None
        },
        giant_mass_earth: meta.giant.as_ref().map(|g| g.mass_earth),
        host: if meta.host.is_some() {
            crate::exomoon::HostMode::Giant
        } else {
            crate::exomoon::HostMode::None
        },
        // A giant keeps the rings stored with it, whether drawn from its
        // seed or by its system.
        given_rings: meta.giant.as_ref().map(|g| crate::giant_world::GivenRings {
            rings: g.rings.clone(),
            dust: Vec::new(),
        }),
        impact_factor: meta.impact_factor,
        // The lock and resonance the base map was made with, which a system
        // can set and the draw of its seed does not tell.
        locked_to_star: Some(meta.star.tidally_locked),
        ridge: meta.ridge,
        perihelion_longitude_deg: meta.star.perihelion_longitude_deg,
        resonance_to_star: Some(meta.star.spin_turns),
        // The gravity it was generated with, so its epochs and detail tiles
        // draw the craters it was drawn with.
        gravity: Some(meta.gravity_modifier),
    };
    let l = &meta.layers;
    let infos = [
        &l.elevation,
        &l.moisture,
        &l.temperature,
        &l.flow,
        &l.volcanic_zone,
        &l.coast_distance,
        &l.highland_offset,
        &l.floodplain,
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
    let read_rivers = || -> Result<Vec<RiverLine>, String> {
        let file = &meta.rivers;
        let text = std::fs::read_to_string(raw.join(file)).map_err(|e| format!("{file}: {e}"))?;
        let rivers: RiversFile = serde_json::from_str(&text).map_err(|e| format!("{file}: {e}"))?;
        Ok(rivers.to_lines(meta.width, meta.height))
    };
    // PNG decoding is single-threaded: decode the files side by side.
    let (layers, (biomes, rivers)) = rayon::join(
        || {
            infos
                .par_iter()
                .map(|info| read_layer(&raw, info, &meta))
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
    ]: [Layer16; 8] = layers?.try_into().expect("eight layers");
    let seasons = crate::generation::seasons(&params);
    let lapse_rate = crate::climate::lapse_rate(&params);
    let albedo = match &meta.layers.albedo {
        Some(layer) => {
            let img = image::open(raw.join(&layer.file))
                .map_err(|e| format!("{}: {e}", layer.file))?
                .into_luma8();
            check_size(&layer.file, img.dimensions(), &meta)?;
            img.into_raw()
        }
        None => Vec::new(),
    };

    Ok(LoadedWorld {
        width: meta.width,
        height: meta.height,
        params,
        elevation,
        moisture,
        temperature,
        seasons,
        lapse_rate,
        flow_km2,
        volcanic_zone,
        coast_distance_km,
        highland_offset,
        floodplain,
        biomes: biomes?,
        albedo,
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
    fn worlds_before_the_seasons_keep_their_climate_rules() {
        let params = WorldParams {
            width: 32,
            height: 16,
            ..crate::generation::tests::params(4, "terran")
        };
        let world = generate(&params).world;
        let world_dir =
            std::env::temp_dir().join(format!("map-generator-seasons-test-{}", std::process::id()));
        let dir = world_dir.join("raw_data");
        std::fs::create_dir_all(&dir).unwrap();
        export_raw_data(&world, &dir, None).unwrap();

        // Today's worlds: their seasons and lapse rate from the parameters,
        // and the biomes they were made with.
        let back = load_world(&world_dir).unwrap();
        assert_eq!(back.seasons, crate::generation::seasons(&back.params));
        assert!(back.seasons.half_range(4, true) > 5.0);
        assert_eq!(back.lapse_rate, crate::climate::lapse_rate(&back.params));
        // The seasons the biomes were chosen with are those a world reads
        // (on the generated world: its stored temperatures are quantised,
        // which can move a pixel right at a threshold).
        let rules = &world.params.planet.rules;
        for i in 0..world.biomes.len() {
            let stored = world.biomes[i];
            // The lake flag is not stored: lakes, and frozen lakes stored
            // as ice cap, are left out.
            if !matches!(stored, Biome::Lake | Biome::River | Biome::IceCap) {
                assert_eq!(
                    world
                        .params
                        .planet
                        .remap(rules.choose(world.biome_input(i))),
                    stored
                );
            }
        }

        std::fs::remove_dir_all(&world_dir).unwrap();
    }

    #[test]
    fn a_world_with_a_long_solar_day_reads_the_swing_of_its_day() {
        // Mercury in its 3:2 resonance: a solar day of two years.
        let request = crate::request::WorldRequest {
            planet: Some(
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../presets/examples/mercury-like.toml"
                )
                .into(),
            ),
            seed: Some(3),
            width: Some(32),
            height: Some(16),
            ..Default::default()
        };
        let params = request.resolve(&mut rand::rng()).unwrap().params;
        let world = generate(&params).world;
        let world_dir =
            std::env::temp_dir().join(format!("map-generator-day-test-{}", std::process::id()));
        let dir = world_dir.join("raw_data");
        std::fs::create_dir_all(&dir).unwrap();
        export_raw_data(&world, &dir, None).unwrap();

        let back = load_world(&world_dir).unwrap();
        assert_eq!(back.seasons, crate::generation::seasons(&back.params));
        assert!(back.seasons.half_range(8, true) > 20.0);
        std::fs::remove_dir_all(&world_dir).unwrap();
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
            star: 64.5,
            orbit_au: 0.1,
            eccentricity: 0.0,
            drift_ma: 0.0,
            moons: crate::moons::MoonsMode::Auto,
            shape: None,
            metallicity: 0.0,
            giants: crate::giants::GiantsMode::None,
            giant_mass_earth: None,
            host: crate::exomoon::HostMode::None,
            given_rings: None,
            impact_factor: None,
            locked_to_star: None,
            ridge: None,
            perihelion_longitude_deg: None,
            resonance_to_star: None,
            gravity: None,
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
        // The star round-trips: a red dwarf at 0.1 AU.
        let star = &meta.star;
        assert_eq!(star.kind, "M4.5V");
        assert_eq!((star.code(), star.orbit_au), (64.5, 0.1));

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
        assert!(layers.bytes() < world.elevation.len() * 19);

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

        // A world older than the supported formats is refused, with the way out.
        newer.format_version = MIN_FORMAT_VERSION - 1;
        newer.width -= 1;
        write_meta(&newer);
        let error = load_world(&world_dir).err().unwrap();
        assert!(
            error.contains("older") && error.contains("clean-systems"),
            "{error}"
        );
        std::fs::remove_dir_all(world_dir).unwrap();
    }
}
