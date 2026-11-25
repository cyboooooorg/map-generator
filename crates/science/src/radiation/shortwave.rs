//! The shortwave: the sunlight a column absorbs, reflects and lets
//! through (ported from `legacy-final`'s `planet/shortwave.rs`).
//!
//! Each band of the star's light crosses the column ([`Levels`], built
//! as [`super::column::Profile::SHORTWAVE`] says) in pieces cut at the
//! cloud tops ([`CLOUD_TOP_SHARE`] of the surface pressure): CO2's and
//! water's lines and continua absorb from 0.7 um to 5 um, Rayleigh
//! scattering is shared among the pieces by their mass, and each piece is
//! a two-stream layer ([`layer`]), stacked by adding ([`Stack`]).
//!
//! Within a band, CO2's and water's g-points overlap at random, as in
//! the longwave column (Lacis & Oinas 1991, *JGR* 96, 9027): each pair
//! of g-points is a stack of its own, weighted by the product of their
//! weights. The g-points of a gas whose whole column is thin (slant
//! depth under [`THIN`]) are merged into one first, which leaves their
//! mean transmission unchanged to the square of their depth.

use std::sync::LazyLock;

use super::column::{CLOUD_TOP_SHARE, Levels};
use super::fast_math;
use super::gases::{CO2_MOLAR_MASS, H2O_MOLAR_MASS};
use super::ktable::{CO2_SELF_SET, CO2_SET, G, Table, WATER_FOREIGN_SET, WATER_SELF_SET};
use super::spectrum::{self, DIFFUSIVITY, Level};
use super::starlight::{BANDS, band_edge_m};

#[cfg(test)]
#[path = "shortwave_tests.rs"]
mod tests;

/// The shortwave table of CO2's and water's lines
/// (`crates/science/data/shortwave.bin`, see [`super::ktable`]), with
/// MT_CKD 4.3's water and CO2 continua binned on its bins, checked
/// against the star's bands.
pub(crate) static SHORTWAVE: LazyLock<Table> = LazyLock::new(|| {
    let table = Table::parse(include_bytes!("../../data/shortwave.bin"))
        .expect("crates/science/data/shortwave.bin is a shortwave table of 8 g-points");
    assert!(
        !table.edges_cm.is_empty(),
        "shortwave.bin is a longwave table"
    );
    assert_eq!(table.water_continuum.len(), table.bins);
    let first = BANDS - table.bins;
    for (j, &edge) in table.edges_cm.iter().enumerate() {
        let lambda_m = band_edge_m(BANDS - j);
        assert!(
            (1e-2 / lambda_m / edge - 1.0).abs() < 1e-9,
            "the shortwave table's bins are not the star's bands"
        );
    }
    assert!(first > 0);
    table
});

/// Calibration: slant optical depth, along [`DIFFUSIVITY`], under which
/// a gas's whole column at a g-point counts as thin: its g-points that
/// thin are merged into one, their weighted mean depth, before the gases
/// are paired (module documentation). A numerical tolerance, not a
/// physical constant, with no source: merging changes their mean
/// transmission by at most half the square of their depth, 5e-3 of their
/// weight; on Earth's air the legacy measured that it moves the Bond
/// albedo by 2e-5 against a tolerance of 1e-2, and cuts the pairs of
/// g-points to stack by a third; measured on this port (under the table's
/// Sun, cloud albedo 0.296), Earth's all-sky Bond albedo is 0.292859
/// without merging and 0.292846 with it (1.3e-5).
const THIN: f64 = 0.1;

/// The absorption of one level of the column, per unit `ln p` (vertical
/// optical depths): each gas's lines at each bin and g-point, and the
/// continua at each bin.
struct LevelAbsorption {
    co2: Vec<f32>,
    water: Vec<f32>,
    smooth: Vec<f32>,
}

