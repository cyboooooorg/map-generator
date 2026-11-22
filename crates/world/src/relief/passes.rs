//! The passes that build a body's height from its physical forcings:
//! the continents (crust line, Airy platform, continental edge, noise), the
//! mountains and the volcanoes. Each returns metres per cell and draws only
//! from the seed hierarchy: every pass takes the **body seed** and derives
//! its own label from it (`relief.continents`, `relief.mountains`,
//! `relief.volcanoes`), so no pass shares a stream with another.
//!
//! Nothing here reads the kind of the central object: the passes take the
//! radius, the gravity, the analogue body, the regime (plates or not), the
//! heat flux and the ocean share of a [`Forcing`].

use super::noise::{Octaves, Spectrum, legacy_unit};
use super::plates::unit;
use super::{ReliefError, sea_level};
use crate::grid::Grid;
use crate::stats::{area_quantile, distance_to};
use science::relief::crust::{CONTINENTAL_SLOPE, continental_share, crust, floating_height};
use science::relief::highlands::{DRY_PLAINS, HIGHLANDS, MOUNTAIN_RELIEF, highland_weight};
use science::relief::plates::arc_band_width;
use science::relief::spectrum::{
    Analogue, Province, REFERENCE_WAVELENGTH, amplitude, analogue_relief_scale, hurst, octaves,
    persistence, relief_scale,
};
use science::relief::volcanoes::{
    ARC_CONE_DENSITY, CONE_PRESENCE, CONE_SPACING, Cone, EARTH_VOLCANIC_INTENSITY, cone_size,
    heat_flux, volcanic_intensity,
};
use science::units::{Fraction, Metre, MetrePerSecondSquared, WattPerSquareMetre};
use std::collections::BTreeSet;
use std::f64::consts::{FRAC_PI_2, PI};
use timeline::{PlanetState, Seed, TectonicRegime};

/// Definition: the frequency of the first octave of the continents' noise
/// on the unit sphere, a sampling choice: the first octave's wavelength is
/// the radius of the body, and [`octaves`] is counted from it down to two
/// cells.
const BASE_FREQUENCY: f64 = 1.0;

/// Definition: the most cells of which the noise's fluctuation is measured
/// (a numerical budget, not a physical value); the cells are evenly
/// strided over the grid.
const FLUCTUATION_SAMPLES: usize = 2048;

/// Definition: samples per cell length over which each half-window mean of
/// the Haar fluctuation is taken, 4. The finest octave of the noise is two
/// cells long, so it is sampled eight times per wavelength: a numerical
/// quadrature of the noise's own resolution, not a physical value.
const HAAR_SAMPLES_PER_CELL: f64 = 4.0;

/// Definition: the octaves of the legacy's continents noise, 5, of gain
/// one half (its `fbm` normalisation, a sum divided by
/// `1 + 1/2 + 1/4 + 1/8 + 1/16 = 1.9375`;
/// `legacy-final:crates/mapgen/src/generation.rs:1255-1275`), the noise
/// [`DRY_PLAINS`] was fitted on. Its deviation per unit,
/// [`legacy_unit`]`(5, 1/2) = 0.314 × √(Σ 4⁻ⁱ) / 1.9375 = 0.187`, is the
/// unit the calibration is expressed in (the legacy's plains kept 0.3 of
/// the three fine octaves, `PLAIN_ROUGHNESS`, not ported, which lowers it
/// by 3 %, to 0.182).
const LEGACY_PLAINS_OCTAVES: u32 = 5;

/// The physical forcings the relief of a body is built from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Forcing {
    /// Radius of the body.
    pub radius: Metre,
    /// Gravity at its surface.
    pub gravity: MetrePerSecondSquared,
    /// The body whose measured relief its regime follows.
    pub analogue: Analogue,
    /// Whether plates move its crust.
    pub plates: bool,
    /// Mean heat flux through its surface.
    pub heat_flux: WattPerSquareMetre,
    /// Share of the surface its ocean covers; `None` on a world without
    /// liquid (a dry world).
    pub ocean_share: Option<Fraction>,
}

