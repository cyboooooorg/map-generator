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
use super::fast_math;
use super::spectrum::{
    self, BANDS, BINS, DIFFUSIVITY, FastLevel, G, POINTS, TAIL, planck_spectrum,
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

/// Levels of the pressure grid per decade of pressure. Calibration
/// (task 7.4 of #523, on the spectral column): against a grid two and a
/// half times as fine, Earth's outgoing radiation at 288 K moves by 0.13
/// to 0.21 W/m², so its thermostat CO₂ by 0.7 to 4.7 % (400 ppm to
/// 1 bar), and the moist column over 500-640 K by under 0.05 W/m²; ten
/// levels move Earth's by 0.5 to 0.6 W/m², its CO₂ by up to 14 %.
const LEVELS_PER_DECADE: f64 = 20.0;

/// Levels per decade of the column that measures the moist plateau
/// ([`plateau_olr_under_skin_w_m2`]): against twice as many, the plateau
/// moves by under 0.5 W/m² (the test
/// `the_plateaus_grid_is_converged`; 0.01 W/m² once the saturation and
/// the cold trap fall between levels, see [`olr_bands`]), and
/// [`LEVELS_PER_DECADE`] gives it within 0.05 W/m².
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
///
/// A planet's solves ask for the same column more than once (its settle
/// loop runs the same balances again): the last [`REMEMBERED`] columns of
/// the thread are kept with their outgoing radiation, keyed on the exact
/// bits of their inputs, so that the answer is the same.
fn olr_on_grid(
    gases: &Gases,
    surface_k: f64,
    skin_k: f64,
    planet: &Planet,
    levels_per_decade: f64,
) -> f64 {
    let key = [
        gases.n2,
        gases.co2,
        gases.h2o,
        gases.ch4,
        gases.nh3,
        gases.so2,
        gases.o2,
        surface_k,
        skin_k,
        planet.gravity_m_s2,
        levels_per_decade,
    ]
    .map(f64::to_bits);
    if let Some(olr) = REMEMBERED_OLR.with_borrow(|memory| memory.find(&key)) {
        return olr;
    }
    let olr = match olr_bands(gases, surface_k, skin_k, planet, levels_per_decade, radiate) {
        Some(bands) => bands.iter().sum(),
        None => SIGMA * surface_k.max(1.0).powi(4),
    };
    REMEMBERED_OLR.with_borrow_mut(|memory| memory.keep(key, olr));
    olr
}

/// Columns [`olr_on_grid`] keeps per thread.
const REMEMBERED: usize = 1024;

/// The inputs of a column, as bits: the seven partial pressures, the
/// surface and skin temperatures, the gravity and the levels per decade.
type ColumnKey = [u64; 11];

/// The last [`REMEMBERED`] columns of a thread and their outgoing
/// radiation, the oldest replaced first.
struct Memory {
    keys: [ColumnKey; REMEMBERED],
    olr: [f64; REMEMBERED],
    len: usize,
    next: usize,
}

impl Memory {
    fn find(&self, key: &ColumnKey) -> Option<f64> {
        (0..self.len)
            .find(|&i| self.keys[i] == *key)
            .map(|i| self.olr[i])
    }

    fn keep(&mut self, key: ColumnKey, olr: f64) {
        self.keys[self.next] = key;
        self.olr[self.next] = olr;
        self.next = (self.next + 1) % REMEMBERED;
        self.len = (self.len + 1).min(REMEMBERED);
    }
}

thread_local! {
    static REMEMBERED_OLR: std::cell::RefCell<Memory> = const {
        std::cell::RefCell::new(Memory {
            keys: [[0; 11]; REMEMBERED],
            olr: [0.0; REMEMBERED],
            len: 0,
            next: 0,
        })
    };
}

/// The outgoing radiation of [`olr_under_skin_w_m2`], W/m², in each band
/// of [`spectrum`], on a grid of `levels_per_decade`, summed by
/// `radiation` ([`radiate`]); `None` without air.
fn olr_bands(
    gases: &Gases,
    surface_k: f64,
    skin_k: f64,
    planet: &Planet,
    levels_per_decade: f64,
    radiation: fn(&Column) -> [f64; BANDS],
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
        let start = t;
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
            let mut next = t - h * lapse(&air, half, p_mid, saturated, vapour_share);
            // The step in which the vapour first saturates is split where
            // it does (the excess `ln(e / e_sat)` taken linear across the
            // step), the rest taken on the moist adiabat: the vapour the
            // column keeps above then moves smoothly with the grid and the
            // surface, instead of jumping by a step's condensation.
            if !saturated && vapour_share > 0.0 && next < WATER_CRITICAL_K {
                let excess = |t: f64, p: f64| {
                    (vapour_share * p / water_saturation_bar(t.min(WATER_CRITICAL_K))).ln()
                };
                let (below, above) = (excess(t, p_here), excess(next, p));
                if above >= 0.0 {
                    let f = if below >= 0.0 {
                        0.0
                    } else {
                        below / (below - above)
                    };
                    let crossed = t + f * (next - t);
                    let from = p_here * (-f * h).exp();
                    let rest = (1.0 - f) * h;
                    let half = crossed - 0.5 * rest * lapse(&air, crossed, from, true, 0.0);
                    next =
                        crossed - rest * lapse(&air, half, from * (-0.5 * rest).exp(), true, 0.0);
                    saturated = true;
                }
            }
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
        // The cold trap: where the air reaches the skin, its vapour and its
        // CO₂ keep the mixing ratio saturation leaves there, at the
        // pressure where it does (`t` taken linear across the step), so
        // that it moves smoothly with the grid and the surface.
        let mut trap = p;
        if t <= skin_k {
            if start > skin_k {
                trap = p_here * (-h * (start - skin_k) / (start - t)).exp();
            }
            t = skin_k;
            frozen = true;
        }
        if saturated || t < WATER_CRITICAL_K {
            let e = water_saturation_bar(t);
            if saturated || vapour_share * trap >= e {
                saturated = true;
                vapour_share = e.min(trap) / trap;
            }
        }
        let dry = trap * (1.0 - vapour_share);
        if dry > 0.0 {
            co2_share = co2_share.min(co2_saturation_bar(t) / dry);
        }
    }

    let column = Column {
        temperature: &temperature[..=n],
        levels: &levels[..=n],
        step,
        last,
        gravity_m_s2: planet.gravity_m_s2,
        has_co2: gases.co2 > 0.0,
        has_water: vapour > 0.0,
    };
    Some(radiation(&column))
}

