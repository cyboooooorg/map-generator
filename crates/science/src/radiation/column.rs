//! A radiative–convective spectral column: the outgoing longwave
//! radiation of an atmosphere whose temperature follows its adiabats
//! (ported from `legacy-final`'s `planet/column.rs`, legacy issues #441
//! and #523).
//!
//! From the surface up, the air follows the dry adiabat of its mixture
//! until its water vapour saturates, then the moist pseudo-adiabat
//! (Pierrehumbert 2010, *Principles of Planetary Climate*, §2.7 "The
//! moist adiabat"); past water's critical point the vapour is a gas like
//! the others. Where the CO₂ of a cold column reaches its own saturation
//! (its frost below its triple point, its liquid above), it condenses and
//! the air follows CO₂'s pseudo-adiabat, its CO₂ thinning to what
//! saturation leaves (Kasting 1991, *Icarus* 94, 1,
//! doi:10.1016/0019-1035(91)90137-I, Appendix B, as Kopparapu et al. 2013,
//! *ApJ* 765, 131, doi:10.1088/0004-637X/765/2/131, §2, follow it; the
//! Appendix itself is paywalled and was not read), so that a thick CO₂
//! air's greenhouse flattens. A skin temperature floors the column (a
//! stratosphere, where the vapour and the CO₂ keep the mixing ratio of
//! their cold trap).
//!
//! The radiation is summed bin by bin over the thermal infrared
//! ([`super::spectrum`]): in each bin the transmission to space of CO₂'s
//! and water's lines (a correlated-k table, a few g-points per bin), of
//! their continua, of the CO₂–CO₂ pairs and of methane's grey stand-in,
//! and the two-stream, non-scattering emission of each layer's mean
//! Planck flux with the diffusivity 1.66. Whatever no gas closes stays
//! open: water's window, the far wings of thin CO₂.
//!
//! On a moist column the levels that radiate to space sit on water's
//! saturation curve whatever the surface temperature, so the outgoing
//! radiation flattens at the Simpson–Nakajima limit (Nakajima, Hayashi &
//! Abe 1992, *J. Atmos. Sci.* 49, 2256,
//! doi:10.1175/1520-0469(1992)049<2256:ASOTGE>2.0.CO;2) until the dry
//! adiabat from a hot surface reaches them.
//!
//! Water clouds cover the share of the sky [`super::clouds`] gives them:
//! that share radiates from a cloud top ([`CLOUD_TOP_SHARE`] of the
//! surface pressure, of emissivity [`cloud_emissivity`]) instead of from
//! the clear column below it ([`radiate`]).
//!
//! Above a temperate surface the troposphere's relative humidity falls
//! with the pressure ([`relative_humidity`], Manabe & Wetherald 1967);
//! over a saturated or supercritical surface, and for the moist plateau
//! ([`Profile::PLATEAU`]), it stays saturated, the column whose maximum
//! is the moist limit a wet surface runs away past.
//!
//! The water saturation and its latent heat are IAPWS's
//! ([`crate::water`]), the CO₂ and methane ones those of
//! [`crate::volatiles`]. The memo of the columns already solved and the
//! single-precision workspace of the sum belong to the caller
//! ([`ColumnCache`]).

use super::clouds::cloud_cover_share;
use super::fast_math;
use super::gases::{
    CH4_HEAT_CAPACITY, CO2_HEAT_CAPACITY, N2_HEAT_CAPACITY, NH3_HEAT_CAPACITY, O2_HEAT_CAPACITY,
    PartialPressures, SO2_HEAT_CAPACITY,
};
use super::spectrum::{
    self, BANDS, BINS, DIFFUSIVITY, FastLevel, G, POINTS, TAIL, planck_spectrum,
};
use crate::constants::{MOLAR_GAS_CONSTANT, STEFAN_BOLTZMANN};
use crate::units::{Kelvin, MetrePerSecondSquared, WattPerSquareMetre};
use crate::volatiles::{co2_latent_j_mol, co2_saturation_bar, methane_saturation_bar};
use crate::water::{CRITICAL_POINT_TEMPERATURE, vapour_latent_j_mol, vapour_saturation_bar};

#[cfg(test)]
#[path = "column_tests.rs"]
mod tests;

/// Definition: water's critical temperature, K ([`CRITICAL_POINT_TEMPERATURE`]):
/// above it no liquid exists, and the vapour is a gas like the others.
const WATER_CRITICAL_K: f64 = CRITICAL_POINT_TEMPERATURE.0;

/// Source: the Shomate coefficients `A` to `E` of water gas from 500 to
/// 1700 K (NIST Chemistry WebBook, water, gas-phase thermochemistry, Chase
/// 1998, *NIST-JANAF Thermochemical Tables*, 4th ed., J. Phys. Chem. Ref.
/// Data Monograph 9; checked digit for digit by the audit, Wiki study 19).
/// [`steam_cp`] also uses them below 500 K, outside their range, where
/// they still give the 33.6 J/(mol·K) of 300 K (the audit's check).
const STEAM_SHOMATE_LOW: [f64; 5] = [30.092, 6.832_514, 6.793_435, -2.534_48, 0.082_139];

/// Source: the Shomate coefficients `A` to `E` of water gas from 1700 to
/// 6000 K (NIST Chemistry WebBook, Chase 1998, as [`STEAM_SHOMATE_LOW`]).
const STEAM_SHOMATE_HIGH: [f64; 5] = [41.964_26, 8.622_053, -1.499_78, 0.098_119, -11.157_64];

/// Source: the temperature, K, where the NIST WebBook's two Shomate sets
/// of water gas meet (500-1700 K and 1700-6000 K, Chase 1998).
const STEAM_SHOMATE_SWITCH_K: f64 = 1700.0;

/// Source: the top of the high Shomate set of water gas, K (NIST WebBook,
/// Chase 1998); [`steam_cp`] holds its value above.
const STEAM_SHOMATE_TOP_K: f64 = 6000.0;

/// Definition: the floor, K, under which [`steam_cp`] holds its value, a
/// numerical guard with no physical source that keeps the Shomate
/// equation's `E / t²` finite; the column's vapour is cold-trapped long
/// before it.
const STEAM_SHOMATE_FLOOR_K: f64 = 200.0;

