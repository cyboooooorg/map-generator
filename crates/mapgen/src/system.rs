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
//! [`hill_spacing`] mutual Hill radii from the last, periapsis to
//! apoapsis, the spacing that keeps systems stable for billions of years:
//! 10 for Earth-mass planets, fewer for heavier ones, 6.4 for a Jupiter
//! and a Saturn (Chambers et al. 1996; Pu & Wu 2015); a hot Jupiter keeps others beyond
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

use crate::activity::{StarActivity, star_activity};
use crate::archetype::Archetype;
use crate::exomoon::{EARTH_KG, EARTH_M, G, HostGiant, HostMode};
use crate::export::Diagnostics;
use crate::giants::{
    GiantKind, GiantSystem, GiantsMode, HOT_JUPITERS, HOT_PERIOD_DAYS, hill_spacing,
};
use crate::moons::{Composition, Moon, MoonSystem, MoonsMode};
use crate::naming::{planet_letter, roman, star_name};
use crate::noise::{stream, stream_seed};
use crate::planet::interim::{base_of, params_for};
use crate::planet::mass::{Origin, draw_mass};
use crate::planet::{Context, PlanetClass, PlanetModel, derive_model, derive_model_with};
use crate::request::{RequestError, WorldRequest};
use crate::star::{StarInfo, draw_eccentricity, parse_type};
use crate::world::WorldParams;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::f64::consts::PI;
use std::sync::Arc;

/// Version of `system.json`, and of the layout of the bodies beside it.
/// Version 2 keeps each resolution of a body in its own directory (see
/// [`crate::sizes`]); version 3 stores each planet's physical model
/// ([`Body::model`]) and types it by its class. A system of another
/// version is refused by [`read_system`], not migrated.
pub const SYSTEM_FORMAT_VERSION: u32 = 3;
/// Most planets a system holds, moons not counted.
pub const MAX_PLANETS: usize = 15;
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

/// The names a whole system may ask planets by: the built-in archetypes
/// ([`crate::archetype`]).
pub fn archetype_names() -> Vec<String> {
    crate::archetype::built_in_names()
        .into_iter()
        .map(str::to_string)
        .collect()
}

/// What a system is drawn from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum SystemRequest {
    /// A whole system: its star and its planets by archetype.
    System(WholeSystemRequest),
    /// A system around one planet, as a world is asked for.
    Around(WorldRequest),
}

impl SystemRequest {
    /// Whether this is a lone planet: a system around a planet that holds
    /// no other planet (see [`WorldRequest::alone`]).
    pub fn is_alone(&self) -> bool {
        matches!(self, SystemRequest::Around(w) if w.alone == Some(true))
    }
}

/// A whole system: a star and a number of planets of each archetype.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(default)]
pub struct WholeSystemRequest {
    /// Spectral type of the star (`G2V`); drawn like the stars nearby when
    /// left out.
    pub star: Option<String>,
    /// `[Fe/H]` of the star, dex; drawn when left out.
    pub metallicity: Option<f32>,
    /// Age of the star, Gyr, in `(0, lifetime]` of its spectral type;
    /// drawn from the seed ([`crate::star::draw_age_gyr`]) when left out.
    /// Every planet and moon of the system shares it.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub age_gyr: Option<f32>,
    /// The planets asked for, by built-in archetype name
    /// ([`archetype_names`]), each at its archetype's insolation (or
    /// period), mass and inputs; the whole system drawn when left out. At most [`MAX_PLANETS`] in all.
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
    /// The star's rotation, magnetic activity and surface
    /// ([`crate::activity::star_activity`] of `info` and the system's
    /// seed). Absent from systems drawn before it existed: [`read_system`]
    /// derives it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub activity: Option<StarActivity>,
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
    /// A planet's class ([`PlanetClass::key`], one of
    /// [`crate::planet::PLANET_CLASSES`]); the preset the world a request
    /// around one asked for was named by; a moon's preset, `rocky-moon` or
    /// `icy-moon`.
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
    /// Whether its maps are in `bodies/<id>/`: its thumbnail, and its
    /// original resolution (its params' size) in
    /// `bodies/<id>/sizes/<W>x<H>/` (see [`crate::sizes`]).
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
    /// A planet's physical model, as [`derive_model`] gave it when the
    /// system was drawn, or as the resolution of its request gave the world
    /// a system is drawn around (a giant's moon included); none for any
    /// other moon (part 4 of #423).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub model: Option<PlanetModel>,
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
    /// star at each periapsis (see `crate::spin`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub spin_orbit_3_2: bool,
    /// Turns per orbit of the spin-orbit resonance its star's tides hold
    /// it in: 1.5 (3:2) or 2 (2:1), drawn by its eccentricity
    /// (see `crate::spin::capture`); absent for a body in none.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f32>", optional))]
    pub resonance_turns: f32,
}

fn is_zero(x: &f32) -> bool {
    *x == 0.0
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
/// following as its poles; giants with `bands` bands between `edges`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Preview {
    /// Linear RGB (a giant's palette is sRGB, converted).
    pub colours: Vec<[f32; 3]>,
    /// The bands of a giant, as it will be generated (0 for a rocky body).
    pub bands: u8,
    /// Latitudes in degrees of the `bands + 1` edges of a giant's bands,
    /// ascending, from the south edge of its polar region to the north
    /// edge of the other: the band between the `k`-th and the next edge is
    /// a belt for even `k`, a zone for odd `k`, in `colours[1]` and
    /// `colours[0]`; beyond the outer edges the polar colour. Empty on
    /// previews made before it existed: the viewer then spaces `bands`
    /// bands evenly.
    #[serde(default)]
    pub edges: Vec<f32>,
    /// Lava glowing on its night side.
    pub glow: bool,
}

/// The id of the system: its star's designation and its seed, lowercase
/// with dashes (`mgs-48213-42`); a system around a planet whose plates are
/// moved to another epoch names it too (`mgs-48213-past250ma-42`, see
/// [`crate::export::drift_label`]), so that the epochs of one planet never
/// collide. A lone planet's system names it too (`mgs-48213-lone-42`), so
/// that it never collides with the full system of the same star and seed.
/// The seed stays last.
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
    let lone = if system.request.is_alone() {
        "-lone"
    } else {
        ""
    };
    match drift {
        Some(epoch) => format!("{slug}-{epoch}{lone}-{}", system.seed),
        None => format!("{slug}{lone}-{}", system.seed),
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

/// Reads `system.json` from `dir`, deriving the star's activity when the
/// file predates it. A system of another [`SYSTEM_FORMAT_VERSION`] is
/// refused: its bodies are not laid out as this version reads them.
pub fn read_system(dir: &std::path::Path) -> Result<System, String> {
    let bytes = std::fs::read(dir.join("system.json")).map_err(|e| e.to_string())?;
    // The version first: an older system may not even parse as today's.
    #[derive(Deserialize)]
    struct Head {
        format_version: u32,
    }
    let head: Head = serde_json::from_slice(&bytes).map_err(|e| format!("system.json: {e}"))?;
    if head.format_version != SYSTEM_FORMAT_VERSION {
        return Err(format!(
            "system.json: format version {} is not the supported version \
             {SYSTEM_FORMAT_VERSION}: the generator does not migrate older systems, delete it \
             (`mise run clean-systems`) and draw it again",
            head.format_version
        ));
    }
    let mut system: System = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    // A system drawn before the star's activity existed gets it derived,
    // the same as a draw would store.
    if system.star.activity.is_none() {
        system.star.activity = Some(star_activity(&system.star.info, system.seed));
    }
    Ok(system)
}

/// Draws the system a request asks for.
pub fn draw(request: &SystemRequest, rng: &mut impl RngExt) -> Result<System, RequestError> {
    match request {
        SystemRequest::System(whole) => draw_whole(request, whole, rng),
        SystemRequest::Around(world) => {
            let resolved = world.resolve(rng)?;
            Ok(draw_around(request, resolved.params, resolved.model))
        }
    }
}

/**
 * The system around a planet already resolved from `world` (its `params`
 * and `model`, as `world.resolve` gave them): the same system [`draw`]
 * gives for `SystemRequest::Around(world)` with those parameters, for a
 * caller that shows them first (the CLI).
 */
pub fn draw_around_params(world: &WorldRequest, params: WorldParams, model: PlanetModel) -> System {
    draw_around(&SystemRequest::Around(world.clone()), params, model)
}

/// How far past the outer orbit of its archetype's range a requested
/// planet may be pushed by the spacing, as a factor. Calibration, no
/// source: Jupiter-mass planets at the Hill spacing ([`hill_spacing`],
/// 6.4 mutual Hill radii) sit 1.8 times as far as the last, so three of
/// them reach 3.1 times the first one's orbit; the Earth-mass ones, 12 %
/// apart, pack far more of them than a system holds.
const REQUESTED_REACH: f64 = 4.0;

/// How far out a drawn hot Jupiter may be spaced, AU. Calibration, no
/// source: its orbit is drawn within 0.1 AU ([`drawn_hot_jupiter`]).
const DRAWN_HOT_JUPITER_MAX_AU: f64 = 0.1;

/// A planet asked for by archetype: the archetype, the class it derives
/// (typed before its model exists, from a trial draw of its inputs at its
/// orbit) and the band of distances spacing may place it in, AU.
#[derive(Clone, Debug, PartialEq)]
struct Asked {
    archetype: Arc<Archetype>,
    class: PlanetClass,
    band: (f64, f64),
}

/// How a slot came to be: drawn from where it formed, with no type,
/// requested by archetype, a giant fixed by a request around one world
/// (its host and its giant neighbours), or the planet of such a request,
/// of the class its model derived.
#[derive(Clone, Debug, PartialEq)]
enum Placement {
    Drawn(Origin),
    Requested(Asked),
    Giant(PlanetClass),
    Focus(PlanetClass),
}

impl Placement {
    /// Whether the slot holds a giant.
    fn is_giant(&self) -> bool {
        match self {
            Placement::Drawn(origin) => origin.is_giant(),
            Placement::Requested(asked) => asked.class.is_giant(),
            Placement::Giant(class) | Placement::Focus(class) => class.is_giant(),
        }
    }

    /// Whether the slot holds a hot Jupiter.
    fn is_hot_jupiter(&self) -> bool {
        match self {
            Placement::Drawn(origin) => *origin == Origin::HotJupiter,
            Placement::Requested(asked) => asked.class == PlanetClass::HotJupiter,
            Placement::Giant(class) | Placement::Focus(class) => *class == PlanetClass::HotJupiter,
        }
    }

    /// The giant class the slot is spaced as, before its model exists:
    /// what [`crate::belts::cold_giant`] reads while the planets are
    /// placed. `None` for a small planet.
    fn giant_class(&self) -> Option<PlanetClass> {
        match self {
            Placement::Drawn(Origin::GasGiant) => Some(PlanetClass::GasGiant),
            Placement::Drawn(Origin::IceGiant) => Some(PlanetClass::IceGiant),
            Placement::Drawn(Origin::HotJupiter) => Some(PlanetClass::HotJupiter),
            Placement::Drawn(Origin::Small) => None,
            Placement::Requested(Asked { class, .. })
            | Placement::Giant(class)
            | Placement::Focus(class) => class.sculpts_belts().then_some(*class),
        }
    }
}

/// A planet being placed: where it came from, mass (Earth masses), orbit
/// and eccentricity, and what it stays as.
#[derive(Clone, Debug)]
struct Slot {
    placement: Placement,
    mass_earth: f64,
    orbit_au: f64,
    /// The orbit it was drawn at, AU, before spacing moved it.
    drawn_au: f64,
    eccentricity: f64,
    /// Kept where it is (the planet of a request around one).
    fixed: bool,
}

/// How far a drawn small planet may move from the orbit it was drawn at,
/// as a factor: it stays within `[a / SMALL_DRIFT, a × SMALL_DRIFT]`.
/// Calibration, no source: keeps a planet near where the formation
/// context put it, so that its model (derived at its orbit) describes the
/// planet that was drawn.
const SMALL_DRIFT: f64 = 1.5;

impl Slot {
    /// The farthest the slot may be pushed out, AU: a requested
    /// archetype's band ([`Asked::band`]); a drawn hot Jupiter's
    /// ([`DRAWN_HOT_JUPITER_MAX_AU`]); a drawn small planet
    /// [`SMALL_DRIFT`] times its drawn orbit (spacing only moves a slot
    /// out, from within a tenth of that orbit, so it never comes nearer
    /// than `a / SMALL_DRIFT`); a drawn giant the system's outer edge.
    fn max_au(&self) -> f64 {
        match &self.placement {
            Placement::Requested(asked) => asked.band.1,
            Placement::Drawn(Origin::HotJupiter) => DRAWN_HOT_JUPITER_MAX_AU,
            Placement::Drawn(Origin::Small) => (SMALL_DRIFT * self.drawn_au).min(OUTER_EDGE_AU),
            Placement::Drawn(_) => OUTER_EDGE_AU,
            // Kept where its request put it.
            Placement::Giant(_) | Placement::Focus(_) => self.orbit_au,
        }
    }

    /// What names the slot when spacing fails: its placement and the orbit
    /// it was drawn at.
    fn failed(&self) -> Failed {
        (self.placement.clone(), self.drawn_au)
    }
}

/// A slot spacing could not place: its placement and drawn orbit, AU.
type Failed = (Placement, f64);

/// Whether two orbits round a star of `star_mass` (M☉) are clear of each
/// other: [`hill_spacing`] mutual Hill radii, periapsis to apoapsis, and
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
    let hot = inner.placement.is_hot_jupiter() || outer.placement.is_hot_jupiter();
    let share = 0.5 * (inner.mass_earth + outer.mass_earth) / (star_mass * sun);
    gap >= hill_spacing(share) * hill
        && (!hot || outer.orbit_au / inner.orbit_au >= 4f64.powf(2.0 / 3.0))
}

/// A hot Jupiter for a drawn system, or none: as often as round a world
/// ([`HOT_JUPITERS`], about 1 % of Sun-like stars, Wright et al. 2012;
/// times `10^(2 [Fe/H])` and the star's mass, like the other gas giants),
/// on an orbit of 1 to 10 days, log-uniform, within 0.1 AU. The spacing
/// then keeps other planets beyond four times its period.
fn drawn_hot_jupiter(star: &StarInfo, metallicity: f64, rng: &mut impl RngExt) -> Option<f64> {
    let mass = f64::from(star.mass);
    let odds =
        (HOT_JUPITERS * 10f64.powf(2.0 * metallicity).min(10.0) * mass.clamp(0.1, 2.5)).min(0.2);
    let (hot, u) = (rng.random::<f64>() < odds, rng.random::<f64>());
    hot.then(|| {
        let (lo, hi) = HOT_PERIOD_DAYS;
        let days = (lo.ln() + u * (hi / lo).ln()).exp();
        (mass * (days / 365.25).powi(2))
            .cbrt()
            .clamp(INNER_EDGE_AU, 0.1)
    })
}

