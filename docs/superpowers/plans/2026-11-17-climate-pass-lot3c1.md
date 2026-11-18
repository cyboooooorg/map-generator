# Climate pass lot 3C-1: surfaces and thin airs — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make every icy and volcanic moon archetype come out as its real
body on every seed: surface age from the process that renews the surface
(with terrain units in 0-D), thin airs from supply and loss, Io's SO2,
icy albedo from surface age, liquid nitrogen, Triton's N2 curve, and the
methane residuals.

**Architecture:** A new module `planet/surface_age.rs` computes the
body's terrain units (age, share, renewing process) deterministically in
the model. A first estimate from heat and resonance history is computed
before the climate (the icy albedo reads it); the full units, which need
the climate's liquid, air and frost, are computed after it and stored in
`PlanetModel.terrain`. `params_for` turns the main unit's age into
`WorldParams.surface_age_ga` with its existing draws. Air loss and SO2
live in `planet/atmosphere.rs`, the icy albedo in `planet/albedo.rs`.

**Tech Stack:** Rust (crate `mapgen`), TypeScript/Svelte card
(`web/src`), ts-rs generated types, vitest.

**Spec:** `docs/superpowers/specs/2026-11-17-climate-pass-lot3c1-design.md`
(approved 2026-11-17). Executors read both.

## Global Constraints

- Branch `feat/climate-lot3c`, worktree `~/Developer/mg-lot3c`, from `main` 99690d2.
- World `FORMAT_VERSION` 27 → 28 (`crates/mapgen/src/export/raw_data.rs:109`, mirrored in `web/src/lib/formatVersion.ts`), `MIN_FORMAT_VERSION` stays 24; `SYSTEM_FORMAT_VERSION` 9 → 10 (`crates/mapgen/src/system.rs:82`). Done once, in Task 10.
- No per-archetype surface age, albedo or pressure in `archetypes/*.toml`: archetypes pin inputs only; every result comes from a law applied to every body.
- Every new constant has a doc comment with its source (author, year, doi or journal) or, for a calibration, the body and value it reproduces; `docs/science/` records each.
- The draw order of `params_for` (`crates/mapgen/src/planet/interim.rs:612-631`) stays: the six look draws, then exactly one regime draw. A body whose renewal rule does not change keeps the same stored surface age to the last bit.
- Bodies right today stay right: Venus, Earth, Mars, Mercury, the giants, Europa's and Enceladus's young surfaces, Io's heat pipe, Titan's 1.3-1.6 bar and its seas, Triton's thin N2 air.
- Tests: `RAYON_NUM_THREADS=4 cargo test -p mapgen --all-features <filter> -- --test-threads=3`; full run `RAYON_NUM_THREADS=4 cargo test --workspace --all-features -- --test-threads=3`, one at a time on this machine; web `cd web && npm run check && npm test`. Never `npm ci` in the worktree, never port 3000, never write under `~/Developer/cyboooooorg/map-generator/{systems,worlds,db}`.
- Census: `cargo build --release -p mapgen --example archetype_census && ./target/release/examples/archetype_census 40 > "$SCRATCH/census.txt"` (about 40 s). Each task that changes a result pastes the affected archetypes' min/median/max into its commit body.
- Documentation in the same commit as the behaviour: `docs/science/` (find the section with `grep -n '^#' docs/science/*.md`), README surface and atmosphere paragraphs (`grep -n 'surface age\|Surface age\|exosphere\|airless' README.md`, never read README whole).

## Acceptance (from the spec), every seed 1-40 of `archetype_census`