impl LevelAbsorption {
    /// The absorption of `level` in every bin of `table`.
    fn of(table: &Table, level: &Level) -> Self {
        let m = table.bins * G;
        let mut out = Self {
            co2: vec![0.0; m],
            water: vec![0.0; m],
            smooth: vec![0.0; table.bins],
        };
        let p = level.pressure_bar();
        let t = level.temperature_k;
        if p <= 0.0 || t <= 0.0 || level.gravity_m_s2 <= 0.0 {
            return out;
        }
        let mut by_air = vec![0.0f32; m];
        let mut by_self = vec![0.0f32; m];
        // Each gas's lines ([`Table::lines`]).
        let mut gas = |partial: f64, molar: f64, sets: (usize, usize), out: &mut [f32]| {
            if partial > 0.0 {
                table.lines(
                    level,
                    partial,
                    molar,
                    sets,
                    m,
                    &mut by_air,
                    &mut by_self,
                    out,
                );
            }
        };
        let g = level.gases;
        gas(g.co2, CO2_MOLAR_MASS, (CO2_SET, CO2_SELF_SET), &mut out.co2);
        gas(
            g.h2o,
            H2O_MOLAR_MASS,
            (WATER_FOREIGN_SET, WATER_SELF_SET),
            &mut out.water,
        );
        for (b, smooth) in out.smooth.iter_mut().enumerate() {
            *smooth = (water_continuum(table, b, level)
                + co2_continuum(table, b, level)
                + co2_pairs(table, b, level)) as f32;
        }
        out
    }
}

/// Optical depth per `ln p` of MT_CKD 4.3's water continuum in bin `b`
/// at `level`: the longwave's ([`spectrum::water_continuum_depth`]) with
/// the bin's binned coefficients at its centre.
fn water_continuum(table: &Table, b: usize, level: &Level) -> f64 {
    spectrum::water_continuum_depth(table.water_continuum[b], centre_cm(table, b), level)
}

/// Optical depth per `ln p` of MT_CKD 4.3's CO2 continuum in bin `b` at
/// `level`: the longwave's ([`spectrum::co2_continuum_depth`]) with the
/// bin's binned coefficient at its centre; `contnm.f90` gives it to
/// 10 000 cm-1.
fn co2_continuum(table: &Table, b: usize, level: &Level) -> f64 {
    spectrum::co2_continuum_depth(
        table.co2_continuum[b],
        table.co2_continuum_t_eff_k,
        centre_cm(table, b),
        level,
    )
}

/// Centre of bin `b` of `table`, cm-1.
fn centre_cm(table: &Table, b: usize) -> f64 {
    0.5 * (table.edges_cm[b] + table.edges_cm[b + 1])
}

/// Optical depth per `ln p` of CO2-CO2 collision-induced absorption in
/// bin `b` at `level`: the mean over the bin of the longwave's
/// ([`spectrum::co2_pairs`], binned from HITRAN's `CO2-CO2_2024.cia`,
/// Karman et al. 2019, *Icarus* 328, 160,
/// doi:10.1016/j.icarus.2019.02.034), taken every
/// [`spectrum::CO2_CIA_CELL_CM`] across it. It grows as the CO2 density
/// squared and is not broadened. Inside the shortwave table's range
/// the file lists CO2-CO2 bands at 2510-3250 cm-1 (3.1-4.0 um) and
/// 3995-4500 cm-1 (the 2.3 um band); it holds none at 1.73 um (5800
/// cm-1) or 1.18 um, so they are left out.
fn co2_pairs(table: &Table, b: usize, level: &Level) -> f64 {
    if level.gases.co2 <= 0.0 {
        return 0.0;
    }
    let (lo, hi) = (table.edges_cm[b], table.edges_cm[b + 1]);
    if lo >= spectrum::CO2_CIA_TOP_CM {
        return 0.0;
    }
    let cell = spectrum::CO2_CIA_CELL_CM;
    let n = ((hi - lo) / cell).ceil().max(1.0) as usize;
    let step = (hi - lo) / n as f64;
    (0..n)
        .map(|i| spectrum::co2_pairs(lo + (i as f64 + 0.5) * step, level))
        .sum::<f64>()
        / n as f64
}

