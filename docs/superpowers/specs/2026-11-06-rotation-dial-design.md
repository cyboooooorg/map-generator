# Rotation dial

## Intent

In the planet view, the user wants to see how fast the body shown turns on
itself. Some bodies make that hard to see: a giant's bands are nearly
uniform along a parallel, and they drift with their own winds (faster or
slower than the body, some backwards), so watching them shows the wind, not
the spin. The dial shows the body's true rotation, the one that sets its
day, and the time of day on it.

The terms involved (sidereal day, solar day, sol, local time, synchronous
rotation) are not common knowledge: the dial explains them where it shows
them.

## Goals

The work comes in two pull requests, in this order: the fixes to how moons
are placed and turned (the first two sections below), then the dial.

### First: moons' orbits fixed against the stars

`placeSystem` (`lib/systemSky.ts`) builds a moon's orbit plane from its
planet's axes, which turn with the planet's day. Every moon's orbit thus
spins round its planet once per planetary day on top of the moon's own
motion: from the planet its moons hang nearly still instead of rising and
setting, and the sun over a locked moon goes round its sky in a fraction of
a day instead of once per orbit. The user chose to fix it, knowing it moves
the moons and the sky seen from them.

- A moon's orbit plane is built from its planet's equatorial frame held
  fixed against the stars: the spin axis, and the planet's equinox direction
  (`start`, where its equator crosses its own orbit's plane) as the zero of
  the node's longitude.
- An inclined moon keeps its inclination to the planet's equator, its node
  fixed against the stars; an orbit inclined over 90° stays retrograde.
- Planets are unchanged: their parent frame is the star's fixed frame.
- A free-spinning moon's `start` and spin axis follow, being built from its
  orbit plane.
- Not modelled, as before: the slow precession of the nodes, and distant
  moons' orbits lying nearer the planet's orbital plane than its equator.

### First: a steady spin for synchronous bodies

`placeSystem` turns a locked body to face its host exactly, even on an
eccentric orbit. The user chose a steady spin, as a real body has: it faces
where its host would be on a circular orbit at the same mean anomaly, so
that its host swings in its sky by the libration in longitude, true anomaly
minus mean anomaly (about ±2e radians). This moves the host and the sky of
every locked body on an eccentric orbit, slightly.

### The dial

- A ring round the body's equator, in its equatorial plane, a little away
  from the surface (not touching it), lightly styled like an instrument
  bezel. Mockups made with the user: a thin bright circle, a fainter outer
  circle, a tick every 5°, a longer tick with its number every 30°.
- A pointer from the body's surface on the equator out to the ring, ending
  in a small arrow head and a label giving its angle on the ring (`137°`).
- A sun marker on the ring, where the star's direction crosses the
  equatorial plane.
- Shown under a switch of its own, off by default, saved with the display
  settings.

### Free-spinning body (planets, captured moons)

- The ring is fixed against the stars. Its 0° is the equinox direction: the
  line where the body's equator crosses its orbit's plane (`start` in
  `placeSystem`, `lib/systemSky.ts`, the direction longitude 0 faces at
  day 0). Angles grow in the body's sense of rotation.
- The pointer starts from longitude 0 (the body's meridian 0) and turns
  with the body: a whole turn of the pointer is a sidereal day.
- With the camera still against the sky (the default for these bodies), the
  ring stays still on screen and the pointer goes round it.

### Synchronous body (locked moons, planets locked to their star)

- The ring is graduated against the host: 0° points at the planet the moon
  orbits, or at the star for a locked planet. It stays still on screen with
  the camera turning with the body (the default for these bodies).
- The pointer starts from the face turned to the host (longitude 180° in
  this app; the scientific convention puts longitude 0 there, which the
  glossary mentions).
- Markers: the sun and, for a moon, the host planet. For a planet locked to
  its star, the host is the sun: one marker.

### The figures

Shown in the *Light and season* panel while the dial is on:

- Free body: rotation angle, local time at meridian 0, sidereal day, solar
  day.
- Synchronous body: "synchronous rotation", local time at the face turned to
  the host, solar day (about one orbit for a moon; none for a planet locked
  to its star, whose sun stands still in its sky), and the libration now
  with its amplitude (`+0.3° (±0.4°)`). The pointer swings round 0° by the
  same angle.
- Local time follows the convention of Mars missions: the solar day cut into
  24 local hours, noon when the sun crosses the meridian, whatever the
  day's length. It is computed from the sun's direction in the body's frame
  (`sysSky.sun`), so that it matches the lighting drawn.
- Sidereal day: `spin.period_hours`, or the orbit's period for a locked
  body. Solar day: `1 / (1/sidereal − 1/year)`, `year` being the orbital
  period of the body round the star (the planet's year for a moon).

### The glossary

- A `?` beside the figures opens a short glossary: sidereal day, solar day,
  sol (the solar day of Mars, 24 h 39 min 35 s; elsewhere, "solar day"),
  local time, synchronous rotation, libration, and the longitude convention
  of synchronous bodies.
- The same text goes in `web/README.md`, in English, with the French strings
  in `lib/i18n/fr.ts`.

## Non-goals

- The star view and the map view: the dial is drawn on the globe only.
- Worlds outside a system: they have no clock to turn them. The dial needs a
  system body (`sysBody`).
- The giant's band winds: the dial does not show them.

## Anti-goals

- Moving anything else already drawn. The two fixes above move the moons,
  the sky seen from them, and the host in the sky of eccentric locked
  bodies; nothing else moves, and the dial itself is an overlay.
- Changing the generated data or `SYSTEM_FORMAT_VERSION`, or regenerating
  anything.
- A dial that disagrees with the scene: its sun marker, pointer and local
  time must match the lighting and the sky drawn at that instant.

## Constraints

- Svelte 5 and Three.js, strict TypeScript, as the rest of `web/`.
- English in the code and docs; user-facing strings in en and fr.
- Fades with the globe's morph, as the other guides do (`bodyGuides.ts`).
- Cheap enough for the light quality profile.

## Decided by the user

- The dial round the equator, a little away from the surface, styled, with
  degree ticks, and a pointer with the angle at its tip.
- Both the rotation against the stars and the local time, with the
  sidereal and solar days shown.
- The vocabulary explained in the app, as it is not common knowledge.
- For synchronous bodies, moons and planets locked to their star alike: the
  ring graduated against the host and still on screen, the pointer from the
  face turned to the host, the sun and host markers, local time and solar
  day.
- The information shown in the mockups.
- Moons' orbit planes fixed against the stars in their planet's equatorial
  frame, accepting that moons and their skies move (with inclined orbits
  keeping their inclination to the equator).
- A steady spin for synchronous bodies, with the libration shown.

## My calls (the user can override them)

- 0° of a free body's ring is the equinox direction (`start`).
- The switch sits in the left icon bar's sky group, next to *Spin axes*; the
  figures in the *Light and season* panel.
- Ring radius: 1.25 body radii, or 1.25 times the largest radius of an
  irregular body's shape.
- Labels (the numbers every 30° and the pointer's) are HTML, placed like the
  guide labels (`skyScreenPositions`, `SkyLabels`), hidden behind the globe.

## Left to the builder

- The 3D construction of the ring, ticks, pointer and markers, and their
  colours, within the mockups' look.
- How the free body's angle is taken from the scene (the body's axes in
  `placeSystem`, or `starsTurn` corrected for `skySpin`, which the star field
  omits today).
- The sense of rotation for the rare retrograde moon orbits (spin is always
  prograde in the data).

## Left to the builder (fixes)

- How the planet's fixed equatorial frame is passed to its moons' orbits in
  `placeSystem`, and which tests (`systemSky` fixtures, the star view's moon
  orbits, the ring moons' gaps and rocks) need their expectations updated
  for the moved moons.
- The star view draws its own moon orbits (`lib/viewer/starSystem.ts`):
  check whether it shares the defect, and keep the two views consistent.
