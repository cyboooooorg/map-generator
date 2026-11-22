# Relief study (W2): the sources of the relief

Date: 2026-11-22. Serves the
[world relief design](../../specs/2026-11-21-world-relief-design.md)
(sections "The study" and "Calibrations left") and Task 1 of its
[plan](../../plans/2026-11-22-world-relief.md).

## Purpose

The relief of `legacy-final` carries about sixty numbers without a source.
Three literature drafts (2026-11-21: plates and boundary relief,
volcanoes, statistical shape of topography) proposed sources for them,
several only second-hand. This study checks every value the design
retains on its source, records where it was read, recomputes what a
dataset can give, and leaves the rest as a short, declared list of
calibrations. The table of retained values below is the reference the
relief code copies into its doc comments (`Source:`, `Calibration:`,
`Definition:`).

## Method

- Each value was opened on its source: the paper's text (arXiv, an
  author or repository copy), its abstract on the publisher, PubMed,
  OpenAlex or a repository record, or the dataset itself. DOIs and
  page ranges were checked on Crossref.
- Tags: **[V]** read on the source, with its section, table, figure or
  page; **[V, abstract]** read on the abstract only; **[V, measured]**
  recomputed from the published dataset with a script of this study;
  **[S]** second-hand (a search snippet, a later paper, a textbook not
  opened); **[D]** derived by arithmetic from tagged values. A value left
  at [S] goes to the calibration list.
- Publisher PDFs that refused automated access (Wiley, Elsevier, USGS
  Publications Warehouse) were not worked around: open copies (arXiv,
  HAL, author pages, repositories) were used where they exist.
- Two values the drafts did not settle were measured on the datasets:
  the amplitude of the topography at a reference wavelength (ETOPO 2022,
  MOLA) and the continental slope angle (ETOPO 2022). The ETOPO
  hypsometry of the legacy tests, which had no script, was recomputed.
  The scripts, standard-library Python, are in [`scripts/`](scripts/);
  the data are fetched from NOAA's and the PDS's public servers (URLs in
  each script) and not stored in the repository.

## Findings, by theme

- [Topography](topography.md): spectrum (H), amplitude, coastlines,
  hypsometry targets, continental share, crusts, continental slope.
- [Plates](plates.md): count, sizes, speeds, slab pull, trenches, arcs,
  rifts, orogens, ridges.
- [Volcanoes](volcanoes.md): heat-flux anchors, intensity, provinces,
  cones, arc volcanoes.

## Corrections to the drafts

- Aharonson, Zuber & Rothman 2001: doi:10.1029/2000JE001403 (Crossref),
  not `2000JE001364`.
- Earth's mean heat flux 0.087 W/m² (44.2 TW) is Pollack, Hurter &
  Johnson 1993 [V, abstract]; Davies & Davies 2010 give 47 TW, about
  0.092 W/m².
- Allmendinger et al. 1997: doi:10.1146/annurev.earth.25.1.139 checked
  (Crossref, OpenAlex); the abstract carries the 350-400 km width.
- Grosse & Kervyn 2018: doi:10.1016/j.geomorph.2017.12.017; the drafts'
  `…12.028` is a paper on coastal boulders.
- Gill 1981 supports no number: dropped.
- England & Katz 2010 is a comment, *Nature* 468, E6-E7.
- Landais et al.: published in *Icarus* 319, 14-20 (2019).
- Slab dip, left open by the drafts: Lallemand, Heuret & Boutelier 2005,
  read on HAL [V].
