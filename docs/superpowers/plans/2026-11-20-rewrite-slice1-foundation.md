# Rewrite slice 1, PR 1: foundation and guardrails — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps
> use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the legacy crates on `main` with the new, empty
hexagonal workspace and land every guardrail that keeps it in shape
(Migration step 4 of the spec).

**Architecture:** Seven crates with the dependency graph of the spec
(`science`, `timeline`, `system`, `world`, `catalogue`, `adapters`,
`cli`), a test-only `regression` crate for tier 2, and a `guardrails`
tool crate whose tests check the architecture (dependency direction,
clippy configuration, science constants, README length) and whose binary
enforces the test-tier budgets. One `mise run ci` runs what GitHub runs.

**Tech Stack:** Rust 1.99 (edition 2024), cargo-nextest, clippy
`disallowed_*`, `cargo_metadata`, `syn`, `quick-xml`, mise,
markdownlint-cli2, GitHub Actions (`jdx/mise-action`,
`Swatinem/rust-cache`, `dorny/paths-filter`).

**Spec:** `docs/superpowers/specs/2026-11-20-architecture-rewrite-design.md`
(sections Migration, Architecture > Crates, Guardrails, Documentation,
Build order > Slice 1). Tracking issue #602, roadmap #427.

## Slice 1 split into pull requests

The spec leaves the split to the builder. Each pull request leaves `main`
buildable and tested, and gets its own plan once the previous one is
merged (recon of `legacy-final` with `codebase-mapper` first):

| PR | Content | Issues |
| --- | --- | --- |
| 1 (this plan) | Remove the legacy crates, empty workspace, guardrails, docs move, agents | Migration step 4 |
| 2 | `science` units and the laws of the star and orbits; `timeline` types (one keyframe, stellar sources as a list); system draw at the present with designations and causes; `cli system` writing the timeline JSON and `--explain`; determinism test; tier-2 reference systems; mdBook science reference | #592 (may be its own PR, see note), #476 |
| 3 | Planet model in the system engine: interiors with self-compression and interior profiles, atmosphere and escape, global climate with the spectral column (data and `docs/science` pages copied from `legacy-final`); SVG cross-sections | #534, #593 |
| 4 | `world`: cube-sphere grid at 15 km, relief, spatialized climate, seasonal statistics, Köppen, surface layers, named biomes; 2:1 PNG projection; `catalogue` on the filesystem | — |
| 5 | Legacy oracle per archetype (`world-coherence-checker`), generation profiling and the budget proposal | — |

Note: mapgen3 offered to take #592 as its own PR and mapgen1 the
science and climate port (PR 3); the user decides the split between
sessions.

## Global Constraints

- Rust for all new code, strictly typed; edition 2024, toolchain
  `rust = "1.99"` (mise).
- Dependencies point inwards only:
  `cli ──> adapters ──> catalogue, system, world, timeline, science`;
  `catalogue ──> system, timeline`; `system ──> timeline, science`;
  `world ──> timeline, science`; `timeline ──> science`.
  `world` never depends on `system`, nor `system` on `world`.
- No random source outside the seed hierarchy (clippy
  `disallowed_methods`); no `std::fs`, `std::net` or process spawning in
  `science`, `timeline`, `system`, `world`, `catalogue`.
- Test tiers and budgets: tier 1 (unit) 2 min, tier 2 (regression)
  8 min, both on every push; tier 3 (heavy) local or by hand, no budget.
- Root `README.md` at most 300 lines, each crate `README.md` at most
  150 lines.
- Every constant in `science` has a doc comment starting with `Source:`
  or `Calibration:` (exemption below: `Definition:`).
- Keep `web/` untouched (its checks keep running), keep `docs/science/`,
  `docs/system-spec.md`, `docs/backlog.md`; move `docs/analysis/` to the
  GitHub Wiki. History kept: nothing force-pushed.
- Shared machine: never `npm ci` in a worktree, throttle local test runs
  (`--test-threads 4`), never touch the data checkout
  `~/Developer/cyboooooorg/map-generator`.
- Conventional Commits; markdownlint default rules on every new or
  rewritten Markdown file.

## Decisions taken in this plan

