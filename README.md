# map-generator

Generates planetary systems and maps of their worlds from a seed, with
physics taken from the literature: a system engine computes each
system's life from its birth, and a map generator lays one dated state of
a body out on its surface.

## Status

The project is being rewritten from a clean slate (roadmap #427, design:
[architecture rewrite](docs/superpowers/specs/2026-11-20-architecture-rewrite-design.md)).

- **Engine phase.** The workspace holds the new crates, empty for now;
  each feature comes back as its own pull request, starting with slice 1
  (#602): science, system engine at the present, world on a cube-sphere
  grid, catalogue, command line.
- **No server, no viewer.** The web viewer in `web/` is kept untouched
  until the viewer reconnection track (#610); only its checks run.
- **Legacy.** The last legacy commit is the tag `legacy-final`. The
  legacy app keeps running on planets.localhost from that tag, and its
  generator serves as a statistical oracle for the new one.

## Prerequisites

- [mise](https://mise.jdx.dev), which installs the pinned toolchain:
  Rust 1.99, cargo-nextest, markdownlint-cli2, actionlint, shellcheck,
  and Node 26 for the front-end.

## Installation

```bash
git clone https://github.com/cyboooooorg/map-generator.git
cd map-generator
mise install
```

## Usage

The command line gains its first command (`map-generator system`) with
slice 1. Until then, the tasks check the workspace:

| Task | What it does |
| --- | --- |
| `mise run ci` | Everything the pipeline runs but the front-end: format, clippy, tiers 1 and 2 with their budgets, markdownlint |
| `mise run test:tier1` | Unit tests and architecture guardrails (budget 2 min) |
| `mise run test:tier2` | Regression tests on shared reference systems (budget 8 min) |
| `mise run test:heavy` | Tier 3: full-size worlds, oracle, censuses (release, no budget) |
| `mise run fmt` | Format the code |
| `mise run web:check` | Type-check and test the front-end |

Options after `--` go to nextest, for example
`mise run test:tier1 -- -E 'package(guardrails)'`.

## Configuration

None yet: generation settings arrive as inputs of the command line with
slice 1. Tool versions are pinned in `mise.toml`; test tiers and their
thread count in `.config/nextest.toml`.

## Crates

Dependencies point inwards only; `tools/guardrails` checks it.

| Crate | Role |
| --- | --- |
| [`science`](crates/science/README.md) | Laws of physics and typed units |
| [`timeline`](crates/timeline/README.md) | The contract between the two engines |
| [`system`](crates/system/README.md) | System engine: a system's life, one timeline per body |
| [`world`](crates/world/README.md) | Map generator: one dated state of a body on the simulation grid |
| [`catalogue`](crates/catalogue/README.md) | Stored inputs and regenerable caches |
| [`adapters`](crates/adapters/README.md) | Files, exports, text catalogues |
| [`cli`](crates/cli/README.md) | The `map-generator` command, composition root |
| [`regression`](crates/regression/README.md) | Tier-2 tests on shared reference systems |
| [`tools/guardrails`](tools/guardrails/README.md) | Architecture checks and test budgets |
| `tools/opacity-table` | Offline builder of the climate's opacity tables from HITRAN |

## Documentation

- Design and plans: `docs/superpowers/specs/`, `docs/superpowers/plans/`.
- Science reference: `docs/science/`, one page per model with its
  sources (rewritten to the new rules as each law is ported).
- Studies written before decisions: the
  [Wiki](https://github.com/cyboooooorg/map-generator/wiki).
- Backlog: the repository's issues, label `backlog`, roadmap #427.
- Working on the repository (branches, tests, agents): `CLAUDE.md`.
