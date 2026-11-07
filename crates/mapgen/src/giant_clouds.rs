//! The cloud decks of a giant planet, and the cloud class they make
//! (`docs/science/gas-giant-bands.md`).
//!
//! **Decks.** A giant of the Sun's composition stacks, from the top,
//! ammonia ice, ammonium hydrosulphide (NH₄SH) and water ice or droplets
//! (Atreya & Wong 2005: for Jupiter the bases are at 0.72, near 2 and
//! 5.7 bar). Each deck forms where the partial pressure of its gas reaches
//! saturation on the temperature profile of the troposphere, so the decks
//! sit deeper on a colder giant and a warm one loses them from the top
//! down. The profile is a dry adiabat from the 1-bar temperature, itself a
//! multiple of the effective temperature: [`temperature_k`].
//!
//! **Classes.** A deck that still reaches the tropopause is a top deck. The
//! ammonia deck on top makes a Sudarsky class I giant, the water deck on top
//! (ammonia gone) class II, no deck class III (Sudarsky, Burrows & Pinto
//! 2000). The bounds are the effective temperatures at which the decks
//! reach the tropopause: about 205 K for ammonia, 350 K for water, the
//! latter as Sudarsky et al. give it ("too hot for significant H₂O
//! condensation", ≳ 350 K) and the former between their 150 K (class I)
//! and 250 K (class II). Class IV and V come from the temperature and the
//! gravity ([`Clouds::of`]).
//!
//! **Looks.** A top deck that only just reaches the tropopause is thin, and
//! the deck below shows through: [`Clouds::thin`] blends the palette of the
//! class with that of the next one, so that no bound is a step. The height
//! between the zones (the top deck) and the belts (the deck beneath, seen
//! where the top one is thin) is the scale height times the log of the
//! ratio of their base pressures: [`Clouds::relief`].
use std::sync::OnceLock;

/// A cloud deck.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Deck {
    Ammonia,
    Hydrosulphide,
    Water,
}

/// The decks, from the top.
const DECKS: [Deck; 3] = [Deck::Ammonia, Deck::Hydrosulphide, Deck::Water];

/// Dry adiabatic exponent `R / cp` of a diatomic gas (hydrogen), `2/7`: the
/// temperature goes as `P^(2/7)`. It puts Jupiter's water base at 5.6 bar
/// and its ammonia base at 0.6 bar from the 1-bar temperature, as Atreya &
/// Wong (2005) have them (5.7 and 0.72 bar).
const KAPPA: f64 = 2.0 / 7.0;
/// Temperature at 1 bar over the effective temperature: Jupiter's 165 K
/// (NASA Space Science Data Coordinated Archive, Jupiter fact sheet) over
/// its effective temperature of 124.7 K, from the 13.7 W/m² it radiates
/// (absorbed 6.25 plus internal 7.485 W/m²; Li et al. 2018,
/// doi:10.1038/s41467-018-06107-2).
///
/// The profile puts the tropopause of Jupiter at 81 K for 0.085 bar (the real
/// one is 110 K near 0.1 bar): the tropopause pressure is a knob that hides
/// the cold top of a dry adiabat, not a measured level.
const ONE_BAR_OVER_TEFF: f64 = 1.32;
/// Pressure of the tropopause, where the decks end, bar: calibration, no
/// source (Jupiter's is near 0.1 bar). It is set so that the water deck
/// leaves the troposphere at an effective temperature of 350 K, as Sudarsky
/// et al. (2000) have it.
const TROPOPAUSE_BAR: f64 = 0.085;
/// Deepest base looked for, bar.
const DEEPEST_BAR: f64 = 1000.0;
/// Atmospheric pressure, bar per atm.
const BAR_PER_ATM: f64 = 1.013_25;
/// Particles per hydrogen atom in a hydrogen–helium gas: half an H₂ plus
/// helium at He/H = 0.0851 (`log ε` = 10.93 on the scale where H is 12,
/// Asplund et al. 2009, doi:10.1146/annurev.astro.46.060407.145222).
const MOLECULES_PER_H: f64 = 0.5 + 0.0851;
/// Solar abundances `log ε` of nitrogen, oxygen and sulphur (Asplund et al.
/// 2009, the same table).
const LOG_N: f64 = 7.83;
const LOG_O: f64 = 8.69;
const LOG_S: f64 = 7.12;

