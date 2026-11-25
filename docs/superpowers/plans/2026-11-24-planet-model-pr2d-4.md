# Planet model PR 2d-4: life, class and wiring Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish the planet chain of slice 1: life and its oxygen with
the climate solved again, the class, one private `chain` behind
`planet::class_of` and `planet::model`, the draw wired to the real
classifier, `ENGINE_VERSION` 4, and the tier-2 net on the eight
reference systems.

**Architecture:** The values (life's ages and chance, the O₂ ratio, the
class thresholds, the basin capacity, the hot period) and their pure
laws go to `science` (`science::life`, `science::classes`).
`system::planet` gets `life.rs` (the life draw, the `Biosphere` node,
the oxygenated air and the second solve) and `classify.rs` (a pure
`class` on plain values, and `classify` that builds them from the chain
and records the `planet.class` node). `chain.rs` runs draw, ground,
spin, climate, boil loop, steam world, life, class, and serves
`class_of` (scratch trace, plain outcome) and `model` (real trace,
`PlanetState`). The spin of every planet is drawn from the trial seed,
so the obliquity the climate reads is the one `Placement.spin` holds.
`timeline::Life` becomes `Traced<Biosphere>`. The tests that draw whole
systems with the real classifier live in tier 2 (`crates/regression`),
which reaches the CLI through a new library target.

**Tech Stack:** Rust, `cargo nextest` tiers, the `Trace`/`Traced` cause
API of `timeline`.

