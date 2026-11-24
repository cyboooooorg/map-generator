//! The liquid on a body's surface and the frost on its ground.
//!
//! Ported from `legacy-final` (`planet/climate.rs`: `liquid`,
//! `sea_freezes_k`, `methane_freezes_k`, `hydrocarbon_freezes_k`,
//! `methane_condenses`, `methane_sea_vapour_bar`, `frozen_share`,
//! `melting`, `frost`), with the values of study
//! [24](https://github.com/cyboooooorg/map-generator/wiki/24-surface-cover-audit)
//! and the rewrite's phase curves (`science::water`, IAPWS;
//! `science::volatiles`). The condensate, the part of the N2 and CH4 that
//! the air cannot hold, is an input: `condense` makes it, at
//! the temperature the climate solves for.

use super::cover::{FROST_LAYER, share};
use crate::constants::{MOLAR_GAS_CONSTANT, PASCALS_PER_BAR, STEFAN_BOLTZMANN};
use crate::radiation::gases::PartialPressures;
use crate::units::{Bar, Fraction, Kelvin, Kilogram, Pascal, WattPerSquareMetre};
use crate::volatiles::{
    ETHANE_TRIPLE_TEMPERATURE, METHANE_TRIPLE_TEMPERATURE, NITROGEN_TRIPLE_TEMPERATURE,
    co2_frost_pressure, ethane_boiling_temperature, methane_saturation_bar,
    nitrogen_saturation_pressure, sulphur_dioxide_frost_pressure,
};
use crate::water::{boiling_temperature, melting_temperature};

/// Source: the temperature from which a rocky surface is molten, 1673 K:
/// Kite, Fegley, Schaefer & Gaidos 2016 (*ApJ* 828, 80,
/// doi:10.3847/0004-637X/828/2/80, footnote 1 and appendix) count as
/// magma worlds the planets with a substellar temperature over 1673 K,
/// hot enough to melt peridotite, and take 1673 K for the edge of a magma
/// pool ("Katz et al. (2003) recommends a parameterization that yields
/// 1673 K for T_lo"). Kite et al. set it on the substellar temperature;
/// [`liquid`] compares it with the surface temperature it is given, the
/// body's mean one, as legacy did: colder than the substellar one, it
/// counts fewer lava worlds (issue #675). Study 24, section 9.
pub const LAVA_TEMPERATURE: Kelvin = Kelvin(1673.0);

/// Source: the methane over a Titan sea as a share of pure methane's
/// saturation, and the sea's methane mole fraction (Raoult's law ties
/// them), 0.5: the Huygens GCMS measured (5.65 +- 0.18) % of methane near
/// the surface (Niemann et al. 2010, *J. Geophys. Res.* 115, E12006,
/// doi:10.1029/2010JE003659), under 1.467 bar at 93.65 K (Fulchignoni et
/// al. 2005), a relative humidity of 0.49 +- 0.02 (derived). The seas'
/// published compositions run from 0.32 to 0.68 (Glein & Shock 2013,
/// doi:10.1016/j.gca.2013.03.030, Table 4). Study 24, section 3.
pub const SEA_METHANE_SHARE: f64 = 0.5;

/// Calibration: the band, 1 K, under a melting point over which a frost
/// melts into its liquid ([`frozen_share`]): a numerical smoothing, not
/// physics. A frozen surface just under a triple point warmed over it and
/// a liquid one just over it cooled under it, so the climate's fixed
/// point had no solution there; 1 K is the narrowest band over which the
/// damped fixed point settles (legacy `PHASE_BAND_K`, issue #562; held by
/// 2d-3's convergence test). See `docs/science/planet-atmospheres.md`,
/// "Phase changes and convergence".
pub const PHASE_BAND: Kelvin = Kelvin(1.0);

/// Source: the nitrogen a methane liquid dissolves under a nitrogen air,
/// as a share of Raoult's law (`p_N2 / p_sat,N2(T)`), 0.5: Thompson,
/// Zollweg & Gabis 1992 (*Icarus* 97, 187,
/// doi:10.1016/0019-1035(92)90127-S) find from their N2 + CH4
/// vapour-liquid equilibrium model that "the N2 content of condensate is
/// ~1/2 of that computed from Raoult's law", 16 to 30 % of Titan's cloud
/// droplets. See `docs/science/atmospheres.md`.
pub const NITROGEN_RAOULT_SHARE: f64 = 0.5;

