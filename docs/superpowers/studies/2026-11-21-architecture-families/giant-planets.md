# Literature study: giant planets in the architecture families

Date: 2026-11-21. Rule followed: a number is listed only if it was read in a
source text during this study. The reading channel is stated for each source:

- **Full text**: the PDF was downloaded and text-extracted, and the passage was
  read.
- **Abstract (search)**: only a search-engine rendering of the abstract was
  read. Treat as unverified against the paper itself.
- **Not reached**: the paper could not be opened. Nothing is claimed from it.

Page numbers `pN` are PDF pages of the arXiv file that was read (the arXiv
preprint, so they differ from the journal pagination). Numbers marked
**(derived)** are my arithmetic on sourced inputs, not quoted. Every number
below is a literature value for real, mostly Sun-like, hosts; the draw aims
for variety, not for real frequencies, so occurrence rates are given only as
plausibility ranges.

Sources used, with the channel:

| Short name | Reference | Channel |
| --- | --- | --- |
| Dawson & Johnson 2018 | ARA&A 56, "Origins of Hot Jupiters", arXiv:1801.06117 | Full text |
| Ford & Rasio 2006 | ApJ 638, L45, arXiv:astro-ph/0512632 | Full text |
| Wright 2012 | ApJ 753, "The Frequency of Hot Jupiters", arXiv:1205.2273 | Full text |
| Wright 2009 | ApJ 693, 1084, arXiv:0812.1582 | Full text |
| Steffen 2012 | PNAS 109, 7982, arXiv:1205.2309 | Full text |
| Huang 2016 | ApJ 825, 98, arXiv:1601.05095 | Full text |
| Knutson 2014 | ApJ 785, 126, arXiv:1312.2954 | Full text (abstract and intro read) |
| Bryan 2016 | ApJ 821, 89, arXiv:1601.07595 | Full text |
| Kipping 2013 | MNRAS 434, L51, arXiv:1306.4982 | Full text |
| Juric & Tremaine 2008 | ApJ 686, arXiv:astro-ph/0703160 | Full text |
| Ford & Rasio 2008 | ApJ 686, 621, arXiv:astro-ph/0703163 | Full text |
| Rosenthal 2023 (CLS IV) | ApJS (2023), arXiv:2310.08707 | Full text |
| Zhu & Wu 2018 | AJ 156, 92, arXiv:1805.02660 | Full text |
| Masuda 2020 | AJ 159, 38, arXiv:1912.00387 | Full text (abstract only read) |
| Bryan 2019 | AJ, arXiv:1806.08799 | Full text (abstract only read) |
| Bryan & Lee 2024 | arXiv:2403.08873 | Full text (abstract only read) |
| He & Weiss 2023 | AJ, arXiv:2306.08846 | Full text (abstract only read) |
| Pu & Lai 2018 | MNRAS 478, 197, arXiv:1801.06220 | Full text (abstract and summary read; equations unreadable in extraction) |
| Cumming 2008 | PASP 120, 531, arXiv:0803.3357 | Full text |
| Fernandes 2019 | ApJ 874, 81, arXiv:1812.05569 | Full text |
| Fulton 2021 (CLS II) | ApJS 255, 14, arXiv:2105.11584 | Full text |
| Johnson 2010 | PASP 122, 905, arXiv:1005.3084 | Full text |
| Spiegel 2011 | ApJ 727, 57, arXiv:1008.5150 | Full text (abstract only read) |
| Kennedy & Kenyon 2008 | ApJ 673, arXiv:0710.1065 | Full text |
| NASA fact sheet | nssdc.gsfc.nasa.gov/planetary/factsheet/ (metric table and ratio-to-Earth table) | Web page, read through a fetch tool |
| Udry & Santos 2007 | ARA&A 45 | Not reached (no arXiv copy tried) |
| Hayashi 1981 | Prog. Theor. Phys. Suppl. (1981) | Not reached; used only through secondary sources below |

Not reached: Udry & Santos 2007 and Hayashi 1981. No number is claimed from
either. Ford & Rasio 2006 was cited in the brief by its "2 a_Roche" result;
the arXiv copy was read in full.

Note on the extraction: the PDF text of Pu & Lai 2018 lost its mathematical
symbols and spaces, so none of its numerical examples is quoted here.

## 1. Hot-Jupiter family

### 1.1 Definition, period range and pile-up