impl Forcing {
    /// The forcings of a planet's state: the regime `Plates` follows Earth
    /// and has plates; `Stagnant`, `Episodic` and `HeatPipe` follow Mars
    /// (relief study, calibration 1). The heat flux is the sum of the
    /// internal heat sources over the surface.
    ///
    /// # Errors
    ///
    /// [`ReliefError::NoSurface`] when the state has no solid surface.
    pub fn read(state: &PlanetState) -> Result<Forcing, ReliefError> {
        let surface = state.surface.as_ref().ok_or(ReliefError::NoSurface)?;
        let (plates, analogue) = match surface.regime.value() {
            TectonicRegime::Plates => (true, Analogue::Earth),
            _ => (false, Analogue::Mars),
        };
        let radius = Metre::from(*state.bulk.radius.value());
        let heat = &state.heat;
        let total = science::units::Watt(
            heat.radiogenic.value().0 + heat.tidal.value().0 + heat.secular.value().0,
        );
        Ok(Forcing {
            radius,
            gravity: *state.bulk.surface_gravity.value(),
            analogue,
            plates,
            heat_flux: heat_flux(total, radius),
            ocean_share: surface.liquid.as_ref().map(|l| *l.ocean_share.value()),
        })
    }
}

/// Length of a cell's side, from the cube's face spanning a quarter turn.
fn cell_length(grid: &Grid, radius: Metre) -> f64 {
    radius.0 * FRAC_PI_2 / f64::from(grid.n())
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// The point `angle` radians from `p` along its meridian, towards the
/// north pole (towards the x axis at the poles).
fn displaced(p: [f64; 3], angle: f64) -> [f64; 3] {
    let north = [0.0, 0.0, 1.0];
    let along = dot(north, p);
    let mut t = [
        north[0] - along * p[0],
        north[1] - along * p[1],
        north[2] - along * p[2],
    ];
    let mut norm = dot(t, t).sqrt();
    if norm < f64::EPSILON {
        t = [1.0, 0.0, 0.0];
        norm = 1.0;
    }
    let (s, c) = angle.sin_cos();
    [0, 1, 2].map(|a| p[a] * c + t[a] / norm * s)
}

/// The noise of the continents in metres and the mask of the thick crust
/// (continental, or highland, cells). Uses `seed` for the noise.
///
/// The noise is the fBm of the analogue's Hurst exponent, of unit variance.
/// The crust line is its area quantile that leaves [`continental_share`]
/// of the area above it; the cells above are the thick crust.
///
/// On a world with liquid the noise is then centred within each province:
/// the area-weighted mean of the province is subtracted from its cells, so
/// that the step between the provinces is the Airy step of [`continents`]
/// alone (the mask is the noise cut at its own quantile: uncentred, the
/// thick crust would carry the upper part of the noise and add its mean
/// to the step). Each province is then scaled so that the mean Haar
/// fluctuation of the noise at [`REFERENCE_WAVELENGTH`] is the analogue's
/// [`amplitude`] in that province, times [`analogue_relief_scale`] of the
/// gravity (the amplitude was measured at the analogue's own gravity).
///
/// A dry world has no sea to level its plains, and no step: its noise is
/// [`DRY_PLAINS`] per unit of the legacy's noise ([`LEGACY_PLAINS_OCTAVES`])
/// times [`relief_scale`], not centred.
pub(crate) fn continent_noise(grid: &Grid, seed: Seed, forcing: &Forcing) -> (Vec<f32>, Vec<bool>) {
    let h = hurst(forcing.analogue);
    let spectrum = Spectrum {
        hurst: h,
        octaves: octaves(forcing.radius, Metre(cell_length(grid, forcing.radius))),
        base_frequency: BASE_FREQUENCY,
    };
    let octs = Octaves::new(seed, &spectrum);
    let unit: Vec<f32> = (0..grid.cell_count())
        .map(|k| octs.fbm(grid.point(k)) as f32)
        .collect();
    let share = continental_share(forcing.analogue).get();
    let line = area_quantile(grid, &unit, 1.0 - share);
    let mask: Vec<bool> = unit.iter().map(|&v| v > line).collect();
    if forcing.ocean_share.is_none() {
        let scale = DRY_PLAINS.0
            * legacy_unit(LEGACY_PLAINS_OCTAVES, persistence(1.0))
            * relief_scale(forcing.gravity);
        let noise = unit
            .iter()
            .map(|&v| (f64::from(v) * scale) as f32)
            .collect();
        return (noise, mask);
    }
    let haar = haar_fluctuation(grid, &octs, forcing, h);
    let gravity = analogue_relief_scale(forcing.gravity, forcing.analogue);
    let level = |province: Province| {
        let thick = province == Province::Thick;
        let mean = province_mean(grid, &unit, &mask, thick);
        let scale = if haar > 0.0 {
            amplitude(forcing.analogue, province).0 * gravity / haar
        } else {
            0.0
        };
        (mean, scale)
    };
    let (thick, thin) = (level(Province::Thick), level(Province::Thin));
    let noise = (0..grid.cell_count())
        .map(|k| {
            let (mean, scale) = if mask[k] { thick } else { thin };
            ((f64::from(unit[k]) - mean) * scale) as f32
        })
        .collect();
    (noise, mask)
}

/// The area-weighted mean of `values` over the cells whose `mask` is `on`;
/// 0 when there are none.
fn province_mean(grid: &Grid, values: &[f32], mask: &[bool], on: bool) -> f64 {
    let (sum, area) =
        (0..grid.cell_count())
            .filter(|&k| mask[k] == on)
            .fold((0.0, 0.0), |(s, a), k| {
                let w = grid.area(k);
                (s + w * f64::from(values[k]), a + w)
            });
    if area > 0.0 { sum / area } else { 0.0 }
}

/// The mean Haar fluctuation of the unit fBm `octs` at
/// [`REFERENCE_WAVELENGTH`]: the mean of `|M2 − M1|`, `M1` and `M2` the
/// means of the noise over the two halves of a window one lag long running
/// north along the meridian, the relief study's statistic
/// (`docs/superpowers/studies/2026-11-22-relief/scripts/etopo_haar_split.py`).
/// Measured on the noise itself over up to [`FLUCTUATION_SAMPLES`] evenly
/// strided windows, each half averaged over [`HAAR_SAMPLES_PER_CELL`]
/// samples per cell length (not derived analytically). On a grid too coarse
/// to resolve the reference wavelength it is measured at a lag of two cells
/// and carried to the reference by the spectrum's own law, `lag^H`.
pub(crate) fn haar_fluctuation(grid: &Grid, octs: &Octaves, forcing: &Forcing, h: f64) -> f64 {
    let cell = cell_length(grid, forcing.radius);
    let lag = REFERENCE_WAVELENGTH.0.max(2.0 * cell);
    let half = lag / 2.0;
    let samples = ((half / cell) * HAAR_SAMPLES_PER_CELL).ceil().max(1.0) as usize;
    let mean = |p: [f64; 3], from: f64| {
        (0..samples)
            .map(|i| {
                let along = from + (i as f64 + 0.5) * half / samples as f64;
                octs.fbm(displaced(p, along / forcing.radius.0))
            })
            .sum::<f64>()
            / samples as f64
    };
    let step = (grid.cell_count() / FLUCTUATION_SAMPLES).max(1);
    let (sum, count) = (0..grid.cell_count())
        .step_by(step)
        .fold((0.0, 0.0), |(s, n), k| {
            let p = grid.point(k);
            (s + (mean(p, half) - mean(p, 0.0)).abs(), n + 1.0)
        });
    sum / count * (REFERENCE_WAVELENGTH.0 / lag).powf(h)
}

/// The continents of a body: its height in metres per cell, and the mask of
/// the continental cells. Uses the seed `body.derive("relief.continents")`.
///
/// A continental cell (thick crust, [`continent_noise`]) stands on the
/// Airy platform, [`floating_height`] of the thick crust, reached from the
/// coast at the slope [`CONTINENTAL_SLOPE`]; the sea floor is the datum, 0.
/// The noise, centred within each province, is added everywhere. A dry
/// world (no ocean) keeps only the noise, its plains.
pub fn continents(grid: &Grid, body: Seed, forcing: &Forcing) -> (Vec<f32>, Vec<bool>) {
    let (noise, mask) = continent_noise(grid, body.derive("relief.continents"), forcing);
    if forcing.ocean_share.is_none() {
        return (noise, mask);
    }
    let crust = crust(forcing.analogue);
    let platform = floating_height(&crust, crust.continental_thickness).0;
    let ocean: Vec<bool> = mask.iter().map(|&m| !m).collect();
    let distance = distance_to(grid, forcing.radius, &ocean, None);
    let rise = CONTINENTAL_SLOPE.0.tan();
    let heights = (0..grid.cell_count())
        .map(|k| {
            let step = if mask[k] {
                platform.min(f64::from(distance[k]) * rise)
            } else {
                0.0
            };
            step as f32 + noise[k]
        })
        .collect();
    (heights, mask)
}

/// Weight of the highland provinces of a world without plates, in `[0, 1]`
/// per cell: the legacy's three-octave noise ([`HIGHLANDS`]) through its
/// ramp. Uses `body.derive("relief.mountains").derive("highlands")`.
///
/// The legacy value was a Perlin sum normalised by its octaves' total
/// amplitude (octave gain one half); here the unit-variance fBm of the same
/// gain is brought back to that normalisation, so that the ramp's edges keep
/// the share of the sphere they were fitted to.
pub fn highlands(grid: &Grid, body: Seed) -> Vec<f32> {
    let spectrum = Spectrum {
        hurst: 1.0,
        octaves: HIGHLANDS.octaves,
        base_frequency: HIGHLANDS.frequency,
    };
    let octs = Octaves::new(
        body.derive("relief.mountains").derive("highlands"),
        &spectrum,
    );
    let legacy = legacy_unit(HIGHLANDS.octaves, persistence(spectrum.hurst));
    (0..grid.cell_count())
        .map(|k| highland_weight(octs.fbm(grid.point(k)) * legacy) as f32)
        .collect()
}

/// The mountains: ridged noise (crests only) divided by its own maximum
/// over the cells it is drawn on (so its highest crest is 1, and the
/// highest summit stands [`MOUNTAIN_RELIEF`] above its plateau, as that
/// height is defined), times `weight` (the orogeny of the plates, or
/// [`highlands`] on a world without), times [`MOUNTAIN_RELIEF`] and
/// [`relief_scale`], in metres per cell. Uses
/// `body.derive("relief.mountains")`.
///
/// # Panics
///
/// Panics when `weight` does not hold one value per cell.
pub fn mountains(grid: &Grid, body: Seed, forcing: &Forcing, weight: &[f32]) -> Vec<f32> {
    assert_eq!(weight.len(), grid.cell_count(), "one weight per cell");
    let spectrum = Spectrum {
        hurst: hurst(forcing.analogue),
        octaves: octaves(forcing.radius, Metre(cell_length(grid, forcing.radius))),
        base_frequency: BASE_FREQUENCY,
    };
    let octs = Octaves::new(body.derive("relief.mountains"), &spectrum);
    let crests: Vec<f64> = (0..grid.cell_count())
        .map(|k| {
            if weight[k] <= 0.0 {
                return 0.0;
            }
            octs.ridged(grid.point(k)).max(0.0)
        })
        .collect();
    let peak = crests.iter().copied().fold(0.0, f64::max);
    if peak <= 0.0 {
        return vec![0.0; grid.cell_count()];
    }
    let scale = MOUNTAIN_RELIEF.0 * relief_scale(forcing.gravity) / peak;
    crests
        .iter()
        .zip(weight)
        .map(|(&c, &w)| (c * f64::from(w) * scale) as f32)
        .collect()
}

/// Raises the columns under the orogens towards their elevation above the
/// sea: each cell gains `w · max(0, e − (height − sea))`, `w` its orogeny
/// weight and `e` its orogen's elevation
/// ([`super::PlateRelief::orogeny`] and [`super::PlateRelief::elevation`]),
/// the sea the level that `ocean_share` sets on `height` ([`sea_level`]).
/// The sources give the orogens' elevations above sea level, so what is
/// added is the lift from the local surface the model built, never a
/// height on top of it: where `w` is 1 the column stands at `e`, and a
/// column already higher is left alone.
///
/// # Panics
///
/// Panics when `weight` or `elevation` does not hold one value per cell.
pub fn raise_orogens(
    grid: &Grid,
    height: &mut [f32],
    weight: &[f32],
    elevation: &[f32],
    ocean_share: Option<Fraction>,
) {
    assert_eq!(weight.len(), height.len(), "one weight per cell");
    assert_eq!(elevation.len(), height.len(), "one elevation per cell");
    let sea = sea_level(grid, height, ocean_share);
    for ((h, &w), &e) in height.iter_mut().zip(weight).zip(elevation) {
        *h += w * (e - (*h - sea)).max(0.0);
    }
}

/// A volcano's cone and where its axis stands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PlacedCone {
    pub centre: [f64; 3],
    pub cone: Cone,
}

