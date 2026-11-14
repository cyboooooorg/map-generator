//! The surface climate of a rocky or icy body and the liquid on its
//! surface (part 2 of the physical planet model, issue #423).
//!
//! [`surface_temperature_k`] warms the equilibrium temperature through a
//! grey greenhouse of CO₂, H₂O and CH₄; [`climate`] adds the water vapour
//! that an ocean or ice keeps in the air, runs the carbonate thermostat of
//! a planet with plates and water, and reads the liquid at the surface off
//! the phase diagrams of water, hydrocarbons and basalt.
use serde::{Deserialize, Serialize};

use super::{Context, Gases, Water};
use crate::geology::Regime;

/// The liquid that covers part of a surface, if any.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum SurfaceLiquid {
    None,
    Water,
    Hydrocarbons,
    Lava,
}

/// The climate at a body's surface.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Climate {
    /// Mean surface temperature, K.
    pub temperature_k: f64,
    pub liquid: SurfaceLiquid,
    /// The carbonate thermostat holds the temperature at
    /// [`THERMOSTAT_K`]: plates, liquid water and enough CO₂ to reach it.
    pub regulated: bool,
}

/// Stefan–Boltzmann constant, W/(m²·K⁴) (CODATA 2018, exact).
const SIGMA: f64 = 5.670_374_419e-8;

/// Grey opacity of CO₂ at 1 bar of CO₂ in 1 bar of air. Calibration, no
/// source: Venus's 737 K under its 85 bar of CO₂ (with
/// [`CO2_EXPONENT`]); it also leaves Mars about 3 K above its
/// equilibrium temperature.
const CO2_OPACITY: f64 = 1.66;

/// Power of the CO₂ partial pressure in its opacity. Calibration, no
/// source: CO₂'s 15 µm band is saturated at its centre, so its forcing
/// grows with the logarithm of its amount (Myhre et al. 1998,
/// doi:10.1029/98GL01908, `ΔF = 5.35 ln(C/C₀)` W/m²); a small power
/// stands in for that logarithm across the six decades from Earth's
/// 3e-4 bar to Venus's 85 bar.
const CO2_EXPONENT: f64 = 0.25;

/// Grey opacity of water vapour at 1 bar in 1 bar of air, with
/// [`LINE_EXPONENT`]. Calibration, no source: with Earth's vapour at
/// 288 K, the thermostat leaves about 3e-4 bar of CO₂ in its air (the
/// pre-industrial 280 ppm) and the vapour makes most of the greenhouse.
const H2O_OPACITY: f64 = 5.9;

/// Grey opacity of methane at 1 bar in 1 bar of air, with
/// [`LINE_EXPONENT`]. Calibration, no source: Titan's 94 K under its
/// 1.5 % of methane in 1.5 bar (2.1 bar in the model). Titan's real
/// greenhouse is mostly collision-induced (N₂–N₂, N₂–CH₄, N₂–H₂; McKay,
/// Pollack & Courtin 1991, doi:10.1126/science.11538492, not checked),
/// and its haze cools it by about 9 K; this opacity carries all of it.
/// That is why it keeps the square root of the whole air's pressure (see
/// [`optical_depth`]): collision-induced absorption grows with the
/// nitrogen faster than line broadening does, not slower.
const CH4_OPACITY: f64 = 3.4;

/// Power of a partial pressure, and of the broadening pressure, in a
/// gas's opacity: the strong-line (square-root) limit, where a pressure-
/// broadened line absorbs as `sqrt(S u α)`, its absorber path `u ∝ p_i`
/// and its width `α` growing with the pressure of the gases round it
/// (Pierrehumbert 2010, *Principles of Planetary Climate*, §4.4; Goody &
/// Yung 1989, *Atmospheric Radiation*, ch. 4).
const LINE_EXPONENT: f64 = 0.5;

/// Pressure of the other gases, bar, up to which they broaden the lines
/// of CO₂ and water vapour in proportion (see [`broadening_bar`]): the
/// gases other than CO₂ and water themselves (N₂, O₂, CH₄...).
/// Calibration: Earth's air, so that Earth, Mars and Venus keep the plain
/// square root of their pressure they were calibrated with.
const BROADENING_KNEE_BAR: f64 = 1.0;

/// Power of the other gases' pressure in the broadening pressure past
/// [`BROADENING_KNEE_BAR`]: their share of a grey opacity then grows as
/// its square root, `P^0.1`, not `P^0.5`.
///
/// Calibration on the warming of doubled nitrogen. Goldblatt et al. 2009
/// (*Nature Geosci.* 2, 891, doi:10.1038/ngeo692; abstract checked) find
/// that doubling the present nitrogen warms the late Archean Earth by
/// 4.4 °C in a 1-D radiative–convective model; Charnay et al. 2013 (*JGR
/// Atmos.* 118, 10414, doi:10.1002/jgrd.50808; §5.2 checked in arXiv
/// 1310.4286) find +7 °C in a GCM for the air going from 1 to 2 bar at
/// 2.5 Ga with 10 mbar of CO₂ and 2 mbar of CH₄, and Wolf & Toon 2014
/// (*Astrobiology* 14, 241, doi:10.1089/ast.2013.1112; abstract checked)
/// 3 to 7 K for plausible nitrogen. In Charnay's setting (no CH₄) the
/// model warms by about 5 K with 0.2, by about 3 K with 0.1 and 8.5 K
/// with 0.3; with the plain square root (1) it ran away, and so did an
/// Earth at 1 AU with 2 bar of N₂ (issue #440). The real rise is milder
/// than the strong-line law because lines overlap and saturate (Vladilo
/// et al. 2013, ApJ 767, 65, doi:10.1088/0004-637X/767/1/65, App. A.3,
/// checked), and the moist runaway limit is set by the water vapour
/// alone (Nakajima, Hayashi & Abe 1992, *J. Atmos. Sci.* 49, 2256,
/// doi:10.1175/1520-0469(1992)049<2256:ASOTGE>2.0.CO;2; abstract
/// checked), which a grey opacity growing as `sqrt(P)` lowers with every
/// bar of nitrogen. The 0.2 power still lowers it, less: the model's
/// peak outgoing radiation at Earth's composition is 253 W/m², 239 W/m²
/// with twice its N₂ (211 with the square root), so past about 11 bar of
/// N₂ an Earth at 1 AU still runs away once its thermostat has no CO₂
/// left to remove.
const THICK_AIR_EXPONENT: f64 = 0.2;

