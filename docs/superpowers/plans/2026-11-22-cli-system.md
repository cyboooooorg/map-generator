# `cli system` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `map-generator system` draws a system from its inputs and prints
it as JSON, or prints the cause chain of one value with `--explain`.

**Architecture:** the `cli` crate parses the flags with clap 4 derive into
`system::Inputs` (one place: `SystemArgs::inputs`), calls
`system::draw_with` with a provisional classifier, and serialises a
`SystemReport`: the star's `Timeline` (one keyframe, the present) and the
planets' draw (designations, orbits, spins, sources) with their own
`CauseGraph`. `--explain` serialises the same report to a
`serde_json::Value`, finds the traced value named by a JSON pointer or a
cause code, and prints its cause tree.

**Tech Stack:** Rust 1.99, clap 4.6.9 (derive), getrandom 0.4.3, serde, serde_json.

**Spec:** `docs/superpowers/specs/2026-11-21-system-draw-design.md`,
sections "`cli system`", "Errors and traces", "Seeds and determinism".

## Global Constraints

- Command: `map-generator system [--seed N] [--present GYR] [--host-mass M]
  [--metallicity DEX] [--archetype ID=COUNT ...] [--explain PATH]`. The bin
  keeps its name `map-generator`.
- `--seed` is optional (user decision, 2026-11-22). Without it, the CLI
  draws the **root** seed from the OS (`getrandom::u64()`), the top of the
  seed hierarchy of the rewrite design (§ Determinism and seeds): nothing
  else is drawn outside it. That one call is the only random source, in
  `crates/cli` only, under `#[expect(clippy::disallowed_methods, reason =
  "…")]` (`tools/guardrails` forbids it only in the domain crates). The
  drawn seed is printed on stderr (`map-generator: seed N drawn; rerun
  with --seed N`) and is in the JSON's `inputs.seed`, so every run can be
  replayed.
- Planet states are not printed until mapgen1's `class_of` is public
  (their PR 2d). The draw still needs a classifier: use the stand-in, and
  say so in the JSON (`"classifier": "stand-in"`).
- CLI skeleton agreed with mapgen2: clap 4 derive in `crates/cli` only;
  `main.rs` (parse, dispatch, `ExitCode`), `args.rs` (`Cli { command:
  Command }`, `SystemArgs` with `fn inputs(…) -> system::Inputs`, the
  single flags-to-`Inputs` place, shared later with `world`; it now takes
  the seed, see Task 2), `system.rs`
  (`SystemCommand { #[flatten] system: SystemArgs, --explain }`). No
  `GlobalArgs` (`--root`, `--cache-budget`) in this PR: they come with
  mapgen2's `world` PR.
- Errors print their code first (`system.unknown-archetype: …`,
  `cli.explain.not-found: …`) on stderr, prefixed `map-generator: `.
  Exit 2 for a usage error (clap's), 1 for a failed command, 0 otherwise.
- Workspace lints: no `unsafe`, no `HashMap`/`HashSet`, no wall clock,
  no random source but the root seed draw above. Use `BTreeMap`.
- Docs in the same change: `README.md` § Usage, `crates/cli/README.md`,
  `crates/system/README.md` (stand-in). `README.md` has a length guardrail:
  keep the addition short. Markdown follows markdownlint defaults.
- Commits: Conventional Commits, scopes `cli`, `system`, `timeline`.

## Review Focus

1. A date outside the star's life (`--host-mass 1 --present 50`; without
   the host mass a red dwarf may live longer than 50 Gyr): exit 1 with
   `system.present-after-life: …`, no JSON on stdout. Test in Task 3.
2. A malformed `--archetype` (`earth-like`, `earth-like=x`,
   `Earth-Like=1`, `=2`): exit 2 with clap's usage message. An unknown but
   well-formed id (`pluto-like=1`): exit 1 with `system.unknown-archetype`.
   Tests in Tasks 2 and 3.
3. The same archetype given twice (`--archetype earth-like=1 --archetype
   earth-like=2`): the counts add up (3). A count of 0 is passed through
   and skipped by the draw. Test in Task 2.
