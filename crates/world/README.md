# `world`: the map generator

Turns one keyframe or event state of one body into a world on the
cube-sphere simulation grid: relief, spatialized climate, seasonal
statistics, Köppen class, surface layers, named biomes. It never feeds
back into the system engine and never reads the kind of the central
object.

## Ports

`TimelineSource` (read a body's keyframes and events up to a date) arrives
with slice 1, PR 4; `ReliefSource` (imported relief) with track T9.

## Dependencies

`timeline`, `science`. Never `system`.

The direction of dependencies is checked by `tools/guardrails`
(`mise run test:tier1`). See the [architecture rewrite design](../../docs/superpowers/specs/2026-11-20-architecture-rewrite-design.md),
section Architecture > Map generator.
