//! Heat a planet makes or loses inside: radioactive decay of its silicates
//! ([`radiogenic`]), the cooling of its mantle and core ([`secular`]) and
//! the tides a host raises in it on an eccentric orbit ([`tidal`]).
//!
//! Ported from the tag `legacy-final` (`planet/heat.rs`) with the values
//! re-sourced in the study
//! `docs/superpowers/studies/2026-11-21-planet-interior/heat-and-composition.md`
//! section A.

use crate::constants::{
    AVOGADRO, DAY, ELECTRON_MASS_ENERGY_KEV, GRAVITATIONAL_CONSTANT, JOULES_PER_KEV,
    JULIAN_YEAR_DAYS, YEARS_PER_GYR,
};
use crate::units::{Gyr, Kilogram, Metre, Mole, Watt};
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
/// Source: K-40 half-life, 1.2522(27) Gyr (DDEP evaluation of 2025, X.
/// Mougeot, ICRM 2025 poster, HAL cea-05087440, read), the newest
/// evaluation, which the argon of the air shares
/// (`docs/science/planet-atmospheres.md`, "Argon"). Every evaluation of
/// Schaen et al. 2020 (GSA Bulletin 133, 461, Table 1) gives 1.2505 to
/// 1.2688 Gyr; ENSDF 2017's 1.2480 Gyr is the "commonly quoted" one. Dye
/// 2012's 1.277 Gyr (Table 2, lambda 17.200e-18 /s), taken before, was
/// 0.6 % (Min et al. 2000's 1.2688) to 2.3 % (ENSDF 2017's) longer than
/// each of them, 2.0 % longer than DDEP 2025's.
pub const HALF_LIFE_K40: Gyr = Gyr(1.2522);

/// Source: heat per kilogram of element, W/kg, uranium (Dye 2012, Table 3:
/// 98.5 uW/kg).
const HEAT_PER_KG_U: f64 = 98.5e-6;
/// Source: heat per kilogram of element, W/kg, thorium (Dye 2012, Table 3:
/// 26.3 uW/kg).
const HEAT_PER_KG_TH: f64 = 26.3e-6;
/// Source: the branches of K-40's decay, each as its share of the decays
/// and the energy it leaves in the rock, keV (DDEP evaluation of 2025,
/// LNE-LNHB table of K-40, X. Mougeot, read on lnhb.fr; to be cited as
/// Mougeot et al. 2026, Metrologia 63, 019001,
/// doi:10.1088/1681-7575/ae3733): beta- to Ca-40, 89.56 %, its electron's
/// mean energy 582.898 keV (the antineutrino carries the rest out of the
/// planet); electron capture to Ar-40's 1460 keV level, 10.34 %, the
/// level's gamma transition, 1460.851 keV (the neutrino's 43.55 keV
/// leaves); beta+, 0.00103 %, the positron's mean 251.757 keV and its
/// annihilation, twice [`ELECTRON_MASS_ENERGY_KEV`]. The capture to
/// Ar-40's ground state (0.098 %) leaves only argon's atomic relaxation,
/// as every capture does: at most its K shell's 3.2 keV (the K-beta
/// X-ray's 3.1905 keV) on 10.44 % of the decays, under 0.05 % of the
/// heat, left out.
const K40_DECAY_HEAT_KEV: [(f64, f64); 3] = [
    (0.8956, 582.898),
    (0.1034, 1460.851),
    (1.03e-5, 251.757 + 2.0 * ELECTRON_MASS_ENERGY_KEV),
];

/// Definition: heat per kilogram of element, W/kg, potassium: its K-40
/// atoms ([`K40_ATOM_SHARE`] times [`AVOGADRO`] over
/// [`POTASSIUM_MOLAR_MASS`], 1.7972e21 per kg, as DDEP prints 1.7972e18
/// per g), times the decay constant `ln 2 / HALF_LIFE_K40` over a Julian
/// year (DDEP prints 17.541e-18 /s), times the heat of a decay,
/// [`K40_DECAY_HEAT_KEV`] (673.1 keV): 3.400e-9 W/kg. Dye 2012's 3.33e-3
/// uW/kg (Table 3), taken until the half-life moved to DDEP's (#694), is
/// the same product on his decay constant, 17.200e-18 /s, 2 % slower
/// (test `potassiums_heat_is_dyes_on_ddeps_decay_constant`).
const HEAT_PER_KG_K: f64 = K40_ATOM_SHARE * AVOGADRO / POTASSIUM_MOLAR_MASS * LN_2
    / (HALF_LIFE_K40.0 * YEARS_PER_GYR * JULIAN_YEAR_DAYS * DAY.0)
    * (K40_DECAY_HEAT_KEV[0].0 * K40_DECAY_HEAT_KEV[0].1
        + K40_DECAY_HEAT_KEV[1].0 * K40_DECAY_HEAT_KEV[1].1
        + K40_DECAY_HEAT_KEV[2].0 * K40_DECAY_HEAT_KEV[2].1)
    * JOULES_PER_KEV;

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

