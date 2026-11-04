# Analysis: giant planets

Backlog item 14. User request (2026-11-04): a scientific study of gas
giants (their types, composition, formation and gravitational influence),
then a first reflection on how to bring them into the generator.

Today every world is a rocky or icy body with a surface: heights, seas,
rivers, biomes, plates, craters (`crates/mapgen`). It has a star and an
orbit (`src/star.rs`, item 11), moons drawn from its seed (`src/moons.rs`,
item 13) and is shown in a planet view, a star view and a moon view. A
world is alone around its star: no other planet, and no giant anywhere.
The moons study left the moons of giants out of scope for that reason
(`11-moons.md`).

## Decisions (2026-11-04)

Taken with the user:

- **All three ways, in order, one pull request each:** A (giant
  neighbours in each world's system), then B (giant worlds), then C
  (rocky worlds orbiting a giant).
- **Giant worlds (B):** cold Jupiters and Saturns, ice giants, hot
  Jupiters (ultra-hot ones included), and rings on a share of them.
- **Metallicity:** a new star parameter, drawn like the stars nearby
  and settable in the form.
- **Exomoons (C):** an Earth-sized moon of a giant is rare at realistic
  odds when a world is drawn, and free to choose in the form.

## 1. What a giant planet is

A giant planet is mostly hydrogen and helium held by its own gravity,
with no solid surface: the atmosphere thickens into a supercritical
fluid, then, in the largest, into metallic hydrogen. Two families share
the name:

- **Gas giants** (Jupiter, Saturn): about 85–90 % hydrogen and helium by
  mass, close to the composition of the Sun, enriched in heavier
  elements.
- **Ice giants** (Uranus, Neptune): only 10–20 % hydrogen and helium,
  in an envelope around a mantle of "ices" (water, ammonia, methane, as
  hot dense fluids, not solids) and a rocky core. Ices and rock make up
  80–90 % of their mass.

### The four giants of the Solar System

| | Jupiter | Saturn | Uranus | Neptune |
| --- | --- | --- | --- | --- |
| Mass (Earth = 1) | 317.8 | 95.2 | 14.5 | 17.1 |
| Equatorial radius (Earth = 1) | 11.2 | 9.45 | 4.01 | 3.88 |
| Mean density (g/cm³) | 1.33 | 0.69 | 1.27 | 1.64 |
| Orbit (AU) | 5.2 | 9.5 | 19.2 | 30.1 |
| Rotation | 9 h 56 min | 10 h 33 min | 17 h 14 min (retrograde) | 16 h 6 min |
| Flattening | 0.065 | 0.098 | 0.023 | 0.017 |
| Axial tilt | 3.1° | 26.7° | 97.8° | 28.3° |
| Effective temperature | 124 K | 95 K | 59 K | 59 K |
| Heat emitted / absorbed | 1.7 | 1.8 | ~1.1 | 2.6 |
| Moons (known, 2025) | 95 | 274 | 29 | 16 |
| Rings | faint dust | bright, icy | narrow, dark | narrow, dark |

Values: NASA planetary fact sheets. The ratio of heat emitted to heat
absorbed shows the internal heat left from formation and from helium
rain (Saturn); Uranus has almost none, an open question.

### Interior

- **Jupiter:** molecular hydrogen outside, metallic hydrogen from about
  80–85 % of the radius inwards (where it conducts and drives the
  strongest magnetic field of the planets), and a core of heavy
  elements. Juno's gravity data favour a **dilute core**, spread over a
  large fraction of the radius rather than a compact rock ball: about
  7 to 25 Earth masses of heavy elements (Wahl et al. 2017).
- **Saturn:** the same layers, less compressed (hence its density, lower
  than water); helium separating from hydrogen and raining inwards
  heats it and depletes its upper atmosphere in helium.
- **Ice giants:** a thin hydrogen–helium envelope, a thick ionic water–
  ammonia mantle (where their tilted, off-centre magnetic fields arise)
  and a rocky core.

### What we see: clouds and colours

The visible surface is the top cloud deck, which depends on temperature:
each gas condenses at its own height. Sudarsky, Burrows & Pinto (2000)
sorted giant planets into five classes by effective temperature, and so
by distance from their star:

| Class | Teff | Top clouds | Look | Example |
| --- | --- | --- | --- | --- |
| I, "Jovian" | ≲ 150 K | ammonia ice | banded cream, ochre, brown | Jupiter, Saturn |
| II, "water cloud" | ~250 K | water ice | bright white (highest albedo) | 47 UMa b (predicted) |
| III, "clear" | ≳ 350 K | none | deep azure (gas scattering, methane absorbs red) | warm giants |
| IV, "roasters" | 900–1 500 K | deep silicates | very dark, alkali metals absorb | HD 209458 b |
| V | ≳ 1 500 K | high silicate and iron clouds | grey-green to dark, glowing red at night | 51 Peg b |

Colder still, methane clouds and methane gas (which absorbs red light)
make ice giants pale cyan (Uranus) or blue (Neptune). Jupiter's colours
come from chromophores in ammonium hydrosulfide and ammonia clouds: pale
**zones** (rising air, high clouds) and darker **belts** (sinking air,
deeper clouds).

### Weather: bands, jets and storms

- **Bands:** fast rotation turns the flow into zonal jets alternating
  with latitude. Their width follows the Rhines scale `L ≈ √(2U/β)`,
  `U` the wind speed and `β = 2Ω cos φ / R` the gradient of rotation;
  shallow-water models reproduce the number of bands of each giant from
  its radius and spin (Scott & Polvani 2007). A faster, larger planet
  has more, narrower bands.
- **Jets:** Jupiter has a prograde equatorial jet and about five
  alternating jets in each hemisphere (up to 150 m/s); Saturn a very
  wide equatorial jet (up to about 400 m/s) and fewer bands; Uranus and
  Neptune three broad jets, retrograde at the equator, prograde at high
  latitudes, Neptune's the fastest in the Solar System (up to about
  600 m/s).
