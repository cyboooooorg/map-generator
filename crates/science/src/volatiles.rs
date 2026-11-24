//! The volatiles a body is born with, and the saturation of the gases
//! that condense on the surface or in the column: carbon dioxide and
//! methane, and the frost of carbon dioxide and ammonia that a cold
//! trap reads ([`co2_frost_pressure`], [`ammonia_frost_pressure`],
//! [`nitrogen_frost_pressure`], [`sulphur_dioxide_frost_pressure`]), the
//! vapour of liquid nitrogen and the boiling of ethane.
//!
//! **Inventory at formation** ([`inventory`]): the mass of CO2, N2,
//! sulphur, CH4 and NH3 a body's rock and iron, silicate and ice carry by
//! the rule alone, before the engine scatters each by `10^N(0,`
//! [`VOLATILE_SPREAD`]`)`. Ported from the tag `legacy-final`
//! (`planet/inputs.rs` l.215-248, 476-505) with the values of study
//! [21](https://github.com/cyboooooorg/map-generator/wiki/21-escape-outgassing-audit),
//! part 1; see `docs/science/planet-atmospheres.md`, "Volatile inventory
//! at formation".
//!
//! Each curve is a vapour-pressure equation of the form
//! `p = p_c exp(T_c / T sum n_i (1 - T/T_c)^t_i)` from the triple to the
//! critical point, joined below the triple point by a sublimation line,
//! and infinite past the critical point, where the gas does not condense.
//!
//! The vapour-pressure coefficients are CoolProp's saturation ancillaries
//! (Bell, Wronski, Quoilin and Lemort 2014, doi:10.1021/ie4033999), read
//! in CoolProp v6.6.0 (`dev/fluids/CarbonDioxide.json` and `Methane.json`,
//! identical in v8.0.0): six-term fits CoolProp made to the reference
//! equations of state, Span and Wagner 1996 (doi:10.1063/1.555991) for
//! CO2 and Setzmann and Wagner 1991 (doi:10.1063/1.555898) for methane,
//! within 0.001 % (CO2) and 0.004 % (methane) of them by the files'
//! `max_abserror_percentage`. They are not the papers' own four-term
//! ancillaries. The papers are paywalled; the check tables of the tests are
//! the NIST WebBook's, which use the same equations of state.
//!
//! Below the triple point, CO2 follows the sublimation equation of Span
//! and Wagner 1996 (their eq. 3.12), read in two open implementations that
//! cite it: thermopack (`fluids/CarbonDioxide.json`, `sublimation_curve`)
//! and MOOSE (`CO2FluidProperties::sublimationPressure`). Methane, for
//! which neither CoolProp nor Setzmann and Wagner give one, follows
//! Clausius-Clapeyron from its triple point with a constant enthalpy.

use crate::constants::{MOLAR_GAS_CONSTANT, PASCALS_PER_BAR};
use crate::units::{Dex, Kelvin, Kilogram, Pascal};

/// Calibration: kg of CO2 per kg of rock and iron, fitted on Venus's air,
/// which has no thermostat: 92 bar of CO2 is about 1e-4 of the planet
/// once 95 % is outgassed. The bulk silicate Earth holds 2.7 times more
/// as CO2 (C 109 +77/-39 ppm, Wang, Lineweaver & Ireland 2018, Icarus
/// 299, 460, doi:10.1016/j.icarus.2017.08.024, Table 1), within the "up
/// to about 500 ppm carbon" of Marty 2012 (doi:10.1016/j.epsl.2011.10.040,
/// abstract); the carbon that never degasses is in the outgassed and
/// recycled shares of `outgassing` (study 21, part 1).
pub const ROCK_CO2: f64 = 1e-4;

/// Calibration: kg of N2 per kg of rock and iron, a value inside the
/// spread of the two budgets read: 1.4e-6 from the primitive mantle's
/// N 2.01 +1.42/-0.71 ppm (Wang et al. 2018, Table 1, times its 0.675),
/// 4.5e-6 from the silicate Earth's 27 ± 16e18 kg (Johnson & Goldblatt
/// 2015, doi:10.1016/j.earscirev.2015.05.006, abstract) (study 21,
/// part 1).
pub const ROCK_N2: f64 = 2e-6;

