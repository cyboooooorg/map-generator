# Planet model PR 2d-1: the air Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port from `legacy-final` the air's own chemistry between the
bare air of 2c and the climate: the methane photolysis by the stars'
Lyman α, the outgassing redone without CO₂ for a reducing air, the cold
trap of CO₂ and NH₃, and the photochemistry (NH₃ to N₂, methane kept
only in a cold air), each gas with its cause tree.

**Architecture:** The laws go to `science` (the NH₃ frost curve and a
CO₂ frost reading in `volatiles`, a new `photochemistry` module for the
photolysis, the reducing test and the NH₃ and methane rules). The steps
go into `system::planet::air`: the photolysis and the reducing test
before the other gases are outgassed, the cold trap and the
photochemistry after the SO₂ wash-out, and before the pressure node, so `bare`
returns the air the climate of 2d-3 will
read. Nothing is public and nothing generated changes: no engine
version bump.

**Tech Stack:** Rust, `cargo nextest` tiers, the `Trace`/`Traced` cause
API of `timeline`, `science::xuv::History`.

**Spec:** `docs/superpowers/specs/2026-11-23-planet-model-pr2d-design.md`
(sections "2d-1: the air", "Rules for the build", "Science to study
first", rows 2d-1). Reference page: `docs/science/planet-atmospheres.md`
("Methane photolysis" l.~872, "Reducing air" l.~926, "Cold trap"
l.~957, "Photochemistry" l.~997; grep the headings, the page is 2 400
lines). Deferred list: #659.

Legacy paths below are under `legacy-final:crates/mapgen/src/planet/`:
`atm` is `atmosphere.rs`, `mod` is `mod.rs`.

## Decisions of 2026-11-23 (with the user)

- **Photolysis, approach A.** Legacy's law (`atm` l.1100-1145) is
  ported with its onset: methane outgassed at a steady rate over the
  age gathers only once the supply outruns the destruction. The
  destruction at age `t` is summed over the sources: each source's
  present bolometric flux times its own Lyman α share at `t`, read from
  the same `science::xuv::History` the timeline's `lyman_alpha` fluence
  comes from (`crates/system/src/sources.rs:88`). Since #622 a band is
  a share of the star's ZAMS luminosity (Lyman α 1.043e-4), turned into
  a share of the present light by the history's `scale`; legacy's
  ratio was a share of the present light. The rewrite's history wins,
  so the yield fitted on legacy's ratio is fitted again (Task 1). The
  onset is found by bisection whatever the number of sources: one code
  path, no analytic branch for one star.
- **The reducing air in one pass.** Legacy ran the escape twice
  (`mod` l.~915-1075): once to see whether methane survives, then again
  with no CO₂ outgassed. In the rewrite the shoreline's kept share does
  not read the air's composition (`air.rs` `shoreline`: mass, radius,
  XUV fluence, dynamo), and the photolysis reads only the methane, so
  the second pass gives the first pass's methane and the same share.
  The port decides `reducing` from the methane first, then outgasses
  CO₂ at 0 when it holds: same values as legacy's rerun, every node
  recorded once. `air()` takes a `hold_back: bool` (`bare` passes
  `true`), which 2d-3's fallback will set to `false`.
- **`condense`, the N₂ and SO₂ frost curves and `SEA_METHANE_SHARE`
  are not in 2d-1:** they read the surface temperature and the liquids
  (2d-2, 2d-3).
- **The reducing-air fallback** (legacy `bare`, `mod` l.~873: run again
  without the hold-back when the climate leaves no Titan-like air) reads
  the climate: 2d-3. 2d-1 ports the hold-back alone.
