# Planet model PR 2b-1: the longwave column Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port the legacy HITRAN correlated-k longwave column into
`science`, with the IAPWS water saturation, its tables, its generator
and its provenance.

**Architecture:** A new module tree `science::radiation` holds the
spectral absorbers, the k-table loader and the column. Every law takes
plain `f64`/`f32` in its loops and typed units at its public boundary.
The column's memo and its single-precision workspace live in a
`ColumnCache` that the caller owns, so `science` keeps no hidden
state. The water saturation joins `science::water`, and the CO₂ and
methane saturation laws go in a new `science::volatiles`.

**Tech Stack:** Rust (workspace lints, clippy denies file I/O in the
domain crates), `include_bytes!`/`include_str!` for the data,
`cargo nextest` tiers, the `tools/opacity-table` generator (std only).

**Spec:**
`docs/superpowers/specs/2026-11-22-planet-model-pr2b-radiative-design.md`.
Reference page: `docs/science/radiative-transfer.md`. Audit with every
citation to use: Wiki study 19,
<https://github.com/cyboooooorg/map-generator/wiki/19-radiative-transfer-audit>.

## Global Constraints

- Legacy is read only at tag `legacy-final`
  (`git show legacy-final:crates/mapgen/src/planet/<file>`); every law
  is ported from it with its tests, never rewritten from memory.
- Every numeric `const`/`static` in `crates/science` (tests included)
  has a doc comment whose first line starts with `Source:`,
  `Calibration:` or `Definition:`; the guardrail test
  `science_constants_are_sourced` fails otherwise. The citation is the
  one the audit (Wiki study 19) checked, not the legacy comment. No
  "from memory".
- No `thread_local!`, no `HashMap`, no file, network, clock or process
  in `science` (root `clippy.toml`).
- Data files are copied byte for byte from `legacy-final`; their
  SHA-256 at `legacy-final`: `opacity.bin`
  `d5645512…ae7`, `opacity-halved.bin` `497afef8…c6ab1` (full hashes:
  the Wiki study, provenance section; check with `shasum -a 256`).
- A stop test (Task 8 plateau, any constant with neither source nor
  defensible calibration) stops the task and goes to the main session;
  no test is `#[ignore]`d to get past it.
- `crates/science/README.md` stays at 150 lines or fewer (guardrail
  `readmes_stay_short`; it is at 150 now).
- `ENGINE_VERSION` is not bumped: nothing outside `science` calls the
  new laws in this PR.
- Before each commit: `cargo fmt`, `cargo clippy -p science -p
  opacity-table --all-targets -- -D warnings`, the task's tests. Before
  the PR: `mise run ci`.
- Commits follow Conventional Commits with the session's attribution
  trailers; scope `science` or `opacity-table`.

## Review Focus

1. A surface above water's critical point (647.096 K, up to the
   legacy steam cases at 1500 K): the saturation must not return NaN
   or infinity; the column must stay finite (pin in Task 2 and Task 8).
2. A surface below 1 K or a skin temperature of 0 K (airless and
   frozen bodies): finite output, black-body limit (Task 8, legacy
   test 5 covers 250 K/skin 0; add 1 K).
3. A gas with a negative or NaN partial pressure from a caller's
   rounding: treated as none, as the legacy `max(0.0)` does (Task 3
   pins `PartialPressures::total` and the column on `-1e-12`).
4. A cache hit must return the bit-identical value of a fresh column,
   and a full cache (over 1 024 entries) must still return correct
   values (Task 8).
5. Pressures past the table's grid (above 100 bar, Venus at 92 bar CO₂
   plus steam at 70 bar): the k scaling as `p` beyond the grid (legacy
   `lines_broaden_with_pressure_past_the_grid`, Task 6).

---

### Task 1: Rebase check and constants

The branch is already rebased on `origin/main` (d77874e6).

**Files:**

- Modify: `crates/science/src/constants.rs`
- Test: `crates/science/src/constants.rs` (existing inline tests, if any;
  otherwise the guardrail)

