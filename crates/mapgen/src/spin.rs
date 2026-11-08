//! How a star's tides set a planet's spin
//! (`docs/science/tidal-locking.md`).
//!
//! **Despun.** The star's tides brake a planet's spin; [`locking_gyr`]
//! gives the time they take for an Earth-like planet, [`giant_locking_gyr`]
//! for a giant. A planet whose time is shorter than the star's age
//! ([`AGE_GYR`], or the star's whole life when shorter) is despun.
//!
//! **Locked or resonant.** A despun rocky planet on a circular orbit ends
//! locked with one face to its star, one turn per orbit. On an eccentric
//! one it may be caught on the way down in a spin-orbit resonance, Mercury's
//! 3:2 (three turns every two orbits) or the 2:1: the odds
//! of capture rise with the eccentricity ([`capture`], Makarov 2012; Ribas
//! et al. 2016), and the world's seed draws the outcome. The tides
//! circularise the orbit far more slowly than they despin the planet
//! ([`circularisation_gyr`]); a planet whose orbit they have circularised
//! ends locked. Both times scale with the planet's mass and radius
//! ([`rocky_locking_gyr`], [`rocky_circularisation_gyr`]). Giants are
//! fluid, with no lasting bulge for the star to hold: a despun giant on a
//! round orbit is locked, on an eccentric one it settles at the
//! pseudo-synchronous rate ([`pseudo_synchronous_turns`]).
//!
//! [`locking_gyr`]: crate::star::locking_gyr
//! [`AGE_GYR`]: crate::star::AGE_GYR

use crate::noise::EARTH_CIRCUMFERENCE_KM;
use crate::star::{AGE_GYR, StarInfo};

/// A spin-orbit resonance a despun planet may be captured in, and the
/// eccentricities between which its capture goes from impossible to
/// certain.
struct Resonance {
    /// Turns per orbit.
    turns: f32,
    /// Eccentricity below which the planet crosses it freely.
    free_below: f32,
    /// Eccentricity from which a planet crossing it is always caught.
    certain_from: f32,
}

/// The resonances a spin crosses on its way down, highest first (the first
/// that catches the planet holds it; one that catches none leaves it
/// locked at 1:1).
///
/// 3:2: caught from 0.06 (Ribas et al. 2016, §4.6,
/// doi:10.1051/0004-6361/201629576: "at eccentricities lower than 0.06,
/// the most probable state becomes the synchronous one") and for certain
/// from 0.20, "guaranteed to be captured at any eccentricity between 0.20
/// and 0.41" (Makarov 2012, §3.2, doi:10.1088/0004-637X/752/1/73, whose
/// Mercury is "always captured in the 3:2 resonance" at e = 0.2056).
/// 2:1: certain from 0.34 ("entrapment in the 2:1 resonance is inevitable
/// for 0.34 < e < 0.49", Makarov 2012, §3.2) and 0.23 likely at Mercury's
/// 0.2056 (the 40 integrations of §3.1: 9 captures; the semi-analytic
/// curve of his fig. 8 gives about 0.3), which a linear ramp reaches from
/// 0.161 (Ribas et al. find the 2:1 stable for Proxima b from about
/// 0.16). The ramps are linear between their ends, a calibration through
/// these points. Makarov's values are for a Mercury-like triaxiality; a
/// more symmetric planet (larger, "keeping all other parameters the same")
/// is caught at lower eccentricities (his minimal certain ones drop to 0.08
/// for the 3:2 and 0.20 for the 2:1), so these are the cautious side. The
/// 2:1 stays certain above 0.49, where the paper's decline of the odds is
/// not modelled. The 5:2 and higher resonances are not modelled: Makarov
/// gives no threshold for them that applies to this planet.
const RESONANCES: [Resonance; 2] = [
    Resonance {
        turns: 2.0,
        free_below: 0.161,
        certain_from: 0.34,
    },
    Resonance {
        turns: 1.5,
        free_below: 0.06,
        certain_from: 0.20,
    },
];

