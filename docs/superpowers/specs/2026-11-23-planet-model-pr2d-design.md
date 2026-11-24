# Planet model PR 2d: air, surface, climate, class

Decided with the user on 2026-11-23. Slice 1, #602. Follows the split
[planet model PR 2 split](2026-11-21-planet-model-pr2-split-design.md)
(2a, 2b-1, 2b-2 and 2c merged as #664, #663 merged as #666, main
b2f6c9da). Deferred steps: #659. Legacy code: `legacy-final:crates/mapgen/src/planet/`.

## Intent

2d finishes the planet model of slice 1. Today the chain stops at the
bare air (`system::planet::bare`): interior, escape, water kept and steam
loss, regime, core, outgassing, cosmic shoreline, SO₂ wash-out. 2d adds
what turns that air into a planet with a surface: the air's own
chemistry (photolysis, cold trap, photochemistry), the surface cover of
the albedo and the surface liquids, the global climate, life and
oxygenation, and the class. It then makes `class_of` and `model`
public and wires them into the system draw, so that the generated
systems carry real planets instead of the mass-only stand-in.

It is the first pull request of the planet model that changes what is
generated.

## Goals

- Port, with their sources and tests, from `legacy-final`:
  - the air steps of #659: methane photolysis, cold trap,
    photochemistry, the outgassing redone without CO₂ for a reducing
    air;
  - the surface cover of the albedo (ocean, land, regolith, ice, frost,
    SO₂ frost, ice line, ocean fraction) and the surface liquids (water,
    hydrocarbons, nitrogen, lava);
  - the global climate solver: wet balance, the albedo and emissivity
    fixed point, condensation and collapse, maximum greenhouse,
    carbonate thermostat, snowball branches, with the #593 levers;
  - the `boils` loop and the steam envelope of a water world past the
    water laws;
  - life and oxygenation (`life.rs`), which `PlanetState` needs;
  - the class (`classify.rs`).
- `planet::class_of(&Trial, &Trace) -> Result<TrialOutcome, SystemError>`
  and `planet::model(&Placement, &mut Trace) -> Result<PlanetState,
  SystemError>` public, wired into `draw_with` in place of
  `stand_in::classify` (CLI, regression crate, determinism tests).
- A tier-2 regression net on the shared reference systems, which reach
  the planet model for the first time.
- Every number carries `Source:`, `Calibration:` or `Definition:`; the
  gaps found by the audit below are studied before their code.

## Non-goals

- The icy surface-age albedo and the vent bisection of ice crusts
  (`icy_unit_albedo`, `vent_strength`, `crust_albedo`, `VENT_STEPS`):
  left out, every ice crust takes the plain ice cover. Recorded on #560.
- Non-thermal loss and the magnetospheric shield of moons: #658 (T2).
- Terrain and archetype targets of `PlanetState`: PR 3. 2d fills them
  as PR 3's design says it will receive them (see "Left to the
  builder").
- The follow-ups kept outside 2d (table below), among them #594
  (Venus), #643 (CO₂ forcing), #655 (stellar spectra).
- Past and future keyframes: the first release has one keyframe, the
  present (architecture design).

## Anti-goals

- No number without its tag, and no value fitted on the code's own
  output to make a test pass.
- No second code path for a value: a law the rewrite already has
  (volcanism, saturation curves, the Lyman α fluence, the water phase
  diagram, `OCEAN_MASS`) is reused, never re-ported beside it.
- The map generator never reads anything 2d computes except through
  `PlanetState`.
