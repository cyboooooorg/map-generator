//! Web server of the planet viewer.
//!
//! Configuration (environment variables):
//!
//! | Variable              | Default      | Purpose                                  |
//! |-----------------------|--------------|------------------------------------------|
//! | `MAPGEN_WORLDS_DIR`   | `worlds`     | generated worlds, read and written       |
//! | `MAPGEN_PRESETS_DIR`  | `presets`    | extra planet presets (`*.toml`)          |
//! | `MAPGEN_STATIC_DIR`   | `web/dist`   | built web front-end                      |
//! | `MAPGEN_MAX_PIXELS`   | `8388608`    | largest map accepted (width × height)    |
//! | `MAPGEN_TILE_CACHE_MB`| `1024`       | memory for worlds loaded for tiles (MiB) |
//! | `HOST`                | `0.0.0.0`    | listen address                           |
//! | `PORT`                | `3000`       | listen port                              |
mod api;
mod app;
mod catalog;
mod tiles;
mod worlds;

use std::path::PathBuf;

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=warn".into()),
        )
        .init();

    let config = app::Config {
        worlds_dir: PathBuf::from(env_or("MAPGEN_WORLDS_DIR", "worlds")),
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
    std::fs::create_dir_all(&config.worlds_dir)?;
    let address = format!("{}:{}", env_or("HOST", "0.0.0.0"), env_or("PORT", "3000"));
    tracing::info!(?config, "listening on http://{address}");

    let listener = tokio::net::TcpListener::bind(&address).await?;
    axum::serve(listener, app::router(config))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
}