/// The levels of a column, from the surface up, as its radiation reads
/// them.
struct Column<'a> {
    /// Temperature of each level, K.
    temperature: &'a [f64],
    /// Partial pressures of each level, bar.
    levels: &'a [Gases],
    /// Step in `ln p` between levels, and the last (top) one.
    step: f64,
    last: f64,
    /// Surface gravity, m/s².
    gravity_m_s2: f64,
    /// Whether the column holds CO₂ and water vapour.
    has_co2: bool,
    has_water: bool,
}

impl Column<'_> {
    /// Index of the top level.
    fn top(&self) -> usize {
        self.levels.len() - 1
    }

    /// Span in `ln p` of the layer above level `i`.
    fn span(&self, i: usize) -> f64 {
        if i + 1 == self.top() {
            self.last
        } else {
            self.step
        }
    }

    fn level(&self, i: usize) -> spectrum::Level {
        spectrum::Level {
            temperature_k: self.temperature[i],
            gases: self.levels[i],
            gravity_m_s2: self.gravity_m_s2,
        }
    }
}

/// Scratch of [`radiate`]: the absorption at a layer's two levels, the
/// slant optical depth to space of each g-point of CO₂ and water and of
/// the grey part, and their transmissions. Kept per thread: it is some
/// 120 kB.
struct Scratch {
    top: FastLevel,
    bottom: FastLevel,
    co2: Depths,
    water: Depths,
    grey: [f32; BINS],
}

/// A gas's slant optical depth to space at each bin and g-point, and its
/// transmission.
struct Depths {
    depth: [f32; POINTS],
    through: [f32; POINTS],
}

impl Depths {
    fn new() -> Self {
        Self {
            depth: [0.0; POINTS],
            through: [0.0; POINTS],
        }
    }

    /// The depths of the first `m` g-points (over the bins) in the uniform
    /// air above the top, from its opacities `rate`: half its rate per
    /// `ln p` (see [`spectrum::depth_above`]), slanted.
    fn above(&mut self, rate: &[f32; POINTS], m: usize) {
        let half = 0.5 * DIFFUSIVITY as f32;
        for (d, r) in self.depth[..m].iter_mut().zip(&rate[..m]) {
            *d = half * r;
        }
        transmissions(&self.depth[..m], &mut self.through[..m]);
    }