/// Turns per orbit of the spin-orbit state named `name` in a preset's
/// `[star] resonance`: `1:1` (locked, 1), `3:2` (1.5) or `2:1` (2).
pub fn parse_resonance(name: &str) -> Option<f32> {
    match name {
        "1:1" => Some(1.0),
        "3:2" => Some(RESONANCE_TURNS_PER_ORBIT),
        "2:1" => Some(2.0),
        _ => None,
    }
}

/// A number in `[0, 1)` from `seed` and the `salt` of one draw
/// (SplitMix64, Steele, Lea & Flood 2014, doi:10.1145/2714064.2660195).
fn unit_draw(seed: u32, salt: u64) -> f32 {
    let mut z = (u64::from(seed) << 8 | salt).wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 40) as f32 / (1u64 << 24) as f32
}

/// The turns per orbit of a despun rocky planet on an orbit of
/// `eccentricity`, drawn from `seed`: the resonance (3:2 as 1.5, 2:1 as 2) that caught it on its way down, or 0 when none did and it
/// ends locked, one turn per orbit. At each resonance (see
/// [`RESONANCES`]) the odds of capture rise linearly from 0 to 1 with the
/// eccentricity; each has its own draw from the seed.
pub fn capture(eccentricity: f32, seed: u32) -> f32 {
    for (k, r) in RESONANCES.iter().enumerate() {
        let odds =
            ((eccentricity - r.free_below) / (r.certain_from - r.free_below)).clamp(0.0, 1.0);
        if unit_draw(seed, k as u64) < odds {
            return r.turns;
        }
    }
    0.0
}

/// Turns per orbit of a planet in the 3:2 resonance: its sidereal day is
/// two thirds of its year, its solar day two years (Mercury: 58.65 and
/// 175.9 days for an 87.97-day year, NASA Mercury fact sheet).
pub const RESONANCE_TURNS_PER_ORBIT: f32 = 1.5;

/// Modified tidal quality factor of a rocky planet, `Q' = 3Q / (2k₂)`
/// (Goldreich & Soter 1966, doi:10.1016/0019-1035(66)90051-0), from the
/// `Q = 100` and `k₂ = 0.3` of [`crate::star::locking_gyr`]: 500.
const ROCKY_Q_PRIME: f64 = 3.0 * 100.0 / (2.0 * 0.3);

/// Time for the star's tides to circularise the orbit of an Earth-like
/// planet, Gyr: `τ_e = (4/63) Q' (m / M) (a / R)⁵ / n`, the planet's tide
/// in the eccentricity damping of Goldreich & Soter 1966, as written by
/// Jackson, Greenberg & Barnes 2008, eq. 2 (doi:10.1086/529187), with
/// `m` and `R` Earth's, `M` the star's mass, `a` the semi-major axis and
/// `n` the mean motion. About 10⁸ Gyr × a^6.5 / M^1.5 (a in AU, M in solar
/// masses), some 10⁵ to 10⁶ times the locking time: only the planets
/// within about 0.05 AU of a red dwarf or 0.07 AU of the Sun are
/// circularised in 4.6 Gyr. This is Earth's mass and radius, as in
/// [`crate::star::locking_gyr`]; [`rocky_circularisation_gyr`] scales it.
pub fn circularisation_gyr(orbit_au: f32, star_mass: f32) -> f32 {
    const G: f64 = 6.674e-11;
    const SUN_KG: f64 = 1.989e30;
    const EARTH_KG: f64 = 5.972e24;
    const EARTH_M: f64 = 6.371e6;
    const AU_M: f64 = 1.496e11;
    const GYR_S: f64 = 3.156e16;
    let a = f64::from(orbit_au.max(1e-4)) * AU_M;
    let m_star = f64::from(star_mass) * SUN_KG;
    let n = (G * m_star / a.powi(3)).sqrt();
    let tau = 4.0 / 63.0 * ROCKY_Q_PRIME * (EARTH_KG / m_star) * (a / EARTH_M).powi(5) / n;
    (tau / GYR_S) as f32
}

/// [`circularisation_gyr`] for a rocky planet of `mass_earth` and
/// `radius_earth` (Earth units): the time goes as `m / R⁵` (the planet's
/// mass in the formula, its radius to the fifth), so a super-Earth of 5.5
/// Earth masses (1.76 radii) circularises about three times faster.
pub fn rocky_circularisation_gyr(
    orbit_au: f32,
    star_mass: f32,
    mass_earth: f32,
    radius_earth: f32,
) -> f32 {
    circularisation_gyr(orbit_au, star_mass) * mass_earth / radius_earth.max(1e-3).powi(5)
}