/// Source: the most nitrogen in the methane liquid, mole fraction 0.3:
/// the top of the 16 to 30 % Thompson, Zollweg & Gabis 1992
/// (doi:10.1016/0019-1035(92)90127-S) find in Titan's cloud droplets;
/// beyond it the rule of [`methane_freezes`] is not checked.
pub const MAX_NITROGEN_IN_LIQUID: f64 = 0.3;

/// Source: the enthalpy of fusion of methane, 939.2 J/mol at its triple
/// point (Vogt & Pitzer 1976, as the NIST Chemistry WebBook gives it).
const METHANE_FUSION_ENTHALPY: f64 = 939.2;

/// Source: the enthalpy of melting of the ethane solid a cold liquid
/// freezes into, 2.79 kJ/mol at 89.5 K (Domalski & Hearing 1996, through
/// the NIST Chemistry WebBook; study 24, section 9). Ethane's own
/// enthalpy of fusion is 0.583 kJ/mol (Atake & Chihara 1976), but its
/// plastic phase lasts under a kelvin below its triple point: under it
/// the solid takes the transition's enthalpy too, the sum Domalski &
/// Hearing give.
const ETHANE_FUSION_ENTHALPY: f64 = 2_790.0;

/// Calibration: the non-ideality of methane in a liquid of methane,
/// ethane and nitrogen, the regular-solution parameter `W / R` = 75.3 K,
/// whose activity coefficient `ln g = (W / R)(1 - x)^2 / T` raises the
/// methane liquidus ([`hydrocarbon_freezes`]). Fitted on Richardson,
/// Hartwig & Leachman 2019's liquid F1 (doi:10.1016/j.ijthermalsci.2018.12.024:
/// 61.0 % methane, 25.6 % ethane, 13.4 % nitrogen, frozen at 71.5 +- 0.5
/// K), which the ideal liquidus puts at 64.9 K: an ideal methane liquidus
/// is too steep, methane's enthalpy of fusion being small. No measured
/// interaction parameter was found. See `docs/science/atmospheres.md`.
const METHANE_EXCESS_W_OVER_R: f64 = 75.3;

/// Temperature of the substellar point of a tidally locked body that
/// absorbs `absorbed` (W/m2, the mean over its whole sphere, `(1 - A)
/// S / 4`, under its own albedo `A`): `(4 absorbed / sigma)^(1/4)`, Kite et al. 2016 (*ApJ* 828,
/// 80, arXiv:1606.06740, eq. 1), `sqrt(2)` times the equilibrium
/// temperature (`Definition:`, derived).
pub fn substellar_temperature(absorbed: WattPerSquareMetre) -> Kelvin {
    Kelvin((4.0 * absorbed.0 / STEFAN_BOLTZMANN).powf(0.25))
}

/// A liquid on a body's surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Liquid {
    /// Liquid water.
    Water,
    /// A sea of methane and ethane, with the nitrogen it dissolves.
    Hydrocarbons,
    /// Liquid nitrogen.
    Nitrogen,
    /// Molten rock.
    Lava,
}

/// `p` in bar.
pub(super) fn bar(p: Pascal) -> f64 {
    p.0 / PASCALS_PER_BAR
}

/// The liquid on a surface at `t` under `air` (bar), with `water` kept
/// by the body (`runaway` when it is all steam) and `condensate` (bar,
/// the N2 and CH4 the air cannot hold) on its ground, from the phase
/// diagrams, each window holding its lower edge and leaving out its upper
/// one:
///
/// - lava from [`LAVA_TEMPERATURE`], read on the hotter of `t` and the
///   `substellar` temperature of a tidally locked body
///   ([`substellar_temperature`]; `None` for any other), on a body that has kept no water
///   (with water on top, its steam or supercritical fluid covers the
///   rock: Hamano, Abe & Genda 2013, doi:10.1038/nature12163);
/// - water from the melting of the stable ice to the boiling point at the total
///   pressure (IAPWS, [`melting_temperature`] of the stable ice,
///   [`boiling_temperature`]), above water's triple-point pressure, with
///   water kept out of a runaway;
/// - hydrocarbons from the sea's freezing point ([`sea_freezes`]) to
///   ethane's boiling point, where methane lies on the ground or the air
///   holds enough to condense ([`methane_condenses`]);
/// - nitrogen where nitrogen condenses above its triple point without a
///   methane sea (under it, a frost, [`frost`]);
/// - none otherwise.
pub fn liquid(
    t: Kelvin,
    air: &PartialPressures,
    water: Kilogram,
    runaway: bool,
    condensate: &PartialPressures,
    substellar: Option<Kelvin>,
) -> Option<Liquid> {
    let pressure = Pascal(air.total() * PASCALS_PER_BAR);
    let water_on_top = water.0 > 0.0;
    let water_window = || {
        boiling_temperature(pressure)
            .is_some_and(|boils| t.0 >= melting_temperature(pressure).0 && t.0 < boils.0)
    };
    let hottest = substellar.map_or(t.0, |ss| t.0.max(ss.0));
    if hottest >= LAVA_TEMPERATURE.0 && !water_on_top {
        Some(Liquid::Lava)
    } else if water_on_top && !runaway && water_window() {
        Some(Liquid::Water)
    } else if (condensate.ch4 > 0.0 || methane_condenses(Bar(air.ch4), t))
        && t.0 >= sea_freezes(Bar(air.n2), condensate).0
        && t.0 < ethane_boiling_temperature(pressure).0
    {
        Some(Liquid::Hydrocarbons)
    } else if condensate.n2 > 0.0 && t.0 >= NITROGEN_TRIPLE_TEMPERATURE.0 {
        Some(Liquid::Nitrogen)
    } else {
        None
    }
}