| Quantity | Value | Location |
| --- | --- | --- |
| Hot Jupiter, definition | mass at least 0.25 M_J and P < 10 d | Dawson & Johnson, p2 |
| Hot Jupiter, definition | P < 10 d and M sin i > 0.1 M_J | Wright 2012, p2 |
| Hot Jupiter, definition | P < 10 d (warm Jupiter: 10 to 200 d) | Huang 2016, abstract, p1 |
| Pile-up period | "a modest peak at ~3 days", but a "misleadingly large" selection effect in transit samples; its significance "has not been definitively established" (a peak or only a drop-off inside 3 d) | Dawson & Johnson, p21 |
| Pile-up period | "between 1 to 5 days has long been noted" | Steffen 2012, p1 |
| Drop-off | the abrupt drop in planet number at P below ~3 d is real, upper limits are not affected until ~1 d | Cumming 2008, p8 |
| Pile-up in a | between 0.03 and 0.07 AU among single-planet systems; 38 of 88 singles with a < 1 AU (43%) have a < 0.07 AU; 7 of 39 planets in multis with a < 1 AU, 15 expected at the same rate | Wright 2009, p24 |
| Shortest periods in a Kepler sample | HJ candidates with P between 0.8 and 6.3 d (radius above 0.6 R_J) | Steffen 2012, p2 to p3 |
| Occurrence | 1.2 +/- 0.38% of FGK dwarfs for P < 10 d, M sin i > 0.1 M_J | Wright 2012, abstract, p1 |
| Occurrence | "Only 1% of Sun-like stars host one"; about one in ten giant-planet systems has a hot Jupiter | Dawson & Johnson, p2 and p28 |

### 1.2 Inner edge and the Roche limit

- Roche separation: R_P = 0.462 a_R mu^(1/3), with mu = m / M_star
  (Ford & Rasio 2006, p1). Dawson & Johnson write
  a_Roche = f_p R_p (M_star / M_p)^(1/3) with f_p = 2.7 (from hydrodynamic
  disruption simulations they cite), and give
  P_Roche = 0.79 d (f_p/2.7)^(3/2) (R_p / 1.3 R_J)^(3/2) (M_J / M_p)^(1/2)
  (Eq. 15, p22). The same page gives f_p = 3^(1/3) = 1.44 for R_Hill = R_p.
- Ford & Rasio 2006 find the inner cutoff "close to twice the Roche limit",
  as expected for planets circularised from highly eccentric orbits, and
  reject a cutoff at or inside the Roche limit (abstract p1, discussion p6).
  The slope of the edge could not be fitted from the sample (p5).
- High-eccentricity migration expects survivors at or beyond 2 a_Roche; many
  hot Jupiters are beyond 2 a_Roche but a population sits between 1 and
  2 a_Roche (Dawson & Johnson, p22). All observed hot Jupiters have
  a / a_Roche > 1 with f_p = 2.7 (p22).
- Host-mass scaling of the edge: if the edge is set by the magnetospheric
  cavity it scales as M_star^(1/7); if by the corotation radius with
  mass-independent stellar spin, as M_star^(1/3) (Dawson & Johnson, p24).
  The observed lack of trend in period against host mass is consistent with
  the M_star^(1/3) scaling (same page).
- **(derived)** P_Roche in Eq. 15 does not contain M_star: the Roche period
  depends on the planet's radius and mass only. At f_p = 2.7, 1.3 R_J and
  1 M_J it is 0.79 d, so an edge at 2 a_Roche is at 2^(3/2) x 0.79 = 2.2 d.
  With the 0.462 convention of Ford & Rasio (f_p = 1/0.462 = 2.16) the same
  planet has P_Roche = 0.79 x (2.16/2.7)^(3/2) = 0.57 d and an edge near
  1.6 d.

### 1.3 Eccentricities and masses

- Among 228 known hot Jupiters (m > 0.1 M_J) at 3 to 10 d, 31 have
  e < 0.1 at two sigma, 10 have e > 0.2 at two sigma, and the rest are
  intermediate or poorly constrained (Dawson & Johnson, p17). Moderately
  eccentric hot Jupiters are read as planets still circularising.
- The tidally circularised planets inside 0.1 AU are removed from eccentricity
  fits by Wright 2009 (p35) and Bryan 2016 (p20), and Rosenthal 2023 fits
  only planets beyond 0.3 au for the same reason (p4).
- Mass range: hot Jupiters "have lower masses than their more distant
  counterparts": a dearth of giants above ~3 M_J between the Roche limit and
  twice the Roche limit (Dawson & Johnson, p29). Hot-Jupiter definitions
  start at 0.1 M_J (Wright 2012) or 0.25 M_J (Dawson & Johnson); the Kepler
  sample of Steffen 2012 starts at radius 0.6 R_J (p2).
- Single giants have a hot-Jupiter pile-up (a < 0.06 au, ~2.3 sigma) that
  multi-giant systems lack; the median M sin i of singles is 0.92 M_J and of
  multis 1.71 M_J (Rosenthal 2023, abstract p1, Table 1 p9).

### 1.4 Close companions

