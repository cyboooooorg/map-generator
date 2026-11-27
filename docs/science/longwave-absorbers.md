# Which gases does the infrared column still miss?

Seen from: the mean surface temperature of every body whose air holds
nitrogen, oxygen, methane or nitrous oxide, and the cause trees that
explain it. Study 36 (issue #703) wrote this page before any code; the
article on the column itself is
[How does a planet's air set its outgoing heat and its
albedo?](radiative-transfer.md).

## Short answer

The app's infrared column holds the lines of water vapour and carbon
dioxide, their continua, and the brief pairs that colliding molecules form
(CO₂–CO₂, and since study 36 N₂–N₂, O₂–O₂, O₂–N₂, N₂–CH₄ and CH₄–CH₄);
the pairs replaced a grey stand-in for Titan's methane. Earth's real air
also traps heat with ozone, methane's and nitrous oxide's (N₂O) lines and
halocarbons. Line-by-line codes put methane, N₂O and the halocarbons at
about 3.39 W/m² of Earth's clear-sky outgoing infrared at present-day
amounts (3.6 with the N₂ and O₂ pairs, which the column now holds), and
methane and N₂O at 2 to 3 W/m² at pre-industrial ones. Two findings
matter more than the numbers:

- The model's Earth carries **no methane and no N₂O** at all: the chain
  has no law that makes them. Adding their lines to the column changes
  nothing for the reference Earth until a law of life's methane and N₂O
  exists. Only the nitrogen and oxygen pairs act at once: about +0.1 to
  +0.2 K.
- The **nitrogen and methane pairs** of HITRAN's collision-induced files
  put a spectral Titan at 92.30 K without any fitted constant (measured:
  93.65 K; 91.81 K since N₂ condenses in its upper troposphere, issue
  #741), and replaced the calibrated grey opacity the column needed
  before. On
  thick nitrogen airs the pairs grow with the pressure squared: −3.3 W/m²
  at 5 bar.

## The science

### What each absorber is worth on Earth

The like-for-like quantity is the change of the clear-sky outgoing
infrared at the top of the air when one gas is taken out, the others and
the temperatures held (an instantaneous removal). Three line-by-line
sources give it:

| Absorber | Removal at present-day amounts, W/m² | Pre-industrial to present, W/m² |
| --- | --- | --- |
| Methane (CH₄) | 1.5 (Hogan & Matricardi 2020); 1.87 (Höpfner et al. 2012) | 0.613 (5 line-by-line codes, s.d. 0.003) |
| Nitrous oxide (N₂O) | 1.5; 1.88 | 0.204 (s.d. 0.001) |
| CFC-11 equivalent, CFC-12 | 0.22, 0.17 | all halocarbons 0.53 (s.d. 0.036) |
| N₂ pairs (collision-induced) | 0.15; 0.17 | none (N₂ does not change) |
| O₂ pairs (collision-induced) | 0.096; 0.11 | none |
| Water's foreign continuum (H₂O–N₂, H₂O–O₂) | already in the column (MT_CKD 4.3) | none |

- **Hogan & Matricardi 2020** (Table 4, read in its spreadsheet): LBLRTM
  12.8, AER's line file 3.6 (HITRAN 2012 with AER's changes), MT_CKD 3.2,
  the O₂ and N₂ collision bands; the mean of 50 profiles, unweighted,
  17 of them chosen for extreme temperature, humidity or ozone. Present
  day is 2020: 415 ppm of CO₂, 1921 ppb of CH₄, 332 ppb of N₂O, 861 ppt of
  CFC-11 equivalent, 495 ppt of CFC-12 (their Table 2). Their
  pre-industrial scenario is 280 ppm, 700 ppb, 270 ppb and 32 ppt of
  CFC-11 equivalent.
- **Höpfner et al. 2012** (Table 1): the KOPRA line-by-line code on
  HITRAN 2008 and MT_CKD 2.5.2, cloud-free columns on a 5° grid of one
  day, standard profiles for the trace gases (1.8 ppm of CH₄). N₂ and O₂
  each take 2.8 W/m² out of a column that holds only them, but water and
  CO₂ hide most of their bands in real air.
- **The RFMIP benchmark** (Pincus et al. 2016, 2020; their published
  fluxes, read here): the clear-sky outgoing infrared of 100 profiles
  weighted to the global mean, by five line-by-line codes (ARTS 2.3,
  GRTCODE, RFM-DISORT, LBLRTM 12.8, 4A/OP 1.5). Present day is 2014
  (1831 ppb of CH₄, 327 ppb of N₂O); pre-industrial is 808 ppb and
  273 ppb. The differences in the table are their outgoing fluxes,
  experiment "PI CH4" or "PI N2O" less "PD" (derived here). RRTMGP, the
  k-distribution parameterization of the same group, gives 0.563 and
  0.180 W/m² on the same profiles.
- **Collins et al. 2006** (RTMIP, Table 8, case 3a-1a): adding 806 ppb
  of CH₄ and 275 ppb of N₂O to an air that had none, on the AFGL
  mid-latitude summer profile (294 K surface), lowers the outgoing
  infrared at the top by 3.63 W/m² (line-by-line mean, s.d. 0.05). This
  is the only published joint value from zero at pre-industrial amounts
  found; the profile is warm and moist, so a global mean is smaller.

**At pre-industrial amounts** (derived, the sources do not give it): the
removal at present day less the RFMIP rise, and less the rise from
RFMIP's present day to Hogan & Matricardi's (Etminan et al. 2016's
square-root terms, 0.043 (√1921 − √1831) = 0.045 W/m² for CH₄ and
0.117 (√332 − √327) = 0.016 for N₂O), gives about 0.84 W/m² for CH₄ and
1.28 for N₂O on Hogan & Matricardi's profiles, 1.26 and 1.68 from
Höpfner et al.'s. The two gases' bands overlap (Etminan et al. 2016 fit
an overlap term for it), so together they take a little less than the
sum: about 2.0 to 2.8 W/m² clear-sky. Halocarbons were nearly absent
before industry: 32 ppt of CFC-11 equivalent in Hogan & Matricardi's
pre-industrial scenario, against 861 ppt in 2020.

**Which ones matter** against the column's excess of 5 to 7 W/m² over
CERES (study 32): methane and N₂O, each about 1 W/m² at pre-industrial
amounts and 1.5 at present-day ones. Halocarbons matter only for a
comparison with CERES's years (0.39 W/m²). The nitrogen and oxygen pairs
are small on Earth (0.25 to 0.28 W/m² in the literature), but cheap,
sourced, and large on thick or dry airs. Ozone is study 34 (#704).

### Methane at high amounts, and in the starlight

Byrne & Goldblatt 2014 (line-by-line on HITRAN 2012, SMART, a
pseudoadiabat to an isothermal 203 K stratosphere, 289 K surface,
clouds included, forcing at the tropopause) find for methane added to
1 bar of N₂: an infrared forcing of 8.6 W/m² at 10⁻⁴ of the air
(100 ppm), and a shortwave forcing of about −1.4 W/m² at 10⁻⁴, −6.7 at
10⁻³ and −20 at 10⁻², mostly below 11 502 cm⁻¹, much stronger than with
HITRAN 2000. The net forcing peaks at 10.3, 9 and 8.3 W/m² under 0.5, 1
and 2 bar of N₂: past about 10⁻³ more methane cools.

On Earth today methane's starlight is small: Hogan & Matricardi give
0.32 W/m² at the top for its removal over a daytime mean of 680.5 W/m²
(about −0.0005 of Bond albedo, derived), Collins et al. 0.15 W/m² for
0 to 806 ppb with N₂O on their summer profile. Etminan et al. 2016 find
that its shortwave bands raise methane's forcing by about 15 % through
the stratosphere's adjustment, a term a single column at fixed
stratosphere does not hold.

### Nitrogen and oxygen pairs, and Titan

A pair of colliding N₂ or O₂ molecules has a brief dipole and absorbs
where neither molecule can alone: N₂'s roto-translational band below
650 cm⁻¹, its fundamental near 2330 cm⁻¹, O₂'s fundamental near
1550 cm⁻¹. The absorption grows with the product of the two densities.
HITRAN's 2024 collision-induced files (Karman et al. 2019 describe the
section; the 2024 references list the source of each block):

| File | Bands, cm⁻¹ | Temperatures, K | Source of the blocks |
| --- | --- | --- | --- |
| `N2-N2_2021.cia` | 0-650; 1850-3000; 4300-5000 | 70-400; 228-363; 200-330 | Chistikov et al. 2019 (trajectories); Lafferty et al. 1996 and Baranov et al. 2005; Hartmann, Boulet & Toon |
| `O2-O2_2024.cia` | 1150-1950 (and visible bands) | 193-353 | Baranov et al. 2005 and others |
| `O2-N2_2024.cia` | 1300-1850 (O₂ band); 1850-3000 (N₂ band) | 193-356; 228-363 | Thibault et al. 1997; Orlando et al. 1991; N₂ band estimated from N₂-N₂ (Lafferty et al. 1996) |
| `N2-CH4_2024.cia` | 0-800 | 70-400 | Finenko et al. 2022 (built for Titan) |
| `CH4-CH4_2011.cia` (alternate set) | 0-990 | 40-400 | Borysow & Frommhold 1987 |

On Titan these pairs make most of the greenhouse: Bézard & Vinatier 2019
fit Cassini's spectra with N₂-N₂, N₂-CH₄, N₂-H₂ and CH₄-CH₄ pairs, and
find that the older N₂-CH₄ coefficients must be raised by about 52 % at
70-85 K and the N₂-N₂ ones beyond 110 cm⁻¹. The 2024 N₂-CH₄ file is the
trajectory-based one of Finenko et al. 2022 that answers it (whether it
holds the whole 52 % was not checked).

### The column's cost of a new gas

`tools/opacity-table` sums each gas's HITRAN lines bin by bin (20 cm⁻¹,
0-5000 cm⁻¹), sorts the coefficients into eight g-points per bin, on 15
pressures and 16 temperatures (100 to 900 K), one set per gas and
broadener (`crates/science/data/OPACITY.md`). Each set adds 250 × 8 × 15
× 16 codes of two bytes, 0.96 MB, to `opacity.bin` (3.84 MB with its
four sets). CH₄ and N₂O broadened by air make 5.76 MB, with CH₄
broadened by itself 6.72 MB. HITRAN's ids are 32-35 for CH₄ and 21-25
for N₂O (HITRAN isotopologue table).

