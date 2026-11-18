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

use super::albedo::{REFERENCE_ALBEDO, bond_albedo_over, cover, sulphuric_deck};
use super::atmosphere::{
    NITROGEN_TRIPLE_K, SEA_METHANE_SHARE, condense, condensed_air, methane_saturation_bar,
};
use super::{Context, Gases, Water, column};
use crate::geology::Regime;

/// The liquid that covers part of a surface, if any.
///
/// `Hydrocarbons` is methane with the nitrogen and ethane it dissolves,
/// a nitrogen-rich liquid where nitrogen itself condenses into it;
/// `Nitrogen` is condensed nitrogen with no methane liquid (see
/// [`liquid`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum SurfaceLiquid {
    None,
    Water,
    Hydrocarbons,
    Nitrogen,
    Lava,
}

/// The climate at a body's surface.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Climate {
    /// Mean surface temperature, K.
    pub temperature_k: f64,
    /// The equilibrium temperature the climate used, K: the context's
    /// under [`Climate::bond_albedo`] ([`Context::with_albedo`]).
    pub equilibrium_k: f64,
    /// Bond albedo the climate absorbed its light under: derived from
    /// what covers the body ([`super::albedo`]) together with its
    /// temperature, and blended towards the moist albedo of [`absorbing`]
    /// in a water runaway.
    pub bond_albedo: f64,
    pub liquid: SurfaceLiquid,
    /// Emissivity the surface radiated under ([`emissivity`]): 1 but
    /// where nitrogen frost covers it. Like the Bond albedo, the one the
    /// climate was solved under, so that it reproduces. 1 in the worlds
    /// stored before it.
    #[serde(default = "black_body")]
    pub emissivity: f64,
    /// The carbonate thermostat holds the temperature at
    /// [`THERMOSTAT_K`]: plates, liquid water and enough CO₂ to reach it.
    pub regulated: bool,
    /// The body is a snowball: its climate also has a temperate state,
    /// and it was drawn frozen over, its CO₂ building up from its
    /// volcanoes towards the thaw ([`climate`], [`snowball_chance`]).
    /// A body with no temperate state (past the outer edge of the
    /// habitable zone), or whose frozen state no CO₂ it can hold thaws,
    /// is frozen but not a snowball.
    pub snowball: bool,
    /// The climate's fixed points settled: the condensation of its air
    /// within [`TOLERANCE_K`] and its albedo and emissivity within
    /// [`ALBEDO_TOLERANCE_K`]. When not, the temperature is the last
    /// estimate of a cycle (issue #562). True in the worlds stored
    /// before it.
    #[serde(default = "converged_default")]
    pub converged: bool,
}

/// [`Climate::converged`] of the worlds stored before it: the climates
/// they show are taken as settled.
fn converged_default() -> bool {
    true
}

/// Stefan–Boltzmann constant, W/(m²·K⁴) (CODATA 2018, exact).
const SIGMA: f64 = 5.670_374_419e-8;

/// The emissivity of a black body, which the surfaces of the worlds
/// stored before [`Climate::emissivity`] radiated as.
fn black_body() -> f64 {
    1.0
}

/// Bolometric emissivity of nitrogen ice: 0.89. Calibration on Triton:
/// with Fray & Schmitt's sublimation curve ([`super::atmosphere`]) and
/// the model's frost albedo, it puts the triton-like archetype's surface
/// at 37.2 to 37.6 K under 1.4e-5 to 1.8e-5 bar of N₂ (seeds 1-40,
/// median 37.4 K and 1.57e-5 bar), against 38 K and the 14 µbar Voyager 2
/// measured (Tyler et al. 1989, doi:10.1126/science.246.4936.1466): the
/// curve gives 14 µbar at 37.2 K, so the two observations fit together
/// within a kelvin, and the calibration centres the archetype between
/// them. Its N₂ ice is β-N₂, whose
/// emissivity Stansberry, Pisano & Yelle 1996 (Planet. Space Sci. 44,
/// 945, doi:10.1016/0032-0633(96)00001-3; abstract checked) compute at
/// 0.85 for 1 cm grains and 0.40 for 1 mm grains (α-N₂, under 35.6 K,
/// 0.30 and 0.11); Bertrand et al. 2022 (Icarus 373, 114764,
/// arXiv:2110.11992) model Triton with 0.8 and quote Hillier et al.
/// 1991's 0.46 ± 0.16 from Voyager (both read in an excerpt of the
/// former, not checked in full). The
/// calibration sits just above the coarse-grained end of that range: a
/// lower emissivity would warm the model's Triton, under its frost
/// albedo of 0.75, past 38 K. One value for α- and β-N₂.
pub const N2_ICE_EMISSIVITY: f64 = 0.89;

/// Emissivity of a surface with `frost` (bar, gas by gas) on its ground:
/// [`N2_ICE_EMISSIVITY`] over the share of its disc that its nitrogen
/// frost covers, as a frost of its own ([`super::albedo::frost_cover`]),
/// 1 elsewhere: rock, water ice and the other frosts radiate close to a
/// black body (water ice about 0.97, from memory). Nitrogen, the most
/// volatile of the frosts, condenses last and lies over the others (the
/// CO₂ the cold trap froze out lies under Triton's N₂ as its bedrock
/// does).
pub fn emissivity(frost: &Gases) -> f64 {
    let nitrogen = Gases {
        n2: frost.n2.max(0.0),
        ..Gases::default()
    };
    let share = super::albedo::frost_cover(&nitrogen);
    1.0 - (1.0 - N2_ICE_EMISSIVITY) * share
}

/// Grey opacity of CO₂ at 1 bar of CO₂ in 1 bar of air: its optical
/// depth through a uniform column (see [`optical_depth_at`]).
/// Calibration, no source, on the column of [`super::column`]: Venus's
/// 737 K under its 85 bar of CO₂ at its real Bond albedo, 0.76, which the
/// derived albedo gives its sulphuric deck (739 K in the model; issue
/// #510), together with the late Archean Earth of [`CO2_EXPONENT`]. It
/// leaves Mars within two kelvin of its equilibrium temperature. (0.74
/// with the exponent 0.07 under the fixed albedo 0.3, which left Venus at
/// 565 K under 0.76; 1.66 on the old Eddington core, 0.575 on the column
/// with the old exponent 0.27, issue #441.)
const CO2_OPACITY: f64 = 1.45;

/// Power of the CO₂ partial pressure in its opacity. Calibration, no
/// source: CO₂'s 15 µm band is saturated at its centre, so its forcing
/// grows with the logarithm of its amount (Myhre et al. 1998,
/// doi:10.1029/98GL01908, `ΔF = 5.35 ln(C/C₀)` W/m²); a small power
/// stands in for that logarithm across the six decades from Earth's
/// 3e-4 bar to Venus's 85 bar.
///
/// Calibration on the column of issue #441, with [`CO2_OPACITY`], on
/// Venus's 737 K at its Bond albedo 0.76 and the late Archean Earth (0.82
/// of today's Sun, 10 mbar of CO₂ in 1 bar of N₂, Earth's albedo), which
/// must stay above 270 K and warm by 3 to 8 K when its nitrogen doubles
/// (Charnay et al. 2013, see [`THICK_AIR_EXPONENT`]): 0.19 gives it
/// 272 K and +7.0 K. A lower power makes small amounts of CO₂ stronger
/// against Venus's 85 bar and the Archean's nitrogen warming larger
/// (0.13: +11.5 K); a higher one the reverse (0.25: 268 K). Under the
/// fixed albedo 0.3, 0.07 held Venus at 737 K (0.27 before issue #441,
/// which left the Archean at 259 K). Modern Earth's thermostat then
/// leaves a few mbar of CO₂ (the real one, 0.28 mbar): the grey column's
/// trace CO₂ is weaker than the real band.
const CO2_EXPONENT: f64 = 0.19;

/// Partial pressure of CO₂, bar, below which its opacity falls in
/// proportion to its amount instead of as [`CO2_EXPONENT`]: the weak-line
/// limit, where the band is no longer saturated at its centre and absorbs
/// in proportion to the absorber (Pierrehumbert 2010, §4.4; Goody & Yung
/// 1989, ch. 4). Calibration, no source, at about 1 ppm of a bar: well
/// under Earth's 280 ppm, under the few ppm the thermostat leaves under 2
/// or 3 bar of N₂, and above the 3e-10 bar on Titan, which the small
/// power would otherwise turn into a greenhouse of a few kelvin.
const CO2_WEAK_BAR: f64 = 1e-6;

/// The power of CO₂'s own pressure in its opacity at `co2` bar: 1 in the
/// weak-line limit under [`CO2_WEAK_BAR`], [`CO2_EXPONENT`] above it.
fn co2_power(co2: f64) -> f64 {
    if co2 < CO2_WEAK_BAR {
        1.0
    } else {
        CO2_EXPONENT
    }
}

/// CO₂'s own factor in its opacity, `(p / 1 bar)^a`, continuous at
/// [`CO2_WEAK_BAR`]: linear under it, [`CO2_EXPONENT`] above it.
fn co2_amount(co2: f64) -> f64 {
    if co2 < CO2_WEAK_BAR {
        CO2_WEAK_BAR.powf(CO2_EXPONENT) * co2 / CO2_WEAK_BAR
    } else {
        co2.powf(CO2_EXPONENT)
    }
}

/// Grey opacity of water vapour at 1 bar of pure steam, with
/// [`LINE_EXPONENT`] (see [`water_broadening_bar`]). Calibration on the
/// column of [`super::column`]: its moist plateau, the Simpson–Nakajima
/// limit [`SIMPSON_NAKAJIMA_W_M2`], at 287 W/m² (282 W/m² in Goldblatt
/// et al. 2013) for Earth's air at 340 to 400 K; held a few W/m² above
/// the limit so that Earth's no-CO₂ climate and the late Archean stay
/// within their targets. It is twenty times
/// the old Eddington core's 5.9 (issue #441) because it now counts the
/// water's own pressure as its broadener, whose lines it widens about
/// twenty times better than nitrogen does (see
/// [`WATER_FOREIGN_BROADENING`]), and because the column radiates from
/// its cold upper levels where the vapour is a trace.
const H2O_OPACITY: f64 = 111.0;

/// Grey opacity of methane at 1 bar in 1 bar of air, with
/// [`LINE_EXPONENT`]. Calibration, no source, together with
/// [`CH4_NEAR_IR_MAX`]: Titan's 93.7 K (Fulchignoni et al. 2005,
/// doi:10.1038/nature04314) and the 5.65 % of methane near its surface
/// (Niemann et al. 2005, doi:10.1038/nature04122), which `titan-like`
/// reaches as 90.8 to 93.1 K (median 92.3 K) and 3.8 to 5.7 % (median
/// 4.8 %) over seeds 1 to 40 of `archetype_census`, against the 90 to
/// 97 K and 3 to 6 % of issue #530, under the vapour of its seas and the
/// anti-greenhouse of [`stratospheric_share`], on the column of
/// [`super::column`] (6.0 before the anti-greenhouse, which put
/// `titan-like` at 90.4 to 94.5 K and 3.7 to 6.8 %; 3.3 before its
/// saturation, issue #443; 3.4 on the old Eddington core, issue #441).
/// Titan's real greenhouse is mostly collision-induced (N₂–N₂, N₂–CH₄,
/// N₂–H₂; McKay, Pollack & Courtin 1991, doi:10.1126/science.11538492,
/// not checked), and its haze cools it by about 9 K; this opacity carries
/// all of it but the methane's own near-infrared share. That is why it keeps the
/// square root of the air's pressure (see [`methane_depth`]):
/// collision-induced absorption grows with the nitrogen faster than line
/// broadening does, not slower.
///
/// It applies to the methane that radiates, the methane cold-trapped at
/// the tropopause left out (see [`super::column`]), and saturates past
/// [`CH4_SATURATION_BAR`] (see [`methane_depth`]).
const CH4_OPACITY: f64 = 7.75;

/// Methane, bar, past which its greenhouse saturates ([`methane_depth`]):
/// the amount `u` its opacity counts grows as `p / (1 + p / s)`, `p` the
/// methane, so that it never exceeds this. Past it the law is not
/// extrapolated: the grey opacity stands for collision-induced bands
/// whose centres are already opaque at Titan's amounts (McKay, Pollack &
/// Courtin 1991, doi:10.1126/science.11538492, not checked), so that more
/// methane only widens their wings, as the logarithmic forcing of a
/// saturated band (Pierrehumbert 2010, §4.4); a body at 90 to 120 K
/// radiates at 25 to 60 µm, where methane has no strong band of its own.
///
/// Calibration, no source, together with [`CH4_OPACITY`], on three
/// constraints at once: Titan's 93.7 K; Titan's warming from 1 to 10 bar
/// of methane under a third of its warming from 0.01 to 0.1 bar (0.6 K
/// against 2.6 K with the opacity 6.0; 0.05 bar gave 2.9 K against
/// 4.2 K), now held on the outgoing radiation alone, since the
/// anti-greenhouse of [`stratospheric_share`] grows over the second step
/// (with it, 0.8 K against 1.2 K); and seed 182's planet d (an
/// equilibrium temperature of 114 K under 4.4 bar of N₂ and 26 bar of
/// methane) under 150 K (149 K before the anti-greenhouse, 135 K with
/// it; 157 K with 0.05 bar). Titan's
/// own radiating methane, some 0.05 bar, is past it: its greenhouse is
/// already saturated. Without it, the `sqrt(p_CH₄ P)` law stretched
/// 500-fold past Titan put that planet at 351 K (issue #443).
const CH4_SATURATION_BAR: f64 = 0.01;

/// Grey optical depth of the methane of a uniform air of `gases` (bar)
/// down to its surface, and its rate per `ln p`.
///
/// `k (u B)^½`: [`CH4_OPACITY`], the amount `u = p / (1 + p / s)` that
/// saturates at [`CH4_SATURATION_BAR`] `s`, and the pressure `B` of the
/// air that collides with it, the whole air with its methane counted as
/// `u` (methane past its saturation adds no absorber, nor a partner). In
/// a uniform air every partial pressure grows as `p`, so the rate is
/// `½ (d ln u / d ln p + d ln B / d ln p)` times the depth.
fn methane_depth(gases: &Gases) -> (f64, f64) {
    let ch4 = gases.ch4.max(0.0);
    if ch4 <= 0.0 {
        return (0.0, 0.0);
    }
    let others = (gases.total() - ch4).max(0.0);
    let growth = 1.0 / (1.0 + ch4 / CH4_SATURATION_BAR);
    let u = ch4 * growth;
    let b = others + u;
    let depth = CH4_OPACITY * (u * b).sqrt();
    let rate = 0.5 * (growth + (others + u * growth) / b);
    (depth, rate * depth)
}

/// Most of the starlight a body absorbs that the methane above its
/// tropopause can take in its near-infrared bands (0.89 to 3.3 µm, from
/// memory) and send back to space ([`stratospheric_share`]): the
/// anti-greenhouse of methane, which McKay, Pollack & Courtin 1991
/// (*Science* 253, 1118, doi:10.1126/science.11538492; title and
/// bibliographic data checked, the paper not) describe on Titan together
/// with its haze's. Calibration, no source, together with
/// [`CH4_OPACITY`] and [`CH4_NEAR_IR_HALF_BAR`]: `titan-like` at 3.8 to
/// 5.7 % of methane and 90.8 to 93.1 K over seeds 1 to 40 of
/// `archetype_census` (targets 3 to 6 % and 90 to 97 K, issue #530),
/// where it takes 11 % of the light of the reference Titan. Because the
/// methane let through the cold trap grows with the tropopause's
/// temperature, a Titan nearer its star loses more of its light: it
/// narrows the spread of the archetype's temperature (2.4 K against
/// 4.1 K without it), and with it the spread of the methane its seas
/// keep in its air. A larger value narrows it more but needs a larger
/// [`CH4_OPACITY`] (0.4 with 8.5: 90.8 to 92.4 K, 3.9 to 5.3 %). The
/// share of the light in methane's bands depends on the star's spectrum,
/// redder stars giving more of it; one value for every star here.
const CH4_NEAR_IR_MAX: f64 = 0.3;

/// Methane above the tropopause, bar, at which it takes half of
/// [`CH4_NEAR_IR_MAX`] ([`stratospheric_share`]): the column past which
/// its near-infrared bands are opaque and only their wings widen.
/// Calibration, no source, with [`CH4_NEAR_IR_MAX`]: Titan's cold trap
/// lets through some 0.003 bar (its stratosphere's 1.4 % of methane over
/// 0.1 bar holds 0.0014 bar, Niemann et al. 2005, from memory), under
/// it, so that its share still grows with the tropopause's temperature;
/// a world of 26 bar of methane under a warm tropopause takes nearly all
/// of [`CH4_NEAR_IR_MAX`] (0.27 for census seed 182's planet d).
const CH4_NEAR_IR_HALF_BAR: f64 = 0.005;

