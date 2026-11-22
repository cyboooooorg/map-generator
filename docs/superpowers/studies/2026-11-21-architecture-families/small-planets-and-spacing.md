# Literature study: small planets, spacing and dynamics of architectures

Date: 2026-11-21. Rule followed: a number is listed only if it was read in a
source text during this study. The reading channel is stated for each source:

- **Full text**: the PDF was downloaded and text-extracted, and the passage was
  read.
- **Abstract (search)**: only a search-engine rendering of the abstract was
  read. Treat as unverified against the paper itself.
- **Second-hand**: the number is quoted by a full-text source that was read,
  not by the original, which was not opened. The citing page is given.
- **Not reached**: the paper could not be opened. Nothing is claimed from it.

Page numbers `pN` are PDF pages of the arXiv file that was read. Numbers
marked **(derived)** are my arithmetic on sourced inputs, not quoted. Scope:
the small-planet families of the system draw (compact, solar-like inner
planets, sparse) and the dynamics of spacing. Giant planets are covered by
another study. Small-planet masses are covered by
`2026-11-21-planet-interior/heat-and-composition.md`, section B9 (Neil &
Rogers 2020, Wu 2019): cited in section 7, not redone.

Sources used, with the channel (all full texts were extracted whole and read
in the sections named in the text):

| Short name | Reference | Channel |
| --- | --- | --- |
| Weiss 2018 | Weiss et al., AJ 155, 48, arXiv:1706.06204 | Full text |
| Fabrycky 2014 | Fabrycky et al., ApJ 790, 146, arXiv:1202.6328 | Full text |
| Pu & Wu 2015 | Pu & Wu, ApJ 807, 44, arXiv:1502.05449 | Full text |
| Fang & Margot 2013 | ApJ 767, 115, arXiv:1302.7190 | Full text |
| Petit 2020 | Petit et al., A&A 641, A176, arXiv:2006.14903 | Full text |
| He 2019 | He, Ford & Ragozzine, MNRAS 490, 4575, arXiv:1907.07773 | Full text |
| He 2020 (II) | He, Ford & Ragozzine, arXiv:2003.04348 | Full text (abstract) |
| He 2020 (III) | He, Ford & Ragozzine, arXiv:2007.14473 | Full text |
| EPOS I | Mulders et al., AJ, arXiv:1805.08211 | Full text |
| Mulders 2015 | Mulders, Pascucci & Apai, ApJ 798, 112, arXiv:1406.7356 | Full text |
| Lee & Chiang 2017 | Lee & Chiang, arXiv:1702.08461 | Full text |
| Millholland 2017 | Millholland, Wang & Laughlin, ApJL 849, L33, arXiv:1710.11152 | Full text |
| Steffen & Hwang 2015 | MNRAS, arXiv:1409.3320 | Full text |
| Van Eylen 2019 | Van Eylen et al., AJ 157, 61, arXiv:1807.00549 | Full text |
| Mills 2019 | Mills et al. (CKS VIII), AJ 157, 198, arXiv:1905.04625 | Full text |
| Xie 2016 | Xie et al., PNAS 113, 11431, arXiv:1609.08633 | Full text |
| Ballard & Johnson 2016 | ApJ 816, 66, arXiv:1410.4192 | Full text |
| Rappaport 2013 | Rappaport et al., ApJL 773, L15, arXiv:1307.4080 | Full text |
| Sanchis-Ojeda 2014 | ApJ 787, 47, arXiv:1403.2379 | Full text |
| Raymond 2006 | Raymond, Quinn & Lunine, Icarus, arXiv:astro-ph/0510284 | Full text |
| Neil & Rogers 2020, Wu 2019 | see B9 of the planet-interior study | Cited, not redone |
| Gladman 1993 | Icarus 106, 247 | Not reached (second-hand) |
| Chambers, Wetherill & Boss 1996 | Icarus 119, 261 | Not reached (second-hand) |
| Chambers 2001 | Icarus 152, 205 | Not reached |
| Deck 2013 | Deck, Payne & Holman, ApJ 774, 129 | Not reached (second-hand) |

## 1. Compact family ("peas in a pod")

### 1.1 Number of planets per compact system

- He 2019 abstract (p1): 0.56 (+0.18/-0.15) of FGK stars have at least one
  planet larger than 0.5 Earth radii between 3 and 300 d. About 98 % of
  these systems consist of one or two clusters, with a median of three
  planets per cluster.
- He 2019, clustered periods and sizes model, Table 5 (p16, best fit KS):
  lambda_c = 0.90 (+0.50/-0.26) clusters per star (about 80 % of systems with
  planets have one cluster, p14); lambda_p = 3.86 (+1.67/-1.38), the
  zero-truncated Poisson parameter of planets per cluster, "peaks at four"
  (p14). Model caps: at most 10 clusters and 15 planets per cluster (p4).
- He 2019 p14: overall roughly 3.5 planets per star in that model (AD
  distances: about 2.6).
- EPOS I abstract (p1): at least 42 % of Sun-like stars have nearly coplanar
  systems with 7 or more planets. The number per system was fixed at m = 10
  in the fit because it is not constrained; "systems with fewer than 6
  planets do not reproduce the observed multiplicity distribution" because
  two sextuple systems are detected (p12). Best-fit fraction of stars with
  such systems: eta_s = 0.67 (+0.17/-0.12) (Table 2, p12).
- Ballard & Johnson 2016 abstract (p1), M dwarfs: the observed 2-5 transiting
  planet counts are recovered if every M dwarf hosts 6.1 +/- 1.9 planets,
  mutual inclination 2.0 (+4.0/-2.0) degrees.
