# Planetary systems: specification

Status: draft for review, 2026-11-05. Written by the two sessions that
pair on the project; the user decides the open points before any code.

## Goals

The generator makes one world today: a planet (or a giant's moon) with
its moons, its drawn giants described in its `world.json`, and the giant
it orbits, all stored under `worlds/<planet>-<seed>/`. It becomes a
**system generator**: a star and every body around it, each with its own
property card, the costly part (a body's maps) made only when someone
wants to see that body.

Two ways to ask for a system:

1. **A whole system.** The star (a spectral type, or drawn) and a number
   of planets of each type (`terran: 2, barren: 1, gas-giant: 2…`, or
   left to the draw), at most 15 planets (moons not counted). The engine
   orders them the way real systems are ordered and draws their moons.
2. **A system around one planet.** A planet fully specified, as the form
   does today (planet type, size, sea level, star, orbit…). The engine
   places it at its orbit and draws the rest of the system around it: how
   many rocky and giant planets, where, and their moons. A giant's moon
   (today's `host = giant`) is this mode with the planet on a giant's
   orbit.

Decisions already taken:

- Planet types are asked as **quantities per type**, not an ordered list:
  the engine orders them.
- **At most 15 planets**, moons not counted.
- **Several habitable planets** may share a system.
- The **map size** of every body is chosen when the system is created
  (moons scaled to their size, giants at a fixed width, as today).
- **No migration**: the worlds made before systems are deleted; the new
  engine regenerates what is wanted.

## Model

A system is a star and a tree of bodies: planets round the star, moons
round planets. Every body has:

- an **id**, path-safe and stable within its system (see
  [Storage](#storage));
- its **designation** (`MGS 48213 c`, `MGS 48213 c II`) and an optional
  given name;
- its **kind** (`planet` or `moon`) and **type**: a preset (`terran`,
  `barren`, `caustic`, `frozen`, `volcanic`, `gas-giant`, `ice-giant`,
  `hot-jupiter`; `rocky-moon`, `icy-moon` for small moons);
- its **parent** (the star, or a planet) and its **orbit**: semi-major
  axis, eccentricity, inclination, longitude of the node, argument of
  periapsis, mean anomaly at the system's epoch, period;
- its **physical facts**: mass, radius, density, surface gravity, shape
  (an irregular moon's axes);
- its **spin**: rotation period, axial tilt, whether it is locked to its
  parent (and to what);
- its **surroundings**: equilibrium temperature, the starlight it gets,
  its place relative to the habitable zone, the tides it raises and feels;
- its **preview**: what the viewer draws before its maps exist (see
  [Spheres before maps](#spheres-before-maps));
- its full **world parameters** (`WorldParams`): everything its
  generation needs, fixed when the system is drawn;
- whether its maps are **generated**.

The physics of today's draws stays where it is (`star.rs`, `giants.rs`,
`moons.rs`, `exomoon.rs`, `giant_world.rs`): the system draw calls them
and records their results. What the viewer draws from a seed today (a
captured moon's spin, a moon's periapsis and node) moves into the
generator and is stored, so the card, the CLI and the viewer agree.

**Determinism.** A system is its request and its seed. Each body's seed
is drawn from the system's seed and the body's id (`stream_seed`), and
its parameters are fixed in the system file at creation, so a body
generated a week later is the world it would have been at once, and
regenerating one body leaves the others alone.

## Drawing a system

### Zones of the star

From the star (its luminosity `L`, temperature, mass, as `StarInfo` has
them):

- the **inner edge**: no planet inside 0.02 AU nor inside the star's
  Roche limit for the planet's density;
- the **habitable zone**: Kopparapu et al. 2013 fluxes, as today;
- the **ice line**, `2.7 √L` AU (Hayashi 1981);
- the **outer edge**: 60 AU, as the giants' draw keeps today, and within
  a stable distance of a companion star if binaries come later.

### Where each type belongs

Each planet type has a band of distances it can sit in and one it prefers
(as a share of the zones above), from which its place is drawn before
spacing:

- `hot-jupiter`: allowed from the inner edge to 0.1 AU, preferred 0.03 to
  0.07 AU;
- `caustic`: allowed inside the habitable zone's inner edge, preferred
  from half that distance to it;
- `volcanic`: allowed inside the ice line, preferred in the inner part of
  the habitable zone and inside it;
- `terran`: allowed in the habitable zone widened by 10 %, preferred in
  the habitable zone;
- `barren`: allowed and preferred anywhere inside the ice line;
- `frozen`: allowed beyond the habitable zone's outer edge, preferred from
  that edge to twice the ice line;
- `gas-giant`: allowed beyond half the ice line (or as a hot Jupiter),
  preferred log-normal around twice the ice line;
- `ice-giant`: allowed beyond the ice line, preferred log-normal around
  five times the ice line.

With **quantities per type**, the engine draws a preferred distance for
each planet asked, then spaces them (below). Types the request leaves to
the draw get occurrence rates by star: rocky planets from the Kepler
counts (about one to three small planets within 1 AU of a Sun-like star,
more round M dwarfs; Mulders et al. 2018), giants as `giants.rs` draws
them today (metallicity and star mass).

### Spacing and order

- Planets are placed from the inside out. Two neighbours must be at least
  **10 mutual Hill radii** apart (periapsis to apoapsis), the spacing that
  keeps multi-planet systems stable for billions of years (Chambers et
  al. 1996; Pu & Wu 2015); a pair closer than that is spread outwards, a
  planet pushed out of its allowed band is redrawn a few times, then the
  request fails (below).
- Neighbouring periods fall mostly between 1.3 and 3 times each other
  (the peas-in-a-pod systems of Weiss et al. 2018): planets of a system
  are of similar size and evenly spaced in `log a`.
- A hot Jupiter clears its neighbourhood: no other planet within a period
  ratio of 4 (Steffen et al. 2012).
- Eccentricities follow Kipping's law, lower in packed systems (He et al.
  2020); mutual inclinations a degree or two (Fabrycky et al. 2014).
- The letters follow the order from the star (`b`, `c`, `d`…), as the
  naming does today.

### Moons

Each planet draws its moons as today: a rocky planet by `moons.rs`
(impacts, captures, tides, the Hill sphere), a giant its regular moons
by the chain `exomoon.rs` draws (two to four, about 10⁻⁴ of its mass, a
1:2:4 resonance), plus captured irregular moons. A giant's large regular
moon may be a habitable world: in mode 2 that is the planet asked for; in
mode 1 a request can ask for it (`moons of giants: terran`).

### Around one planet (mode 2)

The planet asked keeps its orbit and its parameters, and is placed first;
the others are drawn around it with the same spacing, those that would
crowd it moved or dropped. A giant's moon asked for (`host = giant`)
puts the giant on that orbit and the world among its regular moons.

### Errors

A request the engine cannot meet answers with a code and the numbers
behind it, as the form's errors do today:

- `too_many_planets`: more than 15;
- `zone_full`: more planets of a type than its band holds at the spacing
  (`{ "type": "terran", "asked": 6, "fits": 3 }`);
- `type_impossible`: a type the star leaves no room for (no habitable
  zone outside the inner edge, a hot Jupiter round a star too faint…);
- the planet's own errors in mode 2, as today.

## Spheres before maps

Until a body is generated, the viewer draws it as a sphere, everywhere a
body is drawn (the star view, the skies of the other bodies, the planet
view when it is picked), lit by its star like the others, with its spin
axis, orbit and labels:

- rocky planets in the colour of their type: `terran` sea blue with green
  land and white poles, `barren` grey-brown, `caustic` pale yellow under
  clouds, `frozen` white, `volcanic` dark with a red glow on the night
  side;
- giants with their bands: their palette and band layout come from
  `giant_world::Look`, which is cheap to draw, so a giant sphere already
  looks like its map will;
- moons grey (rock) or pale (ice), irregular ones with their shape.

The system file stores each body's preview (its colours, or its band
palette and layout), so the viewer draws it without knowing the presets.

## Generating on demand

Opening a body that is not generated asks the server to generate it and
its neighbourhood, those not generated yet, in one job:

- **a planet**: the planet and its moons;
- **a moon**: the moon, its planet, and the planet's other moons;
- **the star**: nothing to generate (the star view draws it).

The job runs in the generation queue (one at a time, cancellable, its
progress streamed), and the viewer shows the sphere meanwhile, under a
progress bar with an explicit message: which body is being made, which of
its neighbours follow, and the current stage (`Generating MGS 48213 c and
its 2 moons: erosion, 3 of 8`). When it ends, the maps replace the
sphere. Creating a system generates only its file (milliseconds) and, in
mode 2, the planet asked and its neighbourhood, so it opens on that
planet as today.

## Storage

TODO (the other session): `systems/<id>/system.json`, `bodies/<id>/`,
body ids, names.

## API

TODO (the other session): systems, bodies, tiles and epochs per body, the
generate-body job and its progress events.

## Interface

TODO (the other session): the form (whole system or around a planet),
the library, the body picker, the property cards, the progress bar.

## Removing the old worlds

TODO (the other session): deleting `worlds/` and the old routes.

## Delivery

TODO (the other session): the phases, each one shippable.

## Open points

- Rocky planets' occurrence by star type, and how far to trust Kepler's
  counts outside 1 AU.
- Whether a system can ask for a habitable moon of a giant in mode 1
  (`moons of giants: terran`), or only in mode 2.
- Binary stars: out of scope here, kept possible by the outer edge.
