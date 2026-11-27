# How much methane and nitrous oxide does life keep in the air?

Seen from: the outgoing heat of every living planet whose air could hold
methane (CH₄) or nitrous oxide (N₂O), and the cause trees that explain
it. It is the study behind issue #719,
[Wiki study 37](https://github.com/cyboooooorg/map-generator/wiki/37-life-gases-audit),
written before any code. The gases' infrared lines are study 36
([Wiki study 36](https://github.com/cyboooooorg/map-generator/wiki/36-longwave-gases-audit),
issue #703); ozone, made the same way from O₂ and the host's light, is
[How much ozone does an oxygen air hold?](ozone.md).

## Short answer

Life breathes out methane (microbes in wetlands, rice fields, guts and
lake beds) and N₂O (microbes in soils and the ocean that turn nitrate
into nitrogen). Neither stays: the air destroys methane in about ten
years, mostly by the OH radical that sunlight makes from ozone and
water, and N₂O in about 120 to 140 years, by ultraviolet light in the
stratosphere. What the air holds is the balance of the two: the amount
made each year times the time a molecule lasts. Before industry, Earth
held 729 ppb of methane and 270 ppb of N₂O (air trapped in polar ice).

The time a molecule lasts depends on the star. A cooler star sends less
of the near ultraviolet that makes OH, so the same biosphere keeps far
more methane: at Earth's fluxes, published models of an Earth around
K and active M dwarfs hold 125 to 1000 ppm (Rugheimer & Kaltenegger
2018; Rugheimer et al. 2015a), 70 to more than 600 times what the same
models give around the Sun. N₂O, split by shorter ultraviolet, rises
less: up to ten times. Carried to the app's Earth (derived here), that
would be about 50 to more than 420 ppm of methane around K and M hosts.
It also depends on the oxygen and on the stage of life: a young,
oxygen-free Earth held a hundred to ten thousand times today's methane
and almost no N₂O.

No published formula gives either gas against the star. The app can
build Earth's values from a measured flux and a measured lifetime, and
carry them to other hosts only through the ratios of one family of
photochemical models: a calibration, like the ozone column's.

## The science

### Earth's pre-industrial methane and N₂O

**The abundances.** Ice cores from Antarctica and Greenland give a
global mean of 729.2 ± 9.4 ppb of CH₄ in 1750 and 807.6 ± 13.8 ppb in
1850, and 270.1 ± 6.0 ppb of N₂O in 1750 and 272.1 ± 5.7 ppb in 1850
(IPCC AR6 WG1, Gulev et al. 2021, §2.2.3.2.2 and §2.2.3.2.3). Between
0 and 1850 CE methane varied between 625 and 807 ppb; N₂O by 5 to 10 ppb
over centuries. In 2019 they were 1866.3 ± 3.3 ppb and 332.1 ± 0.4 ppb (same
chapter).

**The budgets.** Prather et al. (2012, Table 1) derive the
pre-industrial budget from the ice-core abundance and a modelled
lifetime, every term with its 1σ:

| Quantity, year 1750 | CH₄ | N₂O |
| --- | --- | --- |
| Abundance (tropospheric mean) | 700 ± 25 ppb | 270 ± 7 ppb |
| Lifetime | 9.5 ± 1.3 yr | 142 ± 14 yr |
| Emissions, all natural | 202 ± 28 Tg CH₄/yr | 9.1 ± 1.0 Tg N/yr |
| Present day (2010) for comparison | 1795 ppb, 9.1 ± 0.9 yr, 554 ± 56 Tg/yr | 323 ppb, 131 ± 10 yr, 15.7 ± 1.1 Tg N/yr |

A ppb of the troposphere is 2.75 Tg of CH₄ (the "fill factor" that
counts the methane missing in the stratosphere; Prather et al. 2012, as
AR6 WG1 Table 6.2 and Saunois et al. 2020 cite it) and 4.79 Tg of
nitrogen as N₂O (Prather et al. 2012, as Tian et al. 2024 cite it).
Their 700 ppb predates the scale correction of the older ice-core
measurements; AR6's 729.2 is on today's scale.

**Where they come from today.** The natural sources of methane are
wetlands (180 Tg/yr from atmospheric inversions, 149 from bottom-up
estimates, 2008-2017), freshwater, geological seeps, termites, wild
animals and the ocean: 215 Tg/yr top-down, 371 bottom-up, the two
methods not reconciled (AR6 WG1, Canadell et al. 2021, Table 5.2). The
natural sources of N₂O are soils under natural vegetation (5.6 Tg N/yr),
the open ocean (3.4), rivers and coasts (0.3) and atmospheric chemistry
(0.4): 9.7 (8.0 to 12.0) Tg N/yr in 2007-2016 (same chapter, Table 5.3,
from Tian et al. 2020). In Tian et al. (2024), natural soils emit 6.4
(3.9 to 8.6) Tg N/yr: the 2010-2019 mean of the models' natural-soil
baseline, a run with pre-industrial (1850) land cover, not an 1850
emission; all natural sources give 11.8 (7.3 to 15.9) Tg N/yr in
2010-2019.

**Where they go.** About 90 % of methane's loss is its reaction with OH
in the troposphere, 5 % its uptake by bacteria in aerobic soils and 5 %
the OH, excited oxygen O(¹D) and chlorine of the stratosphere (AR6 WG1,
§5.2.2, from Saunois et al. 2020). Present-day lifetimes: 9.7 ± 1.1 yr
against chemistry, 135 ± 44 yr against soils, 9.1 ± 0.9 yr in all (AR6
WG1, Table 6.2). N₂O is lost to photolysis and O(¹D) in the
stratosphere, 13.1 (12.4 to 13.6) Tg N/yr; its lifetime is now
116 ± 9 yr (Prather et al. 2015, as AR6 WG1 §5.2.3.1 gives it), 119.3
years in the 2000s and 117 in the 2010s (Tian et al. 2024, from Prather
et al. 2023).

**The feedback.** More methane uses up OH and lengthens its own
lifetime: the perturbation lifetime is $f$ times the total one, with
$f = 1/(1-s)$ and $s = \partial \ln \tau / \partial \ln [\mathrm{CH_4}]$;
AR6 assesses $f$ = 1.30 ± 0.07 (AR6 WG1, §6.3.1). From $B = \Phi\tau$
and the definition of $s$, a burden then grows locally as the flux to
the power $f$, 1.3 (derived here). N₂O's own feedback is small and of
the other sign (AR6 WG1, §5.2.3.1).

**A steady state** links the three: the burden $B$ is the flux $\Phi$
times the lifetime $\tau$. Per unit of surface, a burden of mass $m$
per m² weighs $m\,g$, and the app's partial pressures are weights (a
gas's mass share times the surface pressure), so

$$p_\text{gas} = \Phi_A\,\tau\,g,$$

with $\Phi_A$ the mass flux per m² and $g$ the surface gravity. On Earth
(derived here, as the code will compute it): Prather et al.'s
202 Tg/yr over Earth's area from `EARTH_EQUATORIAL_RADIUS` (6378.1 km,
5.112e14 m²) is 1.25e-11 kg m⁻² s⁻¹; times 9.5 years and the nominal
gravity of that radius and `EARTH_MASS_PARAMETER` (9.798 m/s²) it gives
3.68e-7 bar of CH₄ by weight, 673 ppb by moles in the app's reference
air (0.746 bar of N₂, 0.228 of O₂, 0.01294 of argon, dry). N₂O: 9.1
Tg N/yr as N₂O times 142 years gives 3.89e-7 bar, 259 ppb.

These are column means: the gas spread uniformly through the air.
Prather et al.'s 700 and 270 ppb are tropospheric means, and the
stratosphere holds less. Their fill factors (burden over a column
uniformly at the tropospheric mean) are 0.971 for CH₄ and 0.969 for N₂O
(Prather et al. 2012, supplement, Table A2, as the review of this page
read them; the supplement was not reached here). Like for like
(derived): 700 ± 25 × 0.971 = 680 ± 24 ppb and 270 ± 7 × 0.969 =
262 ± 7 ppb against the law's 673 and 259; or the law's values over
the fill factors, 693 and 268 ppb, against 700 and 270.

### Methane and N₂O against the star

Photochemical models hold a biosphere's surface fluxes fixed at those
that give Earth its measured gases around the Sun, then move the planet
to another star at the same total light. All the published grids come
from one family of one-dimensional codes (Kasting's photochemistry, in
EXO-P, EXO-Prime and Atmos):

- **Segura et al. 2005** (M dwarfs; their model as in Segura et al.
  2003): fluxes of 9.54e14 g CH₄/yr and 1.32e13 g N₂O/yr give Earth's
  present 1.7 ppm and 0.3 ppm around the Sun. Around the active AD Leo
  and GJ 643, methane exceeds 300 ppm, "nearly a factor of 200", and N₂O
  triples to about 1 ppm. The reason: methane's sink in an oxygen air is
  O₃ + hν (λ < 310 nm) → O(¹D), O(¹D) + H₂O → 2 OH, CH₄ + OH; M dwarfs
  send little light between 200 and 300 nm, so OH falls "by a factor
  of 10⁵" (about 1 against 10⁶ cm⁻³: the two numbers, as printed,
  make 10⁶) and methane lasts about 200 years instead of 10 to 12.
  Around quiescent model M dwarfs, methane "runs away": no steady state
  exists at Earth's flux, and the authors fix it at 500 ppm instead.
- **Rugheimer et al. 2013** (F0V to K7V, 4250 to 7000 K): fluxes of
  5.3e14 g CH₄/yr and 7.9e12 g N₂O/yr give 1.6 ppm and 0.3 ppm around
  the Sun. Methane rises as the host cools, N₂O changes little below
  20 km; both are shown only in their Fig. 7.
- **Rugheimer et al. 2015a** (M0 to M9, active and inactive, Table 3):
  at Earth's fluxes (5.65e14 g CH₄/yr, 7.99e12 g N₂O/yr), methane holds
  330 to 910 ppm around active M0 to M4 dwarfs and 490 to 650 ppm around
  inactive M0 to M2; past that it runs away and is fixed at 1000 ppm,
  "the last stable run". N₂O holds 0.70 to 3.5 ppm around active dwarfs
  and 34 ppm to a cap of 1.5 % around inactive ones.
- **Rugheimer & Kaltenegger 2018** (Table 2) tabulate both gases at the
  surface for nine hosts and four epochs (below).
- **Schwieterman et al. 2022** (Atmos, Sun and FGKM hosts, N₂O fluxes of
  0.01 to 100 Tmol/yr at 1 % to 100 % of present O₂, their Figs. 4 to
  7): Earth's modern flux, about 0.4 Tmol/yr, gives about 330 ppb around
  the Sun and about 3 ppm around the late K dwarf HD 85512 and the M0
  dwarf K2-3, ten times more. At 10 to 100 Tmol/yr, they hold 5 to
  50 ppm around the Sun and 90 to 1600 ppm around these two hosts: it is
  for those fluxes that the paper writes "about two orders of
  magnitude".
  Photolysis is the main sink around every host; O(¹D) is a strong
  second one around F and G stars only. Late K dwarfs sit in a "sweet
  spot" of least loss. Around M dwarfs the loss is set by the
  far-ultraviolet continuum, not by Lyman α, so models that scale an
  active star's spectrum or drop the chromosphere (the "inactive" models
  above) disagree with observed spectra. N₂O rises with O₂, "smooth and
  largely linear in log-log space", because O₂ shields it.

