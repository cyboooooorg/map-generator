//! The thermal infrared, bin by bin: how much each gas absorbs in each
//! bin of wavenumber, and how much a black body emits there (issue #523,
//! the longwave band model of lot 3C-2).
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
//!   `tools/opacity-table` (provenance in `crates/mapgen/data/OPACITY.md`).
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
//! - **CO₂–CO₂ collision-induced absorption:** HITRAN's CIA file (Karman
//!   et al. 2019), binned ([`CO2_CIA`]), growing as the CO₂ density
//!   squared, not broadened.
//! - **Methane:** its grey stand-in for Titan's collision-induced
//!   absorption ([`super::climate`]'s methane law), over
//!   0-[`METHANE_TOP_CM`] cm⁻¹ only.
//!
//! The continua, the pairs and methane are smooth across a bin: each is
//! one optical depth per bin, added to every g-point ([`FastLevel`]). The
//! double-precision sums the tests hold the fast ones to are in
//! `reference`.
//! CO₂ and water overlap at random within a bin (Lacis & Oinas 1991,
//! *JGR* 96, 9027): the transmission of a path is the product of each
//! gas's transmission, each the weighted sum over its g-points. The
//! column masses come from `p / g` with the body's gravity
//! ([`Level::gravity_m_s2`]).

use std::sync::LazyLock;

use super::Gases;
use super::climate::methane_depth;
pub(crate) use super::ktable::G;
use super::ktable::{CO2_SELF_SET, CO2_SET, Table, WATER_FOREIGN_SET, WATER_SELF_SET};

#[cfg(test)]
pub(crate) mod reference;

/// Width of a spectral bin, cm⁻¹. Halving it moves the CO₂ doubling
/// forcing of an Earth column by 0.006 W/m² (the test
/// `halving_bins_keeps_the_forcing`, which allows 0.1), so the bins need
/// not be finer; the papers fit their bands on 10 cm⁻¹ averages
/// (Jeevanjee & Fueglistaler 2020, §3a).
pub(crate) const BIN_WIDTH_CM: f64 = 20.0;

/// Upper end of the spectrum, cm⁻¹: a black body at 640 K, the hottest
/// steam column the climate solves, emits 99.6 % of its flux below it
/// (the test `planck_bins_sum_to_sigma_t4`).
pub(crate) const SPECTRUM_TOP_CM: f64 = 5000.0;

/// Number of bins over 0-[`SPECTRUM_TOP_CM`].
pub(crate) const BINS: usize = (SPECTRUM_TOP_CM / BIN_WIDTH_CM) as usize;

/// Diffusivity factor of the two-stream emission, `1/cos θ̄`: Elsasser's
/// 1.66 (Elsasser 1942; the spectral models of Koll, Jeevanjee & Lutsko
/// 2023 use `cos θ̄ = 3/5`, Table 1). Each bin's optical depth from
/// [`fast_level`] is vertical; the column multiplies it by this.
pub(crate) const DIFFUSIVITY: f64 = 1.66;

/// One level of a column, as the spectrum reads it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Level {
    /// Temperature, K.
    pub(crate) temperature_k: f64,
    /// Partial pressures at the level, bar; their sum is its pressure.
    pub(crate) gases: Gases,
    /// Surface gravity of the body, m/s²: a column of `p` bar weighs
    /// `p / g` per unit area.
    pub(crate) gravity_m_s2: f64,
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
            return N2_KG_MOL;
        }
        (g.n2.max(0.0) * N2_KG_MOL
            + g.co2.max(0.0) * CO2_KG_MOL
            + g.h2o.max(0.0) * H2O_KG_MOL
            + g.ch4.max(0.0) * CH4_KG_MOL
            + g.nh3.max(0.0) * NH3_KG_MOL
            + g.so2.max(0.0) * SO2_KG_MOL
            + g.o2.max(0.0) * O2_KG_MOL)
            / p
    }

    /// Mass of a gas of molar mass `molar_kg` and partial pressure
    /// `partial_bar`, kg/m², per unit `ln p` of the column at this level:
    /// `(p_i M_i / (p M̄)) p / g`, its mass share of the air above.
    pub(crate) fn mass_per_ln_p(&self, partial_bar: f64, molar_kg: f64) -> f64 {
        partial_bar.max(0.0) * PA_PER_BAR * molar_kg / (self.molar_mass_kg() * self.gravity_m_s2)
    }
}

/// Pascals per bar.
pub(crate) const PA_PER_BAR: f64 = 1e5;

/// Avogadro constant, 1/mol (CODATA 2018, exact).
pub(crate) const AVOGADRO: f64 = 6.022_140_76e23;

/// Boltzmann constant, J/K (CODATA 2018, exact).
const BOLTZMANN: f64 = 1.380_649e-23;

/// Stefan–Boltzmann constant, W/(m²·K⁴) (CODATA 2018, exact).
const SIGMA: f64 = 5.670_374_419e-8;

/// Second radiation constant `h c / k`, cm·K (CODATA 2018,
/// 1.438 776 877e-2 m·K).
pub(crate) const C2_CM_K: f64 = 1.438_776_877;

/// Molar masses, kg/mol, from the IUPAC standard atomic weights (H
/// 1.008, C 12.011, N 14.007, O 15.999, S 32.06).
const N2_KG_MOL: f64 = 28.014e-3;
pub(crate) const CO2_KG_MOL: f64 = 44.009e-3;
pub(crate) const H2O_KG_MOL: f64 = 18.015e-3;
const CH4_KG_MOL: f64 = 16.043e-3;
const NH3_KG_MOL: f64 = 17.031e-3;
const SO2_KG_MOL: f64 = 64.058e-3;
const O2_KG_MOL: f64 = 31.998e-3;

/// Edges of bin `i`, cm⁻¹.
pub(crate) fn bin_edges(i: usize) -> (f64, f64) {
    edges_of(BIN_WIDTH_CM, i)
}

/// Edges of bin `i` on bins `width` cm⁻¹ wide.
fn edges_of(width: f64, i: usize) -> (f64, f64) {
    (i as f64 * width, (i + 1) as f64 * width)
}

/// `∫₀^∞ x³/(eˣ − 1) dx = π⁴/15`.
const PLANCK_TOTAL: f64 =
    std::f64::consts::PI * std::f64::consts::PI * std::f64::consts::PI * std::f64::consts::PI
        / 15.0;

/// `x` under which the Planck integral is summed from 0 ([`planck_head`])
/// and over which from infinity ([`planck_tail`]).
const SERIES_SWITCH: f64 = 2.0;

/// Bernoulli numbers `B₂ … B₂₀`.
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

