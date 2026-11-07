# Analysis: asteroid belts and Lagrange points

Backlog item 17. User request (2026-11-06): study scientifically asteroid
fields, asteroid belts, and the Lagrange points with the asteroids they
hold in theory.

**Status (2026-11-07): implemented.** The proposal of section 4 is in
the app: `src/belts.rs` draws the belts, their gaps, the trojans and the
crater rate from the belts' mass, the star view draws them with the
Lagrange points (`web/src/lib/viewer/smallBodies.ts`), and rocky planets
keep out of a giant's belt zone. Co-orbital moons are not drawn. Section
3 describes the app before this study.

Before it, a system (`src/system.rs`, `system.json`) held a star, its
planets and their moons. Small bodies did not exist: there was no belt,
no trojan swarm and no Lagrange point anywhere in the data or the
viewers. The only trace of them was the crater rate, which `giants.rs`
set from the mass of the outermost giant (`impact_factor`, after Horner
& Jones 2008).

## 1. What a belt is

A belt is a ring of planetesimals that never grew into a planet, because
a giant planet's resonances stirred them too fast to merge, and then
emptied most of the ring.

| Population      | Where (Solar System)                  | Mass                                         | Made of                                    |
| --------------- | ------------------------------------- | -------------------------------------------- | ------------------------------------------ |
| Main belt       | 2.1–3.3 AU, between Mars and Jupiter  | 4.0 × 10⁻⁴ M⊕, 3 % of the Moon               | rock and metal inside, carbon-rich outside |
| Hildas          | 3.7–4.2 AU, Jupiter's 3:2 resonance   | small, a fraction of a percent of the belt's | dark, primitive                            |
| Jupiter trojans | Jupiter's orbit, 60° ahead and behind | uncertain, well below the main belt's        | dark, primitive (D and P types)            |
| Kuiper belt     | 30–50 AU, beyond Neptune              | about 0.01–0.1 M⊕                            | ices and rock                              |
| Scattered disk  | 30 to hundreds of AU, eccentric       | uncertain, comparable to the Kuiper belt     | ices and rock                              |

- **Mass.** Pitjeva & Pitjev (2018) weigh the main belt from the motions of
  the planets and of spacecraft at (4.008 ± 0.029) × 10⁻⁴ M⊕. Half of that
  is in Ceres, Vesta, Pallas and Hygiea.
- **Depletion.** The belt once held hundreds to thousands of times its
  present mass: enough, at the density of the disk round it, for a Mars or
  two. Jupiter's resonances and its early migration (the Grand Tack,
  Walsh et al. 2011) or the giants' later instability emptied it and
  then partly refilled it, so the belt today is a remnant (Morbidelli et
  al. 2015; Raymond & Izidoro 2017).
- **Composition.** Rocky S types dominate the inner belt and carbon-rich
  C types the outer. This is a trace of the disk's temperature gradient
  across the snow line, blurred by migration: primitive bodies reach the
  inner belt, and they make up more than half the belt by mass once the
  four largest bodies are left out (DeMeo & Carry 2014).
- **Kuiper belt.** Bernstein et al. (2004) put the classical belt at
  about 0.01 M⊕. The cold classicals alone weigh about 0.0014 M⊕ (DEEP V,
  2023). The primordial belt was perhaps a hundred times more massive.

### Structure set by the giant

The belt's edges and gaps are where an asteroid's period is a simple
fraction of the giant's. An asteroid going round `j` times while the giant
goes round `k` times sits at `a = a_giant · (k / j)^(2/3)`.

| Feature (Jupiter)       | Resonance              | Distance    | Share of `a_Jupiter` | What it does                                            |
| ----------------------- | ---------------------- | ----------- | -------------------- | ------------------------------------------------------- |
| Inner edge              | ν6 (secular), near 4:1 | 2.06–2.1 AU | 0.40                 | raises eccentricities until bodies cross Mars and Earth |
| Kirkwood gap            | 3:1                    | 2.50 AU     | 0.48                 | empty gap, the main source of meteorites                |
| Kirkwood gap            | 5:2                    | 2.82 AU     | 0.54                 | empty gap                                               |
| Kirkwood gap            | 7:3                    | 2.95 AU     | 0.57                 | narrow gap                                              |
| Outer edge (Hecuba gap) | 2:1                    | 3.28 AU     | 0.63                 | ends the belt                                           |
| Hildas                  | 3:2                    | 3.97 AU     | 0.76                 | _stable_ resonance: a group gathers instead of a gap    |
| Trojans                 | 1:1                    | 5.2 AU      | 1.0                  | stable at L4 and L5                                     |

- Most resonances of Jupiter empty a gap because their overlap with the
  secular resonances ν5 and ν6 makes orbits chaotic. The 3:2 and the 1:1
  hold bodies instead, because the resonance keeps them away from the
  giant at conjunction.