- Pu & Wu 2015 (p1): in the highest multiplicity systems (4-6 transiting) the
  spacing is so tight that "of order 10 planets" could fit inside Mercury's
  orbit. Their simulated average system (P_in about 5 d, 5 Earth masses)
  holds "roughly 7 planets (ranging from 6-10)" inward of 200 d (p7).

**For the draw** (proposal): the family definition of 4-7 planets lies
inside every sourced range above (EPOS 7-10, Ballard 6.1 +/- 1.9, He 3-4
per cluster with one or two clusters). A uniform integer 4-7 is a
`Definition:`; the sourced shape is a peak at about 4 per cluster
(He 2019) and a tail to 7-10 (EPOS). No source gives a per-host-mass
multiplicity except Ballard (M dwarfs, 6.1) and He 2020 (II) (same
architecture across spectral types, section 1.2).

### 1.2 Innermost planet, inner edge and host-mass scaling

- Mulders 2015 abstract (p1): occurrence of 1-4 Earth-radius planets rises to
  a break at about 10 d and plateaus outside. The semi-major axis of the
  cutoff scales as M^(1/3). Table 2 (p7): co-rotation M^(1/3);
  sublimation M^0.7 (or M^(11/9), M^(7/9) with assumptions); stellar tides
  M^(9/13); planetary tides M^(3/13). The M^(1/3) index matches the
  pre-main-sequence co-rotation radius and, less well, planetary tides
  (conclusion, p11).
- Mulders 2015 p7: curves of M, K, G stars match F stars when the
  semi-major axis is shifted by factors 1.6, 1.4 and 1.2 (planets extend
  closer in around lower-mass stars). Median masses of the two samples
  compared in Fig. 3: 0.47 (cool stars, Teff < 4000 K) and 0.95 solar masses
  (all Kepler, p5).
- **(derived)** From Kepler's third law, a cutoff scaling as M^(1/3) in
  semi-major axis is a constant period: P = 2 pi sqrt(a^3 / (G M)) gives
  P independent of M. A 10 d period gives a = 0.039, 0.053, 0.072, 0.091,
  0.114 and 0.182 AU for 0.08, 0.2, 0.5, 1, 2 and 8 solar masses.
- Lee & Chiang 2017 (p1): sub-Neptune occurrence is flat in log P down to
  about 10 d, then dN/dlogP is proportional to P^1.5 inside, "regardless of
  whether the host star is of type FGK or early M". Fig. 1 (p2): slopes
  1.5 +/- 0.3 (FGK) and 1.4 +/- 0.2 (M dwarf); break labelled
  a_break about 0.13 AU for 1.0 and 0.11 AU for 0.5 solar masses. The break
  is attributed to disk truncation at co-rotation, drawn from pre-main-
  sequence rotation periods peaking near 10 d (p2). They predict, without
  testing, a break near 1 d for A stars (abstract, p1).
- EPOS I Table 2 (p12): the innermost planet of multi-planet systems peaks at
  P_in = 12 (+3/-2) d, with power-law index 1.6 (+0.4/-0.2) towards shorter
  periods and -0.9 (+0.4/-0.5) towards longer ones. Text p14: "12 +/- 2 days".
  Abstract (p1): fewer than about 8 % and 3 % of systems have no planet
  inside Mercury's and Venus's orbits, respectively.
- Millholland 2017 p5 (systems with 3 or more planets, 128 systems):
  the inner planet's semi-major axis correlates with stellar mass,
  beta = 0.069 AU per solar mass (p < 1e-5), with metallicity
  beta = -0.057 AU/dex; the two explain about 22 % of the variation
  (R^2 = 0.22).
- Pu & Wu 2015 p7: the inner-planet period of their model systems is drawn
  from a Rayleigh distribution with scale 5 d, described as a good
  representation of the sample.
- He 2020 (II) abstract (p1): fraction of stars with planets (0.5-10 Earth
  radii, 3-300 d) rises from 0.32 (+0.12/-0.11) for F2V to 0.96
  (+0.04/-0.19) for mid-K dwarfs, 0.57 (+0.14/-0.10) for G2V; multiplicity
  distributions of the bluer and redder halves are both matched by one
  architecture "aside from a shift in the overall fraction".
- The shortest observed periods (hours, not days) are in section 6.

**For the draw** (proposal): place the innermost compact planet at a period
drawn around 10-12 d with a tail to shorter periods (index 1.5-1.6 below
the peak, EPOS I / Lee & Chiang), and a hard floor from section 6. For
the host, the sourced statement is "constant period" (derived from
Mulders 2015) or the shallower stellar-mass trend of Millholland (0.069 AU
per solar mass, inner planet of 3+ planet systems). The two are not the
same law and both cover only about 0.2-1.4 solar masses (Kepler FGK-M).
Beyond that range (A stars, brown-dwarf-like hosts) only Lee & Chiang's
untested prediction exists. A family is dropped when its innermost slot
falls inside the Roche floor of section 6 or the star's surface, not at
the 10 d edge, which is a statistical break, not a limit.

Missing: no source measured the inner edge for hosts above about 1.4 or
below 0.2 solar masses; the 0.13 AU / 0.11 AU labels of Lee & Chiang's Fig. 1
do not equal a 10 d orbit by Kepler's law for 1.0 solar mass (0.091 AU,
derived above), so treat them as plot annotations, not a calibration.

### 1.3 Outer extent

