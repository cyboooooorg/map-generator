//! Impact flux and crater size of a world against the Moon's, from its
//! orbit.
//!
//! A world with a model ([`WorldParams::model`]) has its impacts from its
//! orbit: the impactors of its system (`belts::impact_factor`, the giants
//! and belts), the asteroids' supply where it orbits, their speed there and
//! the world's own gravity. A moon (a round one with its model too) and a
//! world without a model keep the Moon's craters at their system's rate:
//! a moon's impactors are focused by its planet, which the swarm rate
//! below does not know. See
//! `docs/science/impact-craters.md`.
//!
//! Every ratio is against the Moon, which is the reference of the crater
//! chronology (`craters::crater_density`): a body at 1 AU from a
//! Sun-like star, with the Moon's gravity and escape speed.

use crate::world::WorldParams;

/// A world's impacts against the Moon's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImpactRates {
    /// Impactors striking it per unit area and time.
    pub flux: f64,
    /// Diameter of the crater the same impactor digs on it.
    pub crater_scale: f64,
}

/// Mean impact speed on the Moon, km/s: Le Feuvre & Wieczorek (2011),
/// "Nonuniform cratering of the Moon and a revised crater chronology of
/// the inner Solar System", Icarus 214, doi:10.1016/j.icarus.2011.03.010,
/// § 4.1 (read, HAL hal-00768797).
const MOON_IMPACT_KM_S: f64 = 19.7;
/// Earth's mean impact speed over the Moon's, same source, Table 3.
const EARTH_IMPACT_RATIO: f64 = 1.04;
/// The Moon's mean radius, km (NASA Moon fact sheet).
const MOON_RADIUS_KM: f64 = 1737.4;
/// π-scaling exponent μ of hard rock, Holsapple & Housen, "Craters from
/// impacts and explosions" (the tables behind Holsapple's cratering
/// calculator, after Holsapple 1993, Annu. Rev. Earth Planet. Sci. 21,
/// read): 0.55 for wet and rocky targets, 0.4 for dry soils and sands.
const MU: f64 = 0.55;
/// Main-belt asteroids per near-Earth asteroid, both larger than 1 km:
/// (1.2 ± 0.5) × 10⁶ in the main belt (Tedesco & Désert 2002, AJ 123,
/// 2070, abstract read) over 981 ± 19 near-Earth ones (Mainzer et al.
/// 2011, ApJ 743, 156, abstract read).
const BELT_PER_CROSSER: f64 = 1.2e6 / 981.0;
/// Spread of the belt's eccentricities, as a Rayleigh law's σ: how far
/// inside the belt its asteroids still cross an orbit. Calibration, no
/// source: set so that a Mars-like world at 1.5 AU of a Sun-like star
/// gets Hartmann's 2.6 times the Moon's flux under [`MOST_CROSSERS`]
/// and [`MOST_IMPACTS`].
const BELT_ECCENTRICITY: f64 = 0.135;
/// The asteroid belt of a star of luminosity `L` (Suns) lies just inside
/// its ice line, at `2.7 √L` AU, as a system places it (`system.rs`).
const BELT_AU: f64 = 2.7;
/// Most crossers an orbit gets against the Moon's, as the ceiling of
/// [`bounded`]. A planet near the belt clears it or is destabilised by it,
/// so the belt's own asteroids never strike it as they strike each other;
/// the Mars-crossers that remain give Mars about three times the Moon's
/// flux (Le Feuvre & Wieczorek 2011, Table 3: 2.76 averaged, 3.20 today).
/// Calibration, no source: 6, about twice what Mars gets, keeps Mars's 2.6
/// reachable.
const MOST_CROSSERS: f64 = 6.0;
/// Most impacts an orbit gets against the Moon's, the crossers, their
/// speed and the world's focusing together, as the ceiling of [`bounded`]:
/// the encounter speed grows as `√(M★ / a)`, without end close to a star
/// (14 times the Moon's flux at 0.005 AU of the Sun before this ceiling).
/// Calibration, no source: 20 keeps the Solar System's look-alikes within
/// a few percent of their unbounded flux (Mercury, Venus and Earth lose
/// 2–3 %), and the closest orbits under 8 times Mars's.
const MOST_IMPACTS: f64 = 20.0;
/// Fastest mean impact a crater is scaled with, against the Moon's
/// ([`MOON_IMPACT_KM_S`]): Mercury's mean impact speed, 2.16 times the
/// Moon's (Le Feuvre & Wieczorek 2011, Table 3), the fastest of the
/// terrestrial planets, as the ceiling of [`bounded_speed`]. Calibration:
/// the speed's `√(M★ / a)` reaches any value close to a star, and the
/// π-scaling has not been tested much beyond the speeds of the Solar
/// System's impacts.
const FASTEST_IMPACT: f64 = 2.16;
/// Speed ratio up to which [`bounded_speed`] leaves the impact speed as it
/// is. Calibration, no source: above every Solar System look-alike's
/// (Mercury's 1.42), so that none of them moves.
const SPEED_KNEE: f64 = 1.5;

