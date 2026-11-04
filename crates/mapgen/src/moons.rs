//! Moons: how many a world has, where they orbit and what they are like
//! (backlog item 13; `docs/analysis/11-moons.md`).
//!
//! **Origins.** Rocky planets get moons three ways, each with its own odds
//! (constants below), all drawn from a noise stream of their own, so the
//! planet of a seed does not depend on its moons:
//!
//! - a **giant impact** (the Moon, Charon): one large round moon, about one
//!   rocky planet in 12 (Elser et al. 2011), one small world in two, with a
//!   mass ratio up to 0.03 on large planets and 0.15 on small ones, where
//!   the barycentre can lie outside the planet (a binary, like Pluto and
//!   Charon);
//! - a **debris disk** of a smaller impact (Phobos and Deimos): one or two
//!   small moons close in;
//! - a **capture**: one to three small irregular moons far out, eccentric,
//!   inclined, half of them retrograde.
//!
//! **Stability.** A moon must orbit beyond the Roche limit (fluid for
//! round moons, rigid for small ones) and within a third of the Hill radius
//! at the planet's periapsis (half for retrograde moons). Moons too close
//! to a larger one are dropped. A planet locked to its star keeps no moon:
//! its synchronous orbit lies beyond its Hill sphere, so tides bring every
//! moon down (Barnes & O'Brien 2002).
//!
//! **Shapes.** Moons smaller than about 900 km across (rock) or 400 km
//! (ice) are not pulled round by their gravity: they get three axes, the
//! more unequal the smaller they are.
//!
//! **Migration.** A moon above the synchronous orbit recedes (the Moon), a
//! moon below it or on a retrograde orbit falls in (Phobos, Triton); a
//! binary is locked face to face and stays put, and far captured moons
//! hardly move.
use crate::noise::{stream, stream_seed};
use crate::star::StarInfo;
use crate::world::WorldParams;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// Whether a world has moons: those its seed draws, none, or a given
/// number (`{"count": 3}` in a request), drawn from the seed too.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "lowercase")]
pub enum MoonsMode {
    #[default]
    Auto,
    None,
    /// Exactly this many moons, 1 to [`MAX_MOON_COUNT`] (0 is `None`).
    Count(u8),
}

/// Most moons a request can ask for.
pub const MAX_MOON_COUNT: u8 = 6;

impl std::str::FromStr for MoonsMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "auto" => Ok(Self::Auto),
            "none" | "0" => Ok(Self::None),
            other => match other.parse::<u8>() {
                Ok(n) if (1..=MAX_MOON_COUNT).contains(&n) => Ok(Self::Count(n)),
                _ => Err(format!(
                    "unknown moons mode {other:?} (auto, none or a number from 1 to {MAX_MOON_COUNT})"
                )),
            },
        }
    }
}

impl MoonsMode {
    /// The number of moons asked for, if one is.
    pub fn count(self) -> Option<u8> {
        match self {
            Self::Count(n) => Some(n),
            _ => None,
        }
    }
}

/// How a moon formed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum MoonOrigin {
    /// From the debris of a giant impact on the planet (the Moon, Charon).
    GiantImpact,
    /// From the debris disk of a smaller impact (Phobos and Deimos).
    Debris,
    /// A passing body captured by the planet.
    Captured,
}

/// What a moon is made of.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "lowercase")]
pub enum Composition {
    Rock,
    Ice,
}

/// Where a moon's orbit is heading, under the planet's tides.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "lowercase")]
pub enum Migration {
    /// Above the synchronous orbit: it recedes (the Moon, 3.8 cm/yr).
    Receding,
    /// Below the synchronous orbit or retrograde: it falls in, and will
    /// break into a ring at the Roche limit (Phobos).
    Decaying,
    /// Locked face to face with its planet (a binary): it stays put.
    Locked,
    /// Too far for the tides to move it much.
    Negligible,
}

/// One moon, as `world.json` stores it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Moon {
    /// Roman numeral by distance from the planet, `I` nearest.
    pub name: String,
    pub origin: MoonOrigin,
    pub composition: Composition,
    /// Mean radius, km.
    pub radius_km: f32,
    /// The three semi-axes, km (all equal for a round moon).
    pub axes_km: [f32; 3],
    /// Pulled round by its own gravity.
    pub round: bool,
    /// Mass relative to the planet's (the Moon: 0.0123, Charon: 0.12).
    pub mass_ratio: f32,
    /// Bulk density, g/cm³.
    pub density: f32,
    /// Surface gravity, relative to Earth's.
    pub gravity: f32,
    /// Semi-major axis of the orbit, km and planet radii.
    pub semi_major_axis_km: f32,
    pub semi_major_axis_radii: f32,
    pub eccentricity: f32,
    /// Inclination to the planet's equator, degrees; above 90 the orbit is
    /// retrograde.
    pub inclination_deg: f32,
    /// Orbital period, Earth days.
    pub period_days: f32,
    /// Keeps the same face towards its planet.
    pub tidally_locked: bool,
    pub migration: Migration,
    /// Apparent diameter seen from the planet's surface, degrees (the Moon:
    /// 0.52).
    pub angular_diameter_deg: f32,
    /// Distance of the barycentre of the planet and this moon from the
    /// planet's centre, km: beyond the planet's radius, they orbit a point
    /// in space between them.
    pub barycentre_km: f32,
    /// Seed of the moon's own surface.
    pub seed: u32,
}