- Steffen 2012: no transiting companion of 2/3 to 5 Earth radii (depending
  on the star's noise) around Kepler hot-Jupiter candidates, and no
  significant TTVs; hot Neptunes and warm Jupiters do show companions
  (abstract, p1). Detectable sizes for the exterior 2:1 resonance range from
  0.88 to 4.7 R_earth with a median of 2.0 R_earth (p4).
- Huang 2016: hot Jupiters (P < 10 d), "with the singular exception of
  WASP-47b", have no detectable companion with P < 50 d and size down to
  2 R_earth (inner companions down to 1 R_earth); half of the warm Jupiters
  (10 to 200 d) have small companions (abstract p1). Multiplicity rate of
  hot Jupiters 0.52 +5.0 -0.52%, of warm Jupiters 37.2 +18.6 -16.3%, with
  90% intervals (p3). Rates when only confirmed planets are used: 0.8% and
  58.1% (p3). 10 of 27 warm Jupiters are in multiples (p7).
- WASP-47b has a near 2:1 outer Neptune and an ultra-short-period super-Earth
  (Dawson & Johnson, p33), and may be the short-period tail of a class of
  resonant warm-Jupiter systems (same page).

### 1.5 Distant companions

- Knutson 2014: occurrence 51 +/- 10% for companions of 1 to 13 M_J at 1 to
  20 AU around transiting hot Jupiters; companion masses between 1 and
  500 M_J and semi-major axes mostly 1 to 75 AU for the detected
  accelerations (abstract, p1). No difference between hot Jupiters with
  misaligned or eccentric orbits and those with aligned circular ones (same).
- Bryan 2016: 52 +/- 5% for 1 to 20 M_J at 5 to 20 AU over all 123 systems
  (abstract, p1). By inner planet: 75.1 +4.4 -5.9% for inner planets inside
  0.1 AU, 48.8 +9.4 -9.5% for 0.1 to 1 AU, 53.7 +7.3 -8.2% for 1 to 5 AU
  (Fig. 19, p19); the hot bin is 2.4 sigma above the warm bin and is
  dominated by a transiting sample (p19). The frequency of giants declines
  beyond 3 to 10 AU (abstract, p1). Occurrence in stellar-mass bins
  0.08 to 0.8, 0.8 to 1.4, 1.4 to 2.1 M_sun: 56.0, 54.6, 51.6%, consistent
  at 0.2 sigma (Fig. 21, p20).
- Dawson & Johnson combine the two: 70 +/- 8% of hot Jupiters have an outer
  planet at 1 to 20 AU with 1 to 13 M_J (p32).

**For the draw (proposal).** A hot-Jupiter family could use: period drawn
log-uniform from just beyond the Roche edge, about 1.5 to 2 d (the 2 a_Roche
cutoff of Ford & Rasio, evaluated for the drawn radius and mass with the
formula of section 1.2), up to 10 d, with a mild weight toward 3 d; mass
0.25 to about 3 M_J (Dawson & Johnson definition and dearth above 3 M_J);
eccentricity near zero for P below ~3 d, with a tail toward 0.2 for 3 to
10 d (counts of section 1.3); no companion inside 50 d (Huang 2016) except as
a rare resonant-pair case modelled on WASP-47; an outer giant with
probability in the 50 to 75% range (Knutson 2014 and Bryan 2016), mass 1 to
13 M_J, separation 1 to 20 AU. The log-uniform and the 3-day weight are my
choice: no source gives the shape of the period density, and Dawson &
Johnson warn that the pile-up is not established.

Still missing: a numerical period density for hot Jupiters (Udry & Santos
not reached), a fitted eccentricity law for P below 10 d, and any source for
the fraction of hot Jupiters with a WASP-47-like inner companion beyond the
1.0 +9.5 -1.0% quoted by Huang (p7, derived by them for a 10% false-positive
rate).

## 2. Eccentric-giant family

### 2.1 Eccentricity distributions

| Sample | Beta(a, b) | Mean a/(a+b) (derived) | Location |
| --- | --- | --- | --- |
| 396 RV planets, K/sigma_K > 5, all periods | a = 0.867 +0.044 -0.044, b = 3.03 +0.17 -0.16 | 0.222 | Kipping 2013, Table 1, p4 |
| Same, split at the median period 382.3 d: long | a = 1.12 +0.11 -0.10, b = 3.09 +0.32 -0.29 | 0.266 | Kipping, Table 2, p5 |
| Same: short | a = 0.697 +0.066 -0.481, b = 3.27 +0.35 -0.32 | 0.176 | Kipping, Table 2, p5 |
| Same, one Beta for all (H1) | a = 0.711, b = 2.57 | 0.217 | Kipping, Table 2, p5 |
| Giants beyond 0.3 au, all | a = 0.85 +0.13 -0.11, b = 2.27 +0.36 -0.32 | 0.272 | Rosenthal 2023, p4 |
| Single giants beyond 0.3 au | a = 0.60 +0.14 -0.12, b = 1.45 +0.34 -0.30 | 0.293 | Rosenthal 2023, p4, Table 1 p9 |
| Multiple giants beyond 0.3 au | a = 1.06 +0.21 -0.18, b = 3.68 +0.81 -0.70 | 0.224 | Rosenthal 2023, p4, Table 1 p9 |

- Kipping: the Beta model is favoured over Rayleigh plus exponential by a
  Bayes odds ratio of 3.7 (p3); the two-population split is favoured at
  11.6 sigma (p5); short-period planets have more low-eccentricity members,
  long-period ones a flatter distribution (p5). Planets with a < 1 in Beta
  have a density diverging at e = 0 (standard property of the Beta law,
  not a quotation).
- Rosenthal 2023, Table 1 (p9): singles have e10 = 0.03, e50 = 0.20,
  e90 = 0.77; multis e10 = 0.03, e50 = 0.23, e90 = 0.47. Singles show "a
  pile-up of nearly circular orbits and a long tail"; multi-giant systems
  have a distribution that "extends to e = 0.47 (90th percentile)"
  (abstract, p1). Of 50 single giants beyond 0.3 au, 11 have median
  e above 0.5 (p10).
- Bryan 2016: planets with an outer companion (two-planet systems)
  have higher eccentricities than singles, at more than 3 sigma (p20); this
  contradicts earlier work finding lower e in multis (Wright 2009,
  Limbach & Turner), which Bryan attributes to their lower sensitivity to
  distant companions (p20 to p21). Wright 2009 abstract: planets in
  multiple systems have "somewhat smaller eccentricities" (p2). The two
  findings disagree; Rosenthal 2023 (larger sample, hierarchical fit) finds
  similar medians and a shorter tail for multis.
- Old-sample statements: "over half of the extrasolar planets beyond 0.1 AU
  have e >= 0.3, and two have e > 0.9" (Ford & Rasio 2008, p1); the median
  eccentricity of known planets is about 0.2 and the largest is 0.93,
  HD 80606b (Juric & Tremaine, p1).
- Zhu & Wu: "cold Jupiters typically have significant eccentricities
  (e ~ 0.3)" (p7), cold Jupiter meaning M > 0.3 M_J and a > 1 au (abstract,
  p1).

