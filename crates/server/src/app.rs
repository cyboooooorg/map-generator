//! HTTP routes and shared state.
//!
//! | Route                           | Purpose                                           |
//! |---------------------------------|---------------------------------------------------|
//! | `GET /healthz`                  | liveness probe                                    |
//! | `GET /api/presets`              | planet presets the form can offer                 |
//! | `GET /api/form`                 | parameter limits and defaults                     |
//! | `GET /api/worlds`               | generated worlds, most recent first               |
//! | `POST /api/worlds`              | queue a generation (`WorldRequest` body)          |
//! | `DELETE /api/worlds/{id}`       | delete a world                                    |
//! | `POST /api/worlds/{id}/hex`     | resample a world onto a hex grid (`HexRequest`)   |
//! | `GET /api/worlds/{id}/tiles/{z}/{x}/{y}?rev=` | deep-zoom detail tile (PNG) |
//! | `GET /api/jobs`                 | recent generation jobs, most recent first         |
//! | `DELETE /api/jobs/{id}`         | cancel a queued or running generation             |
//! | `GET /api/jobs/{id}/events`     | job progress as server-sent events (`JobStatus`)  |
//! | `GET /worlds/{id}/...`          | world files (maps, raw data, diagnostic maps)     |
//! | anything else                   | the web front-end (single-page app)               |
use crate::api::{
    ApiError, FormSpec, HexRequest, JobCreated, JobInfo, JobStatus, ParamSpec, PresetInfo,
    WorldSummary,
};
use crate::catalog::Catalog;
use crate::tiles::{self, TileCache};
use crate::worlds;
use axum::extract::{Path as UrlPath, Query, State};
use axum::http::{StatusCode, header};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use mapgen::detail::TileId;
use mapgen::export::{ExportOptions, export_world, world_dir_name};
use mapgen::generation::{GenerateOptions, Stage, generate_with_progress};
use mapgen::hex::{DEFAULT_RIVER_FLOW_KM2, HexMap};
use mapgen::request::{self, WorldRequest, fallback};
use std::collections::HashMap;
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::{Notify, Semaphore, watch};
use tokio_stream::StreamExt;
use tokio_stream::wrappers::WatchStream;
use tower_http::services::{ServeDir, ServeFile};

/// Largest map side the server accepts.
pub const MAX_SIZE: u32 = 8192;
/// Default largest map the server accepts, in pixels (4096 × 2048). A
/// generation peaks at about 140 bytes per pixel (about 1.1 GB here): keep the
/// limit under the memory the server can use, or it gets killed mid-job.
pub const DEFAULT_MAX_PIXELS: u64 = 4096 * 2048;
/// Peak generation memory per pixel, for the error message.
const BYTES_PER_PIXEL: u64 = 140;
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
}

type Shared = Arc<AppState>;

/// An API error: a status code and a message, sent as [`ApiError`] JSON.
pub struct Failure(StatusCode, String);

impl IntoResponse for Failure {
    fn into_response(self) -> Response {
        (self.0, Json(ApiError { error: self.1 })).into_response()
    }
}

