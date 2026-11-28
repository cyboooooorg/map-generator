# `regression`: tier-2 tests

The tier-2 regression suite (see `CLAUDE.md`, Tests): a few reference
systems generated once per run on a coarse grid, then statistics with
tolerances, contracts and determinism asserted on them. No other test
generates a system or a world of its own outside tier 3.

Test-only: it may depend on every engine, which a domain crate may not,
even as a dev-dependency. Budget: 8 minutes on CI.

`tests/planets.rs` holds one test, `the_reference_planets`, whose named
sub-checks share one generation of the references (`system::generate_with`;
nextest runs each test in its own process, and each process that calls
`references()` draws and models the eight systems again): the pinned
digests of the reference systems and the purity of the generation
(reference 1 generated again with `class_of`, same digest and
timelines; a failure names the first differing body and field, as a
JSON pointer), `generate`'s timelines as `world::run` reads them (one per
body, star first, body seeds, one keyframe at the present), every
placement modelled with a
valid state, unique trial seeds, reference 7's asked `earth-like` placed
with a surface (the archetype's promise), the obliquity `class_of` read
equal to the placement's, the class changes between draw and model and
the planets' class, liquid, biosphere and temperature pinned, the
living planets' stage, and the boil-off leaving references 6 and 8's
stripped planets bare. The digests were pinned on macOS; macOS and
Linux gave the same values for the earlier stand-in digests. Split the
table per platform if their libm ever differs by an ulp (a threshold of
the draw would flip).

`tests/contracts.rs` checks the reference seeds, a pinned system id and
that the workspace builds.

`tests/cli.rs` runs the `map-generator system` command through the
library (`report`, `explain`, `dispatch`) on reference 1's inputs only:
the report's shape and `--explain`. Reference 1 is giants only, so its
draw solves no climate and stays cheap: each test generates it (about 0.1 s, #785)
rather than read `references()`, which would cost each test's process
the eight systems.

## Ports

None.

## Dependencies

Every engine, `adapters`, and `map-generator` (the CLI's library) for
`tests/cli.rs`.

The direction of dependencies is checked by `tools/guardrails`
(`mise run test:tier1`). See the [architecture rewrite design](../../docs/superpowers/specs/2026-11-20-architecture-rewrite-design.md),
section Guardrails > Tests.
