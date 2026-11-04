//! HTTP routes and shared state.
//!
//! | Route                           | Purpose                                           |
//! |---------------------------------|---------------------------------------------------|
//! | `GET /healthz`                  | liveness probe                                    |
//! | `GET /api/presets`              | planet presets the form can offer                 |
//! | `GET /api/form`                 | parameter limits and defaults                     |
//! | `GET /api/worlds`               | generated worlds, most recent first               |
//! | `POST /api/worlds`              | queue a generation (`WorldRequest` body)          |
//! | `POST /api/worlds?replace=true` | same, replacing the world of that id once done    |
//! | `DELETE /api/worlds/{id}`       | delete a world (409 while it is being generated)  |
//! | `POST /api/worlds/{id}/hex`     | resample a world onto a hex grid (`HexRequest`)   |
//! | `GET /api/worlds/{id}/tiles/{z}/{x}/{y}?rev=&epoch=` | deep-zoom detail tile (PNG), at an epoch in Ma when set |
//! | `GET /api/worlds/{id}/epoch/{ma}/{file}?rev=&scale=` | `surface.png`, `elevation.png` or `biomes.png` of a world at an epoch (see [`mapgen::epoch`]) |
//! | `GET /api/worlds/{id}/thumb?rev=` | thumbnail (PNG), made for older worlds (see [`thumbs`]) |
//! | `GET /api/jobs`                 | recent generation jobs, most recent first         |
//! | `DELETE /api/jobs/{id}`         | cancel a queued or running generation             |
//! | `GET /api/jobs/{id}/events`     | job progress as server-sent events (`JobStatus`)  |
//! | `GET /worlds/{id}/...`          | world files (maps, raw data, diagnostic maps)     |
//! | anything else                   | the web front-end (single-page app)               |
//!
//! World files and the front-end's `index.html` are sent with
//! `Cache-Control: no-cache` (a world id is reused when a world is deleted
//! and generated again; the browser revalidates, a 304 when unchanged), the
//! hashed front-end bundles under `/assets/` with a one-year `immutable`.
//! JSON, JavaScript, CSS and SVG answers above 1 KiB are gzipped (fastest
//! level) when the client accepts it; PNG images are already compressed and
//! sent as they are.
use crate::api::{
    ApiError, CreateQuery, ErrorDetail, FormSpec, HexRequest, JobCreated, JobInfo, JobStatus,
    ParamSpec, PresetInfo, WorldSummary,
};
use crate::catalog::Catalog;
use crate::memory;
use crate::thumbs::{ThumbError, Thumbnails};
use crate::tiles::{self, TileCache, TileError};
use crate::worlds;
use axum::extract::{Path as UrlPath, Query, State};
use axum::http::{Extensions, HeaderMap, HeaderValue, StatusCode, Version, header};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use mapgen::detail::{RENDER_VERSION, TileId};
use mapgen::export::{ExportOptions, export_host, export_moons, export_world, world_dir_name};
use mapgen::generation::{GenerateOptions, Stage, generate_with_progress};
use mapgen::hex::{self, DEFAULT_RIVER_FLOW_KM2, HexMap};
use mapgen::request::{self, WorldRequest, fallback};
use std::collections::HashMap;
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::{Notify, Semaphore, watch};
use tokio_stream::StreamExt;
use tokio_stream::wrappers::WatchStream;
use tower_http::compression::predicate::{Predicate, SizeAbove};
use tower_http::compression::{CompressionLayer, CompressionLevel};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;

/// Largest map side the server accepts.
pub const MAX_SIZE: u32 = 8192;
/// Default largest map the server accepts, in pixels (4096 × 2048). A
/// generation without noise maps, as the server runs it, peaks at about 70
/// bytes per pixel (measured in Docker: 545 MiB here, 2.0 GiB at 8192 ×
/// 4096): keep the limit under the memory the server can use, or it gets
/// killed mid-job.
pub const DEFAULT_MAX_PIXELS: u64 = 4096 * 2048;
/// Peak generation memory per pixel, for the error message.
const BYTES_PER_PIXEL: u64 = 70;
/// Finished jobs kept for `GET /api/jobs`; older ones are forgotten.
const FINISHED_JOBS_KEPT: usize = 20;

/// A generation job: its progress, and how to cancel it.
struct Job {
    world_id: String,
    request: WorldRequest,
    status: watch::Receiver<JobStatus>,
    cancel: Arc<Cancel>,
}

/// Cancellation of a job, seen by the waiting task and the generation thread.
#[derive(Default)]
struct Cancel {
    requested: AtomicBool,
    /// Wakes the task while it waits for its turn in the queue.
    notify: Notify,
}

impl Cancel {
    fn request(&self) {
        self.requested.store(true, Ordering::SeqCst);
        // Stores a permit when the task is not waiting yet.
        self.notify.notify_one();
    }

    fn is_requested(&self) -> bool {
        self.requested.load(Ordering::SeqCst)
    }
}

/// Unwinding payload that stops a cancelled generation at its next stage.
struct Cancelled;

/// Where the server reads and writes its files.
#[derive(Clone, Debug)]
pub struct Config {
    pub worlds_dir: PathBuf,
    pub presets_dir: PathBuf,
    /// Built front-end (`web/dist`); missing in development, where Vite serves it.
    pub static_dir: PathBuf,
    /// Largest map accepted, in pixels (see [`DEFAULT_MAX_PIXELS`]).
    pub max_pixels: u64,
    /// Memory budget of the worlds loaded for tiles and hex grids (see
    /// [`tiles`]).
    pub tile_cache_bytes: usize,
}

pub struct AppState {
    config: Config,
    catalog: Catalog,
    jobs: Mutex<HashMap<String, Job>>,
    next_job: AtomicU64,
    /// One generation at a time: each one already uses every core.
    queue: Semaphore,
    /// Worlds being generated, to refuse a second job writing the same directory.
    pending: Mutex<Vec<String>>,
    tiles: TileCache,
    /// Tiles rendered at once (each one uses every core). A request waits
    /// for its turn without holding a thread, so an abandoned one (the
    /// viewer moved on) is dropped before it renders.
    renders: Semaphore,
    /// Hex grids resampled at once: each one holds a histogram per hex.
    hex_grids: Semaphore,
    thumbnails: Thumbnails,
}

/// Tiles rendered at once.
const CONCURRENT_RENDERS: usize = 4;
/// Cache policy of a URL whose content never changes.
const IMMUTABLE: &str = "public, max-age=31536000, immutable";
/// Smallest answer worth compressing, in bytes.
const COMPRESS_ABOVE: u64 = 1024;

/// Seconds after which a client should retry a tile or a hex grid refused
/// while a generation runs.
const RETRY_AFTER_S: &str = "10";

type Shared = Arc<AppState>;

/// An API error: a status code and a message, sent as [`ApiError`] JSON.
pub struct Failure(StatusCode, String);

impl IntoResponse for Failure {
    fn into_response(self) -> Response {
        Rejection(self, None).into_response()
    }
}

/// A [`Failure`] with the machine-readable reason clients translate.
pub struct Rejection(Failure, Option<ErrorDetail>);

impl From<Failure> for Rejection {
    fn from(failure: Failure) -> Self {
        Self(failure, None)
    }
}

impl IntoResponse for Rejection {
    fn into_response(self) -> Response {
        let Rejection(Failure(status, error), detail) = self;
        (status, Json(ApiError { error, detail })).into_response()
    }
}