/// Definition: kelvins per the Shomate equation's unit of temperature
/// (`t = T / 1000`).
const SHOMATE_KELVIN: f64 = 1000.0;

/// Molar heat capacity of water vapour, J/(mol·K), at `temperature_k`:
/// the Shomate equation with the NIST Chemistry WebBook's coefficients
/// ([`STEAM_SHOMATE_LOW`], [`STEAM_SHOMATE_HIGH`]). It grows from 34 to
/// 51 J/(mol·K) between 300 and 2000 K, which flattens the dry adiabat of
/// hot steam.
fn steam_cp(temperature_k: f64) -> f64 {
    let t = temperature_k.clamp(STEAM_SHOMATE_FLOOR_K, STEAM_SHOMATE_TOP_K) / SHOMATE_KELVIN;
    let [a, b, c, d, e] = if temperature_k < STEAM_SHOMATE_SWITCH_K {
        STEAM_SHOMATE_LOW
    } else {
        STEAM_SHOMATE_HIGH
    };
    a + t * (b + t * (c + t * d)) + e / (t * t)
}

/// Definition: the transmission under which a band of the column is
/// opaque, `exp(−40)`: below it the levels and the surface it hides add
/// nothing to the outgoing radiation. A numerical cut-off with no source
/// (audit, Wiki study 19).
const OPAQUE: f64 = 4.248_354_255_291_589e-18;

/// Calibration: levels of the pressure grid per decade of pressure, a
/// numerical choice with no source, held by convergence (legacy task 7.4
/// of #523): against a grid two and a half times as fine, Earth's
/// outgoing radiation at 288 K moves by 0.13 to 0.21 W/m², its thermostat
/// CO₂ by 0.7 to 4.7 % (400 ppm to 1 bar), and the moist column over
/// 500-640 K by under 0.05 W/m²; ten levels move Earth's by 0.5 to
/// 0.6 W/m². No data exists for it: it is a property of the
/// discretisation (`docs/science/radiative-transfer.md`, numerical
/// choices).
const LEVELS_PER_DECADE: f64 = 20.0;

/// Calibration: levels per decade of the column that measures the moist
/// plateau ([`Profile::PLATEAU`]), a numerical choice held by the test
/// `the_plateaus_grid_is_converged`: against twice as many, the plateau
/// moves by under 0.5 W/m², and [`LEVELS_PER_DECADE`] gives it within
/// 0.05 W/m² (`docs/science/radiative-transfer.md`, numerical choices).
pub(crate) const PLATEAU_LEVELS_PER_DECADE: f64 = 80.0;

/// Definition: most levels in a column, eight decades of pressure at
/// twice [`PLATEAU_LEVELS_PER_DECADE`] (the steam is capped at
/// [`STEAM_MAX_BAR`], so a column spans about seven).
const MAX_LEVELS: usize = 1280;

/// Calibration: top of the column as a share of the surface pressure, a
/// numerical choice with no source and no convergence test yet: it keeps
/// the emitting levels of water (near 1e-3 bar) inside a thick column.
/// Above the top the air is taken uniform at the top's temperature
/// ([`spectrum::smooth_above`]), exact in a stratosphere at the skin
/// temperature (`docs/science/radiative-transfer.md`, numerical choices).
const TOP_SHARE: f64 = 1e-3;

/// Calibration: the column's top is never above this pressure, bar, the
/// companion of [`TOP_SHARE`], a numerical choice with no source.
const TOP_BAR: f64 = 1e-4;

/// Source: pressure of the water clouds' tops, as a share of the surface
/// pressure: ISCCP's global annual mean cloud-top pressure, 580 mb
/// (Rossow & Schiffer 1999, *BAMS* 80, 2261,
/// doi:10.1175/1520-0477(1999)080<2261:AIUCFI>2.0.CO;2, §6a and Table 4,
/// D2 1986), over Earth's global mean surface pressure, 985.50 hPa
/// (Trenberth & Smith 2005, *J. Climate* 18, 864, doi:10.1175/JCLI-3299.1,
/// abstract); both confirmed by the audit, Wiki study 19. ISCCP measures
/// the tops over the real ground, so the share is against the mean
/// surface pressure, not the sea level's.
pub(crate) const CLOUD_TOP_SHARE: f64 = 580.0 / 985.5;

/// Source: infrared optical depth of the water clouds: ISCCP's global mean
/// visible optical thickness, 3.8 (Rossow & Schiffer 1999, §6a and Table
/// 4), over 2, the empirical ratio of the visible to the infrared optical
/// thickness ISCCP takes (same paper, §5: `τ_ir = τ / 2.0`); confirmed by
/// the audit, Wiki study 19.
const CLOUD_IR_DEPTH: f64 = 3.8 / 2.0;

/// The clouds' infrared emissivity for the column's fluxes:
/// `1 − exp(−D τ_ir)`, with the diffusivity `D` the column's gases take
/// ([`DIFFUSIVITY`]): 0.957. A cloud is grey in the infrared, and black
/// past a few optical depths.
fn cloud_emissivity() -> f64 {
    1.0 - (-DIFFUSIVITY * CLOUD_IR_DEPTH).exp()
}

/// Source: pressure, bar, of the tropopause of a thick atmosphere, where
/// its air is coldest: about 0.1 bar on Earth, Titan, Jupiter, Saturn,
/// Uranus and Neptune, which Robinson & Catling 2014 (*Nature Geosci.* 7,
/// 12, doi:10.1038/ngeo2020) explain by the infrared opacity of the air
/// above it and the shortwave opacity of the air below (confirmed by the
/// audit, Wiki study 19). In a thinner air the whole column is above it.
const TROPOPAUSE_BAR: f64 = 0.1;

