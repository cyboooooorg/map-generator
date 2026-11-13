# Physical Planet Model (Part 3a) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make every planet of a drawn system what the model of parts 1
and 2 says it is: its mass comes from a mass function, not a drawn
type; its stored type is the derived class; its world is generated from
the derived state (size, air, sky, sea, regime, climate, life) through
one transitional function.

**Architecture:** The system draw draws placement slots and masses with
no type, runs `planet_model` on each planet once its id is known, and
stores the model on the body (`system.json`). A new module
`planet::interim` turns a model into the `Preset` and the `WorldParams`
the generator reads today, so the generator, the server and the web keep
their current inputs; #424 deletes that module. Requests and the form
still speak today's type names (`--system terran=1`), read as placement
hints only; part 3b replaces them with archetypes.

**Tech Stack:** Rust (crates `mapgen`, `server`, `cli`), serde, ts-rs,
rand; Svelte and TypeScript (`web/`).

**Spec:** `docs/superpowers/specs/2026-11-11-physical-planet-model-design.md`
("Inputs", "Giants", "One global state, one grid", "Interim surface and
sky", "Storage and formats", "Order of work" step 3). Earlier plans:
`docs/superpowers/plans/2026-11-12-planet-model-part1.md` and
`…-part2.md`.

## Parts of step 3

The user split the spec's step 3 into three plans (2026-11-13):

- **3a, this plan.** Drawn systems from the formation context, the model
  stored and driving the worlds through `planet::interim`, the type is
  the class, `SYSTEM_FORMAT_VERSION` 3 and `FORMAT_VERSION` 17, older
  systems listed as outdated.
- **3b.** Archetypes replace presets (`--planet`, `--system`, the
  catalog, `/api/presets`, lone worlds drawn from the context with no
  name); `WorldParams` stops embedding a `Preset` and carries the inputs
  and the derived state; the web reads that state instead of
  `meta.preset.*`; the direct knobs (`sea_level`, `land_fraction`,
  `volcanic`, `surface_age_ga`, `erosion`, `pressure_bar`,
  `circumference_km`) leave the request. Bumps the formats again.
- **3c.** The relief issues the user folded into step 3: #312 (crater
  transition from composition), #313 (impact flux and chronology for
  Mercury, Venus, Earth) and #315 (isostasy: crustal thickness and the
  platform at the waterline). They read the stored composition and
  regime, so they come after 3b. The draft PRs #357 and #358 are empty
  placeholders; 3c takes over their branches and leaves #311, #316 and
  #317 to them.

## Global Constraints

- Scope: only what this plan lists. No archetype, no change to
  `WorldRequest`'s fields, `/api/presets`, `FormSpec.planet_types`, the
  CLI's `--planet` or `--system` vocabulary. Moons keep `rocky-moon` and
  `icy-moon` and have no model (part 4). A world asked for alone
  (`SystemRequest::Around`) keeps its named preset; its drawn neighbours
  go through the model.
- Formats: `SYSTEM_FORMAT_VERSION` 2 → 3 (`system.rs:58`);
  `FORMAT_VERSION` and `MIN_FORMAT_VERSION` 16 → 17
  (`export/raw_data.rs:52,60`), with `web/src/lib/formatVersion.ts:6` in
  the same commit. No open PR bumps either (checked 2026-11-13); confirm
  with the peer sessions before merging (CLAUDE.md).
- No legacy path: older systems are refused as today, listed as
  outdated, never migrated, never deleted by the server
  (spec "Storage and formats").
- Anti-goal "types in disguise": nothing in `planet::` reads a type
  name; `planet::interim` reads the model only and is the one place that
  maps a model onto today's presets.
- Every relation cites its source in its doc comment; every constant
  without one says `Calibration, no source.` and names what it
  reproduces. Values below are from memory: check them, fix the value
  (not the tolerance) when the source differs.
- Determinism: the model keeps its own stream,
  `body_seed(system.seed, "<id>/model")`; the mass is drawn on the
  system stream where `draw_mass` is drawn today. A drawn system is the
  same at every run; removing a body leaves the others' models as they
  were.
- Shared machine: work in `~/Developer/mg-planet-model-p3`, branch
  `feat/planet-model-part3a`. Tests:
  `RAYON_NUM_THREADS=4 cargo test -j 4 --workspace --all-features -- --test-threads=3`,
  clippy `cargo clippy -j 4 --workspace --all-targets --all-features -- -D warnings`,
  web `cd web && npm run check && npx vitest run`. Never `npm ci` in the
  worktree, never port 3000, never write to the main checkout's
  `systems/`, `worlds/`, `db/`.