/// [`crate::star::locking_gyr`] for a rocky planet of `mass_earth` and
/// `radius_earth` (Earth units): the despinning time of Gladman et al.
/// 1996 (`t = ω a⁶ I Q / (3 G M² k₂ R⁵)`, doi:10.1006/icar.1996.0117) with
/// `I = α m R²` goes as `m / R³`, Earth's `Q` and `k₂` kept (a calibration:
/// no source gives them for other masses).
pub fn rocky_locking_gyr(orbit_au: f32, star_mass: f32, mass_earth: f32, radius_earth: f32) -> f32 {
    crate::star::locking_gyr(orbit_au, star_mass) * mass_earth / radius_earth.max(1e-3).powi(3)
}

/// The mass and radius (Earth units) of the rocky planet of a world of
/// `circumference_km` (see [`crate::world::rocky_gravity`]).
pub fn rocky_body(circumference_km: f32) -> (f32, f32) {
    let radius = circumference_km / EARTH_CIRCUMFERENCE_KM;
    (
        crate::world::rocky_gravity(radius) * radius * radius,
        radius,
    )
}

/// The age the tides have had to work on a planet of `star`, Gyr: the
/// Sun's, or the star's whole main-sequence life if shorter.
pub fn tidal_age_gyr(star: &StarInfo) -> f32 {
    AGE_GYR.min(star.lifetime_gyr)
}

/// Whether the star's tides have despun a rocky planet of `star` (locked or
/// held in the 3:2 resonance).
pub fn despun(star: &StarInfo) -> bool {
    star.locking_gyr < tidal_age_gyr(star)
}

/// Whether the star's tides have circularised the orbit of an Earth-like
/// rocky planet of `star` (see [`circularisation_gyr`]).
pub fn circularised(star: &StarInfo) -> bool {
    circularisation_gyr(star.orbit_au, star.mass) < tidal_age_gyr(star)
}

/// Whether the star's tides have circularised the orbit of a rocky planet
/// of `mass_earth` and `radius_earth` (see [`rocky_circularisation_gyr`]).
pub fn rocky_circularised(star: &StarInfo, mass_earth: f32, radius_earth: f32) -> bool {
    rocky_circularisation_gyr(star.orbit_au, star.mass, mass_earth, radius_earth)
        < tidal_age_gyr(star)
}

/// The axial tilt of the rocky world `p` drawn as `drawn` degrees, once its
/// star's tides have worked on it: none when they despun it (locked or in
/// the 3:2 resonance), its axis standing along its orbit's in a Cassini
/// state (Mercury's obliquity is 2 arcminutes, NASA Mercury fact sheet).
/// The torque that despins a planet erodes its obliquity on the same
/// timescale: "Earth-like obliquities of terrestrial planets in the IHZ
/// around <0.25 solar mass stars are eroded in <0.1 Gyr" (Heller, Leconte
/// & Barnes 2011, doi:10.1051/0004-6361/201015809). The moon of a giant
/// keeps the giant's tilt.
pub fn upright(p: &crate::world::WorldParams, drawn: f32) -> f32 {
    let star = rocky_star(p);
    if despun(&star) && crate::exomoon::HostGiant::draw(p).is_none() {
        0.0
    } else {
        drawn
    }
}

/// The star of world `p` taken as a rocky planet of its own mass and
/// radius, its resonance drawn from its seed.
fn rocky_star(p: &crate::world::WorldParams) -> StarInfo {
    let radius = p.circumference_km / EARTH_CIRCUMFERENCE_KM;
    let mass = p.mass_earth();
    StarInfo::for_planet(p.star, p.orbit_au, p.eccentricity, mass, radius, p.seed)
}