### 2.2 Mass-eccentricity trend

- Wright 2009 (p35, Section 3.7, a > 0.1 AU only): mean e of 0.19 for
  M sin i below 1.0 M_J and 0.34 above; sub-Jupiters peak at e < 0.1,
  super-Jupiters are spread over 0.0 to 0.6; K-S test p(D+) < 0.1%.
- Rosenthal 2023: multi-giant systems have higher median mass (1.71 vs
  0.92 M_J) and masses of giants in one system are correlated (abstract, p1;
  Pearson R = 0.45 for pairs, p8).

### 2.3 Planet-planet scattering outcomes

- Juric & Tremaine 2008: ~3250 integrations of random systems, 100 Myr; the
  final distribution of dynamically active systems is
  dn proportional to e exp[-(1/2)(e/0.3)^2] de, a Rayleigh law with
  sigma_e = 0.3 (abstract p1; Eq. 2, p3; the e > 0.2 part fits observations,
  too few low-e planets are produced). Medians of active ensembles are
  0.35 to 0.4 (p5). **(derived)** For a Rayleigh law the median is
  sigma sqrt(2 ln 2) = 0.35 and the mean sigma sqrt(pi/2) = 0.38.
- Juric & Tremaine: the median mutual-Hill separation D_H of surviving
  neighbours converges to 12 to 14 for active and partially active
  ensembles (p16), and all neighbours are more than 4 R_H apart at 100 Myr
  (p14). Active systems ejected on average 50% of the initial mass when they
  started with 10 planets (p18). About 80% of planets excited to e > 0.6 are
  removed by 100 Myr (p10).
- Ford & Rasio 2008: two-planet scattering from nearly circular orbits gives
  a maximum eccentricity "of about 0.8, independent of the distribution of
  planet mass ratios" (abstract p1, p5); the median final e after ejection
  follows e_f = 1.44 beta_f^1.23 with beta_f = m_ejected / (m1 + m2)
  (p8), 0.62 for equal masses (p8). **(derived)** e_f = 0.20 for
  beta_f = 0.2 and 0.085 for 0.1. They adopt a Keck mass power-law index of
  0.31 +/- 0.20 from Cumming 2008 (p7).
- Stargrazers: the 0.8 limit applies only after the other planet is
  ejected; smaller pericentres occur earlier (Ford & Rasio 2008, p5).

### 2.4 Effect of giants on inner small planets

