# Climate pass lot 3C-2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The 0-D climate takes its outer edge, albedo and methane cooling from cited physics instead of tuned stand-ins, never fails silently, and every cause tree replays the model exactly (boils, steam mass loss, heat-pipe moons).

**Architecture:** All changes are in `crates/mapgen/src/planet/` (`climate.rs`, `albedo.rs`, `atmosphere.rs`, `mod.rs`, `regime.rs`, `explain.rs`), with gravity and radius plumbed into the climate, a convergence flag on `Climate`, and new cause notes mirrored in the web i18n. Census examples measure every step; archetype tests guard the real bodies.

**Tech Stack:** Rust (mapgen crate, ts-rs for web types), Svelte/TypeScript i18n for the new notes.

**Spec:** `docs/superpowers/specs/2026-11-18-climate-pass-lot3c2-design.md` (PR 2 sections and "Design calls").

## Global Constraints

- Physics tasks (1-8) run on opus (CLAUDE.md: physics is a hard task); 9-12 on sonnet.
- Order of the refits is fixed by the spec: #523 albedo terms, then #522 condensation, then #563 methane anchor, then the CO₂ law refit, then #565.
- Bounds (spec): Earth surface 288 ± 2 K, thermostat CO₂ in [1.87e-4, 4.2e-4] bar; Titan 93.7 ± 1 K; outer edge round the Sun 1.67 ± 0.1 AU; K and M stars within ±15 % of Kopparapu's maximum-greenhouse S_eff; equilibrium temperature moves < 0.5 K across the runaway edge for a 1 % flux change.
- No new tuned constant standing in for a process. Every new constant cites its source in its doc comment, or says "Calibration" and on what.
- If faithful physics and an archetype's real body cannot both hold: stop, report to the main session, add no constant to force either.
- Every archetype still matches its real body on seeds 1-40 (`archetype_census`).
- Formats: `SYSTEM_FORMAT_VERSION` 10 → 11 (`crates/mapgen/src/system.rs:88`), `FORMAT_VERSION` 28 → 29 (`crates/mapgen/src/export/raw_data.rs:119`), history comments updated; the main session confirms the numbers with peers before the push.
- New stored fields carry `#[serde(default …)]` so format-10 systems still load (then show as outdated).
- Shared machine: run tests throttled (`cargo test -p mapgen -j 4 -- --test-threads 4`); censuses in `--release`. Worktree `~/Developer/mg-lot3c2`; no `npm ci` (symlink `web/node_modules` to the main checkout's).
- Cause notes: new keys in `explain.rs` (Note doc at about :173-190) and in `web/src/lib/i18n/en.ts` / `fr.ts` next to `co2_build_up` (en.ts:524, fr.ts:484); `web/src/lib/causes.test.ts:131` lists note keys. Wording names the process in plain words.

## Review Focus

1. A planet sitting exactly on a phase change (CH₄ freezing, N₂ triple point 63.14 K) must return a converged temperature or a flagged one, never a silent cycle — test in Task 1.
2. A high-gravity super-Earth and a low-gravity moon with the same surface pressure must scatter differently (column mass p/g) — test in Task 2.
3. An airless or near-airless body (Mercury, Callisto exosphere) must keep its albedo: the new air terms vanish with the column — test in Task 2.
4. A dry steam runaway (all water lost) ends with zero water fraction and a positive mass equal to the dry mass — test in Task 9.
5. Format-10 systems stored on disk still deserialize (missing `converged` field) — test in Task 1.

---

### Task 1: Convergence is reported (#562)

**Files:**
- Modify: `crates/mapgen/src/planet/climate.rs` (`Climate` struct :40, `settle` :1845, `under_albedo` condensation loop :1754-1766), `crates/mapgen/src/planet/atmosphere.rs` (`methane_activity` :835), `crates/mapgen/src/planet/explain.rs` (note), `crates/mapgen/examples/climate_sweep.rs` (probe section), web i18n
- Test: `climate.rs` tests

**Interfaces:**
- Produces: `Climate.converged: bool` with `#[serde(default = "converged_default")]` returning `true`; `Cause::note("not_converged")` added under the temperature root when false; `climate_sweep phase` section printing flux, T, iterations, converged.

- [ ] **Step 1: Write the failing tests**:
  - `a_format_10_climate_reads_as_converged`: deserialize a `Climate` JSON without `converged` → `converged == true`.
  - `the_fixed_point_converges_across_the_methane_freezing_point`: a Titan-like body swept over 200 fluxes spanning `methane_freezes_k` ± 3 K; every result `converged`, and T is monotonic in flux (no jump > 1 K between neighbours).
  - `the_fixed_point_converges_across_the_nitrogen_triple_point`: same across 63.14 K for a Triton-like body (N₂ frost emissivity 0.89 → 1).
- [ ] **Step 2: Run** `cargo test -p mapgen converge -- --test-threads 4` — FAIL (field missing).
- [ ] **Step 3: Implement**: the loops return whether they met `TOLERANCE_K` / `ALBEDO_TOLERANCE_K`; `Climate.converged` carries it. Write the probe (`climate_sweep phase`). Where the probe shows a cycle, smooth the step: the methane activity and the N₂ emissivity ramp over a narrow band around the threshold (builder picks the width, ≤ 1 K, and states it as a numerical smoothing, not physics). Add the `not_converged` note and its EN/FR strings ("The climate did not settle on one temperature: the value shown is the last estimate").
- [ ] **Step 4: Run** the tests and `cargo run --release -p mapgen --example climate_sweep phase` — PASS, probe shows no cycle.
- [ ] **Step 5: Commit** `fix(planet): report and smooth the climate's phase-change cycles` (footer `Closes: #562`).

### Task 2: Gravity, area and air in the albedo (#523 part 1)

**Files:**
- Modify: `crates/mapgen/src/planet/climate.rs` (`climate`, `climate_over` signatures :1333/:1360), `crates/mapgen/src/planet/albedo.rs` (`rayleigh_550` :359, `cover` :454-475, `CLOUD_ALBEDO` :297, `bond_albedo_over` :535), `crates/mapgen/src/planet/mod.rs` (`bare` :723-779 builds the new argument), every caller of `climate`/`climate_over` (tests, explain replay)

**Interfaces:**
- Produces: `pub struct Planet { pub gravity_m_s2: f64, pub radius_earth: f64 }` in `climate.rs`, passed by reference as a new last-but-one parameter of `climate` and `climate_over` (before `crust`); `bond_albedo_over(cover, gases, deck, crust, planet: &Planet)`. Earth's values are `EARTH_GRAVITY_M_S2 = 9.81`, radius 1.0.

Physics (decided here):
- Rayleigh optical depth scales with column mass `p / g`: multiply the column by `EARTH_GRAVITY_M_S2 / planet.gravity_m_s2`.
- Ice reach: the global ocean layer depth is `OCEAN_LAYER_KM / radius_earth²` (same volume over the planet's area).
- Shortwave absorption by the air: a cited term (e.g. Earth absorbs about 23 % of incoming sunlight in the air, Trenberth et al. 2009, scaled with the column of absorbers: H₂O, CO₂, CH₄, haze) that removes light both before and after reflection. The cloud albedo constant becomes the cloud-top albedo before absorption; the builder re-derives its value so Earth's Bond albedo stays 0.29-0.31.

- [ ] **Step 1: Write the failing tests** (`albedo.rs` tests):
  - `rayleigh_counts_the_column_mass`: same gases at 1.5 bar, `g = 1.35` (Titan) vs `g = 9.81`: Titan's Rayleigh reflectance ≥ 3× the other's optical depth (ratio of g is 7.3; assert optical depth ratio within 5 % of 9.81/1.35).
  - `an_airless_body_keeps_its_surface_albedo`: zero gases → Bond albedo equals the bare surface albedo for g = 3.7 and 9.81.
  - `ice_reach_scales_with_area`: same `oceans`, radius 2 vs 1 → reach ratio 1/4 (below saturation).
  - `earth_keeps_its_bond_albedo`: Earth model → 0.29 ≤ A ≤ 0.31.
- [ ] **Step 2: Run** `cargo test -p mapgen albedo -- --test-threads 4` — FAIL.
- [ ] **Step 3: Implement** the plumbing and the three terms.
- [ ] **Step 4: Run** `cargo test -p mapgen -- --test-threads 4` and `cargo run --release -p mapgen --example archetype_census -- 10` — PASS; report Earth, Mars, Venus, Titan Bond albedos and temperatures before and after in the commit body.
- [ ] **Step 5: Commit** `feat(planet): weigh the air column and absorb sunlight in the albedo` (`Refs: #523`).

### Task 3: Cloud cover and the runaway edge (#523 part 2)

**Files:**
- Modify: `albedo.rs` (`cover` cloud term :475, `CLOUD_H2O_BAR` :282), `climate.rs` (`runaway_albedo` :687, `STEAM_ALBEDO_BAR` :661)

**Interfaces:**
- Consumes: `Planet` (Task 2).

- [ ] **Step 1: Write the failing tests** (`climate.rs`):
  - `the_runaway_edge_has_no_step`: Earth-like swept in flux across its runaway edge in 0.1 % steps; the equilibrium temperature changes by < 0.5 K between neighbours 1 % apart.
  - `cloud_cover_depends_on_more_than_vapour`: two bodies with equal vapour but different surface temperature (or relative humidity, per the cited law chosen) get different cover.
  - Earth's cloud cover stays 0.67 ± 0.03.
- [ ] **Step 2: Run** — FAIL.
- [ ] **Step 3: Implement**: cloud cover from a cited dependence (builder's choice, e.g. relative humidity as in Sundqvist-type schemes, or the moist-greenhouse cover of Kopparapu/Goldblatt); the runaway albedo blends continuously from the wet albedo at the edge (no switch on `water.runaway` alone).
- [ ] **Step 4: Run** all mapgen tests and the census — PASS.
- [ ] **Step 5: Commit** `feat(planet): grow clouds with humidity and blend albedo at the runaway edge` (`Refs: #523`).

### Task 4: Maximum greenhouse from CO₂ condensation (#522)

**Files:**
- Modify: `climate.rs` (`Body::new` cap :1497-1505, `most_co2`, `thermostat` :1894, `maximum_greenhouse` :1975, `climate_over` snowball :1388-1393, `SNOWBALL_BUILT_UP` :1414, `branches` :1658), `atmosphere.rs` (`co2_frost_bar` :897 stays as the saturation curve), explain notes

**Interfaces:**
- Produces: `fn tropopause_k(skin_k: f64) -> f64` (skin temperature of the actual column, `T_eff · 2^(-1/4)`, cited) and the cap `co2_frost_bar(tropopause_k(...))` evaluated on the settled climate, not on `context.equilibrium_k`. `SNOWBALL_BUILT_UP` deleted. New note key `co2_condenses` ("CO₂ condenses high in the air: more of it no longer warms the ground").

Physics: the CO₂ that warms is capped where it saturates at the column's own upper-air temperature, which depends on the albedo it produces, so the cap is solved inside the settle loop. A snowball's CO₂ is what the thermostat reaches (min of the thaw CO₂ and the condensation cap); the "frozen for good" outcome then exists only where even the cap cannot thaw.

- [ ] **Step 1: Write the failing tests**:
  - `the_outer_edge_round_the_sun_is_kopparapus`: Earth-like (plates, Earth water and carbon) round the Sun; find the largest orbit with liquid water by bisection; 1.57 ≤ edge ≤ 1.77 AU (Kopparapu et al. 2013 give 1.70 AU, the spec's 1.67 ± 0.1 covers it).
  - `the_outer_edge_follows_the_star`: same for T_eff 4500 K and 3300 K; S_eff of the edge within ±15 % of Kopparapu's maximum greenhouse fit `S_eff = S_⊙ + a T + b T² + c T³ + d T⁴`, `T = T_eff − 5780`. Builder copies the maximum-greenhouse coefficients from Kopparapu et al. 2013 (doi:10.1088/0004-637X/765/2/131, corrected table of the erratum/arXiv v3: S_⊙ = 0.3438, a = 5.8942e-5, b = 1.6558e-9, c = −3.0045e-12, d = −5.2983e-16 — verify against the paper before use).
  - `a_snowball_holds_the_co2_its_thermostat_reaches`: a snowball body's CO₂ equals `min(thaw_bar, cap)` within 1 %.
  - The existing `climate_at(&earth, 1.6)` tests (:2533/:2549) and `snowball_chance` tests (:2847-2961) updated to the new physics, not deleted; the 1.5-1.65 AU chance-1 band re-measured and reported in the commit body.
- [ ] **Step 2: Run** — FAIL.
- [ ] **Step 3: Implement**; the `thermostat`/`co2_build_up` explain notes gain `co2_condenses` when the cap binds; EN/FR strings.
- [ ] **Step 4: Run** all mapgen tests; `climate_sweep sweep` — PASS; edge distance in the commit body.
- [ ] **Step 5: Commit** `feat(planet)!: take the maximum greenhouse from CO2 condensing aloft` (`Closes: #522`).

### Task 5: CO₂ on hazy hydrocarbon worlds (#557)

**Files:**
- Modify: `atmosphere.rs` (`outgassed` :211, `cold_trap` :1111), `mod.rs` (`bare` :765-766 inventory and frozen CO₂), `web/src/lib/causes.ts` (dominant frost naming, `FROST_COVERS`), `classify.rs` tests (:791-816)

- [ ] **Step 1: Measure** with `planet_census` and `archetype_census titan-like`: CO₂ inventory, frozen CO₂ and frost cover share of hazy hydrocarbon worlds vs Titan (air CO₂ ≈ 15 ppb, Coustenis et al. 2010; no surface CO₂ ice layer). Write the numbers in the task report.
- [ ] **Step 2: Write the failing tests**:
  - `a_hazy_hydrocarbon_world_wears_no_co2_frost_sheet`: census seed 387 planet d (rebuild through `system` test helpers) → class `HydrocarbonWorld` and CO₂ frost cover < 0.5 of the disc.
  - `titan_like` CO₂ frost cover < 0.1 on seeds 1-40.
  - web `causes.test.ts`: a body with hydrocarbon liquid and a CO₂ frost over half the disc is named by its liquid, not the frost.
- [ ] **Step 3: Implement** the supply change the measurement justifies (cited: CO₂ destroyed or not produced in a reducing, methane-rich air), and the naming precedence liquid > frost.
- [ ] **Step 4: Run** mapgen tests, web `npm test` — PASS.
- [ ] **Step 5: Commit** `fix(planet): keep CO2 frost off hazy hydrocarbon worlds` (`Closes: #557`).

### Task 6: Methane anti-greenhouse anchored (#563)

**Files:**
- Modify: `climate.rs` (`CH4_NEAR_IR_MAX` :286, `CH4_NEAR_IR_HALF_BAR` :297, docs :196, :274-304)

- [ ] **Step 1: Write the failing test** `the_methane_anti_greenhouse_matches_mckay`: Titan-like `stratospheric_share` equals the independent value (McKay et al. 1991 methane-only absorbed share, or the Huygens DISR surface flux, Tomasko et al. 2008 — builder picks one and cites it in the test) within 10 %.
- [ ] **Step 2: Run** — FAIL.
- [ ] **Step 3: Implement**: fix the anchored constant from that number, re-fit the other on Titan's 93.7 K.
- [ ] **Step 4: Run**: Titan-like 93.7 ± 1 K over seeds 1-40 (`archetype_census titan-like 40`), all tests — PASS.
- [ ] **Step 5: Commit** `fix(planet): anchor the methane anti-greenhouse on Titan's measured flux` (`Closes: #563`).

### Task 7: CO₂ law refit on Earth (#523 end)

**Files:**
- Modify: `climate.rs` (`CO2_OPACITY` :127, `CO2_EXPONENT` :148, tests :2773-2818)

- [ ] **Step 1: Write the failing test** `earth_holds_its_preindustrial_co2`: Earth model → thermostat CO₂ in [1.87e-4, 4.2e-4] bar and surface 286-290 K; the late-Archean and #441 calibration points documented at :122-148 still hold within their stated tolerances.
- [ ] **Step 2: Run** — FAIL (today 1.2e-3).
- [ ] **Step 3: Refit** the two constants; update their doc comments with the new calibration points.
- [ ] **Step 4: Run** all mapgen tests; `archetype_census -- 40` — every archetype in range.
- [ ] **Step 5: Commit** `fix(planet): refit the CO2 law so Earth holds 280 ppm` (`Closes: #523`).

### Task 8: Titan's pressure over 500 seeds (#565)

**Files:**
- Modify: `crates/mapgen/src/archetype.rs` (`titan_like_has_titan_air` :1392-1399, whole-system variant :1408-1413), `archetypes/titan-like.toml` (or the built-in file holding `n2`) if retuned

- [ ] **Step 1: Measure** titan-like pressure over seeds 1-500 (`archetype_census titan-like 500`): min, 5th, 50th, 95th percentile, max.
- [ ] **Step 2:** Retune `n2` so the median sits at 1.47 ± 0.05 bar, then set both test ranges to the measured 1-500 spread rounded outward to 0.05 bar, with the measurement in a comment (current tests use 1.2-1.8 over seeds 0..20).
- [ ] **Step 3: Run** the archetype tests over seeds 0..40 — PASS.
- [ ] **Step 4: Commit** `test(archetype): bound titan pressure by its 500-seed spread` (`Closes: #565`).

### Task 9: Mass after a steam runaway (#513)

**Files:**
- Modify: `crates/mapgen/src/planet/mod.rs` (`chain` :473-540, boil block :509-526), `regime.rs` (`steam` :110), `explain.rs` (`mass()` :357, `replay` :339), web i18n

**Interfaces:**
- Produces: `fn after_steam(inputs: &PlanetInputs, water: &Water) -> PlanetInputs` in `mod.rs` returning `mass_earth` and `water_fraction` with the escaped water removed; `drawn_mass_earth` stays the formation mass. The mass tree shows `Mass` ← `WaterFormed`, `Escape` (water lost, in Earth oceans).

- [ ] **Step 1: Write the failing tests**:
  - `a_steam_runaway_weighs_only_the_water_it_kept`: a boiled body: `mass_earth == drawn − lost_water_kg / EARTH_KG` (1e-9 relative), `water_fraction` from kept water, radius and gravity re-derived (smaller than before for a water-rich body).
  - `a_dry_runaway_keeps_its_dry_mass`: all water lost → `water_fraction == 0`, `mass_earth > 0`.
  - explain: the mass tree of a boiled body has a water-lost cause; `explain` returns `Some` for it.
- [ ] **Step 2: Run** — FAIL.
- [ ] **Step 3: Implement**: the boil block re-derives `interior` from `after_steam` before the steam/condensed `bare` runs; the replay does the same.
- [ ] **Step 4: Run** all mapgen tests — PASS.
- [ ] **Step 5: Commit** `fix(planet): drop the escaped water from a steam world's mass` (`Closes: #513`).

### Task 10: The boil that condensed again (#517)

**Files:**
- Modify: `explain.rs` (water tree / temperature root), web i18n EN/FR, `web/src/lib/causes.test.ts`

- [ ] **Step 1: Write the failing test** `a_recondensed_boil_shows_in_the_tree`: a body with `water.boiled && !water.runaway` has a `boiled_recondensed` note whose value is the water lost (Earth oceans).
- [ ] **Step 2: Run** — FAIL.
- [ ] **Step 3: Implement** the note; strings: EN "The ocean boiled once: escape took {lost} of its water to space, and what was left condensed again"; FR « L'océan a bouilli une fois : l'échappement a emporté {lost} de son eau dans l'espace, et le reste s'est condensé de nouveau ».
- [ ] **Step 4: Run** mapgen and web tests — PASS.
- [ ] **Step 5: Commit** `feat(planet): explain an ocean that boiled and condensed again` (`Closes: #517`).

### Task 11: Heat-pipe moons replay (#520)

**Files:**
- Modify: `explain.rs` (`replay` :339, `surface_interior` :322, `temperature_tree` :504-543), `crates/mapgen/src/system.rs` (test next to `body_context_rebuilds_the_drawn_contexts` :4486)

- [ ] **Step 1: Write the failing test** `heat_pipe_moons_explain_their_temperature`: request earth-like as a giant's moon (`HostMode::Giant`), request seeds 1-40, caller seeded 5 as in the issue; `explain` is `Some` for every body, and the temperature root equals the stored one within `TEMPERATURE_TOLERANCE_K`.
- [ ] **Step 2: Run** — FAIL on the seeds listed in #520 (if Tasks 1-9 changed which seeds fail, the test still must fail before the fix on at least one seed; if none fails, record that and keep the test as a guard).
- [ ] **Step 3: Implement**: find the step the chain applies to a steaming heat-pipe `VolcanicWorld` that `replay` leaves out (steam layer interior, vapour or absorbing context passed to `surface_temperature_k`) and replay it.
- [ ] **Step 4: Run** all mapgen tests — PASS.
- [ ] **Step 5: Commit** `fix(planet): replay the climate of tidally heated heat-pipe moons` (`Closes: #520`).

### Task 12: Formats, docs and census

**Files:**
- Modify: `crates/mapgen/src/system.rs:60-88`, `crates/mapgen/src/export/raw_data.rs:100-119`, `docs/science/planet-atmospheres.md` (`## Climate and surface liquid` :709, CO₂ table :781-794, cap :995, albedo :1013, "Branches and snowballs" :1191-1210, `## Census` :1487), `docs/science/habitable-zone.md` (outer edge), `docs/system-spec.md` (format sections :576, :1058-1073), README lines listing versions (:252, :548), in-code docs (climate.rs :1280-1330, :1466; albedo.rs :261-297; explain.rs :173-190)

- [ ] **Step 1:** Bump `SYSTEM_FORMAT_VERSION` to 11 and `FORMAT_VERSION` to 29 with history lines; run the server tests (`cargo test -p server -j 4 -- --test-threads 4`) that pin versions.
- [ ] **Step 2:** Rewrite the docs above: the outer edge from CO₂ condensing aloft (Kopparapu 2013), the albedo with column mass, air absorption, area-scaled ice and cloud cover, the methane anchor, convergence flag, steam mass loss; refresh the census tables from `planet_census -- 500` and `archetype_census -- 40`.
- [ ] **Step 3:** Full run: `cargo test --workspace -j 4 -- --test-threads 4`, web `npm test` and `npm run check` — PASS.
- [ ] **Step 4: Commit** `docs(planet): document the lot 3C-2 climate` (and the version bumps in a `feat(planet)!:` commit with `BREAKING CHANGE: system format 11 and world format 29; stored systems are outdated`).
