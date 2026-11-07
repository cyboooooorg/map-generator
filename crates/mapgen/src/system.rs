//! Planetary systems (`docs/system-spec.md`): a star and every body around
//! it, drawn in milliseconds and stored in `system.json`; a body's maps are
//! generated later, on demand, from the parameters fixed here.
//!
//! **Two requests.** A whole system ([`WholeSystemRequest`]: a star and a
//! number of planets of each type, the others drawn), or a system around
//! one planet ([`SystemRequest::Around`]: today's [`WorldRequest`], the
//! rest of the system drawn round it, a giant's moon putting its giant on
//! its orbit).
//!
//! **Zones.** From the star: the inner edge (0.02 AU), the habitable zone
//! (Kopparapu et al. 2013, as [`StarInfo`] has it), the ice line `2.7 √L`
//! AU (Hayashi 1981) and the outer edge (60 AU). Each planet type has a
//! band it may sit in and one it prefers ([`band`]).
//!
//! **Spacing.** Planets are placed from the star out, each at least
//! [`HILL_SPACING`] mutual Hill radii from the last, periapsis to
//! apoapsis, the spacing that keeps systems stable for billions of years
//! (Chambers et al. 1996; Pu & Wu 2015); a hot Jupiter keeps others beyond
//! four times its period (Steffen et al. 2012). A planet pushed out of its
//! band is drawn again a few times, then the request fails
//! ([`RequestError::ZoneFull`]).
//!
//! **Moons.** A rocky planet draws its moons as a world does
//! ([`MoonSystem::draw`]); a giant its regular moons, a resonant chain
//! sharing about 10⁻⁴ of its mass, as Jupiter's, drawn as a giant world
//! draws them (`MoonSystem::draw` too).
//!
//! **Determinism.** A system is its request and its seed: each body's seed
//! comes from the system's seed and the body's id, and its parameters are
//! stored, so a body generated later is the world it would have been at
//! once.

use crate::exomoon::{EARTH_KG, EARTH_M, G, HostGiant, HostMode};
use crate::giants::{GiantKind, GiantSystem, GiantsMode, JUPITER_EARTH_MASSES};
use crate::moons::{Composition, Moon, MoonSystem, MoonsMode};
use crate::naming::{planet_letter, roman, star_name};
use crate::noise::{stream, stream_seed};
use crate::request::{RequestError, WorldRequest};
use crate::star::{StarInfo, draw_eccentricity, draw_type, parse_type};
use crate::world::WorldParams;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::f64::consts::PI;

/// Version of `system.json`.
pub const SYSTEM_FORMAT_VERSION: u32 = 1;
/// Most planets a system holds, moons not counted.
pub const MAX_PLANETS: usize = 15;
/// Mutual Hill radii kept between neighbouring planets.
pub const HILL_SPACING: f64 = 10.0;
/// Inner and outer edges of a system, AU.
const INNER_EDGE_AU: f64 = 0.02;
const OUTER_EDGE_AU: f64 = 60.0;
/// Tries to place a system's planets before a request fails.
const TRIES: usize = 24;
/// Default map size, as the requests have it.
const DEFAULT_SIZE: (u32, u32) = (
    crate::request::fallback::WIDTH,
    crate::request::fallback::HEIGHT,
);

/// The planet types a system may be asked for: the planet presets.
pub const PLANET_TYPES: [&str; 8] = [
    "terran",
    "barren",
    "caustic",
    "frozen",
    "volcanic",
    "gas-giant",
    "ice-giant",
    "hot-jupiter",
];

/// What a system is drawn from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum SystemRequest {
    /// A whole system: its star and its planets by type.
    System(WholeSystemRequest),
    /// A system around one planet, as a world is asked for.
    Around(WorldRequest),
}

/// A whole system: a star and a number of planets of each type.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(default)]
pub struct WholeSystemRequest {
    /// Spectral type of the star (`G2V`); drawn like the stars nearby when
    /// left out.
    pub star: Option<String>,
    /// `[Fe/H]` of the star, dex; drawn when left out.
    pub metallicity: Option<f32>,
    /// The planets asked for, by type ([`PLANET_TYPES`]); the whole system
    /// drawn when left out. At most [`MAX_PLANETS`] in all.
    pub planets: Option<BTreeMap<String, u32>>,
    pub seed: Option<u32>,
    /// Map size of every planet (moons scaled to their size, giants at
    /// their fixed width).
    pub width: Option<u32>,
    pub height: Option<u32>,
}

/// A system as `system.json` stores it.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct System {
    pub format_version: u32,
    pub request: SystemRequest,
    pub seed: u32,
    /// Map size of the planets.
    pub width: u32,
    pub height: u32,
    pub star: SystemStar,
    /// The planets from the star out, each followed by its moons, nearest
    /// first.
    pub bodies: Vec<Body>,
    /// The body a request around a planet asked for (a giant's moon for
    /// `host = giant`), generated with the system; none for a whole system.
    #[serde(default)]
    pub focus: Option<String>,
    /// Its belts of small bodies, nearest first (see `crate::belts`); none
    /// in the systems drawn before them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<crate::belts::Belt>>", optional))]
    pub belts: Vec<crate::belts::Belt>,
}

/// The star of a system.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct SystemStar {
    /// Spectral type (`G2V`).
    #[serde(rename = "type")]
    pub spectral: String,
    /// Catalogue designation (`MGS 48213`).
    pub designation: String,
    /// The star seen from 1 AU (its temperature, luminosity, mass, radius,
    /// habitable zone); the orbital fields are those of a 1 AU circle.
    pub info: StarInfo,
    pub metallicity: f32,
    /// The ice line, AU.
    pub ice_line_au: f32,
}

/// A planet, or a moon of one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum BodyKind {
    Planet,
    Moon,
}

/// One body of a system.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Body {
    /// `b` to `p` for the planets from the star out, `<planet>-<n>` for
    /// the moons (from 1, nearest first).
    pub id: String,
    /// `MGS 48213 c`, `MGS 48213 c II`.
    pub designation: String,
    pub kind: BodyKind,
    /// Its preset: a planet type, `rocky-moon` or `icy-moon`.
    #[serde(rename = "type")]
    pub body_type: String,
    /// The planet a moon orbits; none for a planet.
    pub parent: Option<String>,
    pub orbit: Orbit,
    pub physical: Physical,
    pub spin: Spin,
    pub surroundings: Surroundings,
    pub preview: Preview,
    /// Everything its generation needs.
    pub params: WorldParams,
    /// Whether its maps are in `bodies/<id>/`.
    pub generated: bool,
    /// A planet's trojans at its L4 and L5 points, where they last (see
    /// `crate::belts`); none for a moon, and in the systems drawn before
    /// them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub trojans: Option<crate::belts::Trojans>,
    /// A ring moon's part in its giant's rings (see `crate::rings`); none
    /// for every other body.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub ring_role: Option<crate::rings::RingRole>,
}

/// An orbit, round the star (a planet) or round its planet (a moon).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Orbit {
    pub semi_major_axis_km: f64,
    pub semi_major_axis_au: f64,
    pub eccentricity: f64,
    /// To the star's equator (a planet) or the planet's (a moon).
    pub inclination_deg: f64,
    /// Longitude of the ascending node, argument of the periapsis and mean
    /// anomaly at the system's epoch.
    pub node_deg: f64,
    pub periapsis_deg: f64,
    pub mean_anomaly_deg: f64,
    pub period_days: f64,
}

/// What a body is made of and how big.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Physical {
    pub mass_earth: f64,
    pub radius_km: f64,
    /// g/cm³.
    pub density: f64,
    /// Surface gravity, relative to Earth's.
    pub gravity: f64,
    /// An irregular moon's semi-axes, km.
    pub axes_km: Option<[f32; 3]>,
}

/// How a body turns.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Spin {
    pub period_hours: f64,
    pub axial_tilt_deg: f64,
    /// Locked to what it orbits: its star (a planet) or its planet (a
    /// moon), its day its orbit.
    pub locked: bool,
    /// Held by its star's tides in the 3:2 spin–orbit resonance, as
    /// Mercury: three turns every two orbits, its long axis towards the
    /// star at each periapsis (see `crate::spin`). Systems drawn before it:
    /// false.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub spin_orbit_3_2: bool,
}

/// What a body gets from its star.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Surroundings {
    /// Mean starlight, relative to Earth's.
    pub flux_earth: f64,
    /// Equilibrium temperature, K, without greenhouse effect.
    pub equilibrium_k: f64,
    /// Place in the habitable zone: 0 its inner edge, 1 its outer edge.
    pub hz_position: f64,
}

/// What the viewer draws before a body's maps exist: a sphere of
/// `colours[0]`, with `colours[1]` as its land or its belts and the
/// following as its poles; giants with `bands` bands.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Preview {
    /// Linear RGB.
    pub colours: Vec<[f32; 3]>,
    pub bands: u8,
    /// Lava glowing on its night side.
    pub glow: bool,
}

/// The id of the system: its star's designation and its seed, lowercase
/// with dashes (`mgs-48213-42`); a system around a planet whose plates are
/// moved to another epoch names it too (`mgs-48213-past250ma-42`, see
/// [`crate::export::drift_label`]), so that the epochs of one planet never
/// collide. The seed stays last.
pub fn system_id(system: &System) -> String {
    let slug: String = system
        .star
        .designation
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let drift = system
        .focus
        .as_ref()
        .and_then(|id| system.bodies.iter().find(|b| &b.id == id))
        .and_then(|b| crate::export::drift_label(b.params.drift_ma));
    match drift {
        Some(epoch) => format!("{slug}-{epoch}-{}", system.seed),
        None => format!("{slug}-{}", system.seed),
    }
}

/// Writes `system.json` into `dir` (a temporary file, then renamed).
pub fn write_system(dir: &std::path::Path, system: &System) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_vec_pretty(system).map_err(|e| e.to_string())?;
    let temp = dir.join(".system.json.tmp");
    std::fs::write(&temp, json).map_err(|e| e.to_string())?;
    std::fs::rename(&temp, dir.join("system.json")).map_err(|e| e.to_string())
}

