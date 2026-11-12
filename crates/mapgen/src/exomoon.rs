//! Rocky worlds orbiting a giant planet (backlog item 14, step C;
//! `docs/analysis/12-gas-giants.md`): the world is the moon of a giant
//! that orbits the star where the world would.
//!
//! **How often.** Moons that form around a giant hold about 10⁻⁴ of its
//! mass (Canup & Ward 2006): a Jupiter's largest are Mercury- to Mars-sized.
//! An Earth-sized moon needs a capture (as Triton was captured) or a
//! super-Jupiter, so a drawn world is a giant's moon rarely
//! ([`DRAWN_ODDS`]); a request can ask for one.
//!
//! **The giant.** Its mass is drawn log-uniformly from 0.3 to 10 Jupiter
//! masses, its kind and look by the giant world pipeline (it is generated
//! as a body of the system, to be seen in the world's sky). The world orbits it between
//! 6 and 40 of its radii, within a third of its Hill radius, on a nearly
//! circular orbit (kept slightly eccentric by resonances with other
//! moons, as Io is by Europa and Ganymede), locked to it: its day is its
//! orbit, the giant fixed over one hemisphere.
//!
//! **Tidal heating.** `Ė = (21/2)(k₂/Q) G M² R⁵ n e² / a⁶` (Peale et al.
//! 1979, [`crate::planet::heat::tidal_power_w`]), with Io's `k₂/Q` of
//! 0.015: Io gets about 2 W/m². Spread over the world it is an internal
//! heat, added to the climate as radiated power (`T⁴` adds up) and
//! raising the volcanism; close in, it drives a runaway greenhouse:
//! Heller & Barnes (2013)'s habitable edge, about 10 giant radii for
//! typical cases. A world of liquid water is kept beyond it (its orbit
//! moved out until starlight and tides stay under the limit); a dry or
//! volcanic one may be kneaded like Io.
use crate::giant_world::GiantInfo;
use crate::giants::{GiantKind, JUPITER_EARTH_MASSES, giant_radius};
use crate::noise::{stream, stream_seed};
use crate::planet::heat::tidal_power_w;
use crate::star::StarInfo;
use crate::world::WorldParams;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// Share of drawn rocky worlds that are the moon of a giant.
pub const DRAWN_ODDS: f32 = 0.02;
/// Gravitational constant, SI.
pub(crate) const G: f64 = 6.674e-11;
/// Earth's mass (kg) and radius (m); Jupiter's radius (m).
pub(crate) const EARTH_KG: f64 = 5.972e24;
pub(crate) const EARTH_M: f64 = 6.371e6;
/// Tidal response of a rocky moon, `k₂/Q` (Io's, Lainey et al. 2009).
const K2_OVER_Q: f64 = 0.015;
/// Stefan–Boltzmann constant.
const SIGMA: f64 = 5.670e-8;
/// Absorbed flux above which water runs away into a steam atmosphere,
/// W/m² (about 300 for an Earth-like world, Kopparapu et al. 2013).
const RUNAWAY_W_M2: f64 = 300.0;

/// Whether a world is the moon of a giant: as its seed draws (rarely), always, or never.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "lowercase")]
pub enum HostMode {
    #[default]
    Auto,
    None,
    Giant,
}

impl std::str::FromStr for HostMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "auto" => Ok(Self::Auto),
            "none" => Ok(Self::None),
            "giant" => Ok(Self::Giant),
            other => Err(format!("unknown host mode {other:?} (auto, none or giant)")),
        }
    }
}

