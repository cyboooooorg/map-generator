# Climate pass lot 3C-1: surfaces and thin airs

Branch `feat/climate-lot3c`, worktree `~/Developer/mg-lot3c`, from `main`
99690d2. Track A of #423, schedule in #427 (comment 6919717434), lot 3C
split in three on 2026-11-17: 3C-1 surfaces and thin airs (this spec),
3C-2 climate (#522 #523 #513 #517 #520), 3C-3 oceans (issue #535 and
issues #536, #537, #540).

## Intent

The user's criterion for the climate pass is maximum physical realism, and
for lot 3C in particular: an archetype must come out as the body it stands
for on every seed. A census of the 21 built-in archetypes (new example
`archetype_census`, seeds 1 to 40, each archetype asked alone in a whole
system) shows that the rocky planets, the giants and Venus, Earth and Mars
are faithful, but the icy and volcanic moons are not:

| Body | Model (40 seeds) | Real |
| --- | --- | --- |
| Titan surface age | 3.2-4.3 Ga, crater-saturated | 0.2-1 Ga (Neish & Lorenz 2012) |
| Triton surface age | 3.2-4.3 Ga | about 10-100 Ma (Schenk & Zahnle 2007) |
| Europa, Ganymede, Callisto air | 3e-6 to 1e-4 bar of CO2 and N2 | exospheres, about 1e-12 bar (Callisto CO2: Carlson 1999) |
| Io air | 5e-5 to 1.2e-3 bar of N2 and CO2, no SO2 | about 1e-9 bar of SO2 over SO2 frost (#525) |
| Callisto temperature | 84-93 K | about 115 K (Bond albedo about 0.2) |
| Ganymede temperature | 84-88 K | about 106 K (Bond albedo about 0.4) |
| Enceladus temperature | 85-89 K | about 60-75 K (Bond albedo 0.81, Howett et al. 2010) |
| Titan CH4 | 3.8-6.7 % | target 3-6 % (#530) |
| Triton | 35-37 K, N2 constant from memory | 38 K, 1.4e-5 bar (#529) |

The user saw a Titan rendered "like a volcanic planet" once; the census
does not reproduce it (40 of 40 `hydrocarbon-world`, base `frozen`), and
PR #551 (cryovolcanism for hot icy ocean moons) probably fixed it. If a seed
shows it again, it joins this lot.

## Goals

1. **Surface age from the process that renews the surface.** The user's
   principle, approved on 2026-11-17: the age of a surface is the time
   since its last renewal, never older than the body, which is never older
   than its star. The process that renews it sets it, not a fixed range per
   regime. Processes, each with a cited rule:
   - **Dense air with an active liquid cycle** (Titan; Earth is already
     young through plates): erosion, rain, sediments and dunes erase
     craters. Titan must come out at 0.2-1 Ga on every seed. Closes #491.
   - **Mobile volatile ice** (N2, CH4, CO frost that holds the body's air in
     vapour equilibrium): seasonal sublimation and ice flow bury and relax
     craters. Triton must come out under 0.1 Ga on every seed. A drawn body
     of the same kind (Pluto-like, captured Kuiper belt object) follows the
     same rule.
   - **Tidal heat and cryovolcanism** (Europa, Enceladus) and **volcanism
     and tectonics** (Io, Venus, Earth): already modelled
     (`ice_lid_age_share`, regime ranges); they stay, and must keep their
     current results.
   - **None of these** (Mercury, Mars, Callisto, Ganymede's dark
     terrain): the surface stays close to the system's age, as now.
   - **Terrain units, in 0-D.** A surface is one or more terrain units,
     each with its own age and share of the surface (user's choice,
     2026-11-17): an old, cratered, dark unit and one or more renewed,
     younger, brighter ones. The model computes the units; it does not
     place them on the map. Ganymede gets its bright grooved terrain (about
     two thirds of the surface, younger) from a past tidal-heating episode
     during its capture into resonance (Showman & Malhotra 1997,
     Icarus 127, 93), by a rule that applies to any drawn moon whose
     history gives it such an episode; its old dark terrain keeps the
     system's age. Callisto keeps a single old unit. Other bodies get the
     units their processes give (Enceladus: young south, older north;
     Mars: old southern highlands, younger northern plains, when the
     model's history supports it).
   - The map keeps one age, that of the main unit (largest share), until
     #424 places the units as regions with their own craters and albedo
     (#552).
   The crater count follows the age through the existing crater code
   (`craters.rs`); nothing changes in the renderer. #491's comment on
   Mercury: Mercury's 3.2-4.3 Ga is right now; keep it.

2. **Thin airs from supply and loss, not from a frost cap alone.** A gas
   makes an atmosphere only if the body can hold it against non-thermal
   loss (photolysis and ionisation, sputtering by the host's magnetospheric
   plasma for a moon) given what supplies it. Otherwise the body has an
   exosphere, stored as a pressure under the airless threshold
   (`AIRLESS_BAR` 1e-4 bar) at the order of magnitude the steady state
   gives. Targets: Europa, Ganymede and Callisto at 1e-13 to 1e-10 bar;
   Titan, Triton and the thick-air worlds unchanged within the existing
   test bounds. The rule is general (a cited loss law applied to every
   body), never a per-archetype value.

3. **Io's SO2.** Volcanic bodies get an SO2 inventory driven by their
   volcanic (tidal) supply, an SO2 frost with a sourced vapour curve, and a
   thin SO2 air at the sublimation equilibrium of the dayside: Io at about
   1e-9 bar of SO2 (order 1e-10 to 1e-8), SO2 frost instead of CO2 frost,
   no N2 or CO2 air above it. Closes #525.

4. **Icy albedo from surface age and composition.** Fresh ice is bright;
   an old ice surface darkens under dust, impact gardening, sublimation lag
   and radiation. The albedo of each icy terrain unit comes from its age
   (goal 1) and its rock share, with a cited law; the body's Bond albedo
   is the area-weighted mix of its units, which is what separates Ganymede
   from Callisto. Targets (Bond albedo,
   mean surface temperature):
   - Enceladus 0.75-0.85, 55-75 K;
   - Europa 0.55-0.70, 90-105 K;
   - Ganymede 0.35-0.50, 100-112 K;
   - Callisto 0.15-0.30, 108-122 K;
   - Triton keeps its N2-frost albedo (0.6 or more) and reaches goal 6;
   - Titan unchanged (its haze sets its albedo).
   Today Ganymede and Callisto look bright through the frost of their
   too-thick CO2 air (`FROST_ALBEDO` 0.75), and Enceladus probably takes
   the regolith albedo (0.11): the builder confirms the cause first.

5. **Liquid nitrogen** (#524): N2 condensed between its triple point
   (63.15 K) and its boiling point at the surface pressure is a surface
   liquid; below the triple point it is a frost. `SurfaceLiquid` gains the
   kind the builder judges right (a nitrogen liquid of its own, or N2 in
   the hydrocarbon mix with its share), and a planet whose air holds
   condensed N2 keeps its hydrocarbon class. Census seed 387 planet d must
   get its hydrocarbon class back.

6. **Triton** (#529): source the N2 sublimation curve (Fray & Schmitt 2009,
   doi:10.1016/j.pss.2009.09.011), add an N2-ice emissivity, and change
   both together so Triton reaches 38 K (37-39) and 1.4e-5 bar of N2
   (1e-5 to 2e-5) on every seed.

7. **Methane residuals** (#530): CH4 near-infrared absorption in the
   stratosphere (anti-greenhouse), the right saturation over CH4 ice (not
   the sea's 0.5), ethane in the methane-nitrogen liquidus (Richardson et
   al. 2019), then Titan at 3-6 % CH4 on every seed, and census seed 182
   planet d re-checked against the 150 K line.

8. **Card and explain.** The card explains what it shows:
   - the surface age gets a cause tree (system age, the renewing process
     and its rate), EN and FR, and lists the terrain units with their age
     and share;
   - an exosphere reads as such ("exosphere", with its order of
     magnitude), not as an air;
   - Io's frost and air read SO2;
   - the albedo node names the surface age when it sets the albedo.

## Non-goals

- The climate items of lot 3C-2 (#522, #523, #513, #517, #520) and the
  ocean items of 3C-3 (#535, #536, #537, #540).
- New biomes or surface layers (#424), and the names of classes (lot 4).
- Radiolytic O2 exospheres modelled species by species: the exosphere's
  pressure is an order of magnitude from the loss law, not a chemistry.
- Placing terrain units on the map (regional craters and albedo): #552,
  with #424.
- Seasonal maps (where the N2 frost sits at a given season).
- Changing the crater renderer or the look of a crater.

## Anti-goals

- A per-archetype surface age, albedo or pressure written in the TOML
  files. Archetypes keep pinning inputs (mass, age, inventories), and the
  physics gives the result. A fixed value that reproduces one body without
  a law is a failure even if the census passes.
- Breaking a body that is right today: Venus, Earth, Mars, Mercury, the
  giants, Europa's and Enceladus's young surfaces, Io's heat pipe, Titan's
  1.5 bar and its seas.
- A constant without a source or a stated calibration target
  (`docs/science/` records each).

## Constraints

- Formats: the surface age, pressure and albedo change stored worlds and
  systems. World `FORMAT_VERSION` 27 to 28 (no open branch holds 28; #541
  and #543 will take the next one), system `SYSTEM_FORMAT_VERSION` 9 to 10.
  Stored systems become outdated (regenerate), as in earlier lots.
- Generation changes are accepted when the new worlds stay coherent (no
  legacy algorithm, no format gate).
- Every existing archetype test keeps passing, or its bound changes to the
  real value with the source in the test.
- The rng draw order of `params_for` (`interim.rs:612-617`) stays fixed, so
  that bodies whose age does not change keep their other draws.
- Documentation in the same change: `docs/science/` (planet atmospheres,
  surfaces, interiors) and the README section on surfaces, in English.

## Design calls

Made with the user:

- Surface age is set by the renewing process and bounded by the system age
  (approved 2026-11-17).
- The four processes of goal 1, Titan by its air and liquid cycle, Triton
  by mobile volatile ice (approved 2026-11-17).
- Terrain units computed in 0-D now, with Ganymede's tidal episode;
  their placement on the map belongs to #424 (2026-11-17).
- Thin airs from supply and loss, Io's SO2 from its volcanoes, icy albedo
  from surface age (played back 2026-11-17, no objection; split approved).

Main-session calls:

- The surface age stays in `WorldParams.surface_age_ga`, computed in
  `params_for` from the model; the processes read the model (air, liquid,
  frost, heat), so the model gains what they need (a frost kind, an
  exosphere flag) rather than `params_for` reading archetype names.
- The albedo of an icy surface needs the surface age inside the climate
  solver (`albedo::cover`), which runs before `params_for`. The age of the
  renewing processes must therefore be computed in the model (a model
  field, deterministic, no rng), and `params_for` keeps only its scatter
  within that bound. The builder places it.
- The exosphere stays a `Gases` value under `AIRLESS_BAR`, so the viewer,
  the craters and the sky need no change.
- The model holds a deterministic renewal age (no rng): the age the
  renewing process allows, capped by the system age. The albedo of goal 4
  reads that age. `params_for` keeps its existing draws in the same order
  and turns the renewal age into the stored age by the scatter it already
  draws (the share of the stagnant range applied to the renewal age).
  Bodies whose renewal age equals today's rule keep their current age
  distribution, not only their range.
- "Icy surface" for goal 4 is the exposed water-ice share of a body whose
  crust is ice (`model_crust_kind == Ice`) and whose air is under
  `ERODING_BAR`. Haze (Titan), volatile frost (Triton's N2) and the frost
  caps and snowballs of rocky planets keep their own albedo paths.
- A volatile frost counts in the albedo only where that frost is stable
  over the body's age (goal 2's test). Goals 2 and 4 are built together:
  the exosphere first, then the albedo law calibrated on the result.
- Io's SO2 supply reads the model's tidal heat (`heat.tidal_w`, already
  separate from radiogenic) and the sulphur inventory (`ROCK_S`); no new
  archetype input.
- Whichever way N2 liquid enters `SurfaceLiquid`, the generated TS types,
  the web checks and the card strings are updated in the same branch.
- World FORMAT 28 is free: no open pull request or branch holds it
  (checked 2026-11-17); #541 and #543 are issues without a branch.
- `archetype_census` (crates/mapgen/examples) is the acceptance tool; it
  prints one row per seed and is extended with the albedo and the frost
  kind.

## Acceptance

`cargo run --release -p mapgen --example archetype_census -- 40`, every
seed of every archetype:

- Titan 0.2-1 Ga, Triton under 0.1 Ga, Mercury, Mars, Callisto and
  Ganymede's old unit at 3 Ga or more and its renewed unit 40-80 % of
  the surface and younger, Europa under 0.2 Ga, Enceladus under 1 Ga,
  Io under 0.002 Ga, Venus 0.3-0.8 Ga, Earth 0.2-0.6 Ga.
- Pressures, albedos and temperatures inside the targets of goals 2, 3, 4
  and 6; Titan 1.3-1.6 bar, 90-97 K, 3-6 % CH4, hydrocarbon world.
- The rows of every other archetype unchanged in class and base look.
- The full test suite and the web checks pass; `planet_census 500` shows
  no class drift beyond what the new liquid N2 and SO2 explain (stated in
  the PR).

## Left to the builder

- The loss law of goal 2 and its source (candidates: photoionisation
  lifetimes at the body's distance; magnetospheric sputtering, Johnson et
  al. 2004 for the Galilean moons); how a moon's host and distance enter.
- The erosion rule of the dense-air process (pressure, liquid kind, or
  both) and its calibration on Titan; the burial rate of the volatile-ice
  process and its source.
- The tidal-episode rule of goal 1 (which moons get one, when, how much
  surface it renews) and the model type that holds the units.
- The albedo law of goal 4 (dark lag versus fresh ice, the rock share).
- The SO2 vapour curve source (Fray & Schmitt 2009 also covers SO2).
- How N2 liquid enters `SurfaceLiquid` (goal 5).
- The cause-tree node layout and strings of goal 8.
- The order of the work and the split into tasks.