/// Pressure, bar, that broadens the lines of CO₂ and of water vapour in
/// `gases`: the two of them in full, plus the other gases (N₂, O₂, CH₄,
/// the rest), in proportion up to [`BROADENING_KNEE_BAR`] and as the
/// [`THICK_AIR_EXPONENT`] power past it. It is the whole pressure in any
/// air with less than 1 bar of other gases.
///
/// A gas broadens its own lines in full (pure CO₂ on Venus and Mars, a
/// steam atmosphere), and CO₂ and water broaden each other's lines at
/// least as well as N₂ does, so neither is tempered as a foreign gas of
/// the other: water vapour broadens CO₂'s lines much more than N₂ and O₂
/// do (Tan, Kochanov, Rothman & Gordon 2019, *JGR Atmos.* 124, 11580,
/// doi:10.1029/2019JD030929; abstract checked), and CO₂ broadens water's
/// 1.33 to 1.45 times as much as air (Yang et al. 2022, *Acta Phys. Sin.*
/// 71, 203301, doi:10.7498/aps.71.20220700; abstract checked; Brown,
/// Humphrey & Gamache 2007, *J. Mol. Spectrosc.* 246, 1,
/// doi:10.1016/j.jms.2007.07.010, in the far infrared, not checked).
/// They count here at par with N₂ under the knee, not above it, so that
/// the opacities keep their calibration. Tempering the CO₂ as water's
/// broadener (issue #440's first rule) cooled an Earth at 1.6 AU, all
/// its 1.8 bar of CO₂ in the air, from 287.8 K to 281 K: it moved in the
/// cold edge of the habitable zone, which Kopparapu et al. 2013 (*ApJ*
/// 765, 131, doi:10.1088/0004-637X/765/2/131) put at about 1.67 AU with
/// CO₂–H₂O atmospheres, by a broadening no calibration constrains (the
/// doubled-nitrogen one holds 10 mbar of CO₂).
///
/// Steam counts up to [`STEAM_MAX_BAR`], the column at which it reaches
/// the cap of the Simpson–Nakajima limit: the outgoing radiation of a
/// thicker steam atmosphere comes from levels above it, so the deeper
/// steam no more broadens CO₂'s lines there than it adds to water's own
/// opacity. Without it, the 1.6e5 bar of a steam runaway gave its few
/// bar of CO₂ an optical depth of a thousand and warmed it by 170 to
/// 200 K.
fn broadening_bar(gases: &Gases) -> f64 {
    let co2 = gases.co2.max(0.0);
    let h2o = gases.h2o.max(0.0);
    let others = (gases.total() - co2 - h2o).max(0.0);
    let strong = co2 + h2o.min(STEAM_MAX_BAR);
    let foreign = if others <= BROADENING_KNEE_BAR {
        others
    } else {
        BROADENING_KNEE_BAR * (others / BROADENING_KNEE_BAR).powf(THICK_AIR_EXPONENT)
    };
    strong + foreign
}

/// The Simpson–Nakajima limit, W/m²: the outgoing longwave radiation of
/// a thick steam atmosphere, whose emission comes from a level pinned to
/// water's saturation curve and so no longer depends on the surface
/// temperature. Goldblatt, Robinson, Zahnle & Crisp 2013 (*Nature
/// Geosci.* 6, 661, doi:10.1038/ngeo1892; abstract checked) find 282 W/m²
/// line by line; Boukrouche, Lichtenberg & Pierrehumbert 2021 (*ApJ* 919,
/// 130, doi:10.3847/1538-4357/ac1345; §III.1 read in arXiv 2107.14150)
/// 283 W/m², and Hamano et al. 2015 (*ApJ* 806, 216,
/// doi:10.1088/0004-637X/806/2/216; read in arXiv 1505.03552) 280 W/m².
const SIMPSON_NAKAJIMA_W_M2: f64 = 282.0;

/// Surface temperature, K, at which the radiation limit of a thick steam
/// atmosphere ends. Boukrouche et al. 2021 (§III.1) find the outgoing
/// radiation flat between 300 and 1800 K under 260 bar of steam (1050 K
/// under 10 bar), the end of the plateau reaching a constant of about
/// 1800 K as the steam thickens to 295 bar; past it the dry adiabat
/// reaches the emitting levels and the radiation rises again. Goldblatt
/// 2015 (*Astrobiology* 15, 362, doi:10.1089/ast.2014.1268; read in arXiv
/// 1503.04835) puts the rise past 1500 K, through the 4 µm window.
const STEAM_BREAKDOWN_K: f64 = 1800.0;

/// Largest grey optical depth of water vapour: the one at which a steam
/// atmosphere alone sends out [`SIMPSON_NAKAJIMA_W_M2`] from a surface at
/// [`STEAM_BREAKDOWN_K`], `(4/3) (σ T_b⁴ / F_SN − 1)`, about 2800. Pure
/// steam thick enough to reach it then sends out `F_SN (T / T_b)⁴` from a
/// surface at `T`, whatever its column: no more than the limit up to the
/// breakdown, rising past it as from a fixed emitting column (slower than
/// the near-exponential rise Boukrouche et al. find, so an upper
/// estimate), and a steam runaway absorbing more than the limit settles a
/// little past 1800 K. The grey law alone (`k p` for pure steam) passes this depth
/// past about 480 bar of steam, and put the 1.6e5 bar of a 30 %-water
/// Earth at 0.7 AU at 8800 K (issue #438). Under that, the grey steam is
/// left as it was: thinner than this cap, it emits more than the limit
/// at the breakdown, and its hot state is cooler than Boukrouche et al.
/// find (with 10 bar of steam the grey surface sends out the limit at
/// about 680 K, not 1050 K).
const STEAM_MAX_TAU: f64 = 4.0 / 3.0
    * (SIGMA * STEAM_BREAKDOWN_K * STEAM_BREAKDOWN_K * STEAM_BREAKDOWN_K * STEAM_BREAKDOWN_K
        / SIMPSON_NAKAJIMA_W_M2
        - 1.0);