/// Source: kg of sulphur per kg of silicate (mantle and crust), the bulk
/// silicate Earth's 235 ± 38 ppm (Wang, Lineweaver & Ireland 2018,
/// Table 1, a concordance of Lyubetskaya & Korenaga 2007, McDonough &
/// Arevalo 2008 and Palme & O'Neill 2014), rounded to the 250 ppm of
/// McDonough 2003 (Treatise on Geochemistry 2.15, the legacy's origin,
/// primary not reached). The core keeps the rest of the planet's.
pub const ROCK_S: f64 = 2.5e-4;

/// Calibration: kg of CO2 per kg of ice, 5 % of the moles of water (times
/// 44/18). Biver et al. 2022 (Comets III, arXiv:2207.04800, Table 3)
/// print 4 to 30 mole %, "typically ranging from 5 to 30 %", and no
/// median; 5 % is the low end and comet 67P's bulk 4.7 ± 1.4 (Rubin et
/// al. 2019, doi:10.1093/mnras/stz2086, Table 2). The legacy 0.05 was
/// 2 mole %, under every comet measured (study 21, part 1).
pub const ICE_CO2: f64 = 0.12;

/// Calibration: kg of N2 per kg of ice, comet 67P's bulk 0.089 ± 0.024
/// mole % of water (Rubin et al. 2019, Table 2), times 28/18. 67P is the
/// only comet with a measured N2 (Biver et al. 2022, Table 3).
pub const ICE_N2: f64 = 1.4e-3;

/// Source: kg of CH4 per kg of ice, the "typical abundances ~0.8 %" of
/// the moles of water (Biver et al. 2022, Comets III, arXiv:2207.04800,
/// Table 3 and its text, from Dello Russo et al. 2016), times 16/18.
pub const ICE_CH4: f64 = 7e-3;

/// Source: kg of NH3 per kg of ice, the middle of the "typical
/// abundances ~0.5-1 %" of the moles of water (Biver et al. 2022,
/// Table 3 and its text), 0.75 mole % times 17/18; also 67P's
/// 0.67 ± 0.20 (Rubin et al. 2019).
pub const ICE_NH3: f64 = 7e-3;

/// Calibration: spread of each volatile around its rule, log-normal. Of
/// the order of the uncertainty of Earth's own budgets (C and N
/// -0.44/+0.23 dex around Wang et al. 2018's concordance, the two
/// nitrogen budgets 0.5 dex apart); no study of the spread between
/// planets found (study 21, part 1).
pub const VOLATILE_SPREAD: Dex = Dex(0.3);

/// The volatiles a body carries at formation, each as the mass of its own
/// gas, except `s`, the mass of sulphur (it comes up as SO2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Inventory {
    /// Carbon dioxide.
    pub co2: Kilogram,
    /// Nitrogen.
    pub n2: Kilogram,
    /// Sulphur.
    pub s: Kilogram,
    /// Methane.
    pub ch4: Kilogram,
    /// Ammonia.
    pub nh3: Kilogram,
}

/// The volatiles a body of `refractory` rock and iron, of which
/// `silicate` is mantle and crust, and `ice` carries by the rule alone,
/// without the scatter of [`VOLATILE_SPREAD`]:
///
/// ```text
/// co2 = 1e-4 refractory + 0.12 ice     n2  = 2e-6 refractory + 1.4e-3 ice
/// s   = 2.5e-4 silicate                ch4 = 7e-3 ice,   nh3 = 7e-3 ice
/// ```
///
/// The caller passes the composition before any steam loss, so that the
/// volatiles keep the mass the body formed with, as in the legacy (the
/// runaway's loss of water does not take the ice's CO2 with it).
pub fn inventory(refractory: Kilogram, silicate: Kilogram, ice: Kilogram) -> Inventory {
    Inventory {
        co2: Kilogram(ROCK_CO2 * refractory.0 + ICE_CO2 * ice.0),
        n2: Kilogram(ROCK_N2 * refractory.0 + ICE_N2 * ice.0),
        s: Kilogram(ROCK_S * silicate.0),
        ch4: Kilogram(ICE_CH4 * ice.0),
        nh3: Kilogram(ICE_NH3 * ice.0),
    }
}

