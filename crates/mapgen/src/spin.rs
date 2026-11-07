//! How a star's tides set a planet's spin
//! (`docs/science/tidal-locking.md`).
//!
//! **Despun.** The star's tides brake a planet's spin; [`locking_gyr`]
//! gives the time they take for an Earth-like planet, [`giant_locking_gyr`]
//! for a giant. A planet whose time is shorter than the star's age
//! ([`AGE_GYR`], or the star's whole life when shorter) is despun.
//!
//! **Locked or resonant.** A despun rocky planet on an orbit of
//! eccentricity [`RESONANCE_CAPTURE_ECCENTRICITY`] (0.06) or more is held
//! in the 3:2 spin–orbit resonance, Mercury's three turns every two
//! orbits; below it, the most probable state is one turn per orbit, locked
//! with one face to its star (Ribas et al. 2016, for Proxima b, an Earth
//! of a red dwarf's habitable zone; Makarov 2012 finds capture into 3:2
//! inevitable for Mercury from e = 0.2 to 0.41, ever less likely below).
//! The tides circularise the orbit far more slowly than they despin the
//! planet ([`circularisation_gyr`]); a planet whose orbit they have
//! circularised ends locked. Higher resonances (2:1, 5:2), which large
//! eccentricities favour, are not modelled. Giants are fluid, with no
//! lasting bulge for the star to hold: a despun giant is locked.
//!
//! [`locking_gyr`]: crate::star::locking_gyr
//! [`AGE_GYR`]: crate::star::AGE_GYR

use crate::star::{AGE_GYR, StarInfo};

/// Eccentricity from which a despun rocky planet is held in the 3:2
/// resonance rather than locked, from Ribas et al. 2016, §4.6
/// (doi:10.1051/0004-6361/201629576), for an Earth-mass planet of a red
/// dwarf with a Maxwell rheology: "the ω≈3n/2 rotation becomes stable for
/// an eccentricity greater than 0.06–0.07"; "At eccentricities lower than
/// 0.06, the most probable state becomes the synchronous one". Taken as a
/// sharp line (in reality the odds of capture rise with the
/// eccentricity: Makarov 2012, doi:10.1088/0004-637X/752/1/73).
pub const RESONANCE_CAPTURE_ECCENTRICITY: f32 = 0.06;

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
/// circularised in 4.6 Gyr. Every rocky planet is given Earth's mass and
/// radius here, as in [`crate::star::locking_gyr`]: the time goes as
/// `m / R⁵`, so a super-Earth of 5.5 Earth masses (1.76 radii) would
/// circularise about three times faster.
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

/// Whether the star's tides have circularised the orbit of a rocky planet
/// of `star` (see [`circularisation_gyr`]).
pub fn circularised(star: &StarInfo) -> bool {
    circularisation_gyr(star.orbit_au, star.mass) < tidal_age_gyr(star)
}

/// The eccentricity a rocky planet of `star` keeps when `drawn` is drawn
/// for it: none when the tides have circularised its orbit.
pub fn tidal_eccentricity(star: &StarInfo, drawn: f32) -> f32 {
    if circularised(star) { 0.0 } else { drawn }
}

/// Whether a rocky planet whose locking time is `locking_gyr`, round a
/// star of `age_gyr`, on an orbit of `eccentricity`, is locked to its star:
/// despun, below [`RESONANCE_CAPTURE_ECCENTRICITY`], one turn per orbit.
pub fn locked(locking_gyr: f32, age_gyr: f32, eccentricity: f32) -> bool {
    locking_gyr < age_gyr && eccentricity < RESONANCE_CAPTURE_ECCENTRICITY
}