- **Storms:** long-lived anticyclones (the Great Red Spot, larger than
  Earth, at least 190 years old; Neptune's dark spots, lasting a few
  years); small white ovals; turbulent eddies where jets shear; polar
  features (Saturn's hexagon, Jupiter's polar cyclone clusters); great
  white storms on Saturn every ~30 years.
- **Flattening:** fast spin flattens the poles: `f ≈ 0.6–0.75 q`, with
  `q = Ω²R³/GM` the ratio of spin to gravity (Jupiter: q = 0.089,
  f = 0.065; Saturn: q = 0.155, f = 0.098).

### Size against mass

A giant's radius barely grows with its mass: added mass compresses it.
Chen & Kipping (2017) fit `R ∝ M^0.59` for "Neptunian" worlds (from about
2 Earth masses) and `R ∝ M^−0.04` for "Jovian" ones (from about 0.4
Jupiter masses, roughly 130 Earth masses): every giant from Saturn's
mass to about 80 Jupiter masses is about one Jupiter radius. Beyond
**about 13 Jupiter masses** deuterium fusion starts: the object counts
as a **brown dwarf**, not a planet (the usual dividing line).

Hot giants are larger than that: close to their star, giants above an
equilibrium temperature of about 1 000 K are **inflated**, up to about
2 Jupiter radii, a few percent of the starlight they get being turned
into internal heat (Thorngren & Fortney 2018, favouring Ohmic heating).

## 2. Kinds of giant planets

By mass:

- **Mini-Neptunes / sub-Neptunes** (about 2–10 Earth masses): a rocky
  or icy core under a hydrogen envelope; the commonest planets the
  Kepler mission found, absent from the Solar System. Out of scope here
  (they would be a separate study).
- **Ice giants / Neptunes** (about 10–50 Earth masses).
- **Saturns** (about 0.1–0.5 Jupiter masses).
- **Jupiters** (about 0.5–3 Jupiter masses).
- **Super-Jupiters** (about 3–13 Jupiter masses), then brown dwarfs.