/// Source: critical temperature of CO2, 304.1282 K (Span and Wagner 1996,
/// doi:10.1063/1.555991; as in CoolProp v6.6.0,
/// `dev/fluids/CarbonDioxide.json`, `pS.T_r`).
const CO2_CRITICAL_TEMPERATURE: f64 = 304.1282;
/// Source: critical pressure of CO2, 7 377 300 Pa (Span and Wagner 1996,
/// doi:10.1063/1.555991; as in CoolProp v6.6.0,
/// `dev/fluids/CarbonDioxide.json`, `pS.reducing_value`).
const CO2_CRITICAL_PRESSURE_PA: f64 = 7_377_300.0;
/// Source: triple-point temperature of CO2, 216.592 K (Span and Wagner
/// 1996, doi:10.1063/1.555991; as in CoolProp v6.6.0,
/// `dev/fluids/CarbonDioxide.json`, `pS.Tmin`).
const CO2_TRIPLE_TEMPERATURE: f64 = 216.592;
/// Source: coefficients `n_i` of the vapour pressure of CO2,
/// `ln(p / p_c) = T_c / T sum n_i theta^t_i`, `theta = 1 - T / T_c`
/// CoolProp's ancillary fit to the Span and Wagner 1996 equation of state
/// (Bell et al. 2014, doi:10.1021/ie4033999), read in CoolProp v6.6.0,
/// `dev/fluids/CarbonDioxide.json`, `ANCILLARIES.pS.n`.
const CO2_VAPOUR_N: [f64; 6] = [
    -5.867399337600407,
    -7.10969550015274,
    11.022781986239263,
    4.8260764050219995,
    -6.240803382557819,
    -6.7009642572439,
];
/// Source: exponents `t_i` of the vapour pressure of CO2 (as
/// [`CO2_VAPOUR_N`], `ANCILLARIES.pS.t`).
const CO2_VAPOUR_T: [f64; 6] = [0.983, 1.322, 1.488, 2.807, 3.571, 1.941];
/// Source: triple-point pressure of CO2, 0.51795 MPa (Span and Wagner
/// 1996, doi:10.1063/1.555991), the reducing pressure of their sublimation
/// equation; read in thermopack (`fluids/CarbonDioxide.json`,
/// `sublimation_curve.reducing_pressure`, commit d68c794c) and MOOSE
/// (`CO2FluidProperties.h`, `_triple_point_pressure`, commit 15eacf5e).
const CO2_TRIPLE_PRESSURE_PA: f64 = 517_950.0;
/// Source: coefficients `a_i` of the sublimation pressure of CO2,
/// `ln(p / p_t) = T_t / T sum a_i x^e_i`, `x = 1 - T / T_t` (Span and
/// Wagner 1996, doi:10.1063/1.555991, eq. 3.12); read in thermopack
/// (`fluids/CarbonDioxide.json`, `sublimation_curve.coeff`, commit
/// d68c794c) and MOOSE (`CO2FluidProperties::sublimationPressure`, commit
/// 15eacf5e), which agree.
const CO2_SUBLIMATION_A: [f64; 3] = [-14.740_846, 2.432_701_5, -5.306_177_8];
/// Source: exponents `e_i` of the sublimation pressure of CO2 (as
/// [`CO2_SUBLIMATION_A`], `sublimation_curve.exponents`).
const CO2_SUBLIMATION_E: [f64; 3] = [1.0, 1.9, 2.9];

