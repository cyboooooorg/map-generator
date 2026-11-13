# Physical Planet Model (Part 2) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Derive, for every planet the model of part 1 describes, its
envelope loss, tectonic regime, core and magnetic field, atmosphere,
surface climate, surface liquid and life, and let the classifier name
the surface classes (lava world, runaway greenhouse, hydrocarbon world,
ocean world).

**Architecture:** Still zero-dimensional and still not stored: the
derivations live in new modules of `crates/mapgen/src/planet/`, each a
pure function of the previous ones, chained in `planet_model` in the
order below. Worlds keep their presets; nothing outside `planet::` and
the `planet_census` example reads the result, so every generated world
stays byte-identical. Part 3 stores the result and drives the worlds
from it.

**Tech Stack:** Rust (crate `mapgen`), serde, rand.

**Spec:** `docs/superpowers/specs/2026-11-11-physical-planet-model-design.md`
("Derived, in order", steps 2 to 7). Part 1's plan,
`docs/superpowers/plans/2026-11-12-planet-model-part1.md`, lists the five
parts ("Parts of track A"); this is part 2.

**Prerequisite:** #433 (k₂/Q per body kind, branch
`fix/planet-tidal-k2q`) is merged into `main` and this branch is rebased
on it before Task 3, which reads the tidal heat.

## Order of the derivation

`planet_model(context, inputs)` becomes, in this order (one task each):

1. interior from the drawn inputs (part 1);
2. **XUV fluence** received over the system's age (Task 1);
3. **envelope escape**: photoevaporation strips or thins the envelope,
   and the interior is recomputed with what is left (Task 2);
