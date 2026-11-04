use crate::atmosphere::{AIRLESS_BAR, CloudKind, LIQUID_WATER_BAR, transport_heat};
use crate::biome::BiomeInput;
use crate::carve;
use crate::climate::LockedClimate;
use crate::climate::{
    Field, MoistureInput, TEMPERATURE_RANGE, moisture, regional_variability, row_temperatures,
    temperature, temperature_anomaly,
};
use crate::clouds::CloudInput;
use crate::craters::{CraterField, Era, RAY_MIN_PIXELS, base_min_diameter_km, darken_maria};
use crate::delta;
use crate::erosion::erode;
use crate::geo::Grid;
use crate::geology::Regime;
use crate::hydrology::hydrology;
use crate::noise::{
    EARTH_CIRCUMFERENCE_KM, cell_value, fbm, nearest_point, ridged, smoothstep, stream, stream_seed,
};
use crate::rivers;
use crate::star::{StarClimate, StarInfo, star_temperatures};
use crate::world::*;
use noise::{NoiseFn, Perlin};
use rayon::prelude::*;
use serde::Serialize;
use std::f64::consts::PI;

/// Every intermediate noise value computed for one pixel.
#[derive(Clone, Copy)]
pub struct Sample {
    pub warp_x: f32,
    pub warp_y: f32,
    pub continent: f32,
    pub mountain: f32,
    pub mountain_weight: f32,
    pub elevation: f32,
    pub biome_elevation: f32,
    pub moisture: f32,
    pub volcanic_raw: f32,
    pub volcanic_zone: f32,
    /// Large-scale temperature anomaly noise, roughly in `[-1, 1]`.
    pub temperature_anomaly: f32,
    /// Regional climate variability noise for temperature and moisture.
    pub variability_t: f32,
    pub variability_m: f32,
    /// Highland line jitter noise, roughly in `[-1, 1]`.
    pub highland_jitter: f32,
    /// Height added by plate boundaries (ranges, trenches, arcs, ridges,
    /// rifts), elevation units; 0 on worlds without plate tectonics.
    pub tectonic_uplift: f32,
}

impl Sample {
    /// Local shift of the highland line used by biome selection.
    pub fn highland_offset(&self) -> f32 {
        HIGHLAND_JITTER * self.highland_jitter
    }
}

/// Weight of the highland line jitter (about 0.09 of standard deviation).
const HIGHLAND_JITTER: f32 = 0.50;

/// Lateral displacement of the continent outline, per octave of the coast
/// warp, in units of the octave's wavelength (on the unit sphere).
const COAST_WARP: f64 = 0.5;
/// Ratio of the displacement share from one coast-warp octave to the next:
/// below 1, so the finest octaves fray the outline less than the coarse ones
/// shape it.
const COAST_WARP_DECAY: f64 = 0.8;
/// Height of the "plate" step added to the continents: a piecewise-constant
/// cell noise that gives coasts straight stretches and angular corners.
const PLATE_STEP: f32 = 0.06;
/// Inverse width of the plate borders, in plate cells: about 1/20 of a cell
/// (100–150 km on an Earth-sized planet), a ramp rather than a cliff.
const PLATE_SHARPNESS: f64 = 20.0;
/// Share of the fine continent octaves kept on low ground inland, so plains
/// and deserts are smooth; coasts and mountains keep them all.
const PLAIN_ROUGHNESS: f32 = 0.3;
/// Height above the waterline over which inland ground turns from coastal to
/// plain roughness.
const COAST_BAND: (f32, f32) = (0.03, 0.2);
/// Height above (or depth below) the waterline over which the outline detail
/// fades out: wide enough for every point the displacement can bring to the
/// coast, narrow enough not to wrinkle the lowlands.
const OUTLINE_BAND: (f32, f32) = (0.02, 0.08);
/// Weight of the second and third ridged octaves of the mountains.
const MOUNTAIN_DETAIL: f32 = 0.35;

/// Octaves of the coast warp: from the finest continent octave down to about
/// two pixels of the map, so the outline is fractal at the map's resolution
/// without aliasing below it.
fn coast_octaves(noise_scale: f64, grid: &Grid) -> u32 {
    let radius_km = grid.circumference_km as f64 / (2.0 * PI);
    let pixel_km = grid.circumference_km as f64 / grid.width as f64;
    // The finest continent octave has frequency 0.8 × s × 16.
    let mut wavelength = radius_km / (0.8 * noise_scale * 16.0);
    let mut octaves = 0;
    while wavelength > 2.0 * pixel_km && octaves < 10 {
        wavelength /= 2.0;
        octaves += 1;
    }
    octaves
}

/// Moisture added on a floodplain (weight 1): rivers green their valleys.
const RIPARIAN_MOISTURE: f32 = 0.15;

/// Volcanic zone away from cones, as a share of the broad zone: below the
/// ash threshold (0.15), so the zone is ordinary land dotted with volcanoes,
/// each with its vent, lava flows and an ash halo.
const VOLCANIC_BACKGROUND: f32 = 0.12;
/// Cone lattice cell, km: at most one volcano per cell.
const CONE_CELL_KM: f64 = 900.0;
/// Share of the cells with a volcano.
const CONE_PRESENCE: f64 = 0.5;
/// Cone radius range, km. In a fully volcanic zone the vent biome reaches
/// about 1.2 radii, lava flows 1.8 and ash 2.6.
const CONE_KM: (f64, f64) = (50.0, 120.0);
/// Height of a cone above the terrain at its vent, in a fully volcanic zone.
const CONE_HEIGHT: f32 = 0.25;

/// Depth below the waterline down to which the crust counts as
/// continental (the continental shelf), elevation units.
const CONTINENTAL_SHELF: f32 = 0.05;
/// Share of the noise's mountain weight kept on worlds with plate
/// tectonics: the rest of their ranges follow the plate boundaries.
const NOISE_MOUNTAINS_WITH_PLATES: f32 = 0.5;
/// Weight of volcanic arcs in the volcanic zone, on a world without
/// volcanism and on a fully volcanic one.
const ARC_VOLCANISM: (f32, f32) = (0.8, 1.0);
/// Volcanoes of an arc: one per cell of this size (km) at most, present in
/// this share of the cells (arcs on Earth have one every 50–100 km; at the
/// base map's resolution, about 20 km per pixel, they would merge).
const ARC_CONE_CELL_KM: f64 = 200.0;
const ARC_CONE_PRESENCE: f64 = 0.8;
/// Below this arc weight, no arc volcano.
const ARC_MIN: f32 = 0.02;

