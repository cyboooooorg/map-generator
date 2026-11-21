# What is inside a star?

Seen from: the system engine's star state, which carries the star's
interior (layers, and density, pressure, temperature and fusion share at
each radius) and its trace nodes `system.star.layer-N.*`. No viewer shows
the interior yet in this rewrite; the star view will draw on it.

## Short answer

A star is a ball of hot gas held up by its own heat, and the heat has to
get out. In some layers light carries it, step by step through the dense
gas: that is radiative transport. In others the gas boils, rising hot and
sinking cool: that is convection. Which one wins depends on the mass. The
Sun has a radiative core and a convective envelope that starts at about
0.71 of its radius. A star below about a third of the Sun's mass is
convective from the centre to the surface. A star above about 1.1 to 1.2
Sun masses has the opposite shape, a convective core inside a radiative
zone. Fusion is always in the core: nearly all of the Sun's power comes
from inside a quarter of its radius.

## The science

### Two ways to carry heat out

Fusion releases energy at the centre, and the energy must cross the star
before it shines (Kippenhahn et al. 2012). The gas carries it in one of
two ways. In **radiative transport**, photons are absorbed and re-emitted
by the gas, and the energy random-walks outward. This works while the gas
is transparent enough. In **convection**, a bubble of gas rises, gives up
its heat and sinks again. It sets in where radiation alone would need a
temperature that falls too steeply with radius (the Schwarzschild
criterion): then a rising bubble stays hotter and lighter than its
surroundings, and keeps rising (Kippenhahn et al. 2012). Two things make
the temperature fall steeply: gas that is opaque, and an energy source
packed near the centre.

**In words.** Where the gas is too opaque for light to carry the heat,
the gas moves instead.

### The Sun: a radiative core, a convective envelope

In the Sun the core is hot enough for light to carry the energy, and it
does so out to about 0.71 R. Beyond that the gas is cool and opaque, as
atoms keep their electrons and absorb the light, and it boils.
Helioseismology, the study of the Sun's sound waves, finds the base of
this convective envelope at 0.713 R (Basu & Antia 1997).

### A small star is convective throughout

Below about 0.35 M☉ a star's gas is so opaque, because it is so cool and
dense, that radiation cannot carry the heat anywhere: the star is
convective from centre to surface (Chabrier & Baraffe 1997). One
consequence is that the whole star is mixed, and its hydrogen is stirred
into the core to be burnt, which is why the lightest star lives for
about 11,000 Gyr (see [What does a G-type star
mean?](star-types.md)). Another is that such a star has a simple
structure. A fully convective gas, with the energy carried by motion
alone, follows nearly an adiabat, and its pressure and density follow a
polytrope of index $n = 1.5$ (Chandrasekhar 1939; Kippenhahn et al.
2012).

### A heavy star has a convective core

Above about 1.1 to 1.2 M☉ the energy source changes. The Sun burns
hydrogen mostly by the proton-proton chain, whose rate grows with
temperature as roughly $T^4$ at the Sun's central 1.5 × 10⁷ K; the
exponent is larger in cooler cores, about 6 at 5 × 10⁶ K (Kippenhahn et
al. 2012, §18.5.1). A heavier star is hotter inside, and the CNO cycle takes over,
which fuses hydrogen using carbon, nitrogen and oxygen as catalysts and is
far more sensitive to temperature. The energy is then released in a small
volume at the very centre, the temperature falls steeply outward, and the
core convects. Such a star has a convective core, a radiative zone around
it, and often a thin convective layer near the surface where hydrogen and
helium ionise.

The rule is not a clean cut. Near the zero-age main sequence (ZAMS, when a
star has just settled into hydrogen burning), a star like the Sun has a
small transient convective core, while its burning of ³He and carbon is
not yet in equilibrium. It is gone after a tenth of the hydrogen is burnt.

### Where fusion happens