/// Reads `system.json` from `dir`.
pub fn read_system(dir: &std::path::Path) -> Result<System, String> {
    let bytes = std::fs::read(dir.join("system.json")).map_err(|e| e.to_string())?;
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

/// Draws the system a request asks for.
pub fn draw(request: &SystemRequest, rng: &mut impl RngExt) -> Result<System, RequestError> {
    match request {
        SystemRequest::System(whole) => draw_whole(request, whole, rng),
        SystemRequest::Around(world) => {
            let params = world.resolve(rng)?.params;
            Ok(draw_around(request, params))
        }
    }
}

/**
 * The system around a planet already resolved from `world` (its `params`,
 * as `world.resolve` gave them): the same system [`draw`] gives for
 * `SystemRequest::Around(world)` with those parameters, for a caller that
 * shows them first (the CLI).
 */
pub fn draw_around_params(world: &WorldRequest, params: WorldParams) -> System {
    draw_around(&SystemRequest::Around(world.clone()), params)
}

/// The band of distances a planet type may sit in, and the one it
/// prefers, AU, round a star of habitable zone `hz` and ice line `ice`.
pub fn band(body_type: &str, hz: (f64, f64), ice: f64) -> Option<((f64, f64), (f64, f64))> {
    let (inner, outer) = hz;
    let edge = INNER_EDGE_AU;
    let far = OUTER_EDGE_AU;
    Some(match body_type {
        "hot-jupiter" => ((edge, 0.1), (0.03, 0.07)),
        "caustic" => ((edge, inner), (0.5 * inner, inner)),
        "volcanic" => ((edge, ice), (0.3 * inner, inner + 0.3 * (outer - inner))),
        "terran" => ((inner * 0.95, outer * 1.05), (inner, outer)),
        "barren" => ((edge, ice), (edge.max(0.2 * inner), ice)),
        "frozen" => ((outer, far), (outer, (2.0 * ice).max(outer * 1.5))),
        "gas-giant" => ((0.5 * ice, far), (ice, 4.0 * ice)),
        "ice-giant" => ((ice, far), (2.5 * ice, 10.0 * ice)),
        _ => return None,
    })
}

/// A planet being placed: its type, mass (Earth masses), orbit and
/// eccentricity, and what it stays as.
#[derive(Clone, Debug)]
struct Slot {
    body_type: String,
    mass_earth: f64,
    orbit_au: f64,
    eccentricity: f64,
    /// Kept where it is (the planet of a request around one).
    fixed: bool,
}

/// Whether two orbits round a star of `star_mass` (M☉) are clear of each
/// other: [`HILL_SPACING`] mutual Hill radii, periapsis to apoapsis, and
/// four times a hot Jupiter's period.
fn clear(a: &Slot, b: &Slot, star_mass: f64) -> bool {
    let (inner, outer) = if a.orbit_au < b.orbit_au {
        (a, b)
    } else {
        (b, a)
    };
    let sun = 332_946.0;
    let hill = ((inner.mass_earth + outer.mass_earth) / (3.0 * star_mass * sun)).cbrt()
        * 0.5
        * (inner.orbit_au + outer.orbit_au);
    let gap =
        outer.orbit_au * (1.0 - outer.eccentricity) - inner.orbit_au * (1.0 + inner.eccentricity);
    let hot = inner.body_type == "hot-jupiter" || outer.body_type == "hot-jupiter";
    gap >= HILL_SPACING * hill && (!hot || outer.orbit_au / inner.orbit_au >= 4f64.powf(2.0 / 3.0))
}

/// A typical mass for a planet type, Earth masses, drawn.
fn draw_mass(body_type: &str, rng: &mut impl RngExt) -> f64 {
    let log_uniform = |lo: f64, hi: f64, rng: &mut dyn rand::Rng| {
        (lo.ln() + rand::RngExt::random::<f64>(rng) * (hi / lo).ln()).exp()
    };
    match body_type {
        "gas-giant" => {
            // dN/d ln M ∝ M^−0.31 between 0.1 and 13 Jupiter masses.
            let (lo, hi) = (0.1f64.powf(-0.31), 13f64.powf(-0.31));
            (lo + rng.random::<f64>() * (hi - lo)).powf(1.0 / -0.31)
                * f64::from(JUPITER_EARTH_MASSES)
        }
        "hot-jupiter" => log_uniform(0.3, 3.0, rng) * f64::from(JUPITER_EARTH_MASSES),
        "ice-giant" => log_uniform(10.0, 50.0, rng),
        "terran" => log_uniform(0.4, 2.5, rng),
        "caustic" | "volcanic" => log_uniform(0.3, 2.0, rng),
        "frozen" => log_uniform(0.05, 3.0, rng),
        _ => log_uniform(0.03, 1.5, rng),
    }
}

/// Whether a type is a giant.
fn is_giant(body_type: &str) -> bool {
    matches!(body_type, "gas-giant" | "ice-giant" | "hot-jupiter")
}

/// The planets drawn when a request leaves them to the draw: small planets
/// from the Kepler counts (about 1.5 within 1 AU of a Sun-like star, 2.5 of
/// an M dwarf; Mulders et al. 2018), a few colder ones beyond, the giants
/// as [`GiantSystem`] draws them; their types from where they fall.
fn drawn_types(star: &StarInfo, metallicity: f64, rng: &mut impl RngExt) -> Vec<(String, f64)> {
    let mass = f64::from(star.mass);
    let inner_mean = if mass < 0.6 {
        2.5
    } else if mass < 1.4 {
        1.5
    } else {
        0.8
    };
    let outer_mean = 1.0;
    let gas_mean = 0.25 * 10f64.powf(2.0 * metallicity).min(10.0) * mass.clamp(0.1, 2.5);
    let ice_mean = 0.3 * 10f64.powf(0.5 * metallicity) * mass.clamp(0.1, 2.5).sqrt();
    let hz = (f64::from(star.hz_inner_au), f64::from(star.hz_outer_au));
    let ice = 2.7 * f64::from(star.luminosity).sqrt();
    let mut out = Vec::new();
    // Inner small planets: periods log-uniform from 1 to 400 days.
    for _ in 0..poisson(inner_mean, rng) {
        let days = (1f64.ln() + rng.random::<f64>() * 400f64.ln()).exp();
        let a = (mass * (days / 365.25).powi(2)).cbrt().max(INNER_EDGE_AU);
        let body_type = if a < 0.5 * hz.0 {
            if rng.random::<f64>() < 0.3 {
                "volcanic"
            } else {
                "barren"
            }
        } else if a < hz.0 {
            if rng.random::<f64>() < 0.5 {
                "caustic"
            } else {
                "barren"
            }
        } else if a <= hz.1 {
            "terran"
        } else if a < ice {
            "barren"
        } else {
            "frozen"
        };
        out.push((body_type.to_string(), a));
    }
    // Colder rocky planets, between the habitable zone and the outer giants.
    for _ in 0..poisson(outer_mean, rng) {
        let a = hz.1 * (1.0 + rng.random::<f64>() * 8.0);
        let body_type = if a < ice { "barren" } else { "frozen" };
        out.push((body_type.to_string(), a));
    }
    for _ in 0..poisson(gas_mean, rng) {
        out.push((
            "gas-giant".to_string(),
            ice * 10f64.powf(0.3 + 0.4 * normal(rng)),
        ));
    }
    for _ in 0..poisson(ice_mean, rng) {
        out.push((
            "ice-giant".to_string(),
            ice * 10f64.powf(0.7 + 0.3 * normal(rng)),
        ));
    }
    out
}

/// A Poisson count of mean `mean` (Knuth's method).
fn poisson(mean: f64, rng: &mut impl RngExt) -> usize {
    let limit = (-mean).exp();
    let mut product = rng.random::<f64>();
    let mut count = 0;
    while product > limit && count < 32 {
        product *= rng.random::<f64>();
        count += 1;
    }
    count
}

/// A standard normal draw (Box–Muller).
fn normal(rng: &mut impl RngExt) -> f64 {
    let u: f64 = rng.random::<f64>().max(1e-12);
    let v: f64 = rng.random();
    (-2.0 * u.ln()).sqrt() * (2.0 * PI * v).cos()
}

/// Spaces planets drawn at their preferred distances: from the star out,
/// each pushed out until it is clear of the last placed (the fixed ones
/// stay, the others around them), and the rocky ones past the inner belt's
/// zone of the first cold giant (`belts::belt_zone`), where that giant
/// stops a planet forming. The zone follows the giant as it settles, a few
/// rounds at most. `None` names the first type pushed out of its band.
fn space(slots: &mut Vec<Slot>, star_mass: f64, hz: (f64, f64), ice: f64) -> Result<(), String> {
    slots.sort_by(|a, b| a.orbit_au.total_cmp(&b.orbit_au));
    let zone_of = |slots: &[Slot]| {
        slots
            .iter()
            .filter(|s| crate::belts::cold_giant(&s.body_type, s.orbit_au, ice))
            .map(|s| s.orbit_au)
            .reduce(f64::min)
            .map(crate::belts::belt_zone)
    };
    let mut zone = zone_of(slots);
    let mut placed = Vec::new();
    for _ in 0..4 {
        placed = place(slots, star_mass, hz, ice, zone)?;
        let settled = zone_of(&placed);
        if settled == zone || placed.iter().all(|s| !in_zone(s, settled)) {
            *slots = placed;
            return Ok(());
        }
        zone = settled;
    }
    match placed.iter().find(|s| in_zone(s, zone_of(&placed))) {
        Some(s) => Err(s.body_type.clone()),
        None => {
            *slots = placed;
            Ok(())
        }
    }
}

/// Whether a rocky planet that may move crosses an inner belt's zone.
fn in_zone(slot: &Slot, zone: Option<(f64, f64)>) -> bool {
    zone.is_some_and(|(lo, hi)| {
        !slot.fixed
            && !is_giant(&slot.body_type)
            && slot.orbit_au * (1.0 + slot.eccentricity) > lo
            && slot.orbit_au * (1.0 - slot.eccentricity) < hi
    })
}

/// One round of [`space`], the rocky planets kept out of `zone`.
fn place(
    slots: &[Slot],
    star_mass: f64,
    hz: (f64, f64),
    ice: f64,
    zone: Option<(f64, f64)>,
) -> Result<Vec<Slot>, String> {
    let mut placed: Vec<Slot> = slots.iter().filter(|s| s.fixed).cloned().collect();
    let mut free: Vec<Slot> = slots.iter().filter(|s| !s.fixed).cloned().collect();
    free.sort_by(|a, b| a.orbit_au.total_cmp(&b.orbit_au));
    for mut slot in free {
        let ((_, max), _) = band(&slot.body_type, hz, ice).expect("known type");
        // Out in small steps until clear of every planet placed, jumping
        // a belt's zone.
        loop {
            if in_zone(&slot, zone) {
                let (_, hi) = zone.expect("in a zone");
                slot.orbit_au = slot.orbit_au.max(hi / (1.0 - slot.eccentricity) * 1.001);
            } else if placed.iter().all(|other| clear(&slot, other, star_mass)) {
                break;
            } else {
                slot.orbit_au *= 1.02;
            }
            if slot.orbit_au > max.min(OUTER_EDGE_AU) {
                return Err(slot.body_type);
            }
        }
        placed.push(slot);
    }
    placed.sort_by(|a, b| a.orbit_au.total_cmp(&b.orbit_au));
    Ok(placed)
}

/// How many planets of a type fit in its band, packed from its inner edge
/// at the spacing, each of a typical mass.
fn fits(body_type: &str, star_mass: f64, hz: (f64, f64), ice: f64) -> u32 {
    let Some(((min, max), _)) = band(body_type, hz, ice) else {
        return 0;
    };
    let mut rng = StdRng::seed_from_u64(0);
    let mass = draw_mass(body_type, &mut rng);
    let mut last = Slot {
        body_type: body_type.to_string(),
        mass_earth: mass,
        orbit_au: min.max(INNER_EDGE_AU),
        eccentricity: 0.0,
        fixed: false,
    };
    let mut count = 1;
    let mut next = last.clone();
    loop {
        next.orbit_au *= 1.02;
        if next.orbit_au > max.min(OUTER_EDGE_AU) {
            return count;
        }
        if clear(&next, &last, star_mass) {
            count += 1;
            last = next.clone();
        }
    }
}

/// A whole system.
fn draw_whole(
    request: &SystemRequest,
    whole: &WholeSystemRequest,
    rng: &mut impl RngExt,
) -> Result<System, RequestError> {
    let seed = whole.seed.unwrap_or_else(|| rng.random());
    let mut rng = StdRng::seed_from_u64(u64::from(stream_seed(seed, stream::SYSTEM)));
    let star_code = match &whole.star {
        Some(text) => parse_type(text).map_err(RequestError::BadStar)?,
        None => draw_type(&mut rng),
    };
    let metallicity = whole
        .metallicity
        .unwrap_or_else(|| crate::giants::draw_metallicity(seed));
    let (width, height) = (
        whole.width.unwrap_or(DEFAULT_SIZE.0),
        whole.height.unwrap_or(DEFAULT_SIZE.1),
    );
    let star = StarInfo::new(star_code, 1.0, 0.0);
    let hz = (f64::from(star.hz_inner_au), f64::from(star.hz_outer_au));
    let ice = 2.7 * f64::from(star.luminosity).sqrt();
    let star_mass = f64::from(star.mass);

    // The preferred band of each planet asked, to draw its distance again
    // when a try fails.
    let mut preferred: Vec<(String, f64, f64)> = Vec::new();
    let asked: Vec<(String, f64)> = match &whole.planets {
        Some(planets) => {
            let total: u32 = planets.values().sum();
            if total as usize > MAX_PLANETS {
                return Err(RequestError::TooManyPlanets {
                    asked: total,
                    max: MAX_PLANETS as u32,
                });
            }
            let mut list = Vec::new();
            for (body_type, &count) in planets {
                let Some(((min, max), (lo, hi))) = band(body_type, hz, ice) else {
                    return Err(RequestError::UnknownType(body_type.clone()));
                };
                if count == 0 {
                    continue;
                }
                if max <= min || max < INNER_EDGE_AU {
                    return Err(RequestError::TypeImpossible {
                        body_type: body_type.clone(),
                    });
                }
                let room = fits(body_type, star_mass, hz, ice);
                if count > room {
                    return Err(RequestError::ZoneFull {
                        body_type: body_type.clone(),
                        asked: count,
                        fits: room,
                    });
                }
                let (lo, hi) = (lo.max(min).max(INNER_EDGE_AU), hi.min(max));
                for _ in 0..count {
                    list.push((body_type.clone(), log_between(lo, hi, &mut rng)));
                    preferred.push((body_type.clone(), lo, hi));
                }
            }
            list
        }
        None => drawn_types(&star, f64::from(metallicity), &mut rng),
    };
    let packed = asked.len() >= 4;
    let mut last_error = None;
    let mut asked = asked;
    for attempt in 0..TRIES {
        // After a failed try, the planets asked are drawn again in their
        // preferred bands: a giant too close to a rocky planet asked (its
        // belt's zone over the planet's band) moves rather than failing.
        if attempt > 0 && !preferred.is_empty() {
            asked = preferred
                .iter()
                .map(|(t, lo, hi)| (t.clone(), log_between(*lo, *hi, &mut rng)))
                .collect();
        }
        let mut slots: Vec<Slot> = asked
            .iter()
            .map(|(body_type, a)| Slot {
                body_type: body_type.clone(),
                mass_earth: draw_mass(body_type, &mut rng),
                orbit_au: *a * (1.0 + 0.2 * (rng.random::<f64>() - 0.5)),
                eccentricity: if body_type == "hot-jupiter" {
                    0.0
                } else {
                    // Packed systems are calmer (He et al. 2020).
                    f64::from(draw_eccentricity(&mut rng)) * if packed { 0.4 } else { 1.0 }
                },
                fixed: false,
            })
            .collect();
        match space(&mut slots, star_mass, hz, ice) {
            Ok(()) => {
                // A drawn system keeps the planets that fit: those pushed
                // out are dropped rather than failing.
                return Ok(assemble(
                    request,
                    seed,
                    star_code,
                    metallicity,
                    width,
                    height,
                    slots,
                    None,
                ));
            }
            Err(body_type) => last_error = Some(body_type),
        }
        if whole.planets.is_none() {
            break;
        }
    }
    let body_type = last_error.unwrap_or_default();
    if whole.planets.is_none() {
        // A drawn system that does not fit is drawn again without the
        // planet that failed, until it fits.
        let mut kept = asked;
        while let Some(i) = kept.iter().rposition(|(t, _)| *t == body_type) {
            kept.remove(i);
            let mut slots: Vec<Slot> = kept
                .iter()
                .map(|(t, a)| Slot {
                    body_type: t.clone(),
                    mass_earth: draw_mass(t, &mut rng),
                    orbit_au: *a,
                    eccentricity: f64::from(draw_eccentricity(&mut rng)) * 0.4,
                    fixed: false,
                })
                .collect();
            if space(&mut slots, star_mass, hz, ice).is_ok() {
                return Ok(assemble(
                    request,
                    seed,
                    star_code,
                    metallicity,
                    width,
                    height,
                    slots,
                    None,
                ));
            }
        }
        return Ok(assemble(
            request,
            seed,
            star_code,
            metallicity,
            width,
            height,
            Vec::new(),
            None,
        ));
    }
    let asked_of = whole
        .planets
        .as_ref()
        .and_then(|p| p.get(&body_type))
        .copied()
        .unwrap_or(0);
    Err(RequestError::ZoneFull {
        fits: fits(&body_type, star_mass, hz, ice).min(asked_of.saturating_sub(1)),
        body_type,
        asked: asked_of,
    })
}

/// A log-uniform draw between `lo` and `hi`.
fn log_between(lo: f64, hi: f64, rng: &mut impl RngExt) -> f64 {
    if hi <= lo {
        return lo;
    }
    (lo.ln() + rng.random::<f64>() * (hi / lo).ln()).exp()
}

/// The planet of a request around one: how it sits in the system.
struct Focus {
    params: WorldParams,
    /// The giant it orbits, when it is a giant's moon.
    host: Option<HostGiant>,
}

/// A system around one planet.
fn draw_around(request: &SystemRequest, params: WorldParams) -> System {
    let seed = params.seed;
    let mut rng = StdRng::seed_from_u64(u64::from(stream_seed(seed, stream::SYSTEM)));
    let star = StarInfo::new(params.star, 1.0, 0.0);
    let hz = (f64::from(star.hz_inner_au), f64::from(star.hz_outer_au));
    let ice = 2.7 * f64::from(star.luminosity).sqrt();
    let star_mass = f64::from(star.mass);
    let host = HostGiant::draw(&params);
    // The planet asked, or the giant it orbits, on its orbit.
    let focus_slot = Slot {
        body_type: match &host {
            Some(h) if h.giant.kind == GiantKind::Ice => "ice-giant".to_string(),
            Some(_) => "gas-giant".to_string(),
            None => params.planet.name.clone(),
        },
        mass_earth: match &host {
            Some(h) => f64::from(h.giant.mass_earth),
            None => (f64::from(params.circumference_km) / 40_075.0).powi(3),
        },
        orbit_au: f64::from(params.orbit_au),
        eccentricity: f64::from(params.eccentricity),
        fixed: true,
    };
    let mut slots = vec![focus_slot];
    // Its giants, as the world draws them today.
    if params.giants != GiantsMode::None
        && let Some(giants) = GiantSystem::draw(&params)
    {
        for g in giants.giants {
            slots.push(Slot {
                body_type: if g.hot {
                    "hot-jupiter".to_string()
                } else if g.kind == GiantKind::Ice {
                    "ice-giant".to_string()
                } else {
                    "gas-giant".to_string()
                },
                mass_earth: f64::from(g.mass_earth),
                orbit_au: f64::from(g.orbit_au),
                eccentricity: f64::from(g.eccentricity),
                fixed: true,
            });
        }
    }
    // The rocky planets round them, those that do not fit dropped.
    let mut rocky = drawn_types(&star, f64::from(params.metallicity), &mut rng);
    rocky.retain(|(t, _)| !is_giant(t));
    for (body_type, a) in rocky {
        let mut slot = Slot {
            mass_earth: draw_mass(&body_type, &mut rng),
            body_type,
            orbit_au: a,
            eccentricity: f64::from(draw_eccentricity(&mut rng)) * 0.4,
            fixed: false,
        };
        let mut trial = slots.clone();
        trial.push(slot.clone());
        if space(&mut trial, star_mass, hz, ice).is_ok() {
            slot.fixed = true;
            slots = trial
                .into_iter()
                .map(|mut s| {
                    s.fixed = true;
                    s
                })
                .collect();
        }
    }
    slots.sort_by(|a, b| a.orbit_au.total_cmp(&b.orbit_au));
    let (width, height) = (params.width, params.height);
    assemble(
        request,
        seed,
        params.star,
        params.metallicity,
        width,
        height,
        slots,
        Some(Focus { params, host }),
    )
}

/// The body seed of `id` in the system of `seed`.
fn body_seed(seed: u32, id: &str) -> u32 {
    // FNV-1a of the id, stable across versions.
    let mut hash: u32 = 0x811c_9dc5;
    for byte in id.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    stream_seed(seed ^ hash, stream::SYSTEM)
}

/// The bodies of placed planets, lettered from the star, each with its
/// moons; the planet of a request around one as asked.
#[allow(clippy::too_many_arguments)]
fn assemble(
    request: &SystemRequest,
    seed: u32,
    star_code: f32,
    metallicity: f32,
    width: u32,
    height: u32,
    slots: Vec<Slot>,
    focus: Option<Focus>,
) -> System {
    let info = StarInfo::new(star_code, 1.0, 0.0);
    let designation = star_name(seed, &info.kind);
    let ice = 2.7 * info.luminosity.sqrt();
    // The star's tides have circularised the closest orbits: a planet
    // locked to its star turns on a circle (a despun one left eccentric is
    // held in the 3:2 resonance, see `crate::spin`). The planet a request
    // is around keeps the orbit its request resolved.
    let slots: Vec<Slot> = slots
        .into_iter()
        .map(|mut slot| {
            let asked = slot.fixed
                && focus
                    .as_ref()
                    .is_some_and(|f| (f64::from(f.params.orbit_au) - slot.orbit_au).abs() < 1e-9);
            if !asked {
                slot.eccentricity = tidal_eccentricity(&slot, star_code);
            }
            slot
        })
        .collect();
    let mut bodies = Vec::new();
    let mut focus_id = None;
    for (rank, slot) in slots.iter().enumerate() {
        let id = planet_letter(rank);
        let planet_designation = format!("{designation} {id}");
        let at_focus = slot.fixed
            && focus
                .as_ref()
                .is_some_and(|f| (f64::from(f.params.orbit_au) - slot.orbit_au).abs() < 1e-9);
        let planet_seed = body_seed(seed, &id);
        let params = match (&focus, at_focus) {
            (Some(f), true) if f.host.is_none() => f.params.clone(),
            (Some(f), true) => {
                let host = f.host.as_ref().expect("host");
                crate::exomoon::host_params(&f.params, host.giant.mass_earth, host.seed)
            }
            _ => planet_params(slot, star_code, metallicity, width, height, planet_seed),
        };
        let mut params = if is_giant(&slot.body_type) {
            with_system_rings(params)
        } else {
            params
        };
        // The moons its rings shape (none for the giant a world of the
        // request orbits: its moons are that world's chain).
        let ring_moons = if is_giant(&slot.body_type) && !at_focus {
            shape_rings(&mut params, slot, star_code)
        } else {
            Vec::new()
        };
        let planet = planet_body(&id, &planet_designation, slot, &params, star_code);
        let planet_index = bodies.len();
        if at_focus && focus.as_ref().is_some_and(|f| f.host.is_none()) {
            focus_id = Some(id.clone());
        }
        bodies.push(planet);
        // Its moons.
        let moons = if is_giant(&slot.body_type) {
            match (&focus, at_focus) {
                // The giant a world of the request orbits: the world among
                // its moons.
                (
                    Some(Focus {
                        params: world,
                        host: Some(host),
                    }),
                    true,
                ) => host_chain(host, world, star_code),
                _ => drawn_moons(&params, slot.mass_earth),
            }
        } else {
            drawn_moons(
                &params,
                (f64::from(params.circumference_km) / 40_075.0).powi(3),
            )
        };
        // Nearest first, the ring moons among the others.
        let giant_kg = slot.mass_earth * EARTH_KG;
        let giant_radius_km = bodies[planet_index].physical.radius_km;
        let mut moons: Vec<(MoonParts, Option<&crate::rings::RingMoon>)> = moons
            .into_iter()
            .map(|m| (m, None))
            .chain(ring_moons.iter().map(|r| {
                (
                    ring_moon_parts(&params, r, giant_kg, giant_radius_km),
                    Some(r),
                )
            }))
            .collect();
        moons.sort_by(|a, b| {
            a.0.2
                .semi_major_axis_km
                .total_cmp(&b.0.2.semi_major_axis_km)
        });
        let mut gap_moons =
            vec![String::new(); ring_moons.iter().filter(|r| r.gap.is_some()).count()];
        for (k, ((moon_type, moon_params, orbit, physical, spin), ring)) in
            moons.into_iter().enumerate()
        {
            let moon_id = format!("{id}-{}", k + 1);
            if let Some(gap) = ring.and_then(|r| r.gap) {
                gap_moons[gap] = moon_id.clone();
            }
            if at_focus
                && focus
                    .as_ref()
                    .is_some_and(|f| f.host.is_some() && f.params.seed == moon_params.seed)
            {
                focus_id = Some(moon_id.clone());
            }
            let surroundings = surroundings_at(&info, slot.orbit_au, slot.eccentricity);
            bodies.push(Body {
                id: moon_id,
                designation: format!("{planet_designation} {}", roman(k + 1)),
                kind: BodyKind::Moon,
                preview: preview_of(&moon_type, &moon_params, star_code),
                body_type: moon_type,
                parent: Some(id.clone()),
                orbit,
                physical,
                spin,
                surroundings,
                params: moon_params,
                generated: false,
                trojans: None,
                ring_role: ring.map(|r| r.role),
            });
        }
        if let Some(rings) = bodies[planet_index]
            .params
            .given_rings
            .as_mut()
            .and_then(|g| g.rings.as_mut())
            && !gap_moons.is_empty()
        {
            rings.gap_moons = gap_moons;
        }
        // The dust rings its moons feed.
        let radius_km = bodies[planet_index].physical.radius_km;
        let sources: Vec<crate::rings::DustSource> = bodies[planet_index + 1..]
            .iter()
            .map(|m| crate::rings::DustSource {
                id: m.id.clone(),
                a_radii: m.orbit.semi_major_axis_km / radius_km,
                role: m.ring_role,
                icy: m.body_type == "icy-moon",
            })
            .collect();
        let seed = bodies[planet_index].params.seed;
        if let Some(given) = bodies[planet_index].params.given_rings.as_mut() {
            given.dust = crate::rings::dust_rings(given.rings.as_ref(), &sources, seed);
        }
    }
    // Its small bodies, from the planets as placed: belts and trojans, and
    // the crater rate they set on every body.
    let ids: Vec<String> = (0..slots.len()).map(planet_letter).collect();
    let tracks: Vec<crate::belts::Track> = slots
        .iter()
        .zip(&ids)
        .map(|(slot, id)| crate::belts::Track {
            id,
            body_type: &slot.body_type,
            mass_earth: slot.mass_earth,
            orbit_au: slot.orbit_au,
            eccentricity: slot.eccentricity,
        })
        .collect();
    let belts = crate::belts::draw_belts(&info, f64::from(ice), &tracks, seed);
    let trojans = crate::belts::draw_trojans(f64::from(info.mass), &tracks, seed);
    for body in &mut bodies {
        let planet = body.parent.as_deref().unwrap_or(&body.id);
        let Some(rank) = tracks.iter().position(|t| t.id == planet) else {
            continue;
        };
        if body.kind == BodyKind::Planet {
            body.trojans = trojans[rank];
        }
        body.params.impact_factor = Some(crate::belts::impact_factor(
            &belts,
            &tracks,
            tracks[rank].orbit_au,
        ));
    }
    System {
        format_version: SYSTEM_FORMAT_VERSION,
        request: request.clone(),
        seed,
        width,
        height,
        star: SystemStar {
            spectral: info.kind.clone(),
            designation,
            info,
            metallicity,
            ice_line_au: ice,
        },
        bodies,
        focus: focus_id,
        belts,
    }
}

/// The eccentricity the star's tides leave a planet drawn for `slot` round
/// a star of type `star`: none when they have circularised its orbit, a
/// giant's with its own tides (see `crate::spin`).
fn tidal_eccentricity(slot: &Slot, star: f32) -> f64 {
    let at = StarInfo::new(star, slot.orbit_au as f32, 0.0);
    let circular = if is_giant(&slot.body_type) {
        let mass = slot.mass_earth as f32;
        crate::spin::giant_circularised(
            &at,
            mass,
            crate::giants::giant_radius(mass, at.equilibrium_k),
        )
    } else {
        crate::spin::circularised(&at)
    };
    if circular { 0.0 } else { slot.eccentricity }
}

/// The world parameters of a placed planet, its preset resolved with its
/// own seed at its orbit.
fn planet_params(
    slot: &Slot,
    star: f32,
    metallicity: f32,
    width: u32,
    height: u32,
    seed: u32,
) -> WorldParams {
    let mut rng = StdRng::seed_from_u64(u64::from(seed));
    let giant = is_giant(&slot.body_type);
    let request = WorldRequest {
        planet: Some(slot.body_type.clone()),
        seed: Some(seed),
        star: Some(StarInfo::new(star, 1.0, 0.0).kind),
        orbit_au: Some(slot.orbit_au as f32),
        eccentricity: Some(slot.eccentricity as f32),
        metallicity: Some(metallicity),
        moons: Some(MoonsMode::Auto),
        giants: if giant { None } else { Some(GiantsMode::None) },
        host: if giant { None } else { Some(HostMode::None) },
        mass_earth: giant.then_some(slot.mass_earth as f32),
        width: Some(width),
        height: Some(height),
        ..WorldRequest::default()
    };
    match request.resolve(&mut rng) {
        Ok(resolved) => resolved.params,
        // A preset that refuses this orbit (its own limits): its defaults
        // at the orbit kept as is.
        Err(_) => {
            let request = WorldRequest {
                planet: Some(slot.body_type.clone()),
                seed: Some(seed),
                width: Some(width),
                height: Some(height),
                ..WorldRequest::default()
            };
            let mut params = request
                .resolve(&mut rng)
                .expect("built-in presets resolve")
                .params;
            params.star = star;
            params.orbit_au = slot.orbit_au as f32;
            params.eccentricity = slot.eccentricity as f32;
            params
        }
    }
}

/// What a body gets from its star at `orbit_au`.
fn surroundings_at(star: &StarInfo, orbit_au: f64, eccentricity: f64) -> Surroundings {
    let at = StarInfo::new(star.code(), orbit_au as f32, eccentricity as f32);
    Surroundings {
        flux_earth: f64::from(at.insolation),
        equilibrium_k: f64::from(at.equilibrium_k),
        hz_position: f64::from(at.hz_position),
    }
}

/// The body of a placed planet.
fn planet_body(id: &str, designation: &str, slot: &Slot, params: &WorldParams, star: f32) -> Body {
    let info = StarInfo::new(star, slot.orbit_au as f32, slot.eccentricity as f32);
    let mut rng = StdRng::seed_from_u64(u64::from(body_seed(params.seed, "orbit")));
    let giant = is_giant(&slot.body_type);
    // A giant locks with its own tides (a hot one always); a rocky planet
    // may be locked or held in the 3:2 resonance (see `crate::spin`), its
    // sidereal day then two thirds of its year.
    let (radius_km, mass_earth, period_hours, locked, resonant) = if giant {
        let look = crate::giant_world::Look::draw(params, slot.mass_earth as f32).info;
        let locked =
            look.hot || crate::spin::giant_locked(&info, look.mass_earth, look.radius_earth);
        (
            f64::from(look.radius_earth) * EARTH_M / 1000.0,
            f64::from(look.mass_earth),
            f64::from(look.rotation_hours),
            locked,
            false,
        )
    } else {
        let radius = f64::from(params.circumference_km) / (2.0 * PI);
        let day = MoonSystem::draw(params).map_or(24.0, |m| f64::from(m.planet_day_hours));
        let resonant = info.spin_orbit_3_2;
        let day = if resonant {
            f64::from(info.year_days) * 24.0 / f64::from(crate::spin::RESONANCE_TURNS_PER_ORBIT)
        } else {
            day
        };
        (
            radius,
            (radius / 6371.0).powi(3),
            day,
            info.tidally_locked,
            resonant,
        )
    };
    let volume = 4.0 / 3.0 * PI * (radius_km * 1000.0).powi(3);
    let density = mass_earth * EARTH_KG / volume / 1000.0;
    let gravity = mass_earth / (radius_km / 6371.0).powi(2);
    Body {
        id: id.to_string(),
        designation: designation.to_string(),
        kind: BodyKind::Planet,
        body_type: slot.body_type.clone(),
        parent: None,
        orbit: Orbit {
            semi_major_axis_km: slot.orbit_au * 1.495_978_707e8,
            semi_major_axis_au: slot.orbit_au,
            eccentricity: slot.eccentricity,
            // A degree or two out of the star's plane (Fabrycky et al. 2014).
            inclination_deg: (1.5 * normal(&mut rng)).abs(),
            node_deg: rng.random::<f64>() * 360.0,
            periapsis_deg: rng.random::<f64>() * 360.0,
            mean_anomaly_deg: rng.random::<f64>() * 360.0,
            period_days: f64::from(info.year_days),
        },
        physical: Physical {
            mass_earth,
            radius_km,
            density,
            gravity,
            axes_km: None,
        },
        spin: Spin {
            period_hours: if locked {
                f64::from(info.year_days) * 24.0
            } else {
                period_hours
            },
            axial_tilt_deg: f64::from(params.axial_tilt_deg),
            locked,
            spin_orbit_3_2: resonant,
        },
        surroundings: surroundings_at(&info, slot.orbit_au, slot.eccentricity),
        preview: preview_of(&slot.body_type, params, star),
        params: params.clone(),
        generated: false,
        trojans: None,
        ring_role: None,
    }
}

/// A moon's body fields: its type, parameters, orbit, physical facts and
/// spin.
type MoonParts = (String, WorldParams, Orbit, Physical, Spin);

/// The moons a planet of `planet_mass` Earth masses draws, as a world
/// does ([`MoonSystem::draw`]): a rocky planet's, or a giant's regular
/// moons (one recipe for a giant of a system and a giant world).
fn drawn_moons(planet: &WorldParams, planet_mass: f64) -> Vec<MoonParts> {
    let Some(system) = MoonSystem::draw(planet) else {
        return Vec::new();
    };
    system
        .moons
        .iter()
        .map(|moon| {
            let params = crate::moons::body_params(planet, moon);
            moon_parts(moon, params, planet_mass)
        })
        .collect()
}

/// The fields of a moon drawn by [`MoonSystem`].
fn moon_parts(moon: &Moon, params: WorldParams, planet_mass_earth: f64) -> MoonParts {
    let tilt = params.axial_tilt_deg;
    let body_type = match moon.composition {
        Composition::Rock => "rocky-moon",
        Composition::Ice => "icy-moon",
    };
    // As the viewer drew them before they were stored: a periapsis and a
    // node from the moon's seed, a captured moon's spin of 6 to 24 hours.
    let seed = f64::from(moon.seed);
    let periapsis = (seed % 7919.0) / 7919.0 * 360.0;
    let node = (seed % 6421.0) / 6421.0 * 360.0;
    let locked = moon.tidally_locked || moon.migration == crate::moons::Migration::Locked;
    let period_hours = if locked {
        f64::from(moon.period_days) * 24.0
    } else {
        6.0 + 18.0 * (seed % 9973.0) / 9973.0
    };
    (
        body_type.to_string(),
        params,
        Orbit {
            semi_major_axis_km: f64::from(moon.semi_major_axis_km),
            semi_major_axis_au: f64::from(moon.semi_major_axis_km) / 1.495_978_707e8,
            eccentricity: f64::from(moon.eccentricity),
            inclination_deg: f64::from(moon.inclination_deg),
            node_deg: node,
            periapsis_deg: periapsis,
            mean_anomaly_deg: 0.0,
            period_days: f64::from(moon.period_days),
        },
        Physical {
            mass_earth: f64::from(moon.mass_ratio) * planet_mass_earth,
            radius_km: f64::from(moon.radius_km),
            density: f64::from(moon.density),
            gravity: f64::from(moon.gravity),
            // Irregular, or stretched by its planet's tides.
            axes_km: (moon.axes_km[0] > moon.axes_km[2]).then_some(moon.axes_km),
        },
        Spin {
            period_hours,
            // Its world's: one tilt for both (see `moons::moon_tilt_deg`).
            axial_tilt_deg: f64::from(tilt),
            locked,
            spin_orbit_3_2: false,
        },
    )
}

/// A giant's params with its rings drawn by the system, from a stream of
/// their own and its preset's odds: the star view and the skies show them
/// before the giant is generated, and its generation takes them (see
/// `WorldParams::given_rings`).
fn with_system_rings(mut params: WorldParams) -> WorldParams {
    let odds = params.planet.giant.as_ref().map_or([0.0, 0.0], |g| g.rings);
    let mut rng = StdRng::seed_from_u64(u64::from(stream_seed(params.seed, stream::RINGS)));
    params.given_rings = Some(crate::giant_world::GivenRings {
        rings: crate::giant_world::draw_rings(&mut rng, odds),
        dust: Vec::new(),
    });
    params
}

/// Shapes the rings the system drew for the giant of `params` (see
/// [`crate::rings::shape`]): their Roche edge and their gaps, kept in its
/// params, and the moons that make them, which this returns. A giant too
/// hot or with too small a Hill sphere for regular moons (see
/// `MoonSystem::draw`) keeps its rings without moons.
fn shape_rings(params: &mut WorldParams, slot: &Slot, star: f32) -> Vec<crate::rings::RingMoon> {
    let Some(rings) = params.given_rings.as_mut().and_then(|g| g.rings.take()) else {
        return Vec::new();
    };
    let info = StarInfo::new(star, slot.orbit_au as f32, slot.eccentricity as f32);
    let mass_kg = slot.mass_earth * EARTH_KG;
    let radius_m = f64::from(params.circumference_km) * 1000.0 / (2.0 * PI);
    let density = mass_kg / (4.0 / 3.0 * PI * radius_m.powi(3)) / 1000.0;
    let hill = slot.orbit_au
        * (1.0 - slot.eccentricity)
        * 1.496e11
        * (mass_kg / (3.0 * f64::from(info.mass) * 1.989e30)).cbrt()
        / radius_m;
    let moons = info.equilibrium_k <= 1000.0 && hill / 3.0 >= 6.0;
    let (rings, ring_moons) = crate::rings::shape(rings, density, mass_kg, moons, params.seed);
    if let Some(given) = params.given_rings.as_mut() {
        given.rings = Some(rings);
    }
    ring_moons
}

/// The highest inclination a ring moon of `role` draws, degrees. Rings are
/// tens of metres to a kilometre thick: a moon in or at them stays within
/// a few km of their plane (Pan 0.0001°, Daphnis 0.0036°, Atlas 0.003°),
/// so that seen from it they lie at its horizon. Shepherds and resonant
/// moons, outside the rings, swing a little farther (Prometheus 0.008°,
/// Pandora 0.05°).
fn ring_moon_inclination_deg(role: crate::rings::RingRole) -> f64 {
    use crate::rings::RingRole;
    match role {
        RingRole::Embedded => 0.001,
        RingRole::Edge => 0.005,
        RingRole::Shepherd | RingRole::Resonant => 0.05,
    }
}

/// The eccentricity of a ring moon of `role`, from `frac` in `[0, 1)`. A
/// moon swings `a e` in and out of its circle: a moonlet must stay in its
/// gap, a few tens of km wide (Pan 1.4e-5, Daphnis 3.3e-5, about ±4 km,
/// which raises the waves on the Keeler gap's edges), so it draws up to
/// 5e-5; an edge moon up to 1.5e-3 (Atlas 1.2e-3); shepherds and resonant
/// moons, outside the rings, 2e-4 to 2.2e-3 (Prometheus 2.2e-3).
fn ring_moon_eccentricity(role: crate::rings::RingRole, frac: f64) -> f64 {
    use crate::rings::RingRole;
    match role {
        RingRole::Embedded => 5e-5 * frac,
        RingRole::Edge => 1.5e-3 * frac,
        RingRole::Shepherd | RingRole::Resonant => 0.0002 + 0.002 * frac,
    }
}

/// The fields of a ring moon of the giant of `giant` (`giant_kg`, of
/// radius `giant_radius_km` as its body gives it): a small icy body on a
/// circular orbit in the giant's equatorial plane, locked to it, its
/// angles from its seed.
fn ring_moon_parts(
    giant: &WorldParams,
    moon: &crate::rings::RingMoon,
    giant_kg: f64,
    giant_radius_km: f64,
) -> MoonParts {
    // The rings' gaps are in the radii of the giant's body
    // (`Physical::radius_km`, from an Earth radius of 6 371 km), the unit
    // the views place its moons in: its circumference (from Earth's 40 075
    // km, an equatorial 6 378 km) would put a moonlet 0.11 % further out,
    // a gap's width beside its gap.
    let a_km = moon.a_radii * giant_radius_km;
    let period_days = crate::rings::period_days(a_km, giant_kg);
    let mass = 4.0 / 3.0 * PI * (moon.radius_km * 1e3).powi(3) * moon.density * 1e3;
    let round = moon.shape == [1.0; 3];
    let shape = (!round).then(|| moon.shape.map(|s| s as f32));
    let params = crate::moons::small_body_params(
        giant,
        Composition::Ice,
        moon.radius_km as f32,
        moon.seed,
        shape,
    );
    let seed = f64::from(moon.seed);
    (
        "icy-moon".to_string(),
        params,
        Orbit {
            semi_major_axis_km: a_km,
            semi_major_axis_au: a_km / 1.495_978_707e8,
            eccentricity: ring_moon_eccentricity(moon.role, (seed % 7907.0) / 7907.0),
            inclination_deg: ring_moon_inclination_deg(moon.role) * (seed % 6067.0) / 6067.0,
            node_deg: (seed % 6421.0) / 6421.0 * 360.0,
            periapsis_deg: (seed % 7919.0) / 7919.0 * 360.0,
            mean_anomaly_deg: (seed % 8933.0) / 8933.0 * 360.0,
            period_days,
        },
        Physical {
            mass_earth: mass / EARTH_KG,
            radius_km: moon.radius_km,
            density: moon.density,
            gravity: G * mass / (moon.radius_km * 1e3).powi(2) / 9.81,
            axes_km: shape.map(|s| s.map(|a| a * moon.radius_km as f32)),
        },
        Spin {
            period_hours: period_days * 24.0,
            axial_tilt_deg: 0.0,
            locked: true,
            spin_orbit_3_2: false,
        },
    )
}

/// The moons of the giant a world of a request orbits: the world among
/// its regular moons, as [`HostGiant`] draws them, nearest first.
fn host_chain(host: &HostGiant, world: &WorldParams, star: f32) -> Vec<MoonParts> {
    let giant_kg = f64::from(host.giant.mass_earth) * EARTH_KG;
    let mut moons: Vec<(f64, MoonParts)> = Vec::new();
    let world_radius = f64::from(world.circumference_km) / (2.0 * PI);
    let world_mass = (world_radius / 6371.0).powi(3);
    moons.push((
        f64::from(host.distance_km),
        (
            world.planet.name.clone(),
            world.clone(),
            Orbit {
                semi_major_axis_km: f64::from(host.distance_km),
                semi_major_axis_au: f64::from(host.distance_km) / 1.495_978_707e8,
                eccentricity: f64::from(host.eccentricity),
                inclination_deg: 0.0,
                node_deg: 0.0,
                periapsis_deg: 0.0,
                mean_anomaly_deg: 0.0,
                period_days: f64::from(host.period_days),
            },
            Physical {
                mass_earth: world_mass,
                radius_km: world_radius,
                density: world_mass * EARTH_KG
                    / (4.0 / 3.0 * PI * (world_radius * 1000.0).powi(3))
                    / 1000.0,
                gravity: world_mass / (world_radius / 6371.0).powi(2),
                axes_km: None,
            },
            Spin {
                period_hours: f64::from(host.period_days) * 24.0,
                axial_tilt_deg: f64::from(world.axial_tilt_deg),
                locked: true,
                spin_orbit_3_2: false,
            },
        ),
    ));
    for (k, sibling) in host.siblings.iter().enumerate() {
        let params = crate::exomoon::sibling_params(world, host, k);
        let mass_kg = 4.0 / 3.0
            * PI
            * (f64::from(sibling.radius_km) * 1000.0).powi(3)
            * if sibling.icy { 1.9e3 } else { 3.3e3 };
        moons.push((
            f64::from(sibling.distance_km),
            (
                if sibling.icy {
                    "icy-moon"
                } else {
                    "rocky-moon"
                }
                .to_string(),
                params,
                Orbit {
                    semi_major_axis_km: f64::from(sibling.distance_km),
                    semi_major_axis_au: f64::from(sibling.distance_km) / 1.495_978_707e8,
                    eccentricity: 0.005,
                    inclination_deg: 0.0,
                    node_deg: 0.0,
                    periapsis_deg: 0.0,
                    mean_anomaly_deg: (1.3 + 2.1 * k as f64).to_degrees() % 360.0,
                    period_days: f64::from(sibling.period_days),
                },
                Physical {
                    mass_earth: mass_kg / EARTH_KG,
                    radius_km: f64::from(sibling.radius_km),
                    density: if sibling.icy { 1.9 } else { 3.3 },
                    gravity: (G * mass_kg / (f64::from(sibling.radius_km) * 1000.0).powi(2)) / 9.81,
                    axes_km: None,
                },
                Spin {
                    period_hours: f64::from(sibling.period_days) * 24.0,
                    axial_tilt_deg: 0.0,
                    locked: true,
                    spin_orbit_3_2: false,
                },
            ),
        ));
    }
    let _ = (giant_kg, star);
    moons.sort_by(|a, b| a.0.total_cmp(&b.0));
    moons.into_iter().map(|(_, parts)| parts).collect()
}

/// The preview of a body: colours by type, a giant's from its look.
fn preview_of(body_type: &str, params: &WorldParams, _star: f32) -> Preview {
    let rgb = |r: f32, g: f32, b: f32| [r, g, b];
    let rocky = |colours: Vec<[f32; 3]>, glow: bool| Preview {
        colours,
        bands: 0,
        glow,
    };
    match body_type {
        "terran" => rocky(
            vec![
                rgb(0.05, 0.18, 0.42),
                rgb(0.18, 0.32, 0.12),
                rgb(0.9, 0.92, 0.95),
            ],
            false,
        ),
        "barren" => rocky(vec![rgb(0.32, 0.27, 0.22), rgb(0.24, 0.2, 0.17)], false),
        "caustic" => rocky(vec![rgb(0.78, 0.68, 0.42), rgb(0.7, 0.6, 0.38)], false),
        "frozen" => rocky(vec![rgb(0.82, 0.86, 0.92), rgb(0.7, 0.76, 0.84)], false),
        "volcanic" => rocky(vec![rgb(0.12, 0.08, 0.07), rgb(0.2, 0.1, 0.06)], true),
        "rocky-moon" => rocky(vec![rgb(0.32, 0.3, 0.28)], false),
        "icy-moon" => rocky(vec![rgb(0.72, 0.76, 0.8)], false),
        _ if is_giant(body_type) => {
            let mass = params.giant_mass_earth.unwrap_or(318.0);
            let info = crate::giant_world::Look::draw(params, mass).info;
            let p = info.palette;
            Preview {
                colours: vec![p.zone, p.belt, p.polar],
                bands: (info.jets.min(24)) as u8,
                glow: false,
            }
        }
        _ => rocky(vec![rgb(0.4, 0.4, 0.4)], false),
    }
}

/// Where a job generating a body's neighbourhood is (see
/// [`generate_neighbourhood`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BodyProgress {
    /// The body being generated.
    pub body: String,
    /// Its place in the job (from 1), and how many the job generates.
    pub body_step: u32,
    pub bodies: u32,
    /// The stage it is at, and its place among a body's stages (from 1).
    pub stage: crate::generation::Stage,
    pub step: u32,
    pub steps: u32,
}

