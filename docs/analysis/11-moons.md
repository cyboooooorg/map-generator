# Analysis: moons

Backlog item 13. User request (2026-11-04): study the feasibility of
giving our planets moons, in realistic numbers, including systems where
the planet and its moon are of comparable size (Pluto and Charon) and
misshapen moons like those of Mars.

Today a world has a star and an orbit (`src/star.rs`, item 11), shown in
the star view (`web/src/lib/viewer/StarView.ts`, item 12), and may be
tidally locked to its star; it has no satellites. All our planet types
are rocky or icy: there are no giant planets, so moons formed in a
circumplanetary disk (the Galilean moons) are out of scope.

## 1. How rocky planets get moons

| Origin                            | Example                               | What it gives                                                                 |
| --------------------------------- | ------------------------------------- | ----------------------------------------------------------------------------- |
| Giant impact, debris re-accreting | the Moon, Charon                      | one large round moon, prograde, low inclination, tidally locked to its planet |
| Debris disk of a smaller impact   | Phobos and Deimos (Rosenblatt et al.) | small irregular moons, close in                                               |
| Capture                           | irregular moons of the giants         | small bodies on distant, eccentric, inclined, sometimes retrograde orbits     |

- **Giant impact:** the debris of an oblique impact forms a disk that
  accretes into a moon near the Roche limit, which then recedes by tides.
  Canup (2005) shows that oblique, low-velocity impacts produce
  satellite-to-planet mass ratios near 0.1 (Pluto–Charon: 0.12; Earth–
  Moon: 0.012).
- **Debris disk:** Rosenblatt et al. (2016) form Phobos and Deimos in the
  outer part of the disk of a large impact on Mars; a larger inner moon
  formed too, and fell back onto Mars within a few million years.
- **Capture:** needs a third body or gas drag to shed energy; it gives
  small moons on irregular orbits.

## 2. How many moons

- The terrestrial planets of the Solar System have three moons between
  four planets (Mercury 0, Venus 0, Earth 1, Mars 2); no exomoon is
  confirmed. The sample is tiny.
- **A large moon:** Elser et al. (2011), from N-body simulations of
  terrestrial planet growth, estimate that about **1 terrestrial planet
  in 12** gets a massive moon (1 in 4 to 1 in 45 depending on the
  assumptions); a later giant impact can destroy it.
- **Small worlds:** satellites are frequent among dwarf planets: three of
  the four brightest Kuiper belt objects have one (Brown et al. 2006),
  against about 11 % of the population as a whole; their satellites are
  thought to come from impacts.
- **Proposed distribution:**
  - most rocky planets have no moon;
  - about 8 % have a large moon (Elser et al.);
  - some have one to three small debris or captured moons (a
    calibration assumption: no statistics exist);
  - worlds of Pluto's size and below often have a companion, sometimes
    a binary one.

## 3. What limits moons

