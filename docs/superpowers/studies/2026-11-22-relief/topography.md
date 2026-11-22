# Relief study: the statistical shape of the topography

Part of the [relief study](README.md). Tags: **[V]** read on the source
(page, table or figure given), **[V, abstract]** read on the publisher's
or an indexer's abstract of the source, **[V, measured]** recomputed from
the published dataset with a script of this study, **[S]** second-hand
(a later paper, a search snippet, a textbook not opened), **[D]** derived
by arithmetic from tagged values.

## Spectrum: the Hurst exponent

Landais, Schmidt & Lovejoy measure the mean Haar fluctuation of
north-south profiles of four bodies (Earth from ETOPO1 at 60 pixels per
degree, Mars from MOLA at 128, the Moon from LOLA at 512, Mercury from
MLA at 64; their Table 1) and fit two scaling regimes either side of a
break near 10-20 km [V, arXiv:1805.11249v1, section 5 and Table 2, p. 10]:

| Body | H above about 10 km | H below about 10 km | C1 above | alpha above |
| --- | --- | --- | --- | --- |
| Earth | 0.479 ± 0.001 | 0.823 ± 0.004 | 0.093 ± 0.002 | 1.70 ± 0.08 |
| Mars | 0.53 ± 0.001 | 0.773 ± 0.003 | 0.110 ± 0.002 | 1.80 ± 0.06 |
| Moon | 0.226 ± 0.002 | 0.878 ± 0.002 | 0.03 ± 0.01 | 1.4 ± 0.1 |
| Mercury | 0.248 ± 0.002 | 0.922 ± 0.003 | 0.059 ± 0.002 | 1.85 ± 0.1 |

The published version is Icarus 319, 14-20 (2019),
doi:10.1016/j.icarus.2018.07.005 (Crossref checked); its Table 2 was not
compared with the preprint's. The break lies below the shortest
wavelength of a 15 km grid (30 km), so the relief needs only the regime
above it: one power law.

Cross-checks:

- My recomputation gives log-log slopes of 0.54 on ETOPO 2022 (every 6th
  cell, 0.1°, 44 to 2002 km) and 0.56 on MOLA (0.25°, 59 to 2011 km)
  [V, measured, `scripts/etopo_stats.py`, `scripts/mola_stats.py`]; the
  points of Landais's Fig. 2 give 0.49 (Earth, 30 to 955 km) and 0.52
  (Mars, 19 to 611 km) [D]. Two-point slopes over a wider range, within
  0.06 of Table 2: the fit of Table 2 is retained.