/// Source: critical temperature of methane, 190.564 K (Setzmann and Wagner
/// 1991, doi:10.1063/1.555898; as in CoolProp v6.6.0,
/// `dev/fluids/Methane.json`, `pS.T_r`).
const METHANE_CRITICAL_TEMPERATURE: f64 = 190.564;
/// Source: critical pressure of methane, 4 599 200 Pa (Setzmann and
/// Wagner 1991, doi:10.1063/1.555898; as in CoolProp v6.6.0,
/// `dev/fluids/Methane.json`, `pS.reducing_value`).
const METHANE_CRITICAL_PRESSURE_PA: f64 = 4_599_200.0;
/// Source: triple-point temperature of methane, 90.6941 K (Setzmann and
/// Wagner 1991, doi:10.1063/1.555898; as in CoolProp v6.6.0,
/// `dev/fluids/Methane.json`, `pS.Tmin`).
pub(crate) const METHANE_TRIPLE_TEMPERATURE: f64 = 90.6941;
/// Source: coefficients `n_i` of the vapour pressure of methane (same form
/// as [`CO2_VAPOUR_N`]): CoolProp's ancillary fit to the Setzmann and
/// Wagner 1991 equation of state (Bell et al. 2014, doi:10.1021/ie4033999),
/// read in CoolProp v6.6.0, `dev/fluids/Methane.json`, `ANCILLARIES.pS.n`.
const METHANE_VAPOUR_N: [f64; 6] = [
    -0.36511751226395045,
    -7.477611035699516,
    2.8218785804602344,
    -0.3138277099613026,
    -14.181166200304828,
    13.10811727353235,
];
/// Source: exponents `t_i` of the vapour pressure of methane (as
/// [`METHANE_VAPOUR_N`], `ANCILLARIES.pS.t`).
const METHANE_VAPOUR_T: [f64; 6] = [0.935, 1.03, 1.204, 3.176, 5.616, 5.744];
/// Source: sublimation enthalpy of methane, 9.7 kJ/mol at 72 K (Stephenson
/// and Malanowski 1987, as listed by the NIST Chemistry WebBook, phase
/// change data of CH4; other entries 9.2 to 10.0). Held constant below the
/// triple point: neither CoolProp nor Setzmann and Wagner 1991 give a
/// sublimation curve (thermopack carries one credited to Lemmon 2002, not
/// adopted here).
const METHANE_SUBLIMATION_ENTHALPY: f64 = 9_700.0;

/// Source: triple-point temperature of ammonia, 195.49 K (Gao et al. 2023,
/// J. Phys. Chem. Ref. Data 52, doi:10.1063/5.0128269, abstract on the
/// NIST page).
const AMMONIA_TRIPLE_TEMPERATURE: f64 = 195.49;
/// Source: triple-point pressure of ammonia, 0.06060 bar (Fonseca and Lobo
/// 1989, via the NIST Chemistry WebBook phase-change page, data page
/// read).
const AMMONIA_TRIPLE_PA: f64 = 6_060.0;
/// Source: sublimation enthalpy of ammonia, 31.2 kJ/mol (Overstreet and
/// Giauque 1937, via the NIST Chemistry WebBook, 177 to 195 K, data page
/// read). Held constant below the triple point.
const AMMONIA_SUBLIMATION_ENTHALPY: f64 = 31_200.0;

/// Definition: smallest temperature of the sublimation lines, 1 K, so that
/// 0 K, a negative or a NaN temperature give a finite pressure.
const FLOOR_TEMPERATURE: f64 = 1.0;

/// How a curve continues below its triple point.
enum Sublimation {
    /// `ln(p / p_t) = T_t / T sum a_i x^e_i`, `x = 1 - T / T_t`.
    Equation {
        triple_pa: f64,
        a: [f64; 3],
        e: [f64; 3],
    },
    /// Clausius-Clapeyron from the triple point of the vapour curve with
    /// a constant enthalpy, J/mol.
    ConstantEnthalpy(f64),
}

/// One gas's saturation curve.
struct Curve {
    critical_temperature: f64,
    critical_pa: f64,
    triple_temperature: f64,
    n: [f64; 6],
    t: [f64; 6],
    sublimation: Sublimation,
}

const CO2: Curve = Curve {
    critical_temperature: CO2_CRITICAL_TEMPERATURE,
    critical_pa: CO2_CRITICAL_PRESSURE_PA,
    triple_temperature: CO2_TRIPLE_TEMPERATURE,
    n: CO2_VAPOUR_N,
    t: CO2_VAPOUR_T,
    sublimation: Sublimation::Equation {
        triple_pa: CO2_TRIPLE_PRESSURE_PA,
        a: CO2_SUBLIMATION_A,
        e: CO2_SUBLIMATION_E,
    },
};