/// Source: share of potassium's atoms that are K-40, 0.011668 %
/// (Naumenko et al. 2013, as the DDEP evaluation of 2025 takes it: X.
/// Mougeot, ICRM 2025 poster, HAL cea-05087440, read).
pub const K40_ATOM_SHARE: f64 = 1.1668e-4;
/// Source: standard atomic weight of potassium, 39.0983(1), in kg/mol
/// (CIAAW, the potassium page of ciaaw.org, read for study 29; the IUPAC
/// value of Prohaska et al. 2022, Pure Appl. Chem. 94, 573,
/// doi:10.1515/pac-2019-0603, which it publishes).
pub const POTASSIUM_MOLAR_MASS: f64 = 39.0983e-3;

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

/// K-40 a body's silicate part has lost to decay since it formed, mol:
/// the potassium of the heat (`POTASSIUM_ABUNDANCE` of Earth's silicate
/// part, times the `abundance` at formation, Earth = 1, as [`radiogenic`]
/// takes it) times [`K40_ATOM_SHARE`] over [`POTASSIUM_MOLAR_MASS`], over
/// the `silicate` mass, carried to formation by its decay and decayed
/// over `age`: `c_K x40 / m_K M a e^(lambda t_earth) (1 - e^(-lambda t))`,
/// `lambda = ln 2 / HALF_LIFE_K40`. The heat's convention: a body forms
/// with the K-40 Earth had at its formation. None at a negative age.
pub fn potassium_40_decayed(silicate: Kilogram, abundance: f64, age: Gyr) -> Mole {
    let lambda = LN_2 / HALF_LIFE_K40.0;
    let at_formation = POTASSIUM_ABUNDANCE * K40_ATOM_SHARE / POTASSIUM_MOLAR_MASS
        * silicate.0
        * abundance
        * (lambda * EARTH_AGE.0).exp();
    Mole(at_formation * -(-lambda * age.0.max(0.0)).exp_m1())
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
    fn potassium_40_decays_with_ddeps_half_life() {
        /// Source: K-40 half-life, 1.2522(27) Gyr (DDEP evaluation of
        /// 2025, X. Mougeot, ICRM 2025 poster, HAL cea-05087440), the
        /// newest evaluation; Schaen et al. 2020 (GSA Bulletin 133, 461,
        /// Table 1) give 1.2505 to 1.2688 Gyr for the others.
        const DDEP_2025: f64 = 1.2522;
        // Potassium's heat at formation over today's is `2^(t_earth /
        // t_half)`, the inverse of the share of K-40 left today.
        let ratio = isotope_term(3, 0.0) / isotope_term(3, EARTH_AGE.0);
        let expected = 2f64.powf(EARTH_AGE.0 / DDEP_2025);
        assert!((ratio / expected - 1.0).abs() < 1e-12, "{ratio} {expected}");
    }

    #[test]
    fn potassiums_heat_is_dyes_on_ddeps_decay_constant() {
        /// Source: heat of potassium, 3.33e-3 uW/kg of element (Dye 2012,
        /// Treatise on Geophysics 2nd ed., vol. 13, Table 3), in W/kg.
        const DYE_HEAT: f64 = 3.33e-9;
        /// Source: K-40's decay constant in Dye 2012, Table 2, 17.200e-18
        /// /s.
        const DYE_LAMBDA: f64 = 17.200e-18;
        /// Source: K-40's decay constant in the DDEP evaluation of 2025,
        /// 17.541(38)e-18 /s (LNE-LNHB table of K-40, X. Mougeot, read).
        const DDEP_LAMBDA: f64 = 17.541e-18;
        /// Definition: the tolerance, twice the rounding of Dye's three
        /// printed digits (0.15 %): his heat per decay may differ from
        /// DDEP 2025's by no more than that for the two to agree.
        const DYE_DIGITS: f64 = 3e-3;
        // Dye's heat carried to DDEP's decay constant: 3.396e-9 W/kg.
        let expected = DYE_HEAT * DDEP_LAMBDA / DYE_LAMBDA;
        assert!(
            (HEAT_PER_KG_K / expected - 1.0).abs() < DYE_DIGITS,
            "{HEAT_PER_KG_K:e} against {expected:e}"
        );

        /// Source: the beta- activity of natural potassium, 28.23(6) per
        /// second and gram (DDEP evaluation of 2025, X. Mougeot, ICRM 2025
        /// poster, HAL cea-05087440, read: the recommended value of 44
        /// measurements), per kilogram.
        const DDEP_BETA_ACTIVITY: f64 = 28.23e3;
        /// Source: its uncertainty, 0.06 per second and gram, as a share.
        const DDEP_BETA_SPREAD: f64 = 0.06 / 28.23;
        // The decays the heat counts, through the beta- branch.
        let beta = K40_ATOM_SHARE * AVOGADRO / POTASSIUM_MOLAR_MASS * LN_2
            / (HALF_LIFE_K40.0 * YEARS_PER_GYR * JULIAN_YEAR_DAYS * DAY.0)
            * K40_DECAY_HEAT_KEV[0].0;
        assert!(
            (beta / DDEP_BETA_ACTIVITY - 1.0).abs() < DDEP_BETA_SPREAD,
            "{beta}"
        );
    }

    #[test]
    fn earth_today_is_inside_the_published_range() {
        let radiogenic_today = radiogenic(EARTH_SILICATE_MASS, 1.0, EARTH_AGE).0 / TW;
        // Dye 2012, Table 7: 11-38 TW.
        assert!(
            (11.0..=38.0).contains(&radiogenic_today),
            "{radiogenic_today}"
        );
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
        /// Source: Io's mean semi-major axis, 421 800 km (JPL planetary
        /// satellite mean elements, ssd.jpl.nasa.gov/sats/elem/, epoch
        /// 2000-01-01.5 TDB; the Wikipedia infobox for Io gives the same).
        const IO_ORBIT: Metre = Metre(421.8e6);
        /// Source: Io's eccentricity, 0.004 (JPL planetary satellite mean
        /// elements, ssd.jpl.nasa.gov/sats/elem/, the same table as the
        /// semi-major axis; printed to one figure).
        const IO_ECCENTRICITY: f64 = 0.004;
        /// Source: Io's `k2/Q`, 0.015 +/- 0.003 (Lainey et al. 2009, Nature
        /// 459, 957; Lainey 2016, Table 1), as its lower and upper ends.
        const IO_K2_OVER_Q: [f64; 2] = [0.012, 0.018];
        let [low, high] =
            IO_K2_OVER_Q.map(|k| tidal(JUPITER, IO_RADIUS, IO_ORBIT, IO_ECCENTRICITY, k).0);
        /// Source: Io's observed heat flow, 1.3 +/- 0.4e14 W, so 0.9e14 to
        /// 1.7e14 W (Veeder et al. 2004, Icarus 169, 264, abstract as read
        /// through a search result).
        const OBSERVED: std::ops::RangeInclusive<f64> = 0.9e14..=1.7e14;
        // The band of the model over Lainey's uncertainty overlaps the
        // observed range: the verdict does not hang on a third digit of e.
        assert!(
            low <= *OBSERVED.end() && high >= *OBSERVED.start(),
            "{low} to {high}"
        );
    }

    #[test]
    fn heat_scales_with_silicate_mass() {
        let one = radiogenic(Kilogram(1e24), 1.0, Gyr(3.0)).0;
        let two = radiogenic(Kilogram(2e24), 1.0, Gyr(3.0)).0;
        assert!((two / one - 2.0).abs() < 1e-12);
        assert!(secular(Kilogram(1e24), Gyr(1.0)).0 > 0.0);
        assert!(secular(Kilogram(0.0), Gyr(1.0)).0 == 0.0);
    }
}