/// Share of the starlight a body absorbs that the methane above its
/// tropopause takes first, in its near-infrared bands, and sends back
/// to space: `m / (m + h)` of [`CH4_NEAR_IR_MAX`], `m` the methane above
/// the tropopause of a stratosphere at `skin_k`
/// ([`column::stratospheric_methane_bar`]) and `h`
/// [`CH4_NEAR_IR_HALF_BAR`]. None without methane.
fn stratospheric_share(gases: &Gases, skin_k: f64) -> f64 {
    let m = column::stratospheric_methane_bar(gases, skin_k);
    if m <= 0.0 {
        return 0.0;
    }
    CH4_NEAR_IR_MAX * m / (m + CH4_NEAR_IR_HALF_BAR)
}

/// What the surface of a body under `gases` absorbs, W/m², at the
/// equilibrium temperature `t_eq` (the light, `σ T_eq⁴`) with
/// `heat_flux_w_m2` of internal heat: the light less the share the
/// stratosphere's methane takes and radiates to space
/// ([`stratospheric_share`], the anti-greenhouse of McKay, Pollack &
/// Courtin 1991, doi:10.1126/science.11538492, not checked), under the
/// skin temperature `T_eq / 2^¼` the column uses. The stratosphere that
/// absorbs it is not warmed in the column: its skin temperature stays
/// that of a grey stratosphere.
fn absorbed_w_m2(t_eq: f64, gases: &Gases, heat_flux_w_m2: f64) -> f64 {
    let t_eq = t_eq.max(0.0);
    let skin = t_eq * 0.5_f64.powf(0.25);
    SIGMA * t_eq.powi(4) * (1.0 - stratospheric_share(gases, skin)) + heat_flux_w_m2.max(0.0)
}

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
///
/// The saturation also acts level by level: see [`LOCAL_KNEE_BAR`].
const THICK_AIR_EXPONENT: f64 = 0.0;

/// Pressure of the other gases at a level, bar, up to which they broaden
/// the lines there in proportion; past it, as `K (p/K)^`
/// [`LOCAL_KNEE_EXPONENT`]. Lines overlap and their absorption saturates
/// as they widen (Vladilo et al. 2013, ApJ 767, 65,
/// doi:10.1088/0004-637X/767/1/65, App. A.3, checked), so that nitrogen
/// warms a planet much less than the strong-line law says.
///
/// Calibration on the column of [`super::column`] (issue #441): at 288 K
/// the column emits from levels of a few tenths of a bar of nitrogen, past
/// the knee, so a modern Earth under 2 or 3 bar of N₂ stays regulated,
/// one without CO₂ falls to 263 K (well below 288 K; Lacis et al. 2010,
/// *Science* 330, 356, doi:10.1126/science.1190653) and doubled nitrogen
/// warms the late Archean by 7.4 K (Goldblatt et al. 2009 find 4.4 K,
/// Charnay et al. 2013 7 K); at the 380 K moist plateau the emitting
/// levels hold a few hundredths of a bar of nitrogen, under the knee, so
/// the plateau keeps its independence from the nitrogen.
const LOCAL_KNEE_BAR: f64 = 0.18;

/// Power of the other gases' pressure past [`LOCAL_KNEE_BAR`] at a level
/// in the pressure that broadens lines there. Calibration with it: the
/// doubled-nitrogen warming of the late Archean (7.4 K at 0.05, 9.2 K at
/// 0.1 with a knee of 0.2 bar).
const LOCAL_KNEE_EXPONENT: f64 = 0.05;

/// How well the other gases broaden water's lines against the water
/// itself, per bar (see [`water_broadening_bar`]). Calibration on the
/// column of [`super::column`]: with it the moist plateau stays within
/// 2 % between one and three times Earth's nitrogen (more nitrogen lifts
/// the plateau through the adiabat and lowers it through broadening; a
/// share of 0 lifts it by several per cent), and Earth's
/// thermostat holds 288 K with under 0.01 bar of CO₂. Water broadens its
/// own lines about five times better than air does (self- against
/// air-broadened half-widths in HITRAN, Gordon et al. 2017, *JQSRT* 203,
/// 3, doi:10.1016/j.jqsrt.2017.06.038; from memory, not checked), and its
/// self-continuum, which dominates the windows of a moist atmosphere, is
/// some hundred times its foreign continuum per molecule (Shine, Ptashnik
/// & Rädel 2012, *Surv. Geophys.* 33, 535, doi:10.1007/s10712-011-9170-y;
/// from memory, not checked): a grey share between the two.
const WATER_FOREIGN_BROADENING: f64 = 0.05;

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
/// `share` (see [`foreign_share`]), saturating past [`LOCAL_KNEE_BAR`].
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
    let others = (gases.total() - co2 - h2o).max(0.0) * share;
    let foreign = if others <= LOCAL_KNEE_BAR {
        others
    } else {
        LOCAL_KNEE_BAR * (others / LOCAL_KNEE_BAR).powf(LOCAL_KNEE_EXPONENT)
    };
    co2 + h2o + foreign
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
///
/// This cap is a surrogate: it stands for the non-grey 4 µm window
/// (Boukrouche et al. 2021; Goldblatt 2015) through which a real hot
/// steam atmosphere radiates and which the grey column cannot see. The
/// window exists in any thick steam column, so the cap holds for every
/// column the model solves: a runaway's steam, and the vapour of a wet
/// surface too (capped at its saturation first, it reaches 70 bar only
/// above about 560 K, near water's critical point).
pub(super) const STEAM_MAX_BAR: f64 = 70.0;

