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

use super::ReliefError;
use super::noise::{Octaves, PERLIN_RMS, Spectrum};
use super::plates::unit;
use crate::grid::Grid;
use crate::stats::{area_quantile, distance_to};
use science::relief::crust::{CONTINENTAL_SLOPE, continental_share, crust, floating_height};
use science::relief::highlands::{DRY_PLAINS, HIGHLANDS, MOUNTAIN_RELIEF, highland_weight};
use science::relief::plates::arc_band_width;
use science::relief::spectrum::{
    Analogue, REFERENCE_WAVELENGTH, amplitude, hurst, octaves, persistence, relief_scale,
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

/// The continents' noise in metres: the fBm of the analogue's Hurst
/// exponent, scaled so that its mean absolute fluctuation over a lag of
/// [`REFERENCE_WAVELENGTH`] is [`amplitude`] (the statistic the relief study
/// measured on ETOPO 2022 and MOLA), times [`relief_scale`] of the gravity.
/// A dry world has no sea to level its plains: its noise is scaled by
/// [`DRY_PLAINS`] per unit instead, times [`relief_scale`].
///
/// The fluctuation is measured on the noise itself, once per pass, over up
/// to 2048 evenly strided cells (not derived analytically). On a grid too
/// coarse to resolve the reference wavelength it is measured at a lag of
/// two cells and carried to the reference by the spectrum's own law,
/// `lag^H`.
pub(crate) fn continent_noise(grid: &Grid, seed: Seed, forcing: &Forcing) -> Vec<f32> {
    let h = hurst(forcing.analogue);
    let spectrum = Spectrum {
        hurst: h,
        octaves: octaves(forcing.radius, Metre(cell_length(grid, forcing.radius))),
        base_frequency: BASE_FREQUENCY,
    };
    let octs = Octaves::new(seed, &spectrum);
    let scale = if forcing.ocean_share.is_none() {
        DRY_PLAINS.0
    } else {
        amplitude_scale(grid, &octs, forcing, h)
    } * relief_scale(forcing.gravity);
    (0..grid.cell_count())
        .map(|k| (octs.fbm(grid.point(k)) * scale) as f32)
        .collect()
}

/// The factor taking the unit-variance fBm to the analogue's amplitude.
fn amplitude_scale(grid: &Grid, octs: &Octaves, forcing: &Forcing, h: f64) -> f64 {
    let cell = cell_length(grid, forcing.radius);
    let lag = REFERENCE_WAVELENGTH.0.max(2.0 * cell);
    let step = (grid.cell_count() / FLUCTUATION_SAMPLES).max(1);
    let (sum, count) = (0..grid.cell_count())
        .step_by(step)
        .fold((0.0, 0.0), |(s, n), k| {
            let p = grid.point(k);
            let q = displaced(p, lag / forcing.radius.0);
            (s + (octs.fbm(q) - octs.fbm(p)).abs(), n + 1.0)
        });
    let measured = sum / count;
    if measured <= 0.0 {
        return 0.0;
    }
    amplitude(forcing.analogue).0 * (lag / REFERENCE_WAVELENGTH.0).powf(h) / measured
}

/// The continents of a body: its height in metres per cell, and the mask of
/// the continental cells. Uses the seed `body.derive("relief.continents")`.
///
/// The noise is cut at the crust line, the quantile of the noise that
/// leaves [`continental_share`] of the area above it; a continental cell
/// stands on the Airy platform, [`floating_height`] of the thick crust,
/// reached from the coast at the slope [`CONTINENTAL_SLOPE`]; the sea floor
/// is the datum, 0. The noise ([`continent_noise`]) is added everywhere. A
/// dry world (no ocean) keeps only the noise, its plains.
pub fn continents(grid: &Grid, body: Seed, forcing: &Forcing) -> (Vec<f32>, Vec<bool>) {
    let noise = continent_noise(grid, body.derive("relief.continents"), forcing);
    let share = continental_share(forcing.analogue).get();
    let line = area_quantile(grid, &noise, 1.0 - share);
    let mask: Vec<bool> = noise.iter().map(|&v| v > line).collect();
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
    let gain = persistence(spectrum.hurst);
    let weights: Vec<f64> = (0..HIGHLANDS.octaves)
        .map(|i| gain.powi(i as i32))
        .collect();
    let total: f64 = weights.iter().sum();
    let power: f64 = weights.iter().map(|w| w * w).sum();
    let legacy = PERLIN_RMS * power.sqrt() / total;
    (0..grid.cell_count())
        .map(|k| highland_weight(octs.fbm(grid.point(k)) * legacy) as f32)
        .collect()
}

/// The mountains: ridged noise (crests only) times `weight` (the orogeny of
/// the plates, or [`highlands`] on a world without), times
/// [`MOUNTAIN_RELIEF`] and [`relief_scale`], in metres per cell. Uses
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
    let scale = MOUNTAIN_RELIEF.0 * relief_scale(forcing.gravity);
    (0..grid.cell_count())
        .map(|k| {
            if weight[k] <= 0.0 {
                return 0.0;
            }
            (octs.ridged(grid.point(k)).max(0.0) * f64::from(weight[k]) * scale) as f32
        })
        .collect()
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