Fusion needs a high temperature, and the temperature falls fast outward.
So the power comes from a small central region. In the standard solar
model B23, 99 % of the Sun's energy is generated inside 0.256 R, and the app's
model gives 98.7 % inside 0.25 R (Herrera & Serenelli 2023). The fusion
is a small furnace in the middle of a much larger star.

## How the app computes it

The app does not solve the equations of stellar structure while it draws a
star. It reads a table made once, offline, and interpolates in it, as
`science::stellar` does. The tables, their sources and their measured
errors are documented in
[`crates/science/data/STELLAR.md`](../../crates/science/data/STELLAR.md).

1. **From 0.3 to 8 M☉**, the table is a grid of 312 runs of YREC, a stellar
   evolution code (Pinsonneault 2025), at four metallicities
   ([Fe/H] = −1, −0.5, 0 and +0.5). It uses a grey atmosphere, the GS98
   solar mixture, and no diffusion and no rotation. For each run the
   table keeps a handful of snapshots from the ZAMS to the end of the
   main sequence, each with its transport layers and about 50 radial
   points. The app interpolates in mass, in metallicity, and in the
   fraction of the main-sequence life elapsed (set by the central
   hydrogen).
2. **From 0.08 to 0.3 M☉**, the YREC runs fail. The star's radius,
   luminosity and temperature come from Baraffe et al. (2015), at solar
   metallicity, with the metallicity shift taken from Dartmouth (Dotter et
   al. 2008). Between 0.30 and 0.35 M☉ the two sources are blended
   so that the values do not jump.
3. **Below 0.30 M☉**, the interior is an $n = 1.5$ polytrope scaled to the
   star's mass and radius, in one convective layer. Its central
   temperature comes from Baraffe's tables: the polytrope's own ideal-gas
   value would be 6.2 × 10⁶ K at 0.08 M☉ and 5 Gyr, against 3.61 × 10⁶ K
   in Baraffe et al., because these stars are partly degenerate. The fusion
   share follows a rate $\varepsilon \propto \rho\,T^\nu$, where the
   pp-chain exponent $\nu \approx 11.3\,T_6^{-1/3} - 2/3$ is taken at the
   star's central temperature: about 6.7 at 0.08 M☉ and 5.1 at 0.29 M☉.
4. **The star has layers**, one per energy transport: a convective or
   radiative core, a radiative zone, a convective envelope. The outermost
   ends at the photosphere (r = R); the atmosphere above it is not
   modelled. The system engine's star state carries the layers with their
   bounds, and the trace records each layer's outer radius, mass and
   transport.

### Checking it against the Sun

