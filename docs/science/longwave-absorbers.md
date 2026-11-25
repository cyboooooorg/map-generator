# Which gases does the infrared column still miss?

Seen from: the mean surface temperature of every body whose air holds
nitrogen, oxygen, methane or nitrous oxide, and the cause trees that
explain it. Study 36 (issue #703) wrote this page before any code; the
article on the column itself is
[How does a planet's air set its outgoing heat and its
albedo?](radiative-transfer.md).

## Short answer

The app's infrared column holds the lines of water vapour and carbon
dioxide, their continua, the CO₂ pairs and a grey stand-in for Titan's
methane. Earth's real air also traps heat with ozone, methane, nitrous
oxide (N₂O), halocarbons, and the brief pairs that colliding nitrogen and
oxygen molecules form. Line-by-line codes put the last five at about
3.6 W/m² of Earth's clear-sky outgoing infrared at present-day amounts,
and methane and N₂O at 2 to 3 W/m² at pre-industrial ones. Two findings
matter more than the numbers:

- The model's Earth carries **no methane and no N₂O** at all: the chain
  has no law that makes them. Adding their lines to the column changes
  nothing for the reference Earth until a law of life's methane and N₂O
  exists. Only the nitrogen and oxygen pairs act at once: about +0.1 to
  +0.2 K.
- The **nitrogen and methane pairs** of HITRAN's collision-induced files
  put a spectral Titan at 92.30 K without any fitted constant (measured:
  93.65 K), where the column now needs a calibrated grey opacity. On
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
  experiment "PI CH4" or "PI N2O" less "PD" (derived here). The CO₂
  parameterization RRTMGP gives 0.563 and 0.180 on the same profiles.
- **Collins et al. 2006** (RTMIP, Table 8, case 3a-1a): adding 806 ppb
  of CH₄ and 275 ppb of N₂O to an air that had none, on the AFGL
  mid-latitude summer profile (294 K surface), lowers the outgoing
  infrared at the top by 3.63 W/m² (line-by-line mean, s.d. 0.05). This
  is the only published joint value from zero at pre-industrial amounts
  found; the profile is warm and moist, so a global mean is smaller.

**At pre-industrial amounts** (derived, the sources do not give it): the
removal at present day less the RFMIP rise, and less the rise from
RFMIP's present day to Hogan & Matricardi's (Etminan et al. 2016,
0.04 W/m² for CH₄ and 0.02 for N₂O), gives about 0.85 W/m² for CH₄ and
1.3 for N₂O on Hogan & Matricardi's profiles, 1.26 and 1.68 from
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
  the air, `spectrum.rs`), MT_CKD's CO₂ continuum, CO₂-CO₂ pairs and the
  grey methane stand-in (`methane.rs`, `CH4_OPACITY` 15.1 and
  `CH4_SATURATION_BAR` 0.01 bar, both calibrations). NH₃ and SO₂ are in
  the air but have no infrared absorption.
- **Which airs hold methane.** Methane is outgassed and survives only
  under an equilibrium temperature of 150 K (`METHANE_SURVIVES`), capped
  by its saturation: Titan's chain air keeps 0.091 bar. No law makes
  biogenic methane, and N₂O is not a gas of the model (the legacy
  removed methane from an oxygenated air and added only O₂).
- **What the study measured** on the column of `main` (1e122416), with
  HITRAN's pair files binned like `co2-cia.tsv` in a scratch probe
  (removed before the commit):

| Case | Without | With the pairs | Change |
| --- | --- | --- | --- |
| Earth's air (N₂ 0.7808, O₂ 0.2095 of 1.014 bar), 288 K, 280 ppm, clear | 274.718 | 274.312 | −0.406 W/m² (N₂ band 0.161 + N₂ fundamental 0.111 + O₂ fundamental 0.134) |
| Same, all-sky | 241.813 | 241.476 | −0.337 W/m² |
| Same at 285.82 K, all-sky | 236.885 | 236.532 | −0.353 W/m² (N₂-N₂ alone −0.205) |
| N₂ and O₂ only, no water or CO₂, 288 K | 390.105 | 383.731 | −6.37 W/m² (Höpfner et al.: 2.8 + 2.8 on their profile) |
| 2, 5, 10 bar of N₂, Earth's water, 288 K | 263.03, 249.97, 240.95 | | −1.12, −3.30, −6.44 W/m² |
| Titan's sourced column (1.467 bar, 5.65 % CH₄), settled | grey stand-in: 93.60 K; nothing: 74.88 K | N₂-N₂: 83.95 K; + N₂-CH₄: 92.23 K; + CH₄-CH₄: 92.30 K | measured 93.65 ± 0.25 K |