4. internal heat on the final radius (part 1, with #433);
5. if an envelope is left (`envelope_fraction >= 1e-3`), the body has no
   surface: `surface` is `None` and the derivation stops here;
6. **runaway** check and **water kept** (Task 3);
7. **tectonic regime** (Task 3);
8. **core and dynamo** (Task 4);
9. **atmosphere**: outgassing minus escape (Task 5);
10. **climate and surface liquid**, with the carbonate thermostat
    (Task 6);
11. **life** and oxygen (Task 7);
12. **class** (Task 8).

The runaway check (6) reads only the insolation, the star and the water,
so it can come before the regime, which needs to know whether water
stays; nothing later feeds back into an earlier step.

## Global Constraints

- Scope: only what this plan lists. No change to `WorldParams`, presets,
  the grid climate, `geology::regime_of`, `carbon_cycle`, the server, the
  web app or any stored format. Part 3 wires the model into worlds.
- No format bump: `SYSTEM_FORMAT_VERSION` and `FORMAT_VERSION` stay. The
  new types derive `Serialize`/`Deserialize` like part 1's, no ts-rs.
- Anti-goal "types in disguise": no function of `planet::` takes a type
  name (`body_type`, preset name, `PLANET_TYPES`). Inputs come from
  `planet::Context`, `PlanetInputs` and earlier derived values only.
- Every relation cites its source in its doc comment (authors, year, doi
  or equation), as part 1 does; every constant without a source says
  `Calibration, no source.` and names the bodies it reproduces. The
  values below are from memory: check each against its source and fix
  the value, not the test tolerance, when the source differs. Where a
  task says "calibrate", the targets are the test; the form of the
  relation is the builder's within the cited sources.
- Determinism: the only new draw (`life_draw`, Task 7) is appended as the
  last draw of `draw_inputs`, so no earlier input moves.
  `MAIN_SIGNATURES` (`system.rs`) must stay unchanged, and
  `body_model_is_deterministic_and_independent` keeps passing.
- Worlds unchanged: no generator code reads `planet::`. Before the PR,
  `world-coherence-checker` confirms byte-identical worlds against
  `main` on a few seeds.
- Units: SI inside the functions (kg, m, W, K, Pa converted to bar at the
  boundary); pressures stored in bar, temperatures in K, fields in µT.
  Constants `G`, `EARTH_KG`, `EARTH_M` are reused from `exomoon.rs:42-45`
  (`pub(crate)`), not duplicated.
- Work in the worktree `~/Developer/mg-planet-model-p2`, branch
  `feat/planet-model-part2`. Never write to the main checkout's
  `systems/`, `worlds/`, `db/`.
- Shared machine: `RAYON_NUM_THREADS=4 cargo test -j 4 --workspace --all-features -- --test-threads=3`;
  `cargo fmt --check`;
  `cargo clippy -j 4 --workspace --all-targets --all-features -- -D warnings`.
  Never `npm ci` in a worktree; never port 3000.
- Docs in the same commit as the behaviour: a new
  `docs/science/planet-atmospheres.md` (regime, core and dynamo,
  atmosphere, climate, liquid, life, surface classes, each relation with
  its source and its calibration targets), linked from
  `docs/science/planet-interiors.md`; the README's planet model section
  (find it with `grep -n 'planet model\|planet_census' README.md`, read
  only that range) gets one paragraph. Commits follow Conventional
  Commits; the PR says `Refs #423`.

## Review Focus

1. A body that loses all its atmosphere (Mercury, the Moon, a close
   small planet) must come out with pressure 0, no liquid, the
   equilibrium temperature, and no NaN anywhere: Task 5,
   `airless_bodies_have_no_pressure`, and Task 6,
   `an_airless_body_sits_at_its_equilibrium_temperature`.
2. Extreme inputs (0.01 M⊕, 20 M⊕ rocky, insolation 1e-3 and 1e4, age
   0.5 and 13 Gyr, M dwarf and A star) must give finite, non-negative
   values for every derived field: Task 8,
   `every_derived_value_is_finite_on_extreme_inputs`.
3. The carbonate thermostat must not invent CO₂: the regulated pressure
   never exceeds what was outgassed, and a planet too far out to be
   warmed by all its CO₂ freezes instead: Task 6,
   `the_thermostat_never_exceeds_the_outgassed_co2`.
4. A hot Jupiter must keep its envelope, and a mini-Neptune at a
   temperate orbit must keep its own: escape strips only what the
   energy allows: Task 2, `giants_and_temperate_mini_neptunes_keep_their_envelope`.
5. A world with no water inventory never gets surface water, life or the
   ocean world class, whatever its temperature: Task 6,
   `no_water_no_ocean`, and Task 7, `no_water_no_life`.

---

### Task 1: The XUV fluence

**Files:**
- Create: `crates/mapgen/src/planet/xuv.rs`
- Modify: `crates/mapgen/src/planet/inputs.rs` (`Context` gains
  `teff_k: f64`, `luminosity: f64`, read by `Context::of`)
- Modify: `crates/mapgen/src/planet/mod.rs` (module declaration)

**Interfaces:**
- Produces: `pub fn xuv_fluence(context: &Context) -> f64`: XUV energy
  received per unit area over the system's age, relative to Earth's
  (Earth around the Sun at 4.567 Gyr = 1.0). `pub const EARTH_XUV_FLUENCE_J_M2: f64`
  (Earth's absolute value, for Task 2's energy budget).

The star's XUV luminosity is saturated at `L_XUV/L_bol ≈ 10^-3` for a
time that grows as the star's mass falls, then decays as a power of age
(Ribas et al. 2005, doi:10.1086/427977, `t^-1.23` for the Sun; saturation
time by mass from Jackson, Davis & Wheatley 2012,
doi:10.1111/j.1365-2966.2012.21384.x, or Johnstone et al. 2021,
doi:10.1051/0004-6361/202038407). Integrate over `[0, age_gyr]`
analytically, times the insolation. `activity.rs` keeps its own model
for the star's current activity; do not change it.

- [ ] **Step 1: Write the failing tests** in `xuv.rs`:
  `earth_receives_one` (`xuv_fluence` of the Sun at 1 AU, 4.567 Gyr in
  `[0.99, 1.01]`); `fluence_falls_as_the_square_of_distance` (at 2 AU,
  0.25 within 1 %); `older_systems_received_more_but_not_much_more` (5 vs
  10 Gyr: ratio in `(1.0, 1.3)`, since most XUV comes early);
  `m_dwarfs_saturate_longer` (a 0.2 M☉ star at the orbit giving Earth's
  insolation: fluence above 10); `the_fluence_is_finite_at_every_age`
  (age 0.01 and 13 Gyr, any star of `StarInfo` codes: finite, positive).
- [ ] **Step 2: Run** `cargo test -p mapgen planet::xuv` — FAIL (no module).
- [ ] **Step 3: Implement** `xuv_fluence` and the `Context` fields.
- [ ] **Step 4: Run** the tests and the part 1 tests (`cargo test -p mapgen planet::`) — PASS.
- [ ] **Step 5: Commit** `feat(planet): integrate the star's XUV over the system's age`.

### Task 2: Envelope escape

**Files:**
- Create: `crates/mapgen/src/planet/escape.rs`
- Modify: `crates/mapgen/src/planet/mod.rs`

**Interfaces:**
- Consumes: `xuv_fluence`, `EARTH_XUV_FLUENCE_J_M2` (Task 1);
  `interior(&PlanetInputs, &Exposure) -> Interior` (part 1).
- Produces: `pub fn energy_limited_loss_kg(interior: &Interior, mass_earth: f64, fluence_rel: f64) -> f64`
  (mass a body can lose over its life, reused by Task 3 for water);
  `pub fn envelope_after_escape(inputs: &PlanetInputs, interior: &Interior, fluence_rel: f64) -> PlanetInputs`
  (inputs with `envelope_fraction` reduced, `0.0` once under `1e-3`, and
  `mass_earth` lowered by the lost envelope).

Energy-limited escape: `ΔM = η π R³ F / (G M)` with `F` the fluence in
J/m² and `R` the envelope's radius (Owen & Wu 2017,
doi:10.3847/1538-4357/aa890a, efficiency `η ≈ 0.1`). It is what opens the
radius valley: no tuning to the valley's position, only the measurement
in Task 8.