/// The column totals of the grey optical depths of CO₂, water and
/// methane in a uniform air of `gases` (bar), whose other gases broaden
/// lines by `share` of their pressure: each absorber's
/// `k_i (p_i / 1 bar)^a_i (P_b / 1 bar)^½`, the last factor the pressure
/// broadening of its lines (see [`LINE_EXPONENT`]). For CO₂ `P_b` is
/// [`broadening_bar`], for water [`water_broadening_bar`]; methane's
/// saturates and its `P_b` is the whole air, since its opacity stands for
/// Titan's collision-induced greenhouse (see [`methane_depth`]).
fn depths(gases: &Gases, share: f64) -> [f64; 3] {
    // `LINE_EXPONENT` is ½: square roots, the column's hot path.
    const { assert!(LINE_EXPONENT == 0.5) };
    let co2 = gases.co2.max(0.0);
    let h2o = gases.h2o.max(0.0);
    [
        if co2 > 0.0 {
            CO2_OPACITY * co2_amount(co2) * broadening_bar(gases, share).max(0.0).sqrt()
        } else {
            0.0
        },
        H2O_OPACITY * (h2o * water_broadening_bar(gases, share).max(0.0)).sqrt(),
        methane_depth(gases).0,
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
/// constant composition integrates back to its totals. Each depth of CO₂
/// and water goes as `p^(a_i + ½)`, so its rate per `ln p` is `(a_i + ½)`
/// times itself; methane's saturates ([`methane_depth`]).
/// Past [`LOCAL_KNEE_BAR`] of foreign gases the broadening grows slower
/// and this rate overstates the derivative a little; the calibrations
/// are made with it.
pub(super) fn opacity_per_ln_p(gases: &Gases, share: f64) -> f64 {
    let [co2, h2o, _] = depths(gases, share);
    (co2_power(gases.co2.max(0.0)) + LINE_EXPONENT) * co2
        + (LINE_EXPONENT + LINE_EXPONENT) * h2o
        + methane_depth(gases).1
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

/// Least and greatest Bond albedo of a body in a water runaway
/// ([`absorbing`]).
const RUNAWAY_ALBEDO: (f64, f64) = (0.0, 0.9);

/// The column's own moist plateau, W/m²: the outgoing radiation of
/// Earth's air over a saturated surface at 380 K (about 287 W/m², the
/// Simpson–Nakajima limit of the column, see [`H2O_OPACITY`]), under the
/// stratosphere the surface solves give it ([`column::olr_under_skin_w_m2`]):
/// the skin `T_eq / 2^¼` of a body at its star's runaway limit `S_rg`.
/// There a runaway absorbs the plateau itself (see [`absorbing`]), so
/// `T_eq = (F_plateau / σ)^¼` whatever the star: the plateau and its skin
/// are solved together by a fixed-point iteration (the skin moves the
/// plateau by under 0.1 W/m², so a few steps settle it).
pub(super) static MOIST_PLATEAU_W_M2: std::sync::LazyLock<f64> = std::sync::LazyLock::new(|| {
    let air = Gases {
        n2: 0.78,
        o2: 0.21,
        co2: 4e-4,
        ..Gases::default()
    };
    let moist = with_vapour(air, 380.0, f64::INFINITY);
    let mut plateau = column::olr_w_m2(&moist, 380.0);
    for _ in 0..8 {
        plateau = column::olr_under_skin_w_m2(&moist, 380.0, plateau_skin_k(plateau));
    }
    plateau
});

/// The skin temperature, K, of the surface solves (`T_eq / 2^¼`) for a
/// body that absorbs `absorbed_w_m2`: `T_eq = (absorbed / σ)^¼`.
fn plateau_skin_k(absorbed_w_m2: f64) -> f64 {
    (absorbed_w_m2 / SIGMA).powf(0.25) * 0.5_f64.powf(0.25)
}

/// Steam column, bar, over which a runaway's Bond albedo moves from
/// its derived one to the moist one `A_rg` (see [`absorbing`]): the e-folding
/// pressure of the blend `1 − exp(−p / STEAM_ALBEDO_BAR)`, as if the
/// steam's optical depth in the bands that set the albedo were `p` over
/// this. Calibration, no source: the moist atmospheres whose spectrum
/// Kopparapu et al. 2013 take at the runaway limit hold a saturated
/// surface of some 340 to 380 K, under a fraction of a bar to about one
/// bar of vapour (`water_saturation_bar`); one bar puts a column of that
/// order two-thirds of the way to `A_rg`, and a steam world's hundreds of
/// bar all the way. Below it the steam is too thin to change the light a
/// body reflects much, and at no water the albedo is the derived one
/// again, with no jump when the last of it escapes.
const STEAM_ALBEDO_BAR: f64 = 1.0;

/// The Bond albedo the climate of a body with `water` and `steam_bar`
/// of steam in its air absorbs its light under, given its derived
/// `albedo` ([`super::albedo::bond_albedo`]): that albedo for a body with
/// no water runaway, and for one in a water runaway (by the insolation
/// limit or a boil) the blend from it to
/// `A_rg = 1 − 4 F_plateau / (S_rg S₀)` by its steam,
/// `A = albedo + (A_rg − albedo) (1 − exp(−p_steam / STEAM_ALBEDO_BAR))`
/// ([`STEAM_ALBEDO_BAR`]). `A_rg` makes the column's moist plateau
/// ([`MOIST_PLATEAU_W_M2`]) bind exactly at the runaway limit `S_rg` of
/// its star ([`crate::star::habitable_fluxes`]), clamped to
/// [`RUNAWAY_ALBEDO`].
///
/// Kopparapu et al. 2013 (doi:10.1088/0004-637X/765/2/131) and 2014
/// (doi:10.1088/2041-8205/787/2/L29) place the runaway limit where the
/// light a moist atmosphere absorbs, with the albedo its own spectrum
/// gives under that star, meets its outgoing limit. The chain decides
/// the runaway by their limit; with Earth's albedo the column would
/// absorb less than its plateau at that flux round a red star (an M9V
/// planet at its 0.91 S⊕ limit takes 0.7 × 0.91 S₀ / 4 = 218 W/m²) and
/// sit on water's critical point instead of running away. A wet body
/// keeps its derived albedo, and so does a runaway whose escape took all
/// its water (Mercury, a dry Venus): no moist atmosphere is left whose
/// albedo the limit describes; the blend brings a runaway losing its
/// water there continuously.
pub(super) fn runaway_albedo(context: &Context, water: &Water, steam_bar: f64, albedo: f64) -> f64 {
    let steam = 1.0 - (-steam_bar.max(0.0) / STEAM_ALBEDO_BAR).exp();
    if !(water.runaway && water.kept_kg > 0.0) || steam <= 0.0 {
        return albedo;
    }
    let limit = f64::from(crate::star::habitable_fluxes(context.teff_k as f32).0);
    // `S₀` as `equilibrium_k` takes it: `σ (255 K)⁴ = (1 − 0.3) S₀ / 4`.
    let earth_eq = f64::from(crate::star::equilibrium_k(1.0));
    let solar_constant = 4.0 * SIGMA * earth_eq.powi(4) / (1.0 - REFERENCE_ALBEDO);
    let moist = (1.0 - 4.0 * *MOIST_PLATEAU_W_M2 / (limit * solar_constant))
        .clamp(RUNAWAY_ALBEDO.0, RUNAWAY_ALBEDO.1);
    albedo + (moist - albedo) * steam
}

/// The `context` the climate of a body with `water` and `steam_bar` of
/// steam absorbs its light in, under its derived `albedo`: the context
/// under the [`runaway_albedo`] ([`Context::with_albedo`]).
pub(super) fn absorbing(context: &Context, water: &Water, steam_bar: f64, albedo: f64) -> Context {
    context.with_albedo(runaway_albedo(context, water, steam_bar, albedo))
}

/// Mean surface temperature, K, of a body with `gases` (bar) under its
/// star's light, plus its internal `heat_flux_w_m2`, whose surface
/// radiates with `emissivity` (1 for a black body; see [`emissivity`]).
///
/// The temperature at which the outgoing radiation of the
/// radiative–convective column of [`super::column`] balances what the
/// surface absorbs, `σ T_eq⁴`, with `T_eq` the context's equilibrium
/// temperature (under the Bond albedo it was given, see
/// [`Context::with_albedo`]) less the share the stratosphere's methane
/// takes ([`absorbed_w_m2`]), plus the internal heat; the column's
/// stratosphere sits at the skin temperature `T_eq / 2^¼` of a grey
/// atmosphere (Pierrehumbert 2010, §4.3; from memory, not checked). Its opacities are calibrated on Venus, Earth, Titan and
/// the moist plateau; a steam atmosphere sends out the Simpson–Nakajima
/// limit until its surface passes about 1800 K. Without air, the surface
/// sits at `T_eq` warmed by its internal heat alone.
///
/// A grey surface (`emissivity` under 1) sends out `ε σ T⁴`: the column's
/// outgoing radiation is taken times `ε`, as through a transparent air.
/// Only nitrogen frost is grey here, and the air over it is thin: under
/// nitrogen's triple point its vapour is at most 0.125 bar.
pub fn surface_temperature_k(
    context: &Context,
    gases: &Gases,
    heat_flux_w_m2: f64,
    emissivity: f64,
) -> f64 {
    let t_eq = context.equilibrium_k.max(0.0);
    let absorbed = absorbed_w_m2(t_eq, gases, heat_flux_w_m2);
    let e = emissivity.clamp(0.01, 1.0);
    let bare = (absorbed / (e * SIGMA)).powf(0.25);
    if gases.total() <= 0.0 || bare <= 0.0 {
        return bare;
    }
    let skin = t_eq * 0.5_f64.powf(0.25);
    let excess = |t: f64| e * column::olr_under_skin_w_m2(gases, t, skin) - absorbed;
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

/// Critical pressure of water, bar (IAPWS-95: 22.064 MPa).
pub const WATER_CRITICAL_BAR: f64 = 220.64;

/// Least column of steam, bar, that hides a runaway's surface and makes
/// it a steam world ([`hides_surface`]): ten times water's critical
/// pressure. Calibration, no source: past the critical point the steam
/// and the water below it are one supercritical fluid, and under ten
/// times the critical pressure the column is several times deeper than
/// any condensed ocean the model draws on a rock (Earth's ocean weighs
/// about 270 bar). In the 500-system census of the climate pass (#492)
/// no runaway holds between 1e3 and 1e5 bar of steam (383 runaways: 381
/// under 100 bar, one under 1e3 bar, two over 1e5 bar), so the line cuts
/// through an empty band of the population.
pub const STEAM_WORLD_MIN_BAR: f64 = 10.0 * WATER_CRITICAL_BAR;

/// Whether a bare body's surface is hidden under a deep supercritical
/// steam column, which makes it a steam world (#492): its water ran
/// away, its steam weighs at least [`STEAM_WORLD_MIN_BAR`], and its
/// surface is past water's critical temperature ([`WATER_CRITICAL_K`]),
/// so that no liquid–gas boundary marks a surface. A runaway with thinner
/// steam (Venus-like) keeps a visible rock; a planet in wet equilibrium
/// is never one (hycean worlds, #500, are hydrogen over liquid water).
pub fn hides_surface(surface: &super::Surface) -> bool {
    surface.water.runaway
        && surface.gases.h2o >= STEAM_WORLD_MIN_BAR
        && surface.climate.temperature_k >= WATER_CRITICAL_K
}

/// Latent heat of vaporisation of water, J/kg: 2.4e6, between its
/// 2.50e6 at 0 °C and 2.26e6 at 100 °C.
pub(super) const WATER_LATENT_J_KG: f64 = 2.4e6;

/// Gas constant of water vapour, J/(kg·K): 461.5.
pub(super) const WATER_R_V: f64 = 461.5;

/// Latent heat of vaporisation of water over its gas constant
/// (`L / R_v`), K: [`WATER_LATENT_J_KG`] over [`WATER_R_V`]. This mean
/// puts the boiling point at 1 atm at 373 K and the saturation at 288 K
/// within 5 % of the tables.
pub(super) const WATER_L_OVER_R: f64 = WATER_LATENT_J_KG / WATER_R_V;

/// Saturation vapour pressure of water at `temperature_k`, bar:
/// Clausius–Clapeyron from the triple point with a constant latent heat
/// (Pierrehumbert 2010, §2.1), capped at the critical point. Below
/// 273 K it stands in for the vapour over ice.
pub(super) fn water_saturation_bar(temperature_k: f64) -> f64 {
    let t = temperature_k.clamp(1.0, WATER_CRITICAL_K);
    WATER_TRIPLE_BAR * (WATER_L_OVER_R * (1.0 / WATER_TRIPLE_K - 1.0 / t)).exp()
}

/// Boiling point of water under `pressure_bar`, K: the same
/// Clausius–Clapeyron inverted, capped at the critical point. It is also
/// the dew point of a vapour of partial pressure `pressure_bar`.
pub(super) fn water_boils_k(pressure_bar: f64) -> f64 {
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

/// Triple point of ethane, K: 90.4 K (Lemmon et al. 2013, as Richardson,
/// Hartwig & Leachman 2019 quote it; checked in their manuscript).
const ETHANE_TRIPLE_K: f64 = 90.4;

/// Enthalpy of melting of the ethane solid that a cold liquid freezes
/// into, over the gas constant, K: 2.79 kJ/mol at 89.5 K (Domalski &
/// Hearing 1996, through the NIST Chemistry WebBook; checked there) over
/// 8.314 J/(mol·K). Ethane's own enthalpy of fusion is 0.583 kJ/mol
/// (Atake & Chihara 1976, NIST), but its plastic phase lasts less than a
/// kelvin under its triple point: under it the solid takes the
/// transition's enthalpy too, the sum Domalski & Hearing give.
const ETHANE_FUSION_L_OVER_R: f64 = 2_790.0 / 8.314;

/// Non-ideality of methane in a liquid of methane, ethane and nitrogen:
/// the regular-solution parameter `W / R`, K, whose activity coefficient
/// `ln γ = (W / R)(1 − x)² / T` raises the methane liquidus
/// ([`hydrocarbon_freezes_k`]). Calibration, no source, on Richardson,
/// Hartwig & Leachman 2019's liquid F1 (61.0 % methane, 25.6 % ethane,
/// 13.4 % nitrogen, frozen at 71.5 ± 0.5 K), which the ideal liquidus
/// puts at 64.9 K: an ideal methane liquidus is too steep, its enthalpy
/// of fusion (0.94 kJ/mol) being small.
const METHANE_EXCESS_W_OVER_R: f64 = 75.3;

/// The temperature, K, under which a liquid of methane with mole
/// fractions `nitrogen` of N₂ and `ethane` of C₂H₆ freezes: the higher of
/// its methane and ethane liquidus (the solid that comes out first).
///
/// A simple fit, labelled so, of a phase diagram this does not resolve
/// (the solid solutions, the ternary eutectic). The ethane liquidus is an
/// ideal solution's, the Schröder–van Laar equation
/// `ln x = −(L / R)(1/T − 1/T_m)` with [`ETHANE_FUSION_L_OVER_R`] and
/// [`ETHANE_TRIPLE_K`]; the methane liquidus a regular solution's, the
/// same with `ln(γ x)` ([`METHANE_EXCESS_W_OVER_R`],
/// [`METHANE_FUSION_L_OVER_R`], [`METHANE_TRIPLE_K`]), nitrogen and
/// ethane taken alike. Richardson, Hartwig & Leachman 2019
/// (doi:10.1016/j.ijthermalsci.2018.12.024; checked in their NASA
/// manuscript, NTRS 20180008677) measured two liquids freezing at 71.5
/// K (F1: 61.0 % CH₄, 25.6 % C₂H₆, 13.4 % N₂, 0.29 bar) and 74.0 K (F2:
/// 46.9 %, 45.3 %, 7.9 %, 0.52 bar), "more than 15 K below" the pure
/// ices: this gives 71.5 K (methane first; the calibration) and 74.5 K
/// (ethane first, the ideal law alone), where the binary
/// methane–nitrogen rule without the ethane gave 81.3 and 85.0 K. Pure
/// methane freezes at its triple point.
fn hydrocarbon_freezes_k(nitrogen: f64, ethane: f64) -> f64 {
    let nitrogen = nitrogen.clamp(0.0, 1.0);
    let ethane = ethane.clamp(0.0, 1.0 - nitrogen);
    let methane = 1.0 - nitrogen - ethane;
    let liquidus = |x: f64, triple_k: f64, l_over_r: f64, w_over_r: f64| {
        if x <= 0.0 {
            return 0.0;
        }
        // `ln x + W (1 − x)² / T = −L (1/T − 1/T_m)`, solved for `1/T`.
        let inverse = (l_over_r / triple_k - x.ln()) / (l_over_r + w_over_r * (1.0 - x).powi(2));
        1.0 / inverse
    };
    let methane_k = liquidus(
        methane,
        METHANE_TRIPLE_K,
        METHANE_FUSION_L_OVER_R,
        METHANE_EXCESS_W_OVER_R,
    );
    let ethane_k = liquidus(ethane, ETHANE_TRIPLE_K, ETHANE_FUSION_L_OVER_R, 0.0);
    methane_k.max(ethane_k)
}

/// The temperature, K, under which the hydrocarbon sea of a body whose
/// air holds `n2_bar` of nitrogen freezes: the liquidus of a sea of
/// methane, ethane and the nitrogen dissolved in it
/// ([`hydrocarbon_freezes_k`]).
///
/// The sea's nitrogen mole fraction is half of Raoult's law at the
/// surface's N₂ partial pressure and pure methane's triple point
/// ([`NITROGEN_RAOULT_SHARE`], Thompson, Zollweg & Gabis 1992), at most
/// [`MAX_NITROGEN_IN_LIQUID`]; its methane [`SEA_METHANE_SHARE`], the
/// share the climate's air keeps over it (Raoult's law); the rest is
/// ethane, which methane's photolysis makes on any methane world (the
/// model does not track it). Titan's 1.45 bar of N₂ gives 19 % N₂ and
/// 31 % ethane and a liquid down to 68.71 K (its surface: 93.7 K;
/// 79.41 K without the ethane); with no nitrogen, half ethane, 76.18 K.
///
/// The methane share is fixed, so the methane liquidus is a constant
/// floor, 67.97 K (half methane): the dissolved nitrogen replaces
/// ethane, not methane, and lowers the freezing point only by diluting
/// the ethane, until the ethane liquidus falls under that floor (from
/// about 2 bar of N₂; the [`MAX_NITROGEN_IN_LIQUID`] cap comes later).
pub fn methane_freezes_k(n2_bar: f64) -> f64 {
    let nitrogen = (NITROGEN_RAOULT_SHARE * n2_bar.max(0.0)
        / nitrogen_vapour_bar(METHANE_TRIPLE_K))
    .min(MAX_NITROGEN_IN_LIQUID);
    let ethane = (1.0 - SEA_METHANE_SHARE - nitrogen).max(0.0);
    hydrocarbon_freezes_k(nitrogen, ethane)
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

/// The liquid at a surface at `temperature_k` under `gases`, with
/// `condensate` (bar) on its ground ([`condense`]), from the phase
/// diagrams: lava above [`LAVA_K`], on a body that has kept no
/// water (with water on top, its steam or supercritical fluid covers the
/// rock: no lava, no liquid, Hamano, Abe & Genda 2013,
/// doi:10.1038/nature12163, for the steam over a magma ocean; not
/// checked); water between the melting of ice and its boiling point at
/// the surface pressure, above its triple point pressure, when water is
/// kept out of a runaway; hydrocarbons between the freezing point of
/// the sea ([`sea_freezes_k`]: methane and ethane with the nitrogen of
/// the air dissolved in them, or nitrogen's triple point where nitrogen condenses
/// into it) and ethane's boiling point, where there is enough methane to
/// condense ([`methane_condenses`]); nitrogen where nitrogen condenses
/// above its triple point without a methane liquid (it is condensed only
/// under its boiling point at its partial pressure and its critical
/// point, [`condense`]; under its triple point it is a frost,
/// [`frost`]); none otherwise.
fn liquid(temperature_k: f64, gases: &Gases, water: &Water, condensate: &Gases) -> SurfaceLiquid {
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
    } else if (condensate.ch4 > 0.0 || methane_condenses(gases.ch4, temperature_k))
        && temperature_k >= sea_freezes_k(gases.n2, condensate)
        && temperature_k < ethane_boils_k(pressure)
    {
        SurfaceLiquid::Hydrocarbons
    } else if condensate.n2 > 0.0 && temperature_k >= NITROGEN_TRIPLE_K {
        SurfaceLiquid::Nitrogen
    } else {
        SurfaceLiquid::None
    }
}

/// The temperature, K, under which the hydrocarbon sea of a body whose
/// air holds `air_n2_bar` of nitrogen, with `condensate` (bar) on its
/// ground ([`condense`]), freezes. Where nitrogen itself condenses, the
/// air holds pure nitrogen's saturation pressure, so the liquid it
/// condenses into is nitrogen with methane dissolved in it: it stays
/// liquid down to nitrogen's triple point ([`NITROGEN_TRIPLE_K`]), where
/// pure nitrogen freezes. A simple rule, labelled so: an ideal solution
/// would stay liquid colder still (the Schröder–van Laar equations with
/// methane's enthalpy of fusion and nitrogen's, 0.72 kJ/mol from memory,
/// meet at about 48 K and two-thirds nitrogen), but N₂ and CH₄ freeze
/// into solid solutions that an ideal liquidus does not describe (not
/// checked), so the rule stops at the triple point and errs towards
/// freezing. Otherwise the liquidus of a methane–ethane sea with the
/// nitrogen the air dissolves in it ([`methane_freezes_k`], issue #530).
/// Issue #524.
pub(crate) fn sea_freezes_k(air_n2_bar: f64, condensate: &Gases) -> f64 {
    if condensate.n2 > 0.0 {
        NITROGEN_TRIPLE_K
    } else {
        methane_freezes_k(air_n2_bar)
    }
}

/// Whether `methane_bar` of methane in the air condenses on a surface at `temperature_k`: it reaches the vapour
/// pressure of a Titan sea there, [`SEA_METHANE_SHARE`] of pure
/// methane's saturation ([`methane_saturation_bar`], Raoult's law). A
/// trace of methane stays a gas and makes no lake; past methane's
/// critical point none condenses.
fn methane_condenses(methane_bar: f64, temperature_k: f64) -> bool {
    methane_bar > 0.0 && methane_bar >= methane_sea_vapour_bar(temperature_k)
}

/// The methane, bar, in the air over a Titan sea at `temperature_k`:
/// [`SEA_METHANE_SHARE`] of pure methane's saturation pressure
/// (Raoult's law). A body needs that much methane for a hydrocarbon
/// liquid; infinite past methane's critical point (190.56 K).
pub fn methane_sea_vapour_bar(temperature_k: f64) -> f64 {
    SEA_METHANE_SHARE * methane_saturation_bar(temperature_k)
}

/// Whether a body that keeps its water out of a runaway boils it: with
/// the air `gases` its climate solved the wet surface with (its CO₂ as
/// the thermostat left it; its water vapour is set by the balance), no
/// wet balance exists ([`wet_balance`] is `None`): the light it absorbs
/// plus its internal heat exceed what its moist column can send out, the
/// moist runaway of a wet planet the insolation limit of
/// [`super::regime::water`] did not see (Kasting 1988,
/// doi:10.1016/0019-1035(88)90116-9, not checked; Goldblatt et al. 2013,
/// doi:10.1038/ngeo1892, abstract checked: a runaway once the absorbed
/// flux exceeds the moist limit). Its ocean is then steam: the
/// chain derives the body again with [`super::regime::steam`] and its
/// energy-limited escape, and keeps the water that escape leaves as an
/// ocean only if that leftover has a wet balance itself (see
/// [`super::planet_model`]). There is no temperature condition. The
/// vapour is bounded by the body's `water_bar` (see [`with_vapour`]), so
/// a trace of water cannot hold its outgoing radiation at the moist limit
/// and does not boil.
pub fn boils(
    context: &Context,
    gases: &Gases,
    water: &Water,
    heat_flux_w_m2: f64,
    water_bar: f64,
) -> bool {
    !water.runaway
        && water.kept_kg > 0.0
        && wet_balance(context, *gases, heat_flux_w_m2, water_bar, 1.0).is_none()
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
    emissivity: f64,
) -> Option<(Gases, f64)> {
    let t_eq = context.equilibrium_k.max(0.0);
    let absorbed = absorbed_w_m2(t_eq, &gases, heat_flux_w_m2);
    let skin = t_eq * 0.5_f64.powf(0.25);
    // A grey surface as in `surface_temperature_k`.
    let e = emissivity.clamp(0.01, 1.0);
    let excess = |t: f64| {
        e * column::olr_under_skin_w_m2(&with_vapour(gases, t, water_bar), t, skin) - absorbed
    };
    // No surface colder than a grey body balances; the vapour only
    // lowers the outgoing radiation, so none colder than the dry
    // temperature either, and the scan from the grey body finds the
    // same first balance.
    let bare = (absorbed / (e * SIGMA)).powf(0.25);
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
/// moist runaway), all of the body's water as vapour (`water_bar`) and
/// the temperature [`surface_temperature_k`] gives it: a placeholder the
/// chain replaces, since the body boils (see [`boils`]).
fn wet_climate(
    context: &Context,
    gases: Gases,
    heat_flux_w_m2: f64,
    water_bar: f64,
    emissivity: f64,
) -> (Gases, f64) {
    wet_balance(context, gases, heat_flux_w_m2, water_bar, emissivity).unwrap_or_else(|| {
        let steam = Gases {
            h2o: water_bar.max(0.0),
            ..gases
        };
        (
            steam,
            surface_temperature_k(context, &steam, heat_flux_w_m2, emissivity),
        )
    })
}

/// Temperature, K, of a surface whose water vapour follows its
/// temperature, as [`climate`] solves it without the thermostat (see
/// [`wet_climate`]); the vapour of `gases` is replaced. The surface
/// radiates with `emissivity` ([`surface_temperature_k`]).
pub(super) fn wet_temperature_k(
    context: &Context,
    gases: &Gases,
    heat_flux_w_m2: f64,
    water_bar: f64,
    emissivity: f64,
) -> f64 {
    wet_climate(context, *gases, heat_flux_w_m2, water_bar, emissivity).1
}

/// Earth's surface gravity, m/s²: 9.81 (standard gravity 9.80665).
pub const EARTH_GRAVITY_M_S2: f64 = 9.81;

/// What the climate reads of the body itself beyond its air and water:
/// its surface gravity, which sets how heavy a column the same surface
/// pressure weighs (the air's Rayleigh scattering and the lapse rate its
/// vapour thins with, [`super::albedo::bond_albedo_over`]), and its
/// radius, over whose area its water spreads into ice
/// ([`super::albedo::cover`]). Issue #523.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Planet {
    /// Surface gravity, m/s².
    pub gravity_m_s2: f64,
    /// Radius, Earth radii.
    pub radius_earth: f64,
}

impl Planet {
    /// Earth: [`EARTH_GRAVITY_M_S2`] and one Earth radius.
    pub const EARTH: Planet = Planet {
        gravity_m_s2: EARTH_GRAVITY_M_S2,
        radius_earth: 1.0,
    };
}

/// What a body's carbon cycle gives its climate beyond its air: the
/// carbon it holds, the CO₂ frost on its ground and the draw of its
/// snowball state; and the SO₂ frost its volcanoes lay, which the
/// climate reads the same way as the CO₂ frost.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CarbonCycle {
    /// CO₂ the body has outgassed, bar, before escape to space and the
    /// cold trap: the carbon inventory weathering and volcanoes trade
    /// with the air ([`super::atmosphere::outgassed`]).
    pub inventory_bar: f64,
    /// CO₂ the cold trap froze out of the air onto the ground before the
    /// climate, bar ([`super::atmosphere::cold_trap`]): a frost where the
    /// surface is cold enough to keep it ([`Body::frost`]), which
    /// brightens the body.
    pub frozen_bar: f64,
    /// SO₂ frost a heat-pipe body's volcanoes lay on its ground, bar
    /// ([`super::atmosphere::volcanic_so2_frost_bar`]): it brightens the
    /// body where its surface keeps it frozen ([`frost`]; Io's in the
    /// model).
    pub so2_frost_bar: f64,
    /// Uniform in `[0, 1]` ([`super::PlanetInputs::snowball_draw`]): a
    /// body with both a temperate and a frozen state is a snowball when
    /// it is below [`snowball_chance`].
    pub snowball_draw: f64,
}

/// The climate of a body and its gases once the climate has set its CO₂
/// and water vapour, and its Bond albedo with them.
///
/// The albedo and the temperature are solved together as a fixed point
/// ([`settle`]): the temperature sets what covers the body
/// ([`super::albedo::cover`]: ice, ocean, frosts, clouds) and whether a
/// sulphuric deck forms, those set the Bond albedo
/// ([`super::albedo::bond_albedo`]), and the albedo the light the climate
/// absorbs ([`Context::with_albedo`]). Under each albedo:
///
/// - In a runaway greenhouse, or without water, the gases are as given
///   (a runaway's steam is already in them) and the temperature is
///   [`surface_temperature_k`]'s. A water runaway absorbs under the
///   albedo of [`absorbing`], which puts the moist plateau at its star's
///   runaway limit.
/// - With water kept, its vapour follows the temperature (77 % of
///   saturation, never more than the `water_bar` all of the body's water
///   would weigh as vapour), solved together with it.
/// - With plates as well, the carbonate thermostat (Walker, Hays & Kasting
///   1981, doi:10.1029/JC086iC10p09776) sets the CO₂ so that the
///   temperature is [`THERMOSTAT_K`], the vapour taken at that
///   temperature. Weathering slows on a cold planet while volcanoes go on,
///   so its CO₂ builds up past what the air held before the climate
///   (Kasting et al. 1993, doi:10.1006/icar.1993.1010; issue #510), up
///   to the carbon inventory (`carbon.inventory_bar`) and at most what
///   does not frost out at the cold trap's temperature
///   ([`super::atmosphere::cold_trap`]): the equilibrium temperature
///   under the reference albedo 0.3 (`context.equilibrium_k`), standing
///   in for the upper troposphere where CO₂ condenses. That cap, not
///   Rayleigh scattering, sets the outer edge of the habitable zone
///   here: without it, the column's CO₂ greenhouse outgrows its
///   scattering (406 K at 1.6 AU round the Sun with Earth's 44.7 bar of
///   carbon), so there is no maximum greenhouse within the inventory.
///   If no CO₂ in that range reaches the thermostat, the planet takes the
///   CO₂ of its warmest climate, the maximum greenhouse
///   ([`maximum_greenhouse`]); if none is still too warm, the planet is
///   hotter. Only a temperature reached is `regulated`.
///
/// Under each albedo too, the gases that condense on the surface
/// ([`condense`]: methane and nitrogen) keep in the air only the vapour
/// of their condensate at the surface temperature, solved together with
/// it (see [`under_albedo`]); the rest lies on the ground, where
/// [`liquid`] sees it and, frozen, brightens the body
/// ([`Body::frost`]). Methane's greenhouse counts only the methane its tropopause lets
/// through and saturates ([`methane_depth`]), so that a cold body under
/// tens of bar of methane stays cold and condenses most of it (issue
/// #443).
///
/// A body that keeps water has two branches ([`branches`]):
///
/// - the temperate one, started warm from a body with no ice: the
///   climate a young planet cools into from its hot start. Where it boils,
///   the body boils (a temperate state it could reach from an icy start
///   is not the one it formed into);
/// - where that one keeps liquid water, the frozen one, started from a
///   body covered in ice, with the CO₂ the temperate one held when it
///   froze: weathering stops under the ice, and volcanic CO₂ builds up
///   until it thaws the planet (Hoffman et al. 2017,
///   doi:10.1126/sciadv.1600983).
///
/// When both exist (the ice–albedo feedback makes the body bistable), it
/// is a snowball when `carbon.snowball_draw` is below
/// [`snowball_chance`]: the share of its history such a planet spends
/// frozen. Its CO₂ is then the amount built up so far, half way
/// ([`SNOWBALL_BUILT_UP`]) from the CO₂ it froze with to the CO₂ that
/// thaws it: one CO₂ per state, so that pinning the state alone
/// ([`super::PlanetModel::formation`]) gives the same planet again. When
/// no CO₂ within what the body can hold thaws it (the chance is then 1),
/// the frozen state is for good: not a [`Climate::snowball`].
///
/// The liquid follows from the temperature and pressure (see
/// [`LAVA_K`] and the phase diagrams of water and hydrocarbons).
///
/// `planet` is the body's gravity and radius ([`Planet`]), which its
/// albedo reads.
#[allow(clippy::too_many_arguments)]
pub fn climate(
    context: &Context,
    gases: Gases,
    water: &Water,
    regime: Regime,
    heat_flux_w_m2: f64,
    water_bar: f64,
    carbon: CarbonCycle,
    planet: &Planet,
) -> (Gases, Climate) {
    climate_over(
        context,
        gases,
        water,
        regime,
        heat_flux_w_m2,
        water_bar,
        carbon,
        planet,
        None,
    )
}

/// [`climate`] of a body whose ice and land are an exposed ice crust of
/// albedo `crust` (`super::crust_albedo`: the area-weighted
/// [`super::albedo::icy_unit_albedo`] of its terrain units) wherever its
/// air is under `interim::ERODING_BAR`; `None` for any other body, whose
/// ice is snow and sea ice and whose land is rock.
#[allow(clippy::too_many_arguments)]
pub fn climate_over(
    context: &Context,
    gases: Gases,
    water: &Water,
    regime: Regime,
    heat_flux_w_m2: f64,
    water_bar: f64,
    carbon: CarbonCycle,
    planet: &Planet,
    crust: Option<f64>,
) -> (Gases, Climate) {
    let body = Body::new(
        context,
        gases,
        water,
        regime,
        heat_flux_w_m2,
        water_bar,
        &carbon,
        planet,
        crust,
    );
    let branches = branches(&body);
    let Some(frozen) = &branches.frozen else {
        return branches.warm;
    };
    let chance = frozen.chance(&body);
    if carbon.snowball_draw >= chance {
        return branches.warm;
    }
    let built_up =
        frozen.froze_bar + SNOWBALL_BUILT_UP * (frozen.still_frozen_bar - frozen.froze_bar);
    let (gases, climate) = body.frozen_at(built_up);
    // A frozen state that no CO₂ the body can hold thaws is frozen for
    // good, not a snowball awaiting its thaw.
    let snowball = frozen_over(&climate) && frozen.thaw_bar.is_finite();
    (
        gases,
        Climate {
            snowball,
            ..climate
        },
    )
}

/// The temperate branch of [`climate`] (the fixed point started warm,
/// never a snowball) and the steps its albedo's fixed point took: a probe
/// of its convergence (`climate_sweep phase`, issue #562), not part of
/// the model's chain.
#[allow(clippy::too_many_arguments)]
pub fn temperate_steps(
    context: &Context,
    gases: Gases,
    water: &Water,
    regime: Regime,
    heat_flux_w_m2: f64,
    water_bar: f64,
    carbon: CarbonCycle,
    planet: &Planet,
) -> (Gases, Climate, usize) {
    let body = Body::new(
        context,
        gases,
        water,
        regime,
        heat_flux_w_m2,
        water_bar,
        &carbon,
        planet,
        None,
    );
    settle_steps(&body, warm_start(&body))
}

/// Whether `climate` is frozen over: no liquid water and a surface under
/// the melting point of ice.
pub(super) fn frozen_over(climate: &Climate) -> bool {
    climate.liquid != SurfaceLiquid::Water && climate.temperature_k < ICE_MELTS_K
}

/// Share of the way from the CO₂ a snowball froze with to the CO₂ that
/// thaws it that a snowball holds ([`climate`]): a half, the mean of a
/// build-up at a steady rate seen at a uniform time. Not drawn, so that a
/// snowball pinned by its state alone (`InputOverrides::snowball`, which
/// sets the draw to 0) holds the CO₂ it was drawn with.
const SNOWBALL_BUILT_UP: f64 = 0.5;

/// The share of its history a planet whose climate has both a temperate
/// and a frozen state spends frozen: a snowball lasts until volcanoes
/// have given it the `thaw_bar` of CO₂ (bar) that melts it, at the rate
/// of its volcanism ([`super::interim::volcanism`] of `heat_flux_w_m2`
/// and `regime`, against Earth's), and the temperate phase between two
/// snowballs lasts as long as Earth's volcanism takes to give
/// [`SNOWBALL_WARM_BAR`]:
///
/// ```text
/// chance = t_frozen / (t_frozen + t_warm) = d / (d + B · v / v⊕)
/// ```
///
/// with `d` the CO₂ to thaw, `B` [`SNOWBALL_WARM_BAR`] and `v / v⊕` the
/// volcanism against Earth's. Rare near Earth's orbit, where a little
/// CO₂ thaws the ice; likelier towards the outer edge of the habitable
/// zone, where more is needed, and on a planet that outgasses little (a
/// stagnant lid, a low heat flux); 1 when the CO₂ cannot thaw it
/// (`thaw_bar` infinite) or nothing outgasses.
pub fn snowball_chance(thaw_bar: f64, heat_flux_w_m2: f64, regime: Regime) -> f64 {
    let earth = super::interim::volcanism(super::interim::EARTH_HEAT_FLUX_W_M2, Regime::Plates);
    let rate = super::interim::volcanism(heat_flux_w_m2, regime) / earth;
    if !thaw_bar.is_finite() || rate <= 0.0 {
        return 1.0;
    }
    let d = thaw_bar.max(0.0);
    d / (d + SNOWBALL_WARM_BAR * rate)
}

/// CO₂, bar, that Earth's volcanism gives the air over one temperate
/// phase between two snowballs ([`snowball_chance`]). Calibration on
/// Earth, the only planet whose snowballs are known: it spent about a
/// tenth of its history in snowball states (the Sturtian and Marinoan
/// glaciations, some 70 Myr, and the Palaeoproterozoic ones; Hoffman et
/// al. 2017, doi:10.1126/sciadv.1600983, from memory, not checked), and
/// the model's Earth at 1 AU needs about 0.09 bar of CO₂ more to thaw
/// than it froze with, so 0.8 bar gives it a chance of 0.1.
const SNOWBALL_WARM_BAR: f64 = 0.8;

/// What [`climate`] solves for: the body's context, its air before the
/// climate, its water, its regime, its internal heat flux (W/m²) and the
/// bar its water would weigh as vapour; the most CO₂ its carbon cycle can
/// put in the air; and the CO₂ the frozen branch holds (`None`: the
/// thermostat's, or the air's).
struct Body<'a> {
    context: &'a Context,
    gases: Gases,
    water: &'a Water,
    regime: Regime,
    heat_flux_w_m2: f64,
    water_bar: f64,
    /// The most CO₂ the air can hold, bar: the carbon inventory, at most
    /// what does not frost out at the cold trap's temperature (the
    /// equilibrium temperature under the reference albedo 0.3, standing
    /// in for the upper troposphere; see [`climate`]), and never less
    /// than the air already holds.
    most_co2: f64,
    /// CO₂ the cold trap froze onto the ground, bar
    /// ([`CarbonCycle::frozen_bar`]).
    frozen_co2: f64,
    /// SO₂ frost its volcanoes lay on the ground, bar
    /// ([`CarbonCycle::so2_frost_bar`]).
    so2_frost: f64,
    co2: Option<f64>,
    /// The CO₂ of the maximum greenhouse, once found.
    greenhouse: std::cell::OnceCell<f64>,
    /// Its gravity and radius ([`Planet`]).
    planet: &'a Planet,
    /// Albedo of its exposed ice crust, if it has one ([`climate_over`]).
    crust: Option<f64>,
}

impl<'a> Body<'a> {
    #[allow(clippy::too_many_arguments)]
    fn new(
        context: &'a Context,
        gases: Gases,
        water: &'a Water,
        regime: Regime,
        heat_flux_w_m2: f64,
        water_bar: f64,
        carbon: &CarbonCycle,
        planet: &'a Planet,
        crust: Option<f64>,
    ) -> Self {
        let condenses = super::atmosphere::co2_frost_bar(context.equilibrium_k);
        Body {
            context,
            gases,
            water,
            regime,
            heat_flux_w_m2,
            water_bar,
            most_co2: carbon.inventory_bar.min(condenses).max(gases.co2).max(0.0),
            frozen_co2: carbon.frozen_bar.max(0.0),
            so2_frost: carbon.so2_frost_bar.max(0.0),
            co2: None,
            greenhouse: std::cell::OnceCell::new(),
            planet,
            crust,
        }
    }

    /// Whether the body keeps water out of a runaway.
    fn wet(&self) -> bool {
        !self.water.runaway && self.water.kept_kg > 0.0
    }

    /// Frost on the ground of the body at `temperature_k` under `air`,
    /// bar, gas by gas ([`frost`]).
    fn frost(&self, temperature_k: f64, air: &Gases) -> Gases {
        frost(
            &self.gases,
            self.frozen_co2,
            self.so2_frost,
            temperature_k,
            air,
        )
    }

    /// [`derived_albedo`] of the body at `temperature_k` under `air`,
    /// with its frost there ([`Body::frost`]), and its ice crust's albedo
    /// where its air is under `interim::ERODING_BAR` (a thicker air
    /// erodes and buries the ice, `surface_age::terrain`).
    fn albedo_at(&self, temperature_k: f64, air: &Gases) -> f64 {
        let crust = self
            .crust
            .filter(|_| air.total() < super::interim::ERODING_BAR);
        derived_albedo_over(
            temperature_k,
            air,
            self.water,
            &self.frost(temperature_k, air),
            self.planet,
            crust,
        )
    }

    /// [`emissivity`] of the body at `temperature_k` under `air`, with
    /// its frost there ([`Body::frost`]).
    fn emissivity_at(&self, temperature_k: f64, air: &Gases) -> f64 {
        emissivity(&self.frost(temperature_k, air))
    }

    /// The frozen branch with `co2` bar of CO₂ in the air: the fixed point
    /// started from a body covered in ice ([`ICE_COVERED_K`]).
    fn frozen_at(&self, co2: f64) -> (Gases, Climate) {
        let body = Body {
            co2: Some(co2),
            gases: Gases { co2, ..self.gases },
            greenhouse: std::cell::OnceCell::new(),
            ..*self
        };
        let start = (
            body.albedo_at(ICE_COVERED_K, &body.gases),
            body.emissivity_at(ICE_COVERED_K, &body.gases),
        );
        settle(&body, start)
    }
}

/// Width, K, of the band under a phase change over which a condensate
/// passes from its frozen phase to its liquid one in the climate
/// ([`frozen_share`]): a numerical smoothing, not physics. At the step
/// itself the climate's fixed points have no solution and cycle (issue
/// #562): nitrogen frost radiates at [`N2_ICE_EMISSIVITY`] and its liquid
/// as a black body, so a surface just under the triple point warms over
/// it and one just over it cools under it; methane's air thins by half
/// over its sea ([`SEA_METHANE_SHARE`]) and its frost darkens into a
/// sea. 1 K: the narrowest band over which the albedo's damped fixed
/// point ([`settle`], [`ALBEDO_DAMPING`]) settles at the triple point,
/// where the emissivity's 0.89-to-1 step moves the surface by about
/// 1.9 K (`ε^-¼` at 63 K): over a band of width `w` its loop gain is
/// about `1.8 K / w`, and the damped step converges under a gain of 3,
/// so from about 0.6 K (`climate_sweep phase` shows the cycles under a
/// step). A planet settling in the band is shifted by at most its width
/// from where the step would put it.
pub(crate) const PHASE_BAND_K: f64 = 1.0;

/// Share of a condensate frozen at `temperature_k` near the point
/// `melts_k` it melts at: 1 at and under `melts_k - PHASE_BAND_K`, 0 at
/// and above `melts_k` (so that above it nothing changes), linear between
/// ([`PHASE_BAND_K`], a numerical smoothing).
pub(crate) fn frozen_share(temperature_k: f64, melts_k: f64) -> f64 {
    ((melts_k - temperature_k) / PHASE_BAND_K).clamp(0.0, 1.0)
}

/// The frost, bar, left of `condensate` (bar) with the share `frozen` of
/// it frozen ([`frozen_share`]): all of it when all is frozen; in the
/// band where it melts, that share of the most that counts towards its
/// cover ([`super::albedo::FROST_LAYER_BAR`]), so that its cover, and
/// with it the albedo and the emissivity, retreats across the whole band
/// rather than at its warm end, where a thick frost would only start to
/// uncover the ground.
fn melting(condensate: f64, frozen: f64) -> f64 {
    let condensate = condensate.max(0.0);
    if frozen >= 1.0 {
        condensate
    } else {
        frozen * condensate.min(super::albedo::FROST_LAYER_BAR)
    }
}

/// Frost on the ground of a body at `temperature_k` under `air`, bar,
/// gas by gas, from its air before the climate (`gases`):
///
/// - the nitrogen it condenses there ([`condense`]) under nitrogen's
///   triple point (above it, a liquid), and the methane it condenses
///   under the freezing point of its sea ([`sea_freezes_k`]; above it, a
///   sea), each melting over the last [`PHASE_BAND_K`] under its point
///   ([`frozen_share`]);
/// - the CO₂ the cold trap froze out (`frozen_co2`,
///   [`CarbonCycle::frozen_bar`]) that the surface keeps frozen: what the
///   air, at most CO₂'s sublimation pressure there, cannot take back (none
///   above CO₂'s triple point);
/// - the SO₂ frost its volcanoes lay (`so2_frost`,
///   [`CarbonCycle::so2_frost_bar`]), less what the air takes back the
///   same way at SO₂'s sublimation pressure.
pub(crate) fn frost(
    gases: &Gases,
    frozen_co2: f64,
    so2_frost: f64,
    temperature_k: f64,
    air: &Gases,
) -> Gases {
    use super::atmosphere::{co2_frost_bar, so2_frost_bar};
    let condensate = condense(*gases, temperature_k).1;
    let n2 = melting(
        condensate.n2,
        frozen_share(temperature_k, NITROGEN_TRIPLE_K),
    );
    let ch4 = melting(
        condensate.ch4,
        frozen_share(temperature_k, sea_freezes_k(air.n2, &condensate)),
    );
    let kept = |frozen: f64, sublimes: f64, in_air: f64| {
        (frozen.max(0.0) - (sublimes - in_air.max(0.0)).max(0.0)).max(0.0)
    };
    Gases {
        n2,
        ch4,
        co2: kept(frozen_co2, co2_frost_bar(temperature_k), air.co2),
        so2: kept(so2_frost, so2_frost_bar(temperature_k), air.so2),
        ..Gases::default()
    }
}

/// Mean surface temperature, K, at and under which a body's water is
/// all ice ([`super::albedo::cover`]): where the frozen branch starts.
const ICE_COVERED_K: f64 = super::albedo::ICE_COVERED_K;

/// Bisection steps of the CO₂ that thaws a snowball ([`branches`]), in
/// `ln CO₂`: 2⁻¹² of the range, under a thousandth of the CO₂ for a range
/// of a few decades.
const THAW_STEPS: usize = 12;

/// The branches of a body's climate ([`climate`]).
struct Branches {
    /// The temperate branch (or the only one).
    warm: (Gases, Climate),
    /// The frozen branch, where the body is bistable.
    frozen: Option<FrozenBranch>,
}

/// A frozen branch beside a temperate one.
struct FrozenBranch {
    /// CO₂ the temperate branch held when it froze, bar.
    froze_bar: f64,
    /// The most CO₂ known to leave the body frozen, bar: just under the
    /// thaw, or all the air can hold when that never thaws it.
    still_frozen_bar: f64,
    /// The least CO₂ known to thaw it, bar; infinite when none does.
    thaw_bar: f64,
}

impl FrozenBranch {
    /// [`snowball_chance`] for this branch of `body`.
    fn chance(&self, body: &Body) -> f64 {
        snowball_chance(
            self.thaw_bar - self.froze_bar,
            body.heat_flux_w_m2,
            body.regime,
        )
    }
}

/// The temperate branch of `body` and, where it keeps liquid water and
/// the same CO₂ also leaves the body frozen from an icy start, the frozen
/// one with the CO₂ that thaws it.
fn branches(body: &Body) -> Branches {
    let warm = settle(body, warm_start(body));
    if !body.wet() || warm.1.liquid != SurfaceLiquid::Water {
        return Branches { warm, frozen: None };
    }
    let froze_bar = warm.0.co2;
    let thawed = |co2: f64| !frozen_over(&body.frozen_at(co2).1);
    if thawed(froze_bar) {
        return Branches { warm, frozen: None };
    }
    let most = body.most_co2;
    let frozen = if most <= froze_bar || !thawed(most) {
        FrozenBranch {
            froze_bar,
            still_frozen_bar: most.max(froze_bar),
            thaw_bar: f64::INFINITY,
        }
    } else {
        // Frozen at `low`, thawed at `high`: bisect in `ln CO₂`.
        let (mut low, mut high) = (froze_bar.max(most * 1e-12), most);
        for _ in 0..THAW_STEPS {
            let mid = (low * high).sqrt();
            if thawed(mid) {
                high = mid;
            } else {
                low = mid;
            }
        }
        FrozenBranch {
            froze_bar,
            still_frozen_bar: low,
            thaw_bar: high,
        }
    };
    Branches {
        warm,
        frozen: Some(frozen),
    }
}

/// Most steps of the albedo's fixed point ([`settle`]): far more than
/// the ten or so it takes to settle, even from no ice to a snowball.
const ALBEDO_ITERATIONS: usize = 200;

/// The temperature step, K, under which the albedo's fixed point has
/// settled ([`settle`]).
const ALBEDO_TOLERANCE_K: f64 = 0.1;

/// Share of the way from the albedo used to the albedo derived that each
/// step of [`settle`] goes: damping, so that a climate on the edge of
/// the ice–albedo feedback does not swing between a frozen and an
/// ice-free cover.
const ALBEDO_DAMPING: f64 = 0.5;

/// One solve of [`under_albedo`].
struct Pass {
    gases: Gases,
    /// Surface temperature, K.
    temperature_k: f64,
    /// The thermostat holds it.
    regulated: bool,
    /// The context it absorbed its light in.
    context: Context,
    /// The condensation's fixed point settled within [`TOLERANCE_K`].
    converged: bool,
}

/// The climate of `body` under the derived Bond albedo `albedo`, before
/// the runaway blend ([`absorbing`]), its surface radiating with
/// `emissivity`: its gases, its temperature, whether the thermostat holds
/// it, the context it absorbed its light in, and whether the
/// condensation of its air settled.
fn under_albedo(body: &Body, albedo: f64, emissivity: f64) -> Pass {
    let context = absorbing(body.context, body.water, body.gases.h2o, albedo);
    let (flux, water_bar) = (body.heat_flux_w_m2, body.water_bar);
    let under = |air: Gases| {
        if !body.wet() {
            let t = surface_temperature_k(&context, &air, flux, emissivity);
            (air, t, false)
        } else if body.regime == Regime::Plates && body.co2.is_none() {
            if air == body.gases {
                thermostat(body, &context, emissivity)
            } else {
                let condensed = Body {
                    gases: air,
                    greenhouse: std::cell::OnceCell::new(),
                    ..*body
                };
                thermostat(&condensed, &context, emissivity)
            }
        } else {
            let (g, t) = wet_climate(&context, air, flux, water_bar, emissivity);
            (g, t, false)
        }
    };
    // The gases that condense on the surface ([`condense`]) keep in the
    // air what its temperature allows, and the temperature follows from
    // that air: from the black body's (the coldest surface but for the
    // methane's anti-greenhouse, which a step down undoes), each step
    // solves the climate with the air the last temperature leaves, which
    // rises towards the coldest balance (more air, more greenhouse). Most
    // bodies condense nothing at their black body's temperature, and
    // nothing at a warmer one either: one step. Where the air overshoots
    // (the step crossed a phase change that thins it, as methane's sea
    // freezing), the steps would swing across it: the temperature is
    // solved for in the bracket the last two steps make.
    let absorbed = SIGMA * context.equilibrium_k.max(0.0).powi(4) + flux.max(0.0);
    let mut t = (absorbed / (emissivity.clamp(0.01, 1.0) * SIGMA)).powf(0.25);
    let mut air = condense(body.gases, t).0;
    let mut pass = under(air);
    if air == body.gases {
        return Pass {
            gases: pass.0,
            temperature_k: pass.1,
            regulated: pass.2,
            context,
            converged: true,
        };
    }
    let mut converged = false;
    let mut last: Option<Point> = None;
    for _ in 0..CONDENSE_ITERATIONS {
        // How far the temperature assumed is above the one its air gives.
        let here = Point {
            x: t,
            f: t - pass.1,
        };
        if here.f.abs() < TOLERANCE_K {
            converged = true;
            break;
        }
        if let Some(before) = last.filter(|b| (b.f < 0.0) != (here.f < 0.0)) {
            let (low, high) = if here.f < 0.0 {
                (here, before)
            } else {
                (before, here)
            };
            if low.x < high.x {
                let excess = |x: f64| x - under(condense(body.gases, x).0).1;
                t = solve(excess, low, high, BRACKET_TOLERANCE_K);
                air = condense(body.gases, t).0;
                pass = under(air);
                converged = (pass.1 - t).abs() < TOLERANCE_K;
                break;
            }
        }
        last = Some(here);
        t = pass.1;
        let next = condense(body.gases, t).0;
        if next == air {
            converged = true;
            break;
        }
        air = next;
        pass = under(air);
    }
    Pass {
        gases: condensed_air(pass.0, &body.gases, pass.1),
        temperature_k: pass.1,
        regulated: pass.2,
        context,
        converged,
    }
}

/// Width, K, of the bracket the condensation's fixed point in
/// [`under_albedo`] solves its temperature in when its steps overshoot:
/// a hundredth of [`TOLERANCE_K`], so that the temperature the air
/// gives, which moves faster than the one assumed across a phase change,
/// lands within [`TOLERANCE_K`] of it.
const BRACKET_TOLERANCE_K: f64 = TOLERANCE_K * 1e-2;

/// Most steps of the condensation's fixed point in [`under_albedo`]:
/// far more than the few it takes, the greenhouse of what condenses
/// being saturated ([`methane_depth`]).
const CONDENSE_ITERATIONS: usize = 100;

/// The Bond albedo of `water` and `gases` at `temperature_k` with
/// `frost_bar` of frost on the ground, with Earth's gravity and radius
/// ([`Planet::EARTH`]): its [`cover`], with the sulphuric
/// deck [`sulphuric_deck`] makes of the SO₂ the washing out leaves
/// ([`washed`]).
#[cfg(test)]
pub(crate) fn derived_albedo(
    temperature_k: f64,
    gases: &Gases,
    water: &Water,
    frost: &Gases,
) -> f64 {
    derived_albedo_over(temperature_k, gases, water, frost, &Planet::EARTH, None)
}

/// [`derived_albedo`] on `planet` over an exposed ice crust of albedo
/// `crust`, if any ([`super::albedo::bond_albedo_over`]).
fn derived_albedo_over(
    temperature_k: f64,
    gases: &Gases,
    water: &Water,
    frost: &Gases,
    planet: &Planet,
    crust: Option<f64>,
) -> f64 {
    let deck = sulphuric_deck(temperature_k, washed(*gases, water).so2);
    bond_albedo_over(
        &cover(temperature_k, gases, water, frost, planet),
        gases,
        deck,
        crust,
        planet,
    )
}

/// `gases` after the washing out of their SO₂
/// ([`super::atmosphere::wash_out`]) on a body with `water`: as over
/// liquid water when the body keeps water out of a runaway, liquid or not
/// at its temperature (a frozen one is too cold for a deck anyway, and
/// one with no wet balance boils: its steam is the chain's next pass, not
/// a deck). The albedo washes with it and the model stores its air washed
/// with it, so that the look's deck is the albedo's.
pub(crate) fn washed(gases: Gases, water: &Water) -> Gases {
    let liquid = if water.is_wet() {
        SurfaceLiquid::Water
    } else {
        SurfaceLiquid::None
    };
    super::atmosphere::wash_out(gases, liquid)
}

/// The albedo and emissivity the fixed point of `body` starts from warm:
/// those of the cover of the climate under [`REFERENCE_ALBEDO`], at least
/// as warm as a body with no ice ([`super::albedo::ICE_FREE_K`]), so that
/// it settles on the warmest climate. A wet body's air holds the vapour
/// its water keeps at that temperature ([`with_vapour`]): its clouds
/// follow their humidity, and the vapour of a colder surface in a warmer
/// air would start the fixed point under too thin a cloud cover.
fn warm_start(body: &Body) -> (f64, f64) {
    let pass = under_albedo(body, REFERENCE_ALBEDO, 1.0);
    let warm = pass.temperature_k.max(super::albedo::ICE_FREE_K);
    let air = if body.wet() {
        with_vapour(pass.gases, warm, body.water_bar)
    } else {
        pass.gases
    };
    (body.albedo_at(warm, &air), body.emissivity_at(warm, &air))
}

/// The fixed point of the albedo, the emissivity and the climate of
/// `body` from the derived albedo and emissivity `start`: each step
/// solves the climate under them ([`under_albedo`]) and moves each
/// [`ALBEDO_DAMPING`] of the way to the one its temperature derives (half
/// as far after each step that turns back, so that a swing settles),
/// until the temperature moves less than [`ALBEDO_TOLERANCE_K`] and the
/// derived albedo and emissivity would move the surface's radiative
/// equilibrium (`T_eq ε^-¼`) less than that too. The climate keeps the
/// albedo and emissivity it was solved under, so that it reproduces. It
/// is not a snowball: [`climate`] marks the frozen branch it draws. It
/// is [`Climate::converged`] when this fixed point and the last
/// condensation in it ([`under_albedo`]) settled.
fn settle(body: &Body, start: (f64, f64)) -> (Gases, Climate) {
    let (gases, climate, _) = settle_steps(body, start);
    (gases, climate)
}

/// [`settle`], with the steps its fixed point took.
fn settle_steps(body: &Body, start: (f64, f64)) -> (Gases, Climate, usize) {
    let radiative_k = |albedo: f64, emissivity: f64| {
        body.context.with_albedo(albedo).equilibrium_k / emissivity.clamp(0.01, 1.0).powf(0.25)
    };
    let (mut albedo, mut emissivity) = start;
    let mut last = f64::NAN;
    let mut pass = under_albedo(body, albedo, emissivity);
    let mut settled = false;
    let mut steps = 0;
    let mut damping = ALBEDO_DAMPING;
    let mut moved = 0.0;
    while steps < ALBEDO_ITERATIONS {
        let t = pass.temperature_k;
        let derived = body.albedo_at(t, &pass.gases);
        let derived_emissivity = body.emissivity_at(t, &pass.gases);
        // Settled when the temperature has stopped moving and the albedo
        // it derives would not move the light by more either (a
        // thermostat holds the temperature whatever the albedo).
        let residual_k =
            (radiative_k(derived, derived_emissivity) - radiative_k(albedo, emissivity)).abs();
        if (t - last).abs() < ALBEDO_TOLERANCE_K && residual_k < ALBEDO_TOLERANCE_K {
            settled = true;
            break;
        }
        // A step that turns back the way the last one came overshot: the
        // fixed point is between them, and the next steps go half as far.
        let step = t - last;
        if step * moved < 0.0 {
            damping *= 0.5;
        }
        moved = step;
        last = t;
        albedo += damping * (derived - albedo);
        emissivity += damping * (derived_emissivity - emissivity);
        pass = under_albedo(body, albedo, emissivity);
        steps += 1;
    }
    let Pass {
        gases,
        temperature_k,
        regulated,
        context,
        converged,
    } = pass;
    let condensate = condense(body.gases, temperature_k).1;
    let liquid = liquid(temperature_k, &gases, body.water, &condensate);
    (
        gases,
        Climate {
            temperature_k,
            equilibrium_k: context.equilibrium_k,
            bond_albedo: runaway_albedo(body.context, body.water, body.gases.h2o, albedo),
            liquid,
            emissivity,
            regulated,
            snowball: false,
            converged: settled && converged,
        },
        steps,
    )
}

/// The carbonate thermostat of `body` under the light of `context`: the
/// CO₂ in `[0, most_co2]` that puts the surface at [`THERMOSTAT_K`],
/// solved in `ln CO₂` (see [`climate`]). Returns the gases, the
/// temperature and whether it was reached. Short of the thermostat with
/// all the CO₂ the air can hold, it takes the [`maximum_greenhouse`].
/// The surface radiates with `emissivity` ([`surface_temperature_k`]):
/// 1 wherever the thermostat holds, far above where nitrogen frosts, but
/// not on a cold body with plates that it cannot reach.
fn thermostat(body: &Body, context: &Context, emissivity: f64) -> (Gases, f64, bool) {
    let (gases, heat_flux_w_m2, water_bar) = (body.gases, body.heat_flux_w_m2, body.water_bar);
    let most = body.most_co2;
    let at_target = with_vapour(gases, THERMOSTAT_K, water_bar);
    let t_eq = context.equilibrium_k.max(0.0);
    let absorbed = absorbed_w_m2(t_eq, &gases, heat_flux_w_m2);
    let skin = t_eq * 0.5_f64.powf(0.25);
    // With `co2` in the air the surface is warmer than the thermostat
    // when, at `THERMOSTAT_K`, it sends out less than it absorbs (the
    // outgoing radiation rises with the surface temperature).
    let e = emissivity.clamp(0.01, 1.0);
    let olr_at =
        |co2: f64| e * column::olr_under_skin_w_m2(&Gases { co2, ..at_target }, THERMOSTAT_K, skin);
    let all = absorbed - olr_at(most);
    if all <= 0.0 {
        let co2 = *body.greenhouse.get_or_init(|| maximum_greenhouse(body));
        let (g, t) = wet_climate(
            context,
            Gases { co2, ..gases },
            heat_flux_w_m2,
            water_bar,
            emissivity,
        );
        return (g, t, false);
    }
    if olr_at(0.0) < absorbed {
        let (g, t) = wet_climate(
            context,
            Gases { co2: 0.0, ..gases },
            heat_flux_w_m2,
            water_bar,
            emissivity,
        );
        return (g, t, false);
    }
    // The CO₂ at which a surface at `THERMOSTAT_K` balances, solved in
    // `ln CO₂` from a trace (2⁻⁶⁰ of the most, the resolution of the
    // bisection it replaces) up to all of it.
    let deficit = |ln_co2: f64| absorbed - olr_at(ln_co2.exp());
    let floor = most * 2f64.powi(-60);
    let low = Point {
        x: floor.ln(),
        f: deficit(floor.ln()),
    };
    let high = if low.f >= 0.0 {
        floor
    } else {
        let top = Point {
            x: most.ln(),
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
        surface_temperature_k(context, &regulated, heat_flux_w_m2, emissivity),
        true,
    )
}

/// Decades of CO₂ under the most the air can hold that
/// [`maximum_greenhouse`] searches.
const GREENHOUSE_DECADES: f64 = 4.0;

/// Golden-section steps of [`maximum_greenhouse`]: 0.618⁴⁰ of its range,
/// some 1e-8 decades.
const GREENHOUSE_STEPS: usize = 40;

/// The CO₂, bar, of the warmest climate of `body` within its
/// `most_co2` (Kasting et al. 1993, doi:10.1006/icar.1993.1010, the
/// maximum greenhouse): the amount at which a surface at
/// [`THERMOSTAT_K`] gains most, the light it absorbs under the albedo
/// that CO₂ derives (its Rayleigh scattering) against what its column
/// sends out. Golden-section search in `ln CO₂` over
/// [`GREENHOUSE_DECADES`] under the most; with the column of
/// [`super::column`] the CO₂'s greenhouse outgrows its scattering, so it
/// is the most the air can hold in practice.
fn maximum_greenhouse(body: &Body) -> f64 {
    let most = body.most_co2;
    if most <= 0.0 {
        return 0.0;
    }
    let at_target = with_vapour(body.gases, THERMOSTAT_K, body.water_bar);
    let gain = |ln_co2: f64| {
        let gases = Gases {
            co2: ln_co2.exp(),
            ..at_target
        };
        let albedo = body.albedo_at(THERMOSTAT_K, &gases);
        let t_eq = body.context.with_albedo(albedo).equilibrium_k.max(0.0);
        let skin = t_eq * 0.5_f64.powf(0.25);
        absorbed_w_m2(t_eq, &gases, 0.0) - column::olr_under_skin_w_m2(&gases, THERMOSTAT_K, skin)
    };
    let ratio = (5f64.sqrt() - 1.0) / 2.0;
    let (mut a, mut b) = (most.ln() - GREENHOUSE_DECADES * 10f64.ln(), most.ln());
    let (mut c, mut d) = (b - ratio * (b - a), a + ratio * (b - a));
    let (mut fc, mut fd) = (gain(c), gain(d));
    for _ in 0..GREENHOUSE_STEPS {
        if fc < fd {
            a = c;
            (c, fc) = (d, fd);
            d = a + ratio * (b - a);
            fd = gain(d);
        } else {
            b = d;
            (d, fd) = (c, fc);
            c = b - ratio * (b - a);
            fc = gain(c);
        }
    }
    // The end of the range when the gain still grows there.
    let best = if gain(most.ln()) >= fc.max(fd) {
        most.ln()
    } else {
        (a + b) / 2.0
    };
    best.exp().min(most)
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
            d.carbon,
            &d.planet,
        );
        (d, g, c)
    }

    /// A carbon cycle of `inventory_bar` that keeps the temperate state.
    fn temperate(inventory_bar: f64) -> CarbonCycle {
        CarbonCycle {
            inventory_bar,
            frozen_bar: 0.0,
            so2_frost_bar: 0.0,
            snowball_draw: 1.0,
        }
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
    fn the_solar_system_has_its_albedos() {
        let albedo = |name: &str| {
            let (_, g, c) = climate_of(name);
            println!(
                "{name}: Bond {:.3}, {:.1} K, {:.3e} bar",
                c.bond_albedo,
                c.temperature_k,
                g.total()
            );
            c.bond_albedo
        };
        let earth = albedo("Earth");
        assert!((0.28..=0.32).contains(&earth), "Earth {earth}");
        let venus = albedo("Venus");
        assert!((0.70..=0.80).contains(&venus), "Venus {venus}");
        let mars = albedo("Mars");
        assert!((0.20..=0.30).contains(&mars), "Mars {mars}");
        let mercury = albedo("Mercury");
        assert!((0.06..=0.14).contains(&mercury), "Mercury {mercury}");
        // Titan's and the icy moons' albedos are the frosts' (Task 6 of
        // the lot): shown, not checked here.
        albedo("Titan");
        albedo("Europa");
        // The equilibrium temperature the climate used is the context's
        // under that albedo.
        let (d, _, c) = climate_of("Venus");
        let expected = d.context.with_albedo(c.bond_albedo).equilibrium_k;
        assert!((c.equilibrium_k - expected).abs() < 1e-9);
    }

    #[test]
    fn earths_cloud_cover_stays_two_thirds() {
        // The cover the model's Earth settles under: 0.67 ± 0.03 (ISCCP
        // and MODIS, Stubenrauch et al. 2013), issue #523.
        let (d, g, c) = climate_of("Earth");
        let cloud = super::super::albedo::cover(
            c.temperature_k,
            &g,
            &d.water,
            &Gases::default(),
            &d.planet,
        )
        .cloud;
        println!("Earth's cloud cover {cloud:.4} at {:.1} K", c.temperature_k);
        assert!((cloud - 0.67).abs() <= 0.03, "{cloud}");
    }

    #[test]
    fn a_runaway_blends_from_the_derived_albedo() {
        // Review Focus 2: a runaway's albedo moves from its derived one
        // (here a sulphuric deck's) to the moist `A_rg` with its steam,
        // with no jump where the last of the steam goes: the equilibrium
        // just inside the runaway (a trace of steam) and just outside it
        // (none) differ by under 2 K.
        let context = crate::planet::reference::sun_at(0.72);
        let steam = Water {
            runaway: true,
            boiled: false,
            kept_kg: 1e20,
            oceans: 1e20 / super::super::OCEAN_KG,
        };
        let deck = 0.76;
        let t_eq = |water: &Water, bar: f64| absorbing(&context, water, bar, deck).equilibrium_k;
        let dry = t_eq(&steam.keeping(0.0), 0.0);
        assert_eq!(dry, context.with_albedo(deck).equilibrium_k);
        let trace = t_eq(&steam.keeping(1e15), 1e-3);
        assert!((trace - dry).abs() < 2.0, "{trace} vs {dry}");
        // A thick column takes the moist albedo whole, whatever it
        // started from.
        let thick = runaway_albedo(&context, &steam, 1e4, deck);
        let from_earth = runaway_albedo(&context, &steam, 1e4, REFERENCE_ALBEDO);
        assert!((thick - from_earth).abs() < 1e-9, "{thick} vs {from_earth}");
        // Through the whole climate: the stored albedo of a runaway with
        // steam is the blend, and it reproduces its equilibrium.
        let water = 0.01;
        let earth = PlanetInputs {
            volatiles: crate::planet::inputs::volatile_rule(1.0 - water, water, 0.325),
            ..crate::planet::reference::inputs(1.0, 0.325, water)
        };
        let (d, g, c) = climate_at(&earth, 0.9);
        assert!(d.water.runaway && g.h2o > 1.0, "{:?} {g:?}", d.water);
        let expected = d.context.with_albedo(c.bond_albedo).equilibrium_k;
        assert!((c.equilibrium_k - expected).abs() < 1e-9, "{c:?}");
    }

    /// Seed 182's planet d in the census (issue #443): a stagnant-lid
    /// water world at 10.5 AU from an F star, its equilibrium temperature
    /// 114 K, under 4.4 bar of N₂ and 26 bar of methane.
    fn cold_methane_world() -> (Context, Gases, Water) {
        let context = Context {
            star_mass: 1.41,
            metallicity: -0.34,
            orbit_au: 10.5,
            eccentricity: 0.02,
            ice_line_au: 5.67,
            insolation: 0.0401,
            equilibrium_k: 114.07,
            age_gyr: 1.457,
            teff_k: 6710.0,
            luminosity: 4.4,
            tide: None,
        };
        let gases = Gases {
            n2: 4.379,
            co2: 1.14e-5,
            ch4: 26.26,
            so2: 2.4e-4,
            ..Gases::default()
        };
        let water = Water {
            runaway: false,
            boiled: false,
            kept_kg: 1.29e23,
            oceans: 92.2,
        };
        (context, gases, water)
    }

    #[test]
    fn a_methane_greenhouse_cannot_warm_a_cold_world() {
        let (context, gases, water) = cold_methane_world();
        let (air, c) = climate(
            &context,
            gases,
            &water,
            Regime::Stagnant,
            0.0283,
            19_065.0,
            temperate(160.9),
            &Planet::EARTH,
        );
        assert!(c.temperature_k < 150.0, "{c:?} {air:?}");
        assert_ne!(c.liquid, SurfaceLiquid::Water, "{c:?}");
        // Most of its methane condenses on the ground.
        assert!(air.ch4 < 0.5 * gases.ch4, "{air:?}");
    }

    #[test]
    fn methane_greenhouse_saturates() {
        let (d, _, titan) = climate_of("Titan");
        let context = d.context.with_albedo(titan.bond_albedo);
        let t = |ch4: f64| {
            let gases = Gases {
                n2: 1.45,
                ch4,
                ..Gases::default()
            };
            surface_temperature_k(&context, &gases, 0.0, 1.0)
        };
        // The greenhouse alone, at Titan's surface temperature: the
        // outgoing radiation methane holds back from 1 to 10 bar is under
        // a third of what it holds back from 0.01 to 0.1 bar.
        let skin = context.equilibrium_k * 0.5_f64.powf(0.25);
        let olr = |ch4: f64| {
            let gases = Gases {
                n2: 1.45,
                ch4,
                ..Gases::default()
            };
            column::olr_under_skin_w_m2(&gases, 93.7, skin)
        };
        let trace = olr(0.01) - olr(0.1);
        let thick = olr(1.0) - olr(10.0);
        assert!(trace > 0.0, "{trace}");
        assert!(thick < trace / 3.0, "{thick} W/m² against {trace} W/m²");
        // With the anti-greenhouse of the methane let through the cold
        // trap, which fills up between 0.01 and 0.1 bar, the warming from
        // 1 to 10 bar stays under that from 0.01 to 0.1 bar.
        let trace = t(0.1) - t(0.01);
        let thick = t(10.0) - t(1.0);
        assert!(trace > 0.0, "{trace}");
        assert!(thick < trace, "{thick} K against {trace} K");
    }

    #[test]
    fn a_trace_of_methane_makes_no_lake() {
        let dry = Water {
            runaway: false,
            boiled: false,
            kept_kg: 0.0,
            oceans: 0.0,
        };
        let none = Gases::default();
        let trace = Gases {
            n2: 1.5,
            ch4: 0.003,
            ..Gases::default()
        };
        assert_ne!(
            liquid(94.0, &trace, &dry, &none),
            SurfaceLiquid::Hydrocarbons
        );
        // Titan's air holds the vapour of its seas.
        let titan = Gases {
            n2: 1.45,
            ch4: methane_sea_vapour_bar(94.0),
            ..Gases::default()
        };
        assert!(
            (0.03..0.06).contains(&(titan.ch4 / titan.total())),
            "{titan:?}"
        );
        assert_eq!(
            liquid(94.0, &titan, &dry, &none),
            SurfaceLiquid::Hydrocarbons
        );
    }

    #[test]
    fn nitrogen_keeps_methane_liquid_below_its_triple_point() {
        // No nitrogen: a sea of half methane, half ethane, liquid down
        // to about 77 K.
        let bare = methane_freezes_k(0.0);
        assert!((75.0..78.0).contains(&bare), "{bare}");
        // Titan's 1.45 bar of N₂: about a fifth of the liquid, and less
        // ethane, down to about 69 K.
        let titan = methane_freezes_k(1.45);
        assert!((67.0..71.0).contains(&titan), "{titan}");
        // More nitrogen, colder liquid, down to the cap, where methane
        // freezes first.
        assert!(methane_freezes_k(0.5) > titan);
        assert!(methane_freezes_k(10.0) < titan);
        let floor = hydrocarbon_freezes_k(MAX_NITROGEN_IN_LIQUID, 1.0 - SEA_METHANE_SHARE - 0.3);
        assert!((methane_freezes_k(100.0) - floor).abs() < 1e-9);
        assert!((66.0..69.0).contains(&floor), "{floor}");
        // A Titan at 89 K under 1 bar of N₂ keeps its lakes.
        let gases = Gases {
            n2: 1.0,
            ch4: methane_sea_vapour_bar(89.0),
            ..Gases::default()
        };
        let seas = Gases {
            ch4: 1.0,
            ..Gases::default()
        };
        let dry = Water {
            runaway: false,
            boiled: false,
            kept_kg: 0.0,
            oceans: 0.0,
        };
        assert_eq!(
            liquid(89.0, &gases, &dry, &seas),
            SurfaceLiquid::Hydrocarbons
        );
        // Without nitrogen its ethane keeps it liquid at 89 K too, but
        // not at 75 K.
        let thin = Gases { n2: 0.0, ..gases };
        assert_eq!(
            liquid(89.0, &thin, &dry, &seas),
            SurfaceLiquid::Hydrocarbons
        );
        let cold = Gases {
            ch4: methane_sea_vapour_bar(75.0),
            ..thin
        };
        assert_eq!(liquid(75.0, &cold, &dry, &seas), SurfaceLiquid::None);
    }

    /// Nitrogen alone, condensed above its triple point and under its
    /// boiling point: a sea of liquid nitrogen (issue #524).
    #[test]
    fn a_pure_nitrogen_sea_is_nitrogen() {
        let dry = Water {
            runaway: false,
            boiled: false,
            kept_kg: 0.0,
            oceans: 0.0,
        };
        let gases = Gases {
            n2: 0.5,
            ..Gases::default()
        };
        let (air, condensate) = condense(gases, 70.0);
        assert!(condensate.n2 > 0.0, "{condensate:?}");
        assert_eq!(
            liquid(70.0, &air, &dry, &condensate),
            SurfaceLiquid::Nitrogen
        );
        // Under its triple point the same nitrogen is a frost, no liquid.
        let (air, condensate) = condense(gases, 60.0);
        assert!(condensate.n2 > 0.0, "{condensate:?}");
        assert_eq!(liquid(60.0, &air, &dry, &condensate), SurfaceLiquid::None);
        let frost = frost(&gases, 0.0, 0.0, 60.0, &air);
        assert!(frost.n2 > 0.0, "{frost:?}");
    }

    /// Nitrogen that condenses into a methane sea makes the sea
    /// nitrogen-rich, liquid down to nitrogen's triple point: below the
    /// methane liquidus of [`methane_freezes_k`], the sea is still liquid
    /// and its methane no frost (issue #524).
    #[test]
    fn condensed_nitrogen_keeps_the_sea_liquid() {
        let dry = Water {
            runaway: false,
            boiled: false,
            kept_kg: 0.0,
            oceans: 0.0,
        };
        let gases = Gases {
            n2: 5.0,
            ch4: 8.0,
            ..Gases::default()
        };
        let t = 70.0;
        let (air, condensate) = condense(gases, t);
        assert!(
            condensate.n2 > 0.0 && condensate.ch4 > 0.0,
            "{condensate:?}"
        );
        assert!(t < methane_freezes_k(air.n2), "{air:?}");
        assert_eq!(
            liquid(t, &air, &dry, &condensate),
            SurfaceLiquid::Hydrocarbons
        );
        let frost = frost(&gases, 0.0, 0.0, t, &air);
        assert_eq!(frost.ch4 + frost.n2, 0.0, "{frost:?}");
    }

    /// Ethane lowers the freezing point of a methane–nitrogen liquid:
    /// Richardson, Hartwig & Leachman 2019's two liquids (F1: 61.0 %
    /// CH₄, 25.6 % C₂H₆, 13.4 % N₂, frozen at 71.5 ± 0.5 K; F2: 46.9 %,
    /// 45.3 %, 7.9 %, at 74.0 ± 0.5 K) within 2 K, where the binary rule
    /// gave 81.3 and 85.0 K (issue #530).
    #[test]
    fn ethane_lowers_the_liquidus() {
        let f1 = hydrocarbon_freezes_k(0.134, 0.256);
        assert!((f1 - 71.5).abs() < 2.0, "{f1}");
        let f2 = hydrocarbon_freezes_k(0.079, 0.453);
        assert!((f2 - 74.0).abs() < 2.0, "{f2}");
        // Without ethane or nitrogen, pure methane's triple point.
        assert!((hydrocarbon_freezes_k(0.0, 0.0) - METHANE_TRIPLE_K).abs() < 1e-9);
        // A sea under Titan's air, with its ethane, stays liquid well
        // under the binary rule's 77.6 K.
        let titan = methane_freezes_k(1.45);
        assert!(titan < 72.0, "{titan}");
    }

    /// Methane over its own ice keeps all of its saturation pressure
    /// (activity 1, a pure solid); over a sea, [`SEA_METHANE_SHARE`] of
    /// it (issue #530).
    #[test]
    fn methane_ice_holds_its_full_saturation() {
        let gases = Gases {
            n2: 1e-3,
            ch4: 1.0,
            ..Gases::default()
        };
        let ice_k = 50.0;
        assert!(ice_k < methane_freezes_k(gases.n2));
        let (air, _) = condense(gases, ice_k);
        let full = methane_saturation_bar(ice_k);
        assert!((air.ch4 / full - 1.0).abs() < 1e-9, "{air:?} {full:e}");
        let sea_k = 95.0;
        let (air, _) = condense(gases, sea_k);
        assert!(
            (air.ch4 / methane_sea_vapour_bar(sea_k) - 1.0).abs() < 1e-9,
            "{air:?}"
        );
    }

    /// Methane above the tropopause absorbs starlight in its
    /// near-infrared bands and sends it back to space (the
    /// anti-greenhouse): a share of the light that grows with the
    /// methane let through the cold trap and saturates, taken off what
    /// the surface absorbs (issue #530).
    #[test]
    fn stratospheric_methane_takes_a_share_of_the_light() {
        let titan = Gases {
            n2: 1.4,
            ch4: 0.08,
            ..Gases::default()
        };
        let dry = Gases { ch4: 0.0, ..titan };
        let skin = 70.0;
        assert_eq!(stratospheric_share(&dry, skin), 0.0);
        let share = stratospheric_share(&titan, skin);
        assert!((0.01..0.2).contains(&share), "{share}");
        // A warmer tropopause lets more methane through: more of the light.
        assert!(stratospheric_share(&titan, skin + 3.0) > share);
        // Twenty-six bar of methane under a warm tropopause: saturated.
        let rich = Gases { ch4: 26.0, ..titan };
        let most = stratospheric_share(&rich, 96.0);
        assert!(most > share && most <= CH4_NEAR_IR_MAX, "{most}");
        // It comes off the light the surface absorbs, not the heat.
        let t_eq: f64 = 83.0;
        let light = SIGMA * t_eq.powi(4);
        let absorbed = absorbed_w_m2(t_eq, &titan, 0.1);
        let expected = light * (1.0 - stratospheric_share(&titan, t_eq * 0.5f64.powf(0.25))) + 0.1;
        assert!((absorbed - expected).abs() < 1e-12, "{absorbed} {expected}");
        assert_eq!(absorbed_w_m2(t_eq, &dry, 0.1), light + 0.1);
    }

    /// Census seed 182's planet d (issue #443): an equilibrium temperature
    /// near 114 K under some 4 bar of N₂ and a methane-rich air, which the
    /// saturated methane greenhouse keeps under the 150 K line, drawn as
    /// `planet_census` draws it (issue #530).
    #[test]
    fn census_182_d() {
        use crate::system::{BodyKind, SystemRequest, WholeSystemRequest, draw};
        use rand::SeedableRng;
        let request = SystemRequest::System(WholeSystemRequest {
            seed: Some(182),
            ..WholeSystemRequest::default()
        });
        let system =
            draw(&request, &mut rand::rngs::StdRng::seed_from_u64(182)).expect("the system draws");
        let body = system
            .bodies
            .iter()
            .find(|b| b.kind == BodyKind::Planet && b.id == "d")
            .expect("planet d");
        let model = crate::planet::body_model(&system, body).expect("a model");
        let s = model.surface.expect("a surface");
        let c = s.climate;
        // The planet the issue names: T_eq about 114.5 K, 4.4 bar of N₂
        // and 3.3 bar of methane over its seas once the rest condensed.
        assert!((110.0..120.0).contains(&c.equilibrium_k), "{c:?}");
        assert!((4.0..5.0).contains(&s.gases.n2), "{:?}", s.gases);
        assert!((2.0..5.0).contains(&s.gases.ch4), "{:?}", s.gases);
        assert!(c.equilibrium_k < 150.0, "{c:?}");
        assert!(c.temperature_k < 150.0, "{c:?} {:?}", s.gases);
        assert_ne!(c.liquid, SurfaceLiquid::Water, "{c:?}");
    }

    /// A frost of nitrogen radiates as β-N₂ ice: a body under it settles
    /// warmer than a black body at the same light, by `ε^-¼`.
    #[test]
    fn nitrogen_frost_radiates_less_than_a_black_body() {
        let frost = Gases {
            n2: 1.0,
            ..Gases::default()
        };
        assert_eq!(emissivity(&Gases::default()), 1.0);
        assert!((emissivity(&frost) - N2_ICE_EMISSIVITY).abs() < 1e-12);
        let c = crate::planet::reference::sun_at(30.0);
        let black = surface_temperature_k(&c, &Gases::default(), 0.0, 1.0);
        let grey = surface_temperature_k(&c, &Gases::default(), 0.0, N2_ICE_EMISSIVITY);
        let ratio = grey / black;
        assert!(
            (ratio - N2_ICE_EMISSIVITY.powf(-0.25)).abs() < 1e-9,
            "{black} {grey}"
        );
    }

    #[test]
    fn an_airless_body_sits_at_its_equilibrium_temperature() {
        // Its equilibrium temperature under its own (regolith) albedo.
        let (d, g, mercury) = climate_of("Mercury");
        let t_eq = d.context.with_albedo(mercury.bond_albedo).equilibrium_k;
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
            temperate(scarce.co2),
            &d.planet,
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
                assert_eq!(
                    liquid(t, &air, &wet, &Gases::default()),
                    SurfaceLiquid::None,
                    "{t} {wet:?}"
                );
            }
        }
        let dry = Water {
            runaway: true,
            boiled: false,
            kept_kg: 0.0,
            oceans: 0.0,
        };
        assert_eq!(
            liquid(2000.0, &air, &dry, &Gases::default()),
            SurfaceLiquid::Lava
        );
    }

    #[test]
    fn a_wet_surface_past_the_moist_limit_boils() {
        // Earth's air and an ocean (some 280 bar if all of it were
        // vapour). At 0.9 AU the Sun gives 1.23 S⊕, 295 W/m² absorbed,
        // over the moist plateau: no wet balance, it boils. At 1 AU
        // (240 W/m²) it balances. A runaway does not boil again, and a
        // trace of water (1e-4 bar) cannot hold the moist limit.
        let air = Gases {
            n2: 0.78,
            co2: 4e-4,
            ..Gases::default()
        };
        let ocean = Water {
            runaway: false,
            boiled: false,
            kept_kg: 1.4e21,
            oceans: 1.0,
        };
        let close = crate::planet::reference::sun_at(0.9);
        let earth = crate::planet::reference::sun_at(1.0);
        assert!(boils(&close, &air, &ocean, 0.09, 280.0));
        assert!(!boils(&earth, &air, &ocean, 0.09, 280.0));
        let runaway = Water {
            runaway: true,
            ..ocean
        };
        assert!(!boils(&close, &air, &runaway, 0.09, 280.0));
        assert!(!boils(&close, &air, &ocean.keeping(5e17), 0.09, 1e-4));
        // Without a balance, the climate puts all the water in the air
        // (a placeholder the chain replaces).
        let (g, _) = climate(
            &close,
            air,
            &ocean,
            Regime::Stagnant,
            0.09,
            280.0,
            temperate(air.co2),
            &Planet::EARTH,
        );
        assert_eq!(g.h2o, 280.0, "{g:?}");
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
        let t_thin = surface_temperature_k(&c, &thin, 0.0, 1.0);
        let t_thick = surface_temperature_k(&c, &thick, 0.0, 1.0);
        assert!(
            t_thin > c.equilibrium_k && t_thick > t_thin,
            "{t_thin} {t_thick}"
        );
        // Internal heat warms an airless surface.
        let bare = surface_temperature_k(&c, &Gases::default(), 0.0, 1.0);
        assert!((bare - c.equilibrium_k).abs() < 1e-9);
        assert!(surface_temperature_k(&c, &Gases::default(), 100.0, 1.0) > bare);
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
            d.carbon,
            &d.planet,
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
        // Goldblatt et al. 2009 (doi:10.1038/ngeo692). Their CO₂ held
        // fixed, no thermostat, and Earth's albedo (0.3): this checks the
        // column's greenhouse, so the ice–albedo feedback of the derived
        // albedo is left out (the wet balance under the given light).
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
            (
                gases,
                wet_temperature_k(&faint, &gases, d.heat_flux_w_m2, d.water_bar, 1.0),
            )
        };
        let (g1, t1) = temperature(air(0.99));
        let (g2, t2) = temperature(air(1.99));
        println!(
            "late Archean: {t1:.1} K, +{:.1} K with twice the N₂",
            t2 - t1
        );
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
        // Below the column's knee, CO₂ keeps the square root of its own
        // and the water's pressure plus the nitrogen's, saturated past
        // the local knee; water its own pressure plus a share of the rest.
        let earth = Gases {
            n2: 0.9,
            co2: 3e-4,
            h2o: 0.0126,
            ..Gases::default()
        };
        let n2 = local_foreign(0.9);
        let water_broadening = 0.0126 + WATER_FOREIGN_BROADENING * (n2 + 3e-4);
        let expected = CO2_OPACITY * 3e-4_f64.powf(CO2_EXPONENT) * (3e-4 + 0.0126 + n2).sqrt()
            + H2O_OPACITY * (0.0126 * water_broadening).sqrt();
        assert!((optical_depth(&earth) / expected - 1.0).abs() < 1e-12);
    }

    /// The pressure of `others` bar of foreign gases at a level, saturated
    /// past [`LOCAL_KNEE_BAR`].
    fn local_foreign(others: f64) -> f64 {
        if others <= LOCAL_KNEE_BAR {
            others
        } else {
            LOCAL_KNEE_BAR * (others / LOCAL_KNEE_BAR).powf(LOCAL_KNEE_EXPONENT)
        }
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
        let n2 = local_foreign(0.89);
        let expected = co2(1.8 + 0.01 + n2) + water(n2);
        assert!((optical_depth(&thick) / expected - 1.0).abs() < 1e-12);
        // Past the knee only the nitrogen is tempered: 5 bar of N₂ count
        // as K (5/K)^THICK_AIR_EXPONENT bar (then saturated past the local
        // knee), the CO₂ and the vapour in full for both.
        let nitrogen = Gases { n2: 5.0, ..thick };
        let n2 = local_foreign(
            BROADENING_KNEE_BAR * (5.0 / BROADENING_KNEE_BAR).powf(THICK_AIR_EXPONENT),
        );
        let expected = co2(1.8 + 0.01 + n2) + water(n2);
        assert!((optical_depth(&nitrogen) / expected - 1.0).abs() < 1e-12);
    }

    /// Earth's inputs with the snowball draw `draw`.
    fn earth_drawn(draw: f64) -> PlanetInputs {
        PlanetInputs {
            snowball_draw: draw,
            ..model_inputs(reference("Earth"))
        }
    }

    /// The chance that `inputs` at `orbit_au` round the Sun are drawn a
    /// snowball: [`snowball_chance`] of their frozen branch, 0 with no
    /// frozen branch beside a temperate one, 1 with no temperate one.
    fn chance_at(inputs: &PlanetInputs, orbit_au: f64) -> f64 {
        let d = derive_at(inputs, orbit_au);
        let body = Body::new(
            &d.context,
            d.gases,
            &d.water,
            d.regime,
            d.heat_flux_w_m2,
            d.water_bar,
            &d.carbon,
            &d.planet,
            None,
        );
        let b = branches(&body);
        match &b.frozen {
            Some(frozen) => frozen.chance(&body),
            None if b.warm.1.liquid == SurfaceLiquid::Water => 0.0,
            None => 1.0,
        }
    }

    #[test]
    fn an_earth_track_planet_stays_temperate_to_the_maximum_greenhouse() {
        // The outer edge of the Sun's habitable zone, the maximum
        // greenhouse, is about 1.67 AU (Kopparapu et al. 2013, *ApJ* 765,
        // 131, doi:10.1088/0004-637X/765/2/131): an Earth on its temperate
        // branch builds up its CO₂ and keeps liquid water to 1.6 AU, and
        // freezes beyond the edge. Colder the farther, with no step but
        // the edge.
        let earth = earth_drawn(1.0);
        let mut last = f64::INFINITY;
        for au in [1.0, 1.4, 1.6, 1.8, 2.5] {
            let (_, g, c) = climate_at(&earth, au);
            println!(
                "{au} AU: {:.1} K, A {:.3}, CO₂ {:.3e} bar, {:?}, regulated {}",
                c.temperature_k, c.bond_albedo, g.co2, c.liquid, c.regulated
            );
            if au <= 1.6 {
                assert!(c.temperature_k >= 273.0, "{au} AU: {c:?} {g:?}");
                assert_eq!(c.liquid, SurfaceLiquid::Water, "{au} AU: {c:?}");
            } else {
                assert!(c.temperature_k < 273.0, "{au} AU: {c:?} {g:?}");
            }
            assert!(!c.snowball, "{au} AU: {c:?}");
            assert!(
                c.temperature_k <= last + 0.05,
                "{au} AU: {c:?} after {last}"
            );
            last = c.temperature_k;
        }
    }

    #[test]
    fn co2_builds_up_past_the_outgassed_snapshot() {
        // At 1.6 AU, an air that holds a tenth of a bar of CO₂ before the
        // climate (what escape and the cold trap left of it): weathering
        // slows and volcanoes give it more, up to the thermostat, within
        // the carbon inventory and what does not frost out.
        let d = derive_at(&earth_drawn(1.0), 1.6);
        let snapshot = Gases {
            co2: 0.1,
            ..d.gases
        };
        let with = |carbon: CarbonCycle| {
            climate(
                &d.context,
                snapshot,
                &d.water,
                d.regime,
                d.heat_flux_w_m2,
                d.water_bar,
                carbon,
                &d.planet,
            )
        };
        let (g, c) = with(d.carbon);
        assert!(g.co2 > snapshot.co2, "{g:?}");
        assert!(g.co2 <= d.carbon.inventory_bar, "{g:?} {:?}", d.carbon);
        let frost = super::super::atmosphere::co2_frost_bar(d.context.equilibrium_k);
        assert!(g.co2 <= frost, "{g:?} {frost}");
        assert!(c.regulated, "{c:?}");
        // A smaller inventory stops it short of the thermostat.
        let (g, c) = with(temperate(0.5));
        assert!(g.co2 <= 0.5, "{g:?}");
        assert!(!c.regulated, "{c:?}");
        // A snowball builds its CO₂ up past what it froze with, the same
        // whatever draw made it one.
        let earth = earth_drawn(1.0);
        let (_, warm, _) = climate_at(&earth, 1.2);
        let half = 0.5 * chance_at(&earth, 1.2);
        let (_, frozen, c) = climate_at(&earth_drawn(half), 1.2);
        let (_, pinned, _) = climate_at(&earth_drawn(0.0), 1.2);
        assert_eq!(frozen, pinned);
        assert!(c.snowball, "{c:?}");
        assert!(frozen.co2 > warm.co2, "{frozen:?} {warm:?}");
        assert!(frozen.co2 <= d.carbon.inventory_bar, "{frozen:?}");
    }

    #[test]
    fn a_frozen_state_no_co2_thaws_is_no_snowball() {
        // Drawn frozen where no CO₂ within the inventory and what does not
        // frost out thaws it: it stays frozen for good, so it is no
        // snowball (whose card promises a thaw). At 1.64 AU: up to
        // 1.61 AU about 1.6 bar of CO₂ thaws it, since its vapour
        // absorbs sunlight (issue #523), and past 1.66 AU the temperate
        // state freezes too.
        let earth = earth_drawn(0.0);
        let d = derive_at(&earth, 1.64);
        let body = Body::new(
            &d.context,
            d.gases,
            &d.water,
            d.regime,
            d.heat_flux_w_m2,
            d.water_bar,
            &d.carbon,
            &d.planet,
            None,
        );
        let frozen = branches(&body).frozen.expect("bistable at 1.64 AU");
        assert!(frozen.thaw_bar.is_infinite(), "{}", frozen.thaw_bar);
        let (_, _, c) = climate_at(&earth, 1.64);
        assert!(frozen_over(&c), "{c:?}");
        assert!(!c.snowball, "{c:?}");
        // Where the CO₂ thaws it, a frozen draw is a snowball.
        let (_, _, c) = climate_at(&earth, 1.2);
        assert!(c.snowball, "{c:?}");
    }

    #[test]
    fn a_dry_acid_air_settles_through_the_deck_onset() {
        // A dry air rich in SO₂ makes a sulphuric deck once warm: its
        // albedo climbs from the bare ground's to the deck's as it warms,
        // which must leave a fixed point the albedo settles on (a step
        // would swing between a warm deckless and a cold decked climate).
        let d = derive_at(&model_inputs(reference("Mars")), 1.0);
        let dry = Water {
            runaway: false,
            boiled: false,
            kept_kg: 0.0,
            oceans: 0.0,
        };
        let air = Gases {
            n2: 3.0,
            co2: 0.05,
            so2: 0.02,
            ..Gases::default()
        };
        let mut near = 0;
        for i in 0..=60 {
            let au = 0.8 + 0.02 * f64::from(i);
            let context = super::super::reference::sun_at(au);
            let body = Body::new(
                &context,
                air,
                &dry,
                d.regime,
                d.heat_flux_w_m2,
                0.0,
                &temperate(air.co2),
                &d.planet,
                None,
            );
            let (g, c) = climate(
                &context,
                air,
                &dry,
                d.regime,
                d.heat_flux_w_m2,
                0.0,
                temperate(air.co2),
                &d.planet,
            );
            if (263.0..=330.0).contains(&c.temperature_k) {
                near += 1;
            }
            let derived = body.albedo_at(c.temperature_k, &g);
            let t_eq = |a: f64| context.with_albedo(a).equilibrium_k;
            assert!(
                (t_eq(derived) - t_eq(c.bond_albedo)).abs() < 0.1,
                "{au} AU: {c:?}, derived albedo {derived}"
            );
        }
        assert!(near >= 3, "too few orbits near the onset: {near}");
    }

    #[test]
    fn far_planets_only_freeze() {
        // Review Focus 3: with no temperate state, the snowball draw
        // changes nothing in the surface.
        for au in [5.0, 10.0] {
            let surface = |draw: f64| {
                crate::planet::planet_model(
                    &crate::planet::reference::sun_at(au),
                    earth_drawn(draw),
                )
                .surface
                .expect("a bare world has a surface")
            };
            let (cold, warm) = (surface(0.0), surface(1.0));
            assert_eq!(cold.gases, warm.gases, "{au} AU");
            assert_eq!(cold.climate, warm.climate, "{au} AU");
            assert!(!cold.climate.snowball, "{au} AU");
        }
    }

    #[test]
    fn snowballs_are_rare_near_earth_and_common_near_the_edge() {
        // Earth spent about a tenth of its history in snowball states
        // (Hoffman et al. 2017, doi:10.1126/sciadv.1600983).
        let earth = earth_drawn(1.0);
        let near = chance_at(&earth, 1.0);
        println!(
            "chance at 1.0 AU {near:.3}, 1.2 AU {:.3}, 1.4 AU {:.3}",
            chance_at(&earth, 1.2),
            chance_at(&earth, 1.4)
        );
        assert!((0.03..=0.15).contains(&near), "{near}");
        let far = chance_at(&earth, 1.5);
        assert!(far > near, "{far} vs {near}");
        assert_eq!(chance_at(&earth, 1.8), 1.0);
        // Less outgassing, longer snowballs.
        let d = derive_at(&earth, 1.0);
        let plates = snowball_chance(0.05, d.heat_flux_w_m2, Regime::Plates);
        let lid = snowball_chance(0.05, d.heat_flux_w_m2, Regime::Stagnant);
        assert!(lid > plates, "{lid} vs {plates}");
        assert_eq!(
            snowball_chance(f64::INFINITY, d.heat_flux_w_m2, Regime::Plates),
            1.0
        );
    }

    #[test]
    fn a_snowball_keeps_its_ocean_frozen() {
        let (_, g, c) = climate_at(&earth_drawn(0.0), 1.2);
        assert!(c.snowball, "{c:?} {g:?}");
        assert_eq!(c.liquid, SurfaceLiquid::None, "{c:?}");
        assert!(c.bond_albedo > 0.5, "{c:?}");
        assert!(c.temperature_k < 260.0, "{c:?}");
    }

    #[test]
    fn a_uniform_column_integrates_to_the_column_totals() {
        // The opacity per ln p of a uniform air, integrated from the top
        // to the surface, gives back the totals of the per-gas laws, for
        // an air whose other gases stay under `LOCAL_KNEE_BAR`.
        let air = Gases {
            n2: 0.15,
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
            volatiles: crate::planet::inputs::volatile_rule(
                1.0 - water_fraction,
                water_fraction,
                0.325,
            ),
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
    fn a_runaway_absorbs_the_moist_plateau_at_its_runaway_limit() {
        // At the runaway limit of Kopparapu et al. for its star, a body
        // in a water runaway absorbs exactly the column's moist plateau;
        // past it, more; a wet body, or a runaway with no water left,
        // keeps Earth's Bond albedo.
        for (kind, teff) in [("M9V", 2400.0), ("G2V", 5780.0), ("F0V", 7200.0)] {
            let star =
                crate::star::StarInfo::new(crate::star::parse_type(kind).unwrap(), 1.0, 0.0, 2.0);
            let mut context = Context::of(&star, 0.0, 2.7);
            context.teff_k = teff;
            let limit = f64::from(crate::star::habitable_fluxes(teff as f32).0);
            let absorbed = |context: &Context| SIGMA * context.equilibrium_k.powi(4);
            for (flux, ratio) in [(limit, 1.0), (2.0 * limit, 2.0)] {
                context.insolation = flux;
                context.equilibrium_k = f64::from(crate::star::equilibrium_k(flux as f32));
                let steam = Water {
                    runaway: true,
                    boiled: false,
                    kept_kg: 1e20,
                    oceans: 1e20 / super::super::OCEAN_KG,
                };
                let runaway = absorbing(&context, &steam, 1e4, REFERENCE_ALBEDO);
                let got = absorbed(&runaway) / (ratio * *MOIST_PLATEAU_W_M2);
                assert!((got - 1.0).abs() < 1e-3, "{kind} {flux}: {got}");
                let wet = Water {
                    runaway: false,
                    ..steam
                };
                let dry = steam.keeping(0.0);
                assert_eq!(absorbing(&context, &wet, 1e4, REFERENCE_ALBEDO), context);
                assert_eq!(absorbing(&context, &dry, 0.0, REFERENCE_ALBEDO), context);
            }
        }
    }

    #[test]
    fn a_runaways_light_follows_its_steam_down_to_none() {
        // As the kept water (and its steam) goes to zero, the light a
        // runaway absorbs goes smoothly to Earth's albedo's: no jump when
        // the last of it escapes, and more steam never absorbs more.
        let star =
            crate::star::StarInfo::new(crate::star::parse_type("M9V").unwrap(), 0.02, 0.0, 2.0);
        let context = Context::of(&star, 0.0, 2.7);
        let steam = Water {
            runaway: true,
            boiled: false,
            kept_kg: 1e21,
            oceans: 1e21 / super::super::OCEAN_KG,
        };
        // The steam a kept mass weighs, bar (any linear law: here
        // 1e-18 bar per kg).
        let t_eq = |kg: f64| {
            absorbing(&context, &steam.keeping(kg), kg * 1e-18, REFERENCE_ALBEDO).equilibrium_k
        };
        let dry = absorbing(&context, &steam.keeping(0.0), 0.0, REFERENCE_ALBEDO).equilibrium_k;
        assert_eq!(dry, context.equilibrium_k);
        let mut last = dry;
        for kg in [1.0, 1e10, 1e15, 1e17, 1e18, 1e19, 1e20, 1e21] {
            let t = t_eq(kg);
            // The moist albedo of an M9V's runaway is under Earth's.
            assert!(t >= last, "{kg}: {t} < {last}");
            last = t;
        }
        assert!(
            (t_eq(1.0) - dry).abs() < 1e-9 * dry,
            "{} vs {dry}",
            t_eq(1.0)
        );
        assert!(
            (t_eq(1e15) - dry).abs() < 1e-3 * dry,
            "{} vs {dry}",
            t_eq(1e15)
        );
        // A thick column takes the moist albedo whole.
        let thick = absorbing(&context, &steam, 1e4, REFERENCE_ALBEDO).equilibrium_k;
        assert!((t_eq(1e21) - thick).abs() < 1e-9 * thick);
    }

    #[test]
    fn the_runaway_plateau_is_the_solves_moist_branch() {
        // The plateau `A_rg` is anchored on is the outgoing radiation the
        // surface solves give a moist column, under the skin of a body at
        // the runaway limit, to 1 % over the moist branch.
        let plateau = *MOIST_PLATEAU_W_M2;
        let skin = plateau_skin_k(plateau);
        for t in [380.0, 450.0, 550.0, 640.0] {
            let olr =
                column::olr_under_skin_w_m2(&with_vapour(earth_air(), t, f64::INFINITY), t, skin);
            assert!(
                (olr / plateau - 1.0).abs() < 0.01,
                "{t} K: {olr} vs {plateau}"
            );
        }
    }

    #[test]
    fn a_planet_just_past_the_runaway_limit_runs_away_hot() {
        // An Earth with 1 % of its mass in water (escape leaves it steam)
        // at 1.01 times its star's runaway limit: its water runs away and
        // its surface settles well above water's critical point, not on
        // it.
        let water = 0.01;
        let earth = PlanetInputs {
            volatiles: crate::planet::inputs::volatile_rule(1.0 - water, water, 0.325),
            ..crate::planet::reference::inputs(1.0, 0.325, water)
        };
        let teff = crate::planet::reference::sun_at(1.0).teff_k;
        let limit = f64::from(crate::star::habitable_fluxes(teff as f32).0);
        let orbit = (1.01 * limit).powf(-0.5);
        let (d, g, c) = climate_at(&earth, orbit);
        assert!(d.water.runaway, "{:?}", d.water);
        assert!(d.water.kept_kg > 0.0, "{:?}", d.water);
        assert!(c.temperature_k > WATER_CRITICAL_K + 100.0, "{c:?} {g:?}");
    }

    #[test]
    fn the_moist_plateau_is_the_simpson_nakajima_limit() {
        // Earth air, saturated column, surface 340..400 K: OLR flat at
        // 282 W/m² ± 5 % (Goldblatt et al. 2013). The moist OLR only
        // reaches the limit asymptotically as the surface warms (Goldblatt
        // et al. 2013, Fig. 1), so at 320 K it is still rising: below the
        // plateau, yet above 240 W/m².
        let olr = |t: f64| column::olr_w_m2(&with_vapour(earth_air(), t, f64::INFINITY), t);
        for t in [340.0, 360.0, 380.0, 400.0] {
            let olr = olr(t);
            assert!((268.0..=296.0).contains(&olr), "{t} K: {olr}");
            println!("plateau at {t} K: {olr:.1} W/m²");
        }
        let (warm, plateau) = (olr(320.0), olr(380.0));
        println!("approach at 320 K: {warm:.1} W/m²");
        assert!(warm > 240.0 && warm < plateau, "{warm} vs {plateau}");
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
            let (g, t) = wet_balance(&d.context, gases, d.heat_flux_w_m2, d.water_bar, 1.0)
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

    #[test]
    fn a_format_10_climate_reads_as_converged() {
        // Review Focus 5: a climate stored before `converged` (format 10)
        // still loads, as a converged one.
        let json = r#"{"temperature_k":288.0,"equilibrium_k":255.0,"bond_albedo":0.3,
            "liquid":"water","emissivity":1.0,"regulated":true,"snowball":false}"#;
        let c: Climate = serde_json::from_str(json).unwrap();
        assert!(c.converged, "{c:?}");
    }

    /// Titan's body under `gases`, its light scaled by each of `count`
    /// factors from `low` to `high`: the flux and the climate.
    fn titan_sweep(gases: Gases, low: f64, high: f64, count: usize) -> Vec<(f64, Climate)> {
        let (d, _, _) = climate_of("Titan");
        (0..count)
            .map(|i| {
                let f = low + (high - low) * i as f64 / (count - 1) as f64;
                let context = Context {
                    insolation: d.context.insolation * f,
                    equilibrium_k: d.context.equilibrium_k * f.powf(0.25),
                    ..d.context
                };
                let (_, c) = climate(
                    &context,
                    gases,
                    &d.water,
                    d.regime,
                    d.heat_flux_w_m2,
                    d.water_bar,
                    d.carbon,
                    &d.planet,
                );
                (context.insolation, c)
            })
            .collect()
    }

    /// Review Focus 1: across `threshold_k`, every climate of `sweep`
    /// settled, and its temperature rises with the flux, with no jump
    /// over `most_jump_k` between neighbours.
    fn settles_across(sweep: &[(f64, Climate)], threshold_k: f64, most_jump_k: f64) {
        let (first, last) = (sweep[0].1, sweep[sweep.len() - 1].1);
        assert!(
            first.temperature_k <= threshold_k - 3.0 && last.temperature_k >= threshold_k + 3.0,
            "the sweep spans {:.2} to {:.2} K, not {threshold_k:.2} ± 3 K",
            first.temperature_k,
            last.temperature_k
        );
        for (flux, c) in sweep {
            assert!(c.converged, "{flux:.6}: {c:?}");
        }
        for pair in sweep.windows(2) {
            let ((f0, a), (f1, b)) = (pair[0], pair[1]);
            let step = b.temperature_k - a.temperature_k;
            assert!(
                step > -ALBEDO_TOLERANCE_K && step < most_jump_k,
                "{f0:.6} -> {f1:.6}: {:.3} -> {:.3} K ({a:?} {b:?})",
                a.temperature_k,
                b.temperature_k
            );
        }
    }

    /// Titan's methane over a thinner nitrogen air, 0.1 bar: one that
    /// stays a gas down to nitrogen's triple point, so that the sweep
    /// crosses the methane sea's freezing point alone.
    fn thin_titan() -> Gases {
        let (d, _, _) = climate_of("Titan");
        Gases { n2: 0.1, ..d.gases }
    }

    /// A Triton-like body: Titan's without its methane, under 0.5 bar
    /// of nitrogen, which frosts on its ground under nitrogen's triple
    /// point and pools above it.
    fn nitrogen_world() -> Gases {
        let (d, _, _) = climate_of("Titan");
        Gases {
            n2: 0.5,
            ch4: 0.0,
            ..d.gases
        }
    }

    #[test]
    fn the_fixed_point_converges_across_the_methane_freezing_point() {
        let gases = thin_titan();
        let freezes_k = methane_freezes_k(gases.n2);
        let sweep = titan_sweep(gases, METHANE_SWEEP.0, METHANE_SWEEP.1, 200);
        // The methane darkening the air (its near-infrared absorption)
        // makes such a body bistable on either side of the freezing
        // point, whatever the phase change: from about 62 K (its frost
        // bright, its air cold and clear) to about 74 K, and over about
        // 1.2 K just above the point, where the air the warming sea
        // gives darkens the body faster than it warms (`climate_sweep
        // phase`). Those folds are steps between settled climates; the
        // phase change itself is crossed through its band, where before
        // its smoothing the fixed point cycled.
        settles_across(&sweep, freezes_k, f64::INFINITY);
        let melting = sweep
            .iter()
            .filter(|(_, c)| (freezes_k - PHASE_BAND_K..freezes_k).contains(&c.temperature_k))
            .count();
        assert!(melting >= 20, "{melting} climates in the band");
    }

    #[test]
    fn the_fixed_point_converges_across_the_nitrogen_triple_point() {
        let sweep = titan_sweep(nitrogen_world(), NITROGEN_SWEEP.0, NITROGEN_SWEEP.1, 200);
        settles_across(&sweep, NITROGEN_TRIPLE_K, 1.0);
    }

    /// The light, against Titan's, over which [`thin_titan`]'s surface
    /// spans the methane sea's freezing point ± 3 K.
    const METHANE_SWEEP: (f64, f64) = (0.7, 1.0);

    /// The light, against Titan's, over which [`nitrogen_world`]'s
    /// surface spans nitrogen's triple point ± 3 K.
    const NITROGEN_SWEEP: (f64, f64) = (0.7, 1.2);
}