/// The moons of a world and what bounds them, as `world.json` stores them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct MoonSystem {
    /// Rotation period of the planet, hours: drawn, its year on a planet
    /// locked to its star, the moon's month for a binary.
    pub planet_day_hours: f32,
    /// Orbit whose period equals the planet's day, km.
    pub synchronous_orbit_km: f32,
    /// Hill radius at the planet's periapsis, km: the star pulls away
    /// anything beyond (stable orbits stay within a third).
    pub hill_radius_km: f32,
    /// The planet and its largest moon orbit a point outside the planet.
    pub binary: bool,
    /// The range of the tides on the planet (moons and star), relative to
    /// Earth's; it sets the width of the tidal flats (see
    /// [`PlanetEffects`]). Missing on worlds made before it: 0.
    #[serde(default)]
    pub tide: f32,
    /// A large moon keeps the planet's axial tilt steady.
    #[serde(default)]
    pub tilt_steadied: bool,
    /// The number of moons the request asked for, when it asked for one
    /// (otherwise the seed alone drew them).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count_asked: Option<u8>,
    /// Nearest first.
    pub moons: Vec<Moon>,
}

/// Gravitational constant, km³ / (kg s²).
const G: f64 = 6.674e-20;
const EARTH_MASS_KG: f64 = 5.972e24;
const EARTH_RADIUS_KM: f64 = 6371.0;
/// The planet's density, g/cm³: Earth's, as `WorldParams::gravity_modifier`
/// assumes.
const PLANET_DENSITY: f64 = 5.51;
const SUN_MASS_KG: f64 = 1.989e30;
const AU_KM: f64 = 1.496e8;
/// Below this equilibrium temperature, K, moons formed from the debris of
/// a cold planet or captured from its neighbourhood are icy.
const SNOW_LINE_K: f32 = 150.0;

/// Odds of a giant-impact moon: on worlds up to `SMALL_WORLD` Earth radii,
/// and from `LARGE_WORLD` up (Elser et al. 2011: 1 in 12, 1 in 4 to 1 in 45
/// depending on the assumptions), linear in between.
const IMPACT_ODDS: (f64, f64) = (0.5, 1.0 / 12.0);
const SMALL_WORLD: f64 = 0.25;
const LARGE_WORLD: f64 = 0.4;
/// Mass ratio range of a giant-impact moon, the upper end on small worlds
/// and on large ones (Canup 2005: about 0.1 from oblique, slow impacts).
const IMPACT_MASS_RATIO: (f64, f64, f64) = (0.005, 0.15, 0.03);
/// Semi-major axis of a giant-impact moon, planet radii, on small and on
/// large worlds (Charon: 16.5; the Moon: 60).
const IMPACT_ORBIT_SMALL: (f64, f64) = (8.0, 25.0);
const IMPACT_ORBIT_LARGE: (f64, f64) = (10.0, 65.0);
/// Odds of debris moonlets (no giant-impact moon), and their size: Phobos
/// (11 km) around Mars (3 390 km), scaled with the planet.
const DEBRIS_ODDS: f64 = 0.2;
const DEBRIS_RADIUS: (f64, f64) = (0.4, 2.5);
const PHOBOS_OVER_MARS: f64 = 11.1 / 3390.0;
/// Odds of captured moons, their radius range, km, and their semi-major
/// axis range as a share of the Hill radius.
const CAPTURE_ODDS: f64 = 0.15;
const CAPTURED_RADIUS_KM: (f64, f64) = (1.0, 30.0);
const CAPTURED_ORBIT: (f64, f64) = (0.05, 0.3);
/// Semi-major axis range of the captured moons added to reach a number
/// asked for, as a share of the Hill radius: wider, so that several fit
/// (retrograde orbits stay stable out to half of it).
const COUNT_CAPTURED_ORBIT: (f64, f64) = (0.03, 0.45);
/// Diameter below which a body is not round, km, for rock and ice.
const ROUND_ROCK_KM: f64 = 900.0;
const ROUND_ICE_KM: f64 = 400.0;
/// Mass ratio from which the largest moon makes a binary, locked face to
/// face with its planet.
const BINARY_MASS_RATIO: f64 = 0.05;
/// Moons closer to a larger one than this many mutual Hill radii are
/// dropped (orbits that close do not last).
const SPACING_HILL: f64 = 8.0;
/// Salt of the draw sequence of the moons added to reach a number asked
/// for, and the most moons drawn for it.
const COUNT_STREAM: u64 = 0x434f_554e_5445_4421;
const COUNT_ATTEMPTS: usize = 1500;
/// Width of the chaotic zone around a moon's orbit, in `a × q^(2/7)`
/// (Wisdom 1980): smaller moons do not last there.
const CHAOTIC_ZONE: f64 = 1.5;
/// Beyond this share of the Hill radius the tides hardly move a moon.
const TIDES_REACH: f64 = 0.1;

/// A SplitMix64 stream of uniform draws.
struct Draws(u64);

impl Draws {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
    }

    fn uniform(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next()
    }

    fn log_uniform(&mut self, lo: f64, hi: f64) -> f64 {
        (lo.ln() + (hi.ln() - lo.ln()) * self.next()).exp()
    }
}

/// A moon being drawn, before the system keeps it.
struct Candidate {
    origin: MoonOrigin,
    composition: Composition,
    mass_kg: f64,
    density: f64,
    a_km: f64,
    eccentricity: f64,
    inclination_deg: f64,
    shape: [f64; 2],
}

impl Candidate {
    fn radius_km(&self) -> f64 {
        // density g/cm³ = 1e12 kg/km³.
        (3.0 * self.mass_kg / (4.0 * PI * self.density * 1e12)).cbrt()
    }