/// A world the tile cache cannot serve: 503 with `Retry-After` while a
/// generation runs, 500 when loading failed.
fn world_failure(error: TileError, what: &str) -> Response {
    match error {
        TileError::Busy => (
            StatusCode::SERVICE_UNAVAILABLE,
            [(header::RETRY_AFTER, RETRY_AFTER_S)],
            Json(ApiError {
                error: format!("a generation is running: {what} are paused, retry when it ends"),
                detail: Some(ErrorDetail::Busy),
            }),
        )
            .into_response(),
        TileError::Failed(e) => Failure(StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

pub fn router(config: Config) -> Router {
    // Exports a killed server left unfinished.
    worlds::sweep_temp_dirs(&config.worlds_dir);
    let catalog = Catalog::load(&config.presets_dir);
    let state = Arc::new(AppState {
        catalog,
        jobs: Mutex::new(HashMap::new()),
        next_job: AtomicU64::new(1),
        queue: Semaphore::new(1),
        pending: Mutex::new(Vec::new()),
        tiles: TileCache::new(config.tile_cache_bytes),
        renders: Semaphore::new(CONCURRENT_RENDERS),
        hex_grids: Semaphore::new(1),
        thumbnails: Thumbnails::default(),
        config: config.clone(),
    });
    let spa = ServeDir::new(&config.static_dir)
        .fallback(ServeFile::new(config.static_dir.join("index.html")));
    // Vite names the bundles after their content hash: cache them for good
    // (but not a 404, which a later build may fill).
    let assets = Router::new()
        .nest_service("/assets", ServeDir::new(config.static_dir.join("assets")))
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            |response: &Response| {
                response
                    .status()
                    .is_success()
                    .then_some(HeaderValue::from_static(IMMUTABLE))
            },
        ));
    let files = Router::new()
        .nest_service("/worlds", ServeDir::new(&config.worlds_dir))
        .fallback_service(spa)
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-cache"),
        ));
    let compression = CompressionLayer::new()
        .quality(CompressionLevel::Fastest)
        .compress_when(SizeAbove::new(COMPRESS_ABOVE).and(compressible));
    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/api/presets", get(presets))
        .route("/api/form", get(form))
        .route("/api/worlds", get(list_worlds).post(create_world))
        .route("/api/worlds/{id}", delete(delete_world))
        .route("/api/worlds/{id}/hex", post(hex_grid))
        .route("/api/worlds/{id}/tiles/{z}/{x}/{y}", get(tile))
        .route("/api/worlds/{id}/epoch/{epoch}/{file}", get(epoch_layer))
        .route(
            "/api/worlds/{id}/moons/{moon}/tiles/{z}/{x}/{y}",
            get(moon_tile),
        )
        .route(
            "/api/worlds/{id}/moons/{moon}/epoch/{epoch}/{file}",
            get(moon_epoch_layer),
        )
        .route("/api/worlds/{id}/thumb", get(thumbnail))
        .route("/api/jobs", get(list_jobs))
        .route("/api/jobs/{id}", delete(cancel_job))
        .route("/api/jobs/{id}/events", get(job_events))
        .merge(assets)
        .merge(files)
        .with_state(state)
        .layer(compression)
}

/// Whether an answer is worth compressing: text formats only. PNG images
/// are compressed already, and the job event stream must not be buffered.
fn compressible(_: StatusCode, _: Version, headers: &HeaderMap, _: &Extensions) -> bool {
    let Some(content_type) = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
    else {
        return false;
    };
    let mime = content_type.split(';').next().unwrap_or("").trim();
    matches!(
        mime,
        "application/json"
            | "text/javascript"
            | "application/javascript"
            | "text/css"
            | "image/svg+xml"
    )
}

async fn presets(State(state): State<Shared>) -> Json<Vec<PresetInfo>> {
    Json(state.catalog.list())
}

async fn form(State(state): State<Shared>) -> Json<FormSpec> {
    let spec = |limits, fallback| ParamSpec { limits, fallback };
    Json(FormSpec {
        sea_level: spec(request::SEA_LEVEL, fallback::SEA_LEVEL),
        land_fraction: request::LAND_FRACTION,
        volcanic: spec(request::VOLCANIC, fallback::VOLCANIC),
        circumference_km: spec(request::CIRCUMFERENCE_KM, fallback::CIRCUMFERENCE_KM),
        axial_tilt_deg: spec(request::AXIAL_TILT_DEG, fallback::AXIAL_TILT_DEG),
        erosion: spec(request::EROSION, fallback::EROSION),
        pressure_bar: spec(request::PRESSURE_BAR, fallback::PRESSURE_BAR),
        surface_age_ga: spec(request::SURFACE_AGE_GA, fallback::SURFACE_AGE_GA),
        hz_position: spec(request::HZ_POSITION, fallback::HZ_POSITION),
        orbit_au: request::ORBIT_AU,
        eccentricity: request::ECCENTRICITY,
        moon_count: request::MOON_COUNT,
        metallicity: request::METALLICITY,
        giant_mass: request::GIANT_MASS,
        default_width: fallback::WIDTH,
        default_height: fallback::HEIGHT,
        max_size: MAX_SIZE,
        max_pixels: state.config.max_pixels,
    })
}

async fn list_worlds(State(state): State<Shared>) -> Json<Vec<WorldSummary>> {
    let dir = state.config.worlds_dir.clone();
    Json(
        tokio::task::spawn_blocking(move || worlds::list(&dir))
            .await
            .unwrap_or_default(),
    )
}

