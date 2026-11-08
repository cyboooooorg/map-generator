# How does axial tilt make climate zones and seasons?

Seen from: the planet view, through the **Temperature** and **Biomes**
layers, the **Axial tilt** line of the world card (and "tilt steadied by
its moon" or "tilt left free" beside the tides), and the **Season** slider
of the **Light and season** panel.

## Short answer

A planet's tilt decides how its star's light is shared between latitudes
over a year. With no tilt the sun always stands over the equator: the
equator is hot, the poles get almost nothing, and there are no seasons.
Tilt the axis and each pole spends half the year facing the star, so the
poles get more light over a year and the equator a little less. Past a
tilt of about 54°, the poles get more light over a year than the equator.
A large moon keeps the tilt steady; without one it can wander over tens of
degrees. An eccentric orbit adds a little light over the year, and uneven
seasons.

## The science

![Annual-mean sunlight against latitude for tilts of 0, 23.4, 45 and 90
degrees: the equator gets less and the poles more as the tilt
grows](images/axial-tilt-climate-insolation.svg)

### Sunlight on a spinning, tilted planet

Take a planet that spins fast compared with its year. At latitude $\varphi$,
on a day when the star stands over latitude $\delta$ (the star's
*declination*), the sunlight averaged over the whole day, at the top of the
atmosphere, is (Nadeau & McGehee 2021, eq. 45–47, after Sellers 1965):

```math
Q_\text{day} = \frac{S_0}{\pi}\left(\frac{a}{r}\right)^2
\left(h_0\sin\varphi\sin\delta + \cos\varphi\cos\delta\sin h_0\right),
\qquad
\cos h_0 = -\tan\varphi\tan\delta
```

$S_0$ is the light the planet receives at its mean distance $a$ from the
star, $r$ its distance that day, and $h_0$ the half-length of the day, as an
angle from noon to sunset (0 in polar night, $\pi$ in polar day). The
declination follows the tilt $\varepsilon$ (the obliquity) and the planet's
place on its orbit, its angle $\lambda$ from the spring equinox:

```math
\sin\delta = \sin\varepsilon\,\sin\lambda
```

**In words.** A latitude gets more light when the star is higher in its sky
and when its day is longer. Through the year the star climbs to the tilt
angle north of the equator, then to the same angle south: that swing is
the seasons. With no tilt it never moves and every day is the same.

**Orders of magnitude.** With the Sun's 1361 W/m² at Earth (NASA Earth fact
sheet) and a circular orbit, the formula gives the equator 433 W/m² at the
equinoxes. At the June solstice the North Pole, lit around the clock, gets
541 W/m² over the day, more than the equator's 397: on that day the pole
is the sunniest place on Earth.

### Tropics and polar circles

Two lines follow from these equations. The star passes overhead at noon
where $\varphi = \delta$, so only between the latitudes $\pm\varepsilon$:
the *tropics*. And $\cos h_0$ reaches $-1$ (the sun never sets) or $+1$
(it never rises) when $\tan\varphi\tan\delta = \pm1$, which happens at some
point in the year poleward of $90^\circ - \varepsilon$: the *polar
circles*.

**In words.** The tilt angle is both how far the tropics reach from the
equator and how far the polar circles reach from the poles.

**Orders of magnitude.** Earth's tilt of 23.44° (NASA Earth fact sheet)
puts the tropics at 23.44° and the polar circles at 66.56°. Past a tilt of
45° the two overlap: some latitudes have both an overhead sun and a
midnight sun in the same year. Uranus, tilted 97.8° (NASA planetary fact
sheet), has them almost everywhere.

### Over a year: the annual mean by latitude

Averaging the daily light over a whole orbit gives the light each latitude
receives over a year. With $y = \sin\varphi$ and $e$ the orbit's
eccentricity, it splits into an amount and a shape (Nadeau & McGehee 2017,
eq. 1–3):

```math
I(y) = \frac{Q_0}{\sqrt{1-e^2}}\; s(y, \varepsilon),
\qquad
s \approx 1 - \tfrac{5}{8}P_2(\cos\varepsilon)P_2(y)
- \tfrac{9}{64}P_4(\cos\varepsilon)P_4(y)
- \tfrac{65}{1024}P_6(\cos\varepsilon)P_6(y)
```

