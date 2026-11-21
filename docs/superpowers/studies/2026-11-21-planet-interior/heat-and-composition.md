# Literature study: internal heat and composition constants

Date: 2026-11-21. Rule followed: a number is listed only if it was read in a
source text during this study. The reading channel is stated for each source:

- **Full text**: the PDF was downloaded and text-extracted, and the passage was
  read.
- **Abstract (search)**: only a search-engine rendering of the abstract was
  read. Treat
  as unverified against the paper itself.
- **Not reached**: the paper could not be opened. Nothing is claimed from it.

Page numbers `pN` are PDF pages of the arXiv or publisher file that was read.
Numbers marked **(derived)** are my arithmetic on sourced inputs, not quoted.

Sources used, with the channel:

| Short name | Reference | Channel |
| --- | --- | --- |
| Dye 2012 | S. T. Dye, Rev. Geophys. 50, RG3007, arXiv:1111.6099 (Tables 2, 3, 7) | Full text |
| Davies 2010 | J. H. Davies & D. R. Davies, Solid Earth 1, 5-24 | Full text |
| Nimmo 2020 | Nimmo, Primack, Faber, Ramirez-Ruiz, Rieke, arXiv:2011.04791 | Full text |
| Unterborn 2015 | Unterborn, Johnson & Panero, ApJ, arXiv:1505.00280 | Full text (abstract only read) |
| Lainey 2016 | V. Lainey, Celest. Mech. Dyn. Astr. 126, 145, doi:10.1007/s10569-016-9695-y, arXiv:1604.04184 | Full text (Table 1, p5) |
| Lainey 2012 | Lainey et al., ApJ 752, 14, arXiv:1204.0895 | Full text |
| Lainey 2009 | Lainey et al., Nature 459, 957, doi:10.1038/nature08108 | Abstract (search); same values repeated in Lainey 2016 Table 1 (full text) |
| Henning 2009 | Henning, O'Connell & Sasselov, ApJ, arXiv:0912.1907 | Full text |
| Meyer & Wisdom 2007 | Icarus 188, 535 (MIT preprint) | Full text |
| Adibekyan 2021 | Science 374, 330, doi:10.1126/science.abg8794, arXiv:2102.12444 | Full text |
| Plotnykov & Valencia 2020 | MNRAS 499, 932, doi:10.1093/mnras/staa2615, arXiv:2010.06480 | Full text |
| Schulze 2021 | PSJ 2, 113, arXiv:2011.08893 | Full text |
| Zeng 2019 | PNAS 116, 9723, doi:10.1073/pnas.1812905116, arXiv:1906.04253 | Full text |
| Luque & Palle 2022 | Science 377, 1211, doi:10.1126/science.abl7164, arXiv:2209.03871 | Full text (main text only, no supplement) |
| Raymond 2005 | Raymond, Quinn & Lunine, Icarus (hot-Jupiter paper), arXiv:astro-ph/0407620 | Full text |
| Lee & Chiang 2016 | ApJ 817, 90, doi:10.3847/0004-637X/817/2/90, arXiv:1510.08855 | Full text |
| Lopez & Fortney 2014 | ApJ, arXiv:1311.0329 | Full text |
| Ginzburg 2016 | Ginzburg, Schlichting & Sari, ApJ 825, 29, arXiv:1512.07925 | Full text (abstract) |
| Wu 2019 | ApJ 874, 91, doi:10.3847/1538-4357/ab06f8, arXiv:1806.04693 | Full text |
| Neil & Rogers 2020 | ApJ, arXiv:1911.03582 | Full text |
| Marty 2012 | EPSL 313-314, 56, doi:10.1016/j.epsl.2011.10.040, arXiv:1405.6336 | Full text |
| Wang 2018 | Wang, Lineweaver & Ireland, Icarus 299, 460, arXiv:1708.08718 | Full text |
| Mumma & Charnley 2011 | ARA&A 49, 471 | Full text |
| Rubin 2019 | MNRAS 489, 594, doi:10.1093/mnras/stz2086, arXiv:1907.11044 | Full text |
| Arevalo 2009 | EPSL 278, 361, doi:10.1016/j.epsl.2008.12.023 | Abstract (search) only |
| Connelly 2012 | Science 338, 651, doi:10.1126/science.1226919 | Abstract (search) only |
| Amelin 2002 | Science 297, 1678, doi:10.1126/science.1073950 | Abstract (search) only |
| Valencia 2007 | Valencia, O'Connell, Sasselov, arXiv:astro-ph/0610122 | Full text |

