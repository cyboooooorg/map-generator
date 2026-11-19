//! A radiative–convective grey column: the outgoing longwave radiation
//! of an atmosphere whose temperature follows its adiabats (issue #441).
//!
//! From the surface up, the air follows the dry adiabat of its mixture
//! until its water vapour saturates, then the moist pseudo-adiabat
//! (Pierrehumbert 2010, *Principles of Planetary Climate*, §2.7, his
//! eq. 2.33); past water's critical point the vapour is a gas like the
//! others. Where the CO₂ of a cold column reaches its own saturation
//! (its frost below its triple point, its liquid above), it condenses
//! and the air follows CO₂'s pseudo-adiabat, its CO₂ thinning to what
//! saturation leaves (Kasting 1991, doi:10.1016/0019-1035(91)90137-I,
//! Appendix B, which Kopparapu et al. 2013 follow; from memory, not
//! checked), so that a thick CO₂ air's greenhouse flattens: with its
//! Rayleigh scattering, the maximum greenhouse at the outer edge of the
//! habitable zone (issue #522). A skin temperature floors the column (a
//! stratosphere, where the vapour and the CO₂ keep the mixing ratio of
//! their cold trap). The grey optical
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
use super::atmosphere::{co2_latent_j_mol, co2_saturation_bar};
use super::climate::{
    Planet, STEAM_MAX_BAR, WATER_CRITICAL_K, WATER_L_OVER_R, water_saturation_bar,
};
use super::spectrum::{
    self, BANDS, DIFFUSIVITY, G, Spectrum, absorption_spectrum, depth_above, planck_spectrum,
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

/// Transmission under which a band of the column is opaque: below it,
/// `exp(−40)`, the levels and the surface it hides add nothing to the
/// outgoing radiation.
const OPAQUE: f64 = 4.248_354_255_291_589e-18;

/// Levels of the pressure grid per decade of pressure. Calibration: the
/// moist plateau moves by under 0.5 W/m² and Earth's thermostat CO₂ by
/// under 5 % against a grid two and a half times as fine; twelve levels
/// put Earth's outgoing radiation 3.6 W/m² off with the trapezoidal rule.
const LEVELS_PER_DECADE: f64 = 20.0;

/// Levels per decade of the column that measures the moist plateau
/// ([`plateau_olr_under_skin_w_m2`]): against twice as many, the plateau
/// moves by under 0.5 W/m² (the test
/// `the_plateaus_grid_is_converged`), where [`LEVELS_PER_DECADE`] leaves
/// it about 2 W/m² high.
pub(crate) const PLATEAU_LEVELS_PER_DECADE: f64 = 80.0;

/// Most levels in a column: eight decades of pressure at twice
/// [`PLATEAU_LEVELS_PER_DECADE`] (the steam is capped at
/// [`STEAM_MAX_BAR`], so a column spans about seven).
const MAX_LEVELS: usize = 1280;

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

    /// The gases that radiate at a level where the dry air has `dry_bar`,
    /// the vapour `vapour_bar` and the CO₂ `co2_bar`, what its
    /// condensation leaves there ([`olr_under_skin_w_m2`]): the other
    /// gases share the rest of the dry air in their surface proportions,
    /// and the methane cold-trapped at the tropopause (see
    /// [`radiating_methane_bar`]) is left out.
    fn with_co2(&self, dry_bar: f64, vapour_bar: f64, co2_bar: f64) -> Gases {
        let others = self.pressure - self.gases.co2;
        let share = if others > 0.0 {
            (dry_bar - co2_bar).max(0.0) / others
        } else {
            0.0
        };
        Gases {
            n2: self.gases.n2 * share,
            co2: co2_bar,
            h2o: vapour_bar,
            ch4: self.radiating_ch4 * share,
            nh3: self.gases.nh3 * share,
            so2: self.gases.so2 * share,
            o2: self.gases.o2 * share,
        }
    }

    /// Molar heat capacity, J/(mol·K), of the gases other than CO₂: the
    /// air CO₂ condenses out of ([`co2_lapse`]); nitrogen's when there is
    /// none.
    fn cp_without_co2(&self) -> f64 {
        let g = &self.gases;
        let others = g.n2 + g.ch4 + g.nh3 + g.so2 + g.o2;
        if others > 0.0 {
            (g.n2 * N2_CP + g.ch4 * CH4_CP + g.nh3 * NH3_CP + g.so2 * SO2_CP + g.o2 * O2_CP)
                / others
        } else {
            N2_CP
        }
    }
}