const METHANE: Curve = Curve {
    critical_temperature: METHANE_CRITICAL_TEMPERATURE,
    critical_pa: METHANE_CRITICAL_PRESSURE_PA,
    triple_temperature: METHANE_TRIPLE_TEMPERATURE,
    n: METHANE_VAPOUR_N,
    t: METHANE_VAPOUR_T,
    sublimation: Sublimation::ConstantEnthalpy(METHANE_SUBLIMATION_ENTHALPY),
};

impl Curve {
    /// `(f, df/dT)` of `f = T_c / T sum n_i theta^t_i` at `t_k` between
    /// the triple and the critical point.
    fn vapour_sum(&self, t_k: f64) -> (f64, f64) {
        let tc = self.critical_temperature;
        let theta = 1.0 - t_k / tc;
        let (mut sum, mut slope) = (0.0, 0.0);
        for (n, t) in self.n.iter().zip(self.t) {
            sum += n * theta.powf(t);
            slope -= n * t * theta.powf(t - 1.0) / tc;
        }
        (tc / t_k * sum, -tc / (t_k * t_k) * sum + tc / t_k * slope)
    }

    /// `(ln p, d ln p / dT)` below the triple point, `p` in Pa, at `t_k`
    /// held at or above [`FLOOR_TEMPERATURE`].
    fn sublimation_log(&self, t_k: f64) -> (f64, f64) {
        let t = t_k.max(FLOOR_TEMPERATURE);
        let tt = self.triple_temperature;
        match self.sublimation {
            Sublimation::Equation { triple_pa, a, e } => {
                let x = 1.0 - t / tt;
                let (mut sum, mut slope) = (0.0, 0.0);
                for (a, e) in a.iter().zip(e) {
                    sum += a * x.powf(e);
                    slope -= a * e * x.powf(e - 1.0) / tt;
                }
                (
                    triple_pa.ln() + tt / t * sum,
                    -tt / (t * t) * sum + tt / t * slope,
                )
            }
            Sublimation::ConstantEnthalpy(enthalpy) => {
                let triple = self.critical_pa.ln() + self.vapour_sum(tt).0;
                (
                    triple - enthalpy / MOLAR_GAS_CONSTANT * (1.0 / t - 1.0 / tt),
                    enthalpy / (MOLAR_GAS_CONSTANT * t * t),
                )
            }
        }
    }

    /// Saturation pressure, Pa: the vapour curve from the triple to the
    /// critical point, the sublimation line below, infinite from the
    /// critical point up.
    fn pressure_pa(&self, t_k: f64) -> f64 {
        if t_k >= self.critical_temperature {
            return f64::INFINITY;
        }
        if t_k >= self.triple_temperature {
            return self.critical_pa * self.vapour_sum(t_k).0.exp();
        }
        self.sublimation_log(t_k).0.exp()
    }

    /// `R T^2 d ln p / dT`, J/mol, with `T` held at or above
    /// [`FLOOR_TEMPERATURE`] below the triple point; 0 from the critical
    /// point up.
    fn latent_j_mol(&self, t_k: f64) -> f64 {
        if t_k >= self.critical_temperature {
            0.0
        } else if t_k >= self.triple_temperature {
            MOLAR_GAS_CONSTANT * t_k * t_k * self.vapour_sum(t_k).1
        } else {
            let t = t_k.max(FLOOR_TEMPERATURE);
            MOLAR_GAS_CONSTANT * t * t * self.sublimation_log(t_k).1
        }
    }
}

/// Pressure of CO2 in equilibrium with its condensate at `t`: frost below
/// the triple point (216.592 K), liquid up to the critical point
/// (304.1282 K), infinite from there, where CO2 does not condense.
pub fn co2_saturation_pressure(t: Kelvin) -> Pascal {
    Pascal(co2_saturation_bar(t.0) * PASCALS_PER_BAR)
}

/// [`co2_saturation_pressure`] at `t_k` in bar.
pub(crate) fn co2_saturation_bar(t_k: f64) -> f64 {
    CO2.pressure_pa(t_k) / PASCALS_PER_BAR
}

