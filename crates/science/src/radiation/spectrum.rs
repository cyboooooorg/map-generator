//! The thermal infrared, bin by bin: how much each gas absorbs in each
//! bin of wavenumber, and how much a black body emits there (ported from
//! `legacy-final`'s `planet/spectrum.rs`, the longwave band model of
//! issue #523).
//!
//! One optical depth for the whole spectrum would close it at once, while CO₂ and
//! water absorb in bands: the opaque part of CO₂'s 15 µm band widens with
//! the logarithm of its amount, which is why its forcing is logarithmic
//! (Romps, Seeley & Edman 2022, *J. Climate* 35, 4027,
//! doi:10.1175/JCLI-D-21-0275.1), and water leaves a window open between
//! its rotation band and its vibration–rotation band, where only its
//! continuum absorbs. This module holds the absorption of each gas, per
//! bin and per unit `ln p` at one level of a column, and the Planck flux
//! of each bin; the column sums the two-stream emission bin by bin with
//! [`DIFFUSIVITY`].
//!
//! The absorbers, each with its source:
//!
//! - **CO₂ and water lines:** a correlated-k table built offline from the
//!   HITRAN2024 line lists (Gordon et al. 2026, *JQSRT* 109807; the
//!   design named HITRAN2020, which HITRAN no longer serves) by
//!   `tools/opacity-table` (provenance in `crates/science/data/OPACITY.md`).
//!   Within a bin the lines' coefficients are sorted into [`G`] g-points,
//!   each with a weight ([`weights`]), on a grid of pressure and
//!   temperature; each gas has a set broadened by air and one broadened by
//!   itself (HITRAN's `γ_self`), mixed by its share of the pressure. The
//!   table holds the lines' temperature dependence (strengths, hot bands,
//!   widths) and their Voigt shape out to 25 cm⁻¹, less the pedestal
//!   MT_CKD's continua assume ([`Table`]).
//! - **CO₂ continuum:** MT_CKD 4.3's, the far wings of CO₂'s lines beyond
//!   25 cm⁻¹ with their sub-Lorentzian shape, binned into the table
//!   ([`co2_continuum_depth`]); it grows as the CO₂ times the pressure.
//! - **Water continuum:** MT_CKD 4.3's self and foreign continua (Mlawer
//!   et al. 2012), the self part growing as the vapour pressure squared
//!   and the foreign part as the vapour times the other gases
//!   ([`MT_CKD`]); it assumes the 25 cm⁻¹ line cutoff the table uses.
//! - **CO₂–CO₂ collision-induced absorption:** HITRAN2024's CIA file,
//!   documented by Terragni et al. (HITRAN2024 CIA), binned
//!   ([`CO2_CIA`]), growing as the CO₂ density squared, not broadened.
//! - **Methane:** its grey stand-in for Titan's collision-induced
//!   absorption ([`super::methane::depth`]), over
//!   0-[`METHANE_TOP_CM`] cm⁻¹ only.
//!
//! The continua, the pairs and methane are smooth across a bin: each is
//! one optical depth per bin, added to every g-point ([`FastLevel`]). The
//! double-precision sums the tests hold the fast ones to are in
//! `reference`.
//! CO₂ and water overlap at random within a bin (Lacis & Oinas 1991,
//! *JGR* 96, 9027, doi:10.1029/90JD01945, the correlated-k method and its
//! random overlap; the table's eight g-intervals are its builder's
//! calibration, see `OPACITY.md`): the transmission of a path is the product of each
//! gas's transmission, each the weighted sum over its g-points. The
//! column masses come from `p / g` with the body's gravity
//! ([`Level::gravity_m_s2`]).

use std::sync::LazyLock;

use super::gases::{
    CH4_MOLAR_MASS, CO2_MOLAR_MASS, H2O_MOLAR_MASS, N2_MOLAR_MASS, NH3_MOLAR_MASS, O2_MOLAR_MASS,
    PartialPressures, SO2_MOLAR_MASS,
};
pub(crate) use super::ktable::G;
use super::ktable::{CO2_SELF_SET, CO2_SET, Table, WATER_FOREIGN_SET, WATER_SELF_SET};
use super::methane;
use crate::constants::{
    AVOGADRO, BOLTZMANN, PASCALS_PER_BAR, SECOND_RADIATION_CONSTANT_CM_K, STEFAN_BOLTZMANN,
};

#[cfg(test)]
#[path = "spectrum_reference.rs"]
pub(crate) mod reference;

#[cfg(test)]
#[path = "spectrum_tests.rs"]
mod tests;

/// Definition: width of a spectral bin, cm⁻¹, the bin width of
/// `opacity.bin` (`OPACITY.md`, Grid). A numerical choice with no source,
/// held by a convergence test: halving it moves the CO₂ doubling forcing
/// of an Earth column by 0.006 W/m² in the legacy (the test
/// `halving_bins_keeps_the_forcing`, which allows 0.1).
pub(crate) const BIN_WIDTH_CM: f64 = 20.0;

/// Definition: upper end of the spectrum, cm⁻¹, the top of
/// `opacity.bin`: a black body at 640 K, the hottest steam column the
/// legacy climate solved, emits 99.6 % of its flux below it (the test
/// `planck_bins_sum_to_sigma_t4`); the rest goes to the tail band
/// ([`TAIL`]).
pub(crate) const SPECTRUM_TOP_CM: f64 = 5000.0;

