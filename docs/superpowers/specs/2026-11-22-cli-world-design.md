# CLI world export: design

Part of W2 of the [world and catalogue design](2026-11-21-world-catalogue-design.md)
(slice 1, #602), section Adapters and CLI, and of the CLI section of the
[relief design](2026-11-21-world-relief-design.md). Owner: mapgen2.

## Intent

W2 ends with a command that turns a stored system into a world and
exports its fields and a 2:1 PNG projection, with a parametrable cache
budget. The relief is on `main` (#642), but nothing produces a planet
timeline yet: `system::generate` is a future signature and
`planet::model` waits on mapgen1's planet PR 2d. This design wires
everything that does not need the engine's planet states now, and leaves
one small step for when they exist.

Update 2026-11-27: `system::generate` exists, with the producer's
signature (Pipeline, below:
`fn(&Inputs) -> Result<Vec<Timeline>, SystemError>`,
`docs/superpowers/specs/2026-11-27-system-generate-design.md`); pull
request B can pass it as the producer.

## Goals

- Global options of the CLI: `--root <DIR>` (the store) and
  `--cache-budget <SIZE>`.
- A world-side engine version in the cache key, so a change to the map
  generator invalidates worlds without bumping `system::ENGINE_VERSION`
  (agreed with mapgen3).
- The world pipeline: get or generate the timelines, pick the body and
  its present keyframe, get or generate the world on its grid, evict over
  the budget, write the world file and the PNG projection of one field.
- The export check carried from #381: two export widths of one world
  agree on area-weighted statistics.

## Non-goals

- `system::generate` and the `world` subcommand itself (pull request B,
  after the planet PR 2d; mapgen3 writes `generate` right after the 2d,
  decided by the user on 2026-11-22, and announces its signature).
- Storing the inputs from `map-generator system` (unchanged: it prints
  its report).
- Listing or removing stored systems from the CLI.

## Anti-goals

- A CLI path that feeds the map generator states the engine did not
  produce (hand-built `--state earth`): a second source for a value.
- A user-facing command that cannot run end to end.
- Writing anywhere but `--root` and `--out`.

## Approaches

Decided with the user on 2026-11-22:

1. **Split** (chosen): pull request A now (skeleton, store wiring,
   world engine version, export pipeline tested on fixture timelines);
   pull request B after the planet PR 2d, which exposes `world` and
   plugs in the engine's timelines.
2. One pull request merged after the 2d. Rejected: nothing can be tried
   end to end before then, and the review grows.
3. Hand-built states behind a CLI flag. Rejected: see Anti-goals.

## Design of pull request A

### Arguments (`crates/cli/src/args.rs`)

- `GlobalArgs`, flattened into `Cli` with `global = true`:
  - `--root <DIR>`, default `./map-generator-store` *(my call: a name
    that cannot be mistaken for the legacy `systems/`, `worlds/`, `db/`
    directories)*;
  - `--cache-budget <SIZE>`: an integer of bytes with an optional IEC
    suffix (`KiB`, `MiB`, `GiB`, `TiB`, case-sensitive), parsed by
    `parse_size`; default `catalogue::DEFAULT_BUDGET` (2 GiB). SI
    suffixes (`MB`) are refused with a message naming the IEC one, so
    a budget is never off by 7 % *(my call)*.
- `SystemArgs` unchanged; `main` keeps the drawn-seed notice, moved into
  a helper the `world` arm will share.

### World engine version

- `world::ENGINE_VERSION: EngineVersion` (1), with the rule in
  `crates/world/README.md`: bumped by any change to what the map
  generator produces.
- `catalogue::WorldKey` gains `world: EngineVersion`; its existing
  `engine` stays the system engine (the timelines it was built from).
- On disk the world file name carries it:
  `<root>/cache/<engine>/<id>/worlds/<body>/<keyframe>-<grid>-w<world>.world`;
  the old `<keyframe>-<grid>.world` names no longer parse and are left
  to orphan cleanup *(my call: caches are never migrated)*.
- `catalogue::evict` takes the current versions as one value
  `Engines { system, world }`: an entry is stale when its system version
  differs, or, for a world, its world version differs.
- `world::World` records both versions; the codec header writes both
  (magic `MGWORLD2`).

### Pipeline (`crates/cli/src/world.rs`)

`run(store, inputs, body, options, timelines: impl FnOnce(&Inputs) ->
Result<Vec<Timeline>, _>)`:

1. store the inputs (`SystemRepository::put`, idempotent: the id is
   the hash of the inputs);
2. timelines from `TimelineCache`, else from the injected producer, then
   put;
3. the body's timeline by designation, its present keyframe, which must
   be a planet with a surface (else a typed error);
4. `Grid::for_radius(radius, cell)` with `--cell-km` (default
   `world::DEFAULT_CELL`), the `WorldKey`, the world from `WorldCache`,
   else `world::relief::generate` into a new `World`, then put;
5. `FsStore::evict(Engines)`;
6. export to `--out`: the world file and `<field>.png` at `--width`.

The producer is injected so pull request A tests the whole pipeline on
fixture timelines (`timeline` with its `fixtures` feature as a
dev-dependency of `cli`), and pull request B passes `system::generate`.

### Export (`crates/adapters`)

- `export::write(out, world, field, width, ramp)`: the world file
  (codec) and the PNG; file names from the designation slug and field
  code.
- A relief ramp: bathymetric blues below 0 m, hypsometric greens to
  browns to white above, stops in the adapter *(colour ramps are the
  builder's, per the parent design)*.

### Tests

- Tier 1: `parse_size` (bytes, each suffix, overflow, SI refused);
  eviction with the two versions (stale system, stale world, current);
  path round trip with the world version; codec round trip `MGWORLD2`;
  #381 on a hand-built world: area-weighted mean and land share of the
  projections at two widths agree with the grid's within the projection
  error.
- Tier 3 for the two pipeline runs that generate a world (a fixture
  timeline and a small grid of a few hundred cells): first run
  generates and stores, second run hits both caches (the producer is
  not called), a budget of 0 evicts everything after export. Decided
  by the user on 2026-11-22: no test generates a world outside tier 3.
  The refusals (width, body, surface, present keyframe, producer
  error) stay in tier 1: they stop before any generation.
- Tier 3, adapters: PNG previews of the hand-built Earth, Mars and Venus
  states of `crates/world/tests/relief_heavy.rs` at 60 km, written
  under `target/relief-previews/` for a visual check before pull
  request B. The states move to a place both tests can read without
  `world` depending on more than `timeline` and `science` *(builder's
  call where)*.

## Pull request B (after the planet PR 2d)

Designed with the user on 2026-11-23. It waits on mapgen1's planet
PR 2d, then on mapgen3's `system::generate` (no stubs, user decision
of 2026-11-23); the signature is fixed:
`pub fn generate(inputs: &Inputs) -> Result<Vec<Timeline>, SystemError>`.

### Command

`map-generator world` takes the flags of `system` (`SystemArgs`) and:

| Flag | Meaning | Default |
| --- | --- | --- |
| `--body <BODY>` | The body part of a designation: `b`, `c`, ... | Required |
| `--cell-km <KM>` | The cell size | `world::DEFAULT_CELL` (15 km) |
| `--field <CODE>` | The field projected to the PNG | `relief.height` |
| `--width <PX>` | The PNG width, even and above 0 | 2048 |
| `--out <DIR>` | Where the world file and the PNG go | Required |

`GlobalArgs` (`--root`, `--cache-budget`) is flattened into `Cli`, so
both commands take them; the `dead_code` expectations of pull request A
go. The drawn-seed notice is shared by `system` and `world`: each
command's arm of `dispatch` returns the drawn seed in `Run.drawn_seed`,
and `main` prints it, also when the command then fails.

### Naming the body

`--body` takes the body part only, since the `MGS` number is drawn with
the system and is unknown before a run, and always unknown without
`--seed`. The CLI never interprets that text: it builds
`MGS <number> <body>` from the number of the generated timelines and
parses it with `Designation::from_str`, the contract's one parser. So
`b` names a planet, `A` names the star (refused, `NotAPlanet`), and the
multiple-star forms of track T7 (#585: `B b` around star B, `AB b`
around the pair) work without a CLI change once the contract learns
them. A body absent from the system is `NoSuchBody`.

`WorldRequest.body` therefore becomes that text, resolved inside
`cli::world::run` after the timelines exist.

### Output

stdout gets a JSON summary of the `WorldRun`: system id, world key,
what came from the cache, what was evicted, the files written. It
matches `system`, which prints JSON.

### Approaches for the body

1. Body part only, resolved against the drawn number (chosen): one
   command, even with a drawn seed.
2. Full designation (`MGS 12345 b`). Rejected: `system` has to run
   first to learn the number.
3. Both forms. Rejected: more parsing for little use.

### Tests of pull request B

- Tier 1: flag parsing; body resolution, the star refused and a missing
  body, through the injected producer and hand-built timelines (no
  generation).
- Tier 3: `world` end to end on the real `system::generate`.
- Once `generate` is on `main`: the oracle report and 2:1 projections
  of a few bodies, for the user's approval.

### Documentation of pull request B

`crates/cli/README.md` (the `world` command, its flags, examples), the
usage text of each flag, and this section.

## Left to the builder

- Error enums, module names, the exact ramp stops.
- Where the hand-built tier-3 states live.

## Documentation

Pull request A: `crates/cli/README.md` (global options, sizes),
`crates/world/README.md` (world engine version), `crates/catalogue/README.md`
and `crates/adapters/README.md` (key and file names, `MGWORLD2`), and the
parent design's Catalogue section where the layout changes.