- No source read gives the outer edge of the compact population as a
  result. What exists are detection windows: Kepler CKS multis reach at
  most 1071 d (Weiss 2018 p6); He 2019 fixes its period window at 3-300 d
  (eq. 2, p4); Pu & Wu 2015 compare within 200 d (p7).
- Weiss 2018 p5: for the adjacent-pair analysis only period ratios below 4
  were used ("somewhat compact systems"), justified in their section 5.1.
- Steffen & Hwang 2015 abstract (p1): beyond period ratio about 2.5 the
  distribution of all pairs follows a power law with exponent
  -1.26 +/- 0.05 (p8).

**For the draw** (proposal): the "about 0.3 AU" outer bound of the spec is
not a sourced number. It can be restated as a consequence of the sourced
period-ratio shape: with a first planet near 10 d and 4-7 planets at
typical ratios of 1.5-2 (section 1.5) the last planet sits at roughly 35-
640 d **(derived: 10 d x ratio^(N-1), ratio 1.5-2, N = 4-7)**, which is
0.2-1.4 AU at one solar mass. Missing: the source for a cutoff.

**Now used by the draw:** the chain ends at 1071 d, constant in days
for every host (Weiss 2018 § IV step 5, #630).

### 1.4 Size and mass similarity of neighbours

- Weiss 2018, 909 planets in 355 CKS multis (p2). Radius of neighbour
  pairs: Pearson r = 0.65, p < 1e-5, 504 detectability-corrected pairs
  (p3); ratio R_(j+1)/R_j has mean 1.29, median 1.14, standard deviation
  0.63 (p5); the outer planet is larger in 65.4 +/- 0.4 % of pairs (p4,
  abstract p1).
- Weiss 2018 abstract (p1), p9: adjacent period ratios are correlated
  within a system (r = 0.46, p < 1e-5, p6) and smaller in systems with
  smaller planets (r = 0.26 between pair mean radius and ratio, p8);
  sub-Earths (under 1 Earth radius) have typical ratios of 1.2-2.0 (p9).
- Millholland 2017 (37 systems, 89 planets with TTV masses, p2): intra-system
  dispersion of mass and radius is smaller than shuffled controls by 8.3
  sigma jointly, 5.3 sigma (radius) and 7.6 sigma (mass) (p4). Stellar mass
  and metallicity explain about 20 % of the variation (abstract, p1). Caveat
  from the search result only, not read: Zhu 2019 disputes the significance
  (Abstract (search); the paper arXiv:1907.02074 was not opened).
- He 2019 Table 5 (p16): planet radii in a cluster are drawn lognormally
  around a cluster radius with scale sigma_R = 0.31 +/- 0.07 (KS), 0.32 (AD)
  (eq. 8, p4); period scale per planet sigma_P = 0.21
  (+0.04/-0.04), and the cluster period spread is the lognormal scale
  N_p sigma_P (eq. 4, p4). He 2020 (III) Table 4 (p19): sigma_R 0.33 / 0.32,
  sigma_P 0.20 / 0.18 (two-Rayleigh model).
- EPOS I assumed equal sizes inside a system (p5).

**For the draw** (proposal): draw one cluster radius and scatter neighbours
with a log-scale about 0.3 (He 2019), or use the observed ratio shape of
Weiss (median 1.14, mean 1.29, outer larger 65 %). Masses follow from the
mass-radius relation or, for similarity, a per-system draw from B9 with a
per-planet scatter; Millholland gives only significance, not a scatter
value. Missing: a numeric intra-system mass scatter.

### 1.5 Period ratios of adjacent pairs and near-resonance fraction

- Fabrycky 2014 (365 multis, 899 candidates, abstract p1):
  - "apparent cut-off interior to the 5:4 resonance" (ratio 1.25) in the
    histogram of all pairs (p7); Kepler-50 sits at 1.20010 +/- 0.00003
    (6:5) and KOI-1665 at 1.17219 (p4); the region is not empty.
  - global mode "just wide of the 3:2 resonance" and a smaller peak wide of
    2:1, with a deficit just narrow of both (p7); a dense cluster spans
    1.505-1.520 for planets under 3 Earth radii (p7).
  - K-S test against log-uniform: p = 1.4e-3 for the |zeta_1| distribution
    (planets prefer first-order resonances), but near 2:1 alone p = 0.031
    and near 3:2 p = 0.0071 (p7). The abstract concludes "planet pairs show
    little statistical preference to be near mean-motion resonances".
- Weiss 2018 p8: "essentially no planets" have adjacent period ratio below
  1.2, for any planet size from 0.5 to Neptune size ("wall"); typical
  Lagrange-stable minimum for 3 Earth-mass (mass ratio 1e-5) and e < 0.04
  is about 1.2 (Deck 2013, quoted p8, Second-hand).
- Steffen & Hwang 2015: excess of pairs near 2.2 (2.13-2.23, p9) besides
  the known features near first-order resonances (abstract p1); high-
  multiplicity systems have their largest peak near 3:2, three-planet
  systems near 2.2 (p6).
- EPOS I (p5, eq. 11, Table 2 p12): spacing parameter
  D = 2 (P^(2/3) - 1)/(P^(2/3) + 1) (the Malhotra 2015 form, equal to the
  fractional separation 2(a2-a1)/(a1+a2)), lognormal with log D =
  -0.39 (+0.07/-0.05) and sigma = 0.18 (+0.05/-0.04); text p14: "typical
  orbital period ratio P = 1.8 with a wide distribution". **(derived)**
  Taking log as base 10: D = 0.41 gives P = 1.86, the 1-sigma range of D
  (0.27-0.62) gives P = 1.50-2.60, and 2 sigma gives P = 1.31-4.56.
- Weiss 2018 pp6-7: the ratio of consecutive period ratios
  (P_(j+2)/P_(j+1)) / (P_(j+1)/P_j) has mean 1.03, median 1.00,
  standard deviation 0.27 over 165 pairs in 104 systems with ratios < 4.
- He 2019: planets "near an MMR" are defined as ratio within 5 % exterior
  to 2:1, 3:2, 4:3 or 5:4 (p5, e.g. 1.5-1.575 for 3:2). In their fully
  clustered model the fraction of all planets near an MMR with another
  planet is f_mmr = 0.29 +/- 0.04 (p27). This is a model-implied fraction,
  not a direct count, and their model has no excess near MMRs built in
  (p27).

**For the draw** (proposal): draw log D normal (-0.39, 0.18) (EPOS I), then
convert to a ratio, with a floor at 1.2 (Weiss wall) and a mild extra
mass-dependent bump just wide of 3:2 and 2:1 only if resonance flavour is
wanted (Fabrycky, Steffen & Hwang). The spacing must also satisfy the
Hill-spacing floors of section 2, which are tighter for massive planets.
Draw Delta in Hill units first (section 2) and derive the period ratio
from the masses (Weiss eq. 5) as an alternative path: **(derived)** at
Delta = 10, 12, 20, 30 two equal 5 Earth-mass planets around 1 solar mass
have ratios 1.38, 1.48, 1.93, 2.73; two 1 Earth-mass planets 1.21, 1.26,
1.46, 1.78; two 10 Earth-mass planets 1.51, 1.64, 2.31, 3.66. Missing: the
near-resonant fraction as a measured number (only the model-implied 0.29).

## 2. Spacing and stability

### 2.1 Definitions of the mutual Hill radius

Every source read defines the same quantity, so Delta values compare
directly.

| Source | Definition | Page |
| --- | --- | --- |
| Weiss 2018 | R_H = ((m_j + m_(j+1))/(3 M))^(1/3) (a_j + a_(j+1))/2; Delta = (a_(j+1) - a_j)/R_H | p9, eq. 3-4 |
| Fabrycky 2014 | same, with "in" and "out" subscripts | p2, eq. 2-3 |
| Fang & Margot 2013 | same | p3, eq. 5-6 |
| Pu & Wu 2015 | same, spacing called K | p2, eq. 1-2 |
| Petit 2020 | same | p3, eq. 2 |
| He 2019, He 2020 (III) | same R_H; Delta = (a_out(1-e_out) - a_in(1+e_in))/R_H (periapsis over apoapsis) | He 2019 p5 eq. 10-11; He 2020 (III) p5 eq. 8 |

Two other forms appear and are not the same unit:

- Chambers, Wetherill & Boss 1996, as described by Petit 2020 (p3): spacing
  Delta = (a_(k+1) - a_k)/a_k, with the survival time fitted against
  Delta in units of R_H. The original was not reached, so its exact
  Hill radius is unconfirmed here (Second-hand).
- Petit 2020 abstract (p1) and p12 (eq. 59): the natural unit of three-
  planet stability scales as the planet-to-star mass ratio to the power
  1/4, not the 1/3 of the Hill radius; the Hill unit is "not adapted".

### 2.2 Observed spacing in mutual Hill radii

- Fang & Margot 2013 (p1, p4): best-fit intrinsic Delta distribution is a
  shifted Rayleigh, f(Delta) = (Delta - 3.5)/sigma^2 exp(-(Delta - 3.5)^2
  / (2 sigma^2)) (eq. 7, p3) with sigma = 14.5 (mean 21.7, standard
  deviation 9.5; about 50 % of pairs above 20 and about 90 % above 10).
  Uncertainty in the mass-radius relation moves sigma over 12.5-16.5 (p5).
  **(derived)** mean = 3.5 + sigma sqrt(pi/2) = 21.7, matching the quoted
  mean. Sample: candidates under 200 d and 1.5-30 Earth radii. Inclinations
  were Rayleigh with sigma 1 or 2 degrees (p3).
- Weiss 2018 p10 and abstract: 93 % of pairs are at least 10 mutual Hill
  radii apart, distribution peaks at about 20. Planets under 1 Earth
  radius are all at least 16 apart (p10). Systems with 4 or more transiting
  planets have the smallest Delta, for example Kepler-11 with Delta < 10
  (p10, using measured masses). Radii-to-mass relations used are
  eq. 6-9 (p9).
- Pu & Wu 2015: for systems of 4 or more transiting planets, spacings
  cluster around 12 mutual Hill radii once transit geometry and sensitivity
  are accounted for (abstract, p1). Best fit with Gaussian K spacing:
  K_mean = 14, sigma_K = 3.4, so K_eff = K_mean - 0.5 sigma_K = 12.3
  (p7, eq. 9-10 on p3).
- Fabrycky 2014: 2 of 761 adjacent pairs violate Delta > 2 sqrt(3) (abstract
  p1).
- He 2019: minimum spacing Delta_c = 8 fixed after exploratory runs
  preferred Delta of 10 or less (p5). He 2020 (III) fitted Delta_c: 9.36
  (+1.85/-1.68) and 9.28 (+1.24/-1.19) in the two-Rayleigh model (Table 4,
  p19), about 9 for distance functions D_W1/D_W2 and 10.79-11.27 with a
  gap-complexity term (p20).

**For the draw** (proposal): draw Delta from the Fang & Margot shifted
Rayleigh (offset 3.5, sigma about 14.5, range 12.5-16.5) for the
variety of spacing, and reject below a floor from section 2.3. Weiss and
Pu & Wu argue the compact subpopulation peaks lower (12-20), so the
compact family can use sigma near 12.5 and a floor of 8-10. The solar-like
family uses sigma near 14.5-16.5 (section 5).

### 2.3 Stability criteria

- Two planets, circular and coplanar, Hill stability: Delta > 2 sqrt(3)
  = 3.46 (Gladman 1993), quoted as eq. 8 (Fabrycky 2014 p4), p2 (Pu & Wu),
  p3 (Petit, eq. 1), and by Fang & Margot (p3). Original Not reached.
- Pu & Wu 2015 p2: first-order resonance overlap (Wisdom 1980, coefficient
  from Duncan 1989): |a2 - a1| / ((a1 + a2)/2) <= 1.49 mu^(2/7), with
  mu = (m1 + m2)/M (eq. 3); in K units K_crit = 3.46 (mu / 4.5e-5)^(-1/21)
  (eq. 4); the two criteria meet at mu = 4.5e-5. Weiss 2018 p8 quotes
  Deck 2013 with 1.46 (mu1 + mu2)^(2/7) and a period ratio of about 1.2 for
  3 Earth masses.
- Three or more planets, empirical (Pu & Wu p2-3, quoting Funk 2010 and
  others, not reached): K_crit = 2 + N for N < 8 and 10 for N >= 8 at
  tau = 10^8.5 and mu = 5e-5 (eq. 8); K_crit about (1.33 +/- 0.3) log tau
  for 8 planets (eq. 6); log tau + 1.7 for 5 planets, mu = 3e-6 (eq. 7)
  (Second-hand).
- Pu & Wu 2015 own result (p3-4): survival probability
  s = 1 - exp(-[0.5 K_eff - 0.6 - 0.35 log tau]^2) for
  K_eff >= 1.2 + 0.7 log tau (eq. 11), tau = t / T_1 (innermost period);
  median-survival spacing K50(t) = 0.7 log tau + 2.87 (eq. 12, p4), which
  is K50 about 10.2 at 1 Gyr for T_1 about 12 d (p4; **(derived)**
  check: 0.7 x 10.5 + 2.87 = 10.2). Setup: 7 planets of 3-9 Earth masses,
  innermost at 0.1 AU, K Gaussian, Rayleigh e and inclination (p3).
- Pu & Wu 2015, effect of eccentricity and inclination, Rayleigh scales
  sigma_e and sigma_inc (eq. 14, p4): K50(sigma_e, sigma_inc) =
  K50(0,0) + (sigma_e/0.01) + (sigma_inc/0.04) (inclination in radians,
  scan sigma_e in [0, 0.05], sigma_inc in [0, 0.16] rad = 9 deg). With
  K50(0,0) = 10.2 this gives 12.3 for the observed Kepler spacing, which
  the abstract states as about 10 (circular, coplanar) and about 12
  (e about 0.02). In Hill-eccentricity units (e_H = R_H/a about 0.024 for
  mu about 2e-5): K50 = K50(0,0) + 0.4 sigma_e/e_H + sigma_inc/(4 e_H)
  (eq. 15, p5; e_H on p4).
- Chambers 1996 behaviour (Petit 2020 p3, second-hand): survival time grows
  exponentially with Delta in Hill units; spread about the linear trend of
  log(T/P) is about 0.43 +/- 0.16 dex (Hussain & Tamayo 2020 quoted);
  minimum spacing for a few billion orbits is "around 10 Hill radii".
  Chambers 1996 itself: Not reached.
- Petit 2020: for three planets on circular coplanar orbits, instability
  time is set by overlap of three-body resonances; a rescaled spacing in
  eps^(1/4) rather than Hill radii (abstract p1, eq. 59 p12). The Hill
  criterion is described as a lower limit, instability can occur beyond
  the predicted overlap (p13). The closed-form survival time was not
  extracted into numbers here.
- Fabrycky 2014 p5, eq. 9: heuristic for three adjacent planets,
  Delta_in + Delta_out > 18, "does not assure stability" for eccentric
  planets. Tested systems survived 10^10 innermost orbits (p5).
- He 2019 (p5) and He 2020 (III) (p5) use Delta > Delta_c with the
  eccentric form of section 2.1, Delta_c = 8 (He 2019) and fitted 9-11
  (He 2020 III).

**For the draw** (proposal): the hard floor is 2 sqrt(3) (about 3.5), which
every two-planet system must satisfy. A usable multi-planet floor is the
empirical K50 of Pu & Wu: about 10 (circular) rising by +1 per 0.01 of e
and +1 per 0.04 rad of inclination, evaluated with the eccentricity just
drawn; or the simpler Delta_c = 8-9 of He 2019/2020 combined with their
periapsis-apoapsis form. Both apply per adjacent pair. Missing: Chambers
1996 and Funk 2010 formulas read directly; any criterion for non-equal
masses in closed form (Petit 2020 gives it analytically, not extracted).

## 3. Small-planet eccentricities and mutual inclinations

### 3.1 Eccentricities, multi-transiting systems

- Van Eylen 2019 (66 multi-planet and 51 single planets with
  asteroseismic stellar densities, abstract p1, Table 3 p7):
  - multis, half-Gaussian: sigma_e = 0.083 (+0.015/-0.020); Rayleigh:
    sigma = 0.061 (+0.010/-0.012) (p6-7). The earlier Van Eylen & Albrecht
    2015 value was a Rayleigh scale 0.049 +/- 0.013 (p1).
  - mixture (half-Gaussian plus Rayleigh) for multis: fraction in the
    Rayleigh part f_multi = 0.08 (+0.03/-0.08) (Table 3, p7).
  - TTV pairs: Hadden & Lithwick 2014 rms eccentricity 0.018 (+0.005/-0.004)
    (quoted p13, Second-hand); the TTV subsample has lower typical e.
  - the main sample is P > 5 d and R < 6 Earth radii. Planets with P < 5 d
    (13 singles, 6 multis) give half-Gaussian sigma_single = 0.10
    (+0.03/-0.05) and sigma_multi = 0.04 (+0.03/-0.04) (p7), as expected
    from tidal circularization; the three hot Jupiters among the singles
    give sigma = 0.012 (+0.010/-0.011) (p8).
- Mills 2019 (CKS VIII, about 1000 planets, stellar radii to about 3 %):
  multis sigma_e = 0.0355 +/- 0.012 (2-sigma) and mutual-inclination scale
  sigma_i = 2.45 (+0.65/-0.53) degrees (p8); mean eccentricity of multis
  about 0.05 (abstract p1).
- Xie 2016 (698 planets, LAMOST stellar parameters): multis mean eccentricity
  0.04 (+0.03/-0.04), mean mutual inclination 1.4 (+0.8/-1.1) degrees
  (abstract p1); mean e below 0.07 and mean inclination 0.3-2.2 degrees
  (p2); mean e of multis with 2, 3 and more than 3 transiting planets all
  near zero within errors (p2). Common relation e about (1-2) x i for
  Kepler multis and Solar System objects (abstract p1).
- He 2019 Table 5 (p16): one Rayleigh scale for all eccentricities,
  sigma_e = 0.020 (+0.014/-0.010) in the fully clustered model
  (0.014 for AD); low-inclination component sigma_i,low = 1.40
  (+0.54/-0.39) degrees (KS), 1.29 (AD). Planets near an MMR are forced into
  the low-inclination population (p5).
- He 2020 (III): eccentricity and inclination scale with the intrinsic
  multiplicity n as medians mu_e,n = mu_e,5 (n/5)^(alpha_e) with mu_e,5 =
  0.031 (+0.004/-0.003) and alpha_e = -1.74 (+0.11/-0.07), and mutual
  inclination median mu_i,5 = 1.10 (+0.15/-0.11) degrees with alpha_i =
  -1.73 (+0.09/-0.08) (eq. 50-51, p25; summary p37). Distributions at each
  n are close to lognormal rather than Rayleigh (p37). Zhu 2018 (quoted p25,
  Second-hand): sigma_i,5 = 0.8 degrees, alpha_i = -3.5.
- Fabrycky 2014 (p9-10): inclinations and eccentricities modelled as
  Rayleigh with sigma_i = delta and sigma_e = n delta; best fit delta =
  0.032, n = 1, i.e. about 1.8 degrees and e about 0.032 (p10); acceptable
  inclination scale 1.0-2.2 degrees, eccentricity unconstrained
  ("circular to several times equipartition", p10).
- EPOS I (p5, p14): mutual inclination Rayleigh mode Delta_i =
  2.1 (+1.0/-0.8) degrees (Table 2, p12), "typically 1-3 degrees"
  (Fabrycky 2014 quoted).
- Pu & Wu 2015 p9: TTV studies give sigma_e about 0.02 for near-resonant
  pairs and another sub-population with sigma_e about 0.1 "likely
  exclusively in low-multiple systems" (Wu & Lithwick 2013; Hadden &
  Lithwick 2014, Second-hand).

