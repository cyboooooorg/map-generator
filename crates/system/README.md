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
| `star` | mapgen3 | `star(...) -> Star`: the asked or drawn host, its values and its interior, traced (crate-internal until the draw uses it) |
| `planet` | mapgen1 | `class_of`, `model` (signatures only); `draws`, `envelope`, `enveloped`, `interior`, `heat`, `codes`: the composition and envelope draws, the structure (rocky, mini-Neptune, giant) and the internal heat of a trial, crate-internal until the chain uses them (laws, sources and limits: [the science page](../../docs/science/planet-interiors.md)) |
| `engine` | shared | `generate(inputs) -> Vec<Timeline>` (signature only) |

The remaining doc-only modules fix their future signatures in `//!` comments.
`SystemError`, the engine's error type, lives in `src/error.rs`.

## Planet interior

A trial's composition is drawn from the seed its caller passes (the body
seed for `model`, the trial seed for `class_of`), each value from its own
sub-seed and uniform between published bounds: core mass fraction 0 to 0.8,
water log-uniform 1e-5 to 1e-2 inside the ice line and uniform
0.1 to 0.67 beyond it or when migrated (chance 0.1, calibrated), radiogenic
abundance ±0.5 dex. `planet::envelope` draws the hydrogen-helium share by
mass (none under 0.1 M⊕, Lammer et al. 2014 bounds up to 5 M⊕, capped at
0.20 under 20 M⊕, then a giant's heavy-element mass around the
Thorngren et al. 2016 fit, inside the cores of the Fortney et al. 2007
tables).

`planet::interior` dispatches on the envelope. Without one, the layers
are core alloy and mantle (holding a dry planet's water), plus a water
world's water layer at the trial's equilibrium temperature, integrated at
zero surface pressure with `science::structure`. A mini-Neptune is that
core (mass `(1 - f) M`) plus the Lopez & Fortney 2014 thickness
(`science::envelope`), under one gas layer. A giant (from 20 M⊕) takes its
radius, core radius and internal luminosity from the Fortney tables
(`science::giants`, no inflation), with a metallic hydrogen layer only
above 50 M⊕ and a core under 25 % of its mass (`science::hydrogen`). A
water world's water beyond the law of `science::water` (355 K) stays an
inventory: the bulk is the condensed planet, until the climate (2d) adds
its steam. The heat is radiogenic, secular and tidal (`science::heat`);
a giant's secular heat is its table luminosity, its radiogenic heat zero.

Every draw and constant carries its `Source:` or `Calibration:`. The laws,
bounds, known jumps and limits are on the page
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

## Star

`star(host_mass, metallicity, present, seed, trace)` takes what is asked
and draws the rest from `seed.derive("star")`, on one ChaCha8 stream, in
this order: class, mass, metallicity, date. The class (M, K, G, F, A, B)
is one chance in six each, only a bucket: the mass is log-uniform inside
the class (`CLASS_BOUNDS`, Pecaut & Mamajek 2013), and the temperature
comes from the stellar models. The metallicity is uniform from -1 to
+0.5 dex; the date uniform from the ZAMS to the end of the main sequence,
capped at 13.8 Gyr. The class is not traced (the trace has no node type
for an index).

A mass or metallicity outside the tables, a date before the ZAMS or after
the main sequence (or 13.8 Gyr) is refused with the matching
`SystemError`. Nodes: `system.star.{mass, metallicity, age}` (leaves,
`Input` or `Drawn`), `lifetime-end`, `radius`, `luminosity`,
`effective-temperature` and `interior` (derived from the leaves), and per
layer `system.star.layer-N.{outer-radius, mass, material, state,
transport}` (a segment cannot start with a digit). Layers are plasma, in
metres and kilograms (the layer's own mass; a thin surface layer can weigh
0 kg). The laws and sources are in `science::stellar`.