| Finding | Number | Location |
| --- | --- | --- |
| Cold Jupiters (M > 0.3 M_J, a > 1 au) and inner super-Earths (a < 1 au) | cold Jupiters appear three times more often around super-Earth hosts; P(CJ given SE) about 30%, about 60% for [Fe/H] > 0.1 | Zhu & Wu, abstract p1, p7 |
| Cold Jupiters without super-Earths | "rare (~1%)" | Zhu & Wu, abstract p1 |
| Inner multiplicity with and without a Kepler cold Jupiter | 1.4 vs 2.5 planets within 400 d | Zhu & Wu, p7 |
| Outer giants of 0.5 to 20 M_J at 1 to 20 AU in super-Earth systems | 39 +/- 7% | Bryan 2019, abstract p1 |
| Super-Earth hosts with a gas giant, metal-rich | 28.0 +4.9 -4.6%; metal-poor 4.5 +2.6 -1.9% | Bryan & Lee 2024, abstract p1 |
| Correlation stronger for giants beyond 3 AU, with e > 0.2, or in multi-giant systems (metal-rich hosts only) | qualitative | Bryan & Lee 2024, abstract p1 |
| Mutual inclination of cold Jupiter vs inner super-Earths | Rayleigh scale sigma = 11.8 +12.7 -5.5 deg (68%), sigma > 3.5 deg (95%); Solar System invariable-plane mean 1.8 deg | Masuda 2020, abstract p1 |
| Inner three-plus-planet systems with an outer giant (50 M_earth to 13 M_J) | all 4 have gap complexity above 0.32, higher than 79% (19 of 24) of those without (median 0.06) | He & Weiss 2023, abstract p1 |
| Stability limit for inner pairs | governed by the parameter epsilon_12, the perturber-driven precession over the mutual precession; loose systems (epsilon_12 above 1) are more susceptible, tight ones less | Pu & Lai 2018, abstract p1 |

- No source read gives a threshold eccentricity above which an outer giant
  destroys inner planets. Pu & Lai 2018 give the dependence on perturber
  mass, semi-major axis and inclination, and state that instability sets in
  for "large enough epsilon_12 and e_p" (p11), with a stability criterion for
  the inner pair taken from Petrovich 2015 (their Eq. 57, equation not
  readable here). Zhu & Wu argue that giants with e ~ 0.3 should disturb
  packed inner systems (p7). Bryan 2019 conclude outer giants do not
  suppress inner super-Earths (abstract p1), and Bryan & Lee 2024 find a
  positive correlation for metal-rich hosts that grows for e > 0.2.
  The observations therefore support "many eccentric giants have inner
  small planets", and are silent on a cut-off.

**For the draw (proposal).** Giant eccentricity: draw from a Beta law. The
long-period Beta(1.12, 3.09) of Kipping (mean 0.27) or the single-giant
Beta(0.60, 1.45) of Rosenthal (e90 = 0.77) fits an "eccentric giant" family;
a scattering-based alternative is a Rayleigh with sigma = 0.3 (Juric &
Tremaine) truncated at about 0.8 for a two-planet system after an ejection
(Ford & Rasio). For two giants, use the multi-giant Beta(1.06, 3.68)
(e90 = 0.47) for the survivors; mass favours the upper end (median 1.7 M_J
for multis, Rosenthal). Inner planets: "few" rather than none, in line with
Zhu & Wu (fewer inner planets with a cold Jupiter: 1.4 vs 2.5). A rule
"inner planets only if the giant's pericentre stays outside the inner
system by a stability margin" could use the Hill spacing of the other agent;
no number for it comes from this study. Mutual inclination of the giant to the
inner system: Rayleigh sigma about 12 deg (Masuda).

Still missing: a threshold e or pericentre ratio from a full numerical
survey (Huang et al. 2017 and Mustill et al. 2017 are cited by Zhu & Wu but
were not read), and the fraction of eccentric giants with inner companions.

## 3. Solar-like family

### 3.1 Where cold giants sit