/// Definition: number of bins over 0-[`SPECTRUM_TOP_CM`].
pub(crate) const BINS: usize = (SPECTRUM_TOP_CM / BIN_WIDTH_CM) as usize;

/// Source: diffusivity factor of the two-stream emission, `1/cos θ̄`,
/// Elsasser's 1.66 (Elsasser 1942, *Heat transfer by infrared radiation
/// in the atmosphere*, Harvard Meteorological Studies 6; the book was not
/// read, the value is the textbook one). Koll, Jeevanjee & Lutsko 2023
/// (*J. Atmos. Sci.* 80, 1923, doi:10.1175/JAS-D-22-0178.1, Table 1,
/// read in the accepted manuscript) take the inverse
/// diffusivity `3/5`, a factor 1.667, 0.4 % apart. Each bin's optical
/// depth from [`fast_level`] is vertical; the column multiplies it by
/// this.
pub(crate) const DIFFUSIVITY: f64 = 1.66;

/// One level of a column, as the spectrum reads it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Level {
    /// Temperature, K.
    pub temperature_k: f64,
    /// Partial pressures at the level, bar; their sum is its pressure.
    pub gases: PartialPressures,
    /// Surface gravity of the body, m/s²: a column of `p` bar weighs
    /// `p / g` per unit area.
    pub gravity_m_s2: f64,
}

impl Level {
    /// Pressure, bar: the sum of the partial pressures.
    pub(crate) fn pressure_bar(&self) -> f64 {
        self.gases.total()
    }

    /// Mean molar mass of the air, kg/mol.
    pub(crate) fn molar_mass_kg(&self) -> f64 {
        let g = &self.gases;
        let p = self.pressure_bar();
        if p <= 0.0 {
            return N2_MOLAR_MASS;
        }
        (g.n2.max(0.0) * N2_MOLAR_MASS
            + g.co2.max(0.0) * CO2_MOLAR_MASS
            + g.h2o.max(0.0) * H2O_MOLAR_MASS
            + g.ch4.max(0.0) * CH4_MOLAR_MASS
            + g.nh3.max(0.0) * NH3_MOLAR_MASS
            + g.so2.max(0.0) * SO2_MOLAR_MASS
            + g.o2.max(0.0) * O2_MOLAR_MASS)
            / p
    }

    /// Mass of a gas of molar mass `molar_kg` and partial pressure
    /// `partial_bar`, kg/m², per unit `ln p` of the column at this level:
    /// `(p_i M_i / (p M̄)) p / g`, its mass share of the air above.
    pub(crate) fn mass_per_ln_p(&self, partial_bar: f64, molar_kg: f64) -> f64 {
        partial_bar.max(0.0) * PASCALS_PER_BAR * molar_kg
            / (self.molar_mass_kg() * self.gravity_m_s2)
    }

    /// Molecules per m² per unit `ln p` of a gas of partial pressure
    /// `partial_bar` at this level, `p_i N_A / (M̄ g)`.
    pub(crate) fn molecules_per_ln_p(&self, partial_bar: f64) -> f64 {
        partial_bar * PASCALS_PER_BAR * AVOGADRO / (self.molar_mass_kg() * self.gravity_m_s2)
    }

    /// Number density, per m³, of a gas of partial pressure `partial_bar`
    /// at this level's temperature.
    fn density_m3(&self, partial_bar: f64) -> f64 {
        partial_bar * PASCALS_PER_BAR / (BOLTZMANN * self.temperature_k)
    }

    /// Thickness of the air per unit `ln p` at this level, m:
    /// `k_B T N_A / (M̄ g)`.
    fn thickness_per_ln_p_m(&self) -> f64 {
        BOLTZMANN * self.temperature_k * AVOGADRO / (self.molar_mass_kg() * self.gravity_m_s2)
    }
}

/// Edges of bin `i`, cm⁻¹.
pub(crate) fn bin_edges(i: usize) -> (f64, f64) {
    edges_of(BIN_WIDTH_CM, i)
}

/// Edges of bin `i` on bins `width` cm⁻¹ wide.
fn edges_of(width: f64, i: usize) -> (f64, f64) {
    (i as f64 * width, (i + 1) as f64 * width)
}

/// Definition: `∫₀^∞ x³/(eˣ − 1) dx = π⁴/15`.
const PLANCK_TOTAL: f64 =
    std::f64::consts::PI * std::f64::consts::PI * std::f64::consts::PI * std::f64::consts::PI
        / 15.0;

/// Definition: `x` under which the Planck integral is summed from 0
/// ([`planck_head`]) and over which from infinity ([`planck_tail`]), a
/// numerical switch: at 2 the head's last term is under 1e-10 of its sum.
const SERIES_SWITCH: f64 = 2.0;

/// Source: the Bernoulli numbers `B₂ … B₂₀` (Abramowitz & Stegun 1964,
/// *Handbook of Mathematical Functions*, Table 23.2).
const BERNOULLI: [f64; 10] = [
    1.0 / 6.0,
    -1.0 / 30.0,
    1.0 / 42.0,
    -1.0 / 30.0,
    5.0 / 66.0,
    -691.0 / 2730.0,
    7.0 / 6.0,
    -3617.0 / 510.0,
    43867.0 / 798.0,
    -174_611.0 / 330.0,
];