/// Steam pressure, bar, at which pure steam reaches [`STEAM_MAX_TAU`]
/// (`k p`, about 480 bar): the steam column the outgoing radiation of a
/// steam atmosphere sees (see [`broadening_bar`]).
const STEAM_MAX_BAR: f64 = STEAM_MAX_TAU / H2O_OPACITY;

/// Grey infrared optical depth of the air: each absorber's
/// `k_i (p_i / 1 bar)^a_i (P_b / 1 bar)^½`, the last factor the pressure
/// broadening of its lines (see [`LINE_EXPONENT`]). For CO₂ and water
/// vapour `P_b` is [`broadening_bar`]; for methane it is the whole
/// pressure, since its opacity stands for Titan's collision-induced
/// greenhouse (see [`CH4_OPACITY`]). The water's share is capped at
/// [`STEAM_MAX_TAU`], the Simpson–Nakajima limit of a steam atmosphere.
/// Pressures in bar.
fn optical_depth(gases: &Gases) -> f64 {
    let total = gases.total().max(0.0);
    let line = |opacity: f64, partial: f64, exponent: f64, broadening: f64| {
        opacity * partial.max(0.0).powf(exponent) * broadening.powf(LINE_EXPONENT)
    };
    let broadening = broadening_bar(gases);
    let water = line(H2O_OPACITY, gases.h2o, LINE_EXPONENT, broadening);
    line(CO2_OPACITY, gases.co2, CO2_EXPONENT, broadening)
        + water.min(STEAM_MAX_TAU)
        + line(CH4_OPACITY, gases.ch4, LINE_EXPONENT, total)
}

/// Mean surface temperature, K, of a body with `gases` (bar) under its
/// star's light, plus its internal `heat_flux_w_m2`.
///
/// A grey atmosphere in radiative equilibrium (the Eddington
/// approximation, Pierrehumbert 2010, *Principles of Planetary Climate*,
/// §4.6): `T⁴ = T_eq⁴ (1 + ¾ τ) + q/σ`, with `T_eq` the context's
/// equilibrium temperature (Bond albedo 0.3) and `τ` the infrared optical
/// depth of `optical_depth`, whose opacities are calibrated on Venus,
/// Earth and Titan and whose steam is capped at the Simpson–Nakajima
/// limit ([`STEAM_MAX_TAU`]): a thick steam runaway settles a little past
/// 1800 K. Without air, the surface sits at `T_eq` warmed by its internal
/// heat alone.
pub fn surface_temperature_k(context: &Context, gases: &Gases, heat_flux_w_m2: f64) -> f64 {
    let tau = optical_depth(gases);
    let t_eq4 = context.equilibrium_k.max(0.0).powi(4);
    (t_eq4 * (1.0 + 0.75 * tau) + heat_flux_w_m2.max(0.0) / SIGMA).powf(0.25)
}

/// Temperature, K, at which the carbonate thermostat holds a planet with
/// plates and liquid water: weathering draws CO₂ down faster when warm,
/// and volcanoes give it back (Walker, Hays & Kasting 1981,
/// doi:10.1029/JC086iC10p09776). Calibration, no source: Earth's mean
/// surface temperature, 288 K.
pub const THERMOSTAT_K: f64 = 288.0;

/// Relative humidity of the air above an ocean or ice: the fixed 77 % of
/// Manabe & Wetherald 1967 (doi:10.1175/1520-0469(1967)024<0241:TEOTAW>2.0.CO;2,
/// their surface value; from memory, not checked).
const RELATIVE_HUMIDITY: f64 = 0.77;

/// Triple point of water, K and bar (IAPWS: 273.16 K, 611.657 Pa).
const WATER_TRIPLE_K: f64 = 273.16;
const WATER_TRIPLE_BAR: f64 = 0.006_116_57;

/// Melting point of ice, K (273.15 K at 1 atm; it barely moves with
/// pressure).
const ICE_MELTS_K: f64 = 273.15;

/// Critical point of water, K (IAPWS-95: 647.096 K, 220.64 bar): above
/// it no liquid exists.
const WATER_CRITICAL_K: f64 = 647.096;

/// Latent heat of vaporisation of water over its gas constant
/// (`L / R_v`), K: `L` 2.4e6 J/kg, between its 2.50e6 at 0 °C and 2.26e6
/// at 100 °C, over `R_v` 461.5 J/(kg·K). This mean puts the boiling point
/// at 1 atm at 373 K and the saturation at 288 K within 5 % of the
/// tables.
const WATER_L_OVER_R: f64 = 2.4e6 / 461.5;

/// Saturation vapour pressure of water at `temperature_k`, bar:
/// Clausius–Clapeyron from the triple point with a constant latent heat
/// (Pierrehumbert 2010, §2.1), capped at the critical point. Below
/// 273 K it stands in for the vapour over ice.
fn water_saturation_bar(temperature_k: f64) -> f64 {
    let t = temperature_k.clamp(1.0, WATER_CRITICAL_K);
    WATER_TRIPLE_BAR * (WATER_L_OVER_R * (1.0 / WATER_TRIPLE_K - 1.0 / t)).exp()
}

/// Boiling point of water under `pressure_bar`, K: the same
/// Clausius–Clapeyron inverted, capped at the critical point.
fn water_boils_k(pressure_bar: f64) -> f64 {
    let p = pressure_bar.max(WATER_TRIPLE_BAR);
    let inverse = 1.0 / WATER_TRIPLE_K - (p / WATER_TRIPLE_BAR).ln() / WATER_L_OVER_R;
    if inverse <= 1.0 / WATER_CRITICAL_K {
        WATER_CRITICAL_K
    } else {
        1.0 / inverse
    }
}

/// Triple point of methane, K: where pure methane freezes (NIST
/// Chemistry WebBook, 90.67 ± 0.03 K, average of 25 values).
const METHANE_TRIPLE_K: f64 = 90.67;

/// Enthalpy of fusion of methane over the gas constant, K: 0.9392 kJ/mol
/// at 90.67 K (Vogt & Pitzer 1976, as the NIST Chemistry WebBook gives
/// it) over 8.314 J/(mol·K).
const METHANE_FUSION_L_OVER_R: f64 = 939.2 / 8.314;

