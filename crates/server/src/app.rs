//! HTTP routes and shared state.
//!
//! | Route                           | Purpose                                           |
//! |---------------------------------|---------------------------------------------------|
//! | `GET /healthz`                  | liveness probe                                    |
//! | `GET /api/presets`              | planet presets the form can offer                 |
//! | `GET /api/form`                 | parameter limits, defaults and map sizes          |
//! | `GET /api/systems`              | drawn systems, most recent first (`SystemSummary`) |
//! | `POST /api/systems?replace=`    | draw a system (`SystemRequest`); mode `around` queues its planet |
//! | `GET /api/systems/{id}`         | a system (`system.json`), the names given and the bodies evicted (`evicted`, see [`evict`]) |
//! | `DELETE /api/systems/{id}`      | delete a system (409 while it or a resolution of a body is being generated) |
//! | `GET /api/systems/{id}/export`  | a `.zip` of the system: `system.json` (seeds, requests, names, generator versions; see `crate::export`) and the PNGs of its generated bodies |
//! | `PUT /api/systems/{id}/names`   | name its bodies, the star too (`SystemNames`)      |
//! | `PUT /api/systems/{id}/name`   | name the system itself, keeping its bodies' names (`SystemName`; null clears) |
//! | `PUT /api/systems/{id}/pin`     | pin it in the main menu (idempotent, answers its summary) |
//! | `DELETE /api/systems/{id}/pin`  | unpin it (idempotent, answers its summary)         |
//! | `PUT /api/systems/{id}/bodies/{body}/name` | name one body, the other names kept (`SystemName`; null clears) |
//! | `POST /api/systems/{id}/bodies/{body}/generate` | queue a body and its neighbourhood (`GenerateStarted`; `regenerated` names each evicted body of the family the job makes again that another generator made; an evicted body also comes back at its displayed size, its neighbours at their original one) |
//! | `GET /api/systems/{id}/bodies/{body}/tiles/…`, `…/epoch/…`, `…/thumb`; `POST …/hex` | as for a world, per generated body; `?size=<W>x<H>` picks a resolution (tiles, epoch, hex; default the displayed one, 404 when not generated, 400 for a bad name); the thumb is the original's, and an evicted body still answers it from its kept `thumb.png` (404 when it has none) |
//! | `GET /api/systems/{id}/bodies/{body}/sizes` | its resolutions: original, displayed, present, pending, offered (`BodySizes`; 404 when not generated) |
//! | `POST /api/systems/{id}/bodies/{body}/sizes/{W}x{H}` | queue that resolution (`GenerateStarted`: no job when present, the pending job when queued; the server marks it displayed when the job is done; 400 `size_not_offered` or `too_many_pixels`) |
//! | `DELETE /api/systems/{id}/bodies/{body}/sizes/{W}x{H}` | delete an added resolution, its cached world and previews (204; 409 `original_size` or `being_generated`; 404 `size_not_generated`) |
//! | `PUT /api/systems/{id}/bodies/{body}/displayed-size` | remember the resolution shown (`DisplayedSize`, 204; 404 `size_not_generated`) |
//! | `GET /api/systems/{id}/bodies/{body}/drift/{ma}/surface.png` | low-resolution preview of a world with plates at another epoch of its plates (`?size=` as above, for its `rev`) |
//! | `GET /systems/{id}/...`         | system files (`system.json`, `bodies/<body>/sizes/<W>x<H>/…`) |
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
    ApiError, BodySizes, DisplayedSize, ErrorDetail, FormSpec, GenerateStarted, GeneratorNote,
    HexRequest, JobInfo, JobStatus, ParamSpec, PendingSize, PresentSize, PresetInfo,
    RegeneratedBody, SystemCreated, SystemDetail, SystemName, SystemNames, SystemQuery,
    SystemSummary,
};
use crate::catalog::Catalog;
use crate::index::{Cause, GENERATOR_FORMAT, GENERATOR_VERSION, Index};
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

pub mod evict;

/// Largest map side the server accepts.
pub const MAX_SIZE: u32 = 8192;
/// Default largest map the server accepts, in pixels (8192 × 4096). A
/// generation without noise maps, as the server runs it, peaks at about 70
/// bytes per pixel (measured in Docker: 545 MiB here, 2.0 GiB at 8192 ×
/// 4096): keep the limit under the memory the server can use, or it gets
/// killed mid-job.
pub const DEFAULT_MAX_PIXELS: u64 = 8192 * 4096;
/// Default days a body can stay unopened before its files are evicted.
pub const DEFAULT_EVICT_AFTER_DAYS: u64 = 14;
/// Peak generation memory per pixel, for the error message.
const BYTES_PER_PIXEL: u64 = 70;
/// Finished jobs kept for `GET /api/jobs`; older ones are forgotten.
const FINISHED_JOBS_KEPT: usize = 20;

/// A generation job: its progress, and how to cancel it.
struct Job {
    world_id: String,
    /// A system's job: the system and the body asked for.
    system: Option<(String, String)>,
    /// A job generating one resolution of that body: its size. `None`
    /// for the body and its neighbourhood.
    size: Option<(u32, u32)>,
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
    /// Write the diagnostic `noise_maps/` of every body generated
    /// (`MAPGEN_NOISE_MAPS`); off by default. `world.svg` is never written
    /// by the server, see [`system_svg`].
    pub noise_maps: bool,
    /// Days a body can stay unopened before its generated files are
    /// evicted (`MAPGEN_EVICT_AFTER_DAYS`, see [`evict`]); 0 never evicts.
    pub evict_after_days: u64,
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
    /// Generations queued or running, to refuse a second job writing the
    /// same directory: a system's own job ([`pending_system`]) and the
    /// resolutions being added to its bodies ([`pending_size`]).
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
    /// The original resolution of each body, read from `system.json` once
    /// per change of the file rather than on every tile.
    originals: Arc<systems::OriginalSizes>,
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
        TileError::TooOld(e) => Failure(StatusCode::GONE, e).into_response(),
        TileError::Failed(e) => Failure(StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

pub fn router(config: Config) -> Router {
    let state = new_state(&config);
    evict::spawn_sweeps(state.clone());
    routes(state, &config)
}

/// The server's state: the tree swept, the index opened and reconciled.
fn new_state(config: &Config) -> Shared {
    // Exports a killed server left unfinished.
    systems::sweep(&config.systems_dir);
    let catalog = Catalog::load(&config.presets_dir);
    let (index, jobs, next_job) = open_index(config);
    Arc::new(AppState {
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
        originals: Arc::default(),
        config: config.clone(),
    })
}

/// The routes over `state`.
fn routes(state: Shared, config: &Config) -> Router {
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
        .route("/api/systems/{id}/export", get(export_system))
        .route("/api/systems/{id}/names", put(rename_system))
        .route("/api/systems/{id}/name", put(name_system))
        .route("/api/systems/{id}/bodies/{body}/name", put(name_body))
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
        .route("/api/systems/{id}/bodies/{body}/sizes", get(body_sizes))
        .route(
            "/api/systems/{id}/bodies/{body}/sizes/{size}",
            post(generate_size).delete(delete_size),
        )
        .route(
            "/api/systems/{id}/bodies/{body}/displayed-size",
            put(set_displayed_size),
        )
        .route("/api/systems/{id}/bodies/{body}/world.svg", get(system_svg))
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
                    size: job.size,
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

/// Records, from a request for a file under `/systems/<id>/bodies/<body>/`,
/// that the body was opened. Not recorded: the thumbnails (the library asks
/// for all of them) and any request with `?peek=1`, which the viewer adds
/// when it takes a body's textures for the sky or the markers of another
/// body's view (see `peeked` in `web/src/lib/api/client.ts`): looking at a body from
/// elsewhere is not opening it.
async fn note_served(
    State(state): State<Shared>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let path = request.uri().path().to_string();
    let parts: Vec<&str> = path.split('/').skip(1).collect();
    if let ["systems", id, "bodies", body, file, ..] = parts.as_slice()
        && *file != "thumb.png"
        && !is_peek(request.uri().query())
        && systems::is_valid_system_id(id)
        && systems::is_valid_body_id(body)
    {
        note_opened(&state, id, body);
    }
    next.run(request).await
}

/// Whether a query string says `peek=1`: the request does not open the body.
fn is_peek(query: Option<&str>) -> bool {
    query.is_some_and(|q| q.split('&').any(|pair| pair == "peek=1"))
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
        size_presets: request::SIZE_PRESETS.iter().map(|&(w, h)| [w, h]).collect(),
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

/// System `id` held for a change of its directory (delete, replace): its
/// pending key is in the list until the guard is dropped, so no job starts
/// and no eviction (`evict::evict_body`, which checks the list under the
/// same lock) touches the system meanwhile.
struct SystemHold {
    state: Shared,
    key: String,
}

impl Drop for SystemHold {
    fn drop(&mut self) {
        let mut pending = self.state.pending.lock().expect("pending lock");
        if let Some(at) = pending.iter().position(|k| *k == self.key) {
            pending.remove(at);
        }
    }
}

/// Holds system `id` (see [`SystemHold`]); `None` when it has a generation
/// queued or running, or is held already. The check and the claim are one
/// step under the pending list's lock.
fn hold_system(state: &Shared, id: &str) -> Option<SystemHold> {
    let mut pending = state.pending.lock().expect("pending lock");
    if system_is_pending(&pending, id) {
        return None;
    }
    let key = pending_system(id);
    pending.push(key.clone());
    Some(SystemHold {
        state: state.clone(),
        key,
    })
}

/// The key a resolution being added to a body takes in the list of
/// pending generations: `<id>/<body>/<W>x<H>`.
fn pending_size(id: &str, body: &str, size: (u32, u32)) -> String {
    format!("{id}/{body}/{}", mapgen::sizes::size_name(size.0, size.1))
}

/// Whether system `id` has a generation queued or running: its own job, or
/// a resolution of one of its bodies.
fn system_is_pending(pending: &[String], id: &str) -> bool {
    let system = pending_system(id);
    let sizes = format!("{id}/");
    pending
        .iter()
        .any(|key| *key == system || key.starts_with(&sizes))
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
    // Held until the files are in place: nothing else touches the system
    // directory meanwhile (a job, a deletion, an eviction).
    let Some(hold) = hold_system(&state, &id) else {
        return Err(Rejection(
            Failure(
                StatusCode::CONFLICT,
                format!("system {id} is being generated"),
            ),
            Some(ErrorDetail::BeingGenerated { world: id }),
        ));
    };
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
    state.originals.forget(&id);
    // The first job claims the system's key itself.
    drop(hold);
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
    .map_err(|e| unreadable_system(&state.config.systems_dir, id, &e))?;
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
        let system = mapgen::system::read_system(&systems::system_dir(&root, &owned))?;
        Ok::<_, String>(SystemDetail {
            system,
            names: index.names(&owned).unwrap_or_default(),
            evicted: index.evicted_bodies(&owned).unwrap_or_default(),
        })
    })
    .await
    .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map(Json)
    .map_err(|e| unreadable_system(&state.config.systems_dir, &id, &e))
}

/// The failure of a system whose `system.json` could not be read: 404 when
/// there is none, else 410 Gone: it was made by an older generator (its
/// file no longer parses) or is damaged, and the way out is the same.
fn unreadable_system(root: &Path, id: &str, error: &str) -> Failure {
    if !systems::system_dir(root, id).join("system.json").is_file() {
        return Failure(StatusCode::NOT_FOUND, format!("no system {id}"));
    }
    Failure(
        StatusCode::GONE,
        format!(
            "system {id} was made by an older generator or is damaged ({error}): delete it \
             (`mise run clean-systems`) and draw it again"
        ),
    )
}

async fn delete_system(
    State(state): State<Shared>,
    UrlPath(id): UrlPath<String>,
) -> Result<StatusCode, Failure> {
    if !systems::is_valid_system_id(&id) {
        return Err(Failure(StatusCode::BAD_REQUEST, "invalid system id".into()));
    }
    // Held until the directory and the index rows are gone: an eviction
    // or a job cannot recreate `system.json` in between.
    let Some(_hold) = hold_system(&state, &id) else {
        return Err(Failure(
            StatusCode::CONFLICT,
            format!("system {id} is being generated: cancel its job first"),
        ));
    };
    let dir = systems::system_dir(&state.config.systems_dir, &id);
    if !dir.is_dir() {
        return Err(Failure(StatusCode::NOT_FOUND, format!("no system {id}")));
    }
    let forgotten = state.tiles.forget(&format!("systems/{id}"));
    state.originals.forget(&id);
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

/// Exports a system (`GET /api/systems/{id}/export`): a `.zip` named after
/// it, see [`crate::export`].
async fn export_system(
    State(state): State<Shared>,
    UrlPath(id): UrlPath<String>,
) -> Result<Response, Failure> {
    if !systems::is_valid_system_id(&id) {
        return Err(Failure(StatusCode::BAD_REQUEST, "invalid system id".into()));
    }
    let (root, owned, index) = (
        state.config.systems_dir.clone(),
        id.clone(),
        state.index.clone(),
    );
    let archive = tokio::task::spawn_blocking(move || {
        crate::export::build_archive(&root, &owned, &index, crate::index::now_ms())
    })
    .await
    .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| match e {
        crate::export::ExportError::System(e) => {
            unreadable_system(&state.config.systems_dir, &id, &e)
        }
        crate::export::ExportError::Other(e) => Failure(StatusCode::INTERNAL_SERVER_ERROR, e),
    })?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/zip".to_owned()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{id}.zip\""),
            ),
        ],
        archive,
    )
        .into_response())
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
    result.map(Json).map_err(|e| naming_failure(e, &id))
}

/// Names the system itself (`PUT /api/systems/{id}/name`): merged into the
/// names given, those of the bodies untouched.
async fn name_system(
    State(state): State<Shared>,
    UrlPath(id): UrlPath<String>,
    Json(request): Json<SystemName>,
) -> Result<Json<SystemSummary>, Failure> {
    if !systems::is_valid_system_id(&id) {
        return Err(Failure(StatusCode::BAD_REQUEST, "invalid system id".into()));
    }
    let root = state.config.systems_dir.clone();
    let owned = id.clone();
    let index = state.index.clone();
    let result = tokio::task::spawn_blocking(move || {
        index.set_system_name(&root, &owned, request.name.as_deref())
    })
    .await
    .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    result.map(Json).map_err(|e| naming_failure(e, &id))
}

/// Names one body (`PUT /api/systems/{id}/bodies/{body}/name`): merged into
/// the names given, the others untouched. `star` works too.
async fn name_body(
    State(state): State<Shared>,
    UrlPath((id, body)): UrlPath<(String, String)>,
    Json(request): Json<SystemName>,
) -> Result<Json<SystemSummary>, Failure> {
    if !systems::is_valid_system_id(&id) || !systems::is_valid_body_id(&body) {
        return Err(Failure(
            StatusCode::BAD_REQUEST,
            "invalid system or body id".into(),
        ));
    }
    let root = state.config.systems_dir.clone();
    let owned = id.clone();
    let index = state.index.clone();
    let result = tokio::task::spawn_blocking(move || {
        index.set_name(&root, &owned, &body, request.name.as_deref())
    })
    .await
    .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    result.map(Json).map_err(|e| naming_failure(e, &id))
}