4. `--explain` on something that is not a traced value (a pointer to
   `/planets/0/designation`, a pointer past the end, a code nobody
   records): exit 1 with `cli.explain.not-traced` or
   `cli.explain.not-found`, never a panic. Tests in Task 4.
5. `--explain` with a code held by several values (`system.draw.flux`, one
   per planet): exit 1 with `cli.explain.ambiguous`, listing the JSON
   pointers that hold it, so the user can pick one. Test in Task 4.

---

## File Structure

- `crates/system/src/stand_in.rs` (new): the provisional classifier,
  moved from `crates/regression/src/lib.rs`.
- `crates/timeline/src/planet.rs`: `TracedFields` for `Orbit` and `Spin`.
- `crates/cli/src/main.rs`: parse, dispatch, map errors to exit codes.
- `crates/cli/src/args.rs`: `Cli`, `Command`, `SystemArgs`, the
  `ID=COUNT` parser.
- `crates/cli/src/system.rs`: `SystemCommand`, `run`.
- `crates/cli/src/report.rs`: `SystemReport`, `PlanetDraw`, `report()`.
- `crates/cli/src/explain.rs`: resolve a pointer or code, print a tree.
- `crates/cli/src/error.rs`: `CliError`.
- `crates/cli/tests/system.rs`: runs the binary
  (`env!("CARGO_BIN_EXE_map-generator")`, `std::process::Command`; no
  `assert_cmd`).

### Task 1: Stand-in classifier in `system`, `TracedFields` for orbits and spins

**Files:**

- Create: `crates/system/src/stand_in.rs`
- Modify: `crates/system/src/lib.rs` (add `pub mod stand_in;`)
- Modify: `crates/regression/src/lib.rs:25-60` (`stand_in_model` becomes
  `pub use system::stand_in::classify as stand_in_model;`, keeping its
  callers working)
- Modify: `crates/system/tests/determinism.rs:15-45` and
  `crates/system/src/draw/tests.rs:30-55` (drop each local `model` and
  `EARTH_DENSITY` copy, call `crate::stand_in::classify` /
  `system::stand_in::classify`; if a copy differs from the regression
  one, keep it and say so in the report)
- Modify: `crates/timeline/src/planet.rs` (two impls), test in the
  crate's existing test module for planets (or `cause_tests.rs`)
- Modify: `crates/system/README.md` (one line in the module table)

**Interfaces:**

- Produces: `pub fn system::stand_in::classify(trial: &Trial, _: &Trace)
  -> Result<TrialOutcome, SystemError>`, the same rules and numbers as
  `regression::stand_in_model` today (the body moves unchanged, with its
  source comment for 5 514 kg/m³). Module doc: provisional until the
  planet model's `class_of` is public; the CLI and the tests use it.
- Produces: `impl TracedFields for timeline::Orbit` (visits
  `semi_major_axis`, `eccentricity`, `period`, `perihelion_longitude`, in
  that order) and `impl TracedFields for timeline::Spin`
  (`rotation_period`, `obliquity`).

- [ ] **Step 1: Write the failing tests.** In timeline:
  `orbit_and_spin_visit_every_traced_field`: record 6 leaves in a `Trace`,
  build an `Orbit` and a `Spin` from them, collect the ids
  `for_each_node` visits, assert they are the 6 ids in field order; then
  freeze a `vec![orbit]` and assert the graph has 4 nodes. In system, a
  unit test in `stand_in.rs`: `stand_in_classes_follow_the_mass_steps`:
  masses 1, 5, 20, 100 at 1 au give `Rocky`, `SuperEarth`,
  `MiniNeptune`, `GasGiant`; 100 at 0.05 au gives `HotJupiter`.
- [ ] **Step 2: Run** `cargo nextest run -p timeline -p system --profile tier1`.
  Expected: compile failure (no impl, no module).
- [ ] **Step 3: Implement** the impls and move the stand-in.
- [ ] **Step 4: Run** `mise run test` (tiers 1 and 2). Expected: PASS,
  with the tier-2 determinism digests of `crates/regression` unchanged
  (the stand-in did not change).
