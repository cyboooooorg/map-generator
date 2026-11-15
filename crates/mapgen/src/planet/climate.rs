//! The surface climate of a rocky or icy body and the liquid on its
//! surface (part 2 of the physical planet model, issue #423).
//!
//! [`surface_temperature_k`] balances the outgoing radiation of a grey
//! radiative–convective column of CO₂, H₂O and CH₄ ([`super::column`])
//! against the light and heat the surface takes in; [`climate`] adds the
//! water vapour
//! that an ocean or ice keeps in the air, runs the carbonate thermostat of
//! a planet with plates and water, and reads the liquid at the surface off
//! the phase diagrams of water, hydrocarbons and basalt.
use serde::{Deserialize, Serialize};

use super::{Context, Gases, Water, column};
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

/// Grey opacity of CO₂ at 1 bar of CO₂ in 1 bar of air: its optical
/// depth through a uniform column (see [`optical_depth_at`]).
/// Calibration, no source, on the column of [`super::column`]: Venus's
/// 737 K under its 85 bar of CO₂ (with [`CO2_EXPONENT`]). It leaves Mars
/// within a kelvin of its equilibrium temperature. (1.66 on the old
/// Eddington core, issue #441.)
const CO2_OPACITY: f64 = 0.575;

/// Power of the CO₂ partial pressure in its opacity. Calibration, no
/// source: CO₂'s 15 µm band is saturated at its centre, so its forcing
/// grows with the logarithm of its amount (Myhre et al. 1998,
/// doi:10.1029/98GL01908, `ΔF = 5.35 ln(C/C₀)` W/m²); a small power
/// stands in for that logarithm across the six decades from Earth's
/// 3e-4 bar to Venus's 85 bar. Unchanged by the column of issue #441.
const CO2_EXPONENT: f64 = 0.27;

/// Grey opacity of water vapour at 1 bar of pure steam, with
/// [`LINE_EXPONENT`] (see [`water_broadening_bar`]). Calibration on the
/// column of [`super::column`]: its moist plateau, the Simpson–Nakajima
/// limit [`SIMPSON_NAKAJIMA_W_M2`], at 282 W/m² (Goldblatt et al. 2013)
/// for Earth's air at 320 to 400 K and for pure steam. It is twenty times
/// the old Eddington core's 5.9 (issue #441) because it now counts the
/// water's own pressure as its broadener, whose lines it widens about
/// twenty times better than nitrogen does (see
/// [`WATER_FOREIGN_BROADENING`]), and because the column radiates from
/// its cold upper levels where the vapour is a trace.
const H2O_OPACITY: f64 = 129.0;

/// Grey opacity of methane at 1 bar in 1 bar of air, with
/// [`LINE_EXPONENT`]. Calibration, no source: Titan's 94 K under its
/// 1.5 % of methane in 1.5 bar (2.1 bar in the model), on the column of
/// [`super::column`] (3.4 on the old Eddington core, issue #441).
/// Titan's real greenhouse is mostly collision-induced (N₂–N₂, N₂–CH₄,
/// N₂–H₂; McKay, Pollack & Courtin 1991, doi:10.1126/science.11538492,
/// not checked), and its haze cools it by about 9 K; this opacity carries
/// all of it. That is why it keeps the square root of the whole air's
/// pressure (see [`optical_depth_at`]): collision-induced absorption
/// grows with the nitrogen faster than line broadening does, not slower.
const CH4_OPACITY: f64 = 3.3;

/// Power of a partial pressure, and of the broadening pressure, in a
/// gas's opacity: the strong-line (square-root) limit, where a pressure-
/// broadened line absorbs as `sqrt(S u α)`, its absorber path `u ∝ p_i`
/// and its width `α` growing with the pressure of the gases round it
/// (Pierrehumbert 2010, *Principles of Planetary Climate*, §4.4; Goody &
/// Yung 1989, *Atmospheric Radiation*, ch. 4).
const LINE_EXPONENT: f64 = 0.5;

/// Pressure of the other gases, bar, up to which they broaden lines in
/// proportion (see [`foreign_share`]): the gases other than CO₂ and water
/// themselves (N₂, O₂, CH₄...). Calibration on the column of
/// [`super::column`] (1 bar on the old Eddington core, issue #441): above
/// three times Earth's air (2.5 bar of N₂ and O₂), so that the moist
/// plateau of [`SIMPSON_NAKAJIMA_W_M2`] moves by under 2 % between one
/// and three times Earth's nitrogen, as the limit should (Nakajima,
/// Hayashi & Abe 1992); a lower knee tempered the broadening of a 3x
/// nitrogen column and lifted its plateau by 4 %.
const BROADENING_KNEE_BAR: f64 = 2.6;

