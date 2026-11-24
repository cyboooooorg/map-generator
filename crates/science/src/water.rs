//! Water below 2.2 GPa: liquid water (IAPWS-95), ice Ih (Feistel and Wagner
//! 2006), the melting curves of ices Ih, III, V and VI (IAPWS 2011), the
//! melting temperature of the ice stable at a pressure
//! ([`melting_temperature`]) and the ice VI / ice VII boundary (AQUA,
//! Haldemann et al. 2020). Ice VII's melting curve (IAPWS R14-08 eq. 5) is
//! not ported: above 2216 MPa the melting temperature is held at 355 K.
//!
//! The materials built from these laws are in [`crate::eos`]. Values and
//! their check tables are in the study
//! `docs/superpowers/studies/2026-11-21-planet-interior/equations-of-state.md`,
//! section 2.7.

use crate::constants::{MOLAR_GAS_CONSTANT, PASCALS_PER_BAR, STANDARD_ATMOSPHERE};
use crate::units::{Kelvin, KilogramPerCubicMetre, Pascal};

/// Source: ice Ih - ice III - liquid triple point, 251.165 K (IAPWS
/// R14-08(2011), Table 1).
pub const IH_III_LIQUID_TEMPERATURE: Kelvin = Kelvin(251.165);
/// Source: ice Ih - ice III - liquid triple point, 208.566 MPa, computed by
/// IAPWS to join its melting curves of ices Ih and III (IAPWS R14-08(2011),
/// Table 1).
pub const IH_III_LIQUID_PRESSURE: Pascal = Pascal(208.566e6);
/// Source: ice III - ice V - liquid triple point, 256.164 K (IAPWS
/// R14-08(2011), Table 1).
pub const III_V_LIQUID_TEMPERATURE: Kelvin = Kelvin(256.164);
/// Source: ice III - ice V - liquid triple point, 350.1 MPa (IAPWS
/// R14-08(2011), Table 1).
pub const III_V_LIQUID_PRESSURE: Pascal = Pascal(350.1e6);
/// Source: ice V - ice VI - liquid triple point, 273.31 K (IAPWS
/// R14-08(2011), Table 1).
pub const V_VI_LIQUID_TEMPERATURE: Kelvin = Kelvin(273.31);
/// Source: ice V - ice VI - liquid triple point, 632.4 MPa (IAPWS
/// R14-08(2011), Table 1).
pub const V_VI_LIQUID_PRESSURE: Pascal = Pascal(632.4e6);
/// Source: ice VI - ice VII - liquid triple point, 355 K (IAPWS
/// R14-08(2011), Table 1).
pub const VI_VII_LIQUID_TEMPERATURE: Kelvin = Kelvin(355.0);
/// Source: ice VI - ice VII - liquid triple point, 2216 MPa (IAPWS
/// R14-08(2011), Table 1).
pub const VI_VII_LIQUID_PRESSURE: Pascal = Pascal(2216e6);
/// Source: triple point of ice Ih, liquid and vapour, 273.16 K (IAPWS
/// R10-06(2009), Table 1).
pub const TRIPLE_POINT_TEMPERATURE: Kelvin = Kelvin(273.16);
/// Source: triple point of ice Ih, liquid and vapour, 611.657 Pa (IAPWS
/// R10-06(2009), Table 1).
pub const TRIPLE_POINT_PRESSURE: Pascal = Pascal(611.657);

/// Definition: iteration count of the bisections of this module; 200
/// halvings exceed `f64` resolution on any bracket used here.
const BISECTIONS: usize = 200;

// ---------------------------------------------------------------- IAPWS-95

/// Source: critical temperature, 647.096 K (IAPWS R6-95(2018), eq. 1).
const CRITICAL_TEMPERATURE: f64 = 647.096;
/// Source: critical density, 322 kg/m3 (IAPWS R6-95(2018), eq. 2).
const CRITICAL_DENSITY: f64 = 322.0;
/// Source: specific gas constant of IAPWS-95, 0.461 518 05 kJ/(kg K), in
/// J/(kg K); the release requires this value with its coefficients
/// (IAPWS R6-95(2018), eq. 3).
const SPECIFIC_GAS_CONSTANT: f64 = 461.518_05;

/// A term `n delta^d tau^t`, times `exp(-delta^c)` when `c > 0`, of the
/// residual Helmholtz energy (IAPWS R6-95(2018), eq. 6, terms 1 to 51).
struct PowerTerm {
    c: i32,
    d: i32,
    t: f64,
    n: f64,
}

/// A Gaussian term `n delta^d tau^t exp(-alpha (delta - epsilon)^2 -
/// beta (tau - gamma)^2)` (IAPWS R6-95(2018), eq. 6, terms 52 to 54).
struct GaussianTerm {
    d: i32,
    t: f64,
    n: f64,
    alpha: f64,
    beta: f64,
    gamma: f64,
    epsilon: f64,
}

/// A non-analytic term `n Delta^b delta psi` (IAPWS R6-95(2018), eq. 6,
/// terms 55 and 56), with `theta = (1 - tau) + A ((delta - 1)^2)^(1/(2 beta))`,
/// `Delta = theta^2 + B ((delta - 1)^2)^a` and
/// `psi = exp(-C (delta - 1)^2 - D (tau - 1)^2)`.
struct NonAnalyticTerm {
    a: f64,
    b: f64,
    big_b: f64,
    n: f64,
    big_c: f64,
    big_d: f64,
    big_a: f64,
    beta: f64,
}

