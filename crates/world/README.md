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
- `World`: a grid, the body's designation, a keyframe index, the system
  engine version, the world engine version (`ENGINE_VERSION`, see
  below) and named fields. `World::insert` refuses a field of the wrong
  length, a duplicate code, or a storage mode that does not fit (`u16`
  storage needs finite values); errors are `WorldError` codes, never
  text. Fields are `f32`, `u16` or `u8`;
  categorical ones (classes) are integers. Each field declares how it is
  quantised for storage. The fields themselves arrive with W2 to W4.
- `project(world, field, width)`: a plate carree `Raster` of
  `width x width/2`. Each pixel is the area-weighted mean of the cells in
  its footprint, or the weighted mode for a categorical field. Pure and
  deterministic; colours and PNG encoding live in `adapters`.
- `relief::generate(state, seed, grid)`: the height field `relief.height`
  of one body, in metres **above sea level** (see below).

## Engine version

`ENGINE_VERSION` is the world engine version, stored in every world and in
its cache key. Bump it with any change to what the map generator produces
(a field, an algorithm, a constant, a quantisation). A change to the system
engine does not need it: that has its own version, and a cached world is
current only when both match.

## Relief

`relief::generate` builds the height of every cell from the physical
forcings of a `PlanetState` (radius, surface gravity, tectonic regime,
heat flux, liquid, ocean share) and a body `Seed`; it never reads what the
central object is. Passes, in order, each drawing from its own label of
the body seed (no shared stream):

1. `continents` (`relief.continents`): fractal noise of the analogue's
   Hurst exponent, the crust line (area quantile of the thick-crust
   share), Airy's step between the crusts, the continental slope.
2. Plates (`relief.plates`, plate worlds only): `PlateMap::draw` and
   `PlateMap::relief` add trenches, arcs, rifts and ridges by boundary
   strength levels; `raise_orogens` lifts continental columns to the
   Andean plateau's or the collision range's elevation. Without plates,
   `highlands` (`relief.mountains` / `highlands`) takes their place.
3. `mountains` (`relief.mountains`): ridged noise weighted by the
   orogeny, bounded at `MOUNTAIN_RELIEF`.
4. `volcanoes` (`relief.volcanoes`): cones from the heat flux, and along
   the arcs.
5. Sea level: the area quantile that leaves `ocean_share` of the sphere
   below it (the area mean on a world without liquid) is subtracted, so
   0 m is the sea.

The field is stored as `relief.height`, `u16` over plus or minus 25 km
(`HEIGHT_RANGE`), a step under a metre. The same seed gives the same
relief. The tier-3 census is `crates/world/tests/relief_heavy.rs`
(`mise run test:heavy`): it asserts the land share, the sea floor over
1 km, Venus within 1 km, no saturation, Earth's H and determinism, and
prints the rest of Earth's hypsometry and Mars's highland share and H,
which W2 does not model. The laws and the departures are in
[mountains and gravity](../../docs/science/mountains-and-gravity.md) and
[plate tectonics](../../docs/science/plate-tectonics.md).

## Ports

`TimelineSource` (read a body's keyframes and events up to a date) comes
with W2; `ReliefSource` (imported relief) with track T9.

## Dependencies

`timeline`, `science`. Never `system`.

The direction of dependencies is checked by `tools/guardrails`
(`mise run test:tier1`). See the [architecture rewrite design](../../docs/superpowers/specs/2026-11-20-architecture-rewrite-design.md),
section Architecture > Map generator.
