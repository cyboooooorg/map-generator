# A faster CO₂ collapse: separate step rules for a rising and a falling gap

Issue: #731 (part 2, re-measured on main a97c1ca1). Branch: `perf/thaw-search`.

## Intent and why

A methane Earth near its outer edge round a cool star takes 45 to 276 s to
solve (4500 K at 0.9 of its edge, 3300 K at 0.8 to 0.92), against under
8 s elsewhere. More than 99 % of that time is in `collapsed_climate`:
each albedo step of a frozen climate collapses its CO₂ again, in 40 to 85
evaluations of about 33 ms. Planet PR 2d-4 will run the climate for every
try of the system draw, so these bodies would stall it.

The gap `g(c) = c − φ(c)` of these collapses is a single hump: it starts
near zero at the fold, rises to 0.3 to 3 bar, then falls to one root.
`collapse_search` caps every accelerated step at `COLLAPSE_STEP` (0.5 %)
of the CO₂. On the rise the cap keeps the step small. On the fall it
stops the secant short of the root, and the plain step closes in slowly
(for example 14 evaluations from 2.9 to 2.2 bar at 3300 K).

## Goals

- Cut the evaluations per collapse on these traces from 83 to about 24
  (4500 K) and from 44 to about 22 (3300 K). This is simulated offline on
  traced gaps; the simulation reproduces today's counts within 2.
- Keep the collapse landing on the warmest root below its start
  wherever the gap has a single root.

## Non-goals

- The thaw bisection itself (stopping it at the first cycling call, idea
  B on #731): left for after, if the cycling calls still cost.
- The cost of a cycling climate's albedo steps in `settle`.

## Anti-goals

- A climate that changes branch, or moves by more than
  `ALBEDO_TOLERANCE_K`, against the current step at the orbits measured.

## Approaches

1. Raise `COLLAPSE_STEP` to 0.05. One constant: 29 / 32 evaluations
   (4500 K / 3300 K). It helps 3300 K little.
2. **Chosen by the user (2026-11-27).** Two rules: while `g` rises or
   stays flat, the step doubles up to a cap of 0.05 of the CO₂; once `g`
   falls, the step goes to the secant's zero, limited only by twice the
   last step. 24 / 22 evaluations.
3. Option 2 plus an extra evaluation at the midpoint of every long step,
   to catch a pair of roots wider than half a step. It costs about one
   evaluation per long step, and it is still no proof.

## Decided

- Option 2. The user accepts the risk: a warm balance (a pair of roots)
  narrower than a rising step, at most 5 % of the CO₂, or hidden before
  the secant's zero on the fall, can be stepped over.
- The guard is a test, not a check in the solver. The tier-3 test
  `collapse_step_is_converged` is extended to the cool-star orbits
  measured here: at half the new steps the climate keeps its branch and
  moves under `ALBEDO_TOLERANCE_K`.

## Left to the builder

- Names and form: a new rising cap beside `COLLAPSE_STEP`, or
  `COLLAPSE_STEP` changing meaning. Either way, each constant keeps its
  `Calibration:` doc and the convergence test that holds it.
- Whether the cool-star cases in the test solve whole bodies or call
  `frozen_at` at the CO₂ measured (19.03 bar at 4500 K and 1.2828 AU;
  45.9 bar at 3300 K and 1.3586 AU), to keep the heavy tier's time
  bounded.
- The noise rule of #743 is unchanged: within the noise, no zero is a
  balance.
- Measure the solves before and after (release, 4 rayon threads) at the
  orbits of #731's comment, and post the numbers on the pull request.
- Update `docs/science/planet-atmospheres.md` ("A collapse from the fold
  (#731)") and the doc comments of `collapse_search` and the constants.
