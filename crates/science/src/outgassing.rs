//! What a body's interior brings up to its air, and what the air keeps
//! of the sulphur dioxide.
//!
//! **Outgassing.** A regime brings up a share of each volatile over the
//! body's life ([`MOBILE_SHARE`], [`LID_SHARE`]), less what subduction
//! returns under plates ([`RECYCLED_UNDER_PLATES`],
//! [`NITROGEN_RECYCLED_UNDER_PLATES`]), times the share a deep water layer
//! lets through ([`released_under_ocean`]). A mass of gas weighs
//! [`column_pressure`] on the surface. The shares are calibrations within
//! the picture of Kite, Manga & Gaidos 2009 (ApJ 700, 1732,
//! doi:10.1088/0004-637X/700/2/1732: plates and lids outgas through
//! melting, lids less of it, "melting is necessary but insufficient";
//! the paper gives no share). Which share a body takes is the caller's:
//! this module knows no regime.
//!
//! **Sulphur.** Sulphur comes up as SO2 ([`SO2_PER_S`]), of which
//! [`SULPHUR_IN_AIR`] stays in the air; a wet or oxidising air washes it
//! out, a dry one keeps it in proportion to its column ([`so2_kept`]).
//!
//! **Ammonia.** Of the NH3 the interior brings up, [`AMMONIA_DECOMPOSED`]
//! comes up as N2, decomposed in the interior ([`decomposed`]): Titan's
//! nitrogen (Glein 2015, study
//! [28](https://github.com/cyboooooorg/map-generator/wiki/28-titan-nitrogen-audit)).
//!
//! Ported from the tag `legacy-final` (`planet/atmosphere.rs` l.24-124,
//! 182-280) with the values and citations of study
//! [21](https://github.com/cyboooooorg/map-generator/wiki/21-escape-outgassing-audit),
//! parts 5 and 6; see `docs/science/planet-atmospheres.md`, "Atmosphere".

use crate::photochemistry::N2_PER_NH3;
use crate::units::{Bar, Fraction, Kilogram, Metre, MetrePerSecondSquared, Pascal};

/// Calibration: share of each volatile that plates, an episodic lid
/// (Venus's resurfacing) and heat pipes (Io) bring up over a body's life:
/// nearly all of it. Within the picture of Kite, Manga & Gaidos 2009,
/// which gives no share; fitted on Venus's 90 bar of CO2 (study 21,
/// part 5).
pub const MOBILE_SHARE: f64 = 0.95;

/// Calibration: share a stagnant lid brings up: it melts its upper
/// mantle early and stops ("stagnant lid planets live fast ... but die
/// young", Kite, Manga & Gaidos 2009, abstract; no number). Fitted on
/// Mars and Titan (study 21, part 5).
pub const LID_SHARE: f64 = 0.2;

/// Calibration: pressure at the floor of a water layer at which half of
/// the volatiles reach the air, 3.7 GPa. Fitted so that a 1 M_earth planet
/// of 30 % water at 1 au lets out about 1e-3 of its air (a few bar of CO2
/// before the thermostat, within the 0.2 to 20 bar Kite & Ford 2018 find
/// on waterworlds) while Titan, Ganymede (1.1 GPa) and Callisto keep 98 %
/// or more of it. Retuned from legacy's 3 GPa for the rewrite's interior,
/// which makes that planet smaller (1.143 R_earth against legacy's 1.187)
/// and its seafloor deeper (P = 2.016e5 bar against 1.735e5): with
/// `P / P½ = 999^(1/k)`, `k = 4.1`, `P½ = 2.016e5 / 5.39 = 3.74e4` bar,
/// rounded to 3.7e4 (the planet then lets out 9.6e-4; Ganymede keeps
/// 99.3 %). Kite & Ford 2018 (ApJ 864, 75, doi:10.3847/1538-4357/aad6e0,
/// Fig. 5) find 3 GPa a full shutoff of a stagnant lid's melting, not a
/// half point: a shutoff there leaves a planet of 30 % water no air at
/// all (study 21, part 5).
pub const SEAFLOOR_HALF_PRESSURE: Bar = Bar(3.7e4);

/// Calibration: steepness `k` of the release across
/// [`SEAFLOOR_HALF_PRESSURE`], `1 / (1 + (P / P½)^k)`: the least that
/// keeps Ganymede at 98 % (4 left it 97.9 %). No source (study 21,
/// part 5).
pub const SEAFLOOR_STEEPNESS: f64 = 4.1;

