# Planet model PR 2d-3b: boiling and the steam envelope Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port from `legacy-final` the boil of a wet planet past the
moist limit (`boils` and its one-pass loop, `Water.boiled` derived, a
forced-steam entry to the water), the steam world line
(`hides_surface`) and Turbet et al. 2020's steam envelope with its
radius, with the values study 26 fixed, without changing what is
generated.

**Architecture:** The Turbet laws and the steam world line go to a new
`science::steam` module. `bare.rs` is split into a part that runs once
(`formed`: body, escape, heat, surface water) and a repeatable surface
pass (`surface_pass`: the planet a water entry leaves, its regime,
volcanism and core), legacy's `Ran`/`ran_after_steam`. The boil loop
lives in `settled.rs`, the only step that runs the climate; `bare` and
`bare_with` keep their chain and their cost. Escape reads a planet
through a new `Exposed` view (radius and whole mass), so that a steam
world's loss and shoreline read its inflated body in one pass.
`steam_interior` joins `system::planet::interior`.

**Tech Stack:** Rust, `cargo nextest` tiers, the `Trace`/`Traced` cause
API of `timeline`.

**Spec:** `docs/superpowers/specs/2026-11-23-planet-model-pr2d-design.md`,
"2d-3: the climate" (the 2d-3b bullets amended after study 26), "Rules
for the build", "Left to the builder". Study 26:
`docs/science/planet-atmospheres.md`, "Boiling and the steam envelope of
the rewrite (study 26)" (l.~2654-2856, grep the heading), the "Steam
worlds" bullet of `docs/science/planet-interiors.md` (l.~1439) and the
Wiki page
[26-boiling-and-steam-audit](https://github.com/cyboooooorg/map-generator/wiki/26-boiling-and-steam-audit).
Recon: `.superpowers/sdd/2026-11-24-planet-model-pr2d-3b/recon.md`.
Legacy paths are under `legacy-final:crates/mapgen/src/planet/`
(`climate.rs`, `mod.rs`, `regime.rs`, `interior.rs`, `classify.rs`).

## Decisions

From the spec and the user (2026-11-24):

- **Nothing generated changes; `ENGINE_VERSION` stays 3.** Finding:
  generation never reaches the planet chain. `system::planet` declares
  every sub-module private under `expect(dead_code, …)` outside tests
  (`planet/mod.rs` l.62-136), no code outside `system/src/planet/`
  calls `planet::` (only doc comments in `placement.rs` and
  `random.rs`), and `draw::draw_with` (`system/src/draw/mod.rs:60`)
  calls none of `bare`, `bare_with`, `water`, `settled`. Turning
  `Water.boiled` into a derived node changes no timeline. No wiring
  (2d-4).
- **`boils`** (legacy `climate.rs` l.1038; `Source:` Kasting 1988 § 3a
  and § 3d, Goldblatt et al. 2013, Barnes et al. 2013): a body that
  keeps water out of a runaway, whose thermostat does not hold it,
  boils when its settled state has no wet balance (`wet_balance` is
  `None`) for its absorbed light plus internal heat. No temperature
  condition. The vapour stays bounded by the water
  (`ClimateBody::water_column`).
- **The loop** (legacy `mod.rs` l.517-585), one pass, in `settled`:
  1. run the planet from the light's water entry (`water`);
  2. if that run boils, run it again from the forced steam
     (`water::boiled`): runaway, the energy-limited loss of the formed
     body, `boiled` true;
  3. if the steam run keeps water, condense that leftover (runaway
     false) and run a third time; keep the third run only if it does
     not boil itself, else the steam run stands.
  `Water.boiled` of the settled planet derives from the verdict node of
  the first run (`planet.climate.boils`).
- **`wet_climate` without a wet balance** keeps legacy's own fallback
  (`climate.rs` l.1212): all the water as vapour and the column's
  balance. It is no longer a placeholder: it is the state the fixed
  points pass through on a body that boils, which the loop then solves
  again from its steam, so no settled climate keeps it. Its doc says
  so.
- **`after_steam`:** 2c's `debug_assert` stays; no rescale.
- **The steam world line:** `STEAM_WORLD_MIN_BAR` is
  `science::water::CRITICAL_PRESSURE` in bar (220.64), a `Definition:`.
  `hides_surface` is `runaway && h2o >= STEAM_WORLD_MIN_BAR && T >=
  CRITICAL_POINT_TEMPERATURE` (the 647 K as a guard).
- **The steam envelope** (Turbet et al. 2020, A&A 638, A41): Eq. 2 in
  its Appendix C form, Eq. 3 with Table 2 of arXiv v3 (legacy's
  `TURBET_ALPHA`, `TURBET_BETA`), `P_transit` 0.1 Pa, the clamps of
  Eq. 3's inputs with their `Source:` tags, the thickness cap at the
  core's radius: **the PR's only `Calibration:`**. Laws in
  `science::steam`, `steam_interior` in `system::planet::interior`, only
  for a steam world.
- **One pass for the steam loss:** the steam world line is read on the
  run the loop kept. For a steam world, the envelope is computed on the
  water handed to the loss (the formed body's surface water), and the
  light's or the boil's water entry, the surface pass and the air are
  run once more with the loss and the shoreline reading that inflated
  body (a deliberate upper bound of the loss). The climate is not
  solved again.
- **The line read again after the loss** (user, 2026-11-24, on the plan
  review's SF-2; replaces the plan's first B6): the steam world line is
  read once more on the water that re-run kept (its air's H₂O, the
  climate's surface temperature). The envelope is built only if the
  kept steam still weighs at least 220.64 bar; otherwise the planet is
  a runaway without an envelope. The climate's air then holds more
  vapour than the water kept: a documented departure.
- **The climate of a layer-born steam world** (plan review SF-3; page
  and spec corrected in `8ae09b3f` and `2baca3a7`): a steam world
  formed with a water layer reads the condensed planet's gravity in its
  climate (#492: about 1.78 against 1.93 g⊕), a documented departure.
- **Departures kept, issues filed:** #684 (Aguichine et al. 2021 past
  10 % water), #685 (no moist-greenhouse loss), #686 (the wet branch
  under the boil's bistability). Earlier: #594 (Venus pin, 2d-4), #659
  (wiring, class), #679-#682.

Builder's decisions (settled by this plan, listed for the user in the
hand-off):

- **B1. `boiled` before the climate** (plan review SF-6): `water` keeps
  `planet.water.boiled` as an imposed leaf (false), as in 2c: before the
  climate nothing is decided. It is derived only where it is decided:
  `water::boiled` from the verdict, and `settled` from the verdict.
  `bare_tests.rs:104-105` keeps its assertion; its comment names
  `settled`.
- **B2. The emissivity of `boils`.** It is the settled climate's own
  (`Settle::emissivity`), not legacy's 1.0, so that the verdict is the
  climate's own wet balance at its settled state. The frost
  emissivity is below 1 only under N₂ frost
  (`science/src/surface/cover.rs:205`), where no boil happens. Written
  on the page under "The boil" as a departure from legacy's 1.0
  (Task 3).
- **B3. The core under the steam.** The core is the iron and rock
  shells; its radius is the last of them; its mass `M_core` is the
  whole mass less the steam, so that Turbet's `x = steam / M_planet`.
  For a water-layer or beyond-the-law body this is legacy's core (the
  layers below the water). For a dry body (`Water::InMantle`) the rock
  shells already hold the steam's mass (`interior.rs:272-277`): the
  steam comes out of them, each rock (mantle) shell's mass recorded
  again times `1 − steam / Σ rock shells`, so that the core's shells
  sum to `M_core` and the shells with the steam to the whole (plan
  review SF-1). The core keeps the radii of the condensed structure
  (uncompressed by its steam, the documented departure); `structure`
  and `central_pressure` stay the rock's.
- **B4. Several stars:** Eq. 3's `S` is the sum of the sources' fluxes.
- **B5. The steam layer:** material `Steam`, state `Gas` (a
  supercritical column, recorded as gas), a new
  `interior::Water::Steam { mass }` variant counted in the bulk.
- **B6. After the steam run:** replaced by the user's decision above
  (the line read again). The `Bare.heat` is the re-run's rebuilt body's,
  as `bare_air` keeps it.

## Global Constraints

- Crates: laws in `science` only. The chain and the records stay in
  `system::planet`, crate-private under the existing `expect(dead_code,
  …)` attributes. No change to `timeline`, `world`, `catalogue`,
  `adapters`, `cli`.
- `ENGINE_VERSION` stays `EngineVersion(3)`. Nothing in
  `system/src/draw/` changes.
- Every number in `science` has a doc comment starting with `Source:`,
  `Calibration:` or `Definition:` (tier-1 guardrail). Test constants
  follow the same rule, as 2d-3a's did.
- One calibration only: the steam envelope's thickness cap. It gets a
  line under its own heading in the PR description.
- One code path per value: the verdict reads the climate's settled
  values and calls `wet_balance`, never a second climate solve; the
  envelope's base pressure, gravity, temperature and thickness come
  from one call (`science::steam::envelope`) and are recorded from its
  result.
- The domain emits codes, never text: new codes go in
  `system/src/planet/codes.rs` as `&'static str`.
- Tests: tier 1 budget 2 min (now about 39 s). A test whose expected
  value is only legacy's output is dropped: `settled` is not reachable
  from `crates/regression` before 2d-4. A chain test over 5 s in the
  tier-1 profile goes to tier 3 with `#[ignore = "heavy: <why, measured
  time>"]`. Each task's report gives the time of every new test.
- Every task ends with `mise run ci` green. No per-task review; the
  end-of-branch agents review once.
- Docs: `docs/science` pages name the rewrite's functions; the task that
  adds a function updates its page section.
- Stop rules ("Rules for the build"): a sourced bound that fails (the
  0.9 AU boil, the 1500-2500 K window, a first run that does not boil
  where a test needs it) stops the task, which reports the gap. No
  window is widened and no test input is moved to make it pass.
- Commits: Conventional Commits, the footer of the session (two lines,
  `Co-Authored-By` and `Claude-Session`).

## Review Focus

1. **Several stars on a boil:** two sources of half the flux at the same
   temperature give the same verdict as one. Task 3,
   `two_half_suns_boil_as_one`.
2. **A boil that escapes all its water:** no third run, a dry runaway,
   one verdict node. Task 4, `a_boil_that_escapes_all_its_water_ends_dry`
   (tier 3) and the unit test `a_dry_steam_run_is_not_condensed` (tier 1).
3. **A steam world whose loss leaves too little steam:** under the line
   after the one-pass loss, it is a runaway without envelope, its
   second line node false; with no steam at all, `steam_interior`
   gives the rock unchanged. Task 6,
   `a_steam_world_that_loses_its_steam_keeps_its_rock`, and Task 5,
   `no_steam_leaves_the_rock_as_it_is`.
4. **The line itself:** 220.64 bar and 647.096 K exactly is a steam
   world (`>=`); a hair under either is not. Task 1,
   `the_steam_world_line_is_the_critical_point`.
5. **A water-layer world turned steam world:** no water or ice shell
   left under the steam, the shells' masses add up to the whole. Task
   5, `steam_replaces_the_condensed_water_layer`.

---

### Task 1: The steam envelope laws in `science::steam` (opus)

**Files:**

- Create: `crates/science/src/steam.rs`, `crates/science/src/steam_tests.rs`.
- Modify: `crates/science/src/lib.rs` (`pub mod steam;`),
  `crates/science/README.md` ("Giants and envelopes": one line for
  `steam`), `docs/science/planet-atmospheres.md` (study 26 sub-section,
  "The steam world line" and "The steam envelope": name the functions).

**Interfaces:**

- Produces (all `pub`, in `science::steam`):
  - `const TURBET_ALPHA: [f64; 6]` and `const TURBET_BETA: [f64; 10]`,
    the values of Table 2 of arXiv:1911.08878v3 as the study page lists
    them (`Source:`, saying v1 and v2 print a wrong α);
  - `const TRANSIT_PRESSURE: Pascal = Pascal(0.1)` (`Source:` § 2.2);
  - `const INSOLATION_RANGE: (EarthFlux, EarthFlux)` = 1 and 30
    (`Source:` § 2.2);
  - `const GRAVITY_RANGE: (MetrePerSecondSquared, MetrePerSecondSquared)`
    = 0.2 and 6 × `constants::STANDARD_GRAVITY` (`Source:` § 2.2, `g⊕`
    a `Definition:` in the comment);
  - `const STEAM_PRESSURE_RANGE: (Pascal, Pascal)` = 2.7e6 and 2.7e10
    (`Source:` § 2.1, the structures' 27 bar to 270 kbar; the comment
    says why not § 2.2's 27 kbar, per the study page);
  - `const THICKNESS_CAP: f64 = 1.0`, in core radii (`Calibration:`:
    it caps unbound envelopes and bound ones thicker than the core's
    radius, with what was searched, a link to the page section);
  - `const STEAM_WORLD_MIN_BAR: Bar` = `CRITICAL_PRESSURE` over
    `PASCALS_PER_BAR` (`Definition:`, the base fluid supercritical;
    Pierrehumbert 2023 § 1);
  - `fn base_pressure(water_share: Fraction, core_gravity:
    MetrePerSecondSquared) -> Pascal`: `x/(1−x) g²/(4πG)` (C.9, C.10);
  - `fn steam_temperature(water_share: Fraction, core_gravity:
    MetrePerSecondSquared, insolation: EarthFlux) -> Kelvin`: Eq. 3,
    `S` and `g` clamped to their ranges, the share clamped to the shares
    whose `base_pressure` at the clamped `g` lies in
    `STEAM_PRESSURE_RANGE` (legacy `steam_temperature_k`, l.252);
  - one call for the envelope (plan review BL-2), so that the system
    records each value from it without computing any again:

    ```rust
    pub struct Envelope {
        /// The steam's weight at the core, `base_pressure`.
        pub base_pressure: Pascal,
        /// The core's gravity, `G M_core / R_core²`.
        pub core_gravity: MetrePerSecondSquared,
        /// Eq. 3's temperature, `steam_temperature`.
        pub temperature: Kelvin,
        /// Eq. 2's thickness, capped.
        pub thickness: Metre,
    }
    pub fn envelope(core_mass: Kilogram, core_radius: Metre,
        water_share: Fraction, insolation: EarthFlux) -> Envelope;
    ```

    Eq. 2 as `1/z = M g / (R T ln(P_surf/P_t)) − 1/R_core`, `M` =
    `radiation::gases::H2O_MOLAR_MASS`, `R` =
    `constants::MOLAR_GAS_CONSTANT`; thickness 0 when the share is 0 or
    `P_surf <= P_t`; `THICKNESS_CAP × R_core` when `(1/z) R_core <= 1`
    (legacy `turbet_thickness`, l.303), which caps both unbound
    envelopes and bound ones past `R_core`. It calls `base_pressure` and
    `steam_temperature` once each;
  - `fn hides_surface(runaway: bool, steam: Bar, surface: Kelvin) ->
    bool`: `runaway && steam >= STEAM_WORLD_MIN_BAR && surface >=
    water::CRITICAL_POINT_TEMPERATURE`.
- The module doc cites Turbet et al. 2020 (doi, arXiv v3), the study,
  and the departure past 10 % water (#684).

- [ ] **Step 1: Write the failing tests** in `steam_tests.rs`:
  - `turbets_worked_example_is_about_1_2_earth_radii`: a core of
    0.3 M⊕ and 0.750 R⊕ (test constants, `Source:` Zeng's MgSiO₃
    mass-radius table that Turbet et al.'s footnote 2 names, 0.750 R⊕
    at 0.3 M⊕ interpolated, study 26), share 0.05, 1 S⊕:
    - `(R_core + envelope(..).thickness) / R⊕` lies in [1.14, 1.26]
      (§ 3.1, "around 1.2 R⊕"; the window is the test's ±5 %, a
      `Definition:`; the laws give 1.23);
    - `G (0.3 M⊕ / 0.95) / (R_core + z)²`, the whole planet's mass,
      lies in [1.8, 2.2] m/s² (§ 3.1, "∼2 m s⁻²", ±10 %; the laws give
      2.05, 1.95 for the core alone: the test comment says so, as the
      page does).
  - `the_base_pressure_is_the_weight_of_the_steam`: for that core and
    share, `base_pressure` equals
    `outgassing::column_pressure(m_w, g, R_core)` with `m_w = x/(1−x)
    M_core`, within 1e-12 relative (a mass balance); `envelope`'s
    `base_pressure`, `core_gravity` and `temperature` equal the
    separate laws' results exactly (one path).
  - `the_temperature_stays_on_the_fit_grid`: `steam_temperature` at
    0.5 S⊕ equals that at 1 S⊕, at 100 S⊕ that at 30; at 0.1 g⊕ that
    at 0.2 g⊕, at 10 g⊕ that at 6; at a share of 0.9 on 1 g⊕ that at
    the share whose base pressure is 2.7e10 Pa.
  - `no_steam_has_no_envelope`: share 0 gives a 0 m thickness; a share
    whose base pressure is under 0.1 Pa gives 0 m.
  - `a_thickness_past_the_core_radius_is_capped` (plan review SF-4):
    the worked example's core and share at 10 and at 30 S⊕ gives
    exactly `R_core`. Eq. 2 is bound there (`R T ln/(M g)` about 0.54
    and 0.63 `R_core`) but its `z`, about 1.18 and 1.73 `R_core`,
    exceeds the core's radius; the cap binds from about 5.7 S⊕
    (`docs/science/planet-atmospheres.md`, corrected in `8ae09b3f`).
  - `the_envelope_thickens_with_the_water`: a 5 M⊕ core of 1.6 R⊕
    (a test input, `Definition:`) at 1 S⊕, shares 1e-5, 1e-4, 1e-3,
    0.01, 0.05, 0.1, 0.2: the thickness never falls, and grows while
    under the cap (legacy `interior.rs` :738).
  - `the_steam_world_line_is_the_critical_point`:
    `STEAM_WORLD_MIN_BAR` is 220.64 bar within 1e-12;
    `hides_surface(true, STEAM_WORLD_MIN_BAR,
    CRITICAL_POINT_TEMPERATURE)` is true (the constants themselves,
    not literals); 0.01 bar under the line, or 0.01 K under the
    temperature, is false; `(false, 1e5 bar, 2000 K)` is false.

- [ ] **Step 2: Run** `cargo nextest run -p science steam`. Expected:
  FAIL (no module).

- [ ] **Step 3: Implement** `steam.rs` with the interfaces above. Port
  the polynomial from legacy l.252-290 term by term; take the
  constants from the study page, not from memory.

- [ ] **Step 4: Run** the same command. Expected: PASS. Then update the
  docs: in `planet-atmospheres.md`'s study 26 sub-section, name
  `science::steam::{hides_surface, STEAM_WORLD_MIN_BAR}` under "The
  steam world line" and `science::steam::{base_pressure,
  steam_temperature, envelope, THICKNESS_CAP}` under "The steam
  envelope"; add the module to `crates/science/README.md`. Run
  `mise run ci`. Expected: green.

- [ ] **Step 5: Commit** `feat(science): add Turbet et al. 2020's steam
  envelope and the steam world line`.

### Task 2: A repeatable surface pass, and escape through `Exposed` (sonnet)

**Files:**

- Modify: `crates/system/src/planet/enveloped.rs` (`Exposed`,
  `Body::exposed`; the `whole_mass` doc l.86-91),
  `crates/system/src/planet/water.rs` (signature of `water`, new
  `boiled`, new `recondensed`),
  `crates/system/src/planet/air.rs` (`air` and `shoreline` read an
  `&Exposed`; doc l.92-96), `crates/system/src/planet/bare.rs` (split),
  `crates/system/src/planet/codes.rs` (`WATER_BOILED` doc),
  `crates/system/src/planet/tests.rs` (`derive_surface`),
  `bare_tests.rs` (the comment of l.104-105 only), `water_tests.rs`,
  `air_tests.rs` (call sites).
- Docs: `docs/science/planet-atmospheres.md` l.~643-645 (the "only
  radius until PR 2d" note) and l.~247-249 (`after_steam` in the
  rewrite: add the forced entry).

**Interfaces:**

- Produces in `enveloped.rs`:

  ```rust
  /// What escape reads of a planet: the radius it loses its air from
  /// and the whole mass that holds it.
  #[derive(Clone, Debug, PartialEq)]
  pub(crate) struct Exposed {
      pub radius: Traced<EarthRadius>,
      pub mass: Kilogram,
      /// The nodes `mass` derives from.
      pub mass_from: Vec<NodeId>,
  }
  impl Body {
      /// The bulk's radius and [`Body::whole_mass`].
      pub(crate) fn exposed(&self) -> Exposed;
  }
  ```

- Produces in `water.rs`:
  - `water(trial: &Trial, exposed: &Exposed, surface: Traced<Kilogram>,
    trace) -> Result<Water, InteriorError>`: as now, reading
    `exposed`; `boiled` stays the imposed leaf (decision B1). The loss
    moves to a private
    `steam_lost(trial, exposed, surface, runaway, trace) ->
    Result<Traced<Kilogram>, _>` (the `WATER_STEAM_LOST` and
    `WATER_INVENTORY` records), shared with `boiled`.
  - `boiled(trial: &Trial, exposed: &Exposed, surface:
    Traced<Kilogram>, verdict: Traced<bool>, trace) -> Result<Water,
    InteriorError>`: the forced steam (legacy `regime::steam`, l.111):
    `runaway` = `record(WATER_RUNAWAY, true, &[verdict])`, the loss of
    `steam_lost`, `boiled` = `record(WATER_BOILED, true, &[verdict])`.
  - `recondensed(left: &Water, trace) -> Result<Water, InteriorError>`:
    `runaway` = `record(WATER_RUNAWAY, false, &[left.inventory.node(),
    left.runaway.node()])`, `inventory` and `boiled` of `left`.
- Produces in `air.rs`: `air(trial, drawn, body, exposed: &Exposed,
  surface, water, regime, core, hold_back, trace)`; `shoreline` reads
  `exposed` instead of `body.bulk().radius` and `body.whole_mass()`.
- Produces in `bare.rs`:

  ```rust
  /// What runs once for a planet with a surface: steps 1 to 3 of
  /// `bare_with` up to its surface water.
  pub(super) struct Formed {
      pub trial: Trial, pub drawn: Drawn, pub body: Body, pub heat: Heat,
      pub envelope_lost: Option<Traced<Kilogram>>,
      pub surface: Traced<Kilogram>,
  }
  pub(super) enum Ground { Enveloped(Box<Bare>), Formed(Box<Formed>) }
  pub(super) fn formed(trial, drawn, trace) -> Result<Ground, InteriorError>;
  /// From `water`: the planet it leaves (`after_steam`, its body and
  /// heat built again), regime, volcanism, core. Escape (the shoreline
  /// in `bare_air`) reads `exposed`, or the body the pass built.
  pub(super) fn surface_pass(formed: &Formed, water: Water,
      exposed: Option<&Exposed>, trace) -> Result<Surfaced, InteriorError>;
  ```

  `Surfaced` gains `exposed: Exposed`, which `bare_air` hands to `air`.
  `bare_ground` is removed: `bare_with` is `formed`, then
  `water(&f.trial, &f.body.exposed(), f.surface, trace)`, then
  `surface_pass(&f, w, None, trace)`, then `bare_air(&g, true, trace)`.
  The nodes come in the same order and with the same values as
  before. `settled_with` switches to `formed`, `water` and
  `surface_pass` the same way, its behaviour unchanged. `bare::rocky`
  is renamed `rock_of` (it collides with `interior::rocky`) and becomes
  `pub(super)` for Task 6.

- [ ] **Step 1: Write the failing tests:**
  - In `bare_tests.rs`, l.104-105 keep their meaning: the comment says
    `water.boiled` stays an imposed leaf until `settled` derives it.
  - In `water_tests.rs`, on `WET_VENUS` with its XUV fluence lowered by
    `tests::with_fluence` (`tests.rs:1406`; a `Definition:` test input)
    so that `9 H` at 1.2 times its radius stays under its surface water
    (plan review BL-1: at its own fluence it loses its whole ocean and
    both losses are capped). Each test asserts that precondition first:
    - `a_boil_forces_the_steam`: a verdict leaf true. `boiled(...)` is
      runaway and boiled, both nodes derived from the verdict; its
      `planet.water.steam-lost` is positive, under the surface water,
      and equals that of `water(...)` on the same body (which runs away
      by its light: the same loss, one code path);
    - `a_larger_radius_loses_more_steam`: `water` with the body's
      `exposed()` and with the same `Exposed` at 1.2 times the radius
      (a new leaf): the steam lost grows as `R³` (Owen & Wu 2017
      eq. 19, `energy_limited_loss ∝ R³/M`), the ratio 1.728 within
      1e-9;
    - `a_leftover_condenses_back`: `recondensed` of a boiled water
      keeps its inventory and `boiled`; its runaway is false and
      derived from the inventory.
  - In `air_tests.rs`, `the_shoreline_reads_the_exposed_body`: the
    reference runaway's `air` with its `exposed()` and with a larger
    `Exposed` radius: the shoreline node derives from the given radius
    node, and the share kept is no larger.

- [ ] **Step 2: Run** `cargo nextest run -p system planet`. Expected:
  FAIL to compile.

- [ ] **Step 3: Implement** the interfaces. Update the docs named above.
  In `codes.rs`, `WATER_BOILED` reads "Whether the climate boiled the
  ocean: false, imposed, before the climate (`water`); derived from the
  boil loop's verdict once settled (`water::boiled`, `settled`)". In
  `water.rs`, the doc of `water` says the same instead of "the next
  pull request's" (l.~80).

- [ ] **Step 4: Run** `cargo nextest run -p system planet`. Expected:
  PASS, every 2c and 2d test unchanged. Then `mise run ci`. Expected:
  green.

- [ ] **Step 5: Commit** `refactor(planet): run the surface pass again
  from any water entry`.

### Task 3: The verdict: `boils` and its node (sonnet)

**Files:**

- Modify: `crates/system/src/planet/climate/wet.rs` (`boils`;
  `wet_climate` doc), `climate/wet_tests.rs`, `climate/record.rs`
  (`record_boils`, `record_hides_surface`), `codes.rs`
  (`CLIMATE_BOILS`, `CLIMATE_HIDES_SURFACE`).
- Docs: `planet-atmospheres.md`, study 26 "The boil" (name
  `climate::wet::boils`; add decision B2 as a departure from legacy's
  emissivity 1.0); l.~1586 ("a placeholder the chain replaces
  when it boils the ocean") rewritten as the `wet_climate` decision
  above.

**Interfaces:**

- Consumes: `wet_balance(&mut ClimateBody, starlight, &air, emissivity)`,
  `ClimateBody::{new, wet, starlight, water_column}`, `Numerics::DEFAULT`,
  `science::steam::hides_surface` (Task 1).
- Produces in `wet.rs`:

  ```rust
  /// Whether a body that keeps water out of a runaway boils it
  /// (legacy `climate::boils`): its thermostat does not hold it
  /// (`regulated` false) and, under the light its `bond_albedo` lets
  /// in plus its internal heat, with `air` (the air its climate left)
  /// and its surface's `emissivity`, no wet balance exists.
  pub(crate) fn boils(inputs: &ClimateInputs, air: &PartialPressures,
      bond_albedo: Fraction, emissivity: Fraction, regulated: bool) -> bool;
  ```

  The doc carries the `Source:` sentences of the study page ("The
  boil": Kasting 1988 § 3a and § 3d, Goldblatt et al. 2013, Barnes et
  al. 2013) and "no temperature condition".
- Produces in `record.rs`:
  - `record_boils(climate: &Climate, bare: &Bare, boils: bool, trace)
    -> Result<Traced<bool>, InteriorError>`: code `CLIMATE_BOILS`
    (`"planet.climate.boils"`), from the climate's `regulated`, Bond
    albedo and emissivity, each partial of its atmosphere, the water's
    runaway and inventory, the three heats, the bulk's radius and
    gravity, each source's flux and effective temperature;
  - `record_hides_surface(runaway: Traced<bool>, h2o:
    Option<Traced<Bar>>, temperature: Traced<Kelvin>, trace) ->
    Result<Traced<bool>, InteriorError>`: code `CLIMATE_HIDES_SURFACE`
    (`"planet.climate.hides-surface"`), value `science::steam::
    hides_surface(runaway, h2o, temperature)`, from those nodes (no H₂O
    partial: 0 bar, from the other two). It takes nodes, not a
    `Climate`, so that Task 6 reads the line a second time on the
    re-run's air.

- [ ] **Step 1: Write the failing tests** in `wet_tests.rs`, on
  `fixture::earth()` with `air` = `balance::EARTH_DRY_AIR` as it is
  (0.78 bar N₂, 0.21 bar O₂, 4e-4 bar CO₂, no H₂O: the measured air of
  2d-3a's tests; legacy's test had no O₂), `water` =
  `habitable::OCEAN_MASS`, the Bond albedo 0.306 (NASA fact sheet,
  `Source:`), emissivity 1, not regulated. The 0.9 AU case has a thin
  margin (about 291 W/m² absorbed against a plateau of about
  282-284 W/m², some 3 %): if `earth_boils_at_0_9_au` or
  `earth_keeps_its_ocean_at_one_au` fails, stop and report the
  absorbed flux and the plateau; do not move the distance, the albedo
  or the air.
  - `earth_keeps_its_ocean_at_one_au`: `boils` is false (Earth keeps
    its oceans, an observation);
  - `earth_boils_at_0_9_au`: on `fixture::moved(.., 0.9)` it is true
    (1.23 S⊕ is past Kopparapu et al. 2014's 1.107 S⊕; assert that
    precondition with `habitable::runaway_flux`);
  - `a_runaway_does_not_boil_again`: the 0.9 AU inputs with `runaway`
    true: false;
  - `a_regulated_climate_does_not_boil`: the 0.9 AU inputs, regulated:
    false;
  - `a_trace_of_water_does_not_boil`: the 0.9 AU inputs with the water
    `m = 1e-4 bar × 4π R² / g` on the fixture's gravity and radius (the
    inverse of `column_pressure`; `water_column` asserted 1e-4 bar
    within 1e-9): false (Kasting 1988 § 3a; Boukrouche et al. 2021
    Fig. 9);
  - `two_half_suns_boil_as_one`: the 0.9 AU inputs with their source
    split in two of half the flux, same temperature: true; at 1 AU:
    false. It holds by construction (`ClimateBody::starlight` sums the
    fluxes and the albedo is an input); the test comment says it guards
    that sum.
  In `settled_tests.rs` (where the record tests live),
  `the_steam_world_line_records_its_causes`: on leaves (runaway true,
  300 bar, 700 K) the node is true and derives from the three; without
  an H₂O node it is false and derives from the other two.

- [ ] **Step 2: Run** `cargo nextest run -p system climate::wet
  the_steam_world_line_records_its_causes`. Expected: FAIL.

- [ ] **Step 3: Implement** `boils`: on a fresh
  `ClimateBody::new(inputs, Numerics::DEFAULT)`, `!regulated &&
  body.wet()`, then `wet_balance` under `body.starlight(bond_albedo)`
  is `None`. Then the two records and
  codes, and the `wet_climate` doc (decision above, citing legacy
  l.1212).

- [ ] **Step 4: Run** the same command and time the new tests. Expected:
  PASS, each under 5 s. Then `mise run ci`. Expected: green.

- [ ] **Step 5: Commit** `feat(planet): add the boiling verdict of the
  climate`.

### Task 4: The boil loop in `settled` (opus)

**Files:**

- Modify: `crates/system/src/planet/settled.rs`, `settled_tests.rs`,
  `crates/system/src/planet/tests.rs` (move `with_star` from
  `climate/heavy_tests.rs` l.370 here as `pub(super)`, and add
  `pub(super) fn water_world_trial(trace, mass, core, water, au) ->
  (Trial, Drawn)`, `reference_trial`'s composition with `migrated:
  Some(true)` inside the ice line), `climate/heavy_tests.rs` (use the
  moved helper; the new tier-3 tests), `planet/mod.rs` (module doc:
  what remains), `crates/system/README.md` (the `planet` row, l.20:
  one clause for the boil loop).
- Docs: `planet-atmospheres.md` study 26 "After the boil": the loop in
  the rewrite's words (`settled_with`'s runs, `water::boiled`,
  `water::recondensed`, the verdict node).

**Interfaces:**

- Consumes: Task 2's `formed`, `surface_pass`, `bare_air`, `water`,
  `boiled`, `recondensed`; Task 3's `boils`, `record_boils`.
- Produces in `settled.rs`:

  ```rust
  /// One run of the planet from a water entry to its climate: the
  /// surface pass, the air, the climate and its reducing-air fallback
  /// (legacy `Ran::bare`).
  #[derive(Clone, Debug, PartialEq)]
  struct Run {
      bare: Bare,
      climate: Climate,
      inputs: ClimateInputs,
      settle: Settle,
      /// Whether its air held its CO2 back (the fallback's choice).
      hold_back: bool,
  }
  fn run(formed: &Formed, water: Water, rotation: &Rotation, draw: f64,
      trace) -> Result<Run, InteriorError>;
  /// The run's verdict: [`boils`] of its settled state, recorded.
  fn verdict(run: &Run, trace) -> Result<Traced<bool>, InteriorError>;
  /// What follows a boil: `steam` stands, or its leftover condensed
  /// again when that run does not boil (the `else` branch below).
  fn after_boil(formed: &Formed, steam: Run, rotation: &Rotation,
      draw: f64, trace) -> Result<Run, InteriorError>;
  ```

  `climate_of` returns the `ClimateInputs` and the `Settle` with the
  climate (a small struct or a tuple). `Settled` keeps `bare` and
  `climate` (the kept run's), and gains `hides_surface:
  Option<Traced<bool>>` for Task 6 (`None` here until Task 6 fills
  it).

- The loop in `settled_with` (decisions above):

  ```text
  f = formed(..)                      // Enveloped: return as now
  first = run(f, water(f.trial, f.body.exposed(), f.surface))
  v = verdict(first)
  if !v: kept = first, its water.boiled = record(WATER_BOILED, false, [v])
  else:
    steam = run(f, boiled(f.trial, f.body.exposed(), f.surface, v))
    kept = after_boil(f, steam):
      left = steam.bare.surface.water
      if left.inventory == 0: steam
      else: wet = run(f, recondensed(left))
            if !verdict(wet) { wet } else { steam }
  ```

- [ ] **Step 1: Write the failing tests.** Tier 1, in
  `settled_tests.rs`:
  - `a_runaway_by_light_does_not_boil`: `settled` of `WET_VENUS`: one
    `planet.climate.boils` node, false; the settled `water.boiled` is
    false, derived from it; the water is the light's runaway.
  - `a_dry_body_does_not_boil`: `MERCURY`: the verdict is false and
    there is one run (one `planet.climate.boils` node).
  - `a_dry_steam_run_is_not_condensed` (Review Focus 2): `after_boil`
    of a `WET_VENUS` run whose water inventory is replaced by a 0 kg
    leaf returns that run unchanged and records no
    `planet.climate.boils` node.
  - The existing Titan, Venus and Mercury tests still pass: the
    verdict runs on them.
  Tier 3, in `climate/heavy_tests.rs`, each `#[ignore = "heavy: a full
  boil loop, <measured> s"]`:
  - `a_boil_that_escapes_all_its_water_ends_dry` (legacy `classify.rs`
    :568): Earth's composition with 3e-4 of water round a 7220 K star
    (`with_star`) at a flux `F0V_BOILING_FLUX` under
    `runaway_flux(7220 K)` (assert it; a `Definition:` test input
    chosen where the first run boils). Assert: the first verdict is
    true; the settled water is runaway and boiled, `boiled` derived
    from the first verdict; no liquid water; one `planet.climate.boils`
    node when the steam run kept no water. Not asserted: "kept 0" and
    the class (model output; class in 2d-4). If no flux under the
    limit boils in the rewrite, the task reports it and stops.
  - `a_boil_whose_leftover_recondenses_keeps_its_ocean` (legacy :652,
    :664; plan review SF-7): the same star and flux, Earth's
    composition with its N₂ cut to a tenth (`Drawn::fixed`'s scatter
    `[0, -1, 0, 0, 0]` dex, order CO₂, N₂, S, CH₄, NH₃), as legacy did
    so that the F star's Rayleigh scattering does not keep the first
    run from boiling (#522). The water is sized on the rewrite's
    entries: the leftover `recondensed` receives (after the steam
    run's energy-limited loss and its shoreline) weighs 0.03 bar. So
    the surface water is `9 H + L / k`, `L` the mass of 0.03 bar on the
    formed body, `H` its `energy_limited_loss`, `k` the
    `planet.air.shoreline-kept` share of a first steam run, and the
    bulk water fraction that surface water over
    `SURFACE_WATER_SHARE × M`; three rounds, as legacy's
    `recondensed()`, since the body barely moves. Assert: the first
    verdict is true (if not, stop and report); the settled water is
    boiled and not runaway, `0 < kept < 1e-3` of the water formed, a
    water liquid, two verdict nodes (true then false), and the runaway
    node derived from the leftover's inventory.
  - `a_cool_small_planet_does_not_boil` (legacy :523): 0.2 M⊕ at
    1.5 AU, 1e-3 of water: not runaway, not boiled, the inventory is
    the surface water.
  - `a_trace_of_water_does_not_boil_a_temperate_planet` (legacy :504):
    0.2 M⊕ at 1.2 AU, 1e-9 of water: not runaway, below 400 K.
  A tier-3 test that runs under 5 s in tier 1 moves to tier 1; say so.

- [ ] **Step 2: Run** `cargo nextest run -p system settled` and
  `cargo nextest run -p system --run-ignored only boil`. Expected:
  FAIL.

- [ ] **Step 3: Implement** `Run`, `run`, `verdict` and the loop.
  The reducing-air fallback stays inside `run`, as now. The settled
  value of `boiled` replaces the run's (`bare.surface.water.boiled`).
  Rewrite the `settled.rs` header (no "Not in it: `boils`"), the
  `planet/mod.rs` doc ("What remains") and the README row.

- [ ] **Step 4: Run** both commands. Expected: PASS. Record each new
  test's time. Then `mise run ci`. Expected: green.

- [ ] **Step 5: Commit** `feat(planet): boil a wet planet past the
  moist limit`.

### Task 5: `steam_interior` (sonnet)

**Files:**

- Modify: `crates/system/src/planet/interior.rs` (`Water::Steam`,
  `steam_interior`; header l.11-14), `enveloped.rs` (`whole_mass`
  match), `water.rs` (`surface_water` match), `codes.rs`
  (`STEAM_TEMPERATURE`, `STEAM_THICKNESS`, `RADIUS_STEAM`,
  `MASS_WITH_STEAM`), `tests.rs` or a new `interior_tests.rs` if the
  interior tests live elsewhere (follow the module's pattern).
- Docs: `docs/science/planet-interiors.md`, the "Steam worlds are left
  to the next pull request" bullet: retitle it (steam worlds) and name
  `interior::steam_interior` and decision B3; l.~1250-1260 (water
  beyond the law, `steam_interior` as #659's): now this PR's.

**Interfaces:**

- Consumes: Task 1's `envelope` (one call; every node below is
  recorded from its fields), Task 4's `water_world_trial`.
- Produces in `interior.rs`:

  ```rust
  pub(crate) enum Water { InMantle, Layer{..}, BeyondLaw{..},
      /// The steam envelope of a steam world, a layer counted in the bulk.
      Steam { mass: Traced<Kilogram> } }

  /// The interior of a steam world (legacy `interior::steam_interior`,
  /// l.344): the iron and rock shells of `rock` under a steam envelope
  /// of `steam` (decision B3: a dry body's rock shells give up the
  /// steam's mass), its thickness from `science::steam::envelope`
  /// under the sum of `trial`'s fluxes. `rock` unchanged when `steam`
  /// is 0.
  pub(crate) fn steam_interior(rock: &Interior, steam: Traced<Kilogram>,
      trial: &Trial, hides: Traced<bool>, trace: &mut Trace)
      -> Result<Interior, InteriorError>;
  ```

  Records: `planet.mass.with-steam` (the whole mass, from the rock's
  whole-mass nodes), `planet.steam.temperature`,
  `planet.steam.thickness` (both from the core's shells, the steam,
  the fluxes, with the values of the one `envelope` call),
  `planet.radius.steam` (`R_core + z`), `SURFACE_GRAVITY` and
  `MEAN_DENSITY` of the whole mass at that radius, for a dry body each
  rock shell's mass again (`LAYER_MASS`, from the shell and the steam),
  and one `ShellRecord` (`LAYER_*` codes: outer radius, mass, material
  `Steam`, state `Gas` from `hides`). `water` is `Water::Steam`.

- [ ] **Step 1: Write the failing tests** (tier 1), on the #492 body
  from `water_world_trial`: 4.78 M⊕, core fraction 1.06/4.22 of the
  refractory mass, water 0.56/4.78, at 1 AU of the Sun (a water layer
  under 355 K; test inputs, `Definition:` from issue #492's
  composition), `interior(..)` for the rock, the whole layer as steam,
  `hides` a true leaf:
  - `steam_replaces_the_condensed_water_layer` (legacy :762, Review
    Focus 5): shell materials are Iron or Rock then one last Steam, no
    Water or Ice; the shells' masses sum to 4.78 M⊕ within 1e-9
    relative; the bulk mass is the whole.
  - `a_steam_world_is_larger_than_its_condensed_twin`: the steam
    radius exceeds the rock's (Turbet et al. 2020 § 3.1, a property);
    gravity is `G M / R²` and density `M / (4/3 π R³)` within 1e-12.
  - `the_steam_envelope_records_its_causes`: the radius node derives
    from the thickness, which derives from the steam's mass node and
    each source's flux.
  - `no_steam_leaves_the_rock_as_it_is` (Review Focus 3): a 0 kg steam
    leaf gives back `rock` (`assert_eq!`).
  - `a_dry_planets_steam_comes_out_of_its_mantle` (plan review SF-1):
    the reference `EARTH`'s rock (`Water::InMantle`) under the steam of
    its whole surface water (`surface_water`, a boiled Earth that kept
    its ocean): the shells are Iron or Rock then one Steam; the core
    shells sum to the whole less the steam, and all shells to the
    whole, within 1e-9 relative; the radius exceeds the rock's.

- [ ] **Step 2: Run** `cargo nextest run -p system steam`. Expected:
  FAIL.

- [ ] **Step 3: Implement.** Extend the two exhaustive matches
  (`Water::Steam` counts in the bulk; `surface_water` returns its
  mass). Update the `interior.rs` header and the interiors page.

- [ ] **Step 4: Run** the same command. Expected: PASS. Then
  `mise run ci`. Expected: green.

- [ ] **Step 5: Commit** `feat(planet): add the steam envelope of a
  steam world`.

### Task 6: The steam world in `settled` (opus)

**Files:**

- Modify: `crates/system/src/planet/settled.rs`, `settled_tests.rs`,
  `climate/heavy_tests.rs`, `enveloped.rs` and `air.rs` docs (the
  radius escape reads), `water.rs` doc of `water` (l.~72-78),
  `crates/system/README.md` (the `planet` row: the steam envelope).
- Docs: `planet-atmospheres.md`, study 26 "The steam loss on the
  inflated body" and "The climate under the envelope" (already
  corrected for the line read again and the layer-born departure in
  `8ae09b3f`): name the rewrite's steps; the class table's line on
  `steam-world` stays (2d-4 writes the class).

**Interfaces:**

- Consumes: Task 3's `record_hides_surface`, Task 4's `Run`/`run`,
  Task 5's `steam_interior`, Task 2's `Exposed`, `water`, `boiled`.
- Produces in `settled_with`, after the loop (`kept` its run, `entry`
  the entry that run came from: the light, or the boil with its
  verdict; a recondensed run is never a runaway, so never a steam
  world):

  ```text
  hides = record_hides_surface(kept water, kept climate's H2O, its T)
  if !hides: Settled { bare: kept.bare, climate, hides_surface: Some(hides) }
  inflated = steam_interior(rock_of(f.body), f.surface, f.trial, hides)
  w = entry again with Body::Rocky(inflated).exposed()
  g = surface_pass(f, w, Some(&inflated exposed))
  bare = bare_air(g, kept.hold_back)                 // no climate
  bare.surface.water.boiled = kept's boiled           // the verdict's node
  still = record_hides_surface(bare water, bare air's H2O, climate's T)
  if still:
    bare.body = Rocky(steam_interior(rock_of(bare.body),
                      bare water inventory, bare.trial, still))
  Settled { bare, climate: kept.climate, hides_surface: Some(still) }
  ```

- [ ] **Step 1: Write the failing tests.** Tier 1, in
  `settled_tests.rs`:
  - `thin_steam_over_rock_stays_a_runaway` (legacy :1037; plan review
    SF-5): a body that keeps thin steam, `WET_VENUS` with its XUV
    fluence lowered by `with_fluence` (a `Definition:` test input) so
    that its bare air keeps `0 < H₂O < 220.64` bar, asserted first. It
    settles with `hides_surface` false, no `Steam` shell, its body the
    rock of its run.
  - `a_steam_world_that_loses_its_steam_keeps_its_rock` (Review Focus
    3, the user's decision on SF-2): drive the steps after the loop
    (a private `fn steam_world(f, kept: Run, entry, trace)` split out
    as Task 4's `after_boil` is) on a kept run whose climate is past
    the line but whose re-run keeps steam under 220.64 bar (the
    `WET_VENUS` run of the test above, its climate's H₂O node and
    temperature node replaced by leaves of 300 bar and 700 K). The
    second `planet.climate.hides-surface` node is false,
    `hides_surface` is that node, and no `Steam` shell is built.
  Tier 3, in `climate/heavy_tests.rs` (`#[ignore = "heavy: …"]` with
  the measured time):
  - `the_492_super_earth_is_a_steam_world` (legacy :921): the #492
    composition of Task 5 round a 2400 K star (`with_star`) at 1 S⊕,
    over `runaway_flux(2400 K)` (assert it). Assert: `hides_surface`
    true; the climate's H₂O over 1e5 bar; its surface over 1000 K
    (Turbet et al. 2020 § 3.1); the last shell `Steam`; the radius over
    the kept run's condensed radius; the final `planet.water.steam-lost`
    derives from a `planet.radius.steam` node (the loss read the
    inflated body); the settled `water.boiled` derives from a
    `planet.climate.boils` node (plan review BL-3); both
    `planet.climate.hides-surface` nodes are true. Not asserted (model
    output, no tier-2 path): 1.61 R⊕, 1679 K, the condensed
    1.6-1.7 R⊕, Teq 258-280 K.
  - `a_thick_steam_runaway_settles_past_the_end_of_the_plateau`
    (legacy `climate.rs` :3978, #438): 1 M⊕, core 0.325, 30 % water
    (`water_world_trial`, migrated), 0.7 AU of the Sun. Assert:
    runaway; H₂O over 1e5 bar; the surface in [1500, 2500] K (Kasting
    1988, Goldblatt et al. 2013, Boukrouche et al. 2021 § 4.3, the
    study's window); no liquid; a steam world. If the temperature falls
    outside the window, stop and report it (a sourced bound): do not
    widen it.

- [ ] **Step 2: Run** `cargo nextest run -p system settled` and
  `cargo nextest run -p system --run-ignored only steam`. Expected:
  FAIL.

- [ ] **Step 3: Implement** the steam world steps. Keep `entry` as a
  small private enum (`Light`, `Boiled(Traced<bool>)`) so the second
  call reuses the first verdict. Document on `Settled` that a steam
  world's climate is the kept run's, whose air holds more vapour than
  the water kept (the user's decision on SF-2). Update the docs named above: the
  enveloped `whole_mass` and the air's shoreline now say a steam
  world's escape reads its envelope.

- [ ] **Step 4: Run** both commands. Expected: PASS; record the times.
  Then `mise run ci`. Expected: green.

- [ ] **Step 5: Commit** `feat(planet): inflate a steam world and let
  its escape read the envelope`.

### Task 7: CI, heavy tests and timings (sonnet)

**Files:** none, unless a stale note is found.

- [ ] **Step 1:** `grep -rn "2d-3b\|next pull request\|placeholder"
  crates/system/src/planet crates/science/src/steam.rs` and the study
  26 sub-section of `planet-atmospheres.md`: no note still says the
  boil or the steam envelope is to come. Fix any in a `docs:` commit.
- [ ] **Step 2:** `mise run ci`. Expected: green. Report the tier-1
  time (`cargo nextest run --profile tier1 --workspace` summary) against
  2d-3a's 39.2 s; expected about +5 to +10 s (the `boils` tests on
  Earth's inputs about 2-4 s, the steam interiors about 1 s, the two
  `WET_VENUS`/`MERCURY` runs a few seconds), under the 2 min budget.
- [ ] **Step 3:** `mise run test:heavy -- boil steam` (the new tier-3
  tests). Expected: all pass; report each time.
- [ ] **Step 4:** Report: the tier-1 time and the slowest new tests,
  the heavy output, the one calibration for the PR text
  (`science::steam::THICKNESS_CAP`), and the issues the PR refs
  (#684, #685, #686, #659, #594).

### Tests not ported

- `classify.rs` :789 `a_steam_runaway_weighs_only_the_water_it_kept`:
  2c's `after_steam` tests cover it (`water_tests.rs`).
- `classify.rs` :961 `a_runaways_albedo_tree_shows_its_steam` and the
  explanation half of :664: explanations are not in the domain
  (adapters, #659); the albedo's causes are 2d-3a's records.
- `classify.rs` :1060 `a_hot_icy_moon_is_no_steam_world_without_a_deep_column`:
  its expectation was set against legacy's 2206 bar line, which the
  user replaced by 220.64 bar; no source says a hot Europa is no steam
  world.
- The pins of #492 (1.61 R⊕, 1679 K, condensed 1.6-1.7 R⊕, Teq
  258-280 K under an M9V star) and the F0V "kept 0": the model's
  output, for a tier-2 net that cannot reach `settled` before 2d-4.
- `interim.rs` :1088, :1132 (viewer) and the census example: not in
  the engines.

### Before the PR (main session)

End-of-branch agents in parallel (branch-reviewer, test-reviewer,
science-coherence-checker), then issue-auditor with their reports and
the draft PR text. The PR text: `Refs #659`, `Refs #594`, `Refs #602`
(slice 1's tracker, as the earlier 2d PRs, if issue-auditor agrees),
the three
departures (#684, #685, #686), the one calibration under its heading,
"Nothing generated changes; ENGINE_VERSION stays 3" with the finding
above, and the heavy output.