/// Calibration: the deepest steam, bar, the column holds: the steam under
/// it does not change the outgoing radiation. Boukrouche, Lichtenberg &
/// Pierrehumbert 2021 (*ApJ* 919, 130, doi:10.3847/1538-4357/ac1345,
/// §III.1) find the radiation of a steam atmosphere flat "between 300 and
/// 1800 K in the 260 bar case, and between 300 and 1050 K in the 10 bar
/// case", the end of the plateau reaching a constant of about 1800 K as
/// the steam thickens to 295 bar; Goldblatt 2015 (*Astrobiology* 15, 362,
/// doi:10.1089/ast.2014.1268, §3.1) puts the rise past 1500 K, through
/// the 4 µm window. The 70 bar itself is the legacy's fit (on the column
/// of legacy issue #441, which had no window) so that the plateau ends at
/// 1800 K; Boukrouche's figure of the radiation against the surface
/// temperature for 1-295 bar would source it and was not extracted
/// (audit, Wiki study 19). The cap is kept so that a runaway's steam stays
/// inside the opacity table (100 bar) and the grid; the window exists in
/// any thick steam column, so it holds for every column.
pub(crate) const STEAM_MAX_BAR: f64 = 70.0;

/// The skin temperature of a body whose effective temperature (that of
/// the light it absorbs and sends back out) is `effective`: the
/// temperature of a thin top layer of a grey atmosphere, which absorbs as
/// much of the planet's emission as it sends out up and down,
/// `T_eff / 2^(1/4)`.
///
/// Source: Pierrehumbert 2010, *Principles of Planetary Climate*, §3.6
/// "Optically thin atmospheres: the skin temperature" (the legacy's
/// `tropopause_k`, whose §4.3 the audit corrected). The column floors its
/// temperature there (its stratosphere); a negative effective temperature
/// reads as 0 K.
pub fn skin_temperature(effective: Kelvin) -> Kelvin {
    Kelvin(effective.0.max(0.0) * 0.5_f64.powf(0.25))
}

/// The methane, bar at the surface, that radiates in a column of `gases`
/// (summing to `pressure_bar`) under a stratosphere at `skin_k`: methane
/// is cold-trapped where the air is coldest, at the tropopause
/// ([`TROPOPAUSE_BAR`]) at about the skin temperature, so that its mixing
/// ratio above it is at most the saturation pressure of pure methane there
/// over the tropopause's pressure ([`methane_saturation_bar`]). Without a
/// stratosphere (`skin_k` 0) all of it. The construction is the model's
/// own: each ingredient is sourced, the combination is not (audit, Wiki
/// study 19).
fn radiating_methane_bar(gases: &PartialPressures, pressure_bar: f64, skin_k: f64) -> f64 {
    let ch4 = gases.ch4.max(0.0);
    if skin_k <= 0.0 || ch4 <= 0.0 {
        return ch4;
    }
    let tropopause = TROPOPAUSE_BAR.min(pressure_bar);
    let share = methane_saturation_bar(skin_k) / tropopause;
    ch4.min(share * pressure_bar)
}

/// The methane, bar, above the tropopause of a uniform air of `gases`
/// under a stratosphere at `skin_k`: its column there, the mixing ratio
/// the cold trap lets through ([`radiating_methane_bar`]) times the
/// tropopause's pressure ([`TROPOPAUSE_BAR`]; the whole of a thinner
/// air). The methane whose near-infrared bands take a share of the
/// starlight before it reaches the troposphere (the anti-greenhouse of
/// PR 2b-2).
#[expect(
    dead_code,
    reason = "the methane anti-greenhouse of PR 2b-2 reads it; ported with the column"
)]
pub(crate) fn stratospheric_methane_bar(gases: &PartialPressures, skin_k: f64) -> f64 {
    let pressure = gases.total();
    if pressure <= 0.0 {
        return 0.0;
    }
    radiating_methane_bar(gases, pressure, skin_k) * TROPOPAUSE_BAR.min(pressure) / pressure
}

/// The gases of a column without their water, and the molar heat capacity
/// of the mixture.
struct DryAir {
    /// Partial pressures, bar, at the surface (no water).
    gases: PartialPressures,
    /// The methane among them that radiates, bar at the surface
    /// ([`radiating_methane_bar`]).
    radiating_ch4: f64,
    /// Their sum, bar.
    pressure: f64,
    /// Molar heat capacity of the mixture, J/(mol·K).
    cp: f64,
}