$Q_0 = S_0/4$ is the global mean over a circular orbit, and the $P_n$ are
Legendre polynomials, with $P_2(x) = (3x^2-1)/2$. The shape $s$ averages to
1 over the sphere. Kept to its first two terms, $1 - \tfrac58
P_2(\cos\varepsilon)P_2(y)$, it is the classic approximation of energy
balance climate models. North (1975) used $1 - 0.482\,P_2(y)$ for Earth,
which he gave as accurate to within 2 % (as quoted by Nadeau & McGehee
2017).

**In words.** The tilt does not change how much light the planet gets in
total, only where it lands. A small tilt piles it on the equator; a large
one spreads it towards the poles. Two terms are enough at Earth's tilt, but
not at others: Nadeau & McGehee (2017) need six to catch the slight "W"
shape of Pluto's curve.

**Orders of magnitude.** From the exact average, plotted above:

| Tilt | Equator (W/m²) | Pole (W/m²) | Pole / equator |
| --- | --- | --- | --- |
| 0° | 433 | 0 | 0 |
| 23.4° (Earth) | 416 | 172 | 0.41 |
| 45° | 373 | 306 | 0.82 |
| 54° | 350 | 351 | 1.00 |
| 90° | 276 | 433 | 1.57 |

The pole catches up with the equator at a tilt of 53.9°. Nadeau & McGehee
(2017) show 54° as a case "where polar and equatorial regions receive
about the same yearly insolation". The global mean stays 340 W/m²
whatever the tilt.

### An eccentric orbit

The factor $1/\sqrt{1-e^2}$ above is the extra light an elliptical orbit
collects over a year, compared with a circle of the same semi-major axis.
It does not change the shape $s$ (Nadeau & McGehee 2017, eq. 1–2).

**In words.** The planet spends longer far from its star than close to
it, but the close passage more than makes up for it. Over a year the
balance is a small gain, spread like the light of a circular orbit.
Williams & Pollard (2002), with climate models of Earths on very eccentric
orbits, found that "long-term climate stability depends primarily on the
average stellar flux received over an entire orbit".

**Orders of magnitude.** The gain is 0.01 % for Earth ($e = 0.0167$),
0.4 % for Mars ($e = 0.094$), 2.2 % for Mercury ($e = 0.206$) and 25 % at
$e = 0.6$ (eccentricities from the NASA fact sheets). The seasons,
though, become uneven: Earth gets 7 % more light at its closest than at
its farthest, Mars 46 %.

### A large moon steadies the tilt

The pull of the star and of the moons on a planet's equatorial bulge makes
its axis precess, like a spinning top. The other planets slowly rock the
orbit. When those two rhythms fall into step, the tilt can drift
chaotically (Laskar & Robutel 1993).

Laskar, Joutel & Robutel (1993) integrated the Earth's spin over 18 million
years for every starting tilt. With the Moon, Earth's tilt is "essentially
stable, exhibiting only small variations of ±1.3° around the mean value of
23.3°". Without it, "the chaotic zone would then extend from nearly 0° up
to about 85°". Mars, with no large moon, is in that state now: its tilt is
"in a large chaotic region, ranging from 0° to 60°" (Laskar & Robutel
1993).

**In words.** The Moon adds its torque to the Sun's and keeps Earth's axis
precessing fast enough to stay clear of the slow rocking of the orbit.

**Orders of magnitude.** The Moon has about 1.2 % of Earth's mass, and Mars's
tilt is 25.2° today (NASA planetary fact sheet), close to Earth's, but over
millions of years it can swing from almost upright to 60°, which moves its
whole climate from the left of the chart above to the right.

### What we do not know

- **Whether a moonless Earth would wander that far.** Lissauer, Barnes &
  Chambers (2012) integrated moonless Earths for up to 4 billion years and
  found that over hundreds of millions of years "the obliquity remains
  within a constrained range, typically 20–25° in extent". They conclude
  that "a large moon thus does not seem to be needed to stabilize the
  obliquity" on the timescales of advanced life. Li & Batygin (2014) find
  the chaotic drift of a moonless Earth "sufficiently slow to not preclude
  long-term habitability". The 0 to 85° of Laskar et al. (1993) is the
  range the tilt *could* reach, not how fast it gets there.
