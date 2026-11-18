//! A radiative–convective grey column: the outgoing longwave radiation
//! of an atmosphere whose temperature follows its adiabats (issue #441).
//!
//! From the surface up, the air follows the dry adiabat of its mixture
//! until its water vapour saturates, then the moist pseudo-adiabat
//! (Pierrehumbert 2010, *Principles of Planetary Climate*, §2.7, his
//! eq. 2.33); past water's critical point the vapour is a gas like the
//! others. A skin temperature floors the column (a stratosphere, where
//! the vapour keeps the mixing ratio of its cold trap). The grey optical
//! depth is integrated down from the top with the per-gas laws of
//! [`super::climate`], and the outgoing radiation is the grey two-stream
//! integral of `σ T⁴` over it.
//!
//! On a moist column the levels that radiate to space sit on water's
//! saturation curve whatever the surface temperature, so the outgoing
//! radiation flattens at the Simpson–Nakajima limit (Nakajima, Hayashi &
//! Abe 1992, doi:10.1175/1520-0469(1992)049<2256:ASOTGE>2.0.CO;2) until
//! the dry adiabat from a hot surface reaches them.
use super::Gases;
use super::climate::{
    STEAM_MAX_BAR, WATER_CRITICAL_K, WATER_L_OVER_R, foreign_share, opacity_per_ln_p,
    optical_depth_at, water_saturation_bar,
};

/// Stefan–Boltzmann constant, W/(m²·K⁴) (CODATA 2018, exact).
const SIGMA: f64 = 5.670_374_419e-8;

/// Molar gas constant, J/(mol·K) (CODATA 2018, exact).
const R_GAS: f64 = 8.314_462_618;

/// Molar heat capacities at constant pressure, J/(mol·K), of the gases
/// that do not condense in the column, at 298.15 K (NIST Chemistry
/// WebBook, gas-phase thermochemistry, Chase 1998 JANAF tables; from
/// memory, not checked): N₂ 29.12, CO₂ 37.13, CH₄ 35.69, NH₃ 35.06,
/// SO₂ 39.87, O₂ 29.38. Held constant: only the steam's, which sets where
/// a hot steam column leaves the radiation limit, follows the temperature
/// (see [`steam_cp`]).
const N2_CP: f64 = 29.12;
const CO2_CP: f64 = 37.13;
const CH4_CP: f64 = 35.69;
const NH3_CP: f64 = 35.06;
const SO2_CP: f64 = 39.87;
const O2_CP: f64 = 29.38;

/// Molar heat capacity of water vapour, J/(mol·K), at `temperature_k`:
/// the Shomate equation with the NIST Chemistry WebBook's coefficients
/// for water gas (Chase 1998; checked on the WebBook), 500 to 1700 K
/// (and below, where it still gives the 33.6 J/(mol·K) of 300 K) and
/// 1700 to 6000 K. It grows from 34 to 52 J/(mol·K) between 300 and
/// 2000 K, which flattens the dry adiabat of hot steam.
fn steam_cp(temperature_k: f64) -> f64 {
    let t = temperature_k.clamp(200.0, 6000.0) / 1000.0;
    let (a, b, c, d, e) = if temperature_k < 1700.0 {
        (30.092, 6.832_514, 6.793_435, -2.534_48, 0.082_139)
    } else {
        (41.964_26, 8.622_053, -1.499_78, 0.098_119, -11.157_64)
    };
    a + t * (b + t * (c + t * d)) + e / (t * t)
}

/// Molar latent heat of vaporisation of water, J/mol: the `L / R_v` of
/// [`super::climate`] times the molar gas constant, so that the adiabat
/// and the saturation curve share one latent heat.
const WATER_L_MOLAR: f64 = WATER_L_OVER_R * R_GAS;

/// Optical depth from the top at which the column radiates to space in
/// the grey two-stream approximation: the diffusivity factor `D` = 3/2
/// of the Eddington closure (Pierrehumbert 2010, §4.3; from memory, not
/// checked), with which the
/// transmission `exp(−D τ)` falls to `1/e` at `τ` = 2/3, the Eddington
/// emission level.
const DIFFUSIVITY: f64 = 1.5;