/// The giant a world orbits, as `world.json` stores it (under `host`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct HostGiant {
    /// The giant itself, as a body of its own in the system.
    pub giant: GiantInfo,
    /// Seed of the giant's world.
    pub seed: u32,
    /// The world's orbit around it: semi-major axis in km and in giant
    /// radii, eccentricity, period (the world's day, as it is locked), in
    /// Earth days.
    pub distance_km: f32,
    pub distance_radii: f32,
    pub eccentricity: f32,
    pub period_days: f32,
    /// Apparent diameter of the giant in the world's sky, degrees.
    pub angular_diameter_deg: f32,
    /// Tidal heating of the world, W/m², and the temperature it would keep
    /// the surface at alone, K.
    pub tidal_flux: f32,
    pub tidal_heat_k: f32,
    /// Whether the world lies inside the habitable edge: starlight and
    /// tidal heat together run away into a steam atmosphere.
    pub inside_edge: bool,
    /// Hill radius of the giant, in its radii: the world stays within a third.
    pub hill_radii: f32,
    /// The giant's other regular moons, nearest first (see [`Sibling`]);
    #[serde(default)]
    pub siblings: Vec<Sibling>,
}

/// One of the giant's other regular moons, born in its disk as the
/// Galilean moons were: their masses share about 10⁻⁴ of the giant's
/// (Canup & Ward 2006), their orbits spaced like Io, Europa and Ganymede
/// (each period twice the last, semi-major axes `2^(2/3)` apart).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Sibling {
    pub radius_km: f32,
    /// Semi-major axis around the giant, km and giant radii.
    pub distance_km: f32,
    pub distance_radii: f32,
    /// Orbital period around the giant, Earth days.
    pub period_days: f32,
    /// Rock (inner, warm) or ice and rock (beyond the giant's snow line).
    pub icy: bool,
}

/// Share of the giant's mass its regular moons hold together (Canup &
/// Ward 2006).
pub(crate) const SATELLITE_MASS_SHARE: f64 = 1e-4;
/// Ratio of the semi-major axes of neighbouring regular moons (periods in
/// a 1:2 chain, as Io, Europa and Ganymede).
pub(crate) const SIBLING_SPACING: f64 = 1.587_401;

