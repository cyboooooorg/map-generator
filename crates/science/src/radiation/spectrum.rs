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
//! - **CO₂, water, methane, ozone and N₂O lines:** a correlated-k table built offline from the
//!   HITRAN2024 line lists (Gordon et al. 2026, *JQSRT* 109807; the
//!   design named HITRAN2020, which HITRAN no longer serves) by
//!   `tools/opacity-table` (provenance in `crates/science/data/OPACITY.md`).
//!   Within a bin the lines' coefficients are sorted into [`G`] g-points,
//!   each with a weight ([`weights`]), on a grid of pressure and
//!   temperature; CO₂, water and methane have a set broadened by air and
//!   one broadened by itself (HITRAN's `γ_self`), mixed by its share of
//!   the pressure, and ozone and N₂O, always traces, one broadened by air
//!   each ([`LineGas`]; studies 36 and 34,
//!   `docs/science/longwave-absorbers.md` and `docs/science/ozone.md`). The
//!   table holds the lines' temperature dependence (strengths, hot bands,
//!   widths) and their Voigt shape out to 25 cm⁻¹, less the pedestal
//!   MT_CKD's continua assume (LBLRTM's flat one for methane, ozone and
//!   N₂O, which have no continuum; [`Table`]).
//! - **CO₂ continuum:** MT_CKD 4.3's, the far wings of CO₂'s lines beyond
//!   25 cm⁻¹ with their sub-Lorentzian shape, binned into the table
//!   ([`co2_continuum_depth`]); it grows as the CO₂ times the pressure.
//! - **Water continuum:** MT_CKD 4.3's self and foreign continua (Mlawer
//!   et al. 2012), the self part growing as the vapour pressure squared
//!   and the foreign part as the vapour times the other gases
//!   ([`MT_CKD`]); it assumes the 25 cm⁻¹ line cutoff the table uses.
//! - **Collision-induced absorption:** HITRAN's CIA files, binned
//!   ([`PAIRS`]): CO₂–CO₂ (HITRAN2024, documented by Terragni et al.,
//!   [`CO2_CIA`]), and the N₂–N₂, O₂–O₂, O₂–N₂, N₂–CH₄ and CH₄–CH₄ pairs
//!   ([`PAIRS_CIA`], study 36, `docs/science/longwave-absorbers.md`),
//!   each growing as the product of its two gases' densities, not
//!   broadened. Titan's greenhouse is mostly these pairs; methane's own
//!   lines add to them, each counting the same methane once.
//!
//! The continua and the pairs are smooth across a bin: each is
//! one optical depth per bin, added to every g-point ([`FastLevel`]). The
//! double-precision sums the tests hold the fast ones to are in
//! `reference`.
//! The gases with lines overlap at random within a bin (Lacis & Oinas 1991,
//! *JGR* 96, 9027, doi:10.1029/90JD01945, the correlated-k method and its
//! random overlap; the table's eight g-intervals are its builder's
//! calibration, see `OPACITY.md`): the transmission of a path is the product of each
//! gas's transmission, each the weighted sum over its g-points. The
//! column masses come from `p / g` with the body's gravity
//! ([`Level::gravity_m_s2`]).

use std::sync::LazyLock;

use super::gases::{
    AR_MOLAR_MASS, CH4_MOLAR_MASS, CO2_MOLAR_MASS, H2O_MOLAR_MASS, N2_MOLAR_MASS, N2O_MOLAR_MASS,
    NH3_MOLAR_MASS, O2_MOLAR_MASS, O3_MOLAR_MASS, PartialPressures, SO2_MOLAR_MASS,
};
pub(crate) use super::ktable::G;
use super::ktable::{
    CH4_SELF_SET, CH4_SET, CO2_SELF_SET, CO2_SET, LONGWAVE_SETS, N2O_SET, O3_SET, Table,
    WATER_FOREIGN_SET, WATER_SELF_SET,
};
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