- **Tiers by location, not by attribute.** Tier 2 is every test of the
  test-only crate `crates/regression` (it may depend on every engine,
  which a domain crate's dev-dependencies may not). Tier 3 is any test
  marked `#[ignore = "heavy: <why>"]`. Tier 1 is everything else.
- **Budgets** are checked on nextest's JUnit report (wall time of the
  run) by `guardrails budget`.
- **Clippy configuration.** The root `clippy.toml` forbids random
  sources, wall clocks, default-hashed maps, files, network and process
  spawning. `crates/adapters`, `crates/cli` and `tools/*` carry their own
  `clippy.toml` (clippy reads the nearest one, without merging) that
  keeps only the random, clock and hash entries. A guardrails test keeps
  the shared entries identical.
- **`Definition:`** is the third accepted doc prefix for `science`
  constants: exact definitions and unit conversions (SI-defined values,
  seconds per day). It is listed in `CLAUDE.md` with the other two.
- **`mise run ci` runs the Rust and Markdown checks.** The front-end
  check stays a separate task (`web:check`, its own CI job), because
  `npm ci` may not run in a worktree on this machine.
- **The mdBook science reference and its GitHub Pages workflow come with
  PR 2**, when its first page is rewritten to the new rules; the
  determinism test also comes with PR 2, when there is a system to
  regenerate. This PR lands the clippy lint for it.
- **Left to PR 2: the tier-2 fixtures.** The spec caches the reference
  worlds on disk under a key hashing the engine sources. `crates/regression`
  falls under the strict root `clippy.toml` (no `std::fs`), so PR 2 decides
  who writes them: `regression` joins the relaxed crates (a `clippy.toml`
  and an entry in `RELAXED` of `tools/guardrails`), or the fixtures go
  through a catalogue port and its `adapters` implementation.
- **Review follow-ups applied on the branch.** `mise run ci` runs its steps
  one after the other (two tiers at once would take eight threads) and
  includes actionlint; tier 1 also runs the doctests; a `let` bound to a
  bare number in `science` must become a documented constant; relaxed
  clippy copies are pinned to `crates/adapters`, `crates/cli` and
  `tools/*`, and hidden `.clippy.toml` files are refused; every member
  inherits the workspace lints; every member has a README; the study and
  code links of `docs/` point at the Wiki and at `legacy-final`.
- **`tools/opacity-table` stays** (standalone, no dependency on the
  legacy crates); its output path moves with the climate port (PR 3).
- Removed with the legacy crates: `Dockerfile`, `compose.yaml`,
  `.dockerignore`, `mise-tasks/up`, `archetypes/`, `looks/`,
  `.cargo/config.toml` (ts-rs export path) and the server and Docker
  mise tasks. `web/src/lib/api/types/` stays committed, untouched.

## Review Focus

1. A crate added later to `crates/` without an entry in the guardrails
   allowlist: the dependency test must fail on an unknown workspace
   member, not skip it.
2. A pull request touching only `docs/` or `web/`: the Rust jobs are
   skipped and the summary job `ci-ok` still reports success; a failing
   Rust job makes `ci-ok` fail.
3. A tier-1 run that is green test by test but over budget: `mise run
   ci` fails with the measured time and the budget in the message.
4. A `science` constant written as `pub(crate) static`, inside an `impl`
   block or a nested module: the constant check must see it.
5. A `README.md` added to a new crate: the length check covers every
   `crates/*/README.md`, not a fixed list.

Each line has its test in Tasks 3 to 5.

---

### Task 1: Remove the legacy crates and lay out the workspace

**Files:**

- Delete: `crates/mapgen/`, `crates/server/`, `crates/cli/` (replaced),
  `Dockerfile`, `compose.yaml`, `.dockerignore`, `mise-tasks/`,
  `archetypes/`, `looks/`, `.cargo/config.toml`
- Modify: `Cargo.toml`, `mise.toml`, `.gitignore`
- Create: `crates/{science,timeline,system,world,catalogue,adapters}/`
  (`Cargo.toml`, `src/lib.rs`, `README.md`), `crates/cli/` (`Cargo.toml`,
  `src/main.rs`, `README.md`), `crates/regression/` (`Cargo.toml`,
  `src/lib.rs`, `tests/contracts.rs`, `README.md`)

