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
}

/// A generated world, as listed in the library.
#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct WorldSummary {
    /// Directory name, used in URLs: `/worlds/<id>/world.png`.
    pub id: String,
    pub planet: String,
    pub seed: u32,
    pub width: u32,
    pub height: u32,
    pub sea_level: f32,
    pub volcanic_intensity: f32,
    pub circumference_km: f32,
    pub axial_tilt_deg: f32,
    pub erosion: f32,
    pub gravity_modifier: f32,
    /// Surface pressure in bar.
    pub pressure_bar: f32,
    /// Age of the oldest surviving surface in Ga; `null` for worlds made
    /// before it existed.
    pub surface_age_ga: Option<f32>,
    /// The epochs the world can be shown at, in Ma before present
    /// (negative in the future): `[future, past]`, both included, or
    /// `null` when it offers none (liquid water today, no craters, or made
    /// before format version 7). See `GET /api/worlds/{id}/epoch/...`.
    pub epochs_ma: Option<[i32; 2]>,
    /// Spectral type of the host star; `null` for worlds made before stars
    /// (the Sun).
    pub star: Option<String>,
    /// Last modification time, in milliseconds since the Unix epoch.
    #[ts(type = "number")]
    pub modified_ms: u64,
    /// The scientific names of its system, and the names given to it and
    /// its moons.
    pub names: WorldNames,
}

/// The names of a world's system. Designations follow the IAU's style for
/// exoplanets (see `mapgen::naming`): the star's catalogue number
/// (`MGS 48213`), a letter for each planet by its distance to the star
/// (`MGS 48213 c`), a Roman numeral for each moon by its distance to its
/// planet (`MGS 48213 c II`).
#[derive(Clone, Debug, Serialize, TS, PartialEq)]
#[ts(export)]
pub struct WorldNames {
    pub star: String,
    /// The world's designation.
    pub designation: String,
    /// The name given to the world, if any.
    pub name: Option<String>,
    /// Designations of the system's other giants, in the order of
    /// `giants.giants` in `world.json`.
    pub giants: Vec<String>,
    /// The world's moons, in the order of `moons.moons` in `world.json`.
    pub moons: Vec<MoonNames>,
    /// Designation of the giant whose moon the world is, if it is one.
    pub host: Option<String>,
    /// Designations of that giant's other moons, in the order of
    /// `host.siblings`.
    pub siblings: Vec<String>,
}

/// A moon's designation and the name given to it, if any.
#[derive(Clone, Debug, Serialize, TS, PartialEq)]
#[ts(export)]
pub struct MoonNames {
    pub designation: String,
    pub name: Option<String>,
}

/// The names given to a world and its moons (`PUT /api/worlds/{id}/names`,
/// stored in the world's `names.json`). The request replaces every given
/// name: a name left out goes back to its designation.
#[derive(Clone, Debug, Default, Serialize, Deserialize, TS, PartialEq)]
#[serde(default, deny_unknown_fields)]
#[ts(export, optional_fields)]
pub struct GivenNames {
    /// The world's name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Names of its moons, by their number (`"1"` is the closest).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub moons: BTreeMap<String, String>,
}

/// Query of a generation request (`POST /api/worlds?replace=true`).
#[derive(Clone, Debug, Default, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export, optional_fields)]
pub struct CreateQuery {
    /// Replace the world of the same id if it exists, instead of refusing
    /// with 409. The old world stays until the new one is complete: a
    /// generation that fails or is cancelled keeps it.
    pub replace: Option<bool>,
}

/// Answer to a generation request.
#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct JobCreated {
    pub job_id: String,
    /// Id the world will have once generated.
    pub world_id: String,
    /// Fields of the request the planet has no use for, ignored (a giant's
    /// `circumference_km`, `erosion`, `volcanic`, `land_fraction`, `giants`
    /// and `host`; a rocky world's `mass_earth`).
    pub ignored: Vec<String>,
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
    },
    Done {
        world_id: String,
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
    /// Id the world will have once generated.
    pub world_id: String,
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
    /// The world exists and the request does not replace it.
    WorldExists { world: String },
    /// A generation is running: tiles and hex grids are paused.
    Busy,
    /// The star is not a spectral type from O3V to M9.5V.
    UnknownStar { star: String },
    /// Both `orbit_au` and `hz_position` are set.
    OrbitAndPosition,
    /// The moons asked for find no stable orbits around the planet (locked
    /// to its star or too close to it, with its star and orbit set).
    NoRoomForMoons { count: u8 },
    /// Moons asked for on the moon of a giant, which keeps none.
    MoonsOfAMoon,
    /// Moons asked for on a hot giant, which keeps none.
    HotGiantMoons { count: u8 },
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