Not reached at all: McDonough & Sun 1995, McDonough 2003, Turcotte & Schubert
2014, Korenaga 2008, Herzberg 2010, Plesa 2018, Pollack 1996, Fulton 2017 (a
radius census, no mass function), Frank 2014, Hirschmann & Dasgupta 2009, Wahr
2006, Howett 2011, Hauck 2013 (read only as cited in Plotnykov & Valencia 2020).

---

## A. Internal heat

### A1. Radiogenic heat today, isotope shares, half-lives

**Legacy**: 20 TW; shares U-238 0.372, U-235 0.016, Th-232 0.419, K-40 0.193;
half-lives 4.468, 0.704, 14.0, 1.248 Gyr.

**Literature (Dye 2012, Table 2, p32)**: decay constants lambda and heat per
unit mass of the parent isotope:

| Isotope | lambda (1e-18 s^-1) | Heat h (uW/kg of isotope) | Half-life (derived, ln2/lambda) |
| --- | --- | --- | --- |
| U-238 | 4.916 | 95.13 | 4.468 Gyr |
| U-235 | 31.210 | 568.47 | 0.704 Gyr |
| Th-232 | 1.563 | 26.28 | 14.05 Gyr |
| K-40 | 17.200 | 28.47 | 1.277 Gyr |

- The three legacy half-lives for U-238, U-235 and Th-232 are reproduced.
- K-40 differs: Dye's lambda gives 1.277 Gyr, not 1.248 Gyr. The 1.248 Gyr value
  was not found in any source read here. It is the commonly quoted total
  half-life, but this study cannot cite it. Difference is 2 %.
- Element-level heat (Dye Table 3, p32-33): U 98.5, Th 26.3, K 3.33e-3 uW/kg of
  element. U-238 carries 95.8 % of uranium heat **(derived)**.
- Heat of the silicate Earth (Dye Table 7, p33, M_SE = 4.03e24 kg):
  - Cosmochemical model: U 12-22 ng/g, Th 45-83 ng/g, K 140-260 ug/g, giving
    11-21 TW.
  - Geophysical model: U 20-35, Th 77-135, K 240-420, giving 19-38 TW.
  - Chondritic (CI) model: 13 TW.
- Arevalo 2009 (abstract via search): K in the silicate Earth is 280 +/- 120
  ug/g
  (K/U = 13 800 +/- 2600). The "20 TW" figure was **not found** in that
  abstract.
  The 20 TW legacy value sits inside Dye's cosmochemical (11-21) and
  geophysical (19-38) ranges but is not a quoted result of either.
- Shares recomputed from Dye Tables 2, 3, 7 **(derived)**: with U and Th at the
  cosmochemical or geophysical ranges and K 140-420 ug/g, shares are U ~0.41,
  Th ~0.42, K ~0.165 (U-235 about 1.7 % of the total). With K = 280 ug/g
  (Arevalo), U = 20 and Th = 80 ng/g, K share is ~0.19. This reproduces the
  legacy
  K share 0.193 and U-238 + U-235 = 0.388 to within the model spread. A
  chondritic
  (CI) K of 544 ug/g would give K 0.54, so the K share is the sensitive one.

**Recommendation**: keep the shares and the 20 TW as a calibration inside the
published 11-38 TW range, cited to Dye 2012 Table 7. Replace K-40 half-life
1.248
by 1.277 Gyr only if a single self-consistent source (Dye 2012) is preferred.
State that K share depends on the assumed K (Arevalo: 280 +/- 120 ug/g).
McDonough & Sun 1995 and Turcotte & Schubert 2014 could not be read: "not found"
for
their tables.

