//! The chance that a planet whose climate has both a temperate and a
//! frozen state is found frozen: a snowball lasts until its volcanoes
//! have given it the CO₂ that thaws it, and the temperate phase between
//! two snowballs lasts as long as Earth's volcanism takes to give
//! [`SNOWBALL_WARM_BAR`].
//!
//! Ported from `legacy-final:crates/mapgen/src/planet/climate.rs`
//! (`snowball_chance` and `SNOWBALL_WARM_BAR`, l.1575-1612), with the
//! volcanism of the rewrite (`relief::volcanoes::volcanic_intensity`,
//! against [`EARTH_VOLCANIC_INTENSITY`]) and the refit of study 25
//! (`docs/science/planet-atmospheres.md`, "Calibration of
//! `SNOWBALL_WARM_BAR` (refit)").

use crate::relief::volcanoes::EARTH_VOLCANIC_INTENSITY;
use crate::units::{Bar, Fraction};

/// Calibration: the CO₂ Earth's volcanism gives the air over one
/// temperate phase between two snowballs, 9.80 bar. Target: Earth's
/// dated share of its history in snowball states, `f⊕` = 0.014 (0.013
/// to 0.016; the Sturtian and Marinoan glaciations, 63 My over 4.6 Gyr,
/// Hoffman et al. 2017, *Sci. Adv.* 3, e1600983, Table 1). Method:
/// `W = d⊕ (1 − f⊕) / (rate⊕ f⊕)` (study 25) = 9.80, to three figures,
/// with `d⊕` = 0.1408 bar the extra CO₂ the reference Earth at 1 AU
/// needs to thaw (thaw 0.144919 bar less froze 0.004071 bar, 0.140848,
/// measured by the climate solver of `system` at its 13 thaw steps,
/// under the stellar spectra of issue #655, with Earth's argon of #694,
/// the cloud albedo refitted with ozone over the climate's own ground,
/// issue #705, and the N₂ and O₂ pairs of #703; it was 0.1458 bar and
/// `W` 10.1 bar before the pairs, 0.1468 bar and 10.2 bar with the cloud
/// albedo of #655, 0.1788 bar and 12.4 bar under the black-body Sun) and
/// `rate⊕` = 0.506064 / 0.5 = 1.0121, its recorded volcanic intensity
/// over [`EARTH_VOLCANIC_INTENSITY`]; Earth's chance is then 0.0140.
/// Searched and not found: a snowball frequency for another planet.
/// Study:
/// [25](https://github.com/cyboooooorg/map-generator/wiki/25-climate-solver-audit),
/// `docs/science/planet-atmospheres.md`, "Calibration of
/// `SNOWBALL_WARM_BAR` (refit)".
pub const SNOWBALL_WARM_BAR: Bar = Bar(9.80);

/// The share of its history a bistable planet spends frozen, from the
/// CO₂ it needs `extra` above what it froze with to thaw (`thaw − froze`;
/// `None` when no CO₂ it can hold thaws it) and its volcanic intensity
/// `volcanism`:
///
/// ```text
/// chance = t_frozen / (t_frozen + t_warm) = d / (d + W · v / v⊕)
/// ```
///
/// with `d` the extra CO₂ (held at 0 or more), `W` [`SNOWBALL_WARM_BAR`]
/// and `v / v⊕` the volcanism against [`EARTH_VOLCANIC_INTENSITY`]: the
/// snowball lasts `d` over the outgassing rate and the temperate phase
/// `W` over Earth's. Rare near Earth's orbit, where a little CO₂ thaws
/// the ice; likelier towards the outer edge of the habitable zone, where
/// more is needed, and on a planet that outgasses little. It is 1 when
/// no CO₂ thaws the body (`extra` is `None`) or nothing outgasses
/// (`volcanism` 0), never the `0 / 0` of a body with neither.
pub fn snowball_chance(extra: Option<Bar>, volcanism: Fraction) -> Fraction {
    let rate = volcanism.get() / EARTH_VOLCANIC_INTENSITY;
    let Some(extra) = extra.filter(|_| rate > 0.0) else {
        return Fraction::new(1.0).expect("one is a fraction");
    };
    let d = extra.0.max(0.0);
    Fraction::new(d / (d + SNOWBALL_WARM_BAR.0 * rate))
        .expect("a share of two non-negative times is a fraction")
}

#[cfg(test)]
#[path = "snowball_tests.rs"]
mod tests;
