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
//! at the planet's periapsis (half for retrograde moons, see
//! `PROGRADE_REACH`); captured moons keep out of the unstable
//! inclinations of 55° to 130°. Moons too close to a larger one are
//! dropped. A planet its star has despun (locked or in the 3:2 resonance,
//! see `crate::spin`) has its synchronous orbit beyond its Hill sphere, so
//! tides bring every moon down: it keeps only those light enough to
//! outlast the system's age (Barnes & O'Brien 2002, eq. 8).
//!
//! **Locking.** A moon is locked to its planet when the planet's tides
//! have despun it within the system's age, and the planet to its largest
//! moon when that moon's tides have despun it (`despin_gyr`, the
//! formula the planets' locking scales); a binary when their barycentre
//! lies outside the planet.
//!
//! **Shapes.** Moons smaller than 600 km across (rock) or 400 km (ice),
//! a mean radius of 300 and 200 km (Lineweaver & Norman 2010), are not
//! pulled round by their gravity: they get three axes, the more unequal
//! the smaller they are, equal at the limit. A locked moon is stretched by
//! its planet's tides into a triaxial ellipsoid, in full from the limit
//! up (`moon_axes`). The same rules apply to a giant's regular moons.
//!
//! **Migration.** A moon above the synchronous orbit recedes (the Moon), a
//! moon below it or on a retrograde orbit falls in (Phobos, Triton); a
//! binary is locked face to face and stays put, and far captured moons
//! hardly move.
//!
//! **Giants.** A giant's regular moons are drawn by [`MoonSystem::draw`]
//! for a giant world and a giant of a system alike (`of_giant`).
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
    /// Grown in a giant planet's circumplanetary disk, as the Galilean
    /// moons (a giant's regular moons).
    Disk,
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
    /// [`PlanetEffects`]).
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
/// The density of the planet `p` of `radius_km`, g/cm³ (1e12 kg/km³), from
/// its mass as `WorldParams::mass_earth` has it (the mass–radius relation
/// of rocky planets): denser the larger it is (Mars's size 3.5, Earth's
/// 5.5, twice Earth's 8.9).
fn planet_density(p: &WorldParams, radius_km: f64) -> f64 {
    EARTH_MASS_KG * f64::from(p.mass_earth()) / (4.0 / 3.0 * PI * radius_km.powi(3)) / 1e12
}
const SUN_MASS_KG: f64 = 1.989e30;
const AU_KM: f64 = 1.496e8;
/// Below this equilibrium temperature, K, moons formed from the debris of
/// a cold planet or captured from its neighbourhood are icy.
const SNOW_LINE_K: f32 = 150.0;

/// Odds of a giant-impact moon: on worlds up to `SMALL_WORLD` Earth radii,
/// and from `LARGE_WORLD` up, linear in between. The large-world odds are
/// Elser et al. 2011's "more than 1 in 12 terrestrial planets hosting a
/// massive moon, with a low-end estimate of 1 in 45 and a high-end
/// estimate of 1 in 4" (doi:10.1016/j.icarus.2011.05.025). They come from
/// simulations of Earth-like planets forming in the habitable zone of a
/// Sun-like star with Jupiter and Saturn (Morishima et al. 2010), and the
/// paper gives no dependence on the star, the planet's mass or its
/// distance: the app applies them round every star (an assumption, not a
/// result). The small-world odds, 1 in 2, are a calibration loosely on
/// Brown et al. 2006 (three of the four brightest Kuiper belt objects have
/// satellites, doi:10.1086/501524).
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
/// Odds of debris moonlets (no giant-impact moon), a calibration with no
/// source (no statistics exist: Mars is the one example, Canup & Salmon
/// 2018, doi:10.1126/sciadv.aar6887), and their size: Phobos (11 km)
/// around Mars (3 390 km), scaled with the planet.
const DEBRIS_ODDS: f64 = 0.2;
const DEBRIS_RADIUS: (f64, f64) = (0.4, 2.5);
const PHOBOS_OVER_MARS: f64 = 11.1 / 3390.0;
/// Odds of captured moons (a calibration with no source: none of the
/// rocky planets has one), their radius range, km, and their semi-major
/// axis range as a share of the Hill radius.
const CAPTURE_ODDS: f64 = 0.15;
const CAPTURED_RADIUS_KM: (f64, f64) = (1.0, 30.0);
const CAPTURED_ORBIT: (f64, f64) = (0.05, 0.3);
/// Semi-major axis range of the captured moons added to reach a number
/// asked for, as a share of the Hill radius: wider, so that several fit
/// (retrograde orbits stay stable out to half of it).
const COUNT_CAPTURED_ORBIT: (f64, f64) = (0.03, 0.45);
/// Diameter from which a body is round, km, for rock and ice: twice the
/// mean radius of about 300 km (rocky asteroids) and 200 km (icy moons)
/// where Lineweaver & Norman 2010 observe the change from irregular to
/// round shapes ("The potato radius", arXiv:1004.1091, Fig. 2).
const ROUND_ROCK_KM: f64 = 600.0;
const ROUND_ICE_KM: f64 = 400.0;
/// Outermost stable orbits, as a share of the Hill radius, for prograde
/// and retrograde moons: the cautious end of the published range. Barnes
/// & O'Brien 2002 take 0.36 and 0.50 (doi:10.1086/341477, after Holman &
/// Wiegert 1999 and the Solar System's moons); Shen & Tremaine 2008 find
/// that "many prograde and retrograde satellites can survive out to radii
/// ∼ 0.5rH and ∼ 0.7rH" (doi:10.1088/0004-6256/136/6/2453). The app loses
/// some far moons a real planet could keep, never the reverse.
const PROGRADE_REACH: f64 = 1.0 / 3.0;
const RETROGRADE_REACH: f64 = 0.5;
/// Inclinations to the planet's orbit between which a moon's orbit is
/// generally unstable, degrees (Carruba et al. 2002, as summarised by Shen
/// & Tremaine 2008): captured moons are drawn outside them.
const UNSTABLE_INCLINATION_DEG: (f64, f64) = (55.0, 130.0);
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
        is_round(self.composition, self.radius_km())
    }
}

/// Whether a body of `composition` and mean `radius_km` is pulled round by
/// its own gravity (see [`ROUND_ROCK_KM`]).
fn is_round(composition: Composition, radius_km: f64) -> bool {
    2.0 * radius_km >= round_limit_km(composition)
}

/// The diameter from which a body of `composition` is round, km.
fn round_limit_km(composition: Composition) -> f64 {
    match composition {
        Composition::Rock => ROUND_ROCK_KM,
        Composition::Ice => ROUND_ICE_KM,
    }
}

/// The three semi-axes of a moon of `composition` and mean `radius_km`, km,
/// the longest first (towards its planet when locked), their product the
/// mean radius cubed.
///
/// - **Irregular** under the round limit, the more so the smaller: an
///   irregularity of `1 − d / limit` (1 for a moonlet, 0 at the limit),
///   the middle axis shortened by 5 to 30 % of it and the short one by as
///   much again (`draws`, two uniform draws). Calibration, no source: the
///   real change is a band, not a line (Proteus stays lumpy above it,
///   Lineweaver & Norman 2010).
/// - **Stretched by its planet's tides** when it is synchronous (`locked`:
///   its mass, its planet's and its semi-major axis, kg and km), as far as
///   its gravity rules its shape (in full from the round limit, not at all
///   for a moonlet held by its strength): the hydrostatic ellipsoid of a
///   homogeneous fluid body (Love number `h₂ = 5/2`) under the tide and its
///   spin, to first order in `q = n² R³ / (G m)`: `a = R (1 + 35q/12)`,
///   `b = R (1 − 5q/6)`, `c = R (1 − 25q/12)`, so `(b − c) / (a − c) = 1/4`
///   (Noyelles 2017, eq. 4, doi:10.1016/j.icarus.2016.10.001; Murray &
///   Dermott 1999, ch. 4) and `a − c = 5qR`. Mimas (`q = 0.0185`, 198 km)
///   comes out 18 km longer than wide; measured, 207.8 × 196.7 × 190.6 km
///   (Thomas 2010, as tabulated by Noyelles, Karatekin & Rambaux 2011,
///   doi:10.1051/0004-6361/201117558).
fn moon_axes(
    composition: Composition,
    radius_km: f64,
    draws: [f64; 2],
    locked: Option<(f64, f64, f64)>,
) -> [f64; 3] {
    let limit = round_limit_km(composition);
    let irregular = (1.0 - 2.0 * radius_km / limit).clamp(0.0, 1.0);
    let b = 1.0 - irregular * (0.05 + 0.25 * draws[0]);
    let c = b - irregular * (0.05 + 0.25 * draws[1]);
    let mut axes = [1.0, b, c];
    if let Some((mass_kg, host_kg, a_km)) = locked {
        let n2 = G * (host_kg + mass_kg) / a_km.powi(3);
        let q = (n2 * radius_km.powi(3) / (G * mass_kg)).min(MAX_HYDROSTATIC_Q) * (1.0 - irregular);
        axes[0] *= 1.0 + 35.0 / 12.0 * q;
        axes[1] *= 1.0 - 5.0 / 6.0 * q;
        axes[2] *= 1.0 - 25.0 / 12.0 * q;
    }
    let scale = radius_km / (axes[0] * axes[1] * axes[2]).cbrt();
    axes.map(|x| x * scale)
}