pub fn router(config: Config) -> Router {
    let catalog = Catalog::load(&config.presets_dir);
    let state = Arc::new(AppState {
        catalog,
        jobs: Mutex::new(HashMap::new()),
        next_job: AtomicU64::new(1),
        queue: Semaphore::new(1),
        pending: Mutex::new(Vec::new()),
        tiles: TileCache::default(),
        config: config.clone(),
    });
    let spa = ServeDir::new(&config.static_dir)
        .fallback(ServeFile::new(config.static_dir.join("index.html")));
    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/api/presets", get(presets))
        .route("/api/form", get(form))
        .route("/api/worlds", get(list_worlds).post(create_world))
        .route("/api/worlds/{id}", delete(delete_world))
        .route("/api/worlds/{id}/hex", post(hex_grid))
        .route("/api/worlds/{id}/tiles/{z}/{x}/{y}", get(tile))
        .route("/api/jobs", get(list_jobs))
        .route("/api/jobs/{id}", delete(cancel_job))
        .route("/api/jobs/{id}/events", get(job_events))
        .nest_service("/worlds", ServeDir::new(&config.worlds_dir))
        .fallback_service(spa)
        .with_state(state)
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
    let dir = state.config.worlds_dir.join(&id);
    if !dir.is_dir() {
        return Err(Failure(StatusCode::NOT_FOUND, format!("no world {id}")));
    }
    state.tiles.forget(&id);
    tokio::fs::remove_dir_all(&dir)
        .await
        .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

/// Query of a tile request: the world revision the client knows.
#[derive(serde::Deserialize)]
struct TileQuery {
    rev: Option<u64>,
}

async fn tile(
    State(state): State<Shared>,
    UrlPath((id, z, x, y)): UrlPath<(String, u32, u32, u32)>,
    Query(query): Query<TileQuery>,
) -> Result<Response, Failure> {
    let tile = TileId { z, x, y };
    if !worlds::is_valid_id(&id) || !tile.is_valid() {
        return Err(Failure(StatusCode::BAD_REQUEST, "invalid tile".into()));
    }
    let dir = state.config.worlds_dir.join(&id);
    let summary = {
        let worlds_dir = state.config.worlds_dir.clone();
        let id = id.clone();
        tokio::task::spawn_blocking(move || worlds::summary(&worlds_dir, &id))
            .await
            .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    }
    .ok_or_else(|| Failure(StatusCode::NOT_FOUND, format!("no world {id}")))?;
    let world = state
        .tiles
        .world(&dir, &id, summary.modified_ms)
        .await
        .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let png = tokio::task::spawn_blocking(move || tiles::render_png(&world, tile))
        .await
        .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    // A URL naming the current revision never changes: cache it for good.
    let cache = if query.rev == Some(summary.modified_ms) {
        "public, max-age=31536000, immutable"
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

async fn hex_grid(
    State(state): State<Shared>,
    UrlPath(id): UrlPath<String>,
    Json(request): Json<HexRequest>,
) -> Result<Json<HexMap>, Failure> {
    if !worlds::is_valid_id(&id) {
        return Err(Failure(StatusCode::BAD_REQUEST, "invalid world id".into()));
    }
    if !(4..=4096).contains(&request.columns) {
        return Err(Failure(
            StatusCode::BAD_REQUEST,
            "columns must be between 4 and 4096".into(),
        ));
    }
    let dir = state.config.worlds_dir.join(&id);
    if !dir.join("raw_data").is_dir() {
        return Err(Failure(
            StatusCode::NOT_FOUND,
            format!("no raw data for {id}"),
        ));
    }
    let river_flow = request.river_flow_km2.unwrap_or(DEFAULT_RIVER_FLOW_KM2);
    tokio::task::spawn_blocking(move || mapgen::hex::resample(&dir, request.columns, river_flow))
        .await
        .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e))
}

async fn create_world(
    State(state): State<Shared>,
    Json(mut request): Json<WorldRequest>,
) -> Result<(StatusCode, Json<JobCreated>), Failure> {
    let original = request.clone();
    let bad = |msg: String| Failure(StatusCode::BAD_REQUEST, msg);
    for (field, value) in [("width", request.width), ("height", request.height)] {
        if value.is_some_and(|v| v > MAX_SIZE) {
            return Err(bad(format!("{field} must be at most {MAX_SIZE}")));
        }
    }
    // Only catalog presets: never a path chosen by the client.
    if let Some(spec) = &request.planet {
        let load = state
            .catalog
            .resolve(spec)
            .ok_or_else(|| bad(format!("unknown planet preset {spec:?}")))?;
        request.planet = Some(load);
    }
    let resolved = request
        .resolve(&mut rand::rng())
        .map_err(|e| bad(e.to_string()))?;
    let params = resolved.params;
    let pixels = params.width as u64 * params.height as u64;
    if pixels > state.config.max_pixels {
        return Err(bad(format!(
            "{} × {} needs about {:.1} GB of memory; this server accepts up to {} pixels \
             (MAPGEN_MAX_PIXELS)",
            params.width,
            params.height,
            (pixels * BYTES_PER_PIXEL) as f64 / 1e9,
            state.config.max_pixels
        )));
    }

    let world_id = world_dir_name(&params);
    {
        let mut pending = state.pending.lock().expect("pending lock");
        if pending.contains(&world_id) || state.config.worlds_dir.join(&world_id).exists() {
            return Err(Failure(
                StatusCode::CONFLICT,
                format!("world {world_id} already exists: delete it or pick another seed"),
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

    // Worlds loaded for the deep zoom would add to the generation's peak.
    state.tiles.clear();
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
            // The web app does not show the diagnostic maps: skip them, which also
            // saves about twenty map-sized layers of memory.
            let generated = generate_with_progress(
                &params,
                GenerateOptions {
                    debug_layers: false,
                },
                report,
            );
            report(Stage::Export);
            let dir = worlds_dir.join(world_dir_name(&generated.world.params));
            export_world(
                &generated,
                &dir,
                ExportOptions {
                    raw_data: true,
                    noise_maps: false,
                },
            )
        })
        .await;
        let status = match result {
            Ok(Ok(())) => JobStatus::Done {
                world_id: task_world.clone(),
            },
            Ok(Err(e)) => JobStatus::Failed { error: e },
            Err(e) if e.is_panic() && cancel.is_requested() => JobStatus::Cancelled,
            Err(e) => JobStatus::Failed {
                error: format!("generation crashed: {e}"),
            },
        };
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
    async fn generates_lists_and_deletes_a_world() {
        let config = temp_config("generate");
        let app = router(config.clone());
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
        let (status, _) = call(&app, "GET", "/worlds/frozen-3/world.png", None).await;
        assert_eq!(status, StatusCode::OK);

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
        let (status, _) = call(
            &app,
            "POST",
            "/api/worlds/frozen-3/hex",
            Some(r#"{"columns": 1}"#),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);

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
            "/api/worlds/nope-1/tiles/0/0/0",
        ] {
            let (status, _) = call(&app, "GET", bad, None).await;
            assert!(status.is_client_error(), "{bad} gave {status}");
        }

        let (status, _) = call(&app, "DELETE", "/api/worlds/frozen-3", None).await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert!(!config.worlds_dir.join("frozen-3").exists());
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