- [ ] **Step 1: Write the failing tests**:
  `a_close_mini_neptune_is_stripped_to_its_core` (5 M⊕, envelope 0.01,
  0.05 AU around the Sun, 5 Gyr: envelope 0, radius below 1.8 R⊕ after
  the recomputed interior);
  `giants_and_temperate_mini_neptunes_keep_their_envelope` (Jupiter's
  inputs at 0.05 AU keep at least 99 % of their envelope; 5 M⊕ with 0.01
  at 0.5 AU keeps at least half);
  `rocky_bodies_are_untouched` (Earth's inputs: unchanged);
  `loss_never_exceeds_the_envelope` (any fluence up to 1e6: envelope in
  `[0, initial]`, mass never below the core's).
- [ ] **Step 2: Run** `cargo test -p mapgen planet::escape` — FAIL.
- [ ] **Step 3: Implement** both functions, and in `planet_model` apply
  `envelope_after_escape` then recompute the interior before the heat.
  `PlanetModel` gains `xuv_fluence: f64` and `envelope_lost: f64`
  (fraction of the initial envelope lost).
- [ ] **Step 4: Run** `cargo test -p mapgen planet::` — PASS. Part 1's
  `the_solar_system_classifies_as_itself` must still pass (escape must
  not strip Uranus or Neptune).
- [ ] **Step 5: Commit** `feat(planet): strip envelopes by energy-limited escape`.

### Task 3: Runaway, water and tectonic regime

**Files:**
- Create: `crates/mapgen/src/planet/regime.rs`
- Modify: `crates/mapgen/src/planet/mod.rs`

**Interfaces:**
- Consumes: `star::habitable_fluxes(teff_k: f32) -> (f32, f32)`
  (`star.rs:288`, runaway limit first); `energy_limited_loss_kg`
  (Task 2); `InternalHeat::flux_w_m2(radius_earth)`;
  `geology::Regime` (reused, not copied).
- Produces:
  `pub struct Water { pub runaway: bool, pub kept_kg: f64, pub oceans: f64 }`
  (`oceans` in Earth oceans, 1.4e21 kg each);
  `pub fn water(context: &Context, inputs: &PlanetInputs, interior: &Interior, fluence_rel: f64) -> Water`;
  `pub fn regime(inputs: &PlanetInputs, heat: &InternalHeat, interior: &Interior, water: &Water) -> Regime`.

`water`: a runaway greenhouse when the insolation exceeds the runaway
limit of `habitable_fluxes` (Kopparapu et al. 2013; the mass correction
of Kopparapu et al. 2014, doi:10.1088/2041-8205/787/2/L29, if checked);
in a runaway the water is in the air and loses its hydrogen at the
energy-limited rate, so `kept_kg = max(0, water − 9 × H lost)`.

`regime`, a labelled calibration (the spec says the literature disagrees
on the mass dependence):
- heat-pipe when the internal heat flux is above `HEAT_PIPE_FLUX_W_M2`
  (Io's ~2 W/m², Moore & Webb 2013, doi:10.1038/nature12424, against
  Earth's 0.087);
- plates when the mass is at least 0.3 M⊕ (`geology::PLATES_MIN_MASS_EARTH`,
  reused) and water is kept;
- episodic when the mass allows plates but the water is gone (Venus);
- stagnant otherwise (Mars, Mercury, the Moon).

- [ ] **Step 1: Write the failing tests**: `venus_ran_away_and_dried`
  (Venus's inputs and orbit: `runaway`, `kept_kg` 0);
  `earth_keeps_its_ocean` (oceans in `[0.9, 1.1]`, not runaway);
  `the_solar_system_has_its_regimes` (Earth plates, Venus episodic, Mars
  and Mercury stagnant, Io's inputs with Io's tidal heat of ~1e14 W
  heat-pipe); `a_dry_super_earth_is_not_plated` (5 M⊕, water 0, outside
  the runaway: not plates).
- [ ] **Step 2: Run** `cargo test -p mapgen planet::regime` — FAIL.
- [ ] **Step 3: Implement** `water` and `regime`.
- [ ] **Step 4: Run** — PASS.
- [ ] **Step 5: Commit** `feat(planet): derive the runaway, the water kept and the tectonic regime`.

### Task 4: Core and dynamo

**Files:**
- Create: `crates/mapgen/src/planet/core.rs`
- Modify: `crates/mapgen/src/planet/mod.rs`

**Interfaces:**
- Consumes: `Interior` layers (the `Material::Iron` layer), `Regime`
  (Task 3), `Context::age_gyr`.
- Produces:
  `pub enum CoreState { None, Liquid, Frozen }` (kebab-case serde);
  `pub struct Core { pub state: CoreState, pub dynamo: bool, pub field_ut: f64 }`
  (`field_ut`: surface equatorial dipole field, µT; 0 without a dynamo);
  `pub fn core(inputs: &PlanetInputs, interior: &Interior, regime: Regime, age_gyr: f64) -> Core`.
  Read by Task 5 and, after part 3, by #345.

A parametrised thermal history in the style of Driscoll & Barnes 2015
(doi:10.1089/ast.2015.1325): a core stays liquid and convecting while the
mantle draws heat from it, which plates do efficiently and a lid does
not; a small core freezes sooner. Calibrate a dynamo lifetime from the
core's mass and the regime so that the targets hold. The field strength
from the energy-flux scaling of Christensen 2010 (or Christensen &
Aubert 2006, doi:10.1111/j.1365-246X.2006.03009.x), taken to the surface
by `(r_core / R)³`.

- [ ] **Step 1: Write the failing tests**:
  `the_solar_system_has_its_dynamos` (Earth: dynamo, field in
  `[20, 70]` µT; Mercury: dynamo, field under 2 µT; Mars at 4.567 Gyr: no
  dynamo; Mars at 0.3 Gyr: dynamo; Venus: no dynamo; the Moon: no
  dynamo); `no_iron_no_core` (CMF 0: `CoreState::None`, field 0);
  `cores_freeze_with_age` (for a fixed body, once frozen at age `t` it
  stays frozen at every later age).
- [ ] **Step 2: Run** `cargo test -p mapgen planet::core` — FAIL.
- [ ] **Step 3: Implement** `core`. Mercury's weak dynamo may need the
  core's sulphur (`Volatiles::s`) to keep its melting point low: note in
  the doc comment whichever lever the calibration uses.
- [ ] **Step 4: Run** — PASS.
- [ ] **Step 5: Commit** `feat(planet): derive the core's state and the dynamo's field`.

### Task 5: Atmosphere

**Files:**
- Create: `crates/mapgen/src/planet/atmosphere.rs`
- Modify: `crates/mapgen/src/planet/mod.rs`

**Interfaces:**
- Consumes: `Volatiles`, `Water` and `Regime` (Task 3), `Core` (Task 4),
  `xuv_fluence` (Task 1), `Interior`.
- Produces:
  `pub struct Gases { pub n2: f64, pub co2: f64, pub h2o: f64, pub ch4: f64, pub nh3: f64, pub so2: f64, pub o2: f64 }`
  (partial pressures, bar; `total()`);
  `pub fn escape_velocity_km_s(interior: &Interior, mass_earth: f64) -> f64`;
  `pub fn outgassed(inputs: &PlanetInputs, water: &Water, regime: Regime, interior: &Interior) -> Gases`;
  `pub fn atmosphere(outgassed: Gases, interior: &Interior, mass_earth: f64, core: &Core, fluence_rel: f64) -> Gases`.

`outgassed`: each volatile's inventory times an outgassed share by regime
(plates, heat-pipe and episodic high, stagnant lower), turned into a
surface pressure by `P = m g / (4π R²)`. In a runaway the water stays as
vapour only if `kept_kg > 0`. `atmosphere`: the cosmic shoreline
(Zahnle & Catling 2017, doi:10.3847/2041-8213/aa7846, `I_XUV ∝ v_esc⁴`)
sets the share kept as a smooth function of how far the body sits above
or below the line; without a dynamo the line moves by
`UNMAGNETISED_ESCAPE` (calibration, no source; ties to #344), small
enough that Venus keeps its air. Calibrate the outgassed shares and the
line on the targets.

- [ ] **Step 1: Write the failing tests**: `venus_has_ninety_bars_of_co2`
  (total in `[60, 120]` bar, CO₂ share above 0.9, N₂ in `[1, 6]` bar);
  `earth_has_a_bar_of_nitrogen` (N₂ in `[0.5, 1.2]` bar, before Task 6's
  thermostat removes the CO₂); `mars_keeps_a_trace`
  (total in `[0.002, 0.05]` bar); `airless_bodies_have_no_pressure`
  (Mercury and the Moon: total below `AIRLESS_BAR`,
  `atmosphere.rs:25`, 1e-4); `titan_keeps_a_thick_nitrogen_atmosphere`
  (Titan's inputs with ice volatiles at 9.58 AU: total in `[0.5, 5]`
  bar); `escape_velocity_of_earth_is_eleven_km_s` (`[11.0, 11.4]`).
- [ ] **Step 2: Run** `cargo test -p mapgen planet::atmosphere` — FAIL.
- [ ] **Step 3: Implement** the three functions. The reference bodies'
  volatiles come from `inputs.rs`'s volatile rule without scatter: add a
  `#[cfg(test)]` helper `reference::model_inputs(body: &ReferenceBody) -> PlanetInputs`
  that fills them (and `life_draw: 0.0` once Task 7 adds it), and use it
  in every later Solar System test.
- [ ] **Step 4: Run** — PASS.
- [ ] **Step 5: Commit** `feat(planet): derive the atmosphere from outgassing and escape`.

### Task 6: Climate and surface liquid

**Files:**
- Create: `crates/mapgen/src/planet/climate.rs`
- Modify: `crates/mapgen/src/planet/mod.rs`

**Interfaces:**
- Consumes: `Gases` (Task 5), `Water`, `Regime` (Task 3),
  `InternalHeat::flux_w_m2`, `Context::equilibrium_k` (Bond albedo 0.3,
  `star.rs:319`).
- Produces:
  `pub enum SurfaceLiquid { None, Water, Hydrocarbons, Lava }` (kebab-case);
  `pub struct Climate { pub temperature_k: f64, pub liquid: SurfaceLiquid, pub regulated: bool }`;
  `pub fn surface_temperature_k(context: &Context, gases: &Gases, heat_flux_w_m2: f64) -> f64`;
  `pub fn climate(context: &Context, gases: Gases, water: &Water, regime: Regime, heat_flux_w_m2: f64) -> (Gases, Climate)`
  (returns the gases with the thermostat's CO₂).

`surface_temperature_k`: a grey atmosphere, `T⁴ = T_eq⁴ (1 + ¾ τ) + q/σ`,
with `τ` from the partial pressures of CO₂, H₂O and CH₄ and pressure
broadening by the total; calibrate `τ` on the targets (labelled
calibration; cite the form, e.g. Pierrehumbert 2010, *Principles of
Planetary Climate*, ch. 4). Water vapour follows the temperature where
there is liquid water (saturation, `climate.rs:1010`'s
`saturation_vapour_hpa` may be reused).

The thermostat (Walker, Hays & Kasting 1981, doi:10.1029/JC086iC10p09776):
with plates and liquid water, the CO₂ is set by bisection so that the
temperature is `THERMOSTAT_K` (calibration, Earth's 288 K), clamped to
`[0, outgassed CO₂]`. The grid's `carbon_cycle` (`generation.rs:405`)
stays as it is; part 3 aligns them.

The liquid, from the phase diagram at the surface temperature and
pressure: water between 273 K and its boiling point at that pressure
(Clausius–Clapeyron), above the triple point (0.006 bar) and with water
kept; hydrocarbons between methane's triple point (90.7 K) and ethane's
boiling point at that pressure, with CH₄ in the air; lava above
`LAVA_K` (basalt solidus, ~1400 K; cite e.g. Kite, Fegley, Schaefer &
Gaidos 2016, doi:10.3847/0004-637X/828/2/80); otherwise none.

- [ ] **Step 1: Write the failing tests**: `the_solar_system_has_its_climates`
  (Earth 288 ± 6 K with water and `regulated`; Venus 737 ± 50 K, no
  liquid; Mars 210 ± 12 K, no liquid; Titan 94 ± 8 K with hydrocarbons);
  `an_airless_body_sits_at_its_equilibrium_temperature` (Mercury: within
  1 K of `T_eq`, plus internal heat that is negligible);
  `the_thermostat_never_exceeds_the_outgassed_co2` (Earth's inputs at
  1.6 AU: CO₂ at most the outgassed amount, and if that is not enough the
  water freezes: liquid `None`); `no_water_no_ocean` (Earth with water 0:
  liquid not `Water`); `a_close_rock_is_a_lava_world` (Earth's inputs at
  0.01 AU: `Lava`).
- [ ] **Step 2: Run** `cargo test -p mapgen planet::climate` — FAIL.
- [ ] **Step 3: Implement** the three functions.
- [ ] **Step 4: Run** — PASS.
- [ ] **Step 5: Commit** `feat(planet): derive the surface climate and liquid`.

### Task 7: Life

**Files:**
- Create: `crates/mapgen/src/planet/life.rs`
- Modify: `crates/mapgen/src/planet/inputs.rs` (`PlanetInputs` gains
  `life_draw: f64`, uniform in `[0, 1)`, drawn last in `draw_inputs`)
- Modify: `crates/mapgen/src/planet/mod.rs`, `reference.rs` helpers

**Interfaces:**
- Consumes: `Climate` (Task 6), `Gases`, `Context::age_gyr`.
- Produces: `pub struct Life { pub arose: bool, pub oxygenated: bool }`;
  `pub fn life(inputs: &PlanetInputs, climate: &Climate, age_gyr: f64) -> Life`;
  `pub fn oxygenate(gases: Gases, life: &Life) -> Gases`.

Life is possible when surface water is liquid now and the system is at
least `LIFE_MIN_GYR` old (1.0, calibration, no source: the spec's "order
of a billion years"); it arises when `life_draw < LIFE_CHANCE` (1/3, a
game choice set with the user on 2026-11-12, documented as such: no
measured rate exists). It
oxygenates the air after `OXYGEN_AFTER_GYR` (about 2.2 Gyr after
formation, Earth's Great Oxidation Event, Lyons, Reinhard & Planavsky
2014, doi:10.1038/nature13068), with O₂ at Earth's ratio to N₂ (0.27,
calibration on Earth) and CH₄ removed.

- [ ] **Step 1: Write the failing tests**: `earth_has_oxygenic_life`
  (Earth with `life_draw` 0: arose, oxygenated, O₂ in `[0.15, 0.3]`
  bar); `a_young_earth_is_lifeless` (age 0.5 Gyr: not arose);
  `a_habitable_world_can_be_lifeless` (`life_draw` 0.99: not arose);
  `no_water_no_life` (Venus and Mars: not arose, whatever the draw);
  `life_draw_is_the_last_draw` (the inputs drawn with the same RNG keep
  every part 1 field equal to what part 1 drew: compare against values
  captured from `main` in the test).
- [ ] **Step 2: Run** `cargo test -p mapgen planet::life` — FAIL.
- [ ] **Step 3: Implement**, and add `life_draw` to every `PlanetInputs`
  literal (tests, `reference.rs` helpers: 0.0).
- [ ] **Step 4: Run** `cargo test -p mapgen planet::` — PASS.
- [ ] **Step 5: Commit** `feat(planet): draw life where water lasted`.

### Task 8: The chain, the surface classes and the census

**Files:**
- Modify: `crates/mapgen/src/planet/mod.rs` (`planet_model` chains the
  tasks; `PlanetModel` gains `surface: Option<Surface>`)
- Modify: `crates/mapgen/src/planet/classify.rs`
- Modify: `crates/mapgen/examples/planet_census.rs`
- Create: `docs/science/planet-atmospheres.md`; modify
  `docs/science/planet-interiors.md`, `README.md` (one paragraph)

**Interfaces:**
- Consumes: everything above.
- Produces:
  `pub struct Surface { pub water: Water, pub regime: Regime, pub core: Core, pub gases: Gases, pub climate: Climate, pub life: Life }`;
  `PlanetModel { inputs, interior, heat, xuv_fluence, envelope_lost, surface: Option<Surface>, class }`
  (`inputs` after escape); `PlanetClass` gains `LavaWorld`,
  `RunawayGreenhouse`, `HydrocarbonWorld`, `OceanWorld` (kebab-case);
  `classify(inputs: &PlanetInputs, interior: &Interior, period_days: f64, surface: Option<&Surface>) -> PlanetClass`.

Classifier order, first match wins: giant rules and mini-Neptune as in
part 1; then lava world (liquid `Lava`); runaway greenhouse (`runaway`
and total pressure above 1 bar); hydrocarbon world (liquid
`Hydrocarbons`); ocean world (liquid `Water` and at least
`OCEAN_WORLD_OCEANS` = 10 Earth oceans: calibration, citing Cowan &
Abbot 2014, doi:10.1088/0004-637X/781/1/27, on how much water drowns all
land); then ice world, super-Earth, rocky as in part 1.

- [ ] **Step 1: Write the failing tests**:
  `the_solar_system_classifies_as_itself` updated (Venus
  `runaway-greenhouse`, Earth `rocky`, Mars `rocky`, giants unchanged;
  Titan `hydrocarbon-world` through `planet_model` with its reference
  context); `a_migrated_water_world_in_the_habitable_zone_is_an_ocean_world`
  (1 M⊕, water 0.3, 1 AU: `ocean-world`);
  `enveloped_bodies_have_no_surface` (Neptune: `surface` is `None`);
  `every_derived_value_is_finite_on_extreme_inputs` (mass 0.01 and 20,
  insolation 1e-3 and 1e4, age 0.5 and 13, stars M5V and A0V: every f64
  of the model finite and non-negative);
  `keys_are_kebab_case_as_serialized` extended to the new classes.
- [ ] **Step 2: Run** `cargo test -p mapgen planet::` — FAIL.
- [ ] **Step 3: Implement** the chain and the classes.
- [ ] **Step 4: Run** the whole suite, fmt and clippy (Global
  Constraints) — PASS; `MAIN_SIGNATURES` unchanged.
- [ ] **Step 5: Extend the census** to print the class mix, the liquid
  mix, the share with life, the regime mix, and a radius histogram (0.25
  R⊕ bins) of planets with periods under 100 days. Run
  `cargo run --release -p mapgen --example planet_census -- 500` and
  record the output in `docs/science/planet-atmospheres.md`. Expected,
  measured not tuned: a dip between 1.5 and 2 R⊕ against the bins on
  either side (Fulton et al. 2017, doi:10.3847/1538-3881/aa80eb). If
  there is none, say so in the doc and in the PR; do not tune for it.
  Mini-Neptunes should fall from part 1's over-representation.
- [ ] **Step 6: Write the docs** (Global Constraints), then commit
  `feat(planet): chain the surface model and name the surface classes`.
- [ ] **Step 7: Check worlds are unchanged** with `world-coherence-checker`
  (this branch's binary against `main`'s, a few seeds: byte-identical).