Rugheimer & Kaltenegger 2018's surface abundances at Earth's modern
fluxes (Table 2):

| Host | F0V | F7V | Sun | G8V | K2V | K7V | M1V | M3V | M8V |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CH₄ (ppm) | 3.58 | 2.46 | 1.73 | 420 | 453 | 125 | 371 | 145 | 1000 (fixed) |
| N₂O (ppm) | 0.163 | 0.298 | 0.303 | 0.308 | 0.310 | 0.840 | 0.704 | 0.818 | 3.09 |
| CH₄ ratio to the Sun | 2.07 | 1.42 | 1 | 243 | 262 | 72.3 | 214 | 83.8 | 578 |
| N₂O ratio to the Sun | 0.538 | 0.983 | 1 | 1.02 | 1.02 | 2.77 | 2.32 | 2.70 | 10.2 |

The ratios are derived here. The hosts run from 7000 to 2400 K and are
those of Rugheimer et al. 2013 and 2015a (IUE or HST ultraviolet joined
to model photospheres); the G8V of 2013 is τ Ceti ([Fe/H] −0.52). The
paper calls its M1V, M3V and M8V the active models of Rugheimer et al.
2015a, yet its M3V holds 145 ppm of methane where 2015a's Table 3 gives
570 ppm for the active M3 (M1: 371 against 370; M8: 1000 in both; N₂O
agrees for all three). Rugheimer et al. 2015a's active models (Table 3,
fluxes giving 1.6 ppm and 0.30 ppm around their Sun):