- **Non-thermal loss** (#658) is not in the rewrite: the photolysis
  reads the shoreline-kept air directly, where legacy read the air after
  `non_thermal_loss`.

## Global Constraints

- Legacy is read only at tag `legacy-final`; every law is ported with
  its tests, then changed only where the decisions above say so.
- Every numeric `const`/`static` and every `let` bound to a bare number
  other than 0, 0.5, 1, 2 has a doc comment starting `Source:`,
  `Calibration:` or `Definition:`, in `science` (guardrail
  `science_constants_are_sourced`, which scans test files too) and in
  `system` by the same rule. Prefixes, values and citations are those of
  the study of Task 1, never the legacy comment: "from memory, not
  checked" notes are not carried over.
- Missing science stops the work: a stop goes to the main session,
  which brings it to the user. No test is `#[ignore]`d and no tolerance
  widened to get past a stop. A legacy bound that is an observation
  (Earth, Titan) stays; if it fails, stop and report. More than two new
  calibrations in this PR: report before going on.
- `science` takes and returns the typed units of `units.rs` at its
  public boundary, never `timeline` types. No `HashMap`,
  `thread_local!`, file, clock or process in `science` or `system`.
- Every step value is recorded in the `Trace` with a code of
  `planet/codes.rs` (`planet.air.<step>.<gas>`), derived from the nodes
  it was computed from: one code path per value.
- No new draw.
- New `system` code stays crate-private under the existing
  `#[cfg_attr(not(test), expect(dead_code, …))]` of `planet/mod.rs`.
- `mise run ci` passes at the end of every task; tier-1 budget 2 min.

## Review Focus

1. **No methane, or age 0:** the photolysis returns the air unchanged,
   no NaN from `supply = ch4 / age`. (Task 3 test
   `no_methane_or_no_age_changes_nothing`.)
2. **Destruction always above the supply** (a close, young or active
   star): the onset is at or past the age and the methane left is 0,
   never negative. (Task 3 test `a_close_planet_keeps_no_methane`.)
3. **A cold trap below 20 K or at the triple point:** the cap is finite
   and non-negative below, and the gas is untouched at or above the
   triple point. (Task 2 test `the_frost_cap_holds_from_10_k_to_the_triple_point`.)
4. **Two stars:** two identical sources at half the flux each destroy
   what one source does, and the onset is the same. (Task 3 test
   `two_half_sources_photolyse_like_one`.)
5. **A reducing air:** CO₂ is 0 in every node after the outgassing
   (kept, trapped, frozen), the other gases keep the values a
   non-reducing pass gives them, except SO₂, whose wash-out reads the
   lighter column on a dry body (a wet one reads the same); `hold_back =
   false` outgasses CO₂ as usual. (Task 5
   tests `titan_outgasses_no_co2` and
   `a_held_back_air_differs_only_by_its_co2`.)

---

### Task 1: The study (opus, main session reviews)

**Files:**

- Create: the Wiki page "23-air-chemistry-audit" (22 is taken by the
  surface water study of #663) in a scratchpad clone
  of `https://github.com/cyboooooorg/map-generator.wiki.git` (pushed
  before the PR, by the main session)
- Modify: `docs/science/planet-atmospheres.md` (the four sections above:
  sources, a calibration section for each `Calibration:` saying what was
  searched and not found)

Answer, with the source read (full text when the abstract does not
carry the number; say which):

1. `CH4_PER_LYMAN_ALPHA_PHOTON` (13.9): Yung, Allen & Pinto 1984
   (ApJS 55, 465) in full. Was Titan's 5.6e4 g cm⁻² lost at today's
   rate or over a history? Fit the yield again with the rewrite's
   Lyman α history (`History::integrated_between`, ZAMS share times
   `scale`, #622) over Titan's age at 9.57 AU (as built; legacy's 9.58 AU
   gives 14.10), against the loss the
   paper gives, and say how far it moves from legacy's 13.9. Whether a
   yield above one is physical (photosensitised dissociation).
2. `METHANE_SURVIVES_K` (150 K): a source for the temperature below
   which water stays too low for OH to oxidise methane; otherwise a
   calibration between Titan and Mars, with its section.
3. NH₃ triple point, triple pressure and sublimation curve: NIST
   WebBook or Fray & Schmitt 2009 (Planet. Space Sci. 57, 2053), read.
4. `N2_PER_NH3` (28/34): confirm that the chain's partial pressures are
   weight pressures (`air.rs` `outgassed`: `weight(mass)`), so a mass
   ratio is right; cite Atreya, Donahue & Kuhn 1978 or drop it for the
   stoichiometry alone (`Definition:`).
5. Earth's N₂ target, 0.746 bar by weight (a source; 0.78 is the volume
   share), and the band the test keeps. As built: the constant was
   refitted from 0.8 to 0.8145 (Earth 0.7458 bar).
6. The test fixtures: Titan and Callisto as planet trials (mass, core
   mass fraction, water fraction, orbit, eccentricity), each with a
   source. Both are moons; as planets they have no host plasma, which
   legacy's non-thermal loss also ignored for them only through the
   host (#658).
7. The Planck constant (CODATA, exact) for the Lyman α photon energy
   `h c / λ` at 121.567 nm (the wavelength sourced), and the methane
   molecule mass from `CH4_MOLAR_MASS / AVOGADRO`.
8. The onset bisection: its step count as a `Definition:` (the
   tolerance in Gyr it guarantees on `[0, age]`), held by the stepwise
   budget test of Task 3.

Output: one table row per value (final value, prefix, source read),
committed in the page and the Wiki study. Stop and report to the main
session at any GAP that stays open.

- [ ] **Step 1:** Write the study and extend the page.
- [ ] **Step 2: Run** `mise run docs:check` — PASS.
- [ ] **Step 3: Commit** `docs(science): source the air's photochemistry`.

### Task 2: Frost curves in `science::volatiles` (sonnet)

**Files:**

- Modify: `crates/science/src/volatiles.rs`,
  `crates/science/src/volatiles_tests.rs`,
  `crates/science/src/constants.rs` (Planck constant, if Task 1 adds it)

**Interfaces:**

- Produces:
  - `pub fn co2_frost_pressure(t: Kelvin) -> Option<Pascal>`: the
    sublimation pressure of the existing CO₂ curve below its triple
    point, `None` at or above it (the gas is not trapped there).
  - `pub fn ammonia_frost_pressure(t: Kelvin) -> Option<Pascal>`: the
    same for NH₃, with the values of Task 1.
  - Both clamp `t` at the module's `FLOOR_TEMPERATURE` as the existing
    curves do.

- [ ] **Step 1: Write the failing tests:**
      `ammonia_frost_meets_its_triple_point` (the triple pressure of
      Task 1 within 1e-6 relative just under the triple temperature);
      `ammonia_frost_follows_its_source` (one tabulated point of Task 1's
      source within its stated precision);
      `co2_frost_is_the_co2_curve_below_its_triple_point` (equal to
      `co2_saturation_pressure` at 150 K and 200 K);
      `the_frost_cap_holds_from_10_k_to_the_triple_point` (Review Focus
      3: positive, finite, strictly increasing from 10 K to just under
      the triple point; `None` at and above it, for both gases).
- [ ] **Step 2: Run** `cargo nextest run -p science volatiles` — FAIL.
- [ ] **Step 3: Implement** with the `Curve`/`Sublimation` machinery of
      the module (a constant-enthalpy or equation branch as Task 1's
      source gives).
- [ ] **Step 4: Run** — PASS.
- [ ] **Step 5: Commit** `feat(science): add the CO2 and ammonia frost curves`.

### Task 3: Methane photolysis in `science::photochemistry` (opus)

**Files:**

- Create: `crates/science/src/photochemistry.rs`,
  `crates/science/src/photochemistry_tests.rs`
- Modify: `crates/science/src/lib.rs` (`pub mod photochemistry`),
  `crates/science/src/xuv.rs` (one method)

**Interfaces:**

- Consumes: `xuv::{History, LYMAN_ALPHA, ratio}`,
  `constants::{SOLAR_IRRADIANCE, AVOGADRO, SPEED_OF_LIGHT}` and the
  Planck constant, `radiation::gases::CH4_MOLAR_MASS`.
- Produces:
  - `impl History { pub fn ratio_at(&self, band: Band, age: Gyr) -> f64 }`:
    the band's share of the present bolometric light at `age`
    (`ratio(band, mass, age) * scale`, 0 for a coronal band outside the
    corona), the integrand of `integrated_between`.
  - `pub struct LymanSource<'a> { pub flux: WattPerSquareMetre,
    pub history: &'a History }`:
    one star's present bolometric flux at the planet and its history.
  - `pub const CH4_PER_LYMAN_ALPHA_PHOTON: f64` (Task 1's value and
    prefix), the photon energy and molecule mass as `Definition:`
    constants.
  - `pub fn methane_destroyed(gravity: MetrePerSecondSquared,
    sources: &[LymanSource], from: Gyr, to: Gyr) -> Bar`:
    the methane the sources' Lyman α destroys between two ages,
    photon-limited (legacy `methane_destroyed_bar_per_gyr` times the
    integrated share, summed over the sources; the flux is averaged over
    the sphere, `/ 4`).
  - `pub fn methane_left(outgassed: Bar, gravity: MetrePerSecondSquared,
    sources: &[LymanSource], age: Gyr) -> Bar`:
    legacy `methane_photolysis` on the sum. The supply is `outgassed /
    age`; the onset is the age where the summed destruction rate falls
    to the supply, by bisection on `[0, age]` (the rate only falls after
    saturation; before the onset nothing gathers); the result is
    `supply · (age − onset) − methane_destroyed(…, onset, age)`, at least
    0. Returns `outgassed` unchanged when it or `age` is 0. The
    bisection's step count is a constant with the prefix of Task 1,
    item 8.

- [ ] **Step 1: Write the failing tests** (from `atm` l.1545-1640, values
      per Task 1):
      `titan_loses_methane_at_the_rate_of_yung_1984` (the present rate at
      9.57 AU (as built) around a 1 M☉, 4.6 Gyr history, in molecules cm⁻² s⁻¹,
      equals Task 1's target within 1 %);
      `photolysis_matches_a_stepwise_budget` (legacy's five cases
      `(9.57, 1.0, 4.6, 19.0)`, `(9.57, 1.0, 4.6, 3.0)`,
      `(5.2, 1.0, 4.6, 0.5)`, `(1.0, 0.3, 8.0, 300.0)`,
      `(9.57, 1.0, 0.05, 19.0)` as (AU, M☉, Gyr, bar), against a
      200 000-step integration of `supply − rate(t)` clamped at 0,
      within `1e-3 · ch4`);
      `the_whole_destruction_matches_the_timelines_fluence`
      (`methane_destroyed` from 0 to the age equals the yield × the
      fluence `history.exposure(LYMAN_ALPHA).fluence(flux)` / photon
      energy × molecule mass × g / 4, within 1e-9 relative);
      `no_methane_or_no_age_changes_nothing` (Review Focus 1);
      `a_close_planet_keeps_no_methane` (Review Focus 2: 0.05 AU, 1 bar);
      `two_half_sources_photolyse_like_one` (Review Focus 4, 1e-9
      relative).
- [ ] **Step 2: Run** `cargo nextest run -p science photochemistry` — FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** — PASS.
- [ ] **Step 5: Commit** `feat(science): port the methane photolysis over
      every star`.

### Task 4: Reducing air, ammonia and methane rules (sonnet)

**Files:**

- Modify: `crates/science/src/photochemistry.rs`,
  `crates/science/src/photochemistry_tests.rs`

**Interfaces:**

- Produces:
  - `pub const METHANE_SURVIVES: Kelvin` (Task 1's value and prefix).
  - `pub const N2_PER_NH3: f64` (Task 1's prefix).
  - `pub fn reducing(methane: Bar, equilibrium: Kelvin) -> bool`:
    methane left and `equilibrium < METHANE_SURVIVES` (`atm` l.1170).
  - `pub fn methane_survives(equilibrium: Kelvin) -> bool`.
  - `pub fn nitrogen_from_ammonia(ammonia: Bar) -> Bar`.

- [ ] **Step 1: Write the failing tests:**
      `photolysis_conserves_nitrogen` (34 bar NH₃ and 1 bar N₂ give
      28.963 bar N₂ within 1e-9 (as built: the molar masses, 28.014 /
      (2 × 17.031), not 28/34));
      `titans_air_is_reducing_and_earths_is_not` (Titan's equilibrium of
      Task 1 with 1 bar of methane is reducing; 255 K is not; no methane
      is never reducing).
- [ ] **Step 2: Run** `cargo nextest run -p science photochemistry` — FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** — PASS.
- [ ] **Step 5: Commit** `feat(science): port the reducing air and the
      ammonia photochemistry`.

### Task 5: The air steps in `system::planet::air` (opus)

**Files:**

- Modify: `crates/system/src/planet/air.rs`,
  `crates/system/src/planet/air_tests.rs`,
  `crates/system/src/planet/codes.rs`,
  `crates/system/src/planet/bare.rs` (`BareSurface`, header doc),
  `crates/system/src/planet/bare_tests.rs`,
  `crates/system/src/planet/tests.rs` (fixtures of Task 1),
  `crates/system/src/planet/mod.rs` (module docs)

**Interfaces:**

- Consumes: Tasks 2-4; `xuv::history(source.mass, trial.metallicity,
  trial.age)` once per source per chain, its `StellarError` carried by a
  new `InteriorError::Stellar(StellarError)` variant (the draw only
  places main-sequence stars, so it is not expected); each source's
  `flux` as `WattPerSquareMetre`;
  `trial.sources.equilibrium_k()` with the sources' `equilibrium` nodes
  as causes; `body.bulk().surface_gravity`.
- Produces: `air(…, hold_back: bool, trace)` returns `pub(crate) struct
  Air { pub atmosphere: Atmosphere, pub water: Water, pub frozen_co2:
  Traced<Bar>, pub reducing: Traced<bool> }` instead of the tuple;
  `BareSurface` gains `frozen_co2` and `reducing` (2d-3's carbon cycle
  and the reducing fallback read them). In `air()`, in order (decision
  "The reducing air in one pass"):
  1. outgassed CH₄, the shoreline share and the kept CH₄, as today;
  2. `planet.air.photolysed.ch4` from the kept CH₄, the surface gravity
     node, each source's `flux` and `mass` nodes, and the trial's
     `metallicity` and `age` nodes (the inputs of the histories);
  3. `planet.air.reducing` from it and the sources' `equilibrium`
     nodes, `false` when `hold_back` is false;
  4. the other outgassed gases; `planet.air.outgassed.co2` is 0 when
     reducing and then cites the reducing node; kept gases and the SO₂
     wash-out as today;
  5. `planet.air.trapped.co2` and `planet.air.trapped.nh3`, the cap of
     Task 2 at the equilibrium temperature;
     `planet.air.frozen-co2` = kept CO₂ − trapped CO₂ (the
     photolysis leaves CO₂ as kept; legacy books all of it as frost, the
     excess #590 tracks for small icy bodies under T2);
  6. `planet.air.photochemistry.n2`, `.nh3` (0), `.ch4`;
  7. the pressure node from the final partials.
  Each node cites the previous node of its gas and the step's own
  inputs, so a gas's tree reads outgassed → shoreline-kept → (SO₂-kept)
  → photolysed → trapped → photochemistry. `Water::keeping` in a runaway
  keeps reading the kept H₂O (no step changes water).

- [ ] **Step 1: Write the failing tests:**
      `earth_has_no_abiotic_methane_or_ammonia` (CH₄ and NH₃ below 1e-3
      bar, `atm` l.1522);
      `earth_has_a_bar_of_nitrogen`, the existing 2c test, its band
      narrowed to Task 1's around 0.746 bar by weight (as built: retuned
      to 0.8145, Earth 0.7458 bar; retune
      `NITROGEN_RECYCLED_UNDER_PLATES` in
      `crates/science/src/outgassing.rs` and its calibration section if
      it misses; the retune goes in the PR description);
      `titan_keeps_its_methane` (CH₄ above 0.01 bar, the Task 1 fixture);
      `callisto_loses_its_methane` (CH₄ below 1e-3 bar, `atm` l.1534);
      `titan_outgasses_no_co2` (reducing, CO₂ 0, its outgassed node
      derives from the reducing node);
      `a_held_back_air_differs_only_by_its_co2` (Review Focus 5: Titan
      with `hold_back` true and false);
      `venus_and_mars_keep_their_co2` (Venus's equilibrium is above the
      CO₂ triple point, so no cap; Mars's is below it, but its CO₂ stays
      under the frost pressure, so the cap does not bite; Venus's CO₂ and
      Mars's total as in 2c's tests, `frozen_co2` 0 for both);
      `each_partial_reads_in_chain_order` (the ancestry of each final
      partial contains its kept node and, for CH₄, the photolysed node;
      `frozen_co2` derives from the photolysed and trapped CO₂).
- [ ] **Step 2: Run** `cargo nextest run -p system planet::air planet::bare` — FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** — PASS; existing `planet::*` tests still pass.
- [ ] **Step 5: Commit** `feat(system): add the photolysis, cold trap and
      photochemistry to the air`.

### Task 6: Documentation and CI (sonnet)

**Files:**

- Modify: `docs/science/planet-atmospheres.md` ("How it computes them"
  of the four sections: rewrite paths and cause trees; "Where it
  departs": the yield fitted again on the ZAMS-share history, several
  stars, the reducing air in one pass, no non-thermal loss before the
  photolysis, `condense` and the fallback in 2d-3),
  `crates/system/README.md` (the chain up to the photochemistry),
  `crates/science/README.md` if it lists modules.

- [ ] **Step 1:** Update the pages (the module docs of `air.rs`,
      `bare.rs` and `planet/mod.rs` are Task 5's).
- [ ] **Step 2: Run** `mise run ci` — PASS, tier budgets included.
- [ ] **Step 3: Commit** `docs: describe the air's photochemistry`.

### Before the PR (main session)

- `branch-reviewer` and `test-reviewer` once on the whole branch, given
  this plan and the design; after fixes, `mise run ci` again.
- Push Wiki study 23 and link it.
- PR description: `Refs #602`, `Refs #659`, `Refs #592`, `Refs #590`
  (the frozen CO₂ booked as in legacy); a heading
  "Calibrations" listing every `Calibration:` added or retuned; a note
  that nothing generated changes (model private until 2d-4), so no
  `mise run test:heavy` output.
