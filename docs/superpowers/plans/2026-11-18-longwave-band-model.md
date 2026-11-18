# Longwave band model Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the grey optical depths of the climate column with a simple spectral model (CO₂ band with exponential wings, H₂O bands and continuum, HITRAN CO₂ CIA) so that Earth's CO₂, the outer edge, the moist plateau, Mars and Titan hold together. This closes #523.

**Architecture:** A new module `crates/mapgen/src/planet/spectrum.rs` holds the bins, the Planck function per bin and the absorption coefficients per gas and level, with cited sources. `column.rs` keeps its temperature profile and replaces its grey two-stream sum by a per-bin sum. `climate.rs` drops the grey laws and re-measures every bound.

**Tech Stack:** Rust (mapgen crate), census examples, generation-profiler.

**Spec:** `docs/superpowers/specs/2026-11-18-longwave-band-model-design.md`, an addendum to `docs/superpowers/specs/2026-11-18-climate-pass-lot3c2-design.md`. This plan replaces Task 7 of `docs/superpowers/plans/2026-11-18-climate-pass-lot3c2.md`. Tasks 8-12 of that plan follow afterwards.

## Global Constraints

- Band parameters take the papers' values and move only inside their stated uncertainty, each choice written down. A missed check stops the work and goes to the user. Never fix a miss with a fitted constant.
- Priority when bounds conflict:
  1. radiation checks (doubling forcing, Earth OLR, plateau, anchor);
  2. Earth;
  3. outer edge and K/M;
  4. Mars and Titan;
  5. Venus, which is measured only.
- Lorentz broadening `κ ∝ p / p_ref`, applied per level. CIA scales as density², with no broadening.
- Spectral range 0-5000 cm⁻¹. Bin width at most 20 cm⁻¹; halving it moves the doubling forcing by under 0.1 W/m². Each bin uses the layer-mean Planck function with diffusivity 1.66.
- CH₄ keeps its grey Titan stand-in over 0-650 cm⁻¹ only.
- Gravity enters through `Planet` (column masses `p / g`).
- Opus for 7.1-7.3, sonnet for 7.4. Shared machine: `cargo … -j 4`, tests `--test-threads 4`, censuses `--release`. Never run `npm ci`.
- Commits: Conventional Commits. 7.3 carries `Closes: #523`.

## Review Focus

1. A column with no CO₂ and no H₂O (Titan, Triton, airless) must still give the right OLR from CH₄ or the bare surface. Test in 7.2.
2. A steam column at 640 K must have its Planck peak inside the range, and the 4 µm window must not be lost at the 5000 cm⁻¹ edge. Test in 7.2.
3. Mars-thin CO₂ (6 mbar) must not be warmed by wings that only open at Earth pressure. Test in 7.3 (Mars 210 ± 5 K).
4. Venus at 92 bar must not overflow or hit NaN in the CIA or the Lorentz scaling. Test in 7.2.
5. Explain replays the same column. Test in 7.3: no `explain` refusal over the census archetypes.

---

### Task 7.1: Spectral coefficients and radiation-only checks

**Files:**
- Create: `crates/mapgen/src/planet/spectrum.rs` (registered in `planet/mod.rs`)
- Test: in `spectrum.rs`

**Interfaces:**
- Produces:
  - `pub(crate) const BINS: usize` and `pub(crate) fn bin_edges(i) -> (f64, f64)` in cm⁻¹;
  - `pub(crate) fn planck_bin_w_m2(i: usize, t_k: f64) -> f64`, the hemispheric flux in the bin;
  - `pub(crate) fn absorption_per_ln_p(i: usize, level: &Level) -> f64`, the optical depth per unit ln p contributed by every gas in bin `i`. `Level` carries p, T, the partial pressures and `g`; reuse the column's level type if it fits.
  - A test helper `fn olr_fixed_profile(profile, gases) -> f64` that integrates a given profile.

- [ ] **Step 0: Clean the worktree.** Save the uncommitted round-2 work: run `git diff > .superpowers/sdd/2026-11-18-climate-pass-lot3c2/task-7-round2-final.diff`, then `git checkout -- crates/`. Keep the HITRAN CIA derivation notes from `task-7-report.md`.
- [ ] **Step 1: Write the failing tests.** Each runs on a fixed US-standard-like Earth profile (288 K surface, 6.5 K/km to 217 K, isothermal above), at 77 % humidity unless stated:
  - `doubling_co2_forces_myhre`: 280 → 560 ppm reduces the OLR by 3.7 ± 0.6 W/m².
  - `co2_forcing_is_logarithmic`: the forcing per doubling over 70 → 140, 280 → 560 and 1000 → 2000 ppm stays within 15 % of its mean.
  - `earth_clear_sky_olr`: 265 ± 10 W/m².
  - `halving_bins_keeps_the_forcing`: the doubling forcing moves by under 0.1 W/m² when the bin width is halved. Use a test-only bin width parameter.
  - `planck_bins_sum_to_sigma_t4`: the sum over bins equals σT⁴ within 0.5 % at 100, 288 and 640 K.
