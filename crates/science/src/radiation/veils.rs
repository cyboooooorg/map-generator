//! The sulphuric acid deck and the methane haze: the veils that hide a
//! body's surface from starlight (ported from `legacy-final`'s
//! `planet/albedo.rs`, `sulphuric_deck` and `haze`, with their
//! constants). Their consumer is the shortwave budget of PR 2b-2.

use super::gases::PartialPressures;
use crate::units::{Bar, Fraction, Kelvin};

/// Source: the Bond albedo of Venus's sulphuric acid deck, 0.76 (Haus et
/// al. 2016, *Icarus* 272, 178-205, doi:10.1016/j.icarus.2016.02.048,
/// value confirmed by the audit, Wiki study 19; the legacy's DOI was
/// wrong; Pioneer Venus gave 0.80 +- 0.02, NASA's fact sheet 0.77). Its
/// only body: the deck hides all below it.
pub(crate) const DECK_ALBEDO: f64 = 0.76;

/// Calibration: least SO₂, bar, that makes a sulphuric deck. Venus holds
/// 1.4e-2 bar near its surface (150 ppm of 92 bar), ten times more. What
/// was searched and not found: an observed threshold; the sourced route
/// is the H₂SO₄-H₂O condensation (Zeleznik 1991, Dai et al. 2022), not
/// done (`docs/science/radiative-transfer.md`, "The cloud's water, the
/// deck and the haze").
const DECK_SO2_BAR: f64 = 1.0e-3;

/// Calibration: least surface temperature, K, at which a deck forms, the
/// melting point of ice. What was searched and not found: a physical
/// onset for an acid cloud from the surface temperature (Venus's cloud
/// base sits near 350 K; the sourced route is the H₂SO₄-H₂O saturation
/// curve, as for [`DECK_SO2_BAR`]).
const DECK_MIN_K: f64 = 273.15;

/// Calibration: warming, K, over which a deck grows from none at
/// [`DECK_MIN_K`] to whole. A numerical width, not physics, so no data
/// exists for it: a deck that appeared whole at one temperature would
/// leave the albedo's fixed point no balance between a warm clear sky and
/// a cold decked one; spread over 40 K the deck moves the temperature it
/// derives by less than the step it takes, so that the damped fixed point
/// converges. Venus's 737 K is far past it.
const DECK_ONSET_K: f64 = 40.0;

/// Calibration: methane, bar, at which a haze reaches `1 - 1/e` of its
/// strength. Titan's 1.5 to 5.65 % of 1.467 bar (0.02 to 0.08 bar; Niemann
/// et al. 2010, doi:10.1029/2010JE003659) make a thick haze, an Earth's
/// trace none. What was searched and not found: the haze's optical depth
/// as a function of the methane in any source (Tomasko et al. 2008 would
/// give the aerosol model by altitude, not opened).
const HAZE_CH4_BAR: f64 = 0.01;

/// Source: the ratio of methane to CO₂ at which the organic haze ends,
/// 0.2 (Trainer et al. 2006, *PNAS* 103, 18035,
/// doi:10.1073/pnas.0608561103: "too much CO₂ will ultimately terminate
/// aerosol formation, as seen at a C/O of 0.6", and C/O = (1 + r) / 2
/// gives r = 0.2; the audit's reading). Below it the haze grows in
/// proportion to the ratio.
const HAZE_CH4_PER_CO2: f64 = 0.2;

/// Calibration: the share of the light falling on a thick methane haze
/// that it reflects, 0.25 ([`HAZE_ABSORPTION`] is the share it absorbs),
/// fitted on Titan's Bond albedo, 0.265 (Li et al. 2011, *GRL* 38,
/// L23201, doi:10.1029/2011GL050053, the audit's corrected DOI and value,
/// not 0.27): the orange haze absorbs most of the blue and hides the
/// bright ice below. What was searched and not found: the split between
/// the two in any source (Li 2011, Creecy et al. 2021, Tomasko et al.
/// 2008); it is a fit choice.
pub(crate) const HAZE_ALBEDO: f64 = 0.25;

/// Calibration: the share of the light falling on a thick methane haze
/// that it absorbs, 0.55 (see [`HAZE_ALBEDO`]).
pub(crate) const HAZE_ABSORPTION: f64 = 0.55;