fn naming_failure(e: systems::NamingError, id: &str) -> Failure {
    match e {
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
    }
}

async fn system_tile(
    State(state): State<Shared>,
    UrlPath((id, body, z, x, y)): UrlPath<(String, String, u32, u32, u32)>,
    Query(query): Query<TileQuery>,
) -> Result<Response, Failure> {
    let size = query_size(query.size.as_deref())?;
    let loc = system_body_loc(&state, &id, &body, size).await?;
    note_opened(&state, &id, &body);
    body_tile(state, loc, TileId { z, x, y }, query).await
}

async fn system_epoch_layer(
    State(state): State<Shared>,
    UrlPath((id, body, epoch_ma, file)): UrlPath<(String, String, i32, String)>,
    Query(query): Query<EpochQuery>,
) -> Result<Response, Failure> {
    let size = query_size(query.size.as_deref())?;
    let loc = system_body_loc(&state, &id, &body, size).await?;
    note_opened(&state, &id, &body);
    body_epoch_layer(state, loc, epoch_ma, file, query).await
}

async fn system_drift_preview(
    State(state): State<Shared>,
    UrlPath((id, body, drift_ma, file)): UrlPath<(String, String, i32, String)>,
    Query(query): Query<DriftQuery>,
) -> Result<Response, Failure> {
    let size = query_size(query.size.as_deref())?;
    let loc = system_body_loc(&state, &id, &body, size).await?;
    note_opened(&state, &id, &body);
    body_drift_preview(state, &id, &body, loc, drift_ma, file, query).await
}

async fn system_thumbnail(
    State(state): State<Shared>,
    UrlPath((id, body)): UrlPath<(String, String)>,
    Query(query): Query<ThumbQuery>,
) -> Result<Response, Failure> {
    // Made from the original resolution, whichever one is displayed.
    let original = original_size(&state, &id, &body).await?;
    let thumb_dir = systems::body_dir(&state.config.systems_dir, &id, &body);
    match system_body_loc(&state, &id, &body, Some(original)).await {
        Ok(loc) => body_thumbnail(state, loc, &thumb_dir, query).await,
        // An evicted body keeps its thumbnail (see [`evict`]), so that
        // its card still looks right.
        Err(Failure(StatusCode::NOT_FOUND, message)) => {
            match tokio::fs::read(thumb_dir.join(mapgen::export::thumbnail::THUMB_FILE)).await {
                Ok(png) => Ok((
                    [
                        (header::CONTENT_TYPE, "image/png"),
                        (header::CACHE_CONTROL, "no-cache"),
                    ],
                    png,
                )
                    .into_response()),
                Err(_) => Err(Failure(StatusCode::NOT_FOUND, message)),
            }
        }
        Err(e) => Err(e),
    }
}

/// Draws the `world.svg` of a body and sends it as a download
/// (`GET /api/systems/{id}/bodies/{body}/world.svg?size=WxH`, the displayed
/// size without one). The server never writes this file: it is drawn here
/// from the stored body, or from its seed and request when the stored data
/// does not read back (see [`mapgen::system::body_svg`]).
async fn system_svg(
    State(state): State<Shared>,
    UrlPath((id, body)): UrlPath<(String, String)>,
    Query(query): Query<SvgQuery>,
) -> Result<Response, Failure> {
    let size = query_size(query.size.as_deref())?;
    let loc = system_body_loc(&state, &id, &body, size).await?;
    let (width, height) = size.unwrap_or_else(|| {
        // The location's own size, for the file name and the drawing.
        loc.dir
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(mapgen::sizes::parse_size_name)
            .unwrap_or((0, 0))
    });
    let system_dir = systems::system_dir(&state.config.systems_dir, &id);
    let owned = body.clone();
    let svg = tokio::task::spawn_blocking(move || {
        mapgen::system::body_svg(&system_dir, &owned, width, height)
    })
    .await
    .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    note_opened(&state, &id, &body);
    let name = format!(
        "{id}-{body}-{}.svg",
        mapgen::sizes::size_name(width, height)
    );
    Ok((
        [
            (header::CONTENT_TYPE, "image/svg+xml".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{name}\""),
            ),
            (header::CACHE_CONTROL, "no-cache".to_string()),
        ],
        svg,
    )
        .into_response())
}

/// Query of an SVG export: the resolution, the displayed one without it.
#[derive(serde::Deserialize)]
struct SvgQuery {
    size: Option<String>,
}

async fn system_hex(
    State(state): State<Shared>,
    UrlPath((id, body)): UrlPath<(String, String)>,
    Query(query): Query<HexQuery>,
    Json(request): Json<HexRequest>,
) -> Result<Response, Failure> {
    let size = query_size(query.size.as_deref())?;
    let loc = system_body_loc(&state, &id, &body, size).await?;
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
    let queued = queue_body_job(&state, &id, &body).await?;
    Ok(Json(match queued {
        Some(q) => GenerateStarted {
            job_id: Some(q.job_id),
            regenerated: q.regenerated,
        },
        None => GenerateStarted {
            job_id: None,
            regenerated: Vec::new(),
        },
    }))
}

/// The resolution a path segment names (`2048x1024`): 400 otherwise.
fn path_size(name: &str) -> Result<(u32, u32), Failure> {
    query_size(Some(name)).map(|size| size.expect("a name was given"))
}

/// The original resolution of a generated body: 404 when it is not
/// generated (see [`original_size`] for the other failures).
async fn generated_original(state: &AppState, id: &str, body: &str) -> Result<(u32, u32), Failure> {
    let original = original_size(state, id, body).await?;
    let root = state.config.systems_dir.clone();
    let (system, owned) = (id.to_string(), body.to_string());
    let generated = tokio::task::spawn_blocking(move || {
        systems::size_revision(&root, &system, &owned, original).is_some()
    })
    .await
    .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if !generated {
        return Err(Failure(
            StatusCode::NOT_FOUND,
            format!("body {body} of {id} is not generated"),
        ));
    }
    Ok(original)
}

/// The resolutions of a generated body
/// (`GET /api/systems/{id}/bodies/{body}/sizes`).
async fn body_sizes(
    State(state): State<Shared>,
    UrlPath((id, body)): UrlPath<(String, String)>,
) -> Result<Json<BodySizes>, Failure> {
    let original = generated_original(&state, &id, &body).await?;
    let (root, index) = (state.config.systems_dir.clone(), state.index.clone());
    let (system, owned) = (id.clone(), body.clone());
    let (present, displayed) = tokio::task::spawn_blocking(move || {
        let body_dir = systems::body_dir(&root, &system, &owned);
        let present: Vec<PresentSize> = mapgen::sizes::sizes_on_disk(&body_dir)
            .into_iter()
            .filter_map(|(width, height)| {
                let revision = systems::size_revision(&root, &system, &owned, (width, height))?;
                Some(PresentSize {
                    width,
                    height,
                    revision,
                })
            })
            .collect();
        let displayed = displayed_size(&index, &root, &system, &owned, original);
        (present, displayed)
    })
    .await
    .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let mut pending: Vec<(u64, PendingSize)> = {
        let jobs = state.jobs.lock().expect("jobs lock");
        jobs.iter()
            .filter(|(_, job)| {
                job.system
                    .as_ref()
                    .is_some_and(|(s, b)| *s == id && *b == body)
                    && !job.status.borrow().is_finished()
            })
            .filter_map(|(job_id, job)| {
                let (width, height) = job.size?;
                Some((
                    job_id.parse().unwrap_or(0),
                    PendingSize {
                        width,
                        height,
                        job_id: job_id.clone(),
                    },
                ))
            })
            .collect()
    };
    pending.sort_unstable_by_key(|(job, _)| *job);
    Ok(Json(BodySizes {
        original: [original.0, original.1],
        displayed: [displayed.0, displayed.1],
        present,
        pending: pending.into_iter().map(|(_, size)| size).collect(),
        presets: request::SIZE_PRESETS.iter().map(|&(w, h)| [w, h]).collect(),
        max_size: MAX_SIZE,
        max_pixels: state.config.max_pixels,
    }))
}

/// Generates one more resolution of a generated body
/// (`POST /api/systems/{id}/bodies/{body}/sizes/{W}x{H}`): no job when it
/// is on disk already, the job already queued when it is being generated.
async fn generate_size(
    State(state): State<Shared>,
    UrlPath((id, body, size)): UrlPath<(String, String, String)>,
) -> Result<Json<GenerateStarted>, Rejection> {
    let size = path_size(&size)?;
    let original = generated_original(&state, &id, &body).await?;
    if size == original {
        return Ok(Json(GenerateStarted {
            job_id: None,
            regenerated: Vec::new(),
        }));
    }
    let (task_state, system, owned) = (state.clone(), id.clone(), body.clone());
    let job_id =
        tokio::task::spawn_blocking(move || claim_size(&task_state, &system, &owned, size))
            .await
            .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))??;
    note_opened(&state, &id, &body);
    Ok(Json(GenerateStarted {
        job_id,
        regenerated: Vec::new(),
    }))
}

/// Under the pending list's lock (so that a size is never both deleted
/// and queued): the job generating `size` of `body` when there is one,
/// none when that size is on disk, else a new job, started at once (so
/// that a request dropped meanwhile leaves no job without a task).
/// Blocking; runs inside the runtime (it spawns the job's task).
fn claim_size(
    state: &Shared,
    id: &str,
    body: &str,
    size: (u32, u32),
) -> Result<Option<String>, Rejection> {
    let key = pending_size(id, body, size);
    let mut pending = state.pending.lock().expect("pending lock");
    if pending.contains(&key) {
        return Ok(unfinished_job(state, id, Some(body), Some(size)));
    }
    if systems::size_revision(&state.config.systems_dir, id, body, size).is_some() {
        return Ok(None);
    }
    check_offered(size, state.config.max_pixels)?;
    pending.push(key.clone());
    let job = register_job(state, id, body, Some(size));
    Ok(Some(start_job(state, key, job)))
}

/// Refuses a resolution that is not offered ([`request::SIZE_PRESETS`]) or
/// larger than this server accepts.
fn check_offered((width, height): (u32, u32), max_pixels: u64) -> Result<(), Rejection> {
    if !request::SIZE_PRESETS.contains(&(width, height)) {
        let offered: Vec<String> = request::SIZE_PRESETS
            .iter()
            .map(|&(w, h)| mapgen::sizes::size_name(w, h))
            .collect();
        return Err(Rejection(
            bad(format!(
                "{width}x{height} is not an offered size: {}",
                offered.join(", ")
            )),
            Some(ErrorDetail::SizeNotOffered { width, height }),
        ));
    }
    let pixels = u64::from(width) * u64::from(height);
    if pixels > max_pixels {
        return Err(Rejection(
            bad(format!(
                "{width} × {height} needs about {:.1} GB of memory; this server accepts up to \
                 {max_pixels} pixels (MAPGEN_MAX_PIXELS)",
                (pixels * BYTES_PER_PIXEL) as f64 / 1e9,
            )),
            Some(ErrorDetail::TooManyPixels {
                width,
                height,
                max_pixels,
            }),
        ));
    }
    Ok(())
}