/// Escape speed, km/s, of a body of surface gravity `g` (m/s²) and
/// radius `radius_km`: `√(2 g R)`.
fn escape_km_s(g: f64, radius_km: f64) -> f64 {
    (2.0 * g / 1000.0 * radius_km).sqrt()
}

/// Encounter speed of the impactors far from the body (`v∞`), km/s, on
/// an orbit of `orbit_au` around a star of `star_mass` Suns: Earth's,
/// from its mean impact speed without its own escape speed, scaled with
/// the orbital speed `√(G M★ / a)`.
fn encounter_km_s(star_mass: f64, orbit_au: f64) -> f64 {
    let earth_escape = escape_km_s(
        crate::craters::EARTH_GRAVITY,
        f64::from(crate::noise::EARTH_CIRCUMFERENCE_KM) / std::f64::consts::TAU,
    );
    let earth = ((MOON_IMPACT_KM_S * EARTH_IMPACT_RATIO).powi(2) - earth_escape.powi(2)).sqrt();
    earth * (star_mass / orbit_au).sqrt()
}

/// The asteroids crossing an orbit `x` times the belt's distance: the
/// near-Earth ones everywhere (1), and the belt's own where their
/// eccentricity reaches it, `1 − x` for a Rayleigh law of spread
/// [`BELT_ECCENTRICITY`]. Many more near the belt (Mars) than inside
/// (Earth, Venus, Mercury), the belt's own beyond.
fn crossers(x: f64) -> f64 {
    let e = 1.0 - x;
    1.0 + BELT_PER_CROSSER * (-e * e / (2.0 * BELT_ECCENTRICITY * BELT_ECCENTRICITY)).exp()
}

/// A ratio `s` against the Moon's bounded smoothly by `ceiling`:
/// `s (1 + S) / (s + S)` with `S` = ceiling − 1, 1 at the Moon's and
/// rising to the ceiling, never above it.
fn bounded(s: f64, ceiling: f64) -> f64 {
    let span = ceiling - 1.0;
    s * (1.0 + span) / (s + span)
}

/// An impact speed ratio `s` against the Moon's, unchanged up to
/// [`SPEED_KNEE`] and bounded smoothly beyond it by [`FASTEST_IMPACT`]
/// (`tanh`, its slope continuous at the knee).
fn bounded_speed(s: f64) -> f64 {
    if s <= SPEED_KNEE {
        return s;
    }
    let room = FASTEST_IMPACT - SPEED_KNEE;
    SPEED_KNEE + room * ((s - SPEED_KNEE) / room).tanh()
}

/// The impactors of the world's system against the Solar System's: its
/// giants and belts (`belts::impact_factor`, set by the system draw), or
/// the giants drawn for it alone (`giants::impact_factor`, 1 without any).
fn impactors(p: &WorldParams) -> f64 {
    match p.impact_factor {
        Some(factor) => f64::from(factor),
        None => crate::giants::GiantSystem::draw(p).map_or(1.0, |s| f64::from(s.impact_factor)),
    }
}

