# Collapse step rules Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Cut the evaluations of a CO₂ collapse by letting its step grow
to 5 % of the CO₂ while the gap rises, and go to the secant's zero once
it falls.

**Architecture:** One function changes: `collapse_search` in
`crates/system/src/planet/climate/wet.rs`. `COLLAPSE_STEP` keeps its name
and its `widest` parameter, but now caps only the steps taken while `g`
rises or stays flat, and moves from 0.005 to 0.05. On a falling `g`, the
secant step is bounded only by twice the last step (and by `c`). Unit
tests on synthetic gaps pin both rules and the accepted limit. A tier-3
test holds the new step at the cool-star frozen climates of #731.

**Tech Stack:** Rust, crate `system`, cargo tests (tier 3: `#[ignore = "heavy: …"]`, release).

**Spec:** `docs/superpowers/specs/2026-11-27-collapse-step-rules-design.md`

## Global Constraints

- Worktree `~/Developer/mg-thaw`, branch `perf/thaw-search`, based on main a97c1ca1.
- Throttled tests: `RAYON_NUM_THREADS=4 cargo test … -- --test-threads=3`, one full run at a time (machine shared with other sessions).
- `COLLAPSE_STEP` = 0.05 (rising cap). Falling step: `min(secant, 2 × last step)`, and `< c`.
- `widest == 0` still steps plainly, as legacy did.
- The noise rule of #743 is unchanged: until `g` has risen above its noise, a step is at least the noise, at most `widest × c`.
- Every constant keeps its `Calibration:` or `Definition:` doc with its source.
- Commits follow Conventional Commits; the probes in the scratchpad (`probes.patch`) are never committed.
- No `FORMAT_VERSION` or `ENGINE_VERSION` bump: outside the tests nothing calls the climate yet (`planet::settled` has no other caller; it gets wired in PR 2d-4), so no stored output changes.

---

### Task 0: Commit the spec and the plan

- [ ] **Step 1:** `git add docs/superpowers/specs/2026-11-27-collapse-step-rules-design.md docs/superpowers/plans/2026-11-27-collapse-step-rules.md && git commit -m "docs: plan faster CO2 collapse step rules"`

## Review Focus

