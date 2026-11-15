//! Types exchanged with the web front-end. `cargo test` regenerates their
//! TypeScript definitions in `web/src/lib/api/types/` (see `.cargo/config.toml`).
use mapgen::archetype::{
    ArchetypeInputs, ArchetypeOrbit, ArchetypeReference, Locale, ParamDefault,
};
use mapgen::generation::Stage;
use mapgen::request::{Limits, WorldRequest};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// An archetype the generation form can offer.
#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct ArchetypeInfo {
    /// Value to send as `WorldRequest.planet`: the built-in name, or the
    /// path of the archetype file under the archetypes directory.
    pub spec: String,
    pub name: String,
    pub description: String,
    /// Whether the archetype is embedded in the generator or read from a file.
    pub built_in: bool,
    /// What the archetype sets of the planet model (the rest is drawn).
    pub inputs: ArchetypeInputs,
    /// Where the archetype sits and how it spins.
    pub orbit: ArchetypeOrbit,
    /// Whether its planet renders as a giant (no ground: no drift, moons
    /// of its own to ask for or giant neighbours), as the model derives it
    /// for the archetype asked alone at seed 0: the form hides those
    /// fields and offers a giant's mass instead.
    pub giant: bool,
    /// Whether it is a moon archetype: asked in a whole system, never as a
    /// single world.
    pub moon: bool,
    /// The built-in giant archetype a moon archetype orbits (`null` for
    /// a planet): the giant a system adds when only the moon is asked.
    pub moon_host: Option<String>,
    /// The body it stands for (`null` for a file that names none).
    pub reference: Option<ArchetypeReference>,
    /// Translated name and description, by language code (`fr`).
    pub locales: BTreeMap<String, Locale>,
}

/// Limits and default draw of one numeric parameter.
#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct ParamSpec {
    pub limits: Limits,
    /// Used when neither the request nor the archetype sets the value.
    pub fallback: ParamDefault,
}

/// Limits of the inputs of the planet model a request can set, beside its
/// archetype (the web form offers them from step 5 of #423).
#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct InputLimits {
    /// Earth masses.
    pub mass_earth: Limits,
    /// Iron share of the refractory mass.
    pub core_mass_fraction: Limits,
    /// Earth oceans.
    pub water_oceans: Limits,
    /// Hydrogen and helium share of the mass.
    pub envelope_fraction: Limits,
    /// Factor on the CO₂ formation rule (1 typical).
    pub co2: Limits,
    /// Factor on the N₂ formation rule (1 typical).
    pub n2: Limits,
    /// Factor on the CH₄ formation rule (1 typical).
    pub ch4: Limits,
    /// Factor on the NH₃ formation rule (1 typical).
    pub nh3: Limits,
    /// Factor on the sulphur formation rule (1 typical).
    pub s: Limits,
    /// Earth's = 1.
    pub radiogenic: Limits,
    /// Degrees.
    pub axial_tilt_deg: Limits,
    /// Age of the star, Gyr: above 0 (the lower bound is excluded) and
    /// at most the longest main-sequence life of any star. The star's own
    /// life is checked when the request is resolved (`bad_age`).
    pub age_gyr: Limits,
}

/// A Solar System value of a request field: what the form draws its
/// reference marks at, and quotes in its help.
#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct ReferenceValue {
    /// Lower-case name of the body (`earth`, `mars`, `titan`).
    pub body: String,
    /// The request field (`mass_earth`, `eccentricity`).
    pub field: String,
    /// The value, in the request's own units.
    pub value: f64,
}

