# Planet model part 4: large moons — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Large moons go through the planet model with their planet's
tides; the giants' moon draw follows the observed satellite systems;
moon archetypes can be asked for in a whole system and in the form;
small moons get a drawn bulk composition; hydrocarbon worlds keep their
liquid near the poles.

**Architecture:** The model gains a host-tide input and an ice shell
over a subsurface ocean (`planet/`). `moons.rs` draws the disk's mass,
its shape (chain or dominant moon), the rock/ice line from the disk's
temperature, forced eccentricities and small-body bulk. `system.rs`
sends every round moon through the model and stores it on the body; a
moon archetype asked for rides on its host giant's `WorldParams` so the
draw stays deterministic. The interim surface keeps a hydrocarbon
world's liquid poleward of a latitude.

**Tech Stack:** Rust (crates `mapgen`, `server`, `cli`), ts-rs generated
types, Svelte 5 + TypeScript (`web/`).

**Spec:** `docs/superpowers/specs/2026-11-14-planet-model-part4-design.md`

**Worktree:** `~/Developer/mg-planet-model-p4`, branch
`feat/planet-model-part4`. Tests: `cargo test --workspace
--all-features` (regenerates `web/src/lib/api/types`), `cd web && npm
test`. Throttle as the shared-machine memory says: `cargo test -j 4`,
no `npm ci` in the worktree (symlink `web/node_modules` from the main
checkout if missing).

## Global Constraints

- Every relation and constant cites its source (doi) or is labelled
  "Calibration, no source" with the bodies it reproduces, in the code
  and in the science page it belongs to (`docs/science/*.md`).
- `SYSTEM_FORMAT_VERSION` 4 → 5 (`system.rs:64`), world
  `FORMAT_VERSION` and `MIN_FORMAT_VERSION` 19 → 20
  (`export/raw_data.rs:66,74`), `web/src/lib/formatVersion.ts:6` 19 →
  20: once, in Task 5. No legacy format gates.
- Frequencies are not tuned for spectacle: no constant is chosen to make
  Titans common.
- An archetype sets causes (mass, ice, volatiles, host, orbit), never a
  class, a haze or a pressure.
- Each task ships its documentation (README sections, CLI help,
  `docs/science`, `docs/system-spec.md`, rustdoc) in the same commit.
  Large docs: find the section with `grep -n`, read only that range.
- Commits: Conventional Commits, English, with the session's
  `Co-Authored-By` / `Claude-Session` trailers.
- Generated TS types are committed with the Rust change that alters
  them (CI runs `git diff --exit-code -- web/src/lib/api/types`).

## Review Focus

1. **An asked moon whose host its star despins** (`of_giant`'s
   `lasting_moon_kg_with` retain, `moons.rs:965-985`) would silently
   drop the asked moon. Expected: the host is placed where the moon
   lasts, or the request fails with `TypeImpossible`. Test in Task 7.
2. **An asked moon inside a requested giant's rings or Roche limit**
   (`giant_moon_floor_radii`). Expected: its orbit is pushed out of the
   rings like the drawn chain, never inside them. Test in Task 7.
3. **A moon archetype asked around a star where its host's band falls
   outside 0.02-60 AU** (an M dwarf asked for a titan-like). Expected:
   `RequestError::TypeImpossible`, no panic. Test in Task 7.
4. **A stored format-4 system** opened after the change. Expected:
   listed as outdated, as format 3 was after part 3a. Test in Task 5.
5. **A lone moon asked around a giant** (`HostMode::Giant`,
   `{"around": …}`) gets its host's tides in its model, and its round
   siblings go through the model too. Test in Task 5.

---

### Task 1: Host tides and the ice shell in the model (opus)

**Files:**
- Modify: `crates/mapgen/src/planet/inputs.rs` (`Context`, :19-70)
- Modify: `crates/mapgen/src/planet/mod.rs` (`planet_model` :320-350,
  `PlanetModel` :158)
- Create: `crates/mapgen/src/planet/ice_shell.rs`
- Modify: `docs/science/` page on interiors (find with
  `grep -ln 'Interior\|interior' docs/science/*.md`)

