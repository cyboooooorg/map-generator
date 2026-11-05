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
mode 2, the planet asked and its neighbourhood. A system opens on its
star view, whether it was just created or picked in the library, so
nothing else is generated until a body is opened; a link naming a body
(`#system=…&body=…`) opens that body.

## Storage

A system is one directory under `systems/` (`MAPGEN_SYSTEMS_DIR`, in place
of `MAPGEN_WORLDS_DIR`):

```text
systems/<system-id>/
├── system.json      ← the system as drawn: star, bodies, orbits, previews
├── names.json       ← names given by the user, by body id (optional)
└── bodies/
    ├── c/           ← a generated body: the files a world has today
    │   ├── world.png, world.svg, legend.png, thumb.png
    │   └── raw_data/   ← world.json, layers, rivers…
    ├── c-1/
    └── e/
```

- **System id**: the star's designation and the seed, lowercase with
  dashes: `mgs-48213-42`. It reads as `<anything>-<digits>`, as world ids
  do today, so the same id check applies. The same request gives the
  same system and id (see [Model](#model)): creating it again is refused
  (409) unless the request asks to replace it, as worlds are today.
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
  - `format_version` (1), the request, the seed and the map size;
  - `star`: today's `StarInfo` and its designation;
  - `bodies`: one entry per planet and moon, with the fields of
    [Model](#model): id, designation, kind, type, parent, orbit, physical
    facts, spin, surroundings, preview, `WorldParams`, and `generated`.

  The server rewrites it (temporary file, then rename) only to set a
  body's `generated` once its directory is complete.
- **A body's directory** holds what a world's holds today. Its
  `world.json` keeps its format with two more fields, `system` and
  `body`. Its `moons`, `giants` and `host` blocks go: the system file
  describes the neighbours, and the generator reads them from it.
- **Writes** keep today's safety: a body is generated into
  `bodies/.tmp-<id>/` and renamed into place once complete, and the
  startup sweep removes what a killed server left.
- **`names.json`** keeps the format of the names given today, keyed by
  body id (`{"c": "Gaia", "c-1": "Séléné", "star": "Helia"}`). Every body
  can be named, the star included.

## API

The world routes give way to system and body routes. A body is
addressed as `<system-id>/<body-id>` everywhere.

- `GET /api/systems`: the systems, most recent first. Each gives its id,
  star, body count per type, the bodies generated, names, map size and
  date.
- `POST /api/systems`: draw a system and queue its first generation.
  The body is a `SystemRequest`:
  - mode 1, `{"system": {"star": "K1V", "planets": {"terran": 2,
    "gas-giant": 1}, "seed": 42, "width": 4096, "height": 2048}}`;
  - mode 2, `{"around": WorldRequest}`, today's request.

  The answer is `{system_id, job_id}`. The system file exists by then,
  since the draw takes milliseconds, so the library and the viewer show
  the spheres at once. In mode 2 the job generates the planet asked and
  its neighbourhood; in mode 1 there is no job (`job_id` null) until a
  body is opened.
- `GET /api/systems/{id}`: its `system.json`, with the names given.
- `DELETE /api/systems/{id}`: delete a system; 409 while one of its
  bodies is being generated.
- `PUT /api/systems/{id}/names`: replace the names given
  (`{"names": {"c": "Gaia"}}`), with today's rules (1 to 60 characters,
  422 otherwise).
- `POST /api/systems/{id}/bodies/{body}/generate`: queue the body and its
  neighbourhood (see [Generating on demand](#generating-on-demand)) as
  one job. Asking for a body already queued answers that job; asking for
  one generated with all its neighbours answers 200 with no job, so a
  double click costs nothing.
- Per generated body, as today per world:
  - `GET /api/systems/{id}/bodies/{body}/tiles/{z}/{x}/{y}?rev=&epoch=`;
  - `GET /api/systems/{id}/bodies/{body}/epoch/{ma}/{file}`;
  - `GET /api/systems/{id}/bodies/{body}/thumb?rev=`;
  - `POST /api/systems/{id}/bodies/{body}/hex`.
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

- **Library.** The world list becomes a system list. Each row shows the
  star's designation (or given name), a strip of small spheres (one per
  planet, in order, coloured as their previews, the generated ones
  marked), the count per type and the date. Search matches designations,
  names and seeds; selecting several and deleting work as today. A row's
  thumbnail, and the compare bar's, is a small orrery drawn from the
  summary: the star in its light's colour, the habitable zone as a faint
  green ring, and each planet as a dot on its orbit (a square-root scale
  of the semi-major axis, filling the box), coloured as its preview,
  giants larger, those not generated yet outlined.
- **Form.** **New system** asks for a mode first:
  - *A whole system*: the star (type or drawn), a number per planet type
    (their sum at most 15, each one checked against the star's zones as
    it is typed), metallicity, seed, map size;
  - *Around a planet*: today's form, unchanged, plus the map size.

  The form remembers its last mode and fields, as it does today.
- **Body picker.** The **Body** list lists the whole system: the star,
  then each planet in order, each followed by its moons, under their
  names, with a mark on those not generated yet. `U` steps through the
  shown body's family (a planet and its moons). The star view becomes a
  **system view**: today's, with every planet drawn as its sphere on its
  orbit (a generated planet wrapped in its map, its thumbnail). Hovering
  a planet rings it and opens its card beside it, linked by a line: name
  and type, radius and mass, semi-major axis and eccentricity, current
  speed and year, equilibrium temperature, place relative to the
  habitable zone, moons, and whether it is generated, with a button that
  opens it. A click or a tap pins the card, and several pinned cards can
  be compared side by side; they follow their planets, and last for the
  session while the system is shown, the planet view included. A double
  click, or the card's button, opens the planet. The planet picked keeps
  its orbit line, its moons and its apsides; the scale (linear to just
  beyond the habitable zone, then compressed) and the clock (in
  Earth days per second, a day a second at the slowest) are the
  system's, so picking another planet moves nothing else.
  Every planet has a dotted line to the star with its distance now
  written along it, a velocity arrow along its orbit labelled with its
  speed now, and an arrow of the star's pull towards the star (**Lines to
  the star**, **Velocities** and **Star's pull** hide them); its name
  sits on the side away from its velocity. The habitable zone is
  crossed by a dashed line giving its width and area.
- **A body not generated.** Picking it shows its sphere in the planet
  view (lit, with its axis and its sky) and its card, with **Generate**.
  Pressing it, or picking a body whose neighbourhood is not complete,
  queues the job; the view switches to the maps when it ends.
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
  strip). **Rename** moves into the card and names any body.

## Removing the old worlds

No migration, as decided: the worlds made before systems are deleted.

- The server stops reading `worlds/`. On its first start with systems,
  if `worlds/` holds anything, it logs once that the directory is no
  longer used. It never deletes it: `mise run clean-worlds` does, when
  the user chooses to.
- The routes under `/api/worlds` and `/worlds/` are removed in the same
  release, with no compatibility layer: nothing else calls them.
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
7. **Clean-up**: dead code, READMEs, backlog, analysis docs,
   `clean-worlds`.

Phase 4 can start once phase 1 has frozen the types, beside phases 2 and
3; phases 5 and 6 need phase 3. The two sessions can split as before:
the generator and the server (1 to 3) on one side, the web (4 to 6) on
the other.

## Open points

- Rocky planets' occurrence by star type, and how far to trust Kepler's
  counts outside 1 AU.
- Whether a system can ask for a habitable moon of a giant in mode 1
  (`moons of giants: terran`), or only in mode 2.
- Binary stars: out of scope here, kept possible by the outer edge.
