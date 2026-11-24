# CLI world command (W2 pull request B) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps
> use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expose `map-generator world`, which draws a system, builds the
world of one body with the real `system::generate`, and exports it.

**Architecture:** Pull request A (#652) left the pipeline
`cli::world::run` with an injected timeline producer, `GlobalArgs` and
`parse_size` unwired. This pull request names the body by its
designation's body part, flattens `GlobalArgs` into `Cli`, adds the
`world` subcommand wired to `system::generate`, and prints a JSON
summary.

**Tech Stack:** Rust, clap 4 (derive), serde_json, cargo-nextest tiers.

**Spec:** `docs/superpowers/specs/2026-11-22-cli-world-design.md`,
section "Pull request B".

## Start gate

Only tasks 3 to 5 call
`system::generate(&Inputs) -> Result<Vec<Timeline>, SystemError>`
(mapgen3, after mapgen1's planet PR 2d), but no task starts before it
is on `main`, per the user's decision of 2026-11-23:

```bash
git fetch origin
git grep -n 'pub fn generate(inputs: &Inputs)' origin/main -- crates/system/src
git merge -m "chore: merge main into feat/cli-world-command" origin/main
```

Message mapgen3 before task 1 (the CLI skeleton changes), and re-read
`world::relief::Forcing::read` (`crates/world/src/relief/passes.rs`):
the relief reads `regime`, `heat`, `bulk` and `liquid.ocean_share`; if
2d changed any of them, stop and tell the user.

## Global Constraints

- `--body` text is never parsed by the CLI: it becomes
  `MGS <number> <body>` and goes through `Designation::from_str`.
- Defaults: `--cell-km` = `world::DEFAULT_CELL` (15 km), `--field` =
  `relief.height`, `--width` = 2048; `--body` and `--out` are required.
- `--root` and `--cache-budget` are global (`global = true`), valid
  before or after the subcommand.
- No test generates a system or a world outside tier 3
  (`#[ignore = "heavy: …"]`).
- No number without `Source:`, `Calibration:` or `Definition:` outside
  test fixtures.
- Commits: Conventional Commits, no emoji, footer `Refs: #602`, then
  the session's attribution trailers.

## Review Focus

1. A malformed `--body` (`""`, `"b c"`, `"aa"`, `"MGS 12345 b"`) must
   be refused before the system engine runs, not after a long
   generation: task 2 parses it with a placeholder number first.
   (`"B"` is left out: it becomes a valid star with track T7, #585.)
2. `--body A` (the star) must exit 1 with `NotAPlanet`, never panic or
   write to `--out`: task 2.
3. A drawn seed (no `--seed`) must print the same stderr notice as
   `system`, so the run can be replayed: task 1 (shared helper) and
   task 3 (binary test).
4. `--cache-budget` before the subcommand
   (`map-generator --cache-budget 1GiB world …`) must parse: task 1.
5. `--cell-km 0`, a negative or a `NaN` cell size must be a usage error:
   today `Grid::for_radius` clamps 0 and negatives silently and turns
   `NaN` into a grid of 0 cells (`crates/world/src/grid.rs:77`): task 3
   (`parse_cell_km`).

A second identical run hitting both caches is covered by task 3's
tier-3 test.

---

### Task 1: Global options and the shared seed notice

**Files:**

- Modify: `crates/cli/src/args.rs` (`Cli`, `GlobalArgs`, `parse_size`,
  their tests)
- Modify: `crates/cli/src/main.rs` (`dispatch`, `announce`)

**Interfaces:**

- Produces: `Cli { global: GlobalArgs, command: Command }` with
  `#[command(flatten)] pub global: GlobalArgs`;
  `fn announce(root: RootSeed)` in `main.rs` (stderr stays in the
  composition root), which writes
  `map-generator: seed {n} drawn; rerun with --seed {n}` to stderr when
  `root.drawn`, and nothing otherwise.

- [ ] **Step 1: Write the failing tests** in `args.rs` `mod tests`:

```rust
#[test]
fn global_options_go_before_or_after_the_command() {
    for argv in [
        &["map-generator", "--cache-budget", "1GiB", "--root", "r", "system"][..],
        &["map-generator", "system", "--cache-budget", "1GiB", "--root", "r"][..],
    ] {
        let cli = Cli::try_parse_from(argv).unwrap();
        assert_eq!(cli.global.cache_budget, 1 << 30);
        assert_eq!(cli.global.root, PathBuf::from("r"));
    }
}
```

Replace the `Probe` struct test by the same defaults asserted on
`Cli::try_parse_from(["map-generator", "system"])`.

- [ ] **Step 2: Run** `cargo nextest run -p cli global_options` —
  expected: FAIL (no field `global`).
- [ ] **Step 3: Implement**: add the field, drop every
  `expect(dead_code)` / `cfg_attr(not(test), …)` on `GlobalArgs`,
  `store()` and `parse_size`; move the notice from `dispatch` into
  `announce` and call it from the `System` arm. `Cli.global` and
  `GlobalArgs::store` stay unread until task 3: keep
  `expect(dead_code, reason = "read by the world command, task 3")` on
  those two only.
- [ ] **Step 4: Run** `cargo nextest run -p cli` — expected: PASS, and
  `cargo nextest run -p cli --test system` still passes (stderr
  notice unchanged).
- [ ] **Step 5: Commit** `feat(cli): make the store options global`.

### Task 2: Name the body by its designation's body part

**Files:**

- Modify: `crates/cli/src/world.rs` (`WorldRequest`, `run`, tests)
- Modify: `crates/cli/src/error.rs` (variants, codes, messages)

**Interfaces:**

- Consumes: `timeline::Designation: FromStr`, `SystemNumber::new`.
- Produces: `WorldRequest.body: String`; new
  `CliError::InvalidBody(String)` (code `cli.world.invalid-body`, exit
  1, like the other `cli.world.*` failures);
  `CliError::NoSuchBody(String)` now carries the
  body text. `NotAPlanet` and `NoPresentKeyframe` keep the resolved
  `Designation`.

- [ ] **Step 1: Write the failing tests** in `world.rs` `mod tests`
  (tier 1, injected producer, no generation):

```rust
#[test]
fn malformed_body_is_refused_before_the_engine_runs() {
    let dir = TempDir::new().unwrap();
    let store = FsStore::new(dir.path().join("store"), u64::MAX);
    for bad in ["", "b c", "aa", "MGS 10001 b"] {
        let request = request(dir.path(), bad);
        let error = run(&store, &inputs(), &request, |_| -> Result<_, ContractError> {
            panic!("the engine must not run")
        })
        .unwrap_err();
        assert!(matches!(error, CliError::InvalidBody(_)), "{bad:?}: {error}");
    }
}

#[test]
fn star_is_refused() {
    let dir = TempDir::new().unwrap();
    let store = FsStore::new(dir.path().join("store"), u64::MAX);
    let request = request(dir.path(), "A");
    let error = run(&store, &inputs(), &request, |_| star_and_planet()).unwrap_err();
    assert!(
        matches!(error, CliError::NotAPlanet(d) if d == Designation::star(number())),
        "{error}"
    );
    assert_eq!(count(&request.out), 0);
}
```

Test helpers: `number()` is `SystemNumber::new(10_001).unwrap()`;
`request(out, rank)` becomes `request(out, body: &str)`. The existing
fixture timeline is rank 1, so the tests that build its world now pass
`"c"` and `unknown_body_is_refused` passes `"d"`.
`star_and_planet() -> Result<Vec<Timeline>, ContractError>` returns a
star timeline (`Designation::star(number())`, one keyframe at the
present 4.57 Gyr) followed by `timelines(false)`; its `StarState` is
built with `timeline::fixtures::leaf`, copying the Sun helper of
`crates/timeline/src/keyframe_tests.rs:8` (nominal solar values,
`StarPhase::MainSequence`).

- [ ] **Step 2: Run** `cargo nextest run -p cli world::` — expected:
  FAIL (type mismatch on `body`).
- [ ] **Step 3: Implement** in `run`: before anything else, check
  `format!("MGS 10000 {body}").parse::<Designation>()` and map the
  error to `InvalidBody(body)`; after the timelines exist, take the
  number from `all.first()` (an empty list is `NoSuchBody`), parse
  `MGS {number} {body}` again (cannot fail now) and search with it.
- [ ] **Step 4: Run** `cargo nextest run -p cli` — expected: PASS.
- [ ] **Step 5: Commit** `feat(cli): name the world's body by its
  designation's body part`.

### Task 3: The `world` subcommand

Needs `system::generate` on `main` (start gate).

**Files:**

- Modify: `crates/cli/src/args.rs` (`Command::World`)
- Modify: `crates/cli/src/world.rs` (`WorldCommand`, `command`,
  `Summary`; drop the remaining `expect(dead_code)`)
- Modify: `crates/cli/src/main.rs` (dispatch)
- Create: `crates/cli/tests/world.rs` (binary tests)

**Interfaces:**

- Consumes: `Cli.global` and `announce` (task 1), `WorldRequest.body:
  String` (task 2), `system::generate`.
- Produces:

```rust
#[derive(clap::Args, Debug)]
pub struct WorldCommand {
    #[command(flatten)] pub system: SystemArgs,
    #[arg(long)] pub body: String,
    #[arg(long, default_value_t = world::DEFAULT_CELL.0 / 1000.0, value_parser = parse_cell_km)]
    pub cell_km: f64,
    #[arg(long, default_value = "relief.height", value_parser = parse_field)] pub field: FieldId,
    #[arg(long, default_value_t = 2048)] pub width: u32,
    #[arg(long)] pub out: PathBuf,
}
pub fn command(command: &WorldCommand, global: &GlobalArgs, seed: Seed) -> Result<String, CliError>;
/// A finite cell size above 0 km.
pub fn parse_cell_km(text: &str) -> Result<f64, String>;
/// A field code, through `Code::new` (the shape only; the export refuses
/// a field the world lacks).
pub fn parse_field(text: &str) -> Result<FieldId, String>;
```

`WorldRequest.cell` is `Metre(cell_km * 1000.0)` (a unit conversion).

`command` builds the `WorldRequest`, calls `run(&global.store(),
&command.system.inputs(seed), &request, system::generate)` and returns
the pretty JSON of a `#[derive(Serialize)] Summary { system: SystemId,
body: Designation, keyframe: u32, grid: u32, engine: EngineVersion,
world_engine: EngineVersion, timelines_cached: bool, world_cached: bool,
evicted: usize, world: PathBuf, png: PathBuf }`, filled from `WorldRun`
(all these types serialise already). `evicted` is a count: `CacheKey`
does not serialise, and the count is what a user reads (my call). The usage
text of each flag states its default and unit as `SystemArgs` does.

- [ ] **Step 1: Write the failing tests**: in `args.rs`, a tier-1 parse
  test asserting the defaults (`cell_km` 15.0, `field` `relief.height`,
  `width` 2048), that a missing `--body` or `--out` is
  `ErrorKind::MissingRequiredArgument`, and that `--cell-km` `0`, `-1`,
  `NaN` and `inf` are `ErrorKind::ValueValidation`. In `tests/world.rs`, tier 3
  (`#[ignore = "heavy: generates a world"]`), through the binary with a
  temp `--root` and `--out` and a small `--width 64`:
  - `exports_a_planet_then_hits_the_caches`: `--seed <S> --body b`
    exits 0, the JSON has `timelines_cached: false`, the PNG exists; a
    second run has both caches `true` and the same PNG bytes. Pick `S`
    once `generate` exists: the first seed whose `system` output has a
    planet `b` with a surface; write the seed and why in the test.
  - `a_drawn_seed_is_announced`: no `--seed`, stderr contains
    `drawn; rerun with --seed` whatever the exit code (a drawn system
    may have no planet `b`). No regex: the crate has no `regex`
    dependency.
  - `the_star_is_refused`: `--seed <S> --body A` exits 1, stderr has
    `cli.world.not-a-planet`, `--out` is empty.
- [ ] **Step 2: Run** `cargo nextest run -p cli` and
  `cargo nextest run -p cli --run-ignored only --test world` —
  expected: FAIL (no `world` subcommand).
- [ ] **Step 3: Implement** the struct, `command`, `Summary` and the
  dispatch arm (`announce(root)` then `world::command`). Update the
  `Cli` `about` text and the `main.rs` module doc, which still say
  `system` only.
- [ ] **Step 4: Run** both commands again — expected: PASS; then
  `mise run ci` — expected: green within the tier budgets.
- [ ] **Step 5: Commit** `feat(cli): add the world command`.

### Task 4: Documentation

**Files:**

- Modify: `crates/cli/README.md` (sections "World pipeline" →
  "The `world` command", "Exit codes")
- Modify: `docs/superpowers/specs/2026-11-22-cli-world-design.md` only
  if the build departed from it

- [ ] **Step 1:** Write the section: purpose, every flag with its
  default, the body naming rule (body part, `A` is the star, multiple
  stars once #585 lands), the JSON summary fields, two copy-pasteable
  examples (`--seed 1 --body b --out out`, and with `--cache-budget
  512MiB` before the subcommand), the new code
  `cli.world.invalid-body`, and `--cell-km` refused at 0 or below.
- [ ] **Step 2: Run** `mise run ci` — expected: markdownlint green.
- [ ] **Step 3: Commit** `docs(cli): document the world command`.

### Task 5: Oracle, projections, review, pull request

Follows CLAUDE.md, "Before opening a pull request" (#668), read from
`main` at execution time in case it changed again.

- [ ] **Step 1:** `mise run ci`.
- [ ] **Step 2:** In parallel, each given the branch, this plan and the
  spec, not conclusions: `branch-reviewer` and `test-reviewer` (the
  branch changes tests). `science-coherence-checker` is not needed: the
  branch touches no law, `crates/science`, `docs/science` or study.
  Then `issue-auditor`, last, with their reports and the draft PR text.
- [ ] **Step 3:** Fix the findings on the branch, re-run `mise run ci`;
  file the issues the audit found missing (`issue-writer`), post its
  comments, put its keywords in the PR text (`Refs #602`).
- [ ] **Step 4:** `mise run test:heavy`; `world-coherence-checker` on
  the oracle's seeds; downscaled 2:1 projections (`sips -Z 800`) of a
  few bodies (one Earth-like, one Mars-like, one without plates).
- [ ] **Step 5:** Develop cycle (CLAUDE.md step 2, warn mapgen1 and
  mapgen3 first), then `pr-shepherd` opens the pull request with the
  heavy output pasted; the user looks at the projections before
  approving the merge.
