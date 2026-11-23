//! Methane's grey stand-in for Titan's collision-induced greenhouse
//! (ported from `legacy-final`'s `planet/climate.rs`, `methane_depth`):
//! one grey optical depth that the spectrum spreads over the far infrared
//! (`spectrum::METHANE_TOP_CM`). A calibration, not a law: spectral CIA
//! tables of N₂–N₂ and N₂–CH₄ exist in HITRAN and would replace it; they
//! were not extracted (`docs/science/radiative-transfer.md`, departures).

use super::gases::PartialPressures;

/// Calibration: grey opacity of methane at 1 bar in 1 bar of air, the
/// square root of its amount times the air's (see [`depth`]); the
/// spectral column takes it over 0-650 cm⁻¹ only (`spectrum`). Fitted in
/// the legacy on Titan's surface, 93.65 ± 0.25 K at 1467 hPa (Fulchignoni
/// et al. 2005, *Nature* 438, 785, doi:10.1038/nature04314), under the
/// anti-greenhouse anchored on McKay, Pollack & Courtin 1991 (legacy
/// issue #563): the legacy `titan-like` archetype reached 92.4 to 94.5 K
/// (mean 93.69 K) and 5.5 to 5.9 % of methane (mean 5.71 %, against the
/// 5.65 % near Titan's surface, Niemann et al. 2010, *J. Geophys. Res.
/// Planets* 115, E12006, doi:10.1029/2010JE003659) over seeds 1 to 40,
/// on the spectral column. Titan's real greenhouse is mostly
/// collision-induced (N₂–N₂, N₂–CH₄, N₂–H₂) and warms it by 21 K (McKay,
/// Pollack & Courtin 1991, *Science* 253, 1118,
/// doi:10.1126/science.11538492, abstract read); this opacity carries all
/// of it, the nitrogen's own included, the haze's 9 K of cooling being
/// the anti-greenhouse's. That is why it keeps the square root of the
/// air's pressure: collision-induced absorption grows with the nitrogen
/// faster than line broadening does, not slower.
///
/// What was searched and not found: a grey or band opacity of Titan's
/// CIA greenhouse usable as one number; the sourced route is HITRAN's
/// N₂–N₂ and N₂–CH₄ CIA files, binned like the CO₂ pairs (future work).
/// It applies to the methane that radiates, the methane cold-trapped at
/// the tropopause left out (the column), and saturates past
/// [`CH4_SATURATION_BAR`].
///
/// It was fitted with the legacy methane sublimation line (9.1 kJ/mol from
/// 90.7 K and 0.117 bar) and was not refitted to the port's
/// (`crate::volatiles`, 9.7 kJ/mol from the equation of state's triple
/// point). At the 68.95 K skin of the tests' Titan column (82 K
/// equilibrium), methane's saturation falls from 2.60e-3 to 2.03e-3 bar
/// (-22 %), and that column's outgoing radiation rises from 1.829 to
/// 1.865 W/m² (+2.0 %; `column` test
/// `a_column_without_co2_or_water_radiates_from_its_methane_or_ground`
/// run with each curve).
const CH4_OPACITY: f64 = 13.4;

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