/// The slots drawn when a request leaves the planets to the draw, each an
/// [`Origin`] and an orbit in AU, none a type: small planets from the
/// Kepler counts (about 1.5 within 1 AU of a Sun-like star, 2.5 of an M
/// dwarf; Mulders et al. 2018) on periods log-uniform from 1 to 400 days,
/// a few colder ones beyond, the giants as [`GiantSystem`] draws them, and
/// last a hot Jupiter ([`drawn_hot_jupiter`]).
fn drawn_slots(star: &StarInfo, metallicity: f64, rng: &mut impl RngExt) -> Vec<(Origin, f64)> {
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
    let hz_outer = f64::from(star.hz_outer_au);
    let ice = 2.7 * f64::from(star.luminosity).sqrt();
    let mut out = Vec::new();
    for _ in 0..poisson(inner_mean, rng) {
        let days = (1f64.ln() + rng.random::<f64>() * 400f64.ln()).exp();
        let a = (mass * (days / 365.25).powi(2)).cbrt().max(INNER_EDGE_AU);
        out.push((Origin::Small, a));
    }
    // Colder small planets, between the habitable zone and the outer giants.
    for _ in 0..poisson(outer_mean, rng) {
        out.push((Origin::Small, hz_outer * (1.0 + rng.random::<f64>() * 8.0)));
    }
    for _ in 0..poisson(gas_mean, rng) {
        out.push((Origin::GasGiant, ice * 10f64.powf(0.3 + 0.4 * normal(rng))));
    }
    for _ in 0..poisson(ice_mean, rng) {
        out.push((Origin::IceGiant, ice * 10f64.powf(0.7 + 0.3 * normal(rng))));
    }
    // Drawn last, so that the other planets' draws stay as they were.
    out.extend(drawn_hot_jupiter(star, metallicity, rng).map(|a| (Origin::HotJupiter, a)));
    // A slot whose orbit falls outside the system is dropped, after every
    // draw, as `place` dropped it.
    out.retain(|(_, a)| (INNER_EDGE_AU..=OUTER_EDGE_AU).contains(a));
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
/// rounds at most. The error names the first slot pushed out of its band
/// ([`Slot::failed`]).
fn space(slots: &mut Vec<Slot>, star_mass: f64, ice: f64) -> Result<(), Failed> {
    slots.sort_by(|a, b| a.orbit_au.total_cmp(&b.orbit_au));
    let zone_of = |slots: &[Slot]| {
        slots
            .iter()
            .filter(|s| {
                s.placement
                    .giant_class()
                    .is_some_and(|class| crate::belts::cold_giant(class, s.orbit_au, ice))
            })
            .map(|s| s.orbit_au)
            .reduce(f64::min)
            .map(crate::belts::belt_zone)
    };
    let mut zone = zone_of(slots);
    let mut placed = Vec::new();
    for _ in 0..4 {
        placed = place(slots, star_mass, zone)?;
        let settled = zone_of(&placed);
        if settled == zone || placed.iter().all(|s| !in_zone(s, settled)) {
            *slots = placed;
            return Ok(());
        }
        zone = settled;
    }
    match placed.iter().find(|s| in_zone(s, zone_of(&placed))) {
        Some(s) => Err(s.failed()),
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
            && !slot.placement.is_giant()
            && slot.orbit_au * (1.0 + slot.eccentricity) > lo
            && slot.orbit_au * (1.0 - slot.eccentricity) < hi
    })
}

/// One round of [`space`], the rocky planets kept out of `zone`.
fn place(slots: &[Slot], star_mass: f64, zone: Option<(f64, f64)>) -> Result<Vec<Slot>, Failed> {
    let mut placed: Vec<Slot> = slots.iter().filter(|s| s.fixed).cloned().collect();
    let mut free: Vec<Slot> = slots.iter().filter(|s| !s.fixed).cloned().collect();
    free.sort_by(|a, b| a.orbit_au.total_cmp(&b.orbit_au));
    for mut slot in free {
        let max = slot.max_au();
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
                return Err(slot.failed());
            }
        }
        placed.push(slot);
    }
    placed.sort_by(|a, b| a.orbit_au.total_cmp(&b.orbit_au));
    Ok(placed)
}

/// How many planets of an archetype fit in its band, packed from its
/// inner edge at the spacing, each of a typical mass.
fn fits(asked: &Asked, star_mass: f64) -> u32 {
    let (min, max) = asked.band;
    let mut rng = StdRng::seed_from_u64(0);
    let mass = asked.archetype.mass(&mut rng);
    let mut last = Slot {
        placement: Placement::Requested(asked.clone()),
        mass_earth: mass,
        orbit_au: min.max(INNER_EDGE_AU),
        drawn_au: min.max(INNER_EDGE_AU),
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

/// The class a requested archetype derives, for the spacing that runs
/// before its model exists: a trial model from the archetype's mass and
/// inputs drawn on a stream of their own, at the middle of its orbits.
fn trial_class(
    archetype: &Archetype,
    seed: u32,
    star_code: f32,
    age_gyr: f32,
    metallicity: f32,
    ice: f64,
    orbit_au: f64,
) -> PlanetClass {
    let mut rng = StdRng::seed_from_u64(u64::from(body_seed(
        seed,
        &format!("trial/{}", archetype.name),
    )));
    let mass = archetype.mass(&mut rng);
    let overrides = archetype.overrides(&mut rng);
    let at = StarInfo::new(star_code, orbit_au as f32, 0.0, age_gyr);
    derive_model_with(
        seed,
        "trial",
        &Context::of(&at, metallicity, ice as f32),
        mass,
        &overrides,
    )
    .class
}

/// The built-in archetype a whole system names a planet by: a file on the
/// server's disk is not a name to ask a system for.
fn system_archetype(name: &str) -> Result<Archetype, RequestError> {
    if !crate::archetype::built_in_names().contains(&name) {
        return Err(RequestError::UnknownArchetype(name.to_string()));
    }
    crate::request::load_archetype(name)
}

/// A whole system.
fn draw_whole(
    request: &SystemRequest,
    whole: &WholeSystemRequest,
    rng: &mut impl RngExt,
) -> Result<System, RequestError> {
    let p = place_whole(whole, rng)?;
    Ok(assemble(
        request,
        p.seed,
        p.star_code,
        p.age_gyr,
        p.metallicity,
        p.width,
        p.height,
        p.slots,
        None,
    ))
}

/// A whole system's star, size and planets as placed, before
/// [`assemble`] makes them bodies.
struct Placed {
    seed: u32,
    star_code: f32,
    age_gyr: f32,
    metallicity: f32,
    width: u32,
    height: u32,
    slots: Vec<Slot>,
}

/// The planets of a whole system, placed.
fn place_whole(whole: &WholeSystemRequest, rng: &mut impl RngExt) -> Result<Placed, RequestError> {
    let seed = whole.seed.unwrap_or_else(|| rng.random());
    let mut rng = StdRng::seed_from_u64(u64::from(stream_seed(seed, stream::SYSTEM)));
    let star_code = match &whole.star {
        Some(text) => parse_type(text).map_err(RequestError::BadStar)?,
        None => crate::star::draw_type_living(whole.age_gyr, &mut rng),
    };
    let metallicity = whole
        .metallicity
        .unwrap_or_else(|| crate::giants::draw_metallicity(seed));
    let (width, height) = (
        whole.width.unwrap_or(DEFAULT_SIZE.0),
        whole.height.unwrap_or(DEFAULT_SIZE.1),
    );
    // From its own stream, after the star is known: no other draw shifts.
    let age_gyr = crate::star::resolve_age_gyr(whole.age_gyr, star_code, seed)?;
    let star = StarInfo::new(star_code, 1.0, 0.0, age_gyr);
    let ice = 2.7 * f64::from(star.luminosity).sqrt();
    let star_mass = f64::from(star.mass);

    // The preferred band of each planet asked, to draw its distance again
    // when a try fails.
    let mut preferred: Vec<(Placement, f64, f64)> = Vec::new();
    let asked: Vec<(Placement, f64)> = match &whole.planets {
        Some(planets) => {
            let total: u32 = planets.values().sum();
            if total as usize > MAX_PLANETS {
                return Err(RequestError::TooManyPlanets {
                    asked: total,
                    max: MAX_PLANETS as u32,
                });
            }
            let mut list = Vec::new();
            for (name, &count) in planets {
                let archetype = system_archetype(name)?;
                if count == 0 {
                    continue;
                }
                // Its orbits: those its insolation (or period) range gives
                // round this star; the star the archetype names is not
                // read, the system's star rules.
                let impossible = || RequestError::TypeImpossible {
                    body_type: name.clone(),
                };
                let (near, far) = archetype
                    .orbit
                    .orbit_range_au(f64::from(star.luminosity), star_mass)
                    .ok_or_else(impossible)?;
                if far < INNER_EDGE_AU || near > OUTER_EDGE_AU {
                    return Err(impossible());
                }
                let lo = near.max(INNER_EDGE_AU);
                let hi = far.min(OUTER_EDGE_AU).max(lo);
                let class = trial_class(
                    &archetype,
                    seed,
                    star_code,
                    age_gyr,
                    metallicity,
                    ice,
                    (lo * hi).sqrt(),
                );
                let asked = Asked {
                    archetype: Arc::new(archetype),
                    class,
                    band: (lo, (hi * REQUESTED_REACH).min(OUTER_EDGE_AU)),
                };
                let room = fits(&asked, star_mass);
                if count > room {
                    return Err(RequestError::ZoneFull {
                        body_type: name.clone(),
                        asked: count,
                        fits: room,
                    });
                }
                let placement = Placement::Requested(asked);
                for _ in 0..count {
                    list.push((placement.clone(), log_between(lo, hi, &mut rng)));
                    preferred.push((placement.clone(), lo, hi));
                }
            }
            list
        }
        None => drawn_slots(&star, f64::from(metallicity), &mut rng)
            .into_iter()
            .map(|(origin, a)| (Placement::Drawn(origin), a))
            .collect(),
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
                .map(|(placement, lo, hi)| (placement.clone(), log_between(*lo, *hi, &mut rng)))
                .collect();
        }
        let mut slots: Vec<Slot> = asked
            .iter()
            .map(|(placement, a)| {
                let mass_earth = slot_mass(placement, &mut rng);
                // A planet asked for by archetype keeps the orbit its
                // range gave (the draw is made all the same); the others
                // are spread a tenth either side.
                let jitter = 1.0 + 0.2 * (rng.random::<f64>() - 0.5);
                let orbit_au = match placement {
                    Placement::Requested(_) => *a,
                    _ => *a * jitter,
                };
                let drawn = if placement.is_hot_jupiter() {
                    0.0
                } else if placement.is_giant() {
                    // Packed systems are calmer (He et al. 2020).
                    f64::from(draw_eccentricity(&mut rng)) * if packed { 0.4 } else { 1.0 }
                } else {
                    // A small planet with companions (Van Eylen et al.
                    // 2019), calmer than a giant.
                    f64::from(crate::star::draw_small_eccentricity(true, &mut rng))
                };
                // An archetype that sets the eccentricity of its orbit
                // replaces the draw (made all the same, so that the
                // others' draws stay put).
                let eccentricity = match &placement {
                    Placement::Requested(asked) => asked
                        .archetype
                        .orbit
                        .eccentricity
                        .map_or(drawn, |e| f64::from(e.pick(&mut rng))),
                    _ => drawn,
                };
                Slot {
                    mass_earth,
                    orbit_au,
                    drawn_au: *a,
                    eccentricity,
                    fixed: false,
                    placement: placement.clone(),
                }
            })
            .collect();
        match space(&mut slots, star_mass, ice) {
            Ok(()) => {
                // A drawn system keeps the planets that fit: those pushed
                // out are dropped rather than failing.
                return Ok(Placed {
                    seed,
                    star_code,
                    age_gyr,
                    metallicity,
                    width,
                    height,
                    slots,
                });
            }
            Err(placement) => last_error = Some(placement),
        }
        if whole.planets.is_none() {
            break;
        }
    }
    if whole.planets.is_none() {
        // A drawn system that does not fit is drawn again without the
        // planet that failed (the one drawn at its orbit, else the last
        // of its origin), until it fits.
        let mut kept = asked;
        while let Some(i) = last_error.as_ref().and_then(|(failed, drawn)| {
            kept.iter()
                .position(|(p, a)| p == failed && a == drawn)
                .or_else(|| kept.iter().rposition(|(p, _)| p == failed))
        }) {
            kept.remove(i);
            let mut slots: Vec<Slot> = kept
                .iter()
                .map(|(p, a)| {
                    let mass_earth = slot_mass(p, &mut rng);
                    // Drawn for every planet, so the others' draws stay put;
                    // a hot Jupiter's orbit is circular.
                    let e = f64::from(draw_eccentricity(&mut rng)) * 0.4;
                    Slot {
                        placement: p.clone(),
                        mass_earth,
                        orbit_au: *a,
                        drawn_au: *a,
                        eccentricity: if p.is_hot_jupiter() { 0.0 } else { e },
                        fixed: false,
                    }
                })
                .collect();
            match space(&mut slots, star_mass, ice) {
                Ok(()) => {
                    return Ok(Placed {
                        seed,
                        star_code,
                        age_gyr,
                        metallicity,
                        width,
                        height,
                        slots,
                    });
                }
                // The next to go is the last of the origin that failed now.
                Err(placement) => last_error = Some(placement),
            }
        }
        return Ok(Placed {
            seed,
            star_code,
            age_gyr,
            metallicity,
            width,
            height,
            slots: Vec::new(),
        });
    }
    let (body_type, fits) = match last_error {
        Some((Placement::Requested(asked), _)) => {
            (asked.archetype.name.clone(), fits(&asked, star_mass))
        }
        _ => (String::new(), 0),
    };
    let asked_of = whole
        .planets
        .as_ref()
        .and_then(|p| p.get(&body_type))
        .copied()
        .unwrap_or(0);
    Err(RequestError::ZoneFull {
        fits: fits.min(asked_of.saturating_sub(1)),
        body_type,
        asked: asked_of,
    })
}

/// The mass of a slot, Earth masses: from its origin when drawn, from its
/// archetype's range when requested.
fn slot_mass(placement: &Placement, rng: &mut impl RngExt) -> f64 {
    match placement {
        Placement::Drawn(origin) => draw_mass(*origin, rng),
        Placement::Requested(asked) => asked.archetype.mass(rng),
        Placement::Giant(_) | Placement::Focus(_) => {
            unreachable!("a planet a request around one fixed has its mass")
        }
    }
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
    /// Its model, from the resolution of its request.
    model: PlanetModel,
    /// The giant it orbits, when it is a giant's moon.
    host: Option<HostGiant>,
}

/// A system around one planet, of `params` and `model`.
fn draw_around(request: &SystemRequest, params: WorldParams, model: PlanetModel) -> System {
    let seed = params.seed;
    let mut rng = StdRng::seed_from_u64(u64::from(stream_seed(seed, stream::SYSTEM)));
    let star = StarInfo::new(params.star, 1.0, 0.0, params.age_gyr);
    let ice = 2.7 * f64::from(star.luminosity).sqrt();
    let star_mass = f64::from(star.mass);
    let host = HostGiant::draw(&params);
    // A lone planet: no giant and no rocky neighbour.
    let alone = request.is_alone();
    // The planet asked, of its model's class and mass, or the giant it
    // orbits, on its orbit.
    let (placement, mass_earth) = match &host {
        Some(h) if h.giant.kind == GiantKind::Ice => (
            Placement::Giant(PlanetClass::IceGiant),
            f64::from(h.giant.mass_earth),
        ),
        Some(h) => (
            Placement::Giant(PlanetClass::GasGiant),
            f64::from(h.giant.mass_earth),
        ),
        None => (Placement::Focus(model.class), model.inputs.mass_earth),
    };
    let focus_slot = Slot {
        placement,
        mass_earth,
        orbit_au: f64::from(params.orbit_au),
        drawn_au: f64::from(params.orbit_au),
        eccentricity: f64::from(params.eccentricity),
        fixed: true,
    };
    let mut slots = vec![focus_slot];
    // Its giants, as the world draws them today.
    if !alone
        && params.giants != GiantsMode::None
        && let Some(giants) = GiantSystem::draw(&params)
    {
        for g in giants.giants {
            let giant_class = if g.hot {
                PlanetClass::HotJupiter
            } else if g.kind == GiantKind::Ice {
                PlanetClass::IceGiant
            } else {
                PlanetClass::GasGiant
            };
            slots.push(Slot {
                placement: Placement::Giant(giant_class),
                mass_earth: f64::from(g.mass_earth),
                orbit_au: f64::from(g.orbit_au),
                drawn_au: f64::from(g.orbit_au),
                eccentricity: f64::from(g.eccentricity),
                fixed: true,
            });
        }
    }
    // The rocky planets round them, those that do not fit dropped.
    let mut rocky = if alone {
        Vec::new()
    } else {
        drawn_slots(&star, f64::from(params.metallicity), &mut rng)
    };
    rocky.retain(|(origin, _)| *origin == Origin::Small);
    for (origin, a) in rocky {
        let placement = Placement::Drawn(origin);
        let mut slot = Slot {
            mass_earth: draw_mass(origin, &mut rng),
            placement,
            orbit_au: a,
            drawn_au: a,
            eccentricity: f64::from(draw_eccentricity(&mut rng)) * 0.4,
            fixed: false,
        };
        let mut trial = slots.clone();
        trial.push(slot.clone());
        if space(&mut trial, star_mass, ice).is_ok() {
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
        params.age_gyr,
        params.metallicity,
        width,
        height,
        slots,
        Some(Focus {
            params,
            model,
            host,
        }),
    )
}

/// The body seed of `id` in the system of `seed`.
pub(crate) fn body_seed(seed: u32, id: &str) -> u32 {
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
    age_gyr: f32,
    metallicity: f32,
    width: u32,
    height: u32,
    slots: Vec<Slot>,
    focus: Option<Focus>,
) -> System {
    let info = StarInfo::new(star_code, 1.0, 0.0, age_gyr);
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
                slot.eccentricity = tidal_eccentricity(&slot, star_code, age_gyr);
            }
            slot
        })
        .collect();
    let mut bodies = Vec::new();
    let mut focus_id = None;
    let mut models = Vec::with_capacity(slots.len());
    for (rank, slot) in slots.iter().enumerate() {
        let id = planet_letter(rank);
        let planet_designation = format!("{designation} {id}");
        let at_focus = slot.fixed
            && focus
                .as_ref()
                .is_some_and(|f| (f64::from(f.params.orbit_au) - slot.orbit_au).abs() < 1e-9);
        let planet_seed = body_seed(seed, &id);
        // The world a request around one asked for has the model its
        // resolution derived; the giant a world asked for orbits, as every
        // other planet, its own.
        let asked = at_focus && focus.as_ref().is_some_and(|f| f.host.is_none());
        // Its physical model, once its id is fixed: its own star seen from
        // its orbit, the system's age, metallicity and ice line, and the
        // mass it was spaced with.
        let mut tilt = None;
        let model = match &focus {
            Some(f) if asked => f.model.clone(),
            _ => {
                let at = StarInfo::new(
                    star_code,
                    slot.orbit_au as f32,
                    slot.eccentricity as f32,
                    age_gyr,
                );
                let context = Context::of(&at, metallicity, ice);
                match &slot.placement {
                    // A planet asked for by archetype: its inputs and its
                    // tilt drawn in the archetype's ranges, on a stream of
                    // the body's own.
                    Placement::Requested(asked) => {
                        let mut stream = StdRng::seed_from_u64(u64::from(body_seed(
                            seed,
                            &format!("{id}/archetype"),
                        )));
                        let overrides = asked.archetype.overrides(&mut stream);
                        tilt = asked
                            .archetype
                            .orbit
                            .axial_tilt
                            .map(|t| t.pick(&mut stream));
                        derive_model_with(seed, &id, &context, slot.mass_earth, &overrides)
                    }
                    _ => derive_model(seed, &id, &context, slot.mass_earth),
                }
            }
        };
        // The preset its world renders with: the giant a world of a
        // request orbits as placed, the closest to its model otherwise.
        let look = match &slot.placement {
            Placement::Giant(class) if at_focus => class.key().to_string(),
            _ => base_of(&model).to_string(),
        };
        let params = match (&focus, at_focus) {
            (Some(f), true) if f.host.is_none() => f.params.clone(),
            (Some(f), true) => {
                let host = f.host.as_ref().expect("host");
                crate::exomoon::host_params(&f.params, host.giant.mass_earth, host.seed)
            }
            _ => planet_params(
                &model,
                &crate::request::Placed {
                    seed: planet_seed,
                    star: star_code,
                    age_gyr,
                    orbit_au: slot.orbit_au as f32,
                    eccentricity: slot.eccentricity as f32,
                    metallicity,
                    axial_tilt_deg: tilt,
                    perihelion_longitude_deg: None,
                    drift_ma: 0.0,
                    width,
                    height,
                    moons: MoonsMode::Auto,
                    giants: GiantsMode::None,
                    host: HostMode::None,
                },
            ),
        };
        // A giant is a planet whose world is a giant's: a giant class, or
        // the giant a request around one asked for.
        let giant = params.giant_mass_earth.is_some();
        let mut params = if giant {
            with_system_rings(params)
        } else {
            params
        };
        // Its moons.
        let moons = if giant {
            match (&focus, at_focus) {
                // The giant a world of the request orbits: the world among
                // its moons.
                (
                    Some(Focus {
                        params: world,
                        host: Some(host),
                        ..
                    }),
                    true,
                ) => host_chain(host, world, star_code),
                // The mass the moons were drawn with.
                _ => drawn_moons(
                    &params,
                    params.giant_mass_earth.map_or(slot.mass_earth, f64::from),
                ),
            }
        } else {
            drawn_moons(&params, f64::from(params.mass_earth()))
        };
        // The moons its rings shape, inside the others (the giant asked
        // for, and the giant a world of the request orbits, among them).
        let ring_moons = if giant {
            shape_rings(&mut params, slot, star_code, &moons)
        } else {
            Vec::new()
        };
        let planet = planet_body(
            &id,
            &planet_designation,
            slot,
            &look,
            Some(model.clone()),
            &params,
            star_code,
        );
        let planet_index = bodies.len();
        if at_focus && focus.as_ref().is_some_and(|f| f.host.is_none()) {
            focus_id = Some(id.clone());
        }
        bodies.push(planet);
        // Nearest first, the ring moons among the others.
        let giant_kg = model.inputs.mass_earth * EARTH_KG;
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
            let mut moon_params = moon_params;
            // A moon's seasons follow its planet's orbit around the star.
            moon_params.perihelion_longitude_deg =
                bodies[planet_index].params.perihelion_longitude_deg;
            let moon_id = format!("{id}-{}", k + 1);
            if let Some(gap) = ring.and_then(|r| r.gap) {
                gap_moons[gap] = moon_id.clone();
            }
            // The world of a request that is a giant's moon keeps the
            // model its resolution derived, at the giant's orbit.
            let mut moon_model = None;
            if at_focus
                && let Some(f) = focus
                    .as_ref()
                    .filter(|f| f.host.is_some() && f.params.seed == moon_params.seed)
            {
                focus_id = Some(moon_id.clone());
                moon_model = Some(f.model.clone());
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
                model: moon_model,
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
        // What the belts read of it: its class and mass.
        models.push((model.class, model.inputs.mass_earth));
    }
    // Its small bodies, from the planets as placed: belts and trojans, and
    // the crater rate they set on every body.
    let ids: Vec<String> = (0..slots.len()).map(planet_letter).collect();
    let tracks: Vec<crate::belts::Track> = slots
        .iter()
        .zip(&ids)
        .zip(&models)
        .map(|((slot, id), &(class, mass_earth))| crate::belts::Track {
            id,
            class,
            mass_earth,
            orbit_au: slot.orbit_au,
            eccentricity: slot.eccentricity,
        })
        .collect();
    // A lone planet has none of them: no belt, no swarm, and its crater
    // rate is the one its world was generated with.
    let alone = request.is_alone();
    let belts = if alone {
        Vec::new()
    } else {
        crate::belts::draw_belts(&info, f64::from(ice), &tracks, seed)
    };
    let trojans = if alone {
        vec![None; tracks.len()]
    } else {
        crate::belts::draw_trojans(f64::from(info.mass), &tracks, seed)
    };
    for body in bodies.iter_mut().filter(|_| !alone) {
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
            activity: Some(star_activity(&info, seed)),
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
fn tidal_eccentricity(slot: &Slot, star: f32, age_gyr: f32) -> f64 {
    let at = StarInfo::new(star, slot.orbit_au as f32, 0.0, age_gyr);
    // Before its model exists: a giant as it was placed.
    let circular = if slot.placement.is_giant() {
        let mass = slot.mass_earth as f32;
        crate::spin::giant_circularised(
            &at,
            mass,
            crate::giants::giant_radius(mass, at.equilibrium_k),
        )
    } else {
        let (mass, radius) =
            crate::spin::rocky_body(crate::world::rocky_circumference_km(slot.mass_earth));
        crate::spin::rocky_circularised(&at, mass, radius)
    };
    if circular { 0.0 } else { slot.eccentricity }
}

/// The world parameters of a placed planet from its `model`, placed as
/// `placed` says: [`params_for`], its own draws from its seed.
fn planet_params(model: &PlanetModel, placed: &crate::request::Placed) -> WorldParams {
    let mut rng = StdRng::seed_from_u64(u64::from(placed.seed));
    params_for(model, placed, &mut rng)
}

/// What a body gets from its star at `orbit_au`.
fn surroundings_at(star: &StarInfo, orbit_au: f64, eccentricity: f64) -> Surroundings {
    let at = StarInfo::new(
        star.code(),
        orbit_au as f32,
        eccentricity as f32,
        star.age_gyr,
    );
    Surroundings {
        flux_earth: f64::from(at.insolation),
        equilibrium_k: f64::from(at.equilibrium_k),
        hz_position: f64::from(at.hz_position),
    }
}

/// The body of a placed planet, typed by its `model`'s class and of its
/// mass, its preview in the colours of the preset `look` its world renders
/// with; the planet a request around one asked for, which has no model,
/// typed by that preset.
fn planet_body(
    id: &str,
    designation: &str,
    slot: &Slot,
    look: &str,
    model: Option<PlanetModel>,
    params: &WorldParams,
    star: f32,
) -> Body {
    let giant = params.giant_mass_earth.is_some();
    let model_mass = model.as_ref().map(|m| m.inputs.mass_earth);
    // A rocky planet's tides scale with its own mass and radius, its
    // resonance is drawn from its seed (see `crate::spin::star_info`).
    let info = if giant {
        StarInfo::new(
            star,
            slot.orbit_au as f32,
            slot.eccentricity as f32,
            params.age_gyr,
        )
    } else {
        crate::spin::star_info(params)
    };
    let mut rng = StdRng::seed_from_u64(u64::from(body_seed(params.seed, "orbit")));
    // A giant locks with its own tides (a hot one always); a rocky planet
    // may be locked or held in the 3:2 resonance (see `crate::spin`), its
    // sidereal day then two thirds of its year.
    let (radius_km, mass_earth, period_hours, locked, resonant) = if giant {
        let look =
            crate::giant_world::Look::draw(params, model_mass.unwrap_or(slot.mass_earth) as f32)
                .info;
        let locked = (look.hot
            || crate::spin::giant_locked(&info, look.mass_earth, look.radius_earth))
            && crate::spin::giant_stays_locked(slot.eccentricity as f32);
        (
            f64::from(look.radius_earth) * EARTH_M / 1000.0,
            model_mass.unwrap_or(f64::from(look.mass_earth)),
            f64::from(look.rotation_hours),
            locked,
            0.0,
        )
    } else {
        let radius = f64::from(params.circumference_km) / (2.0 * PI);
        let day = MoonSystem::draw(params).map_or(24.0, |m| f64::from(m.planet_day_hours));
        let resonant = info.spin_turns;
        let day = if resonant > 0.0 {
            f64::from(info.year_days) * 24.0 / f64::from(resonant)
        } else {
            day
        };
        // Its model's mass; the planet asked, the mass of the gravity its
        // world is generated with.
        let mass = model_mass.unwrap_or_else(|| f64::from(params.mass_earth()));
        (radius, mass, day, info.tidally_locked, resonant)
    };
    let volume = 4.0 / 3.0 * PI * (radius_km * 1000.0).powi(3);
    let density = mass_earth * EARTH_KG / volume / 1000.0;
    let gravity = if giant {
        mass_earth / (radius_km / 6371.0).powi(2)
    } else {
        f64::from(params.gravity_modifier())
    };
    let mut body = Body {
        id: id.to_string(),
        designation: designation.to_string(),
        kind: BodyKind::Planet,
        body_type: model
            .as_ref()
            .map_or_else(|| look.to_string(), |m| m.class.key().to_string()),
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
            spin_orbit_3_2: resonant == crate::spin::RESONANCE_TURNS_PER_ORBIT,
            resonance_turns: resonant,
        },
        surroundings: surroundings_at(&info, slot.orbit_au, slot.eccentricity),
        preview: preview_of(look, params, star),
        params: params.clone(),
        generated: false,
        trojans: None,
        ring_role: None,
        model,
    };
    // The sky the viewer draws puts the planet on its node line (the spring
    // equinox of its northern hemisphere) at an angle `periapsis_deg` before
    // its periapsis, so the longitude of perihelion the climate's seasons
    // read, from the equinox, is that argument of periapsis (see
    // `web/src/lib/systemSky.ts`, `placeSystem`, and the test below).
    body.params.perihelion_longitude_deg = Some(body.orbit.periapsis_deg.rem_euclid(360.0) as f32);
    body
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
            resonance_turns: 0.0,
        },
    )
}