**For the draw** (proposal): small planets of the compact family:
Rayleigh e with scale 0.02-0.06 (He 2019 low end, Van Eylen 2019 high end;
Mills 0.036 in between) and inclination Rayleigh scale 1-2.5 degrees
(Fabrycky, Mills, He 2019). For variety, tie the scale to the planet count
by the He 2020 (III) power law (alpha about -1.7 for both). Each source
fits a different functional form to the same transit-duration or
asteroseismic data; the choice of Rayleigh is the common denominator, the
half-Gaussian of Van Eylen differs only by a factor of 1.4 in the scale
(0.083/0.061).

## 4. Sparse family and the Kepler dichotomy

- Van Eylen 2019 (abstract p1, Table 3 p7): single-transiting systems:
  half-Gaussian sigma_e = 0.32 +/- 0.06; Rayleigh sigma = 0.24 +/- 0.04.
  Mixture model theta = {sigma_Gauss, sigma_Rayleigh, f_single, f_multi} =
  {0.049 (+0.017/-0.024), 0.26 (+0.04/-0.06), 0.76 (+0.21/-0.12),
  0.08 (+0.03/-0.08)}: a fraction 0.76 of singles in a Rayleigh component
  peaking at 0.26, the rest near-circular. A cut at P > 10 d instead of
  5 d gives half-Gaussian sigma_single = 0.36 (+0.06/-0.08), sigma_multi =
  0.09 (+0.01/-0.04) and mixture {0.042, 0.27, 0.82, 0.10}, consistent at
  1 sigma (p8). Short-period singles (P < 5 d) have lower eccentricity,
  sigma = 0.10 (p7).