**Interfaces:**

- Produces: packages named `science`, `timeline`, `system`, `world`,
  `catalogue`, `adapters`, `regression`, and `map-generator` (in
  `crates/cli`, binary `map-generator`). Workspace members
  `["crates/*", "tools/*"]`; `tools/guardrails` comes in Task 3.

- [ ] **Step 1: Delete the legacy files** with `git rm -r` (list above).
  Check `git status` shows no deletion under `web/`, `docs/science/`,
  `tools/opacity-table/`.

- [ ] **Step 2: Write the workspace `Cargo.toml`.** Members
  `["crates/*", "tools/*"]`, resolver 3, edition 2024, version 0.1.0.
  `[workspace.dependencies]` lists the internal crates by path only
  (`science = { path = "crates/science" }`, …); external ones are added
  by the tasks that use them. Drop the legacy profile overrides but
  keep `[profile.dev.package."*"] opt-level = 3`. Add
  `[workspace.lints.rust] unsafe_code = "forbid"` and
  `[workspace.lints.clippy] disallowed_methods = "deny"`,
  `disallowed_types = "deny"`; every member sets
  `[lints] workspace = true`.

- [ ] **Step 3: Create the crates** with the internal dependencies of
  the Global Constraints graph (`cli` depends on all seven, `regression`
  on all engines and `adapters`). Each `lib.rs` holds only its crate doc
  comment (role and ports, one paragraph, from spec Architecture);
  `cli/src/main.rs` prints the usage and exits 2 until PR 2.
  `regression/tests/contracts.rs` holds one test,
  `workspace_builds`, that asserts `true` so the tier-2 profile has
  something to run (replaced by the reference systems in PR 2).

- [ ] **Step 4: Write each crate `README.md`**: purpose, ports it owns
  (none yet: say which come in which PR), what it may depend on, link to
  the spec section. Under 40 lines each.

- [ ] **Step 5: Verify the build**

  ```bash
  cargo build --workspace
  cargo clippy --workspace --all-targets -- -D warnings
  ```

  Expected: both succeed with no warning.

- [ ] **Step 6: Commit**

  ```bash
  git commit -m "refactor!: replace the legacy crates with the rewrite workspace" \
    -m "BREAKING CHANGE: crates/mapgen, crates/server and the legacy CLI are removed; the last legacy commit is the tag legacy-final."
  ```

### Task 2: Clippy configuration for determinism and boundaries

**Files:**

- Create: `clippy.toml`, `crates/adapters/clippy.toml`,
  `crates/cli/clippy.toml`, `tools/opacity-table/clippy.toml`
- Modify: `tools/opacity-table/Cargo.toml` (`[lints] workspace = true`
  only if it passes; otherwise leave it out of the workspace lints and
  note why in its `clippy.toml`)

**Interfaces:**

- Produces: the exact entry lists Task 3's
  `clippy_configs_share_the_determinism_entries` test reads.

- [ ] **Step 1: Write `clippy.toml`** with, each with a `reason`:
  - `disallowed-methods`: `rand::rng`, `rand::thread_rng`,
    `rand::random`, `rand::make_rng`, `getrandom::fill`,
    `getrandom::getrandom`, `std::time::SystemTime::now`,
    `std::time::Instant::now`, `std::process::Command::new`,
    `std::env::var`, `std::env::vars`;
  - `disallowed-types`: `std::collections::HashMap`,
    `std::collections::HashSet` (iteration order depends on a random
    hasher: use `BTreeMap` / `BTreeSet`), `std::fs::File`,
    `std::fs::OpenOptions`, `std::net::TcpStream`,
    `std::net::TcpListener`, `std::net::UdpSocket`;
  - `disallowed-methods` also lists the free functions of `std::fs`
    (`read`, `read_to_string`, `write`, `create_dir_all`, `remove_file`,
    `remove_dir_all`, `rename`, `copy`, `read_dir`, `metadata`).
- [ ] **Step 2: Write the three relaxed `clippy.toml`** with only the
  random, clock (`SystemTime::now` stays forbidden; `Instant::now` is
  allowed in adapters and cli for timings) and hash entries.
