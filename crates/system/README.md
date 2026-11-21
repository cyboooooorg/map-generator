# `system`: the system engine

Computes a system from its birth to its horizon: a hierarchical tree of
orbits, stars with phases, the planets' interiors, atmospheres and global
climate, events. Publishes one timeline per body. Owns the inputs: the
request, the archetype targets, the present date.

## Modules and owners

| Module | Owner | Content |
| --- | --- | --- |
| `inputs` | shared | `Inputs`, `Request`, `SystemRequest`, `BodyRequest` |
| `archetype` | mapgen3, mapgen1 | `ArchetypeId`; placement half (draw), targets half (planet model) |
| `placement` | shared | `Trial`, `Placement`: the seam |
| `draw` | mapgen3 | `draw(inputs, trace) -> Drawn` (signature only) |
| `star` | mapgen3 | `StarState` computation (signature only) |
| `planet` | mapgen1 | `class_of`, `model` (signatures only); `draws`, `interior`, `heat`, `codes`: the composition draws, the structure and the internal heat of a trial, crate-internal until the chain uses them (laws, sources and limits: [the science page](../../docs/science/planet-interiors.md)) |
| `engine` | shared | `generate(inputs) -> Vec<Timeline>` (signature only) |

The doc-only modules fix their future signatures in `//!` comments;
`SystemError` is created with the first of them.

## Planet interior

A trial's composition is drawn from the seed its caller passes (the body
seed for `model`, the trial seed for `class_of`): core mass fraction (split
normal around 0.32), water share (log-normal inside the ice line, uniform
0.5 to 0.67 for a water world, formed beyond it or migrated, both flags
traced), radiogenic abundance (0.23 dex). The layers are core alloy and
mantle (holding a dry planet's water), plus a water world's water layer at
the trial's equilibrium temperature, integrated at zero surface pressure
with `science::structure`. From 355 K a water world's water is an inventory
outside the structure until the climate adds its steam envelope. The
heat is radiogenic, secular and tidal (`science::heat`). Every draw and
constant carries its `Source:` or `Calibration:`; the departures and limits
(cold structure, extrapolated core, water range) are on the page
[What are planets made of?](../../docs/science/planet-interiors.md#where-it-departs-from-the-science).

## The seam

The draw places orbits and produces a `Trial` per candidate body, with
its own seed. Once every orbit is placed and the letters assigned, a
trial becomes a `Placement` (designation, body seed, spin). The planet
model runs on the placement and draws only from the body seed, never
from `trial.seed`. The orbit's period is the draw's, never recomputed.

`SystemError` (`error.rs`) is the error type of the engine, `non_exhaustive`;
it wraps `ContractError` and the host errors of `science::stellar`.

## Versions

- `INPUTS_SCHEMA` versions the shape of the stored `Inputs`. The inputs
  are the only stored truth.
- `ENGINE_VERSION` is bumped whenever a timeline or a world could change
  for the same inputs; it invalidates the caches, which are never
  migrated.

## Ports

`EnvironmentSource` (the galactic place and birth cluster) arrives with
the external events track (T6, #608).

## Dependencies

`timeline`, `science`, `serde`, `rand_chacha` and `rand_core` (the
seeded draws of `planet::draws`, always from a sub-seed of the seed the
caller passes). Never `world`.

The direction of dependencies is checked by `tools/guardrails`
(`mise run test:tier1`). See the [architecture rewrite design](../../docs/superpowers/specs/2026-11-20-architecture-rewrite-design.md),
section Architecture > System engine.
