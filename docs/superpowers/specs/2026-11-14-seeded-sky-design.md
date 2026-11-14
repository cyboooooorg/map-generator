# Seeded sky with gas and dust clouds: design

Issue #429. Decided with the user on 2026-11-11 (issue comment) and
2026-11-14 (this conversation).

## Intent

Every view draws the same sky today: `starField()` in
`web/src/lib/viewer/stars.ts:49` defaults to seed `7` and none of its three
callers passes one. Moving to another system should feel like being
somewhere else. For the user, two things carry that feeling:

1. **The gas clouds and nebulae differ** in shape and colour from one
   system to another. This is the main signal.
2. **The sky is brighter or darker** with the system's place in the
   galaxy: brighter toward the galactic core, darker in the outskirts.

The sky must not conflict with the future galactic map (#420, track E
#421). That map places real nearby stars, and drawn systems will be given
a place on it later. So the sky is not a function of the seed alone: it is
a function of the system's **galactic position**, stored in the system,
and the seed only drives what is really random (the fine shape of the
clouds, the individual stars).

## Goals

- Each system has a galactic position, stored in `system.json`, drawn at
  creation. The galactic map will read the same field later. A system from
  a catalogue will write its real position there.
- The viewer's sky derives from that position plus the system seed:
  - the brightness and density of the Milky Way band, and the visible
    bulge toward the galactic centre, from the position;
  - the band's orientation, from the system's orientation relative to the
    galactic plane (also stored);
  - the shapes and colour mix of the clouds and the individual stars,
    from the seed.
- Faint gas and dust clouds: emission nebulae (red H-alpha, blue-green
  O III), blue reflection nebulae, dark dust lanes that dim the band and
  the stars behind them, concentrated along the band like the real
  galactic plane.
- Every view of a system (planet and moons in `PlanetView`, the system in
  `StarView`, the star close-up in `StarBodyView`) shows the same sky.
- A viewer control with three choices: the system's sky (default), the
  current fixed sky, no sky.

## Non-goals

- The galactic map itself (#420), and assigning drawn systems a place on
  it.
- Real catalogue positions (#419, #420) — only the field they will fill.
- Old stored systems: no care is needed for them (user, 2026-11-14). See
  "Format".
- A sky reroll or seed override in the viewer (may come later).
- Real constellations or a real Milky Way photograph for the Solar System.

## Anti-goals

- Clouds that compete with the planet: they are a backdrop, faint behind
  it.
- Clouds that fight the star close-up's photo filter (#415).
- A noise shader evaluated every frame (decided 2026-11-11: bake once).
- A drawn position that the galactic map would have to recompute or
  contradict: one stored field, one source.

## Decisions

### Galactic position (generator, Rust)

A new block on `System` (`crates/mapgen/src/system.rs:136`), for example
`galaxy: GalacticPlace`, in a new module `crates/mapgen/src/galaxy.rs`:

- **Position**: galactocentric Cartesian coordinates in kpc, the convention
  the galactic map will use (Sun at about R = 8.2 kpc, z = +0.02 kpc).
- **Orientation**: how the system's own frame sits in the galactic frame:
  enough to give, in the system frame, the direction of the galactic north
  pole and the direction of the galactic centre. **The system frame is the
  star's equatorial frame**, the one planets' orbit inclinations are given
  against (`Orbit.inclination_deg`, system.rs:247), with the reference
  direction of the nodes (`node_deg`) as its x axis (builder's call
  noted below).
- **Sky seed**: a `u32` drawn from the same stream, which the web uses for
  the clouds and the individual stars, so that systems with neighbouring
  seeds do not look alike and the web needs no hash of its own.
- **Source**: `drawn` now; `observed` for a catalogue system later (the
  provenance principle of #421, kept to this one block).

How it is drawn, for a whole system (`place_whole`, system.rs:821) and a
system around a planet (`WorldRequest::resolve`, request.rs:558):

- From its own rng stream (a new constant in `noise::stream`, next to
  `METALLICITY = 27`; 43 is free), seeded by `stream_seed(seed, stream)`
  like the metallicity's (giants.rs:195) and shared with no other draw, so
  that no existing draw shifts: every other value of a system stays
  identical. All the draws of the block (radius scatter, height, azimuth,
  orientation, sky seed) come from this one stream, in an order the
  builder picks. It needs the metallicity, so it runs once that is known.
- **Galactocentric radius from the metallicity** (user's choice): invert
  the radial metallicity gradient of the disc, about −0.06 dex/kpc
  (Cepheids, Genovali et al. 2014), plus a scatter (the spread of [Fe/H] at
  a given radius, about 0.1 dex), clamped to the disc (about 2 to 16 kpc).
  The generator's mean metallicity (−0.1, giants.rs:90, the solar
  neighbourhood's) maps to the Sun's radius, 8.2 kpc, so a typical system
  has a sky like ours and markedly brighter or darker skies are each about
  one system in six (radius within about 5 kpc, or beyond about 11 kpc).
  A metal-rich star tends to sit closer to the core, so its sky tends to be
  brighter. The metallicity used is the system's, drawn or set in the
  request (`metallicity` of a request sets the radius too).
- **Height above the plane**: drawn from the thin disc's vertical
  profile (exponential, scale height about 300 pc).
- **Azimuth** around the centre: uniform.
- **Orientation**: uniform random rotation.

### Format

- The block is always written. No `SYSTEM_FORMAT_VERSION` bump: on read,
  a system without it gets one drawn from its seed and its stored
  metallicity (`star.metallicity`), with the same stream, as
  `read_system` already fills a missing `star.activity` (system.rs:394).
  This keeps clear of the bump to 5 in #465 (planet model part 4).
- ts-rs exports the type to `web/src/lib/api/types/`; the web reads it
  from `SystemDetail` (`fetchSystem`, `world.svelte.ts:520`).

### Sky (web)

- **Frames**: the cube map and the star directions are built once in the
  system frame (the star's equator). Each view rotates them into its own
  scene frame, as `PlanetView` already turns its stars about the spin axis
  (`setStarsTurn`, PlanetView.ts:1902): a planet's view applies its
  obliquity to the star's equator, a moon's view also its orbit's tilt to
  its planet; the system and star views use the star's equator directly.
  The sky stays fixed against the stars, never turning with the globe.
- **One sky per system, baked once** into a cube map (decided
  2026-11-11) when the system opens, and shared by its views. The clouds
  are in the cube map. The stars stay points over it (sharp at any
  resolution), drawn by `starField`, which now takes the seed and the
  galactic place.
- **Brightness from the position**: the band's surface brightness and
  star density follow the disc's density at the system's radius
  (exponential disc, scale length about 2.6 kpc) relative to the Sun's;
  the bulge toward the galactic centre grows as the radius shrinks; far
  from the plane, the band narrows and the sky outside it empties.
- **Band orientation** from the stored orientation, replacing the fixed
  `BAND_TILT` = 60°.
- **The seed** of the clouds and stars is the block's sky seed, not
  `system.seed`.
- **Dust dims what is behind it**: the dark lanes darken the band in the
  cube map and also the star points behind them.
- **Faint**: the clouds stay well below the band's brightness and below
  the planet. The photo filter (#415) is a CPU change of the star's own
  light (`StarBodyView.applyLight`, StarBodyView.ts:304) and leaves the
  sky alone; the clouds must simply stay faint there too.
- **Hidden by the host star's disc** in `PlanetView` as the stars are
  (`HIDE_HOST_STAR`, shaders.ts:775): drawn behind the stars (render
  order below −3) and behind the disc.
- **Compared worlds each keep their own sky**, even with linked cameras.
  The comment in `stars.ts` saying they share the same stars is updated.
- **A world without a system** (standalone, `world.system` null) shows
  the fixed sky.

### Sky control

- The stars toggle (`Viewer.svelte:1904`, display key `stars`, shortcut
  `b`, linked display) becomes a three-way choice: the system's sky
  (default), the fixed sky (today's), no sky. A stored boolean migrates
  (`true` to the system's sky, `false` to none) in `migrateDisplay`.
- It applies to all three views; `StarView` and `StarBodyView` have no sky
  toggle today and get one.

## Calls made by the author (not discussed with the user)

- The system frame is the star's equator, x along the nodes' reference.
- A stored sky seed rather than a hash of `system.seed` in the web.
- The mean metallicity maps to the Sun's radius (a typical sky is ours).
- A request's explicit metallicity also sets the radius.
- A stored stars toggle migrates to the system's sky or to none.

## Left to the builder

- The cube map's resolution and whether it is baked on the GPU (render to
  cube target) or the CPU; how long the bake may take as long as opening a
  system does not visibly stall.
- The cloud noise (fBm, domain warping…), the exact colours, the cloud
  amount and how much each colour family varies from seed to seed, within
  "faint" and "different shape and colour per system".
- The exact numbers of the gradient, scatter, clamps and brightness laws
  above (starting points given), and the exact serialization of the
  orientation (quaternion, or two unit vectors).
- How the views receive the sky (a new setter, or a shared object built in
  `Viewer.svelte`).
- How dust dims the stars (sampling the cube map in the stars' shader, or
  computing it in `starField`).

## Done when

- Two systems with different seeds show different clouds; the same system
  always shows the same sky, in every view.
- A system near the core has a visibly brighter band and bulge than one in
  the outskirts.
- Clouds are visible but discreet; dust lanes darken the band.
- `system.json` stores the galactic place; every other value of a
  regenerated system is unchanged (world-coherence check).
- The viewer offers the sky choice in every view.
- Tests: Rust (deterministic draw, radius tied to metallicity, no shifted
  draws, missing block filled on read); web (`stars.test.ts` covers the
  seeding, the position's effect on brightness, the band orientation).
- Documentation: README (system file and viewer sky), `docs/system-spec.md`
  (the new block), the viewer's help text.