/// Source: coefficients `c_i, d_i, t_i, n_i` of terms 1 to 51 of the
/// residual part, `c_i = 0` for terms 1 to 7 which have none (IAPWS
/// R6-95(2018), Table 2).
const POWER_TERMS: [PowerTerm; 51] = [
    PowerTerm {
        c: 0,
        d: 1,
        t: -0.5,
        n: 0.125_335_479_355_23e-1,
    },
    PowerTerm {
        c: 0,
        d: 1,
        t: 0.875,
        n: 0.789_576_347_228_28e1,
    },
    PowerTerm {
        c: 0,
        d: 1,
        t: 1.0,
        n: -0.878_032_033_035_61e1,
    },
    PowerTerm {
        c: 0,
        d: 2,
        t: 0.5,
        n: 0.318_025_093_454_18,
    },
    PowerTerm {
        c: 0,
        d: 2,
        t: 0.75,
        n: -0.261_455_338_593_58,
    },
    PowerTerm {
        c: 0,
        d: 3,
        t: 0.375,
        n: -0.781_997_516_879_81e-2,
    },
    PowerTerm {
        c: 0,
        d: 4,
        t: 1.0,
        n: 0.880_894_931_021_34e-2,
    },
    PowerTerm {
        c: 1,
        d: 1,
        t: 4.0,
        n: -0.668_565_723_079_65,
    },
    PowerTerm {
        c: 1,
        d: 1,
        t: 6.0,
        n: 0.204_338_109_509_65,
    },
    PowerTerm {
        c: 1,
        d: 1,
        t: 12.0,
        n: -0.662_126_050_396_87e-4,
    },
    PowerTerm {
        c: 1,
        d: 2,
        t: 1.0,
        n: -0.192_327_211_560_02,
    },
    PowerTerm {
        c: 1,
        d: 2,
        t: 5.0,
        n: -0.257_090_430_034_38,
    },
    PowerTerm {
        c: 1,
        d: 3,
        t: 4.0,
        n: 0.160_748_684_862_51,
    },
    PowerTerm {
        c: 1,
        d: 4,
        t: 2.0,
        n: -0.400_928_289_258_07e-1,
    },
    PowerTerm {
        c: 1,
        d: 4,
        t: 13.0,
        n: 0.393_434_226_032_54e-6,
    },
    PowerTerm {
        c: 1,
        d: 5,
        t: 9.0,
        n: -0.759_413_770_881_44e-5,
    },
    PowerTerm {
        c: 1,
        d: 7,
        t: 3.0,
        n: 0.562_509_793_518_88e-3,
    },
    PowerTerm {
        c: 1,
        d: 9,
        t: 4.0,
        n: -0.156_086_522_571_35e-4,
    },
    PowerTerm {
        c: 1,
        d: 10,
        t: 11.0,
        n: 0.115_379_964_229_51e-8,
    },
    PowerTerm {
        c: 1,
        d: 11,
        t: 4.0,
        n: 0.365_821_651_442_04e-6,
    },
    PowerTerm {
        c: 1,
        d: 13,
        t: 13.0,
        n: -0.132_511_800_746_68e-11,
    },
    PowerTerm {
        c: 1,
        d: 15,
        t: 1.0,
        n: -0.626_395_869_124_54e-9,
    },
    PowerTerm {
        c: 2,
        d: 1,
        t: 7.0,
        n: -0.107_936_009_089_32,
    },
    PowerTerm {
        c: 2,
        d: 2,
        t: 1.0,
        n: 0.176_114_910_087_52e-1,
    },
    PowerTerm {
        c: 2,
        d: 2,
        t: 9.0,
        n: 0.221_322_951_675_46,
    },
    PowerTerm {
        c: 2,
        d: 2,
        t: 10.0,
        n: -0.402_476_697_635_28,
    },
    PowerTerm {
        c: 2,
        d: 3,
        t: 10.0,
        n: 0.580_833_999_857_59,
    },
    PowerTerm {
        c: 2,
        d: 4,
        t: 3.0,
        n: 0.499_691_469_908_06e-2,
    },
    PowerTerm {
        c: 2,
        d: 4,
        t: 7.0,
        n: -0.313_587_007_125_49e-1,
    },
    PowerTerm {
        c: 2,
        d: 4,
        t: 10.0,
        n: -0.743_159_297_103_41,
    },
    PowerTerm {
        c: 2,
        d: 5,
        t: 10.0,
        n: 0.478_073_299_154_80,
    },
    PowerTerm {
        c: 2,
        d: 6,
        t: 6.0,
        n: 0.205_279_408_959_48e-1,
    },
    PowerTerm {
        c: 2,
        d: 6,
        t: 10.0,
        n: -0.136_364_351_103_43,
    },
    PowerTerm {
        c: 2,
        d: 7,
        t: 10.0,
        n: 0.141_806_344_006_17e-1,
    },
    PowerTerm {
        c: 2,
        d: 9,
        t: 1.0,
        n: 0.833_265_048_807_13e-2,
    },
    PowerTerm {
        c: 2,
        d: 9,
        t: 2.0,
        n: -0.290_523_360_095_85e-1,
    },
    PowerTerm {
        c: 2,
        d: 9,
        t: 3.0,
        n: 0.386_150_855_742_06e-1,
    },
    PowerTerm {
        c: 2,
        d: 9,
        t: 4.0,
        n: -0.203_934_865_137_04e-1,
    },
    PowerTerm {
        c: 2,
        d: 9,
        t: 8.0,
        n: -0.165_540_500_637_34e-2,
    },
    PowerTerm {
        c: 2,
        d: 10,
        t: 6.0,
        n: 0.199_555_719_795_41e-2,
    },
    PowerTerm {
        c: 2,
        d: 10,
        t: 9.0,
        n: 0.158_703_083_241_57e-3,
    },
    PowerTerm {
        c: 2,
        d: 12,
        t: 8.0,
        n: -0.163_885_683_425_30e-4,
    },
    PowerTerm {
        c: 3,
        d: 3,
        t: 16.0,
        n: 0.436_136_157_238_11e-1,
    },
    PowerTerm {
        c: 3,
        d: 4,
        t: 22.0,
        n: 0.349_940_054_637_65e-1,
    },
    PowerTerm {
        c: 3,
        d: 4,
        t: 23.0,
        n: -0.767_881_978_446_21e-1,
    },
    PowerTerm {
        c: 3,
        d: 5,
        t: 23.0,
        n: 0.224_462_773_320_06e-1,
    },
    PowerTerm {
        c: 4,
        d: 14,
        t: 10.0,
        n: -0.626_897_104_146_85e-4,
    },
    PowerTerm {
        c: 6,
        d: 3,
        t: 50.0,
        n: -0.557_111_185_656_45e-9,
    },
    PowerTerm {
        c: 6,
        d: 6,
        t: 44.0,
        n: -0.199_057_183_544_08,
    },
    PowerTerm {
        c: 6,
        d: 6,
        t: 46.0,
        n: 0.317_774_973_307_38,
    },
    PowerTerm {
        c: 6,
        d: 6,
        t: 50.0,
        n: -0.118_411_824_259_81,
    },
];

/// Source: coefficients `d_i, t_i, n_i, alpha_i, beta_i, gamma_i,
/// epsilon_i` of terms 52 to 54 (IAPWS R6-95(2018), Table 2).
const GAUSSIAN_TERMS: [GaussianTerm; 3] = [
    GaussianTerm {
        d: 3,
        t: 0.0,
        n: -0.313_062_603_234_35e2,
        alpha: 20.0,
        beta: 150.0,
        gamma: 1.21,
        epsilon: 1.0,
    },
    GaussianTerm {
        d: 3,
        t: 1.0,
        n: 0.315_461_402_377_81e2,
        alpha: 20.0,
        beta: 150.0,
        gamma: 1.21,
        epsilon: 1.0,
    },
    GaussianTerm {
        d: 3,
        t: 4.0,
        n: -0.252_131_543_416_95e4,
        alpha: 20.0,
        beta: 250.0,
        gamma: 1.25,
        epsilon: 1.0,
    },
];

/// Source: coefficients `a_i, b_i, B_i, n_i, C_i, D_i, A_i, beta_i` of
/// terms 55 and 56 (IAPWS R6-95(2018), Table 2).
const NON_ANALYTIC_TERMS: [NonAnalyticTerm; 2] = [
    NonAnalyticTerm {
        a: 3.5,
        b: 0.85,
        big_b: 0.2,
        n: -0.148_746_408_567_24,
        big_c: 28.0,
        big_d: 700.0,
        big_a: 0.32,
        beta: 0.3,
    },
    NonAnalyticTerm {
        a: 3.5,
        b: 0.95,
        big_b: 0.2,
        n: 0.318_061_108_784_44,
        big_c: 32.0,
        big_d: 800.0,
        big_a: 0.32,
        beta: 0.3,
    },
];

/// Derivative of the residual Helmholtz energy with respect to the reduced
/// density, `phir_delta(delta, tau)` (IAPWS R6-95(2018), Table 5).
fn residual_delta(delta: f64, tau: f64) -> f64 {
    let mut sum = 0.0;
    for term in &POWER_TERMS {
        let power = term.n * delta.powi(term.d - 1) * tau.powf(term.t);
        sum += if term.c == 0 {
            power * f64::from(term.d)
        } else {
            let delta_c = delta.powi(term.c);
            power * (-delta_c).exp() * (f64::from(term.d) - f64::from(term.c) * delta_c)
        };
    }
    for term in &GAUSSIAN_TERMS {
        let gauss = (-term.alpha * (delta - term.epsilon).powi(2)
            - term.beta * (tau - term.gamma).powi(2))
        .exp();
        sum += term.n
            * delta.powi(term.d)
            * tau.powf(term.t)
            * gauss
            * (f64::from(term.d) / delta - 2.0 * term.alpha * (delta - term.epsilon));
    }
    for term in &NON_ANALYTIC_TERMS {
        let offset = delta - 1.0;
        let square = offset * offset;
        let theta = (1.0 - tau) + term.big_a * square.powf(1.0 / (2.0 * term.beta));
        let big_delta = theta * theta + term.big_b * square.powf(term.a);
        let psi = (-term.big_c * square - term.big_d * (tau - 1.0).powi(2)).exp();
        let psi_delta = -2.0 * term.big_c * offset * psi;
        let big_delta_delta = offset
            * (term.big_a * theta * 2.0 / term.beta * square.powf(1.0 / (2.0 * term.beta) - 1.0)
                + 2.0 * term.big_b * term.a * square.powf(term.a - 1.0));
        let power_delta = term.b * big_delta.powf(term.b - 1.0) * big_delta_delta;
        sum += term.n
            * (big_delta.powf(term.b) * (psi + delta * psi_delta) + power_delta * delta * psi);
    }
    sum
}