- Aharonson, Zuber & Rothman 2001, doi:10.1029/2000JE001403 (the DOI
  checked on Crossref; the drafts' `2000JE001364` was wrong): Mars's 1-D
  spectrum, highlands exponent about 2.1 at long wavelengths, lowlands
  about 1.4 [V, author manuscript, Fig. 5]; for a self-affine profile the
  1-D spectral exponent is 2H + 1, so H about 0.55 and 0.2 [D].
- Balmino 1993 (Earth degree variance slope about −2) [S, quoted by
  Ermakov et al. 2018; not opened].

**Venus.** No measured H was found: Landais et al. leave Venus out (their
section 3), Turcotte 1987 calls it Brownian with a lower amplitude [S,
abstract only], Wieczorek's chapter shows a change of slope near degree
100 without an exponent [V, Treatise on Geophysics 10.05]. The design
gives Venus Mars's H *(my call)*.

**Lacunarity.** A sampling choice: with persistence `lacunarity^-H` any
lacunarity gives the same spectrum. 2 is a Definition.

## Amplitude at a reference wavelength

The slope of the spectrum does not set its amplitude. The design scales
the noise by the fluctuation of the relief at one reference wavelength.

Landais et al. Fig. 2 plots the mean Haar fluctuation in metres against
the scale. Read off the figure (log axes, about ±10 %) at 100 km
[V, Fig. 2, p. 17, read by pixel position]:

| Body | Fluctuation at 100 km | At 1000 km |
| --- | --- | --- |
| Earth | about 115 m | about 385 m |
| Mars | about 231 m | about 810 m |
| Moon | about 380-400 m | |
| Mercury | about 390 m | |

I recomputed the same statistic (mean of `|M2 − M1|`, the two half-window
means of a north-south profile) on ETOPO 2022 (surface, ice included,
every 6th cell) and on the MOLA MEGDR at 4 pixels per degree
[V, measured, `scripts/etopo_stats.py`, `scripts/etopo_haar_split.py`,
`scripts/mola_stats.py`, `scripts/mola_haar_split.py`]:

| Lag | Earth, all | Earth, land windows | Earth, sea-floor windows | Mars, all | Mars, highland windows | Mars, lowland windows |
| --- | --- | --- | --- | --- | --- | --- |
| 89 km | 207 m | 124 m | 236 m | 220 m | 254 m | 80 m |
| 111-118 km | 230 m (111) | 138 m (111) | 255 m (111) | 254 m (118) | 287 m (118) | 94 m (118) |
| 200-207 km | 316 m | 186 m | 317 m | 337 m | 361 m | 126 m |
| 1000 km | 845 m | 443 m | 549 m | 808 m | 648 m | 278 m |

Interpolated log-log to 100 km [D]: Earth 219 m (land 131 m, sea floor
246 m), Mars 233 m.

- **Mars agrees** with Landais's figure within 1 % at 100 and 1000 km.
- **Earth does not**: the figure's curve is about 1.9 times below my
  whole-surface values, close to my land-only windows. The figure gives no
  table, and the paper's text does not say how the Earth profiles were
  taken beyond "ETOPO1". The recomputation is reproducible; the figure
  reading is not. Retained: the recomputed whole-surface values, with the
  discrepancy stated.
- The split shows where the roughness lies: Earth's sea floor (ridges,
  trenches, seamounts) is rougher than its land at 100 km; Mars's
  highlands are three times as rough as its lowlands, as Aharonson et al.
  found (lowland slopes typically under 0.5°, abstract [V, abstract]).

The retained amplitude is a Haar fluctuation, not a standard deviation:
the noise is scaled by measuring its own Haar fluctuation at the
reference lag, never by equating the two.

## Coastlines

The level set of a self-affine surface of Hurst exponent H has a fractal
dimension 2 − H (Mandelbrot 1983; Turcotte 1997) [S, textbooks not
opened]. The legacy coast warp (`COAST_WARP_DECAY` 0.8) gives a
displacement amplitude growing as the wavelength to the power 1.32, so no
dimension above 1 [D]. With the measured H the warp is not needed; the
design drops it. Mandelbrot 1967's 1.25 for Britain [S] is the only
published coast dimension met; none is retained as a number.

## Hypsometry targets (Earth)

ETOPO 2022 (60 arc-second surface model, ice surface;
doi:10.25921/fd45-gt74), every 6th cell, area-weighted by the cosine of
latitude [V, measured, `scripts/etopo_stats.py`]:

| Statistic | Recomputed | Legacy comment (`generation.rs:3202`) |
| --- | --- | --- |
| Land share (above 0 m) | 29.1 % | 0.29 |
| Median land height | 418 m | 0.42 km |
| Land under 500 m | 56.0 % | 56 % |
| Land above 1 km | 25.8 % | |
| Land above 2 km | 10.85 % | 10.8 % |
| Land above 3 km | 5.06 % | |
| Land above 4 km | 1.77 % | 1.8 % |
| Sea floor deeper than 200 m | 92.7 % | |
| Sea floor deeper than 1 km | 88.4 % | (test: over 70 %) |
| Sea floor deeper than 3 km | 75.4 % | |
| Sea floor deeper than 4 km | 53.2 % | |
| Sea floor deeper than 6 km | 1.02 % | 1 % |
| Area mean height | −2392 m | |

The legacy figures had no script; they now do, and they hold. They are
test targets (Task 9), never a curve imposed on the relief.

## Hypsometry targets (Mars, Venus)

- **Mars's highland share.** The northern lowlands "cover approximately
  42 % of the surface" (Marinova, Aharonson & Asphaug 2008, Nature 453,
  1216-1219) [V, abstract, PubMed 18580945]: highlands 0.58 [D].
