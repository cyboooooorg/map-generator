# Planet model PR 2d-3a: the climate solver Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port from `legacy-final` the climate solver of a planet with a
surface (wet balance, albedo and emissivity fixed point, condensation,
collapse, thermostat, maximum greenhouse, snowball branches and their
draw, the reducing-air fallback), with the laws it needs in `science`
(outer-edge fit, ices III and V, `condense`, frost emissivity, locked
rotators, the radiative balance) and the #646 refit, without changing
what is generated.

**Architecture:** Laws go to `science`: `habitable`, `water`,
`volatiles`, `surface` (new `condense.rs`) and a new top-level
`balance` module (the radiative balance of one column, its root finders,
the moist plateau and the runaway albedo). The solver goes to a new
`system::planet::climate` directory, crate-private under the
`expect(dead_code, …)` pattern of `planet/mod.rs`. A new chain step,
`planet/settled.rs`, runs `bare_with` then the climate and the
reducing-air fallback, and records the climate's cause nodes. `bare`
and `bare_with` keep their signatures and their cost, so 2c's tests do
not run the climate. Nothing is wired into `draw_with`, and
`ENGINE_VERSION` stays 3.

**Tech Stack:** Rust, `cargo nextest` tiers, the `Trace`/`Traced` cause
API of `timeline`, `science::radiation::{column, albedo}` with their
explicit caches.

