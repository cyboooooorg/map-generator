//! Heat a planet makes or loses inside: radioactive decay of its silicates
//! ([`radiogenic`]), the cooling of its mantle and core ([`secular`]) and
//! the tides a host raises in it on an eccentric orbit ([`tidal`]).
//!
//! Ported from the tag `legacy-final` (`planet/heat.rs`) with the values
//! re-sourced in the study
//! `docs/superpowers/studies/2026-11-21-planet-interior/heat-and-composition.md`
//! section A.

use crate::constants::GRAVITATIONAL_CONSTANT;
use crate::units::{Gyr, Kilogram, Metre, Watt};
use std::f64::consts::LN_2;

/// Source: mass of Earth's bulk silicate part, 4.03e24 kg (Dye 2012,
/// Treatise on Geophysics 2nd ed., vol. 13, Table 7, "Primitive Mantle",
/// M_SE). Not the PREM crust plus mantle (4.30e24 kg).
pub const EARTH_SILICATE_MASS: Kilogram = Kilogram(4.03e24);

/// Source: Earth's age, 4.5673 Gyr, the age of the oldest solids of the
/// solar system (CAI, 4567.30 +/- 0.16 Myr; Connelly et al. 2012, Science
/// 338, 651). Only the abstract was read. The date of the solar system, used
/// as the age of Earth.
pub const EARTH_AGE: Gyr = Gyr(4.5673);

/// Source: U-238 half-life, 4.468 Gyr (Dye 2012, Table 2, ln 2 / lambda with
/// lambda 4.916e-18 /s).
pub const HALF_LIFE_U238: Gyr = Gyr(4.468);
/// Source: U-235 half-life, 0.704 Gyr (Dye 2012, Table 2, lambda 31.210e-18
/// /s).
pub const HALF_LIFE_U235: Gyr = Gyr(0.704);
/// Source: Th-232 half-life, 14.05 Gyr (Dye 2012, Table 2, lambda 1.563e-18
/// /s).
pub const HALF_LIFE_TH232: Gyr = Gyr(14.05);
/// Source: K-40 half-life, 1.277 Gyr (Dye 2012, Table 2, lambda 17.200e-18
/// /s). The commonly quoted 1.248 Gyr is not in a source read.
pub const HALF_LIFE_K40: Gyr = Gyr(1.277);

/// Source: heat per kilogram of element, W/kg, uranium (Dye 2012, Table 3:
/// 98.5 uW/kg).
const HEAT_PER_KG_U: f64 = 98.5e-6;
/// Source: heat per kilogram of element, W/kg, thorium (Dye 2012, Table 3:
/// 26.3 uW/kg).
const HEAT_PER_KG_TH: f64 = 26.3e-6;
/// Source: heat per kilogram of element, W/kg, potassium (Dye 2012, Table 3:
/// 3.33e-3 uW/kg).
const HEAT_PER_KG_K: f64 = 3.33e-9;

/// Source: share of uranium's heat made by U-238, 0.958 (derived from Dye
/// 2012, Table 2: 95.13 uW/kg of isotope and its abundance; the study reads
/// it as 95.8 %). The rest is U-235.
const U238_SHARE_OF_U: f64 = 0.958;

/// Calibration: uranium of Earth's silicate part, 20 ng/g. Chosen inside
/// the published models (Dye 2012, Table 7: cosmochemical 12-22 ng/g,
/// geophysical 20-35 ng/g). Searched: a single abundance set that all
/// sources agree on; none found, the models span 11-38 TW. See
/// `docs/science/planet-interiors.md#the-heat-inside`.
const URANIUM_ABUNDANCE: f64 = 20e-9;
/// Calibration: thorium of Earth's silicate part, 80 ng/g, inside Dye 2012,
/// Table 7 (cosmochemical 45-83, geophysical 77-135 ng/g). Searched as
/// [`URANIUM_ABUNDANCE`]; see
/// `docs/science/planet-interiors.md#the-heat-inside`.
const THORIUM_ABUNDANCE: f64 = 80e-9;
/// Source: potassium of Earth's silicate part, 280 ug/g (Arevalo, McDonough
/// & Luong 2009, EPSL 278, 361: 280 +/- 120 ug/g; abstract only, via the
/// study), inside Dye 2012, Table 7 (140-420 ug/g). It is the sensitive
/// share: the heat follows it linearly.
const POTASSIUM_ABUNDANCE: f64 = 280e-6;