/// Molar latent heat of CO2, J/mol, at `t_k`, taken from the slope of
/// [`co2_saturation_bar`] by Clausius-Clapeyron for an ideal vapour over a
/// condensate of negligible volume, `L = R T^2 d ln p / dT`, as
/// [`crate::water::vapour_latent_j_mol`] does: the slope of the
/// sublimation equation below the triple point, of the vapour curve
/// above, 0 from the critical point up (the legacy clamp). The moist adiabat that uses it then agrees
/// with the curve.
///
/// This is the ideal-vapour latent heat, a declared simplification: CO2's
/// vapour is not ideal at its saturation pressures, so it is about 29 %
/// above the real enthalpy of vaporisation at 250 K (16.45 kJ/mol against
/// the 12.73 kJ/mol of the NIST WebBook table, Span and Wagner 1996). It
/// is kept because it is consistent with the curve used in the same
/// adiabat and behaves as the legacy did (its single 16.6 kJ/mol mean).
pub(crate) fn co2_latent_j_mol(t_k: f64) -> f64 {
    CO2.latent_j_mol(t_k)
}

/// Pressure of methane in equilibrium with its condensate at `t`: frost
/// below the triple point (90.6941 K), liquid up to the critical point
/// (190.564 K), infinite from there, where methane does not condense.
pub fn methane_saturation_pressure(t: Kelvin) -> Pascal {
    Pascal(methane_saturation_bar(t.0) * PASCALS_PER_BAR)
}

/// [`methane_saturation_pressure`] at `t_k` in bar.
pub(crate) fn methane_saturation_bar(t_k: f64) -> f64 {
    METHANE.pressure_pa(t_k) / PASCALS_PER_BAR
}

/// Pressure of CO2 over its frost at `t`, the sublimation branch of the
/// curve of [`co2_saturation_pressure`]; `None` at or above the triple
/// point (216.592 K), where CO2 is no longer trapped as a solid. `t` is
/// held at or above 1 K.
pub fn co2_frost_pressure(t: Kelvin) -> Option<Pascal> {
    (t.0 < CO2_TRIPLE_TEMPERATURE).then(|| Pascal(CO2.pressure_pa(t.0)))
}

/// Pressure of NH3 over its frost at `t`: Clausius-Clapeyron from the
/// triple point (195.49 K, 0.06060 bar) with a constant enthalpy of
/// 31.2 kJ/mol; `None` at or above the triple point. `t` is held at or
/// above 1 K.
pub fn ammonia_frost_pressure(t: Kelvin) -> Option<Pascal> {
    if t.0 >= AMMONIA_TRIPLE_TEMPERATURE {
        return None;
    }
    let t = t.0.max(FLOOR_TEMPERATURE);
    let ln_ratio = -AMMONIA_SUBLIMATION_ENTHALPY / MOLAR_GAS_CONSTANT
        * (1.0 / t - 1.0 / AMMONIA_TRIPLE_TEMPERATURE);
    Some(Pascal(AMMONIA_TRIPLE_PA * ln_ratio.exp()))
}

/// Source: triple-point temperature of nitrogen, 63.14 K (Angus, de
/// Reuck et al. 1979, via the NIST Chemistry WebBook; study 24,
/// finding 1).
pub const NITROGEN_TRIPLE_TEMPERATURE: Kelvin = Kelvin(63.14);
/// Source: triple-point pressure of nitrogen, 0.1253 bar (Angus, de
/// Reuck et al. 1979, via the NIST Chemistry WebBook; study 24, finding
/// 1), in Pa.
pub const NITROGEN_TRIPLE_PRESSURE: Pascal = Pascal(12_530.0);
/// Source: alpha-beta transition of solid nitrogen at low pressure,
/// 35.61 K (Scott 1976, Phys. Rep. 27, 89, as quoted by Tegler et al.
/// 2019; study 24, finding 1).
pub const NITROGEN_ALPHA_BETA: Kelvin = Kelvin(35.61);
/// Source: coefficients `A0` to `A5` of `ln(p / 1 bar) = sum A_i / T^i`
/// for cubic alpha-N2, fit `N2-1` of Fray and Schmitt 2009 (Planet.
/// Space Sci. 57, 2053, doi:10.1016/j.pss.2009.09.011), as printed in
/// Table 2 of Lisse et al. 2021 (Icarus 356, 114072,
/// doi:10.1016/j.icarus.2020.114072); study 24, finding 1.
const NITROGEN_ALPHA_FIT: [f64; 6] = [12.40, -807.4, -3926.0, 62_970.0, -463_300.0, 1_325_000.0];
/// Source: the same for hexagonal beta-N2, fit `N2-2` (Fray and Schmitt
/// 2009, via Table 2 of Lisse et al. 2021); study 24, finding 1.
const NITROGEN_BETA_FIT: [f64; 6] = [8.514, -458.4, -19_870.0, 480_000.0, -4_524_000.0, 0.0];
/// Definition: coldest temperature, 10 K, at which the alpha fit is
/// read; colder, the curve carries on with its slope at 10 K (a constant
/// enthalpy), because the polynomial turns over under about 6 K.
const NITROGEN_FIT_FLOOR: f64 = 10.0;

