# `map-generator`: the command line

The composition root: wires the adapters into the engines' ports and
parses the command line. It has no command yet; `system` (write a
timeline as JSON, print cause trees with `--explain`) arrives with slice 1,
PR 2.

## Ports

None.

## Dependencies

Every crate.

The direction of dependencies is checked by `tools/guardrails`
(`mise run test:tier1`). See the [architecture rewrite design](../../docs/superpowers/specs/2026-11-20-architecture-rewrite-design.md),
section Architecture > Crates.
