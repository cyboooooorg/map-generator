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
//! | `PUT /api/worlds/{id}/names`    | name a world and its moons (`GivenNames` body)    |
//! | `POST /api/worlds/{id}/hex`     | resample a world onto a hex grid (`HexRequest`)   |
//! | `GET /api/worlds/{id}/tiles/{z}/{x}/{y}?rev=&epoch=` | deep-zoom detail tile (PNG), at an epoch in Ma when set |
//! | `GET /api/worlds/{id}/epoch/{ma}/{file}?rev=&scale=` | `surface.png`, `elevation.png` or `biomes.png` of a world at an epoch (see [`mapgen::epoch`]) |
//! | `GET /api/worlds/{id}/host/siblings/{n}/tiles/…`, `…/epoch/…` | the same for the other moons of a giant whose moon the world is |
//! | `GET /api/worlds/{id}/thumb?rev=` | thumbnail (PNG), made for older worlds (see [`thumbs`]) |
//! | `GET /api/systems`              | drawn systems, most recent first (`SystemSummary`) |
//! | `POST /api/systems?replace=`    | draw a system (`SystemRequest`); mode `around` queues its planet |
//! | `GET /api/systems/{id}`         | a system (`system.json`) and the names given       |
//! | `DELETE /api/systems/{id}`      | delete a system (409 while it is being generated)  |
//! | `PUT /api/systems/{id}/names`   | name its bodies, the star too (`SystemNames`)      |
//! | `POST /api/systems/{id}/bodies/{body}/generate` | queue a body and its neighbourhood (`GenerateStarted`) |
//! | `GET /api/systems/{id}/bodies/{body}/tiles/…`, `…/epoch/…`, `…/thumb`; `POST …/hex` | as for a world, per generated body |
//! | `GET /systems/{id}/...`         | system files (`system.json`, `bodies/<body>/…`)   |
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
    ApiError, CreateQuery, ErrorDetail, FormSpec, GenerateStarted, GivenNames, HexRequest,
    JobCreated, JobInfo, JobStatus, ParamSpec, PresetInfo, SystemCreated, SystemDetail,
    SystemNames, SystemQuery, SystemSummary, WorldSummary,
};
use crate::catalog::Catalog;
use crate::memory;
use crate::systems;
use crate::thumbs::{ThumbError, Thumbnails};
use crate::tiles::{self, TileCache, TileError};
use crate::worlds;
use axum::extract::{Path as UrlPath, Query, State};
use axum::http::{Extensions, HeaderMap, HeaderValue, StatusCode, Version, header};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use mapgen::detail::{RENDER_VERSION, TileId};
use mapgen::export::{ExportOptions, export_host, export_moons, export_world, world_dir_name};
use mapgen::generation::{GenerateOptions, Stage, generate_with_progress};
use mapgen::hex::{self, DEFAULT_RIVER_FLOW_KM2, HexMap};
use mapgen::request::{self, WorldRequest, fallback};
use std::collections::HashMap;
use std::convert::Infallible;
use std::path::{Path, PathBuf};
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
    /// A system's job: the system and the body asked for.
    system: Option<(String, String)>,
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
    /// Drawn systems (see [`systems`]).
    pub systems_dir: PathBuf,
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
    /// Epoch surfaces rendered at once: each one decodes its whole world
    /// and renders it at full size, outside the tile cache's budget.
    epoch_surfaces: Semaphore,
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
    systems::sweep(&config.systems_dir);
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
        epoch_surfaces: Semaphore::new(1),
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
        .nest_service("/systems", ServeDir::new(&config.systems_dir))
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
        .route("/api/worlds/{id}/names", put(rename_world))
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
        .route(
            "/api/worlds/{id}/host/siblings/{sibling}/tiles/{z}/{x}/{y}",
            get(sibling_tile),
        )
        .route(
            "/api/worlds/{id}/host/siblings/{sibling}/epoch/{epoch}/{file}",
            get(sibling_epoch_layer),
        )
        .route("/api/worlds/{id}/thumb", get(thumbnail))
        .route("/api/systems", get(list_systems).post(create_system))
        .route("/api/systems/{id}", get(get_system).delete(delete_system))
        .route("/api/systems/{id}/names", put(rename_system))
        .route(
            "/api/systems/{id}/bodies/{body}/generate",
            post(generate_body),
        )
        .route(
            "/api/systems/{id}/bodies/{body}/tiles/{z}/{x}/{y}",
            get(system_tile),
        )
        .route(
            "/api/systems/{id}/bodies/{body}/epoch/{epoch}/{file}",
            get(system_epoch_layer),
        )
        .route(
            "/api/systems/{id}/bodies/{body}/thumb",
            get(system_thumbnail),
        )
        .route("/api/systems/{id}/bodies/{body}/hex", post(system_hex))
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
        planet_types: mapgen::system::PLANET_TYPES
            .iter()
            .map(|t| (*t).to_owned())
            .collect(),
        max_planets: mapgen::system::MAX_PLANETS as u32,
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