    fn round(&self) -> bool {
        let limit = match self.composition {
            Composition::Rock => ROUND_ROCK_KM,
            Composition::Ice => ROUND_ICE_KM,
        };
        2.0 * self.radius_km() >= limit
    }
}

/// Roman numeral of `n` (1 to 39).
fn roman(mut n: usize) -> String {
    let mut s = String::new();
    for (value, digit) in [(10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I")] {
        while n >= value {
            s.push_str(digit);
            n -= value;
        }
    }
    s
}

impl MoonSystem {
    /// The moons of world `p`, `None` when its request turned them off.
    pub fn draw(p: &WorldParams) -> Option<Self> {
        if p.moons == MoonsMode::None {
            return None;
        }
        let mut rng = Draws(u64::from(stream_seed(p.seed, stream::MOONS)));
        let star = StarInfo::new(p.star, p.orbit_au, p.eccentricity);
        let r_p = f64::from(p.circumference_km) / (2.0 * PI);
        let size = r_p / EARTH_RADIUS_KM;
        let m_p = EARTH_MASS_KG * size.powi(3) * PLANET_DENSITY / 5.51;
        let periapsis_km = f64::from(star.periapsis_au.max(1e-4)) * AU_KM;
        let hill = periapsis_km * (m_p / (3.0 * f64::from(star.mass) * SUN_MASS_KG)).cbrt();
        let cold = star.equilibrium_k < SNOW_LINE_K;
        let synchronous = |day_hours: f64| {
            let t = day_hours * 3600.0;
            (G * m_p * t * t / (4.0 * PI * PI)).cbrt()
        };
        // Drawn even on a locked planet, so a world's draws do not depend
        // on its star.
        let drawn_day = rng.log_uniform(12.0, 36.0);
        if star.tidally_locked {
            let day = f64::from(star.year_days) * 24.0;
            return Some(
                Self {
                    planet_day_hours: day as f32,
                    synchronous_orbit_km: synchronous(day) as f32,
                    hill_radius_km: hill as f32,
                    binary: false,
                    tide: 0.0,
                    tilt_steadied: false,
                    count_asked: p.moons.count(),
                    moons: Vec::new(),
                }
                .with_effects(p),
            );
        }

        // Shares of the way from small to large worlds.
        let large = ((size - SMALL_WORLD) / (LARGE_WORLD - SMALL_WORLD)).clamp(0.0, 1.0);
        let impact_odds = IMPACT_ODDS.0 + (IMPACT_ODDS.1 - IMPACT_ODDS.0) * large;
        let q_max = IMPACT_MASS_RATIO.1 + (IMPACT_MASS_RATIO.2 - IMPACT_MASS_RATIO.1) * large;
        let orbit = if large >= 1.0 {
            IMPACT_ORBIT_LARGE
        } else if large <= 0.0 {
            IMPACT_ORBIT_SMALL
        } else {
            let mix = |a: f64, b: f64| a + (b - a) * large;
            (
                mix(IMPACT_ORBIT_SMALL.0, IMPACT_ORBIT_LARGE.0),
                mix(IMPACT_ORBIT_SMALL.1, IMPACT_ORBIT_LARGE.1),
            )
        };
        let surface = |icy: bool| {
            if icy {
                Composition::Ice
            } else {
                Composition::Rock
            }
        };
        // One moon of each origin, from the draws it takes (always all of
        // them, so one origin's odds never shift another's draws).
        let impact_moon = |rng: &mut Draws| {
            let q = rng.log_uniform(IMPACT_MASS_RATIO.0, q_max);
            let a_km = rng.log_uniform(orbit.0, orbit.1) * r_p;
            let (eccentricity, inclination_deg) = (rng.uniform(0.0, 0.06), rng.uniform(0.0, 8.0));
            let shape = [rng.next(), rng.next()];
            // Small cold worlds are ice and rock, like Charon (1.7 g/cm³).
            let icy = cold && size < LARGE_WORLD;
            Candidate {
                origin: MoonOrigin::GiantImpact,
                composition: surface(icy),
                mass_kg: q * m_p,
                density: if icy { 1.7 } else { 3.3 },
                a_km,
                eccentricity,
                inclination_deg,
                shape,
            }
        };
        // The inner (k = 0) or outer debris moonlet.
        let debris_moon = |rng: &mut Draws, k: usize| {
            let radius = r_p * PHOBOS_OVER_MARS * rng.log_uniform(DEBRIS_RADIUS.0, DEBRIS_RADIUS.1);
            let a_km = if k == 0 {
                rng.log_uniform(2.2, 4.0)
            } else {
                rng.log_uniform(5.0, 9.0)
            } * r_p;
            let (eccentricity, inclination_deg) = (rng.uniform(0.0, 0.02), rng.uniform(0.0, 2.0));
            let shape = [rng.next(), rng.next()];
            let density = 1.9;
            Candidate {
                origin: MoonOrigin::Debris,
                composition: surface(cold),
                mass_kg: 4.0 / 3.0 * PI * radius.powi(3) * density * 1e12,
                density,
                a_km,
                eccentricity,
                inclination_deg,
                shape,
            }
        };
        let captured_moon = |rng: &mut Draws, (lo, hi): (f64, f64)| {
            let radius = rng.log_uniform(CAPTURED_RADIUS_KM.0, CAPTURED_RADIUS_KM.1);
            let a_km = rng.log_uniform(lo, hi) * hill;
            let eccentricity = rng.uniform(0.1, 0.45);
            let inclination_deg = if rng.next() < 0.5 {
                rng.uniform(100.0, 170.0)
            } else {
                rng.uniform(10.0, 60.0)
            };
            let shape = [rng.next(), rng.next()];
            let density = if cold { 1.5 } else { 1.9 };
            Candidate {
                origin: MoonOrigin::Captured,
                composition: surface(cold),
                mass_kg: 4.0 / 3.0 * PI * radius.powi(3) * density * 1e12,
                density,
                a_km,
                eccentricity,
                inclination_deg,
                shape,
            }
        };
        // Stable orbits only: beyond the Roche limit, within the Hill sphere.
        let stable = |c: &Candidate| {
            let roche = if c.round() {
                2.44 * r_p * (PLANET_DENSITY / c.density).cbrt()
            } else {
                1.26 * r_p * (PLANET_DENSITY / c.density).cbrt()
            };
            let reach = if c.inclination_deg > 90.0 {
                0.5
            } else {
                1.0 / 3.0
            };
            c.a_km * (1.0 - c.eccentricity) > roche
                && c.a_km * (1.0 + c.eccentricity) < reach * hill
        };
        // Orbits too close to a larger moon's do not last.
        let clear = |kept: &[Candidate], c: &Candidate| {
            kept.iter().all(|k| {
                let mutual =
                    (c.a_km + k.a_km) / 2.0 * ((c.mass_kg + k.mass_kg) / (3.0 * m_p)).cbrt();
                (c.a_km - k.a_km).abs() > SPACING_HILL * mutual
                    && (c.a_km / k.a_km).max(k.a_km / c.a_km) > 1.2
            })
        };

        let mut candidates = Vec::new();
        let impact = rng.next() < impact_odds;
        let moon = impact_moon(&mut rng);
        if impact {
            candidates.push(moon);
        }
        let debris = rng.next() < DEBRIS_ODDS && !impact;
        let debris_count = 1 + usize::from(rng.next() < 0.5);
        for k in 0..2 {
            let moon = debris_moon(&mut rng, k);
            if debris && k < debris_count {
                candidates.push(moon);
            }
        }
        let captured = rng.next() < CAPTURE_ODDS;
        let captured_count = 1 + (rng.next() * 3.0) as usize;
        for k in 0..3 {
            let moon = captured_moon(&mut rng, CAPTURED_ORBIT);
            if captured && k < captured_count {
                candidates.push(moon);
            }
        }
        candidates.retain(|c| stable(c));
        candidates.sort_by(|a, b| b.mass_kg.total_cmp(&a.mass_kg));
        let mut kept: Vec<Candidate> = Vec::new();
        for c in candidates {
            if clear(&kept, &c) {
                kept.push(c);
            }
        }
        // Moons added to reach a number asked for keep out of the chaotic
        // zone of each heavier one (about 1.5 a q^(2/7) around its orbit,
        // Wisdom 1980), from periapsis to apoapsis: the rule for orbits of
        // comparable masses (`clear`) would leave no room beside a large
        // moon.
        let apart = |kept: &[Candidate], c: &Candidate| {
            kept.iter().all(|k| {
                let big = if k.mass_kg >= c.mass_kg { k } else { c };
                let zone = CHAOTIC_ZONE * big.a_km * (big.mass_kg / m_p).powf(2.0 / 7.0);
                let (inner, outer) = if c.a_km < k.a_km { (c, k) } else { (k, c) };
                outer.a_km * (1.0 - outer.eccentricity) - inner.a_km * (1.0 + inner.eccentricity)
                    > zone
                    && outer.a_km / inner.a_km > 1.2
            })
        };
        // A number of moons asked for: those the seed drew first (the
        // largest when there are too many), then more from a draw sequence
        // of their own: a giant-impact moon if there is none yet, debris
        // moonlets close in, then captured moons far out, each kept when
        // its orbit is stable and clear of the others. A Hill sphere too
        // small for them all keeps fewer (the request checks it).
        if let Some(n) = p.moons.count() {
            let n = usize::from(n);
            kept.truncate(n);
            let mut extra = Draws(u64::from(stream_seed(p.seed, stream::MOONS)) ^ COUNT_STREAM);
            let mut attempt = 0;
            while kept.len() < n && attempt < COUNT_ATTEMPTS {
                let has_impact = kept.iter().any(|c| c.origin == MoonOrigin::GiantImpact);
                let debris_kept = kept
                    .iter()
                    .filter(|c| c.origin == MoonOrigin::Debris)
                    .count();
                let c = if !has_impact && attempt < COUNT_ATTEMPTS / 4 {
                    impact_moon(&mut extra)
                } else if debris_kept < 2 && attempt % 2 == 0 {
                    debris_moon(&mut extra, debris_kept)
                } else {
                    // Far out only retrograde orbits last, as for the outer
                    // irregular moons of Jupiter.
                    let mut c = captured_moon(&mut extra, COUNT_CAPTURED_ORBIT);
                    // Less eccentric, so that several share the space.
                    c.eccentricity *= 0.5;
                    if c.inclination_deg < 90.0 && c.a_km * (1.0 + c.eccentricity) >= hill / 3.0 {
                        c.inclination_deg = 180.0 - c.inclination_deg;
                    }
                    c
                };
                if stable(&c) && apart(&kept, &c) {
                    kept.push(c);
                }
                attempt += 1;
            }
        }
        kept.sort_by(|a, b| a.a_km.total_cmp(&b.a_km));

        let period_days =
            |c: &Candidate| 2.0 * PI * (c.a_km.powi(3) / (G * (m_p + c.mass_kg))).sqrt() / 86_400.0;
        // A binary: the planet turns with its largest moon.
        let largest = kept
            .iter()
            .max_by(|a, b| a.mass_kg.total_cmp(&b.mass_kg))
            .filter(|c| c.mass_kg / m_p >= BINARY_MASS_RATIO);
        let binary = largest.is_some_and(|c| c.a_km * c.mass_kg / (m_p + c.mass_kg) > r_p);
        let day_hours = largest.map_or(drawn_day, |c| period_days(c) * 24.0);
        let sync_km = synchronous(day_hours);
        let seed = p.seed;

        let moons = kept
            .iter()
            .enumerate()
            .map(|(k, c)| {
                let radius = c.radius_km();
                let round = c.round();
                let axes = if round {
                    [radius; 3]
                } else {
                    // The smaller, the more unequal.
                    let limit = match c.composition {
                        Composition::Rock => ROUND_ROCK_KM,
                        Composition::Ice => ROUND_ICE_KM,
                    };
                    let irregular = (1.0 - 2.0 * radius / limit).clamp(0.2, 1.0);
                    let b = 1.0 - irregular * (0.05 + 0.25 * c.shape[0]);
                    let c_axis = b - irregular * (0.05 + 0.25 * c.shape[1]);
                    let scale = radius / (b * c_axis).cbrt();
                    [scale, scale * b, scale * c_axis]
                };
                let is_largest = largest.is_some_and(|l| std::ptr::eq(l, c));
                let migration = if is_largest {
                    Migration::Locked
                } else if c.inclination_deg > 90.0 {
                    Migration::Decaying
                } else if c.a_km > TIDES_REACH * hill {
                    Migration::Negligible
                } else if c.a_km > sync_km {
                    Migration::Receding
                } else {
                    Migration::Decaying
                };
                let gravity = G * c.mass_kg / (radius * radius) / 9.81e-3;
                let barycentre = c.a_km * c.mass_kg / (m_p + c.mass_kg);
                Moon {
                    name: roman(k + 1),
                    origin: c.origin,
                    composition: c.composition,
                    radius_km: radius as f32,
                    axes_km: axes.map(|a| a as f32),
                    round,
                    mass_ratio: (c.mass_kg / m_p) as f32,
                    density: c.density as f32,
                    gravity: gravity as f32,
                    semi_major_axis_km: c.a_km as f32,
                    semi_major_axis_radii: (c.a_km / r_p) as f32,
                    eccentricity: c.eccentricity as f32,
                    inclination_deg: c.inclination_deg as f32,
                    period_days: period_days(c) as f32,
                    tidally_locked: c.origin != MoonOrigin::Captured,
                    migration,
                    angular_diameter_deg: (2.0 * (radius / (c.a_km - r_p)).atan()).to_degrees()
                        as f32,
                    barycentre_km: barycentre as f32,
                    seed: stream_seed(seed, stream::MOONS).wrapping_add(k as u32 + 1),
                }
            })
            .collect();
        Some(
            Self {
                planet_day_hours: day_hours as f32,
                synchronous_orbit_km: sync_km as f32,
                hill_radius_km: hill as f32,
                binary,
                tide: 0.0,
                tilt_steadied: false,
                count_asked: p.moons.count(),
                moons,
            }
            .with_effects(p),
        )
    }

    /// The system with what it does to its planet filled in.
    fn with_effects(mut self, p: &WorldParams) -> Self {
        let effects = PlanetEffects::of(p, Some(&self));
        self.tide = effects.tide;
        self.tilt_steadied = effects.stabilised;
        self
    }
}

/// Mass ratio from which a moon stabilises its planet's axial tilt (the
/// Moon: 0.0123; Laskar et al. 1993).
const STABILISING_MASS_RATIO: f64 = 0.005;
/// Tidal height of the Moon and of the Sun on Earth, in the units of
/// [`tidal_height`] (`mass ratio × (radius / distance)³ × radius in Earth
/// radii`): the Sun's is 0.46 of the Moon's.
const EARTH_MOON_TIDE: f64 = 0.0123 / (60.27 * 60.27 * 60.27);
const EARTH_SUN_TIDE: f64 = 0.46 * EARTH_MOON_TIDE;
/// Range of the shore width, as a share of the preset's: no tides (a
/// planet locked to its star) to strong ones. It follows the fourth root of
/// the tides: flats widen with their range, but far more with the slope of
/// the coast, which the tides do not change.
const SHORE_FACTOR: (f64, f64) = (0.5, 2.0);
/// Axial tilt range drawn without a stabilising moon, degrees, in place of
/// the usual `[5, 40]`: Mars, without one, ranges up to 60° (Laskar &
/// Robutel 1993).
pub const UNSTABLE_TILT_DEG: (f32, f32) = (0.0, 60.0);

/// What a world's moons do to it (step 5 of backlog item 13).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanetEffects {
    /// A large moon keeps the axial tilt steady.
    pub stabilised: bool,
    /// The range of the tides (moons and star), relative to Earth's.
    pub tide: f32,
    /// The preset's shore width is multiplied by this: tidal flats widen
    /// with the tides.
    pub shore_factor: f32,
}