/// The temperature under which the hydrocarbon sea of a body whose air
/// holds `air_n2` of nitrogen, with `condensate` (bar) on its ground,
/// freezes. Where nitrogen itself condenses, the sea is nitrogen with
/// methane dissolved in it, liquid down to nitrogen's triple point, where
/// pure nitrogen freezes (a simple rule that errs towards freezing: N2
/// and CH4 freeze into solid solutions an ideal liquidus does not
/// describe; issue #524). Otherwise the liquidus of a methane-ethane sea
/// with the nitrogen the air dissolves in it ([`methane_freezes`]).
pub fn sea_freezes(air_n2: Bar, condensate: &PartialPressures) -> Kelvin {
    if condensate.n2 > 0.0 {
        NITROGEN_TRIPLE_TEMPERATURE
    } else {
        methane_freezes(air_n2)
    }
}

/// The temperature under which the hydrocarbon sea of a body whose air
/// holds `n2` of nitrogen freezes: the liquidus of a sea of methane,
/// ethane and the nitrogen dissolved in it ([`hydrocarbon_freezes`]).
///
/// The sea's nitrogen mole fraction is [`NITROGEN_RAOULT_SHARE`] of
/// Raoult's law at `n2` and methane's triple point (liquid nitrogen's
/// saturation there, [`nitrogen_saturation_pressure`], Span et al. 2000:
/// 3.824 bar), at most [`MAX_NITROGEN_IN_LIQUID`];
/// its methane [`SEA_METHANE_SHARE`]; the rest ethane, which methane's
/// photolysis makes on any methane world. The methane share is fixed, so
/// the methane liquidus is a floor (about 68 K): the dissolved nitrogen
/// replaces ethane, and lowers the freezing point only by diluting it.
pub fn methane_freezes(n2: Bar) -> Kelvin {
    let saturation = bar(nitrogen_saturation_pressure(Kelvin(
        METHANE_TRIPLE_TEMPERATURE,
    )));
    let nitrogen = (NITROGEN_RAOULT_SHARE * n2.0.max(0.0) / saturation).min(MAX_NITROGEN_IN_LIQUID);
    let ethane = (1.0 - SEA_METHANE_SHARE - nitrogen).max(0.0);
    hydrocarbon_freezes(nitrogen, ethane)
}

/// The temperature under which a liquid of methane with mole fractions
/// `nitrogen` of N2 and `ethane` of C2H6 freezes: the higher of its
/// methane and ethane liquidus (the solid that comes out first).
///
/// A simple fit of a phase diagram it does not resolve (the solid
/// solutions, the ternary eutectic). Each liquidus solves
/// `ln x + (W / R)(1 - x)^2 / T = -(L / R)(1/T - 1/T_m)` (Schröder-van
/// Laar, with a regular solution's activity), which is linear in `1/T`
/// and solved exactly: ethane's ideal (`W` = 0), with its triple point
/// and [`ETHANE_FUSION_ENTHALPY`]; methane's with
/// [`METHANE_EXCESS_W_OVER_R`], its fusion enthalpy and its triple point,
/// nitrogen and ethane taken alike. Pure methane freezes at its triple
/// point. Richardson, Hartwig & Leachman 2019 measured two liquids
/// freezing at 71.5 K and 74.0 K.
pub fn hydrocarbon_freezes(nitrogen: f64, ethane: f64) -> Kelvin {
    let nitrogen = nitrogen.clamp(0.0, 1.0);
    let ethane = ethane.clamp(0.0, 1.0 - nitrogen);
    let methane = 1.0 - nitrogen - ethane;
    let liquidus = |x: f64, triple: f64, enthalpy: f64, w_over_r: f64| {
        if x <= 0.0 {
            return 0.0;
        }
        let l_over_r = enthalpy / MOLAR_GAS_CONSTANT;
        let inverse = (l_over_r / triple - x.ln()) / (l_over_r + w_over_r * (1.0 - x).powi(2));
        1.0 / inverse
    };
    let methane = liquidus(
        methane,
        METHANE_TRIPLE_TEMPERATURE,
        METHANE_FUSION_ENTHALPY,
        METHANE_EXCESS_W_OVER_R,
    );
    let ethane = liquidus(
        ethane,
        ETHANE_TRIPLE_TEMPERATURE.0,
        ETHANE_FUSION_ENTHALPY,
        0.0,
    );
    Kelvin(methane.max(ethane))
}

