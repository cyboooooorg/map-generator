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
//! | `GET /api/jobs/{id}/events`     | job progress as server-sent events (`JobStatus`)  |
//! | `GET /worlds/{id}/...`          | world files (maps, raw data, diagnostic maps)     |
//! | anything else                   | the web front-end (single-page app)               |
use crate::api::{
    ApiError, FormSpec, HexRequest, JobCreated, JobStatus, ParamSpec, PresetInfo, WorldSummary,
};
use crate::catalog::Catalog;
use crate::worlds;
use axum::extract::{Path as UrlPath, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use mapgen::export::{ExportOptions, export_world, world_dir_name};
use mapgen::generation::{Stage, generate_with_progress};
use mapgen::hex::{DEFAULT_RIVER_FLOW_KM2, HexMap};
use mapgen::request::{self, WorldRequest, fallback};
use std::collections::HashMap;
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::{Semaphore, watch};
use tokio_stream::StreamExt;
use tokio_stream::wrappers::WatchStream;
use tower_http::services::{ServeDir, ServeFile};

/// Largest map side the server accepts: generation memory grows with the
/// pixel count, and this keeps one job under a few GB.
pub const MAX_SIZE: u32 = 8192;

/// Where the server reads and writes its files.
#[derive(Clone, Debug)]
pub struct Config {
    pub worlds_dir: PathBuf,
    pub presets_dir: PathBuf,
    /// Built front-end (`web/dist`); missing in development, where Vite serves it.
    pub static_dir: PathBuf,
}

pub struct AppState {
    config: Config,
    catalog: Catalog,
    jobs: Mutex<HashMap<String, watch::Receiver<JobStatus>>>,
    next_job: AtomicU64,
    /// One generation at a time: each one already uses every core.
    queue: Semaphore,
    /// Worlds being generated, to refuse a second job writing the same directory.
    pending: Mutex<Vec<String>>,
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
        .route("/api/jobs/{id}/events", get(job_events))
        .nest_service("/worlds", ServeDir::new(&config.worlds_dir))
        .fallback_service(spa)
        .with_state(state)
}

async fn presets(State(state): State<Shared>) -> Json<Vec<PresetInfo>> {
    Json(state.catalog.list())
}

async fn form() -> Json<FormSpec> {
    let spec = |limits, fallback| ParamSpec { limits, fallback };
    Json(FormSpec {
        sea_level: spec(request::SEA_LEVEL, fallback::SEA_LEVEL),
        volcanic: spec(request::VOLCANIC, fallback::VOLCANIC),
        circumference_km: spec(request::CIRCUMFERENCE_KM, fallback::CIRCUMFERENCE_KM),
        axial_tilt_deg: spec(request::AXIAL_TILT_DEG, fallback::AXIAL_TILT_DEG),
        erosion: spec(request::EROSION, fallback::EROSION),
        default_width: fallback::WIDTH,
        default_height: fallback::HEIGHT,
        max_size: MAX_SIZE,
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
    tokio::fs::remove_dir_all(&dir)
        .await
        .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
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

    let world_id = format!("{}-{}", params.planet.name, params.seed);
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
    state
        .jobs
        .lock()
        .expect("jobs lock")
        .insert(job_id.clone(), rx);

    let task_state = state.clone();
    let task_world = world_id.clone();
    tokio::spawn(async move {
        let _permit = task_state.queue.acquire().await;
        let worlds_dir = task_state.config.worlds_dir.clone();
        let progress = tx.clone();
        let result = tokio::task::spawn_blocking(move || {
            let steps = Stage::ALL.len() as u32;
            let report = |stage: Stage| {
                let step = Stage::ALL.iter().position(|s| *s == stage).unwrap_or(0) as u32 + 1;
                progress.send_replace(JobStatus::Running { stage, step, steps });
            };
            let generated = generate_with_progress(&params, report);
            report(Stage::Export);
            let dir = worlds_dir.join(world_dir_name(&generated.world));
            export_world(&generated, &dir, ExportOptions::default())
        })
        .await;
        let status = match result {
            Ok(Ok(())) => JobStatus::Done {
                world_id: task_world.clone(),
            },
            Ok(Err(e)) => JobStatus::Failed { error: e },
            Err(e) => JobStatus::Failed {
                error: format!("generation crashed: {e}"),
            },
        };
        if let JobStatus::Failed { error } = &status {
            tracing::error!("generation of {task_world} failed: {error}");
        }
        tx.send_replace(status);
        task_state
            .pending
            .lock()
            .expect("pending lock")
            .retain(|id| id != &task_world);
    });

    Ok((StatusCode::ACCEPTED, Json(JobCreated { job_id, world_id })))
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
        .cloned()
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

        let (status, _) = call(&app, "DELETE", "/api/worlds/frozen-3", None).await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert!(!config.worlds_dir.join("frozen-3").exists());
    }
}