- **From light to temperature.** The equations above give the light at the
  top of the atmosphere. How warm each latitude gets also depends on the
  heat that air and oceans carry towards the poles, and on ice and clouds:
  that takes a climate model, not a formula.

## In the app

### What you see

- **The tilt.** The creation wizard has an **Axial tilt (°)** field (help:
  "Earth: 23.4°"), from 0 to 90°. Left empty, the tilt is drawn: from 5 to
  40° with a large moon, from 0 to 85° without one. The world card in the
  **Info** panel shows it on its **Axial tilt** line, and after the tides
  "tilt steadied by its moon" or "tilt left free".
- **Climate zones.** The **Temperature** layer (−30 °C to 40 °C) shows the
  annual mean. The **Biomes** layer shows the zones: ice caps, then tundra
  and taiga, temperate land, tropical land towards the equator. The biomes
  read the seasons as well as the mean: ice needs a summer below freezing,
  and land with a cold winter is never tropical. A world tilted past about
  54° has warm-annual poles, but their winters freeze, so they read as
  temperate land, not jungle.
- **The Season slider moves only the light.** The **Season** slider of the
  **Light and season** panel ("Sun declination follows the axial tilt") and
  **Play** (`S`) move the sun between the tropics with **Sunlight** on, and
  say where it stands ("Summer solstice · sun over 23.4° N"). The
  Temperature layer stays the annual mean whatever the season. On a world
  alone, the sun's latitude follows $\sin\delta = \sin\varepsilon\sin\lambda$,
  as in a system.

### How it computes them

1. **The tilt.** A tilt asked for is kept. A drawn one comes from 5 to 40°.
   If no moon has at least 0.005 of the planet's mass (the Moon: 0.0123),
   the draw is stretched to 0 to 85°, the chaotic zone of a moonless Earth
   (Laskar, Joutel & Robutel 1993). A moon of a giant keeps the 5 to 40°
   range: its tilt follows the giant's. A drawn tilt is 0 for a planet whose
   star's tides have despun it (locked, or in the 3:2 resonance).
2. **The light of each row of the map**, relative to the global mean: the
   exact annual mean of the daily formula above (eq. 45–47), averaged over
   360 steps of the orbit, with $\sin\delta = \sin\varepsilon\sin\lambda$.
3. **Into a temperature.** The light is rescaled so that Earth's tilt gives
   0 at the poles and 1 at the equator. A curve fitted to Earth's annual
   zonal mean turns it into degrees: $T = -18 + 56.28\,t - 11.28\,t^2$,
   which gives −18 °C at the poles, 12 °C at 42°, 27 °C at the equator.
4. **The same global mean at every tilt.** The curve bends, so the rows
   are then shifted to keep the global mean of Earth's tilt (14.5 °C),
   averaged by area. The tilt moves the light between latitudes without
   changing its global mean. In an energy balance model whose outgoing
   radiation is linear in temperature, the mean temperature depends only on
   the mean absorbed light (Budyko 1969; North et al. 1981).
5. **Atmosphere and star.** Dense air evens the rows out. The star then
   moves them by the light the planet gets relative to Earth's, including
   the factor $1/\sqrt{1-e^2}$ of an eccentric orbit (see the README,
   "Temperature — axial tilt").
6. **Seasons.** Each row gets a seasonal half-range, on land and on the sea:
   half the difference between its sunniest and darkest day, in the same
   units as step 3, times 45 °C per unit (the mean slope of the curve), times
   a damping for thermal inertia (0.45 on land, 0.10 on the sea), times the
   atmosphere's heat contrast, then moved by the star. Summer is the mean
   plus the half-range, winter the mean minus it. A land pixel near the sea
   swings less: its range runs from the sea's at the coast to the interior's
   over about 400 km. A planet locked to its star has no seasons.
7. **Each pixel** cools with altitude on land and gets a large-scale
   anomaly standing for ocean currents and continents. The lapse rate is
   6.5 °C per km at 1 bar or more (the standard atmosphere, NASA Glenn),
   and less in thinner air, down to none on an airless body.
