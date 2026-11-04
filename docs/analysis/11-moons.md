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

## 8. Proposed steps

1. Drawing the moons, their data and the star view; no seed changes.
2. Rendering the moons, round then misshapen, in the sky and in a panel.
3. Effects on the planet (axial tilt, tides, binary rotation); seeds
   change.

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