/// The bodies opening `body_id` generates, not generated yet, the body
/// first: a planet and its moons; a moon, its planet and the planet's
/// other moons. Empty when they all are (or the id is unknown).
pub fn neighbourhood(system: &System, body_id: &str) -> Vec<String> {
    let Some(body) = system.bodies.iter().find(|b| b.id == body_id) else {
        return Vec::new();
    };
    let planet = body.parent.as_deref().unwrap_or(&body.id);
    let mut ids = vec![body.id.clone()];
    for other in &system.bodies {
        let family = other.id == planet || other.parent.as_deref() == Some(planet);
        if family && other.id != body.id {
            ids.push(other.id.clone());
        }
    }
    ids.retain(|id| system.bodies.iter().any(|b| &b.id == id && !b.generated));
    ids
}

/// Stages a body's generation reports (the moons are bodies of their own).
const BODY_STAGES: usize = 8;

/// Generates the bodies opening `body_id` asks for (see [`neighbourhood`])
/// in the system written in `system_dir`, one by one, each into
/// `bodies/<id>/` with the layout of a world: written into
/// `bodies/.tmp-<id>/`, renamed into place once complete, then marked
/// generated in `system.json`, so a job stopped between two bodies (a
/// cancelled generation unwinds from `progress`) keeps those done and
/// leaves no half body. Its `world.json` names its system and body.
/// Returns the bodies generated, in order.
pub fn generate_neighbourhood(
    system_dir: &std::path::Path,
    body_id: &str,
    progress: impl FnMut(BodyProgress),
) -> Result<Vec<String>, String> {
    generate_neighbourhood_with(system_dir, body_id, false, progress)
}