/// Noise sources and derived constants for one world.
pub struct Sampler {
    grid: Grid,
    /// The plates and their boundaries, on worlds with plate tectonics.
    tectonics: Option<crate::plates::PlateMap>,
    /// Share of the noise's mountain weight kept on such worlds: their
    /// ranges follow the plate boundaries.
    noise_mountains: f32,
    /// Weight of volcanic arcs in the volcanic zone.
    arc_volcanism: f32,
    sea_level: f32,
    /// Absolute elevation of the sea surface.
    waterline: f32,
    noise_scale: f64,
    coast_octaves: u32,
    coast_noise: Perlin,
    plate_seed: u32,
    cone_seed: u32,
    arc_cone_seed: u32,
    mountain_blend: f32,
    volcanic_threshold: f32,
    elevation_noise: Perlin,
    moisture_noise: Perlin,
    continent_noise: Perlin,
    warp_noise_a: Perlin,
    warp_noise_b: Perlin,
    volcano_noise: Perlin,
    /// Continental-scale temperature anomalies (currents, monsoons, continentality).
    anomaly_noise: Perlin,
    /// Regional climate variability (soils, aspect, local circulation).
    variability_noise: Perlin,
    highland_noise: Perlin,
}

impl Sampler {
    pub fn new(p: &WorldParams) -> Self {
        let seed = p.seed;
        // Scale noise frequencies by planet size: a larger circumference
        // stretches the unit-sphere coordinates, producing broader continents.
        // Earth (40 075 km) ≡ scale 1.0.
        let noise_scale = (EARTH_CIRCUMFERENCE_KM / p.circumference_km.max(1.0)) as f64;
        let grid = p.grid();
        let mut sampler = Self {
            tectonics: None,
            noise_mountains: 1.0,
            arc_volcanism: ARC_VOLCANISM.0
                + (ARC_VOLCANISM.1 - ARC_VOLCANISM.0) * p.volcanic_intensity.clamp(0.0, 1.0),
            coast_octaves: coast_octaves(noise_scale, &grid),
            grid,
            sea_level: p.sea_level,
            waterline: p.sea_level + p.planet.rules.waterline,
            noise_scale,
            coast_noise: Perlin::new(stream_seed(seed, stream::COAST)),
            plate_seed: stream_seed(seed, stream::PLATES),
            cone_seed: stream_seed(seed, stream::CONES),
            arc_cone_seed: stream_seed(seed, stream::ARC_CONES),
            // Stronger gravity suppresses mountain relief; sqrt dampens the effect
            // for extreme values. Baseline 0.35 at Earth gravity.
            mountain_blend: 0.35 / p.gravity_modifier().sqrt(),
            // Higher intensity lowers the threshold, making more terrain volcanic.
            volcanic_threshold: 1.0 - p.volcanic_intensity.clamp(0.0, 1.0),
            // wrapping_add: a random seed close to u32::MAX must not overflow.
            elevation_noise: Perlin::new(stream_seed(seed, stream::ELEVATION)),
            moisture_noise: Perlin::new(stream_seed(seed, stream::MOISTURE)),
            continent_noise: Perlin::new(stream_seed(seed, stream::CONTINENT)),
            warp_noise_a: Perlin::new(stream_seed(seed, stream::WARP_A)),
            warp_noise_b: Perlin::new(stream_seed(seed, stream::WARP_B)),
            // Low-frequency noise that selects which mountain chains turn volcanic.
            volcano_noise: Perlin::new(stream_seed(seed, stream::VOLCANO)),
            anomaly_noise: Perlin::new(stream_seed(seed, stream::ANOMALY)),
            variability_noise: Perlin::new(stream_seed(seed, stream::VARIABILITY)),
            highland_noise: Perlin::new(stream_seed(seed, stream::HIGHLAND)),
        };
        if p.planet.geology.regime == crate::geology::Regime::Plates {
            // The crust of each side of a boundary: continental above the
            // edge of the shelf, from the continents' shape alone.
            let shelf = sampler.waterline - CONTINENTAL_SHELF;
            let map = crate::plates::PlateMap::with_crust(p, |q| {
                let (coarse, fine) = sampler.continent_shape(q);
                coarse + fine > shelf
            });
            sampler.tectonics = Some(map);
            sampler.noise_mountains = NOISE_MOUNTAINS_WITH_PLATES;
        }
        sampler
    }

    /// The continents' shape at a unit vector: its coarse octaves and its
    /// fine ones (see [`Self::sample`]).
    fn continent_shape(&self, p: [f64; 3]) -> (f32, f32) {
        let c = 0.8 * self.noise_scale;
        let norm = 1.0 + 0.5 + 0.25 + 0.125 + 0.0625;
        let octave = |k: u32| {
            let f = c * f64::from(1u32 << k);
            self.continent_noise.get([p[0] * f, p[1] * f, p[2] * f]) as f32 / (1u32 << k) as f32
        };
        (
            (octave(0) + octave(1)) / norm,
            (octave(2) + octave(3) + octave(4)) / norm,
        )
    }

    /// Volcanic cone profile at a point of the unit sphere: 1 at a cone's
    /// vent, falling with distance over a few cone radii, with lobed lava
    /// flows. Cones are one per [`CONE_CELL_KM`] cell at most, sized in km,
    /// so their size does not depend on the planet's.
    fn cone(&self, p: [f64; 3]) -> f32 {
        self.cone_on(p, self.cone_seed, CONE_CELL_KM, CONE_PRESENCE)
    }

    /// Like [`Self::cone`], on a lattice of its own: one point per
    /// `cell_km` cell, present with probability `presence`.
    fn cone_on(&self, p: [f64; 3], seed: u32, cell_km: f64, presence: f64) -> f32 {
        let radius_km = self.grid.circumference_km as f64 / (2.0 * PI);
        let f = radius_km / cell_km;
        let Some((d, h)) = nearest_point(seed, p[0] * f, p[1] * f, p[2] * f, presence) else {
            return 0.0;
        };
        let size = CONE_KM.0 + (CONE_KM.1 - CONE_KM.0) * ((h >> 8) & 0xFF) as f64 / 255.0;
        // Flows reach further in some directions than others.
        let g = radius_km / (2.0 * size);
        let lobe = self
            .volcano_noise
            .get([p[0] * g + 71.0, p[1] * g - 13.0, p[2] * g + 29.0]);
        let r = d * cell_km / size * (1.0 + 0.4 * lobe);
        (-0.5 * r * r).exp() as f32
    }

