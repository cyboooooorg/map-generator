# World and catalogue W1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Lay the infrastructure of the map generator and the catalogue:
the cube-sphere grid, the world field container, the 2:1 projection, the
catalogue's identity, ports and eviction policy, and their filesystem,
binary and PNG adapters.

**Architecture:** Pure domain code in `world` (grid, fields, projection)
and `catalogue` (identity, ports, keys, eviction); everything touching
files or encodings in `adapters` (fs store, world codec, PNG). No CLI
command, no physics.

**Tech Stack:** Rust 2024 workspace, serde/serde_json, `png` crate
(adapters only), cargo-nextest tiers, `mise run ci`.

**Spec:** [world and catalogue
design](../specs/2026-11-21-world-catalogue-design.md),
section "W1 in detail". Parent: [architecture
rewrite](../specs/2026-11-20-architecture-rewrite-design.md).

## Global Constraints

- Worktree `~/Developer/mg-world-catalogue`, branch
  `feat/world-grid-catalogue` from `main`. Never touch `develop`.
- `world` depends on `timeline` and `science` only; `catalogue` on
  `system` and `timeline` only (`tools/guardrails`). Do not edit
  `Cargo.toml` dependency edges beyond what a task lists.
- Domain crates: no file, clock, process or env access (`clippy.toml`
  denies them). The wall clock (`SystemTime::now`) is denied in adapters
  too.
- Ordered maps only (`BTreeMap`), never `HashMap` with default hasher.
- Every test in this plan is tier 1 (outside `crates/regression`); none
  generates a system or a world through the engines.
- Every commit follows Conventional Commits, ends with the attribution
  lines (`Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`,
  `Claude-Session: https://claude.ai/code/session_01NRxWVdciTecCe8JKgycSVq`).
- Run `cargo nextest run -p <crate>` after each task, `mise run ci`
  before the last commit. Never `npm ci` or `mise run web:check` here.
- Docs in English; markdownlint default rules.

## Review Focus

- A cell on a face edge or at a three-face corner: its neighbours are
  eight distinct valid cells, and neighbourhood is mutual (Task 1).
- The 8 M cap on a large radius: `for_radius` never returns more than
  `MAX_CELLS` cells, and a tiny body still gets at least one cell per
  face side (Task 1).
- A projection row at the poles: a pixel there is a long thin strip and
  must still average every cell it covers (Task 3).
- A truncated, corrupt or other-version world or timeline file on disk:
  a cache miss, never a panic or an error (Tasks 6 and 7).
- A cache over budget with stale-engine entries and a tie on last use:
  stale go first, ties break by key order, so eviction is deterministic
  (Task 5).

---

### Task 1: Cube-sphere grid

**Files:**

- Create: `crates/world/src/grid.rs`, `crates/world/src/grid_tests.rs`
- Modify: `crates/world/src/lib.rs`

**Interfaces:**

- Produces (`world::grid`, re-exported at `world::`):
  - `pub const DEFAULT_CELL: Metre` = `Metre(15_000.0)`, doc
    `Definition: the default cell size of the simulation grid on solid
    worlds (architecture rewrite design, Simulation grid).`
  - `pub const MAX_CELLS: usize` = `8_000_000`, doc `Definition: the
    cap on the cell count (architecture rewrite design, Simulation grid).`
  - `#[derive(Clone, Debug, PartialEq)] pub struct Grid` (private
    fields: `n: u32`, `face_areas: Vec<f64>` of `n²` solid angles, the
    same on every face).
  - `Grid::new(n: u32) -> Grid` (panics on `n == 0`).
  - `Grid::for_radius(radius: Metre, cell: Metre) -> Grid`.
  - `n(&self) -> u32`, `cell_count(&self) -> usize`,
    `point(&self, k: usize) -> [f64; 3]` (unit vector),
    `index_of(&self, p: [f64; 3]) -> usize`,
    `neighbours(&self, k: usize) -> impl Iterator<Item = usize> + '_`
    (the eight around `k`), `area(&self, k: usize) -> f64` (steradians).

- [ ] **Step 1: Port the legacy grid and its tests.** Copy
  `git show legacy-final:crates/mapgen/src/cube.rs` into `grid.rs`
  (rename `CubeGrid` to `Grid`, make `size` the private `n`), and its two
  tests (`cells_cover_the_sphere_evenly`,
  `neighbours_across_an_edge_are_mutual`) into `grid_tests.rs`, wired
  with `#[cfg(test)] #[path = "grid_tests.rs"] mod tests;` as
  `crates/timeline/src/sources.rs` does. Keep the legacy doc comments.

