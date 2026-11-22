//! Saturation of the gases that condense on the surface or in the column:
//! carbon dioxide and methane.
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
use crate::units::{Kelvin, Pascal};

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
const METHANE_TRIPLE_TEMPERATURE: f64 = 90.6941;
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

#[cfg(test)]
#[path = "volatiles_tests.rs"]
mod tests;
