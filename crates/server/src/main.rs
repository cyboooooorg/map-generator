//! Web server of the planet viewer.
//!
//! Configuration (environment variables):
//!
//! | Variable              | Default      | Purpose                                  |
//! |-----------------------|--------------|------------------------------------------|
//! | `MAPGEN_SYSTEMS_DIR`  | `systems`    | drawn systems and their bodies           |
//! | `MAPGEN_PRESETS_DIR`  | `presets`    | extra planet presets (`*.toml`)          |
//! | `MAPGEN_STATIC_DIR`   | `web/dist`   | built web front-end                      |
//! | `MAPGEN_MAX_PIXELS`   | `8388608`    | largest map accepted (width × height)    |
//! | `MAPGEN_TILE_CACHE_MB`| `1024`       | memory for worlds loaded for tiles (MiB) |
//! | `HOST`                | `0.0.0.0`    | listen address                           |
//! | `PORT`                | `3000`       | listen port                              |
//! | `MALLOC_ARENA_MAX`    | glibc's      | glibc arenas; the Docker image sets `2`  |
//!
//! After a generation and after evicting worlds from the tile cache, the
//! server gives the freed memory back to the system (see [`memory`]).
//!
//! Tiles, epochs, hex grids, world loads and generations all run their
//! work on rayon's global pool at once. A worker that waits in a join runs
//! other queued jobs on top of its stack, so the stack grows with every
//! request in flight, about 5 KiB a level in a debug build: the pool's
//! threads get a deep stack (`RAYON_STACK`) and a name (`rayon-<n>`), which
//! a crash then reports instead of `<unknown>`.
mod api;
mod app;
mod catalog;
mod memory;
mod previews;
mod staging;
mod systems;
mod thumbs;
mod tiles;

use std::path::PathBuf;

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

/// Stack of each rayon worker: room for hundreds of nested joins, even in a
/// debug build (the default 2 MiB overflowed there under browser traffic).
/// Reserved, not committed: a thread only uses what it touches.
const RAYON_STACK: usize = 16 << 20;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    rayon::ThreadPoolBuilder::new()
        .thread_name(|i| format!("rayon-{i}"))
        .stack_size(RAYON_STACK)
        .build_global()
        .expect("the rayon pool is built once, before any work");
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=warn".into()),
        )
        .init();

    let config = app::Config {
        systems_dir: PathBuf::from(env_or("MAPGEN_SYSTEMS_DIR", "systems")),
        presets_dir: PathBuf::from(env_or("MAPGEN_PRESETS_DIR", "presets")),
        static_dir: PathBuf::from(env_or("MAPGEN_STATIC_DIR", "web/dist")),
        max_pixels: std::env::var("MAPGEN_MAX_PIXELS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(app::DEFAULT_MAX_PIXELS),
        tile_cache_bytes: std::env::var("MAPGEN_TILE_CACHE_MB")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .map_or(tiles::DEFAULT_BUDGET_BYTES, |mb| mb << 20),
    };
    // Worlds made before systems are no longer read (no migration, see
    // docs/system-spec.md): say so once, and leave removing them to the user.
    let old_worlds = PathBuf::from(env_or("MAPGEN_WORLDS_DIR", "worlds"));
    if std::fs::read_dir(&old_worlds).is_ok_and(|mut d| {
        d.any(|e| e.is_ok_and(|e| !e.file_name().to_string_lossy().starts_with('.')))
    }) {
        tracing::warn!(
            "{} holds worlds made before systems, which are no longer read: remove them with `mise run clean-worlds`",
            old_worlds.display()
        );
    }
    std::fs::create_dir_all(&config.systems_dir)?;
    let address = format!("{}:{}", env_or("HOST", "0.0.0.0"), env_or("PORT", "3000"));
    tracing::info!(?config, "listening on http://{address}");

    let listener = tokio::net::TcpListener::bind(&address).await?;
    axum::serve(listener, app::router(config))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
}
