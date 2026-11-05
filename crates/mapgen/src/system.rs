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
//! sharing about 10⁻⁴ of its mass, as Jupiter's ([`regular_moons`]).
//!
//! **Determinism.** A system is its request and its seed: each body's seed
//! comes from the system's seed and the body's id, and its parameters are
//! stored, so a body generated later is the world it would have been at
//! once.

use crate::exomoon::{
    EARTH_KG, EARTH_M, G, HostGiant, HostMode, SATELLITE_MASS_SHARE, SIBLING_SPACING,
};
use crate::giants::{GiantKind, GiantSystem, GiantsMode, JUPITER_EARTH_MASSES, giant_radius};
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
/// with dashes (`mgs-48213-42`).
pub fn system_id(system: &System) -> String {
    let slug: String = system
        .star
        .designation
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    format!("{slug}-{}", system.seed)
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
/// stay, the others around them). `None` names the first type pushed out
/// of its band.
fn space(slots: &mut Vec<Slot>, star_mass: f64, hz: (f64, f64), ice: f64) -> Result<(), String> {
    slots.sort_by(|a, b| a.orbit_au.total_cmp(&b.orbit_au));
    let mut placed: Vec<Slot> = slots.iter().filter(|s| s.fixed).cloned().collect();
    let mut free: Vec<Slot> = slots.iter().filter(|s| !s.fixed).cloned().collect();
    free.sort_by(|a, b| a.orbit_au.total_cmp(&b.orbit_au));
    for mut slot in free {
        let ((_, max), _) = band(&slot.body_type, hz, ice).expect("known type");
        // Out in small steps until clear of every planet placed.
        while !placed.iter().all(|other| clear(&slot, other, star_mass)) {
            slot.orbit_au *= 1.02;
            if slot.orbit_au > max.min(OUTER_EDGE_AU) {
                return Err(slot.body_type);
            }
        }
        placed.push(slot);
    }
    placed.sort_by(|a, b| a.orbit_au.total_cmp(&b.orbit_au));
    *slots = placed;
    Ok(())
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
                }
            }
            list
        }
        None => drawn_types(&star, f64::from(metallicity), &mut rng),
    };
    let packed = asked.len() >= 4;
    let mut last_error = None;
    for _ in 0..TRIES {
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
        let planet = planet_body(&id, &planet_designation, slot, &params, star_code);
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
                _ => regular_moons(&params, slot, star_code, planet_seed),
            }
        } else {
            rocky_moons(&params)
        };
        for (k, (moon_type, moon_params, orbit, physical, spin)) in moons.into_iter().enumerate() {
            let moon_id = format!("{id}-{}", k + 1);
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
            });
        }
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
    }
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
    let (radius_km, mass_earth, period_hours, locked) = if giant {
        let look = crate::giant_world::Look::draw(params, slot.mass_earth as f32).info;
        (
            f64::from(look.radius_earth) * EARTH_M / 1000.0,
            f64::from(look.mass_earth),
            f64::from(look.rotation_hours),
            info.tidally_locked,
        )
    } else {
        let radius = f64::from(params.circumference_km) / (2.0 * PI);
        let day = MoonSystem::draw(params).map_or(24.0, |m| f64::from(m.planet_day_hours));
        let locked = info.tidally_locked;
        (radius, (radius / 6371.0).powi(3), day, locked)
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
        },
        surroundings: surroundings_at(&info, slot.orbit_au, slot.eccentricity),
        preview: preview_of(&slot.body_type, params, star),
        params: params.clone(),
        generated: false,
    }
}

/// A moon's body fields: its type, parameters, orbit, physical facts and
/// spin.
type MoonParts = (String, WorldParams, Orbit, Physical, Spin);

