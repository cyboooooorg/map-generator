//! Tide spheres: how far from a body its host's tides reach, as the radii
//! the planet view draws round the host.
//!
//! Each sphere inverts, for the distance, a formula the generator already
//! uses to decide something at a given distance; none is new science:
//!
//! - **Hill**: where the host's gravity outweighs its primary's tide
//!   ([`hill_km`]);
//! - **locking**: within which the host has despun the satellite by the
//!   system's age ([`locking_km`], from [`crate::moons`]' despinning time);
//! - **heating**: within which the satellite's tidal heat exceeds its
//!   radiogenic heat ([`heating_km`], from [`crate::planet::heat`]);
//! - **Roche**: within which a fluid satellite is torn apart ([`roche_km`],
//!   from [`crate::rings`]).
//!
//! The spheres belong to the body shown (see [`tide_zones`]): a moon's are
//! centred on its planet; a planet's are its most massive moon's, or a
//! typical satellite's when it has none.

use crate::exomoon::EARTH_KG;
use crate::moons::{MOON_INERTIA, despin_gyr};
use crate::planet::heat::{self, ICY_K2_OVER_Q, ROCKY_K2_OVER_Q};
use crate::rings::fluid_roche;
use crate::system::{Body, System};
use serde::Serialize;
use std::collections::BTreeMap;

/// Mass of the Sun, kg: the IAU 2015 nominal solar mass parameter
/// (Resolution B3, 1.327 124 4e20 m³/s²) over the G of `crate::exomoon`
/// (6.674e-11), as `crate::planet` takes it.
const SUN_KG: f64 = 1.988_5e30;

/// Largest eccentricity [`hill_km`] takes: beyond it the periapsis, and
/// with it the sphere, would shrink to nothing. Calibration, no source.
const MAX_HILL_ECCENTRICITY: f64 = 0.95;

/// Least eccentricity that heats a satellite: on a rounder orbit the tide
/// hardly flexes it, and [`heating_km`] gives no sphere. Calibration, no
/// source (the Moon's free eccentricity is 0.055, Io's forced one 0.0041).
const MIN_HEATING_ECCENTRICITY: f64 = 1e-4;

/// Any reference distance, km, the spheres scale from: the times and
/// powers go as powers of `a`, so the radius does not depend on it.
const REFERENCE_KM: f64 = 1e5;

/// Density of the typical satellite of a moonless planet, g/cm³: rock
/// ([`tide_zones`] draws its Roche sphere for it).
const TYPICAL_DENSITY: f64 = 3.0;

/// The Moon, the typical satellite whose locking sphere a moonless planet
/// shows: its mass, kg, mean radius, km, and density, g/cm³ (NASA Moon
/// fact sheet).
const MOON_KG: f64 = 7.342e22;
const MOON_RADIUS_KM: f64 = 1737.4;
const MOON_DENSITY: f64 = 3.344;

/// The tide spheres of a body, centred on its host, km.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct TideZones {
    /// The body the spheres are centred on: the moon's planet, or the planet itself.
    pub host: String,
    /// The body that feels the tide: the moon shown, or a planet's most
    /// massive moon; `None` for a moonless planet's typical satellite.
    pub satellite: Option<String>,
    /// The host's Hill sphere round its own primary ([`hill_km`]).
    pub hill_km: f64,
    /// Within it the host has despun the satellite ([`locking_km`]).
    pub locking_km: Option<f64>,
    /// Within it the satellite's tidal heat exceeds its radiogenic heat
    /// ([`heating_km`]); `None` on a circular orbit.
    pub heating_km: Option<f64>,
    /// Within it a fluid satellite is torn apart ([`roche_km`]).
    pub roche_km: Option<f64>,
}

/// Hill radius, km, of a body of `mass_kg` on an orbit of semi-major axis
/// `a_km` and `eccentricity` round a primary of `primary_kg`, at its
/// periapsis: `a (1 − e) ∛(m / 3M)` (Hamilton & Burns 1992,
/// doi:10.1016/0019-1035(92)90156-S), `e` at most
/// [`MAX_HILL_ECCENTRICITY`]. The viewer drew it so before (the
/// `hillRadiusKm` of the planet view); `crate::rings` takes the circular
/// one. Earth's is 1.5 million km, Jupiter's 50 million.
pub fn hill_km(a_km: f64, eccentricity: f64, mass_kg: f64, primary_kg: f64) -> f64 {
    let e = eccentricity.clamp(0.0, MAX_HILL_ECCENTRICITY);
    a_km * (1.0 - e) * (mass_kg / (3.0 * primary_kg)).cbrt()
}