- Mills 2019: singles best fit sigma_e = 0.167 (+0.013/-0.008) (95 %, p4);
  mean e about 0.21 (abstract p1). Two-population fit: low-e scale at most
  about 0.05 and high-e scale at least about 0.3, 69 % of singles in the
  low-e population (p5). Evidence that high-e planets prefer metal-rich
  stars (abstract p1).
- Xie 2016 (p1-2): singles mean e = 0.32 +/- 0.02 (Rayleigh); two-
  population fit with cold population fixed to the multis: hot fraction
  F_hot about 16-36 % (1 sigma 0.157-0.364) with mean e_hot above 0.47
  (p2, Fig. 4 p7).
- Ballard & Johnson 2016 (abstract p1, p6): 55 (+23/-12) % of M-dwarf
  planetary systems are single or contain highly mutually inclined planets
  (mixture parameter f = 0.55, p6). The best single-population models
  (N = 6 +/- 2, sigma =
  7.4 (+2.5/-4.6) degrees) still underpredict singles (p4-5).
- He 2019 (abstract p1, Table 5 p16): the dichotomy is explained by a
  population of high mutual inclination: f_sigma_i,high = 0.42 (+0.08/-0.07)
  of systems, with sigma_i,high = 48 (+17/-17) degrees (KS), versus
  sigma_i,low = 1.40 degrees. "Unlikely to be solely due to a population of
  intrinsically single planet systems". He 2020 (III) abstract (p1): intrinsic
  singles have Rayleigh eccentricity scale sigma_e,1 about 0.25 (Table 4
  p19, maximum-AMD model, six fits: 0.27 +/- 0.13, 0.30 +/- 0.12, 0.20
  (+0.16/-0.11), 0.25 +/- 0.14, 0.23 (+0.13/-0.11), 0.30 (+0.11/-0.16)).
  Fraction of systems with a transiting planet at 5-10 d that have another
  planet of greater RV signal: about 53 % (abstract p1).
