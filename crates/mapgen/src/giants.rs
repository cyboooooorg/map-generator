//! The giant planets of a world's system, drawn from its seed (backlog
//! item 14, step A; `docs/analysis/12-gas-giants.md`).
//!
//! **How many.** A Poisson number of gas giants (0.1 to 13 Jupiter masses)
//! and of ice giants (10 to 50 Earth masses). A Sun-like star of the Sun's
//! metallicity has 0.25 gas giants and 0.3 ice giants on average (Fulton
//! et al. 2021 count about 0.23 giants per star between 2 and 32 AU); gas
//! giants grow a hundredfold per dex of metallicity squared
//! (`10^(2 [Fe/H])`, Fischer & Valenti 2005) and with the star's mass
//! (Johnson et al. 2010), ice giants more weakly. About 1 % of Sun-like
//! stars have a hot Jupiter (Wright et al. 2012), five times fewer when the
//! world orbits close to its star: hot Jupiters are lonely (Steffen et al.
//! 2012).
//!
//! **Where.** Gas giants peak beyond the ice line, `2.7 √L` AU (Hayashi
//! 1981; Fulton et al. 2021): their distances are log-normal around twice
//! it, ice giants around five times it. Masses follow `dN/d ln M ∝ M^−0.31`
//! (Cumming et al. 2008) for gas giants, log-uniform for ice giants;
//! eccentricities Kipping's law, like the world's, none for hot Jupiters.
//!
//! **Spacing.** Two orbits closer than `2√3` mutual Hill radii, apoapsis to
//! periapsis, are unstable (Gladman 1993): a giant that close to the world
//! or to another giant is redrawn a few times, then dropped. At most
//! [`MAX_GIANTS`].
//!
//! **Effects on the world.** An eccentric giant forces an eccentricity on
//! the world (Laplace–Lagrange secular theory; the request raises a drawn
//! eccentricity to it), and the giant outside the world's orbit sets how
//! many impactors reach it (Horner & Jones 2008; see [`impact_factor`]).
use crate::noise::{smoothstep, stream, stream_seed};
use crate::star::{
    StarValues, draw_eccentricity, equilibrium_k, mean_flux_factor, values, year_days,
};
use crate::world::WorldParams;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use serde::{Deserialize, Serialize};

/// Most giants a system keeps.
pub const MAX_GIANTS: usize = 4;
/// Earth masses in a Jupiter mass, and in a solar mass.
pub const JUPITER_EARTH_MASSES: f32 = 317.8;
const SUN_EARTH_MASSES: f64 = 332_946.0;
/// Mean number of gas and ice giants around a Sun-like star of the Sun's
/// metallicity.
const GAS_GIANTS: f64 = 0.25;
const ICE_GIANTS: f64 = 0.3;
/// Share of Sun-like stars of the Sun's metallicity with a hot Jupiter.
const HOT_JUPITERS: f64 = 0.01;
/// Gas giant masses, Jupiter masses: from 0.1 (about a third of Saturn) to
/// the deuterium-burning limit, and the slope of `dN/d ln M`.
const GAS_MASS: (f64, f64) = (0.1, 13.0);
const GAS_MASS_SLOPE: f64 = -0.31;
/// Ice giant masses, Earth masses.
const ICE_MASS: (f64, f64) = (10.0, 50.0);
/// Distances, as `log10` of a multiple of the ice line: mean and spread.
const GAS_ORBIT: (f64, f64) = (0.3, 0.4);
const ICE_ORBIT: (f64, f64) = (0.7, 0.3);
/// Distances kept, AU.
const ORBIT_RANGE: (f64, f64) = (0.02, 60.0);
/// Hot Jupiter periods, days.
const HOT_PERIOD_DAYS: (f64, f64) = (1.0, 10.0);
/// Least gap between two orbits, in mutual Hill radii (Gladman 1993).
const HILL_GAP: f64 = 3.464;
/// Tries to place a giant before it is dropped.
const TRIES: usize = 6;
/// Metallicity of the stars nearby, dex: mean, spread and limits
/// (Casagrande et al. 2011).
pub const METALLICITY_MEAN: f32 = -0.1;
pub const METALLICITY_SPREAD: f32 = 0.2;
pub const METALLICITY: (f32, f32) = (-1.0, 0.5);
/// Largest forced eccentricity applied to a world.
const MAX_FORCED: f64 = 0.5;