/// Radius, km, within which a host of `host_kg` has despun a satellite of
/// `mass_kg`, `radius_km` and `density` (g/cm³) by `age_gyr`: the
/// despinning time [`despin_gyr`] goes as `a⁶` (its Love number does not
/// depend on `a`), so it equals the age at `a₀ (age / t(a₀))^(1/6)`, with
/// the moment of inertia [`MOON_INERTIA`] the generator gives moons. The
/// same assumptions (first day of 12 h, `Q = 100`) make it agree with the
/// `locked` flag the generator stores. The Moon's lies beyond its orbit.
pub fn locking_km(mass_kg: f64, radius_km: f64, density: f64, host_kg: f64, age_gyr: f64) -> f64 {
    let t0 = despin_gyr(
        mass_kg,
        radius_km,
        density,
        MOON_INERTIA,
        host_kg,
        REFERENCE_KM,
    );
    REFERENCE_KM * (age_gyr / t0).powf(1.0 / 6.0)
}

/// Radius, km, within which a satellite of `radius_km` on an orbit of
/// `eccentricity` round a host of `host_kg`, with a tidal response
/// `k2_over_q`, makes more tidal heat than its `radiogenic_w`: the tidal
/// power [`heat::tidal_power_w`] (Peale, Cassen & Reynolds 1979) goes as
/// `a^(−15/2)` at a fixed `e`, so it equals the radiogenic power at
/// `a₀ (Ė(a₀) / H)^(2/15)`. Both cross the satellite's surface: comparing
/// powers is comparing fluxes. `None` below
/// [`MIN_HEATING_ECCENTRICITY`]. Io lies well inside Jupiter's.
pub fn heating_km(
    host_kg: f64,
    radius_km: f64,
    eccentricity: f64,
    k2_over_q: f64,
    radiogenic_w: f64,
) -> Option<f64> {
    if eccentricity < MIN_HEATING_ECCENTRICITY {
        return None;
    }
    let a0_m = REFERENCE_KM * 1e3;
    let power = heat::tidal_power_w(host_kg, radius_km * 1e3, a0_m, eccentricity, k2_over_q);
    Some(REFERENCE_KM * (power / radiogenic_w).powf(2.0 / 15.0))
}

/// Fluid Roche limit, km, of a satellite of `density` round a host of
/// `host_radius_km` and `host_density` (g/cm³): [`fluid_roche`]'s
/// `2.44 ∛(ρ_host / ρ)` host radii (Murray & Dermott 1999, §4.7), the
/// limit the generator keeps round moons beyond. Ice round Saturn: about
/// 2.2 Saturn radii.
pub fn roche_km(host_radius_km: f64, host_density: f64, density: f64) -> f64 {
    fluid_roche(host_density, density) * host_radius_km
}

/// The tide spheres of the body `body_id` of `system`; `None` when it is
/// not in the system, or a moon's planet is missing.
///
/// - A **moon**'s are centred on its planet: the planet's Hill sphere round
///   its own primary (its parent if any, else the star), and the moon's
///   locking, heating and Roche spheres round the planet, from the moon's
///   own mass, radius, density, eccentricity and composition, at the
///   system's age.
/// - A **planet with moons** shows those of its most massive moon, which
///   the zones name as `satellite`.
/// - A **moonless planet** shows its own Hill sphere and those of a typical
///   satellite: Roche for rock ([`TYPICAL_DENSITY`]), locking for the Moon,
///   and no heating sphere (no eccentricity).
///
/// A sphere not finite, not larger than the host, or not smaller than the
/// host's Hill sphere is dropped (`None`).
pub fn tide_zones(system: &System, body_id: &str) -> Option<TideZones> {
    let body = find(system, body_id)?;
    match body.parent.as_deref() {
        Some(parent) => satellite_zones(system, body, find(system, parent)?),
        None => {
            let largest = system
                .bodies
                .iter()
                .filter(|b| b.parent.as_deref() == Some(body.id.as_str()))
                .max_by(|a, b| a.physical.mass_earth.total_cmp(&b.physical.mass_earth));
            match largest {
                Some(moon) => satellite_zones(system, moon, body),
                None => Some(typical_zones(system, body)),
            }
        }
    }
}