- ts-rs bindings regenerate on `cargo test`; commit
  `web/src/lib/api/types/` with the Rust change that alters them (CI runs
  `git diff --exit-code` on it).

## Review Focus

1. **A small slot that comes out a giant.** A slot drawn at 15 M⊕ can
   keep an envelope and classify as `mini-neptune` or `ice-giant`. Its
   world must take the giant path, with a radius between 2 and 4 R⊕ for
   a mini-Neptune (Task 5 test `a_mini_neptune_generates_a_giant_world`).
2. **Temperatures past the grid's range.** A lava world's model can sit
   above 1500 K while the grid clamps at 600 °C
   (`TEMPERATURE_RANGE`, `climate.rs:14`). The offset must aim at the
   clamped target and the world still generate (Task 4 test
   `a_lava_world_aims_at_the_grid_ceiling`).
3. **Dry and drowned extremes.** No water gives no sea at all; more than
   the relief can hold gives no land, without the bisection leaving its
   bounds (Task 3 tests `no_water_leaves_no_sea`,
   `a_deep_ocean_drowns_every_summit`).
4. **Old systems on disk.** A `system.json` of version 2 is listed with
   `outdated: true` and can be deleted from the list; nothing deletes it
   unasked, and opening it still names `mise run clean-systems` (Task 6
   tests).
5. **Requested types still work.** `--system terran=1,gas-giant=1`
   places a planet in each band as today; the stored type is whatever
   the model derives, and a requested giant that the model does not make
   a giant is still a valid world (Task 2 test
   `requested_slots_store_the_derived_class`).

---

### Task 1: Mass function and typeless slots

**Files:**
- Create: `crates/mapgen/src/planet/mass.rs`
- Modify: `crates/mapgen/src/planet/mod.rs` (re-exports),
  `crates/mapgen/src/system.rs:430-700` (`Slot`, `draw_mass`,
  `drawn_types`, `drawn_hot_jupiter`, `is_giant`, `clear`, `fits`),
  `:705-910` (`draw_whole`, `draw_around`)
- Test: `crates/mapgen/src/planet/mass.rs`, `crates/mapgen/src/system.rs`

**Interfaces:**
- Produces:
  - `pub enum Origin { Small, GasGiant, IceGiant, HotJupiter }` in
    `planet::mass`, with `pub fn is_giant(self) -> bool` (all but
    `Small`): where a slot was drawn, used for spacing and eccentricity
    only, never stored.
  - `pub fn draw_mass(origin: Origin, rng: &mut impl RngExt) -> f64`,
    Earth masses.
  - In `system.rs`: `enum Placement { Drawn(Origin), Requested(String) }`
    replacing `Slot.body_type`; `Placement::is_giant(&self) -> bool`
    (`Requested` keeps today's `is_giant(name)`); `fn drawn_slots(star:
    &StarInfo, metallicity: f64, rng) -> Vec<(Origin, f64)>` replacing
    `drawn_types` and `drawn_hot_jupiter` (orbit in AU).

- [ ] **Step 1: Write the failing tests**

In `planet/mass.rs`:

```rust
#[test]
fn small_planets_span_the_mass_function() {
    let mut rng = StdRng::seed_from_u64(1);
    let m: Vec<f64> = (0..20_000).map(|_| draw_mass(Origin::Small, &mut rng)).collect();
    assert!(m.iter().all(|&x| (SMALL_MASS.0..=SMALL_MASS.1).contains(&x)));
    let below_2 = m.iter().filter(|&&x| x < 2.0).count() as f64 / m.len() as f64;
    // Log-uniform from 0.05 to 20 M⊕: log(2/0.05)/log(20/0.05) = 0.62.
    assert!((below_2 - 0.62).abs() < 0.02, "{below_2}");
}

