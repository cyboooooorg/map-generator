# How tall do volcanoes grow, and how many are there?

Seen from: the relief of a rocky world, its cones on the map, and the
heat it loses (the planet model of the rewrite; no viewer yet).

## Short answer

A volcano is a pile of lava and ash over the vent that fed it. Most
shield volcanoes are low and wide, between a hundredth and a tenth as
high as they are broad. How high they can grow depends on gravity: under
Mars's weak pull, Olympus Mons rises almost 22 km above its
surroundings. How many a planet has depends on how much heat its
interior lets out through melt: Earth's mostly leaves by conduction
through its plates, Io's almost all through volcanoes. No measurement
yet says how many volcanoes a given heat flux makes, so the app
interpolates between those two ends. Along subduction zones, volcanoes
line up in arcs, scattered like random points rather than evenly spaced.

## The science

### The shape of a volcano

Grosse & Kervyn (2018) measured 158 shield volcanoes on 90 m elevation
models. Their height over their basal width,

```math
a = \frac{H}{W_B},
```

lies mostly between 0.01 and 0.1, their mean flank slopes between 1° and
15°, their volumes between 0.1 and more than 1000 km³. Above about 12° and
an aspect of 0.10 a cone reads as a stratovolcano rather than a shield.

**In words.** A typical shield is ten to a hundred times as wide as it
is tall.

**Orders of magnitude.** A shield 2 km tall spans 20 to 200 km at its
base; one 1 km tall, 10 to 100 km.

### How tall

The weight argument of [Why are mountains lower on heavier
planets?](mountains-and-gravity.md) holds for a volcano too: rock bears a
column up to about $\sigma / \rho g$. Plescia (2004) measured Mars's
volcanoes on the MOLA altimetry: their relief runs from 1.0 km to almost
22 km. Scaled to Earth's gravity by $1/g$ (Mars's 3.73 m/s² against the
standard 9.80665),

```math
H_\text{max} \approx 22\ \text{km} \times \frac{3.73}{9.81} \approx 8.4\ \text{km},
```

**In words.** The tallest volcano known, carried to Earth's gravity,
would stand about 8 km high.

### How many: heat that leaves through melt

A planet loses its internal heat at its surface. Earth's mean flux is
87 mW/m², 44.2 TW in all (Pollack, Hurter & Johnson 1993), most of it by
conduction through the plates. Io loses about 2.5 W/m², some 40 times
Earth's flux, almost all of it through volcanism: a heat pipe (Moore &
Webb 2013).

```math
F = \frac{Q}{4 \pi R^2}
```

**In words.** The flux $F$ is the planet's heat $Q$ (radiogenic, tidal
and secular) spread over its surface of radius $R$.

**Orders of magnitude.** Earth, 0.087 W/m²; Io, 2.5 W/m²; Mars and the
Moon, under 0.02 and 0.012 W/m² (Moore & Webb 2013).

### Arcs