| Result | Value | Location |
| --- | --- | --- |
| Break in giant occurrence vs period, RV, 0.1 to 20 M_J, 1 to 10000 d | break at 1766 d (~2.8 au); probability 0.17% that a single power law describes the data (Davies test) | Fernandes 2019, p5 |
| Asymmetric broken power law | P_break 1285 to 2149 d (2.3 to 3.2 au); index before 0.53 +/- 0.09, after -1.22 +/- 0.47 | Fernandes 2019, p5 |
| Symmetric broken power law | P_break 698 to 1020 d (1.5 to 2.0 au); p1 = -p2 = 0.63 +/- 0.11 | Fernandes 2019, p5 |
| Log-normal fit | P_break 814 to 1024 d | Fernandes 2019, p5 |
| Snow line in disks of young Sun-like stars | 2 to 5 au; Solar System about 2.5 au at planetesimal formation | Fernandes 2019, p1 (citing Mulders 2015, DeMeo & Carry 2014) |
| Giant occurrence, 0.03 to 30 au | higher at 1 to 10 au; increase near 1 au by a factor ~4 | Fulton 2021, abstract p1 |
| Broken power law in a | a0 = 3.6 +2.0 -1.8 au, index beyond the break beta = -0.86 +/- 0.41, index inside beta + gamma with gamma = 1.59 +0.36 -0.33 (so about 0.73) | Fulton 2021, Table 1, p6 |
| Plateau or decline beyond the peak | 99.4% of posterior samples | Fulton 2021, p5 |
| Slope inside the break | 0.72 +0.16 -0.20, versus 0.26 +/- 0.1 of Cumming 2008 | Fulton 2021, p5 |
| Occurrence 2 to 8 au; 8 to 32 au | 14.1 +2.0 -1.8; 8.9 +3.0 -2.4 per 100 stars (1.5 sigma fall-off) | Fulton 2021, abstract p1, p6 |
| Period distribution of Keck giants (M > 0.3 M_J, P < 2000 d) | flat in log P with dN/dlog10 P = 1.3 +/- 0.4% below ~300 d and 6.5 +/- 1.4% above, a factor ~5 | Cumming 2008, p12 |
| Occurrence versus Cumming fit | dN = C M^alpha P^beta dlnM dlnP, beta = 0.26 +/- 0.1 | Cumming 2008, abstract p1 |
| Declining frequency beyond | 3 to 10 AU (companions in known systems) | Bryan 2016, abstract p1 |
| Giants more frequent at | 1 to 10 au than inside or outside | Fulton 2021, abstract p1 |
| Jump near 1 AU in the a distribution of single-planet systems; ice line quoted "near 3 AU for solar-type stars" | no fit, no number beyond these | Wright 2009, p24 to p25, p9 |

Wright 2009 attributes the jump near 1 AU to the ice line or a dead zone and
does not fit it (p25). Fernandes and Fulton disagree on the decline after
the peak: Fulton's index beyond the break (-0.86) is shallower than
Fernandes's asymmetric one (-1.22), and Fulton calls the difference
"potentially discrepant" with the flat prediction of Wittenmyer 2020 (p5 to
p6).

### 3.2 Giant mass function

- Cumming 2008: for M > 0.3 M_J and P < 2000 d, alpha = -0.31 +/- 0.2 in
  dN = C M^alpha P^beta dlnM dlnP (abstract, p1). **(derived)**
  dN/dM is proportional to M^(-1.31).
- Fulton 2021: occurrence is constant in ln(M sin i) within 30 to 900 M_earth
  at 1 to 5 au and "falls off with ~2 sigma significance above 900 M_earth"
  (p6; Fig. 6 caption, p7). **(derived)** 900 M_earth = 2.83 M_J with
  1 M_J = 317.8 M_earth (NASA fact sheet, ratio table), and 30 M_earth is
  0.094 M_J. Sub-Jovian planets (0.1 to 1 M_J) are enhanced at 1 to 10 au
  just as Jovians are (abstract, p1).
- Giant occurrence grows with host mass above about 1 M_sun and with
  [Fe/H] (Fulton 2021, p7).
- Rosenthal 2023: giants in multi-giant systems have a median M sin i of
  1.71 M_J against 0.92 M_J for singles; the mean is 2.88 +/- 0.42 against
  2.07 +/- 0.33 M_J (Table 1, p9).

### 3.3 Adjacent giants: spacing and eccentricity

- Observed pairs: "the majority of CLS-detected giant planet pairs have a
  semi-major axis ratio less than 10, with a peak ratio frequency around 3"
  (Rosenthal 2023, p10; Fig. 13 shows the kernel density of a2/a1 for
  two-giant systems on a 3 to 100 axis). **(derived)** a ratio of 3 is a
  period ratio of 3^1.5 = 5.2.
- Rosenthal's synthetic pairs use a minimum ln(a2/a1) of 0.5 "to prevent
  unlikely or unphysical draws of pairs within 3:2 resonance" (p8). This
  is their sampling rule, not a measurement. **(derived)** exp(0.5) = 1.65.
- Dynamical floor after scattering: Juric & Tremaine find all neighbours more
  than 4 mutual Hill radii apart after 100 Myr of active evolution (p14),
  with a median of 12 to 14 (p16); the Solar System has a median
  separation of 15.6 (four giants) or 21.2 (all eight) mutual Hill radii,
  so it is "inactive" in their classification (p18).
- Core spacing: Kennedy & Kenyon space cores by 8 R_H at isolation, giving
  about 4 cores over 6 to 11 AU for the Sun (p5).
