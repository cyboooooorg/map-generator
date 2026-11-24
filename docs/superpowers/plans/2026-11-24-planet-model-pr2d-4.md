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
class thresholds, the basin capacity) and their pure laws go to
`science` (`science::life`, `science::classes`). `system::planet` gets
`life.rs` (the life draw, the `Biosphere` node, the oxygenated air and
the second solve) and `classify.rs` (a pure `class` on plain values,
and `classify` that builds them from the chain and records the
`planet.class` node). `chain.rs` runs draw, ground, spin, climate, boil
loop, steam world, life, class, and serves `class_of` (scratch trace,
plain outcome) and `model` (real trace, `PlanetState`). The spin of
every planet is drawn from the trial seed, so the obliquity the climate
reads is the one `Placement.spin` holds. `timeline::Life` becomes
`Traced<Biosphere>`.

**Tech Stack:** Rust, `cargo nextest` tiers, the `Trace`/`Traced` cause
API of `timeline`.

**Spec:** `docs/superpowers/specs/2026-11-23-planet-model-pr2d-design.md`,
"2d-4: life, class and wiring" (amended in `db3b8248` with the seed,
spin, life-contract and Titan decisions), "Constraints", "Rules for the
build", "Left to the builder". Science: `docs/science/planet-atmospheres.md`
sections "Life" (l.~2974) and "The chain and the surface classes"
(l.~3084), `docs/science/planet-interiors.md` "Classes" (grep the
heading); Wiki
[27-life-and-class-audit](https://github.com/cyboooooorg/map-generator/wiki/27-life-and-class-audit).
Recon: `.superpowers/sdd/2026-11-24-planet-model-pr2d-4/recon.md` (every
`file:line` below comes from it or was checked at `a69d22ad`). Ledger:
`progress.md` there. Legacy: `legacy-final:crates/mapgen/src/planet/`
(`life.rs`, `classify.rs`, `explain.rs` l.1122-1134, `reference.rs`).

## Decisions

From the spec and the user (2026-11-24; details in the ledger):

- **Seed.** `class_of` and `model` both draw the chain from the trial
  seed (`placement.trial.seed` in `model`). Trial labels are unique per
  slot (`<id>/<k>`, `slot-<n>`, `draw/place.rs:162-165`, `:184`), so the
  trial seed is per planet. `Placement.seed` (the body seed) is no
  longer read by the planet model.
- **Spin (#677).** `draw/mod.rs:69-78` draws every planet's spin from
  `planet.trial.seed` instead of the body seed. The chain calls the same
  `spin::spin` to get the obliquity of its `Rotation`; `locked` is an
  imposed `false` until #604. One value: in `model` the climate reads
  the node `placement.spin.obliquity` itself; in `class_of` it reads the
  obliquity of a `spin::spin` call on the scratch trace, equal to it by
  construction (Task 1 proves it, B2 and B3).
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
  solved once more (`climate_of`, same rotation, same snowball draw).
  The steam envelope stays after the loss (`water.rs:237`
  `debug_assert`): life needs liquid water, so a steam world never gets
  there. Reference Earth: 286 to 290 K at 280 ppm (the tests' window,
  `PREINDUSTRIAL_RANGE`, not Myhre's).
- **Class** (study 27). Read `Settled.hides_surface`, never the steam
  line again. Gas giant / hot Jupiter = `Body::Giant`; ice giant / hot
  Neptune = `Body::MiniNeptune` with mass ≥ `MINI_NEPTUNE_MAX_MASS`
  (10 M⊕, `envelope.rs:54`); hot when the orbit's period ≤ 10 d (Wright
  et al. 2012); mini-Neptune = `Body::MiniNeptune` under 10 M⊕, no
  radius rule. Ocean world: water kept ≥ `3.7 (R/R⊕)² (g⊕/g)` Earth
  oceans (Cowan & Abbot 2014), every regime. Ice world:
  `heat::ICY_MIN_WATER` and `ICE_WORLD_MIN_OCEANS` 0.01
  (`Calibration:`). Runaway greenhouse: air over `DENSE_AIR_BAR` 1 bar
  (`Calibration:`). Hydrocarbon world: air over methane's triple-point
  pressure, the rewrite's `methane_saturation_bar(METHANE_TRIPLE_TEMPERATURE)`.
  Super-Earth from 1.25 R⊕ (Borucki et al. 2011). **Exactly two new
  calibrations**: `DENSE_AIR_BAR`, `ICE_WORLD_MIN_OCEANS`.
- **#679** is fixed in `fix/titan-nitrogen-inventory`, merged before
  this PR; Task 11 waits on it (rebase first).
- **Generation changes.** `ENGINE_VERSION` 3 → 4; golden digests and
  the `SystemId` pin regenerated; `mise run test:heavy` output and the
  profiler's before and after in the PR description.

Builder's decisions (settled by this plan, listed for the user in the
hand-off; each has a recommendation there):

- **B1. Test build speed.** The chain at opt-level 0 (workspace crates
  are built at 0 in `dev`, `Cargo.toml:49-50` raises dependencies only)
  is about 20 times slower than in release (heavy-test comments: 7.1 s
  against 0.3 s). Wiring `class_of` puts it in tier-1 tests that draw
  whole systems (`system/tests/determinism.rs`, `cli/tests/system.rs`)
  and in tier 2. Task 8 measures; over a budget, it stops and the user
  chooses (see the hand-off).
- **B2. The rocky spin is drawn in units of the critical rate.** In
  `spin.rs` the rocky branch draws the three normals `n`, keeps them
  when `ROCKY_SIGMA_OVER_CRITICAL · |n| < 1`, and takes the fraction
  `ROCKY_SIGMA_OVER_CRITICAL · |n|` and the obliquity
  `acos(n_z / |n|)`: `sigma` and `critical` cancel exactly, so the
  obliquity depends on the seed and on the law (rocky or giant) only,
  bit for bit. The period is still `2π / (fraction · critical)`. Today
  the vector is scaled by `sigma` before the ratio, which differs in the
  last bits between radii. The law is unchanged; the values change in
  the last bits (they change anyway with the seed).
- **B3. `class_of`'s spin call.** The chain calls `spin::spin` only for
  a body with a surface, after its ground is formed: with the class
  `PlanetClass::Rocky` (every class with a surface takes the rocky law,
  `spin::is_giant` false), the formed body's radius, the trial's
  archetype placement (`trial.archetype.as_ref().and_then(archetype::placement)`,
  so an asked obliquity is the same `Provenance::Input` value in both)
  and the trial seed. `model` calls nothing: it reads
  `placement.spin.obliquity`.
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
- **B6. `SurfaceLiquid`.** Water: `inventory` is the `Water.inventory`
  node (the contract's doc), `ocean_share` the open-ocean share of the
  settled cover, recorded as `planet.climate.ocean-share` (the solver's
  `Cover.ocean` at the settled state, one path). Hydrocarbons and
  nitrogen: `inventory` the condensate's column mass
  (`science::outgassing::column_mass`, the inverse of `column_pressure`,
  `Definition:` hydrostatic balance), `ocean_share` an imposed 0 with a
  follow-up issue. Lava: `inventory` and `ocean_share` imposed 0, same
  issue.
- **B7. The life node's parents.** The liquid node when there is one,
  else the climate's state node (it says why there is none), plus the
  age and the life draw. A body without a surface has no `Surface`, so
  no life node.
- **B8. The oxygenated air.** Made from the bare air (the inventory the
  climate condenses from): `planet.life.air.o2` from the N₂ and the
  biosphere, `planet.life.air.ch4` from the biosphere, the
  `planet.life.air.pressure` from the partials. The class reads the
  climate of the second solve; the life node reads the first. No second
  boil verdict on the second solve.
- **B9. Tier-2 pins of crate-private cases.** Venus 577 K (#594), #492
  (1.643 R⊕, 1526 K) and the two tier-3 settled tests need the
  crate-private chain with a fixed composition, which
  `crates/regression` cannot build. They go to a `pinned` test module
  in `system::planet`, put in tier 2 by the nextest filters
  (`.config/nextest.toml`: tier 1 `not (package(regression) |
  test(/::pinned::/))`, tier 2 `package(regression) |
  test(/::pinned::/)`).
- **B10. Titan's total pressure** bound: 1.467 bar (Fulchignoni et al.
  2005, Huygens) within the tolerance #679 sets for its N₂ (10 %), a
  `Definition:`.
- **B11. The CLI's `classifier` field** is removed (nothing is a
  stand-in any more); `schema` stays 1 (a field removed from a
  printout read by no program).
- **B12. The `boiled_recondensed` note** is not ported: the rewrite's
  chain already shows it (`planet.water.boiled` true, from the verdict;
  `planet.water.runaway` false, from the leftover's inventory and
  runaway, `water.rs:~188`), and the water lost would be a difference
  computed to explain, against "one code path". The unread `boiled`
  node of the steam world's second pass (`settled.rs:225-230`) stays:
  `Trace::freeze` keeps only reachable nodes. Task 10 tests both.
- **B13. `PlanetState` fields from the chain.** `composition`:
  `bare.drawn.core_mass_fraction` and `water_fraction` (the kept
  planet's, without envelope); `envelope_fraction`:
  `bare.drawn.envelope_fraction`; `bulk`: `bare.body.bulk()`; `heat`:
  `bare.heat`; `sources`, `age`, `orbit`: the trial's; `spin`:
  `placement.spin`; `terrain`: `Terrain { units: vec![] }` (PR 3);
  `targets`: empty (no `ArchetypeTargets` type exists yet, PR 3).

## Global Constraints

- Crates: laws and numbers in `science`; chain, records and class in
  `system::planet`; the contract change in `timeline`. Dependencies as
  `CLAUDE.md` draws them.
- `crates/system/src/draw/**`: only `draw/mod.rs` (the spin seed line
  and its doc) and the one draw test of B4. `placement.rs`: doc only.
  `archetype.rs`: untouched.
- Every number in `science` (and every new number in the engines and
  tests) has a doc comment starting with `Source:`, `Calibration:` or
  `Definition:`. Two new calibrations, no more: `DENSE_AIR_BAR`,
  `ICE_WORLD_MIN_OCEANS`; each a line under its own heading in the PR
  description.
- One code path per value: the class reads `hides_surface`, the liquid,
  the runaway node, the air's pressure node, the water inventory node;
  it computes only the basin capacity and the period test. The ocean
  share is the solver's cover.
- The domain emits codes (`system/src/planet/codes.rs`), never text.
- No draw outside the seed hierarchy; every draw of the chain derives
  from the seed it is given, never from the trace.
- Tests: tier 1 budget 2 min (about 39 s now), tier 2 8 min. A full
  bistable-Earth solve is tier 3 (user, 2d-3a). A new chain test over
  5 s at opt-level 0 goes to tier 3 with
  `#[ignore = "heavy: <why, measured time>"]`. Each task's report
  gives the time of every new test. Expected values come from sources
  or definitions; pins of the code's output only in tier 2.
- Stop rules: a sourced or observed bound that fails (Earth's 286 to
  290 K, Titan's 1.467 bar, the Solar System's classes) stops the task,
  which reports the gap; no window is widened, no input moved, no
  constant refitted on the code's output.
- Every task ends with `mise run ci` green. No per-task reviewer; the
  end-of-branch agents review once.
- Commits: Conventional Commits, the session footer (two lines,
  `Co-Authored-By` and `Claude-Session`).

## Review Focus

1. **The second solve changes the liquid.** An oxygenated air warms the
   planet (more pressure); a body near an edge may lose or gain its
   water. Expected: the biosphere stays what the first climate decided
   (its node), the class and `SurfaceLiquid` read the second climate,
   nothing panics, no second boil. Task 5,
   `the_second_solve_is_what_the_class_reads`; Task 9 checks every
   living reference planet.
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
5. **A chain error on an asked archetype.** It surfaces through
   `draw_with` as `SystemError::Planet(_)` with a code in its `Display`
   (`planet-interior-…`), and on a free slot it spends the try
   (`place.rs:355`). Task 7, `an_interior_error_is_a_system_error`.

---

### Task 1: Spin from the trial seed (sonnet)

**Files:**

- Modify: `crates/system/src/spin.rs` (rocky branch l.~96-114 per B2;
  doc l.59-78, "Drawn from `body_seed.derive("spin")`" → the trial
  seed; parameter `body_seed` → `seed`), `crates/system/src/draw/mod.rs`
  (l.44-52 doc, l.69-78: `planet.trial.seed` passed to `spin::spin`),
  `crates/system/src/placement.rs` (l.1-22 doc),
  `crates/system/src/draw/tests.rs:714-737` (B4),
  `crates/system/README.md` (the `placement` and `spin` rows, the
  "Planet interior" paragraph l.~63-65: both paths draw from the trial
  seed).

**Interfaces:**

- Produces: `spin::spin(class, &mass, radius, archetype, seed, trace)`
  unchanged in shape; for any two classes with `is_giant` false, any
  mass and radius, the same `seed` and `archetype` give the same
  `obliquity` value bit for bit.

- [ ] **Step 1: Write the failing test** in `spin.rs` tests:
  `the_rocky_obliquity_depends_only_on_the_seed`: for seeds 0..200,
  classes `Rocky`, `SuperEarth`, `OceanWorld`, `SteamWorld`,
  `MiniNeptune`, masses 0.1, 1, 5 M⊕ and radii 0.5, 1.0, 1.643, 2.5 R⊕,
  `obliquity.value()` equals (`assert_eq!` on the `f64`, no tolerance)
  that of `(Rocky, 1 M⊕, 1 R⊕)` with the same seed. The existing
  isotropy and period tests stay as they are and must still pass.
- [ ] **Step 2: Run** `cargo nextest run -p system spin`. Expected: the
  new test FAILS (last-bit differences) or passes by chance on some
  seeds; it must fail on at least one, else note it in the report and
  keep the refactor (the doc of the law requires the cancellation).
- [ ] **Step 3: Implement** B2 in `spin.rs`, then the seed line and the
  docs. `placement.rs`'s doc says: the trial's seed is
  `system_seed.derive("trial/<archetype id>/<k>")` or
  `"trial/slot-<n>"` (check the exact labels at `place.rs:162-165`,
  `:184`), unique per planet; `class_of` and `model` both draw the chain
  from it, and the spin is drawn from it too, so that the obliquity the
  climate reads is the placement's; the body seed stays for what is
  drawn after the designation (none in the planet model).
- [ ] **Step 4: Run** `cargo nextest run -p system`. Expected: PASS
  (the draw tests use `stand_in`; spins change, no test pins one but
  B4's, updated). Run `mise run ci`. Expected: green. Commit
  `feat(system): draw the spin from the trial seed` with `Refs: #677`.

### Task 2: Life and class laws in `science` (opus)

**Files:**

- Create: `crates/science/src/life.rs`, `crates/science/src/life_tests.rs`,
  `crates/science/src/classes.rs`, `crates/science/src/classes_tests.rs`.
- Modify: `crates/science/src/lib.rs`, `crates/science/src/volatiles.rs`
  (make the triple-point pressure public, below),
  `crates/science/src/surface/condense.rs:~88` (`titan_like_air` reads
  it), `crates/science/src/outgassing.rs` (`column_mass`),
  `crates/science/README.md`, the two `docs/science` sections (name the
  functions).

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
    al. 2012 § 3);
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
  - `o2_per_n2_is_the_cipm_ratio`: within 1e-15 of 0.209390 / 0.780848
    and within 1e-4 of 0.2682 (the page);
  - `oxygen_comes_in_two_steps`: on an air of N₂ 0.78, CH₄ 1e-3, O₂ 0,
    CO₂ 4e-4 bar: `Anoxic` unchanged; `Oxidising` CH₄ 0, O₂ 0, the rest
    unchanged; `Modern` CH₄ 0, O₂ `O2_PER_N2 × 0.78` exactly, in 0.15
    to 0.30 bar (legacy `earth_has_oxygenic_life`).
  `classes_tests.rs`:
  - `an_earth_holds_three_point_seven_oceans`: `basin_capacity_oceans(
    EarthRadius(1.0), STANDARD_GRAVITY)` is 3.7 within 1e-12; twice
    the radius and twice the gravity give 7.4;
  - `the_methane_triple_point_pressure`: `methane_triple_pressure()`
    within 1e-3 relative of 0.1169 bar (NIST, as legacy
    `METHANE_TRIPLE_BAR`), and `titan_like_air` reads it (an air of
    CH₄ 1e-3 bar and total a hair above it is Titan-like, a hair under
    is not);
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
  `crates/timeline/README.md` (if it lists `Life`),
  `crates/system/src/planet/codes.rs` (nothing yet).

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

### Task 4: The settled cover's ocean share (sonnet)

**Files:**

- Modify: `crates/system/src/planet/climate/body.rs:106-120`
  (`Underneath` keeps the `Cover` it built), `climate/settle.rs:45-75`,
  `:362-374` (`Settle.ocean: Fraction`), `climate/record.rs:23-40`
  (`Climate.ocean: Option<Traced<Fraction>>`, recorded when the liquid
  is water; the doc at l.36-37 says the inventory is the water's node),
  `codes.rs` (`CLIMATE_OCEAN_SHARE = "planet.climate.ocean-share"`).
- Test: `crates/system/src/planet/settled_tests.rs`.

**Interfaces:**

- Produces: `Settle.ocean` = the `ocean` of the `cover` that
  `underneath` builds at the settled temperature and air (one more
  call of `underneath` at the end of `settle`, the same function the
  albedo used); `Climate.ocean`, parents the surface temperature and
  the water inventory nodes; `None` without liquid water.

- [ ] **Step 1: Write the failing test** `a_wet_climate_records_its_open_ocean`
  on a body already settled in a tier-1 test without a bistable solve
  (pick one: the steam-free wet fixture of `wet_tests.rs`, or a
  `climate/fixture.rs` input; the implementer checks its time): the
  recorded share equals `science::surface::cover::cover(...)`'s
  `ocean` at the settled temperature and air within 1e-12, lies in
  (0, 1), and Venus (`venus_is_bright_and_has_no_liquid`) has
  `ocean: None`.
- [ ] **Step 2: Run** `cargo nextest run -p system settled`. Expected:
  FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the test, then `mise run ci`. Expected: green
  (`Settle` gains a field: the solver's tests build none by hand, check
  `settle_tests.rs`). Commit
  `feat(planet): record the open ocean of the settled climate`.

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
      snowball: f64,        // the same draw the first solve used
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
  water).

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
    whose ancestry contains `planet.life.air.o2`, and with draw 0.99
    the climate is the first one.
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
  `crates/system/src/planet/heat.rs:19` (doc: the ice world reads it).

**Interfaces:**

- Consumes: `science::classes` (Task 2),
  `science::volatiles::methane_triple_pressure`,
  `heat::ICY_MIN_WATER`, `envelope::MINI_NEPTUNE_MAX_MASS` (make it
  `pub(crate)`), `Settled`, `Body`, `science::habitable::OCEAN_MASS`.
- Produces:

  ```rust
  /// What the class reads, as plain values (legacy `classify`'s inputs).
  pub(crate) struct ClassView {
      pub kind: BodyKind,             // Rocky | MiniNeptune | Giant, from Body
      pub mass: EarthMass,            // the bulk's
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
      pub water_fraction: f64,        // bare.drawn.water_fraction
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
  the triple point, the basin capacity). `classify`
  builds the view from `settled` and records `CLASS` from the nodes the
  rules up to the winning one read (mass, period, `hides_surface`,
  liquid, runaway, the air pressure node, the water inventory, the
  bulk's radius and gravity, the water fraction, the regime).

- [ ] **Step 1: Write the failing tests** in `classify_tests.rs`:
  - `each_rule_of_the_table` (tier 1, synthetic views): one view per
    row gives its class; precedence pairs: a hot `Giant` (period
    `HOT_GIANT_MAX_PERIOD`) is `HotJupiter`, one day more `GasGiant`;
    a `MiniNeptune` of exactly 10 M⊕ is `IceGiant` and hot
    `HotNeptune`, of 9.99 M⊕ `MiniNeptune` whatever its period; a steam
    world that is also a runaway over 1 bar is `SteamWorld`; lava over
    runaway; a runaway at exactly 1 bar is not `RunawayGreenhouse`; an
    ocean over the capacity with `HeatPipe` is `OceanWorld`; a heat-pipe
    ice world stays `IceWorld`; `HeatPipe` otherwise `VolcanicWorld`;
    radius `SUPER_EARTH_MIN_RADIUS` is `SuperEarth`, a hair under
    `Rocky`; an icy body (`water_fraction` = `ICY_MIN_WATER`) with
    oceans just under `ICE_WORLD_MIN_OCEANS` is not `IceWorld` (#452);
  - `the_solar_system_classifies_as_itself` (tier 1 where each body is
    cheap; Earth goes to the tier-3 test of Task 5, asserting `Rocky`):
    Mercury `Rocky`, Venus `RunawayGreenhouse` with runaway true and
    boiled false, Mars `Rocky`, Callisto `IceWorld` with no CH₄ and no
    liquid, Jupiter (`jupiter_at(5.2)`) `GasGiant`, Neptune (the
    `NEPTUNE` fixture, made `pub(super)`) `IceGiant`. Titan waits for
    Task 11. Ganymede and Europa only if
    `legacy-final:crates/mapgen/src/planet/reference.rs` gives their
    values with a source; else listed in the report;
  - `a_close_gas_giant_is_a_hot_jupiter`: `jupiter_at(0.05)` is
    `HotJupiter`, `jupiter_at(0.2)` `GasGiant` with its period over
    10 d;
  - `a_close_ice_giant_is_a_hot_neptune`: a 17.1 M⊕, 15 % envelope
    (`NEPTUNE`) inside 10 d is `HotNeptune` if it keeps its envelope
    (assert the body kind first; pick the orbit by the period, not by
    the result), at 30 au `IceGiant`;
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
  model`), `crates/system/src/planet/settled.rs` (split
  `settled_with` l.122-169: `settled_on(formed: Formed, rotation,
  snowball, trace)` after `formed`, so the chain gets the ground
  before the spin; doc l.1-14), `crates/system/src/planet/interior.rs:100`
  (`InteriorError` becomes `pub`, `#[non_exhaustive]`),
  `crates/system/src/error.rs` (`SystemError::Planet(InteriorError)`,
  `From`, `Display` `planet-{error}`), `crates/system/src/lib.rs`
  (re-export `planet::InteriorError` if clippy asks), `codes.rs`
  (`SPIN_LOCKED = "planet.spin.locked"`, `INTERIOR_SAMPLES =
  "planet.interior.samples"`, `LIQUID_OCEAN_SHARE`,
  `LIQUID_INVENTORY` for the imposed leaves of B6).

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
      -> Result<Chained, InteriorError>;
  /// The same with the composition and the two draws given (tests).
  fn chain_with(trial: &Trial, drawn: &Drawn, spun: Spun<'_>,
      snowball: f64, life: f64, trace: &mut Trace)
      -> Result<Chained, InteriorError>;
  pub fn class_of(trial: &Trial, trace: &Trace)
      -> Result<TrialOutcome, SystemError>;
  pub fn model(placement: &Placement, trace: &mut Trace)
      -> Result<PlanetState, SystemError>;
  ```

  `chain`: `draw(trial.seed, …)`, `formed`; enveloped → no rotation,
  `classify` on the enveloped `Settled`; else the rotation (`Spun::Trial`
  → `spin::spin` per B3; `Placed` → its `obliquity` node), `locked` the
  imposed `SPIN_LOCKED` false leaf, `settled_on`, `living` with
  `life_draw(trial.seed.derive(LABELS.life))` and the snowball draw of
  `trial.seed.derive(LABELS.snowball)`, `classify`. `class_of` clones
  the trace, runs `chain(trial, Spun::Trial, &mut scratch)` and returns
  the class value and `bulk().radius`, `bulk().mean_density` values.
  `model` runs `chain(&placement.trial, Spun::Placed(&placement.spin),
  trace)` and builds `PlanetState` per B5, B6, B13; it calls
  `validate()` before returning.

- [ ] **Step 1: Write the failing tests** in `chain_tests.rs` (tier 1,
  each timed):
  - `the_same_planet_gives_the_same_answer`: three cheap trials (a dry
    hot rocky planet at `HOT_ORBIT_AU`, a cold one at 5 au, a giant of
    318 M⊕ at 5.2 au; seeds chosen once and named): `class_of(&t,
    &trace)` then a `Placement` built as `draw_with` does (`spin::spin(
    outcome.class, &t.mass, outcome.radius, archetype, t.seed, &mut
    trace)`), then `model`: `model.class`, `bulk.radius`,
    `bulk.mean_density` values equal the outcome's (`assert_eq!`, no
    tolerance), and for the two rocky trials `chain(&t, Spun::Trial,
    &mut scratch)`'s `rotation.obliquity` value equals
    `placement.spin.obliquity.value()`, while `model`'s chain reads the
    placement's node itself (same `NodeId`);
  - `a_giant_is_modelled_without_a_surface` (Review Focus 2): Jupiter
    and the Neptune fixture: `surface: None`, `validate()` Ok, layers
    the `Body::layers()` nodes (same `NodeId`s), samples empty;
  - `an_asked_obliquity_reaches_both_paths` (Review Focus 3): a trial
    with `archetype: Some(earth-like)` on a cheap composition through
    `chain_with` with `Spun::Trial` and with `Spun::Placed`: the
    climate's obliquity is the archetype's value, `Provenance::Input`,
    in both;
  - `an_interior_error_is_a_system_error` (Review Focus 5):
    `SystemError::from(InteriorError::NoEnvelope)` is
    `SystemError::Planet(_)` and its `to_string()` starts with
    `planet-interior-`;
  - `the_planet_state_holds_the_chain` on the settled Venus through
    `chain_with`: `surface.water` is the settled water, `liquid` None,
    `life` `Lifeless`, `class` `RunawayGreenhouse`, `terrain.units`
    empty, `targets` empty, the frozen graph (`trace.freeze(&mut
    state)`) is Ok;
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
  `SystemError::Planet`, the seed of both paths, the order of the
  chain), `settled.rs:12-14`, `climate/record.rs:36-37`,
  `planet/mod.rs:62-65`.
- [ ] **Step 4: Run** the tests, the tier-3 one in release, then
  `mise run ci`. Expected: green. Commit
  `feat(planet): chain the planet behind class_of and model` with
  `Refs: #659, #677`.

### Task 8: Wiring, engine version and the budget gate (sonnet)

**Files:**

- Modify: `crates/cli/src/report.rs:5`, `:26-28`, `:61`, `:89`
  (`planet::class_of`; field `classifier` removed, B11),
  `crates/cli/README.md:34-36`, `crates/cli/tests/system.rs:28`
  (asserts the field is absent), `crates/regression/src/lib.rs:16-18`,
  `:53-63` (`stand_in_model` re-export removed; `planet::class_of`),
  `crates/system/tests/determinism.rs:9`, `:67-69`,
  `crates/system/src/lib.rs:31-38` (`EngineVersion(4)`, doc line "4:
  the planet model classifies the draw (class, radius, density from the
  chain), and spins are drawn from the trial seed"),
  `crates/system/README.md` (`stand_in` row: the draw's own tests only;
  `planet` row: what is wired; `ENGINE_VERSION` l.~109),
  `crates/regression/tests/determinism.rs` (golden digests),
  `crates/regression/tests/contracts.rs` (the `SystemId` pin
  `528d67d35f485f94`).

- [ ] **Step 1: Switch the callers** and the engine version. Run
  `cargo nextest run -p regression` once to read the new digests and
  `SystemId`; paste them into the two tests (tier-2 pins of the code's
  output, allowed there; the commit body says why they change).
- [ ] **Step 2: Measure the gate.** Run `mise run test:tier1` and
  `mise run test:tier2` and note both totals and the slowest ten tests
  (`target/nextest/*/junit.xml`). Expected: tier 1 under 120 s,
  tier 2 under 480 s. **Over either budget: stop, commit nothing past
  Step 1's working tree, and report the numbers** (B1: the user
  chooses).
- [ ] **Step 3: Run** `mise run ci`. Expected: green. Commit
  `feat(system)!: classify the draw with the planet model` with a
  `BREAKING CHANGE:` footer (engine 4: every cached system and world
  is invalid) and `Refs: #602`.

### Task 9: The tier-2 net (sonnet)

**Files:**

- Modify: `crates/regression/src/lib.rs` (`REFERENCE_SEEDS` =
  `reference_requests()`'s seeds 1 to 8; `pub struct Reference {
  pub inputs: Inputs, pub drawn: Drawn, pub trace: Trace, pub planets:
  Vec<PlanetState> }`; `pub fn references() -> &'static [Reference]`,
  drawn once, `model` called on each placement with the kept trace;
  `reference_systems()` kept as the drawn systems for the digests),
  `crates/regression/tests/contracts.rs:8`.