    /// Samples every noise layer for the pixel at column `x`, row `y`.
    pub fn sample(&self, x: u32, y: u32) -> Sample {
        let s = self.noise_scale;

        // Spherical mapping of the pixel centre: longitude 0..2π, latitude
        // +π/2 (row 0, north) to -π/2 (last row, south). Projects the flat map
        // onto a unit sphere → no seams, no mirror symmetry.
        let lon = (x as f64 + 0.5) / self.grid.width as f64 * 2.0 * PI;
        let lat = self.grid.latitude(y);
        let nx = lat.cos() * lon.cos();
        let ny = lat.cos() * lon.sin();
        let nz = lat.sin();

        // Domain warping: twist coordinates before sampling for organic coastlines.
        let warp_x = self
            .warp_noise_a
            .get([nx * 2.0 * s, ny * 2.0 * s, nz * 2.0 * s]);
        let warp_y =
            self.warp_noise_b
                .get([nx * 2.0 * s + 5.2, ny * 2.0 * s + 1.3, nz * 2.0 * s + 3.7]);
        let wnx = nx + warp_x * 0.25;
        let wny = ny + warp_y * 0.25;

        // Continent shape: 5 octaves, split into the 2 coarse ones (continents,
        // basins) and the 3 fine ones (hills).
        let c = 0.8 * s;
        // Same normalisation as `fbm` with 5 octaves.
        let norm = 1.0 + 0.5 + 0.25 + 0.125 + 0.0625;
        let shape = |x: f64, y: f64, z: f64| {
            let octave = |k: u32| {
                let f = c * f64::from(1u32 << k);
                self.continent_noise.get([x * f, y * f, z * f]) as f32 / (1u32 << k) as f32
            };
            (
                (octave(0) + octave(1)) / norm,
                (octave(2) + octave(3) + octave(4)) / norm,
            )
        };
        let (coarse, fine) = shape(nx, ny, nz);
        // Continent outline: near the coast the noise is sampled at a
        // position displaced sideways by every octave finer than its own,
        // each by a share of its wavelength (thinning at finer octaves), and
        // a plate step is added, so coasts get capes, bays, islands, straight
        // stretches and corners at every scale down to the pixel. The
        // displacement fades out away from the coast, where it would only
        // wrinkle the relief.
        let coastal = 1.0
            - smoothstep(
                OUTLINE_BAND.0,
                OUTLINE_BAND.1,
                (coarse + fine - self.waterline).abs(),
            );
        let (coarse, fine) = if coastal > 0.0 {
            let (mut qx, mut qy, mut qz) = (nx, ny, nz);
            let mut f = 0.8 * s * 16.0;
            let mut share = COAST_WARP * coastal as f64;
            for k in 0..self.coast_octaves {
                let a = share / f;
                let o = 17.0 * k as f64;
                qx += a * self.coast_noise.get([nx * f + o, ny * f, nz * f]);
                qy += a * self.coast_noise.get([nx * f, ny * f + o, nz * f + 3.1]);
                qz += a * self.coast_noise.get([nx * f + 5.7, ny * f, nz * f + o]);
                f *= 2.0;
                share *= COAST_WARP_DECAY;
            }
            let (coarse, fine) = shape(qx, qy, qz);
            let pf = 2.5 * s;
            let plate = PLATE_STEP
                * cell_value(self.plate_seed, qx * pf, qy * pf, qz * pf, PLATE_SHARPNESS);
            (coarse + coastal * plate, fine)
        } else {
            (coarse, fine)
        };
        let mountain_weight = ((coarse + fine - 0.2) * 2.5).clamp(0.0, 1.0);
        // Low ground inland keeps a share of the fine octaves: plains and
        // deserts are smooth, coasts keep their outline and mountains their
        // foothills.
        // Away from the coast both ways: inland plains, and abyssal plains
        // on the sea floor (seen on planets whose seas have dried up).
        let inland = smoothstep(
            COAST_BAND.0,
            COAST_BAND.1,
            (coarse + fine - self.waterline).abs(),
        );
        let roughness = (1.0 - (1.0 - PLAIN_ROUGHNESS) * inland).max(mountain_weight);
        let continent = coarse + fine * roughness;

        // Ridged mountains blended only onto elevated terrain: a main ridge,
        // with two finer ridged octaves concentrated on it.
        let ridge = |f: f64, o: f64| {
            ridged(
                &self.elevation_noise,
                wnx * f + o,
                wny * f - o,
                nz * f + 0.5 * o,
            )
        };
        let main = ridge(5.0 * s, 0.0);
        let detail = 0.6 * ridge(10.7 * s, 23.0) + 0.4 * ridge(22.1 * s, 41.0);
        let mountain = main * (1.0 - MOUNTAIN_DETAIL + MOUNTAIN_DETAIL * detail);

        // Volcanic zones are broad; volcanoes are cones within them, which
        // rise above the terrain (islands at sea). Away from a cone the zone
        // stays below the ash threshold.
        let volcanic_raw = fbm(&self.volcano_noise, nx * s, ny * s, nz * s, 3);
        // Plate boundaries: their own relief, ranges where they close, and
        // volcanic arcs over subducting plates.
        let plates = self
            .tectonics
            .as_ref()
            .map_or(crate::plates::Relief::default(), |t| t.relief(lon, lat));
        // Relief scales with gravity as the mountains do.
        let tectonic_uplift = plates.uplift * self.mountain_blend / 0.35;
        let mountain_weight = (mountain_weight * self.noise_mountains).max(plates.orogeny);
        let broad = ((volcanic_raw - self.volcanic_threshold) * 4.0)
            .clamp(0.0, 1.0)
            .max(plates.arc * self.arc_volcanism);
        let cone = if broad > 0.0 {
            self.cone([nx, ny, nz])
        } else {
            0.0
        };
        // Volcanic arcs: a chain of volcanoes, much closer together than
        // the cones of volcanic provinces.
        let cone = if plates.arc > ARC_MIN {
            cone.max(self.cone_on(
                [nx, ny, nz],
                self.arc_cone_seed,
                ARC_CONE_CELL_KM,
                ARC_CONE_PRESENCE,
            ))
        } else {
            cone
        };
        let volcanic_zone = broad * (VOLCANIC_BACKGROUND + (1.0 - VOLCANIC_BACKGROUND) * cone);

        let elevation = (continent
            + tectonic_uplift
            + mountain * mountain_weight * self.mountain_blend
            + CONE_HEIGHT * broad * cone)
            .clamp(-1.0, 1.0);

        // Positive sea_level raises the waterline (more ocean); negative lowers it.
        let biome_elevation = (elevation - self.sea_level).clamp(-1.0, 1.0);

        let moisture = fbm(
            &self.moisture_noise,
            nx * 1.5 * s,
            ny * 1.5 * s,
            nz * 1.5 * s,
            4,
        );

        // Features of ~3 000 km: whole zones shift between continents.
        let temperature_anomaly = fbm(
            &self.anomaly_noise,
            nx * 1.2 * s,
            ny * 1.2 * s,
            nz * 1.2 * s,
            3,
        );

        // Features from ~1 000 km down to ~60 km; two independent samples.
        let f = 6.0 * s;
        let variability_t = fbm(&self.variability_noise, nx * f, ny * f, nz * f, 5);
        let variability_m = fbm(
            &self.variability_noise,
            nx * f + 17.3,
            ny * f - 4.1,
            nz * f + 9.7,
            5,
        );

        let h = 4.0 * s;
        let highland_jitter = fbm(&self.highland_noise, nx * h, ny * h, nz * h, 4);

        Sample {
            warp_x: warp_x as f32,
            warp_y: warp_y as f32,
            continent,
            mountain,
            mountain_weight,
            elevation,
            biome_elevation,
            moisture,
            volcanic_raw,
            volcanic_zone,
            temperature_anomaly,
            variability_t,
            variability_m,
            highland_jitter,
            tectonic_uplift,
        }
    }
}