/// Whether a rocky planet despun on an orbit of `eccentricity` turns three
/// times every two orbits (see the module's doc).
pub fn resonant(locking_gyr: f32, age_gyr: f32, eccentricity: f32) -> bool {
    locking_gyr < age_gyr && eccentricity >= RESONANCE_CAPTURE_ECCENTRICITY
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
    let star = StarInfo::new(p.star, p.orbit_au, p.eccentricity);
    if despun(&star) && crate::exomoon::HostGiant::draw(p).is_none() {
        0.0
    } else {
        drawn
    }
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
    let mut star = StarInfo::new(p.star, p.orbit_au, p.eccentricity);
    if crate::moons::is_moon_world(p) || crate::exomoon::HostGiant::draw(p).is_some() {
        star.tidally_locked = false;
        star.spin_orbit_3_2 = false;
    } else if let Some(mass) = p.giant_mass_earth {
        let radius = f64::from(p.circumference_km) / 40_075.0;
        star.locking_gyr = giant_locking_gyr(p.orbit_au, star.mass, mass, radius as f32);
        star.tidally_locked = star.locking_gyr < tidal_age_gyr(&star)
            || star.equilibrium_k > crate::giant_world::HOT_GIANT_K;
        star.spin_orbit_3_2 = false;
    }
    // A stored world keeps the lock it was generated with.
    if let Some(locked) = p.locked_to_star {
        star.tidally_locked = locked;
        star.spin_orbit_3_2 &= !locked;
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

/// Whether the star of `star` has locked a giant of `mass_earth` and
/// `radius_earth` (see [`giant_locking_gyr`]).
pub fn giant_locked(star: &StarInfo, mass_earth: f32, radius_earth: f32) -> bool {
    giant_locking_gyr(star.orbit_au, star.mass, mass_earth, radius_earth) < tidal_age_gyr(star)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::star::{SUN, StarInfo, orbit_at, parse_type};

    #[test]
    fn a_mercury_like_planet_is_held_in_the_three_two_resonance() {
        // Mercury: 0.387 AU from the Sun, e = 0.2056. Despun (0.56 Gyr),
        // never circularised: three turns every two orbits.
        let mercury = StarInfo::new(SUN, 0.387, 0.2056);
        assert!(despun(&mercury) && !circularised(&mercury));
        assert!(mercury.spin_orbit_3_2 && !mercury.tidally_locked);
        // On a circle, or on an orbit as round as Proxima b's, it would be
        // locked (Ribas et al. 2016).
        for e in [0.0, 0.0478, 0.059] {
            let round = StarInfo::new(SUN, 0.387, e);
            assert!(round.tidally_locked && !round.spin_orbit_3_2, "{e}");
        }
        // A habitable-zone Earth of an M5 dwarf at e = 0.0478 is locked,
        // at 0.1 resonant.
        let red = parse_type("M5V").unwrap();
        let a = orbit_at(red, 0.5);
        assert!(StarInfo::new(red, a, 0.0478).tidally_locked);
        assert!(StarInfo::new(red, a, 0.1).spin_orbit_3_2);
        // Earth is neither.
        let earth = StarInfo::new(SUN, 1.0, 0.0167);
        assert!(!earth.tidally_locked && !earth.spin_orbit_3_2);
    }

    #[test]
    fn the_mercury_like_preset_turns_three_times_in_two_years() {
        let request = crate::request::WorldRequest {
            planet: Some(
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../presets/examples/mercury-like.toml"
                )
                .into(),
            ),
            seed: Some(3),
            width: Some(64),
            height: Some(32),
            ..Default::default()
        };
        let p = request.resolve(&mut rand::rng()).unwrap().params;
        let star = star_info(&p);
        assert!(star.spin_orbit_3_2 && !star.tidally_locked);
        assert!((p.eccentricity - 0.2056).abs() < 1e-6);
        let day = crate::moons::MoonSystem::draw(&p).unwrap().planet_day_hours;
        assert!(
            (day * 1.5 / (star.year_days * 24.0) - 1.0).abs() < 1e-4,
            "{day}"
        );
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
            assert!(star.tidally_locked || star.spin_orbit_3_2);
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
        assert_eq!(tidal_eccentricity(&hz, 0.1), 0.1);
        let close = StarInfo::new(red, 0.02, 0.0);
        assert_eq!(tidal_eccentricity(&close, 0.1), 0.0);
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