/// [`generate_neighbourhood`], with the diagnostic noise maps of each body
/// (`bodies/<id>/noise_maps/`, see [`crate::export::NoiseMapWriter`]) when
/// `noise_maps` is set.
pub fn generate_neighbourhood_with(
    system_dir: &std::path::Path,
    body_id: &str,
    noise_maps: bool,
    mut progress: impl FnMut(BodyProgress),
) -> Result<Vec<String>, String> {
    use crate::generation::{GenerateOptions, Stage, generate_with_progress};
    let system = read_system(system_dir)?;
    if !system.bodies.iter().any(|b| b.id == body_id) {
        return Err(format!("no body {body_id} in this system"));
    }
    let ids = neighbourhood(&system, body_id);
    let system_name = system_id(&system);
    let bodies_dir = system_dir.join("bodies");
    std::fs::create_dir_all(&bodies_dir).map_err(|e| e.to_string())?;
    for (k, id) in ids.iter().enumerate() {
        let body = system.bodies.iter().find(|b| &b.id == id).expect("listed");
        let report = std::cell::RefCell::new(&mut progress);
        let at = |stage: Stage| {
            let step = Stage::ALL.iter().position(|s| *s == stage).unwrap_or(0) as u32 + 1;
            (report.borrow_mut())(BodyProgress {
                body: id.clone(),
                body_step: k as u32 + 1,
                bodies: ids.len() as u32,
                stage,
                step: step.min(BODY_STAGES as u32),
                steps: BODY_STAGES as u32,
            });
        };
        let temp = bodies_dir.join(format!(".tmp-{id}"));
        let _ = std::fs::remove_dir_all(&temp);
        // The noise maps are written as the generation computes their
        // layers, each one freed once encoded.
        let writer = if noise_maps {
            Some(
                crate::export::NoiseMapWriter::new(
                    &temp.join("noise_maps"),
                    body.params.width,
                    body.params.height,
                )
                .map_err(|e| e.to_string())?,
            )
        } else {
            None
        };
        let options = GenerateOptions {
            debug: writer.as_ref().map(|w| w as _),
        };
        let generated = generate_with_progress(&body.params, options, at);
        if let Some(writer) = writer {
            writer.finish()?;
        }
        at(Stage::Export);
        crate::export::export_world(
            generated,
            &temp,
            crate::export::ExportOptions { raw_data: true },
        )?;
        name_world(&temp, &system_name, id)?;
        let target = bodies_dir.join(id);
        let _ = std::fs::remove_dir_all(&target);
        std::fs::rename(&temp, &target).map_err(|e| e.to_string())?;
        // Read again: another job may have marked other bodies meanwhile.
        let mut current = read_system(system_dir)?;
        if let Some(entry) = current.bodies.iter_mut().find(|b| &b.id == id) {
            entry.generated = true;
        }
        write_system(system_dir, &current)?;
    }
    Ok(ids)
}

