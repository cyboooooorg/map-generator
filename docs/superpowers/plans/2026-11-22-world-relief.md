# World relief (W2) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give every planet whose keyframe has a `Surface` a relief
field (`relief.height`, metres, relative to its sea level) on the
cube-sphere grid, with every number sourced, defined or a declared
calibration.

**Architecture:** Laws and constants go to a new `science::relief`
module (the only crate the constants guardrail scans); the passes go
to a new `world::relief` module, each a function of a cell's 3D point
or a neighbourhood pass on `world::Grid`. The structure is ported from
`legacy-final` (Airy crust, plates, volcanoes) with the geometry of
the 2026-11-21 studies; the noise is rebuilt on the measured spectrum.

**Tech Stack:** Rust workspace, `noise` 0.9.0 (as in legacy) in
`world`, nextest tiers, `mise run ci`.

**Spec:** `docs/superpowers/specs/2026-11-21-world-relief-design.md`
(parent: `docs/superpowers/specs/2026-11-21-world-catalogue-design.md`).
Branch: `feat/world-relief` (the design is its first commit). The CLI
export is a separate plan.

## Global Constraints

- `world` depends on `timeline` and `science` only (plus `noise`);
  `world` never reads the central object's kind.
- Every numeric constant lives in `science::relief` with a doc comment
  starting `Source:`, `Calibration:` or `Definition:`; `world` holds no
  bare physical number (only `0`, `0.5`, `1`, `2` and loop bounds).
- Draws: `Seed::derive` on the body's seed, one label per pass:
  `relief.continents`, `relief.mountains`, `relief.plates`,
  `relief.volcanoes`. No other randomness.
- No row walking: per-cell functions of `Grid::point(k)` or passes
  over `Grid::neighbours(k)`; area weights from `Grid::area(k)`.
- Heights in metres; `relief.height` stored `Storage::U16 { min:
  -25_000.0, max: 25_000.0 }`.
- Physics ported from `legacy-final` comes with its legacy tests
  (adapted to metres); read legacy with `git show legacy-final:<path>`.
- No whole world generated outside tier 3 (`#[ignore = "heavy: …"]`).
- Commits: Conventional Commits, scope `science` or `world`, with the
  session's attribution lines.

## Review Focus

- A dry world (`surface.liquid == None`) or `ocean_share` 0: no sea
  level quantile, heights relative to the area-weighted mean (Task 8).
- `ocean_share` 1: every cell under the liquid, the sea level above
  the highest cell, no panic (Task 8).
- A planet without `Surface` (a giant): `ReliefError::NoSurface`, no
  panic (Task 8).
- A tiny grid (`Grid::new(1)`, `n = 1`, 6 cells) and a tiny body (the
  octave count must stay ≥ 1): relief still built (Tasks 4 and 8).
- High gravity (super-Earth) and low gravity (relief scale capped at
  2): heights stay within ±25 km, no saturation (Task 9).

---

### Task 1: Verify the study and write it down

Research task (opus). No Rust code.

**Files:**

- Create: `docs/superpowers/studies/2026-11-22-relief/README.md`,
  `plates.md`, `volcanoes.md`, `topography.md`
- Modify: `docs/science/mountains-and-gravity.md`,
  `docs/science/plate-tectonics.md`; add a volcano section (in
  `mountains-and-gravity.md`, or a new `docs/science/volcanoes.md`
  linked from `docs/science/README.md`)

**Interfaces:**