- [ ] **Step 5: Commit**
  `refactor(system): move the stand-in classifier into the system crate`
  (the timeline impls may go in a separate
  `feat(timeline): trace orbits and spins as fields` commit).

### Task 2: Command-line skeleton and flags-to-`Inputs`

**Files:**

- Modify: `Cargo.toml` (`[workspace.dependencies]`:
  `clap = { version = "4.6.9", features = ["derive"] }`; `tools/guardrails`
  switches to `clap.workspace = true`)
- Modify: `Cargo.toml` (`getrandom = "0.4.3"`, already in `Cargo.lock`)
- Modify: the shared random-source bans, which list `getrandom::fill` and
  `getrandom::getrandom` but not `getrandom::u64`: add
  `getrandom::u32` and `getrandom::u64` (same reason text) to
  `clippy.toml`, `crates/adapters/clippy.toml`, `crates/cli/clippy.toml`
  and `SHARED_METHODS` in `tools/guardrails/src/clippy_config.rs:21`.
  Without it the `#[expect]` on the root seed draw is unfulfilled and
  `-D warnings` fails.
- Modify: `crates/cli/Cargo.toml` (clap, getrandom, serde, serde_json,
  science)
- Rewrite: `crates/cli/src/main.rs`
- Create: `crates/cli/src/args.rs`, `crates/cli/src/system.rs` (stub
  `run` returning `Ok(String::new())` until Task 3),
  `crates/cli/src/error.rs`

**Interfaces:**

- Produces (`args.rs`):

  ```rust
  #[derive(Parser)] #[command(version, about)]
  pub struct Cli { #[command(subcommand)] pub command: Command }
  #[derive(Subcommand)]
  pub enum Command { /// Draw a system … System(SystemCommand) }
  #[derive(Args)]
  pub struct SystemArgs {
      #[arg(long)] pub seed: Option<u64>,       // drawn when omitted
      #[arg(long)] pub present: Option<f64>,    // Gyr
      #[arg(long)] pub host_mass: Option<f64>,  // M☉
      #[arg(long)] pub metallicity: Option<f64>,// dex
      #[arg(long = "archetype", value_parser = parse_archetype)]
      pub archetypes: Vec<(ArchetypeId, u32)>,
  }
  impl SystemArgs {
      /// The given seed, or a root seed drawn from the OS (`drawn: true`).
      pub fn root_seed(&self) -> Result<RootSeed, CliError>
      pub fn inputs(&self, seed: Seed) -> system::Inputs
  }
  pub struct RootSeed { pub seed: Seed, pub drawn: bool }
  pub fn parse_archetype(text: &str) -> Result<(ArchetypeId, u32), String>
  ```

  `inputs` stays pure (the seed comes in), which differs from the
  `inputs(&self)` agreed with mapgen2: tell mapgen2 when the PR opens.
  `root_seed` is the only impure call, under
  `#[expect(clippy::disallowed_methods, reason = "the root seed of a run
  without --seed: the top of the seed hierarchy")]`; `getrandom` failing gives
  `CliError::Seed` (`cli.seed.unavailable: …`). `inputs()` sets
  `schema: INPUTS_SCHEMA`, `seed`,
  `present: present.map(Gyr)`, `request: Request::System(SystemRequest {
  host_mass, metallicity, archetypes })`, summing the counts of a repeated
  id into the `BTreeMap`. Range checks stay in the draw (it owns them);
  `parse_archetype` only checks the shape: one `=`, an id that
  `Code::new` accepts, a `u32` count.
- Produces (`system.rs`): `#[derive(Args)] pub struct SystemCommand {
  #[command(flatten)] pub system: SystemArgs, #[arg(long)] pub explain:
  Option<String> }` and `pub fn run(command: &SystemCommand, seed: Seed)
  -> Result<String, CliError>` (the text for stdout; `main` resolves the
  seed first).
