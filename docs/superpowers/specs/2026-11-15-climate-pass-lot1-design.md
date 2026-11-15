# Climate pass, lot 1: moist radiation limit, boiling and steam worlds

Issues: #441, #442, #492. Roadmap: #423 (track A), #427 (climate clean-up
pass, group a).

## Intent

The user's goal for the whole climate pass is the most physical realism
possible. Lot 1 fixes the hot end of the climate chain, which is today the
most visibly wrong part of the model:

- the grey climate has no moist radiation limit (#441), so the runaway
  threshold and the temperature of thin steam atmospheres are artefacts;
- a boil is kept only when its dry re-run is past 647 K (#442), which leaves
  a wet planet at 1488 K;
- a planet buried under hundreds of thousands of bar of supercritical water
  is drawn as a molten rock, with its pressure clamped to 1000 bar and a
  radius that ignores the envelope (#492).

The pass is split into four lots, in an order the user approved on
2026-11-15: 1) runaway and steam worlds (this document), 2) cold
atmospheres and atmosphere draws (#443, #447, #453, #454, #464),
3) subsurface oceans of icy moons (#463), 4) names of classes and climate
states (#498, #499, ultra-hot half of #501). Lot 4 comes last so that its
thresholds read the corrected climates.

## Goals

1. A radiative-convective grey climate whose outgoing longwave radiation
   (OLR) plateaus at the Simpson-Nakajima limit on the moist branch,
   independent of the N2 inventory.
2. A runaway decided by physics, when absorbed stellar flux plus internal
   heat exceeds what the moist atmosphere can emit, not by a 647 K test.
3. An end state consistent with the water left after escape.
4. A steam-world class: deep supercritical water hides the surface, inflates
   the radius, and the body renders as an envelope.

## Non-goals

- Non-grey (band or line-by-line) radiation.
- Cold-end climate problems (methane cold trap #443 and the rest of lot 2).
- Hycean worlds (#500). They will share the water-share threshold set here.
- Names of climate states (lot 4).

## Anti-goals

- The Solar System losing its climates: Earth, Venus, Mars and Titan must
  stay inside their current test ranges after recalibration.
- A steam world that still shows a rocky map, or a card that shows a
  pressure other than the model's.
- A legacy path or a format gate that keeps the old grey core selectable
  (project policy: worlds may change if they stay coherent).

## Design

### 1. Radiative-convective grey core (#441)

Replace the closed form `T_s^4 = T_eq^4 (1 + 3/4 tau) + q / sigma`
(`crates/mapgen/src/planet/climate.rs`, `surface_temperature_k`) with a
column model:

- From the surface upward the temperature follows the dry adiabat of the
  air until water saturates, then the moist (saturated) adiabat. Past the
  critical point the column is supercritical and follows the dry adiabat of
  the mixture.
- The grey optical depth is integrated down from the top of the column with
  the existing opacity laws (CO2, H2O, CH4, pressure broadening), expressed
  per unit pressure instead of per column.
- The emission temperature is read where the optical depth from the top is
  2/3. A radiative stratosphere (skin temperature) caps the column above it.
- `OLR(T_s) = sigma T_emit^4`. The surface temperature solves
  `OLR(T_s) = absorbed + internal heat` by bisection.

On a moist column the emission level sits on the saturation curve, so the
OLR plateaus. The opacity constants are recalibrated so that the plateau is
282 W/m2 (Goldblatt et al. 2013, doi:10.1038/ngeo1892) and does not move
with N2, and so that the Solar System tests hold (Earth 282-294 K regulated
by the thermostat with less than 0.01 bar CO2, Venus 687-787 K, Mars
198-222 K, Titan 86-102 K). Thin steam runaways should then follow the
plateau to much higher temperatures (Boukrouche et al. 2021,
doi:10.3847/1538-4357/ac1345: to about 1050 K at 10 bar). The grey model is
not expected to match that point exactly; the builder reports how close it
gets.

Acceptance (my call): the four Solar System ranges and Earth's regulation
by the thermostat (under 0.01 bar CO2) are hard limits; the plateau must
land at 282 W/m2 within 5 % and vary under 2 % between 1x and 3x Earth's
N2. If one set of grey constants cannot meet both, stop and report to the
main session rather than widening the ranges.

The ad hoc devices that stood in for the limit (`STEAM_MAX_TAU`,
`STEAM_MAX_BAR`, `STEAM_BREAKDOWN_K` and their tests) go if the column
model makes them unnecessary; their tests are replaced by tests of the
plateau and of the breakdown.

### 2. Runaway and boiling (#442)

- A planet with surface water runs away when no wet equilibrium exists:
  absorbed flux plus internal heat exceeds the moist OLR plateau. This
  replaces the `boils` / `past_critical` test (`climate.rs`, `mod.rs` boil
  block in `chain`).
- The steam pass keeps its energy-limited escape (`regime::steam`).
- Order, in one pass with no loop (my call): (1) the wet pass with the
  initial water decides the runaway; (2) a runaway runs the steam pass with
  its energy-limited escape; (3) the chain re-runs with the water left
  after escape; (4) if a wet equilibrium exists for that leftover water
  (its absorbed flux is under the plateau), the leftover water condenses
  and the wet state with that water is used; otherwise the steam state
  stands. There is no temperature condition.
- Census planet 458 c (#442) ends dry: no liquid, no water kept, runaway
  kept. Its temperature is not pinned (about 618 K today under 93 bar of
  CO2); the test checks the dry state, not a number.
  The 0.2 M_earth case of the test `a_boil_whose_steam_cools_is_not_kept`
  (`classify.rs`) is re-examined: with a real plateau it should no longer
  boil at all; the test is rewritten around the new rule.

### 3. Steam worlds (#492)

- **Threshold.** A steam world is a runaway end state (step 2 above) whose
  surface is hidden by a deep supercritical column: an H2O column over ten
  times the critical pressure (221 bar) on a surface past 647 K. A planet
  in wet equilibrium is never a steam world (my call; hycean worlds, #500,
  are hydrogen over liquid water and reuse only the water-share part).
  In `classify`, `steam-world` is tested before `runaway-greenhouse`
  (a steam world has no H/He envelope, so the mini-Neptune rule does not
  reach it). The builder confirms
  or adjusts the factor from the census and states it in the code; the same
  water threshold is reused later by #500.
- **Radius.** The steam envelope is added to the radius with the
  mass-radius relations of Turbet et al. 2020 (A&A 638, A41) for
  water-rich planets in runaway, so that radius, density and gravity agree.
  `interior` runs before the climate in `chain`, so the steam interior is
  recomputed once the steam state is known: radius, density and gravity,
  in a single pass (the climate is not re-run with the new gravity; my
  call). The envelope thickness comes from the Turbet et al. relations
  with the kept water as the water fraction, the refractory mass as the
  core, and the planet's irradiation; where their grid does not cover the
  planet, clamp to its edges and say so in the code.
- **Class.** A new `PlanetClass::SteamWorld` (`steam-world`) in
  `classify.rs`. `runaway-greenhouse` stays for thin steam over a visible
  rock (Venus-like cases). The class is added to the generated TypeScript
  type, `web/src/lib/planetClass.ts` (as an envelope class, with
  `mini-neptune`), the class tests and the EN and FR translations.
- **Web.** The lot touches the web app: generated `PlanetClass` type,
  `planetClass.ts` (envelope class), `systemStar.ts` kind, card radius,
  i18n EN and FR. `viewer-visual-check` screenshots a steam world (whole
  view and close-up) as part of acceptance.
- **Look (the user's choice).** A fully clouded world in shades of grey,
  as if it were only clouds: no surface map, no rocky relief. It renders
  through the envelope path of `interim::params_for` (the `None if giant`
  branch or a sibling) with a new `looks/steam-world.toml`: low-contrast
  grey bands and cloud texture, no hue.
- **Pressure.** The stored model keeps `gases.total()` at the base of the
  supercritical column, and the card shows it (hundreds of thousands of
  bar). The 1000 bar clamp applies only to what a rocky world's
  renderer needs, never to the stored model.

### 4. Formats and tests

- `SYSTEM_FORMAT_VERSION` goes from 5 to 6 (radii and classes change).
  Bump the world `FORMAT_VERSION` and `MIN_FORMAT_VERSION` (and the web
  mirror): steam worlds change render path and every temperature moves,
  so world bytes will change. Before the bump, the main session agrees on
  the number with the peer sessions (mapgen2's `fix/dry-world-relief`
  takes world format 21); the plan carries that as an explicit step.
- Tests: the OLR plateau at 282 W/m2 at 1, 2 and 3 times Earth's N2;
  the Solar System climates; the runaway threshold; a 458 c-like planet
  ending dry; the #492 case (`mgs-81447-3145862552` b: 4.78 M_earth, 12 %
  water, M9V at 0.018 AU) classed `steam-world` with an inflated radius;
  a thin-steam case still `runaway-greenhouse`.
- Census: run `planet_census` (500 systems) before and after; report
  the class mix, how many steam worlds appear, the temperatures of thin
  steam runaways, and the radius histogram. The world-coherence check
  compares a temperate Earth-like world before and after.
- Performance: the column model is evaluated many times per planet
  (thermostat bisection, wet fixed point). Measure the cost of a whole
  system draw before and after with `generation-profiler`. Budget (my
  call): a whole system draw may slow by at most 20 %; past that,
  tabulate OLR against surface temperature and composition, or coarsen
  the grid.

## Left to the builder

- The pressure grid of the column, the skin-temperature rule, and the
  numerical scheme of the solve.
- The exact form of the opacity per unit pressure and the recalibrated
  constants.
- The deep-steam factor (starting at 10x critical pressure) and the way
  the Turbet et al. relations are fitted in.
- Whether the steam look reuses the ice-giant renderer with a grey palette
  or needs its own parameters.

## Documentation

`README.md` sections on the planet model's climate and classes, the
generated class list, and the census example's output description, in the
same change.