/// Levels of the pressure grid per decade of pressure. Calibration: the
/// moist plateau moves by under 0.5 W/m² and Earth's thermostat CO₂ by
/// under 5 % against a grid two and a half times as fine; twelve levels
/// put Earth's outgoing radiation 3.6 W/m² off with the trapezoidal rule.
const LEVELS_PER_DECADE: f64 = 20.0;

/// Most levels in a column: sixteen decades of pressure (the steam is
/// capped at [`STEAM_MAX_BAR`], so a column spans about seven).
const MAX_LEVELS: usize = 320;

/// Top of the column: `TOP_SHARE` of the surface pressure, and never
/// above `TOP_BAR`, so that the emitting levels of water (near 1e-3 bar)
/// stay inside a thick column. Above the top the air is taken uniform at
/// the top's temperature (its optical depth is the per-gas laws' column
/// total at the top's pressures), exact in a stratosphere at the skin
/// temperature.
const TOP_SHARE: f64 = 1e-3;
const TOP_BAR: f64 = 1e-4;

/// Pressure, bar, of the tropopause of a thick atmosphere, where its
/// air is coldest: about 0.1 bar on every thick atmosphere of the Solar
/// System (Earth, Titan, Jupiter, Saturn, Uranus, Neptune), which
/// Robinson & Catling 2014 (*Nature Geosci.* 7, 12, doi:10.1038/ngeo2020;
/// abstract checked) explain by the infrared opacity of the air above it
/// and the shortwave opacity of the air below. In a thinner air the
/// whole column is above it.
const TROPOPAUSE_BAR: f64 = 0.1;

/// The methane, bar at the surface, that radiates in a column of `gases`
/// under a stratosphere at `skin_k`: methane is cold-trapped where the
/// air is coldest, at the tropopause ([`TROPOPAUSE_BAR`]) at about the
/// skin temperature, so that its mixing ratio above it is at most the
/// saturation pressure of pure methane there over the tropopause's
/// pressure ([`super::atmosphere::methane_saturation_bar`]); the methane
/// that radiates to space at the column's emitting levels is that of
/// the air above. Without a stratosphere (`skin_k` 0) all of it.
fn radiating_methane_bar(gases: &Gases, pressure_bar: f64, skin_k: f64) -> f64 {
    let ch4 = gases.ch4.max(0.0);
    if skin_k <= 0.0 || ch4 <= 0.0 {
        return ch4;
    }
    let tropopause = TROPOPAUSE_BAR.min(pressure_bar);
    let share = super::atmosphere::methane_saturation_bar(skin_k) / tropopause;
    ch4.min(share * pressure_bar)
}

/// The methane, bar, above the tropopause of a uniform air of `gases`
/// under a stratosphere at `skin_k`: its column there, the mixing ratio
/// the cold trap lets through ([`radiating_methane_bar`]) times the
/// tropopause's pressure ([`TROPOPAUSE_BAR`]; the whole of a thinner
/// air). The methane whose near-infrared bands take a share of the
/// starlight before it reaches the troposphere
/// ([`super::climate`]'s anti-greenhouse).
pub(super) fn stratospheric_methane_bar(gases: &Gases, skin_k: f64) -> f64 {
    let pressure = gases.total();
    if pressure <= 0.0 {
        return 0.0;
    }
    radiating_methane_bar(gases, pressure, skin_k) * TROPOPAUSE_BAR.min(pressure) / pressure
}

/// The gases of a column without their water, as mole fractions and the
/// molar heat capacity of the mixture.
struct DryAir {
    /// Partial pressures, bar, at the surface (no water).
    gases: Gases,
    /// The methane among them that radiates, bar at the surface
    /// ([`radiating_methane_bar`]).
    radiating_ch4: f64,
    /// Their sum, bar.
    pressure: f64,
    /// Molar heat capacity of the mixture, J/(mol·K).
    cp: f64,
}