/// Vapour pressure of nitrogen, bar, at `temperature_k`: the Antoine
/// equation `log10 p = A − B / (T + C)` with the NIST Chemistry WebBook's
/// coefficients from Edejer & Thodos 1967 (A 3.7362, B 264.651,
/// C −6.788), fitted from 63.14 to 126 K.
fn nitrogen_vapour_bar(temperature_k: f64) -> f64 {
    10f64.powf(3.7362 - 264.651 / (temperature_k - 6.788))
}

/// Share of nitrogen dissolved in a methane liquid under nitrogen, as a
/// share of what Raoult's law gives (`p_N₂ / p_sat,N₂(T)`): Thompson,
/// Zollweg & Gabis 1992 (doi:10.1016/0019-1035(92)90127-S) find from
/// their N₂ + CH₄ vapour–liquid equilibrium model that "the N₂ content of
/// condensate is ∼1/2 of that computed from Raoult's law", 16 to 30 % of
/// Titan's cloud droplets.
const NITROGEN_RAOULT_SHARE: f64 = 0.5;

/// Most nitrogen in the methane liquid, mole fraction: the top of the
/// 16 to 30 % Thompson, Zollweg & Gabis 1992 find in Titan's cloud
/// droplets; beyond it the rule below is not checked.
const MAX_NITROGEN_IN_LIQUID: f64 = 0.3;

/// The temperature, K, under which the surface liquid of a body whose air
/// holds `n2_bar` of nitrogen freezes: the methane liquidus lowered by the
/// nitrogen dissolved in it; pure methane's triple point without
/// nitrogen.
///
/// A simple fit, labelled so. The liquid's nitrogen mole fraction `x` is
/// half of Raoult's law at the surface's N₂ partial pressure and pure
/// methane's triple point ([`NITROGEN_RAOULT_SHARE`], Thompson, Zollweg &
/// Gabis 1992), at most [`MAX_NITROGEN_IN_LIQUID`]; the methane liquidus
/// is an ideal solution's (the Schröder–van Laar equation,
/// `ln(1 − x) = −(L_fus / R)(1/T − 1/T_m)`, with methane's enthalpy of
/// fusion). Titan's 1.45 bar of N₂ gives `x` = 0.19 and a liquid down to
/// 77.6 K (its surface: 93.7 K); the cap stops the fall at 70.4 K.
/// Richardson, Hartwig & Leachman 2019
/// (doi:10.1016/j.ijthermalsci.2018.12.024) measured liquid
/// methane–ethane–nitrogen mixtures freezing at 71.5 K (13.4 % N₂, 0.29
/// bar) and 74.0 K (7.9 % N₂, 0.52 bar), "more than 15 K below" pure
/// methane's triple point; for those nitrogen shares this binary rule,
/// without the ethane, gives 81.3 and 85.0 K: it errs towards freezing.
pub fn methane_freezes_k(n2_bar: f64) -> f64 {
    let x = (NITROGEN_RAOULT_SHARE * n2_bar.max(0.0) / nitrogen_vapour_bar(METHANE_TRIPLE_K))
        .min(MAX_NITROGEN_IN_LIQUID);
    1.0 / (1.0 / METHANE_TRIPLE_K - (1.0 - x).ln() / METHANE_FUSION_L_OVER_R)
}

/// Normal boiling point of ethane, K, at 1 atm (NIST Chemistry WebBook,
/// 184.6 K; from memory, not checked).
const ETHANE_BOILS_K: f64 = 184.6;

/// Enthalpy of vaporisation of ethane at its boiling point over the gas
/// constant, K: 14.7 kJ/mol over 8.314 J/(mol·K) (NIST Chemistry
/// WebBook; from memory, not checked).
const ETHANE_L_OVER_R: f64 = 14_700.0 / 8.314;

/// Critical temperature of ethane, K (305.3 K, NIST; from memory, not
/// checked).
const ETHANE_CRITICAL_K: f64 = 305.3;

/// One standard atmosphere, bar.
const ATM_BAR: f64 = 1.013_25;

/// Boiling point of ethane under `pressure_bar`, K: Clausius–Clapeyron
/// from its normal boiling point, capped at its critical point.
fn ethane_boils_k(pressure_bar: f64) -> f64 {
    let p = pressure_bar.max(1e-12);
    let inverse = 1.0 / ETHANE_BOILS_K - (p / ATM_BAR).ln() / ETHANE_L_OVER_R;
    if inverse <= 1.0 / ETHANE_CRITICAL_K {
        ETHANE_CRITICAL_K
    } else {
        1.0 / inverse
    }
}

/// Temperature, K, above which a rocky surface is molten: the basalt
/// solidus, about 1400 K, the threshold of Kite, Fegley, Schaefer &
/// Gaidos 2016 (doi:10.3847/0004-637X/828/2/80) for a magma surface on
/// hot rocky exoplanets (value from the plan, not checked against the
/// paper).
pub const LAVA_K: f64 = 1400.0;

/// The liquid at a surface at `temperature_k` under `gases`, from the
/// phase diagrams: lava above [`LAVA_K`], on a body that has kept no
/// water (with water on top, its steam or supercritical fluid covers the
/// rock: no lava, no liquid, Hamano, Abe & Genda 2013,
/// doi:10.1038/nature12163, for the steam over a magma ocean; not
/// checked); water between the melting of ice and its boiling point at
/// the surface pressure, above its triple point pressure, when water is
/// kept out of a runaway; hydrocarbons between the freezing point of
/// methane with the nitrogen of the air dissolved in it
/// ([`methane_freezes_k`]) and ethane's boiling point, with methane in
/// the air; none otherwise.
fn liquid(temperature_k: f64, gases: &Gases, water: &Water) -> SurfaceLiquid {
    let pressure = gases.total();
    let water_on_top = water.kept_kg > 0.0;
    if temperature_k >= LAVA_K && !water_on_top {
        SurfaceLiquid::Lava
    } else if water_on_top
        && !water.runaway
        && pressure >= WATER_TRIPLE_BAR
        && temperature_k >= ICE_MELTS_K
        && temperature_k < water_boils_k(pressure)
    {
        SurfaceLiquid::Water
    } else if gases.ch4 > 0.0
        && temperature_k >= methane_freezes_k(gases.n2)
        && temperature_k < ethane_boils_k(pressure)
    {
        SurfaceLiquid::Hydrocarbons
    } else {
        SurfaceLiquid::None
    }
}