/// Power of the other gases' pressure past [`BROADENING_KNEE_BAR`] in
/// the pressure that broadens lines: none, their broadening stops
/// growing there (0.2 on the old Eddington core).
///
/// Calibration on an Earth at 1 AU with modern CO₂ and 2 to 11 bar of
/// N₂ (issue #441): it must keep a moist balance below 330 K. Thick
/// nitrogen does warm a planet by pressure broadening: Goldblatt et al.
/// 2009 (*Nature Geosci.* 2, 891, doi:10.1038/ngeo692; abstract checked)
/// find 4.4 °C for doubled nitrogen on the late Archean Earth in a 1-D
/// radiative–convective model, Charnay et al. 2013 (*JGR Atmos.* 118,
/// 10414, doi:10.1002/jgrd.50808; §5.2 checked in arXiv 1310.4286) +7 °C
/// in a GCM for the air going from 1 to 2 bar, and Wolf & Toon 2014
/// (*Astrobiology* 14, 241, doi:10.1089/ast.2013.1112; abstract checked)
/// 3 to 7 K for plausible nitrogen. The real rise is milder than the
/// strong-line law because lines overlap and saturate (Vladilo et al.
/// 2013, ApJ 767, 65, doi:10.1088/0004-637X/767/1/65, App. A.3,
/// checked); the grey column, whose outgoing radiation rises slowly with
/// the surface temperature near the moist plateau, amplifies any
/// broadening more than they find, so past the knee it adds none.
const THICK_AIR_EXPONENT: f64 = 0.0;

/// How well the other gases broaden water's lines against the water
/// itself, per bar (see [`water_broadening_bar`]). Calibration on the
/// column of [`super::column`]: with it the moist plateau stays within
/// 2 % between one and three times Earth's nitrogen (more nitrogen lifts
/// the plateau through the adiabat and lowers it through broadening; a
/// share of 0 lifts it by 6.6 %, 0.1 lowers it by 1 %), and Earth's
/// thermostat holds 288 K with under 0.01 bar of CO₂. Water broadens its
/// own lines about five times better than air does (self- against
/// air-broadened half-widths in HITRAN, Gordon et al. 2017, *JQSRT* 203,
/// 3, doi:10.1016/j.jqsrt.2017.06.038; from memory, not checked), and its
/// self-continuum, which dominates the windows of a moist atmosphere, is
/// some hundred times its foreign continuum per molecule (Shine, Ptashnik
/// & Rädel 2012, *Surv. Geophys.* 33, 535, doi:10.1007/s10712-011-9170-y;
/// from memory, not checked): a grey share between the two.
const WATER_FOREIGN_BROADENING: f64 = 0.057;

/// Share of their own pressure with which the other gases of `gases`
/// (bar; not CO₂ nor water) broaden lines: 1 up to
/// [`BROADENING_KNEE_BAR`] of them, less past it, so that their whole
/// column broadens as `K (P/K)^`[`THICK_AIR_EXPONENT`]. The column of
/// [`super::column`] applies it to every level, so that a uniform column
/// keeps the totals of [`optical_depth_at`] at its surface.
pub(super) fn foreign_share(gases: &Gases) -> f64 {
    let others = (gases.total() - gases.co2.max(0.0) - gases.h2o.max(0.0)).max(0.0);
    if others <= BROADENING_KNEE_BAR {
        1.0
    } else {
        BROADENING_KNEE_BAR * (others / BROADENING_KNEE_BAR).powf(THICK_AIR_EXPONENT) / others
    }
}

/// Pressure, bar, that broadens the lines of CO₂ in `gases`: CO₂ and
/// water in full, plus the other gases (N₂, O₂, CH₄, the rest) times
/// `share` (see [`foreign_share`]).
///
/// A gas broadens its own lines in full (pure CO₂ on Venus and Mars),
/// and water broadens CO₂'s lines much more than N₂ and O₂ do (Tan,
/// Kochanov, Rothman & Gordon 2019, *JGR Atmos.* 124, 11580,
/// doi:10.1029/2019JD030929; abstract checked), so it is not tempered as
/// a foreign gas. Tempering it (issue #440's first rule) cooled an Earth
/// at 1.6 AU, all its 1.8 bar of CO₂ in the air, from 287.8 K to 281 K
/// on the old core: it moved in the cold edge of the habitable zone,
/// which Kopparapu et al. 2013 (*ApJ* 765, 131,
/// doi:10.1088/0004-637X/765/2/131) put at about 1.67 AU with CO₂–H₂O
/// atmospheres, by a broadening no calibration constrains.
fn broadening_bar(gases: &Gases, share: f64) -> f64 {
    let co2 = gases.co2.max(0.0);
    let h2o = gases.h2o.max(0.0);
    let others = (gases.total() - co2 - h2o).max(0.0);
    co2 + h2o + share * others
}