/// The parameters a body's world is generated with: its stored ones, a
/// moon or a rocky planet stored before they carried their gravity (a
/// system made by an older version) taking its card's, from the mass and
/// radius the system stored, not the gravity the radius alone gives.
pub fn world_params(body: &Body) -> WorldParams {
    let mut params = body.params.clone();
    let rocky = body.kind == BodyKind::Moon || params.planet.giant.is_none();
    if rocky && params.gravity.is_none() {
        params.gravity = Some(body.physical.gravity as f32);
    }
    params
}

/// The parameters a body's world is generated with: [`world_params`],
/// and for a planet whose model gives it a surface (no giant) the sea
/// level its water fills and the warming toward its model's temperature
/// ([`crate::planet::interim::calibrate`]); for one that is no giant
/// whose envelope hides its surface, the warming toward its bare
/// temperature ([`crate::planet::interim::calibrate_enveloped`]). Those pass over its relief,
/// so the system's draw leaves them to here; listing and indexing a
/// system read [`world_params`], which stays cheap.
pub fn generation_params(body: &Body) -> WorldParams {
    let mut params = world_params(body);
    if let Some(model) = &body.model
        && !model.class.is_giant()
        && params.giant_mass_earth.is_none()
    {
        if model.surface.is_some() {
            crate::planet::interim::calibrate(&mut params, model);
        } else {
            crate::planet::interim::calibrate_enveloped(&mut params, model);
        }
    }
    params
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
/// params, and the moons that make them, which this returns. They stay
/// inside its other `moons`: Hill stable from the innermost, `a_2 > a_1 (1 +
/// 2·3^(1/6) μ^(1/3))` (Gladman 1993, doi:10.1006/icar.1993.1169, in the
/// form of Obertas et al. 2017, doi:10.1016/j.icarus.2017.04.010), the ring
/// moons' masses neglected; and within a third of its Hill radius, where
/// its regular moons keep. A giant too hot or with too small a Hill sphere
/// for regular moons (see [`drawn_moons`]) keeps its rings without
/// moons. The resonances in its rings take its `J2` from its flattening
/// and spin ([`crate::rings::j2`]).
fn shape_rings(
    params: &mut WorldParams,
    slot: &Slot,
    star: f32,
    moons: &[MoonParts],
) -> Vec<crate::rings::RingMoon> {
    let Some(rings) = params.given_rings.as_mut().and_then(|g| g.rings.take()) else {
        return Vec::new();
    };
    let info = StarInfo::new(
        star,
        slot.orbit_au as f32,
        slot.eccentricity as f32,
        params.age_gyr,
    );
    let mass_kg = slot.mass_earth * EARTH_KG;
    let radius_m = f64::from(params.circumference_km) * 1000.0 / (2.0 * PI);
    let density = mass_kg / (4.0 / 3.0 * PI * radius_m.powi(3)) / 1000.0;
    let hill = slot.orbit_au
        * (1.0 - slot.eccentricity)
        * 1.496e11
        * (mass_kg / (3.0 * f64::from(info.mass) * 1.989e30)).cbrt()
        / radius_m;
    let room = moons
        .iter()
        .map(|(_, _, orbit, physical, _)| {
            let mu = physical.mass_earth * EARTH_KG / mass_kg;
            orbit.semi_major_axis_km * 1000.0
                / radius_m
                / (1.0 + 2.0 * 3f64.powf(1.0 / 6.0) * mu.cbrt())
        })
        .fold(hill / 3.0, f64::min);
    let look = crate::giant_world::Look::draw(params, slot.mass_earth as f32).info;
    let omega = 2.0 * PI / (f64::from(look.rotation_hours) * 3600.0);
    let q = omega * omega * (f64::from(look.radius_earth) * EARTH_M).powi(3) / (G * mass_kg);
    // The globe is drawn flattened by `min(0.7 q, 0.12)` (`giant_world`):
    // past that cap J2 would fall as the giant spins faster, so it takes
    // the uncapped `0.7 q`, which keeps it rising with the spin.
    let flattening = f64::from(look.flattening).max(0.7 * q);
    let giant = crate::rings::Giant {
        density,
        mass_kg,
        j2: crate::rings::j2(flattening, q),
        room,
        moons: info.equilibrium_k <= 1000.0 && hill / 3.0 >= 6.0,
    };
    let (rings, ring_moons) = crate::rings::shape(rings, &giant, params.seed);
    if let Some(given) = params.given_rings.as_mut() {
        given.rings = rings;
    }
    ring_moons
}

/// The highest inclination a ring moon of `role` draws, degrees. Rings are
/// tens of metres to a kilometre thick: a moon in or at them stays within
/// a few km of their plane (Pan 0.0001°, Daphnis 0.0036°, Atlas 0.003°),
/// so that seen from it they lie at its horizon. Shepherds and resonant
/// moons, outside the rings, swing a little farther (Prometheus 0.008°,
/// Pandora 0.05°). The small moons that feed dust rings swing farther
/// still, and so thicken their rings (Amalthea 0.37°, Thebe 1.08°, NASA
/// Jovian satellite fact sheet; Burns et al. 1999,
/// doi:10.1126/science.284.5417.1146).
fn ring_moon_inclination_deg(role: crate::rings::RingRole) -> f64 {
    use crate::rings::RingRole;
    match role {
        RingRole::Embedded => 0.001,
        RingRole::Edge | RingRole::Confining => 0.005,
        RingRole::Shepherd | RingRole::Resonant => 0.05,
        RingRole::Source => 1.1,
    }
}

/// The eccentricity of a ring moon of `role`, from `frac` in `[0, 1)`. A
/// moon swings `a e` in and out of its circle: a moonlet must stay in its
/// gap, a few tens of km wide (Pan 1.4e-5, Daphnis 3.3e-5, about ±4 km,
/// which raises the waves on the Keeler gap's edges), so it draws up to
/// 5e-5; an edge moon up to 1.5e-3 (Atlas 1.2e-3); shepherds and resonant
/// moons, outside the rings, 2e-4 to 2.2e-3 (Prometheus 2.2e-3); the
/// small moons that feed dust rings 1e-3 to 1.8e-2 (Amalthea 3e-3, Thebe
/// 1.8e-2, NASA Jovian satellite fact sheet).
fn ring_moon_eccentricity(role: crate::rings::RingRole, frac: f64) -> f64 {
    use crate::rings::RingRole;
    match role {
        RingRole::Embedded => 5e-5 * frac,
        RingRole::Edge | RingRole::Confining => 1.5e-3 * frac,
        RingRole::Shepherd | RingRole::Resonant => 0.0002 + 0.002 * frac,
        RingRole::Source => 0.001 + 0.017 * frac,
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
    let gravity = G * mass / (moon.radius_km * 1e3).powi(2) / 9.81;
    let params = WorldParams {
        gravity: Some(gravity as f32),
        ridge: moon.ridge,
        ..crate::moons::small_body_params(
            giant,
            Composition::Ice,
            moon.radius_km as f32,
            moon.seed,
            shape,
        )
    };
    let seed = f64::from(moon.seed);
    // A moonlet in the rings of a giant larger than Saturn keeps the swing
    // it would have there, a few km in its gap (Daphnis: ±4 km at 136,505
    // km), its eccentricity scaled down with its distance.
    let swing = match moon.role {
        crate::rings::RingRole::Embedded => (136_505.0 / a_km).min(1.0),
        _ => 1.0,
    };
    (
        "icy-moon".to_string(),
        params,
        Orbit {
            semi_major_axis_km: a_km,
            semi_major_axis_au: a_km / 1.495_978_707e8,
            eccentricity: ring_moon_eccentricity(moon.role, swing * (seed % 7907.0) / 7907.0),
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
            gravity,
            axes_km: shape.map(|s| s.map(|a| a * moon.radius_km as f32)),
        },
        Spin {
            period_hours: period_days * 24.0,
            axial_tilt_deg: 0.0,
            locked: true,
            spin_orbit_3_2: false,
            resonance_turns: 0.0,
        },
    )
}

/// The moons of the giant a world of a request orbits: the world among
/// its regular moons, as [`HostGiant`] draws them, nearest first.
fn host_chain(host: &HostGiant, world: &WorldParams, star: f32) -> Vec<MoonParts> {
    let giant_kg = f64::from(host.giant.mass_earth) * EARTH_KG;
    let mut moons: Vec<(f64, MoonParts)> = Vec::new();
    let world_radius = f64::from(world.circumference_km) / (2.0 * PI);
    let world_mass = f64::from(world.mass_earth());
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
                gravity: f64::from(world.gravity_modifier()),
                axes_km: None,
            },
            Spin {
                period_hours: f64::from(host.period_days) * 24.0,
                axial_tilt_deg: f64::from(world.axial_tilt_deg),
                locked: true,
                spin_orbit_3_2: false,
                resonance_turns: 0.0,
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
                    resonance_turns: 0.0,
                },
            ),
        ));
    }
    let _ = (giant_kg, star);
    moons.sort_by(|a, b| a.0.total_cmp(&b.0));
    moons.into_iter().map(|(_, parts)| parts).collect()
}