- [ ] **Step 2: Write the failing tests for the new parts.**

  ```rust
  #[test]
  fn earth_and_mars_at_15_km_match_the_design() {
      // n = round(π/2 × R / cell): R in metres from the IAU nominal
      // equatorial Earth radius; Mars mean radius 3389.5 km (Seidelmann
      // et al. 2007, IAU WGCCRE).
      let earth = Grid::for_radius(Metre::from(EarthRadius(1.0)), DEFAULT_CELL);
      assert_eq!(earth.n(), 668);
      assert_eq!(earth.cell_count(), 2_677_344);
      let mars = Grid::for_radius(Metre(3_389_500.0), DEFAULT_CELL);
      assert_eq!(mars.n(), 355);
  }

  #[test]
  fn the_cap_holds_the_cell_count_under_eight_million() {
      let big = Grid::for_radius(Metre::from(EarthRadius(3.0)), DEFAULT_CELL);
      assert!(big.cell_count() <= MAX_CELLS);
      assert_eq!(big.n(), 1154); // floor(sqrt(8e6 / 6))
  }

  #[test]
  fn a_tiny_body_keeps_one_cell_per_side() {
      assert_eq!(Grid::for_radius(Metre(1_000.0), DEFAULT_CELL).n(), 1);
  }

  #[test]
  fn areas_sum_to_the_whole_sphere() {
      for n in [1, 2, 7, 64] {
          let g = Grid::new(n);
          let total: f64 = (0..g.cell_count()).map(|k| g.area(k)).sum();
          assert!((total - 4.0 * std::f64::consts::PI).abs() < 1e-9, "n={n}");
      }
  }

  #[test]
  fn corner_cells_have_eight_distinct_neighbours() {
      let g = Grid::new(8);
      for face in 0..6u32 {
          for (x, y) in [(0, 0), (7, 0), (0, 7), (7, 7)] {
              let k = ((face * 8 + y) * 8 + x) as usize;
              let mut ns: Vec<usize> = g.neighbours(k).collect();
              ns.sort_unstable();
              ns.dedup();
              assert!(ns.len() >= 7 && !ns.contains(&k), "face {face} ({x},{y}): {ns:?}");
          }
      }
  }

  #[test]
  fn area_ratio_stays_within_the_equiangular_spread() {
      // Equi-angular cube map: cell areas vary by about 30 % (design).
      let g = Grid::new(64);
      let (min, max) = (0..g.cell_count()).map(|k| g.area(k))
          .fold((f64::MAX, 0.0_f64), |(a, b), v| (a.min(v), b.max(v)));
      assert!(max / min < 1.45, "{}", max / min);
  }
  ```

  (A three-face corner has only seven distinct neighbours: one of the
  eight diagonal offsets falls on a cell already counted. The legacy
  `neighbours` yields eight items; the test accepts seven distinct.)

- [ ] **Step 3: Run them, expect failures** (`for_radius`, `area`,
  constants missing): `cargo nextest run -p world`.

- [ ] **Step 4: Implement `for_radius` and `area`.**
  `n = max(1, round(π/2 × R / cell))`, then
  `n = min(n, floor(sqrt(MAX_CELLS / 6)))`. Cell areas: with gnomonic
  coordinates `s = tan(α)` of the cell edges (`α` from `-π/4` to `π/4`
  in `n` equal steps), the solid angle of `[0, s] × [0, t]` on the unit
  face plane is `Ω(s, t) = atan(s·t / sqrt(1 + s² + t²))`, and a cell
  `[s0, s1] × [t0, t1]` is
  `Ω(s1,t1) − Ω(s0,t1) − Ω(s1,t0) + Ω(s0,t0)`. Compute the `n²` values
  of one face in `Grid::new` (`area(k)` indexes them by `k mod n²`).

- [ ] **Step 5: Run `cargo nextest run -p world`, expect PASS; then
  `cargo clippy -p world --all-targets -- -D warnings`.**

- [ ] **Step 6: Commit** `feat(world): port the cube-sphere grid with cell
      areas`.

### Task 2: Fields and the world container

**Files:**