- [ ] **Step 3: Prove the lint bites.** Add temporarily
  `let _ = std::fs::read("x");` to `crates/world/src/lib.rs` and run
  `cargo clippy -p world -- -D warnings`. Expected: error
  `use of a disallowed method`. Remove the line; run again: clean.
- [ ] **Step 4: Commit** `build: forbid random sources and I/O in the domain crates`.

### Task 3: Guardrails crate: architecture tests

**Files:**

- Create: `tools/guardrails/Cargo.toml`, `tools/guardrails/src/lib.rs`,
  `tools/guardrails/src/deps.rs`, `tools/guardrails/src/constants.rs`,
  `tools/guardrails/src/readme.rs`, `tools/guardrails/src/clippy_config.rs`,
  `tools/guardrails/tests/workspace.rs`, `tools/guardrails/README.md`
- Dependencies: `cargo_metadata`, `syn` (features `full`, `visit`),
  `toml`, `walkdir`

**Interfaces:**

- Produces (all in `guardrails`):
  - `deps::allowed() -> BTreeMap<&'static str, BTreeSet<&'static str>>`
    (workspace-internal edges, the graph of Global Constraints;
    `regression` and `guardrails` listed with their own sets) and
    `deps::external_allowed(package: &str) -> Option<&'static [&'static str]>`
    (`Some(&[])` for `science` for now, `None` = unrestricted).
  - `deps::violations(meta: &cargo_metadata::Metadata) -> Vec<String>`:
    one line per forbidden edge, normal, build and dev dependencies
    alike, and one per workspace member missing from `allowed()`.
  - `constants::undocumented(source: &str) -> Vec<String>`: names of
    `const` and `static` items (any visibility, in `impl` blocks, traits
    and nested inline modules) with a numeric type or a numeric literal
    initializer whose doc comment does not start with `Source:`,
    `Calibration:` or `Definition:`.
  - `readme::too_long(root: &Path) -> Vec<String>`: root `README.md` over
    300 lines, any `crates/*/README.md` over 150.
  - `clippy_config::missing_shared_entries(root: &Path) -> Vec<String>`:
    entries of the root `clippy.toml` marked shared (random, hash,
    `SystemTime::now`) absent from any other `clippy.toml` of the
    workspace.

- [ ] **Step 1: Write the failing unit tests** in each module, on
  inline fixtures:

  ```rust
  #[test]
  fn world_depending_on_system_is_a_violation() { /* metadata fixture:
     world -> system; assert violations contains "world -> system" */ }
  #[test]
  fn unknown_member_is_a_violation() { /* member "newcrate" absent from
     allowed(); assert violations contains "newcrate: not in the allowlist" */ }
  #[test]
  fn dev_dependency_counts() { /* world dev-depends on system: violation */ }

  #[test]
  fn constant_doc_prefixes() {
      let src = r#"
          /// Source: CODATA 2018.
          pub const G: f64 = 6.674_30e-11;
          /// Seconds per day.
          pub const DAY: f64 = 86_400.0;
          /// Definition: seconds per day.
          pub const DAY_S: f64 = 86_400.0;
          pub(crate) static K: f32 = 0.3;
          mod inner { const X: u32 = 7; }
          impl Foo { const Y: f64 = 1.5; }
          const NAME: &str = "x";
      "#;
      assert_eq!(constants::undocumented(src), ["DAY", "K", "inner::X", "Foo::Y"]);
  }

  #[test]
  fn readme_limits() { /* tempdir with a 301-line README.md and a
     151-line crates/foo/README.md: both reported; 300 and 150 pass */ }
  ```

- [ ] **Step 2: Run them to see them fail**

  Run: `cargo nextest run -p guardrails`
  Expected: FAIL (functions not defined).

- [ ] **Step 3: Implement the four modules.** `deps` builds the edge set
  from `metadata.packages` filtered to workspace members (`dependencies`
  with `path` set); fixtures are built as JSON and parsed with
  `cargo_metadata::Metadata` via serde. `constants` walks with
  `syn::visit::Visit` and keeps a module/impl path for the names.

