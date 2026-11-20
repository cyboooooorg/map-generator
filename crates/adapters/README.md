# `adapters`: the outside world

Implementations of the domain ports: filesystem storage with atomic
staging, PNG projections, SVG cross-sections, the English text catalogue
of codes and cause trees, relief files. The only library crate allowed to
read and write files.

## Ports

None of its own: it implements the ports of the domain crates.

## Dependencies

Any domain crate. No domain crate may depend on it.

The direction of dependencies is checked by `tools/guardrails`
(`mise run test:tier1`). See the [architecture rewrite design](../../docs/superpowers/specs/2026-11-20-architecture-rewrite-design.md),
section Architecture > Crates.
