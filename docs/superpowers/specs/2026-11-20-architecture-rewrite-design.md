# Architecture rewrite: design

Status: draft, 2026-11-20. Decided with the user in a design discussion
on 2026-11-20 (two sessions). Every point marked *(my call)* was filled
in by the writer and is open to correction; everything else was decided
by the user.

## Intent

Features stacked on the current generator keep colliding: one crate of
about 70 000 lines (`crates/mapgen`), where the system draw, the planet
model, the climate, the map passes, the storage formats and the exports
all reach into each other. Each new feature adds constraints that every
later change must respect, for people and for agents alike.

The rewrite gives the project clear boundaries, so that a feature lands
in one place and cannot break the others:

- a **system engine** that computes a system's whole life, from its
  birth, and hands out a **timeline** per body;
- a **map generator** that only turns one dated state of a body into a
  map;
- a pure **science** crate both depend on;
- storage where **only the inputs are the source of truth**.

The rewrite also lands decisions the old structure could not take
cheaply: one simulation grid per planet independent of the display
resolution (#425), the surface layers (#424), cause trees produced by
the computation itself, and systems that change over time.

## Goals

1. A hexagonal, domain-driven layout: each bounded context is a crate
   with its domain, its ports (traits it owns) and no knowledge of the
   adapters plugged into them.
2. A system that evolves: keyframes and dated events from birth to a
   horizon, with a chosen *present* date.
3. A body's map computed once on a fixed simulation grid with no
   polar singularity; any display or export resolution is derived.
4. "Why" for every derived value of a body, at any keyframe, produced by
   the same code that computes the value.
5. Inputs only as stored truth: everything else is a regenerable cache,
   so format migrations shrink to the inputs.
6. A cross-section of every body (planets, large and small moons,
   stars) at any keyframe, from the interiors the engine computes.
7. Room for what is already planned without touching the planet chain:
   multiple-star systems, exotic and evolving hosts, real systems and
   relief, a galactic environment, rogue planets.

## Non-goals

- **The viewer and the server, in the engine phase.** `web/` stays in
  the tree untouched (its unit tests keep running); the server is
  removed. Reconnecting a viewer is a later track (see Viewer contract,
  later).
- **N-body integration.** Instabilities and collisions are stability
  criteria plus a deterministic draw of the date.
- **Full-life plate tectonics** (supercontinent cycles, level C of the
  drift discussion): rejected by the user.
- **Fantasy planets that bypass the physics** (#423, later): not built,
  but the design must not block them (see Causes and provenance).
- **Byte-identical worlds with the legacy generator.** Worlds may change
  as long as they stay coherent (project policy).

## Anti-goals

The rewrite fails, even if it runs, when:

- the map generator reads the central object's kind, or feeds anything
  back into the system engine;
- two code paths compute the same value (the legacy `explain.rs`
  recomputation is the example not to repeat);
- a stored file other than the inputs needs a migration;
- `science` grows a type that knows a body, a system or an id;
- domain code emits human text instead of codes;
- a random draw happens outside the seed hierarchy (#473);
- the calibrated physics of the legacy generator (climate lots, science
  fixes #291-#335 and later) is rewritten from memory instead of ported.

## Constraints

- **Rust** for all new code, strictly typed (the user's preference):
  typed units, no stringly-typed codes in the domain.
- **Determinism:** the same inputs and engine version give the same
  output on the same platform and build.
- **The user's data is never touched.** planets.localhost keeps running
  on the legacy tag during the rewrite; its data directories (`systems/`,
  `worlds/`, `db/` of the data checkout) are read-only for this work.
- **Repository workflow unchanged:** one branch and one pull request per
  feature into `main`, tested on `develop` (`CLAUDE.md`), Conventional
  Commits, cocogitto check.
- **Peer sessions:** the rewrite starts only once mapgen1 and mapgen3
  have merged their work into `main`.

## Approaches

How to migrate:

1. **Refactor in place, behaviour constant**, extracting crates one by
   one with byte-identical worlds. Rejected: with everything coupled,
   extraction costs more than a clean start, and new features would wait
   behind it.
2. **Rewrite beside the legacy and switch at parity.** Rejected: two
   engines to maintain, against the "no legacy algorithms" policy.
3. **Clean slate on `main`, legacy kept as a tag and an oracle**
   (chosen by the user). `main` loses features at first; each feature
   comes back as its own pull request.

In what order to build (approach 3):

1. **Vertical slice, then widen** (chosen): the first pull requests go
   through the whole chain in a minimal form, then each pull request
   adds one feature.
2. Layer by layer (all of science, then the whole system engine, then
   maps). Rejected: no map for weeks, and the timeline contract would be
   designed without its only consumer.
3. Maps first on a stub timeline. Rejected: the stub freezes a contract
   before the engine exists.

## Migration

1. Wait for mapgen1 and mapgen3 to merge into `main`.
2. Rewrite the issues (see Issues rewrite) before the first code pull
   request, so each issue points at a section of this document.
3. Tag the last legacy commit `legacy-final` (an annotated tag, plus a
   `legacy` branch only if a deploy needs one). planets.localhost is
   deployed from it until the user decides the rewrite is enough.
4. The first code pull request removes `crates/mapgen`, `crates/server`,
   `crates/cli`, the server's Docker and mise tasks, and lays out the new
   workspace. It keeps `web/` untouched, keeps `docs/science/`,
   `docs/system-spec.md` and `docs/backlog.md` as references, moves
   `docs/analysis/` to the GitHub Wiki (see Documentation), and
   rewrites `README.md` for the new state. It also
   lands the guardrails (see Guardrails): the mise tasks and the CI that
   runs them, the CI checks, the `CLAUDE.md` architecture section and
   the updated agents. History is
   kept: nothing is force-pushed.
5. Each following pull request adds one feature (see Build order).

**Porting rule.** Physics is ported, not reinvented: a law, a
calibration or a fitted constant is copied from the legacy code into
`science` (with its source citation and its tests), then adapted to
typed units. Only the structure around it is new. Passes that walk the
map row by row are the exception: they are rewritten for the new grid
(see Map generator).

**Legacy oracle.** A `legacy-final` binary stays buildable. The
`world-coherence-checker` agent compares worlds of the two binaries by
statistics (area-weighted land share, relief, temperature, moisture,
and biomes through a mapping from legacy biomes to the new named
biomes), never by bytes. Each feature pull request states the tolerance
it was checked against, and explains any deliberate departure.

## Architecture

### Crates

```text
crates/
  science/    laws and typed units; no body, system or id
  timeline/   the published contract between the two engines
  system/     system engine (domain + ports)
  world/      map generator (domain + ports)
  catalogue/  inputs repository, caches, eviction (domain + ports)
  adapters/   fs storage, PNG export, English text, relief files
  cli/        composition root: wires adapters into ports
```

Dependencies point inwards only:

```text
cli ──> adapters ──> catalogue, system, world, timeline, science
system ──> timeline, science
world  ──> timeline, science
catalogue ──> timeline
timeline ──> science (units only)
```

`timeline` is a *published language* in the DDD sense *(my call)*: the
two engines share its types instead of each translating its own copy,
which would double the contract without adding a boundary. It holds
data types only, no behaviour beyond validation.

### Science

Laws and typed units: radiative balance, escape, phase diagrams,
interiors, tides, orbital mechanics (Kepler, Hill radius, stability
criteria), stellar evolution tracks, crater scaling, and so on.

- Functions take and return typed quantities (`Kelvin`, `Pascal`,
  `Kilogram`, `Metre`, `WattPerSquareMetre`, `Year`…). The exact unit
  library or hand-written newtypes is left to the builder.
- Anything that needs to know a body, a system, a seed or an id is not
  science and lives in an engine.
- No randomness: draws belong to the engines.

### System engine

Owns a system from its birth to its horizon.

**Model.**

- A system is a **hierarchical tree of orbits**, never "one star plus
  planets". A single star is a one-node tree; a close binary is a pair
  node whose circumbinary planets are its children; a hierarchical
  triple is two nested pairs. A tree may have **no central object**
  (rogue planets, #502).
- Tree members are **bodies** (stars, remnants, planets, moons, dwarf
  planets) or **populations** (belts, rings, dust rings, trans-Neptunian
  populations; #305, #331).
- A star is an **object with phases**: protostar and disk, main
  sequence, subgiant, giant, then a remnant (white dwarf, neutron star,
  magnetar, black hole). The phase is a timeline state, not a fixed
  type. The first phase of a system may carry a **protoplanetary disk**
  with accreting planets (#504).
- **Identity:** the IAU designation is assigned at birth and never
  recomputed; ejections and collisions keep the letters. User-given
  names are catalogue data, not engine state.

**Computation.**

- The engine computes the whole life at generation: global climate and
  its feedbacks (albedo, CO2, regime, snowball, runaway, ocean loss),
  interiors, atmospheres and their optical depths, spin and tides,
  orbits, stellar evolution.
- **Collisions and instabilities:** stability criteria, then a
  deterministic draw of the date; no N-body.
- **External events** (stellar flybys, captures, rogue planets passing):
  rate-driven draws from the system's environment, one sub-seed per
  event type, effects by impulse approximation. Flybys feed comet
  showers into the impact flux. Rates are **realistic by default**; an
  injectable setting (dense environment, eventful history) changes the
  environment, never the physics.

**Ports.**

- `EnvironmentSource`: the galactic place and birth cluster. Drawn from
  the seed now; a galaxy context later (#420), which then owns
  encounters between known systems so they stay reciprocal.
- Observed data (catalogues, #418, #419) arrives through the inputs (see
  Storage), not through a port of its own *(my call)*: once fetched, it
  is an input like any other.

### Timeline (the contract)

Per body:

- **Keyframes:** the body's full state at a date. A keyframe is emitted
  when a watched quantity has changed beyond its threshold since the last
  one, with a minimum and a maximum spacing. Thresholds proposed *(my
  call, to be tuned on the first slices)*:

  | Quantity | Threshold |
  | --- | --- |
  | Bolometric flux received | 2 % |
  | XUV flux received | 10 % |
  | Body mass | 1 % |
  | Semi-major axis | 1 % |
  | Eccentricity | 0.01 |
  | Obliquity | 1° |
  | Mean surface temperature | 2 K |
  | Surface pressure | 5 % |
  | Maximum spacing | 100 Myr |

- **Events:** dated, with the state before and after: collisions,
  ejections, captures, flybys, entering the giant phase, ocean loss,
  snowball onset and exit, tidal locking, ring formation… An event
  always brings a keyframe on each side.
- **Epochs** (for drift previews and the viewer's history) are derived
  from keyframes and events, not stored.

What a keyframe carries for the map generator is **physical forcings
and states, never the central object's kind**: bolometric and XUV/X
fluxes (summed over all stars, with a variability term for circumbinary
orbits; #592), stellar wind and field at the body, the stellar spectrum
class needed for colours, orbital elements, spin state (period,
obliquity, lock and its substellar longitude), mass and radius, interior
and regime, the global climate state, atmosphere composition, pressure
and optical depths, surface liquid and its inventory, life. A new kind
of central object therefore never touches the map generator.

**Span.** The timeline runs from birth to a horizon; the *present* is a
marked date chosen in the inputs. Horizon *(my call)*: the end of the
last host phase that still changes the forcings, capped at the present
plus a configurable span (default 10 Gyr), because a red dwarf's main
sequence outlasts any useful date.

### Map generator

Turns one keyframe or event state of one body into a world. Maps exist
only at keyframes and events, generated on demand.

**Port:** `TimelineSource` (read a body's keyframe at a date). One seed
per context, derived from the system seed and the body's designation.
Also `ReliefSource` for imported relief (#422), resampled onto the
simulation grid.

**Ocean share** *(my call)*: the system engine owns how much of the
surface is ocean (it needs it for the global albedo and climate), from
the water inventory and a hypsometry model of its own; the timeline
publishes that share. The map generator sets its sea level as the
quantile of its relief that gives this share. The legacy code computed
the share twice (`ocean_fraction` in `planet/albedo.rs` against
`sea_level_for_water` in `generation.rs`); there is now one owner.

**It never feeds back.** Global climate comes from the timeline; the
map generator spatializes it (latitudinal and seasonal distribution,
ocean currents, orographic rain, day-night patterns of slow and locked
rotators, #322-#325).

**Simulation grid.**

- An equi-angular **cube-sphere** (the legacy `crates/mapgen/src/cube.rs`
  is the starting point): six faces, cells varying by about 30 %, no
  pole anywhere, neighbours found across face edges and the eight
  three-face corners.
- **Fixed cell size of 15 km** on solid-surface and ice-shell worlds,
  whatever their radius. An Earth has about 2.7 M cells; Mars about
  0.76 M; a 500 km moon a few thousand. The cell size is an engine
  constant recorded with the engine version; `generation-profiler`
  measures the first slice, and it may go down to 10 km if the cost
  allows.
- **Cap** *(my call)*: above 8 M cells (a solid body of about 1.7 Earth
  radii), the cell grows so the count stays at 8 M, keeping the
  generation peak near 1 GB. Giant-atmosphere worlds do not use the
  15 km cell (Jupiter would need about 285 M cells): their grid has a
  fixed cell count chosen for the bands, set with that world kind.
- Memory order of magnitude for an Earth at 15 km: about 120 MB for the
  stored fields in `f32` plus about 40 MB of seasonal statistics, 250 to
  350 MB at the generation peak. Fields that do not need `f32` are
  quantised on disk (`u16`, `u8`).
- Passes that walked rows in the legacy code (moisture transport, coast
  distance, priority-flood hydrology, erosion, cold currents) are
  rewritten as neighbourhood passes on this grid; local passes (column,
  atmosphere, surface classification) are ported as they are.
- Plate simulation on a point mesh (#293) remains a later option on the
  same grid.

**World kinds.** A body's world is one of:

1. **Solid surface**: relief, hydrology, climate fields, surface layers.
2. **Ice shell over an internal ocean**: shell thickness, fractures,
   geysers and plumes (Europa, Enceladus, Ganymede).
3. **Giant atmosphere**: bands, winds, spots; no relief.

The kind follows from the keyframe's state, not from a preset.

**Surface layers (#424), built directly** (no legacy biome is ported):

- climate: a full Köppen-Geiger class (about 30) from temperature,
  seasonality and precipitation;
- surface: continuous fields for material (rock types, regolith, sand,
  salt, ice), surface liquid and its nature, vegetation cover 0-1 (only
  where there is life), seasonal snow;
- a **named biome** derived from climate and surface, for labels and
  legends; the invented biomes go away.

Köppen needs seasonality, so each cell stores seasonal statistics (for
example warmest, coldest and driest month, precipitation seasonality),
about ten fields, not twelve full months *(my call)*.

**Drift.** Level A, previews of the relief at keyframes (from the legacy
`drift.rs`), and level B, a cached low-resolution animation. No
full-life tectonics.

**Output resolutions are derived.** An export or display at any
resolution, including the equirectangular 2:1 map, is a projection: for
each output pixel, take its true point on the sphere and sample the
grid there, averaging every cell the pixel covers (which matters near
the poles, where a pixel is a long, thin strip). Fine detail below the
cell size is synthesised, as the legacy deep-zoom tiles do.

### Catalogue and storage

**Only inputs are the source of truth.** A system's stored inputs are:
seed, archetype, environment overrides, the present date, the engine
version, observed data with its provenance (#418, #419), and imported
relief files (#422). Everything else (timelines, worlds, projections,
thumbnails) is a regenerable cache.

- Ports: `SystemRepository` (inputs), `TimelineCache`, `WorldCache`;
  `SystemIndex` (the SQLite read model) comes back with the server.
- Eviction policy is catalogue domain; atomic staging of files is an fs
  adapter detail.
- Inputs carry a schema version; only they ever migrate. A cache is
  invalidated by the engine version, never migrated.
- Storage format of caches *(my call)*: JSON for timelines and inputs
  while they are small and debugged by eye; a binary layout for world
  fields from the start. Both are free to change, being caches.
- When a new engine version regenerates a system from its inputs, its
  worlds may change; that is the project's policy, not a bug.
- **Legacy systems are not carried over.** The systems stored by the
  legacy app stay on planets.localhost (deployed from `legacy-final`);
  no importer is written, and the new store starts empty.

### Requests and archetypes

Two kinds of request, as in the legacy app *(my call)*: a whole system
(host, metallicity, present date, planets drawn or asked per archetype),
or one body with a system drawn around it (around a star, or as a moon
of a drawn giant).

**An archetype describes the body at the present date**, not at its
birth: "earth-like" promises an Earth-like planet today. The engine:

- sets what barely changes over a life directly: mass, bulk
  composition, orbit;
- searches backwards for the birth inventories (water, CO2, N2…) that
  give the archetype's present state, by running the forward
  computation on one or two parameters until it lands on the target
  (a shooting search with a bounded number of runs);
- records a target it cannot reach (for example an ocean that boils
  under the present luminosity whatever the inventory) as an outcome
  with its cause, and an outcome forced by the archetype (a pinned
  snowball, life) with the provenance *imposed*.

The tolerance of each target and the search method are left to the
builder.

### Cross-sections

Every body can be shown cut open: planets, large and small moons, and
stars (#423 listed the planet's cross-section for later; it becomes
part of the design). The cut is **data produced by the system engine**,
which already computes interiors; how it is drawn belongs to adapters
(an SVG export now, the viewer later).

Per body and per keyframe, the timeline carries an **interior profile**:

- an ordered list of **layers** from the centre out, each with its
  outer radius, material or phase, and state (solid, liquid, plasma,
  partly molten, porous);
- **radial profiles** sampled along the radius: density, pressure,
  temperature, and for stars the energy transport (radiative or
  convective) and the share of fusion;
- the **causes** of each layer (why the core is liquid, why there is an
  ocean under the ice), like any other derived quantity.

What the layers are depends on the body, not on a fixed template:

| Body | Layers, for example |
| --- | --- |
| Rocky planet | Inner and outer core, mantle, crust, ocean, atmosphere |
| Large icy moon | Core, rocky mantle, high-pressure ice, ocean, ice shell |
| Small body | Undifferentiated rock and ice, or a rubble pile |
| Giant | Core, metallic hydrogen, molecular envelope, cloud decks |
| Main-sequence star | Core, radiative zone, convective zone, photosphere |
| Evolved star | Inert core, burning shells, envelope |
| Remnant | Crust and degenerate interior, or a horizon for a black hole |

Small bodies keep their irregular shape (`docs/science/moon-shapes.md`):
the profile then holds the shape's outline in the cut plane, not only a
radius. Profiles evolve with the keyframes, so a cut can be shown at
any date (a core freezing, an ocean refreezing, a star growing a
convective envelope).

### Causes and provenance

- Every derived quantity of a body is **produced with its cause node**
  by the computation that derives it: its value, the rule that gave it
  (a code), and the quantities it came from. There is no second path
  that recomputes values to explain them. The mechanism (a wrapper type,
  a recording context…) is left to the builder; the requirement is one
  code path.
- Events carry causes too ("ocean lost at 3.2 Gyr: runaway greenhouse,
  because the host's luminosity reached L at distance a"). Every
  keyframe carries its trees, so "why" works at any date.
- **Leaves carry provenance:** input set by the user, observed
  (catalogue), drawn (seeded), or imposed (an archetype that forces an
  outcome, the door left open for fantasy planets).
- **Map cells** do not store trees. "Why this biome here?" recomputes
  locally from the cell's stored fields with the same code, which is
  safe since the computation is deterministic.
- Trees are codes and numbers only (see Text and languages); the CLI
  prints them with `--explain`.

### Determinism and seeds

- One root seed per system, and a derived sub-seed per context: system
  draw, each event type, each body (by designation), each map pass. No
  draw outside this hierarchy; ambient or unseeded random sources are
  forbidden by a lint (clippy `disallowed_methods`) and a test that
  regenerates a system twice.
- Tests that compare outputs use statistics or same-platform bytes, never
  cross-platform byte goldens.

### Text and languages

The domain emits codes, parameters and cause trees, never text.
Translation lives in adapters: a minimal English catalogue for the CLI
now, the web app's English and French catalogues when the viewer comes
back.

### Viewer contract, later

Not built in the engine phase, but the design keeps it possible:

- The current viewer talks to the server through JSON
  (`web/src/lib/api/client.ts`) and reads binary formats directly
  (`raw_data` in `web/src/lib/layers/decode.ts` and its workers, tiles
  in `web/src/lib/viewer/TileLayer.ts`). Reconnecting it means a server
  adapter that serves the same contract, with equirectangular `raw_data`
  projected on demand from the simulation grid; resolution chips become
  a display choice, size jobs disappear.
- **No WebAssembly:** the server hands out orbital elements per keyframe
  and the viewer keeps solving Kepler itself, as it does today
  (`keplerPosition` in `web/src/lib/viewer/starSystem.ts:281`). About a
  hundred lines of Kepler are easy to rewrite in any renderer, including
  a game engine later.
- The galactic sky backdrop stays a viewer concern, fed by the
  environment and varying with the date.
- The pole artefacts of #425 disappear only once a viewer samples the
  grid directly (a cube map for example), a later step.

## Guardrails

The architecture is enforced by tools first, by written rules second.
All of it lands in the first code pull request, with the crates it
describes.

**One command for the CI, locally and on GitHub.** The first pull
request brings back mise tasks for the new workspace, with a single
`mise run ci` that runs everything the pipeline runs: format check,
clippy, tests, the guardrail checks below and markdownlint. The GitHub
workflow installs mise and calls the same tasks (one job per task when
they are worth running in parallel), so a green `mise run ci` locally
means a green pipeline. Heavy tests keep their own task outside `ci`
(`test:heavy`, see Tests). Other tasks (`gen`, `export`, `oracle`,
`fmt`) are added with the features that need them.

### Tests

The legacy suite grew to 1 069 Rust tests, and its pipeline from about
8 minutes to more than two hours (two runs on 2026-11-20: 1 h 12 and
2 h 23). Most of the time went into generating worlds: each feature
added tests that generated their own planet. The rewrite keeps tests in
three tiers with hard budgets.

| Tier | What | Where | Budget |
| --- | --- | --- | --- |
| 1. Unit | Laws, pure functions vs published values | Every push | 2 min |
| 2. Regression | Shared reference systems, contracts | Every push | 8 min |
| 3. Heavy | 15 km worlds, oracle, censuses, profiles | Local, manual | None |

Tiers 1 and 2 run on GitHub at every push; tier 3 runs locally or by
hand on GitHub.

- **Tier 2 shares its worlds.** A few reference systems are generated
  once per run on a coarse test grid (60 km *(my call)*) and every
  tier-2 test asserts on them: statistics with tolerances, contracts,
  determinism. No test generates a world of its own.
- **Budgets are enforced.** cargo-nextest runs one profile per tier,
  and CI fails when tier 1 or tier 2 passes its budget. The answer is
  an arbitration: move a test to tier 3, merge it with another, or
  delete it.
- **Tier 3 is proven in the pull request.** A pull request that
  changes what the engines produce pastes the output of
  `mise run test:heavy` (oracle report, censuses) in its description.
  The same tier can be started by hand on GitHub
  (`workflow_dispatch`).
- **Expected values come from sources, not from the code** (No
  invented data). A test that pins the code's current output is allowed
  only in tier 2, as a regression net, never as proof that a law is
  right.
- The budgets are the user's to change.

**`test-reviewer` agent** (new, read-only, sonnet, medium effort), run
once per branch before its pull request, alongside `branch-reviewer`.
It checks that each new test sits in the right tier and generates no
world of its own, finds duplicates and tests made useless by the
change, checks that expected values have a source, and measures with
nextest what the branch adds to each tier against its budget. It
proposes arbitrations (move, merge, delete); the main session decides,
and the user when a budget is at stake.

**GitHub Actions caches**, to keep the pipeline short:

- `jdx/mise-action` caches the tools and `Swatinem/rust-cache` the
  compiled dependencies, as in the legacy workflow.
- **Path filters:** a pull request touching only `docs/` or `web/` does
  not run the Rust jobs. A job that always reports keeps the required
  checks satisfied.
- **Tier-2 fixtures cached:** the reference worlds are written to disk
  and cached under a key hashing the sources of `science`, `timeline`,
  `system`, `world` and `Cargo.lock`, so a change to adapters, the CLI
  or the docs does not regenerate them, and a stale fixture cannot
  survive an engine change.
- **Partitions:** tier 2 is split across runners
  (`cargo nextest --partition`) when its budget gets tight.
- **`main` keeps the cache warm:** branches read `main`'s cache, not
  each other's, so CI runs on every push to `main`. GitHub keeps 10 GB
  per repository and evicts the least recently used entries.
- **Tier 3 on GitHub** caches the `legacy-final` oracle binary under
  the tag's SHA, so it is built once.

**CI checks.**

- A test reads `cargo metadata` and fails when a dependency points the
  wrong way (the graph in Crates): `world` depending on `system`,
  `science` depending on anything but its units, a domain crate
  depending on `adapters`.
- Clippy `disallowed_methods` forbids random sources outside the seed
  hierarchy; `disallowed_types` forbids `std::fs`, `std::net` and
  process spawning in `science`, `timeline`, `system`, `world` and
  `catalogue`. Only `adapters` and `cli` touch the outside world.
- The determinism test (Determinism and seeds).
- **A short README.** The legacy `README.md` grew to about 240 KB, too
  large to read or maintain. The new one keeps to purpose,
  prerequisites, installation, usage and configuration, and links out
  for the rest; each crate has its own short `README.md` for its role
  and ports. A CI check fails when the root README passes 300 lines or
  a crate README 150 *(my call)*: detail goes into `docs/`, never into
  a README.

**`CLAUDE.md` architecture section**, short and pointing to this
document:

- the crates, their role and the direction of dependencies;
- where a change goes:

  | Change | Crate |
  | --- | --- |
  | A physical law, a unit, a calibrated constant | `science` |
  | How a body or system changes over time, an event | `system` |
  | What the engines exchange | `timeline` |
  | How a state is laid out on the surface | `world` |
  | What is stored, cached, evicted | `catalogue` |
  | A file format, an export, a text catalogue | `adapters` |
  | Wiring, command-line options | `cli` |

- the anti-goals of this document as rules: no feedback from the map to
  the system engine, no central-object kind read by the map, one code
  path per value (causes included), codes not text in the domain, only
  inputs stored, physics ported not reinvented;
- the **no invented data** rule (Documentation), for Claude and every
  agent.

**Agents** (`.claude/agents/`), updated to the new layout:

| Agent | Change |
| --- | --- |
| `codebase-mapper` | The new crate map |
| `branch-reviewer` | Boundaries, engine version, uncited numbers |
| `science-fixer` | Targets `science`, ports physics, studies first |
| `branch-syncer` | Description without `raw_data` and tiles |
| `test-reviewer` | New: tiers, duplicates, sources, budgets (Tests) |
| `world-coherence-checker` | The oracle: legacy against new, by distributions |
| `generation-profiler` | The new CLI |
| `develop-deployer` | Paused during the engine phase |
| `viewer-visual-check` | Paused during the engine phase |

The `CLAUDE.md` table of agents and the README follow.

## Documentation

Two kinds of documents, kept in two places:

- **Studies** (`docs/analysis/`, about 700 KB): written once before a
  decision (deep zoom, host stars, binaries…), then archived. They move
  to the repository's **GitHub Wiki** in the first code pull request,
  and links to them (issues, specs, `CLAUDE.md`) point there. New
  studies are written in the Wiki.
- **Science reference** (`docs/science/`, about 900 KB): the model's
  laws, sources and calibrations. It stays in the repository, because a
  page must change in the same pull request as the law it describes,
  and is published as a site on **GitHub Pages** built with mdBook,
  deployed by a workflow on every merge into `main` (a `docs:build`
  mise task builds it locally too).

**Writing rules for the science reference**, enforced in review and
partly by CI:

- Written like a scientific article, in English: abstract,
  introduction, model (with its equations), calibration, validation
  against observed bodies, limitations, references.
- **Every law, constant and calibration cites its source** (authors,
  year, title, journal, DOI or URL) in the page's references; a number
  without a source is either cited, derived on the page, or marked as
  the project's own calibration with how it was fitted.
- Equations in LaTeX (rendered by MathJax or KaTeX).
- **Figures**: diagrams (SVG or Mermaid) for structures and flows, and
  plots for laws and validations. Plots are generated by a script from
  the code's own output (a `docs:figures` mise task), never drawn by
  hand, so they follow the code.
- The `science` crate's doc comments cite the same source and link to
  the page.
- CI checks: the book builds, links resolve, markdownlint passes, and
  every page has a non-empty References section.

The legacy pages are rewritten to these rules as their laws are ported
(Porting rule), not all at once.

### No invented data

Values have been made up in the past where the science was missing.
The rule, for Claude and every agent, in `CLAUDE.md` and in the agents
that touch physics:

1. **Missing or thin science stops the work.** When a law, a constant
   or a range is needed and the reference does not cover it, or covers
   it with a study too shallow for the case, do the study first: search
   the literature, then create or extend the reference page (with its
   sources) before writing the code that uses it. A larger question
   gets a study in the Wiki, and the page cites it.
2. **Calibrate only when no data exists.** If the study finds no
   measurement, model or published range for the case, a value may be
   calibrated. It is then:
   - marked in the code: its doc comment starts with `Calibration:`,
     says what it was fitted against and how, and links the page;
   - described in the page's calibration section, with what was
     searched and not found;
   - listed in the pull request description, under its own heading, so
     the review sees every new calibration.
3. **Every number in `science` is either sourced or calibrated.** A CI
   check fails when a constant (`const`, `static`, or a numeric literal
   bound by name) in `science` has no doc comment starting with
   `Source:` or `Calibration:`. Plain mathematical numbers (0, 1, 2, π,
   unit conversions) are exempt; the exact exemption list is left to
   the builder.
4. `branch-reviewer` flags any new number that is neither sourced nor
   calibrated, anywhere in the engines too, not only in `science`.

## Build order

### Slice 1: the vertical slice

Through the whole chain, minimal:

- `science`: only the laws the slice uses, ported with their tests;
- system engine: one main-sequence star, rocky planets and giants, no
  moons; a timeline with **one keyframe, the present**; global climate;
  causes; designations; the interior profiles of the star and the
  planets (Cross-sections);
- map generator: cube-sphere grid at 15 km, solid-surface worlds only:
  relief, spatialized climate, Köppen class, surface layers, named
  biomes; seasonal statistics;
- catalogue: inputs repository and caches on the filesystem;
- CLI: generate from inputs, write the timeline as JSON and the world
  fields, export a 2:1 PNG projection and an SVG cross-section of each
  body, print cause trees with `--explain`;
- determinism lint and test; legacy oracle comparison on a fixed set of
  seeds per archetype.

It may be split into several pull requests (science and units, system
engine, grid, map passes, CLI and oracle), each leaving `main`
buildable and tested.

### Then, one pull request per feature

The order below is a proposal, refined in the rewritten roadmap:

1. Hydrology, erosion, coast distance and ocean currents on the grid.
2. Moons and the tree's nesting; tides and spin; the moons' profiles,
   small irregular bodies included.
3. Ice-shell and giant-atmosphere world kinds.
4. Keyframes in the past and future: stellar evolution on the main
   sequence, thresholds, epochs; drift level A then B.
5. Events: snowball, runaway, ocean loss, tidal locking; then
   collisions and instabilities.
6. Environment and external events (flybys, captures, comet showers).
7. Host phases and remnants, with their profiles; binaries and triples
   (#585's steps).
8. Populations: belts, rings, dust rings, trans-Neptunian objects.
9. Real systems and real relief (#418, #419, #422).
10. Server and viewer reconnection (a track of its own).

## Issues rewrite

Done once this design is approved, before the first code pull request:

- Roadmap #427 becomes the rewrite roadmap, with its new tracks,
  matching Build order.
- Every open issue (145 on 2026-11-20) is triaged:
  - **kept**: a scientific or functional need that survives, rewritten
    for the new architecture (what, why, which crate, which section of
    this document) and placed in a track;
  - **merged** into another issue;
  - **closed as not planned** with the label `superseded-by-rewrite` and
    a link to what replaces it.
- Expected superseded, among others: resolution and size issues
  (#380-#386), explanations that disagree with stored values (#517,
  #520, #526), server storage and costs (#382, #383), double
  derivation (#538), the hex grid (removed by #424).
- Viewer issues stay open, grouped under the viewer reconnection track.
- Science issues (#291-#335, #494-#597 and others) are kept when their
  physics still applies, and rewritten to target `science` or an engine.

## Proof

- Each slice and feature pull request: unit tests ported with the law,
  the determinism test, and a `world-coherence-checker` comparison
  against `legacy-final` with its tolerance stated.
- The first slice is profiled (`generation-profiler`) before the cell
  size is fixed.
- `branch-reviewer` reviews each branch once before its pull request.
- **Testing on `develop` during the engine phase** *(my call)*: with no
  server, `develop-deployer` and planets.localhost are not used for the
  new code. A pull request is tried by building `develop` and running
  the CLI on the oracle's seeds; it delivers the oracle report and
  downscaled 2:1 PNG projections of a few bodies, which the user looks
  at before approving the merge. The rebuild of `develop` and the
  merge rules of `CLAUDE.md` stay as they are.

## Left to the builder

- The units library or hand-written newtypes.
- The mechanism that records causes during computation.
- The binary layout of world fields and their quantisation.
- The mdBook plugins (maths, Mermaid, link check) and the plotting tool.
- The exact list of seasonal statistics, within Köppen's needs.
- The split of slice 1 into pull requests.
- The set of seeds and archetypes used by the oracle, and the tolerance
  per statistic (stated in each pull request).
- Keyframe thresholds and horizon cap: the values above are a start.