- Lissauer 2011 and Hansen & Murray 2013, as quoted by Van Eylen 2019 p12
  (Second-hand): up to two thirds of singles from a different population
  (Lissauer); excess singles about 50 % (Hansen & Murray).
- Pu & Wu 2015 abstract (p1): they propose the singles and doubles are
  descendants of more closely packed systems pared down by instability;
  about 75 % of the initial systems are disrupted by 1 Gyr in their model
  (p8, "this fraction could rise or drop" with the assumed initial
  population).
- Fang & Margot 2013 abstract (p1): at least 31 %, 35 % and 45 % of 2-, 3-
  and 4-planet systems are dynamically packed (stable with room for no
  extra planet), a lower bound.

**For the draw** (proposal): a sparse system (0-2 planets) can draw
eccentricity from a two-component mixture: about 25-50 % near-circular
(Rayleigh sigma about 0.05) and the rest Rayleigh with sigma about
0.25-0.3 (Van Eylen mixture 0.76/0.26; Mills 0.69 low / sigma_high >= 0.3;
Xie F_hot 0.16-0.36, e_hot > 0.47). The fraction of sparse systems is a
`Definition:` (the family is chosen evenly); no source gives 0-2 as a count.
Where a mutual inclination applies (two planets), the He 2019 bimodal
scale (about 1.4 degrees, 48 degrees at 42 %) is the sourced shape.
Missing: the sparse family's period distribution (the sources give
occurrence, not the arrangement of 0-2 planets).

