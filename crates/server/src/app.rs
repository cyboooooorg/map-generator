//! HTTP routes and shared state.
//!
//! | Route                           | Purpose                                           |
//! |---------------------------------|---------------------------------------------------|
//! | `GET /healthz`                  | liveness probe                                    |
//! | `GET /api/presets`              | planet presets the form can offer                 |
//! | `GET /api/form`                 | parameter limits and defaults                     |
//! | `GET /api/systems`              | drawn systems, most recent first (`SystemSummary`) |
//! | `POST /api/systems?replace=`    | draw a system (`SystemRequest`); mode `around` queues its planet |
//! | `GET /api/systems/{id}`         | a system (`system.json`) and the names given       |
//! | `DELETE /api/systems/{id}`      | delete a system (409 while it is being generated)  |
//! | `PUT /api/systems/{id}/names`   | name its bodies, the star too (`SystemNames`)      |
//! | `PUT /api/systems/{id}/pin`     | pin it in the main menu (idempotent, answers its summary) |
//! | `DELETE /api/systems/{id}/pin`  | unpin it (idempotent, answers its summary)         |
//! | `POST /api/systems/{id}/bodies/{body}/generate` | queue a body and its neighbourhood (`GenerateStarted`) |
//! | `GET /api/systems/{id}/bodies/{body}/tiles/…`, `…/epoch/…`, `…/thumb`; `POST …/hex` | as for a world, per generated body |
//! | `GET /api/systems/{id}/bodies/{body}/drift/{ma}/surface.png` | low-resolution preview of a world with plates at another epoch of its plates |
//! | `GET /systems/{id}/...`         | system files (`system.json`, `bodies/<body>/…`)   |
//! | `GET /api/jobs`                 | recent generation jobs, most recent first         |
//! | `DELETE /api/jobs/{id}`         | cancel a queued or running generation             |
//! | `GET /api/jobs/{id}/events`     | job progress as server-sent events (`JobStatus`)  |
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
    ApiError, ErrorDetail, FormSpec, GenerateStarted, HexRequest, JobInfo, JobStatus, ParamSpec,
    PresetInfo, SystemCreated, SystemDetail, SystemNames, SystemQuery, SystemSummary,
};
use crate::catalog::Catalog;
use crate::index::{Cause, Index};
use crate::memory;
use crate::previews::{PreviewCache, PreviewKey};
use crate::systems;
use crate::thumbs::{ThumbError, Thumbnails};
use crate::tiles::{self, TileCache, TileError};
use axum::extract::{Path as UrlPath, Query, State};
use axum::http::{Extensions, HeaderMap, HeaderValue, StatusCode, Version, header};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use mapgen::detail::{RENDER_VERSION, TileId};
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
    /// Drawn systems (see [`systems`]).
    pub systems_dir: PathBuf,
    /// The SQLite file of the index (see [`crate::index`]); created, with
    /// its directory, when missing.
    pub db_path: PathBuf,
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
    /// Systems, bodies, names and jobs, beside the tree of files.
    index: Arc<Index>,
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
    /// Drift previews generated at once: each one runs a whole (small)
    /// generation on every core.
    drift_previews: Semaphore,
    drift_cache: PreviewCache,
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
    systems::sweep(&config.systems_dir);
    let catalog = Catalog::load(&config.presets_dir);
    let (index, jobs, next_job) = open_index(&config);
    let state = Arc::new(AppState {
        catalog,
        index,
        jobs: Mutex::new(jobs),
        next_job: AtomicU64::new(next_job),
        queue: Semaphore::new(1),
        pending: Mutex::new(Vec::new()),
        tiles: TileCache::new(config.tile_cache_bytes),
        renders: Semaphore::new(CONCURRENT_RENDERS),
        hex_grids: Semaphore::new(1),
        epoch_surfaces: Semaphore::new(1),
        drift_previews: Semaphore::new(1),
        drift_cache: PreviewCache::default(),
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
        .nest_service("/systems", ServeDir::new(&config.systems_dir))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            note_served,
        ))
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
        .route("/api/systems", get(list_systems).post(create_system))
        .route("/api/systems/{id}", get(get_system).delete(delete_system))
        .route("/api/systems/{id}/names", put(rename_system))
        .route(
            "/api/systems/{id}/pin",
            put(pin_system).delete(unpin_system),
        )
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
            "/api/systems/{id}/bodies/{body}/drift/{ma}/{file}",
            get(system_drift_preview),
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