/// `∫₀^x t³/(eᵗ − 1) dt` for `x` < [`SERIES_SWITCH`]: the series
/// `Σ Bₙ x^(n+3) / (n! (n+3))` of `t/(eᵗ − 1)`, which converges for
/// `x < 2π`; at `x` = 2 its last term is under 1e-10 of the sum.
fn planck_head(x: f64) -> f64 {
    let x = x.max(0.0);
    let mut sum = x * x * x / 3.0 - x.powi(4) / 8.0;
    let mut power = x.powi(3);
    let mut factorial = 1.0;
    for (k, b) in BERNOULLI.iter().enumerate() {
        let n = 2 * (k + 1);
        factorial *= ((n - 1) * n) as f64;
        power *= x * x;
        sum += b * power / (factorial * (n + 3) as f64);
    }
    sum
}

/// Definition: `1/n` for `n` from 1 to 64, the terms of [`planck_tail`].
const INVERSES: [f64; 64] = {
    let mut out = [0.0; 64];
    let mut n = 0;
    while n < 64 {
        out[n] = 1.0 / (n + 1) as f64;
        n += 1;
    }
    out
};

/// `∫ₓ^∞ t³/(eᵗ − 1) dt` for `x` ≥ [`SERIES_SWITCH`]:
/// `Σₙ e^(−nx) (x³/n + 3x²/n² + 6x/n³ + 6/n⁴)`, summed until its terms
/// fall under 1e-16 of the first.
fn planck_tail(x: f64) -> f64 {
    let mut sum = 0.0;
    // `e^(−nx)` as the powers of `e^(−x)`: one exponential per edge.
    let first = (-x).exp();
    let mut e = 1.0;
    for inverse in &INVERSES {
        e *= first;
        let (i1, i2) = (*inverse, inverse * inverse);
        let term = e * (x * x * x * i1 + 3.0 * x * x * i2 + 6.0 * x * i2 * i1 + 6.0 * i2 * i2);
        sum += term;
        if term <= 1e-16 * sum {
            break;
        }
    }
    sum
}

/// The longwave table of CO₂'s and water's lines
/// (`crates/science/data/opacity.bin`, see [`super::ktable`]), on the
/// model's bins.
pub(crate) static OPACITY: LazyLock<Table> = LazyLock::new(|| {
    let table = Table::parse(include_bytes!("../../data/opacity.bin"))
        .expect("crates/science/data/opacity.bin is a table of 8 g-points");
    assert!(
        table.edges_cm.is_empty() && table.bins == BINS && table.bin_width_cm == BIN_WIDTH_CM,
        "the opacity table's bins are not the model's"
    );
    table
});

/// Weight of each g-point: the share of its bin's wavenumbers it stands
/// for; they sum to 1.
pub(crate) fn weights() -> &'static [f64; G] {
    &OPACITY.weights
}

/// Definition: spacing of [`MT_CKD`]'s nodes, cm⁻¹, the rows of
/// `mt-ckd-water.tsv`.
const MT_CKD_STEP_CM: f64 = 50.0;

/// Source: reference temperature of [`MT_CKD`], K, the file's `ref_temp`
/// (MT_CKD 4.3, `absco-ref_wv-mt-ckd.nc`; LBLRTM `contnm.f90` `T0=296.`).
pub(crate) const MT_CKD_REFERENCE_K: f64 = 296.0;

/// Source: reference pressure of [`MT_CKD`], bar, the file's `ref_press`
/// of 1013 mbar (MT_CKD 4.3, `absco-ref_wv-mt-ckd.nc`; LBLRTM
/// `contnm.f90` `P0=1013.`).
pub(crate) const MT_CKD_REFERENCE_BAR: f64 = 1.013;

/// Source: the water vapour continuum of MT_CKD 4.3 (Mlawer et al. 2012, *Phil.
/// Trans. R. Soc. A* 370, 2520, doi:10.1098/rsta.2011.0295; file
/// `absco-ref_wv-mt-ckd.nc` of github.com/AER-RC/MT_CKD_H2O, tag 4.3,
/// SHA-256 in `OPACITY.md`), every [`MT_CKD_STEP_CM`] cm⁻¹ from 0 to
/// 5000, read from `crates/science/data/mt-ckd-water.tsv`: wavenumber
/// (cm⁻¹), the self continuum coefficient at 296 K, its temperature
/// exponent, and the foreign continuum coefficient at 296 K (cm²/molecule
/// per cm⁻¹; the file's points at those wavenumbers, not bin means).
/// Between nodes the coefficients are interpolated in their logarithm
/// ([`mt_ckd_at`]). The optical depth they give is
/// [`water_continuum_depth`]'s. MT_CKD holds from cold air to steam; its
/// own validity is checked against laboratory and field data at Earth's
/// temperatures (Mlawer et al. 2012).
static MT_CKD: LazyLock<Vec<[f64; 4]>> = LazyLock::new(|| {
    let rows = parse_rows::<4>(
        include_str!("../../data/mt-ckd-water.tsv"),
        "mt-ckd-water.tsv",
    );
    for (k, row) in rows.iter().enumerate() {
        assert!(
            row[0] == k as f64 * MT_CKD_STEP_CM && row[1] > 0.0 && row[3] > 0.0,
            "mt-ckd-water.tsv row {k} is off the grid: {row:?}"
        );
    }
    assert_eq!(
        rows.len() as f64,
        SPECTRUM_TOP_CM / MT_CKD_STEP_CM + 1.0,
        "mt-ckd-water.tsv does not cover the spectrum"
    );
    rows
});

