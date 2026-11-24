//! Methane's grey stand-in for Titan's collision-induced greenhouse
//! (ported from `legacy-final`'s `planet/climate.rs`, `methane_depth`):
//! one grey optical depth that the spectrum spreads over the far infrared
//! (`spectrum::METHANE_TOP_CM`). A calibration, not a law: spectral CIA
//! tables of N₂–N₂ and N₂–CH₄ exist in HITRAN and would replace it; they
//! were not extracted (`docs/science/radiative-transfer.md`, departures).

use super::column::stratospheric_methane_bar;
use super::gases::PartialPressures;
use crate::units::{Fraction, Kelvin};

/// Calibration: grey opacity of methane at 1 bar in 1 bar of air, the
/// square root of its amount times the air's (see [`depth`]); the
/// spectral column takes it over 0-650 cm⁻¹ only (`spectrum`). Fitted on
/// Titan's surface, 93.65 ± 0.25 K at 1467 hPa (Fulchignoni et al. 2005,
/// *Nature* 438, 785, doi:10.1038/nature04314), on the sourced column of
/// `radiation::test_points`: 1.467 bar, 5.65 % of methane (Niemann et al.
/// 2010, *J. Geophys. Res. Planets* 115, E12006,
/// doi:10.1029/2010JE003659), the light of an 82 K effective temperature
/// (McKay, Pollack & Courtin 1991) on Titan's gravity, no internal heat,
/// a black surface, the port's methane sublimation line
/// (`crate::volatiles`, 9.7 kJ/mol from the equation of state's triple
/// point) and the anti-greenhouse of [`CH4_NEAR_IR_HALF_BAR`]
/// (`crate::balance::surface_temperature`). Method (issue #646, study 25,
/// `docs/science/planet-atmospheres.md`, "Calibration of `CH4_OPACITY`"):
/// the constant was bisected, the column solved at each step, until it
/// gave 93.65 K to three figures of the opacity: 13.4 gave 91.24 K, 16
/// gave 94.83 K, 14.7 93.06 K, 15.35 93.95 K, 15.0 93.47 K, 15.2
/// 93.74 K, 15.1 93.60 K (15.13 93.645 K, 15.14 93.658 K). The tier-1
/// test `titans_column_sits_at_its_measured_surface` holds the column at
/// 93.65 ± 0.25 K. The same 94 K is McKay, Pollack & Courtin 1991's
/// budget (*Science* 253, 1118, doi:10.1126/science.11538492): an 82 K
/// effective temperature, +21 K of greenhouse, −9 K of anti-greenhouse.
///
/// The old value, 13.4, was fitted in the legacy with the legacy methane
/// sublimation line (9.1 kJ/mol from 90.7 K and 0.117 bar), on the
/// legacy `titan-like` archetype over seeds 1 to 40 (92.4 to 94.5 K);
/// the port's line lets 22 % less methane past the cold trap at the
/// column's 68.95 K skin, so the column cooled to 91.24 K. Titan's real
/// greenhouse is mostly collision-induced (N₂–N₂, N₂–CH₄, N₂–H₂); this
/// opacity carries all of it, the nitrogen's own included, the haze's
/// 9 K of cooling being the anti-greenhouse's. That is why it keeps the
/// square root of the air's pressure: collision-induced absorption grows
/// with the nitrogen faster than line broadening does, not slower.
///
/// What was searched and not found: a grey or band opacity of Titan's
/// CIA greenhouse usable as one number; the sourced route is HITRAN's
/// N₂–N₂ and N₂–CH₄ CIA files, binned like the CO₂ pairs (future work).
/// It applies to the methane that radiates, the methane cold-trapped at
/// the tropopause left out (the column), and saturates past
/// [`CH4_SATURATION_BAR`].
const CH4_OPACITY: f64 = 15.1;

/// Calibration: methane, bar, past which its greenhouse saturates
/// ([`depth`]): the amount `u` its opacity counts grows as
/// `p / (1 + p / s)`, `p` the methane, so that it never exceeds this.
/// Past it the law is not extrapolated: the grey opacity stands for
/// collision-induced bands whose centres are already opaque at Titan's
/// amounts, so that more methane only widens their wings, as the
/// logarithmic forcing of a saturated band does (Pierrehumbert 2010,
/// *Principles of Planetary Climate*, §4.4, the claim not read there); a
/// body at 90 to 120 K radiates at 25 to 60 µm, where methane has no
/// strong band of its own.
///
/// Fitted in the legacy, together with [`CH4_OPACITY`], on three
/// constraints at once: Titan's 93.7 K; Titan's warming from 1 to 10 bar
/// of methane under a third of its warming from 0.01 to 0.1 bar; and the
/// legacy seed 182's planet d (an equilibrium temperature of 114 K under
/// 4.4 bar of N₂ and 26 bar of methane) under 150 K. Without it, the
/// `sqrt(p_CH₄ P)` law stretched 500-fold past Titan put that planet at
/// 351 K (legacy issue #443). What was searched and not found: no data on
/// a methane-saturating CIA greenhouse beyond Titan (audit, Wiki study
/// 19).
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
pub(crate) fn depth(gases: &PartialPressures) -> (f64, f64) {
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

    /// Definition: a Titan-like air, bar: N₂ and CH₄.
    const TITAN_AIR: [f64; 2] = [1.45, 0.07];

    /// Definition: the relative step of `ln p` of the finite difference.
    const STEP: f64 = 1e-6;

    /// Definition: the tolerance of a central difference of step
    /// [`STEP`] against the analytic derivative.
    const DERIVATIVE_TOLERANCE: f64 = 1e-8;

    fn titan_scaled(factor: f64) -> PartialPressures {
        PartialPressures {
            n2: TITAN_AIR[0] * factor,
            ch4: TITAN_AIR[1] * factor,
            ..PartialPressures::default()
        }
    }

    #[test]
    fn the_rate_is_the_depths_derivative_in_ln_p() {
        // In a uniform air every partial pressure grows as `p`: the rate
        // is `d τ / d ln p`, a central difference of the depth.
        let (_, rate) = depth(&titan_scaled(1.0));
        let up = depth(&titan_scaled(STEP.exp())).0;
        let down = depth(&titan_scaled((-STEP).exp())).0;
        let numeric = (up - down) / (2.0 * STEP);
        assert!(
            (rate / numeric - 1.0).abs() < DERIVATIVE_TOLERANCE,
            "rate {rate} against {numeric}"
        );
    }

    #[test]
    fn no_methane_no_depth() {
        let air = PartialPressures {
            n2: 1.0,
            ch4: -1e-12,
            ..PartialPressures::default()
        };
        assert_eq!(depth(&air), (0.0, 0.0));
    }

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