async fn delete_world(
    State(state): State<Shared>,
    UrlPath(id): UrlPath<String>,
) -> Result<StatusCode, Failure> {
    if !worlds::is_valid_id(&id) {
        return Err(Failure(StatusCode::BAD_REQUEST, "invalid world id".into()));
    }
    if state.pending.lock().expect("pending lock").contains(&id) {
        return Err(Failure(
            StatusCode::CONFLICT,
            format!("world {id} is being generated: cancel its job first"),
        ));
    }
    let dir = state.config.worlds_dir.join(&id);
    if !dir.is_dir() {
        return Err(Failure(StatusCode::NOT_FOUND, format!("no world {id}")));
    }
    let forgotten = state.tiles.forget(&id);
    tokio::fs::remove_dir_all(&dir)
        .await
        .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if forgotten {
        let _ = tokio::task::spawn_blocking(memory::release).await;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Query of a tile request: the world revision and the renderer version
/// the client knows, and the epoch shown (Ma before present, negative in
/// the future; the world as generated without it).
#[derive(serde::Deserialize)]
struct TileQuery {
    rev: Option<u64>,
    render: Option<u32>,
    epoch: Option<i32>,
}

/// Checks that world `id` offers `epoch_ma` (see
/// [`crate::api::WorldSummary::epochs_ma`]): 404 when it offers none, 400
/// out of its range. Epoch 0 is the world as generated (`None`).
async fn checked_epoch(
    state: &AppState,
    id: &str,
    epoch_ma: Option<i32>,
) -> Result<Option<i32>, Failure> {
    let Some(epoch_ma) = epoch_ma.filter(|&e| e != 0) else {
        return Ok(None);
    };
    let worlds_dir = state.config.worlds_dir.clone();
    let owned = id.to_string();
    let summary = tokio::task::spawn_blocking(move || worlds::summary(&worlds_dir, &owned))
        .await
        .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| Failure(StatusCode::NOT_FOUND, format!("no world {id}")))?;
    let Some([future, past]) = summary.epochs_ma else {
        return Err(Failure(
            StatusCode::NOT_FOUND,
            format!(
                "world {id} offers no other epoch: it has liquid water, no craters, \
                 or was made before format version 7"
            ),
        ));
    };
    if !(future..=past).contains(&epoch_ma) {
        return Err(Failure(
            StatusCode::BAD_REQUEST,
            format!("epoch {epoch_ma} Ma is outside the epochs of {id}, {future} to {past} Ma"),
        ));
    }
    Ok(Some(epoch_ma))
}

async fn tile(
    State(state): State<Shared>,
    UrlPath((id, z, x, y)): UrlPath<(String, u32, u32, u32)>,
    Query(query): Query<TileQuery>,
) -> Result<Response, Failure> {
    if !worlds::is_valid_id(&id) {
        return Err(Failure(StatusCode::BAD_REQUEST, "invalid tile".into()));
    }
    body_tile(state, id, TileId { z, x, y }, query).await
}

/// A detail tile of moon `moon` (from 1) of a world, as [`tile`].
async fn moon_tile(
    State(state): State<Shared>,
    UrlPath((id, moon, z, x, y)): UrlPath<(String, u32, u32, u32, u32)>,
    Query(query): Query<TileQuery>,
) -> Result<Response, Failure> {
    let body = moon_body(&id, moon)?;
    body_tile(state, body, TileId { z, x, y }, query).await
}

/// The body path of moon `moon` (from 1) of world `id`, as the world
/// directory holds it: `<id>/moons/<moon>`.
fn moon_body(id: &str, moon: u32) -> Result<String, Failure> {
    if !worlds::is_valid_id(id) || !(1..=MAX_MOONS).contains(&moon) {
        return Err(Failure(StatusCode::BAD_REQUEST, "invalid moon".into()));
    }
    Ok(format!("{id}/moons/{moon}"))
}

/// The world id of a body path: the world itself, or a moon's world.
fn world_of(body: &str) -> &str {
    body.split('/').next().unwrap_or(body)
}

/// More moons than a world can have.
const MAX_MOONS: u32 = 16;

/// A detail tile of a body: a world (`id`) or one of its moons
/// (`id/moons/n`, a world of its own in the world's directory).
async fn body_tile(
    state: Shared,
    id: String,
    tile: TileId,
    query: TileQuery,
) -> Result<Response, Failure> {
    if !tile.is_valid() {
        return Err(Failure(StatusCode::BAD_REQUEST, "invalid tile".into()));
    }
    // A moon is generated with its world: it takes the world's revision.
    let revision = world_revision(&state, world_of(&id)).await?;
    let epoch = checked_epoch(&state, &id, query.epoch).await?;
    let dir = state.config.worlds_dir.join(&id);
    let world = match state.tiles.world_at(&dir, &id, revision, epoch).await {
        Ok(world) => world,
        Err(e) => return Ok(world_failure(e, "tiles")),
    };
    if world.is_giant() {
        return Err(Failure(
            StatusCode::NOT_FOUND,
            "a giant planet has no ground to detail: no detail tiles".into(),
        ));
    }
    // Wait for a turn here, not on a blocking thread: a request the viewer
    // abandons meanwhile is dropped without rendering.
    let _turn = state
        .renders
        .acquire()
        .await
        .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let png = tokio::task::spawn_blocking(move || tiles::render_png(&world, tile))
        .await
        .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    // A URL naming the current revision and renderer never changes: cache
    // it for good.
    let cache = if query.rev == Some(revision) && query.render == Some(RENDER_VERSION) {
        IMMUTABLE
    } else {
        "no-cache"
    };
    Ok((
        [
            (header::CONTENT_TYPE, "image/png"),
            (header::CACHE_CONTROL, cache),
        ],
        png,
    )
        .into_response())
}

/// Query of an epoch layer request: the world revision and the renderer
/// version the client knows, and a reduction factor (1, 2 or 4) for the
/// lighter layers shown while the time slider moves.
#[derive(serde::Deserialize)]
struct EpochQuery {
    rev: Option<u64>,
    render: Option<u32>,
    scale: Option<u32>,
}

/// A base-resolution layer of a world shown at an epoch (Ma before
/// present, negative in the future): `surface.png` (the hillshaded render,
/// as `raw_data/surface.png`), `elevation.png` (16-bit, encoded as
/// `raw_data/elevation.png`) or `biomes.png` (8-bit biome ids). Computed
/// from the world loaded for the tiles at that epoch.
async fn epoch_layer(
    State(state): State<Shared>,
    UrlPath((id, epoch_ma, file)): UrlPath<(String, i32, String)>,
    Query(query): Query<EpochQuery>,
) -> Result<Response, Failure> {
    if !worlds::is_valid_id(&id) {
        return Err(Failure(StatusCode::BAD_REQUEST, "invalid world id".into()));
    }
    body_epoch_layer(state, id, epoch_ma, file, query).await
}

/// A layer of moon `moon` (from 1) of a world at an epoch, as
/// [`epoch_layer`].
async fn moon_epoch_layer(
    State(state): State<Shared>,
    UrlPath((id, moon, epoch_ma, file)): UrlPath<(String, u32, i32, String)>,
    Query(query): Query<EpochQuery>,
) -> Result<Response, Failure> {
    let body = moon_body(&id, moon)?;
    body_epoch_layer(state, body, epoch_ma, file, query).await
}

/// A layer of a body (see [`body_tile`]) at an epoch.
async fn body_epoch_layer(
    state: Shared,
    id: String,
    epoch_ma: i32,
    file: String,
    query: EpochQuery,
) -> Result<Response, Failure> {
    let layer = tiles::EpochLayer::from_file(&file).ok_or_else(|| {
        Failure(
            StatusCode::NOT_FOUND,
            format!("no layer {file}: surface.png, elevation.png or biomes.png"),
        )
    })?;
    let scale = query.scale.unwrap_or(1);
    if ![1, 2, 4].contains(&scale) {
        return Err(Failure(
            StatusCode::BAD_REQUEST,
            "scale must be 1, 2 or 4".into(),
        ));
    }
    // A moon is generated with its world: it takes the world's revision.
    let revision = world_revision(&state, world_of(&id)).await?;
    let epoch = checked_epoch(&state, &id, Some(epoch_ma)).await?;
    let dir = state.config.worlds_dir.join(&id);
    let world = match state.tiles.world_at(&dir, &id, revision, epoch).await {
        Ok(world) => world,
        Err(e) => return Ok(world_failure(e, "epochs")),
    };
    let _turn = state
        .renders
        .acquire()
        .await
        .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let png = tokio::task::spawn_blocking(move || tiles::epoch_png(&world, layer, scale))
        .await
        .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let cache = if query.rev == Some(revision) && query.render == Some(RENDER_VERSION) {
        IMMUTABLE
    } else {
        "no-cache"
    };
    Ok((
        [
            (header::CONTENT_TYPE, "image/png"),
            (header::CACHE_CONTROL, cache),
        ],
        png,
    )
        .into_response())
}

/// Query of a thumbnail request: the world revision the client knows.
#[derive(serde::Deserialize)]
struct ThumbQuery {
    rev: Option<u64>,
}

/// The thumbnail of a world, made from its `world.png` if it has none
/// (see [`crate::thumbs`]).
async fn thumbnail(
    State(state): State<Shared>,
    UrlPath(id): UrlPath<String>,
    Query(query): Query<ThumbQuery>,
) -> Result<Response, Failure> {
    if !worlds::is_valid_id(&id) {
        return Err(Failure(StatusCode::BAD_REQUEST, "invalid world id".into()));
    }
    let revision = world_revision(&state, &id).await?;
    let dir = state.config.worlds_dir.join(&id);
    let png = state.thumbnails.get(&dir).await.map_err(|e| match e {
        ThumbError::NotFound => Failure(StatusCode::NOT_FOUND, format!("no map for {id}")),
        ThumbError::Failed(e) => Failure(StatusCode::INTERNAL_SERVER_ERROR, e),
    })?;
    // Like a tile: a URL naming the current revision never changes.
    let cache = if query.rev == Some(revision) {
        IMMUTABLE
    } else {
        "no-cache"
    };
    Ok((
        [
            (header::CONTENT_TYPE, "image/png"),
            (header::CACHE_CONTROL, cache),
        ],
        png,
    )
        .into_response())
}

/// The revision of a world (see [`worlds::revision`]), or 404.
async fn world_revision(state: &AppState, id: &str) -> Result<u64, Failure> {
    let worlds_dir = state.config.worlds_dir.clone();
    let owned = id.to_string();
    tokio::task::spawn_blocking(move || worlds::revision(&worlds_dir, &owned))
        .await
        .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| Failure(StatusCode::NOT_FOUND, format!("no raw data for {id}")))
}