- Earth's Haar fluctuation read off Landais et al.'s Fig. 2 is about 1.9
  times below its recomputation on ETOPO 2022, while Mars agrees within
  1 % ([topography](topography.md#amplitude-at-a-reference-wavelength)).

## Retained values

One row per constant of plan Tasks 2-6. Tag: `Source`, `Calibration` or
`Definition`, as the doc comment will start.

### Spectrum and scale (Task 2)

| Name | Value | Unit | Tag | Citation |
| --- | --- | --- | --- | --- |
| `hurst(Earth)` | 0.479 | — | Source | Landais, Schmidt & Lovejoy 2019, Table 2 (scales above 10 km) [V] |
| `hurst(Mars)` | 0.53 | — | Source | same [V] |
| `LACUNARITY` | 2 | — | Definition | sampling choice: persistence `2^-H` keeps the spectrum |
| `octaves` | circumference down to 2 cells | — | Definition | the grid's Nyquist limit |
| `REFERENCE_WAVELENGTH` | 100 | km | Definition | the lag at which the amplitude is measured, above the 10-20 km break |
| `amplitude(Earth)` | 219 | m | Source | ETOPO 2022, mean Haar fluctuation of north-south profiles at 100 km, whole surface [V, measured, `scripts/etopo_stats.py`] |
| `amplitude(Mars)` | 233 | m | Source | MOLA MEGDR, same statistic [V, measured, `scripts/mola_stats.py`]; Landais et al. Fig. 2 about 231 m |
| `HEIGHT_RANGE` | 25 | km | Definition | the storage range; Olympus Mons's relief "almost 22 km" (Plescia 2004) [V, abstract] |
| `STANDARD_GRAVITY` | 9.80665 | m/s² | Definition | CGPM 1901 |
| `MAX_RELIEF_SCALE` | 2 (legacy) or 2.49 | — | Calibration, or Source if 2.49 | Olympus Mons 22 km / Everest 8.849 km = 2.49 [D]; see the calibration list |

### Crust (Task 3)

| Name | Value | Unit | Tag | Citation |
| --- | --- | --- | --- | --- |
| `EARTH_CRUST.continental_thickness` | 41.1 | km | Source | Christensen & Mooney 1995 [V, abstract] |
| `EARTH_CRUST.oceanic_thickness` | 7.1 | km | Source | White, McKenzie & O'Nions 1992 [V, abstract] |
| `EARTH_CRUST.crust_density` | 2830 | kg/m³ | Source | Christensen & Mooney 1995 [V, abstract on the USGS record] |
| `EARTH_CRUST.mantle_density` | 3300 | kg/m³ | Calibration | textbook value [S]; PREM's lid about 3380 [S]; see the calibration list |
| `MARS_CRUST.continental_thickness` | 58 | km | Source | Neumann et al. 2004, southern modal thickness [V, abstract] |
| `MARS_CRUST.oceanic_thickness` | 32 | km | Source | Neumann et al. 2004, northern modal thickness [V, abstract] |
| `MARS_CRUST.crust_density` | 2900 | kg/m³ | Source | Wieczorek 2007, Fig. 11 caption [V] |
| `MARS_CRUST.mantle_density` | 3500 | kg/m³ | Source | Wieczorek 2007, Fig. 11 caption [V]; Neumann et al.'s 600 kg/m³ contrast [V, abstract] |
| `continental_share(Earth)` | 0.41 | — | Source | Cogley 1984 [V, abstract] |
| `continental_share(Mars)` | 0.58 | — | Source | 1 − 0.42, Marinova, Aharonson & Asphaug 2008 [V, abstract] |
| `CONTINENTAL_SLOPE` | 2.4 | ° | Source | ETOPO 2022, mean gradient 200-2500 m deep within 150 km of land [V, measured, `scripts/etopo_slope.py`]; 2.2-3.1° over the method's choices |

### Plates (Tasks 4 and 6)

| Name | Value | Unit | Tag | Citation |
| --- | --- | --- | --- | --- |
| `PLATE_COUNT` | (8, 14) | — | Source | Morra et al. 2013, 8 large plates [V]; Bird 2003, 14 large plates [V, abstract] |
| `PLATE_SIZE_EXPONENT` | 0.25 | — | Source | Sornette & Pisarenko 2003, µ = 0.25 ± 0.05 [V] |
| `PLATE_SPEED` | (2.6, 7.9) | cm/yr | Source | Zahirovic et al. 2015, median RMS speeds without subduction and above 25 % subducting [V] |
| `SLAB_FULL_SHARE` | 0.25 | — | Source | Zahirovic et al. 2015, class boundary [V] |
| cm/yr to m/s | 0.01 / 31 557 600 | — | Definition | Julian year |
| `ARC_SLAB_DEPTH` | 105 | km | Source | Syracuse & Abers 2006, global average (72-173) [V, abstract] |
| `SLAB_DIP` | 32 | ° | Source | Lallemand, Heuret & Boutelier 2005, mean shallow dip 0-125 km, 159 transects [V] |
| slab dip by upper plate (optional) | 42.5 oceanic, 27.5 continental | ° | Source | same, section 4.4 medians [V] |
| trench relief | 0.9-4.7 | km | Source | Zhang et al. 2018, W0 of the global array [V] |
| trench width | 31-93 | km | Source | Zhang et al. 2018, X0, axis to zero crossing [V] |
| value used inside each range | midpoint: 2.8 km, 62 km | — | Definition | the middle of a sourced range |
| rift width | under 100 | km | Source | Hopper & Buck 1996 [V]; Buck 2007 [V] |
| collision height | 5 | km | Source | Royden et al. 2008, Tibet [V] |
| Andean uplift | 3.65 over 350-400 | km | Source | Allmendinger et al. 1997, Altiplano-Puna [V] |
| `ridge_depth` | 2.5 + 0.35 √(t / Ma) | km | Source | Parsons & Sclater 1977, 0-70 Ma [V, abstract] |

### Volcanoes (Task 4)

| Name | Value | Unit | Tag | Citation |
| --- | --- | --- | --- | --- |
| Earth mean heat flux | 0.087 | W/m² | Source | Pollack, Hurter & Johnson 1993, 44.2 TW [V, abstract] |
| heat-pipe flux | 2.5 | W/m² | Source | Moore & Webb 2013, Io [V] |
| `CONE_ASPECT` | (0.01, 0.1) | — | Source | Grosse & Kervyn 2018, H/WB [V, abstract] |
| `CONE_HEIGHT_MAX` | 8.4 at standard gravity | km | Source | Plescia 2004's 22 km [V, abstract] × 3.73 / 9.80665 (NASA Mars fact sheet [V]; Weisskopf 1975) [D] |
| arc cone placement | random points in the arc band | — | Definition | de Bremond d'Ars et al. 1995: spacing "corresponds to randomly generated points" [V, abstract]; replaces `ARC_SPACING_SHAPE` |

### Test targets (Task 9)

| Target | Value | Citation |
| --- | --- | --- |
| Earth land share | 29.1 % | ETOPO 2022 [V, measured] |
| Land under 500 m, above 2 km, above 4 km | 56.0 %, 10.85 %, 1.77 % | ETOPO 2022 [V, measured] |
| Sea floor deeper than 1 km, 6 km | 88.4 %, 1.02 % | ETOPO 2022 [V, measured] |
| Mars highland share | 0.58 | Marinova et al. 2008 [V, abstract] |
| Venus within 1 km of the mean | more than 80 % | Ford & Pettengill 1992 [V, abstract] |
| H of the relief | 0.479, 0.53 | Landais et al. 2019 [V] |

## Choices of the design (*my call*), not calibrations

- One continental share per regime, constant (Earth's with plates,
  Mars's otherwise).
- Crusts not scaled with the planet's mass.
- Every surface liquid treated alike for the sea level.
- Venus, and every regime without plates, takes Mars's H and amplitude.
- The amplitude is the whole-surface fluctuation (land and sea floor
  together), as Landais et al. measure it; the land-only value (131 m on
  Earth) is the alternative if the relief comes out too rough on land.
- Arc cones as random points in the arc band, after de Bremond d'Ars et
  al., instead of a Gamma law with parameters.

## Final calibration list

Each will carry a `Calibration:` doc comment, a calibration section in its
`docs/science` page and a line in the pull request.

| # | Calibration | Value (legacy unless said) | Fitted on, or to fit on | Searched and not found |
| --- | --- | --- | --- | --- |
| 1 | Regime to analogue body | Plates → Earth; Stagnant, Episodic, HeatPipe → Mars (Venus too) | Task 9: H, Mars highland share, Venus 80 % within 1 km | a measured H for Venus; crust shares of non-plate regimes |
| 2 | Earth upper-mantle density | 3300 kg/m³ (Airy step 4.84 km) | ETOPO 2022 hypsometry (Task 9); PREM's 3380 would make it a source and the step 5.53 km | PREM table (IRIS refused), Turcotte & Schubert (not opened) |
| 3 | Volcanic intensity curve | shape between Earth (0.087 W/m²) and a heat pipe (2.5 W/m² → 1); Earth's value 0.5 | nothing published; checked at its anchors (Task 4) | a law from heat flux to volcanic share or cone density (Kite et al. 2009, Crisp 1984, Moore & Webb 2013, Kankanamge & Moore 2019) |
| 4 | `VOLCANIC_PROVINCE_SHARE` | 0.3 at intensity 1 | bounded by the Moon's 16-17 %, Mercury's about 18 %, Venus's 50-75 % [S] | Mars and Venus volcanic unit areas (Tanaka et al. 2014, Ivanov & Head 2011 tables) |
| 5 | Cone size draw | how sizes are drawn inside 0.1-1000 km³ and H/WB 0.01-0.1 | Grosse & Kervyn's ranges | a global size distribution of volcanoes |
| 6 | Isolated cone spacing and presence | one per 900 km cell, in half the cells | none | a global law (ten Brink 1991, Hahn & Byrne 2023 give local densities [S]) |
| 7 | Arc cone density | one per 250 km of arc (200 km cell, 80 %) | regional 50-70 km [S] | de Bremond d'Ars et al.'s Gamma mean (full text refused) |
| 8 | Island-arc uplift | 3 km at 120 km, sigma 60 km | none | arc edifice and forearc height statistics |
| 9 | Collision orogen width | sigma 350 km (FWHM 583 km) | bounded by the Altiplano (350-400 km) and Tibet (about 1000 km [S]) | a published orogen width |
| 10 | Rift shoulder and axis | +0.5 km at 120 km, −1.5 km at the axis | bounded by 0-3 km (Buck 2007) | typical shoulder uplift, axis depth statistics |
| 11 | `RELIEF_SPEED`, `ARC_SPEED` | 0.5-5 cm/yr, 2 cm/yr | none | a convergence threshold for trenches, orogens, arcs (Jarrard 1986, Heuret & Lallemand 2005) |
| 12 | `PLATE_WEIGHT` | 0.35 | to fit: Sornette & Pisarenko's µ = 0.25 and PB2002's areas | a source for the spread device itself (none expected) |
| 13 | Slab-pull ramp | speed rise between 0 and 25 % subducting | Zahirovic et al.'s 4.4 cm/yr under 25 % as check point | a continuous law of speed against subducting share |
| 14 | `MAX_RELIEF_SCALE` | 2 | none; the sourced ratio Olympus/Everest is 2.49, which would make it a source | — |
| 15 | `HIGHLANDS` (ported) | frequency 1.6, 3 octaves, edge (0.23, 0.33) | about 9 % of the sphere, Venus's highland share (Ivanov & Head 2011) | already documented in legacy |
| 16 | `DRY_PLAINS_KM` (ported) | 3 km per noise unit at Earth's gravity | Venus: more than 80 % within 1 km (79 % on `venus-like`) | already documented in legacy |
| 17 | `MOUNTAIN_ONSET`, `MOUNTAIN_BLEND` (only if Task 9 needs them) | 0.28, 0.45 | ETOPO 2022 hypsometry | mountain range widths, ridge spacing |

Left the list since the spec: the Andean uplift (now Allmendinger et
al.), the arc position (slab depth and dip), `STAGE_MA` (the drift is not
ported), the Gamma shape of arc spacing (random placement). Joined it:
numbers 2, 5 and 14, unsourced numbers of the legacy that the drafts had
not listed.

## Open items

- The three drafts and this study are to become one Wiki study, which
  the pages will then cite; until then the pages cite this folder.
- A primary read of PREM would turn calibration 2 into a source and move
  the Earth step from 4.84 to 5.53 km: the user's decision, with Task 9's
  hypsometry.
- Earth's Haar fluctuation disagrees with Landais et al.'s figure by a
  factor 1.9; a note to the authors' data, or a reading of the published
  Icarus version, could settle it.
