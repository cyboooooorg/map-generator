//! Web server of the planet viewer.
//!
//! Configuration (environment variables):
//!
//! | Variable              | Default      | Purpose                                  |
//! |-----------------------|--------------|------------------------------------------|
//! | `MAPGEN_SYSTEMS_DIR`  | `systems`    | drawn systems and their bodies           |
//! | `MAPGEN_DB`           | `db/mapgen.sqlite` | the index: systems, bodies, names, jobs |
//! | `MAPGEN_ARCHETYPES_DIR` | `archetypes` | extra planet archetypes (`*.toml`); missing: built-ins only |
//! | `MAPGEN_STATIC_DIR`   | `web/dist`   | built web front-end                      |
//! | `MAPGEN_MAX_PIXELS`   | `33554432`   | largest map accepted (width × height)    |
//! | `MAPGEN_TILE_CACHE_MB`| `1024`       | memory for worlds loaded for tiles (MiB) |
//! | `MAPGEN_NOISE_MAPS`  | off         | `true` (or `1`, `yes`, `on`): write the diagnostic `noise_maps/` of every body generated (debug) |
//! | `MAPGEN_EVICT_AFTER_DAYS` | `14` | days a body can stay unopened before its generated files are deleted (it is generated again when opened); `0`: never; a value that is not a number of days turns it off, with a warning |
//! | `HOST`                | `0.0.0.0`    | listen address                           |
//! | `PORT`                | `3000`       | listen port                              |
//! | `MALLOC_ARENA_MAX`    | glibc's      | glibc arenas; the Docker image sets `2`  |
//!
//! The index (see [`index`]) is a SQLite file the server owns. It is filled
//! from `MAPGEN_SYSTEMS_DIR` when missing or empty and reconciled with it
//! at every start; keep it on a volume that survives restarts.
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
mod export;
mod index;
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

/// Whether `key` is set to `true`, `1`, `yes` or `on` (any case).
fn env_flag(key: &str) -> bool {
    std::env::var(key).is_ok_and(|v| {
        matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "true" | "1" | "yes" | "on"
        )
    })
}

/// Days a body can stay unopened before its generated files are evicted
/// (`MAPGEN_EVICT_AFTER_DAYS`): 14 unless set, 0 turns eviction off. A value
/// that is not a whole number of days (`-1`, `off`) turns eviction off too,
/// with a warning: a typo must not delete files on a schedule nobody chose.
fn evict_after_days() -> u64 {
    parse_evict_after_days(std::env::var("MAPGEN_EVICT_AFTER_DAYS").ok().as_deref())
}

fn parse_evict_after_days(value: Option<&str>) -> u64 {
    let Some(value) = value else {
        return app::DEFAULT_EVICT_AFTER_DAYS;
    };
    value.trim().parse().unwrap_or_else(|_| {
        tracing::warn!(
            "MAPGEN_EVICT_AFTER_DAYS={value:?} is not a number of days: eviction is off"
        );
        0
    })
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
        db_path: PathBuf::from(env_or("MAPGEN_DB", "db/mapgen.sqlite")),
        archetypes_dir: PathBuf::from(env_or("MAPGEN_ARCHETYPES_DIR", "archetypes")),
        static_dir: PathBuf::from(env_or("MAPGEN_STATIC_DIR", "web/dist")),
        max_pixels: std::env::var("MAPGEN_MAX_PIXELS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(app::DEFAULT_MAX_PIXELS),
        tile_cache_bytes: std::env::var("MAPGEN_TILE_CACHE_MB")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .map_or(tiles::DEFAULT_BUDGET_BYTES, |mb| mb << 20),
        noise_maps: env_flag("MAPGEN_NOISE_MAPS"),
        evict_after_days: evict_after_days(),
    };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bad_eviction_delay_turns_eviction_off() {
        assert_eq!(parse_evict_after_days(None), 14);
        assert_eq!(parse_evict_after_days(Some("30")), 30);
        assert_eq!(parse_evict_after_days(Some(" 7 ")), 7);
        assert_eq!(parse_evict_after_days(Some("0")), 0);
        for bad in ["-1", "off", "", "1.5", "14 days"] {
            assert_eq!(parse_evict_after_days(Some(bad)), 0, "{bad:?}");
        }
    }
}