### A2. Surface heat flow, secular cooling, Earth age

**Legacy**: total 46-47 TW; secular cooling 26 TW; age 4.567 Gyr.

- Davies 2010 (p1-2): preferred total **47 TW** (rounded from 46.7 TW, error
  +/- 2 TW). They quote JLM07 at 46 +/- 3 TW. The legacy 46-47 TW is supported.
- Secular cooling 26 TW is **not a sourced quantity**. It equals 46 - 20
  **(derived)**. Nimmo 2020 (p3) says only that Earth loses heat "roughly twice
  as
  fast as it is being generated by radioactive decay". Valencia 2007 (p5) takes
  44 TW surface flow, crust radiogenic ~8 TW, mantle radiogenic ~58 % of the
  total, and says secular cooling is "difficult" to scale.
- Age: Connelly 2012 gives CAI age 4567.30 +/- 0.16 Myr (abstract via search).
  Amelin 2002 gives CAI 4567.2 +/- 0.6 Ma (abstract via search). 4.567 Gyr is
  supported as the age of the solar system, not as a separate Earth age.

**Recommendation**: keep 46-47 TW (Davies 2010) and 4.567 Gyr (Connelly 2012).
Keep
secular cooling 26 TW as a documented difference (total - radiogenic), not as a
sourced constant. Keep the 4.567 Gyr label as "CAI age".

### A3. Cooling e-folding time and mass scaling

**Legacy**: e-folding ~10 Gyr, "calibration on Earth and Mars" (Herzberg 2010,
Plesa 2018).

- Arevalo 2009 (abstract via search): convective Urey ratio ~0.34, implying
  present mantle cooling of **70-130 K/Gyr**. This is the only K/Gyr rate found.
- Herzberg 2010: not reached in full text. The search returned only that
  potential temperature fell ~200-300 K since ~3 Ga (secondary). No K/Gyr value
  from the paper itself. Plesa 2018: not read.
- Korenaga 2008 Urey-ratio range 0.08-0.38 (seen only through a citing paper's
  abstract, so not verified).
- Valencia 2007 (p5): states it is difficult to scale secular cooling with
  mass. It scales radioactive heat with mantle mass and expects a more massive
  planet to convect more vigorously (higher Rayleigh number). It gives no
  scaling law for cooling.
- A parameterised mass-scaling law for secular cooling: **not found** in the
  sources read. Search leads not opened: arXiv:2110.03295 (parameterised model
  for
  1-10 Earth masses), Van Heck & Tackley 2011.

**Recommendation**: keep the 10 Gyr e-folding as a calibration, with a
cross-check: 70-130 K/Gyr on a ~1600-1700 K mantle is an e-fold of order 10-20
Gyr
**(derived, mantle temperature not sourced here)**. Do not claim a mass scaling.
Say "no sourced law found".

### A4. Tidal dissipation k2/Q

**Legacy**: rocky 0.015 (Io, Lainey 2009), icy 2.5e-3 (legacy calibration:
Wahr 2006, Howett 2011), Jupiter 1.1e-5.

| Body | k2 | Q | k2/Q | Source |
| --- | --- | --- | --- | --- |
| Earth | 0.302 (Wahr 1981, as used by Lainey) | 280 (230-360, 1 sigma) | 1.08e-3 **(derived)** | Lainey 2016 Table 1 (p5), from Ray 2001 |
| Moon | 0.024059 (Konopliv 2013) | 37.5 +/- 4 (month) | 6.4e-4 **(derived)** | Lainey 2016 Table 1 (Williams 2014) |
| Mars | 0.173 +/- 0.009 | 99.5 +/- 4.9 | 1.74e-3 **(derived)** | Lainey 2016 Table 1 |
| Io | n/a | n/a | **0.015 +/- 0.003** | Lainey 2009; Lainey 2016 Table 1 |
| Jupiter | n/a | n/a | **(1.102 +/- 0.203)e-5** | Lainey 2009; Lainey 2016 Table 1 |
| Saturn | 0.390 +/- 0.024 | n/a | **(1.59 +/- 0.74)e-4** | Lainey 2016 Table 1 (Lainey 2017) |
| Saturn (older) | assumed 0.341 | n/a | (2.3 +/- 0.7)e-4 | Lainey 2012 abstract, p2 |
| Titan | 0.589 +/- 0.150 (2 sigma) | not given | not given | Lainey 2016 Table 1 (Iess 2012) |
| Enceladus | 0.0018 (assumed) | 20 (assumed) | 9e-5 **(derived)** | Meyer & Wisdom 2007 p1, quoting Porco 2006 |
| Europa | **not found** | | | searched Wahr 2009 and Europa tidal papers; no k2/Q value read |
| Super-Earth baseline | 0.3 | 50 (at 1 Earth mass) | 6e-3 **(derived)** | Henning 2009 p5, a model baseline, not a measurement |