/// The star of world `p` with what its tides do to it: [`StarInfo::new`],
/// whose locking is a rocky planet's, with a giant's own instead for a
/// giant world (locked, never resonant, see [`giant_locking_gyr`]). A moon
/// (a moon's world, or a world orbiting a giant) is locked to its planet,
/// not to the star: never locked nor resonant against the star, whatever
/// its planet's orbit (its world takes the planet's star and orbit). A
/// reloaded world's [`locked_to_star`](crate::world::WorldParams::locked_to_star)
/// overrides the lock, so the tiles and epochs of a world generated under
/// an older rule agree with its stored map.
pub fn star_info(p: &crate::world::WorldParams) -> StarInfo {
    let mut star = rocky_star(p);
    if crate::moons::is_moon_world(p) || crate::exomoon::HostGiant::draw(p).is_some() {
        star.tidally_locked = false;
        star.spin_orbit_3_2 = false;
        star.spin_turns = 0.0;
    } else if let Some(mass) = p.giant_mass_earth {
        let radius = f64::from(p.circumference_km) / 40_075.0;
        star.locking_gyr = giant_locking_gyr(p.orbit_au, star.mass, mass, radius as f32);
        star.tidally_locked = (star.locking_gyr < tidal_age_gyr(&star)
            || star.equilibrium_k > crate::giant_world::HOT_GIANT_K)
            && giant_stays_locked(p.eccentricity);
        star.spin_orbit_3_2 = false;
        star.spin_turns = 0.0;
    }
    // A stored world keeps the lock and the resonance it was generated
    // with.
    if let Some(locked) = p.locked_to_star {
        star.tidally_locked = locked;
        if locked {
            star.spin_orbit_3_2 = false;
            star.spin_turns = 0.0;
        } else if let Some(turns) = p.resonance_to_star {
            star.spin_turns = turns;
            star.spin_orbit_3_2 = turns == RESONANCE_TURNS_PER_ORBIT;
        } else {
            // Made before the resonances were drawn: 3:2 or nothing.
            star.spin_turns = if star.spin_orbit_3_2 {
                RESONANCE_TURNS_PER_ORBIT
            } else {
                0.0
            };
        }
    }
    star
}

/// Jupiter's tidal Love number measured by Juno, `k₂ = 0.565 ± 0.006`
/// (Durante et al. 2020, doi:10.1029/2019GL086572, as quoted by Idini &
/// Stevenson 2021, arXiv:2102.09072; the hydrostatic models give 0.59).
pub const GIANT_K2: f64 = 0.565;
/// Jupiter's tidal quality factor, from the `k₂ / Q = (1.102 ± 0.203) ×
/// 10⁻⁵` Lainey et al. 2009 measured in the orbits of the Galilean moons
/// (doi:10.1038/nature08108) and [`GIANT_K2`]: about 5 × 10⁴, within the
/// 10⁴ to 10⁵ Goldreich & Soter 1966 found for the giants.
pub const GIANT_Q: f64 = GIANT_K2 / 1.102e-5;
/// Jupiter's moment of inertia over `MR²` (NASA Jupiter fact sheet), and
/// Earth's that [`crate::star::locking_gyr`] assumes (0.33).
const GIANT_INERTIA: f64 = 0.254;
const ROCKY_INERTIA: f64 = 0.33;
/// The first day of a giant, hours (Jupiter's is 9.9 h); a rocky planet's
/// is 12 h in [`crate::star::locking_gyr`].
const GIANT_FIRST_DAY_H: f64 = 10.0;

/// Time for the star's tides to lock a giant of `mass_earth` and
/// `radius_earth` (Earth units) on an orbit of `orbit_au` round a star of
/// `star_mass`, Gyr: the despinning time of Gladman et al. 1996
/// (`t = ω a⁶ I Q / (3 G M² k₂ R⁵)`, doi:10.1006/icar.1996.0117) with a
/// giant's own values ([`GIANT_K2`], [`GIANT_Q`], a moment of inertia of
/// 0.254 MR², a first day of 10 h), scaled from the rocky one: with
/// `I = α m R²` the time goes as `ω α Q m / (k₂ R³)`. Jupiter's density
/// makes it about 56 times the rocky time at the same place: a hot Jupiter
/// locks in a hundred thousand years, a giant at 0.3 AU from the Sun not
/// at all.
pub fn giant_locking_gyr(orbit_au: f32, star_mass: f32, mass_earth: f32, radius_earth: f32) -> f32 {
    let scale = (12.0 / GIANT_FIRST_DAY_H)
        * (GIANT_INERTIA / ROCKY_INERTIA)
        * (GIANT_Q / 100.0)
        * (0.3 / GIANT_K2)
        * f64::from(mass_earth)
        / f64::from(radius_earth.max(1e-3)).powi(3);
    crate::star::locking_gyr(orbit_au, star_mass) * scale as f32
}

