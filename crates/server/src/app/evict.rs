//! Eviction: taking back the disk of the bodies nobody opens.
//!
//! A generated body is its files (`bodies/<body>/sizes/<W>x<H>/`, the
//! original resolution and the ones added): up to GBs for the larger
//! sizes. Everything needed to make it again is in `system.json` and the
//! index (the body's seed and parameters, the resolution it is displayed
//! at), so a body can lose its files and be generated again when someone
//! opens it. See `docs/analysis/15-storage.md` ("Step done: evicting bodies").
//!
//! - [`evict_body`] is the one function that evicts a body. The scheduled
//!   sweep ([`sweep_unopened`]) calls it, and so must the "reduce a
//!   system" action: for every body of the system, one call each.
//! - The sweep ([`spawn_sweeps`]) runs at startup and every
//!   [`SWEEP_EVERY`], for the bodies not opened for
//!   `MAPGEN_EVICT_AFTER_DAYS` days.
//!
//! An evicted body is "not generated" (`generated: false` in
//! `system.json` and the index) like a body never generated; the index
//! keeps the generator that made it ([`Index::evicted_generator`]), which
//! tells the two apart: opening the view of an evicted body queues its
//! generation (`POST .../generate`), and the answer says when the
//! current generator draws it differently.
use super::*;
use std::time::Duration;

/// Time between two sweeps.
pub const SWEEP_EVERY: Duration = Duration::from_secs(6 * 60 * 60);
const DAY_MS: u64 = 24 * 60 * 60 * 1000;

/// What [`evict_body`] took back.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Evicted {
    /// Resolutions deleted, the original included.
    pub sizes: usize,
    /// Bytes they took on disk.
    pub bytes: u64,
}

/// Why a body was not evicted.
#[derive(Debug, PartialEq, Eq)]
pub enum EvictError {
    /// A job of the system is queued or running (its own, or a resolution
    /// of one of its bodies): nothing is evicted while it may write the
    /// body. Try again later.
    Pending,
    /// No such system or body, or an unreadable `system.json`.
    Unknown(String),
    Failed(String),
}

impl std::fmt::Display for EvictError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "a generation of the system is pending"),
            Self::Unknown(e) | Self::Failed(e) => write!(f, "{e}"),
        }
    }
}

/// **The eviction entry point**: deletes every generated file of body
/// `body` of system `system`, all of `bodies/<body>/sizes/*`, the original
/// resolution too. Blocking: call it from a blocking thread.
///
/// What it keeps, so that the body can be generated again and looks the
/// same in lists:
///
/// - `system.json`, where the body is marked `generated: false`;
/// - the index rows: the body's seed and parameters, its displayed
///   resolution, its last opened date, and the generator that made it
///   (to say when the current one draws it differently); the names and
///   the pins;
/// - `bodies/<body>/thumb.png`, so that cards still look right.
///
/// It refuses (`Err(EvictError::Pending)`) when a job of the system is
/// queued or running, and checks that under the same lock job starts take
/// (`AppState::pending`, see `claim_size`, `queue_body`): a job cannot
/// start between the check and the deletion, and the files are out of
/// every route's sight (renamed away) before the lock is released. A body
/// that is not generated is not an error: `Evicted::default()`.
///
/// To reduce a system, call it for each of its bodies (all of
/// `system.bodies`, moons and giants too) from one blocking task, and
/// answer which were `Pending`:
///
/// ```ignore
/// let evicted: Vec<_> = system.bodies.iter()
///     .map(|b| (b.id.clone(), evict_body(&state, &id, &b.id)))
///     .collect();
/// ```
///
/// Opening such a body again queues its generation, at its displayed
/// resolution when one is recorded (`queue_body`, then the follow-up of
/// `start_job`), else at its original one.
pub fn evict_body(state: &AppState, system: &str, body: &str) -> Result<Evicted, EvictError> {
    if !systems::is_valid_system_id(system) || !systems::is_valid_body_id(body) {
        return Err(EvictError::Unknown(format!("invalid body {system}/{body}")));
    }
    let root = &state.config.systems_dir;
    let system_dir = systems::system_dir(root, system);
    let body_dir = systems::body_dir(root, system, body);
    let sizes_dir = body_dir.join(mapgen::sizes::SIZES_DIR);
    let doomed = systems::system_dir(root, system)
        .join("bodies")
        .join(format!(".tmp-evict-{body}-{}", crate::index::now_ms()));
    let sizes = mapgen::sizes::sizes_on_disk(&body_dir);
    {
        // The lock job starts take: no job of the system starts, or is
        // running, while the body's files go.
        let pending = state.pending.lock().expect("pending lock");
        if system_is_pending(&pending, system) {
            return Err(EvictError::Pending);
        }
        let mut file = mapgen::system::read_system(&system_dir).map_err(EvictError::Unknown)?;
        let entry = file
            .bodies
            .iter_mut()
            .find(|b| b.id == body)
            .ok_or_else(|| EvictError::Unknown(format!("system {system} has no body {body}")))?;
        let was_generated = std::mem::replace(&mut entry.generated, false);
        if !was_generated && !sizes_dir.exists() {
            return Ok(Evicted::default());
        }
        // The file first: killed in between, the body is "not generated"
        // with files left, which generating it again replaces.
        if was_generated {
            mapgen::system::write_system(&system_dir, &file).map_err(EvictError::Failed)?;
        }
        match std::fs::rename(&sizes_dir, &doomed) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(EvictError::Failed(format!("{}: {e}", sizes_dir.display()))),
        }
        // Under the lock too: a job finishing right after reads the tree
        // as it is now.
        state
            .index
            .refresh_system(root, system, Cause::Scan)
            .map_err(|e| EvictError::Failed(e.to_string()))?;
    }
    let freed = Evicted {
        sizes: sizes.len(),
        bytes: dir_bytes(&doomed),
    };
    if let Err(e) = std::fs::remove_dir_all(&doomed)
        && e.kind() != std::io::ErrorKind::NotFound
    {
        // The start's sweep clears what is left of it.
        tracing::warn!("could not remove {}: {e}", doomed.display());
    }
    let mut forgotten = false;
    for size in sizes {
        let key = size_key(system, body, size);
        forgotten |= state.tiles.forget(&key);
        state.drift_cache.forget(&key);
    }
    if forgotten {
        memory::release();
    }
    Ok(freed)
}

