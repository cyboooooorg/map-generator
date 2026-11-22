//! Saturation of the gases that condense on the surface or in the column:
//! carbon dioxide and methane.
//!
//! Each curve is a vapour-pressure equation of the form
//! `p = p_c exp(T_c / T * sum n_i (1 - T/T_c)^t_i)` from the triple to the
//! critical point, joined below the triple point by a sublimation line
//! (Clausius-Clapeyron from the triple point with a constant enthalpy), and
//! infinite past the critical point, where the gas does not condense.
//!
//! The coefficients are those of the saturation ancillary of CoolProp
//! v6.6.0 (`dev/fluids/CarbonDioxide.json` and `Methane.json`, identical
//! in v8.0.0), which fits the vapour-pressure curve of the reference
//! equations of state: Span and Wagner 1996 (doi:10.1063/1.555991) for
//! CO2, Setzmann and Wagner 1991 (doi:10.1063/1.555898) for methane. The
//! papers are paywalled; the check tables of the tests are the NIST
//! WebBook's, which use the same equations of state. The CoolProp files
//! give a maximum error of 0.001 % (CO2) and 0.004 % (methane) against the
//! equations of state. They give no sublimation curve.

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
/// (Span and Wagner 1996, doi:10.1063/1.555991, as transcribed in
/// CoolProp v6.6.0, `dev/fluids/CarbonDioxide.json`, `ANCILLARIES.pS.n`;
/// the paper is paywalled).
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
/// Source: sublimation enthalpy of CO2, 26.1 kJ/mol at 207 K (Stephenson
/// and Malanowski 1987, as listed by the NIST Chemistry WebBook, phase
/// change data of CO2; other entries 25.2 to 27.2). Held constant below
/// the triple point.
const CO2_SUBLIMATION_ENTHALPY: f64 = 26_100.0;

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
/// as [`CO2_VAPOUR_N`]; Setzmann and Wagner 1991, doi:10.1063/1.555898, as
/// transcribed in CoolProp v6.6.0, `dev/fluids/Methane.json`,
/// `ANCILLARIES.pS.n`; the paper is paywalled).
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
/// triple point; neither CoolProp nor the equation of state gives a
/// sublimation curve.
const METHANE_SUBLIMATION_ENTHALPY: f64 = 9_700.0;

/// Definition: smallest temperature of the sublimation lines, 1 K, so that
/// 0 K, a negative or a NaN temperature give a finite pressure.
const FLOOR_TEMPERATURE: f64 = 1.0;

/// One gas's saturation curve.
struct Curve {
    critical_temperature: f64,
    critical_pa: f64,
    triple_temperature: f64,
    n: [f64; 6],
    t: [f64; 6],
    sublimation_enthalpy: f64,
}

const CO2: Curve = Curve {
    critical_temperature: CO2_CRITICAL_TEMPERATURE,
    critical_pa: CO2_CRITICAL_PRESSURE_PA,
    triple_temperature: CO2_TRIPLE_TEMPERATURE,
    n: CO2_VAPOUR_N,
    t: CO2_VAPOUR_T,
    sublimation_enthalpy: CO2_SUBLIMATION_ENTHALPY,
};

const METHANE: Curve = Curve {
    critical_temperature: METHANE_CRITICAL_TEMPERATURE,
    critical_pa: METHANE_CRITICAL_PRESSURE_PA,
    triple_temperature: METHANE_TRIPLE_TEMPERATURE,
    n: METHANE_VAPOUR_N,
    t: METHANE_VAPOUR_T,
    sublimation_enthalpy: METHANE_SUBLIMATION_ENTHALPY,
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
        let triple = self.critical_pa * self.vapour_sum(self.triple_temperature).0.exp();
        let t = t_k.max(FLOOR_TEMPERATURE);
        triple
            * (-self.sublimation_enthalpy / MOLAR_GAS_CONSTANT
                * (1.0 / t - 1.0 / self.triple_temperature))
                .exp()
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
/// [`crate::water::vapour_latent_j_mol`] does: the sublimation enthalpy
/// below the triple point, the vaporisation above, 0 from the critical
/// point up (the legacy clamp). The moist adiabat that uses it then agrees
/// with the curve. CO2's vapour is not ideal at its saturation pressures
/// (17.85 bar at 250 K): this is 16.4 kJ/mol there where the real
/// enthalpy of vaporisation is 12.7 kJ/mol; the legacy mean was 16.6.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the moist adiabat of the longwave column reads it"
    )
)]
pub(crate) fn co2_latent_j_mol(t_k: f64) -> f64 {
    if t_k >= CO2.critical_temperature {
        0.0
    } else if t_k >= CO2.triple_temperature {
        MOLAR_GAS_CONSTANT * t_k * t_k * CO2.vapour_sum(t_k).1
    } else {
        CO2.sublimation_enthalpy
    }
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