- Create: `crates/world/src/fields.rs`, `crates/world/src/fields_tests.rs`,
  `crates/world/src/error.rs`
- Modify: `crates/world/src/lib.rs`, `crates/world/Cargo.toml` (add
  `serde.workspace = true` only if a type below derives it; none needs
  to)

**Interfaces:**

- Consumes: `Grid` (Task 1); `timeline::{Code, Designation, EngineVersion}`.
- Produces (re-exported at `world::`):
  - `#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)] pub struct
    FieldId(pub Code);`
  - `#[derive(Clone, Debug, PartialEq)] pub enum Values { F32(Vec<f32>),
    U16(Vec<u16>), U8(Vec<u8>) }`
    with `len(&self) -> usize` and `get_f32(&self, k: usize) -> f32`.
  - `#[derive(Clone, Copy, Debug, PartialEq)] pub enum Storage { Exact, U16 {
    min: f32, max: f32 } }`
    — how an `F32` field is written to disk; `U16` maps `[min, max]`
    linearly onto `0..=65535`.
  - `#[derive(Clone, Debug, PartialEq)] pub struct Field { pub values: Values,
    pub categorical: bool, pub storage: Storage }`
  - `#[derive(Clone, Debug, PartialEq)] pub struct World` with private
    fields `grid: Grid, body: Designation, keyframe: u32, engine: EngineVersion,
    fields: BTreeMap<FieldId, Field>`;
    `World::new(grid, body, keyframe, engine) -> World`;
    accessors `grid()`, `body()`, `keyframe()`, `engine()`;
    `insert(&mut self, id: FieldId, field: Field) -> Result<(), WorldError>`;
    `field(&self, id: &FieldId) -> Option<&Field>`;
    `fields(&self) -> impl Iterator<Item = (&FieldId, &Field)>`.
  - `#[derive(Clone, Debug, PartialEq)] pub enum WorldError { WrongLength {
    field: Code, expected: usize, found: usize }, Duplicate(Code),
    Categorical(Code), Missing(Code), Storage(Code) }`
    implementing `Display` (codes and numbers only) and `Error`.

- [ ] **Step 1: Write the failing tests** in `fields_tests.rs`:
  `a_field_of_the_wrong_length_is_refused` (`Grid::new(2)` has 24
  cells; inserting 23 values gives `WrongLength { expected: 24, found: 23, ..
  }`),
  `a_field_is_inserted_once` (second insert of the same id gives
  `Duplicate`), `a_categorical_field_is_integer` (`categorical: true`
  with `Values::F32` gives `Categorical`),
  `u16_storage_needs_a_range` (`Storage::U16 { min: 5.0, max: 5.0 }`
  or on a non-`F32` field gives `Storage`), and
  `fields_iterate_in_code_order` (insert `relief.height` then
  `climate.temperature`, iterate, get `climate.temperature` first).
- [ ] **Step 2: Run, expect compile failure:** `cargo nextest run -p world`.
- [ ] **Step 3: Implement** the types above; `insert` checks length,
  duplicate, categorical-is-`U8`-or-`U16`, and storage validity.
- [ ] **Step 4: Run, expect PASS.**
- [ ] **Step 5: Commit** `feat(world): add the world field container`.

### Task 3: Equirectangular projection

**Files:**

- Create: `crates/world/src/project.rs`, `crates/world/src/project_tests.rs`
- Modify: `crates/world/src/lib.rs`

**Interfaces:**

- Consumes: `World`, `FieldId`, `Field`, `Values`, `WorldError` (Task 2), `Grid`
  (Task 1).
- Produces (re-exported at `world::`):
  - `#[derive(Clone, Debug, PartialEq)] pub struct Raster { pub width: u32, pub
    height: u32, pub values: Vec<f32> }`
    (row-major from the north-west corner, `height = width / 2`).
  - `pub fn project(world: &World, id: &FieldId, width: u32) -> Result<Raster,
    WorldError>`
    (`Missing` for an unknown id; panics on an odd or zero `width`).