/// Mixing ratio (particles of the element per particle of gas) of an
/// element of abundance `log_eps`, solar composition.
fn mixing_ratio(log_eps: f64) -> f64 {
    10f64.powf(log_eps - 12.0) / MOLECULES_PER_H
}

/// Temperature at pressure `p_bar`, K, on the profile of a giant of
/// effective temperature `teff`.
pub fn temperature_k(teff: f64, p_bar: f64) -> f64 {
    ONE_BAR_OVER_TEFF * teff * p_bar.powf(KAPPA)
}

/// `ln` of how far the gas of `deck` is past saturation at `p_bar` and
/// `t` K: above 0 it condenses.
///
/// Water: the triple point of water (273.16 K, 611.657 Pa, IAPWS) and the
/// Clausius–Clapeyron slope of its latent heats, 2.834 and 2.501 MJ/kg for
/// ice and liquid over the gas constant of vapour, 461.5 J/(kg K).
/// Ammonia: its triple point (195.4 K, 6.06 kPa, CRC Handbook), the slope
/// of 2 973 K that joins it to the normal boiling point (239.8 K at 1 atm)
/// and 681 K more for the heat of fusion (5.66 kJ/mol) on the solid.
/// Ammonium hydrosulphide: `log10 Kp = 14.82 − 4705 / T` with `Kp` in atm²
/// for `NH₃ + H₂S ⇌ NH₄SH` as Lewis (1969,
/// doi:10.1016/0019-1035(69)90091-8) gives it, as quoted by Li & Ingersoll
/// (2018, arXiv:1801.05529) and checked by the base it gives Jupiter (not
/// read from Lewis's paper). It is fitted to 180-300 K and extended in a
/// straight line outside, which a colder giant needs; only the relief
/// between decks uses it (the classes come from ammonia and water).
fn excess(deck: Deck, p_bar: f64, t: f64) -> f64 {
    match deck {
        Deck::Water => {
            let slope = if t < 273.16 { 6140.0 } else { 5420.0 };
            let saturation = 611.657 * (slope * (1.0 / 273.16 - 1.0 / t)).exp();
            (mixing_ratio(LOG_O) * p_bar * 1e5 / saturation).ln()
        }
        Deck::Ammonia => {
            let slope = if t < 195.4 { 3654.0 } else { 2973.0 };
            let saturation = 6060.0 * (slope * (1.0 / 195.4 - 1.0 / t)).exp();
            (mixing_ratio(LOG_N) * p_bar * 1e5 / saturation).ln()
        }
        Deck::Hydrosulphide => {
            let p_atm = p_bar / BAR_PER_ATM;
            let product = mixing_ratio(LOG_N) * mixing_ratio(LOG_S) * p_atm * p_atm;
            product.ln() - std::f64::consts::LN_10 * (14.82 - 4705.0 / t)
        }
    }
}

