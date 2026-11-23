# Planet model PR 2c: escape and outgassing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port from `legacy-final` the part of the planet chain between
the interior and the climate: energy-limited envelope escape, the water
kept (runaway threshold, steam loss, the body derived again), the
minimal tectonic regime, the core and its dynamo, the volatile inventory
draw, outgassing, the cosmic shoreline and the SO2 wash-out, each gas
with its cause tree.

**Architecture:** Laws and their numbers go to new `science` modules
(`escape`, `habitable`, `tectonics`, `dynamo`, `outgassing`) and to
`science::volatiles`; `science` knows no `timeline` type, so the
regime's share and Nusselt mappings live in `system`. The steps go to
new `system::planet` modules (`escape`, `water`, `regime`, `core`,
`air`) and one crate-private chain `bare` that runs them in the legacy
order and returns the timeline contract values (`Water`, `Core`,
`Traced<TectonicRegime>`, `Atmosphere`) for 2d to put into `Surface`.
Nothing is public and nothing generated changes: no engine version
bump.

**Tech Stack:** Rust, `cargo nextest` tiers, the `Trace`/`Traced`
cause API of `timeline`, the seed hierarchy (`Seed::derive`,
`system::random::normal`).

**Spec:**
`docs/superpowers/specs/2026-11-21-planet-model-pr2-split-design.md`
(row 2c), with the decisions of 2026-11-23 below. Reference page:
`docs/science/planet-atmospheres.md` ("Volatile inventory at
formation", "Envelope escape", "Runaway greenhouse, water kept and
tectonic regime", "Core and dynamo", "Atmosphere" down to "Non-thermal
loss"; grep the headings, the page is 1 900 lines). Audit: Wiki study
21,
<https://github.com/cyboooooorg/map-generator/wiki/21-escape-outgassing-audit>
(one row per value: final value, prefix, source read).

Legacy paths below are under `legacy-final:crates/mapgen/src/planet/`:
`esc` is `escape.rs`, `reg` is `regime.rs`, `cor` is `core.rs`, `atm`
is `atmosphere.rs`, `inp` is `inputs.rs`, `mod` is `mod.rs`.

## Decisions of 2026-11-23 (with the user)

- **Out of 2c:** the non-thermal loss and the moons' magnetospheric
  shield (`atm` l.322-548, `mod` l.471-489) wait for the moons, #658
  under T2. `methane_photolysis`, `cold_trap`, `photochemistry`, the
  outgassing redone for a reducing air (`atm` l.1115-1300) and the
  `boils` loop (`mod` l.524-575) read the climate or sit after it: 2d.
  The legacy Mars CO2 factor (`reference.rs:216`, a test fudge on an
  unchecked 6.1 mbar) is not ported: it served the climate test.
- **In 2c:** `water()`, `steam()`, the body derived again after a
  runaway (`ran_after_steam`, `after_steam`, `mod` l.664-710) and
  `Water::keeping` after the shoreline (`mod` l.~930, l.577-590).
- **Values changed from legacy by study 21:** `WET_SO2_SHARE` 2.2e-9
  (1.5 d over 1.9 Myr); `CORE_HEAT_CAPACITY` 840 J/(kg K), `Source:`
  Christensen et al. 2009; the dipole share split into `F^(1/3)` and
  `1/7` (both `Source:`), `1/√2` (`Definition:`) and a calibrated
  remainder 0.82; the ice volatiles read in Biver et al. 2022 (Comets
  III, arXiv:2207.04800) instead of the unreachable Mumma & Charnley
  2011 (Task 4 table).
- **The runaway threshold under several stars** compares the total
  flux with `runaway_flux` at the flux-weighted effective temperature
  (`StellarSources::weighted`). Legacy had one star; T7 refines it.
- **Cause trees, not one `Escape` quantity:** each gas's partial
  pressure derives from its outgassed node, the shoreline's kept share,
  and for SO2 the wash-out share; the surface pressure derives from the
  partials (legacy `explain::pressure_tree` folded the shoreline and
  the non-thermal loss into one difference).

## Global Constraints

- Legacy is read only at tag `legacy-final`; every law is ported with
  its tests, then changed only where the decisions above say so.
- Every numeric `const`/`static` and every `let` bound to a bare number
  other than 0, 0.5, 1, 2 has a doc comment starting `Source:`,
  `Calibration:` or `Definition:`, in `science` (guardrail
  `science_constants_are_sourced`) and in `system` by the same rule
  (`branch-reviewer` checks it). Prefixes, values and citations are the
  ones of study 21's table, never the legacy comment: legacy "from
  memory, not checked" notes are not carried over.
- Missing science stops the work: a stop goes to the main session,
  which brings it to the user. No test is `#[ignore]`d and no tolerance
  widened to get past a stop. A calibration target that breaks after a
  sourced value replaced a legacy one (`CORE_HEAT_CAPACITY`) is retuned
  in its `Calibration:` constant, and the retune is written in the PR
  description under its own heading.
- `science` takes and returns the typed units of `units.rs` at its
  public boundary, never `timeline` types. No `HashMap`, `thread_local!`,
  file, clock or process in `science` or `system`.
- Every step value is recorded in the `Trace` with a code of
  `planet/codes.rs` (`planet.<dotted-lowercase>`), derived from the
  nodes it was computed from: one code path per value.
- New draws use a new sub-seed label; existing labels and the order of
  their draws do not change (the 2a draws stay identical, a test says
  so).
- The new modules are crate-private and carry the same
  `#[cfg_attr(not(test), expect(dead_code, …))]` as the existing slice
  until 2d wires the chain.
- `mise run ci` passes at the end of every task; tier-1 budget 2 min.

## Review Focus

1. **A mini-Neptune stripped below `MIN_ENVELOPE`** is rebuilt as a
   rocky body of `M - f M`, with its core's water share; its envelope
   fraction node is 0 and derives from the loss. (Task 6 test
   `a_stripped_mini_neptune_is_rebuilt_rocky`.)
2. **A star without corona or a zero XUV fluence** loses nothing and
   keeps all its air: no NaN, no division by a zero line. (Task 1 test
   `no_fluence_no_loss`, Task 9 test `no_xuv_keeps_the_whole_air`.)
3. **A hot water world whose water is beyond the law** (355 K or more,
   `Water::BeyondLaw`) runs away, loses steam and is rebuilt without
   error, whichever side of 355 K the kept water lands on. (Task 7 test
   `a_hot_water_world_runs_away_and_rebuilds`.)
4. **A giant at 0.05 AU** loses at most 1 % of its envelope and is
   rebuilt in the Fortney tables with its drawn heavy mass, its
   fraction `1 - Mz / M'` derived from the kept mass. (Task 6 test
   `a_close_giant_keeps_its_envelope`.)
5. **Two stars:** the runaway threshold reads the weighted effective
   temperature and the total flux; one source gives the legacy result.
   (Task 7 test `the_runaway_threshold_reads_every_source`.)

---

### Task 1: `science::escape`, energy limit and shoreline (opus)

**Files:**

- Create: `crates/science/src/escape.rs`, `crates/science/src/escape_tests.rs`
- Modify: `crates/science/src/lib.rs` (module),
  `docs/science/planet-atmospheres.md` ("How it computes them" lines naming
  `science::escape`)

**Interfaces:**

- Consumes: `science::xuv::EARTH_XUV_FLUENCE` (`xuv.rs:128`),
  `science::constants` (G).
- Produces:
  - `pub const EFFICIENCY: f64` (0.1, `Source:` Owen & Wu 2017 §2.2 eq. 19)
  - `pub const MIN_ENVELOPE: f64` (1e-3, `Calibration:`)
  - `pub const WATER_PER_HYDROGEN: f64` (9, `Definition:`)
  - `pub fn energy_limited_loss(radius: Metre, mass: Kilogram, fluence:
    JoulePerSquareMetre) -> Kilogram` — `η π R³ F / (G M)`, uncapped (`esc`
    l.28-32)
  - `pub fn escape_velocity(mass: Kilogram, radius: Metre) -> MetrePerSecond`
  - `pub const SHORELINE_VELOCITY: MetrePerSecond` (6.5 km/s),
    `SHORELINE_STEEPNESS: f64` (6), `UNMAGNETISED_ESCAPE: f64` (2), all
    `Calibration:`
  - `pub fn shoreline_kept(escape_velocity: MetrePerSecond, fluence:
    JoulePerSquareMetre, magnetised: bool) -> Fraction` — `1 / (1 + (φ s / (v /
    v₀)⁴)^k)` with `φ` the fluence over `EARTH_XUV_FLUENCE`, `s` 1 or
    `UNMAGNETISED_ESCAPE` (`atm` l.288-320)

- [ ] **Step 1: Write the failing tests** in `escape_tests.rs`:
  `the_loss_is_owen_and_wus_energy_limit` (Earth radius and mass,
  `EARTH_XUV_FLUENCE`: equals `0.1 π R³ F / (G M)` evaluated by hand in
  the test, relative 1e-12); `the_loss_scales_as_r_cubed_over_m`;
  `no_fluence_no_loss` (0 J/m² gives 0 kg; `shoreline_kept` gives 1);
  `earths_escape_velocity_is_11_2_km_s` (11.186 km/s within 0.5 %,
  `Source:` the NASA Earth fact sheet value the page cites);
  `the_shoreline_passes_half_at_its_line` (v = 6.5 km/s at Earth's
  fluence, magnetised: 0.5); `a_missing_dynamo_doubles_the_fluence`
  (unmagnetised at φ equals magnetised at 2φ).
- [ ] **Step 2: Run** `cargo nextest run -p science escape` — FAIL (module
      missing).
- [ ] **Step 3: Implement** the items above; port the legacy docs, with study
      21's citations.
- [ ] **Step 4: Run** the same command — PASS; `cargo nextest run -p guardrails`
      — PASS.
- [ ] **Step 5: Commit** `feat(science): port the energy-limited escape and the
      cosmic shoreline`.

### Task 2: `science::habitable` and `science::tectonics` (sonnet)

**Files:**

- Create: `crates/science/src/habitable.rs`, `crates/science/src/tectonics.rs`
  (tests inline in a `#[cfg(test)] mod tests` or `*_tests.rs` as the crate does
  for small modules)
- Modify: `crates/science/src/lib.rs`

**Interfaces:**

- Produces:
  - `pub fn runaway_flux(effective_temperature: Kelvin) -> EarthFlux` —
    Kopparapu et al. 2014 Table 1, runaway greenhouse, 1 M⊕: `S = 1.107 + a T +
    b T² + c T³ + d T⁴`, `T = Teff - 5780 K`, `a..d` = 1.332e-4, 1.58e-8,
    -8.308e-12, -1.931e-15 (`docs/science/habitable-zone.md:161-167`); `Teff`
    clamped to [2600 K, 7200 K], the fit's range
    (`legacy-final:crates/mapgen/src/star.rs:293`)
  - `pub const HEAT_PIPE_FLUX: WattPerSquareMetre` (1.0, `Calibration:`), `pub
    const PLATES_MIN_MASS: EarthMass` (0.3, `Calibration:`, geometric mean of
    Mars and Venus), `pub const OCEAN_MASS: Kilogram` (1.4e21, `Definition:`,
    1.332e9 km³ per study 21)

- [ ] **Step 1: Write the failing tests:** `the_sun_runs_away_at_1_107` (5780 K
      gives 1.107 exactly, Kopparapu 2014 Table 1);
      `the_fit_is_clamped_to_its_range` (2000 K equals 2600 K, 9000 K equals
      7200 K, by the paper's stated range);
      `cooler_stars_run_away_at_lower_flux` (3000 K gives less than 1.107: the
      inner edge moves in toward cool stars, Kopparapu et al. 2014 Fig. 1).
- [ ] **Step 2: Run** `cargo nextest run -p science habitable` — FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** — PASS; guardrails PASS.
- [ ] **Step 5: Commit** `feat(science): add the runaway threshold and the
      regime constants`.

### Task 3: `science::dynamo`, core cooling and field (opus)

**Files:**

- Create: `crates/science/src/dynamo.rs`, `crates/science/src/dynamo_tests.rs`
- Modify: `crates/science/src/lib.rs`

**Interfaces:**

- Produces (port of `cor` l.21-205, numbers from study 21):
  - `pub const CORE_HEAT_CAPACITY: f64` 840 J/(kg K) (`Source:` Christensen et
    al. 2009), `MANTLE_CONDUCTIVITY` 4 W/(m K), `INITIAL_EXCESS` 2800 K,
    `THERMAL_DYNAMO_END` 0.075, `INNER_CORE_START` 1.0, `CORE_FROZEN` 2.25 (in
    cooling times), `NUSSELT_PLATES` 50, `NUSSELT_EPISODIC` 10, `NUSSELT_LID` 5
    (stagnant lid and heat pipe), all `Calibration:` with their targets
  - `pub const FLUX_SCALING_C: f64` 0.63, `pub const DYNAMO_EFFICIENCY: f64` (F
    = 0.52, entering as `F^(1/3)` = 0.804: Christensen et al. 2009 eq. 2,
    `B²/2µ₀ = c f_ohm ρ^(1/3) (F q)^(2/3)`), `pub const DIPOLE_OVER_FIELD: f64`
    (1/7, "typical ratio B/B_dip of around seven", pp. 168-169), each `Source:`;
    `RMS_TO_EQUATORIAL` (1/√2, `Definition:`, the rms of a dipole over a sphere
    is √2 its equatorial value); `pub const DIPOLE_REMAINDER: f64` (0.82,
    `Calibration:` on Earth's 30 µT; the product of the four is legacy's 0.067);
    `DEEP_DIPOLE_SHARE` 3e-4 stays one `Calibration:` (Mercury: the paper says
    the scaling is not applicable there)
  - `pub fn cooling_time(core_density: KilogramPerCubicMetre, core_radius:
    Metre, mantle_thickness: Metre, nusselt: f64) -> Gyr` — `τ = ρ c r_c d / (3
    k Nu)`
  - `pub enum CorePhase { Liquid { dynamo: bool }, Frozen }` and `pub fn
    core_phase(age: Gyr, tau: Gyr, plates: bool) -> CorePhase` — frozen at `t/τ
    ≥ CORE_FROZEN`; thermal dynamo if `plates` or `t/τ < THERMAL_DYNAMO_END`;
    compositional dynamo from `INNER_CORE_START`
  - `pub fn surface_field(core_density: KilogramPerCubicMetre, core_radius:
    Metre, planet_radius: Metre, age: Gyr, tau: Gyr, deep: bool) -> Tesla` — `q
    = ρ c r_c ΔT₀ e^(-t/τ) / (3τ)`, `B = sqrt(2 µ₀ C ρ^(1/3) q^(2/3))` × share ×
    `(r_c / R)³`; `deep` picks the deep-dipole share (legacy's choice, `cor`
    l.180-200)

- [ ] **Step 1: Write the failing tests:** `the_flux_scaling_is_christensens` (a
      test of `B` against the core field Christensen et al. 2009 print for Earth
      if study 21 records one, else the formula evaluated by hand,
      `Definition:`); `the_phases_follow_their_thresholds` (each threshold, both
      sides); `a_core_cools_monotonically` (`surface_field` falls with age,
      `core_phase` never returns from `Frozen`: legacy `cores_freeze_with_age`,
      `cor` l.258); `no_core_no_field` (zero core radius gives 0 T).
- [ ] **Step 2: Run** `cargo nextest run -p science dynamo` — FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** — PASS; guardrails PASS.
- [ ] **Step 5: Commit** `feat(science): port the core's cooling and dynamo
      field`.

### Task 4: `science::volatiles` inventory and `science::outgassing` (opus)

**Files:**

- Modify: `crates/science/src/volatiles.rs`,
  `crates/science/src/volatiles_tests.rs`
- Create: `crates/science/src/outgassing.rs`,
  `crates/science/src/outgassing_tests.rs`
- Modify: `crates/science/src/lib.rs`

**Interfaces:**

- Produces in `volatiles` (port of `inp` l.215-248, 476-505):
  - `pub struct Inventory { pub co2: Kilogram, pub n2: Kilogram, pub s:
    Kilogram, pub ch4: Kilogram, pub nh3: Kilogram }`
  - `pub fn inventory(refractory: Kilogram, silicate: Kilogram, ice: Kilogram)
    -> Inventory` — the rule without scatter: rock CO2 and N2 on `refractory`, S
    on `silicate`, the four ice volatiles on `ice`
  - `pub const VOLATILE_SPREAD: Dex` (0.3, `Calibration:`) and the rule
    constants:

    | Constant | Value | Prefix |
    | --- | --- | --- |
    | `ROCK_CO2` | 1e-4 | `Calibration:` (Venus) |
    | `ROCK_N2` | 2e-6 | `Calibration:` |
    | `ROCK_S` | 2.5e-4 | `Source:` Wang et al. 2018 |
    | `ICE_CO2` | 0.12 (5 mole %; legacy 0.05 was 2 mole %) | `Calibration:` (Biver et al. 2022 range, 67P) |
    | `ICE_N2` | 1.4e-3 | `Calibration:` (67P, Rubin et al. 2019) |
    | `ICE_CH4` | 7e-3 | `Source:` Biver et al. 2022, Table 3 |
    | `ICE_NH3` | 7e-3 | `Source:` Biver et al. 2022, Table 3 |

- Produces in `outgassing` (port of `atm` l.24-124, 182-280):
  - `pub const MOBILE_SHARE: f64` 0.95 (plates, episodic, heat pipe),
    `LID_SHARE` 0.2 (stagnant lid), `SEAFLOOR_HALF_PRESSURE: Bar` 3e4,
    `SEAFLOOR_STEEPNESS` 4.1, `RECYCLED_UNDER_PLATES` 0.63,
    `NITROGEN_RECYCLED_UNDER_PLATES` 0.8, `SULPHUR_IN_AIR` 5e-5, `OXIDISING_O2:
    Bar` 0.01, `WET_SO2_SHARE` 2.2e-9, `DRY_COLUMN: Bar` 10, all `Calibration:`;
    `SO2_PER_S` 2 (`Definition:`, 64.07 / 32.06 rounded as legacy; the doc says
    so)
  - `pub fn column_pressure(mass: Kilogram, gravity: MetrePerSecondSquared,
    radius: Metre) -> Bar` — `m g / (4π R²)` (`Definition:`)
  - `pub fn released_under_ocean(seafloor: Bar) -> Fraction` — `1 / (1 + (P /
    P½)^k)`
  - `pub fn so2_kept(wet: bool, o2: Bar, total: Bar) -> Fraction` —
    `WET_SO2_SHARE` when wet or `o2 ≥ OXIDISING_O2`, else `min(total /
    DRY_COLUMN, 1)` (`atm` l.251-274)

- [ ] **Step 1: Write the failing tests:** `earths_silicate_sulphur_is_wangs`
      (`inventory` of Earth's silicate mass gives `ROCK_S` × silicate, 235 ± 38
      ppm, Wang et al. 2018); `ice_carries_only_ice_volatiles` (no rock, no S);
      `a_column_of_earths_air_is_a_bar` (the mass of Earth's atmosphere as
      Trenberth & Smith 2005 print it, read in the paper, on Earth's gravity and
      radius: the mean surface pressure they give, within 1 %);
      `half_is_released_under_the_half_pressure`; `wash_out_shares` (legacy
      `atm` l.1479 exact values: dry 1 bar gives 0.1, dry 20 bar gives 1, wet or
      0.01 bar O2 gives `WET_SO2_SHARE`).
- [ ] **Step 2: Run** `cargo nextest run -p science volatiles outgassing` —
      FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** — PASS; guardrails PASS.
- [ ] **Step 5: Commit** `feat(science): port the volatile inventory and the
      outgassing laws`.

### Task 5: The volatile scatter draw (sonnet)

**Files:**

- Modify: `crates/system/src/planet/draws.rs`,
  `crates/system/src/planet/codes.rs`, `crates/system/src/planet/tests.rs`

**Interfaces:**

- Consumes: `science::volatiles::VOLATILE_SPREAD`, `crate::random::normal`.
- Produces:
  - `Labels.volatiles: &'static str`, `LABELS.volatiles = "planet/volatiles"`
  - `pub(crate) struct VolatileScatter { pub co2: Traced<Dex>, pub n2:
    Traced<Dex>, pub s: Traced<Dex>, pub ch4: Traced<Dex>, pub nh3: Traced<Dex>
    }` — `N(0, VOLATILE_SPREAD)` drawn in the order CO2, N2, S, CH4, NH3 from
    one sub-seed (`inp` l.445-460), each a `Provenance::Drawn` leaf with code
    `planet.volatiles.scatter.<gas>`
  - `Drawn.volatiles: VolatileScatter`; `Drawn::fixed` gains the scatter as an
    argument (`None` gives 0 dex leaves, `Provenance::Imposed`, for the
    reference bodies)

- [ ] **Step 1: Write the failing tests:**
      `the_volatile_draw_leaves_the_other_draws_unchanged` (same seed, two label
      sets differing only in `volatiles`: the core, water, radiogenic and
      envelope values are equal, the scatter differs);
      `the_volatile_scatter_has_its_spread` (2 000 seeds: mean 0 ± 0.02 dex,
      standard deviation 0.3 ± 0.02, `Calibration:` the constant itself);
      `volatile_draws_are_independent` (different labels, as
      `different_labels_draw_independently`).
- [ ] **Step 2: Run** `cargo nextest run -p system planet::tests` — FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** — PASS.
- [ ] **Step 5: Commit** `feat(system): draw the volatile scatter of a planet`.

### Task 6: The envelope escape step (opus)

**Files:**

- Create: `crates/system/src/planet/escape.rs`,
  `crates/system/src/planet/escape_tests.rs`
- Modify: `crates/system/src/planet/mod.rs`, `codes.rs`

**Interfaces:**

- Consumes: `science::escape::{energy_limited_loss, MIN_ENVELOPE}`,
  `interior::interior`, `Trial.sources.band_total(|s| &s.xuv)` (fluence),
  `Body::bulk`.
- Produces:
  - `pub(crate) struct Escaped { pub trial: Trial, pub drawn: Drawn, pub body:
    Body, pub lost: Traced<Kilogram> }`
  - `pub(crate) fn escaped(trial: &Trial, drawn: &Drawn, body: Body, trace: &mut
    Trace) -> Result<Escaped, InteriorError>` — port of `envelope_after_escape`
    (`esc` l.40-64): the loss on the formed body's radius and mass, clamped to
    [0, f M]; the kept mass node `planet.mass.kept` (from `trial.mass` and the
    loss node `planet.envelope.lost`); the kept fraction node
    `planet.envelope.kept-fraction`, 0 below `MIN_ENVELOPE` (mass then `M - f
    M`); for a giant `1 - Mz / M'` from the drawn heavy mass. When nothing is
    lost (no envelope, or zero fluence), `body` is returned as given and the
    trial and drawn are clones; otherwise `interior::interior` runs again on the
    trial with the kept mass and the drawn with the kept fraction.

- [ ] **Step 1: Write the failing tests** (ported from `esc` l.66-131 with the
      fixtures of `planet/tests.rs`):
      `a_close_mini_neptune_is_stripped_to_its_core` (5 M⊕, 1 % envelope, 0.05
      AU: fraction 0, radius below 1.8 R⊕);
      `a_stripped_mini_neptune_is_rebuilt_rocky` (Review Focus 1: `Body::Rocky`,
      mass `M - f M`, the fraction node derives from the loss node);
      `a_close_giant_keeps_its_envelope` (Review Focus 4: Jupiter at 0.05 AU
      keeps ≥ 0.99, the body is a `Giant`);
      `a_temperate_mini_neptune_keeps_half` (0.5 AU: ≥ 0.5);
      `a_rocky_planet_is_untouched` (Earth at 1 and 400 times the fluence: the
      same body, no new node); `the_loss_never_exceeds_the_envelope` (fraction
      in [0, f], mass in [core, M]).
- [ ] **Step 2: Run** `cargo nextest run -p system planet::escape` — FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** — PASS.
- [ ] **Step 5: Commit** `feat(system): port the energy-limited escape of an
      envelope`.

### Task 7: Water kept, steam and the regime (opus)

**Files:**

- Create: `crates/system/src/planet/water.rs`,
  `crates/system/src/planet/regime.rs`,
  `crates/system/src/planet/water_tests.rs`
- Modify: `mod.rs`, `codes.rs`

**Interfaces:**

- Consumes: `science::habitable::runaway_flux`,
  `science::escape::{energy_limited_loss, WATER_PER_HYDROGEN}`,
  `science::tectonics::*`, `heat::heat`, `Escaped`.
- Produces:
  - `pub(crate) fn water(trial: &Trial, drawn: &Drawn, body: &Body, trace: &mut
    Trace) -> Result<timeline::Water, InteriorError>` — port of `water` and
    `steam` (`reg` l.86-121): `runaway` when the total flux exceeds
    `runaway_flux` at the weighted effective temperature; then `inventory =
    max(0, w M - 9 · loss)`, else `w M`; `boiled` false (the `boils` loop is
    2d). Codes `planet.water.runaway`, `planet.water.inventory`,
    `planet.water.steam-lost`.
  - `pub(crate) fn after_steam(trial: &Trial, drawn: &Drawn, water:
    &timeline::Water, trace: &mut Trace) -> Result<Option<(Trial, Drawn)>,
    InteriorError>` — port of `after_steam` (`mod` l.684-710): `None` when
    nothing was lost; else the mass less the lost water and the water fraction
    of what was kept, as derived nodes (`planet.mass.after-steam`,
    `planet.water.kept-fraction`); the envelope fraction and the volatile
    scatter keep their mass (the inventory rule reads masses, so nothing to
    rescale).
  - `pub(crate) fn regime(trial: &Trial, body: &Body, heat: &timeline::Heat,
    water: &timeline::Water, trace: &mut Trace) ->
    Result<Traced<TectonicRegime>, InteriorError>` — port of `regime` (`reg`
    l.135-150): heat pipe above `HEAT_PIPE_FLUX` (total heat over `4π R²`),
    stagnant lid below `PLATES_MIN_MASS`, plates with water kept and no runaway,
    episodic otherwise; code `planet.regime`, derived from the heat, mass and
    water nodes it read.

- [ ] **Step 1: Write the failing tests** (ported from `reg` l.152-233, with
      reference trials built like
      `an_earth_trial_has_earths_bulk_and_no_water_layer`; Venus, Mars and
      Mercury from `reference.rs:74-77` and their eccentricities
      `reference.rs:100-103`, each a `Source:` comment as legacy's):
      `venus_ran_away_and_dried` (runaway, inventory 0 with 2.3e-4 water);
      `earth_keeps_its_ocean` (0.9 to 1.1 `OCEAN_MASS`);
      `the_solar_system_has_its_regimes` (Earth plates, Venus episodic, Mars and
      Mercury stagnant lid; Io's case: an Io-mass rocky body with a
      `timeline::Heat` built by hand, tidal 1e14 W and 7.5e13 W as legacy `reg`
      l.212, gives a heat pipe); `a_dry_super_earth_is_not_plated`;
      `a_hot_water_world_runs_away_and_rebuilds` (Review Focus 3);
      `the_runaway_threshold_reads_every_source` (Review Focus 5: two sources at
      0.5 the flux each equal one source).
- [ ] **Step 2: Run** `cargo nextest run -p system planet::water` — FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** — PASS.
- [ ] **Step 5: Commit** `feat(system): port the water kept, the steam loss and
      the regime`.

### Task 8: The core step (sonnet)

**Files:**

- Create: `crates/system/src/planet/core.rs`,
  `crates/system/src/planet/core_tests.rs`
- Modify: `mod.rs`, `codes.rs`

**Interfaces:**

- Consumes: `science::dynamo::*`, the iron layer of `Body::layers()` (material
  `Material::Iron`, its `outer_radius` and `mass`), `Traced<TectonicRegime>`,
  `trial.age`.
- Produces: `pub(crate) struct CoreRecord { pub core: timeline::Core, pub
  dynamo: Traced<bool> }` and `pub(crate) fn core(trial: &Trial, body: &Body,
  regime: &Traced<TectonicRegime>, trace: &mut Trace) -> Result<CoreRecord,
  InteriorError>` — port of `core` (`cor` l.150-205): Nusselt by regime
  (`NUSSELT_LID` for stagnant lid and heat pipe), `CoreState::None` and 0 T
  without an iron layer; codes `planet.core.cooling-time`, `planet.core.state`,
  `planet.core.dynamo`, `planet.core.field`. A giant or a mini-Neptune's core
  gets `CoreState::None` here: its core state is `enveloped::GIANT_CORE_STATE`
  and a surface only exists below `MIN_ENVELOPE`.

- [ ] **Step 1: Write the failing tests** (from `cor` l.207-286, the targets of
      study 21): `the_solar_system_has_its_dynamos` (Earth field in [20, 70] µT;
      Mercury above 0 and below 2 µT; Mars none at 4.567 Gyr, one at 0.3 Gyr;
      Venus none); `no_iron_no_core`.
- [ ] **Step 2: Run** `cargo nextest run -p system planet::core` — FAIL.
- [ ] **Step 3: Implement.** If a target fails with 840 J/(kg K), retune the
      threshold the target calibrates (Global Constraints) and write it down for
      the PR.
- [ ] **Step 4: Run** — PASS.
- [ ] **Step 5: Commit** `feat(system): port the core and its dynamo`.

### Task 9: The air: outgassing, shoreline, wash-out (opus)

**Files:**

- Create: `crates/system/src/planet/air.rs`,
  `crates/system/src/planet/air_tests.rs`
- Modify: `mod.rs`, `codes.rs`

**Interfaces:**

- Consumes: `science::outgassing::*`, `science::volatiles::inventory`,
  `science::escape::{escape_velocity, shoreline_kept}`, `Drawn.volatiles`,
  `timeline::Water`, `Traced<TectonicRegime>`, `CoreRecord.dynamo`, the XUV
  fluence.
- Produces: `pub(crate) fn air(trial: &Trial, drawn: &Drawn, body: &Body, water:
  &timeline::Water, regime: &Traced<TectonicRegime>, core: &CoreRecord, trace:
  &mut Trace) -> Result<(Atmosphere, timeline::Water), InteriorError>` — in
  order:
  1. outgassed partials (`atm` l.211-247), one node per gas
     `planet.air.outgassed.<gas>`, from the inventory (rule × `10^scatter`), the
     regime's share (`MOBILE_SHARE` or `LID_SHARE`), the ocean's release and the
     plates' recycling; H2O only in a runaway (its kept water as vapour), O2 0;
  2. the shoreline's kept share `planet.air.shoreline-kept`, one node, and each
     partial `planet.air.kept.<gas>` from its outgassed node and that share;
  3. in a runaway, the water inventory is the kept vapour back in kilograms
     (`Water::keeping`, `mod` l.~930), a new `planet.water.inventory` node; the
     caller derives the body again from it (Task 10);
  4. SO2 washed: `planet.air.so2-kept` from `so2_kept(wet, o2, total)`, `wet`
     being `!runaway && inventory > 0` (`climate::washed`, legacy
     `climate.rs:2130`);
  5. `Atmosphere { surface_pressure: planet.air.pressure from every partial,
     partial }`.

- [ ] **Step 1: Write the failing tests** (from `atm` l.1358-1810, the planets
      only; targets as study 21 states them): `venus_has_ninety_bars_of_co2`
      (total in [60, 120] bar, CO2 above 90 %, N2 in [1, 6]);
      `earth_has_a_bar_of_nitrogen` (N2 in [0.5, 1.2]);
      `earths_air_keeps_no_sulphur_dioxide` (below 1e-6 bar);
      `mars_keeps_a_trace` (total in [0.002, 0.05] bar, without the legacy Mars
      factor); `mercury_is_airless` (below 1e-4 bar, the 2d class threshold
      `AIRLESS_BAR`, `Definition:` in the test);
      `earth_has_no_abiotic_methane_or_ammonia`; `a_dynamo_shields_the_air`;
      `a_deep_ocean_holds_back_what_the_mantle_outgasses` (Earth above 0.99999
      released, a 30 % water world in [5e-4, 2e-3]);
      `a_runaway_keeps_its_water_as_vapour`; `no_xuv_keeps_the_whole_air`
      (Review Focus 2); `each_partial_cites_its_outgassing_and_the_shoreline`
      (the cause tree: `derived_from` of a partial is its outgassed node and the
      shoreline node; SO2's also the wash-out node; the pressure's are the
      partials).
- [ ] **Step 2: Run** `cargo nextest run -p system planet::air` — FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** — PASS.
- [ ] **Step 5: Commit** `feat(system): port outgassing, the cosmic shoreline
      and the SO2 wash-out`.

### Task 10: The `bare` chain (opus)

**Files:**

- Create: `crates/system/src/planet/bare.rs`,
  `crates/system/src/planet/bare_tests.rs`
- Modify: `mod.rs` (module docs: "What exists today")

**Interfaces:**

- Consumes: every step above, `draws::draw`, `interior::interior`, `heat::heat`.
- Produces: `pub(crate) struct Bare { pub trial: Trial, pub drawn: Drawn, pub
  body: Body, pub heat: timeline::Heat, pub envelope_lost:
  Option<Traced<Kilogram>>, pub surface: Option<BareSurface> }`, `pub(crate)
  struct BareSurface { pub regime: Traced<TectonicRegime>, pub core:
  timeline::Core, pub water: timeline::Water, pub atmosphere: Atmosphere }`, and
  `pub(crate) fn bare(trial: &Trial, seed: Seed, trace: &mut Trace) ->
  Result<Bare, InteriorError>`, in the order of `mod` l.480-640 and l.895-1075:
  draw, interior, escape, heat; no surface at or above `MIN_ENVELOPE`; water;
  after a runaway `after_steam` then interior and heat again; regime; core; air;
  after a runaway whose shoreline took vapour, `after_steam` and interior once
  more, keeping the heat (legacy `mod` l.577-590). 2d replaces the end of the
  chain with the climate.

- [ ] **Step 1: Write the failing tests:**
      `the_same_seed_gives_the_same_bare_planet` (two runs, equal values);
      `an_enveloped_planet_has_no_surface` (a 10 M⊕ mini-Neptune at 1 AU);
      `a_runaway_planet_is_lighter` (Venus: mass below the trial's, the mass
      node derives from the steam loss);
      `every_surface_value_traces_back_to_the_trial` (walk every node of
      `BareSurface` to leaves: only `Input`, `Drawn`, `Observed` or `Imposed`
      leaves, no orphan).
- [ ] **Step 2: Run** `cargo nextest run -p system planet::bare` — FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** — PASS; `cargo nextest run --profile ci` within the tier-1
      budget.
- [ ] **Step 5: Commit** `feat(system): chain the planet from its draw to its
      bare air`.

### Task 11: Documentation and CI (sonnet)

**Files:**

- Modify: `docs/science/planet-atmospheres.md` (each section's "How it computes
  them": rewrite paths, the cause trees), `crates/system/README.md` (the planet
  chain up to the bare air), `crates/science/README.md` if it lists modules,
  `.markdownlint-cli2.jsonc` (add this plan's path)

- [ ] **Step 1:** Update the pages: each law points at its `science` item and
      its step at its `system` module; the departures from legacy (decisions
      above, the values study 21 changed) in "Where it departs".
- [ ] **Step 2: Run** `mise run ci` — PASS, tier budgets included.
- [ ] **Step 3: Commit** `docs: describe the planet chain up to the bare air`.

### Before the PR (main session)

- `branch-reviewer` and `test-reviewer` once on the whole branch, given
  this plan and the split design; after fixes, `mise run ci` again.
- Push Wiki study 21 (written in the scratchpad clone) and link it.
- PR description: `Refs #602`; a heading "Calibrations" listing every
  `Calibration:` constant added (from study 21's table) and any retune
  of Task 8; a note that nothing generated changes (model private
  until 2d), so no `mise run test:heavy` output.
- Tell mapgen3 when the 2d design is written (commitment of PR 2).
