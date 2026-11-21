# `world`: the map generator

Turns one keyframe or event state of one body into a world on the
cube-sphere simulation grid: relief, spatialized climate, seasonal
statistics, Köppen class, surface layers, named biomes. It never feeds
back into the system engine and never reads the kind of the central
object.

## What it holds

- `Grid`: the equi-angular cube sphere, cell `k = (face * n + y) * n + x`,
  eight neighbours across edges and corners, and the area of each cell.
  `Grid::for_radius(radius, DEFAULT_CELL)` sizes a face from the body's
  radius (15 km cells, capped at `MAX_CELLS`, 8 M cells); the tier-2 grid
  is coarser. Areas sum to `4 pi`: weight every statistic by them.
- `World`: a grid, the body's designation, a keyframe index, the engine
  version and named fields. `World::insert` refuses a field of the wrong
  length, a duplicate code, or a storage mode that does not fit (`u16`
  storage needs finite values); errors
  are `WorldError` codes, never text. Fields are `f32`, `u16` or `u8`;
  categorical ones (classes) are integers. Each field declares how it is
  quantised for storage. The fields themselves arrive with W2 to W4.
- `project(world, field, width)`: a plate carree `Raster` of
  `width x width/2`. Each pixel is the area-weighted mean of the cells in
  its footprint, or the weighted mode for a categorical field. Pure and
  deterministic; colours and PNG encoding live in `adapters`.

## Ports

`TimelineSource` (read a body's keyframes and events up to a date) comes
with W2; `ReliefSource` (imported relief) with track T9.

## Dependencies

`timeline`, `science`. Never `system`.

The direction of dependencies is checked by `tools/guardrails`
(`mise run test:tier1`). See the [architecture rewrite design](../../docs/superpowers/specs/2026-11-20-architecture-rewrite-design.md),
section Architecture > Map generator.