/// [`tide_zones`] of every body of `system`, by id (the bodies it gives
/// none are left out).
pub fn system_tides(system: &System) -> BTreeMap<String, TideZones> {
    system
        .bodies
        .iter()
        .filter_map(|b| tide_zones(system, &b.id).map(|z| (b.id.clone(), z)))
        .collect()
}

fn find<'a>(system: &'a System, id: &str) -> Option<&'a Body> {
    system.bodies.iter().find(|b| b.id == id)
}

fn age_gyr(system: &System) -> f64 {
    f64::from(system.star.info.age_gyr)
}

/// The Hill sphere of `host` round its primary: its parent if it has one,
/// else the star.
fn host_hill_km(system: &System, host: &Body) -> f64 {
    let primary_kg = host
        .parent
        .as_deref()
        .and_then(|p| find(system, p))
        .map_or(f64::from(system.star.info.mass) * SUN_KG, |p| {
            p.physical.mass_earth * EARTH_KG
        });
    hill_km(
        host.orbit.semi_major_axis_km,
        host.orbit.eccentricity,
        host.physical.mass_earth * EARTH_KG,
        primary_kg,
    )
}

/// `radius`, when it is finite and lies between the host's surface and
/// its Hill sphere `hill`: a satellite cannot orbit beyond the latter.
fn within(host: &Body, hill: f64, radius: f64) -> Option<f64> {
    (radius.is_finite() && radius > host.physical.radius_km && radius < hill).then_some(radius)
}

/// The spheres of `moon` round `host`.
fn satellite_zones(system: &System, moon: &Body, host: &Body) -> Option<TideZones> {
    let age = age_gyr(system);
    let host_kg = host.physical.mass_earth * EARTH_KG;
    let mass_kg = moon.physical.mass_earth * EARTH_KG;
    let p = &moon.physical;
    let (k2_over_q, radiogenic_w) = match &moon.model {
        Some(model) => (
            heat::k2_over_q(model.inputs.envelope_fraction, model.inputs.water_fraction),
            model.heat.radiogenic_w,
        ),
        None => {
            let k2q = if moon.body_type == "icy-moon" {
                ICY_K2_OVER_Q
            } else {
                ROCKY_K2_OVER_Q
            };
            let rock = f64::from(p.bulk.unwrap_or_default().rock);
            (k2q, heat::radiogenic_w(mass_kg * rock, 1.0, age))
        }
    };
    let heating = heating_km(
        host_kg,
        p.radius_km,
        moon.orbit.eccentricity,
        k2_over_q,
        radiogenic_w,
    );
    let hill = host_hill_km(system, host);
    Some(TideZones {
        host: host.id.clone(),
        satellite: Some(moon.id.clone()),
        hill_km: hill,
        locking_km: within(
            host,
            hill,
            locking_km(mass_kg, p.radius_km, p.density, host_kg, age),
        ),
        heating_km: heating.and_then(|r| within(host, hill, r)),
        roche_km: within(
            host,
            hill,
            roche_km(host.physical.radius_km, host.physical.density, p.density),
        ),
    })
}

