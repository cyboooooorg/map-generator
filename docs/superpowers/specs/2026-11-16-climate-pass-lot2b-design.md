# Climate pass lot 2b: energy budget and cold atmospheres

Refs #423 (track A), climate pass lot 2 (see #427). Closes #443, #447 and
#510. Closes #464 together with lot 2a (Titan's scatter was fixed there;
Triton's air is fixed here).

## Intent

The user wants planets that look the way they would really look at their
distance from their star. Their criterion for the climate pass is the most
physical realism the model can afford. Planet sizes and masses by
distance are not part of this: they belong to the distribution step
(#494 to #497).

A census of `main` at 454c66b (`crates/mapgen/examples/climate_sweep.rs`
on this branch, a lone `earth-like` around a G2V at 4.57 Gyr, and 500
whole systems) shows where the model departs from the real references:

| Case | Model today | Reference |
|---|---|---|
| Earth at 1 AU | SO2 5.5e-3 bar; heat flux 0.031 W/m²; volcanism 0.30 | SO2 about 1e-9 bar; 0.087-0.092 W/m² (Davies & Davies 2010, doi:10.5194/se-1-5-2010); 0.5 |
| Earth-like at 1.5 AU | 288 K under 3 bar CO2 | temperate (Kopparapu et al. 2013, doi:10.1088/0004-637X/765/2/131) |
| Earth-like at 1.6-1.67 AU | frozen at once (258 K, 242 K) with less CO2 (1.9 bar) than at 1.5 AU | temperate up to the maximum greenhouse, about 1.67 AU |
| Every body | Bond albedo 0.3 | Venus 0.76, Earth 0.30, Mars 0.25, Titan 0.27, Triton about 0.75 |
| Frozen earth-likes | sulphuric clouds (from the SO2 excess) | no acid cloud |
| Titan | 90-95 K, 1.3-1.5 bar, CH4 2-3.5 %, clouds of kind "water" | 94 K, 1.45 bar, about 5 % CH4 near the surface, methane clouds |
| Triton | 45-47 K, 1.1e-3 to 2.3e-3 bar | 38 K, 1.4e-5 bar (Tyler et al. 1989) |
| Seed 182, planet d (10.5 AU, T_eq 114 K) | 351 K, liquid water, 26 bar CH4 | a cold world: most of that CH4 condenses |
| Methane liquid | 8 hydrocarbon worlds in 500 systems, smallest CH4 0.0027 bar | liquid only where CH4 can condense |

The inner edge of the habitable zone is already right (runaway from
0.95 AU) and must stay so.

## Goals

1. **The Bond albedo comes from the body.** It is derived from what
   covers the planet: open ocean, bare land, ice and snow, frosts (N2,
   CH4, CO2), clouds and hazes, and the Rayleigh scattering of a thick
   atmosphere. It replaces the constant `EQUILIBRIUM_ALBEDO` = 0.3
   (`crates/mapgen/src/planet/climate.rs`) everywhere the climate uses it.
   The runaway albedo blend of lot 1 (`absorbing`) stays and plugs into
   it.
2. **Ice-albedo feedback and snowball planets.** A planet whose climate
   has both a temperate and a fully frozen solution can be drawn in
   either. A snowball keeps all its relief (mountains, valleys,
   continents) under snow and ice, like after a nuclear winter; frozen
   oceans and snowy continents stay distinguishable (coastlines, flatter
   sea ice of another tint). This is the user's choice.
3. **CO2 accumulates on cold planets** (#510). On a planet with plates
   whose weathering stops (frozen or cold), volcanic CO2 keeps building up
   beyond the outgassed snapshot until the thermostat is satisfied, the
   CO2 condenses, or the carbon inventory runs out (Kasting et al. 1993,
   doi:10.1006/icar.1993.1010). Together with goal 1 (Rayleigh scattering
   of thick CO2), this puts the outer edge of the Sun's habitable zone near
   1.67 AU, the maximum greenhouse. This is also what ends a snowball
   (goal 2).
4. **Sulphur and internal heat** (#447).
   - The sulphur that can outgas comes from the silicate part (mantle and
     crust, bulk silicate Earth about 250 ppm S, McDonough 2003), not from
     the core.
   - SO2 is washed out of an air over liquid water or with free oxygen
     (lifetime of days on Earth); Venus's dry air keeps about 150 ppm.
   - Internal heat gains a secular cooling term (mantle and core cooling;
     Urey ratio about 0.3 to 0.5, Korenaga 2008), decaying with age, so
     that the Earth's total flux is 0.087-0.092 W/m² and its volcanism
     0.5 through the existing `EARTH_VOLCANIC * sqrt(F / 0.087)` law.
5. **Methane** (#443).
   - Methane is cold-trapped where the air is coldest (the tropopause,
     about the skin temperature `T_eq / 2^(1/4)`), not at `T_eq`.
   - Its greenhouse saturates instead of following the Titan-calibrated
     `sqrt(p_CH4 * P)` law 500-fold past its calibration point; its
     near-infrared absorption (anti-greenhouse) is taken into account if
     the builder finds it matters at the census's amounts.
   - A surface hydrocarbon liquid needs enough methane to condense at the
     surface, not a trace.
   - Methane clouds (Titan) are no longer of kind "water".
6. **Triton** (#464). With a bright N2 frost (goal 1), Triton reaches about
   38 K and its N2 air the vapour pressure of N2 ice there, about 1.4e-5
   bar. If the grey climate cannot reach it, the builder reports why
   instead of tuning Triton alone.
7. **Venus's look.** A viewer check (`viewer-visual-check`) confirms that
   a `venus-like` world shows its cloud deck; the census reads
   `cloud_cover` 0 on every runaway. If the deck is missing, it is fixed
   here.

## Non-goals

- Planet sizes, masses and their distribution by distance (#494 to #497,
  the distribution step).
- Subsurface oceans (#463, lot 3). Planet type names (#498, #499, the
  ultra-hot part of #501, lot 4).
- Clouds of slow rotators at the substellar point (#324): the derived
  albedo has a cloud term the issue can later refine.
- The methane-nitrogen liquidus ignoring ethane (the remaining
  simplification noted in #464): it is left to an issue of its own.

## Anti-goals

- A per-archetype or per-body value tuned to hit a reference. Each
  reference above must come out of a general rule; constants are
  calibrated on several bodies at once (as in part 3c's flux rule).
- A world that looks wrong for its state: a snowball as a smooth ball, a
  hot world looking icy, an acid cloud on a cold Earth.
- An Earth at 1 AU that is no longer the Earth: 288 K, liquid oceans,
  plates, about 1 bar, Bond albedo about 0.30.
- A cliff in the distance sweep: temperature, CO2 and albedo move
  continuously with distance except across the real bistable band, where
  the two branches are both valid.
- Legacy algorithms or format gates: worlds change, old ones are marked
  outdated (generation changes policy).

## Design

### Bond albedo

The derived albedo is a cover-weighted mix:

- surface: open water (about 0.06), land by kind (rock, sand, regolith,
  about 0.1 to 0.3), water ice and snow (about 0.5 to 0.8, darker for sea
  ice), N2, CH4 and CO2 frosts (high);
- atmosphere: cloud cover times cloud albedo (water clouds, the sulphuric
  deck of a Venus, Titan's haze), and Rayleigh scattering growing with the
  column (what caps CO2's warming at the maximum greenhouse).

It stays zero-dimensional, like the rest of the planet model (my call):
planet-wide fractions, not tiles. The ocean fraction comes from the
model's water inventory; the ice and snow fraction from the mean surface
temperature (an energy-balance ice line in the manner of Budyko and
Sellers, Earth's caps at 288 K, total cover on the frozen branch); frosts
from the volatiles that condense at the surface temperature; clouds from
the air (water clouds over a wet world, the sulphuric deck when SO2 and
the temperature allow it, hazes). The albedo and the temperature are
solved together as a fixed point, inside or around the CO2 thermostat.
The same cloud term feeds the render's `cloud_cover`, so the albedo and
the look agree.

Archetypes do not carry an albedo: they give inputs (volatiles, water),
and the albedo follows from them. Triton's bright frost comes from its N2
condensing at its own temperature, found by the same fixed point.

Calibration anchors, all at once: Earth 0.30 with its oceans, land, ice
caps and clouds; Venus about 0.76; Mars about 0.25; Titan about 0.27;
Triton about 0.75; airless rocky bodies near 0.1 (Mercury 0.09, the Moon
0.11). After this, `CO2_OPACITY` is recalibrated on Venus at its real
albedo (the reason #510 gives for it).

### Snowball state

When a planet's climate has two stable solutions, the temperate one and a
frozen one (ice albedo), the planet is drawn in one of them on its own
seed stream, so a seed always gives the same state.

The two branches are the fixed points of goal 1 started warm and started
frozen. On the frozen branch, the CO2 is the amount built up so far
(goal 3): a snowball is a planet caught while its CO2 accumulates
towards the thaw.

Solar System archetypes stay faithful (lot 2a): an asked `earth-like`
always takes the warm branch, as `mars-like` and the others take the
state of their body. Drawn planets have no such pin.

Proposed during the discussion and accepted by the user: the chance of the
snowball state follows from the physics, not from a fixed rate. A snowball
on a planet with plates and volcanism lasts until enough CO2 has built up
to melt it (goal 3), so it is:

- rare near 1 AU around the Sun (the Earth spent a small fraction of its
  history in snowball states);
- more likely towards the outer edge, where more CO2 is needed, and on
  planets with little outgassing (stagnant lid, low heat);
- certain past the edge, where no temperate solution exists.

The builder chooses the exact rule (for example the fraction of time a
planet would spend frozen, from the CO2 build-up time against the warm
phase) and documents it.

### Look

- A snowball renders from the frozen base with its full relief. Frozen
  oceans (`Biome::FrozenOcean` exists) and snow-covered continents stay
  distinguishable.
- A frozen world without sulphuric air gets no sulphuric cloud.
- Titan's clouds are methane clouds.
- The card's cause trees explain the new causes for learning: the albedo
  and what sets it, the snowball state and why it happened, the CO2
  build-up, the methane cold trap. The card states that a snowball's
  thaw comes from volcanic CO2.

### Formats

Generated worlds and stored system models change: the world
`FORMAT_VERSION` (23 today) and `SYSTEM_FORMAT_VERSION` (7 today) are
bumped, numbers agreed with the peer sessions at the time.

## Order of work

One branch, `feat/climate-lot2b`. Each step keeps the tests green:

1. Heat and sulphur (#447). If the regime or other interior outputs read
   the same heat, they move too; the Earth must keep its plates and the
   Venus its regime.
2. Derived Bond albedo, Venus recalibration, ice-albedo feedback, the
   snowball state and the CO2 build-up on cold planets (#510), together:
   the build-up is what ends a snowball, so they land as one step.
3. Methane (#443).
4. Triton (#464).
5. Look: snowball rendering, sulphuric clouds, Titan's clouds, Venus's
   deck (after the viewer check), card causes. Generator-side first
   (biomes, look, sky); the viewer or tiles only if a snowball cannot be
   shown otherwise.

## Acceptance

Measured with `climate_sweep` (committed with the branch) and the planet
census, then seen in the viewer:

- Earth at 1 AU: 288 K ± 2, liquid oceans, plates, about 1 bar, Bond
  albedo 0.28-0.32, SO2 under 1e-6 bar, flux 0.087-0.092 W/m²,
  volcanism 0.5.
- Venus-like at 0.72 AU: Bond albedo 0.7-0.8, surface 700-760 K, about
  90 bar, sulphuric deck visible.
- Mars-like at 1.52 AU: Bond albedo 0.2-0.3, about 210 K, no sulphuric
  cloud.
- Earth-like G2V sweep (the archetype is pinned to the warm branch; the
  sweep also prints the frozen branch where it exists): runaway from
  about 0.95 AU (unchanged); temperate up to 1.6-1.7 AU; frozen
  everywhere beyond; no step on either branch except where it ends.
- Snowballs appear in the whole-system census within the habitable zone,
  rarer near S = 1 than near its outer edge.
- Titan: 90-97 K, 1.3-1.6 bar, CH4 3-6 % near the surface, hydrocarbon
  lakes, methane clouds.
- Triton: 35-41 K, N2 1e-5 to 3e-5 bar.
- Census: no planet with T_eq under 150 K above 273 K from a methane
  greenhouse; every hydrocarbon liquid has enough CH4 to condense.
- Viewer (`viewer-visual-check`): a snowball shows mountain ranges and
  readable coastlines; a frozen earth-like has no acid cloud; Venus shows
  its deck; Titan's clouds are not white water clouds.

## Left to the builder

- The exact albedo values per cover, within the ranges above, and how
  cloud cover is derived.
- The snowball probability rule, within the constraints of "Snowball
  state".
- The secular cooling law (Urey ratio or a cooling term decaying with
  age), as long as the Earth lands in range and older or smaller bodies
  get less.
- The form of the CH4 opacity saturation and of the condensation
  criterion, as long as Titan keeps its lakes.
- Whether the CO2 build-up is a closed form or an extension of the
  thermostat's bisection.
- Documentation: README sections on the climate and the card, the
  `docs/system-spec.md` climate part, and each touched constant's source.