- [ ] **Step 2: Run** `cargo test -p mapgen spectrum -j 4 -- --test-threads 4` and confirm it FAILS.
- [ ] **Step 3: Implement** the coefficients from the papers.
  - CO₂: band centre 667.5 cm⁻¹ with its exponential-wing strength and decay length (Romps, Seeley & Edman 2022; Jeevanjee et al. 2021).
  - H₂O: the rotation and 6.3 µm bands (Jeevanjee & Fueglistaler 2020), plus the self and foreign continuum.
  - CO₂-CO₂ CIA: binned from HITRAN, Karman et al. 2019, as a cited table.
  - CH₄: the grey stand-in over 0-650 cm⁻¹, with the current `CH4_OPACITY` law.
  Every value carries its source in a doc comment, and every choice inside an uncertainty range is noted.
- [ ] **Step 4: Run** the tests and confirm they PASS. If a radiation check misses outside the papers' range, stop and report the numbers.
- [ ] **Step 5: Commit** `feat(planet): add a simple spectral model of CO2 and water` (`Refs: #523`).

### Task 7.2: Spectral column

**Files:**
- Modify: `crates/mapgen/src/planet/column.rs` (`olr_under_skin_w_m2` :313; the grey sum :433-448; `DIFFUSIVITY` :81), `crates/mapgen/src/planet/climate.rs` (`opacity_per_ln_p` :651, `optical_depth_at` :636, `depths` :617)

**Interfaces:**
- Consumes: `spectrum::*` from 7.1, and `Planet` (gravity).
- Produces: `olr_under_skin_w_m2(gases, surface_k, skin_k, planet: &Planet) -> f64`, now a per-bin sum on the existing profile. `MOIST_PLATEAU_W_M2` is recomputed from it.

- [ ] **Step 1: Write the failing tests:**
  - `the_moist_plateau_is_goldblatts`: 282 ± 14 W/m².
  - `the_plateau_ignores_co2_and_nitrogen`: the plateau moves within 3 % for CO₂ from 4e-4 to 1e-2 bar and N₂ from 1 to 3 times Earth's.
  - `the_runaway_anchor_is_flat`: within 2 % over 380-640 K.
  - `a_column_without_co2_or_water_radiates_from_its_methane_or_ground`: Titan-like and airless cases are finite and below σTs⁴.
  - `venus_column_is_finite`: 92 bar CO₂ at 737 K gives a finite, positive OLR.
- [ ] **Step 2: Run** and confirm FAIL.
- [ ] **Step 3: Implement** the per-bin two-stream on the same levels. Keep `STEAM_MAX_BAR` unless the 4 µm window makes it redundant, and say which in the report.
- [ ] **Step 4: Run** the new tests and the column tests, and confirm PASS. Climate tests may fail at this step; list them in the report for 7.3.
- [ ] **Step 5: Commit** `feat(planet): integrate the climate column over spectral bins` (`Refs: #523`).

### Task 7.3: Climate bounds on the spectral column

**Files:**
- Modify: `crates/mapgen/src/planet/climate.rs` (remove `CO2_OPACITY`, `CO2_EXPONENT`, `CO2_WEAK_BAR`, `H2O_OPACITY`, the broadening knees and `CO2_PAIR_*`; re-derive `SNOWBALL_WARM_BAR` :~1452 on its stated calibration), its tests (the calibration tests listed in the recon: `earth_holds_its_preindustrial_co2`, `doubling_the_nitrogen_warms_a_late_archean_earth_by_a_few_kelvin`, `the_outer_edge_round_the_sun_is_kopparapus`, `the_outer_edge_follows_the_star`, `the_solar_system_has_its_climates`, `the_methane_anti_greenhouse_matches_mckay`, the plateau tests), `explain.rs` if signatures moved, `docs/science/planet-atmospheres.md` (climate section, CO₂ table), `README.md` where it describes the climate.

- [ ] **Step 1:** Update the calibration tests to the lot's bounds. Each checks a physical target, never the old constant:
  - Earth: CO₂ in [1.87e-4, 4.2e-4] bar, surface 286-290 K.
  - Outer edge: 1.60-1.80 AU; K and M within ±15 %.
  - Archean: about 281 K, and doubled N₂ warms it by +3 to +8 K.
  - Mars: 210 ± 5 K. Titan: 93.7 ± 1 K. Triton unchanged.
  - Venus: the bound is set to what the model reaches, with a comment naming the gap. The main session files the issue.
- [ ] **Step 2: Run** the tests and record which ones fail.
- [ ] **Step 3: Implement.** Remove the grey laws. Re-derive `SNOWBALL_WARM_BAR` and explain its change physically. If CH₄ is no longer at 93.7 K, re-anchor `CH4_OPACITY` on Titan (its calibration target). If any bound cannot hold within the papers' uncertainty, stop and report.
- [ ] **Step 4: Run** the full mapgen suite, `archetype_census -- 40` and `planet_census -- 500`, and confirm PASS. In the report, give a before/after table of every bound, the snowball counts and Venus.
- [ ] **Step 5: Commit** `feat(planet)!: take the greenhouse from a spectral column` (`Closes: #523`).

### Task 7.4: Performance

- [ ] **Step 1:** Run `generation-profiler` (agent) to compare the whole-system draw time on the same seeds: main at the branch base (3c47979) against HEAD.
- [ ] **Step 2:** If the cost is over +20 %, cache or tabulate the OLR (keyed on bins of the surface temperature, the skin and the composition). Alternatively, coarsen the bins within the 7.1 halving check. Re-run the profiler and all the checks.
- [ ] **Step 3: Commit** `perf(planet): …` if anything changed.
