//! Eviction: taking back the disk of the bodies nobody opens.
//!
//! A generated body is its files (`bodies/<body>/sizes/<W>x<H>/`, the
//! original resolution and the ones added): up to GBs for the larger
//! sizes. Everything needed to make it again is in `system.json` and the
//! index (the body's seed and parameters, the resolution it is displayed
//! at), so a body can lose its files and be generated again when someone
//! opens it. See `docs/analysis/15-storage.md` ("Step done: evicting bodies").
//!
//! - [`evict_body`] is the one function that evicts a body, whatever its
//!   age: the "reduce a system" action calls it for every body of the
//!   system, one call each. The scheduled sweep ([`sweep_unopened`]) calls
//!   [`evict_body_unopened_since`] instead, the same eviction preceded by
//!   an age check made under the lock (a body opened after the sweep
//!   listed it survives).
//! - The sweep ([`spawn_sweeps`]) runs at startup and every
//!   [`SWEEP_EVERY`], for the bodies not opened for
//!   `MAPGEN_EVICT_AFTER_DAYS` days. It also clears the
//!   `.tmp-evict-*` directories a killed server left.
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
    /// The body was opened (or generated again) after the cutoff given to
    /// [`evict_body_unopened_since`]: it is not unopened any more.
    Opened,
    /// No such system or body, or an unreadable `system.json`.
    Unknown(String),
    Failed(String),
}

impl std::fmt::Display for EvictError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "a generation of the system is pending"),
            Self::Opened => write!(f, "the body was opened lately"),
            Self::Unknown(e) | Self::Failed(e) => write!(f, "{e}"),
        }
    }
}

/// **The eviction entry point**: deletes every generated file of body
/// `body` of system `system`, all of `bodies/<body>/sizes/*`, the original
/// resolution too, whatever the body's age and last opened date. Blocking:
/// call it from a blocking thread. The scheduled sweep does not call this
/// one but [`evict_body_unopened_since`], which checks the age first.
///
/// What it keeps, so that the body can be generated again and looks the
/// same in lists:
///
/// - `system.json`, where the body is marked `generated: false` (its
///   modification time is kept: the library dates a system by that file,
///   and an eviction is no change of the system);
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
/// every route's sight (renamed away) before the lock is released. Deleting
/// or replacing the system takes the same key (`hold_system`), so neither
/// can run meanwhile. A body that is not generated is not an error:
/// `Evicted::default()`. When the files cannot be moved aside, `system.json`
/// is put back as it was.
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
#[allow(
    dead_code,
    reason = "the entry point of the future reduce-a-system action; the sweep checks the age first"
)]
pub fn evict_body(state: &AppState, system: &str, body: &str) -> Result<Evicted, EvictError> {
    evict(state, system, body, None)
}

/// [`evict_body`] for a body that was not opened since `cutoff_ms` (ms
/// since the Unix epoch): the age is read again under the lock, just before
/// the deletion, so a body opened after the caller chose it (the sweep
/// lists bodies, then evicts them one by one, for minutes) is kept and
/// answers `Err(EvictError::Opened)`. A body counts from its last opened
/// date, or from when it was generated when that is later; a body that
/// cannot be dated is kept too.
pub fn evict_body_unopened_since(
    state: &AppState,
    system: &str,
    body: &str,
    cutoff_ms: u64,
) -> Result<Evicted, EvictError> {
    evict(state, system, body, Some(cutoff_ms))
}

fn evict(
    state: &AppState,
    system: &str,
    body: &str,
    unopened_since: Option<u64>,
) -> Result<Evicted, EvictError> {
    if !systems::is_valid_system_id(system) || !systems::is_valid_body_id(body) {
        return Err(EvictError::Unknown(format!("invalid body {system}/{body}")));
    }
    let root = &state.config.systems_dir;
    let system_dir = systems::system_dir(root, system);
    let body_dir = systems::body_dir(root, system, body);
    let sizes_dir = body_dir.join(mapgen::sizes::SIZES_DIR);
    let doomed = systems::system_dir(root, system)
        .join("bodies")
        .join(format!("{EVICT_PREFIX}{body}-{}", crate::index::now_ms()));
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
        if let Some(cutoff) = unopened_since
            && last_activity(state, system, body).is_none_or(|last| last >= cutoff)
        {
            return Err(EvictError::Opened);
        }
        // The file first: killed in between, the body is "not generated"
        // with files left, which generating it again replaces.
        if was_generated {
            write_system_keeping_date(&system_dir, &file).map_err(EvictError::Failed)?;
        }
        match std::fs::rename(&sizes_dir, &doomed) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                // Nothing moved: the body is generated still.
                if was_generated {
                    if let Some(entry) = file.bodies.iter_mut().find(|b| b.id == body) {
                        entry.generated = true;
                    }
                    if let Err(back) = write_system_keeping_date(&system_dir, &file) {
                        tracing::error!("{system}/{body} not marked generated again: {back}");
                    }
                }
                return Err(EvictError::Failed(format!("{}: {e}", sizes_dir.display())));
            }
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
        // The next sweep clears what is left of it ([`clear_leftovers`]).
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

