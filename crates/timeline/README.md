# `timeline`: the contract between the engines

The published language shared by the system engine and the map
generator: per body, keyframes (the full state at a date) and dated
events with the state before and after, each with its cause trees. Data
types only, with validation.

A keyframe carries physical forcings and states (fluxes per stellar
source, orbit, spin, interior, climate, atmosphere, surface liquid), never
the kind of the central object.

## Ports

None: the map generator reads timelines through its own `TimelineSource`
port. The types arrive with slice 1, PR 2.

## Dependencies

`science` (units only).

The direction of dependencies is checked by `tools/guardrails`
(`mise run test:tier1`). See the [architecture rewrite design](../../docs/superpowers/specs/2026-11-20-architecture-rewrite-design.md),
section Architecture > Timeline (the contract).