Notes:

- Henning 2009 p5: Earth k2 = 0.299 (Yoder 1995), Mars 0.14, Jupiter 0.38; "less
  than
  a factor of 2 uncertainty in k2" for terrestrial planets. Q=10 to 100 is
  treated as a plausible range.
- Lainey 2012 p11: Enceladus' own dissipation could not be fitted; it was
  bracketed
  by two extreme scenarios (at thermal equilibrium vs not dissipative).
- Mini-Neptune or ice-giant values: **not found** in the sources read (Lainey
  2016
  says Uranus and Neptune k2/Q cannot yet be determined, abstract via search).
- Legacy icy 2.5e-3: Wahr 2006 (the search found a Wahr 2009 Icarus paper
  instead,
  with no Q value) and Howett 2011 were not reached. **Not found.** The
  measured/assumed Enceladus value (9e-5) is 28 times lower than the legacy icy
  value. The measured Earth, Moon and Mars k2/Q values are 6e-4 to 1.7e-3,
  closer to 2.5e-3.

**Recommendation**: keep Io 0.015 and Jupiter 1.1e-5 (verified). Replace "icy
2.5e-3, Wahr 2006, Howett 2011" with a documented calibration: neither paper was
verified, and sourced icy-moon values range from 9e-5 (Enceladus, assumed) to
~1e-3 (Earth, Mars, Moon). Use Saturn 1.59e-4 and Earth 1.08e-3 (derived) as
sourced anchors. For super-Earths use Henning 2009 baseline 6e-3 flagged as a
model assumption.

### A5. Silicate mass of Earth

**Legacy**: 4.03e24 kg (PREM).

- Dye 2012 Table 7 (p33): "Primitive Mantle ... (M_SE = 4.03x10^24 kg)". Table 4
  (p33) gives crust 0.30e24, upper mantle 1.06e24, lower mantle 2.81e24, D"
  0.13e24, outer core 1.84e24, inner core 0.10e24, Earth 5.97e24 kg. Crust +
  mantle = 4.30e24 kg is not the same thing as M_SE; the 4.03e24 kg is Dye's
  choice for the bulk silicate Earth model.
- Marty 2012 (p16, p31) uses a bulk Earth mass of 5.98e27 g and a silicate Earth
  of
  4.03e27 g **(derived from the ratio of his columns, not stated)**.
- PREM itself (Dziewonski & Anderson 1981) was not reached.

**Recommendation**: keep 4.03e24 kg, cited to Dye 2012 Table 7 (not PREM).

---

## B. Composition draws

### B6. Core mass fraction (CMF)

**Legacy**: mean 0.325, +0.15/dex [Fe/H], sigma 0.05, range 0.15-0.55;
Mercury-like
frequency with CMF ~0.7.

- Plotnykov & Valencia 2020, abstract p1: the star-inferred CMF distribution is
  **0.32 +0.14/-0.12**; rocky planets follow a broader, lower-peaked
  distribution,
  **0.24 +0.33/-0.18**. Maximum iron enrichment seen is CMF ~0.8. Earth CMF
  0.325
  (Wang 2018) and Mercury CMF 0.63 +/- 0.03 (Hauck 2013, quoted on p6) are used.
