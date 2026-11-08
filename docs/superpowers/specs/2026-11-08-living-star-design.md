# Living star, eruptions and magnetic fields: design

Date: 2026-11-08. Status: approved by the user. Tracked in #343 (part 1),
#344 (part 2) and #345 (part 3).

## Intent

Today a system's star is a backdrop. The system view (`StarView.ts`) draws
it as a shaded sphere (`starSurfaceFragment`, `starSurface` in
`lib/viewer/starSystem.ts`) with limb darkening, granulation and spots
whose amounts are per-class guesses, and a few prominences placed from the
seed. Nothing on it changes with time, nothing leaves it, and no body has
a magnetic field.

The user wants the star to be a body of its own: something to look at up
close, alive, erupting, its eruptions reaching the planets, with the
magnetic fields of every body visible on demand. Like the rest of the
project, what is drawn follows published science, and what is only a
calibration says so.

## Goals

The work is three parts, delivered in this order, each in its own pull
requests and usable on its own.

### Part 1: the living star

- **A generated star.** The generator draws, for each system's star, what
  its surface needs: rotation period, activity level, activity cycle,
  the latitude band of its active regions, spot and facula coverage,
  granulation scale and contrast, limb darkening, from the star's type,
  mass, temperature and age, with sources (gyrochronology for rotation,
  activity from the Rossby number, and so on). Stored with the system, so
  the same seed gives the same star.
- **A living surface, deterministic in time.** Granulation that boils;
  active regions that are born in the activity band, carry spots and
  faculae, turn with the star (differential rotation) and decay over days
  to weeks; their number follows the cycle. The surface at a given date is
  a function of the seed and the date, so scrubbing time back and forth
  shows the same surface, as orbits already do.
- **Coronal loops** ("grands arcs") stand over the active regions and
  follow them; they replace today's seed-placed prominences.
- **A new star view**, separate from today's system view: the star alone
  as a 3D globe, like a planet in its viewer, with its corona, loops and
  live surface, its rotation and the shared time controls.
- **The same star everywhere.** The system view and the sky seen from a
  planet show this same star: its colour, limb darkening and large spots
  and regions, the fine detail (granulation) simplified when it is small
  on screen.

### Part 2: eruptions and the stellar wind

- **Coronal mass ejections**, animated, leaving from active regions:
  - spontaneous, at a rate that follows the star's activity (and its
    cycle), with speed, mass and energy drawn from published statistics
    (solar CME catalogues; stellar flare–CME relations for other stars);
  - triggered by hand: a button launches one, aimed by clicking an active
    region or a planet, its strength drawn for that star (with a setting
    to choose it).
- **The stellar wind**: speed, density and dynamic pressure, from the
  star (a Parker-type wind scaled by published stellar mass-loss
  relations), shown in the star view.
- **"Show dynamic data"** (an option, off by default): while an ejection
  runs, a data card appears with its speed, mass, energy, direction and
  the time it reaches each planet in its path. The card sits around the
  star, never in front of it, not far from the ejection; it is draggable;
  a thin line ties it to the centre of the ejection and follows it.
- **Arrival at a planet**: the ejection travels out through the system
  view and reaches the planets in its path. When it reaches one, a data
  card shows there too (arrival speed, dynamic pressure, magnetopause
  stand-off distance, the latitude the aurorae reach). With
  **animations** on (an option), the planet's magnetosphere is compressed
  and its aurorae light up, in the system view and in that planet's own
  view. A planet without a magnetic field gets no auroral oval; its
  atmosphere is stripped by the wind instead (shown as such).
- Ejections take one to several days to reach a planet at 1 AU: they play
  on the existing time controls and their acceleration.

### Part 3: magnetic fields in every view

- **An option "magnetic field"** in every view (star view, system view,
  planet view).
- **Field lines in dotted lines**, leaving one magnetic pole and returning
  to the other, as a dipole.
