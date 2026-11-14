# What are planets made of, and how big does that make them?

Seen from: the model behind the planet cards, in the generator's code
(`crates/mapgen/src/planet/`). Part 1 of the physical planet model
computes it for every planet of a stored system but shows it nowhere yet,
and does not change any generated world.

## Short answer

A planet is a stack of layers: an iron core, a rock mantle, a shell of
water and ices, and sometimes a hydrogen and helium envelope. How much of
each depends on where the planet formed (inside or beyond the ice line),
around what star (its iron content) and how massive it grew (a heavy core
captures gas). Given the layers, the radius follows from how much each
material is squeezed by the planet's own weight, plus the thickness of the
envelope. The model then adds the heat made inside (radioactive decay and
tides), the ice shell a frozen water layer keeps over an ocean, and reads
a class off the result: rocky, super-Earth, ice world,
mini-Neptune, ice giant, hot Neptune, gas giant or hot Jupiter. No class name goes into
the calculation; it only comes out.

## The science

### What a planet starts with

**The ice line.** Beyond a certain distance from the star, water freezes
in the protoplanetary disc and solid planets form with a large share of
ice. The model reads the system's ice line (see
[How do asteroid belts form?](asteroid-belts.md) and
[giant planet placement](giant-planet-placement.md)): beyond it, ices make
20 to 50 % of the non-gas mass, like Ganymede and Titan (about 40 to 50 %);
inside it, 0.01 to 0.2 % (Earth holds 0.023 %, Marty 2012). One planet in
ten inside the line is drawn as if it had formed beyond it and migrated in.

**Metallicity.** Stars rich in iron form planets rich in iron: Adibekyan et
al. (2021) find the iron fraction of rocky planets rising with their
star's. The model draws the iron core's share of the refractory (iron and
rock) mass around Earth's 0.325, shifted by 0.15 per dex of the star's
`[Fe/H]`, with a spread of 0.05. A few planets close to the star (5 % inside 0.3
of the ice line) are drawn Mercury-like, their mantle stripped by giant
impacts, with a core share of 0.55 to 0.75.

**Core accretion.** A solid core that grows in the gas disc captures
hydrogen and helium. Below about 0.5 Earth masses a body finishes forming
after the disc is gone and keeps none (Mars). Between 0.5 and 5 Earth
masses the chance to have gathered gas rises linearly; those that did hold
a few percent of their mass in it (Lee & Chiang 2016). From about 10 Earth
masses the core accretes gas faster than solids and runs away into a giant
(Pollack et al. 1996): Uranus and Neptune hold 10 to 20 % of their mass in
gas, Jupiter and Saturn 85 to 95 % (Guillot 2005, Helled et al. 2011).

### From layers to radius

A sphere of density $\rho$ and mass $M$ has radius $R = (3M / 4\pi\rho)^{1/3}$.
Rock under pressure is denser than rock at the surface, so a heavier planet
is smaller than its uncompressed size. For the iron and rock, Zeng,
Sasselov & Jacobsen (2016) fit models of Earth-like planets scaled from
PREM, with $M$ in Earth masses and $c$ the core's share of the refractory
mass:

```math
R / R_\oplus = (1.07 - 0.21\,c)\,M^{1/3.7}
```

**In words.** A planet is slightly smaller the more iron it holds, and
grows more slowly than the cube root of its mass.

**Orders of magnitude.** Earth ($c$ = 0.325, 1 $M_\oplus$) gives 1.00
$R_\oplus$; a 5-Earth-mass planet of the same make gives 1.5 times that.
The fit was made for 1 to 8 Earth masses.

The model takes the smaller of this radius and the uncompressed size (so
Mars, which does not compress its rock, is not shrunk below its real
size). The ratio of the two volumes, $k$, measures the compression. The
iron core shrinks by $k$, and the water shell, softer than rock, by
$k^{1.5}$, a calibration against the models of Zeng et al. (2019): a
5-Earth-mass planet with half its mass in water comes out at 1.95 $R_\oplus$,
between their 1.911 (300 K) and 1.981 (700 K), and within 6 % of them from
1 to 20 Earth masses.