- [ ] **Step 1: Write the failing tests:**
  - `a_constant_field_stays_constant_at_every_width`: `Grid::new(16)`,
    a field of 3.5 everywhere, widths 8, 64 and 512, every pixel equals
    3.5 within `1e-6`.
  - `a_latitude_field_projects_to_its_latitude`: field = `asin(z)` of
    each cell centre (degrees) on `Grid::new(64)`; at width 64, each
    pixel's value is within one cell's angular size (90/64 degrees) of
    its row's centre latitude.
  - `areas_agree_between_grid_and_projection`: field = 1.0 where
    `z > 0.5`, else 0.0 on `Grid::new(64)`; the grid's area-weighted
    mean (`Σ area·v / 4π`, exact value 0.25 for the cap above `z = 0.5`)
    and the projection's `cos φ`-weighted pixel mean at widths 128 and
    1024 agree within 0.01.
  - `polar_rows_average_every_cell_they_cover`: on `Grid::new(32)`,
    field = cell index modulo 2 as `F32`; at width 16 the top row's
    values are strictly between 0 and 1 (several cells averaged), not
    a single cell's 0 or 1.
  - `categorical_fields_take_the_mode`: `U8` field, class 3 on cells
    with `z > 0` and class 7 elsewhere, width 64; every pixel is
    exactly 3.0 or 7.0 and the equator-adjacent rows are the right
    class.
- [ ] **Step 2: Run, expect compile failure.**
- [ ] **Step 3: Implement `project`.** Pixel `(i, j)` covers longitude
  `[-π + 2π·i/w, -π + 2π·(i+1)/w]` and latitude
  `[π/2 − π·(j+1)/h, π/2 − π·j/h]`. Sample it on a `kx × ky` lattice
  uniform in longitude and in `sin φ` (each sample covers the same
  area, so the mean needs no weights), with
  `cell = sqrt(4π / cell_count)` (the mean cell side on the unit
  sphere), `kx = ceil(2 · Δλ · cos φ_max / cell)` (`φ_max` the pixel's
  latitude nearest the equator) and `ky = ceil(2 · Δφ / cell)`, each at
  least 1. Each sample point `(cos φ cos λ, cos φ sin λ, sin φ)` goes
  through `grid.index_of`. Continuous fields average `get_f32`;
  categorical fields take the most frequent class (ties: the smallest
  class). Whether to cap `kx`, `ky` is left to the implementer; note
  any cap in the doc comment.
- [ ] **Step 4: Run, expect PASS** (`cargo nextest run -p world`).
- [ ] **Step 5: Commit** `feat(world): project a field onto a 2:1 raster`.

### Task 4: Catalogue identity, stored system and ports

**Files:**

- Modify: `crates/timeline/src/seed.rs`, `crates/timeline/src/lib.rs`
  (make `fnv1a64` public, re-export it)
- Create: `crates/catalogue/src/id.rs`, `crates/catalogue/src/stored.rs`,
  `crates/catalogue/src/ports.rs`, `crates/catalogue/src/catalogue_tests.rs`
- Modify: `crates/catalogue/src/lib.rs`, `crates/catalogue/Cargo.toml`
  (add `serde.workspace = true`, `serde_json.workspace = true`)

**Interfaces:**

- Consumes: `system::{Inputs, INPUTS_SCHEMA}`,
  `timeline::{fnv1a64, Designation, EngineVersion, Timeline}`.
- Produces:
  - `timeline::fnv1a64(bytes: &[u8]) -> u64` (unchanged body; doc
    says it is part of the contract and used by the catalogue).
  - `#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)] pub
    struct SystemId(u64);`
    `SystemId::of(inputs: &Inputs) -> SystemId` (FNV-1a 64 of
    `serde_json::to_string(inputs)`); `Display` as 16 lowercase hex
    digits; `FromStr` back (exactly 16 hex digits); serde as that
    string.
  - `#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)] pub struct
    StoredSystem { pub id: SystemId, pub inputs: Inputs, pub engine:
    EngineVersion, pub name: Option<String> }`
    with `StoredSystem::new(inputs: Inputs, engine: EngineVersion) ->
    StoredSystem`
    and `check_schema(&self) -> Result<(), CatalogueError>`.
  - `pub enum CatalogueError { UnknownSchema { found: u32, expected: u32 },
    BadId(String) }` (`Display`, `Error`).
  - `#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)] pub struct WorldKey
    { pub system: SystemId, pub body: Designation, pub keyframe: u32, pub grid:
    u32, pub engine: EngineVersion }`
    (`EngineVersion` needs `PartialOrd, Ord`: add them in `timeline`).
  - Traits:

    ```rust
    pub trait SystemRepository {
        type Error: std::error::Error;
        fn put(&self, system: &StoredSystem) -> Result<(), Self::Error>;
        fn get(&self, id: SystemId) -> Result<Option<StoredSystem>, Self::Error>;
        fn list(&self) -> Result<Vec<SystemId>, Self::Error>; // sorted
        fn remove(&self, id: SystemId) -> Result<bool, Self::Error>;
    }
    pub trait TimelineCache {
        type Error: std::error::Error;
        fn get(&self, system: SystemId, engine: EngineVersion) -> Result<Option<Vec<Timeline>>, Self::Error>;
        fn put(&self, system: SystemId, engine: EngineVersion, timelines: &[Timeline]) -> Result<(), Self::Error>;
    }
    pub trait WorldCache {
        type World;
        type Error: std::error::Error;
        fn get(&self, key: &WorldKey) -> Result<Option<Self::World>, Self::Error>;
        fn put(&self, key: &WorldKey, world: &Self::World) -> Result<(), Self::Error>;
    }
    ```