/// Largest `q` the first-order hydrostatic shape is taken to: beyond it a
/// moon would be torn apart near its Roche limit (Mimas, among the most
/// stretched, has 0.0185). Calibration, no source.
const MAX_HYDROSTATIC_Q: f64 = 0.05;

/// Rigidity of rock, Pa, as Thirouin et al. 2014 take it from Gladman et
/// al. 1996 (doi:10.1051/0004-6361/201423567): used for ice too, which
/// makes icy moons lock a little slower than they would.
const RIGIDITY_PA: f64 = 3e10;
/// Tidal quality factor, Love number cap, first day and the moments of
/// inertia (over `m R²`) of the despinning time: those of
/// [`crate::star::locking_gyr`] (`Q = 100`, `k₂ = 0.3`, 12 h, Earth's 0.33)
/// for a planet, a uniform body's 0.4 for a moon (the Moon: 0.394, NASA).
const TIDAL_Q: f64 = 100.0;
const MAX_K2: f64 = 0.3;
const FIRST_DAY_S: f64 = 12.0 * 3600.0;
const PLANET_INERTIA: f64 = 0.33;
const MOON_INERTIA: f64 = 0.4;
const GYR_S: f64 = 3.156e16;

/// Tidal Love number of a solid body of `mass_kg`, `radius_km` and
/// `density` (g/cm³): `k₂ = 1.5 / (1 + 19μ / (2ρgR))` (Gladman et al. 1996,
/// as quoted by Thirouin et al. 2014, eq. 6), with the rigidity of rock,
/// at most Earth's 0.3: a small rigid body hardly deforms (Phobos: 6 ×
/// 10⁻⁷).
fn love_number(mass_kg: f64, radius_km: f64, density: f64) -> f64 {
    let g = G * mass_kg / (radius_km * radius_km) * 1000.0;
    let rho_g_r = density * 1000.0 * g * radius_km * 1000.0;
    (1.5 / (1.0 + 19.0 * RIGIDITY_PA / (2.0 * rho_g_r))).min(MAX_K2)
}

/// Time for a host of `host_kg` to despin a body of `mass_kg`, `radius_km`,
/// `density` (g/cm³) and moment of inertia `inertia` (over `m R²`) orbiting
/// it at `a_km`, Gyr: `t = ω a⁶ I Q / (3 G M² k₂ R⁵)`, the formula
/// [`crate::star::locking_gyr`] scales for planets (Gladman et al. 1996,
/// doi:10.1006/icar.1996.0117, as quoted by Thirouin et al. 2014, eq. 7),
/// from a first day of 12 h, with [`love_number`]. Phobos locks in about
/// 20 000 years; the Earth would take 35 Gyr to turn with the Moon, Pluto
/// a million years with Charon.
pub(crate) fn despin_gyr(
    mass_kg: f64,
    radius_km: f64,
    density: f64,
    inertia: f64,
    host_kg: f64,
    a_km: f64,
) -> f64 {
    let omega = 2.0 * PI / FIRST_DAY_S;
    let moment = inertia * mass_kg * radius_km * radius_km;
    let k2 = love_number(mass_kg, radius_km, density);
    omega * a_km.powi(6) * moment * TIDAL_Q
        / (3.0 * G * host_kg * host_kg * k2 * radius_km.powi(5))
        / GYR_S
}

/// The heaviest moon that outlasts `age_gyr` round a planet of `m_p` kg and
/// `r_p` km that its star (of `star_kg`, at `a_p_km`) has despun, kg: Barnes
/// & O'Brien 2002, eq. 8 (doi:10.1086/341477),
/// `M_m ≤ (2/13) ((f a_p)³ / 3M*)^(13/6) M_p^(8/3) Q / (3 k₂ T R⁵ √G)`,
/// a moon starting from the edge of the stable zone (`reach`, the share `f`
/// of the Hill radius) and spiralling in, with the planet's `Q` and `k₂` of
/// [`despin_gyr`]. Round an Earth in the habitable zone of an M5 dwarf
/// (0.07 AU), about 2 × 10¹⁸ kg, a moonlet of 60 km; round Mercury, a moon
/// of a few hundred km; never a giant-impact moon.
fn lasting_moon_kg(reach: f64, a_p_km: f64, star_kg: f64, m_p: f64, r_p: f64, age_gyr: f64) -> f64 {
    // The planet's own density, g/cm³ (1e12 kg/km³), as its mass and radius give it.
    let density = m_p / (4.0 / 3.0 * PI * r_p.powi(3)) / 1e12;
    let k2 = love_number(m_p, r_p, density);
    lasting_moon_kg_with(reach, a_p_km, star_kg, m_p, r_p, age_gyr, k2, TIDAL_Q)
}

/// [`lasting_moon_kg`] for a planet of Love number `k2` and tidal quality
/// factor `q`: a giant's ([`crate::spin::GIANT_K2`], [`crate::spin::GIANT_Q`]),
/// the case Barnes & O'Brien 2002 treat. A Jupiter at 0.12 AU from the Sun,
/// locked to it, keeps moons up to about 10²² kg, none as large as the
/// Galilean ones.
#[allow(clippy::too_many_arguments)]
fn lasting_moon_kg_with(
    reach: f64,
    a_p_km: f64,
    star_kg: f64,
    m_p: f64,
    r_p: f64,
    age_gyr: f64,
    k2: f64,
    q: f64,
) -> f64 {
    let t = age_gyr * GYR_S;
    2.0 / 13.0
        * ((reach * a_p_km).powi(3) / (3.0 * star_kg)).powf(13.0 / 6.0)
        * m_p.powf(8.0 / 3.0)
        * q
        / (3.0 * k2 * t * r_p.powi(5) * G.sqrt())
}