impl DryAir {
    /// The dry air of `gases` under a stratosphere at `skin_k`; a negative
    /// or NaN part is none.
    fn of(gases: &PartialPressures, skin_k: f64) -> Self {
        let g = PartialPressures {
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
            (g.n2 * N2_HEAT_CAPACITY
                + g.co2 * CO2_HEAT_CAPACITY
                + g.ch4 * CH4_HEAT_CAPACITY
                + g.nh3 * NH3_HEAT_CAPACITY
                + g.so2 * SO2_HEAT_CAPACITY
                + g.o2 * O2_HEAT_CAPACITY)
                / pressure
        } else {
            N2_HEAT_CAPACITY
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
    /// condensation leaves there ([`with_column`]): the other gases share
    /// the rest of the dry air in their surface proportions, and the
    /// methane cold-trapped at the tropopause (see
    /// [`radiating_methane_bar`]) is left out.
    fn with_co2(&self, dry_bar: f64, vapour_bar: f64, co2_bar: f64) -> PartialPressures {
        let others = self.pressure - self.gases.co2;
        let share = if others > 0.0 {
            (dry_bar - co2_bar).max(0.0) / others
        } else {
            0.0
        };
        PartialPressures {
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
            (g.n2 * N2_HEAT_CAPACITY
                + g.ch4 * CH4_HEAT_CAPACITY
                + g.nh3 * NH3_HEAT_CAPACITY
                + g.so2 * SO2_HEAT_CAPACITY
                + g.o2 * O2_HEAT_CAPACITY)
                / others
        } else {
            N2_HEAT_CAPACITY
        }
    }
}

/// `d T / d ln p`, K, at `temperature_k` and `pressure_bar` on the moist
/// pseudo-adiabat of `latent_j_mol` (the condensable's molar latent heat)
/// for a condensable of saturation pressure `saturation_bar` and molar
/// heat capacity `cp_condensable`, condensing out of a non-condensing air
/// of molar heat capacity `cp_air`.
///
/// Pierrehumbert 2010, §2.7, `d ln T / d ln p_a = (R_a/c_pa)(1 + L r /
/// (R_a T)) / (1 + (c_pv/c_pa + (L/(R_v T) − 1) L/(c_pa T)) r)` with `r`
/// the condensable's mass mixing ratio (his eq. 2.33, the number quoted
/// secondhand: audit, Wiki study 19), written per mole and multiplied
/// through by `p_a` so that it holds down to the pure condensable; then
/// `d ln p = d ln p_a (p_a + e_s L A/(R T)) / p` with `A` that slope,
/// since `p = p_a + e_s(T)`. The pure condensable follows its
/// Clausius–Clapeyron curve, `d T/d ln p = R T²/L`.
fn pseudo_adiabat(
    temperature_k: f64,
    pressure_bar: f64,
    saturation_bar: f64,
    latent_j_mol: f64,
    cp_air: f64,
    cp_condensable: f64,
) -> f64 {
    let t = temperature_k;
    let e = saturation_bar.min(pressure_bar);
    let dry = (pressure_bar - e).max(0.0);
    let l_over_r = latent_j_mol / MOLAR_GAS_CONSTANT;
    let l_t = latent_j_mol / t;
    let a = (MOLAR_GAS_CONSTANT * dry + l_t * e)
        / (cp_air * dry + (cp_condensable + (l_over_r / t - 1.0) * l_t) * e);
    a * t * pressure_bar / (dry + a * e * l_over_r / t)
}

/// `d T / d ln p`, K, at `temperature_k` and `pressure_bar` on the moist
/// pseudo-adiabat of CO₂ condensing out of `air` ([`pseudo_adiabat`] with
/// CO₂ the condensable, [`co2_latent_j_mol`] its latent heat, and the
/// other gases and the little water vapour so cold an air holds the
/// non-condensing part): Kasting 1991, Appendix B, which Kopparapu et al.
/// 2013 (§2) follow "in the upper troposphere when condensation was
/// encountered" (the Appendix itself not read: audit, Wiki study 19).
fn co2_lapse(air: &DryAir, temperature_k: f64, pressure_bar: f64) -> f64 {
    let t = temperature_k;
    pseudo_adiabat(
        t,
        pressure_bar,
        co2_saturation_bar(t),
        co2_latent_j_mol(t),
        air.cp_without_co2(),
        CO2_HEAT_CAPACITY,
    )
}

/// `d T / d ln p`, K, at `temperature_k` and `pressure_bar` on water's
/// moist pseudo-adiabat (`saturated`, [`pseudo_adiabat`] with the IAPWS
/// saturation and its latent heat, [`vapour_latent_j_mol`]) or on the dry
/// adiabat of air holding `vapour_share` of vapour (mole fraction).
fn lapse(
    air: &DryAir,
    temperature_k: f64,
    pressure_bar: f64,
    saturated: bool,
    vapour_share: f64,
) -> f64 {
    let t = temperature_k;
    if !saturated && vapour_share <= 0.0 {
        return t * MOLAR_GAS_CONSTANT / air.cp;
    }
    let cp_v = steam_cp(t);
    if saturated {
        pseudo_adiabat(
            t,
            pressure_bar,
            vapour_saturation_bar(t),
            vapour_latent_j_mol(t),
            air.cp,
            cp_v,
        )
    } else {
        let cp = (1.0 - vapour_share) * air.cp + vapour_share * cp_v;
        t * MOLAR_GAS_CONSTANT / cp
    }
}

/// The sky and humidity of a column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sky {
    /// The climate's: its water clouds radiate in its place
    /// ([`outgoing_longwave`]) and its troposphere's humidity falls with
    /// the pressure ([`relative_humidity`]).
    Cloudy,
    /// The same air under a clear sky.
    Clear,
    /// The moist plateau's: a clear sky over a troposphere saturated
    /// above where its vapour condenses, as the line-by-line plateaus of
    /// Goldblatt et al. 2013 and Kopparapu et al. 2013 (§3.1, "a fully
    /// saturated troposphere") take it.
    Plateau,
}

impl Sky {
    /// The sky as a number of the memo's key.
    fn key(self) -> f64 {
        match self {
            Sky::Cloudy => 0.0,
            Sky::Clear => 1.0,
            Sky::Plateau => 2.0,
        }
    }
}

/// Source: the floor of the troposphere's relative humidity profile, as
/// a share of the surface pressure: `RH_s (Q − 0.02) / (1 − 0.02)`,
/// Manabe & Wetherald 1967 (*J. Atmos. Sci.* 24, 241,
/// doi:10.1175/1520-0469(1967)024<0241:TEOTAW>2.0.CO;2, eq. 2). The paper
/// is not open; the profile is confirmed indirectly by climlab's
/// `ManabeWaterVapor` (`RH_s ((Q − 0.02)/(1 − 0.02))`, "following Manabe
/// 1967") and by arXiv:2006.03098, as the audit (Wiki study 19) records.
const HUMIDITY_FLOOR_SHARE: f64 = 0.02;

/// Relative humidity of the troposphere at `share` of the surface
/// pressure (`Q = p / p_s` of the whole air), over a surface at `surface`
/// relative humidity: `RH_s (Q − 0.02) / (1 − 0.02)`
/// ([`HUMIDITY_FLOOR_SHARE`], Manabe & Wetherald 1967). The surface keeps
/// its own humidity; the air dries upwards, to none at 2 % of the surface
/// pressure.
fn relative_humidity(surface: f64, share: f64) -> f64 {
    surface * ((share - HUMIDITY_FLOOR_SHARE) / (1.0 - HUMIDITY_FLOOR_SHARE)).max(0.0)
}

/// How a column is built: its sky and humidity and its pressure grid,
/// `levels_per_decade` levels per decade from the surface up to
/// `top_share` of the surface pressure, never above `top_bar`. Every
/// column of the model is one of these, through one builder.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Profile {
    sky: Sky,
    levels_per_decade: f64,
    top_share: f64,
    top_bar: f64,
}

impl Profile {
    /// The climate's column: its water clouds, its falling humidity, on
    /// [`LEVELS_PER_DECADE`] up to [`TOP_SHARE`] of the surface pressure
    /// (never above [`TOP_BAR`]).
    pub const CLIMATE: Profile = Profile {
        sky: Sky::Cloudy,
        levels_per_decade: LEVELS_PER_DECADE,
        top_share: TOP_SHARE,
        top_bar: TOP_BAR,
    };

