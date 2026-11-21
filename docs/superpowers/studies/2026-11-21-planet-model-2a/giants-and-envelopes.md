# Literature study: radii and layers of giants, mini-Neptunes and steam worlds

Planet model PR 2a, goal 4 (giants and mini-Neptunes) and the steam item of
goal 1. Written on 2026-11-21 for the design
[planet-model-pr2a-design.md](../../specs/2026-11-21-planet-model-pr2a-design.md).

Method and rules followed: every number below was read in a source text
downloaded and extracted during this session (arXiv through ar5iv HTML, the
publisher or author web pages, Crossref and Europe PMC abstracts, the NASA
fact sheet and the NASA Exoplanet Archive). A number that I computed myself
from sources is marked **(computed here)**. A number that is not in a source I
read is marked **not found**. DOIs were checked against Crossref when the API
answered; the others come from the search result that pointed to the paper and
are marked **(DOI from search result)**. Where a detail rests on my
recollection instead of a text I read, it is said so.

Conventions: `RJ` = 71 492 km, the equatorial radius of Jupiter at 1 bar, the
unit of Fortney et al. 2007 (near the end of their paper, Sect. VI) and of the
NASA fact sheet diameter used below; `MJ` = Jupiter mass; `Gerg` = 10^9 erg;
1 Gerg s^-1 cm^-2 = 10^6 W m^-2.

## 0. Summary of the proposals

