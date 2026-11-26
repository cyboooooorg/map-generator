//! The double-precision spectrum the tests hold the column's fast one to
//! (ported from `legacy-final`'s `planet/spectrum/reference.rs`): the same table, continua and pairs
//! as [`super::fast_level`], bin by bin and band by band in `f64`, the
//! black body of each bin by its exact integral, and the uniform air
//! above a column's top. The column's own reference sum
//! (`column::radiate_reference`) and the fixed-profile radiation checks
//! read it; the generator never does.

use super::*;
use crate::radiation::ktable::{CO2_SELF_SET, CO2_SET, Table, WATER_FOREIGN_SET, WATER_SELF_SET};

/// Hemispheric flux, W/m², that a black body at `t_k` emits in bin `i`:
/// `π ∫ B_ν dν` over the bin, exact (see [`planck_between`]).
pub(crate) fn planck_bin_w_m2(i: usize, t_k: f64) -> f64 {
    let (lo, hi) = bin_edges(i);
    planck_between(lo, hi, t_k)
}

/// Hemispheric black-body flux, W/m², at `t_k` between `lo` and `hi`
/// cm⁻¹: `σT⁴` times the share of the Planck function between
/// `x = c₂ν/T` at the two edges, `(15/π⁴) ∫ x³/(eˣ − 1) dx`.
pub(crate) fn planck_between(lo: f64, hi: f64, t_k: f64) -> f64 {
    if t_k <= 0.0 || hi <= lo {
        return 0.0;
    }
    let (a, b) = (
        SECOND_RADIATION_CONSTANT_CM_K * lo / t_k,
        SECOND_RADIATION_CONSTANT_CM_K * hi / t_k,
    );
    let integral = if a >= SERIES_SWITCH {
        planck_tail(a) - planck_tail(b)
    } else if b < SERIES_SWITCH {
        planck_head(b) - planck_head(a)
    } else {
        (PLANCK_TOTAL - planck_tail(b)) - planck_head(a)
    };
    STEFAN_BOLTZMANN * t_k.powi(4) * integral.max(0.0) / PLANCK_TOTAL
}

/// The absorption of one bin at one level, per unit `ln p` (vertical
/// optical depths): each gas's lines at its g-points, and what is smooth
/// across the bin.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct BinAbsorption {
    /// CO₂'s lines at each g-point.
    pub(crate) co2: [f64; G],
    /// Water's lines at each g-point.
    pub(crate) water: [f64; G],
    /// The continua of water and CO₂ and the collision-induced pairs.
    pub(crate) smooth: f64,
}

impl BinAbsorption {
    /// Mean transmission of the bin through `amount` times this
    /// absorption (an optical depth per `ln p` times a span of `ln p`),
    /// along the slant of [`DIFFUSIVITY`]: CO₂'s and water's
    /// transmissions, each summed over its g-points with [`weights`],
    /// multiply (random overlap), and the smooth part's with them.
    pub(crate) fn transmission(&self, amount: f64) -> f64 {
        let w = weights();
        let gas = |k: &[f64; G]| -> f64 {
            k.iter()
                .zip(w)
                .map(|(k, w)| w * (-DIFFUSIVITY * k * amount).exp())
                .sum()
        };
        gas(&self.co2) * gas(&self.water) * (-DIFFUSIVITY * self.smooth * amount).exp()
    }
}

/// Absorption per unit `ln p` (vertical optical depths) in bin `i` of
/// every absorber at `level`: CO₂'s and water's lines at their g-points,
/// and the continua of water and CO₂ and the collision-induced pairs (see
/// the module documentation).
pub(crate) fn absorption_per_ln_p(i: usize, level: &Level) -> BinAbsorption {
    OPACITY.bin_absorption(i, level)
}

/// Optical depth per `ln p` between `lo` and `hi` cm⁻¹ of what is smooth
/// across a bin: the continua of water and CO₂ and the collision-induced
/// pairs at the bin's centre.
fn smooth_between(lo: f64, hi: f64, level: &Level) -> f64 {
    let p = level.pressure_bar();
    let t = level.temperature_k;
    if p <= 0.0 || t <= 0.0 || level.gravity_m_s2 <= 0.0 {
        return 0.0;
    }
    let nu = 0.5 * (lo + hi);
    water_continuum(nu, level)
        + co2_continuum((lo / BIN_WIDTH_CM).round() as usize, level)
        + pairs_depth(nu, level)
}

/// Optical depth per `ln p` of every collision-induced pair at `nu`
/// cm⁻¹ at `level` ([`Pair::depth`] summed over [`PAIRS`]).
pub(crate) fn pairs_depth(nu: f64, level: &Level) -> f64 {
    PAIRS
        .iter()
        .enumerate()
        .filter(|(i, _)| absorbs(*i))
        .map(|(_, pair)| pair.depth(nu, level))
        .sum()
}