- [ ] **Step 4: Write the workspace tests** in `tests/workspace.rs`:
  `dependency_graph_points_inwards` (runs `cargo_metadata::MetadataCommand`
  on the workspace root, asserts `violations` is empty),
  `science_constants_are_sourced` (every `.rs` under `crates/science/src`),
  `readmes_stay_short`, `clippy_configs_share_the_determinism_entries`.
  `guardrails` itself is a tool, outside the domain crates: its own
  `clippy.toml` (relaxed, as in Task 2) allows its file reads.

- [ ] **Step 5: Run everything**

  Run: `cargo nextest run -p guardrails`
  Expected: PASS, unit and workspace tests.

- [ ] **Step 6: Commit** `test(guardrails): check dependency direction,
  science constants and README length`.

### Task 4: Test tiers, budgets and the mise tasks

**Files:**

- Create: `.config/nextest.toml`, `tools/guardrails/src/budget.rs`,
  `tools/guardrails/src/main.rs`, `.markdownlint-cli2.jsonc`
- Modify: `mise.toml`, `tools/guardrails/Cargo.toml` (`quick-xml`, `clap`)

**Interfaces:**

- Consumes: nothing from Task 3 beyond the crate.
- Produces: `budget::wall_seconds(junit_xml: &str) -> Result<f64, String>`
  (the `time` of the root `testsuites` element) and the binary
  `guardrails budget <junit.xml> <seconds> <tier name>`, exit 1 with
  `tier 1 took 131.4 s, budget 120 s: move, merge or delete tests
  (see CLAUDE.md, Tests)`.
- mise tasks: `fmt`, `fmt:check`, `lint` (clippy, warnings denied),
  `test:tier1`, `test:tier2`, `test:heavy`, `test` (tier 1 + 2),
  `md:lint`, `ci` (depends on `fmt:check`, `lint`, `test:tier1`,
  `test:tier2`, `md:lint`), `web:install`, `web:check`, `web:build`
  (kept as they are).

- [ ] **Step 1: Write the failing test** `budget_reads_the_run_wall_time`
  on a JUnit fixture whose `testsuites time="131.4"` and whose test
  cases sum to more: assert `wall_seconds == 131.4`; and
  `over_budget_message_names_the_tier`.
- [ ] **Step 2: Run it** (`cargo nextest run -p guardrails budget`),
  expect FAIL; **implement** with `quick_xml::Reader`; run again, PASS.
- [ ] **Step 3: Write `.config/nextest.toml`**: profiles `tier1`
  (`default-filter = "not package(regression)"`), `tier2`
  (`default-filter = "package(regression)"`), `heavy`
  (`default-filter = "all()"`, run with `--run-ignored only`), each with
  `junit.path = "junit.xml"`; `test-threads = 4` in the default
  profile, which the tier profiles inherit (shared machine). CI passes
  `--test-threads num-cpus` on the command line.
- [ ] **Step 4: Write the mise tasks.** `test:tier1` runs
  `cargo nextest run --profile tier1 --workspace` then
  `guardrails budget target/nextest/tier1/junit.xml 120 "tier 1"`;
  `test:tier2` the same with 480 s; `test:heavy` runs
  `cargo nextest run --profile heavy --release --run-ignored only`.
  Tools: add `"cargo:cargo-nextest"` and `"npm:markdownlint-cli2"`
  to `[tools]`. `md:lint` runs `markdownlint-cli2` on the globs of
  `.markdownlint-cli2.jsonc`: `README.md`, `CLAUDE.md`,
  `crates/**/*.md`, `tools/**/*.md`, `.claude/agents/*.md`,
  `docs/superpowers/plans/2026-11-20-rewrite-*.md` (legacy docs are
  linted when rewritten).
- [ ] **Step 5: Prove the budget fails.** Run
  `guardrails budget target/nextest/tier1/junit.xml 0.001 "tier 1"`
  (through `cargo run -q -p guardrails --`).
  Expected: exit 1 with the message above.
- [ ] **Step 6: Run `mise run ci`.** Expected: all green, tier 1 and 2
  times printed under their budgets.
- [ ] **Step 7: Commit** `build: add test tiers with budgets and mise run ci`.

### Task 5: GitHub workflows