/// Reflectance and transmission of a homogeneous layer of absorption
/// optical depth `absorption` and Rayleigh scattering optical depth
/// `scattering` (vertical), under diffuse light: the two-stream solution
/// of Meador & Weaver 1980 (*J. Atmos. Sci.* 37, 630,
/// doi:10.1175/1520-0469(1980)037<0630:TSATRT>2.0.CO;2, eqs. 18, 25 and
/// 26, quoted from the ECMWF ecRad code that implements them; the article
/// itself was not opened), `R = γ₂ sinh(λτ) / (λ cosh(λτ) + γ₁ sinh(λτ))`,
/// `T = λ / (λ cosh(λτ) + γ₁ sinh(λτ))`, `λ = √(γ₁² − γ₂²)`, with the
/// hemispheric-mean coefficients of Toon et al. 1989 (*JGR* 94, 16287,
/// doi:10.1029/JD094iD13p16287, Table 1; checked through Heng et al.
/// 2014, arXiv:1405.0026, eq. 67, and ecRad; the table itself not read) at the column's
/// diffusivity `D` = 1.66 in place of their 2: `γ₁ = D (1 − ω/2)`,
/// `γ₂ = D ω/2`, `ω` the single scattering albedo; Rayleigh's phase
/// function scatters half of the light back (`g` = 0). The same `D` as
/// the longwave column's ([`DIFFUSIVITY`], Elsasser's): a pure absorber
/// transmits `e^{−Dτ}`, and a pure scatterer reflects
/// `(D/2) τ / (1 + (D/2) τ)` (see `docs/science/radiative-transfer.md`,
/// "The shortwave half").
pub(crate) fn layer(absorption: f64, scattering: f64) -> (f64, f64) {
    let tau = absorption.max(0.0) + scattering.max(0.0);
    if tau <= 0.0 {
        return (0.0, 1.0);
    }
    let d = DIFFUSIVITY;
    let omega = scattering.max(0.0) / tau;
    let (g1, g2) = (d * (1.0 - 0.5 * omega), d * 0.5 * omega);
    // λ = D √(1 − ω), from the absorption so as not to lose it to
    // rounding when the scattering dominates.
    let lambda = d * (absorption.max(0.0) / tau).sqrt();
    let x = lambda * tau;
    // cosh and sinh / λ, both times 2 e^{−x}, so that a thick layer does
    // not overflow.
    let e = (-x).exp();
    let cosh = 0.5 * (1.0 + e * e);
    let sinh_over = if x < 1e-8 {
        tau
    } else if x < 1e-2 {
        -(-2.0 * x).exp_m1() / (2.0 * lambda)
    } else {
        (1.0 - e * e) / (2.0 * lambda)
    };
    let denominator = cosh + g1 * sinh_over;
    (g2 * sinh_over / denominator, e / denominator)
}

/// [`layer`] in single precision with [`fast_math::exp`], for the many
/// layers of the shortwave paths (within 1e-6 of it).
#[inline]
pub(crate) fn layer_fast(absorption: f32, scattering: f32) -> (f32, f32) {
    let tau = absorption.max(0.0) + scattering.max(0.0);
    if tau <= 0.0 {
        return (0.0, 1.0);
    }
    let d = DIFFUSIVITY as f32;
    let omega = scattering.max(0.0) / tau;
    let (g1, g2) = (d * (1.0 - 0.5 * omega), d * 0.5 * omega);
    let lambda = d * (absorption.max(0.0) / tau).sqrt();
    let x = lambda * tau;
    let e = fast_math::exp(-x);
    let cosh = 0.5 * (1.0 + e * e);
    // `1 − e^{−2x}` loses its digits for a thin layer: its series there.
    let sinh_over = if x < 1e-2 {
        tau * (1.0 - x * (1.0 - x * (2.0 / 3.0)))
    } else {
        (1.0 - e * e) / (2.0 * lambda)
    };
    let denominator = cosh + g1 * sinh_over;
    (g2 * sinh_over / denominator, e / denominator)
}

/// The layers above the ground, from the top down, stacked: their
/// reflectance to the light from above (`down`) and from below (`up`),
/// and their transmission.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Stack {
    pub(crate) down: f64,
    pub(crate) up: f64,
    pub(crate) through: f64,
}