/// The tidal height a body of mass `mass_ratio` (relative to the planet's)
/// raises at `distance_radii` planet radii on a planet of `size` Earth radii.
fn tidal_height(mass_ratio: f64, distance_radii: f64, size: f64) -> f64 {
    mass_ratio / distance_radii.powi(3) * size
}

impl PlanetEffects {
    /// The effects of `system` (none when moons were turned off) on world
    /// `p`. Tides only reshape coasts where they come and go: a planet
    /// locked to its star feels none from it, and a binary's moon, locked
    /// face to face with it, raises a bulge that never moves.
    pub fn of(p: &WorldParams, system: Option<&MoonSystem>) -> Self {
        let star = StarInfo::new(p.star, p.orbit_au, p.eccentricity);
        let r_p = f64::from(p.circumference_km) / (2.0 * PI);
        let size = r_p / EARTH_RADIUS_KM;
        let m_p = EARTH_MASS_KG * size.powi(3);
        let moons = system.map_or(&[][..], |s| &s.moons[..]);
        let stabilised = moons
            .iter()
            .any(|m| f64::from(m.mass_ratio) >= STABILISING_MASS_RATIO);
        let from_moons: f64 = moons
            .iter()
            .filter(|m| m.migration != Migration::Locked)
            .map(|m| {
                tidal_height(
                    f64::from(m.mass_ratio),
                    f64::from(m.semi_major_axis_radii),
                    size,
                )
            })
            .sum();
        let from_star = if star.tidally_locked {
            0.0
        } else {
            let star_ratio = f64::from(star.mass) * SUN_MASS_KG / m_p;
            tidal_height(star_ratio, f64::from(star.orbit_au) * AU_KM / r_p, size)
        };
        let tide = (from_moons + from_star) / (EARTH_MOON_TIDE + EARTH_SUN_TIDE);
        Self {
            stabilised,
            tide: tide as f32,
            shore_factor: tide.powf(0.25).clamp(SHORE_FACTOR.0, SHORE_FACTOR.1) as f32,
        }
    }