| Item | Proposal |
| --- | --- |
| Giants, 10 to 3 600 Earth masses | Fortney, Marley & Barnes 2007, the 25 `.dat` files of the author's page (326 016 bytes, 9 ages, 5 distances, 5 core masses). Radius at 1 bar. |
| Mini-Neptunes, 1 to 20 Earth masses | Lopez & Fortney 2014, Tables 2 to 7 (3 fluxes, 8 masses, 11 envelope fractions, 3 ages, 2 opacities), interpolated. The power-law fit only as a sanity check: it is off by 7 to 30 %. Radius at 20 mbar. |
| Hot-Jupiter inflation | Not in the tables. Thorngren & Fortney 2018 give a heating efficiency, not a radius; turning it into a radius needs an evolution code (#621). For 2a: either no inflation (and say so), or the empirical offset of Sestovic et al. 2018, Eq. 11. |
| Ice giants (10 to 50 Earth masses, 60 to 90 % heavy) | **Not covered by any table that I could obtain**. Closest: Lopez & Fortney tables up to 20 % envelope (Neptune comes out within -5 to +15 %), Baraffe et al. 2008 Z = 0.9 (isolated, Neptune +13 %). |
| Molecular-metallic hydrogen | A single boundary pressure of 100 GPa (1 Mbar), range 50 to 200 GPa, with the sources of Sect. 5. Located with the n = 1 polytrope of Hubbard 2013 (analytic, sourced), validated against Debras & Chabrier 2019 for Jupiter only. |
| Steam or supercritical water | Turbet et al. 2020 Eqs. 2 and 3 for water fractions up to a few percent; Aguichine et al. 2021 grid (coefficients on Zenodo, 140 KB) from 10 % water. The 5 to 10 % gap is interpolated. |

## 1. Giant planet radii: Fortney, Marley & Barnes 2007

### 1.1 What is tabulated

Source: J. J. Fortney, M. S. Marley, J. W. Barnes, "Planetary Radii across Five
Orders of Magnitude in Mass and Stellar Insolation: Application to Transits",
ApJ 659, 1661 (2007), doi:10.1086/512120, arXiv:astro-ph/0612671 (v4, which
corrects the coefficients of Eqs. 7 and 8; an erratum was sent to the journal,
per the arXiv listing).

- Tables 2, 3 and 4 of the paper: giant planet radii in `RJ` at 300 Myr, 1 Gyr
  and 4.5 Gyr. Each has 5 distances (0.02, 0.045, 0.1, 1.0, 9.5 AU) times 5
  core masses (0, 10, 25, 50, 100 Earth masses) = 25 rows, and 12 planet masses
  (0.052 to 11.3 MJ = 17 to 3 594 Earth masses). A "-" marks a planet lighter
  than its core. Equilibrium temperatures: 1960, 1300, 875, 260 and 78 K at the
  five distances (Note of Table 4).
- Table 1: ice, rock and iron planets, 0.01 to 316 Earth masses, in Earth
  radii. Eqs. 7 and 8 fit it (Sect. 1.4).
- The model: hydrogen-helium with Y = 0.28 (Saumon et al. 1995 equation of
  state), a core of 50/50 ice and rock by mass (ANEOS water and olivine, zero
  temperature plus a thermal term for water), a non-gray atmosphere grid for
  the irradiation, the radius at 1 bar. The paper states that "the models
  correspond to a pressure of 1 bar" and ignores the transit-radius effect.
  Heat content of the core is ignored. Cores are constant masses; the 10, 50
  and 90 % cores of Fig. 7 are not tabulated.
- Irradiation: the Sun's present luminosity at the stated distance, flux
  divided by 4 for the planet-wide mean, no dependence on the spectral type of
  the star (the paper says this is ignored).
- No extra internal heating: the paper says tidal heating "may be important for
  the hot Jupiters", that HD 209458b "must have an additional interior energy
  source", and does not include any. **The tables are the uninflated radius.**
- Helium phase separation is neglected (adds about 1 000 km to cold giants at
  Gyr ages, per the paper).

### 1.2 Machine-readable data

The author's page `https://www.ucolick.org/~jfortney/models.htm`, section
"Models of Planetary Radii", links 25 files `models/tbl_Evpcore_<core>_<dist>.dat`
with `<core>` in `00b` (`00c` at 0.02 AU), `10a`, `25a`, `50a`, `100a` and
`<dist>` in `point02AU`, `point045AU`, `point1AU`, `1AU`, `9point5AU`. The page
says they are "interpolated from" the thermal evolution models of the 2007
paper, run from 1 Myr to 10 Gyr, and that "some very low-mass young models are
excluded, particularly at the highest irradiation".

I downloaded all 25 (`curl`, no login). Format, verified on the files:

- A 5-line header, then 9 blocks introduced by `t (Gyr) = 0.0010`, 0.0032,
  0.0100, 0.0316, 0.1000, 0.3162, 1.0000, 3.1623, 10.0000 (half a decade
  apart).
- Each block has columns `Me Mj Rj g Tint L`: mass in Earth and Jupiter
  masses, radius in `RJ`, gravity in m s^-2, internal temperature in K, internal
  luminosity in erg s^-1.
- The mass grid is logarithmic, 9 per decade (10.00, 12.92, 16.68, 21.54, ...,
  3 593.81) plus 317.89 (1 MJ). Rows exist only where the mass exceeds the core
  mass and, at young ages and high irradiation, only where the model ran:
  18 to 25 rows per block in the coreless files, 15 in the 100-Earth-mass-core
  files, 21 in the 25-core files, 18 in the 50-core files.
- Total size: 326 016 bytes as text. As `f32` radii only, 25 files times 9 ages
  times at most 25 masses is under 6 000 values (about 23 KB).
- Licence: **not found**. The page states none. The numbers are also printed in
  the paper (4.5 Gyr, 1 Gyr, 300 Myr only), so cite both; asking the author is
  cheap.

The paper's own tables (3 ages) are a subset; the `.dat` files add 6 ages and
all masses. I extracted the three printed tables from the ar5iv HTML as a
cross-check: the 4.5 Gyr value of the table lies between the 3.16 and 10 Gyr
blocks of the `.dat` everywhere I looked (Jupiter at 1 AU, 10-Earth-mass core:
1.068 at 3.16 Gyr, 1.037 at 10 Gyr, 1.058 printed at 4.5 Gyr).

**One misprint in the paper.** Table 4, 9.5 AU, core 25 Earth masses, column
464 Earth masses reads 1.037, larger than its neighbour of 774 Earth masses
(1.035) and than the 10-core row (1.024). The `.dat` gives 1.013 at 3.16 Gyr
and 0.985 at 10 Gyr, so about 1.005 at 4.5 Gyr. Use the `.dat`, not the printed
tables, as the source of the data, and the printed table only as a check.

### 1.3 Validity limits

- Masses 10 to 3 594 Earth masses (core-free), cores 0 to 100 Earth masses,
  distances 0.02 to 9.5 AU, ages 1 Myr to 10 Gyr. Outside, clamp or refuse; do
  not extrapolate.
- Equivalent distance for another star: the tables are for a solar luminosity,
  so interpolate in `d_eq = a / sqrt(L / Lsun)` (flux-equivalent distance; the
  paper's Eq. 11 does this with a mass-luminosity law, `d_sun = d (Msun/M)^(eta/2)`
  with `eta` about 3.5, and warns that Bond albedos depend on the star type).
  This replacement is mine; the paper only gives the idea.
- Radius at 1 bar, not at the transit level (5 to 6 scale heights higher per
  the paper, Burrows et al. 2006).
- No inflation (Sect. 3), no deuterium burning, no helium rain, 50/50 ice-rock
  heavy elements only.
- A planet is drawn with a continuous core mass and age: interpolate bilinear
  in `log M`, `log age`, and linear in core mass and `log d_eq`. The tables hold
  radius at fixed core mass, so a core fraction above about 60 % at 17 Earth
  masses is outside (Sect. 4).

### 1.4 Core radius, for the cross-section

Table 1 lists the 50/50 ice-rock sphere: 2.48 `R_earth` at 10 Earth masses,
3.23 at 31.6, 4.03 at 100 (Table 1, row "50/50", the first of the two 50/50
rows, which is the ice/rock mixture). Eq. 7 reproduces it:
`R = (0.0912 imf + 0.1603)(log M)^2 + (0.3330 imf + 0.7387) log M +
(0.4639 imf + 1.1193)`, with `imf` the ice mass fraction, `M` in Earth masses,
radius in Earth radii, accurate to 2.5 % on average over 0.01 to 100 Earth
masses. The paper does not state the logarithm base; base 10 gives 2.462
**(computed here)** for `imf = 0.5`, `M = 10`, against 2.48 in Table 1, so base
10. Eq. 8 for rock-iron: `R = (0.0592 rmf + 0.0975)(log M)^2 +
(0.2337 rmf + 0.4938) log M + (0.3102 rmf + 0.7932)`.

How to build the cross-section (my proposal, consistent with the design's
"core integrated with `science::structure`, envelope filling the rest"):

1. Total radius `R_tot` from the tables at the planet's mass, age,
   `d_eq` and heavy-element mass (the "core" of the tables).
2. Core radius `R_core` from `science::structure` for that heavy-element mass.
3. Envelope thickness `R_tot - R_core`.

The tables' own core is an ice-rock sphere of 2.48 `R_earth` at 10 Earth
masses; the project's core, rock and iron with the planet's water share, will
differ. The step 3 thickness is then the table envelope plus the difference of
the two cores. A consistency number to log: `R_core,table` from Eq. 7 against
`R_core` from the structure. If the project core is much larger than the
tabulated one, the envelope thickness can become small or negative: the code
must refuse that case rather than clip it silently.

### 1.5 Newer or alternative tables

| Source | What | Verdict |
| --- | --- | --- |
| Baraffe, Chabrier & Barman 2008, A&A 482, 315, doi:10.1051/0004-6361:20079321 | 10 Earth masses to 10 MJ, four total heavy-element fractions Z = 0.02, 0.10, 0.50, 0.90, 8 ages 0.01 to 7 Gyr; files `PLANET08_iso.noirrad` and `.irrad` (14 KB each) at `perso.ens-lyon.fr/isabelle.baraffe/PLANET08/`. Irradiated files are for one distance only (a Sun at 0.045 AU, per `FILES_DESCRIPTION`). | Useful as a check and for Z = 0.9, **not** as the main table: one distance, no free core mass. |
| Müller & Helled 2021, MNRAS, doi:10.1093/mnras/stab2250 (DOI from search result), arXiv:2108.00949 | 45 000 models, 0.1 to 30 MJ, Z up to 0.8, flux 10 to 10^9 erg s^-1 cm^-2. Public Python module `github.com/tiny-hippo/planetsynth` (MIT) with interpolators. | Better physics (CMS equation of state, bulk and envelope metallicity), **but the repository holds 245 MB of interpolators** (5 zip parts, 52 MB each, read from the GitHub tree), the model data are "shared on request", and the authors warn that fluxes above 2x10^8 erg s^-1 cm^-2 "should be treated with caution". Too big for `include_bytes!`. |
| Linder et al. 2019, A&A, doi:10.1051/0004-6361/201833873 | 5 Earth masses to 2 MJ, **isolated** planets (cooling curves). | Not applicable: no irradiation. |
| Chen & Rogers 2016 Table 1 | 17 to 3 594 Earth masses, 50/50 ice-rock cores, MESA, reproduces Fortney 2007 to 3 % beyond 1 Gyr and 7 % at worst (their Sect. III.1). | Cross-check only. |

**Proposal:** Fortney et al. 2007 for giants. It is the only table I found that
has a free core mass, five irradiation levels and a size of a few hundred KB.

### 1.6 Test values

Solar system, radii at 1 bar from the NASA fact sheet diameters
(`nssdc.gsfc.nasa.gov/planetary/factsheet/`, read 2026-11-21): Jupiter 71 492 km
(mass 1 898x10^24 kg), Saturn 60 268 km (568), Uranus 25 559 km (86.8), Neptune
24 764 km (102). Earth masses **(computed here)** from 5.97x10^24 kg: Jupiter
318, Saturn 95.1, Uranus 14.5, Neptune 17.1.

Table 4 values (4.5 Gyr, 9.5 AU, so the Saturn and Jupiter row) interpolated
**(computed here)**:

| Planet | Core 10 Earth masses | Core 25 Earth masses | Observed |
| --- | --- | --- | --- |
| Jupiter (318) | 0.999 RJ (-0.1 %) | 0.972 RJ (-2.8 %) | 1.000 RJ |
| Saturn (95.1) | 0.871 RJ (+3.4 %) | 0.791 RJ (-6.2 %) | 0.843 RJ |

Saturn's observed radius falls at a 15 Earth-mass core by linear interpolation
**(computed here)**, against 19.1 +- 1.0 Earth masses of heavy elements in the
ring-seismology models of Mankovich & Fuller 2021 (Sect. 5). Jupiter's heavy
elements are 7 to 25 Earth masses (Wahl et al. 2017 abstract, via the search
result). A good tier-1 test: the model must reproduce Jupiter and Saturn to
within a few percent at a core between 10 and 25 Earth masses.

Exoplanets (NASA Exoplanet Archive, table `pscomppars`, queried 2026-11-21, the
default parameter set; values drift between releases):

| Planet | Mass | Radius | Teq | a (AU) | Age (Gyr) | Table, 4.5 Gyr |
| --- | --- | --- | --- | --- | --- | --- |
| HD 189733 b | 1.13 MJ | 1.13 RJ | 1209 K | 0.0313 | 1.4 | about 1.1 RJ (core 0): uninflated, matches |
| HD 209458 b | 0.73 MJ | 1.39 RJ | 1459 K | 0.0471 | 3.1 | about 1.1 RJ: **+26 % inflation** |
| HAT-P-12 b | 0.211 MJ | 0.959 RJ | 963 K | 0.0384 | 2.5 | 0.94 RJ at core 10 **(computed here)** |
| HD 149026 b | 0.38 MJ | 0.74 RJ | 1626 K | 0.0436 | 2.6 | 0.59 RJ at core 100, about 0.8 at core 50 (0.045 AU, 129 Earth masses): between them, a 60 to 80 Earth-mass core, as the paper says |
| WASP-12 b | 1.47 MJ | 1.965 RJ | 2601 K | 0.0234 | 3.05 | about 1.17 RJ: strongly inflated |

(HD 209458 b and HD 189733 b rows: the 0.045 AU, core-0 row of Table 4 gives
1.107 to 1.113 RJ from 1 to 1.5 MJ; the paper's Table 4 is the source.)

## 2. Mini-Neptunes: Lopez & Fortney 2014

Source: E. D. Lopez, J. J. Fortney, "Understanding the Mass-Radius Relation
for Sub-Neptunes: Radius as a Proxy for Composition", ApJ 792, 1 (2014),
doi:10.1088/0004-637X/792/1/1 (Crossref), arXiv:1311.0329.

### 2.1 The model and how the radius is built

`R_p = R_core + R_env + R_atm`:

- Core: isothermal rock and iron, 2:1 Earth-like ratio, ANEOS olivine and
  SESAME 2140 iron. `R_core = (M_core / M_earth)^0.25` in Earth radii (Eq. 1),
  "to within about 2 %" for the Earth-like composition, about 10 % if the iron
  fraction varies. For sub-Neptunes `M_core` is about `M_p` (90 to 99 % of the
  mass).
- Envelope: hydrogen-helium (Saumon et al. 1995), fully adiabatic.
- Radiative atmosphere: isothermal at `T_eq`,
  `R_atm = ln(100 bar / 20 mbar) H = 9 k T_eq / (g mu)` (Eq. 2), typically
  0.1 Earth radii; the radiative-convective boundary is at 100 to 1000 bar.
- The planet radius is defined at **20 mbar** ("appropriate for the slant
  viewing geometry in optical transits"). This is not Fortney's 1 bar; the
  difference is about 9 scale heights.
- Core heat capacity and radiogenic heating (mostly 40K) included; no
  photoevaporation; hot start at 1 Myr with 10 k_B per baryon.

### 2.2 The tables (preferred) and the fit

Tables 2 to 7 of the paper, radii in Earth radii:

- Tables 2, 3, 4: solar-metallicity atmospheres at 100 Myr, 1 Gyr, 10 Gyr.
- Tables 5, 6, 7: 50x solar enhanced opacity at the same ages.
- Each: 3 incident fluxes (0.1, 10, 1000 F_earth) times 8 masses (1, 1.5, 2.4,
  3.6, 5.5, 8.5, 13, 20 Earth masses) times 11 envelope fractions (0.01, 0.02,
  0.05, 0.1, 0.2, 0.5, 1, 2, 5, 10, 20 %) = 264 values.
- I extracted all six from the ar5iv HTML (264 numbers each, count checked).
  Total 1 584 values, a few KB. Obtain them by the same route, or by typing the
  paper's tables from the journal version; the paper has no data file.
- The paper's text covers envelope fractions 0.01 to 60 %, but the tables stop
  at 20 %.

Sample rows (Table 6 and Table 7, enhanced opacity, F = 10 F_earth, M = 5.5
Earth masses, columns 0.01 % to 20 %): 1 Gyr `1.65 1.66 1.69 1.72 1.90 2.07
2.27 2.56 3.31 4.36 6.14`; 10 Gyr `1.67 1.69 1.71 1.75 1.84 1.93 2.07 2.27 2.75
3.44 4.61`.

The analytic fit, Eq. 3 (enhanced-opacity models):

```text
R_env = 2.06 R_earth (M_p/M_earth)^-0.21 (f_env/5 %)^0.59
        (F_p/F_earth)^0.044 (age/5 Gyr)^-0.18
```

The text says the age power law is `t^0.11` for solar metallicity and `t^0.18`
for enhanced opacity "after 100 Myr", and prints the exponent as `-0.18` in
Eq. 3 (the sign in the sentence is lost; the equation is the one to follow, an
envelope contracts with age). The fit "matches our full models to within about
0.1 dex" and, by the authors' own words, is "not to be used in place of the
full models" (Sect. III.1). Parameter ranges: 1 to 20 Earth masses, 0.1 to 1000
F_earth, 0.01 to 60 %, 10 Myr to 10 Gyr; the paper states no validity limits
for the fit itself.

How good is the fit **(computed here, against the extracted tables, envelope
fractions of 1 % and above, `R_core + R_env` without `R_atm`)**:

| Table | Mean error | RMS | Worst |
| --- | --- | --- | --- |
| 6 (1 Gyr, enhanced) | +4.3 % | 6.6 % | +14.8 % (0.1 F, 1.5 Earth masses, 10 %) |
| 7 (10 Gyr, enhanced) | +3.6 % | 8.5 % | +34.9 % (1000 F, 1 Earth mass, 10 %) |
| 5 (100 Myr, enhanced) | +6.8 % | 11.3 % | -22.8 % and +25.8 % |

Applied to the planets of Table 1 of the paper (their mass, radius and
inferred envelope fraction, age and flux from the NASA archive, flux from
`Teq` with 255 K for Earth, **(computed here)**): GJ 436 b 4.52 against 4.22
Earth radii, Kepler-11 e 5.08 against 4.19, GJ 1214 b 3.41 against 2.67,
Kepler-18 d 7.09 against 6.97, HAT-P-11 b 5.37 against 4.73. Use the tables.

### 2.3 Chen & Rogers 2016

Source: H. Chen, L. A. Rogers, "Evolutionary Analysis of Gaseous
Sub-Neptune-Mass Planets with MESA", ApJ 831, 180 (2016), arXiv:1603.06596.
They give Tables 2 and 3 (radii at 10 Gyr on a mass, envelope fraction, flux
grid, for rocky and for 70 % ice + 30 % rock cores) and, in Eq. 5, a quadratic
fit in the logarithms:

```text
log10((R_p - R_core)/R_earth) = c0 + sum_i c_i x_i + sum_(i<=j) c_ij x_i x_j
x1 = log10(M_p/M_earth)   x2 = log10(f_env/0.05)
x3 = log10(F_p/F_earth)   x4 = log10(t/5 Gyr)
```

Coefficients (Table 4 of the paper; rocky core / ice-rock core): c0 0.131 /
0.169; c1 -0.348 / -0.436; c2 0.631 / 0.572; c3 0.104 / 0.154; c4 -0.179 /
-0.173; c12 0.028 / 0.014; c13 -0.168 / -0.210; c14 0.008 / 0.006; c23 -0.045 /
-0.048; c24 -0.036 / -0.040; c34 0.031 / 0.031; c11 0.209 / 0.246; c22 0.086 /
0.074; c33 0.052 / 0.059; c44 -0.009 / -0.006. `R_core` about
0.97 M_core^0.28 (rocky) or 1.27 M_core^0.27 (ice-rock).
RMS 0.035 and 0.047 dex (R^2 0.993 and 0.987).

Validity, printed in the paper: `1e-4 <= f_env <= 0.2`, 1 to 20 Earth masses,
**4 to 400 F_earth**, 100 Myr to 10 Gyr, "and should not be extrapolated beyond
these ranges". I checked why **(computed here)**: Neptune (17.08 Earth masses,
15 % envelope, 4.5 Gyr) at 0.0011 F_earth gives 18.8 Earth radii with the
ice-rock coefficients (the `c33` term). Even at the lower validity edge,
4 F_earth, it gives 4.4 (ice-rock) or 4.1 (rocky). Their reference radius is
not stated in the text I read (**not found**).

**Comparison.** Lopez & Fortney is a plain power law and robust outside its
grid (flux exponent 0.044), the Chen & Rogers fit is 3 times tighter inside its
ranges and unusable outside. A planet in the habitable or cold zone is outside
both flux ranges, which is why the tables of Lopez & Fortney (flux down to 0.1
F_earth, weak dependence) are the proposal.

### 2.4 Proposal and test values

Radius from Tables 2 to 7 of Lopez & Fortney (solar metallicity as default;
enhanced opacity is a physical choice that belongs to the atmosphere model, not
to this PR: expose the metallicity as an input). Interpolation: log mass, log
envelope fraction, log flux, log age. Beyond 20 Earth masses or 20 %: switch to
Fortney 2007 (Sect. 1), and warn about the change of reference level, 20 mbar
against 1 bar, about `R_atm` above or below.

Layers: rock-iron core from `science::structure`; the envelope is
`R_p - R_core - R_atm`, with `R_atm` from Eq. 2; the radiative atmosphere is
the top `R_atm`.

Test values: the 33 planets of Table 1 of the paper, each with mass, radius and
envelope fraction (inferred by the same models, so a regression net, not an
independent check; the paper gives mass and radius and the fraction): GJ 436 b
23.0 Earth masses, 4.22 Earth radii, 12.0 %; Kepler-11 e 8.0, 4.19, 15.7 %; GJ
1214 b 6.46, 2.67, 3.8 %; Kepler-18 c 17.2, 5.48, 23.6 %.

## 3. Hot-Jupiter inflation

### 3.1 Thorngren & Fortney 2018

Source: D. P. Thorngren, J. J. Fortney, "Bayesian Analysis of Hot Jupiter
Radius Anomalies: Evidence for Ohmic Dissipation?", AJ 155, 214 (2018),
doi:10.3847/1538-3881/aaba13 (DOI from search result), arXiv:1709.04539 (v2;
the equation numbers below are those of the v2 text I read; the legacy code
cites "eq. 34 of the preprint", which is v1 numbering).

The model adds a heating `epsilon F pi R^2` to the interior energy balance
`dE/dt = pi R^2 (epsilon F - 4 F_int)` (Eq. 4); `F` is the time-averaged
incident flux. Sample: 281 giants (Table 1 of the paper). Their best
parametric form (Eq. 35, the Gaussian model; DIC -1723, equal to the Gaussian
process one), "as a percent of flux and with flux in units of Gerg s^-1
cm^-2":

```text
epsilon[%] = 2.37 (+1.3/-0.26) exp( - (log10 F - 0.14 (+0.060/-0.069))^2
                                   / (2 (0.37 (+0.038/-0.059))^2) )
```

- Peak at `F = 10^0.14 = 1.38 Gerg s^-1 cm^-2`; the abstract (via the search
  result) says about 2.5 % near 1500 K, falling to about 0.2 % at 2500 K.
- The heating efficiency of the Gaussian form at the fluxes of Sect. 1.6
  **(computed here)**: HD 209458 b (1.03 Gerg, from `4 sigma Teq^4`) 2.2 %.
- At thermal equilibrium (`dR/dt = 0`) `T_int = epsilon^(1/4) T_eq` (Eq. 36).
- The "conventional cut-off" for non-inflated planets is 2x10^8 erg s^-1 cm^-2
  (`Teq` about 1000 K), per Thorngren et al. 2016 and Müller & Helled 2021.

**It is not a radius.** To get a radius one must integrate the evolution with
this heating (Thorngren used an evolution code with the Saumon et al. 1995
equation of state and the atmosphere grids of Fortney et al. 2007). That is
issue #621. Nothing in the paper turns `epsilon` into `Delta R` in closed form
(**not found**; the paper shows radii only as curves, Fig. 2).

### 3.2 Empirical offset: Sestovic, Demory & Queloz 2018

A&A 616, A76, doi:10.1051/0004-6361/201731454 (Crossref), arXiv:1804.03075.
Hierarchical Bayesian fit of 286 giants, radius against mass and flux, with
flux `F` in W m^-2. Eq. 1: `R/RJ = C` below a threshold flux, and
`C + A (log10 F - log10 Fs)` above. Eq. 11, the inflation `Delta R` in `RJ`:

| Mass range (MJ) | Delta R (RJ) | A | C | log10 Fs |
| --- | --- | --- | --- | --- |
| 0.10 to 0.37 | -0.33 (log10 F - 6.09) | -0.33 | 0.95 | 6.1 |
| 0.37 to 0.98 | 0.70 (log10 F - 5.5) | 0.70 | 0.98 | 5.52 |
| 0.98 to 2.5 | 0.52 (log10 F - 5.8) | 0.52 | 1.06 | 5.82 |
| 2.5 and above | 0.22 (log10 F - 5.2) | 0.22 | 0.99 | 5.2 |

(Mass boundaries 0.37 +- 0.02, 0.98 +0.04/-0.05, 2.5 fixed. The paper warns
that the lowest-mass bin fits poorly: planets of 0.1 to 0.37 MJ above 10^6
W m^-2 are rare, and the sign of `A` is negative there. Beyond about 1.6x10^6
W m^-2 no non-inflated planet exists in the 0.37 to 0.98 MJ bin.)

A check **(computed here)**: HD 209458 b (1.03x10^6 W m^-2, 0.73 MJ):
`Delta R = 0.36`, so 1.34 RJ from `C + Delta R` against 1.39 observed. Added to
the Fortney 1.11 RJ it gives 1.47 (+6 %). WASP-12 b (1.04x10^7 W m^-2,
1.47 MJ): `Delta R = 0.63`, so 1.69 RJ (-14 %) or 1.8 with the table (-8 %).
HD 189733 b (4.9x10^5 W m^-2, 1.13 MJ): below `Fs`, no inflation, as observed.

### 3.3 How to combine

- The Fortney tables are the baseline (the uninflated, evolved radius). Do not
  add `Delta R` of Sestovic to them as is: its baseline `C` is the population
  mean (0.98 to 1.06 RJ), not Fortney's radius, so the sum overshoots by about
  6 % in the one case tested.
- Two options for 2a, the choice is the user's. (a) `R = R_Fortney +
  Delta R` with `Delta R = A (log10 F - log10 Fs)` from Eq. 11 above `Fs` and
  zero below: an empirical, published, population-level offset, with the 6 to
  14 % scatter seen above, documented with `Source:` Sestovic et al. 2018,
  Eq. 11. (b) No inflation in 2a, stated in the science page, and the
  physical route (`epsilon(F)` of Thorngren & Fortney plus an evolution) goes
  to #621. I lean to (b): the heating law is the only physical mechanism the
  literature offers, and an offset of a different baseline hides it.
- The inflated radius must not feed back on the mass-radius draw of the core
  mass: Thorngren et al. 2016 (below) derived the heavy-element masses from
  planets with `F < 2x10^8 erg s^-1 cm^-2` only, because inflation biases them.

### 3.4 Heavy-element mass of a giant (for the draw bounds)

Thorngren, Fortney, Murray-Clay & Lopez 2016, "The Mass-Metallicity Relation
for Giant Planets", arXiv:1511.07854 (journal reference **not found** in the
text read): 47 transiting cool (non-inflated) giants, 20 Earth masses to 20
MJ; fit `M_z = (57.9 +- 7.03) M^(0.61 +- 0.08)` in Earth masses, with an
intrinsic multiplicative scatter 10^sigma = 1.82 +- 0.09. The unit of `M` is
not printed in the extracted text (Sestovic et al. 2018 restate it as
`M_C/M_earth = k x 57.9 x M^0.61`; Jupiter masses is the natural reading,
**to verify on the PDF** before use). Heavy elements are mixed in the
envelope in those models (a 10 Earth-mass core was assumed for the
comparison), so a "core mass" in the Fortney tables is a modelling
convention, not a measurement.

## 4. Ice giants (10 to 50 Earth masses)

What each source covers, 10 to 20 Earth masses at 60 to 90 % heavy elements:

| Source | Coverage | Result for Neptune (17.08 Earth masses) |
| --- | --- | --- |
| Fortney 2007 `.dat` | Core 10 gives 59 % at 17 Earth masses; no core between 10 and 100 Earth masses, and the 90 %-heavy curves of Fig. 7 are **not tabulated**. At 28 Earth masses the 25-core file is 89 % heavy: 0.378 RJ (4.24 Earth radii) at 9.5 AU, 4.5 Gyr. | Not covered. A 10-core planet of 17 Earth masses gives 0.508 RJ = 5.7 Earth radii (too large). |
| Lopez & Fortney tables 2 to 7 | up to 20 Earth masses and 20 % envelope, rock-iron core | at 0.1 F_earth, 4.5 Gyr, 15 % envelope: 4.2 (solar) or 4.5 (enhanced) Earth radii, +9 to +15 %; at 10 %: 3.67 or 3.81, -5 to -2 % **(computed here)**, against 3.88 (24 764 km / 6378 km) |
| Chen & Rogers 2016 Eq. 5 | flux outside range | invalid |
| Baraffe et al. 2008 Z = 0.9, isolated, 5 Gyr | 10 and 20 Earth masses: 0.357 and 0.403 RJ | about 0.393 RJ = 4.4 Earth radii (+13 %) by log interpolation **(computed here)** |

Uranus (14.54 Earth masses, 25 559 km = 4.01 Earth radii): Lopez & Fortney 4.2
(solar, 15 %), Baraffe 4.3 (+7 %).

**Proposal.** An ice giant between 10 and 20 Earth masses with an envelope of
5 to 20 % is a mini-Neptune of Sect. 2 (the tables reach it); above 20 Earth
masses the planet has at least 28 Earth masses and uses Fortney with a core of
10 to 25 Earth masses (ratio core/mass 36 to 89 %). The range of mass 12 to 28
Earth masses with an envelope of 20 to 60 % has no published table that I
found: the nearest are the two above. This is a gap, to state in the science
page, not to fill with a fit. A third route, a full integration, is #621.

(A 17 Earth-mass planet of Neptune's composition (65 % water) is not in the
Lopez & Fortney tables either: their core is rock and iron, so the radius from
the table is that of a dry core. The legacy code thinned the envelope by the
water fraction; I did not find a published rule for that.)

## 5. The molecular-to-metallic hydrogen transition and where it sits

### 5.1 Published pressures

- Morales, Hamel, Caspersen, Schwegler et al. 2010, PNAS 107, 12799,
  doi:10.1073/pnas.1007309107 (Crossref abstract): first-order liquid-liquid
  transition in hydrogen from ab initio simulations, "critical point of the
  transition at temperatures near 2,000 K and pressures near 120 GPa".
  Below the critical temperature the transition is first order; above, it is a
  continuous crossover. Jupiter's adiabat is hotter than that at these
  pressures (see Debras & Chabrier below), so inside a hot giant it is a
  crossover.
- Knudson et al. 2015, Science 348, 1455, doi:10.1126/science.aaa7471 (Europe
  PMC abstract has no numbers). Dynamic compression of deuterium on the Z
  machine: an abrupt insulator-metal transition at 280 to 305 GPa, as
  quoted by Desjarlais, Knudson & Redmer 2020, PRB 101, 104101 (arXiv:1912.05660,
  introduction).
- Celliers et al. 2018, Science 361, 677, doi:10.1126/science.aat0970
  (Europe PMC abstract): deuterium to 600 GPa, "onset of absorption of visible
  light near 150 GPa" and "metal-like reflectivity (exceeding 30 %) near 200
  GPa", all below 2000 K. The two experiments differ by about 95 GPa; the
  reconciliation (a temperature correction for latent heat) is disputed
  (Desjarlais et al. 2020: "approximately 42 GPa difference remains" for the
  1450 K case).
- Desjarlais et al. 2020 give their own calculated transition at about 204 GPa
  and 1345 K (deuterium).
- Debras & Chabrier 2019, ApJ 872 (DOI 10.3847/1538-4357/aaff65 from
  search result; page number not checked), arXiv:1901.05697: the transition
  (plasma phase transition or liquid-liquid transition) in "a domain
  P_c about 1 to 2 Mbar, for temperatures below T_c about 2 000 to 5 000 K"; the
  "ionization boundary located around P about 1 Mbar"; an inhomogeneous region
  at 0.1 to 2 Mbar of 10 to 15 % of the radius; the outer homogeneous envelope
  to about 93 % of the radius.
- Wahl et al. 2017, GRL 44, 4649 (doi:10.1002/2017GL073160, DOI from search
  result), arXiv:1707.01997: "onset of the metalization transition" at 50 GPa
  in their perturbation tests (their text), helium rain-out at 100 to 300 GPa
  on their adiabats.

**Proposal: one boundary pressure, 100 GPa (1 Mbar).** It is the number used
by the Juno-era models for the "ionization boundary" (Debras & Chabrier),
inside both the experimental onset (150 GPa, Celliers; 50 GPa onset in Wahl)
and the simulated critical point (120 GPa, Morales). State the range 50 to 200
GPa and that the laboratory results differ by up to 95 GPa. **Definition:**
the code treats it as a definition of the layer boundary, not as a law of
nature (`Definition:` or `Source:` per CLAUDE.md, with these papers). Pressure
only: the temperature does not enter (no temperature profile is computed).

### 5.2 Locating the pressure without an integration

For a giant, an index-one polytrope, `P = K rho^2` (Hubbard 2013, ApJ 768, 43,
doi:10.1088/0004-637X/768/1/43, Sect. III.3, Eqs. 61 and 62; the paper builds
one matched to Jupiter's mass and equatorial radius as a test of its method):

```text
rho(r) = rho_c sin(xi)/xi,   xi = pi r / R
rho_c  = (pi^2 / 3) rho_mean,   rho_mean = 3 M / (4 pi R^3)
K      = 2 G R^2 / pi,          P(r) = K rho(r)^2
```

Only Eqs. 61 and 62 are printed in Hubbard; the constants `rho_c` and `K` follow
from the Lane-Emden solution `theta = sin xi / xi` with `R = pi sqrt(K/(2 pi G))`
and the integral of `xi sin xi` over `[0, pi]`, which I derived **(computed
here)**; they are textbook (Chandrasekhar 1939) and not read from a source in
this session. Pressure decreases monotonically with `xi`, so the radius of
a given pressure is a bisection or a Brent root in `(0, pi)`, at the same cost
as one function evaluation.

Is it acceptable? **Only for Jupiter-like planets, and only roughly.**

- Against Debras & Chabrier's Jupiter (their region II, P of 0.1 to 2 Mbar,
  starting at about 93 % of the radius and spanning 10 to 15 %): the polytrope
  gives 0.1 Mbar at 0.95 R and 2 Mbar at 0.80 R, 1 Mbar at 0.853 R
  (61 000 km) **(computed here, equatorial radius, 318 Earth masses)**.
  Agreement is to about 0.02 R at the top and to the listed span at the bottom.
  Duarte et al. 2018 (quoted by D&C) put `R_50`, where the conductivity jumps
  as H2 recombines, at about 0.9 RJ, a different quantity but of the same
  region: the n = 1 polytrope reaches 0.5 Mbar at 0.893 R and 1 Mbar at 0.853
  R.
- Central pressure: the polytrope gives 36 to 40 Mbar for Jupiter **(computed
  here)**. I did not find the published central pressure of the Juno-era
  models in the papers read (**not found**).
- Saturn and Neptune: **not validated**. The same formulas give 1 Mbar at
  0.68 R (Saturn, 95.1 Earth masses, 60 268 km) and 0.70 R (Neptune)
  **(computed here)**; I found no published pressure profile to check them
  against, and a planet with a large core is not an n = 1 body.
- Not applicable to mini-Neptunes with a thin envelope: the weight of the
  envelope is carried by the core's gravity. The thin-shell estimate
  `P_base = G M_core M_env / (4 pi R_core^4)`, with `R_core = M_core^0.25`
  (Lopez & Fortney Eq. 1) **(computed here)**, gives 0.3 Mbar at the base of
  a 5 % envelope on a 5 Earth-mass core, 0.6 Mbar at 5 % and 1.3 Mbar at 10 %
  on a 10 Earth-mass core, 2.9 Mbar at 20 %. So the 1 Mbar boundary falls
  inside the envelope of a mini-Neptune only from about 10 % of envelope at 10
  Earth masses; thinner ones are molecular throughout. This is my estimate,
  unsourced beyond the hydrostatic relation, and rough (a thick envelope is
  not a thin shell).
- Alternative with the same cost, no polytrope: the envelope of constant `g`
  and the ideal-gas isotherm. It is worse for a giant (gravity falls by half).

Proposed: n = 1 polytrope of the whole planet (mass, `R_tot` of the table) for
`M > 50` Earth masses and a core below 25 % of the mass; the thin-shell
estimate for a planet with a thin envelope; nothing for an ice giant (Sect. 4),
the layer boundary there is "not computed".

### 5.3 Jupiter and Saturn layers, for the tests

- Debras & Chabrier 2019: outer homogeneous envelope to about 93 % of
  Jupiter's radius, inhomogeneous region at 0.1 to 2 Mbar of 10 to 15 % of the
  radius (their conclusion); metallization region about 0.89 to 0.92 RJ; diluted
  core to about 85 %; the "valid models" have a boundary region of
  about 15 % of the radius; the molecular H2 "fully recombines" so that the
  radius `R_50` of the dynamo is about 0.9 RJ (Duarte et al. 2018, quoted).
- Wahl et al. 2017: Jupiter's core of 7 to 25 Earth masses with a dilute core
  spreading over a large share of the radius (abstract, via the search result).
- Mankovich & Fuller 2021, Nature Astronomy 5, 1103,
  doi:10.1038/s41550-021-01448-3 (DOI from search result), arXiv:2104.13385:
  Saturn's stably stratified diffuse core reaches 0.59 +- 0.01 R_S (0.6) and has
  55.1 +- 1.7 Earth masses, of which 17.4 +- 1.2 Earth masses of ice and rock;
  total heavy elements 19.1 +- 1.0 Earth masses; half the heavy mass within
  0.32 +- 0.01 R_S; equatorial radius 60 268 km at 1 bar.

Test ideas (the tolerances are mine, not from a source): Jupiter's radius from
Table 4 at a core of 10 to 25 Earth masses within 3 %; the core of Saturn that
reproduces 0.843 RJ between 10 and 20 Earth masses (heavy elements 19.1 +- 1.0
Earth masses in Mankovich & Fuller); the 1 Mbar radius of the Jupiter
polytrope inside the published region of Debras & Chabrier (0.80 to 0.93 R for
2 to 0.1 Mbar).

## 6. Water worlds hotter than 355 K

The 355 K of `science::water` is the ice VI, ice VII and liquid triple point
(`VI_VII_LIQUID_TEMPERATURE`, source in its doc comment), the limit above which
no ice exists at any pressure. It is not the limit of a steam atmosphere. The
published relations are driven by irradiation, not by a surface temperature.

### 6.1 Turbet et al. 2020

M. Turbet, E. Bolmont, D. Ehrenreich et al., "Revised mass-radius
relationships for water-rich rocky planets more irradiated than the runaway
greenhouse limit", A&A 638, A41 (2020), doi:10.1051/0004-6361/201937151
(Crossref), arXiv:1911.08878. A 1-D radiative-convective model of a pure
steam atmosphere over a dry rocky core (cores of Zeng et al. 2016,
doi:10.3847/0004-637X/819/2/127), the transit radius at the top of the moist
convective layer, `P_transit = 0.1 Pa`.

Method (Appendix D "quick guide"): take a dry core mass `M_core` and radius
`R_core`; planet mass `M_planet = M_core / (1 - x)`; `R_planet = R_core +
z_atm`. With `x` the water mass fraction, `S_eff` the stellar flux in Earth
units, `g_core = G M_core / R_core^2`:

```text
z_atm = ( 1 / [ ln( x/(1-x) * g_core^2 / (4 pi G P_transit) )
                * R_gas T_eff / (mu_H2O g_core) ]  -  1/R_core )^-1      (Eq. 2)

log10 T_eff = b1 + b2 X + b3 Y + b4 Z + b5 X Y + b6 Y^2 + b7 X^3
            + b8 X^2 Y + b9 X Y^2 + b10 Y^4                              (Eq. 3)
X = (log10 x   - a1)/a2,   Y = (log10 g_core - a3)/a4,
Z = (log10 S_eff - a5)/a6                       (g in m s^-2, S_eff in Earth units)
```

Constants: `R_gas = 8.314 J K^-1 mol^-1`, `mu_H2O = 1.8e-2 kg mol^-1`,
`G = 6.67e-11`, `P_transit = 0.1 Pa` (about, "roughly equal to 10^-1 Pa"; Turbet
sets it to that). The logarithm of Eq. 2 is natural: I derived the isothermal
hydrostatic profile with the gravity falling as `1/r^2` and obtain exactly Eq. 2
with `ln` **(computed here)**.

Table 2 coefficients (as printed): a1 -3.550, a2 1.310, a3 1.099, a4 0.4683,
a5 0.7664, a6 0.4224; b1 2.846, b2 0.1555, b3 0.08777, b4 0.06045, b5 0.01143,
b6 0.01736, b7 0.01859, b8 0.04314, b9 0.03393, b10 -0.01034.

Validity (the paper): irradiation 1 to 30 S_earth **above the runaway
greenhouse limit** (which depends on the star, Kopparapu et al. 2013, and on
the planet, Kopparapu et al. 2014), surface gravity 0.2 to 6 g_earth, water
vapour pressure 2.7 bar to 27 kbar, surface temperature 300 to 4300 K;
maximum error about 10 %, "a few percent" for most of the space. The paper
tested masses 0.2 to 2 Earth masses and water fractions 0.01 to 5 %
(Aguichine et al. 2021, introduction). The additive approximation
`R = R_core + z` is good to 1 % for 5 % water (Appendix A); beyond that the
water weight compresses the core (limit: basal pressure well below about
10^10 Pa). Planets lighter than about 0.5 Earth mass cannot keep more than a
few percent of water.

Check **(computed here)**: Eqs. 2 and 3 for a 0.285 Earth-mass core of
0.70 Earth radii (my assumption for the dry radius, not sourced), 5 % water,
`S_eff` 1.0 to 1.5, give a planet radius of 1.16 to 1.21 Earth radii, against
"the transit radius lies around 1.2 R_earth" for a 0.3 Earth-mass silicate
core with 5 % water just above the threshold (paper, Sect. 3.1). So the
implementation reproduces the published statement.

### 6.2 Aguichine et al. 2021

A. Aguichine, O. Mousis, M. Deleuil, E. Marcq, "Mass-Radius Relationships for
Irradiated Ocean Planets", ApJ 914, 84 (2021), doi:10.3847/1538-4357/abfa99
(DOI from search result), arXiv:2105.01102. An interior (Mazevet et al. 2019
water equation of state), a condensed supercritical water layer and a steam
atmosphere, connected at 300 bar (P_crit = 220.67 bar); Earth-like rock and
iron; the radius at `P_top = 0.1 Pa` (the same level as Turbet); `T_irr`
irradiation temperature. It supersedes Mousis et al. 2020 (ApJL 896, L22,
doi:10.3847/2041-8213/ab9530), which gives no fit.

- Fit (Eq. 29), per composition and `T_irr`: `log10 R_p = a log10 M_p + b +
  exp(-d (log10 M_p + c))`, `R_p`, `M_p` in Earth units; mean absolute error
  0.01 to 1 %, worst point 2.3 %.
- Data: Zenodo `doi:10.5281/zenodo.4552188`, CC-BY 4.0 (read from the record
  API): `fit_coefficients.dat` (140 256 bytes, 1 000 rows of `x_core'`, `x_H2O`,
  `T_irr`, a, b, d, c, the number of masses (of 20), MAE, maximum error) and
  `mr_all.dat` (2 382 642 bytes, the models). **The columns are ordered
  `a b d c`, not `a b c d`.** I implemented the fit **(computed here)** and
  read it back: GJ 9827 d (M = 4.04 Earth masses, T_irr 686 K, observed
  R = 2.02 +- 0.05, from the paper's Table 3) gives 1.96 at 20 % water and 2.08
  at 30 % (core fraction 0.3, 700 K), consistent with the 20 +- 10 % they
  report.
- Grid: `x_core'` 0 to 0.9 (core share of core plus mantle), `x_H2O` **0.1 to
  1.0** (steps of 0.1), `T_irr` **400 to 1300 K** (steps of 100 K), masses 1
  to 20 Earth masses. The `number` column shows which masses succeeded (for
  instance 17 of 20 at the smallest water fraction and 400 K).
- Limits: low gravity cases are excluded (Sect. III.3), an irradiation
  temperature beyond 1300 K and a water fraction below 10 % are outside.
  The paper discusses atmospheric escape: water is much less subject to loss
  than hydrogen.

### 6.3 Proposed treatment of the bulk radius

For a planet with a water inventory above the steam threshold (a surface
above the water critical temperature, or an irradiation above the runaway
greenhouse limit):

1. Dry core of mass `(1 - x) M` from `science::structure`, radius `R_core`.
2. Water fraction `x` below about 5 %: add `z_atm` of Turbet Eqs. 2 and 3
   (core gravity, `S_eff`), valid for gravity 0.2 to 6 g_earth.
3. Water fraction from 10 %: interpolate the Aguichine grid, a `T_irr` of
   400 K or more, mass 1 to 20 Earth masses.
4. Between 5 and 10 %: interpolate the two radii in `x`. Neither paper covers
   it, so this is an interpolation, not a model; state the gap.
5. Below the runaway greenhouse limit the water stays a condensed layer: the
   existing structure with the water layer applies.

This replaces "mixing the whole mass with the dry radius", the issue of goal 1,
by a core of mass `(1 - x) M` plus a sourced steam layer. The criterion to
choose the regime is not the 355 K. I found no single published criterion in
surface temperature: Turbet's is the stellar flux against the Kopparapu
runaway greenhouse limit, Aguichine's the irradiation temperature (grid from
400 K). The planet's
`equilibrium_k` (or the flux the engine already has) is enough for both.

## 7. What was not found

- A licence for the Fortney 2007 `.dat` files and for the Lopez & Fortney
  tables (they are in a published paper; the machine-readable form of the
  latter does not exist, I extracted it from the HTML of the paper).
- A table of giants with 60 to 90 % heavy elements and an irradiation
  (Fortney Fig. 7 curves are not in the `.dat`), hence no ice-giant table.
- A closed-form radius for hot-Jupiter inflation from `epsilon`.
- A published pressure profile of Saturn or Neptune to validate the polytrope.
- The Jupiter-model central pressure.
- Chen & Rogers's radius definition level; the unit of `M` in Thorngren et al.
  2016's relation; the ApJ volume of Thorngren et al. 2016 and the DOIs of the
  papers marked "from search result" (Crossref answered with HTTP 429 for
  some).
- A published rule for the 5 to 10 % water gap and for the mass range of 12 to
  28 Earth masses with 20 to 60 % envelope.

## Decisions (user, 2026-11-21)

1. **No hot-Jupiter inflation in 2a.** The Fortney et al. 2007 tables
   are used as they are; inflation goes to the integrated structure of
   #621 (Thorngren & Fortney 2018 heating efficiency). Hot Jupiters stay
   near 1 R_J until then.
2. **Ice-giant envelope upper bound lowered to 0.20**, the upper edge of
   the Lopez & Fortney 2014 tables, instead of extrapolating to the 0.25
   of Helled et al. 2011.
3. **Steam and supercritical water worlds move to PR 2d.** 355 K is the
   ice VI-VII-liquid triple point, not a steam criterion; the published
   criteria (runaway limit, Turbet et al. 2020; irradiation temperature,
   Aguichine et al. 2021) need the climate of 2d. PR 2a only removes the
   355 K trigger, with no interim threshold.
4. **The Fortney et al. 2007 data are versioned in the repository**,
   rebuilt from the author's `.dat` files with the paper's citation and
   a provenance note, as the stellar tables are. The Lopez & Fortney
   2014 tables are transcribed from the paper the same way.