/// Everything the generation form needs to validate its inputs.
#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct FormSpec {
    /// Position of the orbit in the star's habitable zone.
    pub hz_position: ParamSpec,
    /// Limits of `orbit_au`, the alternative to `hz_position`.
    pub orbit_au: Limits,
    /// Limits of the orbit's eccentricity.
    pub eccentricity: Limits,
    /// Limits of the number of moons a request can ask for (`moons:
    /// {"count": n}`); without one, the seed draws them.
    pub moon_count: Limits,
    /// Limits of the star's metallicity (dex).
    pub metallicity: Limits,
    /// Limits of the inputs of the planet model a request can set.
    pub inputs: InputLimits,
    pub default_width: u32,
    pub default_height: u32,
    /// The map sizes the form offers, `[width, height]`, all 2:1.
    pub size_presets: Vec<[u32; 2]>,
    /// Largest width or height the server accepts.
    pub max_size: u32,
    /// Largest map the server accepts, in pixels (width × height).
    #[ts(type = "number")]
    pub max_pixels: u64,
    /// The archetypes a whole system can be asked for (built-in names).
    pub archetypes: Vec<String>,
    /// The moon archetypes a whole system can be asked for (built-in
    /// names), apart from `archetypes`, which keeps planets and giants.
    pub moon_archetypes: Vec<String>,
    /// Most planets a system holds (moons not counted).
    pub max_planets: u32,
    /// The Solar System's values of the model fields: the form's reference
    /// marks and the numbers of its help text.
    pub references: Vec<ReferenceValue>,
}

/// State of a generation job, streamed as server-sent events.
#[derive(Clone, Debug, Serialize, TS, PartialEq)]
#[serde(tag = "status", rename_all = "snake_case")]
#[ts(export)]
pub enum JobStatus {
    /// Waiting for an earlier generation to finish.
    Queued,
    /// `step` of `steps` stages started (1-based).
    Running {
        stage: Stage,
        step: u32,
        steps: u32,
        /// A system's job: the body being generated (`null` for a world),
        /// its place in the job (from 1) and how many the job generates.
        body: Option<String>,
        body_step: Option<u32>,
        bodies: Option<u32>,
    },
    Done {
        /// The world, or the system, the job generated.
        world_id: String,
        /// A system's job: the bodies generated, in order (empty for a world).
        bodies: Vec<String>,
    },
    Failed {
        error: String,
    },
    /// Stopped at the client's request (`DELETE /api/jobs/{id}`).
    Cancelled,
}

impl JobStatus {
    pub fn is_finished(&self) -> bool {
        matches!(
            self,
            JobStatus::Done { .. } | JobStatus::Failed { .. } | JobStatus::Cancelled
        )
    }
}

/// A generation job, as listed by `GET /api/jobs`.
#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct JobInfo {
    pub job_id: String,
    /// Id the world will have once generated, or the system's.
    pub world_id: String,
    /// A system's job: the system, and the body it was asked for.
    pub system_id: Option<String>,
    pub body: Option<String>,
    /// The request, as sent by the client, so a reloaded page can show it.
    pub request: WorldRequest,
    pub status: JobStatus,
    /// A job generating one resolution of a body
    /// (`POST /api/systems/{id}/bodies/{body}/sizes/{W}x{H}`): its
    /// `[width, height]`; `null` for a body and its neighbourhood.
    pub size: Option<[u32; 2]>,
}

/// Error body of every failed API call.
#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct ApiError {
    /// What went wrong, in English.
    pub error: String,
    /// The reason in a form clients can word themselves (the web app
    /// translates it), for the errors a user can fix.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub detail: Option<ErrorDetail>,
}

