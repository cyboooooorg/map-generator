//! Whether life arose on a planet with surface water, and the oxygen it
//! gives the air, in two steps.
//!
//! Ported from `legacy-final` (`planet/atmosphere.rs`: the life gate and
//! the oxygenated air), with the values of study
//! [27](https://github.com/cyboooooorg/map-generator/wiki/27-life-and-class-audit)
//! decided on 2026-11-24: life from 1.17 Gyr (legacy 1.0, a calibration),
//! oxygen in two steps (legacy one, at 2.2 Gyr), and O₂ at the CIPM ratio
//! to N₂ (legacy 0.27). Page: `docs/science/planet-atmospheres.md`,
//! "Life". The engine draws the life draw from the planet's seed; this
//! module only reads it.

use crate::radiation::gases::{N2_MOLAR_MASS, O2_MOLAR_MASS, PartialPressures};
use crate::units::Gyr;

/// Source: the oldest generally accepted evidence of life after Earth
/// formed, 1.17 Gyr: the age of the CAIs, 4.567 Ga (4567.30 ± 0.16 Myr,
/// Connelly et al. 2012, *Science* 338, 651, doi:10.1126/science.1226919,
/// abstract), less the microbial biosphere "by at least 3.4 billion years
/// ago" of Javaux 2019 (*Nature* 572, 451, doi:10.1038/s41586-019-1436-4,
/// abstract). Page: `docs/science/planet-atmospheres.md`, "Life".
pub const LIFE_MIN_GYR: Gyr = Gyr(1.17);

/// Definition: the chance that life arises where it can, 1/3, a game
/// choice set with the user on 2026-11-12. No measured rate exists: one
/// world with life gives no probability (Spiegel & Turner 2012, *PNAS*
/// 109, 395, doi:10.1073/pnas.1111694108). Nothing is calibrated on it.
/// Page: `docs/science/planet-atmospheres.md`, "Life".
pub const LIFE_CHANCE: f64 = 1.0 / 3.0;

/// Source: the permanent rise of O₂ 2.4 to 2.1 Ga (Lyons, Reinhard &
/// Planavsky 2014, *Nature* 506, 307, doi:10.1038/nature13068, p. 307),
/// narrowed to 2.4 to 2.3 Ga (p. 314): 2.17 to 2.27 Gyr after the CAIs,
/// 2.2 Gyr. From it the air is oxidising: no methane, its O₂ under 0.1 %
/// of modern (p. 311), taken as none. Page:
/// `docs/science/planet-atmospheres.md`, "Life".
pub const OXYGEN_AFTER_GYR: Gyr = Gyr(2.2);

/// Source: O₂ "rose to something like modern values about 600 million
/// years (Myr) ago" (Lyons, Reinhard & Planavsky 2014, p. 307): 4.567 Ga
/// less 0.6, 3.97, about 4.0 Gyr after the CAIs. Page:
/// `docs/science/planet-atmospheres.md`, "Life".
pub const MODERN_OXYGEN_AFTER_GYR: Gyr = Gyr(4.0);

/// Source: the mole fraction of N₂ in the reference dry air of the
/// CIPM-2007 formula, 0.780 848 (Picard, Davis, Gläser & Fujii 2008,
/// *Metrologia* 45, 149, Table 1).
pub const CIPM_N2_SHARE: f64 = 0.780_848;

/// Source: the mole fraction of O₂ in the reference dry air of the
/// CIPM-2007 formula, 0.209 390 (Picard et al. 2008, Table 1).
pub const CIPM_O2_SHARE: f64 = 0.209_390;

/// Definition: the O₂ of a modern living air per bar of its N₂, by
/// weight: the CIPM shares times the molar masses,
/// `(O2 share × O2_MOLAR_MASS) / (N2 share × N2_MOLAR_MASS)`, about 0.306.
/// The chain's partial pressures are weights (a gas's mass share times the
/// surface pressure), so the mole-fraction ratio, 0.2682, would give the
/// reference Earth 0.200 bar of O₂ instead of 0.228. Earth's air carried
/// over to another planet by its nitrogen. Page:
/// `docs/science/planet-atmospheres.md`, "The oxygenated air".
pub const O2_PER_N2: f64 = (CIPM_O2_SHARE * O2_MOLAR_MASS) / (CIPM_N2_SHARE * N2_MOLAR_MASS);

/// Where a planet's life stands, and so its air.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// No life: no water, too young, or the draw said no.
    Lifeless,
    /// Life before [`OXYGEN_AFTER_GYR`]: the air it found.
    Anoxic,
    /// From [`OXYGEN_AFTER_GYR`]: no methane, O₂ taken as none.
    Oxidising,
    /// From [`MODERN_OXYGEN_AFTER_GYR`]: modern O₂, no methane.
    Modern,
}

/// The stage of a planet whose surface liquid is water now (`water`), in
/// a system `age` old, with its life `draw` in `[0, 1)`: [`Stage::Lifeless`]
/// unless `water`, `age` at least [`LIFE_MIN_GYR`] and `draw` under
/// [`LIFE_CHANCE`]; then by age, each step from its threshold on. The
/// model counts from the system's formation, not from when the planet
/// became habitable, which it does not know (a declared simplification).
pub fn stage(water: bool, age: Gyr, draw: f64) -> Stage {
    if !(water && age >= LIFE_MIN_GYR && draw < LIFE_CHANCE) {
        Stage::Lifeless
    } else if age >= MODERN_OXYGEN_AFTER_GYR {
        Stage::Modern
    } else if age >= OXYGEN_AFTER_GYR {
        Stage::Oxidising
    } else {
        Stage::Anoxic
    }
}

/// The air of a planet at `stage`: unchanged without life and before
/// [`OXYGEN_AFTER_GYR`]; without methane from it (OH destroys it within
/// about nine years in an oxygenated air, and the model has no flux that
/// would keep its steady state: a `Definition:` of the model's scope);
/// and with `O2_PER_N2 × N₂` of O₂ from [`MODERN_OXYGEN_AFTER_GYR`]. The
/// other gases pass through.
pub fn oxygenate(air: &PartialPressures, stage: Stage) -> PartialPressures {
    match stage {
        Stage::Lifeless | Stage::Anoxic => *air,
        Stage::Oxidising => PartialPressures { ch4: 0.0, ..*air },
        Stage::Modern => PartialPressures {
            ch4: 0.0,
            o2: O2_PER_N2 * air.n2,
            ..*air
        },
    }
}

#[cfg(test)]
#[path = "life_tests.rs"]
mod tests;