/// `d T / d ln p`, K, at `temperature_k` and `pressure_bar` on the moist
/// pseudo-adiabat of CO₂ condensing out of `air` (Kasting 1991,
/// doi:10.1016/0019-1035(91)90137-I, from memory, not checked; its
/// Appendix B, which Kopparapu et al. 2013 follow in the upper
/// troposphere, checked on their arXiv:1301.6674v2; written here as the general
/// pseudo-adiabat of Pierrehumbert 2010, eq. 2.33, used for water in
/// [`lapse`], with CO₂ the condensable, [`co2_latent_j_mol`] its latent
/// heat, and the other gases and the little water vapour so cold an air
/// holds the non-condensing part). Pure CO₂ follows its saturation curve,
/// `d T/d ln p = R T²/L`.
fn co2_lapse(air: &DryAir, temperature_k: f64, pressure_bar: f64) -> f64 {
    let t = temperature_k;
    let e = co2_saturation_bar(t).min(pressure_bar);
    let dry = (pressure_bar - e).max(0.0);
    let latent = co2_latent_j_mol(t);
    let l_over_r = latent / R_GAS;
    let l_t = latent / t;
    let a = (R_GAS * dry + l_t * e)
        / (air.cp_without_co2() * dry + (CO2_CP + (l_over_r / t - 1.0) * l_t) * e);
    a * t * pressure_bar / (dry + a * e * l_over_r / t)
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
pub(crate) fn olr_w_m2(gases: &Gases, surface_k: f64, planet: &Planet) -> f64 {
    olr_under_skin_w_m2(gases, surface_k, 0.0, planet)
}

/// Outgoing longwave radiation, W/m², of the column of [`olr_w_m2`] whose
/// temperature never falls below `skin_k` (its stratosphere).
///
/// The column runs from the surface, where the vapour is `gases.h2o`
/// capped at saturation below water's critical point (and at
/// [`STEAM_MAX_BAR`] in any column, wet or runaway: the non-grey window
/// it stands for exists in every thick steam column), up to [`TOP_SHARE`] of the surface pressure
/// (never above [`TOP_BAR`]) on a log grid. Without air the surface
/// radiates as a black body. `planet`'s gravity turns the pressures into
/// the absorbers' column masses (`p / g`).
pub(crate) fn olr_under_skin_w_m2(
    gases: &Gases,
    surface_k: f64,
    skin_k: f64,
    planet: &Planet,
) -> f64 {
    olr_on_grid(gases, surface_k, skin_k, planet, LEVELS_PER_DECADE)
}

/// [`olr_under_skin_w_m2`] on the finer grid of
/// [`PLATEAU_LEVELS_PER_DECADE`], for the moist plateau.
pub(crate) fn plateau_olr_under_skin_w_m2(
    gases: &Gases,
    surface_k: f64,
    skin_k: f64,
    planet: &Planet,
) -> f64 {
    olr_on_grid(gases, surface_k, skin_k, planet, PLATEAU_LEVELS_PER_DECADE)
}

/// [`olr_under_skin_w_m2`] with `levels_per_decade` levels per decade.
fn olr_on_grid(
    gases: &Gases,
    surface_k: f64,
    skin_k: f64,
    planet: &Planet,
    levels_per_decade: f64,
) -> f64 {
    match olr_bands(gases, surface_k, skin_k, planet, levels_per_decade) {
        Some(bands) => bands.iter().sum(),
        None => SIGMA * surface_k.max(1.0).powi(4),
    }
}

/// The outgoing radiation of [`olr_under_skin_w_m2`], W/m², in each band
/// of [`spectrum`], on a grid of `levels_per_decade`; `None` without air.
fn olr_bands(
    gases: &Gases,
    surface_k: f64,
    skin_k: f64,
    planet: &Planet,
    levels_per_decade: f64,
) -> Option<[f64; BANDS]> {
    let surface_k = surface_k.max(1.0);
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
        return None;
    }
    let top_bar = (surface_bar * TOP_SHARE).min(TOP_BAR);
    let span = (surface_bar / top_bar).ln();
    // Fixed steps in ln p from the surface, the last one shorter, so that
    // the outgoing radiation does not jump as the surface pressure moves.
    let step = (std::f64::consts::LN_10 / levels_per_decade).max(span / MAX_LEVELS as f64);
    let n = ((span / step).ceil() as usize).clamp(1, MAX_LEVELS);
    let last = span - (n - 1) as f64 * step;
    let step_of = |i: usize| if i + 1 == n { last } else { step };

    // Temperature and gases of each level, from the surface up.
    let mut temperature = [0.0; MAX_LEVELS + 1];
    let mut levels = [Gases::default(); MAX_LEVELS + 1];
    let mut saturated = !supercritical && steam >= water_saturation_bar(surface_k);
    let mut frozen = false;
    let mut vapour_share = vapour / surface_bar;
    let mut t = surface_k;
    let mut p = surface_bar;
    let (ratio, half_ratio) = ((-step).exp(), (-0.5 * step).exp());
    let (last_ratio, last_half) = ((-last).exp(), (-0.5 * last).exp());
    // CO₂'s share of the dry air: where the air reaches its saturation it
    // condenses, the air follows its pseudo-adiabat ([`co2_lapse`]) and
    // the share falls to what saturation leaves; above the skin it keeps
    // the share of its cold trap.
    let mut co2_share = if air.pressure > 0.0 {
        air.gases.co2 / air.pressure
    } else {
        0.0
    };
    let mut co2_saturated = false;
    for i in 0..=n {
        let v = vapour_share * p;
        let level = air.with_co2(p - v, v, co2_share * (p - v));
        temperature[i] = t;
        levels[i] = level;
        if i == n {
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
        // Midpoint (second-order Runge–Kutta) step up the adiabat. Where
        // CO₂ saturates (so cold an air holds next to no water), its
        // pseudo-adiabat; the step in which it first saturates is split
        // where it does (the excess `ln(p_CO₂ / p_sat)` taken linear
        // across the step), so that the column moves smoothly with its
        // CO₂.
        let moist = |t: f64, from: f64, h: f64| {
            let half = t - 0.5 * h * co2_lapse(&air, t, from);
            t - h * co2_lapse(&air, half, from * (-0.5 * h).exp())
        };
        if co2_saturated {
            t = moist(t, p_here, h);
        } else {
            let half = t - 0.5 * h * lapse(&air, t, p_here, saturated, vapour_share);
            let next = t - h * lapse(&air, half, p_mid, saturated, vapour_share);
            let excess = |t: f64, co2: f64| (co2 / co2_saturation_bar(t)).ln();
            let below = excess(t, level.co2);
            let above = excess(next, co2_share * p * (1.0 - vapour_share));
            if level.co2 > 0.0 && above >= 0.0 {
                let f = if below >= 0.0 {
                    0.0
                } else {
                    below / (below - above)
                };
                let crossed = t + f * (next - t);
                t = moist(crossed, p_here * (-f * h).exp(), (1.0 - f) * h);
                co2_saturated = true;
            } else {
                t = next;
            }
        }
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
        let dry = p * (1.0 - vapour_share);
        if dry > 0.0 {
            co2_share = co2_share.min(co2_saturation_bar(t) / dry);
        }
    }

    // Two-stream radiation to space in each band, summed from the top
    // down: the layer-mean Planck flux of the band times what the layer
    // takes of the transmission to space. Within a band each g-point of
    // CO₂ and of water keeps its own transmission; the band's is the
    // product of the gases' weighted sums and the grey part's (random
    // overlap, see `spectrum`).
    let level_at = |i: usize| spectrum::Level {
        temperature_k: temperature[i],
        gases: levels[i],
        gravity_m_s2: planet.gravity_m_s2,
    };
    let (has_co2, has_water) = (gases.co2 > 0.0, vapour > 0.0);
    let w = spectrum::weights();
    let mut work = Box::new(Work::default());
    let Work {
        rate,
        rate_below,
        through: Spectrum { co2, water, grey },
    } = &mut *work;
    let (mut planck, mut planck_below) = ([0.0; BANDS], [0.0; BANDS]);
    let mut seen = [0.0; BANDS];
    absorption_spectrum(&level_at(n), rate);
    depth_above(&level_at(n), rate, rate_below);
    planck_spectrum(temperature[n], &mut planck);
    let mut out = [0.0; BANDS];
    for k in 0..BANDS {
        let mut through = (-DIFFUSIVITY * rate_below.grey[k]).exp();
        grey[k] = through;
        if has_co2 {
            co2[k] = rate_below.co2[k].map(|d| (-DIFFUSIVITY * d).exp());
            through *= dot(w, &co2[k]);
        }
        if has_water {
            water[k] = rate_below.water[k].map(|d| (-DIFFUSIVITY * d).exp());
            through *= dot(w, &water[k]);
        }
        seen[k] = through;
        out[k] = planck[k] * (1.0 - through);
    }
    for i in (0..n).rev() {
        absorption_spectrum(&level_at(i), rate_below);
        planck_spectrum(temperature[i], &mut planck_below);
        let h = step_of(i);
        let mut open = false;
        for k in 0..BANDS {
            grey[k] *= (-DIFFUSIVITY * log_mean(rate.grey[k], rate_below.grey[k]) * h).exp();
            let mut below = grey[k];
            if has_co2 {
                below *= layer(w, &mut co2[k], &rate.co2[k], &rate_below.co2[k], h);
            }
            if has_water {
                below *= layer(w, &mut water[k], &rate.water[k], &rate_below.water[k], h);
            }
            out[k] += 0.5 * (planck[k] + planck_below[k]) * (seen[k] - below);
            seen[k] = below;
            open |= below > OPAQUE;
        }
        std::mem::swap(rate, rate_below);
        std::mem::swap(&mut planck, &mut planck_below);
        if !open {
            return Some(out);
        }
    }
    // `planck` is now the surface's, at `temperature[0]`.
    for k in 0..BANDS {
        out[k] += planck[k] * seen[k];
    }
    Some(out)
}

/// Scratch of [`olr_bands`]: the absorption at a layer's two levels, and
/// the transmission to space of each g-point of CO₂ and water and of the
/// grey part, per band.
#[derive(Default)]
struct Work {
    rate: Spectrum,
    rate_below: Spectrum,
    through: Spectrum,
}

/// One gas through one layer of `span` in `ln p`: each g-point's
/// transmission `seen` loses the layer's depth, the log mean of its rates
/// at the layer's top and bottom; returns the gas's weighted transmission.
fn layer(w: &[f64; G], seen: &mut [f64; G], top: &[f64; G], bottom: &[f64; G], span: f64) -> f64 {
    let mut sum = 0.0;
    for g in 0..G {
        seen[g] *= (-DIFFUSIVITY * log_mean(top[g], bottom[g]) * span).exp();
        sum += w[g] * seen[g];
    }
    sum
}

/// `Σ a b`.
fn dot(a: &[f64; G], b: &[f64; G]) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet::climate::{
        MOIST_PLATEAU_W_M2, PLATEAU_SURFACES_K, Planet, tropopause_k, with_vapour,
    };

    /// Earth's dry air, bar.
    fn earth_air() -> Gases {
        Gases {
            n2: 0.78,
            o2: 0.21,
            co2: 4e-4,
            ..Gases::default()
        }
    }

    /// The skin of the moist plateau's solves: `T_eq / 2^¼` of a body that
    /// absorbs the plateau.
    fn plateau_skin() -> f64 {
        tropopause_k((*MOIST_PLATEAU_W_M2 / SIGMA).powf(0.25))
    }

    /// Outgoing radiation of `air` over a moist surface at `t`, under the
    /// plateau's skin, on Earth.
    fn moist_olr(air: Gases, t: f64) -> f64 {
        olr_under_skin_w_m2(
            &with_vapour(air, t, f64::INFINITY),
            t,
            plateau_skin(),
            &Planet::EARTH,
        )
    }

    #[test]
    fn the_moist_plateau_is_goldblatts() {
        // Goldblatt, Robinson, Zahnle & Crisp 2013: 282 W/m² line by line;
        // 5 % covers Boukrouche et al. 2021 (283) and Hamano et al. 2015
        // (280).
        let plateau = *MOIST_PLATEAU_W_M2;
        println!("moist plateau {plateau:.2} W/m²");
        assert!((plateau - 282.0).abs() <= 14.0, "{plateau} W/m²");
    }

    #[test]
    fn the_plateau_ignores_co2_and_nitrogen() {
        let base = moist_olr(earth_air(), 380.0);
        for co2 in [4e-4, 1e-3, 1e-2] {
            for n2 in [0.78, 1.56, 2.34] {
                let olr = moist_olr(
                    Gases {
                        co2,
                        n2,
                        ..earth_air()
                    },
                    380.0,
                );
                println!("CO₂ {co2} bar, N₂ {n2} bar: {olr:.2} W/m²");
                assert!(
                    (olr / base - 1.0).abs() < 0.03,
                    "CO₂ {co2}, N₂ {n2}: {olr} vs {base}"
                );
            }
        }
    }

    /// Outgoing radiation of Earth's moist air over a saturated surface at
    /// `t`, under the plateau's skin, on `levels` levels per decade.
    fn moist_olr_on(t: f64, levels: f64) -> f64 {
        olr_on_grid(
            &with_vapour(earth_air(), t, f64::INFINITY),
            t,
            plateau_skin(),
            &Planet::EARTH,
            levels,
        )
    }

    #[test]
    fn the_runaway_anchor_is_flat() {
        // Past the curve's maximum the outgoing radiation is flat within
        // 1 % of its asymptote, the plateau, over 500-640 K, and the
        // maximum (near 350-400 K, Goldblatt et al. 2013) stands at most
        // 6 % above it.
        let plateau = *MOIST_PLATEAU_W_M2;
        for t in PLATEAU_SURFACES_K {
            let olr = moist_olr_on(t, PLATEAU_LEVELS_PER_DECADE);
            println!("anchor at {t} K: {olr:.2} W/m²");
            assert!(
                (olr / plateau - 1.0).abs() < 0.01,
                "{t} K: {olr} vs {plateau}"
            );
        }
        let peak = (0..9)
            .map(|i| moist_olr_on(340.0 + 20.0 * i as f64, PLATEAU_LEVELS_PER_DECADE))
            .fold(0.0, f64::max);
        println!("peak {peak:.2} W/m² over the plateau {plateau:.2}");
        assert!(peak <= 1.06 * plateau, "peak {peak} vs {plateau}");
        assert!(peak > plateau, "no maximum before the plateau: {peak}");
    }

    #[test]
    fn the_plateaus_grid_is_converged() {
        let skin = plateau_skin();
        let mean = |levels: f64| {
            PLATEAU_SURFACES_K
                .iter()
                .map(|&t| moist_olr_on(t, levels))
                .sum::<f64>()
                / PLATEAU_SURFACES_K.len() as f64
        };
        let (plateau, fine) = (
            mean(PLATEAU_LEVELS_PER_DECADE),
            mean(2.0 * PLATEAU_LEVELS_PER_DECADE),
        );
        println!("asymptote under skin {skin:.1} K: {plateau:.2}, twice as fine {fine:.2}");
        assert!((plateau - fine).abs() < 0.5, "{plateau} vs {fine}");
    }

    #[test]
    fn a_column_without_co2_or_water_radiates_from_its_methane_or_ground() {
        // Titan: 1.45 bar of N₂ and 5 % methane over 93.7 K, under the skin
        // of its 82 K equilibrium, at its gravity.
        let titan = Planet {
            gravity_m_s2: 1.352,
            radius_earth: 0.404,
        };
        let air = Gases {
            n2: 1.45,
            ch4: 0.07,
            ..Gases::default()
        };
        let black = SIGMA * 93.7f64.powi(4);
        let olr = olr_under_skin_w_m2(&air, 93.7, tropopause_k(82.0), &titan);
        println!("Titan-like column: {olr:.3} W/m² against σT⁴ {black:.3}");
        assert!(olr.is_finite() && olr > 0.0 && olr < black, "{olr}");
        // Nitrogen alone is transparent: the ground radiates through it.
        let n2 = Gases {
            n2: 1.45,
            ..Gases::default()
        };
        let bare = olr_under_skin_w_m2(&n2, 93.7, tropopause_k(82.0), &titan);
        assert!((bare / black - 1.0).abs() < 1e-9, "{bare} vs {black}");
        // And with no air at all.
        let airless = olr_w_m2(&Gases::default(), 250.0, &Planet::EARTH);
        assert!((airless - SIGMA * 250f64.powi(4)).abs() < 1e-6, "{airless}");
    }

    #[test]
    fn venus_column_is_finite() {
        let venus = Planet {
            gravity_m_s2: 8.87,
            radius_earth: 0.95,
        };
        let air = Gases {
            co2: 92.0,
            ..Gases::default()
        };
        let olr = olr_under_skin_w_m2(&air, 737.0, tropopause_k(230.0), &venus);
        println!("Venus column: {olr:.3} W/m²");
        assert!(olr.is_finite() && olr > 0.0, "{olr}");
        assert!(olr < SIGMA * 737f64.powi(4), "{olr}");
    }
}
