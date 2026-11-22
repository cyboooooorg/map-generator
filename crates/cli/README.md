# `map-generator`: the command line

The composition root: wires the adapters into the engines' ports and
parses the command line. One command, `system`, so far:

```bash
map-generator system [--seed N] [--present GYR] [--host-mass M]
    [--metallicity DEX] [--archetype ID=COUNT ...] [--explain PATH]
```

Without `--seed`, the root seed is drawn from the OS and printed on stderr
so the run can be replayed. The flags are parsed and turned into the draw's
inputs; the JSON output and `--explain` arrive with the next commits of the
same pull request. A usage error exits 2, a failed command 1.

## Ports

None.

## Dependencies

Every crate.

The direction of dependencies is checked by `tools/guardrails`
(`mise run test:tier1`). See the [architecture rewrite design](../../docs/superpowers/specs/2026-11-20-architecture-rewrite-design.md),
section Architecture > Crates.
