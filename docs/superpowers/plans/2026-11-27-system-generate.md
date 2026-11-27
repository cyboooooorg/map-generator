# `system::generate` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** One function turns `Inputs` into one `Timeline` per body, and
the report and the regression references go through it.

**Architecture:** `system::engine::generate_with` draws, models each
placement once, then builds the present keyframes; `generate` is it with
`planet::class_of`. `cli system` (report schema 3) and
`regression::references()` call `generate_with`.

**Tech Stack:** Rust workspace (`crates/system`, `crates/timeline`,
`crates/cli`, `crates/regression`), nextest tiers, `mise run ci`.

**Spec:** `docs/superpowers/specs/2026-11-27-system-generate-design.md`

## Global Constraints

- Signature, exact: `pub fn generate(inputs: &Inputs) -> Result<Vec<Timeline>, SystemError>`.
- `model` is called exactly once per placement; the classifier only by
  `draw_with`.
- No change to `draw/**`, `planet/**`, `ENGINE_VERSION` (stays 4) or the
  regression `EXPECTED` digests and pinned values.
- Timelines: star first, then planets in placement order; each has
  `engine: ENGINE_VERSION`, `present = *drawn.age.value()`, exactly one
  keyframe dated `present`; planet `seed` = `placement.seed`, star
  `seed` = `drawn.seed`.
- Every keyframe is built after the last `model` call.
- No test generates a system of its own outside tier 3: new tests use
  `regression::references()` or the CLI tests' reference 1.
- `REPORT_SCHEMA` goes from 2 to 3.

## Review Focus