- Create: `crates/regression/tests/planets.rs`;
  `crates/system/src/planet/pinned.rs` (`#[cfg(test)] mod pinned;`),
  `.config/nextest.toml` (B9 filters), `CLAUDE.md` tiers table
  (the `pinned` modules are tier 2).

- [ ] **Step 1: Write the tests.** `planets.rs`:
  - `every_reference_planet_is_modelled`: one `PlanetState` per
    placement, `validate()` Ok, its class equal to the value of the
    draw's `system.draw.class` node whose parents include that
    placement's `trial.mass` node;
  - `every_trial_seed_is_unique`: within each system, the placements'
    `trial.seed`s are distinct;
  - `the_reference_planets_are_pinned`: per system and designation,
    class, surface temperature (to 1e-9 relative), liquid kind and
    biosphere, as a table pasted from the first run (a pin, the
    comment says so);
  - `living_planets_keep_their_stage` (Review Focus 1): every planet
    whose biosphere is not `Lifeless` had liquid water in its first
    climate (the life node's parent) and has a climate.
  `pinned.rs` (tier 2 by the filter), each a pin of the code's output
  with its issue in the comment:
  - `venus_settles_at_577_k` (#594): settled `VENUS`, surface
    temperature 577 K within 0.5 K (the value the tier-1 comment
    `settled_tests.rs:279-282` names; read the exact one first);
  - `the_492_super_earth_pins` (#684): radius 1.643 R⊕ and surface
    1526 K within 1e-3 relative;
  - `thin_steam_over_rock_stays_a_runaway` and
    `a_steam_world_that_loses_its_steam_keeps_its_rock` moved from
    `settled_tests.rs:881-936` (their `#[ignore]` dropped).
- [ ] **Step 2: Run** `mise run test:tier2`. Expected: PASS, under
  480 s; over it, stop and report (test-reviewer proposes, the user
  decides).
- [ ] **Step 3: Run** `mise run ci`. Commit
  `test(regression): pin the reference planets` with `Refs: #594`.

### Task 10: The boil in the cause tree (sonnet)

**Files:**

- Test: `crates/system/src/planet/chain_tests.rs` (tier 3 where it
  needs a boil, reusing `recondensing_water()` of
  `climate/heavy_tests.rs:1512`).

- [ ] **Step 1: Write the tests** (B12):
  - `a_recondensed_boil_shows_in_the_tree` (tier 3): the F0V Earth of
    `a_boil_whose_leftover_recondenses_keeps_its_ocean` through
    `chain_with` and `model`: `water.boiled` true, `water.runaway`
    false; the frozen graph holds the `planet.climate.boils` true node
    under `boiled`, and `runaway`'s parents are the steam run's
    inventory and runaway nodes;
  - `the_steam_pass_boiled_node_is_not_kept` (tier 3, the #492 body):
    after `freeze` of the `PlanetState`, exactly one
    `planet.water.boiled` node is in the graph.
- [ ] **Step 2: Run** in release with `--run-ignored only`. Expected:
  PASS (no code change; a failure is a bug of Task 7, fixed there).
  Commit `test(planet): show the boil in the cause tree`.

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
  test). `the_solar_system_classifies_as_itself` adds Titan
  `HydrocarbonWorld`.
- [ ] **Step 2: Run** `cargo nextest run -p system titan solar`.
  Expected: PASS; a failing bound stops the task (stop rule).
- [ ] **Step 3: Run** `mise run ci`. Commit
  `test(planet): settle Titan on the air its chain makes` with
  `Refs: #679`.

### Task 12: Documentation and follow-ups (sonnet)

**Files:**

- Modify: `docs/science/planet-atmospheres.md` (l.546, l.1050, l.2341,
  l.2350, l.2647: "in 2d-4" → what was done; l.~3138-3140, #492's
  values 1679 K and 1.700 R⊕ → the pinned 1526 K and 1.643 R⊕; the
  class table and the life section name the functions if Task 2 did
  not), `crates/system/src/planet/climate/heavy_tests.rs:940-942`
  (census 182 d: the rewrite's seed 182 is not legacy's system, so the
  case stays on its census inputs), `crates/system/README.md`,
  `crates/timeline/README.md` (`Biosphere`), `docs/science/SUMMARY.md`
  only if a page is added (none planned).
- Issues (`issue-writer`, label `backlog` + theme, placed in the
  roadmap per `CLAUDE.md`): planet radial samples need a temperature
  profile (B5); ocean share and inventory of non-water liquids (B6).

- [ ] **Step 1:** Make the edits; run `mise run docs:check` and
  `mise run ci`. Expected: green. Commit
  `docs(planet): document the wired chain, life and the class`.

## Points for the main session

- **Before Task 1:** `generation-profiler` on the reference systems
  with the `main` binary (stand-in classifier), for the "before" of
  #593. Tell mapgen3 that Task 1 changes `spin.rs` per B2 (their zone)
  once the user accepts B2.
- **After Task 8:** `generation-profiler` again on the same inputs (the
  "after", with the climate's and the column's share), for the PR and
  for mapgen3. The PR closes #593 with these numbers.
- **After Task 12:** the end-of-branch agents (`branch-reviewer`,
  `test-reviewer`, `science-coherence-checker`, `issue-auditor`), then
  `mise run test:heavy` (output in the PR), the `develop` cycle with
  the oracle (`world-coherence-checker`) and 2:1 projections, warning
  the other sessions before the rebuild. PR text: `Closes #593`,
  `Closes #659`, `Closes #677`; `Refs #602`, `#594`, `#604`, `#679`,
  `#684`; a heading per calibration (`DENSE_AIR_BAR`,
  `ICE_WORLD_MIN_OCEANS`); the Wiki push command for study 27
  (`0c504a7`, `3e520da`).