/// Pressure of the base of `deck` on a giant of effective temperature
/// `teff` K, bar; `None` when the deck does not reach the tropopause (the
/// giant is too warm for it).
pub fn base_bar(deck: Deck, teff: f64) -> Option<f64> {
    let at = |p: f64| excess(deck, p, temperature_k(teff, p));
    if at(TROPOPAUSE_BAR) <= 0.0 {
        return None;
    }
    if at(DEEPEST_BAR) >= 0.0 {
        return Some(DEEPEST_BAR);
    }
    // The excess falls with pressure: bisect in ln P.
    let (mut lo, mut hi) = (TROPOPAUSE_BAR.ln(), DEEPEST_BAR.ln());
    for _ in 0..40 {
        let mid = 0.5 * (lo + hi);
        if at(mid.exp()) > 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Some((0.5 * (lo + hi)).exp())
}

/// Effective temperature above which the silicate clouds of a giant of
/// the gravity below [`LOW_GRAVITY`] sit high, K: the lower end of class
/// IV (Sudarsky et al. 2000 reduce the 1 500 K of class V for low gravity
/// but give no number; calibration, no source).
const LOW_GRAVITY_CLASS_V_K: f32 = 900.0;
/// Surface gravity under which a roaster is class V at lower temperatures,
/// m/s² (Sudarsky et al. 2000: 10³ cm/s²).
pub const LOW_GRAVITY: f32 = 10.0;

/// The look of an ice giant's clouds with methane in the gas: it absorbs the
/// red wherever it stays methane, which is all the way up to class III
/// (Atreya & Wong 2005 on the colour of Uranus and Neptune; Sudarsky et al.
/// 2000: methane features "prevalent" in classes I and II). The share
/// of red kept is a calibration, no source.
pub fn methane_tint(colour: [f32; 3]) -> [f32; 3] {
    [colour[0] * METHANE_RED, colour[1] * 0.97, colour[2]]
}
/// Share of the red an ice giant's methane lets through in classes II
/// and III: calibration, no source.
const METHANE_RED: f32 = 0.85;

/// The clouds of a giant.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Clouds {
    /// Cloud class of Sudarsky, Burrows & Pinto (2000), 1 to 5.
    pub class: u8,
    /// The class whose look shows through as the clouds thin.
    pub next: u8,
    /// The deck on top, when there is one (classes I and II).
    pub top: Option<Deck>,
    /// How thin the top deck is, `[0, 1]`: 0 for a thick one, 1 for one that
    /// just leaves the troposphere. The look of the next class shows
    /// through by that share.
    pub thin: f32,
    /// Height between the zones and the belts relative to Jupiter's: the
    /// scale height times the log ratio of the base pressures of the top
    /// deck and the one below it.
    pub relief: f32,
}

/// Reference relief: Jupiter's, from its effective temperature and gravity.
fn jupiter_relief() -> f32 {
    static JUPITER: OnceLock<f32> = OnceLock::new();
    *JUPITER.get_or_init(|| relief_of(124.7, 24.8, Some(Deck::Ammonia)))
}

/// Height between the top deck and the one below it, m.
fn relief_of(teff: f64, gravity: f64, top: Option<Deck>) -> f32 {
    let Some(top) = top else {
        return 0.0;
    };
    let p_top = base_bar(top, teff).unwrap_or(TROPOPAUSE_BAR);
    let below = DECKS
        .iter()
        .skip_while(|&&d| d != top)
        .skip(1)
        .find_map(|&d| base_bar(d, teff));
    // The last deck modelled has nothing beneath: one e-fold of pressure.
    let ratio = below.map_or(std::f64::consts::E, |p| p / p_top);
    // Scale height R T / (μ g), μ = 2.22 g/mol.
    let height = 8.314 * temperature_k(teff, p_top) / (2.22e-3 * gravity.max(0.1));
    let relief = (height * ratio.max(1.0).ln()) as f32;
    if relief.is_finite() { relief } else { 0.0 }
}

impl Clouds {
    /// The clouds of a giant of effective temperature `teff` K and surface
    /// gravity `gravity` m/s².
    ///
    /// Class V from 1 500 K, or from 900 K in a giant of low gravity
    /// ([`LOW_GRAVITY`]); class IV from 900 K; below, the decks decide.
    pub fn of(teff: f32, gravity: f32) -> Self {
        let t = f64::from(teff);
        if teff >= 1500.0 || (gravity < LOW_GRAVITY && teff >= LOW_GRAVITY_CLASS_V_K) {
            return Self::flat(5, 5, 0.0);
        }
        // The flat classes blend into the next over the last 100 K, so that
        // no bound is a step in colour (calibration, no source).
        let ramp = |from: f32| smooth(f64::from((teff - from) / 100.0)) as f32;
        if teff >= 900.0 {
            return Self::flat(4, 5, ramp(1400.0));
        }
        let ammonia = base_bar(Deck::Ammonia, t);
        let water = base_bar(Deck::Water, t);
        let (class, top, base) = match (ammonia, water) {
            (Some(p), _) => (1, Some(Deck::Ammonia), p),
            (None, Some(p)) => (2, Some(Deck::Water), p),
            (None, None) => {
                let next = if gravity < LOW_GRAVITY { 5 } else { 4 };
                return Self::flat(3, next, ramp(800.0));
            }
        };
        // Thick once the base lies 4 times deeper than the tropopause.
        let depth = (base / TROPOPAUSE_BAR).ln() / 4f64.ln();
        let thin = 1.0 - smooth(depth);
        let relief = relief_of(t, f64::from(gravity), top) / jupiter_relief();
        Clouds {
            class,
            next: class + 1,
            top,
            thin: thin as f32,
            relief: if relief.is_finite() {
                relief.clamp(0.3, 2.0)
            } else {
                1.0
            },
        }
    }