Over a sinking slab, volcanoes form a ribbon about ten times as long as
it is wide. Their spacing has no characteristic length: in each of 16
margins it follows a Gamma distribution, the one random points give in
the same ribbon (de Bremond d'Ars, Jaupart & Sparks 1995).

### What we do not know

- **Heat to volcanoes.** No law links a heat flux to the share of a
  surface under volcanic provinces or to a number of volcanoes; models
  give the melt produced (Kite, Manga & Gaidos 2009; Kankanamge & Moore
  2019), not where it surfaces.
- **Sizes.** A global distribution of volcano sizes was not found, nor a
  rule for the spacing of isolated volcanoes.

## In the app

### What you see

Cones on the relief of rocky worlds: scattered over volcanic provinces,
more of them the more heat the planet loses, and in a line behind the
trenches of plate worlds. A cold planet has none away from its arcs.

### How it computes them

The rewrite (`science::relief`, `world::relief`; the values and their
sources are in the [relief
study](../superpowers/studies/2026-11-22-relief/volcanoes.md)):

1. **Intensity.** The heat flux $F$ above, from the planet's keyframe,
   sets a volcanic intensity from 0 to 1 along a declared curve through
   two anchors: Earth's flux, 0.087 W/m², and a heat pipe's, 2.5 W/m²,
   where it reaches 1.
2. **Provinces and isolated cones.** Cones on a lattice, present in a
   share of the cells that grows with the intensity.
3. **Arc cones.** Random points along the arcs of the plate boundaries,
   after de Bremond d'Ars et al.
4. **Shape.** Every cone has an aspect between 0.01 and 0.1 (Grosse &
   Kervyn 2018) and stands at most 8.4 km at standard gravity, scaled by
   the relief scale $\min(g_0/g, 2)$ like the mountains.

### Where it departs from the science

Calibrations, each with its search in the [relief
study](../superpowers/studies/2026-11-22-relief/README.md#final-calibration-list):

- **The intensity curve** between its anchors, and Earth's value on it.
- **The share of the surface in volcanic provinces**, bounded by the
  Moon's maria (16–17 %), Mercury's volcanic plains (about 18 %) and
  Venus's (a half to three quarters), but not measured against a flux.
- **How cone sizes are drawn** inside the published ranges.
- **The spacing of isolated cones** and **the density of arc cones**: the
  arcs of Alaska carry one volcano every 50–70 km, a regional figure, not
  a global one.

The flux is the whole planet's heat over its surface; a stagnant lid
that conducts its crust's radiogenic heat without melting would make
fewer volcanoes than its flux says.

## Sources

1. Grosse, P. & Kervyn, M. 2018, "Morphometry of terrestrial shield
   volcanoes", *Geomorphology* 304, 1–14,
   [doi:10.1016/j.geomorph.2017.12.017](https://doi.org/10.1016/j.geomorph.2017.12.017).
2. Plescia, J. B. 2004, "Morphometric properties of Martian volcanoes",
   *Journal of Geophysical Research: Planets* 109, E03003,
   [doi:10.1029/2002JE002031](https://doi.org/10.1029/2002JE002031).
3. Weisskopf, V. F. 1975, "Of atoms, mountains, and stars: a study in
   qualitative physics", *Science* 187, 605–612,
   [doi:10.1126/science.187.4177.605](https://doi.org/10.1126/science.187.4177.605).
4. Pollack, H. N., Hurter, S. J. & Johnson, J. R. 1993, "Heat flow from
   the Earth's interior: analysis of the global data set", *Reviews of
   Geophysics* 31, 267–280,
   [doi:10.1029/93RG01249](https://doi.org/10.1029/93RG01249).
5. Moore, W. B. & Webb, A. A. G. 2013, "Heat-pipe Earth", *Nature* 501,
   501–505, [doi:10.1038/nature12473](https://doi.org/10.1038/nature12473).
6. de Bremond d'Ars, J., Jaupart, C. & Sparks, R. S. J. 1995,
   "Distribution of volcanoes in active margins", *Journal of Geophysical
   Research* 100, 20421–20432,
   [doi:10.1029/95JB02153](https://doi.org/10.1029/95JB02153).
7. Kite, E. S., Manga, M. & Gaidos, E. 2009, "Geodynamics and rate of
   volcanism on massive Earth-like planets", *Astrophysical Journal* 700,
   1732–1749,
   [doi:10.1088/0004-637X/700/2/1732](https://doi.org/10.1088/0004-637X/700/2/1732).
8. Kankanamge, D. G. J. & Moore, W. B. 2019, "A parameterization for
   volcanic heat flux in heat pipe planets", *Journal of Geophysical
   Research: Planets* 124, 114–127,
   [doi:10.1029/2018JE005800](https://doi.org/10.1029/2018JE005800).
9. Denevi, B. W. et al. 2013, "The distribution and origin of smooth
   plains on Mercury", *Journal of Geophysical Research: Planets* 118,
   891–907, [doi:10.1002/jgre.20075](https://doi.org/10.1002/jgre.20075).
10. NASA Space Science Data Coordinated Archive,
    [Mars fact sheet](https://nssdc.gsfc.nasa.gov/planetary/factsheet/marsfact.html).

## See also

- The [relief study](../superpowers/studies/2026-11-22-relief/README.md)
  of the rewrite: every value, its source and the calibration list.
- [Why are mountains lower on heavier planets?](mountains-and-gravity.md)
  and [Do all rocky planets have plate tectonics?](plate-tectonics.md)
- The [world relief design](../superpowers/specs/2026-11-21-world-relief-design.md).