/// Definition: the temperatures, K, of the columns of [`CO2_CIA`], the
/// table's temperature grid. An author's choice with no source: for each
/// band the file's set nearest each of them is binned (see [`CO2_CIA`]),
/// and the coefficient is interpolated linearly between them, held at the
/// ends ([`co2_cia_column`]).
const CO2_CIA_K: [f64; 4] = [200.0, 300.0, 450.0, 800.0];

/// Definition: width of the cells of [`CO2_CIA`], cm⁻¹.
pub(crate) const CO2_CIA_CELL_CM: f64 = 20.0;

/// Source: upper end, cm⁻¹, of the last band of [`CO2_CIA`], the end of
/// HITRAN's `CO2-CO2_2024.cia` (1-4500 cm⁻¹).
pub(crate) const CO2_CIA_TOP_CM: f64 = 4500.0;

/// Source: CO₂–CO₂ collision-induced absorption, binned from HITRAN's
/// `CO2-CO2_2024.cia` (hitran.org/data/CIA/main/CO2-CO2_2024.cia,
/// HITRAN2024, documented by Terragni et al., "Collision Induced
/// Absorption in HITRAN2024", and `CIA_Readme_2024.pdf`; SHA-256 in
/// `OPACITY.md`) by `tools/opacity-table --continua` into
/// `crates/science/data/co2-cia.tsv`: the start of each
/// [`CO2_CIA_CELL_CM`] cm⁻¹ cell and the trapezoid mean of the binary
/// absorption coefficient over it, cm⁵/molecule², at each of
/// [`CO2_CIA_K`]. Each band takes the file's set nearest that
/// temperature, held at its warmest past it. The bands and their sources
/// (`CIA_Readme_2024.pdf`):
///
/// - roto-translational, 1-750 cm⁻¹, sets at 200 to 800 K: Gruszka &
///   Borysow 1997, *Icarus* 129, 172, doi:10.1006/icar.1997.5773;
/// - dimer bands at 1285/1388 cm⁻¹, 1150-1850 cm⁻¹ at 192 to 360 K:
///   Baranov, Lafferty & Fraser 2004, *J. Mol. Spectrosc.* 228, 432,
///   doi:10.1016/j.jms.2004.04.010; 1120-1800 cm⁻¹ at 375 to 500 K:
///   Terragni et al. 2025, *JQSRT* 346, 109595,
///   doi:10.1016/j.jqsrt.2025.109595;
/// - 2510-2850 cm⁻¹, sets at 221 to 297 K: Baranov et al. 2003 (the
///   readme's reference 38);
/// - 2850-3250 and 3995-4500 cm⁻¹, sets at 230 to 298 K: Baranov 2018,
///   *J. Mol. Spectrosc.* 345, 11, doi:10.1016/j.jms.2017.11.005, and
///   Tran, Hartmann, Rambinison & Turbet 2024, *Icarus* 422, 116265,
///   doi:10.1016/j.icarus.2024.116265.
///
/// Cells under 1e-50 cm⁵ at every temperature are left out (at Venus's
/// 92 bar and 735 K they would add under 0.013 to the depth per `ln p`).
static CO2_CIA: LazyLock<Vec<(f64, [f64; 4])>> = LazyLock::new(|| {
    let rows: Vec<(f64, [f64; 4])> =
        parse_rows::<5>(include_str!("../../data/co2-cia.tsv"), "co2-cia.tsv")
            .into_iter()
            .map(|r| (r[0], [r[1], r[2], r[3], r[4]]))
            .collect();
    assert!(
        rows.windows(2).all(|w| w[0].0 < w[1].0)
            && rows.iter().all(|(start, k)| {
                *start % CO2_CIA_CELL_CM == 0.0
                    && *start < CO2_CIA_TOP_CM
                    && k.iter().all(|k| *k >= 0.0)
            }),
        "co2-cia.tsv is not on ascending cells of {CO2_CIA_CELL_CM} cm⁻¹"
    );
    rows
});

/// The rows of a tab-separated table of `N` numbers per line, its lines
/// starting with `#` skipped; a malformed line panics, naming `file`.
fn parse_rows<const N: usize>(text: &str, file: &str) -> Vec<[f64; N]> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.starts_with('#') && !line.trim().is_empty())
        .map(|(index, line)| {
            let fields: Vec<&str> = line.split('\t').collect();
            assert!(
                fields.len() == N,
                "{file} line {}: {} fields, expected {N}",
                index + 1,
                fields.len()
            );
            std::array::from_fn(|i| match fields[i].trim().parse::<f64>() {
                Ok(v) if v.is_finite() => v,
                _ => panic!("{file} line {}: {:?} is not a number", index + 1, fields[i]),
            })
        })
        .collect()
}

/// Optical depth per `ln p` of CO₂–CO₂ pairs at `nu` cm⁻¹:
/// `k(ν, T) n²` per metre of air (`n` the CO₂ number density), times the
/// air's thickness per `ln p`, `k_B T / (m̄ g)`. Not broadened.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the shortwave of PR 2b-2 reads it; the tests' reference sum does now"
    )
)]
pub(crate) fn co2_pairs(nu: f64, level: &Level) -> f64 {
    let co2 = level.gases.co2;
    if co2 <= 0.0 {
        return 0.0;
    }
    let Some(row) = co2_cia_row(nu) else {
        return 0.0;
    };
    let values = CO2_CIA[row].1;
    let (j, f) = co2_cia_column(level.temperature_k);
    let k_cm5 = values[j] * (1.0 - f) + values[j + 1] * f;
    let density_m3 = level.density_m3(co2);
    k_cm5 * 1e-10 * density_m3 * density_m3 * level.thickness_per_ln_p_m()
}