/// Whether a world has the giant planets its seed draws, or none (giants
/// not modelled: the impact rate stays the Solar System's).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "lowercase")]
pub enum GiantsMode {
    #[default]
    Auto,
    None,
}

impl std::str::FromStr for GiantsMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "auto" => Ok(Self::Auto),
            "none" => Ok(Self::None),
            other => Err(format!("unknown giants mode {other:?} (auto or none)")),
        }
    }
}

/// What a giant is mostly made of.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "lowercase")]
pub enum GiantKind {
    /// Hydrogen and helium (Jupiter, Saturn).
    Gas,
    /// Water, ammonia and methane under a thin envelope (Uranus, Neptune).
    Ice,
}

/// The rings of a giant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "lowercase")]
pub enum Rings {
    None,
    /// Narrow or dusty, hard to see (Jupiter, Uranus, Neptune).
    Faint,
    /// Broad and icy (Saturn).
    Bright,
}

/// One giant planet of the system, as `world.json` stores it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Giant {
    /// Its letter in the system, by distance from the star (`b`, `c`, …).
    pub name: String,
    pub kind: GiantKind,
    /// Mass and radius, Earth units.
    pub mass_earth: f32,
    pub radius_earth: f32,
    /// Semi-major axis, AU, and eccentricity of its orbit.
    pub orbit_au: f32,
    pub eccentricity: f32,
    /// Orientation of its periapsis and its place on its orbit when the
    /// world is at its own starting point, radians: only a look.
    pub periapsis_angle: f32,
    pub phase: f32,
    /// Orbital period, Earth days.
    pub year_days: f32,
    /// Equilibrium temperature (Earth's albedo) and effective temperature
    /// with its own internal heat, K.
    pub equilibrium_k: f32,
    pub effective_k: f32,
    /// Cloud class of Sudarsky, Burrows & Pinto (2000), 1 to 5, from the
    /// effective temperature: ammonia clouds, water clouds, clear, alkali
    /// "roasters", silicate clouds.
    pub class: u8,
    /// A hot giant (`Teq` over 1 000 K): inflated, dark, locked.
    pub hot: bool,
    /// Colour of its disk, sRGB in `[0, 1]`.
    pub colour: [f32; 3],
    pub rings: Rings,
    /// Seed of its own look (its bands and storms, for its own view).
    pub seed: u32,
}

/// The giants of a world's system and what they do to the world, as
/// `world.json` stores them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct GiantSystem {
    /// Giants by distance from the star.
    pub giants: Vec<Giant>,
    /// Eccentricity the giants force on the world's orbit (secular theory):
    /// a drawn eccentricity is at least this.
    pub forced_eccentricity: f32,
    /// Impact rate on the world relative to the Solar System's, from the
    /// giant outside its orbit (see [`impact_factor`]).
    pub impact_factor: f32,
}

/// A metallicity drawn like those of the stars nearby, from its own random
/// stream (the other drawn values of a seed do not change).
pub fn draw_metallicity(seed: u32) -> f32 {
    let mut rng = StdRng::seed_from_u64(u64::from(stream_seed(seed, stream::METALLICITY)));
    (METALLICITY_MEAN + METALLICITY_SPREAD * normal(&mut rng) as f32)
        .clamp(METALLICITY.0, METALLICITY.1)
}

