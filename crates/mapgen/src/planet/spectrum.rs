//! A simple spectral model of the thermal infrared: how much each gas
//! absorbs in each bin of wavenumber, and how much a black body emits
//! there (issue #523, the longwave band model of lot 3C-2).
//!
//! A grey optical depth closes the whole spectrum at once, while CO₂ and
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
//! The absorbers, each with its source (every value is the paper's):
//!
//! - **CO₂ lines:** the 15 µm band as one exponential band,
//!   `κ = κ₀ exp(−|ν − ν₀| / l)`, broadened in proportion to the pressure
//!   ([`CO2_BAND_CM`], [`CO2_KAPPA_M2_KG`], [`CO2_DECAY_CM`],
//!   [`CO2_REFERENCE_BAR`]).
//! - **Water lines:** the rotation band and the vibration–rotation band,
//!   two exponential bands, the larger of the two at each wavenumber,
//!   broadened in proportion to the pressure (Koll, Jeevanjee & Lutsko
//!   2023, [`WATER_ROTATION`], [`WATER_VIBRATION`]).
//! - **Water continuum:** MT_CKD 4.3's self and foreign continua (Mlawer
//!   et al. 2012), the self part growing as the vapour pressure squared
//!   and the foreign part as the vapour times the other gases
//!   ([`MT_CKD`]).
//! - **CO₂–CO₂ collision-induced absorption:** HITRAN's CIA file (Karman
//!   et al. 2019), binned ([`CO2_CIA`]), growing as the CO₂ density
//!   squared, not broadened.
//! - **Methane:** its grey stand-in for Titan's collision-induced
//!   absorption ([`super::climate`]'s methane law), over
//!   0-[`METHANE_TOP_CM`] cm⁻¹ only.
//!
//! Line strengths do not depend on the temperature: the papers neglect it
//! (Jeevanjee et al. 2021, §2; Koll et al. 2023, §2), so the hot bands of
//! a warm CO₂ air are left out. The column masses come from `p / g` with
//! the body's gravity ([`Level::gravity_m_s2`]).
#![cfg_attr(not(test), allow(dead_code))]

use super::Gases;
use super::climate::methane_depth;

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
/// [`absorption_per_ln_p`] is vertical; the column multiplies it by this.
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
    fn molar_mass_kg(&self) -> f64 {
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
    fn mass_per_ln_p(&self, partial_bar: f64, molar_kg: f64) -> f64 {
        partial_bar.max(0.0) * PA_PER_BAR * molar_kg / (self.molar_mass_kg() * self.gravity_m_s2)
    }
}

/// Pascals per bar.
const PA_PER_BAR: f64 = 1e5;

/// Avogadro constant, 1/mol (CODATA 2018, exact).
const AVOGADRO: f64 = 6.022_140_76e23;

/// Boltzmann constant, J/K (CODATA 2018, exact).
const BOLTZMANN: f64 = 1.380_649e-23;

/// Stefan–Boltzmann constant, W/(m²·K⁴) (CODATA 2018, exact).
const SIGMA: f64 = 5.670_374_419e-8;

/// Second radiation constant `h c / k`, cm·K (CODATA 2018,
/// 1.438 776 877e-2 m·K).
const C2_CM_K: f64 = 1.438_776_877;

/// Molar masses, kg/mol, from the IUPAC standard atomic weights (H
/// 1.008, C 12.011, N 14.007, O 15.999, S 32.06).
const N2_KG_MOL: f64 = 28.014e-3;
const CO2_KG_MOL: f64 = 44.009e-3;
const H2O_KG_MOL: f64 = 18.015e-3;
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

/// Hemispheric flux, W/m², that a black body at `t_k` emits in bin `i`:
/// `π ∫ B_ν dν` over the bin, exact (see [`planck_between`]).
pub(crate) fn planck_bin_w_m2(i: usize, t_k: f64) -> f64 {
    let (lo, hi) = bin_edges(i);
    planck_between(lo, hi, t_k)
}