/// Optical depth per `ln p` of the water continuum at `nu` cm⁻¹:
/// [`water_continuum_depth`] with [`MT_CKD`]'s coefficients at `nu`
/// ([`mt_ckd_at`]).
fn water_continuum(nu: f64, level: &Level) -> f64 {
    if level.gases.h2o <= 0.0 || nu <= 0.0 {
        return 0.0;
    }
    water_continuum_depth(mt_ckd_at(nu), nu, level)
}

/// Optical depth per `ln p` of MT_CKD's CO₂ continuum in bin `i`:
/// [`co2_continuum_depth`] with the longwave table's coefficient at the
/// bin's centre.
fn co2_continuum(i: usize, level: &Level) -> f64 {
    if i >= BINS {
        return 0.0;
    }
    let table = &*OPACITY;
    co2_continuum_depth(
        table.co2_continuum[i],
        table.co2_continuum_t_eff_k,
        BIN_TABLE.centre_cm[i],
        level,
    )
}

/// The absorption of every band at one level, per unit `ln p` (see
/// [`BinAbsorption`]; the tail's is the last bin's, see [`TAIL`]).
#[derive(Clone, Debug)]
pub(crate) struct Spectrum {
    /// CO₂'s lines, per band and g-point.
    pub(crate) co2: [[f64; G]; BANDS],
    /// Water's lines, per band and g-point.
    pub(crate) water: [[f64; G]; BANDS],
    /// The continua of water and CO₂ and the collision-induced pairs, per
    /// band.
    pub(crate) smooth: [f64; BANDS],
}

impl Default for Spectrum {
    fn default() -> Self {
        Self {
            co2: [[0.0; G]; BANDS],
            water: [[0.0; G]; BANDS],
            smooth: [0.0; BANDS],
        }
    }
}

/// [`absorption_per_ln_p`] of every band at `level` into `out` (the
/// tail's is the last bin's, see [`TAIL`]): the same sums, with what does
/// not depend on the bin taken once per level.
pub(crate) fn absorption_spectrum(level: &Level, out: &mut Spectrum) {
    out.co2[..BINS].fill([0.0; G]);
    out.water[..BINS].fill([0.0; G]);
    out.smooth[..BINS].fill(0.0);
    let p = level.pressure_bar();
    let t = level.temperature_k;
    if p > 0.0 && t > 0.0 && level.gravity_m_s2 > 0.0 {
        lines_spectrum(level, out);
        smooth_spectrum(level, &mut out.smooth);
    }
    out.co2[TAIL] = out.co2[BINS - 1];
    out.water[TAIL] = out.water[BINS - 1];
    out.smooth[TAIL] = out.smooth[BINS - 1];
}

/// CO₂'s and water's lines of every bin at `level`, from [`OPACITY`] as
/// [`Table::bin_absorption`] takes them.
fn lines_spectrum(level: &Level, out: &mut Spectrum) {
    let table = &*OPACITY;
    let p = level.pressure_bar();
    let t = level.temperature_k;
    let g = &level.gases;
    let interpolate = |at: &[usize; 4], w: &[f64; 4], i: usize| {
        w[0] * f64::from(table.ln_k[at[0] + i])
            + w[1] * f64::from(table.ln_k[at[1] + i])
            + w[2] * f64::from(table.ln_k[at[2] + i])
            + w[3] * f64::from(table.ln_k[at[3] + i])
    };
    if g.co2 > 0.0 {
        let (by_air, wa, scale) = table.corners(CO2_SET, p, t);
        let (own, ws, _) = table.corners(CO2_SELF_SET, p, t);
        let mass = scale * level.mass_per_ln_p(g.co2, CO2_MOLAR_MASS);
        let share = (g.co2 / p).clamp(0.0, 1.0);
        let (air_mass, self_mass) = (mass * (1.0 - share), mass * share);
        for (b, row) in out.co2[..BINS].iter_mut().enumerate() {
            for (k, o) in row.iter_mut().enumerate() {
                let i = b * G + k;
                *o = air_mass * interpolate(&by_air, &wa, i).exp()
                    + self_mass * interpolate(&own, &ws, i).exp();
            }
        }
    }
    if g.h2o > 0.0 {
        let (foreign, wf, scale) = table.corners(WATER_FOREIGN_SET, p, t);
        let (own, ws, _) = table.corners(WATER_SELF_SET, p, t);
        let mass = scale * level.mass_per_ln_p(g.h2o, H2O_MOLAR_MASS);
        let share = (g.h2o / p).clamp(0.0, 1.0);
        let (by_air, by_self) = (mass * (1.0 - share), mass * share);
        for (b, row) in out.water[..BINS].iter_mut().enumerate() {
            for (k, o) in row.iter_mut().enumerate() {
                let i = b * G + k;
                *o = by_air * interpolate(&foreign, &wf, i).exp()
                    + by_self * interpolate(&own, &ws, i).exp();
            }
        }
    }
}

/// [`smooth_between`] of every bin at `level` into `out`.
fn smooth_spectrum(level: &Level, out: &mut [f64; BANDS]) {
    let rates = SmoothRates::of(level);
    for (i, o) in out[..BINS].iter_mut().enumerate() {
        *o = rates.bin(i);
    }
}