/// `1/n` for `n` from 1 to 64, the terms of [`planck_tail`].
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
/// (`crates/mapgen/data/opacity.bin`, see [`super::ktable`]), on the
/// model's bins.
pub(crate) static OPACITY: LazyLock<Table> = LazyLock::new(|| {
    let table = Table::parse(include_bytes!("../../data/opacity.bin"))
        .expect("crates/mapgen/data/opacity.bin is a table of 8 g-points");
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

/// Spacing of [`MT_CKD`]'s nodes, cm⁻¹.
const MT_CKD_STEP_CM: f64 = 50.0;

/// Reference temperature (K) and pressure (bar) of [`MT_CKD`]: 296 K and
/// 1013 mbar (the file's `ref_temp` and `ref_press`).
pub(crate) const MT_CKD_REFERENCE_K: f64 = 296.0;
pub(crate) const MT_CKD_REFERENCE_BAR: f64 = 1.013;

/// The water vapour continuum of MT_CKD 4.3 (Mlawer et al. 2012,
/// doi:10.1098/rsta.2011.0295; file `absco-ref_wv-mt-ckd.nc` of
/// github.com/AER-RC/MT_CKD_H2O, read on 2026-11-18), every
/// [`MT_CKD_STEP_CM`] cm⁻¹ from 0 to 5000: wavenumber (cm⁻¹), the self
/// continuum coefficient at 296 K, its temperature exponent, and the
/// foreign continuum coefficient at 296 K (cm²/molecule per cm⁻¹; the
/// file's 10 cm⁻¹ grid taken at its nodes). Between nodes the
/// coefficients are interpolated in their logarithm. The 50 cm⁻¹ nodes
/// move Earth's outgoing radiation by 0.04 W/m² against the full grid
/// (task 7.1 report). The optical depth they give is
/// [`water_continuum_depth`]'s. MT_CKD holds from cold air to steam; its own validity
/// is checked against laboratory and field data at Earth's temperatures
/// (Mlawer et al. 2012).
const MT_CKD: [(f64, f64, f64, f64); 101] = [
    (0.0, 2.877e-21, 6.413, 8.563e-23),
    (50.0, 1.863e-21, 5.557, 1.376e-22),
    (100.0, 9.882e-22, 4.481, 1.063e-22),
    (150.0, 5.740e-22, 3.548, 5.243e-23),
    (200.0, 3.031e-22, 2.756, 2.197e-23),
    (250.0, 1.509e-22, 2.100, 7.131e-24),
    (300.0, 7.544e-23, 1.575, 2.686e-24),
    (350.0, 3.974e-23, 1.431, 8.588e-25),
    (400.0, 2.221e-23, 1.628, 2.343e-25),
    (450.0, 1.287e-23, 1.976, 1.181e-25),
    (500.0, 7.568e-24, 2.330, 5.720e-26),
    (550.0, 4.471e-24, 2.675, 2.851e-26),
    (600.0, 2.644e-24, 3.058, 1.267e-26),
    (650.0, 1.564e-24, 3.527, 5.714e-27),
    (700.0, 9.940e-25, 3.721, 3.208e-27),
    (750.0, 6.702e-25, 4.187, 2.072e-27),
    (800.0, 4.617e-25, 5.058, 1.327e-27),
    (850.0, 3.434e-25, 4.953, 8.354e-28),
    (900.0, 2.597e-25, 5.276, 5.481e-28),
    (950.0, 1.847e-25, 5.360, 3.599e-28),
    (1000.0, 1.331e-25, 5.636, 2.451e-28),
    (1050.0, 1.013e-25, 5.688, 1.713e-28),
    (1100.0, 8.149e-26, 5.727, 1.332e-28),
    (1150.0, 6.973e-26, 5.979, 1.598e-28),
    (1200.0, 7.009e-26, 6.112, 4.200e-28),
    (1250.0, 9.220e-26, 5.901, 1.170e-27),
    (1300.0, 1.813e-25, 3.173, 2.910e-27),
    (1350.0, 3.671e-25, 2.273, 9.987e-27),
    (1400.0, 7.331e-25, 2.309, 4.572e-26),
    (1450.0, 1.341e-24, 2.737, 1.425e-25),
    (1500.0, 2.431e-24, 3.011, 3.023e-25),
    (1550.0, 3.554e-24, 3.388, 4.011e-25),
    (1600.0, 3.858e-24, 3.864, 2.678e-25),
    (1650.0, 3.618e-24, 3.821, 3.459e-25),
    (1700.0, 2.785e-24, 3.490, 2.916e-25),
    (1750.0, 1.689e-24, 3.178, 1.373e-25),
    (1800.0, 9.237e-25, 3.116, 6.110e-26),
    (1850.0, 4.914e-25, 3.001, 2.486e-26),
    (1900.0, 2.598e-25, 2.908, 8.302e-27),
    (1950.0, 1.423e-25, 2.948, 3.198e-27),
    (2000.0, 7.720e-26, 3.260, 1.294e-27),
    (2050.0, 4.500e-26, 3.616, 5.583e-28),
    (2100.0, 2.406e-26, 3.966, 2.417e-28),
    (2150.0, 1.444e-26, 4.269, 1.084e-28),
    (2200.0, 1.035e-26, 4.709, 4.975e-29),
    (2250.0, 7.798e-27, 5.147, 2.469e-29),
    (2300.0, 6.143e-27, 5.566, 1.201e-29),
    (2350.0, 5.041e-27, 5.843, 5.509e-30),
    (2400.0, 4.293e-27, 6.059, 2.599e-30),
    (2450.0, 3.775e-27, 6.180, 1.273e-30),
    (2500.0, 3.411e-27, 6.209, 7.313e-31),
    (2550.0, 3.158e-27, 6.085, 1.174e-30),
    (2600.0, 3.003e-27, 5.930, 5.733e-30),
    (2650.0, 3.015e-27, 5.739, 1.503e-29),
    (2700.0, 3.176e-27, 5.558, 1.852e-29),
    (2750.0, 2.955e-27, 5.294, 1.938e-29),
    (2800.0, 2.851e-27, 4.940, 1.635e-29),
    (2850.0, 2.948e-27, 4.435, 8.433e-30),
    (2900.0, 3.276e-27, 3.803, 1.179e-29),
    (2950.0, 4.081e-27, 3.366, 6.415e-29),
    (3000.0, 5.953e-27, 2.983, 3.390e-28),
    (3050.0, 9.350e-27, 2.634, 8.381e-28),
    (3100.0, 1.288e-26, 2.941, 1.200e-27),
    (3150.0, 1.373e-26, 3.490, 8.601e-28),
    (3200.0, 1.349e-26, 3.722, 9.989e-28),
    (3250.0, 1.336e-26, 3.536, 1.000e-27),
    (3300.0, 1.415e-26, 3.103, 6.438e-28),
    (3350.0, 1.863e-26, 2.559, 4.709e-28),
    (3400.0, 3.103e-26, 2.412, 6.567e-28),
    (3450.0, 5.651e-26, 2.287, 1.379e-27),
    (3500.0, 1.047e-25, 2.135, 3.562e-27),
    (3550.0, 2.020e-25, 2.075, 9.563e-27),
    (3600.0, 3.896e-25, 2.415, 3.197e-26),
    (3650.0, 7.281e-25, 3.112, 8.439e-26),
    (3700.0, 1.102e-24, 3.513, 1.108e-25),
    (3750.0, 1.400e-24, 3.568, 1.210e-25),
    (3800.0, 1.241e-24, 3.897, 1.230e-25),
    (3850.0, 8.890e-25, 3.503, 1.090e-25),
    (3900.0, 5.045e-25, 2.764, 3.400e-26),
    (3950.0, 2.485e-25, 2.410, 7.000e-27),
    (4000.0, 1.282e-25, 2.871, 2.372e-27),
    (4050.0, 6.922e-26, 3.339, 1.084e-27),
    (4100.0, 3.788e-26, 3.623, 4.920e-28),
    (4150.0, 2.090e-26, 3.961, 2.641e-28),
    (4200.0, 1.155e-26, 4.364, 1.578e-28),
    (4250.0, 6.418e-27, 4.815, 1.010e-28),
    (4300.0, 4.443e-27, 5.245, 6.868e-29),
    (4350.0, 3.395e-27, 5.632, 4.855e-29),
    (4400.0, 2.661e-27, 5.953, 3.536e-29),
    (4450.0, 2.143e-27, 6.183, 2.636e-29),
    (4500.0, 1.773e-27, 6.263, 2.022e-29),
    (4550.0, 1.514e-27, 6.141, 1.677e-29),
    (4600.0, 1.354e-27, 6.050, 1.569e-29),
    (4650.0, 1.227e-27, 6.005, 1.233e-29),
    (4700.0, 1.098e-27, 5.833, 9.672e-30),
    (4750.0, 1.036e-27, 5.542, 7.944e-30),
    (4800.0, 1.028e-27, 5.222, 6.499e-30),
    (4850.0, 1.075e-27, 4.810, 5.537e-30),
    (4900.0, 1.199e-27, 4.334, 4.956e-30),
    (4950.0, 1.436e-27, 3.831, 7.451e-30),
    (5000.0, 2.645e-27, 3.367, 1.670e-29),
];

/// Temperatures, K, of the columns of [`CO2_CIA`].
const CO2_CIA_K: [f64; 4] = [200.0, 300.0, 450.0, 800.0];

/// Width of the cells of [`CO2_CIA`], cm⁻¹.
pub(crate) const CO2_CIA_CELL_CM: f64 = 20.0;

/// Upper end, cm⁻¹, of the last band of [`CO2_CIA`].
pub(crate) const CO2_CIA_TOP_CM: f64 = 4500.0;

/// CO₂–CO₂ collision-induced absorption, binned from HITRAN's
/// `CO2-CO2_2024.cia` (hitran.org/cia; Karman et al. 2019, *Icarus*
/// 328, 160, doi:10.1016/j.icarus.2019.02.034): the start of each
/// [`CO2_CIA_CELL_CM`] cm⁻¹ cell and the binary absorption coefficient
/// averaged over it, cm⁵/molecule², at each of [`CO2_CIA_K`]. Each
/// region takes the file's set nearest that temperature:
///
/// - roto-translational band, 1-750 cm⁻¹, sets at 200 to 800 K;
/// - the 1285/1388 cm⁻¹ dimer bands, 1120-1850 cm⁻¹, sets at 192 to
///   500 K;
/// - 2510-2850, 2850-3250 and 3995-4500 cm⁻¹, sets at 221 to 298 K, held
///   at their warmest past it.
///
/// Cells under 1e-50 cm⁵ at every temperature are left out (at Venus's
/// 92 bar and 735 K they would add under 0.013 to the depth per `ln p`).
/// Between temperatures the coefficient is interpolated linearly, and
/// held at the ends.
const CO2_CIA: [(f64, [f64; 4]); 139] = [
    (0.0, [3.812e-44, 1.157e-44, 5.660e-45, 1.882e-45]),
    (20.0, [1.571e-43, 5.311e-44, 2.902e-44, 1.170e-44]),
    (40.0, [1.902e-43, 7.565e-44, 4.757e-44, 2.546e-44]),
    (60.0, [1.486e-43, 7.107e-44, 5.071e-44, 3.635e-44]),
    (80.0, [9.603e-44, 5.572e-44, 4.415e-44, 4.088e-44]),
    (100.0, [5.645e-44, 3.991e-44, 3.454e-44, 3.935e-44]),
    (120.0, [3.146e-44, 2.717e-44, 2.541e-44, 3.399e-44]),
    (140.0, [1.696e-44, 1.793e-44, 1.799e-44, 2.717e-44]),
    (160.0, [8.947e-45, 1.158e-44, 1.241e-44, 2.050e-44]),
    (180.0, [4.646e-45, 7.377e-45, 8.408e-45, 1.480e-44]),
    (200.0, [2.385e-45, 4.648e-45, 5.624e-45, 1.034e-44]),
    (220.0, [1.214e-45, 2.904e-45, 3.724e-45, 7.034e-45]),
    (240.0, [6.135e-46, 1.803e-45, 2.447e-45, 4.688e-45]),
    (260.0, [3.084e-46, 1.114e-45, 1.599e-45, 3.072e-45]),
    (280.0, [1.543e-46, 6.850e-46, 1.039e-45, 1.985e-45]),
    (300.0, [7.690e-47, 4.198e-46, 6.723e-46, 1.269e-45]),
    (320.0, [3.821e-47, 2.565e-46, 4.336e-46, 8.033e-46]),
    (340.0, [1.893e-47, 1.563e-46, 2.788e-46, 5.045e-46]),
    (360.0, [9.356e-48, 9.504e-47, 1.788e-46, 3.147e-46]),
    (380.0, [4.615e-48, 5.768e-47, 1.144e-46, 1.951e-46]),
    (400.0, [2.272e-48, 3.494e-47, 7.308e-47, 1.204e-46]),
    (420.0, [1.117e-48, 2.114e-47, 4.659e-47, 7.397e-47]),
    (440.0, [5.482e-49, 1.277e-47, 2.966e-47, 4.527e-47]),
    (460.0, [2.687e-49, 7.703e-48, 1.885e-47, 2.761e-47]),
    (480.0, [1.316e-49, 4.642e-48, 1.197e-47, 1.679e-47]),
    (500.0, [6.433e-50, 2.794e-48, 7.592e-48, 1.018e-47]),
    (520.0, [3.143e-50, 1.681e-48, 4.810e-48, 6.158e-48]),
    (540.0, [1.534e-50, 1.010e-48, 3.045e-48, 3.717e-48]),
    (560.0, [7.482e-51, 6.064e-49, 1.926e-48, 2.239e-48]),
    (580.0, [3.646e-51, 3.638e-49, 1.217e-48, 1.347e-48]),
    (600.0, [1.776e-51, 2.182e-49, 7.683e-49, 8.084e-49]),
    (620.0, [8.642e-52, 1.307e-49, 4.849e-49, 4.846e-49]),
    (640.0, [4.203e-52, 7.829e-50, 3.058e-49, 2.901e-49]),
    (660.0, [2.043e-52, 4.686e-50, 1.927e-49, 1.735e-49]),
    (680.0, [9.925e-53, 2.803e-50, 1.214e-49, 1.036e-49]),
    (700.0, [4.820e-53, 1.676e-50, 7.642e-50, 6.181e-50]),
    (720.0, [2.339e-53, 1.002e-50, 4.809e-50, 3.684e-50]),
    (740.0, [6.690e-54, 3.376e-51, 1.687e-50, 1.238e-50]),
    (1120.0, [0.000e+00, 0.000e+00, 1.135e-46, 1.215e-46]),
    (1140.0, [1.274e-46, 8.887e-47, 2.749e-46, 2.941e-46]),
    (1160.0, [5.660e-46, 5.944e-46, 6.640e-46, 7.106e-46]),
    (1180.0, [1.043e-45, 1.160e-45, 1.665e-45, 1.782e-45]),
    (1200.0, [2.025e-45, 2.849e-45, 3.896e-45, 4.169e-45]),
    (1220.0, [5.103e-45, 6.276e-45, 8.201e-45, 8.770e-45]),
    (1240.0, [1.713e-44, 1.418e-44, 1.550e-44, 1.656e-44]),
    (1260.0, [5.362e-44, 3.030e-44, 2.734e-44, 2.915e-44]),
    (1280.0, [9.175e-44, 4.213e-44, 3.542e-44, 3.770e-44]),
    (1300.0, [3.997e-44, 2.606e-44, 2.490e-44, 2.655e-44]),
    (1320.0, [2.989e-44, 2.211e-44, 2.228e-44, 2.376e-44]),
    (1340.0, [3.426e-44, 2.430e-44, 2.284e-44, 2.436e-44]),
    (1360.0, [6.647e-44, 3.627e-44, 2.897e-44, 3.088e-44]),
    (1380.0, [1.104e-43, 5.018e-44, 3.661e-44, 3.897e-44]),
    (1400.0, [6.070e-44, 3.588e-44, 2.796e-44, 2.981e-44]),
    (1420.0, [2.496e-44, 2.010e-44, 1.802e-44, 1.924e-44]),
    (1440.0, [1.065e-44, 1.037e-44, 1.075e-44, 1.149e-44]),
    (1460.0, [5.707e-45, 5.542e-45, 6.495e-45, 6.947e-45]),
    (1480.0, [3.120e-45, 3.106e-45, 4.062e-45, 4.346e-45]),
    (1500.0, [1.805e-45, 1.766e-45, 2.447e-45, 2.618e-45]),
    (1520.0, [9.654e-46, 9.615e-46, 1.414e-45, 1.513e-45]),
    (1540.0, [5.165e-46, 5.236e-46, 7.608e-46, 8.139e-46]),
    (1560.0, [2.763e-46, 2.851e-46, 4.055e-46, 4.338e-46]),
    (1580.0, [1.478e-46, 1.553e-46, 2.183e-46, 2.336e-46]),
    (1600.0, [7.909e-47, 8.454e-47, 1.155e-46, 1.236e-46]),
    (1620.0, [4.231e-47, 4.604e-47, 6.081e-47, 6.504e-47]),
    (1640.0, [2.264e-47, 2.507e-47, 3.194e-47, 3.416e-47]),
    (1660.0, [1.211e-47, 1.365e-47, 1.676e-47, 1.792e-47]),
    (1680.0, [6.479e-48, 7.433e-48, 8.791e-48, 9.402e-48]),
    (1700.0, [3.466e-48, 4.048e-48, 4.609e-48, 4.929e-48]),
    (1720.0, [1.854e-48, 2.204e-48, 2.416e-48, 2.583e-48]),
    (1740.0, [9.921e-49, 1.200e-48, 1.266e-48, 1.354e-48]),
    (1760.0, [5.308e-49, 6.536e-49, 6.634e-49, 7.093e-49]),
    (1780.0, [2.840e-49, 3.559e-49, 3.475e-49, 3.715e-49]),
    (1800.0, [1.519e-49, 1.938e-49, 0.000e+00, 0.000e+00]),
    (1820.0, [8.127e-50, 1.055e-49, 0.000e+00, 0.000e+00]),
    (1840.0, [2.511e-50, 3.307e-50, 0.000e+00, 0.000e+00]),
    (2500.0, [1.354e-47, 2.868e-47, 2.868e-47, 2.868e-47]),
    (2520.0, [1.013e-45, 5.247e-46, 5.247e-46, 5.247e-46]),
    (2540.0, [2.209e-45, 1.018e-45, 1.018e-45, 1.018e-45]),
    (2560.0, [1.047e-45, 5.783e-46, 5.783e-46, 5.783e-46]),
    (2580.0, [1.880e-46, 1.476e-46, 1.476e-46, 1.476e-46]),
    (2600.0, [1.481e-46, 1.364e-46, 1.364e-46, 1.364e-46]),
    (2620.0, [6.476e-46, 4.306e-46, 4.306e-46, 4.306e-46]),
    (2640.0, [2.878e-45, 1.500e-45, 1.500e-45, 1.500e-45]),
    (2660.0, [6.037e-45, 2.620e-45, 2.620e-45, 2.620e-45]),
    (2680.0, [3.251e-45, 1.648e-45, 1.648e-45, 1.648e-45]),
    (2700.0, [9.399e-46, 6.371e-46, 6.371e-46, 6.371e-46]),
    (2720.0, [1.076e-46, 1.018e-46, 1.018e-46, 1.018e-46]),
    (2740.0, [6.471e-47, 2.825e-47, 2.825e-47, 2.825e-47]),
    (2760.0, [3.334e-46, 2.026e-46, 2.026e-46, 2.026e-46]),
    (2780.0, [1.116e-45, 4.422e-46, 4.422e-46, 4.422e-46]),
    (2800.0, [8.358e-46, 4.216e-46, 4.216e-46, 4.216e-46]),
    (2820.0, [2.351e-46, 1.396e-46, 1.396e-46, 1.396e-46]),
    (2840.0, [8.837e-47, 1.949e-47, 1.949e-47, 1.949e-47]),
    (2860.0, [9.082e-47, 0.000e+00, 0.000e+00, 0.000e+00]),
    (2880.0, [2.409e-46, 4.985e-47, 4.985e-47, 4.985e-47]),
    (2900.0, [1.348e-46, 1.935e-46, 1.935e-46, 1.935e-46]),
    (2920.0, [6.182e-46, 4.039e-46, 4.039e-46, 4.039e-46]),
    (2940.0, [1.170e-45, 8.313e-46, 8.313e-46, 8.313e-46]),
    (2960.0, [2.546e-45, 1.614e-45, 1.614e-45, 1.614e-45]),
    (2980.0, [4.704e-45, 2.672e-45, 2.672e-45, 2.672e-45]),
    (3000.0, [7.759e-45, 3.946e-45, 3.946e-45, 3.946e-45]),
    (3020.0, [6.361e-45, 2.973e-45, 2.973e-45, 2.973e-45]),
    (3040.0, [3.751e-45, 1.788e-45, 1.788e-45, 1.788e-45]),
    (3060.0, [1.948e-45, 9.725e-46, 9.725e-46, 9.725e-46]),
    (3080.0, [9.803e-46, 5.136e-46, 5.136e-46, 5.136e-46]),
    (3100.0, [3.940e-46, 2.972e-46, 2.972e-46, 2.972e-46]),
    (3120.0, [2.231e-46, 1.792e-46, 1.792e-46, 1.792e-46]),
    (3140.0, [2.030e-46, 8.528e-47, 8.528e-47, 8.528e-47]),
    (3160.0, [5.367e-47, 1.596e-47, 1.596e-47, 1.596e-47]),
    (3180.0, [0.000e+00, 1.122e-46, 1.122e-46, 1.122e-46]),
    (3200.0, [0.000e+00, 1.011e-46, 1.011e-46, 1.011e-46]),
    (3220.0, [0.000e+00, 2.749e-47, 2.749e-47, 2.749e-47]),
    (3240.0, [0.000e+00, 5.411e-49, 5.411e-49, 5.411e-49]),
    (3980.0, [4.865e-48, 8.563e-49, 8.563e-49, 8.563e-49]),
    (4000.0, [2.350e-47, 1.712e-47, 1.712e-47, 1.712e-47]),
    (4020.0, [2.564e-47, 3.418e-47, 3.418e-47, 3.418e-47]),
    (4040.0, [6.623e-47, 6.789e-47, 6.789e-47, 6.789e-47]),
    (4060.0, [1.111e-46, 7.540e-47, 7.540e-47, 7.540e-47]),
    (4080.0, [5.664e-47, 4.485e-47, 4.485e-47, 4.485e-47]),
    (4100.0, [2.174e-47, 2.244e-47, 2.244e-47, 2.244e-47]),
    (4120.0, [1.949e-47, 1.227e-47, 1.227e-47, 1.227e-47]),
    (4140.0, [9.308e-48, 7.225e-48, 7.225e-48, 7.225e-48]),
    (4160.0, [9.199e-48, 5.759e-48, 5.759e-48, 5.759e-48]),
    (4180.0, [0.000e+00, 1.007e-47, 1.007e-47, 1.007e-47]),
    (4200.0, [0.000e+00, 1.296e-47, 1.296e-47, 1.296e-47]),
    (4220.0, [0.000e+00, 1.986e-47, 1.986e-47, 1.986e-47]),
    (4240.0, [0.000e+00, 2.142e-47, 2.142e-47, 2.142e-47]),
    (4260.0, [0.000e+00, 3.144e-47, 3.144e-47, 3.144e-47]),
    (4280.0, [0.000e+00, 2.490e-47, 2.490e-47, 2.490e-47]),
    (4300.0, [0.000e+00, 1.762e-47, 1.762e-47, 1.762e-47]),
    (4320.0, [0.000e+00, 1.676e-47, 1.676e-47, 1.676e-47]),
    (4340.0, [0.000e+00, 2.314e-47, 2.314e-47, 2.314e-47]),
    (4360.0, [0.000e+00, 3.534e-47, 3.534e-47, 3.534e-47]),
    (4380.0, [0.000e+00, 3.677e-47, 3.677e-47, 3.677e-47]),
    (4400.0, [0.000e+00, 2.542e-47, 2.542e-47, 2.542e-47]),
    (4420.0, [0.000e+00, 1.712e-47, 1.712e-47, 1.712e-47]),
    (4440.0, [0.000e+00, 1.070e-47, 1.070e-47, 1.070e-47]),
    (4460.0, [0.000e+00, 5.115e-48, 5.115e-48, 5.115e-48]),
    (4480.0, [0.000e+00, 6.184e-48, 6.184e-48, 6.184e-48]),
];

/// Optical depth per `ln p` of CO₂–CO₂ pairs at `nu` cm⁻¹:
/// `k(ν, T) n²` per metre of air (`n` the CO₂ number density), times the
/// air's thickness per `ln p`, `k_B T / (m̄ g)`. Not broadened.
pub(crate) fn co2_pairs(nu: f64, level: &Level) -> f64 {
    let co2 = level.gases.co2;
    if co2 <= 0.0 {
        return 0.0;
    }
    let cell = (nu / CO2_CIA_CELL_CM).floor() * CO2_CIA_CELL_CM;
    let Ok(row) = CO2_CIA.binary_search_by(|(start, _)| start.total_cmp(&cell)) else {
        return 0.0;
    };
    let values = CO2_CIA[row].1;
    let t = level.temperature_k;
    let k_cm5 = if t <= CO2_CIA_K[0] {
        values[0]
    } else if t >= CO2_CIA_K[3] {
        values[3]
    } else {
        let j = CO2_CIA_K.iter().rposition(|&k| k <= t).unwrap_or(0).min(2);
        let f = (t - CO2_CIA_K[j]) / (CO2_CIA_K[j + 1] - CO2_CIA_K[j]);
        values[j] * (1.0 - f) + values[j + 1] * f
    };
    let density_m3 = co2 * PA_PER_BAR / (BOLTZMANN * t);
    let thickness_m = BOLTZMANN * t * AVOGADRO / (level.molar_mass_kg() * level.gravity_m_s2);
    k_cm5 * 1e-10 * density_m3 * density_m3 * thickness_m
}

/// The radiation term `R(ν, T) = ν tanh(c₂ν / 2T)` of MT_CKD's continua,
/// cm⁻¹, at `nu` cm⁻¹ and `t_k`.
pub(crate) fn radiation_term(nu: f64, t_k: f64) -> f64 {
    nu * (0.5 * C2_CM_K * nu / t_k).tanh()
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
/// foreign part as `e (p − e)`. The shortwave's binned coefficients take
/// it too (`shortwave::water_continuum`).
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
    let molecules_m2 = e * PA_PER_BAR * AVOGADRO / (level.molar_mass_kg() * level.gravity_m_s2);
    per_molecule_cm2 * 1e-4 * molecules_m2
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
/// `crates/mapgen/data/OPACITY.md`). It is broadened by the whole air,
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
    let molecules_m2 = co2 * PA_PER_BAR * AVOGADRO / (level.molar_mass_kg() * level.gravity_m_s2);
    let density = p / MT_CKD_REFERENCE_BAR * MT_CKD_REFERENCE_K / t;
    let ln_t = (t / t_eff_k).ln();
    co2_continuum_per_molecule(coefficient, nu, t, ln_t) * density * 1e-4 * molecules_m2
}

/// Upper end, cm⁻¹, of methane's grey stand-in: the far infrared of
/// Titan's N₂–N₂ and CH₄–N₂ collision-induced absorption (Courtin,
/// Gautier & McKay 1995, the design's call).
const METHANE_TOP_CM: f64 = 650.0;

/// Bands of the column's sum: the [`BINS`] bins, and the tail past
/// [`SPECTRUM_TOP_CM`] as one more band ([`TAIL`]).
pub(crate) const BANDS: usize = BINS + 1;

/// Index of the tail band, from [`SPECTRUM_TOP_CM`] to infinity. It takes
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
    /// foreign coefficient, interpolated as `reference::water_continuum` does.
    continuum: [(f64, f64, f64); BINS],
    /// Row of [`CO2_CIA`], if the bin has one.
    cia_row: [Option<usize>; BINS],
    /// Share of the bin under [`METHANE_TOP_CM`].
    methane_share: [f64; BINS],
}

/// [`BinTable::continuum`] in single precision. The self and foreign
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
        let x = (nu / MT_CKD_STEP_CM).clamp(0.0, (MT_CKD.len() - 1) as f64);
        let k = (x as usize).min(MT_CKD.len() - 2);
        let f = x - k as f64;
        let (a, b) = (MT_CKD[k], MT_CKD[k + 1]);
        let log_lerp = |u: f64, v: f64| (u.ln() * (1.0 - f) + v.ln() * f).exp();
        table.continuum[i] = (
            log_lerp(a.1, b.1),
            a.2 * (1.0 - f) + b.2 * f,
            log_lerp(a.3, b.3),
        );
        let cell = (nu / CO2_CIA_CELL_CM).floor() * CO2_CIA_CELL_CM;
        table.cia_row[i] = CO2_CIA
            .binary_search_by(|(start, _)| start.total_cmp(&cell))
            .ok();
        table.methane_share[i] = ((METHANE_TOP_CM.min(hi) - lo) / (hi - lo)).clamp(0.0, 1.0);
    }
    table
});

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
    /// Methane's rate per `ln p` ([`super::climate`]'s methane law).
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
            let molecules_m2 =
                e * PA_PER_BAR * AVOGADRO / (level.molar_mass_kg() * level.gravity_m_s2);
            let scale = ratio / MT_CKD_REFERENCE_BAR * 1e-4 * molecules_m2;
            (e, ratio.ln(), others, scale)
        });
        let pairs = (g.co2 > 0.0).then(|| {
            let (j, f) = if t <= CO2_CIA_K[0] {
                (0, 0.0)
            } else if t >= CO2_CIA_K[3] {
                (2, 1.0)
            } else {
                let j = CO2_CIA_K.iter().rposition(|&k| k <= t).unwrap_or(0).min(2);
                (j, (t - CO2_CIA_K[j]) / (CO2_CIA_K[j + 1] - CO2_CIA_K[j]))
            };
            let density_m3 = g.co2 * PA_PER_BAR / (BOLTZMANN * t);
            let thickness_m =
                BOLTZMANN * t * AVOGADRO / (level.molar_mass_kg() * level.gravity_m_s2);
            (j, f, 1e-10 * density_m3 * density_m3 * thickness_m)
        });
        let methane = (g.ch4 > 0.0).then(|| methane_depth(g).1);
        let co2_continuum = (g.co2 > 0.0).then(|| {
            let molecules_m2 =
                g.co2 * PA_PER_BAR * AVOGADRO / (level.molar_mass_kg() * level.gravity_m_s2);
            let density = p / MT_CKD_REFERENCE_BAR * MT_CKD_REFERENCE_K / t;
            (
                (t / OPACITY.co2_continuum_t_eff_k).ln(),
                density * 1e-4 * molecules_m2,
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

    /// [`SmoothRates::bin`] of every bin into `out`, in single precision
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
            let c2_t = (C2_CM_K / self.t) as f32;
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

/// Number of a level's g-points over every bin.
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
            CO2_KG_MOL,
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
            H2O_KG_MOL,
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
    let (total, methane_rate) = methane_depth(&level.gases);
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
    let black = SIGMA * t_k.powi(4) / PLANCK_TOTAL;
    // At each edge, the head integral from 0 under the switch and the
    // tail integral to infinity over it, as `planck_between` takes them.
    let edge = |k: usize| {
        let x = C2_CM_K * k as f64 * BIN_WIDTH_CM / t_k;
        if x >= SERIES_SWITCH {
            planck_tail(x)
        } else {
            planck_head(x)
        }
    };
    let mut x_lo = 0.0;
    let mut lo = edge(0);
    for (i, o) in out[..bins.min(BINS)].iter_mut().enumerate() {
        let x_hi = C2_CM_K * (i + 1) as f64 * BIN_WIDTH_CM / t_k;
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

#[cfg(test)]
mod tests {
    use super::reference::*;
    use super::*;
    use crate::planet::climate::{EARTH_GRAVITY_M_S2, water_saturation_bar};

    /// The table on bins half as wide (10 cm⁻¹) over 0-2500 cm⁻¹, at
    /// 200-300 K only, built by the same tool for
    /// `halving_bins_keeps_the_forcing` (see `OPACITY.md`).
    static HALVED: LazyLock<Table> = LazyLock::new(|| {
        Table::parse(include_bytes!("../../data/opacity-halved.bin")).expect("a table")
    });

    /// Gas constant of dry air, J/(kg·K): the molar gas constant over
    /// 28.964 g/mol (US Standard Atmosphere 1976).
    const DRY_AIR_R: f64 = 287.05;

    /// One level of a fixed profile: pressure (bar), temperature (K) and
    /// water vapour (bar).
    #[derive(Clone, Copy)]
    struct Point {
        pressure_bar: f64,
        temperature_k: f64,
        vapour_bar: f64,
    }

    /// A US-standard-like Earth column: `surface_k` at 1 bar, 6.5 K/km up
    /// to a tropopause at `tropopause_k`, isothermal above, with
    /// `humidity` relative humidity in the troposphere and the
    /// tropopause's mixing ratio above it; 20 levels per decade of
    /// pressure up to 1e-6 bar.
    fn earth_profile(surface_k: f64, tropopause_k: f64, humidity: f64) -> Vec<Point> {
        let surface_bar = 1.0;
        let lapse = 6.5e-3;
        let exponent = DRY_AIR_R * lapse / EARTH_GRAVITY_M_S2;
        let tropopause_bar = surface_bar * (tropopause_k / surface_k).powf(1.0 / exponent);
        let trapped = humidity * water_saturation_bar(tropopause_k) / tropopause_bar;
        let levels = 6 * 20;
        (0..=levels)
            .map(|i| {
                let p = surface_bar * 10f64.powf(-(i as f64) / 20.0);
                let t = (surface_k * (p / surface_bar).powf(exponent)).max(tropopause_k);
                let e = if p < tropopause_bar {
                    trapped * p
                } else {
                    humidity * water_saturation_bar(t)
                };
                Point {
                    pressure_bar: p,
                    temperature_k: t,
                    vapour_bar: e,
                }
            })
            .collect()
    }

    /// Earth's dry air with `co2_ppm` of CO₂, as mole fractions.
    fn earth_air(co2_ppm: f64) -> Gases {
        let co2 = co2_ppm * 1e-6;
        Gases {
            n2: 1.0 - 0.21 - co2,
            co2,
            o2: 0.21,
            ..Gases::default()
        }
    }

    /// A path through the air: the optical depth of each g-point of each
    /// gas, and the smooth part's, summed layer by layer.
    fn add_layer(path: &mut BinAbsorption, a: &BinAbsorption, b: &BinAbsorption, span: f64) {
        for g in 0..G {
            path.co2[g] += log_mean(a.co2[g], b.co2[g]) * span;
            path.water[g] += log_mean(a.water[g], b.water[g]) * span;
        }
        path.smooth += log_mean(a.smooth, b.smooth) * span;
    }

    /// Outgoing longwave radiation, W/m², through the `bins` of `table`,
    /// of a fixed `profile` (from the surface up, uniform steps in
    /// `ln p`) whose dry air has the mole fractions `gases`: per bin, the
    /// two-stream emission with [`DIFFUSIVITY`], the layer-mean Planck
    /// flux and the logarithmic mean of the opacities at a layer's two
    /// levels, CO₂ and water overlapping at random, as the column sums
    /// it. Above the top the air is taken isothermal and its depth is
    /// half the top's rate per `ln p`, exact for absorbers that grow as
    /// `p²` (broadened lines, the continuum, the pairs). The surface is a
    /// black body at the first level's temperature.
    fn olr_bins(
        table: &Table,
        bins: std::ops::Range<usize>,
        profile: &[Point],
        gases: &Gases,
    ) -> f64 {
        let dry = gases.total();
        let level = |point: &Point| {
            let share = (point.pressure_bar - point.vapour_bar).max(0.0) / dry;
            let mut g = Gases {
                n2: gases.n2 * share,
                co2: gases.co2 * share,
                h2o: 0.0,
                ch4: gases.ch4 * share,
                nh3: gases.nh3 * share,
                so2: gases.so2 * share,
                o2: gases.o2 * share,
            };
            g.h2o = point.vapour_bar;
            Level {
                temperature_k: point.temperature_k,
                gases: g,
                gravity_m_s2: EARTH_GRAVITY_M_S2,
            }
        };
        let levels: Vec<Level> = profile.iter().map(level).collect();
        let n = levels.len() - 1;
        let width = table.bin_width_cm;
        let mut total = 0.0;
        for b in bins {
            let (lo, hi) = (b as f64 * width, (b + 1) as f64 * width);
            let rates: Vec<BinAbsorption> =
                levels.iter().map(|l| table.bin_absorption(b, l)).collect();
            let planck: Vec<f64> = levels
                .iter()
                .map(|l| planck_between(lo, hi, l.temperature_k))
                .collect();
            let mut path = BinAbsorption::default();
            add_layer(&mut path, &rates[n], &rates[n], 0.5);
            let mut seen = path.transmission(1.0);
            let mut out = planck[n] * (1.0 - seen);
            for i in (0..n).rev() {
                let span = (profile[i].pressure_bar / profile[i + 1].pressure_bar).ln();
                add_layer(&mut path, &rates[i], &rates[i + 1], span);
                let below = path.transmission(1.0);
                out += 0.5 * (planck[i] + planck[i + 1]) * (seen - below);
                seen = below;
            }
            total += out + planck[0] * seen;
        }
        total
    }

    /// [`olr_bins`] on the model's bins.
    fn olr_fixed_profile(profile: &[Point], gases: &Gases) -> f64 {
        olr_bins(&OPACITY, 0..BINS, profile, gases)
    }

    fn earth_olr(co2_ppm: f64) -> f64 {
        olr_fixed_profile(&earth_profile(288.0, 217.0, 0.77), &earth_air(co2_ppm))
    }

    fn doubling_forcing(co2_ppm: f64) -> f64 {
        let profile = earth_profile(288.0, 217.0, 0.77);
        olr_fixed_profile(&profile, &earth_air(co2_ppm))
            - olr_fixed_profile(&profile, &earth_air(2.0 * co2_ppm))
    }

    #[test]
    fn the_table_is_on_the_models_bins() {
        let table = &*OPACITY;
        assert_eq!((table.bins, table.bin_width_cm), (BINS, BIN_WIDTH_CM));
        let sum: f64 = weights().iter().sum();
        assert!((sum - 1.0).abs() < 1e-12, "weights sum to {sum}");
        assert!(table.ln_k.iter().all(|k| k.is_finite()));
    }

    #[test]
    fn water_mixes_its_two_broadenings_by_its_share() {
        // At 1 bar and 300 K in the rotation band (bin 10, 200-220 cm⁻¹):
        // a trace of vapour takes the air-broadened set, pure steam the
        // self-broadened one, which is wider and so stronger between the
        // lines.
        let level = |h2o: f64, n2: f64| Level {
            temperature_k: 300.0,
            gases: Gases {
                h2o,
                n2,
                ..Gases::default()
            },
            gravity_m_s2: EARTH_GRAVITY_M_S2,
        };
        let per_kg = |l: &Level| {
            let a = absorption_per_ln_p(10, l);
            let mass = l.mass_per_ln_p(l.gases.h2o, H2O_KG_MOL);
            a.water.map(|k| k / mass)
        };
        let trace = per_kg(&level(1e-6, 1.0));
        let steam = per_kg(&level(1.0, 0.0));
        let table = &*OPACITY;
        let mut foreign = [0.0; G];
        let mut own = [0.0; G];
        table.coefficients(WATER_FOREIGN_SET, 1.0, 300.0, 10, 1.0, &mut foreign);
        table.coefficients(WATER_SELF_SET, 1.0, 300.0, 10, 1.0, &mut own);
        for g in 0..G {
            assert!((trace[g] / foreign[g] - 1.0).abs() < 1e-5, "g {g}");
            assert!((steam[g] / own[g] - 1.0).abs() < 1e-9, "g {g}");
        }
        assert!(own[0] > foreign[0], "{own:?} vs {foreign:?}");
    }

    #[test]
    fn lines_broaden_with_pressure_past_the_grid() {
        // Over 100 bar the coefficients scale as p (Lorentz).
        let table = &*OPACITY;
        let mut at_100 = [0.0; G];
        let mut at_200 = [0.0; G];
        table.coefficients(CO2_SET, 100.0, 700.0, 33, 1.0, &mut at_100);
        table.coefficients(CO2_SET, 200.0, 700.0, 33, 1.0, &mut at_200);
        for g in 0..G {
            assert!((at_200[g] / at_100[g] - 2.0).abs() < 1e-6, "g {g}");
        }
    }

    #[test]
    fn doubling_co2_forces_myhre() {
        let forcing = doubling_forcing(280.0);
        assert!(
            (forcing - 3.7).abs() <= 0.6,
            "280 to 560 ppm forces {forcing:.3} W/m², against 3.7 ± 0.6"
        );
    }

    #[test]
    fn co2_forcing_is_logarithmic() {
        let forcings = [70.0, 280.0, 1000.0].map(doubling_forcing);
        let mean = forcings.iter().sum::<f64>() / 3.0;
        assert!(mean > 0.0, "no forcing: {forcings:?}");
        for f in forcings {
            assert!(
                (f / mean - 1.0).abs() < 0.15,
                "forcing per doubling {forcings:?} strays over 15 % from {mean:.3}"
            );
        }
    }

    #[test]
    fn earth_clear_sky_olr() {
        let olr = earth_olr(280.0);
        assert!(
            (olr - 265.0).abs() <= 10.0,
            "Earth's clear-sky OLR {olr:.2} W/m², against 265 ± 10"
        );
    }

    #[test]
    fn halving_bins_keeps_the_forcing() {
        // The halved table covers 0-2500 cm⁻¹; past it both sums take the
        // model's bins, which then cancel in the comparison.
        let halved = &*HALVED;
        assert_eq!(halved.bin_width_cm, 0.5 * BIN_WIDTH_CM);
        let split = (halved.bins as f64 * halved.bin_width_cm / BIN_WIDTH_CM).round() as usize;
        let profile = earth_profile(288.0, 217.0, 0.77);
        let fine_olr = |ppm: f64| {
            olr_bins(halved, 0..halved.bins, &profile, &earth_air(ppm))
                + olr_bins(&OPACITY, split..BINS, &profile, &earth_air(ppm))
        };
        let coarse = doubling_forcing(280.0);
        let fine = fine_olr(280.0) - fine_olr(560.0);
        assert!(coarse > 0.0, "no forcing");
        assert!(
            (coarse - fine).abs() < 0.1,
            "halving the bins moves the forcing from {coarse:.4} to {fine:.4} W/m²"
        );
    }

    #[test]
    fn planck_bins_sum_to_sigma_t4() {
        for t in [100.0, 288.0, 640.0] {
            let sum: f64 = (0..BINS).map(|i| planck_bin_w_m2(i, t)).sum();
            let black = SIGMA * f64::powi(t, 4);
            assert!(
                (sum / black - 1.0).abs() < 5e-3,
                "at {t} K the bins hold {sum:.3} W/m², against σT⁴ = {black:.3}"
            );
        }
    }

    #[test]
    fn a_venus_level_absorbs_finitely_in_every_bin() {
        let level = Level {
            temperature_k: 735.0,
            gases: Gases {
                n2: 3.2,
                co2: 88.8,
                h2o: 3e-3,
                so2: 1.4e-2,
                ..Gases::default()
            },
            gravity_m_s2: 8.87,
        };
        for i in 0..BINS {
            let a = absorption_per_ln_p(i, &level);
            let all = a.co2.iter().chain(&a.water).chain([&a.smooth]);
            for x in all {
                assert!(x.is_finite() && *x >= 0.0, "bin {i}: {a:?}");
            }
        }
    }

    #[test]
    fn the_spectrum_of_a_level_is_its_bins() {
        let levels = [
            Level {
                temperature_k: 288.0,
                gases: Gases {
                    h2o: 0.013,
                    ..earth_air(400.0)
                },
                gravity_m_s2: EARTH_GRAVITY_M_S2,
            },
            Level {
                temperature_k: 735.0,
                gases: Gases {
                    n2: 3.2,
                    co2: 88.8,
                    h2o: 3e-3,
                    ..Gases::default()
                },
                gravity_m_s2: 8.87,
            },
            Level {
                temperature_k: 93.7,
                gases: Gases {
                    n2: 1.45,
                    ch4: 0.07,
                    ..Gases::default()
                },
                gravity_m_s2: 1.352,
            },
            Level {
                temperature_k: 640.0,
                gases: Gases {
                    h2o: 70.0,
                    n2: 0.78,
                    co2: 4e-4,
                    ..Gases::default()
                },
                gravity_m_s2: EARTH_GRAVITY_M_S2,
            },
        ];
        let mut spectrum = Spectrum::default();
        let close = |a: f64, b: f64| (a - b).abs() <= 1e-12 * b.abs().max(1e-300);
        for level in &levels {
            absorption_spectrum(level, &mut spectrum);
            for i in 0..BINS {
                let one = absorption_per_ln_p(i, level);
                for g in 0..G {
                    assert!(close(spectrum.co2[i][g], one.co2[g]), "CO₂ bin {i}");
                    assert!(close(spectrum.water[i][g], one.water[g]), "water bin {i}");
                }
                assert!(
                    close(spectrum.smooth[i], one.smooth),
                    "bin {i} at {} K: {} vs {}",
                    level.temperature_k,
                    spectrum.smooth[i],
                    one.smooth
                );
            }
            assert_eq!(spectrum.co2[TAIL], spectrum.co2[BINS - 1]);
            assert_eq!(spectrum.water[TAIL], spectrum.water[BINS - 1]);
            assert_eq!(spectrum.smooth[TAIL], spectrum.smooth[BINS - 1]);
        }
    }

    #[test]
    fn the_planck_spectrum_is_the_bins_and_their_rest() {
        let mut planck = [0.0; BANDS];
        for t in [40.0, 100.0, 288.0, 640.0, 1800.0, 3000.0] {
            planck_spectrum(t, &mut planck);
            for (i, p) in planck[..BINS].iter().enumerate() {
                let one = planck_bin_w_m2(i, t);
                assert!(
                    (p - one).abs() <= 1e-12 * one.max(1e-300),
                    "bin {i}: {p} vs {one}"
                );
            }
            let black = SIGMA * f64::powi(t, 4);
            let sum: f64 = planck.iter().sum();
            assert!((sum / black - 1.0).abs() < 1e-9, "{t} K: {sum} vs {black}");
        }
    }
}