- Consumes: the scratchpad drafts `study-plates.md`,
  `study-volcanoes.md`, `study-topography.md` (copy them into the
  session's scratchpad path the controller gives).
- Produces: the **retained values table** in the study README: one
  row per constant of Tasks 2-6 (name, value, unit, tag
  `Source`/`Calibration`/`Definition`, citation), the reference the
  later tasks copy into doc comments.

- [ ] **Step 1:** For every value the design retains (spec, sections
  The study and Calibrations left), open the source itself (DOI,
  publisher page, arXiv, NASA ADS full text) and record it as verified
  `[V]` with page, table or figure. Values found only second-hand stay
  `[S]` and go to the calibration list unless a primary source is found.
- [ ] **Step 2:** Fix the inconsistencies the drafts flagged: the
  Aharonson DOI is `10.1029/2000JE001403`; the Earth mean flux
  0.087 W/m² is Pollack et al. 1993, not Davies & Davies; the
  Allmendinger 1997 DOI must be checked; Gill 1981 supports no number.
- [ ] **Step 3:** Write the study files (purpose, method, findings per
  theme with tags, Not found with what was searched, the retained
  values table, the final calibration list).
- [ ] **Step 4:** Update the `docs/science` pages: "How it computes
  them" describes the new passes; "Where it departs from the science"
  lists the calibrations and the choices of the spec marked *my call*
  (constant continental share per regime, crusts not scaled with mass,
  every liquid alike, Venus taking Mars's H); Sources cite the study.
- [ ] **Step 5:** `npx --no-install markdownlint-cli2 "docs/**/*.md"`
  → 0 issues.
- [ ] **Step 6:** Commit `docs(science): study the sources of the relief`.
- [ ] **Step 7 (checkpoint):** The controller shows the user the final
  calibration list. No later task starts before the user approves it.

### Task 2: `science::relief` spectrum and scale laws

**Files:**

- Create: `crates/science/src/relief/mod.rs`,
  `crates/science/src/relief/spectrum.rs`,
  `crates/science/src/relief/spectrum_tests.rs`
- Modify: `crates/science/src/lib.rs` (add `pub mod relief;`)

**Interfaces:**

- Produces:
  - `pub enum Analogue { Earth, Mars }` (the regime's analogue body).
  - `pub fn hurst(analogue: Analogue) -> f64`: Earth 0.479, Mars 0.53
    (`Source:` Landais, Schmidt & Lovejoy 2018, Table 2, as verified
    in Task 1).
  - `pub const LACUNARITY: f64 = 2.0` (`Definition:`).
  - `pub fn persistence(hurst: f64) -> f64` = `LACUNARITY.powf(-hurst)`.
  - `pub fn octaves(circumference: Metre, cell: Metre) -> u32`: from
    the circumference down to a wavelength of two cells, at least 1
    (`Definition:` the grid's Nyquist limit).
  - `pub const MAX_RELIEF_SCALE: f64 = 2.0` and
    `pub fn relief_scale(gravity: MetrePerSecondSquared) -> f64` =
    `min(g_earth / g, MAX_RELIEF_SCALE)` (port of
    `legacy-final:crates/mapgen/src/generation.rs:407-420`; Weisskopf
    1975, Plescia 2004).

- [ ] **Step 1: Write the failing tests** in `spectrum_tests.rs`:
  `persistence_follows_the_hurst_exponent` (`persistence(0.5)` ≈
  `1/√2`, `persistence(1.0)` = 0.5); `octaves_reach_two_cells`
  (Earth circumference 40 075 km at 15 km → `floor(log2(40075/30)) +
  1` = 11; `octaves(1 km, 15 km)` = 1); `analogues_keep_their_measured_hurst`
  (Earth 0.479, Mars 0.53); ported
  `relief_stands_as_one_over_gravity_up_to_a_cap` from
  `generation.rs:3491` (Earth 1, Mars ≈ 2.6 → capped 2, 2 g → 0.5).
- [ ] **Step 2:** `cargo nextest run -p science relief::spectrum` →
  FAIL (module missing).
- [ ] **Step 3:** Implement the signatures above.
- [ ] **Step 4:** `cargo nextest run -p science relief` and
  `cargo nextest run -p guardrails` → PASS.
- [ ] **Step 5:** Commit `feat(science): add the relief spectrum and scale
  laws`.

### Task 3: `science::relief` crust and isostasy

**Files:**

- Create: `crates/science/src/relief/crust.rs`, `crust_tests.rs`

**Interfaces:**

- Produces:
  - `pub struct Crust { pub continental_thickness: Metre, pub oceanic_thickness:
    Metre, pub crust_density: KilogramPerCubicMetre, pub mantle_density:
    KilogramPerCubicMetre }`
  - `pub const EARTH_CRUST: Crust`, `pub const MARS_CRUST: Crust`
    (values and citations of `generation.rs:505-548`, in SI).
  - `pub fn crust(analogue: Analogue) -> Crust`.
  - `pub fn continental_share(analogue: Analogue) -> Fraction`: Earth
    0.41 (Cogley 1984), Mars 0.58 (Marinova et al. 2008).
  - `pub fn floating_height(crust: &Crust, thickness: Metre) -> Metre`:
    Airy, `(t - t_oceanic) × (1 - ρc/ρm)` (port of
    `generation.rs:593-660`).
  - `pub const ABYSS_DEPTH: Metre` (6.4 km, Parsons & Sclater 1977).

- [ ] **Step 1: Write the failing tests**: port
  `the_crust_floats_by_its_thickness` (`generation.rs:3352`, Airy
  contrast 3.5-6 km between continent and ocean floor for Earth) and
  `worlds_without_plates_have_marss_highland_share` as a pure check of
  `continental_share` (0.58 Mars, 0.41 Earth).
- [ ] **Step 2:** Run → FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** `cargo nextest run -p science relief` and the
  guardrails → PASS.
- [ ] **Step 5:** Commit `feat(science): port the floating crust`.

### Task 4: `science::relief` plate boundaries and volcanism

**Files:**

- Create: `crates/science/src/relief/plates.rs`, `plates_tests.rs`,
  `crates/science/src/relief/volcanoes.rs`, `volcanoes_tests.rs`

**Interfaces:**

- Produces (plates; values from the Task 1 table):
  - `pub enum Boundary { Collision, Andean, IslandArc, Rift, Ridge, Transform }`
  - `pub fn boundary_height(kind: Boundary, distance: Metre, convergence:
    MetrePerSecond) -> Metre`:
    the profile across the boundary (positive distance on the
    overriding side). Trench relief and width (Zhang et al. 2018);
    arc at `ARC_SLAB_DEPTH / tan(dip)` (Syracuse & Abers 2006, England
    & Katz 2010); rift axis width (Hopper & Buck 1996); orogen width
    and height (Royden et al. 2008, Allmendinger et al. 1997); ridge
    crest `RIDGE_CREST` 2.5 km and subsidence (Parsons & Sclater 1977,
    port of `plates.rs:485-491`); amplitudes without a source are the
    calibrations of the Task 1 list.
  - `pub const PLATE_COUNT: (u32, u32)`, `pub const PLATE_SPEED:
    (MetrePerSecond, MetrePerSecond)`,
    `pub const PLATE_SIZE_EXPONENT: f64` (Sornette & Pisarenko 2003),
    `pub const SLAB_FULL_SHARE: Fraction` (0.25, Zahirovic et al. 2015).
  - Add `MetrePerSecond` to `science::units` if absent (with a
    `Definition:` conversion from cm/yr).
- Produces (volcanoes):
  - `pub fn heat_flux(total: Watt, radius: Metre) -> WattPerSquareMetre` = total
    / 4πR².
  - `pub fn volcanic_intensity(flux: WattPerSquareMetre) -> Fraction`:
    the declared calibration through Earth 0.087 W/m² (Pollack et al.
    1993) and a heat pipe 2.5 W/m² (Moore & Webb 2013) = 1.
  - `pub const CONE_ASPECT: (f64, f64)` (height/width 0.01-0.1,
    Grosse & Kervyn 2018), `pub const CONE_HEIGHT_MAX: Metre`,
    `pub const ARC_SPACING_MEAN: Metre` and `ARC_SPACING_SHAPE: f64`
    (Gamma, de Bremond d'Ars et al. 1995), `pub const CONE_SPACING: Metre`
    and `VOLCANIC_PROVINCE_SHARE: Fraction` (calibrations).

- [ ] **Step 1: Write the failing tests**:
  `trenches_sit_in_the_published_range` (minimum of the Andean and
  island-arc profiles between −4.7 and −0.9 km, width at half depth
  between 31 and 93 km); `arcs_stand_above_the_slab_depth` (arc peak
  at `ARC_SLAB_DEPTH / tan(dip)` ± one width);
  `rifts_are_narrower_than_a_hundred_kilometres`;
  `collisions_reach_tibets_height` (peak ≤ 5 km at Earth gravity);
  port `ridges_follow_the_age_depth_relation_of_the_ocean_floor`
  (`plates.rs:1212`: 2.5 km crest, 3.55 km at 9 Ma);
  `volcanism_meets_its_anchors` (0.087 → Earth's value of the Task 1
  table, 2.5 → 1, monotonic); `earths_heat_flux_is_its_mean_flux`
  (44.2 TW over Earth's radius → 0.087 W/m² ± 1 %).
- [ ] **Step 2:** Run → FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** `cargo nextest run -p science relief` and the
  guardrails → PASS.
- [ ] **Step 5:** Commit `feat(science): add plate boundary and volcano relief
  laws`.

### Task 5: `world::relief` noise and area quantile

**Files:**

- Create: `crates/world/src/relief/mod.rs`, `noise.rs`, `noise_tests.rs`,
  `crates/world/src/stats.rs`, `stats_tests.rs`
- Modify: `crates/world/Cargo.toml` (`noise = "0.9.0"`, added to
  `[workspace.dependencies]`), `crates/world/src/lib.rs`

**Interfaces:**

- Produces:
  - `pub struct Spectrum { pub hurst: f64, pub octaves: u32, pub base_frequency:
    f64 }`
    (`base_frequency`: one wavelength per circumference on the unit
    sphere).
  - `pub fn fbm(seed: Seed, spectrum: &Spectrum, p: [f64; 3]) -> f64`
    and `pub fn ridged(seed: Seed, spectrum: &Spectrum, p: [f64; 3]) -> f64`:
    Perlin octaves with amplitude `persistence(H)^i`, normalised to unit
    variance; `noise::Perlin::new` seeded with the low 32 bits of
    `seed.derive("octave/<i>")`.
  - `pub fn area_quantile(grid: &Grid, values: &[f32], share: f64) -> f32`:
    the value below which `share` of the sphere's area lies
    (`share` 0 → min, 1 → max).
  - `pub fn area_mean(grid: &Grid, values: &[f32]) -> f32`.

- [ ] **Step 1: Write the failing tests**:
  `fbm_is_deterministic` (same seed and point, same value; another
  seed, another value); `fbm_follows_its_hurst_exponent` (structure
  function of 10⁴ point pairs at two separations, log-log slope within
  ±0.05 of `H` for H = 0.48 and 0.8, over the octave band);
  `area_quantile_splits_the_sphere` (field = z coordinate on
  `Grid::new(32)`: quantile at 0.5 ≈ 0, at 0.29 ≈ `2 × 0.29 − 1` = −0.42 (Archimedes)
  within one cell); `area_quantile_handles_the_ends` (0 → min, 1 → max,
  `Grid::new(1)`).
- [ ] **Step 2:** Run → FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** `cargo nextest run -p world` → PASS.
- [ ] **Step 5:** Commit `feat(world): add the relief noise and area quantiles`.

### Task 6: `world::relief` plate map on the grid

**Files:**

- Create: `crates/world/src/relief/plates.rs`, `plates_tests.rs`

**Interfaces:**

- Consumes: `science::relief::plates::*` (Task 4), `Grid`.
- Produces:
  - `pub struct PlateMap { plate: Vec<u16>, continental: Vec<bool>, motion:
    Vec<[f64; 3]> }`
    built by `pub fn PlateMap::draw(grid: &Grid, seed: Seed, continental:
    &[bool]) -> PlateMap`:
    port of `legacy-final:crates/mapgen/src/plates.rs` (`Plates::new`
    at 126, `plate_at`, the subplate lattice and slab pull at 934) onto
    `world::Grid` (same cube layout), drift (`drift_ma`) left out.
  - `pub fn PlateMap::boundary_height(&self, grid: &Grid, gravity:
    MetrePerSecondSquared) -> Vec<f32>`:
    classify each boundary cell (convergence from the two Euler
    velocities, which side is continental), compute the distance to
    the boundary by a multi-source Dijkstra over `Grid::neighbours`
    with great-circle edge lengths, apply
    `science::relief::plates::boundary_height`, scaled by
    `relief_scale`.

- [ ] **Step 1: Write the failing tests** (`Grid::new(24)`, ~3500
  cells, hand-built continental mask): port
  `plates_cover_the_sphere_with_varied_sizes` (`plates.rs:1186`;
  count within `PLATE_COUNT`),
  `boundaries_raise_ranges_and_arcs_and_sink_trenches`
  (`plates.rs:1333`), `boundary_relief_does_not_step_from_cell_to_cell`
  (`plates.rs:1243`: no jump between neighbours above the profile's
  slope × cell size); `plate_map_is_deterministic`.
- [ ] **Step 2:** Run → FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** `cargo nextest run -p world relief::plates` → PASS.
- [ ] **Step 5:** Commit `feat(world): port the plate map onto the cube grid`.

### Task 7: `world::relief` continents, mountains and volcanoes

**Files:**

- Create: `crates/world/src/relief/passes.rs`, `passes_tests.rs`

**Interfaces:**

- Consumes: Tasks 2-6.
- Produces (each returns metres per cell):
  - `pub fn continents(grid: &Grid, seed: Seed, forcing: &Forcing) -> (Vec<f32>,
    Vec<bool>)`:
    fBm at the analogue's H, crust line = `area_quantile` at
    `1 − continental_share`, Airy floating height per cell, the mask of
    continental cells. Non-plate regimes: the legacy `HIGHLANDS` noise
    (`generation.rs:729`); a world without liquid: dry plains
    `DRY_PLAINS_KM` (`generation.rs:2154`), both ported as calibrations
    into `science::relief`.
  - `pub fn mountains(grid: &Grid, seed: Seed, forcing: &Forcing, weight:
    &[f32]) -> Vec<f32>`:
    ridged noise × weight (orogeny or highlands) × relief scale;
    `MOUNTAIN_ONSET` and `MOUNTAIN_BLEND` (`generation.rs:386-400`)
    ported as calibrations only if Task 9's hypsometry test needs them.
  - `pub fn volcanoes(grid: &Grid, seed: Seed, forcing: &Forcing, arcs: &[f32])
    -> Vec<f32>`:
    isolated cones on a lattice of `CONE_SPACING` with presence from
    the intensity, arc cones along `arcs` with Gamma spacing; cone
    profile from `CONE_ASPECT`.
  - `pub struct Forcing { pub radius: Metre, pub gravity: MetrePerSecondSquared,
    pub analogue: Analogue, pub plates: bool, pub heat_flux: WattPerSquareMetre,
    pub ocean_share: Option<Fraction> }`
    with `pub fn Forcing::read(state: &PlanetState) -> Result<Forcing,
    ReliefError>`
    (regime Plates → Earth and `plates`; Stagnant, Episodic, HeatPipe
    → Mars; `ocean_share` from `surface.liquid`).

- [ ] **Step 1: Write the failing tests** (`Grid::new(24)`, hand-built
  `Forcing`): `continental_share_lies_above_the_crust_line` (area of
  the mask = 0.41 ± one cell's share for Earth);
  `the_continent_floor_step_is_airys` (median continent − median floor
  within the Airy contrast of Task 3); `volcanoes_keep_their_aspect`
  (every cone's height/half-width within `CONE_ASPECT`);
  `a_cold_planet_has_no_isolated_cones` (flux 0 → no cone);
  `forcing_reads_the_regime_not_the_body` (`Forcing::read` on the
  timeline fixture, Plates → Earth analogue).
- [ ] **Step 2:** Run → FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** `cargo nextest run -p world relief` → PASS.
- [ ] **Step 5:** Commit `feat(world): build continents, mountains and
  volcanoes`.

### Task 8: `world::relief::generate` and the sea level

**Files:**

- Modify: `crates/world/src/relief/mod.rs`, `crates/world/src/error.rs`
- Create: `crates/world/src/relief/generate_tests.rs`

**Interfaces:**

- Produces:
  - `pub const HEIGHT: &str = "relief.height"` (the `FieldId` code).
  - `pub enum ReliefError { NoSurface }` (or a `WorldError` variant;
    the builder picks, one place only).
  - `pub fn sea_level(grid: &Grid, heights: &[f32], ocean_share:
    Option<Fraction>) -> f32`:
    `area_quantile(…, share)` when `Some(share)` with share > 0;
    `area_mean` otherwise; share 1 → the maximum.
  - `pub fn generate(state: &PlanetState, seed: Seed, grid: &Grid) ->
    Result<Field, ReliefError>`:
    `Forcing::read`, continents + plates + mountains + volcanoes, minus
    `sea_level`, as `Field { values: Values::F32, categorical: false, storage:
    Storage::U16 { min: -25_000.0, max: 25_000.0 } }`.

- [ ] **Step 1: Write the failing tests**:
  `sea_level_leaves_the_ocean_share_under_water`
  (hand-built heights on `Grid::new(16)`, share 0.71 → area below 0
  after subtraction = 0.71 ± one cell); `a_dry_world_is_measured_from_its_mean`;
  `a_full_ocean_covers_every_cell` (share 1 → all heights ≤ 0);
  `a_planet_without_surface_has_no_relief` (state with `surface: None`
  → `NoSurface`); `the_smallest_grid_still_has_relief` (`Grid::new(1)`,
  fixture state → 6 finite values).
- [ ] **Step 2:** Run → FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** `cargo nextest run -p world` → PASS.
- [ ] **Step 5:** Commit `feat(world): generate the relief and its sea level`.

### Task 9: Tier-3 relief censuses

**Files:**

- Create: `crates/world/tests/relief_heavy.rs` and its hand-built
  states (Earth-, Mars- and Venus-like `PlanetState`s, built from the
  structural fixture of `crates/timeline/src/keyframe_tests.rs:62`,
  promoted to a `timeline` test-support module if needed without
  adding a dependency to `world`).

**Interfaces:**

- Consumes: `world::relief::generate` (Task 8).

- [ ] **Step 1: Write the tests**, each `#[ignore = "heavy: world at 60 km"]`,
  seeds 1-3, `Grid::for_radius(R, 60 km)`:
  `earth_like_has_earths_hypsometry` (ocean share 0.71 → land 0.29;
  land above 2 km 4-15 %, above 4 km 0.5-4 %, sea deeper than 6 km
  under 3 %: ETOPO 2022 as in `generation.rs:3202`);
  `mars_like_keeps_its_highland_share` (0.55-0.61);
  `venus_like_is_unimodal` (80 % within 1 km of the mean, Ford &
  Pettengill 1992); `relief_has_its_analogues_hurst_exponent` (H
  measured on the field within ±0.08 of 0.479 and 0.53);
  `relief_never_saturates` (Earth-like at 3 g and Mars-like at 0.2 g:
  all heights within ±25 km); `same_seed_same_relief`.
- [ ] **Step 2:** `mise run test:heavy` (or `cargo nextest run
  --profile heavy -p world --run-ignored only`) → record results. A
  failing hypsometry test: port `MOUNTAIN_ONSET`/`MOUNTAIN_BLEND` or the
  shelf constants as calibrations (Task 7), then rerun; report the
  final numbers.
- [ ] **Step 3:** Commit `test(world): add the relief censuses`.

### Task 10: Documentation, follow-ups, CI

**Files:**

- Modify: `crates/world/README.md` (the relief field, its sea-level
  convention, the passes and their seeds), `crates/science/README.md`
  (the `relief` module).

- [ ] **Step 1:** Update the READMEs.
- [ ] **Step 2:** File the follow-up issues of the spec (craters on
  the cube grid; erosion as a neighbourhood pass after W3; a
  continental share driven by the planet's history; tier-3 relief
  tests to tier 2 after mapgen1's PR 2d) with `issue-writer`, each
  placed in the roadmap (#427 rules in CLAUDE.md).
- [ ] **Step 3:** `mise run ci` → passes (format, clippy, tiers 1-2
  within budget, markdownlint).
- [ ] **Step 4:** Commit `docs(world): document the relief`.
- [ ] **Step 5:** `branch-reviewer` and `test-reviewer` on the whole
  branch with the spec; fix, rerun `mise run ci`; the PR description
  lists every calibration under its own heading and pastes the Task 9
  census output.