- The same scheme gives the Kuiper belt its shape, from outside Neptune:
  the Plutinos sit in the 3:2 at 39.4 AU (1.31 `a_Neptune`), and the
  classical belt runs to the 2:1 at 47.7 AU (1.59 `a_Neptune`), where it
  ends.

### Sizes and spacing

- **Count.** About 2 × 10⁶ main-belt bodies are larger than 1 km. The
  number grows steeply towards small sizes, roughly as `D^-2.5` in
  cumulative counts, close to the collisional equilibrium of Dohnanyi
  (1969).
- **Spacing.** Spread over the belt's volume, that leaves about a million
  km between bodies larger than a kilometre, and millions of km between
  those you could see from one another. Pioneer, Voyager, Galileo,
  Cassini and New Horizons crossed the belt without steering round
  anything.
- **"Asteroid fields".** A dense field of tumbling rocks, as science
  fiction shows it, does not exist round a star. The nearest real things
  are young collisional families, which stay bunched along their orbit
  for a few million years, and the rubble of a moon broken inside a
  planet's Roche limit, which becomes a ring (see analysis 14, rings).
  A belt is invisible from inside it, and from a planet the only sign of
  it is a faint zodiacal glow from its dust.

### Belts round other stars

- **Exo-Kuiper belts.** Cold dust belts are seen in the far infrared round
  about one FGK star in five (DUNES: 22 %; DEBRIS: 17 %, about 28 % once
  corrected for completeness), at every age from 0.1 to 10 Gyr, but only
  2 % of M dwarfs (a shallower survey). These are belts tens to hundreds
  of times more massive in dust than the Kuiper belt; ours would not be
  detected.
- **Warm belts**, analogues of the main belt, are rarer and harder to see
  (a few percent of stars show warm dust).
- **Belts and planets.** Belt edges are expected to sit just outside the
  orbit of a giant, where its chaotic zone ends, or at its resonances,
  as in the Solar System. Some imaged belts have sharp edges or gaps
  that point to unseen planets.

## 2. Lagrange points

In the frame turning with a planet, five points balance the star's and
the planet's pulls.

| Point | Where                                                     | Stable?              | What lives there                   |
| ----- | --------------------------------------------------------- | -------------------- | ---------------------------------- |
| L1    | between the star and the planet, at about one Hill radius | no, months to years  | nothing natural; spacecraft (SOHO) |
| L2    | beyond the planet, at about one Hill radius               | no                   | nothing natural; spacecraft (JWST) |
| L3    | opposite the planet, behind the star                      | no                   | nothing                            |
| L4    | on the orbit, 60° ahead of the planet                     | yes, if `μ < 0.0385` | trojans                            |
| L5    | on the orbit, 60° behind the planet                       | yes, if `μ < 0.0385` | trojans                            |

- **Stability.** L4 and L5 are linearly stable while the planet's share of
  the mass `μ = m / (M + m)` is below Routh's value, `½ (1 − √(23/27))` =
  0.0385. Every planet meets this, up to about 40 Jupiter masses round the
  Sun, so every planet has two stable points.
- **Swarm shape.** A trojan does not sit at the point: it librates round
  it on a _tadpole_ orbit. In longitude, that runs from about 30° to more
  than 90° from the planet; in distance, it is as wide as `√(8μ/3) · a`
  each side of the orbit (Murray & Dermott 1999). Wider librations become
  _horseshoe_ orbits, which take in L3, L4 and L5 together and are less
  stable.
  - Jupiter (`μ` = 9.5 × 10⁻⁴): ±0.05 `a`, ±0.26 AU. The swarms also
    spread with inclinations up to 40°.
  - Earth (`μ` = 3 × 10⁻⁶): ±0.0028 `a`, about ±420 000 km.

### Trojans in the Solar System

| Planet  | Known trojans            | Notes                                                                                        |
| ------- | ------------------------ | -------------------------------------------------------------------------------------------- |
| Jupiter | more than 10 000 known   | about 2.6 × 10⁵ larger than 1 km, a tenth of the main belt; L4 holds 1.4 times as many as L5 |
| Neptune | about 30                 | thought to be as many as Jupiter's, or more                                                  |
| Mars    | 16, 15 of them stable    | mostly at L5, some of them fragments of one body                                             |
| Uranus  | two, both temporary      | Saturn's and Jupiter's pulls destabilize them                                                |
| Earth   | two (2010 TK7, 2020 XL5) | temporary, a few thousand years; tens to hundreds expected, not thousands                    |
| Saturn  | none stable              | Jupiter, near a 5:2 resonance, empties its L4 and L5                                         |
| Venus   | one temporary co-orbital | —                                                                                            |