/// Row of [`CO2_CIA`] whose cell holds `nu` cm⁻¹, if any.
fn co2_cia_row(nu: f64) -> Option<usize> {
    let cell = (nu / CO2_CIA_CELL_CM).floor() * CO2_CIA_CELL_CM;
    CO2_CIA
        .binary_search_by(|(start, _)| start.total_cmp(&cell))
        .ok()
}

/// Where `t_k` falls among [`CO2_CIA_K`]: the column below it and the
/// share of the next, held at the ends.
fn co2_cia_column(t_k: f64) -> (usize, f64) {
    if t_k <= CO2_CIA_K[0] {
        (0, 0.0)
    } else if t_k >= CO2_CIA_K[3] {
        (2, 1.0)
    } else {
        let j = CO2_CIA_K
            .iter()
            .rposition(|&k| k <= t_k)
            .unwrap_or(0)
            .min(2);
        (j, (t_k - CO2_CIA_K[j]) / (CO2_CIA_K[j + 1] - CO2_CIA_K[j]))
    }
}

/// The radiation term `R(ν, T) = ν tanh(c₂ν / 2T)` of MT_CKD's continua,
/// cm⁻¹, at `nu` cm⁻¹ and `t_k`.
pub(crate) fn radiation_term(nu: f64, t_k: f64) -> f64 {
    nu * (0.5 * SECOND_RADIATION_CONSTANT_CM_K * nu / t_k).tanh()
}

/// Optical depth per `ln p` of MT_CKD 4.3's water continuum at `nu`
/// cm⁻¹ at `level`, from its `coefficients` there (the self continuum at
/// 296 K, its temperature exponent `n`, the foreign continuum at 296 K,
/// cm² per molecule per cm⁻¹; see [`MT_CKD`]): the absorption per H₂O
/// molecule, as in MT_CKD's `mt_ckd_h2o_absco`,
///
/// - self: `C_s (296/T)^n (e / p_ref) (296/T) R(ν, T)`;
/// - foreign: `C_f ((p − e) / p_ref) (296/T) R(ν, T)`;
///
/// ([`radiation_term`]), times the vapour's molecules per unit area per
/// `ln p`, `e N_A / (M̄ g)`, so that the self part grows as `e²` and the
/// foreign part as `e (p − e)`. The shortwave's binned coefficients
/// (a later change) take it too.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the shortwave of PR 2b-2 reads it; the tests' reference sum does now"
    )
)]
pub(crate) fn water_continuum_depth(coefficients: (f64, f64, f64), nu: f64, level: &Level) -> f64 {
    let e = level.gases.h2o;
    if e <= 0.0 {
        return 0.0;
    }
    let (own, exponent, foreign) = coefficients;
    let t = level.temperature_k;
    let ratio = MT_CKD_REFERENCE_K / t;
    let radiation = radiation_term(nu, t);
    let others = (level.pressure_bar() - e).max(0.0);
    let per_molecule_cm2 = (own * ratio.powf(exponent) * e + foreign * others)
        / MT_CKD_REFERENCE_BAR
        * ratio
        * radiation;
    per_molecule_cm2 * 1e-4 * level.molecules_per_ln_p(e)
}

/// Absorption, cm², of one CO₂ molecule at `nu` cm⁻¹ and `t` K (`ln_t`
/// is `ln (T / T_eff)`) in air at MT_CKD's reference density, from the
/// bin's `coefficient` (its value at `T_eff` and its exponent, see
/// [`co2_continuum_depth`]).
pub(crate) fn co2_continuum_per_molecule(
    coefficient: (f64, f64),
    nu: f64,
    t: f64,
    ln_t: f64,
) -> f64 {
    let (c, a) = coefficient;
    if c <= 0.0 {
        return 0.0;
    }
    c * (a * ln_t).exp() * radiation_term(nu, t)
}

/// Optical depth per `ln p` of MT_CKD's CO₂ continuum at `nu` cm⁻¹ at
/// `level`, from a table bin's `coefficient` and the table's `t_eff_k`:
/// the far wings of CO₂'s lines beyond the tables' 25 cm⁻¹ cutoff, with
/// their sub-Lorentzian shape and line coupling, as LBLRTM's `CONTNM`
/// takes it, `C(ν) (p / p_ref)(296 K / T) R(ν, T) (T / T_eff)^n` per
/// molecule ([`co2_continuum_per_molecule`]; the coefficients binned by
/// `tools/opacity-table` from MT_CKD 4.3's `contnm.f90`; see
/// `crates/science/data/OPACITY.md`). It is broadened by the whole air,
/// like water's foreign continuum, so that it grows as the CO₂ times the
/// pressure: it closes the windows of a thick CO₂ air (a vertical depth
/// of about 7 at 1000 cm⁻¹ under 8 bar) and is negligible at Earth's
/// 400 ppm outside the 15 µm band.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the shortwave of PR 2b-2 reads it; the tests' reference sum does now"
    )
)]
pub(crate) fn co2_continuum_depth(
    coefficient: (f64, f64),
    t_eff_k: f64,
    nu: f64,
    level: &Level,
) -> f64 {
    let co2 = level.gases.co2;
    if co2 <= 0.0 {
        return 0.0;
    }
    let t = level.temperature_k;
    let p = level.pressure_bar();
    let density = p / MT_CKD_REFERENCE_BAR * MT_CKD_REFERENCE_K / t;
    let ln_t = (t / t_eff_k).ln();
    co2_continuum_per_molecule(coefficient, nu, t, ln_t)
        * density
        * 1e-4
        * level.molecules_per_ln_p(co2)
}

