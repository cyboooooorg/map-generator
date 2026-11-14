# Planetary systems: specification

Status: draft for review, 2026-11-05. Written by the two sessions that
pair on the project; the user decides the open points before any code.

## Goals

The generator makes one world today: a planet (or a giant's moon) with
its moons, its drawn giants described in its `world.json`, and the giant
it orbits, all stored under `worlds/<planet>-<seed>/` (the layout before
systems). It becomes a
**system generator**: a star and every body around it, each with its own
property card, the costly part (a body's maps) made only when someone
wants to see that body.

Two ways to ask for a system:

1. **A whole system.** The star (a spectral type, or drawn) and a number
   of planets of each built-in archetype (`earth-like: 2,
   mars-like: 1, jupiter-like: 2…`, or
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
- its **kind** (`planet` or `moon`) and **type**: a drawn planet's
  class, from its physical model (`rocky`, `super-earth`, `ice-world`,
  `ocean-world`, `mini-neptune`, `gas-giant`, …; see
  [Storage](#storage)); the planet a request around one asked for too,
  its archetype (`earth-like`, `mars-like`…, or none) kept beside it for
  display only; `rocky-moon`, `icy-moon` for small moons;
- its **parent** (the star, or a planet) and its **orbit**: semi-major
  axis, eccentricity, inclination, longitude of the node, argument of
  periapsis, mean anomaly at the system's epoch, period (a moon's orbit
  is inclined to its planet's equatorial plane, held fixed against the
  stars, and its node's longitude is counted from the planet's equinox
  direction);
- its **physical facts**: mass, radius, density, surface gravity, shape
  (`axes_km`: an irregular moon's axes, or a round one's stretched by its
  planet's tides);
- its **spin**: rotation period, axial tilt, whether it is locked to its
  parent (and to what), and for a planet whether its star holds it in
  a spin–orbit resonance (`spin_orbit_3_2`, absent when false: three
  turns every two orbits, as Mercury; `resonance_turns`, absent when 0:
  the turns per orbit of any, 1.5 or 2, drawn by its eccentricity,
  see `spin.rs`). A giant on an eccentric orbit turns at the
  pseudo-synchronous rate and is not locked. A locked body turns
  at a steady pace, facing its host on average; the host swings in its
  sky by the libration (true minus mean anomaly, about ±2e radians; the
  dial gives its exact amplitude, `librationAmplitude` in
  `web/src/lib/systemSky.ts`). A
  moon's axial tilt is its world's (`moons::moon_tilt_deg`: the Moon's
  6.68° to its orbit for a giant-impact moon, 0 for the others). A planet
  locked to its star turns on a circular orbit, upright;
- its **surroundings**: equilibrium temperature, the starlight it gets,
  its place relative to the habitable zone, the tides it raises and feels;
- its **preview**: what the viewer draws before its maps exist (see
  [Spheres before maps](#spheres-before-maps));
- its full **world parameters** (`WorldParams`): everything its
  generation needs, fixed when the system is drawn. A giant's include
  its rings (`given_rings`), drawn with the system from the stream
  `RINGS` and its look's odds, so the star view and the skies have
  them before its maps exist and its generation keeps them (backlog
  item 18; absent on systems drawn before, whose giants draw their rings
  from their seed when generated);
- whether its maps are **generated**.

The star carries its **activity** (`star.activity`, `activity.rs`): its
envelope, age, rotation period and differential rotation, convective
turnover time, Rossby number and X-ray activity, its cycle (period, phase
at day 0, depth), the latitudes of its active band, its spot coverage,
facula ratio and the statistics of its active regions (rate, median area,
spread, decay rate), its granulation and limb darkening, and the seed the
viewer draws the regions from (see
[the science](science/star-activity.md)). It is drawn from the system's
seed and the star alone, on its own stream (`STAR_ACTIVITY`). A system
drawn before it existed has none in its file: `read_system` derives it on
read, the same value a new draw stores, and it is saved with the system
the next time its file is written (when a body is generated, for
example).

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

### Where each archetype belongs

A planet asked for by archetype sits where its archetype says: at an
insolation drawn in the archetype's range (`a = sqrt(L / S)`, `S` in
Earth's, so a `venus-like` planet gets 1.7 to 2.1) or, for a hot Jupiter,
at a period drawn in its range (Kepler's third law). It keeps that
orbit unless the spacing pushes it out, up to four times the outer orbit
of its range. Its mass is drawn in the archetype's range on the system's
stream, its other inputs (core share, water, life…) and its axial tilt
on a stream of the body's own (`<id>/archetype`), so that no other body's
draws shift them. Its mass, orbit and eccentricity still come off the
system's stream in request order and ids go by rank, so removing a body
can move the others. The star the archetype names is ignored
here: the system's star rules.

With **quantities per archetype**, the engine draws an orbit for each
planet asked, then spaces them (below). Types the request leaves to
the draw get occurrence rates by star: rocky planets from the Kepler
counts (about one to three small planets within 1 AU of a Sun-like star,
more round M dwarfs; Mulders et al. 2018), giants as `giants.rs` draws
them today (metallicity and star mass), a hot Jupiter included: about
1 % of Sun-like stars of the Sun's metallicity get one (Wright et al.
2012), on a 1 to 10 day orbit.

### Spacing and order

- Planets are placed from the inside out. Two neighbours must be at least
  **10 mutual Hill radii** apart (periapsis to apoapsis) when they weigh
  about an Earth, the spacing that keeps multi-planet systems stable for
  billions of years (Chambers et al. 1996; Pu & Wu 2015). Heavier pairs
  need fewer: the spacing falls as their mass share to the power −1/12
  (Chambers et al. 1996 found stable spacings scale with `μ^¼`, the Hill
  radius with `μ^⅓`), 6.4 for a Jupiter and a Saturn, which are 6.5
  apart; never under `2√3` (Gladman 1993). A pair closer than that is
  spread outwards, a planet pushed out of its allowed band is redrawn a
  few times, then the request fails (below). A planet the draw adds has
  no type and no band: a small one stays within 1.5 times the orbit it
  was drawn at (calibration: it keeps the planet near where its
  formation context put it), a giant within the system; one that finds
  no room there is dropped.
- Neighbouring periods fall mostly between 1.3 and 3 times each other
  (the peas-in-a-pod systems of Weiss et al. 2018): planets of a system
  are of similar size and evenly spaced in `log a`.
- A hot Jupiter clears its neighbourhood: no other planet within a period
  ratio of 4 (Steffen et al. 2012).
- Eccentricities follow Kipping's law for giants, lower in packed systems
  (He et al. 2020), and Van Eylen et al.'s (2019) for the rocky planets of
  a system (mean 0.05); mutual inclinations a degree or two (Fabrycky et al. 2014).
- The letters follow the order from the star (`b`, `c`, `d`…), as the
  naming does today.
- Rocky planets keep out of the first cold giant's belt zone, 0.40 to
  0.63 of its distance, where the giant stops a planet forming: a planet
  there is pushed past it, and the zone follows the giant as it settles.
  A planet fixed by the request may stay in it. A request whose planets
  cannot all fit draws their distances again in their preferred bands
  before it fails.

### Small bodies

Once the planets are placed, the system draws its belts and trojans
(`src/belts.rs`, analysis 13), each from a stream of its own (`BELTS`,
`TROJANS`). The planets and moons of a seed do not move. A giant, here,
is a gas giant, an ice giant or a hot Jupiter
(`PlanetClass::sculpts_belts`): a mini-Neptune, whose world is
generated as a giant's, is too light to shape belts, empty trojan points
or stir impactors.

- **Inner belt.** It needs a giant beyond half the ice line that is not a
  hot Jupiter, the first of them. Its span runs from 0.40 of that giant's
  distance to its 2:1 resonance. The other planets clear 5 Hill radii
  beyond their periapsis and apoapsis, and the widest piece they leave is
  kept if its ratio is 1.15 at least.
  - Its gaps are at the 3:1, 5:2 and 7:3, each `0.012 a √(μ / μ_Jupiter)`
    wide.
  - The Hildas gather in the 3:2, where clear.
  - Its mass is log-uniform from 10⁻⁵ to 10⁻² M⊕.
  - Its composition is rocky, mixed or icy, by where it lies against the
    ice line.
- **Outer belt.** It spans the outermost giant's 3:2 to its 2:1, or 11 to
  18 ice lines without a giant, with the same clearing.
  - It is drawn for half of F, G and K stars, a quarter of M dwarfs and
    three in ten hotter stars.
  - Its mass is log-uniform from 10⁻³ to 1 M⊕.
  - With a giant, a **scattered disk** goes with it, from the giant to
    four times its distance, with 0.3 to 3 times the belt's mass.
- **Trojans.** Bodies larger than 1 km at L4 and L5. A planet has them
  where its mass share is under Routh's 0.0385 and no heavier giant
  orbits within a period ratio of 3 (Mars keeps its trojans next to the
  heavier, rocky Earth).
  - A gas giant, or the outermost ice giant: 10⁴ to 10⁶.
  - Another ice giant: 10³ to 10⁵.
  - A hot Jupiter: up to 10³.
  - A rocky planet: up to 100.
  - L4 holds 0.41 to 0.59 of them.
- **Craters.** Each body gets a crater rate, `WorldParams::impact_factor`:
  the curve of the giant just beyond it (Horner & Jones 2008), times
  `0.8 A + 0.2 C`, a twentieth at least. Here `A` is the inner belt's
  mass against the Sun's 4 × 10⁻⁴ M⊕, and `C` the outer belt's and
  scattered disk's against 0.05 M⊕, each at most 10. A moon takes its
  planet's rate.

`system.json` stores them as `belts` (each one's kind, span, mass, gaps
as centre and width, composition, typical eccentricity and inclination,
and the planet that shapes it) and, on each planet, `trojans` (`l4`,
`l5`). Systems drawn before read with none of them, and their bodies keep
their crater rates.

### Moons

Each planet draws its moons as a world does, by `MoonSystem::draw` in
`moons.rs`: a rocky planet's (impacts, captures, tides, the Hill
sphere), a giant's regular moons (two to five, 1 to 2.5 × 10⁻⁴ of its
mass in a chain or in one dominant moon, a 1:2:4 resonance, ice beyond a
line set by the giant's temperature, on some ice giants a captured
Triton), one recipe for a
giant of a system and a giant world. A giant's large regular
moon may be a habitable world: in mode 2 that is the planet asked for; in
mode 1 a request can ask for it (`moons of giants: earth-like`).

A giant with rings also gets the moons that shape them (`rings.rs`,
backlog item 18), drawn from the stream `RING_MOONS` so the other moons
keep their draws. They are `icy-moon` bodies marked with a `ring_role`:

- `embedded`: zero to two moonlets in the outer 40 % of bright rings,
  each clearing a gap 7 to 17 of its Hill radii wide. Each grew from a
  core of solid ice, 1.5 to 10 km in radius, under a mantle of ring
  particles with 65 % voids, until it filled its Roche lobe: it ends at
  the critical density of its distance, `3M / (γ a³)` (`γ` 1.59), its
  long semi-axis `R_c (4π/(3γ) (ρ_c − ρ_m) / (ρ_crit − ρ_m))^(1/3)`, in
  the lobe's 3 : 2 : 2 shape (Porco et al. 2007): about 4 to 30 km
  across round a Saturn;
- `resonant`: on 60 % of bright rings, a moon of 150 to 250 km in radius
  outside them (irregular below 200 km, round above) whose 2:1 inner
  Lindblad resonance, placed with the giant's `J2`, cuts the edge of the
  inner ring and clears a division 0.04 to 0.09 radii wide outward of it
  (Mimas, the B ring's edge and the Cassini Division);
- `edge`: one to three moons of 10 to 100 km just beyond a bright ring's
  outer edge, the larger the farther out (Charnoz et al. 2010);
- `confining`: on half the bright rings, the largest edge moon, if 40 km
  or more in radius, moved to where its 7:6 inner Lindblad resonance
  falls on the rings' outer edge, which it sets at 0.88 to 0.99 of the
  Roche limit (Janus and the A ring); the embedded moonlets it leaves
  outside are dropped. Bodies with a `ridge` in their params (embedded
  moonlets and edge moons up to 30 km) have an equatorial ridge in their
  shape layer (Pan, Atlas). A bright ring's `rings.d_ring` is an optional
  faint D-like ring (`inner`, `outer`, `opacity`) inside it;
- `shepherd`: on 70 % of faint rings, a pair of 10 to 25 km on either
  side of the ring;
- `source`: beyond a faint ring, one or two small moons of 4 to 90 km at
  2 to 3.2 radii (Amalthea, Thebe), and on half of those giants one of
  5 to 20 km at 3.4 to 4.2 radii (Mab), which may feed dust rings.

Edge moons, shepherds and sources are no less dense than the critical
density at their distance, or solid ice's where that is lower; those
raised to it fill their Roche lobe and take its shape. Where the critical
density exceeds solid ice (close to a dense giant), no loose mantle holds:
the moon is bare solid ice (0.9 g/cm³), irregular, outside its rigid
Roche limit.

Ring moons keep to the rings' plane, which is tens of metres to a kilometre
thick: an embedded moonlet's inclination is drawn up to 0.001° (Pan:
0.0001°, Daphnis: 0.0036°), an edge moon's up to 0.005° (Atlas: 0.003°),
so that from them the rings lie on the horizon; shepherds and resonant
moons, outside the rings, up to 0.05° (Prometheus: 0.008°, Pandora:
0.05°); dust sources up to 1.1° (Thebe: 1.08°). Their eccentricities keep
them as close: an embedded moonlet's up to 5 × 10⁻⁵, a swing of a few km
inside its gap (Pan: 1.4 × 10⁻⁵, Daphnis: 3.3 × 10⁻⁵), scaled down past
Daphnis's distance so that the swing stays as small round a larger giant;
an edge moon's up to 1.5 × 10⁻³ (Atlas: 1.2 × 10⁻³); shepherds and
resonant moons draw 2 × 10⁻⁴ to 2.2 × 10⁻³ (Prometheus: 2.2 × 10⁻³);
dust sources 10⁻³ to 1.8 × 10⁻² (Thebe: 1.8 × 10⁻²). Within bright
rings, gaps never overlap: a moonlet whose gap the resonance's division
would cover is not kept.

Bright rings end at the fluid Roche limit of their icy particles (0.5 to
0.9 g/cm³), from the giant's density over its equatorial radius: 2.15 to
2.62 radii round a Saturn, past 3 round a denser giant. Only physics
bounds them: the rings and their moons stay Hill stable inside the
giant's innermost other moon (Gladman 1993) and within a third of its
Hill radius, and a bright ring keeps 0.3 radii above 1.11 (Saturn's D
ring), or is not kept. Their gaps are those of their moons, and the
rings' `gap_moons` name the moon of each gap. The ring moons are numbered
with the others, nearest first, at their distance in the radii of the
giant's body (`physical.radius_km`, the unit the views place its moons
in), so that an embedded moonlet sits in the middle of its gap. A giant
without regular moons (hot, or with too small a Hill sphere) keeps its
rings without moons. The giant asked for, and the giant a world of the
request orbits, are shaped as the others, inside that world's chain.

Some of those moons feed faint dust rings (`dust_rings`, stream
`DUST_RINGS`), stored with the giant's rings (`given_rings.dust`): on
80 % of them, the one or two outermost edge moons or sources within 3.3
radii each spread a gossamer ring of impact dust from the main rings out
to their orbit and up to 1.26 times past it (Jupiter's Amalthea and Thebe
rings, and Thebe's extension); a source farther out wraps its orbit in a
faint belt from 0.913 to 1.087 times its distance (Uranus's μ ring round
Mab). On a quarter of giants, the innermost icy regular moon within 12
radii blows out a broad ring of ice grains from 0.75 to 2 times its
distance (Saturn's E ring round Enceladus): which moon is active is
drawn, its tidal heating not computed. Each names its source moon.
Systems have no irregular moons, so no ring like Phoebe's.

### Around one planet (mode 2)

The planet asked keeps its orbit and its parameters, and is placed first;
the others are drawn around it with the same spacing, those that would
crowd it moved or dropped. A giant's moon asked for (`host = giant`)
puts the giant on that orbit and the world among its regular moons.

**A lone planet.** With `"alone": true` in the `WorldRequest`, the system
holds only its star, the planet asked and its moons; for a giant's moon,
the giant (with its moons) and nothing else. No giant neighbour, no
rocky sibling, no belt and no trojans are drawn. The planet is resolved
with `giants = none` (a `giants` set in the request is overridden and
listed in `ignored`), so that it is not generated as if giants existed
that the system does not hold; its impact rate is then the Solar
System's, as for any world with `giants = none`. The normal path
(`alone` absent or false) is unchanged, and requests and stored systems
without the field load as before.

### Errors

A request the engine cannot meet answers with a code and the numbers
behind it, as the form's errors do today:

- `too_many_planets`: more than 15;
- `zone_full`: more planets of a type than its band holds at the spacing
  (`{ "type": "earth-like", "asked": 6, "fits": 3 }`);
- `unknown_archetype`: a name that is not a built-in archetype (the old
  type names `terran`, `barren`… are refused);
- `type_impossible`: an archetype the star leaves no room for (its orbits
  inside the inner edge of the system or beyond its outer one);
- the planet's own errors in mode 2, as today.

## Spheres before maps

Until a body is generated, the viewer draws it as a sphere, everywhere a
body is drawn (the star view, the skies of the other bodies, the planet
view when it is picked), lit by its star like the others, with its spin
axis, orbit and labels:

- rocky planets in the colour of the look their model renders with
  (`planet::interim::base_of`): `terran` sea blue with green land and
  white poles, `barren` grey-brown, `caustic` pale yellow under clouds,
  `frozen` white, `volcanic` dark with a red glow on the night side;
- giants with their bands: their palette and band layout come from
  `giant_world::Look`, which is cheap to draw, so a giant sphere already
  looks like its map will;
- moons grey (rock) or pale (ice), irregular ones with their shape.

The system file stores each body's preview (its colours, or its band
palette and layout), so the viewer draws it without knowing the archetypes or looks.
A preview's colours are linear RGB (a giant's sRGB palette is converted
when the preview is made); a giant's `edges` are the latitudes of the
bands it will be generated with, so the preview sphere shows the same
belts and zones as the globe (`bands` is their number; a preview with no
`edges`, made earlier, is drawn with evenly spaced bands).
Such an earlier preview also keeps a giant's sRGB palette read as linear, so
it stays washed out until the system is drawn again.

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
mode 2, the planet asked and its neighbourhood. A system opens on its
star view, whether it was just created or picked in the library, so
nothing else is generated until a body is opened; a link naming a body
(`#system=…&body=…`) opens that body, and one naming pinned cards
(`&pins=c,e`) opens them pinned in the star view.

## Storage

A system is one directory under `systems/` (`MAPGEN_SYSTEMS_DIR`;
`MAPGEN_WORLDS_DIR` no longer exists):

```text
systems/<system-id>/
├── system.json      ← the system as drawn: star, bodies, orbits, previews
├── names.json       ← names given by the user, by body id (optional)
└── bodies/
    ├── c/           ← a generated body
    │   ├── thumb.png       ← from its original resolution
    │   └── sizes/
    │       ├── 2048x1024/  ← a resolution: the files a world has
    │       │   ├── world.png, legend.png (world.svg only from the CLI)
    │       │   ├── raw_data/   ← world.json, layers, rivers…
    │       │   └── noise_maps/ ← diagnostic images (optional: CLI, or the server with MAPGEN_NOISE_MAPS)
    │       └── 4096x2048/  ← another one, generated since
    ├── c-1/
    └── e/
```

- **System id**: the star's designation and the seed, lowercase with
  dashes: `mgs-48213-42`. It reads as `<anything>-<digits>`, as world ids
  do today, so the same id check applies. A lone planet's system adds
  `lone` before the seed (`mgs-48213-lone-42`), so that it never
  collides with the full system of the same star and seed. The same
  request gives the same system and id (see [Model](#model)): creating
  it again is refused (409) unless the request asks to replace it, as
  worlds are today.
- **Body id**: derived from the body's place, path-safe and stable once
  the system is drawn:
  - `star`;
  - a planet: its letter from the star, `b` to `p` (15 planets at most);
  - a moon: its planet's id and its number from the planet outwards,
    `c-1` (designated `MGS 48213 c I`).

  Moons of moons are not modelled. Ids are not names: the designation and
  a given name are kept apart (below).
- **`system.json`** is written once, when the system is drawn, and holds
  everything a card, a sphere, the sky and the star view need:
  - `format_version` (4), the request, the seed and the map size. A
    system of another version is refused, not migrated: version 1 kept a
    body's files directly in `bodies/<id>/`, version 2 typed its planets
    by preset and stored no physical model, version 3 kept a preset in
    each body's world parameters and no model for the planet asked for;
  - `star`: today's `StarInfo` and its designation. `info.age_gyr` is the
    age of the star in Gyr, drawn from the `AGE` stream between 0.5 and
    10 Gyr (uniform; the star's lifetime if shorter) or set by `age_gyr`
    of the request, which must lie in `(0, lifetime]`. Every body shares
    it: `bodies[].params.age_gyr` repeats it. Files without one read as
    4.6 Gyr (`DEFAULT_AGE_GYR`);
  - `bodies`: one entry per planet and moon, with the fields of
    [Model](#model): id, designation, kind, type, parent, orbit, physical
    facts, spin, surroundings, preview, `WorldParams`, and `generated`.
    A planet also stores `model`, its physical model (issue #423: what
    it is made of, its interior, heat, surface, air, climate, life and
    class), derived once when the system is drawn from its orbit, the
    system's age, metallicity and ice line, and the mass drawn for it
    (`model.drawn_mass_earth`), from its own stream (`<id>/model`). Its
    `type` is the model's class (`rocky`, `super-earth`, `ice-world`,
    `mini-neptune`, `ice-giant`, `gas-giant`, `hot-jupiter`,
    `hot-neptune`, `lava-world`, `runaway-greenhouse`, `hydrocarbon-world`,
    `ocean-world`), its `physical.mass_earth` the model's mass after
    photoevaporation; its world renders, until #424, with the look
    closest to the model (`planet::interim::base_of`), and the model of
    a planet with a surface drives the world
    (`planet::interim::params_for`): its size, gravity,
    surface pressure, regime, sky, erosion, volcanism, surface age, the
    sea level its oceans fill, a world without life stripped of its
    forests and grasslands, and a mean temperature aimed at the model's.
    The sea level and the temperature pass over the relief, so they are
    set when the world is generated (`system::generation_params`), not
    when the system is drawn: `bodies[].params` stores the drawn sea
    level and no temperature shift, and the body's `world.json` the
    calibrated ones.
    A planet that is no giant whose envelope hides its surface (at least
    0.1 % of its mass, under 1.6 R⊕: the model derives no surface under
    it) renders with its class's look, `frozen` for an ice world and
    `barren` otherwise, at the model's radius and gravity, with no ocean
    and a mean temperature aimed at a bare surface's (its equilibrium
    plus its internal heat, no greenhouse); the surface under an
    envelope is not derived yet (a later part of #423). About 0.3 % of
    drawn planets (5 of 1 577 over 500 systems).
    A giant class keeps the giant world of its look at the model's
    mass (a mini-Neptune under 5 M⊕ at 5 M⊕). A request asks for
    planets by archetype: an archetype places the planet by its own
    insolation and mass, and the model types it. The planet a request
    around one asks for has its model too (`model`, derived from the
    request's inputs, its archetype's ranges and its formation context),
    its archetype stored beside for display. A moon has no model yet and
    keeps `rocky-moon` or `icy-moon`;

  The server rewrites it (temporary file, then rename) only to set a
  body's `generated` once its directory is complete.
- **A body's directory** holds its thumbnail and, under `sizes/`, one
  directory per resolution named `<width>x<height>`
  (`crates/mapgen/src/sizes.rs`): the original one, at the size of the
  body's parameters, and any other generated since (`map-generator
  --in-system <id> --body <body> --size 4096x2048`, which leaves
  `system.json` as it is). Each holds what a world's holds today. Its
  `world.json` keeps its format with two more fields, `system` and
  `body`. Its `moons`, `giants` and `host` blocks go: the system file
  describes the neighbours, and the generator reads them from it.
- **Writes** keep today's safety: a body is generated into
  `bodies/.tmp-<id>/` and renamed into place once complete, another
  resolution into `bodies/<id>/sizes/.tmp-<W>x<H>/`, and the startup
  sweep removes what a killed server left.
- **`names.json`** keeps the format of the names given today, keyed by
  body id (`{"c": "Gaia", "c-1": "Séléné", "star": "Helia"}`). Every body
  can be named, the star included.
- **The index** (`db/mapgen.sqlite`, `MAPGEN_DB`) is a SQLite database the
  server owns, beside the tree and mounted like it. It lists the systems
  and bodies (what `GET /api/systems` answers), and keeps each body's
  seed and parameters, the generator it was made with (crate version and
  `world.json` format revision), when it was last served or opened, the
  names, and the jobs. The tree stays the source of the files and of
  `system.json`: the index is rebuilt from it when missing and
  reconciled with it at every start (a system is read again when its
  `system.json` or `names.json` modification time differs from the
  recorded one, and dropped when its directory is gone). For now names
  are written to `names.json` as well as to the index, so the tree is
  self-sufficient; the last opened dates and the job history are in the
  database only. WAL journal when the file system allows it (checked at
  every start, with a fallback to the rollback journal and a warning), so
  `db/` holds `mapgen.sqlite`, `-wal` and `-shm`: copy it with the server
  stopped or with `sqlite3 .backup`. A system replaced keeps the names
  that still match one of its bodies.
- **Eviction.** A body not opened for `MAPGEN_EVICT_AFTER_DAYS` days
  (14; 0 never) loses its generated files: all of `bodies/<body>/sizes/*`,
  the original too. It keeps `system.json` (marked `generated: false`),
  its index rows (seed and parameters, displayed size, generator, names,
  pins) and `thumb.png`; `system.json` keeps its modification time, so
  the library does not move the system. The server sweeps at startup and
  every 6 hours, and never evicts a body while a job of its system is
  queued or running (deleting or replacing a system holds the same key,
  so neither races an eviction). The age is checked again under that lock
  just before each eviction: a body opened since the sweep listed it
  stays. A body counts from its last opened date or from when it was
  generated, the later one, and at most from when the index first saw it
  (a rebuilt index does not evict everything old). Only a body's own view
  marks it opened; the textures of other bodies (the sky, the star view's
  markers, the host planet) are fetched with `?peek=1` and do not. An
  invalid `MAPGEN_EVICT_AFTER_DAYS` turns eviction off with a warning.
  The generator recorded is `<crate version>+<git commit>`. An evicted body is "not generated" in the library and
  the system view; opening its view queues its generation, at its
  displayed size when one is remembered, else at the original one, with
  the current generator (see [Generating on demand](#generating-on-demand)).

## API

The world routes give way to system and body routes. A body is
addressed as `<system-id>/<body-id>` everywhere.

- `GET /api/systems`: the systems, most recent first. Each gives its id,
  star, body count per type, the bodies generated, names, map size and
  date, and for each body whether it is a habitable world (`habitable`:
  a planet whose model keeps liquid water on its surface, the planet a
  request around one asked for included), which the index stores.
- `POST /api/systems`: draw a system and queue its first generation.
  The body is a `SystemRequest`:
  - mode 1, `{"system": {"star": "K1V", "planets": {"earth-like": 2,
    "jupiter-like": 1}, "seed": 42, "width": 4096, "height": 2048}}`;
  - mode 2, `{"around": WorldRequest}`, today's request; with
    `"alone": true` in it, a lone planet (see
    [Around one planet](#around-one-planet-mode-2)).

  The answer is `{system_id, job_id}`. The system file exists by then,
  since the draw takes milliseconds, so the library and the viewer show
  the spheres at once. In mode 2 the job generates the planet asked and
  its neighbourhood; in mode 1 there is no job (`job_id` null) until a
  body is opened.
- `GET /api/systems/{id}`: its `system.json`, with the names given and
  `evicted`, the bodies that lost their files (see Storage, *Eviction*).
- `DELETE /api/systems/{id}`: delete a system; 409 while one of its
  bodies is being generated.
- `PUT /api/systems/{id}/names`: replace the names given
  (`{"names": {"c": "Gaia"}}`), with today's rules (1 to 60 characters,
  422 otherwise).
- `PUT /api/systems/{id}/name`: set only the system's own name, the one
  given to its star (`{"name": "Sol"}`; null or blank clears it), merged
  into the names given so that those of the bodies are kept (422 for an
  invalid name).
- `PUT /api/systems/{id}/bodies/{body}/name`: set (or, with null or blank,
  clear) one body's name, `star` included, merged like `/name` (422 for
  an invalid name or an unknown body, 400 for an invalid id).
- `PUT /api/systems/{id}/pin` and `DELETE /api/systems/{id}/pin`: pin or
  unpin a system in the main menu (idempotent, 404 for an unknown
  system). Both answer its summary, whose `pinned` flag `GET /api/systems`
  also gives. The pin lives in the index only.
- `GET /api/systems/{id}/export`: the system as `<id>.zip` (400 for an
  invalid id, 404 for an unknown system, 410 for one an older generator
  made). It holds `<id>/system.json`, a document with a `schema` field
  (1), and the PNGs of the generated bodies: `bodies/<body>/thumb.png`
  and, per resolution on disk, `bodies/<body>/sizes/<W>x<H>/world.png`
  and `legend.png`; tiles, raw data, SVG and noise maps are not in it.
  The document gives the system's `request`, `seed`, `create` (the
  body of the `POST` that draws the system again: the request with the
  seed filled in and, around a planet, the focus body's resolved
  parameters pinned), the star's and the bodies' names, and per body its
  `seed`, its `request` (the `WorldParams` of `system.json`), whether it
  is `generated`, `made_with` (crate version and `format_version`, from
  the index) and its `files`. A body never generated, or whose files are
  gone, is still listed. A system drawn again from `create` is
  identical, a whole system or one around a planet: what a request
  around a planet leaves open comes from the random generator, not the
  seed, so `create` sets each of those fields to the focus body's value.
  Format and rationale: the README, *Export of a system*, and
  `docs/analysis/15-storage.md`, *Export done*. The main menu's
  **Export** button downloads it.
- `POST /api/systems/{id}/bodies/{body}/generate`: queue the body and its
  neighbourhood (see [Generating on demand](#generating-on-demand)) as
  one job. Asking for a body already queued answers that job; asking for
  one generated with all its neighbours answers 200 with no job, so a
  double click costs nothing. For a body evicted and made by another
  build of the generator than the current one, the answer carries
  `regenerated` (one entry per body of the family, the planet and its
  moons, that the job makes again and another build made: its id, the
  version, build id and format recorded, and the current ones), only when
  the job answered is the family's. Bringing evicted bodies back does not
  move their system in the library (`system.json` keeps its date); a job
  that makes a body never generated before dates it as ever. Once the
  job is done, the body asked for, if it was displayed at another size
  than its original, gets that size queued too; its neighbours made by
  the same job (moons, giant, siblings) stay at their original size.
- Per generated body, as today per world:
  - `GET /api/systems/{id}/bodies/{body}/tiles/{z}/{x}/{y}?rev=&epoch=`;
  - `GET /api/systems/{id}/bodies/{body}/epoch/{ma}/{file}`;
  - `GET /api/systems/{id}/bodies/{body}/drift/{ma}/surface.png?width=&rev=`:
    a low-resolution preview of a planet with plate tectonics at
    another epoch of its plates (`ma` from -300 to 300, negative in the
    past; `width` 256 or 512, the default). The server generates the
    planet again from its stored parameters (calibrated once per
    revision as its generation calibrates them, and kept for every
    epoch) with `drift_ma` set, at the preview's size. 404 for a body without plates, 400 out of range,
    503 while a generation runs;
  - `GET /api/systems/{id}/bodies/{body}/thumb?rev=`;
  - `POST /api/systems/{id}/bodies/{body}/hex`.

  The tiles, epoch layers, drift previews and hex grids take
  `?size=<W>x<H>` (`2048x1024`), the resolution to read
  (`bodies/<body>/sizes/<W>x<H>/`); without it, the displayed one (the
  original unless `PUT .../displayed-size` chose another one still on
  disk). A size not generated answers 404, a name that is not a size
  400. Each resolution is a world of its own in the tile cache, with its
  own revision (`rev=`). The thumbnail is the body's, made from its
  original resolution.

  - `GET /api/systems/{id}/bodies/{body}/sizes`: its resolutions
    (`BodySizes`);
  - `POST` / `DELETE /api/systems/{id}/bodies/{body}/sizes/{W}x{H}`:
    queue an offered size (a job, deduplicated) or delete an added one;
  - `PUT /api/systems/{id}/bodies/{body}/displayed-size`: remember the
    size shown, in the index (see the README, *Resolutions*).
- `GET /systems/{id}/...`: the files, `system.json` and
  `bodies/<body>/...`.
- `GET /api/jobs`, `DELETE /api/jobs/{id}` and
  `GET /api/jobs/{id}/events` stay. A job names its system and the bodies
  it generates, in order. `JobStatus::Running` gains the body being made
  and its place in the job:

  ```json
  {"status": "running", "body": "c-1", "body_step": 2, "bodies": 3,
   "stage": "erosion", "step": 3, "steps": 8}
  ```

  `Done` lists the bodies made; `Failed` names the body that failed (the
  ones before it stay generated).
- **Errors** keep `ErrorDetail` codes, so that the web words them in its
  language: those of the draw (`too_many_planets`, `zone_full`,
  `type_impossible`) and the planet's own in mode 2.
- **Revisions** stay per body (the modification time of its
  `world.json`), so a body's tiles survive the generation of another.

## Interface

- **Themes.** The interface comes in five themes (Default, Holo HUD,
  Orbital station, Cyberpunk, Military), picked in the header and
  remembered; the colours that carry meaning (habitable zone, frost line,
  arrows, map layers) keep their hue in all of them.
- **Viewer chrome.** The star view's data is in a dock of panels by
  theme (the star, the planet, the planets), each of which can float over
  the view; its display switches are an icon bar, and its time a playback
  bar at the bottom (play, date, jumps, speed). The planet viewer has the
  same chrome: panels (info, map, light and season, epoch, host planet,
  hexes), an icon bar of its display switches, and one clock in days for
  its seasons and moons. Both views show an axes gizmo; the star view's
  right button pans the camera, and opening a planet flies to it (and
  back out on return).
- **Main menu.** The app opens on a menu, and no viewer starts until a
  system is opened. The menu shows the systems in sections (pinned,
  today since the last local midnight, older; empty ones hidden), sorted
  within each,
  with filter chips (star class from the first letter of the spectral
  type; a habitable world, i.e. a planet whose model keeps liquid water
  on its surface, wherever it orbits; moons, including those of a giant,
  do not count) and a search that matches designations, names and seeds. Three
  views, chosen in the cog's **Display** menu and remembered: cards (an
  orbit strip, the name, the star, a chip per planet type, the moons and
  the date), compact cards (a small orrery on the left) and a table.
  Each card or row has pin, rename, **Export** (the system's `.zip`, see
  [API](#api)) and delete buttons. Selecting several to compare or delete
  works in all three. Opening a
  card opens its viewer, whose **Menu** button goes back; the address is `#menu` (or empty) on the menu and `#system=…` in
  a viewer, and the browser's Back returns from a viewer to the menu. A
  global indicator counts the generations under way and opens their
  list. A card's thumbnail, and the viewer bar's, is a small orrery
  drawn from the summary: the star in its light's colour, the habitable
  zone as a faint green ring, and each planet as a dot on its orbit (an
  ellipse with the star at a focus, on a square-root scale of the
  distance, filling the box), coloured as its preview, giants larger,
  those not generated yet outlined; hovering it gives the planet count
  and the zone, and each dot its planet's name.
- **Wizard.** The menu's **Random system** and **Around a planet** open
  a step-by-step wizard: the kind of creation, the star, the planets (or
  the planet), the advanced options, folded, and a summary. **All
  random**, on the first step, creates a whole system at once.
  - *Random system*: the star (type or drawn), a number per planet type
    (their sum at most 15, each one checked against the star's zones as
    it is typed), metallicity, seed, map size;
  - *Around a planet*: the planet, as before, plus the map size, and a
    **Lone planet (no other planets in its system)** checkbox on the
    planet step: it sends `alone: true`, replaces the giant-planets
    choice with a note and adds a line to the summary. **Vary…** keeps
    the flag of the system it varies.

  The wizard remembers its last mode and fields. **Vary…** opens it over
  the viewer, on its summary.
- **Body picker.** The **Body** list lists the whole system: the star,
  then each planet in order, each followed by its moons, under their
  names, with a mark on those not generated yet. `U` steps through the
  shown body's family (a planet and its moons). The star view becomes a
  **system view**: today's, with every planet drawn as its sphere on its
  orbit (a generated planet wrapped in its map, its thumbnail, one not
  generated yet in its preview's belts, land and caps). Hovering
  a planet rings it and opens its card beside it, linked by a line: name
  and type, radius and mass, semi-major axis and eccentricity, current
  speed, year, age (the system's, spelled out with its unit kept: "8.12 billion years (8.12 Ga)"), day, light, surface gravity, equilibrium temperature,
  place relative to the habitable zone, moons (links to them), and
  whether it is generated, with a button that opens it. A click or a tap
  pins the card, and several pinned cards can be compared side by side,
  each line's highest and lowest value among them marked, **Unpin all**
  closing them; they follow their planets, and last while the system is
  shown, the planet view included, kept in the address (`pins=`) so
  that a reload or a shared link restores them. A double
  click, or the card's button, opens the planet. The planet picked keeps
  its orbit line, its moons and its apsides; the scale (linear to just
  beyond the habitable zone, then compressed) and the clock (in
  Earth days per second, a day a second at the slowest) are the
  system's, so picking another planet moves nothing else.
  Every planet has a dotted line to the star with its distance now
  written along it, a velocity arrow along its orbit labelled with its
  speed now, and an arrow of the star's pull towards the star (**Lines to
  the star**, **Velocities** and **Star's pull** hide them); the
  arrows share one scale (km/s, m/s²) across the planets, so a faster
  planet has a longer arrow, and drawn to scale the longest is bounded
  to half the narrowest gap between the drawn orbits; the Hill spheres
  are drawn to scale there, solid; its name
  sits on the side away from its velocity. The panel's planet list selects a
  planet in place, and pins its card; every planet shows its moons, and
  every orbit its inclination. The view can show the frost line and
  the planets' Hill spheres, be drawn to scale, and be moved to any day
  with a time bar (or to the selected planet's next periapsis). The
  habitable zone is crossed by a dashed line giving its width and area.
- **A body not generated.** Picking it shows its sphere in the planet
  view (lit, with its axis and its sky) and its card, with **Generate**.
  Pressing it, or picking a body whose neighbourhood is not complete,
  queues the job; the view switches to the maps when it ends.
- **A body evicted.** Its card says its maps were deleted after a while
  unopened. Opening its view (the planet view, not the system view or a
  card in a list) queues its generation by itself, as pressing
  **Generate** would. When the generator that made it differs from the
  current one (`regenerated` in the answer of `POST …/generate`, which also
  names the evicted moons or planet a job makes with it), a toast tells
  the user once per body that the world was made by an older version and
  may look different from before. As the generation starts, a toast of
  about 5 seconds tells the user once that the body's maps were deleted
  after a while unopened and are being made again (without the number of
  days); when the older-generator notice applies to the same body, the two
  are one toast. A moon's card shows its planet's, biomes and rivers
  included (read as peeks when the moon is opened first, so that no body
  counts as opened); if they cannot be read the card says why.
- **Progress bar.** While a job runs, the viewer shows a bar over the
  canvas, in the style of the loading bar, with the message of
  [Generating on demand](#generating-on-demand) and **Cancel**. Its share
  is `(body_step − 1 + step / steps) / bodies`. The job list keeps every
  job, named by system and body.
- **Property cards.** One card component for every body, fed by its entry
  in `system.json`, and by its `world.json` and statistics once
  generated:
  - the star: today's star section;
  - a rocky planet: today's world card;
  - a giant: today's giant rows;
  - a moon: today's moon panel.

  A sphere's card gives what the draw fixed (orbit, size, mass, type,
  temperature, spin) and says that its maps are not made yet. The card
  follows the body picker, under a short system summary (star, planet
  strip). **Rename** moves into the card and names any body. The age of
  the system, drawn per system, shows in the star's summary, and on each
  body's card with its surface age (capped at the system's).

## Removing the old worlds

No migration, as decided: the worlds made before systems are deleted.

- Nothing reads, writes or mounts `worlds/` any more: the
  server ignores it (no startup warning), `compose.yaml` does not mount
  it, `MAPGEN_WORLDS_DIR` is gone and so is `mise run clean-worlds`. The
  server never deletes user data; a `worlds/` directory left in a
  checkout can be deleted by hand.
- The routes under `/api/worlds` and `/worlds/` are removed, with no
  compatibility layer: nothing else calls them (the Vite dev proxy no
  longer forwards `/worlds`).
- The code that only served the old layout goes:
  - the world library (`worlds.rs`, in favour of `systems.rs`);
  - the `moons/<n>` and `host/siblings/<n>` paths;
  - the separate moon and host exports, replaced by the generation of a
    body and its neighbourhood;
  - a world's `names.json`.
- The CLI draws a system (mode 1 or 2) into `systems/<id>/`, generates
  the planet asked in mode 2 or `--body <id>`, and prints the system's
  bodies. `mise run gen` and the README examples follow.
- Browser state naming worlds (view links with `#world=`, the remembered
  form) is read once and dropped: an old link opens the library with a
  notice.

### Older formats of a system

The same rule holds for a system written by an older generator: the
generator keeps no code to re-derive it.

- A body whose `raw_data/world.json` is below `MIN_FORMAT_VERSION` (18 at
  the time of writing): its tiles, epochs and hex grids answer 410 Gone
  with a message asking to delete it (`mise run clean-systems`) and draw it
  again, and the viewer shows that message instead of requesting them.
- A `system.json` that no longer parses (the body rules of systems before
  format 11 carry `shrubland_moisture`, now an unknown field):
  `GET /api/systems/{id}` and the generation of its bodies answer 410 Gone
  with the same message, not 404. Only a missing `system.json` is 404.
- The server still starts and lists these systems. One whose
  `system.json` is not of the current `SYSTEM_FORMAT_VERSION` carries
  `outdated: true` in `GET /api/systems` (its id, seed, star and date when
  they parse, no bodies). The main menu labels it "Outdated: made by an
  older generator" and offers only the delete button; the server never
  deletes it itself.

## Delivery

One PR or a few per phase, each merged on green with its tests and docs,
the app usable after each:

1. **The draw** (generator): `SystemRequest`, zones, types and spacing,
   moons, ids and designations, previews, errors; `system.json` written
   by the CLI (`--draw-only`). Tests: determinism, order and spacing, the
   limits and errors, mode 2 keeping its planet exactly. This phase
   freezes the shared types (exported with ts-rs).
2. **Generating from a system** (generator): a body and its
   neighbourhood generated from `system.json` into `bodies/<id>/`, their
   moons, giant and siblings read from the system instead of drawn again;
   the CLI's `--body`.
3. **The server**: the `systems/` library, every route above, jobs per
   neighbourhood with the new progress, per-body revisions; the old
   routes and `worlds/` dropped.
4. **The library and the form** (web): the system list and its planet
   strip, the two-mode form, the API client.
5. **The viewer** (web): the body picker over the system, spheres from
   previews, **Generate** and the progress bar, the system view, the
   skies built from `system.json`.
6. **The cards** (web): one card for every body, renaming any body, the
   system summary.
7. **Clean-up**: dead code, READMEs, backlog, analysis docs, the
   `worlds/` mount, startup warning and `clean-worlds` (done, #303).

Phase 4 can start once phase 1 has frozen the types, beside phases 2 and
3; phases 5 and 6 need phase 3. The two sessions can split as before:
the generator and the server (1 to 3) on one side, the web (4 to 6) on
the other.

## Open points

- Rocky planets' occurrence by star type, and how far to trust Kepler's
  counts outside 1 AU.
- Whether a system can ask for a habitable moon of a giant in mode 1
  (`moons of giants: earth-like`), or only in mode 2.
- Binary stars: out of scope here, kept possible by the outer edge.
