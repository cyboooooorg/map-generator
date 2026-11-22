# CLI world export, pull request A: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps
> use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire the store, a world-side engine version and the world
export pipeline into the CLI crate, tested on fixture timelines, so that
pull request B only adds the `world` subcommand and plugs in
`system::generate`.

**Architecture:** `world::ENGINE_VERSION` travels in `World`, in
`catalogue::WorldKey` and in the world file name; eviction compares both
versions. `crates/cli/src/world.rs` runs the pipeline with the timeline
producer injected. `adapters::export` writes the world file and the PNG.

**Tech Stack:** Rust workspace, `clap` derive, `png`, `tempfile` (dev),
nextest tiers, `mise run ci`.

**Spec:** `docs/superpowers/specs/2026-11-22-cli-world-design.md`
(parents: `2026-11-21-world-catalogue-design.md`,
`2026-11-21-world-relief-design.md`). Branch `feat/cli-world`, worktree
`~/Developer/mg-cli-world`; the design is its first commit.

## Global Constraints

- Crate dependencies stay as `tools/guardrails` checks them: `world` on
  `timeline` and `science` only; `catalogue` on `system` and `timeline`.
- No file, clock or random source in `world` or `catalogue` (clippy).
- Caches are never migrated: an old file name or magic is a miss.
- Default cache budget `catalogue::DEFAULT_BUDGET` (2 GiB); sizes take
  IEC suffixes `KiB`, `MiB`, `GiB`, `TiB` only.
- Default root `./map-generator-store`.
- No user-facing option or command that does nothing yet: `GlobalArgs`
  is defined and tested here, flattened into `Cli` in pull request B
  (refines the design's "flattened into `Cli`", which assumed the world
  command in the same pull request).
- Every number in `science` is sourced, calibrated or a definition; ramp
  stops are adapter colours, not science.
- Commits: Conventional Commits, footer `Refs: #602`, plus the session's
  attribution lines.

## Review Focus

1. A world file left by `main` (`<k>-<g>.world`, magic `MGWORLD1`) in an
   existing store: a miss, then deleted by `evict`'s sweep, never an
   error (Task 2, test `old_world_names_are_evicted`).
2. `--body` naming a body absent from the system: a typed error naming
   the designation, not a panic (Task 6). The star case (`NotAPlanet`)
   needs a star timeline, which only the engine makes: pull request B
   tests it on the tier-2 reference systems.
3. A planet keyframe without a surface (a giant): `relief.no-surface`
   through `CliError`, nothing written to `--out` (Task 6).
4. `--cache-budget 0`: the export still completes from the in-memory
   world, the caches end empty (Task 6).
5. `--cache-budget` with `MB`, a space (`2 GiB`), a fraction or an
   overflow: refused with a message (Task 3).

---

### Task 1: World engine version in `World` and the codec

**Files:**

- Modify: `crates/world/src/lib.rs`, `crates/world/src/fields.rs`
  (`World`), `crates/world/src/fields_tests.rs`,
  `crates/world/src/project_tests.rs`
- Modify: `crates/adapters/src/world_codec.rs`,
  `crates/adapters/tests/world_codec.rs`

**Interfaces:**

- Produces: `world::ENGINE_VERSION: EngineVersion` (= `EngineVersion(1)`,
  doc comment starting `Definition:`: the version of what the map
  generator produces, bumped by any change to it);
  `World::new(grid: Grid, body: Designation, keyframe: u32, engine:
  EngineVersion, world_engine: EngineVersion) -> World`;
  `World::world_engine(&self) -> EngineVersion`. `engine` stays the
  system engine of the timeline the world was built from.
- Codec: magic `MGWORLD2`, header `u32 engine, u32 world_engine`, then
  as before. `MGWORLD1` decodes to `CodecError::Magic` (a miss).

- [ ] **Step 1:** In `crates/adapters/tests/world_codec.rs`, extend the
  round-trip test to build `World::new(.., EngineVersion(3),
  EngineVersion(7))` and assert `decoded.world_engine() ==
  EngineVersion(7)`; add `mgworld1_is_refused`: bytes of an encoded
  world with the first 8 bytes replaced by `b"MGWORLD1"` →
  `decode(..).is_err()`.