/// A failure the client can fix.
fn bad(message: String) -> Failure {
    Failure(StatusCode::BAD_REQUEST, message)
}

/// The key a system takes in the list of pending generations (beside world
/// ids): a system cannot be deleted, renamed away or replaced meanwhile.
fn pending_system(id: &str) -> String {
    format!("systems/{id}")
}

async fn list_systems(State(state): State<Shared>) -> Json<Vec<SystemSummary>> {
    let root = state.config.systems_dir.clone();
    Json(
        tokio::task::spawn_blocking(move || systems::list(&root))
            .await
            .unwrap_or_default(),
    )
}

/// Draws a system and writes its `system.json` (`POST /api/systems`). A
/// system of the same id is refused with 409 unless the query replaces it.
async fn create_system(
    State(state): State<Shared>,
    Query(query): Query<SystemQuery>,
    Json(request): Json<mapgen::system::SystemRequest>,
) -> Result<(StatusCode, Json<SystemCreated>), Rejection> {
    check_system_request(&request)?;
    let drawn = request.clone();
    let system =
        tokio::task::spawn_blocking(move || mapgen::system::draw(&drawn, &mut rand::rng()))
            .await
            .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .map_err(|e| Rejection(bad(e.to_string()), ErrorDetail::of_request(&e)))?;
    let pixels = u64::from(system.width) * u64::from(system.height);
    if pixels > state.config.max_pixels {
        return Err(Rejection(
            bad(format!(
                "{} × {} is more than the {} pixels this server accepts",
                system.width, system.height, state.config.max_pixels
            )),
            Some(ErrorDetail::TooManyPixels {
                width: system.width,
                height: system.height,
                max_pixels: state.config.max_pixels,
            }),
        ));
    }
    let id = mapgen::system::system_id(&system);
    if !systems::is_valid_system_id(&id) {
        return Err(Failure(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("invalid system id {id}"),
        )
        .into());
    }
    let replace = query.replace.unwrap_or(false);
    if state
        .pending
        .lock()
        .expect("pending lock")
        .contains(&pending_system(&id))
    {
        return Err(Rejection(
            Failure(
                StatusCode::CONFLICT,
                format!("system {id} is being generated"),
            ),
            Some(ErrorDetail::BeingGenerated { world: id }),
        ));
    }
    let root = state.config.systems_dir.clone();
    let target = systems::system_dir(&root, &id);
    if !replace && target.exists() {
        return Err(Rejection(
            Failure(
                StatusCode::CONFLICT,
                format!("system {id} already exists: replace it, delete it or pick another seed"),
            ),
            Some(ErrorDetail::SystemExists { system: id }),
        ));
    }
    let written = id.clone();
    tokio::task::spawn_blocking(move || {
        let temp = crate::staging::TempDir::create(&root, &written)?;
        mapgen::system::write_system(temp.path(), &system)?;
        if replace {
            temp.commit_replacing(&target)
        } else {
            temp.commit(&target)
        }
    })
    .await
    .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    state.tiles.forget(&format!("systems/{id}"));
    let job_id = match &request {
        mapgen::system::SystemRequest::Around(_) => first_body_job(&state, &id).await?,
        mapgen::system::SystemRequest::System(_) => None,
    };
    Ok((
        StatusCode::CREATED,
        Json(SystemCreated {
            system_id: id,
            job_id,
        }),
    ))
}

/// Refuses what a system request may not ask of this server before the
/// draw: a map side beyond [`MAX_SIZE`], and, around a planet, a preset
/// that is not built in. The draw loads a preset by its spec, and a spec
/// ending in `.toml` is a path: only names, never a path chosen by the
/// client.
fn check_system_request(request: &mapgen::system::SystemRequest) -> Result<(), Rejection> {
    use mapgen::system::SystemRequest;
    let (width, height, planet) = match request {
        SystemRequest::System(whole) => (whole.width, whole.height, None),
        SystemRequest::Around(world) => (world.width, world.height, world.planet.as_ref()),
    };
    for (field, value) in [("width", width), ("height", height)] {
        if let Some(value) = value.filter(|&v| v > MAX_SIZE) {
            return Err(Rejection(
                bad(format!("{field} must be at most {MAX_SIZE}")),
                Some(ErrorDetail::SizeOutOfRange {
                    field: field.to_owned(),
                    value,
                    min: mapgen::request::MAP_SIZE.0,
                    max: MAX_SIZE,
                }),
            ));
        }
    }
    if let Some(spec) = planet
        && !mapgen::preset::built_in_names().contains(&spec.as_str())
    {
        return Err(Rejection(
            bad(format!("unknown planet preset {spec:?}")),
            Some(ErrorDetail::UnknownPreset {
                preset: spec.clone(),
            }),
        ));
    }
    Ok(())
}