/// The bytes of the files under `dir`.
fn dir_bytes(dir: &Path) -> u64 {
    let Ok(read) = std::fs::read_dir(dir) else {
        return 0;
    };
    read.flatten()
        .map(|entry| match entry.metadata() {
            Ok(m) if m.is_dir() => dir_bytes(&entry.path()),
            Ok(m) => m.len(),
            Err(_) => 0,
        })
        .sum()
}

/// What a sweep did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SweepReport {
    /// Bodies evicted, and the bytes freed.
    pub evicted: usize,
    pub bytes: u64,
    /// Bodies left for the next sweep because a job of their system was
    /// pending.
    pub busy: usize,
    /// Bodies that could not be evicted.
    pub failed: usize,
}

/// When body `body` was generated: the modification time of its original
/// resolution's `world.json`. `None` when it cannot be told.
fn generated_at(state: &AppState, system: &str, body: &str) -> Option<u64> {
    let root = &state.config.systems_dir;
    let original = state.originals.original_size(root, system, body).ok()??;
    systems::size_revision(root, system, body, original)
}

/// Evicts the generated bodies not opened for `after_days` days at
/// `now_ms` (ms since the Unix epoch); `after_days` 0 evicts nothing.
/// A body counts from its last opened date, or from when it was
/// generated when that is later (a body never opened, or opened before it
/// was generated again). Logs the count and the bytes freed. Blocking.
pub fn sweep_unopened(state: &AppState, now_ms: u64, after_days: u64) -> SweepReport {
    let mut report = SweepReport::default();
    if after_days == 0 {
        return report;
    }
    let cutoff = now_ms.saturating_sub(after_days.saturating_mul(DAY_MS));
    let bodies = match state.index.generated_bodies() {
        Ok(bodies) => bodies,
        Err(e) => {
            tracing::error!("eviction sweep: bodies not listed: {e}");
            return report;
        }
    };
    for (system, body, opened) in bodies {
        let last = opened.max(generated_at(state, &system, &body));
        // Not dated (no file to tell): left alone.
        if last.is_none_or(|last| last >= cutoff) {
            continue;
        }
        match evict_body(state, &system, &body) {
            Ok(freed) => {
                report.evicted += 1;
                report.bytes += freed.bytes;
                tracing::info!(
                    "evicted {system}/{body}: {} resolutions, {} bytes",
                    freed.sizes,
                    freed.bytes
                );
            }
            Err(EvictError::Pending) => report.busy += 1,
            Err(e) => {
                report.failed += 1;
                tracing::warn!("could not evict {system}/{body}: {e}");
            }
        }
    }
    tracing::info!(
        "eviction sweep (bodies not opened for {after_days} days): {} evicted, {:.1} MiB freed, \
         {} busy, {} failed",
        report.evicted,
        report.bytes as f64 / f64::from(1 << 20),
        report.busy,
        report.failed
    );
    report
}

/// Starts the sweeps: one now, then one every [`SWEEP_EVERY`], while the
/// server runs. Nothing when eviction is off (`evict_after_days` 0).
/// Needs a Tokio runtime.
pub fn spawn_sweeps(state: Shared) {
    let days = state.config.evict_after_days;
    if days == 0 {
        tracing::info!("eviction of unopened bodies is off (MAPGEN_EVICT_AFTER_DAYS=0)");
        return;
    }
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(SWEEP_EVERY);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            let task = state.clone();
            let swept = tokio::task::spawn_blocking(move || {
                sweep_unopened(&task, crate::index::now_ms(), days)
            })
            .await;
            if let Err(e) = swept {
                tracing::error!("eviction sweep crashed: {e}");
            }
        }
    });
}