/// Deletes an added resolution of a body
/// (`DELETE /api/systems/{id}/bodies/{body}/sizes/{W}x{H}`): its files, its
/// cached world and previews; the body is displayed at its original size
/// again if it was displayed at this one.
async fn delete_size(
    State(state): State<Shared>,
    UrlPath((id, body, size)): UrlPath<(String, String, String)>,
) -> Result<StatusCode, Rejection> {
    let size = path_size(&size)?;
    let (width, height) = size;
    let original = original_size(&state, &id, &body).await?;
    if size == original {
        return Err(Rejection(
            Failure(
                StatusCode::CONFLICT,
                format!("{width}x{height} is the original size of {body}: it cannot be deleted"),
            ),
            Some(ErrorDetail::OriginalSize),
        ));
    }
    let (task_state, system, owned) = (state.clone(), id.clone(), body.clone());
    tokio::task::spawn_blocking(move || {
        let state = task_state;
        let key = pending_size(&system, &owned, size);
        let body_dir = systems::body_dir(&state.config.systems_dir, &system, &owned);
        let dir = mapgen::sizes::size_dir(&body_dir, width, height);
        // Moved away under the lock, so that a size is never both deleted
        // and queued; removed after, out of sight of every route (the
        // start's sweep clears what a stop leaves of it).
        let doomed = body_dir.join(mapgen::sizes::SIZES_DIR).join(format!(
            ".tmp-delete-{width}x{height}-{}",
            crate::index::now_ms()
        ));
        {
            let pending = state.pending.lock().expect("pending lock");
            if pending.contains(&key) {
                return Err(Rejection(
                    Failure(
                        StatusCode::CONFLICT,
                        format!(
                            "{width}x{height} of {owned} is being generated: cancel its job first"
                        ),
                    ),
                    Some(ErrorDetail::BeingGenerated { world: key }),
                ));
            }
            if !dir.is_dir() {
                return Err(Rejection(
                    Failure(
                        StatusCode::NOT_FOUND,
                        format!(
                            "size {width}x{height} of body {owned} of {system} is not generated"
                        ),
                    ),
                    Some(ErrorDetail::SizeNotGenerated { width, height }),
                ));
            }
            std::fs::rename(&dir, &doomed)
                .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }
        if let Err(e) = std::fs::remove_dir_all(&doomed) {
            tracing::warn!("could not remove {}: {e}", doomed.display());
        }
        if state.index.displayed_size(&system, &owned).ok().flatten() == Some(size) {
            state
                .index
                .set_displayed_size(&system, &owned, None)
                .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }
        Ok(())
    })
    .await
    .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))??;
    let key = size_key(&id, &body, size);
    let forgotten = state.tiles.forget(&key);
    state.drift_cache.forget(&key);
    if forgotten {
        let _ = tokio::task::spawn_blocking(memory::release).await;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Remembers the resolution a body is displayed at
/// (`PUT /api/systems/{id}/bodies/{body}/displayed-size`): the default of
/// its tiles, layers and hex grids, in every browser.
async fn set_displayed_size(
    State(state): State<Shared>,
    UrlPath((id, body)): UrlPath<(String, String)>,
    Json(request): Json<DisplayedSize>,
) -> Result<StatusCode, Rejection> {
    let size = (request.width, request.height);
    let original = original_size(&state, &id, &body).await?;
    let (root, index) = (state.config.systems_dir.clone(), state.index.clone());
    let (system, owned) = (id.clone(), body.clone());
    let present = tokio::task::spawn_blocking(move || {
        if systems::size_revision(&root, &system, &owned, size).is_none() {
            return Ok(false);
        }
        // The original is the default: nothing to remember.
        let remembered = (size != original).then_some(size);
        index
            .set_displayed_size(&system, &owned, remembered)
            .map(|()| true)
    })
    .await
    .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if !present {
        let (width, height) = size;
        return Err(Rejection(
            Failure(
                StatusCode::NOT_FOUND,
                format!("size {width}x{height} of body {body} of {id} is not generated"),
            ),
            Some(ErrorDetail::SizeNotGenerated { width, height }),
        ));
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Query of a tile request: the world revision and the renderer version
/// the client knows, the epoch shown (Ma before present, negative in the
/// future; the world as generated without it) and the resolution (see
/// [`query_size`]).
#[derive(serde::Deserialize)]
struct TileQuery {
    rev: Option<u64>,
    render: Option<u32>,
    epoch: Option<i32>,
    size: Option<String>,
}

/// Query of a hex grid request: the resolution (see [`query_size`]).
#[derive(serde::Deserialize)]
struct HexQuery {
    size: Option<String>,
}

/// The resolution a `?size=<W>x<H>` query names (see
/// [`mapgen::sizes::parse_size_name`]); `None` without one. 400 for a
/// name that is not a size.
fn query_size(size: Option<&str>) -> Result<Option<(u32, u32)>, Failure> {
    size.map(|name| {
        mapgen::sizes::parse_size_name(name).ok_or_else(|| {
            Failure(
                StatusCode::BAD_REQUEST,
                format!("invalid size {name:?}: <width>x<height>, such as 2048x1024"),
            )
        })
    })
    .transpose()
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
                 or was made by an older generator (delete it with `mise run clean-systems`)"
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

/// Where one resolution of a body is, the key of its world in the tile
/// cache, and its revision (which keys its cached tiles in browsers).
struct BodyLoc {
    /// The resolution's directory, `bodies/<body>/sizes/<W>x<H>/`.
    dir: PathBuf,
    /// `systems/<id>/<body>/<W>x<H>`: each resolution is a world of its own
    /// in the tile cache, and forgetting `systems/<id>` forgets them all.
    key: String,
    revision: u64,
}

/// The original resolution of body `body` of system `id` (see
/// [`systems::OriginalSizes`]): 404 when the system has no such body,
/// 404 or 410 when its `system.json` is missing or unreadable.
async fn original_size(state: &AppState, id: &str, body: &str) -> Result<(u32, u32), Failure> {
    if !systems::is_valid_system_id(id) || !systems::is_valid_body_id(body) {
        return Err(Failure(StatusCode::BAD_REQUEST, "invalid body".into()));
    }
    let (originals, root) = (state.originals.clone(), state.config.systems_dir.clone());
    let (system, owned) = (id.to_string(), body.to_string());
    tokio::task::spawn_blocking(move || originals.original_size(&root, &system, &owned))
        .await
        .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map_err(|e| unreadable_system(&state.config.systems_dir, id, &e))?
        .ok_or_else(|| {
            Failure(
                StatusCode::NOT_FOUND,
                format!("body {body} of {id} is not generated"),
            )
        })
}

/// The `size` resolution of a generated body of a system, in
/// `systems/<id>/bodies/<body>/sizes/<W>x<H>/`, with its own revision, so
/// that generating another body or another size keeps its tiles. Without
/// a size, the displayed one (see [`displayed_size`]). 404 when that size
/// is not generated.
async fn system_body_loc(
    state: &AppState,
    id: &str,
    body: &str,
    size: Option<(u32, u32)>,
) -> Result<BodyLoc, Failure> {
    let original = original_size(state, id, body).await?;
    let (root, index) = (state.config.systems_dir.clone(), state.index.clone());
    let (system, owned) = (id.to_string(), body.to_string());
    let (size, revision) = tokio::task::spawn_blocking(move || {
        let size = size.unwrap_or_else(|| displayed_size(&index, &root, &system, &owned, original));
        (size, systems::size_revision(&root, &system, &owned, size))
    })
    .await
    .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let name = mapgen::sizes::size_name(size.0, size.1);
    let revision = revision.ok_or_else(|| {
        let message = if size == original {
            format!("body {body} of {id} is not generated")
        } else {
            format!("size {name} of body {body} of {id} is not generated")
        };
        Failure(StatusCode::NOT_FOUND, message)
    })?;
    let body_dir = systems::body_dir(&state.config.systems_dir, id, body);
    Ok(BodyLoc {
        dir: mapgen::sizes::size_dir(&body_dir, size.0, size.1),
        key: size_key(id, body, size),
        revision,
    })
}

/// The key of a body's resolution in the tile and preview caches:
/// `systems/<id>/<body>/<W>x<H>`.
fn size_key(id: &str, body: &str, size: (u32, u32)) -> String {
    format!(
        "systems/{id}/{body}/{}",
        mapgen::sizes::size_name(size.0, size.1)
    )
}

/// The resolution body `body` of system `id` is displayed at: the one the
/// index remembers while it is on disk, else `original`. Blocking.
fn displayed_size(
    index: &Index,
    root: &Path,
    id: &str,
    body: &str,
    original: (u32, u32),
) -> (u32, u32) {
    let remembered = index.displayed_size(id, body).unwrap_or_else(|e| {
        tracing::warn!("displayed size of {id}/{body} not read: {e}");
        None
    });
    remembered
        .filter(|&size| systems::size_revision(root, id, body, size).is_some())
        .unwrap_or(original)
}

/// A detail tile of a body of a system.
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
        ..
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
/// version the client knows, a reduction factor (1, 2 or 4) for the
/// lighter layers shown while the time slider moves, and the resolution
/// (see [`query_size`]).
#[derive(serde::Deserialize)]
struct EpochQuery {
    rev: Option<u64>,
    render: Option<u32>,
    scale: Option<u32>,
    size: Option<String>,
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
            format!("no layer {file}: colour.png, normal.png, elevation.png or biomes.png"),
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
        ..
    } = loc;
    let epoch = checked_epoch(&dir, &id, Some(epoch_ma)).await?;
    let world = match state.tiles.world_at(&dir, &id, revision, epoch).await {
        Ok(world) => world,
        Err(e) => return Ok(world_failure(e, "epochs")),
    };
    // The colour and the normals decode the whole world and render it at
    // full size, outside the tile cache's budget (several GB at 8192 x 4096
    // if four ran together): one at a time. The other layers read the
    // cached world.
    let semaphore = if matches!(layer, tiles::EpochLayer::Colour | tiles::EpochLayer::Normal) {
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

/// Query of a drift preview request: the width (256 or 512, default 512),
/// the world revision the client knows and the resolution whose revision
/// it is (see [`query_size`]; the preview itself is generated from the
/// body's parameters, whatever the resolution).
#[derive(serde::Deserialize)]
struct DriftQuery {
    rev: Option<u64>,
    width: Option<u32>,
    size: Option<String>,
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
    // The calibrated parameters, when kept; otherwise the body, whose
    // drawn parameters check the request at once (the calibration keeps
    // its plates) and whose calibration waits for a turn.
    let cached = state.drift_cache.params(&loc.key, loc.revision);
    let found = match &cached {
        Some(_) => None,
        None => {
            let root = state.config.systems_dir.clone();
            let (owned_system, owned_body) = (system.to_owned(), body.to_owned());
            let found = tokio::task::spawn_blocking(move || {
                systems::read_body(&root, &owned_system, &owned_body)
            })
            .await
            .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or_else(|| {
                Failure(
                    StatusCode::NOT_FOUND,
                    format!("no parameters for {}", loc.key),
                )
            })?;
            Some(found)
        }
    };
    // Refused before waiting for a turn.
    let refused = |e: PreviewError| match e {
        PreviewError::NoPlates => Failure(StatusCode::NOT_FOUND, e.to_string()),
        PreviewError::OutOfRange | PreviewError::BadWidth => {
            Failure(StatusCode::BAD_REQUEST, e.to_string())
        }
    };
    let checked = match (&cached, &found) {
        (Some(params), _) => drift::preview_params(params, drift_ma as f32, width),
        (None, Some(body)) => {
            drift::preview_params(&mapgen::system::world_params(body), drift_ma as f32, width)
        }
        (None, None) => unreachable!("a body read when no parameters are kept"),
    };
    checked.map_err(refused)?;
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
    // (and the calibration of their parameters) wait for it to end, as
    // tiles do.
    let Ok(_generation) = state.queue.try_acquire() else {
        return Ok(world_failure(TileError::Busy, "drift previews"));
    };
    // Calibrated once per revision, whatever the epoch (another request
    // may have kept them while this one waited).
    let params = match cached.or_else(|| state.drift_cache.params(&loc.key, loc.revision)) {
        Some(params) => params,
        None => {
            let body = found.expect("a body read when no parameters are kept");
            let params = tokio::task::spawn_blocking(move || {
                Arc::new(mapgen::system::generation_params(&body))
            })
            .await
            .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            state
                .drift_cache
                .insert_params(loc.key.clone(), loc.revision, params.clone());
            params
        }
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
    thumb_dir: &Path,
    query: ThumbQuery,
) -> Result<Response, Failure> {
    let BodyLoc {
        dir,
        key: id,
        revision,
        ..
    } = loc;
    let png = state
        .thumbnails
        .get(thumb_dir, &dir.join("world.png"))
        .await
        .map_err(|e| match e {
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
        ..
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
    Ok(queue_body_job(state, id, body).await?.map(|q| q.job_id))
}

/// The job [`queue_body_job`] answers.
struct QueuedBody {
    job_id: String,
    /// The bodies of the family the job makes again that another
    /// generator made, read before the job could make them (once it is
    /// done they are generated, and nothing is said). Empty when the job
    /// answered is another family's, which leaves `body` alone.
    regenerated: Vec<RegeneratedBody>,
}

/// [`queue_body`], saying whether the job answered is for `body` itself.
async fn queue_body_job(
    state: &Shared,
    id: &str,
    body: &str,
) -> Result<Option<QueuedBody>, Rejection> {
    let dir = systems::system_dir(&state.config.systems_dir, id);
    let read = dir.clone();
    let system = tokio::task::spawn_blocking(move || mapgen::system::read_system(&read))
        .await
        .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map_err(|e| unreadable_system(&state.config.systems_dir, id, &e))?;
    if !system.bodies.iter().any(|b| b.id == body) {
        return Err(Failure(
            StatusCode::NOT_FOUND,
            format!("system {id} has no body {body}"),
        )
        .into());
    }
    note_opened(state, id, body);
    // Before the job: once it is made the bodies are generated, and
    // nothing is said. While it runs they are still evicted, so every
    // request for the family says it.
    let regenerated = older_evicted(state, id, mapgen::system::family(&system, body)).await;
    let key = pending_system(id);
    let job = {
        let mut pending = state.pending.lock().expect("pending lock");
        if pending.contains(&key) {
            return Ok(unfinished_system_job(state, id).map(|(job_id, job_body)| {
                // The job's family: it made the bodies it was started for
                // already marked generated in `system.json` too.
                let own = mapgen::system::family(&system, &job_body)
                    .iter()
                    .any(|b| b == body);
                QueuedBody {
                    job_id,
                    regenerated: if own { regenerated } else { Vec::new() },
                }
            }));
        }
        if mapgen::system::neighbourhood(&system, body).is_empty() {
            return Ok(None);
        }
        pending.push(key.clone());
        let job = register_job(state, id, body, None);
        start_job(state, key, job)
    };
    Ok(Some(QueuedBody {
        job_id: job,
        regenerated,
    }))
}

/// Of `bodies` of system `id`, those evicted whose generator is not the
/// current one, with what made them.
async fn older_evicted(state: &Shared, id: &str, bodies: Vec<String>) -> Vec<RegeneratedBody> {
    let (index, system) = (state.index.clone(), id.to_string());
    tokio::task::spawn_blocking(move || {
        bodies
            .into_iter()
            .filter_map(|body| {
                let old = index
                    .evicted_generator(&system, &body)
                    .unwrap_or_else(|e| {
                        tracing::warn!("generator of {system}/{body} not read: {e}");
                        None
                    })?;
                old.differs_from_current().then(|| RegeneratedBody {
                    body,
                    from: GeneratorNote {
                        version: old.version,
                        format: old.format,
                        current_version: GENERATOR_VERSION.to_string(),
                        current_format: GENERATOR_FORMAT,
                    },
                })
            })
            .collect()
    })
    .await
    .unwrap_or_default()
}

/// The id of the unfinished job of system `id` generating `size` of
/// `body` (any body when `None`), or its own job when `size` is `None`.
fn unfinished_job(
    state: &AppState,
    id: &str,
    body: Option<&str>,
    size: Option<(u32, u32)>,
) -> Option<String> {
    let jobs = state.jobs.lock().expect("jobs lock");
    jobs.iter()
        .find(|(_, job)| {
            job.system
                .as_ref()
                .is_some_and(|(s, b)| s == id && body.is_none_or(|body| b == body))
                && job.size == size
                && !job.status.borrow().is_finished()
        })
        .map(|(job_id, _)| job_id.clone())
}

/// The unfinished job of system `id` for a body and its neighbourhood (not
/// a resolution): its id and the body it was asked for.
fn unfinished_system_job(state: &AppState, id: &str) -> Option<(String, String)> {
    let jobs = state.jobs.lock().expect("jobs lock");
    jobs.iter().find_map(|(job_id, job)| {
        let (system, body) = job.system.as_ref()?;
        (system == id && job.size.is_none() && !job.status.borrow().is_finished())
            .then(|| (job_id.clone(), body.clone()))
    })
}

/// A job listed (see [`register_job`]), not started yet.
struct NewJob {
    number: u64,
    id: String,
    body: String,
    size: Option<(u32, u32)>,
    tx: watch::Sender<JobStatus>,
    cancel: Arc<Cancel>,
}

/// Lists a new queued job of system `id`: `body` and its neighbourhood, or
/// its `size` resolution alone. Called under the pending list's lock, with
/// the job's key pushed, so that a second request for the same generation
/// always finds the job ([`unfinished_job`]).
fn register_job(state: &AppState, id: &str, body: &str, size: Option<(u32, u32)>) -> NewJob {
    let number = state.next_job.fetch_add(1, Ordering::Relaxed);
    let (tx, rx) = watch::channel(JobStatus::Queued);
    let cancel = Arc::new(Cancel::default());
    let mut jobs = state.jobs.lock().expect("jobs lock");
    forget_old_jobs(&mut jobs);
    jobs.insert(
        number.to_string(),
        Job {
            world_id: id.to_string(),
            system: Some((id.to_string(), body.to_string())),
            size,
            request: WorldRequest::default(),
            status: rx,
            cancel: cancel.clone(),
        },
    );
    NewJob {
        number,
        id: id.to_string(),
        body: body.to_string(),
        size,
        tx,
        cancel,
    }
}

/// Starts the task of a job listed by [`register_job`] (it stores the
/// job, then waits for its turn), whose `key` is in the pending list
/// already. Call it right after [`register_job`], with no `.await` in
/// between: `body` and its neighbourhood, or its `size` resolution
/// alone. Every job waits for its turn on the one generation queue, so a
/// resolution is never generated while its system's own job runs. Answers
/// the job's id.
fn start_job(state: &Shared, key: String, job: NewJob) -> String {
    let NewJob {
        number: job_number,
        id,
        body,
        size,
        tx,
        cancel,
    } = job;
    let (id, body) = (id.as_str(), body.as_str());
    let dir = systems::system_dir(&state.config.systems_dir, id);
    let job_id = job_number.to_string();
    let task_state = state.clone();
    let (task_system, task_body) = (id.to_string(), body.to_string());
    // Spawned at once, with no wait between the listing of the job and
    // here: a request dropped by its client must not leave a job listed
    // and pending with no task to run it.
    tokio::spawn(async move {
        {
            let (index, owned, owned_body) = (
                task_state.index.clone(),
                task_system.clone(),
                task_body.clone(),
            );
            let queued = tokio::task::spawn_blocking(move || {
                index.job_queued(
                    job_number,
                    &owned,
                    Some((&owned, &owned_body)),
                    &WorldRequest::default(),
                    size,
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
        // Stored before it is announced, so that a restart right after
        // finds what a client saw. `None` is a job that starts running.
        let store = |status: Option<JobStatus>| {
            let (index, root, system) = (
                task_state.index.clone(),
                task_state.config.systems_dir.clone(),
                task_system.clone(),
            );
            let body = task_body.clone();
            async move {
                let stored = tokio::task::spawn_blocking(move || match status {
                    None => index.job_running(job_number),
                    Some(status) => {
                        // What the job generated is in the tree: read it.
                        if status.is_finished() {
                            index.refresh_system(&root, &system, Cause::Generated)?;
                        }
                        // A resolution that is ready becomes the one
                        // displayed, whether or not a tab follows the job.
                        if let (JobStatus::Done { .. }, Some(size)) = (&status, size) {
                            index.set_displayed_size(&system, &body, Some(size))?;
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
                let what = size.map_or(String::new(), |(w, h)| format!(" at {w}x{h}"));
                tracing::error!("generation of {task_system}/{task_body}{what} failed: {error}");
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
        let diagnostics = mapgen::export::Diagnostics {
            svg: false,
            noise_maps: task_state.config.noise_maps,
        };
        let index = task_state.index.clone();
        let task_system_id = task_system.clone();
        let result = tokio::task::spawn_blocking(move || {
            let report = |p: mapgen::system::BodyProgress| {
                // A cancelled job stops when the next stage starts; the
                // bodies done before it stay generated (a resolution
                // stopped midway is removed below).
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
            };
            match size {
                Some((width, height)) => {
                    mapgen::system::generate_size(&dir, &body, width, height, diagnostics, report)
                        .map(|()| vec![body.clone()])
                }
                None => {
                    // Bodies that were evicted coming back do not move the
                    // system in the library; put back when this ends,
                    // however it ends.
                    let _date =
                        evict::KeptDate::when_only_restoring(&index, &dir, &task_system_id, &body);
                    mapgen::system::generate_neighbourhood_with(&dir, &body, diagnostics, report)
                }
            }
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
        // A resolution cancelled or failed midway: its staging directory
        // can hold GBs (`generate_size` removes it on an error, not on the
        // cancel's unwind). Still under the job's pending key, so no other
        // job writes it.
        if let (Some((width, height)), false) = (size, matches!(status, JobStatus::Done { .. })) {
            let staging =
                systems::body_dir(&task_state.config.systems_dir, &task_system, &task_body)
                    .join(mapgen::sizes::SIZES_DIR)
                    .join(format!(".tmp-{}", mapgen::sizes::size_name(width, height)));
            let _ = tokio::task::spawn_blocking(move || {
                if let Err(e) = std::fs::remove_dir_all(&staging)
                    && e.kind() != std::io::ErrorKind::NotFound
                {
                    tracing::warn!("could not remove {}: {e}", staging.display());
                }
            })
            .await;
        }
        let _ = tokio::task::spawn_blocking(memory::release).await;
        drop(paused_tiles);
        store(Some(status.clone())).await;
        // The body asked for, made again after an eviction, comes back at
        // the resolution it was displayed at, not only at its original.
        // Its neighbours (moons, giant, siblings) made by the same job stay
        // at their original size: nobody opened them.
        let reopened = matches!(
            (&status, size),
            (JobStatus::Done { bodies, .. }, None) if bodies.contains(&task_body)
        );
        finish(status);
        if reopened {
            let (state, system, opened) =
                (task_state.clone(), task_system.clone(), task_body.clone());
            let _ = tokio::task::spawn_blocking(move || {
                regenerate_displayed_size(&state, &system, &opened);
            })
            .await;
        }
    });
    job_id
}

/// Queues the displayed resolution of `body` when the index remembers one
/// that is not on disk and not the original: a body generated again after
/// its eviction (see [`evict`]) comes back at the size it was shown at.
/// The job marks it displayed when it is done. Blocking; runs inside the
/// runtime (it spawns the job's task).
fn regenerate_displayed_size(state: &Shared, id: &str, body: &str) {
    let root = &state.config.systems_dir;
    let remembered = state.index.displayed_size(id, body).unwrap_or_else(|e| {
        tracing::warn!("displayed size of {id}/{body} not read: {e}");
        None
    });
    let Some(size) = remembered else {
        return;
    };
    if systems::size_revision(root, id, body, size).is_some() {
        return;
    }
    if let Err(Rejection(Failure(_, error), _)) = claim_size(state, id, body, size) {
        tracing::warn!(
            "{id}/{body} not generated again at {}: {error}",
            mapgen::sizes::size_name(size.0, size.1)
        );
    }
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
                    size: job.size.map(|(w, h)| [w, h]),
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

    #[test]
    fn viewer_and_server_agree_on_the_planet_classes() {
        let viewer = include_str!("../../../web/src/lib/planetClass.test.ts");
        for key in mapgen::planet::PLANET_CLASSES {
            assert!(
                viewer.contains(&format!("'{key}'")),
                "planetClass.test.ts must list the class `{key}`"
            );
        }
        let listed = viewer.matches("\n  '").count();
        assert_eq!(
            listed,
            mapgen::planet::PLANET_CLASSES.len(),
            "planetClass.test.ts lists a class the server does not know"
        );
    }

    #[test]
    fn viewer_and_server_agree_on_the_oldest_format() {
        let viewer = include_str!("../../../web/src/lib/formatVersion.ts");
        let min = mapgen::export::raw_data::MIN_FORMAT_VERSION;
        let expected = format!("export const MIN_FORMAT_VERSION = {min};");
        assert!(
            viewer.contains(&expected),
            "formatVersion.ts must declare `{expected}`"
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
            noise_maps: false,
            evict_after_days: 0,
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
    async fn answers_410_for_what_an_older_generator_made() {
        let config = temp_config("too-old");
        let root = config.systems_dir.clone();
        let app = router(config);
        // A system whose file no longer parses, and a world below the
        // oldest format: gone, with the way out, not "no system" or a 500.
        let dir = root.join("mgs-1-1");
        std::fs::create_dir_all(dir.join("bodies/b/raw_data")).unwrap();
        std::fs::write(
            dir.join("system.json"),
            r#"{"format_version": 1, "bodies": 7}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("bodies/b/raw_data/world.json"),
            r#"{"format_version": 10}"#,
        )
        .unwrap();
        let (status, body) = call(&app, "GET", "/api/systems/mgs-1-1", None).await;
        assert_eq!(status, StatusCode::GONE, "{body}");
        assert!(body.contains("clean-systems"), "{body}");
        let (status, body) = call(
            &app,
            "GET",
            "/api/systems/mgs-1-1/bodies/b/tiles/0/0/0",
            None,
        )
        .await;
        assert_eq!(status, StatusCode::GONE, "{body}");
        assert!(body.contains("clean-systems"), "{body}");
        // No such system stays a 404.
        let (status, _) = call(&app, "GET", "/api/systems/mgs-2-2", None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        // A system of today's format holding a world below the oldest one.
        let dir = root.join("mgs-3-3");
        std::fs::create_dir_all(dir.join("bodies/b/sizes/64x32/raw_data")).unwrap();
        let version = mapgen::system::SYSTEM_FORMAT_VERSION;
        std::fs::write(
            dir.join("system.json"),
            format!(
                r#"{{"format_version": {version}, "width": 64, "height": 32,
                     "bodies": [{{"id": "b", "params": {{"width": 64, "height": 32}}}}]}}"#
            ),
        )
        .unwrap();
        std::fs::write(
            dir.join("bodies/b/sizes/64x32/raw_data/world.json"),
            r#"{"format_version": 10}"#,
        )
        .unwrap();
        let (status, body) = call(
            &app,
            "GET",
            "/api/systems/mgs-3-3/bodies/b/tiles/0/0/0",
            None,
        )
        .await;
        assert_eq!(status, StatusCode::GONE, "{body}");
        assert!(body.contains("clean-systems"), "{body}");
    }

    /// Writes a `system.json` of format 2 (an older generator's) under
    /// `root` as system `id`.
    fn write_outdated_system(root: &Path, id: &str) {
        let dir = root.join(id);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("system.json"),
            r#"{"format_version": 2, "seed": 7, "width": 64, "height": 32,
                "star": {"type": "G2V", "designation": "MGS 7"}, "bodies": []}"#,
        )
        .unwrap();
    }

    #[tokio::test]
    async fn an_outdated_system_is_listed_not_deleted() {
        let config = temp_config("outdated-listed");
        let root = config.systems_dir.clone();
        write_outdated_system(&root, "mgs-7-7");
        let app = router(config);
        let (status, body) = call(&app, "GET", "/api/systems", None).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let list: serde_json::Value = serde_json::from_str(&body).unwrap();
        let system = &list.as_array().unwrap()[0];
        assert_eq!(system["id"], "mgs-7-7");
        assert_eq!(system["outdated"], true);
        assert_eq!(system["seed"], 7);
        assert_eq!(system["star"], "G2V");
        assert!(system["planets"].as_object().unwrap().is_empty());
        assert!(root.join("mgs-7-7").is_dir());
    }

    #[tokio::test]
    async fn an_outdated_system_can_be_deleted_by_request() {
        let config = temp_config("outdated-deleted");
        let root = config.systems_dir.clone();
        write_outdated_system(&root, "mgs-7-7");
        let app = router(config);
        let (status, body) = call(&app, "DELETE", "/api/systems/mgs-7-7", None).await;
        assert_eq!(status, StatusCode::NO_CONTENT, "{body}");
        assert!(!root.join("mgs-7-7").exists());
        let (_, body) = call(&app, "GET", "/api/systems", None).await;
        assert_eq!(body.trim(), "[]");
    }

    #[tokio::test]
    async fn opening_an_outdated_system_names_clean_systems() {
        let config = temp_config("outdated-opened");
        let root = config.systems_dir.clone();
        write_outdated_system(&root, "mgs-7-7");
        let app = router(config);
        let (status, body) = call(&app, "GET", "/api/systems/mgs-7-7", None).await;
        assert_eq!(status, StatusCode::GONE, "{body}");
        assert!(body.contains("clean-systems"), "{body}");
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
            r#", "surface_age_ga": 4.0, "age_gyr": 6.0"#,
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
        let stored = image::open(
            systems::body_dir(&root, &old, &rock).join("sizes/256x128/raw_data/elevation.png"),
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
        for file in ["colour.png", "normal.png", "elevation.png", "biomes.png"] {
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
            (format!("{base}/4100/colour.png"), StatusCode::BAD_REQUEST),
            (format!("{base}/-1001/colour.png"), StatusCode::BAD_REQUEST),
            (
                format!("{wet_base}/epoch/100/colour.png"),
                StatusCode::NOT_FOUND,
            ),
            (format!("{base}/100/flow.png"), StatusCode::NOT_FOUND),
            // The hillshaded surface is gone: the viewer lights the relief.
            (format!("{base}/100/surface.png"), StatusCode::NOT_FOUND),
            (
                format!("{base}/100/colour.png?scale=3"),
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
    async fn generates_no_svg_and_no_noise_maps_by_default() {
        let config = temp_config("no-diagnostics");
        let root = config.systems_dir.clone();
        let app = router(config);
        let (id, planet) = generated_planet(&app, "terran", 7, "").await;
        let size = mapgen::sizes::size_dir(&systems::body_dir(&root, &id, &planet), 256, 128);
        assert!(size.join("world.png").is_file());
        assert!(size.join("legend.png").is_file());
        assert!(!size.join("world.svg").exists());
        assert!(!size.join("noise_maps").exists());
    }

    #[tokio::test]
    async fn the_debug_option_writes_the_noise_maps() {
        let config = Config {
            noise_maps: true,
            ..temp_config("noise-maps")
        };
        let root = config.systems_dir.clone();
        let app = router(config);
        let (id, planet) = generated_planet(&app, "terran", 7, "").await;
        let size = mapgen::sizes::size_dir(&systems::body_dir(&root, &id, &planet), 256, 128);
        assert!(size.join("noise_maps").read_dir().unwrap().next().is_some());
        // The SVG stays on request.
        assert!(!size.join("world.svg").exists());
    }

    #[tokio::test]
    async fn exports_the_svg_of_a_body_on_request() {
        let config = temp_config("svg-export");
        let root = config.systems_dir.clone();
        let app = router(config);
        let (id, planet) = generated_planet(&app, "terran", 7, "").await;
        let uri = format!("/api/systems/{id}/bodies/{planet}/world.svg");
        let get = |uri: String| {
            let app = app.clone();
            async move {
                let response = app
                    .oneshot(Request::get(&uri).body(Body::empty()).unwrap())
                    .await
                    .unwrap();
                let (status, headers) = (response.status(), response.headers().clone());
                let bytes = response.into_body().collect().await.unwrap().to_bytes();
                (status, headers, bytes)
            }
        };
        let (status, headers, bytes) = get(uri.clone()).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(headers["content-type"], "image/svg+xml");
        assert!(
            headers["content-disposition"]
                .to_str()
                .unwrap()
                .contains(".svg")
        );
        let text = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(text.starts_with("<?xml") && text.contains(r#"width="256" height="128""#));
        // Drawn on request: nothing is stored.
        let size = mapgen::sizes::size_dir(&systems::body_dir(&root, &id, &planet), 256, 128);
        assert!(!size.join("world.svg").exists());
        // Unknown size, unknown body, bad size name.
        assert_eq!(
            get(format!("{uri}?size=1024x512")).await.0,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            get(format!("{uri}?size=x")).await.0,
            StatusCode::BAD_REQUEST
        );
        let other = format!("/api/systems/{id}/bodies/zz/world.svg");
        assert_eq!(get(other).await.0, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn serves_each_resolution_of_a_body() {
        let config = temp_config("sizes");
        let root = config.systems_dir.clone();
        let app = router(config);
        let (id, planet) = generated_planet(&app, "terran", 7, "").await;
        let system_dir = systems::system_dir(&root, &id);
        let owned = planet.clone();
        tokio::task::spawn_blocking(move || {
            mapgen::system::generate_size(
                &system_dir,
                &owned,
                512,
                256,
                mapgen::export::Diagnostics::default(),
                |_| {},
            )
        })
        .await
        .unwrap()
        .unwrap();
        let get = |uri: String| {
            let app = app.clone();
            async move {
                let request = Request::get(&uri).body(Body::empty()).unwrap();
                let response = app.oneshot(request).await.unwrap();
                let status = response.status();
                let bytes = response.into_body().collect().await.unwrap().to_bytes();
                (status, bytes)
            }
        };
        let base = format!("/api/systems/{id}/bodies/{planet}");
        let tile = |size: &str| format!("{base}/tiles/0/0/0?size={size}");
        // Each size its own tiles, whatever the order they are asked in:
        // the tile cache keys a world by its size.
        let (status, large) = get(tile("512x256")).await;
        assert_eq!(status, StatusCode::OK);
        let (status, small) = get(tile("256x128")).await;
        assert_eq!(status, StatusCode::OK);
        assert_ne!(large, small);
        assert_eq!(get(tile("512x256")).await.1, large);
        assert_eq!(get(tile("256x128")).await.1, small);
        // Without a size, the original one.
        assert_eq!(get(format!("{base}/tiles/0/0/0")).await.1, small);
        // A size not generated, and a name that is not a size.
        for (uri, expected) in [
            (tile("1024x512"), StatusCode::NOT_FOUND),
            (tile("512X256"), StatusCode::BAD_REQUEST),
            (tile("0512x256"), StatusCode::BAD_REQUEST),
            (
                format!("{base}/epoch/0/elevation.png?size=1024x512"),
                StatusCode::NOT_FOUND,
            ),
            (
                format!("{base}/epoch/0/elevation.png?size=x"),
                StatusCode::BAD_REQUEST,
            ),
            (
                format!("{base}/drift/10/surface.png?size=512X256"),
                StatusCode::BAD_REQUEST,
            ),
        ] {
            assert_eq!(get(uri.clone()).await.0, expected, "{uri}");
        }
        // The layers of a size are that size's.
        let (status, layer) = get(format!("{base}/epoch/0/elevation.png?size=512x256")).await;
        assert_eq!(status, StatusCode::OK);
        let layer = image::load_from_memory(&layer).unwrap();
        assert_eq!((layer.width(), layer.height()), (512, 256));
        // The hex grid of a size.
        let hex = |size: &str| {
            let app = app.clone();
            let uri = format!("{base}/hex?size={size}");
            async move {
                let request = Request::post(&uri)
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"columns": 8}"#))
                    .unwrap();
                app.oneshot(request).await.unwrap().status()
            }
        };
        assert_eq!(hex("512x256").await, StatusCode::OK);
        assert_eq!(hex("1024x512").await, StatusCode::NOT_FOUND);
        assert_eq!(hex("512X256").await, StatusCode::BAD_REQUEST);
        // The thumbnail is the body's, beside its sizes, and made again
        // from the original size when it is gone.
        let thumb = systems::body_dir(&root, &id, &planet).join("thumb.png");
        let (status, bytes) = get(format!("{base}/thumb")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(bytes, std::fs::read(&thumb).unwrap());
        std::fs::remove_file(&thumb).unwrap();
        let (status, _) = get(format!("{base}/thumb")).await;
        assert_eq!(status, StatusCode::OK);
        assert!(thumb.is_file());
    }

    #[tokio::test]
    async fn sizes_are_generated_listed_remembered_and_deleted() {
        let app = router(temp_config("size-jobs"));
        // One pixel short of the smallest offered size.
        let tight = Config {
            max_pixels: 1024 * 512 - 1,
            ..temp_config("size-jobs-tight")
        };
        let (id, planet) = generated_planet(&app, "barren", 9, "").await;
        let base = format!("/api/systems/{id}/bodies/{planet}");
        let json = |body: &str| serde_json::from_str::<serde_json::Value>(body).unwrap();
        let sizes = || {
            let (app, uri) = (app.clone(), format!("{base}/sizes"));
            async move {
                let (status, body) = call(&app, "GET", &uri, None).await;
                assert_eq!(status, StatusCode::OK, "{body}");
                json(&body)
            }
        };
        let listed = sizes().await;
        assert_eq!(listed["original"], serde_json::json!([256, 128]));
        assert_eq!(listed["displayed"], serde_json::json!([256, 128]));
        assert_eq!(listed["present"].as_array().unwrap().len(), 1);
        assert_eq!(listed["present"][0]["width"], 256);
        assert!(listed["present"][0]["revision"].as_u64().unwrap() > 0);
        assert_eq!(listed["pending"], serde_json::json!([]));
        assert_eq!(
            listed["presets"],
            serde_json::json!([[1024, 512], [2048, 1024], [4096, 2048], [8192, 4096]])
        );
        assert_eq!(listed["max_size"], MAX_SIZE);
        assert_eq!(listed["max_pixels"], DEFAULT_MAX_PIXELS);

        // Asked twice at once, then once more: one job.
        let post = format!("{base}/sizes/1024x512");
        let ((status, first), (other, second)) = tokio::join!(
            call(&app, "POST", &post, None),
            call(&app, "POST", &post, None)
        );
        assert_eq!(status, StatusCode::OK, "{first}");
        assert_eq!(other, StatusCode::OK, "{second}");
        let job = json(&first)["job_id"].as_str().unwrap().to_string();
        assert_eq!(json(&second)["job_id"], job.as_str());
        let (_, third) = call(&app, "POST", &post, None).await;
        assert_eq!(json(&third)["job_id"], job.as_str());
        // Listed as pending meanwhile, the job naming its size.
        let pending = sizes().await["pending"].clone();
        if pending.as_array().unwrap().is_empty() {
            // Finished already: nothing to check.
        } else {
            assert_eq!(pending[0]["job_id"], job.as_str());
            assert_eq!(pending[0]["width"], 1024);
        }
        let (_, jobs) = call(&app, "GET", "/api/jobs", None).await;
        assert_eq!(json(&jobs)[0]["size"], serde_json::json!([1024, 512]));
        // Neither the system nor its size can go while it is generated.
        let (status, _) = call(&app, "DELETE", &format!("/api/systems/{id}"), None).await;
        assert_eq!(status, StatusCode::CONFLICT);
        let events = wait_job(&app, &first).await;
        assert!(events.contains("\"status\":\"done\""), "{events}");
        let listed = sizes().await;
        let present: Vec<[u64; 2]> = listed["present"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| [p["width"].as_u64().unwrap(), p["height"].as_u64().unwrap()])
            .collect();
        assert_eq!(present, [[256, 128], [1024, 512]]);
        assert_eq!(listed["pending"], serde_json::json!([]));
        // The finished size is the displayed one, with no PUT and no tab.
        assert_eq!(listed["displayed"], serde_json::json!([1024, 512]));
        // Present: no job.
        let (status, again) = call(&app, "POST", &post, None).await;
        assert_eq!(status, StatusCode::OK);
        assert!(json(&again)["job_id"].is_null(), "{again}");

        // Refused: a size not offered, and one beyond the server's limit.
        let (status, body) = call(&app, "POST", &format!("{base}/sizes/1000x500"), None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(json(&body)["detail"]["code"], "size_not_offered");
        let (status, _) = call(&app, "POST", &format!("{base}/sizes/1024X512"), None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let (status, _) = call(
            &app,
            "GET",
            &format!("/api/systems/{id}/bodies/zz/sizes"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);

        // The displayed size is remembered, and the default of every route.
        let put = format!("{base}/displayed-size");
        let (status, body) =
            call(&app, "PUT", &put, Some(r#"{"width": 1024, "height": 512}"#)).await;
        assert_eq!(status, StatusCode::NO_CONTENT, "{body}");
        assert_eq!(sizes().await["displayed"], serde_json::json!([1024, 512]));
        let (status, layer) = {
            let request = Request::get(format!("{base}/epoch/0/elevation.png"))
                .body(Body::empty())
                .unwrap();
            let response = app.clone().oneshot(request).await.unwrap();
            let status = response.status();
            (
                status,
                response.into_body().collect().await.unwrap().to_bytes(),
            )
        };
        assert_eq!(status, StatusCode::OK);
        let layer = image::load_from_memory(&layer).unwrap();
        assert_eq!((layer.width(), layer.height()), (1024, 512));
        let (status, body) = call(
            &app,
            "PUT",
            &put,
            Some(r#"{"width": 2048, "height": 1024}"#),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(json(&body)["detail"]["code"], "size_not_generated");

        // Deleted: gone from the list and the routes, the original shown.
        let tile = format!("{base}/tiles/0/0/0?size=1024x512");
        assert_eq!(call(&app, "GET", &tile, None).await.0, StatusCode::OK);
        let (status, body) = call(&app, "DELETE", &post, None).await;
        assert_eq!(status, StatusCode::NO_CONTENT, "{body}");
        let listed = sizes().await;
        assert_eq!(listed["displayed"], serde_json::json!([256, 128]));
        assert_eq!(listed["present"].as_array().unwrap().len(), 1);
        assert_eq!(
            call(&app, "GET", &tile, None).await.0,
            StatusCode::NOT_FOUND
        );
        let (status, body) = call(&app, "DELETE", &post, None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(json(&body)["detail"]["code"], "size_not_generated");
        // The original stays.
        let (status, body) = call(&app, "DELETE", &format!("{base}/sizes/256x128"), None).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(json(&body)["detail"]["code"], "original_size");

        // A server whose limit is below the size asked for.
        let tight = router(tight);
        let (id, planet) = generated_planet(&tight, "barren", 9, "").await;
        let post = format!("/api/systems/{id}/bodies/{planet}/sizes/1024x512");
        let (status, body) = call(&tight, "POST", &post, None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
        assert_eq!(json(&body)["detail"]["code"], "too_many_pixels");
    }

    /// A state and its routes (eviction off), with a generated planet:
    /// the system's id and the planet's body id.
    async fn evictable(name: &str, seed: u64) -> (Config, Shared, Router, String, String) {
        let config = temp_config(name);
        let state = new_state(&config);
        let app = routes(state.clone(), &config);
        let (id, planet) = generated_planet(&app, "barren", seed, "").await;
        wait_released(&state).await;
        (config, state, app, id, planet)
    }

    fn json(text: &str) -> serde_json::Value {
        serde_json::from_str(text).unwrap()
    }

    /// Waits until the body shown at `size` is listed as displayed: up to
    /// three minutes, since it takes a regeneration and then a second job at
    /// that size, which a shared CI runner can take well over 30 s to run.
    async fn wait_displayed(app: &Router, base: &str, size: [u64; 2]) -> serde_json::Value {
        for _ in 0..9000 {
            let (status, body) = call(app, "GET", &format!("{base}/sizes"), None).await;
            if status == StatusCode::OK && json(&body)["displayed"] == serde_json::json!(size) {
                return json(&body);
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        panic!("{base} never displayed at {size:?}");
    }

    #[tokio::test]
    async fn eviction_deletes_every_size_and_keeps_the_rest() {
        let (config, state, app, id, planet) = evictable("evict", 21).await;
        let base = format!("/api/systems/{id}/bodies/{planet}");
        let body_dir = systems::body_dir(&config.systems_dir, &id, &planet);
        // A second size (displayed once done), a name and a pin.
        let (_, started) = call(&app, "POST", &format!("{base}/sizes/1024x512"), None).await;
        wait_job(&app, &started).await;
        wait_released(&state).await;
        let (status, _) = call(
            &app,
            "PUT",
            &format!("{base}/name"),
            Some(r#"{"name": "Gaia"}"#),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let pin = format!("/api/systems/{id}/pin");
        assert_eq!(call(&app, "PUT", &pin, None).await.0, StatusCode::OK);
        let seed = state.index.regeneration_of(&id, &planet).unwrap();
        assert!(seed.is_some());
        let thumb = std::fs::read(body_dir.join("thumb.png")).unwrap();

        let freed = evict::evict_body(&state, &id, &planet).unwrap();
        assert_eq!(freed.sizes, 2, "the original and the added size");
        assert!(freed.bytes > 0);

        // Every generated file is gone, nothing is left aside; the thumbnail stays.
        assert!(!body_dir.join("sizes").exists());
        assert_eq!(std::fs::read(body_dir.join("thumb.png")).unwrap(), thumb);
        let aside: Vec<_> = std::fs::read_dir(body_dir.parent().unwrap())
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(aside, std::slice::from_ref(&planet), "no .tmp-evict left");
        // system.json stays, the body marked not generated.
        let system =
            mapgen::system::read_system(&systems::system_dir(&config.systems_dir, &id)).unwrap();
        assert!(
            !system
                .bodies
                .iter()
                .find(|b| b.id == planet)
                .unwrap()
                .generated
        );
        // The index keeps its rows: seed and parameters, displayed size, name, pin.
        assert_eq!(state.index.regeneration_of(&id, &planet).unwrap(), seed);
        assert_eq!(
            state.index.displayed_size(&id, &planet).unwrap(),
            Some((1024, 512))
        );
        let (_, list) = call(&app, "GET", "/api/systems", None).await;
        let list = json(&list);
        assert_eq!(list[0]["names"][&planet], "Gaia");
        assert_eq!(list[0]["pinned"], true);
        // Not generated in the library, listed as evicted in the system view.
        assert_eq!(list[0]["bodies"][0]["generated"], false);
        let (_, detail) = call(&app, "GET", &format!("/api/systems/{id}"), None).await;
        assert_eq!(json(&detail)["evicted"], serde_json::json!([planet]));
        // The view has nothing to show, the card still has its thumbnail.
        assert_eq!(
            call(&app, "GET", &format!("{base}/sizes"), None).await.0,
            StatusCode::NOT_FOUND
        );
        let request = Request::get(format!("{base}/thumb"))
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(bytes.as_ref(), thumb.as_slice());
        // Evicting again is not an error.
        assert_eq!(
            evict::evict_body(&state, &id, &planet).unwrap(),
            evict::Evicted::default()
        );
        // A body the system does not have.
        assert!(matches!(
            evict::evict_body(&state, &id, "zz"),
            Err(evict::EvictError::Unknown(_))
        ));
    }

    #[tokio::test]
    async fn a_body_with_a_pending_job_is_not_evicted() {
        let (config, state, _app, id, planet) = evictable("evict-pending", 22).await;
        let sizes = systems::body_dir(&config.systems_dir, &id, &planet).join("sizes");
        for key in [pending_size(&id, &planet, (1024, 512)), pending_system(&id)] {
            state.pending.lock().unwrap().push(key.clone());
            assert_eq!(
                evict::evict_body(&state, &id, &planet),
                Err(evict::EvictError::Pending)
            );
            assert!(sizes.is_dir(), "{key}: files untouched");
            assert!(state.index.generator_of(&id, &planet).unwrap().is_some());
            state.pending.lock().unwrap().retain(|k| *k != key);
        }
        assert_eq!(evict::evict_body(&state, &id, &planet).unwrap().sizes, 1);
    }

    #[tokio::test]
    async fn eviction_takes_the_lock_that_job_starts_take() {
        let (config, state, _app, id, planet) = evictable("evict-lock", 23).await;
        let sizes = systems::body_dir(&config.systems_dir, &id, &planet).join("sizes");
        // A job start holds the lock while it claims its key: an eviction
        // meanwhile waits, and finds the key when it gets the lock.
        let held = state.pending.lock().unwrap();
        let task = {
            let (state, id, planet) = (state.clone(), id.clone(), planet.clone());
            std::thread::spawn(move || evict::evict_body(&state, &id, &planet))
        };
        std::thread::sleep(std::time::Duration::from_millis(300));
        assert!(!task.is_finished(), "waits for the lock");
        assert!(sizes.is_dir());
        let mut held = held;
        held.push(pending_size(&id, &planet, (1024, 512)));
        drop(held);
        assert_eq!(task.join().unwrap(), Err(evict::EvictError::Pending));
        assert!(sizes.is_dir());
    }

    /// Dates `world.json` of the original resolution of a body, as if the
    /// body had been generated `days` days ago.
    fn age_body(config: &Config, id: &str, body: &str, days: u64) {
        let dir =
            mapgen::sizes::size_dir(&systems::body_dir(&config.systems_dir, id, body), 256, 128);
        let file = std::fs::File::options()
            .write(true)
            .open(dir.join("raw_data/world.json"))
            .unwrap();
        file.set_modified(
            std::time::SystemTime::now() - std::time::Duration::from_secs(days * 86_400),
        )
        .unwrap();
    }

    #[tokio::test]
    async fn the_sweep_evicts_only_the_bodies_past_the_threshold() {
        let config = temp_config("evict-sweep");
        let state = new_state(&config);
        let app = routes(state.clone(), &config);
        let mut bodies = Vec::new();
        for seed in [31, 32, 33, 34] {
            bodies.push(generated_planet(&app, "barren", seed, "").await);
            wait_released(&state).await;
        }
        let [(a, a_body), (b, b_body), (c, c_body), (d, d_body)] = &bodies[..] else {
            unreachable!()
        };
        let day = 86_400_000;
        let now = crate::index::now_ms();
        // a: opened 20 days ago. b: opened yesterday. c: never opened, generated
        // 20 days ago. d: never opened, generated just now.
        for (system, body, days) in [(a, a_body, 20), (b, b_body, 20), (c, c_body, 20)] {
            age_body(&config, system, body, days);
        }
        state.index.write_opened(a, a_body, now - 20 * day).unwrap();
        state.index.write_opened(b, b_body, now - day).unwrap();
        // Written by a blocking task, a moment after the request.
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        let generated = |system: &String, body: &String| {
            state.index.generator_of(system, body).unwrap().is_some()
        };
        for (system, body) in [(c, c_body), (d, d_body)] {
            state.index.clear_opened(system, body).unwrap();
        }

        // 0 turns it off, whatever the dates.
        let off = evict::sweep_unopened(&state, now, 0);
        assert_eq!(off, evict::SweepReport::default());
        assert!(bodies.iter().all(|(s, b)| generated(s, b)));

        let report = evict::sweep_unopened(&state, now, 14);
        assert_eq!(report.evicted, 2, "{report:?}");
        assert!(report.bytes > 0);
        assert_eq!((report.busy, report.failed), (0, 0));
        assert!(!generated(a, a_body), "opened 20 days ago");
        assert!(generated(b, b_body), "opened yesterday");
        assert!(!generated(c, c_body), "never opened, generated 20 days ago");
        assert!(generated(d, d_body), "never opened, generated just now");
        // Nothing left to evict; a longer wait evicts the rest.
        assert_eq!(evict::sweep_unopened(&state, now, 14).evicted, 0);
        assert_eq!(evict::sweep_unopened(&state, now + 30 * day, 14).evicted, 2);
    }

    #[tokio::test]
    async fn the_sweep_leaves_a_body_with_a_job_pending() {
        let (config, state, _app, id, planet) = evictable("evict-sweep-busy", 35).await;
        age_body(&config, &id, &planet, 40);
        state.index.write_opened(&id, &planet, 1).unwrap();
        state.pending.lock().unwrap().push(pending_system(&id));
        let report = evict::sweep_unopened(&state, crate::index::now_ms(), 14);
        assert_eq!((report.evicted, report.busy), (0, 1));
        assert!(state.index.generator_of(&id, &planet).unwrap().is_some());
    }

    #[tokio::test]
    async fn opening_an_evicted_body_queues_it_at_its_displayed_size() {
        let (config, state, app, id, planet) = evictable("evict-reopen", 41).await;
        let base = format!("/api/systems/{id}/bodies/{planet}");
        let sizes = systems::body_dir(&config.systems_dir, &id, &planet).join("sizes");
        // Displayed at 1024x512 when it is evicted ...
        let (_, started) = call(&app, "POST", &format!("{base}/sizes/1024x512"), None).await;
        wait_job(&app, &started).await;
        wait_released(&state).await;
        evict::evict_body(&state, &id, &planet).unwrap();
        // ... a listing or a card does not queue it ...
        call(&app, "GET", "/api/systems", None).await;
        call(&app, "GET", &format!("/api/systems/{id}"), None).await;
        call(&app, "GET", &format!("{base}/thumb"), None).await;
        assert!(state.pending.lock().unwrap().is_empty());
        assert!(!sizes.exists());
        // ... opening its view does, and the generator is the current one.
        let (status, body) = call(&app, "POST", &format!("{base}/generate"), None).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let started = json(&body);
        assert!(started["job_id"].is_string(), "{body}");
        assert_eq!(started["regenerated"], serde_json::json!([]), "{body}");
        let listed = wait_displayed(&app, &base, [1024, 512]).await;
        let present: Vec<[u64; 2]> = listed["present"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| [p["width"].as_u64().unwrap(), p["height"].as_u64().unwrap()])
            .collect();
        assert_eq!(present, [[256, 128], [1024, 512]]);
        wait_released(&state).await;
        let (_, detail) = call(&app, "GET", &format!("/api/systems/{id}"), None).await;
        assert_eq!(json(&detail)["evicted"], serde_json::json!([]));
        let generator = state.index.generator_of(&id, &planet).unwrap().unwrap();
        assert_eq!(generator.version.as_deref(), Some(GENERATOR_VERSION));

        // With no displayed size recorded, the original size alone.
        let (id, planet) = generated_planet(&app, "barren", 42, "").await;
        wait_released(&state).await;
        let base = format!("/api/systems/{id}/bodies/{planet}");
        evict::evict_body(&state, &id, &planet).unwrap();
        assert_eq!(state.index.displayed_size(&id, &planet).unwrap(), None);
        let (_, body) = call(&app, "POST", &format!("{base}/generate"), None).await;
        wait_job(&app, &body).await;
        wait_released(&state).await;
        let (_, listed) = call(&app, "GET", &format!("{base}/sizes"), None).await;
        let listed = json(&listed);
        assert_eq!(listed["displayed"], serde_json::json!([256, 128]));
        assert_eq!(listed["present"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn only_opening_a_body_marks_it_opened() {
        let (config, _state, _app, id, planet) = evictable("evict-opened", 61).await;
        // A fresh state: no date written lately, whatever the generation did.
        let state = new_state(&config);
        let app = routes(state.clone(), &config);
        state.index.clear_opened(&id, &planet).unwrap();
        let base = format!("/api/systems/{id}/bodies/{planet}");
        // The library, a system, a card and its thumbnail, the resolutions
        // listed: nobody looks at the body.
        for uri in [
            "/api/systems".to_string(),
            format!("/api/systems/{id}"),
            format!("{base}/thumb"),
            format!("/systems/{id}/bodies/{planet}/thumb.png"),
            format!("{base}/sizes"),
            format!("/systems/{id}/system.json"),
        ] {
            assert_eq!(
                call(&app, "GET", &uri, None).await.0,
                StatusCode::OK,
                "{uri}"
            );
        }
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        assert_eq!(state.index.last_opened(&id, &planet).unwrap(), None);
        // Another body's sky and the star view's markers take its textures
        // with `?peek=1`: seen from elsewhere, not opened.
        let file = format!("/systems/{id}/bodies/{planet}/sizes/256x128/raw_data/world.json");
        for uri in [
            format!("{file}?peek=1"),
            format!("{file}?v=3&peek=1"),
            format!("/systems/{id}/bodies/{planet}/sizes/256x128/raw_data/colour_small.png?peek=1"),
        ] {
            let status = call(&app, "GET", &uri, None).await.0;
            assert!(
                status == StatusCode::OK || status == StatusCode::NOT_FOUND,
                "{uri}"
            );
        }
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        assert_eq!(
            state.index.last_opened(&id, &planet).unwrap(),
            None,
            "peeked"
        );
        // `peek=10` or `unpeek=1` is not the flag.
        assert!(!is_peek(Some("peek=10")) && !is_peek(Some("unpeek=1")) && !is_peek(None));
        // Its view loads its files: opened.
        assert_eq!(call(&app, "GET", &file, None).await.0, StatusCode::OK);
        for _ in 0..100 {
            if state.index.last_opened(&id, &planet).unwrap().is_some() {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        panic!("not marked opened");
    }

    /// A generator version that is not the current one: another build of
    /// the same crate version when the build has an id, else another crate
    /// version.
    fn older_build() -> String {
        match GENERATOR_VERSION.split_once('+') {
            Some((crate_version, _)) => format!("{crate_version}+deadbeef0000"),
            None => "0.0.1".to_string(),
        }
    }

    #[tokio::test]
    async fn the_api_says_when_an_older_world_is_made_again() {
        let (_config, state, app, id, planet) = evictable("evict-notice", 51).await;
        let base = format!("/api/systems/{id}/bodies/{planet}");
        // Made by another build (an older one), then evicted.
        let older = older_build();
        state
            .index
            .set_generator(&id, &planet, Some(&older), Some(1))
            .unwrap();
        evict::evict_body(&state, &id, &planet).unwrap();
        let (_, body) = call(&app, "POST", &format!("{base}/generate"), None).await;
        let started = json(&body);
        let regenerated = &started["regenerated"];
        assert_eq!(regenerated.as_array().unwrap().len(), 1, "{body}");
        assert_eq!(regenerated[0]["body"], planet.as_str());
        let note = &regenerated[0]["from"];
        assert!(started["job_id"].is_string(), "{body}");
        assert_eq!(note["version"], older.as_str());
        assert_eq!(note["format"], 1);
        assert_eq!(note["current_version"], GENERATOR_VERSION);
        assert_eq!(note["current_format"], GENERATOR_FORMAT);
        // Said again while the job runs (the viewer tells the user once).
        let (_, again) = call(&app, "POST", &format!("{base}/generate"), None).await;
        assert_eq!(json(&again)["regenerated"], *regenerated);
        wait_job(&app, &body).await;
        wait_released(&state).await;
        // Made again by the current generator: nothing more to say.
        let (_, done) = call(&app, "POST", &format!("{base}/generate"), None).await;
        assert!(json(&done)["job_id"].is_null());
        assert_eq!(json(&done)["regenerated"], serde_json::json!([]));
    }

    #[tokio::test]
    async fn a_body_opened_after_the_sweep_listed_it_survives() {
        let (config, state, _app, id, planet) = evictable("evict-recheck", 71).await;
        age_body(&config, &id, &planet, 40);
        let now = crate::index::now_ms();
        let cutoff = now - 14 * 86_400_000;
        // The sweep listed the body as unopened (dated by its file) ...
        state.index.clear_opened(&id, &planet).unwrap();
        // ... then someone opened it: the check under the lock sees it.
        state.index.write_opened(&id, &planet, now).unwrap();
        assert_eq!(
            evict::evict_body_unopened_since(&state, &id, &planet, cutoff),
            Err(evict::EvictError::Opened)
        );
        let sizes = systems::body_dir(&config.systems_dir, &id, &planet).join("sizes");
        assert!(sizes.is_dir());
        assert!(state.index.generator_of(&id, &planet).unwrap().is_some());
        // Generated again since (a fresh world.json) counts as well.
        state.index.clear_opened(&id, &planet).unwrap();
        age_body(&config, &id, &planet, 0);
        assert_eq!(
            evict::evict_body_unopened_since(&state, &id, &planet, cutoff),
            Err(evict::EvictError::Opened)
        );
        // Unopened for good: evicted. And the unconditional entry point
        // takes no age into account.
        age_body(&config, &id, &planet, 40);
        state.index.write_opened(&id, &planet, 1).unwrap();
        assert_eq!(
            evict::evict_body_unopened_since(&state, &id, &planet, cutoff)
                .unwrap()
                .sizes,
            1
        );
    }

    #[tokio::test]
    async fn evicting_a_body_does_not_move_its_system_in_the_library() {
        let config = temp_config("evict-date");
        let state = new_state(&config);
        let app = routes(state.clone(), &config);
        let (old, old_body) = generated_planet(&app, "barren", 81, "").await;
        wait_released(&state).await;
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        let (new, _) = generated_planet(&app, "barren", 82, "").await;
        wait_released(&state).await;
        let order = |list: &str| -> Vec<(String, u64)> {
            json(list)
                .as_array()
                .unwrap()
                .iter()
                .map(|s| {
                    (
                        s["id"].as_str().unwrap().to_string(),
                        s["modified_ms"].as_u64().unwrap(),
                    )
                })
                .collect()
        };
        let (_, before) = call(&app, "GET", "/api/systems", None).await;
        let before = order(&before);
        assert_eq!(before[0].0, new, "the newer system first");
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;

        evict::evict_body(&state, &old, &old_body).unwrap();

        let (_, after) = call(&app, "GET", "/api/systems", None).await;
        assert_eq!(order(&after), before, "same order, same dates");
        // Nor does a restart (a reconcile) read another date.
        let state = new_state(&config);
        let app = routes(state, &config);
        let (_, restarted) = call(&app, "GET", "/api/systems", None).await;
        assert_eq!(order(&restarted), before);
    }

    #[tokio::test]
    async fn a_rebuilt_index_does_not_evict_every_old_body() {
        let (mut config, state, _app, id, planet) = evictable("evict-rebuild", 91).await;
        age_body(&config, &id, &planet, 40);
        drop(state);
        // The index is lost (or its layout bumped): rebuilt from the tree,
        // where the files are 40 days old.
        std::fs::remove_dir_all(config.db_path.parent().unwrap()).unwrap();
        config.evict_after_days = 14;
        let state = new_state(&config);
        let report = evict::sweep_unopened(&state, crate::index::now_ms(), 14);
        assert_eq!(report.evicted, 0, "{report:?}");
        assert!(state.index.generator_of(&id, &planet).unwrap().is_some());
        // The body counts from when the index first saw it.
        let report = evict::sweep_unopened(&state, crate::index::now_ms() + 15 * 86_400_000, 14);
        assert_eq!(report.evicted, 1, "{report:?}");
    }

    #[tokio::test]
    async fn a_failed_eviction_puts_system_json_back() {
        use std::os::unix::fs::PermissionsExt;
        let (config, state, _app, id, planet) = evictable("evict-rollback", 92).await;
        let system_dir = systems::system_dir(&config.systems_dir, &id);
        let file = system_dir.join("system.json");
        let before = std::fs::metadata(&file).unwrap().modified().unwrap();
        let body_dir = systems::body_dir(&config.systems_dir, &id, &planet);
        // `sizes/` cannot be moved out of a directory that is read-only.
        std::fs::set_permissions(&body_dir, std::fs::Permissions::from_mode(0o555)).unwrap();
        let result = evict::evict_body(&state, &id, &planet);
        std::fs::set_permissions(&body_dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        if result.is_ok() {
            // Running as root: permissions do not apply, nothing to check.
            return;
        }
        assert!(
            matches!(result, Err(evict::EvictError::Failed(_))),
            "{result:?}"
        );
        let system = mapgen::system::read_system(&system_dir).unwrap();
        assert!(
            system
                .bodies
                .iter()
                .find(|b| b.id == planet)
                .unwrap()
                .generated
        );
        assert_eq!(
            std::fs::metadata(&file).unwrap().modified().unwrap(),
            before
        );
        assert!(body_dir.join("sizes").is_dir());
        assert!(state.index.generator_of(&id, &planet).unwrap().is_some());
        // And once the problem is gone, the eviction works.
        assert_eq!(evict::evict_body(&state, &id, &planet).unwrap().sizes, 1);
    }

    #[tokio::test]
    async fn the_sweep_clears_the_evictions_a_crash_left() {
        let (config, state, _app, id, planet) = evictable("evict-leftover", 93).await;
        let bodies = systems::system_dir(&config.systems_dir, &id).join("bodies");
        let now = crate::index::now_ms();
        let old = bodies.join(format!(".tmp-evict-{planet}-{}", now - 2 * 3_600_000));
        let young = bodies.join(format!(".tmp-evict-{planet}-{now}"));
        let other = bodies.join(".tmp-other");
        for dir in [&old, &young, &other] {
            std::fs::create_dir_all(dir.join("x")).unwrap();
        }
        evict::sweep_unopened(&state, now, 14);
        assert!(!old.exists(), "a leftover from hours ago");
        assert!(young.exists(), "may still be being deleted");
        assert!(other.exists(), "not an eviction's");
        assert!(bodies.join(&planet).is_dir());
    }

    #[tokio::test]
    async fn a_system_held_for_a_change_is_not_evicted_or_started() {
        let (config, state, app, id, planet) = evictable("evict-hold", 94).await;
        let hold = hold_system(&state, &id).expect("free");
        assert!(hold_system(&state, &id).is_none(), "held once");
        assert_eq!(
            evict::evict_body(&state, &id, &planet),
            Err(evict::EvictError::Pending)
        );
        // Deleting meanwhile is refused too, and nothing is touched.
        let (status, _) = call(&app, "DELETE", &format!("/api/systems/{id}"), None).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert!(systems::system_dir(&config.systems_dir, &id).is_dir());
        drop(hold);
        assert!(state.pending.lock().unwrap().is_empty(), "released");
        // A delete holds the key while it works and releases it: an
        // eviction right after finds no system, not a half-deleted one.
        let (status, _) = call(&app, "DELETE", &format!("/api/systems/{id}"), None).await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert!(state.pending.lock().unwrap().is_empty());
        assert!(matches!(
            evict::evict_body(&state, &id, &planet),
            Err(evict::EvictError::Unknown(_))
        ));
        assert!(!systems::system_dir(&config.systems_dir, &id).exists());
    }

    #[tokio::test]
    async fn replacing_a_system_holds_it_and_survives_evictions() {
        let (config, state, app, id, planet) = evictable("evict-replace", 95).await;
        let evictions = {
            let (state, id, planet) = (state.clone(), id.clone(), planet.clone());
            std::thread::spawn(move || {
                let mut pending = 0;
                for _ in 0..200 {
                    if evict::evict_body(&state, &id, &planet) == Err(evict::EvictError::Pending) {
                        pending += 1;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(2));
                }
                pending
            })
        };
        let request = r#"{"around": {"planet": "barren", "seed": 95, "width": 256, "height": 128, "moons": "none", "giants": "none"}}"#;
        let (status, created) =
            call(&app, "POST", "/api/systems?replace=true", Some(request)).await;
        assert_eq!(status, StatusCode::CREATED, "{created}");
        wait_job(&app, &created).await;
        wait_released(&state).await;
        evictions.join().unwrap();
        // The replacement is whole: its file reads, whatever the evictions did.
        let system =
            mapgen::system::read_system(&systems::system_dir(&config.systems_dir, &id)).unwrap();
        assert!(system.bodies.iter().any(|b| b.id == planet));
        assert!(
            systems::system_dir(&config.systems_dir, &id)
                .read_dir()
                .unwrap()
                .flatten()
                .all(|e| !e.file_name().to_string_lossy().starts_with(".old-"))
        );
    }

    /// A system with a planet and a moon, both generated: the system id,
    /// the planet and the moon.
    async fn system_with_a_moon(app: &Router, state: &Shared) -> (String, String, String) {
        for seed in 101..140 {
            let request = format!(
                r#"{{"around": {{"planet": "terran", "seed": {seed}, "width": 256, "height": 128, "moons": "auto", "giants": "none"}}}}"#
            );
            let (status, created) = call(app, "POST", "/api/systems", Some(&request)).await;
            if status != StatusCode::CREATED {
                continue;
            }
            wait_job(app, &created).await;
            wait_released(state).await;
            let id = json(&created)["system_id"].as_str().unwrap().to_string();
            let (_, detail) = call(app, "GET", &format!("/api/systems/{id}"), None).await;
            let system = &json(&detail)["system"];
            let focus = system["focus"].as_str().unwrap().to_string();
            let moon = system["bodies"]
                .as_array()
                .unwrap()
                .iter()
                .find(|b| b["parent"] == focus.as_str())
                .and_then(|b| b["id"].as_str());
            if let Some(moon) = moon {
                return (id, focus, moon.to_string());
            }
        }
        panic!("no seed gave a planet with a moon");
    }

    #[tokio::test]
    async fn only_the_opened_body_comes_back_at_its_displayed_size() {
        let config = temp_config("evict-neighbours");
        let state = new_state(&config);
        let app = routes(state.clone(), &config);
        let (id, planet, moon) = system_with_a_moon(&app, &state).await;
        for body in [&planet, &moon] {
            state
                .index
                .set_displayed_size(&id, body, Some((1024, 512)))
                .unwrap();
            evict::evict_body(&state, &id, body).unwrap();
        }
        let base = format!("/api/systems/{id}/bodies/{planet}");
        let (_, started) = call(&app, "POST", &format!("{base}/generate"), None).await;
        wait_job(&app, &started).await;
        wait_displayed(&app, &base, [1024, 512]).await;
        wait_released(&state).await;
        // The moon was made by the same job, at its original size only.
        let (status, sizes) = call(
            &app,
            "GET",
            &format!("/api/systems/{id}/bodies/{moon}/sizes"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{sizes}");
        let sizes = json(&sizes);
        assert_eq!(sizes["present"].as_array().unwrap().len(), 1);
        assert_eq!(
            sizes["displayed"], sizes["original"],
            "the original size only"
        );
        assert!(
            !systems::body_dir(&config.systems_dir, &id, &moon)
                .join("sizes/1024x512")
                .exists()
        );
    }

    #[tokio::test]
    async fn a_job_names_each_older_body_it_makes_again() {
        let config = temp_config("evict-notice-family");
        let state = new_state(&config);
        let app = routes(state.clone(), &config);
        let (id, planet, moon) = system_with_a_moon(&app, &state).await;
        let older = older_build();
        state
            .index
            .set_generator(&id, &moon, Some(&older), Some(1))
            .unwrap();
        // Only the moon lost its files: the planet's own view makes it
        // again (a planet's neighbourhood), and says so.
        evict::evict_body(&state, &id, &moon).unwrap();
        let busy = state.queue.acquire().await.unwrap();
        let (_, first) = call(
            &app,
            "POST",
            &format!("/api/systems/{id}/bodies/{planet}/generate"),
            None,
        )
        .await;
        let first = json(&first);
        let regenerated = first["regenerated"].as_array().unwrap();
        assert_eq!(regenerated.len(), 1, "{first}");
        assert_eq!(regenerated[0]["body"], moon.as_str());
        assert_eq!(regenerated[0]["from"]["version"], older.as_str());
        // The moon's own request, while that job runs, says it too.
        let (_, second) = call(
            &app,
            "POST",
            &format!("/api/systems/{id}/bodies/{moon}/generate"),
            None,
        )
        .await;
        let second = json(&second);
        assert_eq!(second["job_id"], first["job_id"]);
        assert_eq!(second["regenerated"], first["regenerated"]);
        drop(busy);
        wait_job(&app, &serde_json::to_string(&first).unwrap()).await;
        wait_released(&state).await;
        // Made again by the current generator: nothing more to say.
        let (_, done) = call(
            &app,
            "POST",
            &format!("/api/systems/{id}/bodies/{moon}/generate"),
            None,
        )
        .await;
        assert_eq!(json(&done)["regenerated"], serde_json::json!([]));
    }

    #[tokio::test]
    async fn a_body_made_by_the_current_generator_is_not_announced() {
        let config = temp_config("evict-notice-same");
        let state = new_state(&config);
        let app = routes(state.clone(), &config);
        let (id, planet, moon) = system_with_a_moon(&app, &state).await;
        for body in [&planet, &moon] {
            evict::evict_body(&state, &id, body).unwrap();
        }
        let (_, started) = call(
            &app,
            "POST",
            &format!("/api/systems/{id}/bodies/{planet}/generate"),
            None,
        )
        .await;
        let started = json(&started);
        assert!(started["job_id"].is_string(), "{started}");
        assert_eq!(started["regenerated"], serde_json::json!([]));
        wait_job(&app, &serde_json::to_string(&started).unwrap()).await;
        wait_released(&state).await;
    }

    #[tokio::test]
    async fn making_evicted_bodies_again_does_not_move_their_system() {
        let config = temp_config("evict-regen-date");
        let state = new_state(&config);
        let app = routes(state.clone(), &config);
        let (id, planet, moon) = system_with_a_moon(&app, &state).await;
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        let (newer, _) = generated_planet(&app, "barren", 83, "").await;
        wait_released(&state).await;
        let modified = |list: &str, id: &str| -> u64 {
            json(list)
                .as_array()
                .unwrap()
                .iter()
                .find(|s| s["id"] == id)
                .unwrap()["modified_ms"]
                .as_u64()
                .unwrap()
        };
        let (_, listing) = call(&app, "GET", "/api/systems", None).await;
        let before = modified(&listing, &id);
        assert!(modified(&listing, &newer) > before);
        let file = systems::system_dir(&config.systems_dir, &id).join("system.json");
        let file_date = std::fs::metadata(&file).unwrap().modified().unwrap();
        for body in [&planet, &moon] {
            evict::evict_body(&state, &id, body).unwrap();
        }
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        // Opened again: the whole family comes back.
        let (_, started) = call(
            &app,
            "POST",
            &format!("/api/systems/{id}/bodies/{moon}/generate"),
            None,
        )
        .await;
        wait_job(&app, &started).await;
        wait_released(&state).await;
        let (_, listing) = call(&app, "GET", "/api/systems", None).await;
        assert_eq!(modified(&listing, &id), before, "same date in the library");
        assert_eq!(
            std::fs::metadata(&file).unwrap().modified().unwrap(),
            file_date
        );
        assert!(modified(&listing, &newer) > modified(&listing, &id));
        // Nothing was lost: the bodies are generated again.
        let system = mapgen::system::read_system(file.parent().unwrap()).unwrap();
        for body in [&planet, &moon] {
            assert!(
                system.bodies.iter().any(|b| &b.id == body && b.generated),
                "{body}"
            );
        }
    }

    #[tokio::test]
    async fn a_body_never_generated_before_moves_its_system() {
        let config = temp_config("evict-regen-first");
        let state = new_state(&config);
        let app = routes(state.clone(), &config);
        let (id, planet, moon) = system_with_a_moon(&app, &state).await;
        let file = systems::system_dir(&config.systems_dir, &id).join("system.json");
        for body in [&planet, &moon] {
            evict::evict_body(&state, &id, body).unwrap();
        }
        // The moon has no record of ever being generated (a first
        // generation): the job is not only an eviction's undoing.
        state.index.set_generator(&id, &moon, None, None).unwrap();
        let before = std::fs::metadata(&file).unwrap().modified().unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        let (_, started) = call(
            &app,
            "POST",
            &format!("/api/systems/{id}/bodies/{planet}/generate"),
            None,
        )
        .await;
        wait_job(&app, &started).await;
        wait_released(&state).await;
        let after = std::fs::metadata(&file).unwrap().modified().unwrap();
        assert!(after > before, "a first generation dates the system");
    }

    /// Waits until no generation is pending (a job frees its key just
    /// after announcing its end).
    async fn wait_released(state: &AppState) {
        for _ in 0..500 {
            if state.pending.lock().unwrap().is_empty() {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        panic!("pending: {:?}", state.pending.lock().unwrap());
    }

    #[tokio::test]
    async fn a_size_job_runs_without_its_request_and_leaves_nothing_when_cancelled() {
        let config = temp_config("size-orphan");
        let state = new_state(&config);
        let app = routes(state.clone(), &config);
        let (id, planet) = generated_planet(&app, "barren", 11, "").await;
        let body_dir = systems::body_dir(&config.systems_dir, &id, &planet);
        let events = |job: &str| {
            let (app, uri) = (app.clone(), format!("/api/jobs/{job}/events"));
            async move {
                tokio::time::timeout(
                    std::time::Duration::from_secs(120),
                    call(&app, "GET", &uri, None),
                )
                .await
                .expect("the job ends")
                .1
            }
        };

        // Claimed, then nothing awaited by any request (as when a client
        // drops it): the job runs all the same and frees its key.
        let job = claim_size(&state, &id, &planet, (1024, 512))
            .ok()
            .flatten()
            .expect("a new job");
        assert!(events(&job).await.contains("\"status\":\"done\""));
        wait_released(&state).await;
        assert!(
            body_dir
                .join("sizes/1024x512/raw_data/world.json")
                .is_file()
        );

        // Cancelled midway: no staging directory is left.
        let job = claim_size(&state, &id, &planet, (2048, 1024))
            .ok()
            .flatten()
            .expect("a new job");
        let staging = body_dir.join("sizes/.tmp-2048x1024");
        wait_running(&app).await;
        let (status, _) = call(&app, "DELETE", &format!("/api/jobs/{job}"), None).await;
        assert_eq!(status, StatusCode::ACCEPTED);
        assert!(events(&job).await.contains("\"status\":\"cancelled\""));
        assert!(!staging.exists());
        assert!(!body_dir.join("sizes/2048x1024").exists());
        wait_released(&state).await;
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
    async fn form_offers_two_to_one_sizes_and_defaults_to_2048() {
        let app = router(temp_config("form-sizes"));
        let (status, body) = call(&app, "GET", "/api/form", None).await;
        assert_eq!(status, StatusCode::OK);
        assert!(
            body.contains(r#""size_presets":[[1024,512],[2048,1024],[4096,2048],[8192,4096]]"#)
        );
        assert!(body.contains(r#""default_width":2048"#));
        assert!(body.contains(r#""default_height":1024"#));
    }

    #[tokio::test]
    async fn rejects_invalid_requests() {
        let app = router(temp_config("invalid"));
        for body in [
            r#"{"around": {"sea_level": 3.0}}"#,
            r#"{"around": {"planet": "/etc/passwd.toml"}}"#,
            r#"{"around": {"width": 100000}}"#,
            r#"{"around": {"width": 8192, "height": 8192}}"#,
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
        // Naming the system keeps the names of its bodies, even those given
        // after a client read them; null clears it.
        let name = format!("/api/systems/{id}/name");
        let (status, named) = call(&app, "PUT", &name, Some(r#"{"name": " Sol "}"#)).await;
        assert_eq!(status, StatusCode::OK, "{named}");
        let named: serde_json::Value = serde_json::from_str(&named).unwrap();
        assert_eq!(named["names"]["star"], "Sol");
        assert_eq!(named["names"][&first], "Gaia");
        let (status, cleared) = call(&app, "PUT", &name, Some(r#"{"name": null}"#)).await;
        assert_eq!(status, StatusCode::OK, "{cleared}");
        let cleared: serde_json::Value = serde_json::from_str(&cleared).unwrap();
        assert!(cleared["names"].get("star").is_none());
        assert_eq!(cleared["names"][&first], "Gaia");
        assert_eq!(
            call(
                &app,
                "PUT",
                &name,
                Some(&format!(r#"{{"name": "{}"}}"#, "x".repeat(99)))
            )
            .await
            .0,
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            call(
                &app,
                "PUT",
                "/api/systems/mgs-0-0/name",
                Some(r#"{"name": "A"}"#)
            )
            .await
            .0,
            StatusCode::NOT_FOUND
        );
        // One body at a time: the others are kept.
        let one = format!("/api/systems/{id}/bodies/{first}/name");
        let (status, body_named) = call(&app, "PUT", &one, Some(r#"{"name": "Terra"}"#)).await;
        assert_eq!(status, StatusCode::OK, "{body_named}");
        let body_named: serde_json::Value = serde_json::from_str(&body_named).unwrap();
        assert_eq!(body_named["names"][&first], "Terra");
        let (_, back) = call(&app, "PUT", &one, Some(r#"{"name": "Gaia"}"#)).await;
        assert!(back.contains("Gaia"), "{back}");
        assert_eq!(
            call(
                &app,
                "PUT",
                &format!("/api/systems/{id}/bodies/zz/name"),
                Some(r#"{"name": "A"}"#)
            )
            .await
            .0,
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            call(
                &app,
                "PUT",
                &format!("/api/systems/{id}/bodies/QQ/name"),
                Some(r#"{"name": "A"}"#)
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            call(
                &app,
                "PUT",
                "/api/systems/..x/name",
                Some(r#"{"name": "A"}"#)
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
        // The star was named "Helia" before: put it back for the rest.
        call(&app, "PUT", &name, Some(r#"{"name": "Helia"}"#)).await;

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
        // Unpinning what is not pinned is fine.
        let (status, again) = call(&app, "DELETE", &pin, None).await;
        assert_eq!(status, StatusCode::OK);
        assert!(again.contains(r#""pinned":false"#), "{again}");
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
        // Two barren planets asked, each counted under its class.
        let planets = list[0]["planets"].as_object().unwrap();
        assert!(
            planets
                .keys()
                .all(|t| mapgen::planet::PLANET_CLASSES.contains(&t.as_str())),
            "{planets:?}"
        );
        let total: u64 = planets.values().map(|n| n.as_u64().unwrap()).sum();
        assert_eq!(total, 2);
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
            systems::body_dir(&root, &id, &planet)
                .join("sizes/64x32/raw_data/world.json")
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
            &format!("/systems/{id}/bodies/{planet}/sizes/64x32/raw_data/world.json"),
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
            &format!("/systems/{id}/bodies/{planet}/sizes/64x32/raw_data/world.json"),
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

    #[tokio::test]
    async fn a_lone_planet_system_is_named_lone_and_listed() {
        let app = router(temp_config("lone"));
        let request = r#"{"around": {"planet": "barren", "seed": 5, "width": 64, "height": 32, "moons": "none", "alone": true}}"#;
        let (status, body) = call(&app, "POST", "/api/systems", Some(request)).await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        let created: serde_json::Value = serde_json::from_str(&body).unwrap();
        let id = created["system_id"].as_str().expect("the system id");
        assert!(id.contains("-lone-5"), "{id}");
        assert!(systems::is_valid_system_id(id), "{id}");
        let (status, system) = call(&app, "GET", &format!("/api/systems/{id}"), None).await;
        assert_eq!(status, StatusCode::OK, "{system}");
        let system: serde_json::Value = serde_json::from_str(&system).unwrap();
        let system = &system["system"];
        let planets = system["bodies"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|b| b["kind"] == "planet")
            .count();
        assert_eq!(planets, 1, "{system}");
        assert_eq!(system["request"]["around"]["alone"], true);
    }

    async fn get_bytes(app: &Router, uri: &str) -> (StatusCode, HeaderMap, Vec<u8>) {
        let response = app
            .clone()
            .oneshot(Request::get(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let (status, headers) = (response.status(), response.headers().clone());
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (status, headers, bytes.to_vec())
    }

    /// The entries of an archive, by name.
    fn zip_entries(bytes: Vec<u8>) -> std::collections::BTreeMap<String, Vec<u8>> {
        use std::io::Read;
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("a zip");
        (0..archive.len())
            .map(|i| {
                let mut file = archive.by_index(i).unwrap();
                let mut data = Vec::new();
                file.read_to_end(&mut data).unwrap();
                (file.name().unwrap().into_owned(), data)
            })
            .collect()
    }

    const EXPORTED: &str = r#"{"system": {"star": "K1V", "planets": {"barren": 3}, "seed": 3, "width": 64, "height": 32}}"#;

    /// Draws `EXPORTED` and generates its planet `b`: the system's id.
    async fn exported_system(app: &Router) -> String {
        let (status, body) = call(app, "POST", "/api/systems", Some(EXPORTED)).await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        let created: serde_json::Value = serde_json::from_str(&body).unwrap();
        let id = created["system_id"].as_str().unwrap().to_string();
        let (status, body) = call(
            app,
            "POST",
            &format!("/api/systems/{id}/bodies/b/generate"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let events = wait_job(app, &body).await;
        assert!(events.contains("\"status\":\"done\""), "{events}");
        id
    }

    #[tokio::test]
    async fn a_system_is_exported_as_a_zip_of_its_json_and_pngs() {
        let app = router(temp_config("export"));
        let id = exported_system(&app).await;
        let names = r#"{"names": {"b": "Gaia", "star": "Helia"}}"#;
        assert_eq!(
            call(
                &app,
                "PUT",
                &format!("/api/systems/{id}/names"),
                Some(names)
            )
            .await
            .0,
            StatusCode::OK
        );
        let (status, headers, bytes) = get_bytes(&app, &format!("/api/systems/{id}/export")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(headers[header::CONTENT_TYPE], "application/zip");
        assert_eq!(
            headers[header::CONTENT_DISPOSITION],
            format!("attachment; filename=\"{id}.zip\"").as_str()
        );
        let entries = zip_entries(bytes);
        let png = |name: &str| {
            entries
                .get(&format!("{id}/{name}"))
                .unwrap_or_else(|| panic!("{name} in {:?}", entries.keys()))
        };
        assert!(png("bodies/b/thumb.png").starts_with(b"\x89PNG"));
        assert!(png("bodies/b/sizes/64x32/world.png").starts_with(b"\x89PNG"));
        assert!(png("bodies/b/sizes/64x32/legend.png").starts_with(b"\x89PNG"));
        // Nothing else: no tiles, raw data, SVG or noise maps.
        for name in entries.keys() {
            assert!(
                name.ends_with(".png") || name == &format!("{id}/system.json"),
                "unexpected {name}"
            );
        }
        let document: serde_json::Value = serde_json::from_slice(png("system.json")).unwrap();
        assert_eq!(document["schema"], 1);
        assert_eq!(document["id"], id.as_str());
        assert_eq!(document["seed"], 3);
        assert!(document["exported_at"].as_str().unwrap().ends_with('Z'));
        assert_eq!(document["star"]["name"], "Helia");
        assert_eq!(document["request"]["system"]["star"], "K1V");
        assert_eq!(document["create"]["system"]["seed"], 3);
        let b = &document["bodies"][0];
        assert_eq!(b["id"], "b");
        assert_eq!(b["name"], "Gaia");
        assert_eq!(b["generated"], true);
        assert_eq!(
            b["made_with"]["format_version"],
            mapgen::export::raw_data::FORMAT_VERSION
        );
        assert!(b["made_with"]["version"].is_string(), "{b}");
        assert!(b["request"]["seed"].is_u64());
    }

    #[tokio::test]
    async fn bodies_never_generated_are_listed_with_their_seed_and_request() {
        let config = temp_config("export-ungenerated");
        let root = config.systems_dir.clone();
        let app = router(config);
        let id = exported_system(&app).await;
        // Planet c was never generated; planet b loses its images.
        std::fs::remove_dir_all(root.join(&id).join("bodies/b/sizes")).unwrap();
        let (status, _, bytes) = get_bytes(&app, &format!("/api/systems/{id}/export")).await;
        assert_eq!(status, StatusCode::OK);
        let entries = zip_entries(bytes);
        assert!(
            entries.keys().all(|n| !n.contains("/sizes/")),
            "{:?}",
            entries.keys()
        );
        let document: serde_json::Value =
            serde_json::from_slice(&entries[&format!("{id}/system.json")]).unwrap();
        let (_, detail) = call(&app, "GET", &format!("/api/systems/{id}"), None).await;
        let detail: serde_json::Value = serde_json::from_str(&detail).unwrap();
        let stored = detail["system"]["bodies"].as_array().unwrap();
        let listed = document["bodies"].as_array().unwrap();
        assert_eq!(listed.len(), stored.len());
        assert!(listed.len() >= 3);
        for (listed, stored) in listed.iter().zip(stored) {
            assert_eq!(listed["id"], stored["id"]);
            assert_eq!(listed["generated"], false, "{listed}");
            assert!(listed["made_with"].is_null());
            assert_eq!(listed["files"].as_array().unwrap().len(), 0);
            assert_eq!(listed["seed"], stored["params"]["seed"]);
            assert_eq!(listed["request"], stored["params"]);
        }
    }

    /// Draws `request` (its seed left to chance), exports the system,
    /// deletes it and draws it again from the export's `create`: the
    /// export document, the system before and the system after.
    async fn drawn_again_from_its_export(
        app: &Router,
        request: &str,
    ) -> (serde_json::Value, serde_json::Value, serde_json::Value) {
        let json = |text: &str| serde_json::from_str::<serde_json::Value>(text).unwrap();
        let (status, body) = call(app, "POST", "/api/systems", Some(request)).await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        let id = json(&body)["system_id"].as_str().unwrap().to_string();
        if json(&body)["job_id"].is_string() {
            wait_job(app, &body).await;
        }
        let (_, _, bytes) = get_bytes(app, &format!("/api/systems/{id}/export")).await;
        let document = json(
            &String::from_utf8(zip_entries(bytes)[&format!("{id}/system.json")].clone()).unwrap(),
        );
        let (_, before) = call(app, "GET", &format!("/api/systems/{id}"), None).await;
        let before = json(&before);
        let deleted = call(app, "DELETE", &format!("/api/systems/{id}"), None).await;
        assert_eq!(deleted.0, StatusCode::NO_CONTENT);
        let create = document["create"].to_string();
        let (status, body) = call(app, "POST", "/api/systems", Some(&create)).await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        assert_eq!(json(&body)["system_id"], id.as_str());
        if json(&body)["job_id"].is_string() {
            wait_job(app, &body).await;
        }
        let (_, after) = call(app, "GET", &format!("/api/systems/{id}"), None).await;
        (document, before, json(&after))
    }

    fn body_seeds(detail: &serde_json::Value) -> Vec<(String, u64)> {
        detail["system"]["bodies"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| {
                (
                    b["id"].as_str().unwrap().to_string(),
                    b["params"]["seed"].as_u64().unwrap(),
                )
            })
            .collect()
    }

    #[tokio::test]
    async fn the_export_of_a_whole_system_draws_the_same_system_again() {
        let app = router(temp_config("export-whole"));
        // The seed is left to chance: only the export's `create` knows it.
        let request =
            r#"{"system": {"star": "K1V", "planets": {"barren": 3}, "width": 64, "height": 32}}"#;
        let (document, before, after) = drawn_again_from_its_export(&app, request).await;
        assert!(document["request"]["system"]["seed"].is_null());
        assert_eq!(document["create"]["system"]["seed"], document["seed"]);
        assert_eq!(after["system"]["seed"], before["system"]["seed"]);
        assert_eq!(body_seeds(&after), body_seeds(&before));
        // The seeds listed in the export are the ones drawn.
        let listed: Vec<(String, u64)> = document["bodies"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| {
                (
                    b["id"].as_str().unwrap().to_string(),
                    b["seed"].as_u64().unwrap(),
                )
            })
            .collect();
        assert_eq!(listed, body_seeds(&before));
        // The whole draw is the same, not only its seeds.
        let (mut after, mut before) = (after["system"].clone(), before["system"].clone());
        for system in [&mut after, &mut before] {
            system["request"]["system"]["seed"] = serde_json::Value::Null;
        }
        assert_eq!(after, before);
    }

    #[tokio::test]
    async fn the_export_of_a_system_round_a_planet_draws_the_same_system_again() {
        let app = router(temp_config("export-around"));
        // What a request around a planet leaves open (the star, the orbit,
        // the world's parameters) is resolved by the random generator, not
        // by the seed: the export's `create` pins the values they took.
        // A planet with neighbours and moons, its star left open; then the
        // moon of a giant, round a Sun-like star.
        for request in [
            r#"{"around": {"planet": "barren", "width": 64, "height": 32, "moons": {"count": 2}}}"#,
            r#"{"around": {"planet": "terran", "star": "G2V", "width": 64, "height": 32, "host": "giant"}}"#,
        ] {
            let (document, before, after) = drawn_again_from_its_export(&app, request).await;
            assert!(document["request"]["around"]["seed"].is_null());
            assert_eq!(document["create"]["around"]["seed"], document["seed"]);
            assert_eq!(after["system"]["seed"], before["system"]["seed"]);
            assert_eq!(after["system"]["focus"], before["system"]["focus"]);
            assert_eq!(body_seeds(&after), body_seeds(&before), "{request}");
            // The whole draw is the same, not only its seeds (the request
            // stored is the export's `create`, open fields pinned).
            let (mut after, mut before) = (after["system"].clone(), before["system"].clone());
            for system in [&mut after, &mut before] {
                system["request"] = serde_json::Value::Null;
            }
            assert_eq!(after, before, "{request}");
        }
    }

    #[tokio::test]
    async fn exporting_an_unknown_or_invalid_system_fails() {
        let app = router(temp_config("export-missing"));
        let (status, _, _) = get_bytes(&app, "/api/systems/mgs-1-1/export").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, _, _) = get_bytes(&app, "/api/systems/..%2Fetc-1/export").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    /// Waits, for 60 s at most, until a job is listed as running.
    async fn wait_running(app: &Router) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        loop {
            let (_, list) = call(app, "GET", "/api/jobs", None).await;
            if list.contains("\"status\":\"running\"") {
                return;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "no job was running after 60 s: {list}"
            );
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
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
        wait_running(&app).await;
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