**Files:**

- Modify: `.github/workflows/ci.yml`
- Create: `.github/workflows/heavy.yml`

**Interfaces:**

- Consumes: the mise tasks of Task 4.

- [ ] **Step 1: Rewrite `ci.yml`** with jobs:
  - `changes`: `dorny/paths-filter@v3` with outputs `rust`
    (`crates/**`, `tools/**`, `Cargo.*`, `clippy.toml`, `**/clippy.toml`,
    `.config/**`, `mise.toml`, `.github/workflows/**`), `web`
    (`web/**`), `markdown` (`**/*.md`, `.markdownlint-cli2.jsonc`);
  - `fmt-lint` (`mise run fmt:check` and `mise run lint`), `tier1`,
    `tier2` (`mise run test:tier1|tier2`, nextest with
    `--test-threads num-cpus`), each `if: needs.changes.outputs.rust == 'true'`,
    with `jdx/mise-action@v5` and `Swatinem/rust-cache@v2`
    (`key: ${{ hashFiles('Cargo.toml') }}`, shared-key per job kind);
  - `markdown` (`mise run md:lint`) when `markdown` changed;
  - `web` (`mise run web:check`, `mise run web:build`) when `web`
    changed;
  - `ci-ok`: `if: always()`, `needs` every job above, fails when any
    needed result is `failure` or `cancelled`, succeeds when they are
    `success` or `skipped`. This is the job to require in a ruleset.
  - Triggers stay `push` to `main` (keeps `main`'s cache warm) and
    `pull_request`.
- [ ] **Step 2: Write `heavy.yml`**: `workflow_dispatch` only, one job
  running `mise run test:heavy`, `timeout-minutes: 300`. The oracle
  binary cache comes with PR 5.
- [ ] **Step 3: Validate** with `actionlint` (`mise x aqua:rhysd/actionlint -- actionlint`).
  Expected: no error.
- [ ] **Step 4: Commit** `ci: run the mise tasks with path filters and a
  summary job`.

### Task 6: Documentation: README, CLAUDE.md, studies to the Wiki

**Files:**

- Rewrite: `README.md` (under 300 lines)
- Modify: `CLAUDE.md`
- Delete: `docs/analysis/` (21 files, after the Wiki push)
- Modify: links to `docs/analysis/` in `CLAUDE.md`,
  `.claude/agents/branch-reviewer.md`, `.claude/agents/science-fixer.md`
  (specs and plans of the legacy keep their links: they are history and
  the files live on at `legacy-final`)

- [ ] **Step 1: Check the Wiki repository exists**:
  `GIT_TERMINAL_PROMPT=0 git ls-remote https://github.com/cyboooooorg/map-generator.wiki.git`.
  If it fails, stop and ask the user to create the Wiki's first page in
  the GitHub UI (the Wiki's git repository only exists after that).
- [ ] **Step 2: Push the studies to the Wiki**: clone it into the
  scratchpad, copy each `docs/analysis/<name>.md` to `<name>.md`
  (Wiki page `<name>`), rewrite their relative links between studies to
  Wiki links and their links into the code to
  `https://github.com/cyboooooorg/map-generator/blob/legacy-final/<path>`,
  add a `Home.md` index linking every study, commit
  `docs: move the studies from docs/analysis`, push.
  Expected: `https://github.com/cyboooooorg/map-generator/wiki/17-host-stars` renders.
- [ ] **Step 3: Remove `docs/analysis/` and fix the repository links**
  to point at `https://github.com/cyboooooorg/map-generator/wiki/<name>`.
- [ ] **Step 4: Rewrite `README.md`**: purpose (the rewrite, link to the
  spec and #427), status (engine phase: no server, viewer paused, the
  legacy app is `legacy-final` on planets.localhost), prerequisites
  (mise), installation (`mise install`), usage (`mise run ci`, the test
  tasks; the CLI comes with PR 2), configuration (none yet), the crate
  map with one line each linking their README, links to the science
  reference (`docs/science/`), the studies (Wiki) and the backlog
  (issues).
- [ ] **Step 5: Update `CLAUDE.md`**: add `## Architecture` (crates,
  dependency graph, the "where a change goes" table of spec Guardrails,
  the anti-goals as rules, link to the spec); add `## No invented data`
  (the four rules of spec Documentation, with the `Source:` /
  `Calibration:` / `Definition:` prefixes); add `## Tests` (the tiers
  and budgets, where each tier lives, `#[ignore = "heavy: …"]`, the
  arbitration rule, `mise run ci` before a pull request, tier-3 output
  in the description of a pull request that changes what the engines
  produce); replace the `mise run test:slow` rule; replace the
  `README.md` size note in "Large documents" (the new README is short;
  `docs/science/` pages and `docs/system-spec.md` stay large); add
  `test-reviewer` to the agents table and mark `develop-deployer` and
  `viewer-visual-check` paused; replace "Testing on develop" with the
  engine-phase rule of spec Proof (build `develop`, run the CLI on the
  oracle's seeds, downscaled projections for the user).
- [ ] **Step 6: Run `mise run md:lint`.** Expected: no error.
- [ ] **Step 7: Commit** `docs: rewrite the README and CLAUDE.md for the
  rewrite workspace`.

### Task 7: Agents

**Files:**

- Modify: `.claude/agents/{codebase-mapper,branch-reviewer,science-fixer,branch-syncer,world-coherence-checker,generation-profiler,develop-deployer,viewer-visual-check}.md`
- Create: `.claude/agents/test-reviewer.md`

- [ ] **Step 1: Update each agent** as in spec Guardrails > Agents:
  - `codebase-mapper`: the new crate map; the legacy code is read at
    `legacy-final` (`git show legacy-final:<path>`, `git grep … legacy-final`).
  - `branch-reviewer`: check boundaries (dependency graph, no central
    object kind read by `world`, no feedback), engine version bump when
    outputs change, every new number sourced or calibrated in all
    engines, causes produced by the computing code path.
  - `science-fixer`: targets `science`; ports physics from
    `legacy-final` with citations and tests; studies first (No invented
    data); runs `mise run test:tier1`.
  - `branch-syncer`: drop `raw_data` and tiles; run `mise run ci`.
  - `world-coherence-checker`: the oracle, legacy against new, per
    archetype as distributions over a fixed seed set, never by bytes or
    seed by seed (its procedure lands with PR 5).
  - `generation-profiler`: the new CLI (lands with PR 2), per-stage
    timings of system and world generation.
  - `develop-deployer`, `viewer-visual-check`: description starts with
    `Paused during the engine phase of the rewrite (no server):` and the
    body says not to run them.
- [ ] **Step 2: Write `test-reviewer.md`**: frontmatter `model: sonnet`,
  `effort: medium`, `tools: Read, Grep, Glob, Bash`, read-only. It
  checks each new test's tier, that no test outside `crates/regression`
  or tier 3 generates a system or world, duplicates and tests made
  useless by the branch, that every expected value cites a source (or
  is a tier-2 regression net), and measures with `mise run test:tier1`
  and `test:tier2` what the branch adds to each tier against its
  budget (run on the branch and on `origin/main`). It proposes moves,
  merges or deletions; it decides nothing.
- [ ] **Step 3: Run `mise run md:lint`.** Expected: no error.
- [ ] **Step 4: Commit** `docs(agents): update the agents to the rewrite workspace`.

### Task 8: Whole-branch check and pull request

- [ ] **Step 1: Run `mise run ci`** on the branch. Expected: green,
  tier times under budget.
- [ ] **Step 2: Run `branch-reviewer` and `test-reviewer`** once on the
  branch with the spec and this plan; fix what they find on the branch,
  re-run `mise run ci`.
- [ ] **Step 3: Open the pull request** into `main` (`pr-shepherd`),
  title `refactor!: replace the legacy crates with the rewrite workspace`,
  body `Refs #602`, the list of removed paths, the Wiki link, and a
  "Calibrations" heading saying none.
- [ ] **Step 4: After merge** (user's approval only): reset `develop`
  to `main`; require the `ci-ok` check in the `main` ruleset (ask the
  user first, it changes the repository settings); edit the open issues
  whose bodies link `docs/analysis/` to point at the Wiki (one comment
  on #602 listing them).
