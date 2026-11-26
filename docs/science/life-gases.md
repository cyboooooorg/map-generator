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
more methane: published models of an Earth around K and M dwarfs hold
50 to 1000 ppm, a hundred to a thousand times Earth's. N₂O, split by
shorter ultraviolet, rises less: two to ten times. It also depends on
the oxygen and on the stage of life: a young, oxygen-free Earth held a
hundred to ten thousand times today's methane and almost no N₂O.

No published formula gives either gas against the star. The app can
build Earth's values from a measured flux and a measured lifetime, and
carry them to other hosts only through the ratios of one family of
photochemical models: a calibration, like the ozone column's.

## The science

### Earth's pre-industrial methane and N₂O

**The abundances.** Ice cores from Antarctica and Greenland give a
global mean of 729.2 ± 9.4 ppb of CH₄ in 1750 and 807.6 ± 13.8 ppb in
1850, and 270.1 ± 6.0 ppb of N₂O in 1750 and 272.1 ± 5.7 ppb in 1850
(IPCC AR6 WG1, Gulev et al. 2021, §2.2.3.2.2-3). Between 0 and 1850 CE
methane varied between 625 and 807 ppb; N₂O by 5 to 10 ppb over
centuries. In 2019 they were 1866.3 ± 3.3 ppb and 332.1 ± 0.4 ppb (same
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
from Tian et al. 2020). Natural soils alone emitted 6.4 (3.9 to 8.6)
Tg N/yr in the 1850s in the models of Tian et al. (2024).

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
AR6 assesses $f$ = 1.30 ± 0.07, so a burden grows locally as the flux
to the power 1.3 (AR6 WG1, §6.3.1). N₂O's own feedback is small and of
the other sign (AR6 WG1, §5.2.3.1).

**A steady state** links the three: the burden $B$ is the flux $\Phi$
times the lifetime $\tau$. Per unit of surface, a burden of mass $m$
per m² weighs $m\,g$, and the app's partial pressures are weights (a
gas's mass share times the surface pressure), so

$$p_\text{gas} = \Phi_A\,\tau\,g,$$

with $\Phi_A$ the mass flux per m² and $g$ the surface gravity. On Earth
(derived here): Prather et al.'s 202 Tg/yr over Earth's 5.1e14 m² is
1.25e-11 kg m⁻² s⁻¹ (4.7e10 molecules cm⁻² s⁻¹); times 9.5 years and
9.81 m/s² it gives 3.69e-7 bar of CH₄ by weight, 681 ppb by moles in
the app's reference air (0.746 bar of N₂ and 0.228 of O₂) and 657 ppb
in a 1.013 bar air. N₂O: 9.1 Tg N/yr as N₂O times 142 years gives
3.90e-7 bar, 263 ppb in the app's air. The step from Prather et al.'s
tropospheric 700 ppb is their fill factor: the column holds its gases
uniformly mixed.

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
  send little light between 200 and 300 nm, so OH falls by 10⁵ (about 1
  against 10⁶ cm⁻³) and methane lasts about 200 years instead of 10 to
  12. Around quiescent model M dwarfs, methane "runs away": no steady
  state exists at Earth's flux, and the authors fix it at 500 ppm
  instead.
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
  dwarf K2-3, "about two orders of magnitude" more for a given flux.
  Photolysis is the main sink around every host; O(¹D) is a strong
  second one around F and G stars only. Late K dwarfs sit in a "sweet
  spot" of least loss. Around M dwarfs the loss is set by the
  far-ultraviolet continuum, not by Lyman α, so models that scale an
  active star's spectrum or drop the chromosphere (the "inactive" models
  above) disagree with observed spectra. N₂O rises with O₂, "smooth and
  largely linear in log-log space", because O₂ shields it.

Rugheimer & Kaltenegger 2018's surface abundances at Earth's modern
fluxes:

| Host | F0V | F7V | Sun | G8V | K2V | K7V | M1V | M3V | M8V |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CH₄ (ppm) | 3.58 | 2.46 | 1.73 | 420 | 453 | 125 | 371 | 145 | 1000 (fixed) |
| N₂O (ppm) | 0.163 | 0.298 | 0.303 | 0.308 | 0.310 | 0.840 | 0.704 | 0.818 | 3.09 |
| CH₄ ratio to the Sun | 2.07 | 1.42 | 1 | 243 | 262 | 72.3 | 214 | 83.8 | 578 |
| N₂O ratio to the Sun | 0.538 | 0.983 | 1 | 1.02 | 1.02 | 2.77 | 2.32 | 2.70 | 10.2 |