    /// [`Profile::CLIMATE`] under a clear sky.
    #[cfg(test)]
    pub(crate) const CLEAR: Profile = Profile {
        sky: Sky::Clear,
        ..Profile::CLIMATE
    };

    /// [`Profile::CLIMATE`] under a clear sky over a troposphere saturated
    /// above where its vapour condenses: the column whose maximum outgoing
    /// radiation is the moist limit a wet surface runs away past.
    pub const SATURATED: Profile = Profile {
        sky: Sky::Plateau,
        ..Profile::CLIMATE
    };

    /// [`Profile::SATURATED`] on the finer grid of
    /// [`PLATEAU_LEVELS_PER_DECADE`], for the moist plateau: Goldblatt et
    /// al. 2013's and the other line-by-line plateaus are clear-sky and
    /// saturated.
    pub const PLATEAU: Profile = Profile {
        levels_per_decade: PLATEAU_LEVELS_PER_DECADE,
        ..Profile::SATURATED
    };

    /// The column the star's near infrared crosses (the shortwave of PR
    /// 2b-2, through [`levels`]): the same adiabats and the same falling
    /// humidity as the climate's, on a coarser grid
    /// ([`SHORTWAVE_LEVELS_PER_DECADE`], up to [`SHORTWAVE_TOP_SHARE`] of
    /// the surface pressure), clear.
    pub const SHORTWAVE: Profile = Profile {
        sky: Sky::Clear,
        levels_per_decade: SHORTWAVE_LEVELS_PER_DECADE,
        top_share: SHORTWAVE_TOP_SHARE,
        top_bar: f64::INFINITY,
    };

    /// This profile on `levels_per_decade` levels per decade.
    #[cfg(test)]
    pub(crate) const fn on(self, levels_per_decade: f64) -> Profile {
        Profile {
            levels_per_decade,
            ..self
        }
    }

    /// This profile up to `top_share` of the surface pressure.
    #[cfg(test)]
    pub(crate) const fn up_to(self, top_share: f64) -> Profile {
        Profile { top_share, ..self }
    }
}

/// Outgoing longwave radiation of a column with surface partial pressures
/// `gases` (`gases.h2o` the vapour at the surface) over a surface at
/// `surface`, whose temperature never falls below `skin` (its
/// stratosphere, [`skin_temperature`]; 0 K for none), on a body of
/// surface gravity `gravity`, built as `profile` says. See the module
/// documentation.
///
/// The column runs from the surface, where the vapour is `gases.h2o`
/// capped at saturation below water's critical point (and at
/// [`STEAM_MAX_BAR`] in any column, wet or runaway), up to the profile's
/// top on a log grid. Without air the surface radiates as a black body.
/// The gravity turns the pressures into the absorbers' column masses
/// (`p / g`). A negative or NaN partial pressure counts as none.
///
/// Under the climate's sky ([`Profile::CLIMATE`]) the water clouds,
/// `f` of the sky ([`super::clouds::cloud_cover`] of the surface
/// temperature and `gases.h2o`), send out the cloudy column's radiation,
/// the rest the clear column's: `(1 − f) OLR_clear + f OLR_cloudy`.
///
/// `cache` keeps the last columns solved with their outgoing radiation,
/// keyed on the exact bits of their inputs, and the workspace of the sum:
/// a hit returns the bits a fresh column gives.
pub fn outgoing_longwave(
    gases: &PartialPressures,
    surface: Kelvin,
    skin: Kelvin,
    gravity: MetrePerSecondSquared,
    profile: Profile,
    cache: &mut ColumnCache,
) -> WattPerSquareMetre {
    let (surface_k, skin_k) = (surface.0, skin.0);
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
        gravity.0,
        profile.levels_per_decade,
        profile.top_share,
        profile.top_bar,
        profile.sky.key(),
    ]
    .map(f64::to_bits);
    let memory = cache.memory.get_or_insert_with(Memory::new);
    if let Some(olr) = memory.find(&key) {
        return WattPerSquareMetre(olr);
    }
    let scratch = cache.scratch.get_or_insert_with(Scratch::new);
    let column = with_column(gases, surface_k, skin_k, gravity.0, profile, |c| {
        radiate(c, scratch)
    });
    let olr = match column {
        Some(bands) => bands.iter().sum(),
        None => STEFAN_BOLTZMANN * surface_k.max(1.0).powi(4),
    };
    memory.keep(key, olr);
    WattPerSquareMetre(olr)
}

/// Definition: columns a [`ColumnCache`] remembers, the legacy memo's
/// size (a numerical choice with no source: audit, Wiki study 19).
const REMEMBERED: usize = 1024;

/// The inputs of a column, as bits: the seven partial pressures, the
/// surface and skin temperatures, the gravity, and the profile's levels
/// per decade, top and sky.
type ColumnKey = [u64; 14];

/// What [`outgoing_longwave`] keeps between calls, owned by the caller:
/// the last [`REMEMBERED`] columns and their outgoing radiation, the
/// oldest replaced first, and the single-precision workspace of the sum
/// (some 100 kB). Both are allocated on first use. A planet's solves ask
/// for the same column more than once; one cache per thread of work.
#[derive(Default)]
pub struct ColumnCache {
    memory: Option<Box<Memory>>,
    scratch: Option<Box<Scratch>>,
}

impl std::fmt::Debug for ColumnCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ColumnCache")
            .field("remembered", &self.memory.as_ref().map_or(0, |m| m.len))
            .field("scratch", &self.scratch.is_some())
            .finish()
    }
}

/// The last [`REMEMBERED`] columns and their outgoing radiation, the
/// oldest replaced first.
struct Memory {
    keys: [ColumnKey; REMEMBERED],
    olr: [f64; REMEMBERED],
    len: usize,
    next: usize,
}

