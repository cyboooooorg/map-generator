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

### Methane's band under N₂O's

N₂O's ν₁ band, near 1285 cm⁻¹, lies under methane's ν₄ band (1306
cm⁻¹), so in an air that holds both each takes less than alone. Two
published fits give the size of the overlap for methane, both read for
this page:

- **Etminan et al. 2016** (Table 1, p. 12620): methane's forcing is
  $[a_3 \bar M + b_3 \bar N + 0.043](\sqrt M - \sqrt{M_0})$ W/m², with
  $a_3 = -1.3 \times 10^{-6}$ and the N₂O term
  $b_3 = -8.2 \times 10^{-6}$ W m⁻² ppb⁻¹, $\bar M$ and $\bar N$ the
  mean of the initial and final amounts in ppb; valid for 200 to 525 ppb
  of N₂O and 340 to 3500 ppb of CH₄. Their own reading (p. 12620,
  Fig. 2d): between 200 and 525 ppb of N₂O, "for CH4 the effect of N2O
  overlap is at less than the 10% level". The fit is of all-sky
  forcings with the stratosphere adjusted and methane's shortwave
  included (§3.2, p. 12616: the shortwave raises methane's forcing by
  15 %).
- **IPCC TAR** (Ramaswamy et al. 2001, Table 6.2, p. 358), the
  expressions of Myhre et al. 1998 with the overlap function Etminan et
  al. trace to Hansen et al. 1988 (p. 12619):
  $\Delta F = 0.036(\sqrt M - \sqrt{M_0}) - [f(M, N_0) - f(M_0, N_0)]$,
  $f(M, N) = 0.47 \ln[1 + 2.01 \times 10^{-5} (MN)^{0.75} + 5.31 \times
  10^{-15} M (MN)^{1.52}]$.

Applied to the cases of the tests (derived here):

| Case | Etminan et al. with N₂O | at 200 ppb | at none (outside the fit) | TAR with N₂O | TAR without | N₂O's share, TAR |
| --- | --- | --- | --- | --- | --- | --- |
| RFMIP's step, 808 → 1831 ppb, 327 ppb of N₂O | 0.5545 | 0.5695 (+2.7 %) | 0.5930 (+6.9 %) | 0.4454 | 0.5171 | 13.9 % |
| Earth's removal, 1803 → 0 ppb, 324 ppb of N₂O | 1.6633 | 1.7065 | 1.7761 | 1.3604 | 1.5286 | 11.0 % |

So N₂O takes 6 to 14 % of methane's step in the fits (Etminan et
al.'s linear term carried to no N₂O, the TAR's function). The column,
with N₂O's HITRAN lines added in a scratch table (2026-11-27, the same
tool and bins), finds 16 %: RFMIP's step falls from 0.7479 to
0.6262 W/m² with RFMIP's 326.988 ppb of N₂O, Earth's removal of 1803
ppb from 2.436 to 1.977 W/m² with AR6's 324.2 ppb (19 %). The column's
overlap sits just above the fits' range; the fits are for a different
quantity (adjusted, all-sky, with the shortwave), and the column's
N₂O step on RFMIP is 3.8 % over the line-by-line codes ("In the app").
That missing overlap was most of methane's 22 % excess: with N₂O,
both of methane's tests fall inside their windows, and N₂O's lines
joined the table (decided by the user on 2026-11-27, "Decisions").

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
broadened by itself 6.72 MB; with ozone's set too, as built by #704,
7.68 MB. HITRAN's ids are 32-35 for CH₄ and 21-25 for N₂O (HITRAN
isotopologue table).

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