- **Stable zone:** between the Roche limit and a fraction of the Hill
  radius `r_H = a · (m_p / 3 M*)^(1/3)`. Numerical runs put the
  outermost stable orbit near a third of `r_H` (as used by Barnes &
  O'Brien 2002).
  - Earth: `r_H` ≈ 1.5 million km; the Moon is at 0.26 `r_H`.
  - The fluid Roche limit is `2.44 R_p (ρ_p / ρ_m)^(1/3)`: about 2.9
    Earth radii for the Moon.
- **Planets locked to their star:** their synchronous orbit lies beyond
  their Hill sphere, so every moon loses energy to tides and spirals in
  (Barnes & O'Brien 2002, for close-in giants; later work finds the
  habitable zones of M dwarfs hostile to large moons).
  - An Earth in the habitable zone of an M5V star has a Hill radius of
    about 200 000 km (31 Earth radii).
  - Consequence for us: worlds with `tidally_locked` get no lasting moon.
- **Tidal evolution:**
  - a moon above the synchronous orbit recedes, as the Moon does
    (3.8 cm/yr);
  - one below it falls in, as Phobos does (about 1.8 cm/yr; it will
    break into a ring in a few tens of Ma, 30 to 94 Ma depending on the
    source).
  - This can be shown (an "inward-spiralling moon") without simulating
    it.

## 4. Binary systems (Pluto and Charon)

- Charon has 12 % of Pluto's mass; the barycentre lies outside Pluto,
  about 2 100 km from its centre for a radius of 1 188 km. Both are
  locked face to face: Pluto turns in 6.4 days and Charon stays over the
  same hemisphere.
- An oblique, slow giant impact gives such mass ratios (Canup 2005);
  recent work also proposes a "kiss-and-capture" collision (Denton et al.
  2025) or a capture (arXiv:2511.17832).
- For the generator:
  - both bodies orbit the barycentre;
  - the planet's day equals the moon's orbital period;
  - a fixed sub-moon point exists, like the substellar point of our
    locked planets.
- Plausible mass ratios: up to about 0.15 from impacts; near-twins are
  possible but rarer, kept for small worlds.

## 5. Round or misshapen

- A body is pulled round by its own gravity above a size that depends on
  its material:
  - **icy:** about 200 to 400 km across (Mimas, 396 km, is round;
    Proteus, 420 km, is not);
  - **rocky:** up to about 900 km across (estimate by M. Brown).
- Below, bodies are **irregular**, roughly triaxial ellipsoids: Phobos is
  27 × 22 × 18 km, with a crater (Stickney, 9 km) large for its size.
- Rule for the generator: round above about 900 km for rock and 400 km
  for ice; below, irregular, the more so the smaller the body.

## 6. Effects of a moon on its planet

- **Axial tilt:** a large moon stabilises it. Without the Moon, Earth's
  obliquity could vary chaotically over 0–85° (Laskar et al. 1993). The
  tilt could be drawn in a narrow range around a stable value with a
  large moon, over a much wider one without: a physical link with our
  `axial_tilt` parameter.
- **Tides:** their range depends on the moon's mass and distance; it
  could widen the tidal flats (the `Wetland` and `Beach` band of low
  coasts).
- **Sight:** the moon in the star view, in the globe's sky, eclipses.

## 7. Feasibility in the project

Moons can be an addition nothing else depends on, so the existing seeds
keep their worlds.

| Part                                       | Work                                                                                                                                                                                                                                                                                                                                   | Cost   |
| ------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ |
| Rust: `moons` module                       | Draw the moons from the seed on a noise stream of their own: origin, radius, mass, semi-major axis (between Roche and a third of `r_H`), eccentricity, inclination, period, shape, migration. Rules: no lasting moon on locked planets, odds by size, a binary above a mass ratio of about 0.05. Stored in `world.json` under `moons`. | low    |
| Star view (`StarView.ts`, `starSystem.ts`) | Moon orbits around the planet with `keplerPosition`; for a binary, both bodies around the barycentre.                                                                                                                                                                                                                                  | low    |
| Surfaces of round moons                    | Each one generated as a small `barren` world at low resolution (craters and maria already exist), for its texture.                                                                                                                                                                                                                     | medium |
| Misshapen moons                            | The pipeline is equirectangular on a sphere: deform the mesh in the viewer only, a triaxial ellipsoid with low-frequency noise and large craters, outside the generation.                                                                                                                                                              | medium |
| Effects on the planet                      | Axial tilt, tidal flats, sub-moon point for binaries: they change existing worlds, so a separate step.                                                                                                                                                                                                                                 | medium |

## 8. Implementation plan (2026-11-04)

Goal set by the user: every moon can be viewed on its own in the viewer.
Each moon is therefore a **body** of its world: a small world generated
with the planet, with its own maps and data, opened in the planet view
like the planet itself (globe and map, layers, detail tiles, measure,
screenshot). Moons draw from a noise stream of their own, so the planet
of a seed does not change until step 5.

### Step 1: the moon system (Rust)

- **Module `src/moons.rs`:** `MoonSystem::draw(params, star)` from a new
  noise stream (`stream::MOONS`, 26). The planet's mass followed from
  its size at Earth's density (as `gravity_modifier` did); since
  2026-11-07 both follow the mass–radius relation of rocky planets
  (`WorldParams::mass_earth`), icy bodies lighter.
- **Channels and odds** (constants to calibrate, see section 2):
  - giant impact: 1 in 12 for planets above about 0.3 Earth radii, mass
    ratio 0.005–0.03 (the Moon: 0.0123); on small worlds (below about
    0.25 Earth radii) one in two, mass ratio up to 0.15;
  - debris moonlets: one or two moons of a few to some tens of km, close
    in, often below the synchronous orbit;
  - captured: one to three small irregular moons far out (0.05–0.3
    `r_H`), eccentric and inclined, some retrograde;
  - none on planets locked to their star.
- **Each moon:** a name by distance (I, II, …), radius (three axes when
  irregular), mass ratio, density and composition (ice beyond the snow
  line, from the planet's equilibrium temperature, rock otherwise),
  orbit (semi-major axis between the Roche limit and a third of `r_H`,
  eccentricity, inclination, period from Kepler's law), the synchronous
  orbit and the migration it implies (receding, or falling in like
  Phobos), tidally locked to its planet or not.
- **Binaries:** above a mass ratio of about 0.05 the barycentre is given,
  and whether it lies outside the planet (Pluto–Charon: yes).
- **Data:** `world.json` gains `moons` (format version 9; older worlds
  have none). The CLI and requests take `--moons auto|none` (default
  `auto`); the parameter line counts the moons.
- **Tests:** over a few thousand seeds, the share of large moons near
  1/12, no moon on locked planets, every orbit inside the stable zone,
  binaries only on small worlds, determinism, and the planet unchanged
  to the bit.

**Outcome (step 1, 2026-11-04).** `src/moons.rs` draws the system as
planned; `world.json` stores it under `moons`, and `--moons none` turns
it off. Over 4 000 seeds:

| World                     | No moon | One  | Two or more | Large moon | Binary |
| ------------------------- | ------- | ---- | ----------- | ---------- | ------ |
| Earth-sized, at 1 AU      | 63 %    | 21 % | 16 %        | 8.8 %      | none   |
| Pluto-sized, at 30 AU     | 37 %    | 50 % | 13 %        | 48 %       | 10 %   |

- The planet's mass followed from its size at Earth's density, as its
  gravity did: a small icy world was then heavier than Pluto (1.85
  g/cm³), so binaries are rarer than a Pluto density would give. Since
  2026-11-07 it follows the mass–radius relation of rocky planets,
  never less dense than the Moon (3.34 g/cm³), still heavier than Pluto.
- Each origin draws all of its values whatever its outcome, so one
  origin's odds never shift another's draws; the planet's day is drawn
  before the tidal-locking test, so a world's moons do not depend on
  the order of the checks.
- Moons are tidally locked to their planet except captured ones.

### Step 2: moon surfaces (Rust and server)

- **Each moon generated as a world:** preset `barren` for rock, a new
  built-in `icy-moon` preset for ice (bright ice, craters, no maria);
  circumference from its mean radius; no air, no erosion; a surface age
  from its origin (old for impact moons); the planet's star and orbit,
  so it gets their light. Its seed derives from the world's and its
  index.
- **Resolution:** the planet's width times the ratio of the radii,
  between 256 and the planet's width (a Moon around an Earth at 4096
  pixels: about 1024), so pixels have a similar size in km.
- **Irregular moons:** a new `shape` layer, the radius as a factor of the
  mean one (a triaxial ellipsoid plus low-frequency noise), exported as
  `shape.png` (16-bit); the elevation, craters included, stays the
  surface detail over it.
- **Files** (since replaced by the bodies of a system, `systems/<id>/bodies/<planet>-<n>/`, see [system-spec.md](../system-spec.md)): `worlds/<id>/moons/<n>/` with the layout of a world
  (`world.json`, `world.png`, `thumb.png`, `raw_data/`); a "Moons"
  stage in the job progress. Static files are already served under
  `/worlds/`.
- **Server:** detail tiles and crater epochs of a moon at
  `/api/worlds/{id}/moons/{n}/tiles/…` and `…/epoch/…`, the tile cache
  keyed by body; deleting or replacing a world takes its moons with it
  (they live in its directory).
- **Budget:** moons are smaller than their planet, so the generation
  should grow by 10–30 %; to measure.

**Outcome (step 2, 2026-11-04).** Moons are written as planned, with
three changes found on the way:

- **Presets in `presets/moons/`:** `rocky-moon` is bare rock everywhere
  (every biome remapped to `RockyWaste`), so the shading draws the
  craters and the albedo the maria; `icy-moon` is ice plains, older ice
  in basins and bare ice ridges. The server's preset catalog skips the
  directory: they are not planet types.
- **Small bodies:** the noise relief kept a planet's amplitude (several
  km) and buried the craters of a 10 km moon; it is now flattened towards
  the datum to 0.6 % of the body's radius over the noise's typical
  amplitude (bodies under about 2 100 km in radius; the Moon keeps 83 %),
  and the noise is no
  finer than at 5 000 km around (`MAX_NOISE_SCALE`; on a 70 km moon it
  turned into grain). The hillshade exaggerated relief 1 500 km per
  elevation unit, set for planetary pixels of 20 km: on a moon's pixels
  of 0.3 km every slope saturated; it now falls with the body's size
  towards the true relief (25 km per unit) under 300 km in radius. Worlds
  that small change; planets do not.
- **Cost:** a moon is a small world (512 to about 1 000 pixels wide):
  0.05 to 0.5 s each.

### Step 3: the moon view (viewer)

- **Subject switch:** Planet / Star / Moon, with a selector of the moons
  (and a shortcut to step through them); the button is disabled on a
  world without moons.
- **The planet view loads a body:** a base URL for files and tiles
  instead of the world id, so every tool works on a moon as on the
  planet, the crater epoch slider included.
- **Irregular moons:** the globe mesh is displaced by the `shape` layer
  (in the vertex shader, as the relief is); the map stays
  equirectangular, as the maps of Phobos are.
- **Moon panel** (like the star panel): origin, size (and axes),
  mass ratio, density, surface gravity, orbit (semi-major axis in km and
  planet radii, period, eccentricity, inclination, prograde or
  retrograde), synchronous orbit and migration, tidal locking, apparent
  size seen from the planet, and the barycentre for binaries. Every
  value explained in a tooltip; English and French catalogs.
- **Star view:** the planet with its moons in an inset (not to scale),
  on Kepler orbits; a binary turns around its barycentre.
- **World card and list:** the number of moons.

**Outcome (step 3, 2026-11-04).** The viewer shows each moon:

- **Bodies:** a moon is named `<world>/moons/<n>` wherever a world id
  goes; URLs encode it segment by segment (`bodyPath`), so files, detail
  tiles and crater epochs reach `moons/<n>/` and the server's moon
  routes, which take the world's revision. `LoadedWorld` keeps the
  world's metadata, statistics and rivers while it shows one of its
  moons, for the world card and the star view.
- **Switch:** Planet / Moon / Star, a selector of the moons, `U` to step
  through them; the hex grid is off on moons (the server resamples worlds
  only).
- **Shape:** the globe's vertex shader scales the sphere by the
  `shape` layer (16 bits, uploaded as a half-float texture) and tilts the
  normal by its slopes, so the light shows the shape; the map stays
  equirectangular.
- **Panel and star view** as planned; in the star view the moons are
  drawn on orbits not to scale, at their real pace relative to the year
  (two seconds a turn at the fastest).
- **Found on the way:** a moon's base map, 256 pixels wide, stretched
  over the globe turned the hillshaded grain of its noise relief into
  blotches; moons now get at least 512 pixels, their relief is flattened
  to 0.6 % of their radius instead of 1.5 %, and the detail tiles' octaves
  are flattened with it (`RENDER_VERSION` 5). The reproduce request and
  command of a world now carry its plate epoch and its moons turned off.

### Step 4: skies (optional)

The planet in the sky of each moon (with its phase), the moons in the
planet's sky; eclipses.

**Outcome (step 4, 2026-11-04).** With Sunlight on, the viewer draws
the other bodies of the system in the sky and their shadows:

- **Sky bodies:** a disk at infinity behind the surface, like the host
  star, but a quad rather than a point (the planet seen from a Phobos
  spans about 40°, beyond the largest point size), lit as a sphere by the
  sun, so phases follow; the planet seen from a moon is mapped with its
  `surface.png`, towards longitude 180° of the moon; the moons seen from
  their planet are plain rock or ice, full at the start of the season and
  turning at their period relative to the year as the seasons play.
- **Eclipses:** up to four shadow casters (the largest moons, or the
  planet seen from its moon), their centres and radii in radii of the
  body shown; the surface shader dims the sunlight by the share of the
  sun each one hides (umbra, penumbra widening with the distance behind
  the caster, annular when the umbra falls short): the Moon at 60 Earth
  radii, as in the real sky, gives annular eclipses at apogee.
- The geometry lives in `lib/skyBodies.ts`, tested against the Moon and
  Phobos; the shader repeats its eclipse formula.

### Step 5: effects on the planet (seeds change)

- The axial tilt drawn in a narrow range with a large moon, over a wide
  one without (Laskar et al. 1993).
- Tidal flats widened by the tidal range of the moons.
- Binaries: the planet's day equals the moon's month, with a sub-moon
  point.

**Outcome (step 5, 2026-11-04).** Moons shape their planet when its
parameters are resolved, so `world.json` explains the world as
generated:

- **Axial tilt:** a drawn tilt (neither the request nor the preset sets
  it) moves from `[5, 40]` to `[0, 60]` without a moon of mass ratio
  0.005 or more, mapped linearly so no draw changes. Since 2026-11-07
  the free range is `[0, 85]`, the chaotic zone of a moonless Earth
  (Laskar, Joutel & Robutel 1993) rather than Mars's 0–60° (Laskar &
  Robutel 1993).