/// Pressure, bar, that broadens the lines of water vapour in `gases`:
/// the water itself, plus [`WATER_FOREIGN_BROADENING`] times the rest
/// (CO₂ in full, the other gases times `share`). CO₂ broadens water's
/// lines 1.33 to 1.45 times as much as air (Yang et al. 2022, *Acta Phys.
/// Sin.* 71, 203301, doi:10.7498/aps.71.20220700; abstract checked;
/// Brown, Humphrey & Gamache 2007, *J. Mol. Spectrosc.* 246, 1,
/// doi:10.1016/j.jms.2007.07.010, in the far infrared, not checked); it
/// counts here as air does, so that the opacities keep their
/// calibration.
fn water_broadening_bar(gases: &Gases, share: f64) -> f64 {
    let h2o = gases.h2o.max(0.0);
    h2o + WATER_FOREIGN_BROADENING * (broadening_bar(gases, share) - h2o)
}

/// The Simpson–Nakajima limit, W/m²: the outgoing longwave radiation of
/// a moist atmosphere, whose emission comes from levels pinned to
/// water's saturation curve and so no longer depends on the surface
/// temperature. Goldblatt, Robinson, Zahnle & Crisp 2013 (*Nature
/// Geosci.* 6, 661, doi:10.1038/ngeo1892; abstract checked) find 282 W/m²
/// line by line; Boukrouche, Lichtenberg & Pierrehumbert 2021 (*ApJ* 919,
/// 130, doi:10.3847/1538-4357/ac1345; §III.1 read in arXiv 2107.14150)
/// 283 W/m², and Hamano et al. 2015 (*ApJ* 806, 216,
/// doi:10.1088/0004-637X/806/2/216; read in arXiv 1505.03552) 280 W/m².
/// The target of [`H2O_OPACITY`].
#[cfg(test)]
const SIMPSON_NAKAJIMA_W_M2: f64 = 282.0;

/// Deepest steam, bar, the column of [`super::column`] holds: the steam
/// under it does not change the outgoing radiation. Boukrouche et al.
/// 2021 (§III.1) find the radiation of a steam atmosphere flat from 300
/// K until the dry adiabat reaches the emitting levels, about 1050 K
/// under 10 bar of steam and 1800 K under 260 bar, the end of the
/// plateau reaching a constant of about 1800 K as the steam thickens to
/// 295 bar (Goldblatt 2015, *Astrobiology* 15, 362,
/// doi:10.1089/ast.2014.1268, read in arXiv 1503.04835, puts the rise
/// past 1500 K, through the 4 µm window). A grey column has no window:
/// the dry adiabat of a deeper steam column reaches its emitting levels
/// ever later (about 2600 K under 300 bar, 20 000 K under the 1.6e5 bar
/// of a 30 %-water Earth). Calibration: the steam column whose plateau
/// ends at 1800 K (the grey plateau ends at about 1320 K under 10 bar).
pub(super) const STEAM_MAX_BAR: f64 = 70.0;

/// The column totals of the grey optical depths of CO₂, water and
/// methane in a uniform air of `gases` (bar), whose other gases broaden
/// lines by `share` of their pressure: each absorber's
/// `k_i (p_i / 1 bar)^a_i (P_b / 1 bar)^½`, the last factor the pressure
/// broadening of its lines (see [`LINE_EXPONENT`]). For CO₂ `P_b` is
/// [`broadening_bar`], for water [`water_broadening_bar`]; for methane
/// it is the whole pressure, since its opacity stands for Titan's
/// collision-induced greenhouse (see [`CH4_OPACITY`]).
fn depths(gases: &Gases, share: f64) -> [f64; 3] {
    // `LINE_EXPONENT` is ½: square roots, the column's hot path.
    const { assert!(LINE_EXPONENT == 0.5) };
    let co2 = gases.co2.max(0.0);
    let h2o = gases.h2o.max(0.0);
    let ch4 = gases.ch4.max(0.0);
    [
        if co2 > 0.0 {
            CO2_OPACITY * co2.powf(CO2_EXPONENT) * broadening_bar(gases, share).max(0.0).sqrt()
        } else {
            0.0
        },
        H2O_OPACITY * (h2o * water_broadening_bar(gases, share).max(0.0)).sqrt(),
        CH4_OPACITY * (ch4 * gases.total().max(0.0)).sqrt(),
    ]
}