- Adibekyan 2021: planet iron fraction correlates with the stellar iron fraction
  with slope >4 (abstract); 6.3 +/- 1.2 (all) vs 4.3 +/- 0.8 (super-Earths only,
  core+mantle iron), p7. Five "super-Mercuries" out of 22 planets (23 %), with
  f_iron 59-71 % depending on assumption (Table S4, p25). They are the five
  planets: K2-38 b, K2-106 b, K2-229 b, Kepler-107 c, Kepler-406 b (p5).
- Stellar side, Adibekyan Table S3 (p24): 21 stars with [Fe/H] and stellar iron
  mass
  fraction. A least-squares fit I ran **(derived)** gives f_iron,star = 33.2 % +
  3.8 %/dex x [Fe/H], correlation r = 0.33, scatter 2.3 %. Weak. TOI-561 (-0.39
  dex,
  25.2 %) leverages it. Multiplying by the planet/star slope of 4-6 gives
  ~15-23 %/dex for the planetary iron fraction **(derived, speculative)**, which
  is
  the order of the legacy +0.15/dex.
- Schulze 2021 (abstract): of 11 planets with host abundances, only Kepler-107 c
  is a likely super-Mercury at 2 sigma. K2-229 b is not at 1 or 2 sigma.
- A published slope in "CMF per dex [Fe/H]": **not found.**

**Recommendation**: mean 0.325 is supported (Earth value, and stars 0.32).
Replace sigma 0.05 and range 0.15-0.55 by the observed planet spread: 0.24
(+0.33/-0.18) from Plotnykov & Valencia, i.e. much wider, plus a tail to 0.8.
Replace Mercury CMF 0.7 with 0.63 +/- 0.03. Keep +0.15/dex as a calibration
(sourced indirectly by the Adibekyan slopes). Mercury-like frequency: 5/22 in
Adibekyan, 1/11 in Schulze; this is the observed range, so a 5-20 % tail is
supported but imprecise.

### B7. Water mass fraction, inside versus beyond the ice line

**Legacy**: inside 10^U(-4,-2.7); beyond U(0.2,0.5); migration chance 10 %.

- Inside: Raymond 2005 (p2) disk model has inner bodies dry, 0.1 % water beyond
  2 AU, 5 % beyond 2.5 AU. Marty 2012 (p18): bulk Earth 2700 +/- 1350 ppm =
  "10 +/- 5 ocean masses", i.e. one ocean ~270 ppm of Earth mass **(derived)**,
  consistent with 2.3e-4. Marty's preferred bulk range is 1000-3000 ppm
  (abstract). The legacy 10^-4 to 10^-2.7 (1e-4 to 2e-3) brackets all of these.
- Beyond: Zeng 2019 (p7, p20): ice fraction x of icy planets is uniform between
  1/2 and 2/3 (solar rock:H2O ratio, or with clatrate/hydrate ices). Luque &
  Palle
  2022 (p3, Fig. 1 caption) use rock:water ice 1:1 (50 %) as the water-world
  model.
  The legacy U(0.2,0.5) is below both sources.
- Migration chance: Luque & Palle say water-rich worlds form outside the snow
  line and migrate (abstract). A fraction of water-rich planets is **not found**
  in the main text read (the supplement was not available).

**Recommendation**: keep inside range (sourced). Replace beyond U(0.2,0.5) by
U(0.5,0.67) (Zeng 2019) if the planet is a "water world", and keep the lower
range as
a calibration for partly-ice bodies. Migration 10 %: keep as calibration, "no
sourced
fraction found".

### B8. H/He envelope fraction and giant thresholds

**Legacy**: thresholds 10 and 30 Earth masses.

- Lee & Chiang 2016 (p1): gas-to-core ratios of super-Earths are of order
  **1-10 %**. Critical core mass ~5-6 Earth masses at Z = 0.5-0.6 and down to 2
  Earth masses at Z = 0.8 (p2). At no metallicity can a 10 Earth-mass core in an
  undepleted nebula remain a super-Earth (p3).