| Body | Surface age | Air | Bond albedo | T |
| --- | --- | --- | --- | --- |
| Titan | 0.2-1 Ga | 1.3-1.6 bar, CH4 3-6 % | unchanged (haze) | 90-97 K |
| Triton | < 0.1 Ga | N2 1e-5 to 2e-5 bar | ≥ 0.6 | 37-39 K |
| Europa | < 0.2 Ga | 1e-13 to 1e-10 bar | 0.55-0.70 | 90-105 K |
| Ganymede | old unit ≥ 3 Ga, renewed unit 40-80 %, younger | 1e-13 to 1e-10 bar | 0.35-0.50 | 100-112 K |
| Callisto | ≥ 3 Ga | 1e-13 to 1e-10 bar | 0.15-0.30 | 108-122 K |
| Enceladus | < 1 Ga | unchanged | 0.75-0.85 | 55-75 K |
| Io | < 0.002 Ga | SO2 1e-10 to 1e-8 bar, no N2/CO2 above it | as today | as today |
| Mercury, Mars | ≥ 3 Ga | unchanged | unchanged | unchanged |
| Venus, Earth | 0.3-0.8, 0.2-0.6 Ga | unchanged | unchanged | unchanged |

Every other archetype keeps its class and base look on every seed.

## Review Focus

1. A drawn icy moon (not an archetype) between the targets, for instance a Callisto-sized moon of a Saturn-like giant: its age, air and albedo must fall between those of the archetypes it resembles, never outside every target (no NaN, no albedo outside 0.05-0.95). Test in Task 8 (`drawn_icy_moons_stay_in_bounds`, 300 drawn systems).
2. A body at the boundary of a renewal process (air exactly `ERODING_BAR`, frost just appearing): the age must not jump by more than the scatter between two seeds that differ only there. Test in Task 2 (`liquid_cycle_age_is_continuous_in_pressure`).
3. A stored system of format 9 opened after the bump: refused as outdated like before (no panic on the missing `terrain` field in a world.json of format 27, which stays readable). Test in Task 10 (`a_format_27_world_reads_without_terrain`).
4. A rocky snowball or a frost-capped rocky planet must keep its albedo path: the icy-age law must not touch it. Test in Task 8 (`a_snowball_keeps_its_albedo`).
5. The card of a body with one terrain unit must not show an empty or a duplicated unit list. Test in Task 9 (vitest `surfaceUnits.test.ts`).

---

### Task 1: Terrain units in the model, today's rules unchanged

Refactor only: the model gains `terrain`, `params_for` reads it, every stored age stays bit-identical.

**Files:**

- Create: `crates/mapgen/src/planet/surface_age.rs`
- Modify: `crates/mapgen/src/planet/mod.rs` (module, `PlanetModel` field, build at :541 and in `vent`/`derive_moon_model` paths), `crates/mapgen/src/planet/interim.rs:617-631` (regime age), `crates/mapgen/examples/archetype_census.rs` (print units)

**Interfaces:**

- Produces:

  ```rust
  #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
  #[serde(rename_all = "kebab-case")] #[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
  pub enum Renewal { Formation, Plates, Episodic, HeatPipe, IceLid, LiquidCycle, VolatileIce, TidalEpisode }

  #[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)] #[cfg_attr(feature = "ts", ...)]
  pub struct TerrainUnit { pub age_gyr: f64, pub share: f64, pub renewal: Renewal }

  /// Units sorted by share, largest first; shares sum to 1.
  #[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)] #[cfg_attr(feature = "ts", ...)]
  pub struct Terrain { pub units: Vec<TerrainUnit> }
  impl Terrain { pub fn main(&self) -> Option<&TerrainUnit>; }

  /// Before the climate: the units an ice crust gets from heat and
  /// resonance history alone (IceLid, Formation, later TidalEpisode).
  pub fn icy_estimate(heat: &InternalHeat, radius_earth: f64, age_gyr: f64, tide: Option<&HostTide>) -> Terrain;
  /// After the climate: every unit of the body.
  pub fn terrain(model: &PlanetModel, age_gyr: f64) -> Terrain;
  ```

  `PlanetModel.terrain: Terrain` with `#[serde(default)]`.
- In this task `terrain` returns one unit: `Plates`/no surface → age = midpoint of `fallback::SURFACE_AGE_GA`; `Episodic`, `HeatPipe` → midpoint of their ranges; `Stagnant` → `IceLid` with `age_gyr * ice_lid_age_share(flux)` on an ice crust, else `Formation` with `age_gyr`. Move `ice_lid_age_share` and its constants from `interim.rs:197-210` into `surface_age.rs`.