/// `sum A_i x^i` and its derivative in `x`, by Horner's rule.
fn polynomial(coefficients: &[f64], x: f64) -> (f64, f64) {
    coefficients
        .iter()
        .rev()
        .fold((0.0, 0.0), |(value, slope), &a| {
            (value * x + a, slope * x + value)
        })
}

/// Pressure of N2 over its frost at `t`: the beta fit of Fray and
/// Schmitt 2009 (`N2-2`) from the triple point (63.14 K, 12 530 Pa) down
/// to the alpha-beta transition (35.61 K), the alpha fit (`N2-1`) below
/// it, each pinned to the curve above it so that the curve is
/// continuous, and the alpha slope carried on under 10 K. The beta fit
/// is pinned by about 3 % and the alpha fit by about 2.6 %. `None` at or
/// above the triple point. `t` is held at or above 1 K.
pub fn nitrogen_frost_pressure(t: Kelvin) -> Option<Pascal> {
    if t.0 >= NITROGEN_TRIPLE_TEMPERATURE.0 {
        return None;
    }
    let ln = |fit: &[f64], t: f64| polynomial(fit, 1.0 / t).0;
    let beta =
        |t: f64| ln(&NITROGEN_BETA_FIT, t) - ln(&NITROGEN_BETA_FIT, NITROGEN_TRIPLE_TEMPERATURE.0);
    let alpha = |t: f64| {
        ln(&NITROGEN_ALPHA_FIT, t) - ln(&NITROGEN_ALPHA_FIT, NITROGEN_ALPHA_BETA.0)
            + beta(NITROGEN_ALPHA_BETA.0)
    };
    let t = t.0.max(FLOOR_TEMPERATURE);
    let ln_ratio = if t >= NITROGEN_ALPHA_BETA.0 {
        beta(t)
    } else if t >= NITROGEN_FIT_FLOOR {
        alpha(t)
    } else {
        let floor = 1.0 / NITROGEN_FIT_FLOOR;
        let slope = polynomial(&NITROGEN_ALPHA_FIT, floor).1;
        alpha(NITROGEN_FIT_FLOOR) + slope * (1.0 / t - floor)
    };
    Some(Pascal(NITROGEN_TRIPLE_PRESSURE.0 * ln_ratio.exp()))
}

/// Source: constants `A`, `B`, `C` of the Antoine equation of liquid
/// nitrogen, `log10(p / 1 bar) = A - B / (T - C)`, 63.14 to 126 K
/// (Edejer and Thodos 1967, via the NIST Chemistry WebBook; study 24,
/// finding 1).
const NITROGEN_ANTOINE: [f64; 3] = [3.7362, 264.651, 6.788];

/// Pressure of liquid nitrogen at its saturation, from the Antoine fit
/// of Edejer and Thodos 1967, valid from 63.14 to 126 K. The fit is not
/// pinned to the triple point: it falls 4.5 % under the normal boiling
/// point (101 325 Pa at 77.355 K) and 12 % under the triple-point
/// pressure. Its only reader sits at the methane triple point, 90.69 K.
pub fn nitrogen_vapour_pressure(t: Kelvin) -> Pascal {
    let [a, b, c] = NITROGEN_ANTOINE;
    Pascal(10f64.powf(a - b / (t.0 - c)) * PASCALS_PER_BAR)
}