**Interfaces:**

- Produces in `science::constants` (all `f64`, each `Source:` CODATA
  2018 / SI 2019 exact or `Definition:`):
  `BOLTZMANN` 1.380_649e-23 J/K, `AVOGADRO` 6.022_140_76e23 /mol,
  `MOLAR_GAS_CONSTANT` 8.314_462_618 J/(mol·K) (k·N_A, exact),
  `SECOND_RADIATION_CONSTANT_CM_K` 1.438_776_877 cm·K (h c / k, exact),
  `STANDARD_GRAVITY` = `MetrePerSecondSquared(9.806_65)` (CGPM 1901,
  definition; replaces the legacy 9.81).

- [ ] **Step 1:** Add the five constants with their doc comments.
- [ ] **Step 2:** Run `cargo nextest run -p guardrails
  science_constants_are_sourced`. Expected: PASS.
- [ ] **Step 3:** Commit `feat(science): add the radiation constants`.

### Task 2: IAPWS water saturation and sublimation

**Files:**

- Modify: `crates/science/src/water.rs` (new section after the melting
  curves; inline `mod tests` at the end, as the file does)

**Interfaces:**

- Produces:
  - `pub fn saturation_pressure(t: Kelvin) -> Pascal`: Wagner & Pruß
    1993 (doi:10.1063/1.555926, IAPWS SR1-86(1992)) eq. for the vapour
    pressure over liquid, 273.16 K to 647.096 K.
  - `pub fn sublimation_pressure(t: Kelvin) -> Pascal`: Wagner et al.
    2011 (doi:10.1063/1.3657937, IAPWS R14-08(2011)) over ice Ih, below
    273.16 K.
  - `pub fn vapour_saturation_pressure(t: Kelvin) -> Pascal`: the
    sublimation curve below the triple point, the saturation curve up
    to the critical point, and the critical pressure (22.064 MPa,
    IAPWS) above it (the legacy clamp at the critical point).
  - `pub(crate) fn vapour_saturation_bar(t_k: f64) -> f64`: the one
    implementation, on plain numbers; the typed functions wrap it.
  - `pub const CRITICAL_PRESSURE: Pascal` (22.064 MPa, IAPWS R6-95)
    and make the existing `CRITICAL_TEMPERATURE` public as
    `pub const CRITICAL_TEMPERATURE: Kelvin`.

- [ ] **Step 1: Write the failing tests** `saturation_matches_iapws`,
  `sublimation_matches_iapws`, `vapour_saturation_is_continuous`:
  - the check values tabulated in IAPWS SR1-86(1992) and R14-08(2011)
    (copy them from the releases; each test constant `Source:` with its
    table), to their published digits;
  - `saturation_pressure(TRIPLE_POINT_TEMPERATURE)` within 1e-4
    relative of 611.657 Pa, and both curves meet there within 1e-4;
  - at the critical temperature, 22.064 MPa within 1e-4;
  - `vapour_saturation_pressure` at 700 K, 1500 K and 3000 K equals
    `CRITICAL_PRESSURE` and is finite; at 1 K it is finite and >= 0.
- [ ] **Step 2:** Run `cargo nextest run -p science water::`. Expected: the new
  tests FAIL (functions missing).
- [ ] **Step 3:** Implement the functions above.
- [ ] **Step 4:** Run the same command. Expected: PASS.
- [ ] **Step 5:** Commit `feat(science): add the IAPWS water saturation curves`.

### Task 3: Partial pressures, CO₂ and methane saturation

**Files:**

- Create: `crates/science/src/radiation/mod.rs`,
  `crates/science/src/radiation/gases.rs`,
  `crates/science/src/volatiles.rs` (+ `volatiles_tests.rs`)
- Modify: `crates/science/src/lib.rs` (`pub mod radiation; pub mod volatiles;`)

**Interfaces:**