/// Opens the index, fills it from the tree (all of it when the database
/// was missing or empty, the difference otherwise), marks the jobs a
/// stop left unfinished as failed, and brings back the job history.
fn open_index(config: &Config) -> (Arc<Index>, HashMap<String, Job>, u64) {
    let started = std::time::Instant::now();
    let index = Index::open(&config.db_path)
        .unwrap_or_else(|e| panic!("cannot open the index {}: {e}", config.db_path.display()));
    let report = index
        .reconcile(&config.systems_dir)
        .unwrap_or_else(|e| panic!("cannot read the systems into the index: {e}"));
    let interrupted = index.interrupt_unfinished().unwrap_or_else(|e| {
        tracing::error!("{e}");
        0
    });
    let stored = index.jobs().unwrap_or_else(|e| {
        tracing::error!("{e}");
        Vec::new()
    });
    let next_job = index.next_job_id().unwrap_or(1);
    tracing::info!(
        added = report.added,
        updated = report.updated,
        removed = report.removed,
        interrupted_jobs = interrupted,
        journal = index.journal_mode(),
        elapsed_ms = started.elapsed().as_millis() as u64,
        "index {} ready",
        config.db_path.display()
    );
    // Jobs of an earlier run only show their final status: the sender is
    // dropped, the receiver keeps the last value.
    let jobs = stored
        .into_iter()
        .map(|job| {
            let (_, status) = watch::channel(job.status);
            (
                job.id.to_string(),
                Job {
                    world_id: job.world_id,
                    system: job.system,
                    request: job.request,
                    status,
                    cancel: Arc::new(Cancel::default()),
                },
            )
        })
        .collect();
    (Arc::new(index), jobs, next_job)
}

/// Records, without waiting for it, that a body's data was served or
/// opened (at most once a minute per body, see [`Index::touch_due`]).
fn note_opened(state: &Shared, system: &str, body: &str) {
    if state.index.touch_due(system, body) {
        let index = state.index.clone();
        let (system, body) = (system.to_string(), body.to_string());
        drop(tokio::task::spawn_blocking(move || {
            if let Err(e) = index.write_opened(&system, &body, crate::index::now_ms()) {
                tracing::warn!("last opened of {system}/{body} not recorded: {e}");
            }
        }));
    }
}