- Solar System giants (NASA fact sheet, ratio-to-Earth table; the ratio to
  Earth is read as AU and years, **derived**): a = 5.20, 9.57, 19.17,
  30.18 au; periods 11.9, 29.4, 83.7, 163.7 yr (days: 4331, 10747, 30589,
  59800). **(derived)** a ratios: 1.84 (Jupiter-Saturn), 2.00
  (Saturn-Uranus), 1.57 (Uranus-Neptune); period ratios 2.48, 2.85, 1.96.
  Eccentricities 0.049, 0.052, 0.047, 0.010 (metric table). Masses in
  M_earth 317.8, 95.2, 14.5, 17.1.
- Eccentricity in multi-giant systems: see section 2.1; multis have e50 =
  0.23 and e90 = 0.47 against singles with 0.20 and 0.77 (Rosenthal 2023,
  Table 1, p9). Solar System giants are below 0.06.

### 3.4 Ice-line location

- No source read gives a first-principles formula by Hayashi 1981 (not
  reached). Kennedy & Kenyon quote the "canonical" a_snow = 2.7 M_star/M_sun
  AU of Ida & Lin 2005 (Fig. 1 caption, p4), and also the simplification
  a_snow proportional to M_star^2 from L proportional to M_star^4 used by an
  earlier study (p3), which they find "too close ... (far from)" the star
  for masses below (above) one solar mass when protoplanets form.
- Their own model locates the snow line where the midplane temperature is
  170 K, with irradiation temperature
  T_irr = T_star (alpha/2)^(1/4) (R_star/a)^(3/4) and
  alpha = 0.005/a_AU + 0.05 a_AU^(2/7) (Eq. 4, p3), plus viscous heating.
  The snow line crosses 2.7 AU at 5e5 yr for the Sun (p3). The result is
  that the snow line "changes weakly with stellar mass" (abstract, p1).
- Where cores form: for the Sun the inner edge of the core-forming region is
  the snow line at ~6 AU, and the region extends from 6 to 11 AU (p5). For a
  3 M_sun star the innermost core forms just inside the snow line at ~8 AU
  and the outermost at ~20 AU (p6). Stars above 3 M_sun push the snow line to
  10 to 15 AU before protoplanets form (abstract, p1).

**For the draw (proposal).** Giant semi-major axes: log-uniform or Fulton's
broken power law with a0 about 3 au (Sun-like), index about +0.7 inside and
-0.9 outside (mean values of Table 1). Scale the break with the snow line of
the host (section 3.4, section 4.3). Masses: log-uniform in mass from
0.1 M_J to about 3 M_J (Fulton, uniform in ln M up to 900 M_earth), with a
power-law decline above if desired (Cumming alpha = -0.31 is the shallow
alternative). Several giants: first giant drawn, then neighbours with
a2/a1 from a log-normal-like law peaking at 3 and mostly below 10
(Rosenthal), with a floor of about 4 mutual Hill radii (Juric & Tremaine) or
the factor-of-two spacing of the Solar System as a gentler example (period
ratios 2 to 3 derived above). Eccentricities of the giants: low (below
~0.05) for a Solar System analogue; the multi-giant Beta of section 2.1 for
a broader family. Terrestrial planets inside the snow line follow the
other agent's compact-system rules.

Still missing: a source for a mean period ratio of adjacent giants for the
whole observed population (the CLS plot is read only through its caption and
text), a fitted formula for the ice-line location other than the Ida and
Lin linear form, and a measured dependence of giant mass on separation.

## 4. Bounds common to giants

### 4.1 Upper mass bound

- Deuterium burning: "13 M_J is generally a reasonable rule of thumb";
  50% of the initial deuterium burns at (13.0 +/- 0.8) M_J for most
  proto-brown dwarf conditions; over models the limit ranges from
  ~11.0 M_J (3 times solar metallicity, 10% burned) to ~16.3 M_J (zero
  metallicity, 90% burned) (Spiegel 2011, abstract, p1). The authors do not
  endorse it as the planet boundary.
- Surveys use 13 M_J as an upper limit: Knutson's 1 to 13 M_J (p1),
  He & Weiss' 13 M_J (abstract, p1), Dawson & Johnson's 1 to 13 M_J (p32).
  Fernandes 2019 uses 0.1 to 20 M_J (abstract, p1), Bryan 2016 and Bryan 2019
  1 to 20 M_J and 0.5 to 20 M_J, Rosenthal 2023 an upper edge of 6000 M_earth
  in its synthetic pairs (p8, **derived** 18.9 M_J).

### 4.2 Lower mass bound of a giant