## 5. Inner rocky planets of a solar-like system

- **(derived)** Solar System terrestrial spacing in mutual Hill radii by
  Weiss 2018 eq. 3-4 (inputs are textbook planet masses 0.0553, 0.815, 1.0,
  0.107 Earth masses and semi-major axes 0.387, 0.723, 1.000, 1.524 AU,
  not read in a source for this study): Mercury-Venus 63.4, Venus-Earth
  26.3, Earth-Mars 40.1, Mars-Jupiter 16.0 (with 317.8 Earth masses at
  5.203 AU). Period ratios 2.55, 1.63, 1.88, 6.31.
- Weiss 2018 Fig. 14 (p10) overlays the Solar System pairs on the Kepler
  histogram; their individual values are not printed in the text.
- Fang & Margot 2013 (p5): a K-S test between their intrinsic Delta
  distribution and the adjacent Solar System planets gives p = 66.2 %: the
  Solar System spacing is consistent with the Kepler distribution. The
  abstract (p1) says the same.
- Pu & Wu 2015 (abstract p1): the formation environment of Kepler systems
  should be more dissipative than that of the terrestrial planets. Fang &
  Margot p5 (not extracted into numbers) discuss the chaos of the inner
  Solar System.
- Raymond 2006 (five N-body runs from 1000-2000 embryos and
  planetesimals, abstract p1; the reference run has a Jupiter-mass giant,
  p2): each run formed 2-4 terrestrial
  planets with masses 0.4-2.6 Earth masses and eccentricities about 0.05.
  Initial embryo separation chosen between 0.3 and 0.6 mutual Hill radii,
  1885 protoplanets in the reference run (p2). Embryo isolation spacing of
  5-10 mutual Hill radii quoted for oligarchic growth (p6-7). Their R_H
  definition was not stated in the lines read.
- Chambers 2001 (Icarus 152, 205): Not reached. No terrestrial count or
  spacing is taken from it. EPOS I (abstract p1): "the solar system is not
  a typical planetary system, but an outlier"; fewer than 8 % of
  systems lack a planet inside Mercury's orbit.