/// Calibration: upper end, cm⁻¹, of methane's grey stand-in
/// ([`super::methane`]), the far infrared where Titan's N₂–N₂ and CH₄–N₂
/// collision-induced absorption lies. Part of the stand-in's calibration
/// with [`super::methane`]'s opacity, fitted on Titan's surface
/// temperature; the legacy names Courtin, Gautier & McKay 1995 (*Icarus*
/// 114, 144) as "the design's call", a paper not read for this value
/// (audit, Wiki study 19). A spectral N₂–CH₄ CIA from HITRAN would
/// source it (future work).
const METHANE_TOP_CM: f64 = 650.0;

/// Definition: bands of the column's sum: the [`BINS`] bins, and the tail past
/// [`SPECTRUM_TOP_CM`] as one more band ([`TAIL`]).
pub(crate) const BANDS: usize = BINS + 1;

/// Definition: index of the tail band, from [`SPECTRUM_TOP_CM`] to infinity. It takes
/// the absorption of the last bin, so that a hot column that the bins
/// leave short of `σT⁴` (0.4 % at 640 K, 43 % at 1800 K) still sends out
/// the rest of its black body through the air there, and a column of
/// transparent air sends out exactly the surface's `σT⁴`, as an airless
/// one does.
pub(crate) const TAIL: usize = BINS;

/// What the smooth absorption of every bin needs that does not depend on
/// the level, at the bin's centre (see [`SmoothRates`]).
struct BinTable {
    /// Centre, cm⁻¹.
    centre_cm: [f64; BINS],
    /// [`MT_CKD`]'s self coefficient, its temperature exponent and its
    /// foreign coefficient at the centre ([`mt_ckd_at`]).
    continuum: [(f64, f64, f64); BINS],
    /// Row of [`CO2_CIA`], if the bin has one.
    cia_row: [Option<usize>; BINS],
    /// Share of the bin under [`METHANE_TOP_CM`].
    methane_share: [f64; BINS],
}

/// Definition: [`BinTable::continuum`] in single precision. The self and foreign
/// coefficients are scaled by 1e20: a cm² per molecule of 1e-27 is
/// under the least normal `f32` times the vapour's molecules; the factor
/// comes back in [`SmoothRates::fast`]'s `scale`.
static FAST_CONTINUUM: LazyLock<[(f32, f32, f32); BINS]> = LazyLock::new(|| {
    BIN_TABLE
        .continuum
        .map(|(s, n, f)| ((s * 1e20) as f32, n as f32, (f * 1e20) as f32))
});

/// [`BinTable::centre_cm`] in single precision.
static FAST_CENTRE: LazyLock<[f32; BINS]> = LazyLock::new(|| BIN_TABLE.centre_cm.map(|c| c as f32));

/// Definition: what each bin's smooth absorption needs that does not
/// depend on the level ([`BinTable`]).
static BIN_TABLE: LazyLock<BinTable> = LazyLock::new(|| {
    let mut table = BinTable {
        centre_cm: [0.0; BINS],
        continuum: [(0.0, 0.0, 0.0); BINS],
        cia_row: [None; BINS],
        methane_share: [0.0; BINS],
    };
    for i in 0..BINS {
        let (lo, hi) = bin_edges(i);
        let nu = 0.5 * (lo + hi);
        table.centre_cm[i] = nu;
        table.continuum[i] = mt_ckd_at(nu);
        table.cia_row[i] = co2_cia_row(nu);
        table.methane_share[i] = ((METHANE_TOP_CM.min(hi) - lo) / (hi - lo)).clamp(0.0, 1.0);
    }
    table
});

/// [`MT_CKD`]'s coefficients at `nu` cm⁻¹: the self coefficient, its
/// temperature exponent and the foreign coefficient, the coefficients
/// interpolated in their logarithm and the exponent linearly between the
/// nodes.
fn mt_ckd_at(nu: f64) -> (f64, f64, f64) {
    let x = (nu / MT_CKD_STEP_CM).clamp(0.0, (MT_CKD.len() - 1) as f64);
    let k = (x as usize).min(MT_CKD.len() - 2);
    let f = x - k as f64;
    let (a, b) = (MT_CKD[k], MT_CKD[k + 1]);
    let log_lerp = |u: f64, v: f64| (u.ln() * (1.0 - f) + v.ln() * f).exp();
    (
        log_lerp(a[1], b[1]),
        a[2] * (1.0 - f) + b[2] * f,
        log_lerp(a[3], b[3]),
    )
}