The column combines gases by multiplying each gas's transmission summed
over its own g-points (random overlap, Lacis & Oinas 1991), so a gas
with lines adds a cost linear in its bins, not a product of g-point
pairs; no equivalent-extinction step is needed. CH₄ and N₂O absorb in a
few bands only (Höpfner et al. 2012 plot each gas's transmission in
their Fig. 1), so holding their g-points
only for the bins where the table has lines keeps the added cost small;
how many bins that is will be read from the built table.
A collision-induced pair costs one optical depth per bin, as CO₂'s pairs
do now.

## In the app

- **Absorbers.** The infrared column holds water and CO₂ lines,
  MT_CKD 4.3's water continuum (self, and foreign with every other gas of
  the air, `spectrum.rs`), MT_CKD's CO₂ continuum, and HITRAN's
  collision-induced pairs (`spectrum::PAIRS`): CO₂–CO₂ (`co2-cia.tsv`),
  and N₂–N₂, O₂–O₂, O₂–N₂, N₂–CH₄ and CH₄–CH₄ (`pairs-cia.tsv`, study 36's
  first pull request, issue #703). NH₃ and SO₂ are in the air but have no
  infrared absorption, and methane none but its pairs.
- **The pairs.** `tools/opacity-table --continua` bins each HITRAN file
  (SHA-256 in `crates/science/data/OPACITY.md`) on the column's 20 cm⁻¹
  bins up to 5000 cm⁻¹, at every 10 K from 70 to 400 K: each band's
  blocks are averaged over the cell and interpolated linearly in
  temperature, held at their ends. A pair's optical depth per `ln p` is
  `k(ν, T) n_a n_b dz/d ln p` (`k` in cm⁵/molecule², `n` the two gases'
  number densities, eq. 3 of HITRAN's `CIA_Readme_2024.pdf`), one depth per bin
  added to every g-point, in the fast path and the double-precision
  reference alike. It grows as the pressure squared, so the air above
  the column's top takes half the top level's rate, as the lines and the
  continua do. The shortwave table takes the CO₂–CO₂ pairs only.
- **Titan's greenhouse is the pairs.** The grey stand-in (`CH4_OPACITY`
  15.1, `CH4_SATURATION_BAR` 0.01 bar, both calibrations on Titan) is
  gone. The sourced Titan column settles at 92.296 K against the
  measured 93.65 ± 0.25 K (it was 93.60 K with the stand-in). The
  tier-1 test allowed 1.4 K under the measurement, a calibration on the
  measured gap: its floor was 93.65 − 0.25 − 1.4 = 92.00 K, 0.3 K under
  the column's 92.30 K. Since N₂ condenses in the column's upper
  troposphere (issue #741) the column settles at 91.81 K, and the user
  refitted the bound by the same rule on 2026-11-27: 1.9 K, a floor of
  91.50 K, 0.31 K under it. Part of the gap is the column's own
  troposphere, 4 to 8 K colder than Huygens's between 0.3 and 0.9 bar
  (issue #750), which hides 1.2 to 2.3 K of the missing warming and
  would cool the column once fixed (study 39,
  [Nitrogen phases](nitrogen-phases.md#titans-column-against-huygens)),
  so the bound is one-sided only for the missing absorbers: N₂–H₂ pairs
  (issue #720), methane's own lines, the haze's infrared, and the
  methane of the lower troposphere: the column applies the cold trap's
  mixing ratio (`radiating_methane_bar`) at every level, so it holds
  about 40 % of the sourced surface CH₄ there (a review's estimate,
  derived, issue #744), likely a large share of the gap since N₂–CH₄ is
  linear in CH₄. Each of them can only warm it.
- **Which airs hold methane.** Methane is outgassed and survives only
  under an equilibrium temperature of 150 K (`METHANE_SURVIVES`), capped
  by its saturation: Titan's chain air keeps 0.091 bar. No law makes
  biogenic methane, and N₂O is not a gas of the model (the legacy
  removed methane from an oxygenated air and added only O₂).
- **What the study measured** on the column of `main` (1e122416), with
  HITRAN's pair files binned like `co2-cia.tsv` in a scratch probe
  (removed before the commit); the committed table is the probe's to the
  last digit:

| Case | Without | With the pairs | Change |
| --- | --- | --- | --- |
| Earth's air (N₂ 0.7808, O₂ 0.2095 of 1.014 bar), 288 K, 280 ppm, clear | 274.718 | 274.312 | −0.406 W/m² (N₂ band 0.161 + N₂ fundamental 0.111 + O₂ fundamental 0.134) |
| Same, all-sky | 241.813 | 241.476 | −0.337 W/m² |
| Same at 285.82 K, all-sky | 236.885 | 236.532 | −0.353 W/m² (N₂-N₂ alone −0.205) |
| N₂ and O₂ only, no water or CO₂, 288 K | 390.105 | 383.731 | −6.37 W/m² (Höpfner et al.: 2.8 + 2.8 on their profile) |
| 2, 5, 10 bar of N₂, Earth's water, 288 K | 263.03, 249.97, 240.95 | | −1.12, −3.30, −6.44 W/m² |
| Titan's sourced column (1.467 bar, 5.65 % CH₄), settled | grey stand-in: 93.60 K; nothing: 74.88 K | N₂-N₂: 83.95 K; + N₂-CH₄: 92.23 K; + CH₄-CH₄: 92.30 K | measured 93.65 ± 0.25 K |

- **What the code gives** (tier-1 tests, on the merged column): Earth's
  measured air at 288 K and 280 ppm, clear, loses 0.408 W/m² to its N₂
  and O₂ pairs (`earths_nitrogen_and_oxygen_pairs_take_what_line_by_line_codes_find`,
  against 0.246 to 0.28 from the line-by-line sources plus the 0.15 W/m²
  of issue #718's excess, a calibration); its clear sky falls from
  274.63 to 274.22 W/m² in the CERES test, whose window, without the N₂
  and O₂ terms of `MINOR_GASES_W_M2` now that the column holds them,
  runs from 271.83 to 286.08 W/m². N₂ and O₂ alone lose 6.375 W/m²
  (`an_air_of_nitrogen_and_oxygen_alone_loses_what_hopfner_finds`,
  Höpfner et al.'s 5.6 within the 26 % two of their line-by-line codes
  differ by on N₂, a calibration of the band's width). Titan:
  `titans_pairs_hold_its_column_just_under_its_measured_surface`.

The column's N₂ pairs take 0.27 W/m² on Earth where the line-by-line
sources give 0.15 to 0.17; the excess sits in the roto-translational
band below 650 cm⁻¹, which water hides. The cause is not isolated: the
column's dry upper air (study 32), HITRAN's newer N₂-N₂ far wing, or the
missing N₂-O₂ roto-translational band, which HITRAN does not give
(issue #718).

## Where it departs, and what is next

- **Earth's methane and N₂O need a law first.** A column with their
  lines changes the reference Earth only once its air carries them. The
  pre-industrial amounts are measured (ice cores, through the scenarios
  above); a law that makes them for any inhabited planet is not, and is
  not written here.
- **Titan's grey stand-in is gone.** Spectral N₂-N₂ and N₂-CH₄ pairs
  reach 92.3 K without a fit. What is still missing is N₂-H₂ (the model's
  air has no H₂, issue #720), methane's own lines, the haze's infrared,
  and the methane of the lower troposphere, which the column holds at
  the cold trap's mixing ratio at every level (`radiating_methane_bar`,
  about 40 % of the sourced surface CH₄, a review's estimate, issue
  #744).
- **The temperature grid** of `opacity.bin` starts at 100 K; Titan's
  column runs from 69 to 94 K and takes 100 K lines. The pairs' table
  starts at 70 K and ends at 400 K: Titan's 69 K skin takes the 70 K
  column, and a Venus at 735 K the 400 K one (each band held at its
  warmest block, 330 to 400 K).

## Decisions (user, 2026-11-25)

- **Earth's methane and N₂O** get a study of their own first, for a law
  of the gases life makes. No imposed value.
- **The N₂ and O₂ pairs ship first, alone**, as the first pull request of
  issue #703.
- **Titan's grey stand-in is replaced by the pairs** (N₂-N₂, N₂-CH₄,
  CH₄-CH₄), accepting 92.30 K against the measured 93.65 K: the test is
  widened, and what is missing is named (N₂-H₂ pairs, methane's own lines,
  the haze's infrared). `CH4_OPACITY` and `CH4_SATURATION_BAR` go.
- **Methane's lines now, N₂O's later.** Ozone's longwave (issue #704,
  study 34) joins the lines work, so `opacity.bin` is rebuilt once.
- **Methane's near infrared** in `shortwave.bin` is a separate issue,
  after #655.
- **Tolerance of the line-by-line tests:** no worse than RRTMGP against
  the same line-by-line references, gas by gas, on RFMIP's step from
  pre-industrial to present day: methane 0.563 against 0.613 W/m²
  (−8.2 %), N₂O 0.180 against 0.204 (−11.8 %), from the RFMIP fluxes of
  Pincus et al. 2016 and 2020, tagged `Source:`.

  Before fixing it, CKDMIP was checked for a published forcing error of a
  k-distribution that would apply directly. Hogan & Matricardi 2020 give
  only that RRTMG underestimates the longwave forcing of cutting methane
  to 350 ppb by about a factor of two (fixed in RRTMGP), and that its
  shortwave forcing by CO₂ and methane is 25 % to 45 % low. Hogan &
  Matricardi 2022 (ecCKD) show the longwave forcing errors only in their
  Fig. 6, and in the text say the 32-term models capture them
  "accurately"; their 16-term model overestimates N₂O's shortwave
  forcing by about a factor of two. Neither gives a longwave figure per
  gas that applies to the column, so the RRTMGP figures were chosen.

## What we do not know

- A global-mean, line-by-line clear-sky removal of methane and N₂O at
  pre-industrial amounts: only the summer profile of Collins et al. 2006
  and derived values exist.
- Why the column's N₂ pairs take about 0.1 W/m² more than the
  line-by-line sources on Earth.
- How much of Titan's last 1.35 K belongs to N₂-H₂, to methane's lines,
  to the haze, or to the methane the column leaves out of its lower
  troposphere (issue #744); that last one is likely a large share, since
  N₂-CH₄ absorbs in proportion to the methane.

## Sources

1. Bézard, B., Vinatier, S. 2019, "On the H₂ abundance and ortho-to-para
   ratio in Titan's troposphere", *Icarus*, doi:10.1016/j.icarus.2019.03.038
   (accepted version, arXiv:1909.10458, read).
2. Byrne, B., Goldblatt, C. 2014, "Radiative forcings for 28 potential
   Archean greenhouse gases", *Clim. Past* 10, 1779,
   doi:10.5194/cp-10-1779-2014 (read, §2.2-2.4 and §3.2).
3. Collins, W. D. et al. 2006, "Radiative forcing by well-mixed
   greenhouse gases: estimates from climate models in the IPCC AR4",
   *J. Geophys. Res.* 111, D14317, doi:10.1029/2005JD006713 (White Rose
   copy read, Tables 2, 8 and 9).
4. Etminan, M., Myhre, G., Highwood, E. J., Shine, K. P. 2016, "Radiative
   forcing of carbon dioxide, methane, and nitrous oxide: a significant
   revision of the methane radiative forcing", *Geophys. Res. Lett.* 43,
   12614, doi:10.1002/2016GL071930 (read, §3.2 and Table 1).
5. Finenko, A. A. et al. 2022, *Astrophys. J. Suppl.* 258, 33,
   doi:10.3847/1538-4365/ac36d3 (as HITRAN's CIA references give it; not
   read).
6. HITRAN collision-induced absorption, files `N2-N2_2021.cia`
   (SHA-256 `f0525f75…849d`), `O2-O2_2024.cia` (`0146acfe…1efd4961`),
   `O2-N2_2024.cia` (`90851fbf…3799f`), `N2-CH4_2024.cia`
   (`7056ee0b…12523`), alternate `CH4-CH4_2011.cia` (`12de7692…403a`),
   <https://hitran.org/cia/>, downloaded 2026-11-25 (and 2026-11-26,
   the same bytes), with `CIA_References_2024.pdf` (read) and
   `CIA_Readme_2024.pdf` (its eq. 3, the absorption of a mixture, read).
7. HITRAN isotopologue metadata, <https://hitran.org/docs/iso-meta/>
   (read 2026-11-25).
8. Hogan, R. J., Matricardi, M. 2020, "Evaluating and improving the
   treatment of gases in radiation schemes: the Correlated K-Distribution
   Model Intercomparison Project (CKDMIP)", *Geosci. Model Dev.* 13, 6501,
   doi:10.5194/gmd-13-6501-2020 (read, with Tables 2 and 4 in their
   spreadsheets and the evaluation of RRTMG).
9. Hogan, R. J., Matricardi, M. 2022, "A tool for generating fast
   k-distribution gas-optics models for weather and climate
   applications", *J. Adv. Model. Earth Syst.* 14, e2022MS003033,
   doi:10.1029/2022MS003033 (Reading repository copy read: its forcing
   evaluation and conclusions).
10. Höpfner, M., Milz, M., Buehler, S., Orphal, J., Stiller, G. 2012,
    "The natural greenhouse effect of atmospheric oxygen (O₂) and nitrogen
    (N₂)", *Geophys. Res. Lett.* 39, L10706, doi:10.1029/2012GL051409
    (author copy read, Table 1).
11. Karman, T. et al. 2019, "Update of the HITRAN collision-induced
    absorption section", *Icarus* 328, 160,
    doi:10.1016/j.icarus.2019.02.034 (not read; the files' own
    references were).
12. Lacis, A. A., Oinas, V. 1991, *J. Geophys. Res.* 96, 9027,
    doi:10.1029/90JD01945.
13. Pincus, R., Forster, P. M., Stevens, B. 2016, *Geosci. Model Dev.* 9,
    3447, doi:10.5194/gmd-9-3447-2016 (RFMIP profiles and RRTMGP fluxes,
    `earth-system-radiation/rrtmgp-data` at `dd983b15`).
14. Pincus, R. et al. 2020, "Benchmark calculations of radiative forcing
    by greenhouse gases", *J. Geophys. Res. Atmos.* 125, e2020JD033483,
    doi:10.1029/2020JD033483 (the text was not reached; its line-by-line
    fluxes were read in `RobertPincus/rfmip-benchmark-paper-figures` at
    `76f2bff4`, `lw-lbl-models.nc`, SHA-256 `11101c39…e094b`).
15. Chistikov, D. N., Finenko, A. A., Lokshtanov, S. E., Petrov, S. V.,
    Vigasin, A. A. 2019, *J. Chem. Phys.* 151 (19),
    doi:10.1063/1.5125756 (N₂–N₂ roto-translational; as HITRAN's
    `CIA_References_2024.pdf` gives it, not read).
16. Lafferty, W. J., Solodov, A. M., Weber, A., Olson, W. B., Hartmann,
    J.-M. 1996, *Appl. Opt.* 35, 5911, doi:10.1364/AO.35.005911 (N₂
    fundamental, 228-272 K; through HITRAN's references, not read).
17. Baranov, Y. I., Lafferty, W. J., Fraser, G. T. 2005, *J. Mol.
    Spectrosc.* 233, 160, doi:10.1016/j.jms.2005.06.008 (the N₂ and O₂
    fundamentals near room temperature; through HITRAN's references, not
    read).
18. Hartmann, J.-M., Boulet, C., Toon, G. C. 2017, *J. Geophys. Res.
    Atmos.* 122, 2419, doi:10.1002/2016JD025677 (N₂ near 2.16 µm; through
    HITRAN's references, not read).
19. Thibault, F. et al. 1997, *Appl. Opt.* 36, 563,
    doi:10.1364/ao.36.000563 (O₂–N₂, O₂'s fundamental; through HITRAN's
    references, not read).
20. Orlando, J. J. et al. 1991, *J. Geophys. Res.* 96, 20755,
    doi:10.1029/91JD02042 (O₂–N₂; through HITRAN's references, not read).
21. Borysow, A., Frommhold, L. 1987, *Astrophys. J.* 318, 940,
    doi:10.1086/165426 (CH₄–CH₄; through HITRAN's references, not read).
22. Fulchignoni, M. et al. 2005, "In situ measurements of the physical
    characteristics of Titan's environment", *Nature* 438, 785,
    doi:10.1038/nature04314 (Titan's surface, 93.65 ± 0.25 K and
    1467 hPa, as [What decides whether a world has an
    atmosphere?](atmospheres.md) reads it).

## See also

- [How does a planet's air set its outgoing heat and its
  albedo?](radiative-transfer.md): the column, its table and its
  comparison with CERES.
- The study behind this page, in the Wiki:
  [Study: the longwave's missing gases](https://github.com/cyboooooorg/map-generator/wiki/36-longwave-gases-audit).