- **Absorbers.** The infrared column holds the lines of water, CO₂,
  methane, ozone and N₂O (`opacity.bin`'s eight sets: CO₂, water and
  methane each broadened by air and by itself, ozone and N₂O by air;
  methane, ozone and N₂O since study 36's second pull request, issue
  #704), MT_CKD 4.3's water
  continuum (self, and foreign with every other gas of the air,
  `spectrum.rs`), MT_CKD's CO₂ continuum, and HITRAN's collision-induced
  pairs (`spectrum::PAIRS`): CO₂–CO₂ (`co2-cia.tsv`), and N₂–N₂, O₂–O₂,
  O₂–N₂, N₂–CH₄ and CH₄–CH₄ (`pairs-cia.tsv`, study 36's first pull
  request, issue #703). Each gas with lines keeps its eight g-points, and
  the gases multiply their transmissions (random overlap,
  `spectrum::LineGas`). NH₃ and SO₂ are in the air but have no infrared
  absorption. N₂O is not a gas of the model's airs until its law
  (#737): its lines are in the table, and only the tests give a column
  N₂O (`Column::with_n2o`, a mole fraction of the dry air at every
  level), RFMIP's and AR6's, to compare like for like with the
  line-by-line codes, whose air holds it.
- **Methane's, ozone's and N₂O's lines.** Their HITRAN2024 lines (ids
  32-35, 16-20 and 21-25, downloaded 2026-11-27) are summed like
  water's, cut at
  25 cm⁻¹ less LBLRTM's flat pedestal (`CONVF4`, "all molecules other
  than co2"), with no continuum (`crates/science/data/OPACITY.md`). The
  table's mean absorption of methane's ν₄ band, 1240-1360 cm⁻¹, equals
  HITRAN's summed intensities within 1.3 % at 300 K and 0.1 % at 200 to
  250 K (checked on the built table). Ozone's column
  (`outgoing_longwave`'s `ozone`, `ozone_column` of the air's O₂ and
  its stars) is laid out on Earth's AFGL shape against the pressure
  ([ozone.md](ozone.md), calibration 5).
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
  (issue #720), the haze's infrared, and the
  methane of the lower troposphere: the column applies the cold trap's
  mixing ratio (`radiating_methane_bar`) at every level, so it holds
  about 40 % of the sourced surface CH₄ there (a review's estimate,
  derived, issue #744), likely a large share of the gap since N₂–CH₄ is
  linear in CH₄. Each of them can only warm it.
- **Which airs hold methane.** Methane is outgassed and survives only
  under an equilibrium temperature of 150 K (`METHANE_SURVIVES`), capped
  by its saturation: Titan's chain air keeps 0.091 bar. The `Modern`
  biosphere's law (`science::life_gases::modern_methane`, study 37,
  [life-gases.md](life-gases.md)) is in `science`, called by no engine
  until planet PR 2d-4; N₂O is not a gas of the model.
- **What the second pull request measured** (the branch
  `feat/climate-methane-ozone-lines`, before #741's N₂ pseudo-adiabat;
  Earth's clear column of the CERES test: measured air, 288 K, 280 ppm,
  1803 ppb of methane, 324.2 ppb of N₂O, 300 DU; RFMIP's 100 sites
  radiated one by one with RFMIP's N₂O, their ground black at the
  surface temperature). Without N₂O in the airs, the first measurements
  are in the second column:

| Case | Column without N₂O | Column with N₂O | Reference | Verdict |
| --- | --- | --- | --- | --- |
| Ozone's 300 DU taken out, Earth clear | 8.371 W/m² | 8.366 W/m² | 4.185 to 10 (Schmidt et al. 2010; Kiehl & Trenberth 1997) | inside |
| Methane's 1803 ppb taken out, Earth clear | 2.436 W/m² | 1.977 W/m² | 1.5 (Hogan & Matricardi 2020, 1921 ppb) to 1.87 (Höpfner et al. 2012), ± 8.2 %: 1.377 to 2.023 | inside (20 % over without N₂O) |
| RFMIP, methane's step "PI CH4" − "PD" | 0.7479 W/m² | 0.6262 W/m² (OLR 264.530 − 263.903) | 0.613 ± 0.050 (line by line; RRTMGP 0.563) | +2.2 %, inside (22 % over without N₂O) |
| RFMIP, N₂O's step "PI N2O" − "PD" | | 0.2117 W/m² (OLR 264.115 − 263.903) | 0.204 ± 0.024 (line by line; RRTMGP 0.180) | +3.8 %, inside |
| N₂O's 324.2 ppb taken out, Earth clear (a probe, no test) | | 2.179 W/m² | 1.5 (Hogan & Matricardi 2020, 332 ppb); 1.88 (Höpfner et al. 2012) | 16 % over the larger |
| RFMIP, CO₂'s step "PI CO2" − "PD" (a probe, without N₂O) | 1.379 W/m² | | LBLRTM 12.8 1.322 (Pincus et al. 2020's fluxes) | 4.3 % over |
| Earth's clear sky against CERES like for like (printed since study 40) | 263.41 W/m² (window 266.14 to 274.58) | 261.24 W/m² | 264.64 to 273.08 (N₂O's 1.5 W/m² out of `MINOR_GASES_W_M2`) | 3.40 under |
| RFMIP's present-day clear sky, the column on the 100 profiles (tier 1 since study 40) | | 263.903 W/m² | LBLRTM 263.229 with the halocarbons, 263.619 without (0.39 W/m²), within 0.4 % (1.053 W/m², Pincus et al. 2019) | +0.284, inside |
| Titan's sourced column | 92.300 K | 92.300 K (91.813 K since N₂ condenses, #741) | 92.296 K before the lines (91.809 K on `main` with #741); measured 93.65 ± 0.25 | +0.004 K |

The methane step hardly depends on the bins (0.710 on 20 cm⁻¹, 0.720
on 10 cm⁻¹, Earth's single column, without N₂O) and the table holds
HITRAN's band strengths; the excess was N₂O's overlap ("Methane's band
under N₂O's", above). With N₂O, both of the gases' RFMIP steps are
within 4 % of the line-by-line codes, while on Earth's clear column
methane's removal sits near the top of its window and N₂O's is 16 % over
Höpfner et al.'s: Earth's test column takes more than the sources' from
both, which points at the column (its upper air, its stratosphere at
the skin temperature) more than at the lines. Ozone's 8.4 W/m² sits
under that stratosphere (on RFMIP's own temperatures it takes
7.46 W/m²). The CERES test's clear sky is 3.4 W/m² under its window,
while on RFMIP's 100 profiles the column is 0.28 W/m² from LBLRTM:
the single column's profile, not the lines, is dark. Study 40 found the
stratosphere's heat is not the cause; the CERES test now only prints
the gap, the RFMIP field holds the radiation, and the single column's
upper atmosphere goes to track T11 (issue #757; see "Where it departs"
in [radiative-transfer.md](radiative-transfer.md)).

The clouds' infrared emissivity was then refitted on CERES (#705,
`CLOUD_EMISSIVITY` 0.848, [radiative-transfer.md](radiative-transfer.md)),
and the bodies measured after #741 (`main` b6845f95 against the branch):
the reference Earth holds 288 K with 6.910e-3 bar of CO₂ (4.071e-3);
Earth's measured air settles at 289.39 K at 280 ppm (288.18 K; 290.95 K
with the lines before the refit, out of its 286-290 K window); Mars
214.131 K and the Triton-like body 37.975 K do not move; the reference
Titan 92.613 → 92.616 K; the methane Earth's outer edge 1.2534 →
1.2720 AU and its cycling orbit 1.264 → 1.282 AU; N₂'s frost and liquid
branches (up to 0.0076953 and down to 0.0064470 Earth fluxes, one jump
0.006565 → 0.006581) do not move; Earth's snowball chance 0.01400 →
0.01408 under 9.80 bar, refitted to 9.85 bar (0.01401).

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

- **Earth's methane waits for planet PR 2d-4.** The column has
  methane's lines, and study 37's law of a `Modern` biosphere's methane
  is in `science` (`life_gases::modern_methane`), but no engine calls it
  yet, so the reference Earth's air holds none. N₂O has its lines in the
  table since #704 but needs a gas of its own and its law (#737); until
  then only the tests' columns hold it.
- **Weights read as moles (issue #749).** The chain's partial pressures
  are weights; the column reads `p_i / p` as mole shares, so it would see
  the law's 673 ppb of methane as 373 ppb (16.04 / 28.96 of it; argon
  likewise, radiative-transfer.md). Not fixed here; the tests build
  their airs from mole fractions, so they are not affected.
- **Titan's grey stand-in is gone.** Spectral N₂-N₂ and N₂-CH₄ pairs
  reach 92.3 K without a fit; methane's lines (#704) add 0.004 K, their
  bands lying far on the Wien side of a 93 K Planck curve. What is still
  missing is N₂-H₂ (the model's air has no H₂, issue #720), the haze's
  infrared,
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
  *Changed on 2026-11-27* (below): N₂O's lines join the same rebuild.
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

## Decisions (user, 2026-11-27)

- **Methane's excess is studied first, with an N₂O probe.** The fits'
  overlap is read (Etminan et al. 2016; the TAR's Table 6.2), and a
  scratch table with N₂O's lines measures it ("Methane's band under
  N₂O's"). N₂O closed the gap to within the sourced windows, so **N₂O's
  lines go into this one rebuild of `opacity.bin`**; N₂O's law stays
  for #737, no air holds N₂O until then, and only the tests carry
  RFMIP's and AR6's N₂O. No window is widened.
- **Then the CERES test and Earth's measured air are measured again**,
  and `CLOUD_ALBEDO` and the clouds' longwave part of #705 are refitted
  by their own methods in the same pull request.

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
   12614, doi:10.1002/2016GL071930 (read in the University of Reading's
   CentAUR copy, CC BY 4.0: §3.2-4, Table 1 and Fig. 2, pp. 12616-12620).
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
23. Myhre, G., Highwood, E. J., Shine, K. P., Stordal, F. 1998, "New
    estimates of radiative forcing due to well mixed greenhouse gases",
    *Geophys. Res. Lett.* 25, 2715, doi:10.1029/98GL01908 (not read, not
    open access; its expressions are read as source 25 gives them).
24. Gulev, S. K. et al. 2021, "Changing state of the climate system", in
    *Climate Change 2021: The Physical Science Basis*, IPCC AR6 WG1,
    chapter 2, doi:10.1017/9781009157896.004 (Table 2.2: methane's global
    mean of 2011, 1803.1 ppb by NOAA, 1803.6 by AGAGE, the CERES test's
    1803 ppb; N₂O's, 324.2 ppb by NOAA, 324.7 by AGAGE, 324.0 by CSIRO,
    the CERES test's 324.2 ppb; read, p. 302).
25. Ramaswamy, V. et al. 2001, "Radiative forcing of climate change", in
    *Climate Change 2001: The Scientific Basis*, IPCC TAR WG1, chapter 6
    (read: Table 6.2, p. 358, the simplified expressions of methane and
    N₂O with their overlap function $f(M, N)$, and §6.3.5, p. 359).
26. Pincus, R., Mlawer, E. J., Delamere, J. S. 2019, "Balancing accuracy,
    efficiency, and flexibility in radiation calculations for dynamical
    models", *J. Adv. Model. Earth Syst.* 11, 3074,
    doi:10.1029/2019MS001621 (read: §3.2 and Table 1, RTE+RRTMGP within
    0.4 % of LBLRTM at the top of RFMIP's profiles; LBLRTM's present-day
    outgoing longwave 263.197 W/m²).

## See also

- [How does a planet's air set its outgoing heat and its
  albedo?](radiative-transfer.md): the column, its table and its
  comparison with CERES.
- The study behind this page, in the Wiki:
  [Study: the longwave's missing gases](https://github.com/cyboooooorg/map-generator/wiki/36-longwave-gases-audit).
