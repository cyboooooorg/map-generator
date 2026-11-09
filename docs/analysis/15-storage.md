# 15. Storage

Study of how the app stores its systems and bodies today, and of the
alternatives (a database, an object store such as MinIO, or others), to
decide what to keep, what to change and in what order. Step 1 of the
recommendation (the SQLite index) is implemented, see *Step 1 done*
below, and so is the system export (decision 6, see *Export done*);
the rest is not.

## Today

Everything lives on the local disk, in directories bind-mounted into
the container (`compose.yaml`): `systems/` (`MAPGEN_SYSTEMS_DIR`) and
`db/`. The old `worlds/` is gone (see #303). The layout is in
`docs/system-spec.md`, *Storage*:

```text
systems/<system-id>/
├── system.json        the system as drawn (written once)
├── names.json         names the user gave, by body id (rewritten on rename)
└── bodies/<body-id>/
    ├── thumb.png      from the original resolution
    └── sizes/<W>x<H>/ one per resolution, the original included
        ├── world.png, world.svg, legend.png
        ├── raw_data/      world.json and the layers, 16-bit PNGs (+ rivers…)
        └── noise_maps/    diagnostic images of every noise layer (optional)
```

How it is used:

- **Writes.** A generation exports a body into a hidden temporary
  directory, then renames it into place (`crates/server/src/staging.rs`):
  a body is complete or absent, never half-written. `system.json` is
  written once when the system is drawn; `names.json` is rewritten whole
  on each rename.
- **Reads.** The viewer fetches files straight from disk through a
  static file service (`/systems/…`, `ServeDir`). The API reads
  `system.json` for summaries, by listing the directory and opening every
  system's file (`systems::list`). Detail tiles are rendered on demand
  from `raw_data/` and kept in an in-memory LRU (`MAPGEN_TILE_CACHE_MB`).
- **Jobs.** The generation queue and the job history are in memory: they
  are lost on a restart.

Production on 2026-11-06 (17 systems, 1 765 files):

| Part          | Size   | What it is                                                     |
| ------------- | ------ | -------------------------------------------------------------- |
| `raw_data/`   | 2.6 GB | the layers the viewer and the tiles read; the source of truth  |
| `world.svg`   | 621 MB | vector export, up to 96 MB a body; the viewer does not read it |
| `world.png`   | 368 MB | the rendered map, read by older worlds and downloads           |
| `noise_maps/` | 312 MB | diagnostic images, on one body only; never read by the app     |
| JSON, thumbs… | small  | `system.json`, `world.json`, `names.json`, thumbnails          |
| **Total**     | 3.9 GB | measured before the old `worlds/` was dropped                                |

A large body (e III of MGS 50909, 4096 × 2048) takes 659 MB, half of it
diagnostics.

## What is not optimal

1. **No index.** Listing systems opens every `system.json`; any query
   (by star type, by date, by body type, the bodies generated) is a scan.
   It is fine at 17 systems, not at thousands.
2. **State that should survive is in memory.** Jobs, their progress and
   their history vanish on a restart; a crash mid-queue loses the queue.
3. **Source, derived data and diagnostics are mixed.** `raw_data/` is the
   source; `world.png`, `world.svg`, thumbnails and tiles derive from it;
   `noise_maps/` is debug output. They share one directory, one lifetime
   and one backup, so the store holds about 30 % it could rebuild or drop.
4. **User data is mixed with generated data.** `names.json` is the only
   thing a user wrote; it sits next to files a generation can replace.
5. **One host.** The data is a bind mount of one checkout: two server
   instances cannot share it, the container's UID must match the host's,
   and moving the app to another machine means copying the tree.
6. **No integrity or version record.** Nothing lists a body's files, their
   sizes or hashes, or the generator version that wrote them, except
   implicitly in `world.json`. A partial copy or a format change is found
   only when a read fails.
7. **No backup or retention policy.** Everything is kept forever, and a
   backup is a copy of the whole tree.

What is right and should be kept: the atomic directory commit, files
served as they are (no transcoding), and the determinism of generation.
The same seed and parameters give the same body, bit for bit (the
project's rule: no output-changing work), so every generated file can in
principle be rebuilt from `system.json`.

## Options

### A. Files, kept, with a small index (SQLite)

Keep the files where they are. Add one SQLite database (`rusqlite` or
`sqlx`) beside them for what needs queries or must survive: systems and
bodies (summary fields), user names, jobs and their history, and a
manifest of each body's files (path, size, hash, generator version).

- **For:** smallest change; no new service; SQLite runs in the server
  process and is a single file to back up; the static file service stays.
- **Against:** still one host; the index must be kept in step with the
  files (rebuilt from the tree at startup if they disagree).

### B. A database for metadata and blobs (PostgreSQL)

Move everything, layers included, into PostgreSQL (`bytea` or large
objects).

- **For:** one system to back up, transactions over everything.
- **Against:** a 30 MB PNG per layer in a database is slow to stream and
  bloats backups and the WAL; the viewer can no longer fetch files
  statically; an extra service to run. Not recommended for the blobs.

### C. Object storage for blobs, a database for metadata

Put the files in an S3-compatible store, behind a `BlobStore` trait in
the server (local disk and S3 back ends), and the metadata in SQLite or
PostgreSQL as in A.

- **For:** the standard shape for large immutable files; several hosts
  can share it; lifecycle rules (expiry, tiers) and replication come with
  the store; the viewer can fetch through presigned URLs or the server.
- **Against:** a service to run and secure; local runs need it too (or
  the disk back end); one more hop per file.

Self-hosted S3 stores, as of late 2025 to 2026 (secondary sources; check
the projects' own pages before choosing):

- **MinIO:** the community edition moved to maintenance mode in late
  2025. It is now distributed as source only, without official images or
  binaries, and its repository is reported archived in April 2026. AGPL.
  Not a good base for something new.
- **Garage** (Deuxfleurs): a single Rust binary, light, made for small
  self-hosted clusters. AGPL. Its S3 coverage is partial (no versioning,
  no IAM), which this app does not need.
- **SeaweedFS:** strong for very many small files. It is more to run (a
  metadata store) and further from strict S3.
- **RustFS:** the closest to MinIO, Apache-2.0. It is young (open-sourced
  in July 2025), with a smaller ecosystem.
- **Ceph RGW:** complete, and heavy for this scale.

### D. Store less: rebuild instead of keeping

As generation is deterministic, keep the source (`system.json`, the
requests, the names) and treat every generated file as a cache. Drop a
body's files when space runs short (least recently used) and regenerate
it when it is opened again. Drop `noise_maps/` and `world.svg` by default
and generate them on request.

- **For:** storage follows use, not history; backups shrink to
  kilobytes; no data is ever really lost.
- **Against:** reopening an evicted body costs a generation (seconds to
  minutes); the generator must stay bit-exact across versions, or a cache
  key must carry the generator version, and an old body is then rebuilt
  as the new version draws it.

## Comparison

| Criterion                       | A: files + SQLite | B: Postgres | C: S3 + DB  | D: rebuild |
| ------------------------------- | ----------------- | ----------- | ----------- | ---------- |
| Change to the code              | small             | large       | medium      | medium     |
| New service to run              | none              | Postgres    | S3 (+DB)    | none       |
| Queries over systems and bodies | yes               | yes         | yes         | with A     |
| Jobs survive a restart          | yes               | yes         | yes         | with A     |
| Several hosts                   | no                | yes         | yes         | with C     |
| Space used                      | as today          | more        | as today    | much less  |
| Viewer reads files directly     | yes               | no          | yes (URLs)  | yes        |
| Backup                          | small DB + tree   | one DB      | DB + bucket | DB only    |

## Recommendation

A first, then D, with C kept possible:

1. **A: an SQLite index**, the database the server owns. It holds the
   systems, bodies, names, jobs and file manifests. It is rebuilt from
   the tree when missing, so today's data needs no migration step.
2. **Separate the kinds of data**, as part of A: diagnostics
   (`noise_maps/`, `world.svg`) become opt-in or move to a cache directory
   that can be wiped, while user data (names) lives in the database
   only.
3. **D: rebuild on demand**, once the manifest records the generator
   version: a cache limit with LRU eviction, and regeneration on a miss.
   This is where most of the space goes back.
4. **C only when needed**, for example a second host, remote hosting or
   cloud backups. A `BlobStore` trait from step 1 keeps it a back-end
   swap. Choose then between Garage (small, simple) and RustFS (closer to
   MinIO), not MinIO itself.

PostgreSQL (B) is worth it only if several server instances must write
the metadata at once; SQLite covers one instance.

## Decisions (2026-11-08)

The user's answers to the study's questions:

1. **Scale:** tens to a few hundred systems, for the user alone.
2. **Hosting:** Docker on this machine, perhaps on a server later; no
   second host and no cloud. SQLite covers it, and no object store (C) is
   needed for now.
3. **Rebuilding:** accepted. The database keeps each body's seed and
   request; a body not opened for **14 days** loses its generated files
   and is regenerated when it is opened again.
4. **Generator version:** a body regenerated after the generator changed
   is drawn by the current version (option a), even though it then
   differs from what was evicted. The database records the version each
   body was made with, so the viewer can say so.
5. **Diagnostics:** `world.svg` and `noise_maps/` are produced only on
   request (an "Export SVG" action and a debug option), about a quarter
   of today's disk.
6. **Backups:** none automatic. The app exports a system's seeds and
   requests (JSON) and its images (PNG) instead.
7. **Persistence:** the database file survives restarts and image
   rebuilds. It lives on the host next to `systems/`, mounted like it
   (`./db:/data/db` in `compose.yaml`), never inside the container. Once
   it holds what the tree no longer does (names, last opened dates, the
   seeds of evicted bodies), losing it loses data, so it is the one file
   to export or copy. SQLite on a bind mount of the Colima VM (virtiofs):
   one server process writes. First decided as the default rollback journal,
   WAL being unchecked on the mount; revised (see *Step 1 done*): WAL is
   tried at every start, with a self-check and a fallback.

The steps that follow are in the backlog, item 22 (diagnostics on request: done, see *Step done: world.svg and noise_maps on request*).

## Step 1 done: the SQLite index

Implemented in `crates/server/src/index.rs` (backlog item 22, step 1).
What was built, and where it departs from the plan above:

- **Library:** `rusqlite` with the `bundled` feature (SQLite compiled in,
  no system library in the image). The server touches the database from
  a few short blocking tasks, so a synchronous driver behind a mutex fits;
  `sqlx` would add an async pool and its machinery for no gain.
- **Journal:** WAL (decision 7 revised at the user's request: readers do
  not block the writer, commits are cheaper). WAL needs the `-wal` and
  `-shm` files and shared memory on the mount, so at every open the index
  sets `journal_mode=WAL`, checks that the pragma answers `wal`, and does
  a write/read round-trip; if anything fails it logs a warning and uses
  the rollback journal. `synchronous=NORMAL` in WAL mode: no corruption,
  but a power cut may lose the last commits. `db/` then holds
  `mapgen.sqlite`, `-wal` and `-shm`; back it up with the server stopped
  or with `sqlite3 .backup`. To be confirmed on the real Colima mount.
- **Replacing a system** keeps the names that still match one of its
  bodies (as the command line does), in `names.json` and the index, and
  its pin.
- **Pins:** the user pins systems to the top of the main menu
  (`PUT`/`DELETE /api/systems/{id}/pin`). Stored in the index only, in
  `systems.pinned_ms` (since layout version 2). Not mirrored to a file, by choice:
  a pin is a view preference, cheap to redo, and a mirror would add a
  second source of truth to reconcile. A lost database loses the pins,
  not the names. This is the first user data that only the database holds.
  Pins are lost too when a start finds a system's directory absent
  (`systems/` not mounted: the reconcile drops the system) and when a
  corrupt file is set aside (the `.corrupt-<ms>` file still holds them).
- **Layout versions:** version 3 since the resolutions (see
  *Resolutions* below). No migration code: a file of any other layout,
  older or newer, is renamed `<file>.layout-<version>-<ms>` with its
  `-wal` and `-shm`, a warning is logged and a new index is built from
  the tree. What only the index held (pins, displayed sizes, last
  opened dates, job history) stays in the file set aside. A process sets
  the file aside only if it is still the file it read (same device and
  inode), so two processes opening it at once set it aside once; an
  empty file gets its layout inside a `BEGIN IMMEDIATE` transaction that
  reads `user_version` again. (Layout 2 added `systems.pinned_ms` by
  migration; that migration went with layout 3.)
- **Single names:** `PUT /api/systems/{id}/name` (the star) and
  `PUT /api/systems/{id}/bodies/{body}/name` merge one name into the names
  under the connection lock (`names.json` first, then the index), so a
  name given since a client read the list is kept; `PUT /names` (bulk)
  takes the same lock. `names.json` is written to a temp file of its own
  (process id and counter) then renamed, so concurrent edits never share
  it.
- **Location:** `MAPGEN_DB`, `/data/db/mapgen.sqlite` in the container,
  `./db` on the host (`compose.yaml`, `mise run up` creates it, `db/` is
  git-ignored).
- **Tables:** `systems` (summary fields, `system.json` modification time,
  the request it was drawn from), `bodies` (summary fields, seed and
  parameters as JSON, generator crate version and format revision, last
  opened), `names`, `jobs`. Layout version in `PRAGMA user_version`.
- **Rebuilt, then reconciled:** one code path. At every start the tree is
  compared with the index by modification times: missing or changed
  systems are read again, removed ones dropped. An absent or empty
  database is simply the case where everything is missing. The
  old `worlds/` is neither mounted nor read, so it is not scanned.
- **Reads:** `GET /api/systems` and the name lookups read the index.
  `GET /api/systems/{id}` and job creation still read `system.json`
  (the full system is what they answer). The API only gained the
  `pinned` flag of `GET /api/systems`. Body routes (tiles, epochs, hex
  grids, thumbnails) need each body's original resolution, from its
  params in `system.json`: the server keeps it in memory per system and
  reads the file again only when its modification time or length
  changes, so a tile costs one `stat` of `system.json`, not a parse.
- **Names:** written to the index and, for now, to `names.json`, so the
  tree is self-sufficient. A `names.json` edited by hand is read again at
  the next start. Writing names to the database only is left for the
  step that separates user data from the tree.
- **Jobs:** stored when queued, running and finished (not their progress).
  At start, jobs left queued or running are marked failed
  (`interrupted by a server restart`), the last 20 finished ones come
  back in `GET /api/jobs`, and ids continue above the highest stored.
- **Last opened:** set when a body's tiles, epochs, drift previews or hex
  grid are served, when it is queued for generation, and when files under
  `/systems/<id>/bodies/<body>/` are fetched, except `thumb.png` (the
  library asks for all of them). At most one write a minute per body.
- **Generator version:** `GENERATOR_VERSION` (the crate version) and
  `GENERATOR_FORMAT` (`mapgen::export::raw_data::FORMAT_VERSION`). A body
  generated by a job is recorded with both; one found in the tree has no
  crate version (unknown) and the `format_version` of its `world.json`.
- **Abstraction:** no `BlobStore` trait yet. The tree is read through
  `systems.rs` and `staging.rs`, the metadata through `Index`; a trait
  over the files is introduced with step 3, which is the first to delete
  files.
- **Corrupt file:** a file that is not a database is kept aside as
  `mapgen.sqlite.corrupt-<ms>` and rebuilt from the tree.

## Step done: world.svg and noise_maps on request

Decision 5, implemented (backlog item 22, issue #327). Neither
`world.svg` nor `noise_maps/` is written when the server generates a
body, which saves about a quarter of today's disk on new bodies. Bodies
already on disk keep the files they have.

- **Switches:** `mapgen::export::Diagnostics { svg, noise_maps }`, off by
  default, goes from the callers to `generate_neighbourhood_with` and
  `generate_size`. `ExportOptions::svg` is off by default too.
- **SVG:** `GET /api/systems/{id}/bodies/{body}/world.svg?size=WxH` draws
  it on request and sends it as a download; nothing is stored. It is drawn
  from the stored raw data (`load_world`, 16-bit layers) and, when those do
  not read back, from the body's seed and request (`mapgen::system::body_svg`).
  The world card has an "Export SVG" button in place of the old link to
  the stored file.
- **Noise maps:** the debug option is `MAPGEN_NOISE_MAPS=true` on the
  server (about 10 bytes per pixel more memory during a generation).
- **Command line:** keeps writing both by default, since whoever runs it
  asked for files and has no app to export from; `--no-svg` and
  `--no-noise-maps` (or `MAPGEN_NO_SVG`, `MAPGEN_NO_NOISE_MAPS`) skip them.

## Step done: evicting bodies not opened for 14 days

Decisions 3 and 4, implemented (issue #328, step 3 of the plan). Code in
`crates/server/src/app/evict.rs`.

- **The function:** `evict::evict_body(state, system, body)` deletes every
  generated file of one body (`bodies/<body>/sizes/*`, originals
  included) and keeps `system.json` (the body marked `generated: false`),
  the index rows, the names and pins, and `bodies/<body>/thumb.png`. It
  is the one eviction path, whatever the body's age: the "reduce a system"
  action must call it for every body of a system (see the planet
  resolutions design, *Reduce and #328*; its doc comment says how). The
  sweep calls `evict_body_unopened_since(state, system, body, cutoff)`
  instead, the same eviction behind an age check (next point).
  `system.json` is rewritten with its previous modification time: the
  index dates a system (and the library sorts and groups) by that file,
  and an eviction is no change of the system. If the files cannot be
  moved aside, `system.json` is put back as it was (`generated: true`).
- **No race with jobs:** it checks and deletes under the same
  `AppState::pending` lock that `claim_size` and `queue_body` take to
  start a job, and refuses (`EvictError::Pending`) when any key of the
  system is pending: its own job (`systems/<id>`) or a resolution of any
  of its bodies (`<id>/<body>/<WxH>`). A neighbourhood job can generate
  any body of the family, so the check is per system, not per body. The
  sizes directory is renamed away, `system.json` and the index updated,
  and only then is the lock released and the files removed. Tests: a
  pending key refuses the eviction, and an eviction waits for the lock.
  Deleting a system and replacing it (`POST /api/systems?replace=true`)
  hold the system's key (`hold_system`) from their check to the end of
  their work, so an eviction (or a job) cannot recreate or overwrite
  `system.json` in between.
- **Age checked under the lock:** the sweep lists the candidate bodies
  from a snapshot and evicts them one by one, for minutes. The age (last
  opened date, `world.json` time) is read again inside the eviction,
  under the lock, against the sweep's cutoff: a body opened since stays
  (`EvictError::Opened`, counted as "opened meanwhile" in the log).
- **Schedule:** a tokio task started by `router()` sweeps at startup,
  then every 6 hours (`evict::SWEEP_EVERY`). `MAPGEN_EVICT_AFTER_DAYS`
  (default 14; 0 turns it off; a value that is not a number of days, such
  as `-1` or `off`, turns it off too, with a warning). Each sweep logs
  bodies evicted, MiB freed, bodies left because a job was pending or
  because they were opened meanwhile, and failures. It first removes the
  `bodies/.tmp-evict-*` directories older than an hour that a killed
  server left (the startup sweep removes them all).
- **Age of a body:** the later of its `last_opened_ms` and the
  modification time of its original `world.json` (when it was generated).
  A body whose age cannot be told is left alone. Floor after a rebuild:
  a body the index finds already generated and did not know (rebuilt
  index after a layout bump or a lost `db/`, restored or copied tree)
  starts with `last_opened_ms` = that moment (`store()`, cause `Scan`;
  no new column, `SCHEMA_VERSION` stays 3). Its age counts at most from
  when the current index first saw it, so a rebuild never evicts every
  body older than 14 days at the next start.
- **What counts as opened:** only a body's own view. `note_opened` writes
  the date (at most once a minute per body) for the files under
  `/systems/<id>/bodies/<body>/` (not `thumb.png`), tiles, epochs, drift
  previews, hex grids, `world.svg`, `generate` and the resolutions' POST.
  Not the library, `GET /api/systems/{id}`, the thumbnail route or the
  resolutions listing, and not a **peek**: a request for a file with
  `?peek=1`, which the middleware `note_served` ignores. The viewer adds
  it (`peeked` in `web/src/lib/api/client.ts`) to the textures of other
  bodies in the sky, the star view's `colour_small.png` markers and the
  host planet of a moon (its `world.json` and layers): seen from
  elsewhere, not opened. A peek has its own URL, so the browser caches it
  apart from the body's own load. Tests pin both sides.
- **Evicted state:** the index keeps `generator_version` and
  `generator_format` of a body that is no longer generated (`store()`
  used to clear them), with format 0 (`UNKNOWN_FORMAT`) when the body had
  none. A body with `generated = 0` and a generator is *evicted*:
  `GET /api/systems/{id}` lists them in `evicted`. No schema change:
  `SCHEMA_VERSION` stays 3. A rebuilt index loses the record: an evicted
  body then reads as never generated.
- **Regeneration:** opening the view of an evicted body makes the viewer
  call `POST …/generate`, which noted the body opened and queues the
  neighbourhood job (the original resolution, from `system.json`). When
  that job is done and the index remembers a displayed size for **the body
  opened** that is not on disk, `start_job` queues that size too
  (`regenerate_displayed_size`); the size job marks it displayed. Its
  neighbours made by the same job (moons, giant, siblings) stay at their
  original size: nobody opened them, and a later open brings them back at
  theirs. Opening a moon regenerates its neighbourhood as on-demand
  generation does. The Viewer queues it once (`generateShown`); the card's
  button uses `generateBody` of `App.svelte`.
- **Generator notice:** `POST …/generate` answers `regenerated`: one
  `{body, from}` per body of the family (the planet and its moons) that
  is evicted and whose generator differs, with the recorded version and
  format and the current ones, on every answer while the job lives (and
  only when the job answered is the family's: a job of another family
  carries no note). A planet's job makes its evicted moons too, so they
  are named, not only the body asked. The viewer toasts once per body and
  session, naming the body by its name or designation.
- **Date of a restored system:** a job whose bodies were all evicted
  (the index holds their generator) puts `system.json`'s modification
  time back when it ends (`KeptDate` in `app/evict.rs`), so restoring a
  body does not move its system to the top of the library. A job that
  also makes a body never generated before keeps the usual behaviour: the
  system is dated by that generation.
- **Build id:** `GENERATOR_VERSION` is `<crate version>+<git commit>`
  (`0.1.0+1a2b3c4d5e6f`), recorded in `generator_version` (no schema
  change). `crates/server/build.rs` takes the commit from `GIT_SHA` of
  the environment, else `git rev-parse HEAD`; with neither, the version is
  the bare crate version. The Docker image has no `.git` (`.dockerignore`):
  `compose.yaml` passes `GIT_SHA` as a build argument (`mise run up`
  exports it from `git rev-parse HEAD`), which the `Dockerfile` hands to
  the Rust stage. The notice fires when the recorded and the current
  versions differ: other crate versions, or both build ids known and
  different. A version without a build id (no git at build time, or
  recorded before build ids) is only compared by crate version, so a build
  without git never announces "older world" on every regeneration.
  `format_version` is compared as before.
- **Thumbnail:** `GET …/thumb` serves `thumb.png` of an evicted body
  (no-cache), so cards look as before.

## Resolutions

A body has its original resolution (the size in its parameters in
`system.json`) and any number of added ones, all laid out alike in
`bodies/<body>/sizes/<W>x<H>/` (see *Today*). What each part keeps:

- **The tree** is the truth for which sizes exist: a size is present
  when `sizes/<W>x<H>/raw_data/world.json` is
  (`mapgen::sizes::sizes_on_disk`). A size is generated into
  `sizes/.tmp-<W>x<H>/` and renamed into place
  (`mapgen::system::generate_size`; a size job that fails, crashes or
  is cancelled removes it); a deleted one is first renamed to
  `sizes/.tmp-delete-<W>x<H>-<ms>/`, then removed. The start's sweep
  clears both kinds of leftovers a killed server leaves.
- **The index** remembers the displayed size of each body
  (`bodies.displayed_width`, `displayed_height`, NULL for the original;
  layout 3), kept when the system is read again from the tree and reset
  when it is drawn again. It is used only while that size is on disk:
  otherwise the original is displayed. A size job that is done sets it
  to the size it added (the server does it, so no tab is needed), and
  `PUT …/displayed-size` sets it to any size on disk. A lost index shows every body at
  its original size. Jobs record the size they add (`jobs.size`,
  `<W>x<H>`, NULL for a body and its neighbourhood).
- **Caches** key a world by its size: `systems/<id>/<body>/<W>x<H>` in
  the tile cache and the drift previews. Deleting a size forgets that
  key in both.
- **Jobs:** a size job takes the key `<id>/<body>/<W>x<H>` in the list of
  pending generations, so two requests for the same size queue one job,
  and a size cannot be deleted while it is queued or running. It waits
  on the one generation queue with every other job, so it never runs
  beside its system's own job. A system cannot be deleted or replaced
  while one of its sizes is pending.

## Sources

- MinIO community edition in maintenance mode: [ayedo](https://ayedo.de/en/posts/minio-im-maintenance-mode/),
  [DEV Community](https://dev.to/alexandrev/minio-in-maintenance-mode-what-it-means-for-the-community-edition-oem-users-and-open-source-3o4j).
- Self-hosted S3 alternatives compared (Garage, SeaweedFS, RustFS, Ceph):
  [glukhov.org](https://glukhov.org/data-infrastructure/object-storage/self-hosted-s3-alternatives/),
  [Pinggy](https://pinggy.io/blog/minio_archived_self_hosted_s3_alternatives/).
- The project's own layout and behaviour: `docs/system-spec.md`
  (*Storage*), `crates/server/src/staging.rs`, `crates/server/src/systems.rs`,
  `compose.yaml`.

## Export done: a system's seeds, requests and images

Implemented in `crates/server/src/export.rs` (issue #329, decision 6).
No backup is automatic, so the app offers the export instead:
`GET /api/systems/{id}/export` answers `<system-id>.zip`, and the
system's card in the main menu has an **Export** button (⤓) that
downloads it. What it does, and where it departs from the plan above:

- **Archive:** `<system-id>/system.json` and, for each generated body,
  `bodies/<body>/thumb.png` and `bodies/<body>/sizes/<W>x<H>/world.png`
  and `legend.png` for every resolution on disk. No tiles, raw data, SVG
  or noise maps: they are rebuilt from the seed, and the SVG is on
  request anyway (#327). PNGs are stored as they are, only the JSON is
  deflated.
- **Built in memory** and sent whole: for a personal library (decision
  1) it keeps the code simple and leaves no temporary file to clean up.
  The memory used is the size of the PNGs; streaming from a temporary
  file is the way out if a system with many large resolutions needs it.
- **`system.json`** is a serde type (`SystemExport`) with a `schema`
  field (1). Its fields are listed in the README, *Export of a system*.
  Bodies never generated, or whose files are gone, are listed with their
  seed and request and `generated: false`.
- **Generator version:** each generated body gives `made_with`, read
  from the index (decision 4): the crate version, `null` for a body found
  in the tree, and the `format_version` of its `world.json`.
- **Reproducing a system:** the export's `create` field, posted to
  `POST /api/systems`, draws the system again exactly: the same id, the
  same bodies and seeds, the same `system.json` (besides its stored
  request). For a *whole system* (`{"system": …}`) it is the stored
  request with its seed filled in. A system *around a planet*
  (`{"around": …}`) resolves what its request leaves open (the star, the
  orbit, the parameters of the world) with the random generator, not
  the seed (issue #402): its `create` also pins the focus body's resolved
  parameters, read from its `params` in `system.json`
  (`WorldRequest::pinned`, which the CLI's "Reproduce with" line uses
  too). Resolving from the seed instead was set aside: it would change
  which systems new requests draw. An import is not built: `create` is
  the whole of what a future one would need.
- **Not exported:** the pin and the displayed sizes (view preferences, see
  *Step 1 done*), and the job history.