By temperature and orbit:

- **Cold giants** beyond the ice line (the Solar System's four).
- **Warm Jupiters** (periods of 10–200 days).
- **Hot Jupiters** (periods under about 10 days, `Teq` over about
  1 000 K): inflated, tidally locked, with hot spots shifted east of the
  substellar point by equatorial superrotation; **ultra-hot Jupiters**
  (`Teq` over about 2 200 K, KELT-9 b at about 4 600 K on its day side)
  where molecules break up and the day side is nearly a star's
  atmosphere.
- **Puffy planets / super-puffs**: very low density (Kepler-51 planets,
  under 0.1 g/cm³), young or with hazy, extended atmospheres.

## 3. Formation

- **Core accretion** (Pollack et al. 1996): dust grows into
  planetesimals, which build a core; past a critical mass of about
  10 Earth masses (5–15, model-dependent) the core pulls in gas faster
  and faster (runaway accretion) until the gap it opens in the disk or
  the disk's dissipation stops it. The difficulty: it must happen
  before the gas disk disappears, after a few million years. **Pebble
  accretion** (Lambrechts & Johansen 2012), sweeping up centimetre-sized
  pebbles drifting through the gas, builds cores much faster and is the
  favoured answer.
- **The ice line** (snow line): beyond it, water freezes and solids are
  several times more abundant, so cores grow large enough. Around the
  young Sun it lay near 2.7 AU (Hayashi 1981); it scales roughly as
  `√L` of the star. Giants are most common just beyond it.
- **Ice giants** formed farther out or later, where and when the gas
  was thinner: they built large cores but captured little gas.
- **Disk instability** (Boss 1997): a massive, cold disk fragments
  directly into giant planets in centuries, with little core. It may
  explain massive giants on very wide orbits found by direct imaging
  (HR 8799, at 15–70 AU).
- **Migration:** a young planet exchanges angular momentum with the gas
  disk and drifts, usually inwards (types I and II). Hot Jupiters cannot
  form where they are: they migrated, either smoothly through the disk
  (Lin, Bodenheimer & Richardson 1996, after 51 Peg b, Mayor & Queloz
  1995) or by **high-eccentricity migration**: a companion pumps the
  orbit's eccentricity (Kozai–Lidov cycles, scattering), and tides at
  periapsis then shrink and circularise it (Fabrycky & Tremaine 2007).
  The Solar System's own giants moved: Jupiter drifted in and back out
  (the **Grand Tack**, Walsh et al. 2011, which would explain a small
  Mars), and a later instability among the giants (the **Nice model**,
  Gomes et al. 2005) scattered small bodies across the system.

## 4. How common they are

- **Around Sun-like stars:** about 14 giant planets (0.1–20 Jupiter
  masses) per 100 stars between 2 and 8 AU, and about 9 per 100 between
  8 and 32 AU (Fulton et al. 2021); occurrence peaks between 1 and
  10 AU, four times higher than inside 1 AU, and falls further out.
- **Hot Jupiters:** about 1 % of Sun-like stars (1.2 ± 0.4 % in the
  solar neighbourhood, Wright et al. 2012; 0.4 % in the Kepler field,
  Howard et al. 2012). They are usually alone: few have close
  neighbours (Steffen et al. 2012), WASP-47 being a famous exception.
- **Mass and period:** small giants outnumber large ones:
  `dN ∝ M^−0.31 P^0.26 d ln M d ln P` (Cumming et al. 2008).
- **Metallicity:** giant planets are much more frequent around
  metal-rich stars: `P ≈ 3 % × 10^(2 [Fe/H])` for close-in giants
  (Fischer & Valenti 2005), about five times more at twice the Sun's
  metals. Our stars have no metallicity parameter yet.
- **Star mass:** fewer around M dwarfs (about 3 % have a Saturn-mass or
  larger planet within 5 AU, Johnson et al. 2010; under 1 % with periods
  under 10 days, Bonfils et al. 2013; giants about 4 times rarer than
  around FGK stars, Clanton & Gaudi 2014), more around A stars: a
  rising function of the star's mass, roughly `M^0.8–1` (Montet et al.
  2014 for M dwarfs).