| Survey | Lower bound | Location |
| --- | --- | --- |
| CLS IV (Rosenthal 2023) | 0.1 M_J or 30 M_earth: roughly the minimum mass at which runaway gas accretion can begin, near the threshold for envelope-dominated composition | p2 |
| Zhu & Wu 2018 | 0.3 M_J | abstract, p1 |
| Wright 2012 | 0.1 M_J | p2 |
| Dawson & Johnson 2018 | 0.25 M_J (0.83 Saturn masses) for hot Jupiters | p2 |
| Cumming 2008 | 0.3 M_J for the power-law fit | abstract, p1 |
| Fernandes 2019 | 0.1 to 20 M_J (radii 5 to 20 R_earth in Kepler) | abstract p1, p3 |
| Fulton 2021 | sub-Jovian planets 0.1 to 1 M_J studied separately; giants above 100 M_earth for the host-mass analysis | abstract p1, p7 |
| He & Weiss 2023 | 50 M_earth | abstract, p1 |

**(derived)** 100 M_earth = 0.31 M_J and 50 M_earth = 0.16 M_J with
1 M_J = 317.8 M_earth.

### 4.3 Host mass

- Johnson 2010: the fraction of stars with a giant planet scales as
  f proportional to M_star, from 3% at 0.5 M_sun to 14% at 2 M_sun at solar
  metallicity; stellar sample 0.2 to 1.9 M_sun; metallicity f proportional to
  10^(1.2 [Fe/H]) (abstract, p1). Their sample is limited to K above
  20 m/s and a < 2.5 AU (p3); the "detectable planets" of an earlier study
  they cite were M sin i above 0.8 M_J and a < 2.5 AU (footnote, p2). A later Bayesian
  reanalysis argued the trend is not firmly established (found only through
  search results, not read): treat as a plausibility range.
- Kennedy & Kenyon: the probability of at least one gas giant rises linearly
  with stellar mass from 0.4 to 3 M_sun; if 6% of solar-mass stars have a
  gas giant, 1% for 0.4 M_sun and 10% for 1.5 M_sun (abstract, p1).
- Fulton 2021: occurrence rises beyond about 1 M_sun (p7). Dawson & Johnson:
  giant occurrence "peaking at 2 M_sun" after Reffert 2015 (p30); hot
  Jupiters rarer around M dwarfs (p2). Bryan 2016: companions beyond 5 AU
  have the same occurrence, within large errors, for 0.08 to 0.8, 0.8 to
  1.4 and 1.4 to 2.1 M_sun (Fig. 21, p20).
- Separation versus host mass: Wright 2009 find a lack of close-in planets
  around stars above 1.5 M_sun and a lack of long-period planets around
  stars below 0.5 M_sun, and read this as a positive correlation of giant
  semi-major axis with stellar mass (Section 3.12, p47, abstract p2),
  flagged as "emerging" and possibly a selection effect for M dwarfs.
  Dawson & Johnson: the inner edge scales as M_star^(1/3) or M_star^(1/7)
  (section 1.2).
- Snow line scaling: see section 3.4. With the Ida & Lin linear form
  **(derived)** a_snow = 0.22 AU at 0.08 M_sun and 21.6 AU at 8 M_sun,
  while the quadratic form a proportional to M_star^2 normalised at 2.7 AU
  gives 0.017 AU and 173 AU, which Kennedy & Kenyon reject; their model
  keeps the snow line weakly dependent on M_star and puts it at 10 to 15 AU
  for stars above 3 M_sun. None of these was fitted down to 0.08 M_sun in
  the sources read.

**For the draw (proposal).** Upper mass limit 13 M_J for the planet class
(Spiegel). Lower edge 0.1 M_J (30 M_earth) as the "giant" boundary, which
agrees with the CLS convention and with the 0.1 M_J of the hot-Jupiter
sources. Host mass: give every family the same shape and scale the ice
line, using a_snow = 2.7 AU times M_star/M_sun as a first-order guess, or
the weaker Kennedy and Kenyon behaviour; apply a plausibility prior that
makes giants about 2 to 5 times rarer for 0.1 to 0.5 M_sun (Johnson 3% at
0.5 M_sun, Kennedy 1% at 0.4 M_sun) and no stronger tilt above, since no
source shows a systematic change of the occurrence shape.

Still missing: any measurement of giants below 0.2 M_sun (Johnson's sample
starts at 0.2 M_sun), the luminosity scaling of the ice line for the main
sequence from a primary source (Hayashi not reached), and giants around
B stars.

## 5. What this study does not settle

- Period density of hot Jupiters below 3 d, the pile-up amplitude, and a
  numerical Udry and Santos distribution (not reached).
- A threshold for what an eccentric giant does to inner planets, beyond the
  qualitative statements and the stability parameter of Pu & Lai.
- A direct source for mean spacing between adjacent giants over the whole
  population, except the Rosenthal peak near 3 in a2/a1.
- The ice-line formula from first principles; only the Ida and Lin linear
  form, quoted by Kennedy and Kenyon, and the 170 K definition are sourced.
- Occurrence for hosts below 0.2 or above 2 M_sun.