impl Memory {
    fn new() -> Box<Self> {
        Box::new(Self {
            keys: [[0; 14]; REMEMBERED],
            olr: [0.0; REMEMBERED],
            len: 0,
            next: 0,
        })
    }

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

/// Calibration: levels per decade of the column the star's near infrared
/// is absorbed in ([`Profile::SHORTWAVE`]), a numerical choice held by
/// convergence: against sixteen per decade, three move Earth's Bond albedo
/// by under 1e-3 and its clear-sky shortwave absorption by under 1 W/m²
/// (the legacy test `albedo::tests::the_shortwave_column_is_converged`,
/// which PR 2b-2 ports).
const SHORTWAVE_LEVELS_PER_DECADE: f64 = 3.0;

/// Calibration: top of the shortwave column, as a share of the surface
/// pressure, a numerical choice with no source and no convergence test:
/// the air above holds 1 % of the column's gases, and the bands it would
/// absorb in are the strongest, whose cores the air below closes anyway.
const SHORTWAVE_TOP_SHARE: f64 = 1e-2;

/// A column's levels, as the shortwave reads them: each level's
/// temperature and partial pressures from the surface up, and the span in
/// `ln p` of each layer between two levels.
#[expect(dead_code, reason = "the shortwave of PR 2b-2 reads it")]
pub(crate) struct Levels {
    pub(crate) levels: Vec<spectrum::Level>,
    pub(crate) spans: Vec<f64>,
}

/// The levels of the column of `gases` over a surface at `surface_k` on a
/// body of surface gravity `gravity_m_s2`, built as `profile` says, with
/// no stratosphere ([`Profile::SHORTWAVE`]: the albedo is solved before
/// the skin it sets, and the air above the tropopause absorbs next to
/// none of the near infrared that CO₂ and water take). `None` without
/// air.
#[expect(dead_code, reason = "the shortwave of PR 2b-2 reads it")]
pub(crate) fn levels(
    gases: &PartialPressures,
    surface_k: f64,
    gravity_m_s2: f64,
    profile: Profile,
) -> Option<Levels> {
    with_column(gases, surface_k, 0.0, gravity_m_s2, profile, |c| Levels {
        levels: (0..=c.top()).map(|i| c.level(i)).collect(),
        spans: (0..c.top()).map(|i| c.span(i)).collect(),
    })
}

/// Builds the column of [`outgoing_longwave`] under a stratosphere at
/// `skin_k` as `profile` says and hands it to `use_column` (the fast sum
/// [`radiate`], the shortwave's [`levels`], or the tests' reference sum);
/// `None` without air.
fn with_column<R>(
    gases: &PartialPressures,
    surface_k: f64,
    skin_k: f64,
    gravity_m_s2: f64,
    profile: Profile,
    use_column: impl FnOnce(&Column) -> R,
) -> Option<R> {
    let Profile {
        sky,
        levels_per_decade,
        top_share,
        top_bar,
    } = profile;
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
        steam.min(vapour_saturation_bar(surface_k))
    };
    let surface_bar = air.pressure + vapour;
    if surface_bar <= 0.0 {
        return None;
    }
    let top_bar = (surface_bar * top_share).min(top_bar);
    let span = (surface_bar / top_bar).ln();
    // Fixed steps in ln p from the surface, the last one shorter, so that
    // the outgoing radiation does not jump as the surface pressure moves.
    let step = (std::f64::consts::LN_10 / levels_per_decade).max(span / MAX_LEVELS as f64);
    let n = ((span / step).ceil() as usize).clamp(1, MAX_LEVELS);
    let last = span - (n - 1) as f64 * step;

    // Temperature and gases of each level, from the surface up.
    let mut temperature = [0.0; MAX_LEVELS + 1];
    let mut levels = [PartialPressures::default(); MAX_LEVELS + 1];
    let mut saturated = !supercritical && steam >= vapour_saturation_bar(surface_k);
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
    // The vapour that radiates: in the climate's columns the troposphere's
    // relative humidity falls with the pressure from the surface's
    // ([`relative_humidity`]), the temperature still following the
    // adiabats above; above the cold trap, the mixing ratio it leaves.
    // A supercritical level holds its vapour as a gas. Over a saturated
    // or supercritical surface (a boiling ocean, a runaway's steam,
    // where the vapour is the air rather than a trace in it) the column
    // stays saturated, as Kopparapu et al. 2013 (§2, §3.1) take their
    // moist and runaway atmospheres.
    let falling = sky != Sky::Plateau && !supercritical && !saturated;
    let surface_humidity = if falling {
        (vapour / vapour_saturation_bar(surface_k)).min(1.0)
    } else {
        1.0
    };
    let mut trapped_share: Option<f64> = None;
    for i in 0..=n {
        let mut v = vapour_share * p;
        if falling {
            if let Some(share) = trapped_share {
                v = v.min(share * p);
            } else if t < WATER_CRITICAL_K {
                let profile = relative_humidity(surface_humidity, p / surface_bar);
                v = v.min(profile * vapour_saturation_bar(t));
            }
        }
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
                    (vapour_share * p / vapour_saturation_bar(t.min(WATER_CRITICAL_K))).ln()
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
            if falling && trapped_share.is_none() {
                let profile = relative_humidity(surface_humidity, trap / surface_bar);
                trapped_share = Some(profile * vapour_saturation_bar(t).min(trap) / trap);
            }
        }
        if saturated || t < WATER_CRITICAL_K {
            let e = vapour_saturation_bar(t);
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

    // The cloud top, `CLOUD_TOP_SHARE` of the surface pressure: in the
    // layer above level `i`, `x` of its span up.
    let cover = match sky {
        Sky::Cloudy => cloud_cover_share(surface_k, gases.h2o),
        Sky::Clear | Sky::Plateau => 0.0,
    };
    let cloud = (cover > 0.0).then(|| {
        let up = -CLOUD_TOP_SHARE.ln();
        let i = ((up / step).floor() as usize).min(n - 1);
        let x = ((up - i as f64 * step) / column_span(i, n, step, last)).clamp(0.0, 1.0);
        Cloud {
            layer: i,
            up: x,
            weight: cover * cloud_emissivity(),
        }
    });
    let column = Column {
        temperature: &temperature[..=n],
        levels: &levels[..=n],
        step,
        last,
        gravity_m_s2,
        has_co2: gases.co2 > 0.0,
        has_water: vapour > 0.0,
        cloud,
    };
    Some(use_column(&column))
}

/// Span in `ln p` of the layer above level `i` of a column of `n`
/// layers, `step` each but the last (top) one's `last`.
fn column_span(i: usize, n: usize, step: f64, last: f64) -> f64 {
    if i + 1 == n { last } else { step }
}

/// A column's water clouds: their top sits `up` (0 to 1) of the way up
/// the layer above level `layer`, in `ln p`, and `weight` is their cover
/// times their emissivity ([`radiate`]).
#[derive(Clone, Copy)]
struct Cloud {
    layer: usize,
    up: f64,
    weight: f64,
}

/// The levels of a column, from the surface up, as its radiation reads
/// them.
struct Column<'a> {
    /// Temperature of each level, K.
    temperature: &'a [f64],
    /// Partial pressures of each level, bar.
    levels: &'a [PartialPressures],
    /// Step in `ln p` between levels, and the last (top) one.
    step: f64,
    last: f64,
    /// Surface gravity, m/s².
    gravity_m_s2: f64,
    /// Whether the column holds CO₂ and water vapour.
    has_co2: bool,
    has_water: bool,
    /// Its water clouds, if any.
    cloud: Option<Cloud>,
}