/// Vertical optical depth of each band above `level`, the top of a
/// column, through a uniform air at its temperature and composition
/// above it: half its rate per `ln p` ([`absorption_spectrum`], in
/// `rate`), since every absorber grows as `p²` there (lines broadened by
/// the pressure, the continua, the pairs).
pub(crate) fn depth_above(rate: &Spectrum, out: &mut Spectrum) {
    for i in 0..BANDS {
        out.co2[i] = rate.co2[i].map(|k| 0.5 * k);
        out.water[i] = rate.water[i].map(|k| 0.5 * k);
        out.smooth[i] = smooth_above(rate.smooth[i]);
    }
}

impl Table {
    /// Mass absorption coefficients, m²/kg, of `set` at `p_bar` and
    /// `t_k` in `bin`, times `factor`, into `out`.
    pub(crate) fn coefficients(
        &self,
        set: usize,
        p_bar: f64,
        t_k: f64,
        bin: usize,
        factor: f64,
        out: &mut [f64; G],
    ) {
        let (at, w, scale) = self.corners(set, p_bar, t_k);
        let base = bin * G;
        for (g, o) in out.iter_mut().enumerate() {
            let i = base + g;
            let ln_k = w[0] * f64::from(self.ln_k[at[0] + i])
                + w[1] * f64::from(self.ln_k[at[1] + i])
                + w[2] * f64::from(self.ln_k[at[2] + i])
                + w[3] * f64::from(self.ln_k[at[3] + i]);
            *o = factor * scale * ln_k.exp();
        }
    }

    /// The absorption of `bin` at `level` (see [`absorption_per_ln_p`]),
    /// its smooth part taken at the bin's centre.
    pub(crate) fn bin_absorption(&self, bin: usize, level: &Level) -> BinAbsorption {
        let mut out = BinAbsorption::default();
        let p = level.pressure_bar();
        let t = level.temperature_k;
        if p <= 0.0 || t <= 0.0 || level.gravity_m_s2 <= 0.0 || bin >= self.bins {
            return out;
        }
        let g = &level.gases;
        if g.co2 > 0.0 {
            let mass = level.mass_per_ln_p(g.co2, CO2_MOLAR_MASS);
            let share = (g.co2 / p).clamp(0.0, 1.0);
            let mut own = [0.0; G];
            self.coefficients(CO2_SET, p, t, bin, mass * (1.0 - share), &mut out.co2);
            self.coefficients(CO2_SELF_SET, p, t, bin, mass * share, &mut own);
            for (c, s) in out.co2.iter_mut().zip(own) {
                *c += s;
            }
        }
        if g.h2o > 0.0 {
            let mass = level.mass_per_ln_p(g.h2o, H2O_MOLAR_MASS);
            let share = (g.h2o / p).clamp(0.0, 1.0);
            let mut own = [0.0; G];
            self.coefficients(
                WATER_FOREIGN_SET,
                p,
                t,
                bin,
                mass * (1.0 - share),
                &mut out.water,
            );
            self.coefficients(WATER_SELF_SET, p, t, bin, mass * share, &mut own);
            for (w, s) in out.water.iter_mut().zip(own) {
                *w += s;
            }
        }
        let lo = bin as f64 * self.bin_width_cm;
        out.smooth = smooth_between(lo, lo + self.bin_width_cm, level);
        out
    }
}

impl super::SmoothRates {
    /// The smooth absorption per `ln p` of bin `i`.
    fn bin(&self, i: usize) -> f64 {
        let table = &*BIN_TABLE;
        let mut o = 0.0;
        if let Some((e, ln_ratio, others, scale)) = self.continuum {
            let (self_coeff, exponent, foreign_coeff) = table.continuum[i];
            let nu = table.centre_cm[i];
            let radiation = radiation_term(nu, self.t);
            o += (self_coeff * (exponent * ln_ratio).exp() * e + foreign_coeff * others)
                * scale
                * radiation;
        }
        for (pair, rates) in PAIRS.iter().zip(&self.pairs) {
            if let Some((column, scale)) = *rates
                && let Some(row) = pair.rows.get(i).copied().flatten()
            {
                o += pair.coefficient(row, column) * scale;
            }
        }
        if let Some((ln_t, scale)) = self.co2_continuum {
            let coefficient = OPACITY.co2_continuum[i];
            o += co2_continuum_per_molecule(coefficient, table.centre_cm[i], self.t, ln_t) * scale;
        }
        o
    }
}

/// Logarithmic mean of two opacities per `ln p`: the exact mean over a
/// layer where the opacity grows exponentially in `ln p` (a power of the
/// pressure, as through a uniform air), and the arithmetic mean's limit
/// when they are equal.
pub(crate) fn log_mean(a: f64, b: f64) -> f64 {
    if a <= 0.0 || b <= 0.0 {
        return 0.5 * (a + b);
    }
    let r = b / a;
    if (r - 1.0).abs() < 1e-6 {
        0.5 * (a + b)
    } else {
        (b - a) / r.ln()
    }
}