- No blended stellar temperature: spectrum-dependent terms are
  flux-weighted sums over the sources (#592).

## Constraints

- Signatures fixed: `class_of(&Trial, &Trace) -> Result<TrialOutcome,
  SystemError>` and `model(&Placement, &mut Trace) ->
  Result<PlanetState, SystemError>`. The doc comment of
  `crates/system/src/planet/mod.rs` still says `class_of` returns a
  `PlanetClass`: 2d-4 corrects it.
- mapgen3 does not touch `crates/system/src/planet/**`. This work does
  not touch the draw modules (`crates/system/src/draw/**`),
  `placement.rs` or the placement half of `archetype.rs`. Wiring the
  classifier means changing its callers only (`cli/src/report.rs`,
  `regression/src/lib.rs`, `system/tests/determinism.rs`), plus the
  `stand_in` module, which stays for the draw's own tests.
- The draw calls the classifier once per try (`draw/place.rs:355`) and
  twice more for the mass-radius slope (`place.rs:476`). Each call runs
  the whole chain, climate included, on a scratch trace. Generation
  time is therefore the main risk of 2d-4 (see the #593 levers).
- 2d-4 changes what is generated: `ENGINE_VERSION` goes from 3 to 4
  (`crates/system/src/lib.rs:38`), a `develop` cycle with the oracle
  (`world-coherence-checker`) and `mise run test:heavy`, the outputs in
  the PR description. mapgen3 is told when this design is written and
  before 2d-4 merges.

## Approaches

1. **Along the chain, four pull requests (chosen):** air, surface,
   climate, class and wiring. Each leaves `main` buildable and tested;
   the model stays crate-private until the last, which alone changes
   the generated systems. Each law lands with its first caller, and a
   study blocks only the pull request that needs it.
2. Science first, then the system, then the wiring (three pull
   requests): rejected, the science pull request lands laws without a
   caller and the system one carries 2 400 lines of solver plus the
   wiring.
3. Everything private, then the wiring (two pull requests): rejected,
   too large to review, as the single pull request was in the split.

For the legacy steps outside 2d's announced scope, the user chose to
port what the climate needs for ordinary planets (frost tables,
volcanism, the snowball draw) and life with oxygenation, and to leave
the ice-crust law as a declared departure (#560).

## The pull requests

| PR | Content | Changes the generated systems |
| --- | --- | --- |
| 2d-1 Air | NH₃ frost curve; methane photolysis; cold trap; photochemistry; the reducing-air rerun | No |
| 2d-2 Surface | Surface cover of the albedo; surface liquids; N₂ and SO₂ frost curves and frost cover; volcanism for the snowball chance | No |
| 2d-3a Climate solver | The solver with the #593 levers and `condense`; the outer-edge fit; snowball branches and their draw; the reducing-air fallback | No |
| 2d-3b Boiling and steam | `boils` loop; `Water.boiled` derived; steam envelope (Turbet 2020) | No |
| 2d-4 Class and wiring | Life and oxygenation; class; public `class_of` and `model`; wiring; `ENGINE_VERSION` 4; tier-2 net | Yes |

2d-3 was split into 2d-3a and 2d-3b on 2026-11-24 (see 2d-3 below).
Each opens with its study (below), then ports test first. The design
document travels with 2d-1; each later pull request writes its own plan
from it.

### 2d-1: the air

From `atmosphere.rs` and `mod.rs` `bare_air` (l.~915-1075). The steps
run inside the bare air, after the shoreline and before the climate,
in legacy's order:

1. `methane_photolysis` (l.1115): methane outgassed steadily over the
   planet's age, destroyed at a rate set by the Lyman α light, gathers
   only once the supply outruns the destruction. Legacy's law is ported,
   extended to several stars (decided with the user on 2026-11-23): the
   destruction at a time `t` is the sum over the sources of each one's
   flux times its own Lyman α ratio at `t` (as built: `History::ratio_at`,
   `History::integrated_between`, the law the timeline's fluence comes from); the
   onset where supply overtakes destruction is found on that sum by
   bisection, for one source as for several. The history is the
   rewrite's (a share of the ZAMS light times its `scale`, #622), so the
   yield fitted on legacy's ratio is fitted again. A test checks that the
   destruction integrated from birth to the age matches the yield times
   the fluence the timeline carries
   (`StellarSources::band_total(|s| &s.lyman_alpha)`): one law, not a
   second path. Taking only the total fluence (methane outgassed minus
   the whole destruction) was rejected: it loses the methane that
   gathers once the young star's Lyman α has faded.
2. `reducing` and `without_co2_if_reducing` (l.1170, l.1217): a reducing
   air outgasses no CO₂. Legacy runs the whole `bare_air` twice; in the
   rewrite the shoreline's share does not read the air's composition
   and the photolysis reads only methane, so the port decides
   `reducing` from the methane first and outgasses CO₂ at 0 when it
   holds, in one pass, with the same values (but for SO₂ on a dry body,
   whose wash-out reads the lighter column). Legacy also undoes the hold-back
   when the climate does not leave a Titan-like air (`bare`, `mod.rs`
   l.~873, `titan_like_air`): that fallback reads the climate and comes
   with 2d-3.
3. `cold_trap` (l.1158) caps CO₂ and NH₃ at their frost curves at the
   equilibrium temperature.
4. `photochemistry` (l.1252): NH₃ to N₂, CH₄ kept only below
   `METHANE_SURVIVES_K`.

Their cause nodes join the per-gas trees 2c records: each step records
the gas it changes with the previous node of that gas as its cause
(`outgassed` → `shoreline-kept` → `SO₂-kept` → `photolysed` →
`trapped` → `photochemistry`), so a gas's tree reads in chain order.
The frozen CO₂ (`kept.co2 - trapped.co2`; the photolysis leaves CO₂ as kept)
is recorded as its own
node: the carbon cycle of 2d-3 reads it.

The NH₃ frost curve joins `science::volatiles` beside the CO₂ and CH₄
curves already there; the cold trap reads the CO₂ curve of
`volatiles`. The legacy `CO2_FROST` and `CH4_FROST` are not ported:
the rewrite's curves replace them. `condense` works at the surface
temperature inside the climate's fixed point and reads the liquids
(`frozen_share`, `sea_freezes_k`): it comes with 2d-3, and the N₂ and
SO₂ frost curves with the frost cover of 2d-2.

Tests: the legacy tests of these functions, and
`earth_has_no_abiotic_methane_or_ammonia` (#659). The 2c review asked
to recheck Earth's N₂ against its target once the ammonia photochemistry
exists (`NITROGEN_RECYCLED_UNDER_PLATES`, comment on #659): done here: the
target is 0.746 bar by weight (0.78 is the volume
share), and the constant was refitted from 0.8 to 0.8145 (Earth 0.7458
bar), with its calibration section.

### 2d-2: the surface

From `albedo.rs` (`Cover`, `cover`, `ocean_fraction`, `ice_line_share`,
`frost_cover`) and the liquids of `climate.rs` (`liquid`,
`sea_freezes_k`, `methane_condenses`, `hydrocarbon_freezes_k`,
`ethane_boils_k`, `nitrogen_vapour_bar`, `frost`, `frozen_share`,
`melting`). The laws go to `science` (a surface cover module beside
`radiation`), as functions of a temperature, an air, a water inventory
and a radius. 2d-2 tests them at observed temperatures (Earth, Mars,
Titan, Triton, Io), without the solver. The atmosphere's part of the
albedo is already in `science::radiation`; the cover gives it its
`Underneath.ground`.

Volcanism: `snowball_chance` reads a volcanic intensity. Legacy's
`interim::volcanism` (a square root of the heat flux, uncited) is not
ported: the rewrite already has a documented law in
`science::relief::volcanoes` (Moore & Webb 2013). The snowball chance
reads that one, and `SNOWBALL_WARM_BAR`, fitted on the legacy law, is
checked again against it in 2d-3's study.

Issue #656 (the methane haze refitted on Titan's Bond albedo 0.265) waited
on Titan's ground albedo, which the surface cover gives: 2d-2 takes it
if its study finds a sourced ground albedo for Titan.

### 2d-3: the climate

Split in two with the user on 2026-11-24, after the 2d-3 recon found
three laws the rewrite lacks (the Kopparapu outer-edge fit, `condense`,
the Turbet 2020 steam envelope):

- **2d-3a, the solver:** wet balance, the albedo and emissivity fixed
  point, `condense`, collapse, thermostat, maximum greenhouse with the
  outer-edge fit, snowball branches and their draw, the reducing-air
  fallback, the #646 refit and the #673 Triton refit. Study 25.
- **2d-3b, boiling and steam:** the `boils` loop (`Water.boiled`
  derived, a forced-steam entry to `water`), `hides_surface` and the
  steam envelope with its radius. Its own study (Kasting 1988, Turbet
  2020).

Decisions taken with the user on 2026-11-24 for 2d-3a:

- **#670:** the two heat-pipe fluxes mean different things and are
  renamed, the law unchanged: `HEAT_PIPE_ONSET_FLUX` (1 W/m², the
  regime's onset, a calibration) and `FULL_VOLCANISM_FLUX` (2.5 W/m²,
  Io's flux, where volcanism is total). A heat pipe between the two
  reads an intensity below 1.
- **#671:** the melting curves of ices III and V (IAPWS R14-08, eqs. 2
  and 3) are ported, and the surface window reads the melting curve of
  the ice stable at the surface pressure.
- **#672 and #675:** a tidally locked rotator takes the substellar
  distribution, both for the sunlit share of the ice and for the
  lava-world threshold (Kite et al. 2016's substellar temperature); a
  fast rotator keeps the zonal form and the mean. The rotation criterion
  is sourced in study 25.
- **The spin as an input:** `class_of(&Trial)` has no spin (it is drawn
  after the class, from the designation's seed), and tidal locking comes
  with #604. The solver therefore takes the obliquity and a "locked"
  flag as parameters; 2d-4 decides where `class_of` gets them, so that
  `class_of` and `model` still give the same answer.

From `climate.rs` l.1-2423: `wet_balance`, `first_balance` (the #600
secant scan), `settle` and `settle_steps` (the #600 secant step of the
albedo fixed point), `under_albedo` (condensation), `collapsed_climate`,
`wet_climate`, `thermostat`, `maximum_greenhouse` (once per body: the
`Body` with its `greenhouse` and `plateau` cells, the #593 lever),
`branches`, `snowball_chance`, `boils`, `hides_surface`,
`steam_albedo`. The solver lives in `system::planet` (it is how a body
settles, not a law); the laws it calls stay in `science`.

- **Several stars.** The absorbed light is the sum over the sources of
  each source's flux times its own shortwave budget at its effective
  temperature (#592). With one source this is legacy's computation.
  The tier-1 test of #592 (two identical half-flux sources give one
  source) covers the climate.
- **The snowball draw.** A new draw of the planet's seed hierarchy
  (`planet/draws.rs`), recorded as a drawn leaf. Legacy drew it after
  the life draw from the body's model stream; the order of the new
  draws is the builder's, inside the seed hierarchy.
- **The `boils` loop** (`mod.rs` l.524-575): one pass, as in legacy.
  `Water.boiled` becomes a derived node. The recondensed water is kept
  only if its new air does not boil.
- **`after_steam` with an envelope:** a planet that keeps a surface has
  no envelope when the loop runs, so the `debug_assert` of 2c stays;
  2d-3 does not port legacy's rescale.
- **The steam envelope** (`hides_surface`, `interior::steam_interior`):
  a water world past the water laws gets its steam envelope and its
  radius (#492), and the steam loss and shoreline read that body
  instead of the condensed radius.
- **Mars's test air:** the climate bound on Mars (210 ± 5 K) runs on a
  surface pressure from a cited source (for example the Viking or MSL
  REMS annual mean), set in the test as an imposed input. Legacy's
  `MARS_CO2_FACTOR` (from memory) is not ported.
- **Venus:** the pinned 577 K test is ported as is (#594).
- **#646:** `CH4_OPACITY` is refitted on Titan with the new methane
  curve here, since 2d-3 is the column's first caller.

Tests: the 56 legacy climate tests, the slow ones under
`#[ignore = "heavy: …"]` (`the_outer_edge_follows_the_star`, the
Kopparapu edges, `census_182_d`, the Titan sweeps). Expected values
come from sources (the Solar System's measured temperatures, Kopparapu
2013's edges); a pinned value stays only as a tier-2 regression net.

### 2d-4: life, class and wiring

- **Life** (`life.rs`): `life()` and `oxygenate()`, a `life_draw` in the
  seed hierarchy, after the climate. Oxygenation changes the air, and
  the climate is solved again on the oxygenated air (one more settle).
  Legacy's declared simplification (the climate solved on the air
  before life) is dropped, decided on 2026-11-24 in 2d-3a: without its
  O₂ the reference Earth's air is 0.75 bar, 283.8 K at 280 ppm of CO₂
  against 286 to 290 K (Myhre et al. 1998), and its thermostat needs
  about 2 000 ppm. With N₂ 0.78 and O₂ 0.21 bar the solver gives
  286.9 K. 2d-3a tests that check on Earth's measured air; 2d-4 makes
  the reference Earth reach it and updates `docs/science` (section
  Life of `planet-atmospheres.md`).
- **Class** (`classify.rs`): the 14 classes of `timeline::PlanetClass`
  in legacy's order (giants, mini-Neptune, the surface classes, ice
  world, volcanic, super-Earth, rocky). `AIRLESS_BAR` is not a class
  threshold; it stays a test definition (#659).
- **`class_of`** runs the chain on a scratch clone of the trace and
  returns the class, the radius and the mean density as plain values
  (split design). **`model`** runs the same chain on the real trace and
  builds `PlanetState`. One private `chain` serves both.
- **Wiring:** `cli`, `regression` and the determinism tests pass
  `planet::class_of` to `draw_with`. The `stand_in` module stays for the
  draw's unit tests.
- **Tier-2 net:** `REFERENCE_SEEDS` is filled, and the shared reference
  systems get assertions on their planets' classes, climates and
  liquids, pinned as a regression net.
- **Timing:** `generation-profiler` measures the draw of the reference
  systems before and after, on the same inputs. The result goes in the
  PR description, with the share of the column in it.

## Science to study first

The audit of the 2d scope (2026-11-23) classed each legacy number.
Missing or thin science stops the work: each item below is studied
(sources read, the page in `docs/science/` extended, a Wiki study for
the larger ones) before the code of its pull request. A value with no
data becomes a `Calibration:` with its "searched and not found"
section.

| PR | Gap | What the study answers |
| --- | --- | --- |
| 2d-1 | `CH4_PER_LYMAN_ALPHA_PHOTON` 13.9 | Yung et al. 1984 read in full: Titan's methane loss over a constant or a decaying Lyman α, whether a yield above one is physical |
| 2d-1 | `METHANE_SURVIVES_K` 150 | The temperature at which water reaches the upper air and OH oxidises methane |
| 2d-1 | `NH3_FROST` curve | NH₃ triple point and sublimation enthalpy from NIST or Fray & Schmitt 2009 |
| 2d-2 | N₂ frost fits, `NITROGEN_ALPHA_BETA_K` | Fray & Schmitt 2009 and Scott 1976 read, not through Lisse 2021 |
| 2d-2 | `SEA_METHANE_SHARE` 0.5 | Glein & Shock 2013, Tan et al. 2013, Niemann et al. 2005 |
| 2d-2 | Surface albedos (ocean 0.06, soil 0.25, regolith 0.11, ice 0.6, frost 0.75) | Read the cited sources or measured values; otherwise calibrations with their section |
| 2d-2 | `SHIELD_BAR` 1e-4 | Whether any source sets the pressure that shields regolith |
| 2d-2 | Ice line (`ICE_LINE_K`, `ICE_LINE_CONTRAST_K`, its form) | Budyko 1969, North 1975 |
| 2d-2 | `ICE_SHEET_KM` 1.5, `ocean_fraction` form and `EARTH_OCEAN_FRACTION` | Fretwell 2013, Plaut 2007; the form labelled as a calibration |
| 2d-2 | `RELATIVE_HUMIDITY` 0.77 | Manabe & Wetherald 1967 itself, or an open source giving the surface value |
| 2d-2 | `LAVA_K` 1400, ethane constants | Kite et al. 2016, Hamano et al. 2013; NIST for ethane |
| 2d-3 | `SNOWBALL_WARM_BAR` 0.8 | Hoffman 2017; refit against the rewrite's volcanism law |
| 2d-3 | `RUNAWAY_ALBEDO` bounds, `STEAM_ALBEDO_BAR`, `STEAM_WORLD_MIN_BAR` | Their reason, or a calibration section each |
| 2d-3 | `boils` | Kasting 1988 read |
| 2d-3 | Solver constants (tolerances, step counts, `SCAN_RATIO`, `RUNAWAY_PROBE_K`, `PLATEAU_SURFACES_K`, `THERMOSTAT_TOLERANCE`, `GREENHOUSE_*`, `THAW_STEPS`, `ALBEDO_*`, `COLLAPSE_*`, `CONDENSE_ITERATIONS`) | Labelled by the rule of #648 (a guard is a `Definition:`, a discretisation a `Calibration:` held by a convergence test); a convergence test each, tier 3 if slow |
| 2d-4 | `LIFE_MIN_GYR` 1 Gyr | The oldest evidence of life after a planet's formation |
| 2d-4 | `OXYGEN_AFTER_GYR` 2.2, `O2_PER_N2` | Lyons, Reinhard & Planavsky 2014 read in full; the O₂ ratio as an Earth calibration |
| 2d-4 | `GAS_GIANT_MIN_ENVELOPE`, `MINI_NEPTUNE_MIN_ENVELOPE`, `ICE_WORLD_MIN_WATER`, `DENSE_AIR_BAR`, `OCEAN_WORLD_OCEANS` | A source each, or a calibration section; Cowan & Abbot 2014 read |

`LIFE_CHANCE` (1/3) is a game choice made with the user on 2026-11-12:
it is a `Definition:` that says so, not a result of science.

Values already in the rewrite and reused as they are: `volatiles` (CO₂
and CH₄ curves), `water` (IAPWS), `xuv::LYMAN_ALPHA`, `outgassing`
(`so2_kept`, wash-out), `radiation` (clouds, veils, methane, column,
shortwave), `habitable::OCEAN_MASS` (`OCEAN_LAYER_KM` derives from it),
`constants` (solar irradiance, Stefan-Boltzmann, gas constant, standard
gravity). Where legacy and the rewrite differ (the methane triple point
90.67 against 90.6941 K, `WET_SO2_SHARE`, `CLOUD_ALBEDO`), the
rewrite's value holds.

## Follow-ups

| Issue | In 2d | Why |
| --- | --- | --- |
| #593 | 2d-3 (levers), closed by 2d-4 once measured | Climate-solver code |
| #646 | 2d-3 | First caller of the column |
| #656 | 2d-2, if a sourced ground albedo for Titan exists | Needs the surface cover |
| #659 | Steps in 2d-1 and 2d-3, closed by 2d-4 | The deferred steps |
| #654 | No | Changes the humidity profile and the cloud fit: its own study and pull request; 2d uses 0.77, the path the cloud fit was made on |
| #655 | No | A stellar spectral library: its own study and data |
| #594, #643 | No | Missing opacities and a forcing bias in `science::radiation`; 2d ports the pinned Venus test as is |
| #647 | No | Declared simplification; to check against #595 later |
| #648 | Its labelling rule only, for 2d's solver constants | The column-top convergence test stays with the column |
| #649, #650 | No | Opacity data provenance |
| #658, #665 | No | T2 and after T4 |

## Rules for the build

Settled after the builder check of this design (design author's calls,
open to the user's correction):

- **`class_of` runs the whole chain.** The surface classes (runaway,
  hydrocarbon, ocean, steam world) read the climate, and the radius it
  returns is the one after the `boils` loop and the steam envelope. No
  cheaper path skips the climate: one chain serves `class_of` and
  `model`.
- **Same planet, same answer.** Every draw (snowball, life) derives from
  the planet's seed, never from the trace: `class_of` on its scratch
  trace and `model` on the real one draw the same values and give the
  same class, radius and density. A tier-1 test checks it.
- **Several stars.** The shortwave budget is computed per source at its
  own effective temperature, with the ground and clouds of the current
  surface, and the absorbed light is the flux-weighted sum; the
  longwave and the emissivity do not depend on the star. The geometry
  (S-type or P-type) stays in the timeline's orbit-mean flux per
  source. With one source the result is legacy's.
- **Which wins when a rewrite value differs from legacy's.** The
  source wins over the legacy test. A legacy bound that is itself an
  observation (Earth 288 K, Mars, Titan, Kopparapu's edges) stays. If
  it fails after a rewrite value replaced a legacy one, the builder
  stops and reports the gap to the user; it does not loosen the bound
  or refit a constant on the code's output. A documented calibration
  whose target is an observation (`SNOWBALL_WARM_BAR` on Earth's
  snowball history) is refitted against that observation.
- **A study finds no source.** The value becomes a calibration with its
  "searched and not found" section and a line in the PR description.
  More than two new calibrations in one pull request, or one fitted on
  an observation the model cannot reproduce, is reported to the user
  before the code goes on.
- **Dead code between pull requests.** Laws in `science` are public, so
  they have no dead-code issue. The solver and the chain steps in
  `system::planet` stay crate-private under the `expect(dead_code, …)`
  attributes 2c uses, and each pull request extends `bare` with its
  steps, so they run in the chain's tests. 2d-4 removes the attributes.
- **Order of the pull requests.** Each merges into `main` before the
  next one opens its pull request. A builder may start the next branch
  from the previous one while it is in review, then rebase it on
  `main` once that one merges.
- **The N₂ retune in 2d-1** changes only the crate-private bare chain,
  so 2d-1 still leaves the generated systems unchanged.
- **Tier-2 net.** It asserts on the eight existing reference requests of
  `crates/regression` (`reference_requests()`), whose seeds fill
  `REFERENCE_SEEDS`; no new reference system is added. If the climate
  pushes tier 2 over its 8-minute budget, `test-reviewer` proposes and
  the user decides.
- **Generation time.** There is no time gate in 2d-4 itself: the
  generation budget is set after slice 1's profiling (#593). 2d-4
  measures the time before and after, and the user judges it with the
  oracle before the merge.

## Left to the builder

- The module layout inside `system::planet` and `science`, and the
  struct names, as long as the crate rules hold.
- The order of the new draws (snowball, life) inside the seed
  hierarchy.
- How `PlanetState`'s `terrain` and `targets` are filled until PR 3,
  following `crates/timeline/src/planet.rs` (an imposed leaf, as `boiled`
  was in 2c, is acceptable if the type requires a value).
- Which legacy tests are merged when two test the same law.
- Moving a test to tier 3 when a budget is at stake: proposed by
  `test-reviewer`, decided by the user.