**Interfaces:**
- Produces:
  - `pub struct HostTide { pub host_kg: f64, pub a_m: f64, pub eccentricity: f64 }`
    in `inputs.rs`, `Context.tide: Option<HostTide>` (serde default
    `None`), `Context::of` leaves it `None`, `Context::with_tide(self, HostTide) -> Self`.
  - `planet_model` uses the host tide when `context.tide` is `Some`
    (instead of the star's), the star's otherwise (unchanged).
  - `pub struct IceShell { pub shell_km: f64, pub ocean_km: f64 }`,
    `PlanetModel.ice_shell: Option<IceShell>` (serde default),
    `pub fn ice_shell(model_parts…) -> Option<IceShell>` in
    `ice_shell.rs`, called at the end of `chain`.
- Rule: a body with a water layer and a frozen surface (no surface
  liquid water, mean surface temperature under 273 K) has a conductive
  ice shell of thickness `d = 651 · ln(T_melt / T_s) / q` metres
  (k = 651/T W m⁻¹, Klinger 1980, doi:10.3189/S0022143000015756;
  Hussmann et al. 2006, doi:10.1016/j.icarus.2006.06.005), `q` the total
  internal heat flux (`InternalHeat::flux_w_m2`), `T_melt` 273 K
  (calibration; ammonia would lower it, not modelled). An ocean exists
  when `d` is less than the water layer's thickness; `ocean_km` is the
  difference.

- [ ] **Step 1: Write the failing tests** in `ice_shell.rs` and
  `mod.rs` tests:
  - `io_heats_by_its_host_giant`: Io inputs (mass 0.015 M⊕, dry,
    rocky), Jupiter host 1.898e27 kg, a 4.217e8 m, e 0.0041, at 5.2 AU
    of the Sun → `heat.tidal_w` within 3e13..3e14 W.
  - `europa_has_a_thin_shell_over_an_ocean`: Europa (0.008 M⊕, water
    fraction 0.08), host Jupiter, a 6.709e8 m, e 0.009 → `ice_shell`
    is `Some`, `shell_km` in 5..40, `ocean_km > 0`.
  - `callisto_shell_is_thick`: Callisto (0.018 M⊕, water 0.5), a
    1.883e9 m, e 0.0074 → `shell_km > 100`.
  - `earth_has_no_ice_shell`: the reference Earth → `None`.
  - `a_planet_without_host_tide_is_unchanged`: a model drawn with
    `Context::of` equals the one before (compare against
    `planet::reference` Earth values the existing tests already pin).
- [ ] **Step 2: Run** `cargo test -j 4 -p mapgen planet::` — new tests
  fail to compile.
- [ ] **Step 3: Implement** `HostTide`, `Context::with_tide`, the tide
  switch in `planet_model`, `ice_shell.rs`, and the field.
- [ ] **Step 4: Run** `cargo test -j 4 --workspace --all-features` —
  all pass; regenerated types diff only adds the new fields.
- [ ] **Step 5: Commit** `feat(planet): host tides and an ice shell over a subsurface ocean`.

### Task 2: A moon's inputs and model (opus)

**Files:**
- Create: `crates/mapgen/src/planet/moon.rs`
- Modify: `crates/mapgen/src/planet/mod.rs` (re-export)
- Modify: `docs/science/atmospheres.md` (Titan section, :315 area)

**Interfaces:**
- Consumes: `HostTide`, `Context::with_tide` (Task 1),
  `draw_inputs_with` (`inputs.rs:256`), `planet_model` (`mod.rs:320`).
- Produces:
  - `pub enum MoonBirth { Disk, Captured, Impact }`
  - `pub struct MoonFormation { pub mass_earth: f64, pub ice_fraction: f64, pub birth: MoonBirth, pub tide: HostTide }`
  - `pub fn derive_moon_model(system_seed: u32, id: &str, planet_context: &Context, moon: &MoonFormation, overrides: &InputOverrides) -> PlanetModel`
    (seed `body_seed(system_seed, "{id}/model")` as `derive_model`).
- Rules: the context is the planet's (insolation, XUV, equilibrium at
  the planet's orbit) with the moon's tide. Mass and water fraction
  come from `MoonFormation` (no envelope: `envelope_fraction` 0).
  Volatiles: a moon born cold in a disk or captured from beyond the ice
  line keeps N₂ (from NH₃ ice) and CH₄ in amounts the builder takes
  from Titan's inventory with a source (e.g. Mandt et al. 2014,
  doi:10.1088/2041-8205/788/2/L24; Tobie et al. 2006,
  doi:10.1038/nature04497); a warm-born rocky moon draws as a planet
  does. The cosmic shoreline, escape and climate stay the planet's
  (fluence at the planet's orbit; `atmosphere.rs:236`).

- [ ] **Step 1: Write the failing tests** in `moon.rs`, one per Solar
  System moon, with real values (G2V, 4.57 Gyr):
  - `titan_comes_out_a_hydrocarbon_world`: 0.0225 M⊕, ice 0.45,
    Disk, Saturn 5.683e26 kg, a 1.2219e9 m, e 0.0288, planet at
    9.54 AU → class `HydrocarbonWorld`, total pressure 1.0..2.5 bar,
    CH₄ share 0.01..0.10, `ice_shell.ocean_km > 0`.
  - `ganymede_is_airless_with_an_ocean`: 0.0248 M⊕, ice 0.45, a
    1.0704e9 m, e 0.0013, 5.2 AU → pressure < 1e-3 bar, ocean.
  - `callisto_is_airless`: 0.018 M⊕, ice 0.5, a 1.883e9 m, e 0.0074 →
    pressure < 1e-3 bar.
  - `io_is_a_heat_pipe`: 0.015 M⊕, ice 0, a 4.217e8 m, e 0.0041 →
    `surface.regime == Regime::HeatPipe`, pressure < 1e-3 bar.
  - `europa_has_an_ocean`: 0.008 M⊕, ice 0.08, a 6.709e8 m, e 0.009.
  - `triton_keeps_a_thin_nitrogen_air`: 0.00359 M⊕, ice 0.3, Captured,
    Neptune 1.024e26 kg, a 3.548e8 m, e 0, 30.07 AU → N₂ pressure in
    1e-6..1e-2 bar.
  - `the_moon_is_airless_and_stagnant`: 0.0123 M⊕, ice 0, Impact,
    Earth 5.972e24 kg, a 3.844e8 m, e 0.0549, 1 AU → pressure < 1e-6,
    regime `StagnantLid`.
  - `enceladus_keeps_an_ocean_under_its_ice`: 1.8e-5 M⊕, ice 0.5,
    Saturn, a 2.3802e8 m, e 0.0047, 9.54 AU → radius within 10 % of
    252 km, `ice_shell.ocean_km > 0` (the mass–radius relations must
    hold down to this mass; extend them with a source if they do not).
  - `a_moon_model_is_deterministic`: same seed and id → equal models.
- [ ] **Step 2: Run** `cargo test -j 4 -p mapgen planet::moon` — fail.
- [ ] **Step 3: Implement** `planet/moon.rs`. Where a test needs a
  relation the planet chain lacks (e.g. N₂ ice vapour pressure for
  Triton), add it with its source; if a Solar System moon cannot come
  out as itself without a calibration the spec does not allow, stop and
  report the gap rather than loosen the test.
- [ ] **Step 4: Run** `cargo test -j 4 --workspace --all-features` —
  pass, planets unchanged (existing reference tests green).
- [ ] **Step 5: Commit** `feat(planet): derive a large moon's model with its planet's tides`.

### Task 3: The giants' moon draw (opus)

**Files:**
- Modify: `crates/mapgen/src/moons.rs` (`of_giant` :864-1000,
  `regular_moon` :1362)
- Modify: `docs/analysis/11-moons.md` (sections near :198-226) and the
  rustdoc of `of_giant`

**Interfaces:**
- Produces (used by Tasks 5 and 7):
  - `pub enum DiskShape { Chain, Dominant }`, `MoonSystem.disk_shape: Option<DiskShape>` (giants only).
  - `fn disk_mass_ratio(rng) -> f64` in 1.0e-4..=2.5e-4.
  - `fn disk_shape(giant_mass_earth: f64, rng) -> DiskShape`.
  - `fn disk_ice_line_radii(giant_equilibrium_k: f64) -> f64`.
  - `fn forced_eccentricity(rng) -> f64` for chain moons.
- Rules:
  - Total regular-moon mass `disk_mass_ratio` × the giant's
    (Canup & Ward 2006, doi:10.1038/nature04860: about 10⁻⁴; the Solar
    System's giants 1 to 2.5 × 10⁻⁴).
  - Shape: `Dominant` puts 0.9-0.97 of the disk's mass in one moon
    placed in the outer half of the chain (Titan at 20 radii holds 96 %
    of Saturn's), the rest shared by the others; `Chain` as today's
    weights. Its probability falls with the giant's mass (Sasaki, Stewart
    & Ida 2010, doi:10.1088/0004-637X/714/2/1052: a giant that opened a
    gap keeps several moons); the switch mass is a labelled calibration
    giving Jupiter `Chain` and Saturn `Dominant` most of the time.
  - Rock/ice: a moon is icy beyond `disk_ice_line_radii`, which grows
    with the giant's equilibrium temperature: calibration so that
    Jupiter (110 K) puts it between Europa (9.4 radii) and Ganymede
    (15), Saturn (81 K) and colder inside the first moon, and replaces
    `ice_line = uniform(8, 11)` and the `< 60 K` test (`moons.rs:887,916`).
  - Chain moons get `forced_eccentricity` (log-uniform 0.001..0.01:
    Ganymede 0.0013, Io 0.0041, Europa 0.009); a dominant moon keeps
    0..0.03 (Titan 0.0288). Update the `a·1.01` stability margin to the
    largest eccentricity.

- [ ] **Step 1: Write the failing tests** in `moons.rs` tests:
  - `disk_mass_stays_within_the_solar_system_range`: 2000 giants →
    every total ratio in 1.0e-4..=2.5e-4.
  - `saturns_get_a_dominant_moon_and_jupiters_a_chain`: 1000 draws at
    95 M⊕ → `Dominant` share > 0.5; at 318 M⊕ → `Chain` share > 0.5;
    both shapes occur at each mass.
  - `the_disk_ice_line_sits_between_europa_and_ganymede_for_jupiter`:
    `disk_ice_line_radii(110.0)` in 9.4..15; `disk_ice_line_radii(81.0)`
    < 4.
  - `chain_moons_keep_a_forced_eccentricity`: every chain moon's e in
    0.001..=0.01.
  - Existing tests in `moons.rs` (resonant chain, Roche, rings, locked
    giants) still pass unchanged or with updated bounds stated in the
    commit body.
- [ ] **Step 2: Run** `cargo test -j 4 -p mapgen moons::` — fail.
- [ ] **Step 3: Implement** in `of_giant`.
- [ ] **Step 4: Run** `cargo run --release -p mapgen --example
  moon_census -- 500` and paste the before (spec "Measured before") and
  after tables into the commit body; `cargo test -j 4 --workspace
  --all-features` — pass.
- [ ] **Step 5: Commit** `feat(moons): draw a giant's moons from its disk's mass, shape and temperature`
  (adds `crates/mapgen/examples/moon_census.rs` with a module doc like
  `planet_census.rs`).

### Task 4: Small moons' bulk composition (sonnet)

**Files:**
- Modify: `crates/mapgen/src/moons.rs` (`Moon` :140, the rocky-planet
  paths :576-640, `regular_moon`), `crates/mapgen/src/rings.rs` (ring
  moons), `crates/mapgen/src/system.rs` (`Physical`)
- Modify: `docs/system-spec.md` (body fields, find with grep)

**Interfaces:**
- Produces: `pub struct Bulk { pub rock: f32, pub ice: f32, pub carbon: f32 }`
  (shares summing to 1), `Moon.bulk: Bulk`, `Physical.bulk: Option<Bulk>`
  (serde default, set for every moon; the card reads it).
- Rules (cite each): a moon born in a warm place is rock with little
  carbon; beyond the ice line, rock/ice about 1:1 by mass (Saturn's
  mid-sized moons, Thomas 2010 doi:10.1016/j.icarus.2010.01.025); a
  captured outer-system body carbon-rich (C-type, Phoebe: Johnson &
  Lunine 2005, doi:10.1038/nature03384); ring moons mostly ice. Drawn
  with scatter from the moon's stream; the density follows the bulk
  (rock 3.3, ice 0.93, carbon 1.8 g/cm³ mixed by volume), replacing the
  fixed per-path densities where they differ.

- [ ] **Step 1: Failing tests**: `bulk_shares_sum_to_one` (all paths,
  200 seeds); `captured_outer_moons_are_carbon_rich` (median carbon >
  0.2 for captured moons of a planet beyond the ice line);
  `warm_impact_moons_are_rock` (rock > 0.8); `density_follows_bulk`.
- [ ] **Step 2: Run** `cargo test -j 4 -p mapgen moons::` — fail.
- [ ] **Step 3: Implement.** `Composition` stays (it picks the small
  moon's look): `Ice` when `bulk.ice` > 0.5.
- [ ] **Step 4: Run** `cargo test -j 4 --workspace --all-features` — pass.
- [ ] **Step 5: Commit** `feat(moons): draw a small moon's bulk composition`.

### Task 5: Round moons enter the model in systems (opus)

**Files:**
- Modify: `crates/mapgen/src/system.rs` (`drawn_moons` :1695,
  `moon_parts` :1710, `assemble` :1395-1434, `host_chain` :1982-2040,
  preview colours :2096-2104, dust `icy` :1453)
- Modify: `crates/mapgen/src/world.rs` (`WorldParams` :16, gravity
  :229), `crates/mapgen/src/moons.rs` (`is_moon_world` :1203,
  `body_params` :1187), `crates/mapgen/src/request.rs` (`World::model`
  :824), `crates/mapgen/src/planet/interim.rs` (`params_for` :380)
- Modify: format constants (Global Constraints), `crates/mapgen/src/export/mod.rs:928` test
- Modify: README sections on systems and `world.json` (README.md:364),
  `docs/system-spec.md:55`

**Interfaces:**
- Consumes: `derive_moon_model`, `MoonFormation`, `MoonBirth` (Task 2);
  `Moon.bulk` (Task 4); `HostTide` (Task 1).
- Produces:
  - `WorldParams.moon: bool` (serde default false): set by
    `small_body_params` and by the round-moon path; `is_moon_world(p)`
    returns `p.moon` (no class test any more). `world.rs:229`'s
    `"icy-moon"` test keeps working for small moons.
  - `fn moon_formation(moon: &Moon, planet: &WorldParams, planet_mass_earth: f64) -> MoonFormation` in `system.rs`.
  - A round moon's body: `body_type = model.class.key()`,
    `body.model = Some(model)`, params from `interim::params_for` with
    `moon = true`, `moons = None`, the moon's tilt and lock as today,
    `Physical` radius, density and gravity from `model.interior`.
    A non-round moon stays `rocky-moon` / `icy-moon` with no model.
    `is_round` is unchanged: `ROUND_ICE_KM` (400) is a diameter, so
    Enceladus (504 km) already counts as round.
  - The lone-moon focus (`HostMode::Giant`, `World::model`) builds its
    context `with_tide` from `HostGiant` (mass, distance, eccentricity);
    round siblings from `host_chain` go through `moon_formation` too.
  - Dust sources (`system.rs:1453`): `icy` from `bulk.ice > 0.5` or the
    model's water fraction ≥ 0.05.
  - Preview colours: a round moon with a model takes its base look's
    colours (`interim::base_of`), as planets do.

- [ ] **Step 1: Failing tests** in `system.rs` tests:
  - `round_moons_carry_a_model_and_small_ones_none`: 50 whole systems →
    every moon body with `params.moon`; round ⇔ `model.is_some()`; a
    round moon's `body_type` is a `PLANET_CLASSES` key.
  - `a_cold_large_moon_can_be_a_hydrocarbon_world`: over seeds 1..500,
    at least one moon's class is `hydrocarbon-world` (report the count).
  - `a_lone_giants_moon_feels_its_host_tides`: an `{"around": …}`
    request with `host = giant` → its model's `heat.tidal_w` equals
    `tidal_power_w` with the `HostGiant` values.
  - `a_format_4_system_is_outdated`: a fixture with `format_version: 4`
    is listed outdated by the server index (follow the format-3 test of
    part 3a in `crates/server`).
  - Update `export/mod.rs:928` to accept a round moon's class.
- [ ] **Step 2: Run** `cargo test -j 4 -p mapgen system::` — fail.
- [ ] **Step 3: Implement**, bump the formats, run
  `cargo run --release -p mapgen --example moon_census -- 500` and keep
  its class-mix table for the PR.
- [ ] **Step 4: Run** `cargo test -j 4 --workspace --all-features` and
  `cd web && npm test` — pass.
- [ ] **Step 5: Commit** `feat(system)!: round moons go through the planet model` with
  `BREAKING CHANGE: system format 5 and world format 20; older systems are listed as outdated.`

### Task 6: Polar liquid on hydrocarbon worlds (sonnet)

**Files:**
- Modify: `crates/mapgen/src/planet/interim.rs` (`calibrate` :589,
  `params_for` :380), `crates/mapgen/src/generation.rs`
  (`sea_level_for_land_fraction` :1575, `Sampler` :706-830,
  `biome_elevation` :1216), `crates/mapgen/src/world.rs`
- Modify: `docs/science/atmospheres.md` (Titan), the interim surface's
  rustdoc

**Interfaces:**
- Produces: `WorldParams.liquid_poleward_deg: Option<f32>` (serde
  default `None`); `const POLAR_LIQUID_LAT_DEG: f32 = 55.0` in
  `interim.rs` with the spec's sources (Hayes 2016, Lorenz 2006,
  Mitchell 2008, Schneider 2012), labelled a calibration with Titan as
  target. `params_for` sets it when the surface liquid is
  `SurfaceLiquid::Hydrocarbons`.
- Rule: with `liquid_poleward_deg = Some(φ)`, a point at |latitude| < φ
  is never under the liquid (its effective sea level is −∞); the sea
  level found by `sea_level_for_land_fraction` applies the same mask,
  so the liquid still covers `1 − HYDROCARBON_LAND_FRACTION` of the
  whole surface. `None` changes nothing: water worlds stay
  byte-identical.

- [ ] **Step 1: Failing tests** in `generation.rs` tests:
  - `hydrocarbon_liquid_stays_poleward`: a 256×128 world with
    `liquid_poleward_deg = Some(55.0)` and the hydrocarbon calibration →
    no liquid pixel at |lat| < 55°, liquid share of the area-weighted
    surface within 0.5..2 %.
  - `water_worlds_are_unchanged`: an Earth-like seed generates the same
    raw bytes with the field `None` as the golden the existing
    byte-identity tests use.
- [ ] **Step 2: Run** `cargo test -j 4 -p mapgen generation::` — fail.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `cargo test -j 4 --workspace --all-features` — pass.
- [ ] **Step 5: Commit** `feat(surface): keep a hydrocarbon world's liquid near its poles`.

### Task 7: Moon archetypes in requests and the CLI (opus)

**Files:**
- Create: `archetypes/{titan,europa,ganymede,callisto,io,triton,enceladus}-like.toml`
- Modify: `crates/mapgen/src/archetype.rs` (schema :11-34, `Archetype`
  :334, `BUILT_IN` :77-130, `validate` :372, test `expected` :514)
- Modify: `crates/mapgen/src/system.rs` (`place_whole` :821-901,
  `system_archetype` :781), `crates/mapgen/src/moons.rs` (`of_giant`),
  `crates/mapgen/src/world.rs`
- Modify: `crates/cli/src/main.rs` help (:25-27, :201-206), README.md:48

**Interfaces:**
- Consumes: `DiskShape`, `disk_mass_ratio` (Task 3); `MoonFormation`
  (Task 2); round-moon bodies (Task 5).
- Produces:
  - TOML `[moon]` table: `host` (a giant archetype name, e.g.
    `"saturn-like"`), `semi_major_axis_radii` (ParamDefault),
    `eccentricity` (ParamDefault), `ice_fraction` (ParamDefault),
    `origin = "disk" | "captured"`. `[inputs] mass_earth` is the moon's;
    `[orbit]` is left out (the host's orbit is used). `Archetype.moon:
    Option<ArchetypeMoon>`; `validate` rejects a `[moon]` whose host is
    not a built-in giant archetype, and checks a moon's `mass_earth`
    against its own limit `MOON_MASS_EARTH` 1e-6..0.1 instead of
    `MASS_EARTH` (0.01..4131, `archetype.rs:135-155`).
  - `pub struct AskedMoon { pub archetype: String, pub mass_earth: f64, pub a_radii: f64, pub eccentricity: f64, pub ice_fraction: f64, pub captured: bool }`,
    `WorldParams.asked_moons: Vec<AskedMoon>` (serde default empty,
    skip if empty), drawn once at placement from the archetype's ranges.
  - `of_giant` places each asked moon first (its mass counts in the
    disk budget; a captured one replaces the drawn Triton), draws the
    others around it, and never drops it in the despun-giant retain.
  - `place_whole`: a moon archetype attaches to a requested giant that
    can carry it (same host archetype, disk budget holds the moon, the
    orbit outside rings and Roche, inside the stable Hill zone);
    otherwise it adds a `Placement::Requested` slot of its `host`
    archetype, counted toward `MAX_PLANETS`. The moon's model uses its
    archetype's `overrides`.
- Values (cite each in the TOML comments):

  | Archetype | host | mass M⊕ | ice | a radii | e | origin |
  |---|---|---|---|---|---|---|
  | titan-like | saturn-like | 0.020-0.025 | 0.40-0.50 | 18-22 | 0.02-0.03 | disk |
  | ganymede-like | jupiter-like | 0.022-0.027 | 0.40-0.50 | 14-16 | 0.001-0.002 | disk |
  | callisto-like | jupiter-like | 0.016-0.020 | 0.45-0.55 | 25-28 | 0.005-0.01 | disk |
  | europa-like | jupiter-like | 0.007-0.009 | 0.06-0.10 | 9-10 | 0.008-0.01 | disk |
  | io-like | jupiter-like | 0.014-0.016 | 0 | 5.8-6.0 | 0.004-0.005 | disk |
  | triton-like | neptune-like | 0.003-0.004 | 0.25-0.35 | 13-15 | 0-0.0001 | captured |
  | enceladus-like | saturn-like | 1.6e-5-2.0e-5 | 0.40-0.55 | 3.9-4.0 | 0.004-0.005 | disk |

  With French locales (`Titanienne`, `Europienne`, `Ganymédienne`,
  `Callistéenne`, `Ionienne`, `Tritonienne`, `Encéladienne`) and a
  one-line description each.

- [ ] **Step 1: Failing tests** in `system.rs` tests:
  - `an_asked_titan_comes_with_its_saturn`: `{"titan-like": 1}` →
    exactly one moon body with `params.archetype == Some("titan-like")`,
    class `hydrocarbon-world`, host a gas giant.
  - `an_asked_titan_rides_an_asked_saturn`: `{"saturn-like": 1,
    "titan-like": 1}` → one gas giant only, carrying the Titan.
  - `neptune_cannot_carry_a_titan`: `{"neptune-like": 1, "titan-like":
    1}` → two giants.
  - `each_moon_archetype_comes_out_as_itself`: each of the seven over
    seeds 1..10 → the outcomes of the spec's Acceptance (Titan
    hydrocarbon-world; Europa, Ganymede, Enceladus ocean; Io heat-pipe;
    Callisto airless; Triton thin N₂).
  - `an_asked_moon_outlasts_a_despun_host` (Review Focus 1),
    `an_asked_moon_keeps_out_of_the_rings` (2),
    `a_moon_archetype_its_star_cannot_host_is_refused` (3: an M5V star
    → `TypeImpossible`).
  - `too_many_planets_counts_the_hosts`: 15 × `earth-like` +
    1 × `titan-like` → `TooManyPlanets`.
  - `archetype.rs`: `expected(name)` entries for the seven, and
    `a_moon_archetype_names_a_giant_host`.
- [ ] **Step 2: Run** `cargo test -j 4 -p mapgen` — fail.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `cargo test -j 4 --workspace --all-features`;
  `cargo run --release -p map-generator -- --system titan-like=1
  --seed 7` (check the CLI binary name in `crates/cli/Cargo.toml`) →
  a system with a hydrocarbon-world moon.
- [ ] **Step 5: Commit** `feat(archetype): ask for a moon by archetype`.

### Task 8: Form, catalog and card (sonnet)

**Files:**
- Modify: `crates/server/src/api.rs` (`ArchetypeInfo` :13, `FormSpec`
  :93), `crates/server/src/catalog.rs` (:73-90), `crates/server/src/app.rs` (:476-500)
- Modify: `web/src/components/GenerateForm.svelte` (:584-606,
  :239-251, :280-288), `web/src/components/BodyCard.svelte` (:163-210)
- Create: `web/src/lib/systemPlanets.ts` + `web/src/lib/systemPlanets.test.ts`
- Modify: `web/src/lib/i18n/en.ts`, `fr.ts` (`viewer.bodyTypes`,
  `form.systemPlanets*`, `card.*`)
- Modify: README section on the web form

**Interfaces:**
- Consumes: `Archetype.moon` (Task 7), `PlanetModel.ice_shell`
  (Task 1), `Physical.bulk` (Task 4).
- Produces:
  - `ArchetypeInfo.moon: bool`; `FormSpec.moon_archetypes: Vec<String>`
    (the planets list keeps planets and giants only).
  - `systemPlanets.ts`: `export function planetsRequest(planets: Record<string, number>, moons: Record<string, number>): Record<string, number> | null`
    and `export function hostsAdded(moons: Record<string, number>, planets: Record<string, number>, hostOf: Record<string, string>): number`
    (giants the moons add when their host is not asked), used for the
    `max_planets` check.
  - The planets step shows a "Moons" group (`t.form.systemMoons`) under
    the planets, same number inputs; the summary lists asked moons.
  - BodyCard shows for a body with a model: surface pressure and main
    gases, "ocean under N km of ice" when `ice_shell.ocean_km > 0`,
    the regime; for a moon without a model: its bulk as
    "rock 60 % · ice 30 % · carbon 10 %".

- [ ] **Step 1: Failing tests** in `systemPlanets.test.ts`:
  `a_moon_whose_host_is_asked_adds_no_planet`,
  `a_moon_alone_adds_its_host`, `planets_and_moons_merge_into_one_map`;
  in `web/src/lib/planetCard.test.ts`: `shows_the_ocean_under_the_ice`,
  `shows_a_small_moons_bulk`; i18n parity tests pick up the new keys.
- [ ] **Step 2: Run** `cd web && npm test` — fail.
- [ ] **Step 3: Implement** server fields, form group, card lines,
  en/fr strings.
- [ ] **Step 4: Run** `cargo test -j 4 --workspace --all-features` and
  `cd web && npm test && npm run check` — pass.
- [ ] **Step 5: Commit** `feat(web): ask for moons in the system form and show a moon's state`.

### Task 9: Measure and look (main session, agents)

- [ ] **Step 1:** `moon_census -- 500` before (main) and after: radius
  table, class mix of round moons, share of systems with a moon over
  2300 km, mini-Neptune moon counts. Into the PR body.
- [ ] **Step 2:** `branch-reviewer` on the whole branch with the spec
  and this plan.
- [ ] **Step 3:** after fixes, rebuild `develop` (warn mapgen1-3 first),
  `develop-deployer`, then `viewer-visual-check` on a `titan-like`
  system: orange haze, methane lakes poleward of 55°, card shows the
  ocean under the ice. The user tests on develop before any merge.