- [ ] **Step 1: Write the failing tests** (`catalogue_tests.rs`, inputs
  built as in `crates/system/src/inputs.rs` tests):
  `the_id_is_stable` (pin `SystemId::of` for the seed-7 earth-like
  `Inputs` of the system tests to the value of
  `fnv1a64(json)` computed in the test from the JSON string, and assert
  two calls agree), `any_input_change_changes_the_id` (seed, present,
  archetype each changed in turn: three different ids),
  `a_drawn_present_differs_from_a_fixed_one` (`present: None` vs
  `Some(Gyr(4.57))`), `the_id_round_trips_as_text` (16 chars, parse
  back equal; `"xyz"` and a 15-digit string give `BadId`),
  `an_unknown_schema_is_refused` (`schema: INPUTS_SCHEMA + 1` gives
  `UnknownSchema`), `a_stored_system_round_trips_through_json`.
- [ ] **Step 2: Run, expect compile failure:** `cargo nextest run -p catalogue`.
- [ ] **Step 3: Implement** the items above.
- [ ] **Step 4: Run `cargo nextest run -p catalogue -p timeline`, expect PASS.**
- [ ] **Step 5: Commit** in two commits:
  `feat(timeline): publish the FNV-1a hash for the catalogue` then
  `feat(catalogue): key stored systems by a hash of their inputs`.
  After the first, the main session tells mapgen1 and mapgen3 (additive
  contract change).

### Task 5: Eviction policy

**Files:**

- Create: `crates/catalogue/src/evict.rs`, `crates/catalogue/src/evict_tests.rs`
- Modify: `crates/catalogue/src/lib.rs`

**Interfaces:**

- Consumes: `SystemId`, `WorldKey` (Task 4), `EngineVersion`.
- Produces:
  - `pub const DEFAULT_BUDGET: u64` = 2 GiB (`2 * 1024 * 1024 * 1024`),
    doc `Definition: the default byte budget of the caches (world and
    catalogue design; the user sets it with --cache-budget).`
  - `#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)] pub enum CacheKey {
    Timelines { system: SystemId, engine: EngineVersion }, World(WorldKey) }`
    with `engine(&self) -> EngineVersion`.
  - `#[derive(Clone, Debug, PartialEq)] pub struct CacheEntry { pub key:
    CacheKey, pub bytes: u64, pub last_use: u64 }`
  - `pub fn evict(entries: &[CacheEntry], current: EngineVersion, budget: u64)
    -> Vec<CacheKey>`:
    every entry of another engine version, then the current ones by
    ascending `last_use` (ties by `key` order) until the remaining
    bytes are at most `budget`. Output in that order.

- [ ] **Step 1: Write the failing tests:** `stale_entries_always_go`
  (one stale entry, budget large: it is returned),
  `under_budget_nothing_current_goes`,
  `least_recently_used_go_first_until_under_budget` (three current
  entries of 100 bytes, uses 3, 1, 2, budget 150: returns the use-1
  then the use-2 entries), `ties_break_by_key_order`,
  `a_zero_budget_drops_everything`.
- [ ] **Step 2: Run, expect failure.**
- [ ] **Step 3: Implement `evict`.**
- [ ] **Step 4: Run, expect PASS.**
- [ ] **Step 5: Commit** `feat(catalogue): add the cache eviction policy`.