The ratios are derived here. The hosts run from 7000 to 2400 K and are
those of Rugheimer et al. 2013 and 2015a (IUE or HST ultraviolet joined
to model photospheres); the M1V, M3V and M8V are the active models of
2015a, and the G8V of 2013 is τ Ceti ([Fe/H] −0.52).

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
- **Proterozoic (O₂ low).** Olson, Reinhard & Lyons (2016, read in full)
  find, with an ocean model, that sulfate-driven anaerobic oxidation of
  methane in the sea limits the air to under about 10 ppm "for the
  second half of Earth history regardless of atmospheric pO₂"; above
  100 ppm needs O₂ of 1 % to 10 % of today's and sulfate far below
  1 mM. As O₂ rises from 0.1 % to 1 % of today's, the flux falls by 20 %
  and methane rises by 200 %: ozone shields methane better than oxygen
  destroys it. Their sensitivity runs give 2.5 to 37.4 ppm over O₂ of
  10⁻⁵ to 10⁻¹ of today's (Table S1). Rugheimer & Kaltenegger 2018
  instead fix the Proterozoic fluxes from earlier fixed mixing ratios
  and hold 1650 ppm at 2.0 Ga (1 % of today's O₂) and 415 ppm at 0.8 Ga
  (10 %) around the Sun, with N₂O at 8.4 and 92 ppb: two orders of
  magnitude above Olson et al. for methane. Schwieterman et al. 2022
  find 1 to 100 ppb of N₂O at 1 % of today's O₂ if all of a
  denitrification flux of 0.2 Tmol/yr ended as N₂O. A
  Proterozoic N₂O greenhouse from copper-starved denitrification (Buick
  2007; Roberson et al. 2011) is known here only through Schwieterman et
  al. 2018 and Olson et al. 2016.
- **Modern (O₂ near today's).** The values of the previous sections.

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

## In the app

### What it does now

- **No biogenic gas.** `science::life::oxygenate` (planet PR 2d-4) gives
  a `Modern` air O₂ at `O2_PER_N2` of its N₂ and removes its methane; an
  `Oxidising` air loses its methane and keeps no O₂ (#717); `Anoxic` and
  `Lifeless` airs pass through. N₂O is not a gas of the model
  (`PartialPressures` holds N₂, CO₂, H₂O, CH₄, NH₃, SO₂ and O₂).
- **Abiotic methane only.** Outgassed methane survives only under an
  equilibrium temperature of 150 K (`photochemistry::METHANE_SURVIVES`,
  a calibration) and is destroyed by Lyman α photolysis
  (`methane_left`).
- **No absorption yet.** The infrared column has no methane or N₂O
  lines (study 36 adds methane's first, N₂O's later).

### What a law can read

| Input | In the app | Used by the literature |
| --- | --- | --- |
| Stage of life | `Stage`: `Lifeless`, `Anoxic`, `Oxidising`, `Modern` | epochs (3.9, 2.0, 0.8 Ga, modern) |
| O₂ | none, or `O2_PER_N2` of the N₂ | 1 % to 100 % of today's |
| Host temperature | each star's effective temperature | F0V to M9V grids |
| Host spectrum | photospheric shares from 0.2 µm (`stellar-spectra.tsv`), no chromosphere | IUE and HST spectra, chromosphere included |
| Host ultraviolet history | Lyman α share against age (`xuv::History`) | Lyman α and the far-ultraviolet continuum |
| Metallicity | the star's [Fe/H] | uncontrolled (the grid stars span −0.52 to +0.21) |
| Gravity, surface pressure | yes | Earth's (1 bar) |
| Biological productivity, land, wetlands | no | Earth's fluxes |

### Options for the law

1. **Status quo.** No biogenic gas: Earth misses 2.0 to 2.8 W/m² (study
   36).
2. **Earth's measured abundance times a host ratio.** The mixing ratio
   is 729.2 ppb (or 270.1 ppb) times the host's ratio to the Sun. Simple,
   but it imposes Earth's abundance on every living planet whatever its
   gravity or air, the "1800 ppb" the user ruled out in another form.
3. **A steady state of a flux and a lifetime (recommended).**
   $p = \Phi_A\,\tau\,g$ by weight, with $\Phi_A$ Earth's pre-industrial
   natural flux per m² (Prather et al. 2012, `Source:`), $\tau$ Earth's
   pre-industrial lifetime (same, `Source:`) times the host factor of
   calibration 1 below. Earth around the Sun comes out at 681 ppb of
   methane and 263 ppb of N₂O in the app's reference air (derived), inside
   Prather et al.'s 700 ± 25 and 270 ± 7; gravity and the stage enter,
   and the flux stays a property of the biosphere, the lifetime one of
   the air and the star.
4. **Option 3 with the methane feedback.** The burden grows as the flux
   to the power $f$ = 1.30 (AR6). Needed only once a flux other than
   Earth's exists; the app has none.
5. **A lifetime from the app's own ultraviolet.** Fit the host factor to
   each star's 200 to 310 nm light in `stellar-spectra.tsv`. No published
   law; the fit would rest on model runs made with observed spectra
   (chromosphere included) and be applied to photospheric ones. Not
   recommended now.

### By stage (recommended)

- **`Modern`:** option 3, with methane first (its lines land first) and
  N₂O with its lines.
- **`Oxidising`:** no CH₄ and no N₂O, as now. The app gives this air no
  O₂ (#717), where a photochemical lifetime is not defined, and the
  literature disagrees by a factor of a hundred (Olson et al. 2016
  against Rugheimer & Kaltenegger 2018). To revisit with #717.
- **`Anoxic`:** unchanged for now. Archean biogenic methane (10² to 10⁴
  times today's) needs its own study: the app's `METHANE_SURVIVES`
  removes methane from every air warmer than 150 K, an oxygen-free one
  included, against Catling & Zahnle 2020.
- **`Lifeless`:** none.

## Calibrations

Proposed, none in the code yet.

### Calibration 1 (proposed): the host factor

What: the lifetime of each gas around a host is Earth's times the ratio
of Rugheimer & Kaltenegger 2018's surface abundance around that host to
their Sun's (Table 2, modern epoch), at their nine hosts, interpolated
in the logarithm between grid points and held past the ends. The grid's
temperatures: 7000 (F0V), 6250 (F7V), 5772 (Sun, the app's), 5500
(G8V), 5000 (K2V), 4250 (K7V), and for the M dwarfs those of Rugheimer
et al. 2015a's grid (M1V 3600 K, M3V 3200 K, M8V 2400 K). A ratio of
abundances stands for a ratio of lifetimes because the flux is the same
in every run. Searched and not found: a law of either gas's lifetime
against the host's ultraviolet; any model outside the Kasting family
around a host other than the Sun; runs at pre-industrial fluxes. Its
known faults: the 243-fold step between 5772 and 5500 K, which belongs
to τ Ceti's measured ultraviolet; F0V above the Sun for methane; the M8V
value fixed by the authors, not computed.

### Calibration 2 (proposed, or a declared scope): Earth's flux per m²

Earth's pre-industrial natural fluxes, per square metre, are carried to
every `Modern` planet, as `O2_PER_N2` carries Earth's O₂ by its N₂. No
source gives a biosphere's flux from what the app knows of a planet.

### Calibration 3 (proposed, or a declared scope): several stars

Loss rates add: $1/\tau = \sum_i w_i/\tau_i$, each star's factor
weighted by its share of the planet's light. The grids hold one star at
Earth's total light; no source treats two hosts.

## Effect on Earth

Estimated from study 36's derived values (clear-sky removal at
pre-industrial amounts: CH₄ 0.85 to 1.26 W/m², N₂O 1.3 to 1.68; all-sky
at the N₂ pairs' 0.83 ratio; temperature at study 32's 2.0 W/m²/K):

| Gas | Clear-sky OLR | All-sky OLR | Settled temperature at 280 ppm |
| --- | --- | --- | --- |
| CH₄, about 680 to 730 ppb | −0.85 to −1.26 W/m² | −0.71 to −1.05 | +0.35 to +0.52 K |
| N₂O, about 260 to 270 ppb | −1.3 to −1.68 | −1.08 to −1.39 | +0.54 to +0.70 K |
| Both, with their overlap | −2.0 to −2.8 | about −1.7 to −2.3 | +0.8 to +1.2 K |

Methane's sunlight at these amounts moves the Bond albedo by less than
0.0005 (study 36). The CERES test of PR #711
(`earths_clear_sky_column_is_ceres_like_for_like`) runs on a fixed test
air, not on the engine's: it reads no law. When methane's lines land,
its air can carry the measured methane of CERES's years as a test input
and its `MINOR_GASES_W_M2` drop methane's 1.5 W/m² (study 36).

## Sources

1. Canadell, J. G. et al. 2021, "Global carbon and other biogeochemical
   cycles and feedbacks", in *Climate Change 2021: The Physical Science
   Basis* (IPCC AR6 WG1), chapter 5, doi:10.1017/9781009157896.007
   (read: §5.2.2-5.2.3, Tables 5.2 and 5.3).
2. Catling, D. C., Zahnle, K. J. 2020, "The Archean atmosphere", *Sci.
   Adv.* 6, eaax1420, doi:10.1126/sciadv.aax1420 (read in full, PMC7043912).
3. Gulev, S. K. et al. 2021, "Changing state of the climate system", IPCC
   AR6 WG1, chapter 2, doi:10.1017/9781009157896.004 (read: §2.2.3.2-3).
4. Kharecha, P., Kasting, J. F., Siefert, J. 2005, *Geobiology* 3, 53,
   doi:10.1111/j.1472-4669.2005.00049.x (not reached; known through
   Catling & Zahnle 2020 and Rugheimer & Kaltenegger 2018).
5. Olson, S. L., Reinhard, C. T., Lyons, T. W. 2016, "Limited role for
   methane in the mid-Proterozoic greenhouse", *PNAS* 113, 11447,
   doi:10.1073/pnas.1608549113 (read in full with its supporting text,
   PMC5068276).
6. Ozaki, K., Thompson, K. J., Simister, R. L., Crowe, S. A., Reinhard,
   C. T. 2019, "Anoxygenic photosynthesis and the delayed oxygenation of
   Earth's atmosphere", *Nat. Commun.* 10, 3026,
   doi:10.1038/s41467-019-10872-z (read; it gives no methane abundance).
   Ozaki et al. 2018, *Nat. Geosci.* 11, 55, was not reached.
7. Prather, M. J., Holmes, C. D., Hsu, J. 2012, "Reactive greenhouse gas
   scenarios: systematic exploration of uncertainties and the role of
   atmospheric chemistry", *Geophys. Res. Lett.* 39, L09803,
   doi:10.1029/2012GL051440 (read, eScholarship copy, Table 1; its
   Tables S1 and S2 not read).
8. Prather, M. J. et al. 2015, *J. Geophys. Res. Atmos.* 120, 5693,
   doi:10.1002/2015JD023267 (not reached; its 116 ± 9 yr as AR6 gives it).
9. Rugheimer, S., Kaltenegger, L., Zsom, A., Segura, A., Sasselov, D.
   2013, *Astrobiology* 13, 251, doi:10.1089/ast.2012.0888,
   arXiv:1212.2638 (read; §2.4 and §3).
10. Rugheimer, S., Kaltenegger, L., Segura, A., Linsky, J., Mohanty, S.
    2015a, *ApJ* 809, 57, doi:10.1088/0004-637X/809/1/57,
    arXiv:1506.07202 (read; §2.3, Tables 1 and 3).
11. Rugheimer, S., Kaltenegger, L. 2018, "Spectra of Earth-like planets
    through geological evolution around FGKM stars", *ApJ* 854, 19,
    doi:10.3847/1538-4357/aaa47a, arXiv:1712.10027 (read; §2.2, Tables 1
    and 2).
12. Saunois, M. et al. 2020, "The global methane budget 2000-2017",
    *Earth Syst. Sci. Data* 12, 1561, doi:10.5194/essd-12-1561-2020
    (read: §1, §3.3.5).
13. Schwieterman, E. W. et al. 2018, "Exoplanet biosignatures: a review of
    remotely detectable signs of life", *Astrobiology* 18, 663,
    doi:10.1089/ast.2017.1729, arXiv:1705.05791 (read: §4.2.3-4.2.4).
14. Schwieterman, E. W. et al. 2022, "Evaluating the plausible range of
    N₂O biosignatures on exo-Earths", *ApJ* 937, 109,
    doi:10.3847/1538-4357/ac8cfb, arXiv:2210.01669 (read: abstract, model
    and results text; its figures not read numerically).
15. Segura, A. et al. 2003, *Astrobiology* 3, 689,
    doi:10.1089/153110703322736024 (not reached; its fluxes through
    Segura et al. 2005).
16. Segura, A. et al. 2005, "Biosignatures from Earth-like planets around
    M dwarfs", *Astrobiology* 5, 706, doi:10.1089/ast.2005.5.706,
    arXiv:astro-ph/0510224 (read in full).
17. Szopa, S. et al. 2021, "Short-lived climate forcers", IPCC AR6 WG1,
    chapter 6, doi:10.1017/9781009157896.008 (read: §6.3.1, Table 6.2).
18. Tian, H. et al. 2024, "Global nitrous oxide budget (1980-2020)",
    *Earth Syst. Sci. Data* 16, 2543, doi:10.5194/essd-16-2543-2024
    (read: abstract, units, the natural soil baseline, the lifetime
    trend). Tian, H. et al. 2020,
    *Nature* 586, 248, doi:10.1038/s41586-020-2780-0 (not read; through
    AR6 Table 5.3).

## See also

- [How much ozone does an oxygen air hold, and what light does it
  take?](ozone.md): the same ratios, for ozone.
- [Planet atmospheres, climate and surface](planet-atmospheres.md): the
  life stages and the abiotic methane.
- [How does a planet's air set its outgoing heat and its
  albedo?](radiative-transfer.md): the column the gases will absorb in.
