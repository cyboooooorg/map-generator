# `science`: laws and typed units

Laws of physics and typed units, shared by both engines: radiative balance,
escape, interiors, orbital mechanics, stellar evolution, and so on.
Functions take and return typed quantities. Anything that knows a body, a
system, a seed or an identifier lives in an engine, and nothing here draws
at random.

Every constant has a doc comment starting with `Source:` (a citation),
`Calibration:` (what it was fitted against, and how) or `Definition:` (an
exact definition or a unit conversion). A test fails otherwise. Physics is
ported from the tag `legacy-final` with its citation and its tests, never
rewritten from memory.

## Ports

None: `science` is a library of pure functions. The laws arrive with the
pull requests of slice 1 that use them (#602).

## Dependencies

None outside the standard library. Every other crate may depend on it.

The direction of dependencies is checked by `tools/guardrails`
(`mise run test:tier1`). See the [architecture rewrite design](../../docs/superpowers/specs/2026-11-20-architecture-rewrite-design.md),
section Architecture > Science.