- Produces in `science::radiation::gases`:
  - `#[derive(Clone, Copy, Debug, Default, PartialEq)] pub struct
    PartialPressures { pub n2: f64, pub co2: f64, pub h2o: f64, pub ch4: f64,
    pub nh3: f64, pub so2: f64, pub o2: f64 }`
    (bar), with `pub fn total(&self) -> f64` summing the parts clamped
    at zero (Review Focus 3).
  - Molar masses (kg/mol, `Source:` IUPAC 2021 standard atomic weights;
    legacy `spectrum.rs` 163-169) and heat capacities at 298.15 K
    (J/(mol·K), `Source:` NIST WebBook Shomate / Chase 1998; legacy
    `column.rs` 71-76 with NH₃ **35.65**), one `pub const` per gas,
    names `N2_MOLAR_MASS`, …, `N2_HEAT_CAPACITY`, ….
- Produces in `science::volatiles` (ported from
  `legacy-final:…/atmosphere.rs` 595-960):
  - `pub fn co2_saturation_pressure(t: Kelvin) -> Pascal` and plain
    `pub(crate) fn co2_saturation_bar(t_k: f64) -> f64` (frost below
    216.6 K, liquid to 304.13 K, infinite above, as legacy).
  - `pub(crate) fn co2_latent_j_mol(t_k: f64) -> f64`.
  - `pub fn methane_saturation_pressure(t: Kelvin) -> Pascal`,
    `pub(crate) fn methane_saturation_bar(t_k: f64) -> f64`.
  - Constants with the audit's citations: CO₂ triple 216.6 K /
    5.185 bar, critical 304.13 K / 73.77 bar, sublimation 26.1 kJ/mol
    (NIST); CH₄ triple 90.7 K / 0.117 bar, boiling 111.67 K, critical
    190.56 K; the CH₄ sublimation enthalpy: the audit found no NIST
    entry at 9.1 kJ/mol, so take a NIST-listed value (9.2 to 10.0
    kJ/mol range) with its exact citation, or stop.

- [ ] **Step 1: Write the failing tests** in `volatiles_tests.rs`:
  `co2_curve_matches_nist` (the curve within 0.6 % of NIST/Span–Wagner
  1996 at 220, 250, 273.15 and 300 K, values copied from the NIST
  WebBook fluid table with `Source:`),
  `co2_caps_at_its_triple_and_critical_points`,
  `methane_boils_at_one_atmosphere` (101 325 Pa at 111.67 K within
  1 %), and `partial_pressures_ignore_negative_parts`
  (`total()` of `{co2: 1.0, h2o: -1e-12, ..}` is 1.0).
- [ ] **Step 2:** Run `cargo nextest run -p science volatiles::
  radiation::gases`. Expected: FAIL.
- [ ] **Step 3:** Port the laws; set `R` to `constants::MOLAR_GAS_CONSTANT` (the
  legacy atmosphere used 8.314).
- [ ] **Step 4:** Run the same command. Expected: PASS.
- [ ] **Step 5:** Commit `feat(science): add the partial pressures and the CO2
  and methane saturation`.

### Task 4: k-table loader, fast math and the data files

**Files:**

- Create: `crates/science/src/radiation/fast_math.rs`,
  `crates/science/src/radiation/ktable.rs`,
  `crates/science/data/opacity.bin`, `crates/science/data/opacity-halved.bin`,
  `crates/science/data/OPACITY.md`
- Port from: `legacy-final:crates/mapgen/src/planet/{fast_math,ktable}.rs`,
  `legacy-final:crates/mapgen/data/{opacity.bin,opacity-halved.bin,OPACITY.md}`

**Interfaces:**

- Produces `pub(crate) fn exp(x: f32) -> f32`, `pub(crate) fn ln(x: f32) -> f32`
  in `radiation::fast_math` (comment fix: least normal `f32` is
  1.175e-38).