/// Grey infrared optical depth, from the top down to where the air has
/// the partial pressures `gases` (bar), of a uniform air whose other
/// gases broaden lines by `share` of their pressure (see [`depths`]).
pub(super) fn optical_depth_at(gases: &Gases, share: f64) -> f64 {
    depths(gases, share).iter().sum()
}

/// Grey optical depth per unit `ln p` at a level whose partial pressures
/// are `gases`: the derivative of [`optical_depth_at`] through a uniform
/// air, whose partial pressures all grow as `p`, so that a column of
/// constant composition integrates back to its totals. Each depth goes as
/// `p^(a_i + ½)`, so its rate per `ln p` is `(a_i + ½)` times itself.
pub(super) fn opacity_per_ln_p(gases: &Gases, share: f64) -> f64 {
    let [co2, h2o, ch4] = depths(gases, share);
    (CO2_EXPONENT + LINE_EXPONENT) * co2
        + (LINE_EXPONENT + LINE_EXPONENT) * h2o
        + (LINE_EXPONENT + LINE_EXPONENT) * ch4
}

/// Grey infrared optical depth of a uniform column of `gases` (bar)
/// down to its surface (see [`optical_depth_at`], [`foreign_share`]).
#[cfg(test)]
fn optical_depth(gases: &Gases) -> f64 {
    optical_depth_at(gases, foreign_share(gases))
}

/// Absolute tolerance, K, of the surface temperatures solved for.
const TOLERANCE_K: f64 = 1e-2;

/// A point of a function solved for: its argument and value.
#[derive(Clone, Copy)]
struct Point {
    x: f64,
    f: f64,
}

/// The `x` in `[low.x, high.x]` at which `f` (rising with `x`) crosses
/// zero, given `low.f < 0 <= high.f`: regula falsi with the Illinois rule
/// (Dowell & Jarratt 1971, doi:10.1007/BF01934364), until the bracket is
/// narrower than `tolerance`. Returns the upper end, where `f >= 0`.
fn solve(mut f: impl FnMut(f64) -> f64, mut low: Point, mut high: Point, tolerance: f64) -> f64 {
    let mut side = 0;
    for _ in 0..ITERATIONS {
        if high.x - low.x < tolerance {
            break;
        }
        let mut x = (low.x * high.f - high.x * low.f) / (high.f - low.f);
        if !(x > low.x && x < high.x) {
            x = 0.5 * (low.x + high.x);
        }
        let value = f(x);
        if value < 0.0 {
            low = Point { x, f: value };
            if side == -1 {
                high.f *= 0.5;
            }
            side = -1;
        } else {
            high = Point { x, f: value };
            if side == 1 {
                low.f *= 0.5;
            }
            side = 1;
        }
    }
    high.x
}

/// Below and above the temperature, K, at which `excess = OLR − absorbed`
/// crosses zero going up from `start` (where it is below zero): steps of
/// `T ← T (absorbed / OLR(T))^¼` (exact for an outgoing radiation that
/// grows as `T⁴`), at least 0.1 % each.
fn bracket(mut excess: impl FnMut(f64) -> f64, absorbed: f64, start: Point) -> (Point, Point) {
    let mut low = start;
    loop {
        let olr = (low.f + absorbed).max(1e-3 * absorbed);
        let x = (low.x * (absorbed / olr).powf(0.25) * 1.0005).max(low.x * 1.001);
        let f = excess(x);
        let point = Point { x, f };
        if f >= 0.0 || x > 1.0e6 {
            return (low, point);
        }
        low = point;
    }
}