/// Calibration: share of the carbon, sulphur and water plates outgas
/// that subduction carries back into the mantle. Set equal to the
/// nitrogen's before that was raised; no target constrains it once the
/// thermostat takes Earth's CO2. Kelemen & Manning 2015 (PNAS 112, E3997,
/// doi:10.1073/pnas.1507889112, as quoted) put the return lower, about
/// half or less (study 21, part 5).
pub const RECYCLED_UNDER_PLATES: f64 = 0.63;

/// Calibration: share of the nitrogen (N2 and NH3) plates outgas that
/// subduction carries back: ammonium in clays and sediments rides the
/// slabs down. Fitted on the weight of Earth's N2, 0.746 bar (study 23 of
/// the Wiki; legacy fitted 0.8 on 0.78 bar, a share by volume): the
/// chain's N2 on Earth, its outgassed N2 plus the N2 its photolysed NH3
/// makes, is proportional to the share kept, `1 - x`. Study 23 fitted
/// 0.8145 (legacy's 0.8 gave 0.804 bar); once Earth's ocean carried no N2
/// ice (`volatiles::traps_nitrogen`, study 28) 0.8145 gave 0.6799 bar,
/// so `x = 1 - 0.1855 × 0.746 / 0.6799`, kept to four digits, 0.7965,
/// which gives 0.7458 bar, by running the chain on the reference Earth.
/// See the calibration on the
/// page `docs/science/planet-atmospheres.md` (*Outgassing*, *Earth's N₂
/// target*). Goldblatt et al. 2009 (doi:10.1038/ngeo692, abstract) and
/// Johnson & Goldblatt 2015 (doi:10.1016/j.earscirev.2015.05.006,
/// abstract: the silicate Earth holds "7 +/- 4 times present atmospheric
/// N") support a large return without giving one (study 21, part 5).
pub const NITROGEN_RECYCLED_UNDER_PLATES: f64 = 0.7965;

/// Calibration: share `ξ` of the NH3 a body brings up that its interior
/// decomposes to N2 before it reaches the air (`2 NH3 → N2 + 3 H2`, the
/// reaction progress of Glein 2015, Icarus 250, 570,
/// doi:10.1016/j.icarus.2015.01.001, eq. as quoted by Erkaev et al. 2020,
/// MNRAS, arXiv:2010.12342, eq. 42: `ξ ≤ 1`). Fitted on Titan's N2, 1.384
/// bar (1.467 bar, Fulchignoni et al. 2005, less its 5.65 % CH4, Niemann
/// et al. 2010): the reference Titan's N2 is its rock's 0.005 bar plus
/// `ξ` times the 11.81 bar of N2 its kept NH3 would make, so
/// `ξ = 1.379 / 11.81 = 0.1168`, kept to three digits. Titan's own N2
/// came from NH3 (14N/15N, Mandt et al. 2014; Scherf et al. 2020,
/// doi:10.1007/s11214-020-00752-0, section 3), not from N2 ice
/// (`volatiles::traps_nitrogen`); no measured `ξ` was found, and the
/// N2 Titan lost to space (0.5 to 2.2 times its present air for a slow to
/// moderate young Sun, Erkaev et al. 2020, Table 3), which the model's
/// escape does not take, is not in the fit. See the calibration on the
/// page `docs/science/planet-atmospheres.md` (*Titan's nitrogen*) and
/// study 28.
pub const AMMONIA_DECOMPOSED: f64 = 0.117;

/// Definition: mass of SO2 per mass of sulphur, 64.07 / 32.06 = 1.998,
/// rounded to 2 as in the legacy.
pub const SO2_PER_S: f64 = 2.0;

/// Calibration: share of the outgassed SO2 that stays in the air, the
/// rest ending in sulphides and sulphates at the surface (Venus's SO2 is
/// lost to its rocks, Fegley & Prinn 1989, Nature 337, 55,
/// doi:10.1038/337055a0, abstract; the share is not in it). Fitted on
/// Venus's SO2 below the clouds, 130 ± 50 ppmv of 92 bar (Marcq et al.
/// 2008, as quoted by doi:10.1051/0004-6361/202140837) (study 21,
/// part 5).
pub const SULPHUR_IN_AIR: f64 = 5.0e-5;

/// Calibration: free oxygen from which an air washes its SO2 out as it
/// does over liquid water, a twentieth of Earth's O2: oxygen and the
/// radicals it makes oxidise SO2 to sulphate long before the air is
/// Earth's. No paper read sets a threshold (study 21, part 6).
pub const OXIDISING_O2: Bar = Bar(0.01);