| Active model | M0 | M1 | M2 | M3 | M4 | M5 | M6 | M7 | M8 | M9 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| $T_\text{eff}$ (K) | 3800 | 3600 | 3400 | 3200 | 3100 | 2800 | 2600 | 2500 | 2400 | 2300 |
| CH₄ (ppm) | 330 | 370 | 450 | 570 | 910 | 1000 | 1000 (fixed) | 1000 (fixed) | 1000 (fixed) | 1000 (fixed) |
| N₂O (ppm) | 0.70 | 0.70 | 0.76 | 0.82 | 0.93 | 0.98 | 1.7 | 3.0 | 3.1 | 3.5 |

Methane rises monotonically along this grid until the authors fix it at
1000 ppm, "the last stable run" of an active M5: past M5, Earth's flux
has no steady state, and 1000 ppm needs only 52 % to 21 % of it.

What the grids share: at a given flux, both gases rise as the host's
near ultraviolet falls, methane by two to three orders of magnitude and
N₂O by up to one, and methane has a threshold below which no steady
state exists. What they do not share: the threshold's place. Between the
Sun (5772 K) and τ Ceti (5500 K) methane jumps by 243 in Rugheimer &
Kaltenegger 2018, a jump set by the measured ultraviolet of one star;
their F0V holds more methane than their Sun, against the trend Rugheimer
et al. 2013 describe; and the models' fluxes differ (9.54e14 against
5.3e14 g/yr for the same 1.6 to 1.7 ppm), which is why only their
ratios carry over.

### Methane and N₂O against oxygen and the stage of life

- **Archean (no O₂).** Catling & Zahnle (2020, read in full) put
  Archean methane at 10² to 10⁴ times today's: anoxic air "could hold
  1000 s of parts per million by volume of CH₄ if a microbial flux of
  CH₄ was comparable to today's" (citing Catling et al. 2001, Kharecha
  et al. 2005, Ozaki et al. 2018). Constraints (their Table 1): more
  than 20 ppm before 2.4 Ga (needed for the sulfur isotope signal),
  more than about 5000 ppm near 3.5 Ga (xenon isotopes, preliminary).
  Methane levels of 10³ to 10⁴ ppm warm by 10 to 15 K; past a CH₄:CO₂
  ratio of about 0.1 a cooling organic haze forms. N₂O was "a few
  ppbv", from lightning only: without nitrate, denitrification had
  nothing to work on.