- **Origin.** Jupiter's trojans are not leftovers from its own orbit. They
  are dark, primitive bodies captured from farther out when the giants'
  orbits shifted (the Nice model: Morbidelli et al. 2005; Nesvorný et
  al. 2013). This is why their colours match the Kuiper belt's rather
  than the main belt's.
- **Who keeps them.** A planet keeps a large swarm when no more massive
  planet perturbs it. This is the case for Jupiter, and for Neptune,
  which is the outermost planet. Saturn and Uranus, next to a heavier
  neighbour, have none or only passing ones. The terrestrial planets'
  swarms are tiny and mostly temporary.
- **Moons have them too.** Saturn's moon Tethys has Telesto at its L4 and
  Calypso at its L5; Dione has Helene and Polydeuces. These are the only
  known co-orbital moons.
- **Round other stars.** No exotrojan is confirmed. The best candidate is
  ALMA emission at L5 of the protoplanet PDS 70 b, holding 0.03 to 2 lunar
  masses of dust (Balsalobre-Ruza et al. 2023, about 4σ, to be confirmed
  by the orbit's motion after 2026). Co-orbital planets are allowed by
  dynamics but none is known.

## 3. What our generator did, and what was missing

| Piece              | Today                                                 | Missing                                                                  |
| ------------------ | ----------------------------------------------------- | ------------------------------------------------------------------------ |
| Belts              | none                                                  | an inner belt inside the first giant, an outer one beyond the last       |
| Belt structure     | —                                                     | edges and gaps at the giant's resonances                                 |
| Trojans            | none                                                  | swarms at L4 and L5, sized by the planet and its neighbours              |
| Lagrange points    | not drawn                                             | the five points of a selected planet, or of a moon round its planet      |
| Co-orbital moons   | none                                                  | small moons at L4 or L5 of a giant's moon (Telesto, Calypso)             |
| Impacts            | `impact_factor` from the outermost giant's mass alone | the belt's mass and the giant's resonances feeding impactors             |
| Spacing of planets | 10 mutual Hill radii, peas in a pod                   | a wide gap left between the rocky planets and a giant, where a belt lies |

Our spacing rule packs planets evenly in `log a`. A belt in a generated
system needs room between the last rocky planet and the first giant, and
the draw does not leave that room on purpose.

## 4. First proposal

These are additions to `system.json`, drawn from a stream of their own
(`stream::BELTS`) after every existing draw. With this, existing systems
keep every planet and moon they have, and only gain belts and swarms.

### Data

```text
system.belts[]: { kind: "inner" | "outer", inner_au, outer_au,
                  mass_earth, gaps_au[], composition: "rocky" | "mixed" | "icy" }
planet.trojans: { l4: count, l5: count }   // bodies larger than 1 km
```

### Rules

- **Inner belt.** Drawn when the first giant beyond the ice line leaves
  room inside it.
  - It runs from the outside of the last rocky planet's chaotic zone (or
    the giant's ν6-like edge, `0.40 a`) out to the giant's 2:1 (`0.63 a`).
  - Its gaps are at the 3:1, 5:2 and 7:3; its Hildas at the 3:2.
  - Its mass is log-uniform from 10⁻⁵ to 10⁻² M⊕, the Solar System's near
    the middle.
  - Its composition goes from rocky to carbon-rich across the ice line.
  - A system without a giant gets no belt: its planetesimals grew into
    planets.
- **Outer belt.** Beyond the outermost giant, from its 3:2 (`1.31 a`) to
  its 2:1 (`1.59 a`), with a few percent of its mass spread farther out.
  - It is icy, with a mass of 10⁻³ to 1 M⊕.
  - One FGK system in four gets one, and one M system in ten. These are
    observed rates; real belts are more common, since ours would go
    undetected.
- **Trojans.** A planet gets swarms when no more massive giant orbits
  within a period ratio of 3 of it. A rocky neighbour does not count: Mars
  keeps its trojans (16 known, 15 stable, Christou et al. 2025) next to
  the heavier Earth.
  - A giant: 10⁴ to 10⁶ bodies larger than 1 km, the L4 swarm a little
    richer.
  - An ice giant on the outside: as many as a gas giant.
  - A rocky planet: 0 to 100, short-lived.
- **Moons.** A giant's regular moon of more than about 10²⁰ kg sometimes
  holds a small moon at L4 or L5 (backlog item 13). This is the moon of
  the moons' study, not this one.
- **Impacts, later.** The crater rate could read the inner belt's mass
  besides the giant's. This changes existing worlds, so it is a decision
  of its own: off by default.

### Viewers

- **Star view.**
  - **Belts:** a toggle drawing each belt as a cloud of points, thin in
    the orbital plane and spread a little in height, with its gaps and its
    Hildas visible. A card gives its range, mass, composition and what
    shapes it ("edge at Jupiter's 2:1").
  - **Lagrange points:** a toggle showing L1–L5 of the selected planet as
    small marks turning with it, its trojan swarms as two banana-shaped
    clouds of points round L4 and L5, and their counts on the card.
  - The points are drawn far denser than any real belt and the help says
    so, like the planets' markers, which are not to scale either.
- **Planet view.** A belt is not visible from inside a system. At most
  there is a faint zodiacal band along the ecliptic, from the belt's dust,
  and only if the user asks for it.

## 5. Choices for the user

1. **Belts:** inner only, inner and outer, or also a scattered disk?
2. **Trojans:** counts on the cards only, or drawn swarms as well?
3. **Lagrange points:** all five marked for the selected planet, or only
   L4 and L5 with their swarms?
4. **Impacts:** keep `impact_factor` as it is (worlds unchanged), or let
   the belt's mass weigh in (craters change on new worlds)?
5. **Spacing:** leave the planets' draw as it is (a belt only where room
   happens to be left), or open a gap inside the first giant on purpose
   (changes new systems' planets)?

## Sources

- Pitjeva & Pitjev 2018, "Masses of the Main Asteroid Belt and the Kuiper
  Belt from the Motions of Planets and Spacecraft", Astronomy Letters:
  <https://arxiv.org/pdf/1811.05191>.
- Walsh et al. 2011, "A low mass for Mars from Jupiter's early
  gas-driven migration", Nature 475, 206; review: "The Grand Tack model:
  a critical review": <https://arxiv.org/pdf/1409.6340>.
- Morbidelli et al. 2015, "The dynamical evolution of the asteroid belt",
  Asteroids IV; Raymond & Nesvorný 2020, "Origin and dynamical evolution
  of the asteroid belt": <https://arxiv.org/pdf/2012.07932>.
- DeMeo & Carry 2014, "Solar System evolution from compositional mapping
  of the asteroid belt", Nature 505, 629; companion paper:
  <https://www.arxiv.org/abs/1307.2424>.
- Kirkwood gaps and their resonances:
  <https://en.wikipedia.org/wiki/Kirkwood_gap>;
  <https://asteroidday.org/resources/event-resources/learn-what-are-kirkwood-gaps/>.
- Bernstein et al. 2004, the classical Kuiper belt's mass:
  <https://arxiv.org/abs/astro-ph/0308467>; DEEP V (2023), the cold
  classicals: <https://arxiv.org/pdf/2309.09478>.
- Spacing of the main belt: Lucy mission, "The Density of the Asteroid
  Belt": <https://lucy.swri.edu/MainBeltDensity.html>;
  <https://www.astronomy.com/science/how-do-spacecraft-avoid-collisions-in-the-asteroid-belt/>.
- Debris disk incidence: Montesinos et al. 2016 (DUNES),
  <https://web3.arxiv.org/abs/1605.05837>; DEBRIS and the disk
  connection: <https://arxiv.org/pdf/2111.06406>; M dwarfs:
  <https://www.arxiv.org/pdf/2502.04441>.
- Routh's criterion for L4 and L5:
  <https://farside.ph.utexas.edu/teaching/celestial/Celestial/node86.html>;
  tadpole widths after Murray & Dermott 1999, in "New asteroid clusters
  and evidence of collisional fragmentation in the L5 Trojan cloud of
  Mars": <https://arxiv.org/pdf/2502.16231>; Dermott & Murray 1981, "The
  dynamics of tadpole and horseshoe orbits", Icarus 48.
- Jupiter trojan counts: "Size Distribution of Small Jupiter Trojans in
  the L5 Swarm" (2022): <https://arxiv.org/pdf/2204.08617>; Dotto et al.,
  "The Trojans in the Planetary System":
  <https://www-n.oca.eu/morby/papers/Trojans.pdf>.
- Trojans of other planets: <https://en.wikipedia.org/wiki/Trojan_(celestial_body)>;
  Earth's second trojan, ESA:
  <https://www.esa.int/Space_Safety/Planetary_Defence/Newly_discovered_asteroid_just_second_of_its_kind>.
- Origin of Jupiter's trojans: Morbidelli et al. 2005, "Chaotic capture of
  Jupiter's Trojan asteroids in the early Solar System", Nature 435, 462;
  Nesvorný, Vokrouhlický & Morbidelli 2013, ApJ 768, 45.
- PDS 70 b's L5: Balsalobre-Ruza et al. 2023, A&A,
  <https://arxiv.org/abs/2307.12811>.
- Horner & Jones 2008, "Jupiter – friend or foe? I: The asteroids",
  International Journal of Astrobiology 7, 251.