/// Machine-readable reason of a failed request (`code`), with its values.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(tag = "code", rename_all = "snake_case")]
#[ts(export)]
pub enum ErrorDetail {
    /// A parameter is outside its limits (`field` is the request field).
    OutOfRange { field: String, min: f32, max: f32 },
    /// The map width or height is outside the accepted sizes.
    SizeOutOfRange {
        field: String,
        value: u32,
        min: u32,
        max: u32,
    },
    /// No archetype of that name or file.
    UnknownArchetype { archetype: String },
    /// The map has more pixels than this server accepts.
    TooManyPixels {
        width: u32,
        height: u32,
        #[ts(type = "number")]
        max_pixels: u64,
    },
    /// The world is being generated already.
    BeingGenerated { world: String },
    /// A generation is running: tiles and hex grids are paused.
    Busy,
    /// The star is not a spectral type from O3V to M9.5V.
    UnknownStar { star: String },
    /// Both `orbit_au` and `hz_position` are set.
    OrbitAndPosition,
    /// The moons asked for find no stable orbits around the planet (too
    /// close to its star: a Hill sphere too small, or despun by its star's
    /// tides with no moon light enough to last; its star and orbit set).
    NoRoomForMoons { count: u8 },
    /// Moons asked for on the moon of a giant, which keeps none.
    MoonsOfAMoon,
    /// More moons asked for than a giant this close to its star keeps: a
    /// hot giant keeps none, one its star has locked only light ones, and
    /// a small Hill sphere fewer.
    HotGiantMoons { count: u8 },
    /// More water asked for than the planet weighs with its envelope.
    WaterHeavierThanPlanet { water_oceans: f64, mass_earth: f64 },
    /// More planets asked for than a system holds.
    TooManyPlanets { asked: u32, max: u32 },
    /// More planets of a type than its band of distances holds.
    ZoneFull {
        body_type: String,
        asked: u32,
        fits: u32,
    },
    /// A giant's moon archetype asked for alone: ask for it in a whole
    /// system, which gives it its giant.
    BadArchetype { archetype: String },
    /// The age asked is not between 0 and the star's main-sequence life.
    BadAge { age_gyr: f32, lifetime_gyr: f32 },
    /// The archetypes asked together set system ages that do not overlap.
    ArchetypeAges { archetypes: Vec<String> },
    /// A planet type the star leaves no room for.
    TypeImpossible { body_type: String },
    /// The system exists and the request does not replace it.
    SystemExists { system: String },
    /// A resolution that is not one of the offered sizes
    /// (`FormSpec.size_presets`).
    SizeNotOffered { width: u32, height: u32 },
    /// The original resolution of a body cannot be deleted.
    OriginalSize,
    /// The body has no such resolution on disk.
    SizeNotGenerated { width: u32, height: u32 },
}

impl ErrorDetail {
    /// The detail of a request validation error, when it has one (archetype
    /// file errors keep their text only).
    pub fn of_request(error: &mapgen::request::RequestError) -> Option<Self> {
        use mapgen::request::{MAP_SIZE, RequestError};
        match error {
            RequestError::OutOfRange { field, limits } => Some(Self::OutOfRange {
                field: (*field).to_owned(),
                min: limits.min,
                max: limits.max,
            }),
            RequestError::BadSize { field, value } => Some(Self::SizeOutOfRange {
                field: (*field).to_owned(),
                value: *value,
                min: MAP_SIZE.0,
                max: MAP_SIZE.1,
            }),
            RequestError::BadStar(e) => Some(Self::UnknownStar { star: e.clone() }),
            RequestError::OrbitAndPosition => Some(Self::OrbitAndPosition),
            RequestError::NoRoomForMoons { count } => Some(Self::NoRoomForMoons { count: *count }),
            RequestError::MoonsOfAMoon => Some(Self::MoonsOfAMoon),
            RequestError::HotGiantMoons { count } => Some(Self::HotGiantMoons { count: *count }),
            RequestError::UnknownArchetype(name) => Some(Self::UnknownArchetype {
                archetype: name.clone(),
            }),
            RequestError::WaterHeavierThanPlanet {
                water_oceans,
                mass_earth,
            } => Some(Self::WaterHeavierThanPlanet {
                water_oceans: *water_oceans,
                mass_earth: *mass_earth,
            }),
            RequestError::MoonArchetypeAlone(archetype) => Some(Self::BadArchetype {
                archetype: archetype.clone(),
            }),
            // An archetype file that does not load: the catalog serves
            // names only, so no request reaches it; the text is all there is.
            RequestError::BadArchetype(_) => None,
            RequestError::BadAge {
                age_gyr,
                lifetime_gyr,
            } => Some(Self::BadAge {
                age_gyr: *age_gyr,
                lifetime_gyr: *lifetime_gyr,
            }),
            RequestError::ArchetypeAges { archetypes } => Some(Self::ArchetypeAges {
                archetypes: archetypes.clone(),
            }),
            RequestError::TooManyPlanets { asked, max } => Some(Self::TooManyPlanets {
                asked: *asked,
                max: *max,
            }),
            RequestError::ZoneFull {
                body_type,
                asked,
                fits,
            } => Some(Self::ZoneFull {
                body_type: body_type.clone(),
                asked: *asked,
                fits: *fits,
            }),
            RequestError::TypeImpossible { body_type } => Some(Self::TypeImpossible {
                body_type: body_type.clone(),
            }),
        }
    }
}

