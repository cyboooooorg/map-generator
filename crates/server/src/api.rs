//! Types exchanged with the web front-end. `cargo test` regenerates their
//! TypeScript definitions in `web/src/lib/api/types/` (see `.cargo/config.toml`).
use mapgen::generation::Stage;
use mapgen::preset::{Defaults, ParamDefault};
use mapgen::request::Limits;
use serde::{Deserialize, Serialize};
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
    pub volcanic: ParamSpec,
    pub circumference_km: ParamSpec,
    pub axial_tilt_deg: ParamSpec,
    pub erosion: ParamSpec,
    pub default_width: u32,
    pub default_height: u32,
    /// Largest width or height the server accepts.
    pub max_size: u32,
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
    /// Last modification time, in milliseconds since the Unix epoch.
    #[ts(type = "number")]
    pub modified_ms: u64,
}

/// Answer to a generation request.
#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct JobCreated {
    pub job_id: String,
    /// Id the world will have once generated.
    pub world_id: String,
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
}

impl JobStatus {
    pub fn is_finished(&self) -> bool {
        matches!(self, JobStatus::Done { .. } | JobStatus::Failed { .. })
    }
}

/// Error body of every failed API call.
#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct ApiError {
    pub error: String,
}

/// Request to resample a world onto a hex grid.
#[derive(Clone, Debug, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export, optional_fields)]
pub struct HexRequest {
    /// Hex columns around the planet, 4 to 4096.
    pub columns: u32,
    /// River flag threshold in km² (default: `mapgen::hex::DEFAULT_RIVER_FLOW_KM2`).
    pub river_flow_km2: Option<f32>,
}
