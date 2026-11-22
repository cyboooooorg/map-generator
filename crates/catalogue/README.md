# `catalogue`: inputs and caches

Stores the inputs of each system, the only source of truth, and caches
everything regenerated from them (timelines, worlds, projections) with
the eviction policy. A cache is invalidated by the engine version, never
migrated; only the inputs carry a schema version.

## Identity

`SystemId::of(&inputs)` is the FNV-1a 64 hash (`timeline::fnv1a64`) of the
inputs' JSON, shown as 16 hex digits. Equal inputs give the same id, so
a regeneration finds its caches. Inputs are hashed as given, defaults
unresolved: `present: None` (draw the age) differs from a fixed date. A
system's MGS number is not a key (it can collide). `StoredSystem` holds
the id, the inputs, the engine version that first generated them and an
optional name; a repository refuses an unknown inputs schema
(`CatalogueError::UnknownSchema`) and a record whose id is not the hash
of its inputs (`CatalogueError::IdMismatch`), on `put` and on `get`.

## Ports

- `SystemRepository`: `put`, `get`, `list`, `remove` of stored systems.
- `TimelineCache`: the timelines of a system for an engine version.
- `WorldCache`: a world by `WorldKey` (system, body, keyframe index, grid
  `n` after the cap, system engine version, world engine version). It
  has an associated `World` type because this crate cannot name
  `world::World`; the adapter fixes it.

`SystemIndex` (the SQLite read model) comes with the server, track T10.

## Invalidation and eviction

A cache entry of another engine version is a miss and is dropped at the
next eviction; nothing is migrated. `evict(entries, current, budget)`
(`current` is `Engines`: the system and the world engine versions; a
world is current when both match) is pure: it returns the keys to
delete, stale engine versions first, then the least recently used (by a
logical tick, never the wall clock) until the total fits `budget` bytes.
`DEFAULT_BUDGET` is 2 GiB; the store takes the budget at
construction and the CLI will expose it with W2.

## Dependencies

`system` (the inputs type), `timeline`.

The direction of dependencies is checked by `tools/guardrails`
(`mise run test:tier1`). See the [architecture rewrite design](../../docs/superpowers/specs/2026-11-20-architecture-rewrite-design.md),
section Architecture > Catalogue and storage.