/// What the smooth absorption of a bin needs at one level, taken once for every bin:
/// the continua of water and CO₂, the CO₂ pairs and methane, each when the level
/// holds it.
struct SmoothRates {
    /// The level's temperature, K.
    t: f64,
    /// Water's continuum: the vapour, bar; `ln (296 K / T)`; the other
    /// gases, bar; and the factor of the absorption per molecule.
    continuum: Option<(f64, f64, f64, f64)>,
    /// CO₂ pairs: the column of [`CO2_CIA_K`] below the temperature, the
    /// share of the next, and the factor of the coefficient.
    pairs: Option<(usize, f64, f64)>,
    /// Methane's rate per `ln p` ([`super::methane::depth`]).
    methane: Option<f64>,
    /// CO₂'s continuum: `ln (T / T_eff)` and the factor of its
    /// coefficient.
    co2_continuum: Option<(f64, f64)>,
}

impl SmoothRates {
    fn of(level: &Level) -> Self {
        let p = level.pressure_bar();
        let t = level.temperature_k;
        let g = &level.gases;
        let continuum = (g.h2o > 0.0).then(|| {
            let e = g.h2o;
            let ratio = MT_CKD_REFERENCE_K / t;
            let others = (p - e).max(0.0);
            let scale = ratio / MT_CKD_REFERENCE_BAR * 1e-4 * level.molecules_per_ln_p(e);
            (e, ratio.ln(), others, scale)
        });
        let pairs = (g.co2 > 0.0).then(|| {
            let (j, f) = co2_cia_column(t);
            let density_m3 = level.density_m3(g.co2);
            (
                j,
                f,
                1e-10 * density_m3 * density_m3 * level.thickness_per_ln_p_m(),
            )
        });
        let methane = (g.ch4 > 0.0).then(|| methane::depth(g).1);
        let co2_continuum = (g.co2 > 0.0).then(|| {
            let density = p / MT_CKD_REFERENCE_BAR * MT_CKD_REFERENCE_K / t;
            (
                (t / OPACITY.co2_continuum_t_eff_k).ln(),
                density * 1e-4 * level.molecules_per_ln_p(g.co2),
            )
        });
        Self {
            t,
            continuum,
            pairs,
            methane,
            co2_continuum,
        }
    }

    /// The smooth absorption per `ln p` of every bin (`reference`'s
    /// `SmoothRates::bin`) into `out`, in single precision
    /// with [`super::fast_math`]'s `exp` (`tanh y = 1 − 2 / (e^{2y} + 1)`).
    fn fast(&self, out: &mut [f32]) {
        use super::fast_math::exp;
        let table = &*BIN_TABLE;
        out.fill(0.0);
        if let Some((e, ln_ratio, others, scale)) = self.continuum {
            let (e, ln_ratio, others, scale) = (
                e as f32,
                ln_ratio as f32,
                others as f32,
                (scale * 1e-20) as f32,
            );
            let c2_t = (SECOND_RADIATION_CONSTANT_CM_K / self.t) as f32;
            for (i, o) in out.iter_mut().enumerate() {
                let (self_coeff, exponent, foreign_coeff) = FAST_CONTINUUM[i];
                let nu = FAST_CENTRE[i];
                // `tanh (y/2)`, from its series where the subtraction
                // would cancel.
                let y = c2_t * nu;
                let tanh = if y < 0.1 {
                    0.5 * y * (1.0 - y * y / 12.0)
                } else {
                    1.0 - 2.0 / (exp(y) + 1.0)
                };
                let radiation = nu * tanh;
                *o += (self_coeff * exp(exponent * ln_ratio) * e + foreign_coeff * others)
                    * scale
                    * radiation;
            }
        }
        if let Some((j, f, scale)) = self.pairs {
            for (o, row) in out.iter_mut().zip(&table.cia_row) {
                if let Some(row) = row {
                    let v = CO2_CIA[*row].1;
                    *o += ((v[j] * (1.0 - f) + v[j + 1] * f) * scale) as f32;
                }
            }
        }
        if let Some(rate) = self.methane {
            for (o, share) in out.iter_mut().zip(&table.methane_share) {
                *o += (share * rate) as f32;
            }
        }
        if let Some((ln_t, scale)) = self.co2_continuum {
            for (i, o) in out.iter_mut().enumerate() {
                let (coefficient, nu) = (OPACITY.co2_continuum[i], BIN_TABLE.centre_cm[i]);
                *o += (co2_continuum_per_molecule(coefficient, nu, self.t, ln_t) * scale) as f32;
            }
        }
    }
}

/// Definition: number of a level's g-points over every bin.
pub(crate) const POINTS: usize = BINS * G;

/// The absorption of one level in the column's fast layout (see
/// [`fast_level`]): CO₂'s and water's lines at each bin and g-point
/// (index `bin · G + g`) and the smooth part of each bin, per unit `ln p`,
/// and their logarithms, in single precision.
#[derive(Clone)]
pub(crate) struct FastLevel {
    /// CO₂'s lines and their logarithms.
    pub(crate) co2: [f32; POINTS],
    pub(crate) co2_ln: [f32; POINTS],
    /// Water's lines and their logarithms.
    pub(crate) water: [f32; POINTS],
    pub(crate) water_ln: [f32; POINTS],
    /// The continua of water and CO₂, CO₂–CO₂ pairs and methane, per bin, and their
    /// logarithms.
    pub(crate) smooth: [f32; BINS],
    pub(crate) smooth_ln: [f32; BINS],
}