/// Definition: each heat-producing isotope with its present power per
/// kilogram of Earth's silicate part, W/kg, and its half-life: the
/// abundance of the element times the heat per kilogram of element (times
/// the U-238/U-235 split). Over [`EARTH_SILICATE_MASS`] it sums to about
/// 20 TW ([`earth_radiogenic_power`]).
const ISOTOPES: [(f64, Gyr); 4] = [
    (
        URANIUM_ABUNDANCE * HEAT_PER_KG_U * U238_SHARE_OF_U,
        HALF_LIFE_U238,
    ),
    (
        URANIUM_ABUNDANCE * HEAT_PER_KG_U * (1.0 - U238_SHARE_OF_U),
        HALF_LIFE_U235,
    ),
    (THORIUM_ABUNDANCE * HEAT_PER_KG_TH, HALF_LIFE_TH232),
    (POTASSIUM_ABUNDANCE * HEAT_PER_KG_K, HALF_LIFE_K40),
];

/// Definition: Earth's present radiogenic power, the sum of [`ISOTOPES`]
/// over [`EARTH_SILICATE_MASS`] (about 20 TW, inside the 11-38 TW of Dye
/// 2012, Table 7).
pub fn earth_radiogenic_power() -> Watt {
    Watt(ISOTOPES.iter().map(|(p, _)| p).sum::<f64>() * EARTH_SILICATE_MASS.0)
}

/// Source: Earth's total surface heat flow, 46.7 TW (Davies & Davies 2010,
/// Solid Earth 1, 5; 47 +/- 2 TW).
pub const EARTH_TOTAL_HEAT_FLOW: Watt = Watt(46.7e12);

/// Calibration: e-folding time of Earth's secular cooling, 10 Gyr.
/// Searched: a sourced cooling time or law of cooling against mass; none
/// found (Arevalo 2009 gives 70-130 K/Gyr, an e-fold of order 10-20 Gyr on
/// a 1600-1700 K mantle, derived; Herzberg 2010 and Plesa 2018 not read).
/// See `docs/science/planet-interiors.md#the-heat-inside`.
pub const SECULAR_COOLING_TIME: Gyr = Gyr(10.0);

/// Calibration: exponent of the cooling time against silicate mass,
/// 2/3 (a time that grows as the area `R^2` of the body, `R^3 ~ M`).
/// Searched: a sourced scaling of secular cooling with mass (Valencia 2007
/// calls it difficult; Van Heck & Tackley 2011 and arXiv:2110.03295 not
/// opened); none found. See
/// `docs/science/planet-interiors.md#the-heat-inside`.
const COOLING_MASS_EXPONENT: f64 = 2.0 / 3.0;

/// Radiogenic power of a body: each isotope's present power per kilogram
/// scaled by the silicate mass and the `abundance` at formation (Earth =
/// 1), carried from Earth's age to `age` by its decay,
/// `sum_i p_i m a exp(lambda_i (t_earth - t))`, `lambda_i = ln 2 / t_half`.
pub fn radiogenic(silicate: Kilogram, abundance: f64, age: Gyr) -> Watt {
    let power: f64 = ISOTOPES
        .iter()
        .map(|&(p, half_life)| p * (LN_2 / half_life.0 * (EARTH_AGE.0 - age.0)).exp())
        .sum();
    Watt(power * silicate.0 * abundance)
}

