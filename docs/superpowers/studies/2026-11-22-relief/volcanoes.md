# Relief study: volcanoes

Part of the [relief study](README.md); tags as in
[topography](topography.md#relief-study-the-statistical-shape-of-the-topography).
Legacy numbers are those of `legacy-final:crates/mapgen/src/generation.rs`
(lines 140-160, 730-790) and of `volcanism` in `planet/interim.rs`,
`0.5 sqrt(F / 0.087)`, 1 on a heat pipe, halved on a stagnant lid.

## Heat flux: the two anchors

- **Earth.** Pollack, Hurter & Johnson 1993: the mean heat flows of
  continents and oceans "are 65 and 101 mW m⁻², respectively, which when
  areally weighted yield a global mean of 87 mW m⁻² and a global heat
  loss of 44.2 × 10¹² W" [V, abstract]. The drafts attributed 0.087 W/m²
  to Davies & Davies 2010; that paper gives 47 ± 2 TW, about 0.092 W/m²
  [S, the 2026-11-21 draft read its abstract]. Retained: Pollack's
  **0.087 W/m²**, as the legacy and the plan's test (44.2 TW over Earth's
  surface).
- **Heat pipe.** Moore & Webb 2013: Io "transports about 40 times Earth's
  heat flux (2.5 W m⁻²) from the interior to the surface through
  volcanism" (p. 501) [V, copy hosted by Durham's mantle plumes site].
  Retained: **2.5 W/m²** as the flux at which volcanism is total.
- The flux the design reads is the planet's total heat over its surface,
  `(radiogenic + tidal + secular) / 4πR²`: an areal rate, the quantity
  both anchors are stated in.

## Heat flux to volcanic intensity: no law