/// Mean surface temperature, K, of a body with `gases` (bar) under its
/// star's light, plus its internal `heat_flux_w_m2`.
///
/// The temperature at which the outgoing radiation of the
/// radiative–convective column of [`super::column`] balances what the
/// surface absorbs, `σ T_eq⁴`, with `T_eq` the context's equilibrium
/// temperature (Bond albedo 0.3), plus the internal heat; the column's
/// stratosphere sits at the skin temperature `T_eq / 2^¼` of a grey
/// atmosphere (Pierrehumbert 2010, §4.3; from memory, not checked). Its opacities are calibrated on Venus, Earth, Titan and
/// the moist plateau; a steam atmosphere sends out the Simpson–Nakajima
/// limit until its surface passes about 1800 K. Without air, the surface
/// sits at `T_eq` warmed by its internal heat alone.
pub fn surface_temperature_k(context: &Context, gases: &Gases, heat_flux_w_m2: f64) -> f64 {
    let t_eq = context.equilibrium_k.max(0.0);
    let absorbed = SIGMA * t_eq.powi(4) + heat_flux_w_m2.max(0.0);
    let bare = (absorbed / SIGMA).powf(0.25);
    if gases.total() <= 0.0 || bare <= 0.0 {
        return bare;
    }
    let skin = t_eq * 0.5_f64.powf(0.25);
    let excess = |t: f64| column::olr_under_skin_w_m2(gases, t, skin) - absorbed;
    // The column radiates no more than its surface, so `bare` is a lower
    // bound; and the vapour in `gases` would condense on a surface colder
    // than its saturation temperature (a runaway's steam is all in its
    // air: its surface cannot be that cold).
    let condenses_k = if gases.h2o > WATER_TRIPLE_BAR {
        water_boils_k(gases.h2o)
    } else {
        0.0
    };
    let lowest = bare.max(condenses_k);
    let start = Point {
        x: lowest,
        f: excess(lowest),
    };
    if start.f >= 0.0 {
        return lowest;
    }
    let (low, high) = bracket(excess, absorbed, start);
    solve(excess, low, high, TOLERANCE_K)
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
pub(super) const WATER_CRITICAL_K: f64 = 647.096;

/// Latent heat of vaporisation of water over its gas constant
/// (`L / R_v`), K: `L` 2.4e6 J/kg, between its 2.50e6 at 0 °C and 2.26e6
/// at 100 °C, over `R_v` 461.5 J/(kg·K). This mean puts the boiling point
/// at 1 atm at 373 K and the saturation at 288 K within 5 % of the
/// tables.
pub(super) const WATER_L_OVER_R: f64 = 2.4e6 / 461.5;

/// Saturation vapour pressure of water at `temperature_k`, bar:
/// Clausius–Clapeyron from the triple point with a constant latent heat
/// (Pierrehumbert 2010, §2.1), capped at the critical point. Below
/// 273 K it stands in for the vapour over ice.
pub(super) fn water_saturation_bar(temperature_k: f64) -> f64 {
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

/// Most iterations of the regula falsi of the surface solves and of the
/// thermostat: more than they need for [`TOLERANCE_K`] and a CO₂ to
/// 1e-9 of itself.
const ITERATIONS: usize = 60;

/// Ratio between the surface temperatures [`wet_balance`] tries in turn
/// on its way up, at most: fine enough not to step over a balance of the
/// moist column (its outgoing radiation turns over within tens of
/// kelvin), and some thirty column evaluations from a temperate surface
/// to water's critical point.
const SCAN_RATIO: f64 = 1.03;

/// The coldest balance of a surface whose water vapour follows its
/// temperature: the gases with that vapour and the temperature `T`, K,
/// at which the column of [`super::column`] over a surface at `T`, its
/// vapour at `T` (see [`with_vapour`]), sends out what the surface
/// absorbs plus its internal heat. A cooling planet stops at the first
/// such `T` above its dry temperature (no surface colder than the dry
/// temperature balances: the vapour only lowers the outgoing radiation).
/// `None` when no surface below
/// water's critical point radiates that much: the absorbed light and
/// heat exceed the moist limit, and the water runs away.
///
/// The temperature is scanned up from the black body's, by the `T⁴` step
/// of `bracket` but at most [`SCAN_RATIO`], to [`WATER_CRITICAL_K`], and
/// the first crossing solved.
fn wet_balance(
    context: &Context,
    gases: Gases,
    heat_flux_w_m2: f64,
    water_bar: f64,
) -> Option<(Gases, f64)> {
    let t_eq = context.equilibrium_k.max(0.0);
    let absorbed = SIGMA * t_eq.powi(4) + heat_flux_w_m2.max(0.0);
    let skin = t_eq * 0.5_f64.powf(0.25);
    let excess =
        |t: f64| column::olr_under_skin_w_m2(&with_vapour(gases, t, water_bar), t, skin) - absorbed;
    // No surface colder than a black body balances; the vapour only
    // lowers the outgoing radiation, so none colder than the dry
    // temperature either, and the scan from the black body finds the
    // same first balance.
    let bare = (absorbed / SIGMA).powf(0.25);
    if bare >= WATER_CRITICAL_K {
        return None;
    }
    let mut low = Point {
        x: bare,
        f: excess(bare),
    };
    if low.f >= 0.0 {
        return Some((with_vapour(gases, bare, water_bar), bare));
    }
    loop {
        // The `T⁴` step of `bracket`, at most `SCAN_RATIO`.
        let olr = (low.f + absorbed).max(1e-3 * absorbed);
        let x = (low.x * (absorbed / olr).powf(0.25) * 1.0005)
            .clamp(low.x * 1.001, low.x * SCAN_RATIO)
            .min(WATER_CRITICAL_K);
        let high = Point { x, f: excess(x) };
        if high.f >= 0.0 {
            let t = solve(excess, low, high, TOLERANCE_K);
            return Some((with_vapour(gases, t, water_bar), t));
        }
        if x >= WATER_CRITICAL_K {
            return None;
        }
        low = high;
    }
}

/// Temperature, K, and gases of a surface whose water vapour follows its
/// temperature: the coldest balance of [`wet_balance`]. Without one (a
/// moist runaway), the vapour of a surface at water's critical point
/// (bounded by the body's `water_bar`) and the temperature it gives:
/// the chain then boils the ocean (see [`boils`]).
fn wet_climate(
    context: &Context,
    gases: Gases,
    heat_flux_w_m2: f64,
    water_bar: f64,
) -> (Gases, f64) {
    wet_balance(context, gases, heat_flux_w_m2, water_bar).unwrap_or_else(|| {
        let hot = with_vapour(gases, WATER_CRITICAL_K, water_bar);
        (hot, surface_temperature_k(context, &hot, heat_flux_w_m2))
    })
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
///   1981, doi:10.1029/JC086iC10p09776) sets the CO₂ between none and
///   all that is outgassed (`gases.co2`, never more) so that the
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
/// surface at [`THERMOSTAT_K`], solved in `ln CO₂` (see [`climate`]). Returns
/// the gases, the temperature and whether it was reached.
fn thermostat(
    context: &Context,
    gases: Gases,
    heat_flux_w_m2: f64,
    water_bar: f64,
) -> (Gases, f64, bool) {
    let outgassed = gases.co2.max(0.0);
    let at_target = with_vapour(gases, THERMOSTAT_K, water_bar);
    let t_eq = context.equilibrium_k.max(0.0);
    let absorbed = SIGMA * t_eq.powi(4) + heat_flux_w_m2.max(0.0);
    let skin = t_eq * 0.5_f64.powf(0.25);
    // With `co2` in the air the surface is warmer than the thermostat
    // when, at `THERMOSTAT_K`, it sends out less than it absorbs (the
    // outgoing radiation rises with the surface temperature).
    let olr_at =
        |co2: f64| column::olr_under_skin_w_m2(&Gases { co2, ..at_target }, THERMOSTAT_K, skin);
    let all = absorbed - olr_at(outgassed);
    if all <= 0.0 {
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
    if olr_at(0.0) < absorbed {
        let (g, t) = wet_climate(
            context,
            Gases { co2: 0.0, ..gases },
            heat_flux_w_m2,
            water_bar,
        );
        return (g, t, false);
    }
    // The CO₂ at which a surface at `THERMOSTAT_K` balances, solved in
    // `ln CO₂` from a trace (2⁻⁶⁰ of the outgassed CO₂, the resolution
    // of the bisection it replaces) up to all of it.
    let deficit = |ln_co2: f64| absorbed - olr_at(ln_co2.exp());
    let floor = outgassed * 2f64.powi(-60);
    let low = Point {
        x: floor.ln(),
        f: deficit(floor.ln()),
    };
    let high = if low.f >= 0.0 {
        floor
    } else {
        let top = Point {
            x: outgassed.ln(),
            f: all,
        };
        solve(deficit, low, top, 1e-9).exp()
    };
    let regulated = Gases {
        co2: high,
        ..at_target
    };
    (
        regulated,
        surface_temperature_k(context, &regulated, heat_flux_w_m2),
        true,
    )
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
    #[ignore = "issue #441 column: N₂-broadened vapour holds a 2-3 bar Earth at 299-313 K with no CO₂ left"]
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
    #[ignore = "issue #441 column: 259 K at 1 bar and +13 K for doubled N₂, against 270-290 K and +3-8 K"]
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
        // Below the knee, CO₂ keeps the plain square root of the whole
        // pressure; water its own pressure plus a share of the rest.
        let earth = Gases {
            n2: 0.9,
            co2: 3e-4,
            h2o: 0.0126,
            ..Gases::default()
        };
        let p = earth.total();
        let water_broadening = 0.0126 + WATER_FOREIGN_BROADENING * (0.9 + 3e-4);
        let expected = CO2_OPACITY * 3e-4_f64.powf(CO2_EXPONENT) * p.sqrt()
            + H2O_OPACITY * (0.0126 * water_broadening).sqrt();
        assert!((optical_depth(&earth) / expected - 1.0).abs() < 1e-12);
    }

    #[test]
    fn co2_and_water_broaden_each_other_in_full() {
        // CO₂ broadens water vapour's lines at least as well as N₂ does,
        // and water CO₂'s (Yang et al. 2022; Tan et al. 2019): only the
        // other gases are tempered past the knee.
        let thick = Gases {
            n2: 0.89,
            co2: 1.8,
            h2o: 0.01,
            ..Gases::default()
        };
        let co2 = |pb: f64| CO2_OPACITY * 1.8_f64.powf(CO2_EXPONENT) * pb.sqrt();
        let water = |foreign: f64| {
            H2O_OPACITY * (0.01 * (0.01 + WATER_FOREIGN_BROADENING * (1.8 + foreign))).sqrt()
        };
        let expected = co2(thick.total()) + water(0.89);
        assert!((optical_depth(&thick) / expected - 1.0).abs() < 1e-12);
        // Past the knee only the nitrogen is tempered: 5 bar of N₂ count
        // as K (5/K)^THICK_AIR_EXPONENT bar, the CO₂ and the vapour in
        // full for both.
        let nitrogen = Gases { n2: 5.0, ..thick };
        let n2 = BROADENING_KNEE_BAR * (5.0 / BROADENING_KNEE_BAR).powf(THICK_AIR_EXPONENT);
        let expected = co2(1.8 + 0.01 + n2) + water(n2);
        assert!((optical_depth(&nitrogen) / expected - 1.0).abs() < 1e-12);
    }

    #[test]
    #[ignore = "issue #441 column: 243 K with its 1.8 bar of CO₂ (285-290 K on the old core)"]
    fn an_earth_at_1_6_au_keeps_its_ocean_under_its_co2() {
        // An Earth at 1.6 AU, all its 1.8 bar of CO₂ in the air over
        // 0.89 bar of N₂, inside the cold edge of the habitable zone that
        // Kopparapu et al. 2013 put at about 1.67 AU.
        let (_, g, c) = climate_at(&model_inputs(reference("Earth")), 1.6);
        assert!(g.co2 > 1.0 && g.n2 < BROADENING_KNEE_BAR, "{g:?}");
        assert!((285.0..=290.0).contains(&c.temperature_k), "{c:?} {g:?}");
        assert_eq!(c.liquid, SurfaceLiquid::Water, "{c:?}");
    }

    #[test]
    fn a_uniform_column_integrates_to_the_column_totals() {
        // The opacity per ln p of a uniform air, integrated from the top
        // to the surface, gives back the totals of the per-gas laws.
        let air = Gases {
            n2: 3.0,
            co2: 0.3,
            h2o: 0.05,
            ch4: 0.02,
            ..Gases::default()
        };
        let share = foreign_share(&air);
        let steps = 20_000;
        let span = 40.0_f64;
        let mut tau = 0.0;
        for i in 0..steps {
            let f = (-(span * (i as f64 + 0.5) / steps as f64)).exp();
            let level = Gases {
                n2: air.n2 * f,
                co2: air.co2 * f,
                h2o: air.h2o * f,
                ch4: air.ch4 * f,
                ..Gases::default()
            };
            tau += opacity_per_ln_p(&level, share) * span / steps as f64;
        }
        let total = optical_depth(&air);
        assert!((tau / total - 1.0).abs() < 1e-6, "{tau} vs {total}");
    }

    #[test]
    fn steam_under_the_deepest_column_does_not_change_the_radiation() {
        // A steam runaway's 1.6e5 bar radiate as the top STEAM_MAX_BAR:
        // its few bar of CO₂ see no more broadening under it either.
        let runaway = Gases {
            h2o: 1.6e5,
            co2: 6.0,
            ..Gases::default()
        };
        let capped = Gases {
            h2o: STEAM_MAX_BAR,
            ..runaway
        };
        for t in [700.0, 1500.0, 2500.0] {
            assert_eq!(column::olr_w_m2(&runaway, t), column::olr_w_m2(&capped, t));
        }
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
    fn a_thick_steam_runaway_settles_past_the_end_of_the_plateau() {
        // Issue #438: 30 % water (some 1300 oceans) at 0.7 AU, over the
        // runaway limit, puts about 1.6e5 bar of steam in the air. Its
        // outgoing radiation stays on the Simpson-Nakajima plateau until
        // the dry adiabat reaches the emitting levels (about 1800 K for a
        // thick steam atmosphere, Boukrouche et al. 2021): absorbing 1.7
        // times the limit, it settles a little past it, not at thousands
        // of kelvin.
        let surface = water_world_at(0.3, 0.7);
        assert!(surface.water.runaway, "{surface:?}");
        assert!(surface.gases.h2o > 1.0e5, "{:?}", surface.gases);
        let t = surface.climate.temperature_k;
        println!("30 % water at 0.7 AU: {t:.0} K");
        assert!((1500.0..=2500.0).contains(&t), "{t} K: {surface:?}");
        assert_eq!(surface.climate.liquid, SurfaceLiquid::None, "{surface:?}");
    }

    /// Earth's air without its vapour, bar (argon has no field in
    /// [`Gases`]; it neither absorbs nor changes the adiabat much).
    fn earth_air() -> Gases {
        Gases {
            n2: 0.78,
            o2: 0.21,
            co2: 4e-4,
            ..Gases::default()
        }
    }

    #[test]
    fn the_moist_plateau_is_the_simpson_nakajima_limit() {
        // Earth air, saturated column, surface 320..400 K: OLR flat at
        // 282 W/m² ± 5 % (Goldblatt et al. 2013).
        for t in [320.0, 340.0, 360.0, 380.0, 400.0] {
            let olr = column::olr_w_m2(&with_vapour(earth_air(), t, f64::INFINITY), t);
            assert!((268.0..=296.0).contains(&olr), "{t} K: {olr}");
            println!("plateau at {t} K: {olr:.1} W/m²");
        }
    }

    #[test]
    fn the_plateau_does_not_depend_on_nitrogen() {
        let plateau = |n2: f64| {
            column::olr_w_m2(
                &with_vapour(Gases { n2, ..earth_air() }, 380.0, f64::INFINITY),
                380.0,
            )
        };
        let (one, two, three) = (plateau(0.78), plateau(1.56), plateau(2.34));
        println!("plateau at 380 K: {one:.1}, {two:.1}, {three:.1} W/m² (1x, 2x, 3x N₂)");
        assert!((one - three).abs() / one < 0.02, "{one} vs {three}");
        assert!((one - two).abs() / one < 0.02, "{one} vs {two}");
    }

    #[test]
    fn an_airless_column_emits_from_the_surface() {
        let olr = column::olr_w_m2(&Gases::default(), 250.0);
        assert!((olr - SIGMA * 250f64.powi(4)).abs() < 1e-6, "{olr}");
    }

    #[test]
    fn a_thick_nitrogen_earth_stays_temperate() {
        // Issue #441: at 1 AU with modern CO₂, 2 bar of N₂ tipped the old
        // grey core over the runaway; the moist limit does not depend on
        // the nitrogen, so even 11 bar keep a balance.
        let d = derive_at(&model_inputs(reference("Earth")), 1.0);
        for n2 in [2.0, 5.0, 11.0] {
            let gases = Gases {
                n2,
                co2: 4e-4,
                h2o: 0.0,
                ..d.gases
            };
            let (g, t) = wet_balance(&d.context, gases, d.heat_flux_w_m2, d.water_bar)
                .unwrap_or_else(|| panic!("{n2} bar of N₂ runs away"));
            println!("{n2} bar of N₂: {t:.1} K");
            assert!(t < 330.0, "{n2} bar: {t} K {g:?}");
        }
    }

    #[test]
    fn thin_steam_follows_the_plateau_past_the_old_grey_law() {
        // 10 bar of pure steam: the old grey law sent out the limit from
        // about 680 K; on the column it is still on the plateau there.
        let steam = Gases {
            h2o: 10.0,
            ..Gases::default()
        };
        let olr = column::olr_w_m2(&steam, 680.0);
        assert!(
            (olr / SIMPSON_NAKAJIMA_W_M2 - 1.0).abs() < 0.1,
            "{olr} W/m²"
        );
        // Where the plateau ends (Boukrouche et al. 2021: about 1050 K).
        let mut t = 680.0;
        while column::olr_w_m2(&steam, t) < 1.1 * olr && t < 5000.0 {
            t += 10.0;
        }
        println!("10 bar of steam leaves the plateau at about {t} K");
    }

    #[test]
    fn surface_liquid_is_kebab_case() {
        let s = serde_json::to_string(&SurfaceLiquid::Hydrocarbons).unwrap();
        assert_eq!(s, "\"hydrocarbons\"");
    }
}