/// Whether the climate of a body that keeps its water out of a runaway
/// has carried its surface past water's critical point: its vapour
/// feedback has run away where the insolation limit of
/// [`super::regime::water`] did not see it (the moist runaway of a wet
/// planet under a thick air; Kasting 1988, doi:10.1016/0019-1035(88)90116-9,
/// not checked). Its ocean is then steam: the chain derives the body
/// again with [`super::regime::steam`], and keeps that runaway only if
/// the steam is still [`past_critical`]. The vapour that carries it there
/// is bounded by the water the body has (see [`climate`]), so a trace of
/// water cannot boil.
pub fn boils(water: &Water, climate: &Climate) -> bool {
    !water.runaway && water.kept_kg > 0.0 && past_critical(climate)
}

/// Whether `climate` holds its surface at or past water's critical point
/// (647.096 K), where no liquid water exists: the test the chain puts to
/// a boiled body's steam (see [`boils`]).
pub fn past_critical(climate: &Climate) -> bool {
    climate.temperature_k >= WATER_CRITICAL_K
}

/// `gases` with the water vapour an ocean or ice keeps at
/// `temperature_k`: [`RELATIVE_HUMIDITY`] of saturation, but never more
/// than the `water_bar` the body's whole water would weigh as vapour (a
/// trace of water cannot fill the air with a hundred bar of steam).
fn with_vapour(gases: Gases, temperature_k: f64, water_bar: f64) -> Gases {
    Gases {
        h2o: (RELATIVE_HUMIDITY * water_saturation_bar(temperature_k)).min(water_bar.max(0.0)),
        ..gases
    }
}

/// Iterations of the water-vapour feedback and of the thermostat's
/// bisection: enough for 1e-3 K and a CO₂ to 2⁻⁶⁰ of its range.
const ITERATIONS: usize = 60;

/// Temperature, K, and gases of a surface whose water vapour follows its
/// temperature. `T ← T(gases with the vapour at T)` from the dry
/// temperature: the map rises with `T`, so the iteration climbs to the
/// coldest balance (the one a cooling planet reaches); the vapour is
/// capped at water's critical point and at the body's `water_bar` (see
/// [`with_vapour`]), so it stays bounded.
fn wet_climate(
    context: &Context,
    gases: Gases,
    heat_flux_w_m2: f64,
    water_bar: f64,
) -> (Gases, f64) {
    let dry = Gases { h2o: 0.0, ..gases };
    let mut t = surface_temperature_k(context, &dry, heat_flux_w_m2);
    for _ in 0..ITERATIONS * 4 {
        let next =
            surface_temperature_k(context, &with_vapour(gases, t, water_bar), heat_flux_w_m2);
        let done = (next - t).abs() < 1e-3;
        t = next;
        if done {
            break;
        }
    }
    (with_vapour(gases, t, water_bar), t)
}

/// The climate of a body and its gases once the climate has set its CO₂
/// and water vapour.
///
/// - In a runaway greenhouse, or without water, the gases are as given
///   (a runaway's steam is already in them) and the temperature is
///   [`surface_temperature_k`]'s.
/// - With water kept, its vapour follows the temperature (77 % of
///   saturation, never more than the `water_bar` all of the body's water
///   would weigh as vapour), solved together with it.
/// - With plates as well, the carbonate thermostat (Walker, Hays & Kasting
///   1981, doi:10.1029/JC086iC10p09776) sets the CO₂ by bisection between
///   none and all that is outgassed (`gases.co2`, never more) so that the
///   temperature is [`THERMOSTAT_K`], the vapour taken at that
///   temperature. If all the CO₂ is not enough, it stays in the air and
///   the planet is colder (weathering stops on a frozen planet, so the
///   CO₂ accumulates); if none is still too warm, the planet is hotter.
///   Only a temperature reached is `regulated`.
///
/// The liquid follows from the temperature and pressure (see
/// [`LAVA_K`] and the phase diagrams of water and hydrocarbons).
pub fn climate(
    context: &Context,
    gases: Gases,
    water: &Water,
    regime: Regime,
    heat_flux_w_m2: f64,
    water_bar: f64,
) -> (Gases, Climate) {
    let wet = !water.runaway && water.kept_kg > 0.0;
    let (gases, temperature_k, regulated) = if !wet {
        let t = surface_temperature_k(context, &gases, heat_flux_w_m2);
        (gases, t, false)
    } else if regime == Regime::Plates {
        thermostat(context, gases, heat_flux_w_m2, water_bar)
    } else {
        let (g, t) = wet_climate(context, gases, heat_flux_w_m2, water_bar);
        (g, t, false)
    };
    let liquid = liquid(temperature_k, &gases, water);
    (
        gases,
        Climate {
            temperature_k,
            liquid,
            regulated,
        },
    )
}

