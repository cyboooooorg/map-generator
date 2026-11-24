# Planet model PR 2d-2: the surface Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port from `legacy-final` the laws of a planet's surface, solver
free: the phase curves they read (N₂ and SO₂ frost, nitrogen liquid,
ethane and water boiling), the surface cover that gives the ground
albedo, the surface liquids and frost, and the volcanic intensity the
snowball chance of 2d-3 will read.

**Architecture:** The laws go to `science`: the curves in `volatiles` and
`water`, and a new `surface` module (`surface/cover.rs`,
`surface/liquids.rs`) whose functions take a temperature, an air, a
water inventory and a radius. They are tested at observed temperatures
(Earth, Mars, Titan, Triton, Io), without the solver. The only chain
step is the volcanic intensity, recorded by `bare` in
`system::planet`. Nothing is public outside `science`, nothing generated
changes, and there is no engine version bump.

**Tech Stack:** Rust, `cargo nextest` tiers, the `Trace`/`Traced` cause
API of `timeline`, `science::radiation::albedo::shortwave_budget`.

**Spec:** `docs/superpowers/specs/2026-11-23-planet-model-pr2d-design.md`
(sections "2d-2: the surface", "Rules for the build", "Science to study
first", rows 2d-2). Study: Wiki
[24-surface-cover-audit](https://github.com/cyboooooorg/map-generator/wiki/24-surface-cover-audit)
and `docs/science/planet-atmospheres.md` "Climate and surface liquid"
(l.~1264, extended by commit 353575a2; grep the headings, the page is
2 400+ lines). Recon: legacy paths below are under
`legacy-final:crates/mapgen/src/planet/`, `albedo` is `albedo.rs`,
`climate` is `climate.rs`, `atm` is `atmosphere.rs`.

## Decisions of 2026-11-24 (with the user)

- **Study 24 values win over legacy** (rule "the source wins"):
  `LAVA_K` 1673 K (Kite et al. 2016; was 1400 K), `ICE_SHEET_KM` 1.94 km
  (Fretwell et al. 2013, Table 7; was 1.5 km), `REGOLITH_ALBEDO` 0.11
  becomes a `Source:` (the Moon). Zero new calibrations.
- **`FROST_ALBEDO` stays 0.75**, a calibration on Pluto (0.72 ± 0.07,
  Buratti et al. 2017, doi:10.1016/j.icarus.2016.11.012). Its doc states
  the gap to Triton (0.82 ± 0.05, Hillier et al. 1990). The joint refit
  with `N2_ICE_EMISSIVITY` goes to 2d-3's study, which has the solver.
- **Volcanism, option B.** The intensity the snowball chance reads is
  `volcanic_intensity(heat_flux)` of `science::relief::volcanoes`, times
  `LID_SHARE / MOBILE_SHARE` (`science::outgassing`, 0.2 / 0.95) under
  a stagnant lid, 1 otherwise. It uses no new number. Legacy's
  `interim::volcanism` is not ported. 2d-3 divides by
  `EARTH_VOLCANIC_INTENSITY` and refits `SNOWBALL_WARM_BAR`. A heat
  pipe has no special case (legacy set it to 1). The regime classes a
  heat pipe from 1 W/m² (`tectonics::HEAT_PIPE_FLUX`), but the law only
  reaches 1 at 2.5 W/m² (`volcanoes::HEAT_PIPE_FLUX`), so a heat pipe
  reads 0.83 to 1. The mismatch is #670.
- **#656 stays open:** no sourced broadband Titan ground albedo exists
  (study 24; comment on #656).
- **Ice sheets are ice.** The ice reach spreads the water inventory
  as ice: `inventory / (ρ_ice · 4πR² · ICE_SHEET_THICKNESS)`. `ρ_ice`
  is `water::ice_ih_density` at one standard atmosphere (101 325 Pa, a
  `Definition:`) and at `ice_ih_melting_temperature` of that pressure
  (IAPWS, about 917 kg/m³). Legacy used liquid water at 1000 kg/m³
  over Earth's area, scaled by `R⁻²`, and that density had no source.
  The function reads the planet's radius in metres, so no Earth-area
  constant is needed, and `OCEAN_LAYER_KM` is not ported.
- **`condense` is 2d-3's.** `frost` and `liquid` take the condensate
  (the part of N₂ and CH₄ that the air cannot hold) as an argument.
- **The cloud cover stays out of `Cover`.** The caller builds
  `Underneath { ground, clouds, deck }` from `ground_albedo`,
  `radiation::clouds::cloud_cover` and `veils::sulphuric_deck`.
- **One relative humidity.** `surface::RELATIVE_HUMIDITY` (0.77) is the
  public constant. `radiation/test_points.rs`'s `MANABE_HUMIDITY` is
  removed and its users read the public one.
- **Water melts on the IAPWS curve.** The water window is
  `[ice_ih_melting_temperature(P), boiling_temperature(P))`, both from
  `science::water`. Legacy's `ICE_MELTS_K` (273.15 K, uncited) is not
  ported.
- **Names.** Legacy's `_K` and `_BAR` suffixes go, since the typed
  units carry them. The new names are `LAVA_TEMPERATURE`,
  `ICE_LINE_TEMPERATURE`, `ICE_LINE_CONTRAST`,
  `ICE_COVERED_TEMPERATURE`, `SHIELD_PRESSURE`, `FROST_LAYER`,
  `ICE_SHEET_THICKNESS` and `PHASE_BAND`. The lists below give each
  constant's legacy name.

## Global Constraints

- Legacy is read only at tag `legacy-final`. Every law is ported with
  its tests, then changed only where the decisions above say so.
- Every numeric `const`/`static`, and every `let` bound to a bare number
  other than 0, 0.5, 1 or 2, has a doc comment starting with `Source:`,
  `Calibration:` or `Definition:` (guardrail
  `science_constants_are_sourced`, test files included). Prefix, value
  and citation come from study 24 (Wiki page and
  `planet-atmospheres.md`), never from the legacy comment. "From
  memory" and "not checked" notes are not carried over.
- Constants that study 24 does not list and legacy marks "checked" or
  labels a calibration keep that label and citation, re-read in
  `planet-atmospheres.md`:
  - `NITROGEN_RAOULT_SHARE` 0.5 (Thompson, Zollweg & Gabis 1992);
  - `MAX_NITROGEN_IN_LIQUID` 0.3;
  - `METHANE_FUSION_L_OVER_R` (Vogt & Pitzer 1976);
  - `ETHANE_TRIPLE_K` 90.4;
  - `ETHANE_FUSION_L_OVER_R`;
  - `METHANE_EXCESS_W_OVER_R` 75.3 K (calibration);
  - `FROST_LAYER_BAR` 0.02 (calibration);
  - `SO2_FROST_ALBEDO` 0.63 (calibration, see #669);
  - `PHASE_BAND_K` 1 K (a smoothing band: `Calibration:` under the #648
    rule, held by 2d-3's convergence test).

  Any other constant without one of these three tags stops the work.
- Where legacy and the rewrite differ, the rewrite's value holds: the
  methane triple point (`volatiles` 90.6941 K, not 90.67), the water
  curves (IAPWS, not legacy's Clausius-Clapeyron) and `OCEAN_MASS`.
- Missing science stops the work: the implementer reports the stop to
  the main session, which brings it to the user. No test is
  `#[ignore]`d and no tolerance widened to get past a stop. A legacy
  bound that is an observation (Earth's Bond albedo 0.29 to 0.31,
  Earth's ice about a tenth, Titan's seas) stays; if it fails, stop and
  report.
- `science` takes and returns the typed units of `units.rs` and
  `radiation::gases::PartialPressures` at its public boundary, never
  `timeline` types. No `HashMap`, `thread_local!`, file, clock or
  process in `science` or `system`.
- `crates/science/README.md` is at its 150-line cap (guardrail
  `readmes_stay_short`): the new module's paragraph replaces or
  shortens existing text.
- New `system` code stays crate-private under the
  `#[cfg_attr(not(test), expect(dead_code, …))]` of `planet/mod.rs`.
  `crates/system/src/draw/**`, `placement.rs` and `timeline` are not
  touched.
- No new draw. `mise run ci` passes at the end of every task. The
  tier-1 budget is 2 minutes.

## Review Focus

1. **Temperatures outside a fit's range** (N₂ below 10 K, SO₂ at 20 K,
   ethane or water above their critical point): every curve stays
   finite, positive and monotonic. Below the floor it carries the slope
   on; at or above the critical point it caps. Tests:
   `nitrogen_frost_follows_fray_and_schmitt` and
   `boiling_caps_at_the_critical_point` (Task 1).
2. **An empty or dry world:** zero water, zero air and zero frost give
   `land == 1` and no NaN from `so2 / total` or `0 / 0` shares. Tests:
   `bare_rock_is_all_land` and `no_frost_divides_nothing` (Task 2).
3. **Shares sum to 1:** for every cover, `ocean + land + ice + frost ==
   1` within 1e-12, and `so2_frost ≤ frost`. Test:
   `cover_shares_sum_to_one` (Task 2), swept over 0 to 10 oceans, 40 to
   400 K and 0 to 10 bar of frost.
4. **A surface right at a liquid's boundary.** Lava at exactly
   `LAVA_TEMPERATURE`, water at exactly its melting or boiling
   temperature, and an air below the water triple pressure each get one
   defined answer, in the order of legacy's tests. Test:
   `liquid_boundaries_are_inclusive_below` (Task 3).
5. **A stagnant lid or a heat pipe** gets the right regime share, and
   the volcanic node derives from the heat, the radius and the regime.
   Test: `volcanism_reads_the_regime` (Task 4).

---

### Task 1: Phase curves in `volatiles` and `water` (sonnet)

**Files:**

- Modify: `crates/science/src/volatiles.rs`,
  `crates/science/src/volatiles_tests.rs`, `crates/science/src/water.rs`
  (its inline `mod tests`, l.~862). Make `METHANE_TRIPLE_TEMPERATURE`
  `pub(crate)` for Task 3.

**Interfaces:**

- Produces:
  - `volatiles::NITROGEN_TRIPLE_TEMPERATURE: Kelvin` (63.14),
    `NITROGEN_TRIPLE_PRESSURE: Pascal` (12 530),
    `NITROGEN_ALPHA_BETA: Kelvin` (35.61);
  - `pub fn nitrogen_frost_pressure(t: Kelvin) -> Option<Pascal>` (`None`
    at or above the triple point);
  - `pub fn nitrogen_vapour_pressure(t: Kelvin) -> Pascal` (Antoine
    `10^(3.7362 − 264.651/(T − 6.788))` bar, NIST from Edejer & Thodos
    1967, 63.14 to 126 K);
  - `SULPHUR_DIOXIDE_TRIPLE_TEMPERATURE: Kelvin` (197.64),
    `SULPHUR_DIOXIDE_TRIPLE_PRESSURE: Pascal` (1 670);
  - `pub fn sulphur_dioxide_frost_pressure(t: Kelvin) -> Option<Pascal>`;
  - `pub fn ethane_boiling_temperature(p: Pascal) -> Kelvin`, with
    `ETHANE_TRIPLE_TEMPERATURE: Kelvin` (90.4);
  - `water::boiling_temperature(p: Pascal) -> Option<Kelvin>`.

- [ ] **Step 1: Write the failing tests**

  - `nitrogen_frost_follows_fray_and_schmitt`, ported from `atm`
    (~l.1368) with the rewrite's names:
    - the pressure at `63.14 − 1e-9` K is 12 530 Pa within 1e-6
      relative;
    - at 37.2 K it is within 1 % of the β fit scaled by 0.1253/0.1215,
      and within 0.05e-5 bar of 1.4e-5 bar (Triton, Tyler et al. 1989);
    - the ratio of the pressures at `35.61 − 1e-9` K and 35.61 K is 1
      within 1e-6;
    - the curve is strictly decreasing and positive from 63.1 K down to
      2.0 K, in 0.1 K steps;
    - it is `None` at 63.14 K.
  - `nitrogen_vapour_follows_its_antoine_fit`:
    - at 77.355 K it is within 5 % of 101 325 Pa (NIST's normal boiling
      point);
    - it is strictly increasing from 63.14 to 126 K.

    The fit is not pinned to the triple point. Computed by hand, it
    gives 0.967 bar at 77.355 K (4.5 % low) and 0.110 bar at 63.14 K
    (12 % low). Its only reader is `methane_freezes`, at the methane
    triple point (90.69 K). The doc comment states the fit's accuracy.
    If 5 % fails, stop and report. Do not pin or refit it.
  - `sulphur_dioxide_frost_holds_from_io_to_its_triple_point`:
    - the pressure at `197.64 − 1e-9` K is 1 670 Pa within 1e-6
      relative;
    - it is strictly increasing over 10 to 197 K, and finite and
      positive at 10 K;
    - at 115 K it is within a factor of 1.2 of Wagman's
      `1.516e8 exp(−4510/T)` bar (study 24; Fray & Schmitt's fit sits a
      factor of 2 higher, as the page says).
  - `water_boils_at_its_normal_point`:
    - `boiling_temperature(101 325 Pa)` is 373.124 K within 0.01 K
      (IAPWS SR1-86);
    - it is `None` below `TRIPLE_POINT_PRESSURE`;
    - it is `CRITICAL_POINT_TEMPERATURE` at or above
      `CRITICAL_PRESSURE`;
    - `saturation_pressure(boiling_temperature(p))` is `p` within 1e-9
      relative for 1 kPa, 100 kPa and 10 MPa.
  - `boiling_caps_at_the_critical_point`:
    - `ethane_boiling_temperature(101 325 Pa)` is 184.6 K within 0.01;
    - at 1 MPa it is above 184.6 K and at most 305.3 K;
    - at 1 GPa it is 305.3 K;
    - it is increasing in pressure.

- [ ] **Step 2: Run** `cargo nextest run -p science volatiles water`.
  Expected: FAIL, the functions do not exist.

- [ ] **Step 3: Implement.**
  - **N₂:** legacy's `nitrogen_sublimation` (`atm` ~l.655-710). β is
    pinned to the triple point, α is pinned to β at the transition, and
    below 10 K (`NITROGEN_FIT_FLOOR`) the slope at 10 K is carried on.
    Use the coefficients of study 24 (`N2-1`, `N2-2`).
  - **SO₂:** pinned to the triple point with `L/R` = 4510 K (legacy
    `SO2_FROST`, Wagman 1979 via Giles et al. 2024). Use the existing
    `Sublimation::ConstantEnthalpy` of `Curve` if it fits; otherwise
    write a dedicated function, as 2d-1 did for NH₃.
  - **Water:** invert `saturation_pressure` by bisection on
    [273.16 K, 647.096 K]. The step count is a `Definition:` (the
    tolerance it guarantees). This is the same IAPWS code path, with no
    second curve.
  - **Ethane:** Clausius-Clapeyron from the normal boiling point, capped
    at the critical temperature.

- [ ] **Step 4: Run** the same command. Expected: PASS. Then run
  `mise run ci`. Expected: green.

- [ ] **Step 5: Commit**
  `feat(science): add the nitrogen, SO2, ethane and water boiling curves`.

### Task 2: The surface cover in `science::surface::cover` (sonnet)

**Files:**

- Create: `crates/science/src/surface/mod.rs`,
  `crates/science/src/surface/cover.rs`,
  `crates/science/src/surface/cover_tests.rs`.
- Modify: `crates/science/src/lib.rs` (`pub mod surface`),
  `crates/science/src/radiation/test_points.rs` and
  `crates/science/src/radiation/clouds.rs` (`MANABE_HUMIDITY` replaced
  by `surface::RELATIVE_HUMIDITY`). `test_points` goes from
  `pub(super)` to `pub(crate)` (module and the constants the surface
  tests read: `EARTH_AIR`, `EARTH_SURFACE_K`, `TITAN_SURFACE_AIR`,
  `TITAN_SURFACE_K`, `TITAN_PRESSURE_BAR`, `TITAN_METHANE_SHARE`,
  `VENUS_*`), still `#[cfg(test)]`.

**Interfaces:**

- Consumes: `water::ice_ih_density`, `water::vapour_saturation_bar`,
  `habitable::OCEAN_MASS`,
  `radiation::albedo::{shortwave_budget, Underneath}`,
  `radiation::clouds::cloud_cover`, `radiation::veils::sulphuric_deck`.
- Produces:
  - `surface::RELATIVE_HUMIDITY: f64` (0.77, `Source:` Dai 2006 and
    Manabe & Wetherald 1967, per study 24);
  - `pub fn surface_vapour(t: Kelvin, available: Bar) -> Bar`, which is
    `min(RELATIVE_HUMIDITY · vapour saturation(T), available)`;
  - `pub struct Cover { pub ocean, pub land, pub ice, pub frost,
    pub so2_frost: Fraction }` (`so2_frost` is the part of `frost` that
    is SO₂);
  - `pub fn cover(t: Kelvin, water: Kilogram, radius: Metre,
    frost: &PartialPressures) -> Cover` (water is 0 when the planet is
    dry or in runaway: the caller decides);
  - `pub fn ocean_fraction(oceans: f64) -> Fraction`;
  - `pub fn ice_line_share(t: Kelvin) -> Fraction`;
  - `pub fn frost_cover(frost: &PartialPressures) -> Fraction`;
  - `pub fn ground_albedo(cover: &Cover, air: Bar) -> Fraction`;
  - `pub const ICE_COVERED_TEMPERATURE: Kelvin`, which is
    `ICE_LINE_TEMPERATURE − ICE_LINE_CONTRAST / 3` (about 254 K, a
    `Definition:`; legacy `ICE_COVERED_K`), the start of 2d-3's frozen
    branch.
  - Constants, with the values and prefixes of study 24 (legacy name in
    brackets):
    - `OCEAN_ALBEDO` 0.06, `SOIL_ALBEDO` 0.25, `REGOLITH_ALBEDO` 0.11,
      `ICE_ALBEDO` 0.6, `FROST_ALBEDO` 0.75 (Pluto, see the decisions),
      `SO2_FROST_ALBEDO` 0.63 (the same names);
    - `SHIELD_PRESSURE` = `Bar(1e-4)` [`SHIELD_BAR`], `FROST_LAYER` =
      `Bar(0.02)` [`FROST_LAYER_BAR`];
    - `ICE_LINE_TEMPERATURE` = `Kelvin(268.15)` [`ICE_LINE_K`],
      `ICE_LINE_CONTRAST` = `Kelvin(42.0)` [`ICE_LINE_CONTRAST_K`];
    - `ICE_SHEET_THICKNESS` = `Metre(1940.0)` [`ICE_SHEET_KM`]: the
      page's 1.94 km, rounded from Fretwell et al.'s 1937 m;
    - `EARTH_OCEAN_COVER` 0.71 [`EARTH_OCEAN_FRACTION`], renamed so it
      does not clash with the private `habitable::EARTH_OCEAN_FRACTION`.

- [ ] **Step 1: Write the failing tests** (ported from `albedo`, legacy
  line numbers in brackets)
  - `earth_has_seventy_one_percent_of_ocean` [940]: unchanged.
  - `ice_cover_grows_as_the_planet_cools` [793]:
    - Earth's water, 1 `OCEAN_MASS`, at Earth's radius;
    - the ice share is non-increasing over 200 to 300 K;
    - at 288 K it is in 0.06 to 0.14 (an observation: Earth's ice and
      snow, about a tenth);
    - at 200, 230, 240 and 250 K it is above 0.999;
    - at 300 K it is 0.
  - `a_nearly_dry_planet_whitens_only_its_poles` [819]:
    - 0.005 `OCEAN_MASS` at 210 K and Earth's radius, CO₂ 6e-3 bar;
    - the ice share satisfies `0 < ice < 0.02`;
    - the Bond albedo, through `shortwave_budget` with
      `ground_albedo`, no cloud and no deck, a 5772 K star and
      `STANDARD_GRAVITY`, is in 0.2 to 0.3;
    - a full ocean at 210 K gives `ice == 1`.
  - `ice_reach_scales_with_area` [1057]: 0.05 `OCEAN_MASS` at 200 K; the
    ice at 2 R⊕ over the ice at 1 R⊕ is 0.25 within 1e-12, and the ice
    at 1 R⊕ is below 1.
  - `bare_rock_is_all_land` [771]:
    - no water, no frost, at T in {100, 250, 440, 700} K;
    - `land == 1` and the other shares are 0;
    - `ground_albedo(cover, Bar(0))` equals `REGOLITH_ALBEDO` within
      1e-12;
    - at 1 bar, the ground albedo is within 1e-3 of `SOIL_ALBEDO`.
  - `frost_brightens_a_body_in_proportion_to_its_amount` [849]:
    - dry, at 40 K: half of `FROST_LAYER` of N₂ gives `frost == 0.5`
      and `land == 0.5`;
    - the ground albedo grows with the frost;
    - `FROST_LAYER` and 10 bar give the same albedo, equal to
      `FROST_ALBEDO`;
    - 10 bar of SO₂ gives `so2_frost == 1` and the ground albedo
      `SO2_FROST_ALBEDO`;
    - over 1 `OCEAN_MASS` at 40 K, a quarter of the layer of N₂ gives
      `ice == 0.75` and `ocean == 0` (the legacy assertion).
  - `earth_keeps_its_bond_albedo` [1228], an observation:
    - Earth at 288 K: the Earth air of `radiation::test_points` with
      `surface_vapour(288 K, …)`, 1 `OCEAN_MASS` at Earth's mean radius;
    - `Underneath { ground: ground_albedo(…), clouds:
      cloud_cover(288 K, h2o), deck: sulphuric_deck(…) }`;
    - the Bond albedo is in 0.29 to 0.31.
  - `cover_shares_sum_to_one` (Review Focus 3).
  - `no_frost_divides_nothing` (Review Focus 2): an empty frost gives
    `so2_frost == 0` and `frost == 0`, with no NaN.
  - `surface_vapour_is_capped_by_the_water`: at 288 K with plenty of
    water it is 0.77 × the IAPWS saturation; with 1e-6 bar available it
    is 1e-6 bar.

- [ ] **Step 2: Run** `cargo nextest run -p science surface`. Expected:
  FAIL to compile.

- [ ] **Step 3: Implement** the port of `albedo` l.481-563 with the
  decisions above:
  - `reach = min(water / (ρ_ice · 4πR² · ICE_SHEET_THICKNESS), 1)`;
  - `oceans = water / OCEAN_MASS`;
  - the shield is `1 − exp(−air / SHIELD_PRESSURE)`;
  - the land albedo is `REGOLITH + (SOIL − REGOLITH) · shield`;
  - the mix follows `albedo` l.721-756:
    `ocean·OCEAN + land·land_albedo + ice·ICE + (frost − so2)·FROST +
    so2·SO2_FROST`.

  If `earth_keeps_its_bond_albedo` or the 288 K ice bound fails with
  study 24's values, stop and report the numbers. Do not retune.

- [ ] **Step 4: Run** the same command. Expected: PASS. Then run
  `mise run ci`. Expected: green.

- [ ] **Step 5: Commit**
  `feat(science): add the surface cover and the ground albedo`.

### Task 3: Surface liquids and frost in `science::surface::liquids` (opus)

**Files:**

- Create: `crates/science/src/surface/liquids.rs`,
  `crates/science/src/surface/liquids_tests.rs`.
- Modify: `crates/science/src/surface/mod.rs`.

**Interfaces:**

- Consumes:
  - Task 1's curves, including `water::boiling_temperature`, and
    `water::ice_ih_melting_temperature`;
  - `volatiles::methane_saturation_bar`, `co2_frost_pressure`, and the
    methane triple point (`pub(crate)`);
  - `cover::FROST_LAYER`.
- Produces:
  - `pub enum Liquid { Water, Hydrocarbons, Nitrogen, Lava }` (2d-3
    maps it to `timeline::LiquidKind`);
  - `pub fn liquid(t: Kelvin, air: &PartialPressures, water: Kilogram,
    runaway: bool, condensate: &PartialPressures) -> Option<Liquid>`;
  - `pub fn sea_freezes(air_n2: Bar, condensate: &PartialPressures) ->
    Kelvin`;
  - `pub fn methane_freezes(n2: Bar) -> Kelvin`;
  - `pub fn hydrocarbon_freezes(nitrogen: f64, ethane: f64) -> Kelvin`;
  - `pub fn methane_sea_vapour(t: Kelvin) -> Bar`;
  - `pub fn methane_condenses(ch4: Bar, t: Kelvin) -> bool`;
  - `pub fn frozen_share(t: Kelvin, melts: Kelvin) -> Fraction`;
  - `pub fn melting(condensate: Bar, frozen: Fraction) -> Bar`;
  - `pub fn frost(condensate: &PartialPressures, frozen_co2: Bar,
    so2_frost: Bar, t: Kelvin, air: &PartialPressures) ->
    PartialPressures`;
  - the constants `LAVA_TEMPERATURE` 1673 K [`LAVA_K`] and
    `SEA_METHANE_SHARE` 0.5 (study 24 prefixes), plus the constants
    listed in the Global Constraints (`PHASE_BAND` = `Kelvin(1.0)`
    [`PHASE_BAND_K`]).

- [ ] **Step 1: Write the failing tests** (ported from `climate`, with
  the condensate given as input in place of `condense`)
  - `a_trace_of_methane_makes_no_lake` [2868]:
    - at 94 K, N₂ 1.5 bar and CH₄ 0.003 bar give no hydrocarbons;
    - Titan's air (`radiation::test_points`, 93.65 K) with CH₄ at
      `methane_sea_vapour(93.65 K)` gives `Hydrocarbons`;
    - the share `methane_sea_vapour(94 K) / (1.45 bar +
      methane_sea_vapour(94 K))` lies within 0.03 to 0.06, around
      Titan's observed 5.65 % (Niemann et al. 2010).
  - `nitrogen_keeps_methane_liquid_below_its_triple_point` [2902]:
    - the bounds of `methane_freezes`: 75 to 78 K at 0 bar, 67 to 71 K
      at 1.45 bar, the floor at 100 bar in 66 to 69 K;
    - Hydrocarbons at 89 K with seas (condensate CH₄ 1 bar), with N₂ 1
      bar and with N₂ 0;
    - nothing at 75 K with N₂ 0.
  - `a_pure_nitrogen_sea_is_nitrogen` [2955]: condensate N₂ 0.5 bar at
    70 K gives `Nitrogen`; at 60 K it gives no liquid and
    `frost(…).n2 > 0`.
  - `condensed_nitrogen_keeps_the_sea_liquid` [2985]: condensate N₂
    5 bar and CH₄ 8 bar at 70 K give `Hydrocarbons`, and the frost has
    no N₂ and no CH₄.
  - `ethane_lowers_the_liquidus` [3018]:
    - `hydrocarbon_freezes(0.134, 0.256)` is within 2 K of 71.5 K
      (Richardson et al. 2019, an observation);
    - `hydrocarbon_freezes(0.079, 0.453)` is within 2 K of 74.0 K;
    - `hydrocarbon_freezes(0, 0)` equals the rewrite's methane triple
      point within 1e-9;
    - `methane_freezes(1.45 bar)` is below 72 K.
  - `water_on_top_is_never_lava` [3255]:
    - 1 `OCEAN_MASS`, N₂ 10 bar and H₂O 200 bar at 700, 2000 and 5000 K
      give no lava, whatever the runaway flag;
    - a dry body in runaway at 2000 K gives `Lava`.
  - `earth_mars_titan_have_their_liquids` (the liquid half of [2463] at
    observed temperatures):
    - Earth at 288 K (1.013 bar) gives `Water`;
    - Mars at 210 K (6e-3 bar CO₂, 0.005 `OCEAN_MASS`) gives none;
    - Titan at 93.65 K gives `Hydrocarbons`;
    - Venus at 737 K (dry, 92 bar) gives none.
  - `liquid_boundaries_are_inclusive_below` (Review Focus 4):
    - a dry body at exactly `LAVA_TEMPERATURE` gives `Lava`;
    - water under 1 bar of N₂ gives `Water` at exactly
      `ice_ih_melting_temperature(1 bar)`, none 0.01 K below it, and
      none at exactly `boiling_temperature(1 bar)`;
    - water under an air below `TRIPLE_POINT_PRESSURE` gives none at
      any temperature from 200 to 400 K.
  - `frozen_frost_melts_back`: `frozen_share(melts − 1 K, melts)` is 1,
    `frozen_share(melts, melts)` is 0, and
    `melting(5 bar, frozen 0.5)` is `0.5 · FROST_LAYER`.
  - `io_keeps_its_so2_frost`: at 110 K, with Io's air SO₂ at
    `sulphur_dioxide_frost_pressure(110 K)`, a stored SO₂ frost of 1 bar
    keeps 1 bar.

- [ ] **Step 2: Run** `cargo nextest run -p science liquids`. Expected:
  FAIL to compile.

- [ ] **Step 3: Implement** the port of `climate`:
  - l.855-1017 (`hydrocarbon_freezes_k`, `methane_freezes_k`,
    `ethane_boils_k`, `liquid`, `sea_freezes_k`, `methane_condenses`,
    `methane_sea_vapour_bar`);
  - l.1779-1840 (`frozen_share`, `melting`, `frost` without its
    `condense` call).

  The liquidus solve (`ln x + W(1−x)²/T = −L(1/T − 1/Tm)`) uses
  bisection with a step count that is a `Definition:`. Water is liquid
  on `[ice_ih_melting_temperature(P), boiling_temperature(P))`, where
  `P` is the total pressure (see the decisions). In `frost`, a CO₂ or
  SO₂ frost curve that returns `None` (at or above its triple point)
  counts as an infinite sublimation pressure, so the frost all goes
  back to the air, as legacy's `cap(INFINITY, T)` does.

- [ ] **Step 4: Run** the same command. Expected: PASS. Then run
  `mise run ci`. Expected: green.

- [ ] **Step 5: Commit**
  `feat(science): add the surface liquids and their frost`.

### Task 4: The volcanic intensity in the bare chain (sonnet)

**Files:**

- Create: `crates/system/src/planet/volcanism.rs` and
  `crates/system/src/planet/volcanism_tests.rs`.
- Modify:
  - `crates/system/src/planet/mod.rs` (module with the `expect(dead_code)`
    block);
  - `codes.rs` (`SURFACE_VOLCANISM = "planet.surface.volcanism"`);
  - `bare.rs` (`BareSurface.volcanism: Traced<Fraction>`, recorded
    after `regime`);
  - `regime.rs:42` (the flux through `science::relief::volcanoes::heat_flux`,
    the same value, one code path).

**Interfaces:**

- Produces `pub(crate) fn volcanism(body: &Body, heat: &Heat, regime:
  &Traced<TectonicRegime>, trace: &mut Trace) -> Result<Traced<Fraction>,
  InteriorError>`:
  - the value is `volcanic_intensity(heat_flux(radiogenic + tidal +
    secular, R))`, times `LID_SHARE / MOBILE_SHARE` under
    `TectonicRegime::Stagnant`, and times 1 otherwise (heat pipes
    included, see the decisions and #670);
  - the node derives from the three heat nodes, the radius node and the
    regime node.

- [ ] **Step 1: Write the failing tests** (in `volcanism_tests.rs`, with
  the `trial(trace, seed, mass, au, ecc)` fixture of `planet/tests.rs`)
  - `volcanism_reads_the_regime` (Review Focus 5):
    - an Earth (1 M⊕, 1 AU) and a Mars (0.107 M⊕, below
      `PLATES_MIN_MASS`, 1.52 AU), through `bare`. Assert the
      precondition first: their regimes are `Plates` and `Stagnant`;
    - each recorded value equals the formula above, computed from the
      `Bare` heat and radius, within 1e-12;
    - the node's causes are exactly the five nodes;
    - the Mars has a lower intensity than the Earth.
  - `a_heat_pipe_reads_the_law`: on a constructed `Heat` whose flux is
    3 W/m² with the regime `HeatPipe`, the intensity is 1. At 1.5 W/m²
    it is `volcanic_intensity(1.5 W/m²)`, below 1.
  - The existing `regime` tests stay green: the shared `heat_flux` gives
    the same flux.

- [ ] **Step 2: Run** `cargo nextest run -p system planet`. Expected:
  FAIL.

- [ ] **Step 3: Implement** as specified. The doc comment cites
  decision B and says 2d-3 divides by `EARTH_VOLCANIC_INTENSITY`.

- [ ] **Step 4: Run** the same command. Expected: PASS. Then run
  `mise run ci`. Expected: green.

- [ ] **Step 5: Commit**
  `feat(planet): record the volcanic intensity the snowball reads`.

### Task 5: Documentation and CI (sonnet)

**Files:**

- Modify: `docs/science/planet-atmospheres.md` ("Climate and surface
  liquid"), `docs/science/radiative-transfer.md` (l.~710-716 and
  l.~1017-1021), `crates/science/README.md`.

- [ ] **Step 1:** Write the decisions of 2026-11-24 into
  `planet-atmospheres.md`:
  - `FROST_ALBEDO` on Pluto, with Triton's gap and the 2d-3 refit;
  - the ice-density reach;
  - the IAPWS melting and boiling window, in place of 273.15 K and
    Clausius-Clapeyron;
  - the accuracy of the N₂ Antoine fit (Task 1);
  - volcanism B, with the heat-pipe gap of #670;
  - `RELATIVE_HUMIDITY` public;
  - a "Where the code lives" line for `science::surface`.

  In `radiative-transfer.md`, the cover now exists (`ground_albedo`),
  and #656 stays open, with the reason from study 24.

- [ ] **Step 2:** Add a `surface` paragraph to the README, keeping it at
  150 lines or fewer.

- [ ] **Step 3: Run** `mise run docs:check` and `mise run ci`. Expected:
  green. Regenerate figures only if `docs:figures` reports a stale
  one.

- [ ] **Step 4: Commit** `docs(science): describe the surface module`.

### Before the PR (main session)

- [ ] Push Wiki study 24 (clone commit 7a9348d). The hook blocks the
  push, so give the user the command: `git -C
  /private/tmp/claude-501/-Users-cyborg-Developer-map-generator/c5de5b2a-0805-47b0-b53d-47ef8dc824df/scratchpad/wiki
  push`.
- [ ] Run CLAUDE.md's "Before opening a pull request" procedure:
  `branch-reviewer` and `test-reviewer` on `45cbf8a8..HEAD`, then
  `science-coherence-checker` and `issue-auditor`.
- [ ] Write the PR description:
  - `Refs #602` and `Refs #656`;
  - a "Calibrations" heading listing the kept calibrations (none new);
  - the values that differ from legacy and will reach the generated
    systems in 2d-4: `LAVA_TEMPERATURE`, `ICE_SHEET_THICKNESS`, the ice
    density of the reach, the IAPWS melting and boiling, and volcanism
    B.

  Generated systems are unchanged, so there is no oracle run.
