# Planet model PR 2b-2: the shortwave Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port the legacy two-stream shortwave and the atmosphere's
part of the Bond albedo into `science`, with `shortwave.bin`, the
Rayleigh depth from refractive indices, and the cloud albedo refitted on
Earth's 0.293 over a ground albedo of 0.122.

**Architecture:** New modules in `science::radiation`: `starlight`
(the star's 240 bands), `rayleigh` (per-gas cross-sections),
`shortwave` (table, two-stream layers, stacks, paths), `veils` (the
sulphuric deck and the methane haze) and `albedo` (the budget over a
given ground, cloud cover and deck, with the caller-owned
`ShortwaveCache` replacing the legacy `thread_local!` memos). The
methane anti-greenhouse joins `methane`. Typed units at the public
boundary, plain `f64`/`f32` inside the loops, as in 2b-1.

**Tech Stack:** Rust, `include_bytes!` for `shortwave.bin`, `cargo
nextest` tiers, the existing `ktable::Table::parse` (already decodes the
`MGSWCK01` format).

**Spec:**
`docs/superpowers/specs/2026-11-22-planet-model-pr2b-radiative-design.md`
(section "PR 2b-2"). Reference page: `docs/science/radiative-transfer.md`
("The shortwave half" l.318-411, "Where it departs" l.517-679). Audit:
Wiki study 19,
<https://github.com/cyboooooorg/map-generator/wiki/19-radiative-transfer-audit>
(albedo rows around l.330-430 of the study).

Legacy paths below are under `legacy-final:crates/mapgen/src/planet/`:
`sw` is `shortwave.rs`, `alb` is `albedo.rs`, `cli` is `climate.rs`.

## Global Constraints

- Legacy is read only at tag `legacy-final` (`git show
  legacy-final:crates/mapgen/src/planet/<file>`); every law is ported
  with its tests, then changed only where the spec's decisions say so.
- Every numeric `const`/`static` (tests included) and every `let` bound
  to a bare number other than 0, 0.5, 1, 2 has a doc comment starting
  `Source:`, `Calibration:` or `Definition:` (guardrail
  `science_constants_are_sourced`). The citation is the one the audit
  checked; legacy "from memory, not checked" comments are not carried
  over. No number read from memory: each is read in a reachable source.
- Missing science stops the work (CLAUDE.md "No invented data"): a
  stop goes to the main session, which brings it to the user. No test is
  `#[ignore]`d and no tolerance widened to get past a stop.
- No `thread_local!`, `HashMap`, `Rc`, file, network, clock or process
  in `science`. Caches are values the caller owns.
- `shortwave.bin` is copied byte for byte from `legacy-final`:
  1 841 972 bytes, SHA-256
  `de6ffe0a5069fc6fcbde77badace9377b40095c3352d3ba385b4aea12b1c2380`
  (already recorded in `crates/science/data/OPACITY.md:36`).
- Every 2b-1 dead-code allowance kept for 2b-2 is removed by the task
  whose code reads the item (they are `#[expect]`, so the build fails
  until removed): `Table::water_continuum` (Task 2),
  `spectrum::co2_pairs`, `water_continuum_depth`, `co2_continuum_depth`,
  `column::Levels`, `column::levels` (Task 4),
  `stratospheric_methane_bar` (Task 6).
- Out of scope (2d): the surface cover and its albedos, `bond_albedo`
  over a `Cover`, `absorbed_w_m2`, the climate solver, the #593 levers.
  `ENGINE_VERSION` is not bumped.
- `crates/science/README.md` stays at 150 lines or fewer (it is at 150).
- Before each commit: `cargo fmt`, `cargo clippy -p science
  --all-targets -- -D warnings`, the task's tests. Before the PR:
  `mise run ci`. Tier 1 stays within 120 s; a test over 10 s goes to
  tier 3 with `#[ignore = "heavy: <why>"]`.
- Commits follow Conventional Commits with the session's attribution
  trailers, scope `science`: type `feat` for Tasks 2-7, `docs` for
  Tasks 1, 1b and 8.
- Implementers: opus for Tasks 1, 1b, 3, 4, 5, 7; sonnet for 2, 6, 8.
- Order: Task 1 gates only Task 5, Task 1b only Task 7. Tasks 2, 3, 4
  and 6 do not wait for them. Task 5 waits for Tasks 1 and 6 (Titan's
  test points); Task 7 waits for Tasks 1b, 4, 5 and 6.

## Review Focus

1. A star of 0 K, a negative or NaN temperature, or one hotter than any
   real star (50 000 K): the bands stay finite and sum to 1. Legacy maps
   `teff <= 0` to the Sun; the port keeps it and adds NaN to that case
   (Task 2).
2. No air at all (an airless body, every pressure 0 or `-1e-12`): the
   budget is the ground albedo under the clouds and deck, with no
   absorption; the paths have no column (Task 7, `no_air_returns_the_ground`).
3. A cache hit returns the bit-identical budget of a fresh computation,
   and past 32 airs a recomputed air is still bit-identical (Task 7).
4. Opaque layers: Venus's 92 bar of CO₂ with steam, `layer(1e3, 1e3)`:
   finite, `0 <= albedo <= 1`, `0 <= air <= 1 - albedo` (Tasks 3 and 7).
5. A deck of exactly 1 or above, a cloud cover of 1, a ground albedo
   of 0 or 1: the budget stays in `[0, 1]`; a full deck returns
   `DECK_ALBEDO` with no air absorption, as legacy (Task 7).

---

### Task 1: Rayleigh study (stop gate for Task 5)

**Files:**

- Modify: `docs/science/radiative-transfer.md` ("Rayleigh scattering"
  paragraph l.353-374; "Sources" l.680)

The audit derived N₂, O₂, CO₂ and CH₄ from He et al. 2021 (ACP 21,
14927, doi:10.5194/acp-21-14927-2021, Table 1 as xlsx) and found no
refractive-index fit for NH₃ and SO₂. The page gives the formula but
no per-gas data. Legacy leaves H₂O out of the Rayleigh sum.

- [ ] **Step 1:** For each of N₂, O₂, CO₂, CH₄, NH₃, SO₂ and H₂O, find
  and read a published refractive-index dispersion formula
  (`n(λ) − 1` at a stated temperature and pressure, or at a stated number
  density) and a King factor `F_K(λ)` or depolarisation ratio. Start
  from He et al. 2021 Table 1 and the sources it cites (Bates 1984,
  Peck & Khanna 1966, Sneep & Ubachs 2005, Thalman et al. 2014), then
  search for NH₃, SO₂, H₂O.
- [ ] **Step 2:** Add to the page, per gas: the formula with its
  coefficients, its validity range in λ, the reference number density,
  `F_K`, the citation (DOI, table or equation), and `σ(550 nm)` computed
  from them (cm² per molecule). Add a check of each `σ` against a
  published measured or tabulated cross-section where one exists
  (Sneep & Ubachs 2005 at 532 nm; Thalman et al. 2014), and Earth's
  dry air at 550 nm against Bucholtz 1995 if its table can be opened.
- [ ] **Step 3: Stop test.** If a gas has no published refractive index
  or no King factor, stop and report to the main session with what was
  searched. The user picks between leaving the gas out of the Rayleigh
  sum (as legacy does for H₂O) and a documented calibration (e.g. its
  static polarizability ratio to N₂). Do not choose.
- [ ] **Step 4:** Commit `docs(science): source the per-gas Rayleigh
  cross-sections`.

### Task 1b: Clear-sky absorption by absorber (stop gate for Task 7)

**Files:**

- Modify: `docs/science/radiative-transfer.md` ("Orders of magnitude"
  l.386-388, "What we do not know" l.400-411, "Sources" l.680),
  `docs/science/planet-atmospheres.md:1160-1162` (the unverified
  "ozone about 13, aerosols about 7")

The model absorbs sunlight only in H₂O and CO₂ (`shortwave.bin` holds
no O₂, O₃ or aerosol). The observed clear-sky window includes
aerosols and ozone. Kim & Ramanathan 2008 (J. Geophys. Res. 113,
D02203, doi:10.1029/2007JD008434), read on 2026-11-23 from
<https://ramanathan.ucsd.edu/wp-content/uploads/sites/460/2017/10/pr157.pdf>
(its text extracts with macOS PDFKit), give for global annual clear
skies, under an insolation of 341.8 W/m² (S₀ = 1367):

- atmospheric absorption 72.1 W/m², TOA reflected 51.4 W/m², surface
  218.3 W/m² (Table 3b, `MACR_BL`; abstract: 72 ± 3);
- aerosol forcing: +4.8 W/m² of atmospheric absorption and +5.9 W/m²
  of TOA reflection (Table 3b, ARF columns; §[51]: aerosol absorption
  "4 to 5 W m⁻²");
- water vapour "about 50 W m⁻²" of the clear-sky absorption (§[51]),
  against 43 in IPCC 2001 (Kiehl & Trenberth 1997 spectroscopy);
- no separate ozone, O₂ or CO₂ value. The "ozone about 13, aerosols
  about 7" of `planet-atmospheres.md:1162` is not in the paper: the
  aerosols are 4.8;
- inputs: total ozone 300 ± 30 DU, precipitable water 3.0 ± 0.3 cm,
  surface albedo 0.15 ± 0.02 (Table 2), gases H₂O, O₃, O₂, CO₂ from
  HITRAN 2000 (§[21]).

Legacy got 48 W/m² for H₂O and CO₂.

- [ ] **Step 1:** Write the facts above on the page with their table
  and paragraph numbers, and correct `planet-atmospheres.md:1160-1162`.
  "What we do not know" says that ozone, O₂ absorption and aerosols are
  not modelled, with a link to #653.
- [ ] **Step 2:** Find and read a source for the CO₂ (and, for the
  record, O₂ and O₃) share of Earth's clear-sky shortwave absorption,
  for a stated profile: Kiehl & Trenberth 1997 Table 4 (cited by Kim &
  Ramanathan §[51] for pre-2000 absorption; whether it splits by gas is
  to be read), the RFMIP/RTMIP line-by-line
  shortwave benchmarks (Pincus et al. 2020; Collins et al. 2006).
  Derive the H₂O + CO₂ window: Kim & Ramanathan's water vapour, about
  50 W/m² on 341.8 (as a share of the insolation, to scale to 340.25),
  plus the CO₂ share read, with the spread of the sources as its
  tolerance. Record the precipitable water of the model's Earth column
  (`EARTH_AIR` at 77 % over 288 K) beside Kim & Ramanathan's 3.0 cm, so
  that a gap from the humidity is not taken for one of the optics.
- [ ] **Step 3:** Derive the aerosol-free clear-sky albedo window: the
  0.149-0.156 of Stephens et al. 2015 and Kato et al. 2018 includes
  aerosols, which reflect +5.9 W/m² of 341.8 (+0.017) in Kim &
  Ramanathan's Table 3b. Write the derivation on the page; Task 7's
  `earths_clear_sky_albedo_is_observed_without_aerosols` uses the
  window the user approves (Step 3b).
- [ ] **Stop test, Step 3b:** both windows change decisions of the spec
  ("2b-2 calibration": 0.149-0.156; decision 4: 72-73 W/m²). Bring the
  derived windows to the main session, which asks the user before
  Task 7 uses them. If no CO₂ share can be read, report what was
  searched; do not choose a window.
- [ ] **Step 4:** Commit `docs(science): source the clear-sky
  absorption by absorber`.

### Task 2: `shortwave.bin` and the star's bands

**Files:**

- Create: `crates/science/data/shortwave.bin` (`git show
  legacy-final:crates/mapgen/data/shortwave.bin > …`),
  `crates/science/src/radiation/starlight.rs`
- Modify: `crates/science/src/radiation/mod.rs`, `ktable.rs:78-81`
  (remove the `water_continuum` allowance), `shortwave.rs` (created
  here with the table only), `crates/science/data/OPACITY.md:36` (the
  file is now present)
- Port from: `alb` 405-480 (`SUN_TEFF_K`, `spectrum`, `SPECTRUM_*`,
  `band_edge_m`), `sw` 61-80 (`TABLE`)

**Interfaces:**

- `pub(crate) const BANDS: usize = 240`, `FROM_M` 0.2e-6, `TO_M` 5.0e-6
  (`Definition:` numerical choices, reference page l.590-607).
- `pub(crate) fn band_edge_m(e: usize) -> f64`.
- `#[derive(Clone, Debug, PartialEq)] pub(crate) struct StarBands {
  pub(crate) star_k: f64, pub(crate) bands: Vec<(f64, f64)> }`, each
  band `(scale = (550 nm / λ)⁴, weight)`, bluest first, weights summing
  to 1.
- `pub(crate) fn star_bands(star_k: f64) -> StarBands`: Planck weights
  with `constants::SECOND_RADIATION_CONSTANT_CM_K` (× 1e-2 for m·K; no
  new literal); `star_k <= 0` or NaN takes
  `constants::SOLAR_EFFECTIVE_TEMPERATURE`. No memo here (Task 7's cache
  keeps the last star).
- `pub(crate) static SHORTWAVE: LazyLock<Table>` in
  `radiation::shortwave`: `Table::parse(include_bytes!(
  "../../data/shortwave.bin"))` with the legacy asserts (non-empty
  edges, each edge on `band_edge_m(BANDS - j)` to 1e-9).

- [ ] **Step 1: Write the failing tests**: in `starlight.rs`,
  `the_bands_carry_all_the_light` (weights sum to 1 to 1e-12 for 2600,
  5772 and 50 000 K; every scale and weight finite and ≥ 0),
  `a_cold_or_unknown_star_is_the_sun` (0 K, -1 K and NaN give the bands
  of 5772 K, `==`, `star_k` holding the temperature used),
  `a_red_star_puts_its_light_in_the_infrared` (share of the weight past
  0.7 µm: 3300 K > 5772 K), `the_bands_peak_where_planck_does` (the
  bands are even in `ln λ`, so a band's weight follows `λ B_λ`, whose
  maximum is at `λ T = c₂ / x` with `x` the root of `x = 4 (1 − e^−x)`,
  3.9207, solved in the test by fixed-point iteration, `Definition:`;
  not Wien's `b`, which is the maximum of `B_λ`: at 5772 K the band of
  largest weight holds that `λ`); in `shortwave.rs`, the
  legacy `the_table_is_on_the_albedo_bands` (`sw` 681: 147 bins, first
  edge 2000 cm⁻¹ to 1e-6, last > 14 300, weights sum to 1 to 1e-12).
- [ ] **Step 2:** Run `cargo nextest run -p science radiation::starlight
  radiation::shortwave`. Expected: FAIL (items missing).
- [ ] **Step 3:** Copy the table and check `shasum -a 256` against the
  hash above; port the code.
- [ ] **Step 4:** Run the same command. Expected: PASS.
- [ ] **Step 5:** Commit `feat(science): add the shortwave table and the
  star's bands`.

### Task 3: Two-stream layers and stacks

**Files:**

- Modify: `crates/science/src/radiation/shortwave.rs`
- Port from: `sw` 200-320 (`layer`, `layer_fast`, `Stack`), tests `sw`
  610-632, 655-680

**Interfaces:**

- `pub(crate) fn layer(absorption: f64, scattering: f64) -> (f64, f64)`
  and `pub(crate) fn layer_fast(absorption: f32, scattering: f32) ->
  (f32, f32)`: reflectance and transmission, Meador & Weaver 1980 with
  the hemispheric-mean coefficients of Toon et al. 1989 Table 1,
  `spectrum::DIFFUSIVITY`.
- `#[derive(Clone, Copy, Debug)] pub(crate) struct Stack { down, up,
  through: f64 }` with `EMPTY`, `over(self, r, t) -> Stack`,
  `then(self, below: Stack) -> Stack`, `on(self, ground: f64) -> (f64,
  f64)` (reflected, absorbed by the ground).
- Citations: the audit's corrected DOI for Meador & Weaver 1980; Toon
  et al. 1989 doi:10.1029/JD094iD13p16287; the pure-scattering limit
  `a / (1 + a)` cites Sobolev 1975 §1.3 only as the audit records it
  (via Zuluaga et al. 2022), else the reference page's wording.

- [ ] **Step 1:** Port the legacy tests
  `a_layer_absorbs_or_scatters_as_the_two_stream_limits_say`,
  `the_fast_layer_is_the_layer`,
  `stacking_two_halves_gives_the_whole_layer` with their tolerances and
  a test doc citing each limit.
- [ ] **Step 2:** Run `cargo nextest run -p science radiation::shortwave`.
  Expected: FAIL.
- [ ] **Step 3:** Port the code.
- [ ] **Step 4:** Same command. Expected: PASS.
- [ ] **Step 5:** Commit `feat(science): port the two-stream layers`.

### Task 4: The light's paths through the column

**Files:**

- Modify: `crates/science/src/radiation/shortwave.rs` (tests may go to
  `shortwave_tests.rs` like `column_tests.rs`), `spectrum.rs:413-418,
  479-484, 532-537`, `column.rs:701, 713` (remove the allowances)
- Port from: `sw` 85-200 (`THIN`, `LevelAbsorption`, the three smooth
  depths), 323-605 (`Budget`, `Piece`, `pieces`, `piece_depth`,
  `piece_depth_fast`, `Air`, `Paths`, `Points`), test `sw` 634-654

**Interfaces:**

- `pub(crate) struct Air<'a> { pub(crate) haze: (f64, f64),
  pub(crate) rayleigh_550: f64, pub(crate) column: Option<&'a Levels> }`.
- `pub(crate) struct Paths` (`Clone, Debug`), `Paths::through(air:
  &Air, bands: &[(f64, f64)]) -> Paths` and `Paths::budget(&self,
  ground: f64, cloud: f64) -> Budget`.
- `#[derive(Clone, Copy, Debug, PartialEq)] pub(crate) struct Budget {
  pub(crate) albedo: f64, pub(crate) air: f64 }`.
- `THIN` 0.1: `Calibration:` (merging moves Earth's Bond albedo by
  2e-5, the legacy check; re-measured in Task 7 Step 5).
- `piece_depth` stays `#[cfg(test)]` and uses
  `spectrum_reference::log_mean`, as legacy.

- [ ] **Step 1:** Port `the_fast_depth_is_the_depth`, and add
  `paths_do_not_depend_on_the_ground`: one `Paths` for `EARTH_AIR` with
  77 % vapour, budgets over grounds 0, 0.122, 1 and clouds 0, 0.5: over
  a ground of albedo 1 the ground absorbs nothing, so `albedo + air =
  1` to 1e-12; the albedo grows with the ground and with the cloud
  reflectance.
- [ ] **Step 2:** Run `cargo nextest run -p science radiation::shortwave`.
  Expected: FAIL.
- [ ] **Step 3:** Port; move `EARTH_AIR` from `column_tests.rs:17` to
  `test_points.rs` so both test modules share it.
- [ ] **Step 4:** Same command, then `cargo clippy -p science
  --all-targets -- -D warnings` (no unfulfilled `expect`). Expected: PASS.
- [ ] **Step 5:** Commit `feat(science): port the shortwave paths`.

### Task 5: Rayleigh depth from refractive indices

Waits for Task 1 and the user's ruling if Task 1 stopped.

**Files:**

- Create: `crates/science/src/radiation/rayleigh.rs`

**Interfaces:**

- `pub(crate) fn cross_section_cm2(gas: Gas, wavelength_m: f64) -> f64`:
  `σ = 24π³/(λ⁴N²)·((n²−1)/(n²+2))²·F_K` with each gas's formula from
  Task 1 (`Source:` per coefficient). `Gas` is a small `pub(crate)`
  enum of the gases Task 1 kept.
- `pub(crate) fn rayleigh_depth_550(gases: &PartialPressures,
  gravity_m_s2: f64) -> f64`: `Σ σ_i(550 nm) N_i`, with
  `N_i = p_i N_A / (g μ̄)` molecules per area and `μ̄` the column's
  mean molar mass (the reference page's `τ_R = σ p / (μ m_u g)`).
  Negative and NaN pressures count as 0.
- Replaces legacy `RAYLEIGH_N2_550`, `CO2_RAYLEIGH_PER_N2` and the
  polarizability table: none of them is ported.

- [ ] **Step 1: Write the failing tests**:
  `each_gas_scatters_as_its_published_cross_section` (each `σ` at the
  Task 1 check wavelengths, within the tolerance Task 1 gives from the
  source's own uncertainty); `co2_scatters_two_and_a_half_times_nitrogen`
  (per molecule at 550 nm, 2.51 ± 0.02, derived on the page from He et
  al. 2021; Kopparapu et al. 2013 §2.2.1 "2.5 times");
  `oxygen_scatters_a_little_less_than_nitrogen` (0.973 ± 0.01);
  `earths_air_is_bucholtzs` (dry Earth air, 1.01325 bar, standard
  gravity: 0.097 within the tolerance the page sets, only if Task 1
  opened Bucholtz 1995 Table 4; else the test checks the page's own
  derived 0.099 and its doc says so); the legacy
  `rayleigh_counts_the_column_mass` (alb 1020: double gravity gives
  exactly half to 1e-12; Titan's sourced air (Task 6 test points) at
  `TITAN_GRAVITY` over Earth's `EARTH_AIR` at `STANDARD_GRAVITY` is the
  ratio of their pressures over gravities within 5 %, the gap being
  their compositions).
- [ ] **Step 2:** Run `cargo nextest run -p science radiation::rayleigh`.
  Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Same command. Expected: PASS.
- [ ] **Step 5:** Commit `feat(science): derive Rayleigh from refractive
  indices`.

### Task 6: Deck, haze and the methane anti-greenhouse

**Files:**

- Create: `crates/science/src/radiation/veils.rs`
- Modify: `crates/science/src/radiation/methane.rs`, `column.rs:254-257`
  (remove the allowance), `test_points.rs` (Titan)
- Port from: `alb` 318-358 (`DECK_*`, `HAZE_*`), 580-600
  (`sulphuric_deck`, `haze`); `cli` 214-263 (`CH4_NEAR_IR_MAX`,
  `CH4_NEAR_IR_HALF_BAR`, `stratospheric_share`), tests `cli` 3053-3114

**Interfaces:**

- `pub fn sulphuric_deck(surface: Kelvin, so2: Bar) -> Fraction` over
  `pub(crate) fn deck_share(t_k: f64, so2_bar: f64) -> f64`.
- `pub(crate) fn haze(gases: &PartialPressures) -> f64`.
- `pub fn stratospheric_share(gases: &PartialPressures, skin: Kelvin) ->
  Fraction` over `pub(crate) fn stratospheric_share_of(gases, skin_k:
  f64) -> f64` (the share of the absorbed starlight the stratosphere's
  methane sends back to space, McKay, Pollack & Courtin 1991). Its
  consumer, `absorbed_w_m2`, is 2d's.
- Constants and their prefixes, from the audit rows: `DECK_ALBEDO` 0.76
  `Source:` Haus et al. 2016 with the audit's DOI; `DECK_SO2_BAR`
  1e-3, `DECK_MIN_K` 273.15, `DECK_ONSET_K` 40, `HAZE_CH4_BAR` 0.01
  `Calibration:` with what was searched (reference page l.563-585);
  `HAZE_CH4_PER_CO2` 0.2 `Source:` Trainer et al. 2006 (C/O 0.6);
  `HAZE_ALBEDO` 0.25 and `HAZE_ABSORPTION` 0.55 `Calibration:` on Titan's
  Bond albedo 0.265 (Li et al. 2011, the audit's corrected value, not
  0.27); `CH4_NEAR_IR_MAX` 0.5 `Definition:` (thin-layer limit, no
  Wikipedia line), `CH4_NEAR_IR_HALF_BAR` 0.002 `Calibration:` on McKay
  1991's 0.306 (the audit's row).
- Test points in `test_points.rs` (`Source:`): Titan's surface
  pressure 1.467 bar and temperature 93.65 K (Fulchignoni et al. 2005),
  methane share 5.65 % (Niemann et al. 2010, doi:10.1029/2010JE003659),
  the rest N₂, effective temperature 82 K (McKay et al. 1991); Venus's
  surface 92 bar and 737 K with 150 ppm of SO₂ (NASA Venus Fact Sheet,
  nssdc.gsfc.nasa.gov, as `VENUS_GRAVITY` cites it).

- [ ] **Step 1: Write the failing tests**: the legacy
  `stratospheric_methane_takes_a_share_of_the_light` without its
  `absorbed_w_m2` lines; `the_methane_anti_greenhouse_matches_mckay` on
  the sourced Titan (skin `skin_temperature(82 K)`) in place of the
  legacy generated system, same tolerances (10 % on 0.306, 15 % on
  0.29); `venus_is_under_its_deck` (Venus's test point: share 1);
  `a_cold_or_sulphur_poor_air_has_no_deck` (272 K or
  `so2 < DECK_SO2_BAR`: 0); `titan_is_hazy_earth_is_not` (sourced
  Titan: haze > 0.9; Earth's air with 1.9 ppm CH₄, `Source:` NOAA
  global mean: haze < 0.001); `co2_cuts_the_haze` (CH₄/CO₂ below 0.2
  scales the haze down, at 0.1 half of it).
- [ ] **Step 2:** Run `cargo nextest run -p science radiation::veils
  radiation::methane`. Expected: FAIL.
- [ ] **Step 3:** Port. If a constant has neither a source the audit
  checked nor a defensible calibration, stop and report.
- [ ] **Step 4:** Same command. Expected: PASS. **Stop test:** if
  `the_methane_anti_greenhouse_matches_mckay` fails on the sourced Titan,
  stop and report the share; do not refit `CH4_NEAR_IR_HALF_BAR` here.
- [ ] **Step 5:** Commit `feat(science): port the deck, the haze and the
  methane anti-greenhouse`.

### Task 7: The budget, its cache and the cloud albedo

**Files:**

- Create: `crates/science/src/radiation/albedo.rs`, `albedo_tests.rs`
- Port from: `alb` 301-313 (`EARTH_CLOUD_COVER` is not ported: the
  fit uses the law's cover), 652-754 (`paths_through`, the
  non-cover part of `shortwave_budget`), tests `alb` 949, 1020, 1043,
  1123, 1162, 1193, 1229 rewritten over a ground albedo

**Interfaces:**

- `#[derive(Clone, Copy, Debug, PartialEq)] pub struct Underneath {
  pub ground: Fraction, pub clouds: Fraction, pub deck: Fraction }`:
  the ground albedo, the water clouds' cover, the deck's share of the
  disc. (Name left to the implementer if a better one fits the crate.)
- `#[derive(Clone, Copy, Debug, PartialEq)] pub struct ShortwaveBudget {
  pub bond_albedo: Fraction, pub absorbed_by_air: Fraction }`.
- `#[derive(Default)] pub struct ShortwaveCache` (manual `Debug`): a
  ring of `REMEMBERED_AIRS` = 32 (`Definition:` the legacy memo's size)
  `(AirKey, Paths)` with `AirKey = [u64; 10]` (seven pressures,
  surface, gravity, star, `to_bits`), oldest replaced first, allocated
  on first use; plus the last `StarBands`.
- `pub fn shortwave_budget(gases: &PartialPressures, surface: Kelvin,
  gravity: MetrePerSecondSquared, star: Kelvin, underneath: Underneath,
  cache: &mut ShortwaveCache) -> ShortwaveBudget`: legacy order (full
  deck returns `DECK_ALBEDO`, air 0; else paths from `haze`,
  `rayleigh_depth_550`, `column::levels(.., Profile::SHORTWAVE)`, then
  `budget(ground, clouds × CLOUD_ALBEDO)`, then the deck mix).
- `CLOUD_ALBEDO`: `Calibration:` refitted in Step 5.
- Earth inputs (test_points, `Source:`): ground albedo 0.122 (CERES
  EBAF Ed4 clear-sky, 29.8/243.8, Kato et al. 2018 Table 5); air
  `EARTH_AIR` with `MANABE_HUMIDITY` of the IAPWS saturation at
  `EARTH_SURFACE_K`; clouds `clouds::cloud_cover` of that air (the law
  2d will call, one code path); `STANDARD_GRAVITY`;
  `SOLAR_EFFECTIVE_TEMPERATURE`; incoming 340.25 W/m² = `SOLAR_IRRADIANCE
  / 4` (`Definition:`).

- [ ] **Step 1: Write the failing tests** (in `albedo_tests.rs`):
  - `earths_clear_sky_albedo_is_observed_without_aerosols`, a **stop
    test**: clouds 0, deck 0,
    Earth ground and air: `bond_albedo` within the clear-sky window
    the user approved after Task 1b (the observed 0.149-0.156 of
    Stephens et al. 2015 and Kato et al. 2018 includes aerosols; the
    aerosol-free window Task 1b derives is about 0.017 lower).
  - `earths_clear_sky_air_absorbs_what_satellites_see` — **stop test**:
    `absorbed_by_air × 340.25` within the H₂O + CO₂ window Task 1b
    writes on the page, with its source in the test's doc (or the
    reference the user chose if Task 1b stopped). The observed 72 ± 3
    W/m² (Kim & Ramanathan 2008; CERES 73.0) includes ozone and
    aerosols, which the model does not carry. Dry air (no H₂O,
    no CO₂) absorbs
    `< 1e-6` and reflects more.
  - `earth_keeps_its_bond_albedo`: all-sky, `|a − 0.293| <= 0.0015`
    (Stephens et al. 2015 0.293; Kato et al. 2018 0.2915).
  - `an_m_stars_albedo_drops_more_than_a_g_stars` (legacy alb 1193 over
    ground 0.122; 3300 K vs 5772 K: drop larger, air absorbs more;
    Kopparapu et al. 2013 §4.1).
  - `thick_co2_scatters_more_light` (legacy alb 949 over ground 0.122:
    1e-3 < 8 < 90 bar of CO₂ at 320 K, all < 1).
  - `titans_air_is_brighter_than_earths` (legacy part of alb 1020:
    Titan's sourced air over ground 0.122 > Earth's + 0.05, clouds 0).
  - `no_air_returns_the_ground` (Review Focus 2: every pressure 0, then
    `-1e-12`: albedo = ground to 1e-12, absorbed 0; with a cloud
    reflectance `c` over a ground `g`, the adding of a non-absorbing
    layer, `c + (1 − c)² g / (1 − c g)`, to 1e-12).
  - `a_full_deck_hides_everything` and `the_budget_stays_in_bounds`
    (Review Focus 4-5: deck 1; ground 0 and 1; clouds 1; Venus 92 bar
    CO₂ + steam at 737 K: `0 <= albedo`, `0 <= air <= 1 − albedo`).
  - `a_cached_budget_is_the_fresh_one` (Review Focus 3: same inputs
    twice through one cache, then a new cache: bit-identical; after 40
    distinct airs, the first is still bit-identical).
  - `the_shortwave_column_is_converged` (legacy alb 1162 with
    `Profile::SHORTWAVE.on(..).up_to(..)`; same tolerances 1e-3 and
    1 W/m², ground 0.122, Earth's cloud cover from the law).
  - The star 0 K or NaN gives the Sun's budget (Review Focus 1).
- [ ] **Step 2:** Run `cargo nextest run -p science radiation::albedo`.
  Expected: FAIL.
- [ ] **Step 3:** Implement with a provisional `CLOUD_ALBEDO` (the
  legacy 0.252).
- [ ] **Step 4:** Run `earths_clear_sky_albedo_is_observed_without_aerosols` and
  `earths_clear_sky_air_absorbs_what_satellites_see`. If either fails,
  stop: report the clear-sky albedo, the absorbed W/m² and the
  Rayleigh, H₂O and CO₂ shares (budgets with each gas removed) to the
  main session. Do not fit anything to pass them.
- [ ] **Step 5:** Fit `CLOUD_ALBEDO` by bisection (a scratch test, not
  committed) so that Earth's all-sky Bond albedo is 0.293; round to
  three digits; write its `Calibration:` doc (target, inputs and their
  sources, method, the legacy 0.252 fitted on 0.30 over the legacy
  cover, and that the fit absorbs what the model lacks: the aerosols'
  clear-sky reflection, about +0.017, and ozone's absorption, #653) and
  the reference page's calibration line. Record Earth's
  cloud cover from the law and the `THIN` merge effect (Bond with and
  without merging) for the PR description.
- [ ] **Step 6:** Run `cargo nextest run -p science radiation::`.
  Expected: PASS; note the slowest test, move any over 10 s to tier 3.
- [ ] **Step 7:** Commit `feat(science): port the shortwave budget and
  refit the cloud albedo`.

### Task 8: Documentation and CI

**Files:**

- Modify: `crates/science/README.md` (the shortwave modules and
  `shortwave.bin` in "Radiation"; stay at 150 lines),
  `docs/science/radiative-transfer.md` ("How it computes them" items 3-4
  l.443-455, "Where the code lives" l.486-516 with the new modules and
  the cache, the calibrations l.563-585, the changes l.624-640),
  `crates/science/data/OPACITY.md` ("Shortwave table" l.243: paths in
  `crates/science`), `docs/science/planet-atmospheres.md` (replace the
  shortwave and albedo-of-the-air paragraphs with a link to the
  reference page; keep the surface-cover prose for 2d; list the
  paragraphs moved in the commit body)

- [ ] **Step 1:** Write the docs.
- [ ] **Step 2:** Run `mise run ci`. Expected: all green, tier 1 within
  120 s.
- [ ] **Step 3:** Commit `docs(science): document the shortwave and the
  atmosphere's albedo`.

### Before the PR (main session)

- `branch-reviewer` and `test-reviewer` once on the whole branch.
- The PR description: the calibrations under their own heading
  (`CLOUD_ALBEDO` with its fit, `HAZE_*`, `DECK_*`, `THIN`,
  `CH4_NEAR_IR_HALF_BAR`), the clear-sky values of Task 7 Step 4, the
  Earth cloud cover, the windows the user approved after Task 1b,
  `Refs #602`, and #653 for the absorbers left out.