The 1 M☉ star at 4.57 Gyr is compared with the B23 standard solar model.
Both are in the table of
[STELLAR.md](../../crates/science/data/STELLAR.md#the-sun-yrec-against-b23):

| Quantity | App | B23 |
| --- | --- | --- |
| Radius | 0.9995 R☉ | 1 R☉ (nominal) |
| Central density | 146.1 g cm⁻³ | 150.0 g cm⁻³ |
| Central temperature | 1.536 × 10⁷ K | 1.558 × 10⁷ K |
| Base of the convective envelope | 0.727 R | 0.713 R (helioseismology) |
| Share of fusion inside 0.25 R | 0.987 | 0.987 |

Density, pressure and temperature agree within 7.1 % from the centre to
0.95 R, and the enclosed mass within 0.6 %. The envelope base is 0.014 R
too deep. Below 0.30 M☉ the polytrope's density is within 4 % of YREC's to
0.8 R at 0.3 M☉, and its central density within 5 % of Baraffe et al. at
0.3 M☉ and 9 % at 0.08 M☉.

## Where the app simplifies

- **No diffusion, no rotation.** The YREC runs let no element settle and
  no rotation mix the gas. The app keeps the model's 0.727 R for the
  envelope base and does not tune it to 0.713 R.
- **A grey atmosphere.** YREC treats the photosphere as a grey gas. At
  0.3 M☉ it comes out 60 to 83 K hotter than Baraffe et al. and about 115 K
  hotter than Dartmouth, both of which use model atmospheres. The blend
  over 0.30-0.35 M☉ spreads the step.
- **A sharp switch of the interior at 0.30 M☉.** The values of radius and
  temperature are blended, but the interior is not: below 0.30 M☉ it is the
  polytrope, above it the YREC profile. YREC at 0.4 M☉ already has a
  radiative zone around a convective core, so the true change of structure
  is not at one mass.
- **The mass range.** The tables cover 0.08 to 8 M☉ and [Fe/H] from −1 to
  +0.5. A star outside is refused. The hot stars above 8 M☉ are
  tracked in issue #615.
- **Between the grid masses**, the values and profiles are interpolated.
  The tracks are 0.1 M☉ apart up to 2 M☉, then 0.2 M☉; the profiles are
  0.1 M☉ apart up to 2.6 M☉, then 1 M☉ from 3 to 8 M☉. The worst
  interpolation error of a profile is 0.136 dex for density, pressure or
  temperature at the worst radius, and 0.078 dex inside 0.9 R. A
  transient or thin layer present in one profile only is kept while that
  profile is the nearer one.
- **Only the main sequence.** The model stops at the end of hydrogen
  burning in the core; a star past it is not drawn.

## Sources

1. Kippenhahn, R., Weigert, A. & Weiss, A. 2012, *Stellar Structure and
   Evolution*, 2nd ed., Springer,
   [doi:10.1007/978-3-642-30304-3](https://doi.org/10.1007/978-3-642-30304-3).
2. Basu, S. & Antia, H. M. 1997, "Seismic measurement of the depth of the
   solar convection zone", *Monthly Notices of the Royal Astronomical
   Society* 287, 189,
   [doi:10.1093/mnras/287.1.189](https://doi.org/10.1093/mnras/287.1.189).
3. Herrera, Y. & Serenelli, A. 2023, standard solar model B23
   (`struct+nu_SF3_MB22m.dat`), Zenodo,
   [doi:10.5281/zenodo.10174172](https://doi.org/10.5281/zenodo.10174172),
   CC-BY 4.0.
4. Chabrier, G. & Baraffe, I. 1997, "Structure and evolution of low-mass
   stars", *Astronomy & Astrophysics* 327, 1039,
   [arXiv:astro-ph/9704118](https://arxiv.org/abs/astro-ph/9704118).
5. Chandrasekhar, S. 1939, *An Introduction to the Study of Stellar
   Structure*, University of Chicago Press (the $n = 1.5$ polytrope).
6. Baraffe, I., Homeier, D., Allard, F. & Chabrier, G. 2015, "New evolutionary
   models for pre-main sequence and main sequence low-mass stars down to
   the hydrogen-burning limit", *Astronomy & Astrophysics* 577, A42,
   [doi:10.1051/0004-6361/201425481](https://doi.org/10.1051/0004-6361/201425481).
7. Dotter, A. et al. 2008, "The Dartmouth Stellar Evolution Database",
   *The Astrophysical Journal Supplement Series* 178, 89,
   [doi:10.1086/589654](https://doi.org/10.1086/589654); tracks read
   through the Kiauhoku grids (Claytor et al. 2020, Zenodo 14908017,
   CC-BY 4.0).
8. Pinsonneault, M. 2025, YREC, Zenodo 17981745 (the code behind the
   312-run grid, run with `yrec` 2026.2.dev1).

## See also

- [What does a G-type star mean?](star-types.md): the mass that sets a
  star's type and lifetime.
- [Why do some stars have spots, and how fast do they
  turn?](star-activity.md): what the convective envelope does at the
  surface.
- [crates/science/data/STELLAR.md](../../crates/science/data/STELLAR.md):
  every number of the tables, with the measured comparisons.
- [docs/superpowers/specs/2026-11-21-system-draw-design.md](../superpowers/specs/2026-11-21-system-draw-design.md):
  the design of the star in the system engine.