/// Share of the disc under a sulphuric acid deck, 0 to 1, over a surface
/// at `t_k` whose air holds `so2_bar` of SO₂ after the washing out. None
/// under [`DECK_SO2_BAR`] or [`DECK_MIN_K`]; past them it grows linearly
/// over [`DECK_ONSET_K`].
pub(crate) fn deck_share(t_k: f64, so2_bar: f64) -> f64 {
    if so2_bar < DECK_SO2_BAR {
        return 0.0;
    }
    ((t_k - DECK_MIN_K) / DECK_ONSET_K).clamp(0.0, 1.0)
}

/// Share of the disc under a sulphuric acid deck over a surface at
/// `surface` whose air holds `so2` of SO₂. See [`deck_share`].
pub fn sulphuric_deck(surface: Kelvin, so2: Bar) -> Fraction {
    Fraction::new(deck_share(surface.0, so2.0)).unwrap_or_else(|| {
        // A NaN temperature reads as no deck.
        Fraction::new(0.0).expect("zero is a fraction")
    })
}

/// Strength of a methane haze in `gases`, 0 to 1 ([`HAZE_CH4_BAR`],
/// [`HAZE_CH4_PER_CO2`]).
pub(crate) fn haze(gases: &PartialPressures) -> f64 {
    let ch4 = gases.ch4.max(0.0);
    if ch4 <= 0.0 || ch4.is_nan() {
        return 0.0;
    }
    let ratio = if gases.co2 > 0.0 {
        (ch4 / (HAZE_CH4_PER_CO2 * gases.co2)).min(1.0)
    } else {
        1.0
    };
    (1.0 - (-ch4 / HAZE_CH4_BAR).exp()) * ratio
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::radiation::test_points::{
        EARTH_AIR, EARTH_METHANE_MIXING_RATIO, TITAN_SURFACE_AIR, TITAN_SURFACE_K, VENUS_SO2_BAR,
        VENUS_SURFACE_K,
    };

    #[test]
    fn venus_is_under_its_deck() {
        let share = sulphuric_deck(Kelvin(VENUS_SURFACE_K), Bar(VENUS_SO2_BAR));
        assert_eq!(share.get(), 1.0);
        // Titan's cold surface under Venus's sulphur has none.
        assert_eq!(
            sulphuric_deck(Kelvin(TITAN_SURFACE_K), Bar(VENUS_SO2_BAR)).get(),
            0.0
        );
    }

    #[test]
    fn a_cold_or_sulphur_poor_air_has_no_deck() {
        assert_eq!(deck_share(272.0, VENUS_SO2_BAR), 0.0);
        assert_eq!(deck_share(VENUS_SURFACE_K, DECK_SO2_BAR * 0.99), 0.0);
        assert_eq!(deck_share(VENUS_SURFACE_K, 0.0), 0.0);
        // It grows over its onset.
        let half = deck_share(DECK_MIN_K + 0.5 * DECK_ONSET_K, DECK_SO2_BAR);
        assert!((half - 0.5).abs() < 1e-12, "{half}");
    }

    #[test]
    fn titan_is_hazy_earth_is_not() {
        let titan = haze(&TITAN_SURFACE_AIR);
        assert!(titan > 0.9, "{titan}");
        let earth = PartialPressures {
            ch4: EARTH_METHANE_MIXING_RATIO * EARTH_AIR.total(),
            ..EARTH_AIR
        };
        let trace = haze(&earth);
        assert!(trace < 0.001, "{trace}");
        assert_eq!(haze(&EARTH_AIR), 0.0);
    }

    /// Definition: the methane, bar, of the haze tests' air (a thick haze:
    /// five times [`HAZE_CH4_BAR`]).
    const TEST_CH4_BAR: f64 = 0.05;

    #[test]
    fn co2_cuts_the_haze() {
        let ch4 = TEST_CH4_BAR;
        let bare = haze(&PartialPressures {
            ch4,
            n2: 1.0,
            ..PartialPressures::default()
        });
        let with = |ratio: f64| {
            haze(&PartialPressures {
                ch4,
                n2: 1.0,
                co2: ch4 / ratio,
                ..PartialPressures::default()
            })
        };
        // At and above the cut-off ratio the haze is whole.
        assert!((with(HAZE_CH4_PER_CO2) - bare).abs() < 1e-12);
        assert!((with(1.0) - bare).abs() < 1e-12);
        // At half of it, half of the haze.
        let half = with(0.5 * HAZE_CH4_PER_CO2);
        assert!((half / bare - 0.5).abs() < 1e-12, "{half} {bare}");
    }
}