    /// One layer of `slant` (its span in `ln p` times [`DIFFUSIVITY`])
    /// over the first `m` g-points: each one's depth gains the log mean of
    /// its opacities at the layer's top and bottom (each the opacity and
    /// its logarithm).
    fn deepen(
        &mut self,
        top: (&[f32; POINTS], &[f32; POINTS]),
        bottom: (&[f32; POINTS], &[f32; POINTS]),
        slant: f32,
        m: usize,
    ) {
        deepen(
            &mut self.depth[..m],
            (&top.0[..m], &top.1[..m]),
            (&bottom.0[..m], &bottom.1[..m]),
            slant,
        );
        transmissions(&self.depth[..m], &mut self.through[..m]);
    }

    /// The gas's transmission in bin `b`, summed over its g-points.
    fn bin(&self, weights: &[f32; G], b: usize) -> f32 {
        let through = &self.through[b * G..(b + 1) * G];
        weights.iter().zip(through).map(|(w, t)| w * t).sum()
    }
}

thread_local! {
    static SCRATCH: std::cell::RefCell<Box<Scratch>> = std::cell::RefCell::new(Box::new(Scratch {
        top: FastLevel::default(),
        bottom: FastLevel::default(),
        co2: Depths::new(),
        water: Depths::new(),
        grey: [0.0; BINS],
    }));
}

/// Share of `σT⁴` at a column's warmest level under which the bins past
/// it are left out of the sum ([`radiate`]): about the precision of its
/// single-precision sums.
const NEGLIGIBLE: f64 = 1e-7;

/// The bins a column of `temperature` (K, each level's) sums: all of them
/// but the last ones whose black-body flux at the warmest level, with the
/// tail's, is under [`NEGLIGIBLE`] of the total. A column under 300 K
/// stops short of 5000 cm⁻¹, one under 100 K short of 1800 cm⁻¹.
fn summed_bins(temperature: &[f64]) -> usize {
    let warmest = temperature.iter().copied().fold(0.0, f64::max);
    let mut planck = [0.0; BANDS];
    planck_spectrum(warmest, &mut planck);
    let floor = NEGLIGIBLE * planck.iter().sum::<f64>();
    let mut rest = planck[TAIL];
    let mut bins = BINS;
    while bins > 1 && rest + planck[bins - 1] <= floor {
        rest += planck[bins - 1];
        bins -= 1;
    }
    bins
}

/// Two-stream radiation to space in each band of `column`, summed from the
/// top down: the layer-mean Planck flux of the band times what the layer
/// takes of the transmission to space. Within a bin each g-point of CO₂
/// and of water keeps its own transmission; the bin's is the product of
/// the gases' weighted sums and the grey part's (random overlap, see
/// `spectrum`). The tail band takes the last bin's absorption, so its
/// transmission is that bin's. A layer's depth is the logarithmic mean of
/// the opacities at its two levels times its span. Above the top the air
/// is uniform ([`spectrum::depth_above`]). The sum stops where every bin
/// is opaque ([`OPAQUE`]); the bins a cold column cannot send out
/// ([`summed_bins`]) are taken as clear, the surface's.
///
/// In single precision with [`spectrum::fast_level`] and [`fast_math`]
/// (the test `the_fast_radiation_is_the_reference` holds it to the
/// double-precision sum of `spectrum`).
fn radiate(column: &Column) -> [f64; BANDS] {
    SCRATCH.with_borrow_mut(|scratch| radiate_with(column, scratch))
}