- **Mars bimodal**, unimodal about the centre of figure (Aharonson et al.
  2001) [V, abstract]. Mars's area mean on the MOLA areoid: −551 m
  [V, measured].
- **Venus unimodal**: "more than 80 % of the surface lying within 1 km of
  the 6051.84-km mean radius" (Ford & Pettengill 1992, JGR 97,
  13103-13114) [V, abstract].

## Continental share (Earth)

Continental crust covers 210.4 million km², "41 % of the earth's
surface", 30.6 % of it submerged; mean thickness 36 km (Cogley 1984,
Rev. Geophys. 22, 101-122) [V, abstract].

## Crusts (Airy isostasy)

| Quantity | Value | Source | Tag |
| --- | --- | --- | --- |
| Earth continental crust | 41.1 km | Christensen & Mooney 1995, area-weighted mean | [V, abstract] |
| Earth continental crust density | 2830 kg/m³ | Christensen & Mooney 1995, "mean crustal density calculated from our model" | [V, abstract on the USGS publication page] |
| Earth oceanic crust | 7.1 ± 0.8 km | White, McKenzie & O'Nions 1992 | [V, abstract] |
| Earth upper mantle density | 3300 kg/m³ | the textbook isostasy value (Turcotte & Schubert) | [S]; PREM's lid is about 3380 kg/m³ [S, not opened] |
| Mars modal crust, south / north | 58 / 32 km | Neumann et al. 2004 | [V, abstract] |
| Mars mantle − crust density | 600 kg/m³ | Neumann et al. 2004 | [V, abstract] |
| Mars crust / mantle density | 2900 / 3500 kg/m³ | Wieczorek 2007, Treatise on Geophysics 10.05, Fig. 11 caption (model parameters for Mars) | [V] |

The Airy step `(t − t_o)(1 − ρc/ρm)` gives 4.84 km for Earth with
3300 kg/m³, 5.53 km with PREM's 3380 [D]; Mars 4.46 km [D].

The Earth mantle density is the one crust number without a primary
source read: it goes to the calibration list (README), with PREM as the
candidate source and the step's effect on the ETOPO hypsometry as the
check.

## Continental slope

The design falls the continent's edge to the ocean floor over a slope
angle. Secondary sources say 3-4° (Pinet 2003, via Wikipedia) [S]. Pratson
& Haxby 1996 (Geology 24, 3-6) measure five U.S. margins: regional slopes
0.5° (Louisiana) to 4.4° (west Florida), local means 2.9-12.0° [S: the
numbers came through a search summary; the abstract page read on Duke's
publication list confirms the study and its five margins only].
Harris et al. 2014 (Marine Geology 352, 4-24) was not reached (403).

Measured instead on ETOPO 2022 (every 3rd cell, 3 arc minutes, 60°S-60°N):
the area-weighted gradient of the sea floor between 200 and 2500 m deep,
within 150 km of land [V, measured, `scripts/etopo_slope.py`]:

| Band, land within | Mean | Quartiles |
| --- | --- | --- |
| 200-2500 m, 150 km | 2.42° | 0.63° / 1.50° / 3.22° |
| 200-3000 m, 150 km | 2.40° | 0.62° / 1.47° / 3.19° |
| 500-2000 m, 150 km | 2.61° | 0.76° / 1.68° / 3.46° |
| 200-2500 m, 75 km | 2.86° | 0.86° / 1.95° / 3.88° |
| 200-2500 m, 300 km | 2.23° | 0.54° / 1.32° / 2.93° |

Retained: **2.4°** (the 150 km row), with 2.2-3.1° as the spread over
the method's choices. The 5.6 km sampling smooths the steepest local
slopes, so it reads as a regional slope, the quantity the relief needs
over a 15 km grid; it sits inside Pratson & Haxby's regional range.

## Not found

- A measured H for Venus (searched Landais 2018 and 2019, Wieczorek 2007,
  Turcotte 1987 abstract, Ermakov et al. 2018 not reached).
- A published table of the Haar fluctuation per body: only Landais's
  Fig. 2, hence the recomputation.
