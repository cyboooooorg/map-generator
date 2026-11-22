# Compact Outer Edge Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps
> use checkbox (`- [ ]`) syntax for tracking.

**Goal:** a compact chain ends at its first drawn planet proposed beyond
1071 d, so no compact planet lies past the Kepler CKS multis (#630).

**Architecture:** a sourced constant `COMPACT_OUTER_EDGE` in
`families/mod.rs` and a predicate `ends_chain` in `families/compact.rs`.
`Placer::attempt` returns a three-way `Try` instead of an `Option`; a
compact proposal beyond the edge gives `Try::ChainEnds`, `Placer::slot`
passes it up as `Filled::ChainEnds`, and the family-slot loop of `place`
breaks. No retry: retrying would keep only short ratios near the edge and
pile planets under it. Drawn systems change, so `ENGINE_VERSION` goes to 3.

**Tech Stack:** Rust 1.99, crate `system`.

**Spec:** `docs/superpowers/specs/2026-11-21-system-draw-design.md`,
§ Planets of a random system; issue #630; study
`docs/superpowers/studies/2026-11-21-architecture-families/small-planets-and-spacing.md`
§ 1.2-1.3.

## Global Constraints

- Edge: **1071 d**, constant in days for every host (user decision,
  2026-11-22). Source: Weiss et al. 2018 (CKS V, AJ 155, 48,
  arXiv:1706.06204), § IV, bootstrap step 5: "Remove any planets with
  orbital periods greater than 1071 days (the maximum observed orbital
  period of a planet in the CKS multis)". Constant period: the study
  § 1.2 derives from Mulders 2015 that an edge scaling as M^(1/3) in axis
  is a constant period.
- Mechanism: **the chain stops** (user decision, 2026-11-22). The first
  compact family-slot try whose proposal lies beyond 1071 d leaves that
  slot and every later family slot empty. Weiss's bootstrap removes such
  planets the same way, it does not redraw them.
- Strictly beyond: a period of exactly 1071 d is kept (`>`), as in Weiss.
- Only `Family::Compact` drawn slots. Asked archetypes (`Slot::Asked`)
  and the other families are unchanged.
- `COMPACT_MAX_PERIOD` (640 d, innermost planet) stays as it is; it is
  below the edge, so slot 0 never ends the chain.
- `ENGINE_VERSION` 2 → 3, agreed with mapgen1 and mapgen2 on 2026-11-22.
- Repository text in English; Markdown passes markdownlint defaults;
  commits follow Conventional Commits with the session's attribution
  lines.

## Review Focus

1. The chain must stop, not retry: a retry would also keep every planet
   inside the edge, so the grid bound alone cannot tell the two apart.
   Task 1 tests the loop itself (`fill_family`, test
   `a_chain_end_stops_the_family_slots`). A K50-rejected try followed by
   a try beyond the edge ends the chain too: `attempt` checks the edge on
   every try.
2. Low-mass hosts (0.08 M☉): the edge is 0.88 au there; slot 0 must
   never end the chain (its period is below 640 d). Task 1 asserts that
   the 0.08 M☉ compact systems of the grid are not all empty.
3. A compact system with an asked archetype: asked planets are never cut
   by the edge, even past 1071 d. Task 1 test
   `asked_planets_ignore_the_compact_edge`.
4. Same inputs, same system: the per-slot streams (`orbit/slot-k`) are
   unchanged, so a chain that ends keeps its earlier planets identical.
   Covered by the existing determinism tests (`crates/system/tests/determinism.rs`).
5. The four other families with a proposal far out: `ends_chain` is
   false for them. Unit test in Task 1.

---

### Task 1: The edge, the chain end and the engine version

**Files:**

- Modify: `crates/system/src/families/mod.rs:25-29` (add the constant
  next to `COMPACT_MAX_PERIOD`; fix the latter's doc: it bounds the
  innermost planet only, the family's chain is bounded by the new edge)
- Modify: `crates/system/src/families/compact.rs` (add `ends_chain`; doc
  of `propose` mentions the edge)
- Modify: `crates/system/src/draw/place.rs` (module doc line ~13,
  `place` loop 176-182, `slot` 220-248, `attempt` 252-361)
- Modify: `crates/system/src/lib.rs:31-36` and `crates/system/README.md:106-108`
  (engine 3)
- Test: `crates/system/src/families/tests.rs`, `crates/system/src/draw/tests.rs`

**Interfaces:**

- Produces: `pub(crate) const COMPACT_OUTER_EDGE: Day = Day(1071.0);`
  in `families/mod.rs`, with a `Source:` doc quoting Weiss 2018 as in
  Global Constraints.
- Produces, in `families/compact.rs`:

  ```rust
  pub(crate) fn ends_chain(ctx: &FamilyContext, proposal: &Proposal) -> bool
  ```

  It is re-exported where `place.rs` imports from
  `crate::families`. True iff `ctx.family == Family::Compact` and
  `orbit::period(proposal.a, ctx.host).0 > COMPACT_OUTER_EDGE.0`.
- Produces (private to `place.rs`):
  `enum Try { Kept(Trial, TrialOutcome), Spent, ChainEnds }` returned by
  `attempt` as `Result<Try, SystemError>`, and
  `enum Filled { Planet(Placed), Empty, ChainEnds }` returned by `slot` as
  `Result<Filled, SystemError>`. Every current `Ok(None)` of `attempt`
  becomes `Ok(Try::Spent)`.
- Produces (private to `place.rs`): the family-slot loop moves into:

  ```rust
  fn fill_family(
      count: u32,
      placed: &mut Vec<Placed>,
      mut slot: impl FnMut(u32, &[Placed]) -> Result<Filled, SystemError>,
  ) -> Result<(), SystemError>
  ```

  It fills slots `0..count` in order, `Planet` pushed, `Empty` skipped,
  `ChainEnds` returns at once. `place` calls it with
  `ctx.counts.small + ctx.counts.giants` and a closure over
  `placer.slot(Slot::Family(n), format!("slot-{n}"), placed, trace)`.

- [ ] **Step 1: Write the failing tests**

In `families/tests.rs`:

```rust
#[test]
fn compact_chains_end_beyond_1071_days() {
    for host in [0.08, 1.0, 8.0] {
        let mut ctx = context(Family::Compact, 0, false);
        ctx.host = SolarMass(host);
        let at = |days: f64| Proposal {
            mass: ctx.system_mass,
            a: orbit::semi_major_axis(Day(days), ctx.host),
            kind: SlotKind::Small,
        };
        assert!(!ends_chain(&ctx, &at(1070.0)), "{host}");
        assert!(!ends_chain(&ctx, &at(1071.0 * (1.0 - 1e-9))), "{host}");
        assert!(ends_chain(&ctx, &at(1072.0)), "{host}");
    }
    for family in [
        Family::SolarLike,
        Family::HotJupiter,
        Family::EccentricGiants,
        Family::Sparse,
    ] {
        let ctx = context(family, 0, false);
        let far = Proposal {
            mass: ctx.system_mass,
            a: orbit::semi_major_axis(Day(5_000.0), ctx.host),
            kind: SlotKind::Small,
        };
        assert!(!ends_chain(&ctx, &far), "{family}");
    }
}
```

(`context(family, giants, stirred)` is the helper at
`families/tests.rs:114`; its host is 1 M☉, overridden above.)

In `draw/tests.rs`, inside `every_family_on_every_host`, for
`family == Family::Compact` (`Orbit` carries `period: Traced<Day>`):

```rust
assert!(
    placed.iter().all(|p| p.trial.orbit.period.value().0 <= COMPACT_OUTER_EDGE.0),
    "{what}: past the compact edge"
);
```

Also count the compact planets placed around 0.08 M☉ over the grid and
assert the count is above zero after the host loop.

Add `a_chain_end_stops_the_family_slots` in `draw/tests.rs`: call
`fill_family(4, &mut placed, slot)` with a closure that records each
slot number it is called with and returns, by slot, `Planet(p0)`,
`Empty`, `ChainEnds`, `Planet(p3)` (build the `Placed` values with the
`placed(...)` helper of `families/tests.rs:136`). Assert the calls are
`[0, 1, 2]` and `placed.len() == 1`.

Add `asked_planets_ignore_the_compact_edge` in `draw/tests.rs`: on
`parts(seed, 1.0, Family::Compact)`, ask `id("jupiter-like")` once (its
band is an insolation of 0.033-0.041 S⊕, about 5 au and 4000 d around a
Sun-like star), run with the `model` classifier, and assert that the
planet with `asked == Some(id("jupiter-like"))` is in the result and its
period is above `COMPACT_OUTER_EDGE`. Use a seed whose run succeeds
(try a few; report the one kept).

- [ ] **Step 2: Run them and check they fail**

Run:

```bash
cargo test -p system -- compact_chains_end_beyond \
  every_family_draws_on_every_host asked_planets_ignore a_chain_end_stops
```

Expected: compile errors on `ends_chain`, `COMPACT_OUTER_EDGE`,
`fill_family` and `Filled`. With stubs (`ends_chain` returning `false`,
`fill_family` ignoring `ChainEnds`): `every_family_draws_on_every_host`
panics with "past the compact edge", `compact_chains_end_beyond_1071_days`
and `a_chain_end_stops_the_family_slots` fail. `asked_planets_ignore_the_compact_edge`
may already pass; it guards the fix against cutting asked planets.

- [ ] **Step 3: Implement**

Constant and `ends_chain` as in Interfaces. In `attempt`, `Slot::Family`
branch, right after `propose(...)` and before the existing solar-like
and Fulton checks: `if ends_chain(ctx, &proposal) { return Ok(Try::ChainEnds); }`.
In `slot`: `Try::Kept` → `Filled::Planet`, `Try::Spent` → next try,
`Try::ChainEnds` → return `Filled::ChainEnds` at once; all tries spent →
`Filled::Empty`. Asked loop: anything but `Filled::Planet` →
`SystemError::NoRoom` as today. Family loop: `fill_family` as in
Interfaces.
`Placer::slope` (calls `propose` for slot 0 only) is unchanged.
Leave `Rules::total` as it is (it already counts empty slots).

Module doc of `place.rs` (~line 13): add that a compact chain ends at
its first proposal beyond 1071 d. `ENGINE_VERSION` doc: add
`/// 3: a compact chain ends at 1071 d (#630), so compact systems lose
their far planets.` and set `EngineVersion(3)`; README: "3 since compact
chains end at 1071 d".

- [ ] **Step 4: Run the crate tests**

Run: `cargo test -p system` then `cargo test -p map-generator`
Expected: all pass. A test pinning a compact draw (e.g.
`draw/tests.rs:569`, `calls == 2 * SLOT_TRIES`) that changes must be
understood before it is re-pinned; report each re-pinned value.

- [ ] **Step 5: Lint and commit**

Run: `mise run lint` (clippy, warnings denied, with the determinism
lints) and `cargo fmt --check`.

```bash
git add crates/system
git commit -m "fix(system)!: end compact chains at 1071 d" -m "<body: Weiss 2018 source, stop not retry, engine 3>" -m "BREAKING CHANGE: ENGINE_VERSION 3; the same inputs draw compact systems without their planets past 1071 d." -m "Refs: #630"
```

### Task 2: Re-measure the grid and update the science docs

**Files:**

- Modify: `crates/system/src/draw/tests.rs` (`every_family_on_every_host`
  and `every_family_draws_on_every_host_full_grid`)
- Modify: `docs/science/system-architectures.md:33` (table row) and
  `:307-312` (departure "Compact chains can run far out")
- Modify: `docs/superpowers/studies/2026-11-21-architecture-families/README.md:131`
  and `small-planets-and-spacing.md` § 1.3 (one line each: what the draw
  now uses, pointing to Weiss § IV step 5; the study text stays as read)
- Modify: `docs/superpowers/specs/2026-11-21-system-draw-design.md:219-220`
  (compact line: "4-7 small planets … chain ends at 1071 d, Weiss et al.
  2018 § IV")

**Interfaces:**

- Consumes: `COMPACT_OUTER_EDGE` from Task 1.
- Produces: `every_family_on_every_host` also returns the longest compact
  period in days and the number of compact planets (extend its tuple; keep
  the other callers compiling).

- [ ] **Step 1: Print the compact figures**

The full-grid test's `eprintln!` adds: compact planets placed, longest
compact period (d), largest compact `a × 2.7 au / ice line` (au).

- [ ] **Step 2: Run the full grid**

Run:

```bash
cargo test --release -p system every_family_draws_on_every_host_full_grid \
  -- --ignored --nocapture
```

Expected: passes; the longest compact period is ≤ 1071 d. Note every
printed figure.

- [ ] **Step 3: Update the docs with the measured figures**

`system-architectures.md`:

- Table row 33: "4 to 7 small planets, the innermost within 640 d, the
  chain ending at 1071 d".
- Replace the departure "Compact chains can run far out" with one stating
  the new departure: the edge is Kepler's detection limit used as the
  edge of the population (Weiss 2018 § IV step 5, the longest period of
  the CKS multis), constant in days for every host (study § 1.2, derived
  from Mulders 2015) although no source measured it outside 0.2-1.4 M☉;
  a chain that reaches it has fewer planets than drawn. Give the grid
  figures of Step 2 (200 seeds, six hosts, stand-in classifier).

- [ ] **Step 4: Lint and commit**

Run: `mise run md:lint` and `cargo test -p system`.

```bash
git add crates/system/src/draw/tests.rs docs/science docs/superpowers/specs docs/superpowers/studies docs/superpowers/plans/2026-11-22-compact-outer-edge.md
git commit -m "docs(system): record the compact edge and re-measure the grid" -m "Refs: #630"
```

### After the tasks (main session)

- `branch-reviewer` on the whole branch with this plan and the spec.
- `mise run test:slow` on the branch.
- Re-run the CLI sweep (`system --seed 1..300`): no compact planet past
  1071 d.
- PR into `main` with `Closes #630`.