/// The spheres of a typical satellite round the moonless `planet`.
fn typical_zones(system: &System, planet: &Body) -> TideZones {
    let host_kg = planet.physical.mass_earth * EARTH_KG;
    let hill = host_hill_km(system, planet);
    TideZones {
        host: planet.id.clone(),
        satellite: None,
        hill_km: hill,
        locking_km: within(
            planet,
            hill,
            locking_km(
                MOON_KG,
                MOON_RADIUS_KM,
                MOON_DENSITY,
                host_kg,
                age_gyr(system),
            ),
        ),
        heating_km: None,
        roche_km: within(
            planet,
            hill,
            roche_km(
                planet.physical.radius_km,
                planet.physical.density,
                TYPICAL_DENSITY,
            ),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::{BodyKind, SystemRequest, WholeSystemRequest, draw};
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    const JUPITER_KG: f64 = 1.898e27;

    fn near(value: f64, expected: f64, tolerance: f64) -> bool {
        (value / expected - 1.0).abs() <= tolerance
    }

    #[test]
    fn earths_hill_sphere_is_a_million_and_a_half_km() {
        let r = hill_km(1.496e8, 0.0167, 5.972e24, 1.989e30);
        assert!(near(r, 1.47e6, 0.1), "{r}");
    }

    #[test]
    fn jupiters_hill_sphere_is_fifty_million_km() {
        let r = hill_km(7.785e8, 0.0489, 1.898e27, 1.989e30);
        assert!(near(r, 5.03e7, 0.1), "{r}");
    }

    #[test]
    fn the_moon_and_the_galilean_moons_lie_inside_their_locking_sphere() {
        let moon = locking_km(7.342e22, 1737.4, 3.344, 5.972e24, 4.5);
        assert!(moon > 384_400.0, "Moon: {moon}");
        for (name, mass, radius, density, a) in [
            ("Io", 8.93e22, 1821.6, 3.53, 421_700.0),
            ("Europa", 4.80e22, 1560.8, 3.01, 671_034.0),
            ("Ganymede", 1.482e23, 2634.1, 1.94, 1_070_412.0),
            ("Callisto", 1.076e23, 2410.3, 1.83, 1_882_709.0),
        ] {
            let r = locking_km(mass, radius, density, JUPITER_KG, 4.5);
            assert!(r > a, "{name}: {r} within {a}");
        }
    }

    #[test]
    fn locking_grows_as_the_sixth_root_of_age() {
        let old = locking_km(7.342e22, 1737.4, 3.344, 5.972e24, 4.0);
        let young = locking_km(7.342e22, 1737.4, 3.344, 5.972e24, 4.0 / 64.0);
        assert!((old / young - 2.0).abs() < 1e-9, "{}", old / young);
    }

    #[test]
    fn io_lies_inside_jupiters_heating_sphere() {
        let rad = heat::radiogenic_w(8.93e22, 1.0, 4.5);
        let r = heating_km(JUPITER_KG, 1821.6, 0.0041, ROCKY_K2_OVER_Q, rad);
        assert!(r.is_some_and(|r| r > 421_700.0), "{r:?}");
    }

    #[test]
    fn a_circular_orbit_has_no_heating_sphere() {
        let rad = heat::radiogenic_w(8.93e22, 1.0, 4.5);
        assert_eq!(
            heating_km(JUPITER_KG, 1821.6, 0.0, ROCKY_K2_OVER_Q, rad),
            None
        );
        assert_eq!(
            heating_km(JUPITER_KG, 1821.6, 5e-5, ROCKY_K2_OVER_Q, rad),
            None
        );
    }

    #[test]
    fn saturns_ice_roche_limit_is_about_two_saturn_radii() {
        let radii = roche_km(60_268.0, 0.687, 0.93) / 60_268.0;
        assert!((2.0..2.5).contains(&radii), "{radii}");
    }

    fn whole(planets: &[(&str, u32)], seed: u32) -> SystemRequest {
        SystemRequest::System(WholeSystemRequest {
            age_gyr: None,
            star: Some("G2V".into()),
            metallicity: Some(0.0),
            planets: Some(
                planets
                    .iter()
                    .map(|(t, n)| ((*t).to_string(), *n))
                    .collect(),
            ),
            seed: Some(seed),
            width: Some(64),
            height: Some(32),
        })
    }

    fn drawn(seeds: std::ops::RangeInclusive<u32>) -> Vec<System> {
        seeds
            .map(|seed| {
                let request = whole(&[("earth-like", 1), ("jupiter-like", 1)], seed);
                draw(&request, &mut StdRng::seed_from_u64(u64::from(seed))).unwrap()
            })
            .collect()
    }

    fn moons_of<'a>(system: &'a System, id: &'a str) -> impl Iterator<Item = &'a Body> {
        system
            .bodies
            .iter()
            .filter(move |b| b.parent.as_deref() == Some(id))
    }

    #[test]
    fn drawn_systems_give_finite_nested_zones() {
        for system in drawn(1..=4) {
            for body in &system.bodies {
                let zones = tide_zones(&system, &body.id)
                    .unwrap_or_else(|| panic!("{}: no zones", body.id));
                let host = find(&system, &zones.host).unwrap();
                let r = host.physical.radius_km;
                assert!(zones.hill_km.is_finite() && zones.hill_km > r, "{zones:?}");
                for x in [zones.locking_km, zones.heating_km, zones.roche_km]
                    .into_iter()
                    .flatten()
                {
                    assert!(x.is_finite() && x > r && x < zones.hill_km, "{zones:?}");
                }
                if let (Some(lock), Some(roche)) = (zones.locking_km, zones.roche_km) {
                    assert!(roche < lock, "{zones:?}");
                }
                match &body.parent {
                    Some(parent) => {
                        assert_eq!(&zones.host, parent);
                        assert_eq!(zones.satellite.as_deref(), Some(body.id.as_str()));
                    }
                    None => {
                        assert_eq!(zones.host, body.id);
                        let largest = moons_of(&system, &body.id)
                            .max_by(|a, b| a.physical.mass_earth.total_cmp(&b.physical.mass_earth));
                        match largest {
                            Some(moon) => {
                                assert_eq!(zones.satellite.as_deref(), Some(moon.id.as_str()));
                                assert_eq!(Some(&zones), tide_zones(&system, &moon.id).as_ref());
                            }
                            None => {
                                assert_eq!(zones.satellite, None);
                                assert_eq!(zones.heating_km, None);
                            }
                        }
                    }
                }
            }
            assert_eq!(system_tides(&system).len(), system.bodies.len());
        }
    }

    #[test]
    fn spheres_inside_the_host_are_dropped() {
        let system = drawn(1..=1).remove(0);
        let moon = system
            .bodies
            .iter()
            .find(|b| b.kind == BodyKind::Moon)
            .expect("a moon");
        let mut swollen = system.clone();
        let parent = moon.parent.clone().unwrap();
        for b in &mut swollen.bodies {
            if b.id == parent {
                // Thin as well as huge: the Roche sphere grows with the
                // host's radius, and lies inside it only round a host far
                // less dense than its satellite.
                b.physical.radius_km = 1e9;
                b.physical.density = 1e-6;
            }
        }
        let zones = tide_zones(&swollen, &moon.id).unwrap();
        assert_eq!(
            (zones.locking_km, zones.heating_km, zones.roche_km),
            (None, None, None),
            "{zones:?}"
        );
    }

    #[test]
    fn spheres_beyond_the_hill_sphere_are_dropped() {
        let system = drawn(1..=1).remove(0);
        let host = &system.bodies[0];
        let r = host.physical.radius_km;
        // A Moon-like satellite's locking radius against a close-in
        // planet's Hill sphere.
        assert_eq!(within(host, 1.28e5, 9.2e5), None);
        assert_eq!(within(host, 1.28e5, 1.28e5), None);
        assert_eq!(within(host, 1.28e5, 1.0e5), Some(1.0e5));
        assert_eq!(within(host, 1.28e5, r * 0.5), None);
    }

    #[test]
    fn stored_locking_agrees_with_the_locking_sphere() {
        let (mut locked, mut unlocked) = (0, 0);
        // Disk-born moons are large and close: every one drawn in seeds 1
        // to 12 is locked. The unlocked moons are the small captured ones
        // of the rocky planets (seed 11), so every unlocked moon is checked.
        for system in drawn(1..=12) {
            for moon in system.bodies.iter().filter(|b| b.kind == BodyKind::Moon) {
                let zones = tide_zones(&system, &moon.id).unwrap();
                let a = moon.orbit.semi_major_axis_km;
                // A sphere dropped inside the host lies below any orbit.
                let lock = zones.locking_km.unwrap_or(0.0);
                if moon.spin.locked {
                    locked += 1;
                    assert!(lock >= a * 0.99, "{}: {lock} below {a}", moon.id);
                } else {
                    unlocked += 1;
                    assert!(lock < a, "{}: {lock} beyond {a}", moon.id);
                }
            }
        }
        assert!(
            locked > 0 && unlocked > 0,
            "{locked} locked, {unlocked} unlocked"
        );
    }
}