- Harris et al. 2014's slope statistics (paywalled); a global mean
  continental slope gradient in a primary source.
- A primary source for Earth's upper mantle density read in this study
  (PREM table at IRIS refused, Turcotte & Schubert not opened).
- Mountain range widths and ridge spacing as global statistics (searched
  in the 2026-11-21 drafts): the ridged noise follows the same spectrum
  instead.

## Sources

1. Landais, F., Schmidt, F. & Lovejoy, S. 2019, "Multifractal topography
   of several planetary bodies in the Solar System", *Icarus* 319, 14-20,
   [doi:10.1016/j.icarus.2018.07.005](https://doi.org/10.1016/j.icarus.2018.07.005),
   read as [arXiv:1805.11249v1](https://arxiv.org/abs/1805.11249).
2. Landais, F., Schmidt, F. & Lovejoy, S. 2019, "Topography of
   (exo)planets", [arXiv:1902.00047](https://arxiv.org/abs/1902.00047).
3. Aharonson, O., Zuber, M. T. & Rothman, D. H. 2001, "Statistics of
   Mars' topography from the Mars Orbiter Laser Altimeter", *Journal of
   Geophysical Research: Planets* 106, 23723-23735,
   [doi:10.1029/2000JE001403](https://doi.org/10.1029/2000JE001403).
4. NOAA National Centers for Environmental Information 2022, ETOPO 2022
   15 Arc-Second Global Relief Model,
   [doi:10.25921/fd45-gt74](https://doi.org/10.25921/fd45-gt74) (the
   60 arc-second surface grid, read through the NCEI THREDDS server).
5. Smith, D. E. et al., MGS MOLA Mission Experiment Gridded Data Record
   (MEGDR) `megt90n000cb`, NASA Planetary Data System,
   [PDS Geosciences Node](https://pds-geosciences.wustl.edu/missions/mgs/megdr.html).
6. Marinova, M. M., Aharonson, O. & Asphaug, E. 2008, "Mega-impact
   formation of the Mars hemispheric dichotomy", *Nature* 453, 1216-1219,
   [doi:10.1038/nature07070](https://doi.org/10.1038/nature07070).
7. Ford, P. G. & Pettengill, G. H. 1992, "Venus topography and
   kilometer-scale slopes", *Journal of Geophysical Research* 97,
   13103-13114,
   [doi:10.1029/92JE01085](https://doi.org/10.1029/92JE01085).
8. Cogley, J. G. 1984, "Continental margins and the extent and number of
   the continents", *Reviews of Geophysics* 22, 101-122,
   [doi:10.1029/RG022i002p00101](https://doi.org/10.1029/RG022i002p00101).
9. Christensen, N. I. & Mooney, W. D. 1995, "Seismic velocity structure
   and composition of the continental crust: a global view", *Journal of
   Geophysical Research* 100, 9761-9788,
   [doi:10.1029/95JB00259](https://doi.org/10.1029/95JB00259).
10. White, R. S., McKenzie, D. & O'Nions, R. K. 1992, "Oceanic crustal
    thickness from seismic measurements and rare earth element
    inversions", *Journal of Geophysical Research* 97, 19683-19715,
    [doi:10.1029/92JB01749](https://doi.org/10.1029/92JB01749).
11. Neumann, G. A. et al. 2004, "Crustal structure of Mars from gravity
    and topography", *Journal of Geophysical Research* 109, E08002,
    [doi:10.1029/2004JE002262](https://doi.org/10.1029/2004JE002262).
12. Wieczorek, M. A. 2007, "Gravity and topography of the terrestrial
    planets", *Treatise on Geophysics* 10, 165-206,
    [doi:10.1016/B978-044452748-6.00156-5](https://doi.org/10.1016/B978-044452748-6.00156-5).
13. Pratson, L. F. & Haxby, W. F. 1996, "What is the slope of the U.S.
    continental slope?", *Geology* 24, 3-6,
    [doi:10.1130/0091-7613(1996)024<0003:WITSOT>2.3.CO;2](https://doi.org/10.1130/0091-7613(1996)024%3C0003:WITSOT%3E2.3.CO;2).
