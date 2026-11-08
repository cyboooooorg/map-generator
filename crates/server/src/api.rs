//! Types exchanged with the web front-end. `cargo test` regenerates their
//! TypeScript definitions in `web/src/lib/api/types/` (see `.cargo/config.toml`).
use mapgen::generation::Stage;
use mapgen::preset::{Defaults, GiantSpec, ParamDefault, PresetLocale};
use mapgen::request::{Limits, WorldRequest};
use mapgen::star::StarSpec;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// A planet preset the generation form can offer.
#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct PresetInfo {
    /// Value to send as `WorldRequest.planet`.
    pub spec: String,
    pub name: String,
    pub description: String,
    /// Whether the preset is embedded in the generator or read from a file.
    pub built_in: bool,
    pub defaults: Defaults,
    /// The preset's host star and orbit (`[star]`).
    pub star: StarSpec,
    /// A giant planet's `[giant]` table; absent for a rocky world.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub giant: Option<GiantSpec>,
    /// Translated name and description, by language code (`fr`).
    pub locales: BTreeMap<String, PresetLocale>,
}

/// Limits and default draw of one numeric parameter.
#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct ParamSpec {
    pub limits: Limits,
    /// Used when neither the request nor the preset sets the value.
    pub fallback: ParamDefault,
}

/// Everything the generation form needs to validate its inputs.
#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct FormSpec {
    pub sea_level: ParamSpec,
    /// Limits of `land_fraction`, the alternative to `sea_level`; without
    /// it, the sea level falls back as usual.
    pub land_fraction: Limits,
    pub volcanic: ParamSpec,
    pub circumference_km: ParamSpec,
    pub axial_tilt_deg: ParamSpec,
    pub erosion: ParamSpec,
    pub pressure_bar: ParamSpec,
    pub surface_age_ga: ParamSpec,
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
    /// Limits of a giant planet's mass (Earth masses).
    pub giant_mass: Limits,
    pub default_width: u32,
    pub default_height: u32,
    /// Largest width or height the server accepts.
    pub max_size: u32,
    /// Largest map the server accepts, in pixels (width × height).
    #[ts(type = "number")]
    pub max_pixels: u64,
    /// The planet types a whole system can be asked for (preset names).
    pub planet_types: Vec<String>,
    /// Most planets a system holds (moons not counted).
    pub max_planets: u32,
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
    /// Both `sea_level` and `land_fraction` are set.
    SeaLevelAndLandFraction,
    /// No preset of that name or file.
    UnknownPreset { preset: String },
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
    /// More planets asked for than a system holds.
    TooManyPlanets { asked: u32, max: u32 },
    /// More planets of a type than its band of distances holds.
    ZoneFull {
        body_type: String,
        asked: u32,
        fits: u32,
    },
    /// A planet type the star leaves no room for.
    TypeImpossible { body_type: String },
    /// A planet type that is not a preset the draw knows.
    UnknownType { body_type: String },
    /// The system exists and the request does not replace it.
    SystemExists { system: String },
}

impl ErrorDetail {
    /// The detail of a request validation error, when it has one (preset
    /// errors keep their text only).
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
            RequestError::SeaLevelAndLandFraction => Some(Self::SeaLevelAndLandFraction),
            RequestError::BadStar(e) => Some(Self::UnknownStar { star: e.clone() }),
            RequestError::OrbitAndPosition => Some(Self::OrbitAndPosition),
            RequestError::NoRoomForMoons { count } => Some(Self::NoRoomForMoons { count: *count }),
            RequestError::MoonsOfAMoon => Some(Self::MoonsOfAMoon),
            RequestError::HotGiantMoons { count } => Some(Self::HotGiantMoons { count: *count }),
            RequestError::Preset(_) => None,
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
            RequestError::UnknownType(t) => Some(Self::UnknownType {
                body_type: t.clone(),
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
    /// How many planets of each type (preset name).
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
}

/// A body of a system, as its summary lists it.
#[derive(Clone, Debug, Serialize, TS, PartialEq)]
#[ts(export)]
pub struct BodySummary {
    pub id: String,
    pub designation: String,
    /// `planet` or `moon`.
    pub kind: String,
    /// Its type: a preset name.
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
}