- **Tides:** `PlanetEffects` sums the tidal heights of the moons and the
  star, `mass ratio × (radius / distance)³ × size`, relative to Earth's
  (the Sun's 0.46 of the Moon's). A planet locked to its star feels no
  tide from it that comes and goes, nor a binary from its locked moon.
  The preset's shore width is multiplied by the fourth root of the tides,
  between 0.5 and 2: with the square root, shores of moonless Earths
  fell from 2.1 % to 0.3 % of the land at 960 pixels, far beyond what the
  range of tides does to tidal flats (the slope of the coast matters
  more). With the fourth root, terran 7 goes from 3.9 % to 3.3 % of shores
  at 1920 pixels (0.75 of the width, moonless). `world.json` stores the
  tides and whether the tilt is steadied, and the preset's shore width as
  used.
- **Binaries:** the moon hangs over the same point of its planet in the
  viewer's sky, at longitude 180°; the planet's day already equals the
  moon's month (step 1).

**Follow-up (moon count, 2026-11-04).** The user found moons rare and
asked for their number in the form, the rest of the world following:

- **Request:** `moons` is `auto`, `none` or `{"count": n}` (1 to 6;
  `--moons n`). The seed's own moons come first, the largest kept when
  there are too many; more come from a draw sequence of their own (a
  giant-impact moon if none, debris moonlets, then captured moons).