/// Pressure of liquid water at temperature `t` and density `rho`,
/// `p = rho R T (1 + delta phir_delta)` (IAPWS-95, Wagner and Pruss 2002;
/// IAPWS R6-95(2018), Table 3). Valid to 1 GPa below 1273 K; the materials
/// of [`crate::eos`] use it below 2.216 GPa and 355 K.
pub fn liquid_pressure(t: Kelvin, rho: KilogramPerCubicMetre) -> Pascal {
    let delta = rho.0 / CRITICAL_DENSITY;
    let tau = CRITICAL_TEMPERATURE / t.0;
    Pascal(rho.0 * SPECIFIC_GAS_CONSTANT * t.0 * (1.0 + delta * residual_delta(delta, tau)))
}

// ------------------------------------------------------------------ ice Ih

/// Source: `g01` to `g04`, J/kg, the pressure coefficients of `g0(p)`
/// (IAPWS R10-06(2009), Table 2); `g00` drops out of the density.
const G0: [f64; 4] = [
    0.655_022_213_658_955,
    -0.189_369_929_326_131e-7,
    0.339_746_123_271_053e-14,
    -0.556_464_869_058_991e-21,
];
/// Source: complex constant `t2` (IAPWS R10-06(2009), Table 2).
const T2: Complex = Complex {
    re: 0.337_315_741_065_416,
    im: 0.335_449_415_919_309,
};
/// Source: `r20`, `r21`, `r22`, J/(kg K), coefficients of `r2(p)` (IAPWS
/// R10-06(2009), Table 2).
const R2: [Complex; 3] = [
    Complex {
        re: -0.725_974_574_329_220e2,
        im: -0.781_008_427_112_870e2,
    },
    Complex {
        re: -0.557_107_698_030_123e-4,
        im: 0.464_578_634_580_806e-4,
    },
    Complex {
        re: 0.234_801_409_215_913e-10,
        im: -0.285_651_142_904_972e-10,
    },
];

/// A complex number, for the Gibbs function of ice Ih written with complex
/// constants (IAPWS R10-06(2009), eq. 1); the logarithm is the principal
/// value.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Complex {
    re: f64,
    im: f64,
}

impl Complex {
    fn real(re: f64) -> Self {
        Self { re, im: 0.0 }
    }

    fn add(self, other: Self) -> Self {
        Self {
            re: self.re + other.re,
            im: self.im + other.im,
        }
    }

    fn sub(self, other: Self) -> Self {
        Self {
            re: self.re - other.re,
            im: self.im - other.im,
        }
    }

    fn mul(self, other: Self) -> Self {
        Self {
            re: self.re * other.re - self.im * other.im,
            im: self.re * other.im + self.im * other.re,
        }
    }

    fn scale(self, factor: f64) -> Self {
        Self {
            re: self.re * factor,
            im: self.im * factor,
        }
    }

    fn div(self, other: Self) -> Self {
        let norm = other.re * other.re + other.im * other.im;
        Self {
            re: (self.re * other.re + self.im * other.im) / norm,
            im: (self.im * other.re - self.re * other.im) / norm,
        }
    }

    fn ln(self) -> Self {
        Self {
            re: self.re.hypot(self.im).ln(),
            im: self.im.atan2(self.re),
        }
    }
}

/// Density of ice Ih at temperature `t` and pressure `p`, `rho = 1 / g_p`
/// from the Gibbs function of Feistel and Wagner 2006 (IAPWS R10-06(2009),
/// eq. 1, Tables 3 and 4).
pub fn ice_ih_density(t: Kelvin, p: Pascal) -> KilogramPerCubicMetre {
    let pt = TRIPLE_POINT_PRESSURE.0;
    let tau = Complex::real(t.0 / TRIPLE_POINT_TEMPERATURE.0);
    let excess = (p.0 - STANDARD_ATMOSPHERE.0) / pt;
    // g0_p = sum over k = 1..4 of k g0k / pt (pi - pi0)^(k - 1).
    let (mut g0_p, mut order, mut power) = (0.0, 1.0, 1.0);
    for coefficient in G0 {
        g0_p += coefficient * order / pt * power;
        order += 1.0;
        power *= excess;
    }
    let r2_p = R2[1].scale(1.0 / pt).add(R2[2].scale(2.0 / pt * excess));
    let bracket = T2
        .sub(tau)
        .mul(T2.sub(tau).ln())
        .add(T2.add(tau).mul(T2.add(tau).ln()))
        .sub(T2.scale(2.0).mul(T2.ln()))
        .sub(tau.mul(tau).div(T2));
    let g_p = g0_p + TRIPLE_POINT_TEMPERATURE.0 * r2_p.mul(bracket).re;
    KilogramPerCubicMetre(1.0 / g_p)
}

// ------------------------------------------------------- phase boundaries

/// Source: coefficients `a1, a2, a3` of the melting pressure of ice Ih
/// (IAPWS R14-08(2011), eq. 1).
const IH_MELTING_A: [f64; 3] = [0.119_539_337e7, 0.808_183_159e5, 0.333_826_860e4];
/// Source: exponents `b1, b2, b3` of the melting pressure of ice Ih (IAPWS
/// R14-08(2011), eq. 1).
const IH_MELTING_B: [f64; 3] = [3.0, 25.75, 103.75];
/// Source: coefficient 0.299948 of the melting pressure of ice III
/// (IAPWS R14-08(2011), eq. 2).
const III_MELTING_A: f64 = 0.299_948;
/// Source: exponent 60 of the melting pressure of ice III (IAPWS
/// R14-08(2011), eq. 2).
const III_MELTING_B: f64 = 60.0;
/// Source: coefficient 1.18721 of the melting pressure of ice V (IAPWS
/// R14-08(2011), eq. 3).
const V_MELTING_A: f64 = 1.187_21;
/// Source: exponent 8 of the melting pressure of ice V (IAPWS
/// R14-08(2011), eq. 3).
const V_MELTING_B: f64 = 8.0;
/// Source: coefficient 1.07476 of the melting pressure of ice VI (IAPWS
/// R14-08(2011), eq. 4).
const VI_MELTING_A: f64 = 1.074_76;
/// Source: exponent 4.6 of the melting pressure of ice VI (IAPWS
/// R14-08(2011), eq. 4).
const VI_MELTING_B: f64 = 4.6;

/// Melting pressure of ice III, `pi = 1 - 0.299948 (1 - theta^60)` with
/// `theta = T / 251.165 K`, `pi = p / 208.566 MPa` (IAPWS R14-08(2011),
/// eq. 2), valid from 251.165 K to 256.164 K.
pub fn ice_iii_melting_pressure(t: Kelvin) -> Pascal {
    let theta = t.0 / IH_III_LIQUID_TEMPERATURE.0;
    Pascal(IH_III_LIQUID_PRESSURE.0 * (1.0 - III_MELTING_A * (1.0 - theta.powf(III_MELTING_B))))
}

/// Melting pressure of ice V, `pi = 1 - 1.18721 (1 - theta^8)` with
/// `theta = T / 256.164 K`, `pi = p / 350.1 MPa` (IAPWS R14-08(2011),
/// eq. 3), valid from 256.164 K to 273.31 K.
pub fn ice_v_melting_pressure(t: Kelvin) -> Pascal {
    let theta = t.0 / III_V_LIQUID_TEMPERATURE.0;
    Pascal(III_V_LIQUID_PRESSURE.0 * (1.0 - V_MELTING_A * (1.0 - theta.powf(V_MELTING_B))))
}