/// Secular cooling of a body: the heat its interior loses as it cools,
/// beyond what it makes. Earth's (46.7 TW less its radiogenic power today)
/// scaled by the silicate mass and decaying with age over a cooling time
/// that grows with mass: `S m exp(t_earth / tau_earth - t / tau)`,
/// `tau = tau_earth (m / m_earth)^(2/3)`. Zero for no silicate.
pub fn secular(silicate: Kilogram, age: Gyr) -> Watt {
    if silicate.0 <= 0.0 {
        return Watt(0.0);
    }
    let earth_secular = EARTH_TOTAL_HEAT_FLOW.0 - earth_radiogenic_power().0;
    let mass = silicate.0 / EARTH_SILICATE_MASS.0;
    let tau = SECULAR_COOLING_TIME.0 * mass.powf(COOLING_MASS_EXPONENT);
    Watt(earth_secular * mass * (EARTH_AGE.0 / SECULAR_COOLING_TIME.0 - age.0 / tau).exp())
}

/// Source: coefficient 21/2 of the tidal heating of a synchronous
/// satellite on an eccentric orbit (Peale, Cassen & Reynolds 1979,
/// Science 203, 892).
const TIDAL_COEFFICIENT: f64 = 10.5;

/// Tidal heating of a satellite of `radius` on an orbit of `semi_major_axis`
/// and `eccentricity` around a `host`, with a tidal response `k2_over_q`:
/// `E = (21/2)(k2/Q) G M_h^2 R^5 n e^2 / a^6`, `n = sqrt(G M_h / a^3)`
/// (Peale, Cassen & Reynolds 1979), the satellite's own mass neglected.
pub fn tidal(
    host: Kilogram,
    radius: Metre,
    semi_major_axis: Metre,
    eccentricity: f64,
    k2_over_q: f64,
) -> Watt {
    let g = GRAVITATIONAL_CONSTANT;
    let n = (g * host.0 / semi_major_axis.0.powi(3)).sqrt();
    Watt(
        TIDAL_COEFFICIENT
            * k2_over_q
            * g
            * host.0
            * host.0
            * radius.0.powi(5)
            * n
            * eccentricity
            * eccentricity
            / semi_major_axis.0.powi(6),
    )
}

/// Kind of body, for its tidal response. The thresholds on composition that
/// choose the kind belong to the system engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TidalKind {
    /// A rocky body (silicates and metal).
    Rocky,
    /// An icy body, an ice shell over an ocean.
    Icy,
    /// A body wrapped in hydrogen and helium.
    Enveloped,
}

/// Calibration: `k2/Q` of a rocky body, 6e-3 (k2 0.3, Q 50 at one Earth
/// mass; Henning, O'Connell & Sasselov 2009, ApJ 707, 1000, p. 5, a model
/// baseline, not a measurement). Io's measured 0.015 +/- 0.003 (Lainey et
/// al. 2009; Lainey 2016, Table 1) is partly molten, Earth 1.08e-3, Mars
/// 1.74e-3 (derived, Lainey 2016, Table 1). See
/// `docs/science/planet-interiors.md#the-heat-inside`.
pub const ROCKY_K2_OVER_Q: f64 = 6e-3;

/// Calibration: `k2/Q` of an icy body, 2.5e-3 as in legacy. Searched: a
/// measured icy-moon value; none found (Wahr 2006 and Howett 2011, the
/// legacy citations, were not reached; Enceladus' assumed 9e-5, Meyer &
/// Wisdom 2007, and Earth, Moon and Mars at 6e-4 to 1.7e-3, Lainey 2016,
/// Table 1, bracket it). See
/// `docs/science/planet-interiors.md#the-heat-inside`.
pub const ICY_K2_OVER_Q: f64 = 2.5e-3;

/// Source: `k2/Q` of a body wrapped in hydrogen and helium, Jupiter's
/// (1.102 +/- 0.203)e-5 (Lainey et al. 2009, Nature 459, 957; Lainey et al.
/// 2016, Celest. Mech. Dyn. Astron. 126, 145, Table 1). Saturn's is
/// (1.59 +/- 0.74)e-4, same table.
pub const ENVELOPED_K2_OVER_Q: f64 = 1.102e-5;