### Task 6: Filesystem store for systems and timelines

**Files:**

- Create: `crates/adapters/src/store/mod.rs`,
  `crates/adapters/src/store/paths.rs`,
  `crates/adapters/src/store/uses.rs`, `crates/adapters/tests/store.rs`
- Modify: `crates/adapters/src/lib.rs`, `crates/adapters/Cargo.toml`
  (`serde`, `serde_json`; dev-dependency `tempfile` at its latest
  release, added to `[workspace.dependencies]`)

**Interfaces:**

- Consumes: Task 4 traits and types, Task 5 `CacheKey`, `CacheEntry`, `evict`.
- Produces:
  - `pub struct FsStore` with `FsStore::new(root: PathBuf, budget: u64) ->
    FsStore`.
  - `#[derive(Debug)] pub enum StoreError { Io(std::io::Error),
    Json(serde_json::Error), Catalogue(catalogue::CatalogueError) }` (`Display`,
    `Error`, `From` each).
  - `impl SystemRepository for FsStore` and `impl TimelineCache for FsStore`
    (`Error = StoreError`).
  - `FsStore::evict(&self, current: EngineVersion) -> Result<Vec<CacheKey>,
    StoreError>`:
    lists the cache entries on disk with their sizes and ticks, calls
    `catalogue::evict` with the store's budget, removes the returned
    files (and directories left empty), drops them from `uses.json`.
  - Layout (spec): `<root>/systems/<id>/inputs.json`,
    `<root>/cache/<engine>/<id>/timelines.json`,
    `<root>/cache/<engine>/<id>/worlds/<slug>/<keyframe>-<grid>.world`,
    `<root>/cache/uses.json` (`{"tick": u64, "uses": {"<path relative to
    cache/>": u64}}`).
    `<slug>` is the designation's text with spaces as `-`, case kept so it
    parses back (`MGS 12345 b` → `MGS-12345-b`).

- [ ] **Step 1: Write the failing integration tests** in
  `crates/adapters/tests/store.rs` (each in its own `tempfile::TempDir`):
  `a_system_round_trips`, `list_is_sorted_and_remove_reports`,
  `an_unknown_schema_on_disk_is_an_error` (write an `inputs.json` with
  `schema` + 1 by hand),
  `timelines_round_trip_and_another_engine_misses` (put under engine 1,
  get under engine 2 is `None`; use `Vec::new()` as the timelines if no
  hand-built `Timeline` is simple to make),
  `a_corrupt_timeline_file_is_a_miss` (write `{` into
  `timelines.json`: `get` returns `Ok(None)`),
  `writes_leave_no_temporary_file` (after puts, no file ending in
  `.tmp` under the root),
  `eviction_drops_stale_then_least_used` (put timelines under engines
  1 and 2 for two systems, `get` one of them, set the budget to the size
  of one file: `evict(EngineVersion(2))` returns the engine-1 entry then
  the unread engine-2 entry, and those files are gone).
- [ ] **Step 2: Run, expect failure:** `cargo nextest run -p adapters`.
- [ ] **Step 3: Implement.** Writes go to `<file>.tmp` in the target
  directory, then `rename`. `get` on the inputs propagates errors and
  calls `check_schema`; `get` on a cache entry maps any read or parse
  failure to `Ok(None)`. Every `put` and every cache hit advances the
  tick and records it in `uses.json` (written with the same staging).
- [ ] **Step 4: Run, expect PASS.**
- [ ] **Step 5: Commit** `feat(adapters): store systems and timelines on the
      filesystem`.

### Task 7: World codec and world cache

**Files:**

- Create: `crates/adapters/src/world_codec.rs`,
  `crates/adapters/tests/world_codec.rs`
- Modify: `crates/adapters/src/store/mod.rs`, `crates/adapters/src/lib.rs`

**Interfaces:**

- Consumes: `world::{World, Grid, FieldId, Field, Values, Storage}`
  (Tasks 1-2), `WorldKey`, `WorldCache` (Task 4), `FsStore` (Task 6).