8. **Biomes.** Land is ice cap when its summer is below 0 °C (Köppen's EF,
   warmest month below 0 °C), tundra when its summer is below 10 °C and
   its mean below the boreal limit (ET). Land whose winter is below 18 °C is
   not tropical, however warm its mean is: Köppen's tropical climates A
   need a coldest month of at least 18 °C. Köppen limits are those of Kottek
   et al. (2006), Table 1.

The core, trimmed (from `crates/mapgen/src/climate.rs`):

```rust
pub const EARTH_AXIAL_TILT_DEG: f32 = 23.44;

pub fn sea_level_celsius(t: f32) -> f32 {
    -18.0 + 56.28 * t - 11.28 * t * t
}

pub fn annual_insolation(latitude: f64, axial_tilt_deg: f32) -> f64 {
    (0..ORBIT_STEPS)
        .map(|k| {
            let longitude = (k as f64 + 0.5) / ORBIT_STEPS as f64 * TAU;
            daily_insolation(latitude, declination(axial_tilt_deg, longitude))
        })
        .sum::<f64>()
        / ORBIT_STEPS as f64
}

pub fn row_temperatures(grid: &Grid, axial_tilt_deg: f32) -> Vec<f32> {
    // Same global mean as Earth's tilt, whatever the tilt.
    let shift = mean_latitude_temperature(EARTH_AXIAL_TILT_DEG)
        - mean_latitude_temperature(axial_tilt_deg);
    (0..grid.height)
        .map(|y| {
            latitude_temperature(grid.latitude(y), axial_tilt_deg) + shift
        })
        .collect()
}

// In Seasons::new: the swing of the daily light, in degrees, then
// summer and winter rows for each damping.
let s = insolation_swing(grid.latitude(y), axial_tilt_deg)
    / (equator - pole);
s as f32 * DEGREES_PER_UNIT * contrast
// land: half_range(LAND_SEASONS), sea: half_range(SEA_SEASONS)
```

The sea-level temperatures this gives, before the atmosphere and the star
act on them (the shift that keeps the global mean is included):

| Tilt | Equator | 45° | Pole |
| --- | --- | --- | --- |
| 0° | 30 °C | 10 °C | −48 °C |
| 23.44° (Earth) | 27 °C | 10 °C | −18 °C |
| 45° | 20 °C | 11 °C | 9 °C |
| 54.7° | 16 °C | 13 °C | 17 °C |
| 90° | 3 °C | 20 °C | 29 °C |

The land's seasonal half-range, before the star, at Earth's air pressure:

| Tilt | 30° | 60° | Pole |
| --- | --- | --- | --- |
| 23.44° (Earth) | 11 °C | 20 °C | 22 °C |
| 45° | 20 °C | 35 °C | 40 °C |
| 90° | 28 °C | 49 °C | 57 °C |

The sea swings about a fifth as much as the land (0.10 against 0.45). At
90° the land pole swings by ±57 °C around its mean of 29 °C, from a
scorching summer to a winter far below freezing.

The tilt is resolved with the moons in
[`crates/mapgen/src/request.rs`](../../crates/mapgen/src/request.rs) and
[`crates/mapgen/src/moons.rs`](../../crates/mapgen/src/moons.rs)
(`PlanetEffects::tilt`). The rows and the seasons are built in
[`crates/mapgen/src/climate.rs`](../../crates/mapgen/src/climate.rs)
(`seasons` in
[`crates/mapgen/src/generation.rs`](../../crates/mapgen/src/generation.rs)),
and the star and the eccentric orbit act on them in
[`crates/mapgen/src/star.rs`](../../crates/mapgen/src/star.rs)
(`star_temperatures`, `mean_flux_factor`). Ice, tundra and the tropics are
chosen in
[`crates/mapgen/src/biome.rs`](../../crates/mapgen/src/biome.rs)
(`BiomeRules`).

### Where it departs from the science

- **Temperature from light by an Earth fit.** One curve, fitted to Earth
  (calibration, no source), turns light into temperature for every tilt.
  The global mean is held at Earth's 14.5 °C by a shift, which an energy
  balance model with linear outgoing radiation justifies. Real planets
  differ: ice and clouds, which would change with the tilt, are not
  modelled, and neither is the heat carried to the poles.