**Spec:** `docs/superpowers/specs/2026-11-23-planet-model-pr2d-design.md`,
"2d-4: life, class and wiring" (amended in `db3b8248` and again with
this plan's amendment: seeds and spin, life contract, Titan, the tier-2
moves, the develop census), "Constraints", "Rules for the build"
("Same trial, same answer"), "Left to the builder". Science:
`docs/science/planet-atmospheres.md` sections "Life" (l.~2974) and "The
chain and the surface classes" (l.~3084),
`docs/science/planet-interiors.md` "Classes" (grep the heading); Wiki
[27-life-and-class-audit](https://github.com/cyboooooorg/map-generator/wiki/27-life-and-class-audit).
Recon: `.superpowers/sdd/2026-11-24-planet-model-pr2d-4/recon.md`; plan
review: `plan-review.md` there (K, S and N items cited below). Ledger:
`progress.md` there. Legacy: `legacy-final:crates/mapgen/src/planet/`
(`life.rs`, `classify.rs`, `explain.rs` l.1122-1134, `reference.rs`).

## Decisions

From the spec and the user (2026-11-24 and 2026-11-25; details in the
ledger):

- **Seed.** `class_of` and `model` both draw the chain from the trial
  seed (`placement.trial.seed` in `model`). Trial labels are unique per
  slot (`<id>/<k>`, `slot-<n>`, `draw/place.rs:162-165`, `:184`), so the
  trial seed is per planet. `Placement.seed` (the body seed) is no
  longer read by the planet model.
- **Same trial, same answer (K1, user 2026-11-25).** The draw classifies
  the circular trial (`place.rs:355`, `circular()` at `e = 0`,
  `place.rs:509-530`); `model` runs on `placement.trial`, the drawn
  eccentric orbit with its higher mean flux (`place.rs:384-398`,
  `sources.rs:54-80`). For the same trial, `class_of` and `model` give
  the same class, radius and density; for an eccentric placement they
  may differ. Tier 1 asserts equality at `e = 0`; tier 2 counts and
  pins the reference planets whose class changes between the draw and
  the model. Follow-up issue: the draw classifies the eccentric trial
  again (Task 12).
- **Spin (#677).** `draw/mod.rs:69-78` draws every planet's spin from
  `planet.trial.seed` instead of the body seed. The chain calls the same
  `spin::spin` to get the obliquity of its `Rotation`; `locked` is an
  imposed `false` until #604. One value: in `model` the climate reads
  the node `placement.spin.obliquity` itself; in `class_of` it reads the
  obliquity of a `spin::spin` call on the scratch trace, equal to it bit
  for bit by construction (B2, B3; Task 1 and Task 9 prove it).
  mapgen3 agreed to B2, B4 and `class_of` calling `spin::spin`, on four
  conditions, all in Tasks 1 and 9: the doc of `spin.rs:74-78` says the
  vector is drawn in units of `ω_cr`; the distribution tests
  `spin.rs:201-268` stay unchanged (a last-bit pin may move); the draw
  test is renamed `a_planet_spin_comes_from_its_trial_seed`; a body
  with a surface is exactly one whose final class has
  `!spin::is_giant`, and a tier-2 test checks, for every placed body
  with a surface, that the obliquity `class_of` read equals
  `placement.spin.obliquity` bit for bit.
- **Life contract.** `timeline::Surface.life: Traced<Biosphere>`,
  `Biosphere { Lifeless, Anoxic, Oxidising, Modern }` (`contract_enum!`,
  kebab-case), replacing `struct Life { arose, oxygenated }`. One node,
  `planet.life`, whose parents are the liquid, the age and the life
  draw.
- **Life values.** `LIFE_MIN_GYR` 1.17 Gyr (`Source:` Javaux 2019,
  Connelly et al. 2012); `LIFE_CHANCE` 1/3 (`Definition:`, game choice,
  user 2026-11-12); `OXYGEN_AFTER_GYR` 2.2 Gyr → `Oxidising` (CH₄
  removed, O₂ none; `Source:` Lyons et al. 2014 pp. 307, 314);
  `MODERN_OXYGEN_AFTER_GYR` 4.0 Gyr → `Modern` (`Source:` Lyons p. 307);
  modern O₂ = `O2_PER_N2` × N₂, the ratio computed in `science` from the
  CIPM-2007 shares O₂ 0.209390 and N₂ 0.780848 (`Source:` Picard et al.
  2008 Table 1); CH₄ 0 from `Oxidising` on (`Definition:`). The life
  draw comes from `seed.derive("planet/life")`, beside the snowball
  draw.
- **Climate after oxygenation.** An `Oxidising` or `Modern` air is
  solved once more (`climate_of`, same rotation, same snowball draw),
  without a second boil verdict (S8: written on the page, counted in
  tier 2). The steam envelope stays after the loss (`water.rs:237`
  `debug_assert`): life needs liquid water, so a steam world never gets
  there. Reference Earth: 286 to 290 K at 280 ppm (the tests' window,
  `PREINDUSTRIAL_RANGE`, not Myhre's).
- **Class** (study 27). Read `Settled.hides_surface`, never the steam
  line again. Gas giant / hot Jupiter = `Body::Giant`; ice giant / hot
  Neptune = `Body::MiniNeptune` whose bulk (kept) mass is at least
  `MINI_NEPTUNE_MAX_MASS` (10 M⊕, `envelope.rs:54`; N9: a body drawn
  just over 10 M⊕ that loses gas becomes a mini-Neptune); mini-Neptune =
  `Body::MiniNeptune` under it, no radius rule. Hot when the orbit's
  period is at most `HOT_GIANT_MAX_PERIOD` (10 d), **one constant** for
  the class and the hot-Jupiter family (S5, below). Ocean world: water
  kept over `3.7 (R/R⊕)² (g⊕/g)` Earth oceans (Cowan & Abbot 2014),
  every regime. Ice world: the kept water fraction (`bare.drawn`, N10)
  at least `heat::ICY_MIN_WATER`, and at least `ICE_WORLD_MIN_OCEANS`
  0.01 (`Calibration:`). Runaway greenhouse: air over `DENSE_AIR_BAR`
  1 bar (`Calibration:`). Hydrocarbon world: air over methane's
  triple-point pressure, the rewrite's
  `methane_saturation_bar(METHANE_TRIPLE_TEMPERATURE)`. Super-Earth from
  1.25 R⊕ (Borucki et al. 2011). **Exactly two new calibrations**:
  `DENSE_AIR_BAR`, `ICE_WORLD_MIN_OCEANS`.
- **The hot period (S5).** `families/mod.rs:45-46` holds
  `HOT_JUPITER_MAX_PERIOD: Day(10.0)` (`Source:` Dawson & Johnson 2018,
  study G 1.1), which places the hot-Jupiter family. It moves to
  `science::classes::HOT_GIANT_MAX_PERIOD`, whose doc cites Wright et
  al. 2012 § 3 (read in full in study 27: a hot Jupiter has a period
  under 10 days) and names Dawson & Johnson 2018's review, which uses
  the same 10 d line. No alias (D5, mapgen3): the definition at
  `families/mod.rs:46` is removed and its three uses read
  `science::classes::HOT_GIANT_MAX_PERIOD` by that name
  (`families/mod.rs:114`, `families/laws.rs:11` and `:344`). A giant
  the hot-Jupiter family places is then a `HotJupiter` by construction.
- **Tier placement (B1, user 2026-11-25).** No build profile changes.
  The tests that draw whole systems with the real classifier move to
  tier 2, into `crates/regression` (the tier definition stays "tier 2
  = `crates/regression`"); `stand_in` stays for the draw's unit tests.
  How each moved test lands there: Task 8 and D1.
- **Tier 3 stays tier 3 (B9 no, user 2026-11-25).** Venus's 577 K
  (#594), #492's radius and temperature and the two heavy settled tests
  stay `#[ignore = "heavy: …"]`; no `pinned` module, no nextest or
  `CLAUDE.md` change. See D4 for the numbers themselves.
- **The CLI report and the develop cycle (S7, B11, user 2026-11-25).**
  The system report prints each planet's class in place of the
  `classifier` field. The develop cycle delivers a census of classes,
  climates and biospheres on the oracle's seeds, before and after and
  against legacy, with the timings; no 2:1 projections (main session,
  see the last section; D3 for what the report carries).
- **#679** is fixed in `fix/titan-nitrogen-inventory` (ξ refitted on
  Titan's N₂ by weight, about 1.418 bar), merged before this PR; Task 11
  waits on it (rebase first).
- **Generation changes.** `ENGINE_VERSION` 3 → 4; golden digests
  re-pinned in Task 1 (spins) and Task 8 (classes); the `SystemId` pin
  (which reads the engine version only) in Task 8; `mise run
  test:heavy` output and the profiler's before and after in the PR
  description.

Builder's decisions (settled by this plan; B1 and B9 were decided by
the user above, B11 is replaced by the report's class):

- **B2. The rocky spin is drawn in units of the critical rate.** In
  `spin.rs` the rocky branch draws the three normals `n`, keeps them
  when `ROCKY_SIGMA_OVER_CRITICAL · |n| < 1`, and takes the fraction
  `ROCKY_SIGMA_OVER_CRITICAL · |n|` and the obliquity
  `acos(n_z / |n|)`: `sigma` and `critical` cancel exactly, so the
  obliquity depends on the seed and on the law (rocky or giant) only,
  bit for bit. The period is still `2π / (fraction · critical)`; the
  fallback scales `n`. The law is unchanged; the values change in the
  last bits (they change anyway with the seed).
- **B3. `class_of`'s spin call.** The chain calls `spin::spin` only for
  a body with a surface, after its ground is formed: with the class
  `PlanetClass::Rocky` (every class a surfaced body can get takes the
  rocky law), the formed body's radius, the trial's archetype placement
  (`trial.archetype.as_ref().and_then(archetype::placement)`, so an
  asked obliquity is the same `Provenance::Input` value in both) and the
  trial seed. `model` calls nothing: it reads `placement.spin.obliquity`.
  The chain `debug_assert!`s that a surfaced body's final class has
  `!spin::is_giant` (N12; `is_giant` becomes `pub(crate)`).
- **B4. `draw/**` beyond the seed line.** `draw/mod.rs`'s doc of
  `draw_with` (l.44-52, "its spin is drawn from that body seed") and the
  draw test `a_planet_spin_comes_from_its_body_seed`
  (`draw/tests.rs:714-737`, `planet.seed` → `planet.trial.seed`,
  renamed `a_planet_spin_comes_from_its_trial_seed`) change with the
  line, or the build breaks. Nothing else in `draw/**`.
- **B5. The interior of `PlanetState`.** `layers`: one `timeline::Layer`
  per `Body::layers()` shell, the same four nodes
  (`outer_radius`, `mass`, `material`, `state`), `transport: None`; no
  node recorded again. `samples`: `science::structure::Sample` has no
  temperature, and `RadialSample.temperature` is required, so the
  samples are an empty `RadialSamples` imposed leaf
  (`planet.interior.samples`, `Provenance::Imposed`) until a planet
  temperature profile exists (follow-up issue, Task 12).
- **B6. `SurfaceLiquid` (S2, S10).** The world reads `ocean_share` as
  the share under water, for its sea level
  (`world/src/relief/mod.rs:42-46`, `:84`). Water: `inventory` is the
  `Water.inventory` node (the contract's doc); `ocean_share` is
  `science::surface::cover::ocean_fraction(inventory / OCEAN_MASS)`, the
  sea term `cover` reads at `cover.rs:282` (ice-covered sea included),
  recorded as `planet.surface.ocean-share` with the nodes that function
  reads: the water inventory, and the liquid node that says the liquid
  is water. Hydrocarbons and nitrogen: `inventory` the condensate's
  column mass (`science::outgassing::column_mass`, the inverse of
  `column_pressure`, `Definition:` hydrostatic balance), `ocean_share`
  an imposed 0. Lava: `inventory` and `ocean_share` imposed 0. The
  imposed zeros reach the world's sea level (no Titan lakes, no lava
  sea): the follow-up issue says so, and mapgen2 is told (main
  session).
- **B7. The life node's parents.** The liquid node when there is one,
  else the climate's state node (it says why there is none), plus the
  age and the life draw. A body without a surface has no `Surface`, so
  no life node.
- **B8. The oxygenated air.** Made from the bare air (the inventory the
  climate condenses from): `planet.life.air.o2` from the N₂ and the
  biosphere, `planet.life.air.ch4` from the biosphere, the
  `planet.life.air.pressure` from the partials. The class and
  `Surface` read the climate of the second solve; the life node reads
  the first. No second boil verdict on the second solve.
- **B10. Titan's total pressure** bound: 1.467 bar (Fulchignoni et al.
  2005, Huygens) within 10 %, the tolerance #679 sets for its N₂, a
  `Definition:`.
- **B12. The `boiled_recondensed` note** is not ported: the rewrite's
  chain already shows it (`planet.water.boiled` true, from the verdict;
  `planet.water.runaway` false, from the leftover's inventory and
  runaway, `water.rs:194-205`), and the water lost would be a difference
  computed to explain, against "one code path". The unread `boiled`
  node of the steam world's second pass (`settled.rs:225-230`) stays:
  `Trace::freeze` keeps only reachable nodes. Task 10 tests both; Task
  12 fixes the page (l.3121).
- **B13. `PlanetState` fields from the chain.** `composition`:
  `bare.drawn.core_mass_fraction` and `water_fraction` (the kept
  planet's, without envelope); `envelope_fraction`:
  `bare.drawn.envelope_fraction`; `bulk`: `bare.body.bulk()`; `heat`:
  `bare.heat`; `sources`, `age`, `orbit`: `placement.trial`'s (the
  eccentric orbit, K1); `spin`: `placement.spin`; `targets`: empty (no
  `ArchetypeTargets` type exists yet, PR 3). In `Surface` (N6):
  `terrain` is `Terrain { units: vec![] }` (PR 3), `atmosphere` the
  second climate's `Climate.atmosphere` (the first one's when there is
  no second solve), `climate` its `GlobalClimate`.
- **S4. Pins across macOS and Linux.** CI runs on Linux, on pull
  requests only (`.github/workflows/ci.yml:3-6`), so a task cannot read
  CI's values. Categorical pins (class, liquid, biosphere, counts) are
  exact; the climate pins of Task 9 hold the surface temperature within
  `ALBEDO_TOLERANCE_K` (the solver's own settling tolerance, a
  `Definition:` the test cites), not to 1e-9; digests and `SystemId` are
  pinned from the local run. When the PR's CI differs, the main session
  splits the digest table per `target_os`, as
  `regression/tests/determinism.rs:6-9` already says, in a fix commit
  on the branch.

Decisions D1 to D5, decided on 2026-11-25 (D1, D3, D4 by the user;
D2 and D5 by mapgen3), as proposed except D5's alias:

- **D1. How the CLI's drawing tests reach tier 2.** The `cli` package
  has only a binary target (`crates/cli/Cargo.toml:8-10`), so
  `crates/regression` cannot call it, and `CARGO_BIN_EXE_map-generator`
  exists only in the CLI's own integration tests. Decided: give `cli`
  a library target (`src/lib.rs` holding today's modules and a `pub fn
  dispatch(cli: &Cli) -> Result<String, CliError>`; `main.rs` keeps
  `main`, the stderr notice and the exit codes). The drawing tests of
  `report.rs:112-147` and `explain.rs:340-440` move to
  `crates/regression/tests/cli.rs` on that library. The binary tests of
  `cli/tests/system.rs` that never reach the classifier stay in tier 1
  (`a_malformed_archetype_is_a_usage_error`,
  `a_negative_host_mass_reaches_the_draw`,
  `a_date_after_the_star_dies_fails`, `an_unknown_archetype_fails`:
  each fails at the star or before the first try). The binary tests
  that must draw (exit code 1 on an explain error, the closed pipe, the
  drawn-seed notice, the bytes of a seed) keep only what needs the
  binary and become tier 3 (`#[ignore = "heavy: draws a system with
  the planet model through the binary"]`); their JSON assertions move
  to tier 2 through `dispatch`. Rejected: keep the binary tests in
  tier 1 on `stand_in` through a hidden CLI flag (a test-only option in
  the user's interface: not recommended).
- **D2. The obliquity `class_of` read, for mapgen3's tier-2 test.**
  `class_of` returns plain values only. Decided: `pub fn
  planet::class_read(trial, trace) -> Result<ClassRead, SystemError>`
  with `pub struct ClassRead { pub outcome: TrialOutcome, pub
  obliquity: Option<Degree> }` (`None` without a surface); `class_of`
  is `class_read(..).map(|r| r.outcome)`. The tier-2 draw passes a
  closure that logs every read by trial seed. No change to the seam
  type `TrialOutcome` (`placement.rs`, mapgen3's).
- **D3. What the report carries per planet.** "Each planet's class" and
  a census of climates and biospheres need the model, not only the
  draw: decided, `PlanetDraw` gains `state: PlanetState` (the model's,
  on the eccentric trial: its class, climate, liquid, biosphere), so
  `--explain` reaches every node and the census reads the JSON; the
  draw's class stays in `causes` (`system.draw.class`). This runs the
  chain a second time per planet in the CLI (draw, then `model`).
  Rejected: only `class` (the model's `Traced<PlanetClass>`) and a
  separate census command later.
- **D4. The tier-3 numbers are not pins.** `CLAUDE.md` allows a test
  that pins the code's output only in tier 2. Kept in tier 3 (B9 no),
  Venus's 577 K and #492's radius and temperature are printed by their
  heavy tests (`println!`, visible in `mise run test:heavy`) and
  recorded in #594 and #684, not asserted. The two heavy settled tests
  assert behaviour, not values, and stay as they are.
- **D5. `families` reads the science constant** (S5), agreed by
  mapgen3 without an alias: `HOT_JUPITER_MAX_PERIOD` disappears, its
  three uses are renamed `HOT_GIANT_MAX_PERIOD` (imported from
  `science::classes`), and no test ties two names.

## Global Constraints

- Crates: laws and numbers in `science`; chain, records and class in
  `system::planet`; the contract change in `timeline`; the CLI library
  target in `cli`. Dependencies as `CLAUDE.md` draws them
  (`regression` may depend on `cli`: it is a test-only crate).
- `crates/system/src/draw/**`: only `draw/mod.rs` (the spin seed line
  and its doc) and the one draw test of B4. `placement.rs`: doc only.
  `archetype.rs`: untouched. `families/mod.rs` and `families/laws.rs`:
  the rename of S5 (D5). `spin.rs`: B2, the seed and its docs, `is_giant` `pub(crate)`.
- Every number in `science` (and every new number in the engines and
  tests) has a doc comment starting with `Source:`, `Calibration:` or
  `Definition:`. Two new calibrations, no more: `DENSE_AIR_BAR`,
  `ICE_WORLD_MIN_OCEANS`; each a line under its own heading in the PR
  description.
- One code path per value: the class reads `hides_surface`, the liquid,
  the runaway node, the air's pressure node, the water inventory node;
  it computes only the basin capacity and the period test. The ocean
  share is `ocean_fraction`, the law the solver's cover reads.
- The domain emits codes (`system/src/planet/codes.rs`), never text.
- No draw outside the seed hierarchy; every draw of the chain derives
  from the trial seed, never from the trace. The snowball and life
  draws are drawn once in `chain` as values (N5) and passed down.
- Tests: tier 1 budget 2 min (about 39 s now), tier 2 8 min. A full
  bistable-Earth solve is tier 3 (user, 2d-3a). A new chain test over
  5 s at opt-level 0 goes to tier 3 with
  `#[ignore = "heavy: <why, measured time>"]`. No tier-1 test draws a
  whole system with the real classifier (B1). Each task's report gives
  the time of every new test. Expected values come from sources or
  definitions; pins of the code's output only in tier 2 (D4).
- Every task ends with `mise run ci` green, Task 1 included (K2: it
  re-pins the digests).
- Stop rules: a sourced or observed bound that fails (Earth's 286 to
  290 K, Titan's 1.467 bar, the Solar System's classes) stops the task,
  which reports the gap; no window is widened, no input moved, no
  constant refitted on the code's output.
- No per-task reviewer; the end-of-branch agents review once.
- Commits: Conventional Commits, the session footer (two lines,
  `Co-Authored-By` and `Claude-Session`).

## Review Focus

1. **The second solve changes the liquid.** An oxygenated air warms the
   planet (more pressure); a body near an edge may lose or gain its
   water. Expected: the biosphere stays what the first climate decided
   (its node), the class and `SurfaceLiquid` read the second climate,
   nothing panics, no second boil. Task 5,
   `the_second_solve_is_what_the_class_reads`; Task 9 counts the living
   reference planets whose second climate has no liquid water.
2. **Bodies without a surface in `model`.** A giant and a mini-Neptune
   give `surface: None`, an interior whose layers validate
   (`PlanetState::validate`), no life, no spin call in `class_of`.
   Task 7, `a_giant_is_modelled_without_a_surface`.
3. **An asked obliquity.** An `earth-like` trial (archetype obliquity
   asked) gives the same `Provenance::Input` obliquity to `class_of`'s
   climate and to `model`'s. Task 7,
   `an_asked_obliquity_reaches_both_paths`.
4. **Extreme inputs.** Mass 0.01 and 20 M⊕, flux 1e-3 and 1e4 S⊕, age
   0.5 and 13 Gyr: `class_of` returns `Ok` or a `SystemError::Planet`,
   never a panic or a NaN in the outcome, and every `PlanetState`
   validates (legacy `every_derived_value_is_finite_on_extreme_inputs`).
   Task 7, tier 3.
5. **An eccentric placement.** `model` on the eccentric trial may give
   another class than the draw's circular one (K1). Expected: no error,
   a valid state, the change counted and pinned. Task 7,
   `an_eccentric_placement_is_modelled_on_its_own_orbit`; Task 9,
   `class_changes_between_draw_and_model_are_pinned`.

---

### Task 1: Spin from the trial seed (sonnet)

**Files:**

- Modify: `crates/system/src/spin.rs` (rocky branch l.~96-114 per B2;
  doc l.59-78: drawn from `seed.derive("spin")`, the trial seed, and
  l.74-78 says the rocky vector is drawn in units of `ω_cr`; parameter
  `body_seed` → `seed`; `is_giant` `pub(crate)`),
  `crates/system/src/draw/mod.rs` (l.44-52 doc, l.69-78:
  `planet.trial.seed` passed to `spin::spin`),
  `crates/system/src/placement.rs` (l.1-22 doc),
  `crates/system/src/draw/tests.rs:714-737` (B4),
  `crates/regression/tests/determinism.rs` (the eight digests, K2).
- Stale seed docs (S9): `planet/draws.rs:5` and `:179-180` ("`model`
  the body seed"), `planet/tests.rs:132-134` (the comment of
  `the_draw_follows_the_seed_it_is_given_not_the_trials`),
  `crates/system/README.md` l.45-47 (Streams: the spin from the trial
  seed), l.94-100 (The seam: `model` draws from the trial seed), the
  `placement` and `spin` rows, and "Planet interior" l.~63-65.
  `lib.rs:34` is history: keep it.

**Interfaces:**

- Produces: `spin::spin(class, &mass, radius, archetype, seed, trace)`
  unchanged in shape; for any two classes with `is_giant` false, any
  mass and radius, the same `seed` and `archetype` give the same
  `obliquity` value bit for bit. `spin::is_giant(class) -> bool`,
  `pub(crate)`.

- [ ] **Step 1: Write the failing test** in `spin.rs` tests:
  `the_rocky_obliquity_depends_only_on_the_seed`: for seeds 0..200,
  classes `Rocky`, `SuperEarth`, `OceanWorld`, `SteamWorld`,
  `MiniNeptune`, masses 0.1, 1, 5 M⊕ and radii 0.5, 1.0, 1.643, 2.5 R⊕,
  `obliquity.value()` equals (`assert_eq!` on the `f64`, no tolerance)
  that of `(Rocky, 1 M⊕, 1 R⊕)` with the same seed. The distribution
  tests `spin.rs:201-268` stay unchanged and must still pass.
- [ ] **Step 2: Run** `cargo nextest run -p system spin`. Expected: the
  new test FAILS on at least one seed (last-bit differences); if it
  passes everywhere, say so in the report and keep the refactor (the
  law's doc requires the cancellation).
- [ ] **Step 3: Implement** B2 in `spin.rs`, then the seed line and the
  docs. `placement.rs`'s doc says: the trial's seed is
  `system_seed.derive("trial/<archetype id>/<k>")` or
  `"trial/slot-<n>"` (check the exact labels at `place.rs:162-165`,
  `:184`), unique per planet; `class_of` and `model` both draw the
  chain from it, and the spin is drawn from it too, so that the
  obliquity the climate reads is the placement's; the draw classifies
  the circular trial and `model` runs on the eccentric one (K1); the
  body seed stays for what is drawn after the designation (none in the
  planet model).
- [ ] **Step 4: Run** `cargo nextest run -p system`, then
  `cargo nextest run -p regression` to read the new golden digests
  (the spins are hashed, `regression/src/lib.rs:94-104`) and paste
  them into `EXPECTED`. The `SystemId` pin does not move (it reads the
  engine version only; check it once). Run `mise run ci`. Expected:
  green. Commit `feat(system): draw the spin from the trial seed`; the
  body says the digests moved because the spins now come from the
  trial seed; footer `Refs: #677`.

### Task 2: Life and class laws in `science` (opus)

**Files:**

- Create: `crates/science/src/life.rs`, `crates/science/src/life_tests.rs`,
  `crates/science/src/classes.rs`, `crates/science/src/classes_tests.rs`.
- Modify: `crates/science/src/lib.rs`, `crates/science/src/volatiles.rs`
  (the triple-point pressure public, below),
  `crates/science/src/surface/condense.rs:~88` (`titan_like_air` reads
  it), `crates/science/src/outgassing.rs` (`column_mass`),
  `crates/system/src/families/mod.rs` (S5, D5: the definition at
  l.45-46 removed, l.114 reads `HOT_GIANT_MAX_PERIOD`),
  `crates/system/src/families/laws.rs` (l.11 imports
  `science::classes::HOT_GIANT_MAX_PERIOD` instead of the family's
  constant, l.344 reads it), `crates/science/README.md`, the two
  `docs/science` sections (name the functions; "Life" gains the
  sentence of S8: the second solve takes no boil verdict, the class and
  the surface read the second climate, the biosphere the first).

**Interfaces:**

- Produces, `science::life` (all `pub`):
  - `const LIFE_MIN_GYR: Gyr = Gyr(1.17)` (`Source:` 4.567 Ga of
    Connelly et al. 2012 minus 3.4 Ga of Javaux 2019);
  - `const LIFE_CHANCE: f64 = 1.0 / 3.0` (`Definition:` game choice,
    user 2026-11-12, no measured rate; Spiegel & Turner 2012);
  - `const OXYGEN_AFTER_GYR: Gyr = Gyr(2.2)` and
    `const MODERN_OXYGEN_AFTER_GYR: Gyr = Gyr(4.0)` (`Source:` Lyons,
    Reinhard & Planavsky 2014, pages as the doc page);
  - `const CIPM_N2_SHARE: f64 = 0.780_848`,
    `const CIPM_O2_SHARE: f64 = 0.209_390` (`Source:` Picard et al.
    2008, Table 1) and `const O2_PER_N2: f64 = CIPM_O2_SHARE /
    CIPM_N2_SHARE` (`Definition:` their ratio);
  - `enum Stage { Lifeless, Anoxic, Oxidising, Modern }` (`Copy`,
    `Eq`, `Debug`);
  - `fn stage(water: bool, age: Gyr, draw: f64) -> Stage`: `Lifeless`
    unless `water && age >= LIFE_MIN_GYR && draw < LIFE_CHANCE`; then
    by age, `>=` at each step;
  - `fn oxygenate(air: &PartialPressures, stage: Stage) ->
    PartialPressures`: `Lifeless`, `Anoxic` unchanged; `Oxidising`
    `ch4 = 0`; `Modern` `ch4 = 0`, `o2 = O2_PER_N2 * n2`; other gases
    unchanged.
- Produces, `science::classes` (all `pub`):
  - `const HOT_GIANT_MAX_PERIOD: Day = Day(10.0)` (`Source:` Wright et
    al. 2012 § 3; Dawson & Johnson 2018 use the same line), read by the
    class and by `families` (S5);
  - `const SUPER_EARTH_MIN_RADIUS: EarthRadius = EarthRadius(1.25)`
    (`Source:` Borucki et al. 2011);
  - `const BASIN_OCEANS: f64 = 3.7` (`Source:` Cowan & Abbot 2014
    § 2.4) and `fn basin_capacity_oceans(radius: EarthRadius, gravity:
    MetrePerSecondSquared) -> f64` = `BASIN_OCEANS · R² · (g⊕ / g)`,
    `g⊕` = `constants::STANDARD_GRAVITY` (Eq. 11; the doc comment says
    the standard gravity stands for Earth's, a `Definition:`);
  - `const DENSE_AIR_BAR: Bar = Bar(1.0)` (`Calibration:` with what
    was searched, link to "The dense air");
  - `const ICE_WORLD_MIN_OCEANS: f64 = 0.01` (`Calibration:` #452, the
    page's searched-not-found, Carr & Head 2015 not reached).
- Produces: `science::volatiles::methane_triple_pressure() -> Bar`
  (`methane_saturation_bar(METHANE_TRIPLE_TEMPERATURE)`), read by
  `titan_like_air` and by the class; `science::outgassing::column_mass(
  pressure: Bar, gravity: MetrePerSecondSquared, radius: Metre) ->
  Kilogram` (B6).

- [ ] **Step 1: Write the failing tests.** `life_tests.rs`:
  - `life_waits_for_the_oldest_accepted_evidence`: `stage(true,
    Gyr(1.16), 0.0)` is `Lifeless`, `stage(true, LIFE_MIN_GYR, 0.0)`
    `Anoxic`;
  - `the_ages_of_a_living_earth`: draw 0, ages 0.5, 1.5, 3.0, 4.6 Gyr
    give `Lifeless`, `Anoxic`, `Oxidising`, `Modern` (the page's
    table); `OXYGEN_AFTER_GYR` and `MODERN_OXYGEN_AFTER_GYR` exactly
    give `Oxidising` and `Modern`;
  - `a_habitable_world_can_be_lifeless`: draws 0.99 and exactly
    `LIFE_CHANCE` give `Lifeless` at 4.6 Gyr;
  - `no_water_no_life`: `water` false, draws 0, 0.5, 0.99, ages 0.5
    and 4.6: `Lifeless`;
  - `o2_per_n2_is_the_cipm_ratio`: within 1e-4 of 0.2682 (the page;
    N4: no self-comparison);
  - `oxygen_comes_in_two_steps`: on an air of N₂ 0.78, CH₄ 1e-3, O₂ 0,
    CO₂ 4e-4 bar: `Anoxic` unchanged; `Oxidising` CH₄ 0, O₂ 0, the rest
    unchanged; `Modern` CH₄ 0, O₂ `O2_PER_N2 × 0.78` exactly, in 0.15
    to 0.30 bar (legacy `earth_has_oxygenic_life`).
  `classes_tests.rs`:
  - `an_earth_holds_three_point_seven_oceans`: `basin_capacity_oceans(
    EarthRadius(1.0), STANDARD_GRAVITY)` is 3.7 within 1e-12; twice
    the radius and twice the gravity give 7.4;
  - `titan_like_air_reads_the_triple_point_pressure` (N3: the value
    itself is already tested at `volatiles_tests.rs:230-233`): an air
    of CH₄ 1e-3 bar and a total a hair above `methane_triple_pressure()`
    is Titan-like, a hair under is not;
  - `column_mass_inverts_column_pressure`: `column_pressure(
    column_mass(p, g, r), g, r)` equals `p` within 1e-12 for Earth's
    1 bar, g and radius.
- [ ] **Step 2: Run** `cargo nextest run -p science life classes
  condense`. Expected: FAIL (no modules).
- [ ] **Step 3: Implement** the interfaces. Doc comments quote the page
  sections; the guardrail checks every constant's tag.
- [ ] **Step 4: Run** the same tests. Expected: PASS. Update
  `crates/science/README.md` (one line per module) and name the
  functions in "Life" and "The chain and the surface classes". Run
  `mise run ci`. Expected: green. Commit
  `feat(science): add the life stages and the class thresholds`.

### Task 3: `Biosphere` in the timeline contract (sonnet)

**Files:**

- Modify: `crates/timeline/src/planet.rs:~280-290` (`Life` →
  `Biosphere`; `Surface.life: Traced<Biosphere>`; `traced_fields!`),
  `crates/timeline/src/lib.rs` (exports), `crates/timeline/src/fixtures.rs:121-122`,
  `crates/timeline/README.md` (if it lists `Life`).

**Interfaces:**

- Produces: `timeline::Biosphere` with variants `Lifeless => "lifeless"`,
  `Anoxic => "anoxic"`, `Oxidising => "oxidising"`, `Modern =>
  "modern"` (doc: the air life has made: none, the air it found, CH₄
  gone, modern O₂); `Surface.life: Traced<Biosphere>`.

- [ ] **Step 1: Write the failing test** in `planet.rs` tests:
  `biosphere_keys_are_kebab_case`: `serde_json::to_string` of each
  variant is `"\"lifeless\""`, `"\"anoxic\""`, `"\"oxidising\""`,
  `"\"modern\""`, and they read back. The fixture's keyframe round trip
  (`keyframe_tests.rs:61`) must still pass with `life: leaf(trace,
  "life", Biosphere::Modern)`.
- [ ] **Step 2: Run** `cargo nextest run -p timeline`. Expected: FAIL.
- [ ] **Step 3: Implement.** No other crate builds a `Life` (checked:
  only `fixtures.rs`).
- [ ] **Step 4: Run** `cargo nextest run --workspace --no-run` then
  `mise run ci`. Expected: green. Commit
  `feat(timeline)!: replace Life with the Biosphere stage` with a
  `BREAKING CHANGE:` footer (the `Surface.life` field's type) and
  `Refs: #602`.

### Task 4: The sea share of a water surface (sonnet)

**Files:**

- Modify: `crates/system/src/planet/climate/record.rs` (a recorder
  beside `record`; the doc at l.36-37 says the inventory is the water's
  node and the share is `sea_share`'s), `codes.rs`
  (`SURFACE_OCEAN_SHARE = "planet.surface.ocean-share"`).
- Test: `crates/system/src/planet/settled_tests.rs`.

**Interfaces:**

- Produces:

  ```rust
  /// The share of the surface under water (B6): `ocean_fraction` of
  /// the water kept in Earth oceans, the sea term the cover reads.
  /// `None` unless the liquid is water.
  pub(crate) fn sea_share(water: &Water, liquid: Option<&Traced<LiquidKind>>,
      trace: &mut Trace) -> Result<Option<Traced<Fraction>>, InteriorError>;
  ```

  Parents: the water inventory and the liquid node (S10).

- [ ] **Step 1: Write the failing tests** (tier 1, on bodies tier-1
  tests already settle):
  - `a_wet_surface_records_its_sea_share`: on a settled wet body (pick
    one under 5 s at opt-level 0, for example a fixture of
    `wet_tests.rs`; check its time), the share equals
    `ocean_fraction(inventory / OCEAN_MASS)` exactly and its parents
    are the inventory and liquid nodes;
  - Venus (`venus_is_bright_and_has_no_liquid`) gives `None`.
- [ ] **Step 2: Run** `cargo nextest run -p system settled`. Expected:
  FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the tests, then `mise run ci`. Expected: green.
  Commit `feat(planet): record the sea share of a water surface`.

### Task 5: Life, the oxygenated air and the second solve (opus)

**Files:**

- Create: `crates/system/src/planet/life.rs`,
  `crates/system/src/planet/life_tests.rs`.
- Modify: `crates/system/src/planet/draws.rs:160-176` (`Labels.life`,
  `"planet/life"`; `fn life_draw(seed: Seed) -> f64` beside
  `snowball_draw`, l.~329, same one-uniform form), `codes.rs`
  (`LIFE = "planet.life"`, `LIFE_DRAW = "planet.life.draw"`,
  `LIFE_AIR_O2`, `LIFE_AIR_CH4`, `LIFE_AIR_PRESSURE` =
  `"planet.life.air.o2"`, `".ch4"`, `".pressure"`), `planet/mod.rs`
  (`mod life;` under the same `expect(dead_code, …)` until Task 7).

**Interfaces:**

- Consumes: `science::life::{stage, oxygenate, Stage}` (Task 2),
  `timeline::Biosphere` (Task 3), `settled::climate_of`,
  `Settled`, `Rotation`.
- Produces:

  ```rust
  pub(crate) struct Living {
      /// The settled planet; its bare air oxygenated when the biosphere
      /// made oxygen, its climate the second solve's then.
      pub settled: Settled,
      /// `None` for a planet that kept an envelope.
      pub life: Option<Traced<Biosphere>>,
  }
  pub(crate) fn living(
      settled: Settled,
      age: &Traced<Gyr>,
      life: f64,            // life_draw(seed.derive(LABELS.life))
      rotation: &Rotation,
      snowball: f64,        // the same draw value the first solve used
      trace: &mut Trace,
  ) -> Result<Living, InteriorError>;
  ```

  Steps: no climate → `life: None`, settled unchanged. Else record the
  draw leaf (`LIFE_DRAW`, `Provenance::Drawn`, a `Fraction`), the
  stage from `climate.liquid == Some(Water)`, the age and the draw, the
  `LIFE` node (parents per B7). `Oxidising` or `Modern`: the bare air
  oxygenated (B8; its `Atmosphere` with the new O₂ and CH₄ nodes and
  pressure), then `climate_of(&oxygenated, rotation, snowball, trace)`
  replaces `settled.climate`; `hides_surface` stays (false: liquid
  water); no verdict.

- [ ] **Step 1: Write the failing tests** in `life_tests.rs` (tier 1
  unless timed over 5 s):
  - `life_draws_from_its_own_sub_seed`: `life_draw(seed.derive("planet/life"))`
    is in (0, 1), differs from the snowball draw of the same seed, and
    the same seed gives the same value twice;
  - `venus_and_mars_are_lifeless`: settled `VENUS` and `MARS`
    (`settled_of`, as `venus_is_bright_and_has_no_liquid`), draws 0
    and 0.99: the `planet.life` node is `Lifeless`, its parents include
    the age and the draw, and the climate is the first one (same
    `surface_temperature` node);
  - `the_second_solve_is_what_the_class_reads` (Review Focus 1), on a
    wet non-bistable body the implementer picks and times (a warm
    water world of the existing fixtures; if none settles under 5 s at
    opt-level 0, tier 3): with draw 0 and age 4.6, `life` is `Modern`,
    the bare air's O₂ node is `O2_PER_N2` × the N₂ node's value
    exactly, CH₄ 0, `settled.climate`'s temperature node is a new node
    whose ancestry contains `planet.life.air.o2`, no second
    `planet.climate.boils` node was recorded, and with draw 0.99 the
    climate is the first one.
  - tier 3, in `climate/heavy_tests.rs`,
    `the_reference_earth_lives_and_reaches_its_climate`: `EARTH`,
    draw 0, the trial's age 4.5673 Gyr: `Modern`, O₂ as above, CH₄ 0;
    the oxygenated bare air with its CO₂ set to `PREINDUSTRIAL_CO2`
    and solved (`climate_of`) lies in `PREINDUSTRIAL_RANGE` (286 to
    290 K). This replaces the measured-air block of
    `earth_has_its_climate` (`settled_tests.rs:658-680`) only if that
    block's own bound still holds; keep both otherwise. A failure stops
    the task.
- [ ] **Step 2: Run** `cargo nextest run -p system life`. Expected:
  FAIL.
- [ ] **Step 3: Implement** `life.rs`. Port legacy `life.rs` l.39-70
  through the `science` laws; `life_draw_is_the_last_draw` is not
  ported (it pins legacy's stream).
- [ ] **Step 4: Run** the tests, then the tier-3 one with
  `cargo nextest run -p system --release --run-ignored only
  the_reference_earth`. Expected: PASS. Run `mise run ci`. Commit
  `feat(planet): add life and solve the climate again on its air`.

### Task 6: The class (sonnet)

**Files:**

- Create: `crates/system/src/planet/classify.rs`,
  `crates/system/src/planet/classify_tests.rs`.
- Modify: `codes.rs` (`CLASS = "planet.class"`), `planet/mod.rs`
  (`mod classify;` under the `expect` until Task 7),
  `crates/system/src/planet/heat.rs:19` (doc: the ice world reads it),
  `crates/system/src/planet/tests.rs:850-860` (N7: a `neptune_at(trace,
  au) -> (Trial, Drawn)` helper beside `jupiter_at`, from the `NEPTUNE`
  tuple), `settled_tests.rs` (`settled_of` `pub(super)` if
  `classify_tests.rs` uses it).

**Interfaces:**

- Consumes: `science::classes` (Task 2),
  `science::volatiles::methane_triple_pressure`,
  `heat::ICY_MIN_WATER`, `envelope::MINI_NEPTUNE_MAX_MASS` (made
  `pub(crate)`), `Settled`, `Body`, `science::habitable::OCEAN_MASS`.
- Produces:

  ```rust
  /// What the class reads, as plain values (legacy `classify`'s inputs).
  pub(crate) struct ClassView {
      pub kind: BodyKind,             // Rocky | MiniNeptune | Giant, from Body
      pub mass: EarthMass,            // the bulk's (kept) mass, N9
      pub period: Day,                // the trial orbit's
      pub surface: Option<SurfaceView>,
  }
  pub(crate) struct SurfaceView {
      pub hides_surface: bool,
      pub liquid: Option<LiquidKind>, // the final climate's
      pub runaway: bool,
      pub air: Bar,                   // the final climate's surface pressure
      pub oceans: f64,                // water inventory / OCEAN_MASS
      pub capacity: f64,              // basin_capacity_oceans(R, g)
      pub water_fraction: f64,        // bare.drawn.water_fraction (kept), N10
      pub regime: TectonicRegime,
      pub radius: EarthRadius,
  }
  pub(crate) fn class(view: &ClassView) -> PlanetClass;
  pub(crate) fn classify(trial: &Trial, settled: &Settled,
      trace: &mut Trace) -> Result<Traced<PlanetClass>, InteriorError>;
  ```

  `class` follows the page's table in order, the first rule that holds
  wins. The table's words set the comparison: "at least" and "within"
  are `>=` and `<=` (giant mass line, ice giant, hot period, steam line,
  ice world, super-Earth), "over" and "exceed" are `>` (`DENSE_AIR_BAR`,
  the triple point, the basin capacity). `classify` builds the view
  from `settled` and records `CLASS` from the nodes the rules up to the
  winning one read (mass, period, `hides_surface`, liquid, runaway, the
  air pressure node, the water inventory, the bulk's radius and
  gravity, the water fraction, the regime). The page's ice-world row
  says the water fraction is the kept planet's (Task 12).

- [ ] **Step 1: Write the failing tests** in `classify_tests.rs`:
  - `each_rule_of_the_table` (tier 1, synthetic views): one view per
    row gives its class; precedence pairs: a hot `Giant` (period
    `HOT_GIANT_MAX_PERIOD`) is `HotJupiter`, one day more `GasGiant`;
    a `MiniNeptune` of exactly 10 M⊕ is `IceGiant` and hot
    `HotNeptune`, of 9.99 M⊕ `MiniNeptune` whatever its period (N9:
    the bulk mass); a steam world that is also a runaway over 1 bar is
    `SteamWorld`; lava over runaway; a runaway at exactly 1 bar is not
    `RunawayGreenhouse`; an ocean over the capacity with `HeatPipe` is
    `OceanWorld`; a heat-pipe ice world stays `IceWorld`; `HeatPipe`
    otherwise `VolcanicWorld`; radius `SUPER_EARTH_MIN_RADIUS` is
    `SuperEarth`, a hair under `Rocky`; an icy body (`water_fraction`
    = `ICY_MIN_WATER`) with oceans just under `ICE_WORLD_MIN_OCEANS`
    is not `IceWorld` (#452);
  - `the_solar_system_classifies_as_itself` (tier 1 where each body is
    cheap; Earth goes to the tier-3 test of Task 5, asserting `Rocky`):
    Mercury `Rocky`, Venus `RunawayGreenhouse` with runaway true and
    boiled false, Mars `Rocky`, Callisto `IceWorld` with no CH₄ and no
    liquid, Jupiter (`jupiter_at(5.2)`) `GasGiant`, Neptune
    (`neptune_at(30.0)`) `IceGiant`. Titan waits for Task 11. Ganymede
    and Europa only if `legacy-final:crates/mapgen/src/planet/reference.rs`
    gives their values with a source; else listed in the report;
  - `a_close_gas_giant_is_a_hot_jupiter`: `jupiter_at(0.05)` is
    `HotJupiter`, `jupiter_at(0.2)` `GasGiant` with its period over
    10 d;
  - `a_close_ice_giant_is_a_hot_neptune`: `neptune_at` inside 10 d is
    `HotNeptune` if it keeps its envelope (assert the body kind first;
    pick the orbit by the period, not by the result), at 30 au
    `IceGiant`;
  - `an_enveloped_planet_is_a_mini_neptune`: 5 M⊕ with a 2 % envelope
    at 0.5 au is `MiniNeptune`; with none, `SuperEarth` or `Rocky` by
    its radius;
  - `a_planet_that_lost_its_water_is_not_an_ice_world` (#452: 0.22 M⊕,
    37 % water, 0.6 au; `oceans < 0.01`, not `IceWorld`) and
    `a_cold_icy_planet_stays_an_ice_world` (0.5 M⊕, 30 %, 4 au):
    tier 1 if under 5 s each, else tier 3;
  - tier 3, added to the existing heavy tests (no new solve):
    `the_492_super_earth_is_a_steam_world` asserts `SteamWorld`;
    `a_boil_that_escapes_all_its_water_ends_dry` asserts
    `RunawayGreenhouse`; `a_boil_whose_leftover_recondenses_keeps_its_ocean`
    asserts not `RunawayGreenhouse`; a new
    `a_migrated_water_world_in_the_habitable_zone_is_an_ocean_world`
    (1 M⊕, 30 % water, 1 au, legacy l.428) asserts `OceanWorld`;
    `a_runaway_weighs_the_water_its_own_pass_kept` (legacy l.704) on
    the #492 body and the K5V bodies (0.3 and 0.6 M⊕, 10 % water,
    0.4 au; star fixtures as `with_star` with K5V's sourced effective
    temperature): the class is read on the kept pass, and the
    steam world's layers sum to its whole mass within 1e-9.
- [ ] **Step 2: Run** `cargo nextest run -p system classify`. Expected:
  FAIL.
- [ ] **Step 3: Implement.** Port legacy `classify.rs` l.206-259 rule by
  rule through `class`.
- [ ] **Step 4: Run** the tier-1 tests and the touched tier-3 ones in
  release. Expected: PASS; a Solar System class that fails stops the
  task. Run `mise run ci`. Commit `feat(planet): classify the planet`.

### Task 7: The chain, `class_of` and `model` (opus)

**Files:**

- Create: `crates/system/src/planet/chain.rs`,
  `crates/system/src/planet/chain_tests.rs`.
- Modify: `crates/system/src/planet/mod.rs` (doc l.1-65 rewritten for
  what exists; remove the 15 `expect(dead_code, …)` attributes
  l.66-141 and those Tasks 5-6 added; `pub fn class_of`, `pub fn
  class_read`, `pub fn model`, `pub use interior::InteriorError` (N2)),
  `crates/system/src/planet/settled.rs` (split `settled_with`
  l.122-169: `settled_on(formed: Formed, rotation: &Rotation,
  snowball: f64, trace)` after `formed`, so the chain gets the ground
  before the spin; `settled_with` keeps its `Seed` parameter for the
  tests and calls `settled_on` with `snowball_draw(seed)`; doc l.1-14),
  `crates/system/src/planet/interior.rs:100` (`InteriorError` becomes
  `pub`, `#[non_exhaustive]`), `crates/system/src/error.rs`
  (`SystemError::Planet(InteriorError)`, `From`, `Display`
  `system.planet: {error}` as the other variants, N1),
  `crates/system/src/lib.rs` (re-export), `codes.rs` (`SPIN_LOCKED =
  "planet.spin.locked"`, `INTERIOR_SAMPLES = "planet.interior.samples"`,
  `LIQUID_INVENTORY` and the imposed `SURFACE_OCEAN_SHARE` leaf of B6).

**Interfaces:**

- Consumes: Tasks 1, 4, 5, 6.
- Produces:

  ```rust
  /// Where the chain's obliquity comes from (B3).
  enum Spun<'a> { Trial, Placed(&'a Spin) }
  struct Chained {
      living: Living,
      class: Traced<PlanetClass>,
      /// The rotation the climate read; `None` without a surface.
      rotation: Option<Rotation>,
  }
  fn chain(trial: &Trial, spun: Spun<'_>, trace: &mut Trace)
      -> Result<Chained, SystemError>;
  /// The same with the composition and the two draw values given (tests).
  fn chain_with(trial: &Trial, drawn: &Drawn, spun: Spun<'_>,
      snowball: f64, life: f64, trace: &mut Trace)
      -> Result<Chained, SystemError>;
  /// What `class_of` read (D2).
  pub struct ClassRead { pub outcome: TrialOutcome, pub obliquity: Option<Degree> }
  pub fn class_read(trial: &Trial, trace: &Trace) -> Result<ClassRead, SystemError>;
  pub fn class_of(trial: &Trial, trace: &Trace)
      -> Result<TrialOutcome, SystemError>;
  pub fn model(placement: &Placement, trace: &mut Trace)
      -> Result<PlanetState, SystemError>;
  ```

  `chain` returns `SystemError` (S1: `spin::spin` returns it; the
  steps' `InteriorError`s convert through `From`). `chain`:
  `draw(trial.seed, …)`; the snowball and life draws as values,
  `snowball_draw(trial.seed.derive(LABELS.snowball))` and
  `life_draw(trial.seed.derive(LABELS.life))` (N5); `formed`;
  enveloped → no rotation, `classify` on the enveloped `Settled`; else
  the rotation (`Spun::Trial` → `spin::spin` per B3; `Placed` → its
  `obliquity` node), `locked` the imposed `SPIN_LOCKED` false leaf,
  `settled_on`, `living`, `classify`, then the `debug_assert!` of B3.
  `class_read` clones the trace, runs `chain(trial, Spun::Trial, &mut
  scratch)` and returns the class value, `bulk().radius` and
  `bulk().mean_density` values and the rotation's obliquity value.
  `model` runs `chain(&placement.trial, Spun::Placed(&placement.spin),
  trace)` and builds `PlanetState` per B5, B6, B13 (the sea share of
  Task 4); it calls `validate()` before returning.

- [ ] **Step 1: Write the failing tests** in `chain_tests.rs` (tier 1,
  each timed):
  - `the_same_trial_gives_the_same_answer` (K1): three cheap trials at
    `e = 0` (a dry hot rocky planet at `HOT_ORBIT_AU`, a cold one at
    5 au, a giant of 318 M⊕ at 5.2 au; seeds chosen once and named):
    `class_read(&t, &trace)`, then a `Placement` built as `draw_with`
    does (`spin::spin(outcome.class, &t.mass, outcome.radius,
    archetype, t.seed, &mut trace)`) on the same `t`, then `model`:
    `model.class`, `bulk.radius`, `bulk.mean_density` values equal the
    outcome's (`assert_eq!`, no tolerance); for the two rocky trials
    the read's obliquity equals `placement.spin.obliquity.value()`, and
    `model`'s chain read the placement's node itself (same `NodeId`);
    for the giant the read's obliquity is `None`;
  - `an_eccentric_placement_is_modelled_on_its_own_orbit` (Review
    Focus 5): the cold rocky trial with `e = 0.3` and its sources at
    that eccentricity (`sources()` as `place.rs:384-398`): `model` is
    Ok and valid, its orbit is the eccentric one and its sources' flux
    the higher mean flux; the class may differ from the circular one's
    (not asserted);
  - `a_giant_is_modelled_without_a_surface` (Review Focus 2): Jupiter
    and `neptune_at(30.0)`: `surface: None`, `validate()` Ok, layers
    the `Body::layers()` nodes (same `NodeId`s), samples empty;
  - `an_asked_obliquity_reaches_both_paths` (Review Focus 3): a trial
    with `archetype: Some(earth-like)` on a cheap composition through
    `chain_with` with `Spun::Trial` and with `Spun::Placed`: the
    climate's obliquity is the archetype's value, `Provenance::Input`,
    in both;
  - `an_interior_error_is_a_system_error`:
    `SystemError::from(InteriorError::NoEnvelope)` is
    `SystemError::Planet(_)` and its `to_string()` starts with
    `system.planet: interior-` (N1);
  - `the_planet_state_holds_the_chain` on the settled Venus through
    `chain_with`: `surface.water` is the settled water, `liquid` None,
    `life` `Lifeless`, `class` `RunawayGreenhouse`,
    `surface.terrain.units` empty, `targets` empty, `surface.atmosphere`
    the climate's, the frozen graph (`trace.freeze(&mut state)`) is Ok;
  - tier 3: `every_planet_state_is_finite_on_extreme_inputs` (Review
    Focus 4): masses 0.01 and 20 M⊕, fluxes 1e-3 and 1e4 S⊕ (as
    `with_fluence`-style trials), ages 0.5 and 13 Gyr: `class_of` is
    Ok or `SystemError::Planet`; each Ok one gives a `model` whose
    serialised JSON has no NaN or infinity (port legacy `not_finite`,
    `first_bad`, l.1171-1190).
- [ ] **Step 2: Run** `cargo nextest run -p system chain`. Expected:
  FAIL.
- [ ] **Step 3: Implement.** Remove the `expect(dead_code)` attributes
  now; an item still dead outside tests is either used by the chain or
  moved under `#[cfg(test)]`, never given a new `expect`. Rewrite the
  stale docs: `planet/mod.rs:1-65` (signatures as they are,
  `SystemError::Planet`, the trial seed of both paths, the circular
  and eccentric trials, the order of the chain), `settled.rs:12-14`,
  `climate/record.rs:36-37`.
- [ ] **Step 4: Measure the budgets (S3).** With a throwaway test or
  example (not committed), time one `draw_with(…, planet::class_of)` of
  each of the eight `reference_requests()`, then `model` on each
  placement, at opt-level 0 (the test profile) and in release. Report
  the eight times and their sum. Tier 2 will run the draw and the
  models once (Task 9), the moved CLI and determinism tests (Task 8)
  and the existing regression tests: if the projected tier 2 exceeds
  480 s, **stop and report** before Task 8 (test-reviewer proposes,
  the user decides).
- [ ] **Step 5: Run** the tests, the tier-3 one in release, then
  `mise run ci`. Expected: green. Commit
  `feat(planet): chain the planet behind class_of and model` with
  `Refs: #659, #677`.

### Task 8: Wiring, engine version and the tier-2 moves (sonnet)

**Files:**

- Create: `crates/cli/src/lib.rs` (D1: the modules of `main.rs`, `pub
  fn dispatch(cli: &Cli) -> Result<String, CliError>`, `report`,
  `explain` and `Cli` public), `crates/regression/tests/cli.rs`.
- Modify: `crates/cli/Cargo.toml` (`[lib] name = "map_generator"`),
  `crates/cli/src/main.rs` (calls the library),
  `crates/cli/src/report.rs:5`, `:26-28`, `:36-55`, `:61`, `:89`
  (`planet::class_of`; `classifier` removed; `PlanetDraw` gains
  `state: PlanetState` from `planet::model`, D3, visited by
  `for_each_node`), `crates/cli/README.md:30-36` (the planets carry
  their state and class), `crates/cli/tests/system.rs` (D1: the
  non-drawing tests stay tier 1; the drawing ones keep their binary
  part as tier 3),
  `crates/regression/Cargo.toml` (depends on `map-generator`),
  `crates/regression/src/lib.rs:16-18`, `:53-63` (`stand_in_model`
  re-export removed; `planet::class_of`),
  `crates/system/tests/determinism.rs` (moved to
  `crates/regression/tests/determinism.rs` as
  `the_draw_is_pure_within_a_run`: draw the first reference request
  again and compare its digest with the shared draw's; the file in
  `system/tests` is deleted, B1),
  `crates/system/src/lib.rs:31-38` (`EngineVersion(4)`, doc line "4:
  the planet model classifies the draw (class, radius, density from the
  chain), and spins are drawn from the trial seed"),
  `crates/system/README.md` (`stand_in` row: the draw's own tests only;
  `planet` row: what is wired; `ENGINE_VERSION` l.~109),
  `crates/regression/tests/determinism.rs` (golden digests),
  `crates/regression/tests/contracts.rs` (the `SystemId` pin
  `528d67d35f485f94`), `CLAUDE.md` nothing (the tier definition does
  not change).

- [ ] **Step 1: Split the CLI** into library and binary (D1) with no
  behaviour change; `mise run ci` green before going on.
- [ ] **Step 2: Move the drawing tests** to `crates/regression/tests/cli.rs`
  through `map_generator::{report, explain, dispatch}`: the three of
  `report.rs:112-147` and the seven of `explain.rs:340-440` (their
  assertions unchanged), and the JSON assertions of the binary tests
  that draw (`prints_the_draw_as_json`, now asserting each planet has
  `state.class` and no `classifier`; `the_same_seed_prints_the_same_bytes`
  through `dispatch`; `explain_prints_a_tree`,
  `explain_a_code_held_by_no_field_names_its_dependents`). The binary
  parts that need a draw (exit code 1 on an explain error, closed pipe,
  drawn-seed notice and replay) become tier 3 in `cli/tests/system.rs`.
- [ ] **Step 3: Switch the callers** (`class_of` in the report,
  regression and the moved determinism test) and the engine version.
  Run `cargo nextest run -p regression` once to read the new digests
  and `SystemId`; paste them into the two tests (tier-2 pins; the
  commit body says why they change, and S4).
- [ ] **Step 4: Run** `mise run test:tier1` and `mise run test:tier2`
  and note both totals and the slowest ten tests
  (`target/nextest/*/junit.xml`). Expected: tier 1 under 120 s with no
  test drawing a system with the real classifier, tier 2 under 480 s;
  over either, stop and report.
- [ ] **Step 5: Run** `mise run ci`. Expected: green. Commit
  `feat(system)!: classify the draw with the planet model` with a
  `BREAKING CHANGE:` footer (engine 4: every cached system and world
  is invalid) and `Refs: #602`.

### Task 9: The tier-2 net (sonnet)

**Files:**

- Modify: `crates/regression/src/lib.rs` (`REFERENCE_SEEDS` =
  `reference_requests()`'s seeds 1 to 8; `pub struct Reference {
  pub inputs: Inputs, pub drawn: Drawn, pub trace: Trace, pub planets:
  Vec<PlanetState>, pub reads: Vec<(Seed, ClassRead)> }`; `pub fn
  references() -> &'static [Reference]`, drawn once with a classifier
  closure that calls `planet::class_read` and logs each read by trial
  seed (D2), then `model` on each placement with the kept trace;
  `reference_systems()` reads the same single draw (S6: returns
  `Vec<&'static Drawn>` built from `references()`); `digest` and the
  contracts test read it), `crates/regression/tests/contracts.rs:8`,
  `crates/regression/tests/determinism.rs` (the iterator).
- Create: `crates/regression/tests/planets.rs`.

- [ ] **Step 1: Write the tests** in `planets.rs`:
  - `every_reference_planet_is_modelled`: one `PlanetState` per
    placement, `validate()` Ok;
  - `class_changes_between_draw_and_model_are_pinned` (K1, Review
    Focus 5): per placement, the draw's class (the value of the
    `system.draw.class` node whose parents include that placement's
    `trial.mass` node) against the model's; the changes are listed by
    system and designation with their eccentricity, and the count and
    the list are pinned (equal for every `e = 0` placement);
  - `the_obliquity_class_of_read_is_the_placements` (mapgen3's
    condition): for every placement whose model has a surface, its
    class has `!is_giant` (the three giant classes, with a comment
    naming `spin::is_giant`), and every logged read for its trial (the
    closure logs `(trial.seed, mass bits, semi-major-axis bits,
    ClassRead)` and the test keeps the reads whose mass and axis equal
    the placement's trial: the rejected tries of a slot share its seed)
    is `Some(obliquity)` with `obliquity.value().to_bits() ==
    placement.spin.obliquity.value().to_bits()`; for every giant
    placement every logged read is `None`. One assertion covers
    `!is_giant` and B2 (the obliquity does not depend on the radius);
  - `every_trial_seed_is_unique`: within each system, the placements'
    `trial.seed`s are distinct;
  - `the_reference_planets_are_pinned`: per system and designation,
    class, liquid kind and biosphere exactly, and the surface
    temperature within `ALBEDO_TOLERANCE_K` (S4), as a table pasted
    from the first run (a pin, the comment says so);
  - `living_planets_keep_their_stage` (Review Focus 1, S8): every
    planet whose biosphere is not `Lifeless` had liquid water in its
    first climate (the life node's parent) and has a climate; the
    number whose second climate has no liquid water is printed and
    pinned.
- [ ] **Step 2: Run** `mise run test:tier2`. Expected: PASS, under
  480 s; over it, stop and report (test-reviewer proposes, the user
  decides).
- [ ] **Step 3: Run** `mise run ci`. Commit
  `test(regression): pin the reference planets` with `Refs: #677`.

### Task 10: The boil in the cause tree, and the tier-3 numbers (sonnet)

**Files:**

- Test: `crates/system/src/planet/chain_tests.rs` (tier 3 where it
  needs a boil, reusing `recondensing_water()` of
  `climate/heavy_tests.rs:1512`), `settled_tests.rs` (Venus),
  `climate/heavy_tests.rs` (#492).

- [ ] **Step 1: Write the tests** (B12, D4):
  - `a_recondensed_boil_shows_in_the_tree` (tier 3): the F0V Earth of
    `a_boil_whose_leftover_recondenses_keeps_its_ocean` through
    `chain_with` and `model`: `water.boiled` true, `water.runaway`
    false; the frozen graph holds the `planet.climate.boils` true node
    under `boiled`, and `runaway`'s parents are the steam run's
    inventory and runaway nodes;
  - `the_steam_pass_boiled_node_is_not_kept` (tier 3, the #492 body):
    after `freeze` of the `PlanetState`, exactly one
    `planet.water.boiled` node is in the graph;
  - D4: a tier-3 `venus_prints_its_temperature` (#594) and
    `the_492_super_earth_is_a_steam_world` print, without asserting,
    Venus's surface temperature and #492's radius and surface
    temperature; the comment at `settled_tests.rs:279-282` points to
    it. The two heavy settled tests (`settled_tests.rs:881-936`) stay
    as they are.
- [ ] **Step 2: Run** in release with `--run-ignored only`. Expected:
  PASS (no code change; a failure is a bug of Task 7, fixed there).
  Commit `test(planet): show the boil in the cause tree`; the body
  gives the printed numbers.

### Task 11: Titan on its own air (sonnet) — waits on #679

Start only once `fix/titan-nitrogen-inventory` is merged into `main`:
`git fetch origin && git rebase origin/main` (or merge `main` in, per
`CLAUDE.md`), `mise run ci` green first.

**Files:**

- Modify: `crates/system/src/planet/settled_tests.rs:335-398`
  (`titan_on_its_measured_air`, `titan_has_its_seas`),
  `classify_tests.rs` (Titan's row).

- [ ] **Step 1: Write the tests.** `titan_has_its_seas` runs on the
  chain's own air (`settled_of(&TITAN, …)`, no measured-air
  substitute); its existing bounds stay; a new assertion: the climate's
  surface pressure within 10 % of 1.467 bar (B10, `Source:` Fulchignoni
  et al. 2005, the tolerance a `Definition:` shared with #679's N₂
  test; #679's fix gives about 1.473 bar). `the_solar_system_classifies_as_itself`
  adds Titan `HydrocarbonWorld`.
- [ ] **Step 2: Run** `cargo nextest run -p system titan solar`.
  Expected: PASS; a failing bound stops the task (stop rule).
- [ ] **Step 3: Run** `mise run ci`. Commit
  `test(planet): settle Titan on the air its chain makes` with
  `Refs: #679`.

### Task 12: Documentation and follow-ups (sonnet)

**Files:**

- Modify: `docs/science/planet-atmospheres.md`:
  - l.546, l.1050, l.2350, l.2647: "in 2d-4" → what was done;
  - l.2341 and l.2647: Venus's 577 K stays a tier-3 number, not a
    tier-2 pin (D4);
  - l.2964 (1.61 R⊕, 1679 K) and l.3225-3226 (1679 K, 1.700 R⊕):
    #492's values → the numbers Task 10 printed, named as the model's
    output;
  - l.3121: the `boiled_recondensed` note → the chain shows `boiled`
    and `runaway` (B12);
  - l.3222: "Titan has 2.1 bar in the model" → Task 11's value;
  - the ice-world row: the kept water fraction (N10); the ocean-world
    paragraph: `ocean_share` is `ocean_fraction`, a second law for
    where the land drowns (N13);
- `crates/system/src/planet/climate/heavy_tests.rs:940-942` (census
  182 d: the rewrite's seed 182 is not legacy's system, so the case
  stays on its census inputs), `crates/system/README.md`,
  `crates/timeline/README.md` (`Biosphere`), `docs/science/SUMMARY.md`
  only if a page is added (none planned).
- End-of-branch grep (S9): `body seed`, `body_seed`, `1679`, `1.700`,
  `stand-in`, `2d-4` over `crates/` and `docs/science/`; each hit fixed
  or kept as history on purpose.
- Issues (`issue-writer`, label `backlog` + theme, placed in the
  roadmap per `CLAUDE.md`): the draw classifies the eccentric trial
  again (K1); planet radial samples need a temperature profile (B5);
  ocean share and inventory of non-water liquids, and their effect on
  the world's sea level (B6); `ocean_fraction` against the basin
  capacity on low-gravity planets (N13).

- [ ] **Step 1:** Make the edits; run `mise run docs:check` and
  `mise run ci`. Expected: green. Commit
  `docs(planet): document the wired chain, life and the class`.

## Points for the main session

- **Before Task 1:** `generation-profiler` on the reference systems
  with the `main` binary (stand-in classifier), for the "before" of
  #593. mapgen3 agreed to B2 and B4; tell them the spin conditions are
  in Tasks 1 and 9; D2 and D5 are agreed.
- **After Task 4 or Task 7:** tell mapgen2 what `ocean_share` now means
  (the share under water, ice-covered sea included) and that
  non-water liquids carry an imposed 0 (B6).
- **After Task 7:** read the budget measurement (S3); if tier 2 is
  projected over 480 s, put it to the user before Task 8.
- **After Task 8:** `generation-profiler` again on the same inputs (the
  "after", with the climate's and the column's share and the trace
  clone of `class_of`, N11), for the PR and for mapgen3. The PR closes
  #593 with these numbers.
- **After Task 12:** the end-of-branch agents (`branch-reviewer`,
  `test-reviewer`, `science-coherence-checker`, `issue-auditor`), then
  `mise run test:heavy` (output in the PR). Then the `develop` cycle,
  warning the other sessions before the rebuild: no world exists, so
  it delivers a census of classes, climates and biospheres from the
  CLI report (`map-generator system --seed <n>`, D3) on the oracle's
  seeds, before and after this branch and against legacy's classes,
  with the timings; no 2:1 projections. Once the PR's CI has run on
  Linux, compare its digests and pins with the local ones and split
  per `target_os` if they differ (S4). PR text: `Closes #593`,
  `Closes #659`, `Closes #677`; `Refs #602`, `#594`, `#604`, `#679`,
  `#684`; a heading per calibration (`DENSE_AIR_BAR`,
  `ICE_WORLD_MIN_OCEANS`); the Wiki push command for study 27
  (`0c504a7`, `3e520da`).