/// A colour from sRGB to linear RGB.
fn srgb_to_linear(c: [f32; 3]) -> [f32; 3] {
    c.map(|v| {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    })
}

/// The preview of a body: colours by type, a giant's from its look.
fn preview_of(body_type: &str, params: &WorldParams, _star: f32) -> Preview {
    let rgb = |r: f32, g: f32, b: f32| [r, g, b];
    let rocky = |colours: Vec<[f32; 3]>, glow: bool| Preview {
        colours,
        bands: 0,
        edges: Vec::new(),
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
        "gas-giant" | "ice-giant" | "hot-jupiter" => {
            let mass = params.giant_mass_earth.unwrap_or(318.0);
            let look = crate::giant_world::Look::draw(params, mass);
            let p = look.info.palette;
            // The palette is sRGB, as the globe's surface is; a preview's
            // colours are linear, and the viewer converts them back.
            let edges = look.band_edges();
            Preview {
                colours: [p.zone, p.belt, p.polar].map(srgb_to_linear).to_vec(),
                bands: (edges.len() - 1) as u8,
                edges,
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
    let mut ids = family(system, body_id);
    ids.retain(|id| system.bodies.iter().any(|b| &b.id == id && !b.generated));
    ids
}

/// `body_id` first, then the bodies that go with it: a planet's moons; a
/// moon's planet and the planet's other moons, generated or not. Empty when
/// the id is unknown.
pub fn family(system: &System, body_id: &str) -> Vec<String> {
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
    ids
}

/// Stages a body's generation reports (the moons are bodies of their own).
const BODY_STAGES: usize = 8;

/// Generates `params` (a body's, at the size they say) into `dir`, which
/// it creates: the maps, `raw_data/` naming its system and body (`names`),
/// and, when `diagnostics` asks, `world.svg` and the diagnostic noise maps
/// (`noise_maps/`); the thumbnail into `thumb_dir` when given. Reports each stage to `at`.
fn generate_into(
    params: &WorldParams,
    dir: &std::path::Path,
    thumb_dir: Option<std::path::PathBuf>,
    diagnostics: Diagnostics,
    names: (&str, &str),
    at: impl Fn(crate::generation::Stage),
) -> Result<(), String> {
    use crate::generation::{GenerateOptions, Stage, generate_with_progress};
    // The noise maps are written as the generation computes their
    // layers, each one freed once encoded.
    let writer = if diagnostics.noise_maps {
        Some(
            crate::export::NoiseMapWriter::new(
                &dir.join("noise_maps"),
                params.width,
                params.height,
            )
            .map_err(|e| e.to_string())?,
        )
    } else {
        None
    };
    let options = GenerateOptions {
        debug: writer.as_ref().map(|w| w as _),
    };
    let generated = generate_with_progress(params, options, &at);
    if let Some(writer) = writer {
        writer.finish()?;
    }
    at(Stage::Export);
    crate::export::export_world(
        generated,
        dir,
        crate::export::ExportOptions {
            svg: diagnostics.svg,
            raw_data: true,
            thumb_dir,
        },
    )?;
    name_world(dir, names.0, names.1)
}

/// The progress callback of body `id`, `k`-th (from 0) of a job of `bodies`.
fn body_stage<'a>(
    report: &'a std::cell::RefCell<&'a mut dyn FnMut(BodyProgress)>,
    id: &'a str,
    k: usize,
    bodies: usize,
) -> impl Fn(crate::generation::Stage) + 'a {
    use crate::generation::Stage;
    move |stage: Stage| {
        let step = Stage::ALL.iter().position(|s| *s == stage).unwrap_or(0) as u32 + 1;
        (report.borrow_mut())(BodyProgress {
            body: id.to_string(),
            body_step: k as u32 + 1,
            bodies: bodies as u32,
            stage,
            step: step.min(BODY_STAGES as u32),
            steps: BODY_STAGES as u32,
        });
    }
}

/// Generates the bodies opening `body_id` asks for (see [`neighbourhood`])
/// in the system written in `system_dir`, one by one, each into
/// `bodies/<id>/` at its original resolution, laid out as
/// [`crate::sizes`] says (`thumb.png`, `sizes/<W>x<H>/`): written into
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
    generate_neighbourhood_with(system_dir, body_id, Diagnostics::default(), progress)
}