#[test]
fn giant_origins_keep_todays_laws() {
    let mut rng = StdRng::seed_from_u64(2);
    for _ in 0..2_000 {
        let gas = draw_mass(Origin::GasGiant, &mut rng);
        assert!((0.1 * JUPITER_EARTH_MASSES..=13.0 * JUPITER_EARTH_MASSES).contains(&gas));
        let ice = draw_mass(Origin::IceGiant, &mut rng);
        assert!((10.0..=50.0).contains(&ice));
        let hot = draw_mass(Origin::HotJupiter, &mut rng);
        assert!((0.3 * JUPITER_EARTH_MASSES..=3.0 * JUPITER_EARTH_MASSES).contains(&hot));
    }
}
```

In `system.rs`:

```rust
#[test]
fn drawn_slots_name_no_type() {
    // Seeds 1..=200 round a G2V: every slot is an origin and an orbit
    // inside the system's edges; hot Jupiters only within 0.1 AU.
    for seed in 1..=200 {
        let mut rng = StdRng::seed_from_u64(seed);
        let star = StarInfo::new(parse_type("G2V").unwrap(), 1.0, 0.0, 4.6);
        for (origin, a) in drawn_slots(&star, 0.0, &mut rng) {
            assert!((INNER_EDGE_AU..=OUTER_EDGE_AU).contains(&a), "{a}");
            if origin == Origin::HotJupiter {
                assert!(a <= 0.1, "{a}");
            }
        }
    }
}
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -j 4 -p mapgen planet::mass system::tests::drawn_slots_name_no_type`
Expected: FAIL, `Origin` and `drawn_slots` not found.

- [ ] **Step 3: Implement**

- `planet::mass`: `SMALL_MASS: (f64, f64) = (0.05, 20.0)`, small planets
  log-uniform on it (calibration, no source; the target is the Kepler
  census of Task 7: super-Earths, 1 to 1.75 R⊕, and sub-Neptunes, 1.75
  to 3.5 R⊕, about equally common within 100 days, Fulton et al. 2017,
  doi:10.3847/1538-3881/aa80eb). Move today's giant laws from
  `system.rs:461` unchanged (gas: dN/dlnM ∝ M^-0.31 from 0.1 to 13
  M_J, Cumming et al. 2008; ice: log-uniform 10 to 50; hot: log-uniform
  0.3 to 3 M_J).
- `drawn_slots` keeps `drawn_types`' counts and orbits (Poisson inner
  and outer counts, log-uniform periods, giants by metallicity and star
  mass, the hot Jupiter's two draws) and drops its type draws: inner and
  outer planets are `Origin::Small`. Keep the draw count per slot fixed
  whatever the branch, so a slot's orbit does not depend on another's
  type draw.
- `draw_whole` and `draw_around` take `drawn_slots`; the mass of a
  drawn slot is `planet::draw_mass(origin, rng)` at the place of today's
  `draw_mass(body_type, rng)`; a requested slot keeps `band`, `fits` and
  the per-name `draw_mass`, renamed `requested_mass(name, rng)`.
  `is_giant` call sites on slots read `Placement::is_giant`; `clear`'s
  hot-Jupiter case reads `Placement::Drawn(Origin::HotJupiter)` or
  `Requested("hot-jupiter")`. `draw_around` keeps only `Small` slots as
  rocky neighbours (as `rocky.retain` does at `:966`).

- [ ] **Step 4: Run the tests**

Run: `cargo test -j 4 -p mapgen planet::mass system::tests::drawn_slots_name_no_type`
Expected: PASS. Other `system.rs` tests may fail until Task 2; do not
commit until they are triaged: those about placement and counts must
pass now; those reading `body_type` or `MAIN_SIGNATURES` are Task 2's.

- [ ] **Step 5: Commit**

```bash
git add crates/mapgen/src/planet/mass.rs crates/mapgen/src/planet/mod.rs crates/mapgen/src/system.rs
git commit -m "feat(planet): draw slot masses from a mass function, not a type"
```

### Task 2: The model stored on each planet

**Files:**
- Modify: `crates/mapgen/src/planet/mod.rs:321` (`body_model`),
  `crates/mapgen/src/planet/*.rs` (ts-rs derives),
  `crates/mapgen/src/planet/classify.rs` (`is_giant`),
  `crates/mapgen/src/system.rs:58,190,1017-1360` (`SYSTEM_FORMAT_VERSION`,
  `Body`, `assemble`, `planet_body`), `:3798` (`MAIN_SIGNATURES`)
- Test: `crates/mapgen/src/system.rs`, `crates/mapgen/src/planet/mod.rs`

**Interfaces:**
- Consumes: Task 1's `Placement`, `Origin`.
- Produces:
  - `Body.model: Option<PlanetModel>` (`#[serde(default,
    skip_serializing_if = "Option::is_none")]`), `Some` for every
    planet, `None` for moons.
  - `Body.body_type` of a planet is `model.class.key()`.
  - `pub fn derive_model(system_seed: u32, id: &str, context: &Context,
    mass_earth: f64) -> PlanetModel` in `planet` (what `body_model` does
    today, from values rather than a stored body); `body_model(system,
    body)` returns `body.model.clone()`.
  - `PlanetModel.drawn_mass_earth: f64`, the mass drawn before escape
    (`inputs.mass_earth` is after it), so the model can be derived again
    from what the system stores.
  - `PlanetClass::is_giant(self) -> bool`: `GasGiant`, `IceGiant`,
    `HotJupiter`, `MiniNeptune`.
  - `Physical.mass_earth` of a planet is `model.inputs.mass_earth` (after
    escape).
  - ts-rs `#[derive(TS)] #[ts(export)]` on `PlanetModel` and every type
    it holds (`PlanetInputs`, `Volatiles`, `Interior`, `Layer`,
    `Material`, `InternalHeat`, `Surface`, `Water`, `Regime`, `Core`,
    `CoreState`, `Gases`, `Climate`, `SurfaceLiquid`, `Life`,
    `PlanetClass`).
  - `SYSTEM_FORMAT_VERSION = 3`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn every_planet_stores_its_model_and_its_class() {
    for seed in 1..=20 {
        let system = whole(seed); // a drawn G2V system, as `drawn()` in planet::mod
        for body in &system.bodies {
            match body.kind {
                BodyKind::Planet => {
                    let model = body.model.as_ref().expect("a planet has a model");
                    assert_eq!(body.body_type, model.class.key(), "{}", body.id);
                    assert_eq!(f64::from(body.physical.mass_earth), model.inputs.mass_earth);
                }
                _ => assert!(body.model.is_none(), "{}", body.id),
            }
        }
    }
}

#[test]
fn the_stored_model_is_the_derived_one() {
    let system = whole(3);
    for body in system.bodies.iter().filter(|b| b.kind == BodyKind::Planet) {
        let p = &body.params;
        let star = StarInfo::new(p.star, p.orbit_au, p.eccentricity, p.age_gyr);
        let context = Context::of(&star, p.metallicity, system.star.ice_line_au);
        let model = body.model.clone().unwrap();
        assert_eq!(derive_model(system.seed, &body.id, &context, model.drawn_mass_earth), model);
    }
}

#[test]
fn requested_slots_store_the_derived_class() {
    // terran=1, gas-giant=1 round a G2V, seeds 1..=10: one planet in
    // each band; each stores a class key, whatever it is.
    for seed in 1..=10 {
        let system = requested(seed, &[("terran", 1), ("gas-giant", 1)]);
        let planets: Vec<_> = system.bodies.iter().filter(|b| b.kind == BodyKind::Planet).collect();
        assert_eq!(planets.len(), 2);
        for b in planets {
            assert!(PLANET_CLASSES.contains(&b.body_type.as_str()), "{}", b.body_type);
        }
    }
}

#[test]
fn a_system_of_version_2_is_refused() {
    // write a system, patch "format_version": 2, read_system fails.
}
```

`whole(seed)` and `requested(seed, counts)` are test helpers drawing a
G2V system at metallicity 0, as `drawn()` in `planet/mod.rs` does. Add
`pub const PLANET_CLASSES: [&str; 11]` in `classify.rs`, the keys of
`PlanetClass::key`, in declaration order; the server and the web
read the same list (Task 6).

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -j 4 -p mapgen system::tests::every_planet system::tests::the_stored system::tests::requested_slots system::tests::a_system_of_version_2`
Expected: FAIL, no field `model`.

- [ ] **Step 3: Implement**

In `assemble`, once a planet's id is fixed, build its `Context` from
its star, orbit, eccentricity, the system age and metallicity and the
ice line, and call `derive_model`. The world parameters (Task 5) and
`planet_body` take the model. `preview_of` takes the class key: today's
colours keyed by the closest preset (Task 5's `base_of`), giants
unchanged. Regenerate `MAIN_SIGNATURES` once, after checking by eye
that the ids, belts and orbits still look like a system (masses and
types change on purpose); say so in the commit body.

- [ ] **Step 4: Run the crate's tests**

Run: `RAYON_NUM_THREADS=4 cargo test -j 4 -p mapgen -- --test-threads=3`
Expected: PASS, after updating the tests that pinned a drawn type
(`system.rs:2170-2293` and later): a test about placement keeps its
assertion on the slot; a test about the type asserts a class key.

- [ ] **Step 5: Commit**

```bash
git add crates/mapgen web/src/lib/api/types
git commit -m "feat(planet)!: store each planet's model and type it by class"
```

with a `BREAKING CHANGE: system.json format 3; older systems are no longer read.` footer.

### Task 3: Sea level from the water inventory

**Files:**
- Modify: `crates/mapgen/src/generation.rs:1066-1120` (next to
  `sea_level_for_land_fraction`)
- Test: `crates/mapgen/src/generation.rs`

**Interfaces:**
- Produces: `pub fn sea_level_for_water(params: &WorldParams,
  depth_m: f64) -> f32`: the sea level at which the oceans of a global
  layer `depth_m` deep fill the relief below it, on the same coarse grid
  and plate frame as `sea_level_for_land_fraction`, bisected within
  `request::SEA_LEVEL`.
- Produces: `pub fn ocean_depth_m(oceans: f64, radius_earth: f64) ->
  f64`: Earth's ocean (`OCEAN_KG`, `planet::regime`) spread over the
  body, `2.7 km × oceans / radius²` (1.4e21 kg over 5.1e14 m² at 1025
  kg/m³; check the figure).

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn earths_ocean_leaves_earths_land() {
    let params = earth_params(); // terran preset, seed 7, 256×128
    let level = sea_level_for_water(&params, ocean_depth_m(1.0, 1.0));
    let land = land_share(&params, level);
    assert!((land - 0.29).abs() < 0.08, "{land}");
}

#[test]
fn no_water_leaves_no_sea() {
    let params = earth_params();
    assert_eq!(sea_level_for_water(&params, 0.0), SEA_LEVEL.min);
}

#[test]
fn a_deep_ocean_drowns_every_summit() {
    let params = earth_params();
    let level = sea_level_for_water(&params, ocean_depth_m(10.0, 1.0));
    assert_eq!(land_share(&params, level), 0.0);
}
```

`land_share` is the land share helper `sea_level_for_land_fraction`'s
tests use (factor it out if there is none).

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -j 4 -p mapgen generation::tests::earths_ocean generation::tests::no_water generation::tests::a_deep_ocean`
Expected: FAIL, functions not found.

- [ ] **Step 3: Implement**

Bisect the level whose mean depth below it, area-weighted over the
sphere, equals `depth_m` in the relief's metres. If Earth's ocean does
not land within the tolerance because the generated relief is not
Earth's hypsometry, scale `depth_m` by one constant
`OCEAN_DEPTH_SCALE`, labelled a calibration on Earth's 0.29 land share
(ETOPO1, Amante & Eakins 2009); do not widen the tolerance.

- [ ] **Step 4: Run the tests**

Same command. Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mapgen/src/generation.rs
git commit -m "feat(generation): find the sea level a water inventory fills"
```

### Task 4: The grid's mean temperature follows the model

**Files:**
- Modify: `crates/mapgen/src/generation.rs:1143-1160,1330-1351`
- Test: `crates/mapgen/src/generation.rs`

**Interfaces:**
- Produces: `pub fn mean_surface_temperature_c(params: &WorldParams) ->
  f32`: the area-weighted mean of the sea-level row temperatures
  (`row_temperatures`, `transport_heat`, `star_temperatures`, or the
  locked profile) with the planet's warming left out, on `LAND_GRID`.
- Produces: `pub fn warming_for(params: &WorldParams, target_k: f64) ->
  f32`: `clamp(target_k − 273.15, TEMPERATURE_RANGE) −
  mean_surface_temperature_c(params)`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn the_world_lands_on_the_models_temperature() {
    // A terran preset world (seed 7, 128×64) at 1 AU round a G2V, with
    // greenhouse 0 and climate.temperature = warming_for(&params, t),
    // for t in 250, 288 and 330 K.
    for (params, target_k) in cases() {
        let world = generate(&params);
        let mean = area_mean(&world.temperature) + 273.15;
        assert!((f64::from(mean) - target_k).abs() < 5.0, "{mean} vs {target_k}");
    }
}

#[test]
fn a_lava_world_aims_at_the_grid_ceiling() {
    let params = earth_params();
    let w = warming_for(&params, 1800.0);
    assert_eq!(w + mean_surface_temperature_c(&params), TEMPERATURE_RANGE.1);
}
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -j 4 -p mapgen generation::tests::the_world_lands generation::tests::a_lava_world`
Expected: FAIL.

- [ ] **Step 3: Implement**

Factor the row-temperature block at `:1330-1348` into a function both
`generate` and `mean_surface_temperature_c` call, so the two cannot
drift apart. The tolerance is 5 K: elevation lapse and regional
variability move the mean of the stored layer off the rows' mean; if a
case misses it, find which and correct the target, not the tolerance.

- [ ] **Step 4: Run the tests**

Same command. Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mapgen/src/generation.rs
git commit -m "feat(climate): aim the grid's mean temperature at a target"
```

### Task 5: `planet::interim`, the model drives the world

**Files:**
- Create: `crates/mapgen/src/planet/interim.rs`
- Modify: `crates/mapgen/src/planet/mod.rs`,
  `crates/mapgen/src/system.rs:1277-1360` (`planet_params`,
  `planet_body`, `preview_of`), `crates/mapgen/src/request.rs:729`
- Test: `crates/mapgen/src/planet/interim.rs`, `crates/mapgen/src/system.rs`

**Interfaces:**
- Consumes: Task 2's `PlanetModel` on the body; Task 3's
  `sea_level_for_water`, `ocean_depth_m`; Task 4's `warming_for`.
- Produces:
  - `pub fn base_of(model: &PlanetModel) -> &'static str`: the built-in
    preset whose rules and remap render the model, in this order:
    giant classes → `gas-giant`, `hot-jupiter`, `ice-giant`
    (`mini-neptune` → `ice-giant`); `LavaWorld` or `Regime::HeatPipe`
    → `volcanic`; `RunawayGreenhouse` → `caustic` (the Venus look; the
    spec names barren or volcanic, caustic is closer to Venus and
    exists); liquid hydrocarbons → `frozen`; liquid water → `terran`;
    no liquid and a mean temperature under 273 K with at least 5 %
    water → `frozen`; otherwise `barren`.
  - `pub fn preset_for(model: &PlanetModel) -> Preset`: the base preset
    with `name` = the class key, `locales` empty, `climate.temperature`
    and `atmosphere.greenhouse` 0 (Task 5's `finish` sets the
    temperature), `climate.internal_heat` = `(flux / σ)^¼` K from
    `heat.flux_w_m2(radius)`, `geology.regime` = the surface's,
    `atmosphere` = `sky_of(model)`, and the lifeless remap composed after
    the base remap when `!life.arose`.
  - `pub fn sky_of(model: &PlanetModel) -> Atmosphere`: Earth's default
    for Earth's air; `rayleigh` scaled by the column (`pressure /
    gravity`, Earth = 1) and yellowed toward caustic's
    `[0.4, 0.55, 0.25]` past 10 bar; methane haze (Titan's orange,
    `[0.9, 0.6, 0.3]`) when CH₄ is over 1 % of air thicker than 0.5
    bar; `clouds`: `None` under `AIRLESS_BAR`, `Sulfuric` where SO₂ and
    H₂O both exceed 1e-6 bar, `Water` where water is liquid or H₂O over
    1e-3 bar (methane clouds render as `Water`: no new kind, spec
    "Interim surface and sky"), `Dust` on dry air under 0.1 bar,
    otherwise `None`. Every threshold labelled a calibration.
  - `pub fn params_for(model: &PlanetModel, request: WorldRequest, rng)
    -> WorldParams`: `resolve_with_preset(request, preset_for(model),
    rng)` with the request carrying `circumference_km = 2π R ×
    6371`, `pressure_bar` = the air's total, `erosion` (1.0 with liquid
    water, 0.5 with hydrocarbons, 0 otherwise, times `min(1, pressure /
    0.01)`), `volcanic` (`0.5 × sqrt(flux / 0.087 W/m²)`, 1.0 for
    heat-pipe, halved for stagnant, clamped to `VOLCANIC`),
    `surface_age_ga` (plates `U(0.2, 0.6)` as `fallback::SURFACE_AGE_GA`;
    episodic `U(0.3, 0.8)`, Venus, McKinnon et al. 1997; heat-pipe
    `U(0.0, 0.01)`, Io; stagnant `age × U(0.7, 0.95)`, Mars and
    Mercury), then `finish`.
  - `fn finish(params: &mut WorldParams, model: &PlanetModel)`: in this
    order, `params.gravity = Some(interior.gravity)`, the regime (not
    `regime_of`), `sea_level = sea_level_for_water(params,
    ocean_depth_m(water.oceans, radius))` (a hydrocarbon world, whose
    lakes the model does not size, gets
    `sea_level_for_land_fraction(params, 0.97)`: Titan's lakes cover
    about 3 % of it, Stofan et al. 2007), then `planet.climate.temperature =
    warming_for(params, climate.temperature_k)`.
  - `resolve_with_preset` leaves `geology.regime` alone when the preset
    comes from `preset_for` (a `Preset.derived: bool`, `serde(skip)`).

Lifeless remap, applied to the base remap's output (each source once):
`Desert ← Jungle, Savanna, Shrubland`; `DustPlain ← Plain`;
`RockyWaste ← Forest, Wetland`; `Tundra ← Taiga`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn solar_bodies_render_with_their_look() {
    // Earth, Venus, Mars, Mercury and Titan's inputs from
    // planet::reference at their orbits round the Sun.
    let expect = [("Earth", "terran"), ("Venus", "caustic"), ("Mars", "barren"),
                  ("Mercury", "barren"), ("Titan", "frozen")];
    for (name, base) in expect {
        assert_eq!(base_of(&model_of(name)), base, "{name}");
    }
}

#[test]
fn a_lifeless_earth_has_no_forest() {
    let mut earth = model_of("Earth");
    earth.surface.as_mut().unwrap().life.arose = false;
    let p = preset_for(&earth);
    for b in [Biome::Forest, Biome::Jungle, Biome::Taiga, Biome::Savanna,
              Biome::Plain, Biome::Shrubland, Biome::Wetland] {
        assert_ne!(p.remap(b), b, "{b:?}");
    }
    assert_eq!(p.remap(Biome::Taiga), Biome::Tundra);
}

#[test]
fn earths_sky_is_earths() {
    assert_eq!(sky_of(&model_of("Earth")), Atmosphere::default());
}

#[test]
fn venus_has_a_sulfuric_deck_and_mars_dust() {
    assert_eq!(sky_of(&model_of("Venus")).clouds, CloudKind::Sulfuric);
    assert_eq!(sky_of(&model_of("Mars")).clouds, CloudKind::Dust);
}

#[test]
fn earths_world_is_earth_sized_wet_and_mild() {
    let params = params_for(&model_of("Earth"), earth_request(), &mut StdRng::seed_from_u64(1));
    assert!((params.circumference_km - 40_075.0).abs() < 2_000.0);
    assert!((params.pressure_bar - 1.0).abs() < 0.5);
    assert_eq!(params.planet.geology.regime, Regime::Plates);
    // land share and mean temperature checked by Tasks 3 and 4
}

#[test]
fn a_mini_neptune_generates_a_giant_world() {
    // A 6 M⊕ slot at 0.1 AU round a G2V that keeps 2 % envelope.
    let body = mini_neptune_body();
    assert_eq!(body.body_type, "mini-neptune");
    assert!(body.params.giant_mass_earth.is_some());
    let r = body.physical.radius_km / 6371.0;
    assert!((2.0..=4.0).contains(&r), "{r}");
}
```

Where a solar body's model does not give the expected base, check the
model first (part 2's `the_solar_system_classifies_as_itself` passes);
change `base_of`'s order only if the spec's rule allows it.

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -j 4 -p mapgen planet::interim`
Expected: FAIL, module not found.

- [ ] **Step 3: Implement** as the Interfaces say. `planet_params`
  becomes `planet_params(model: &PlanetModel, request: WorldRequest,
  seed: u32) -> WorldParams` calling `params_for` for a body with a
  surface and today's giant path (`mass_earth: Some(model mass)`, the
  base giant preset) for a giant class. If a mini-Neptune's
  `giant_radius` lands outside 2 to 4 R⊕, take its radius from
  `model.interior.radius_earth` in `planet_body` and note the
  disagreement in an issue (`issue-writer`), not here.
- [ ] **Step 4: Run the crate's tests**

Run: `RAYON_NUM_THREADS=4 cargo test -j 4 -p mapgen -- --test-threads=3`
Expected: PASS. Add one case to Task 4's
`the_world_lands_on_the_models_temperature`: Earth's, Mars's and
Venus's models through `params_for` land within 5 K of their
`climate.temperature_k` (Venus at the grid's ceiling if above it).

- [ ] **Step 5: Commit**

```bash
git add crates/mapgen web/src/lib/api/types
git commit -m "feat(planet): generate planets' worlds from their derived state"
```

### Task 6: Format 17 and outdated systems

**Files:**
- Modify: `crates/mapgen/src/export/raw_data.rs:52,60`,
  `web/src/lib/formatVersion.ts:6`, `crates/server/src/systems.rs:180-320`
  (`Head`, `summarize`), `crates/server/src/api.rs` (`SystemSummary`),
  `crates/server/src/index.rs` (summary column or recompute),
  `web/src/components/SystemCard.svelte`, the system list component,
  `web/src/i18n/en.ts`, `web/src/i18n/fr.ts`
- Test: `crates/server/src/app.rs` (tests module), `crates/server/src/systems.rs`, a vitest next to the list component

**Interfaces:**
- Produces: `SystemSummary.outdated: bool` (ts-rs), true when the
  stored `format_version` is not `SYSTEM_FORMAT_VERSION`; such a summary
  still lists id, seed, star and dates when they parse, empty planet
  counts otherwise.

- [ ] **Step 1: Write the failing tests**

- `an_outdated_system_is_listed_not_deleted`: write a system, patch its
  `format_version` to 2, `GET /api/systems` lists it with
  `outdated: true`; the directory still exists after the call.
- `an_outdated_system_can_be_deleted_by_request`: `DELETE
  /api/systems/{id}` removes it (the existing route).
- `opening_an_outdated_system_names_clean_systems`: `GET
  /api/systems/{id}` answers the existing error containing
  `clean-systems` (`app.rs:2574` style).
- Web: the card of an outdated system shows the `outdated` label and a
  delete button, and no open link.

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -j 4 -p server outdated` and `cd web && npx vitest run SystemCard`
Expected: FAIL.

- [ ] **Step 3: Implement.** Bump `FORMAT_VERSION` and
  `MIN_FORMAT_VERSION` to 17 and `formatVersion.ts` with them. The
  labels: en `Outdated: made by an older generator`, fr `Obsolète :
  créé par un générateur plus ancien`. Deleting stays the user's click.
- [ ] **Step 4: Run the server and web tests**

Run: `RAYON_NUM_THREADS=4 cargo test -j 4 -p server -- --test-threads=3` and `cd web && npm run check && npx vitest run`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates web
git commit -m "feat(server)!: bump world format to 17 and list older systems as outdated"
```

with `BREAKING CHANGE: worlds of format 16 are no longer read.`

### Task 7: Class keys in the web, census and docs

**Files:**
- Modify: `web/src/lib/menuCards.ts:99,183`, `web/src/lib/orrery.ts:35`,
  `web/src/lib/systemStar.ts:133-144`,
  `web/src/components/BodyCard.svelte:49`,
  `web/src/i18n/en.ts:726-737`, `web/src/i18n/fr.ts:688-699`,
  `web/src/i18n/names.svelte.ts:24-33`,
  `crates/mapgen/examples/planet_census.rs`, `README.md:133`,
  `docs/science/planet-interiors.md`,
  `docs/science/planet-atmospheres.md`
- Test: vitest beside `menuCards.ts`, `orrery.ts`, `systemStar.ts`

**Interfaces:**
- Consumes: `PLANET_CLASSES` (Task 2), `Body.model` in the generated
  `Body.ts`.
- Produces: one web module `web/src/lib/planetClass.ts` with
  `GIANT_CLASSES` (`gas-giant`, `ice-giant`, `hot-jupiter`,
  `mini-neptune`) and `isHabitable(body: Body): boolean` (liquid water
  on the surface), used by the four files above in place of their own
  sets; `systemStar.ts` maps `mini-neptune` to the ice kind.

- [ ] **Step 1: Write the failing tests**

- `planetClass.test.ts`: every key of `PLANET_CLASSES` (copied as a
  literal; a server test in `app.rs`, like the one at `:2446`, reads
  the TS file with `include_str!` and checks it against
  `PLANET_CLASSES`) has an `en` and `fr` `bodyTypes` label;
  `isHabitable` is true for a body whose model has liquid water, false
  for one with none and for a moon.
- `orrery.test.ts`: a `mini-neptune` body is drawn with the giant
  radius rule.

- [ ] **Step 2: Run them to verify they fail**

Run: `cd web && npx vitest run planetClass orrery`
Expected: FAIL.

- [ ] **Step 3: Implement.** Labels: en `Rocky planet`, `Super-Earth`,
  `Ice world`, `Mini-Neptune`, `Ice giant`, `Gas giant`, `Hot Jupiter`,
  `Lava world`, `Runaway greenhouse`, `Hydrocarbon world`, `Ocean
  world`; fr `Planète rocheuse`, `Super-Terre`, `Monde glacé`,
  `Mini-Neptune`, `Géante de glace`, `Géante gazeuse`, `Jupiter chaud`,
  `Monde de lave`, `Serre emballée`, `Monde d'hydrocarbures`, `Monde
  océan`. Keep the old keys' labels: requests still use them.
  `presetName` falls back to `bodyTypes[name]` when no locale matches,
  so `meta.preset.name` (a class key) reads as a class.
  `planet_census` drops the type/class cross table (the type is the
  class) and prints the base preset mix of `interim::base_of`.
  Docs: README §133 says the model is stored and drives the worlds
  through `planet::interim`, the formats, outdated systems, and that
  requests still name types as placement hints until 3b; the two
  science pages gain a "From the model to the map" section (`base_of`,
  sky, sea level, grid offset, their calibrations).
- [ ] **Step 4: Run every test and the census**

Run the full workspace tests, clippy and the web checks (Global
Constraints), then
`cargo run --release -p mapgen --example planet_census -- 500`.
Expected: all PASS; the census shows super-Earths and sub-Neptunes
within a factor 1.5 of each other within 100 days. If not, tune
`SMALL_MASS` (Task 1) and say so in the commit.

- [ ] **Step 5: Commit**

```bash
git add web crates/mapgen/examples README.md docs/science
git commit -m "feat(web): read planet classes and document the model-driven worlds"
```

### Before the PR

- `branch-reviewer` on the whole branch with this plan and the spec.
- `world-coherence-checker` on 20 drawn systems, old binary against new:
  worlds change on purpose; report the class mix, land share and mean
  temperature per class for the main session to judge.
- Warn the peer sessions: `SYSTEM_FORMAT_VERSION` 3 and `FORMAT_VERSION`
  17 land with this PR, and every stored system becomes outdated.
- The PR description lists `Refs #423`, `Refs #427`.