/// Request to resample a world onto a hex grid.
#[derive(Clone, Debug, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export, optional_fields)]
pub struct HexRequest {
    /// Hex columns around the planet, 4 to 1024 (`mapgen::hex::COLUMNS`).
    pub columns: u32,
    /// River flag threshold in km² (default: `mapgen::hex::DEFAULT_RIVER_FLOW_KM2`).
    pub river_flow_km2: Option<f32>,
}

/// A drawn system, as listed in the library (`GET /api/systems`).
#[derive(Clone, Debug, Serialize, TS, PartialEq)]
#[ts(export)]
pub struct SystemSummary {
    /// Directory name, used in URLs: `/systems/<id>/system.json`.
    pub id: String,
    pub seed: u32,
    /// The map size of every body.
    pub width: u32,
    pub height: u32,
    /// Spectral type of the star.
    pub star: String,
    /// The star's designation (`MGS 48213`).
    pub designation: String,
    /// The colour of the star's light (linear RGB, white-balanced on the
    /// Sun), for the library's orrery.
    pub star_light: [f32; 3],
    /// The habitable zone's inner and outer edges, AU (0 when the system
    /// file does not give them).
    pub hz_inner_au: f32,
    pub hz_outer_au: f32,
    /// How many planets of each class (their `type`: `rocky`, `gas-giant`…).
    pub planets: BTreeMap<String, u32>,
    pub moons: u32,
    /// Every body, planets and moons, in the system's order.
    pub bodies: Vec<BodySummary>,
    /// Names given, by body id (`star` for the star).
    pub names: BTreeMap<String, String>,
    /// When the system was drawn, in milliseconds since the Unix epoch.
    #[ts(type = "number")]
    pub modified_ms: u64,
    /// Pinned in the main menu (`PUT`/`DELETE /api/systems/{id}/pin`): the
    /// library lists pinned systems first. Kept in the index only.
    pub pinned: bool,
    /// Made by an older generator: either its `system.json` is not of the
    /// current format (it cannot be opened, only listed, deleted or, when
    /// it is older, drawn again; the other fields are empty when its file does not give
    /// them), or a world it generated is below the oldest supported
    /// format (see `outdated_reason`).
    pub outdated: bool,
    /// Why the system is `outdated`, `None` when it is not.
    pub outdated_reason: Option<OutdatedReason>,
}

/// Why a system is outdated (`SystemSummary::outdated_reason`), which
/// tells what `POST /api/systems/{id}/regenerate` does.
#[derive(Clone, Copy, Debug, Serialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
#[allow(
    clippy::enum_variant_names,
    reason = "the variants name the format that is outdated"
)]
pub enum OutdatedReason {
    /// `system.json` is of another format: regenerating draws the system
    /// again from its seed (names that no longer fit are lost).
    SystemFormat,
    /// `system.json` is current but a generated world is older than
    /// `MIN_FORMAT_VERSION`: regenerating makes those worlds again, the
    /// system and its names are kept.
    WorldFormat,
    /// `system.json` is of a format newer than this generator's: it
    /// cannot be opened, only listed and deleted. Regenerating would
    /// destroy a system a newer generator can read, so it is refused.
    NewerSystemFormat,
}