- **Room:** the spacing of eight mutual Hill radii, made for orbits of
  comparable masses, left no room beside a large moon (a Moon at 43
  Earth radii excluded 0 to 106); added moons keep out of the chaotic
  zone of each heavier one instead, `1.5 a q^(2/7)` from periapsis to
  apoapsis (Wisdom 1980). Captured ones range from 0.03 to 0.45 Hill
  radii, retrograde beyond a third of it (where only retrograde orbits
  last), half as eccentric: over 1 200 Earth systems of 1 to 6 moons,
  fewer than 10 fall short.
- **Coherence:** a drawn star and orbit that leave no room (a planet
  locked to its star, a Hill sphere too small) are drawn again, up to 64
  times; set ones are refused (`no_room_for_moons`, worded by the web
  app). The tilt and tides then follow the moons as in step 5.
- The seed's draws in `auto` are unchanged: a hash of 6 000 systems
  matches before and after.

**Follow-up (a giant's moon, 2026-11-04).** With the rocky moons of
giants (item 14, step C), `PlanetEffects` covers a world with a host:

- **Tilt:** a moon locked to its giant has its equator in the giant's
  equatorial plane (a Cassini state, as the Galilean moons'); its seasons
  follow the giant's own tilt, drawn in the usual `[5, 40]`, so the tilt
  counts as steadied.
- **Tides:** the star's come and go with the moon's month, whether or not
  the giant is locked to the star. The giant's bulge, `mass ratio ×
  (radius / distance)³`, is huge but still on a locked moon; only the part
  an eccentric orbit swings, about `3e` of it, comes and goes: 1.5 to 18
  times Earth's tides around a Jupiter at 20 of its radii (eccentricities
  of 0.001 to 0.012), so the shores reach their widest (×2).

**Follow-up (the giant's other moons, 2026-11-04).** `HostGiant` gains
`siblings`, drawn from the host stream after its other values (so none
of them moves): two to four regular moons sharing about 10⁻⁴ of the
giant's mass (Canup & Ward 2006), on the slots of a resonant chain
(semi-major axes `2^(2/3)` apart, periods doubling, as Io, Europa and
Ganymede) around the world's, inside a third of the Hill radius and
outside 3 giant radii; icy beyond 12 radii or a cold star. Around a
Jupiter they come out 500 to 4 000 km in radius. The viewer draws them in
the world's sky beside the giant (`siblingSky`), their direction and size
from the world as they turn at their period relative to the world's,
and casts their shadows, the four largest of all bodies kept.

### Choices (confirmed by the user, 2026-11-04)

- Moons are generated with the planet (not on demand when first
  opened): simpler, and the viewer opens them at once.
- Moon surfaces come from the generator, not from a procedural shader in
  the browser: they get craters, maria, detail tiles and data layers
  like any world.
- The user can turn moons off (`none`), not choose their number: they
  follow from the seed and the physics.

## 9. Science fixes (2026-11-07)

The science articles (`docs/science/how-many-moons.md`,
`moon-shapes.md`, `tidal-locking.md`) found where the rules above depart
from the science. What changed:

- **Round from 300 km (rock).** The rock limit is now 600 km across, a
  mean radius of 300 km, where Lineweaver & Norman 2010 observe the
  change (the 900 km of section 5 came from an estimate). The ice limit
  stays 400 km. The irregularity falls to 0 at the limit, so the shape no
  longer jumps there, and a giant's regular moons follow the same limits.
- **Tidal shapes.** A locked moon is stretched into the hydrostatic
  ellipsoid of a fluid body, `(b − c) / (a − c) = 1/4` (Noyelles 2017),
  in full from the round limit up.
- **Locking by timescale.** A moon is locked when the planet's tides
  have despun it within the system's age (Gladman et al. 1996, its Love
  number from the rigidity of rock), and the planet to its largest moon
  likewise; the mass ratio of 0.05 of section 4 is gone.
- **Despun planets.** A planet its star has despun keeps only the moons
  light enough to outlast the system (Barnes & O'Brien 2002, eq. 8), not
  none; so does a giant its star has locked, with its own tides. A
  giant's chain ends at a third of its Hill radius. A moon's world is
  locked to its planet, not to the star.
- **A reloaded world keeps its lock.** A reloaded world takes the lock
  its base map was made with, from the `tidally_locked` of its stored
  `world.json` (`WorldParams.locked_to_star`), not the rule above: its
  detail tiles and epochs (the craters of a world locked to its star
  favour its leading hemisphere) then agree with the map and its card.
  Only newly generated worlds follow the new rule.
- **Captured moons** avoid the inclinations of 55° to 130° (Carruba et al.
  2002, via Shen & Tremaine 2008).
- **One recipe for a giant's moons**: a giant of a system draws them as a
  giant world does (two to five, a captured Triton on 35 % of ice
  giants).
- **The odds** are labelled in `moons.rs` as a source or a calibration.
  Elser et al. 2011 give no dependence on the star, so their 1 in 12
  stays the same round every star.

## Sources

- Elser, Stadel, Moore & Morishima 2011, "How common are Earth-Moon
  planetary systems?", Icarus, doi 10.1016/j.icarus.2011.05.025:
  <https://www.arxiv.org/pdf/1105.4616>.
- Barnes & O'Brien 2002, "Stability of Satellites Around Close-in
  Extrasolar Giant Planets", ApJ 575, 1087:
  <https://ar5iv.labs.arxiv.org/html/astro-ph/0205035>.
- "Constraints on the habitability of extrasolar moons":
  <https://arxiv.org/pdf/1210.5172>; "Exomoons in the habitable zones of
  M dwarfs": <https://arxiv.org/pdf/1910.12054>; "Tidally Torn: Why the
  Most Common Stars May Lack Large, Habitable-Zone Moons" (2025):
  <https://arxiv.org/html/2511.03625v1>.
- Canup 2005, "A giant impact origin of Pluto-Charon", Science 307, 546:
  <https://www2.boulder.swri.edu/~robin/canup2005.pdf>; Canup 2011:
  <https://www2.boulder.swri.edu/~robin/canup2011.pdf>;
  arXiv:2511.17832: <https://arxiv.org/html/2511.17832v4>; SwRI on the
  "kiss-and-capture" model:
  <https://www.swri.org/newsroom/press-releases/swri-models-pluto-charon-formation-scenario-mimics-earth-moon-system>.
- Rosenblatt et al. 2016, Nature Geoscience 9, 581 (giant-impact origin
  of Phobos and Deimos), CNRS press release:
  <https://www.cnrs.fr/en/press/giant-impact-solving-mystery-how-mars-moons-formed>.
- Decay of Phobos: "Tidal disruptions of rubble piles: The case of
  Phobos": <https://arxiv.org/html/2602.21912v1>;
  <https://www.iflscience.com/martian-moon-phobos-could-break-into-planetary-rings-sooner-than-we-thought-83399>.
- Brown et al. 2006, "Satellites of the largest Kuiper belt objects",
  ApJ 639, L43: <https://arxiv.org/pdf/astro-ph/0510029>; Barkume, Brown
  & Schaller: <https://www.arxiv.org/pdf/astro-ph/0601534>.
- Hydrostatic equilibrium thresholds:
  <https://en.wikipedia.org/wiki/List_of_possible_dwarf_planets>;
  <https://en.wikipedia.org/wiki/List_of_Solar_System_objects_by_size>.
- Laskar, Joutel & Robutel 1993, "Stabilization of the Earth's obliquity
  by the Moon", Nature 361, 615:
  <https://perso.imcce.fr/jacques-laskar/pdf/Laskar_Joutel_Robutel_1993.pdf>.