fn radiate_with(column: &Column, scratch: &mut Scratch) -> [f64; BANDS] {
    let Scratch {
        top,
        bottom,
        co2,
        water,
        grey,
    } = scratch;
    let n = column.top();
    let weights = spectrum::weights().map(|w| w as f32);
    let (has_co2, has_water) = (column.has_co2, column.has_water);
    let bins = summed_bins(column.temperature);
    let m = bins * G;
    let mut seen = [0.0; BINS];
    let mut out = [0.0; BANDS];
    if bins < BINS {
        let mut ground = [0.0; BANDS];
        planck_spectrum(column.temperature[0], &mut ground);
        out[bins..].copy_from_slice(&ground[bins..]);
    }
    let (mut planck, mut planck_below) = ([0.0; BANDS], [0.0; BANDS]);
    let top_level = column.level(n);
    spectrum::fast_level(&top_level, bins, top);
    spectrum::planck_spectrum_upto(column.temperature[n], bins, &mut planck);
    if has_co2 {
        co2.above(&top.co2, m);
    }
    if has_water {
        water.above(&top.water, m);
    }
    for (b, (depth, rate)) in grey.iter_mut().zip(&top.grey).take(bins).enumerate() {
        *depth = (DIFFUSIVITY * spectrum::grey_above(&top_level, f64::from(*rate), b)) as f32;
    }
    let through = |b: usize, grey: &[f32; BINS], co2: &Depths, water: &Depths| {
        let mut through = fast_math::exp(-grey[b]);
        if has_co2 {
            through *= co2.bin(&weights, b);
        }
        if has_water {
            through *= water.bin(&weights, b);
        }
        f64::from(through)
    };
    // The tail is summed with the last bin, when every bin is.
    let tail = bins == BINS;
    for b in 0..bins {
        seen[b] = through(b, grey, co2, water);
        out[b] = planck[b] * (1.0 - seen[b]);
    }
    if tail {
        out[TAIL] = planck[TAIL] * (1.0 - seen[BINS - 1]);
    }
    for i in (0..n).rev() {
        spectrum::fast_level(&column.level(i), bins, bottom);
        spectrum::planck_spectrum_upto(column.temperature[i], bins, &mut planck_below);
        let slant = (DIFFUSIVITY * column.span(i)) as f32;
        deepen(
            &mut grey[..bins],
            (&top.grey[..bins], &top.grey_ln[..bins]),
            (&bottom.grey[..bins], &bottom.grey_ln[..bins]),
            slant,
        );
        if has_co2 {
            co2.deepen(
                (&top.co2, &top.co2_ln),
                (&bottom.co2, &bottom.co2_ln),
                slant,
                m,
            );
        }
        if has_water {
            water.deepen(
                (&top.water, &top.water_ln),
                (&bottom.water, &bottom.water_ln),
                slant,
                m,
            );
        }
        let mut open = false;
        let last_seen = seen[BINS - 1];
        for b in 0..bins {
            let below = through(b, grey, co2, water);
            out[b] += 0.5 * (planck[b] + planck_below[b]) * (seen[b] - below);
            seen[b] = below;
            open |= below > OPAQUE;
        }
        if tail {
            out[TAIL] += 0.5 * (planck[TAIL] + planck_below[TAIL]) * (last_seen - seen[BINS - 1]);
        }
        std::mem::swap(top, bottom);
        std::mem::swap(&mut planck, &mut planck_below);
        if !open {
            return out;
        }
    }
    // `planck` is now the surface's, at `temperature[0]`.
    for b in 0..bins {
        out[b] += planck[b] * seen[b];
    }
    if tail {
        out[TAIL] += planck[TAIL] * seen[BINS - 1];
    }
    out
}

/// `depth` gains `slant` times the log mean of the opacities at a layer's
/// top and bottom (each the opacity and its logarithm), in a function of
/// its own so that it takes vector instructions.
#[inline(never)]
fn deepen(depth: &mut [f32], top: (&[f32], &[f32]), bottom: (&[f32], &[f32]), slant: f32) {
    let layers = top.0.iter().zip(top.1).zip(bottom.0.iter().zip(bottom.1));
    for (d, ((a, ln_a), (b, ln_b))) in depth.iter_mut().zip(layers) {
        *d += slant * log_mean_f32(*a, *ln_a, *b, *ln_b);
    }
}

/// `e^(−depth)` of every depth, into `out`.
#[inline(never)]
fn transmissions(depth: &[f32], out: &mut [f32]) {
    for (o, d) in out.iter_mut().zip(depth) {
        *o = fast_math::exp(-d);
    }
}

/// [`log_mean`] of `a` and `b` given their logarithms, branch-free: within
/// 1 % of each other, its series `a (1 + d/2 + d²/6 + d³/24)` in `d = ln
/// (b/a)` (remainder under 1e-10), and the arithmetic mean if either is
/// zero.
#[inline(always)]
fn log_mean_f32(a: f32, ln_a: f32, b: f32, ln_b: f32) -> f32 {
    let d = ln_b - ln_a;
    let series = a * (1.0 + d * (0.5 + d * (1.0 / 6.0 + d * (1.0 / 24.0))));
    let exact = (b - a) / d;
    let mean = if d.abs() < 1e-2 { series } else { exact };
    if a > 0.0 && b > 0.0 {
        mean
    } else {
        0.5 * (a + b)
    }
}

