# 15. Storage

Study of how the app stores its systems and bodies today, and of the
alternatives (a database, an object store such as MinIO, or others), to
decide what to keep, what to change and in what order. Step 1 of the
recommendation (the SQLite index) is implemented, see *Step 1 done*
below; the rest is not.

## Today

Everything lives on the local disk, in two directories bind-mounted into
the container (`compose.yaml`): `systems/` (`MAPGEN_SYSTEMS_DIR`) and the
older `worlds/` (`MAPGEN_WORLDS_DIR`). The layout is in
`docs/system-spec.md`, *Storage*:

```text
systems/<system-id>/
├── system.json        the system as drawn (written once)
├── names.json         names the user gave, by body id (rewritten on rename)
└── bodies/<body-id>/
    ├── world.png, world.svg, legend.png, thumb.png
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
| **Total**     | 3.9 GB | plus 113 MB of legacy `worlds/`                                |

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

The steps that follow are in the backlog, item 22.

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
  `systems.pinned_ms` (layout version 2; a version 1 file gets the column
  by `ALTER TABLE` when it is opened). Not mirrored to a file, by choice:
  a pin is a view preference, cheap to redo, and a mirror would add a
  second source of truth to reconcile. A lost database loses the pins,
  not the names. This is the first user data that only the database holds.
  Pins are lost too when a start finds a system's directory absent
  (`systems/` not mounted: the reconcile drops the system) and when a
  corrupt file is set aside (the `.corrupt-<ms>` file still holds them).
- **Layout versions:** an older file is migrated when opened, inside a
  `BEGIN IMMEDIATE` transaction that reads `user_version` again, so two
  processes opening a version 1 file migrate it once. A newer file is
  refused. Downgrading: an older server stops with `database layout 2,
  this server knows 1`; move the file (and `-wal`, `-shm`) aside, the
  index is rebuilt, names come back from `names.json`, pins are lost.
- **System name:** `PUT /api/systems/{id}/name` merges the name of the
  star into the names under the connection lock (`names.json` first, then
  the index), so a body named since a client read the list is kept.
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
  database is simply the case where everything is missing. Legacy
  `worlds/` is not read by the server any more, so it is not scanned.
- **Reads:** `GET /api/systems` and the name lookups read the index.
  `GET /api/systems/{id}` and job creation still read `system.json`
  (the full system is what they answer). The API only gained the
  `pinned` flag of `GET /api/systems`.
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

## Sources

- MinIO community edition in maintenance mode: [ayedo](https://ayedo.de/en/posts/minio-im-maintenance-mode/),
  [DEV Community](https://dev.to/alexandrev/minio-in-maintenance-mode-what-it-means-for-the-community-edition-oem-users-and-open-source-3o4j).
- Self-hosted S3 alternatives compared (Garage, SeaweedFS, RustFS, Ceph):
  [glukhov.org](https://glukhov.org/data-infrastructure/object-storage/self-hosted-s3-alternatives/),
  [Pinggy](https://pinggy.io/blog/minio_archived_self_hosted_s3_alternatives/).
- The project's own layout and behaviour: `docs/system-spec.md`
  (*Storage*), `crates/server/src/staging.rs`, `crates/server/src/systems.rs`,
  `compose.yaml`.