/// The moons a rocky planet draws, as a world does.
fn rocky_moons(planet: &WorldParams) -> Vec<MoonParts> {
    let Some(system) = MoonSystem::draw(planet) else {
        return Vec::new();
    };
    let planet_mass = (f64::from(planet.circumference_km) / 40_075.0).powi(3);
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
            axes_km: (!moon.round).then_some(moon.axes_km),
        },
        Spin {
            period_hours,
            axial_tilt_deg: f64::from(params_tilt()),
            locked,
        },
    )
}

/// A moon's axial tilt: upright, as the moons' worlds are drawn.
fn params_tilt() -> f32 {
    0.0
}

/// The regular moons of a giant: two to four on a resonant chain (each
/// period twice the last) from 6 of its radii out, inside a third of its
/// Hill radius, sharing about 10⁻⁴ of its mass (Canup & Ward 2006); icy
/// beyond its own snow line, as Ganymede and Callisto.
fn regular_moons(giant: &WorldParams, slot: &Slot, star: f32, seed: u32) -> Vec<MoonParts> {
    let mut rng = StdRng::seed_from_u64(u64::from(stream_seed(seed, stream::SIBLINGS)));
    let info = StarInfo::new(star, slot.orbit_au as f32, slot.eccentricity as f32);
    let mass_kg = slot.mass_earth * EARTH_KG;
    let radius_m = f64::from(giant_radius(slot.mass_earth as f32, info.equilibrium_k)) * EARTH_M;
    let hill = slot.orbit_au
        * (1.0 - slot.eccentricity)
        * 1.496e11
        * (mass_kg / (3.0 * f64::from(info.mass) * 1.989e30)).cbrt()
        / radius_m;
    // Hot giants keep none: their Hill sphere is too small.
    if info.equilibrium_k > 1000.0 || hill / 3.0 < 6.0 {
        return Vec::new();
    }
    let count = 2 + (rng.random::<f64>() * 3.0) as usize;
    let shares: Vec<f64> = (0..count).map(|_| 0.3 + rng.random::<f64>()).collect();
    let total: f64 = shares.iter().sum();
    let first = 6.0 * (1.0 + rng.random::<f64>());
    let mut out = Vec::new();
    for (k, share) in shares.iter().enumerate() {
        let radii = first * SIBLING_SPACING.powi(k as i32);
        if radii > hill / 3.0 {
            break;
        }
        let icy = info.equilibrium_k < 150.0 || radii > 12.0;
        let density = if icy { 1.9e3 } else { 3.3e3 };
        let mass = SATELLITE_MASS_SHARE * mass_kg * share / total;
        let radius_km = (3.0 * mass / (4.0 * PI * density)).cbrt() / 1000.0;
        let a_m = radii * radius_m;
        let period_days = 2.0 * PI * (a_m.powi(3) / (G * mass_kg)).sqrt() / 86_400.0;
        let moon_seed = stream_seed(seed, stream::SIBLINGS + 1 + k as u32);
        let composition = if icy {
            Composition::Ice
        } else {
            Composition::Rock
        };
        let params =
            crate::moons::small_body_params(giant, composition, radius_km as f32, moon_seed, None);
        let gravity = (G * mass / (radius_km * 1000.0).powi(2)) / 9.81;
        out.push((
            if icy { "icy-moon" } else { "rocky-moon" }.to_string(),
            params,
            Orbit {
                semi_major_axis_km: a_m / 1000.0,
                semi_major_axis_au: a_m / 1.495_978_707e11,
                eccentricity: 0.001 + 0.01 * rng.random::<f64>(),
                inclination_deg: 0.5 * rng.random::<f64>(),
                node_deg: rng.random::<f64>() * 360.0,
                periapsis_deg: rng.random::<f64>() * 360.0,
                mean_anomaly_deg: rng.random::<f64>() * 360.0,
                period_days,
            },
            Physical {
                mass_earth: mass / EARTH_KG,
                radius_km,
                density: density / 1000.0,
                gravity,
                axes_km: None,
            },
            Spin {
                period_hours: period_days * 24.0,
                axial_tilt_deg: 0.0,
                locked: true,
            },
        ));
    }
    out
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
}