No published relation links the surface heat flux to a volcanic surface
share or a density of cones. Searched (2026-11-21 draft and here): Kite,
Manga & Gaidos 2009 (melt production per unit mass for plates and
stagnant lids, no power law against flux [S, draft read the arXiv
text]), Crisp 1984 (Earth's magma output, 26-34 km³/yr [S]), Moore &
Webb 2013, Kankanamge & Moore 2019 (a parameterisation of the volcanic
heat flux of heat-pipe planets, doi:10.1029/2018JE005800 [V, abstract]:
it computes the melt flux from the interior, not a surface share).

The legacy's square root, its 0.5 at Earth, its cap reached at four
times Earth's flux and its halving on a stagnant lid are calibrations.
The design keeps a declared curve through the two anchors (Earth's value
of the curve and 1 at 2.5 W/m²); its shape and Earth's value are the
calibration.

## Volcanic provinces

The share of a surface under volcanic provinces: the Moon's maria about
16-17 % [S]; Mercury's smooth plains about 27 % of the surface, more
than 65 % of them volcanic (Denevi et al. 2013) [S, the draft read the
abstract; not re-read]; Venus's volcanic plains a half to three quarters
[S]. They bound the legacy's 0.3 at full intensity by a factor of two;
no source gives the share against intensity: a calibration. W2 draws no
volcanic province, so this share is not ported (calibration 3).

## Cones

- **Shape.** Grosse & Kervyn 2018, 158 shield volcanoes from SRTM DEMs:
  "height/basal width (H/WB) ratios mostly from 0.01 to 0.1", average
  slopes mostly 1-15°, volumes "from 0.1 to >1000 km³", a shield to
  stratovolcano threshold near 12° and H/WB 0.10 [V, abstract, read on
  the CONICET repository record]. The drafts gave the DOI
  `10.1016/j.geomorph.2017.12.028`, which is another paper (coastal
  boulders); the right one is **10.1016/j.geomorph.2017.12.017**
  (Crossref checked).
- **Tallest.** Plescia 2004, MOLA DEMs of Mars's volcanoes: "relief varies
  from 1.0 km to almost 22 km", average flank slopes under 1° to about
  10° [V, abstract]. Scaled to Earth's gravity by Weisskopf's `1/g`
  (Mars 3.73 m/s², NASA fact sheet [V]; standard gravity 9.80665 m/s²):
  22 × 3.73 / 9.80665 = **8.4 km** [D]. Mauna Loa rises about 9 km from
  the sea floor [S, USGS]: consistent.
- **Sizes between.** No global distribution of cone sizes was found; the
  volumes 0.1-1000 km³ bound it. How sizes are drawn inside the bounds is
  a calibration.
- **Isolated cone spacing.** Not found as a global law: ten Brink 1991
  ties hotspot volcano spacing to the elastic thickness of the plate [S],
  Hahn & Byrne 2023 count 4-10 edifices per 1000 km² in Venus's shield
  fields [S]. The legacy lattice (one cone per 900 km cell, in half of
  them) stays a calibration.

## Arc volcanoes

de Bremond d'Ars, Jaupart & Sparks 1995, 479 volcanic systems in 16
margins: arcs are ribbons with a length-to-width ratio of about 10;
"There is no characteristic spacing between volcanoes"; in each arc the
spacing "is best represented by a Gamma distribution which corresponds to
randomly generated points in the same geometrical conditions"
[V, abstract]. The Gamma parameters (mean, shape) are not in the
abstract; the full text was not reached (Wiley, 403). Regional spacings
of about 50 km (Alaska Peninsula) and 70 km (west of Cook Inlet) come from
the Alaska Science Forum [S].

Consequence for the design: the paper's result is that arc volcanoes sit
like **random points in a ribbon**. Placing arc cones uniformly at random
along the arc band reproduces it without a Gamma shape parameter
(Definition, after the paper); only the density, one cone per so many
kilometres of arc, is left. Its value is a calibration (legacy: one per
200 km cell in 80 % of cells, one per 250 km; regional spacings 50-70 km
[S]).

The legacy arc cone height (10 km above the vent) is replaced by the
cone shape above: `CONE_ASPECT` and the ceiling apply to every cone.

## Not found

- A law from heat flux to volcanic surface share or cone density.
- The mean and shape of de Bremond d'Ars's Gamma laws (full text not
  reached).
- A global distribution of volcano sizes or of isolated-volcano spacing.
- A global volcanic surface share for Mars (Tanaka et al. 2014 unit areas
  not reached) and Venus (Ivanov & Head 2011 area table not opened).

## Sources

1. Pollack, H. N., Hurter, S. J. & Johnson, J. R. 1993, "Heat flow from
   the Earth's interior: analysis of the global data set", *Reviews of
   Geophysics* 31, 267-280,
   [doi:10.1029/93RG01249](https://doi.org/10.1029/93RG01249).
2. Davies, J. H. & Davies, D. R. 2010, "Earth's surface heat flux",
   *Solid Earth* 1, 5-24,
   [doi:10.5194/se-1-5-2010](https://doi.org/10.5194/se-1-5-2010).
3. Moore, W. B. & Webb, A. A. G. 2013, "Heat-pipe Earth", *Nature* 501,
   501-505, [doi:10.1038/nature12473](https://doi.org/10.1038/nature12473).
4. Kankanamge, D. G. J. & Moore, W. B. 2019, "A parameterization for
   volcanic heat flux in heat pipe planets", *Journal of Geophysical
   Research: Planets* 124, 114-127,
   [doi:10.1029/2018JE005800](https://doi.org/10.1029/2018JE005800).
5. Kite, E. S., Manga, M. & Gaidos, E. 2009, "Geodynamics and rate of
   volcanism on massive Earth-like planets", *Astrophysical Journal* 700,
   1732-1749,
   [doi:10.1088/0004-637X/700/2/1732](https://doi.org/10.1088/0004-637X/700/2/1732).
6. Denevi, B. W. et al. 2013, "The distribution and origin of smooth
   plains on Mercury", *Journal of Geophysical Research: Planets* 118,
   891-907, [doi:10.1002/jgre.20075](https://doi.org/10.1002/jgre.20075).
7. Grosse, P. & Kervyn, M. 2018, "Morphometry of terrestrial shield
   volcanoes", *Geomorphology* 304, 1-14,
   [doi:10.1016/j.geomorph.2017.12.017](https://doi.org/10.1016/j.geomorph.2017.12.017).
8. Plescia, J. B. 2004, "Morphometric properties of Martian volcanoes",
   *Journal of Geophysical Research: Planets* 109, E03003,
   [doi:10.1029/2002JE002031](https://doi.org/10.1029/2002JE002031).
9. de Bremond d'Ars, J., Jaupart, C. & Sparks, R. S. J. 1995,
   "Distribution of volcanoes in active margins", *Journal of Geophysical
   Research* 100, 20421-20432,
   [doi:10.1029/95JB02153](https://doi.org/10.1029/95JB02153).
10. Weisskopf, V. F. 1975, "Of atoms, mountains, and stars: a study in
    qualitative physics", *Science* 187, 605-612,
    [doi:10.1126/science.187.4177.605](https://doi.org/10.1126/science.187.4177.605).
11. NASA Space Science Data Coordinated Archive,
    [Mars fact sheet](https://nssdc.gsfc.nasa.gov/planetary/factsheet/marsfact.html).