/// Resamples a world onto a hex grid from the world loaded for the tiles
/// (loading it if needed). Nothing is written: the `hexgrid` tool writes
/// `hex/hex.json` and `hex/hex.png`. While a generation runs, answers 503
/// like the tiles: loading a world then would add to the generation's peak.
async fn hex_grid(
    State(state): State<Shared>,
    UrlPath(id): UrlPath<String>,
    Json(request): Json<HexRequest>,
) -> Result<Response, Failure> {
    if !worlds::is_valid_id(&id) {
        return Err(Failure(StatusCode::BAD_REQUEST, "invalid world id".into()));
    }
    if !hex::COLUMNS.contains(&request.columns) {
        return Err(Failure(
            StatusCode::BAD_REQUEST,
            format!(
                "columns must be between {} and {}",
                hex::COLUMNS.start(),
                hex::COLUMNS.end()
            ),
        ));
    }
    let revision = world_revision(&state, &id).await?;
    let river_flow = request.river_flow_km2.unwrap_or(DEFAULT_RIVER_FLOW_KM2);
    let _turn = state
        .hex_grids
        .acquire()
        .await
        .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let dir = state.config.worlds_dir.join(&id);
    let world = match state.tiles.world(&dir, &id, revision).await {
        Ok(world) => world,
        Err(e) => return Ok(world_failure(e, "hex grids")),
    };
    tokio::task::spawn_blocking(move || {
        hex::resample_world(
            world.world(),
            |i| world.is_river(i),
            request.columns,
            river_flow,
        )
    })
    .await
    .map(|map: HexMap| Json(map).into_response())
    .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Queues a generation. A world of the same id is refused with 409, unless
/// the query asks to replace it: the new world is then generated into its
/// temporary directory as usual and swapped in place of the old one at the
/// end (see [`worlds::TempDir::commit_replacing`]), so a generation that
/// fails or is cancelled keeps the old world. The old world cannot be
/// deleted meanwhile (409, like any world being generated).
async fn create_world(
    State(state): State<Shared>,
    Query(query): Query<CreateQuery>,
    Json(request): Json<WorldRequest>,
) -> Result<(StatusCode, Json<JobCreated>), Rejection> {
    let replace = query.replace.unwrap_or(false);
    let original = request.clone();
    let bad = |msg: String| Failure(StatusCode::BAD_REQUEST, msg);
    let rejected = |msg: String, detail: ErrorDetail| {
        Rejection(Failure(StatusCode::BAD_REQUEST, msg), Some(detail))
    };
    for (field, value) in [("width", request.width), ("height", request.height)] {
        if let Some(value) = value.filter(|&v| v > MAX_SIZE) {
            return Err(rejected(
                format!("{field} must be at most {MAX_SIZE}"),
                ErrorDetail::SizeOutOfRange {
                    field: field.to_owned(),
                    value,
                    min: mapgen::request::MAP_SIZE.0,
                    max: MAX_SIZE,
                },
            ));
        }
    }
    // Only catalog presets: never a path chosen by the client.
    let preset = match &request.planet {
        Some(spec) => Some((
            state.catalog.resolve(spec).ok_or_else(|| {
                rejected(
                    format!("unknown planet preset {spec:?}"),
                    ErrorDetail::UnknownPreset {
                        preset: spec.clone(),
                    },
                )
            })?,
            spec.clone(),
        )),
        None => None,
    };
    // A land fraction samples the relief to find its sea level (50-100 ms
    // of parallel work): off the async workers.
    let resolved = tokio::task::spawn_blocking(move || {
        let rng = &mut rand::rng();
        match preset {
            Some((preset, spec)) => request.resolve_with_preset(preset, spec, rng),
            None => request.resolve(rng),
        }
    })
    .await
    .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| Rejection(bad(e.to_string()), ErrorDetail::of_request(&e)))?;
    let params = resolved.params;
    let pixels = params.width as u64 * params.height as u64;
    if pixels > state.config.max_pixels {
        return Err(rejected(
            format!(
                "{} × {} needs about {:.1} GB of memory; this server accepts up to {} pixels \
                 (MAPGEN_MAX_PIXELS)",
                params.width,
                params.height,
                (pixels * BYTES_PER_PIXEL) as f64 / 1e9,
                state.config.max_pixels
            ),
            ErrorDetail::TooManyPixels {
                width: params.width,
                height: params.height,
                max_pixels: state.config.max_pixels,
            },
        ));
    }

    let world_id = world_dir_name(&params);
    {
        let mut pending = state.pending.lock().expect("pending lock");
        if pending.contains(&world_id) {
            return Err(Rejection(
                Failure(
                    StatusCode::CONFLICT,
                    format!("world {world_id} is being generated already"),
                ),
                Some(ErrorDetail::BeingGenerated {
                    world: world_id.clone(),
                }),
            ));
        }
        if !replace && state.config.worlds_dir.join(&world_id).exists() {
            return Err(Rejection(
                Failure(
                    StatusCode::CONFLICT,
                    format!(
                        "world {world_id} already exists: replace it, delete it or pick another seed"
                    ),
                ),
                Some(ErrorDetail::WorldExists {
                    world: world_id.clone(),
                }),
            ));
        }
        pending.push(world_id.clone());
    }

    let job_id = state.next_job.fetch_add(1, Ordering::Relaxed).to_string();
    let (tx, rx) = watch::channel(JobStatus::Queued);
    let cancel = Arc::new(Cancel::default());
    {
        let mut jobs = state.jobs.lock().expect("jobs lock");
        forget_old_jobs(&mut jobs);
        jobs.insert(
            job_id.clone(),
            Job {
                world_id: world_id.clone(),
                request: original,
                status: rx,
                cancel: cancel.clone(),
            },
        );
    }

    let task_state = state.clone();
    let task_world = world_id.clone();
    tokio::spawn(async move {
        let finish = |status: JobStatus| {
            if let JobStatus::Failed { error } = &status {
                tracing::error!("generation of {task_world} failed: {error}");
            }
            tx.send_replace(status);
            task_state
                .pending
                .lock()
                .expect("pending lock")
                .retain(|id| id != &task_world);
        };
        // A job cancelled while it waits for its turn never starts.
        let _permit = tokio::select! {
            permit = task_state.queue.acquire() => permit,
            () = cancel.notify.notified() => {
                finish(JobStatus::Cancelled);
                return;
            }
        };
        if cancel.is_requested() {
            finish(JobStatus::Cancelled);
            return;
        }
        // Worlds loaded for the tiles would add to the generation's peak:
        // free them, wait for a load in progress, and load none until it
        // ends.
        let paused_tiles = task_state.tiles.suspend().await;
        let worlds_dir = task_state.config.worlds_dir.clone();
        let progress = tx.clone();
        let thread_cancel = cancel.clone();
        let result = tokio::task::spawn_blocking(move || {
            let steps = Stage::ALL.len() as u32;
            let report = |stage: Stage| {
                // Stages cannot be interrupted: a cancelled generation stops
                // when the next one starts. `resume_unwind` skips the panic
                // hook, so nothing is logged as a crash.
                if thread_cancel.is_requested() {
                    std::panic::resume_unwind(Box::new(Cancelled));
                }
                let step = Stage::ALL.iter().position(|s| *s == stage).unwrap_or(0) as u32 + 1;
                progress.send_replace(JobStatus::Running { stage, step, steps });
            };
            // The web app does not show the diagnostic maps: skip them (no
            // debug sink), and the time they take.
            let generated = generate_with_progress(&params, GenerateOptions::default(), report);
            report(Stage::Export);
            // Written into a temporary directory, renamed into place once
            // complete: a cancelled, failed or crashed export leaves nothing.
            let id = world_dir_name(&generated.world.params);
            let temp = worlds::TempDir::create(&worlds_dir, &id)?;
            export_world(generated, temp.path(), ExportOptions { raw_data: true })?;
            // The planet's memory is freed by now: its moons, one by one.
            report(Stage::Moons);
            export_moons(&params, temp.path(), ExportOptions { raw_data: true })?;
            // A giant's moon: the giant, for its sky.
            export_host(&params, temp.path(), ExportOptions { raw_data: true })?;
            // The export cannot be interrupted: a cancel that arrived
            // meanwhile drops it here.
            if thread_cancel.is_requested() {
                std::panic::resume_unwind(Box::new(Cancelled));
            }
            let target = worlds_dir.join(&id);
            if replace {
                temp.commit_replacing(&target)
            } else {
                temp.commit(&target)
            }
        })
        .await;
        let status = match result {
            Ok(Ok(())) => {
                // Drop any cached copy of the replaced world: the cache is
                // emptied while a generation runs, but the swap must never
                // leave the old revision behind.
                if replace {
                    task_state.tiles.forget(&task_world);
                }
                JobStatus::Done {
                    world_id: task_world.clone(),
                }
            }
            Ok(Err(e)) => JobStatus::Failed { error: e },
            Err(e) if e.is_panic() && cancel.is_requested() => JobStatus::Cancelled,
            Err(e) => JobStatus::Failed {
                error: format!("generation crashed: {e}"),
            },
        };
        // The generation's memory, freed by now, goes back to the system
        // (a few milliseconds), then tiles resume before the job reports its
        // end, so a viewer opening the new world is served at once.
        let _ = tokio::task::spawn_blocking(memory::release).await;
        drop(paused_tiles);
        finish(status);
    });

    Ok((StatusCode::ACCEPTED, Json(JobCreated { job_id, world_id })))
}