/// Hemispheric black-body flux, W/m², at `t_k` between `lo` and `hi`
/// cm⁻¹: `σT⁴` times the share of the Planck function between
/// `x = c₂ν/T` at the two edges, `(15/π⁴) ∫ x³/(eˣ − 1) dx`.
fn planck_between(lo: f64, hi: f64, t_k: f64) -> f64 {
    if t_k <= 0.0 || hi <= lo {
        return 0.0;
    }
    let (a, b) = (C2_CM_K * lo / t_k, C2_CM_K * hi / t_k);
    let integral = if a >= SERIES_SWITCH {
        planck_tail(a) - planck_tail(b)
    } else if b < SERIES_SWITCH {
        planck_head(b) - planck_head(a)
    } else {
        (PLANCK_TOTAL - planck_tail(b)) - planck_head(a)
    };
    SIGMA * t_k.powi(4) * integral.max(0.0) / PLANCK_TOTAL
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

/// `∫ₓ^∞ t³/(eᵗ − 1) dt` for `x` ≥ [`SERIES_SWITCH`]:
/// `Σₙ e^(−nx) (x³/n + 3x²/n² + 6x/n³ + 6/n⁴)`, summed until its terms
/// fall under 1e-16 of the first.
fn planck_tail(x: f64) -> f64 {
    let mut sum = 0.0;
    for n in 1..=64 {
        let n = n as f64;
        let e = (-n * x).exp();
        let term =
            e * (x * x * x / n + 3.0 * x * x / (n * n) + 6.0 * x / (n * n * n) + 6.0 / n.powi(4));
        sum += term;
        if term <= 1e-16 * sum {
            break;
        }
    }
    sum
}

/// Vertical optical depth per unit `ln p`, in bin `i`, of every absorber
/// at `level`: CO₂'s and water's lines, water's continuum, CO₂–CO₂
/// collision-induced absorption and methane's grey stand-in (see the
/// module documentation).
pub(crate) fn absorption_per_ln_p(i: usize, level: &Level) -> f64 {
    let (lo, hi) = bin_edges(i);
    absorption_between(lo, hi, level)
}

/// [`absorption_per_ln_p`] over the bin from `lo` to `hi` cm⁻¹. The
/// bands are taken at the bin's centre: the papers fit them to the
/// logarithmic (geometric) mean of the line-by-line absorption over
/// 10 cm⁻¹ bins (Jeevanjee & Fueglistaler 2020, §3a) or to its median
/// (Koll et al. 2023, §2), and the exponential's value at the centre is
/// its geometric mean over the bin.
fn absorption_between(lo: f64, hi: f64, level: &Level) -> f64 {
    let p = level.pressure_bar();
    let t = level.temperature_k;
    if p <= 0.0 || t <= 0.0 || level.gravity_m_s2 <= 0.0 {
        return 0.0;
    }
    let nu = 0.5 * (lo + hi);
    co2_lines(nu, level)
        + water_lines(nu, level)
        + water_continuum(nu, level)
        + co2_pairs(nu, level)
        + methane(lo, hi, level)
}

/// Centre of CO₂'s 15 µm band, cm⁻¹: 667.5, close to the 667.66 cm⁻¹ Q
/// branch (Jeevanjee & Fueglistaler 2020, *J. Atmos. Sci.* 77, 479,
/// doi:10.1175/JAS-D-18-0347.1, Table 1; Jeevanjee, Seeley, Paynter &
/// Fueglistaler 2021, *J. Climate* 34, 9463,
/// doi:10.1175/JCLI-D-19-0756.1, Table 1).
const CO2_BAND_CM: f64 = 667.5;

/// Mass absorption coefficient of CO₂ at the band centre, m²/kg, at
/// [`CO2_REFERENCE_BAR`] and 260 K: `κ_Q` = 110 (Jeevanjee &
/// Fueglistaler 2020, Table 1), from a direct fit to line-by-line
/// spectroscopy (RFM, HITRAN 2016) together with [`CO2_DECAY_CM`].
/// Romps, Seeley & Edman 2022 (§4b) fit their sorted spectrum at 1 bar
/// and 289 K independently: `κ₀ e^(b ν₂)` = 8.4e-15 m²/mol ×
/// e^(0.04 × 867) = 9.7 m²/mol, 220 m²/kg at 1 bar, the same peak as
/// 110 m²/kg at 0.5 bar under linear broadening. Jeevanjee et al. 2021
/// (§3b) optimise 50 m²/kg at 100 hPa (500 at 1 bar) for their
/// emission-level model, with the shorter decay 10.2 cm⁻¹; that pair is
/// not taken (see [`CO2_DECAY_CM`]).
const CO2_KAPPA_M2_KG: f64 = 110.0;

/// Pressure, bar, at which [`CO2_KAPPA_M2_KG`] holds: 500 hPa (Jeevanjee
/// & Fueglistaler 2020, eq. 4). The lines broaden as `p / p_ref`
/// (Lorentz), with the total pressure.
const CO2_REFERENCE_BAR: f64 = 0.5;

/// Spectroscopic decay length of CO₂'s band, cm⁻¹: `l_Q` = 11.5
/// (Jeevanjee & Fueglistaler 2020, Table 1), the direct fit to the
/// spectroscopy. Its stated range: Jeevanjee et al. 2021 (§3b) find
/// 10.2 cm⁻¹ best for their emission-level model and write that it "is
/// close to the l = 11–11.5-cm⁻¹ range reported in Jeevanjee and
/// Fueglistaler (2020b) and Wilson and Gea-Banacloche (2012) from direct
/// fits to the spectroscopy, and that all values in this range yield a
/// reasonable fit". Romps et al. 2022's sorted fit, `b` = 0.04 cm over
/// a two-sided band, is 1/(2b) = 12.5 cm⁻¹. This model integrates the
/// emission bin by bin rather than at one emission level, so it takes
/// the direct fit; with 10.2 cm⁻¹ (and 500 m²/kg at 1 bar) Earth's
/// doubling forcing is 3.01 W/m² instead of 3.41, under the 3.1 the
/// test `doubling_co2_forces_myhre` allows; with 11.0 cm⁻¹, 3.28 (task
/// 7.1 report).
const CO2_DECAY_CM: f64 = 11.5;

/// Optical depth per `ln p` of CO₂'s lines at `nu` cm⁻¹:
/// `κ(ν) (p / p_ref)` times the CO₂'s mass per `ln p`.
fn co2_lines(nu: f64, level: &Level) -> f64 {
    let co2 = level.gases.co2;
    if co2 <= 0.0 {
        return 0.0;
    }
    let kappa = CO2_KAPPA_M2_KG * (-(nu - CO2_BAND_CM).abs() / CO2_DECAY_CM).exp();
    kappa * (level.pressure_bar() / CO2_REFERENCE_BAR) * level.mass_per_ln_p(co2, CO2_KG_MOL)
}

/// One exponential band of water: centre (cm⁻¹), mass absorption
/// coefficient at its centre (m²/kg at [`WATER_REFERENCE_BAR`]) and
/// decay length (cm⁻¹).
struct Band {
    centre_cm: f64,
    kappa_m2_kg: f64,
    decay_cm: f64,
}

impl Band {
    fn kappa(&self, nu: f64) -> f64 {
        self.kappa_m2_kg * (-(nu - self.centre_cm).abs() / self.decay_cm).exp()
    }
}

/// Water's rotation band: centre 150 cm⁻¹, 165 m²/kg, decay 55 cm⁻¹
/// (Koll, Jeevanjee & Lutsko 2023, *J. Atmos. Sci.* 80, 1923,
/// doi:10.1175/JAS-D-22-0178.1; read in the authors' accepted
/// manuscript, §2b and Table 1), fitted at 1 bar to the median of line-by-line H₂O line
/// opacities, continuum left out, over 150-1500 cm⁻¹. Jeevanjee &
/// Fueglistaler 2020 (Table 1) fit the geometric mean: 127 m²/kg at
/// 500 hPa (254 at 1 bar), 56 cm⁻¹. The median is taken because the
/// mean transmission of a bin is set by its most transparent
/// wavenumbers (Koll et al. 2023).
const WATER_ROTATION: Band = Band {
    centre_cm: 150.0,
    kappa_m2_kg: 165.0,
    decay_cm: 55.0,
};

/// Water's vibration–rotation band: centre 1500 cm⁻¹, 15 m²/kg, decay
/// 38 cm⁻¹ (Koll, Jeevanjee & Lutsko 2023, Table 1; Jeevanjee &
/// Fueglistaler 2020: 3.8 m²/kg at 500 hPa, 1450 cm⁻¹, 40 cm⁻¹). The fit
/// stops at 1500 cm⁻¹; the band is taken symmetric about its centre, as
/// the paper writes it (`|ν − ν_v-r|`), so it says nothing past
/// 1500 cm⁻¹ where the real band peaks (1595 cm⁻¹) and spreads to
/// 2000 cm⁻¹.
const WATER_VIBRATION: Band = Band {
    centre_cm: 1500.0,
    kappa_m2_kg: 15.0,
    decay_cm: 38.0,
};

/// Pressure, bar, at which the water bands hold: 1 bar (Koll et al.
/// 2023, Table 1, `p₀`). They broaden as `p / p₀`, with the total
/// pressure.
const WATER_REFERENCE_BAR: f64 = 1.0;

/// Optical depth per `ln p` of water's lines at `nu` cm⁻¹: the larger of
/// its two bands (Koll et al. 2023, eq. 12) times `p / p₀` times the
/// vapour's mass per `ln p`.
fn water_lines(nu: f64, level: &Level) -> f64 {
    let h2o = level.gases.h2o;
    if h2o <= 0.0 {
        return 0.0;
    }
    let kappa = WATER_ROTATION.kappa(nu).max(WATER_VIBRATION.kappa(nu));
    kappa * (level.pressure_bar() / WATER_REFERENCE_BAR) * level.mass_per_ln_p(h2o, H2O_KG_MOL)
}

/// Spacing of [`MT_CKD`]'s nodes, cm⁻¹.
const MT_CKD_STEP_CM: f64 = 50.0;

/// Reference temperature (K) and pressure (bar) of [`MT_CKD`]: 296 K and
/// 1013 mbar (the file's `ref_temp` and `ref_press`).
const MT_CKD_REFERENCE_K: f64 = 296.0;
const MT_CKD_REFERENCE_BAR: f64 = 1.013;

/// The water vapour continuum of MT_CKD 4.3 (Mlawer et al. 2012,
/// doi:10.1098/rsta.2011.0295; file `absco-ref_wv-mt-ckd.nc` of
/// github.com/AER-RC/MT_CKD_H2O, read on 2026-11-18), every
/// [`MT_CKD_STEP_CM`] cm⁻¹ from 0 to 5000: wavenumber (cm⁻¹), the self
/// continuum coefficient at 296 K, its temperature exponent, and the
/// foreign continuum coefficient at 296 K (cm²/molecule per cm⁻¹; the
/// file's 10 cm⁻¹ grid taken at its nodes). Between nodes the
/// coefficients are interpolated in their logarithm. The 50 cm⁻¹ nodes
/// move Earth's outgoing radiation by 0.04 W/m² against the full grid
/// (task 7.1 report). The absorption per H₂O molecule is, as in MT_CKD's
/// `mt_ckd_h2o_absco`:
///
/// - self: `C_s(ν) (296/T)^n(ν) (e / p_ref) (296/T) R(ν, T)`;
/// - foreign: `C_f(ν) ((p − e) / p_ref) (296/T) R(ν, T)`;
///
/// with `R = ν tanh(c₂ν / 2T)` the radiation term, so that the self part
/// of the optical depth grows as `e²` and the foreign part as
/// `e (p − e)`. MT_CKD holds from cold air to steam; its own validity
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

/// Optical depth per `ln p` of the water continuum at `nu` cm⁻¹ (see
/// [`MT_CKD`]): the absorption per molecule times the vapour's molecules
/// per unit area per `ln p`, `e N_A / (M̄ g)`.
fn water_continuum(nu: f64, level: &Level) -> f64 {
    let e = level.gases.h2o;
    if e <= 0.0 || nu <= 0.0 {
        return 0.0;
    }
    let t = level.temperature_k;
    let x = (nu / MT_CKD_STEP_CM).clamp(0.0, (MT_CKD.len() - 1) as f64);
    let k = (x as usize).min(MT_CKD.len() - 2);
    let f = x - k as f64;
    let (a, b) = (MT_CKD[k], MT_CKD[k + 1]);
    let log_lerp = |u: f64, v: f64| (u.ln() * (1.0 - f) + v.ln() * f).exp();
    let self_coeff = log_lerp(a.1, b.1);
    let exponent = a.2 * (1.0 - f) + b.2 * f;
    let foreign_coeff = log_lerp(a.3, b.3);
    let ratio = MT_CKD_REFERENCE_K / t;
    let radiation = nu * (0.5 * C2_CM_K * nu / t).tanh();
    let others = (level.pressure_bar() - e).max(0.0);
    let per_molecule_cm2 = (self_coeff * ratio.powf(exponent) * e + foreign_coeff * others)
        / MT_CKD_REFERENCE_BAR
        * ratio
        * radiation;
    let molecules_m2 = e * PA_PER_BAR * AVOGADRO / (level.molar_mass_kg() * level.gravity_m_s2);
    per_molecule_cm2 * 1e-4 * molecules_m2
}

/// Temperatures, K, of the columns of [`CO2_CIA`].
const CO2_CIA_K: [f64; 4] = [200.0, 300.0, 450.0, 800.0];

/// Width of the cells of [`CO2_CIA`], cm⁻¹.
const CO2_CIA_CELL_CM: f64 = 20.0;

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
fn co2_pairs(nu: f64, level: &Level) -> f64 {
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

/// Upper end, cm⁻¹, of methane's grey stand-in: the far infrared of
/// Titan's N₂–N₂ and CH₄–N₂ collision-induced absorption (Courtin,
/// Gautier & McKay 1995, the design's call).
const METHANE_TOP_CM: f64 = 650.0;

/// Optical depth per `ln p` of methane between `lo` and `hi` cm⁻¹: the
/// rate of [`super::climate`]'s grey methane law at this level, on the
/// share of the bin under [`METHANE_TOP_CM`].
fn methane(lo: f64, hi: f64, level: &Level) -> f64 {
    let share = ((METHANE_TOP_CM.min(hi) - lo) / (hi - lo)).clamp(0.0, 1.0);
    if share <= 0.0 || level.gases.ch4 <= 0.0 {
        return 0.0;
    }
    share * methane_depth(&level.gases).1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet::climate::{EARTH_GRAVITY_M_S2, water_saturation_bar};

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

    /// Outgoing longwave radiation, W/m², of a fixed `profile` (from the
    /// surface up, uniform steps in `ln p`) whose dry air has the mole
    /// fractions `gases`, on bins `width` cm⁻¹ wide: per bin, the
    /// two-stream emission with [`DIFFUSIVITY`], the layer-mean Planck
    /// flux and the logarithmic mean of the opacities at a layer's two
    /// levels, as the column sums it. Above the top the air is taken
    /// isothermal and its depth is half the top's rate per `ln p`, exact
    /// for absorbers that grow as `p²` (broadened lines, the continuum,
    /// the pairs). The surface is a black body at the first level's
    /// temperature.
    fn olr_with(width: f64, profile: &[Point], gases: &Gases) -> f64 {
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
        let bins = (SPECTRUM_TOP_CM / width).round() as usize;
        let mut total = 0.0;
        for b in 0..bins {
            let (lo, hi) = edges_of(width, b);
            let opacity: Vec<f64> = levels
                .iter()
                .map(|l| absorption_between(lo, hi, l))
                .collect();
            let planck: Vec<f64> = levels
                .iter()
                .map(|l| planck_between(lo, hi, l.temperature_k))
                .collect();
            let mut seen = (-DIFFUSIVITY * 0.5 * opacity[n]).exp();
            let mut out = planck[n] * (1.0 - seen);
            for i in (0..n).rev() {
                let step = (profile[i].pressure_bar / profile[i + 1].pressure_bar).ln();
                let depth = log_mean(opacity[i], opacity[i + 1]) * step;
                let below = seen * (-DIFFUSIVITY * depth).exp();
                out += 0.5 * (planck[i] + planck[i + 1]) * (seen - below);
                seen = below;
            }
            total += out + planck[0] * seen;
        }
        total
    }

    /// Logarithmic mean of two opacities per `ln p` (as the column's).
    fn log_mean(a: f64, b: f64) -> f64 {
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

    /// [`olr_with`] on the model's own bins.
    fn olr_fixed_profile(profile: &[Point], gases: &Gases) -> f64 {
        olr_with(BIN_WIDTH_CM, profile, gases)
    }

    fn earth_olr(co2_ppm: f64) -> f64 {
        olr_fixed_profile(&earth_profile(288.0, 217.0, 0.77), &earth_air(co2_ppm))
    }

    fn doubling_forcing(width: f64, co2_ppm: f64) -> f64 {
        let profile = earth_profile(288.0, 217.0, 0.77);
        olr_with(width, &profile, &earth_air(co2_ppm))
            - olr_with(width, &profile, &earth_air(2.0 * co2_ppm))
    }

    #[test]
    fn doubling_co2_forces_myhre() {
        let forcing = doubling_forcing(BIN_WIDTH_CM, 280.0);
        assert!(
            (forcing - 3.7).abs() <= 0.6,
            "280 to 560 ppm forces {forcing:.3} W/m², against 3.7 ± 0.6"
        );
    }

    #[test]
    fn co2_forcing_is_logarithmic() {
        let forcings = [70.0, 280.0, 1000.0].map(|ppm| doubling_forcing(BIN_WIDTH_CM, ppm));
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
        let coarse = doubling_forcing(BIN_WIDTH_CM, 280.0);
        let fine = doubling_forcing(0.5 * BIN_WIDTH_CM, 280.0);
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
            assert!(a.is_finite() && a >= 0.0, "bin {i}: {a}");
        }
    }
}