- Produces in `radiation::ktable`: `pub(crate) const G: usize = 8`,
  `LONGWAVE_MAGIC`, `SHORTWAVE_MAGIC`, the four set indices,
  `pub(crate) struct Table` with the legacy fields,
  `pub(crate) fn parse(bytes: &[u8]) -> Option<Table>`, `corners`,
  `lines` (legacy signatures, `Level` from Task 6, so `lines` lands in
  Task 6 if the compiler needs it).
- `OPACITY.md` moves with paths updated to `crates/science/…`, and
  gains: the SHA-256 of the three `.bin` files, the HITRAN release
  (HITRAN2024, Gordon et al. 2026, doi:10.1016/j.jqsrt.2026.109807),
  the generator's commit at which the tables were built (from
  `git log legacy-final -- crates/mapgen/data/opacity.bin`), and a note
  that the line lists cannot be fetched again identically.

- [ ] **Step 1:** Copy the three files with `git show legacy-final:<path> >
  <dest>`; check `shasum -a 256` against the Wiki study.
- [ ] **Step 2: Port the tests** `exp_is_the_librarys`, `ln_is_the_librarys`
  (legacy tolerances 3e-7, 2e-7) and add `the_table_parses`
  (`parse(include_bytes!("../../data/opacity.bin"))` is `Some`, 250 bins, 20
  cm⁻¹, weights sum to 1 within 1e-12).
- [ ] **Step 3:** Run `cargo nextest run -p science radiation::`. Expected:
  FAIL.
- [ ] **Step 4:** Port the code.
- [ ] **Step 5:** Run the same command. Expected: PASS.
- [ ] **Step 6:** Commit `feat(science): load the opacity table`.

### Task 5: The generator writes the continuum and CIA tables

**Files:**

- Modify: `tools/opacity-table/src/continuum.rs`, `src/main.rs`,
  `fetch.sh`, `README.md`
- Create: `tools/opacity-table/src/cia.rs`,
  `crates/science/data/mt-ckd-water.tsv`, `crates/science/data/co2-cia.tsv`

**Interfaces:**

- `WaterContinuum::nodes(&self, step_cm: f64, top_cm: f64) -> Vec<(f64, f64,
  f64, f64)>`:
  the file's points at each multiple of 50 cm⁻¹ from 0 to 5000 (point
  values, not bin means; 101 rows).
- `cia::Cia::parse(text: &str) -> Option<Cia>` (HITRAN `.cia` blocks:
  header with range, count, temperature; then `ν coefficient` rows) and
  `Cia::binned(&self, cell_cm: f64, temperatures: &[f64; 4]) -> Vec<(f64, [f64;
  4])>`:
  the trapezoid mean over each 20 cm⁻¹ cell, from the set nearest each
  of 200/300/450/800 K under the legacy region rules
  (`legacy-final:…/spectrum.rs` 405-421), cells under 1e-50 at every
  temperature left out.
- New flag `--continua` in `main.rs`: writes both TSV files to
  `crates/science/data/` and nothing else.
- TSV format: a `#` header line naming the columns and units, then one
  row per node/cell; `mt-ckd-water.tsv`: `wavenumber_cm self_296K
  exponent foreign_296K`; `co2-cia.tsv`: `cell_start_cm k_200K k_300K
  k_450K k_800K` (cm⁵/molecule²).
- `fetch.sh`: downloads `https://hitran.org/data/CIA/main/CO2-CO2_2024.cia`
  to `raw/`, and checks the SHA-256 of `absco-ref_wv-mt-ckd.nc`,
  `contnm.f90` and the CIA file against the values it records
  (`shasum -a 256 -c`), failing on a mismatch.

- [ ] **Step 1: Write the failing tests** (in the tool):
  `a_cia_block_parses` (a two-block synthetic file),
  `cia_cells_are_trapezoid_means` (a linear coefficient gives its
  midpoint value), `water_nodes_are_the_files_points` (synthetic
  points at 10 cm⁻¹, nodes at 50).
