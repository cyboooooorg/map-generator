# Analysis: extreme trans-Neptunian objects (Sednoids)

Issue #330 (backlog item 23). User request: study the bodies like Sedna,
whose orbits lie far beyond the giants' reach, and decide how systems
draw them (#331), how the star view shows them and whether they get
cards and maps.

**Status (2026-11-17): study written; choices 1, 2 and 4 of section 6
decided by the user, 3 and 5 open.** The
draw (#331) waits for the dwarf-planet class (#496), in the
distribution step of the roadmap (#427).

## 1. What they are

Beyond Neptune, the small bodies fall into a few dynamical families
(Gladman et al. 2008):

| Family          | Orbit                                                         | Neptune's hold                          |
| --------------- | ------------------------------------------------------------- | --------------------------------------- |
| Classical belt  | 37–48 AU, low eccentricity                                    | none today, shaped by it in the past    |
| Resonant        | in a mean-motion resonance (Plutinos at the 3:2)              | locked by it                            |
| Scattering      | perihelion near Neptune's orbit, `a` changing over 10 Myr     | strong: encounters move `a` all along   |
| Detached        | `e > 0.24`, not resonant, not scattering; perihelion ≳ 40 AU   | weak: `a` stays put over 10 Myr         |
| Sednoids        | the far end of the detached: `q > 50` AU and `a > 150` AU     | none: Neptune cannot have placed them   |

- **Detached** objects are those a numerical integration shows neither
  scattered by Neptune nor held in a resonance. Their perihelia mostly
  lie between 38 and 50 AU, just past where Neptune's encounters stop.
  Neptune's own resonances, with the Kozai effect, can lift a perihelion
  up to about 50 AU, so these can come from the scattered disk.
- **Sednoids** are defined by Trujillo & Sheppard (2014) as `q > 50` AU
  and `a > 150` AU, some authors use `q > 60` AU. Their perihelia are out
  of reach of every known planet: something outside the planets must
  have lifted them after Neptune scattered them out. This is why they
  matter: they are fossils of the Sun's surroundings, not of its planets.

### The known ones

| Body                     | `q` (AU) | `a` (AU) | `Q` (AU) | `i`   | Period (yr) | Diameter                       |
| ------------------------ | -------- | -------- | -------- | ----- | ----------- | ------------------------------ |
| 90377 Sedna              | 76       | ~506     | ~937     | 11.9° | ~11 400     | 995 ± 80 km (Herschel)         |
| 2012 VP113               | 80.5     | ~265     | ~450     | 24°   | ~4 300      | ~450 km (albedo 0.15 assumed)  |
| 541132 Leleākūhonua      | 65       | ~1 090   | ~2 100   | 11.7° | ~36 000     | 220 km (stellar occultation)   |
| 2023 KQ14 ("Ammonite")   | 66       | ~252     | ~440     | —     | ~4 000      | a few hundred km, not measured |
| 2000 CR105 (detached)    | ~44      | ~225     | ~410     | 23°   | ~3 400      | ~300 km                        |
| 2004 XR190 (detached)    | ~51      | ~57      | ~63      | 47°   | ~430        | ~500 km                        |

- The `a` of the long orbits changes with the fit; the table rounds the
  values of the Minor Planet Center and of the discovery papers.
- 2004 XR190 shows that detached orbits are not all elongated: its
  perihelion is high, its eccentricity only 0.11, its inclination 47°.
- The Sednoids' inclinations are modest, 10° to 25°. A body pushed by
  a passing star or the cluster's tide keeps roughly the plane it was
  scattered in.

### How many

- **Seen:** four Sednoids, a few dozen detached objects with `q > 40` AU.
- **Bias:** a Sednoid is bright enough to be found only near perihelion.
  Sedna spends about 1.8 % of its orbit inside 100 AU, some 200 years
  out of 11 400. The survey of Rubin's LSST expects Sednoids to be about
  one in a thousand of its trans-Neptunian detections.
- **Estimates of the real population:**
  - Brown (2004), from Sedna's discovery: 40 to 120 bodies of Sedna's
    size on similar orbits.
  - Trujillo & Sheppard (2014), from the sky area searched: about 900
    bodies of Sedna's size or more, which would make the inner Oort
    cloud (`a` from a few hundred to a few thousand AU) more massive
    than the Kuiper belt and the main belt together.
  - Both rest on one to two detections and are uncertain by an order
    of magnitude. For the generator, the population's mass is best
    drawn over a wide range: from a tenth of the Kuiper belt's to a few
    times it.

## 2. Where they come from

Every scenario needs the same first step: a giant (Neptune, Uranus)
scatters planetesimals outwards onto orbits of hundreds of AU. Then
something must raise their perihelia away from the giant before it
scatters them again.

| Scenario                         | How it lifts the perihelion                                                     | Status                                                                                   |
| -------------------------------- | ------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| **Birth cluster**                | the tide of the cluster's gas and stars, and close passages, over its first Myr | generic: Sedna sits at the inner edge of the inner Oort cloud the cluster makes          |
| **One close stellar flyby**      | a single star passing within a few hundred AU                                   | possible, but rare: ≲ 5 % of early encounters meet every constraint (Hu et al. 2025)     |
| **A distant planet (Planet 9)**  | its secular pull, today, still at work                                          | debated: it predicts aligned orbits; 2023 KQ14 does not fit the expected alignment       |
| **A rogue planet, later ejected** | an extra planet of ~2 M⊕ on an eccentric orbit, for ~100 Myr                    | fits the primordial alignment of the three best orbits (Huang & Gladman 2024)            |
| **Galactic tide (Oort cloud)**   | the Galaxy's tide on bodies at `a` ≳ 2 000 AU                                    | works for the classical Oort cloud, too weak today at Sedna's `a`                        |

- **Birth cluster.** Brasser, Duncan & Levison (2006) reproduce Sedna in
  embedded clusters of central density above about 10⁴ M☉ pc⁻³; Kaib &
  Quinn (2008), in open clusters of more than about 30 stars pc⁻³;
  Brasser et al. (2012) find Sedna a generic outcome once the cluster's
  gas is accounted for. Nesvorný et al. (2023) find that the radial
  distribution of distant objects also points to a birth cluster.
- **Flyby.** Morbidelli & Levison (2004) and Kenyon & Bromley (2004)
  first proposed it. Field stars do not pass close enough often enough,
  and a flyby inside the cluster does not give the observed alignment
  (Hu et al. 2025).
- **Distant planet.** Batygin & Brown (2016) proposed it; Batygin et al.
  (2019) put it at 5 to 10 M⊕ and 400 to 800 AU. The clustering of the orbits it explains may be a survey bias.
  Not found so far.

### What each implies for other stars

- **Most stars form in clusters or associations** (Lada & Lada 2003), so
  the birth-cluster scenario makes Sednoids a common outcome, not a
  coincidence of the Sun. How many depends on the cluster's density and
  on how long the star stays in it.
- **They need a scatterer.** Without a giant (or an ice giant) beyond
  the ice line, nothing sends planetesimals to hundreds of AU: no
  Sednoids. A system with a scattered disk has the first step.
- **Low-mass stars.** An M dwarf's planetesimals are bound more weakly
  and its giants are rarer: fewer Sednoids, scaled to closer orbits.
  The radius where a star's pull and a cluster's tide balance grows as
  the cube root of the star's mass.
- **Distant planets and rogue planets** are chance events of one
  system's history: they would make an exception, not a rule.

### Constraints from observation

None direct. A body of 1000 km at 100 AU from another star is far below
any telescope's reach, and exo-Kuiper belts are seen only through their
dust (analysis 13, § 1). The only indirect hint is interstellar objects:
1I/ʻOumuamua (2017), 2I/Borisov (2019) and 3I/ATLAS (2025) show that
planetesimals are ejected from other systems in large numbers, about 0.1
to 0.2 per cubic AU of space (Do et al. 2018), as expected if outer
reservoirs like the Oort cloud are common. They do not measure an inner
Oort cloud.

## 3. Sizes and surfaces

- **Sizes.** The four Sednoids are 200 to 1000 km across. Sedna, at
  about 1000 km, is large enough to be round; the others are near or
  below the limit (about 400 km for icy bodies). The size distribution
  is expected to follow the Kuiper belt's, steep below 100 km.
- **Colour.** Sedna is among the reddest bodies in the Solar System
  (ultra-red), with an albedo of 0.32. 2012 VP113 is moderately red.
- **Composition.** JWST found ethane, acetylene, ethylene, water ice and
  perhaps CO₂ on Sedna (Emery et al. 2024). Ethane and its neighbours
  are made when cosmic rays and solar wind break methane: Sedna, the
  farthest of the three bodies observed, shows the most of them. Methane
  must be resupplied to the surface, which hints at internal melting in
  the past.
- **Temperature.** About 30 K at Sedna's perihelion, about 9 K at its
  aphelion (equilibrium, Bond albedo 0.2). Nitrogen and methane frosts
  are stable; a thin atmosphere near perihelion is possible but not
  observed.
- **Never heated by the planets.** They never come near the planets
  again after their perihelion is raised: their surfaces are the most
  pristine of any large body.

## 4. What our generator does today

| Piece                 | Today                                                                                                       | Missing                                          |
| --------------------- | ----------------------------------------------------------------------------------------------------------- | ------------------------------------------------ |
| Planets               | drawn from 0.02 to 60 AU (`OUTER_EDGE_AU`, `src/system.rs`)                                                  | —                                                |
| Belts                 | inner, Hildas, outer, scattered (`crates/mapgen/src/belts.rs`, `stream::BELTS`)                              | nothing beyond the scattered disk                |
| Scattered disk        | from the last giant's orbit to 4 times it, `e` = 0.3, `i` = 15°, drawn only with an outer belt and a giant  | eccentricity stored but not drawn                |
| Small bodies          | belts are records (range, mass, gaps), not bodies; no orbit, perihelion or aphelion for a small body        | individual distant bodies with orbits            |
| Classes               | 14 planet classes, mass floor 0.05 M⊕ (`planet/mass.rs`)                                                     | dwarf planets (#496): Sedna is ~0.0002 M⊕        |
| Cards and maps        | only `Body` entries, planets and moons                                                                      | cards for large small bodies                     |
| Star view             | planets on ellipses (`StarView.ts`), belts as point clouds on a log scale (`smallBodies.ts`)               | orbits reaching hundreds of AU                   |

The pieces are there: the scattered disk is the source population, the
star view already draws eccentric planet orbits, and a new stream keeps
existing systems unchanged.

## 5. Proposal

### Data

A new list in `system.json`, drawn from a stream of its own
(`stream::DETACHED`, a new id) after every other draw, so that planets,
moons and belts stay as they are. `SYSTEM_FORMAT_VERSION` goes up.

```text
system.detached: {
  mass_earth,                 // the whole population, mostly unseen
  bodies[]: { id, name, diameter_km, albedo, colour,
              orbit: { a_au, eccentricity, inclination_deg,
                       node_deg, periapsis_deg, mean_anomaly_deg } }
}
```

### Rules

- **Only with a scatterer.** Drawn only when the system has a scattered
  disk, that is an outer belt and a giant beyond the ice line. No giant,
  no Sednoids.
- **Birth cluster.** Half of these systems get a population (the star
  was born in a cluster dense enough). This is a guess with no
  observation behind it, chosen to keep Sednoids common but not
  universal.
- **Population mass:** log-uniform from 0.1 to 3 times the outer belt's.
- **Bodies drawn one by one:** the largest 1 to 6, the largest from 300
  to 1500 km (log-uniform), the next ones following a steep size
  distribution, down to about 200 km. The rest stays in `mass_earth`.
  The system creation form gets a field for this count (empty: drawn
  from 1 to 6).
- **Orbits,** scaled by the outermost giant `a_g` (30 AU for Neptune) and
  by the star's mass `M` (in M☉):
  - perihelion `q` from 1.65 to 2.7 `a_g` (50 to 80 AU for the Sun);
  - semi-major axis `a` log-uniform from 150 to 1500 AU times `M^(1/3)`,
    and at least 2 `q`;
  - inclination 5° to 30°; node and argument of perihelion uniform (no
    clustering, since we draw no distant planet);
  - mean anomaly uniform: most bodies sit far out, near aphelion, as in
    reality.
- **Surfaces:** albedo 0.15 to 0.35, colour from red to ultra-red.
- **Impacts:** none; they never cross the planets' orbits.

### Viewers

- **Star view.**
  - The drawn bodies appear on their ellipses, with the code the
    planets' orbits already use (`orbitDistance`, star at a focus).
    Their orbits follow the existing "Orbits" toggle
    (`StarView.setOrbitVisible`), like every other orbit: no switch of
    their own. Their markers show with the planets'.
  - The system keeps today's frame, set by the planets: the detached
    orbits run out beyond it on the logarithmic scale, and the camera's
    zoom range grows to take in their aphelia.
  - The population is not drawn as a cloud: it would be invisible
    points spread over thousands of AU. The card states its mass.
  - On the logarithmic scale, perihelion and aphelion stay readable
    together; in the real-scale mode, the planets shrink to a dot near
    the star, which is the honest picture.
- **Cards.** Each drawn body gets a card: orbit (`q`, `a`, `Q`, `i`,
  period), size, albedo, colour, current distance, and a line on the
  scenario ("perihelion lifted by the birth cluster"). Bodies above about
  400 km are dwarf planets and take that class from #496.
- **Maps.** Not in #331. A map needs the small-body world model that
  #496 decides (icy dwarf planets: relief, ices, colours). Once #496
  can generate a dwarf planet, large Sednoids reuse it.
- **Planet view.** Nothing: they are invisible from any planet.

## 6. Choices for the user

1. **Frequency:** half of the systems with a scattered disk. _Decided
   (2026-11-17)._
2. **How many bodies drawn:** the largest 1 to 6, with a field in the
   creation form to set the count. _Decided (2026-11-17)._
3. **Distant planet:** leave Planet-9-like planets out (proposal), or
   draw one sometimes, with the orbits it aligns?
4. **Star view:** their orbits under the existing "Orbits" toggle, with
   every other orbit. _Decided (2026-11-17)._
5. **Cards and maps:** cards for every drawn body and maps later through
   #496 (proposal), or maps from the start?

## Sources

- Gladman, Marsden & VanLaerhoven 2008, "Nomenclature in the Outer
  Solar System", in The Solar System Beyond Neptune; detached objects:
  <https://en.wikipedia.org/wiki/Detached_object>.
- Sednoids and their orbits: <https://en.wikipedia.org/wiki/Sednoid>;
  <https://en.wikipedia.org/wiki/List_of_Solar_System_objects_most_distant_from_the_Sun>.
- Pál et al. 2012, "TNOs are Cool VII: Sedna and 2010 EK139", A&A 541,
  L6 (Sedna's size and albedo): <https://arxiv.org/pdf/1204.0899>.
- Trujillo & Sheppard 2014, "A Sedna-like body with a perihelion of 80
  astronomical units", Nature 507, 471; Carnegie summary:
  <https://sites.google.com/carnegiescience.edu/sheppard/inner_oort_cloud>.
- Buie et al. 2020, "A single-chord stellar occultation by the extreme
  trans-Neptunian object (541132) Leleākūhonua", PSJ:
  <https://arxiv.org/abs/2011.03889>.
- Chen et al. 2025, "Discovery and dynamics of a Sedna-like object with
  a perihelion of 66 au", Nature Astronomy (2023 KQ14); summary:
  <https://www.astronomy.com/science/new-sednoid-ammonite-discovery-deepens-planet-nine-mystery/>.
- Schwamb, Brown & Rabinowitz 2009, "A Search for Distant Solar System
  Bodies in the Region of Sedna", ApJL 694, L45:
  <https://arxiv.org/abs/0901.4173>.
- Brasser, Duncan & Levison 2006, "Embedded star clusters and the
  formation of the Oort Cloud", Icarus 184, 59; Brasser et al. 2012,
  "Reassessing the formation of the inner Oort cloud in an embedded star
  cluster": <https://arxiv.org/abs/1110.5114>.
- Kaib & Quinn 2008, "The formation of the Oort cloud in open cluster
  environments", Icarus 197, 221: <https://arxiv.org/abs/0707.4515>.
- Nesvorný et al. 2023, "Radial distribution of distant trans-Neptunian
  objects points to Sun's formation in a stellar cluster":
  <https://arxiv.org/abs/2308.11059>.
- Morbidelli & Levison 2004, "Scenarios for the origin of the orbits of
  the trans-Neptunian objects 2000 CR105 and 2003 VB12 (Sedna)", AJ 128,
  2564; Kenyon & Bromley 2004, "Stellar encounters as the origin of
  distant Solar System objects in highly eccentric orbits", Nature 432,
  598.
- Hu, Huang, Gladman & Zhu 2025, "Early stellar flybys are unlikely:
  improved constraints from Sednoids and large-q TNOs":
  <https://arxiv.org/abs/2505.16317>.
- Huang & Gladman 2024, "Primordial orbital alignment of Sednoids",
  ApJL: <https://doaj.org/article/c830eaa5006c451fb1235b1c65a773ec>.
- Batygin & Brown 2016, "Evidence for a distant giant planet in the
  Solar System", AJ 151, 22: <https://arxiv.org/abs/1601.05438>;
  Batygin et al. 2019, "The planet nine hypothesis", Physics Reports
  805, 1: <https://arxiv.org/abs/1902.10103>.
- Emery et al. 2024, "A tale of 3 dwarf planets: ices and organics on
  Sedna, Gonggong, and Quaoar from JWST spectroscopy", Icarus 414,
  116017: <https://arxiv.org/abs/2309.15230>.
- Large-orbit TNOs in LSST and the Sednoids' share of detections:
  <https://arxiv.org/abs/2608.04287>.
- Lada & Lada 2003, "Embedded clusters in molecular clouds", ARA&A 41,
  57.
- Do, Tucker & Tonry 2018, "Interstellar interlopers: number density and
  origin of ʻOumuamua-like objects", ApJL 855, L10; Moro-Martín 2019,
  "Origin of 1I/ʻOumuamua. II. An ejected exo-Oort cloud object?":
  <https://arxiv.org/abs/1811.00023>.