- Lopez & Fortney 2014 (p8): below ~10 Earth masses planets typically have less
  than ~5 % H/He with no clear mass relation; above, the envelope fraction
  rises.
  Runaway typically at core 5-10 Earth masses (p8). Model range 0.01-60 % H/He.
- Ginzburg 2016 (abstract): cores of a few Earth masses get a few-percent
  envelopes; "lighter than a few % of the planet's mass" can be stripped by
  core-powered loss.
- The 30 Earth-mass threshold: **not found**. Pollack 1996 not read.
- Spread of the fraction: no published sigma found beyond the 1-10 % range.

**Recommendation**: keep 10 Earth masses as the lower runaway threshold
(Lopez & Fortney, Lee & Chiang: 5-10). Keep 30 as a calibration. Draw
envelope fraction log-uniform in 1-10 % below ~10 Earth masses.

### B9. Small-planet mass distribution

**Legacy**: log-uniform 0.05-20 Earth masses calibrated on the Fulton 2017
radius valley.

- Neil & Rogers 2020, Table 1 (p7): a log-normal in M with parameters
  (ln of mass in Earth masses):

  | Model | mu_M | sigma_M | Median mass |
  | --- | --- | --- | --- |
  | 1 (single population) | 0.29 +0.14/-0.16 | 1.72 +/- 0.08 | 1.3 |
  | 2 (envelope mass loss) | 1.00 +/- 0.08 | 1.65 +/- 0.06 | 2.7 |
  | 3 | 1.72 +0.13/-0.14 | 1.38 +/- 0.08 | 5.6 |
  | 4 (two populations) | 0.60 +/- 0.48 (component 1), 1.72 +/- 0.08 (rocky, component 2) | 2.39, 0.63 | 1.8, 5.6 |

  (Medians and column labels are **(derived)** from exp(mu); assignment of
  model-4
  components to "gaseous" or "rocky" was not confirmed.) Lower bound in the fit
  0.1 Earth masses, upper 10 000 (p3).
- Wu 2019 (p1, p8): super-Earth masses peak narrowly around **8 Earth masses x
  (M*/Msun)**; mu = Mp/M* ~ 2.5e-5.
- Mass functions are described as broken power laws with a break at ~10 Earth
  masses for Kepler planets and ~20 for microlensing (Neil & Rogers p3, citing
  Pascucci 2018, Suzuki 2016; not read directly).
- Fulton 2017 gives radii only: **no mass function**.

**Recommendation**: replace log-uniform by a log-normal with median 2.7-5.6
Earth
masses and sigma ln M ~1.4-1.7 (Neil & Rogers model 2 or 3) for the transiting
population (P < ~100 d). Caveat: that sample is detection-biased and
short-period.

### B10. Volatile budgets

**Legacy**: rock CO2 1e-4, N2 2e-6, S 2.5e-4; ice CO2 0.05, N2 1e-3, CH4 0.01,
NH3 0.01 (mass fractions).

Rock side:

- Wang 2018 Table 1 (p7), primitive mantle: **C 109 +77/-39 ppm**, **N 2.01
  +1.42/-0.71 ppm**, **S 235 +/- 38 ppm**. Converted **(derived)**: CO2 = 109 x
  44/12
  = 4.0e-4 by mass; N2 = 2.0e-6; S = 2.35e-4.
- Marty 2012 Table 1 (p31), bulk Earth, normalised to total Earth mass: C 526
  +/- 206
  ppm; N 1.68 +/- 0.85 ppm; H2O 2699 +/- 1347 ppm. C as CO2 = 1.9e-3
  **(derived)**.
  Marty's abstract: ~500 ppm C and 1000-3000 ppm water largely in the solid
  Earth.
  Table columns were extracted from PDF text, the ppm column is read as bulk
  Earth; the mol/g columns could not be reconciled exactly, so treat as +/- 20
  %.
- McDonough 2003 and McDonough & Sun 1995 for S: not reached. S 250 +/- 50 ppm
  attributed to McDonough by a search result, not verified. Wang 2018 gives
  235 +/- 38.

Ice side (mol percent relative to H2O):