impl HostGiant {
    /// The giant the world `p` orbits, or `None` when it orbits its star.
    pub fn draw(p: &WorldParams) -> Option<Self> {
        if p.giant_mass_earth.is_some() || p.shape.is_some() {
            return None;
        }
        let mut rng = StdRng::seed_from_u64(u64::from(stream_seed(p.seed, stream::HOST)));
        let roll: f32 = rng.random();
        match p.host {
            HostMode::None => return None,
            HostMode::Auto if roll >= DRAWN_ODDS => return None,
            _ => {}
        }
        let mass_earth = (0.3f64.ln() + rng.random::<f64>() * (10.0f64 / 0.3).ln()).exp() as f32
            * JUPITER_EARTH_MASSES;
        let star = StarInfo::new(p.star, p.orbit_au, p.eccentricity, p.age_gyr);
        let radius_earth = giant_radius(mass_earth, star.equilibrium_k);
        let radius_m = f64::from(radius_earth) * EARTH_M;
        let mass_kg = f64::from(mass_earth) * EARTH_KG;
        // Hill radius at the giant's periapsis, in its radii.
        let hill = f64::from(p.orbit_au)
            * (1.0 - f64::from(p.eccentricity))
            * 1.496e11
            * (mass_kg / (3.0 * f64::from(star.mass) * 1.989e30)).cbrt()
            / radius_m;
        let widest = (hill / 3.0).clamp(4.0, 40.0);
        let closest = 6.0f64.min(widest);
        let mut distance_radii =
            (closest.ln() + rng.random::<f64>() * (widest / closest).ln()).exp();
        let eccentricity = 0.001 + 0.011 * rng.random::<f64>().powi(2);
        let seed = rng.random::<u32>();
        // The giant's other regular moons, drawn after the values above so
        // that they keep their draws: two to four orbits of the resonant
        // chain around the world's, inside a third of the Hill radius and
        // outside 3 radii (the Roche limit).
        let count = 2 + (rng.random::<f64>() * 3.0) as usize;
        let shares: Vec<f64> = (0..count).map(|_| 0.3 + rng.random::<f64>()).collect();
        let offset = (rng.random::<f64>() * 3.0) as i32;

        let world_radius = f64::from(p.circumference_km) * 1000.0 / (2.0 * PI);
        // Tidal heating at `radii` giant radii, W/m².
        let flux_at = |radii: f64| {
            tidal_power_w(
                mass_kg,
                world_radius,
                radii * radius_m,
                eccentricity,
                K2_OVER_Q,
            ) / (4.0 * PI * world_radius * world_radius)
        };
        // Absorbed starlight (Earth's albedo), to weigh against the runaway
        // limit.
        let starlight = 0.25 * 0.7 * 1361.0 * f64::from(star.insolation);
        // A world of liquid water orbits beyond the habitable edge (the
        // others, volcanic or dry, may be kneaded like Io): the heating
        // falls as `a^−7.5`, so move it out until it stays under the limit.
        let watery = p.pressure_bar >= crate::atmosphere::LIQUID_WATER_BAR
            && p.planet.remap(crate::biome::Biome::Ocean) == crate::biome::Biome::Ocean;
        let allowed = (0.8 * RUNAWAY_W_M2 - starlight).max(0.05);
        if watery && flux_at(distance_radii) > allowed {
            distance_radii = (distance_radii * (flux_at(distance_radii) / allowed).powf(1.0 / 7.5))
                .min(widest.max(closest));
        }
        let flux = flux_at(distance_radii);
        let a = distance_radii * radius_m;
        let n = (G * mass_kg / a.powi(3)).sqrt();
        let giant =
            crate::giant_world::Look::draw(&host_params(p, mass_earth, seed), mass_earth).info;
        // Slots of the chain, the world's at 0, from inside out; the
        // siblings fill the others the offset leaves, as far as they fit.
        let total: f64 = shares.iter().sum();
        let mut siblings = Vec::new();
        let mut slot = -offset;
        for share in &shares {
            if slot == 0 {
                slot += 1;
            }
            let radii = distance_radii * SIBLING_SPACING.powi(slot);
            slot += 1;
            if radii < 3.0 || radii > hill / 3.0 {
                continue;
            }
            // Icy beyond the giant's snow line, as Ganymede and Callisto.
            let icy = star.equilibrium_k < 150.0 || radii > 12.0;
            let density = if icy { 1.9e3 } else { 3.3e3 };
            let mass = SATELLITE_MASS_SHARE * mass_kg * share / total;
            let radius = (3.0 * mass / (4.0 * PI * density)).cbrt();
            let a_m = radii * radius_m;
            siblings.push(Sibling {
                radius_km: (radius / 1000.0) as f32,
                distance_km: (a_m / 1000.0) as f32,
                distance_radii: radii as f32,
                period_days: (2.0 * PI * (a_m.powi(3) / (G * mass_kg)).sqrt() / 86_400.0) as f32,
                icy,
            });
        }
        siblings.sort_by(|a, b| a.distance_km.total_cmp(&b.distance_km));
        Some(HostGiant {
            giant,
            seed,
            distance_km: (a / 1000.0) as f32,
            distance_radii: distance_radii as f32,
            eccentricity: eccentricity as f32,
            period_days: (2.0 * PI / n / 86_400.0) as f32,
            angular_diameter_deg: (2.0 * (radius_m / (a - world_radius)).atan()).to_degrees()
                as f32,
            tidal_flux: flux as f32,
            tidal_heat_k: (flux / SIGMA).powf(0.25) as f32,
            inside_edge: starlight + flux > RUNAWAY_W_M2,
            hill_radii: hill as f32,
            siblings,
        })
    }

    /// Volcanism the tides raise: none below 0.05 W/m², Io's 2 W/m² a
    /// little over half, all at 50 W/m².
    pub fn volcanism(&self) -> f32 {
        ((f64::from(self.tidal_flux) / 0.05).log10() / 3.0).clamp(0.0, 1.0) as f32
    }
}

/// The parameters of sibling `k` (from 0, nearest first) of the world `p`
/// orbiting `host`, as a world of its own (a moon of the giant in the
/// world's system, see `crate::system`): a
/// rocky or icy moon of its size, seeded from the giant's seed, on the
/// world's scale.
pub fn sibling_params(p: &WorldParams, host: &HostGiant, k: usize) -> WorldParams {
    let sibling = &host.siblings[k];
    let composition = if sibling.icy {
        crate::moons::Composition::Ice
    } else {
        crate::moons::Composition::Rock
    };
    let seed = stream_seed(host.seed, stream::SIBLINGS + k as u32);
    crate::moons::small_body_params(p, composition, sibling.radius_km, seed, None)
}