/// Inverts `pi = 1 - a (1 - theta^b)` for `T = theta T*`.
fn invert_melting(p: Pascal, a: f64, b: f64, t_star: Kelvin, p_star: Pascal) -> Kelvin {
    let pi = p.0 / p_star.0;
    Kelvin(t_star.0 * (1.0 - (1.0 - pi) / a).powf(1.0 / b))
}

/// Melting temperature of the ice stable in equilibrium with the liquid
/// at pressure `p`: ice Ih up to 208.566 MPa, III up to 350.1 MPa, V up
/// to 632.4 MPa, VI up to 2216 MPa (IAPWS R14-08(2011), Table 1 and eqs.
/// 1 to 4, III to VI inverted in closed form).
///
/// Limit: ice VII (eq. 5) is not ported, a choice; above 2216 MPa the
/// temperature is held at the ice VI - ice VII - liquid triple point,
/// 355 K.
pub fn melting_temperature(p: Pascal) -> Kelvin {
    if p.0 <= IH_III_LIQUID_PRESSURE.0 {
        ice_ih_melting_temperature(p)
    } else if p.0 <= III_V_LIQUID_PRESSURE.0 {
        invert_melting(
            p,
            III_MELTING_A,
            III_MELTING_B,
            IH_III_LIQUID_TEMPERATURE,
            IH_III_LIQUID_PRESSURE,
        )
    } else if p.0 <= V_VI_LIQUID_PRESSURE.0 {
        invert_melting(
            p,
            V_MELTING_A,
            V_MELTING_B,
            III_V_LIQUID_TEMPERATURE,
            III_V_LIQUID_PRESSURE,
        )
    } else if p.0 <= VI_VII_LIQUID_PRESSURE.0 {
        invert_melting(
            p,
            VI_MELTING_A,
            VI_MELTING_B,
            V_VI_LIQUID_TEMPERATURE,
            V_VI_LIQUID_PRESSURE,
        )
    } else {
        VI_VII_LIQUID_TEMPERATURE
    }
}

/// Melting pressure of ice Ih, `pi = 1 + sum a_i (1 - theta^b_i)` with
/// `theta = T / 273.16 K`, `pi = p / 611.657 Pa` (IAPWS R14-08(2011),
/// eq. 1), valid from 251.165 K to 273.16 K.
pub fn ice_ih_melting_pressure(t: Kelvin) -> Pascal {
    let theta = t.0 / TRIPLE_POINT_TEMPERATURE.0;
    let mut pi = 1.0;
    for (a, b) in IH_MELTING_A.iter().zip(IH_MELTING_B) {
        pi += a * (1.0 - theta.powf(b));
    }
    Pascal(pi * TRIPLE_POINT_PRESSURE.0)
}

/// Melting temperature of ice Ih at pressure `p`: the inverse of
/// [`ice_ih_melting_pressure`] by bisection, 273.16 K at and below the
/// triple-point pressure and 251.165 K at and above the ice Ih - ice III -
/// liquid triple point (the ends of the curve).
pub fn ice_ih_melting_temperature(p: Pascal) -> Kelvin {
    if p.0 <= TRIPLE_POINT_PRESSURE.0 {
        return TRIPLE_POINT_TEMPERATURE;
    }
    if p.0 >= IH_III_LIQUID_PRESSURE.0 {
        return IH_III_LIQUID_TEMPERATURE;
    }
    let (mut cold, mut warm) = (IH_III_LIQUID_TEMPERATURE.0, TRIPLE_POINT_TEMPERATURE.0);
    for _ in 0..BISECTIONS {
        let mid = 0.5 * (cold + warm);
        // The melting pressure falls as the temperature rises.
        if ice_ih_melting_pressure(Kelvin(mid)).0 > p.0 {
            cold = mid;
        } else {
            warm = mid;
        }
    }
    Kelvin(0.5 * (cold + warm))
}

/// Melting pressure of ice VI, `pi = 1 - 1.07476 (1 - theta^4.6)` with
/// `theta = T / 273.31 K`, `pi = p / 632.4 MPa` (IAPWS R14-08(2011),
/// eq. 4), valid from 273.31 K to 355 K.
pub fn ice_vi_melting_pressure(t: Kelvin) -> Pascal {
    let theta = t.0 / V_VI_LIQUID_TEMPERATURE.0;
    Pascal(V_VI_LIQUID_PRESSURE.0 * (1.0 - VI_MELTING_A * (1.0 - theta.powf(VI_MELTING_B))))
}

/// Source: coefficients `x1` to `x4`, K, of the ice VI / ice VII boundary
/// `T = x1 + x2 P + x3 ln P + x4 sqrt(P)`, P in Pa, fitted by Haldemann et
/// al. 2020 (AQUA, A&A 643, A105, eq. 22, Table 2) up to the triple point at
/// 2.216 GPa; the logarithm is natural (it gives 355.0 K at 2.216 GPa).
const VI_VII_BOUNDARY: [f64; 4] = [-1.4699e5, 6.107_91e-6, 8.1529e3, -8.8439e-1];

/// Source: lowest pressure of the bisection of [`ice_vi_vii_pressure`],
/// the ice V - ice VI - liquid triple point (IAPWS R14-08(2011), Table 1):
/// ice VI starts there on the melting curve.
const VI_VII_SEARCH_FLOOR: Pascal = V_VI_LIQUID_PRESSURE;

/// Temperature of the ice VI / ice VII boundary at pressure `p` (AQUA,
/// Haldemann et al. 2020, eq. 22).
pub fn ice_vi_vii_temperature(p: Pascal) -> Kelvin {
    let [x1, x2, x3, x4] = VI_VII_BOUNDARY;
    Kelvin(x1 + x2 * p.0 + x3 * p.0.ln() + x4 * p.0.sqrt())
}

/// Pressure of the ice VI / ice VII boundary at temperature `t`: the
/// inverse of [`ice_vi_vii_temperature`] by bisection between 632.4 MPa and
/// the triple point at 2216 MPa, where the boundary temperature rises with
/// pressure.
pub fn ice_vi_vii_pressure(t: Kelvin) -> Pascal {
    let (mut low, mut high) = (VI_VII_SEARCH_FLOOR.0, VI_VII_LIQUID_PRESSURE.0);
    for _ in 0..BISECTIONS {
        let mid = 0.5 * (low + high);
        if ice_vi_vii_temperature(Pascal(mid)).0 < t.0 {
            low = mid;
        } else {
            high = mid;
        }
    }
    Pascal(0.5 * (low + high))
}

// ------------------------------------------- saturation and sublimation

/// Source: critical temperature, 647.096 K (IAPWS R6-95(2018), eq. 1;
/// IAPWS SR1-86(1992), section 2), where the saturation curve ends.
pub const CRITICAL_POINT_TEMPERATURE: Kelvin = Kelvin(CRITICAL_TEMPERATURE);
/// Source: critical pressure, 22.064 MPa (IAPWS SR1-86(1992), section 2,
/// identical to the IAPS 1983 statement of the critical point).
pub const CRITICAL_PRESSURE: Pascal = Pascal(22.064e6);

/// Source: coefficients `a1` to `a6` of the vapour pressure over liquid
/// water (IAPWS SR1-86(1992), eq. 1; Wagner and Pruss 1993,
/// doi:10.1063/1.555926).
const SATURATION_A: [f64; 6] = [
    -7.859_517_83,
    1.844_082_59,
    -11.786_649_7,
    22.680_741_1,
    -15.961_871_9,
    1.801_225_02,
];
/// Source: exponents of `tau` in the vapour pressure over liquid water,
/// 1, 1.5, 3, 3.5, 4 and 7.5 (IAPWS SR1-86(1992), eq. 1).
const SATURATION_N: [f64; 6] = [1.0, 1.5, 3.0, 3.5, 4.0, 7.5];