/// Answer to `POST /api/systems/{id}/regenerate`.
#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct SystemRegenerated {
    /// The system to show afterwards: `id`, or the new id when the draw
    /// from the same seed named it differently.
    pub system_id: String,
    /// The system that was removed because the new draw has another id.
    pub removed_id: Option<String>,
    /// What was outdated, hence what was done.
    pub reason: OutdatedReason,
    /// The job making the worlds again (`world_format`); none for a
    /// system drawn again (`system_format`), whose bodies are generated
    /// when opened.
    pub job_id: Option<String>,
}

/// A body of a system, as its summary lists it.
#[derive(Clone, Debug, Serialize, TS, PartialEq)]
#[ts(export)]
pub struct BodySummary {
    pub id: String,
    pub designation: String,
    /// `planet` or `moon`.
    pub kind: String,
    /// Its type: a planet's class (`rocky`, `ocean-world`, …), or
    /// `rocky-moon` or `icy-moon` for a moon.
    pub body_type: String,
    /// Its main colour in the preview (linear RGB), for the library's strip.
    pub colour: [f32; 3],
    /// Whether its maps are generated.
    pub generated: bool,
    /// Its orbit's semi-major axis, AU: round the star for a planet, round
    /// its planet for a moon (0 when the system file does not give it).
    pub orbit_au: f64,
    /// Its orbit's eccentricity, and the longitude of its periapsis
    /// (the node's longitude plus the argument of periapsis, degrees,
    /// 0 to 360), for the library's orrery (0 when not given).
    pub eccentricity: f64,
    pub periapsis_deg: f64,
    /// A habitable world: a planet with liquid water on its surface, as
    /// its model says. A moon never is.
    pub habitable: bool,
}

/// Query of a system request (`POST /api/systems?replace=true`).
#[derive(Clone, Debug, Default, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export, optional_fields)]
pub struct SystemQuery {
    /// Replace the system of the same id if it exists, instead of refusing
    /// with 409.
    pub replace: Option<bool>,
}

/// Answer to a system request: the system is drawn and written; a job
/// generates the planet asked for and its neighbourhood (mode `around`).
#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct SystemCreated {
    pub system_id: String,
    pub job_id: Option<String>,
    /// Fields of an `around` request its planet has no use for, ignored (a
    /// giant's `circumference_km`, `erosion`…; a rocky world's `mass_earth`).
    pub ignored: Vec<String>,
}

/// A system as `GET /api/systems/{id}` gives it: its `system.json` and the
/// names given to its bodies.
#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct SystemDetail {
    pub system: mapgen::system::System,
    pub names: BTreeMap<String, String>,
    /// The bodies that were generated once and lost their files (evicted
    /// after 14 days unopened, see `MAPGEN_EVICT_AFTER_DAYS`): not
    /// generated, like a body never generated, but opening their view
    /// generates them again.
    pub evicted: Vec<String>,
    /// What the planet model says of each body that has one, by body id:
    /// the cause trees of its derived values and the archetype it
    /// resembles.
    pub extras: BTreeMap<String, BodyExtras>,
}

/// The planet model's account of one body, for the viewer's cards.
#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct BodyExtras {
    /// Why its derived values are what they are; none when the model
    /// cannot be explained (see `mapgen::planet::explain`).
    pub explanation: Option<mapgen::planet::Explanation>,
    /// The built-in archetype it resembles: its own when it was asked by
    /// archetype, else the nearest one within the threshold.
    pub similar: Option<String>,
}

/// What a request would draw (`POST /api/systems/resolve`): the system
/// `POST /api/systems` writes for it, with what the viewer's cards show.
#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct ResolvedSystem {
    /// The system drawn; its `focus` is the body asked for.
    pub system: mapgen::system::System,
    /// The planet model's account of each body that has one, by body id.
    pub extras: BTreeMap<String, BodyExtras>,
    /// The inputs the focus body formed with, as a request sets them
    /// (`PlanetModel::formation`): what the form shows for a field left
    /// random. `None` when the focus has no model.
    pub formation: Option<mapgen::planet::Formation>,
    /// Fields of an `around` request its planet has no use for.
    pub ignored: Vec<String>,
}

