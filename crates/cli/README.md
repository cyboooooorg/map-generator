# `map-generator`: the command line

The composition root: wires the adapters into the engines' ports and
parses the command line. One command, `system`, so far:

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
| `--explain PATH` | Explain one value of the draw (not available yet) |

Without `--seed`, the root seed is drawn from the OS, the only random
source of the program. It is printed on stderr
(`map-generator: seed N drawn; rerun with --seed N`) and is in the JSON's
`inputs.seed`: rerun with `--seed N` and the same other flags to get the
same bytes.

## Output

stdout is one pretty-printed JSON document (`schema` 1):

- `inputs`: the draw's inputs as given, to rerun it;
- `engine`, `family`: the engine version and the family of the system;
- `classifier`: `"stand-in"`. The planet classes are not printed yet; the
  draw uses a stand-in classifier until the planet engine's `class_of` is
  public (see the [system crate](../system/README.md));
- `star`: the star's timeline, one keyframe at the present;
- `planets`: the planets in the draw's order (closest first), each with its
  designation, seed, archetype, mass, orbit, spin and stellar sources;
- `causes`: the cause graph of `planets`; the traced values hold ids of its
  nodes.

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Success |
| 1 | The command failed; stderr is `map-generator: <code>: <message>` |
| 2 | Usage error (bad flag or malformed `ID=COUNT`) |

## Ports

None.

## Dependencies

Every crate.

The direction of dependencies is checked by `tools/guardrails`
(`mise run test:tier1`). See the [architecture rewrite design](../../docs/superpowers/specs/2026-11-20-architecture-rewrite-design.md),
section Architecture > Crates.