/// Writes `system.json` and gives the file the modification time it had:
/// the index dates a system by that file (the library sorts and groups by
/// it), and marking a body evicted, or put back, does not change the
/// system.
fn write_system_keeping_date(dir: &Path, system: &mapgen::system::System) -> Result<(), String> {
    let file = dir.join(systems::SYSTEM_FILE);
    let before = std::fs::metadata(&file).and_then(|m| m.modified());
    mapgen::system::write_system(dir, system)?;
    if let Ok(modified) = before {
        std::fs::File::options()
            .write(true)
            .open(&file)
            .and_then(|f| f.set_modified(modified))
            .map_err(|e| format!("{}: {e}", file.display()))?;
    }
    Ok(())
}

/// The date of a system's `system.json`, put back when this is dropped:
/// what a job that only makes evicted bodies again holds, so that the
/// library, which dates a system by that file, does not see the system
/// move when a body comes back (as [`write_system_keeping_date`] does for
/// the eviction itself). Dropped also when the job is cancelled midway.
pub struct KeptDate {
    file: std::path::PathBuf,
    modified: std::time::SystemTime,
}

impl KeptDate {
    /// Holds the date of system `system`'s file when generating `body` and
    /// its neighbourhood would only make bodies that were evicted again,
    /// else `None`: a body generated for the first time dates the system
    /// as before (the new world is a change to it). Blocking.
    pub fn when_only_restoring(
        index: &crate::index::Index,
        dir: &Path,
        system: &str,
        body: &str,
    ) -> Option<Self> {
        let file = dir.join(systems::SYSTEM_FILE);
        let wanted = mapgen::system::neighbourhood(&mapgen::system::read_system(dir).ok()?, body);
        let evicted = index.evicted_bodies(system).ok()?;
        if wanted.is_empty() || !wanted.iter().all(|id| evicted.contains(id)) {
            return None;
        }
        let modified = std::fs::metadata(&file).and_then(|m| m.modified()).ok()?;
        Some(Self { file, modified })
    }
}

impl Drop for KeptDate {
    fn drop(&mut self) {
        let put_back = std::fs::File::options()
            .write(true)
            .open(&self.file)
            .and_then(|f| f.set_modified(self.modified));
        if let Err(e) = put_back {
            tracing::warn!("{} keeps its new date: {e}", self.file.display());
        }
    }
}

/// Prefix of the directory a body's `sizes/` is renamed to while it is
/// deleted: `bodies/.tmp-evict-<body>-<ms>`.
const EVICT_PREFIX: &str = ".tmp-evict-";
/// A `.tmp-evict-*` directory younger than this may belong to an eviction
/// still deleting it: [`clear_leftovers`] leaves it.
const LEFTOVER_AFTER: Duration = Duration::from_secs(60 * 60);

/// Deletes the `bodies/.tmp-evict-*` directories under the systems
/// directory that an eviction killed (or failed) midway left, older than
/// an hour. The startup sweep (`systems::sweep`) removes them all; this
/// one runs with every eviction sweep, for a server that stays up. Blocking.
pub fn clear_leftovers(state: &AppState, now_ms: u64) {
    let root = &state.config.systems_dir;
    let cutoff = now_ms.saturating_sub(LEFTOVER_AFTER.as_millis() as u64);
    for system in systems::system_ids(root) {
        let bodies = systems::system_dir(root, &system).join("bodies");
        let Ok(read) = std::fs::read_dir(&bodies) else {
            continue;
        };
        for entry in read.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let Some(rest) = name.strip_prefix(EVICT_PREFIX) else {
                continue;
            };
            // `<body>-<ms>`: dated by its name; one that cannot be read is old.
            let made = rest
                .rsplit('-')
                .next()
                .and_then(|ms| ms.parse::<u64>().ok());
            if made.is_some_and(|made| made >= cutoff) {
                continue;
            }
            match std::fs::remove_dir_all(entry.path()) {
                Ok(()) => tracing::info!("removed {}", entry.path().display()),
                Err(e) => tracing::warn!("could not remove {}: {e}", entry.path().display()),
            }
        }
    }
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
    /// Bodies opened since the sweep listed them, kept.
    pub opened: usize,
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

/// When a body was last in use: its last opened date, or when it was
/// generated when that is later. The index starts a body's opened date at
/// the time it first saw it (`Index::refresh_system`), so an index rebuilt
/// from the tree does not count a body from the age of its files. `None`
/// when it cannot be told.
fn last_activity(state: &AppState, system: &str, body: &str) -> Option<u64> {
    let opened = state.index.last_opened(system, body).ok().flatten();
    opened.max(generated_at(state, system, body))
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
    clear_leftovers(state, now_ms);
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
        // The listing is a snapshot: the age is checked again under the lock.
        match evict_body_unopened_since(state, &system, &body, cutoff) {
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
            Err(EvictError::Opened) => report.opened += 1,
            Err(e) => {
                report.failed += 1;
                tracing::warn!("could not evict {system}/{body}: {e}");
            }
        }
    }
    tracing::info!(
        "eviction sweep (bodies not opened for {after_days} days): {} evicted, {:.1} MiB freed, \
         {} busy, {} opened meanwhile, {} failed",
        report.evicted,
        report.bytes as f64 / f64::from(1 << 20),
        report.busy,
        report.opened,
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