    /// A class with no volatile deck on top: haze and silicate clouds only
    /// show a faint relief (calibration, no source).
    fn flat(class: u8, next: u8, thin: f32) -> Self {
        Clouds {
            class,
            next,
            top: None,
            thin,
            relief: 0.5,
        }
    }
}

/// Smoothstep of `x` from 0 to 1.
fn smooth(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jupiters_decks_sit_where_atreya_and_wong_put_them() {
        // Teff of Jupiter from Li et al. (2018): 124.7 K.
        let near = |deck, lo, hi| {
            let p = base_bar(deck, 124.7).unwrap();
            assert!((lo..hi).contains(&p), "{deck:?} {p}");
        };
        near(Deck::Ammonia, 0.5, 0.9);
        near(Deck::Hydrosulphide, 1.6, 2.8);
        near(Deck::Water, 4.5, 7.0);
    }

    #[test]
    fn a_colder_giant_has_deeper_decks() {
        // Saturn is colder than Jupiter: NH4SH near 5 bar, water 12.6 bar
        // (solar oxygen) in Atreya & Wong; the dry profile puts them
        // deeper than Jupiter's, within a factor of two of those.
        let (j, s) = (95.0, 124.7);
        for deck in DECKS {
            assert!(base_bar(deck, j).unwrap() > base_bar(deck, s).unwrap());
        }
        let water = base_bar(Deck::Water, 95.0).unwrap();
        assert!((9.0..25.0).contains(&water), "{water}");
    }

    #[test]
    fn the_decks_leave_the_troposphere_from_the_top_down() {
        // Ammonia first, between Sudarsky's 150 and 250 K; water at 350 K.
        let lost = |deck| {
            (50..800)
                .find(|&t| base_bar(deck, f64::from(t)).is_none())
                .unwrap()
        };
        let (ammonia, water) = (lost(Deck::Ammonia), lost(Deck::Water));
        assert!((190..220).contains(&ammonia), "{ammonia}");
        assert!((340..360).contains(&water), "{water}");
    }

    #[test]
    fn the_classes_follow_the_decks_and_blend_across_their_bounds() {
        let class = |t: f32| Clouds::of(t, 25.0);
        assert_eq!(class(125.0).class, 1);
        assert_eq!(class(250.0).class, 2);
        assert_eq!(class(600.0).class, 3);
        assert_eq!(class(1200.0).class, 4);
        assert_eq!(class(1700.0).class, 5);
        // A giant at 300 K, white before, is thinning towards clear.
        assert_eq!(class(300.0).class, 2);
        assert!(class(300.0).thin > class(250.0).thin);
        // Just under the bound, the water deck is thin; the ammonia one is
        // thick on Jupiter.
        assert!(class(345.0).thin > 0.9);
        assert!(class(125.0).thin < 0.1);
        // Jupiter sets the relief.
        assert!((class(124.7).relief - 1.0).abs() < 0.05);
    }

    #[test]
    fn the_hot_classes_blend_before_their_bounds_too() {
        assert!(Clouds::of(899.0, 25.0).thin > 0.95);
        assert_eq!(Clouds::of(899.0, 25.0).next, 4);
        assert!(Clouds::of(1499.0, 25.0).thin > 0.95);
        assert_eq!(Clouds::of(1499.0, 25.0).next, 5);
        // A low-gravity giant goes straight to the pale class V.
        assert_eq!(Clouds::of(899.0, 8.0).next, 5);
        assert_eq!(Clouds::of(700.0, 25.0).thin, 0.0);
        // No NaN from a giant with no gravity.
        assert!(Clouds::of(150.0, 0.0).relief.is_finite());
    }

    #[test]
    fn a_low_gravity_roaster_is_class_v_at_lower_temperatures() {
        assert_eq!(Clouds::of(1200.0, 25.0).class, 4);
        assert_eq!(Clouds::of(1200.0, 8.0).class, 5);
        // The low gravity does not touch cold giants.
        assert_eq!(Clouds::of(120.0, 8.0).class, 1);
    }
}
