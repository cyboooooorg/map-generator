# Literature study: published bounds for the composition draws (PR 2a)

Goal: each composition draw becomes uniform (log-uniform when it spans
decades) between the published bounds of the plausible physical range, not
the 68 % interval of the observed distribution. This study proposes the
bounds, with the exact place of every number. Papers were read as arXiv
full texts unless stated. A value marked **(derived)** is arithmetic on
read numbers; a value marked **not found** was searched and not located.

Citation shorthand: "p." is the page of the arXiv PDF as extracted.

## 1. Core mass fraction (CMF) of rocky planets

**Shape**: uniform.

**Proposed bounds**: 0.0 to 0.8.

### Lower bound, 0.0

- Elkins-Tanton & Seager 2008, "Coreless Terrestrial Exoplanets",
  arXiv:0808.1908, section 2 (p. 2-3): two accretion paths give a planet
  with no metallic core, either from material oxidised before accretion
  ("chondrite meteorites that have no metallic iron but bear excess water
  in the form of OH") or from water oxidising the metal ("the iron is
  oxidized and trapped in the mantle, unable to form a core"). The paper
  is a hypothesis with plausibility arguments: CMF = 0 is the physical
  floor, not an observed value.
- Brinkman et al. 2024, arXiv:2409.08361, Table 2 (p. 6): the fitted CMF of
  real planets goes down to Kepler-78 b 0.01 +/- 0.17, Wasp-47 e
  -0.01 +/- 0.18, TOI-561 b -0.10 +/- 0.17 and HD 3167 b 0.03 +/- 0.09. A
  CMF at or below zero is inside the error bars of measured planets.
- Plotnykov & Valencia 2020, MNRAS 499, 932, arXiv:2010.06480, abstract:
  rocky planets follow 0.24 +0.33/-0.18, whose lower 68 % edge is 0.06
  **(derived)**; the distribution is not truncated at the abstract level.

### Upper bound, 0.8

- Plotnykov & Valencia 2020, abstract: "what seems to be the maximum iron
  enrichment that rocky planets attain during formation (rho0 ~ 6 and cmf
  ~ 0.8)"; Fig. 8 caption (p. 10): the most iron-rich planets of the
  sample have cmf ~ 0.8 and "may be delineating the maximum iron
  enrichment attained during planet formation".
- Mercury, 0.63 +/- 0.03 (Hauck 2013, quoted by Plotnykov & Valencia 2020
  p. 6, as recorded in `heat-and-composition.md` B6), sits inside the range.
- Super-Mercuries: Adibekyan et al. 2021, Science, arXiv:2102.12444:
  iron-mass fractions 59-71 % (Table S4, as recorded in B6). Brinkman et al.
  2024 Table 3 (p. 11): the same planets re-fitted give CMF 0.31-0.56
  (K2-38 b 0.4, K2-106 b 0.31, Kepler-406 b 0.52, K2-229 b 0.56) against
  0.63-0.75 in Adibekyan et al.; Brinkman et al. abstract: "do not identify
  a distinct population of high-density super-Mercuries".

### Stellar-abundance range (does not bound the planets)

- Unterborn et al. 2023, "The Nominal Range of Rocky Planet Masses, Radii,
  Surface Gravities and Bulk Densities", arXiv:2212.03934, p. 12: sampling
  stellar Fe/Mg and Si/Mg gives "an average CMF of 0.29", equivalent to
  Fe/Mg = 0.71; Earth-like and solar abundances give CMF 0.33 and 0.30.
  Fig. 4 is plotted on a CMF axis (contours 0.05 to 0.5 in the figure on
  p. 13); a numeric 99.7 % CMF range was not read.
- Schulze et al. 2021, arXiv:2112.14512: not re-read here; see B6.

**Not found / needs calibration**: a published lower bound from an
observation (the value 0 is a theoretical floor). Whether the draw should
include the hypothetical coreless planets is a design choice: the
alternative lower bound is 0.06 (Plotnykov & Valencia, 68 % edge,
derived). The upper bound 0.8 is "what seems to be" the maximum, an
inference from a sample of about 60 planets.

## 2. Water mass fraction

### 2a. Dry (inner) rocky planets

**Shape**: log-uniform.

**Proposed bounds**: 1e-5 to 1e-2 (mass fraction of the planet).

- Lower, 1e-5: Raymond, Quinn & Lunine, arXiv:astro-ph/0510285 (published
  as Astrobiology 7, 2007), Table 1 (p. 20), rows "Solar system", with
  footnote 5: water contents from Morbidelli et al. (2000). Mercury 1e-5,
  Mars 2e-4, Venus 5e-4, Earth 1e-3. Mercury is the driest planet of
  that table. Not independently read: Morbidelli et al. 2000.
- Lower, an independent indication: Izidoro & Piani 2022, "Origin of Water
  in the Terrestrial Planets", arXiv:2302.02674, p. 9: achondrite estimates
  "<= 0.007 wt.% of water in the Vestan mantle" (Stephant et al. 2021),
  that is 7e-5 as a mass fraction **(derived)**.
- Upper, 1e-2: Izidoro & Piani 2022 p. 7: for planets formed inside the
  snowline, "by analogy with inner Solar System materials, one would
  naively expect them to have a relatively low water content, potentially
  lower than 1 wt.%".
- Earth: total 2700 +/- 1350 ppm (Marty 2012, Table 1, p. 31, as recorded
  in B7); Izidoro & Piani 2022 Fig. caption p. 13 quotes a total of 3900 ppm
  (Peslier & De Sanctis 2022) with asymmetric error bars that did not
  extract legibly from the PDF.
- Population-level cap, for context: Rogers, Dorn, Raj, Schlichting &
  Young 2024, "Most Super-Earths Have Less Than 3% Water",
  arXiv:2409.17394, abstract: "a 1-sigma upper limit on total water mass
  fraction of <~ 3 % at the population level".
- Raymond et al. Table 1 (p. 20): simulated planets that accreted water
  from beyond 2.5 AU reach 9.3e-4 to 2e-2 (15 planets; initial water 0,
  0.1 % between 2 and 2.5 AU and 5 % beyond, section 2). Those are the
  migration/delivery case (2c), see
  below.

Enstatite versus carbonaceous chondrites (the user's suggestion): the
direct sources (Piani et al. 2020, Science 369, 1110; Alexander 2012,
Science 337, 721) are paywalled and the open copies could not be fetched
(HTTP 403 and an HTML landing page). The EC water contents (0.08-0.54 wt%)
and carbonaceous-chondrite contents (7.2-9.1 wt%) appeared only in a web
search summary of the preprint, **not verified**; they are not used.
Izidoro & Piani 2022 Fig. 13 caption shows the same ranges graphically.

### 2b. Icy (outer) planets

**Shape**: uniform.

**Proposed bounds**: 0.1 to 0.67.

- Upper, 0.67: Zeng et al. 2019, PNAS 116, 9723, p. 7 and p. 20 (recorded
  in B7): the ice fraction of icy planets lies between 1/2 and 2/3.
- Lower, 0.1: Marboeuf et al. 2014, "From stellar nebula to planetesimals",
  A&A 570, A35, arXiv:1407.7271, p. 11-12: "the amount of ices incorporated
  in icy planetesimals (resp. the ice/rock mass ratio) beyond the snow line
  ranges from 10 % to 60 % (see Fig. 8) of their total mass (resp. from 0.1
  to 1.5)"; and "the ice/rock mass ratio in planetesimals varies from 0.2
  to 1.5 (resp. 0.1 to 1.1) for non-irradiated (resp. irradiated)
  models". That is all ices, not only water, so 0.1 is an upper estimate
  of the minimum water share.
- Inferred values of real planets: Aguichine et al. 2021, arXiv:2105.01102,
  abstract and Table 3: GJ 9827 d, water mass fraction "~20 +/- 10
  %", "5-30 %" with the ternary diagrams. Luque & Palle 2022,
  arXiv:2209.03871, Fig. 2 caption: the water-world reference model is 50
  % water, 50 % Earth-like rock. Mousis et al. 2020, arXiv:2002.05243:
  models hydrospheres of 50 % of
  the mass ("50% LW - 50% Mantle" in the figures); its introduction cites
  rocky planets with steam atmospheres of "up to 5 wt% of water".

The range 0.5-0.67 (Zeng) is the formation outcome at solar rock:H2O; the
other sources show real candidates and planetesimal compositions below it.
The range should widen to 0.1-0.67 if the draw is meant to include
partially icy bodies.

### 2c. Migration chance (formed beyond the ice line, now inside)

**Not found.** Searched: arXiv listing for Izidoro, Raymond, Venturini and
population synthesis; read Raymond et al. 2018, MNRAS Letters,
arXiv:1805.10345 (abstract: inward migration of icy embryos "naively
predicts that most super-Earths should be very water-rich" but "the
shortest-period planets formed in the migration model are often purely
rocky"; no fraction given), Izidoro & Piani 2022 (no fraction) and
Rogers et al. 2024 (population-level water below ~3 %, which argues
against a large water-rich fraction). A web search also surfaced that
population-synthesis results "vary widely with disc lifetime and migration
efficiency", without a number. The chance 0.1 stays an unsourced
calibration.

## 3. Radiogenic abundance (dex of Earth's)

**Shape**: uniform in dex.

**Proposed bounds**: -0.5 to +0.5 dex for Th and U (heat from K is not
varied).

- Nimmo et al. 2020, "Radiogenic Heating and its Influence on Rocky Planet
  Dynamos and Habitability", arXiv:2011.04791, p. 5: "The 10-90 %
  percentile range observed in [Eu/Mg] is from about -0.3 to +0.3 for
  likely planet-hosting stars [...] while based on the Hypatia catalog the
  1-99 % range is about -0.5 to +0.5, or 30 % to 300 %". Appendix p. 13:
  "the range for [Eu/Mg] of -0.5 to +0.5 dex is conservative and would be
  higher if the Luck data were included". Abstract: Th and U "likely to
  vary by up to an order of magnitude between different planetary
  systems".
- Unterborn et al. 2015, "Thorium Abundances in Solar Twins and Analogues",
  arXiv:1505.00280, abstract: "log eps(Th) varies from 59 % to 251 % that
  of Solar", i.e. -0.23 to +0.40 dex **(derived)**, a narrower solar-twin
  sample.
- Botelho et al. 2019, "Thorium in solar twins: implications for
  habitability in rocky planets", arXiv:1810.10413, abstract: for 53
  thin-disc solar twins, the pristine [Th/H] is super-solar on average
  "under a uniform dispersion of 0.056 dex"; a much narrower population.

**Not found / needs calibration**: Frank et al. 2014 and the Wang et al.
2020 paper were not located in arXiv searches (searched "radiogenic
heating evolution model", "Th U stars"). The bounds are percentile ranges
(1-99 %) of a stellar catalogue, not absolute extremes; Nimmo says the
range is conservative. The effect of the heat on planets is the subject
of Nimmo et al., p. 9: -99 K to +166 K in mantle temperature (recorded in
B11).

## 4. Hydrogen-helium envelope fraction

The draw is mass dependent. Four regimes are proposed; none uses a sharp
class threshold from the literature, because none was found.

### 4a. Below about 5 Earth masses: nebula-captured envelope

**Shape**: log-uniform between bounds that depend on the core mass.

**Proposed bounds**: Lammer et al. 2014, "Origin and loss of
nebula-captured hydrogen envelopes from sub- to super-Earths in the
habitable zone of Sun-like stars", arXiv:1401.2765 (MNRAS 2014), Tables 1-3
(p. 4-5). Envelope mass fraction f_env = M_atm / M_pl for a rocky core in
a nebula at 1 AU around a Sun-like star, over the grid of dust depletion
f = 0.001, 0.01, 0.1 and accretion rate M_acc / M_pl = 1e-5 to 1e-8 per
year. The minimum and maximum over the 12 cases of each core mass (Tables
1-3), read from the tables and tabulated here:

| Core mass (Earth masses) | Minimum f_env | Maximum f_env |
| --- | --- | --- |
| 0.1 | 4.4e-8 | 1.0e-4 |
| 0.5 | 1.5e-6 | 5.9e-3 |
| 1 | 8.1e-6 | 2.5e-2 |
| 2 | 6.1e-5 | 0.11 |
| 3 | 2.0e-4 | 0.33 |
| 5 | 8.7e-4 | 0.35 |

The largest envelopes occur at the lowest accretion rate and the lowest
dust depletion factor f, the smallest at the highest rate and highest f
(Tables 1-3).
These are the envelopes captured before the disc disperses; the loss
after dispersal is the job of PR 2c. A smooth bound between the nodes
(log-linear in mass) is an interpolation, not a published value.

Mass below which no envelope is kept: **not found** as a sharp number. The
lowest mass in a published model that still captures a hydrogen envelope
is 0.1 Earth mass (Lammer et al. 2014 abstract: bodies "between
'sub-Earth'-like bodies of 0.1 M_earth and 'super-Earths' with 5 M_earth").
Stoekl et al. 2015, arXiv:1505.06859, abstract: for planets of about
Mars mass "the atmospheric mass changes drastically and on very short
timescales". The legacy cut at 0.5 Earth mass is not supported by a
source; a cut at 0.1 Earth mass is the lowest published modelled mass,
not a measured threshold.

### 4b. Mini-Neptunes, about 5 to 10 Earth masses (observed)

- Lopez & Fortney 2014, arXiv:1311.0329, ApJ 792, 1, p. 2: the model grid
  covers "planets from 1-20 Earth masses, 0.1-1000 F_earth, 0.01-60 %
  H/He" (a grid, not a physical bound). P. 8: "there is a fundamental
  change in the relationship around ~10 M_earth. Below this planets
  typically have less than ~5 % of their mass in H/He with no clear
  relationship between envelope fraction and mass. Above this [...] a
  steady rise in envelope fraction from sub-Neptunes up to gas giants";
  p. 8: "if a planet's rocky core becomes sufficiently massive, typically
  ~5-10 M_earth, then its gravity becomes sufficiently strong to trigger
  runaway accretion".
- Lopez & Fortney 2014 Table 1 (p. 10-11), inferred envelopes of measured
  planets: 55 Cnc e 0.14 %, Kepler-68 b 0.35 %, HD 97658 b 0.99 %, GJ 1214 b
  3.8 %, Kepler-11 d 6.6 %, Kepler-36 c 8.6 %, Kepler-11 e 15.7 % (8 Earth
  masses), Kepler-18 d 37.5 % (16.3 Earth masses).
- Wolfgang & Lopez 2015, arXiv:1409.2982, abstract: "H+He envelopes are
  most likely to be ~1 % of these planets' total mass with an intrinsic
  scatter of +/- 0.5 dex". This is the observed distribution that the
  legacy used (median 0.02 M/5, sigma 0.5 dex, which is close); the
  user's decision is to replace it by bounds.
- Lee & Chiang 2016 (recorded in B8): gas-to-core ratios of order 1-10 %;
  critical core mass 5-6 Earth masses at Z = 0.5-0.6.

**Proposed bounds** for 5 to 10 Earth masses: log-uniform from the 5 Earth
mass lower node of 4a (8.7e-4) to 0.35 (the 5 Earth mass upper node and
inside Lopez & Fortney's inferred 0.14-37.5 %). **Not found**: the
Lammer table stops at 5 Earth masses; above it only observed values and
the LF14 grid exist.

### 4c. Ice giants, about 10 to 20 Earth masses

**Shape**: uniform.

**Proposed bounds**: 0.09 to 0.25.

- Helled et al. 2011, "Interior Models of Uranus and Neptune",
  arXiv:1010.5546, p. 8: heavy-element masses 10.9
  (rock) to 13.2 (water) Earth masses for Uranus (14.536 Earth masses, p. 1)
  and 12.9 to 15.4 for Neptune (17.147 Earth masses). The remainder is
  hydrogen and helium: Uranus 1.3-3.6 Earth masses = 9-25 % and Neptune
  1.7-4.2 = 10-25 % **(derived)**.
- Helled et al. 2020, "Uranus and Neptune: Origin,
  Evolution and Internal Structure", arXiv:1909.04891, p. 14: the mass of
  H-He in both planets "is estimated to be a couple of M_earth".
- Mousis et al. 2020, arXiv:2002.05243, abstract: hydrogen-helium envelopes
  "up to ~30 % of the planetary mass" for planets of 1.6-3.9 Earth radii.
- The legacy draw U(0.05, 0.3) from 10 to 30 Earth masses: the upper 0.3
  matches Mousis et al.; the lower 0.05 matches only LF14 "less than ~5 %"
  for planets below 10 Earth masses. Within the ice-giant mass range the
  sourced range is 0.09-0.25.

### 4d. Gas giants, from about 20 Earth masses

**Shape**: heavy-element mass log-uniform; envelope = 1 - Mz / M.

- Thorngren, Fortney, Murray-Clay & Lopez 2016, "The Mass-Metallicity
  Relation for Giant Planets", arXiv:1511.07854, abstract: 47 transiting
  planets with "20 M_earth < M < 20 M_Jup"; p. 9: "a fit to the log of the
  data gives Mz = (57.9 +/- 7.03) M^(0.61 +/- 0.08)" with Mz in Earth
  masses and M in Jupiter masses; "the intrinsic spread was the factor
  10^sigma = 1.82 +/- 0.09". At 20 Earth masses the fit gives 10.7 Earth
  masses of heavy elements and at 1 Jupiter mass 57.9 **(derived)**. P. 1:
  "large heavy element masses of many planets (> 50 M_earth) suggest
  significant amounts of heavy elements in H/He envelopes, rather than
  cores".
- Thorngren et al. 2016 Table 1 (p. 8): total heavy-element mass of
  Jupiter 10-40 Earth masses and Saturn 20-30 (Guillot 1999), 37 and 27
  from their models. P. 9 text: Hubbard & Militzer 2016 favours about 22.
- Wahl et al. 2017, arXiv:1707.01997, p. 9: models with a dilute core "have
  core masses between 10 and 24 M_earth", total heavy elements 24 to 27;
  entropy-jump models give core masses down to ~8; p. 10 with the REOS3
  equation of state: "a small, compact core of ~6 M_earth or a diluted core
  of ~19 M_earth". Helled et al. 2011 intro quotes core masses of Jupiter
  between 0 and 6 Earth masses (Saumon & Guillot 2004, not read).
- The legacy core U(5, 20) capped at 0.1 M is in the range of Wahl et al.
  (6 to 24); the cap 0.1 M has no source. The legacy envelope fraction
  then equals 1 - core / M, close to 0.94 for Jupiter, whereas Thorngren's
  total heavy-element mass 57.9 M^0.61 gives 0.82 at 1 Jupiter mass.

**Proposed bounds**: heavy-element mass between the fit divided and
multiplied by the intrinsic scatter factor 1.82, that is the 1-sigma
spread, not a published extreme **(calibration needed: no published
bound)**; or, as the published-range alternative for Jupiter-mass planets
only, 10 to 40 Earth masses (Guillot 1999 via Thorngren Table 1).

### 4e. Class thresholds

- 10 Earth masses, change of regime: Lopez & Fortney 2014 p. 8 (above).
- 5-10 Earth masses, runaway core: Lopez & Fortney 2014 p. 8; Thorngren
  et al. 2016 introduction: "a seed core of ~5-10 M_earth of ice/rock must
  build up first, which begins accreting nebular gas" (citing Pollack et al.
  1996, **not read**).
- 20 Earth masses, giant planets: Thorngren et al. 2016 abstract (sample
  cut, not a physical threshold).
- 30 Earth masses (legacy): **not found**.

## Check of the legacy values

| Legacy value | Verdict |
| --- | --- |
| No envelope under 0.5 Earth mass | Unsourced; Lammer 2014 models down to 0.1 |
| Chance (M - 0.5) / 4.5 up to 10 | Unsourced; replaced by the uniform draw |
| Median 0.02 M / 5, 0.5 dex, cap 0.3 | Close to Wolfgang & Lopez 2015 (1 %, 0.5 dex) as an observed distribution; cap 0.3 matches Mousis 2020 |
| U(0.05, 0.3) from 10 to 30 | Upper bound sourced, lower bound narrowed to 0.09 by Helled 2011 |
| Gas giant core U(5, 20) | In range of Wahl 2017 (6 to 24) and the Thorngren 5-10 seed core |
| Core cap 0.1 M | Unsourced |
| Thresholds 10 and 30 Earth masses | 10 supported (Lopez & Fortney 2014); 30 not found |

## Summary

| Quantity | Shape | Proposed bounds | Main sources | Not found |
| --- | --- | --- | --- | --- |
| Core mass fraction | uniform | 0.0 to 0.8 | Elkins-Tanton & Seager 2008 (0); Plotnykov & Valencia 2020 (0.8); Brinkman 2024 Table 2 | An observed lower bound |
| Dry water fraction | log-uniform | 1e-5 to 1e-2 | Raymond 2006 Table 1 (Mercury 1e-5); Izidoro & Piani 2022 p. 7 (1 wt%) | EC and carbonaceous contents unverified |
| Icy water fraction | uniform | 0.1 to 0.67 | Zeng 2019 (0.67); Marboeuf 2014 p. 11-12 (0.1) | Marboeuf's number is all ices |
| Migration chance | n/a | 0.1 unsourced | Raymond 2018 (qualitative) | Any published fraction |
| Radiogenic abundance | uniform in dex | -0.5 to +0.5 | Nimmo 2020 p. 5 | Frank 2014, Wang 2020 |
| Envelope, 0.1 to 5 Earth masses | log-uniform | Lammer 2014 Tables 1-3 per core mass | Lammer 2014 | Sharp lower mass cut |
| Envelope, 5 to 10 | log-uniform | 8.7e-4 to 0.35 | Lopez & Fortney 2014 Table 1 | Model bounds above 5 |
| Envelope, ice giants | uniform | 0.09 to 0.25 | Helled 2011 p. 8 | Bounds for non-Solar-system cases |
| Envelope, gas giants | log-uniform in Mz | 57.9 M^0.61 / 1.82 to x 1.82 | Thorngren 2016 p. 9 | A published bound |
| Giant threshold 30 Earth masses | n/a | none | Lopez & Fortney 2014 (10); Thorngren 2016 (20 sample cut) | The 30 value |

## Decisions (user, 2026-11-21)

The bounds of the summary table are taken as proposed. Where no source
gives a bound:

1. **Ice giant to gas giant at 20 M⊕**, the lower validity bound of the
   Thorngren et al. 2016 heavy-element fit; the legacy 30 M⊕ has no
   source. The envelope fraction jumps at 20 M⊕ (from at most 0.25 to
   about 0.46 at the fit's centre), accepted.
2. **Gas-giant heavy-element mass log-uniform within two sigma** of the
   Thorngren et al. 2016 fit (a factor 1.82² either side): a
   calibration, since the paper gives a one-sigma scatter and no
   extremes.
3. **Envelope upper bound 0.35 held from 5 to 10 M⊕**: a calibration,
   the Lammer et al. 2014 grid stops at 5 M⊕. The envelope drawn is the
   primordial one; loss after the disc is PR 2c's.
4. **Migration chance 0.1 kept** as a calibration (no published
   fraction found).

Amended by the giants study's decisions: the ice-giant envelope upper
bound is 0.20 (the edge of the Lopez & Fortney 2014 tables), not 0.25
(`giants-and-envelopes.md`, Decisions).