/// Whether the star's tides have circularised the orbit of a giant of
/// `mass_earth` and `radius_earth` round `star`: [`circularisation_gyr`]
/// with a giant's `Q' = 3Q / (2k₂)` ([`GIANT_Q`], [`GIANT_K2`]), the time
/// going as `Q' m / R⁵`. Its large radius makes Jupiter circularise twice
/// as fast as the Earth at the same place: a hot Jupiter's orbit is round.
/// This `Q'` (about 1.4 × 10⁵) is the one Lainey et al. 2009 measured at
/// Io's tidal frequency; Jackson, Greenberg & Barnes 2008 fit a planetary
/// tidal Q of about 10^6.5 to the eccentricities of close-in giants, which
/// would circularise them about 20 times more slowly.
pub fn giant_circularised(star: &StarInfo, mass_earth: f32, radius_earth: f32) -> bool {
    let q_prime = 3.0 * GIANT_Q / (2.0 * GIANT_K2);
    let scale =
        q_prime / ROCKY_Q_PRIME * f64::from(mass_earth) / f64::from(radius_earth.max(1e-3)).powi(5);
    circularisation_gyr(star.orbit_au, star.mass) * (scale as f32) < tidal_age_gyr(star)
}

/// Turns per orbit of a fluid planet the tides have brought to the
/// pseudo-synchronous rate on an orbit of `eccentricity`: the rate at which
/// the tidal torque averaged over the orbit vanishes, `N(e) / Ω(e)` of Hut
/// 1981 (A&A 99, 126, bibcode 1981A&A....99..126H; no DOI), with
/// `N = (1 + 15/2 e² + 45/8 e⁴ + 5/16 e⁶) / (1 − e²)⁶` and
/// `Ω = (1 + 3e² + 3/8 e⁴) / (1 − e²)^(9/2)`, so that their ratio is the
/// polynomials' over `(1 − e²)^(3/2)`, as Levrard et al. 2007 and
/// Leconte et al. 2010 (doi:10.1051/0004-6361/201014337, eq. 13 and
/// appendix A) write it. To second order it is `1 + 6e²`, the value Leconte
/// et al. give. A fluid giant has no lasting bulge to hold a resonance: it
/// settles there, 1 on a circle, 1.06 at e = 0.1, 1.56 at e = 0.3.
pub fn pseudo_synchronous_turns(eccentricity: f32) -> f32 {
    let e2 = f64::from(eccentricity).powi(2);
    let n = 1.0 + 7.5 * e2 + 45.0 / 8.0 * e2 * e2 + 5.0 / 16.0 * e2.powi(3);
    let o = 1.0 + 3.0 * e2 + 3.0 / 8.0 * e2 * e2;
    (n / (o * (1.0 - e2).powf(1.5))) as f32
}

/// How far from one turn per orbit a giant's pseudo-synchronous rate may
/// be and still count as locked, one face to its star (a fifth of a turn
/// in four years at this tolerance; calibration, no source).
const LOCKED_TOLERANCE: f32 = 0.05;

/// Whether a giant despun by its star on an orbit of `eccentricity` is
/// locked: its pseudo-synchronous rate within [`LOCKED_TOLERANCE`] of one
/// turn per orbit (see [`pseudo_synchronous_turns`]). A more eccentric one
/// is despun but turns at that rate.
pub fn giant_stays_locked(eccentricity: f32) -> bool {
    pseudo_synchronous_turns(eccentricity) - 1.0 < LOCKED_TOLERANCE
}