- [ ] **Step 2:** Run `cargo nextest run -p opacity-table`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run the same command. Expected: PASS.
- [ ] **Step 5:** Run `tools/opacity-table/fetch.sh` (needs network; the main
  session asks the user if it is not allowed), then `cargo run --release -j 4 -p
  opacity-table -- --continua`.
  Expected: `mt-ckd-water.tsv` equals the legacy `MT_CKD` array to its
  four printed digits on all 101 rows; `co2-cia.tsv` matches the legacy
  `CO2_CIA` within 0.2 % on at least 109 of 139 rows (the audit's
  figure), the rest being edge cells under 1e-46. Write the comparison
  in the task report.
- [ ] **Step 6:** Update the tool's README: every `crates/mapgen` path to
  `crates/science`, the `--continua` flag, the checksum check.
- [ ] **Step 7:** Commit `feat(opacity-table): write the water continuum and CO2
  CIA tables`.

### Task 6: Spectrum and the methane stand-in

**Files:**

- Create: `crates/science/src/radiation/spectrum.rs`,
  `spectrum_tests.rs`, `spectrum_reference.rs` (`#[cfg(test)]`),
  `crates/science/src/radiation/methane.rs`
- Port from: `legacy-final:…/spectrum.rs`, `spectrum/reference.rs`,
  `climate.rs` 150-215 (`CH4_OPACITY`, `CH4_SATURATION_BAR`,
  `methane_depth`)

**Interfaces:**

- `radiation::spectrum`: `pub(crate) struct Level { pub temperature_k: f64, pub
  gases: PartialPressures, pub gravity_m_s2: f64 }`
  and every legacy `pub(crate)` item with its legacy signature (`BINS`,
  `BANDS`, `TAIL`, `POINTS`, `DIFFUSIVITY`, `FastLevel`, `fast_level`,
  `smooth_above`, `planck_spectrum`, `planck_spectrum_upto`,
  `co2_pairs`, the continuum depths).