/// The names given to a system's bodies (`PUT /api/systems/{id}/names`):
/// every name, by body id (`star` for the star); one left out goes back to
/// its designation.
#[derive(Clone, Debug, Default, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct SystemNames {
    pub names: BTreeMap<String, String>,
}

/// Body of `PUT /api/systems/{id}/name`: the name of the system itself
/// (the one given to its star); null or blank brings its designation back.
/// The names of its bodies are left as they are.
#[derive(Clone, Debug, Default, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct SystemName {
    pub name: Option<String>,
}

/// Answer to `POST /api/systems/{id}/bodies/{body}/generate`: the job, or
/// none when the body and its neighbourhood are generated already.
#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct GenerateStarted {
    pub job_id: Option<String>,
    /// The bodies the job makes again that another generator than the
    /// current one made before they lost their files: the worlds may look
    /// different from the ones they replace. Every body of the family
    /// (the planet and its moons) that is in this case is named, not only
    /// the one asked for: a planet's job makes its evicted moons too.
    /// Said on every answer while the job is queued or running; the
    /// viewer tells the user once per body.
    pub regenerated: Vec<RegeneratedBody>,
}

/// A body made again by another generator than the one that made it, see
/// [`GenerateStarted::regenerated`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct RegeneratedBody {
    /// The body's id in its system.
    pub body: String,
    /// The generator that made it and the current one.
    pub from: GeneratorNote,
}

/// The generator that made a world, and the current one: what the user is
/// told when an older world is made again.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct GeneratorNote {
    /// Version of the generator that made the world (`0.1.0+<git sha>`: crate
    /// version and build id); null when it was not recorded.
    pub version: Option<String>,
    /// Revision of the data it wrote (`format_version` of its
    /// `world.json`); 0 when not known.
    pub format: Option<u32>,
    /// The generator that makes the world now.
    pub current_version: String,
    pub current_format: u32,
}

/// The resolutions of a generated body
/// (`GET /api/systems/{id}/bodies/{body}/sizes`). Sizes are
/// `[width, height]` in pixels.
#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct BodySizes {
    /// The size the body was generated at (its parameters in
    /// `system.json`); never deleted.
    pub original: [u32; 2],
    /// The size the viewer shows: the one remembered
    /// (`PUT .../displayed-size`) while it is on disk, else the original.
    pub displayed: [u32; 2],
    /// The sizes on disk, the original included, smallest first.
    pub present: Vec<PresentSize>,
    /// The sizes being generated, with their job.
    pub pending: Vec<PendingSize>,
    /// The sizes that can be generated (`POST .../sizes/{W}x{H}`), all 2:1.
    pub presets: Vec<[u32; 2]>,
    /// Largest width or height the server accepts.
    pub max_size: u32,
    /// Largest map the server accepts, in pixels: larger presets are
    /// refused.
    #[ts(type = "number")]
    pub max_pixels: u64,
}

/// A resolution of a body on disk.
#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct PresentSize {
    pub width: u32,
    pub height: u32,
    /// Its revision, as tile and layer URLs take it (`rev=`).
    #[ts(type = "number")]
    pub revision: u64,
}

/// A resolution of a body being generated.
#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct PendingSize {
    pub width: u32,
    pub height: u32,
    /// Its job (`GET /api/jobs/{id}/events`).
    pub job_id: String,
}

/// Body of `PUT /api/systems/{id}/bodies/{body}/displayed-size`: the
/// resolution to show the body at, from now on and in every browser.
#[derive(Clone, Debug, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct DisplayedSize {
    pub width: u32,
    pub height: u32,
}