impl Stack {
    /// Definition: nothing, a stack that reflects nothing and lets all
    /// the light through.
    pub(crate) const EMPTY: Stack = Stack {
        down: 0.0,
        up: 0.0,
        through: 1.0,
    };

    /// The stack with a symmetric layer of reflectance `r` and
    /// transmission `t` under it, the light reflected back and forth
    /// between them.
    pub(crate) fn over(self, r: f64, t: f64) -> Stack {
        let bounce = 1.0 / (1.0 - self.up * r);
        Stack {
            down: self.down + self.through * self.through * r * bounce,
            up: r + t * t * self.up * bounce,
            through: self.through * t * bounce,
        }
    }

    /// The stack with the stack `below` under it (the adding of two
    /// stacks; each transmits the same both ways).
    pub(crate) fn then(self, below: Stack) -> Stack {
        let bounce = 1.0 / (1.0 - self.up * below.down);
        Stack {
            down: self.down + self.through * self.through * below.down * bounce,
            up: below.up + below.through * below.through * self.up * bounce,
            through: self.through * below.through * bounce,
        }
    }

    /// Over a ground of albedo `ground`: the share of the light reflected
    /// to space, and the share the ground absorbs.
    pub(crate) fn on(self, ground: f64) -> (f64, f64) {
        let bounce = 1.0 / (1.0 - self.up * ground);
        (
            self.down + self.through * self.through * ground * bounce,
            self.through * (1.0 - ground) * bounce,
        )
    }
}

/// Where the star's light goes, as shares of it: reflected to space (the
/// Bond albedo of the sky over the ground) and absorbed by the air (its
/// gases and haze); the rest is absorbed by the ground.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Budget {
    pub(crate) albedo: f64,
    pub(crate) air: f64,
}

/// One piece of the column between two of its levels (or a part of one,
/// cut at the cloud tops): its levels from the bottom, the share of the
/// span between them it covers (from `from` to `to`, 0 at the lower
/// level), its span in `ln p`, and the share of the air's mass in it.
#[derive(Clone, Copy, Debug)]
struct Piece {
    lower: usize,
    from: f64,
    to: f64,
    span: f64,
    mass: f64,
}

/// The pieces of `levels` from the top down, the column cut at the cloud
/// tops, and the index of the first piece below the clouds.
fn pieces(levels: &Levels) -> (Vec<Piece>, usize) {
    let n = levels.spans.len();
    let surface = levels.levels[0].pressure_bar();
    let cloud_up = -CLOUD_TOP_SHARE.ln();
    let mut out = Vec::with_capacity(n + 1);
    let mut below_cloud = 0;
    let mut up = 0.0;
    for (i, &span) in levels.spans.iter().enumerate() {
        let cuts: &[(f64, f64)] = if up <= cloud_up && cloud_up < up + span {
            let x = (cloud_up - up) / span;
            &[(0.0, x), (x, 1.0)]
        } else {
            &[(0.0, 1.0)]
        };
        for &(from, to) in cuts {
            let p = |share: f64| surface * (-(up + share * span)).exp();
            // The air above the top weighs into the top piece.
            let top = if i + 1 == n && to == 1.0 { 0.0 } else { p(to) };
            out.push(Piece {
                lower: i,
                from,
                to,
                span: (to - from) * span,
                mass: (p(from) - top) / surface,
            });
            if up + to * span <= cloud_up + 1e-12 {
                below_cloud = out.len();
            }
        }
        up += span;
    }
    out.reverse();
    let cloud_at = out.len() - below_cloud;
    (out, cloud_at)
}

/// Vertical optical depth of `piece` for a rate per `ln p` of `lower` at
/// its lower level and `upper` at its upper level, taken exponential in
/// `ln p` between them: the tests' double-precision depth, which
/// [`piece_depth_fast`] is held to.
#[cfg(test)]
fn piece_depth(piece: &Piece, lower: f64, upper: f64) -> f64 {
    if lower <= 0.0 || upper <= 0.0 {
        let at = |x: f64| lower + (upper - lower) * x;
        return piece.span * 0.5 * (at(piece.from) + at(piece.to));
    }
    let at = |x: f64| {
        if x == 0.0 {
            lower
        } else if x == 1.0 {
            upper
        } else {
            lower * (upper / lower).powf(x)
        }
    };
    piece.span * spectrum::reference::log_mean(at(piece.from), at(piece.to))
}