/// How a debug layer is coloured: signed values span `[-1, 1]`, unsigned `[0, 1]`.
#[derive(Clone, Copy)]
pub enum Ramp {
    Signed,
    Unsigned,
}

/// An intermediate layer for the diagnostic maps, or a band of its rows.
pub struct DebugLayer {
    pub name: &'static str,
    pub ramp: Ramp,
    /// Row-major values: a whole layer, or the next rows of one.
    pub values: Vec<f32>,
}

/// Receives the diagnostic layers as the generation computes them, so they
/// can be written and freed one by one instead of all being kept until the
/// export (they would cost about twenty map-sized layers of memory).
///
/// Most layers come whole, from the generation thread. The noise layers
/// are sampled band by band: they come as successive [`DebugLayer`]s of the
/// same name holding the next rows, top to bottom, one band of every noise
/// layer at a time from parallel threads. A layer is complete once it has
/// one value per pixel. [`NoiseMapWriter`](crate::export::noise_maps::NoiseMapWriter)
/// writes them as PNGs.
pub trait DebugSink: Sync {
    /// Takes a whole layer or the next band of one.
    fn layer(&self, layer: DebugLayer);
}

/// The output of [`generate`]: the world, and what its export reuses.
pub struct Generated {
    pub world: World,
    /// Anti-aliased river coverage of the map (see [`rivers::coverage`]),
    /// computed for the biomes and handed over to the export.
    pub river_coverage: Vec<f32>,
}

/// A noise layer exported as a diagnostic map: name, colour ramp, extractor.
type NoiseLayer = (&'static str, Ramp, fn(&Sample) -> f32);

const NOISE_LAYERS: &[NoiseLayer] = &[
    ("noise_warp_x", Ramp::Signed, |s| s.warp_x),
    ("noise_warp_y", Ramp::Signed, |s| s.warp_y),
    ("noise_continent", Ramp::Signed, |s| s.continent),
    ("noise_mountain", Ramp::Unsigned, |s| s.mountain),
    ("noise_mountain_wt", Ramp::Unsigned, |s| s.mountain_weight),
    ("noise_elevation", Ramp::Signed, |s| s.elevation),
    ("noise_biome_elev", Ramp::Signed, |s| s.biome_elevation),
    ("noise_moisture", Ramp::Signed, |s| s.moisture),
    ("noise_volcanic_raw", Ramp::Signed, |s| s.volcanic_raw),
    ("noise_volcanic_zone", Ramp::Unsigned, |s| s.volcanic_zone),
    ("noise_temperature_anomaly", Ramp::Signed, |s| {
        s.temperature_anomaly
    }),
    ("noise_variability_t", Ramp::Signed, |s| s.variability_t),
    ("noise_variability_m", Ramp::Signed, |s| s.variability_m),
    ("noise_highland_jitter", Ramp::Signed, |s| s.highland_jitter),
    ("noise_tectonic", Ramp::Signed, |s| s.tectonic_uplift * 4.0),
];

/// Coast distances are computed up to this value; further pixels get the cap.
/// Also the full scale of the `coast_distance` debug map.
pub const COAST_DISTANCE_CAP_KM: f32 = 2_000.0;

/// Full scale of the `flow` debug map: log10 of the drainage area in km².
const FLOW_LOG_MAX: f32 = 7.0;

/// A generation step, reported to progress callbacks as it starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Stage {
    Noise,
    Erosion,
    Temperature,
    Coast,
    Moisture,
    Rivers,
    Biomes,
    /// Writing the output files (reported by the caller of [`export_world`](crate::export::export_world)).
    Export,
}

impl Stage {
    /// Every stage, in the order they run.
    pub const ALL: [Stage; 8] = [
        Stage::Noise,
        Stage::Erosion,
        Stage::Temperature,
        Stage::Coast,
        Stage::Moisture,
        Stage::Rivers,
        Stage::Biomes,
        Stage::Export,
    ];
}

/// What [`generate_with_progress`] produces besides the world.
#[derive(Clone, Copy, Default)]
pub struct GenerateOptions<'a> {
    /// Receives every intermediate layer for the diagnostic maps as soon as
    /// it is computed. None (the default) skips them, and the work and
    /// memory they need.
    pub debug: Option<&'a dyn DebugSink>,
}

/// The noise fields later passes need, one value per pixel (much smaller
/// than keeping every [`Sample`]).
struct NoiseFields {
    elevation: Vec<f32>,
    moisture: Vec<f32>,
    volcanic_zone: Vec<f32>,
    temperature_anomaly: Vec<f32>,
    variability_t: Vec<f32>,
    variability_m: Vec<f32>,
    highland_offset: Vec<f32>,
}

/// Rows sampled at a time by [`NoiseFields::sample`]: enough to keep every
/// core busy, few enough that the band of samples stays small (about 30 MB
/// at 8192 pixels of width).
const BAND_ROWS: usize = 64;

