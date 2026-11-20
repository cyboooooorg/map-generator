# `regression`: tier-2 tests

The tier-2 regression suite (see `CLAUDE.md`, Tests): a few reference
systems generated once per run on a coarse grid, then statistics with
tolerances, contracts and determinism asserted on them. No other test
generates a system or a world of its own outside tier 3.

Test-only: it may depend on every engine, which a domain crate may not,
even as a dev-dependency. Budget: 8 minutes on CI.

## Ports

None.

## Dependencies

Every engine and `adapters`.

The direction of dependencies is checked by `tools/guardrails`
(`mise run test:tier1`). See the [architecture rewrite design](../../docs/superpowers/specs/2026-11-20-architecture-rewrite-design.md),
section Guardrails > Tests.