/// A standard normal variate (Box–Muller).
fn normal(rng: &mut impl RngExt) -> f64 {
    let (u1, u2): (f64, f64) = (rng.random::<f64>().max(1e-12), rng.random());
    (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
}

/// A Poisson variate of mean `mean` (Knuth; means here are small).
fn poisson(mean: f64, rng: &mut impl RngExt) -> usize {
    let limit = (-mean).exp();
    let (mut k, mut p) = (0, rng.random::<f64>());
    while p > limit && k < 16 {
        k += 1;
        p *= rng.random::<f64>();
    }
    k
}

/// Mass of a world in Earth masses: Earth's density, from its circumference;
/// for the moon of a giant, its giant's (they share the orbit).
fn world_mass_earth(p: &WorldParams) -> f64 {
    match crate::exomoon::HostGiant::draw(p) {
        Some(host) => f64::from(host.giant.mass_earth),
        None => (f64::from(p.circumference_km) / 40_075.0).powi(3),
    }
}

/// Radius of a giant, Earth radii, from its mass (Earth masses): through
/// Neptune (`M^0.59` below it, Chen & Kipping 2017), Saturn and Jupiter,
/// then `M^−0.04`; inflated when hot (Thorngren & Fortney 2018), the less
/// massive the more.
pub fn giant_radius(mass_earth: f32, equilibrium_k: f32) -> f32 {
    const ANCHORS: [(f32, f32); 3] = [(17.15, 3.88), (95.16, 9.45), (317.8, 11.21)];
    let m = mass_earth;
    let cold = if m <= ANCHORS[0].0 {
        ANCHORS[0].1 * (m / ANCHORS[0].0).powf(0.59)
    } else if m >= ANCHORS[2].0 {
        ANCHORS[2].1 * (m / ANCHORS[2].0).powf(-0.04)
    } else {
        let i = usize::from(m > ANCHORS[1].0);
        let ((m0, r0), (m1, r1)) = (ANCHORS[i], ANCHORS[i + 1]);
        let f = (m / m0).ln() / (m1 / m0).ln();
        (r0.ln() + (r1.ln() - r0.ln()) * f).exp()
    };
    let heat = smoothstep(1000.0, 2200.0, equilibrium_k);
    let inflation = (1.0 + 0.7 * heat * (JUPITER_EARTH_MASSES / m.max(30.0)).powf(0.25)).min(1.9);
    cold * if m > 30.0 { inflation } else { 1.0 }
}

/// Sudarsky class (1 to 5) of a giant of effective temperature `teff`.
pub fn cloud_class(teff: f32) -> u8 {
    match teff {
        t if t < 150.0 => 1,
        t if t < 350.0 => 2,
        t if t < 900.0 => 3,
        t if t < 1500.0 => 4,
        _ => 5,
    }
}

/// Colour of a giant's disk from its kind and class, varied by `t` in
/// `[0, 1]`: cream to ochre Jovians, cyan to blue cold ice giants, white
/// water clouds, azure clear skies, near-black roasters, dark grey silicate
/// clouds.
pub fn giant_colour(kind: GiantKind, class: u8, t: f32) -> [f32; 3] {
    let mix = |a: [f32; 3], b: [f32; 3]| [0, 1, 2].map(|k| a[k] + (b[k] - a[k]) * t);
    match (kind, class) {
        (GiantKind::Ice, 1) => mix([0.66, 0.85, 0.88], [0.32, 0.5, 0.88]),
        (_, 1) => mix([0.86, 0.79, 0.66], [0.76, 0.6, 0.44]),
        (_, 2) => mix([0.94, 0.94, 0.91], [0.86, 0.88, 0.9]),
        (_, 3) => mix([0.42, 0.6, 0.86], [0.3, 0.48, 0.82]),
        (_, 4) => mix([0.2, 0.22, 0.3], [0.16, 0.16, 0.2]),
        _ => mix([0.34, 0.32, 0.29], [0.28, 0.25, 0.22]),
    }
}

/// Impact rate on a rocky planet relative to the Solar System's, from the
/// mass (Jupiter masses) of the giant just outside its orbit, after the
/// N-body runs of Horner & Jones (2008), moderated: low without a giant,
/// highest for about a fifth of Jupiter's mass, Jupiter's own the
/// calibration, lower for super-Jupiters.
pub fn impact_factor(giant_mass_jupiter: Option<f32>) -> f32 {
    const CURVE: [(f32, f32); 7] = [
        (0.01, 0.8),
        (0.05, 1.2),
        (0.2, 1.8),
        (0.5, 1.4),
        (1.0, 1.0),
        (3.0, 0.8),
        (13.0, 0.7),
    ];
    let Some(m) = giant_mass_jupiter else {
        return 0.7;
    };
    let x = m.clamp(CURVE[0].0, CURVE[6].0).log10();
    let i = CURVE
        .windows(2)
        .position(|w| x <= w[1].0.log10())
        .unwrap_or(5);
    let ((m0, f0), (m1, f1)) = (CURVE[i], CURVE[i + 1]);
    f0 + (f1 - f0) * ((x - m0.log10()) / (m1.log10() - m0.log10())).clamp(0.0, 1.0)
}

/// Eccentricity a giant on orbit `(a_g, e_g)` forces on a planet at `a`,
/// to first order in secular theory (Heppenheimer 1978): `(5/4)(a/a_g)
/// e_g/(1 − e_g²)` inside the giant's orbit, the ratio inverted outside it.
pub fn forced_eccentricity(a: f64, a_g: f64, e_g: f64) -> f64 {
    let ratio = if a < a_g { a / a_g } else { a_g / a };
    1.25 * ratio * e_g / (1.0 - e_g * e_g).max(1e-6)
}

/// An orbit being placed: its body's mass (solar masses) and its closest
/// and farthest distances.
#[derive(Clone, Copy)]
struct Track {
    mass: f64,
    orbit: f64,
    eccentricity: f64,
}

impl Track {
    /// Whether two orbits keep the least gap of [`HILL_GAP`] mutual Hill
    /// radii between them.
    fn clear_of(&self, other: &Track, star_mass: f64) -> bool {
        let (inner, outer) = if self.orbit < other.orbit {
            (self, other)
        } else {
            (other, self)
        };
        let hill = ((self.mass + other.mass) / (3.0 * star_mass)).cbrt()
            * 0.5
            * (self.orbit + other.orbit);
        let gap =
            outer.orbit * (1.0 - outer.eccentricity) - inner.orbit * (1.0 + inner.eccentricity);
        gap >= HILL_GAP * hill
    }
}

impl GiantSystem {
    /// The giants of the world `p`, or `None` when its request turned them
    /// off. The same parameters always give the same giants.
    pub fn draw(p: &WorldParams) -> Option<Self> {
        if p.giants == GiantsMode::None {
            return None;
        }
        let star = values(p.star);
        let mut rng = StdRng::seed_from_u64(u64::from(stream_seed(p.seed, stream::GIANTS)));
        let world = Track {
            mass: world_mass_earth(p) / SUN_EARTH_MASSES,
            orbit: f64::from(p.orbit_au),
            eccentricity: f64::from(p.eccentricity),
        };
        let metals = f64::from(p.metallicity);
        let star_mass = f64::from(star.mass);
        let mass_factor = star_mass.clamp(0.1, 2.5);
        let gas_mean = GAS_GIANTS * 10f64.powf(2.0 * metals).min(10.0) * mass_factor;
        let ice_mean = ICE_GIANTS * 10f64.powf(0.5 * metals) * mass_factor.sqrt();
        let ice_line = 2.7 * f64::from(star.luminosity).sqrt();
        // A rocky world close in leaves little room for a hot Jupiter.
        let lonely = if world.orbit < 0.5 { 0.2 } else { 1.0 };
        let hot_odds =
            (HOT_JUPITERS * 10f64.powf(2.0 * metals).min(10.0) * mass_factor * lonely).min(0.2);

        // Draw every candidate first, so their number does not change the
        // draws of the others.
        let hot = rng.random::<f64>() < hot_odds;
        let gas = poisson(gas_mean, &mut rng);
        let ice = poisson(ice_mean, &mut rng);
        let mut candidates: Vec<(GiantKind, bool)> = Vec::new();
        if hot {
            candidates.push((GiantKind::Gas, true));
        }
        candidates.extend(std::iter::repeat_n((GiantKind::Gas, false), gas));
        candidates.extend(std::iter::repeat_n((GiantKind::Ice, false), ice));

        let mut placed: Vec<(Track, GiantKind, bool, u32)> = Vec::new();
        for (kind, is_hot) in candidates {
            let mass_earth = match kind {
                GiantKind::Gas => {
                    let (lo, hi) = (
                        GAS_MASS.0.powf(GAS_MASS_SLOPE),
                        GAS_MASS.1.powf(GAS_MASS_SLOPE),
                    );
                    (lo + rng.random::<f64>() * (hi - lo)).powf(1.0 / GAS_MASS_SLOPE)
                        * f64::from(JUPITER_EARTH_MASSES)
                }
                GiantKind::Ice => {
                    (ICE_MASS.0.ln() + rng.random::<f64>() * (ICE_MASS.1 / ICE_MASS.0).ln()).exp()
                }
            };
            let seed = rng.random::<u32>();
            for _ in 0..TRIES {
                let (orbit, eccentricity) = if is_hot {
                    let days = (HOT_PERIOD_DAYS.0.ln()
                        + rng.random::<f64>() * (HOT_PERIOD_DAYS.1 / HOT_PERIOD_DAYS.0).ln())
                    .exp();
                    ((star_mass * (days / 365.25).powi(2)).cbrt(), 0.0)
                } else {
                    let (mean, spread) = if kind == GiantKind::Gas {
                        GAS_ORBIT
                    } else {
                        ICE_ORBIT
                    };
                    let orbit = ice_line * 10f64.powf(mean + spread * normal(&mut rng));
                    (orbit, f64::from(draw_eccentricity(&mut rng)))
                };
                if !(ORBIT_RANGE.0..=ORBIT_RANGE.1).contains(&orbit) {
                    continue;
                }
                let track = Track {
                    mass: mass_earth / SUN_EARTH_MASSES,
                    orbit,
                    eccentricity,
                };
                if track.clear_of(&world, star_mass)
                    && placed
                        .iter()
                        .all(|(other, ..)| track.clear_of(other, star_mass))
                {
                    placed.push((track, kind, is_hot, seed));
                    break;
                }
            }
            if placed.len() == MAX_GIANTS {
                break;
            }
        }
        placed.sort_by(|a, b| a.0.orbit.total_cmp(&b.0.orbit));

        let giants: Vec<Giant> = placed
            .iter()
            .enumerate()
            .map(|(i, &(track, kind, _, seed))| {
                // Lettered among the planets, the world's orbit included.
                let rank = i + usize::from(track.orbit > world.orbit);
                describe(&star, track, kind, seed, rank)
            })
            .collect();
        let forced = giants
            .iter()
            .map(|g| {
                forced_eccentricity(
                    world.orbit,
                    f64::from(g.orbit_au),
                    f64::from(g.eccentricity),
                )
                .powi(2)
            })
            .sum::<f64>()
            // An empty sum is −0.
            .max(0.0)
            .sqrt()
            .min(MAX_FORCED);
        // The impactors come from beyond the world, stirred by the most
        // massive giant just outside its orbit (1.5 to 20 times as far).
        let outer = giants
            .iter()
            .filter(|g| (1.5..=20.0).contains(&(f64::from(g.orbit_au) / world.orbit)))
            .map(|g| g.mass_earth / JUPITER_EARTH_MASSES)
            .max_by(f32::total_cmp);
        Some(GiantSystem {
            giants,
            forced_eccentricity: forced as f32,
            impact_factor: impact_factor(outer),
        })
    }
}

/// The stored description of a placed giant, the `rank`-th planet from the
/// star (see [`crate::naming`]).
fn describe(star: &StarValues, track: Track, kind: GiantKind, seed: u32, rank: usize) -> Giant {
    let mut rng = StdRng::seed_from_u64(u64::from(seed));
    let mass_earth = (track.mass * SUN_EARTH_MASSES) as f32;
    let orbit_au = track.orbit as f32;
    let eccentricity = track.eccentricity as f32;
    let flux = star.luminosity / (orbit_au * orbit_au) * mean_flux_factor(eccentricity);
    let equilibrium = equilibrium_k(flux);
    // Heat left from formation: Jupiter's internal temperature is about
    // 100 K; less for smaller giants, little for ice giants.
    let internal = match kind {
        GiantKind::Gas => 100.0 * (mass_earth / JUPITER_EARTH_MASSES).powf(0.4),
        GiantKind::Ice => 40.0,
    };
    let effective = (equilibrium.powi(4) + internal.powi(4)).powf(0.25);
    let class = cloud_class(effective);
    let hot = equilibrium > 1000.0;
    // The odds of the giant worlds' presets (`[giant] rings`): bright
    // rings only on cold gas giants, faint ones on most giants, rarely on
    // hot ones, whose ice would boil away.
    let [bright, faint] = match (hot, kind) {
        (true, _) => [0.0, 0.05],
        (false, GiantKind::Gas) => [0.25, 0.5],
        (false, GiantKind::Ice) => [0.0, 0.7],
    };
    let rings = match rng.random::<f32>() {
        r if r < bright => Rings::Bright,
        r if r < bright + faint => Rings::Faint,
        _ => Rings::None,
    };
    Giant {
        name: crate::naming::planet_letter(rank),
        kind,
        mass_earth,
        radius_earth: giant_radius(mass_earth, equilibrium),
        orbit_au,
        eccentricity,
        periapsis_angle: rng.random::<f32>() * std::f32::consts::TAU,
        phase: rng.random::<f32>() * std::f32::consts::TAU,
        year_days: year_days(orbit_au, star.mass),
        equilibrium_k: equilibrium,
        effective_k: effective,
        class,
        hot,
        colour: giant_colour(kind, class, rng.random()),
        rings,
        seed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preset::Preset;

    fn params(seed: u32, star: f32, orbit_au: f32, metallicity: f32) -> WorldParams {
        WorldParams {
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
            star,
            orbit_au,
            eccentricity: 0.02,
            drift_ma: 0.0,
            moons: crate::moons::MoonsMode::None,
            shape: None,
            metallicity,
            giants: GiantsMode::Auto,
            giant_mass_earth: None,
            host: crate::exomoon::HostMode::None,
            given_rings: None,
            impact_factor: None,
        }
    }

    #[test]
    fn the_radius_goes_through_the_solar_giants() {
        for (mass, radius) in [(17.15, 3.88), (95.16, 9.45), (317.8, 11.21)] {
            assert!((giant_radius(mass, 100.0) - radius).abs() < 0.01, "{mass}");
        }
        // Barely smaller at ten Jupiter masses; inflated when hot.
        assert!((giant_radius(3178.0, 100.0) / 11.21 - 0.912).abs() < 0.01);
        assert!(giant_radius(317.8, 1800.0) > 1.4 * 11.21);
        assert!(giant_radius(317.8, 1800.0) < 1.9 * 11.21);
    }

    #[test]
    fn the_solar_giants_get_their_classes_and_the_curve_its_peak() {
        assert_eq!(cloud_class(124.0), 1);
        assert_eq!(cloud_class(1200.0), 4);
        assert_eq!(impact_factor(Some(1.0)), 1.0);
        assert!((impact_factor(Some(0.2)) - 1.8).abs() < 1e-5);
        assert!(impact_factor(None) < 1.0);
        // Jupiter (e = 0.049) forces about 0.012 on Earth.
        assert!((forced_eccentricity(1.0, 5.2, 0.049) - 0.0118).abs() < 0.001);
    }

    #[test]
    fn sun_like_stars_have_giants_at_the_observed_rates() {
        let n = 4000;
        let systems: Vec<_> = (0..n)
            .map(|s| GiantSystem::draw(&params(s, 42.0, 1.0, 0.0)).unwrap())
            .collect();
        let gas: Vec<&Giant> = systems
            .iter()
            .flat_map(|s| &s.giants)
            .filter(|g| g.kind == GiantKind::Gas)
            .collect();
        let per_star = gas.len() as f64 / f64::from(n);
        assert!((0.17..0.3).contains(&per_star), "{per_star}");
        // Most beyond 1 AU; a few hot Jupiters.
        let far = gas.iter().filter(|g| g.orbit_au > 1.0).count() as f64 / gas.len() as f64;
        assert!(far > 0.75, "{far}");
        let hot = gas.iter().filter(|g| g.hot).count() as f64 / f64::from(n);
        assert!((0.003..0.02).contains(&hot), "{hot}");
        // Every giant clear of the world and of the others.
        for system in &systems {
            assert!(system.giants.len() <= MAX_GIANTS);
            for g in &system.giants {
                let gap = if g.orbit_au > 1.0 {
                    g.orbit_au * (1.0 - g.eccentricity) - 1.02
                } else {
                    0.98 - g.orbit_au * (1.0 + g.eccentricity)
                };
                assert!(gap > 0.0, "{g:?}");
            }
            assert!(
                system
                    .giants
                    .windows(2)
                    .all(|w| w[0].orbit_au < w[1].orbit_au)
            );
        }
    }

    #[test]
    fn metal_rich_and_massive_stars_have_more_giants() {
        let count = |star: f32, metals: f32| -> usize {
            (0..3000)
                .map(|s| {
                    GiantSystem::draw(&params(s, star, 1.0, metals))
                        .unwrap()
                        .giants
                        .iter()
                        .filter(|g| g.kind == GiantKind::Gas)
                        .count()
                })
                .sum()
        };
        let solar = count(42.0, 0.0);
        assert!(count(42.0, 0.3) > 2 * solar);
        assert!(count(64.0, 0.0) < solar / 2);
    }

    #[test]
    fn the_same_world_gets_the_same_giants_and_none_when_off() {
        let p = params(11, 42.0, 1.0, 0.1);
        assert_eq!(GiantSystem::draw(&p), GiantSystem::draw(&p));
        let off = WorldParams {
            giants: GiantsMode::None,
            ..p
        };
        assert_eq!(GiantSystem::draw(&off), None);
        // Metallicities near the Sun's, within the limits.
        let drawn: Vec<f32> = (0..2000).map(draw_metallicity).collect();
        let mean = drawn.iter().sum::<f32>() / drawn.len() as f32;
        assert!((mean - METALLICITY_MEAN).abs() < 0.02);
        assert!(
            drawn
                .iter()
                .all(|m| (METALLICITY.0..=METALLICITY.1).contains(m))
        );
    }
}