/// [`generate_neighbourhood`], with the extra files `diagnostics` asks for
/// in each body: `world.svg` and the noise maps
/// (`bodies/<id>/sizes/<W>x<H>/noise_maps/`, see
/// [`crate::export::NoiseMapWriter`]).
pub fn generate_neighbourhood_with(
    system_dir: &std::path::Path,
    body_id: &str,
    diagnostics: Diagnostics,
    mut progress: impl FnMut(BodyProgress),
) -> Result<Vec<String>, String> {
    let system = read_system(system_dir)?;
    if !system.bodies.iter().any(|b| b.id == body_id) {
        return Err(format!("no body {body_id} in this system"));
    }
    let ids = neighbourhood(&system, body_id);
    let system_name = system_id(&system);
    let bodies_dir = system_dir.join("bodies");
    std::fs::create_dir_all(&bodies_dir).map_err(|e| e.to_string())?;
    let report = std::cell::RefCell::new(&mut progress as &mut dyn FnMut(BodyProgress));
    for (k, id) in ids.iter().enumerate() {
        let body = system.bodies.iter().find(|b| &b.id == id).expect("listed");
        let params = generation_params(body);
        let temp = bodies_dir.join(format!(".tmp-{id}"));
        let _ = std::fs::remove_dir_all(&temp);
        let size = crate::sizes::size_dir(&temp, params.width, params.height);
        generate_into(
            &params,
            &size,
            Some(temp.clone()),
            diagnostics,
            (&system_name, id),
            body_stage(&report, id, k, ids.len()),
        )?;
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

/// Generates another resolution, `width` × `height`, of body `body_id` of
/// the system written in `system_dir`: the same world at another size,
/// into `bodies/<id>/sizes/<W>x<H>/` (see [`crate::sizes`]), written into
/// `sizes/.tmp-<W>x<H>/` and renamed into place once complete. With the
/// extra files `diagnostics` asks for. The body's thumbnail
/// and `system.json` are left as they are.
///
/// Refused when the body is unknown, not generated yet (its original
/// resolution comes from [`generate_neighbourhood`]), when that size is
/// on disk already, or outside [`crate::request::MAP_SIZE`].
pub fn generate_size(
    system_dir: &std::path::Path,
    body_id: &str,
    width: u32,
    height: u32,
    diagnostics: Diagnostics,
    mut progress: impl FnMut(BodyProgress),
) -> Result<(), String> {
    use crate::request::MAP_SIZE;
    for (field, value) in [("width", width), ("height", height)] {
        if !(MAP_SIZE.0..=MAP_SIZE.1).contains(&value) {
            return Err(format!(
                "{field} is {value}, it must be between {} and {}",
                MAP_SIZE.0, MAP_SIZE.1
            ));
        }
    }
    let system = read_system(system_dir)?;
    let body = system
        .bodies
        .iter()
        .find(|b| b.id == body_id)
        .ok_or_else(|| format!("no body {body_id} in this system"))?;
    if !body.generated {
        return Err(format!("body {body_id} is not generated"));
    }
    let body_dir = system_dir.join("bodies").join(body_id);
    let name = crate::sizes::size_name(width, height);
    let target = crate::sizes::size_dir(&body_dir, width, height);
    if target.exists() {
        return Err(format!("body {body_id} has its {name} size already"));
    }
    let mut params = generation_params(body);
    params.width = width;
    params.height = height;
    let temp = body_dir
        .join(crate::sizes::SIZES_DIR)
        .join(format!(".tmp-{name}"));
    let _ = std::fs::remove_dir_all(&temp);
    let report = std::cell::RefCell::new(&mut progress as &mut dyn FnMut(BodyProgress));
    let generated = generate_into(
        &params,
        &temp,
        None,
        diagnostics,
        (&system_id(&system), body_id),
        body_stage(&report, body_id, 0, 1),
    );
    if let Err(e) = generated {
        let _ = std::fs::remove_dir_all(&temp);
        return Err(e);
    }
    std::fs::rename(&temp, &target).map_err(|e| e.to_string())
}

/// The `world.svg` of body `body_id` at `width` × `height`, drawn on
/// request, as bytes. From the body's stored raw data when it reads back;
/// else the body is generated again from its seed and request (the same
/// world, up to the 16-bit quantisation of the stored layers). Nothing is
/// written. Fails when the system or the body is unknown.
pub fn body_svg(
    system_dir: &std::path::Path,
    body_id: &str,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, String> {
    let system = read_system(system_dir)?;
    let body = system
        .bodies
        .iter()
        .find(|b| b.id == body_id)
        .ok_or_else(|| format!("no body {body_id} in this system"))?;
    let size = crate::sizes::size_dir(&system_dir.join("bodies").join(body_id), width, height);
    let world = match crate::export::raw_data::load_world(&size) {
        Ok(world) => world,
        Err(_) => {
            let mut params = generation_params(body);
            params.width = width;
            params.height = height;
            crate::generation::generate(&params).world
        }
    };
    crate::export::svg_bytes(&world).map_err(|e| e.to_string())
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
    use crate::planet::PLANET_CLASSES;
    use crate::rings::RingRole;

    /// Whether `b` is a planet whose world is a giant's.
    pub(super) fn giant(b: &Body) -> bool {
        b.kind == BodyKind::Planet && b.params.giant_mass_earth.is_some()
    }

    /// The giant the world a request around one asked for orbits.
    pub(super) fn host_of(system: &System) -> &Body {
        let focus = system.focus.as_deref().expect("the world asked");
        let moon = system.bodies.iter().find(|b| b.id == focus).unwrap();
        let parent = moon.parent.as_deref().expect("a giant's moon");
        let host = system.bodies.iter().find(|b| b.id == parent).unwrap();
        assert!(giant(host), "{}", host.id);
        host
    }

    #[test]
    fn drawn_slots_name_no_type() {
        // Seeds 1..=200 round a G2V: every slot is an origin and an orbit
        // inside the system's edges; hot Jupiters only within 0.1 AU.
        for seed in 1..=200 {
            let mut rng = StdRng::seed_from_u64(seed);
            let star = StarInfo::new(parse_type("G2V").unwrap(), 1.0, 0.0, 4.6);
            for (origin, a) in drawn_slots(&star, 0.0, &mut rng) {
                assert!((INNER_EDGE_AU..=OUTER_EDGE_AU).contains(&a), "{a}");
                if origin == Origin::HotJupiter {
                    assert!(a <= 0.1, "{a}");
                }
            }
        }
    }

    #[test]
    fn the_solar_system_planets_are_spaced_far_enough() {
        // (kind, Earth masses, AU, eccentricity), NASA fact sheets.
        let slot = |kind: &str, mass_earth: f64, orbit_au: f64, eccentricity: f64| Slot {
            placement: match kind {
                "gas-giant" => Placement::Giant(PlanetClass::GasGiant),
                "ice-giant" => Placement::Giant(PlanetClass::IceGiant),
                _ => Placement::Drawn(Origin::Small),
            },
            mass_earth,
            orbit_au,
            drawn_au: orbit_au,
            eccentricity,
            fixed: false,
        };
        let planets = [
            slot("rocky", 0.055, 0.387, 0.206),
            slot("rocky", 0.815, 0.723, 0.007),
            slot("rocky", 1.0, 1.0, 0.017),
            slot("rocky", 0.107, 1.524, 0.094),
            slot("gas-giant", 317.8, 5.20, 0.049),
            slot("gas-giant", 95.2, 9.57, 0.057),
            slot("ice-giant", 14.5, 19.17, 0.046),
            slot("ice-giant", 17.1, 30.18, 0.010),
        ];
        for pair in planets.windows(2) {
            assert!(clear(&pair[0], &pair[1], 1.0), "{pair:?}");
        }
        // Two Earths at Jupiter and Saturn's 6.5 Hill radii are too close.
        assert!(!clear(
            &slot("rocky", 1.0, 5.20, 0.049),
            &slot("rocky", 1.0, 5.47, 0.057),
            1.0
        ));
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
                ("earth-like", 2),
                ("mercury-like", 1),
                ("jupiter-like", 1),
                ("neptune-like", 1),
            ],
            7,
        );
        // Placed as asked, each in its archetype's band.
        let SystemRequest::System(asked) = &request else {
            unreachable!()
        };
        let placed = place_whole(asked, &mut StdRng::seed_from_u64(1)).unwrap();
        let count = |name: &str| {
            placed
                .slots
                .iter()
                .filter(
                    |s| matches!(&s.placement, Placement::Requested(a) if a.archetype.name == name),
                )
                .count()
        };
        assert_eq!(
            (
                count("earth-like"),
                count("mercury-like"),
                count("jupiter-like"),
                count("neptune-like")
            ),
            (2, 1, 1, 1)
        );
        for slot in &placed.slots {
            let Placement::Requested(asked) = &slot.placement else {
                unreachable!()
            };
            assert!(
                slot.orbit_au >= asked.band.0 * 0.9 && slot.orbit_au <= asked.band.1,
                "{}: {} outside {:?}",
                asked.archetype.name,
                slot.orbit_au,
                asked.band
            );
        }
        let system = draw(&request, &mut StdRng::seed_from_u64(1)).unwrap();
        let p = planets(&system);
        assert_eq!(p.len(), 5);
        assert_eq!(system.focus, None);
        // Each typed by its class.
        for b in &p {
            assert!(
                PLANET_CLASSES.contains(&b.body_type.as_str()),
                "{}",
                b.body_type
            );
        }
        // From the star out, lettered in order, each clear of the last.
        for (k, pair) in p.windows(2).enumerate() {
            assert!(pair[0].orbit.semi_major_axis_au < pair[1].orbit.semi_major_axis_au);
            assert_eq!(pair[0].id, planet_letter(k));
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

    /// The insolation of `body` from its star, Earth's = 1.
    fn insolation(system: &System, body: &Body) -> f64 {
        f64::from(system.star.info.luminosity) / body.orbit.semi_major_axis_au.powi(2)
    }

    #[test]
    fn a_requested_archetype_sits_at_its_insolation() {
        // Round a G2V, Venus's range (1.7 to 2.1) holds, whatever the seed.
        for seed in 0..12 {
            let system = draw(
                &whole(&[("venus-like", 1)], seed),
                &mut StdRng::seed_from_u64(1),
            )
            .unwrap();
            let p = planets(&system);
            assert_eq!(p.len(), 1);
            let s = insolation(&system, p[0]);
            assert!((1.65..2.15).contains(&s), "seed {seed}: {s}");
        }
    }

    #[test]
    fn three_earths_share_the_habitable_zone() {
        for seed in 0..6 {
            let system = match draw(
                &whole(&[("earth-like", 3)], seed),
                &mut StdRng::seed_from_u64(1),
            ) {
                Ok(system) => system,
                // A clear error is as good as a system.
                Err(RequestError::ZoneFull { body_type, .. }) => {
                    assert_eq!(body_type, "earth-like");
                    continue;
                }
                Err(e) => panic!("seed {seed}: {e:?}"),
            };
            let p = planets(&system);
            assert_eq!(p.len(), 3, "seed {seed}");
            let star_mass = f64::from(system.star.info.mass);
            for pair in p.windows(2) {
                let slot = |b: &Body| Slot {
                    placement: Placement::Drawn(Origin::Small),
                    mass_earth: b.physical.mass_earth,
                    orbit_au: b.orbit.semi_major_axis_au,
                    drawn_au: b.orbit.semi_major_axis_au,
                    eccentricity: b.orbit.eccentricity,
                    fixed: false,
                };
                let (mut inner, outer) = (slot(pair[0]), slot(pair[1]));
                inner.orbit_au *= 0.999;
                assert!(clear(&inner, &outer, star_mass), "seed {seed}");
            }
            for b in p {
                let model = b.model.as_ref().unwrap();
                assert!(
                    (0.9..=1.1).contains(&model.inputs.mass_earth),
                    "seed {seed}"
                );
            }
        }
    }

    #[test]
    fn requested_archetypes_keep_their_inputs() {
        let system = draw(
            &whole(&[("earth-like", 1)], 4),
            &mut StdRng::seed_from_u64(1),
        )
        .unwrap();
        let model = planets(&system)[0].model.clone().unwrap();
        assert_eq!(model.inputs.core_mass_fraction, 0.325_f32 as f64);
        assert_eq!(model.inputs.life_draw, 0.0);
        assert_eq!(planets(&system)[0].params.axial_tilt_deg, 23.44);
    }

    #[test]
    fn an_old_type_name_is_refused() {
        for name in ["terran", "caustic", "frozen", "volcanic", "barren"] {
            assert!(
                matches!(
                    draw(&whole(&[(name, 1)], 1), &mut StdRng::seed_from_u64(1)),
                    Err(RequestError::UnknownArchetype(n)) if n == name
                ),
                "{name}"
            );
        }
    }

    #[test]
    fn a_giant_archetype_is_spaced_as_a_giant() {
        let system = draw(
            &whole(&[("hot-jupiter", 1), ("jupiter-like", 1)], 3),
            &mut StdRng::seed_from_u64(1),
        )
        .unwrap();
        let p = planets(&system);
        assert_eq!(p[0].body_type, "hot-jupiter");
        assert!(p[0].orbit.semi_major_axis_au < 0.1);
        assert_eq!(p[1].body_type, "gas-giant");
    }

    #[test]
    fn the_same_request_draws_the_same_system() {
        let request = whole(&[("earth-like", 1), ("jupiter-like", 2)], 42);
        let a = draw(&request, &mut StdRng::seed_from_u64(1)).unwrap();
        let b = draw(&request, &mut StdRng::seed_from_u64(99)).unwrap();
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap()
        );
    }

    #[test]
    fn requests_beyond_the_limits_fail_with_their_reason() {
        let too_many = whole(&[("mercury-like", 16)], 1);
        assert!(matches!(
            draw(&too_many, &mut StdRng::seed_from_u64(1)),
            Err(RequestError::TooManyPlanets { asked: 16, max: 15 })
        ));
        let crowded = whole(&[("earth-like", 12)], 1);
        match draw(&crowded, &mut StdRng::seed_from_u64(1)) {
            Err(RequestError::ZoneFull {
                body_type,
                asked,
                fits,
            }) => {
                assert_eq!((body_type.as_str(), asked), ("earth-like", 12));
                assert!((1..12).contains(&fits));
            }
            other => panic!("expected zone_full, got {other:?}"),
        }
        assert!(matches!(
            draw(
                &whole(&[("gas-dwarf", 1)], 1),
                &mut StdRng::seed_from_u64(1)
            ),
            Err(RequestError::UnknownArchetype(_))
        ));
    }

    #[test]
    fn drawn_planets_keep_the_spacing_with_their_own_masses() {
        // The masses stored, not only those the slots were spaced with.
        for seed in 0..40 {
            let request = SystemRequest::System(WholeSystemRequest {
                seed: Some(seed),
                width: Some(64),
                height: Some(32),
                ..WholeSystemRequest::default()
            });
            let system = draw(&request, &mut StdRng::seed_from_u64(1)).unwrap();
            let star_mass = f64::from(system.star.info.mass);
            let mut planets = planets(&system);
            planets.sort_by(|a, b| {
                a.orbit
                    .semi_major_axis_au
                    .total_cmp(&b.orbit.semi_major_axis_au)
            });
            for pair in planets.windows(2) {
                let slot = |b: &Body| Slot {
                    // Spaced by what it is: a giant as one, a hot Jupiter too.
                    placement: match b.body_type.as_str() {
                        "hot-jupiter" => Placement::Giant(PlanetClass::HotJupiter),
                        "gas-giant" => Placement::Giant(PlanetClass::GasGiant),
                        "ice-giant" => Placement::Giant(PlanetClass::IceGiant),
                        _ => Placement::Drawn(Origin::Small),
                    },
                    mass_earth: b.physical.mass_earth,
                    orbit_au: b.orbit.semi_major_axis_au,
                    drawn_au: b.orbit.semi_major_axis_au,
                    eccentricity: b.orbit.eccentricity,
                    fixed: false,
                };
                // A hair of slack for the stored values' rounding.
                let (mut inner, outer) = (slot(pair[0]), slot(pair[1]));
                inner.orbit_au *= 0.999;
                assert!(
                    clear(&inner, &outer, star_mass),
                    "seed {seed}: {} and {}",
                    pair[0].id,
                    pair[1].id
                );
            }
        }
    }

    #[test]
    fn a_drawn_system_sometimes_has_a_hot_jupiter() {
        let (mut hot, mut close) = (0, 0);
        let n = 600;
        for seed in 0..n {
            let request = SystemRequest::System(WholeSystemRequest {
                star: Some("G2V".into()),
                metallicity: Some(0.0),
                seed: Some(seed),
                width: Some(64),
                height: Some(32),
                ..WholeSystemRequest::default()
            });
            let system = draw(&request, &mut StdRng::seed_from_u64(1)).unwrap();
            let planets = planets(&system);
            if let Some(h) = planets.iter().find(|b| b.body_type == "hot-jupiter") {
                hot += 1;
                // Lonely: no planet within four times its period.
                let a = h.orbit.semi_major_axis_au;
                close += planets
                    .iter()
                    .filter(|b| b.body_type != "hot-jupiter")
                    .filter(|b| b.orbit.semi_major_axis_au < a * 4f64.powf(2.0 / 3.0))
                    .count();
            }
        }
        // About 1 % of Sun-like stars (Wright et al. 2012).
        assert!((1..=15).contains(&hot), "{hot} of {n}");
        assert_eq!(close, 0);
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

    /// The sun's declination at angle `theta` along a body's orbit, from
    /// the geometry of `web/src/lib/systemSky.ts` (`orbitPlane`,
    /// `pointOnOrbit`, `placeSystem`): the plane's node line, normal and
    /// third axis in a parent frame whose axes are the unit vectors, the
    /// body's axis the normal tilted about the node line.
    fn sky_declination(orbit: &Orbit, tilt_deg: f64, theta: f64) -> f64 {
        type V = [f64; 3];
        let dot = |a: V, b: V| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let cross = |a: V, b: V| {
            [
                a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0],
            ]
        };
        let scale = |a: V, k: f64| [a[0] * k, a[1] * k, a[2] * k];
        let add = |a: V, b: V| [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
        let rotate = |v: V, axis: V, angle: f64| {
            let (c, s) = (angle.cos(), angle.sin());
            add(
                add(scale(v, c), scale(cross(axis, v), s)),
                scale(axis, dot(axis, v) * (1.0 - c)),
            )
        };
        let node_angle = orbit.node_deg.to_radians();
        // pz cos(node) + px sin(node), with px = x, py = y, pz = z.
        let node = [node_angle.sin(), 0.0, node_angle.cos()];
        let normal = rotate([0.0, 1.0, 0.0], node, orbit.inclination_deg.to_radians());
        let third = cross(normal, node);
        let axis = rotate(normal, node, tilt_deg.to_radians());
        // The body's place from the star; the sun is the other way.
        let place = add(scale(node, theta.cos()), scale(third, theta.sin()));
        (-dot(place, axis)).clamp(-1.0, 1.0).asin()
    }

    #[test]
    fn a_body_perihelion_longitude_is_the_sky_s_argument_of_periapsis() {
        use crate::climate::{EccentricOrbit, declination};
        let system = draw(
            &whole(&[("earth-like", 2), ("jupiter-like", 1)], 7),
            &mut StdRng::seed_from_u64(7),
        )
        .unwrap();
        let mut checked = 0;
        for body in &system.bodies {
            let longitude = body.params.perihelion_longitude_deg.expect("drawn") as f64;
            let planet = body.parent.as_ref().map_or(body, |id| {
                system.bodies.iter().find(|b| &b.id == id).unwrap()
            });
            // A planet reads its own orbit, a moon its planet's.
            assert!(
                (longitude - planet.orbit.periapsis_deg.rem_euclid(360.0)).abs() < 1e-3,
                "{}",
                body.id
            );
            if body.kind != BodyKind::Planet {
                continue;
            }
            // The climate's declination at that longitude from the equinox
            // is the sky's at that angle along the orbit, at every angle.
            let tilt = body.spin.axial_tilt_deg;
            for k in 0..24 {
                let theta = f64::from(k) / 24.0 * std::f64::consts::TAU;
                let sky = sky_declination(&body.orbit, tilt, theta);
                let climate = declination(tilt as f32, theta);
                assert!((sky - climate).abs() < 1e-5, "{} {sky} {climate}", body.id);
            }
            checked += 1;
        }
        assert!(checked >= 3);
        // A body whose periapsis is at 90 degrees (perihelion at the
        // northern summer solstice): the sky's sun is north there, and the
        // climate's light peaks in the northern summer.
        let mut orbit = system.bodies[0].orbit.clone();
        orbit.periapsis_deg = 90.0;
        let tilt = 25.0;
        let sky = sky_declination(&orbit, tilt, orbit.periapsis_deg.to_radians());
        assert!((sky - tilt.to_radians()).abs() < 1e-5, "{sky}");
        let mars = EccentricOrbit {
            eccentricity: 0.09,
            perihelion_longitude_deg: orbit.periapsis_deg as f32,
        };
        let solstice = std::f64::consts::FRAC_PI_2;
        let north = crate::climate::daily_insolation(
            60f64.to_radians(),
            declination(tilt as f32, solstice),
        ) * mars.light_at(solstice);
        let south = crate::climate::daily_insolation(
            (-60f64).to_radians(),
            declination(tilt as f32, solstice),
        ) * mars.light_at(solstice);
        assert!(north > 2.0 * south, "{north} {south}");
    }

    #[test]
    fn the_planet_asked_for_has_a_model() {
        let world = WorldRequest {
            planet: Some("earth-like".into()),
            seed: Some(21),
            width: Some(64),
            height: Some(32),
            ..WorldRequest::default()
        };
        let resolved = world.resolve(&mut StdRng::seed_from_u64(3)).unwrap();
        let system = draw_around_params(&world, resolved.params.clone(), resolved.model.clone());
        let focus = system.focus.as_deref().expect("the world asked");
        let body = system.bodies.iter().find(|b| b.id == focus).unwrap();
        // The model its resolution derived, stored, and its type its class.
        assert_eq!(body.model.as_ref(), Some(&resolved.model));
        assert_eq!(body.body_type, resolved.model.class.key());
        assert_eq!(body.body_type, "rocky");
        assert_eq!(body.params.seed, resolved.params.seed);
        // Its mass on its card is its model's.
        assert!((body.physical.mass_earth - resolved.model.inputs.mass_earth).abs() < 1e-9);
    }

    #[test]
    fn a_system_around_a_planet_keeps_it_as_asked() {
        let world = WorldRequest {
            planet: Some("earth-like".into()),
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
        // The world asked has the model its resolution derived, and is
        // typed by its class.
        let model = focus.model.as_ref().expect("the world asked has a model");
        assert_eq!(focus.body_type, model.class.key());
        assert_eq!(focus.body_type, "rocky");
        assert_eq!(system.seed, 1234);
        assert_eq!(system.focus.as_deref(), Some(focus.id.as_str()));
        // Another epoch of the same planet is another system.
        let drifted = WorldRequest {
            planet: Some("earth-like".into()),
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
    fn a_giants_preview_is_linear_and_follows_the_bands_it_will_get() {
        let mut checked = 0;
        for seed in 0..10 {
            let system = draw(
                &whole(
                    &[("jupiter-like", 2), ("neptune-like", 1), ("earth-like", 1)],
                    seed,
                ),
                &mut StdRng::seed_from_u64(u64::from(seed)),
            )
            .unwrap();
            for body in &system.bodies {
                if !giant(body) {
                    continue;
                }
                let mass = body.params.giant_mass_earth.unwrap();
                let look = crate::giant_world::Look::draw(&body.params, mass);
                let p = look.info.palette;
                // Linear: the sRGB palette converted, so the viewer's own
                // conversion back gives the globe's bytes.
                let to_srgb = |c: f32| {
                    if c <= 0.003_130_8 {
                        12.92 * c
                    } else {
                        1.055 * c.powf(1.0 / 2.4) - 0.055
                    }
                };
                for (got, want) in body.preview.colours.iter().zip([p.zone, p.belt, p.polar]) {
                    for k in 0..3 {
                        assert!((to_srgb(got[k]) - want[k]).abs() < 1e-4, "{got:?} {want:?}");
                    }
                }
                // The bands the globe is generated with, not the jet count.
                assert_eq!(body.preview.edges, look.band_edges());
                assert_eq!(
                    usize::from(body.preview.bands) + 1,
                    body.preview.edges.len()
                );
                assert!(body.preview.bands > u8::try_from(look.info.jets).unwrap());
                checked += 1;
            }
        }
        assert!(checked > 10, "{checked}");
    }

    #[test]
    fn giants_get_their_rings_from_the_system() {
        let (mut bright, mut faint, mut giants) = (0, 0, 0);
        for seed in 0..40 {
            let system = draw(
                &whole(
                    &[("jupiter-like", 2), ("neptune-like", 1), ("earth-like", 1)],
                    seed,
                ),
                &mut StdRng::seed_from_u64(u64::from(seed)),
            )
            .unwrap();
            for body in &system.bodies {
                if giant(body) {
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
                    if body.params.planet.name == "ice-giant" {
                        assert!(given.rings.as_ref().is_none_or(|r| !r.bright));
                    }
                    // The giant world takes them, and nothing else of its look moves.
                    let mass = body.params.giant_mass_earth.unwrap();
                    let look = crate::giant_world::Look::draw(&body.params, mass).info;
                    assert_eq!(look.rings, given.rings);
                    let own = WorldParams {
                        given_rings: None,
                        impact_factor: None,
                        locked_to_star: None,
                        ridge: None,
                        resonance_to_star: None,
                        gravity: None,
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
                &whole(&[("jupiter-like", 2), ("neptune-like", 2)], seed),
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
                    // At the fluid Roche limit of ice of 0.5 to 0.9 g/cm³.
                    let roche = crate::rings::fluid_roche(giant.physical.density, 0.5);
                    assert!(f64::from(rings.outer) <= roche * 1.01, "{}", rings.outer);
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
                        RingRole::Edge | RingRole::Source | RingRole::Confining => {
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
            &whole(&[("jupiter-like", 3)], 3),
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
            planet: Some("earth-like".into()),
            seed: Some(8),
            host: Some(HostMode::Giant),
            width: Some(64),
            height: Some(32),
            ..WorldRequest::default()
        };
        let system = draw(&SystemRequest::Around(world), &mut StdRng::seed_from_u64(5)).unwrap();
        let giant = host_of(&system);
        let moons: Vec<_> = system
            .bodies
            .iter()
            .filter(|b| b.parent.as_deref() == Some(&giant.id))
            .collect();
        let focus = system.focus.as_deref().expect("the world asked");
        // The world asked is among them, with the model of its resolution.
        assert!(
            moons
                .iter()
                .any(|m| m.id == focus && m.model.is_some() && m.params.giant_mass_earth.is_none())
        );
        assert!(moons.len() >= 2);
    }

    #[test]
    fn a_giants_moon_is_generated_with_its_giant_and_siblings_as_bodies() {
        use crate::export::raw_data::{WorldMetadata, load_layers, read_metadata};
        let request = SystemRequest::Around(WorldRequest {
            planet: Some("earth-like".into()),
            seed: Some(8),
            host: Some(HostMode::Giant),
            width: Some(64),
            height: Some(32),
            ..WorldRequest::default()
        });
        let system = draw(&request, &mut StdRng::seed_from_u64(5)).unwrap();
        let dir = std::env::temp_dir().join(format!("mapgen-host-gen-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        write_system(&dir, &system).unwrap();
        let focus = system.focus.clone().expect("the world asked");
        let done = generate_neighbourhood(&dir, &focus, |_| {}).unwrap();
        let giant = host_of(&system);
        // The giant and every sibling come with the world, as bodies of
        // the system: nothing is written beside it (no `host/`, `moons/`).
        assert_eq!(done[0], focus);
        assert!(done.contains(&giant.id));
        assert!(done.len() >= 3);
        let entries: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert!(entries.iter().all(|n| n == "system.json" || n == "bodies"));
        for id in &done {
            let body = system.bodies.iter().find(|b| &b.id == id).unwrap();
            let size = crate::sizes::size_dir(
                &dir.join("bodies").join(id),
                body.params.width,
                body.params.height,
            );
            let raw = size.join("raw_data");
            let meta: WorldMetadata = read_metadata(&raw).unwrap();
            // The world records its giant; the giant is a world of its own.
            if *id == focus {
                assert!(meta.host.is_some());
            }
            if *id == giant.id {
                assert!(meta.giant.is_some());
            }
            // Read back as the server does.
            load_layers(&size).unwrap();
        }
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn opening_a_body_generates_it_and_its_family_once() {
        // A rocky planet with moons, small maps.
        let request = SystemRequest::Around(WorldRequest {
            planet: Some("mars-like".into()),
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
                    && b.id == system.focus.as_deref().unwrap()
                    && system
                        .bodies
                        .iter()
                        .any(|m| m.parent.as_ref() == Some(&b.id))
            })
            .expect("the planet asked and its moons");
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
            let params = &system.bodies.iter().find(|b| &b.id == id).unwrap().params;
            let body = dir.join("bodies").join(id);
            let world = crate::sizes::size_dir(&body, params.width, params.height)
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

    /// A lone barren planet with no moons, drawn at 256 × 128 into a
    /// directory of its own; returns the directory and the planet's id.
    fn lone_planet(name: &str) -> (std::path::PathBuf, String) {
        let request = SystemRequest::Around(WorldRequest {
            planet: Some("mars-like".into()),
            seed: Some(7),
            moons: Some(MoonsMode::None),
            alone: Some(true),
            width: Some(256),
            height: Some(128),
            ..WorldRequest::default()
        });
        let system = draw(&request, &mut StdRng::seed_from_u64(1)).unwrap();
        let dir = std::env::temp_dir().join(format!("mapgen-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        write_system(&dir, &system).unwrap();
        let focus = system.focus.clone().expect("the planet asked");
        (dir, focus)
    }

    #[test]
    fn a_body_has_no_svg_and_no_noise_maps_unless_asked() {
        use crate::export::Diagnostics;
        use crate::sizes::size_dir;
        let (dir, id) = lone_planet("system-diagnostics-off");
        generate_neighbourhood(&dir, &id, |_| {}).unwrap();
        let size = size_dir(&dir.join("bodies").join(&id), 256, 128);
        assert!(size.join("world.png").is_file());
        assert!(size.join("legend.png").is_file());
        assert!(!size.join("world.svg").exists());
        assert!(!size.join("noise_maps").exists());
        // Another resolution follows the same rule.
        generate_size(&dir, &id, 512, 256, Diagnostics::default(), |_| {}).unwrap();
        let added = size_dir(&dir.join("bodies").join(&id), 512, 256);
        assert!(added.join("world.png").is_file());
        assert!(!added.join("world.svg").exists());
        assert!(!added.join("noise_maps").exists());

        // On request: the debug option writes the noise maps, the SVG
        // is a separate switch.
        let (dir, id) = lone_planet("system-diagnostics-on");
        let debug = Diagnostics {
            svg: false,
            noise_maps: true,
        };
        generate_neighbourhood_with(&dir, &id, debug, |_| {}).unwrap();
        let size = size_dir(&dir.join("bodies").join(&id), 256, 128);
        assert!(size.join("noise_maps").read_dir().unwrap().next().is_some());
        assert!(!size.join("world.svg").exists());
        let (dir, id) = lone_planet("system-diagnostics-svg");
        generate_neighbourhood_with(&dir, &id, Diagnostics::all(), |_| {}).unwrap();
        let size = size_dir(&dir.join("bodies").join(&id), 256, 128);
        assert!(size.join("world.svg").is_file());
        assert!(size.join("noise_maps").is_dir());
    }

    #[test]
    fn draws_the_svg_of_a_stored_body_and_of_a_missing_one() {
        use crate::sizes::size_dir;
        let (dir, id) = lone_planet("system-svg-on-demand");
        generate_neighbourhood(&dir, &id, |_| {}).unwrap();
        // From the stored raw data.
        let stored = body_svg(&dir, &id, 256, 128).unwrap();
        let text = String::from_utf8(stored).unwrap();
        assert!(text.starts_with("<?xml") && text.contains("<svg"));
        assert!(text.contains(r#"width="256" height="128""#));
        // Without its raw data: regenerated from the seed and request.
        let size = size_dir(&dir.join("bodies").join(&id), 256, 128);
        std::fs::remove_dir_all(size.join("raw_data")).unwrap();
        let regenerated = body_svg(&dir, &id, 256, 128).unwrap();
        assert!(String::from_utf8(regenerated).unwrap().contains("<svg"));
        assert!(body_svg(&dir, "zz", 256, 128).is_err());
    }

    #[test]
    fn each_resolution_of_a_body_has_its_own_directory() {
        use crate::sizes::size_dir;
        let (dir, id) = lone_planet("system-sizes");
        assert_eq!(
            generate_neighbourhood(&dir, &id, |_| {}).unwrap(),
            [id.as_str()]
        );
        let body = dir.join("bodies").join(&id);
        assert!(body.join("thumb.png").is_file());
        assert!(!body.join("world.png").exists());
        assert!(!body.join("raw_data").exists());
        let original = size_dir(&body, 256, 128);
        assert!(original.join("raw_data").join("world.json").is_file());
        assert!(original.join("world.png").is_file());
        assert!(!original.join("thumb.png").exists());
        assert_eq!(crate::sizes::sizes_on_disk(&body), [(256, 128)]);

        // Another resolution: a directory beside it, system.json untouched.
        let system_json = std::fs::read(dir.join("system.json")).unwrap();
        let thumb = std::fs::read(body.join("thumb.png")).unwrap();
        generate_size(&dir, &id, 512, 256, Diagnostics::default(), |_| {}).unwrap();
        let added = size_dir(&body, 512, 256);
        let world: serde_json::Value = serde_json::from_slice(
            &std::fs::read(added.join("raw_data").join("world.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            (world["width"].as_u64(), world["height"].as_u64()),
            (Some(512), Some(256))
        );
        assert_eq!(world["body"], serde_json::Value::from(id.as_str()));
        assert!(world["system"].is_string());
        assert!(!added.join("thumb.png").exists());
        assert_eq!(std::fs::read(body.join("thumb.png")).unwrap(), thumb);
        assert_eq!(std::fs::read(dir.join("system.json")).unwrap(), system_json);
        assert_eq!(crate::sizes::sizes_on_disk(&body), [(256, 128), (512, 256)]);
        assert!(!body.join("sizes").join(".tmp-512x256").exists());

        // A size on disk already (the original too), an unknown body.
        assert!(generate_size(&dir, &id, 512, 256, Diagnostics::default(), |_| {}).is_err());
        assert!(generate_size(&dir, &id, 256, 128, Diagnostics::default(), |_| {}).is_err());
        assert!(generate_size(&dir, "zz", 512, 256, Diagnostics::default(), |_| {}).is_err());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_size_of_a_body_not_generated_is_refused() {
        let (dir, id) = lone_planet("system-size-ungenerated");
        let error = generate_size(&dir, &id, 512, 256, Diagnostics::default(), |_| {}).unwrap_err();
        assert!(error.contains("not generated"), "{error}");
        assert!(!dir.join("bodies").join(&id).exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_system_of_another_format_is_refused() {
        let (dir, _) = lone_planet("system-old-format");
        let path = dir.join("system.json");
        let mut json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        json["format_version"] = serde_json::Value::from(1);
        std::fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();
        let error = read_system(&dir).unwrap_err();
        assert!(error.contains("format version 1"), "{error}");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_system_of_version_2_is_refused() {
        let (dir, _) = lone_planet("system-format-2");
        let path = dir.join("system.json");
        let mut json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        json["format_version"] = serde_json::Value::from(2);
        std::fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();
        let error = read_system(&dir).unwrap_err();
        assert!(error.contains("format version 2"), "{error}");
        let _ = std::fs::remove_dir_all(dir);
    }

    /// The whole system `seed` draws round a G2V at metallicity 0, its
    /// planets drawn (as `drawn()` in `planet::tests`).
    fn drawn_g2v(seed: u32) -> System {
        let request = SystemRequest::System(WholeSystemRequest {
            age_gyr: None,
            star: Some("G2V".into()),
            metallicity: Some(0.0),
            planets: None,
            seed: Some(seed),
            width: Some(64),
            height: Some(32),
        });
        draw(&request, &mut StdRng::seed_from_u64(u64::from(seed))).unwrap()
    }

    /// The system `seed` draws round a G2V at metallicity 0 with the
    /// planets `counts` asks for, by type.
    fn requested(seed: u32, counts: &[(&str, u32)]) -> System {
        let request = SystemRequest::System(WholeSystemRequest {
            metallicity: Some(0.0),
            ..match whole(counts, seed) {
                SystemRequest::System(w) => w,
                SystemRequest::Around(_) => unreachable!(),
            }
        });
        draw(&request, &mut StdRng::seed_from_u64(u64::from(seed))).unwrap()
    }

    #[test]
    fn every_planet_stores_its_model_and_its_class() {
        // Around a planet: every planet, the world asked included.
        for seed in 1..=10 {
            let world = WorldRequest {
                planet: Some("earth-like".into()),
                seed: Some(seed),
                width: Some(64),
                height: Some(32),
                ..WorldRequest::default()
            };
            let system =
                draw(&SystemRequest::Around(world), &mut StdRng::seed_from_u64(1)).unwrap();
            let focus = system.focus.as_deref().expect("the world asked");
            for body in system.bodies.iter().filter(|b| b.kind == BodyKind::Planet) {
                let model = body.model.as_ref().expect("a planet has a model");
                assert_eq!(body.body_type, model.class.key(), "{}", body.id);
            }
            let asked = system.bodies.iter().find(|b| b.id == focus).unwrap();
            assert!(asked.model.is_some(), "seed {seed}");
        }
        // A whole system: every planet.
        for seed in 1..=20 {
            let system = drawn_g2v(seed);
            for body in &system.bodies {
                match body.kind {
                    BodyKind::Planet => {
                        let model = body.model.as_ref().expect("a planet has a model");
                        assert_eq!(body.body_type, model.class.key(), "{}", body.id);
                        assert_eq!(body.physical.mass_earth, model.inputs.mass_earth);
                    }
                    BodyKind::Moon => assert!(body.model.is_none(), "{}", body.id),
                }
            }
        }
    }

    #[test]
    fn the_stored_model_is_the_derived_one() {
        let system = drawn_g2v(3);
        let mut checked = 0;
        for body in system.bodies.iter().filter(|b| b.kind == BodyKind::Planet) {
            let p = &body.params;
            let star = StarInfo::new(p.star, p.orbit_au, p.eccentricity, p.age_gyr);
            let context = Context::of(&star, p.metallicity, system.star.ice_line_au);
            let model = body.model.clone().unwrap();
            assert_eq!(
                derive_model(system.seed, &body.id, &context, model.drawn_mass_earth),
                model,
                "{}",
                body.id
            );
            checked += 1;
        }
        assert!(checked > 0);
        // And it survives `system.json`.
        let json = serde_json::to_string(&system).unwrap();
        let back: System = serde_json::from_str(&json).unwrap();
        for (a, b) in system.bodies.iter().zip(&back.bodies) {
            assert_eq!(a.model, b.model, "{}", a.id);
        }
    }

    #[test]
    fn requested_slots_store_the_derived_class() {
        // terran=1, gas-giant=1 round a G2V, seeds 1..=10: one planet in
        // each band; each stores a class key, whatever it is.
        for seed in 1..=10 {
            let system = requested(seed, &[("earth-like", 1), ("jupiter-like", 1)]);
            let planets: Vec<_> = system
                .bodies
                .iter()
                .filter(|b| b.kind == BodyKind::Planet)
                .collect();
            assert_eq!(planets.len(), 2);
            for b in planets {
                assert!(
                    PLANET_CLASSES.contains(&b.body_type.as_str()),
                    "{}",
                    b.body_type
                );
            }
        }
    }

    #[test]
    fn a_system_round_trips_through_its_file() {
        let system = draw(
            &whole(&[("earth-like", 1), ("neptune-like", 1)], 3),
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
    fn drawn_system_stores_activity() {
        let system = draw(
            &whole(&[("earth-like", 1)], 5),
            &mut StdRng::seed_from_u64(5),
        )
        .unwrap();
        assert_eq!(
            system.star.activity,
            Some(crate::activity::star_activity(
                &system.star.info,
                system.seed
            ))
        );
    }

    #[test]
    fn legacy_system_gets_activity_on_read() {
        let system = draw(
            &whole(&[("earth-like", 1)], 6),
            &mut StdRng::seed_from_u64(6),
        )
        .unwrap();
        let mut json = serde_json::to_value(&system).unwrap();
        json["star"]
            .as_object_mut()
            .unwrap()
            .remove("activity")
            .unwrap();
        let dir = std::env::temp_dir().join(format!("mapgen-legacy-star-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("system.json"), serde_json::to_vec(&json).unwrap()).unwrap();
        let back = read_system(&dir).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(
            back.star.activity,
            Some(crate::activity::star_activity(&back.star.info, back.seed))
        );
    }

    #[test]
    fn every_body_of_a_system_shares_its_age() {
        let mut other_than_default = false;
        for seed in 1..=8 {
            let system = draw(&whole_drawn(seed), &mut StdRng::seed_from_u64(3)).unwrap();
            let age = system.star.info.age_gyr;
            other_than_default |= age != crate::star::DEFAULT_AGE_GYR;
            assert!(age > 0.0 && age <= system.star.info.lifetime_gyr);
            for body in &system.bodies {
                assert_eq!(body.params.age_gyr, age, "seed {seed}, {}", body.id);
            }
        }
        assert!(other_than_default);
    }

    #[test]
    fn a_requested_age_is_kept() {
        let request = SystemRequest::System(WholeSystemRequest {
            age_gyr: Some(2.0),
            star: Some("G2V".into()),
            planets: Some([("earth-like".to_string(), 1)].into()),
            seed: Some(5),
            width: Some(64),
            height: Some(32),
            ..Default::default()
        });
        let system = draw(&request, &mut StdRng::seed_from_u64(5)).unwrap();
        assert_eq!(system.star.info.age_gyr, 2.0);
        assert!(system.bodies.iter().all(|b| b.params.age_gyr == 2.0));
    }

    #[test]
    fn no_surface_is_older_than_a_young_system() {
        for seed in 1..=4 {
            let request = SystemRequest::System(WholeSystemRequest {
                age_gyr: Some(0.6),
                star: Some("G2V".into()),
                planets: Some([("earth-like".to_string(), 2), ("mars-like".to_string(), 1)].into()),
                seed: Some(seed),
                width: Some(64),
                height: Some(32),
                ..Default::default()
            });
            let system = draw(&request, &mut StdRng::seed_from_u64(u64::from(seed))).unwrap();
            assert!(!system.bodies.is_empty());
            for body in &system.bodies {
                assert!(
                    body.params.surface_age_ga <= 0.6,
                    "seed {seed}, {}: {} Ga",
                    body.id,
                    body.params.surface_age_ga
                );
            }
        }
    }

    #[test]
    fn an_age_does_not_refuse_a_star_that_was_drawn() {
        for seed in 1..=60 {
            let request = SystemRequest::System(WholeSystemRequest {
                age_gyr: Some(5.0),
                seed: Some(seed),
                width: Some(64),
                height: Some(32),
                ..Default::default()
            });
            let system = draw(&request, &mut StdRng::seed_from_u64(1))
                .unwrap_or_else(|e| panic!("seed {seed}: {e}"));
            assert_eq!(system.star.info.age_gyr, 5.0);
        }
    }

    #[test]
    fn an_age_beyond_the_star_life_is_refused() {
        let request = SystemRequest::System(WholeSystemRequest {
            age_gyr: Some(5.0),
            star: Some("B5V".into()),
            seed: Some(5),
            ..Default::default()
        });
        let Err(error) = draw(&request, &mut StdRng::seed_from_u64(5)) else {
            panic!("a B5 star does not live 5 Gyr");
        };
        let text = error.to_string();
        assert!(
            text.contains("age_gyr is 5") && text.contains("main-sequence"),
            "{text}"
        );
        assert!(matches!(error, RequestError::BadAge { lifetime_gyr, .. } if lifetime_gyr < 5.0));
    }

    #[test]
    fn a_system_without_age_reads_with_the_default() {
        let system = draw(
            &whole(&[("earth-like", 1)], 6),
            &mut StdRng::seed_from_u64(6),
        )
        .unwrap();
        let mut json = serde_json::to_value(&system).unwrap();
        json["star"]["info"]
            .as_object_mut()
            .unwrap()
            .remove("age_gyr")
            .unwrap();
        for body in json["bodies"].as_array_mut().unwrap() {
            body["params"]
                .as_object_mut()
                .unwrap()
                .remove("age_gyr")
                .unwrap();
        }
        let dir = std::env::temp_dir().join(format!("mapgen-no-age-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("system.json"), serde_json::to_vec(&json).unwrap()).unwrap();
        let back = read_system(&dir).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        let default = crate::star::DEFAULT_AGE_GYR;
        assert_eq!(back.star.info.age_gyr, default);
        assert!(!back.bodies.is_empty());
        assert!(back.bodies.iter().all(|b| b.params.age_gyr == default));
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
            let Some(giant_au) = planets
                .iter()
                .filter(|b| {
                    b.model.as_ref().is_some_and(|m| {
                        crate::belts::cold_giant(m.class, b.orbit.semi_major_axis_au, ice)
                    })
                })
                .map(|b| b.orbit.semi_major_axis_au)
                .reduce(f64::min)
            else {
                continue;
            };
            zones += 1;
            let (lo, hi) = crate::belts::belt_zone(giant_au);
            for p in planets.iter().filter(|b| !giant(b)) {
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
                &whole(
                    &[("earth-like", 1), ("jupiter-like", 1), ("neptune-like", 1)],
                    seed,
                ),
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
            &whole(&[("earth-like", 1), ("jupiter-like", 1)], 7),
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
            age_gyr: None,
            star: Some("G2V".into()),
            metallicity: Some(0.2),
            planets: None,
            seed: Some(seed),
            width: Some(64),
            height: Some(32),
        })
    }
}

/// The rings of the giant a request asks for, or that its world orbits:
/// shaped as every other giant's.
#[cfg(test)]
mod focus_ring_tests {
    use super::tests::host_of;
    use super::*;
    use crate::rings::RingRole;

    fn around(planet: &str, seed: u32, host: Option<HostMode>) -> System {
        let world = WorldRequest {
            planet: Some(planet.into()),
            seed: Some(seed),
            host,
            width: Some(64),
            height: Some(32),
            ..WorldRequest::default()
        };
        draw(
            &SystemRequest::Around(world),
            &mut StdRng::seed_from_u64(u64::from(seed)),
        )
        .unwrap()
    }

    #[test]
    fn a_giant_asked_for_has_its_own_mass_and_rings_shaped_by_it() {
        let (mut bright, mut owned) = (0, 0);
        for seed in 1..=40 {
            let system = around("jupiter-like", seed, None);
            let id = system.focus.as_deref().expect("the giant asked");
            let giant = system.bodies.iter().find(|b| b.id == id).unwrap();
            let mass = giant.params.giant_mass_earth.expect("a giant");
            assert!(
                (giant.physical.mass_earth / f64::from(mass) - 1.0).abs() < 1e-4,
                "seed {seed}: {} against {mass}",
                giant.physical.mass_earth
            );
            let Some(rings) = giant
                .params
                .given_rings
                .as_ref()
                .and_then(|g| g.rings.as_ref())
            else {
                continue;
            };
            // Every gap has its moon, none drawn at random.
            assert_eq!(rings.gap_moons.len(), rings.gaps.len(), "seed {seed}");
            owned += rings.gap_moons.len();
            if rings.bright {
                bright += 1;
                let roche = (
                    crate::rings::fluid_roche(giant.physical.density, 0.9),
                    crate::rings::fluid_roche(giant.physical.density, 0.5),
                );
                let outer = f64::from(rings.outer);
                // The Roche edge, or lower where the giant's innermost
                // regular moon keeps it in.
                let first = system
                    .bodies
                    .iter()
                    .filter(|b| b.parent.as_deref() == Some(id) && b.ring_role.is_none())
                    .map(|b| b.orbit.semi_major_axis_km)
                    .fold(f64::INFINITY, f64::min);
                let capped = outer * giant.physical.radius_km < first;
                assert!(
                    outer < roche.1 * 1.01 && (outer > roche.0 * 0.99 || capped),
                    "seed {seed}: {outer} outside {roche:?}"
                );
                // An edge moon keeps the edge, unless a resonant moon's
                // division or the innermost regular moon leaves no room.
                let roles = |role| {
                    system
                        .bodies
                        .iter()
                        .any(|b| b.parent.as_deref() == Some(id) && b.ring_role == Some(role))
                };
                assert!(
                    roles(RingRole::Edge)
                        || roles(RingRole::Resonant)
                        || first < outer * giant.physical.radius_km * 1.25,
                    "seed {seed}: no edge moon"
                );
            }
        }
        assert!(bright > 3 && owned > 0, "{bright} {owned}");
    }

    #[test]
    fn the_rings_of_a_worlds_giant_stay_inside_its_chain() {
        for seed in 0..30 {
            let system = around("earth-like", seed, Some(HostMode::Giant));
            let giant = host_of(&system);
            let moons: Vec<&Body> = system
                .bodies
                .iter()
                .filter(|b| b.parent.as_deref() == Some(&giant.id))
                .collect();
            let first = moons
                .iter()
                .filter(|m| m.ring_role.is_none())
                .map(|m| m.orbit.semi_major_axis_km)
                .fold(f64::INFINITY, f64::min);
            if let Some(rings) = giant
                .params
                .given_rings
                .as_ref()
                .and_then(|g| g.rings.as_ref())
            {
                assert_eq!(rings.gap_moons.len(), rings.gaps.len());
                assert!(f64::from(rings.outer) * giant.physical.radius_km < first);
            }
            for m in moons.iter().filter(|m| m.ring_role.is_some()) {
                assert!(m.orbit.semi_major_axis_km < first, "seed {seed}: {}", m.id);
            }
        }
    }
}

/// The star's tides on a system's planets, and its moons' spins (see
/// `crate::spin`).
#[cfg(test)]
mod spin_tests {
    use super::tests::giant;
    use super::*;

    fn red_dwarf_system(seed: u32) -> System {
        let request = SystemRequest::System(WholeSystemRequest {
            age_gyr: None,
            star: Some("M4V".into()),
            metallicity: Some(0.0),
            planets: None,
            seed: Some(seed),
            width: Some(64),
            height: Some(32),
        });
        draw(&request, &mut rand::rng()).unwrap()
    }

    /// A rocky planet's day follows its spin state from `star_info`: a
    /// year when locked, a year over the turns in a resonance. The moon
    /// system once drew its own, on an Earth's mass, and could disagree.
    #[test]
    fn rocky_day_follows_the_one_spin_state() {
        let (mut locked, mut resonant) = (0, 0);
        for seed in 0..300 {
            let system = red_dwarf_system(seed);
            for body in system.bodies.iter().filter(|b| b.kind == BodyKind::Planet) {
                if giant(body) {
                    continue;
                }
                let year_h = body.orbit.period_days * 24.0;
                let want = if body.spin.locked {
                    locked += 1;
                    Some(year_h)
                } else if body.spin.resonance_turns > 0.0 {
                    resonant += 1;
                    Some(year_h / f64::from(body.spin.resonance_turns))
                } else {
                    None
                };
                if let Some(want) = want {
                    // The moon system, which sets the day of a planet not
                    // despun, must agree with the card that says it is.
                    if let Some(moons) = MoonSystem::draw(&body.params) {
                        let day = f64::from(moons.planet_day_hours);
                        assert!(
                            (day - want).abs() <= 1e-4 * want,
                            "{}: moons say {day} h, wanted {want} h",
                            body.id
                        );
                    }
                    assert!(
                        (body.spin.period_hours - want).abs() <= 1e-6 * want,
                        "{}: {} h, wanted {want} h",
                        body.id,
                        body.spin.period_hours
                    );
                }
            }
        }
        assert!(
            locked > 0 && resonant > 0,
            "locked {locked}, resonant {resonant}"
        );
    }

    #[test]
    fn planets_locked_to_their_star_turn_on_circles_upright() {
        let (mut locked, mut resonant) = (0, 0);
        for seed in 0..60 {
            let system = red_dwarf_system(seed);
            for body in system.bodies.iter().filter(|b| b.kind == BodyKind::Planet) {
                if giant(body) {
                    continue;
                }
                let info = crate::spin::star_info(&body.params);
                assert_eq!(body.spin.locked, info.tidally_locked, "{}", body.id);
                assert_eq!(body.spin.spin_orbit_3_2, info.spin_orbit_3_2, "{}", body.id);
                assert_eq!(body.spin.resonance_turns, info.spin_turns, "{}", body.id);
                if body.spin.locked {
                    locked += 1;
                    // Below 0.06 always; above it only when no resonance
                    // caught the planet, the odds rising with e.
                    assert!(
                        body.orbit.eccentricity < 0.06
                            || crate::spin::capture(
                                body.orbit.eccentricity as f32,
                                body.params.seed
                            ) == 0.0,
                        "{}",
                        body.id
                    );
                }
                // A circularised orbit is a circle, and its planet locked.
                let (mass, radius) = crate::spin::rocky_body(body.params.circumference_km);
                if crate::spin::rocky_circularised(&info, mass, radius) {
                    assert_eq!(body.orbit.eccentricity, 0.0, "{}", body.id);
                    assert!(body.spin.locked, "{}", body.id);
                }
                if body.spin.spin_orbit_3_2 {
                    resonant += 1;
                    // Three turns every two orbits (two per orbit in 2:1).
                    let year_h = body.orbit.period_days * 24.0;
                    let turns = f64::from(body.spin.resonance_turns);
                    assert!((body.spin.period_hours * turns / year_h - 1.0).abs() < 1e-3);
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
    fn a_giants_moons_weigh_what_their_size_and_density_say() {
        // Around a giant asked for: its moons drawn with its params' mass.
        let request = SystemRequest::Around(WorldRequest {
            planet: Some("jupiter-like".into()),
            star: Some("G2V".into()),
            orbit_au: Some(5.2),
            mass_earth: Some(318.0),
            seed: Some(2),
            width: Some(64),
            height: Some(32),
            ..WorldRequest::default()
        });
        let system = draw(&request, &mut rand::rng()).unwrap();
        let mut checked = 0;
        for moon in system.bodies.iter().filter(|b| b.kind == BodyKind::Moon) {
            if moon.ring_role.is_some() {
                continue;
            }
            let p = &moon.physical;
            let kg = 4.0 / 3.0 * PI * (p.radius_km * 1000.0).powi(3) * p.density * 1000.0;
            assert!(
                (p.mass_earth * EARTH_KG / kg - 1.0).abs() < 1e-3,
                "{}",
                moon.id
            );
            checked += 1;
        }
        assert!(checked >= 2, "{checked}");
    }

    #[test]
    fn a_moons_spin_and_world_share_one_tilt_and_giants_one_recipe() {
        for seed in 0..20 {
            let request = SystemRequest::System(WholeSystemRequest {
                age_gyr: None,
                star: Some("G2V".into()),
                metallicity: Some(0.0),
                planets: Some(
                    [
                        ("earth-like".to_string(), 2),
                        ("jupiter-like".to_string(), 1),
                    ]
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
            for planet in system.bodies.iter().filter(|b| giant(b)) {
                let drawn = MoonSystem::draw(&planet.params).map_or(0, |m| m.moons.len());
                let regular = system
                    .bodies
                    .iter()
                    .filter(|b| b.parent.as_deref() == Some(&planet.id) && b.ring_role.is_none())
                    .count();
                assert_eq!(regular, drawn, "{}", planet.id);
            }
        }
    }
}

#[cfg(test)]
mod gravity_tests {
    use super::*;

    #[test]
    fn a_moon_stored_without_gravity_takes_its_cards() {
        // A world with moons, as a system draws around it.
        let world = WorldRequest {
            planet: Some("earth-like".into()),
            seed: Some(3),
            moons: Some(crate::moons::MoonsMode::Count(2)),
            ..WorldRequest::default()
        };
        let resolved = world.resolve(&mut StdRng::seed_from_u64(3)).unwrap();
        let system = draw_around_params(&world, resolved.params, resolved.model);
        let moon = system
            .bodies
            .iter()
            .find(|b| b.kind == BodyKind::Moon)
            .expect("a moon");
        assert_eq!(world_params(moon).gravity, moon.params.gravity);
        // As stored before moons carried their gravity.
        let mut old = moon.clone();
        old.params.gravity = None;
        let g = world_params(&old).gravity_modifier();
        assert!((f64::from(g) - moon.physical.gravity).abs() < 1e-6);
    }

    #[test]
    fn a_rocky_planet_stored_without_gravity_takes_its_cards() {
        let world = WorldRequest {
            planet: Some("earth-like".into()),
            seed: Some(5),
            ..WorldRequest::default()
        };
        let resolved = world.resolve(&mut StdRng::seed_from_u64(5)).unwrap();
        let system = draw_around_params(&world, resolved.params, resolved.model);
        let mut old = system
            .bodies
            .iter()
            .find(|b| b.kind == BodyKind::Planet && b.params.planet.giant.is_none())
            .expect("a rocky planet")
            .clone();
        // A system made by main stored no gravity; its card shows the
        // gravity of the stored mass and radius.
        old.params.gravity = None;
        old.physical.gravity = 1.94;
        let g = world_params(&old).gravity_modifier();
        assert!((f64::from(g) - 1.94).abs() < 1e-6, "got {g}");
    }
}

/// A lone planet: the star, the planet asked for and its moons.
#[cfg(test)]
mod lone_tests {
    use super::*;

    fn around(planet: &str, seed: u32, host: Option<HostMode>, alone: Option<bool>) -> System {
        let world = WorldRequest {
            planet: Some(planet.into()),
            seed: Some(seed),
            host,
            alone,
            width: Some(64),
            height: Some(32),
            ..WorldRequest::default()
        };
        draw(
            &SystemRequest::Around(world),
            &mut StdRng::seed_from_u64(u64::from(seed)),
        )
        .unwrap()
    }

    fn planets(system: &System) -> Vec<&Body> {
        system
            .bodies
            .iter()
            .filter(|b| b.kind == BodyKind::Planet)
            .collect()
    }

    #[test]
    fn a_lone_planet_has_no_other_planet_belt_or_giant_neighbour() {
        let mut crowded = 0;
        for seed in 1..=30 {
            let full = around("earth-like", seed, None, None);
            let lone = around("earth-like", seed, None, Some(true));
            if planets(&full).len() > 1 {
                crowded += 1;
            }
            let only = planets(&lone);
            assert_eq!(only.len(), 1, "seed {seed}");
            assert_eq!(lone.focus.as_deref(), Some(only[0].id.as_str()));
            assert!(lone.belts.is_empty() && only[0].trojans.is_none());
            assert_eq!(only[0].params.giants, GiantsMode::None, "seed {seed}");
            assert!(system_id(&lone).contains("-lone-"), "seed {seed}");
            assert_ne!(system_id(&lone), system_id(&full));
            assert!(!system_id(&full).contains("lone"));
        }
        assert!(crowded > 20, "{crowded} full systems with company");
    }

    #[test]
    fn a_lone_moon_of_a_giant_keeps_its_host_and_nothing_else() {
        for seed in 1..=10 {
            let lone = around("earth-like", seed, Some(HostMode::Giant), Some(true));
            let only = planets(&lone);
            assert_eq!(only.len(), 1, "seed {seed}");
            assert!(only[0].params.giant_mass_earth.is_some());
            let focus = lone.focus.as_deref().unwrap();
            let moon = lone.bodies.iter().find(|b| b.id == focus).unwrap();
            assert_eq!(moon.parent.as_deref(), Some(only[0].id.as_str()));
            assert!(lone.belts.is_empty());
        }
    }

    #[test]
    fn a_lone_giant_asked_for_is_alone_with_its_moons() {
        let lone = around("jupiter-like", 3, None, Some(true));
        assert_eq!(planets(&lone).len(), 1);
        assert!(lone.bodies.iter().any(|b| b.kind == BodyKind::Moon));
    }

    #[test]
    fn a_request_without_alone_is_the_one_it_was() {
        // Neither the flag left out nor false changes the system.
        for seed in 1..=5 {
            let a = serde_json::to_string(&around("earth-like", seed, None, None)).unwrap();
            let b = serde_json::to_string(&around("earth-like", seed, None, Some(false))).unwrap();
            let strip = |s: &str| s.replace("\"alone\":false,", "");
            assert_eq!(strip(&b), a);
        }
    }

    /// The bodies of a normal system around a planet, as main drew them:
    /// id, type, orbit (AU), impact factor, belts and trojans.
    fn signature(seed: u32) -> String {
        let system = around("earth-like", seed, None, None);
        let bodies: Vec<String> = system
            .bodies
            .iter()
            .map(|b| {
                format!(
                    "{}:{}:{:.4}:{:.4}:{}",
                    b.id,
                    b.body_type,
                    b.orbit.semi_major_axis_au,
                    b.params.impact_factor.unwrap_or(-1.0),
                    b.trojans.is_some()
                )
            })
            .collect();
        format!(
            "{} belts={} {}",
            system_id(&system),
            system.belts.len(),
            bodies.join(" ")
        )
    }

    /// Drawn by the generator since part 3b of #423 sent the world asked
    /// for through the planet model (seeds 1 to 4, around an `earth-like`
    /// world): planets typed by class, the world asked included, drawn
    /// small slots kept near their drawn orbit, and a mini-Neptune no giant
    /// to the small bodies (it shapes no belt, empties no trojan point,
    /// stirs no impactor). Regenerated on purpose then (the world asked
    /// sits at the archetype's star and light, so the systems around it
    /// changed); any other change is a regression of a normal system.
    const MAIN_SIGNATURES: [&str; 4] = [
        "mgs-72469-1 belts=2 b:lava-world:0.0231:0.0822:true c:runaway-greenhouse:0.2071:0.0822:true d:mini-neptune:0.6127:0.0822:true d-1:icy-moon:0.0002:0.0822:false d-2:icy-moon:0.0002:0.0822:false d-3:icy-moon:0.0003:0.0822:false d-4:rocky-moon:0.0006:0.0822:false d-5:icy-moon:0.0010:0.0822:false e:rocky:0.9802:0.0822:true f:gas-giant:7.3269:0.0350:true f-1:icy-moon:0.0006:0.0350:false f-2:icy-moon:0.0009:0.0350:false f-3:icy-moon:0.0009:0.0350:false f-4:icy-moon:0.0011:0.0350:false f-5:rocky-moon:0.0021:0.0350:false f-6:icy-moon:0.0031:0.0350:false f-7:icy-moon:0.0051:0.0350:false f-8:icy-moon:0.0081:0.0350:false f-9:icy-moon:0.0130:0.0350:false g:ice-world:13.6776:0.0350:false g-1:icy-moon:0.0109:0.0350:false g-2:icy-moon:0.0165:0.0350:false g-3:icy-moon:0.0219:0.0350:false",
        "mgs-26800-2 belts=1 b:lava-world:0.0328:0.0365:true c:lava-world:0.1379:0.0365:true d:mini-neptune:0.4847:0.0365:true d-1:icy-moon:0.0002:0.0365:false d-2:rocky-moon:0.0005:0.0365:false d-3:rocky-moon:0.0007:0.0365:false d-4:icy-moon:0.0012:0.0365:false d-5:icy-moon:0.0019:0.0365:false e:rocky:1.0568:0.0365:true f:ice-world:4.9333:0.0365:true",
        "mgs-65727-3 belts=0 b:rocky:0.0476:0.0350:true c:rocky:0.0633:0.0350:true d:rocky:0.6279:0.0350:true d-1:rocky-moon:0.0001:0.0350:false d-2:rocky-moon:0.0002:0.0350:false e:rocky:0.9970:0.0350:true e-1:rocky-moon:0.0002:0.0350:false e-2:rocky-moon:0.0003:0.0350:false f:rocky:2.4861:0.0350:true",
        "mgs-86994-4 belts=0 b:super-earth:0.0409:0.0350:true c:rocky:0.0725:0.0350:true c-1:rocky-moon:0.0002:0.0350:false d:rocky:0.9925:0.0350:true e:hydrocarbon-world:9.6954:0.0350:true e-1:rocky-moon:0.0012:0.0350:false e-2:icy-moon:0.0050:0.0350:false e-3:icy-moon:0.0062:0.0350:false e-4:icy-moon:0.0096:0.0350:false f:ice-world:13.6131:0.0350:true",
    ];

    /// A planet of seed 5 at `orbit_au` round a G2V of 4.6 Gyr on a
    /// circular orbit, 64×32, as a system places it.
    fn placed_at(orbit_au: f32) -> crate::request::Placed {
        crate::request::Placed {
            seed: 5,
            star: parse_type("G2V").unwrap(),
            age_gyr: 4.6,
            orbit_au,
            eccentricity: 0.0,
            metallicity: 0.0,
            axial_tilt_deg: None,
            perihelion_longitude_deg: None,
            drift_ma: 0.0,
            width: 64,
            height: 32,
            moons: MoonsMode::Auto,
            giants: GiantsMode::None,
            host: HostMode::None,
        }
    }

    /// The body of a 6 M⊕ planet at 0.1 AU round a G2V that keeps 2 % of
    /// its mass in hydrogen and helium, placed as a system places it.
    fn mini_neptune_body() -> Body {
        let star = crate::star::parse_type("G2V").unwrap();
        let slot = Slot {
            placement: Placement::Drawn(crate::planet::Origin::Small),
            mass_earth: 6.0,
            orbit_au: 0.1,
            drawn_au: 0.1,
            eccentricity: 0.0,
            fixed: false,
        };
        let at = StarInfo::new(star, 0.1, 0.0, 4.6);
        let context = Context::of(&at, 0.0, 2.7);
        // Formed with more, so that photoevaporation leaves it about 2 %.
        let mut inputs = crate::planet::reference::with_envelope(6.0, 0.02);
        let mut model = crate::planet::planet_model(&context, inputs.clone());
        while model.inputs.envelope_fraction < 0.02 {
            inputs.envelope_fraction *= 1.2;
            model = crate::planet::planet_model(&context, inputs.clone());
        }
        let params = planet_params(&model, &placed_at(0.1));
        let look = base_of(&model);
        planet_body("b", "Test b", &slot, look, Some(model), &params, star)
    }

    #[test]
    fn an_enveloped_rock_renders_its_class_at_the_models_size() {
        // An Earth-mass body at 1 AU round a G2V keeping 0.2 % of its mass
        // in hydrogen and helium (no giant, its surface hidden), as rocky
        // and as an ice world.
        let at = StarInfo::new(parse_type("G2V").unwrap(), 1.0, 0.0, 4.6);
        let context = Context::of(&at, 0.0, 2.7);
        let model = crate::planet::planet_model(
            &context,
            crate::planet::reference::with_envelope(1.0, 0.002),
        );
        assert!(
            model.surface.is_none() && !model.class.is_giant(),
            "{:?}",
            model.class
        );
        let mut icy = model.clone();
        icy.class = PlanetClass::IceWorld;
        for (model, base) in [(model, "barren"), (icy, "frozen")] {
            assert_eq!(base_of(&model), base);
            let params = planet_params(&model, &placed_at(1.0));
            let radius_km = model.interior.radius_earth * EARTH_M / 1000.0;
            let circumference = (2.0 * PI * radius_km) as f32;
            assert!(
                (params.circumference_km - circumference).abs() < 1.0,
                "{base}"
            );
            assert_eq!(
                params.gravity,
                Some(model.interior.gravity as f32),
                "{base}"
            );
            assert_eq!(params.sea_level, crate::request::SEA_LEVEL.min, "{base}");
            assert_eq!(params.planet.climate.temperature, 0.0, "{base}");
            assert_eq!(params.planet.atmosphere.greenhouse, 0.0, "{base}");
            // Its generation aims it at a bare surface's temperature: the
            // equilibrium, plus a little internal heat.
            let slot = Slot {
                placement: Placement::Drawn(crate::planet::Origin::Small),
                mass_earth: 1.0,
                orbit_au: 1.0,
                drawn_au: 1.0,
                eccentricity: 0.0,
                fixed: false,
            };
            let star = at.code();
            let body = planet_body("b", "Test b", &slot, base, Some(model), &params, star);
            let generated = generation_params(&body);
            let mean = crate::generation::mean_surface_temperature_c(&generated)
                + generated.planet.climate.temperature
                + 273.15;
            let eq = f64::from(at.equilibrium_k);
            assert!(
                (f64::from(mean) - eq) > -1.0 && (f64::from(mean) - eq) < 5.0,
                "{base}: {mean} K against {eq} K"
            );
        }
    }

    #[test]
    fn a_planet_is_calibrated_when_its_world_is_generated() {
        // Seed 1: the world asked for and its neighbours.
        let system = around("earth-like", 1, None, None);
        let json = |p: &WorldParams| serde_json::to_string(p).unwrap();
        let (mut calibrated, mut kept) = (0, 0);
        for body in &system.bodies {
            let drawn = world_params(body);
            let generated = generation_params(body);
            match &body.model {
                // A planet with a surface, the world asked for among them:
                // the draw leaves the temperature shift to the generation.
                Some(model) if model.surface.is_some() && drawn.giant_mass_earth.is_none() => {
                    assert_eq!(drawn.planet.climate.temperature, 0.0, "{}", body.id);
                    let mut expected = drawn.clone();
                    crate::planet::interim::calibrate(&mut expected, model);
                    assert_eq!(json(&generated), json(&expected), "{}", body.id);
                    assert_ne!(generated.planet.climate.temperature, 0.0, "{}", body.id);
                    calibrated += 1;
                }
                // A giant world and a moon are not calibrated.
                Some(model) if model.class.is_giant() => {
                    assert_eq!(json(&generated), json(&drawn), "{}", body.id);
                    kept += 1;
                }
                None => {
                    assert_eq!(json(&generated), json(&drawn), "{}", body.id);
                    kept += 1;
                }
                Some(_) => {}
            }
        }
        let focus = system.focus.as_deref().unwrap();
        let asked = system.bodies.iter().find(|b| b.id == focus).unwrap();
        assert!(asked.model.as_ref().unwrap().surface.is_some());
        assert!(calibrated >= 2 && kept >= 1, "{calibrated} {kept}");
    }

    #[test]
    fn a_mini_neptune_generates_a_giant_world() {
        // A 6 M⊕ slot at 0.1 AU round a G2V that keeps 2 % envelope.
        let body = mini_neptune_body();
        assert_eq!(body.body_type, "mini-neptune");
        assert!(body.params.giant_mass_earth.is_some());
        let r = body.physical.radius_km / 6371.0;
        assert!((2.0..=4.0).contains(&r), "{r}");
    }

    #[test]
    fn a_normal_system_is_the_one_main_drew() {
        for (seed, expected) in (1..=4).zip(MAIN_SIGNATURES) {
            assert_eq!(signature(seed), expected, "seed {seed}");
        }
    }

    #[test]
    fn a_lone_planet_keeps_the_world_impact_rate_and_no_giants_when_stored() {
        use crate::world::WorldParams;
        for (planet, host) in [
            ("earth-like", None),
            ("earth-like", Some(HostMode::Giant)),
            ("jupiter-like", None),
        ] {
            let lone = around(planet, 7, host, Some(true));
            let json = serde_json::to_string(&lone).unwrap();
            let stored: System = serde_json::from_str(&json).unwrap();
            let focus = stored
                .bodies
                .iter()
                .find(|b| Some(&b.id) == stored.focus.as_ref())
                .unwrap();
            // As the body is regenerated from its stored params.
            let params: WorldParams = world_params(focus);
            assert!(params.impact_factor.is_none(), "{planet} {host:?}");
            assert_eq!(params.giants, GiantsMode::None, "{planet} {host:?}");
            assert!(
                focus.params.impact_factor.is_none() && focus.params.giants == GiantsMode::None
            );
        }
    }

    #[test]
    fn a_lone_request_reports_the_giants_it_overrides() {
        let resolve = |giants, alone| {
            WorldRequest {
                planet: Some("earth-like".into()),
                seed: Some(3),
                giants,
                alone,
                ..WorldRequest::default()
            }
            .resolve(&mut rand::rng())
            .unwrap()
        };
        let lone = resolve(Some(GiantsMode::Auto), Some(true));
        assert_eq!(lone.ignored, ["giants"]);
        assert_eq!(lone.params.giants, GiantsMode::None);
        assert!(
            resolve(Some(GiantsMode::None), Some(true))
                .ignored
                .is_empty()
        );
        assert!(resolve(Some(GiantsMode::Auto), None).ignored.is_empty());
    }

    #[test]
    fn a_stored_request_without_alone_loads_and_is_not_lone() {
        let json = r#"{"around":{"planet":"earth-like","seed":3}}"#;
        let request: SystemRequest = serde_json::from_str(json).unwrap();
        assert!(!request.is_alone());
        let out = serde_json::to_string(&request).unwrap();
        assert!(!out.contains("alone"), "{out}");
    }
}