impl DryAir {
    fn of(gases: &Gases, skin_k: f64) -> Self {
        let g = Gases {
            n2: gases.n2.max(0.0),
            co2: gases.co2.max(0.0),
            h2o: 0.0,
            ch4: gases.ch4.max(0.0),
            nh3: gases.nh3.max(0.0),
            so2: gases.so2.max(0.0),
            o2: gases.o2.max(0.0),
        };
        let pressure = g.total();
        let cp = if pressure > 0.0 {
            (g.n2 * N2_CP
                + g.co2 * CO2_CP
                + g.ch4 * CH4_CP
                + g.nh3 * NH3_CP
                + g.so2 * SO2_CP
                + g.o2 * O2_CP)
                / pressure
        } else {
            N2_CP
        };
        Self {
            radiating_ch4: radiating_methane_bar(&g, pressure, skin_k),
            gases: g,
            pressure,
            cp,
        }
    }

    /// The gases that radiate at a level where the dry air has `dry_bar`
    /// and the vapour `vapour_bar`: the methane cold-trapped at the
    /// tropopause (see [`radiating_methane_bar`]) is left out.
    fn at(&self, dry_bar: f64, vapour_bar: f64) -> Gases {
        let share = if self.pressure > 0.0 {
            dry_bar / self.pressure
        } else {
            0.0
        };
        Gases {
            n2: self.gases.n2 * share,
            co2: self.gases.co2 * share,
            h2o: vapour_bar,
            ch4: self.radiating_ch4 * share,
            nh3: self.gases.nh3 * share,
            so2: self.gases.so2 * share,
            o2: self.gases.o2 * share,
        }
    }
}

/// `d T / d ln p`, K, at `temperature_k` and `pressure_bar` on the moist
/// pseudo-adiabat (`saturated`) or on the dry adiabat of air holding
/// `vapour_share` of vapour (mole fraction).
///
/// Moist: Pierrehumbert 2010, eq. 2.33, `d ln T / d ln p_a =
/// (R_a/c_pa)(1 + L r / (R_a T)) / (1 + (c_pv/c_pa + (L/(R_v T) − 1)
/// L/(c_pa T)) r)` with `r` the vapour's mass mixing ratio, written per
/// mole and multiplied through by `p_a` so that it holds down to pure
/// steam; then `d ln p = d ln p_a (p_a + e_s L A/(R_v T)) / p` with `A`
/// that slope, since `p = p_a + e_s(T)`. Pure steam follows the
/// Clausius–Clapeyron curve `d T/d ln p = R_v T²/L`.
fn lapse(
    air: &DryAir,
    temperature_k: f64,
    pressure_bar: f64,
    saturated: bool,
    vapour_share: f64,
) -> f64 {
    let t = temperature_k;
    if !saturated && vapour_share <= 0.0 {
        return t * R_GAS / air.cp;
    }
    let cp_v = steam_cp(t);
    if saturated {
        let e = water_saturation_bar(t).min(pressure_bar);
        let dry = (pressure_bar - e).max(0.0);
        let l_t = WATER_L_MOLAR / t;
        let a = (R_GAS * dry + l_t * e)
            / (air.cp * dry + (cp_v + (WATER_L_OVER_R / t - 1.0) * l_t) * e);
        a * t * pressure_bar / (dry + a * e * WATER_L_OVER_R / t)
    } else {
        let cp = (1.0 - vapour_share) * air.cp + vapour_share * cp_v;
        t * R_GAS / cp
    }
}

/// Outgoing longwave radiation, W/m², of a column with surface partial
/// pressures `gases` (bar; `gases.h2o` the vapour at the surface) over a
/// surface at `surface_k`, with no stratosphere. See the module
/// documentation and [`olr_under_skin_w_m2`].
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn olr_w_m2(gases: &Gases, surface_k: f64) -> f64 {
    olr_under_skin_w_m2(gases, surface_k, 0.0)
}