- **Proterozoic (O₂ low).** Olson, Reinhard & Lyons (2016, read in full) find,
  with an ocean model, that sulfate-driven anaerobic oxidation of methane in the
  sea limits the air to under about 10 ppm "for the second half of Earth history
  regardless of atmospheric pO₂"; above 100 ppm needs O₂ of 1 % to 10 % of
  today's and sulfate far below 1 mM. As O₂ rises from 0.1 % to 1 % of today's,
  the flux falls by more than 20 % and methane rises by more than 200 %: ozone
  shields methane better than oxygen destroys it. Their sensitivity runs give
  2.5 to 37.4 ppm over O₂ of 10⁻⁵ to 10⁻¹ of today's (Table S1). Rugheimer &
  Kaltenegger 2018 instead fix the Proterozoic fluxes from earlier fixed mixing
  ratios and hold 1650 ppm at 2.0 Ga (1 % of today's O₂) and 415 ppm at 0.8 Ga
  (10 %) around the Sun, with N₂O at 8.4 and 92 ppb: two orders of magnitude
  above Olson et al. for methane. A Proterozoic N₂O greenhouse from
  copper-starved denitrification (Buick 2007; Roberson et al. 2011) is known
  here only through Schwieterman et al. 2018 and Olson et al. 2016.
- **Modern (O₂ near today's).** The values of the previous sections.
- **Without life (a false positive).** Schwieterman et al. 2022 bound
  abiotic N₂O from lightning: the largest NOₓ flux Wong et al. (2017)
  predict on an early Earth (1 bar of CO₂), 6.5e8 molecules cm⁻² s⁻¹
  (about 0.2 Tmol/yr), would hold 1 to 100 ppb of N₂O around every host
  if all of it became N₂O, even at 1 % of today's O₂, and less without
  O₂'s shielding.

### What we do not know

- **A lifetime against the star's light.** No published formula gives
  methane's or N₂O's lifetime from a star's spectrum; only model runs at
  fixed fluxes, around nine to sixteen particular stars, from one family
  of codes.
- **Where methane's threshold sits.** Between the Sun and a G8 dwarf in
  one grid; set by the measured ultraviolet of the stars chosen, which
  the app's photospheric spectra do not carry (no chromosphere).
- **The response at pre-industrial amounts.** The grids hold the fluxes
  that give the present day's 1.6 to 1.7 ppm; whether the ratios hold at
  0.73 ppm is not tested (the methane feedback makes the response
  non-linear).
- **Another biosphere's flux.** Nothing gives a methane or N₂O flux from
  what a generator knows of a planet (its ocean, land, temperature):
  Earth's natural flux is the only measured one.
- **The Proterozoic.** Two families of models differ by a factor of
  about a hundred.
- **Which abundance the column should hold.** The law gives a column mean (673
  ppb of CH₄ on the app's Earth); the troposphere holds more (its mean is the
  column's over the fill factor, 693 ppb), and AR6 gives 729 ppb at the surface
  in 1750. The column holds one uniform value; which mean it should be was not
  tested radiatively. The future tier-1 test should compare the law's column
  mean with Prather et al.'s tropospheric mean times its fill factor (680 ± 24
  ppb of CH₄, 262 ± 7 of N₂O), the same source as its flux and lifetime: a check
  of units and of the fill factor, since Prather et al. derived the flux from
  that abundance and lifetime; AR6's 729.2 ppb is a surface value on a corrected
  scale.

## In the app

### What it does now

- **The `Modern` methane law is in `science`, not yet wired.**
  `science::life_gases::modern_methane(stars, gravity)` gives the
  methane of a `Modern` air, bar by weight (the column reads it as a
  mole share, issue #749): $p = \Phi_A \tau g$ with
  Prather et al. 2012's 202 Tg/yr spread over Earth's 5.112e14 m² and
  their 9.5 years times the host factor of each star (calibration 1,
  `methane_host_factor`), the stars' loss rates added by their share of
  the light (calibration 2, `methane_lifetime`). Around the Sun on
  Earth's 9.798 m/s² it gives 3.678e-7 bar, 672.8 ppb by moles in the
  reference air, inside Prather et al.'s 680 ± 24 like for like
  (`modern_methane_around_the_sun_is_prathers_column`). Planet PR 2d-4
  calls it from its `Modern` stage (`life::oxygenate`); until then no
  engine reads it, and the reference bodies' airs are unchanged.
- **No biogenic gas in the engines yet.** `science::life::oxygenate`
  (planet PR 2d-4) gives a `Modern` air O₂ at `O2_PER_N2` of its N₂ and
  removes its methane; an `Oxidising` air loses its methane and keeps no
  O₂ (#717); `Anoxic` and `Lifeless` airs pass through. N₂O is not a gas
  of the model (`PartialPressures` holds N₂, CO₂, H₂O, CH₄, NH₃, SO₂, O₂
  and Ar; #737).
- **Abiotic methane only.** Outgassed methane survives only under an
  equilibrium temperature of 150 K (`photochemistry::METHANE_SURVIVES`,
  a calibration) and is destroyed by Lyman α photolysis
  (`methane_left`).
- **Methane's lines are in the column (#704).** The infrared column
  absorbs in methane's HITRAN lines (broadened by air and by methane
  itself, `opacity.bin`'s sets 4 and 5) on top of its N₂–CH₄ and
  CH₄–CH₄ pairs (`radiation::spectrum::PAIRS`), the gases overlapped at
  random. The grey stand-in that acted on any air holding methane
  (`CH4_OPACITY`) went with study 36's first pull request, so no path
  counts methane twice: the column reads one `ch4` per level, the lines
  and the pairs each once, and the test airs that hold none
  (`EARTH_MEASURED_AIR`, `EARTH_DRY_AIR`) stay without methane; the
  CERES test carries CERES's 1803 ppb and dropped methane's 1.5 W/m²
  from its `MINOR_GASES_W_M2`.
- **Weights read as moles (a departure).** The chain's partial pressures
  are weights; the column reads $p_i/p$ as a share of the moles. The
  law's 3.678e-7 bar of methane is 672.8 ppb by moles in Earth's air,
  but the column would read it as 373 ppb ($16.04/28.96$ of it), so
  methane's lines would take about half of what they should once 2d-4
  wires it. The same holds for argon (radiative-transfer.md, "Gases that
  only broaden") and for Titan's chain methane. Not fixed here: it
  changes every air the column reads (issue #749).

### Where the code's docs disagree

- The doc of `photochemistry::METHANE_SURVIVES` says water vapour
  reaching the upper air makes, by its photolysis, the OH that oxidises
  methane. On an oxygen Earth the OH that destroys methane is made in
  the troposphere from ozone's photolysis to O(¹D) and its reaction with
  water (Segura et al. 2005).
- The doc of `science::life::oxygenate` says OH destroys methane
  "within about nine years in an oxygenated air": today's lifetime, 9.1
  ± 0.9 years; the 1750 air's is 9.5 ± 1.3 years (Prather et al. 2012),
  the one the law uses.

### What a law can read

| Input | In the app | Used by the literature |
| --- | --- | --- |
| Stage of life | `Stage`: `Lifeless`, `Anoxic`, `Oxidising`, `Modern` | epochs (3.9, 2.0, 0.8 Ga, modern) |
| O₂ | none, or `O2_PER_N2` of the N₂ | 1 % to 100 % of today's |
| Host temperature | each star's effective temperature | F0V to M9V grids |
| Host spectrum | photospheric shares from 0.2 µm (`stellar-spectra.tsv`), no chromosphere | IUE and HST spectra, chromosphere included |
| Host ultraviolet history | Lyman α share against age (`xuv::History`) | Lyman α and the far-ultraviolet continuum |
| Metallicity | the star's [Fe/H] | uncontrolled: Rugheimer & Kaltenegger 2018's F to K hosts span −0.52 to 0.00; the M models give none (solar-metallicity photospheres) |
| Gravity, surface pressure | yes | Earth's (1 bar) |
| Biological productivity, land, wetlands | no | Earth's fluxes |

### Options for the law

1. **Status quo.** No biogenic gas: Earth misses 2.0 to 2.8 W/m² (study
   36).
2. **Earth's measured abundance times a host ratio.** The mixing ratio
   is 729.2 ppb (or 270.1 ppb) times the host's ratio to the Sun. Simple,
   but it imposes Earth's abundance on every living planet whatever its
   gravity or air, the "1800 ppb" the user ruled out in another form.
3. **A steady state of a flux and a lifetime (recommended, decided).**
   $p = \Phi_A\,\tau\,g$ by weight, with $\Phi_A$ Earth's pre-industrial
   natural flux per m² (Prather et al. 2012, `Source:`), $\tau$ Earth's
   pre-industrial lifetime (same, `Source:`) times the host factor of
   calibration 1 below. Earth around the Sun comes out at 673 ppb of
   methane and 259 ppb of N₂O in the app's reference air (derived),
   inside Prather et al.'s 1σ like for like (above); gravity and the
   stage enter, and the flux stays a property of the biosphere, the
   lifetime one of the air and the star.
4. **Option 3 with the methane feedback.** The burden grows as the flux
   to the power $f$ = 1.30 (AR6). Needed only once a flux other than
   Earth's exists; the app has none.
5. **A lifetime from the app's own ultraviolet.** Fit the host factor to
   each star's 200 to 310 nm light in `stellar-spectra.tsv`. No published
   law; the fit would rest on model runs made with observed spectra
   (chromosphere included) and be applied to photospheric ones. Not
   recommended now.

### By stage (recommended, decided)

- **`Modern`:** option 3, with methane first (its lines land first) and
  N₂O with its lines (decided 2026-11-26, below).
- **`Oxidising`:** no CH₄ and no N₂O, as now. The app gives this air no
  O₂ (#717), where a photochemical lifetime is not defined, and the
  literature disagrees by a factor of a hundred (Olson et al. 2016
  against Rugheimer & Kaltenegger 2018). To revisit with #717 (#736).
- **`Anoxic`:** unchanged for now. Archean biogenic methane (10² to 10⁴
  times today's) needs its own study: the app's `METHANE_SURVIVES`
  removes methane from every air warmer than 150 K, an oxygen-free one
  included, against Catling & Zahnle 2020 (#735).
- **`Lifeless`:** none.

## Decisions (user, 2026-11-26)

The user took every recommendation of the study, then ruled on two
points after its review:

- **`Modern`: option 3.** The partial pressure by weight is Earth's 1750
  natural flux per m² times its 1750 lifetime times the host factor
  times the planet's gravity. The flux and the lifetime are `Source:`
  (Prather et al. 2012, measured numbers). Applying Earth's flux per m²
  to every `Modern` planet is a departure (#738), not a definition. No
  methane feedback for now (#738).
- **The host factor** (calibration 1) takes Rugheimer & Kaltenegger
  2018's ratios for F, G and K hosts and Rugheimer et al. 2015a's active
  M models for M dwarfs: one family of runs, monotonic among the M
  dwarfs. Interpolated in the logarithm, held past the ends, with its
  cliff and its faults written down.
- **Methane first, with its lines** (study 36's second pull request).
  N₂O's lines joined the same rebuild of the table (decided 2026-11-27:
  methane's tests need the N₂O of the line-by-line codes' air); N₂O as
  a gas, its law and the `FORMAT` change come later (#737).
- **`Oxidising`** keeps no CH₄ and no N₂O until #717 gives it O₂; a
  study follows then (#736). **`Anoxic`** gets its own study (#735).
- **Several stars** (calibration 2): the loss rates add, each star
  weighted by its share of the planet's light.
- **Later:** fluxes derived from the planet (land, wetlands,
  temperature, productivity) and the methane feedback (#738).

## Calibrations

Decided 2026-11-26. Methane's are in the code since #704
(`science::life_gases`); N₂O's wait for N₂O as a gas (#737; its
lines are in the column's table since #704).

### Calibration 1: the host factor

What: the lifetime of each gas around a host is Earth's times the ratio
of a model's surface abundance around that host to the same model's
around the Sun, at Earth's fluxes:

| Host | F0V | F7V | Sun | G8V | K2V | K7V | M1 | M3 | M8 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| $T_\text{eff}$ (K) | 7000 | 6250 | 5772 | 5500 | 5000 | 4250 | 3600 | 3200 | 2400 |
| CH₄ ratio | 2.07 | 1.42 | 1 | 243 | 262 | 72.3 | 231 | 356 | 625, a lower bound |
| N₂O ratio | 0.538 | 0.983 | 1 | 1.02 | 1.02 | 2.77 | 2.33 | 2.73 | 10.3 |
| Source | RK2018 | RK2018 | RK2018 | RK2018 | RK2018 | RK2018 | 2015a | 2015a | 2015a |

RK2018 is Rugheimer & Kaltenegger 2018, Table 2, modern epoch, over
their Sun's 1.73 and 0.303 ppm; 2015a is Rugheimer et al. 2015a, Table
3, active models, over the 1.6 and 0.30 ppm their fluxes give around
their Sun (ratios derived here). The Sun sits at the app's 5772 K;
the M temperatures are 2015a's Table 1. Between grid points the ratio
is interpolated in the logarithm of the ratio, linear in temperature,
and held past 7000 and 2400 K. A ratio of abundances stands for a ratio
of lifetimes because the flux is the same in every run. On the app's
Earth (derived): about 49 ppm of methane around a K7 host, 420 ppm or
more around an M8.

Searched and not found: a law of either gas's lifetime against the
host's ultraviolet; any model outside the Kasting family around a host
other than the Sun; runs at pre-industrial fluxes.

In the code (methane): `life_gases::methane_host_factor`, from the
abundances `RK2018_CH4_PPM` and `RUGHEIMER_2015A_CH4_PPM` over their
Suns' 1.73 and 1.6 ppm (`Calibration:` and `Source:`), tested at every
grid point and past both ends (`host_factor_is_the_grids_ratio_at_its_points`).

Its known faults:

- **The cliff:** methane's 243-fold step between the Sun (5772 K) and
  τ Ceti (5500 K), set by one star's measured ultraviolet.
- **Steps that are not monotonic among F to K hosts:** F0V above F7V
  and the Sun; K2V's 262 above K7V's 72.3, then 231 at M1.
- **The M8 value is a lower bound:** the authors fixed methane at
  1000 ppm, which needs 24 % of Earth's flux; at Earth's whole flux no
  steady state exists, so the true ratio is higher than 625.
- **RK2018's own M column is not used.** Its M1V, M3V and M8V
  (214, 83.8, 578) zigzag, and its M3V's 145 ppm disagrees with the
  570 ppm 2015a give for the same active M3. 2015a's grid avoids that
  zigzag.
- **Two Suns.** RK2018's Sun holds 1.73 ppm of methane, 2015a's 1.6.
  Each ratio takes its own paper's Sun; over RK2018's Sun the M ratios
  would be 8 % lower (214, 329, 578), M1's then equal to RK2018's own.
- **RK2018's GJ 581, GJ 832 and GJ 1214 are not used.** They are single
  observed stars (HST ultraviolet), each with its own activity, at
  temperatures inside the grid (3498, 3620 and 3250 K); held against the
  grid they would zigzag again (GJ 832 at 3620 K: 546 ppm against M1's
  370 at 3600 K; GJ 1214 at 3250 K: 1630 against M3's 570 at 3200 K).
- **Active against quiescent M dwarfs.** These ratios take active M
  models; the ozone column (calibration 1 of [ozone.md](ozone.md)) takes
  Segura et al. 2005's quiescent model M dwarfs. The two gases thus see
  two different M dwarfs (#739).

### Calibration 2: several stars

Loss rates add: $1/\tau = \sum_i w_i/\tau_i$, each star's factor
weighted by its share of the planet's light. The grids hold one star at
Earth's total light; no source treats two hosts. In the code:
`methane_lifetime` (stars whose flux is not positive count for nothing;
with none, Earth's 9.5 years), tested by
`several_stars_add_their_loss_rates_by_light`.

## Departures

- **Earth's flux on every `Modern` planet (#738).** Earth's 1750
  natural fluxes per m² (Prather et al. 2012, `Source:`) are applied to
  every `Modern` planet, as `O2_PER_N2` carries Earth's O₂ by its N₂.
  No source gives a biosphere's flux from what the app knows of a
  planet.
- **No methane feedback (#738)**, since no flux other than Earth's
  exists yet.
- **No CH₄ or N₂O in `Oxidising` (#736) or biogenic in `Anoxic`
  (#735) airs; no N₂O in any air until N₂O is a gas (#737), though
  the column's table holds its lines.**
- **M dwarfs' activity (#739):** active here, quiescent for ozone.

## Effect on Earth

Derived from study 36's values. Its clear-sky removals, CH₄ 0.84 to
1.26 W/m² and N₂O 1.28 to 1.68, are for RFMIP's pre-industrial 808 and
273 ppb; rescaled here to the law's 673 and 259 ppb with Etminan et
al. 2016's square-root term alone (0.043 (√808 − √673) = 0.107 W/m² for
CH₄, 0.117 (√273 − √259) = 0.049 for N₂O; Table 1 as study 36 read it,
the overlap and mean-abundance terms left out). All-sky at the N₂
pairs' 0.83 ratio, temperature at study 32's 2.0 W/m²/K:

| Gas | Clear-sky OLR | All-sky OLR | Settled temperature at 280 ppm |
| --- | --- | --- | --- |
| CH₄, 673 ppb | −0.73 to −1.15 W/m² | −0.61 to −0.96 | +0.30 to +0.48 K |
| N₂O, 259 ppb | −1.23 to −1.63 | −1.02 to −1.35 | +0.51 to +0.68 K |
| Both | a little under −1.96 to −2.78 (their overlap) | a little under −1.63 to −2.31 | about +0.8 to +1.15 K |

Methane's sunlight at these amounts moves the Bond albedo by less than
0.0005 (study 36). The CERES test of PR #711
(`earths_clear_sky_column_is_ceres_like_for_like`) runs on a fixed test
air, not on the engine's: it reads no law. When methane's lines land,
its air can carry the measured methane of CERES's years as a test input
and its `MINOR_GASES_W_M2` drop methane's 1.5 W/m² (study 36).

## Sources

1. Buick, R. 2007, "Did the Proterozoic 'Canfield Ocean' cause a laughing
   gas greenhouse?", *Geobiology* 5, 97,
   doi:10.1111/j.1472-4669.2007.00110.x (not read; through Schwieterman et
   al. 2018).
2. Canadell, J. G. et al. 2021, "Global carbon and other biogeochemical
   cycles and feedbacks", in *Climate Change 2021: The Physical Science
   Basis* (IPCC AR6 WG1), chapter 5, doi:10.1017/9781009157896.007
   (read: §5.2.2-5.2.3, Tables 5.2 and 5.3).
3. Catling, D. C., Zahnle, K. J., McKay, C. P. 2001, "Biogenic methane,
   hydrogen escape, and the irreversible oxidation of early Earth",
   *Science* 293, 839, doi:10.1126/science.1061976 (not read; through
   Catling & Zahnle 2020).
4. Catling, D. C., Zahnle, K. J. 2020, "The Archean atmosphere", *Sci.
   Adv.* 6, eaax1420, doi:10.1126/sciadv.aax1420 (read in full, PMC7043912).
5. Etminan, M., Myhre, G., Highwood, E. J., Shine, K. P. 2016,
   *Geophys. Res. Lett.* 43, 12614, doi:10.1002/2016GL071930 (not read
   here; its Table 1 square-root terms as study 36 read them).
6. Gulev, S. K. et al. 2021, "Changing state of the climate system", IPCC
   AR6 WG1, chapter 2, doi:10.1017/9781009157896.004 (read: §2.2.3.2.2
   and §2.2.3.2.3).
7. Kharecha, P., Kasting, J. F., Siefert, J. 2005, *Geobiology* 3, 53,
   doi:10.1111/j.1472-4669.2005.00049.x (not reached; known through
   Catling & Zahnle 2020 and Rugheimer & Kaltenegger 2018).
8. Olson, S. L., Reinhard, C. T., Lyons, T. W. 2016, "Limited role for
   methane in the mid-Proterozoic greenhouse", *PNAS* 113, 11447,
   doi:10.1073/pnas.1608549113 (read in full with its supporting text,
   PMC5068276).
9. Ozaki, K., Thompson, K. J., Simister, R. L., Crowe, S. A., Reinhard,
   C. T. 2019, "Anoxygenic photosynthesis and the delayed oxygenation of
   Earth's atmosphere", *Nat. Commun.* 10, 3026,
   doi:10.1038/s41467-019-10872-z (read; it gives no methane abundance).
10. Ozaki, K., Tajika, E., Hong, P. K., Nakagawa, Y., Reinhard, C. T.
    2018, "Effects of primitive photosynthesis on Earth's early climate
    system", *Nat. Geosci.* 11, 55, doi:10.1038/s41561-017-0031-2 (not
    reached; through Catling & Zahnle 2020).
11. Prather, M. J., Holmes, C. D., Hsu, J. 2012, "Reactive greenhouse gas
    scenarios: systematic exploration of uncertainties and the role of
    atmospheric chemistry", *Geophys. Res. Lett.* 39, L09803,
    doi:10.1029/2012GL051440 (read, eScholarship copy, Table 1; its
    supplement not reached: its Table A2 fill factors as the review of
    this page read them).
12. Prather, M. J. et al. 2015, *J. Geophys. Res. Atmos.* 120, 5693,
    doi:10.1002/2015JD023267 (not reached; its 116 ± 9 yr as AR6 gives it).
13. Prather, M. J., Froidevaux, L., Livesey, N. J. 2023, "Observed changes
    in stratospheric circulation: decreasing lifetime of N₂O, 2005-2021",
    *Atmos. Chem. Phys.* 23, 843, doi:10.5194/acp-23-843-2023 (not read;
    through Tian et al. 2024).
14. Roberson, A. L., Roadt, J., Halevy, I., Kasting, J. F. 2011,
    "Greenhouse warming by nitrous oxide and methane in the Proterozoic
    Eon", *Geobiology* 9, 313, doi:10.1111/j.1472-4669.2011.00286.x (not
    read; through Schwieterman et al. 2018).
15. Rugheimer, S., Kaltenegger, L., Zsom, A., Segura, A., Sasselov, D.
    2013, *Astrobiology* 13, 251, doi:10.1089/ast.2012.0888,
    arXiv:1212.2638 (read; §2.4 and §3).
16. Rugheimer, S., Kaltenegger, L., Segura, A., Linsky, J., Mohanty, S.
    2015a, *ApJ* 809, 57, doi:10.1088/0004-637X/809/1/57,
    arXiv:1506.07202 (read; §2.3, Tables 1 and 3).
17. Rugheimer, S., Kaltenegger, L. 2018, "Spectra of Earth-like planets
    through geological evolution around FGKM stars", *ApJ* 854, 19,
    doi:10.3847/1538-4357/aaa47a, arXiv:1712.10027 (read; §2.2, Tables 1
    and 2).
18. Saunois, M. et al. 2020, "The global methane budget 2000-2017",
    *Earth Syst. Sci. Data* 12, 1561, doi:10.5194/essd-12-1561-2020
    (read: §1, §3.3.5).
19. Schwieterman, E. W. et al. 2018, "Exoplanet biosignatures: a review of
    remotely detectable signs of life", *Astrobiology* 18, 663,
    doi:10.1089/ast.2017.1729, arXiv:1705.05791 (read: §4.2.3-4.2.4).
20. Schwieterman, E. W. et al. 2022, "Evaluating the plausible range of
    N₂O biosignatures on exo-Earths", *ApJ* 937, 109,
    doi:10.3847/1538-4357/ac8cfb, arXiv:2210.01669 (read: abstract, model
    and results text; its figures not read numerically).
21. Segura, A. et al. 2003, *Astrobiology* 3, 689,
    doi:10.1089/153110703322736024 (not reached; its fluxes through
    Segura et al. 2005).
22. Segura, A. et al. 2005, "Biosignatures from Earth-like planets around
    M dwarfs", *Astrobiology* 5, 706, doi:10.1089/ast.2005.5.706,
    arXiv:astro-ph/0510224 (read in full).
23. Szopa, S. et al. 2021, "Short-lived climate forcers", IPCC AR6 WG1,
    chapter 6, doi:10.1017/9781009157896.008 (read: §6.3.1, Table 6.2).
24. Tian, H. et al. 2024, "Global nitrous oxide budget (1980-2020)",
    *Earth Syst. Sci. Data* 16, 2543, doi:10.5194/essd-16-2543-2024
    (read: abstract, units, the natural soil baseline, the lifetime
    trend). Tian, H. et al. 2020,
    *Nature* 586, 248, doi:10.1038/s41586-020-2780-0 (not read; through
    AR6 Table 5.3).
25. Wong, M. L., Charnay, B. D., Gao, P., Yung, Y. L., Russell, M. J.
    2017, "Nitrogen oxides in early Earth's atmosphere as electron
    acceptors for life's emergence", *Astrobiology* 17, 975,
    doi:10.1089/ast.2016.1473 (not read; through Schwieterman et al. 2022).

## See also

- [How much ozone does an oxygen air hold, and what light does it
  take?](ozone.md): ozone's host ratios, from Rugheimer et al. 2013 and
  Segura et al. 2005's quiescent M dwarfs, not this page's table (#739).
- [Planet atmospheres, climate and surface](planet-atmospheres.md): the
  life stages and the abiotic methane.
- [How does a planet's air set its outgoing heat and its
  albedo?](radiative-transfer.md): the column the gases will absorb in.