- [ ] **Step 1: Write the failing tests** in `surface_age.rs`:
  - `one_unit_per_body_today`: for `io-like`, `europa-like`, `callisto-like`, `mars-like`, `earth-like` seeds 1-5 (use `system::draw` as `archetype_census` does), `terrain.units.len() == 1` and `shares` sum to 1 within 1e-12.
  - `stored_ages_do_not_move`: for all 21 archetypes, seeds 1-10, `generation_params(body).surface_age_ga` equals the value on `main`. Record the expected values first by running the census on `main` (99690d2) into a fixture `crates/mapgen/tests/fixtures/surface_ages_99690d2.json` (archetype, seed, f32 bits) and compare bits.

- [ ] **Step 2: Run** `RAYON_NUM_THREADS=4 cargo test -p mapgen --all-features surface_age -- --test-threads=3`. Expected: FAIL (module missing).
- [ ] **Step 3: Implement** the types, `terrain`, `icy_estimate` (one unit, `IceLid` at `age_gyr * ice_lid_age_share(flux)`, for now) and the field. In `params_for` the `Stagnant` arm becomes `main.age_gyr as f32 * STAGNANT_SURFACE_AGE_SHARE.pick(rng)`; the other arms keep their own draws (one draw, same position).
- [ ] **Step 4: Run** the filter, then the full suite. Expected: PASS, and `stored_ages_do_not_move` passes bit for bit.
- [ ] **Step 5: Commit** `refactor(planet): hold the surface's terrain units in the model`.

### Task 2: Dense air with a liquid cycle renews the surface (#491)

**Files:** `planet/surface_age.rs`, `planet/interim.rs` (regime arm), tests in `surface_age.rs` and `system.rs` (`a_titan_keeps_few_craters` at :5891, `an_old_cratered_titan_keeps_about_a_percent_of_liquid` at :7216: rename and re-bound to the new age).

**Interfaces:**

- Consumes: Task 1's `terrain`, `Renewal::LiquidCycle`.
- Produces: `pub const LIQUID_CYCLE_RETENTION_GYR: f64` and `fn liquid_cycle_age_gyr(pressure_bar: f64, liquid: SurfaceLiquid, age_gyr: f64) -> Option<f64>`.

Rule: a body with `gases.total() >= ERODING_BAR` (0.01 bar, `interim.rs:156`) and a surface liquid of `Water` or `Hydrocarbons` whose regime is not `Plates` (plates already renew faster) gets one `LiquidCycle` unit, age `min(age_gyr, LIQUID_CYCLE_RETENTION_GYR * ramp)`, where `ramp` rises smoothly from 1 at `10 * ERODING_BAR` to `age_gyr / LIQUID_CYCLE_RETENTION_GYR` at `ERODING_BAR` (log-pressure interpolation) so the age is continuous in pressure. `LIQUID_CYCLE_RETENTION_GYR = 1.0`: upper bound of Titan's crater retention age, Neish & Lorenz 2012 (doi:10.1016/j.pss.2011.02.016). `params_for` scatters it with a dedicated range `LIQUID_CYCLE_AGE_SHARE = Range([0.2, 1.0])` drawn in the regime draw's position.