/// Definition: upper end of the spectrum of the longwave table, cm⁻¹,
/// the top of `opacity.bin`: a black body at 640 K, the hottest steam
/// column the legacy climate solved, emits 99.6 % of its flux below it
/// (0.38 % past it, audit, Wiki study 19); the rest goes to the tail band
/// ([`TAIL`]), which the test
/// `the_planck_spectrum_is_the_bins_and_their_rest` sums with the bins to
/// `σT⁴`, and which the column sums on the shortwave table's bins
/// (`column::beyond`, study 41).
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
    /// Ozone at the level, bar, a share of its moles as the other gases
    /// are read ([`Level::mass_per_ln_p`]); a trace, left out of the
    /// pressure. The column lays it out from the air's ozone column
    /// ([`super::ozone::OzoneProfile`]).
    pub ozone_bar: f64,
    /// N₂O at the level, bar, a share of its moles, a trace left out of
    /// the pressure like ozone. No air of the model holds N₂O yet (its
    /// law is issue #737): only tests set it, through the column's
    /// `with_n2o`.
    pub n2o_bar: f64,
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
            + g.o2.max(0.0) * O2_MOLAR_MASS
            + g.ar.max(0.0) * AR_MOLAR_MASS)
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

/// Definition: the number of gases with lines ([`LineGas`]).
pub(crate) const LINE_GASES: usize = 5;

/// A gas whose lines the longwave table holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LineGas {
    Co2,
    Water,
    Ch4,
    O3,
    N2o,
}

impl LineGas {
    /// Definition: every gas with lines, in the order of [`FastLevel::lines`].
    pub(crate) const ALL: [LineGas; LINE_GASES] = [
        LineGas::Co2,
        LineGas::Water,
        LineGas::Ch4,
        LineGas::O3,
        LineGas::N2o,
    ];

    /// Its sets in the table, broadened by air and by itself (ozone and
    /// N₂O, always traces, have their air set only).
    pub(crate) fn sets(self) -> (usize, usize) {
        match self {
            LineGas::Co2 => (CO2_SET, CO2_SELF_SET),
            LineGas::Water => (WATER_FOREIGN_SET, WATER_SELF_SET),
            LineGas::Ch4 => (CH4_SET, CH4_SELF_SET),
            LineGas::O3 => (O3_SET, O3_SET),
            LineGas::N2o => (N2O_SET, N2O_SET),
        }
    }

    /// Its molar mass, kg/mol.
    pub(crate) fn molar_kg(self) -> f64 {
        match self {
            LineGas::Co2 => CO2_MOLAR_MASS,
            LineGas::Water => H2O_MOLAR_MASS,
            LineGas::Ch4 => CH4_MOLAR_MASS,
            LineGas::O3 => O3_MOLAR_MASS,
            LineGas::N2o => N2O_MOLAR_MASS,
        }
    }

