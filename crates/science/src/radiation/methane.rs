//! Methane's anti-greenhouse: the share of the starlight that the methane
//! above the tropopause, and the haze its photolysis makes, takes and
//! sends back to space (McKay, Pollack & Courtin 1991).
//!
//! Methane's greenhouse is not here: it is the collision-induced pairs
//! N₂–CH₄ and CH₄–CH₄ of HITRAN's files in the spectrum
//! (`spectrum::PAIRS`, study 36, `docs/science/longwave-absorbers.md`),
//! which replaced the grey stand-in ported from `legacy-final`
//! (`methane_depth`, `CH4_OPACITY` and `CH4_SATURATION_BAR`, two
//! calibrations on Titan).

use super::column::stratospheric_methane_bar;
use super::gases::PartialPressures;
use crate::units::{Fraction, Kelvin};

/// Definition: the most of the starlight a body absorbs that its
/// stratosphere can send back to space ([`stratospheric_share_of`]): one
/// half. A thin layer that absorbs all the light and is transparent in the
/// thermal infrared radiates what it absorbs half up to space and half down
/// to the surface (the textbook layer argument), so that the surface keeps
/// at least half of it: the largest anti-greenhouse, which cools an
/// effective temperature by the factor `2^-¼` (0.84). The absorber is
/// methane in its near-infrared bands and the organic haze its photolysis
/// makes, which McKay, Pollack & Courtin 1991 (*Science* 253, 1118,
/// doi:10.1126/science.11538492) find does most of Titan's: one
/// stratosphere for both here, its absorption measured by its methane. The
/// half is the layer's limit, not a fit. One value for every star, though
/// redder stars put more of their light in methane's bands.
const CH4_NEAR_IR_MAX: f64 = 0.5;

/// Calibration: methane above the tropopause, bar, at which the
/// stratosphere takes half of [`CH4_NEAR_IR_MAX`]: the column past which
/// its near-infrared bands and its haze are opaque and only their wings
/// widen. Fitted on McKay, Pollack & Courtin 1991 (*Science* 253, 1118,
/// doi:10.1126/science.11538492; the audit, Wiki study 19, checked the
/// abstract): the anti-greenhouse cools Titan's 94 K surface by 9 K, the
/// greenhouse warming it by 21 K, so that under the same greenhouse its
/// surface keeps `(94 / 103)^4` of the light it absorbs, a share of 0.306
/// sent back to space. The sourced Titan of `test_points.rs` (1.467 bar,
/// 5.65 % of methane, effective temperature 82 K, so a skin of 68.95 K)
/// lets 2.03e-3 bar of methane past its cold trap, with the Setzmann &
/// Wagner sublimation line of PR 2b-1; `h = m (0.5 / 0.306 - 1)` with that
/// `m` gives 1.29e-3, rounded to 0.0013 (the share is then 0.306 within
/// 1 %). Their split of that light, 1.5 W/m2 absorbed in the stratosphere
/// of 2.6 W/m2 (as Robinson & Catling 2012, ApJ 757, 104, arXiv:1209.1833,
/// section 4.3, take it from them; the audit did not read section 4.3),
/// half of which goes up, gives 0.29, within 15 %. Why it moved: the
/// legacy 0.002 was anchored on 0.0031 bar let through, which came from
/// the legacy sublimation line and a generated Titan; refitted on
/// 2026-11-23 by the user's decision. What was searched and not found: a
/// measured methane column above Titan's tropopause with its share of the
/// light. Under it, the share still grows with the tropopause's
/// temperature; a world of 26 bar of methane under a warm tropopause takes
/// nearly all of [`CH4_NEAR_IR_MAX`].
const CH4_NEAR_IR_HALF_BAR: f64 = 0.0013;

/// Share of the starlight a body absorbs that its stratosphere, the
/// methane above its tropopause and the haze it makes, takes first and
/// sends back to space: `m / (m + h)` of [`CH4_NEAR_IR_MAX`], `m` the
/// methane above the tropopause of a stratosphere at `skin_k`
/// ([`stratospheric_methane_bar`]) and `h` [`CH4_NEAR_IR_HALF_BAR`]. None
/// without methane.
pub(crate) fn stratospheric_share_of(gases: &PartialPressures, skin_k: f64) -> f64 {
    let m = stratospheric_methane_bar(gases, skin_k);
    if m <= 0.0 {
        return 0.0;
    }
    CH4_NEAR_IR_MAX * m / (m + CH4_NEAR_IR_HALF_BAR)
}