1. A warm balance wider than a rising step: the search must still stop at its upper root (test in Task 1).
2. A warm balance narrower than a rising step: stepped over, the accepted limit; pinned by a test so the limit is visible (Task 1).
3. A falling gap with tiny slope (`g0` barely above `g`, just past the hump's peak): the secant is huge, and only the `2 × last` bound holds it. The hump test goes through its peak, so it covers this case, but no test isolates it.
4. A gap that jumps across its root by its noise: `a_collapse_stops_where_its_gap_jumps_across_a_root` must still pass unchanged (Task 1).
5. A cycling frozen climate at 4500 K: the new step must not change its branch (heavy test, Task 2).

---

### Task 1: Two step rules in `collapse_search`

**Files:**
- Modify: `crates/system/src/planet/climate/wet.rs` (`collapse_search`, about l.240-330, and its doc comment)
- Modify: `crates/system/src/planet/climate/mod.rs:104-118` (`COLLAPSE_STEP` value and doc)
- Test: `crates/system/src/planet/climate/wet_tests.rs` (synthetic collapse tests, l.373-545)

**Interfaces:**
- Consumes: nothing new.
- Produces: `collapse_search(start, solve, tolerance, most, widest)`, same signature; `widest` is now the rising cap. `COLLAPSE_STEP: f64 = 0.05`.

- [ ] **Step 1: Update the warm-balance test, add the accepted-limit test and the hump test**

In `wet_tests.rs`:

- `a_collapse_stops_at_a_warm_balance_wider_than_its_step`: its dip becomes `(4.0, 15.4, 0.5)`. `g` is then negative from 16.515 to 15.110 bar (1.4 bar), wider than a rising step there (`COLLAPSE_STEP × 16.5` = 0.83 bar), and away from the fold. A dip centred higher (16.0 or above, half-width 0.4 or more) already makes `g` negative at the fold and tests nothing. Simulated: plain stops at 16.5252 in 78 evaluations, the new rules at 16.5187 in 10. Its assertions: `plain` within 0.01 of 16.525; `fast` within 0.01 of `plain`; `synthetic_gap(fast) >= −SYNTHETIC_NOISE`; `fast_steps * 3 < plain_steps`. Add an assertion that `g` is negative over more than `COLLAPSE_STEP × 16.515` (sample `synthetic_gap` every 0.001 bar), so the test keeps its meaning if the constant moves.
- New `a_collapse_steps_over_a_warm_balance_narrower_than_its_step`: the old dip `(0.15, 16.6, 0.05)`. Assert `plain` stops at about 16.67 and `fast` reaches the cold balance (`(fast − 0.6).abs() < 1e-3`). Its comment says this is the limit the user accepted on 2026-11-27 (#731).
- New `a_falling_gap_closes_on_its_root_by_the_secant`: a hump in the shape traced at 3300 K (#731), from 45.9 to a root at 2.2 bar:

  ```rust
  let gap = |c: f64| 0.00605 * (c - 2.2) * (46.0 - c);
  ```

  It is 0.026 bar at the start and peaks at 2.9 bar near 24 bar, as traced. Its noise is `0.0016 + 0.0097 * c / 45.9`. Assert the search holds, lands within 0.01 bar of 2.2, and takes at most 25 evaluations. Simulated: 21 with the new rules, 49 with the old ones, 62 plain.

- [ ] **Step 2: Run the unit tests and see them fail**

Run: `cd ~/Developer/mg-thaw && RAYON_NUM_THREADS=4 cargo test -p system --lib climate::wet_tests -- --test-threads=3`
Expected: the warm-balance test passes (the old step stops there too), the narrower-balance test FAILS (`fast` stops at 16.675 with the 0.005 step), and the hump test FAILS on its evaluation count (about 49 > 25).

- [ ] **Step 3: Implement the two rules**

In `collapse_search`, the accelerated step when `before` is set becomes:

```rust
let longer = if g0 > g {
    // Falling: to the secant's zero, at most twice the last step.
    secant.min(2.0 * last_step)
} else {
    // Rising or flat: doubling, at most `widest` of the CO₂.
    (2.0 * last_step).min(widest * c)
};
```

The rest stays (`longer > step && longer < c`, the noise rule, the bracket). Set `COLLAPSE_STEP` to 0.05. Rewrite the doc comments of `collapse_search` ("Otherwise (#731) …" paragraph) and `COLLAPSE_STEP` to state both rules, the evaluations measured (83 → 24 at 4500 K, 44 → 22 at 3300 K, simulated on traced gaps), and the limit: a warm balance narrower than a rising step, at most 5 % of the CO₂, or hidden before the secant's zero, can be stepped over. Remove the half-step measurements of the old value from `COLLAPSE_STEP`'s doc; Task 2 writes the new ones.

- [ ] **Step 4: Run the unit tests and see them pass**

Run: `RAYON_NUM_THREADS=4 cargo test -p system --lib climate:: -- --test-threads=3`
Expected: all PASS, including the four other synthetic collapse tests unchanged.

- [ ] **Step 5: Commit**

```bash
git add crates/system/src/planet/climate/{wet.rs,mod.rs,wet_tests.rs}
git commit -m "perf(planet): step a collapse to the secant's zero once its gap falls"
```

### Task 2: Hold the new step at the cool-star climates, and document it

**Files:**
- Modify: `crates/system/src/planet/climate/heavy_tests.rs` (`collapse_step_is_converged`, about l.1960-2008)
- Modify: `crates/system/src/planet/climate/mod.rs` (`COLLAPSE_STEP` doc: the new measurements)
- Modify: `docs/science/planet-atmospheres.md` ("A collapse from the fold (#731)", l.3638)

**Interfaces:**
- Consumes: `COLLAPSE_STEP` (Task 1), `frozen_at(&mut ClimateBody, Bar) -> Settle` (`settle.rs`), `inputs_at(&EARTH, au, teff)`, `titan()`.
- Produces: nothing for other tasks.

- [ ] **Step 1: Extend `collapse_step_is_converged`**

Add a `Definition:` constant `COOL_FROZEN: [(Kelvin, f64, Bar); 3]` (star, AU, CO₂) = `[(Kelvin(4500.0), 1.2828, Bar(19.0296)), (Kelvin(4500.0), 1.2828, Bar(19.6173)), (Kelvin(3300.0), 1.3586, Bar(45.9))]`. Its doc says these are frozen calls of the thaw search measured on #731, and that the second one cycled with the old step. For each, build the methane Earth (`inputs_at(&EARTH, au, teff)` with `ch4: titan().air.ch4`). Then call `super::settle::frozen_at` on `ClimateBody::new(&inputs, numerics)`, once with `Numerics::DEFAULT` and once with half the step: the numerics live in the body. Same assertions as the existing loop: converged or cycles, same `frozen_over`, temperatures within `ALBEDO_TOLERANCE_K`. Print both temperatures and CO₂. Update the `#[ignore = "heavy: …"]` text.

- [ ] **Step 2: Run the collapse heavy tests**

Run: `RAYON_NUM_THREADS=4 cargo test --release -p system --lib -- --ignored --test-threads=3 --nocapture collapse_step_is_converged collapse_tolerance_is_converged a_climate_with_no_fixed_point_stops_on_its_cold_side thaw_steps_is_converged one_greenhouse_search_on_a_condensing_air`
Expected: all PASS. If `collapse_step_is_converged` fails, stop and report the temperatures: the step is a decision for the user, not for the implementer.

- [ ] **Step 3: Document**

Write the measured half-step gaps into `COLLAPSE_STEP`'s doc and into "A collapse from the fold (#731)": the two rules, why (the hump traced at 4500 and 3300 K), the accepted limit, and the evaluations before and after.

- [ ] **Step 4: Commit**

```bash
git commit -am "test(planet): hold the collapse step at a methane Earth round cool stars"
```

### Task 3: Measure, full suites, and the pull request

- [ ] **Step 1: Measure before and after**

Apply the scratchpad's `probes.patch` (`git apply`) on this branch, and on a temporary worktree of a97c1ca1 (`git worktree add --detach ~/Developer/mg-thaw-base a97c1ca1`, removed afterwards). Run before and after one after the other, never at the same time. Run `probe_thaw_cool_stars` with `PROBE_KIND=methane PROBE_SHARES=0.8,0.85,0.9,0.92`, and `probe_thaw_search` at `1.25,1.2534`. Revert the patch (`git checkout -- crates`). Expected: 4500 K at 0.9 from about 261 s to under 100 s; 3300 K under 40 s. Record solve times and thaw CO₂ before and after.

- [ ] **Step 2: Full suites**

Run: `RAYON_NUM_THREADS=4 cargo test --workspace --all-features -- --test-threads=3`, then `mise run test:slow`. Expected: PASS (the flaky index migration test may fail under load: rerun it alone).

- [ ] **Step 3: Review and pull request**

`branch-reviewer` on the branch with the spec and this plan. Then a PR into main, `Refs #731` (part 2 is answered by measurement; the bisection idea stays open), with the measurement table and the heavy tests' output.
