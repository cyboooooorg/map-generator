# `guardrails`: architecture checks

Keeps the workspace in the shape of the
[architecture rewrite design](../../docs/superpowers/specs/2026-11-20-architecture-rewrite-design.md)
(section Guardrails). Its tests run in tier 1 (`mise run test:tier1`):

- **Dependency direction** (`src/deps.rs`): every edge between workspace
  crates, normal, build or dev, follows the graph of `CLAUDE.md`;
  `science` has no external dependency unless listed; a crate missing
  from the allowlist fails, so a new crate is placed in the graph first.
- **Science constants** (`src/constants.rs`): every numeric `const` or
  `static` of `crates/science` has a doc comment starting with `Source:`,
  `Calibration:` or `Definition:`.
- **Science pages** (`src/science_pages.rs`): every article of
  `docs/science/` has a `## Sources` section holding at least one line;
  a page exempted for now (`EXEMPT`, with why) fails once it has one,
  so the list only shrinks.
- **README length** (`src/readme.rs`): the root `README.md` stays under
  300 lines, each `crates/*/README.md` under 150.
- **Clippy configurations** (`src/clippy_config.rs`): every
  `clippy.toml` keeps the determinism entries of the root one.

Its binary gates the test tiers on their budgets, from nextest's JUnit
report:

```bash
cargo run -q -p guardrails -- budget target/nextest/tier1/junit.xml 120 "tier 1"
```

`mise run test:tier1` and `test:tier2` call it after their run.