/// `piece_depth` (the tests' double-precision depth) in single precision
/// with [`fast_math`]'s `exp` and `ln`, for the many depths of
/// [`Paths::through`].
#[inline]
fn piece_depth_fast(piece: &Piece, lower: f32, upper: f32) -> f64 {
    let span = piece.span as f32;
    if lower <= 0.0 || upper <= 0.0 {
        let at = |x: f32| lower + (upper - lower) * x;
        return f64::from(span * 0.5 * (at(piece.from as f32) + at(piece.to as f32)));
    }
    let ln_ratio = fast_math::ln(upper) - fast_math::ln(lower);
    let at = |x: f64| {
        if x == 0.0 {
            lower
        } else if x == 1.0 {
            upper
        } else {
            lower * fast_math::exp(ln_ratio * x as f32)
        }
    };
    let (a, b) = (at(piece.from), at(piece.to));
    let ln_ab = ln_ratio * (piece.to - piece.from) as f32;
    let mean = if ln_ab.abs() < 1e-4 {
        0.5 * (a + b)
    } else {
        (b - a) / ln_ab
    };
    f64::from(span * mean)
}

/// What the air alone sets of the light's way: its methane haze (its
/// reflectance and transmission), its Rayleigh depth at 550 nm and its
/// column ([`super::column::levels`], `None` without air); the
/// ground and the clouds under it are the cover's ([`Paths::budget`]).
pub(crate) struct Air<'a> {
    pub(crate) haze: (f64, f64),
    pub(crate) rayleigh_550: f64,
    pub(crate) column: Option<&'a Levels>,
}

/// The light's paths through an air ([`Air`]), whatever the ground and
/// the clouds under it: for each band and each pair of g-points, its
/// weight in the light and the stacks of the air above the cloud tops and
/// below them. A given air gives given paths, so that the climate, which
/// asks for the albedo of the same air under covers that change with its
/// temperature, solves the air once ([`Paths::budget`] then costs a few
/// operations per path).
#[derive(Clone, Debug)]
pub(crate) struct Paths {
    paths: Vec<(f64, Stack, Stack)>,
}

impl Paths {
    /// The paths of `bands` (each band's `(550 nm / lambda)^4` and its
    /// share of the light, bluest first, as
    /// [`super::starlight::StarBands`] holds them) through `air`.
    pub(crate) fn through(air: &Air, bands: &[(f64, f64)]) -> Paths {
        let (hr, ht) = air.haze;
        let top = Stack::EMPTY.over(hr, ht);
        let column = air.column.filter(|c| !c.spans.is_empty());
        let Some(levels) = column else {
            // No air: the Rayleigh layer (none, or a trace) over the clouds.
            let paths = bands
                .iter()
                .map(|&(scale, weight)| {
                    let (r, t) = layer(0.0, air.rayleigh_550 * scale);
                    (weight, top.over(r, t), Stack::EMPTY)
                })
                .collect();
            return Paths { paths };
        };
        let table = &*SHORTWAVE;
        let (pieces, cloud_at) = pieces(levels);
        let absorption: Vec<LevelAbsorption> = levels
            .levels
            .iter()
            .map(|level| LevelAbsorption::of(table, level))
            .collect();
        let first_absorbed = bands.len() - table.bins;
        let mut co2 = Points::new(pieces.len());
        let mut water = Points::new(pieces.len());
        let mut smooth = vec![0.0; pieces.len()];
        let stacks = |depths: &dyn Fn(usize) -> f64, scale: f64| -> (Stack, Stack) {
            let (mut above, mut below) = (top, Stack::EMPTY);
            for (i, piece) in pieces.iter().enumerate() {
                let (r, t) = layer_fast(
                    depths(i) as f32,
                    (air.rayleigh_550 * scale * piece.mass) as f32,
                );
                let (r, t) = (f64::from(r), f64::from(t));
                if i < cloud_at {
                    above = above.over(r, t);
                } else {
                    below = below.over(r, t);
                }
            }
            (above, below)
        };
        let mut paths = Vec::with_capacity(bands.len() * 4);
        for (i, &(scale, weight)) in bands.iter().enumerate() {
            if i < first_absorbed {
                let (above, below) = stacks(&|_| 0.0, scale);
                paths.push((weight, above, below));
                continue;
            }
            let bin = bands.len() - 1 - i;
            for (k, piece) in pieces.iter().enumerate() {
                let (lower, upper) = (&absorption[piece.lower], &absorption[piece.lower + 1]);
                smooth[k] = piece_depth_fast(piece, lower.smooth[bin], upper.smooth[bin]);
                for g in 0..G {
                    let at = bin * G + g;
                    co2.depth[g][k] = piece_depth_fast(piece, lower.co2[at], upper.co2[at]);
                    water.depth[g][k] = piece_depth_fast(piece, lower.water[at], upper.water[at]);
                }
            }
            co2.merge(&table.weights);
            water.merge(&table.weights);
            for (wc, dc) in co2.points() {
                for (ww, dw) in water.points() {
                    let (above, below) = stacks(&|k| dc[k] + dw[k] + smooth[k], scale);
                    paths.push((weight * wc * ww, above, below));
                }
            }
        }
        Paths { paths }
    }