/// Adds the system and the body to a generated body's `world.json`.
fn name_world(dir: &std::path::Path, system: &str, body: &str) -> Result<(), String> {
    let path = dir.join("raw_data").join("world.json");
    let text = std::fs::read(&path).map_err(|e| e.to_string())?;
    let mut json: serde_json::Value = serde_json::from_slice(&text).map_err(|e| e.to_string())?;
    if let Some(map) = json.as_object_mut() {
        map.insert("system".into(), system.into());
        map.insert("body".into(), body.into());
    }
    let out = serde_json::to_vec(&json).map_err(|e| e.to_string())?;
    std::fs::write(&path, out).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rings::RingRole;

    fn whole(planets: &[(&str, u32)], seed: u32) -> SystemRequest {
        SystemRequest::System(WholeSystemRequest {
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

    fn planets(system: &System) -> Vec<&Body> {
        system
            .bodies
            .iter()
            .filter(|b| b.kind == BodyKind::Planet)
            .collect()
    }

    #[test]
    fn a_whole_system_has_the_planets_asked_ordered_and_spaced() {
        let request = whole(
            &[
                ("terran", 2),
                ("barren", 1),
                ("gas-giant", 1),
                ("ice-giant", 1),
            ],
            7,
        );
        let system = draw(&request, &mut StdRng::seed_from_u64(1)).unwrap();
        let p = planets(&system);
        assert_eq!(p.len(), 5);
        assert_eq!(system.focus, None);
        let count = |t: &str| p.iter().filter(|b| b.body_type == t).count();
        assert_eq!(
            (
                count("terran"),
                count("barren"),
                count("gas-giant"),
                count("ice-giant")
            ),
            (2, 1, 1, 1)
        );
        // From the star out, lettered in order, each clear of the last.
        for (k, pair) in p.windows(2).enumerate() {
            assert!(pair[0].orbit.semi_major_axis_au < pair[1].orbit.semi_major_axis_au);
            assert_eq!(pair[0].id, planet_letter(k));
        }
        let star = &system.star.info;
        for b in &p {
            if b.body_type == "terran" {
                let a = b.orbit.semi_major_axis_au;
                assert!(
                    a >= f64::from(star.hz_inner_au) * 0.95
                        && a <= f64::from(star.hz_outer_au) * 1.05
                );
            }
            if b.body_type == "gas-giant" {
                assert!(b.orbit.semi_major_axis_au >= 0.5 * f64::from(system.star.ice_line_au));
            }
        }
        // Moons follow their planet, nearest first, with their ids.
        for moon in system.bodies.iter().filter(|b| b.kind == BodyKind::Moon) {
            let parent = moon.parent.as_ref().unwrap();
            assert!(moon.id.starts_with(&format!("{parent}-")));
        }
        assert_eq!(
            system_id(&system),
            format!(
                "{}-7",
                system.star.designation.to_lowercase().replace(' ', "-")
            )
        );
    }

    #[test]
    fn the_same_request_draws_the_same_system() {
        let request = whole(&[("terran", 1), ("gas-giant", 2)], 42);
        let a = draw(&request, &mut StdRng::seed_from_u64(1)).unwrap();
        let b = draw(&request, &mut StdRng::seed_from_u64(99)).unwrap();
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap()
        );
    }

    #[test]
    fn requests_beyond_the_limits_fail_with_their_reason() {
        let too_many = whole(&[("barren", 16)], 1);
        assert!(matches!(
            draw(&too_many, &mut StdRng::seed_from_u64(1)),
            Err(RequestError::TooManyPlanets { asked: 16, max: 15 })
        ));
        let crowded = whole(&[("terran", 12)], 1);
        match draw(&crowded, &mut StdRng::seed_from_u64(1)) {
            Err(RequestError::ZoneFull {
                body_type,
                asked,
                fits,
            }) => {
                assert_eq!((body_type.as_str(), asked), ("terran", 12));
                assert!((1..12).contains(&fits));
            }
            other => panic!("expected zone_full, got {other:?}"),
        }
        assert!(matches!(
            draw(
                &whole(&[("gas-dwarf", 1)], 1),
                &mut StdRng::seed_from_u64(1)
            ),
            Err(RequestError::UnknownType(_))
        ));
    }

    #[test]
    fn a_drawn_system_keeps_at_most_fifteen_planets() {
        for seed in 0..40 {
            let request = SystemRequest::System(WholeSystemRequest {
                seed: Some(seed),
                width: Some(64),
                height: Some(32),
                ..WholeSystemRequest::default()
            });
            let system = draw(&request, &mut StdRng::seed_from_u64(1)).unwrap();
            assert!(planets(&system).len() <= MAX_PLANETS);
        }
    }

    #[test]
    fn a_system_around_a_planet_keeps_it_as_asked() {
        let world = WorldRequest {
            planet: Some("terran".into()),
            seed: Some(1234),
            width: Some(64),
            height: Some(32),
            ..WorldRequest::default()
        };
        let asked = world.resolve(&mut StdRng::seed_from_u64(5)).unwrap().params;
        let system = draw(&SystemRequest::Around(world), &mut StdRng::seed_from_u64(5)).unwrap();
        let focus = system
            .bodies
            .iter()
            .find(|b| (b.orbit.semi_major_axis_au - f64::from(asked.orbit_au)).abs() < 1e-6)
            .expect("the planet asked");
        assert_eq!(focus.params.seed, asked.seed);
        assert_eq!(focus.params.circumference_km, asked.circumference_km);
        assert_eq!(focus.body_type, "terran");
        assert_eq!(system.seed, 1234);
        assert_eq!(system.focus.as_deref(), Some(focus.id.as_str()));
        // Another epoch of the same planet is another system.
        let drifted = WorldRequest {
            planet: Some("terran".into()),
            seed: Some(1234),
            drift_ma: Some(-250.0),
            width: Some(64),
            height: Some(32),
            ..WorldRequest::default()
        };
        let other = draw(
            &SystemRequest::Around(drifted),
            &mut StdRng::seed_from_u64(5),
        )
        .unwrap();
        assert!(system_id(&other).ends_with("-past250ma-1234"));
        assert_ne!(system_id(&other), system_id(&system));
    }

    #[test]
    fn giants_get_their_rings_from_the_system() {
        let (mut bright, mut faint, mut giants) = (0, 0, 0);
        for seed in 0..40 {
            let system = draw(
                &whole(&[("gas-giant", 2), ("ice-giant", 1), ("terran", 1)], seed),
                &mut StdRng::seed_from_u64(u64::from(seed)),
            )
            .unwrap();
            for body in &system.bodies {
                if body.kind == BodyKind::Planet && is_giant(&body.body_type) {
                    giants += 1;
                    let given = body
                        .params
                        .given_rings
                        .as_ref()
                        .expect("rings drawn with the system");
                    match &given.rings {
                        Some(r) if r.bright => bright += 1,
                        Some(_) => faint += 1,
                        None => {}
                    }
                    if body.body_type == "ice-giant" {
                        assert!(given.rings.as_ref().is_none_or(|r| !r.bright));
                    }
                    // The giant world takes them, and nothing else of its look moves.
                    let mass = body.params.giant_mass_earth.unwrap();
                    let look = crate::giant_world::Look::draw(&body.params, mass).info;
                    assert_eq!(look.rings, given.rings);
                    let own = WorldParams {
                        given_rings: None,
                        impact_factor: None,
                        ..body.params.clone()
                    };
                    let mut alone = crate::giant_world::Look::draw(&own, mass).info;
                    alone.rings = look.rings.clone();
                    assert_eq!(alone, look);
                } else {
                    assert!(body.params.given_rings.is_none(), "{}", body.id);
                }
            }
        }
        assert!(giants > 60, "{giants}");
        assert!(bright > 0 && faint > bright, "{bright} {faint}");
    }

    #[test]
    fn ring_moons_open_the_gaps_and_join_the_moons() {
        let (mut embedded, mut resonant, mut edge, mut shepherds, mut dusts) = (0, 0, 0, 0, 0);
        for seed in 0..40 {
            let system = draw(
                &whole(&[("gas-giant", 2), ("ice-giant", 2)], seed),
                &mut StdRng::seed_from_u64(u64::from(seed)),
            )
            .unwrap();
            for giant in system.bodies.iter().filter(|b| b.kind == BodyKind::Planet) {
                let moons: Vec<&Body> = system
                    .bodies
                    .iter()
                    .filter(|b| b.parent.as_deref() == Some(&giant.id))
                    .collect();
                // Nearest first, numbered in that order.
                assert!(
                    moons
                        .windows(2)
                        .all(|w| w[0].orbit.semi_major_axis_km <= w[1].orbit.semi_major_axis_km)
                );
                for (k, m) in moons.iter().enumerate() {
                    assert_eq!(m.id, format!("{}-{}", giant.id, k + 1));
                }
                // Every dust ring has its moon, among the giant's.
                for dust in giant.params.given_rings.iter().flat_map(|g| &g.dust) {
                    assert!(moons.iter().any(|m| m.id == dust.source), "{}", dust.source);
                    dusts += 1;
                }
                let rings = giant
                    .params
                    .given_rings
                    .as_ref()
                    .and_then(|g| g.rings.as_ref());
                let ring_moons: Vec<&&Body> =
                    moons.iter().filter(|m| m.ring_role.is_some()).collect();
                let Some(rings) = rings else {
                    assert!(ring_moons.is_empty());
                    continue;
                };
                // Every gap has its moon, among the giant's.
                assert_eq!(rings.gap_moons.len(), rings.gaps.len());
                for id in &rings.gap_moons {
                    let moon = moons.iter().find(|m| &m.id == id).expect("the gap's moon");
                    assert!(matches!(
                        moon.ring_role,
                        Some(RingRole::Embedded | RingRole::Resonant)
                    ));
                }
                if rings.bright {
                    assert!((2.0..=2.5).contains(&rings.outer));
                }
                let radius = giant.physical.radius_km;
                for m in ring_moons {
                    let a = m.orbit.semi_major_axis_km / radius;
                    assert!(m.body_type == "icy-moon" && m.spin.locked);
                    // In the rings' plane, within a few km of it.
                    let role = m.ring_role.unwrap();
                    assert!(m.orbit.inclination_deg <= ring_moon_inclination_deg(role));
                    // Its swing in and out stays within the rings' scale.
                    assert!(m.orbit.eccentricity < ring_moon_eccentricity(role, 1.0));
                    if matches!(role, RingRole::Embedded) {
                        let off_km =
                            m.orbit.semi_major_axis_km * m.orbit.inclination_deg.to_radians();
                        assert!(off_km < 5.0, "{off_km}");
                        // Well inside its gap.
                        let swing_km = m.orbit.semi_major_axis_km * m.orbit.eccentricity;
                        assert!(swing_km < 10.0, "{swing_km}");
                    }
                    match m.ring_role.unwrap() {
                        RingRole::Embedded => {
                            embedded += 1;
                            assert!(a > f64::from(rings.inner) && a < f64::from(rings.outer));
                            // In the middle of its gap, in the radii the
                            // views use: a tenth of its width at most off.
                            let k = rings
                                .gap_moons
                                .iter()
                                .position(|id| id == &m.id)
                                .expect("its gap");
                            let [centre, width] = rings.gaps[k];
                            let off = (a - f64::from(centre)).abs() / f64::from(width);
                            assert!(off < 0.1, "{}: {off} of its gap's width off", m.id);
                        }
                        RingRole::Resonant => resonant += 1,
                        RingRole::Edge => {
                            edge += 1;
                            assert!(a > f64::from(rings.outer));
                        }
                        RingRole::Shepherd => shepherds += 1,
                    }
                    // Inside the regular moons, which keep their draws.
                    let first_regular = moons
                        .iter()
                        .find(|r| r.ring_role.is_none())
                        .map(|r| r.orbit.semi_major_axis_km);
                    assert!(first_regular.is_none_or(|r| m.orbit.semi_major_axis_km < r));
                }
            }
        }
        assert!(dusts > 10, "{dusts}");
        assert!(
            embedded > 5 && resonant > 5 && edge > 10 && shepherds > 10,
            "{embedded} {resonant} {edge} {shepherds}"
        );
    }

    #[test]
    fn rings_given_by_a_system_survive_its_system_json() {
        let system = draw(
            &whole(&[("gas-giant", 3)], 3),
            &mut StdRng::seed_from_u64(3),
        )
        .unwrap();
        let json = serde_json::to_string(&system).unwrap();
        let back: System = serde_json::from_str(&json).unwrap();
        for (a, b) in system.bodies.iter().zip(&back.bodies) {
            assert_eq!(a.params.given_rings, b.params.given_rings);
        }
        // A system drawn before has none, and reads as it was.
        let old = json.replace("\"given_rings\"", "\"unknown_field\"");
        let back: System = serde_json::from_str(&old).unwrap();
        assert!(back.bodies.iter().all(|b| b.params.given_rings.is_none()));
    }

    #[test]
    fn a_giants_moon_puts_its_giant_on_its_orbit() {
        let world = WorldRequest {
            planet: Some("terran".into()),
            seed: Some(8),
            host: Some(HostMode::Giant),
            width: Some(64),
            height: Some(32),
            ..WorldRequest::default()
        };
        let system = draw(&SystemRequest::Around(world), &mut StdRng::seed_from_u64(5)).unwrap();
        let giant = system
            .bodies
            .iter()
            .find(|b| b.kind == BodyKind::Planet && is_giant(&b.body_type))
            .expect("the giant");
        let moons: Vec<_> = system
            .bodies
            .iter()
            .filter(|b| b.parent.as_deref() == Some(&giant.id))
            .collect();
        assert!(moons.iter().any(|m| m.body_type == "terran"));
        let focus = system.focus.as_deref().expect("the world asked");
        assert!(
            moons
                .iter()
                .any(|m| m.id == focus && m.body_type == "terran")
        );
        assert!(moons.len() >= 2);
    }

    #[test]
    fn opening_a_body_generates_it_and_its_family_once() {
        // A rocky planet with moons, small maps.
        let request = SystemRequest::Around(WorldRequest {
            planet: Some("barren".into()),
            seed: Some(31),
            moons: Some(MoonsMode::Count(2)),
            width: Some(64),
            height: Some(32),
            ..WorldRequest::default()
        });
        let system = draw(&request, &mut StdRng::seed_from_u64(1)).unwrap();
        let dir = std::env::temp_dir().join(format!("mapgen-system-gen-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        write_system(&dir, &system).unwrap();
        let planet = system
            .bodies
            .iter()
            .find(|b| {
                b.kind == BodyKind::Planet
                    && b.body_type == "barren"
                    && system
                        .bodies
                        .iter()
                        .any(|m| m.parent.as_ref() == Some(&b.id))
            })
            .expect("the barren planet and its moons");
        let moon = system
            .bodies
            .iter()
            .find(|b| b.parent.as_ref() == Some(&planet.id))
            .unwrap();
        // From a moon: the moon first, then its planet and its siblings.
        let family = neighbourhood(&system, &moon.id);
        assert_eq!(family[0], moon.id);
        assert!(family.contains(&planet.id));
        let mut seen = Vec::new();
        let done = generate_neighbourhood(&dir, &moon.id, |p| seen.push(p)).unwrap();
        assert_eq!(done, family);
        for id in &done {
            let world = dir
                .join("bodies")
                .join(id)
                .join("raw_data")
                .join("world.json");
            let json: serde_json::Value =
                serde_json::from_slice(&std::fs::read(world).unwrap()).unwrap();
            assert_eq!(json["body"], serde_json::Value::from(id.as_str()));
        }
        let back = read_system(&dir).unwrap();
        assert!(
            back.bodies
                .iter()
                .filter(|b| done.contains(&b.id))
                .all(|b| b.generated)
        );
        // Every body reported its stages, in order, under its place in the job.
        let last = seen.last().unwrap();
        assert_eq!(
            (last.body_step, last.bodies),
            (done.len() as u32, done.len() as u32)
        );
        assert!(
            seen.windows(2)
                .all(|w| (w[0].body_step, w[0].step) <= (w[1].body_step, w[1].step))
        );
        // Asked again: nothing left to make.
        assert!(
            generate_neighbourhood(&dir, &planet.id, |_| {})
                .unwrap()
                .is_empty()
        );
        assert!(generate_neighbourhood(&dir, "zz", |_| {}).is_err());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_system_round_trips_through_its_file() {
        let system = draw(
            &whole(&[("terran", 1), ("ice-giant", 1)], 3),
            &mut StdRng::seed_from_u64(1),
        )
        .unwrap();
        let dir = std::env::temp_dir().join(format!("mapgen-system-{}", std::process::id()));
        write_system(&dir, &system).unwrap();
        let back = read_system(&dir).unwrap();
        assert_eq!(
            serde_json::to_string(&system).unwrap(),
            serde_json::to_string(&back).unwrap()
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn rocky_planets_keep_out_of_the_first_cold_giants_belt_zone() {
        let mut zones = 0;
        for seed in 0..200 {
            let system = draw(
                &whole_drawn(seed),
                &mut StdRng::seed_from_u64(u64::from(seed)),
            )
            .unwrap();
            let ice = f64::from(system.star.ice_line_au);
            let planets: Vec<&Body> = planets(&system);
            let Some(giant) = planets
                .iter()
                .filter(|b| crate::belts::cold_giant(&b.body_type, b.orbit.semi_major_axis_au, ice))
                .map(|b| b.orbit.semi_major_axis_au)
                .reduce(f64::min)
            else {
                continue;
            };
            zones += 1;
            let (lo, hi) = crate::belts::belt_zone(giant);
            for p in planets.iter().filter(|b| !is_giant(&b.body_type)) {
                let (a, e) = (p.orbit.semi_major_axis_au, p.orbit.eccentricity);
                assert!(
                    a * (1.0 + e) <= lo || a * (1.0 - e) >= hi,
                    "seed {seed}: {} at {a} in {lo}–{hi}",
                    p.id
                );
            }
        }
        assert!(zones > 20, "{zones}");
    }

    #[test]
    fn systems_get_belts_trojans_and_their_crater_rate() {
        let (mut inner, mut trojans) = (0, 0);
        for seed in 0..40 {
            let system = draw(
                &whole(&[("terran", 1), ("gas-giant", 1), ("ice-giant", 1)], seed),
                &mut StdRng::seed_from_u64(u64::from(seed)),
            )
            .unwrap();
            inner += system
                .belts
                .iter()
                .filter(|b| b.kind == crate::belts::BeltKind::Inner)
                .count();
            trojans += system.bodies.iter().filter(|b| b.trojans.is_some()).count();
            for body in &system.bodies {
                assert!(
                    body.params.impact_factor.is_some_and(|f| f > 0.0),
                    "{}",
                    body.id
                );
                assert!(body.kind == BodyKind::Planet || body.trojans.is_none());
            }
            // A moon strikes at its planet's rate.
            for moon in system.bodies.iter().filter(|b| b.kind == BodyKind::Moon) {
                let planet = system
                    .bodies
                    .iter()
                    .find(|b| Some(&b.id) == moon.parent.as_ref())
                    .unwrap();
                assert_eq!(moon.params.impact_factor, planet.params.impact_factor);
            }
        }
        assert!(inner > 10 && trojans > 10, "{inner} {trojans}");
    }

    #[test]
    fn systems_drawn_before_small_bodies_read_as_they_were() {
        let system = draw(
            &whole(&[("terran", 1), ("gas-giant", 1)], 7),
            &mut StdRng::seed_from_u64(7),
        )
        .unwrap();
        let mut json: serde_json::Value = serde_json::to_value(&system).unwrap();
        json.as_object_mut().unwrap().remove("belts");
        for body in json["bodies"].as_array_mut().unwrap() {
            body.as_object_mut().unwrap().remove("trojans");
            body["params"]
                .as_object_mut()
                .unwrap()
                .remove("impact_factor");
        }
        let old: System = serde_json::from_value(json).unwrap();
        assert!(old.belts.is_empty());
        assert!(
            old.bodies
                .iter()
                .all(|b| b.trojans.is_none() && b.params.impact_factor.is_none())
        );
        // Written back, it stays as it was.
        let back: serde_json::Value = serde_json::to_value(&old).unwrap();
        assert!(back.get("belts").is_none());
    }

    fn whole_drawn(seed: u32) -> SystemRequest {
        SystemRequest::System(WholeSystemRequest {
            star: Some("G2V".into()),
            metallicity: Some(0.2),
            planets: None,
            seed: Some(seed),
            width: Some(64),
            height: Some(32),
        })
    }
}

/// The star's tides on a system's planets, and its moons' spins (see
/// `crate::spin`).
#[cfg(test)]
mod spin_tests {
    use super::*;

    fn red_dwarf_system(seed: u32) -> System {
        let request = SystemRequest::System(WholeSystemRequest {
            star: Some("M4V".into()),
            metallicity: Some(0.0),
            planets: None,
            seed: Some(seed),
            width: Some(64),
            height: Some(32),
        });
        draw(&request, &mut rand::rng()).unwrap()
    }

    #[test]
    fn planets_locked_to_their_star_turn_on_circles_upright() {
        let (mut locked, mut resonant) = (0, 0);
        for seed in 0..60 {
            let system = red_dwarf_system(seed);
            let star = parse_type(&system.star.spectral).unwrap();
            for body in system.bodies.iter().filter(|b| b.kind == BodyKind::Planet) {
                if is_giant(&body.body_type) {
                    continue;
                }
                let info = StarInfo::new(
                    star,
                    body.orbit.semi_major_axis_au as f32,
                    body.orbit.eccentricity as f32,
                );
                assert_eq!(body.spin.locked, info.tidally_locked, "{}", body.id);
                assert_eq!(body.spin.spin_orbit_3_2, info.spin_orbit_3_2, "{}", body.id);
                if body.spin.locked {
                    locked += 1;
                    assert!(body.orbit.eccentricity < 0.06, "{}", body.id);
                }
                // A circularised orbit is a circle, and its planet locked.
                if crate::spin::circularised(&info) {
                    assert_eq!(body.orbit.eccentricity, 0.0, "{}", body.id);
                    assert!(body.spin.locked, "{}", body.id);
                }
                if body.spin.spin_orbit_3_2 {
                    resonant += 1;
                    // Three turns every two orbits.
                    let year_h = body.orbit.period_days * 24.0;
                    assert!((body.spin.period_hours * 1.5 / year_h - 1.0).abs() < 1e-3);
                    assert!(body.orbit.eccentricity > 0.0);
                }
                if crate::spin::despun(&info) {
                    assert_eq!(body.spin.axial_tilt_deg, 0.0, "{}", body.id);
                }
            }
        }
        assert!(
            locked > 0 && resonant > 0,
            "{locked} locked, {resonant} resonant"
        );
    }

    #[test]
    fn a_moons_spin_and_world_share_one_tilt_and_giants_one_recipe() {
        for seed in 0..20 {
            let request = SystemRequest::System(WholeSystemRequest {
                star: Some("G2V".into()),
                metallicity: Some(0.0),
                planets: Some(
                    [("terran".to_string(), 2), ("gas-giant".to_string(), 1)]
                        .into_iter()
                        .collect(),
                ),
                seed: Some(seed),
                width: Some(64),
                height: Some(32),
            });
            let system = draw(&request, &mut rand::rng()).unwrap();
            for moon in system.bodies.iter().filter(|b| b.kind == BodyKind::Moon) {
                if moon.ring_role.is_none() {
                    assert_eq!(
                        moon.spin.axial_tilt_deg,
                        f64::from(moon.params.axial_tilt_deg),
                        "{}",
                        moon.id
                    );
                }
            }
            // A giant's regular moons are those the giant world draws.
            for giant in system.bodies.iter().filter(|b| b.body_type == "gas-giant") {
                let drawn = MoonSystem::draw(&giant.params).map_or(0, |m| m.moons.len());
                let regular = system
                    .bodies
                    .iter()
                    .filter(|b| b.parent.as_deref() == Some(&giant.id) && b.ring_role.is_none())
                    .count();
                assert_eq!(regular, drawn, "{}", giant.id);
            }
        }
    }
}