/// Records, from a request for a file under `/systems/<id>/bodies/<body>/`
/// (thumbnails excepted: the library asks for all of them), that the body
/// was opened.
async fn note_served(
    State(state): State<Shared>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let path = request.uri().path().to_string();
    let parts: Vec<&str> = path.split('/').skip(1).collect();
    if let ["systems", id, "bodies", body, file, ..] = parts.as_slice()
        && *file != "thumb.png"
        && systems::is_valid_system_id(id)
        && systems::is_valid_body_id(body)
    {
        note_opened(&state, id, body);
    }
    next.run(request).await
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
    let index = state.index.clone();
    Json(
        tokio::task::spawn_blocking(move || index.list_systems())
            .await
            .map_err(|e| e.to_string())
            .and_then(|r| r.map_err(|e| e.to_string()))
            .unwrap_or_else(|e| {
                tracing::error!("listing systems: {e}");
                Vec::new()
            }),
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
    // Around a planet: its preset from the catalog only (never a path
    // chosen by the client), its parameters resolved here, the system drawn
    // around them.
    let preset = match &request {
        mapgen::system::SystemRequest::Around(world) => match &world.planet {
            Some(spec) => Some((
                state.catalog.resolve(spec).ok_or_else(|| {
                    Rejection(
                        bad(format!("unknown planet preset {spec:?}")),
                        Some(ErrorDetail::UnknownPreset {
                            preset: spec.clone(),
                        }),
                    )
                })?,
                spec.clone(),
            )),
            None => None,
        },
        mapgen::system::SystemRequest::System(_) => None,
    };
    let drawn = request.clone();
    let (system, ignored) = tokio::task::spawn_blocking(move || {
        let rng = &mut rand::rng();
        match &drawn {
            mapgen::system::SystemRequest::Around(world) => {
                let resolved = match preset {
                    Some((preset, spec)) => world.resolve_with_preset(preset, spec, rng)?,
                    None => world.resolve(rng)?,
                };
                let ignored = resolved.ignored.iter().map(|f| (*f).to_owned()).collect();
                Ok((
                    mapgen::system::draw_around_params(world, resolved.params),
                    ignored,
                ))
            }
            mapgen::system::SystemRequest::System(_) => {
                mapgen::system::draw(&drawn, rng).map(|s| (s, Vec::new()))
            }
        }
    })
    .await
    .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e: mapgen::request::RequestError| {
        Rejection(bad(e.to_string()), ErrorDetail::of_request(&e))
    })?;
    let pixels = u64::from(system.width) * u64::from(system.height);
    if pixels > state.config.max_pixels {
        return Err(Rejection(
            bad(format!(
                "{} × {} needs about {:.1} GB of memory; this server accepts up to {} pixels \
                 (MAPGEN_MAX_PIXELS)",
                system.width,
                system.height,
                (pixels * BYTES_PER_PIXEL) as f64 / 1e9,
                state.config.max_pixels
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
    let index = state.index.clone();
    tokio::task::spawn_blocking(move || {
        // A system replaced keeps the names that still fit one of its bodies.
        let kept: systems::GivenNames = if replace {
            index
                .names(&written)
                .map_err(|e| e.to_string())?
                .into_iter()
                .filter(|(body, _)| {
                    body == systems::STAR || system.bodies.iter().any(|b| &b.id == body)
                })
                .collect()
        } else {
            systems::GivenNames::new()
        };
        let temp = crate::staging::TempDir::create(&root, &written)?;
        mapgen::system::write_system(temp.path(), &system)?;
        if replace {
            temp.commit_replacing(&target)?;
        } else {
            temp.commit(&target)?;
        }
        if !kept.is_empty() {
            systems::write_names_file(&root, &written, &kept).map_err(|e| format!("{e:?}"))?;
        }
        index
            .refresh_system(&root, &written, Cause::Drawn)
            .map(drop)
            .map_err(|e| e.to_string())
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
            ignored,
        }),
    ))
}

/// Refuses a map side beyond [`MAX_SIZE`] before the draw (the preset of a
/// planet asked for is checked against the catalog by [`create_system`]).
fn check_system_request(request: &mapgen::system::SystemRequest) -> Result<(), Rejection> {
    use mapgen::system::SystemRequest;
    let (width, height) = match request {
        SystemRequest::System(whole) => (whole.width, whole.height),
        SystemRequest::Around(world) => (world.width, world.height),
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
    let index = state.index.clone();
    tokio::task::spawn_blocking(move || {
        let system = mapgen::system::read_system(&systems::system_dir(&root, &owned)).ok()?;
        Some(SystemDetail {
            system,
            names: index.names(&owned).unwrap_or_default(),
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
    let (index, owned) = (state.index.clone(), id.clone());
    tokio::task::spawn_blocking(move || index.remove_system(&owned))
        .await
        .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if forgotten {
        let _ = tokio::task::spawn_blocking(memory::release).await;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Pins a system in the main menu (`PUT /api/systems/{id}/pin`).
async fn pin_system(
    state: State<Shared>,
    id: UrlPath<String>,
) -> Result<Json<SystemSummary>, Failure> {
    set_pin(state, id, true).await
}

/// Unpins a system (`DELETE /api/systems/{id}/pin`).
async fn unpin_system(
    state: State<Shared>,
    id: UrlPath<String>,
) -> Result<Json<SystemSummary>, Failure> {
    set_pin(state, id, false).await
}

async fn set_pin(
    State(state): State<Shared>,
    UrlPath(id): UrlPath<String>,
    pinned: bool,
) -> Result<Json<SystemSummary>, Failure> {
    if !systems::is_valid_system_id(&id) {
        return Err(Failure(StatusCode::BAD_REQUEST, "invalid system id".into()));
    }
    let index = state.index.clone();
    let owned = id.clone();
    tokio::task::spawn_blocking(move || index.set_pinned(&owned, pinned))
        .await
        .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or_else(|| Failure(StatusCode::NOT_FOUND, format!("no system {id}")))
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
    let index = state.index.clone();
    let result =
        tokio::task::spawn_blocking(move || index.give_names(&root, &owned, &request.names))
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
    note_opened(&state, &id, &body);
    body_tile(state, loc, TileId { z, x, y }, query).await
}

async fn system_epoch_layer(
    State(state): State<Shared>,
    UrlPath((id, body, epoch_ma, file)): UrlPath<(String, String, i32, String)>,
    Query(query): Query<EpochQuery>,
) -> Result<Response, Failure> {
    let loc = system_body_loc(&state, &id, &body).await?;
    note_opened(&state, &id, &body);
    body_epoch_layer(state, loc, epoch_ma, file, query).await
}

async fn system_drift_preview(
    State(state): State<Shared>,
    UrlPath((id, body, drift_ma, file)): UrlPath<(String, String, i32, String)>,
    Query(query): Query<DriftQuery>,
) -> Result<Response, Failure> {
    let loc = system_body_loc(&state, &id, &body).await?;
    note_opened(&state, &id, &body);
    body_drift_preview(state, &id, &body, loc, drift_ma, file, query).await
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
    note_opened(&state, &id, &body);
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
    let epochs = tokio::task::spawn_blocking(move || systems::epochs_at(&owned))
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

/// Query of a drift preview request: the width (256 or 512, default 512)
/// and the world revision the client knows.
#[derive(serde::Deserialize)]
struct DriftQuery {
    rev: Option<u64>,
    width: Option<u32>,
}

/// A low-resolution preview of a body at `drift_ma` Ma from the present
/// (negative in the past), generated from the body's stored parameters
/// with the epoch changed (see [`mapgen::drift`]): 404 for a body without
/// plates, 400 outside `[-300, 300]` Ma, 503 while a generation runs.
async fn body_drift_preview(
    state: Shared,
    system: &str,
    body: &str,
    loc: BodyLoc,
    drift_ma: i32,
    file: String,
    query: DriftQuery,
) -> Result<Response, Failure> {
    use mapgen::drift::{self, PreviewError};
    if file != "surface.png" {
        return Err(Failure(
            StatusCode::NOT_FOUND,
            format!("no layer {file}: surface.png"),
        ));
    }
    let width = query
        .width
        .unwrap_or(*drift::PREVIEW_WIDTHS.last().expect("widths"));
    let cache_header = |revision: u64| {
        if query.rev == Some(revision) {
            IMMUTABLE
        } else {
            "no-cache"
        }
    };
    let png_response = |png: Arc<Vec<u8>>, revision: u64| {
        (
            [
                (header::CONTENT_TYPE, "image/png"),
                (header::CACHE_CONTROL, cache_header(revision)),
            ],
            png.as_ref().clone(),
        )
            .into_response()
    };
    let key = PreviewKey {
        body: loc.key.clone(),
        revision: loc.revision,
        drift_ma,
        width,
    };
    if let Some(png) = state.drift_cache.get(&key) {
        return Ok(png_response(png, loc.revision));
    }
    let root = state.config.systems_dir.clone();
    let (owned_system, owned_body) = (system.to_owned(), body.to_owned());
    let params = tokio::task::spawn_blocking(move || {
        systems::body_params(&root, &owned_system, &owned_body)
    })
    .await
    .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or_else(|| {
        Failure(
            StatusCode::NOT_FOUND,
            format!("no parameters for {}", loc.key),
        )
    })?;
    // Refused before waiting for a turn.
    let refused = |e: PreviewError| match e {
        PreviewError::NoPlates => Failure(StatusCode::NOT_FOUND, e.to_string()),
        PreviewError::OutOfRange | PreviewError::BadWidth => {
            Failure(StatusCode::BAD_REQUEST, e.to_string())
        }
    };
    drift::preview_params(&params, drift_ma as f32, width).map_err(refused)?;
    let _turn = state
        .drift_previews
        .acquire()
        .await
        .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    // Another request may have made it while this one waited.
    if let Some(png) = state.drift_cache.get(&key) {
        return Ok(png_response(png, loc.revision));
    }
    // A generation uses every core and all the memory it can: previews
    // wait for it to end, as tiles do.
    let Ok(_generation) = state.queue.try_acquire() else {
        return Ok(world_failure(TileError::Busy, "drift previews"));
    };
    let png = tokio::task::spawn_blocking(move || {
        let preview = drift::preview(&params, drift_ma as f32, width).map_err(|e| e.to_string())?;
        let image = image::RgbImage::from_fn(preview.width, preview.height, |x, y| {
            image::Rgb(preview.pixels[(y * preview.width + x) as usize])
        });
        let mut bytes = std::io::Cursor::new(Vec::new());
        image
            .write_to(&mut bytes, image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;
        Ok::<_, String>(bytes.into_inner())
    })
    .await
    .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let png = Arc::new(png);
    state.drift_cache.insert(key, png.clone());
    Ok(png_response(png, loc.revision))
}

/// Query of a thumbnail request: the world revision the client knows.
#[derive(serde::Deserialize)]
struct ThumbQuery {
    rev: Option<u64>,
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
    note_opened(state, id, body);
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

    let job_number = state.next_job.fetch_add(1, Ordering::Relaxed);
    let job_id = job_number.to_string();
    let (tx, rx) = watch::channel(JobStatus::Queued);
    let cancel = Arc::new(Cancel::default());
    {
        let (index, owned, owned_body) = (state.index.clone(), id.to_string(), body.to_string());
        let queued = tokio::task::spawn_blocking(move || {
            index.job_queued(
                job_number,
                &owned,
                Some((&owned, &owned_body)),
                &WorldRequest::default(),
                FINISHED_JOBS_KEPT,
            )
        })
        .await;
        if let Err(e) = queued
            .map_err(|e| e.to_string())
            .and_then(|r| r.map_err(|e| e.to_string()))
        {
            tracing::error!("job {job_number} not stored: {e}");
        }
    }
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
        // Stored before it is announced, so that a restart right after
        // finds what a client saw. `None` is a job that starts running.
        let store = |status: Option<JobStatus>| {
            let (index, root, system) = (
                task_state.index.clone(),
                task_state.config.systems_dir.clone(),
                task_system.clone(),
            );
            async move {
                let stored = tokio::task::spawn_blocking(move || match status {
                    None => index.job_running(job_number),
                    Some(status) => {
                        // What the job generated is in the tree: read it.
                        if status.is_finished() {
                            index.refresh_system(&root, &system, Cause::Generated)?;
                        }
                        index.job_status(job_number, &status)
                    }
                })
                .await;
                if let Err(e) = stored
                    .map_err(|e| e.to_string())
                    .and_then(|r| r.map_err(|e| e.to_string()))
                {
                    tracing::error!("job {job_number} not stored: {e}");
                }
            }
        };
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
                store(Some(JobStatus::Cancelled)).await;
                finish(JobStatus::Cancelled);
                return;
            }
        };
        if cancel.is_requested() {
            store(Some(JobStatus::Cancelled)).await;
            finish(JobStatus::Cancelled);
            return;
        }
        store(None).await;
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
        store(Some(status.clone())).await;
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
        std::fs::create_dir_all(root.join("systems")).unwrap();
        Config {
            systems_dir: root.join("systems"),
            db_path: root.join("db").join("mapgen.sqlite"),
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

    /// Draws a system round `planet` (seed 3) and generates its planet:
    /// the system's id and the planet's body id.
    /// Generates `planet` around its own system, with seed `seed`: a system's
    /// id is its star's designation and its seed, so two planets of one test
    /// need two seeds, or two stars drawn alike would collide.
    async fn generated_planet(
        app: &Router,
        planet: &str,
        seed: u64,
        extra: &str,
    ) -> (String, String) {
        let request = format!(
            r#"{{"around": {{"planet": "{planet}", "seed": {seed}, "width": 256, "height": 128, "moons": "none", "giants": "none"{extra}}}}}"#
        );
        let (status, body) = call(app, "POST", "/api/systems", Some(&request)).await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        wait_job(app, &body).await;
        let created: serde_json::Value = serde_json::from_str(&body).unwrap();
        let id = created["system_id"].as_str().unwrap().to_string();
        let (_, detail) = call(app, "GET", &format!("/api/systems/{id}"), None).await;
        let detail: serde_json::Value = serde_json::from_str(&detail).unwrap();
        let focus = detail["system"]["focus"].as_str().unwrap().to_string();
        (id, focus)
    }

    #[tokio::test]
    async fn serves_bodies_at_other_epochs() {
        let config = temp_config("epochs");
        let root = config.systems_dir.clone();
        let app = router(config);
        // A catalog example preset, airless and cratered; a wet terran.
        let (old, rock) = generated_planet(
            &app,
            "examples/mercury-like.toml",
            3,
            r#", "surface_age_ga": 4.0"#,
        )
        .await;
        let (wet, sea) = generated_planet(&app, "terran", 4, "").await;

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
        let base = format!("/api/systems/{old}/bodies/{rock}/epoch");
        // The present epoch is the body as generated.
        let (status, now) = png(format!("{base}/0/elevation.png")).await;
        assert_eq!(status, StatusCode::OK);
        let now = image::load_from_memory(&now).unwrap().into_luma16();
        let stored =
            image::open(systems::body_dir(&root, &old, &rock).join("raw_data/elevation.png"))
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
        let tile = format!("/api/systems/{old}/bodies/{rock}/tiles/3/2/1?epoch=-500");
        assert_eq!(call(&app, "GET", &tile, None).await.0, StatusCode::OK);
        // Refused: outside the range, a body without epochs, unknown layer, bad scale.
        let wet_base = format!("/api/systems/{wet}/bodies/{sea}");
        for (uri, expected) in [
            (format!("{base}/4100/surface.png"), StatusCode::BAD_REQUEST),
            (format!("{base}/-1001/surface.png"), StatusCode::BAD_REQUEST),
            (
                format!("{wet_base}/epoch/100/surface.png"),
                StatusCode::NOT_FOUND,
            ),
            (format!("{base}/100/flow.png"), StatusCode::NOT_FOUND),
            (
                format!("{base}/100/surface.png?scale=3"),
                StatusCode::BAD_REQUEST,
            ),
            (
                format!("{wet_base}/tiles/1/0/0?epoch=100"),
                StatusCode::NOT_FOUND,
            ),
        ] {
            let (status, _) = call(&app, "GET", &uri, None).await;
            assert_eq!(status, expected, "{uri}");
        }
    }

    #[tokio::test]
    async fn previews_plates_at_other_epochs() {
        let config = temp_config("drift");
        let app = router(config);
        // Size and air pinned: a random small or thin-air draw would turn
        // the plates into a stagnant lid (see `mapgen::geology::regime_of`)
        // and the route would answer 404.
        let (plates, planet) = generated_planet(
            &app,
            "terran",
            5,
            r#", "circumference_km": 40000, "pressure_bar": 1.0"#,
        )
        .await;
        let (dry, rock) = generated_planet(&app, "examples/mercury-like.toml", 6, "").await;
        let get = |uri: String| {
            let app = app.clone();
            async move {
                let request = Request::get(&uri).body(Body::empty()).unwrap();
                let response = app.oneshot(request).await.unwrap();
                let status = response.status();
                let cache = response.headers().get(header::CACHE_CONTROL).cloned();
                let bytes = response.into_body().collect().await.unwrap().to_bytes();
                (status, cache, bytes)
            }
        };
        let base = format!("/api/systems/{plates}/bodies/{planet}/drift");
        // A coarse and a finer preview at the aspect ratio of the world
        // (256 x 128, so the finer one is capped at its width).
        let (status, _, small) = get(format!("{base}/-120/surface.png?width=256")).await;
        assert_eq!(status, StatusCode::OK);
        let image = image::load_from_memory(&small).unwrap();
        assert_eq!((image.width(), image.height()), (256, 128));
        let (_, _, again) = get(format!("{base}/-120/surface.png?width=256")).await;
        assert_eq!(small, again, "served from the cache, the same bytes");
        let (_, _, other) = get(format!("{base}/120/surface.png?width=256")).await;
        assert_ne!(small, other);
        // Only a URL naming the current revision is cached for good.
        let (status, cache, _) = get(format!("{base}/0/surface.png?width=256&rev=1")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(cache.unwrap(), "no-cache");
        // Refused: out of range, bad width, unknown layer, a world without plates.
        for (uri, expected) in [
            (format!("{base}/301/surface.png"), StatusCode::BAD_REQUEST),
            (format!("{base}/-301/surface.png"), StatusCode::BAD_REQUEST),
            (
                format!("{base}/10/surface.png?width=300"),
                StatusCode::BAD_REQUEST,
            ),
            (format!("{base}/10/elevation.png"), StatusCode::NOT_FOUND),
            (
                format!("/api/systems/{dry}/bodies/{rock}/drift/10/surface.png"),
                StatusCode::NOT_FOUND,
            ),
            (
                format!("/api/systems/{plates}/bodies/zz/drift/10/surface.png"),
                StatusCode::NOT_FOUND,
            ),
        ] {
            let (status, _, _) = get(uri.clone()).await;
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
            r#"{"around": {"sea_level": 3.0}}"#,
            r#"{"around": {"planet": "/etc/passwd.toml"}}"#,
            r#"{"around": {"width": 100000}}"#,
            r#"{"around": {"width": 8192, "height": 4096}}"#,
            r#"{"around": {"unknown": 1}}"#,
            r#"{"elsewhere": {}}"#,
        ] {
            let (status, _) = call(&app, "POST", "/api/systems", Some(body)).await;
            assert!(status.is_client_error(), "{body} gave {status}");
        }
        let (status, _) = call(&app, "DELETE", "/api/systems/..%2Fetc-1", None).await;
        assert!(status.is_client_error());
    }

    #[tokio::test]
    async fn rejected_requests_carry_a_machine_readable_detail() {
        let app = router(temp_config("detail"));
        let detail = |body: &str| -> serde_json::Value {
            serde_json::from_str::<serde_json::Value>(body).unwrap()["detail"].clone()
        };
        let post = |body: &'static str| {
            let app = app.clone();
            async move { call(&app, "POST", "/api/systems", Some(body)).await }
        };
        let (status, body) = post(r#"{"around": {"sea_level": 3.0}}"#).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let out_of_range = detail(&body);
        assert_eq!(out_of_range["code"], "out_of_range");
        assert_eq!(out_of_range["field"], "sea_level");
        assert!(out_of_range["min"].is_number() && out_of_range["max"].is_number());
        let (_, body) = post(r#"{"around": {"width": 100000}}"#).await;
        assert_eq!(detail(&body)["code"], "size_out_of_range");
        let (_, body) = post(r#"{"around": {"planet": "nowhere"}}"#).await;
        assert_eq!(detail(&body)["code"], "unknown_preset");
        let (_, body) = post(r#"{"around": {"sea_level": 0.1, "land_fraction": 0.3}}"#).await;
        assert_eq!(detail(&body)["code"], "sea_level_and_land_fraction");
        let (_, body) = post(
            r#"{"system": {"star": null, "metallicity": null, "planets": {"terran": 16}, "seed": 1, "width": 64, "height": 32}}"#,
        )
        .await;
        assert_eq!(detail(&body)["code"], "too_many_planets");
        // Errors without a detail send none.
        let (_, body) = call(&app, "DELETE", "/api/systems/..%2Fetc-1", None).await;
        assert!(detail(&body).is_null());
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
        // Names given are kept when the system is replaced, those of bodies
        // it no longer has dropped.
        let (_, drawn) = call(&app, "GET", &format!("/api/systems/{id}"), None).await;
        let drawn: serde_json::Value = serde_json::from_str(&drawn).unwrap();
        let first = drawn["system"]["bodies"][0]["id"]
            .as_str()
            .unwrap()
            .to_string();
        let names = format!(r#"{{"names": {{"{first}": "Gaia", "star": "Helia"}}}}"#);
        let uri = format!("/api/systems/{id}/names");
        assert_eq!(
            call(&app, "PUT", &uri, Some(&names)).await.0,
            StatusCode::OK
        );
        // A pin is kept too.
        let pin = format!("/api/systems/{id}/pin");
        let (status, pinned) = call(&app, "PUT", &pin, None).await;
        assert_eq!(status, StatusCode::OK, "{pinned}");
        assert!(pinned.contains(r#""pinned":true"#), "{pinned}");
        let (status, _) = call(&app, "POST", "/api/systems?replace=true", Some(request)).await;
        assert_eq!(status, StatusCode::CREATED);
        let (_, list) = call(&app, "GET", "/api/systems", None).await;
        assert!(list.contains(r#""pinned":true"#), "{list}");
        let (status, unpinned) = call(&app, "DELETE", &pin, None).await;
        assert_eq!(status, StatusCode::OK);
        assert!(unpinned.contains(r#""pinned":false"#), "{unpinned}");
        assert_eq!(
            call(&app, "PUT", "/api/systems/mgs-0-0/pin", None).await.0,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            call(&app, "DELETE", "/api/systems/..x/pin", None).await.0,
            StatusCode::BAD_REQUEST
        );
        let kept = std::fs::read_to_string(root.join(&id).join("names.json")).unwrap();
        assert!(kept.contains("Gaia") && kept.contains("Helia"), "{kept}");
        let (_, list) = call(&app, "GET", "/api/systems", None).await;
        let list: serde_json::Value = serde_json::from_str(&list).unwrap();
        assert_eq!(list[0]["names"][&first], "Gaia");

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
    async fn the_index_keeps_names_jobs_and_openings_across_a_restart() {
        let config = temp_config("restart");
        let db = config.db_path.clone();
        let app = router(config.clone());
        let request = r#"{"around": {"planet": "barren", "seed": 8, "width": 64, "height": 32, "moons": "none", "giants": "none"}}"#;
        let (status, body) = call(&app, "POST", "/api/systems", Some(request)).await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        wait_job(&app, &body).await;
        let created: serde_json::Value = serde_json::from_str(&body).unwrap();
        let (id, job) = (
            created["system_id"].as_str().unwrap().to_string(),
            created["job_id"].as_str().unwrap().to_string(),
        );
        let (_, detail) = call(&app, "GET", &format!("/api/systems/{id}"), None).await;
        let detail: serde_json::Value = serde_json::from_str(&detail).unwrap();
        let planet = detail["system"]["focus"].as_str().unwrap().to_string();

        // The list tells the body is generated, from the index.
        let (_, list) = call(&app, "GET", "/api/systems", None).await;
        let list: serde_json::Value = serde_json::from_str(&list).unwrap();
        assert_eq!(list[0]["id"], id.as_str());
        let generated = |list: &serde_json::Value| {
            list[0]["bodies"]
                .as_array()
                .unwrap()
                .iter()
                .any(|b| b["id"] == planet.as_str() && b["generated"] == true)
        };
        assert!(generated(&list));

        // A name, and a file of the body served.
        let names = format!(r#"{{"names": {{"{planet}": "Gaia"}}}}"#);
        let uri = format!("/api/systems/{id}/names");
        assert_eq!(
            call(&app, "PUT", &uri, Some(&names)).await.0,
            StatusCode::OK
        );
        let file = get_gzip(
            &app,
            &format!("/systems/{id}/bodies/{planet}/raw_data/world.json"),
        )
        .await;
        assert_eq!(file.status(), StatusCode::OK);
        let (_, before) = call(&app, "GET", "/api/jobs", None).await;

        // Without its database, the index is rebuilt from the tree.
        let rebuilt = router(Config {
            db_path: db.with_file_name("rebuilt.sqlite"),
            ..config.clone()
        });
        let (_, again) = call(&rebuilt, "GET", "/api/systems", None).await;
        let again: serde_json::Value = serde_json::from_str(&again).unwrap();
        assert_eq!(again[0]["names"][&planet], "Gaia", "from names.json");
        assert!(generated(&again));
        assert_eq!(call(&rebuilt, "GET", "/api/jobs", None).await.1, "[]");

        // A restart on the same database: the same list, the job history,
        // and the date the body was opened.
        drop(app);
        let restarted = router(config);
        let (_, after) = call(&restarted, "GET", "/api/systems", None).await;
        let after: serde_json::Value = serde_json::from_str(&after).unwrap();
        assert_eq!(after, list_with_name(&list, &planet, "Gaia"));
        let (_, jobs) = call(&restarted, "GET", "/api/jobs", None).await;
        assert_eq!(jobs, before);
        assert!(jobs.contains(&format!("\"job_id\":\"{job}\"")), "{jobs}");
        let index = Index::open(&db).unwrap();
        // Written by a blocking task, a moment after the request.
        for _ in 0..100 {
            if index.last_opened(&id, &planet).unwrap().is_some() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        assert!(index.last_opened(&id, &planet).unwrap().is_some());
        // The next job does not reuse an id.
        let generate = format!("/api/systems/{id}/bodies/{planet}/generate");
        let (_, none) = call(&restarted, "POST", &generate, None).await;
        assert!(serde_json::from_str::<serde_json::Value>(&none).unwrap()["job_id"].is_null());
    }

    /// `list` with the name `name` given to `body` of its first system.
    fn list_with_name(list: &serde_json::Value, body: &str, name: &str) -> serde_json::Value {
        let mut list = list.clone();
        list[0]["names"][body] = name.into();
        list
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

    #[tokio::test]
    async fn lists_and_cancels_jobs() {
        let config = temp_config("cancel");
        let root = config.systems_dir.clone();
        let app = router(config);
        // The first job holds the queue; the second waits behind it.
        let first = r#"{"around": {"planet": "barren", "seed": 1, "width": 512, "height": 256, "moons": "none", "giants": "none"}}"#;
        let small = r#"{"around": {"planet": "barren", "seed": 2, "width": 64, "height": 32, "moons": "none", "giants": "none"}}"#;
        let (_, body) = call(&app, "POST", "/api/systems", Some(first)).await;
        let first: serde_json::Value = serde_json::from_str(&body).unwrap();
        let (_, body) = call(&app, "POST", "/api/systems", Some(small)).await;
        let second: serde_json::Value = serde_json::from_str(&body).unwrap();
        let second_id = second["job_id"].as_str().unwrap();
        let (first_system, second_system) = (
            first["system_id"].as_str().unwrap(),
            second["system_id"].as_str().unwrap(),
        );

        // A system being generated cannot be deleted.
        let (status, _) = call(
            &app,
            "DELETE",
            &format!("/api/systems/{first_system}"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);

        let (status, list) = call(&app, "GET", "/api/jobs", None).await;
        assert_eq!(status, StatusCode::OK);
        let list: serde_json::Value = serde_json::from_str(&list).unwrap();
        assert_eq!(list[0]["job_id"], second_id, "most recent first");
        assert_eq!(list[0]["system_id"], second_system);

        let (status, _) = call(&app, "DELETE", &format!("/api/jobs/{second_id}"), None).await;
        assert_eq!(status, StatusCode::ACCEPTED);
        let uri = format!("/api/jobs/{second_id}/events");
        let (_, events) = call(&app, "GET", &uri, None).await;
        assert!(events.contains("\"status\":\"cancelled\""), "{events}");
        assert!(!root.join(second_system).join("bodies").join("b").exists());

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

        // The queue is free again.
        let generate = format!("/api/systems/{second_system}/bodies/b/generate");
        let (status, body) = call(&app, "POST", &generate, None).await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }
}