impl NoiseFields {
    /// Samples every pixel, band by band, writing each band's fields
    /// straight into their layers (and, when `debug` is given, handing it
    /// each band of the noise debug layers): every sample is never held at
    /// once.
    fn sample(sampler: &Sampler, width: u32, height: u32, debug: Option<&dyn DebugSink>) -> Self {
        let n = (width * height) as usize;
        let w = width as usize;
        let mut f = Self {
            elevation: vec![0.0; n],
            moisture: vec![0.0; n],
            volcanic_zone: vec![0.0; n],
            temperature_anomaly: vec![0.0; n],
            variability_t: vec![0.0; n],
            variability_m: vec![0.0; n],
            highland_offset: vec![0.0; n],
        };
        let mut band: Vec<Sample> = Vec::with_capacity(BAND_ROWS * w);
        for y0 in (0..height as usize).step_by(BAND_ROWS) {
            let y1 = (y0 + BAND_ROWS).min(height as usize);
            band.clear();
            band.par_extend(
                (y0..y1)
                    .into_par_iter()
                    .flat_map_iter(|y| (0..w).map(move |x| sampler.sample(x as u32, y as u32))),
            );
            let range = y0 * w..y1 * w;
            let fill = |layer: &mut [f32], value: fn(&Sample) -> f32| {
                layer[range.clone()]
                    .par_iter_mut()
                    .zip(band.par_iter())
                    .for_each(|(v, s)| *v = value(s));
            };
            fill(&mut f.elevation, |s| s.elevation);
            fill(&mut f.moisture, |s| s.moisture);
            fill(&mut f.volcanic_zone, |s| s.volcanic_zone);
            fill(&mut f.temperature_anomaly, |s| s.temperature_anomaly);
            fill(&mut f.variability_t, |s| s.variability_t);
            fill(&mut f.variability_m, |s| s.variability_m);
            fill(&mut f.highland_offset, Sample::highland_offset);
            if let Some(sink) = debug {
                // One task per layer: each one is coloured and encoded as
                // its band arrives.
                NOISE_LAYERS.par_iter().for_each(|&(name, ramp, value)| {
                    sink.layer(DebugLayer {
                        name,
                        ramp,
                        values: band.iter().map(value).collect(),
                    })
                });
            }
        }
        f
    }
}

/// Size of the grid [`sea_level_for_land_fraction`] samples the relief on.
const LAND_GRID: (u32, u32) = (512, 256);
/// Most passes [`sea_level_for_land_fraction`] makes; each one samples the
/// relief at the previous estimate, since the coast outline depends on it.
const LAND_PASSES: usize = 4;

/// The sea level that leaves `fraction` of the planet's surface (by area,
/// in `[0, 1]`) above the waterline, for the other parameters of `params`.
///
/// Samples the relief on a coarse grid and takes the area-weighted quantile
/// of its elevation. The coast outline shifts with the sea level, so this
/// repeats from the new estimate until it settles. Erosion runs later and
/// is not modelled, yet on test worlds the land share lands within about
/// 0.1 point of the target.
pub fn sea_level_for_land_fraction(params: &WorldParams, fraction: f32) -> f32 {
    let limits = crate::request::SEA_LEVEL;
    let mut coarse = params.clone();
    (coarse.width, coarse.height) = LAND_GRID;
    let grid = coarse.grid();
    let weights: Vec<f64> = (0..grid.height).map(|y| grid.latitude(y).cos()).collect();
    let total: f64 = weights.iter().sum::<f64>() * grid.width as f64;
    let waterline_shift = params.planet.rules.waterline;
    for _ in 0..LAND_PASSES {
        let sampler = Sampler::new(&coarse);
        let mut samples: Vec<(f32, f64)> = (0..grid.height)
            .into_par_iter()
            .flat_map_iter(|y| {
                let sampler = &sampler;
                let w = weights[y as usize];
                (0..grid.width).map(move |x| (sampler.sample(x, y).elevation, w))
            })
            .collect();
        samples.par_sort_unstable_by(|a, b| b.0.total_cmp(&a.0));
        // Walk down from the highest ground until the target area is reached.
        let target = fraction.clamp(0.0, 1.0) as f64 * total;
        let mut area = 0.0;
        let mut waterline = samples.last().map_or(-1.0, |s| s.0);
        for &(elevation, w) in &samples {
            area += w;
            if area >= target {
                waterline = elevation;
                break;
            }
        }
        let level = (waterline - waterline_shift).clamp(limits.min, limits.max);
        let settled = (level - coarse.sea_level).abs() < 1e-3;
        coarse.sea_level = level;
        if settled {
            break;
        }
    }
    coarse.sea_level
}

/// Generates a world, running every pass in order, without debug layers.
pub fn generate(params: &WorldParams) -> Generated {
    generate_with_progress(params, GenerateOptions::default(), |_| {})
}