/// The double-precision sum that [`radiate`] stands for, band by band
/// with [`spectrum::absorption_spectrum`], over every band and level.
#[cfg(test)]
fn radiate_reference(column: &Column) -> [f64; BANDS] {
    use spectrum::{Spectrum, absorption_spectrum, depth_above};
    let n = column.top();
    let (has_co2, has_water) = (column.has_co2, column.has_water);
    let w = spectrum::weights();
    let mut rate = Spectrum::default();
    let mut rate_below = Spectrum::default();
    let mut through_g = Spectrum::default();
    let Spectrum { co2, water, grey } = &mut through_g;
    let (mut planck, mut planck_below) = ([0.0; BANDS], [0.0; BANDS]);
    let mut seen = [0.0; BANDS];
    absorption_spectrum(&column.level(n), &mut rate);
    depth_above(&column.level(n), &rate, &mut rate_below);
    planck_spectrum(column.temperature[n], &mut planck);
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
        absorption_spectrum(&column.level(i), &mut rate_below);
        planck_spectrum(column.temperature[i], &mut planck_below);
        let h = column.span(i);
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
        }
        std::mem::swap(&mut rate, &mut rate_below);
        std::mem::swap(&mut planck, &mut planck_below);
    }
    for k in 0..BANDS {
        out[k] += planck[k] * seen[k];
    }
    out
}

/// One gas through one layer of `span` in `ln p`: each g-point's
/// transmission `seen` loses the layer's depth, the log mean of its rates
/// at the layer's top and bottom; returns the gas's weighted transmission.
#[cfg(test)]
fn layer(w: &[f64; G], seen: &mut [f64; G], top: &[f64; G], bottom: &[f64; G], span: f64) -> f64 {
    let mut sum = 0.0;
    for g in 0..G {
        seen[g] *= (-DIFFUSIVITY * log_mean(top[g], bottom[g]) * span).exp();
        sum += w[g] * seen[g];
    }
    sum
}

/// `Σ a b`.
#[cfg(test)]
fn dot(a: &[f64; G], b: &[f64; G]) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}

/// Logarithmic mean of two opacities per `ln p`: the exact mean over a
/// layer where the opacity grows exponentially in `ln p` (a power of the
/// pressure, as through a uniform air), and the arithmetic mean's limit
/// when they are equal.
#[cfg(test)]
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
    fn the_fast_radiation_is_the_reference() {
        // The single-precision sum over the open bins against the
        // double-precision one over every band, on columns from Titan to
        // Venus and a steam runaway.
        let titan = Planet {
            gravity_m_s2: 1.352,
            radius_earth: 0.404,
        };
        let venus = Planet {
            gravity_m_s2: 8.87,
            radius_earth: 0.95,
        };
        let mars = Planet {
            gravity_m_s2: 3.71,
            radius_earth: 0.53,
        };
        let moist = |t: f64| with_vapour(earth_air(), t, f64::INFINITY);
        let cases = [
            (moist(288.0), 288.0, 200.0, Planet::EARTH),
            (moist(250.0), 250.0, 180.0, Planet::EARTH),
            (moist(380.0), 380.0, plateau_skin(), Planet::EARTH),
            (moist(600.0), 600.0, plateau_skin(), Planet::EARTH),
            (
                Gases {
                    h2o: 70.0,
                    co2: 6.0,
                    ..earth_air()
                },
                1500.0,
                250.0,
                Planet::EARTH,
            ),
            (
                Gases {
                    co2: 92.0,
                    n2: 3.2,
                    h2o: 3e-3,
                    ..Gases::default()
                },
                737.0,
                tropopause_k(230.0),
                venus,
            ),
            (
                Gases {
                    co2: 6e-3,
                    ..Gases::default()
                },
                210.0,
                tropopause_k(200.0),
                mars,
            ),
            (
                Gases {
                    n2: 1.45,
                    ch4: 0.07,
                    ..Gases::default()
                },
                93.7,
                tropopause_k(82.0),
                titan,
            ),
            (
                Gases {
                    co2: 0.3,
                    ..earth_air()
                },
                288.0,
                190.0,
                Planet::EARTH,
            ),
        ];
        for (gases, t, skin, planet) in cases {
            let olr = |radiation| {
                olr_bands(&gases, t, skin, &planet, LEVELS_PER_DECADE, radiation)
                    .expect("air")
                    .iter()
                    .sum::<f64>()
            };
            let (fast, reference) = (olr(radiate), olr(radiate_reference));
            println!("{t} K: fast {fast:.5}, reference {reference:.5} W/m²");
            assert!(
                (fast - reference).abs() < 1e-5 * reference + 1e-6,
                "{gases:?} at {t} K: {fast} vs {reference}"
            );
        }
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