**The envelope.** A hydrogen and helium envelope adds to the radius a
thickness that Lopez & Fortney (2014) fit:

```math
R_\mathrm{env} = 2.06\,R_\oplus\, M^{-0.21} \left(\frac{f}{5\,\%}\right)^{0.59} S^{0.044} \left(\frac{t}{5\,\mathrm{Gyr}}\right)^{-0.11}
```

with $M$ the mass in Earth masses, $f$ the envelope's share of the mass,
$S$ the light received (Earth's = 1) and $t$ the age. The age exponent is
that of their solar-metallicity models (the text of the paper gives −0.11
for them and −0.18 for the enhanced-opacity models the printed fit was
made on).

**In words.** A heavier planet squeezes its envelope; a thicker envelope
swells it; more light and a younger age swell it a little. The relation
holds on the grid it was fitted on (1 to 20 Earth masses, 0.1 to 1000
times Earth's light, 0.1 to 10 Gyr) and the model clamps to it.

**Orders of magnitude.** 2 % of a 5-Earth-mass core's mass in gas adds about
0.9 $R_\oplus$, which is why most planets above 1.6 $R_\oplus$ are not
rocky (Rogers 2015).

For a giant (10 Earth masses or more with at least 5 % gas) the model uses the
generator's existing giant radius, through Neptune, Saturn and Jupiter
(Chen & Kipping 2017), inflated when hot (Thorngren & Fortney 2018). The
solid layers stay inside it, and the envelope fills the rest.

### The heat inside

**Radiogenic.** Uranium-238 and -235, thorium-232 and potassium-40 decay
and heat the mantle. Earth makes about 20 TW today (Arevalo et al. 2009).
The model splits that among the four isotopes by their present
abundances and heat per kilogram (McDonough & Sun 1995, Turcotte &
Schubert 2014) and runs each backwards or forwards by its half-life:

```math
H = \sum_i s_i\, H_\oplus\, \frac{m_\mathrm{sil}}{m_{\mathrm{sil},\oplus}}\, a\, e^{\lambda_i (4.567 - t)}
```

where $s_i$ is each isotope's share of Earth's heat, $H_\oplus$ = 20 TW,
$m_\mathrm{sil}$ the silicate mass, $a$ the planet's abundance relative to
Earth's, $\lambda_i = \ln 2 / t_{1/2,i}$ and $t$ the age in Gyr.

**In words.** The younger the planet, the more heat; the isotopes with
short half-lives dominate early.

**Orders of magnitude.** Earth formed with about five times its present
radiogenic heat.

**Tidal.** A host raises tides in a planet on an eccentric orbit, and
friction turns them into heat (Peale, Cassen & Reynolds 1979):

```math
\dot E = \frac{21}{2}\,\frac{k_2}{Q}\,\frac{G M_h^2 R^5 n\, e^2}{a^6}
```

with $M_h$ the host's mass, $R$ the planet's radius, $a$ and $e$ its orbit's
semi-major axis and eccentricity, $n$ its mean motion, and $k_2/Q$ the
tidal response. The host is the star for a planet; for a moon it is its
planet, on the moon's orbit around it (`Context::with_tide`), while the
star's light still arrives at the planet's orbit.

The tidal response $k_2/Q$ depends on what the planet is made of, read
from its composition, never from a class or a type name:

| Body | Rule | $k_2/Q$ | Source |
| --- | --- | --- | --- |
| Enveloped | at least 0.1 % of the mass in gas | $1.1 \times 10^{-5}$ | Jupiter's, measured (Lainey et al. 2009) |
| Icy | otherwise, at least 5 % water and ices | $2.5 \times 10^{-3}$ | calibration (Europa, Enceladus) |
| Rocky | the rest | 0.015 | Io's, measured (Lainey et al. 2009) |

The thresholds are the mini-Neptune and ice world lines of the classes
below. A gas envelope sets the radius, whose fifth power the heat follows,
but dissipates little: Jupiter's $k_2/Q$ is about 1400 times smaller than
Io's. Saturn's, measured at $1.6$ to $2.3 \times 10^{-4}$ (Lainey et al.
2012; Lainey 2016), is 15 to 20 times Jupiter's; the eccentricities of
close-in giants point to a weaker one, a $Q' = 3Q/2k_2$ of $10^{6.5}$, about
$5 \times 10^{-7}$ (Jackson, Greenberg & Barnes 2008). No icy moon has a
measured $k_2/Q$: the icy value is a $k_2$ of about 0.25 for an ice shell
over an ocean (Wahr et al. 2006) over a $Q$ of 100, a calibration.

**Orders of magnitude.** Io, with $k_2/Q$ = 0.015, gets about $10^{14}$ W
from Jupiter (Lainey et al. 2009). Europa, with the icy value, gets about
$1.1 \times 10^{12}$ W, 35 mW/m², within the 10 to 50 mW/m² of tidal
heating that thermal models of its ice shell consider (Kimura 2024), and
Enceladus about 3.7 GW, a quarter of the 15.8 GW it radiates (Howett et
al. 2011). A Jupiter at 0.04 AU from the Sun on an orbit of eccentricity
0.1 gets about 4 % of the starlight it absorbs; with Io's response, which
every body took before, it got 60 times that starlight. Through the
model, Io round Jupiter (`HostTide`, $e$ = 0.0041) gets $8.5 \times
10^{13}$ W, 2.1 W/m², and loses it through a heat pipe.

### The ice shell and the ocean beneath

A body with a water layer whose surface is frozen (no liquid water on it,
a mean surface temperature under 273 K) carries its internal heat out
through an ice shell. The conductivity of ice rises as it gets colder,
$k = 651/T$ W m⁻¹ K⁻¹ (Klinger 1980), so Fourier's law integrates to a
shell of thickness

```math
d = \frac{651\,\ln(T_\mathrm{melt} / T_s)}{q}
```

with $T_s$ the surface temperature, $q$ the internal heat flux (radiogenic
and tidal) and $T_\mathrm{melt}$ = 273 K at the base (Hussmann, Sohl &
Spohn 2006). Below the shell the water stays liquid: an ocean, as thick as
the water layer less the shell. When the shell would be thicker than the
water layer, the whole layer is ice and there is no ocean.

**In words.** The more heat comes up from inside, the thinner the ice and
the deeper the ocean; a colder surface thickens the ice, but only through
the logarithm.

**Orders of magnitude.** Europa (8 % water, 36 mW/m² with its tides, 112 K)
gets a shell of 16 km over an ocean of about 100 km; Callisto, almost
without tides (3 mW/m²), a shell of 194 km.

### Classes

The class is read off the result, in this order; the first rule that holds
wins.

| Class | Rule | Source of the threshold |
| --- | --- | --- |
| Giant: gas giant | at least 10 Earth masses and 5 % gas; at least 50 % of the mass in gas | calibration (parts Jupiter and Saturn from Uranus and Neptune) |
| Giant: hot Jupiter | a gas giant with a period of 10 days or less | Wright et al. 2012 |
| Giant: hot Neptune | an ice giant with a period of 10 days or less (the sparse "Neptune desert") | Wright et al. 2012; Mazeh, Holczer & Faigler 2016, doi:10.1051/0004-6361/201528065 |
| Giant: ice giant | a giant with less than 50 % gas | same calibration |
| Mini-Neptune | at least 0.1 % gas and 1.6 $R_\oplus$ or more | Rogers 2015; the gas share is a calibration |
| Ice world | at least 5 % water and ices | calibration (Europa about 8 %, Earth 0.02 %) |
| Super-Earth | 1.25 $R_\oplus$ or more | Borucki et al. 2011 |
| Rocky | the rest | |

## In the app

### What you see

Nothing yet. Part 1 computes the model for the planets of a stored system
(`planet::body_model`) and the system's age is now drawn, but no card shows
the class, the layers or the heat, and no world changes: the
`body_type` that the user asks for still decides what is generated. Later
parts make the model decide.

### How it computes them

`planet::draw_inputs` draws the iron share, the water share, the envelope,
the volatiles and the radiogenic abundance from the formation context (the
star's metallicity, the orbit over the ice line) and the mass.
`planet::interior` gives the layers and the radius, `planet::heat` the heat
and `planet::classify` the class; `planet::body_model` chains them for one
planet. Each planet's inputs come from their own random stream, seeded by
the system's seed and `<id>/model`, so they depend on no other body.

Water is drawn as a share of the mass that is not gas, then scaled by one
minus the envelope, so that a wet giant keeps its rock.

Sources: [mod.rs](../../crates/mapgen/src/planet/mod.rs),
[inputs.rs](../../crates/mapgen/src/planet/inputs.rs),
[interior.rs](../../crates/mapgen/src/planet/interior.rs),
[heat.rs](../../crates/mapgen/src/planet/heat.rs),
[classify.rs](../../crates/mapgen/src/planet/classify.rs).

### Measuring it

```bash
cargo run --release -p mapgen --example planet_census -- 500
```

draws 500 whole systems (seeds 1 to 500, nothing written) and prints the
base preset each world renders with (`planet::interim::base_of`; the
`body_type` is the class itself), the share of planets per
radius bin of Fulton et al. (2017), and the median age, iron share, water
and envelope per class
([planet_census.rs](../../crates/mapgen/examples/planet_census.rs)).

### From the model to the map

A drawn planet's world is generated from its model, not from a type name
(`planet::interim`, a transitional layer that issue #424 replaces with
archetypes):

- `base_of` picks the built-in preset whose rules, palette and remap
  render the model, the first rule that holds: a giant class renders as
  `gas-giant`, `hot-jupiter` or `ice-giant` (a hot Neptune as a hot
  Jupiter, its methane gone; a mini-Neptune as an ice giant); a lava world or a heat-pipe regime as `volcanic`; a runaway
  greenhouse as `caustic`; a body whose envelope hides its surface as
  `frozen` for an ice world, `barren` otherwise; liquid hydrocarbons as
  `frozen`; liquid water
  as `terran`; no liquid, under 273 K and at least 5 % water, or at
  least 0.01 Earth oceans kept at the surface, as `frozen`; otherwise
  `barren`. The thresholds are the ones of the classes, so the look
  follows the class (Earth is `terran`, Venus `caustic`, Mars and
  Mercury `barren`, Titan `frozen`), except for a cold rock whose bulk is
  dry but whose surface keeps water: its sea level is calibrated on that
  water, which the barren look would draw as rock, so it renders
  `frozen`, a frozen sea in its basins (the 0.01 oceans: a little over
  twice Mars's 0.004, its polar deposits and ground ice of about 34 m of
  global layer, Carr & Head 2015, doi:10.1002/2014GL062464).
- A body that is no giant whose envelope hides its surface (the model
  derives none under it; about 0.3 % of drawn planets) keeps its
  class's preset at the model's radius and gravity, with no ocean, and
  its world aims at a bare surface's temperature, its equilibrium plus
  its internal heat (`calibrate_enveloped`). Part 3b of #423 derives the
  surface under the envelope.
- The gravity and the tectonic regime are the model's, and the radius is
  the interior's. The world is resolved at that gravity, so the star's
  tides, the moons it draws, the tilt they leave it and its tidal flats
  read the model's mass, not the one the rocky relation gives its size. The relief parameters follow: volcanism from the heat
  flux (Earth's 0.087 W/m2 gives the preset's value), halved when
  stagnant, full under a heat pipe; the age of the surface from the
  regime.
- The grid offset: the temperature shift of the world is found when the
  world is generated, so that the grid's mean matches the model's surface
  temperature (`generation::warming_for`). The sea level is found the
  same way, where the oceans the model keeps fill the relief
  (`sea_level_for_water`), none for a runaway whose water is steam. A
  world with no water at its surface is dry: no open water anywhere, its
  sea level only the datum of its relief (`DRY_SEA_LEVEL`, see
  [mountains and gravity](mountains-and-gravity.md)). Both
  pass over a coarse grid of the relief (about a second in release), so a
  system's draw leaves them to the generation of the world: the
  parameters stored with the body keep the drawn values, and the
  calibrated ones are in the world's `world.json`.
- Calibrations, no source: the share of water that makes a body frozen
  (5 %) and the water at its surface that does (0.01 oceans), the
  hydrocarbon world's land share (Titan's 99 %), the erosion of methane
  (half of water's).

### Where it departs from the science

- **Calibrations.** Every constant marked "Calibration, no source" in the
  code is a choice, not a measurement: the shares of the iron core and of
  ice, the chance of keeping gas, the envelope share per mass, the iron
  and water densities, the $k^{1.5}$ compression of water, and the class
  thresholds that the table marks. Each names the bodies it reproduces.
- **Uranus is 11.6 % too small.** The envelope of a giant uses the
  generator's existing giant radius, which gives Uranus 3.52 $R_\oplus$
  against 3.98 observed. A test for it is ignored until the radius is
  fixed (issue #432).
- **Mini-Neptunes and the radius valley.** Photoevaporation now strips
  close-in envelopes (see [planet-atmospheres.md](planet-atmospheres.md)),
  and mini-Neptunes fall to 3.6 % of the planets. The census still shows
  no dip between 1.5 and 2 $R_\oplus$ (Fulton et al. 2017), because the
  generator draws few planets that large; the doc reports it as measured.
- **Envelope grid.** The envelope fit stops at 20 % of the mass: a small
  planet with a 30 % envelope (the model allows up to 30 %) is sized as if
  it had 20 %.
- **Moons have no model** yet: the host tide and the ice shell are in
  the model, but no drawn moon goes through it.
- **The ice shell** is conductive and pure: no convection in its warm
  lower part, no ammonia or salt to lower the melting point (273 K is a
  calibration), and no high-pressure ice at the bottom of a thick water
  layer, so Ganymede's and Callisto's oceans come out as the whole water
  layer under the shell (hundreds of km) where interior models put about
  100 km at most, Callisto's thin or absent.
- **Tides** use one $k_2/Q$ per kind of body, not one per planet: a cold
  rocky planet takes Io's, which is partly molten (Mars's is about
  $1.7 \times 10^{-3}$, Lainey 2016), and every enveloped planet takes
  Jupiter's, from a mini-Neptune to a hot Jupiter. They neglect the
  planet's own mass.

## Sources

1. Adibekyan, V. et al. 2021, "A compositional link between rocky exoplanets
   and their host stars", *Science* 374, 330–332,
   [doi:10.1126/science.abg8794](https://doi.org/10.1126/science.abg8794).
2. Arevalo, R., McDonough, W. F. & Luong, M. 2009, "The K/U ratio of the
   silicate Earth", *Earth and Planetary Science Letters* 278, 361–369,
   [doi:10.1016/j.epsl.2008.12.023](https://doi.org/10.1016/j.epsl.2008.12.023).
3. Borucki, W. J. et al. 2011, *The Astrophysical Journal* 736, 19,
   [doi:10.1088/0004-637X/736/1/19](https://doi.org/10.1088/0004-637X/736/1/19).
4. Chen, J. & Kipping, D. 2017, "Probabilistic forecasting of the masses and
   radii of other worlds", *The Astrophysical Journal* 834, 17,
   [doi:10.3847/1538-4357/834/1/17](https://doi.org/10.3847/1538-4357/834/1/17).
5. Fulton, B. J. et al. 2017, "The California-Kepler Survey. III. A gap in
   the radius distribution of small planets", *The Astronomical Journal*
   154, 109,
   [doi:10.3847/1538-3881/aa80eb](https://doi.org/10.3847/1538-3881/aa80eb).
6. Guillot, T. 2005, "The interiors of giant planets: models and
   outstanding questions", *Annual Review of Earth and Planetary
   Sciences* 33, 493–530,
   [doi:10.1146/annurev.earth.33.092203.122639](https://doi.org/10.1146/annurev.earth.33.092203.122639).
7. Helled, R. et al. 2011, *The Astrophysical Journal* 726, 15,
   [doi:10.1088/0004-637X/726/1/15](https://doi.org/10.1088/0004-637X/726/1/15).
8. Howett, C. J. A., Spencer, J. R., Pearl, J. & Segura, M. 2011, "High
   heat flow from Enceladus' south polar region measured using 10–600
   cm⁻¹ Cassini/CIRS data", *Journal of Geophysical Research* 116, E03003,
   [doi:10.1029/2010JE003718](https://doi.org/10.1029/2010JE003718).
9. Hussmann, H., Sohl, F. & Spohn, T. 2006, "Subsurface oceans and deep
   interiors of medium-sized outer planet satellites and large
   trans-neptunian objects", *Icarus* 185, 258–273,
   [doi:10.1016/j.icarus.2006.06.005](https://doi.org/10.1016/j.icarus.2006.06.005).
10. Jackson, B., Greenberg, R. & Barnes, R. 2008, "Tidal evolution of
    close-in extrasolar planets", *The Astrophysical Journal* 678,
    1396–1406, [doi:10.1086/529187](https://doi.org/10.1086/529187).
11. Kimura, J. 2024, "Europa's structural conditions for the existence of
    subsurface ocean and the absence of metallic core-driven magnetic
    field", arXiv:2403.01336,
    [arxiv.org/abs/2403.01336](https://arxiv.org/abs/2403.01336).
12. Klinger, J. 1980, "Influence of a phase transition of ice on the heat
    and mass balance of comets", *Science* 209, 271–272,
    [doi:10.1126/science.209.4453.271](https://doi.org/10.1126/science.209.4453.271).
13. Lainey, V., Arlot, J.-E., Karatekin, Ö. & Van Hoolst, T. 2009, "Strong
    tidal dissipation in Io and Jupiter from astrometric observations",
    *Nature* 459, 957–959,
    [doi:10.1038/nature08108](https://doi.org/10.1038/nature08108).
14. Lainey, V. 2016, "Quantification of tidal parameters from Solar System
    data", *Celestial Mechanics and Dynamical Astronomy* 126, 145–156
    (table 1),
    [doi:10.1007/s10569-016-9695-y](https://doi.org/10.1007/s10569-016-9695-y).
15. Lainey, V. et al. 2012, "Strong tidal dissipation in Saturn and
    constraints on Enceladus' thermal state from astrometry", *The
    Astrophysical Journal* 752, 14,
    [doi:10.1088/0004-637X/752/1/14](https://doi.org/10.1088/0004-637X/752/1/14).
16. Lee, E. J. & Chiang, E. 2016, *The Astrophysical Journal* 817, 90,
    [doi:10.3847/0004-637X/817/2/90](https://doi.org/10.3847/0004-637X/817/2/90).
17. Lopez, E. D. & Fortney, J. J. 2014, "Understanding the mass-radius
    relation for sub-Neptunes: radius as a proxy for composition", *The
    Astrophysical Journal* 792, 1,
    [doi:10.1088/0004-637X/792/1/1](https://doi.org/10.1088/0004-637X/792/1/1).
18. Marty, B. 2012, "The origins and concentrations of water, carbon,
    nitrogen and noble gases on Earth", *Earth and Planetary Science
    Letters* 313–314, 56–66,
    [doi:10.1016/j.epsl.2011.10.040](https://doi.org/10.1016/j.epsl.2011.10.040).
19. McDonough, W. F. & Sun, S.-s. 1995, "The composition of the Earth",
    *Chemical Geology* 120, 223–253,
    [doi:10.1016/0009-2541(94)00140-4](https://doi.org/10.1016/0009-2541(94)00140-4).
20. Peale, S. J., Cassen, P. & Reynolds, R. T. 1979, "Melting of Io by tidal
    dissipation", *Science* 203, 892–894,
    [doi:10.1126/science.203.4383.892](https://doi.org/10.1126/science.203.4383.892).
21. Pollack, J. B. et al. 1996, "Formation of the giant planets by
    concurrent accretion of solids and gas", *Icarus* 124, 62–85,
    [doi:10.1006/icar.1996.0190](https://doi.org/10.1006/icar.1996.0190).
22. Rogers, L. A. 2015, "Most 1.6 Earth-radius planets are not rocky", *The
    Astrophysical Journal* 801, 41,
    [doi:10.1088/0004-637X/801/1/41](https://doi.org/10.1088/0004-637X/801/1/41).
23. Thorngren, D. & Fortney, J. J. 2018, "Bayesian analysis of hot-Jupiter
    radius anomalies", *The Astronomical Journal* 155, 214,
    [doi:10.3847/1538-3881/aaba13](https://doi.org/10.3847/1538-3881/aaba13).
24. Turcotte, D. L. & Schubert, G. 2014, *Geodynamics*, 3rd ed., Cambridge
    University Press (table 4.2).
25. Wahr, J. M., Zuber, M. T., Smith, D. E. & Lunine, J. I. 2006, "Tides on
    Europa, and the thickness of Europa's icy shell", *Journal of
    Geophysical Research* 111, E12005,
    [doi:10.1029/2006JE002729](https://doi.org/10.1029/2006JE002729).
26. Wright, J. T. et al. 2012, *The Astrophysical Journal* 753, 160,
    [doi:10.1088/0004-637X/753/2/160](https://doi.org/10.1088/0004-637X/753/2/160).
27. Zeng, L., Sasselov, D. D. & Jacobsen, S. B. 2016, "Mass-radius
    relation for rocky planets based on PREM", *The Astrophysical Journal*
    819, 127,
    [doi:10.3847/0004-637X/819/2/127](https://doi.org/10.3847/0004-637X/819/2/127).
28. Zeng, L. et al. 2019, "Growth model interpretation of planet size
    distribution", *Proceedings of the National Academy of Sciences* 116,
    9723–9728,
    [doi:10.1073/pnas.1812905116](https://doi.org/10.1073/pnas.1812905116).

The three DOIs above were checked against the publishers' pages; the
DOIs of the tidal sources (Howett, Jackson, Kimura, Lainey, Wahr) and of
the ice shell's (Hussmann, Klinger) against their Crossref records; the
$651/T$ conductivity is quoted as the ice-shell literature cites Klinger,
not checked in the paper itself. Of the tidal figures, Jupiter's and Io's $k_2/Q$,
Saturn's $1.59 \times 10^{-4}$ and Mars's $k_2$ and $Q$ match table 1 of
Lainey 2016; Saturn's $2.3 \times 10^{-4}$, the planetary $Q$ of
$10^{6.5}$ of Jackson et al. and Kimura's 10 to 50 mW/m² match their
abstracts; Howett's 15.8 GW and Wahr's $k_2$ of 0.25 were not checked in
the papers themselves. Of the
figures quoted from papers, Marty's 1000 to 3000 ppm of water and up to
500 ppm of carbon match the abstract; his 2 ppm of nitrogen (the abstract
only says nitrogen is depleted by about an order of magnitude) and the
"5 of 22 rocky planets" of Adibekyan et al. 2021 (22 comes from press
reports of the paper) were not checked in the papers themselves.

## See also

- [What is the habitable zone?](habitable-zone.md): the zone whose planets
  this model sizes.
- [Why does a world always show the same face to its
  host?](tidal-locking.md): the system's age, now drawn, sets how far
  locking has gone.
- [docs/superpowers/specs/2026-11-11-physical-planet-model-design.md](../superpowers/specs/2026-11-11-physical-planet-model-design.md):
  the design of the model.

For the atmosphere, climate and XUV history of a planet see
[planet-atmospheres.md](planet-atmospheres.md).