/// Outgoing longwave radiation, W/m², of the column of [`olr_w_m2`] whose
/// temperature never falls below `skin_k` (its stratosphere).
///
/// The column runs from the surface, where the vapour is `gases.h2o`
/// capped at saturation below water's critical point (and at
/// [`STEAM_MAX_BAR`] in any column, wet or runaway: the non-grey window
/// it stands for exists in every thick steam column), up to [`TOP_SHARE`] of the surface pressure
/// (never above [`TOP_BAR`]) on a log grid. Without air the surface
/// radiates as a black body.
pub(crate) fn olr_under_skin_w_m2(gases: &Gases, surface_k: f64, skin_k: f64) -> f64 {
    let surface_k = surface_k.max(1.0);
    let black = SIGMA * surface_k.powi(4);
    let air = DryAir::of(gases, skin_k);
    let steam = if gases.h2o > 0.0 {
        gases.h2o.min(STEAM_MAX_BAR)
    } else {
        0.0
    };
    let supercritical = surface_k >= WATER_CRITICAL_K;
    let vapour = if supercritical {
        steam
    } else {
        steam.min(water_saturation_bar(surface_k))
    };
    let surface_bar = air.pressure + vapour;
    if surface_bar <= 0.0 {
        return black;
    }
    // The other gases broaden lines in proportion to their pressure, by
    // the share their whole column has (see `foreign_share`).
    let share = foreign_share(&air.at(air.pressure, steam));
    let top_bar = (surface_bar * TOP_SHARE).min(TOP_BAR);
    let span = (surface_bar / top_bar).ln();
    // Fixed steps in ln p from the surface, the last one shorter, so that
    // the outgoing radiation does not jump as the surface pressure moves.
    let step = (std::f64::consts::LN_10 / LEVELS_PER_DECADE).max(span / MAX_LEVELS as f64);
    let n = ((span / step).ceil() as usize).clamp(1, MAX_LEVELS);
    let last = span - (n - 1) as f64 * step;
    let step_of = |i: usize| if i + 1 == n { last } else { step };

    // Temperature and opacity per ln p of each level, from the surface up.
    let mut temperature = [0.0; MAX_LEVELS + 1];
    let mut opacity = [0.0; MAX_LEVELS + 1];
    let mut saturated = !supercritical && steam >= water_saturation_bar(surface_k);
    let mut frozen = false;
    let mut vapour_share = vapour / surface_bar;
    let mut t = surface_k;
    let mut p = surface_bar;
    let (ratio, half_ratio) = ((-step).exp(), (-0.5 * step).exp());
    let (last_ratio, last_half) = ((-last).exp(), (-0.5 * last).exp());
    let mut top = air.at(air.pressure, vapour);
    for i in 0..=n {
        let v = vapour_share * p;
        let level = air.at(p - v, v);
        temperature[i] = t;
        opacity[i] = opacity_per_ln_p(&level, share);
        if i == n {
            top = level;
            break;
        }
        let (h, r, half_r) = if i + 1 == n {
            (last, last_ratio, last_half)
        } else {
            (step, ratio, half_ratio)
        };
        let p_here = p;
        let p_mid = p * half_r;
        p *= r;
        if frozen {
            continue;
        }
        // Midpoint (second-order Runge–Kutta) step up the adiabat.
        let half = t - 0.5 * h * lapse(&air, t, p_here, saturated, vapour_share);
        t -= h * lapse(&air, half, p_mid, saturated, vapour_share);
        if t <= skin_k {
            t = skin_k;
            frozen = true;
        }
        if saturated || t < WATER_CRITICAL_K {
            let e = water_saturation_bar(t);
            if saturated || vapour_share * p >= e {
                saturated = true;
                vapour_share = e.min(p) / p;
            }
        }
    }

    // Grey two-stream radiation to space, summed from the top down.
    let mut tau = optical_depth_at(&top, share);
    let mut seen = (-DIFFUSIVITY * tau).exp();
    let mut out = SIGMA * temperature[n].powi(4) * (1.0 - seen);
    for i in (0..n).rev() {
        let depth = log_mean(opacity[i], opacity[i + 1]) * step_of(i);
        let source = 0.5 * SIGMA * (temperature[i].powi(4) + temperature[i + 1].powi(4));
        let below = seen * (-DIFFUSIVITY * depth).exp();
        out += source * (seen - below);
        seen = below;
        tau += depth;
        if DIFFUSIVITY * tau > 40.0 {
            return out;
        }
    }
    out + black * seen
}

/// Logarithmic mean of two opacities per `ln p`: the exact mean over a
/// layer where the opacity grows exponentially in `ln p` (a power of the
/// pressure, as through a uniform air), and the arithmetic mean's limit
/// when they are equal.
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
