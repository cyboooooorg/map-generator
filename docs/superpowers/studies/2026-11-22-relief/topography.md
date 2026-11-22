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

## Outer scale (measured, not used in W2)

Status: measured, not used in W2. A commit that started each province's
noise at these outer scales was tried and dropped: it barely changed the
deep sea floor (the share deeper than 6 km stayed far above Earth's, so the
cause is not the noise's large scales but the missing sea-floor age-depth
law) and it broke Earth's Hurst exponent (H left the 0.479 +- 0.08 window).
The noise therefore starts again at the circumference. The measurement
stays here as a finding for a later track.

The fluctuation cannot grow forever: beyond some lag, the *outer scale*,
the relief adds no variance, and a noise whose octaves start at one
radius has too much of it at large scale (the first census's deep sea
floor). I measured the same Haar statistic from 100 km to half the
circumference, on whole meridian great circles (a meridian and its
antimeridian, cyclic), whole surface and per province (windows wholly in
it, as above) [V, measured, `scripts/etopo_outer_scale.py`,
`scripts/mola_outer_scale.py`; n the number of windows]:

| Lag (km) | Earth, all (m) | Earth, land (m) | n | Earth, sea floor (m) | n |
| --- | --- | --- | --- | --- | --- |
| 111 | 230 | 137 | 338 379 | 255 | 620 672 |
| 400 | 481 | 272 | 292 887 | 423 | 567 637 |
| 1001 | 846 | 471 | 225 794 | 567 | 483 559 |
| 1401 | 1033 | 565 | 190 628 | 601 | 435 953 |
| 1601 | 1115 | 605 | 174 647 | **607** | 413 522 |
| 1801 | 1191 | 641 | 159 451 | 606 | 391 701 |
| 2002 | 1263 | 674 | 144 930 | 603 | 371 258 |
| 2802 | 1518 | 812 | 95 738 | 580 | 301 919 |
| 3603 | 1732 | 942 | 53 504 | 567 | 245 960 |
| 4403 | 1898 | 999 | 26 349 | 555 | 201 100 |
| 4804 | 1963 | **1002** | 18 639 | 545 | 181 399 |
| 5604 | 2056 | 950 | 8 526 | 519 | 146 877 |
| 6405 | 2104 | 792 | 2 826 | 489 | 116 663 |
| 8006 | **2127** | | 0 | 418 | 70 560 |
| 12 009 | 2038 | | 0 | 374 | 17 171 |
| 20 015 | 1663 | | 0 | | 0 |

| Lag (km) | Mars, all (m) | Mars, highlands (m) | n | Mars, lowlands (m) | n |
| --- | --- | --- | --- | --- | --- |
| 118 | 253 | 286 | 535 590 | 94 | 307 976 |
| 1006 | 851 | 703 | 438 583 | 278 | 201 730 |
| 2011 | 1249 | 910 | 360 698 | 375 | 106 223 |
| 3195 | 1527 | 1008 | 288 028 | 435 | 38 417 |
| 3609 | 1603 | **1011** | 266 200 | 444 | 23 542 |
| 4792 | 1818 | 980 | 208 391 | 539 | 947 |
| 6389 | 2127 | 981 | 132 030 | | 0 |
| 7986 | 2440 | 1005 | 66 613 | | 0 |
| 10 648 | 2857 | 543 | 2 924 | | 0 |

- **Earth's sea floor** levels off at 601-607 m from 1.4 to 2.0 Mm,
  its maximum at 1.6-1.7 Mm, then falls: outer scale **1600 km**
  [D, measured].
- **Earth's land** levels off at 999-1002 m from 4.4 to 4.8 Mm, then
  falls: outer scale **4800 km** [D, measured]. Few windows lie wholly on
  land beyond 4 Mm (18 639 at 4.8 Mm against 338 379 at 111 km), so the
  turnover rests on the largest continents (Eurasia, Africa, the Americas,
  Antarctica's ice sheet).
- **Earth's whole surface** grows to 2127 m at 8 Mm: it carries the
  continent-ocean step, which the model builds apart (Airy), so it is not
  the noise's outer scale.
- **Mars** has none as a whole: the whole surface grows up to half the
  circumference (the dichotomy is hemispheric), the lowlands as far as
  their windows reach. The highlands alone level off near 1.0 km from 3.2
  to 3.6 Mm, about Mars's radius (3.39 Mm), where a noise without outer
  scale already starts.
- The two Earth provinces disagree by a factor 3, so the outer scale is
  kept per province, not one value per body.

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
| Earth upper mantle density | 3380 kg/m³ | PREM (Dziewonski & Anderson 1981), Table I: the LID, radius 6291.0-6346.6 km (depth 24.4-80 km), ρ = 2.6910 + 0.6924 x g/cm³ with x = r / 6371 km; 3.3807 g/cm³ at the top of the mantle, 3.3747 at 80 km | [V, Table I; the two values D] |
| Mars modal crust, south / north | 58 / 32 km | Neumann et al. 2004 | [V, abstract] |
| Mars mantle − crust density | 600 kg/m³ | Neumann et al. 2004 | [V, abstract] |
| Mars crust / mantle density | 2900 / 3500 kg/m³ | Wieczorek 2007, Treatise on Geophysics 10.05, Fig. 11 caption (model parameters for Mars) | [V] |

The Airy step `(t − t_o)(1 − ρc/ρm)` gives, for Earth,
(41.1 − 7.1) × (1 − 2830 / 3380) = 34 × 0.16272 = 5.53 km [D]; for Mars
(58 − 32) × (1 − 2900 / 3500) = 4.46 km [D]. The legacy's assumed
3300 kg/m³ gave 4.84 km; it was replaced by PREM at the user's review
(2026-11-22). PREM was read on an open copy of the paper (Harvard CfA,
`lzeng/papers/PREM.pdf`); the IRIS table had refused access.

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
14. Dziewonski, A. M. & Anderson, D. L. 1981, "Preliminary reference
    Earth model", *Physics of the Earth and Planetary Interiors* 25,
    297-356,
    [doi:10.1016/0031-9201(81)90046-7](https://doi.org/10.1016/0031-9201(81)90046-7).