/// Like [`generate`], with options, calling `progress` as each stage starts.
pub fn generate_with_progress(
    params: &WorldParams,
    options: GenerateOptions,
    progress: impl Fn(Stage),
) -> Generated {
    let (width, height) = (params.width, params.height);
    let planet = &params.planet;
    let sink = options.debug;
    // Hands a debug layer to the sink, computing it only when there is one.
    let debug = |name: &'static str, ramp: Ramp, values: &dyn Fn() -> Vec<f32>| {
        if let Some(sink) = sink {
            sink.layer(DebugLayer {
                name,
                ramp,
                values: values(),
            });
        }
    };

    // ── Pass 1: noise ─────────────────────────────────────────────────────────
    progress(Stage::Noise);
    // Every pixel is independent: sample rows in parallel. The result stays
    // deterministic because each pixel only depends on its coordinates.
    let sampler = Sampler::new(params);
    let fields = NoiseFields::sample(&sampler, width, height, sink);

    let rules = &planet.rules;
    let grid = params.grid();
    let n = params.pixel_count();
    // The tectonic plates, as diagnostic maps only for now (see `plates`).
    if let Some(sink) = sink {
        let map = crate::plates::PlateMap::new(params);
        let count = map.plate.iter().copied().max().unwrap_or(0).max(1);
        let at = |i: usize| {
            let (x, y) = (i as u32 % width, i as u32 / width);
            let lon = (f64::from(x) + 0.5) / f64::from(width) * 2.0 * PI;
            map.index_at(lon, grid.latitude(y))
        };
        sink.layer(DebugLayer {
            name: "plates",
            ramp: Ramp::Unsigned,
            values: (0..n)
                .into_par_iter()
                .map(|i| f32::from(map.plate[at(i)]) / f32::from(count))
                .collect(),
        });
        sink.layer(DebugLayer {
            name: "plate_boundaries",
            ramp: Ramp::Signed,
            // Convergence (10 cm/yr full scale), fading 400 km from the
            // boundary: red where plates close, blue where they part.
            values: (0..n)
                .into_par_iter()
                .map(|i| {
                    let k = at(i);
                    let fade = (-map.boundary_km[k] / 400.0).exp();
                    (map.convergence[k] / 10.0 * fade).clamp(-1.0, 1.0)
                })
                .collect(),
        });
    }

    // ── Pass 2: hydraulic erosion ─────────────────────────────────────────────
    progress(Stage::Erosion);
    let NoiseFields {
        mut elevation,
        moisture: moisture_noise,
        volcanic_zone: raw_volcanic_zone,
        temperature_anomaly: anomaly,
        variability_t,
        variability_m,
        highland_offset,
    } = fields;
    // Rows of sea-level temperature, evened out by dense air.
    // The host star: its light, regulated by a carbon cycle on wet worlds
    // with plate tectonics, and the planet's own heat.
    let star_climate = StarClimate {
        star: params.star,
        orbit_au: params.orbit_au,
        eccentricity: params.eccentricity,
        regulated: planet.geology.regime == Regime::Plates
            && params.pressure_bar >= LIQUID_WATER_BAR,
        internal_heat_k: planet.climate.internal_heat,
    };
    // A tidally locked planet's climate follows the angle from the point
    // under its star instead of the latitude.
    let locked = StarInfo::new(params.star, params.orbit_au, params.eccentricity)
        .tidally_locked
        .then(|| {
            let mut locked = LockedClimate::new();
            locked.transport_heat(params.pressure_bar);
            star_temperatures(locked.profile_mut(), star_climate);
            locked
        });
    let row_temperature = match &locked {
        Some(locked) => locked.rows(&grid),
        None => {
            let mut rows = row_temperatures(&grid, params.axial_tilt_deg);
            transport_heat(&mut rows, &grid, params.pressure_bar);
            star_temperatures(&mut rows, star_climate);
            rows
        }
    };
    // The planet's shift and its atmosphere's greenhouse warming.
    let warming = planet.climate.temperature + planet.atmosphere.greenhouse;
    let thaw = crate::erosion::thaw(&row_temperature, warming, rules.polar);
    // Impact craters: those of the wet era are worn by the erosion, the
    // younger ones are drawn over it. Their height change is only kept for
    // its debug map.
    let craters = CraterField::new(params, base_min_diameter_km(&grid));
    let mut crater_relief = sink.map(|_| elevation.clone());
    craters.apply(&mut elevation, &grid, Era::Wet);
    if let Some(relief) = crater_relief.as_mut() {
        relief
            .par_iter_mut()
            .zip(&elevation)
            .for_each(|(r, e)| *r = e - *r);
    }
    // The height change is only kept for its debug map.
    let mut erosion = sink.map(|_| vec![0.0; n]);
    erode(
        &mut elevation,
        (width, height),
        params.sea_level + rules.waterline,
        params.erosion,
        params.seed,
        &thaw,
        erosion.as_deref_mut(),
    );
    if let (Some(sink), Some(mut values)) = (sink, erosion) {
        // ×10: height changes are small next to the [-1, 1] elevation range.
        values
            .par_iter_mut()
            .for_each(|d| *d = (*d * 10.0).clamp(-1.0, 1.0));
        sink.layer(DebugLayer {
            name: "erosion",
            ramp: Ramp::Signed,
            values,
        });
    }
    // The young craters' height change, kept to export the elevation
    // without them (the surface the time slider draws them on).
    let mut young_craters = elevation.clone();
    let mare = craters.apply(&mut elevation, &grid, Era::Dry);
    drop(craters);
    // The bright ejecta and rays of fresh craters, down to the pixel, over
    // the dark lava of the maria.
    let pixel_km = f64::from(grid.circumference_km) / f64::from(width);
    let mut albedo = CraterField::new(params, RAY_MIN_PIXELS * pixel_km).albedo(&grid);
    darken_maria(&mut albedo, &mare);
    young_craters
        .par_iter_mut()
        .zip(&elevation)
        .for_each(|(d, e)| *d = e - *d);
    if let (Some(sink), Some(mut values)) = (sink, crater_relief) {
        // The young craters' change added to the old ones' (the erosion in
        // between left out); ×5: a crater moves the elevation by a few
        // tenths at most.
        values
            .par_iter_mut()
            .zip(&young_craters)
            .for_each(|(r, d)| *r = ((*r + d) * 5.0).clamp(-1.0, 1.0));
        sink.layer(DebugLayer {
            name: "craters",
            ramp: Ramp::Signed,
            values,
        });
    }
    // Elevation relative to the shifted sea level: what biome selection sees.
    // Computed where needed rather than kept as a layer.
    let relief = |e: f32| (e - params.sea_level).clamp(-1.0, 1.0);

    // ── Pass 3: temperature ───────────────────────────────────────────────────
    progress(Stage::Temperature);
    let raw_temperature: Vec<f32> = elevation
        .par_iter()
        .enumerate()
        .map(|(i, &e)| {
            let above_water = relief(e) - rules.waterline;
            let sea_level = match &locked {
                Some(locked) => locked.at_angle(LockedClimate::angle_at(
                    &grid,
                    (i % width as usize) as u32,
                    (i / width as usize) as u32,
                )),
                None => row_temperature[i / width as usize],
            };
            temperature(sea_level, above_water)
                + temperature_anomaly(anomaly[i], above_water >= 0.0)
        })
        .collect();
    drop(anomaly);
    debug("temperature", Ramp::Unsigned, &|| {
        // −30 °C to 40 °C over the ramp.
        raw_temperature
            .par_iter()
            .map(|t| ((t + 30.0) / 70.0).clamp(0.0, 1.0))
            .collect()
    });

    // ── Pass 4: distance to the coast ─────────────────────────────────────────
    progress(Stage::Coast);
    // Shores are the land pixels close to open water.
    let open_water: Vec<bool> = elevation
        .par_iter()
        .map(|&e| relief(e) < rules.waterline)
        .collect();
    let coast_distance_km = grid.distance_to(&open_water, COAST_DISTANCE_CAP_KM);
    drop(open_water);
    debug("coast_distance", Ramp::Unsigned, &|| {
        coast_distance_km
            .par_iter()
            .map(|d| d / COAST_DISTANCE_CAP_KM)
            .collect()
    });

    // ── Pass 5: moisture ──────────────────────────────────────────────────────
    progress(Stage::Moisture);
    // Winds carry ocean evaporation inland; it rains out with distance and on
    // windward slopes, leaving rain shadows behind mountains.
    let height_above_water: Vec<f32> = elevation
        .par_iter()
        .map(|&e| relief(e) - rules.waterline)
        .collect();
    let (raw_moisture, precipitation) = moisture(&MoistureInput {
        grid,
        height_above_water: &height_above_water,
        temperature: &raw_temperature,
        noise: &moisture_noise,
    });
    drop(moisture_noise);
    // Clouds form where air rises and rains (see the clouds module).
    let clouds = crate::clouds::clouds(&CloudInput {
        grid,
        kind: if params.pressure_bar < AIRLESS_BAR {
            CloudKind::None
        } else {
            planet.atmosphere.clouds
        },
        cover_shift: planet.atmosphere.cloud_cover,
        precipitation: &precipitation,
        seed: params.seed,
    });
    if let Some(sink) = sink {
        sink.layer(DebugLayer {
            name: "precipitation",
            ramp: Ramp::Signed,
            values: precipitation,
        });
    } else {
        drop(precipitation);
    }
    if !clouds.is_empty() {
        debug("clouds", Ramp::Unsigned, &|| clouds.clone());
    }
    debug("moisture", Ramp::Signed, &|| raw_moisture.clone());

    // Planet-type global offsets shift the whole climate before rivers and
    // biomes. Store the climate exactly as biome selection sees it, so the
    // exported data explains the biome of every pixel.
    let shift = planet.climate;
    let (dt, dm, dvz) = (warming, shift.moisture, shift.volcanic);
    // Too thin an atmosphere keeps no liquid water: no rain anywhere, so no
    // rivers or lakes.
    let liquid_water = params.pressure_bar >= LIQUID_WATER_BAR;
    // Regional variability is part of the stored climate, so the exported
    // layers still explain every biome.
    // Shifted in place: the raw fields are not needed any more.
    let mut moisture = raw_moisture;
    moisture
        .par_iter_mut()
        .zip(&variability_m)
        .for_each(|(m, v)| {
            *m = if liquid_water {
                (*m + dm + regional_variability(*v, Field::Moisture)).clamp(-1.0, 1.0)
            } else {
                -1.0
            }
        });
    let mut temperature = raw_temperature;
    temperature
        .par_iter_mut()
        .zip(&variability_t)
        .for_each(|(t, v)| {
            *t = (*t + dt + regional_variability(*v, Field::Temperature))
                .clamp(TEMPERATURE_RANGE.0, TEMPERATURE_RANGE.1)
        });
    drop((variability_t, variability_m));

    // ── Pass 6: rivers and lakes ──────────────────────────────────────────────
    progress(Stage::Rivers);
    let mut water = hydrology(&grid, &height_above_water, &moisture, params.seed);
    // Planets without anything liquid to flow (rivers remapped to dry
    // ground) have no rivers to draw, carve or end in deltas: only the
    // drainage flow is kept. Lava rivers of volcanic worlds still flow.
    let flowing = planet.remap(Biome::River);
    let traced = (flowing.is_water() || flowing.is_lava()).then(|| rivers::trace(&grid, &water));
    // The receivers and river pixels only serve the tracing.
    water.release_routing();
    let mut river_lines = match traced {
        Some(traced) => rivers::shape(&traced, &grid, &water.filled, params.seed),
        None => Vec::new(),
    };
    // Deltas and estuaries at the mouths of the large rivers.
    let deltas = delta::outlets(&mut river_lines, &grid, &height_above_water, params.seed);
    drop(height_above_water);
    // Valleys and floodplains along the rivers. Carving never creates water,
    // so the coast distance and the drainage stay valid.
    let waterline = params.sea_level + rules.waterline;
    let mut above: Vec<f32> = elevation.par_iter().map(|e| e - waterline).collect();
    let mut floodplain = carve::carve(&river_lines, &grid, &water.filled, &water.lake, &mut above);
    // Only the flow and the lakes are needed from here on.
    let (flow_km2, lake) = water.into_flow_and_lakes();
    let delta_plain = delta::plains(&deltas, &river_lines, &grid, &mut above, &mut moisture);
    floodplain
        .par_iter_mut()
        .zip(delta_plain)
        .for_each(|(f, d)| *f = f.max(d));
    elevation
        .par_iter_mut()
        .zip(above)
        .for_each(|(e, a)| *e = a + waterline);
    // Rivers water their floodplain: green strips across dry land.
    moisture
        .par_iter_mut()
        .zip(&floodplain)
        .for_each(|(m, f)| *m = (*m + RIPARIAN_MOISTURE * f).clamp(-1.0, 1.0));
    debug("elevation", Ramp::Signed, &|| elevation.clone());
    debug("floodplain", Ramp::Unsigned, &|| floodplain.clone());
    // The biome layer marks rivers where they are drawn, so it matches the map.
    let river_coverage =
        rivers::coverage(&river_lines, width, height, params.circumference_km as f64);
    debug("flow", Ramp::Unsigned, &|| {
        flow_km2
            .par_iter()
            .map(|f| (f + 1.0).log10() / FLOW_LOG_MAX)
            .collect()
    });

    // ── Pass 7: biomes ────────────────────────────────────────────────────────
    progress(Stage::Biomes);
    let mut volcanic_zone = raw_volcanic_zone;
    volcanic_zone
        .par_iter_mut()
        .for_each(|v| *v = (*v + dvz).clamp(0.0, 1.0));
    let biomes: Vec<Biome> = (0..n)
        .into_par_iter()
        .map(|i| {
            let input = BiomeInput {
                elevation: relief(elevation[i]),
                moisture: moisture[i],
                temperature: temperature[i],
                volcanic_zone: volcanic_zone[i],
                coast_distance_km: coast_distance_km[i],
                lake: lake[i],
                river: !lake[i] && river_coverage[i] >= rivers::RIVER_COVERAGE,
                highland_offset: highland_offset[i],
            };
            planet.remap(rules.choose(input))
        })
        .collect();

    // Later passes (valleys, deltas) change the elevation additively: they
    // stay in the base.
    young_craters
        .par_iter_mut()
        .zip(&elevation)
        .for_each(|(d, e)| *d = e - *d);
    let elevation_base = Some(young_craters);
    Generated {
        world: World {
            params: params.clone(),
            width,
            height,
            elevation,
            elevation_base,
            moisture,
            temperature,
            flow_km2,
            volcanic_zone,
            coast_distance_km,
            floodplain,
            highland_offset,
            biomes,
            rivers: river_lines,
            albedo,
            clouds,
        },
        river_coverage,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::climate::EARTH_AXIAL_TILT_DEG;
    use crate::preset::Preset;

    pub fn params(seed: u32, planet: &str) -> WorldParams {
        WorldParams {
            width: 64,
            height: 32,
            seed,
            planet: Preset::load(planet).unwrap(),
            sea_level: 0.1,
            volcanic_intensity: 0.5,
            circumference_km: EARTH_CIRCUMFERENCE_KM,
            axial_tilt_deg: EARTH_AXIAL_TILT_DEG,
            erosion: 1.0,
            pressure_bar: 1.0,
            surface_age_ga: 0.4,
            star: crate::star::SUN,
            orbit_au: 1.0,
            eccentricity: 0.0,
        }
    }

    fn small_world(seed: u32, planet: &str) -> World {
        generate(&params(seed, planet)).world
    }

    #[test]
    fn same_seed_produces_same_world() {
        let a = small_world(42, "terran");
        let b = small_world(42, "terran");
        assert_eq!(a.biomes, b.biomes);
        assert_eq!(a.elevation, b.elevation);
    }

    #[test]
    fn seed_near_u32_max_does_not_overflow() {
        small_world(u32::MAX, "terran");
    }

    #[test]
    fn every_layer_has_one_value_per_pixel() {
        let w = small_world(7, "frozen");
        for len in [
            w.elevation.len(),
            w.moisture.len(),
            w.temperature.len(),
            w.biomes.len(),
        ] {
            assert_eq!(len, 64 * 32);
        }
    }

    #[test]
    fn stored_climate_includes_planet_offsets() {
        // Without erosion, which only runs where it thaws, so the relief
        // (and its lapse rate) is the same on both.
        let world = |planet: &str| {
            let mut p = params(5, planet);
            // Without plate tectonics on either, which would add relief to
            // the terran world only.
            p.planet.geology.regime = crate::geology::Regime::Episodic;
            generate(&WorldParams { erosion: 0.0, ..p }).world
        };
        // Both around the Sun at 1 AU, where the star changes nothing.
        let (caustic, terran) = (world("caustic"), world("terran"));
        // The caustic preset warms every pixel by its shift and greenhouse.
        let planet = &caustic.params.planet;
        let shift = planet.climate.temperature + planet.atmosphere.greenhouse;
        assert_eq!(planet.climate.temperature, 4.5);
        for (c, t) in caustic.temperature.iter().zip(&terran.temperature) {
            assert!((c - (t + shift)).abs() < 1e-3, "{c} vs {t}");
        }
        assert!(caustic.moisture.iter().all(|m| (-1.0..=1.0).contains(m)));
    }

    #[test]
    fn gravity_is_earth_baseline_and_clamped() {
        let g = |circumference_km| {
            WorldParams {
                circumference_km,
                ..params(0, "terran")
            }
            .gravity_modifier()
        };
        assert!((g(EARTH_CIRCUMFERENCE_KM) - 1.0).abs() < 1e-6);
        assert_eq!(g(1.0), 0.1);
        assert_eq!(g(1e9), 5.0);
    }

    #[test]
    fn progress_reports_every_generation_stage_in_order() {
        let seen = std::sync::Mutex::new(Vec::new());
        generate_with_progress(&params(1, "terran"), GenerateOptions::default(), |s| {
            seen.lock().unwrap().push(s)
        });
        assert_eq!(seen.into_inner().unwrap(), Stage::ALL[..7]);
    }

    /// A sink that keeps every debug layer, joining the bands of each one.
    #[derive(Default)]
    struct Collect(std::sync::Mutex<Vec<DebugLayer>>);

    impl DebugSink for Collect {
        fn layer(&self, piece: DebugLayer) {
            let mut layers = self.0.lock().unwrap();
            match layers.iter_mut().find(|l| l.name == piece.name) {
                Some(layer) => layer.values.extend(piece.values),
                None => layers.push(piece),
            }
        }
    }

    /// Generates with every debug layer, collected.
    fn generate_with_debug(params: &WorldParams) -> (Generated, Vec<DebugLayer>) {
        let sink = Collect::default();
        let g = generate_with_progress(params, GenerateOptions { debug: Some(&sink) }, |_| {});
        (g, sink.0.into_inner().unwrap())
    }

    #[test]
    fn skipping_debug_layers_gives_the_same_world() {
        let p = params(13, "volcanic");
        let (full, debug) = generate_with_debug(&p);
        let lean = generate(&p);
        assert_eq!(debug.len(), NOISE_LAYERS.len() + 12);
        assert_eq!(full.world.biomes, lean.world.biomes);
        assert_eq!(full.world.elevation, lean.world.elevation);
        assert_eq!(full.world.moisture, lean.world.moisture);
        assert_eq!(full.world.rivers.len(), lean.world.rivers.len());
        assert_eq!(full.river_coverage, lean.river_coverage);
    }

    #[test]
    fn dry_planets_have_no_rivers() {
        let barren = generate(&params(3, "barren")).world;
        assert!(barren.rivers.is_empty());
        assert!(barren.floodplain.iter().all(|&f| f == 0.0));
        assert!(!generate(&params(3, "terran")).world.rivers.is_empty());
        assert!(!generate(&params(3, "volcanic")).world.rivers.is_empty());
    }

    #[test]
    fn debug_layers_match_the_world() {
        let p = params(11, "terran");
        let (g, debug) = generate_with_debug(&p);
        let n = p.pixel_count();
        assert!(debug.iter().all(|l| l.values.len() == n));
        let elevation = debug.iter().find(|l| l.name == "elevation").unwrap();
        assert_eq!(elevation.values, g.world.elevation);
    }

    #[test]
    fn handed_over_river_coverage_matches_the_rivers() {
        let g = generate(&params(3, "terran"));
        let w = &g.world;
        let coverage = rivers::coverage(
            &w.rivers,
            w.width,
            w.height,
            w.params.circumference_km as f64,
        );
        assert_eq!(g.river_coverage, coverage);
    }
}
