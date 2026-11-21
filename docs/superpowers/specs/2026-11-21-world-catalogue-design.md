# World and catalogue: design

Part of slice 1 (#602) of the architecture rewrite (roadmap #427).
Parent design: [architecture rewrite](2026-11-20-architecture-rewrite-design.md),
sections Map generator, Catalogue and storage, Tests. Contract:
[rewrite contract](2026-11-21-rewrite-contract-design.md). Owner: mapgen2.

## Intent

The slice needs a map generator (`world`) that turns one planet keyframe
into a world on the cube-sphere grid, and a catalogue that stores the
inputs of a system and caches what is regenerated from them. Both crates
are empty on `main`. The contract (`timeline::PlanetState`) already
carries every field the map generator reads; mapgen1 fills the global
climate, water and class in the planet model's PR 2, and mapgen3 the
star and the system draw.

## Goals

- A world for a solid-surface planet at the present keyframe: relief,
  sea level from the published ocean share, spatialized climate,
  seasonal statistics, Köppen-Geiger class, surface layers, named biome.
- A catalogue: inputs as the only stored truth, caches of timelines and
  worlds invalidated by the engine version, an eviction policy.
- An equirectangular 2:1 projection of any world field, for the PNG
  export and the oracle.

## Non-goals

- Ice-shell and giant-atmosphere worlds, moons, drift, past and future
  keyframes, hydrology and erosion (later tracks, parent design Build
  order).
- The SQLite index (`SystemIndex`, track T10).
- Imported relief (`ReliefSource`, track T9).

## Anti-goals

- `world` reads the central object's kind, or writes anything back into
  the timeline.
- A value computed twice: sea level is the quantile of the relief that
  gives the timeline's `ocean_share`, never a second water budget.
- A cache that is migrated, or a world stored as truth.
- A projection that resamples rules per pixel: every field lives on the
  grid, and a pixel averages the cells it covers.

## Constraints

- Domain crates (`world`, `catalogue`) touch no file, clock or process
  (`clippy.toml`): file layout, staging, PNG and binary encoding live in
  `adapters`.
- Dependencies: `world` on `timeline` and `science` only; `catalogue` on
  `system` and `timeline` (`tools/guardrails`).
- Draws from `Seed::derive` only: one sub-seed per map pass, derived from
  the body's seed.
- Physics ported from `legacy-final` with its source and tests; passes
  that walked rows are rewritten as neighbourhood passes.
- Tests: tier 1 on pure functions and hand-built states; tier 2 on the
  shared reference systems at a 60 km test grid; 15 km worlds only in
  tier 3.

## Approaches

How to split the work (decided with the user on 2026-11-21):

1. **Infrastructure first, then one pull request per layer of science**
   (chosen): W1 catalogue, grid, field container and projection; W2
   relief; W3 spatialized climate and seasonal statistics; W4 Köppen,
   surface layers and named biomes. Each review is either plumbing or
   science, never both, and W1 waits on no one.
2. Relief first, for an early map. Rejected: it mixes the storage and
   grid review with the first physics port.
3. One pull request for the whole map generator. Rejected: too large to
   review, and W3 waits on mapgen1's climate anyway.

## The pull requests

| PR | Content | Waits on |
| --- | --- | --- |
| W1 | Catalogue ports and policy, fs store, grid, fields, projection, PNG | Nothing |
| W2 | Relief on the grid, sea level, first CLI world export | mapgen3's system command (CLI only) |
| W3 | Spatialized climate, seasonal statistics | mapgen1's planet PR 2 (climate, water) |
| W4 | Köppen-Geiger, surface layers, named biome | W3 |

W2 to W4 get their own short design when they start: each begins with
the science study the No invented data rule asks for (relief from
`legacy-final` noise, plates and craters; climate spatialization; the
Köppen-Geiger definitions of Kottek et al. 2006 and Beck et al. 2018).
This document fixes W1 and the shape the others plug into.

## W1 in detail

### Grid (`world::grid`)

- Port `legacy-final:crates/mapgen/src/cube.rs` with its two tests
  (even coverage, mutual neighbours across edges): equi-angular cube,
  cell `k = (face × n + y) × n + x`, eight neighbours, edges and
  three-face corners resolved through the sphere.
- `Grid::for_radius(radius, cell)`: faces of `n = round(π/2 × R / cell)`
  cells per side (a face spans a quarter of a great circle), so an Earth
  at 15 km has `n = 667` and 2.67 M cells. Above 8 M cells, `n` is
  reduced to stay at 8 M (the parent design's cap). The default cell size
  (15 km) is a `Definition:` constant recorded with the engine version;
  the tier-2 test grid uses 60 km.
- Per-cell area (the spherical quadrilateral's solid angle) is computed
  once and kept with the grid: every area-weighted statistic uses it.

### Fields (`world::fields`)

- `Field<T>`: one value per cell, `T` in `f32`, `u16`, `u8`; a
  `World` holds the grid, the body's designation, the keyframe date,
  the engine version and a map of named fields (`FieldId`, a code).
- W1 ships the container and its invariants (length equals the cell
  count; fields named by code); the fields themselves arrive with W2 to
  W4.
- Quantisation for storage is declared per field (range and step), so
  the 20 K to 3000 K temperature criterion (from #450) is a property of
  the field's declaration, checked by a tier-1 test when W3 adds it.

### Projection (`world::project`)

- `project(world, field, width) -> Raster<f32>` with `height = width/2`.
  For each pixel, the pixel's footprint on the sphere (a latitude-
  longitude box) is sampled with `k × k` points, `k` chosen from the
  ratio of the footprint's area to the local cell area (at least 1), each
  point weighted by its area (`cos φ`); the pixel is the weighted mean of
  the cells hit. Categorical fields (`u8` classes) take the area-weighted
  mode instead of the mean.
- Pure and deterministic; the PNG encoding and the colour ramps live in
  `adapters`.
- Acceptance criterion carried from #381 (tier 2, with W2): two export
  resolutions of one world agree on area-weighted statistics with the
  grid's.

### Catalogue (`catalogue`)

- **Identity.** A stored system is keyed by `SystemId`: a 64-bit FNV-1a
  hash of the canonical JSON of its `Inputs`, written as 16 hex digits
  *(my call)*. The same inputs give the same id, so a regeneration finds
  its caches; the MGS number stays the engine's designation (it is drawn
  from the seed and can collide, so it cannot be a key). A user-given
  name is catalogue data beside the inputs. The inputs are hashed as
  given, not with their defaults resolved *(my call, discussed with
  mapgen3)*: `present: None` means "draw the age", a different request
  from a fixed date even when the draw lands on it, and resolving it
  would run the engine before storing. The archetype counts are a
  `BTreeMap`, so their JSON is already in sorted order.
- **Stored truth.** `StoredSystem { id, inputs, engine, name }`: the
  inputs and the engine version that first generated them (stamped by
  the catalogue, as the contract design says). The inputs schema is
  `system::INPUTS_SCHEMA`; a repository refuses an unknown schema with a
  typed error (no migration exists yet, none is written before one is
  needed).
- **Canonical form.** The canonical JSON is `serde_json::to_string`
  of `Inputs`: struct fields in declaration order, maps already sorted
  (`BTreeMap`), floats in serde_json's shortest round-trip form. The
  hash is the FNV-1a 64 of `timeline::seed`, made public there (one
  implementation; a small additive contract change announced to mapgen1
  and mapgen3) *(my call)*.
- **Ports** (traits in `catalogue`):
  - `SystemRepository`: `put`, `get`, `list`, `remove` on stored
    systems.
  - `TimelineCache`: `get` and `put` of a system's timelines, one
    `timeline::Timeline` per body (`Vec<Timeline>`), keyed by
    `(SystemId, EngineVersion)`.
  - `WorldCache`: `get` and `put` of a world, keyed by `WorldKey {
    system: SystemId, body: Designation, keyframe: u32, grid: u32,
    engine: EngineVersion }`. `catalogue` cannot name `world::World`
    (dependency rule), so the trait has an associated `type World`; the
    fs adapter, which depends on `world`, fixes it to `world::World` and
    owns the codec *(my call)*.
  - `keyframe` is the keyframe's index in the body's timeline, stable
    for one engine version (timelines are deterministic and the key
    carries the version), which avoids `f64` dates in keys and file
    names *(my call)*.
  - `grid` is the effective cells per face side `n` after the 8 M cap,
    not the requested cell size: two requests that give the same grid
    share a world *(my call)*.
- **Invalidation.** A cache entry whose engine version differs from
  `system::ENGINE_VERSION` is a miss, and is removed by the next
  eviction; never read, never migrated.
- **Eviction policy** (domain, pure): given the entries (key, size in
  bytes, last use as seconds since the Unix epoch, a `u64` the adapter
  reads from the file's modification time and touches on every cache
  hit) and a byte budget,
  return the entries to drop: stale engine versions first, then least
  recently used until under budget. The budget is a parameter (asked by
  the user): the policy takes it as an argument, the fs store takes it
  at construction, and W2 exposes it as a CLI option
  (`--cache-budget`, in bytes with an optional unit). Its default is
  2 GiB *(my call)*.
- **Filesystem adapter** (`adapters::store`): one root directory,

  ```text
  <root>/systems/<id>/inputs.json        stored truth
  <root>/cache/<engine>/<id>/timelines.json
  <root>/cache/<engine>/<id>/worlds/<designation>/<keyframe>-<grid>.world
  ```

  writes staged in a temporary file then renamed; reads of a missing or
  corrupt cache entry are misses, never errors.
- **World binary layout** (adapter): little-endian; magic `MGWORLD1`,
  header (engine version, designation, date, grid `n`, field count),
  then per field its code, type and quantisation, then the cell values.
  Free to change, being a cache.

### Adapters and CLI

- `adapters` gains the fs store, the world codec, and the PNG writer of
  a raster with a colour ramp (`png` crate).
- No CLI command in W1: `world` and the catalogue are wired into the
  CLI with W2, once there is a relief worth exporting. mapgen3 confirmed
  on 2026-11-21 that `crates/cli` and `crates/adapters` are mapgen2's
  for now; its `system` command (its PR 3) is added next to the world
  commands and plugs into the store. A change to the CLI's skeleton
  (argument parser, `main`) is announced to mapgen3.

### Tests of W1 (tier 1)

- Grid: the ported legacy tests; cell counts for Earth and Mars radii
  against the parent design's figures; areas summing to `4π`.
- Projection: a field that is a known function of latitude projects to
  that function; a constant field stays constant at every width; a
  categorical field keeps its classes.
- Catalogue: id stable across runs and changed by any input change;
  eviction order on a hand-built entry list; stale engine is a miss.
- Adapter: round trip of inputs, timelines and a small world through a
  temporary directory; a truncated world file is a miss.

## Left to the builder

- Module names inside the crates, the exact error enums, the colour
  ramps of the PNG writer.
- Whether the projection's `k` is capped (a performance choice, measured
  with `generation-profiler`).

## Documentation

W1 updates the READMEs of `world`, `catalogue` and `adapters` (ports,
layout on disk) and the parent design's Catalogue section if a call
here changes it.