- Produces:
  - `pub fn encode(world: &World) -> Vec<u8>`
  - `pub fn decode(bytes: &[u8]) -> Result<World, CodecError>`;
    `#[derive(Debug)] pub enum CodecError { Magic, Truncated, Invalid(String)
    }`.
  - `impl WorldCache for FsStore` with `type World = world::World`.
  - Layout, little-endian: `b"MGWORLD1"`; `u32` engine; `u16` length +
    UTF-8 designation text; `u32` keyframe; `u32` grid `n`; `u32` field
    count; per field (in `FieldId` order): `u16` length + code text,
    `u8` type (0 `F32`, 1 `U16`, 2 `U8`), `u8` categorical (0/1), `u8`
    storage (0 exact, 1 `U16` then `f32` min, `f32` max); then the cell
    values: an `F32` field with `Storage::U16` is written as `u16`
    (`round((v − min)/(max − min) × 65535)`, clamped), every other field
    in its own type.

- [ ] **Step 1: Write the failing tests:** `a_world_round_trips` (a
  `Grid::new(4)` world with one field of each type, decode equals
  encode's input), `u16_storage_round_trips_within_one_step`
  (`Storage::U16 { min: 20.0, max: 3000.0 }`, values 20, 288.15, 3000:
  each within `(max − min)/65535`), `a_truncated_file_fails`
  (`Truncated` for every prefix shorter than the whole),
  `a_wrong_magic_fails`, `a_world_put_is_got_back` (through `FsStore`,
  key with grid 4), `a_corrupt_world_file_is_a_miss`.
- [ ] **Step 2: Run, expect failure.**
- [ ] **Step 3: Implement** codec and the `WorldCache` impl (path from
  the key; misses as in Task 6; ticks as in Task 6; `evict` now lists
  world files too).
- [ ] **Step 4: Run `cargo nextest run -p adapters`, expect PASS.**
- [ ] **Step 5: Commit** `feat(adapters): encode worlds and cache them on disk`.

### Task 8: PNG of a raster

**Files:**

- Create: `crates/adapters/src/png.rs`, `crates/adapters/tests/png.rs`
- Modify: `crates/adapters/src/lib.rs`, `crates/adapters/Cargo.toml`,
  root `Cargo.toml` (`png` at its latest release in
  `[workspace.dependencies]`)

**Interfaces:**

- Consumes: `world::Raster` (Task 3).
- Produces:
  - `#[derive(Clone, Debug)] pub struct Ramp { stops: Vec<(f32, [u8; 3])> }`
    with `Ramp::new(stops) -> Ramp` (panics unless sorted and non-empty)
    and `Ramp::grey(min: f32, max: f32) -> Ramp`;
    `colour(&self, v: f32) -> [u8; 3]`
    (linear between stops, clamped at the ends; NaN is black).
  - `pub fn encode_png(raster: &Raster, ramp: &Ramp) -> Vec<u8>` (8-bit RGB).
  - `pub fn write_png(path: &Path, raster: &Raster, ramp: &Ramp)`
    `-> std::io::Result<()>` (staged write).

- [ ] **Step 1: Write the failing tests:** `grey_ramp_maps_ends_and_middle`
  (0 → `[0,0,0]`, 1 → `[255,255,255]`, 0.5 → `[128,128,128]` with
  `Ramp::grey(0.0, 1.0)`; values outside clamp),
  `a_png_decodes_to_its_size_and_colours`
  (encode a 4×2 raster, decode with `png::Decoder`, check 4×2 RGB and
  the first pixel's colour).
- [ ] **Step 2: Run, expect failure.**
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run, expect PASS.**
- [ ] **Step 5: Commit** `feat(adapters): write a raster as a PNG`.

### Task 9: Documentation and CI

**Files:**

- Modify: `crates/world/README.md`, `crates/catalogue/README.md`,
  `crates/adapters/README.md` (create it if missing),
  `crates/timeline/README.md`
  (mention the published hash), the parent design's Catalogue section
  only if a W1 call contradicts it.

- [ ] **Step 1: Update the READMEs:** `world` (grid, fields, projection;
  `TimelineSource` still to come with W2); `catalogue` (identity by
  input hash, the three ports, invalidation, eviction and its budget
  parameter); `adapters` (the on-disk layout block from the spec, world
  binary layout pointer, PNG). Keep each under the guardrail's README
  length limit.
- [ ] **Step 2: Run `mise run ci`.** Expected: format, clippy, tier 1
  and tier 2 within budgets, markdownlint, all green. Fix anything red
  in the task that owns it.
- [ ] **Step 3: Commit**
  `docs(world): document the grid, the catalogue and the store`.