/// Queues the body a system made around a planet was drawn for (its
/// `focus`: the planet asked, or the moon of a giant asked) and its
/// neighbourhood.
async fn first_body_job(state: &Shared, id: &str) -> Result<Option<String>, Rejection> {
    let root = state.config.systems_dir.clone();
    let owned = id.to_string();
    let focus = tokio::task::spawn_blocking(move || {
        mapgen::system::read_system(&systems::system_dir(&root, &owned)).map(|s| s.focus)
    })
    .await
    .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    match focus {
        Some(body) => queue_body(state, id, &body).await,
        None => Ok(None),
    }
}

async fn get_system(
    State(state): State<Shared>,
    UrlPath(id): UrlPath<String>,
) -> Result<Json<SystemDetail>, Failure> {
    if !systems::is_valid_system_id(&id) {
        return Err(Failure(StatusCode::BAD_REQUEST, "invalid system id".into()));
    }
    let root = state.config.systems_dir.clone();
    let owned = id.clone();
    tokio::task::spawn_blocking(move || {
        let system = mapgen::system::read_system(&systems::system_dir(&root, &owned)).ok()?;
        Some(SystemDetail {
            system,
            names: systems::given_names(&root, &owned),
        })
    })
    .await
    .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map(Json)
    .ok_or_else(|| Failure(StatusCode::NOT_FOUND, format!("no system {id}")))
}