impl Default for FastLevel {
    fn default() -> Self {
        Self {
            co2: [0.0; POINTS],
            co2_ln: [0.0; POINTS],
            water: [0.0; POINTS],
            water_ln: [0.0; POINTS],
            smooth: [0.0; BINS],
            smooth_ln: [0.0; BINS],
        }
    }
}

/// `reference::absorption_spectrum` of the first `bins` bins at `level`, into `out`
/// in its fast layout (the other bins are left as they were): the same
/// table, interpolation and sums, in single precision with
/// [`super::fast_math`]'s `exp` and `ln`, in loops over the bins and
/// g-points that compile to vector instructions. An absent gas has no
/// lines (zero).
pub(crate) fn fast_level(level: &Level, bins: usize, out: &mut FastLevel) {
    let table = &*OPACITY;
    let bins = bins.min(BINS);
    let m = bins * G;
    let p = level.pressure_bar();
    let t = level.temperature_k;
    let g = &level.gases;
    let valid = p > 0.0 && t > 0.0 && level.gravity_m_s2 > 0.0;
    // Each gas's lines ([`Table::lines`]), then their logarithms.
    let mut own_ln = [0.0; POINTS];
    if valid && g.co2 > 0.0 {
        let sets = (CO2_SET, CO2_SELF_SET);
        let (by_air, sum) = (&mut out.co2_ln[..m], &mut out.co2[..m]);
        table.lines(
            level,
            g.co2,
            CO2_MOLAR_MASS,
            sets,
            m,
            by_air,
            &mut own_ln[..m],
            sum,
        );
        ln_all(&out.co2[..m], &mut out.co2_ln[..m]);
    } else {
        out.co2[..m].fill(0.0);
    }
    if valid && g.h2o > 0.0 {
        let sets = (WATER_FOREIGN_SET, WATER_SELF_SET);
        let (by_air, sum) = (&mut out.water_ln[..m], &mut out.water[..m]);
        table.lines(
            level,
            g.h2o,
            H2O_MOLAR_MASS,
            sets,
            m,
            by_air,
            &mut own_ln[..m],
            sum,
        );
        ln_all(&out.water[..m], &mut out.water_ln[..m]);
    } else {
        out.water[..m].fill(0.0);
    }
    if valid {
        SmoothRates::of(level).fast(&mut out.smooth[..bins]);
    } else {
        out.smooth[..bins].fill(0.0);
    }
    ln_all(&out.smooth[..bins], &mut out.smooth_ln[..bins]);
}

/// `ln x` of every `x`, into `out`, in a function of its own so that it
/// takes vector instructions.
#[inline(never)]
fn ln_all(x: &[f32], out: &mut [f32]) {
    for (o, x) in out.iter_mut().zip(x) {
        *o = super::fast_math::ln(*x);
    }
}

/// Vertical optical depth per bin above `level`, the top of a column, of
/// what is smooth: `reference::depth_above`'s, from the level's smooth rate `smooth` in
/// bin `bin`.
pub(crate) fn smooth_above(level: &Level, smooth: f64, bin: usize) -> f64 {
    let (total, methane_rate) = methane::depth(&level.gases);
    0.5 * smooth + BIN_TABLE.methane_share[bin] * (total - 0.5 * methane_rate)
}

/// Black-body flux, W/m², of every band at `t_k` into `out`: the bins'
/// exact integrals (`reference::planck_bin_w_m2`) and the tail's rest of `σT⁴` past
/// [`SPECTRUM_TOP_CM`], each Planck integral taken once per edge.
pub(crate) fn planck_spectrum(t_k: f64, out: &mut [f64; BANDS]) {
    planck_spectrum_upto(t_k, BINS, out);
}

/// [`planck_spectrum`] of the first `bins` bins, and of the tail when they
/// are all of them; the other bands are left at zero.
pub(crate) fn planck_spectrum_upto(t_k: f64, bins: usize, out: &mut [f64; BANDS]) {
    out.fill(0.0);
    if t_k <= 0.0 {
        return;
    }
    let black = STEFAN_BOLTZMANN * t_k.powi(4) / PLANCK_TOTAL;
    // At each edge, the head integral from 0 under the switch and the
    // tail integral to infinity over it, as `planck_between` takes them.
    let edge = |k: usize| {
        let x = SECOND_RADIATION_CONSTANT_CM_K * k as f64 * BIN_WIDTH_CM / t_k;
        if x >= SERIES_SWITCH {
            planck_tail(x)
        } else {
            planck_head(x)
        }
    };
    let mut x_lo = 0.0;
    let mut lo = edge(0);
    for (i, o) in out[..bins.min(BINS)].iter_mut().enumerate() {
        let x_hi = SECOND_RADIATION_CONSTANT_CM_K * (i + 1) as f64 * BIN_WIDTH_CM / t_k;
        let hi = edge(i + 1);
        let integral = if x_lo >= SERIES_SWITCH {
            lo - hi
        } else if x_hi < SERIES_SWITCH {
            hi - lo
        } else {
            (PLANCK_TOTAL - hi) - lo
        };
        *o = black * integral.max(0.0);
        (x_lo, lo) = (x_hi, hi);
    }
    if bins < BINS {
        return;
    }
    let rest = if x_lo >= SERIES_SWITCH {
        lo
    } else {
        PLANCK_TOTAL - lo
    };
    out[TAIL] = black * rest.max(0.0);
}