- `MT_CKD` and `CO2_CIA` are parsed from
  `include_str!("../../data/mt-ckd-water.tsv")` and `co2-cia.tsv` into
  `LazyLock` tables (the crate's TSV pattern, `giants.rs:20`).
- `OPACITY: LazyLock<Table>` from `include_bytes!("../../data/opacity.bin")`.
- `radiation::methane::depth(gases: &PartialPressures) -> (f64, f64)`
  (legacy `methane_depth`); `CH4_OPACITY` 13.4 and
  `CH4_SATURATION_BAR` 0.01 are `Calibration:` with the audit's record
  (Titan surface, Fulchignoni et al. 2005; McKay et al. 1991;
  Niemann et al. 2010 for 5.65 %).
- Citations to fix while porting (audit): CIA file documented by
  HITRAN2024 (Terragni et al.) with its component sources; the
  g-split is `Calibration:` (Lacis & Oinas 1991 give the method only);
  `DIFFUSIVITY` cites Elsasser 1942 and Koll et al. 2023 Table 1.

- [ ] **Step 1: Port the 11 legacy tests** with their tolerances, the
  test helpers (`earth_profile`, `earth_air`, `doubling_forcing`) using
  `water::vapour_saturation_bar` and `STANDARD_GRAVITY`, and:
  - `earth_clear_sky_olr`: the anchor becomes CERES EBAF clear-sky OLR
    (Kato et al. 2018, doi:10.1175/JCLI-D-17-0523.1, the value and
    window the audit gives, 264 to 268 W/m²), `Source:` on the constant;
  - `doubling_co2_forces_myhre`: search for a published instantaneous
    clear-sky top-of-atmosphere forcing of a CO₂ doubling for a
    standard profile (for example the line-by-line results of
    RTMIP, Collins et al. 2006, or RFMIP, Pincus et al. 2016). If found,
    use it with its tolerance; if not, keep 3.7 ± 0.6 with a doc saying
    it is a plausibility band, and report it for the PR's calibration
    list;
  - the fixture's 77 % surface humidity cites Manabe & Wetherald 1967
    with the indirect confirmation the audit names, or is a
    `Definition:` of the test profile.
- [ ] **Step 2:** Run `cargo nextest run -p science radiation::spectrum`.
  Expected: FAIL.
- [ ] **Step 3:** Port the code (Review Focus 5: keep the `p`-scaling past 100
  bar).
- [ ] **Step 4:** Run the same command. Expected: PASS. If a ported test moves
  past its tolerance because of the IAPWS humidity, report the values and stop.
- [ ] **Step 5:** Commit `feat(science): port the spectral absorbers`.

### Task 7: Cloud cover

**Files:**

- Create: `crates/science/src/radiation/clouds.rs`
- Port from: `legacy-final:…/albedo.rs` 272-300 and 565-575

**Interfaces:**

- `pub fn cloud_cover(surface: Kelvin, h2o: Bar) -> Fraction` and plain
  `pub(crate) fn cloud_cover_share(t_k: f64, h2o_bar: f64) -> f64`
  (humidity from `water::vapour_saturation_bar`).
- `CLOUD_HUMIDITY_EXPONENT` 0.25 (`Source:` Xu & Randall 1996),
  `CLOUD_H2O_BAR` 1.0e-2 (`Calibration:` on Earth's cover 0.68 ± 0.03,
  Stubenrauch et al. 2013; MODIS 0.67, King et al. 2013; what was
  searched, per the reference page).

- [ ] **Step 1: Write the failing tests** `a_dry_air_has_no_cloud`
  (h2o 0 gives 0), `earth_is_two_thirds_cloudy` (288 K, 77 % of the
  saturation: within 0.64 to 0.71, the Stubenrauch range), and
  `cloud_cover_depends_on_more_than_vapour` (legacy test: same vapour,
  warmer air, less cover).
- [ ] **Step 2:** Run `cargo nextest run -p science radiation::clouds`.
  Expected: FAIL.
- [ ] **Step 3:** Port.
- [ ] **Step 4:** Run the same command. Expected: PASS. If Earth leaves the
  range with the IAPWS curve, report and stop (the calibration moves).
- [ ] **Step 5:** Commit `feat(science): port the cloud cover`.

### Task 8: The column

**Files:**

- Create: `crates/science/src/radiation/column.rs`, `column_tests.rs`
- Port from: `legacy-final:…/column.rs` (all), `climate.rs` 277
  (`tropopause_k`), 332 (`STEAM_MAX_BAR`), 412-470 (test helpers)

**Interfaces:**

- `#[derive(Clone, Copy, Debug, PartialEq)] pub struct Profile` with
  the legacy fields and consts `CLIMATE`, `SATURATED`, `PLATEAU`,
  `SHORTWAVE` (and `CLEAR`, `on`, `up_to` under `#[cfg(test)]`).
- `pub struct ColumnCache` (`Default`): the legacy `Memory` (1 024
  entries, key `[u64; 14]`, oldest replaced first) and the boxed
  `Scratch` (about 101 kB), allocated once on first use.
- `pub fn outgoing_longwave(gases: &PartialPressures, surface: Kelvin, skin:
  Kelvin, gravity: MetrePerSecondSquared, profile: Profile, cache: &mut
  ColumnCache) -> WattPerSquareMetre`
  (legacy `olr_w_m2`; the memo key uses `gravity.0`).
- `pub fn skin_temperature(effective: Kelvin) -> Kelvin`: `T_eff / 2^(1/4)`,
  `Source:` Pierrehumbert 2010 §3.6 (legacy `tropopause_k`).
- `pub(crate) struct Levels` and
  `pub(crate) fn levels(gases: &PartialPressures, surface_k: f64, gravity_m_s2:
  f64, profile: Profile) -> Option<Levels>`
  (2b-2's entry).
- `pub(crate) fn stratospheric_methane_bar(gases: &PartialPressures, skin_k:
  f64) -> f64`.
- Constants: `STEAM_MAX_BAR` 70, the grid ones, `OPAQUE`, `CLOUD_TOP_SHARE`
  (Rossow & Schiffer 1999 Table 4 580 mb over Trenberth & Smith 2005
  985.50 hPa, both confirmed), `CLOUD_IR_DEPTH` 3.8/2, `TROPOPAUSE_BAR`
  0.1 (Robinson & Catling 2014): citations from the audit; the
  relative humidity cites Manabe & Wetherald 1967 eq. 2 with the
  audit's indirect confirmation and no longer credits Kopparapu; the
  CO₂ moist adiabat cites Kasting 1991 App. B as followed by Kopparapu
  et al. 2013 §2.

- [ ] **Step 1: Port the 7 legacy tests** with the test helpers
  (`EARTH_AIR`, `with_vapour` at 77 %, `plateau_w_m2` with its 8
  fixed-point iterations, the `TITAN`/`VENUS`/`MARS` gravities as
  `Source:` test constants) and add:
  - `a_cached_column_is_the_fresh_one`: the same inputs through one
    cache twice, then through a new cache, give bit-identical results;
    after 1 100 distinct columns, the first one recomputed is still
    bit-identical (Review Focus 4);
  - `extreme_surfaces_stay_finite`: surfaces of 1 K and 3000 K with a
    skin of 0 K, steam 70 bar plus CO₂ 6 bar, give finite positive
    OLR below `σT⁴` (Review Focus 1-2);
  - `a_negative_rounding_is_no_gas`: `h2o: -1e-12` gives the same OLR
    as `h2o: 0.0` (Review Focus 3).
- [ ] **Step 2:** Run `cargo nextest run -p science radiation::column`.
  Expected: FAIL.
- [ ] **Step 3:** Port the code, the memo and scratch into `ColumnCache`.
- [ ] **Step 4:** Run the same command. Expected: PASS.
  **Stop test:** if `the_moist_plateau_is_goldblatts` (282 ± 14),
  `the_runaway_anchor_is_flat` (1 %, peak ≤ 6 %) or
  `the_plateau_ignores_co2_and_nitrogen` (3 %) fails with the IAPWS
  saturation, stop and report the values; do not widen a tolerance.
  Either way, record the plateau and the runaway values before (legacy
  binary or the legacy curve swapped in temporarily) and after, for
  the PR description.
- [ ] **Step 5:** Measure: tier-1 time of `radiation::` tests
  (`cargo nextest run -p science radiation::`); any test over 10 s moves
  to tier 3 with `#[ignore = "heavy: <why>"]`.
- [ ] **Step 6:** Commit `feat(science): port the longwave column`.

### Task 9: Speed against the legacy column

**Files:** none committed (a scratch benchmark).

- [ ] **Step 1:** In a scratch crate outside the repository, time 1 000
  distinct columns (the nine cases of
  `the_fast_radiation_is_the_reference`, swept in surface temperature)
  through `legacy-final`'s `olr_w_m2` and through
  `outgoing_longwave` with a fresh cache, release build, same machine,
  median of 5 runs (method of the `generation-profiler` agent).
- [ ] **Step 2:** Expected: the new column costs no more than 1.10 times
  the legacy one. Otherwise report the profile to the main session
  before going on.

### Task 10: Documentation and CI

**Files:**

- Modify: `crates/science/README.md` (a "Radiation" section; trim to
  stay at 150 lines or fewer), `docs/science/radiative-transfer.md`
  ("In the app": module paths, the cache, what 2b-1 ships and what
  2b-2 will), `docs/science/planet-atmospheres.md` (replace the column
  paragraphs of "Climate and surface liquid" l.808-989 with a link to
  the reference page; keep the climate prose)

- [ ] **Step 1:** Write the docs.
- [ ] **Step 2:** Run `mise run ci`. Expected: all green, tier 1 within 120 s.
- [ ] **Step 3:** Commit `docs(science): document the longwave column`.