async fn delete_system(
    State(state): State<Shared>,
    UrlPath(id): UrlPath<String>,
) -> Result<StatusCode, Failure> {
    if !systems::is_valid_system_id(&id) {
        return Err(Failure(StatusCode::BAD_REQUEST, "invalid system id".into()));
    }
    if state
        .pending
        .lock()
        .expect("pending lock")
        .contains(&pending_system(&id))
    {
        return Err(Failure(
            StatusCode::CONFLICT,
            format!("system {id} is being generated: cancel its job first"),
        ));
    }
    let dir = systems::system_dir(&state.config.systems_dir, &id);
    if !dir.is_dir() {
        return Err(Failure(StatusCode::NOT_FOUND, format!("no system {id}")));
    }
    let forgotten = state.tiles.forget(&format!("systems/{id}"));
    tokio::fs::remove_dir_all(&dir)
        .await
        .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if forgotten {
        let _ = tokio::task::spawn_blocking(memory::release).await;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Gives names to the bodies of a system (`PUT /api/systems/{id}/names`),
/// and answers its summary.
async fn rename_system(
    State(state): State<Shared>,
    UrlPath(id): UrlPath<String>,
    Json(request): Json<SystemNames>,
) -> Result<Json<SystemSummary>, Failure> {
    if !systems::is_valid_system_id(&id) {
        return Err(Failure(StatusCode::BAD_REQUEST, "invalid system id".into()));
    }
    let root = state.config.systems_dir.clone();
    let owned = id.clone();
    let result =
        tokio::task::spawn_blocking(move || systems::give_names(&root, &owned, &request.names))
            .await
            .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    result.map(Json).map_err(|e| match e {
        systems::NamingError::NotFound => Failure(StatusCode::NOT_FOUND, format!("no system {id}")),
        systems::NamingError::InvalidName(name) => Failure(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!(
                "invalid name {name:?}: 1 to {} characters, without control characters",
                systems::MAX_NAME_CHARS
            ),
        ),
        systems::NamingError::NoSuchBody(body) => Failure(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("system {id} has no body {body}"),
        ),
        systems::NamingError::Io(e) => Failure(StatusCode::INTERNAL_SERVER_ERROR, e),
    })
}

async fn system_tile(
    State(state): State<Shared>,
    UrlPath((id, body, z, x, y)): UrlPath<(String, String, u32, u32, u32)>,
    Query(query): Query<TileQuery>,
) -> Result<Response, Failure> {
    let loc = system_body_loc(&state, &id, &body).await?;
    body_tile(state, loc, TileId { z, x, y }, query).await
}

async fn system_epoch_layer(
    State(state): State<Shared>,
    UrlPath((id, body, epoch_ma, file)): UrlPath<(String, String, i32, String)>,
    Query(query): Query<EpochQuery>,
) -> Result<Response, Failure> {
    let loc = system_body_loc(&state, &id, &body).await?;
    body_epoch_layer(state, loc, epoch_ma, file, query).await
}

async fn system_thumbnail(
    State(state): State<Shared>,
    UrlPath((id, body)): UrlPath<(String, String)>,
    Query(query): Query<ThumbQuery>,
) -> Result<Response, Failure> {
    let loc = system_body_loc(&state, &id, &body).await?;
    body_thumbnail(state, loc, query).await
}

async fn system_hex(
    State(state): State<Shared>,
    UrlPath((id, body)): UrlPath<(String, String)>,
    Json(request): Json<HexRequest>,
) -> Result<Response, Failure> {
    let loc = system_body_loc(&state, &id, &body).await?;
    body_hex(state, loc, request).await
}

/// Queues the generation of a body and its neighbourhood
/// (`POST /api/systems/{id}/bodies/{body}/generate`).
async fn generate_body(
    State(state): State<Shared>,
    UrlPath((id, body)): UrlPath<(String, String)>,
) -> Result<Json<GenerateStarted>, Rejection> {
    if !systems::is_valid_system_id(&id) || !systems::is_valid_body_id(&body) {
        return Err(Failure(StatusCode::BAD_REQUEST, "invalid body".into()).into());
    }
    let job_id = queue_body(&state, &id, &body).await?;
    Ok(Json(GenerateStarted { job_id }))
}

/// Gives a world and its moons names (`PUT /api/worlds/{id}/names`), and
/// answers its summary.
async fn rename_world(
    State(state): State<Shared>,
    UrlPath(id): UrlPath<String>,
    Json(given): Json<GivenNames>,
) -> Result<Json<WorldSummary>, Failure> {
    if !worlds::is_valid_id(&id) {
        return Err(Failure(StatusCode::BAD_REQUEST, "invalid world id".into()));
    }
    if state.pending.lock().expect("pending lock").contains(&id) {
        return Err(Failure(
            StatusCode::CONFLICT,
            format!("world {id} is being generated: wait for its job"),
        ));
    }
    let dir = state.config.worlds_dir.clone();
    let world = id.clone();
    let result = tokio::task::spawn_blocking(move || worlds::give_names(&dir, &world, given))
        .await
        .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    result.map(Json).map_err(|e| match e {
        worlds::NamingError::NotFound => Failure(StatusCode::NOT_FOUND, format!("no world {id}")),
        worlds::NamingError::InvalidName(name) => Failure(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!(
                "invalid name {name:?}: 1 to {} characters, without control characters",
                worlds::MAX_NAME_CHARS
            ),
        ),
        worlds::NamingError::NoSuchMoon(n) => Failure(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("world {id} has no moon {n}"),
        ),
        worlds::NamingError::Io(e) => Failure(StatusCode::INTERNAL_SERVER_ERROR, e),
    })
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
    dir: &Path,
    id: &str,
    epoch_ma: Option<i32>,
) -> Result<Option<i32>, Failure> {
    let Some(epoch_ma) = epoch_ma.filter(|&e| e != 0) else {
        return Ok(None);
    };
    let owned = dir.to_path_buf();
    let epochs = tokio::task::spawn_blocking(move || worlds::epochs_at(&owned))
        .await
        .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| Failure(StatusCode::NOT_FOUND, format!("no world {id}")))?;
    let Some([future, past]) = epochs else {
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

/// Where a body's files are, the key of its world in the tile cache, and
/// its revision (which keys its cached tiles in browsers).
struct BodyLoc {
    dir: PathBuf,
    key: String,
    revision: u64,
}

/// A world, or a moon of a world (`<id>/moons/<n>`), in `worlds/`: it takes
/// its world's revision, as it is generated with it.
async fn world_loc(state: &AppState, id: String) -> Result<BodyLoc, Failure> {
    let revision = world_revision(state, world_of(&id)).await?;
    Ok(BodyLoc {
        dir: state.config.worlds_dir.join(&id),
        key: id,
        revision,
    })
}

/// A generated body of a system, in `systems/<id>/bodies/<body>/`: its own
/// revision, so that generating another body keeps its tiles.
async fn system_body_loc(state: &AppState, id: &str, body: &str) -> Result<BodyLoc, Failure> {
    if !systems::is_valid_system_id(id) || !systems::is_valid_body_id(body) {
        return Err(Failure(StatusCode::BAD_REQUEST, "invalid body".into()));
    }
    let root = state.config.systems_dir.clone();
    let (system, owned) = (id.to_string(), body.to_string());
    let revision =
        tokio::task::spawn_blocking(move || systems::body_revision(&root, &system, &owned))
            .await
            .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or_else(|| {
                Failure(
                    StatusCode::NOT_FOUND,
                    format!("body {body} of {id} is not generated"),
                )
            })?;
    Ok(BodyLoc {
        dir: systems::body_dir(&state.config.systems_dir, id, body),
        key: format!("systems/{id}/{body}"),
        revision,
    })
}

async fn tile(
    State(state): State<Shared>,
    UrlPath((id, z, x, y)): UrlPath<(String, u32, u32, u32)>,
    Query(query): Query<TileQuery>,
) -> Result<Response, Failure> {
    if !worlds::is_valid_id(&id) {
        return Err(Failure(StatusCode::BAD_REQUEST, "invalid tile".into()));
    }
    body_tile(
        state.clone(),
        world_loc(&state, id).await?,
        TileId { z, x, y },
        query,
    )
    .await
}

/// A detail tile of moon `moon` (from 1) of a world, as [`tile`].
async fn moon_tile(
    State(state): State<Shared>,
    UrlPath((id, moon, z, x, y)): UrlPath<(String, u32, u32, u32, u32)>,
    Query(query): Query<TileQuery>,
) -> Result<Response, Failure> {
    let body = moon_body(&id, moon)?;
    body_tile(
        state.clone(),
        world_loc(&state, body).await?,
        TileId { z, x, y },
        query,
    )
    .await
}

/// The body path of moon `moon` (from 1) of world `id`, as the world
/// directory holds it: `<id>/moons/<moon>`.
fn moon_body(id: &str, moon: u32) -> Result<String, Failure> {
    if !worlds::is_valid_id(id) || !(1..=MAX_MOONS).contains(&moon) {
        return Err(Failure(StatusCode::BAD_REQUEST, "invalid moon".into()));
    }
    Ok(format!("{id}/moons/{moon}"))
}

/// A detail tile of sibling `sibling` (from 1) of a giant's moon world:
/// another moon of its giant, as [`tile`].
async fn sibling_tile(
    State(state): State<Shared>,
    UrlPath((id, sibling, z, x, y)): UrlPath<(String, u32, u32, u32, u32)>,
    Query(query): Query<TileQuery>,
) -> Result<Response, Failure> {
    let body = sibling_body(&id, sibling)?;
    body_tile(
        state.clone(),
        world_loc(&state, body).await?,
        TileId { z, x, y },
        query,
    )
    .await
}

/// A layer of sibling `sibling` (from 1) of a world at an epoch, as
/// [`epoch_layer`].
async fn sibling_epoch_layer(
    State(state): State<Shared>,
    UrlPath((id, sibling, epoch_ma, file)): UrlPath<(String, u32, i32, String)>,
    Query(query): Query<EpochQuery>,
) -> Result<Response, Failure> {
    let body = sibling_body(&id, sibling)?;
    body_epoch_layer(
        state.clone(),
        world_loc(&state, body).await?,
        epoch_ma,
        file,
        query,
    )
    .await
}

/// The body path of sibling `sibling` (from 1) of world `id`, the moon of
/// a giant: `<id>/host/siblings/<sibling>` (see `mapgen::export::export_host`).
fn sibling_body(id: &str, sibling: u32) -> Result<String, Failure> {
    if !worlds::is_valid_id(id) || !(1..=MAX_MOONS).contains(&sibling) {
        return Err(Failure(StatusCode::BAD_REQUEST, "invalid sibling".into()));
    }
    Ok(format!("{id}/host/siblings/{sibling}"))
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
    loc: BodyLoc,
    tile: TileId,
    query: TileQuery,
) -> Result<Response, Failure> {
    if !tile.is_valid() {
        return Err(Failure(StatusCode::BAD_REQUEST, "invalid tile".into()));
    }
    let BodyLoc {
        dir,
        key: id,
        revision,
    } = loc;
    let epoch = checked_epoch(&dir, &id, query.epoch).await?;
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
    body_epoch_layer(
        state.clone(),
        world_loc(&state, id).await?,
        epoch_ma,
        file,
        query,
    )
    .await
}

/// A layer of moon `moon` (from 1) of a world at an epoch, as
/// [`epoch_layer`].
async fn moon_epoch_layer(
    State(state): State<Shared>,
    UrlPath((id, moon, epoch_ma, file)): UrlPath<(String, u32, i32, String)>,
    Query(query): Query<EpochQuery>,
) -> Result<Response, Failure> {
    let body = moon_body(&id, moon)?;
    body_epoch_layer(
        state.clone(),
        world_loc(&state, body).await?,
        epoch_ma,
        file,
        query,
    )
    .await
}

/// A layer of a body (see [`body_tile`]) at an epoch.
async fn body_epoch_layer(
    state: Shared,
    loc: BodyLoc,
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
    let BodyLoc {
        dir,
        key: id,
        revision,
    } = loc;
    let epoch = checked_epoch(&dir, &id, Some(epoch_ma)).await?;
    let world = match state.tiles.world_at(&dir, &id, revision, epoch).await {
        Ok(world) => world,
        Err(e) => return Ok(world_failure(e, "epochs")),
    };
    // The surface decodes the whole world and renders it at full size,
    // outside the tile cache's budget (several GB at 8192 x 4096 if four
    // ran together): one at a time. The other layers read the cached world.
    let semaphore = if layer == tiles::EpochLayer::Surface {
        &state.epoch_surfaces
    } else {
        &state.renders
    };
    let _turn = semaphore
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
    let loc = world_loc(&state, id).await?;
    body_thumbnail(state, loc, query).await
}

/// The thumbnail of a body (see [`thumbnail`]).
async fn body_thumbnail(
    state: Shared,
    loc: BodyLoc,
    query: ThumbQuery,
) -> Result<Response, Failure> {
    let BodyLoc {
        dir,
        key: id,
        revision,
    } = loc;
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
    let loc = world_loc(&state, id).await?;
    body_hex(state, loc, request).await
}

/// The hex grid of a body (see [`hex_grid`]).
async fn body_hex(state: Shared, loc: BodyLoc, request: HexRequest) -> Result<Response, Failure> {
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
    let BodyLoc {
        dir,
        key: id,
        revision,
    } = loc;
    let river_flow = request.river_flow_km2.unwrap_or(DEFAULT_RIVER_FLOW_KM2);
    let _turn = state
        .hex_grids
        .acquire()
        .await
        .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
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
/// end (see [`crate::staging::TempDir::commit_replacing`]), so a generation that
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
    let ignored: Vec<String> = resolved.ignored.iter().map(|f| (*f).to_owned()).collect();
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
                system: None,
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
                progress.send_replace(JobStatus::Running {
                    stage,
                    step,
                    steps,
                    body: None,
                    body_step: None,
                    bodies: None,
                });
            };
            // The web app does not show the diagnostic maps: skip them (no
            // debug sink), and the time they take.
            let generated = generate_with_progress(&params, GenerateOptions::default(), report);
            report(Stage::Export);
            // Written into a temporary directory, renamed into place once
            // complete: a cancelled, failed or crashed export leaves nothing.
            let id = world_dir_name(&generated.world.params);
            let temp = crate::staging::TempDir::create(&worlds_dir, &id)?;
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
                    bodies: Vec::new(),
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

    Ok((
        StatusCode::ACCEPTED,
        Json(JobCreated {
            job_id,
            world_id,
            ignored,
        }),
    ))
}

/// Queues the generation of `body` of system `id` and its neighbourhood
/// (see `mapgen::system::neighbourhood`), and answers the job: none when
/// they are all generated, the job already queued when the system has one
/// (a system generates one job at a time).
async fn queue_body(state: &Shared, id: &str, body: &str) -> Result<Option<String>, Rejection> {
    let dir = systems::system_dir(&state.config.systems_dir, id);
    let read = dir.clone();
    let system = tokio::task::spawn_blocking(move || mapgen::system::read_system(&read))
        .await
        .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map_err(|_| Failure(StatusCode::NOT_FOUND, format!("no system {id}")))?;
    if !system.bodies.iter().any(|b| b.id == body) {
        return Err(Failure(
            StatusCode::NOT_FOUND,
            format!("system {id} has no body {body}"),
        )
        .into());
    }
    let key = pending_system(id);
    {
        let mut pending = state.pending.lock().expect("pending lock");
        if pending.contains(&key) {
            let jobs = state.jobs.lock().expect("jobs lock");
            let queued = jobs.iter().find(|(_, job)| {
                job.system.as_ref().is_some_and(|(s, _)| s == id)
                    && !job.status.borrow().is_finished()
            });
            return Ok(queued.map(|(job_id, _)| job_id.clone()));
        }
        if mapgen::system::neighbourhood(&system, body).is_empty() {
            return Ok(None);
        }
        pending.push(key.clone());
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
                world_id: id.to_string(),
                system: Some((id.to_string(), body.to_string())),
                request: WorldRequest::default(),
                status: rx,
                cancel: cancel.clone(),
            },
        );
    }

    let task_state = state.clone();
    let (task_system, task_body) = (id.to_string(), body.to_string());
    tokio::spawn(async move {
        let finish = |status: JobStatus| {
            if let JobStatus::Failed { error } = &status {
                tracing::error!("generation of {task_system}/{task_body} failed: {error}");
            }
            tx.send_replace(status);
            task_state
                .pending
                .lock()
                .expect("pending lock")
                .retain(|k| k != &key);
        };
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
        let paused_tiles = task_state.tiles.suspend().await;
        let progress = tx.clone();
        let thread_cancel = cancel.clone();
        let body = task_body.clone();
        let result = tokio::task::spawn_blocking(move || {
            mapgen::system::generate_neighbourhood(&dir, &body, |p| {
                // A cancelled job stops when the next stage starts; the
                // bodies done before it stay generated.
                if thread_cancel.is_requested() {
                    std::panic::resume_unwind(Box::new(Cancelled));
                }
                progress.send_replace(JobStatus::Running {
                    stage: p.stage,
                    step: p.step,
                    steps: p.steps,
                    body: Some(p.body),
                    body_step: Some(p.body_step),
                    bodies: Some(p.bodies),
                });
            })
        })
        .await;
        let status = match result {
            Ok(Ok(bodies)) => JobStatus::Done {
                world_id: task_system.clone(),
                bodies,
            },
            Ok(Err(e)) => JobStatus::Failed { error: e },
            Err(e) if e.is_panic() && cancel.is_requested() => JobStatus::Cancelled,
            Err(e) => JobStatus::Failed {
                error: format!("generation crashed: {e}"),
            },
        };
        let _ = tokio::task::spawn_blocking(memory::release).await;
        drop(paused_tiles);
        finish(status);
    });
    Ok(Some(job_id))
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
                    system_id: job.system.as_ref().map(|(s, _)| s.clone()),
                    body: job.system.as_ref().map(|(_, b)| b.clone()),
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
        assert_eq!(
            sibling_body("terran-5", 2).ok().as_deref(),
            Some("terran-5/host/siblings/2")
        );
        assert!(sibling_body("terran-5", 0).is_err());
        assert_eq!(world_of("terran-5/host/siblings/2"), "terran-5");
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
            systems_dir: root.join("systems"),
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

    #[tokio::test]
    async fn names_a_world_and_its_moons() {
        let config = temp_config("names");
        let world = config.worlds_dir.join("terran-42");
        std::fs::create_dir_all(world.join("raw_data")).unwrap();
        std::fs::write(
            world.join("raw_data/world.json"),
            r#"{"planet_type":"terran","seed":42,"format_version":10,
                "star":{"type":"G2V","orbit_au":1.0},
                "giants":{"giants":[{"orbit_au":5.2},{"orbit_au":0.1}]},
                "moons":{"moons":[{},{}]}}"#,
        )
        .unwrap();
        let app = router(config);
        let star = mapgen::naming::star_name(42, "G2V");

        let (_, body) = call(&app, "GET", "/api/worlds", None).await;
        let listed: serde_json::Value = serde_json::from_str(&body).unwrap();
        let names = &listed[0]["names"];
        assert_eq!(names["designation"], format!("{star} c"));
        assert_eq!(
            names["giants"],
            serde_json::json!([format!("{star} d"), format!("{star} b")])
        );
        assert_eq!(names["moons"][1]["designation"], format!("{star} c II"));
        assert!(names["name"].is_null());

        let (status, body) = call(
            &app,
            "PUT",
            "/api/worlds/terran-42/names",
            Some(r#"{"name":"  Gaia ","moons":{"2":"Séléné"}}"#),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let summary: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(summary["names"]["name"], "Gaia");
        assert_eq!(summary["names"]["moons"][1]["name"], "Séléné");
        assert!(summary["names"]["moons"][0]["name"].is_null());
        assert!(world.join("names.json").exists());

        for bad in [
            r#"{"name":"   "}"#,
            r#"{"moons":{"3":"Nyx"}}"#,
            r#"{"moons":{"01":"Nyx"}}"#,
        ] {
            let (status, body) = call(&app, "PUT", "/api/worlds/terran-42/names", Some(bad)).await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{bad}: {body}");
        }
        let (status, _) = call(&app, "PUT", "/api/worlds/terran-7/names", Some("{}")).await;
        assert_eq!(status, StatusCode::NOT_FOUND);

        // No names at all: back to the designations, and no file.
        let (status, body) = call(&app, "PUT", "/api/worlds/terran-42/names", Some("{}")).await;
        assert_eq!(status, StatusCode::OK);
        let summary: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert!(summary["names"]["name"].is_null());
        assert!(!world.join("names.json").exists());
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
    #[tokio::test]
    async fn an_epoch_of_a_world_is_a_world_of_its_own() {
        let app = router(temp_config("drift"));
        let request = |drift: &str| {
            format!(r#"{{"planet":"barren","seed":7,"width":64,"height":32{drift}}}"#)
        };
        let (status, present) = call(&app, "POST", "/api/worlds", Some(&request(""))).await;
        assert_eq!(status, StatusCode::ACCEPTED, "{present}");
        wait_job(&app, &present).await;
        let (status, past) = call(
            &app,
            "POST",
            "/api/worlds",
            Some(&request(r#","drift_ma":-40"#)),
        )
        .await;
        // Not a conflict with the present: an id of its own, listed.
        assert_eq!(status, StatusCode::ACCEPTED, "{past}");
        let created: serde_json::Value = serde_json::from_str(&past).unwrap();
        assert_eq!(created["world_id"], "barren-past40ma-7");
        wait_job(&app, &past).await;
        assert_eq!(world_width(&app, "barren-past40ma-7").await, Some(64));
        assert_eq!(world_width(&app, "barren-7").await, Some(64));
    }

    #[tokio::test]
    async fn systems_are_drawn_then_generated_body_by_body() {
        let config = temp_config("systems");
        let root = config.systems_dir.clone();
        let app = router(config);
        let request = r#"{"system": {"star": "K1V", "planets": {"barren": 2}, "seed": 3, "width": 64, "height": 32}}"#;
        let (status, body) = call(&app, "POST", "/api/systems", Some(request)).await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        let created: serde_json::Value = serde_json::from_str(&body).unwrap();
        let id = created["system_id"].as_str().unwrap().to_string();
        // A whole system generates nothing until a body is opened.
        assert!(created["job_id"].is_null());
        assert!(root.join(&id).join("system.json").exists());
        // Drawn again: 409, unless replaced.
        let (status, body) = call(&app, "POST", "/api/systems", Some(request)).await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&body).unwrap()["detail"]["code"],
            "system_exists"
        );
        let (status, _) = call(&app, "POST", "/api/systems?replace=true", Some(request)).await;
        assert_eq!(status, StatusCode::CREATED);

        let (_, list) = call(&app, "GET", "/api/systems", None).await;
        let list: serde_json::Value = serde_json::from_str(&list).unwrap();
        assert_eq!(list[0]["id"], id.as_str());
        assert_eq!(list[0]["planets"]["barren"], 2);
        let (status, detail) = call(&app, "GET", &format!("/api/systems/{id}"), None).await;
        assert_eq!(status, StatusCode::OK);
        let detail: serde_json::Value = serde_json::from_str(&detail).unwrap();
        let planet = detail["system"]["bodies"][0]["id"]
            .as_str()
            .unwrap()
            .to_string();

        // Not generated: no tiles yet.
        let tile = format!("/api/systems/{id}/bodies/{planet}/tiles/0/0/0");
        assert_eq!(
            call(&app, "GET", &tile, None).await.0,
            StatusCode::NOT_FOUND
        );
        let (status, started) = call(
            &app,
            "POST",
            &format!("/api/systems/{id}/bodies/{planet}/generate"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{started}");
        let events = wait_job(&app, &started).await;
        assert!(
            events.contains(&format!("\"body\":\"{planet}\"")),
            "{events}"
        );
        assert!(events.contains("\"status\":\"done\""), "{events}");
        assert!(
            root.join(&id)
                .join("bodies")
                .join(&planet)
                .join("raw_data/world.json")
                .exists()
        );
        // Generated: nothing left to do, and its files and tiles are served.
        let (_, again) = call(
            &app,
            "POST",
            &format!("/api/systems/{id}/bodies/{planet}/generate"),
            None,
        )
        .await;
        assert!(serde_json::from_str::<serde_json::Value>(&again).unwrap()["job_id"].is_null());
        let (_, list) = call(&app, "GET", "/api/systems", None).await;
        let list: serde_json::Value = serde_json::from_str(&list).unwrap();
        assert_eq!(list[0]["bodies"][0]["generated"], true);
        let file = get_gzip(
            &app,
            &format!("/systems/{id}/bodies/{planet}/raw_data/world.json"),
        )
        .await;
        assert_eq!(file.status(), StatusCode::OK);
        let (status, _) = call(
            &app,
            "GET",
            &format!("/api/systems/{id}/bodies/{planet}/thumb"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        // Names by body id; an unknown body is refused.
        let names = format!(r#"{{"names": {{"{planet}": "Gaia", "star": "Helia"}}}}"#);
        let (status, renamed) = call(
            &app,
            "PUT",
            &format!("/api/systems/{id}/names"),
            Some(&names),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{renamed}");
        let (status, _) = call(
            &app,
            "PUT",
            &format!("/api/systems/{id}/names"),
            Some(r#"{"names": {"z": "Nyx"}}"#),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        // Bad ids never reach the disk.
        assert_eq!(
            call(&app, "GET", "/api/systems/..%2Fetc-1", None).await.0,
            StatusCode::BAD_REQUEST
        );
        let bad = format!("/api/systems/{id}/bodies/..%2Fx/tiles/0/0/0");
        assert_eq!(
            call(&app, "GET", &bad, None).await.0,
            StatusCode::BAD_REQUEST
        );

        let (status, _) = call(&app, "DELETE", &format!("/api/systems/{id}"), None).await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert!(!root.join(&id).exists());
    }

    #[tokio::test]
    async fn system_requests_never_name_a_file() {
        let app = router(temp_config("system-paths"));
        for body in [
            r#"{"around": {"planet": "/etc/passwd.toml"}}"#,
            r#"{"around": {"planet": "../presets/terran.toml"}}"#,
            r#"{"around": {"planet": "nowhere"}}"#,
            r#"{"system": {"star": null, "metallicity": null, "planets": {"/etc/x.toml": 1}, "seed": 1, "width": 64, "height": 32}}"#,
        ] {
            let (status, answer) = call(&app, "POST", "/api/systems", Some(body)).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{body}: {answer}");
        }
        let (status, answer) = call(
            &app,
            "POST",
            "/api/systems",
            Some(r#"{"system": {"star": null, "metallicity": null, "planets": null, "seed": 1, "width": 100000, "height": 32}}"#),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{answer}");
        assert!(answer.contains("size_out_of_range"), "{answer}");
    }

    #[tokio::test]
    async fn a_system_round_a_planet_generates_that_planet_first() {
        let app = router(temp_config("around"));
        let request = r#"{"around": {"planet": "barren", "seed": 5, "width": 64, "height": 32, "moons": "none", "giants": "none"}}"#;
        let (status, body) = call(&app, "POST", "/api/systems", Some(request)).await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        let created: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert!(created["job_id"].is_string(), "{body}");
        let events = wait_job(&app, &body).await;
        assert!(events.contains("\"status\":\"done\""), "{events}");
    }

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