/// Impact flux and crater size of the world `p` against the Moon's.
///
/// With a model, the flux per unit area is the impact rate of a body in a
/// swarm, crossers × encounter speed × gravitational cross-section
/// `π R² (1 + v_esc² / v∞²)`, each against the Moon's, times its system's
/// impactors; the crossers are bounded by [`MOST_CROSSERS`], the whole
/// orbit's flux by [`MOST_IMPACTS`]. The crater of the same impactor
/// scales as `g^(−μ/(2+μ)) v^(2μ/(2+μ))` (π-scaling in the gravity
/// regime), with the impact speed `v = √(v∞² + v_esc²)` against the
/// Moon's cited [`MOON_IMPACT_KM_S`], bounded by [`FASTEST_IMPACT`].
/// A moon, or a world without a model: the system's impactors and the
/// Moon's craters (its planet's focusing is not modelled).
pub fn rates(p: &WorldParams) -> ImpactRates {
    let impactors = impactors(p);
    if p.model.is_none() || crate::moons::is_moon_world(p) {
        return ImpactRates {
            flux: impactors,
            crater_scale: 1.0,
        };
    }
    let star = crate::spin::star_info(p);
    let star_mass = f64::from(star.mass).max(1e-3);
    let luminosity = f64::from(star.luminosity).max(1e-6);
    let orbit_au = f64::from(p.orbit_au).max(1e-3);
    let gravity = crate::craters::EARTH_GRAVITY * f64::from(p.gravity_modifier());
    let radius_km = f64::from(p.circumference_km) / std::f64::consts::TAU;

    let v_inf = encounter_km_s(star_mass, orbit_au);
    let v_esc = escape_km_s(gravity, radius_km);
    let moon_inf = encounter_km_s(1.0, 1.0);
    let moon_esc = escape_km_s(crate::craters::MOON_GRAVITY, MOON_RADIUS_KM);

    let supply = bounded(
        crossers(orbit_au / (BELT_AU * luminosity.sqrt())) / crossers(1.0 / BELT_AU),
        MOST_CROSSERS,
    );
    let speed = v_inf / moon_inf;
    let focusing = (1.0 + (v_esc / v_inf).powi(2)) / (1.0 + (moon_esc / moon_inf).powi(2));
    let flux = impactors * bounded(supply * speed * focusing, MOST_IMPACTS);

    // Against the Moon's cited speed, not one rebuilt from Earth's
    // encounter speed (17.3 km/s): Le Feuvre & Wieczorek's Moon and Earth
    // do not share one v∞.
    let impact = (v_inf.powi(2) + v_esc.powi(2)).sqrt();
    let crater_scale = (gravity / crate::craters::MOON_GRAVITY).powf(-MU / (2.0 + MU))
        * bounded_speed(impact / MOON_IMPACT_KM_S).powf(2.0 * MU / (2.0 + MU));
    ImpactRates { flux, crater_scale }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preset(name: &str) -> WorldParams {
        crate::generation::tests::preset_params(name, 1, 64, 32)
    }

    /// A look-alike among the Solar System's impactors: Jupiter beyond it
    /// and the main belt, `belts::impact_factor` 1 inside the belt.
    fn solar(name: &str) -> WorldParams {
        WorldParams {
            impact_factor: Some(1.0),
            ..preset(name)
        }
    }

    fn near(x: f64, target: f64) -> bool {
        (x / target - 1.0).abs() <= 0.2
    }

    /// The crater of the same impactor against the Moon's, by the same
    /// π-scaling as [`rates`], from a gravity and an impact speed ratio.
    fn pi_scaled(gravity_ratio: f64, speed_ratio: f64) -> f64 {
        let mu = 0.55;
        gravity_ratio.powf(-mu / (2.0 + mu)) * speed_ratio.powf(2.0 * mu / (2.0 + mu))
    }

    #[test]
    fn solar_system_look_alikes_have_their_published_crater_production() {
        // Mars: Hartmann's isochrons (PSI page), 2.6 for asteroids and a
        // crater 0.865 × 0.868 = 0.751 times the Moon's.
        let mars = rates(&solar("mars-like"));
        assert!(near(mars.flux, 2.6), "mars flux {}", mars.flux);
        assert!(
            near(mars.crater_scale, 0.751),
            "mars scale {}",
            mars.crater_scale
        );
        // Mercury, Venus, Earth: Le Feuvre & Wieczorek (2011), Table 3,
        // impact rate, mean impact speed and gravity against the Moon's;
        // the crater from the last two through the same π-scaling.
        for (name, flux, speed, gravity) in [
            ("mercury-like", 1.82, 2.16, 2.2),
            ("venus-like", 1.75, 1.28, 5.3),
            ("earth-like", 1.58, 1.04, 5.9),
        ] {
            let r = rates(&solar(name));
            assert!(near(r.flux, flux), "{name} flux {}", r.flux);
            let scale = pi_scaled(gravity, speed);
            assert!(
                near(r.crater_scale, scale),
                "{name} scale {} against {scale}",
                r.crater_scale
            );
        }
    }

    #[test]
    fn no_orbit_gets_more_than_a_few_times_marss_flux() {
        // Swept from 0.3 to 3 AU around a G2V star among the Solar
        // System's impactors: the flux, speed and focusing included, stays
        // within 3.5 times Mars's 2.6, finite and above zero.
        for name in ["mars-like", "earth-like"] {
            let mut p = solar(name);
            p.star = crate::star::parse_type("G2V").unwrap();
            for k in 0..=270 {
                p.orbit_au = 0.3 + 0.01 * k as f32;
                let flux = rates(&p).flux;
                assert!(
                    flux.is_finite() && flux > 0.0 && flux < 3.5 * 2.6,
                    "{name} at {} AU: {flux}",
                    p.orbit_au
                );
            }
        }
        for s in [1.0, 10.0, 1e3, 1e9] {
            assert!(bounded(s, MOST_IMPACTS) <= MOST_IMPACTS, "{s}");
            assert!(bounded(s, MOST_CROSSERS) <= MOST_CROSSERS, "{s}");
        }
        assert!((bounded(1.0, MOST_IMPACTS) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn no_orbit_or_star_pushes_the_impacts_past_their_ceilings() {
        // From the closest orbit a request takes to 30 AU, around an M
        // dwarf, the Sun and an A star: the flux under its ceiling, the
        // crater under the one the speed's ceiling gives, everywhere.
        let speed_exponent = 2.0 * MU / (2.0 + MU);
        for name in ["mercury-like", "earth-like", "mars-like"] {
            for star in ["M5V", "G2V", "A0V"] {
                let mut p = solar(name);
                p.star = crate::star::parse_type(star).unwrap();
                let gravity = crate::craters::EARTH_GRAVITY * f64::from(p.gravity_modifier());
                let ceiling = (gravity / crate::craters::MOON_GRAVITY).powf(-MU / (2.0 + MU))
                    * FASTEST_IMPACT.powf(speed_exponent);
                let (lo, hi) = (f64::from(crate::request::ORBIT_AU.min).ln(), 30f64.ln());
                for k in 0..=200 {
                    p.orbit_au = (lo + (hi - lo) * f64::from(k) / 200.0).exp() as f32;
                    let r = rates(&p);
                    let at = format!("{name} around {star} at {} AU", p.orbit_au);
                    assert!(r.flux.is_finite() && r.flux > 0.0, "{at}: {}", r.flux);
                    assert!(r.flux <= MOST_IMPACTS, "{at}: flux {}", r.flux);
                    assert!(
                        r.crater_scale.is_finite() && r.crater_scale <= ceiling,
                        "{at}: scale {} over {ceiling}",
                        r.crater_scale
                    );
                }
            }
        }
    }

    #[test]
    fn a_lone_world_takes_the_solar_systems_impactors() {
        // A planet alone with its star: no belts, no giants, so no
        // impactors drawn for it. Its unknown context takes the Solar
        // System's, 1: its flux is its orbit's alone.
        let request = crate::request::WorldRequest {
            planet: Some("earth-like".to_string()),
            seed: Some(3),
            alone: Some(true),
            moons: Some(crate::moons::MoonsMode::None),
            ..Default::default()
        };
        let lone = request
            .resolve(&mut <rand::rngs::StdRng as rand::SeedableRng>::seed_from_u64(3))
            .unwrap()
            .params;
        assert!(lone.impact_factor.is_none());
        assert_eq!(lone.giants, crate::giants::GiantsMode::None);
        let r = rates(&lone);
        let orbit = rates(&WorldParams {
            impact_factor: Some(1.0),
            ..lone.clone()
        });
        assert!(r.flux.is_finite() && r.flux > 0.0, "{}", r.flux);
        assert_eq!(r.flux, orbit.flux);
    }

    #[test]
    fn the_moons_cited_impact_speed_is_the_reference() {
        // Earth's impacts are 1.04 times as fast as the Moon's 19.7 km/s
        // (Le Feuvre & Wieczorek 2011, Table 3): its crater against the
        // Moon's takes that ratio, not the one against a Moon rebuilt from
        // Earth's encounter speed (17.3 km/s).
        let p = solar("earth-like");
        let gravity = crate::craters::EARTH_GRAVITY * f64::from(p.gravity_modifier());
        let radius_km = f64::from(p.circumference_km) / std::f64::consts::TAU;
        let impact = (encounter_km_s(1.0, f64::from(p.orbit_au)).powi(2)
            + escape_km_s(gravity, radius_km).powi(2))
        .sqrt();
        let expected = pi_scaled(
            gravity / crate::craters::MOON_GRAVITY,
            impact / MOON_IMPACT_KM_S,
        );
        let scale = rates(&p).crater_scale;
        assert!(
            (scale / expected - 1.0).abs() < 1e-3,
            "{scale} against {expected}"
        );
    }

    #[test]
    fn moons_keep_the_moons_production() {
        // Without a model: the system's impactors alone, the Moon's craters.
        let mut p = preset("mars-like");
        p.model = None;
        p.impact_factor = Some(1.3);
        let r = rates(&p);
        assert!((r.flux - 1.3).abs() < 1e-6, "{}", r.flux);
        assert_eq!(r.crater_scale, 1.0);
        p.impact_factor = None;
        let giants =
            crate::giants::GiantSystem::draw(&p).map_or(1.0, |s| f64::from(s.impact_factor));
        assert_eq!(rates(&p).flux, giants);
    }
}