impl Column<'_> {
    /// Index of the top level.
    fn top(&self) -> usize {
        self.levels.len() - 1
    }

    /// Span in `ln p` of the layer above level `i`.
    fn span(&self, i: usize) -> f64 {
        column_span(i, self.top(), self.step, self.last)
    }

    fn level(&self, i: usize) -> spectrum::Level {
        spectrum::Level {
            temperature_k: self.temperature[i],
            gases: self.levels[i],
            gravity_m_s2: self.gravity_m_s2,
        }
    }
}

/// Workspace of [`radiate`]: the absorption at a layer's two levels, the
/// slant optical depth to space of each g-point of CO₂ and water and of
/// the smooth part, and their transmissions. Kept in the caller's
/// [`ColumnCache`]: it is some 100 kB.
struct Scratch {
    top: FastLevel,
    bottom: FastLevel,
    co2: Depths,
    water: Depths,
    smooth: [f32; BINS],
}

impl Scratch {
    fn new() -> Box<Self> {
        Box::new(Self {
            top: FastLevel::default(),
            bottom: FastLevel::default(),
            co2: Depths::new(),
            water: Depths::new(),
            smooth: [0.0; BINS],
        })
    }
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
    /// `ln p` (see `spectrum::reference::depth_above`), slanted.
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

/// Definition: share of `σT⁴` at a column's warmest level under which the
/// bins past it are left out of the sum ([`radiate`]), about the
/// precision of a single-precision sum (`f32` epsilon 1.2e-7; audit, Wiki
/// study 19).
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
/// the gases' weighted sums and the smooth part's (random overlap, see
/// [`super::spectrum`]). The tail band takes the last bin's absorption, so
/// its transmission is that bin's. A layer's depth is the logarithmic
/// mean of the opacities at its two levels times its span. Above the top
/// the air is uniform ([`spectrum::smooth_above`]). The sum stops where
/// every bin is opaque ([`OPAQUE`]); the bins a cold column cannot send
/// out ([`summed_bins`]) are taken as clear, the surface's.
///
/// Water clouds ([`Cloud`]) cover `f` of the sky with a grey layer of
/// emissivity `ε` at their top. Under it the clear column's radiation
/// from below reaches the top through `1 − ε` of it, and the cloud adds
/// `ε B(T_c)` times the transmission to space from its top, so that
/// each band's `(1 − f) OLR_clear + f OLR_cloudy` is
/// `OLR_clear − f ε (OLR_clear − above_c − seen_c B_c)`, with `above_c`
/// the radiation of the air above the top, `seen_c` its transmission to
/// space and `B_c` its Planck flux, each taken linear in `ln p` across
/// the layer it sits in. The cloudy column costs no second sum. Bins a
/// cold column leaves out stay clear; a column opaque above the cloud
/// sends out the same either way.
///
/// In single precision with [`spectrum::fast_level`] and [`fast_math`]
/// (the test `the_fast_radiation_is_the_reference` holds it to the
/// double-precision sum of `spectrum`), in the caller's `scratch`.
fn radiate(column: &Column, scratch: &mut Scratch) -> [f64; BANDS] {
    let Scratch {
        top,
        bottom,
        co2,
        water,
        smooth,
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
    for (b, (depth, rate)) in smooth.iter_mut().zip(&top.smooth).take(bins).enumerate() {
        *depth = (DIFFUSIVITY * spectrum::smooth_above(&top_level, f64::from(*rate), b)) as f32;
    }
    let through = |b: usize, smooth: &[f32; BINS], co2: &Depths, water: &Depths| {
        let mut through = fast_math::exp(-smooth[b]);
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
        seen[b] = through(b, smooth, co2, water);
        out[b] = planck[b] * (1.0 - seen[b]);
    }
    if tail {
        out[TAIL] = planck[TAIL] * (1.0 - seen[BINS - 1]);
    }
    // What a cloud top sends out per band, `above_c + seen_c B_c`, once
    // the sum has passed it.
    let mut cloud_top: Option<[f64; BANDS]> = None;
    let mut above = [0.0; BANDS];
    let mut seen_above = [0.0; BINS];
    for i in (0..n).rev() {
        let at_cloud = column.cloud.filter(|c| c.layer == i);
        if at_cloud.is_some() {
            above = out;
            seen_above = seen;
        }
        spectrum::fast_level(&column.level(i), bins, bottom);
        spectrum::planck_spectrum_upto(column.temperature[i], bins, &mut planck_below);
        let slant = (DIFFUSIVITY * column.span(i)) as f32;
        deepen(
            &mut smooth[..bins],
            (&top.smooth[..bins], &top.smooth_ln[..bins]),
            (&bottom.smooth[..bins], &bottom.smooth_ln[..bins]),
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
            let below = through(b, smooth, co2, water);
            out[b] += 0.5 * (planck[b] + planck_below[b]) * (seen[b] - below);
            seen[b] = below;
            open |= below > OPAQUE;
        }
        if tail {
            out[TAIL] += 0.5 * (planck[TAIL] + planck_below[TAIL]) * (last_seen - seen[BINS - 1]);
        }
        if let Some(cloud) = at_cloud {
            let x = cloud.up;
            let mid = |below: f64, above: f64| below + x * (above - below);
            let mut sent = [0.0; BANDS];
            for b in 0..bins {
                let seen_c = mid(seen[b], seen_above[b]);
                sent[b] = mid(out[b], above[b]) + seen_c * mid(planck_below[b], planck[b]);
            }
            if tail {
                let seen_c = mid(seen[BINS - 1], seen_above[BINS - 1]);
                sent[TAIL] =
                    mid(out[TAIL], above[TAIL]) + seen_c * mid(planck_below[TAIL], planck[TAIL]);
            }
            cloud_top = Some(sent);
        }
        std::mem::swap(top, bottom);
        std::mem::swap(&mut planck, &mut planck_below);
        if !open {
            return under_clouds(out, column.cloud, cloud_top, bins, tail);
        }
    }
    // `planck` is now the surface's, at `temperature[0]`.
    for b in 0..bins {
        out[b] += planck[b] * seen[b];
    }
    if tail {
        out[TAIL] += planck[TAIL] * seen[BINS - 1];
    }
    under_clouds(out, column.cloud, cloud_top, bins, tail)
}

/// The clear column's radiation `out` per band, with `cloud`'s share of
/// the sky sending out what its top does (`sent`, once the sum has
/// passed it; see [`radiate`]) over the first `bins` bins, and the tail
/// with them when `tail`.
fn under_clouds(
    mut out: [f64; BANDS],
    cloud: Option<Cloud>,
    sent: Option<[f64; BANDS]>,
    bins: usize,
    tail: bool,
) -> [f64; BANDS] {
    if let (Some(cloud), Some(sent)) = (cloud, sent) {
        let blend = |clear: f64, sent: f64| clear - cloud.weight * (clear - sent);
        for b in 0..bins {
            out[b] = blend(out[b], sent[b]);
        }
        if tail {
            out[TAIL] = blend(out[TAIL], sent[TAIL]);
        }
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

/// Definition: the distance `|ln(b/a)|` under which [`log_mean_f32`]
/// takes its series: there the series' remainder is under 1e-10, the
/// exact form's cancellation larger.
const LOG_MEAN_SERIES_BELOW: f32 = 1e-2;

/// The logarithmic mean (`spectrum::reference::log_mean`) of `a` and `b`
/// given their logarithms, branch-free: within 1 % of each other, its
/// series `a (1 + d/2 + d²/6 + d³/24)` in `d = ln (b/a)`, and the
/// arithmetic mean if either is zero.
#[inline(always)]
fn log_mean_f32(a: f32, ln_a: f32, b: f32, ln_b: f32) -> f32 {
    let d = ln_b - ln_a;
    let series = a * (1.0 + d * (0.5 + d * (1.0 / 6.0 + d * (1.0 / 24.0))));
    let exact = (b - a) / d;
    let mean = if d.abs() < LOG_MEAN_SERIES_BELOW {
        series
    } else {
        exact
    };
    if a > 0.0 && b > 0.0 {
        mean
    } else {
        0.5 * (a + b)
    }
}

/// The double-precision sum that [`radiate`] stands for, band by band
/// with `spectrum::reference::absorption_spectrum`, over every band and
/// level.
#[cfg(test)]
fn radiate_reference(column: &Column) -> [f64; BANDS] {
    use spectrum::reference::{Spectrum, absorption_spectrum, depth_above, log_mean};
    let n = column.top();
    let (has_co2, has_water) = (column.has_co2, column.has_water);
    let w = spectrum::weights();
    let mut rate = Spectrum::default();
    let mut rate_below = Spectrum::default();
    let mut through_g = Spectrum::default();
    let Spectrum { co2, water, smooth } = &mut through_g;
    let (mut planck, mut planck_below) = ([0.0; BANDS], [0.0; BANDS]);
    let mut seen = [0.0; BANDS];
    absorption_spectrum(&column.level(n), &mut rate);
    depth_above(&column.level(n), &rate, &mut rate_below);
    planck_spectrum(column.temperature[n], &mut planck);
    let mut out = [0.0; BANDS];
    for k in 0..BANDS {
        let mut through = (-DIFFUSIVITY * rate_below.smooth[k]).exp();
        smooth[k] = through;
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
    let mut cloud_top = None;
    for i in (0..n).rev() {
        let (above, seen_above) = (out, seen);
        absorption_spectrum(&column.level(i), &mut rate_below);
        planck_spectrum(column.temperature[i], &mut planck_below);
        let h = column.span(i);
        for k in 0..BANDS {
            smooth[k] *= (-DIFFUSIVITY * log_mean(rate.smooth[k], rate_below.smooth[k]) * h).exp();
            let mut below = smooth[k];
            if has_co2 {
                below *= layer(w, &mut co2[k], &rate.co2[k], &rate_below.co2[k], h);
            }
            if has_water {
                below *= layer(w, &mut water[k], &rate.water[k], &rate_below.water[k], h);
            }
            out[k] += 0.5 * (planck[k] + planck_below[k]) * (seen[k] - below);
            seen[k] = below;
        }
        if let Some(cloud) = column.cloud.filter(|c| c.layer == i) {
            let mid = |below: f64, above: f64| below + cloud.up * (above - below);
            let mut sent = [0.0; BANDS];
            for k in 0..BANDS {
                sent[k] = mid(out[k], above[k])
                    + mid(seen[k], seen_above[k]) * mid(planck_below[k], planck[k]);
            }
            cloud_top = Some(sent);
        }
        std::mem::swap(&mut rate, &mut rate_below);
        std::mem::swap(&mut planck, &mut planck_below);
    }
    for k in 0..BANDS {
        out[k] += planck[k] * seen[k];
    }
    under_clouds(out, column.cloud, cloud_top, BINS, true)
}

/// One gas through one layer of `span` in `ln p`: each g-point's
/// transmission `seen` loses the layer's depth, the log mean of its rates
/// at the layer's top and bottom; returns the gas's weighted transmission.
#[cfg(test)]
fn layer(w: &[f64; G], seen: &mut [f64; G], top: &[f64; G], bottom: &[f64; G], span: f64) -> f64 {
    let mut sum = 0.0;
    for g in 0..G {
        seen[g] *= (-DIFFUSIVITY * spectrum::reference::log_mean(top[g], bottom[g]) * span).exp();
        sum += w[g] * seen[g];
    }
    sum
}

/// `Σ a b`.
#[cfg(test)]
fn dot(a: &[f64; G], b: &[f64; G]) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
