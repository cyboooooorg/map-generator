# `catalogue`: inputs and caches

Stores the inputs of each system, the only source of truth, and caches
everything regenerated from them (timelines, worlds, projections) with
the eviction policy. A cache is invalidated by the engine version, never
migrated; only the inputs carry a schema version.

## Ports

`SystemRepository`, `TimelineCache` and `WorldCache` arrive with slice 1,
PR 4; `SystemIndex` (the SQLite read model) with the server, track T10.

## Dependencies

`system` (the inputs type), `timeline`.

The direction of dependencies is checked by `tools/guardrails`
(`mise run test:tier1`). See the [architecture rewrite design](../../docs/superpowers/specs/2026-11-20-architecture-rewrite-design.md),
section Architecture > Catalogue and storage.