/// Drops the oldest finished jobs beyond [`FINISHED_JOBS_KEPT`].
fn forget_old_jobs(jobs: &mut HashMap<String, Job>) {
    let mut finished: Vec<u64> = jobs
        .iter()
        .filter(|(_, job)| job.status.borrow().is_finished())
        .filter_map(|(id, _)| id.parse().ok())
        .collect();
    if finished.len() < FINISHED_JOBS_KEPT {
        return;
    }
    finished.sort_unstable();
    for id in &finished[..=finished.len() - FINISHED_JOBS_KEPT] {
        jobs.remove(&id.to_string());
    }
}

async fn list_jobs(State(state): State<Shared>) -> Json<Vec<JobInfo>> {
    let jobs = state.jobs.lock().expect("jobs lock");
    let mut list: Vec<(u64, JobInfo)> = jobs
        .iter()
        .map(|(id, job)| {
            (
                id.parse().unwrap_or(0),
                JobInfo {
                    job_id: id.clone(),
                    world_id: job.world_id.clone(),
                    request: job.request.clone(),
                    status: job.status.borrow().clone(),
                },
            )
        })
        .collect();
    list.sort_unstable_by_key(|(id, _)| std::cmp::Reverse(*id));
    Json(list.into_iter().map(|(_, info)| info).collect())
}

async fn cancel_job(
    State(state): State<Shared>,
    UrlPath(id): UrlPath<String>,
) -> Result<StatusCode, Failure> {
    let jobs = state.jobs.lock().expect("jobs lock");
    let job = jobs
        .get(&id)
        .ok_or_else(|| Failure(StatusCode::NOT_FOUND, format!("no job {id}")))?;
    if job.status.borrow().is_finished() {
        return Err(Failure(
            StatusCode::CONFLICT,
            format!("job {id} is already finished"),
        ));
    }
    job.cancel.request();
    // The final status (cancelled) follows on the event stream.
    Ok(StatusCode::ACCEPTED)
}