**Spec:** `docs/superpowers/specs/2026-11-23-planet-model-pr2d-design.md`
("2d-3: the climate" with the split and the user's decisions, "Science
to study first", "Follow-ups", "Rules for the build"). Study: Wiki
[25-climate-solver-audit](https://github.com/cyboooooorg/map-generator/wiki/25-climate-solver-audit)
and `docs/science/planet-atmospheres.md`, "The climate solver of the
rewrite (study 25)" (l.~2310-2516; grep the heading). Recon:
`.superpowers/sdd/2026-11-24-planet-model-pr2d-3a/recon.md`. Legacy
paths below are under `legacy-final:crates/mapgen/src/planet/`:
`climate` is `climate.rs`, `atm` is `atmosphere.rs`, `mod` is `mod.rs`.
Out of scope (2d-3b): `boils`, `Water.boiled` derived, `hides_surface`,
the steam envelope.

## Decisions

From the spec and study 25 (user decisions of 2026-11-24):

- **Snowball:** the target is Hoffman et al. 2017's dated share,
  f⊕ = 0.014 (0.013 to 0.016). `SNOWBALL_WARM_BAR` is refitted by the
  study's method, `W = d⊕ (1 − f⊕) / (rate⊕ f⊕)`, with `d⊕` the extra
  CO₂ the reference Earth at 1 AU needs to thaw (computed by the code)
  and `rate⊕` Earth's recorded volcanism over
  `EARTH_VOLCANIC_INTENSITY` (about 1.012). Expect about 6.4 bar.
- **Outer edge:** Kopparapu et al. 2014's maximum-greenhouse fit lands
  in `science::habitable`. Its caller is a tier-3 test that measures
  the model's edge against it. The gap is a documented departure
  (#595), not a bound. Legacy's 0.42 albedo and 1.535 AU bounds are not
  ported.
- **Mars:** 214 ± 5 K (NASA fact sheet) on an imposed air of 6.36 mbar:
  CO₂ 6.05 mbar, N₂ 0.165 mbar, O₂ 0.010 mbar. `MARS_CO2_FACTOR` is not
  ported.
- **Triton (#673):** `N2_ICE_EMISSIVITY` 0.80 (`Source:` Lellouch et al.
  2011 via Meza et al. 2019). `FROST_ALBEDO` keeps 0.75, relabelled a
  calibration on Triton's 38 ± 1 K (Tryka et al. 1993/1994).
- **NH₃** stays frozen by the cold trap only: `condense` handles N₂ then
  CH₄.
- **Ten numerical calibrations** under the #648 rule (study table "Solver
  constants"): reported to the user, no objection. Each is held by the
  convergence test the table names, in the tier it names.
- **#670:** `tectonics::HEAT_PIPE_FLUX` becomes `HEAT_PIPE_ONSET_FLUX`,
  and `relief::volcanoes::HEAT_PIPE_FLUX` becomes `FULL_VOLCANISM_FLUX`.
  The law is unchanged. The docs already use the new names.
- **Spin as an input:** the climate takes an obliquity and a `locked`
  flag as traced inputs (`Rotation`). It never reads `Spin`. #677
  decides in 2d-4 where `class_of` gets them.
- **Several stars (#592):** the settle keeps one scalar albedo, the
  flux-weighted mean `Σ wᵢ Aᵢ`, where `wᵢ = fluxᵢ / Σ flux` and `Aᵢ` is
  the shortwave budget at source `i`'s effective temperature. Then
  `S₀/4 · Σ fluxᵢ (1 − Aᵢ) = S₀/4 · Σ flux · (1 − Σ wᵢ Aᵢ)` exactly. The
  runaway albedo is weighted the same way: `A_rg,i` uses
  `runaway_flux(Teffᵢ)`, so the plateau binds exactly where
  `Σ fluxᵢ / S_rg,i = 1`, the multi-star limit `habitable::runaway_flux`
  documents. The longwave, the skin and the emissivity do not depend on
  the star.
- **#593 lever 2:** the maximum greenhouse is searched once per body,
  on the inventory air, and kept across the condensation steps and
  `frozen_at`. Legacy rebuilt it on each step's condensed air
  (`climate` l.1750-1756, 1975-1983). For an air in which nothing
  condenses (Earth) the values are unchanged; a tier-3 test proves the
  outer edge does not move (#593's acceptance condition). Where N₂ or
  CH₄ condense, the change is a stated departure, measured in tier 3
  on a Titan-like body and reported in the PR.
- **thaw_bar infinite** cannot be recorded, because the trace refuses
  non-finite numbers. A bistable body records a flag `planet.climate.thaws`
  (derived from the inputs). Only when it is true does it record
  `planet.climate.thaw-co2` (Bar). The chance derives from whichever
  exists and from `planet.surface.volcanism`. "Thaws false" gives
  chance 1, as study 25 says ("the chance 1 is recorded without it").
- **The snowball draw** is a new label, `planet/snowball`, in
  `draws::LABELS`. Its value is one uniform in (0, 1) from
  `seed.derive(LABELS.snowball)`. It is recorded as a drawn leaf
  (`planet.climate.snowball-draw`) only for a bistable body. Its value
  does not depend on the trace, so the same planet gives the same
  answer.

Where the code contradicts the recon, the study or the spec:

- **N₂ above its triple point** (user, 2026-11-24). The study's
  `condense` reads "the pure gas's saturation pressure". The rewrite has
  only the Antoine fit (`volatiles::nitrogen_vapour_pressure`,
  `pub(crate)`), 12.5 % under the triple-point pressure, so `condense`
  would step at 63.14 K. Legacy's Clausius-Clapeyron curve is not ported
  either. Task 3 ports the ancillary vapour-pressure equation of the N₂
  reference equation of state (Span, Lemmon, Jacobsen, Wagner & Yokozeki
  2000, *J. Phys. Chem. Ref. Data* 29, 1361), with its critical point
  from the same paper, as `volatiles` already does for CO₂ and CH₄. It
  replaces the Antoine fit, and `methane_freezes` reads it: one curve
  for one value. The switch from frost to liquid stays at
  `NITROGEN_TRIPLE_TEMPERATURE` (Angus et al. 1979), where the frost fit
  is pinned. Span et al.'s triple point is read only to report the gap
  between the two sourced triple-point pressures, which bounds the
  join.
- **Frozen for good** (user, 2026-11-24). A body frozen with no CO₂ able
  to thaw it (legacy: frozen, not a snowball) records a new contract
  variant, `ClimateState::Frozen => "frozen"`
  (`crates/timeline/src/planet.rs:218`). mapgen2 and mapgen3 agree, and
  nothing outside `timeline` matches on the enum. Task 9 adds it; there
  is no separate `frozen` flag. `Frozen` is a body with a water
  inventory, frozen over, not a snowball, which no reachable CO₂ thaws
  (bistable with no thaw, or frozen on the temperate branch past the
  outer edge). Dry bodies and hydrocarbon or nitrogen sea worlds stay
  `Temperate`, as in legacy.
- **"Frozen over"** reads the melting curve of the stable ice
  (`water::melting_temperature(P)`, Task 2), the curve `liquid` reads,
  instead of legacy's 273.15 K: one code path for one value (#671).
- **Volcanic SO₂ frost** (legacy `volcanic_so2_frost_bar`, #525) has no
  port in the rewrite: it needs the non-thermal loss of #658, and #678
  tracks it. `ClimateInputs` takes `so2_frost: Bar` as a field, and its
  one setter is `settled`, which passes `Bar(0.0)` with a code comment
  citing #678.
- **The moist plateau** exists only as private helpers of
  `radiation/column_tests.rs` (`plateau_w_m2`, `plateau_skin`,
  `with_vapour`, `MOIST_PLATEAU_W_M2`, `SIMPSON_NAKAJIMA_W_M2`). Task 5
  makes them public in `science::balance`, and the column tests read
  the public functions: one code path. `with_vapour` becomes
  `surface::surface_vapour`, `plateau_skin` becomes `balance::skin`.
- **SO₂ wash-out:** 2c's `air` already applies `so2_kept`
  (`planet.air.so2-kept`). Legacy's `washed`, a second wash inside the
  albedo, is not ported. The deck reads the bare air's SO₂.
- **`REFERENCE_ALBEDO`** (0.3), which the study does not port, is also
  legacy's `warm_start` albedo. `Light` carries each source's
  `equilibrium` (`StellarSource::equilibrium`, the timeline's, at the
  sources' reference albedo), and the warm start is legacy's: a pass
  from the equilibrium temperatures, the maximum of that and
  `ICE_FREE_TEMPERATURE`, the vapour at it. No new constant is added.
- **`ICE_FREE_K`** has no rewrite name. Task 4 adds
  `surface::ICE_FREE_TEMPERATURE = ICE_LINE_TEMPERATURE + 2/3 ·
  ICE_LINE_CONTRAST` (a `Definition:`, legacy `albedo.rs:244`).
- **Carbon inventory.** Legacy read the outgassed CO₂ (`out.co2`; `lost`
  is the non-thermal loss, absent here). 2c's `Air` does not return it.
  Task 7 adds `outgassed_co2: Traced<Bar>` to `Air` and `BareSurface`
  (the `planet.air.outgassed.co2` node).
- **`titan_like_air`** reads methane's triple-point pressure. The
  temperature is `pub(crate)` in `volatiles`, so the predicate lives in
  `science::surface` (Task 3).
- **`bare` stays cheap.** The spec's "extend `bare`" becomes a new step,
  `settled`, over `bare_with`. Otherwise every 2c test of `bare` (the
  loop in `bare_tests.rs:92` included) would pay for the climate.
- **The reducing fallback** reruns only the air and its runaway
  follow-up, as legacy (`mod` l.889-893). `bare_with` splits into a
  private `bare_ground` (interior, escape, heat, water, regime,
  volcanism, core: everything before `air`) and `bare_air(ground,
  hold_back)` (`air`, already taking `hold_back`, and the `after_steam`
  follow-up). `bare_with` is `bare_air(bare_ground(..), true)`.
- **`wet_climate`'s steam fallback** is a placeholder that 2d-3b's
  `boils` loop replaces; `settled` records it until then.
- **Where the constants live.** Laws and physical calibrations go to
  `science`: `THERMOSTAT_K`, the solver's `Solver`/`SOLVER` (`TOLERANCE`,
  `ITERATIONS`, `BRACKET_TOLERANCE` together) and the bracket literals
  in `balance`; `SNOWBALL_WARM_BAR` and `snowball_chance` in a new
  `science::snowball`. Only the numerical constants of the solver's own
  loops stay in `system::planet::climate` (`SCAN_RATIO`,
  `COLLAPSE_TOLERANCE`, `COLLAPSE_ITERATIONS`, `THAW_STEPS`,
  `ALBEDO_TOLERANCE_K`, `ALBEDO_ITERATIONS`, `ALBEDO_DAMPING`,
  `CONDENSE_ITERATIONS`, `THERMOSTAT_TOLERANCE`, `GREENHOUSE_STEPS`,
  `GREENHOUSE_DECADES`, `RUNAWAY_PROBE_K`). `Numerics::DEFAULT` reads
  `science::balance::SOLVER` for the first three.
- **Ice VII** (IAPWS R14-08 eq. 5) is not ported: a choice. Above
  2216 MPa the melting temperature is held at 355 K, and the limit text
  says so; the PR mentions it on #671.

## Global Constraints

- Legacy is read only at tag `legacy-final`. Each function is ported
  from the lines named, then changed only where the Decisions say so.
- Every numeric `const`/`static`, and every `let` bound to a bare number
  other than 0, 0.5, 1 or 2, in `science` and in `system`, test files
  included, has a doc comment starting with `Source:`, `Calibration:` or
  `Definition:`. Prefix, value and citation come from study 25's table
  and `planet-atmospheres.md`, never from the legacy comment. "From
  memory" and "not checked" notes are not carried over. Any other
  untagged number stops the work.
- Values (crates: Decisions, "Where the constants live"):
  `THERMOSTAT_K` 288 K (Calibration, NASA Earth fact sheet);
  `STEAM_ALBEDO_BAR` 1 bar (Calibration); `RUNAWAY_ALBEDO` (0, 0.9)
  (Definition); `RUNAWAY_PROBE_K` 380 K (Definition); `TOLERANCE_K`
  0.01 K, `BRACKET_TOLERANCE_K` `TOLERANCE_K / 100`, `SCAN_RATIO` 1.03,
  `PLATEAU_SURFACES` 500 to 640 K by 20 K, `COLLAPSE_TOLERANCE` 1e-4,
  `THAW_STEPS` 12, `ALBEDO_TOLERANCE_K` 0.1 K, `THERMOSTAT_TOLERANCE`
  1e-3 in ln CO₂, `GREENHOUSE_STEPS` 18 (Calibration);
  `ITERATIONS` 60, `COLLAPSE_ITERATIONS` 100, `ALBEDO_ITERATIONS` 200,
  `ALBEDO_DAMPING` 0.5, `CONDENSE_ITERATIONS` 100,
  `GREENHOUSE_DECADES` 4 (Definition); the bracket literals
  `OLR_FLOOR` 1e-3 (of the absorbed flux), `OVERSHOOT` 1.0005 and
  `LEAST_STEP` 1.001 (`climate` l.383-384 and 1170-1177, Definition, in
  `balance`). `S₀` is `constants::SOLAR_IRRADIANCE`. Test inputs (an
  albedo of 0.3, 11 bar of N₂, 2 AU…) are named constants of their test
  file, tagged like any other.
- Observed windows cite their source with a stated tolerance: Bond
  albedos from the NASA fact sheets (Mercury 0.088, Venus 0.76, Earth
  0.306, Mars 0.250), Earth's cloud cover 0.68 ± 0.03 (Stubenrauch et
  al. 2013, as `radiation/clouds.rs`), never "legacy l.N".
- Missing science stops the work. The implementer reports the stop to
  the main session, which brings it to the user. No test is `#[ignore]`d
  and no tolerance is widened to get past a stop. An observed bound
  (Earth 288 K, Mars 214 K, Titan, CERES, Kopparapu's ordering, Venus's
  pinned 577 K per #594) that fails is reported with its numbers, never
  retuned.
- `science` takes and returns the typed units and `PartialPressures` at
  its boundary, never `timeline` types. There is no `HashMap`,
  `thread_local!`, file, clock or process in `science` or `system`. The
  column and shortwave caches are owned by the solve call and passed as
  `&mut`. A `OnceLock` static is allowed for Earth's moist plateau, as
  `stellar/tables.rs` does.
- New `system` code stays crate-private under the
  `#[cfg_attr(not(test), expect(dead_code, …))]` of `planet/mod.rs`. Do
  not touch `crates/system/src/draw/**`, `placement.rs`,
  `ENGINE_VERSION` or `draw_with`. `timeline` changes only by the
  `ClimateState::Frozen` variant (Task 9).
- Tier 1 has 2 minutes of wall time on 4 threads, with the workspace
  crates built at opt-level 0. A climate solve is costly. Each task
  reports the time of each new tier-1 test (from
  `target/nextest/tier1/junit.xml`). A test over 10 s, or a tier-1 run
  over 110 s, is reported to the main session before going on. Moving a
  test to tier 3 to fit the budget is the user's call (CLAUDE.md,
  Tests). Tier-3 tests are `#[ignore = "heavy: <why>"]`. Legacy's edge
  test took about 5 s per Earth climate in release; at opt-level 0
  (`Cargo.toml:49-50`) it is slower. Task 7 starts by timing one full
  Earth solve in the tier-1 profile and reports it. Before Tasks 9 and
  10, if two full bistable-Earth solves would push tier 1 over 110 s,
  the implementer stops and reports which tests share a solve or go to
  tier 3; the user decides.
- `mise run ci` passes at the end of every task.

## Review Focus

1. **A body with almost no light** (a planet at 50 AU, or a source flux
   of 1e-6 S⊕): every solve returns a finite, positive temperature, at
   least the internal heat's `(q/σ)^¼`. Legacy's `bracket` stopped at
   1e6 K, so check the floor as well. Test:
   `a_dark_body_is_warmed_by_its_heat_alone` (Task 5) and
   `far_planets_only_freeze` (Task 9).
2. **Several stars of different temperatures:** the albedo is the
   flux-weighted mean of the per-source budgets, and two identical
   half-flux sources give exactly one source. Tests:
   `the_albedo_is_the_flux_weighted_budgets` (Task 7) and
   `two_half_suns_are_one_sun` (Task 8).
3. **A bistable body whose thaw is out of reach:** no non-finite number
   reaches the trace, the chance is 1, and the trace validates. Test:
   `an_unreachable_thaw_records_no_infinity` (Task 10).
4. **A body no volcanism feeds** (intensity 0): the chance is 1 and not
   a NaN from `0/0`. Test: `snowball_chance_follows_its_law` (Task 9).
5. **Condensing air across a phase point** (the N₂ triple point, a
   methane sea freezing): `condense` is continuous across
   `NITROGEN_TRIPLE_TEMPERATURE` within the reported gap between the two
   sourced triple points, and the fixed point reports `converged`. Tests:
   `nitrogen_saturation_joins_its_frost` (Task 3) and
   `a_triton_like_body_settles` (Task 8). The sweeps run in tier 3
   (Task 11).

---

### Task 1: Heat-pipe names and the outer-edge fit (sonnet)

**Files:**

- Modify: `crates/science/src/tectonics.rs:21` and
  `tectonics_tests.rs:24`, `crates/science/src/relief/volcanoes.rs:20,39,48`
  and `volcanoes_tests.rs:25-26`, `crates/system/src/planet/regime.rs:10,18,50`
  (renames).
- Modify: `crates/science/src/habitable.rs` and
  `crates/science/src/habitable_tests.rs`.

**Interfaces:**

- Produces:
  - `tectonics::HEAT_PIPE_ONSET_FLUX` (1 W/m²);
  - `relief::volcanoes::FULL_VOLCANISM_FLUX` (2.5 W/m²);
  - `pub fn maximum_greenhouse_flux(effective_temperature: Kelvin) ->
    EarthFlux` in `habitable`, which is `0.356 + aT + bT² + cT³ + dT⁴`
    with `T = Teff − FIT_ORIGIN` clamped to `FIT_MIN..FIT_MAX` like
    `runaway_flux`. The coefficients are `OUTER_A` 6.171e-5, `OUTER_B`
    1.698e-9, `OUTER_C` −3.198e-12 and `OUTER_D` −5.575e-16, and the
    constant term is `OUTER_SOLAR_FLUX` 0.356. Each is a `Source:`
    Kopparapu et al. 2014, Table 1, maximum greenhouse, as the runaway
    fit cites it.

- [ ] **Step 1: Write the failing test** `the_outer_edge_is_kopparapus`:
  - `maximum_greenhouse_flux(5780 K)` is 0.356 within 1e-12;
  - at 5772 K it gives an edge `1/√S` of 1.677 AU within 0.002 (study:
    1.676 AU round the Sun);
  - at 3300 K it is 0.2411 within 5e-4 (the published coefficients,
    evaluated by hand);
  - it rises from 2600 to 7200 K in 100 K steps;
  - it is below `runaway_flux` at every one of those temperatures;
  - 2000 K equals 2600 K, and 9000 K equals 7200 K (the clamp).

- [ ] **Step 2: Run** `cargo nextest run -p science habitable`. Expected:
  FAIL to compile.

- [ ] **Step 3: Implement** the function, and the two renames with
  their doc comments. The onset is "the regime's onset, a calibration";
  the full-volcanism flux is "Io's flux, where volcanism is total". The
  module doc gains one paragraph on the outer edge. Use `sed` over the
  files listed, then `cargo build`.

- [ ] **Step 4: Run** the same command and `cargo nextest run -p system
  regime`. Expected: PASS. Then `mise run ci`. Expected: green.

- [ ] **Step 5: Commit** in two:
  `refactor(science): rename the heat-pipe fluxes` (`Closes: #670`),
  then `feat(science): add the outer-edge fit of Kopparapu 2014`.

### Task 2: Ices III and V and the stable ice (sonnet)

**Files:**

- Modify: `crates/science/src/water.rs` (its inline `mod tests`,
  l.~882), `crates/science/src/surface/liquids.rs:118-160` and
  `liquids_tests.rs`.

**Interfaces:**

- Produces in `water`:
  - `pub fn ice_iii_melting_pressure(t: Kelvin) -> Pascal`, which is
    `π = 1 − 0.299948 (1 − θ⁶⁰)`, `θ = T / 251.165 K`,
    `π = p / 208.566 MPa` (IAPWS R14-08 eq. 2), valid to 256.164 K;
  - `pub fn ice_v_melting_pressure(t: Kelvin) -> Pascal`, which is
    `π = 1 − 1.18721 (1 − θ⁸)`, `θ = T / 256.164 K`,
    `π = p / 350.1 MPa` (eq. 3), valid to 273.31 K;
  - `pub const III_V_LIQUID_TEMPERATURE: Kelvin` (256.164, `Source:`
    IAPWS R14-08 Table 1);
  - `pub fn melting_temperature(p: Pascal) -> Kelvin`, the melting
    temperature of the ice stable at `p`:
    - Ih up to 208.566 MPa;
    - III up to 350.1 MPa;
    - V up to 632.4 MPa;
    - VI up to 2216 MPa;
    - above that, 355 K, with the limit stated: ice VII (IAPWS R14-08
      eq. 5) is not ported, a choice (Decisions).

    III, V and VI are inverted in closed form:
    `θ = (1 − (1 − π)/a)^(1/b)`.

- [ ] **Step 1: Write the failing tests**
  - `ices_iii_and_v_melt_on_the_iapws_curves`:
    - `ice_iii_melting_pressure(254 K)` is 268.685 MPa within 1e-5
      relative (IAPWS R14-08 Table 3 check value);
    - `ice_v_melting_pressure(265 K)` is 479.640 MPa within 1e-5;
    - each curve gives its triple-point pressures at both ends within
      1e-4 relative (208.566 and 350.1 MPa for III; 350.1 and 632.4 MPa
      for V).

    If a check value differs, re-read Table 3 of R14-08. Do not change
    the tolerance.
  - `melting_follows_the_stable_ice`:
    - at 101 325 Pa it equals `ice_ih_melting_temperature`;
    - at 300 MPa it is 254.96 K within 0.01 K;
    - at 500 MPa it lies in (256.164, 273.31) K;
    - at 1 GPa it lies in (273.31, 355) K;
    - each result's own curve gives back `p` within 1e-6 relative;
    - it is continuous at the three triple points within 1e-4 K, the
      published curves' own consistency (V/VI differ by 3.2e-5 K, VI/VII
      by 7.7e-5 K): the test comment says so;
    - its minimum over 0.1 to 2000 MPa is 251.165 K at 208.566 MPa;
    - at 3 GPa it is 355 K.
  - `water_under_ice_iii_pressure_freezes` (#671), with 1 `OCEAN_MASS`
    and an N₂ air of 3000 bar (300 MPa):
    - `liquid` at 254 K is not `Water` (legacy's Ih floor said it was);
    - at 256 K it is `Water`.

- [ ] **Step 2: Run** `cargo nextest run -p science water liquids`.
  Expected: FAIL.

- [ ] **Step 3: Implement.** The water window of `liquid` becomes
  `[melting_temperature(P), boiling_temperature(P))`. Remove its
  "Limit:" paragraph. Rewrite the module doc of `water.rs` to name ices
  III and V.

- [ ] **Step 4: Run** the same command. Expected: PASS, the 2d-2 liquid
  tests included. Then `mise run ci`. Expected: green.

- [ ] **Step 5: Commit**
  `feat(science): melt water on the curve of the stable ice`
  (`Closes: #671`).

### Task 3: Condensation at the surface (opus)

**Files:**

- Modify: `crates/science/src/volatiles.rs` (the Antoine fit l.~482-500
  is replaced), `volatiles_tests.rs`,
  `crates/science/src/surface/liquids.rs` (`methane_freezes` reads the
  new curve), `docs/science/planet-atmospheres.md` (l.~1105-1115 and
  l.~2226-2240: the liquid N₂ curve, and its `## Sources` entry).
- Create: `crates/science/src/surface/condense.rs`,
  `crates/science/src/surface/condense_tests.rs`, and the `mod` lines in
  `surface/mod.rs`.

**Interfaces:**

- Consumes: `volatiles::{nitrogen_frost_pressure,
  methane_saturation_pressure}`, `liquids::{sea_freezes, frozen_share,
  SEA_METHANE_SHARE}`.
- Produces:
  - `pub fn nitrogen_saturation_pressure(t: Kelvin) -> Pascal`:
    - the frost curve below `NITROGEN_TRIPLE_TEMPERATURE`;
    - from there, the ancillary vapour-pressure equation of Span et al.
      2000 (`ln(p/p_c) = (T_c/T) Σ Nᵢ θ^kᵢ`, `θ = 1 − T/T_c`, the form
      and coefficients as the paper gives them);
    - `f64::INFINITY` at and above the paper's critical temperature, as
      methane's curve is.

    Its coefficients and exponents, `NITROGEN_CRITICAL_TEMPERATURE` and
    `NITROGEN_CRITICAL_PRESSURE` are `Source:` Span et al. 2000, with
    the table or equation number, read in the paper. A copy (CoolProp's
    ancillary) is used only if each coefficient is checked term by term
    against the paper's equation. **If the paper cannot be reached, the
    task stops and reports**: no value from memory or from an uncredited
    copy. The paper's triple-point pressure is a `Source:` constant of
    `volatiles_tests.rs`, used only for the join's bound.
  - `pub struct Condensed { pub air: PartialPressures, pub condensate:
    PartialPressures }`;
  - `pub fn condense(inventory: &PartialPressures, t: Kelvin) ->
    Condensed`, a port of `atm` l.760-880:
    - N₂ first, with activity 1;
    - then CH₄, with activity `SEA_METHANE_SHARE + (1 −
      SEA_METHANE_SHARE) · frozen_share(t, sea_freezes(air.n2,
      condensate))`;
    - each gas keeps at most `activity · p_sat(t)`;
    - `air + condensate == inventory`;
  - `pub fn condensed_air(air: &PartialPressures, inventory:
    &PartialPressures, t: Kelvin) -> PartialPressures`: `air` with its N₂
    and CH₄ replaced by `condense(inventory, t).air`'s (`atm` l.885);
  - `pub fn titan_like_air(air: &PartialPressures) -> bool`, which is
    `air.ch4 > 0` and `air.total() > methane_saturation_pressure(methane
    triple point)` in bar (`atm` l.1183, study "The reducing-air
    fallback", a `Definition:`).

- [ ] **Step 1: Write the failing tests**
  - `nitrogen_saturation_is_span_2000`: at four or more temperatures of
    the paper's saturation table, normal boiling point (77.355 K,
    0.101325 MPa) included, the curve is within the ancillary's stated
    deviation (the paper's figure for it, quoted in the doc comment).
  - `nitrogen_saturation_joins_its_frost` (Review Focus 5):
    - the frost curve at `NITROGEN_TRIPLE_TEMPERATURE − 1e-9` K and the
      vapour curve at that temperature agree within the relative gap
      between the two sourced triple-point pressures
      (`NITROGEN_TRIPLE_PRESSURE`, Angus, and Span et al.'s), computed
      in the test from both constants. The frost fit states no
      deviation of its own, so no other figure is assumed. If the join
      exceeds that gap, stop and report;
    - it is infinite at the critical temperature;
    - it rises strictly from 30 K to the critical point in 0.1 K steps;
    - at 90.69 K, `methane_freezes`'s only point, print its gap to the
      Antoine fit's 3.82 bar for the commit message.
  - The 2d-2 bounds of `methane_freezes` stay green, unchanged. If one
    fails, stop and report.
  - `titans_air_holds_its_sea_vapour` (study, "Condensation"), with N₂
    1.384 bar at 93.65 K:
    - an inventory of 1 bar of CH₄ keeps 0.0845 bar within 5e-4 in the
      air, and the condensate is the rest within 1e-12;
    - the measured 0.0829 bar of CH₄ condenses nothing;
    - N₂ never condenses at 93.65 K.
  - `methane_ice_holds_its_full_saturation` (legacy `climate` l.3035):
    below the sea's freezing point less `PHASE_BAND`, the air's CH₄ is
    `methane_saturation_pressure(t)` within 1e-9 relative.
  - `nitrogen_condenses_first`: Triton-like, 10 bar of N₂ at 38 K keeps
    `nitrogen_frost_pressure(38 K)` in the air within 1e-9 relative, and
    the rest is condensate.
  - `condensation_keeps_the_inventory`: over N₂ in {0, 1e-3, 1, 10} bar,
    CH₄ in {0, 1e-3, 1, 10} bar and T from 30 to 200 K in 5 K steps,
    `air + condensate == inventory` within 1e-12, and no part is
    negative or NaN.
  - `condensed_air_changes_only_what_condenses`: CO₂, H₂O, NH₃, SO₂ and
    O₂ of `air` pass through unchanged.
  - `titan_like_air_is_dense_and_methane_bearing`:
    - Titan's surface air is `true`;
    - N₂ 1.4e-5 bar with a CH₄ trace is `false`;
    - Earth's air is `false`.

- [ ] **Step 2: Run** `cargo nextest run -p science condense volatiles
  liquids`. Expected: FAIL.

- [ ] **Step 3: Implement** as specified. Remove the Antoine fit and its
  constant. Update the doc comment of `methane_freezes`. In
  `planet-atmospheres.md`, replace the Antoine and NIST
  Clausius-Clapeyron text with Span et al. 2000 and its deviation, and
  add the paper to `## Sources`.

- [ ] **Step 4: Run** the same command. Expected: PASS. Then
  `mise run ci`. Expected: green.

- [ ] **Step 5: Commit**
  `feat(science): condense nitrogen and methane at the surface`.

### Task 4: Frost emissivity and locked rotators (sonnet)

**Files:**

- Modify: `crates/science/src/surface/cover.rs`, `cover_tests.rs`,
  `liquids.rs`, `liquids_tests.rs`.

**Interfaces:**

- Produces in `surface`:
  - `pub const N2_ICE_EMISSIVITY: f64` (0.80, `Source:` Lellouch et al.
    2011 as Meza et al. 2019, § 4, adopt it);
  - `pub fn frost_emissivity(frost: &PartialPressures) -> Fraction`,
    which is `1 − (1 − N2_ICE_EMISSIVITY) · frost_cover(N₂ alone)`
    (`climate` l.113-131);
  - the doc of `FROST_ALBEDO` becomes the study's `Calibration:` on
    Triton's 38 ± 1 K, with Hillier's gap. The value is unchanged;
  - `pub const ICE_FREE_TEMPERATURE: Kelvin` (`Definition:`, see
    Decisions);
  - `pub fn locked_sunlit_ice(area: Fraction) -> Fraction`, which is
    `max(0, 2a − 1)²` (`Definition:`, study "Tidally locked rotators");
  - `cover` gains a last parameter `locked: bool`. When it is true,
    `ice_sunlit` is `locked_sunlit_ice(line) · reach · (1 − frost)`.
    When it is false, it is as before. The "slow or tidally locked
    rotator is out of scope" sentence goes;
  - `pub fn substellar_temperature(absorbed: WattPerSquareMetre) ->
    Kelvin`, which is `(4 · absorbed / σ)^¼`: Kite et al. 2016 eq. 1,
    `√2 · T_eq` under the body's own albedo (`Definition:`);
  - `liquid` gains a last parameter `substellar: Option<Kelvin>`. A
    locked body passes `Some`, and the lava test reads `max(t,
    substellar)`.

- [ ] **Step 1: Write the failing tests**
  - `nitrogen_frost_radiates_less_than_a_black_body` (legacy `climate`
    l.3153):
    - no frost gives 1;
    - `FROST_LAYER` of N₂ gives 0.80 within 1e-12;
    - half the layer gives 0.90;
    - 10 bar of CO₂ frost alone gives 1.
  - `triton_frost_balances_at_38_k`: with `FROST_ALBEDO`, the orbit-mean
    1.508 W/m² (NASA Neptune fact sheet) and `N2_ICE_EMISSIVITY`,
    `((1 − A) F / (4 ε σ))^¼` is in 37 to 39 K (Tryka et al. 1993: 38 ± 1
    K; study: 37.97 K).
  - `a_locked_body_freezes_from_its_night_side`:
    - `locked_sunlit_ice` is 0 at 0.5 and below, 0.25 at 0.75 and 1 at 1;
    - Earth's cover at 270 K: the ice line is about 0.386, so locked
      `ice_sunlit` is 0 and the zonal one about 0.29. At 250 K both
      would be 1 (under `ICE_COVERED_TEMPERATURE`). The implementer
      computes the expected values from the formulas, not from the
      code.
  - `a_locked_rock_melts_at_its_substellar_point`:
    - `substellar_temperature(σ · 1300⁴)` is `√2 · 1300` K within 1e-9;
    - a dry body with no air at a mean of 1300 K gives no liquid when
      `substellar` is `None`;
    - with `Some(substellar_temperature(σ · 1300⁴))` it gives `Lava`.
  - Update the call sites of `cover` and `liquid` in their tests with
    `false` and `None`.

- [ ] **Step 2: Run** `cargo nextest run -p science surface`. Expected:
  FAIL.

- [ ] **Step 3: Implement** as specified.

- [ ] **Step 4: Run** the same command. Expected: PASS. Then
  `mise run ci`. Expected: green.

- [ ] **Step 5: Commit**
  `feat(science): add the frost emissivity and the locked-rotator laws`
  (footers `Refs: #672`, `Refs: #673`, `Refs: #675`).

### Task 5: The radiative balance in `science::balance` (opus)

**Files:**

- Create: `crates/science/src/balance.rs`,
  `crates/science/src/balance_tests.rs`, and `pub mod balance` in
  `lib.rs`.
- Modify: `crates/science/src/radiation/column_tests.rs`. Its private
  `plateau_w_m2`, `plateau_skin`, `with_vapour`, `MOIST_PLATEAU_W_M2`,
  `SIMPSON_NAKAJIMA_W_M2`, `PLATEAU_SURFACES_K`, `PLATEAU_ITERATIONS`
  and `PLATEAU_SETTLED_W_M2` go. The plateau tests call
  `balance::moist_plateau` and `earth_moist_plateau`; `moist_olr`,
  `moist_olr_on` and the tests at l.~342, 409-410 and 502-503 call
  `surface::surface_vapour` for `with_vapour` and `balance::skin` for
  `plateau_skin`. `radiation/test_points.rs`: `EARTH_AIR` aliases
  `balance::EARTH_DRY_AIR`.

**Interfaces:**

- Consumes: `radiation::column::{outgoing_longwave, skin_temperature,
  Profile, ColumnCache}`, `radiation::methane::stratospheric_share`,
  `surface::surface_vapour`, `water::{boiling_temperature,
  TRIPLE_POINT_PRESSURE}`, `habitable::runaway_flux`.
- Produces:
  - `pub struct Solver { pub tolerance: Kelvin, pub iterations: usize,
    pub bracket_tolerance: Kelvin }` and `pub const SOLVER: Solver`
    (`TOLERANCE` 0.01 K, `ITERATIONS` 60, `BRACKET_TOLERANCE`
    `TOLERANCE / 100`);
  - `pub const THERMOSTAT_K: Kelvin` (288 K, Calibration, Global
    Constraints), and the bracket literals `OLR_FLOOR`, `OVERSHOOT` and
    `LEAST_STEP` (Definition), which `first_balance` reads too;
  - `pub struct Point { pub x: f64, pub f: f64 }`;
  - `pub struct Root { pub x: f64, pub iterations: usize }` and
    `pub fn regula_falsi(f: impl FnMut(f64) -> f64, low: Point, high:
    Point, tolerance: f64, iterations: usize) -> Root` (Illinois,
    `climate` l.339-378). `Root::iterations` lets a test check that a
    solve stopped by tolerance, under the cap;
  - `pub fn bracket_up(excess: impl FnMut(f64) -> f64, absorbed: f64,
    start: Point) -> (Point, Point)` (l.380-394). It stops at
    `HOTTEST` (1e6 K, a `Definition:` guard);
  - `pub fn absorbed(starlight: WattPerSquareMetre, gases:
    &PartialPressures, heat: WattPerSquareMetre) -> WattPerSquareMetre`,
    which is `starlight · (1 − stratospheric_share(gases,
    skin(starlight))) + heat` (l.291-297);
  - `pub fn skin(starlight: WattPerSquareMetre) -> Kelvin`, which is
    `skin_temperature((starlight/σ)^¼)`;
  - `pub fn surface_temperature(starlight: WattPerSquareMetre, gases:
    &PartialPressures, heat: WattPerSquareMetre, emissivity: Fraction,
    gravity: MetrePerSecondSquared, solver: Solver, cache: &mut
    ColumnCache) -> Kelvin` (l.658-697, `Profile::CLIMATE`, the
    emissivity clamped to 0.01 to 1 as a `Definition:` guard);
  - `pub const PLATEAU_SURFACES: [Kelvin; 8]`, plus the `Definition:`s
    `PLATEAU_ITERATIONS` 8 and `PLATEAU_SETTLED` 0.01 W/m²;
  - `pub const SIMPSON_NAKAJIMA_W_M2: WattPerSquareMetre` (282, moved
    from `column_tests.rs:30-37` with its `Source:`);
  - `pub fn moist_plateau(dry: &PartialPressures, water: Bar, gravity:
    MetrePerSecondSquared, profile: Profile, surfaces: &[Kelvin], start:
    WattPerSquareMetre, cache: &mut ColumnCache) -> WattPerSquareMetre`
    (l.446-472 and 624-656);
  - `pub const EARTH_DRY_AIR: PartialPressures` (`Definition:`, the
    text of `test_points::EARTH_AIR`), and
    `test_points::EARTH_AIR` becomes an alias of it, so its seven user
    files stay untouched;
  - `pub fn earth_moist_plateau() -> WattPerSquareMetre`: a `OnceLock`
    of `moist_plateau(EARTH_DRY_AIR, …)` on `STANDARD_GRAVITY` and
    `Profile::PLATEAU`, started from `SIMPSON_NAKAJIMA_W_M2`. A body's
    own plateau starts from `earth_moist_plateau()` (legacy
    `moist_plateau_of`, l.624-656), so the initialisation never
    re-enters itself;
  - `pub const RUNAWAY_ALBEDO: (f64, f64)` (0, 0.9) and
    `pub fn runaway_albedo(star: Kelvin, plateau: WattPerSquareMetre) ->
    Fraction`, which is `1 − 4 F / (runaway_flux(star) · S₀)`, clamped
    (l.550-560, `REFERENCE_ALBEDO` not ported);
  - `pub const STEAM_ALBEDO_BAR: Bar` (1) and `pub fn steam_share(steam:
    Bar) -> Fraction`, which is `1 − exp(−steam/STEAM_ALBEDO_BAR)`
    (l.475-545).

- [ ] **Step 1: Write the failing tests**
  - `more_greenhouse_gas_is_warmer` (legacy l.3339), under the starlight
    of a 255 K body:
    - an air of N₂ 1 bar and CO₂ 1e-3 bar is warmer than 255 K;
    - CO₂ 1 bar is warmer still;
    - no air gives 255 K within 1e-9;
    - 100 W/m² of heat warms the airless body.
  - `a_dark_body_is_warmed_by_its_heat_alone` (Review Focus 1): with a
    starlight of 1e-9 W/m² and a heat of 0.05 W/m², with no air and with
    1 bar of N₂, the result is finite and at least `(0.05/σ)^¼` (30.6 K).
  - `regula_falsi_finds_a_crossing`: `x³ − 2` on (1, 2) gives `2^⅓`
    within the tolerance. `bracket_up` brackets the crossing of
    `σT⁴ − 240` from 100 K.
  - `iterations_is_converged` (`ITERATIONS`): `surface_temperature` on
    Earth's air, on Titan's column and on 1 bar of N₂ under a dark
    starlight stops by tolerance, `Root::iterations` under `ITERATIONS`.
  - Column-only checks moved out of `system` (`Profile::CLEAR` is
    `#[cfg(test)] pub(crate)`, `radiation/column.rs:503`): Mars's and
    Titan's surface airs at their measured surface temperatures give
    the same `outgoing_longwave` under `Profile::CLEAR` and
    `Profile::CLIMATE` within 1e-9 relative (no cloud greenhouse,
    legacy l.2628). Earth's CERES clear-sky check already exists
    (`spectrum_tests.rs:60-64`, 267.9 W/m², Kato et al. 2018) and is not
    duplicated.
  - `the_runaway_albedo_binds_the_plateau_at_the_limit` (legacy
    l.3996):
    - for 2600, 5772 and 7200 K,
      `(1 − runaway_albedo(T, F)) · runaway_flux(T) · S₀ / 4` equals `F`
      within 1e-9 relative, with `F = earth_moist_plateau()`;
    - `runaway_albedo(5772 K, F)` is 0.247 within 0.005 (study);
    - over 2600 to 7200 K it stays in 0.087 to 0.357 within 0.005
      (study).
  - `a_runaways_light_follows_its_steam_down_to_none` (legacy l.4032
    and l.2739):
    - `steam_share(0)` is 0, and `steam_share(1 bar)` is `1 − e⁻¹` within
      1e-12;
    - the share rises with the steam;
    - the blend `A + (A_rg − A) · share` at 1e-9 bar is within 1e-8 of
      `A`.
  - `an_airless_column_emits_from_the_surface` (legacy l.4168) and
    `thin_steam_stays_on_the_plateau_at_680_k` (legacy l.4207): port
    each only if `column_tests.rs` has no equivalent. Grep first and
    list the duplicates in the commit message.
  - The column plateau tests (`the_moist_plateau_is_goldblatts`,
    `the_runaway_anchor_is_flat`, `the_plateaus_grid_is_converged`,
    `the_plateau_ignores_co2_and_nitrogen`) pass unchanged on the public
    functions.

- [ ] **Step 2: Run** `cargo nextest run -p science balance column`.
  Expected: FAIL to compile.

- [ ] **Step 3: Implement** as specified. In the module doc, write that
  the balance of one column is a law (radiative-convective equilibrium)
  and that the solver which settles a body lives in
  `system::planet::climate`.

- [ ] **Step 4: Run** the same command. Expected: PASS, with
  `the_moist_plateau_is_goldblatts` no slower. Then `mise run ci`.
  Expected: green.

- [ ] **Step 5: Commit**
  `feat(science): add the radiative balance of a surface`.

### Task 6: `CH4_OPACITY` refit on Titan's column (opus)

**Files:**

- Modify: `crates/science/src/radiation/methane.rs:47` (value and doc),
  `crates/science/src/balance_tests.rs`.

**Interfaces:**

- Consumes: `balance::{absorbed, surface_temperature, SOLVER}`,
  `radiation::test_points::{TITAN_SURFACE_AIR, TITAN_GRAVITY,
  TITAN_EFFECTIVE_K, TITAN_SURFACE_K}`.

- [ ] **Step 1: Write the failing test**
  `titans_column_sits_at_its_measured_surface`: the starlight `σ · 82⁴`,
  `TITAN_SURFACE_AIR`, no heat, emissivity 1 and `TITAN_GRAVITY` give
  93.65 ± 0.25 K (Fulchignoni et al. 2005). Run it on the current 13.4
  and record the temperature it gives.

- [ ] **Step 2: Run** `cargo nextest run -p science titans_column`.
  Expected: FAIL, or a pass by chance; report the value either way.

- [ ] **Step 3: Refit.** Bisect `CH4_OPACITY` by editing the constant
  and rerunning the test until the column gives 93.65 K to three
  figures. Keep the bracket and each step's temperature for the doc
  comment. The doc becomes a `Calibration:` that names the target, the
  column (1.467 bar, 5.65 % CH₄, 82 K), the method, the old value, and
  McKay et al. 1991's 82 + 21 − 9 K. Rewrite the "not refitted" text.

- [ ] **Step 4: Run** `mise run ci`. Expected: green, including
  `the_methane_anti_greenhouse_matches_mckay` and the column's Titan
  tests. A test that fails on the new value stops the work; report it.

- [ ] **Step 5: Commit**
  `fix(science): refit the methane opacity on Titan's column`
  (`Closes: #646`).

### Task 7: The climate body and its balances (opus)

**Files:**

- Create: `crates/system/src/planet/climate/mod.rs` (the module doc,
  `ClimateInputs`, `Light`, `Rotation`, `Numerics`),
  `climate/body.rs`, `climate/wet.rs`, `climate/carbon.rs`,
  `climate/fixture.rs` (test only), `climate/wet_tests.rs` and
  `climate/carbon_tests.rs`.
- Modify: `crates/system/src/planet/mod.rs` (the `climate` module under
  the `expect(dead_code)` block), `air.rs` and `bare.rs`
  (`outgassed_co2: Traced<Bar>` on `Air` and `BareSurface`).

**Interfaces:**

- Consumes: Tasks 3 to 5, `science::surface::{cover, ground_albedo,
  surface_vapour, frost, frost_emissivity}`,
  `science::radiation::{albedo::shortwave_budget, clouds::cloud_cover,
  veils::sulphuric_deck}`, `science::outgassing::column_pressure`,
  `science::volatiles::co2_saturation_pressure`.
- Produces:
  - `pub(crate) struct Light { pub flux: EarthFlux, pub
    effective_temperature: Kelvin, pub equilibrium: Kelvin }`, filled
    from `StellarSource::{flux, effective_temperature, equilibrium}`;
  - `pub(crate) struct Rotation { pub obliquity: Traced<Degree>, pub
    locked: Traced<bool> }`;
  - `pub(crate) struct ClimateInputs`, with plain values:
    - `sources: Vec<Light>`;
    - `air: PartialPressures` (the bare air, the inventory of what
      condenses);
    - `water: Kilogram`, `runaway: bool`, `plates: bool`;
    - `heat_flux: WattPerSquareMetre`, `gravity: MetrePerSecondSquared`,
      `radius: Metre`;
    - `carbon: Bar` (outgassed CO₂), `frozen_co2: Bar`, `so2_frost: Bar`;
    - `volcanism: Fraction`, `obliquity: Radian`, `locked: bool`.

    `ClimateInputs::of(bare: &Bare, rotation: &Rotation, so2_frost:
    Bar) -> Option<ClimateInputs>` builds it, `None` without a surface.
    `settled` is the one caller that sets `so2_frost` (Decisions,
    #678);
  - `pub(crate) struct Numerics`, with `solver: Solver`, one field per
    `system` constant of the Decisions ("Where the constants live"),
    `probe: Option<Kelvin>` and `greenhouse_once: bool`.
    `Numerics::DEFAULT` reads `science::balance::SOLVER` and the tagged
    constants. `probe: None` always scans the saturated column.
    Convergence tests swap one field;
  - `pub(crate) struct ClimateBody<'a>` (legacy `Body`, `climate`
    l.1605-1785). It holds the inputs, the numerics, `co2:
    Option<Bar>`, the cached `greenhouse: Option<Bar>` and `plateau:
    Option<WattPerSquareMetre>`, and its `ColumnCache` and
    `ShortwaveCache`. Methods take `&mut self`:
    - `water_column() -> Bar` (`column_pressure`);
    - `wet() -> bool`;
    - `starlight(albedo: Fraction) -> WattPerSquareMetre`, which is
      `S₀/4 · Σ flux · (1 − albedo)`;
    - `derived_albedo(t, air) -> Fraction`, the flux-weighted
      `shortwave_budget` per source over
      `Underneath { ground_albedo(cover(t, water if wet else 0, radius,
      frost, obliquity, locked)), cloud_cover(t, air.h2o),
      sulphuric_deck(t, air.so2) }`;
    - `emissivity_at(t, air) -> Fraction`;
    - `frost(t, air) -> PartialPressures`
      (`liquids::frost(condense(inventory, t).condensate, …)`);
    - `plateau() -> WattPerSquareMetre` (l.1672);
    - `runaway_albedo(derived) -> Fraction`, the per-source `A_rg`
      weighted by flux and blended by `steam_share(air.h2o)` when
      `runaway` and some water is kept;
    - `greenhouse_co2() -> Bar` (searched once: Decisions, #593
      lever 2);
  - in `wet.rs`:
    - `wet_balance(body, starlight, air, emissivity) ->
      Option<(PartialPressures, Kelvin)>` (l.1109-1156, with the probe
      when `numerics.probe` is `Some`);
    - `first_balance(excess, absorbed, bare, numerics) -> Option<f64>`
      (l.1158-1194, the #600 secant scan, capped at
      `CRITICAL_POINT_TEMPERATURE`);
    - `wet_climate(…) -> (PartialPressures, Kelvin)` (l.1196-1231; its
      steam fallback is 2d-3b's placeholder, Decisions);
    - `collapsed_climate(…) -> Collapsed { air, temperature,
      iterations }` (l.1233-1270; `iterations` for the convergence
      test);
  - in `carbon.rs` (reading `balance::THERMOSTAT_K`):
    - `thermostat(body, starlight, emissivity) -> (PartialPressures,
      Kelvin, bool)` (l.2274-2350);
    - `maximum_greenhouse(body) -> Bar` (l.2352-2423).

- [ ] **Step 0: Time one full Earth solve.** Before the tests, time
  `bare_with(EARTH)` plus a thermostat and a collapse at 1 AU in the
  tier-1 profile (opt-level 0), and report it to the main session
  (Global Constraints).

- [ ] **Step 1: Write the failing tests** (a `fixture.rs` helper builds
  `ClimateInputs` from `reference_trial` and `bare_with`, Earth's
  obliquity 23.44°, unlocked, NASA fact sheet)
  - `the_albedo_is_the_flux_weighted_budgets` (Review Focus 2): Earth's
    inputs with two sources, 0.7 S⊕ at 5772 K and 0.3 S⊕ at 3300 K. At
    288 K, `derived_albedo` equals `0.7 A(5772) + 0.3 A(3300)` within
    1e-12, each `A` from `shortwave_budget` over the same `Underneath`.
  - `a_thick_nitrogen_earth_keeps_a_balance` (legacy l.4180): Earth's
    inputs with 11 bar of N₂, under the starlight of albedo 0.3, have a
    `wet_balance` (both inputs named, tagged constants of the test
    file).
  - `the_probe_changes_no_balance` (`RUNAWAY_PROBE_K`, Definition): the
    same balance within 1e-12 with `probe: None`, at 1.0, 0.95 and 0.9
    AU.
  - `the_thermostat_holds_earth_at_288_k`, Earth at 1 AU under albedo
    0.3:
    - it is regulated;
    - the temperature is 288 K within 0.05 K;
    - the CO₂ is at most `greenhouse_co2()`, which is at most `carbon`.
  - `the_thermostat_tolerance_is_converged` (Calibration, tier 1): with
    `thermostat_tolerance` 1e-4, the temperature moves under 0.01 K.
  - `a_collapse_keeps_what_the_surface_holds`: Earth's inputs at 2 AU
    with 10 bar of CO₂ give an air CO₂ at most
    `co2_saturation_pressure(T) · (1 + COLLAPSE_TOLERANCE)`.
  - `collapse_iterations_is_converged` (`COLLAPSE_ITERATIONS`): the
    study's collapsing Earth at 1.6 AU stops by tolerance, its
    `iterations` under `COLLAPSE_ITERATIONS`.
  - `the_maximum_greenhouse_lies_inside_its_range`
    (`GREENHOUSE_DECADES`): 5 decades give the same CO₂ within 0.2 %
    (the golden section's resolution).

- [ ] **Step 2: Run** `cargo nextest run -p system climate`. Expected:
  FAIL to compile.

- [ ] **Step 3: Implement** as specified. Port each function from the
  lines named. Absorbed light goes through `balance::absorbed`, the
  column through `outgoing_longwave`, the vapour through
  `surface_vapour`, the root finding through `balance::{regula_falsi,
  bracket_up}`. Put no number in `system` that `science` already holds.

- [ ] **Step 4: Run** the same command. Expected: PASS. Report each
  test's time. Then `mise run ci`. Expected: green.

- [ ] **Step 5: Commit**
  `feat(planet): add the climate body and its balances`.

### Task 8: The settle and the condensation fixed point (opus)

**Files:**

- Create: `crates/system/src/planet/climate/settle.rs`,
  `climate/settle_tests.rs`.

**Interfaces:**

- Consumes: Task 7.
- Produces:
  - `pub(crate) struct Pass { air, temperature, regulated, starlight,
    converged }` (l.1950-1965);
  - `under_albedo(body, albedo, emissivity) -> Pass` (l.1967-2080, with
    `condense`/`condensed_air` from Task 3);
  - `warm_start(body) -> (Fraction, Fraction)` (l.2146-2174: a pass
    from the sources' `equilibrium`, the maximum with
    `ICE_FREE_TEMPERATURE`, the vapour at it; Decisions). A test that
    uses Earth's 255 K calls it a start, not the 254.6 K balance;
  - `settle(body, start) -> Settle`, where `Settle` holds the air, `T`,
    `T_eq`, the Bond albedo after the runaway blend, the emissivity,
    `regulated`, `converged`, the condensate and the liquid
    (l.2176-2272, the #593 secant step). The liquid passes
    `substellar_temperature(starlight)` when `locked`;
  - `frozen_at(body, co2) -> Settle` (l.1749-1785, from
    `ICE_COVERED_TEMPERATURE`). `Settle` also carries the condensation
    loop's `iterations`.

- [ ] **Step 1: Write the failing tests**
  - `mars_sits_at_its_measured_temperature`: Mars's inputs
    (`reference_trial(MARS)`, obliquity 25.19°) with the imposed air of
    the Decisions:
    - `T` is 214 ± 5 K (NASA Mars fact sheet);
    - no liquid;
    - the Bond albedo is 0.250 ± 0.05 (NASA Mars fact sheet);
    - the clear-sky check is a `science` test (Task 5).
  - `two_half_suns_are_one_sun` (#592, Review Focus 2): the Mars case
    under one source of flux F, and under two sources of F/2 at the
    same temperature. `T`, `T_eq` and the albedo agree within 1e-9
    relative.
  - `no_water_no_ocean` (l.3238): Earth's inputs with `water` 0 give no
    `Water`, not regulated.
  - `a_methane_greenhouse_cannot_warm_a_cold_world` (l.2810): the
    legacy `cold_methane_world` case stays under 150 K.
  - `a_hot_locked_earth_has_lava` (the law's test is Task 4's
    `a_locked_rock_melts_at_its_substellar_point`): Earth's inputs at
    0.035 AU, dry, with no air:
    - unlocked, no liquid (`T` near 1445 K);
    - locked, `Lava`.
  - `a_triton_like_body_settles` (Review Focus 5): 10 bar of N₂ under
    Triton's flux and no water. It is converged, at 37 to 39 K.
  - `the_fixed_points_are_converged` (Calibrations and Definitions,
    tier 1, on the Mars and Triton-like cases):
    - `TOLERANCE_K / 10` moves `T` under 0.01 K;
    - `ALBEDO_TOLERANCE_K` 0.01 K moves it under 0.1 K;
    - doubling `ALBEDO_ITERATIONS` and `CONDENSE_ITERATIONS` changes
      nothing (bitwise);
    - `CONDENSE_ITERATIONS` on a Titan case (`reference_trial(TITAN)`):
      the condensation loop stops by tolerance, its `iterations` under
      the cap (Triton in tier 3, Task 11);
    - `BRACKET_TOLERANCE_K / 10` moves it under 0.01 K.

- [ ] **Step 2: Run** `cargo nextest run -p system climate`. Expected:
  FAIL.

- [ ] **Step 3: Implement** as specified.

- [ ] **Step 4: Run** the same command. Expected: PASS. Report the
  times. Then `mise run ci`. Expected: green.

- [ ] **Step 5: Commit**
  `feat(planet): settle the albedo, emissivity and condensation`.

### Task 9: Branches, snowball chance and draw (opus)

**Files:**

- Create: `crates/science/src/snowball.rs`, `snowball_tests.rs` and
  `pub mod snowball` in `lib.rs`;
  `crates/system/src/planet/climate/branches.rs` and
  `climate/branches_tests.rs`.
- Modify: `crates/system/src/planet/draws.rs` (`Labels.snowball`,
  `LABELS.snowball = "planet/snowball"`, `pub(crate) fn
  snowball_draw(seed: Seed) -> f64`; the test labels of `draw_with` gain
  the field), `crates/timeline/src/planet.rs:218`,
  `crates/timeline/src/keyframe_tests.rs:116`.

**Interfaces:**

- Consumes: Tasks 7 and 8, `science::relief::volcanoes::EARTH_VOLCANIC_INTENSITY`.
- Produces:
  - in `science::snowball`: `pub const SNOWBALL_WARM_BAR: Bar`
    (Calibration, refitted in Step 3) and `pub fn snowball_chance(extra:
    Option<Bar>, volcanism: Fraction) -> Fraction`, with `extra = thaw −
    froze` (legacy `FrozenBranch::chance`, l.1883-1889). It is `d / (d +
    W · v / EARTH_VOLCANIC_INTENSITY)`, and 1 when `extra` is `None` or
    `v` is 0 (l.1575-1598, reading the recorded intensity);
  - `pub(crate) struct Bistable { pub froze: Bar, pub still_frozen: Bar,
    pub thaw: Option<Bar>, pub chance: Fraction }`;
  - `branches(body) -> (Settle, Option<Bistable>)` (l.1861-1935);
  - `ClimateState::Frozen => "frozen"` in `timeline::planet` (the
    `contract_enum!` at l.218), with a line in
    `keyframe_tests.rs::contract_enums_report_their_serialised_name`:
    `ClimateState::Frozen` serialises as `"frozen"`;
  - `pub(crate) struct Solved { pub settle: Settle, pub state:
    ClimateState, pub bistable: Option<Bistable> }`;
  - `pub(crate) fn solve(inputs: &ClimateInputs, draw: f64) -> Solved`
    and `solve_with(inputs, draw, &Numerics) -> Solved`
    (`climate_over`, l.1475-1525):
    - `state` is `Runaway` when `runaway`;
    - it is `Snowball` when the body is bistable, `draw < chance`, the
      frozen branch is frozen over and the thaw is `Some`;
    - it is `Frozen` when the body has a water inventory, is frozen
      over (`T` below `water::melting_temperature(P)`, Decisions), is
      not a snowball, and no reachable CO₂ thaws it: bistable with no
      thaw, or frozen on the temperate branch past the outer edge;
    - it is `Temperate` otherwise, dry bodies and hydrocarbon or
      nitrogen sea worlds included.

- [ ] **Step 1: Write the failing tests**
  - `snowball_chance_follows_its_law` (Review Focus 4, in
    `science/src/snowball_tests.rs`):
    - `None` gives 1;
    - a volcanism of 0 gives 1, and not NaN;
    - `Some(d)` with `v = EARTH_VOLCANIC_INTENSITY` gives
      `d/(d + W)` within 1e-12;
    - it rises with `d` and falls with `v`.
  - `the_snowball_draw_is_a_seed_draw`: `snowball_draw(s)` is in
    (0, 1) and is the same twice.
  - `earth_snowballs_as_often_as_hoffman_dates` (Calibration held by
    its target), Earth's inputs at 1 AU:
    - it is bistable with a finite thaw, and a water inventory;
    - its chance is in 0.013 to 0.016 (Hoffman et al. 2017);
    - `solve(…, 1.0)` is `Temperate` and `Water`;
    - `solve(…, 0.0)` is `Snowball`, with no liquid, a Bond albedo
      above 0.5 and `T` under 260 K (legacy l.3752).
  - `far_planets_only_freeze` (legacy l.3706, Review Focus 1): Earth's
    inputs at 5 and 10 AU are not bistable, their state is `Frozen`, and
    `solve(…, 0.0) == solve(…, 1.0)`.
  - `a_dry_cold_body_is_temperate`: Earth's inputs at 10 AU with
    `water` 0 are `Temperate`, not `Frozen`.

- [ ] **Step 2: Run** `cargo nextest run -p system climate draws`.
  Expected: FAIL.

- [ ] **Step 3: Implement, then refit `SNOWBALL_WARM_BAR`.**
  - Print `d⊕ = thaw − froze` and `v⊕` for Earth.
  - Set `W = d⊕ (1 − 0.014) / ((v⊕ / EARTH_VOLCANIC_INTENSITY) · 0.014)`
    to three figures.
  - The doc comment is a `Calibration:` that gives the target, the
    method, `d⊕` and `v⊕` as measured, and the study link.
  - If Earth is not bistable, or `W` falls outside 3 to 13 bar (twice
    the study's 6.4 either way), stop and report.

- [ ] **Step 4: Run** the same command. Expected: PASS. Report the
  times; the Earth test is the costliest of the plan. Then
  `mise run ci`. Expected: green.

- [ ] **Step 5: Commit** in two: first
  `feat(timeline): add the frozen climate state` (the variant and its
  contract test), then
  `feat(planet): add the snowball branches, chance and draw`.

### Task 10: The chain step, its causes and the fallback (sonnet)

**Files:**

- Create: `crates/system/src/planet/climate/record.rs`,
  `crates/system/src/planet/settled.rs`,
  `crates/system/src/planet/settled_tests.rs`.
- Modify: `crates/system/src/planet/codes.rs`, `bare.rs` (the split of
  the Decisions: private `bare_ground` and `bare_air(ground, hold_back,
  trace)`; `bare_with` is `bare_air(bare_ground(..), true)`), `mod.rs`
  (the modules and the module doc).

**Interfaces:**

- Consumes: Tasks 7 to 9, `science::surface::{titan_like_air,
  condense}`.
- Produces:
  - codes (`planet.climate.` prefix unless said):
    - `surface-temperature`, `equilibrium`, `bond-albedo`, `emissivity`,
      `state`, `regulated`, `converged`;
    - for a bistable body: `froze-co2`, `thaws`, `thaw-co2`,
      `snowball-chance`, `snowball-draw`;
    - `air.<gas>` for the seven gases, and `air.pressure`;
    - `condensate.n2` and `condensate.ch4`, recorded when positive;
    - `titan-like`, for a reducing body only;
    - `planet.surface.liquid`;
  - `pub(crate) struct Climate { pub global: GlobalClimate, pub
    emissivity: Traced<Fraction>, pub regulated: Traced<bool>, pub
    converged: Traced<bool>, pub atmosphere:
    Atmosphere, pub condensate: BTreeMap<Gas, Traced<Bar>>, pub liquid:
    Option<Traced<LiquidKind>>, pub snowball: Option<SnowballNodes> }`
    and `pub(crate) struct SnowballNodes { pub froze: Traced<Bar>, pub
    thaws: Traced<bool>, pub thaw: Option<Traced<Bar>>, pub chance:
    Traced<Fraction>, pub draw: Traced<Fraction> }`. `SurfaceLiquid`'s
    inventory and share are 2d-4's;
  - `record(solved, bare, rotation, draw, trace) -> Result<Climate,
    InteriorError>`, with these causes, `I` being:
    - each source's flux and effective temperature;
    - the bare partials;
    - water inventory and runaway;
    - the three heats;
    - radius, gravity, regime and volcanism;
    - `frozen_co2` and `outgassed_co2`;
    - obliquity and locked.

    The nodes derive as follows:
    - `froze-co2`, `thaws` and `thaw-co2` from `I`;
    - the chance from `[thaws, thaw-co2 if any, froze-co2, volcanism]`;
    - the draw is a drawn leaf;
    - `state` from `I` plus the chance and the draw;
    - `T` from `I` plus `state`;
    - albedo, emissivity, regulated and converged from `[T] + I`;
    - `equilibrium` from the albedo and the fluxes;
    - `air.<gas>` from `[its bare partial, T]`, plus `outgassed_co2` and
      `frozen_co2` for CO₂ and the water inventory for H₂O;
    - each condensate from `[its bare partial, T]`;
    - the liquid from `[T, air.pressure, water inventory, runaway,
      locked]` and the condensates, plus each source's flux and the
      albedo when locked (the substellar temperature);
  - `pub(crate) struct Settled { pub bare: Bare, pub climate:
    Option<Climate> }`;
  - `pub(crate) fn settled(trial, seed, rotation, trace)`;
  - `settled_with(trial, drawn, rotation, snowball: Seed, trace)`:
    - `bare_ground`, then `bare_air(ground, true)`, then the climate
      (`ClimateInputs::of(…, Bar(0.0))`, citing #678);
    - if `reducing` is true and `titan_like_air(climate air)` is false,
      `bare_air(ground, false)` and the climate again (`mod`
      l.889-893): the ground is not rebuilt;
    - the discarded pass's nodes stay in the trace, as 2c's rebuilt
      bodies do.

- [ ] **Step 1: Write the failing tests** (`settled_tests.rs`, one body
  per test so that they run in parallel)
  - `earth_has_its_climate` merges legacy l.2463 (Earth), 2509, 2560,
    2591, 2614 and 2641. Through `settled_with(EARTH)`, at Earth's
    obliquity, unlocked:
    - `T` is 288 ± 6 K (NASA fact sheet), with `Water`, regulated and
      CO₂ under 0.01 bar;
    - the Bond albedo is 0.306 ± 0.03 (NASA Earth fact sheet);
    - the cloud cover is 0.68 ± 0.03 (Stubenrauch et al. 2013);
    - the column-only CERES checks live in `science` (Task 5);
    - 280 ppm (Myhre et al. 1998) under its own albedo gives 286 to
      290 K through `wet_climate`, and the thermostat's CO₂ lies
      between the CO₂s that give 286 and 290 K;
    - the snowball chance is recorded, in 0.013 to 0.016.
  - `venus_is_pinned_where_the_column_puts_it` (#594, ported as is):
    577 ± 5 K, no liquid, a Bond albedo of 0.76 ± 0.05 (NASA Venus fact
    sheet). If it fails, stop and report.
  - `titan_has_its_seas`:
    - 94 ± 8 K (Fulchignoni 2005; a validation now that #646 targets
      the column);
    - `Hydrocarbons`;
    - the air's CH₄ is at most the `condense` cap at its `T`;
    - `titan-like` is true, and its outgassed CO₂ is 0;
    - the state is `temperate` (a hydrocarbon sea world).
  - `mercury_sits_at_its_equilibrium_temperature` (l.3177):
    - `T` is within 1 K of `T_eq` under its own albedo;
    - no liquid, not regulated, the state `temperate` (dry);
    - a Bond albedo of 0.088 ± 0.04 (NASA Mercury fact sheet).
  - `the_equilibrium_is_the_starlight_under_the_albedo`: on Venus,
    `σ T_eq⁴` equals `starlight(albedo)` within 1e-9 relative (l.2560).
  - `the_climate_records_its_causes` (Mercury and Mars; Mars's state
    is `temperate`):
    - each code above is present when its condition holds;
    - `T`'s causes include every source's flux and temperature node;
    - `trace.graph().validate()` passes.
  - `an_unreachable_thaw_records_no_infinity` (Review Focus 3): `record`
    of a hand-built `Solved` with `Bistable { thaw: None, chance: 1 }`
    records `froze-co2`, `thaws` false, no `thaw-co2`, a chance of 1
    and the state `frozen`; the graph validates.
  - `a_reducing_air_keeps_its_co2_only_when_titan_like`:
    - Titan keeps its held-back air;
    - for the other branch, find a reducing body whose climate air is
      not Titan-like (Callisto, or an icy 0.0036 M⊕ trial at 30 AU) and
      assert its final air was solved with its CO₂ (`reducing` false).
      If no chain body reaches that branch, test the decision on
      hand-built airs and report it.
  - `same_planet_same_climate`: `settled` of Earth (bistable, so the
    snowball draw is exercised) from one seed, on a clone of the trace
    and on the trace, gives the same values (the rule "same planet,
    same answer"). It doubles the costliest solve: if Task 7's timing
    gate says it does not fit tier 1, it merges into Task 11's
    `earth_twice_is_the_same_earth` and the report says why. A
    Mars-like body is not used: it never draws.

- [ ] **Step 2: Run** `cargo nextest run -p system settled`. Expected:
  FAIL.

- [ ] **Step 3: Implement** as specified. The module doc of
  `planet/mod.rs` gains `climate` and `settled` in "What exists today"
  and drops "the climate … come with the next pull requests" for
  2d-3a's part. It names what remains (`boils`, steam envelope, life,
  class). The doc of `bare.rs` points to `settled`.

- [ ] **Step 4: Run** the same command. Expected: PASS. Report the time
  of each test. Then `mise run ci`. Expected: green. If tier 1 goes
  over 110 s, stop and report the times.

- [ ] **Step 5: Commit**
  `feat(planet): record the climate in the chain`.

### Task 11: Heavy tests (sonnet)

**Files:**

- Create: `crates/system/src/planet/climate/heavy_tests.rs`, with every
  test `#[ignore = "heavy: <why>"]`.

**Interfaces:**

- Consumes: Tasks 7 to 10, and `habitable::maximum_greenhouse_flux`. A
  helper `with_star(trace, trial, teff)` replaces the source's
  effective temperature, as `with_fluence` replaces its fluence.

- [ ] **Step 1: Write the tests**
  - `the_outer_edge_against_kopparapu_2014` (#595, legacy l.3765-3890),
    for Earth at 5772, 4500 and 3300 K, bisected between 1 and 3 AU in
    12 steps, with `solve(…, 1.0)` (the temperate branch) and each
    orbit's inputs rebuilt from the trial at that orbit (as legacy
    `reference.rs::derive_at`), not one body's inputs with a scaled
    flux:
    - the edge exists;
    - the edge flux falls with the star's temperature, as the fit does;
    - print `S/S_fit − 1` for each star. No bound is frozen: the gap
      goes to the PR description and #595.
  - `one_greenhouse_search_per_body_moves_no_edge` (#593 lever 2): the
    Sun's edge with `greenhouse_once` false and true agrees within
    0.001 AU.
  - `one_greenhouse_search_on_a_condensing_air` (#593 lever 2,
    Decisions): a Titan-like body solved with `greenhouse_once` false
    and true; print the gap in `T` for the PR, no bound.
  - `condense_iterations_is_converged_on_triton`: the Triton-like case
    stops by tolerance under `CONDENSE_ITERATIONS`.
  - Convergence tests of the study's tier-3 rows:
    - `SCAN_RATIO` 1.01: the same first balance for Earth and over
      `S_rg × {0.9, 0.95, 0.99}`;
    - `PLATEAU_SURFACES` every 10 K to 700 K: the plateau moves under
      0.5 W/m²;
    - `COLLAPSE_TOLERANCE` 1e-5: under 0.01 K;
    - `THAW_STEPS` 16: Earth's thaw CO₂ moves under 0.1 %;
    - `ALBEDO_DAMPING` 0.25: the converged climates agree within
      `ALBEDO_TOLERANCE_K`;
    - `GREENHOUSE_STEPS` 30: the edge moves under 0.001 AU;
    - `PHASE_BAND`: the two sweeps below.
  - Legacy ports, bounds unchanged unless a Decision changed them:
    - `the_runaway_edge_steps_only_by_the_branches_albedos` (l.2658);
    - `census_182_d` (l.3121);
    - `the_thermostat_never_exceeds_the_outgassed_co2` (l.3191);
    - `an_earth_with_two_or_three_bar_of_nitrogen_stays_temperate`
      (l.3385);
    - `doubling_the_nitrogen_warms_a_late_archean_earth_by_a_few_kelvin`
      (l.3404);
    - `an_earth_track_planet_stays_temperate_to_the_maximum_greenhouse`
      (l.3525; the orbits measured in the edge test replace 1.5 AU);
    - `co2_builds_up_past_the_outgassed_snapshot` (l.3557);
    - `a_frozen_state_no_co2_thaws_is_no_snowball` (l.3617);
    - `a_dry_acid_air_settles_through_the_deck_onset` (l.3649);
    - `snowballs_are_rare_near_earth_and_common_near_the_edge` (l.3726,
      near in 0.013 to 0.016, far above near);
    - `a_snowball_holds_the_co2_its_thermostat_reaches` (l.3892);
    - `a_planet_just_past_the_runaway_limit_runs_away_hot` (l.4105);
    - `the_fixed_point_converges_across_the_methane_freezing_point`
      (l.4334);
    - `the_fixed_point_converges_across_the_nitrogen_triple_point`
      (l.4357);
    - `earth_twice_is_the_same_earth` (same planet, same answer, on a
      bistable body).
  - Not ported:
    - the Kopparapu albedo and the 1.535 AU edge (l.3816 and 3847,
      Decisions);
    - the serde tests (l.4230 and 4236);
    - 2d-3b's tests (l.3289 boils, l.3939 and 3978 steam);
    - tests already in `science`: methane l.2829, 3060 and 3099;
      plateau l.4075, 4125 and 4151; liquids l.2868 to 3018 and 3255
      (2d-2);
    - the l.3249 lava test, replaced by the locked-rock tests.

- [ ] **Step 2: Run** `mise run test:heavy -- climate`. Expected: PASS,
  or a failing legacy bound reported with its numbers. Put the time and
  the printed gaps in the report.

- [ ] **Step 3: Commit** `test(planet): port the heavy climate tests`.

### Task 12: Documentation (sonnet)

**Files:**

- Modify:
  - `docs/science/planet-atmospheres.md`: l.~1281, 1368, 1409,
    1608-1626, 1813, 1864-1867, 2018-2041, 2078-2100, 2251 and
    2270-2306 (grep each; the page is 3 260 lines);
  - `docs/science/habitable-zone.md` (the outer-edge paragraph);
  - `crates/science/README.md` (`balance`, `surface::condense`).

- [ ] **Step 1:** Make the older paragraphs agree with the study-25
  sub-section and with Tasks 1 to 11's measured values:
  - the outer edge is the measured gap against Kopparapu 2014, not
    "1.535 AU frozen";
  - Mars is 214 ± 5 K on the fact sheet's air, without
    `MARS_CO2_FACTOR`;
  - `N2_ICE_EMISSIVITY` is 0.80;
  - `CH4_OPACITY` takes its new value and target;
  - `SNOWBALL_WARM_BAR` takes its refitted value and f⊕ 0.014;
  - the calibration-targets table takes the rewrite's results;
  - a "Where the code lives" line names `science::balance`,
    `surface::condense` and `system::planet::climate`.

  Keep the legacy history where it explains a choice.

- [ ] **Step 2:** Run `mise run docs:check` and
  `npx --no-install markdownlint-cli2` on the linted pages touched.
  `mise run docs:figures` is not needed: no figure draws a law this
  plan changes (`tools/figures` draws stars and giants). Confirm that
  `docs:check` reports no stale figure.

- [ ] **Step 3: Commit**
  `docs(science): describe the climate solver of the rewrite`.

### Task 13: CI and timings (sonnet)

- [ ] **Step 1: Run** `mise run ci`. Expected: green, with tier 1 under
  120 s and tier 2 under 480 s.
- [ ] **Step 2:** Report to the main session:
  - the tier-1 wall time before the branch (base `5bb25e66`) and now;
  - the ten slowest new tests;
  - the time of `mise run test:heavy -- climate`.
- [ ] **Step 3:** Commit nothing if nothing changed.

### Before the PR (main session)

- [ ] Push Wiki study 25 (the clone at
  `/private/tmp/claude-501/-Users-cyborg-Developer-map-generator/08fb1acd-12c9-400e-8804-e0196a1faf3c/scratchpad/wiki`,
  commits `f2babb9`, `c9c2d02` and `68f724f`). The hook blocks the push,
  so give the user the command.
- [ ] Run CLAUDE.md's procedure on `5bb25e66..HEAD`:
  `branch-reviewer`, `test-reviewer` and `science-coherence-checker`,
  then `issue-auditor`.
- [ ] Write the PR description:
  - `Closes #646`, `Closes #670`, `Closes #671` and `Closes #673`;
  - `Refs #593`, `#595`, `#602`, `#659`, `#677` and `#678`;
  - `issue-auditor` decides between `Closes` and `Refs` for #672 and
    #675: the laws land, but nothing sets `locked` before #677;
  - a "Calibrations" heading with the ten numerical ones and the
    retargeted physical ones (`FROST_ALBEDO`, `SNOWBALL_WARM_BAR`,
    `STEAM_ALBEDO_BAR`, `THERMOSTAT_K`, `CH4_OPACITY`);
  - the measured outer-edge gaps;
  - the tier-1 and heavy timings;
  - the new contract variant `ClimateState::Frozen`;
  - the #593 lever-2 gap measured on the Titan-like body.
- [ ] Comment on #671 that ice VII is not ported (Decisions).

  Generated systems are unchanged, so there is no oracle run.
