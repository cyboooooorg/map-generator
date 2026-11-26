# Can a nitrogen world be frozen or melted under the same light?

Seen from: the climate of a cold body whose air is nitrogen in vapour
equilibrium with nitrogen frost or liquid nitrogen on its ground: a
Triton with more nitrogen, or a Titan that has lost its methane. It is
the study the user asked for before the first pull request of study 36
(issue #703, branch `feat/climate-longwave-gases`, commits `8bbe27ec`
and `e59e6a92`) could go on:
[Wiki study 38](https://github.com/cyboooooorg/map-generator/wiki/38-nitrogen-phases-audit).
The solver fix it called for landed in that pull request;
[In the app](#in-the-app) says what the climate solver did with such a
body and what it does now.

## Short answer

Yes, two states under the same light are expected. Nitrogen frost is
bright (an albedo of 0.7 to 0.8 on Triton); liquid nitrogen is clear in
visible light, so a sea of it shows the darker ground under it. A body
just under nitrogen's triple point (63.14 K, 0.1253 bar, as the code
takes it from Angus et al. 1979) that keeps its
frost reflects enough light to stay frozen; the same body with its
nitrogen melted absorbs enough to stay liquid. A thicker air over the
liquid also holds more heat in, through the infrared absorption of
colliding N₂ molecules (pairs), which grows as the square of the
pressure. A global model of early Titan without methane finds the same
pair of states: a Triton-like frozen state "stabilized" by the albedo of
its ice, and an unfrozen one with nitrogen seas (Charnay et al. 2014).

Three states are not expected. Between frozen and melted, a body
covered in part by frost is in balance only at one point, and that
balance is unstable: a little warmer, the frost retreats, the body
darkens and warms more. The climate solver of the branch reported such a
state as settled because its stopping test did not check stability; it
now keeps only a balance that attracts (see [In the app](#in-the-app)).
So the bistability is physics, and the third state was a numerical
artefact of the solver.

## The science

### Two states, and why

A body's surface temperature settles where the light it absorbs equals
what it sends to space. Write `F*(T)` for the light at which a surface
at `T` balances. Where `F*` rises with `T`, a small warming sends out
more than it takes in and the state is stable; where `F*` falls with
`T`, a warming takes in more than it sends out and the state is
unstable. This is the slope-stability theorem of energy-balance climate
models: a steady state is stable if the slope of its ice line against
the solar constant is positive and unstable if it is negative (Cahalan
& North 1979, abstract read). When `F*(T)` folds back on itself (an S
curve), two stable branches overlap over a range of light, joined by
an unstable one. The planet keeps the state it came from: hysteresis.

Three things change across nitrogen's melting:

- **The albedo falls.** A frost of small grains scatters light back.
  Liquid nitrogen "is not radiatively active at visible wavelengths and
  so its impact on the albedo should be limited" (Charnay et al. 2014,
  §2, read): the light goes through to the ground under it, darker
  than the frost (a bedrock albedo of 0.6 to 0.7 for Triton in Bertrand
  et al. 2022, abstract read; 0.2 to 0.5 for Titan's past in Charnay et
  al.). Melting makes the body darker, so it warms more: a positive
  feedback, as snow and ice on Earth.
- **The emissivity rises.** N₂ ice radiates with an emissivity of about
  0.8 (large β-N₂ grains, below); a liquid is taken as a black body
  (Charnay et al. take an emissivity of 1 for every surface; no
  measurement of liquid N₂'s was found). A melted surface sends out more
  at the same temperature: a negative feedback.
- **The air thickens.** The air is in vapour equilibrium with the
  ground, so its pressure follows the saturation curve: from 0.054 bar
  at 59.3 K to 0.1253 bar at the triple point and 0.16 bar at 64.4 K
  (Fray & Schmitt's β-N₂ fit and Span et al.'s liquid curve, the model's
  `science::volatiles`). The pairs' absorption grows as the square of
  the density, so the greenhouse grows with temperature: a positive
  feedback, the one that collapses a thin N₂ air when it cools (Lorenz,
  McKay & Lunine 1997, abstract read: methane depletion "may have
  allowed Titan's atmosphere to cool so that nitrogen, its main
  constituent, condenses onto the surface, collapsing Titan into a
  Triton-like frozen state with a thin atmosphere").

Whether melting is unstable depends first on the net of the first two
(the third only adds to the warming): on
`(1 − A)/ε`, which sets the temperature of a surface that balances
`ε σ T⁴ = (1 − A) F / 4`. For a frost of albedo 0.75 and emissivity
0.80, `(1 − A)/ε` = 0.31; for a liquid over a ground of albedo 0.6 and
emissivity 1, 0.40 (derived). The liquid is the warmer state at the
same light, so the frost is lost all at once once it starts to go. Over
a ground brighter than about 0.69 (where `1 − A` falls to the frost's
0.31, derived), the emissivity would win and, unless the thickening
air's greenhouse tips it back, the body would sit at the triple point
with part of its frost, a stable two-phase state, the way melting snow
holds 0 °C.

The literature finds the pair of states:

- **Early Titan without methane.** Charnay et al. 2014 (arXiv:1407.1791,
  read), a three-dimensional model of a pure-nitrogen Titan with the
  N₂–N₂ pairs and nitrogen clouds: "an albedo higher than 0.65 is
  required at 4 Ga to trigger nitrogen freezing at the surface,
  corresponding to a surface pressure below the triple point of N2 (i.e.
  0.125 bar). With nitrogen ice on the surface, the albedo could rise to
  ∼0.7-0.8, as on Triton, stabilizing this frozen state. According to
  our model, an albedo of 0.8 is sufficient to maintain a frozen state
  even today." Their Table 2, at the light of 4 Ga (2.8 W/m² against
  3.77 W/m² today), gives 0.21 bar and 67.2 K for a surface albedo of
  0.6, 0.09 bar and 61.7 K for 0.7, and 0.03 bar and 56.5 K for 0.8.
- **Mars's CO₂, the same mechanism with another gas.** The CO₂ of Mars
  is shared between its air and its caps, and "this Martian
  CO2-dominated climate system has two different stable steady states of
  climate modes under the present condition (Fanale and Salvail 1982;
  McKay et al. 1991; Haberle et al. 1994; Nakamura and Tajika 2001,
  2002)" (Watanabe, Tajika & Kamada 2026, arXiv:2411.06871, read): one
  with a permanent CO₂ cap and a collapsed air, one without.

No source found describes a third stable state between the frozen and
the melted one.

### The state in between

In a model of one temperature for the whole body (zero-dimensional),
the frost's share at the triple point is free: every share from 0 to 1
has the same surface temperature, and only one share balances the
light. With melting unstable (above), that balance is the unstable
middle branch of the S curve: it separates the frozen and the melted
states and is never reached by a body that starts elsewhere.

In three dimensions it is different, because "the nitrogen cycle
operates differently, with all the atmosphere in equilibrium with the
coldest region" (Charnay et al. 2014, §4.2): the frost or the liquid
collects at the poles, the cold traps, and the pressure follows their
temperature, not the mean one. Charnay et al. find polar nitrogen seas
and a warmer equator together. A partial cover is stable there for the
reason ice caps are stable on Earth: the light falls off towards the
poles. A one-temperature model has no poles, so it cannot hold that
state; it is a limitation of the model, not a third branch.

### The ingredients near 60 K

| Ingredient | In the model | What the sources give | Verdict |
| --- | --- | --- | --- |
| N₂ vapour pressure | `nitrogen_saturation_pressure`: β-N₂ fit of Fray & Schmitt 2009 pinned to NIST's triple point (63.14 K, 0.1253 bar), Span et al. 2000 above | 0.0534 bar at 59.3 K from the raw `N2-2` polynomial (Lisse et al. 2021, Table 2), 0.0551 bar scaled by its 3.1 % pin (derived); the model gives 0.0544 bar at 59.25 K | sourced, right |
| Phase of the ice | β-N₂ (hexagonal) from 35.61 K to the triple point | the α–β transition is at 35.61 K (Scott 1976, as Tegler et al. 2019 quote it); at 59 to 63 K the ice is β-N₂ | right |
| N₂ ice emissivity | 0.80 (`N2_ICE_EMISSIVITY`, Lellouch et al. 2011 as Meza et al. 2019 adopt it) | pure β-N₂: 0.85 for 1 cm grains, 0.40 for 1 mm (Stansberry, Pisano & Yelle 1996, abstract) | sourced; grain size at 60 K unknown |
| N₂ frost albedo | 0.75 (`FROST_ALBEDO`, a calibration on Triton's 38 ± 1 K) | "∼0.7-0.8, as on Triton" (Charnay et al. 2014); none measured near the triple point | calibration; not known at 60 K |
| Liquid N₂ albedo | none of its own: the ground under it shows (Titan's water ice, 0.6) | "not radiatively active at visible wavelengths" (Charnay et al. 2014) | matches |
| Liquid N₂ emissivity | 1 (no frost) | Charnay et al. take 1 for every surface; no measurement found | assumption |
| N₂–N₂ pairs at 60 K | HITRAN `N2-N2_2021.cia`, its coldest block (70 K) used below 70 K | HITRAN's main roto-translational set starts at 70 K (Chistikov et al. 2019); Borysow & Frommhold 1986 model 50 to 300 K (abstract read) | right order; somewhat too weak at 60 K (below) |
| Temperature of the air | dry adiabat from the surface to the skin temperature (N₂'s pseudo-adiabat since #741, [below](#what-the-code-does-since)) | an air saturated at the ground follows its saturation curve (pure condensable: Clausius–Clapeyron, Pierrehumbert 2010, §2.7, as the column applies it to CO₂ and water) | **missing law: too much greenhouse** (below); added by #741 |

**The pairs are about right for the air the model draws.** Read from
HITRAN's file, the N₂–N₂ absorption at 70 K peaks at 2.741e-44 cm⁵
molecule⁻² at 53.3 cm⁻¹, and its band integral is 1.742e-42, against
2.200e-44 and 1.471e-42 at 80 K: 18 % more per 10 K of cooling. The
column of an isothermal air, `∫ n² dz = n₀² H / 2`, on the Titan-like
body of the test (1.389 m/s²) puts the peak's vertical optical depth at
0.77 for 0.054 bar at 59.3 K, 3.3 for 0.116 bar at 62.75 K and 6.0 for
0.158 bar at 64.4 K (derived; 353 for Titan's 1.467 bar). A
line-by-line sum over HITRAN's 70 K spectrum through the model's own
profile (a dry adiabat to the skin temperature, two streams with the
diffusivity 1.66; scratch script, not committed) gives an outgoing
radiation of 0.9424, 0.8607 and 0.8407 of `σ T⁴` for the three states,
against the model's 0.9432, 0.8705 and 0.8417: the model's column
absorbs what HITRAN says for the profile it draws. Below 70 K it reads
the 70 K block; at 60 K the true absorption is stronger (the trend
above), so the model's greenhouse there is, if anything, short.

**The profile is not.** On a body whose ground holds nitrogen frost or
liquid, the air at the ground is saturated. A parcel that rises cools
along the dry adiabat, `d T / d ln p = (R / c_p) T`, about 17.6 K per
e-fold of pressure at 62 K; nitrogen then condenses out of it, and its
latent heat holds the parcel on the saturation curve,
`d T / d ln p = R T² / L`, about 4.6 K per e-fold with the sublimation
heat of 6.7 to 7.0 kJ/mol (derived; the slope of the β fit). Charnay et
al. condense nitrogen in the air wherever it falls under the saturation
temperature (§2). The column does this for water and for CO₂, not for
N₂: one scale height up its N₂ air is at 44.6 to 48.4 K where the
saturation curve keeps it at 55.3 to 59.5 K (the column does since
issue #741, [What the code does since](#what-the-code-does-since)). The
same line-by-line sum
along the saturation curve gives 0.9818, 0.9503 and 0.9290 of `σ T⁴`:
the pairs' greenhouse is a third to a half of what the model draws
(from 5.8 % to 1.8 %, from 14 % to 5.0 %, from 16 % to 7.1 % of the
outgoing radiation).

### What we do not know

- The albedo and the grain size of N₂ ice near its triple point: Triton
  and Pluto are at 37 to 40 K. Ices sinter faster when warmer (Molaro et
  al. 2019 for water ice, abstract read), which coarsens the grains,
  raises the emissivity (Stansberry et al.'s 1 mm against 1 cm) and
  could darken the frost; no measurement was found.
- The far-infrared emissivity of liquid nitrogen.
- The N₂–N₂ absorption between 50 and 70 K on HITRAN's grid: Borysow &
  Frommhold 1986 give it, but not in HITRAN's current main set.
- How wide the hysteresis is on a body with poles: the
  three-dimensional model of Charnay et al. does not map it.

## In the app

### What the branch's climate does

Measured on the branch `feat/climate-longwave-gases` at `e59e6a92`, on
the test body of `the_fixed_point_converges_until_the_nitrogen_frost_melts`
(the reference Titan, 0.5 bar of N₂ and no methane, its water ice under
the frost; scratch probes, not committed). The light is in Earth fluxes;
Titan's is 0.010919.

The model's own `F*(T)`, its derived albedo and emissivity at each
temperature balanced against its column, is an S curve:

| Branch | Temperature | `F*`, the branch | `F*`, a transparent air | `F*`, pairs on a saturated air (estimate) |
| --- | --- | --- | --- | --- |
| Frost (albedo 0.752, ε 0.80) | 59.25 K | 0.0062168 | 0.0065935 | 0.0064743 |
| Frost, its last point | 62.10 K | 0.0070411 | 0.0080282 | 0.0076876 |
| Melting band (frost cover falling) | 62.75 K | 0.0062031 | 0.0071309 | 0.0067757 |
| Liquid, its first point (albedo 0.613, ε 1) | 63.15 K | 0.0059183 | 0.0068183 | 0.0064486 |
| Liquid | 64.35 K | 0.0062274 | 0.0074039 | 0.0068882 |
| Liquid | 66.85 K | 0.0068460 | 0.0087759 | 0.0078437 |

`F*` rises on the frost branch, falls across the 1 K melting band
(62.14 to 63.14 K, where `PHASE_BAND` ramps the frost's cover from 1 to
0) and rises again on the liquid branch. So the frozen and the melted
states are both stable from 0.005918 to 0.007041 Earth fluxes (54 % to
64 % of Titan's light, a range of 19 %), and the melting band is the
unstable branch between them. A transparent air (the model before the
pairs) gives the same shape from 0.006818 to 0.008028 (18 %). The pairs
lower the light both states need and weaken the branches' slopes (the
frost branch's `d ln F*/dT`, the chord of the table from 59.25 to
62.10 K, falls from 0.069 to 0.044 per K, derived; the study first
quoted 0.068 and 0.052, local slopes that the table's points do not
show);
they make no third branch. The last column is the scratch line-by-line
sum (above) along N₂'s saturation curve, with the model's albedo,
emissivity and air at each temperature; the same sum on the model's dry
adiabat gives the branch's own `F*` within 0.2 %. With nitrogen
condensing in the air the two states overlap from 0.006449 to 0.007688
(19 %): the bistability stays, about 9 % higher in light.

### Why the sweep found three states

The test sweeps the light and starts every climate warm. Three probes
show what it found:

- **From five starts and two tolerances** (frost, three points of the
  melting band, liquid; the default `ALBEDO_TOLERANCE_K` of 0.1 K and
  0.001 K) at 0.0062424 and 0.0062481, every solve ends at 59.33 to
  59.35 K (frost) or 64.40 to 64.43 K (liquid). The frost branch
  started from frost holds to 0.0069 (61.57 K) and is gone at 0.00703;
  the liquid one holds to 0.00595 and is gone at 0.00592, as the S
  curve says.
- **From the warm start**, at 0.0062424, the solver stops at 62.749 K
  under an albedo of 0.6720 and an emissivity of 0.9156 (push 0.098 K,
  inside the 0.1 K tolerance), and at a tolerance of 0.001 K at
  62.739 K under 0.6552 and 0.9582 (push 0.0000 K). The bracket of
  #729 and its Illinois steps find the zero of the push, and a zero is
  also an unstable balance: the stopping test (temperature and push
  under the tolerance) does not ask whether the push brings a
  disturbed climate back.
- **The albedo and emissivity it stores are not the ones it derives.**
  At that 62.739 K the body derives 0.6690 and 0.9197. The fixed point
  moves two numbers against one residual, the radiative equilibrium
  `T_eq ε^-¼`, which fixes only `(1 − A)/ε`: every pair on that line
  passes (0.6552 and 0.9582 give 0.360, 0.6690 and 0.9197 give 0.360).
  The scan's "melting frost" spreads its emissivity from 0.91 to 1.00
  at one temperature for that reason.

So the two jumps, each between neighbours of the sweep 0.26 % of light
apart (the finer scan put each within 0.013 %), are where the warm
start's path changes its landing, between the frozen branch, the unstable root and
the liquid branch; they are not thresholds of the model. The one jump
of the model before the pairs (59.77 to 63.15 K at 0.006861 to
0.006878) was the same: the warm start lands on the liquid branch a
little above its first point (0.006818), and from a frozen start the
frost would hold to 0.008028. The "melting frost at 63.13 K" that the
stellar spectra of #655 gave on `main` sits at the band's warm end,
where the unstable branch meets the liquid one (not probed on `main`).

### What follows for the code

1. **Keep the bistability.** It is the expected physics (Charnay et al.
   2014; the Mars analogue). The tier-3 test can allow one jump per
   phase boundary, from frost to liquid, once the solver stores stable
   states only.
2. **Do not store an unstable root** (numerics, `system`). At the end
   of `settle`, a settled climate whose push grows with its radiative
   temperature (the slope of the last two steps, or one disturbed step)
   is unstable: the solver goes on from the side the push leaves to,
   as the runaway edge and #729 already do for a bracket whose ends
   push apart. And settle on the derived pair itself: stop when the
   albedo and emissivity used are within a tolerance of those derived,
   or store the derived pair. Both are `Definition:`s of the solver, no
   new number.
3. **Condense nitrogen in the column** (a law, `science`, issue #741):
   where the air is saturated in N₂ at the ground, the column follows N₂'s
   saturation curve (frost below 63.14 K, liquid above) up to the skin
   temperature, as it follows CO₂'s. It cuts the pairs' greenhouse at
   these pressures to a third to a half and raises the light of the
   hysteresis by about 9 % (above), without removing it. It touches only
   bodies whose nitrogen condenses at the ground.
4. **Extend the pairs below 70 K** (issue #742) with Borysow &
   Frommhold 1986 when it is read; until then the 70 K block stands, a
   little weak.
5. **Leave `PHASE_BAND` a numerical smoothing.** Its width moves the
   frost branch's last point (at `T_m − w`) and so the top of the
   hysteresis, by about 4 to 5 % of light between `w` = 1 K and none
   (derived: 4.4 % from the frost branch's chord, `d ln F*/dT` = 0.044
   per K; 5.2 % from the 0.052 per K the study first quoted).
   Its stated purpose, a solution at the triple point, holds only when
   the emissivity's negative feedback outweighs the albedo's (a ground
   at least as bright as the frost); over a darker ground it holds the
   unstable branch.

### What the code does since

Item 2 landed with the N₂ pairs, in #703's first pull request:

- `settle` keeps a root only where it attracts: the push taken again at
  a radiative equilibrium `ALBEDO_TOLERANCE_K` (0.1 K) colder must fall
  from there to the root (`attracting`, the condition the bracket asks
  of its ends; `probe_root`). Only a root the bracket's steps reached
  needs that probe: the plain step leaves an unstable root by itself, so
  a root it reached with its push falling is kept as it is. An unstable
  root is left from its colder probe with the plain step, so the fixed
  point reaches the cold branch, the side #729 stores a jump on. A root
  the plain step comes back to from that probe is kept.
- It settles only once the albedo's and the emissivity's own residuals
  are under the tolerance too (`pair_residuals`), so the stored pair is
  the derived one.
- The warm-start sweep jumps once, 59.361 K (frost) at 0.006252 to
  64.503 K (liquid) at 0.006269 Earth fluxes. Followed along the light,
  the frost branch holds up to 0.0070506 and the liquid one down to
  0.0059258, within three of the sweep's steps (about the 0.1 K
  tolerance) of the folds above (tier-3 test
  `a_nitrogen_world_keeps_its_frost_and_its_liquid_over_a_range_of_light`).
- Earth, Mars, Titan and the Triton-like body settle where they did, to
  the printed digit: their roots attract and their pairs were derived.

Item 3 landed with issue #741, in its own pull request:

- **The law.** Where the air reaches N₂'s saturation
  (`volatiles::nitrogen_saturation_pressure`: the β-N₂ fit of Fray &
  Schmitt below 63.14 K, Span et al. 2000 above), the column condenses
  N₂ and follows its pseudo-adiabat (`column::n2_lapse`): the
  construction it applies to CO₂ (Pierrehumbert 2010, §2.7, per mole),
  with N₂ the condensable, the gases other than N₂ and CO₂ the air it
  condenses out of, and N₂'s share thinning to what saturation leaves.
  For a pure N₂ air it is the saturation curve itself,
  `d T / d ln p = R T² / L`.
- **The latent heat** is the slope of the curve it condenses on, by
  Clausius–Clapeyron for an ideal vapour (`nitrogen_latent_j_mol_over`,
  as CO₂'s): 6.70 kJ/mol at 59.25 K and 6.64 kJ/mol at the triple
  point over the frost, 6.10 kJ/mol at the triple point over the
  liquid. So the column cools 4.8 K per e-fold of pressure at 62 K,
  against 17.7 K on the dry adiabat (the 4.6 K above took 7.0 kJ/mol,
  the top of the β fit's range). Against Span et al.'s saturation table
  (p. 1403 of the NIST reprint: the enthalpies and densities of the
  saturated liquid and vapour), the liquid curve's slope meets
  Clapeyron's `Δh / (T Δv)` within 0.024 % at 63.151, 70 and 77 K; the
  ideal-vapour heat is 1.0 % over the real `Δh` at the triple point
  (6099.7 against 6037.3 J/mol), 2.4 % at 70 K and 4.9 % at 77 K, a
  declared simplification that the pseudo-adiabat of a pure N₂ air does
  not feel (it follows the curve whatever `L`). The heat capacity is
  `N2_HEAT_CAPACITY`, 29.12 J/(mol·K) at 298 K; Span et al.'s ideal-gas
  table gives `c_p/R` = 3.5003 from 25 to 75 K (29.10 J/(mol·K), p. 1402),
  so it holds at 60 K within 0.06 %.
- **The triple point.** The heat jumps there from the liquid's 6.10 to
  the frost's 6.64 kJ/mol, so the step that crosses 63.14 K is split
  there from the start (`column::n2_step`, through the `split_step`
  that water's and CO₂'s steps share): the cold N₂ body sits on that
  point. The jump, 0.54 kJ/mol, is the fusion the two fits imply; a
  measured enthalpy of fusion was not read (Giauque & Clayton 1933, the
  usual source, is closed). The frost's curve sits 0.27 % over the
  liquid's there (*The nitrogen vapour curves* in
  [Planet atmospheres](planet-atmospheres.md)), so a column that starts
  over the liquid runs 0.013 K over the frost's curve above it.
- **Tests** (tier 1, `column_tests.rs`, `volatiles_tests.rs`). A pure N₂
  column over its frost at 59.25 and 62.75 K stays within 5.5e-4 K of
  N₂'s saturation temperature at every level, and over its liquid at
  64.4 K within that plus the join (0.0135 K); one e-fold of pressure
  up it is at 55.21, 58.21 and 59.51 K, where the dry adiabat put it at
  44.53, 47.16 and 48.40 K. Its outgoing radiation is 0.9817, 0.9490 and
  0.9278 of `σ T⁴`, against the 0.9818, 0.9503 and 0.9290 of the
  line-by-line sum above (within 0.14 %; printed, not asserted). A
  saturated N₂ air swept from 63.5 to 66 K, its column crossing the
  triple point from its first layer to its fourth, sends out a rise that
  changes by 0.26 % from one step to the next (6.3 % with a step taken
  over the phase of each point it reads); one step across the triple
  point lands within 1.3e-4 K of the same step cut into 2000 (0.013 to
  0.022 K over each point's phase). The bounds are calibrations of the
  model's numerics (below).
- **The S curve.** The warm-start sweep still jumps once, now from
  59.470 K (frost) at 0.006565 to 63.526 K (liquid) at 0.006581 Earth
  fluxes, the absorbed light and `ε OLR` both ×1.5614 across it.
  Followed along the light, the frost holds up to 0.0076953 and the
  liquid down to 0.0064470, against the 0.007688 and 0.006449 estimated
  above: within 0.1 %. The bistability stays, about 9 % higher in
  light.
- **It reaches Titan too.** Study 38 expected the law to touch only
  bodies whose N₂ condenses at the ground; it did not probe Titan's
  column. The sourced Titan's dry adiabat runs from 92.3 K to its
  68.95 K skin, and on the way its N₂ passes saturation at about
  0.65 bar and 73.5 K (20 % over it at 0.58 bar). The column now
  condenses N₂ from there to the skin, up to 2.7 K warmer than the dry
  adiabat, and sends out more: the tier-1 column settles at 91.81 K
  against its 92.30 K, the settled Titan at 92.33 K against 92.64 K on
  its measured air and 92.61 K against 92.97 K on the chain's own. The
  tier-1 test's floor, 92.00 K (`TITAN_MISSING_WARMING_K`, a calibration
  the user accepted on 2026-11-25), is 0.19 K over the new column. A
  thin methane world swept across its sea's freezing point starts at
  61.49 K instead of 61.77 K, and a methane Earth frozen at 9.57 AU
  settles at 80.402 K instead of 80.520 K. Earth, Mars, the methane
  Earth's outer edge (1.2534 AU) and its cycling orbit (1.264 AU) do not
  move: their columns stay over N₂'s critical point, 126.192 K. The
  Triton-like body moves by 0.009 K, inside the solver's tolerance.
- **Calibrations** (tests only, numerics, no source gives a number):
  `NITROGEN_PROFILE_TOLERANCE_K` (0.002 K, a factor 3.6 over the
  5.5e-4 K measured, against the dry adiabat's 11 K),
  `NITROGEN_SMOOTH_RISE_CHANGE` (1 %, between the 0.26 % split and the
  6.3 % unsplit) and `N2_STEP_TOLERANCE_K` (0.001 K, between 1.3e-4 K
  and 0.013 K).
- **Limitations.** Argon and methane do not condense in the column (a
  Titan's methane is cold-trapped at its tropopause instead, and argon
  is a trace); the frost's albedo and grain size at 60 K and the pairs
  below 70 K (#742) are as above; a body is one temperature, with no
  cold traps at its poles.

Item 4 waits in #742.

## Decisions (user, 2026-11-26)

The user took every recommendation of the study:

1. **The bistability is accepted, the tristability is not.** A frozen
   and a melted N₂ state under the same light are physics; the melting
   frost between them is an unstable balance, never a stored climate.
2. **The solver is fixed in study 36's first pull request** (issue
   #703, branch `feat/climate-longwave-gases`): `settle` rejects an
   unstable root and converges on the albedo and emissivity it derives,
   with no new number. A tier-3 hysteresis test follows each branch up
   and down the light, and the sweep's test keeps one jump, frost to
   liquid.
3. **Nitrogen condenses in the column** in its own pull request right
   after that one (issue #741): a saturated N₂ air follows N₂'s
   saturation curve, as the column's CO₂ does.
4. **The N₂–N₂ pairs below 70 K** wait for Borysow & Frommhold 1986 to
   be read (issue #742).
5. **Departure: a partial N₂ cover held by polar cold traps.** In
   three dimensions the frost or the liquid gathers at the poles and
   sets the pressure there (Charnay et al. 2014, §4.2), so a body can
   hold polar N₂ seas and a warmer equator together. The climate solves
   one temperature for the whole body: it keeps only the frozen and the
   melted states, and steps from one to the other where a body with
   poles would pass through a partial cover.

## Sources

1. Bertrand, T., Lellouch, E., Holler, B. J. et al. 2022, "Volatile
   transport modeling on Triton with new observational constraints",
   *Icarus* 373, 114764, doi:10.1016/j.icarus.2021.114764 (abstract
   read: bedrock albedo 0.6 to 0.7).
2. Borysow, A., Frommhold, L. 1986, "Collision-induced rototranslational
   absorption spectra of N2-N2 pairs for temperatures from 50 to 300 K",
   *ApJ* 311, 1043, doi:10.1086/164841 (abstract read).
3. Cahalan, R. F., North, G. R. 1979, "A stability theorem for
   energy-balance climate models", *J. Atmos. Sci.* 36, 1178,
   doi:10.1175/1520-0469(1979)036<1178:ASTFEB>2.0.CO;2 (abstract read).
4. Charnay, B., Forget, F., Tobie, G., Sotin, C., Wordsworth, R. 2014,
   "Titan's past and future: 3D modeling of a pure nitrogen atmosphere
   and geological implications", *Icarus* 241, 269,
   doi:10.1016/j.icarus.2014.07.009 (arXiv:1407.1791, read: §2, §4.2,
   §4.3, conclusions, Table 2).
5. Fray, N., Schmitt, B. 2009, *Planet. Space Sci.* 57, 2053,
   doi:10.1016/j.pss.2009.09.011, through Lisse et al. 2021 (*Icarus*
   356, 114072, arXiv:2009.02277, Table 2), as
   [Planet atmospheres](planet-atmospheres.md) reads them.
6. HITRAN collision-induced absorption, `N2-N2_2021.cia` (the 70 K and
   80 K blocks read) and `CIA_Readme_2024.pdf` (Table 1: the main N₂–N₂
   roto-translational set, 0-450 cm⁻¹, 70-200 K, Chistikov et al. 2019),
   <https://hitran.org/cia/>, as downloaded for study 36.
7. Lorenz, R. D., McKay, C. P., Lunine, J. I. 1997, "Photochemically
   driven collapse of Titan's atmosphere", *Science* 275, 642,
   doi:10.1126/science.275.5300.642 (abstract read).
8. Lorenz, R. D., McKay, C. P., Lunine, J. I. 1999, "Analytic
   investigation of climate stability on Titan: sensitivity to volatile
   inventory", *Planet. Space Sci.* 47, 1503,
   doi:10.1016/S0032-0633(99)00038-0 (not read: no abstract or copy
   reached).
9. Molaro, J. L., Choukroun, M., Phillips, C. B. et al. 2019, "The
   microstructural evolution of water ice in the Solar System through
   sintering", *J. Geophys. Res. Planets* 124, 243,
   doi:10.1029/2018JE005773 (abstract read).
10. Pierrehumbert, R. T. 2010, *Principles of Planetary Climate*,
    Cambridge University Press, §2.7 (not read; as the column's code
    and [radiative transfer](radiative-transfer.md) quote it).
11. Scott, T. A. 1976, *Phys. Rep.* 27, 89 (not read; through Tegler et
    al. 2019, arXiv:1905.13250, read for study 24).
12. Stansberry, J. A., Pisano, D. J., Yelle, R. V. 1996, *Planet. Space
    Sci.* 44, 945 (abstract excerpts, as
    [Planet atmospheres](planet-atmospheres.md) quotes them).
13. Watanabe, Y., Tajika, E., Kamada, A. 2026, "Behaviors of Martian
    CO2-driven dry climate system and conditions for atmospheric
    collapses", *Icarus* 444, 116795, doi:10.1016/j.icarus.2025.116795
    (arXiv:2411.06871, read: abstract, introduction, results on the
    multiple climate modes).
14. Angus, S., de Reuck, K. M. et al. 1979, nitrogen's triple point,
    63.14 K and 0.1253 bar, as the code takes it (`volatiles.rs`), through
    the NIST Chemistry WebBook (as [Planet
    atmospheres](planet-atmospheres.md) reads it).
15. Span, R., Lemmon, E. W., Jacobsen, R. T., Wagner, W., Yokozeki, A.
    2000, "A reference equation of state for the thermodynamic properties
    of nitrogen for temperatures from 63.151 to 1000 K and pressures to
    2200 MPa", *J. Phys. Chem. Ref. Data* 29, 1361, doi:10.1063/1.1349047
    (the liquid's vapour-pressure curve, as the code takes it; read in the
    NIST reprint, <https://srd.nist.gov/JPCRD/jpcrd593.pdf>, for #741: the
    ideal-gas heat capacity table, p. 1402, and the saturation table,
    p. 1403).
16. Lellouch, E., Stansberry, J., Emery, J., Grundy, W., Cruikshank, D. P.
    2011, "Thermal properties of Pluto's and Charon's surfaces from Spitzer
    observations", *Icarus* 214, 701 (the N₂ ice emissivity 0.8; not read,
    cited through Meza et al. 2019).
17. Meza, E. et al. 2019, "Pluto's lower atmosphere and pressure evolution
    from ground-based stellar occultations, 1988-2016", *A&A* 625, A42,
    doi:10.1051/0004-6361/201834281, arXiv:1903.02315 (§ 4 read for
    [Planet atmospheres](planet-atmospheres.md)).
18. Giauque, W. F., Clayton, J. O. 1933, "The heat capacity and entropy
    of nitrogen. Heat of vaporization. Vapor pressures of solid and
    liquid", *J. Am. Chem. Soc.* 55, 4875, doi:10.1021/ja01339a024
    (closed, not read: the calorimetric enthalpy of fusion, which #741
    could not compare with the 0.54 kJ/mol the two fits imply).

## See also

- [Planet atmospheres, climate and surface](planet-atmospheres.md),
  "Phase changes and convergence" and "A climate with no fixed point"
- [How does a planet's air set its outgoing heat and its albedo?](radiative-transfer.md)