/// Share of the starlight a body absorbs that the methane of its
/// stratosphere sends back to space (the methane anti-greenhouse, McKay,
/// Pollack & Courtin 1991), for a stratosphere at the skin temperature
/// `skin`. See [`stratospheric_share_of`].
pub fn stratospheric_share(gases: &PartialPressures, skin: Kelvin) -> Fraction {
    Fraction::new(stratospheric_share_of(gases, skin.0).clamp(0.0, CH4_NEAR_IR_MAX)).unwrap_or_else(
        || {
            // A NaN skin reads as no share.
            Fraction::new(0.0).expect("zero is a fraction")
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::radiation::column::skin_temperature;
    use crate::radiation::test_points::{TITAN_EFFECTIVE_K, TITAN_SURFACE_AIR};

    /// Definition: the skin temperature, K, of the legacy share test's Titan
    /// (a stratosphere near Titan's 69 K).
    const LEGACY_SKIN_K: f64 = 70.0;

    /// Methane above the tropopause takes a share of the starlight that
    /// grows with the methane let through the cold trap and saturates
    /// (legacy `stratospheric_methane_takes_a_share_of_the_light`, without
    /// its `absorbed_w_m2` lines, which belong to the climate of 2d, and
    /// its 0.2 to 0.4 window on the legacy Titan, which pinned the legacy's
    /// own calibration).
    #[test]
    fn stratospheric_methane_takes_a_share_of_the_light() {
        let titan = PartialPressures {
            n2: 1.4,
            ch4: 0.08,
            ..PartialPressures::default()
        };
        let dry = PartialPressures { ch4: 0.0, ..titan };
        let skin = LEGACY_SKIN_K;
        assert_eq!(stratospheric_share_of(&dry, skin), 0.0);
        let share = stratospheric_share_of(&titan, skin);
        assert!(share > 0.0, "{share}");
        // A warmer tropopause lets more methane through: more of the light.
        assert!(stratospheric_share_of(&titan, skin + 3.0) > share);
        // Twenty-six bar of methane under a warm tropopause: saturated.
        let rich = PartialPressures { ch4: 26.0, ..titan };
        let most = stratospheric_share_of(&rich, 96.0);
        assert!(most > share && most <= CH4_NEAR_IR_MAX, "{most}");
        assert_eq!(stratospheric_share(&dry, Kelvin(skin)).get(), 0.0);
    }

    /// Titan's anti-greenhouse cools its 94 K surface by 9 K (McKay,
    /// Pollack & Courtin 1991, *Science* 253, 1118,
    /// doi:10.1126/science.11538492): under the same greenhouse, the light
    /// its surface keeps is `(94 / 103)⁴` of what it would keep without it,
    /// a share of 0.306 sent back to space. The same paper's split of the
    /// light Titan absorbs, 1.5 W/m² in its stratosphere of 2.6 W/m² (as
    /// reproduced by Robinson & Catling 2012, ApJ 757, 104, §4.3), half of
    /// which the stratosphere radiates up, gives 0.29. The sourced Titan's
    /// stratosphere takes it within 10 % and 15 %, and a column of methane
    /// past saturation never more than half of the light. The first
    /// assertion checks the calibration of `CH4_NEAR_IR_HALF_BAR`, which
    /// was fitted on 0.306: it is a net if the physics moves, not an
    /// independent check. The 1.5 / 2.6 split is the independent one.
    #[test]
    fn the_methane_anti_greenhouse_matches_mckay() {
        let skin = skin_temperature(Kelvin(TITAN_EFFECTIVE_K));
        let share = stratospheric_share(&TITAN_SURFACE_AIR, skin).get();
        let mckay = 1.0 - (94.0_f64 / (94.0 + 9.0)).powi(4);
        assert!((share / mckay - 1.0).abs() < 0.1, "{share} against {mckay}");
        let split = 0.5 * 1.5 / 2.6;
        assert!(
            (share / split - 1.0).abs() < 0.15,
            "{share} against {split}"
        );
        let rich = PartialPressures {
            ch4: 26.0,
            ..TITAN_SURFACE_AIR
        };
        assert!(stratospheric_share(&rich, Kelvin(100.0)).get() <= 0.5);
    }
}