The column's N₂ pairs take 0.27 W/m² on Earth where the line-by-line
sources give 0.15 to 0.17; the excess sits in the roto-translational
band below 650 cm⁻¹, which water hides. The cause is not isolated: the
column's dry upper air (study 32), HITRAN's newer N₂-N₂ far wing, or the
missing N₂-O₂ roto-translational band, which HITRAN does not give.

## Where it departs, and what is next

- **Earth's methane and N₂O need a law first.** A column with their
  lines changes the reference Earth only once its air carries them. The
  pre-industrial amounts are measured (ice cores, through the scenarios
  above); a law that makes them for any inhabited planet is not, and is
  not written here.
- **Titan's grey stand-in can go.** Spectral N₂-N₂ and N₂-CH₄ pairs
  reach 92.3 K without a fit. What is still missing is N₂-H₂ (the model's
  air has no H₂), methane's own lines, and the haze's infrared.
- **The temperature grid** of `opacity.bin` starts at 100 K; Titan's
  column runs from 69 to 94 K and takes 100 K lines.

## What we do not know

- A global-mean, line-by-line clear-sky removal of methane and N₂O at
  pre-industrial amounts: only the summer profile of Collins et al. 2006
  and derived values exist.
- Why the column's N₂ pairs take about 0.1 W/m² more than the
  line-by-line sources on Earth.
- How much of Titan's last 1.35 K belongs to N₂-H₂, to methane's lines
  or to the haze.

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
   <https://hitran.org/cia/>, downloaded 2026-11-25, with
   `CIA_References_2024.pdf` (read).
7. HITRAN isotopologue metadata, <https://hitran.org/docs/iso-meta/>
   (read 2026-11-25).
8. Hogan, R. J., Matricardi, M. 2020, "Evaluating and improving the
   treatment of gases in radiation schemes: the Correlated K-Distribution
   Model Intercomparison Project (CKDMIP)", *Geosci. Model Dev.* 13, 6501,
   doi:10.5194/gmd-13-6501-2020 (read; Tables 2 and 4 in their
   spreadsheets).
9. Höpfner, M., Milz, M., Buehler, S., Orphal, J., Stiller, G. 2012, "The
   natural greenhouse effect of atmospheric oxygen (O₂) and nitrogen
   (N₂)", *Geophys. Res. Lett.* 39, L10706, doi:10.1029/2012GL051409
   (author copy read, Table 1).
10. Karman, T. et al. 2019, "Update of the HITRAN collision-induced
    absorption section", *Icarus* 328, 160,
    doi:10.1016/j.icarus.2019.02.034 (not read; the files' own
    references were).
11. Lacis, A. A., Oinas, V. 1991, *J. Geophys. Res.* 96, 9027,
    doi:10.1029/90JD01945.
12. Pincus, R., Forster, P. M., Stevens, B. 2016, *Geosci. Model Dev.* 9,
    3447, doi:10.5194/gmd-9-3447-2016 (RFMIP profiles and RRTMGP fluxes,
    `earth-system-radiation/rrtmgp-data` at `dd983b15`).
13. Pincus, R. et al. 2020, "Benchmark calculations of radiative forcing
    by greenhouse gases", *J. Geophys. Res. Atmos.* 125, e2020JD033483,
    doi:10.1029/2020JD033483 (the text was not reached; its line-by-line
    fluxes were read in `RobertPincus/rfmip-benchmark-paper-figures` at
    `76f2bff4`, `lw-lbl-models.nc`, SHA-256 `11101c39…e094b`).

## See also

- [How does a planet's air set its outgoing heat and its
  albedo?](radiative-transfer.md): the column, its table and its
  comparison with CERES.
- The study behind this page, in the Wiki:
  [Study: the longwave's missing gases](https://github.com/cyboooooorg/map-generator/wiki/36-longwave-gases-audit).