/// Calibration: share of its SO2 an air keeps over liquid water or with
/// free oxygen: the wet lifetime, about 1.5 days (1 to 3 days in a review
/// of Kilauea's sulphur chemistry, as quoted; Seinfeld & Pandis 2016 not
/// reached), over the dry lifetime of Venus's SO2 against its crust,
/// 1.9 Myr (Fegley & Prinn 1989, abstract, as quoted): 1.296e5 s over
/// 5.996e13 s. Bullock & Grinspoon 2001 (doi:10.1006/icar.2000.6570, as
/// quoted) find a dry lifetime ten times longer, which would give 2e-10;
/// Earth's SO2 stays under 1e-6 bar at either (study 21, part 6).
pub const WET_SO2_SHARE: f64 = 2.2e-9;

/// Calibration: column of a dry, oxygen-free air from which it keeps all
/// its SO2; a thinner one keeps it in proportion to its column (the dry
/// sink is photochemical, at a rate per unit area that does not grow with
/// the column: the legacy's reasoning, not a read result). Fitted on
/// Venus (130 ± 50 ppmv below the clouds) and Mars together (under its
/// upper limit, Krasnopolsky 2012, Icarus 217, 144,
/// doi:10.1016/j.icarus.2011.10.019; the limit itself not read) (study
/// 21, part 6).
pub const DRY_COLUMN: Bar = Bar(10.0);

/// Pressure a `mass` of gas spread over a body of surface `gravity` and
/// `radius` puts on its surface: `P = m g / (4π R²)`, the weight of the
/// column over the area. Takes a positive radius.
pub fn column_pressure(mass: Kilogram, gravity: MetrePerSecondSquared, radius: Metre) -> Bar {
    let area = 4.0 * std::f64::consts::PI * radius.0 * radius.0;
    Pascal(mass.0 * gravity.0 / area).into()
}

/// Share of the volatiles a body outgasses that reaches the air under a
/// water layer whose floor bears `seafloor`:
/// `1 / (1 + (P / P½)^k)` ([`SEAFLOOR_HALF_PRESSURE`],
/// [`SEAFLOOR_STEEPNESS`]); all of it without water.
///
/// The weight of a deep water layer curtails the decompression melting
/// that outgasses the mantle (Kite & Ford 2018), high-pressure ice at its
/// base seals the rock (Noack et al. 2016, Icarus 277, 215,
/// doi:10.1016/j.icarus.2016.05.009, record only), and the ice's own
/// volatiles stay dissolved or in clathrates: one share for all. A
/// runaway's steam weighs on the rock as its ocean did, so the share
/// holds in a runaway too. Takes a finite pressure.
pub fn released_under_ocean(seafloor: Bar) -> Fraction {
    let ratio = seafloor.0.max(0.0) / SEAFLOOR_HALF_PRESSURE.0;
    Fraction::new(1.0 / (1.0 + ratio.powf(SEAFLOOR_STEEPNESS)))
        .expect("a share in 0 to 1 for a finite pressure")
}

/// Share of the SO2 the volcanoes gave that an air keeps: over liquid
/// water (`wet`) or with `o2` of [`OXIDISING_O2`] or more, the wet share
/// [`WET_SO2_SHARE`]; otherwise `min(total / DRY_COLUMN, 1)` of a surface
/// pressure `total` ([`DRY_COLUMN`]): the thinner the dry air, the less
/// it keeps. Sulphuric acid clouds are not made here. Takes finite
/// pressures.
pub fn so2_kept(wet: bool, o2: Bar, total: Bar) -> Fraction {
    let kept = if wet || o2.0 >= OXIDISING_O2.0 {
        WET_SO2_SHARE
    } else {
        (total.0.max(0.0) / DRY_COLUMN.0).min(1.0)
    };
    Fraction::new(kept).expect("a share in 0 to 1 for finite pressures")
}

/// What the interior brings up of a mass of NH3: N2 and the NH3 left.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Decomposed {
    /// The N2 the decomposed NH3 makes.
    pub n2: Kilogram,
    /// The NH3 left whole.
    pub nh3: Kilogram,
}

/// `ammonia` brought up from the interior once [`AMMONIA_DECOMPOSED`] of
/// it is decomposed: that share becomes N2 at `N2_PER_NH3` of its mass
/// (`2 NH3 → N2 + 3 H2`, the hydrogen lost), the rest stays NH3, which the
/// cold trap and the photochemistry read as before.
pub fn decomposed(ammonia: Kilogram) -> Decomposed {
    Decomposed {
        n2: Kilogram(ammonia.0 * AMMONIA_DECOMPOSED * N2_PER_NH3),
        nh3: Kilogram(ammonia.0 * (1.0 - AMMONIA_DECOMPOSED)),
    }
}

#[cfg(test)]
#[path = "outgassing_tests.rs"]
mod tests;
