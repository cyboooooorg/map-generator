# `map-generator`: the command line

The composition root: wires the adapters into the engines' ports and
parses the command line. Two commands: `system` draws a planetary system,
`world` maps one of its planets (see [The `world` command](#the-world-command)).
Run them from the workspace with `cargo run -q -p map-generator -- system …`,
or install the binary with `cargo install --path crates/cli` and call
`map-generator`:

```bash
map-generator system [--seed N] [--present GYR] [--host-mass M]
    [--metallicity DEX] [--archetype ID=COUNT ...] [--explain PATH]
```

| Flag | Meaning |
| --- | --- |
| `--seed N` | The root seed of the run. Omitted: drawn from the OS |
| `--present GYR` | The star's age in Gyr. Omitted: drawn |
| `--host-mass M` | The host star's mass in solar masses, 0.08-8. Omitted: drawn |
| `--metallicity DEX` | The host star's metallicity. Omitted: drawn |
| `--archetype ID=COUNT` | Ask for planets of an archetype; repeatable, counts add up |
| `--explain PATH` | Explain one value of the draw: a JSON pointer or a cause code (see below) |

Without `--seed`, the root seed is drawn from the OS, the only random
source of the program. It is printed on stderr
(`map-generator: seed N drawn; rerun with --seed N`) and is in the JSON's
`inputs.seed`: rerun with `--seed N` and the same other flags to get the
same bytes.

## Output

stdout is one pretty-printed JSON document (`schema` 3: schema 2 had a
`star` timeline and each planet's `state` under `planets`, with one cause
graph for all the planets; schema 1 had a `classifier` field):

- `inputs`: the draw's inputs as given, to rerun it;
- `engine`, `family`: the engine version and the family of the system;
- `timelines`: `system::generate`'s output, one timeline per body: the
  star's first, then one per planet in the draw's order (closest first),
  each with its body seed and one keyframe at the present. A planet's
  keyframe holds the planet model's `PlanetState` (orbit, spin, stellar
  sources, class, climate, liquid, biosphere...) on the drawn orbit, under
  `state/planet`; the star's holds its state under `state/star`. Each
  keyframe has its own `causes` graph, used by the pointers into it. The
  draw classifies each planet with `planet::class_of` (its class is
  `system.draw.class`); the state runs the chain a second time per planet,
  on the drawn, possibly eccentric, orbit, so for an eccentric planet its
  class may differ from the draw's;
- `planets`: what the draw decided of each planet beyond its state, in
  `timelines[1..]`'s order: its designation, archetype and drawn mass;
- `causes`: the cause graph of `planets` (the drawn masses) only.

## Explaining a value

`--explain PATH` prints the cause tree of one drawn value as text, not JSON.
`PATH` is either a JSON pointer into the output, or a cause code:

```bash
map-generator system --seed 1 \
  --explain /timelines/1/keyframes/0/state/planet/orbit/period
```

```text
system.draw.period = 5.350512601505564 d
  system.draw.semi-major-axis = 0.10157348219710759 au [drawn]
  system.star.mass = 4.883678834209888 Msun [drawn]
```

One line per node, indented by depth. A leaf ends with its provenance
(`input`, `observed`, `drawn`, `imposed`); a derived node is followed by the
nodes it comes from. A node already printed in the tree appears as
`code (see above)`. Dimensionless values print without a unit. A value
with `|x| >= 1e6` or `0 < |x| < 1e-3` prints in scientific notation with
5 significant digits (`1.1447e30 kg`); the JSON output keeps full precision.

A code several values share (the flux of each planet) is ambiguous: the
error lists the pointers to choose from. A code no field holds itself, only
values that depend on it (`system.draw.break-up-fraction`), is reported as
"held by no field" and lists those values instead.

| Error | Meaning |
| --- | --- |
| `cli.explain.not-found` | No value at the pointer, or no node with the code |
| `cli.explain.not-traced` | The pointer leads to a value with no cause tree (a designation, say) |
| `cli.explain.ambiguous` | Several values share the code; the message lists their pointers |

## The `world` command

`world` draws a system with the same flags as `system`, builds the world
of one of its planets at its present (the relief so far), and writes it
to a directory: the world file and an equirectangular PNG of one field.
Run it to look at a planet, or to fill the store's cache.

```bash
map-generator [--root PATH] [--cache-budget SIZE] world --body BODY --out DIR
    [--seed N] [--present GYR] [--host-mass M] [--metallicity DEX]
    [--archetype ID=COUNT ...] [--cell-km KM] [--field CODE] [--width PIXELS]
```

| Flag | Meaning |
| --- | --- |
| `--body BODY` | Required. The last part of the body's designation: `b`, `c`… for the planets in order from the star, `A` for the star |
| `--out DIR` | Required. Where the world file and the PNG are written |
| `--cell-km KM` | The grid's cell size in km, finite and above 0. Default 15 |
| `--field CODE` | The field to project. Default `relief.height` |
| `--width PIXELS` | The PNG width, even and above 0; the height is half. Default 2048 |
| `--root PATH` | The store's root directory. Default `map-generator-store` |
| `--cache-budget SIZE` | The cache budget in bytes, with `KiB`, `MiB`, `GiB` or `TiB`. Default 2 GiB. SI suffixes (`MB`) are refused |

The `system` flags (`--seed` and the others) mean what they mean for
`system`, and a drawn seed is announced on stderr the same way.
`--root` and `--cache-budget` belong to every command: they go before or
after the command's name.

`--body` takes the body part only: the command builds the full
designation (`MGS 99401 c`) from the drawn system's number. A text that
is not a body part is refused before the system is drawn
(`cli.world.invalid-body`); the star `A` is refused once it is drawn
(`cli.world.not-a-planet`), and so is a planet without a solid surface
(`relief.no-surface`). Multiple-star systems (#585) will name their
stars `A`, `B`… with no change to the flag.

The first run stores the inputs, then caches the timelines and the
world; a second run with the same flags reads both from the cache and
writes the same bytes. After each run, the eviction deletes every cache
entry of another engine version (a cache is never migrated), then the
least recently used ones until the cache fits the budget.

It prints a JSON summary on stdout:

| Field | Meaning |
| --- | --- |
| `system` | The system's id in the store (16 hex digits) |
| `body` | The full designation |
| `keyframe` | The index of the present keyframe in the body's timeline |
| `grid` | The grid's subdivision count |
| `engine`, `world_engine` | The system engine and map generator versions |
| `timelines_cached`, `world_cached` | Whether each came from the cache |
| `evicted` | How many cache entries the eviction removed |
| `world`, `png` | The files written |

Map planet c of seed 15, which has surface water with engine 4:

```bash
map-generator world --seed 15 --body c --out out
```

The same with a smaller cache, given before the command:

```bash
map-generator --cache-budget 512MiB world --seed 15 --body c --out out
```

The `world` command fails with exit 1 and one of these codes, after
those of the draw (`system.*`), the relief (`relief.*`) and the export:

| Code | Meaning |
| --- | --- |
| `cli.world.invalid-body` | `--body` is not a body part (`b`, `c`…, `A`); refused before the draw |
| `cli.world.no-such-body` | The system has no such body |
| `cli.world.not-a-planet` | The body is the star |
| `cli.world.no-present-keyframe` | The body's timeline has no state at its present |
| `cli.world.width` | The width is odd or zero (from the library; the flag refuses it first) |
| `cli.store` | The store could not be read or written |

A wrong `--field` that is well formed is refused by the export, after the
world is built.

The world is stamped with the body timeline's engine version and the map
generator's, so a new engine never reads an old cache entry.

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Success (also when the reader closes stdout early) |
| 1 | The command failed; stderr is `map-generator: <code>: <message>` |
| 2 | Usage error: a bad flag, a malformed `ID=COUNT`, a `--cell-km` at or below 0 or not finite, an odd or zero `--width`, a malformed `--field` |

A stdout write error other than a closed pipe exits 1 with `cli.io`.

## Ports

None.

## Dependencies

Every crate.

The direction of dependencies is checked by `tools/guardrails`
(`mise run test:tier1`). See the [architecture rewrite design](../../docs/superpowers/specs/2026-11-20-architecture-rewrite-design.md),
section Architecture > Crates.