impl MoonSystem {
    /// The moons of world `p`, `None` when its request turned them off.
    pub fn draw(p: &WorldParams) -> Option<Self> {
        if p.moons == MoonsMode::None {
            return None;
        }
        if let Some(mass_earth) = p.giant_mass_earth {
            return Some(Self::of_giant(p, mass_earth));
        }
        let mut rng = Draws(u64::from(stream_seed(p.seed, stream::MOONS)));
        // The spin (despun, resonance) is the world's own, as its card and
        // `planet_body` read it, not an Earth's against this star.
        let star = crate::spin::star_info(p);
        let r_p = f64::from(p.circumference_km) / (2.0 * PI);
        let size = r_p / EARTH_RADIUS_KM;
        let m_p = EARTH_MASS_KG * f64::from(p.mass_earth());
        let planet_density = planet_density(p, r_p);
        let periapsis_km = f64::from(star.periapsis_au.max(1e-4)) * AU_KM;
        let hill = periapsis_km * (m_p / (3.0 * f64::from(star.mass) * SUN_MASS_KG)).cbrt();
        let cold = star.equilibrium_k < SNOW_LINE_K;
        let synchronous = |day_hours: f64| {
            let t = day_hours * 3600.0;
            (G * m_p * t * t / (4.0 * PI * PI)).cbrt()
        };
        // Drawn even on a despun planet, so a world's draws do not depend
        // on its star.
        let drawn_day = rng.log_uniform(12.0, 36.0);
        let age_gyr = f64::from(crate::spin::tidal_age_gyr(&star));
        // A planet its star has despun turns once a year, or three times
        // every two years in the 3:2 resonance. Its synchronous orbit then
        // lies beyond its Hill sphere: every moon spirals in, the heavy
        // ones first. Only those light enough to outlast the system's age
        // remain (Barnes & O'Brien 2002, eq. 8, see `lasting_moon_kg`).
        // A planet of the star is despun when `star_info` locks it or puts
        // it in a resonance, which also honours a stored world's lock and
        // resonance. A moon's world is never so against the star, and keeps
        // the despinning time alone.
        let bound_to_star = !is_moon_world(p) && crate::exomoon::HostGiant::draw(p).is_none();
        let despun = if bound_to_star {
            star.tidally_locked || star.spin_turns > 0.0
        } else {
            crate::spin::despun(&star)
        };
        let star_day = despun.then(|| {
            let turns = if star.spin_turns > 0.0 {
                f64::from(star.spin_turns)
            } else {
                1.0
            };
            f64::from(star.year_days) * 24.0 / turns
        });
        let lasting = |reach: f64| {
            if despun {
                lasting_moon_kg(
                    reach,
                    f64::from(star.orbit_au) * AU_KM,
                    f64::from(star.mass) * SUN_MASS_KG,
                    m_p,
                    r_p,
                    age_gyr,
                )
            } else {
                f64::INFINITY
            }
        };

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
            // Outside the unstable band, half of them retrograde.
            let inclination_deg = if rng.next() < 0.5 {
                rng.uniform(UNSTABLE_INCLINATION_DEG.1, 170.0)
            } else {
                rng.uniform(10.0, UNSTABLE_INCLINATION_DEG.0)
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
                2.44 * r_p * (planet_density / c.density).cbrt()
            } else {
                1.26 * r_p * (planet_density / c.density).cbrt()
            };
            let reach = if c.inclination_deg > 90.0 {
                RETROGRADE_REACH
            } else {
                PROGRADE_REACH
            };
            c.a_km * (1.0 - c.eccentricity) > roche
                && c.a_km * (1.0 + c.eccentricity) < reach * hill
                && c.mass_kg <= lasting(reach)
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
                    if c.inclination_deg < 90.0
                        && c.a_km * (1.0 + c.eccentricity) >= PROGRADE_REACH * hill
                    {
                        c.inclination_deg =
                            180.0 - c.inclination_deg.min(180.0 - UNSTABLE_INCLINATION_DEG.1);
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
        // The planet turns with its largest moon when that moon's tides
        // have despun it within the system's age (the same despinning time
        // as the planets', with Earth's moment of inertia); a binary when
        // their barycentre lies outside the planet, as Pluto and Charon.
        // The star's tides despin a planet first: its moons are light.
        let largest = kept
            .iter()
            .max_by(|a, b| a.mass_kg.total_cmp(&b.mass_kg))
            .filter(|c| {
                !despun
                    && despin_gyr(m_p, r_p, planet_density, PLANET_INERTIA, c.mass_kg, c.a_km)
                        < age_gyr
            });
        let binary = largest.is_some_and(|c| c.a_km * c.mass_kg / (m_p + c.mass_kg) > r_p);
        let day_hours =
            star_day.unwrap_or_else(|| largest.map_or(drawn_day, |c| period_days(c) * 24.0));
        let sync_km = synchronous(day_hours);
        let seed = p.seed;

        let moons = kept
            .iter()
            .enumerate()
            .map(|(k, c)| {
                let radius = c.radius_km();
                let round = c.round();
                let is_largest = largest.is_some_and(|l| std::ptr::eq(l, c));
                let locked = is_largest
                    || despin_gyr(c.mass_kg, radius, c.density, MOON_INERTIA, m_p, c.a_km)
                        < age_gyr;
                let axes = moon_axes(
                    c.composition,
                    radius,
                    c.shape,
                    locked.then_some((c.mass_kg, m_p, c.a_km)),
                );
                let migration = if is_largest {
                    Migration::Locked
                } else if despun || c.inclination_deg > 90.0 {
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
                    name: crate::naming::roman(k + 1),
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
                    tidally_locked: locked,
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

    /// The moons of a giant world: regular moons grown in its disk, about
    /// 10⁻⁴ of its mass together (Canup & Ward 2006), on a resonant chain
    /// from 4 to 8 of its radii outwards (periods doubling, as Io, Europa
    /// and Ganymede), rock close in and ice and rock beyond about 9 radii
    /// (Europa's and Ganymede's change); two to five of them, or as many as
    /// asked; on some ice giants a large retrograde moon captured instead
    /// (Triton, which scattered the regular ones, left closer in). None
    /// orbits inside the giant's Roche limit or its rings (see
    /// [`giant_moon_floor_radii`]), and the chain ends at a third of the
    /// Hill radius (as many as fit, fewer than asked if need be). A hot
    /// giant keeps none: its Hill sphere is too small and its tides drag
    /// them in; one its star has locked keeps only the moons light enough
    /// to outlast the system's age (Barnes & O'Brien 2002).
    fn of_giant(p: &WorldParams, mass_earth: f32) -> Self {
        let mut rng = Draws(u64::from(stream_seed(p.seed, stream::MOONS)));
        let star = StarInfo::new(p.star, p.orbit_au, p.eccentricity, p.age_gyr);
        let look = crate::giant_world::Look::draw(p, mass_earth).info;
        let r_g = f64::from(p.circumference_km) / (2.0 * PI);
        let m_g = f64::from(mass_earth) * EARTH_MASS_KG;
        let periapsis_km = f64::from(star.periapsis_au.max(1e-4)) * AU_KM;
        let hill = periapsis_km * (m_g / (3.0 * f64::from(star.mass) * SUN_MASS_KG)).cbrt();
        let day = f64::from(look.rotation_hours);
        let synchronous = {
            let t = day * 3600.0;
            (G * m_g * t * t / (4.0 * PI * PI)).cbrt()
        };
        let asked = p.moons.count();
        let drawn = 2 + (rng.next() * 4.0) as usize;
        let captured_triton = look.kind == crate::giants::GiantKind::Ice && rng.next() < 0.35;
        let first = rng.uniform(4.0, 8.0);
        let mut moons: Vec<Moon> = Vec::new();
        if !look.hot {
            let count = asked.map_or(drawn, usize::from);
            // Masses sharing 1e-4 of the giant's, unevenly.
            let weights: Vec<f64> = (0..count).map(|_| rng.uniform(0.3, 1.7)).collect();
            let total: f64 = weights.iter().sum();
            let ice_line = rng.uniform(8.0, 11.0);
            // A captured giant moon sweeps the outer disk: the regular
            // moons left keep inside it, but outside the Roche limit and
            // the rings (only those chains move: an uncaptured chain
            // starts beyond both).
            let floor = giant_moon_floor_radii(
                f64::from(look.density),
                look.rings.as_ref().map(|r| f64::from(r.outer)),
            );
            let base = if captured_triton {
                (first * 0.5).max(floor / 0.95)
            } else {
                first.max(floor / 0.95)
            };
            // Triton's orbit, 12 to 16 radii out: the regular moons left
            // keep inside it (1.2 times closer at least).
            let triton_radii = captured_triton.then(|| rng.uniform(12.0, 16.0));
            for (k, w) in weights.iter().enumerate() {
                let a_radii = base * 1.587_401f64.powi(k as i32) * rng.uniform(0.95, 1.05);
                // The chain ends at the edge of the stable zone (with room
                // for the moon's eccentricity, at most 0.01): the moons
                // beyond would not last.
                let a = a_radii * r_g;
                if a * 1.01 >= PROGRADE_REACH * hill
                    || triton_radii.is_some_and(|t| asked.is_none() && a_radii * 1.2 > t)
                {
                    break;
                }
                let mass = 1e-4 * m_g * w / total * if captured_triton { 0.02 } else { 1.0 };
                let icy = a_radii > ice_line || star.equilibrium_k < 60.0;
                let density = if icy { 1.9 } else { 3.3 };
                moons.push(regular_moon(
                    p,
                    &mut rng,
                    m_g,
                    r_g,
                    a,
                    mass,
                    density,
                    icy,
                    synchronous,
                    MoonOrigin::Disk,
                    0.3,
                ));
            }
            if captured_triton && asked.is_none() {
                // Triton: 2.4 times its mass in a retrograde orbit at 14 radii.
                let a = triton_radii.unwrap_or(14.0) * r_g;
                let mass = rng.uniform(1.5e-4, 3e-4) * m_g;
                let triton = regular_moon(
                    p,
                    &mut rng,
                    m_g,
                    r_g,
                    a,
                    mass,
                    2.06,
                    true,
                    synchronous,
                    MoonOrigin::Captured,
                    157.0,
                );
                // Within the retrograde stable zone.
                if a * 1.01 < RETROGRADE_REACH * hill {
                    moons.push(triton);
                }
            }
        }
        // A giant its star has locked (a hot one keeps none at all) turns
        // once a year: its synchronous orbit lies beyond its Hill sphere and
        // its moons spiral in. Only those light enough to outlast the
        // system's age remain (Barnes & O'Brien 2002, eq. 8, with the
        // giant's own tides).
        if crate::spin::giant_locked(&star, mass_earth, look.radius_earth) {
            let age = f64::from(crate::spin::tidal_age_gyr(&star));
            let a_p = f64::from(star.orbit_au) * AU_KM;
            let star_kg = f64::from(star.mass) * SUN_MASS_KG;
            moons.retain(|m| {
                let reach = if m.inclination_deg > 90.0 {
                    RETROGRADE_REACH
                } else {
                    PROGRADE_REACH
                };
                let limit = lasting_moon_kg_with(
                    reach,
                    a_p,
                    star_kg,
                    m_g,
                    r_g,
                    age,
                    crate::spin::GIANT_K2,
                    crate::spin::GIANT_Q,
                );
                f64::from(m.mass_ratio) * m_g <= limit
            });
            for m in &mut moons {
                m.migration = Migration::Decaying;
            }
        }
        moons.sort_by(|a, b| a.semi_major_axis_km.total_cmp(&b.semi_major_axis_km));
        for (k, moon) in moons.iter_mut().enumerate() {
            moon.name = crate::naming::roman(k + 1);
        }
        Self {
            planet_day_hours: day as f32,
            synchronous_orbit_km: synchronous as f32,
            hill_radius_km: hill as f32,
            binary: false,
            // A giant has no shores for tides to set, and its tilt is its own.
            tide: 0.0,
            tilt_steadied: false,
            count_asked: asked,
            moons,
        }
    }

    /// The system with what it does to its planet filled in.
    fn with_effects(mut self, p: &WorldParams) -> Self {
        let effects = PlanetEffects::of(p, Some(&self));
        self.tide = effects.tide;
        self.tilt_steadied = effects.stabilised;
        self
    }
}

/// Density of a giant's icy regular moon, g/cm³ (Ganymede 1.94, Titan
/// 1.88): the least dense, so the farthest Roche limit.
const ICY_MOON_DENSITY: f64 = 1.9;
/// Room kept between a giant's rings and its first regular moon, in the
/// giant's radii (Mimas, the closest of Saturn's, orbits 0.8 beyond the A
/// ring; the gap holds only small ring moons).
const RING_CLEARANCE_RADII: f64 = 0.3;

/// The closest a giant's regular moon may orbit, in the giant's radii:
/// beyond the fluid Roche limit of an icy moon round a giant of
/// `giant_density` (g/cm³), `2.44 (ρ_giant / ρ_moon)^(1/3)`, and beyond
/// its rings' outer edge (`ring_outer`, radii) with some room.
pub(crate) fn giant_moon_floor_radii(giant_density: f64, ring_outer: Option<f64>) -> f64 {
    let roche = 2.44 * (giant_density / ICY_MOON_DENSITY).cbrt();
    ring_outer.map_or(roche, |outer| roche.max(outer + RING_CLEARANCE_RADII))
}

/// Mass ratio from which a moon stabilises its planet's axial tilt (the
/// Moon: 0.0123). The app's choice, between the Moon and a moon too small
/// to matter: the studies computed only Earth with and without its Moon.
/// With the Moon, Earth's tilt is "essentially stable, exhibiting only
/// small variations of ±1.3° around the mean value of 23.3°" (Laskar,
/// Joutel & Robutel 1993, doi:10.1038/361615a0).
const STABILISING_MASS_RATIO: f64 = 0.005;
/// Tidal height of the Moon and of the Sun on Earth, in the units of
/// [`tidal_height`] (`mass ratio × (radius / distance)³ × radius in Earth
/// radii`): the Sun's is 0.46 of the Moon's.
const EARTH_MOON_TIDE: f64 = 0.0123 / (60.27 * 60.27 * 60.27);
const EARTH_SUN_TIDE: f64 = 0.46 * EARTH_MOON_TIDE;
/// Share of a giant's still tidal bulge that an eccentric orbit swings,
/// per unit of eccentricity (the radial tide varies as `3e`).
const GIANT_TIDE_SWING: f64 = 3.0;
/// Range of the shore width, as a share of the look's: no tides (a
/// planet locked to its star) to strong ones. It follows the fourth root of
/// the tides: flats widen with their range, but far more with the slope of
/// the coast, which the tides do not change.
const SHORE_FACTOR: (f64, f64) = (0.5, 2.0);
/// Axial tilt range drawn without a stabilising moon, degrees, in place of
/// the usual `[5, 40]`. Without the Moon, Earth's "chaotic zone would then
/// extend from nearly 0° up to about 85°" (Laskar, Joutel & Robutel 1993,
/// doi:10.1038/361615a0); Mars, without a large moon, ranges from 0 to 60°
/// (Laskar & Robutel 1993, doi:10.1038/361608a0). The range is where a
/// moonless tilt can be found, not how fast it gets there: over hundreds
/// of millions of years a moonless Earth's tilt stays "within a
/// constrained range, typically 20–25° in extent" (Lissauer, Barnes &
/// Chambers 2012, doi:10.1016/j.icarus.2011.10.013). The app draws one
/// tilt for the world's whole life, anywhere in the chaotic zone.
pub const UNSTABLE_TILT_DEG: (f32, f32) = (0.0, 85.0);

/// What a world's moons do to it (step 5 of backlog item 13).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanetEffects {
    /// A large moon (or, for a giant's moon, the giant) keeps the axial
    /// tilt steady.
    pub stabilised: bool,
    /// The range of the tides (moons and star), relative to Earth's.
    pub tide: f32,
    /// The look's shore width is multiplied by this: tidal flats widen
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
        // A rocky world's lock is the one `spin::star_info` gives it, so a
        // stored or resonant world agrees with its card.
        let star = if p.giant_mass_earth.is_none() {
            crate::spin::star_info(p)
        } else {
            StarInfo::new(p.star, p.orbit_au, p.eccentricity, p.age_gyr)
        };
        let r_p = f64::from(p.circumference_km) / (2.0 * PI);
        let size = r_p / EARTH_RADIUS_KM;
        let m_p = EARTH_MASS_KG * f64::from(p.mass_earth());
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
        // The moon of a giant (step C of item 14) turns with the giant, not
        // the star: the star's tide comes and goes with its month. Locked to
        // the giant, it feels the giant's huge bulge as a still one; only
        // the part its eccentric orbit swings (about `3e` of it, as Io's
        // flexes) comes and goes. Its equator lies in the giant's equatorial
        // plane, the giant holding its spin axis: its seasons follow the
        // giant's tilt, drawn in the usual range, never left free.
        let host = crate::exomoon::HostGiant::draw(p);
        let from_star = if star.tidally_locked && host.is_none() {
            0.0
        } else {
            let star_ratio = f64::from(star.mass) * SUN_MASS_KG / m_p;
            tidal_height(star_ratio, f64::from(star.orbit_au) * AU_KM / r_p, size)
        };
        let from_giant = host.as_ref().map_or(0.0, |h| {
            let giant_ratio = f64::from(h.giant.mass_earth) * EARTH_MASS_KG / m_p;
            let still = tidal_height(giant_ratio, f64::from(h.distance_km) / r_p, size);
            GIANT_TIDE_SWING * f64::from(h.eccentricity) * still
        });
        let stabilised = stabilised || host.is_some();
        let tide = (from_moons + from_star + from_giant) / (EARTH_MOON_TIDE + EARTH_SUN_TIDE);
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
/// Axial tilt of a giant-impact moon to its orbit, degrees: the Moon's
/// obliquity to its orbit, 6.68° (NASA Moon fact sheet), in the Cassini
/// state its tides hold it in (its spin axis, its orbit's and the
/// ecliptic's in one plane). It tips the planet north and south in the
/// moon's sky over a month: the libration in latitude.
pub const MOON_TILT_DEG: f32 = 6.68;
/// Axial tilt of the other moons to their orbits, degrees: upright.
/// Calibration, no source: close moons and a giant's regular moons orbit
/// in their planet's equator, where no Cassini state as tilted as the
/// Moon's is known to the app.
pub const CLOSE_MOON_TILT_DEG: f32 = 0.0;

/// The axial tilt of `moon` to its orbit, degrees: its world's and its
/// spin's in a system (see [`MOON_TILT_DEG`]).
pub fn moon_tilt_deg(moon: &Moon) -> f32 {
    match moon.origin {
        MoonOrigin::GiantImpact => MOON_TILT_DEG,
        _ => CLOSE_MOON_TILT_DEG,
    }
}
/// Low-frequency bumps of an irregular body, as a share of its radius at
/// full irregularity (equal axes add none).
const SHAPE_BUMPS: f64 = 0.12;

/// The world of `moon`, one of the moons of the world `planet`: a body of
/// the `rocky-moon` or `icy-moon` look, airless and never wet, cratered
/// for 4 Ga, under the planet's star, its map as many pixels per km as the
/// planet's (at least [`MOON_MIN_WIDTH`] wide, at most the planet's
/// width), tilted by [`moon_tilt_deg`]. The long axis of an irregular or
/// tidally stretched moon points at longitude 180°, the centre of its map:
/// towards its planet, which tides lock it to. Its gravity is the moon's
/// own, from the density it was drawn with.
pub fn body_params(planet: &WorldParams, moon: &Moon) -> WorldParams {
    let [a, _, c] = moon.axes_km;
    let mut params = small_body_params(
        planet,
        moon.composition,
        moon.radius_km,
        moon.seed,
        (a > c).then(|| moon.axes_km.map(|x| x / moon.radius_km)),
    );
    params.axial_tilt_deg = moon_tilt_deg(moon);
    params.gravity = Some(moon.gravity);
    params
}

/// Whether `p` is the world of a moon of a system (the `rocky-moon` or
/// `icy-moon` class, see [`small_body_params`]).
pub fn is_moon_world(p: &WorldParams) -> bool {
    matches!(p.class.as_str(), "rocky-moon" | "icy-moon")
}

/// Age of the oldest surviving surface of a moon, Ga: impacts since it
/// formed, as the moon looks (`looks/moons/*.toml`) had it; both kinds the
/// age of the Solar System (4.57 Ga) within a few per cent.
/// Calibration, no source: reproduces the former `surface_age` of the
/// `rocky-moon` and `icy-moon` looks.
const ICY_MOON_SURFACE_AGE_GA: f32 = 4.0;
const ROCKY_MOON_SURFACE_AGE_GA: f32 = 4.0;

/// The surface age of a moon of `composition`, Ga.
fn moon_surface_age_ga(composition: Composition) -> f32 {
    match composition {
        Composition::Rock => ROCKY_MOON_SURFACE_AGE_GA,
        Composition::Ice => ICY_MOON_SURFACE_AGE_GA,
    }
}

/// Bulk density of a small body of `composition` whose own is not given,
/// g/cm³: the giants' regular moons' (rock 3.3, ice and rock 1.9).
fn small_body_density(composition: Composition) -> f64 {
    match composition {
        Composition::Rock => 3.3,
        Composition::Ice => 1.9,
    }
}

/// The parameters of a small airless body near the world `planet`, as a
/// world of its own: a rocky or icy moon of `radius_km` (its map as wide
/// as its size makes it on the planet's scale, within limits), its seed,
/// and its shape (semi-axes over the mean radius) when it is not a sphere;
/// upright ([`CLOSE_MOON_TILT_DEG`]). Its gravity is that of its radius at
/// the density of its composition ([`small_body_density`]); a caller that
/// drew another density sets its own.
pub fn small_body_params(
    planet: &WorldParams,
    composition: Composition,
    radius_km: f32,
    seed: u32,
    shape: Option<[f32; 3]>,
) -> WorldParams {
    let mut look = crate::look::Look::moon(composition);
    if radius_km < MARIA_MIN_RADIUS_KM {
        look.geology.maria = None;
    }
    let ratio = f64::from(radius_km) * 2.0 * PI / f64::from(planet.circumference_km);
    let widest = planet.width.max(MOON_MIN_WIDTH);
    let width =
        ((f64::from(planet.width) * ratio).round() as u32).clamp(MOON_MIN_WIDTH, widest) & !1;
    // A surface is never older than its system.
    let surface_age_ga = moon_surface_age_ga(composition).min(planet.age_gyr);
    WorldParams {
        width,
        height: width / 2,
        seed,
        class: crate::look::Look::moon_name(composition).to_string(),
        archetype: None,
        model: None,
        look,
        sea_level: 0.0,
        volcanic_intensity: 0.0,
        circumference_km: (f64::from(radius_km) * 2.0 * PI) as f32,
        axial_tilt_deg: CLOSE_MOON_TILT_DEG,
        erosion: 0.0,
        pressure_bar: 0.0,
        surface_age_ga,
        star: planet.star,
        orbit_au: planet.orbit_au,
        eccentricity: planet.eccentricity,
        age_gyr: planet.age_gyr,
        drift_ma: 0.0,
        moons: MoonsMode::None,
        shape,
        metallicity: planet.metallicity,
        giants: crate::giants::GiantsMode::None,
        giant_mass_earth: None,
        host: crate::exomoon::HostMode::None,
        given_rings: None,
        impact_factor: None,
        locked_to_star: None,
        ridge: None,
        perihelion_longitude_deg: None,
        resonance_to_star: None,
        // G ρ (4/3 π r³) / r², as the moons' cards have it.
        gravity: Some(
            (G * 4.0 / 3.0
                * PI
                * f64::from(radius_km)
                * small_body_density(composition)
                * 1e12
                * 1000.0
                / 9.81) as f32,
        ),
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
    let radius = ellipsoid * (1.0 + SHAPE_BUMPS * irregular * bumps);
    Some((radius * p.ridge.map_or(1.0, |h| ridge_scale(f64::from(h), v[2]))) as f32)
}

/// Half-width of an equatorial ridge, in the sine of the latitude (about
/// 17 degrees): a calibration, no source gives it for Pan or Atlas, whose
/// ridges are "roughly symmetric about the equator" and hold 10 to 25 % of
/// their volume (Charnoz et al. 2007, doi:10.1126/science.1148631).
const RIDGE_WIDTH: f64 = 0.3;

/// The factor a ridge of height `h` (a share of the radius) gives the
/// radius at `sin_lat`: `1 + h exp(-(sin lat / w)²)`, over the cube root of
/// the volume it adds, so that the body keeps its volume (the mean radius
/// of its card): the volume gained is `½ ∫ ((1 + h g)³ - 1) d sin lat`,
/// in closed form for a narrow ridge.
fn ridge_scale(h: f64, sin_lat: f64) -> f64 {
    let w = RIDGE_WIDTH;
    let gain = 0.5
        * w
        * (3.0 * h * PI.sqrt() + 3.0 * h * h * (PI / 2.0).sqrt() + h.powi(3) * (PI / 3.0).sqrt());
    (1.0 + h * (-(sin_lat / w).powi(2)).exp()) / (1.0 + gain).cbrt()
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

/// A moon of a giant on a near-circular orbit of semi-major axis `a` km
/// (inclination `inclination` degrees, retrograde above 90): round from the
/// limits of [`ROUND_ROCK_KM`] and [`ROUND_ICE_KM`], irregular below, and
/// locked to the giant when its tides have despun it within the system's
/// age ([`despin_gyr`]).
#[allow(clippy::too_many_arguments)]
fn regular_moon(
    p: &WorldParams,
    rng: &mut Draws,
    m_g: f64,
    r_g: f64,
    a: f64,
    mass: f64,
    density: f64,
    icy: bool,
    synchronous: f64,
    origin: MoonOrigin,
    inclination: f64,
) -> Moon {
    let age_gyr = f64::from(crate::spin::tidal_age_gyr(&StarInfo::new(
        p.star,
        p.orbit_au,
        p.eccentricity,
        p.age_gyr,
    )));
    let radius = (3.0 * mass / (4.0 * PI * density * 1e12)).cbrt();
    let period = 2.0 * PI * (a.powi(3) / (G * (m_g + mass))).sqrt() / 86_400.0;
    let retrograde = inclination > 90.0;
    let composition = if icy {
        Composition::Ice
    } else {
        Composition::Rock
    };
    let locked = despin_gyr(mass, radius, density, MOON_INERTIA, m_g, a) < age_gyr;
    let eccentricity = rng.uniform(0.0, 0.01);
    let inclination_deg = (inclination + rng.uniform(-0.3, 0.3)).max(0.0);
    let seed = (rng.next() * f64::from(u32::MAX)) as u32;
    let axes = moon_axes(
        composition,
        radius,
        [rng.next(), rng.next()],
        locked.then_some((mass, m_g, a)),
    );
    Moon {
        name: String::new(),
        origin,
        composition,
        radius_km: radius as f32,
        axes_km: axes.map(|x| x as f32),
        round: is_round(composition, radius),
        mass_ratio: (mass / m_g) as f32,
        density: density as f32,
        gravity: (G * mass / (radius * radius) * 1000.0 / 9.81) as f32,
        semi_major_axis_km: a as f32,
        semi_major_axis_radii: (a / r_g) as f32,
        eccentricity: eccentricity as f32,
        inclination_deg: inclination_deg as f32,
        period_days: period as f32,
        tidally_locked: locked,
        migration: if retrograde || a < synchronous {
            Migration::Decaying
        } else {
            Migration::Receding
        },
        angular_diameter_deg: (2.0 * (radius / (a - r_g)).atan()).to_degrees() as f32,
        barycentre_km: (a * mass / (m_g + mass)) as f32,
        seed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::look::Look;

    fn params(seed: u32, circumference_km: f32) -> WorldParams {
        WorldParams {
            age_gyr: crate::star::DEFAULT_AGE_GYR,
            width: 64,
            height: 32,
            seed,
            class: "rocky".into(),
            archetype: None,
            model: None,
            look: Look::load("terran"),
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
            giant_mass_earth: None,
            host: crate::exomoon::HostMode::None,
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
    fn a_moon_has_a_moon_look_and_no_model() {
        let planet = params(3, 40_000.0);
        for (composition, class) in [
            (Composition::Rock, "rocky-moon"),
            (Composition::Ice, "icy-moon"),
        ] {
            let moon = small_body_params(&planet, composition, 300.0, 7, None);
            assert_eq!(moon.class, class);
            assert!(is_moon_world(&moon));
            assert!(moon.model.is_none() && moon.archetype.is_none());
            let look = Look::moon(composition);
            for b in crate::biome::Biome::ALL {
                assert_eq!(moon.look.remap(b), look.remap(b), "{b:?}");
            }
            assert_eq!(moon.surface_age_ga, 4.0);
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
                        == (2.0 * m.radius_km >= 600.0 && m.composition == Composition::Rock
                            || 2.0 * m.radius_km >= 400.0 && m.composition == Composition::Ice)
                );
            }
        }
        let share = large as f64 / f64::from(n);
        assert!((0.06..0.11).contains(&share), "{share}");
        // Most planets have no moon; Earth-sized ones rarely a binary (a
        // heavy moon close enough to have despun its planet, the
        // barycentre outside it).
        assert!(any < n as usize / 2, "{any}");
        assert!(binaries < n as usize / 100, "{binaries}");
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
        assert_eq!(without.tilt(40.0, (5.0, 40.0)), 85.0);
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
    fn a_giants_moon_keeps_its_tilt_and_feels_the_swing_of_the_giants_tide() {
        let mut p = params(4, 40_075.0);
        p.moons = MoonsMode::None;
        let alone = PlanetEffects::of(&p, None);
        p.host = crate::exomoon::HostMode::Giant;
        let host = crate::exomoon::HostGiant::draw(&p).unwrap();
        let moon = PlanetEffects::of(&p, None);
        // Its tilt follows the giant's, in the usual range.
        assert!(moon.stabilised && !alone.stabilised);
        assert_eq!(moon.tilt(37.0, (5.0, 40.0)), 37.0);
        // The giant's bulge is still; its eccentric swing is tide enough.
        let giant_ratio = f64::from(host.giant.mass_earth) / 1.0;
        let still = giant_ratio / (f64::from(host.distance_km) / 6378.0).powi(3);
        let swing = 3.0 * f64::from(host.eccentricity) * still / (EARTH_MOON_TIDE + EARTH_SUN_TIDE);
        assert!(
            (f64::from(moon.tide - alone.tide) - swing).abs() < 0.05 * swing.max(1e-3),
            "{moon:?} {alone:?} {swing}"
        );
        assert!(moon.tide > alone.tide && moon.shore_factor >= alone.shore_factor);
        // Nearly circular, it stays far below the still bulge.
        assert!(f64::from(moon.tide) < still / (EARTH_MOON_TIDE + EARTH_SUN_TIDE));
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
        assert!(
            locked.moons.iter().all(|m| m.mass_ratio < 1e-6),
            "{locked:?}"
        );
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
        assert!(body.look.geology.maria.is_none());
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
    }

    #[test]
    fn giants_have_regular_moons_on_a_resonant_chain_and_hot_ones_none() {
        let giant = |preset: &str, seed: u32, mass: f32, orbit_au: f32, moons: MoonsMode| {
            let mut p = params(seed, 40_075.0);
            p.look = crate::look::Look::load(preset);
            p.giant_mass_earth = Some(mass);
            p.orbit_au = orbit_au;
            p.moons = moons;
            p.circumference_km = crate::giant_world::radius_earth(&p, mass) * 40_075.0;
            p
        };
        let (mut triton, mut disk_systems) = (0, 0);
        for seed in 0..200 {
            let p = giant("gas-giant", seed, 317.8, 5.2, MoonsMode::Auto);
            let system = MoonSystem::draw(&p).unwrap();
            assert!(
                (2..=5).contains(&system.moons.len()),
                "{}",
                system.moons.len()
            );
            let total: f32 = system.moons.iter().map(|m| m.mass_ratio).sum();
            assert!((total - 1e-4).abs() < 1e-6, "{total}");
            for pair in system.moons.windows(2) {
                let ratio = pair[1].period_days / pair[0].period_days;
                assert!((1.6..2.4).contains(&ratio), "{ratio}");
            }
            // Galilean sizes.
            assert!(
                system
                    .moons
                    .iter()
                    .all(|m| (300.0..3500.0).contains(&m.radius_km))
            );
            assert!(
                system
                    .moons
                    .iter()
                    .all(|m| m.origin == MoonOrigin::Disk && m.tidally_locked)
            );
            disk_systems += 1;
            let ice =
                MoonSystem::draw(&giant("ice-giant", seed, 17.0, 20.0, MoonsMode::Auto)).unwrap();
            triton += usize::from(
                ice.moons
                    .iter()
                    .any(|m| m.origin == MoonOrigin::Captured && m.inclination_deg > 90.0),
            );
        }
        assert_eq!(disk_systems, 200);
        assert!((40..100).contains(&triton), "{triton}");
        // As many as asked; none for a hot Jupiter.
        let asked =
            MoonSystem::draw(&giant("gas-giant", 1, 317.8, 5.2, MoonsMode::Count(6))).unwrap();
        assert_eq!(asked.moons.len(), 6);
        let hot = MoonSystem::draw(&giant("hot-jupiter", 1, 300.0, 0.04, MoonsMode::Auto)).unwrap();
        assert!(hot.moons.is_empty());
    }

    #[test]
    fn giant_moons_keep_outside_the_roche_limit_and_the_rings() {
        let mut checked = 0;
        for (preset, mass, orbit_au) in [("ice-giant", 17.0, 20.0), ("gas-giant", 3000.0, 5.2)] {
            for seed in 0..200 {
                let mut p = params(seed, 40_075.0);
                p.look = crate::look::Look::load(preset);
                p.giant_mass_earth = Some(mass);
                p.orbit_au = orbit_au;
                p.circumference_km = crate::giant_world::radius_earth(&p, mass) * 40_075.0;
                let r_g = f64::from(p.circumference_km) / (2.0 * PI);
                let look = crate::giant_world::Look::draw(&p, mass).info;
                let floor = giant_moon_floor_radii(
                    f64::from(look.density),
                    look.rings.as_ref().map(|r| f64::from(r.outer)),
                );
                let system = MoonSystem::draw(&p).unwrap();
                for moon in &system.moons {
                    let periapsis =
                        f64::from(moon.semi_major_axis_km) * (1.0 - f64::from(moon.eccentricity));
                    assert!(
                        periapsis / r_g > floor * 0.99,
                        "{preset} seed {seed}: {} < {floor}",
                        periapsis / r_g
                    );
                    checked += 1;
                }
            }
        }
        assert!(checked > 600, "{checked}");
    }

    #[test]
    fn a_stored_lock_sets_the_moon_system_s_day() {
        // A world stored as locked turns once a year, even where its star
        // would not despin it now; one stored as free keeps a day of its
        // own (drawn, or its largest moon's), even where its star would.
        let mut earth = params(1, 40_075.0);
        earth.locked_to_star = Some(true);
        let year_h = crate::spin::star_info(&earth).year_days * 24.0;
        let day = MoonSystem::draw(&earth).unwrap().planet_day_hours;
        assert!(
            (day - year_h).abs() < 1e-3 * year_h,
            "{day} h, year {year_h} h"
        );

        let mut dwarf = params(1, 40_075.0);
        dwarf.star = crate::star::parse_type("M5V").unwrap();
        dwarf.orbit_au = crate::star::orbit_at(dwarf.star, 0.5);
        assert!(crate::spin::despun(&crate::spin::star_info(&dwarf)));
        dwarf.locked_to_star = Some(false);
        dwarf.resonance_to_star = Some(0.0);
        let year_h = crate::spin::star_info(&dwarf).year_days * 24.0;
        let day = MoonSystem::draw(&dwarf).unwrap().planet_day_hours;
        assert!(
            (day - year_h).abs() > 0.01 * year_h,
            "{day} h, year {year_h} h"
        );
    }

    #[test]
    fn planets_despun_by_their_star_keep_only_light_moons() {
        // An Earth in the habitable zone of an M5 dwarf: never a large moon,
        // now and then a moonlet that outlasts the system.
        let (mut kept, mut light) = (0, 0);
        for seed in 0..400 {
            let mut p = params(seed, 40_075.0);
            p.star = crate::star::parse_type("M5V").unwrap();
            p.orbit_au = crate::star::orbit_at(p.star, 0.5);
            let system = MoonSystem::draw(&p).unwrap();
            let year_h = crate::star::StarInfo::new(p.star, p.orbit_au, p.eccentricity, p.age_gyr)
                .year_days
                * 24.0;
            assert!((system.planet_day_hours - year_h).abs() < 1e-3 * year_h);
            assert!(!system.binary);
            for m in &system.moons {
                kept += 1;
                light += usize::from(m.mass_ratio < 1e-6);
                assert_eq!(m.migration, Migration::Decaying, "{m:?}");
            }
        }
        assert!(kept > 0 && light == kept, "{light} of {kept}");
        // Barnes & O'Brien's limit grows as a^6.5.
        let star_kg = 0.16 * SUN_MASS_KG;
        let near = lasting_moon_kg(1.0 / 3.0, 0.05 * AU_KM, star_kg, EARTH_MASS_KG, 6371.0, 4.6);
        let far = lasting_moon_kg(1.0 / 3.0, 0.1 * AU_KM, star_kg, EARTH_MASS_KG, 6371.0, 4.6);
        assert!((far / near / 2f64.powf(6.5) - 1.0).abs() < 1e-6);
        // About 2 × 10¹⁸ kg in the middle of the zone, 0.07 AU: a moonlet
        // of 60 km at Phobos's density.
        let mid = lasting_moon_kg(1.0 / 3.0, 0.07 * AU_KM, star_kg, EARTH_MASS_KG, 6371.0, 4.6);
        assert!((1e18..4e18).contains(&mid), "{mid:e}");
    }

    #[test]
    fn captured_moons_avoid_the_unstable_inclinations() {
        let mut captured = 0;
        for seed in 0..3000 {
            for n in [None, Some(5)] {
                let mut p = params(seed % 600, 21_300.0);
                if let Some(n) = n {
                    p.moons = MoonsMode::Count(n);
                }
                let system = MoonSystem::draw(&p).unwrap();
                for m in system
                    .moons
                    .iter()
                    .filter(|m| m.origin == MoonOrigin::Captured)
                {
                    captured += 1;
                    assert!(
                        !(55.0..130.0).contains(&m.inclination_deg),
                        "{}",
                        m.inclination_deg
                    );
                }
            }
        }
        assert!(captured > 100, "{captured}");
    }

    #[test]
    fn moons_lock_by_their_despinning_time() {
        // Phobos round Mars: about 20 000 years.
        let mars = 6.417e23;
        let phobos = despin_gyr(1.066e16, 11.1, 1.87, MOON_INERTIA, mars, 9_376.0);
        assert!((5e-6..1e-4).contains(&phobos), "{phobos}");
        // The Moon: a few million years; the Earth would need 35 Gyr to
        // turn with it; Pluto turns with Charon within a few million years.
        let moon = despin_gyr(7.35e22, 1737.0, 3.34, 0.394, EARTH_MASS_KG, 384_400.0);
        assert!((1e-3..0.1).contains(&moon), "{moon}");
        let earth = despin_gyr(
            EARTH_MASS_KG,
            6371.0,
            5.51,
            PLANET_INERTIA,
            7.35e22,
            384_400.0,
        );
        assert!((20.0..60.0).contains(&earth), "{earth}");
        let pluto = despin_gyr(1.303e22, 1188.0, 1.85, PLANET_INERTIA, 1.59e21, 19_596.0);
        assert!(pluto < 0.05, "{pluto}");
        // Drawn moons: close ones locked, some far captured ones free.
        let (mut free, mut locked_close) = (0, 0);
        for seed in 0..2000 {
            let system = MoonSystem::draw(&params(seed, 21_300.0)).unwrap();
            for m in &system.moons {
                if m.semi_major_axis_radii < 10.0 {
                    assert!(m.tidally_locked, "{m:?}");
                    locked_close += 1;
                }
                free += usize::from(!m.tidally_locked);
            }
        }
        assert!(free > 0 && locked_close > 0, "{free} {locked_close}");
    }

    #[test]
    fn moon_shapes_round_off_continuously_and_tides_stretch_round_moons() {
        // Rock rounds from 300 km in radius, ice from 200 km.
        assert!(is_round(Composition::Rock, 300.0) && !is_round(Composition::Rock, 299.0));
        assert!(is_round(Composition::Ice, 200.0) && !is_round(Composition::Ice, 199.0));
        // No jump at the limit: just under it, nearly a sphere.
        let under = moon_axes(Composition::Rock, 299.9, [1.0, 1.0], None);
        let over = moon_axes(Composition::Rock, 300.0, [1.0, 1.0], None);
        assert!(
            under[2] / under[0] > 0.998 && over == [300.0; 3],
            "{under:?}"
        );
        let tiny = moon_axes(Composition::Rock, 10.0, [0.5, 0.5], None);
        assert!(tiny[2] / tiny[0] < 0.8, "{tiny:?}");
        // Mimas round Saturn: 198 km, 1.15 g/cm³, 185 539 km out; measured
        // 207.8 × 196.7 × 190.6 km.
        let mass = 4.0 / 3.0 * PI * 198.0f64.powi(3) * 1.15e12;
        let mimas = moon_axes(
            Composition::Ice,
            198.0,
            [0.5, 0.5],
            Some((mass, 5.683e26, 185_539.0)),
        );
        let [a, b, c] = mimas;
        assert!(((a - c) - 17.2).abs() < 2.5, "{mimas:?}");
        assert!(((b - c) / (a - c) - 0.25).abs() < 0.01, "{mimas:?}");
        assert!(((a * b * c).cbrt() - 198.0).abs() < 1e-6);
    }

    #[test]
    fn a_giants_small_regular_moons_are_not_round() {
        // A giant world with a captured Triton: its regular moons share a
        // fiftieth of the mass, the smallest under the ice limit.
        let (mut small, mut checked) = (0, 0);
        for seed in 0..200 {
            let mut p = params(seed, 40_075.0);
            p.look = crate::look::Look::load("ice-giant");
            p.giant_mass_earth = Some(17.0);
            p.orbit_au = 20.0;
            p.circumference_km = crate::giant_world::radius_earth(&p, 17.0) * 40_075.0;
            for m in MoonSystem::draw(&p).unwrap().moons {
                checked += 1;
                assert_eq!(m.round, is_round(m.composition, f64::from(m.radius_km)));
                if !m.round {
                    small += 1;
                    let [a, _, c] = m.axes_km;
                    assert!(c < a, "{m:?}");
                }
            }
        }
        assert!(checked > 0 && small > 0, "{small} of {checked}");
    }

    #[test]
    fn a_close_giant_keeps_its_moons_apart_and_only_light_ones_once_locked() {
        // A Jupiter at 0.12 AU from the Sun: locked by its star, a small
        // Hill sphere.
        for seed in 0..60 {
            let mut p = params(seed, 40_075.0);
            p.look = crate::look::Look::load("gas-giant");
            p.giant_mass_earth = Some(318.0);
            p.orbit_au = 0.12;
            p.circumference_km = crate::giant_world::radius_earth(&p, 318.0) * 40_075.0;
            let system = MoonSystem::draw(&p).unwrap();
            let star = StarInfo::new(p.star, p.orbit_au, p.eccentricity, p.age_gyr);
            let look = crate::giant_world::Look::draw(&p, 318.0).info;
            assert!(crate::spin::giant_locked(&star, 318.0, look.radius_earth));
            let m_g = 318.0 * EARTH_MASS_KG;
            for m in &system.moons {
                assert!(f64::from(m.mass_ratio) * m_g < 1e22, "{m:?}");
                assert!(m.semi_major_axis_km < system.hill_radius_km / 3.0);
                assert_eq!(m.migration, Migration::Decaying);
            }
            for pair in system.moons.windows(2) {
                let ratio = pair[1].semi_major_axis_km / pair[0].semi_major_axis_km;
                assert!(ratio > 1.2, "seed {seed}: {ratio}");
            }
        }
        // Far out, the full chain, kept apart.
        for seed in 0..60 {
            let mut p = params(seed, 40_075.0);
            p.look = crate::look::Look::load("gas-giant");
            p.giant_mass_earth = Some(318.0);
            p.orbit_au = 5.2;
            p.circumference_km = crate::giant_world::radius_earth(&p, 318.0) * 40_075.0;
            let system = MoonSystem::draw(&p).unwrap();
            assert!(system.moons.len() >= 2);
            for pair in system.moons.windows(2) {
                assert!(pair[1].semi_major_axis_km / pair[0].semi_major_axis_km > 1.2);
            }
        }
    }

    #[test]
    fn a_moons_world_is_locked_to_its_planet_not_to_the_star() {
        // An Earth locked to an M5 dwarf, with a moonlet.
        let mut p = params(3, 40_075.0);
        p.star = crate::star::parse_type("M5V").unwrap();
        p.orbit_au = crate::star::orbit_at(p.star, 0.5);
        p.moons = MoonsMode::Count(1);
        assert!(crate::spin::star_info(&p).tidally_locked);
        let moon = &MoonSystem::draw(&p).unwrap().moons[0];
        let world = body_params(&p, moon);
        let star = crate::spin::star_info(&world);
        assert!(!star.tidally_locked && !star.spin_orbit_3_2);
        // A 3:2 planet's moon is not resonant either.
        p.eccentricity = 0.1;
        p.seed = (0..)
            .find(|&s| crate::spin::capture(0.1, s) == 1.5)
            .unwrap();
        assert!(crate::spin::star_info(&p).spin_orbit_3_2);
        if let Some(moon) = MoonSystem::draw(&p).unwrap().moons.first() {
            assert!(!crate::spin::star_info(&body_params(&p, moon)).spin_orbit_3_2);
        }
    }

    #[test]
    fn a_moons_world_takes_its_tilt() {
        let planet = params(5, 40_075.0);
        let moons = (0..400)
            .filter_map(|seed| MoonSystem::draw(&params(seed, 40_075.0)))
            .flat_map(|s| s.moons)
            .collect::<Vec<_>>();
        let impact = moons
            .iter()
            .find(|m| m.origin == MoonOrigin::GiantImpact)
            .unwrap();
        assert_eq!(body_params(&planet, impact).axial_tilt_deg, MOON_TILT_DEG);
        if let Some(other) = moons.iter().find(|m| m.origin != MoonOrigin::GiantImpact) {
            assert_eq!(
                body_params(&planet, other).axial_tilt_deg,
                CLOSE_MOON_TILT_DEG
            );
        }
    }

    #[test]
    fn the_moon_floor_follows_the_giants_density_and_rings() {
        // Saturn: the Roche limit of ice at 1.9 g/cm³ is about 1.75 R,
        // inside its rings, which set the floor instead.
        assert!((giant_moon_floor_radii(0.69, None) - 1.74).abs() < 0.02);
        assert!((giant_moon_floor_radii(0.69, Some(2.27)) - 2.57).abs() < 1e-9);
        // Neptune: no bright rings, the Roche limit of an icy moon.
        assert!((giant_moon_floor_radii(1.64, None) - 2.32).abs() < 0.02);
    }

    #[test]
    fn a_ring_moons_ridge_rings_its_equator_and_keeps_its_volume() {
        let planet = params(5, 21_300.0);
        let shape = Some(crate::rings::LOBE_SHAPE.map(|s| s as f32));
        let plain = small_body_params(&planet, Composition::Ice, 15.0, 3, shape);
        let ridged = WorldParams {
            ridge: Some(0.2),
            ..plain.clone()
        };
        let (flat, ridge) = (shape_layer(&plain).unwrap(), shape_layer(&ridged).unwrap());
        let row = |layer: &[f32], y: u32| {
            let at = (y * plain.width) as usize;
            layer[at..at + plain.width as usize].iter().sum::<f32>() / plain.width as f32
        };
        // Higher on the equator than the same body without it, lower off it.
        let equator = plain.height / 2;
        assert!(row(&ridge, equator) > 1.1 * row(&flat, equator));
        assert!(row(&ridge, 2) < row(&flat, 2));
        // Symmetric about the equator.
        assert!((row(&ridge, equator - 3) - row(&ridge, equator + 2)).abs() < 0.02);
        // The volume, within the pixels' own error.
        let volume = |layer: &[f32]| {
            (0..plain.height)
                .map(|y| {
                    let lat = (f64::from(y) + 0.5) / f64::from(plain.height) * PI - PI / 2.0;
                    lat.cos() * f64::from(row(layer, y).powi(3))
                })
                .sum::<f64>()
        };
        let ratio = volume(&ridge) / volume(&flat);
        assert!((ratio - 1.0).abs() < 0.03, "{ratio}");
    }
}