/// Source: coefficients `a1` to `a3` of the sublimation pressure of ice Ih
/// (IAPWS R14-08(2011), eq. 6; Wagner et al. 2011, doi:10.1063/1.3657937,
/// Table 3).
const SUBLIMATION_A: [f64; 3] = [-0.212_144_006e2, 0.273_203_819e2, -0.610_598_130e1];
/// Source: exponents `b1` to `b3` of the sublimation pressure of ice Ih
/// (IAPWS R14-08(2011), eq. 6; Wagner et al. 2011, Table 3).
const SUBLIMATION_B: [f64; 3] = [0.333_333_333e-2, 0.120_666_667e1, 0.170_333_333e1];

/// The sum `f = sum a_i tau^n_i` of the vapour pressure over liquid,
/// `ln(p / pc) = (Tc / T) f` with `tau = 1 - T / Tc` (IAPWS SR1-86(1992),
/// eq. 1), and its derivative `df / dtau`.
fn saturation_sum(t_k: f64) -> (f64, f64) {
    let tau = 1.0 - t_k / CRITICAL_TEMPERATURE;
    let mut f = 0.0;
    let mut slope = 0.0;
    for (a, n) in SATURATION_A.iter().zip(SATURATION_N) {
        f += a * tau.powf(n);
        slope += a * n * tau.powf(n - 1.0);
    }
    (f, slope)
}

/// Vapour pressure over liquid water, Pa, at `t_k` (IAPWS SR1-86(1992),
/// eq. 1), valid from 273.16 K to 647.096 K.
fn saturation_pa(t_k: f64) -> f64 {
    let (f, _) = saturation_sum(t_k);
    CRITICAL_PRESSURE.0 * (CRITICAL_TEMPERATURE / t_k * f).exp()
}

/// Sublimation pressure of ice Ih, Pa, at `t_k`:
/// `ln(p / pt) = theta^-1 sum a_i theta^b_i`, `theta = T / 273.16 K`
/// (IAPWS R14-08(2011), eq. 6), valid from 50 K to 273.16 K. It is 0 at
/// and below 0 K, where the equation has no value.
fn sublimation_pa(t_k: f64) -> f64 {
    if t_k.is_nan() || t_k <= 0.0 {
        return 0.0;
    }
    let theta = t_k / TRIPLE_POINT_TEMPERATURE.0;
    let mut sum = 0.0;
    for (a, b) in SUBLIMATION_A.iter().zip(SUBLIMATION_B) {
        sum += a * theta.powf(b);
    }
    TRIPLE_POINT_PRESSURE.0 * (sum / theta).exp()
}

/// Vapour pressure over liquid water at `t` (Wagner and Pruss 1993,
/// IAPWS SR1-86(1992), eq. 1), valid from the triple point (273.16 K) to
/// the critical point (647.096 K).
pub fn saturation_pressure(t: Kelvin) -> Pascal {
    Pascal(saturation_pa(t.0))
}

/// Boiling temperature of water at pressure `p`: the inverse of
/// [`saturation_pressure`] by bisection on the triple point to the critical
/// point (the same IAPWS curve, no second one). `None` under the
/// triple-point pressure, where water does not boil; the critical
/// temperature at and above the critical pressure.
pub fn boiling_temperature(p: Pascal) -> Option<Kelvin> {
    if p.0 < TRIPLE_POINT_PRESSURE.0 {
        return None;
    }
    if p.0 >= CRITICAL_PRESSURE.0 {
        return Some(CRITICAL_POINT_TEMPERATURE);
    }
    let (mut cold, mut warm) = (TRIPLE_POINT_TEMPERATURE.0, CRITICAL_POINT_TEMPERATURE.0);
    for _ in 0..BISECTIONS {
        let mid = 0.5 * (cold + warm);
        if saturation_pressure(Kelvin(mid)).0 < p.0 {
            cold = mid;
        } else {
            warm = mid;
        }
    }
    Some(Kelvin(0.5 * (cold + warm)))
}

/// Sublimation pressure of ice Ih at `t` (Wagner et al. 2011, IAPWS
/// R14-08(2011), eq. 6), valid from 50 K to the triple point; 0 at 0 K.
pub fn sublimation_pressure(t: Kelvin) -> Pascal {
    Pascal(sublimation_pa(t.0))
}

/// Pressure of water vapour in equilibrium with its condensate at `t`:
/// [`vapour_saturation_bar`] in pascals.
pub fn vapour_saturation_pressure(t: Kelvin) -> Pascal {
    Pascal(vapour_saturation_bar(t.0) * PASCALS_PER_BAR)
}

/// Pressure of water vapour in equilibrium with its condensate at `t_k`,
/// bar: the sublimation curve of ice Ih below the triple point, the
/// saturation curve over liquid up to the critical point, and the
/// critical pressure above it, where no liquid exists (the legacy clamp,
/// `water_saturation_bar`). 0 at and below 0 K.
pub(crate) fn vapour_saturation_bar(t_k: f64) -> f64 {
    let pa = if t_k.is_nan() || t_k < TRIPLE_POINT_TEMPERATURE.0 {
        sublimation_pa(t_k)
    } else if t_k < CRITICAL_TEMPERATURE {
        saturation_pa(t_k)
    } else {
        CRITICAL_PRESSURE.0
    };
    pa / PASCALS_PER_BAR
}