/// The methane in the air over a Titan sea at `t`: [`SEA_METHANE_SHARE`]
/// of pure methane's saturation pressure (Raoult's law). A body needs
/// that much methane for a hydrocarbon liquid; infinite past methane's
/// critical point (190.564 K).
pub fn methane_sea_vapour(t: Kelvin) -> Bar {
    Bar(SEA_METHANE_SHARE * methane_saturation_bar(t.0))
}

/// Whether `ch4` of methane in the air condenses on a surface at `t`: it
/// reaches the vapour of a Titan sea there ([`methane_sea_vapour`]). A
/// trace of methane stays a gas and makes no lake; past methane's
/// critical point none condenses.
pub fn methane_condenses(ch4: Bar, t: Kelvin) -> bool {
    ch4.0 > 0.0 && ch4.0 >= methane_sea_vapour(t).0
}

/// The share of a condensate frozen at `t` near the point `melts` it
/// melts at: 1 at and under `melts - PHASE_BAND`, 0 at and above `melts`
/// (so that above it nothing changes), linear between ([`PHASE_BAND`], a
/// numerical smoothing).
pub fn frozen_share(t: Kelvin, melts: Kelvin) -> Fraction {
    share((melts.0 - t.0) / PHASE_BAND.0)
}

/// The frost left of `condensate` with the share `frozen` of it frozen
/// ([`frozen_share`]): all of it when all is frozen; in the band where it
/// melts, that share of the most that counts towards its cover
/// ([`FROST_LAYER`]), so that its cover, and with it the albedo and the
/// emissivity, retreats across the whole band rather than at its warm
/// end.
pub fn melting(condensate: Bar, frozen: Fraction) -> Bar {
    let condensate = condensate.0.max(0.0);
    if frozen.get() >= 1.0 {
        Bar(condensate)
    } else {
        Bar(frozen.get() * condensate.min(FROST_LAYER.0))
    }
}

/// The frost on the ground of a body at `t` under `air`, bar, gas by gas:
///
/// - the N2 of `condensate` under nitrogen's triple point (above it, a
///   liquid), and its CH4 under the sea's freezing point
///   ([`sea_freezes`]; above it, a sea), each melting over the last
///   [`PHASE_BAND`] under its point ([`melting`]);
/// - the CO2 the cold trap froze out (`frozen_co2`) that the air, at most
///   CO2's sublimation pressure there, cannot take back;
/// - the SO2 frost its volcanoes lay (`so2_frost`), less what the air
///   takes back the same way at SO2's sublimation pressure.
///
/// Over a frost's triple point its curve gives no pressure, read as an
/// infinite one: the frost all goes back to the air.
pub fn frost(
    condensate: &PartialPressures,
    frozen_co2: Bar,
    so2_frost: Bar,
    t: Kelvin,
    air: &PartialPressures,
) -> PartialPressures {
    let n2 = melting(
        Bar(condensate.n2),
        frozen_share(t, NITROGEN_TRIPLE_TEMPERATURE),
    );
    let ch4 = melting(
        Bar(condensate.ch4),
        frozen_share(t, sea_freezes(Bar(air.n2), condensate)),
    );
    let kept = |frozen: Bar, sublimes: Option<Pascal>, in_air: f64| {
        let sublimes = sublimes.map_or(f64::INFINITY, bar);
        (frozen.0.max(0.0) - (sublimes - in_air.max(0.0)).max(0.0)).max(0.0)
    };
    PartialPressures {
        n2: n2.0,
        ch4: ch4.0,
        co2: kept(frozen_co2, co2_frost_pressure(t), air.co2),
        so2: kept(so2_frost, sulphur_dioxide_frost_pressure(t), air.so2),
        ..PartialPressures::default()
    }
}

#[cfg(test)]
#[path = "liquids_tests.rs"]
mod tests;