- A `Request::Body` input: `generate` returns `SystemError::BodyRequestNotDrawn`,
  no panic (covered by an existing draw path; Task 1 adds the assertion
  on `generate`'s error without drawing a system).
- A system with zero planets: `generate` returns the star timeline only
  (Task 1 asserts `timelines.len() == 1 + placements.len()` on every
  reference, which includes it when one has none).
- `world::run` reading the result: the planet's present keyframe is
  found by `date() == timeline.present` (Task 1 asserts it).
- `--explain <code>` with the code present in several keyframes: lists
  the pointers (Task 2, `a_repeated_code_is_ambiguous`).
- Node ids of `drawn` stay valid against `Generated.trace` (Task 1:
  `trace.node(placement.trial.mass.node())` is `Some`).

---

### Task 1: `generate_with`, `generate`, and the regression references on it

**Files:**
- Modify: `crates/system/src/engine.rs` (doc only today), `crates/system/src/lib.rs` (re-exports), `crates/system/README.md`
- Modify: `crates/regression/src/lib.rs:64-118` (`Reference`, `references()`)
- Modify: `crates/regression/tests/planets.rs` (every `r.planets`, `r.trace.node`, `ancestors_coded(&r.trace, …)` on planet-state ids)
- Create: `crates/regression/tests/generate.rs`

**Interfaces:**
- Produces (in `system`, re-exported at the crate root):

  ```rust
  pub struct Generated { pub drawn: Drawn, pub trace: Trace, pub timelines: Vec<Timeline> }
  pub fn generate_with(
      inputs: &Inputs,
      classify: impl Fn(&Trial, &Trace) -> Result<TrialOutcome, SystemError>,
  ) -> Result<Generated, SystemError>;
  pub fn generate(inputs: &Inputs) -> Result<Vec<Timeline>, SystemError>;
  ```

  `Generated` derives `Debug`. Its doc says `drawn`'s node ids index
  `trace`; timeline states are renumbered against their keyframe's
  `causes()`.
- Produces (in `regression`): `Reference { inputs, drawn, trace, timelines: Vec<Timeline>, reads }`
  (`planets` removed) and

  ```rust
  impl Reference {
      /// Per placement, in order: the present keyframe's planet state and its graph.
      pub fn planets(&self) -> impl Iterator<Item = (&PlanetState, &CauseGraph)>;
  }
  ```

- [ ] **Step 1: Write the failing tests** in `crates/regression/tests/generate.rs` (tier 2):
  - `one_timeline_per_body_star_first`: for every `r` in `references()`,
    `r.timelines.len() == 1 + r.drawn.placements.len()`;
    `r.timelines[0].body == r.drawn.designation`, `seed == r.drawn.seed`,
    its keyframe state is `BodyState::Star`; for each `(i, p)`,
    `r.timelines[i + 1].body == p.designation`, `.seed == p.seed`.
  - `each_timeline_has_its_present_keyframe`: every timeline has
    `engine == ENGINE_VERSION`, `present == *r.drawn.age.value()`,
    `keyframes.len() == 1`, `keyframes.iter().find(|k| k.date() == t.present).is_some()`,
    and `keyframe.validate()` is `Ok`.
  - `the_draw_s_ids_index_the_trace`: for every placement,
    `r.trace.node(p.trial.mass.node()).is_some()`.
  - `generate_is_generate_with_class_of`: on `reference_requests()[0]`,
    `system::generate(&inputs).unwrap() == system::generate_with(&inputs, class_of).unwrap().timelines`,
    and `generate` twice gives equal output (determinism). Two
    generations of reference 1: note the time in the commit body; if it
    exceeds 5 s release, compare against `references()[0].timelines`
    instead (same classifier outcome; the logging closure returns
    `read.outcome`).
  - `a_body_request_is_not_drawn`: `generate` on an `Inputs` with
    `Request::Body(..)` returns `Err(SystemError::BodyRequestNotDrawn)`
    (fails before any draw, so it generates nothing).

- [ ] **Step 2: Run them to see them fail**

  Run: `cargo nextest run -p regression --test generate`
  Expected: compile error (`generate_with`, `Reference::timelines` missing).

- [ ] **Step 3: Implement `generate_with` and `generate` in `crates/system/src/engine.rs`**

  Order: `Trace::new()`; `draw_with(inputs, &mut trace, classify)?`;
  `model(p, &mut trace)?` for each placement into a `Vec<PlanetState>`;
  then the star timeline (`BodyState::Star(drawn.star.clone())`), then
  one timeline per placement; `Keyframe::new(present, state, &trace)?`
  and `Timeline::new(..)?` (`SystemError: From<ContractError>`). Replace
  the "Future signature" doc with what the functions do and the
  invariants of the Global Constraints. Re-export `Generated`,
  `generate`, `generate_with` from `lib.rs`.

- [ ] **Step 4: Move `references()` onto `generate_with`**

  Keep the logging closure as is; destructure `Generated { drawn, trace, timelines }`
  into `Reference`. Implement `Reference::planets()` by reading
  `timelines[1..]`, each `keyframes[0]`, matching `BodyState::Planet`
  (panic with the designation otherwise). Update the doc comments of
  `Reference` and `references()`.

- [ ] **Step 5: Port `crates/regression/tests/planets.rs`**

  `planets()` zips placements with `r.planets()` and yields the graph
  too where a test walks causes. `r.trace.node(state_id)` becomes
  `graph.node(state_id)`; `ancestors_coded` takes `&CauseGraph` (both
  `Trace::node` and `CauseGraph::node` return `Option<&Node>`). Ids that
  come from `drawn` keep using `r.trace`. No expected value changes.

- [ ] **Step 6: Run the regression tier and the new tests**

  Run: `cargo nextest run -p regression --profile tier2`
  Expected: PASS, `golden_digests` and `the_reference_planets_are_pinned` unchanged.

- [ ] **Step 7: Document** `crates/system/README.md`: the engine's entry
  points (`generate`, `generate_with`, `Generated`), replacing any
  mention of the future signature.

- [ ] **Step 8: Commit**

  ```bash
  git add crates/system crates/regression
  git commit -m "feat(system): generate one timeline per body"
  ```

  Body: what `generate_with` returns, regression moved onto it,
  `Refs: #602`.

### Task 2: report schema 3 on `generate_with`, `--explain` over every keyframe

**Files:**
- Modify: `crates/cli/src/report.rs` (whole `report` and the structs)
- Modify: `crates/cli/src/explain.rs:118-129` (`graphs`), its doc at lines 1-10, its unit tests if they build a schema-2 value
- Modify: `crates/cli/src/system.rs` (help text naming pointers, if any)
- Modify: `crates/regression/tests/cli.rs`, `crates/cli/tests/system.rs:70`
- Modify: `crates/cli/README.md`, `README.md:46`, `docs/superpowers/specs/2026-11-22-cli-world-design.md` (`generate` exists)

**Interfaces:**
- Consumes: `system::{generate_with, Generated}` from Task 1.
- Produces:

  ```rust
  pub const REPORT_SCHEMA: u32 = 3;
  pub struct SystemReport {
      pub schema: u32, pub inputs: Inputs, pub engine: EngineVersion,
      pub family: &'static str,
      /// One per body: the star, then the planets closest first.
      pub timelines: Vec<Timeline>,
      /// What the draw decided of each planet, in `timelines[1..]`'s order.
      pub planets: Vec<PlanetDraw>,
      /// The causes of `planets` (the drawn masses).
      pub causes: CauseGraph,
  }
  pub struct PlanetDraw { pub designation: Designation, pub archetype: Option<ArchetypeId>, pub mass: Traced<EarthMass> }
  ```

- [ ] **Step 1: Update the tests to schema 3** in `crates/regression/tests/cli.rs`:
  - `prints_the_draw_as_json`: keys `["inputs", "engine", "family", "timelines", "planets", "causes"]`,
    no `star`; `json["schema"] == 3`; each `planets[i]` has
    `designation`, `archetype`, `mass` and no `state` or `seed`.
  - `the_star_timeline_has_one_keyframe_at_the_present` reads
    `report.timelines[0]`; `a_fixed_present_is_the_star_s_present`
    checks every timeline's `present`.
  - `every_planet_node_is_in_the_planet_graph`: every node of
    `report.planets[i].mass` is in `report.causes`, and every node of
    each planet keyframe's state is in that keyframe's `causes()`.
  - `a_pointer_to_a_period_explains_it`: pointer
    `/timelines/1/keyframes/0/state/<planet variant key>/orbit/period`
    (read the serde shape of `BodyState` for the variant key), same
    expected text as today.
  - `a_repeated_code_is_ambiguous`: count planets from
    `report["timelines"].len() - 1`; the error still lists one pointer
    per planet.
  - `crates/cli/tests/system.rs:70`: `/planets/0/designation` still
    exists in schema 3; keep the test, check it still exits 1.

- [ ] **Step 2: Run them to see them fail**

  Run: `cargo nextest run -p regression --test cli`
  Expected: FAIL (schema 2 shape).

- [ ] **Step 3: Rewrite `report`** on `generate_with(inputs, class_of)?`:
  build `planets` from `drawn.placements` (designation, archetype,
  trial mass), `causes = trace.freeze(&mut planets)?`, move `timelines`
  in. `PlanetDraw` implements `TracedFields` over `mass` only.

- [ ] **Step 4: `graphs()` reads every timeline's keyframes**
  (`/timelines/*/keyframes/*`) plus the report's own graph. Update the
  module doc (lines 1-10) for the new shape.

- [ ] **Step 5: Run the CLI tests**

  Run: `cargo nextest run -p regression --test cli && cargo nextest run -p cli`
  Expected: PASS.

- [ ] **Step 6: Documentation**: `README.md:46` and `crates/cli/README.md`
  examples use the schema-3 pointer and describe the report keys;
  `2026-11-22-cli-world-design.md` says `system::generate` exists (one
  line, signature unchanged).

- [ ] **Step 7: Commit**

  ```bash
  git add crates/cli crates/regression README.md docs
  git commit -m "feat(cli)!: print one timeline per body in the system report"
  ```

  Footer: `BREAKING CHANGE: the system report is schema 3: timelines replaces star, planets keep designation, archetype and mass; --explain pointers move under /timelines.` and `Refs: #602`.

### Task 3: Verification

- [ ] **Step 1:** `mise run ci` in `~/Developer/mg-system-generate`. Expected: PASS within the tier budgets (tier 2 ≤ 480 s).
- [ ] **Step 2:** `branch-reviewer` and `test-reviewer` on `feat/system-generate` with the spec and this plan, then `issue-auditor` with their reports and the draft PR text. Fix, re-run `mise run ci`.
- [ ] **Step 3:** Tell mapgen1 and mapgen2 before any `develop` rebuild; ask the user before opening the PR.