/// Source: triple-point temperature of sulphur dioxide, 197.64 K
/// (Giauque and Stephenson 1938, via the NIST Chemistry WebBook; study
/// 24, finding 2).
pub const SULPHUR_DIOXIDE_TRIPLE_TEMPERATURE: Kelvin = Kelvin(197.64);
/// Source: triple-point pressure of sulphur dioxide, 0.0167 bar
/// (Giauque and Stephenson 1938, via the NIST Chemistry WebBook; study
/// 24, finding 2), in Pa.
pub const SULPHUR_DIOXIDE_TRIPLE_PRESSURE: Pascal = Pascal(1_670.0);
/// Source: `L / R` of the sublimation of sulphur dioxide, 4510 K, from
/// `P = 1.516e8 exp(-4510 K / T)` bar (Wagman 1979, NBS Chemical
/// Thermodynamic Tables, as quoted by Giles et al. 2024, eq. 1,
/// arXiv:2405.19253); study 24, finding 2.
const SULPHUR_DIOXIDE_SUBLIMATION_L_OVER_R: f64 = 4510.0;

/// Pressure of SO2 over its frost at `t`: Wagman's slope (`L / R` =
/// 4510 K) anchored at the triple point (197.64 K, 1 670 Pa); `None` at
/// or above the triple point. Used below the triple point down to Io's
/// frost; Wagman's fitting range is not published. `t` is held at or
/// above 1 K.
pub fn sulphur_dioxide_frost_pressure(t: Kelvin) -> Option<Pascal> {
    if t.0 >= SULPHUR_DIOXIDE_TRIPLE_TEMPERATURE.0 {
        return None;
    }
    let t = t.0.max(FLOOR_TEMPERATURE);
    let ln_ratio = -SULPHUR_DIOXIDE_SUBLIMATION_L_OVER_R
        * (1.0 / t - 1.0 / SULPHUR_DIOXIDE_TRIPLE_TEMPERATURE.0);
    Some(Pascal(SULPHUR_DIOXIDE_TRIPLE_PRESSURE.0 * ln_ratio.exp()))
}

/// Source: normal boiling point of ethane, 184.6 K (average of 23
/// values, NIST Chemistry WebBook phase-change data; study 24, finding
/// 9).
const ETHANE_BOILING_TEMPERATURE: f64 = 184.6;
/// Source: enthalpy of vaporisation of ethane, 14.703 kJ/mol at 184.1 K
/// (Witt and Kemp 1937, via the NIST Chemistry WebBook; study 24,
/// finding 9), rounded to 14.7 kJ/mol and held constant.
const ETHANE_VAPORISATION_ENTHALPY: f64 = 14_700.0;
/// Source: critical temperature of ethane, 305.3 K (average of 41
/// values, NIST Chemistry WebBook; study 24, finding 9).
const ETHANE_CRITICAL_TEMPERATURE: f64 = 305.3;
/// Source: triple-point temperature of ethane, 90.4 K (Lemmon et al.
/// 2013; study 24, finding 9).
pub const ETHANE_TRIPLE_TEMPERATURE: Kelvin = Kelvin(90.4);
/// Definition: one standard atmosphere, 101 325 Pa, the pressure of the
/// normal boiling point.
const STANDARD_ATMOSPHERE: f64 = 101_325.0;

/// Boiling temperature of ethane at pressure `p`: Clausius-Clapeyron
/// from the normal boiling point (184.6 K at 101 325 Pa) with a constant
/// enthalpy of 14.7 kJ/mol, capped at the critical temperature (305.3
/// K). A pressure at or under zero reads as the smallest positive `f64`.
pub fn ethane_boiling_temperature(p: Pascal) -> Kelvin {
    let ratio = p.0.max(f64::MIN_POSITIVE) / STANDARD_ATMOSPHERE;
    let inverse = 1.0 / ETHANE_BOILING_TEMPERATURE
        - ratio.ln() * MOLAR_GAS_CONSTANT / ETHANE_VAPORISATION_ENTHALPY;
    if inverse <= 1.0 / ETHANE_CRITICAL_TEMPERATURE {
        Kelvin(ETHANE_CRITICAL_TEMPERATURE)
    } else {
        Kelvin(1.0 / inverse)
    }
}

#[cfg(test)]
#[path = "volatiles_tests.rs"]
mod tests;
