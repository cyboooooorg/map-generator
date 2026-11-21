# `regression`: tier-2 tests

The tier-2 regression suite (see `CLAUDE.md`, Tests): a few reference
systems generated once per run on a coarse grid, then statistics with
tolerances, contracts and determinism asserted on them. No other test
generates a system or a world of its own outside tier 3.

Test-only: it may depend on every engine, which a domain crate may not,
even as a dev-dependency. Budget: 8 minutes on CI.

`tests/determinism.rs` pins the digests of the reference systems, one
table per platform (libm results may differ by an ulp). A platform
without its table yet (Linux, until pinned from CI) prints its digests
and passes with "linux digests not pinned yet"; the tier-2 profile shows
that output on success.

## Ports

None.

## Dependencies

Every engine and `adapters`.

The direction of dependencies is checked by `tools/guardrails`
(`mise run test:tier1`). See the [architecture rewrite design](../../docs/superpowers/specs/2026-11-20-architecture-rewrite-design.md),
section Guardrails > Tests.