    /// The [`Budget`] of these paths over a ground of albedo `ground`,
    /// with the water clouds' layer of reflectance `cloud` at the cloud
    /// tops.
    pub(crate) fn budget(&self, ground: f64, cloud: f64) -> Budget {
        let cloud = cloud.clamp(0.0, 1.0);
        let mut out = Budget {
            albedo: 0.0,
            air: 0.0,
        };
        for (weight, above, below) in &self.paths {
            let stack = above.over(cloud, 1.0 - cloud).then(*below);
            let (a, ground) = stack.on(ground);
            out.albedo += weight * a;
            out.air += weight * (1.0 - a - ground);
        }
        out
    }
}

/// A gas's g-points in one band: each one's vertical depth in each piece
/// of the column, and, once merged ([`Points::merge`]), the points kept
/// with their weights.
struct Points {
    /// Depth of each g-point in each piece.
    depth: [Vec<f64>; G],
    /// The points kept: their weights and depths per piece (the first
    /// `count` of them; the rest are buffers).
    kept: Vec<(f64, Vec<f64>)>,
    count: usize,
}

impl Points {
    fn new(pieces: usize) -> Self {
        Self {
            depth: std::array::from_fn(|_| vec![0.0; pieces]),
            kept: (0..=G).map(|_| (0.0, vec![0.0; pieces])).collect(),
            count: 0,
        }
    }

    /// Keeps each g-point whose whole column is thicker than [`THIN`]
    /// along the slant, and merges the others into one, of their total
    /// weight and their weighted mean depth in each piece.
    fn merge(&mut self, weights: &[f64; G]) {
        self.count = 0;
        let mut thin_weight = 0.0;
        let thin_at = G;
        self.kept[thin_at].1.iter_mut().for_each(|d| *d = 0.0);
        for (g, &w) in weights.iter().enumerate() {
            let column: f64 = self.depth[g].iter().sum();
            if DIFFUSIVITY * column > THIN {
                let (kw, kd) = &mut self.kept[self.count];
                *kw = w;
                kd.copy_from_slice(&self.depth[g]);
                self.count += 1;
            } else {
                thin_weight += w;
                for (t, d) in self.kept[thin_at].1.iter_mut().zip(&self.depth[g]) {
                    *t += w * d;
                }
            }
        }
        if thin_weight > 0.0 {
            self.kept[thin_at]
                .1
                .iter_mut()
                .for_each(|d| *d /= thin_weight);
            self.kept[thin_at].0 = thin_weight;
            self.kept.swap(self.count, thin_at);
            self.count += 1;
        }
    }

    /// The points kept, each with its weight and depths.
    fn points(&self) -> impl Iterator<Item = (f64, &[f64])> {
        self.kept[..self.count]
            .iter()
            .map(|(w, d)| (*w, d.as_slice()))
    }
}