- [ ] **Step 1: Failing tests:** `titan_is_young` (titan-like seeds 1-40: `surface_age_ga` in 0.2-1.0, unit `LiquidCycle`); `liquid_cycle_age_is_continuous_in_pressure` (call `liquid_cycle_age_gyr` at P = 0.0099…0.11 bar in 50 log steps: no step changes the age by more than 10 %); `mars_and_mercury_stay_old` (≥ 3 Ga, seeds 1-40).
- [ ] **Step 2: Run** them. Expected: `titan_is_young` FAIL (3.2-4.3 Ga today).
- [ ] **Step 3: Implement** the rule in `terrain`.
- [ ] **Step 4: Run** filter, the Titan tests in `system.rs` and `interim.rs` (`a_titan_has_its_lakes_near_the_poles`, `titans_seas_are_hydrocarbons`), then the full suite; then the census. Expected: PASS; Titan rows 0.2-1 Ga; no other archetype's age moves.
- [ ] **Step 5: Commit** `fix(planet): renew the surface of worlds with a liquid cycle` with `Closes: #491` (the Mercury comment of #491 is answered by `mars_and_mercury_stay_old`), docs updated.

### Task 3: Thin airs from supply and loss (exospheres)

Science task (opus). The spec leaves the loss law to the builder; this task decides it, within these limits.

**Files:** `planet/atmosphere.rs` (after `atmosphere()` at :288, before `cold_trap`), `planet/mod.rs` (`bare()` chain at :650-713), `planet/explain.rs` (`pressure_tree`: one new cause), tests in `atmosphere.rs`, `moon.rs`.

**Interfaces:**

- Produces: `pub fn non_thermal_loss(gases: Gases, body: &LossContext) -> (Gases, Gases)` returning `(kept, lost)`; `pub struct LossContext { pub gravity: f64, pub radius_m: f64, pub orbit_au: f64, pub age_gyr: f64, pub host: Option<HostPlasma> }`; `pub struct HostPlasma { pub host_mass_earth: f64, pub semi_major_axis_radii: f64 }`; `pub const EXOSPHERE_BAR: f64` (the stored pressure order of an exosphere, below `AIRLESS_BAR` 1e-4); `Quantity::Exosphere` is NOT added: the pressure tree uses a new note key `exosphere` and the existing `Quantity::Escape`.
- The law: an air whose column is lost faster than its supply over the body's age (photolysis and photo-ionisation at the body's distance, and for a moon inside its giant's magnetosphere, plasma sputtering) collapses to a supply-limited exosphere. Write the derivation and sources in the doc comment and `docs/science/planet-atmospheres.md`: candidate sources Johnson et al. 2004 (Galilean sputtering rates, in *Jupiter: The Planet, Satellites and Magnetosphere*), Carlson 1999 (Callisto CO2, doi:10.1126/science.283.5403.820). The host's plasma scales with the giant's mass and the moon's distance in host radii; a calibration constant is allowed if its target is stated (Callisto's CO2 column).
- Must hold: Titan (1.5 bar) and Triton (1.4e-5 bar N2 in frost equilibrium, resupplied by its frost) keep their air; thick airs are never touched (the loss over the age is far below the column).

- [ ] **Step 1: Failing tests:** `galilean_icy_moons_have_exospheres` (europa, ganymede, callisto seeds 1-20: `gases.total()` in 1e-13..1e-10); `titan_and_triton_keep_their_air` (existing bounds); `thick_airs_are_untouched` (earth-like, venus-like, mars-like: gases identical to before, compare with `planet_model` without the step).
- [ ] **Step 2: Run.** Expected: first FAIL.
- [ ] **Step 3: Implement** the law and call it in `bare()` after `atmosphere(...)`. A frost of a gas whose air was lost no longer counts in `Body::frost_bar` (`climate.rs:1235`).
- [ ] **Step 4: Run** filter, `moon.rs` tests (`ganymede_is_airless`, `callisto_is_airless`, `triton_keeps_a_thin_nitrogen_air`), full suite, census. Paste Europa/Ganymede/Callisto/Io/Titan/Triton rows.
- [ ] **Step 5: Commit** `feat(planet): reduce thin airs to exospheres by non-thermal loss`, docs updated.

### Task 4: Io's SO2 (#525)

**Files:** `planet/atmosphere.rs` (`SO2_FROST` beside `CO2_FROST` at :358-377, `outgassed` SO2 term at :211), `planet/climate.rs` (`Body::frost_bar`), tests in `atmosphere.rs`, `moon.rs`.

**Interfaces:**

- Consumes: Task 3's `non_thermal_loss` (which removes Io's N2 and CO2 in the plasma torus).
- Produces: `pub const SO2_FROST: Frost` (triple 197.64 K, 0.0167 bar; sublimation enthalpy and curve from Fray & Schmitt 2009, doi:10.1016/j.pss.2009.09.011 — check the triple-point values there and cite the table); `fn volcanic_so2_supply(heat: &InternalHeat, inputs: &PlanetInputs) -> f64` (supply from tidal heat and `ROCK_S`, no archetype input).
- Rule: a `HeatPipe` body with SO2 supply holds SO2 frost; its SO2 air is the frost's sublimation equilibrium at the frost temperature (dayside frost, cited); the CO2 frost no longer stands in for it.

- [ ] **Step 1: Failing tests:** `io_breathes_so2` (io-like seeds 1-40: `so2` in 1e-10..1e-8 bar, `so2 > n2 + co2`, frost kind SO2); `venus_keeps_its_so2` (venus-like SO2 unchanged).
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** filter, `io_is_a_heat_pipe`, `io_is_a_volcanic_world`, `io_keeps_its_lava`, full suite, census (Io rows).
- [ ] **Step 5: Commit** `feat(planet): give volcanic moons an SO2 frost and air` with `Closes: #525`.

### Task 5: Liquid nitrogen and Triton's N2 curve (#524, #529)

**Files:** `planet/climate.rs` (`SurfaceLiquid` at :22, `liquid()` at :804, `nitrogen_vapour_bar` at :715, a new emissivity in `surface_temperature_k`), `planet/atmosphere.rs` (`N2_FROST` at :373, `CONDENSING[1]`), `planet/classify.rs` (hydrocarbon class), web uses of `SurfaceLiquid` (`grep -rn "hydrocarbons'" web/src`), tests.

**Interfaces:**

- Produces: the decision on N2 liquid, recorded in the commit body. Default unless the code shows a reason against it: N2 joins the existing `Hydrocarbons` liquid as a solvent (Titan's seas already hold N2, `methane_freezes_k` reads `n2`), and a pure N2 liquid (no CH4) becomes `SurfaceLiquid::Nitrogen`; below the triple point N2 is frost. `pub const N2_ICE_EMISSIVITY: f64` with source.
- `N2_FROST.sublimation_j_mol` and triple values from Fray & Schmitt 2009; then the emissivity tuned with it so Triton reaches 37-39 K and N2 1e-5..2e-5 bar.

- [ ] **Step 1: Failing tests:** `triton_is_38_kelvin` (triton-like seeds 1-40: T 37-39 K, N2 1e-5..2e-5 bar); `condensed_nitrogen_keeps_the_hydrocarbon_class` (planet census seed 387 planet d: class `hydrocarbon-world`; reproduce it as `planet_census` does); `a_pure_nitrogen_sea_is_nitrogen` (a constructed `Gases` with N2 only at 70 K and 0.5 bar: `liquid() == Nitrogen`).
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement**, update ts types (`cargo test -p mapgen --features ts export_bindings` as the repo does; `grep -n ts-rs Cargo.toml` and README for the command), web switch statements and FR/EN liquid names.
- [ ] **Step 4: Run** filter, full suite, web checks, census (Titan and Triton rows).
- [ ] **Step 5: Commit** `fix(planet): treat condensed nitrogen as a liquid or a frost` with `Closes: #524` and `Closes: #529`.

### Task 6: Mobile volatile ice renews the surface (Triton)

**Files:** `planet/surface_age.rs`, tests.

**Interfaces:**

- Consumes: Task 5's N2 frost; `albedo::Cover.frost` share.
- Produces: `pub const VOLATILE_ICE_RETENTION_GYR: f64 = 0.1` (Schenk & Zahnle 2007, Icarus 192, 135, doi:10.1016/j.icarus.2007.07.004: Triton's surface ≲ 10-100 Ma); `Renewal::VolatileIce` unit whose share is the frost cover share of the body (the rest keeps its other unit), and `params_for` scatter `VOLATILE_ICE_AGE_SHARE = Range([0.1, 1.0])` in the regime draw's position when it is the main unit.
- Rule: a body whose air is in vapour equilibrium with an N2, CH4 or CO frost at the surface (the condensate of `condense()` is non-zero and `T` is below the gas's triple point).

- [ ] **Step 1: Failing tests:** `triton_is_young` (seeds 1-40: main unit `VolatileIce`, `surface_age_ga < 0.1`); `titan_is_not_volatile_ice` (Titan stays `LiquidCycle`).
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** filter, full suite, census.
- [ ] **Step 5: Commit** `feat(planet): renew surfaces under mobile volatile ice`.

### Task 7: A past tidal episode renews part of a resonant moon (Ganymede)

Science task (opus).

**Files:** `planet/surface_age.rs`, `planet/resonance.rs` (read: `MIGRATION_AGES`, `host_k2_over_q` at :243, `Resonance` at :328), `planet/moon.rs`, tests.

**Interfaces:**

- Consumes: the moon's resonance (`HostTide` in `Context.tide`, `Resonance`), `heat.rs` `tidal_power_w`.
- Produces: `Renewal::TidalEpisode` unit: `share` and `age_gyr` from a rule written in the doc comment with Showman & Malhotra 1997 (Icarus 127, 93: Ganymede's eccentricity pumped to about 0.07 during a passage through a Laplace-like resonance, heating it enough to resurface the grooved terrain). The episode's age comes from the moon's resonance capture under resonance locking (the same migration clock as `host_k2_over_q`); its share from the episode's heat flux through the existing `ice_lid_age_share` or a cited fracturing threshold. Applies to every moon in a resonance chain, never by name. Feeds `icy_estimate` too, so the albedo (Task 8) sees the mix.

- [ ] **Step 1: Failing tests:** `ganymede_has_grooved_terrain` (ganymede-like seeds 1-40: two units, the renewed one 0.4-0.8 of the surface and younger, the old one ≥ 3 Ga); `callisto_has_one_old_unit` (one unit ≥ 3 Ga); `europa_and_io_stay_young` (their main unit unchanged).
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** filter, full suite, census; also `ocean_census 20 300` to show how many drawn moons get an episode (paste the count in the commit body).
- [ ] **Step 5: Commit** `feat(planet): renew part of a resonant moon in a past tidal episode`.

### Task 8: Icy albedo from surface age

Science task (opus).

**Files:** `planet/albedo.rs` (`cover` at :317, `ICE_ALBEDO` at :70, `bond_albedo` at :383), `planet/climate.rs` (`Body::albedo_at` :1255, `derived_albedo` :1439), `planet/mod.rs` (pass the icy estimate into the climate), tests (`moon.rs:328` `a_moon_uses_its_own_cover` re-bounded with sources).

**Interfaces:**

- Consumes: `surface_age::icy_estimate` (Tasks 1, 7), a `Terrain` whose units' `(age_gyr, share)` it mixes, the rock share `1 - inputs.water_fraction`.
- Produces: `pub fn icy_unit_albedo(age_gyr: f64, rock_share: f64) -> f64` with a cited darkening law (fresh ice about 0.8-0.9, old dusty lag about 0.2; candidates: Spencer 1987 sublimation lag on Callisto, Moore et al. 1999 Icarus 140, 294, Howett et al. 2010 Icarus 206, 573 for Enceladus 0.81). Applies only to exposed water ice on an ice crust (`model_crust_kind == Ice`) under an air below `ERODING_BAR`; haze, volatile frost, rocky ice caps and snowballs keep their paths.
- First confirm and record in the commit body why Enceladus is at 85-89 K today (expected: it takes `REGOLITH_ALBEDO` 0.11).

- [ ] **Step 1: Failing tests:** `icy_moons_take_their_albedo` (seeds 1-40, the albedo and T targets of the Acceptance table for Enceladus, Europa, Ganymede, Callisto); `a_snowball_keeps_its_albedo` (earth-like frozen branch as `climate_sweep` builds it: albedo unchanged); `drawn_icy_moons_stay_in_bounds` (300 drawn systems: every icy moon albedo in 0.05-0.95, T finite, air ≤ `AIRLESS_BAR` unless it has frost or a dense air).
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** filter, full suite, census (all icy rows).
- [ ] **Step 5: Commit** `feat(planet): darken old ice surfaces`.

### Task 9: Methane residuals (#530)

**Files:** `planet/climate.rs` (`methane_depth` :185, `methane_freezes_k` :751), `planet/albedo.rs` (`haze` :357), `planet/atmosphere.rs` (`CONDENSING[0]` activity under the triple point), tests (`titan_comes_out_a_hydrocarbon_world` at `moon.rs:216`).

**Interfaces:**

- Produces: CH4 near-infrared stratospheric absorption (anti-greenhouse; McKay et al. 1991, Science 253, 1118, Titan's anti-greenhouse); activity 1 over CH4 ice (pure solid), `SEA_METHANE_SHARE` 0.5 over the sea only; ethane in the liquidus so cold methane-nitrogen-ethane liquids freeze near Richardson et al. 2019 (doi:10.1016/j.ijthermalsci.2018.12.024: 71.5 and 74.0 K where the model gives 81.3 and 85.0 K).

- [ ] **Step 1: Failing tests:** `titan_methane_is_three_to_six_percent` (seeds 1-40); `ethane_lowers_the_liquidus` (the two Richardson points within 2 K); `census_182_d` (planet census seed 182 planet d: record T and class, assert on the 150 K line as the code decides, value pasted in the commit).
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** filter, full suite, census (Titan, Triton).
- [ ] **Step 5: Commit** `fix(planet): settle the methane residuals after the cold trap` with `Closes: #530`.

### Task 10: Card, explain and formats

**Files:** `planet/explain.rs` (new `Quantity::SurfaceAge`, `Quantity::TerrainUnit`; notes `exosphere`, `liquid_cycle`, `volatile_ice`, `tidal_episode`), `web/src/lib/i18n/en.ts` and `fr.ts` (`causes.quantities`, `causes.units`, `causes.notes`, `cards`), `web/src/components/body/SurfaceCard.svelte:34` (age row gets a `CauseTree` and the unit list), `web/src/lib/format.ts:262` (`formatAtmosphere`: exosphere wording under `AIRLESS_BAR`, SO2 named; Io's frost reads SO2), the `BondAlbedo` node of `temperature_tree` (`explain.rs:523`) gets the surface-age units as causes when the icy law set it, `web/src/lib/causes.test.ts:15`, new `web/src/lib/surfaceUnits.ts` + `surfaceUnits.test.ts`, `raw_data.rs:109` + its doc list, `system.rs:82`, `web/src/lib/formatVersion.ts`, `crates/mapgen/tests/old_format.rs`.

**Interfaces:**

- Consumes: `PlanetModel.terrain` (Task 1), the `exosphere` note (Task 3).
- `explain` has no `WorldParams`: the surface-age tree shows the model's units (age, share, renewal) and the system age (`Quantity::Age`); the card's displayed age stays `meta.surface_age_ga`.
- `pub fn unitsFor(terrain: Terrain | undefined): Unit[]` in `surfaceUnits.ts`: empty for no terrain, one row per unit, merged when two units share renewal and age.

- [ ] **Step 1: Failing tests:** Rust `explain_has_a_surface_age_tree_for_ganymede` (two unit children, renewal keys), `a_format_27_world_reads_without_terrain` (old_format.rs: load the format-27 fixture or a world written by `main`, `terrain` defaults empty); vitest `surfaceUnits.test.ts` (no terrain → [], one unit → one row, Ganymede-like → two rows), `causes.test.ts` entries for the new quantities.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement**, bump the formats, regenerate ts bindings, EN and FR strings (FR uses the decimal comma per #488's helper if merged, else as today).
- [ ] **Step 4: Run** full Rust suite, `cd web && npm run check && npm test`, census.
- [ ] **Step 5: Commit** `feat(web)!: explain surface ages and exospheres on the card` with `BREAKING CHANGE: world format 28 and system format 10; stored systems must be regenerated.`

### Task 11: Acceptance pass and documentation sweep

- [ ] **Step 1:** Run the census at 40 seeds and `planet_census 500`; check every row of the Acceptance table; list any class drift in `planet_census` and explain it (new N2 liquid, SO2).
- [ ] **Step 2:** `docs/science/` sections cover every new constant (grep each constant name in docs); README paragraphs on surfaces and air updated; `archetype_census.rs` header documents its columns.
- [ ] **Step 3:** Full Rust suite and web checks once more.
- [ ] **Step 4: Commit** any doc fix as `docs(planet): …`.
