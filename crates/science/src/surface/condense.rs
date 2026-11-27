//! The nitrogen, methane and argon that condense on a body's surface at
//! the temperature the climate solves for, and the air they leave.
//!
//! Ported from `legacy-final` (`planet/atmosphere.rs` l.760-885:
//! `Condensable`, `CONDENSING`, `methane_activity`, `condense`,
//! `condensed_air`; l.1183: `titan_like_air`), with the saturation curves
//! of `science::volatiles` (Span et al. 2000 for liquid nitrogen instead
//! of legacy's Clausius-Clapeyron; Setzmann and Wagner 1991 for methane)
//! and the sea of `super::liquids`. Study
//! [25](https://github.com/cyboooooorg/map-generator/wiki/25-climate-solver-audit),
//! "Condensation at the surface"; argon, which legacy did not have, from
//! study [29b](https://github.com/cyboooooorg/map-generator/wiki/29b-argon-frost);
//! see `docs/science/planet-atmospheres.md`.

use super::liquids::{SEA_METHANE_SHARE, bar, frozen_share, sea_freezes};
use crate::radiation::gases::{AR_MOLAR_MASS, CH4_MOLAR_MASS, N2_MOLAR_MASS, Weights};
use crate::units::{Bar, Kelvin};
use crate::volatiles::{
    METHANE_TRIPLE_TEMPERATURE, argon_saturation_pressure, methane_saturation_bar,
    nitrogen_saturation_pressure,
};

/// What a surface leaves of an inventory of gases: the air and the
/// condensate on the ground, as weights (bar, [`Weights`]).
/// `air + condensate` is the inventory.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Condensed {
    /// The gases left in the air.
    pub air: Weights,
    /// The N2, CH4 and Ar on the ground, as a liquid or a frost.
    pub condensate: Weights,
}

/// Definition: the most rounds of [`condense`], 50, a guard: each round
/// shrinks the gap by about the share of the air the other gases' caps
/// move, so gases that condense together settle in a few rounds; one
/// that stops at the guard keeps the last round's air.
pub const CONDENSE_ROUNDS: usize = 50;

/// Definition: the relative move of every weight under which the rounds
/// of [`condense`] have settled, 1e-12: a few roundings.
pub const CONDENSE_SETTLED: f64 = 1e-12;