/// Whether the star of `star` has locked a giant of `mass_earth` and
/// `radius_earth` (see [`giant_locking_gyr`]).
pub fn giant_locked(star: &StarInfo, mass_earth: f32, radius_earth: f32) -> bool {
    giant_locking_gyr(star.orbit_au, star.mass, mass_earth, radius_earth) < tidal_age_gyr(star)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::star::{SUN, StarInfo, orbit_at, parse_type};

    /// The share of 2000 seeds the eccentricity `e` leaves in each state:
    /// locked, 3:2, 2:1.
    fn shares(e: f32) -> [f32; 3] {
        let mut n = [0.0; 3];
        for seed in 0..2000 {
            let k = match capture(e, seed) {
                0.0 => 0,
                1.5 => 1,
                _ => 2,
            };
            n[k] += 1.0;
        }
        n.map(|x| x / 2000.0)
    }

    #[test]
    fn the_odds_of_capture_rise_with_the_eccentricity() {
        // A round orbit locks, whatever the seed (Ribas et al. 2016: below
        // 0.06 the synchronous state is the most probable).
        assert_eq!(shares(0.0), [1.0, 0.0, 0.0]);
        assert_eq!(shares(0.059)[0], 1.0);
        // The odds of the 3:2 resonance climb from 0 at 0.06 to 1 at 0.20
        // (Makarov 2012), 0.5 half way.
        let half = shares(0.13);
        assert!((half[1] - 0.5).abs() < 0.05, "{half:?}");
        assert!(shares(0.1)[1] < half[1] && half[1] < shares(0.17)[1]);
        // Mercury's e = 0.2056: the 2:1 resonance catches about a quarter
        // of the planets (0.23 in Makarov's integrations), the 3:2 the rest.
        let mercury = shares(0.2056);
        assert!((0.2..0.32).contains(&mercury[2]), "{mercury:?}");
        assert_eq!(mercury[0], 0.0);
        // From 0.34 the 2:1 is certain (Makarov: inevitable to 0.49), and
        // nothing higher is modelled.
        for e in [0.34, 0.4, 0.6] {
            assert_eq!(shares(e), [0.0, 0.0, 1.0], "{e}");
        }
        // The same seed always gives the same state.
        assert_eq!(capture(0.15, 7), capture(0.15, 7));
    }

    #[test]
    fn a_despun_planet_on_a_round_orbit_is_locked_and_an_eccentric_one_may_not_be() {
        // Mercury: 0.387 AU from the Sun, e = 0.2056. Despun (0.56 Gyr),
        // never circularised.
        let mercury = StarInfo::new(SUN, 0.387, 0.2056);
        assert!(despun(&mercury) && !circularised(&mercury));
        assert!(!mercury.tidally_locked && mercury.spin_turns >= 1.5);
        assert_eq!(mercury.spin_orbit_3_2, mercury.spin_turns == 1.5);
        // On a circle, or on an orbit as round as Proxima b's, it is locked.
        for e in [0.0, 0.0478, 0.059] {
            let round = StarInfo::new(SUN, 0.387, e);
            assert!(round.tidally_locked && round.spin_turns == 0.0, "{e}");
        }
        // Seeds give both states at one eccentricity.
        let red = parse_type("M5V").unwrap();
        let a = orbit_at(red, 0.5);
        let states: Vec<f32> = (0..200)
            .map(|seed| StarInfo::for_planet(red, a, 0.13, 1.0, 1.0, seed).spin_turns)
            .collect();
        assert!(states.contains(&0.0) && states.contains(&1.5));
        // Earth is neither.
        let earth = StarInfo::new(SUN, 1.0, 0.0167);
        assert!(!earth.tidally_locked && earth.spin_turns == 0.0);
    }

    #[test]
    fn the_tides_scale_with_the_planets_mass_and_radius() {
        // A super-Earth of 5.5 Earth masses, 1.76 radii: about three times
        // faster to circularise, as heavy for its radius to despin.
        let earth = circularisation_gyr(0.05, 1.0);
        let big = rocky_circularisation_gyr(0.05, 1.0, 5.5, 1.76);
        assert!((2.5..3.5).contains(&(earth / big)), "{earth} {big}");
        let l = rocky_locking_gyr(0.3, 1.0, 5.5, 1.76) / crate::star::locking_gyr(0.3, 1.0);
        assert!((0.9..1.2).contains(&l), "{l}");
        // A Mars (0.107 masses, 0.53 radii) is slower to circularise: its
        // time goes as m / R^5.
        assert!(rocky_circularisation_gyr(0.05, 1.0, 0.107, 0.53) > earth);
        // An orbit at the edge: the super-Earth's is circular where the
        // Earth's is not.
        let star = StarInfo::new(SUN, 0.08, 0.0);
        assert!(!circularised(&star) && rocky_circularised(&star, 5.5, 1.76));
    }

    #[test]
    fn the_mercury_like_preset_is_in_the_three_two_resonance_whatever_the_seed() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../presets/examples/mercury-like.toml"
        );
        for seed in 0..60 {
            let request = crate::request::WorldRequest {
                planet: Some(path.into()),
                seed: Some(seed),
                width: Some(64),
                height: Some(32),
                ..Default::default()
            };
            let p = request.resolve(&mut rand::rng()).unwrap().params;
            let star = star_info(&p);
            // Mercury's observed state, pinned by the preset's
            // `[star] resonance`, not drawn.
            assert!(star.spin_orbit_3_2 && !star.tidally_locked, "{seed}");
            assert_eq!(star.spin_turns, 1.5, "{seed}");
            assert!((p.eccentricity - 0.2056).abs() < 1e-6);
            // Planets with moons: their day is the resonance's.
            if let Some(system) = crate::moons::MoonSystem::draw(&p) {
                let day = system.planet_day_hours;
                assert!(
                    (day * 1.5 / (star.year_days * 24.0) - 1.0).abs() < 1e-4,
                    "{day}"
                );
            }
        }
    }

    #[test]
    fn a_preset_resonance_is_checked() {
        let spec = |r: &str| crate::star::StarSpec {
            resonance: Some(r.into()),
            ..Default::default()
        };
        for ok in ["1:1", "3:2", "2:1"] {
            assert!(spec(ok).validate().is_ok(), "{ok}");
        }
        let err = spec("5:2").validate().unwrap_err();
        assert!(err.contains("star.resonance"), "{err}");
        assert!(spec("").validate().is_err());
    }

    #[test]
    fn worlds_made_before_the_resonances_keep_their_spin() {
        // An eccentric world stored in the 3:2 resonance (`spin_orbit_3_2`
        // and no `spin_turns`, as before the draw) keeps it whatever its
        // seed; one stored with neither is free; one stored locked is
        // locked.
        let red = parse_type("M5V").unwrap();
        let base = crate::request::WorldRequest {
            planet: Some("terran".into()),
            host: Some(crate::exomoon::HostMode::None),
            width: Some(64),
            height: Some(32),
            ..Default::default()
        }
        .resolve(&mut rand::rng())
        .unwrap()
        .params;
        let world = |seed: u32, locked: bool, turns: Option<f32>| crate::world::WorldParams {
            star: red,
            orbit_au: orbit_at(red, 0.5),
            eccentricity: 0.1,
            seed,
            locked_to_star: Some(locked),
            resonance_to_star: turns,
            ..base.clone()
        };
        for seed in 0..50 {
            let old = star_info(&world(seed, false, Some(RESONANCE_TURNS_PER_ORBIT)));
            assert!(old.spin_orbit_3_2 && !old.tidally_locked, "{seed}");
            assert_eq!(old.spin_turns, 1.5);
            let free = star_info(&world(seed, false, Some(0.0)));
            assert!(!free.spin_orbit_3_2 && !free.tidally_locked && free.spin_turns == 0.0);
            let locked = star_info(&world(seed, true, Some(1.5)));
            assert!(locked.tidally_locked && !locked.spin_orbit_3_2 && locked.spin_turns == 0.0);
        }
    }

    #[test]
    fn despun_planets_stand_upright_and_lone_locked_ones_turn_on_circles() {
        let red = |seed: u32| crate::request::WorldRequest {
            planet: Some("terran".into()),
            star: Some("M5V".into()),
            hz_position: Some(0.5),
            seed: Some(seed),
            width: Some(64),
            height: Some(32),
            ..Default::default()
        };
        for seed in 0..20 {
            let p = red(seed).resolve(&mut rand::rng()).unwrap().params;
            assert_eq!(p.axial_tilt_deg, 0.0);
            let star = star_info(&p);
            assert!(star.tidally_locked || star.spin_turns > 0.0);
        }
        // Close in, the orbit is circularised: locked.
        let close = crate::request::WorldRequest {
            planet: Some("barren".into()),
            star: Some("M5V".into()),
            orbit_au: Some(0.015),
            seed: Some(1),
            width: Some(64),
            height: Some(32),
            ..Default::default()
        };
        let p = close.resolve(&mut rand::rng()).unwrap().params;
        assert_eq!(p.eccentricity, 0.0);
        assert!(star_info(&p).tidally_locked);
    }

    #[test]
    fn the_tides_circularise_only_the_closest_orbits() {
        // About 10⁸ Gyr at 1 AU from the Sun.
        let one_au = circularisation_gyr(1.0, 1.0);
        assert!((0.9e8..1.3e8).contains(&one_au), "{one_au}");
        assert!(circularisation_gyr(0.05, 1.0) < 4.6);
        assert!(circularisation_gyr(0.1, 1.0) > 4.6);
        // Far slower than the despinning.
        let ratio = one_au / crate::star::locking_gyr(1.0, 1.0);
        assert!((1e5..1e7).contains(&ratio), "{ratio}");
        // A red dwarf's habitable zone is despun but not circularised.
        let red = parse_type("M5V").unwrap();
        let hz = StarInfo::new(red, orbit_at(red, 0.5), 0.0);
        assert!(despun(&hz) && !circularised(&hz));
        assert!(circularised(&StarInfo::new(red, 0.02, 0.0)));
    }

    #[test]
    fn eccentric_giants_turn_pseudo_synchronously() {
        // Hut 1981: one turn per orbit on a circle, 1 + 6e² to second
        // order (Leconte et al. 2010), 1.56 at e = 0.3.
        assert!((pseudo_synchronous_turns(0.0) - 1.0).abs() < 1e-6);
        assert!((pseudo_synchronous_turns(0.05) - 1.015).abs() < 1e-3);
        assert!((pseudo_synchronous_turns(0.3) - 1.557).abs() < 0.01);
        assert!(giant_stays_locked(0.0) && giant_stays_locked(0.05));
        assert!(!giant_stays_locked(0.2));
        // A giant of 0.05 AU round the Sun with e = 0.3 is despun but
        // turns 1.56 times per orbit, not once.
        let mut p = crate::request::WorldRequest {
            planet: Some("gas-giant".into()),
            star: Some("G2V".into()),
            orbit_au: Some(0.05),
            eccentricity: Some(0.3),
            seed: Some(2),
            width: Some(64),
            height: Some(32),
            ..Default::default()
        }
        .resolve(&mut rand::rng())
        .unwrap()
        .params;
        let info = crate::star::StarInfo::new(p.star, p.orbit_au, p.eccentricity);
        let mass = p.giant_mass_earth.unwrap();
        let look = crate::giant_world::Look::draw(&p, mass).info;
        assert!(giant_locked(&info, mass, look.radius_earth));
        assert!(!star_info(&p).tidally_locked);
        let turns = info.year_days * 24.0 / look.rotation_hours;
        assert!(
            (turns - pseudo_synchronous_turns(0.3)).abs() < 1e-3,
            "{turns}"
        );
        // On a circle it is locked.
        p.eccentricity = 0.0;
        assert!(star_info(&p).tidally_locked);
    }

    #[test]
    fn giants_lock_with_their_own_tides() {
        // Jupiter: 317.8 Earth masses, 11.21 Earth radii.
        let at = |au: f32| giant_locking_gyr(au, 1.0, 317.8, 11.21);
        let ratio = at(0.3) / crate::star::locking_gyr(0.3, 1.0);
        assert!((50.0..65.0).contains(&ratio), "{ratio}");
        // A hot Jupiter locks fast; a warm one at 0.3 AU does not.
        assert!(at(0.05) < 1e-3);
        assert!(at(0.3) > 4.6);
        assert!((GIANT_Q - 5.1e4).abs() < 1e3, "{GIANT_Q}");
    }
}