    /// An axial tilt drawn from `[lo, hi]` (the usual range), moved to
    /// [`UNSTABLE_TILT_DEG`] when no moon stabilises it.
    pub fn tilt(&self, drawn: f32, (lo, hi): (f32, f32)) -> f32 {
        if self.stabilised || hi <= lo {
            return drawn;
        }
        let share = ((drawn - lo) / (hi - lo)).clamp(0.0, 1.0);
        UNSTABLE_TILT_DEG.0 + share * (UNSTABLE_TILT_DEG.1 - UNSTABLE_TILT_DEG.0)
    }
}

/// Radius under which a rocky moon has no maria, km: only large moons kept
/// enough heat for lava to flood their basins (the Moon: 1 737 km).
const MARIA_MIN_RADIUS_KM: f32 = 500.0;
/// Narrowest map of a moon, pixels.
const MOON_MIN_WIDTH: u32 = 512;
/// Axial tilt of a moon, degrees: tides keep it small (the Moon: 1.5°).
const MOON_TILT_DEG: f32 = 1.5;
/// Low-frequency bumps of an irregular body, as a share of its radius at
/// full irregularity (equal axes add none).
const SHAPE_BUMPS: f64 = 0.12;

/// The world of `moon`, one of the moons of the world `planet`: a body of
/// the `rocky-moon` or `icy-moon` preset, airless and never wet, cratered
/// for 4 Ga, under the planet's star, its map as many pixels per km as the
/// planet's (at least [`MOON_MIN_WIDTH`] wide, at most the planet's
/// width). The long axis of an irregular moon points at longitude 180°,
/// the centre of its map: towards its planet, which tides lock it to.
pub fn body_params(planet: &WorldParams, moon: &Moon) -> WorldParams {
    let name = match moon.composition {
        Composition::Rock => "rocky-moon",
        Composition::Ice => "icy-moon",
    };
    let mut preset = crate::preset::Preset::moon(name).expect("moon presets are valid");
    if moon.radius_km < MARIA_MIN_RADIUS_KM {
        preset.geology.maria = None;
    }
    let ratio = f64::from(moon.radius_km) * 2.0 * PI / f64::from(planet.circumference_km);
    let widest = planet.width.max(MOON_MIN_WIDTH);
    let width =
        ((f64::from(planet.width) * ratio).round() as u32).clamp(MOON_MIN_WIDTH, widest) & !1;
    let surface_age_ga = preset.defaults.surface_age.map_or(4.0, |d| match d {
        crate::preset::ParamDefault::Fixed(v) => v,
        crate::preset::ParamDefault::Range([lo, _]) => lo,
    });
    WorldParams {
        width,
        height: width / 2,
        seed: moon.seed,
        planet: preset,
        sea_level: 0.0,
        volcanic_intensity: 0.0,
        circumference_km: (f64::from(moon.radius_km) * 2.0 * PI) as f32,
        axial_tilt_deg: MOON_TILT_DEG,
        erosion: 0.0,
        pressure_bar: 0.0,
        surface_age_ga,
        star: planet.star,
        orbit_au: planet.orbit_au,
        eccentricity: planet.eccentricity,
        drift_ma: 0.0,
        moons: MoonsMode::None,
        shape: (!moon.round).then(|| moon.axes_km.map(|a| a / moon.radius_km)),
        metallicity: planet.metallicity,
        giants: crate::giants::GiantsMode::None,
    }
}