    /// Its partial pressure at `level`, bar.
    pub(crate) fn bar(self, level: &Level) -> f64 {
        match self {
            LineGas::Co2 => level.gases.co2,
            LineGas::Water => level.gases.h2o,
            LineGas::Ch4 => level.gases.ch4,
            LineGas::O3 => level.ozone_bar,
            LineGas::N2o => level.n2o_bar,
        }
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

/// The longwave table of the lines of CO₂, water, methane, ozone and N₂O
/// (`crates/science/data/opacity.bin`, see [`super::ktable`]), on the
/// model's bins.
pub(crate) static OPACITY: LazyLock<Table> = LazyLock::new(|| {
    let table = Table::parse(include_bytes!("../../data/opacity.bin"))
        .expect("crates/science/data/opacity.bin is a table of 8 g-points");
    assert!(
        table.edges_cm.is_empty()
            && table.bins == BINS
            && table.bin_width_cm == BIN_WIDTH_CM
            && table.sets == LONGWAVE_SETS,
        "the opacity table's bins or sets are not the model's"
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
/// ends ([`Pair::coefficient`]).
const CO2_CIA_K: [f64; 4] = [200.0, 300.0, 450.0, 800.0];

/// Definition: width of the cells of the pair tables ([`CO2_CIA`],
/// [`PAIRS_CIA`]), cm⁻¹, the model's bin width.
pub(crate) const CIA_CELL_CM: f64 = 20.0;

/// Source: upper end, cm⁻¹, of the last band of [`CO2_CIA`], the end of
/// HITRAN's `CO2-CO2_2024.cia` (1-4500 cm⁻¹).
pub(crate) const CO2_CIA_TOP_CM: f64 = 4500.0;

/// Source: CO₂–CO₂ collision-induced absorption, binned from HITRAN's
/// `CO2-CO2_2024.cia` (hitran.org/data/CIA/main/CO2-CO2_2024.cia,
/// HITRAN2024, documented by Terragni et al., "Collision Induced
/// Absorption in HITRAN2024", and `CIA_Readme_2024.pdf`; SHA-256 in
/// `OPACITY.md`) by `tools/opacity-table --continua` into
/// `crates/science/data/co2-cia.tsv`: the start of each
/// [`CIA_CELL_CM`] cm⁻¹ cell and the trapezoid mean of the binary
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
static CO2_CIA: LazyLock<Vec<(f64, Vec<f64>)>> = LazyLock::new(|| {
    let text = include_str!("../../data/co2-cia.tsv");
    // The header names the columns' temperatures: a table written at other
    // temperatures than [`CO2_CIA_K`] must not be read as these.
    assert_eq!(
        text.lines().next(),
        Some(cia_header("# cell_start_cm", &CO2_CIA_K).as_str()),
        "co2-cia.tsv's columns are not at {CO2_CIA_K:?} K"
    );
    let rows: Vec<(f64, Vec<f64>)> = parse_rows::<5>(text, "co2-cia.tsv")
        .into_iter()
        .map(|r| (r[0], r[1..].to_vec()))
        .collect();
    check_cells(&rows, CO2_CIA_TOP_CM, "co2-cia.tsv");
    rows
});

/// Definition: the temperatures, K, of the columns of [`PAIRS_CIA`],
/// every 10 K from 70 to 400 K: the grid of HITRAN's N₂–N₂
/// roto-translational blocks (Chistikov et al. 2019) and of its N₂–CH₄
/// file (Finenko et al. 2022), onto which the other files' blocks are
/// interpolated. The coefficient is interpolated linearly between them,
/// held at the ends ([`Pair::coefficient`]): Titan's 69 K skin takes the
/// 70 K column, a Venus at 735 K the 400 K one.
pub(crate) const PAIRS_CIA_K: [f64; 34] = {
    let mut out = [0.0; 34];
    let mut i = 0;
    while i < 34 {
        out[i] = 70.0 + 10.0 * i as f64;
        i += 1;
    }
    out
};

/// Source: the nitrogen, oxygen and methane pairs, binned from HITRAN's
/// collision-induced absorption files by `tools/opacity-table
/// --continua` into `crates/science/data/pairs-cia.tsv` (study 36,
/// `docs/science/longwave-absorbers.md`; SHA-256 of each file in
/// `OPACITY.md`): per pair, the start of each [`CIA_CELL_CM`] cm⁻¹ cell
/// and the trapezoid mean of the binary absorption coefficient over it,
/// cm⁵/molecule², at each of [`PAIRS_CIA_K`], each band's blocks
/// interpolated linearly in temperature and held at their ends. The files
/// and the sources of their blocks (`CIA_References_2024.pdf`, Karman et
/// al. 2019, *Icarus* 328, 160, doi:10.1016/j.icarus.2019.02.034):
///
/// - `N2-N2_2021.cia`: roto-translational, 0-650 cm⁻¹ at 70 to 400 K,
///   Chistikov et al. 2019 (*J. Chem. Phys.* 151 (19),
///   doi:10.1063/1.5125756, classical trajectories); fundamental,
///   2000-2698 cm⁻¹ at 228 to 272 K, Lafferty et al. 1996 (*Appl. Opt.*
///   35, 5911, doi:10.1364/AO.35.005911), and 1850-3000 cm⁻¹ at 301 to
///   363 K, Baranov, Lafferty & Fraser 2005 (*J. Mol. Spectrosc.* 233,
///   160, doi:10.1016/j.jms.2005.06.008); first overtone, 4300-5000 cm⁻¹
///   at 200 to 330 K, Hartmann, Boulet & Toon 2017 (*J. Geophys. Res.
///   Atmos.* 122, 2419, doi:10.1002/2016JD025677);
/// - `O2-O2_2024.cia`: fundamental, 1150-1950 cm⁻¹ at 193 to 353 K,
///   Baranov, Lafferty & Fraser 2005 (its bands past 5000 cm⁻¹ are
///   outside the spectrum);
/// - `O2-N2_2024.cia`: O₂'s fundamental induced by N₂, 1300-1850 cm⁻¹ at
///   193 to 356 K, Thibault et al. 1997 (*Appl. Opt.* 36, 563,
///   doi:10.1364/ao.36.000563) and Orlando et al. 1991 (*J. Geophys.
///   Res.* 96, 20755, doi:10.1029/91JD02042); N₂'s fundamental induced
///   by O₂, 1850-3000 cm⁻¹ at 228 to 363 K, estimated from N₂–N₂ with
///   Lafferty et al. 1996's eq. 7;
/// - `N2-CH4_2024.cia`: 0-800 cm⁻¹ at 70 to 400 K, Finenko et al. 2022
///   (*Astrophys. J. Suppl.* 258, 33, doi:10.3847/1538-4365/ac36d3),
///   built for Titan;
/// - `CH4-CH4_2011.cia` (HITRAN's alternate set, the only CH₄–CH₄ file it
///   serves): 0-990 cm⁻¹ at 40 to 400 K, Borysow & Frommhold 1987
///   (*Astrophys. J.* 318, 940, doi:10.1086/165426).
///
/// Cells under 1e-50 cm⁵ at every temperature are left out.
static PAIRS_CIA: LazyLock<Vec<(String, f64, Vec<f64>)>> = LazyLock::new(|| {
    let text = include_str!("../../data/pairs-cia.tsv");
    assert_eq!(
        text.lines().next(),
        Some(cia_header("# pair cell_start_cm", &PAIRS_CIA_K).as_str()),
        "pairs-cia.tsv's columns are not at {PAIRS_CIA_K:?} K"
    );
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.starts_with('#') && !line.trim().is_empty())
        .map(|(index, line)| {
            let (name, numbers) = line
                .split_once('\t')
                .unwrap_or_else(|| panic!("pairs-cia.tsv line {}: no pair", index + 1));
            let row = parse_rows::<35>(numbers, "pairs-cia.tsv")
                .pop()
                .unwrap_or_else(|| panic!("pairs-cia.tsv line {}: no numbers", index + 1));
            (name.to_string(), row[0], row[1..].to_vec())
        })
        .collect()
});

/// The header line of a pair table: `first`, then a `k_<T>K` per
/// column.
fn cia_header(first: &str, temperatures: &[f64]) -> String {
    temperatures
        .iter()
        .fold(String::from(first), |h, t| h + &format!(" k_{t}K"))
}

/// Panics, naming `file`, unless `rows` are on ascending cells of
/// [`CIA_CELL_CM`] under `top_cm`, with coefficients of zero or more.
fn check_cells(rows: &[(f64, Vec<f64>)], top_cm: f64, file: &str) {
    assert!(
        rows.windows(2).all(|w| w[0].0 < w[1].0)
            && rows.iter().all(|(start, k)| {
                *start % CIA_CELL_CM == 0.0 && *start < top_cm && k.iter().all(|k| *k >= 0.0)
            }),
        "{file} is not on ascending cells of {CIA_CELL_CM} cm⁻¹"
    );
}

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

/// A gas of a collision-induced pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Partner {
    N2,
    O2,
    Co2,
    Ch4,
}

impl Partner {
    /// Partial pressure of this gas in `gases`, bar.
    fn bar(self, gases: &PartialPressures) -> f64 {
        match self {
            Self::N2 => gases.n2,
            Self::O2 => gases.o2,
            Self::Co2 => gases.co2,
            Self::Ch4 => gases.ch4,
        }
    }
}

/// A collision-induced pair: its two gases and its binned coefficients.
pub(crate) struct Pair {
    /// The two gases whose densities multiply.
    pub(crate) gases: (Partner, Partner),
    /// Temperatures, K, of the columns of `cells`, ascending.
    temperatures_k: &'static [f64],
    /// The cells that hold absorption: the index of the cell
    /// ([`CIA_CELL_CM`] cm⁻¹ from 0, the model's bin) and its
    /// coefficient at each temperature, cm⁵/molecule².
    cells: Vec<(usize, Vec<f64>)>,
    /// For each cell from 0, its row of `cells`, if any.
    rows: Vec<Option<usize>>,
}

impl Pair {
    /// The pair `name` of `gases`, from the rows `(cell start, k at each
    /// of temperatures_k)` of its table.
    fn new(
        name: &'static str,
        gases: (Partner, Partner),
        temperatures_k: &'static [f64],
        rows: impl IntoIterator<Item = (f64, Vec<f64>)>,
    ) -> Self {
        let cells: Vec<(usize, Vec<f64>)> = rows
            .into_iter()
            .map(|(start, k)| {
                assert_eq!(k.len(), temperatures_k.len(), "{name}: a row's columns");
                ((start / CIA_CELL_CM).round() as usize, k)
            })
            .collect();
        let mut index = vec![None; cells.last().map_or(0, |c| c.0 + 1)];
        for (row, (cell, _)) in cells.iter().enumerate() {
            index[*cell] = Some(row);
        }
        Self {
            gases,
            temperatures_k,
            cells,
            rows: index,
        }
    }

    /// Where `t_k` falls among the pair's temperatures: the column below
    /// it and the share of the next, held at the ends.
    fn column(&self, t_k: f64) -> (usize, f64) {
        let ts = self.temperatures_k;
        let last = ts.len() - 1;
        if t_k <= ts[0] {
            (0, 0.0)
        } else if t_k >= ts[last] {
            (last - 1, 1.0)
        } else {
            let j = ts
                .iter()
                .rposition(|&k| k <= t_k)
                .unwrap_or(0)
                .min(last - 1);
            (j, (t_k - ts[j]) / (ts[j + 1] - ts[j]))
        }
    }

    /// The pair's coefficient, cm⁵/molecule², in the cell of `row` at the
    /// column `j` and share `f` of the next ([`Pair::column`]).
    fn coefficient(&self, row: usize, (j, f): (usize, f64)) -> f64 {
        let k = &self.cells[row].1;
        k[j] * (1.0 - f) + k[j + 1] * f
    }

    /// The factor of the pair's coefficient at `level`, per `ln p`:
    /// `1e-10 n_a n_b dz/d ln p` (cm⁵ to m⁵, the two gases' number
    /// densities, the air's thickness per `ln p`); none when either gas
    /// is absent.
    fn factor(&self, level: &Level) -> Option<f64> {
        let (a, b) = (
            self.gases.0.bar(&level.gases),
            self.gases.1.bar(&level.gases),
        );
        (a > 0.0 && b > 0.0).then(|| {
            1e-10 * level.density_m3(a) * level.density_m3(b) * level.thickness_per_ln_p_m()
        })
    }

    /// Optical depth per `ln p` of the pair at `nu` cm⁻¹ at `level`:
    /// `k(ν, T) n_a n_b` per metre of air (the binary absorption
    /// coefficient `k` of HITRAN's `CIA_Readme_2024.pdf`, its eq. 3, the
    /// same for a like and an unlike pair), times the air's thickness per `ln p`,
    /// `k_B T / (m̄ g)`. Not broadened.
    pub(crate) fn depth(&self, nu: f64, level: &Level) -> f64 {
        let Some(factor) = self.factor(level) else {
            return 0.0;
        };
        let cell = (nu / CIA_CELL_CM).floor();
        if cell < 0.0 {
            return 0.0;
        }
        match self.rows.get(cell as usize).copied().flatten() {
            Some(row) => self.coefficient(row, self.column(level.temperature_k)) * factor,
            None => 0.0,
        }
    }
}

/// Definition: index of CO₂–CO₂ in [`PAIRS`].
pub(crate) const CO2_PAIR: usize = 0;

/// Definition: the number of pairs in [`PAIRS`].
pub(crate) const PAIR_COUNT: usize = 6;

/// The collision-induced pairs of the column, each with its source
/// ([`CO2_CIA`], [`PAIRS_CIA`]): CO₂–CO₂, N₂–N₂, O₂–O₂, O₂–N₂, N₂–CH₄
/// and CH₄–CH₄.
pub(crate) static PAIRS: LazyLock<[Pair; PAIR_COUNT]> = LazyLock::new(|| {
    let of = |name: &'static str, gases| {
        let rows = PAIRS_CIA
            .iter()
            .filter(|(n, _, _)| n == name)
            .map(|(_, start, k)| (*start, k.clone()))
            .collect::<Vec<_>>();
        assert!(!rows.is_empty(), "pairs-cia.tsv holds no {name}");
        check_cells(&rows, SPECTRUM_TOP_CM, "pairs-cia.tsv");
        Pair::new(name, gases, &PAIRS_CIA_K, rows)
    };
    use Partner::{Ch4, Co2, N2, O2};
    [
        Pair::new("CO2-CO2", (Co2, Co2), &CO2_CIA_K, CO2_CIA.iter().cloned()),
        of("N2-N2", (N2, N2)),
        of("O2-O2", (O2, O2)),
        of("O2-N2", (O2, N2)),
        of("N2-CH4", (N2, Ch4)),
        of("CH4-CH4", (Ch4, Ch4)),
    ]
});

#[cfg(test)]
thread_local! {
    /// Test builds only: the pairs of [`PAIRS`] (a bit per index) the
    /// spectrum leaves out on this thread while it is set, so that a test
    /// can take a pair's share of the outgoing radiation as the line-by-line
    /// sources do (the gas kept in the air, its absorption removed). A test
    /// that sets it uses a fresh `ColumnCache`, and clears it after through
    /// a drop guard (`column_tests::without_pairs`).
    pub(crate) static PAIRS_LEFT_OUT: std::cell::Cell<u8> = const { std::cell::Cell::new(0) };
}

/// Whether pair `i` of [`PAIRS`] absorbs: always, but in a test that
/// leaves it out ([`PAIRS_LEFT_OUT`]).
fn absorbs(i: usize) -> bool {
    #[cfg(test)]
    {
        PAIRS_LEFT_OUT.with(|left| left.get() & (1 << i) == 0)
    }
    #[cfg(not(test))]
    {
        let _ = i;
        true
    }
}

/// Optical depth per `ln p` of CO₂–CO₂ pairs at `nu` cm⁻¹ at `level`
/// ([`Pair::depth`]), the only pair the shortwave table takes.
pub(crate) fn co2_pairs(nu: f64, level: &Level) -> f64 {
    PAIRS[CO2_PAIR].depth(nu, level)
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
/// take it too ([`super::shortwave`]).
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

/// Definition: bands of the column's sum: the [`BINS`] bins, and the tail past
/// [`SPECTRUM_TOP_CM`] as one more band ([`TAIL`]).
pub(crate) const BANDS: usize = BINS + 1;

/// Definition: index of the tail band, from [`SPECTRUM_TOP_CM`] to
/// infinity: 0.4 % of a black body at 640 K, 40 % at 1800 K, 74 % at
/// 3000 K. The longwave table holds no absorption there; the column sums
/// it on its own (`column::beyond`, study 41): the near infrared on the
/// shortwave table's bins up to 14 364 cm⁻¹ (0.7 µm), and past them the
/// ground's light through Rayleigh scattering alone, so that a column of
/// transparent air still sends out exactly the surface's `σT⁴`, as an
/// airless one does. Its absorption in [`reference::Spectrum`] is none.
pub(crate) const TAIL: usize = BINS;

/// What the smooth absorption of every bin needs that does not depend on
/// the level, at the bin's centre (see [`SmoothRates`]).
struct BinTable {
    /// Centre, cm⁻¹.
    centre_cm: [f64; BINS],
    /// [`MT_CKD`]'s self coefficient, its temperature exponent and its
    /// foreign coefficient at the centre ([`mt_ckd_at`]).
    continuum: [(f64, f64, f64); BINS],
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
    };
    for i in 0..BINS {
        let (lo, hi) = bin_edges(i);
        let nu = 0.5 * (lo + hi);
        table.centre_cm[i] = nu;
        table.continuum[i] = mt_ckd_at(nu);
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
/// the continua of water and CO₂ and the collision-induced pairs, each when the level
/// holds it.
struct SmoothRates {
    /// The level's temperature, K.
    t: f64,
    /// Water's continuum: the vapour, bar; `ln (296 K / T)`; the other
    /// gases, bar; and the factor of the absorption per molecule.
    continuum: Option<(f64, f64, f64, f64)>,
    /// Each pair of [`PAIRS`] the level holds: the column of its
    /// temperatures below the level's, the share of the next
    /// ([`Pair::column`]), and the factor of its coefficient
    /// ([`Pair::factor`]).
    pairs: [Option<((usize, f64), f64)>; PAIR_COUNT],
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
        let pairs = std::array::from_fn(|i| {
            let pair = &PAIRS[i];
            absorbs(i)
                .then(|| pair.factor(level))
                .flatten()
                .map(|factor| (pair.column(t), factor))
        });
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
            co2_continuum,
        }
    }

    /// The smooth absorption per `ln p` of every bin (`reference`'s
    /// `SmoothRates::bin`) into `out`, in single precision
    /// with [`super::fast_math`]'s `exp` (`tanh y = 1 − 2 / (e^{2y} + 1)`).
    fn fast(&self, out: &mut [f32]) {
        use super::fast_math::exp;
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
        for (pair, rates) in PAIRS.iter().zip(&self.pairs) {
            if let Some((column, scale)) = *rates {
                for (row, (bin, _)) in pair.cells.iter().enumerate() {
                    if let Some(o) = out.get_mut(*bin) {
                        *o += (pair.coefficient(row, column) * scale) as f32;
                    }
                }
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
/// [`fast_level`]): each gas's lines ([`LineGas::ALL`]) at each bin and
/// g-point (index `bin · G + g`) and the smooth part of each bin, per
/// unit `ln p`, and their logarithms, in single precision.
#[derive(Clone)]
pub(crate) struct FastLevel {
    /// Each gas's lines, in the order of [`LineGas::ALL`], and their
    /// logarithms.
    pub(crate) lines: [[f32; POINTS]; LINE_GASES],
    pub(crate) lines_ln: [[f32; POINTS]; LINE_GASES],
    /// The continua of water and CO₂ and the collision-induced pairs, per bin, and their
    /// logarithms.
    pub(crate) smooth: [f32; BINS],
    pub(crate) smooth_ln: [f32; BINS],
}

impl Default for FastLevel {
    fn default() -> Self {
        Self {
            lines: [[0.0; POINTS]; LINE_GASES],
            lines_ln: [[0.0; POINTS]; LINE_GASES],
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
    let valid = p > 0.0 && t > 0.0 && level.gravity_m_s2 > 0.0;
    // Each gas's lines ([`Table::lines`]), then their logarithms.
    let mut own_ln = [0.0; POINTS];
    for (k, gas) in LineGas::ALL.into_iter().enumerate() {
        let bar = gas.bar(level);
        let (sum, by_air) = (&mut out.lines[k][..m], &mut out.lines_ln[k][..m]);
        if valid && bar > 0.0 {
            table.lines(
                level,
                bar,
                gas.molar_kg(),
                gas.sets(),
                0..m,
                by_air,
                &mut own_ln[..m],
                sum,
            );
            ln_all(&out.lines[k][..m], &mut out.lines_ln[k][..m]);
        } else {
            sum.fill(0.0);
        }
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

/// Vertical optical depth per bin above the top of a column, of what is
/// smooth: `reference::depth_above`'s, half the top level's smooth rate
/// `smooth` (every smooth absorber grows as `p²` in a uniform air above
/// it).
pub(crate) fn smooth_above(smooth: f64) -> f64 {
    0.5 * smooth
}

/// Black-body flux, W/m², of every band at `t_k` into `out`: the bins'
/// exact integrals (`reference::planck_bin_w_m2`) and the tail's rest of `σT⁴` past
/// [`SPECTRUM_TOP_CM`], each Planck integral taken once per edge.
pub(crate) fn planck_spectrum(t_k: f64, out: &mut [f64; BANDS]) {
    planck_spectrum_upto(t_k, BINS, out);
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

/// Black-body flux, W/m², at `t_k` between each two of the ascending
/// `edges_cm` (cm⁻¹) into `out[j]`, `j` from 0, and past the last edge
/// into `out[edges_cm.len() − 1]`: the exact integrals of
/// [`planck_spectrum`] on edges of the caller's (`out` as long as
/// `edges_cm`), each Planck integral taken once per edge.
pub(crate) fn planck_over(t_k: f64, edges_cm: &[f64], out: &mut [f64]) {
    out.fill(0.0);
    if t_k <= 0.0 || edges_cm.is_empty() {
        return;
    }
    let black = STEFAN_BOLTZMANN * t_k.powi(4) / PLANCK_TOTAL;
    let x = |cm: f64| SECOND_RADIATION_CONSTANT_CM_K * cm / t_k;
    // The head integral from 0 under the switch, the tail integral to
    // infinity over it, as `planck_spectrum_upto` takes them.
    let edge = |x: f64| {
        if x >= SERIES_SWITCH {
            planck_tail(x)
        } else {
            planck_head(x)
        }
    };
    let mut x_lo = x(edges_cm[0]);
    let mut lo = edge(x_lo);
    for (j, &cm) in edges_cm.iter().enumerate().skip(1) {
        let x_hi = x(cm);
        let hi = edge(x_hi);
        let integral = if x_lo >= SERIES_SWITCH {
            lo - hi
        } else if x_hi < SERIES_SWITCH {
            hi - lo
        } else {
            (PLANCK_TOTAL - hi) - lo
        };
        out[j - 1] = black * integral.max(0.0);
        (x_lo, lo) = (x_hi, hi);
    }
    let rest = if x_lo >= SERIES_SWITCH {
        lo
    } else {
        PLANCK_TOTAL - lo
    };
    out[edges_cm.len() - 1] = black * rest.max(0.0);
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
