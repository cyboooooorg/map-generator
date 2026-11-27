# `timeline`: the contract between the engines

The published language shared by the system engine and the map
generator: per body, keyframes (the full state at a date) and dated
events with the state before and after, each with its cause trees. Data
types only, with validation.

A keyframe carries physical forcings and states (fluxes per stellar
source, orbit, spin, interior, climate, atmosphere, surface liquid), never
the kind of the central object.

## Types

- `Seed`, `Designation`: the seed hierarchy and the catalogue name of a
  body.
- `Trace`, `Traced<T>`, `CauseGraph`: the causes (see below).
- `StellarSources`: one to three stars as a planet sees them (flux,
  equilibrium temperature, star parameters, and per band of high-energy
  light, XUV and Lyman α, a `BandLight`: present flux in W/m² and fluence
  since the star's birth in J/m²; `band_total` sums them).
- `StarState`, `PlanetState` (and their parts: `Orbit`, `Spin`, `Bulk`,
  `Composition`, `Heat`, `Surface`, `Atmosphere`, `GlobalClimate`,
  `SurfaceLiquid`, `Terrain`, `Biosphere`): the state of a body at a date.
  A giant has no `Surface`. An `Atmosphere` holds its surface pressure and
  a weight per `Gas` (N₂, CO₂, H₂O, CH₄, NH₃, SO₂, O₂, Ar; the gas's mass
  times the gravity over the planet's area, in bar, in the field
  `partial`), whose sum is the surface pressure. A gas's partial pressure
  depends on the whole air: a reader converts it
  (`science::radiation::gases::Weights::partial_pressures`, issue #749).
- `InteriorProfile`: layers with their stored mass, and radial samples.
- `Keyframe`: a date, a `BodyState` and the cause graph of its fields.
- `Timeline`: the keyframes of one body, with its seed, the
  `EngineVersion` that made it and the present date.

Enums are `#[non_exhaustive]`, serialise in kebab-case and report their
variant as a code to the cause graph.

## The cause rule

- Only a `Trace` makes a `Traced<T>`: the code that computes a value
  records its node (`leaf` for an input, `record` for a derived value
  with its sources). Nothing recomputes a value to explain it.
- One trace per system. Each keyframe freezes its own subgraph:
  `Keyframe::new` keeps the nodes its state holds and their sources,
  renumbers them from 0 and rewrites the ids in the state.
- A keyframe that came through deserialisation is checked with
  `Keyframe::validate`: valid graph, valid state, every id present.

## The seed rule

`fnv1a64` is public: the catalogue hashes the inputs of a system with it
to make its id, so its output never changes.

Nothing draws outside the seed hierarchy: a child seed is derived from
its parent by label (`Seed::derive`). A timeline stores its seed and
engine version; both are cache keys, nothing is migrated.

## Validation

`Keyframe::new` and `Timeline::new` return a `ContractError` (a code
with parameters, never text): eccentricity below 1, interior layers
strictly increasing in radius, finite numbers, at least one keyframe,
dates strictly increasing, one kind of body per timeline.

## Features

`fixtures`: structural fixtures (`timeline::fixtures`, for example
`earth_like_fixture` and `leaf`) for the tests of the crates that consume
the contract. Enable it in `dev-dependencies` or a test crate, as `world`
does: a production build does not need it.

## Left to later tracks

The fields not in the contract yet are listed in the table of the
[rewrite contract design](../../docs/superpowers/specs/2026-11-21-rewrite-contract-design.md),
section Fields left to later tracks.

## Ports

None: the map generator reads timelines through its own `TimelineSource`
port.

## Dependencies

`science` (units only).

The direction of dependencies is checked by `tools/guardrails`
(`mise run test:tier1`). See the [architecture rewrite design](../../docs/superpowers/specs/2026-11-20-architecture-rewrite-design.md),
section Architecture > Timeline (the contract).