- **Neptunes** are more common than Jupiters, and mini-Neptunes and
  super-Earths far more common still.

## 5. Gravitational influence

### On the orbits of other planets

- **Hill sphere:** a planet dominates gravity within
  `r_H = a (m / 3M★)^(1/3)` (Jupiter: 0.35 AU). Moons are stable within
  about a third to a half of it (prograde and retrograde).
- **Spacing:** two planets on near-circular orbits never meet if their
  orbits are more than `2√3 ≈ 3.5` mutual Hill radii apart (Gladman
  1993); systems of three or more need about 10 or more to stay stable
  over billions of years (Chambers et al. 1996). On eccentric orbits
  what counts is the gap between one's apoapsis and the next one's
  periapsis. A rocky planet in the habitable zone of a Sun-like star is
  safe from a Jupiter beyond about 2–3 AU on a near-circular orbit.
- **Forced eccentricity:** a giant on an eccentric orbit forces an
  eccentricity on an inner planet, `e_forced ≈ (5/4) (a / a_g) e_g /
  (1 − e_g²)` (Laplace–Lagrange secular theory, Heppenheimer 1978): a
  Jupiter at 5 AU with `e = 0.2` forces about 0.05 on a planet at 1 AU.
  The planet's eccentricity then swings around that value over tens of
  thousands of years: on Earth, Jupiter and Venus drive the 100 000-
  and 405 000-year eccentricity cycles of Milankovitch's climate
  theory.
- **Resonances** clear or trap small bodies: the Kirkwood gaps of the
  asteroid belt are Jupiter's resonances; resonant chains of planets
  (TRAPPIST-1) come from migration.
- **Hot Jupiters** migrating through the inner system scatter or destroy
  the rocky planets on their way; high-eccentricity migration does so
  too. A rocky planet in the habitable zone of a hot Jupiter's star is
  unlikely, though not impossible.

### On the habitability of rocky planets

- **Impacts:** the old idea of Jupiter as a shield does not hold simply.
  In N-body simulations varying the mass of a giant on Jupiter's orbit,
  Earth's impact rate from asteroids peaks for a giant of about a fifth
  of Jupiter's mass (about twice today's rate) and stays high for
  Jupiter's own mass (Horner & Jones 2008; 2009 for Centaurs; 2012).
  Without any giant, the rate is low.
- **Water:** Jupiter's growth scattered water-rich planetesimals from
  beyond the ice line into the inner system, one of the likely sources
  of Earth's water (Raymond & Izidoro 2017).
- **A small Mars and an asteroid belt** (Grand Tack); a late shake-up of
  the system (Nice model) possibly behind the Late Heavy Bombardment.

### On their own moons

- **Regular moon systems** form in a circumplanetary disk: their total
  mass is about 10⁻⁴ of the planet's (Canup & Ward 2006). A Jupiter's
  largest moons are Mercury- to Mars-sized at most; an Earth-sized moon
  would need a planet of about 30 Jupiter masses, or a capture (Triton
  was captured; binary-exchange capture, Williams 2013).
- **Tidal heating:** moons on eccentric orbits, kept eccentric by
  resonances, are kneaded by the planet's tides: Io (the most volcanic
  body known, 1:2:4 Laplace resonance with Europa and Ganymede),
  Europa's and Enceladus's underground oceans.
- **Habitable exomoons:** a moon of a giant in the star's habitable zone
  gets starlight, light reflected by the giant, the giant's own heat and
  tidal heating. Closer than the **habitable edge**, about 10 planetary
  radii for typical cases, tidal and planetary heat drive a runaway
  greenhouse (Heller & Barnes 2013). Such moons are locked to their
  giant: their day is their orbit around it, and they pass through its
  shadow, with eclipses of the star every orbit for low tilts.