/// `k2/Q` of a kind of body.
pub fn k2_over_q(kind: TidalKind) -> f64 {
    match kind {
        TidalKind::Rocky => ROCKY_K2_OVER_Q,
        TidalKind::Icy => ICY_K2_OVER_Q,
        TidalKind::Enveloped => ENVELOPED_K2_OVER_Q,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Definition: watts in a terawatt.
    const TW: f64 = 1e12;

    fn isotope_term(index: usize, age: f64) -> f64 {
        let (p, half_life) = ISOTOPES[index];
        p * (LN_2 / half_life.0 * (EARTH_AGE.0 - age)).exp()
    }

    #[test]
    fn each_isotope_halves_over_one_half_life() {
        // Definition: half-life, the time over which a decaying amount halves.
        for (i, (_, half_life)) in ISOTOPES.iter().enumerate() {
            let now = isotope_term(i, 1.0);
            let later = isotope_term(i, 1.0 + half_life.0);
            assert!((later / now - 0.5).abs() < 1e-12, "isotope {i}");
        }
    }

    #[test]
    fn earth_today_is_inside_the_published_range() {
        let radiogenic_today = radiogenic(EARTH_SILICATE_MASS, 1.0, EARTH_AGE).0 / TW;
        // Dye 2012, Table 7: 11-38 TW.
        assert!(
            (11.0..=38.0).contains(&radiogenic_today),
            "{radiogenic_today}"
        );
        let total = radiogenic_today + secular(EARTH_SILICATE_MASS, EARTH_AGE).0 / TW;
        // Davies & Davies 2010: 46.7 +/- 2 TW.
        assert!((total - 46.7).abs() <= 2.0, "{total}");
    }

    #[test]
    fn io_is_heated_about_as_observed() {
        /// Source: Jupiter's mass, 1898.13e24 kg (NASA Jupiter Fact Sheet,
        /// nssdc.gsfc.nasa.gov/planetary/factsheet/jupiterfact.html).
        const JUPITER: Kilogram = Kilogram(1898.13e24);
        /// Source: Io's radius, half of the 3643 km diameter of the NASA
        /// Galilean satellite fact sheet (nssdc.gsfc.nasa.gov/planetary/
        /// factsheet/galileanfact_table.html). The sheet gives its mass as
        /// 8.93e22 kg (unused: the satellite's mass is neglected).
        const IO_RADIUS: Metre = Metre(1821.5e3);
        /// Source: Io's orbital distance, 422e3 km (same fact sheet).
        const IO_ORBIT: Metre = Metre(422e6);
        /// Source: Io's eccentricity, 0.004 (same fact sheet).
        const IO_ECCENTRICITY: f64 = 0.004;
        /// Source: Io's `k2/Q`, 0.015 (Lainey et al. 2009, Nature 459, 957;
        /// Lainey 2016, Table 1).
        const IO_K2_OVER_Q: f64 = 0.015;
        let heat = tidal(JUPITER, IO_RADIUS, IO_ORBIT, IO_ECCENTRICITY, IO_K2_OVER_Q).0;
        /// Source: Io's observed heat flow, 0.83e14 to 1.7e14 W: from
        /// 8.33e13 W (Galileo PPR, Rathbun et al. 2004, Icarus 169, 127,
        /// as tabulated in an energy-budget table seen through a search
        /// result) to 1.3 +/- 0.4e14 W (Veeder et al. 2004, Icarus 169,
        /// 264, abstract). The fact sheet's eccentricity is rounded to one
        /// figure, which alone moves the heat by several per cent.
        const OBSERVED: std::ops::RangeInclusive<f64> = 0.83e14..=1.7e14;
        assert!(OBSERVED.contains(&heat), "{heat}");
    }

    #[test]
    fn heat_scales_with_silicate_mass() {
        let one = radiogenic(Kilogram(1e24), 1.0, Gyr(3.0)).0;
        let two = radiogenic(Kilogram(2e24), 1.0, Gyr(3.0)).0;
        assert!((two / one - 2.0).abs() < 1e-12);
        assert!(secular(Kilogram(1e24), Gyr(1.0)).0 > 0.0);
        assert!(secular(Kilogram(0.0), Gyr(1.0)).0 == 0.0);
    }

    #[test]
    fn k2_over_q_by_kind() {
        assert_eq!(k2_over_q(TidalKind::Rocky), ROCKY_K2_OVER_Q);
        assert!(k2_over_q(TidalKind::Enveloped) < k2_over_q(TidalKind::Icy));
    }
}
