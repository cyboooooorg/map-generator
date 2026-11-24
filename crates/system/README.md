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
| `placement` | shared | `Trial`, `Placement`, `TrialOutcome`: the seam (the outcome holds plain values, since `class_of` runs on a scratch trace) |
| `draw` | mapgen3 | `draw_with(inputs, trace, classify) -> Drawn`: the star, its system number, the family and one `Placement` per planet; a body request is `BodyRequestNotDrawn` |
| `families` | mapgen3 | `Family`: the five system architectures, untraced engine data carried by `Drawn`; `families::table` holds every count range |
| `spin` | mapgen3 | the primordial spin of a planet (period, obliquity) |
| `stand_in` | mapgen3 | `classify`: a provisional classifier for `draw_with` (mass steps, one radius law), until `class_of` is public; used by the CLI and the tests |
| `star` | mapgen3 | `star(...) -> Star`: the asked or drawn host, its values and its interior, traced |
| `planet` | mapgen1 | `class_of`, `model` (signatures only); `draws`, `envelope`, `enveloped`, `interior`, `heat`, `escape`, `water`, `regime`, `core`, `air`, `bare`, `climate`, `settled`, `codes`: `bare::bare` chains a trial in legacy's order from its draws, structure (rocky, mini-Neptune, giant), envelope escape and heat to, once the envelope is gone, its water, regime, core and bare air, which runs to the photochemistry (outgassed, shoreline, methane photolysis, reducing air, cold trap, photochemistry); `climate` is the climate solver (the body it settles, its albedo, the carbonate thermostat, the state branches, the snowball chance, `record` for its causes) and `settled::settled` chains it after `bare` with the reducing-air fallback (the air made again with its CO₂ when the climate leaves no Titan-like air) and the boil loop (a wet planet with no wet balance boils: run again from the forced steam, its leftover condensed again when that run does not boil); `settled` is not wired into generation until 2d-4 (#659), crate-internal until 2d (laws, sources and limits: [interiors](../../docs/science/planet-interiors.md), [atmospheres](../../docs/science/planet-atmospheres.md#the-chain-and-the-surface-classes)) |
| `engine` | shared | `generate(inputs) -> Vec<Timeline>` (signature only) |

The remaining doc-only module fixes its future signature in a `//!` comment.
`SystemError`, the engine's error type, lives in `src/error.rs`.

## Planet draw

`draw_with` draws the star, then the family on `seed.derive("family")`
(compact, solar-like, hot Jupiter, eccentric giants, sparse, evenly), then
the planets, sorted by distance (`b` the closest). `classify` is a
parameter (the planet model's `class_of`, a closure in tests): it returns
a `TrialOutcome` of plain values (class, radius, mean density), since it
runs on a scratch trace.

- **Slots.** Asked archetypes first, never moved, then the family's slots.
  Each reads its own stream: `trial/<id>` (copy `k` of an asked archetype:
  `trial/<id>/<k>`), `trial/slot-<n>` for family slot `n`. A try proposes a
  mass and an axis, classifies a circular trial, draws the eccentricity
  from the class, and checks the star, the Roche floor, the spacing to
  every placed planet and the hot Jupiters' cleared zone. 20 tries per
  slot: a drawn slot stays empty, an asked one fails with `NoRoom`. A
  compact try past 1071 d ends the chain: its slot and the later ones
  stay empty, with no retry. More
  than 649 planets is `TooManyPlanets`, before any allocation.
- **Streams.** The perihelion longitude draws from `orbit/<trial label>`;
  the body seed is `derive(<designation text>)`, the spin
  `body_seed.derive("spin")`; the system number comes from
  `derive("designation")`. Adding a draw never shifts another.
- **Constants.** Count ranges and chances live in `families/table.rs`, the
  sourced laws in `families/laws.rs`; each has a `Source:`,
  `Calibration:` or `Definition:`. Orbit laws (Kepler, ice line
  `2.7 au x M / Msun`, Roche floor, mutual Hill spacing, `K50`) are in
  `science::orbit`.
- **Spin.** Rocky: Kokubo & Genda's Gaussian spin vector; giants
  (ice giants too): a log-uniform fraction of break-up, 0.07 to 0.40;
  obliquity isotropic. Birth values: tidal locking is track T2 (#604).
- **Cause codes** start with `system.draw.`.

The sources, the choices that are the draw's own and where it departs from
the science are in [the science page](../../docs/science/system-architectures.md).

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
are core alloy and mantle (holding a dry planet's water, 0.085 of which
`planet::water` puts at its surface), plus a water world's water layer
(all at its surface) at the trial's equilibrium temperature, integrated at
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
- `ENGINE_VERSION` (3 since compact chains end at 1071 d) is bumped
  whenever a timeline or a world could change for the same inputs; it
  invalidates the caches, which are never migrated.

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
`SystemError`; a NaN input is refused with `system.not-a-number:
<input>` (`present`, `host-mass` or `metallicity`), and an infinite one
with the range errors. Nodes: `system.star.{mass, metallicity, age}` (leaves,
`Input` or `Drawn`), `lifetime-end`, `radius`, `luminosity`,
`effective-temperature` and `interior` (derived from the leaves), and per
layer `system.star.layer-N.{outer-radius, mass, material, state,
transport}` (a segment cannot start with a digit). Layers are plasma, in
metres and kilograms (the layer's own mass; a thin surface layer can weigh
0 kg). The laws and sources are in `science::stellar`.