/// The air and the condensate of `inventory` (bar, weights) on a surface
/// at `t`. Each gas that condenses there keeps in the air at most the
/// weight whose partial pressure is its activity times its saturation
/// pressure at `t` ([`Weights::weight_at`] over the other gases' weights
/// as they are then), and the rest lies on the ground:
///
/// - nitrogen, with activity 1 (a pure frost, which Triton's air sits in
///   equilibrium with, issue #464; a condensed liquid is nitrogen-rich,
///   issue #524), under [`nitrogen_saturation_pressure`];
/// - methane, under its saturation pressure, with activity
///   `SEA_METHANE_SHARE` over a sea (Raoult's law) and 1 over its own
///   ice (a pure solid, issue #530), ramped by [`frozen_share`] over the
///   last `PHASE_BAND` under the sea's freezing point ([`sea_freezes`],
///   which reads the partial pressure of the nitrogen the air keeps and
///   whether nitrogen condenses): a numerical smoothing without which the
///   climate's fixed point cycles (#562);
/// - argon, with activity 1 over its own frost or liquid (the rule of
///   methane over its ice: the most argon of the three pictures of a
///   minor volatile over N2 ice, Lellouch et al. 2010; study 29b), under
///   [`argon_saturation_pressure`].
///
/// A gas that condenses raises the others' partial pressures, so the
/// three are capped in turn, each from its inventory over the others as
/// the last cap left them, in rounds until no weight moves by more than
/// [`CONDENSE_SETTLED`] of itself (at most [`CONDENSE_ROUNDS`]): every
/// condensing gas then sits at its cap together, as in phase
/// equilibrium. CO2, H2O, NH3, SO2 and O2 pass through: CO2 collapses in
/// the climate, NH3 is frozen by the cold trap. Past a gas's critical
/// point its saturation is infinite and none of it condenses. A
/// `Definition:` (phase equilibrium and Raoult's law, on partial
/// pressures; issue #749); its one number is `SEA_METHANE_SHARE`.
pub fn condense(inventory: &Weights, t: Kelvin) -> Condensed {
    let n2_cap = Bar(bar(nitrogen_saturation_pressure(t)));
    let ar_cap = Bar(bar(argon_saturation_pressure(t)));
    let methane = methane_saturation_bar(t.0);
    // What the air keeps of `stock` when it may hold `most`, and what
    // lies on the ground (none of a NaN or negative stock).
    let keep = |stock: f64, most: Bar| if stock > most.0 { most.0 } else { stock };
    let ground = |stock: f64, kept: f64| if stock > kept { stock - kept } else { 0.0 };
    let mut air = *inventory;
    for _ in 0..CONDENSE_ROUNDS {
        let before = air;
        air.n2 = keep(
            inventory.n2,
            Weights::weight_at(&Weights { n2: 0.0, ..air }, N2_MOLAR_MASS, n2_cap),
        );
        let nitrogen = Weights {
            n2: ground(inventory.n2, air.n2),
            ..Weights::default()
        };
        let n2_pressure = Bar(air.partial_pressures().n2);
        let ice = frozen_share(t, sea_freezes(n2_pressure, &nitrogen)).get();
        let activity = SEA_METHANE_SHARE + (1.0 - SEA_METHANE_SHARE) * ice;
        air.ch4 = keep(
            inventory.ch4,
            Weights::weight_at(
                &Weights { ch4: 0.0, ..air },
                CH4_MOLAR_MASS,
                Bar(activity * methane),
            ),
        );
        air.ar = keep(
            inventory.ar,
            Weights::weight_at(&Weights { ar: 0.0, ..air }, AR_MOLAR_MASS, ar_cap),
        );
        let moved = |a: f64, b: f64| (a - b).abs() > CONDENSE_SETTLED * a.abs().max(b.abs());
        if !(moved(air.n2, before.n2) || moved(air.ch4, before.ch4) || moved(air.ar, before.ar)) {
            break;
        }
    }
    let condensate = Weights {
        n2: ground(inventory.n2, air.n2),
        ch4: ground(inventory.ch4, air.ch4),
        ar: ground(inventory.ar, air.ar),
        ..Weights::default()
    };
    Condensed { air, condensate }
}

/// `air` with its N2, CH4 and Ar set to what their `inventory` (bar,
/// weights, in the air and on the ground) leaves in it at `t`
/// ([`condense`] of those three gases over `air`'s others): the air a
/// climate solved at a temperature close to `t`, made to agree with it
/// exactly. The other gases of `air` pass through.
pub fn condensed_air(air: &Weights, inventory: &Weights, t: Kelvin) -> Weights {
    condense(&condensables(air, inventory), t).air
}

/// `air` holding the N2, CH4 and Ar of `inventory`: what condenses over
/// `air`'s other gases ([`condensed_air`]).
pub fn condensables(air: &Weights, inventory: &Weights) -> Weights {
    Weights {
        n2: inventory.n2,
        ch4: inventory.ch4,
        ar: inventory.ar,
        ..*air
    }
}

/// Whether the air a climate leaves (bar, weights) is a Titan's: it
/// still holds methane, which makes its haze, and its total pressure is
/// over methane's triple-point pressure (0.117 bar, the saturation at
/// `METHANE_TRIPLE_TEMPERATURE`), dense enough for a hydrocarbon liquid.
/// A Triton's thin nitrogen air, a few microbar, is not one. A
/// `Definition:` (study 25, "The reducing-air fallback").
pub fn titan_like_air(air: &Weights) -> bool {
    air.ch4 > 0.0 && air.total() > methane_saturation_bar(METHANE_TRIPLE_TEMPERATURE)
}

#[cfg(test)]
#[path = "condense_tests.rs"]
mod tests;