/// The cones of a body: isolated ones on a lattice of [`CONE_SPACING`],
/// present in [`CONE_PRESENCE`] of the cells at Earth's volcanic intensity
/// (in proportion to the intensity, up to every cell), and arc cones in the
/// band of `arcs` at random in their cell, so that the expected number per
/// metre of arc is [`ARC_CONE_DENSITY`] whatever the band's width: a cell's
/// presence is its weight times the density per area,
/// `ARC_CONE_DENSITY / arc_band_width()`, times the cell's area.
/// Their sizes are `cone_size` at standard gravity, both height and radius
/// then scaled by [`relief_scale`] (the shape, so the aspect, is kept).
pub(crate) fn cones(grid: &Grid, body: Seed, forcing: &Forcing, arcs: &[f32]) -> Vec<PlacedCone> {
    assert_eq!(arcs.len(), grid.cell_count(), "one arc weight per cell");
    let seed = body.derive("relief.volcanoes");
    let scale = relief_scale(forcing.gravity);
    let draw = |label: &str| unit(seed.derive(label).get());
    let size = |label: &str| {
        let volume = Fraction::new(draw(&format!("{label}/volume"))).expect("a unit draw");
        let aspect = Fraction::new(draw(&format!("{label}/aspect"))).expect("a unit draw");
        let c = cone_size(volume, aspect);
        Cone {
            height: Metre(c.height.0 * scale),
            radius: Metre(c.radius.0 * scale),
        }
    };
    let mut placed = Vec::new();
    let intensity = volcanic_intensity(forcing.heat_flux).get();
    let presence = (CONE_PRESENCE * intensity / EARTH_VOLCANIC_INTENSITY).min(1.0);
    if presence > 0.0 {
        let rows = ((PI * forcing.radius.0 / CONE_SPACING.0).round() as usize).max(1);
        for r in 0..rows {
            let (lo, hi) = (
                -FRAC_PI_2 + PI * r as f64 / rows as f64,
                -FRAC_PI_2 + PI * (r + 1) as f64 / rows as f64,
            );
            let middle = (lo + hi) / 2.0;
            let columns = ((2.0 * PI * middle.cos() * forcing.radius.0 / CONE_SPACING.0).round()
                as usize)
                .max(1);
            for c in 0..columns {
                let label = format!("lattice/{r}/{c}");
                if draw(&format!("{label}/present")) >= presence {
                    continue;
                }
                // Uniform in area within the band: uniform in sin(latitude).
                let z = lo.sin() + (hi.sin() - lo.sin()) * draw(&format!("{label}/z"));
                let lon = 2.0 * PI * (c as f64 + draw(&format!("{label}/lon"))) / columns as f64;
                let ring = (1.0 - z * z).max(0.0).sqrt();
                placed.push(PlacedCone {
                    centre: [ring * lon.cos(), ring * lon.sin(), z],
                    cone: size(&label),
                });
            }
        }
    }
    let side = cell_length(grid, forcing.radius);
    let band = arc_band_width().0;
    for k in (0..grid.cell_count()).filter(|&k| arcs[k] > 0.0) {
        let label = format!("arc/{k}");
        let expected = f64::from(arcs[k]) * ARC_CONE_DENSITY * side * side / band;
        if draw(&format!("{label}/present")) >= expected {
            continue;
        }
        let (face, x, y) = grid.locate(grid.point(k));
        let jitter = |axis: &str| draw(&format!("{label}/{axis}")) - 0.5;
        placed.push(PlacedCone {
            centre: grid.point_at(face, x + jitter("x"), y + jitter("y")),
            cone: size(&label),
        });
    }
    placed
}