- Produces (`error.rs`): `#[non_exhaustive] pub enum CliError {
  System(SystemError), Contract(ContractError), Json(serde_json::Error),
  Seed(getrandom::Error) }` with `Display` starting with its code:
  `SystemError`'s own text, `cli.contract: …`, `cli.json: …`,
  `cli.seed.unavailable: …`. `From` for each. Task 4 adds
  `Explain(ExplainError)`.
- `main.rs`: `Cli::parse()` (clap exits 2 on usage errors), resolve the
  root seed (print the stderr note when drawn), dispatch,
  print `Ok` text to stdout with a trailing newline, `Err(e)` as
  `map-generator: {e}` on stderr, `ExitCode::from(1)`.
- Flag help texts give the unit and range: `--host-mass` "in M☉, 0.08-8";
  `--present` "the star's age in Gyr [default: drawn]"; `--archetype`
  "ID=COUNT, repeatable; ids: earth-like, …" (list the 14 ids of
  `crates/system/src/archetype.rs:100`).

- [ ] **Step 1: Write the failing unit tests** in `args.rs`:
  - `inputs_carry_every_flag`: `Cli::try_parse_from(["map-generator",
    "system", "--seed", "7", "--present", "4.5", "--host-mass", "1.1",
    "--metallicity", "-0.2", "--archetype", "earth-like=2"])` gives
    `root_seed()` = `RootSeed { seed: Seed::new(7), drawn: false }` and
    `inputs(Seed::new(7))` =
    `Inputs { schema: INPUTS_SCHEMA, seed: Seed::new(7), present:
    Some(Gyr(4.5)), request: System { host_mass: Some(SolarMass(1.1)),
    metallicity: Some(Dex(-0.2)), archetypes: {earth-like: 2} } }`.
  - `repeated_archetypes_add_up`: `earth-like=1` and `earth-like=2` give 3.
  - `malformed_archetypes_are_usage_errors`: each of `earth-like`,
    `earth-like=x`, `Earth-Like=1`, `=2`, `earth-like=-1` makes
    `try_parse_from` fail with `ErrorKind::ValueValidation`.
  - `an_omitted_seed_is_drawn`: without `--seed`, parsing succeeds and
    `root_seed()` gives `drawn: true` (two calls give different seeds,
    which fails with probability 2^-64).
- [ ] **Step 2: Run** `cargo nextest run -p map-generator`. Expected: FAIL
  (types missing).
- [ ] **Step 3: Implement** `args.rs`, `error.rs`, `main.rs`, the stub.
- [ ] **Step 4: Run** `cargo nextest run -p map-generator` and
  `cargo clippy -p map-generator --all-targets -- -D warnings`. Expected:
  PASS, no warning.
- [ ] **Step 5: Commit** `feat(cli): parse the system command's flags`.

### Task 3: The draw as JSON

**Files:**