**For the draw** (proposal): rocky inner planets of the solar-like family
get Delta from the same shifted Rayleigh as the compact family with a
larger sigma (14.5-16.5) so that the Solar System values 26-63 are inside
the support, and a count of 2-4 from Raymond 2006 (a simulation with a
Jupiter, not an observation). Missing: any observed count of rocky planets
inside the ice line (Kepler is blind outside about 1 AU and RV sensitivity
is not in the sources read); Chambers 2001.

## 6. Ultra-short-period limit

- Rappaport 2013 eq. 1 (p1): Roche limit of an incompressible fluid,
  a_min = 2.44 R_star (rho_star/rho_planet)^(1/3). In period (eq. 2):
  P_min = sqrt(3 pi (2.44)^3/(G rho_p)) = 12.6 hr (rho_p / 1 g cm^-3)^(-1/2),
  essentially independent of the star. For a highly compressible planet
  (Roche lobe radius 0.49 (m/M)^(1/3) a, Eggleton 1983, eq. 3) eq. 4 gives
  P_min = 9.6 hr (rho_p / 1 g cm^-3)^(-1/2). For an iron-silicate planet,
  eq. 5 (p2, interpolated from Lai et al. 1993 polytropes) is
  P_min = 12.6 hr (rho_p / 1 g cm^-3)^(-1/2) (rho_0/rho_p)^(-0.16), with
  central-to-mean density ratio rho_0/rho_p between about 1 and 2.5.
- Rappaport 2013 p2: below about 12 hr the limit constrains gaseous
  components; below about 6 hr it is "highly constraining even for
  terrestrial bodies"; planets with periods as short as 4 hours must have
  mean density of at least 7-9 g cm^-3 (p2). Pure iron (10-12 g cm^-3)
  survives down to 3.5 hr, pure silicate (3.5-3.9 g cm^-3) down to 6.0 hr;
  Mercury (0.055 Earth masses, 5.4 g cm^-3) could not survive at 4.245 hr
  (p4).
- **(derived)** For an Earth-density planet (5.51 g cm^-3) eq. 2 gives
  P_min = 5.4 hr and eq. 4 gives 4.1 hr; for 3.5 g cm^-3, 6.7 hr and 5.1 hr;
  for 8 g cm^-3, 4.5 hr and 3.4 hr. The separation limit for a planet of
  Earth density around a Sun-density star is a_min = 2.44 (1.41/5.51)^(1/3)
  = 1.54 stellar radii (eq. 1, density of the Sun 1.41 g cm^-3 not read in
  a source here).
- Observed floor: KOI 1843.03, period 4.245 hr (0.18 d), radius about
  0.6 Earth radii (abstract p1), the shortest known period at the time.
  Sanchis-Ojeda 2014 (abstract p1): 106 USP candidates (P < 1 day) from
  200 000 Kepler targets; USPs exist around about 0.51 +/- 0.07 % of G
  dwarfs and 0.83 +/- 0.18 % of K dwarfs; most are accompanied by planets
  of period 1-50 d (unlike hot Jupiters); almost all USP radii are under
  2 Earth radii, 95 % under 1.9 (p7). Candidates with P < 10 hr require mean
  densities above 2 g cm^-3 (Fig. 7 caption, p9).
- Lee & Chiang 2017 abstract (p1): USPs are "more widely spaced than their
  longer period counterparts", attributed to tidal migration from the disk
  edge (planets brought inwards by asynchronous equilibrium tides).

**For the draw** (proposal): the innermost allowed orbit of a rocky planet
is the Roche period of eq. 2 with the planet's own mean density, with a
margin factor left as a `Definition:`; in practice a floor of about 4-6 hr
for rocky planets and about 12 hr (eq. 2 at 1 g cm^-3) for puffy ones. An
observed floor of 4.2 hr is the reference for "known to exist". The
compact family would normally not reach the floor because of section 1.2.

## 7. Small-planet masses

Covered by section B9 of the planet-interior study (Neil & Rogers 2020,
Table 1 p7, log-normal in M; median 2.7-5.6 Earth masses for models 2-3;
Wu 2019 p1, p8). Host-mass scaling noted there: super-Earth masses peak
narrowly near 8 Earth masses x (M_star / M_sun), mass ratio mu = M_p/M_star
about 2.5e-5 (Wu 2019). Millholland 2017 p5 finds that stellar mass
explains no significant variation in median radius (p5, "M_star produces no
significant impact") but a positive trend for the inner planet's semi-major
axis (section 1.2). Not redone here.

## 8. Not reached, abstract-only, and what is missing

- Not reached: Gladman 1993, Chambers, Wetherill & Boss 1996, Chambers
  2001, Deck et al. 2013, Funk et al. 2010, Hansen & Murray 2013, Zhu et al.
  2018, Hadden & Lithwick 2014/2017, Lissauer 2011, Malhotra 2015, Wisdom
  1980. Where numbers from them appear, the citing page is given and the
  channel is Second-hand.
- No source was read through search abstracts only; the arXiv PDFs were
  extracted in full for all papers in the sources table. Zhu 2019
  (arXiv:1907.02074) appears only as a search-result title and was not
  opened.
- Missing numbers: a measured outer edge of compact systems; the inner edge
  outside 0.2-1.4 solar masses; a numeric intra-system mass scatter;
  a measured near-resonant fraction (only the model-implied 0.29 of
  He 2019); a count of rocky planets inside the ice line from observation;
  the closed-form three-planet survival time of Petit 2020 as a ready
  formula; the sparse-family arrangement (periods of 0-2 planets).