/// Molar latent heat of water, J/mol, at `t_k`, taken from the slope of
/// [`vapour_saturation_bar`] by Clausius-Clapeyron for an ideal vapour over
/// a condensate of negligible volume, `L = R T^2 d ln e_s / dT`
/// (Pierrehumbert 2010, section 2.6), the derivative taken analytically
/// from the equation of each branch: sublimation below the triple point,
/// vaporisation above. The moist adiabat that uses it then agrees with the
/// curve, as the legacy model's single latent heat did. Above the critical
/// temperature it keeps its value there (the legacy clamp). The ideal-vapour
/// form does not fall to zero at the critical point as the real latent heat
/// does: it gives `-R Tc a1`, 42.3 kJ/mol.
pub(crate) fn vapour_latent_j_mol(t_k: f64) -> f64 {
    // A NaN temperature reads as 0 K, like the pressure's.
    let t = if t_k.is_nan() {
        0.0
    } else {
        t_k.clamp(0.0, CRITICAL_TEMPERATURE)
    };
    if t < TRIPLE_POINT_TEMPERATURE.0 {
        // d ln p / dT = (1/Tt) sum a_i (b_i - 1) theta^(b_i - 2).
        let theta = t / TRIPLE_POINT_TEMPERATURE.0;
        let mut sum = 0.0;
        for (a, b) in SUBLIMATION_A.iter().zip(SUBLIMATION_B) {
            sum += a * (b - 1.0) * theta.powf(b);
        }
        MOLAR_GAS_CONSTANT * TRIPLE_POINT_TEMPERATURE.0 * sum
    } else {
        // d ln p / dT = -(Tc f) / T^2 - f'(tau) / T.
        let (f, slope) = saturation_sum(t);
        MOLAR_GAS_CONSTANT * (-CRITICAL_TEMPERATURE * f - t * slope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Source: normal boiling point of water, 373.124 K at 101 325 Pa
    /// (IAPWS SR1-86(1992), Table 1 of the saturation line).
    const NORMAL_BOILING: (f64, f64) = (373.124, 101_325.0);
    /// Definition: tolerance on the normal boiling point, 0.01 K.
    const NORMAL_BOILING_TOLERANCE: f64 = 0.01;
    /// Definition: pressures, Pa, of the round trip: 1 kPa, 100 kPa,
    /// 10 MPa.
    const BOILING_ROUND_TRIP: [f64; 3] = [1e3, 1e5, 1e7];

    #[test]
    fn water_boils_at_its_normal_point() {
        let t = boiling_temperature(Pascal(NORMAL_BOILING.1)).unwrap();
        assert!(
            (t.0 - NORMAL_BOILING.0).abs() < NORMAL_BOILING_TOLERANCE,
            "{}",
            t.0
        );
        assert!(boiling_temperature(Pascal(TRIPLE_POINT_PRESSURE.0 * 0.999)).is_none());
        assert_eq!(
            boiling_temperature(CRITICAL_PRESSURE),
            Some(CRITICAL_POINT_TEMPERATURE)
        );
        assert_eq!(
            boiling_temperature(Pascal(2.0 * CRITICAL_PRESSURE.0)),
            Some(CRITICAL_POINT_TEMPERATURE)
        );
        for p in BOILING_ROUND_TRIP {
            let t = boiling_temperature(Pascal(p)).unwrap();
            let back = saturation_pressure(t).0;
            assert!(((back - p) / p).abs() < 1e-9, "{p} Pa -> {back}");
        }
    }

    /// Source: IAPWS R6-95(2018), Table 7: (T in K, rho in kg/m3, p in Pa)
    /// in the single-phase region.
    const IAPWS95_CHECKS: [(f64, f64, f64); 7] = [
        (300.0, 0.996_556_0e3, 0.992_418_352e-1 * 1e6),
        (300.0, 0.100_530_8e4, 0.200_022_515e2 * 1e6),
        (300.0, 0.118_820_2e4, 0.700_004_704e3 * 1e6),
        (500.0, 0.838_025_0e3, 0.100_003_858e2 * 1e6),
        (500.0, 0.108_456_4e4, 0.700_000_405e3 * 1e6),
        (647.0, 0.358_000_0e3, 0.220_384_756e2 * 1e6),
        (900.0, 0.870_769_0e3, 0.700_000_006e3 * 1e6),
    ];
    /// Source: `phir_delta = -0.364 366 650` at 500 K and 838.025 kg/m3
    /// (IAPWS R6-95(2018), Table 6).
    const IAPWS95_RESIDUAL_DELTA: f64 = -0.364_366_650;
    /// Source: density 838.025 kg/m3 of the same check point.
    const IAPWS95_CHECK_DENSITY: f64 = 838.025;
    /// Source: temperature 500 K of the same check point.
    const IAPWS95_CHECK_TEMPERATURE: f64 = 500.0;
    /// Definition: relative tolerance against the nine printed digits of
    /// the IAPWS-95 checks (the release warns that the low-pressure liquid
    /// value may lose a digit).
    const NINE_DIGITS: f64 = 1e-8;

    /// Source: IAPWS R10-06(2009), Table 6: (T in K, p in Pa, rho in kg/m3)
    /// at the triple point, the normal melting point and 100 K, 100 MPa.
    const ICE_IH_CHECKS: [(f64, f64, f64); 3] = [
        (273.16, 611.657, 0.916_709_492_200e3),
        (273.152_519, 101_325.0, 0.916_721_463_419e3),
        (100.0, 1e8, 0.941_678_203_297e3),
    ];
    /// Definition: relative tolerance against the twelve printed digits of
    /// the ice Ih checks.
    const TWELVE_DIGITS: f64 = 1e-11;

    /// Source: melting pressure of ice Ih, 138.268 MPa at 260 K (IAPWS
    /// R14-08(2011), Table 3).
    const IH_MELTING_CHECK: (f64, f64) = (260.0, 138.268e6);
    /// Source: melting pressure of ice VI, 1356.76 MPa at 320 K (IAPWS
    /// R14-08(2011), Table 3).
    const VI_MELTING_CHECK: (f64, f64) = (320.0, 1356.76e6);
    /// Source: normal-pressure melting temperature of ice Ih, 273.152 519 K
    /// (IAPWS R10-06(2009), section 1).
    const NORMAL_MELTING_TEMPERATURE: f64 = 273.152_519;
    /// Definition: relative tolerance against the six printed digits of the
    /// melting checks.
    const SIX_DIGITS: f64 = 1e-5;
    /// Definition: temperature tolerance of the VI / VII boundary at its
    /// triple point, a tenth of the 0.1 K resolution of the printed fit
    /// (four or five significant figures).
    const BOUNDARY_TOLERANCE: f64 = 0.01;

    /// Source: IAPWS SR1-86(1992), Table 1: (T in K, vapour pressure in Pa)
    /// at the triple point, the normal boiling point and the critical
    /// point.
    const SATURATION_CHECKS: [(f64, f64); 3] = [
        (273.16, 611.657),
        (373.124_3, 0.101_325e6),
        (647.096, 22.064e6),
    ];
    /// Source: slope of the vapour pressure at the triple point,
    /// 44.436 693 Pa/K (IAPWS SR1-86(1992), Table 1).
    const SATURATION_SLOPE_AT_TRIPLE_POINT: f64 = 44.436_693;
    /// Source: sublimation pressure of ice Ih, 8.947 35e-6 MPa at 230 K
    /// (IAPWS R14-08(2011), Table 3; Wagner et al. 2011, section 3), in Pa.
    const SUBLIMATION_CHECK: (f64, f64) = (230.0, 8.947_35);
    /// Definition: relative tolerance against the eight printed digits of
    /// the slope check.
    const EIGHT_DIGITS: f64 = 1e-7;
    /// Definition: relative tolerance at which the two curves must meet at
    /// the triple point and the critical pressure be reached.
    const FOUR_DIGITS: f64 = 1e-4;
    /// Definition: temperatures past the critical point, K: just past it,
    /// the hottest legacy steam case (1500 K) and twice that.
    const PAST_THE_CRITICAL_POINT: [f64; 3] = [700.0, 1500.0, 3000.0];
    /// Definition: a cold surface, 1 K, far below the 50 K floor of the
    /// sublimation curve.
    const COLD: f64 = 1.0;

    /// Source: NIST Chemistry WebBook (SRD 69, Lemmon, McLinden and
    /// Friend), saturation table of water at 300 K computed from IAPWS-95
    /// (Wagner and Pruss 2002): specific enthalpies of the saturated liquid
    /// 112.564 86 kJ/kg and vapour 2549.854 1 kJ/kg, here (T in K,
    /// h' in J/kg, h'' in J/kg). Fetched 2026-11-22 from
    /// <https://webbook.nist.gov/cgi/fluid.cgi?ID=C7732185&Action=Data&Type=SatP&TLow=300&THigh=300&TInc=1&TUnit=K&HUnit=kJ%2Fkg>.
    const VAPORISATION_300_K: (f64, f64, f64) = (300.0, 112.564_86e3, 2_549.854_1e3);
    /// Source: IAPWS R6-95(2018), Table 8: saturated liquid and vapour at
    /// 275 K, h' = 0.775 972 202e1 kJ/kg, h'' = 0.250 428 995e4 kJ/kg, here
    /// (T in K, h' in J/kg, h'' in J/kg).
    const VAPORISATION_275_K: (f64, f64, f64) = (275.0, 0.775_972_202e4, 0.250_428_995e7);
    /// Source: specific enthalpy of the saturated vapour at the triple
    /// point, 2500.5e3 J/kg (IAPWS SR1-86(1992), Table 1), on the IAPWS-95
    /// reference state (internal energy and entropy of the liquid zero at
    /// the triple point).
    const VAPOUR_ENTHALPY_AT_TRIPLE_POINT: f64 = 2500.5e3;
    /// Source: specific enthalpy of ice Ih at the triple point,
    /// -0.333 444 254 079e6 J/kg, with the IAPWS-95 value of `s0` (IAPWS
    /// R10-06(2009), Table 6), on the same reference state as
    /// [`VAPOUR_ENTHALPY_AT_TRIPLE_POINT`].
    const ICE_ENTHALPY_AT_TRIPLE_POINT: f64 = -0.333_444_254_079e6;
    /// Source: latent heat of sublimation of ice, J/mol, fitted to the
    /// IAPWS-95 vapour and the Feistel and Wagner ice Ih Gibbs function:
    /// `L = c0 + c1 T + c2 T^2 + c3 exp(-(T / c4)^2)`, T > 30 K, with
    /// `[c0, c1, c2, c3, c4]` (Murphy and Koop 2005, Q. J. R. Meteorol. Soc.
    /// 131, 1539, doi:10.1256/qj.04.94, eq. 5). Wagner et al. 2011 tabulate
    /// no sublimation enthalpy.
    const ICE_LATENT_FIT: [f64; 5] = [46_782.5, 35.892_5, -0.074_14, 541.5, 123.75];
    /// Definition: temperature of the sublimation check, 250 K.
    const ICE_LATENT_CHECK_TEMPERATURE: f64 = 250.0;
    /// Definition: the stop tolerance of the plan on the latent heat taken
    /// from the curve's slope, 2 %.
    const LATENT_TOLERANCE: f64 = 0.02;

    fn close(a: f64, b: f64, tolerance: f64) -> bool {
        (a / b - 1.0).abs() < tolerance
    }

    #[test]
    fn liquid_pressure_matches_the_iapws_95_checks() {
        let delta = IAPWS95_CHECK_DENSITY / CRITICAL_DENSITY;
        let tau = CRITICAL_TEMPERATURE / IAPWS95_CHECK_TEMPERATURE;
        let residual = residual_delta(delta, tau);
        assert!(
            close(residual, IAPWS95_RESIDUAL_DELTA, NINE_DIGITS),
            "{residual}"
        );
        for (t, rho, p) in IAPWS95_CHECKS {
            let got = liquid_pressure(Kelvin(t), KilogramPerCubicMetre(rho));
            assert!(close(got.0, p, NINE_DIGITS), "{t} K {rho}: {got:?}");
        }
    }

    #[test]
    fn ice_ih_density_matches_the_iapws_checks() {
        for (t, p, rho) in ICE_IH_CHECKS {
            let got = ice_ih_density(Kelvin(t), Pascal(p));
            assert!(close(got.0, rho, TWELVE_DIGITS), "{t} K {p} Pa: {got:?}");
        }
    }

    #[test]
    fn melting_pressures_match_the_iapws_checks() {
        let (t, p) = IH_MELTING_CHECK;
        let ih = ice_ih_melting_pressure(Kelvin(t));
        assert!(close(ih.0, p, SIX_DIGITS), "{ih:?}");
        let (t, p) = VI_MELTING_CHECK;
        let vi = ice_vi_melting_pressure(Kelvin(t));
        assert!(close(vi.0, p, SIX_DIGITS), "{vi:?}");
    }

    #[test]
    fn the_melting_curves_meet_their_triple_points() {
        let ih = ice_ih_melting_pressure(IH_III_LIQUID_TEMPERATURE);
        assert!(close(ih.0, IH_III_LIQUID_PRESSURE.0, SIX_DIGITS), "{ih:?}");
        let vi = ice_vi_melting_pressure(VI_VII_LIQUID_TEMPERATURE);
        assert!(close(vi.0, VI_VII_LIQUID_PRESSURE.0, SIX_DIGITS), "{vi:?}");
    }

    /// Source: melting pressure of ice III, 268.685 MPa at 254 K (IAPWS
    /// R14-08(2011), Table 3).
    const III_MELTING_CHECK: (f64, f64) = (254.0, 268.685e6);
    /// Source: melting pressure of ice V, 479.640 MPa at 265 K (IAPWS
    /// R14-08(2011), Table 3).
    const V_MELTING_CHECK: (f64, f64) = (265.0, 479.640e6);
    /// Definition: relative tolerance on the triple points the curves
    /// join (IAPWS rounds them to four digits).
    const TRIPLE_POINT_TOLERANCE: f64 = 1e-4;
    /// Definition: relative tolerance of a round trip through a curve.
    const ROUND_TRIP: f64 = 1e-6;
    /// Source: melting temperature of the stable ice at 300 MPa, 254.96 K
    /// (ice III, IAPWS R14-08(2011), eq. 2 inverted by hand).
    const MELTING_AT_300_MPA: (f64, f64) = (300e6, 254.96);
    /// Definition: tolerance on that temperature, 0.01 K.
    const MELTING_TOLERANCE: f64 = 0.01;
    /// Definition: a pressure in the field of ice V, 500 MPa.
    const ICE_V_PRESSURE: f64 = 500e6;
    /// Definition: a pressure in the field of ice VI, 1 GPa.
    const ICE_VI_PRESSURE: f64 = 1e9;
    /// Definition: a pressure over the ice VI - ice VII triple point,
    /// 3 GPa.
    const ICE_VII_PRESSURE: f64 = 3e9;
    /// Definition: the continuity tolerance at a triple point, 1e-4 K.
    /// The published curves are consistent to this: ice V and VI differ
    /// by 3.2e-5 K at their triple point, ice VI and VII by 7.7e-5 K.
    const CONTINUITY: f64 = 1e-4;
    /// Definition: the sweep of the minimum, 0.1 to 2000 MPa in 0.1 MPa
    /// steps.
    const SWEEP: (f64, f64, f64) = (0.1e6, 2000e6, 0.1e6);

    #[test]
    fn ices_iii_and_v_melt_on_the_iapws_curves() {
        let (t, p) = III_MELTING_CHECK;
        let iii = ice_iii_melting_pressure(Kelvin(t));
        assert!(close(iii.0, p, SIX_DIGITS_TABLE), "{iii:?}");
        let (t, p) = V_MELTING_CHECK;
        let v = ice_v_melting_pressure(Kelvin(t));
        assert!(close(v.0, p, SIX_DIGITS_TABLE), "{v:?}");
        let ends = [
            (
                ice_iii_melting_pressure(IH_III_LIQUID_TEMPERATURE),
                IH_III_LIQUID_PRESSURE,
            ),
            (
                ice_iii_melting_pressure(III_V_LIQUID_TEMPERATURE),
                III_V_LIQUID_PRESSURE,
            ),
            (
                ice_v_melting_pressure(III_V_LIQUID_TEMPERATURE),
                III_V_LIQUID_PRESSURE,
            ),
            (
                ice_v_melting_pressure(V_VI_LIQUID_TEMPERATURE),
                V_VI_LIQUID_PRESSURE,
            ),
        ];
        for (got, want) in ends {
            assert!(
                close(got.0, want.0, TRIPLE_POINT_TOLERANCE),
                "{got:?} {want:?}"
            );
        }
    }

    /// Definition: relative tolerance on a Table 3 check value, 1e-5.
    const SIX_DIGITS_TABLE: f64 = 1e-5;

    /// The pressure of the curve of the ice stable at `p`, at `t`.
    fn own_curve(p: f64, t: Kelvin) -> Pascal {
        if p <= IH_III_LIQUID_PRESSURE.0 {
            ice_ih_melting_pressure(t)
        } else if p <= III_V_LIQUID_PRESSURE.0 {
            ice_iii_melting_pressure(t)
        } else if p <= V_VI_LIQUID_PRESSURE.0 {
            ice_v_melting_pressure(t)
        } else {
            ice_vi_melting_pressure(t)
        }
    }

    #[test]
    fn melting_follows_the_stable_ice() {
        assert_eq!(
            melting_temperature(STANDARD_ATMOSPHERE),
            ice_ih_melting_temperature(STANDARD_ATMOSPHERE)
        );
        let (p, t) = MELTING_AT_300_MPA;
        let got = melting_temperature(Pascal(p));
        assert!((got.0 - t).abs() < MELTING_TOLERANCE, "{got:?}");
        let v = melting_temperature(Pascal(ICE_V_PRESSURE));
        assert!(
            v.0 > III_V_LIQUID_TEMPERATURE.0 && v.0 < V_VI_LIQUID_TEMPERATURE.0,
            "{v:?}"
        );
        let vi = melting_temperature(Pascal(ICE_VI_PRESSURE));
        assert!(
            vi.0 > V_VI_LIQUID_TEMPERATURE.0 && vi.0 < VI_VII_LIQUID_TEMPERATURE.0,
            "{vi:?}"
        );
        for p in [300e6, ICE_V_PRESSURE, ICE_VI_PRESSURE] {
            let t = melting_temperature(Pascal(p));
            let back = own_curve(p, t).0;
            assert!(close(back, p, ROUND_TRIP), "{p} Pa -> {back}");
        }
        // Continuous at the three triple points, to the published curves'
        // own consistency (see CONTINUITY).
        for p in [
            IH_III_LIQUID_PRESSURE,
            III_V_LIQUID_PRESSURE,
            V_VI_LIQUID_PRESSURE,
        ] {
            let below = melting_temperature(Pascal(p.0 * (1.0 - 1e-9)));
            let above = melting_temperature(Pascal(p.0 * (1.0 + 1e-9)));
            assert!(
                (below.0 - above.0).abs() < CONTINUITY,
                "{p:?}: {below:?} {above:?}"
            );
        }
        // The grid of the sweep does not hold 208.566 MPa: add it.
        let (mut p, mut min, mut at) = (SWEEP.0, f64::MAX, 0.0);
        let mut pressures = vec![IH_III_LIQUID_PRESSURE.0];
        while p <= SWEEP.1 {
            pressures.push(p);
            p += SWEEP.2;
        }
        for p in pressures {
            let t = melting_temperature(Pascal(p)).0;
            if t < min {
                (min, at) = (t, p);
            }
        }
        assert!(
            (min - IH_III_LIQUID_TEMPERATURE.0).abs() < CONTINUITY,
            "{min}"
        );
        assert_eq!(at, IH_III_LIQUID_PRESSURE.0);
        // Ice VII is not ported: the melting temperature is held.
        assert_eq!(
            melting_temperature(Pascal(ICE_VII_PRESSURE)),
            VI_VII_LIQUID_TEMPERATURE
        );
    }

    #[test]
    fn ice_ih_melts_at_its_normal_melting_point_at_one_atmosphere() {
        let t = ice_ih_melting_temperature(STANDARD_ATMOSPHERE);
        assert!(close(t.0, NORMAL_MELTING_TEMPERATURE, SIX_DIGITS), "{t:?}");
        assert_eq!(
            ice_ih_melting_temperature(Pascal(0.0)),
            TRIPLE_POINT_TEMPERATURE
        );
    }

    #[test]
    fn the_vi_vii_boundary_meets_its_triple_point() {
        let t = ice_vi_vii_temperature(VI_VII_LIQUID_PRESSURE);
        assert!(
            (t.0 - VI_VII_LIQUID_TEMPERATURE.0).abs() < BOUNDARY_TOLERANCE,
            "{t:?}"
        );
        let p = ice_vi_vii_pressure(t);
        assert!(close(p.0, VI_VII_LIQUID_PRESSURE.0, SIX_DIGITS), "{p:?}");
    }

    #[test]
    fn saturation_matches_iapws() {
        for (t, p) in SATURATION_CHECKS {
            let got = saturation_pressure(Kelvin(t));
            assert!(close(got.0, p, SIX_DIGITS), "{t} K: {got:?}");
        }
        let triple = saturation_pressure(TRIPLE_POINT_TEMPERATURE);
        assert!(
            close(triple.0, TRIPLE_POINT_PRESSURE.0, FOUR_DIGITS),
            "{triple:?}"
        );
        let critical = saturation_pressure(CRITICAL_POINT_TEMPERATURE);
        assert!(
            close(critical.0, CRITICAL_PRESSURE.0, FOUR_DIGITS),
            "{critical:?}"
        );
    }

    #[test]
    fn sublimation_matches_iapws() {
        let (t, p) = SUBLIMATION_CHECK;
        let got = sublimation_pressure(Kelvin(t));
        assert!(close(got.0, p, SIX_DIGITS), "{got:?}");
        let triple = sublimation_pressure(TRIPLE_POINT_TEMPERATURE);
        assert!(
            close(triple.0, TRIPLE_POINT_PRESSURE.0, FOUR_DIGITS),
            "{triple:?}"
        );
    }

    #[test]
    fn vapour_saturation_is_continuous() {
        let ice = sublimation_pressure(TRIPLE_POINT_TEMPERATURE);
        let liquid = saturation_pressure(TRIPLE_POINT_TEMPERATURE);
        assert!(close(ice.0, liquid.0, FOUR_DIGITS), "{ice:?} {liquid:?}");
        let joined = vapour_saturation_pressure(TRIPLE_POINT_TEMPERATURE);
        assert!(close(joined.0, liquid.0, FOUR_DIGITS), "{joined:?}");
        let critical = vapour_saturation_pressure(CRITICAL_POINT_TEMPERATURE);
        assert!(
            close(critical.0, CRITICAL_PRESSURE.0, FOUR_DIGITS),
            "{critical:?}"
        );
        for t in PAST_THE_CRITICAL_POINT {
            let p = vapour_saturation_pressure(Kelvin(t));
            assert!(p.0.is_finite(), "{t} K: {p:?}");
            assert_eq!(p, CRITICAL_PRESSURE, "{t} K");
            assert!(vapour_latent_j_mol(t).is_finite(), "{t} K");
        }
        for t in [COLD, 0.0, -COLD, f64::NAN] {
            let p = vapour_saturation_pressure(Kelvin(t));
            assert!(p.0.is_finite() && p.0 >= 0.0, "{t} K: {p:?}");
            assert!(vapour_latent_j_mol(t).is_finite(), "{t} K");
        }
    }

    #[test]
    fn latent_heat_is_the_curves_slope() {
        // The slope at the triple point, p L / (R T^2), against IAPWS.
        let t = TRIPLE_POINT_TEMPERATURE.0;
        let slope = saturation_pressure(TRIPLE_POINT_TEMPERATURE).0 * vapour_latent_j_mol(t)
            / (MOLAR_GAS_CONSTANT * t * t);
        assert!(
            close(slope, SATURATION_SLOPE_AT_TRIPLE_POINT, EIGHT_DIGITS),
            "{slope}"
        );
        // Definition: the molar mass of IAPWS-95, R_m / R, kg/mol.
        let molar_mass = MOLAR_GAS_CONSTANT / SPECIFIC_GAS_CONSTANT;
        for (t, liquid, vapour) in [VAPORISATION_300_K, VAPORISATION_275_K] {
            let latent = vapour_latent_j_mol(t);
            let table = (vapour - liquid) * molar_mass;
            assert!(
                close(latent, table, LATENT_TOLERANCE),
                "{t} K: {latent} against {table}"
            );
        }
        let [c0, c1, c2, c3, c4] = ICE_LATENT_FIT;
        let t = ICE_LATENT_CHECK_TEMPERATURE;
        let fit = c0 + c1 * t + c2 * t * t + c3 * (-(t / c4).powi(2)).exp();
        let latent = vapour_latent_j_mol(t);
        assert!(
            close(latent, fit, LATENT_TOLERANCE),
            "{latent} against {fit}"
        );
        let below = vapour_latent_j_mol(TRIPLE_POINT_TEMPERATURE.0.next_down());
        let table = (VAPOUR_ENTHALPY_AT_TRIPLE_POINT - ICE_ENTHALPY_AT_TRIPLE_POINT) * molar_mass;
        assert!(
            close(below, table, LATENT_TOLERANCE),
            "{below} against {table}"
        );
    }
}