async fn job_events(
    State(state): State<Shared>,
    UrlPath(id): UrlPath<String>,
) -> Result<Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>>, Failure> {
    let rx = state
        .jobs
        .lock()
        .expect("jobs lock")
        .get(&id)
        .map(|job| job.status.clone())
        .ok_or_else(|| Failure(StatusCode::NOT_FOUND, format!("no job {id}")))?;
    let mut finished = false;
    // Stream every status change; stop after the final one.
    let stream = WatchStream::new(rx)
        .take_while(move |status| {
            let keep = !finished;
            finished = status.is_finished();
            keep
        })
        .map(|status| {
            Ok(Event::default()
                .json_data(status)
                .expect("status serializes"))
        });
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[test]
    fn moons_are_bodies_in_their_world_directory() {
        assert_eq!(
            moon_body("terran-42", 1).ok().as_deref(),
            Some("terran-42/moons/1")
        );
        assert!(moon_body("terran-42", 0).is_err());
        assert!(moon_body("terran-42", MAX_MOONS + 1).is_err());
        assert!(moon_body("../etc", 1).is_err());
        assert_eq!(world_of("terran-42/moons/1"), "terran-42");
        assert_eq!(world_of("terran-42"), "terran-42");
    }

    #[test]
    fn viewer_and_server_agree_on_the_tile_renderer_version() {
        let viewer = include_str!("../../../web/src/lib/viewer/TileLayer.ts");
        let expected = format!("export const TILE_RENDER_VERSION = {RENDER_VERSION};");
        assert!(
            viewer.contains(&expected),
            "TileLayer.ts must declare `{expected}`"
        );
    }

    fn temp_config(name: &str) -> Config {
        let root =
            std::env::temp_dir().join(format!("mapgen-server-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("worlds")).unwrap();
        Config {
            worlds_dir: root.join("worlds"),
            presets_dir: PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../presets")),
            static_dir: root.join("static"),
            max_pixels: DEFAULT_MAX_PIXELS,
            tile_cache_bytes: tiles::DEFAULT_BUDGET_BYTES,
        }
    }

    async fn call(
        app: &Router,
        method: &str,
        uri: &str,
        body: Option<&str>,
    ) -> (StatusCode, String) {
        let mut request = Request::builder().method(method).uri(uri);
        if body.is_some() {
            request = request.header("content-type", "application/json");
        }
        let request = request
            .body(body.map_or_else(Body::empty, |b| Body::from(b.to_string())))
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (status, String::from_utf8_lossy(&bytes).into_owned())
    }

    async fn get_gzip(app: &Router, uri: &str) -> Response {
        let request = Request::get(uri)
            .header(header::ACCEPT_ENCODING, "gzip")
            .body(Body::empty())
            .unwrap();
        app.clone().oneshot(request).await.unwrap()
    }

    #[tokio::test]
    async fn caches_hashed_bundles_and_revalidates_the_page() {
        let config = temp_config("static");
        std::fs::create_dir_all(config.static_dir.join("assets")).unwrap();
        std::fs::write(config.static_dir.join("index.html"), "<!doctype html>").unwrap();
        let script = "console.log('planets');\n".repeat(100);
        std::fs::write(config.static_dir.join("assets/index-abc123.js"), &script).unwrap();
        let app = router(config);

        let response = get_gzip(&app, "/assets/index-abc123.js").await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CACHE_CONTROL], IMMUTABLE);
        assert_eq!(response.headers()[header::CONTENT_ENCODING], "gzip");
        let missing = get_gzip(&app, "/assets/index-old.js").await;
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
        assert!(!missing.headers().contains_key(header::CACHE_CONTROL));

        // The page itself, and any route of the single-page app.
        for uri in ["/", "/index.html", "/some/route"] {
            let response = get_gzip(&app, uri).await;
            assert_eq!(response.status(), StatusCode::OK, "{uri}");
            assert_eq!(
                response.headers()[header::CACHE_CONTROL],
                "no-cache",
                "{uri}"
            );
        }
    }

    #[tokio::test]
    async fn serves_worlds_at_other_epochs() {
        use mapgen::export::{ExportOptions, export_world};
        let config = temp_config("epochs");
        let planet = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../presets/examples/mercury-like.toml"
        );
        for (planet, id) in [(planet, "mercury-like-3"), ("terran", "terran-3")] {
            let request = WorldRequest {
                planet: Some(planet.into()),
                seed: Some(3),
                surface_age_ga: Some(4.0),
                width: Some(256),
                height: Some(128),
                ..Default::default()
            };
            let params = request.resolve(&mut rand::rng()).unwrap().params;
            let generated = mapgen::generation::generate(&params);
            export_world(
                generated,
                &config.worlds_dir.join(id),
                ExportOptions { raw_data: true },
            )
            .unwrap();
        }
        let app = router(config.clone());
        let (_, list) = call(&app, "GET", "/api/worlds", None).await;
        let list: serde_json::Value = serde_json::from_str(&list).unwrap();
        let epochs = |id: &str| {
            list.as_array()
                .unwrap()
                .iter()
                .find(|w| w["id"] == id)
                .unwrap()["epochs_ma"]
                .clone()
        };
        assert_eq!(epochs("mercury-like-3"), serde_json::json!([-1000, 4000]));
        assert_eq!(epochs("terran-3"), serde_json::Value::Null);

        let png = |uri: String| {
            let app = app.clone();
            async move {
                let request = Request::get(&uri).body(Body::empty()).unwrap();
                let response = app.oneshot(request).await.unwrap();
                let status = response.status();
                let bytes = response.into_body().collect().await.unwrap().to_bytes();
                (status, bytes)
            }
        };
        let base = "/api/worlds/mercury-like-3/epoch";
        // The present epoch is the world as generated.
        let (status, now) = png(format!("{base}/0/elevation.png")).await;
        assert_eq!(status, StatusCode::OK);
        let now = image::load_from_memory(&now).unwrap().into_luma16();
        let stored = image::open(
            config
                .worlds_dir
                .join("mercury-like-3/raw_data/elevation.png"),
        )
        .unwrap()
        .into_luma16();
        assert_eq!(now, stored);
        // Back in time and ahead, the relief changes.
        for epoch in [3900, -1000] {
            let (status, then) = png(format!("{base}/{epoch}/elevation.png")).await;
            assert_eq!(status, StatusCode::OK, "{epoch}");
            assert_ne!(
                image::load_from_memory(&then).unwrap().into_luma16(),
                stored
            );
        }
        // Every layer, reduced for the slider.
        for file in ["surface.png", "elevation.png", "biomes.png"] {
            let (status, bytes) = png(format!("{base}/2000/{file}?scale=4")).await;
            assert_eq!(status, StatusCode::OK, "{file}");
            let image = image::load_from_memory(&bytes).unwrap();
            assert_eq!((image.width(), image.height()), (64, 32), "{file}");
        }
        // Tiles at an epoch.
        let (status, _) = call(
            &app,
            "GET",
            "/api/worlds/mercury-like-3/tiles/3/2/1?epoch=-500",
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        // Refused: outside the range, a world without epochs, unknown layer, bad scale.
        for (uri, expected) in [
            (format!("{base}/4100/surface.png"), StatusCode::BAD_REQUEST),
            (format!("{base}/-1001/surface.png"), StatusCode::BAD_REQUEST),
            (
                "/api/worlds/terran-3/epoch/100/surface.png".into(),
                StatusCode::NOT_FOUND,
            ),
            (format!("{base}/100/flow.png"), StatusCode::NOT_FOUND),
            (
                format!("{base}/100/surface.png?scale=3"),
                StatusCode::BAD_REQUEST,
            ),
            (
                "/api/worlds/terran-3/tiles/1/0/0?epoch=100".into(),
                StatusCode::NOT_FOUND,
            ),
        ] {
            let (status, _) = call(&app, "GET", &uri, None).await;
            assert_eq!(status, expected, "{uri}");
        }
    }

    #[tokio::test]
    async fn lists_built_in_and_example_presets() {
        let app = router(temp_config("presets"));
        let (status, body) = call(&app, "GET", "/api/presets", None).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("\"spec\":\"terran\""));
        assert!(body.contains("\"spec\":\"examples/mars-like.toml\""));
    }

    #[tokio::test]
    async fn rejects_invalid_requests() {
        let app = router(temp_config("invalid"));
        for body in [
            r#"{"sea_level": 3.0}"#,
            r#"{"planet": "/etc/passwd.toml"}"#,
            r#"{"width": 100000}"#,
            r#"{"width": 8192, "height": 4096}"#,
            r#"{"unknown": 1}"#,
        ] {
            let (status, _) = call(&app, "POST", "/api/worlds", Some(body)).await;
            assert!(status.is_client_error(), "{body} gave {status}");
        }
        let (status, _) = call(&app, "DELETE", "/api/worlds/..%2Fetc-1", None).await;
        assert!(status.is_client_error());
    }

    #[tokio::test]
    async fn rejected_requests_carry_a_machine_readable_detail() {
        let app = router(temp_config("detail"));
        let detail = |body: &str| -> serde_json::Value {
            serde_json::from_str::<serde_json::Value>(body).unwrap()["detail"].clone()
        };
        let (status, body) = call(&app, "POST", "/api/worlds", Some(r#"{"sea_level": 3.0}"#)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let out_of_range = detail(&body);
        assert_eq!(out_of_range["code"], "out_of_range");
        assert_eq!(out_of_range["field"], "sea_level");
        assert!(out_of_range["min"].is_number() && out_of_range["max"].is_number());
        let (_, body) = call(&app, "POST", "/api/worlds", Some(r#"{"width": 100000}"#)).await;
        assert_eq!(detail(&body)["code"], "size_out_of_range");
        let (_, body) = call(
            &app,
            "POST",
            "/api/worlds",
            Some(r#"{"planet": "nowhere"}"#),
        )
        .await;
        assert_eq!(detail(&body)["code"], "unknown_preset");
        let (_, body) = call(
            &app,
            "POST",
            "/api/worlds",
            Some(r#"{"sea_level": 0.1, "land_fraction": 0.3}"#),
        )
        .await;
        assert_eq!(detail(&body)["code"], "sea_level_and_land_fraction");
        // Errors without a detail send none.
        let (_, body) = call(&app, "DELETE", "/api/worlds/..%2Fetc-1", None).await;
        assert!(detail(&body).is_null());
    }

    #[tokio::test]
    async fn generates_lists_and_deletes_a_world() {
        let config = temp_config("generate");
        // An export a killed server left unfinished is swept at startup.
        let leftover = config.worlds_dir.join(".tmp-frozen-3");
        std::fs::create_dir_all(leftover.join("raw_data")).unwrap();
        let app = router(config.clone());
        assert!(!leftover.exists());
        let request = r#"{"planet": "frozen", "seed": 3, "width": 64, "height": 32}"#;
        let (status, body) = call(&app, "POST", "/api/worlds", Some(request)).await;
        assert_eq!(status, StatusCode::ACCEPTED, "{body}");
        let created: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(created["world_id"], "frozen-3");

        // The event stream ends with the final status.
        let uri = format!("/api/jobs/{}/events", created["job_id"].as_str().unwrap());
        let (_, events) = call(&app, "GET", &uri, None).await;
        assert!(events.contains("\"status\":\"done\""), "{events}");

        let (_, list) = call(&app, "GET", "/api/worlds", None).await;
        assert!(list.contains("\"id\":\"frozen-3\""));
        // World files are revalidated, PNG images never compressed.
        let response = get_gzip(&app, "/worlds/frozen-3/world.png").await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-cache");
        assert!(!response.headers().contains_key(header::CONTENT_ENCODING));
        let response = get_gzip(&app, "/worlds/frozen-3/raw_data/world.json").await;
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-cache");
        assert_eq!(response.headers()[header::CONTENT_ENCODING], "gzip");

        // The thumbnail: exported with the world, made again when missing
        // (a world from before thumbnails), cached for good under the
        // current revision.
        let list_json: serde_json::Value = serde_json::from_str(&list).unwrap();
        let rev = list_json[0]["modified_ms"].as_u64().unwrap();
        let thumb_file = config.worlds_dir.join("frozen-3/thumb.png");
        let exported = std::fs::read(&thumb_file).unwrap();
        std::fs::remove_file(&thumb_file).unwrap();
        let uri = format!("/api/worlds/frozen-3/thumb?rev={rev}");
        let response = get_gzip(&app, &uri).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "image/png");
        assert_eq!(response.headers()[header::CACHE_CONTROL], IMMUTABLE);
        assert!(!response.headers().contains_key(header::CONTENT_ENCODING));
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body, exported);
        assert_eq!(std::fs::read(&thumb_file).unwrap(), exported);
        let response = get_gzip(&app, "/api/worlds/frozen-3/thumb?rev=1").await;
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-cache");
        for (bad, expected) in [
            ("/api/worlds/nope-1/thumb", StatusCode::NOT_FOUND),
            ("/api/worlds/..%2Fetc-1/thumb", StatusCode::BAD_REQUEST),
        ] {
            let (status, _) = call(&app, "GET", bad, None).await;
            assert_eq!(status, expected, "{bad}");
        }

        // Same planet and seed again: the directory exists.
        let (status, _) = call(&app, "POST", "/api/worlds", Some(request)).await;
        assert_eq!(status, StatusCode::CONFLICT);

        let (status, hex) = call(
            &app,
            "POST",
            "/api/worlds/frozen-3/hex",
            Some(r#"{"columns": 8}"#),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{hex}");
        assert!(hex.contains("\"columns\":8"));
        for columns in [1, 1025] {
            let body = format!(r#"{{"columns": {columns}}}"#);
            let (status, _) = call(&app, "POST", "/api/worlds/frozen-3/hex", Some(&body)).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{columns} columns");
        }

        // Detail tiles: valid ones render as PNG, others are rejected.
        let list: serde_json::Value = serde_json::from_str(&list).unwrap();
        let rev = list[0]["modified_ms"].as_u64().unwrap();
        let (status, _) = call(
            &app,
            "GET",
            &format!("/api/worlds/frozen-3/tiles/2/3/1?rev={rev}"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        for bad in [
            "/api/worlds/frozen-3/tiles/2/8/0",
            "/api/worlds/frozen-3/tiles/9/0/0",
            "/api/worlds/frozen-3/tiles/40/0/0",
            "/api/worlds/nope-1/tiles/0/0/0",
        ] {
            let (status, _) = call(&app, "GET", bad, None).await;
            assert!(status.is_client_error(), "{bad} gave {status}");
        }

        // While a generation runs, tiles and hex grids are refused for later.
        let big = r#"{"planet": "barren", "seed": 9, "width": 512, "height": 256}"#;
        let (_, body) = call(&app, "POST", "/api/worlds", Some(big)).await;
        let big: serde_json::Value = serde_json::from_str(&body).unwrap();
        loop {
            let (_, list) = call(&app, "GET", "/api/jobs", None).await;
            if list.contains("\"status\":\"running\"") {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let request = Request::get("/api/worlds/frozen-3/tiles/1/0/0")
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert!(response.headers().contains_key(header::RETRY_AFTER));
        let request = Request::post("/api/worlds/frozen-3/hex")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"columns": 8}"#))
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert!(response.headers().contains_key(header::RETRY_AFTER));
        let job = big["job_id"].as_str().unwrap();
        call(&app, "DELETE", &format!("/api/jobs/{job}"), None).await;
        call(&app, "GET", &format!("/api/jobs/{job}/events"), None).await;
        let (status, _) = call(&app, "GET", "/api/worlds/frozen-3/tiles/1/0/0", None).await;
        assert_eq!(status, StatusCode::OK);
        // The server resamples without writing hex/ into the world.
        assert!(!config.worlds_dir.join("frozen-3/hex").exists());

        let (status, _) = call(&app, "DELETE", "/api/worlds/frozen-3", None).await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert!(!config.worlds_dir.join("frozen-3").exists());
    }

    /// Waits for the end of a job and returns its event stream.
    async fn wait_job(app: &Router, created: &str) -> String {
        let created: serde_json::Value = serde_json::from_str(created).unwrap();
        let uri = format!("/api/jobs/{}/events", created["job_id"].as_str().unwrap());
        call(app, "GET", &uri, None).await.1
    }

    async fn world_width(app: &Router, id: &str) -> Option<u64> {
        let (_, list) = call(app, "GET", "/api/worlds", None).await;
        let list: serde_json::Value = serde_json::from_str(&list).unwrap();
        list.as_array()
            .unwrap()
            .iter()
            .find(|w| w["id"] == id)
            .map(|w| w["width"].as_u64().unwrap())
    }

    #[tokio::test]
    async fn replaces_a_world_once_the_new_one_is_complete() {
        let config = temp_config("replace");
        let app = router(config.clone());
        let old = r#"{"planet": "frozen", "seed": 5, "width": 64, "height": 32}"#;
        let (status, body) = call(&app, "POST", "/api/worlds", Some(old)).await;
        assert_eq!(status, StatusCode::ACCEPTED, "{body}");
        assert!(wait_job(&app, &body).await.contains("\"status\":\"done\""));
        // Loaded for the tiles, so the cache holds the old revision.
        let (status, _) = call(&app, "GET", "/api/worlds/frozen-5/tiles/0/0/0", None).await;
        assert_eq!(status, StatusCode::OK);

        // Without the flag, the existing world is refused.
        let new = r#"{"planet": "frozen", "seed": 5, "width": 128, "height": 64}"#;
        let (status, _) = call(&app, "POST", "/api/worlds", Some(new)).await;
        assert_eq!(status, StatusCode::CONFLICT);
        let (status, _) = call(&app, "POST", "/api/worlds?replace=maybe", Some(new)).await;
        assert!(status.is_client_error());

        // Cancelled mid-generation: the old world stays, untouched.
        let slow = r#"{"planet": "frozen", "seed": 5, "width": 512, "height": 256}"#;
        let (status, body) = call(&app, "POST", "/api/worlds?replace=true", Some(slow)).await;
        assert_eq!(status, StatusCode::ACCEPTED, "{body}");
        // Being generated: neither deleted nor queued twice.
        let (status, _) = call(&app, "DELETE", "/api/worlds/frozen-5", None).await;
        assert_eq!(status, StatusCode::CONFLICT);
        let (status, _) = call(&app, "POST", "/api/worlds?replace=true", Some(new)).await;
        assert_eq!(status, StatusCode::CONFLICT);
        let created: serde_json::Value = serde_json::from_str(&body).unwrap();
        let job = created["job_id"].as_str().unwrap();
        loop {
            let (_, list) = call(&app, "GET", "/api/jobs", None).await;
            if list.contains("\"status\":\"running\"") {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert_eq!(
            world_width(&app, "frozen-5").await,
            Some(64),
            "kept while it runs"
        );
        call(&app, "DELETE", &format!("/api/jobs/{job}"), None).await;
        assert!(
            wait_job(&app, &body)
                .await
                .contains("\"status\":\"cancelled\"")
        );
        assert_eq!(world_width(&app, "frozen-5").await, Some(64));
        assert!(!config.worlds_dir.join(".tmp-frozen-5").exists());
        assert!(!config.worlds_dir.join(".old-frozen-5").exists());

        // Completed: the new world takes the place (and id) of the old one,
        // and its tiles come from the new revision.
        let (status, body) = call(&app, "POST", "/api/worlds?replace=true", Some(new)).await;
        assert_eq!(status, StatusCode::ACCEPTED, "{body}");
        assert!(wait_job(&app, &body).await.contains("\"status\":\"done\""));
        assert_eq!(world_width(&app, "frozen-5").await, Some(128));
        assert!(!config.worlds_dir.join(".old-frozen-5").exists());
        let (status, _) = call(&app, "GET", "/api/worlds/frozen-5/tiles/0/0/0", None).await;
        assert_eq!(status, StatusCode::OK);

        // Nothing to replace: the flag generates as usual.
        let other = r#"{"planet": "frozen", "seed": 6, "width": 64, "height": 32}"#;
        let (status, body) = call(&app, "POST", "/api/worlds?replace=true", Some(other)).await;
        assert_eq!(status, StatusCode::ACCEPTED, "{body}");
        assert!(wait_job(&app, &body).await.contains("\"status\":\"done\""));
        assert_eq!(world_width(&app, "frozen-6").await, Some(64));
    }

    #[tokio::test]
    async fn lists_and_cancels_jobs() {
        let config = temp_config("cancel");
        let app = router(config.clone());
        // The first job holds the queue; the second waits behind it.
        let first = r#"{"planet": "barren", "seed": 1, "width": 512, "height": 256}"#;
        let small = r#"{"planet": "barren", "seed": 2, "width": 64, "height": 32}"#;
        let (_, body) = call(&app, "POST", "/api/worlds", Some(first)).await;
        let first: serde_json::Value = serde_json::from_str(&body).unwrap();
        let (_, body) = call(&app, "POST", "/api/worlds", Some(small)).await;
        let second: serde_json::Value = serde_json::from_str(&body).unwrap();
        let second_id = second["job_id"].as_str().unwrap();

        // A world being generated cannot be deleted.
        let (status, _) = call(&app, "DELETE", "/api/worlds/barren-1", None).await;
        assert_eq!(status, StatusCode::CONFLICT);

        let (status, list) = call(&app, "GET", "/api/jobs", None).await;
        assert_eq!(status, StatusCode::OK);
        let list: serde_json::Value = serde_json::from_str(&list).unwrap();
        assert_eq!(list[0]["job_id"], second_id, "most recent first");
        assert_eq!(list[0]["world_id"], "barren-2");
        assert_eq!(list[0]["request"]["seed"], 2);

        let (status, _) = call(&app, "DELETE", &format!("/api/jobs/{second_id}"), None).await;
        assert_eq!(status, StatusCode::ACCEPTED);
        let uri = format!("/api/jobs/{second_id}/events");
        let (_, events) = call(&app, "GET", &uri, None).await;
        assert!(events.contains("\"status\":\"cancelled\""), "{events}");
        assert!(!config.worlds_dir.join("barren-2").exists());

        // A finished job cannot be cancelled; an unknown one does not exist.
        let (status, _) = call(&app, "DELETE", &format!("/api/jobs/{second_id}"), None).await;
        assert_eq!(status, StatusCode::CONFLICT);
        let (status, _) = call(&app, "DELETE", "/api/jobs/999", None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);

        // A running job stops when its next stage starts.
        let first_id = first["job_id"].as_str().unwrap();
        loop {
            let (_, list) = call(&app, "GET", "/api/jobs", None).await;
            if list.contains("\"status\":\"running\"") {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let (status, _) = call(&app, "DELETE", &format!("/api/jobs/{first_id}"), None).await;
        assert_eq!(status, StatusCode::ACCEPTED);
        let (_, events) = call(&app, "GET", &format!("/api/jobs/{first_id}/events"), None).await;
        assert!(events.contains("\"status\":\"cancelled\""), "{events}");
        assert!(!config.worlds_dir.join("barren-1").exists());

        // The queue is free again.
        let (status, _) = call(&app, "POST", "/api/worlds", Some(small)).await;
        assert_eq!(status, StatusCode::ACCEPTED);
    }
}