| Species | Comets (Mumma & Charnley 2011, p33 table) | Comet 67P (Rubin 2019, Table 2, p13) | 67P as mass fraction of H2O **(derived)** | Legacy |
| --- | --- | --- | --- | --- |
| CO2 | 2-30 % | 4.7 +/- 1.4 | 0.115 | 0.05 |
| CH4 | 0.4-1.6 % | 0.34 +/- 0.07 | 0.0030 | 0.01 |
| NH3 | 0.2-1.4 % | 0.67 +/- 0.20 | 0.0063 | 0.01 |
| N2 | not tabulated | 0.089 +/- 0.024 | 0.0014 | 1e-3 |

Mumma & Charnley p15: CO2 exceeds 20 % in 7 of 14 comets (Akari survey, Fig. 7).
The legacy "ice fraction" basis (mass fraction of the ice or of H2O) is not
stated
in the brief; the conversion above assumes mass relative to H2O.

**Recommendation**: rock S 2.5e-4 and N2 2e-6 are supported (Wang 2018: 2.35e-4,
2.0e-6). Rock CO2 1e-4 is **4x low** against Wang 2018 mantle C (4.0e-4) and
~19x
low against Marty's bulk Earth (1.9e-3): replace by ~4e-4 (mantle) or keep as a
calibration (surface-accessible fraction). Ice: N2 1e-3 matches 67P (1.4e-3),
NH3 0.01 and CH4 0.01 are 1.6x and 3x above 67P but inside the range of comets
(NH3 up to 1.4 mol %, CH4 up to 1.6 mol %); CO2 0.05 is below 67P and mid-range
of the comet spread (2-30 mol %, i.e. 0.05-0.73 by mass of H2O). Keep ice values
as calibration with range, note that CO2 varies by a factor 10 between comets.

### B11. Spread of radiogenic abundance among stars

**Legacy**: 0.15 dex.

- Unterborn 2015 (abstract): log eps(Th) varies from **59 % to 251 % of solar**,
  i.e. -0.23 to +0.40 dex **(derived)** on a sample of solar twins and analogues
  (sample size not read).
- Nimmo 2020 (p5): [Eu/Mg] 10-90 % range about -0.3 to +0.3 for likely
  planet-hosting stars; the 1-99 % range from the Hypatia catalogue is -0.5 to
  +0.5
  (30-300 %). Th and U "likely to vary by up to an order of magnitude" between
  systems (abstract). K variation between stars expected to be limited (p5).
- Nimmo 2020 p9: +/- variations in heating give present mantle temperatures
  -99 K to +166 K.
- Frank 2014: not reached. No standard deviation read. A 2013 conference
  abstract
  mentioned Th/U between 1.02 and 1.64 (from a search result, secondary).

**Recommendation**: 0.15 dex appears small. A 10-90 % range of +/-0.3 dex
implies a
1 sigma of ~0.23 dex (**derived**, if Gaussian). Replace 0.15 by ~0.2-0.25 dex
for
U and Th (not K), citing Nimmo 2020 p5 and Unterborn 2015. Keep 0.15 as a
documented
calibration if a narrower spread is preferred.

---

## Summary of what could not be verified

- Arevalo 2009 "20 TW", Korenaga 2008, Herzberg 2010, Plesa 2018, Turcotte &
  Schubert 2014 Table 4.2, McDonough & Sun 1995 shares: not reached. K/Gyr value
  from Arevalo is from its abstract only.
- K-40 half-life 1.248 Gyr: not found in a read source; Dye gives 1.277 Gyr.
- Secular-cooling law scaling with mass: not found.
- Europa k2/Q and icy 2.5e-3 (Wahr 2006, Howett 2011): not found.
- 30 Earth-mass giant threshold, Pollack 1996: not found.
- Water-world fraction and migration probability: not found (supplement of
  Luque & Palle not read).
- Frank 2014 spread: not found.
- Amelin 2002, Connelly 2012, Lainey 2009: values seen in abstracts via search.
- PREM itself: not read.
- Marty 2012 Table 1 columns: extracted from a PDF with imperfect layout.