/// The radius of the body `p` at a unit vector, as a share of its mean
/// radius: its ellipsoid, the first axis along `-x` (longitude 180°),
/// with low-frequency bumps, the more so the more unequal its axes.
/// `None` for a round body.
pub fn shape_at(p: &WorldParams, noise: &noise::Perlin, v: [f64; 3]) -> Option<f32> {
    use noise::NoiseFn;
    let [a, b, c] = p.shape?.map(f64::from);
    let ellipsoid = 1.0 / ((v[0] / a).powi(2) + (v[1] / b).powi(2) + (v[2] / c).powi(2)).sqrt();
    let irregular = 1.0 - c / a;
    let bumps = noise.get([v[0] * 1.5, v[1] * 1.5, v[2] * 1.5])
        + 0.5 * noise.get([v[0] * 3.0 + 7.0, v[1] * 3.0, v[2] * 3.0]);
    Some((ellipsoid * (1.0 + SHAPE_BUMPS * irregular * bumps)) as f32)
}

/// The shape of body `p` at every pixel (see [`shape_at`]), `None` for a
/// round body.
pub fn shape_layer(p: &WorldParams) -> Option<Vec<f32>> {
    use rayon::prelude::*;
    p.shape?;
    let noise = noise::Perlin::new(stream_seed(p.seed, stream::MOONS));
    let grid = p.grid();
    Some(
        (0..p.pixel_count())
            .into_par_iter()
            .map(|i| {
                let (x, y) = (i as u32 % p.width, i as u32 / p.width);
                let lon = (f64::from(x) + 0.5) / f64::from(p.width) * 2.0 * PI;
                let lat = grid.latitude(y);
                let v = [lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin()];
                shape_at(p, &noise, v).unwrap_or(1.0)
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preset::Preset;

    fn params(seed: u32, circumference_km: f32) -> WorldParams {
        WorldParams {
            width: 64,
            height: 32,
            seed,
            planet: Preset::load("terran").unwrap(),
            sea_level: 0.2,
            volcanic_intensity: 0.5,
            circumference_km,
            axial_tilt_deg: 23.4,
            erosion: 1.0,
            pressure_bar: 1.0,
            surface_age_ga: 0.4,
            star: crate::star::SUN,
            orbit_au: 1.0,
            eccentricity: 0.0,
            drift_ma: 0.0,
            moons: MoonsMode::Auto,
            shape: None,
            metallicity: 0.0,
            giants: crate::giants::GiantsMode::None,
        }
    }

    #[test]
    fn earths_get_a_large_moon_about_once_in_twelve() {
        let n = 4000;
        let (mut large, mut any, mut binaries) = (0, 0, 0);
        for seed in 0..n {
            let system = MoonSystem::draw(&params(seed, 40_075.0)).unwrap();
            large += usize::from(
                system
                    .moons
                    .iter()
                    .any(|m| m.origin == MoonOrigin::GiantImpact),
            );
            any += usize::from(!system.moons.is_empty());
            binaries += usize::from(system.binary);
            for m in &system.moons {
                // Inside the stable zone, a period that follows Kepler.
                assert!(
                    m.semi_major_axis_km * (1.0 + m.eccentricity) < 0.5 * system.hill_radius_km
                );
                assert!(m.semi_major_axis_radii > 1.0 && m.period_days > 0.0);
                assert!(
                    m.round
                        == (2.0 * m.radius_km >= 900.0 && m.composition == Composition::Rock
                            || 2.0 * m.radius_km >= 400.0 && m.composition == Composition::Ice)
                );
            }
        }
        let share = large as f64 / f64::from(n);
        assert!((0.06..0.11).contains(&share), "{share}");
        // Most planets have no moon; Earth-sized ones no binary.
        assert!(any < n as usize / 2, "{any}");
        assert_eq!(binaries, 0);
    }

    #[test]
    fn small_worlds_often_have_a_companion_and_sometimes_a_binary() {
        let (mut companions, mut binaries) = (0, 0);
        // Pluto: 7 460 km around, far from a cold star.
        for seed in 0..1000 {
            let mut p = params(seed, 7_460.0);
            p.orbit_au = 30.0;
            let system = MoonSystem::draw(&p).unwrap();
            companions += usize::from(
                system
                    .moons
                    .iter()
                    .any(|m| m.origin == MoonOrigin::GiantImpact),
            );
            if system.binary {
                binaries += 1;
                let moon = system
                    .moons
                    .iter()
                    .find(|m| m.migration == Migration::Locked)
                    .unwrap();
                assert!(moon.barycentre_km > 7_460.0 / (2.0 * std::f32::consts::PI));
                assert!((system.planet_day_hours / 24.0 - moon.period_days).abs() < 1e-3);
                assert_eq!(moon.composition, Composition::Ice);
            }
        }
        assert!(companions > 350, "{companions}");
        assert!(binaries > 50, "{binaries}");
    }

    #[test]
    fn moons_steady_the_tilt_and_raise_the_tides() {
        // Earth with the Moon: Earth's tides, Earth's shores.
        let earth = params(1, 40_075.0);
        let moon = Moon {
            name: "I".into(),
            origin: MoonOrigin::GiantImpact,
            composition: Composition::Rock,
            radius_km: 1737.0,
            axes_km: [1737.0; 3],
            round: true,
            mass_ratio: 0.0123,
            density: 3.3,
            gravity: 0.17,
            semi_major_axis_km: 384_400.0,
            semi_major_axis_radii: 60.27,
            eccentricity: 0.05,
            inclination_deg: 5.0,
            period_days: 27.3,
            tidally_locked: true,
            migration: Migration::Receding,
            angular_diameter_deg: 0.52,
            barycentre_km: 4670.0,
            seed: 0,
        };
        let system = |moons: Vec<Moon>| MoonSystem {
            planet_day_hours: 24.0,
            synchronous_orbit_km: 42_164.0,
            hill_radius_km: 1.5e6,
            binary: false,
            tide: 0.0,
            tilt_steadied: false,
            count_asked: None,
            moons,
        };
        let with = PlanetEffects::of(&earth, Some(&system(vec![moon.clone()])));
        assert!(with.stabilised);
        assert!((with.tide - 1.0).abs() < 0.05, "{with:?}");
        assert!((with.shore_factor - 1.0).abs() < 0.03);
        // Without it: the Sun's tides only, narrower shores, a free tilt.
        let without = PlanetEffects::of(&earth, Some(&system(Vec::new())));
        assert!(!without.stabilised);
        assert!((without.tide - 0.315).abs() < 0.02, "{without:?}");
        assert!((without.shore_factor - 0.75).abs() < 0.02, "{without:?}");
        assert_eq!(without.tilt(5.0, (5.0, 40.0)), 0.0);
        assert_eq!(without.tilt(40.0, (5.0, 40.0)), 60.0);
        assert_eq!(with.tilt(23.0, (5.0, 40.0)), 23.0);
        // A binary's moon raises a bulge that never moves.
        let locked = Moon {
            migration: Migration::Locked,
            ..moon
        };
        let binary = PlanetEffects::of(&earth, Some(&system(vec![locked])));
        assert!(binary.stabilised && (binary.tide - without.tide).abs() < 1e-6);
        // A planet locked to its star has no tides at all.
        let mut red = params(1, 40_075.0);
        red.star = crate::star::parse_type("M5V").unwrap();
        red.orbit_au = crate::star::orbit_at(red.star, 0.5);
        let still = PlanetEffects::of(&red, None);
        assert_eq!((still.tide, still.shore_factor), (0.0, 0.5));
    }

    #[test]
    fn a_number_asked_for_gives_that_many_stable_moons() {
        // Short of room now and then (a crowded Hill sphere): the request
        // then draws another orbit.
        let mut short = 0;
        for seed in 0..200 {
            for n in 1..=MAX_MOON_COUNT {
                let mut p = params(seed, 40_075.0);
                p.moons = MoonsMode::Count(n);
                let system = MoonSystem::draw(&p).unwrap();
                if system.moons.len() < usize::from(n) {
                    short += 1;
                    continue;
                }
                assert_eq!(system.moons.len(), usize::from(n), "seed {seed} count {n}");
                assert_eq!(system.count_asked, Some(n));
                for m in &system.moons {
                    assert!(
                        m.semi_major_axis_km * (1.0 + m.eccentricity) < 0.5 * system.hill_radius_km
                    );
                }
                // On a planet whose seed draws none, one moon is a large one,
                // as the Moon.
                let drawn = MoonSystem::draw(&params(seed, 40_075.0)).unwrap();
                if n == 1 && drawn.moons.is_empty() {
                    assert_eq!(system.moons[0].origin, MoonOrigin::GiantImpact);
                }
            }
        }
        assert!(short < 10, "{short} of 1200 systems short of moons");
        // The moons the seed draws come first: asking for as many keeps them.
        let mut p = params(19, 21_300.0);
        let auto = MoonSystem::draw(&p).unwrap();
        p.moons = MoonsMode::Count(auto.moons.len().max(1) as u8);
        let asked = MoonSystem::draw(&p).unwrap();
        if !auto.moons.is_empty() {
            assert_eq!(asked.moons, auto.moons);
        }
        assert_eq!("3".parse::<MoonsMode>(), Ok(MoonsMode::Count(3)));
        assert_eq!("0".parse::<MoonsMode>(), Ok(MoonsMode::None));
        assert!("7".parse::<MoonsMode>().is_err());
    }

    #[test]
    fn small_moons_are_misshapen_and_locked_planets_have_none() {
        let mut irregular = 0;
        for seed in 0..2000 {
            // Mars-sized.
            let system = MoonSystem::draw(&params(seed, 21_300.0)).unwrap();
            for m in system.moons.iter().filter(|m| !m.round) {
                irregular += 1;
                let [a, b, c] = m.axes_km;
                assert!(a >= b && b >= c && c > 0.3 * a, "{:?}", m.axes_km);
            }
        }
        assert!(irregular > 50, "{irregular}");
        // An Earth in the habitable zone of an M5V star is locked to it.
        let mut p = params(1, 40_075.0);
        p.star = crate::star::parse_type("M5V").unwrap();
        p.orbit_au = crate::star::orbit_at(p.star, 0.5);
        let locked = MoonSystem::draw(&p).unwrap();
        assert!(locked.moons.is_empty());
        // Turned off.
        p.moons = MoonsMode::None;
        assert!(MoonSystem::draw(&p).is_none());
        // Each moon is a world of its own: a small airless one, its map
        // sized like the planet's in km, irregular ones shaped.
        let mut planet = params(5, 21_300.0);
        planet.width = 2048;
        planet.height = 1024;
        let moons = (0..200)
            .filter_map(|seed| MoonSystem::draw(&params(seed, 21_300.0)))
            .flat_map(|s| s.moons)
            .collect::<Vec<_>>();
        let small = moons.iter().find(|m| !m.round).unwrap();
        let body = body_params(&planet, small);
        assert_eq!((body.width, body.height), (512, 256));
        assert_eq!(body.pressure_bar, 0.0);
        assert!(body.planet.geology.maria.is_none());
        let shape = shape_layer(&body).unwrap();
        assert_eq!(shape.len(), body.pixel_count());
        let (lo, hi) = shape
            .iter()
            .fold((f32::MAX, 0.0f32), |(a, b), &v| (a.min(v), b.max(v)));
        assert!(lo > 0.3 && hi < 1.8 && hi - lo > 0.05, "{lo} {hi}");
        // The long axis at the centre of the map, the short one at the pole.
        let centre = shape[(body.height / 2 * body.width + body.width / 2) as usize];
        assert!(centre > shape[(body.width / 2) as usize], "{centre}");
        // Deterministic.
        assert_eq!(
            MoonSystem::draw(&params(9, 21_300.0)),
            MoonSystem::draw(&params(9, 21_300.0))
        );
        assert_eq!(roman(14), "XIV");
    }
}