/// The volcanoes of a body: the height of their cones in metres per cell.
/// A cone is a straight profile from its height at the axis to 0 at its
/// radius; one narrower than a cell puts its volume into the cell it stands
/// in (mean height over the cell). Cones overlap by their maximum. Uses
/// `body.derive("relief.volcanoes")`; see [`cones`].
pub fn volcanoes(grid: &Grid, body: Seed, forcing: &Forcing, arcs: &[f32]) -> Vec<f32> {
    let mut out = vec![0.0_f32; grid.cell_count()];
    let side = cell_length(grid, forcing.radius);
    for placed in cones(grid, body, forcing, arcs) {
        let Cone { height, radius } = placed.cone;
        let centre = grid.index_of(placed.centre);
        if radius.0 < side / 2.0 {
            // Volume of a cone, pi r^2 h / 3, over the cell's area.
            let volume = PI * radius.0 * radius.0 * height.0 / 3.0;
            let area = grid.area(centre) * forcing.radius.0 * forcing.radius.0;
            let mean = (volume / area).min(height.0) as f32;
            out[centre] = out[centre].max(mean);
            continue;
        }
        let mut seen = BTreeSet::from([centre]);
        let mut frontier = vec![centre];
        while let Some(k) = frontier.pop() {
            let d = forcing.radius.0 * dot(placed.centre, grid.point(k)).clamp(-1.0, 1.0).acos();
            if d >= radius.0 {
                continue;
            }
            out[k] = out[k].max((height.0 * (1.0 - d / radius.0)) as f32);
            for j in grid.neighbours(k) {
                if seen.insert(j) {
                    frontier.push(j);
                }
            }
        }
    }
    out
}

#[cfg(test)]
#[path = "passes_tests.rs"]
mod tests;
