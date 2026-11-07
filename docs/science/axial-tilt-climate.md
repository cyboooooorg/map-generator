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
  "Earth: 23.4°"), from 0 to 90°. Left empty, the tilt is drawn. The world
  card in the **Info** panel shows it on its **Axial tilt** line, and after
  the tides "tilt steadied by its moon" or "tilt left free".
- **Climate zones.** The **Temperature** layer (−30 °C to 40 °C) shows the
  annual mean, and the **Biomes** layer its zones: ice caps, then tundra
  and taiga, temperate land, tropical land towards the equator. A world
  tilted past about 55° turns this upside down: warm poles and a cooler
  equator.
- **Seasons, in the light only.** The **Season** slider of the **Light and
  season** panel ("Sun declination follows the axial tilt") and **Play**
  (`S`) move the sun between the tropics with **Sunlight** on, and say where
  it stands ("Summer solstice · sun over 23.4° N"). The temperatures and
  biomes do not change with it.

### How it computes them

1. **The tilt.** A tilt asked for is kept. A drawn one comes from 5 to 40°.
   If no moon has at least 0.005 of the planet's mass (the Moon: 0.0123),
   the draw is stretched to 0 to 60°, the range Laskar & Robutel (1993)
   found for Mars. A moon of a giant keeps the 5 to 40° range: its tilt
   follows the giant's.
2. **The light of each row of the map**, relative to the global mean, from
   the two-term formula above.
3. **Into a temperature.** The light is rescaled so that Earth's tilt gives
   0 at the poles and 1 at the equator. A curve fitted to Earth's annual
   zonal mean turns it into degrees: −18 °C at the poles, 12 °C at 42°,
   27 °C at the equator.
4. **Atmosphere and star.** Dense air evens the rows out. The star then
   moves them by the light the planet gets relative to Earth's, including
   the factor $1/\sqrt{1-e^2}$ of an eccentric orbit (see the README,
   "Temperature — axial tilt").
5. **Each pixel** then cools with altitude on land and gets a large-scale
   anomaly standing for ocean currents and continents.

The core, trimmed (from `crates/mapgen/src/climate.rs`):

```rust
pub const EARTH_AXIAL_TILT_DEG: f32 = 23.44;

pub fn celsius(t: f32) -> f32 {
    -18.0 + 66.2 * t - 21.2 * t * t
}

fn insolation(latitude: f64, axial_tilt_deg: f32) -> f64 {
    let s2 = -5.0 / 8.0 * p2((axial_tilt_deg as f64).to_radians().cos());
    1.0 + s2 * p2(latitude.sin())
}

pub fn row_temperatures(grid: &Grid, axial_tilt_deg: f32) -> Vec<f32> {
    // Earth's tilt sets the scale: 0 at its poles, 1 at its equator.
    let pole = insolation(FRAC_PI_2, EARTH_AXIAL_TILT_DEG);
    let equator = insolation(0.0, EARTH_AXIAL_TILT_DEG);
    (0..grid.height)
        .map(|y| {
            let s = insolation(grid.latitude(y), axial_tilt_deg);
            let t = ((s - pole) / (equator - pole)) as f32;
            celsius(t.clamp(INSOLATION_RANGE.0, INSOLATION_RANGE.1))
        })
        .collect()
}
```

The sea-level temperatures this gives, before the atmosphere and the star
act on them:

| Tilt | Equator | 45° | Pole |
| --- | --- | --- | --- |
| 0° | 29 °C | 7 °C | −33 °C |
| 23.44° (Earth) | 27 °C | 10 °C | −18 °C |
| 45° | 21 °C | 15 °C | 7 °C |
| 54.7° | 17 °C | 17 °C | 17 °C |
| 90° | 7 °C | 21 °C | 29 °C |

The tilt is resolved with the moons in
[`crates/mapgen/src/request.rs`](../../crates/mapgen/src/request.rs) and
[`crates/mapgen/src/moons.rs`](../../crates/mapgen/src/moons.rs)
(`PlanetEffects::tilt`). The rows are built in
[`crates/mapgen/src/climate.rs`](../../crates/mapgen/src/climate.rs), and
the star and the eccentric orbit act on them in
[`crates/mapgen/src/star.rs`](../../crates/mapgen/src/star.rs)
(`star_temperatures`, `mean_flux_factor`), called from
[`crates/mapgen/src/generation.rs`](../../crates/mapgen/src/generation.rs).

### Where it departs from the science

- **No seasons in the climate.** Temperatures, ice and biomes come from
  annual means. The **Season** slider moves only the light. On Earth-like
  tilts this is what maps of climate zones show. At high tilt it hides the
  real story: at 90°, a pole that gets 433 W/m² over a year spends half of
  it in darkness, yet the app paints it as a warm, steady climate.
- **Two terms of the series.** Close to Earth's tilt the two-term formula
  stays within 5 % of the exact average. At other tilts it drifts. With no
  tilt it gives the poles 128 W/m² instead of none, so they are less cold
  than they would be (−33 °C rather than the −56 °C floor the clamp
  allows). It makes the curve exactly flat at 54.7°, where the true one
  dips by about 6 % at mid-latitudes, and moves the tilt where the poles
  overtake the equator from 53.9° to 54.7°.
- **Temperature from light by an Earth fit.** One curve, fitted to Earth,
  turns light into temperature for every tilt. Because it bends, the
  planet's mean temperature moves with the tilt even though the mean light
  does not: 13.5 °C at 0°, 14.8 °C at Earth's tilt, 16.7 °C at 54.7°. Ice
  and clouds, which would change with the tilt, are not modelled.
- **A steady tilt, set once.** Each world has one tilt for its whole life.
  Real tilts wander (by ±1.3° for Earth, by tens of degrees for Mars), and
  the ice ages follow such swings.
- **The moon rule is a threshold.** A moon of 0.005 of the planet's mass or
  more steadies the tilt, a lighter one does not. That cut is the app's
  choice, between the Moon and a moon too small to matter; the studies
  above computed only Earth with and without its Moon. Without a large
  moon the tilt is drawn from 0 to 60°, Mars's range, while Laskar et al.
  (1993) found up to 85° possible for a moonless Earth, and Lissauer et al.
  (2012) a much narrower range over long times.
- **No tilt above 90°.** The app takes tilts from 0 to 90°. A planet that
  spins backwards, like Venus (177.4°) or Uranus (97.8°), can only be
  entered as its mirror image (2.6°, 82.2°). For the annual light this
  changes nothing, since the series depends on the tilt only through
  $\cos^2\varepsilon$.
- **Eccentric orbits: the year's mean only.** The app adds the
  $1/\sqrt{1-e^2}$ factor, which is right for annual means, but not the
  uneven seasons it brings (46 % more light at perihelion on Mars).
- **The season slider on a lone world.** On a world that is not part of a
  system, the sun's latitude is $\varepsilon\sin(2\pi\,\text{season})$,
  not $\arcsin(\sin\varepsilon\sin\lambda)$. The two agree at the
  equinoxes and solstices; between them the sun stands too far from the
  equator, by up to 0.3° at Earth's tilt, 5° at 60° and 19° at 90°. In
  a system the sun's direction comes from the orbit geometry and is
  exact.

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
