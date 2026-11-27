# `map-generator`: the command line

The composition root: wires the adapters into the engines' ports and
parses the command line. One command, `system`, so far. Run it from the
workspace with `cargo run -q -p map-generator -- system …`, or install the
binary with `cargo install --path crates/cli` and call `map-generator`:

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

stdout is one pretty-printed JSON document (`schema` 1):

- `inputs`: the draw's inputs as given, to rerun it;
- `engine`, `family`: the engine version and the family of the system;
- `star`: the star's timeline, one keyframe at the present. The keyframe
  has its own `causes` graph, used by the `/star/...` pointers;
- `planets`: the planets in the draw's order (closest first), each with its
  designation, seed, archetype, mass, orbit, spin and stellar sources, and
  its `state`: the planet model's `PlanetState` (class, climate, liquid,
  biosphere...) on the drawn orbit. The draw itself classifies each
  planet with `planet::class_of` (its class is in `causes`, as
  `system.draw.class`); `state` runs the chain a second time per planet,
  on the drawn, possibly eccentric, orbit, so for an eccentric planet its
  class may differ from the draw's;
- `causes`: the cause graph of `planets` only (not of the star); the
  traced values of the planets hold ids of its nodes.

## Explaining a value

`--explain PATH` prints the cause tree of one drawn value as text, not JSON.
`PATH` is either a JSON pointer into the output, or a cause code:

```bash
map-generator system --seed 1 --explain /planets/0/orbit/period
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

## World pipeline

The crate holds the world pipeline (`src/world.rs`): it takes a stored
system, builds or reads the cached timeline and world, and exports a world
file and a PNG. Its store options arrive with the `world` command in the
next pull request:

| Flag | Meaning |
| --- | --- |
| `--root PATH` | The store's root directory. Default `map-generator-store` |
| `--cache-budget SIZE` | The cache budget in bytes, with `KiB`, `MiB`, `GiB` or `TiB`. Default 2 GiB. SI suffixes (`MB`) are refused |

The pipeline refuses a PNG width that is odd or zero (`cli.world.width`)
before touching the store, and stamps the world with the body timeline's
engine version.

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Success (also when the reader closes stdout early) |
| 1 | The command failed; stderr is `map-generator: <code>: <message>` |
| 2 | Usage error (bad flag or malformed `ID=COUNT`) |

A stdout write error other than a closed pipe exits 1 with `cli.io`.

## Ports

None.

## Dependencies

Every crate.

The direction of dependencies is checked by `tools/guardrails`
(`mise run test:tier1`). See the [architecture rewrite design](../../docs/superpowers/specs/2026-11-20-architecture-rewrite-design.md),
section Architecture > Crates.