/// The carbonate thermostat: the CO₂ in `[0, gases.co2]` that puts the
/// surface at [`THERMOSTAT_K`], by bisection (see [`climate`]). Returns
/// the gases, the temperature and whether it was reached.
fn thermostat(
    context: &Context,
    gases: Gases,
    heat_flux_w_m2: f64,
    water_bar: f64,
) -> (Gases, f64, bool) {
    let outgassed = gases.co2.max(0.0);
    let at_target = with_vapour(gases, THERMOSTAT_K, water_bar);
    let temperature =
        |co2: f64| surface_temperature_k(context, &Gases { co2, ..at_target }, heat_flux_w_m2);
    if temperature(outgassed) < THERMOSTAT_K {
        let (g, t) = wet_climate(
            context,
            Gases {
                co2: outgassed,
                ..gases
            },
            heat_flux_w_m2,
            water_bar,
        );
        return (g, t, false);
    }
    if temperature(0.0) > THERMOSTAT_K {
        let (g, t) = wet_climate(
            context,
            Gases { co2: 0.0, ..gases },
            heat_flux_w_m2,
            water_bar,
        );
        return (g, t, false);
    }
    let (mut low, mut high) = (0.0, outgassed);
    for _ in 0..ITERATIONS {
        let mid = 0.5 * (low + high);
        if temperature(mid) < THERMOSTAT_K {
            low = mid;
        } else {
            high = mid;
        }
    }
    let regulated = Gases {
        co2: high,
        ..at_target
    };
    (regulated, temperature(high), true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet::PlanetInputs;
    use crate::planet::reference::{Derived, derive_at, model_inputs, reference};

    /// The climate of `inputs` at `orbit_au`, with the chain's gases.
    fn climate_at(inputs: &PlanetInputs, orbit_au: f64) -> (Derived, Gases, Climate) {
        let d = derive_at(inputs, orbit_au);
        let (g, c) = climate(
            &d.context,
            d.gases,
            &d.water,
            d.regime,
            d.heat_flux_w_m2,
            d.water_bar,
        );
        (d, g, c)
    }

    /// The climate of the reference body `name`.
    fn climate_of(name: &str) -> (Derived, Gases, Climate) {
        let b = reference(name);
        climate_at(&model_inputs(b), b.orbit_au)
    }

    #[test]
    fn the_solar_system_has_its_climates() {
        let (_, g, earth) = climate_of("Earth");
        assert!(
            (282.0..=294.0).contains(&earth.temperature_k),
            "{earth:?} {g:?}"
        );
        assert_eq!(earth.liquid, SurfaceLiquid::Water, "{earth:?}");
        assert!(earth.regulated, "{earth:?}");
        // The thermostat leaves a trace of CO₂, not tens of bars.
        assert!(g.co2 < 0.01, "{g:?}");

        let (_, g, venus) = climate_of("Venus");
        assert!(
            (687.0..=787.0).contains(&venus.temperature_k),
            "{venus:?} {g:?}"
        );
        assert_eq!(venus.liquid, SurfaceLiquid::None, "{venus:?}");

        let (_, g, mars) = climate_of("Mars");
        assert!(
            (198.0..=222.0).contains(&mars.temperature_k),
            "{mars:?} {g:?}"
        );
        assert_eq!(mars.liquid, SurfaceLiquid::None, "{mars:?}");

        let (_, g, titan) = climate_of("Titan");
        assert!(
            (86.0..=102.0).contains(&titan.temperature_k),
            "{titan:?} {g:?}"
        );
        assert_eq!(titan.liquid, SurfaceLiquid::Hydrocarbons, "{titan:?}");
    }

    #[test]
    fn nitrogen_keeps_methane_liquid_below_its_triple_point() {
        // No nitrogen: pure methane.
        assert_eq!(methane_freezes_k(0.0), METHANE_TRIPLE_K);
        // Titan's 1.45 bar of N₂: about a fifth of the liquid, down to
        // about 78 K.
        let titan = methane_freezes_k(1.45);
        assert!((76.0..80.0).contains(&titan), "{titan}");
        // More nitrogen, colder liquid, down to the cap.
        assert!(methane_freezes_k(0.5) > titan);
        assert!(methane_freezes_k(10.0) < titan);
        assert!((methane_freezes_k(100.0) - 70.4).abs() < 0.2);
        // A Titan at 89 K under 1 bar of N₂ keeps its lakes.
        let gases = Gases {
            n2: 1.0,
            ch4: 0.03,
            ..Gases::default()
        };
        let dry = Water {
            runaway: false,
            boiled: false,
            kept_kg: 0.0,
            oceans: 0.0,
        };
        assert_eq!(liquid(89.0, &gases, &dry), SurfaceLiquid::Hydrocarbons);
        let thin = Gases { n2: 0.0, ..gases };
        assert_eq!(liquid(89.0, &thin, &dry), SurfaceLiquid::None);
    }

    #[test]
    fn an_airless_body_sits_at_its_equilibrium_temperature() {
        let (d, g, mercury) = climate_of("Mercury");
        let t_eq = d.context.equilibrium_k;
        assert!(d.heat_flux_w_m2 < 1.0, "{}", d.heat_flux_w_m2);
        assert!(
            (mercury.temperature_k - t_eq).abs() < 1.0,
            "{mercury:?} {t_eq} {g:?}"
        );
        assert_eq!(mercury.liquid, SurfaceLiquid::None);
        assert!(!mercury.regulated);
    }

    #[test]
    fn the_thermostat_never_exceeds_the_outgassed_co2() {
        let earth = model_inputs(reference("Earth"));
        let (d, g, c) = climate_at(&earth, 1.6);
        assert!(g.co2 <= d.gases.co2, "{g:?} {:?}", d.gases);
        if !c.regulated {
            // Short of CO₂: all of it is in the air, and the water is
            // liquid only while the surface stays above freezing.
            assert_eq!(g.co2, d.gases.co2, "{g:?}");
            let liquid = c.liquid == SurfaceLiquid::Water;
            assert_eq!(liquid, c.temperature_k >= 273.15, "{c:?}");
        }
        // At 2 AU the CO₂ the cold leaves in the air is not enough.
        let (d, g, c) = climate_at(&earth, 2.0);
        assert_eq!(g.co2, d.gases.co2, "{g:?}");
        assert!(!c.regulated, "{c:?}");
        assert_eq!(c.liquid, SurfaceLiquid::None, "{c:?}");
        // With a ten-thousandth of its CO₂, the thermostat runs out: it puts
        // all of it in the air, and the water freezes.
        let d = derive_at(&earth, 1.6);
        let scarce = Gases {
            co2: d.gases.co2 * 1e-4,
            ..d.gases
        };
        let (g, c) = climate(
            &d.context,
            scarce,
            &d.water,
            d.regime,
            d.heat_flux_w_m2,
            d.water_bar,
        );
        assert!(g.co2 <= scarce.co2, "{g:?}");
        assert!(!c.regulated, "{c:?}");
        assert_eq!(c.liquid, SurfaceLiquid::None, "{c:?}");
    }

    #[test]
    fn no_water_no_ocean() {
        let dry = PlanetInputs {
            water_fraction: 0.0,
            ..model_inputs(reference("Earth"))
        };
        let (_, _, c) = climate_at(&dry, 1.0);
        assert_ne!(c.liquid, SurfaceLiquid::Water, "{c:?}");
        assert!(!c.regulated, "{c:?}");
    }

    #[test]
    fn a_close_rock_is_a_lava_world() {
        let (_, _, c) = climate_at(&model_inputs(reference("Earth")), 0.01);
        assert_eq!(c.liquid, SurfaceLiquid::Lava, "{c:?}");
    }

    #[test]
    fn water_on_top_is_never_lava() {
        let air = Gases {
            n2: 10.0,
            h2o: 200.0,
            ..Gases::default()
        };
        for runaway in [false, true] {
            let wet = Water {
                runaway,
                boiled: false,
                kept_kg: 1.4e21,
                oceans: 1.0,
            };
            for t in [700.0, 2000.0, 5000.0] {
                assert_eq!(liquid(t, &air, &wet), SurfaceLiquid::None, "{t} {wet:?}");
            }
        }
        let dry = Water {
            runaway: true,
            boiled: false,
            kept_kg: 0.0,
            oceans: 0.0,
        };
        assert_eq!(liquid(2000.0, &air, &dry), SurfaceLiquid::Lava);
        // A wet surface past the critical point boils; a runaway's does not
        // boil again.
        let hot = Climate {
            temperature_k: 700.0,
            liquid: SurfaceLiquid::None,
            regulated: false,
        };
        let ocean = Water {
            runaway: false,
            boiled: false,
            kept_kg: 1.4e21,
            oceans: 1.0,
        };
        assert!(boils(&ocean, &hot));
        assert!(!boils(
            &Water {
                runaway: true,
                boiled: false,
                ..ocean
            },
            &hot
        ));
        assert!(!boils(
            &ocean,
            &Climate {
                temperature_k: 600.0,
                ..hot
            }
        ));
    }

    #[test]
    fn more_greenhouse_gas_is_warmer() {
        let c = crate::planet::reference::sun_at(1.0);
        let thin = Gases {
            n2: 1.0,
            co2: 1e-3,
            ..Gases::default()
        };
        let thick = Gases { co2: 1.0, ..thin };
        let t_thin = surface_temperature_k(&c, &thin, 0.0);
        let t_thick = surface_temperature_k(&c, &thick, 0.0);
        assert!(
            t_thin > c.equilibrium_k && t_thick > t_thin,
            "{t_thin} {t_thick}"
        );
        // Internal heat warms an airless surface.
        let bare = surface_temperature_k(&c, &Gases::default(), 0.0);
        assert!((bare - c.equilibrium_k).abs() < 1e-9);
        assert!(surface_temperature_k(&c, &Gases::default(), 100.0) > bare);
    }

    /// The model Earth at 1 AU with `n2_bar` of nitrogen in its air and
    /// `water_fraction` of its mass in water, through the whole climate
    /// (the carbonate thermostat included).
    fn earth_with_nitrogen(n2_bar: f64, water_fraction: f64) -> (Gases, Climate) {
        let earth = PlanetInputs {
            water_fraction,
            ..model_inputs(reference("Earth"))
        };
        let d = derive_at(&earth, 1.0);
        let gases = Gases {
            n2: n2_bar,
            ..d.gases
        };
        climate(
            &d.context,
            gases,
            &d.water,
            d.regime,
            d.heat_flux_w_m2,
            d.water_bar,
        )
    }

    #[test]
    fn an_earth_with_two_or_three_bar_of_nitrogen_stays_temperate() {
        // Archean Earth may have held 2 to 3 bar of N₂ (Goldblatt et al.
        // 2009; Charnay et al. 2013): the thermostat still holds it, with
        // Earth's water and with 1 % of its mass in water.
        for water_fraction in [0.000_23, 0.01] {
            for n2 in [2.0, 3.0] {
                let (g, c) = earth_with_nitrogen(n2, water_fraction);
                assert!(c.regulated, "{n2} bar, {water_fraction}: {c:?} {g:?}");
                assert!(
                    (282.0..=294.0).contains(&c.temperature_k),
                    "{n2} bar, {water_fraction}: {c:?} {g:?}"
                );
                assert_eq!(c.liquid, SurfaceLiquid::Water, "{c:?}");
                assert!(g.co2 > 0.0, "{g:?}");
            }
        }
    }

    #[test]
    fn doubling_the_nitrogen_warms_a_late_archean_earth_by_a_few_kelvin() {
        // Charnay et al. 2013 (§5.2): the Sun at 2.5 Ga (Gough 1981,
        // doi:10.1007/BF00151270, 0.82 of today's light), 10 mbar of CO₂, the air from 1 to 2 bar
        // of N₂: +7 K in their GCM, +4 to 5 K in the 1-D model of
        // Goldblatt et al. 2009. Their CO₂ held fixed, no thermostat.
        let d = derive_at(&model_inputs(reference("Earth")), 1.0);
        let faint = Context {
            equilibrium_k: d.context.equilibrium_k * 0.82_f64.powf(0.25),
            ..d.context
        };
        let air = |n2: f64| Gases {
            n2,
            co2: 0.010,
            h2o: 0.0,
            ch4: 0.0,
            nh3: 0.0,
            so2: 0.0,
            o2: 0.0,
        };
        let temperature = |gases: Gases| {
            let (g, c) = climate(
                &faint,
                gases,
                &d.water,
                Regime::Stagnant,
                d.heat_flux_w_m2,
                d.water_bar,
            );
            (g, c.temperature_k)
        };
        let (g1, t1) = temperature(air(0.99));
        let (g2, t2) = temperature(air(1.99));
        assert!((270.0..=290.0).contains(&t1), "{t1} {g1:?}");
        let warming = t2 - t1;
        assert!((3.0..=8.0).contains(&warming), "{t1} -> {t2}: {g2:?}");
    }

    #[test]
    fn a_pure_gas_broadens_its_own_lines_in_full() {
        // Venus's CO₂ and a steam atmosphere: the gas is its own air, so
        // the strong-line law holds whole.
        let venus = Gases {
            co2: 85.0,
            ..Gases::default()
        };
        let expected = CO2_OPACITY * 85.0_f64.powf(CO2_EXPONENT) * 85.0_f64.sqrt();
        assert!((optical_depth(&venus) / expected - 1.0).abs() < 1e-12);
        let steam = Gases {
            h2o: 260.0,
            ..Gases::default()
        };
        let expected = H2O_OPACITY * 260.0;
        assert!((optical_depth(&steam) / expected - 1.0).abs() < 1e-12);
        // Below 1 bar of other gases, nothing changes from the plain
        // square root of the whole pressure.
        let earth = Gases {
            n2: 0.9,
            co2: 3e-4,
            h2o: 0.0126,
            ..Gases::default()
        };
        let p = earth.total();
        let expected = p.sqrt()
            * (CO2_OPACITY * 3e-4_f64.powf(CO2_EXPONENT) + H2O_OPACITY * 0.0126_f64.sqrt());
        assert!((optical_depth(&earth) / expected - 1.0).abs() < 1e-12);
    }

    #[test]
    fn co2_and_water_broaden_each_other_in_full() {
        // CO₂ broadens water vapour's lines at least as well as N₂ does,
        // and water CO₂'s (Yang et al. 2022; Tan et al. 2019): only the
        // other gases are tempered past the knee. An Earth at 1.6 AU, all
        // its 1.8 bar of CO₂ in the air over 0.89 bar of N₂, keeps the
        // plain square root of its whole pressure.
        let (_, g, c) = climate_at(&model_inputs(reference("Earth")), 1.6);
        assert!(g.co2 > 1.0 && g.n2 < BROADENING_KNEE_BAR, "{g:?}");
        assert!((285.0..=290.0).contains(&c.temperature_k), "{c:?} {g:?}");
        assert_eq!(c.liquid, SurfaceLiquid::Water, "{c:?}");
        let thick = Gases {
            n2: 0.89,
            co2: 1.8,
            h2o: 0.01,
            ..Gases::default()
        };
        let p = thick.total();
        let expected =
            p.sqrt() * (CO2_OPACITY * 1.8_f64.powf(CO2_EXPONENT) + H2O_OPACITY * 0.01_f64.sqrt());
        assert!((optical_depth(&thick) / expected - 1.0).abs() < 1e-12);
        // Past the knee only the nitrogen is tempered: 3 bar of N₂ count
        // as 3^0.2 bar, the CO₂ and the vapour in full for both.
        let nitrogen = Gases { n2: 3.0, ..thick };
        let pb = 1.8 + 0.01 + 3.0_f64.powf(THICK_AIR_EXPONENT);
        let expected =
            pb.sqrt() * (CO2_OPACITY * 1.8_f64.powf(CO2_EXPONENT) + H2O_OPACITY * 0.01_f64.sqrt());
        assert!((optical_depth(&nitrogen) / expected - 1.0).abs() < 1e-12);
        // Steam deeper than the column the Simpson-Nakajima cap sees no
        // longer broadens the CO₂ either: a steam runaway's few bar of CO₂
        // keep the same opacity under 1.6e5 bar of steam as under 1e3.
        let runaway = Gases {
            h2o: 1.6e5,
            co2: 6.0,
            ..Gases::default()
        };
        let thinner = Gases {
            h2o: 1.0e3,
            ..runaway
        };
        const { assert!(STEAM_MAX_BAR < 1.0e3) };
        assert_eq!(optical_depth(&runaway), optical_depth(&thinner));
    }

    /// A 1 M⊕ planet with `water_fraction` of its mass in water and the
    /// volatiles `draw_inputs` gives its rock and ice (no scatter), round
    /// the Sun at `orbit_au`, through the whole chain.
    fn water_world_at(water_fraction: f64, orbit_au: f64) -> crate::planet::Surface {
        let inputs = PlanetInputs {
            water_fraction,
            volatiles: crate::planet::inputs::volatile_rule(1.0 - water_fraction, water_fraction),
            ..crate::planet::reference::inputs(1.0, 0.325, 0.0)
        };
        let model =
            crate::planet::planet_model(&crate::planet::reference::sun_at(orbit_au), inputs);
        model.surface.expect("a bare body has a surface")
    }

    #[test]
    fn a_steam_runaway_settles_past_the_end_of_the_radiation_limit() {
        // Issue #438: 30 % water (some 1300 oceans) at 0.7 AU, over the
        // runaway limit, puts about 1.6e5 bar of steam in the air. A thick
        // steam atmosphere radiates the Simpson-Nakajima limit until its
        // surface reaches about 1800 K, and more beyond (Boukrouche et al.
        // 2021): absorbing 1.7 times the limit (489 against 282 W/m²), it
        // settles a little past 1800 K, not at thousands of kelvin.
        let surface = water_world_at(0.3, 0.7);
        assert!(surface.water.runaway, "{surface:?}");
        assert!(surface.gases.h2o > 1.0e5, "{:?}", surface.gases);
        let t = surface.climate.temperature_k;
        assert!((1800.0..=2500.0).contains(&t), "{t} K: {surface:?}");
        assert_eq!(surface.climate.liquid, SurfaceLiquid::None, "{surface:?}");
        // Past the breakdown the steam column no longer matters: with a
        // third of the water (and more CO₂ left in the air), nearly the
        // same hot state.
        let less = water_world_at(0.1, 0.7);
        assert!(less.water.runaway && less.gases.h2o > 1.0e4, "{less:?}");
        let dt = (less.climate.temperature_k - t).abs();
        assert!(dt < 20.0, "{t} K and {} K", less.climate.temperature_k);
    }

    #[test]
    fn thick_steam_sends_out_the_simpson_nakajima_limit_at_the_breakdown() {
        let olr = |t: f64, gases: &Gases| SIGMA * t.powi(4) / (1.0 + 0.75 * optical_depth(gases));
        for h2o in [1.0e3, 1.0e4, 1.6e5] {
            let steam = Gases {
                h2o,
                ..Gases::default()
            };
            let at_breakdown = olr(STEAM_BREAKDOWN_K, &steam);
            assert!(
                (at_breakdown - SIMPSON_NAKAJIMA_W_M2).abs() < 1e-6,
                "{h2o} bar: {at_breakdown} W/m²"
            );
            assert!(olr(1500.0, &steam) < SIMPSON_NAKAJIMA_W_M2, "{h2o} bar");
            assert!(olr(2000.0, &steam) > SIMPSON_NAKAJIMA_W_M2, "{h2o} bar");
        }
        // Under about 480 bar the grey law is under the cap and holds.
        let steam = Gases {
            h2o: 400.0,
            ..Gases::default()
        };
        assert!((optical_depth(&steam) / (H2O_OPACITY * 400.0) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn surface_liquid_is_kebab_case() {
        let s = serde_json::to_string(&SurfaceLiquid::Hydrocarbons).unwrap();
        assert_eq!(s, "\"hydrocarbons\"");
    }
}
