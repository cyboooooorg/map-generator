# `system::generate` design

Rewrite slice 1 (#602), track T0 of the roadmap (#427). Written by
mapgen3 on 2026-11-27, after the planet model PR 2d-4 (#780) merged.

## Intent

The system engine draws a system (`draw_with`) and models each planet
(`planet::model`), but nothing hands out the engine's product: one
`Timeline` per body. Three places assemble the draw and the model by
hand today:

- `crates/cli/src/report.rs:62` (`cli system`): a star timeline, then
  planets as `PlanetDraw` with one shared cause graph (report schema 2);
- `crates/regression/src/lib.rs:95` (reference systems): the drawn
  system plus bare planet states;
- nothing for `world`: mapgen2's `world` subcommand (W2 pull request B)
  waits on `generate` and assumes its exact signature
  (`docs/superpowers/specs/2026-11-22-cli-world-design.md`, injected
  producer `impl FnOnce(&Inputs) -> Result<Vec<Timeline>, _>`).

This change writes `generate` once and makes the report and the
regression references read it, so a single code path goes from the
inputs to the timelines.

## Goals

- `pub fn generate(inputs: &Inputs) -> Result<Vec<Timeline>, SystemError>`
  in `crates/system/src/engine.rs`, the signature announced there.
- One assembly path: `cli system` and `crates/regression` call the same
  function as `generate`.
- `generate` calls `planet::model` exactly once per placement, and the
  classifier exactly as the draw does today (no added cost; #771 covers
  the existing double solve inside the model).

## Non-goals

- Keyframes other than the present (T4, #606).
- Removing the double solve between `class_of` and `model` (#771).
- The `world` subcommand (mapgen2, W2 pull request B).
- Any change to the draw or the planet model: no `ENGINE_VERSION`
  bump, the regression digests stay as they are.

## Anti-goals

- A second assembly loop left anywhere (report, regression, tests).
- Calling `model` or `class_of` more often than today.
- Timelines that `world::run` cannot read: the present keyframe must be
  dated exactly at `timeline.present`, and a planet's `timeline.seed`
  must be its body seed (the relief derives from it).

## Approaches

- **A. `generate` alone**, report and regression unchanged. Smallest
  pull request, but leaves the draw-then-model loop in three places.
- **B. `generate` plus the report on it** (report schema 3). Removes
  the report's loop; the regression loop stays because it injects a
  logging classifier and keeps the `Drawn` for its digest.
- **B with `generate_with`** (chosen by the user): a public layer under
  `generate` takes the classifier and returns the drawn system with the
  timelines. `generate`, the report and the regression all go through
  it, so no loop is duplicated.

## Design

### API (`crates/system/src/engine.rs`)

```rust
/// What one generation produced: the draw and one timeline per body.
pub struct Generated {
    pub drawn: Drawn,
    /// The system's live trace: the draw's nodes, then the model's.
    /// `drawn`'s traced values (star, trial masses) index into it.
    pub trace: Trace,
    pub timelines: Vec<Timeline>,
}

pub fn generate_with(
    inputs: &Inputs,
    classify: impl Fn(&Trial, &Trace) -> Result<TrialOutcome, SystemError>,
) -> Result<Generated, SystemError>;

pub fn generate(inputs: &Inputs) -> Result<Vec<Timeline>, SystemError> {
    generate_with(inputs, planet::class_of).map(|g| g.timelines)
}
```

Both are re-exported from `system` (`system::generate`,
`system::generate_with`, `system::Generated`).

### Assembly

1. One `Trace` for the system; `draw_with(inputs, &mut trace, classify)`.
   A `Request::Body` fails there with `BodyRequestNotDrawn`, unchanged.
2. `model(&placement, &mut trace)` for each placement, in placement
   order, once each.
3. Only then the keyframes, so every keyframe freezes the complete
   trace: `Keyframe::new(present, state, &trace)`, with
   `present = *drawn.age.value()` (equal to every `trial.age`).
   `Trace::freeze` takes `&self`: freezing the one trace once per body
   is safe, leaves it unchanged, and copies shared ancestors into each
   keyframe's graph. The star state is cloned from `drawn.star`, so
   `drawn` stays whole.
4. Timelines: the star first (`drawn.designation`, `drawn.seed`), then
   one per planet in placement order (`placement.designation`,
   `placement.seed`), each with `ENGINE_VERSION`, `present` and its one
   keyframe.

`Drawn` stays whole in `Generated` (placements included) for the
callers that need the draw's values: family, archetype, trial mass,
digest. Its node ids are valid against `Generated.trace` only; the
states inside the timelines are renumbered against their own
keyframe's `causes()`.

### Report schema 3 (`crates/cli/src/report.rs`)

```text
{ schema: 3, inputs, engine, family,
  timelines: [Timeline, …],          // star first, then planets
  planets: [{ designation, archetype, mass }] }
```

- `mass` is the drawn trial mass (`Traced<EarthMass>`). Its cause node
  lives in the draw's trace (`Generated.trace`), which no keyframe
  holds; it is frozen into a small report-level `causes` graph kept for
  the `planets` list only.
  The body seed is already in the planet's timeline.
- The single report-wide graph for the planet states goes: each
  keyframe carries its own.
- `--explain` (`crates/cli/src/explain.rs`) already resolves a pointer
  against the nearest ancestor holding `causes`; `graphs()` reads every
  keyframe of every timeline instead of `/star/keyframes` only, so a
  cause code is searched in all graphs. A code found in several graphs
  keeps today's rule: the error lists the pointers and asks for one.
- Pointers change: `/planets/0/state/orbit/period` becomes
  `/timelines/1/keyframes/0/state/planet/orbit/period` (exact path
  follows `BodyState`'s serde shape).

### Regression (`crates/regression/src/lib.rs`)

`references()` calls `generate_with` with its logging classifier
around `class_read`. `Reference` keeps `inputs`, `drawn`, `trace`
(from `Generated`, for the draw's nodes) and `reads`, and replaces
`planets: Vec<PlanetState>` with `timelines: Vec<Timeline>` plus an
accessor that yields, per placement, the present keyframe's planet
state with its own `CauseGraph`. Tests read planet states and walk
their causes through that pair (`planets.rs:418-495`: `r.trace.node`
and `ancestors_coded(&r.trace, …)` move to the keyframe's graph), so
the regression checks what a consumer of the timelines sees. The
goldens (`EXPECTED` digests, pinned classes, climates, liquids, life)
must not change.

### Tests

- Tier 2, on the shared references (no system generated of its own):
  one timeline per body, star first, designations and body seeds as in
  `drawn`, one keyframe dated at `present`, every keyframe valid, and
  the planet states equal to what the previous regression loop
  produced (the pinned values staying green is that proof).
  Determinism: `generate` twice on one reference gives equal output.
- `generate` (with `class_of`) and `generate_with(…, class_of)` agree:
  checked on the reference whose report the CLI tests already build, so
  no extra generation.
- `crates/cli/tests/system.rs` and `crates/regression/tests/cli.rs`
  move to schema 3 and the new pointers.
- Tier budgets: the regression references are generated once per
  process as today; the extra cost is the keyframe freezes only.

### Consumers of the report

Only `cli system`, `--explain` and the tests read the report. The
`web/` fixtures are legacy (`format_version` 1) and do not read it; no
server runs during the engine phase. The schema bump has no other
reader.

### Documentation

- `crates/system/src/engine.rs` doc (replaces "Future signature"),
  `crates/system/README.md`.
- `crates/cli/README.md` and `README.md`: report schema 3, the
  `--explain` example pointer.
- `docs/superpowers/specs/2026-11-22-cli-world-design.md`: `generate`
  exists, signature unchanged.

### Coordination

- mapgen2: the signature is the announced one; `Generated` and
  `generate_with` are additions. Told when the pull request opens.
- mapgen1: `planet/**` is not touched. `regression/src/lib.rs` changes
  (shared crate): told before the pull request.
- No `develop` rebuild without telling mapgen1 and mapgen2 first.

## Left to the builder

- The exact `BodyState` serde path in the new `--explain` pointers.
- Whether `report`'s error type needs a new variant (it should not:
  `CliError: From<SystemError> + From<ContractError>`).

## Verification

`mise run ci`. No `mise run test:heavy`: the engine's output does not
change (same draw, same model, same `ENGINE_VERSION`); the regression
digests and pins are the check. Then `branch-reviewer` on the whole
branch, then the pull request; no merge without the user's approval.