- **The star's field** is its own and is not deformed by its wind.
- **A planet's field** is deformed by the star's wind: compressed on the
  day side, drawn out into a long tail on the night side; the stronger
  the wind (and during an ejection's arrival), the more it is compressed.
- **Which bodies have a field, how strong and how tilted**: added to the
  generator, from published dynamo scalings (from the body's mass,
  rotation, interior and age): rocky planets with an active core get one,
  those without (Mars-, Venus-like) none, giants strong ones. A moon inside
  its giant's magnetosphere sits in the giant's field, not in the wind.

## Non-goals

- No magnetohydrodynamic simulation: the corona, ejections, wind and
  magnetospheres are drawn from analytic models and published statistics,
  not solved.
- No spectra, no radio or X-ray views, no flare light curves beyond what
  the visuals and data cards need.
- No change to how planets are generated, except the new magnetic field
  of part 3.

## Anti-goal: the frame rate

The user would wince at large frame-rate drops in the browser. This bounds
every choice:

- With the new options off, the frame time of every existing view does not
  measurably change.
- With everything on, each view keeps a smooth frame rate on the user's
  machine (an Apple-silicon Mac, a 1080p to 1440p window). The builder
  measures before and after, in the PRs.
- The surface is drawn on the GPU (procedural granulation in the shader;
  active regions as a small uniform array or data texture updated only
  when they change); ejections are a capped number of instanced particles
  or meshes; field lines are a few precomputed polylines, rebuilt only
  when the wind changes noticeably; detail falls off with the star's size
  on screen; anything off or out of view costs nothing.

## Decided by the user

- A separate star view, distinct from today's system view, with the
  surface in 3D, spots, granulation, loops and realistic ejections.
- The surface is alive (it changes with time) in that view.
- Ejections are both spontaneous (following activity) and triggered by
  hand.
- The data cards: shown when "show dynamic data" is on; around the star,
  not in front, near the ejection; draggable; tied by a line to the
  ejection's centre. Also shown at a planet when an ejection reaches it,
  with the animations when those are on.
- Magnetic field lines as dotted lines from pole to pole, available in all
  views; planets' fields deformed by the wind; the star's not.
- Order: living star, then ejections and wind, then magnetic fields.
- No big frame-rate drops.
- **Opening the star view: both ways.** Picking the star in the body list
  still opens the system view by default; a button (and a double-click on
  the star) switches to the star view, and the same in the other
  direction, like two tabs of the same star.

## Calls made by the designer (to check)

- **One clock.** "The date" is the system's clock, in days from the
  system's t = 0, the one the orbits already run on. The star view, the
  system view and a planet's sky read the same clock, so they show the
  same instant; the star's rotation phase and its active regions are
  functions of the seed and that clock. It can run backwards and far
  (an 11-year cycle over days of play): the functions are written for
  that precision (days as f64 on the Rust side, a phase reduced before it
  reaches the shader).
- **The star's age** is the one the generator already uses for a system
  (`giants::system_age_gyr`), with a seeded scatter around the
  rotation–age relation so two stars of the same type differ, as observed
  stars do at a given age. No change to planets.
- **Stars without a convective envelope** (about A and hotter, above
  roughly 7 000 to 7 500 K) have no spots, faculae, loops or coronal
  ejections: a smooth, limb-darkened surface and a fast line-driven wind
  instead. Fully convective M dwarfs get their own (stronger, flaring)
  activity relations. The star view shows each as it is.
- **Where the new data live.** The star's activity fields belong to the
  system's star record (one per system), not to `StarInfo`, which is per
  world. Part 3's magnetic fields are derived from what each body already
  stores (mass, radius, rotation, type, the system's age) by deterministic
  functions, so they need no new draw and change no world.
- **What an ejection looks like**: an expanding loop-shaped front (a
  flux rope seen in 3D, as coronagraphs show it) with a capped number of
  particles; in the system view it keeps travelling out at the AU scale,
  as a widening front, until it passes the outermost planet in its path.
- **Moons**: a moon whose orbit lies inside its giant's magnetopause
  stand-off distance sits in the giant's field; one outside it meets the
  wind. Systems have one star (the generator draws no binaries).
- The surface's deterministic-in-time rule (seed + date), so time
  scrubbing is consistent.
- Coronal loops replace today's seed-placed prominences.
- The new options are off by default.
- Stored star activity data are new optional fields; systems drawn before
  them get them derived on load from the star's stored type, mass,
  temperature and the system's age (no format gate needed: they are
  derived, not re-drawn).

## Left to the builder

- The exact sources and relations, chosen from the literature and cited
  in code and docs, with calibrations labelled.
- Shader techniques, data layout of active regions, particle counts,
  level-of-detail thresholds, within the frame-rate bound.
- The data card's layout and placement algorithm, within the rules above.
- How time acceleration presets fit ejections (hours to days).
- Splitting each part into pull requests, each with its docs and tests,
  and the GitHub issues that track them (label `backlog`, one per part at
  least).