- Create: `crates/cli/src/report.rs`
- Modify: `crates/cli/src/system.rs` (`run` without `--explain`)
- Create: `crates/cli/tests/system.rs`
- Modify: `README.md` § Usage (replace "gains its first command … Until
  then" with a two-line example and a pointer to `crates/cli/README.md`),
  `crates/cli/README.md` (command, flags, output shape, exit codes, the
  stand-in, the drawn root seed and how to replay it)

**Interfaces:**

- Consumes: Task 1's `system::stand_in::classify`, `TracedFields` for
  `Orbit`/`Spin`; Task 2's `SystemArgs::inputs(seed)`,
  `run(command, seed)`, `CliError`.
- Produces (`report.rs`):

  ```rust
  pub const REPORT_SCHEMA: u32 = 1;
  #[derive(Serialize)]
  pub struct SystemReport {
      pub schema: u32,              // REPORT_SCHEMA
      pub inputs: Inputs,           // as given, to rerun it
      pub engine: EngineVersion,    // system::ENGINE_VERSION
      pub family: &'static str,     // Family::code()
      pub classifier: &'static str, // "stand-in"
      pub star: Timeline,           // one keyframe, the present
      pub planets: Vec<PlanetDraw>, // in the draw's order
      pub causes: CauseGraph,       // of `planets`, frozen
  }
  #[derive(Serialize)]
  pub struct PlanetDraw {
      pub designation: Designation, pub seed: Seed,
      pub archetype: Option<ArchetypeId>,
      pub mass: Traced<EarthMass>, pub orbit: Orbit, pub spin: Spin,
      pub sources: StellarSources,
  }
  impl TracedFields for PlanetDraw   // mass, orbit, spin, sources
  pub fn report(inputs: &Inputs) -> Result<SystemReport, CliError>
  ```

  `report` makes a `Trace`, calls `draw_with(inputs, &mut trace,
  stand_in::classify)`, builds the star timeline as
  `Timeline::new(drawn.designation, drawn.seed, ENGINE_VERSION, present,
  vec![Keyframe::new(present, BodyState::Star(drawn.star), &trace)?])`
  with `present = *drawn.age.value()`, maps each `Placement` to a
  `PlanetDraw` (from `placement.trial` for mass, orbit, sources and
  archetype), then `causes = trace.freeze(&mut planets)?`. `run` prints
  `serde_json::to_string_pretty(&report)`.

Tests that index `/planets/0` assume seed 1 draws at least one planet;
the first unit test asserts it, so a change of draw shows up there first.

- [ ] **Step 1: Write the failing tests.** Unit, in `report.rs`:
  - `the_star_timeline_has_one_keyframe_at_the_present`: seed 1; at
    least one planet; `star.keyframes.len() == 1` and
    `star.keyframes[0].date() == star.present`; with `--present 4.5
    --host-mass 1`, `star.present == Gyr(4.5)`.
  - `every_planet_node_is_in_the_planet_graph`: seed 1; for each
    planet, every id `for_each_node` visits is `< causes.nodes.len()`,
    and the graph validates (`causes.validate()`).
  - `asked_archetypes_are_placed`: `--archetype earth-like=1` (via
    `inputs(Seed::new(3))`); one planet has
    `archetype == Some(earth-like)`.

  Integration, in `crates/cli/tests/system.rs` (spawn the binary):
  - `prints_the_draw_as_json`: `system --seed 1` exits 0; stdout parses
    as JSON with keys `schema` (1), `inputs`, `engine`, `family`,
    `classifier` (`"stand-in"`), `star`, `planets`, `causes`; stderr empty.
  - `the_same_seed_prints_the_same_bytes`: two runs of `--seed 42`
    give identical stdout.
  - `a_date_after_the_star_dies_fails`: `--seed 1 --host-mass 1
    --present 50` exits 1,
    stdout empty, stderr starts with
    `map-generator: system.present-after-life`.
  - `an_unknown_archetype_fails`: `--archetype pluto-like=1` exits 1,
    stderr contains `system.unknown-archetype`.
  - `a_malformed_archetype_is_a_usage_error`: `--archetype earth-like`
    exits 2.
  - `an_omitted_seed_is_drawn_and_replayable`: `system` alone exits 0;
    stderr is `map-generator: seed N drawn; rerun with --seed N`; stdout's
    `inputs.seed` is N; `system --seed N` prints the same stdout, with an
    empty stderr.
- [ ] **Step 2: Run** `cargo nextest run -p map-generator`. Expected: FAIL.
- [ ] **Step 3: Implement** `report.rs` and `run`; write the docs.
- [ ] **Step 4: Run** `cargo nextest run -p map-generator`, clippy as in
  Task 2, `mise run md:lint`, and `cargo run -q -p map-generator --
  system --seed 1 | head -40` to look at the shape. Expected: PASS,
  JSON readable.
- [ ] **Step 5: Commit** `feat(cli): print a drawn system as JSON`.

### Task 4: `--explain`

**Files:**

- Create: `crates/cli/src/explain.rs`
- Modify: `crates/cli/src/system.rs`, `crates/cli/src/error.rs`
- Modify: `crates/cli/tests/system.rs`, `crates/cli/README.md`
  (`--explain` section with one example of output)

**Interfaces:**

- Consumes: Task 3's `report`.
- Produces:

  ```rust
  #[non_exhaustive]
  pub enum ExplainError {
      NotFound { path: String },                     // cli.explain.not-found
      NotTraced { path: String },                    // cli.explain.not-traced
      Ambiguous { code: String, pointers: Vec<String> }, // cli.explain.ambiguous
  }
  pub fn explain(report: &serde_json::Value, path: &str) -> Result<String, ExplainError>
  ```

  Resolution, on the report serialised to a `serde_json::Value`:
  - A **traced value** is a JSON object with exactly the keys `value` and
    `node` (how `Traced<T>` serialises). Its **graph** is the `causes`
    array of its nearest ancestor object that has a `causes` key (the
    star keyframe's own graph, or the report's for planets).
  - `path` starting with `/` is a JSON pointer (`Value::pointer`):
    missing gives `NotFound`, not a traced value gives `NotTraced`.
  - Otherwise `path` is a cause code: collect every node of every graph
    (the report's `causes` and each keyframe's) whose `code` equals it,
    render each tree, and drop identical renderings (a star node is in
    both graphs). One left: print it. None: `NotFound`. Several:
    `Ambiguous` with the pointers of the traced values whose node has
    that code (walk the JSON depth-first, in key order).
  - Rendering: one line per node, two spaces of indent per depth,
    `"{code} = {value}{ unit} [{provenance}]"` for a leaf (`input`,
    `observed`, `drawn`, `imposed`) and `"{code} = {value}{ unit}"` for a
    derived node followed by its sources in `from` order. A `Code` value
    prints its text, a flag `true`/`false`, a number with Rust's `{}`. A
    node already printed in this tree prints as `"{code} (see above)"`
    without its sources.
  - `run` with `--explain` builds the report, converts it with
    `serde_json::to_value`, calls `explain`, prints its text (no JSON).

- [ ] **Step 1: Write the failing tests.** Unit, in `explain.rs` (on a
  report built for seed 1, or a hand-made `json!` for the edge cases):
  - `a_pointer_to_a_semi_major_axis_explains_it`: pointer
    `/planets/0/orbit/semi_major_axis`; the first line starts with
    `system.draw.semi-major-axis = `, and some line contains
    `system.star.mass`; every line without sources ends with `]` or
    `(see above)`.
  - `a_unique_code_explains_it`: `system.star.mass` gives a single tree
    starting with `system.star.mass = ` (the star node is in both graphs
    and collapses).
  - `a_repeated_code_is_ambiguous`: for a seed with at least 2 planets
    (search seeds 1.. in the test), `system.draw.flux` gives `Ambiguous`
    whose pointers all start with `/planets/` and number the planets.
  - `a_pointer_to_a_designation_is_not_traced`: `/planets/0/designation`
    gives `NotTraced`.
  - `unknown_paths_are_not_found`: `/planets/99/mass` and
    `system.draw.nothing` give `NotFound`.
  - `a_shared_source_prints_once`: a `json!` graph where node 2 derives
    from 0 and 1, and 1 from 0: node 0 prints once, then
    `(see above)`.

  Integration: `explain_prints_a_tree`: `system --seed 1 --explain
  /planets/0/orbit/period` exits 0 and stdout starts with
  `system.draw.period = `; `explain_errors_exit_1`: `--explain
  /planets/0/designation` exits 1, stderr starts with
  `map-generator: cli.explain.not-traced`.
- [ ] **Step 2: Run** `cargo nextest run -p map-generator`. Expected: FAIL.
- [ ] **Step 3: Implement** `explain.rs`, `CliError::Explain` (`From`
  and `Display`), `run`'s `--explain` branch, and the README section (one example of output, generated
  by running the command).
- [ ] **Step 4: Run** `mise run ci`. Expected: PASS within the tier
  budgets, markdownlint clean.
- [ ] **Step 5: Commit** `feat(cli): explain a drawn value with --explain`.

## After the tasks

`branch-reviewer` on the whole branch against the spec and this plan,
then the PR into `main` (`Refs #602`), then `develop` per CLAUDE.md.