/// The parameters of the giant a world orbits, as a giant world of its own
/// (its kind from its mass: an ice giant below 50 Earth masses).
pub fn host_params(p: &WorldParams, mass_earth: f32, seed: u32) -> WorldParams {
    let kind = if mass_earth < 50.0 {
        "ice-giant"
    } else {
        "gas-giant"
    };
    let mut planet = crate::preset::Preset::load(kind).expect("giant presets are built in");
    if let Some(giant) = planet.giant.as_mut() {
        giant.kind = if mass_earth < 50.0 {
            GiantKind::Ice
        } else {
            GiantKind::Gas
        };
    }
    let star = StarInfo::new(p.star, p.orbit_au, p.eccentricity, p.age_gyr);
    let width = 512;
    WorldParams {
        width,
        height: width / 2,
        seed,
        planet,
        sea_level: 0.0,
        volcanic_intensity: 0.0,
        circumference_km: giant_radius(mass_earth, star.equilibrium_k) * 40_075.0,
        axial_tilt_deg: p.axial_tilt_deg,
        erosion: 0.0,
        pressure_bar: 0.3,
        surface_age_ga: 0.0,
        star: p.star,
        orbit_au: p.orbit_au,
        eccentricity: p.eccentricity,
        age_gyr: p.age_gyr,
        drift_ma: 0.0,
        moons: crate::moons::MoonsMode::None,
        shape: None,
        metallicity: p.metallicity,
        giants: crate::giants::GiantsMode::None,
        giant_mass_earth: Some(mass_earth),
        host: HostMode::None,
        given_rings: None,
        impact_factor: None,
        locked_to_star: None,
        ridge: None,
        perihelion_longitude_deg: None,
        resonance_to_star: None,
        gravity: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preset::Preset;

    fn params(seed: u32, host: HostMode, orbit_au: f32) -> WorldParams {
        WorldParams {
            age_gyr: crate::star::DEFAULT_AGE_GYR,
            width: 64,
            height: 32,
            seed,
            planet: Preset::load("terran").unwrap(),
            sea_level: 0.0,
            volcanic_intensity: 0.0,
            circumference_km: 40_075.0,
            axial_tilt_deg: 23.44,
            erosion: 0.0,
            pressure_bar: 1.0,
            surface_age_ga: 4.0,
            star: crate::star::SUN,
            orbit_au,
            eccentricity: 0.0,
            drift_ma: 0.0,
            moons: crate::moons::MoonsMode::None,
            shape: None,
            metallicity: 0.0,
            giants: crate::giants::GiantsMode::None,
            giant_mass_earth: None,
            host,
            given_rings: None,
            impact_factor: None,
            locked_to_star: None,
            ridge: None,
            perihelion_longitude_deg: None,
            resonance_to_star: None,
            gravity: None,
        }
    }

    #[test]
    fn a_world_is_rarely_a_giants_moon_unless_asked() {
        let drawn = (0..5000)
            .filter(|&s| HostGiant::draw(&params(s, HostMode::Auto, 1.0)).is_some())
            .count();
        assert!((60..140).contains(&drawn), "{drawn}");
        assert!((0..50).all(|s| HostGiant::draw(&params(s, HostMode::Giant, 1.0)).is_some()));
        assert!((0..50).all(|s| HostGiant::draw(&params(s, HostMode::None, 1.0)).is_none()));
    }

    #[test]
    fn orbits_stay_within_the_hill_sphere_and_tides_heat_closer_moons_more() {
        for s in 0..200 {
            let host = HostGiant::draw(&params(s, HostMode::Giant, 1.0)).unwrap();
            assert!(host.distance_radii <= host.hill_radii / 3.0 + 1e-3);
            assert!((4.0..=40.0).contains(&host.distance_radii));
            assert!(
                host.period_days > 0.3 && host.period_days < 60.0,
                "{}",
                host.period_days
            );
            // A world of liquid water stays beyond the habitable edge.
            assert!(!host.inside_edge, "{host:?}");
        }
        // A dry, volcanic world may be kneaded like Io.
        let hot = (0..400)
            .filter_map(|s| {
                let mut p = params(s, HostMode::Giant, 1.0);
                p.planet = Preset::load("volcanic").unwrap();
                HostGiant::draw(&p)
            })
            .filter(|h| h.tidal_flux > 2.0)
            .count();
        assert!(hot > 10, "{hot}");
        // Io's numbers: Jupiter's mass, 5.9 radii, e = 0.0041, Io's size.
        let io_flux = {
            let (m, r, a, e) = (1.898e27f64, 1.8216e6f64, 4.217e8f64, 0.0041f64);
            tidal_power_w(m, r, a, e, K2_OVER_Q) / (4.0 * PI * r * r)
        };
        assert!((1.0..4.0).contains(&io_flux), "{io_flux}");
    }

    #[test]
    fn each_sibling_is_a_world_of_its_own_size_and_seed() {
        let (p, host) = (0..400)
            .map(|s| params(s, HostMode::Giant, 1.0))
            .find_map(|p| {
                let host = HostGiant::draw(&p)?;
                (host.siblings.len() >= 2).then_some((p, host))
            })
            .expect("a giant with two siblings");
        let bodies: Vec<_> = (0..host.siblings.len())
            .map(|k| sibling_params(&p, &host, k))
            .collect();
        for (body, sibling) in bodies.iter().zip(&host.siblings) {
            let radius = f64::from(body.circumference_km) / (2.0 * std::f64::consts::PI);
            assert!((radius - f64::from(sibling.radius_km)).abs() < 1.0);
            assert_eq!(body.pressure_bar, 0.0);
            assert_eq!(body.height * 2, body.width);
            let preset = if sibling.icy {
                "icy-moon"
            } else {
                "rocky-moon"
            };
            assert_eq!(body.planet.name, preset);
        }
        assert_ne!(bodies[0].seed, bodies[1].seed);
        assert_ne!(bodies[0].seed, host.seed);
        // The same world draws the same siblings.
        assert_eq!(sibling_params(&p, &host, 0).seed, bodies[0].seed);
    }

    #[test]
    fn the_giant_has_other_regular_moons_in_a_resonant_chain() {
        let mut with = 0;
        for s in 0..200 {
            let host = HostGiant::draw(&params(s, HostMode::Giant, 1.0)).unwrap();
            with += usize::from(!host.siblings.is_empty());
            assert!(host.siblings.len() <= 4);
            let giant_kg = f64::from(host.giant.mass_earth) * EARTH_KG;
            let mut mass = 0.0;
            for sib in &host.siblings {
                // Inside the stable zone, apart from the world's own orbit.
                assert!(sib.distance_radii >= 3.0 && sib.distance_radii <= host.hill_radii / 3.0);
                let ratio = f64::from(sib.distance_radii / host.distance_radii);
                assert!((ratio.ln() / SIBLING_SPACING.ln()).round().abs() >= 1.0);
                let density = if sib.icy { 1.9e3 } else { 3.3e3 };
                mass += 4.0 / 3.0 * PI * (f64::from(sib.radius_km) * 1000.0).powi(3) * density;
            }
            assert!(mass <= 1.01 * SATELLITE_MASS_SHARE * giant_kg);
        }
        assert!(with > 150, "{with}");
        // A Jupiter's: moons of the size of the Galilean ones.
        let host = (0..400)
            .filter_map(|s| HostGiant::draw(&params(s, HostMode::Giant, 5.2)))
            .find(|h| (0.8..1.3).contains(&(h.giant.mass_earth / 318.0)) && !h.siblings.is_empty())
            .unwrap();
        assert!(
            host.siblings
                .iter()
                .all(|s| (500.0..4000.0).contains(&s.radius_km)),
            "{:?}",
            host.siblings
        );
    }
}