- **The seasons are a calibration.** The damping factors (0.45 on land,
  0.10 on the sea) and the 400 km that a coast's mild air reaches inland
  have no source. They were chosen so that continental interiors swing by
  about ±10 °C at 30° and ±20 °C at 60°, the order of Phoenix's and
  Fairbanks's ranges (not checked against station normals). A
  mixed-layer sea 50 m deep, with a restoring rate of 5.4 W/m²/K (the
  slope of the app's curve), would give 0.13 for the sea by the response
  of a heat capacity to a yearly swing, $1/\sqrt{1+(\omega C/\lambda)^2}$;
  that sum is the app's, not North et al.'s, who take a mixed layer of
  75 m (which would give 0.09 here) and a restoring rate of 2.09 W/m²/K.
- **Only ice, tundra and the tropics read the seasons.** The Temperature
  layer and the other biome rules use the annual mean. The app has no
  monthly temperatures: summer and winter are the mean plus or minus the
  half-range.
- **Land with a cold winter is called temperate.** Köppen puts land with a
  coldest month below 18 °C in the warm temperate (C), cold (D) or dry (B)
  climates. The app turns it into temperate forest or plain, or arid land
  where it is dry.
- **No uneven seasons on eccentric orbits.** The app adds the
  $1/\sqrt{1-e^2}$ factor, which is right for annual means, but not the
  uneven seasons it brings (46 % more light at perihelion on Mars). They
  need a longitude of perihelion, which the parameters do not have
  (`docs/backlog.md`).
- **A steady tilt, set once.** Each world has one tilt for its whole life.
  Real tilts wander (by ±1.3° for Earth, by tens of degrees for Mars), and
  the ice ages follow such swings.
- **The moon rule is a threshold.** A moon of 0.005 of the planet's mass or
  more steadies the tilt, a lighter one does not. That cut is the app's
  choice, between the Moon and a moon too small to matter; the studies
  above computed only Earth with and without its Moon. Without a large
  moon the tilt is drawn from 0 to 85°, the chaotic zone of a moonless
  Earth (Laskar et al. 1993), while Lissauer et al. (2012) found a much
  narrower range over long times.
- **No tilt above 90°.** The app takes tilts from 0 to 90°. A planet that
  spins backwards, like Venus (177.4°) or Uranus (97.8°), can only be
  entered as its mirror image (2.6°, 82.2°). For the annual light and the
  seasonal swing this changes nothing, since both depend on the tilt only
  through the star's declination, which is symmetric.
- **Tilt 90° pushes the model.** The wind belts, the rain bands and the
  rain-shadow rules follow geographic latitude, while the warm poles
  evaporate most (`docs/backlog.md`).

## Sources

1. Nadeau, A. & McGehee, R. 2017, "A simple formula for a planet's mean
   annual insolation by latitude", *Icarus* 291, 46–50,
   [doi:10.1016/j.icarus.2017.01.040](https://doi.org/10.1016/j.icarus.2017.01.040),
   preprint [arXiv:1510.04542](https://arxiv.org/abs/1510.04542). The
   annual mean by latitude and obliquity (eq. 1–3), the eccentricity
   factor, North's approximation, the 54° case.
2. Nadeau, A. & McGehee, R. 2021, "Approximating annual mean incoming solar
   radiation", *Journal of Mathematical Analysis and Applications* 500,
   125129,
   [doi:10.1016/j.jmaa.2021.125129](https://doi.org/10.1016/j.jmaa.2021.125129),
   preprint [arXiv:1810.10081](https://arxiv.org/abs/1810.10081). The
   daily-mean insolation formula (eq. 45–47).
3. Sellers, W. D. 1965, *Physical Climatology*, University of Chicago
   Press. The daily-mean formula, as reproduced in source 2.
4. North, G. R. 1975, "Theory of energy-balance climate models", *Journal
   of the Atmospheric Sciences* 32, 2033–2043,
   [doi:10.1175/1520-0469(1975)032<2033:TOEBCM>2.0.CO;2](https://doi.org/10.1175/1520-0469(1975)032%3C2033:TOEBCM%3E2.0.CO;2).
5. Laskar, J., Joutel, F. & Robutel, P. 1993, "Stabilization of the Earth's
   obliquity by the Moon", *Nature* 361, 615–617,
   [doi:10.1038/361615a0](https://doi.org/10.1038/361615a0).
6. Laskar, J. & Robutel, P. 1993, "The chaotic obliquity of the planets",
   *Nature* 361, 608–612,
   [doi:10.1038/361608a0](https://doi.org/10.1038/361608a0).
7. Lissauer, J. J., Barnes, J. W. & Chambers, J. E. 2012, "Obliquity
   variations of a moonless Earth", *Icarus* 217, 77–87,
   [doi:10.1016/j.icarus.2011.10.013](https://doi.org/10.1016/j.icarus.2011.10.013).
8. Li, G. & Batygin, K. 2014, "On the spin-axis dynamics of a moonless
   Earth", *The Astrophysical Journal* 790, 69,
   [doi:10.1088/0004-637X/790/1/69](https://doi.org/10.1088/0004-637X/790/1/69),
   preprint [arXiv:1404.7505](https://arxiv.org/abs/1404.7505).
9. Williams, D. M. & Pollard, D. 2002, "Earth-like worlds on eccentric
   orbits: excursions beyond the habitable zone", *International Journal
   of Astrobiology* 1, 61–69,
   [doi:10.1017/S1473550402001064](https://doi.org/10.1017/S1473550402001064).
10. NASA Space Science Data Coordinated Archive, [Earth fact
    sheet](https://nssdc.gsfc.nasa.gov/planetary/factsheet/earthfact.html)
    (solar irradiance 1361.0 W/m², obliquity 23.44°, eccentricity 0.0167)
    and [planetary fact
    sheet](https://nssdc.gsfc.nasa.gov/planetary/factsheet/) (obliquities
    and eccentricities of Mercury, Venus, Mars and Uranus; the masses of
    the Earth and the Moon).
11. Budyko, M. I. 1969, "The effect of solar radiation variations on the
    climate of the Earth", *Tellus* 21, 611–619,
    [doi:10.3402/tellusa.v21i5.10109](https://doi.org/10.3402/tellusa.v21i5.10109).
    The energy balance model with linear outgoing radiation.
12. North, G. R., Cahalan, R. F. & Coakley, J. A. 1981, "Energy balance
    climate models", *Reviews of Geophysics and Space Physics* 19, 91–121,
    [doi:10.1029/RG019i001p00091](https://doi.org/10.1029/RG019i001p00091).
    The mean temperature set by the mean absorbed light, and the thermal
    inertia of land and sea against a yearly swing.
13. Kottek, M., Grieser, J., Beck, C., Rudolf, B. & Rubel, F. 2006, "World
    map of the Köppen-Geiger climate classification updated",
    *Meteorologische Zeitschrift* 15, 259–263,
    [doi:10.1127/0941-2948/2006/0130](https://doi.org/10.1127/0941-2948/2006/0130).
    Table 1: the limits of the polar (EF: warmest month below 0 °C; ET:
    below 10 °C), tropical (A: coldest month at least 18 °C) and cold
    climates.

The insolation values in the tables and the chart, and the app's
temperatures, were computed for this article from sources 1 and 2 and
from the code.

## See also

- [README, "Temperature — axial tilt"](../../README.md#temperature--axial-tilt):
  the app's whole temperature chain.
- [docs/analysis/07-atmospheres.md](../analysis/07-atmospheres.md),
  "Climate effects": heat transport and greenhouse on top of the rows.
- [docs/analysis/10-stars.md](../analysis/10-stars.md), "Consequences for a
  planet" and "Eccentric orbits": the star's light and the orbit.
- [docs/analysis/11-moons.md](../analysis/11-moons.md), "Effects of a moon
  on its planet": the tilt steadied by a large moon.
- Related articles: [What does an eccentric orbit
  change?](eccentric-orbits.md), [Why does a world always show the same
  face to its host?](tidal-locking.md), [Why is one side of a mountain
  range dry?](rain-shadows.md).