- [ ] **Step 2:** `cargo nextest run -p adapters --test world_codec` →
  fails to compile (`World::new` takes 4 arguments, 5 supplied).
- [ ] **Step 3:** Add the field, accessor, constant (re-exported from
  `world`'s root) and codec change; update the other `World::new` call
  sites (`fields_tests.rs`, `project_tests.rs`, codec decode).
- [ ] **Step 4:** `cargo nextest run -p world -p adapters` → passes.
- [ ] **Step 5:** Commit `feat(world): stamp worlds with a world engine version`.

### Task 2: World version in the cache key and eviction

**Files:**

- Modify: `crates/catalogue/src/ports.rs` (`WorldKey`),
  `crates/catalogue/src/evict.rs`, `crates/catalogue/src/evict_tests.rs`,
  `crates/catalogue/src/lib.rs`
- Modify: `crates/adapters/src/store/paths.rs`,
  `crates/adapters/src/store/mod.rs` (`entries`, `evict`,
  `world_matches`, module doc layout), `crates/adapters/tests/store.rs`,
  `crates/adapters/tests/world_codec.rs`

**Interfaces:**

- Consumes: `World::world_engine()` (Task 1).
- Produces: `WorldKey { system, body, keyframe, grid, engine, world:
  EngineVersion }`; `catalogue::Engines { pub system: EngineVersion,
  pub world: EngineVersion }` (`Clone, Copy, Debug, PartialEq, Eq`);
  `CacheKey::is_current(&self, current: Engines) -> bool` (timelines:
  `system` equal; worlds: both equal); `catalogue::evict(entries:
  &[CacheEntry], current: Engines, budget: u64) -> Vec<CacheKey>`;
  `FsStore::evict(&self, current: Engines) -> Result<Vec<CacheKey>,
  StoreError>`.
- File name `<keyframe>-<grid>-w<world>.world`, use key
  `<engine>/<id>/worlds/<slug>/<keyframe>-<grid>-w<world>.world`;
  `parse_world_name(&str) -> Option<(u32, u32, u32)>` accepts only that
  form.

- [ ] **Step 1:** Tests in `evict_tests.rs`: `stale_world_version_goes_first`
  (a world entry with `world: EngineVersion(0)` among current ones is
  returned first under an unlimited budget, a timelines entry of the
  same system engine is kept); the existing tests pass `Engines`.
  In `crates/adapters/tests/store.rs`: `old_world_names_are_evicted`
  (write a stored system, then a file
  `cache/<engine>/<id>/worlds/<slug>/0-8.world` by hand; `get` of any
  key is `Ok(None)`; `evict` returns without error and the file is
  gone, since unparseable names under a system's worlds directory are
  removed with the directory sweep), `world_key_roundtrip_with_version`
  (put then get with `world: EngineVersion(2)`; get with
  `EngineVersion(1)` is `None`). Update the existing `WorldKey`
  literals of both test files with `world: EngineVersion(1)`.
- [ ] **Step 2:** `cargo nextest run -p catalogue -p adapters` → fails
  to compile.
- [ ] **Step 3:** Implement. `world_matches` also compares
  `world.world_engine() == key.world`. `entries` stays read-only; a new
  sweep in `FsStore::evict`, next to `remove_temporaries`, deletes every
  file under a `worlds/<slug>/` directory whose name does not parse (a
  cache entry of an older layout), so Review Focus 1 holds.
- [ ] **Step 4:** `cargo nextest run -p catalogue -p adapters` → passes.
- [ ] **Step 5:** Commit `feat(catalogue)!: key worlds by the world engine version`
  with footer `BREAKING CHANGE: world cache files are renamed; old ones
  are evicted.`

### Task 3: `parse_size` and `GlobalArgs`

**Files:**

- Modify: `crates/cli/src/args.rs`, `crates/cli/Cargo.toml` (nothing new
  expected)

**Interfaces:**

- Produces: `pub fn parse_size(text: &str) -> Result<u64, String>`;
  `#[derive(clap::Args, Debug)] pub struct GlobalArgs { #[arg(long,
  global = true, default_value = "map-generator-store")] pub root:
  PathBuf, #[arg(long, global = true, value_parser = parse_size,
  default_value_t = catalogue::DEFAULT_BUDGET)] pub cache_budget: u64 }`;
  `GlobalArgs::store(&self) -> adapters::FsStore`.
- Not flattened into `Cli` in this pull request; add
  `#[cfg_attr(not(test), expect(dead_code, reason = "flattened into Cli
  by the world command, pull request B"))]` where clippy needs it.

- [ ] **Step 1:** Tests in `args.rs`: `parse_size("1024") == Ok(1024)`,
  `"1KiB" == 1024`, `"3MiB" == 3 << 20`, `"2GiB" == DEFAULT_BUDGET`,
  `"1TiB" == 1 << 40`, `"0" == 0`; errors for `"1MB"` (message contains
  `MiB`), `"2 GiB"`, `"1.5GiB"`, `"GiB"`, `""`, `"-1"`,
  `"16777216TiB"` (overflow). A test-only `#[derive(Parser)] struct
  Probe { #[command(flatten)] g: GlobalArgs }`: no flags gives root
  `map-generator-store` and `DEFAULT_BUDGET`; `--cache-budget 512MiB`
  gives `512 << 20`.
- [ ] **Step 2:** `cargo nextest run -p map-generator args` → fails.
- [ ] **Step 3:** Implement (`checked_mul` for the overflow).
- [ ] **Step 4:** Run again → passes.
- [ ] **Step 5:** Commit `feat(cli): parse the store root and the cache budget`.

### Task 4: Export and relief ramp (`adapters`)

**Files:**

- Create: `crates/adapters/src/export.rs`,
  `crates/adapters/tests/export.rs`
- Modify: `crates/adapters/src/png.rs` (`Ramp::relief`),
  `crates/adapters/src/lib.rs`

**Interfaces:**

- Produces: `pub struct Exported { pub world: PathBuf, pub png: PathBuf }`;
  `pub fn export(out: &Path, world: &World, field: &FieldId, width: u32,
  ramp: &Ramp) -> Result<Exported, ExportError>`; `pub enum ExportError
  { Io(io::Error), World(WorldError) }` with `Display` codes
  `export.io(..)`, and the world error's own code. Files:
  `<out>/<slug>-<keyframe>.world` (codec bytes, written through
  `store::write_atomic`) and `<out>/<slug>-<keyframe>-<field code>.png`,
  slug as `store::paths::body_slug` (make `body_slug` `pub(crate)` and
  declare `pub(crate) mod paths;` in `store/mod.rs`, private today).
- `Ramp::relief() -> Ramp`, heights in metres: `-8000` `[8, 29, 88]`,
  `-200` `[44, 127, 184]`, `-0.5` `[161, 218, 236]`, `0` `[57, 135,
  64]`, `1000` `[201, 188, 109]`, `3000` `[140, 90, 50]`, `6000`
  `[250, 250, 250]`.

- [ ] **Step 1:** Tests in `tests/export.rs` on a `Grid::new(4)` world
  with a `relief.height` field (half the cells at -1000, half at 1000):
  both files exist under a `tempfile::TempDir`, the world file decodes
  to an equal world, the PNG decodes (`png` crate) to `width × width/2`;
  a missing field gives `ExportError::World(_)` and creates no file;
  `Ramp::relief().colour(-1.0)` is blue-dominant and `colour(1.0)`
  green-dominant.
- [ ] **Step 2:** `cargo nextest run -p adapters --test export` → fails.
- [ ] **Step 3:** Implement: project first, then write both files.
- [ ] **Step 4:** Run again → passes.
- [ ] **Step 5:** Commit `feat(adapters): export a world file and its PNG projection`.

### Task 5: Two export widths agree (#381)

**Files:**

- Modify: `crates/world/src/project_tests.rs`

**Interfaces:**

- Consumes: `project`, `stats::area_mean`, `Grid::point`.

- [ ] **Step 1:** Test `two_widths_agree_on_area_statistics`: on
  `Grid::new(24)`, a field `h(p) = 3000·sin(3·lon)·cos(lat) + 1500·sin(lat) - 400`
  (from each cell's `point`). For widths 96 and 384, the raster's
  area-weighted mean (row weight `sin φ_top − sin φ_bottom`) and its
  area share above 0 each match the grid's (`area_mean`; the area share
  of cells above 0) within 1 % of the field's range (mean) and 0.01
  (share), and the two widths match each other within the same bounds.
  The bounds are the projection's averaging error for this smooth field,
  stated in the test's doc comment.
- [ ] **Step 2:** `cargo nextest run -p world two_widths` → passes or
  fails; if it fails, report the measured gaps rather than widening the
  bounds.
- [ ] **Step 3:** Commit `test(world): check two export widths agree on area statistics`
  with footer `Refs: #381`.

### Task 6: The world pipeline (`crates/cli/src/world.rs`)

**Files:**

- Create: `crates/cli/src/world.rs`
- Modify: `crates/cli/src/main.rs` (`mod world;`), `crates/cli/src/error.rs`,
  `crates/cli/Cargo.toml` (dev-dependencies `tempfile`, `timeline` with
  `features = ["fixtures"]`)

**Interfaces:**

- Consumes: Tasks 1-4; `catalogue::{CacheKey, StoredSystem, SystemId,
  WorldKey, Engines, SystemRepository, TimelineCache, WorldCache}`;
  `adapters::{FsStore, StoreError, Exported, ExportError, Ramp,
  export}`;
  `world::relief::{generate, HEIGHT}`; `system::ENGINE_VERSION`.
- Produces:

  ```rust
  pub struct WorldRequest {
      pub body: Designation,
      pub cell: Metre,          // default world::DEFAULT_CELL
      pub field: FieldId,       // default relief.height
      pub width: u32,           // default 2048, even, > 0
      pub out: PathBuf,
  }
  pub struct WorldRun {
      pub system: SystemId,
      pub key: WorldKey,
      pub timelines_cached: bool,
      pub world_cached: bool,
      pub evicted: Vec<CacheKey>,
      pub exported: Exported,
  }
  pub fn run<E>(
      store: &FsStore,
      inputs: &Inputs,
      request: &WorldRequest,
      timelines: impl FnOnce(&Inputs) -> Result<Vec<Timeline>, E>,
  ) -> Result<WorldRun, CliError>
  where
      CliError: From<E>;
  ```

- New `CliError` variants (codes first): `Store(StoreError)` →
  `cli.store: ..`; `Export(ExportError)`; `Relief(ReliefError)`;
  `NoSuchBody(Designation)` → `cli.world.no-such-body: <designation>`;
  `NotAPlanet(Designation)` → `cli.world.not-a-planet: ..`;
  `NoPresentKeyframe(Designation)` → `cli.world.no-present-keyframe: ..`.
- Order: put `StoredSystem::new(inputs.clone(), system::ENGINE_VERSION)`
  only when `get(id)` is `None` (keeps the first engine stamp); timelines
  from the cache else the producer then `put`; the body's timeline; the
  keyframe whose date equals `present` (index as `u32`); its
  `BodyState::Planet` state; `Grid::for_radius(Metre::from(radius),
  cell)`; key with `engine: system::ENGINE_VERSION`, `world:
  world::ENGINE_VERSION`; world from the cache, else `relief::generate`
  with the timeline's `seed` into a new `World`, then `put`;
  `store.evict(Engines { .. })`; `export(.., &Ramp::relief())` for
  `relief.height` (grey ramp over the field's min-max otherwise).
  Nothing is written to `out` before the world exists.

- [ ] **Step 1:** Tests in `world.rs` with a fixture producer (one
  planet timeline from `timeline::fixtures::earth_like_fixture` at
  4.57 Gyr: `Timeline::new(designation, Seed::new(1),
  system::ENGINE_VERSION, Gyr(4.57), vec![Keyframe::new(Gyr(4.57),
  BodyState::Planet(state), &trace)?])`, designation
  `Designation::planet(SystemNumber::new(10_001)?, 1)`; no star
  timeline), `cell` 2000 km (`n = 5`, 150 cells), width 64, in a
  `TempDir`:
  `first_run_generates_then_second_hits_caches` (second producer panics
  if called; `timelines_cached` and `world_cached` true; same exported
  bytes); `unknown_body_is_refused` (rank 2 → `NoSuchBody`, `out`
  empty); `surfaceless_planet_is_refused` (fixture with `surface =
  None` → `Relief(NoSurface)`, `out` empty); `zero_budget_still_exports`
  (budget 0: files in `out`, `evicted` holds the timelines and the
  world, the cache directory has no `.world` or `timelines.json` left);
  `producer_error_is_passed_through` (producer returns
  `ContractError::NoKeyframe` → `CliError::Contract`).
- [ ] **Step 2:** `cargo nextest run -p map-generator world` → fails.
- [ ] **Step 3:** Implement; mark `run` and its types
  `#[cfg_attr(not(test), expect(dead_code, reason = "called by the
  world command, pull request B"))]`.
- [ ] **Step 4:** Run again → passes; `cargo clippy -p map-generator
  --all-targets -- -D warnings` clean.
- [ ] **Step 5:** Commit `feat(cli): run the world pipeline over the store`.

### Task 7: Relief previews (tier 3)

**Files:**

- Modify: `crates/timeline/src/fixtures.rs` (add the hand-built bodies),
  `crates/world/tests/relief_heavy.rs` (use them)
- Create: `crates/adapters/tests/relief_previews.rs`;
  `crates/adapters/Cargo.toml` dev-dependency `timeline` with
  `features = ["fixtures"]`

**Interfaces:**

- Produces in `timeline::fixtures`: `earth_relief_fixture(trace)`,
  `mars_relief_fixture(trace, ocean_share: Option<f64>)`,
  `venus_relief_fixture(trace)` returning `PlanetState`, moved verbatim
  from `relief_heavy.rs` (`body`, `earth`, `mars`, `venus` and their
  sourced constants `MARS_RADIUS`, `VENUS_RADIUS`, `VENUS_GRAVITY`
  with their `Source:` comments). Fixtures only: announce to mapgen1
  and mapgen3 as a test-only addition to the contract crate.

- [ ] **Step 1:** Move the states; `cargo nextest run -p world
  --no-run` → compiles (the heavy censuses are unchanged).
- [ ] **Step 2:** `relief_previews.rs`, `#[ignore = "heavy: three 60 km
  worlds written as PNG previews"]`: for Earth, Mars (dry) and Venus at
  seed 1, `Grid::for_radius(radius, 60 km)`, `relief::generate`,
  `export` at width 2048 to
  `concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/relief-previews/<name>")`;
  assert each PNG exists.
- [ ] **Step 3:** `cargo nextest run -p adapters --run-ignored only
  relief_previews` → passes; downscale one PNG with
  `sips -Z 800` into the scratchpad for the user, never read the full
  one.
- [ ] **Step 4:** Commit
  `test(adapters): write relief previews of Earth, Mars and Venus`.

### Task 8: Documentation and CI

**Files:**

- Modify: `crates/cli/README.md` (the pipeline, `--root`,
  `--cache-budget` and sizes, marked "with the world command"),
  `crates/world/README.md` (world engine version and when to bump it),
  `crates/catalogue/README.md` (`WorldKey.world`, `Engines`, eviction
  rule), `crates/adapters/README.md` (layout with `-w<world>`,
  `MGWORLD2`, `export`, relief ramp),
  `docs/superpowers/specs/2026-11-21-world-catalogue-design.md`
  (Catalogue section: the world file name and the two versions).

- [ ] **Step 1:** Update the files above.
- [ ] **Step 2:** `mise run ci` → passes (format, clippy, tiers 1-2
  within budget, markdownlint).
- [ ] **Step 3:** Commit `docs: document the world engine version and the export`.

Then `branch-reviewer` and `test-reviewer` on the whole branch, fixes,
`mise run ci`, and `pr-shepherd` with `Refs #602` and `Refs #381` (the #381
criterion is carried by #602).
