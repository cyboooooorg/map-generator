//! The nitrogen and methane that condense on a body's surface at the
//! temperature the climate solves for, and the air they leave.
//!
//! Ported from `legacy-final` (`planet/atmosphere.rs` l.760-885:
//! `Condensable`, `CONDENSING`, `methane_activity`, `condense`,
//! `condensed_air`; l.1183: `titan_like_air`), with the saturation curves
//! of `science::volatiles` (Span et al. 2000 for liquid nitrogen instead
//! of legacy's Clausius-Clapeyron; Setzmann and Wagner 1991 for methane)
//! and the sea of `super::liquids`. Study
//! [25](https://github.com/cyboooooorg/map-generator/wiki/25-climate-solver-audit),
//! "Condensation at the surface"; see `docs/science/planet-atmospheres.md`.

use super::liquids::{SEA_METHANE_SHARE, bar, frozen_share, sea_freezes};
use crate::radiation::gases::PartialPressures;
use crate::units::{Bar, Kelvin};
use crate::volatiles::{
    METHANE_TRIPLE_TEMPERATURE, methane_saturation_bar, nitrogen_saturation_pressure,
};

/// What a surface leaves of an inventory of gases: the air and the
/// condensate on the ground, bar. `air + condensate` is the inventory.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Condensed {
    /// The gases left in the air.
    pub air: PartialPressures,
    /// The N2 and CH4 on the ground, as a liquid or a frost.
    pub condensate: PartialPressures,
}

/// The air and the condensate of `inventory` (bar) on a surface at `t`.
/// Each gas that condenses there keeps in the air at most its activity
/// times its saturation pressure at `t`, and the rest lies on the ground:
///
/// - nitrogen first, with activity 1 (a pure frost, which Triton's air
///   sits in equilibrium with, issue #464; a condensed liquid is
///   nitrogen-rich, issue #524), under [`nitrogen_saturation_pressure`];
/// - then methane, under its saturation pressure, with activity
///   `SEA_METHANE_SHARE` over a sea (Raoult's law) and 1 over its own
///   ice (a pure solid, issue #530), ramped by [`frozen_share`] over the
///   last `PHASE_BAND` under the sea's freezing point ([`sea_freezes`],
///   which reads the nitrogen the air keeps and condenses): a numerical
///   smoothing without which the climate's fixed point cycles (#562).
///
/// CO2, H2O, NH3, SO2 and O2 pass through: CO2 collapses in the climate,
/// NH3 is frozen by the cold trap. Past a gas's critical point its
/// saturation is infinite and none of it condenses. A `Definition:`
/// (phase equilibrium and Raoult's law); its one number is
/// `SEA_METHANE_SHARE`.
pub fn condense(inventory: &PartialPressures, t: Kelvin) -> Condensed {
    let mut air = *inventory;
    let mut condensate = PartialPressures::default();
    let n2 = bar(nitrogen_saturation_pressure(t));
    if air.n2 > n2 {
        condensate.n2 = air.n2 - n2;
        air.n2 = n2;
    }
    let ice = frozen_share(t, sea_freezes(Bar(air.n2), &condensate)).get();
    let activity = SEA_METHANE_SHARE + (1.0 - SEA_METHANE_SHARE) * ice;
    let ch4 = activity * methane_saturation_bar(t.0);
    if air.ch4 > ch4 {
        condensate.ch4 = air.ch4 - ch4;
        air.ch4 = ch4;
    }
    Condensed { air, condensate }
}

/// `air` with its N2 and CH4 set to what their `inventory` (bar, in the
/// air and on the ground) leaves in the air at `t` ([`condense`]): the
/// air a climate solved at a temperature close to `t`, made to agree
/// with it exactly. The other gases of `air` pass through.
pub fn condensed_air(
    air: &PartialPressures,
    inventory: &PartialPressures,
    t: Kelvin,
) -> PartialPressures {
    let kept = condense(inventory, t).air;
    PartialPressures {
        n2: kept.n2,
        ch4: kept.ch4,
        ..*air
    }
}

/// Whether the air a climate leaves is a Titan's: it still holds methane,
/// which makes its haze, and its total pressure is over methane's
/// triple-point pressure (0.117 bar, the saturation at
/// `METHANE_TRIPLE_TEMPERATURE`), dense enough for a hydrocarbon liquid.
/// A Triton's thin nitrogen air, a few microbar, is not one. A
/// `Definition:` (study 25, "The reducing-air fallback").
pub fn titan_like_air(air: &PartialPressures) -> bool {
    air.ch4 > 0.0 && air.total() > methane_saturation_bar(METHANE_TRIPLE_TEMPERATURE)
}

#[cfg(test)]
#[path = "condense_tests.rs"]
mod tests;