- **Rings:** inside the Roche limit, debris cannot gather into a moon.
  Saturn's bright rings may be young (a few hundred million years,
  debated); every giant of the Solar System has rings.

## 6. What it would mean for the generator

A giant has no surface: nothing of today's pipeline (heights, erosion,
rivers, biomes, plates, craters, hexes) applies to it. The generator can
meet giants in three ways, independent of each other:

### A. Giant neighbours in the system (cheap, affects every world)

Each world gets the giant planets of its system, drawn from the seed:

- **Draw:** odds by star mass (fewer around M dwarfs, more around F and
  A stars), mass from `M^−0.31`, orbits peaking just beyond the ice line
  (from about `2.7 √L` AU), hot Jupiters at about 1 %, eccentricities
  from the same Beta distribution as the world's (Kipping 2013).
- **Stability:** a giant too close to the world's orbit (under about
  3.5 mutual Hill radii, apoapsis to periapsis) is pushed out or
  dropped; a hot Jupiter excludes a rocky planet in the habitable zone
  of the same system except in rare cases.
- **Effects on the world:**
  - its eccentricity: at least the forced eccentricity of its giants
    (`src/star.rs` draws it today);
  - its craters: the impact rate scaled by the Horner & Jones curve
    (the crater model of item 8 takes a density);
  - its sky: giants as bright "stars" that move, and in the star view
    their orbits around the star, not to scale like the rest.
- **Data:** a `giants` list in `world.json` (mass, radius, orbit,
  eccentricity, class, colour), a new format version.
- **Cost:** negligible: a few draws and formulas per world.

### B. Giants as worlds of their own (a new kind of world)

A world type for giants (Jupiter, Saturn, ice giant, hot Jupiter),
generated by a separate, much cheaper pipeline:

- **Bands:** latitudes split into jets whose count follows the Rhines
  scale from the radius and spin (Jupiter's about a dozen bands, an ice
  giant's three), each with a speed and a colour from its class
  (Sudarsky), zones pale, belts dark.
- **Texture:** noise advected along the bands, turbulence and festoons
  where neighbouring jets shear, vortices placed in the bands
  (anticyclones in zones, a great spot with some probability, white
  ovals), polar hexagon or cyclones.
- **Layers** for the viewer: colour, wind speed (signed, east or west),
  temperature at the cloud tops, cloud height; the panel shows mass,
  radius, density, class, flattening, internal heat.
- **Viewer:** an oblate globe (flattening from the spin), the bands
  turning at their own speed when animated (differential rotation, a
  shader offset by latitude: cheap), rings (a ring disk with gaps and
  shadow) for a share of giants, the night side of hot giants glowing
  red, an inflated radius for hot ones.
- **Not applicable:** relief, sea level, rivers, hexes, plates, craters,
  erosion, detail tiles (or detail tiles of the cloud texture only).
  The form, world card, list and URLs would need a "giant" variant.
- **Cost:** generation fast (a few noise passes, no erosion); the work is
  in the viewer and in branching the form and card.

### C. Rocky worlds orbiting a giant (habitable exomoons)

A world could be a moon of a giant rather than a planet of the star:

- **The giant in the sky:** huge (Jupiter from Europa is about 12° wide,
  23 times the Moon from Earth), with phases; it hides the star once per orbit for
  moons near its equator (eclipses), and lights the night side.
- **Locked to the giant:** the day is the orbit around the giant (days
  rather than hours), the giant fixed in the sky of one hemisphere,
  never seen from the other.
- **Tidal heating:** a closer or more eccentric orbit raises volcanism
  (the volcanic parameter, item 8), up to Io's; inside the habitable
  edge, a runaway greenhouse (a caustic world).
- **Size:** realistic in-situ moons are at most Mars-sized for a
  Jupiter-mass giant; an Earth-sized habitable moon needs a capture or a
  super-Jupiter: allowed but drawn rarely, or chosen in the form.
- **Overlap:** the moons work (item 13) already draws the sky with a
  planet and eclipses for a moon of a rocky planet: this would reuse
  it, with roles reversed (the world as the moon).

### Proposed order

1. **A, giant neighbours** first: small, self-contained, it gives every
   existing world a richer system and measurable effects (eccentricity,
   craters, sky), and builds the data that B and C need (a giant's mass,
   radius, class, colour).
2. **B, giant worlds:** the most visible, the largest change (a second
   pipeline and a viewer variant).
3. **C, exomoons:** builds on A (the parent giant) and on the moon
   views; the most original result (a habitable moon under a giant).

## 7. Implementation plan

### Step A: giant neighbours

- **Metallicity** (`[Fe/H]`, dex): drawn from the solar neighbourhood
  (a normal law of mean −0.1 and spread 0.2, within −1 and 0.5,
  Casagrande et al. 2011), on its own random stream so the other drawn
  values of a seed do not change; a `metallicity` request and form
  parameter and a `[star] metallicity` preset key.
- **Module `src/giants.rs`:** `GiantSystem::draw(params)` on a new
  random stream. A Poisson number of gas giants (0.1 to 13 Jupiter
  masses) of mean 0.25 for a Sun-like star at the Sun's metallicity,
  times `10^(2 [Fe/H])` and the star's mass (capped), and of ice giants
  (10 to 50 Earth masses) of mean 0.3, times `10^(0.5 [Fe/H])`; masses
  from `M^−0.31` (gas) or log-uniform (ice); orbits log-normal around
  two (gas) and five (ice) times the ice line `2.7 √L` AU; about 1 % of
  Sun-like stars with a hot Jupiter (periods of 1 to 10 days), fewer
  when the world orbits nearby (hot Jupiters are lonely);
  eccentricities from Kipping's law, none for hot Jupiters.
- **Spacing:** a giant closer than `2√3` mutual Hill radii (apoapsis to
  periapsis) to the world or to another giant is redrawn a few times,
  then dropped; at most four giants.
- **Each giant:** a letter (b, c, … by distance, the world being the
  system's planet of interest), mass, radius (power laws through
  Neptune, Saturn and Jupiter, `M^−0.04` above; inflated when
  `Teq > 1 000 K`), orbit, eccentricity, year, equilibrium temperature,
  Sudarsky class (ice giants apart), a colour for the views, rings
  (none, faint or bright).
- **Effects on the world:**
  - **eccentricity:** when drawn, at least the forced eccentricity of
    the giants (secular theory), with its range of swing stored;
  - **craters:** the impact rate scaled by the mass of the giant
    outside its orbit (Horner & Jones 2008): low without one, highest
    for a fifth of Jupiter's mass, Jupiter's as calibrated today.
- **Data:** `world.json` gains `star.metallicity` and `giants` (format
  version 10; older worlds have none and a metallicity of 0).
- **Viewer:** the star view draws the giants on their orbits (beyond the
  world's system, distances compressed and said so) and animates them
  at Kepler's pace; the panel lists them; a **Giants** checkbox; the
  world card counts them; the form gets the metallicity.

### Steps B and C

Planned in detail once A is in, from what it settles (the giant's data,
the system scale of the star view).

## Sources

- NASA Goddard, planetary fact sheets (Jupiter, Saturn, Uranus,
  Neptune): <https://nssdc.gsfc.nasa.gov/planetary/factsheet/>.
- Sudarsky, Burrows & Pinto 2000, "Albedo and Reflection Spectra of
  Extrasolar Giant Planets", ApJ 538, 885:
  <https://arxiv.org/abs/astro-ph/9910504>; Sudarsky, Burrows & Hubeny
  2003, ApJ 588, 1121.
- Wahl et al. 2017, "Comparing Jupiter interior structure models to
  Juno gravity measurements and the role of a dilute core", GRL 44,
  4649: <https://arxiv.org/abs/1707.01997>.
- Scott & Polvani 2007, "Forced-dissipative shallow-water turbulence on
  the sphere and the atmospheric circulation of the giant planets",
  J. Atmos. Sci. 64, 3158; Rhines 1975, J. Fluid Mech. 69, 417.
- Chen & Kipping 2017, "Probabilistic Forecasting of the Masses and
  Radii of Other Worlds", ApJ 834, 17:
  <https://arxiv.org/abs/1603.08614>.
- Spiegel, Burrows & Milsom 2011, "The Deuterium-burning Mass Limit for
  Brown Dwarfs and Giant Planets", ApJ 727, 57.
- Thorngren & Fortney 2018, "Bayesian Analysis of Hot-Jupiter Radius
  Anomalies: Evidence for Ohmic Dissipation?", AJ 155, 214:
  <https://arxiv.org/abs/1709.04539>.
- Pollack et al. 1996, "Formation of the Giant Planets by Concurrent
  Accretion of Solids and Gas", Icarus 124, 62; Lambrechts & Johansen
  2012, "Rapid growth of gas-giant cores by pebble accretion", A&A 544,
  A32; Boss 1997, "Giant Planet Formation by Gravitational Instability",
  Science 276, 1836; Hayashi 1981, Prog. Theor. Phys. Suppl. 70, 35.
- Mayor & Queloz 1995, Nature 378, 355; Lin, Bodenheimer & Richardson
  1996, Nature 380, 606; Fabrycky & Tremaine 2007, ApJ 669, 1298.
- Walsh et al. 2011, "A low mass for Mars from Jupiter's early gas-driven
  migration", Nature 475, 206; Gomes et al. 2005, Nature 435, 466.
- Fulton et al. 2021, "California Legacy Survey. II. Occurrence of Giant
  Planets beyond the Ice Line", ApJS 255, 14:
  <https://arxiv.org/abs/2105.11584>.
- Wright et al. 2012, ApJ 753, 160; Howard et al. 2012, ApJS 201, 15;
  Steffen et al. 2012, PNAS 109, 7982.
- Cumming et al. 2008, "The Keck Planet Search: Detectability and the
  Minimum Mass and Orbital Period Distribution of Extrasolar Planets",
  PASP 120, 531.
- Fischer & Valenti 2005, "The Planet-Metallicity Correlation", ApJ 622,
  1102.
- Johnson et al. 2010, PASP 122, 905; Bonfils et al. 2013, A&A 549,
  A109; Clanton & Gaudi 2014, ApJ 791, 91; Montet et al. 2014, ApJ 781,
  28.
- Gladman 1993, "Dynamics of systems of two close planets", Icarus 106,
  247; Chambers, Wetherill & Boss 1996, Icarus 119, 261; Heppenheimer
  1978, A&A 65, 421.
- Horner & Jones 2008, "Jupiter – friend or foe? I: the asteroids",
  IJA 7, 251: <https://arxiv.org/abs/0806.2795>; 2009 (II: the Centaurs),
  <https://arxiv.org/abs/0903.3305>; Horner & Jones 2012,
  <https://arxiv.org/abs/1202.1314>.
- Raymond & Izidoro 2017, "Origin of water in the inner Solar System:
  Planetesimals scattered inward during Jupiter and Saturn's rapid gas
  accretion", Icarus 297, 134.
- Canup & Ward 2006, "A common mass scaling for satellite systems of
  gaseous planets", Nature 441, 834; Williams 2013, "Terrestrial
  Exomoons", Astrobiology 13, 315.
- Heller & Barnes 2013, "Exomoon Habitability Constrained by
  Illumination and Tidal Heating", Astrobiology 13, 18:
  <https://arxiv.org/abs/1209.5323>.
- Kipping 2013, MNRAS 434, L51 (eccentricity distribution, as for the
  world's orbit).
- Casagrande et al. 2011, "New constraints on the chemical evolution of
  the solar neighbourhood and Galactic disc(s)", A&A 530, A138 (the
  metallicity of nearby stars).
